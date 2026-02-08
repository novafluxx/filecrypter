// commands/archive.rs - Archive Utilities for Batch Operations
//
// This module provides utilities for creating and extracting compressed TAR archives
// for use with the batch archive encryption mode.
//
// Archive Flow:
// - Encrypt: Files -> TAR.ZSTD archive -> Encrypt archive as single unit
// - Decrypt: Decrypt archive -> Extract TAR.ZSTD -> Original files
//
// Security Considerations:
// - Path traversal prevention (reject entries with ".." or absolute paths)
// - Symlink rejection (don't include/extract symlinks)
// - Decompression bomb protection (validate extracted size)

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::error::{CryptoError, CryptoResult};
use crate::security::create_secure_tempfile;
use chrono::Local;
use tar::{Archive, Builder, EntryType};

#[cfg(windows)]
use crate::security::set_owner_only_dacl;

/// Maximum decompressed size relative to archive size (100x multiplier).
/// Text files can easily compress 20x or more with ZSTD, so a conservative
/// ratio like 10x would cause false positives for legitimate archives.
/// This ratio check is combined with an absolute size limit for defense in depth.
const MAX_DECOMPRESSION_RATIO: u64 = 100;

/// Absolute maximum extracted size in bytes (10 GB).
/// This provides a hard cap regardless of archive size to prevent resource exhaustion.
/// Even with the ratio check, a very large archive could theoretically extract to
/// an unreasonably large size, so this cap provides an additional safety layer.
const MAX_EXTRACTED_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024; // 10 GB

/// Default ZSTD compression level for archives
const ARCHIVE_COMPRESSION_LEVEL: i32 = 3;

/// Progress callback type for archive operations
pub type ArchiveProgressCallback = Box<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Create a compressed TAR archive from multiple files
///
/// Files are bundled into a TAR archive and compressed with ZSTD.
/// Archive entries use relative paths based on the common prefix of input paths.
///
/// # Arguments
/// * `input_paths` - Paths to files to include in the archive
/// * `output_path` - Where to write the .tar.zst archive
/// * `progress_callback` - Optional callback (files_processed, total_files, current_file)
///
/// # Returns
/// Ok(()) on success, or CryptoError on failure
pub fn create_tar_zstd_archive<P, Q>(
    input_paths: &[P],
    output_path: Q,
    progress_callback: Option<ArchiveProgressCallback>,
) -> CryptoResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    if input_paths.is_empty() {
        return Err(CryptoError::FormatError(
            "No files provided for archive".to_string(),
        ));
    }

    let output_path = output_path.as_ref();
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));

    // Create secure temp file
    let temp_file = create_secure_tempfile(parent)?;
    let temp_path = temp_file.path().to_path_buf();

    // Create ZSTD compressed writer
    let file = File::create(&temp_path)?;
    let zstd_writer =
        zstd::Encoder::new(BufWriter::new(file), ARCHIVE_COMPRESSION_LEVEL)?.auto_finish();

    // Create TAR builder
    let mut tar_builder = Builder::new(zstd_writer);

    // Compute common prefix for relative paths
    let paths: Vec<PathBuf> = input_paths
        .iter()
        .map(|p| p.as_ref().to_path_buf())
        .collect();
    let common_prefix = compute_common_prefix(&paths);

    let total_files = input_paths.len();

    for (index, input_path) in input_paths.iter().enumerate() {
        let input_path = input_path.as_ref();

        // Validate input path
        let canonical_path = validate_archive_input(input_path)?;

        // Get filename for progress
        let file_name = canonical_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback(index, total_files, &file_name);
        }

        // Compute archive entry name (relative path from common prefix)
        let archive_name = compute_archive_entry_name(&canonical_path, &common_prefix)?;

        // Add file to archive
        let mut file = File::open(&canonical_path)?;
        tar_builder.append_file(&archive_name, &mut file)?;
    }

    // Finish TAR archive
    let zstd_writer = tar_builder.into_inner()?;
    drop(zstd_writer); // Ensure ZSTD encoder is flushed

    // Persist temp file to output path
    fs::rename(&temp_path, output_path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        CryptoError::Io(e)
    })?;

    // Report completion
    if let Some(ref callback) = progress_callback {
        callback(total_files, total_files, "");
    }

    Ok(())
}

/// Extract a compressed TAR archive to a directory
///
/// Validates archive entries for security (path traversal, symlinks, decompression bombs).
///
/// # Arguments
/// * `archive_path` - Path to the .tar.zst archive
/// * `output_dir` - Directory where files will be extracted
/// * `allow_overwrite` - Whether to overwrite existing files
/// * `progress_callback` - Optional callback (files_processed, total_files, current_file)
///
/// # Returns
/// Vector of extracted file paths on success, or CryptoError on failure
pub fn extract_tar_zstd_archive<P, Q>(
    archive_path: P,
    output_dir: Q,
    allow_overwrite: bool,
    progress_callback: Option<ArchiveProgressCallback>,
) -> CryptoResult<Vec<PathBuf>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    // Validate output directory
    if !output_dir.is_dir() {
        return Err(CryptoError::FormatError(
            "Output directory does not exist".to_string(),
        ));
    }

    // Calculate decompression bomb limits using a combined approach:
    // 1. Ratio-based limit: archive size * MAX_DECOMPRESSION_RATIO (100x)
    // 2. Absolute limit: MAX_EXTRACTED_SIZE_BYTES (10 GB hard cap)
    // The effective limit is the minimum of these two values.
    let archive_size = fs::metadata(archive_path)?.len();
    let ratio_based_limit = archive_size.saturating_mul(MAX_DECOMPRESSION_RATIO);
    let max_extracted_size = ratio_based_limit.min(MAX_EXTRACTED_SIZE_BYTES);

    // Open archive with ZSTD decompression
    let file = File::open(archive_path)?;
    let zstd_reader = zstd::Decoder::new(BufReader::new(file))?;
    let mut archive = Archive::new(zstd_reader);

    // First pass: count entries and validate
    let file = File::open(archive_path)?;
    let zstd_reader = zstd::Decoder::new(BufReader::new(file))?;
    let mut count_archive = Archive::new(zstd_reader);

    let mut total_files = 0usize;
    let mut total_size = 0u64;

    for entry in count_archive.entries()? {
        let entry = entry?;

        // Validate entry
        validate_archive_entry(&entry)?;

        total_size = total_size.saturating_add(entry.size());
        total_files += 1;

        // Check for decompression bomb (combined ratio + absolute limit check)
        if total_size > max_extracted_size {
            let limit_type = if max_extracted_size == MAX_EXTRACTED_SIZE_BYTES {
                "absolute limit of 10 GB"
            } else {
                "100x compression ratio limit"
            };
            return Err(CryptoError::ArchiveError(format!(
                "Archive extraction would exceed safe size limit ({} bytes, {})",
                max_extracted_size, limit_type
            )));
        }
    }

    // Second pass: extract files
    let mut extracted_paths = Vec::with_capacity(total_files);
    let canonical_output = fs::canonicalize(output_dir)?;

    for (index, entry) in archive.entries()?.enumerate() {
        let mut entry = entry?;

        // Get entry path
        let entry_path = entry.path()?.to_path_buf();
        let file_name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback(index, total_files, &file_name);
        }

        // Validate entry type (only regular files)
        // Hard links, directories, FIFOs, device files, etc. are silently skipped.
        // This is intentional: we only extract actual file content, not metadata-only
        // entries or special file types that could pose security risks.
        match entry.header().entry_type() {
            EntryType::Regular | EntryType::Continuous => {}
            _ => continue, // Skip directories, hard links, symlinks, etc.
        }

        // Compute safe output path
        let safe_output_path = compute_safe_output_path(&entry_path, &canonical_output)?;

        // Check overwrite
        if safe_output_path.exists() && !allow_overwrite {
            // Use collision avoidance
            let resolved_path =
                crate::commands::file_utils::resolve_output_path(&safe_output_path, false)?;
            extract_entry_to_path(&mut entry, &resolved_path)?;
            extracted_paths.push(resolved_path);
        } else {
            // Create parent directories if needed
            if let Some(parent) = safe_output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            extract_entry_to_path(&mut entry, &safe_output_path)?;
            extracted_paths.push(safe_output_path);
        }
    }

    // Report completion
    if let Some(ref callback) = progress_callback {
        callback(total_files, total_files, "");
    }

    Ok(extracted_paths)
}

/// Validate an input file path for archiving
fn validate_archive_input(path: &Path) -> CryptoResult<PathBuf> {
    // Check path exists
    if !path.exists() {
        return Err(CryptoError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", path.display()),
        )));
    }

    // Check it's a regular file (not a symlink or directory)
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(CryptoError::InvalidPath(
            "Symlinks are not allowed for security reasons".to_string(),
        ));
    }
    if !metadata.file_type().is_file() {
        return Err(CryptoError::InvalidPath(
            "Only regular files can be archived".to_string(),
        ));
    }

    // Return canonical path
    fs::canonicalize(path).map_err(CryptoError::Io)
}

/// Validate an archive entry for security
fn validate_archive_entry<R: Read>(entry: &tar::Entry<R>) -> CryptoResult<()> {
    let path = entry.path()?;

    // Check for absolute paths
    if path.is_absolute() {
        return Err(CryptoError::PathTraversal(
            "Archive contains absolute path".to_string(),
        ));
    }

    // Check for path traversal
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(CryptoError::PathTraversal(
                "Archive contains path traversal (../)".to_string(),
            ));
        }
    }

    // Check for symlinks (security concern - could point outside extraction directory)
    //
    // Symlinks are explicitly rejected because they could:
    // 1. Point to files outside the extraction directory (path escape)
    // 2. Create symlinks to sensitive system files
    // 3. Be used in symlink race attacks
    //
    // Hard links (EntryType::Link) are NOT rejected here because:
    // - They are silently skipped during extraction (only Regular/Continuous are processed)
    // - Hard links can only reference files within the same filesystem
    // - They cannot escape the extraction directory like symlinks can
    // - Rejecting them would prevent extracting legitimate archives that contain hard links
    let entry_type = entry.header().entry_type();
    if matches!(entry_type, EntryType::Symlink) {
        return Err(CryptoError::ArchiveError(
            "Archive contains symlinks which are not allowed for security reasons".to_string(),
        ));
    }

    Ok(())
}

/// Compute a safe output path that stays within the output directory
fn compute_safe_output_path(entry_path: &Path, output_dir: &Path) -> CryptoResult<PathBuf> {
    // Build the target path
    let target = output_dir.join(entry_path);

    // Verify it's still under output_dir after normalization
    // We can't canonicalize yet since the file doesn't exist, so we normalize manually
    let normalized = normalize_path(&target);

    // Check that normalized path starts with output_dir
    if !normalized.starts_with(output_dir) {
        return Err(CryptoError::PathTraversal(format!(
            "Path escape attempt: {}",
            entry_path.display()
        )));
    }

    Ok(normalized)
}

/// Normalize a path by resolving .. and . without requiring the path to exist
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            _ => {
                result.push(component);
            }
        }
    }

    result
}

/// Extract a tar entry to a specific path with secure permissions
fn extract_entry_to_path<R: Read>(entry: &mut tar::Entry<R>, path: &Path) -> CryptoResult<()> {
    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create file with secure permissions
    let mut file = create_output_file(path)?;

    // Copy data
    std::io::copy(entry, &mut file)?;
    file.flush()?;

    Ok(())
}

/// Create an output file with secure permissions
fn create_output_file(path: &Path) -> CryptoResult<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        Ok(file)
    }

    #[cfg(windows)]
    {
        let file = File::create(path)?;
        if let Err(e) = set_owner_only_dacl(path) {
            let _ = fs::remove_file(path);
            return Err(CryptoError::Io(e.into()));
        }
        Ok(file)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(File::create(path)?)
    }
}

/// Compute the common prefix directory for a set of paths
///
/// Returns the deepest common directory containing all files.
pub fn compute_common_prefix(paths: &[PathBuf]) -> PathBuf {
    if paths.is_empty() {
        return PathBuf::new();
    }

    if paths.len() == 1 {
        // For a single file, use its parent directory
        return paths[0].parent().unwrap_or(&PathBuf::new()).to_path_buf();
    }

    // Get canonicalized parents
    let parents: Vec<PathBuf> = paths
        .iter()
        .filter_map(|p| p.parent().map(|parent| parent.to_path_buf()))
        .collect();

    if parents.is_empty() {
        return PathBuf::new();
    }

    // Find common prefix among parent directories
    let first = &parents[0];
    let mut common_components: Vec<_> = first.components().collect();

    for parent in &parents[1..] {
        let components: Vec<_> = parent.components().collect();
        let matching = common_components
            .iter()
            .zip(components.iter())
            .take_while(|(a, b)| a == b)
            .count();
        common_components.truncate(matching);
    }

    common_components.iter().collect()
}

/// Compute the archive entry name for a file (relative to common prefix)
fn compute_archive_entry_name(file_path: &Path, common_prefix: &Path) -> CryptoResult<PathBuf> {
    // Log when common prefix is empty (helps debug cross-drive scenarios on Windows)
    if common_prefix.as_os_str().is_empty() {
        log::debug!(
            "Empty common prefix detected, using filename fallback for {}",
            file_path.display()
        );
    }

    // Try to strip the common prefix
    if let Ok(relative) = file_path.strip_prefix(common_prefix) {
        // Guard against empty prefix returning absolute path (Windows cross-drive scenario)
        // When files are on different drives (C:\ and D:\), common_prefix is empty,
        // and strip_prefix("") succeeds but returns the original absolute path.
        if !relative.is_absolute() {
            return Ok(relative.to_path_buf());
        }
    }

    // Fall back to just the filename
    Ok(file_path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("file")))
}

/// Maximum length for custom archive names.
/// This limit prevents excessively long filenames that could cause issues
/// with filesystem limits (commonly 255 bytes) and keeps names manageable.
const MAX_ARCHIVE_NAME_LENGTH: usize = 200;

/// Sanitize a custom archive name to prevent path traversal and invalid characters.
///
/// # Security Rationale
///
/// User-provided archive names are untrusted input that could contain:
/// - Path traversal sequences ("../../../etc/passwd" -> writes to /etc/passwd)
/// - Invalid filesystem characters that cause errors or unexpected behavior
/// - Leading dots that create hidden files or leave residual ".." after stripping slashes
///
/// # Sanitization Steps
///
/// 1. **Remove path separators** (`/` and `\`): Prevents directory escapes
/// 2. **Remove Windows-invalid chars** (`<>:"|?*`): Ensures cross-platform compatibility
/// 3. **Strip leading dots**: Prevents hidden files and residual `..` after step 1
///    (e.g., "../foo" becomes "..foo" after removing `/`, then "foo" after stripping dots)
/// 4. **Trim whitespace**: Removes accidental leading/trailing spaces
/// 5. **Limit length**: Prevents filesystem issues with overly long names
///
/// # Returns
///
/// A sanitized string safe for use as a filename. May be empty if the input
/// contained only invalid characters.
fn sanitize_archive_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        .collect();

    // Trim whitespace and leading dots (prevents hidden files and residual ".." after stripping slashes)
    let trimmed = sanitized.trim().trim_start_matches('.');

    if trimmed.len() > MAX_ARCHIVE_NAME_LENGTH {
        trimmed[..MAX_ARCHIVE_NAME_LENGTH].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Generate a timestamped archive filename
///
/// Format: archive_YYYYMMDD_HHMMSS.tar.zst.encrypted
pub fn generate_archive_name(custom_name: Option<&str>) -> String {
    if let Some(name) = custom_name {
        let sanitized = sanitize_archive_name(name);
        if !sanitized.is_empty() {
            return format!("{}.tar.zst", sanitized);
        }
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    format!("archive_{}.tar.zst", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::tempdir;

    fn test_password() -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("{now_nanos:x}{counter:x}")
    }

    #[test]
    fn test_compute_common_prefix_single_file() {
        let paths = vec![PathBuf::from("/home/user/docs/file.txt")];
        let prefix = compute_common_prefix(&paths);
        assert_eq!(prefix, PathBuf::from("/home/user/docs"));
    }

    #[test]
    fn test_compute_common_prefix_multiple_files() {
        let paths = vec![
            PathBuf::from("/home/user/docs/a.txt"),
            PathBuf::from("/home/user/docs/b.txt"),
            PathBuf::from("/home/user/docs/sub/c.txt"),
        ];
        let prefix = compute_common_prefix(&paths);
        assert_eq!(prefix, PathBuf::from("/home/user/docs"));
    }

    #[test]
    fn test_compute_common_prefix_different_roots() {
        let paths = vec![
            PathBuf::from("/home/user/docs/a.txt"),
            PathBuf::from("/var/data/b.txt"),
        ];
        let prefix = compute_common_prefix(&paths);
        assert_eq!(prefix, PathBuf::from("/"));
    }

    #[test]
    fn test_generate_archive_name_custom() {
        let name = generate_archive_name(Some("my_backup"));
        assert_eq!(name, "my_backup.tar.zst");
    }

    #[test]
    fn test_generate_archive_name_timestamp() {
        let name = generate_archive_name(None);
        assert!(name.starts_with("archive_"));
        assert!(name.ends_with(".tar.zst"));
    }

    #[test]
    fn test_normalize_path() {
        let path = PathBuf::from("/home/user/../user/./docs/file.txt");
        let normalized = normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("/home/user/docs/file.txt"));
    }

    #[test]
    fn test_archive_roundtrip() {
        let temp = tempdir().unwrap();
        let input_dir = temp.path().join("input");
        let output_dir = temp.path().join("output");
        let extract_dir = temp.path().join("extract");

        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();
        fs::create_dir_all(&extract_dir).unwrap();

        // Create test files
        let file1 = input_dir.join("file1.txt");
        let file2 = input_dir.join("file2.txt");
        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();

        // Create archive
        let archive_path = output_dir.join("test.tar.zst");
        create_tar_zstd_archive(&[&file1, &file2], &archive_path, None).unwrap();
        assert!(archive_path.exists());

        // Extract archive
        let extracted = extract_tar_zstd_archive(&archive_path, &extract_dir, false, None).unwrap();
        assert_eq!(extracted.len(), 2);

        // Verify content
        for path in &extracted {
            let original_name = path.file_name().unwrap();
            let original_path = input_dir.join(original_name);
            let original_content = fs::read(&original_path).unwrap();
            let extracted_content = fs::read(path).unwrap();
            assert_eq!(original_content, extracted_content);
        }
    }

    #[test]
    fn test_archive_rejects_path_traversal() {
        // This tests that validation rejects path traversal attempts
        let path = PathBuf::from("../../../etc/passwd");
        let output_dir = PathBuf::from("/tmp/safe");
        let result = compute_safe_output_path(&path, &output_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_archive_name_removes_path_separators() {
        assert_eq!(sanitize_archive_name("../../../etc/passwd"), "etcpasswd");
        assert_eq!(
            sanitize_archive_name("..\\..\\windows\\system32"),
            "windowssystem32"
        );
        assert_eq!(sanitize_archive_name("foo/bar/baz"), "foobarbaz");
    }

    #[test]
    fn test_sanitize_archive_name_removes_invalid_chars() {
        assert_eq!(sanitize_archive_name("file<name>"), "filename");
        assert_eq!(sanitize_archive_name("file:name"), "filename");
        assert_eq!(sanitize_archive_name("file\"name"), "filename");
        assert_eq!(sanitize_archive_name("file|name"), "filename");
        assert_eq!(sanitize_archive_name("file?name"), "filename");
        assert_eq!(sanitize_archive_name("file*name"), "filename");
    }

    #[test]
    fn test_sanitize_archive_name_trims_whitespace() {
        assert_eq!(sanitize_archive_name("  my_backup  "), "my_backup");
    }

    #[test]
    fn test_sanitize_archive_name_limits_length() {
        let long_name = "a".repeat(300);
        let result = sanitize_archive_name(&long_name);
        assert_eq!(result.len(), MAX_ARCHIVE_NAME_LENGTH);
    }

    #[test]
    fn test_generate_archive_name_sanitizes_input() {
        // Path traversal attempt should be sanitized
        let name = generate_archive_name(Some("../../../malicious"));
        assert_eq!(name, "malicious.tar.zst");

        // Only invalid chars should fall back to timestamp
        let name = generate_archive_name(Some("///"));
        assert!(name.starts_with("archive_"));
    }

    #[test]
    fn test_compute_archive_entry_name_empty_prefix() {
        // Simulates Windows cross-drive scenario where common_prefix is empty
        let file_path = PathBuf::from("/absolute/path/file.txt");
        let empty_prefix = PathBuf::new();

        let result = compute_archive_entry_name(&file_path, &empty_prefix).unwrap();

        // Should fall back to filename, not return absolute path
        assert!(!result.is_absolute());
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    /// Tests the Windows multi-drive scenario where files are on different drive letters.
    ///
    /// On Windows, when files are selected from C:\ and D:\, `compute_common_prefix()`
    /// returns an empty PathBuf because there's no common ancestor. The code must
    /// detect this and fall back to using just the filename for archive entries.
    ///
    /// Note: This test uses Windows-style paths explicitly. On Unix, paths starting
    /// with a drive letter like "C:\\" are treated as relative paths (not absolute),
    /// so the test verifies the filename fallback behavior regardless of platform.
    #[test]
    #[cfg(windows)]
    fn test_compute_archive_entry_name_windows_cross_drive() {
        // Simulate files on different Windows drives
        let file_c = PathBuf::from("C:\\Users\\test\\file1.txt");
        let file_d = PathBuf::from("D:\\Data\\file2.txt");
        let paths = vec![file_c.clone(), file_d.clone()];

        // Common prefix should be empty for cross-drive paths
        let common = compute_common_prefix(&paths);
        assert_eq!(
            common,
            PathBuf::new(),
            "Common prefix should be empty for cross-drive paths"
        );

        // Verify fallback produces filename-only entries (not absolute paths)
        let entry1 = compute_archive_entry_name(&file_c, &common).unwrap();
        let entry2 = compute_archive_entry_name(&file_d, &common).unwrap();

        assert!(
            !entry1.is_absolute(),
            "Entry should not be absolute: {:?}",
            entry1
        );
        assert!(
            !entry2.is_absolute(),
            "Entry should not be absolute: {:?}",
            entry2
        );
        assert_eq!(entry1, PathBuf::from("file1.txt"));
        assert_eq!(entry2, PathBuf::from("file2.txt"));
    }

    /// Cross-platform test for empty prefix fallback behavior.
    /// Verifies that the filename fallback works even when strip_prefix succeeds
    /// but would return an absolute path.
    #[test]
    fn test_compute_archive_entry_name_absolute_after_strip() {
        // Create a scenario where strip_prefix("") succeeds but returns absolute path
        // This happens on Windows with cross-drive paths, simulated here with
        // explicit absolute path check
        let empty_prefix = PathBuf::new();

        // On Unix, this is absolute; on Windows, a path like "C:\\" would be absolute
        #[cfg(unix)]
        let absolute_path = PathBuf::from("/var/data/important.txt");
        #[cfg(windows)]
        let absolute_path = PathBuf::from("C:\\var\\data\\important.txt");

        let result = compute_archive_entry_name(&absolute_path, &empty_prefix).unwrap();

        // Must always return a relative path (the filename)
        assert!(
            !result.is_absolute(),
            "Entry should never be absolute: {:?}",
            result
        );
        assert_eq!(result, PathBuf::from("important.txt"));
    }

    #[test]
    fn test_compute_archive_entry_name_normal_case() {
        let file_path = PathBuf::from("/home/user/docs/file.txt");
        let common_prefix = PathBuf::from("/home/user");

        let result = compute_archive_entry_name(&file_path, &common_prefix).unwrap();

        assert!(!result.is_absolute());
        assert_eq!(result, PathBuf::from("docs/file.txt"));
    }

    #[test]
    fn test_archive_encrypt_decrypt_pipeline() {
        use crate::crypto::{
            decrypt_file_streaming, encrypt_file_streaming, Password, DEFAULT_CHUNK_SIZE,
        };

        let temp = tempdir().unwrap();
        let input_dir = temp.path().join("input");
        let archive_dir = temp.path().join("archive");
        let encrypted_dir = temp.path().join("encrypted");
        let decrypted_dir = temp.path().join("decrypted");
        let extract_dir = temp.path().join("extract");

        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&archive_dir).unwrap();
        fs::create_dir_all(&encrypted_dir).unwrap();
        fs::create_dir_all(&decrypted_dir).unwrap();
        fs::create_dir_all(&extract_dir).unwrap();

        // Create test files with varied content
        let files = vec![
            ("readme.txt", b"This is a readme file." as &[u8]),
            ("binary.dat", &[0u8, 1, 2, 128, 255, 0, 42]),
            ("notes.md", b"# Notes\n\nSome markdown content.\n"),
        ];

        let input_paths: Vec<PathBuf> = files
            .iter()
            .map(|(name, content)| {
                let path = input_dir.join(name);
                fs::write(&path, content).unwrap();
                path
            })
            .collect();

        let input_refs: Vec<&Path> = input_paths.iter().map(|p| p.as_path()).collect();

        // Step 1: Create archive
        let archive_path = archive_dir.join("test.tar.zst");
        create_tar_zstd_archive(&input_refs, &archive_path, None).unwrap();
        assert!(archive_path.exists());

        // Step 2: Encrypt the archive
        let encrypted_path = encrypted_dir.join("test.tar.zst.encrypted");
        let password = Password::new(test_password());
        encrypt_file_streaming(
            &archive_path,
            &encrypted_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
            None,
            None,
        )
        .unwrap();
        assert!(encrypted_path.exists());

        // Step 3: Decrypt the archive
        let decrypted_archive_path = decrypted_dir.join("test.tar.zst");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_archive_path,
            &password,
            None,
            false,
            None,
        )
        .unwrap();
        assert!(decrypted_archive_path.exists());

        // Step 4: Extract the archive
        let extracted =
            extract_tar_zstd_archive(&decrypted_archive_path, &extract_dir, false, None).unwrap();
        assert_eq!(extracted.len(), files.len());

        // Step 5: Verify contents match originals
        for (name, original_content) in &files {
            let extracted_path = extracted
                .iter()
                .find(|p| p.file_name().unwrap().to_string_lossy() == *name)
                .unwrap_or_else(|| panic!("Missing extracted file: {}", name));
            let extracted_content = fs::read(extracted_path).unwrap();
            assert_eq!(
                &extracted_content, original_content,
                "Content mismatch for {}",
                name
            );
        }
    }

    #[test]
    fn test_archive_roundtrip_cross_directory() {
        // Files in separate temp dirs (simulates no common prefix beyond root)
        let temp1 = tempdir().unwrap();
        let temp2 = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let extract_dir = tempdir().unwrap();

        let file1 = temp1.path().join("file1.txt");
        let file2 = temp2.path().join("file2.txt");
        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();

        // Create archive
        let archive_path = output_dir.path().join("test.tar.zst");
        create_tar_zstd_archive(&[&file1, &file2], &archive_path, None).unwrap();

        // Extract - should NOT fail with PathTraversal error
        let extracted =
            extract_tar_zstd_archive(&archive_path, extract_dir.path(), false, None).unwrap();

        assert_eq!(extracted.len(), 2);
    }
}

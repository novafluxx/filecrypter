# FileCrypter (User Guide)

FileCrypter is an app that password-encrypts files and can decrypt them later.

By default, the app creates a new encrypted file (with a `.encrypted` extension) and leaves your original file unchanged. It only overwrites a destination file if you turn off **Never overwrite existing files** and choose an output path that already exists.

## Quick Start

### Encrypt a file
1. Open **Encrypt**.
2. **Browse** (or drag-and-drop) a file.
3. (Optional) **Change** the output location/name.
4. (Optional) Check **Enable compression (ZSTD)** to reduce file size before encryption.
5. (Optional) Add a **Key File** for two-factor protection (see [Key File](#key-file-two-factor-encryption) below).
6. Enter a password (minimum **8** characters).
7. Click **Encrypt File**.

You'll get an output file like `example.pdf.encrypted`.

### Decrypt a file
1. Open **Decrypt**.
2. **Browse** (or drag-and-drop) your `.encrypted` file.
3. (Optional) **Change** the output location/name.
4. If the file was encrypted with a key file, **Browse** to select the same key file.
5. Enter the same password used for encryption.
6. Click **Decrypt File**.

## Batch Mode (Multiple Files)
Use **Batch** to encrypt or decrypt many files at once.

Batch supports two modes:
- **Individual files**: each file is processed separately (one output per input file).
- **Archive mode**: multiple files are bundled into one encrypted archive (`.tar.zst.encrypted`).

### Batch (Individual files)
1. Choose **Encrypt** or **Decrypt**.
2. Click **Browse** to select multiple files.
3. Choose an **Output Directory**.
4. (Optional) Add a **Key File** for two-factor protection.
5. Enter a password (the same password will be used for all selected files).
6. Click **Encrypt**/**Decrypt** to run the batch operation.

Notes:
- Batch encryption outputs `filename.ext.encrypted` into the output directory.
- Batch decryption removes `.encrypted` if present; otherwise it writes `filename.ext.decrypted`.
- **Compression is automatically enabled** for batch encryption (ZSTD) to reduce file sizes.
- Batch operations support large files (streaming), but are limited to **1000 files per run**.

### Batch (Archive mode)
Archive mode is useful when you want to move/store many files as a single encrypted file.

**Encrypt an encrypted archive**
1. Open **Batch** and choose **Encrypt**.
2. Set **Batch Mode** to **Archive mode**.
3. Click **Browse** to select multiple files.
4. Choose an **Output Directory**.
5. (Optional) Set **Archive Name** (without extension).
6. (Optional) Add a **Key File** for two-factor protection.
7. Enter a password.
8. Click **Create Encrypted Archive**.

You'll get an output file like `archive_YYYYMMDD_HHMMSS.tar.zst.encrypted`.

**Decrypt & extract an encrypted archive**
1. Open **Batch** and choose **Decrypt**.
2. Set **Batch Mode** to **Archive mode**.
3. Click **Browse** and select the single encrypted archive (`*.encrypted`).
4. Choose an **Output Directory**.
5. If the archive was encrypted with a key file, **Browse** to select the same key file.
6. Enter the password used to encrypt the archive.
7. Click **Decrypt & Extract Archive**.

## Compression
FileCrypter supports optional ZSTD compression to reduce file sizes before encryption.

- **Single file mode**: Compression is optional (checkbox in Encrypt tab). Off by default (unless enabled in **Settings**).
- **Batch mode**: Individual-file batches compress each file with ZSTD before encryption; archive mode creates a compressed TAR archive (ZSTD) and encrypts that archive.

**Compression effectiveness:**
- Text files, documents, spreadsheets: ~70% size reduction
- Already-compressed files (images, videos, archives): ~5-10% reduction
- Encryption is slightly slower when compression is enabled

**Note:** Compressed files are automatically detected and decompressed during decryption. No special action is needed.

## Key File (Two-Factor Encryption)
FileCrypter supports optional key file protection for two-factor encryption. When a key file is used, both the password **and** the key file are required to decrypt — neither alone is sufficient.

### Generating a key file
1. On the **Encrypt** tab (or **Batch Encrypt**), click **Generate** in the Key File section.
2. Choose where to save the file.
3. FileCrypter creates a file containing 32 cryptographically random bytes.

You can also use any existing file (up to 10 MB) as a key file.

### Using a key file
- **Encrypt**: Browse or generate a key file before clicking Encrypt. The key file is optional — if you skip it, encryption uses only your password.
- **Decrypt**: If the file was encrypted with a key file, you must provide the same key file. FileCrypter will display an error if a required key file is missing.
- **Batch**: Key files work in both individual and archive batch modes. The same key file applies to all files in the batch.

### Key file tips
- **Back up your key file.** If you lose it, files encrypted with it cannot be recovered (even with the correct password).
- Store the key file separately from the encrypted file (e.g., on a USB drive or a different device).
- Do not modify the key file after encryption — any change will prevent decryption.
- The Generate button is only available on encrypt tabs, not decrypt tabs (you should never need to generate a new key file when decrypting).

## Output Safety (Overwrite / Auto-Rename)
FileCrypter includes a **Never overwrite existing files** option.

- When enabled, if the destination file already exists, FileCrypter auto-renames the output to avoid overwriting (for example: `example.pdf (1).encrypted`).
- When disabled, the app is allowed to overwrite the destination file.

## Large Files & Performance
- All files use streaming (chunked) encryption to avoid loading the entire file into memory.
- Encryption and decryption speed depend on file size and your device; key derivation is intentionally slow to make password guessing harder.
- Enabling compression adds a small performance overhead but can significantly reduce output file size.

## Updates
- Some builds can notify you when an update is available.
- Click **Update Now** to download, install, and relaunch the app, or **Later** to dismiss.

## Password & Key File Tips (Important)
- There is no "password reset". If you forget the password, the encrypted file cannot be recovered.
- If you used a key file, losing that key file also means the encrypted file cannot be recovered.
- Use a long, unique password (consider a password manager).
- Keep your password private; anyone with the password (and key file, if used) and the `.encrypted` file can decrypt it.

## Troubleshooting

### "Incorrect password or corrupted file"
This usually means:
- The password is wrong, or
- The key file is wrong or missing (if one was used during encryption), or
- The encrypted file is incomplete/corrupted, or
- The file was modified after encryption.

Try decrypting again with the exact same password (and key file, if applicable), and make sure the `.encrypted` file fully copied/downloaded.

### “Permission denied”
You may not have access to read the input file or write to the destination folder.
Choose a different output location (like your Desktop/Documents) or adjust OS file permissions.

### “Too many files selected”
Batch mode supports up to **1000** files per run. Split your batch into smaller groups and try again.

### File selection won’t work / nothing happens
If you’re using drag-and-drop, try using **Browse** instead, and make sure you’re dragging a real file (not a shortcut/alias).

## Privacy & Security (High Level)
- FileCrypter performs encryption/decryption locally on your device (no cloud upload is required by the app).
- Encrypted output files include the data required to decrypt later (like a per-file salt/nonce) but do not store your password or key file.
- For safety, FileCrypter rejects symlinked paths to reduce the risk of writing to unexpected locations.

## FAQ

### Does FileCrypter replace my original file?
No. By default it writes a separate output file. Overwriting only happens if you disable **Never overwrite existing files** and choose an existing destination.

### Can I decrypt a file on a different computer?
Yes. Copy the `.encrypted` file to the other computer and decrypt it with the same password in FileCrypter. If you used a key file, you'll need to copy that as well.

### Can I rename the `.encrypted` file?
Yes. The extension is mainly for convenience. Just make sure you select the correct file when decrypting.

### Where can I find the app version?
The app version is shown in the app header.

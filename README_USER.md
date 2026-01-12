# FileCrypter (User Guide)

FileCrypter is a desktop app that password-encrypts files and can decrypt them later.

By default, the app creates a new encrypted file (with a `.encrypted` extension) and leaves your original file unchanged. It only overwrites a destination file if you turn off **Never overwrite existing files** and choose an output path that already exists.

## Quick Start

### Encrypt a file
1. Open **Encrypt**.
2. **Browse** (or drag-and-drop) a file.
3. (Optional) **Change** the output location/name.
4. (Optional) Check **Enable compression (ZSTD)** to reduce file size before encryption.
5. Enter a password (minimum **8** characters).
6. Click **Encrypt File**.

You'll get an output file like `example.pdf.encrypted`.

### Decrypt a file
1. Open **Decrypt**.
2. **Browse** (or drag-and-drop) your `.encrypted` file.
3. (Optional) **Change** the output location/name.
4. Enter the same password used for encryption.
5. Click **Decrypt File**.

## Batch Mode (Multiple Files)
Use **Batch** to encrypt or decrypt many files at once.

1. Choose **Encrypt** or **Decrypt**.
2. Click **Browse** to select multiple files.
3. Choose an **Output Directory**.
4. Enter a password (the same password will be used for all selected files).
5. Click **Encrypt**/**Decrypt** to run the batch operation.

Notes:
- Batch encryption outputs `filename.ext.encrypted` into the output directory.
- Batch decryption removes `.encrypted` if present; otherwise it writes `filename.ext.decrypted`.
- **Compression is automatically enabled** for batch encryption (ZSTD) to reduce file sizes.
- Batch operations support large files (streaming), but are limited to **1000 files per run**.

## Compression
FileCrypter supports optional ZSTD compression to reduce file sizes before encryption.

- **Single file mode**: Compression is optional (checkbox in Encrypt tab). Off by default (unless enabled in **Settings**).
- **Batch mode**: Compression is automatically enabled for all files.

**Compression effectiveness:**
- Text files, documents, spreadsheets: ~70% size reduction
- Already-compressed files (images, videos, archives): ~5-10% reduction
- Encryption is slightly slower when compression is enabled

**Note:** Compressed files are automatically detected and decompressed during decryption. No special action is needed.

## Output Safety (Overwrite / Auto-Rename)
FileCrypter includes a **Never overwrite existing files** option.

- When enabled, if the destination file already exists, FileCrypter auto-renames the output to avoid overwriting (for example: `example.pdf (1).encrypted`).
- When disabled, the app is allowed to overwrite the destination file.

## Large Files & Performance
- All files use streaming (chunked) encryption to avoid loading the entire file into memory.
- Encryption and decryption speed depend on file size and your device; key derivation is intentionally slow to make password guessing harder.
- Enabling compression adds a small performance overhead but can significantly reduce output file size.

## Password Tips (Important)
- There is no “password reset”. If you forget the password, the encrypted file cannot be recovered.
- Use a long, unique password (consider a password manager).
- Keep your password private; anyone with the password and the `.encrypted` file can decrypt it.

## Troubleshooting

### “Incorrect password or corrupted file”
This usually means:
- The password is wrong, or
- The encrypted file is incomplete/corrupted, or
- The file was modified after encryption.

Try decrypting again with the exact same password, and make sure the `.encrypted` file fully copied/downloaded.

### “Permission denied”
You may not have access to read the input file or write to the destination folder.
Choose a different output location (like your Desktop/Documents) or adjust OS file permissions.

### “Too many files selected”
Batch mode supports up to **1000** files per run. Split your batch into smaller groups and try again.

### File selection won’t work / nothing happens
If you’re using drag-and-drop, try using **Browse** instead, and make sure you’re dragging a real file (not a shortcut/alias).

## Privacy & Security (High Level)
- FileCrypter performs encryption/decryption locally on your device (no cloud upload is required by the app).
- Encrypted output files include the data required to decrypt later (like a per-file salt/nonce) but do not store your password.
- For safety, FileCrypter rejects symlinked paths to reduce the risk of writing to unexpected locations.

## FAQ

### Does FileCrypter replace my original file?
No. By default it writes a separate output file. Overwriting only happens if you disable **Never overwrite existing files** and choose an existing destination.

### Can I decrypt a file on a different computer?
Yes. Copy the `.encrypted` file to the other computer and decrypt it with the same password in FileCrypter.

### Can I rename the `.encrypted` file?
Yes. The extension is mainly for convenience. Just make sure you select the correct file when decrypting.

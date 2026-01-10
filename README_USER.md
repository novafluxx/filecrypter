# FileCrypter (User Guide)

FileCrypter is a desktop app that password-encrypts files and can decrypt them later.

The app creates a new encrypted file (with a `.encrypted` extension) and leaves your original file unchanged unless you choose to overwrite it.

## Quick Start

### Encrypt a file
1. Open **Encrypt**.
2. **Browse** (or drag-and-drop) a file.
3. (Optional) **Change** the output location/name.
4. Enter a password (minimum **8** characters).
5. Click **Encrypt File**.

You’ll get an output file like `example.pdf.encrypted`.

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
- Batch operations have a per-file size limit (currently **100 MB**). For larger files, use single-file Encrypt/Decrypt instead.

## Output Safety (Overwrite / Auto-Rename)
FileCrypter includes a **Never overwrite existing files** option.

- When enabled, if the destination file already exists, FileCrypter auto-renames the output to avoid overwriting (for example: `file (1).encrypted`).
- When disabled, the app is allowed to overwrite the destination file.

## Large Files & Performance
- Single-file Encrypt/Decrypt automatically switches to a streaming mode for files larger than about **10 MB** to avoid loading the entire file into memory.
- Encryption and decryption speed depend on file size and your device; key derivation is intentionally slow to make password guessing harder.

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

### “File is too large”
- In **Batch** mode, files larger than **100 MB** are rejected.
- Use single-file Encrypt/Decrypt for large files.

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

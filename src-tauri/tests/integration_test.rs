use std::fs;
use std::path::{Path, PathBuf};

use filecypter_lib::crypto::{
    decrypt, decrypt_file_streaming, derive_key, encrypt, encrypt_file_streaming, generate_salt,
    EncryptedFile, Password, DEFAULT_CHUNK_SIZE,
};
use tempfile::tempdir;

fn write_input_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_encrypt_decrypt_roundtrip_on_disk() {
    let dir = tempdir().unwrap();
    let input_path = write_input_file(dir.path(), "input.txt", b"secret content");
    let encrypted_path = dir.path().join("encrypted.bin");
    let decrypted_path = dir.path().join("decrypted.txt");

    let salt = generate_salt().unwrap();
    let password = Password::new("password123".to_string());
    let key = derive_key(&password, &salt).unwrap();
    let plaintext = fs::read(&input_path).unwrap();
    let (nonce, ciphertext) = encrypt(&key, &plaintext).unwrap();

    let encrypted_file = EncryptedFile {
        salt,
        nonce,
        ciphertext,
    };
    fs::write(&encrypted_path, encrypted_file.serialize()).unwrap();

    let encrypted_bytes = fs::read(&encrypted_path).unwrap();
    let parsed = EncryptedFile::deserialize(&encrypted_bytes).unwrap();
    let password = Password::new("password123".to_string());
    let key = derive_key(&password, &parsed.salt).unwrap();
    let decrypted = decrypt(&key, &parsed.nonce, &parsed.ciphertext).unwrap();
    fs::write(&decrypted_path, &decrypted).unwrap();

    let final_bytes = fs::read(&decrypted_path).unwrap();
    assert_eq!(final_bytes, b"secret content");
}

#[test]
fn test_encrypt_decrypt_wrong_password_fails() {
    let dir = tempdir().unwrap();
    let input_path = write_input_file(dir.path(), "input.txt", b"secret content");
    let encrypted_path = dir.path().join("encrypted.bin");

    let salt = generate_salt().unwrap();
    let password = Password::new("password123".to_string());
    let key = derive_key(&password, &salt).unwrap();
    let plaintext = fs::read(&input_path).unwrap();
    let (nonce, ciphertext) = encrypt(&key, &plaintext).unwrap();

    let encrypted_file = EncryptedFile {
        salt,
        nonce,
        ciphertext,
    };
    fs::write(&encrypted_path, encrypted_file.serialize()).unwrap();

    let encrypted_bytes = fs::read(&encrypted_path).unwrap();
    let parsed = EncryptedFile::deserialize(&encrypted_bytes).unwrap();
    let wrong_password = Password::new("wrong_password".to_string());
    let wrong_key = derive_key(&wrong_password, &parsed.salt).unwrap();
    let result = decrypt(&wrong_key, &parsed.nonce, &parsed.ciphertext);

    assert!(result.is_err());
}

#[test]
fn test_streaming_roundtrip_on_disk() {
    let dir = tempdir().unwrap();
    let content = vec![0x5Au8; DEFAULT_CHUNK_SIZE + 128];
    let input_path = write_input_file(dir.path(), "input.bin", &content);
    let encrypted_path = dir.path().join("encrypted.stream");
    let decrypted_path = dir.path().join("decrypted.bin");

    encrypt_file_streaming(
        &input_path,
        &encrypted_path,
        "password123",
        DEFAULT_CHUNK_SIZE,
        None,
    )
    .unwrap();

    decrypt_file_streaming(&encrypted_path, &decrypted_path, "password123", None).unwrap();

    let decrypted = fs::read(&decrypted_path).unwrap();
    assert_eq!(decrypted, content);
}

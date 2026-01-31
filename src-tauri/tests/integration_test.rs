use std::fs;
use std::path::{Path, PathBuf};

use filecrypter_lib::crypto::{
    decrypt_file_streaming, encrypt_file_streaming, Password, DEFAULT_CHUNK_SIZE,
};
use tempfile::tempdir;

fn write_input_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_streaming_roundtrip_basic() {
    let dir = tempdir().unwrap();
    let input_path = write_input_file(dir.path(), "input.txt", b"secret content");
    let encrypted_path = dir.path().join("encrypted.bin");
    let decrypted_path = dir.path().join("decrypted.txt");

    let password = Password::new("password123".to_string());
    encrypt_file_streaming(
        &input_path,
        &encrypted_path,
        &password,
        DEFAULT_CHUNK_SIZE,
        None,
        false,
        None,
        None,
    )
    .unwrap();

    decrypt_file_streaming(
        &encrypted_path,
        &decrypted_path,
        &password,
        None,
        false,
        None,
    )
    .unwrap();

    let final_bytes = fs::read(&decrypted_path).unwrap();
    assert_eq!(final_bytes, b"secret content");
}

#[test]
fn test_streaming_wrong_password_fails() {
    let dir = tempdir().unwrap();
    let input_path = write_input_file(dir.path(), "input.txt", b"secret content");
    let encrypted_path = dir.path().join("encrypted.bin");
    let decrypted_path = dir.path().join("decrypted.txt");

    let password = Password::new("password123".to_string());
    encrypt_file_streaming(
        &input_path,
        &encrypted_path,
        &password,
        DEFAULT_CHUNK_SIZE,
        None,
        false,
        None,
        None,
    )
    .unwrap();

    let wrong_password = Password::new("wrong_password".to_string());
    let result = decrypt_file_streaming(
        &encrypted_path,
        &decrypted_path,
        &wrong_password,
        None,
        false,
        None,
    );

    assert!(result.is_err());
}

#[test]
fn test_streaming_roundtrip_large_file() {
    let dir = tempdir().unwrap();
    let content = vec![0x5Au8; DEFAULT_CHUNK_SIZE + 128];
    let input_path = write_input_file(dir.path(), "input.bin", &content);
    let encrypted_path = dir.path().join("encrypted.stream");
    let decrypted_path = dir.path().join("decrypted.bin");

    let password = Password::new("password123".to_string());
    encrypt_file_streaming(
        &input_path,
        &encrypted_path,
        &password,
        DEFAULT_CHUNK_SIZE,
        None,
        false,
        None,
        None,
    )
    .unwrap();

    decrypt_file_streaming(
        &encrypted_path,
        &decrypted_path,
        &password,
        None,
        false,
        None,
    )
    .unwrap();

    let decrypted = fs::read(&decrypted_path).unwrap();
    assert_eq!(decrypted, content);
}

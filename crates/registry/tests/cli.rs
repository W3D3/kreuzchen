use std::process::Command;

use argon2::password_hash::PasswordHash;
use argon2::{Argon2, PasswordVerifier};
use base64::Engine;

#[test]
fn key_prints_matching_pair() {
    let out = Command::new(env!("CARGO_BIN_EXE_kreuzchen-registry"))
        .arg("key")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    let key = lines.next().unwrap().strip_prefix("key: ").unwrap();
    let hash = lines.next().unwrap().strip_prefix("key_hash: ").unwrap();
    assert_eq!(key.len(), 43);
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(key)
        .unwrap();
    assert_eq!(raw.len(), 32);
    let parsed = PasswordHash::new(hash).unwrap();
    assert_eq!(parsed.algorithm.as_str(), "argon2id");
    Argon2::default()
        .verify_password(key.as_bytes(), &parsed)
        .unwrap();
}

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

/// One raw HTTP/1.1 request against the running server; returns the whole response.
fn http(port: u16, method: &str, path: &str, auth: &str, body: &str) -> String {
    use std::io::{Read, Write};
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: x\r\nAuthorization: {auth}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn practice_keys_are_never_logged_or_stored() {
    use std::io::Read;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("definitions")).unwrap();
    let (_, hash) = kreuzchen_registry::new_key();
    std::fs::write(
        dir.path().join("config.yaml"),
        format!(
            "practices:\n  - user: practice_a\n    key_hash: {hash:?}\n    questionnaires: []\n"
        ),
    )
    .unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut server = Command::new(env!("CARGO_BIN_EXE_kreuzchen-registry"))
        .args(["serve", "--port", &port.to_string(), "--data"])
        .arg(dir.path())
        .env("KREUZCHEN_ADMIN_TOKEN", "cli-test-token")
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let bearer = "Bearer cli-test-token";
    let key_in = |resp: &str| {
        let body = resp.split("\r\n\r\n").nth(1).unwrap();
        let v: serde_json::Value = serde_json::from_str(body).unwrap();
        v["key"].as_str().unwrap().to_owned()
    };
    let added = http(
        port,
        "POST",
        "/admin/api/practices",
        bearer,
        r#"{"user":"practice_b"}"#,
    );
    assert!(added.starts_with("HTTP/1.1 201"), "{added}");
    let rekeyed = http(
        port,
        "POST",
        "/admin/api/practices/practice_a/key",
        bearer,
        "",
    );
    assert!(rekeyed.starts_with("HTTP/1.1 200"), "{rekeyed}");
    let keys = [key_in(&added), key_in(&rekeyed)];
    let basic = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("practice_b:{}", keys[0]))
    );
    assert!(http(port, "GET", "/v1/questionnaires", &basic, "").starts_with("HTTP/1.1 200"));

    server.kill().unwrap();
    let mut log = String::new();
    server
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut log)
        .unwrap();
    server.wait().unwrap();
    assert!(log.contains("/admin/api/practices 201"), "{log}");
    assert!(log.contains("practice_b /v1/questionnaires 200"), "{log}");
    assert!(!log.contains("cli-test-token"), "{log}");
    let cfg = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    for key in &keys {
        assert!(!log.contains(key.as_str()), "{log}");
        assert!(!cfg.contains(key.as_str()));
    }
}

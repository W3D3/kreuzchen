use std::fs;
use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use kreuzchen_registry::{hash, load, router};
use tower::ServiceExt;

const BEISPIEL: &str = include_str!("../../../examples/definitionen/beispiel.yaml");

fn def(id: &str) -> String {
    BEISPIEL.replacen("id: beispiel", &format!("id: {id:?}"), 1)
}

fn app() -> (Router, tempfile::TempDir) {
    let key_hash = Argon2::default()
        .hash_password(b"geheim", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitionen")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!("praxen:\n  - user: praxis_a\n    key_hash: {key_hash:?}\n    fragebogen: [fragebogen_a]\n"),
    )
    .unwrap();
    for id in ["fragebogen_a", "fragebogen_b"] {
        fs::write(
            dir.path().join("definitionen").join(format!("{id}.yaml")),
            def(id),
        )
        .unwrap();
    }
    let reg = load(dir.path()).unwrap();
    (router(Arc::new(reg)), dir)
}

fn basic(user: &str, key: &str) -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{user}:{key}"))
    )
}

async fn get(app: &Router, uri: &str, auth: Option<String>) -> (Response<Body>, Vec<u8>) {
    let mut r = Request::get(uri);
    if let Some(a) = auth {
        r = r.header("authorization", a);
    }
    let resp = app
        .clone()
        .oneshot(r.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = resp.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();
    (Response::from_parts(parts, Body::empty()), bytes)
}

fn ok() -> Option<String> {
    Some(basic("praxis_a", "geheim"))
}

fn no_store(r: &Response<Body>) {
    assert_eq!(r.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn healthz_is_open_and_empty() {
    let (app, _d) = app();
    let (r, b) = get(&app, "/healthz", None).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(b.is_empty());
    no_store(&r);
}

#[tokio::test]
async fn list_requires_auth() {
    let (app, _d) = app();
    let mut seen = vec![];
    for a in [
        None,
        Some(basic("nobody", "geheim")),
        Some(basic("praxis_a", "falsch")),
    ] {
        let (r, b) = get(&app, "/v1/fragebogen", a).await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert!(b.is_empty());
        no_store(&r);
        let mut h: Vec<_> = r
            .headers()
            .iter()
            .filter(|(k, _)| *k != "date")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        h.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        seen.push(h);
    }
    assert!(seen.windows(2).all(|w| w[0] == w[1]));
    assert!(!seen[0].iter().any(|(k, _)| k == "www-authenticate"));
}

#[tokio::test]
async fn list_shows_only_entitled() {
    let (app, d) = app();
    let bytes = fs::read(d.path().join("definitionen/fragebogen_a.yaml")).unwrap();
    let (r, b) = get(&app, "/v1/fragebogen", ok()).await;
    assert_eq!(r.status(), StatusCode::OK);
    no_store(&r);
    let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
    let a = v.as_array().unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0]["id"], "fragebogen_a");
    assert_eq!(a[0]["hash"], hash(&bytes));
    assert!(a[0]["title"].is_string() && a[0]["version"].is_string());
    assert_eq!(a[0].as_object().unwrap().len(), 4);
}

#[tokio::test]
async fn download_entitled() {
    let (app, d) = app();
    let bytes = fs::read(d.path().join("definitionen/fragebogen_a.yaml")).unwrap();
    let (r, b) = get(&app, "/v1/fragebogen/fragebogen_a", ok()).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["content-type"], "application/yaml");
    assert_eq!(r.headers()["x-kreuzchen-hash"], hash(&bytes).as_str());
    no_store(&r);
    assert_eq!(b, bytes);
}

#[tokio::test]
async fn not_entitled_equals_missing() {
    let (app, _d) = app();
    for id in ["fragebogen_b", "gibtsnicht"] {
        let (r, b) = get(&app, &format!("/v1/fragebogen/{id}"), ok()).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert!(b.is_empty());
        no_store(&r);
    }
}

use std::fs;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use kreuzchen_engine::hash;
use kreuzchen_registry::{load, router};
use tower::ServiceExt;

const EXAMPLE: &str = include_str!("../../../examples/definitions/beispiel.yaml");

fn def(id: &str) -> String {
    EXAMPLE.replacen("id: beispiel", &format!("id: {id:?}"), 1)
}

fn app() -> (Router, tempfile::TempDir) {
    let key_hash = Argon2::default()
        .hash_password(b"secret", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitions")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!("practices:\n  - user: practice_a\n    key_hash: {key_hash:?}\n    questionnaires: [questionnaire_a]\n"),
    )
    .unwrap();
    for id in ["questionnaire_a", "questionnaire_b"] {
        fs::write(
            dir.path().join("definitions").join(format!("{id}.yaml")),
            def(id),
        )
        .unwrap();
    }
    let reg = load(dir.path()).unwrap();
    (router(reg, None, None, Default::default()), dir)
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
    Some(basic("practice_a", "secret"))
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
        Some(basic("nobody", "secret")),
        Some(basic("practice_a", "wrong")),
    ] {
        let (r, b) = get(&app, "/v1/questionnaires", a).await;
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
    let bytes = fs::read(d.path().join("definitions/questionnaire_a.yaml")).unwrap();
    let (r, b) = get(&app, "/v1/questionnaires", ok()).await;
    assert_eq!(r.status(), StatusCode::OK);
    no_store(&r);
    let e = &load(d.path()).unwrap().entries["questionnaire_a"];
    let want = serde_json::json!([{"id": "questionnaire_a", "title": e.title, "version": e.version, "hash": hash(&bytes)}]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&b).unwrap(),
        want
    );
}

#[tokio::test]
async fn download_entitled() {
    let (app, d) = app();
    let bytes = fs::read(d.path().join("definitions/questionnaire_a.yaml")).unwrap();
    let (r, b) = get(&app, "/v1/questionnaires/questionnaire_a", ok()).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["content-type"], "application/yaml");
    assert_eq!(r.headers()["x-kreuzchen-hash"], hash(&bytes).as_str());
    no_store(&r);
    assert_eq!(b, bytes);
}

#[tokio::test]
async fn not_entitled_equals_missing() {
    let (app, _d) = app();
    for id in ["questionnaire_b", "missing_user"] {
        let (r, b) = get(&app, &format!("/v1/questionnaires/{id}"), ok()).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert!(b.is_empty());
        no_store(&r);
    }
}

#[tokio::test]
async fn unmatched_v1_paths_require_auth() {
    let (app, _d) = app();
    for uri in ["/v1/questionnaires/a/b", "/v1/nope"] {
        let (r, b) = get(&app, uri, None).await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert!(b.is_empty());
        let (r, b) = get(&app, uri, ok()).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert!(b.is_empty());
        no_store(&r);
    }
}

#[tokio::test]
async fn old_paths_of_the_0_4_app_still_work() {
    let (app, _d) = app();
    for (old, new) in [
        ("/v1/fragebogen", "/v1/questionnaires"),
        (
            "/v1/fragebogen/questionnaire_a",
            "/v1/questionnaires/questionnaire_a",
        ),
    ] {
        let (r_old, b_old) = get(&app, old, ok()).await;
        let (r_new, b_new) = get(&app, new, ok()).await;
        assert_eq!(r_old.status(), StatusCode::OK, "{old}");
        assert_eq!(
            (b_old, r_old.headers().get("x-kreuzchen-hash")),
            (b_new, r_new.headers().get("x-kreuzchen-hash"))
        );
    }
    let (r, _) = get(&app, "/v1/fragebogen", None).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let (r, _) = get(&app, "/v1/fragebogen/questionnaire_b", ok()).await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

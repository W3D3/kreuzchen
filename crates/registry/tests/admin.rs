use std::fs;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use kreuzchen_registry::{load, router, MAX_UPLOAD};
use tower::ServiceExt;

const BEISPIEL: &str = include_str!("../../../examples/definitionen/beispiel.yaml");
const TOKEN: &str = "admin-token-fuer-tests";

fn def(id: &str) -> String {
    BEISPIEL.replacen("id: beispiel", &format!("id: {id:?}"), 1)
}

fn setup(token: Option<&str>) -> (Router, tempfile::TempDir) {
    let key_hash = Argon2::default()
        .hash_password(b"geheim", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitionen")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!(
            "praxen:\n  - user: praxis_a\n    key_hash: {key_hash:?}\n    fragebogen: [fragebogen_a]\n  - user: praxis_b\n    key_hash: {key_hash:?}\n    fragebogen: []\n"
        ),
    )
    .unwrap();
    for id in ["fragebogen_a", "fragebogen_b"] {
        fs::write(dir.path().join(format!("definitionen/{id}.yaml")), def(id)).unwrap();
    }
    (router(load(dir.path()).unwrap(), token), dir)
}

fn basic() -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("praxis_a:geheim")
    )
}

fn bearer() -> String {
    format!("Bearer {TOKEN}")
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    auth: Option<String>,
    body: impl Into<Body>,
) -> (StatusCode, String) {
    let mut r = Request::builder().method(method).uri(uri);
    if let Some(a) = auth {
        r = r.header("authorization", a);
    }
    if uri.ends_with("/praxen") {
        r = r.header("content-type", "application/json");
    }
    let resp = app
        .clone()
        .oneshot(r.body(body.into()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn admin(
    app: &Router,
    method: &str,
    uri: &str,
    body: impl Into<Body>,
) -> (StatusCode, String) {
    call(app, method, uri, Some(bearer()), body).await
}

/// What practice A currently sees.
async fn practice_ids(app: &Router) -> Vec<String> {
    let (s, b) = call(app, "GET", "/v1/fragebogen", Some(basic()), Body::empty()).await;
    assert_eq!(s, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn disabled_without_token() {
    for token in [None, Some("")] {
        let (app, _d) = setup(token);
        for uri in ["/admin", "/admin/api/fragebogen"] {
            let (s, _) = call(&app, "GET", uri, Some(bearer()), Body::empty()).await;
            assert_eq!(s, StatusCode::NOT_FOUND, "{uri}");
        }
        assert_eq!(practice_ids(&app).await, ["fragebogen_a"]);
    }
}

#[tokio::test]
async fn auth_is_separate() {
    let (app, _d) = setup(Some(TOKEN));
    for auth in [
        None,
        Some(basic()),
        Some("Bearer falsch".into()),
        Some(format!("Basic {TOKEN}")),
    ] {
        let (s, _) = call(&app, "GET", "/admin/api/fragebogen", auth, Body::empty()).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let (s, _) = call(&app, "GET", "/v1/fragebogen", Some(bearer()), Body::empty()).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, b) = admin(&app, "GET", "/admin/api/fragebogen", Body::empty()).await;
    assert_eq!(s, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    assert_eq!(v["praxen"], serde_json::json!(["praxis_a", "praxis_b"]));
    assert_eq!(v["fragebogen"][0]["id"], "fragebogen_a");
    assert_eq!(
        v["fragebogen"][0]["praxen"],
        serde_json::json!(["praxis_a"])
    );
    assert!(!b.contains("argon2"));
}

#[tokio::test]
async fn page_is_served_with_token_set() {
    let (app, _d) = setup(Some(TOKEN));
    let (s, b) = call(&app, "GET", "/admin", None, Body::empty()).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains("Bibliotheksserver"));
    assert!(!b.contains(TOKEN));
}

#[tokio::test]
async fn upload_valid_then_assign() {
    let (app, d) = setup(Some(TOKEN));
    let (s, _) = admin(&app, "POST", "/admin/api/fragebogen", def("fragebogen_c")).await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(
        fs::read_to_string(d.path().join("definitionen/fragebogen_c.yaml")).unwrap(),
        def("fragebogen_c")
    );
    let (s, _) = admin(&app, "POST", "/admin/api/fragebogen", def("fragebogen_c")).await;
    assert_eq!(s, StatusCode::CONFLICT);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/fragebogen_c/praxen",
        r#"["praxis_a"]"#,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(practice_ids(&app).await, ["fragebogen_a", "fragebogen_c"]);
    let reloaded = load(d.path()).unwrap();
    assert_eq!(
        reloaded.practices[0].fragebogen,
        ["fragebogen_a", "fragebogen_c"]
    );
}

#[tokio::test]
async fn upload_invalid_is_rejected_and_not_saved() {
    let (app, d) = setup(Some(TOKEN));
    let broken = def("fragebogen_c").replacen("title:", "titel:", 1);
    for body in [broken, "id: [".into(), def("../boese")] {
        let (s, msg) = admin(&app, "POST", "/admin/api/fragebogen", body).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert!(msg.starts_with("Datei: "), "{msg}");
    }
    let mut files: Vec<_> = fs::read_dir(d.path().join("definitionen"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, ["fragebogen_a.yaml", "fragebogen_b.yaml"]);
}

#[tokio::test]
async fn upload_size_is_capped() {
    let (app, _d) = setup(Some(TOKEN));
    let big = format!("{}\n#{}", def("fragebogen_c"), "x".repeat(MAX_UPLOAD));
    let (s, _) = admin(&app, "POST", "/admin/api/fragebogen", big).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn replace_keeps_backup() {
    let (app, d) = setup(Some(TOKEN));
    let path = d.path().join("definitionen/fragebogen_a.yaml");
    let old = fs::read_to_string(&path).unwrap();
    let new = def("fragebogen_a").replacen("version:", "# neu\nversion:", 1);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/fragebogen_b",
        new.clone(),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "id mismatch");
    let (s, _) = admin(&app, "PUT", "/admin/api/fragebogen/fragebogen_a", "id: [").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(fs::read_to_string(&path).unwrap(), old);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/fragebogen_a",
        new.clone(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(fs::read_to_string(&path).unwrap(), new);
    assert_eq!(
        fs::read_to_string(path.with_extension("yaml.bak")).unwrap(),
        old
    );
    let (_, b) = call(
        &app,
        "GET",
        "/v1/fragebogen/fragebogen_a",
        Some(basic()),
        Body::empty(),
    )
    .await;
    assert_eq!(b, new);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/gibtsnicht",
        def("gibtsnicht"),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_unassigns_and_keeps_backup() {
    let (app, d) = setup(Some(TOKEN));
    let path = d.path().join("definitionen/fragebogen_a.yaml");
    let old = fs::read_to_string(&path).unwrap();
    let (s, _) = admin(
        &app,
        "DELETE",
        "/admin/api/fragebogen/fragebogen_a",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert!(!path.exists());
    assert_eq!(
        fs::read_to_string(path.with_extension("yaml.bak")).unwrap(),
        old
    );
    assert!(practice_ids(&app).await.is_empty());
    let reloaded = load(d.path()).unwrap();
    assert!(reloaded.practices[0].fragebogen.is_empty());
    assert!(!reloaded.entries.contains_key("fragebogen_a"));
    let (s, _) = admin(
        &app,
        "DELETE",
        "/admin/api/fragebogen/fragebogen_a",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn assignment_persists_and_keeps_key_hashes() {
    let (app, d) = setup(Some(TOKEN));
    let before = load(d.path()).unwrap();
    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/fragebogen_b/praxen",
        r#"["praxis_b"]"#,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/fragebogen_a/praxen",
        "[]",
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, msg) = admin(
        &app,
        "PUT",
        "/admin/api/fragebogen/fragebogen_a/praxen",
        r#"["praxis_x"]"#,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(msg.contains("praxis_x"));

    let after = load(d.path()).unwrap();
    assert!(after.practices[0].fragebogen.is_empty());
    assert_eq!(after.practices[1].fragebogen, ["fragebogen_b"]);
    for (a, b) in before.practices.iter().zip(&after.practices) {
        assert_eq!((&a.user, &a.key_hash), (&b.user, &b.key_hash));
    }
    assert!(practice_ids(&app).await.is_empty());
    assert!(!d.path().join("config.yaml.tmp").exists());
}

#[tokio::test]
async fn concurrent_writes_all_land() {
    let (app, d) = setup(Some(TOKEN));
    let ids: Vec<_> = (0..8).map(|i| format!("parallel_{i}")).collect();
    let tasks: Vec<_> = ids
        .iter()
        .map(|id| {
            let (app, id) = (app.clone(), id.clone());
            tokio::spawn(async move {
                assert_eq!(
                    admin(&app, "POST", "/admin/api/fragebogen", def(&id))
                        .await
                        .0,
                    StatusCode::CREATED
                );
                let users = r#"["praxis_a","praxis_b"]"#;
                let uri = format!("/admin/api/fragebogen/{id}/praxen");
                assert_eq!(admin(&app, "PUT", &uri, users).await.0, StatusCode::OK);
            })
        })
        .collect();
    for t in tasks {
        t.await.unwrap();
    }
    let reg = load(d.path()).unwrap();
    for p in &reg.practices {
        assert!(
            ids.iter().all(|id| p.fragebogen.contains(id)),
            "{:?}",
            p.fragebogen
        );
    }
}

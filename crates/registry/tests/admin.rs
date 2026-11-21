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

const EXAMPLE: &str = include_str!("../../../examples/definitions/beispiel.yaml");
const TOKEN: &str = "admin-token-fuer-tests";

fn def(id: &str) -> String {
    EXAMPLE.replacen("id: beispiel", &format!("id: {id:?}"), 1)
}

fn setup(token: Option<&str>) -> (Router, tempfile::TempDir) {
    let key_hash = Argon2::default()
        .hash_password(b"secret", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitions")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!(
            "practices:\n  - user: practice_a\n    key_hash: {key_hash:?}\n    questionnaires: [questionnaire_a]\n  - user: practice_b\n    key_hash: {key_hash:?}\n    questionnaires: []\n"
        ),
    )
    .unwrap();
    for id in ["questionnaire_a", "questionnaire_b"] {
        fs::write(dir.path().join(format!("definitions/{id}.yaml")), def(id)).unwrap();
    }
    (router(load(dir.path()).unwrap(), token, None), dir)
}

fn basic() -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("practice_a:secret")
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
    if uri.ends_with("/practices") {
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
    let (s, b) = call(
        app,
        "GET",
        "/v1/questionnaires",
        Some(basic()),
        Body::empty(),
    )
    .await;
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
        for uri in ["/admin", "/admin/api/questionnaires"] {
            let (s, _) = call(&app, "GET", uri, Some(bearer()), Body::empty()).await;
            assert_eq!(s, StatusCode::NOT_FOUND, "{uri}");
        }
        assert_eq!(practice_ids(&app).await, ["questionnaire_a"]);
    }
}

#[tokio::test]
async fn auth_is_separate() {
    let (app, _d) = setup(Some(TOKEN));
    for auth in [
        None,
        Some(basic()),
        Some("Bearer wrong".into()),
        Some(format!("Basic {TOKEN}")),
    ] {
        let (s, _) = call(
            &app,
            "GET",
            "/admin/api/questionnaires",
            auth,
            Body::empty(),
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let (s, _) = call(
        &app,
        "GET",
        "/v1/questionnaires",
        Some(bearer()),
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, b) = admin(&app, "GET", "/admin/api/questionnaires", Body::empty()).await;
    assert_eq!(s, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    assert_eq!(
        v["practices"],
        serde_json::json!(["practice_a", "practice_b"])
    );
    assert_eq!(v["questionnaires"][0]["id"], "questionnaire_a");
    assert_eq!(
        v["questionnaires"][0]["practices"],
        serde_json::json!(["practice_a"])
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
    let (s, _) = admin(
        &app,
        "POST",
        "/admin/api/questionnaires",
        def("questionnaire_c"),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(
        fs::read_to_string(d.path().join("definitions/questionnaire_c.yaml")).unwrap(),
        def("questionnaire_c")
    );
    let (s, _) = admin(
        &app,
        "POST",
        "/admin/api/questionnaires",
        def("questionnaire_c"),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/questionnaire_c/practices",
        r#"["practice_a"]"#,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        practice_ids(&app).await,
        ["questionnaire_a", "questionnaire_c"]
    );
    let reloaded = load(d.path()).unwrap();
    assert_eq!(
        reloaded.practices[0].questionnaires,
        ["questionnaire_a", "questionnaire_c"]
    );
}

#[tokio::test]
async fn upload_invalid_is_rejected_and_not_saved() {
    let (app, d) = setup(Some(TOKEN));
    let broken = def("questionnaire_c").replacen("title:", "titel:", 1);
    for body in [broken, "id: [".into(), def("../boese")] {
        let (s, msg) = admin(&app, "POST", "/admin/api/questionnaires", body).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert!(msg.starts_with("Datei: "), "{msg}");
    }
    let mut files: Vec<_> = fs::read_dir(d.path().join("definitions"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, ["questionnaire_a.yaml", "questionnaire_b.yaml"]);
}

#[tokio::test]
async fn upload_size_is_capped() {
    let (app, _d) = setup(Some(TOKEN));
    let big = format!("{}\n#{}", def("questionnaire_c"), "x".repeat(MAX_UPLOAD));
    let (s, _) = admin(&app, "POST", "/admin/api/questionnaires", big).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn replace_keeps_backup() {
    let (app, d) = setup(Some(TOKEN));
    let path = d.path().join("definitions/questionnaire_a.yaml");
    let old = fs::read_to_string(&path).unwrap();
    let new = def("questionnaire_a").replacen("version:", "# new\nversion:", 1);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/questionnaire_b",
        new.clone(),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "id mismatch");
    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/questionnaire_a",
        "id: [",
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(fs::read_to_string(&path).unwrap(), old);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/questionnaire_a",
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
        "/v1/questionnaires/questionnaire_a",
        Some(basic()),
        Body::empty(),
    )
    .await;
    assert_eq!(b, new);

    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/missing_user",
        def("missing_user"),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_unassigns_and_keeps_backup() {
    let (app, d) = setup(Some(TOKEN));
    let path = d.path().join("definitions/questionnaire_a.yaml");
    let old = fs::read_to_string(&path).unwrap();
    let (s, _) = admin(
        &app,
        "DELETE",
        "/admin/api/questionnaires/questionnaire_a",
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
    assert!(reloaded.practices[0].questionnaires.is_empty());
    assert!(!reloaded.entries.contains_key("questionnaire_a"));
    let (s, _) = admin(
        &app,
        "DELETE",
        "/admin/api/questionnaires/questionnaire_a",
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
        "/admin/api/questionnaires/questionnaire_b/practices",
        r#"["practice_b"]"#,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/questionnaire_a/practices",
        "[]",
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, msg) = admin(
        &app,
        "PUT",
        "/admin/api/questionnaires/questionnaire_a/practices",
        r#"["practice_x"]"#,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(msg.contains("practice_x"));

    let after = load(d.path()).unwrap();
    assert!(after.practices[0].questionnaires.is_empty());
    assert_eq!(after.practices[1].questionnaires, ["questionnaire_b"]);
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
                    admin(&app, "POST", "/admin/api/questionnaires", def(&id))
                        .await
                        .0,
                    StatusCode::CREATED
                );
                let users = r#"["practice_a","practice_b"]"#;
                let uri = format!("/admin/api/questionnaires/{id}/practices");
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
            ids.iter().all(|id| p.questionnaires.contains(id)),
            "{:?}",
            p.questionnaires
        );
    }
}

#[tokio::test]
async fn writes_build_on_hand_edits_since_startup() {
    let (app, d) = setup(Some(TOKEN));
    let cfg = d.path().join("config.yaml");
    let hash = load(d.path()).unwrap().practices[0].key_hash.clone();
    let mut text = fs::read_to_string(&cfg).unwrap();
    text += &format!(
        "  - user: practice_new\n    key_hash: {hash:?}\n    questionnaires: [questionnaire_b]\n"
    );
    fs::write(&cfg, text).unwrap();

    let uri = "/admin/api/questionnaires/questionnaire_a/practices";
    let (s, _) = admin(&app, "PUT", uri, r#"["practice_new"]"#).await;
    assert_eq!(s, StatusCode::OK);
    let reg = load(d.path()).unwrap();
    let added = reg
        .practices
        .iter()
        .find(|p| p.user == "practice_new")
        .unwrap();
    assert_eq!(added.key_hash, hash);
    assert_eq!(added.questionnaires, ["questionnaire_b", "questionnaire_a"]);
}

#[tokio::test]
async fn broken_hand_edit_stops_write_before_any_change() {
    let (app, d) = setup(Some(TOKEN));
    let cfg = d.path().join("config.yaml");
    fs::write(&cfg, "practices: [kaputt").unwrap();
    let path = d.path().join("definitions/questionnaire_a.yaml");
    let (s, msg) = admin(
        &app,
        "DELETE",
        "/admin/api/questionnaires/questionnaire_a",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(msg.contains("config.yaml"), "{msg}");
    assert!(path.exists());
    assert_eq!(fs::read_to_string(&cfg).unwrap(), "practices: [kaputt");
    // Practices keep being served from the last good state.
    assert_eq!(practice_ids(&app).await, ["questionnaire_a"]);
}

fn basic_for(user: &str, key: &str) -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{user}:{key}"))
    )
}

async fn practice_status(app: &Router, user: &str, key: &str) -> StatusCode {
    call(
        app,
        "GET",
        "/v1/questionnaires",
        Some(basic_for(user, key)),
        Body::empty(),
    )
    .await
    .0
}

fn key_of(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    v["key"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn add_practice_validates() {
    let (app, d) = setup(Some(TOKEN));
    let cfg = fs::read_to_string(d.path().join("config.yaml")).unwrap();
    let long = "a".repeat(65);
    for (body, want) in [
        (
            r#"{"user":"Practice_C"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        (r#"{"user":""}"#.to_owned(), StatusCode::BAD_REQUEST),
        (
            r#"{"user":"practice c"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        (
            r#"{"user":"practice:c"}"#.to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        (format!(r#"{{"user":"{long}"}}"#), StatusCode::BAD_REQUEST),
        (
            r#"{"user":"practice_c","questionnaires":["missing_user"]}"#.to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        (
            r#"{"user":"practice_c","key_hash":"x"}"#.to_owned(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (r#"{"user":"practice_a"}"#.to_owned(), StatusCode::CONFLICT),
    ] {
        let (s, _) = call_json(&app, "POST", "/admin/api/practices", body.clone()).await;
        assert_eq!(s, want, "{body}");
    }
    assert_eq!(
        fs::read_to_string(d.path().join("config.yaml")).unwrap(),
        cfg
    );
}

async fn call_json(app: &Router, method: &str, uri: &str, body: String) -> (StatusCode, String) {
    let r = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", bearer())
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(r).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn add_practice_stores_only_the_hash() {
    let (app, d) = setup(Some(TOKEN));
    let body = r#"{"user":"practice_new-2","questionnaires":["questionnaire_b"]}"#.to_owned();
    let (s, b) = call_json(&app, "POST", "/admin/api/practices", body).await;
    assert_eq!(s, StatusCode::CREATED);
    let key = key_of(&b);
    assert_eq!(key.len(), 43);

    let cfg = fs::read_to_string(d.path().join("config.yaml")).unwrap();
    assert!(!cfg.contains(&key));
    let reg = load(d.path()).unwrap();
    let p = reg
        .practices
        .iter()
        .find(|p| p.user == "practice_new-2")
        .unwrap();
    assert_eq!(p.questionnaires, ["questionnaire_b"]);
    assert!(p.key_hash.starts_with("$argon2id$"));

    assert_eq!(
        practice_status(&app, "practice_new-2", &key).await,
        StatusCode::OK
    );
    let (_, list) = call(
        &app,
        "GET",
        "/v1/questionnaires",
        Some(basic_for("practice_new-2", &key)),
        Body::empty(),
    )
    .await;
    assert!(list.contains("questionnaire_b") && !list.contains("questionnaire_a"));
    let (_, admin_list) = admin(&app, "GET", "/admin/api/questionnaires", Body::empty()).await;
    assert!(!admin_list.contains(&key) && !admin_list.contains("argon2"));
}

#[tokio::test]
async fn new_key_replaces_old_at_once() {
    let (app, d) = setup(Some(TOKEN));
    assert_eq!(
        practice_status(&app, "practice_a", "secret").await,
        StatusCode::OK
    );
    let (s, b) = admin(
        &app,
        "POST",
        "/admin/api/practices/practice_a/key",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let key = key_of(&b);
    assert_eq!(
        practice_status(&app, "practice_a", "secret").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        practice_status(&app, "practice_a", &key).await,
        StatusCode::OK
    );
    assert!(!fs::read_to_string(d.path().join("config.yaml"))
        .unwrap()
        .contains(&key));
    // Other fields of the practice are kept.
    assert_eq!(
        load(d.path()).unwrap().practices[0].questionnaires,
        ["questionnaire_a"]
    );

    let (s, _) = admin(
        &app,
        "POST",
        "/admin/api/practices/missing_user/key",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_practice() {
    let (app, d) = setup(Some(TOKEN));
    let (s, _) = admin(
        &app,
        "DELETE",
        "/admin/api/practices/practice_a",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(
        practice_status(&app, "practice_a", "secret").await,
        StatusCode::UNAUTHORIZED
    );
    let reg = load(d.path()).unwrap();
    assert_eq!(reg.practices.len(), 1);
    assert_eq!(reg.practices[0].user, "practice_b");
    let (s, _) = admin(
        &app,
        "DELETE",
        "/admin/api/practices/practice_a",
        Body::empty(),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn practice_routes_need_the_admin_token() {
    let (app, d) = setup(Some(TOKEN));
    let cfg = fs::read_to_string(d.path().join("config.yaml")).unwrap();
    for (method, uri) in [
        ("POST", "/admin/api/practices"),
        ("POST", "/admin/api/practices/practice_a/key"),
        ("DELETE", "/admin/api/practices/practice_a"),
    ] {
        let (s, _) = call(&app, method, uri, Some(basic()), r#"{"user":"x"}"#).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
    assert_eq!(
        fs::read_to_string(d.path().join("config.yaml")).unwrap(),
        cfg
    );
}

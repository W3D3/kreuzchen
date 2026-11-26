use std::fs;
use std::path::Path;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use kreuzchen_registry::{load, router, MAX_TICKET, TICKETS_PER_HOUR};
use serde_json::{json, Value};
use tower::ServiceExt;

const TOKEN: &str = "admin-token-fuer-tests";

fn setup() -> tempfile::TempDir {
    let key_hash = Argon2::default()
        .hash_password(b"secret", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitions")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!(
            "practices:\n  - user: practice_a\n    key_hash: {key_hash:?}\n    questionnaires: []\n  - user: practice_b\n    key_hash: {key_hash:?}\n    questionnaires: []\n"
        ),
    )
    .unwrap();
    dir
}

fn app(dir: &Path) -> Router {
    router(load(dir).unwrap(), Some(TOKEN), None, Default::default())
}

fn basic(user: &str) -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{user}:secret"))
    )
}

fn bearer() -> String {
    format!("Bearer {TOKEN}")
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    auth: &str,
    body: impl Into<Body>,
) -> (StatusCode, String) {
    let r = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", auth)
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap();
    let resp = app.clone().oneshot(r).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn ticket() -> Value {
    json!({
        "type": "bug",
        "subject": "Skala falsch",
        "message": "Die Summe stimmt nicht.",
        "reply_email": "praxis@example.org",
        "questionnaire": {"id": "beispiel", "title": "Beispiel", "version": "1", "hash": "abc"},
        "app_version": "0.6.0",
        "os": "linux"
    })
}

async fn send(app: &Router, user: &str, t: &Value) -> (StatusCode, String) {
    call(app, "POST", "/v1/tickets", &basic(user), t.to_string()).await
}

async fn admin_list(app: &Router, query: &str) -> Vec<Value> {
    let (s, body) = call(
        app,
        "GET",
        &format!("/admin/api/tickets{query}"),
        &bearer(),
        "",
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn create_persists_and_survives_reload() {
    let dir = setup();
    let a = app(dir.path());
    let (s, body) = send(&a, "practice_a", &ticket()).await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["id"], 1);
    let (_, body) = send(&a, "practice_a", &ticket()).await;
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["id"], 2);
    assert!(dir.path().join("tickets/000001.json").is_file());

    // A fresh server on the same data dir sees both, with every stored field.
    let list = admin_list(&app(dir.path()), "").await;
    assert_eq!(list.len(), 2);
    let t = &list[1];
    assert_eq!(t["id"], 1);
    assert_eq!(t["practice"], "practice_a");
    assert_eq!(t["type"], "bug");
    assert_eq!(t["status"], "open");
    assert_eq!(t["admin_note"], "");
    assert_eq!(t["reply_email"], "praxis@example.org");
    assert_eq!(t["questionnaire"]["id"], "beispiel");
    assert_eq!(t["app_version"], "0.6.0");
    assert!(t["created_at"].as_str().unwrap().ends_with('Z'));
}

#[tokio::test]
async fn credentials_stay_on_their_side() {
    let dir = setup();
    let a = app(dir.path());
    assert_eq!(
        call(&a, "POST", "/v1/tickets", &bearer(), ticket().to_string())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&a, "GET", "/v1/tickets", &bearer(), "").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&a, "GET", "/admin/api/tickets", &basic("practice_a"), "")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &a,
            "PATCH",
            "/admin/api/tickets/1",
            &basic("practice_a"),
            "{}"
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&a, "POST", "/v1/tickets", "", ticket().to_string())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn practices_see_only_their_own_tickets_without_note() {
    let dir = setup();
    let a = app(dir.path());
    send(&a, "practice_a", &ticket()).await;
    let mut t = ticket();
    t["subject"] = "Von B".into();
    send(&a, "practice_b", &t).await;
    call(
        &a,
        "PATCH",
        "/admin/api/tickets/1",
        &bearer(),
        r#"{"admin_note":"intern"}"#,
    )
    .await;

    let (s, body) = call(&a, "GET", "/v1/tickets", &basic("practice_b"), "").await;
    assert_eq!(s, StatusCode::OK);
    let list: Vec<Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["subject"], "Von B");
    let (_, body) = call(&a, "GET", "/v1/tickets", &basic("practice_a"), "").await;
    let list: Vec<Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], 1);
    assert!(!body.contains("intern"), "{body}");
    // Its own text and thread, but not the reply email or the internal note.
    assert_eq!(list[0]["message"], "Die Summe stimmt nicht.");
    assert_eq!(list[0]["replies"], json!([]));
    assert!(list[0].get("reply_email").is_none() && list[0].get("admin_note").is_none());
}

#[tokio::test]
async fn validation_and_limits() {
    let dir = setup();
    let a = app(dir.path());
    let with = |k: &str, v: Value| {
        let mut t = ticket();
        t[k] = v;
        t
    };
    for (t, want) in [
        (with("subject", "  ".into()), StatusCode::BAD_REQUEST),
        (
            with("subject", "x".repeat(201).into()),
            StatusCode::BAD_REQUEST,
        ),
        (with("subject", "a\nb".into()), StatusCode::BAD_REQUEST),
        (with("message", "".into()), StatusCode::BAD_REQUEST),
        (
            with("message", "x".repeat(10_001).into()),
            StatusCode::BAD_REQUEST,
        ),
        (
            with("reply_email", "keine-adresse".into()),
            StatusCode::BAD_REQUEST,
        ),
        (
            with("type", "spam".into()),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        // Closed shape: answers or a subject id cannot ride along.
        (
            with("answers", json!({"1": 3})),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            with("subject_id", "K-17".into()),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            with(
                "questionnaire",
                json!({"id": "../x", "title": "t", "version": "1", "hash": "h"}),
            ),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (s, body) = send(&a, "practice_a", &t).await;
        assert_eq!(s, want, "{t} → {body}");
        assert!(!body.contains("xxxxxxxx"), "error echoes content: {body}");
    }
    let big = with("message", "x".repeat(MAX_TICKET).into());
    assert_eq!(
        send(&a, "practice_a", &big).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // Optional fields may be left out; an empty email counts as none.
    let mut t = with("reply_email", "".into());
    t.as_object_mut().unwrap().remove("questionnaire");
    assert_eq!(send(&a, "practice_a", &t).await.0, StatusCode::CREATED);
    let list = admin_list(&a, "").await;
    assert_eq!(list.len(), 1);
    assert!(list[0]["reply_email"].is_null() && list[0]["questionnaire"].is_null());
}

#[tokio::test]
async fn rate_limit_per_practice() {
    let dir = setup();
    let a = app(dir.path());
    for _ in 0..TICKETS_PER_HOUR {
        assert_eq!(
            send(&a, "practice_a", &ticket()).await.0,
            StatusCode::CREATED
        );
    }
    assert_eq!(
        send(&a, "practice_a", &ticket()).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        send(&a, "practice_b", &ticket()).await.0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn admin_updates_status_and_note_and_filters() {
    let dir = setup();
    let a = app(dir.path());
    send(&a, "practice_a", &ticket()).await;
    let mut general = ticket();
    general.as_object_mut().unwrap().remove("questionnaire");
    send(&a, "practice_a", &general).await;
    send(&a, "practice_b", &ticket()).await;

    let (s, body) = call(
        &a,
        "PATCH",
        "/admin/api/tickets/3",
        &bearer(),
        r#"{"status":"done","admin_note":"behoben in 0.7"}"#,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    let t: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (t["status"].as_str(), t["admin_note"].as_str()),
        (Some("done"), Some("behoben in 0.7"))
    );

    let ids = |l: Vec<Value>| {
        l.iter()
            .map(|t| t["id"].as_u64().unwrap())
            .collect::<Vec<_>>()
    };
    // Open first, newest first within a status; persisted across a restart.
    let b = app(dir.path());
    assert_eq!(ids(admin_list(&b, "").await), [2, 1, 3]);
    assert_eq!(ids(admin_list(&b, "?status=open").await), [2, 1]);
    assert_eq!(ids(admin_list(&b, "?status=done").await), [3]);
    assert_eq!(ids(admin_list(&b, "?questionnaire=beispiel").await), [1, 3]);
    assert_eq!(ids(admin_list(&b, "?questionnaire=").await), [2]);
    assert_eq!(
        ids(admin_list(&b, "?status=open&questionnaire=beispiel").await),
        [1]
    );

    // Status alone keeps the note.
    call(
        &b,
        "PATCH",
        "/admin/api/tickets/3",
        &bearer(),
        r#"{"status":"open"}"#,
    )
    .await;
    assert_eq!(
        admin_list(&b, "?status=open").await[0]["admin_note"],
        "behoben in 0.7"
    );

    for (uri, body, want) in [
        (
            "/admin/api/tickets/99",
            r#"{"status":"done"}"#,
            StatusCode::NOT_FOUND,
        ),
        (
            "/admin/api/tickets/1",
            r#"{"status":"weg"}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/admin/api/tickets/1",
            r#"{"practice":"x"}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        assert_eq!(
            call(&b, "PATCH", uri, &bearer(), body).await.0,
            want,
            "{uri} {body}"
        );
    }
    let note = json!({"admin_note": "x".repeat(10_001)}).to_string();
    assert_eq!(
        call(&b, "PATCH", "/admin/api/tickets/1", &bearer(), note)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

async fn reply(app: &Router, auth: &str, id: u64, message: &str) -> (StatusCode, String) {
    let uri = if auth.starts_with("Bearer") {
        format!("/admin/api/tickets/{id}/replies")
    } else {
        format!("/v1/tickets/{id}/replies")
    };
    let body = json!({ "message": message }).to_string();
    call(app, "POST", &uri, auth, body).await
}

async fn own_list(app: &Router, user: &str) -> Vec<Value> {
    let (s, body) = call(app, "GET", "/v1/tickets", &basic(user), "").await;
    assert_eq!(s, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn thread_persists_and_practice_reply_reopens() {
    let dir = setup();
    let a = app(dir.path());
    send(&a, "practice_a", &ticket()).await;

    let (s, body) = reply(&a, &bearer(), 1, " Bitte Version prüfen. ").await;
    assert_eq!(s, StatusCode::OK, "{body}");
    let t: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(t["replies"][0]["author"], "admin");
    assert_eq!(t["replies"][0]["message"], "Bitte Version prüfen.");
    call(
        &a,
        "PATCH",
        "/admin/api/tickets/1",
        &bearer(),
        r#"{"status":"done"}"#,
    )
    .await;

    // An admin reply leaves the status alone; a practice reply re-opens a done ticket.
    reply(&a, &bearer(), 1, "Nachtrag").await;
    assert_eq!(admin_list(&a, "").await[0]["status"], "done");
    let (s, body) = reply(&a, &basic("practice_a"), 1, "Ist 0.7.0").await;
    assert_eq!(s, StatusCode::OK, "{body}");
    let t: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(t["status"], "open");
    assert!(t.get("admin_note").is_none() && t.get("reply_email").is_none());

    // A fresh server sees the whole thread, oldest first, numbered within the ticket.
    let b = app(dir.path());
    let mine = own_list(&b, "practice_a").await;
    let replies = mine[0]["replies"].as_array().unwrap();
    let summary: Vec<_> = replies
        .iter()
        .map(|r| (r["id"].as_u64().unwrap(), r["author"].as_str().unwrap()))
        .collect();
    assert_eq!(summary, [(1, "admin"), (2, "admin"), (3, "practice")]);
    assert!(replies[2]["created_at"].as_str().unwrap().ends_with('Z'));
    assert_eq!(
        admin_list(&b, "").await[0]["replies"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn replies_only_on_own_tickets_with_limits() {
    let dir = setup();
    let a = app(dir.path());
    send(&a, "practice_a", &ticket()).await;
    let before = fs::read_to_string(dir.path().join("tickets/000001.json")).unwrap();

    // Someone else's ticket and a missing one look the same.
    assert_eq!(
        reply(&a, &basic("practice_b"), 1, "fremd").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        reply(&a, &basic("practice_a"), 99, "x").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(reply(&a, &bearer(), 99, "x").await.0, StatusCode::NOT_FOUND);
    for (m, want) in [
        ("  ", StatusCode::BAD_REQUEST),
        (&*"x".repeat(10_001), StatusCode::BAD_REQUEST),
    ] {
        let (s, body) = reply(&a, &basic("practice_a"), 1, m).await;
        assert_eq!(s, want);
        assert!(!body.contains("xxxxxxxx"), "error echoes content: {body}");
        assert_eq!(reply(&a, &bearer(), 1, m).await.0, want);
    }
    let big = json!({ "message": "x".repeat(MAX_TICKET) }).to_string();
    let uri = "/v1/tickets/1/replies";
    assert_eq!(
        call(&a, "POST", uri, &basic("practice_a"), big).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let extra = json!({ "message": "m", "author": "admin" }).to_string();
    assert_eq!(
        call(&a, "POST", uri, &basic("practice_a"), extra).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("tickets/000001.json")).unwrap(),
        before
    );

    // Its own hourly budget, separate from new tickets; admins are not limited.
    for _ in 0..TICKETS_PER_HOUR {
        assert_eq!(
            reply(&a, &basic("practice_a"), 1, "m").await.0,
            StatusCode::OK
        );
    }
    assert_eq!(
        reply(&a, &basic("practice_a"), 1, "m").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        send(&a, "practice_a", &ticket()).await.0,
        StatusCode::CREATED
    );
    assert_eq!(reply(&a, &bearer(), 1, "admin").await.0, StatusCode::OK);
}

#[tokio::test]
async fn reply_credentials_stay_on_their_side() {
    let dir = setup();
    let a = app(dir.path());
    send(&a, "practice_a", &ticket()).await;
    let body = || json!({ "message": "m" }).to_string();
    for (uri, auth) in [
        ("/v1/tickets/1/replies", bearer()),
        ("/v1/tickets/1/replies", String::new()),
        ("/admin/api/tickets/1/replies", basic("practice_a")),
        ("/admin/api/tickets/1/replies", String::new()),
        ("/admin/api/notify", basic("practice_a")),
        ("/admin/api/notify/test", basic("practice_a")),
    ] {
        assert_eq!(
            call(&a, "POST", uri, &auth, body()).await.0,
            StatusCode::UNAUTHORIZED,
            "{uri} {auth}"
        );
    }
    assert!(admin_list(&a, "").await[0]["replies"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn tickets_from_before_replies_still_load() {
    let dir = setup();
    fs::create_dir(dir.path().join("tickets")).unwrap();
    let old = json!({"id": 7, "created_at": "2026-11-20T10:00:00Z", "practice": "practice_a", "type": "bug",
        "subject": "s", "message": "m", "reply_email": null, "questionnaire": null, "app_version": "0.7.0",
        "os": "linux", "status": "done", "admin_note": ""});
    fs::write(dir.path().join("tickets/000007.json"), old.to_string()).unwrap();
    let a = app(dir.path());
    assert_eq!(own_list(&a, "practice_a").await[0]["replies"], json!([]));
    assert_eq!(
        reply(&a, &basic("practice_a"), 7, "noch da?").await.0,
        StatusCode::OK
    );
    assert_eq!(admin_list(&a, "").await[0]["status"], "open");
}

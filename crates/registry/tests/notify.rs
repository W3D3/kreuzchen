//! Notification delivery against a local mock of Discord, Telegram and a JSON webhook.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::extract::{Path as UrlPath, State};
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use kreuzchen_registry::{load, router, Notify, Telegram};
use serde_json::{json, Value};
use tower::ServiceExt;

const ADMIN: &str = "admin-token-fuer-tests";
const BOT_TOKEN: &str = "123456:bot-secret-token";
const HOOK_SECRET: &str = "discord-webhook-secret";

#[derive(Default)]
struct Mock {
    /// `(path, body)` of every request received.
    got: Vec<(String, Value)>,
    /// Answer this status for the next `fail` requests, then 200.
    fail: usize,
    status: u16,
    /// Delay every answer, to prove nothing waits for it.
    delay: Duration,
}

type Shared = Arc<Mutex<Mock>>;

async fn receive(
    State(m): State<Shared>,
    UrlPath(path): UrlPath<String>,
    body: String,
) -> StatusCode {
    let delay = {
        let mut m = m.lock().unwrap();
        m.got.push((path, serde_json::from_str(&body).unwrap()));
        m.delay
    };
    tokio::time::sleep(delay).await;
    let mut m = m.lock().unwrap();
    if m.fail > 0 {
        m.fail -= 1;
        return StatusCode::from_u16(m.status).unwrap();
    }
    StatusCode::OK
}

async fn mock(m: Mock) -> (String, Shared) {
    let m = Arc::new(Mutex::new(m));
    let app = Router::new()
        .route("/{*path}", post(receive))
        .with_state(m.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (url, m)
}

fn notify(url: &str, public_url: Option<&str>) -> Notify {
    Notify {
        discord_webhook: Some(format!("{url}/discord/{HOOK_SECRET}")),
        telegram: Some(Telegram {
            token: BOT_TOKEN.into(),
            chat_id: "4711".into(),
            api_url: format!("{url}/tg"),
        }),
        webhook: Some(format!("{url}/hook")),
        public_url: public_url.map(str::to_owned),
        backoff: vec![Duration::from_millis(50); 2],
    }
}

fn setup() -> tempfile::TempDir {
    let key_hash = Argon2::default()
        .hash_password(b"secret", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitions")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!("practices:\n  - user: praxis_mitsche\n    key_hash: {key_hash:?}\n    questionnaires: []\n"),
    )
    .unwrap();
    dir
}

fn app(dir: &Path, n: Notify) -> Router {
    router(load(dir).unwrap(), Some(ADMIN), None, n)
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    auth: &str,
    body: String,
) -> (StatusCode, String) {
    let r = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", auth)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(r).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn practice() -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("praxis_mitsche:secret")
    )
}

fn bearer() -> String {
    format!("Bearer {ADMIN}")
}

const SUBJECT: &str = "Betreff-Geheim";
const MESSAGE: &str = "Nachricht-Geheim";
const EMAIL: &str = "geheim@example.org";

async fn new_ticket(a: &Router) -> (StatusCode, String) {
    let t = json!({"type": "bug", "subject": SUBJECT, "message": MESSAGE, "reply_email": EMAIL,
        "questionnaire": {"id": "srs2", "title": "SRS-2 4–18", "version": "2", "hash": "h"},
        "app_version": "0.7.0", "os": "linux"});
    call(a, "POST", "/v1/tickets", &practice(), t.to_string()).await
}

/// Waits until the mock got `n` requests.
async fn wait_for(m: &Shared, n: usize) -> Vec<(String, Value)> {
    let start = Instant::now();
    loop {
        let got = m.lock().unwrap().got.clone();
        if got.len() >= n || start.elapsed() > Duration::from_secs(10) {
            return got;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn find<'a>(got: &'a [(String, Value)], prefix: &str) -> Vec<&'a Value> {
    got.iter()
        .filter(|(p, _)| p.starts_with(prefix))
        .map(|(_, b)| b)
        .collect()
}

#[tokio::test]
async fn new_ticket_and_practice_reply_notify_every_channel() {
    let (url, m) = mock(Mock::default()).await;
    let dir = setup();
    let a = app(dir.path(), notify(&url, Some("https://bib.example/")));
    assert_eq!(new_ticket(&a).await.0, StatusCode::CREATED);
    let got = wait_for(&m, 3).await;
    assert_eq!(got.len(), 3, "{got:?}");

    let text = "Neues Ticket #1 – praxis_mitsche – Fehler – SRS-2 4–18 (v2)\nhttps://bib.example/admin#ticket-1";
    let discord = find(&got, &format!("discord/{HOOK_SECRET}"));
    assert_eq!(
        discord,
        [&json!({"content": text, "allowed_mentions": {"parse": []}})]
    );
    let tg = find(&got, &format!("tg/bot{BOT_TOKEN}/sendMessage"));
    assert_eq!(
        tg,
        [&json!({"chat_id": "4711", "text": text, "disable_web_page_preview": true})]
    );
    let hook = find(&got, "hook");
    assert_eq!(hook[0]["event"], "new_ticket");
    assert_eq!(hook[0]["ticket_id"], 1);
    assert_eq!(hook[0]["url"], "https://bib.example/admin#ticket-1");

    // Admin replies and status changes stay quiet; a practice reply notifies.
    let body = json!({"message": "Antwort-Geheim"}).to_string();
    call(
        &a,
        "POST",
        "/admin/api/tickets/1/replies",
        &bearer(),
        body.clone(),
    )
    .await;
    call(
        &a,
        "PATCH",
        "/admin/api/tickets/1",
        &bearer(),
        r#"{"status":"done"}"#.into(),
    )
    .await;
    let (s, _) = call(&a, "POST", "/v1/tickets/1/replies", &practice(), body).await;
    assert_eq!(s, StatusCode::OK);
    let got = wait_for(&m, 6).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let got_after = m.lock().unwrap().got.clone();
    assert_eq!(got_after.len(), 6, "{got_after:?}");
    let replies: Vec<_> = find(&got[3..], "tg")
        .into_iter()
        .map(|b| b["text"].clone())
        .collect();
    assert_eq!(
        replies,
        [json!(
            "Neue Antwort auf Ticket #1 – praxis_mitsche\nhttps://bib.example/admin#ticket-1"
        )]
    );
    assert_eq!(find(&got[3..], "hook")[0]["event"], "new_reply");

    // Metadata only, in every request ever sent.
    let all = serde_json::to_string(&got_after).unwrap();
    for s in [SUBJECT, MESSAGE, EMAIL, "Antwort-Geheim"] {
        assert!(!all.contains(s), "{s} in {all}");
    }
}

#[tokio::test]
async fn without_public_url_there_is_no_link() {
    let (url, m) = mock(Mock::default()).await;
    let dir = setup();
    let a = app(dir.path(), notify(&url, None));
    new_ticket(&a).await;
    let got = wait_for(&m, 3).await;
    assert_eq!(
        find(&got, "tg")[0]["text"],
        "Neues Ticket #1 – praxis_mitsche – Fehler – SRS-2 4–18 (v2)"
    );
    assert!(find(&got, "hook")[0]["url"].is_null());
}

#[tokio::test]
async fn retries_then_delivers() {
    let (url, m) = mock(Mock {
        fail: 2,
        status: 503,
        ..Mock::default()
    })
    .await;
    let dir = setup();
    let mut n = notify(&url, None);
    n.discord_webhook = None;
    n.webhook = None;
    let a = app(dir.path(), n);
    new_ticket(&a).await;
    // 503, 503, then 200: three requests, and no fourth.
    assert_eq!(wait_for(&m, 3).await.len(), 3);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(m.lock().unwrap().got.len(), 3);
}

#[tokio::test]
async fn client_errors_are_not_retried() {
    let (url, m) = mock(Mock {
        fail: 5,
        status: 404,
        ..Mock::default()
    })
    .await;
    let dir = setup();
    let mut n = notify(&url, None);
    n.telegram = None;
    n.webhook = None;
    let a = app(dir.path(), n);
    new_ticket(&a).await;
    wait_for(&m, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(m.lock().unwrap().got.len(), 1);
}

#[tokio::test]
async fn slow_or_failing_channels_do_not_affect_tickets() {
    let (url, m) = mock(Mock {
        fail: 99,
        status: 500,
        delay: Duration::from_secs(3),
        ..Mock::default()
    })
    .await;
    let dir = setup();
    let a = app(dir.path(), notify(&url, None));
    let start = Instant::now();
    let (s, body) = new_ticket(&a).await;
    assert_eq!((s, body.as_str()), (StatusCode::CREATED, r#"{"id":1}"#));
    let (s, _) = call(
        &a,
        "POST",
        "/v1/tickets/1/replies",
        &practice(),
        json!({"message": "m"}).to_string(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
    // Nowhere to deliver at all is fine too.
    let unreachable = Notify {
        discord_webhook: Some("http://127.0.0.1:9/x".into()),
        ..Notify::default()
    };
    let b = app(setup().path(), unreachable);
    assert_eq!(new_ticket(&b).await.0, StatusCode::CREATED);
    drop(m);
}

#[tokio::test]
async fn admin_sees_channels_and_sends_a_test_without_secrets() {
    let (url, m) = mock(Mock::default()).await;
    let dir = setup();
    let a = app(dir.path(), notify(&url, Some("https://bib.example")));
    let (s, body) = call(&a, "GET", "/admin/api/notify", &bearer(), String::new()).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({"channels": ["discord", "telegram", "webhook"], "public_url": "https://bib.example"})
    );
    assert!(!body.contains(BOT_TOKEN) && !body.contains(HOOK_SECRET) && !body.contains(&url));

    let (s, body) = call(
        &a,
        "POST",
        "/admin/api/notify/test",
        &bearer(),
        String::new(),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    let r: Value = serde_json::from_str(&body).unwrap();
    assert!(
        r["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["ok"] == true),
        "{r}"
    );
    let got = m.lock().unwrap().got.clone();
    assert_eq!(find(&got, "hook")[0]["event"], "test");
    assert!(find(&got, "tg")[0]["text"]
        .as_str()
        .unwrap()
        .ends_with("https://bib.example/admin#tickets"));

    // Failures name the channel and the status, never the token or the webhook URL.
    m.lock().unwrap().fail = 99;
    m.lock().unwrap().status = 401;
    let (_, body) = call(
        &a,
        "POST",
        "/admin/api/notify/test",
        &bearer(),
        String::new(),
    )
    .await;
    let r: Value = serde_json::from_str(&body).unwrap();
    assert!(
        r["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["ok"] == false),
        "{r}"
    );
    assert!(body.contains("401"), "{body}");
    let down = Notify {
        telegram: Some(Telegram {
            token: BOT_TOKEN.into(),
            chat_id: "1".into(),
            api_url: "http://127.0.0.1:9".into(),
        }),
        discord_webhook: Some(format!("http://127.0.0.1:9/{HOOK_SECRET}")),
        ..Notify::default()
    };
    let b = app(setup().path(), down);
    let (_, down_body) = call(
        &b,
        "POST",
        "/admin/api/notify/test",
        &bearer(),
        String::new(),
    )
    .await;
    assert!(down_body.contains("nicht erreichbar"), "{down_body}");
    for b in [&body, &down_body] {
        assert!(
            !b.contains(BOT_TOKEN) && !b.contains(HOOK_SECRET) && !b.contains("127.0.0.1"),
            "{b}"
        );
    }

    // Nothing configured: a clear 409.
    let c = app(setup().path(), Notify::default());
    let (s, body) = call(&c, "GET", "/admin/api/notify", &bearer(), String::new()).await;
    assert_eq!(
        (s, body.as_str()),
        (StatusCode::OK, r#"{"channels":[],"public_url":null}"#)
    );
    assert_eq!(
        call(
            &c,
            "POST",
            "/admin/api/notify/test",
            &bearer(),
            String::new()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

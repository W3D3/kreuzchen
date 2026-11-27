//! GitHub sync against a local mock of the GitHub REST API.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use kreuzchen_engine::hash;
use kreuzchen_registry::{load, router, Source};
use tower::ServiceExt;

const EXAMPLE: &str = include_str!("../../../examples/definitions/beispiel.yaml");
const ADMIN: &str = "admin-token-fuer-tests";
const GH_TOKEN: &str = "github_pat_secret_123";
const SHA1: &str = "1111111111111111111111111111111111111111";
const SHA2: &str = "2222222222222222222222222222222222222222";

/// A questionnaire with the 0.4 German names, as a repo not switched yet still holds.
const OLD: &str = include_str!("../../../fixtures/registry-data-0.4.0/definitionen/beispiel.yaml");

fn def(id: &str, version: &str) -> String {
    EXAMPLE
        .replacen("id: beispiel", &format!("id: {id:?}"), 1)
        .replacen("version: \"1\"", &format!("version: {version:?}"), 1)
}

type Files = Vec<(&'static str, String)>;

/// The mock's blob sha: content-addressed like git's, so unchanged content keeps its sha.
fn blob_sha(text: &str) -> String {
    hash(text.as_bytes())
}

#[derive(Default)]
struct Mock {
    head: String,
    /// Commit sha → every file in that commit.
    commits: HashMap<String, Files>,
    fail: bool,
    truncated: bool,
    /// Every tree and blob request, as `trees/<id>` or `blobs/<sha>`.
    requests: Vec<String>,
    auth: Vec<String>,
}

type Shared = Arc<Mutex<Mock>>;

async fn commits(State(m): State<Shared>, Path(branch): Path<String>, h: HeaderMap) -> Response {
    let auth = h
        .get("authorization")
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let mut m = m.lock().unwrap();
    m.auth.push(auth.clone());
    if m.fail {
        // A body echoing the request must never end up in status or logs.
        return (StatusCode::BAD_GATEWAY, format!("kaputt {auth}")).into_response();
    }
    if branch != "main" || h["accept"] != "application/vnd.github.sha" {
        return StatusCode::NOT_FOUND.into_response();
    }
    m.head.clone().into_response()
}

/// One tree level. A tree id is `<commit>:<folder>`; the bare commit sha is its root tree.
async fn trees(State(m): State<Shared>, Path(id): Path<String>) -> Response {
    let mut m = m.lock().unwrap();
    m.requests.push(format!("trees/{id}"));
    let (commit, dir) = id.split_once(':').unwrap_or((&id, ""));
    let Some(files) = m.commits.get(commit) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    let mut tree: Vec<serde_json::Value> = Vec::new();
    for (path, text) in files {
        let Some(rest) = path.strip_prefix(&prefix) else {
            continue;
        };
        let entry = match rest.split_once('/') {
            Some((sub, _)) => serde_json::json!({"path": sub, "mode": "040000", "type": "tree",
                "sha": format!("{commit}:{prefix}{sub}")}),
            None => serde_json::json!({"path": rest, "mode": "100644", "type": "blob",
                "sha": blob_sha(text), "size": text.len()}),
        };
        if !tree.contains(&entry) {
            tree.push(entry);
        }
    }
    let truncated = m.truncated;
    axum::Json(serde_json::json!({"sha": id, "tree": tree, "truncated": truncated})).into_response()
}

async fn blobs(State(m): State<Shared>, Path(sha): Path<String>, h: HeaderMap) -> Response {
    let mut m = m.lock().unwrap();
    m.requests.push(format!("blobs/{sha}"));
    if h["accept"] != "application/vnd.github.raw+json" {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let text = m
        .commits
        .values()
        .flatten()
        .find(|(_, t)| blob_sha(t) == sha);
    match text {
        Some((_, t)) => t.clone().into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Serves the mock on a free port; returns its base URL.
async fn github(m: Shared) -> String {
    let app = Router::new()
        .route("/repos/o/r/commits/{branch}", get(commits))
        .route("/repos/o/r/git/trees/{id}", get(trees))
        .route("/repos/o/r/git/blobs/{sha}", get(blobs))
        .with_state(m);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url
}

/// What the repo holds at SHA1: two questionnaires in `questionnaires/`, decoys elsewhere.
fn first_commit() -> Files {
    vec![
        ("questionnaires/a.yaml", def("fb_a", "1")),
        (
            "questionnaires/b.yml",
            OLD.replacen("id: beispiel", "id: fb_b", 1),
        ),
        ("questionnaires/note.md", "not YAML".into()),
        ("questionnaires/old/c.yaml", def("fb_subfolder", "1")),
        ("other/d.yaml", def("fb_other_folder", "1")),
        ("e.yaml", def("fb_root", "1")),
        // A large source PDF next to the questionnaires, as the real repo has.
        ("quellen/scan.pdf", "%PDF".repeat(1 << 19)),
    ]
}

fn data_dir(api_url: &str) -> tempfile::TempDir {
    let key_hash = Argon2::default()
        .hash_password(b"secret", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!(
            "source:\n  type: github\n  repo: o/r\n  branch: main\n  path: /questionnaires/\n  api_url: {api_url}\n\
             practices:\n  - user: practice_a\n    key_hash: {key_hash:?}\n    questionnaires: [fb_a, fb_b, fb_c]\n"
        ),
    )
    .unwrap();
    dir
}

struct Env {
    app: Router,
    mock: Shared,
    dir: tempfile::TempDir,
}

/// A server whose start-up sync has finished.
async fn setup(fail_first: bool) -> Env {
    let mock = Shared::default();
    {
        let mut m = mock.lock().unwrap();
        m.head = SHA1.into();
        m.commits.insert(SHA1.into(), first_commit());
        m.fail = fail_first;
    }
    let dir = data_dir(&github(mock.clone()).await);
    let app = router(
        load(dir.path()).unwrap(),
        Some(ADMIN),
        Some(GH_TOKEN),
        Default::default(),
    );
    for _ in 0..500 {
        let s = status(&app).await;
        if !s["checked_at"].is_null() || !s["error"].is_null() {
            return Env { app, mock, dir };
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("start-up sync did not finish");
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    auth: String,
    body: &str,
) -> (StatusCode, String) {
    let mut r = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", auth);
    if uri.ends_with("/practices") {
        r = r.header("content-type", "application/json");
    }
    let resp = app
        .clone()
        .oneshot(r.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn admin(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    call(app, method, uri, format!("Bearer {ADMIN}"), body).await
}

async fn practice(app: &Router, uri: &str) -> (StatusCode, String) {
    let basic = base64::engine::general_purpose::STANDARD.encode("practice_a:secret");
    call(app, "GET", uri, format!("Basic {basic}"), "").await
}

async fn status(app: &Router) -> serde_json::Value {
    let (s, b) = admin(app, "GET", "/admin/api/sync", "").await;
    assert_eq!(s, StatusCode::OK);
    serde_json::from_str(&b).unwrap()
}

async fn sync_now(app: &Router) -> serde_json::Value {
    let (s, b) = admin(app, "POST", "/admin/api/sync", "").await;
    assert_eq!(s, StatusCode::OK, "{b}");
    serde_json::from_str(&b).unwrap()
}

async fn practice_ids(app: &Router) -> Vec<String> {
    let (s, b) = practice(app, "/v1/questionnaires").await;
    assert_eq!(s, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn first_sync_publishes_only_yaml_directly_in_path() {
    let env = setup(false).await;
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);
    let (s, body) = practice(&env.app, "/v1/questionnaires/fb_a").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body, def("fb_a", "1"));
    // The 0.4 file loads through the upgrade, yet is served as the repo has it.
    let (_, body) = practice(&env.app, "/v1/questionnaires/fb_b").await;
    assert_eq!(body, OLD.replacen("id: beispiel", "id: fb_b", 1));

    let st = status(&env.app).await;
    assert_eq!(st["source"], "github");
    assert_eq!(st["repo"], "o/r");
    assert_eq!(st["branch"], "main");
    assert_eq!(st["sha"], SHA1);
    assert!(st["checked_at"].as_u64().unwrap() > 0);
    assert!(st["error"].is_null());
    assert_eq!(st["invalid"], serde_json::json!([]));

    // The admin list shows the same set; a restart serves it before reaching GitHub.
    let (_, list) = admin(&env.app, "GET", "/admin/api/questionnaires", "").await;
    assert!(list.contains("fb_a") && !list.contains("fb_root"));
    let ids: Vec<_> = load(env.dir.path()).unwrap().entries.into_keys().collect();
    assert_eq!(ids, ["fb_a", "fb_b"]);
}

#[tokio::test]
async fn only_the_yaml_blobs_in_path_are_requested() {
    let env = setup(false).await;
    let files = first_commit();
    let blob = |name: &str| {
        let text = &files.iter().find(|(p, _)| *p == name).unwrap().1;
        format!("blobs/{}", blob_sha(text))
    };
    // Not the PDF, not its folder, not the decoys: just the walk to `path` and its two YAML files.
    assert_eq!(
        env.mock.lock().unwrap().requests,
        [
            format!("trees/{SHA1}"),
            format!("trees/{SHA1}:questionnaires"),
            blob("questionnaires/a.yaml"),
            blob("questionnaires/b.yml"),
        ]
    );
}

#[tokio::test]
async fn unchanged_head_skips_the_download() {
    let env = setup(false).await;
    let before = env.mock.lock().unwrap().requests.len();
    let st = sync_now(&env.app).await;
    assert_eq!(st["sha"], SHA1);
    assert_eq!(env.mock.lock().unwrap().requests.len(), before);
    assert_eq!(env.mock.lock().unwrap().auth.len(), 2);
}

#[tokio::test]
async fn new_head_downloads_only_changed_blobs() {
    let env = setup(false).await;
    let new_b = def("fb_b", "2");
    {
        let mut m = env.mock.lock().unwrap();
        m.head = SHA2.into();
        m.requests.clear();
        let files = vec![
            ("questionnaires/a.yaml", def("fb_a", "1")),
            ("questionnaires/b.yml", new_b.clone()),
        ];
        m.commits.insert(SHA2.into(), files);
    }
    assert_eq!(sync_now(&env.app).await["sha"], SHA2);
    let blobs: Vec<_> = env
        .mock
        .lock()
        .unwrap()
        .requests
        .iter()
        .filter(|r| r.starts_with("blobs/"))
        .cloned()
        .collect();
    assert_eq!(blobs, [format!("blobs/{}", blob_sha(&new_b))]);
    assert_eq!(
        practice(&env.app, "/v1/questionnaires/fb_a").await.1,
        def("fb_a", "1")
    );
    assert_eq!(practice(&env.app, "/v1/questionnaires/fb_b").await.1, new_b);
}

#[tokio::test]
async fn oversized_file_is_invalid_and_never_downloaded() {
    let env = setup(false).await;
    let big = format!("{}#{}\n", def("fb_a", "2"), "x".repeat(1 << 20));
    {
        let mut m = env.mock.lock().unwrap();
        m.head = SHA2.into();
        let files = vec![
            ("questionnaires/a.yaml", big.clone()),
            ("questionnaires/b.yml", def("fb_b", "2")),
        ];
        m.commits.insert(SHA2.into(), files);
    }
    let st = sync_now(&env.app).await;
    assert_eq!(st["sha"], SHA2);
    assert_eq!(st["invalid"][0]["file"], "a.yaml");
    assert!(st["invalid"][0]["error"]
        .as_str()
        .unwrap()
        .contains("größer als 1 MiB"));
    let big_blob = format!("blobs/{}", blob_sha(&big));
    assert!(!env.mock.lock().unwrap().requests.contains(&big_blob));
    // Its last good version stays.
    assert_eq!(
        practice(&env.app, "/v1/questionnaires/fb_a").await.1,
        def("fb_a", "1")
    );
}

#[tokio::test]
async fn truncated_tree_or_missing_folder_fails_and_keeps_the_set() {
    let env = setup(false).await;
    {
        let mut m = env.mock.lock().unwrap();
        m.head = SHA2.into();
        m.truncated = true;
        m.commits.insert(
            SHA2.into(),
            vec![("questionnaires/b.yml", def("fb_b", "2"))],
        );
    }
    let st = sync_now(&env.app).await;
    assert!(
        st["error"].as_str().unwrap().contains("unvollständig"),
        "{st}"
    );
    assert_eq!(st["sha"], SHA1);
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);

    {
        let mut m = env.mock.lock().unwrap();
        m.truncated = false;
        m.commits
            .insert(SHA2.into(), vec![("other/b.yml", def("fb_b", "2"))]);
    }
    let st = sync_now(&env.app).await;
    assert!(st["error"].as_str().unwrap().contains("fehlt"), "{st}");
    assert_eq!(st["sha"], SHA1);
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);
}

#[tokio::test]
async fn new_head_is_published_and_invalid_files_keep_their_last_good_version() {
    let env = setup(false).await;
    {
        let mut m = env.mock.lock().unwrap();
        m.head = SHA2.into();
        let files = vec![
            ("questionnaires/a.yaml", "id: fb_a\nkaputt: [".to_string()),
            ("questionnaires/b.yml", def("fb_b", "2")),
            ("questionnaires/c.yaml", "id: fb_c\n".to_string()),
        ];
        m.commits.insert(SHA2.into(), files);
    }
    let st = sync_now(&env.app).await;
    assert_eq!(st["sha"], SHA2);
    assert!(st["error"].is_null());
    let bad: Vec<_> = st["invalid"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["file"].as_str().unwrap())
        .collect();
    assert_eq!(bad, ["a.yaml", "c.yaml"]);
    assert!(st["invalid"][0]["error"]
        .as_str()
        .unwrap()
        .contains("a.yaml"));

    // a: last good version; b: new version; c: new and broken, so absent.
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);
    assert_eq!(
        practice(&env.app, "/v1/questionnaires/fb_a").await.1,
        def("fb_a", "1")
    );
    assert_eq!(
        practice(&env.app, "/v1/questionnaires/fb_b").await.1,
        def("fb_b", "2")
    );
    assert_eq!(
        practice(&env.app, "/v1/questionnaires/fb_c").await.0,
        StatusCode::NOT_FOUND
    );

    // Exactly one snapshot plus the pointer remain, and they survive a restart.
    assert_eq!(
        fs::read_dir(env.dir.path().join("github")).unwrap().count(),
        2
    );
    let reg = load(env.dir.path()).unwrap();
    assert_eq!(reg.entries["fb_a"].hash, hash(def("fb_a", "1").as_bytes()));
    assert_eq!(reg.entries["fb_b"].hash, hash(def("fb_b", "2").as_bytes()));
}

#[tokio::test]
async fn failed_fetch_keeps_the_set_and_never_shows_the_token() {
    let env = setup(false).await;
    {
        let mut m = env.mock.lock().unwrap();
        m.fail = true;
        m.head = SHA2.into();
    }
    let st = sync_now(&env.app).await;
    let error = st["error"].as_str().unwrap();
    assert!(error.contains("502"), "{error}");
    assert_eq!(st["sha"], SHA1);
    assert!(!st.to_string().contains(GH_TOKEN));
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);

    // Missing commit tree: same, the published set stays.
    env.mock.lock().unwrap().fail = false;
    let st = sync_now(&env.app).await;
    assert!(st["error"].as_str().unwrap().contains("404"));
    assert_eq!(st["sha"], SHA1);
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);
}

#[tokio::test]
async fn start_without_github_serves_nothing_until_a_sync_works() {
    let env = setup(true).await;
    assert!(status(&env.app).await["sha"].is_null());
    assert!(practice_ids(&env.app).await.is_empty());
    env.mock.lock().unwrap().fail = false;
    assert_eq!(sync_now(&env.app).await["sha"], SHA1);
    assert_eq!(practice_ids(&env.app).await, ["fb_a", "fb_b"]);
}

#[tokio::test]
async fn token_is_sent_as_bearer_header() {
    let env = setup(false).await;
    let auth = env.mock.lock().unwrap().auth.clone();
    assert_eq!(auth, [format!("Bearer {GH_TOKEN}")]);
}

#[tokio::test]
async fn git_mode_blocks_file_changes_but_not_assignments() {
    let env = setup(false).await;
    let blocked = [
        ("POST", "/admin/api/questionnaires".to_string()),
        ("PUT", "/admin/api/questionnaires/fb_a".to_string()),
        ("DELETE", "/admin/api/questionnaires/fb_a".to_string()),
    ];
    for (method, uri) in blocked {
        let (s, b) = admin(&env.app, method, &uri, &def("fb_a", "9")).await;
        assert_eq!(s, StatusCode::CONFLICT, "{method} {uri}");
        assert_eq!(
            b,
            "Im Git-Modus nicht verfügbar – Änderungen bitte im Repository vornehmen"
        );
    }
    assert_eq!(
        practice(&env.app, "/v1/questionnaires/fb_a").await.1,
        def("fb_a", "1")
    );

    let (s, b) = admin(
        &env.app,
        "PUT",
        "/admin/api/questionnaires/fb_b/practices",
        "[]",
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(practice_ids(&env.app).await, ["fb_a"]);
    // The rewritten config keeps the source and the assignment that waits for fb_c.
    let reg = load(env.dir.path()).unwrap();
    assert!(matches!(reg.source, Source::Github(ref gh) if gh.repo == "o/r"));
    assert_eq!(reg.practices[0].questionnaires, ["fb_a", "fb_c"]);
}

#[tokio::test]
async fn git_mode_manages_practices_and_keeps_the_source() {
    let env = setup(false).await;
    let body = r#"{"user": "practice_new", "questionnaires": ["fb_b"]}"#;
    let (s, b) = admin(&env.app, "POST", "/admin/api/practices", body).await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    let (s, b) = admin(
        &env.app,
        "POST",
        "/admin/api/practices/practice_new/key",
        "",
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, _) = admin(&env.app, "DELETE", "/admin/api/practices/practice_a", "").await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let reg = load(env.dir.path()).unwrap();
    assert!(matches!(reg.source, Source::Github(ref gh) if gh.repo == "o/r"));
    let users: Vec<_> = reg.practices.iter().map(|p| p.user.as_str()).collect();
    assert_eq!(users, ["practice_new"]);
    assert_eq!(reg.entries.len(), 2, "the synced set is untouched");
}

#[tokio::test]
async fn local_mode_reports_its_source_and_refuses_sync() {
    let key_hash = Argon2::default()
        .hash_password(b"secret", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitions")).unwrap();
    fs::write(
        dir.path().join("config.yaml"),
        format!("practices:\n  - user: p\n    key_hash: {key_hash:?}\n    questionnaires: []\n"),
    )
    .unwrap();
    let app = router(
        load(dir.path()).unwrap(),
        Some(ADMIN),
        Some(GH_TOKEN),
        Default::default(),
    );
    assert_eq!(status(&app).await, serde_json::json!({"source": "local"}));
    let (s, _) = admin(&app, "POST", "/admin/api/sync", "").await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = admin(&app, "POST", "/admin/api/questionnaires", &def("new", "1")).await;
    assert_eq!(s, StatusCode::CREATED);
}

// Multi-thread: this thread blocks on the log while the mock keeps serving.
#[tokio::test(flavor = "multi_thread")]
async fn server_log_never_contains_the_token() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let mock = Shared::default();
    mock.lock().unwrap().fail = true;
    let dir = data_dir(&github(mock.clone()).await);
    let mut child = Command::new(env!("CARGO_BIN_EXE_kreuzchen-registry"))
        .args(["serve", "--port", "0", "--data"])
        .arg(dir.path())
        .env("KREUZCHEN_GITHUB_TOKEN", GH_TOKEN)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = tx.send(line.unwrap());
        }
    });
    let mut log = Vec::new();
    let seen = |log: &[String], p: &str| log.iter().any(|l| l.starts_with(p));
    while !(seen(&log, "sync ") && seen(&log, "listening ")) {
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(line) => log.push(line),
            Err(_) => break,
        }
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let log = log.join("\n");
    assert!(log.contains("source github o/r@main"), "{log}");
    assert!(log.contains("sync o/r@main: GitHub antwortet 502"), "{log}");
    assert!(!log.contains(GH_TOKEN), "{log}");
    assert_eq!(mock.lock().unwrap().auth, [format!("Bearer {GH_TOKEN}")]);
}

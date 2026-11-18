use crate::{
    Paths,
    definitions::{DefinitionEntry, YAML, check_name, hash, load_text, scan},
    store::write_atomic,
};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, sync::OnceLock, time::Duration};

const UNREACHABLE: &str = "Registry nicht erreichbar";
const WRONG_KEY: &str = "Benutzername oder Schlüssel falsch";
const BAD_URL: &str = "Ungültige Adresse";
const FILE: &str = "registry.json";

#[derive(Serialize, Deserialize)]
pub struct Connection {
    pub url: String,
    pub user: String,
    pub key: String,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("url", &self.url)
            .field("user", &self.user)
            .field("key", &"***")
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RemoteEntry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub hash: String,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RowState {
    Available,
    Current,
    Update,
    Gone,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub state: RowState,
    pub file: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallOutcome {
    Installed,
    NeedsReplaceConfirm,
}

/// Bytes of a download received so far; `total` is `None` when the server sends no length.
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub received: u64,
    pub total: Option<u64>,
}

/// Adds `https://` when no scheme is given and strips trailing `/`; https only (http for loopback).
pub fn normalize_url(input: &str) -> Result<String, String> {
    let s = input.trim();
    let s = if s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    };
    let u = Url::parse(&s).map_err(|_| BAD_URL.to_string())?;
    if u.host_str().is_none_or(str::is_empty) {
        return Err(BAD_URL.into());
    }
    // Basic credentials must not travel in clear text; http only for local testing.
    let loopback = matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match u.scheme() {
        "https" => {}
        "http" if loopback => {}
        "http" => return Err("Nur https-Adressen sind erlaubt.".into()),
        _ => return Err(BAD_URL.into()),
    }
    Ok(s.trim_end_matches('/').to_string())
}

pub fn load_connection(app_data: &Path) -> Option<Connection> {
    serde_json::from_slice(&fs::read(app_data.join(FILE)).ok()?).ok()
}

impl Connection {
    /// Validated url, whitespace-trimmed credentials (pasted keys often carry a newline).
    pub fn new(url: &str, user: &str, key: &str) -> Result<Self, String> {
        Ok(Self {
            url: normalize_url(url)?,
            user: user.trim().into(),
            key: key.trim().into(),
        })
    }
}

pub fn save_connection(app_data: &Path, c: &Connection) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(&c).map_err(|e| e.to_string())?;
    write_atomic(app_data, FILE, &json)
}

pub fn clear_connection(app_data: &Path) {
    let _ = fs::remove_file(app_data.join(FILE));
}

/// The id a registry file stands for: its definition id, else the file stem.
fn installed_id(e: &DefinitionEntry) -> String {
    e.summary.as_ref().map_or_else(
        || {
            Path::new(&e.file)
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
        },
        |s| s.id.clone(),
    )
}

/// Registry screen rows. `installed` is the registry folder's scan; `None` means offline.
pub fn rows(remote: Option<&[RemoteEntry]>, installed: &[DefinitionEntry]) -> Vec<Row> {
    let installed: Vec<_> = installed.iter().filter(|e| !e.file.is_empty()).collect();
    let ids: Vec<String> = installed.iter().map(|e| installed_id(e)).collect();
    let mut out: Vec<Row> = installed
        .iter()
        .zip(&ids)
        .map(|(e, id)| {
            let state = match remote {
                None => RowState::Current,
                Some(r) => match r.iter().find(|r| &r.id == id) {
                    None => RowState::Gone,
                    Some(r) if r.hash == e.hash => RowState::Current,
                    Some(_) => RowState::Update,
                },
            };
            Row {
                id: id.clone(),
                title: e
                    .summary
                    .as_ref()
                    .map_or_else(|| e.file.clone(), |s| s.title.clone()),
                state,
                file: Some(e.file.clone()),
            }
        })
        .collect();
    out.extend(
        remote
            .unwrap_or_default()
            .iter()
            .filter(|r| !ids.contains(&r.id))
            .map(|r| Row {
                id: r.id.clone(),
                title: r.title.clone(),
                state: RowState::Available,
                file: None,
            }),
    );
    out.sort_by(|a, b| (a.title.to_lowercase(), &a.id).cmp(&(b.title.to_lowercase(), &b.id)));
    out
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        // Same provider as the updater plugin; a second install attempt is a harmless Err.
        let _ = rustls::crypto::ring::default_provider().install_default();
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("HTTP client")
    })
}

/// GET `<url>/v1/fragebogen[/<id>]` with Basic auth; the hash header and body, `None` if unreachable.
/// `progress` sees the body arrive chunk by chunk.
async fn get(
    c: &Connection,
    id: Option<&str>,
    mut progress: impl FnMut(Progress),
) -> Result<Option<(Option<String>, Vec<u8>)>, String> {
    let mut url = Url::parse(&c.url).map_err(|_| BAD_URL.to_string())?;
    url.path_segments_mut()
        .map_err(|()| BAD_URL.to_string())?
        .pop_if_empty()
        .extend(["v1", "fragebogen"])
        .extend(id);
    let Ok(mut resp) = client()
        .get(url)
        .basic_auth(&c.user, Some(&c.key))
        .send()
        .await
    else {
        return Ok(None);
    };
    match resp.status() {
        StatusCode::UNAUTHORIZED => return Err(WRONG_KEY.into()),
        s if !s.is_success() => return Err(format!("Fehler der Registry: {s}")),
        _ => {}
    }
    let header = resp
        .headers()
        .get("x-kreuzchen-hash")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let total = resp.content_length();
    let mut body = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => body.extend_from_slice(&chunk),
            Ok(None) => return Ok(Some((header, body))),
            Err(_) => return Ok(None),
        }
        progress(Progress {
            received: body.len() as u64,
            total,
        });
    }
}

/// The server list, or `None` when the registry cannot be reached.
pub async fn try_fetch_list(c: &Connection) -> Result<Option<Vec<RemoteEntry>>, String> {
    let Some((_, body)) = get(c, None, |_| {}).await? else {
        return Ok(None);
    };
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|_| "Antwort der Registry ist ungültig".to_string())
}

#[derive(Serialize, Debug, PartialEq)]
pub struct RowsResult {
    pub online: bool,
    pub rows: Vec<Row>,
    pub error: Option<String>,
}

/// The registry screen's rows; remote problems become `error` so installed rows stay listed.
pub async fn rows_result(c: Option<&Connection>, installed: &[DefinitionEntry]) -> RowsResult {
    let (remote, error) = match c {
        None => (None, None),
        Some(c) => match try_fetch_list(c).await {
            Ok(r) => (r, None),
            Err(e) => (None, Some(e)),
        },
    };
    RowsResult {
        online: remote.is_some(),
        rows: rows(remote.as_deref(), installed),
        error,
    }
}

pub async fn fetch_list(c: &Connection) -> Result<Vec<RemoteEntry>, String> {
    try_fetch_list(c).await?.ok_or_else(|| UNREACHABLE.into())
}

/// Deletes `file` from the registry folder only.
pub fn remove(registry: &Path, file: &str) -> Result<(), String> {
    check_name(file, YAML)?;
    fs::remove_file(registry.join(file)).map_err(|e| format!("Löschen fehlgeschlagen: {e}"))
}

pub async fn install(
    c: &Connection,
    paths: &Paths,
    id: &str,
    replace_local: bool,
    progress: impl FnMut(Progress),
) -> Result<InstallOutcome, String> {
    let file = format!("{id}.yaml");
    check_name(&file, YAML)?;
    let local: Vec<String> = scan(&paths.definitions)
        .into_iter()
        .filter(|e| e.summary.as_ref().is_some_and(|s| s.id == id))
        .map(|e| e.file)
        .collect();
    if !local.is_empty() && !replace_local {
        return Ok(InstallOutcome::NeedsReplaceConfirm);
    }
    let list = fetch_list(c).await?;
    let entry = list
        .iter()
        .find(|e| e.id == id)
        .ok_or("Der Fragebogen ist nicht mehr auf dem Server.")?;
    let (header, bytes) = get(c, Some(id), progress).await?.ok_or(UNREACHABLE)?;
    let h = hash(&bytes);
    if header.as_deref() != Some(h.as_str()) || entry.hash != h {
        return Err("Die heruntergeladene Datei ist beschädigt.".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "Datei ist nicht UTF-8".to_string())?;
    if load_text(text)?.def.id != id {
        return Err("Die heruntergeladene Datei hat eine andere id.".into());
    }
    // Write first: a failed write keeps the local copy, so one of the two always exists.
    write_atomic(&paths.registry, &file, &bytes)?;
    for f in local {
        fs::remove_file(paths.definitions.join(f))
            .map_err(|e| format!("Löschen fehlgeschlagen: {e}"))?;
    }
    Ok(InstallOutcome::Installed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::Summary;
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };
    use std::{sync::Arc, time::Instant};

    const FIXTURE: &str = include_str!("../../examples/definitionen/beispiel.yaml");

    fn def(id: &str) -> String {
        FIXTURE.replacen("id: beispiel", &format!("id: {id}"), 1)
    }

    #[test]
    fn normalize_url_cases() {
        assert_eq!(
            normalize_url("kreuzchen.wedenig.dev").unwrap(),
            "https://kreuzchen.wedenig.dev"
        );
        assert_eq!(normalize_url("https://x.y/").unwrap(), "https://x.y");
        assert!(normalize_url("ftp://x").is_err());
        assert!(normalize_url("http://kreuzchen.wedenig.dev").is_err());
        assert_eq!(
            normalize_url("http://127.0.0.1:8080/").unwrap(),
            "http://127.0.0.1:8080"
        );
        assert!(normalize_url("http://localhost:1").is_ok());
        assert!(normalize_url("http://[::1]:1").is_ok());
        assert!(normalize_url("https://").is_err());
    }

    #[test]
    fn connection_trims_credentials() {
        let d = tempfile::tempdir().unwrap();
        let c = Connection::new("x.y", " praxis \n", "abc\n").unwrap();
        assert_eq!(c.url, "https://x.y");
        save_connection(d.path(), &c).unwrap();
        let got = load_connection(d.path()).unwrap();
        assert_eq!((got.user.as_str(), got.key.as_str()), ("praxis", "abc"));
        clear_connection(d.path());
        assert!(load_connection(d.path()).is_none());
    }

    fn remote(id: &str, hash: &str) -> RemoteEntry {
        RemoteEntry {
            id: id.into(),
            title: id.to_uppercase(),
            version: "1".into(),
            hash: hash.into(),
        }
    }

    fn local(id: &str, hash: &str) -> DefinitionEntry {
        DefinitionEntry {
            file: format!("{id}.yaml"),
            hash: hash.into(),
            summary: Some(Summary {
                id: id.into(),
                title: id.to_uppercase(),
                version: "1".into(),
                item_count: 1,
                scale_count: 1,
            }),
            error: None,
            needs_app: None,
            locked: true,
        }
    }

    #[test]
    fn rows_states() {
        let r = [remote("a", "h1"), remote("b", "h1"), remote("c", "h1")];
        let i = [local("a", "h1"), local("b", "h0"), local("d", "h1")];
        let got: Vec<_> = rows(Some(&r), &i)
            .into_iter()
            .map(|r| (r.id, r.state, r.file.is_some()))
            .collect();
        let s = |id: &str, st, f| (id.to_string(), st, f);
        use RowState::*;
        assert_eq!(
            got,
            [
                s("a", Current, true),
                s("b", Update, true),
                s("c", Available, false),
                s("d", Gone, true)
            ]
        );
        let ids: Vec<_> = rows(None, &i).into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["a", "b", "d"]);
    }

    struct Env {
        _dir: tempfile::TempDir,
        paths: Paths,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let paths = Paths {
            data: p.into(),
            definitions: p.join("definitionen"),
            results: p.join("auswertungen"),
            drafts: p.join("entwuerfe"),
            registry: p.join("registry"),
        };
        fs::create_dir(&paths.definitions).unwrap();
        fs::create_dir(&paths.registry).unwrap();
        Env { _dir: dir, paths }
    }

    async fn serve(app: axum::Router) -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
        url
    }

    /// A real registry serving `fragebogen_a` to `praxis_a` / `geheim`; keep the dir alive.
    async fn registry() -> (String, tempfile::TempDir) {
        let key_hash = Argon2::default()
            .hash_password(b"geheim", &SaltString::generate(&mut OsRng))
            .unwrap()
            .to_string();
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("definitionen")).unwrap();
        fs::write(
            dir.path().join("config.yaml"),
            format!(
                "praxen:\n  - user: praxis_a\n    key_hash: {key_hash:?}\n    fragebogen: [fragebogen_a]\n"
            ),
        )
        .unwrap();
        fs::write(
            dir.path().join("definitionen/fragebogen_a.yaml"),
            def("fragebogen_a"),
        )
        .unwrap();
        let reg = kreuzchen_registry::load(dir.path()).unwrap();
        (serve(kreuzchen_registry::router(Arc::new(reg))).await, dir)
    }

    fn conn(url: &str, key: &str) -> Connection {
        Connection {
            url: url.into(),
            user: "praxis_a".into(),
            key: key.into(),
        }
    }

    #[tokio::test]
    async fn install_end_to_end() {
        let (url, _d) = registry().await;
        let c = conn(&url, "geheim");
        let list = fetch_list(&c).await.unwrap();
        let bytes = def("fragebogen_a");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "fragebogen_a");
        assert_eq!(list[0].hash, hash(bytes.as_bytes()));
        let e = env();
        let mut seen = Vec::new();
        assert_eq!(
            install(&c, &e.paths, "fragebogen_a", false, |p| seen.push(p)).await,
            Ok(InstallOutcome::Installed)
        );
        let len = bytes.len() as u64;
        assert!(seen.windows(2).all(|w| w[0].received < w[1].received));
        assert_eq!(
            seen.last(),
            Some(&Progress {
                received: len,
                total: Some(len)
            })
        );
        let written = fs::read(e.paths.registry.join("fragebogen_a.yaml")).unwrap();
        assert_eq!(written, bytes.as_bytes());
        let r = rows(Some(&list), &scan(&e.paths.registry));
        assert_eq!(r[0].state, RowState::Current);
    }

    #[tokio::test]
    async fn install_asks_before_replacing_local() {
        let (url, _d) = registry().await;
        let c = conn(&url, "geheim");
        let e = env();
        let local = e.paths.definitions.join("meine_kopie.yaml");
        let installed = e.paths.registry.join("fragebogen_a.yaml");
        fs::write(&local, def("fragebogen_a")).unwrap();
        assert_eq!(
            install(&c, &e.paths, "fragebogen_a", false, |_| {}).await,
            Ok(InstallOutcome::NeedsReplaceConfirm)
        );
        assert!(local.exists() && !installed.exists());
        // A failed write (registry folder missing) keeps the local copy.
        fs::remove_dir(&e.paths.registry).unwrap();
        assert!(
            install(&c, &e.paths, "fragebogen_a", true, |_| {})
                .await
                .is_err()
        );
        assert!(local.exists());
        fs::create_dir(&e.paths.registry).unwrap();
        assert_eq!(
            install(&c, &e.paths, "fragebogen_a", true, |_| {}).await,
            Ok(InstallOutcome::Installed)
        );
        assert!(!local.exists() && installed.exists());
    }

    /// A fake registry: `list` as the list, every download serves `bytes` with `header`.
    async fn fake(list: Vec<RemoteEntry>, header: String, bytes: String) -> Connection {
        use axum::{Json, routing::get};
        let app = axum::Router::new()
            .route("/v1/fragebogen", get(move || async move { Json(list) }))
            .route(
                "/v1/fragebogen/{id}",
                get(move || async move { ([("x-kreuzchen-hash", header)], bytes) }),
            );
        conn(&serve(app).await, "x")
    }

    fn entry_a(hash: String) -> RemoteEntry {
        RemoteEntry {
            id: "fragebogen_a".into(),
            title: "A".into(),
            version: "1".into(),
            hash,
        }
    }

    /// Installs `fragebogen_a` over an existing "alt" file; returns the error, checks the file kept.
    async fn install_fails(c: Connection) -> String {
        let e = env();
        let old = e.paths.registry.join("fragebogen_a.yaml");
        fs::write(&old, "alt").unwrap();
        let err = install(&c, &e.paths, "fragebogen_a", false, |_| {})
            .await
            .unwrap_err();
        assert_eq!(fs::read(&old).unwrap(), b"alt");
        assert_eq!(fs::read_dir(&e.paths.registry).unwrap().count(), 1);
        err
    }

    #[tokio::test]
    async fn install_keeps_old_file_on_bad_hash() {
        let bytes = def("fragebogen_a");
        let h = hash(bytes.as_bytes());
        let c = fake(vec![entry_a(h)], "falsch".into(), bytes).await;
        assert_eq!(
            install_fails(c).await,
            "Die heruntergeladene Datei ist beschädigt."
        );
    }

    #[tokio::test]
    async fn install_rejects_list_hash_mismatch() {
        let bytes = def("fragebogen_a");
        let h = hash(bytes.as_bytes());
        let c = fake(vec![entry_a("anders".into())], h, bytes).await;
        assert_eq!(
            install_fails(c).await,
            "Die heruntergeladene Datei ist beschädigt."
        );
    }

    #[tokio::test]
    async fn install_rejects_other_def_id() {
        let bytes = def("fragebogen_b");
        let h = hash(bytes.as_bytes());
        let c = fake(vec![entry_a(h.clone())], h, bytes).await;
        assert_eq!(
            install_fails(c).await,
            "Die heruntergeladene Datei hat eine andere id."
        );
    }

    #[tokio::test]
    async fn install_rejects_id_not_in_list() {
        let bytes = def("fragebogen_a");
        let h = hash(bytes.as_bytes());
        let c = fake(vec![], h, bytes).await;
        assert_eq!(
            install_fails(c).await,
            "Der Fragebogen ist nicht mehr auf dem Server."
        );
    }

    #[test]
    fn remove_stays_in_registry_folder() {
        let e = env();
        let outside = e.paths.data.join("x.yaml");
        let local = e.paths.definitions.join("x.yaml");
        let installed = e.paths.registry.join("x.yaml");
        for f in [&outside, &local, &installed] {
            fs::write(f, "x").unwrap();
        }
        assert_eq!(
            remove(&e.paths.registry, "../x.yaml"),
            Err("Ungültiger Dateiname".into())
        );
        remove(&e.paths.registry, "x.yaml").unwrap();
        assert!(outside.exists() && local.exists() && !installed.exists());
    }

    #[tokio::test]
    async fn install_rejects_bad_id() {
        let e = env();
        let c = conn("http://127.0.0.1:1", "x");
        assert_eq!(
            install(&c, &e.paths, "../x", false, |_| {}).await,
            Err("Ungültiger Dateiname".into())
        );
    }

    #[tokio::test]
    async fn wrong_key_message() {
        let (url, _d) = registry().await;
        assert_eq!(
            fetch_list(&conn(&url, "falsch")).await,
            Err(WRONG_KEY.into())
        );
    }

    #[tokio::test]
    async fn wrong_key_still_lists_installed_rows() {
        let (url, _d) = registry().await;
        let got = rows_result(Some(&conn(&url, "falsch")), &[local("a", "h1")]).await;
        assert!(!got.online);
        assert_eq!(got.error, Some(WRONG_KEY.into()));
        assert_eq!(got.rows.len(), 1);
        assert_eq!(got.rows[0].file.as_deref(), Some("a.yaml"));
    }

    #[tokio::test]
    async fn unreachable_is_reported_quickly() {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let start = Instant::now();
        let got = fetch_list(&conn(&format!("http://127.0.0.1:{port}"), "x")).await;
        assert_eq!(got, Err(UNREACHABLE.into()));
        assert!(start.elapsed() < Duration::from_secs(11));
    }
}

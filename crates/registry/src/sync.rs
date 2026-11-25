//! GitHub as the source of the questionnaires: resolve the branch head, and when it moved, list
//! `path` in that commit's tree, download its YAML blobs (nothing outside `path`), validate every
//! file and publish the result atomically.
//!
//! On disk: `github/<sha>-<millis>/` holds one published set, `github/current` names the live one.
//! `load` reads only through that pointer, so a set is either fully published or not at all.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::ACCEPT;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::{is_yaml, load, parse_entry, write_atomic, GitHub};
use crate::{Registry, MAX_UPLOAD};

const POINTER: &str = "current";
const MAX_TREE: usize = 16 * 1024 * 1024;

/// The directory `github/current` points at; `None` before the first sync.
pub(crate) fn published(data: &Path) -> Option<PathBuf> {
    let root = data.join("github");
    let name = fs::read_to_string(root.join(POINTER)).ok()?;
    let name = name.trim();
    snapshot_sha(name).map(|_| root.join(name))
}

/// The commit of a snapshot directory name `<sha>-<millis>`.
fn snapshot_sha(name: &str) -> Option<&str> {
    let (sha, millis) = name.split_once('-')?;
    (is_sha(sha) && !millis.is_empty() && millis.bytes().all(|b| b.is_ascii_digit())).then_some(sha)
}

fn is_sha(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Invalid {
    file: String,
    error: String,
}

#[derive(Clone, Default)]
struct Status {
    /// Commit of the published set.
    sha: Option<String>,
    /// Unix seconds of the last successful check against GitHub.
    checked_at: Option<u64>,
    error: Option<String>,
    invalid: Vec<Invalid>,
}

// No Debug: it holds the token.
pub(crate) struct Sync {
    gh: GitHub,
    token: Option<String>,
    client: reqwest::Client,
    reg: Arc<RwLock<Arc<Registry>>>,
    /// The admin write lock: publishing never interleaves with an admin read-modify-write.
    write: Arc<Mutex<()>>,
    /// Held for a whole run, so runs never overlap.
    running: Mutex<()>,
    status: std::sync::Mutex<Status>,
    /// Blob sha → bytes of the last published commit, so an unchanged file is not downloaded again.
    blobs: std::sync::Mutex<HashMap<String, Vec<u8>>>,
}

#[derive(Deserialize)]
struct Tree {
    tree: Vec<TreeEntry>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Deserialize)]
struct TreeEntry {
    path: String,
    mode: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
}

impl Sync {
    pub fn new(
        gh: GitHub,
        token: Option<&str>,
        reg: Arc<RwLock<Arc<Registry>>>,
        write: Arc<Mutex<()>>,
    ) -> Self {
        // Same provider as the app; a second install attempt is a harmless Err.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("kreuzchen-registry")
            .build()
            .expect("HTTP client");
        let sha = published(&reg.read().unwrap().dir)
            .and_then(|d| Some(snapshot_sha(d.file_name()?.to_str()?)?.to_owned()));
        Sync {
            gh,
            token: token.filter(|t| !t.is_empty()).map(str::to_owned),
            client,
            reg,
            write,
            running: Mutex::new(()),
            status: std::sync::Mutex::new(Status {
                sha,
                ..Status::default()
            }),
            blobs: Default::default(),
        }
    }

    /// Syncs now, then every `interval_minutes`, for as long as the server runs.
    pub fn spawn(self: Arc<Self>) {
        let every = Duration::from_secs(self.gh.interval_minutes * 60);
        tokio::spawn(async move {
            loop {
                self.run().await;
                tokio::time::sleep(every).await;
            }
        });
    }

    /// Syncs now. A failure keeps the published set and is recorded in the status.
    pub async fn run(&self) {
        let _running = self.running.lock().await;
        let prev = self.status.lock().unwrap().clone();
        let result = self.check(&prev).await;
        let mut st = self.status.lock().unwrap();
        match result {
            Ok(published) => {
                if let Some((sha, invalid)) = published {
                    eprintln!(
                        "sync {}@{}: {} veröffentlicht, {} ungültig",
                        self.gh.repo,
                        self.gh.branch,
                        &sha[..7],
                        invalid.len()
                    );
                    st.sha = Some(sha);
                    st.invalid = invalid;
                }
                st.checked_at = Some(now());
                st.error = None;
            }
            Err(e) => {
                eprintln!("sync {}@{}: {e}", self.gh.repo, self.gh.branch);
                st.error = Some(e);
            }
        }
    }

    pub fn status_json(&self) -> serde_json::Value {
        let st = self.status.lock().unwrap().clone();
        json!({
            "source": "github",
            "repo": self.gh.repo,
            "branch": self.gh.branch,
            "path": self.gh.path,
            "sha": st.sha,
            "checked_at": st.checked_at,
            "error": st.error,
            "invalid": st.invalid,
        })
    }

    /// The new sha and its invalid files once published; `None` if the head has not moved.
    async fn check(&self, prev: &Status) -> Result<Option<(String, Vec<Invalid>)>, String> {
        let head = self
            .get(
                &["commits", &self.gh.branch],
                "application/vnd.github.sha",
                100,
            )
            .await?;
        let sha = String::from_utf8_lossy(&head).trim().to_owned();
        if !is_sha(&sha) {
            return Err("GitHub lieferte keinen Commit für den Branch".into());
        }
        // The first run after a start always downloads: the invalid list is not kept on disk.
        if prev.checked_at.is_some() && prev.sha.as_deref() == Some(&sha) {
            return Ok(None);
        }
        let (files, blobs) = self.files(&sha).await?;
        let _write = self.write.lock().await;
        let data = self.reg.read().unwrap().dir.clone();
        let invalid = publish(&data, &sha, files)?;
        let reg = load(&data)?;
        *self.reg.write().unwrap() = Arc::new(reg);
        *self.blobs.lock().unwrap() = blobs;
        Ok(Some((sha, invalid)))
    }

    /// The YAML files directly in `path` at commit `sha`, and their bytes by blob sha. Walks one
    /// tree level per folder, so nothing outside `path` is listed or downloaded.
    async fn files(&self, sha: &str) -> Result<(Files, HashMap<String, Vec<u8>>), String> {
        let mut entries = self.tree(sha).await?;
        for dir in self.gh.path.split('/').filter(|s| !s.is_empty()) {
            let sub = entries
                .iter()
                .find(|e| e.kind == "tree" && e.path == dir)
                .ok_or_else(|| format!("Ordner {:?} fehlt im Repository", self.gh.path))?
                .sha
                .clone();
            entries = self.tree(&sub).await?;
        }
        let cache = self.blobs.lock().unwrap().clone();
        let mut files = Vec::new();
        let mut blobs = HashMap::new();
        // Regular files only, as the tarball import took them: no symlinks (120000), no submodules.
        for e in entries {
            if e.kind != "blob"
                || e.mode == "120000"
                || e.path.contains(['/', '\\'])
                || !is_yaml(Path::new(&e.path))
            {
                continue;
            }
            let bytes = if e.size.unwrap_or(0) > MAX_UPLOAD as u64 {
                Err(format!("{}: größer als 1 MiB", e.path))
            } else if let Some(b) = cache.get(&e.sha) {
                Ok(b.clone())
            } else {
                let accept = "application/vnd.github.raw+json";
                Ok(self
                    .get(&["git", "blobs", &e.sha], accept, MAX_UPLOAD)
                    .await?)
            };
            if let Ok(b) = &bytes {
                blobs.insert(e.sha, b.clone());
            }
            files.push((e.path, bytes));
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok((files, blobs))
    }

    /// One level of a git tree; a truncated listing is an error, never a silently shorter set.
    async fn tree(&self, sha: &str) -> Result<Vec<TreeEntry>, String> {
        let body = self
            .get(
                &["git", "trees", sha],
                "application/vnd.github+json",
                MAX_TREE,
            )
            .await?;
        let tree: Tree = serde_json::from_slice(&body)
            .map_err(|e| format!("Dateiliste von GitHub unlesbar: {e}"))?;
        if tree.truncated {
            return Err("Dateiliste von GitHub unvollständig (truncated)".into());
        }
        Ok(tree.tree)
    }

    /// GET `<api_url>/repos/<repo>/<path…>`, at most `max` bytes.
    async fn get(&self, path: &[&str], accept: &str, max: usize) -> Result<Vec<u8>, String> {
        // For messages: the endpoint without its argument (a sha or the branch).
        let what = path[..path.len() - 1].join("/");
        let mut url = Url::parse(&self.gh.api_url)
            .map_err(|_| format!("api_url {:?} ist ungültig", self.gh.api_url))?;
        let (owner, name) = self.gh.repo.split_once('/').unwrap_or_default();
        url.path_segments_mut()
            .map_err(|()| format!("api_url {:?} ist ungültig", self.gh.api_url))?
            .pop_if_empty()
            .extend(["repos", owner, name])
            .extend(path);
        let mut req = self
            .client
            .get(url)
            .header(ACCEPT, accept)
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        // Only the status goes into messages: a response body could echo request headers.
        let mut resp = req
            .send()
            .await
            .map_err(|e| format!("GitHub nicht erreichbar ({what}): {}", e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            let hint = if matches!(status.as_u16(), 401 | 403 | 404) {
                " – Repo, Branch und Token prüfen"
            } else {
                ""
            };
            return Err(format!("GitHub antwortet {status} ({what}){hint}"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("Download abgebrochen ({what}): {}", e.without_url()))?
        {
            if body.len() + chunk.len() > max {
                return Err(format!("{what}: größer als {} MiB", max >> 20));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

type Files = Vec<(String, Result<Vec<u8>, String>)>;

/// Writes the valid `files` (an invalid one falls back to its published version) to a new snapshot
/// and points `github/current` at it. Returns the invalid files.
fn publish(data: &Path, sha: &str, files: Files) -> Result<Vec<Invalid>, String> {
    let io = |e: std::io::Error| format!("github/: nicht schreibbar: {e}");
    let root = data.join("github");
    let old = published(data);
    let mut ids = HashSet::new();
    let mut keep = Vec::new();
    let mut invalid = Vec::new();
    for (name, new) in files {
        let entry = match new.and_then(|b| parse_entry(&name, b, PathBuf::new())) {
            Ok(e) => e,
            Err(error) => {
                invalid.push(Invalid {
                    file: name.clone(),
                    error,
                });
                let last = old.as_ref().and_then(|d| fs::read(d.join(&name)).ok());
                match last.and_then(|b| parse_entry(&name, b, PathBuf::new()).ok()) {
                    Some(e) => e,
                    None => continue,
                }
            }
        };
        if !ids.insert(entry.id.clone()) {
            invalid.push(Invalid {
                error: format!("{name}: doppelte id {:?}", entry.id),
                file: name,
            });
            continue;
        }
        keep.push((name, entry.bytes));
    }

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let snapshot = format!("{sha}-{millis}");
    let tmp = root.join(format!("{snapshot}.tmp"));
    fs::create_dir_all(&tmp).map_err(io)?;
    for (name, bytes) in &keep {
        write_atomic(&tmp.join(name), bytes).map_err(io)?;
    }
    fs::rename(&tmp, root.join(&snapshot)).map_err(io)?;
    write_atomic(&root.join(POINTER), snapshot.as_bytes()).map_err(io)?;

    // Nothing reads old snapshots any more: the served set is in memory.
    for e in fs::read_dir(&root).map_err(io)?.flatten() {
        let n = e.file_name();
        if n != POINTER && n != snapshot.as_str() {
            let _ = fs::remove_dir_all(e.path());
        }
    }
    Ok(invalid)
}

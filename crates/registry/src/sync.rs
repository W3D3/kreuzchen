//! GitHub as the source of the questionnaires: resolve the branch head, download its tarball when
//! it moved, validate every YAML file in `pfad` and publish the result atomically.
//!
//! On disk: `github/<sha>-<millis>/` holds one published set, `github/aktuell` names the live one.
//! `load` reads only through that pointer, so a set is either fully published or not at all.

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::ACCEPT;
use reqwest::Url;
use serde::Serialize;
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::{is_yaml, load, parse_entry, write_atomic, GitHub};
use crate::{Registry, MAX_UPLOAD};

const POINTER: &str = "aktuell";
const MAX_TARBALL: usize = 64 * 1024 * 1024;

/// The directory `github/aktuell` points at; `None` before the first sync.
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
    datei: String,
    fehler: String,
}

#[derive(Clone, Default)]
struct Status {
    /// Commit of the published set.
    sha: Option<String>,
    /// Unix seconds of the last successful check against GitHub.
    stand: Option<u64>,
    fehler: Option<String>,
    ungueltig: Vec<Invalid>,
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
        }
    }

    /// Syncs now, then every `intervall_minuten`, for as long as the server runs.
    pub fn spawn(self: Arc<Self>) {
        let every = Duration::from_secs(self.gh.intervall_minuten * 60);
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
                    st.ungueltig = invalid;
                }
                st.stand = Some(now());
                st.fehler = None;
            }
            Err(e) => {
                eprintln!("sync {}@{}: {e}", self.gh.repo, self.gh.branch);
                st.fehler = Some(e);
            }
        }
    }

    pub fn status_json(&self) -> serde_json::Value {
        let st = self.status.lock().unwrap().clone();
        json!({
            "quelle": "github",
            "repo": self.gh.repo,
            "branch": self.gh.branch,
            "pfad": self.gh.pfad,
            "sha": st.sha,
            "stand": st.stand,
            "fehler": st.fehler,
            "ungueltig": st.ungueltig,
        })
    }

    /// The new sha and its invalid files once published; `None` if the head has not moved.
    async fn check(&self, prev: &Status) -> Result<Option<(String, Vec<Invalid>)>, String> {
        let head = self
            .get(
                "commits",
                &self.gh.branch,
                "application/vnd.github.sha",
                100,
            )
            .await?;
        let sha = String::from_utf8_lossy(&head).trim().to_owned();
        if !is_sha(&sha) {
            return Err("GitHub lieferte keinen Commit für den Branch".into());
        }
        // The first run after a start always downloads: the invalid list is not kept on disk.
        if prev.stand.is_some() && prev.sha.as_deref() == Some(&sha) {
            return Ok(None);
        }
        let tarball = self
            .get("tarball", &sha, "application/vnd.github+json", MAX_TARBALL)
            .await?;
        let files = extract(&tarball, &self.gh.pfad)?;
        let _write = self.write.lock().await;
        let data = self.reg.read().unwrap().dir.clone();
        let invalid = publish(&data, &sha, files)?;
        let reg = load(&data)?;
        *self.reg.write().unwrap() = Arc::new(reg);
        Ok(Some((sha, invalid)))
    }

    /// GET `<api_url>/repos/<repo>/<what>/<arg>`, at most `max` bytes.
    async fn get(
        &self,
        what: &str,
        arg: &str,
        accept: &str,
        max: usize,
    ) -> Result<Vec<u8>, String> {
        let mut url = Url::parse(&self.gh.api_url)
            .map_err(|_| format!("api_url {:?} ist ungültig", self.gh.api_url))?;
        let (owner, name) = self.gh.repo.split_once('/').unwrap_or_default();
        url.path_segments_mut()
            .map_err(|()| format!("api_url {:?} ist ungültig", self.gh.api_url))?
            .pop_if_empty()
            .extend(["repos", owner, name, what, arg]);
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

/// The YAML files directly in `pfad` of a GitHub tarball, whose entries all sit in one top folder.
fn extract(tarball: &[u8], pfad: &str) -> Result<Files, String> {
    let bad = |e: std::io::Error| format!("Archiv von GitHub unlesbar: {e}");
    let want: Vec<&str> = pfad.split('/').filter(|s| !s.is_empty()).collect();
    let mut files = Vec::new();
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    for entry in archive.entries().map_err(bad)? {
        let entry = entry.map_err(bad)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path().map_err(bad)?.into_owned();
        let parts: Option<Vec<&str>> = path
            .components()
            .map(|c| match c {
                Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .collect();
        let Some((name, dir)) = parts.as_deref().and_then(|p| p.get(1..)?.split_last()) else {
            continue;
        };
        if dir != want || !is_yaml(Path::new(name)) {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .take(MAX_UPLOAD as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(bad)?;
        let bytes = if bytes.len() > MAX_UPLOAD {
            Err(format!("{name}: größer als 1 MiB"))
        } else {
            Ok(bytes)
        };
        files.push((name.to_string(), bytes));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Writes the valid `files` (an invalid one falls back to its published version) to a new snapshot
/// and points `github/aktuell` at it. Returns the invalid files.
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
            Err(fehler) => {
                invalid.push(Invalid {
                    datei: name.clone(),
                    fehler,
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
                fehler: format!("{name}: doppelte id {:?}", entry.id),
                datei: name,
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

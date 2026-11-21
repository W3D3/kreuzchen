use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString};
use argon2::Argon2;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Practice {
    pub user: String,
    pub key_hash: String,
    pub questionnaires: Vec<String>,
}

/// Where the questionnaires come from. One per deployment, never both.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    /// `definitions/`, managed through `/admin`.
    #[default]
    Local,
    /// A folder in a GitHub repo, synced into `github/` (see `sync`).
    Github(GitHub),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitHub {
    /// `owner/name`
    pub repo: String,
    pub branch: String,
    /// Folder of the YAML files inside the repo; empty for the top level.
    pub path: String,
    #[serde(default = "default_interval")]
    pub interval_minutes: u64,
    #[serde(default = "default_api", skip_serializing_if = "is_default_api")]
    pub api_url: String,
}

fn default_interval() -> u64 {
    5
}

fn default_api() -> String {
    "https://api.github.com".into()
}

fn is_default_api(url: &str) -> bool {
    url == default_api()
}

// deny_unknown_fields means serializing a Config back loses no data, only comments.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "is_local")]
    pub source: Source,
    pub practices: Vec<Practice>,
}

fn is_local(s: &Source) -> bool {
    *s == Source::Local
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub hash: String,
    pub bytes: Vec<u8>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Registry {
    pub dir: PathBuf,
    pub source: Source,
    pub practices: Vec<Practice>,
    pub entries: BTreeMap<String, Entry>,
}

/// A new practice key (32 random bytes, base64url) and its argon2id hash; only the hash is stored.
pub fn new_key() -> (String, String) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let key = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    // Same parameters as the API's dummy hash, so login timing matches.
    let hash = Argon2::default()
        .hash_password(key.as_bytes(), &SaltString::generate(&mut OsRng))
        .expect("hash key")
        .to_string();
    (key, hash)
}

/// `^[A-Za-z0-9_-]+$`
pub fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn load_entry(path: &Path) -> Result<Entry, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: nicht lesbar: {e}", path.display()))?;
    parse_entry(&path.display().to_string(), bytes, path.to_owned())
}

/// Parses and validates a definition exactly as `load` does; `name` prefixes error messages.
pub fn parse_entry(name: &str, bytes: Vec<u8>, path: PathBuf) -> Result<Entry, String> {
    let text = std::str::from_utf8(&bytes).map_err(|_| format!("{name}: nicht UTF-8"))?;
    // shortcut: 0.4 files (a Bibliothek repo not switched yet) load through an in-memory
    // upgrade while their bytes are served unchanged; drop once no 0.4 files are published.
    let q = kreuzchen_engine::load(&kreuzchen_engine::migrate::upgraded(text)).map_err(|errs| {
        let msgs: Vec<_> = errs.iter().map(|e| e.to_string()).collect();
        format!("{name}: {}", msgs.join("; "))
    })?;
    let d = q.def;
    if !safe_id(&d.id) {
        return Err(format!("{name}: unzulässige id {:?}", d.id));
    }
    Ok(Entry {
        hash: kreuzchen_engine::hash(&bytes),
        id: d.id,
        title: d.title,
        version: d.version,
        bytes,
        path,
    })
}

pub fn load(data: &Path) -> Result<Registry, String> {
    let cfg_path = data.join("config.yaml");
    let text = fs::read_to_string(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let config: Config =
        serde_norway::from_str(text).map_err(|e| format!("{}: {e}", cfg_path.display()))?;

    let dir = match &config.source {
        Source::Local => Some(data.join("definitions")),
        Source::Github(gh) => {
            check_github(gh).map_err(|e| format!("{}: source: {e}", cfg_path.display()))?;
            crate::sync::published(data)
        }
    };
    let files = match dir {
        Some(dir) => yaml_files(&dir).map_err(|e| format!("{}: {e}", dir.display()))?,
        None => Vec::new(),
    };

    let mut entries = BTreeMap::new();
    for path in files {
        let e = load_entry(&path)?;
        if let Some(prev) = entries.insert(e.id.clone(), e) {
            return Err(format!("{}: doppelte id {:?}", path.display(), prev.id));
        }
    }

    let mut users = HashSet::new();
    for p in &config.practices {
        if !users.insert(p.user.as_str()) {
            return Err(format!("doppelter Benutzer {:?}", p.user));
        }
        PasswordHash::new(&p.key_hash)
            .map_err(|e| format!("Benutzer {:?}: ungültiger key_hash: {e}", p.user))?;
        // From GitHub a questionnaire may vanish with the next sync; its assignments then wait.
        if config.source != Source::Local {
            continue;
        }
        if let Some(id) = p
            .questionnaires
            .iter()
            .find(|id| !entries.contains_key(*id))
        {
            return Err(format!(
                "Benutzer {:?}: unbekannter Fragebogen {id:?}",
                p.user
            ));
        }
    }
    Ok(Registry {
        dir: data.to_owned(),
        source: config.source,
        practices: config.practices,
        entries,
    })
}

/// The `*.yaml`/`*.yml` files directly in `dir`, sorted.
pub(crate) fn yaml_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_yaml(p))
        .collect();
    files.sort();
    Ok(files)
}

pub(crate) fn is_yaml(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml"))
}

fn check_github(gh: &GitHub) -> Result<(), String> {
    let part = |s: &str| {
        !s.is_empty()
            && s != "."
            && s != ".."
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
    };
    if !gh
        .repo
        .split_once('/')
        .is_some_and(|(o, r)| part(o) && part(r))
    {
        return Err(format!("repo {:?} ist nicht „owner/name“", gh.repo));
    }
    if gh.branch.is_empty() {
        return Err("branch fehlt".into());
    }
    if gh.interval_minutes == 0 {
        return Err("interval_minutes muss mindestens 1 sein".into());
    }
    if !(gh.api_url.starts_with("https://") || gh.api_url.starts_with("http://")) {
        return Err(format!(
            "api_url {:?} ist keine http(s)-Adresse",
            gh.api_url
        ));
    }
    Ok(())
}

/// Replaces `path` atomically: a reader sees the old or the new content, never a mix.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut f = fs::File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    fs::rename(&tmp, path)?;
    if let Some(dir) = path.parent() {
        fs::File::open(dir)?.sync_all()?;
    }
    Ok(())
}

/// Rewrites `config.yaml` with `practices`, atomically. Comments in the file are not kept.
pub fn write_config(dir: &Path, source: &Source, practices: Vec<Practice>) -> Result<(), String> {
    let config = Config {
        source: source.clone(),
        practices,
    };
    let text = serde_norway::to_string(&config).map_err(|e| e.to_string())?;
    write_atomic(&dir.join("config.yaml"), text.as_bytes())
        .map_err(|e| format!("config.yaml: nicht schreibbar: {e}"))
}

/// Upgrades a 0.4 data dir once: `definitionen/` becomes `definitions/`, and the `praxen` and
/// `fragebogen` keys of config.yaml become `practices` and `questionnaires` (the old file is kept
/// as `config.yaml.bak`). Every step is atomic and skips itself when done, so a rerun after a
/// crash finishes the job. Returns what it did, for the log.
pub fn migrate(data: &Path) -> Result<Vec<String>, String> {
    let mut log = Vec::new();
    let (old, new) = (data.join("definitionen"), data.join("definitions"));
    if old.is_dir() {
        if !new.exists() {
            fs::rename(&old, &new).map_err(|e| format!("{}: {e}", old.display()))?;
            log.push("definitionen/ → definitions/".to_string());
        } else {
            for e in fs::read_dir(&old).map_err(|e| format!("{}: {e}", old.display()))? {
                let e = e.map_err(|e| e.to_string())?;
                let to = new.join(e.file_name());
                if to.exists() {
                    log.push(format!(
                        "{} bleibt liegen: {} gibt es schon",
                        e.path().display(),
                        to.display()
                    ));
                } else {
                    fs::rename(e.path(), &to).map_err(|e| e.to_string())?;
                    log.push(format!("{} → {}", e.path().display(), to.display()));
                }
            }
            // Fails while files were left behind; they stay for the admin to sort out.
            let _ = fs::remove_dir(&old);
        }
    }
    let path = data.join("config.yaml");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(log);
    };
    if let Some(upgraded) = upgrade_config(&text).map_err(|e| format!("{}: {e}", path.display()))? {
        write_atomic(&data.join("config.yaml.bak"), text.as_bytes())
            .and_then(|()| write_atomic(&path, upgraded.as_bytes()))
            .map_err(|e| format!("{}: nicht schreibbar: {e}", path.display()))?;
        log.push(
            "config.yaml: praxen/fragebogen → practices/questionnaires (alt: config.yaml.bak)"
                .into(),
        );
    }
    Ok(log)
}

/// config.yaml with the English keys, `None` when it has them already. Edits the text so
/// comments stay; falls back to re-serializing when the edit does not parse to the same config.
fn upgrade_config(text: &str) -> Result<Option<String>, String> {
    use serde_norway::Value;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut v: Value = serde_norway::from_str(text).map_err(|e| e.to_string())?;
    let Some(m) = v.as_mapping_mut() else {
        return Ok(None);
    };
    let Some(mut list) = m.remove("praxen") else {
        return Ok(None);
    };
    for p in list.as_sequence_mut().into_iter().flatten() {
        if let Some(p) = p.as_mapping_mut() {
            if let Some(q) = p.remove("fragebogen") {
                p.insert("questionnaires".into(), q);
            }
        }
    }
    m.insert("practices".into(), list);
    let want: Config = serde_norway::from_value(v).map_err(|e| e.to_string())?;
    let want = serde_norway::to_string(&want).map_err(|e| e.to_string())?;

    let edited: String = text
        .lines()
        .map(|l| {
            let indent = l.len() - l.trim_start_matches([' ', '-']).len();
            let (head, rest) = l.split_at(indent);
            match (
                rest.strip_prefix("praxen:"),
                rest.strip_prefix("fragebogen:"),
            ) {
                (Some(r), _) if indent == 0 => format!("practices:{r}"),
                (_, Some(r)) => format!("{head}questionnaires:{r}"),
                _ => l.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let same = serde_norway::from_str::<Config>(&edited)
        .ok()
        .and_then(|c| serde_norway::to_string(&c).ok())
        .is_some_and(|c| c == want);
    Ok(Some(if same { edited } else { want }))
}

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString};
use argon2::Argon2;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Practice {
    pub user: String,
    pub key_hash: String,
    pub fragebogen: Vec<String>,
}

/// Where the questionnaires come from. One per deployment, never both.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    /// `definitionen/`, managed through `/admin`.
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
    pub praxen: Vec<Practice>,
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

/// Lowercase hex SHA-256, identical to the app's `definitions::hash`.
pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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
    let q = kreuzchen_engine::load(text).map_err(|errs| {
        let msgs: Vec<_> = errs.iter().map(|e| e.to_string()).collect();
        format!("{name}: {}", msgs.join("; "))
    })?;
    let d = q.def;
    if !safe_id(&d.id) {
        return Err(format!("{name}: unzulässige id {:?}", d.id));
    }
    Ok(Entry {
        hash: hash(&bytes),
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
        Source::Local => Some(data.join("definitionen")),
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
    for p in &config.praxen {
        if !users.insert(p.user.as_str()) {
            return Err(format!("doppelter Benutzer {:?}", p.user));
        }
        PasswordHash::new(&p.key_hash)
            .map_err(|e| format!("Benutzer {:?}: ungültiger key_hash: {e}", p.user))?;
        // From GitHub a questionnaire may vanish with the next sync; its assignments then wait.
        if config.source != Source::Local {
            continue;
        }
        if let Some(id) = p.fragebogen.iter().find(|id| !entries.contains_key(*id)) {
            return Err(format!(
                "Benutzer {:?}: unbekannter Fragebogen {id:?}",
                p.user
            ));
        }
    }
    Ok(Registry {
        dir: data.to_owned(),
        source: config.source,
        practices: config.praxen,
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
        praxen: practices,
    };
    let text = serde_norway::to_string(&config).map_err(|e| e.to_string())?;
    write_atomic(&dir.join("config.yaml"), text.as_bytes())
        .map_err(|e| format!("config.yaml: nicht schreibbar: {e}"))
}

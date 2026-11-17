use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use argon2::password_hash::PasswordHash;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Practice {
    pub user: String,
    pub key_hash: String,
    pub fragebogen: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub praxen: Vec<Practice>,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct Registry {
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

/// `^[A-Za-z0-9_-]+$`
pub fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn load_entry(path: &Path) -> Result<Entry, String> {
    let name = path.display();
    let bytes = fs::read(path).map_err(|e| format!("{name}: nicht lesbar: {e}"))?;
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
    })
}

pub fn load(data: &Path) -> Result<Registry, String> {
    let cfg_path = data.join("config.yaml");
    let text = fs::read_to_string(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let config: Config =
        serde_norway::from_str(text).map_err(|e| format!("{}: {e}", cfg_path.display()))?;

    let dir = data.join("definitionen");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                    e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml")
                })
        })
        .collect();
    files.sort();

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
        if let Some(id) = p.fragebogen.iter().find(|id| !entries.contains_key(*id)) {
            return Err(format!(
                "Benutzer {:?}: unbekannter Fragebogen {id:?}",
                p.user
            ));
        }
    }
    Ok(Registry {
        practices: config.praxen,
        entries,
    })
}

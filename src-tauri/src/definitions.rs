use kreuzchen_engine::{Questionnaire, load, schema::Definition};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[derive(Serialize)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub version: String,
    pub item_count: usize,
    pub scale_count: usize,
}

#[derive(Serialize)]
pub struct DefinitionEntry {
    pub file: String,
    pub hash: String,
    pub summary: Option<Summary>,
    pub error: Option<String>,
    pub locked: bool,
}

#[derive(Serialize)]
pub struct OpenedDefinition {
    pub file: String,
    pub hash: String,
    pub definition: Definition,
    pub locked: bool,
}

pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn check_hash(current: &str, expected: &str) -> Result<(), String> {
    if current == expected {
        Ok(())
    } else {
        Err("Der Fragebogen wurde geändert. Bitte neu starten.".into())
    }
}

/// Accepts a bare file name (no path, `..` or drive) ending in one of `exts` (lowercase, no dot).
pub fn check_name(name: &str, exts: &[&str]) -> Result<(), String> {
    let path = Path::new(name);
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    if !name.contains(['/', '\\', ':'])
        && !name.contains("..")
        && path.file_name() == Some(std::ffi::OsStr::new(name))
        && ext.is_some_and(|e| exts.contains(&e.as_str()))
    {
        Ok(())
    } else {
        Err("Ungültiger Dateiname".into())
    }
}

pub const YAML: &[&str] = &["yaml", "yml"];

pub fn summary(q: &Questionnaire) -> Summary {
    Summary {
        id: q.def.id.clone(),
        title: q.def.title.clone(),
        version: q.def.version.clone(),
        item_count: q.item_count(),
        scale_count: q.scale_count(),
    }
}

fn parse(bytes: &[u8]) -> Result<Questionnaire, String> {
    load_text(std::str::from_utf8(bytes).map_err(|_| "Datei ist nicht UTF-8".to_string())?)
}

/// `load` with the errors joined into one German message, one per line.
pub fn load_text(text: &str) -> Result<Questionnaire, String> {
    load(text).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn broken(file: String, error: String) -> DefinitionEntry {
    DefinitionEntry {
        file,
        hash: String::new(),
        summary: None,
        error: Some(error),
        locked: false,
    }
}

fn scan_entry(e: std::fs::DirEntry) -> Option<DefinitionEntry> {
    if !e.file_type().is_ok_and(|t| t.is_file()) {
        return None;
    }
    let path = e.path();
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if ext != "yaml" && ext != "yml" {
        return None;
    }
    let lossy = e.file_name().to_string_lossy().into_owned();
    let Ok(file) = e.file_name().into_string() else {
        return Some(broken(lossy, "Dateiname ist nicht UTF-8".into()));
    };
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(err) => return Some(broken(file, format!("Datei nicht lesbar: {err}"))),
    };
    let (summary, error) = match parse(&bytes) {
        Ok(q) => (Some(summary(&q)), None),
        Err(e) => (None, Some(e)),
    };
    Some(DefinitionEntry {
        hash: hash(&bytes),
        file,
        summary,
        error,
        locked: false,
    })
}

/// Valid definitions first (by title), then broken ones (by file name).
pub fn scan(dir: &Path) -> Vec<DefinitionEntry> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => return vec![broken(String::new(), format!("Ordner nicht lesbar: {e}"))],
    };
    let mut entries: Vec<DefinitionEntry> = rd
        .filter_map(|e| match e {
            Ok(e) => scan_entry(e),
            Err(e) => Some(broken(String::new(), format!("Ordner nicht lesbar: {e}"))),
        })
        .collect();
    entries.sort_by(|a, b| match (&a.summary, &b.summary) {
        (Some(x), Some(y)) => x.title.cmp(&y.title),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.file.cmp(&b.file),
    });
    entries
}

/// Local entries, then registry entries (locked), each sorted as by `scan`.
pub fn scan_all(local: &Path, registry: &Path) -> Vec<DefinitionEntry> {
    let locked = scan(registry)
        .into_iter()
        .map(|e| DefinitionEntry { locked: true, ..e });
    scan(local).into_iter().chain(locked).collect()
}

/// The entry with `id` in either folder: the one matching `prefer_hash` first, then the registry one.
pub fn find_by_id(
    local: &Path,
    registry: &Path,
    id: &str,
    prefer_hash: Option<&str>,
) -> Option<DefinitionEntry> {
    scan_all(local, registry)
        .into_iter()
        .filter(|e| e.summary.as_ref().is_some_and(|s| s.id == id))
        .min_by_key(|e| (Some(e.hash.as_str()) != prefer_hash, !e.locked))
}

pub fn open(dir: &Path, file: &str) -> Result<(Questionnaire, String), String> {
    check_name(file, YAML)?;
    let bytes = fs::read(dir.join(file)).map_err(|e| format!("Datei nicht lesbar: {e}"))?;
    Ok((parse(&bytes)?, hash(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const FIXTURE: &str = include_str!("../../examples/definitionen/beispiel.yaml");

    #[test]
    fn scan_filters_and_sorts() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        fs::write(p.join("beispiel.yaml"), FIXTURE).unwrap();
        fs::write(
            p.join("kaputt.yml"),
            FIXTURE.replace("scale: Y, key: normal", "scale: Z, key: normal"),
        )
        .unwrap();
        for f in ["README.txt", ".DS_Store", "fragebogen.yaml.bak"] {
            fs::write(p.join(f), "x").unwrap();
        }
        fs::create_dir(p.join("x.yaml")).unwrap();
        let e = scan(p);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].file, "beispiel.yaml");
        assert_eq!(e[0].summary.as_ref().unwrap().item_count, 4);
        assert_eq!(e[1].file, "kaputt.yml");
        assert!(e[1].summary.is_none());
        assert!(e[1].error.as_ref().unwrap().contains("Item 4: Skala „Z“"));
    }

    #[test]
    fn bom_crlf_file_scans() {
        let d = tempfile::tempdir().unwrap();
        let s = format!("\u{feff}{}", FIXTURE.replace('\n', "\r\n"));
        fs::write(d.path().join("a.yaml"), s).unwrap();
        assert!(scan(d.path())[0].summary.is_some());
    }

    #[test]
    fn open_rejects_traversal() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            open(d.path(), "../x.yaml").err().as_deref(),
            Some("Ungültiger Dateiname")
        );
    }

    #[test]
    fn open_rejects_colon() {
        let d = tempfile::tempdir().unwrap();
        assert!(open(d.path(), "C:x.yaml").is_err());
    }

    #[test]
    fn hash_changes_with_content() {
        assert_ne!(hash(b"a"), hash(b"b"));
        assert_eq!(hash(b"a"), hash(b"a"));
    }

    #[test]
    fn stale_hash_detected() {
        assert!(check_hash("a", "a").is_ok());
        assert_eq!(
            check_hash("a", "b").unwrap_err(),
            "Der Fragebogen wurde geändert. Bitte neu starten."
        );
    }

    #[test]
    fn uppercase_extension_listed() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("A.YAML"), FIXTURE).unwrap();
        assert!(scan(d.path())[0].summary.is_some());
    }

    #[test]
    fn non_utf8_content_is_broken_entry() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.yaml"), [0xff, 0xfe, 0xfd]).unwrap();
        let e = scan(d.path());
        assert_eq!(e[0].error.as_deref(), Some("Datei ist nicht UTF-8"));
    }

    #[test]
    fn scan_all_marks_registry_locked() {
        let (local, registry) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fs::write(local.path().join("a.yaml"), FIXTURE).unwrap();
        let other = FIXTURE.replace("id: beispiel", "id: fragebogen_a");
        fs::write(registry.path().join("fragebogen_a.yaml"), other).unwrap();
        let e = scan_all(local.path(), registry.path());
        let got: Vec<_> = e.iter().map(|e| (e.file.as_str(), e.locked)).collect();
        assert_eq!(got, [("a.yaml", false), ("fragebogen_a.yaml", true)]);
    }

    #[test]
    fn find_by_id_prefers_hash_then_registry() {
        let (local, registry) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let changed = FIXTURE.replace("Beispiel-Fragebogen", "Anders");
        fs::write(local.path().join("beispiel.yaml"), FIXTURE).unwrap();
        fs::write(registry.path().join("beispiel.yaml"), &changed).unwrap();
        let find = |h: Option<&str>| find_by_id(local.path(), registry.path(), "beispiel", h);
        let local_hash = hash(FIXTURE.as_bytes());
        assert!(!find(Some(&local_hash)).unwrap().locked);
        assert_eq!(
            find(Some(&hash(changed.as_bytes()))).unwrap().hash,
            hash(changed.as_bytes())
        );
        assert!(find(Some("andere")).unwrap().locked);
        assert!(find(None).unwrap().locked);
        assert!(find_by_id(local.path(), registry.path(), "nichts", None).is_none());
    }

    #[test]
    fn missing_dir_is_one_broken_entry() {
        let d = tempfile::tempdir().unwrap();
        let e = scan(&d.path().join("nope"));
        assert_eq!(e.len(), 1);
        assert!(
            e[0].error
                .as_ref()
                .unwrap()
                .starts_with("Ordner nicht lesbar")
        );
    }
}

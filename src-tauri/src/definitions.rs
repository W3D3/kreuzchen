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
}

#[derive(Serialize)]
pub struct OpenedDefinition {
    pub file: String,
    pub hash: String,
    pub definition: Definition,
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

fn parse(bytes: &[u8]) -> Result<Questionnaire, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Datei ist nicht UTF-8".to_string())?;
    load(text).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Valid definitions first (by title), then broken ones (by file name).
pub fn scan(dir: &Path) -> Vec<DefinitionEntry> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut entries: Vec<DefinitionEntry> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let file = e.file_name().into_string().ok()?;
            let ext = Path::new(&file).extension()?.to_str()?.to_lowercase();
            if ext != "yaml" && ext != "yml" {
                return None;
            }
            let bytes = fs::read(e.path()).ok()?;
            let (summary, error) = match parse(&bytes) {
                Ok(q) => (
                    Some(Summary {
                        id: q.def.id.clone(),
                        title: q.def.title.clone(),
                        version: q.def.version.clone(),
                        item_count: q.item_count(),
                        scale_count: q.scale_count(),
                    }),
                    None,
                ),
                Err(e) => (None, Some(e)),
            };
            Some(DefinitionEntry {
                hash: hash(&bytes),
                file,
                summary,
                error,
            })
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

pub fn open(dir: &Path, file: &str) -> Result<(Questionnaire, String), String> {
    if file.contains(['/', '\\']) || file.contains("..") {
        return Err("Ungültiger Dateiname".into());
    }
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
        for f in ["README.txt", ".DS_Store", "mbas.yaml.bak"] {
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
}

use kreuzchen_engine::{Grading, Input};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct QRef {
    pub id: String,
    pub title: String,
    pub version: String,
    pub hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SavedRecord {
    pub format: u32,
    pub kennung: String,
    pub created_at: String,
    pub questionnaire: QRef,
    pub input: Input,
    pub grading: Grading,
    pub text: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct SavedSummary {
    pub name: String,
    pub kennung: String,
    pub title: String,
    pub created_at: String,
}

const BAD_FILE: &str = "Diese Datei ist keine gültige Kreuzchen-Auswertung.";

fn check_name(name: &str) -> Result<(), String> {
    crate::definitions::check_name(name, &["json"])
}

/// Writes `dir/name` via a temporary file and rename, so readers never see half a file.
pub fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let tmp = dir.join(format!("{name}.tmp"));
    fs::write(&tmp, bytes)
        .and_then(|()| fs::rename(&tmp, dir.join(name)))
        .map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("Speichern fehlgeschlagen: {e}")
        })
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

pub fn save(dir: &Path, rec: &SavedRecord) -> Result<String, String> {
    let base = format!(
        "{}_{}",
        sanitize(&rec.created_at),
        sanitize(&rec.questionnaire.id)
    );
    let mut name = format!("{base}.json");
    let mut n = 1;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("{base}-{n}.json");
    }
    let json = serde_json::to_vec_pretty(rec).map_err(|e| e.to_string())?;
    write_atomic(dir, &name, &json)?;
    Ok(name)
}

pub fn list(dir: &Path) -> Vec<SavedSummary> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<SavedSummary> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            check_name(&name).ok()?;
            let rec: SavedRecord = serde_json::from_slice(&fs::read(e.path()).ok()?).ok()?;
            Some(SavedSummary {
                name,
                kennung: rec.kennung,
                title: rec.questionnaire.title,
                created_at: rec.created_at,
            })
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.name.cmp(&b.name)));
    out
}

pub fn load(dir: &Path, name: &str) -> Result<SavedRecord, String> {
    check_name(name)?;
    let bytes = fs::read(dir.join(name)).map_err(|e| format!("Datei nicht lesbar: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|_| BAD_FILE.to_string())
}

pub fn delete(dir: &Path, name: &str) -> Result<(), String> {
    check_name(name)?;
    fs::remove_file(dir.join(name)).map_err(|e| format!("Löschen fehlgeschlagen: {e}"))
}

pub fn export(dir: &Path, name: &str, dest: &Path) -> Result<(), String> {
    check_name(name)?;
    let src = dir.join(name);
    if fs::canonicalize(dest).ok() == fs::canonicalize(&src).ok() && dest.exists() {
        return Err("Die Auswertung kann nicht auf sich selbst exportiert werden.".into());
    }
    fs::copy(src, dest)
        .map(|_| ())
        .map_err(|e| format!("Export fehlgeschlagen: {e}"))
}

pub fn import(dir: &Path, src: &Path) -> Result<String, String> {
    let bytes = fs::read(src).map_err(|e| format!("Datei nicht lesbar: {e}"))?;
    let rec: SavedRecord = serde_json::from_slice(&bytes).map_err(|_| BAD_FILE.to_string())?;
    if rec.format != 1 {
        return Err(BAD_FILE.into());
    }
    save(dir, &rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs};

    fn rec(created_at: &str) -> SavedRecord {
        SavedRecord {
            format: 1,
            kennung: "A/B: *x".into(),
            created_at: created_at.into(),
            questionnaire: QRef {
                id: "beispiel".into(),
                title: "Beispiel".into(),
                version: "1".into(),
                hash: "abc".into(),
            },
            input: Input {
                fields: BTreeMap::new(),
                answers: BTreeMap::from([("a".to_string(), 1)]),
            },
            grading: Grading {
                fields: vec![],
                rows: vec![],
                rules: vec![],
                caveats: vec![],
                verdict: None,
                missing: 0,
                missing_items: vec![],
            },
            text: "Text".into(),
        }
    }

    #[test]
    fn export_onto_self_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let name = save(d.path(), &rec(T)).unwrap();
        assert_eq!(
            export(d.path(), &name, &d.path().join(&name))
                .err()
                .as_deref(),
            Some("Die Auswertung kann nicht auf sich selbst exportiert werden.")
        );
        assert_eq!(load(d.path(), &name).unwrap(), rec(T));
    }

    #[test]
    fn rejects_drive_style_names() {
        let d = tempfile::tempdir().unwrap();
        assert!(load(d.path(), "C:x.json").is_err());
    }

    const T: &str = "2026-11-11T22:23:05";

    #[test]
    fn save_load_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let name = save(d.path(), &rec(T)).unwrap();
        assert_eq!(load(d.path(), &name).unwrap(), rec(T));
    }

    #[test]
    fn name_from_timestamp_not_kennung() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            save(d.path(), &rec(T)).unwrap(),
            "2026-11-11T22-23-05_beispiel.json"
        );
        let mut r = rec(T);
        r.questionnaire.id = "../x".into();
        assert_eq!(save(d.path(), &r).unwrap(), "2026-11-11T22-23-05_---x.json");
    }

    #[test]
    fn collision_suffix() {
        let d = tempfile::tempdir().unwrap();
        let n: Vec<_> = (0..3).map(|_| save(d.path(), &rec(T)).unwrap()).collect();
        assert_eq!(
            n,
            [
                "2026-11-11T22-23-05_beispiel.json",
                "2026-11-11T22-23-05_beispiel-2.json",
                "2026-11-11T22-23-05_beispiel-3.json"
            ]
        );
        assert!(!d.path().join(format!("{}.tmp", n[0])).exists());
    }

    #[test]
    fn list_newest_first_skips_garbage() {
        let d = tempfile::tempdir().unwrap();
        save(d.path(), &rec("2026-01-01T10:00:00")).unwrap();
        save(d.path(), &rec("2026-02-01T10:00:00")).unwrap();
        fs::write(d.path().join("junk.json"), "{}").unwrap();
        let l = list(d.path());
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].created_at, "2026-02-01T10:00:00");
        assert_eq!(l[0].name, "2026-02-01T10-00-00_beispiel.json");
        assert_eq!(l[0].title, "Beispiel");
    }

    #[test]
    fn import_rejects_invalid() {
        let d = tempfile::tempdir().unwrap();
        let bad = d.path().join("bad.json");
        fs::write(&bad, r#"{"format":2}"#).unwrap();
        let msg = "Diese Datei ist keine gültige Kreuzchen-Auswertung.";
        assert_eq!(import(d.path(), &bad), Err(msg.to_string()));
        let mut r = rec(T);
        r.format = 2;
        fs::write(&bad, serde_json::to_vec(&r).unwrap()).unwrap();
        assert_eq!(import(d.path(), &bad), Err(msg.to_string()));

        let name = save(d.path(), &rec(T)).unwrap();
        let out = d.path().join("export.txt");
        export(d.path(), &name, &out).unwrap();
        let new = import(d.path(), &out).unwrap();
        assert_ne!(new, name);
        assert_eq!(load(d.path(), &new).unwrap(), rec(T));
    }

    #[test]
    fn rejects_bad_names() {
        let d = tempfile::tempdir().unwrap();
        let e = "Ungültiger Dateiname".to_string();
        assert_eq!(load(d.path(), "../a.json"), Err(e.clone()));
        assert_eq!(delete(d.path(), "a.txt"), Err(e.clone()));
        assert_eq!(export(d.path(), "a/b.json", &d.path().join("x")), Err(e));
    }

    #[test]
    fn delete_removes() {
        let d = tempfile::tempdir().unwrap();
        let name = save(d.path(), &rec(T)).unwrap();
        delete(d.path(), &name).unwrap();
        assert!(list(d.path()).is_empty());
    }
}

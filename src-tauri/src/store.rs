use kreuzchen_engine::{Grading, Input, schema::FieldValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{fs, io::Write, path::Path};

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

/// An unfinished entry, autosaved while typing; removed once its result is saved.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Draft {
    pub saved_at: String,
    pub questionnaire: QRef,
    pub input: Input,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct DraftSummary {
    pub name: String,
    pub kennung: String,
    pub title: String,
    pub saved_at: String,
    pub answered: usize,
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

/// Writes `dir/name` via a synced temporary file and rename, so a crash leaves the old or the new file, never half of one.
pub fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let tmp = dir.join(format!("{name}.tmp"));
    fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
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

/// `<stamp>_<id>.json`, with `-2`, `-3`, … when taken.
fn free_name(dir: &Path, stamp: &str, id: &str) -> String {
    let base = format!("{}_{}", sanitize(stamp), sanitize(id));
    let mut name = format!("{base}.json");
    let mut n = 1;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("{base}-{n}.json");
    }
    name
}

fn write_json(dir: &Path, name: &str, value: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    write_atomic(dir, name, &json)
}

pub fn save(dir: &Path, rec: &SavedRecord) -> Result<String, String> {
    let name = free_name(dir, &rec.created_at, &rec.questionnaire.id);
    write_json(dir, &name, rec)?;
    Ok(name)
}

/// Saves the result into `results`, then removes its draft from `drafts`.
pub fn save_finished(
    results: &Path,
    drafts: &Path,
    rec: &SavedRecord,
    draft: Option<&str>,
) -> Result<String, String> {
    let name = save(results, rec)?;
    if let Some(d) = draft {
        // The result is safe; a draft left behind is only clutter the user can delete.
        let _ = delete(drafts, d);
    }
    Ok(name)
}

/// Overwrites draft `name`, or creates a new one when `name` is `None`; returns its name.
pub fn save_draft(dir: &Path, name: Option<&str>, draft: &Draft) -> Result<String, String> {
    let name = match name {
        Some(n) => check_name(n).map(|()| n.to_owned())?,
        None => free_name(dir, &draft.saved_at, &draft.questionnaire.id),
    };
    write_json(dir, &name, draft)?;
    Ok(name)
}

/// Every readable `*.json` in `dir`; others are skipped.
fn read_all<T: DeserializeOwned>(dir: &Path) -> Vec<(String, T)> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    rd.flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            check_name(&name).ok()?;
            Some((
                name,
                serde_json::from_slice(&fs::read(e.path()).ok()?).ok()?,
            ))
        })
        .collect()
}

pub fn list(dir: &Path) -> Vec<SavedSummary> {
    let mut out: Vec<SavedSummary> = read_all::<SavedRecord>(dir)
        .into_iter()
        .map(|(name, rec)| SavedSummary {
            name,
            kennung: rec.kennung,
            title: rec.questionnaire.title,
            created_at: rec.created_at,
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.name.cmp(&b.name)));
    out
}

pub fn list_drafts(dir: &Path) -> Vec<DraftSummary> {
    let mut out: Vec<DraftSummary> = read_all::<Draft>(dir)
        .into_iter()
        .map(|(name, d)| DraftSummary {
            name,
            kennung: match d.input.fields.get("kennung") {
                Some(FieldValue::Text(k)) => k.clone(),
                _ => String::new(),
            },
            title: d.questionnaire.title,
            saved_at: d.saved_at,
            answered: d.input.answers.len(),
        })
        .collect();
    out.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then(a.name.cmp(&b.name)));
    out
}

pub fn load<T: DeserializeOwned>(dir: &Path, name: &str) -> Result<T, String> {
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
        assert_eq!(load::<SavedRecord>(d.path(), &name).unwrap(), rec(T));
    }

    #[test]
    fn rejects_drive_style_names() {
        let d = tempfile::tempdir().unwrap();
        assert!(load::<SavedRecord>(d.path(), "C:x.json").is_err());
    }

    const T: &str = "2026-11-11T22:23:05";

    #[test]
    fn save_load_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let name = save(d.path(), &rec(T)).unwrap();
        assert_eq!(load::<SavedRecord>(d.path(), &name).unwrap(), rec(T));
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
        assert_eq!(load::<SavedRecord>(d.path(), &new).unwrap(), rec(T));
    }

    #[test]
    fn rejects_bad_names() {
        let d = tempfile::tempdir().unwrap();
        let e = "Ungültiger Dateiname".to_string();
        assert_eq!(load::<SavedRecord>(d.path(), "../a.json"), Err(e.clone()));
        assert_eq!(delete(d.path(), "a.txt"), Err(e.clone()));
        assert_eq!(export(d.path(), "a/b.json", &d.path().join("x")), Err(e));
    }

    fn draft(saved_at: &str, answers: &[(&str, usize)]) -> Draft {
        let r = rec(saved_at);
        Draft {
            saved_at: saved_at.into(),
            questionnaire: r.questionnaire,
            input: Input {
                fields: BTreeMap::from([("kennung".into(), FieldValue::Text("K1".into()))]),
                answers: answers.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            },
        }
    }

    #[test]
    fn draft_roundtrip_and_overwrite_in_place() {
        let d = tempfile::tempdir().unwrap();
        let name = save_draft(d.path(), None, &draft(T, &[("1", 0)])).unwrap();
        assert_eq!(name, "2026-11-11T22-23-05_beispiel.json");
        assert_eq!(
            load::<Draft>(d.path(), &name).unwrap(),
            draft(T, &[("1", 0)])
        );

        let later = draft("2026-11-11T22:30:00", &[("1", 2), ("2", 1)]);
        assert_eq!(save_draft(d.path(), Some(&name), &later).unwrap(), name);
        assert_eq!(load::<Draft>(d.path(), &name).unwrap(), later);
        assert_eq!(
            fs::read_dir(d.path()).unwrap().count(),
            1,
            "no temp file left"
        );

        // A second entry started in the same second gets its own file.
        assert_ne!(save_draft(d.path(), None, &draft(T, &[])).unwrap(), name);
        assert_eq!(
            save_draft(d.path(), Some("../x.json"), &later),
            Err("Ungültiger Dateiname".into())
        );
    }

    #[test]
    fn drafts_list_newest_first_with_kennung_and_count() {
        let d = tempfile::tempdir().unwrap();
        save_draft(d.path(), None, &draft("2026-01-01T10:00:00", &[("1", 0)])).unwrap();
        save_draft(
            d.path(),
            None,
            &draft("2026-02-01T10:00:00", &[("1", 0), ("2", 3)]),
        )
        .unwrap();
        // A crash before the rename leaves only a `.tmp`, which is not a draft.
        fs::write(d.path().join("x.json.tmp"), "{").unwrap();
        fs::write(d.path().join("junk.json"), "{}").unwrap();
        let l = list_drafts(d.path());
        assert_eq!(
            l[0],
            DraftSummary {
                name: "2026-02-01T10-00-00_beispiel.json".into(),
                kennung: "K1".into(),
                title: "Beispiel".into(),
                saved_at: "2026-02-01T10:00:00".into(),
                answered: 2,
            }
        );
        assert_eq!(l.len(), 2);
        delete(d.path(), &l[0].name).unwrap();
        assert_eq!(list_drafts(d.path()).len(), 1);
    }

    #[test]
    fn saving_the_result_removes_its_draft_only() {
        let (results, drafts) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let mine = save_draft(drafts.path(), None, &draft(T, &[("1", 0)])).unwrap();
        let other = save_draft(drafts.path(), None, &draft(T, &[])).unwrap();
        let name = save_finished(results.path(), drafts.path(), &rec(T), Some(&mine)).unwrap();
        assert_eq!(load::<SavedRecord>(results.path(), &name).unwrap(), rec(T));
        let left: Vec<_> = list_drafts(drafts.path())
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(left, [other]);
        // Already gone (or never autosaved): the result is still saved.
        assert!(save_finished(results.path(), drafts.path(), &rec(T), Some(&mine)).is_ok());
        assert!(save_finished(results.path(), drafts.path(), &rec(T), None).is_ok());
        assert_eq!(list(results.path()).len(), 3);
    }

    #[test]
    fn delete_removes() {
        let d = tempfile::tempdir().unwrap();
        let name = save(d.path(), &rec(T)).unwrap();
        delete(d.path(), &name).unwrap();
        assert!(list(d.path()).is_empty());
    }
}

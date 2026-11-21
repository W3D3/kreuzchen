//! One-time upgrade of a 0.4 app data folder: English folder names, `subject_id` instead of
//! `kennung`, and definitions upgraded by `kreuzchen_engine::migrate`.
//! shortcut: delete once no 0.4 installs are left.
//!
//! Order, chosen so a crash at any point is finished by the next start:
//! 1. copy the data folder to `backup-0.4-<UTC time>` (built as `.partial`, renamed when complete);
//! 2. rename the folders, merging into existing new ones without overwriting;
//! 3. rewrite saved results and drafts, pointing them at the hash their definition will have;
//! 4. rewrite local definitions (only now, so step 3 can still see their old hashes);
//! 5. write the marker. Every write is atomic, and every step skips what is already done.
//!
//! Bibliothek files (`registry/`) stay byte-identical: the server compares their hashes, so a
//! rewrite would show a false "Update verfügbar". They are upgraded in memory when loaded
//! (`definitions::load_file`) and replaced by the server's English version once it has one.

use crate::store::write_atomic;
use kreuzchen_engine::{hash, migrate::upgrade};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MARKER: &str = "migrated-0.5.0";
const BACKUP: &str = "backup-0.4-";
const PARTIAL: &str = ".partial";
const FOLDERS: [(&str, &str); 3] = [
    ("definitionen", "definitions"),
    ("auswertungen", "results"),
    ("entwuerfe", "drafts"),
];

/// Upgrades `data` unless its marker says it is done. Returns the problems to show: files left
/// as they were (each still in the backup).
pub fn run(data: &Path) -> Vec<String> {
    if data.join(MARKER).exists() {
        return vec![];
    }
    let mut problems = Vec::new();
    if let Err(e) = steps(data, &mut problems) {
        // Not marked done: the next start tries again.
        problems.push(format!(
            "Die Daten von Kreuzchen 0.4 konnten nicht übernommen werden: {e}"
        ));
        return problems;
    }
    let note = problems.join("\n");
    if let Err(e) = write_atomic(data, MARKER, note.as_bytes()) {
        eprintln!("migration: {e}");
    }
    problems
}

fn steps(data: &Path, problems: &mut Vec<String>) -> Result<(), String> {
    let old_layout = FOLDERS.iter().any(|(old, _)| data.join(old).is_dir());
    if old_layout {
        backup(data)?;
    }
    for (old, new) in FOLDERS {
        merge(&data.join(old), &data.join(new))?;
    }
    let defs = data.join("definitions");
    let upgraded = upgrade_definitions(&defs, problems);
    let hashes: BTreeMap<String, String> = upgraded
        .iter()
        .map(|(_, old, new)| (hash(old.as_bytes()), hash(new.as_bytes())))
        .collect();
    for dir in ["results", "drafts"] {
        upgrade_records(&data.join(dir), &hashes, problems)?;
    }
    for (file, _, new) in &upgraded {
        write_atomic(&defs, file, new.as_bytes())?;
    }
    Ok(())
}

/// Backups, temp files, and what the webview keeps here (caches, storage, WebKit/WebView2 data).
fn skip_in_backup(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with(BACKUP)
        || n.ends_with(".tmp")
        || n == "storage"
        || ["cache", "webkit", "ebwebview", "mediakeys", "hsts-storage"]
            .iter()
            .any(|w| n.contains(w))
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let name = e.file_name();
        if skip_in_backup(&name.to_string_lossy()) {
            continue;
        }
        let t = e.file_type()?;
        if t.is_dir() {
            copy_dir(&e.path(), &to.join(&name))?;
        } else if t.is_file() {
            fs::copy(e.path(), to.join(&name))?;
        }
    }
    Ok(())
}

/// Copies `data` into `backup-0.4-<time>` once; a half-made copy from a crash is redone.
fn backup(data: &Path) -> Result<(), String> {
    let mut done = false;
    for e in fs::read_dir(data).map_err(|e| e.to_string())?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with(BACKUP) && name.ends_with(PARTIAL) {
            fs::remove_dir_all(e.path()).map_err(|e| format!("{name}: {e}"))?;
        } else {
            done |= name.starts_with(BACKUP);
        }
    }
    if done {
        return Ok(());
    }
    let name = format!("{BACKUP}{}", utc_stamp(SystemTime::now()));
    let partial = data.join(format!("{name}{PARTIAL}"));
    copy_dir(data, &partial)
        .and_then(|()| fs::rename(&partial, data.join(&name)))
        .map_err(|e| format!("Sicherungskopie fehlgeschlagen: {e}"))
}

/// `2026-11-21T10-15-00Z`: UTC, with `-` so it is a valid file name everywhere.
fn utc_stamp(t: SystemTime) -> String {
    let s = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = ((s / 86_400) as i64, s % 86_400);
    // Days to civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}-{:02}-{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// `name` in `dir`, or `stem-0.4.ext`, `stem-0.4-2.ext`, … when taken.
fn free(dir: &Path, name: &str) -> String {
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let ext = p
        .extension()
        .map_or_else(String::new, |e| format!(".{}", e.to_string_lossy()));
    let mut candidate = format!("{stem}-0.4{ext}");
    for n in 2.. {
        if !dir.join(&candidate).exists() {
            break;
        }
        candidate = format!("{stem}-0.4-{n}{ext}");
    }
    candidate
}

/// Moves `old` to `new`: a rename when `new` is missing, else file by file. A name taken by a
/// different file moves under a free name; an identical one is dropped (it is in the backup).
fn merge(old: &Path, new: &Path) -> Result<(), String> {
    if !old.is_dir() {
        return Ok(());
    }
    let err = |e: std::io::Error| format!("{}: {e}", old.display());
    if !new.exists() {
        return fs::rename(old, new).map_err(err);
    }
    for e in fs::read_dir(old).map_err(err)? {
        let e = e.map_err(err)?;
        let name = e.file_name().to_string_lossy().into_owned();
        let to = new.join(&name);
        if !to.exists() {
            fs::rename(e.path(), &to).map_err(err)?;
        } else if fs::read(e.path()).ok() == fs::read(&to).ok() {
            fs::remove_file(e.path()).map_err(err)?;
        } else {
            let other = free(new, &name);
            eprintln!(
                "migration: {} and {} both exist; the old one is now {other}",
                e.path().display(),
                to.display()
            );
            fs::rename(e.path(), new.join(other)).map_err(err)?;
        }
    }
    fs::remove_dir(old).map_err(err)
}

/// `(file, old text, new text)` for each local definition that needs the upgrade.
fn upgrade_definitions(dir: &Path, problems: &mut Vec<String>) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let file = e.file_name().to_string_lossy().into_owned();
        let yaml = Path::new(&file)
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("yaml") || x.eq_ignore_ascii_case("yml"));
        if !yaml || !e.path().is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(e.path()) else {
            problems.push(format!(
                "Fragebogen „{file}“: nicht lesbar, unverändert gelassen"
            ));
            continue;
        };
        match upgrade(&text) {
            Ok(Some(new)) => out.push((file, text, new)),
            Ok(None) => {}
            Err(m) => problems.push(format!("Fragebogen „{file}“ unverändert gelassen: {m}")),
        }
    }
    out
}

fn upgrade_records(
    dir: &Path,
    hashes: &BTreeMap<String, String>,
    problems: &mut Vec<String>,
) -> Result<(), String> {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let file = e.file_name().to_string_lossy().into_owned();
        if !file.to_lowercase().ends_with(".json") || !e.path().is_file() {
            continue;
        }
        let Some(mut v) = fs::read(e.path())
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        else {
            problems.push(format!("„{file}“ ist nicht lesbar und bleibt unverändert"));
            continue;
        };
        if upgrade_record(&mut v, hashes) {
            let json = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
            write_atomic(dir, &file, &json)?;
        }
    }
    Ok(())
}

fn rename_key(obj: &mut serde_json::Map<String, Value>, from: &str, to: &str) -> bool {
    if obj.contains_key(to) {
        return false;
    }
    obj.remove(from).map(|v| obj.insert(to.into(), v)).is_some()
}

/// Upgrades a saved result or draft as JSON; `hashes` maps old definition hashes to new ones.
/// Returns whether anything changed. Unknown keys are kept.
pub fn upgrade_record(v: &mut Value, hashes: &BTreeMap<String, String>) -> bool {
    let Some(rec) = v.as_object_mut() else {
        return false;
    };
    let mut changed = rename_key(rec, "kennung", "subject_id");
    if let Some(fields) = rec
        .get_mut("input")
        .and_then(|i| i.get_mut("fields"))
        .and_then(Value::as_object_mut)
    {
        changed |= rename_key(fields, "kennung", "subject_id");
    }
    let lines = rec
        .get_mut("grading")
        .and_then(|g| g.get_mut("fields"))
        .and_then(Value::as_array_mut);
    for id in lines.into_iter().flatten().filter_map(|l| l.get_mut("id")) {
        if id == "kennung" {
            *id = "subject_id".into();
            changed = true;
        }
    }
    if let Some(h) = rec.get_mut("questionnaire").and_then(|q| q.get_mut("hash"))
        && let Some(new) = h.as_str().and_then(|s| hashes.get(s))
    {
        *h = new.clone().into();
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Draft, SavedRecord};
    use std::path::PathBuf;

    const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/app-data-0.4.0");
    const RESULT: &str = "2026-11-17T14-30-00_beispiel.json";
    const RESULT_REGISTRY: &str = "2026-11-18T10-15-00_beispiel_normen.json";
    const DRAFT: &str = "2026-11-19T09-00-00_beispiel.json";

    fn old_data() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        copy_dir(Path::new(FIXTURE), d.path()).unwrap();
        // What the OS keeps in the app data folder must not land in the backup.
        for dir in ["WebKitCache", "CacheStorage", "storage", "mediakeys"] {
            fs::create_dir(d.path().join(dir)).unwrap();
            fs::write(d.path().join(dir).join("blob"), "x").unwrap();
        }
        fs::write(d.path().join("hsts-storage.sqlite"), "x").unwrap();
        d
    }

    fn read(p: PathBuf) -> String {
        fs::read_to_string(p).unwrap()
    }

    fn backups(d: &Path) -> Vec<String> {
        let mut b: Vec<_> = fs::read_dir(d)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(BACKUP))
            .collect();
        b.sort();
        b
    }

    #[test]
    fn upgrades_a_0_4_data_folder() {
        let d = old_data();
        let p = d.path();
        let old_def = read(p.join("definitionen/beispiel.yaml"));
        let old_registry = fs::read(p.join("registry/beispiel_normen.yaml")).unwrap();

        let problems = run(p);

        // Backup: the whole old folder, minus caches.
        let b = backups(p);
        assert_eq!(b.len(), 1, "{b:?}");
        let backup = p.join(&b[0]);
        assert_eq!(read(backup.join("definitionen/beispiel.yaml")), old_def);
        assert!(backup.join("auswertungen").join(RESULT).exists());
        assert!(backup.join("registry.json").exists());
        for skipped in [
            "WebKitCache",
            "CacheStorage",
            "storage",
            "mediakeys",
            "hsts-storage.sqlite",
        ] {
            assert!(!backup.join(skipped).exists(), "{skipped}");
        }

        for (old, new) in FOLDERS {
            assert!(!p.join(old).exists() && p.join(new).is_dir(), "{old}");
        }

        // The local definition is upgraded in place, comments kept.
        let def = read(p.join("definitions/beispiel.yaml"));
        assert!(
            def.starts_with("# Fictional example questionnaire"),
            "{def}"
        );
        assert!(def.contains("id: subject_id") && def.contains("when: missing <= 1"));
        assert!(def.contains("min_app: \"0.5.0\""));
        let q = crate::definitions::load_text(&def).unwrap();

        // Records: new keys, and the result still matches its (upgraded) definition.
        let rec: SavedRecord = serde_json::from_str(&read(p.join("results").join(RESULT))).unwrap();
        assert_eq!(rec.subject_id, "Anonym 17.11.2026 14:30");
        assert!(rec.input.fields.contains_key("subject_id"));
        assert_eq!(rec.grading.fields[0].id, "subject_id");
        assert_eq!(rec.questionnaire.hash, hash(def.as_bytes()));
        assert_eq!(q.grade(&rec.input).unwrap().rows, rec.grading.rows);
        let draft: Draft = serde_json::from_str(&read(p.join("drafts").join(DRAFT))).unwrap();
        assert_eq!(draft.questionnaire.hash, hash(def.as_bytes()));
        assert!(draft.input.fields.contains_key("subject_id"));

        // Bibliothek file: untouched, so its hash still matches the server and its result.
        assert_eq!(
            fs::read(p.join("registry/beispiel_normen.yaml")).unwrap(),
            old_registry
        );
        let rec: SavedRecord =
            serde_json::from_str(&read(p.join("results").join(RESULT_REGISTRY))).unwrap();
        assert_eq!(rec.questionnaire.hash, hash(&old_registry));
        let installed = crate::definitions::scan(&p.join("registry"));
        assert_eq!(installed[0].hash, hash(&old_registry));
        assert!(installed[0].error.is_none(), "{:?}", installed[0].error);
        let server = crate::registry::RemoteEntry {
            id: "beispiel_normen".into(),
            title: "Beispiel mit Normwerten".into(),
            version: "1".into(),
            hash: hash(&old_registry),
        };
        let rows = crate::registry::rows(Some(&[server]), &installed);
        assert_eq!(
            rows[0].state,
            crate::registry::RowState::Current,
            "no false update"
        );

        // Unmigratable files: untouched and reported.
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("broken.yaml") || problems[1].contains("broken.yaml"));
        assert!(problems.iter().any(|m| m.contains("truncated.json")));
        assert_eq!(
            read(p.join("definitions/broken.yaml")),
            read(backup.join("definitionen/broken.yaml"))
        );
        assert!(
            p.join("results/2026-11-10T08-00-00_truncated.json")
                .exists()
        );
        assert!(p.join(MARKER).exists());
    }

    fn snapshot(d: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(d: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for e in fs::read_dir(d).unwrap().flatten() {
                if e.path().is_dir() {
                    walk(&e.path(), out);
                } else {
                    out.insert(e.path(), fs::read(e.path()).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(d, &mut out);
        out
    }

    #[test]
    fn a_second_run_changes_nothing() {
        let d = old_data();
        run(d.path());
        let before = snapshot(d.path());
        assert!(run(d.path()).is_empty());
        // Even without the marker every step finds its work done.
        fs::remove_file(d.path().join(MARKER)).unwrap();
        run(d.path());
        let mut after = snapshot(d.path());
        after.insert(
            d.path().join(MARKER),
            before[&d.path().join(MARKER)].clone(),
        );
        assert_eq!(before, after);
    }

    #[test]
    fn a_rerun_after_a_crash_finishes_the_job() {
        // Crash after the backup and the results folder rename, before anything else.
        let d = old_data();
        let p = d.path();
        backup(p).unwrap();
        fs::rename(p.join("auswertungen"), p.join("results")).unwrap();
        // ...and a half-made second backup from a crash during a later copy attempt.
        fs::create_dir(p.join(format!("{BACKUP}x{PARTIAL}"))).unwrap();
        run(p);
        assert_eq!(
            backups(p).len(),
            1,
            "no second backup of half-migrated data"
        );
        let def = read(p.join("definitions/beispiel.yaml"));
        let rec: SavedRecord = serde_json::from_str(&read(p.join("results").join(RESULT))).unwrap();
        assert_eq!(rec.questionnaire.hash, hash(def.as_bytes()));

        // Crash after rewriting the records but before the definitions: the rerun still
        // upgrades the definition, and the records already point at its new hash.
        let d = old_data();
        let p = d.path();
        let old = read(p.join("definitionen/beispiel.yaml"));
        let new = upgrade(&old).unwrap().unwrap();
        for (o, n) in FOLDERS {
            merge(&p.join(o), &p.join(n)).unwrap();
        }
        let map = BTreeMap::from([(hash(old.as_bytes()), hash(new.as_bytes()))]);
        upgrade_records(&p.join("results"), &map, &mut vec![]).unwrap();
        run(p);
        assert_eq!(read(p.join("definitions/beispiel.yaml")), new);
        let rec: SavedRecord = serde_json::from_str(&read(p.join("results").join(RESULT))).unwrap();
        assert_eq!(rec.questionnaire.hash, hash(new.as_bytes()));
    }

    #[test]
    fn existing_new_folders_are_merged_without_overwriting() {
        let d = old_data();
        let p = d.path();
        fs::create_dir(p.join("results")).unwrap();
        fs::write(p.join("results").join(RESULT), "{\"mine\": true}").unwrap();
        let theirs = fs::read(p.join("auswertungen").join(RESULT_REGISTRY)).unwrap();
        fs::write(p.join("results").join(RESULT_REGISTRY), &theirs).unwrap();
        run(p);
        assert_eq!(read(p.join("results").join(RESULT)), "{\"mine\": true}");
        let moved = read(p.join("results/2026-11-17T14-30-00_beispiel-0.4.json"));
        assert!(moved.contains("\"subject_id\""));
        assert!(
            !p.join("results/2026-11-18T10-15-00_beispiel_normen-0.4.json")
                .exists()
        );
        assert!(!p.join("auswertungen").exists());
    }

    #[test]
    fn fresh_install_only_writes_the_marker() {
        let d = tempfile::tempdir().unwrap();
        assert!(run(d.path()).is_empty());
        assert!(backups(d.path()).is_empty());
        assert!(d.path().join(MARKER).exists());
    }

    #[test]
    fn utc_stamp_formats() {
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_795_256_100);
        assert_eq!(utc_stamp(t), "2026-11-21T10-15-00Z");
        assert_eq!(utc_stamp(UNIX_EPOCH), "1970-01-01T00-00-00Z");
    }
}

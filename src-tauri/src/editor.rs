use crate::definitions::{YAML, check_name, hash, load_text};
use crate::store::write_atomic;
use std::{fs, path::Path};

pub fn read_source(dir: &Path, file: &str) -> Result<String, String> {
    check_name(file, YAML)?;
    fs::read_to_string(dir.join(file)).map_err(|e| format!("Datei nicht lesbar: {e}"))
}

/// Validates `yaml` and writes it to `dir/file`, returning the new hash.
/// With `expected_hash` the file must still match it; without, it must not exist yet.
pub fn save(
    dir: &Path,
    file: &str,
    yaml: &str,
    expected_hash: Option<&str>,
) -> Result<String, String> {
    check_name(file, YAML)?;
    load_text(yaml)?;
    let path = dir.join(file);
    match expected_hash {
        Some(h) if fs::read(&path).map(|b| hash(&b)).ok().as_deref() != Some(h) => {
            return Err("Die Datei wurde inzwischen geändert.".into());
        }
        None if path.exists() => {
            return Err(format!("Die Datei „{file}“ gibt es schon."));
        }
        _ => {}
    }
    write_atomic(dir, file, yaml.as_bytes())?;
    Ok(hash(yaml.as_bytes()))
}

pub fn delete(dir: &Path, file: &str) -> Result<(), String> {
    check_name(file, YAML)?;
    fs::remove_file(dir.join(file)).map_err(|e| format!("Löschen fehlgeschlagen: {e}"))
}

/// Copies `file` to `<stem>-kopie.yaml` (or `-kopie-2` …) and returns the new name.
pub fn duplicate(dir: &Path, file: &str) -> Result<String, String> {
    let text = read_source(dir, file)?;
    let stem = Path::new(file)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let mut name = format!("{stem}-kopie.yaml");
    let mut n = 1;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("{stem}-kopie-{n}.yaml");
    }
    write_atomic(dir, &name, text.as_bytes())?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../examples/definitionen/beispiel.yaml");

    fn dir_with_fixture() -> (tempfile::TempDir, String) {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("b.yaml"), FIXTURE).unwrap();
        (d, hash(FIXTURE.as_bytes()))
    }

    #[test]
    fn save_with_current_hash_writes_and_returns_new_hash() {
        let (d, h) = dir_with_fixture();
        let new = FIXTURE.replace("Beispiel-Fragebogen", "Anders");
        let got = save(d.path(), "b.yaml", &new, Some(&h)).unwrap();
        assert_eq!(got, hash(new.as_bytes()));
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), new);
        assert!(!d.path().join("b.yaml.tmp").exists());
    }

    #[test]
    fn save_with_stale_hash_is_refused() {
        let (d, _) = dir_with_fixture();
        assert_eq!(
            save(d.path(), "b.yaml", FIXTURE, Some("alt")),
            Err("Die Datei wurde inzwischen geändert.".into())
        );
    }

    #[test]
    fn save_with_hash_of_deleted_file_is_refused() {
        let (d, h) = dir_with_fixture();
        delete(d.path(), "b.yaml").unwrap();
        assert!(save(d.path(), "b.yaml", FIXTURE, Some(&h)).is_err());
        assert!(!d.path().join("b.yaml").exists());
    }

    #[test]
    fn create_over_existing_file_is_refused() {
        let (d, _) = dir_with_fixture();
        let new = FIXTURE.replace("Beispiel-Fragebogen", "Anders");
        assert_eq!(
            save(d.path(), "b.yaml", &new, None),
            Err("Die Datei „b.yaml“ gibt es schon.".into())
        );
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn create_new_file() {
        let d = tempfile::tempdir().unwrap();
        save(d.path(), "neu.yaml", FIXTURE, None).unwrap();
        assert_eq!(read_source(d.path(), "neu.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn invalid_yaml_is_refused_and_file_untouched() {
        let (d, h) = dir_with_fixture();
        let bad = FIXTURE.replace("scale: Y, key: normal", "scale: Z, key: normal");
        let e = save(d.path(), "b.yaml", &bad, Some(&h)).unwrap_err();
        assert!(e.contains("Skala „Z“"), "{e}");
        let e = save(d.path(), "b.yaml", "id: [", Some(&h)).unwrap_err();
        assert!(e.starts_with("Zeile"), "{e}");
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn delete_removes_file() {
        let (d, _) = dir_with_fixture();
        delete(d.path(), "b.yaml").unwrap();
        assert!(!d.path().join("b.yaml").exists());
        assert!(
            delete(d.path(), "b.yaml")
                .unwrap_err()
                .starts_with("Löschen fehlgeschlagen")
        );
    }

    #[test]
    fn duplicate_picks_free_name_and_keeps_text() {
        let (d, _) = dir_with_fixture();
        assert_eq!(duplicate(d.path(), "b.yaml").unwrap(), "b-kopie.yaml");
        assert_eq!(duplicate(d.path(), "b.yaml").unwrap(), "b-kopie-2.yaml");
        assert_eq!(read_source(d.path(), "b-kopie-2.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn bad_names_are_refused() {
        let (d, _) = dir_with_fixture();
        let e = "Ungültiger Dateiname".to_string();
        for name in [
            "../b.yaml",
            "a/b.yaml",
            "a\\b.yaml",
            "C:b.yaml",
            "b.txt",
            "b",
            ".yaml",
        ] {
            assert_eq!(
                save(d.path(), name, FIXTURE, None).unwrap_err(),
                e,
                "{name}"
            );
            assert_eq!(read_source(d.path(), name).unwrap_err(), e, "{name}");
            assert_eq!(delete(d.path(), name).unwrap_err(), e, "{name}");
            assert_eq!(duplicate(d.path(), name).unwrap_err(), e, "{name}");
        }
        assert!(save(d.path(), "Neu.YML", FIXTURE, None).is_ok());
    }
}

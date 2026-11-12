use crate::definitions::{YAML, check_name, hash, load_text, scan};
use crate::store::write_atomic;
use std::{collections::HashSet, fs, path::Path};

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

/// Copies `file` under a fresh id (`{id}_kopie`, `{id}_kopie2` …) as `{id}.yaml`; returns the new name.
/// Saved results find their definition by id, so two files must never share one.
pub fn duplicate(dir: &Path, file: &str) -> Result<String, String> {
    let mut def =
        kreuzchen_engine::parse_definition(&read_source(dir, file)?).map_err(|e| e.to_string())?;
    let used: HashSet<String> = scan(dir)
        .into_iter()
        .filter_map(|e| e.summary.map(|s| s.id))
        .collect();
    let base = format!("{}_kopie", def.id);
    let mut id = base.clone();
    for n in 2.. {
        if !used.contains(&id) && !dir.join(format!("{id}.yaml")).exists() {
            break;
        }
        id = format!("{base}{n}");
    }
    def.id = id;
    let name = format!("{}.yaml", def.id);
    check_name(&name, YAML)?;
    write_atomic(dir, &name, kreuzchen_engine::to_yaml(&def).as_bytes())?;
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
    fn duplicate_gets_fresh_id_and_file() {
        let (d, _) = dir_with_fixture();
        assert_eq!(
            duplicate(d.path(), "b.yaml").unwrap(),
            "beispiel_kopie.yaml"
        );
        assert_eq!(
            duplicate(d.path(), "b.yaml").unwrap(),
            "beispiel_kopie2.yaml"
        );
        let ids: Vec<String> = ["b.yaml", "beispiel_kopie.yaml", "beispiel_kopie2.yaml"]
            .iter()
            .map(|f| {
                load_text(&read_source(d.path(), f).unwrap())
                    .unwrap()
                    .def
                    .id
            })
            .collect();
        assert_eq!(ids, ["beispiel", "beispiel_kopie", "beispiel_kopie2"]);
    }

    #[test]
    fn duplicate_skips_ids_used_under_other_file_names() {
        let (d, _) = dir_with_fixture();
        let other = FIXTURE.replace("id: beispiel", "id: beispiel_kopie");
        fs::write(d.path().join("anders.yaml"), other).unwrap();
        assert_eq!(
            duplicate(d.path(), "b.yaml").unwrap(),
            "beispiel_kopie2.yaml"
        );
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

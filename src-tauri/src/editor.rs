use crate::definitions::{DefinitionEntry, YAML, check_name, load_text, scan, scan_all};
use crate::store::write_atomic;
use kreuzchen_engine::hash;
use std::{collections::HashSet, fs, path::Path};

pub fn read_source(dir: &Path, file: &str) -> Result<String, String> {
    check_name(file, YAML)?;
    fs::read_to_string(dir.join(file)).map_err(|e| format!("Datei nicht lesbar: {e}"))
}

/// Validates `yaml` and writes it to `dir/file`, returning the new hash.
/// With `expected_hash` the file must still match it; without, it must not exist yet.
/// Refuses an id used by another file in `dir` or `registry`: ids must be unique across both folders.
/// Changing the id of an existing file needs `id_change_confirmed`, since its drafts and
/// saved results find it by the old id.
pub fn save(
    dir: &Path,
    registry: &Path,
    file: &str,
    yaml: &str,
    expected_hash: Option<&str>,
    id_change_confirmed: bool,
) -> Result<String, String> {
    check_name(file, YAML)?;
    let id = load_text(yaml)?.def.id;
    if scan(registry)
        .iter()
        .any(|e| e.summary.as_ref().is_some_and(|s| s.id == id))
    {
        return Err(format!(
            "Die ID „{id}“ gehört zu einem Fragebogen aus der Bibliothek."
        ));
    }
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
    let id_of = |e: &DefinitionEntry| e.summary.as_ref().map(|s| s.id.clone());
    let local = scan(dir);
    if let Some(other) = local
        .iter()
        .find(|e| e.file != file && id_of(e).as_ref() == Some(&id))
    {
        return Err(format!(
            "Die ID „{id}“ hat schon der Fragebogen in „{}“.",
            other.file
        ));
    }
    let old = local.iter().find(|e| e.file == file).and_then(id_of);
    if let Some(old) = old.filter(|old| *old != id && !id_change_confirmed) {
        return Err(format!(
            "Die ID soll von „{old}“ zu „{id}“ geändert werden. Das muss erst bestätigt werden."
        ));
    }
    write_atomic(dir, file, yaml.as_bytes())?;
    Ok(hash(yaml.as_bytes()))
}

pub fn delete(dir: &Path, file: &str) -> Result<(), String> {
    check_name(file, YAML)?;
    fs::remove_file(dir.join(file)).map_err(|e| format!("Löschen fehlgeschlagen: {e}"))
}

/// Copies `file` (from `registry` if `file_locked`, else `dir`) into `dir` under a fresh id
/// (`{id}_kopie`, `{id}_kopie2` …) as `{id}.yaml`; returns the new name.
/// Saved results find their definition by id, so no two files in either folder may share one.
pub fn duplicate(
    dir: &Path,
    registry: &Path,
    file_locked: bool,
    file: &str,
) -> Result<String, String> {
    let src = if file_locked { registry } else { dir };
    let mut def =
        kreuzchen_engine::parse_definition(&read_source(src, file)?).map_err(|e| e.to_string())?;
    let used: HashSet<String> = scan_all(dir, registry)
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

    /// A registry folder that does not exist (its temp parent is removed on drop).
    fn empty() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().path().join("registry")
    }

    fn dir_with_fixture() -> (tempfile::TempDir, String) {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("b.yaml"), FIXTURE).unwrap();
        (d, hash(FIXTURE.as_bytes()))
    }

    #[test]
    fn save_with_current_hash_writes_and_returns_new_hash() {
        let (d, h) = dir_with_fixture();
        let new = FIXTURE.replace("Beispiel-Fragebogen", "Anders");
        let got = save(d.path(), &empty(), "b.yaml", &new, Some(&h), false).unwrap();
        assert_eq!(got, hash(new.as_bytes()));
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), new);
        assert!(!d.path().join("b.yaml.tmp").exists());
    }

    #[test]
    fn save_with_stale_hash_is_refused() {
        let (d, _) = dir_with_fixture();
        assert_eq!(
            save(d.path(), &empty(), "b.yaml", FIXTURE, Some("alt"), false),
            Err("Die Datei wurde inzwischen geändert.".into())
        );
    }

    #[test]
    fn save_with_hash_of_deleted_file_is_refused() {
        let (d, h) = dir_with_fixture();
        delete(d.path(), "b.yaml").unwrap();
        assert!(save(d.path(), &empty(), "b.yaml", FIXTURE, Some(&h), false).is_err());
        assert!(!d.path().join("b.yaml").exists());
    }

    #[test]
    fn create_over_existing_file_is_refused() {
        let (d, _) = dir_with_fixture();
        let new = FIXTURE.replace("Beispiel-Fragebogen", "Anders");
        assert_eq!(
            save(d.path(), &empty(), "b.yaml", &new, None, false),
            Err("Die Datei „b.yaml“ gibt es schon.".into())
        );
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn create_new_file() {
        let d = tempfile::tempdir().unwrap();
        save(d.path(), &empty(), "neu.yaml", FIXTURE, None, false).unwrap();
        assert_eq!(read_source(d.path(), "neu.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn invalid_yaml_is_refused_and_file_untouched() {
        let (d, h) = dir_with_fixture();
        let bad = FIXTURE.replace("scale: Y, key: normal", "scale: Z, key: normal");
        let e = save(d.path(), &empty(), "b.yaml", &bad, Some(&h), false).unwrap_err();
        assert!(e.contains("Skala „Z“"), "{e}");
        let e = save(d.path(), &empty(), "b.yaml", "id: [", Some(&h), false).unwrap_err();
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
            duplicate(d.path(), &empty(), false, "b.yaml").unwrap(),
            "beispiel_kopie.yaml"
        );
        assert_eq!(
            duplicate(d.path(), &empty(), false, "b.yaml").unwrap(),
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
            duplicate(d.path(), &empty(), false, "b.yaml").unwrap(),
            "beispiel_kopie2.yaml"
        );
    }

    #[test]
    fn save_refuses_id_of_another_local_file() {
        let (d, h) = dir_with_fixture();
        fs::write(d.path().join("a.yaml"), FIXTURE).unwrap();
        let err = Err("Die ID „beispiel“ hat schon der Fragebogen in „a.yaml“.".to_string());
        assert_eq!(
            save(d.path(), &empty(), "neu.yaml", FIXTURE, None, false),
            err
        );
        assert!(!d.path().join("neu.yaml").exists());
        let edited = FIXTURE.replace("Beispiel-Fragebogen", "Anders");
        assert_eq!(
            save(d.path(), &empty(), "b.yaml", &edited, Some(&h), true),
            err
        );
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), FIXTURE);
    }

    #[test]
    fn id_change_needs_confirmation() {
        let (d, h) = dir_with_fixture();
        let renamed = FIXTURE.replace("id: beispiel", "id: beispiel_neu");
        assert_eq!(
            save(d.path(), &empty(), "b.yaml", &renamed, Some(&h), false),
            Err("Die ID soll von „beispiel“ zu „beispiel_neu“ geändert werden. Das muss erst bestätigt werden.".into())
        );
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), FIXTURE);
        save(d.path(), &empty(), "b.yaml", &renamed, Some(&h), true).unwrap();
        assert_eq!(read_source(d.path(), "b.yaml").unwrap(), renamed);
    }

    #[test]
    fn save_refuses_registry_id() {
        let (registry, _) = dir_with_fixture();
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            save(d.path(), registry.path(), "neu.yaml", FIXTURE, None, false),
            Err("Die ID „beispiel“ gehört zu einem Fragebogen aus der Bibliothek.".into())
        );
        assert!(!d.path().join("neu.yaml").exists());
    }

    #[test]
    fn duplicate_of_registry_file_gets_id_free_in_both() {
        let (local, registry) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let a = FIXTURE.replace("id: beispiel", "id: fragebogen_a");
        fs::write(registry.path().join("fragebogen_a.yaml"), &a).unwrap();
        let kopie = FIXTURE.replace("id: beispiel", "id: fragebogen_a_kopie");
        fs::write(local.path().join("fragebogen_a_kopie.yaml"), kopie).unwrap();
        let name = duplicate(local.path(), registry.path(), true, "fragebogen_a.yaml").unwrap();
        assert_eq!(name, "fragebogen_a_kopie2.yaml");
        let src = read_source(local.path(), &name).unwrap();
        assert_eq!(load_text(&src).unwrap().def.id, "fragebogen_a_kopie2");
        assert!(!registry.path().join(&name).exists());
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
                save(d.path(), &empty(), name, FIXTURE, None, false).unwrap_err(),
                e,
                "{name}"
            );
            assert_eq!(read_source(d.path(), name).unwrap_err(), e, "{name}");
            assert_eq!(delete(d.path(), name).unwrap_err(), e, "{name}");
            assert_eq!(
                duplicate(d.path(), &empty(), false, name).unwrap_err(),
                e,
                "{name}"
            );
        }
        delete(d.path(), "b.yaml").unwrap();
        assert!(save(d.path(), &empty(), "Neu.YML", FIXTURE, None, false).is_ok());
    }
}

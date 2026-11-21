//! Start-up upgrade of a 0.4.0 data dir (fixtures/registry-data-0.4.0).
use std::fs;
use std::path::Path;

use kreuzchen_engine::hash;
use kreuzchen_registry::{load, migrate};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/registry-data-0.4.0"
);

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().map(Result::unwrap) {
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &to.join(e.file_name()));
        } else {
            fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

fn old_data() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    copy(Path::new(FIXTURE), d.path());
    d
}

fn read(d: &Path, f: &str) -> String {
    fs::read_to_string(d.join(f)).unwrap()
}

#[test]
fn upgrades_a_0_4_data_dir_keeping_comments_and_key_hashes() {
    let d = old_data();
    let old_cfg = read(d.path(), "config.yaml");
    let old_def = fs::read(d.path().join("definitionen/beispiel.yaml")).unwrap();
    assert!(
        load(d.path()).is_err(),
        "0.4 config does not load unmigrated"
    );

    let log = migrate(d.path()).unwrap();
    assert_eq!(log.len(), 2, "{log:?}");
    assert!(!d.path().join("definitionen").exists());
    let cfg = read(d.path(), "config.yaml");
    assert_eq!(read(d.path(), "config.yaml.bak"), old_cfg);
    assert_eq!(
        cfg,
        old_cfg
            .replace("\npraxen:", "\npractices:")
            .replace("    fragebogen:", "    questionnaires:"),
        "only the keys change; comments and the key hash stay"
    );

    let reg = load(d.path()).unwrap();
    assert_eq!(reg.practices[0].questionnaires, ["beispiel"]);
    // The definition file is not rewritten: its hash, which apps compare, stays.
    assert_eq!(reg.entries["beispiel"].hash, hash(&old_def));

    assert!(migrate(d.path()).unwrap().is_empty(), "idempotent");
    assert_eq!(read(d.path(), "config.yaml"), cfg);
}

#[test]
fn a_rerun_after_a_crash_finishes_and_merges_without_overwriting() {
    let d = old_data();
    // Crashed after renaming the folder; meanwhile someone created definitions/ by hand too.
    fs::create_dir(d.path().join("definitions")).unwrap();
    fs::write(d.path().join("definitions/beispiel.yaml"), "mine").unwrap();
    fs::write(d.path().join("definitionen/other.yaml"), "other").unwrap();

    let log = migrate(d.path()).unwrap();
    assert!(log.iter().any(|l| l.contains("bleibt liegen")), "{log:?}");
    assert_eq!(read(d.path(), "definitions/beispiel.yaml"), "mine");
    assert_eq!(read(d.path(), "definitions/other.yaml"), "other");
    assert!(
        d.path().join("definitionen/beispiel.yaml").exists(),
        "conflict is kept, not lost"
    );
    assert!(read(d.path(), "config.yaml").contains("practices:"));
}

#[test]
fn flow_style_config_falls_back_to_reserializing() {
    let d = old_data();
    let hash_line = read(d.path(), "config.yaml");
    let key = hash_line.split('"').nth(1).unwrap();
    fs::write(
        d.path().join("config.yaml"),
        format!("praxen: [{{user: a, key_hash: \"{key}\", fragebogen: [beispiel]}}]\n"),
    )
    .unwrap();
    migrate(d.path()).unwrap();
    let reg = load(d.path()).unwrap();
    assert_eq!(reg.practices[0].key_hash, key);
    assert_eq!(reg.practices[0].questionnaires, ["beispiel"]);
}

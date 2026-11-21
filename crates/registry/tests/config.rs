use std::fs;
use std::path::Path;

use kreuzchen_engine::hash;
use kreuzchen_registry::load;
use tempfile::TempDir;

const EXAMPLE: &str = include_str!("../../../examples/definitions/beispiel.yaml");
const KEY_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21l$Y2hlY2tzdW1jaGVja3N1bWNoZWNrc3VtY2hlY2tzdW0";

fn def(id: &str) -> String {
    EXAMPLE.replacen("id: beispiel", &format!("id: {id:?}"), 1)
}

fn config(users: &[(&str, &str, &[&str])]) -> String {
    let mut s = String::from("practices:\n");
    for (user, key, ids) in users {
        s += &format!("  - user: {user}\n    key_hash: {key:?}\n    questionnaires: {ids:?}\n");
    }
    s
}

fn setup(cfg: &str, defs: &[(&str, String)]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("definitions")).unwrap();
    fs::write(dir.path().join("config.yaml"), cfg).unwrap();
    for (file, text) in defs {
        fs::write(dir.path().join("definitions").join(file), text).unwrap();
    }
    dir
}

fn err(dir: &Path) -> String {
    load(dir).unwrap_err()
}

fn ab() -> Vec<(&'static str, String)> {
    vec![
        ("a.yaml", def("questionnaire_a")),
        ("b.yml", def("questionnaire_b")),
    ]
}

#[test]
fn loads_valid_registry() {
    let dir = setup(
        &config(&[("practice1", KEY_HASH, &["questionnaire_a"])]),
        &ab(),
    );
    let r = load(dir.path()).unwrap();
    assert_eq!(r.entries.len(), 2);
    let e = &r.entries["questionnaire_a"];
    assert_eq!(e.hash, hash(&e.bytes));
    assert_eq!(r.practices[0].questionnaires, ["questionnaire_a"]);
}

#[test]
fn loads_uppercase_extension() {
    let dir = setup(&config(&[]), &[("A.YAML", def("questionnaire_a"))]);
    assert!(load(dir.path())
        .unwrap()
        .entries
        .contains_key("questionnaire_a"));
}

#[test]
fn refuses_invalid_definition() {
    let dir = setup(&config(&[]), &[("kaputt.yaml", "schema: 1\n".into())]);
    assert!(err(dir.path()).contains("kaputt.yaml"));
}

#[test]
fn refuses_duplicate_id() {
    let defs = [
        ("a.yaml", def("questionnaire_a")),
        ("b.yaml", def("questionnaire_a")),
    ];
    let dir = setup(&config(&[]), &defs);
    assert!(err(dir.path()).contains("questionnaire_a"));
}

#[test]
fn refuses_unknown_entitlement() {
    let dir = setup(
        &config(&[("practice1", KEY_HASH, &["questionnaire_x"])]),
        &ab(),
    );
    let e = err(dir.path());
    assert!(e.contains("practice1") && e.contains("questionnaire_x"));
}

#[test]
fn refuses_duplicate_user() {
    let cfg = config(&[("practice1", KEY_HASH, &[]), ("practice1", KEY_HASH, &[])]);
    let dir = setup(&cfg, &ab());
    assert!(err(dir.path()).contains("practice1"));
}

#[test]
fn refuses_bad_key_hash() {
    let dir = setup(&config(&[("practice1", "nope", &[])]), &ab());
    assert!(err(dir.path()).contains("practice1"));
}

#[test]
fn refuses_unsafe_id() {
    let dir = setup(&config(&[]), &[("x.yaml", def("../x"))]);
    assert!(err(dir.path()).contains("../x"));
}

#[test]
fn github_source_reads_only_the_published_set() {
    let source =
        "source:\n  type: github\n  repo: owner/fragen\n  branch: main\n  path: questionnaires\n";
    let cfg = source.to_string() + &config(&[("practice1", KEY_HASH, &["noch_nicht_da"])]);
    let r = load(setup(&cfg, &ab()).path()).unwrap();
    assert!(r.entries.is_empty(), "definitions/ is ignored");
    match r.source {
        kreuzchen_registry::Source::Github(gh) => {
            assert_eq!(gh.interval_minutes, 5);
            assert_eq!(gh.api_url, "https://api.github.com");
        }
        s => panic!("{s:?}"),
    }
}

#[test]
fn refuses_bad_github_source() {
    for (source, needle) in [
        (
            "type: github\n  repo: nur-ein-teil\n  branch: main\n  path: x",
            "nur-ein-teil",
        ),
        (
            "type: github\n  repo: o/../r\n  branch: main\n  path: x",
            "o/../r",
        ),
        (
            "type: github\n  repo: o/r\n  branch: \"\"\n  path: x",
            "branch",
        ),
        (
            "type: github\n  repo: o/r\n  branch: main\n  path: x\n  interval_minutes: 0",
            "interval",
        ),
        (
            "type: github\n  repo: o/r\n  branch: main\n  path: x\n  tokn: secret",
            "tokn",
        ),
        ("type: gitlab", "gitlab"),
    ] {
        let cfg = format!("source:\n  {source}\n") + &config(&[]);
        let e = err(setup(&cfg, &ab()).path());
        assert!(e.contains(needle), "{needle}: {e}");
    }
}

//! Checks definitions in the gitignored `local/` folder, which holds copyrighted questionnaires.
//! Skips itself where the folder is absent, as in CI.
//!
//! Every `local/definitionen/<name>.yaml` must load. Hand-scored cases live in
//! `local/faelle/<name>/*.json` as `{ "input": Input, "expect": { "<row id>": value }, "verdict": "…" }`;
//! `verdict` is optional.
use kreuzchen_engine::{load, Input, ResultRow};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn local(rel: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../local/")).join(rel)
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect();
    paths.sort();
    paths
}

#[derive(Deserialize)]
struct Case {
    input: Input,
    expect: BTreeMap<String, f64>,
    verdict: Option<String>,
}

#[test]
fn local_definitions_load_and_match_hand_cases() {
    let defs = files(&local("definitionen"), "yaml");
    if defs.is_empty() {
        eprintln!("skipped: no local/definitionen/*.yaml");
        return;
    }
    for def in defs {
        let yaml = fs::read_to_string(&def).unwrap();
        let q = load(&yaml).unwrap_or_else(|e| panic!("{}: {e:?}", def.display()));
        let name = def.file_stem().unwrap().to_string_lossy();
        for path in files(&local(&format!("faelle/{name}")), "json") {
            let case: Case = serde_json::from_str(&fs::read_to_string(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let g = q.grade(&case.input).unwrap();
            for (id, want) in &case.expect {
                let got = g.rows.iter().find_map(|r| match r {
                    ResultRow::Sum { id: i, value, .. } if i == id => Some(*value),
                    _ => None,
                });
                assert_eq!(got, Some(*want), "{} row {id}", path.display());
            }
            if let Some(v) = &case.verdict {
                assert_eq!(g.verdict.as_ref(), Some(v), "{} verdict", path.display());
            }
            eprintln!("checked {}", path.display());
        }
    }
}

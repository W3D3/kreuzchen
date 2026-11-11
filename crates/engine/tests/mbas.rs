//! Runs only where the copyrighted questionnaire exists in the gitignored `local/` folder.
use kreuzchen_engine::{load, Grading, Input, Questionnaire, ResultRow};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn local(rel: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../local/")).join(rel)
}

fn mbas() -> Option<Questionnaire> {
    let Ok(yaml) = std::fs::read_to_string(local("definitionen/mbas.yaml")) else {
        eprintln!("skipped: local/definitionen/mbas.yaml not present");
        return None;
    };
    Some(load(&yaml).unwrap_or_else(|e| panic!("mbas.yaml failed to load: {e:?}")))
}

fn sum(g: &Grading, id: &str) -> i64 {
    g.rows
        .iter()
        .find_map(|r| match r {
            ResultRow::Sum { id: i, value, .. } if i == id => Some(*value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no sum row {id}"))
}

fn rule(g: &Grading, id: &str) -> bool {
    g.rules.iter().find(|r| r.id == id).unwrap().met
}

/// Items 1-51 all at `idx`; `sev` stays at 0 (it has fewer options).
fn uniform(q: &Questionnaire, idx: usize) -> Grading {
    let mut answers: BTreeMap<String, usize> = (1..=51).map(|i| (i.to_string(), idx)).collect();
    answers.insert("sev".into(), 0);
    q.grade(&Input {
        fields: BTreeMap::new(),
        answers,
    })
    .unwrap()
}

#[test]
fn mbas_loads() {
    let Some(q) = mbas() else { return };
    assert_eq!(q.item_count(), 51);
    assert_eq!(q.scale_count(), 4);
}

#[test]
fn mbas_all_niemals() {
    let Some(q) = mbas() else { return };
    let g = uniform(&q, 0);
    for (id, v) in [("A", 52), ("B", 44), ("C", 12), ("D", 0), ("gesamt", 108)] {
        assert_eq!(sum(&g, id), v, "scale {id}");
    }
    assert!(rule(&g, "k1") && rule(&g, "k2"));
    assert_eq!(
        g.verdict.as_deref(),
        Some("Verdacht auf Asperger-Syndrom (Kriterium 2 erfüllt)")
    );
}

#[test]
fn mbas_all_immer() {
    let Some(q) = mbas() else { return };
    let g = uniform(&q, 4);
    for (id, v) in [("A", 16), ("B", 8), ("C", 36), ("D", 36), ("gesamt", 96)] {
        assert_eq!(sum(&g, id), v, "scale {id}");
    }
    assert!(!rule(&g, "k1"));
    assert_eq!(g.verdict.as_deref(), Some("Kein Verdacht nach MBAS"));
}

#[derive(Deserialize)]
struct Case {
    input: Input,
    expect: BTreeMap<String, i64>,
}

#[test]
fn mbas_hand_cases() {
    let Some(q) = mbas() else { return };
    let Ok(dir) = std::fs::read_dir(local("faelle")) else {
        eprintln!("skipped: local/faelle not present");
        return;
    };
    for path in dir.map(|e| e.unwrap().path()) {
        if path.extension().is_some_and(|e| e == "json") {
            let case: Case = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let g = q.grade(&case.input).unwrap();
            for (id, v) in &case.expect {
                assert_eq!(sum(&g, id), *v, "{} scale {id}", path.display());
            }
            eprintln!("checked {}", path.display());
        }
    }
}

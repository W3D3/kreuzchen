use kreuzchen_engine::{load, Grading, Input, NormValue, Questionnaire, ResultRow};

const YAML: &str = include_str!("../../../examples/definitions/beispiel_normen.yaml");

fn grade(q: &Questionnaire, answers: &[(&str, usize)]) -> Grading {
    let input = Input {
        fields: Default::default(),
        answers: answers.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    };
    q.grade(&input).unwrap()
}

/// All six items answered with the same answer index.
fn all(i: usize) -> Vec<(&'static str, usize)> {
    ["1", "2", "3", "4", "5", "6"]
        .into_iter()
        .map(|k| (k, i))
        .collect()
}

fn norm(g: &Grading, id: &str) -> Option<NormValue> {
    g.rows
        .iter()
        .find_map(|r| match r {
            ResultRow::Sum { id: i, norm, .. } if i == id => Some(norm.clone()),
            _ => None,
        })
        .unwrap()
}

fn t(value: i64, band: &str) -> Option<NormValue> {
    Some(NormValue {
        name: "T".into(),
        value,
        band: Some(band.into()),
    })
}

#[test]
fn looks_up_listed_raw_scores() {
    let q = load(YAML).unwrap();
    // A = 0 + 3 + 0, B = 0 + 0 + 3, gesamt = 6.
    let g = grade(&q, &all(0));
    assert_eq!(norm(&g, "A"), t(45, "unauffällig"));
    assert_eq!(norm(&g, "gesamt"), t(50, "unauffällig"));
    // A = 3 + 0 + 3, gesamt = 12.
    let g = grade(&q, &all(3));
    assert_eq!(norm(&g, "A"), t(60, "auffällig"));
    assert_eq!(norm(&g, "gesamt"), t(66, "auffällig"));
}

#[test]
fn gap_uses_next_lower_listed_raw_score() {
    let q = load(YAML).unwrap();
    // A = 1 + 3 + 0 = 4, B = 3, gesamt = 7: not listed, takes the row for 6.
    let mut answers = all(0);
    answers[0].1 = 1;
    let g = grade(&q, &answers);
    assert_eq!(norm(&g, "gesamt"), t(50, "unauffällig"));
}

#[test]
fn rows_without_a_table_have_no_norm() {
    let yaml = YAML.replacen("    A: { 0: 30", "    # A: { 0: 30", 1);
    let g = grade(&load(&yaml).unwrap(), &all(0));
    assert_eq!(norm(&g, "A"), None);
}

#[test]
fn copy_text_shows_norm_and_band() {
    let q = load(YAML).unwrap();
    let g = grade(&q, &all(3));
    let text = q.plain_text(&g);
    assert!(
        text.contains("Gesamtrohwert: 12 (T = 66, auffällig) (Cut-off 10) – über Cut-off\n"),
        "{text}"
    );
    assert!(text.contains("Skala A: 6 (T = 60, auffällig)\n"), "{text}");
}

#[test]
fn per_scale_missing_count() {
    let q = load(YAML).unwrap();
    let a_missing = "Skala A: fehlende Antworten, Skala nicht auswertbar.".to_string();
    let without = |k: &str| {
        all(0)
            .into_iter()
            .filter(|(i, _)| *i != k)
            .collect::<Vec<_>>()
    };
    let g = grade(&q, &without("2"));
    assert!(g.caveats.contains(&a_missing), "{:?}", g.caveats);
    let g = grade(&q, &without("4"));
    assert!(!g.caveats.contains(&a_missing), "{:?}", g.caveats);
}

fn errors(yaml: &str) -> Vec<String> {
    match load(yaml) {
        Ok(_) => panic!("expected Err"),
        Err(e) => e.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn rejects_table_for_unknown_scale() {
    let y = YAML.replacen("gesamt: { 0: 25", "gesammt: { 0: 25", 1);
    assert!(errors(&y)
        .contains(&"Normtabelle „gesammt“: keine Skala und kein abgeleiteter Wert".to_string()));
}

#[test]
fn rejects_unsorted_bands() {
    let y = YAML.replacen("from: 60", "from: -1", 1);
    assert!(errors(&y)
        .contains(&"Ausprägungsgrade müssen nach „from“ aufsteigend sortiert sein".to_string()));
}

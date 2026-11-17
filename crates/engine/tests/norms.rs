use kreuzchen_engine::{load, Grading, Input, NormValue, Questionnaire, ResultRow};

const YAML: &str = include_str!("../../../examples/definitionen/beispiel.yaml");

const NORMS: &str = "
norms:
  bands:
    - { from: 0, label: unauffällig }
    - { from: 60, label: auffällig }
  tables:
    X: { 0: 40, 2: 50, 4: 65 }
    gesamt: { 0: 30, 7: 70 }
";

fn with_norms(extra: &str) -> String {
    format!("{YAML}{NORMS}{extra}")
}

fn grade(q: &Questionnaire, answers: &[(&str, usize)]) -> (Grading, Input) {
    let input = Input {
        fields: Default::default(),
        answers: answers.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    };
    (q.grade(&input).unwrap(), input)
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
    let q = load(&with_norms("")).unwrap();
    let (g, _) = grade(&q, &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)]);
    assert_eq!(norm(&g, "X"), t(65, "auffällig"));
    assert_eq!(norm(&g, "gesamt"), t(70, "auffällig"));
    assert_eq!(norm(&g, "Y"), None);
}

#[test]
fn gap_uses_next_lower_listed_raw_score() {
    let q = load(&with_norms("")).unwrap();
    // X = 3 and gesamt = 3 are not listed: X takes the row for 2, gesamt the row for 0.
    let (g, _) = grade(&q, &[("1", 0), ("2", 0), ("3", 0), ("4", 0), ("zusatz", 0)]);
    assert_eq!(norm(&g, "X"), t(50, "unauffällig"));
    assert_eq!(norm(&g, "gesamt"), t(30, "unauffällig"));
}

#[test]
fn copy_text_shows_norm_and_band() {
    let q = load(&with_norms("")).unwrap();
    let (g, input) = grade(&q, &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)]);
    let text = q.plain_text(&input, &g);
    assert!(
        text.contains("Skala X: 4 (T = 65, auffällig) (Cut-off 4) – über Cut-off\n"),
        "{text}"
    );
    assert!(text.contains("Skala Y: 3 (Cut-off 3)"), "{text}");
}

#[test]
fn per_scale_missing_count() {
    let extra = "caveats:\n  - { when: X.fehlend > 0, text: X unvollständig }\n";
    let yaml = with_norms("").replacen("caveats:\n", extra, 1);
    let q = load(&yaml).unwrap();
    let (g, _) = grade(&q, &[("1", 0), ("3", 0), ("4", 0), ("zusatz", 0)]);
    assert!(
        g.caveats.contains(&"X unvollständig".to_string()),
        "{:?}",
        g.caveats
    );
    let (g, _) = grade(&q, &[("1", 0), ("2", 0), ("4", 0), ("zusatz", 0)]);
    assert!(!g.caveats.contains(&"X unvollständig".to_string()));
}

fn errors(yaml: &str) -> Vec<String> {
    match load(yaml) {
        Ok(_) => panic!("expected Err"),
        Err(e) => e.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn rejects_table_for_unknown_scale() {
    let y = with_norms("").replacen("gesamt: { 0: 30", "gesammt: { 0: 30", 1);
    assert!(errors(&y)
        .contains(&"Normtabelle „gesammt“: keine Skala und kein abgeleiteter Wert".to_string()));
}

#[test]
fn rejects_unsorted_bands() {
    let y = with_norms("").replacen("from: 60", "from: -1", 1);
    assert!(errors(&y)
        .contains(&"Ausprägungsgrade müssen nach „from“ aufsteigend sortiert sein".to_string()));
}

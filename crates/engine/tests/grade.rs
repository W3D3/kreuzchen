use kreuzchen_engine::schema::FieldValue;
use kreuzchen_engine::{load, GradeError, Grading, Input, ResultRow};

const YAML: &str = include_str!("../../../examples/definitionen/beispiel.yaml");

fn grade_with(
    yaml: &str,
    answers: &[(&str, usize)],
    fields: &[(&str, FieldValue)],
) -> Result<Grading, GradeError> {
    let q = load(yaml).unwrap();
    q.grade(&Input {
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        answers: answers.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    })
}

fn grade(answers: &[(&str, usize)], fields: &[(&str, FieldValue)]) -> Grading {
    grade_with(YAML, answers, fields).unwrap()
}

fn sum(g: &Grading, id: &str) -> (i64, bool) {
    g.rows
        .iter()
        .find_map(|r| match r {
            ResultRow::Sum {
                id: i, value, over, ..
            } if i == id => Some((*value, *over)),
            _ => None,
        })
        .unwrap()
}

fn rule(g: &Grading, id: &str) -> bool {
    g.rules.iter().find(|r| r.id == id).unwrap().met
}

fn answer(g: &Grading, id: &str) -> Option<String> {
    g.rows
        .iter()
        .find_map(|r| match r {
            ResultRow::Answer { id: i, answer, .. } if i == id => Some(answer.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn all_first() {
    let g = grade(
        &[("1", 0), ("2", 0), ("3", 0), ("4", 0), ("zusatz", 0)],
        &[],
    );
    assert_eq!(sum(&g, "X"), (3, false));
    assert_eq!(sum(&g, "Y"), (0, false));
    assert_eq!(sum(&g, "gesamt"), (3, false));
    assert!(!rule(&g, "r1"));
    assert!(!rule(&g, "r2"));
    assert!(rule(&g, "vollstaendig"));
    assert_eq!(g.verdict.as_deref(), Some("Unauffällig"));
    assert_eq!(answer(&g, "zusatz"), Some("Nein".to_string()));
    assert!(g.caveats.is_empty());
    assert_eq!(g.missing, 0);
}

#[test]
fn all_last() {
    let g = grade(
        &[("1", 3), ("2", 3), ("3", 3), ("4", 3), ("zusatz", 1)],
        &[],
    );
    assert_eq!(sum(&g, "X"), (3, false));
    assert_eq!(sum(&g, "Y"), (5, true));
    assert_eq!(sum(&g, "gesamt"), (8, true));
    assert!(rule(&g, "r1"));
    assert!(!rule(&g, "r2"));
    assert_eq!(g.verdict.as_deref(), Some("Auffällig (Regel 1)"));
    assert_eq!(
        g.caveats,
        vec!["Zusatzfrage mit Ja beantwortet.".to_string()]
    );
}

#[test]
fn exactly_at_cutoff() {
    let g = grade(
        &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)],
        &[],
    );
    assert_eq!(sum(&g, "X"), (4, true));
    assert_eq!(sum(&g, "Y"), (3, true));
    assert_eq!(sum(&g, "gesamt"), (7, true));
    assert!(rule(&g, "r2"));
    assert_eq!(g.verdict.as_deref(), Some("Auffällig (Regel 2)"));
}

#[test]
fn one_below_cutoff() {
    let g = grade(
        &[("1", 3), ("2", 3), ("3", 0), ("4", 3), ("zusatz", 0)],
        &[("erwachsen", FieldValue::Bool(true))],
    );
    assert_eq!(sum(&g, "X"), (3, false));
    assert_eq!(sum(&g, "gesamt"), (6, false));
    assert_eq!(g.verdict.as_deref(), Some("Unauffällig"));
    assert_eq!(g.caveats, vec!["Nur für Kinder normiert.".to_string()]);
}

#[test]
fn missing_answers() {
    let g = grade(&[("1", 3)], &[]);
    assert_eq!(sum(&g, "X"), (3, false));
    assert_eq!(sum(&g, "Y"), (0, false));
    assert_eq!(g.missing, 3);
    assert_eq!(g.missing_items, vec!["2", "3", "4"]);
    assert!(!rule(&g, "vollstaendig"));
    assert_eq!(
        g.verdict.as_deref(),
        Some("Nicht auswertbar: zu viele fehlende Antworten")
    );
    assert_eq!(answer(&g, "zusatz"), None);
}

#[test]
fn strict_compare() {
    let yaml = YAML.replace("\">=\"", "\">\"");
    let g = grade_with(
        &yaml,
        &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)],
        &[],
    )
    .unwrap();
    assert_eq!(sum(&g, "X"), (4, false));
    assert_eq!(sum(&g, "gesamt"), (7, false));
    assert_eq!(g.verdict.as_deref(), Some("Unauffällig"));
}

#[test]
fn unknown_item() {
    assert_eq!(
        grade_with(YAML, &[("9", 0)], &[]).unwrap_err(),
        GradeError::UnknownItem("9".into())
    );
}

#[test]
fn out_of_range() {
    assert_eq!(
        grade_with(YAML, &[("1", 4)], &[]).unwrap_err(),
        GradeError::AnswerOutOfRange {
            item: "1".into(),
            index: 4
        }
    );
}

#[test]
fn unknown_field() {
    assert_eq!(
        grade_with(YAML, &[], &[("foo", FieldValue::Bool(true))]).unwrap_err(),
        GradeError::UnknownField("foo".into())
    );
}

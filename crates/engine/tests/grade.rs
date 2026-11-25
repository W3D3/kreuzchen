use kreuzchen_engine::schema::FieldValue;
use kreuzchen_engine::{load, GradeError, Grading, Input, ResultRow};

const YAML: &str = include_str!("../../../examples/definitions/beispiel.yaml");

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
            } if i == id => Some((*value as i64, *over)),
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
        &[
            (
                "subject_id",
                FieldValue::Text("Anonym 11.11.2026 22:23".into()),
            ),
            ("alter", FieldValue::Num(9.0)),
        ],
    );
    assert_eq!(sum(&g, "X"), (4, true));
    assert_eq!(sum(&g, "Y"), (3, true));
    assert_eq!(sum(&g, "gesamt"), (7, true));
    assert!(rule(&g, "r2"));
    assert_eq!(g.verdict.as_deref(), Some("Auffällig (Regel 2)"));
    let fields: Vec<_> = g
        .fields
        .iter()
        .map(|f| (f.id.as_str(), f.label.as_str(), f.value.as_str()))
        .collect();
    assert_eq!(
        fields,
        [
            ("subject_id", "Kennung", "Anonym 11.11.2026 22:23"),
            ("alter", "Alter", "9"),
            ("erwachsen", "Volljährig", "Nein")
        ]
    );
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

#[test]
fn wrong_field_type() {
    assert_eq!(
        grade_with(YAML, &[], &[("erwachsen", FieldValue::Text("true".into()))]).unwrap_err(),
        GradeError::WrongFieldType("erwachsen".into())
    );
}

#[test]
fn rows_in_result_order() {
    let g = grade(
        &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)],
        &[],
    );
    let sum = |id: &str, label: &str, value, cutoff| ResultRow::Sum {
        id: id.into(),
        label: label.into(),
        value,
        cutoff: Some(cutoff),
        over: true,
        norm: None,
    };
    assert_eq!(
        g.rows,
        vec![
            sum("gesamt", "Gesamtsumme", 7.0, 7),
            sum("X", "Skala X", 4.0, 4),
            sum("Y", "Skala Y", 3.0, 3),
            ResultRow::Answer {
                id: "zusatz".into(),
                label: "Zusatzfrage".into(),
                answer: Some("Nein".into()),
            },
        ]
    );
}

#[test]
fn unset_number_field_is_eval_error() {
    let yaml = YAML.replace("erwachsen, text:", "alter > 24, text:");
    let yaml = yaml.replace("{ when: erwachsen,", "{ when: alter > 24,");
    match grade_with(&yaml, &[], &[]).unwrap_err() {
        GradeError::Eval(m) => assert!(m.contains("„alter“ hat keinen Wert"), "{m}"),
        e => panic!("{e:?}"),
    }
}

#[test]
fn derived_mean_keeps_decimals() {
    let yaml = YAML.replacen("value: X + Y", "value: (X + Y) / 2", 1);
    let answers = [("1", 0), ("2", 0), ("3", 0), ("4", 0), ("zusatz", 0)];
    let g = grade_with(&yaml, &answers, &[]).unwrap();
    let mean = g.rows.iter().find_map(|r| match r {
        ResultRow::Sum { id, value, .. } if id == "gesamt" => Some(*value),
        _ => None,
    });
    assert_eq!(mean, Some(1.5));
    let q = load(&yaml).unwrap();
    assert!(q.plain_text(&g).contains("Gesamtsumme: 1,5 (Cut-off 7)\n"));
}

#[test]
fn division_by_zero_is_an_error() {
    let yaml = YAML.replacen("value: X + Y", "value: X / Y", 1);
    let answers = [("1", 0), ("2", 0), ("3", 0), ("4", 0), ("zusatz", 0)];
    assert!(matches!(
        grade_with(&yaml, &answers, &[]),
        Err(GradeError::Eval(m)) if m.contains("Division durch 0")
    ));
}

#[test]
fn age_rule_works_with_and_without_an_age() {
    let yaml = YAML.replace(
        r#"  - { when: erwachsen, text: "Nur für Kinder normiert." }"#,
        r#"  - { when: alter.present and alter >= 18, text: "Nur für Kinder normiert." }
  - { when: not alter.present, text: "Alter fehlt." }"#,
    );
    let answers = [("1", 0), ("2", 0), ("3", 0), ("4", 0), ("zusatz", 0)];
    let g = grade_with(&yaml, &answers, &[("alter", FieldValue::Num(30.0))]).unwrap();
    assert_eq!(g.caveats, vec!["Nur für Kinder normiert.".to_string()]);
    let g = grade_with(&yaml, &answers, &[("alter", FieldValue::Num(12.0))]).unwrap();
    assert!(g.caveats.is_empty());
    let g = grade_with(&yaml, &answers, &[]).unwrap();
    assert_eq!(g.caveats, vec!["Alter fehlt.".to_string()]);
}

#[test]
fn min_app_is_checked_before_the_full_load() {
    use kreuzchen_engine::needs_newer_app;
    let newer = YAML.replacen("min_app: \"0.5.0\"", "min_app: \"0.10.0\"", 1);
    assert_eq!(needs_newer_app(&newer, "0.9.3").as_deref(), Some("0.10.0"));
    assert_eq!(needs_newer_app(&newer, "0.10.0"), None);
    assert_eq!(needs_newer_app(YAML, "0.5.0"), None);
    // A future key the schema rejects does not hide the version hint.
    let future = format!("{newer}\nfuture_key: 1\n");
    assert!(load(&future).is_err());
    assert_eq!(needs_newer_app(&future, "0.9.3").as_deref(), Some("0.10.0"));
    assert!(load(&newer).is_ok());
    assert!(load(&YAML.replacen("min_app: \"0.5.0\"", "min_app: new", 1)).is_err());
}

#[test]
fn count_is_a_derived_number() {
    use kreuzchen_engine::needs_newer_app;
    let yaml = YAML
        .replacen("min_app: \"0.5.0\"", "min_app: \"0.6.0\"", 1)
        .replacen(
            "derived:\n",
            "derived:\n  - { id: anzahl, label: Anzahl, value: \"count(X.over, Y.over, zusatz == \\\"Ja\\\", alter.present and alter >= 18)\", cutoff: 2 }\n",
            1,
        )
        .replacen("result: [", "result: [anzahl, ", 1)
        .replacen("when: missing <= 1", "when: missing <= 1 and anzahl < 4", 1);
    assert_eq!(needs_newer_app(&yaml, "0.5.0").as_deref(), Some("0.6.0"));
    assert_eq!(needs_newer_app(&yaml, "0.6.0"), None);
    // Nothing answered: unanswered items and an empty age count as false, not as errors.
    let g = grade_with(&yaml, &[], &[]).unwrap();
    assert_eq!(sum(&g, "anzahl"), (0, false));
    let all = [("1", 3), ("2", 0), ("3", 3), ("4", 3), ("zusatz", 1)];
    let g = grade_with(&yaml, &all, &[("alter", FieldValue::Num(40.0))]).unwrap();
    assert_eq!(sum(&g, "anzahl"), (4, true));
    assert!(!rule(&g, "vollstaendig"));
    let g = grade_with(&yaml, &[("zusatz", 1)], &[("alter", FieldValue::Num(9.0))]).unwrap();
    assert_eq!(sum(&g, "anzahl"), (1, false));
    assert!(g.verdict.is_some());
}

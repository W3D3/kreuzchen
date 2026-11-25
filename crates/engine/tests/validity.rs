use kreuzchen_engine::schema::FieldValue;
use kreuzchen_engine::{
    load, needs_newer_app, parse_definition, to_yaml, Grading, Input, ResultRow,
};

const YAML: &str = include_str!("../../../examples/definitions/beispiel.yaml");
const NORMS: &str = include_str!("../../../examples/definitions/beispiel_normen.yaml");

/// `beispiel.yaml` with three criteria: a rule, a per-scale limit and a `count(…)`.
fn with_validity() -> String {
    YAML.replacen("min_app: \"0.5.0\"", "min_app: \"0.7.0\"", 1)
        .replacen(
            "verdict:\n",
            "validity:\n  - { require: vollstaendig, invalid: \"Mehr als 1 Antwort fehlt.\" }\n  - { require: X.missing == 0, invalid: \"Skala X unvollständig.\" }\n  - { require: 'count(X.missing > 0, Y.missing > 0) < 2', invalid: \"Beide Skalen unvollständig.\" }\nverdict:\n",
            1,
        )
}

fn grade(
    yaml: &str,
    answers: &[(&str, usize)],
    fields: &[(&str, FieldValue)],
) -> (Grading, String) {
    let q = load(yaml).unwrap();
    let g = q
        .grade(&Input {
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            answers: answers.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        })
        .unwrap();
    let text = q.plain_text(&g);
    (g, text)
}

fn errors(yaml: &str) -> Vec<String> {
    match load(yaml) {
        Ok(_) => panic!("expected Err"),
        Err(e) => e.into_iter().map(|e| e.message).collect(),
    }
}

const ALL: [(&str, usize); 5] = [("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)];

#[test]
fn valid_result_keeps_its_verdict() {
    let (g, _) = grade(&with_validity(), &ALL, &[]);
    assert!(g.invalid.is_empty());
    assert_eq!(g.verdict.as_deref(), Some("Auffällig (Regel 2)"));
}

#[test]
fn one_failed_criterion_replaces_the_verdict() {
    // Item 1 (scale X) unanswered: one missing answer is allowed in total, none in X.
    let (g, text) = grade(
        &with_validity(),
        &ALL[1..],
        &[("erwachsen", FieldValue::Bool(true))],
    );
    assert_eq!(g.invalid, vec!["Skala X unvollständig."]);
    assert_eq!(g.verdict, None);
    // Scores, rules and caveats are still there.
    assert!(g.rules.iter().any(|r| r.id == "vollstaendig" && r.met));
    assert!(matches!(&g.rows[0], ResultRow::Sum { id, .. } if id == "gesamt"));
    assert_eq!(g.caveats, vec!["Nur für Kinder normiert."]);
    let warn = text.find("Warnung: Nur für Kinder normiert.").unwrap();
    let invalid = text
        .find("Nicht auswertbar: Skala X unvollständig.")
        .unwrap();
    assert!(warn < invalid, "{text}");
    assert!(!text.contains("Ergebnis:"), "{text}");
    assert!(
        text.contains("Die Werte sind nicht interpretierbar.\nGesamtsumme:"),
        "{text}"
    );
}

#[test]
fn every_failed_criterion_is_listed_in_order() {
    let (g, _) = grade(&with_validity(), &[("2", 1), ("4", 1)], &[]);
    assert_eq!(g.missing, 2);
    assert_eq!(
        g.invalid,
        vec![
            "Mehr als 1 Antwort fehlt.",
            "Skala X unvollständig.",
            "Beide Skalen unvollständig."
        ]
    );
    assert_eq!(g.verdict, None);
}

#[test]
fn without_validity_nothing_changes() {
    let (g, text) = grade(YAML, &[("2", 1), ("4", 1)], &[]);
    assert!(g.invalid.is_empty());
    assert_eq!(
        g.verdict.as_deref(),
        Some("Nicht auswertbar: zu viele fehlende Antworten")
    );
    assert!(!text.contains("nicht interpretierbar"), "{text}");
}

#[test]
fn invalid_results_have_no_norm_values() {
    let yaml = NORMS.replacen("min_app: \"0.5.0\"", "min_app: \"0.7.0\"", 1).replacen(
        "verdict:\n",
        "validity:\n  - { require: A.missing == 0, invalid: \"Skala A unvollständig.\" }\nverdict:\n",
        1,
    );
    let all: Vec<_> = ["1", "2", "3", "4", "5", "6"].map(|k| (k, 1)).to_vec();
    let norms = |g: &Grading| {
        g.rows
            .iter()
            .filter(|r| matches!(r, ResultRow::Sum { norm: Some(_), .. }))
            .count()
    };
    let (g, _) = grade(&yaml, &all, &[]);
    assert_eq!(norms(&g), 3);
    let (g, _) = grade(&yaml, &all[1..], &[]);
    assert_eq!(g.invalid, vec!["Skala A unvollständig."]);
    assert_eq!(norms(&g), 0);
}

#[test]
fn validity_needs_min_app_0_7() {
    let need = r#"Gültigkeitskriterien (validity) brauchen min_app: "0.7.0""#.to_string();
    let yaml = with_validity();
    assert_eq!(needs_newer_app(&yaml, "0.6.0").as_deref(), Some("0.7.0"));
    assert_eq!(needs_newer_app(&yaml, "0.7.0"), None);
    let y = yaml.replacen("min_app: \"0.7.0\"", "min_app: \"0.6.0\"", 1);
    assert_eq!(errors(&y), vec![need.clone()]);
    // Without min_app the count(…) gate fires as well.
    let y = yaml.replacen("min_app: \"0.7.0\"\n", "", 1);
    assert!(errors(&y).contains(&need));
}

#[test]
fn validity_errors_are_named() {
    let y = with_validity()
        .replacen("require: vollstaendig,", "require: X,", 1)
        .replacen("invalid: \"Skala X unvollständig.\"", "invalid: \" \"", 1)
        .replacen("require: 'count(", "require: 'nope or count(", 1);
    assert_eq!(
        errors(&y),
        vec![
            "Gültigkeitskriterium 1: Bedingung muss wahr/falsch ergeben",
            "Gültigkeitskriterium 2: Text bei „invalid“ fehlt",
            "Gültigkeitskriterium 3: „nope“ hat keinen Wert",
        ]
    );
    // `when` belongs to caveats; a criterion states what must hold.
    let y = with_validity().replacen("require: vollstaendig", "when: vollstaendig", 1);
    assert!(load(&y).is_err());
}

#[test]
fn round_trips_and_omits_empty_validity() {
    let def = parse_definition(&with_validity()).unwrap();
    assert_eq!(def.validity.len(), 3);
    assert_eq!(load(&to_yaml(&def)).unwrap().def, def);
    assert!(!to_yaml(&parse_definition(YAML).unwrap()).contains("validity"));
}

#[test]
fn saved_gradings_without_invalid_still_load() {
    let (g, _) = grade(YAML, &ALL, &[]);
    let mut json = serde_json::to_value(&g).unwrap();
    json.as_object_mut().unwrap().remove("invalid");
    let back: Grading = serde_json::from_value(json).unwrap();
    assert_eq!(back, g);
}

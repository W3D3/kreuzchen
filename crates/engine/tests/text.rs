use kreuzchen_engine::schema::FieldValue;
use kreuzchen_engine::{load, Input};

const YAML: &str = include_str!("../../../examples/definitionen/beispiel.yaml");

fn text(answers: &[(&str, usize)], fields: &[(&str, FieldValue)]) -> String {
    let q = load(YAML).unwrap();
    let input = Input {
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        answers: answers.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    };
    let g = q.grade(&input).unwrap();
    q.plain_text(&input, &g)
}

#[test]
fn exactly_at_cutoff_text() {
    let t = text(
        &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)],
        &[
            (
                "kennung",
                FieldValue::Text("Anonym 11.11.2026 22:23".into()),
            ),
            ("alter", FieldValue::Num(9.0)),
        ],
    );
    let expected = "Beispiel-Fragebogen (Version 1)
Kennung: Anonym 11.11.2026 22:23
Alter: 9
Volljährig: Nein

Gesamtsumme: 7 (Cut-off 7) – über Cut-off
Skala X: 4 (Cut-off 4) – über Cut-off
Skala Y: 3 (Cut-off 3) – über Cut-off
Zusatzfrage: Nein

Regel 1: Gesamtsumme über Cut-off: erfüllt
Regel 2: Gesamtsumme und Skala X über Cut-off: erfüllt
Höchstens 1 Antwort fehlt: erfüllt
Fehlende Antworten: 0
Ergebnis: Auffällig (Regel 2)";
    assert_eq!(t, expected);
}

#[test]
fn missing_text() {
    let t = text(&[("1", 3)], &[]);
    assert!(t.contains("Fehlende Antworten: 3 (Fragen 2, 3, 4)"), "{t}");
    assert!(t.contains("Zusatzfrage: –"), "{t}");
    assert!(t.contains("Alter: –"), "{t}");
}

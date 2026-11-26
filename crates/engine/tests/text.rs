use kreuzchen_engine::schema::FieldValue;
use kreuzchen_engine::{load, Input};

const YAML: &str = include_str!("../../../examples/definitions/beispiel.yaml");

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
    q.plain_text(&g)
}

#[test]
fn exactly_at_cutoff_text() {
    let t = text(
        &[("1", 3), ("2", 2), ("3", 0), ("4", 3), ("zusatz", 0)],
        &[
            (
                "subject_id",
                FieldValue::Text("Anonym 11.11.2026 22:23".into()),
            ),
            ("alter", FieldValue::Num(9.0)),
        ],
    );
    let expected = "Beispiel-Fragebogen (Version 1)
Kennung: Anonym 11.11.2026 22:23
Alter: 9
Volljährig: Nein

Ergebnis: Auffällig (Regel 2)
Regel 1: Gesamtsumme über Cut-off: erfüllt
Regel 2: Gesamtsumme und Skala X über Cut-off: erfüllt
Höchstens 1 Antwort fehlt: erfüllt
Fehlende Antworten: 0

Gesamtsumme: 7 (Cut-off 7) – über Cut-off
Skala X: 4 (Cut-off 4) – über Cut-off
Skala Y: 3 (Cut-off 3) – über Cut-off
Zusatzfrage: Nein";
    assert_eq!(t, expected);
}

#[test]
fn missing_text() {
    let t = text(&[("1", 3)], &[]);
    assert!(t.contains("Fehlende Antworten: 3 (Fragen 2, 3, 4)"), "{t}");
    assert!(t.contains("Zusatzfrage: –"), "{t}");
    assert!(t.contains("Alter: –"), "{t}");
}

#[test]
fn number_fields_use_a_decimal_comma() {
    let t = text(&[], &[("alter", FieldValue::Num(7.5))]);
    assert!(t.contains("Alter: 7,5\n"), "{t}");
}

#[test]
fn tiny_negative_numbers_print_as_zero() {
    assert_eq!(kreuzchen_engine::number(-0.001), "0");
    assert_eq!(kreuzchen_engine::number(-0.5), "-0,5");
}

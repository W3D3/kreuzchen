use kreuzchen_engine::load;

const FIXTURE: &str = include_str!("../../../examples/definitions/beispiel.yaml");

fn errs_multi(reps: &[(&str, &str)]) -> Vec<String> {
    let mut y = FIXTURE.to_string();
    for (f, t) in reps {
        assert!(y.contains(f), "fixture lacks {f}");
        y = y.replacen(f, t, 1);
    }
    match load(&y) {
        Ok(_) => panic!("expected Err"),
        Err(e) => e.into_iter().map(|e| e.message).collect(),
    }
}

fn errs(from: &str, to: &str) -> Vec<String> {
    errs_multi(&[(from, to)])
}

#[test]
fn beispiel_is_valid() {
    let q = load(FIXTURE).unwrap();
    assert_eq!(q.item_count(), 4);
    assert_eq!(q.scale_count(), 2);
}

#[test]
fn unknown_scale() {
    assert!(errs("scale: Y, key: normal", "scale: Z, key: normal")
        .contains(&"Item 4: Skala „Z“ ist nicht definiert".to_string()));
}

#[test]
fn key_length() {
    assert!(errs("[0, 0, 1, 2]", "[0, 1]").contains(
        &"Item 3: Punkteschlüssel hat 2 Werte, das Antwortset hat 4 Antworten".to_string()
    ));
}

#[test]
fn duplicate_item() {
    assert!(errs("n: 4", "n: 3").contains(&"Item 3 ist doppelt".to_string()));
}

#[test]
fn item_without_key() {
    let e = errs("n: 4, scale", "scale");
    assert!(e.contains(&"Item ohne Nummer oder ID in Abschnitt „Teil 2“".to_string()));
    assert!(!e.iter().any(|m| m.contains("doppelt")));
}

#[test]
fn duplicate_name() {
    assert!(errs("id: gesamt,", "id: X,").contains(&"Name „X“ ist doppelt vergeben".to_string()));
}

#[test]
fn bad_schema() {
    assert!(errs("schema: 1", "schema: 2").contains(&"Unbekannte Schema-Version 2".to_string()));
}

#[test]
fn bad_expression() {
    let e = errs("when: r1 and X.over", "when: r1 and and");
    assert_eq!(e.len(), 1);
    assert!(e[0].starts_with("Regel „r2“: "));
}

#[test]
fn unknown_name() {
    let e = errs("when: gesamt.over", "when: gesamtt.over");
    assert_eq!(e.len(), 1);
    assert!(e[0].contains("„gesamtt.over“ hat keinen Wert"));
}

#[test]
fn rule_sees_only_earlier_rules() {
    assert!(!errs("when: r1 and X.over", "when: r2 and X.over").is_empty());
}

#[test]
fn wrong_type() {
    assert!(errs("value: X + Y", "value: X > Y")
        .contains(&"Abgeleiteter Wert „gesamt“: Ergebnis muss eine Zahl sein".to_string()));
}

#[test]
fn wrong_condition_type() {
    assert!(errs("when: erwachsen,", "when: 1 + 1,")
        .contains(&"Hinweis 1: Bedingung muss wahr/falsch ergeben".to_string()));
}

#[test]
fn unknown_answer_set() {
    assert!(errs("answers: ja_nein", "answers: jn")
        .contains(&"Item zusatz: Antwortset „jn“ ist nicht definiert".to_string()));
}

#[test]
fn bad_compare() {
    assert!(!errs("cutoff_compare: \">=\"", "cutoff_compare: \"=>\"").is_empty());
}

#[test]
fn unknown_result() {
    assert!(
        errs("result: [gesamt, X, Y, zusatz]", "result: [gesamt, Q]")
            .contains(&"Ergebnis: „Q“ ist nicht definiert".to_string())
    );
}

#[test]
fn scale_without_key_and_back() {
    assert!(errs("scale: Y, key: normal", "scale: Y")
        .contains(&"Item 4: Skala ohne Punkteschlüssel".to_string()));
    assert!(errs("scale: Y, key: normal", "key: normal")
        .contains(&"Item 4: Punkteschlüssel ohne Skala".to_string()));
}

#[test]
fn collects_all() {
    assert!(
        errs_multi(&[
            ("scale: Y, key: normal", "scale: Z, key: normal"),
            ("[0, 0, 1, 2]", "[0, 1]")
        ])
        .len()
            >= 2
    );
}

#[test]
fn default_type_mismatch() {
    let e = errs("type: checkbox }", "type: checkbox, default: 1 }");
    assert!(e.contains(&"Feld „erwachsen“: Standardwert passt nicht zum Typ".to_string()));
}

#[test]
fn rejects_bad_names() {
    let msg = |n: &str| format!("Name „{n}“ ist nicht erlaubt");
    for (from, to, bad) in [
        ("id: zusatz,", "id: \"zu satz\",", "zu satz"),
        ("id: erwachsen,", "id: and,", "and"),
        ("id: subject_id,", "id: missing,", "missing"),
        ("id: alter,", "id: A.over,", "A.over"),
    ] {
        assert!(errs(from, to).contains(&msg(bad)), "{bad}");
    }
}

#[test]
fn shortcuts_are_valid() {
    let y = FIXTURE.replacen(
        "keys:",
        "shortcuts:\n  haeufigkeit: \"1234\"\n  ja_nein: jn\nkeys:",
        1,
    );
    assert!(load(&y).is_ok());
}

#[test]
fn shortcuts_errors() {
    let e = |s: &str| errs("keys:", &format!("shortcuts:\n  {s}\nkeys:"));
    assert!(e("haeufigkeit: \"123\"").contains(
        &"Tasten für „haeufigkeit“: 3 Tasten, das Antwortset hat 4 Antworten".to_string()
    ));
    assert!(e("haeufigkeit: \"1231\"")
        .contains(&"Tasten für „haeufigkeit“: Taste „1“ ist doppelt".to_string()));
    assert!(e("ja_nein: \"j \"")
        .contains(&"Tasten für „ja_nein“: Leerzeichen sind nicht erlaubt".to_string()));
    assert!(e("gibtsnicht: \"12\"")
        .contains(&"Tasten für „gibtsnicht“: Antwortset ist nicht definiert".to_string()));
}

#[test]
fn count_needs_min_app_0_6() {
    let need = r#"count(…) und Felder, Fragen oder „missing“ in abgeleiteten Werten brauchen min_app: "0.6.0""#;
    for value in [r#"count(X.over, zusatz == "Ja")"#, "X + missing"] {
        let e = errs("value: X + Y", &format!("value: '{value}'"));
        assert_eq!(e, vec![need.to_string()], "{value}");
    }
    assert_eq!(
        errs("when: missing <= 1", "when: 'count(r1, r2) >= 1'"),
        vec![need.to_string()]
    );
    let ok = FIXTURE
        .replacen("min_app: \"0.5.0\"", "min_app: \"0.6.0\"", 1)
        .replacen(
            "value: X + Y",
            r#"value: 'count(X.over, zusatz == "Ja")'"#,
            1,
        );
    assert!(load(&ok).is_ok());
}

#[test]
fn count_errors_name_the_expression() {
    let e = errs_multi(&[
        ("min_app: \"0.5.0\"", "min_app: \"0.6.0\""),
        ("value: X + Y", "value: 'count(X.over, X)'"),
    ]);
    assert_eq!(
        e,
        vec!["Abgeleiteter Wert „gesamt“: count(): Bedingung 2 muss wahr/falsch ergeben, Zahl gefunden"]
    );
    let e = errs_multi(&[
        ("min_app: \"0.5.0\"", "min_app: \"0.6.0\""),
        ("value: X + Y", "value: count()"),
    ]);
    assert_eq!(
        e,
        vec!["Abgeleiteter Wert „gesamt“: count() braucht mindestens eine Bedingung"]
    );
}

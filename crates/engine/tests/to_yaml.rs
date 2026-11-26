use kreuzchen_engine::{load, needs_newer_app, parse_definition, raise_min_app, to_yaml};

const YAML: &str = include_str!("../../../examples/definitions/beispiel.yaml");

#[test]
fn beispiel_round_trips() {
    let def = parse_definition(YAML).unwrap();
    let out = to_yaml(&def);
    assert_eq!(load(&out).unwrap().def, def);
}

#[test]
fn omits_unset_optionals() {
    let out = to_yaml(&parse_definition(YAML).unwrap());
    assert!(!out.contains("null"), "{out}");
    assert!(!out.contains("authors"), "{out}");
}

#[test]
fn shortcuts_round_trip() {
    let y = YAML.replacen("keys:", "shortcuts:\n  ja_nein: nj\nkeys:", 1);
    let def = parse_definition(&y).unwrap();
    assert_eq!(def.shortcuts["ja_nein"], "nj");
    assert_eq!(load(&to_yaml(&def)).unwrap().def, def);
}

#[test]
fn raise_min_app_follows_the_expressions_and_never_lowers() {
    let raised = |yaml: &str| {
        let mut def = parse_definition(yaml).unwrap();
        raise_min_app(&mut def);
        def.min_app
    };
    let base = YAML.replacen("min_app: \"0.5.0\"\n", "", 1);
    assert_eq!(raised(&base), None);
    assert_eq!(raised(YAML).as_deref(), Some("0.5.0"));
    for (from, to) in [
        ("value: X + Y", r#"value: 'count(X.over, zusatz == "Ja")'"#),
        ("value: X + Y", "value: X + missing"),
        ("when: missing <= 1", "when: 'count(r1, r2) >= 1'"),
    ] {
        let uses = base.replacen(from, to, 1);
        assert_eq!(raised(&uses).as_deref(), Some("0.6.0"), "{to}");
        let mut def = parse_definition(&uses).unwrap();
        raise_min_app(&mut def);
        let out = to_yaml(&def);
        assert!(load(&out).is_ok(), "{to}");
        assert_eq!(needs_newer_app(&out, "0.5.0").as_deref(), Some("0.6.0"));
        let newer = YAML.replacen("min_app: \"0.5.0\"", "min_app: \"0.7.1\"", 1);
        assert_eq!(
            raised(&newer.replacen(from, to, 1)).as_deref(),
            Some("0.7.1")
        );
    }
}

#[test]
fn form_save_raises_min_app_for_validity() {
    let y = YAML.replacen(
        "verdict:\n",
        "validity:\n  - { require: missing <= 1, invalid: \"Zu viele fehlende Antworten.\" }\nverdict:\n",
        1,
    );
    let mut def = parse_definition(&y).unwrap();
    assert!(load(&y).is_err(), "min_app 0.5.0 is too old for validity");
    raise_min_app(&mut def);
    assert_eq!(def.min_app.as_deref(), Some("0.7.0"));
    let out = to_yaml(&def);
    assert!(load(&out).is_ok());
    assert_eq!(needs_newer_app(&out, "0.6.0").as_deref(), Some("0.7.0"));
}

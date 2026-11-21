use kreuzchen_engine::{load, parse_definition, to_yaml};

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

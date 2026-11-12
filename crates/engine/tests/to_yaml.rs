use kreuzchen_engine::{load, parse_definition, to_yaml};

const YAML: &str = include_str!("../../../examples/definitionen/beispiel.yaml");

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

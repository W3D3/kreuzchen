use kreuzchen_engine::schema::*;
use kreuzchen_engine::parse_definition;

const YAML: &str = include_str!("../../../examples/definitionen/beispiel.yaml");

#[test]
fn parses_beispiel() {
    let d = parse_definition(YAML).unwrap();
    assert_eq!(d.id, "beispiel");
    assert_eq!(d.sections.len(), 2);
    assert!(matches!(d.cutoff_compare, Compare::Ge));
    assert!(matches!(d.sections[0].items[2].key, Some(KeyRef::Inline(ref v)) if v == &vec![0, 0, 1, 2]));
    assert_eq!(d.sections[1].items[1].key_name(), "zusatz");
    assert_eq!(d.sections[0].items[0].key_name(), "1");
    assert!(matches!(d.verdict[3], VerdictEntry::Else { .. }));
}

#[test]
fn parses_with_bom_and_crlf() {
    let y = format!("\u{feff}{}", YAML.replace('\n', "\r\n"));
    assert_eq!(parse_definition(&y).unwrap().id, "beispiel");
}

#[test]
fn syntax_error_has_line() {
    let e = parse_definition("schema: 1\nid: [\n").unwrap_err();
    assert!(e.line.is_some());
}

#[test]
fn unknown_field_rejected() {
    assert!(parse_definition(&YAML.replace("title:", "titel:")).is_err());
}

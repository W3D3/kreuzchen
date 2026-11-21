use kreuzchen_engine::parse_definition;
use kreuzchen_engine::schema::*;

const YAML: &str = include_str!("../../../examples/definitions/beispiel.yaml");

#[test]
fn parses_beispiel() {
    let d = parse_definition(YAML).unwrap();
    assert_eq!(d.id, "beispiel");
    assert_eq!(d.sections.len(), 2);
    assert!(matches!(d.cutoff_compare, Compare::Ge));
    assert!(
        matches!(d.sections[0].items[2].key, Some(KeyRef::Inline(ref v)) if v == &vec![0, 0, 1, 2])
    );
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
    let e = parse_definition(&format!("{YAML}\nbogus: 1\n")).unwrap_err();
    assert!(e.message.contains("bogus"), "{e}");
    let nested = YAML.replacen("{ n: 1,", "{ bogus: 1, n: 1,", 1);
    assert!(parse_definition(&nested).is_err());
}

#[test]
fn cutoff_compare_defaults_to_ge() {
    let y = include_str!("../../../examples/definitions/beispiel.yaml").replacen(
        "cutoff_compare: \">=\"\n",
        "",
        1,
    );
    let q = kreuzchen_engine::load(&y).unwrap();
    assert_eq!(q.def.cutoff_compare, kreuzchen_engine::schema::Compare::Ge);
}

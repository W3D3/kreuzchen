pub mod expr;
mod grade;
pub mod schema;
mod text;
mod validate;

pub use grade::{FieldLine, GradeError, Grading, Input, ResultRow, RuleOutcome};
pub use validate::Questionnaire;

use schema::Definition;
use std::fmt;

#[derive(Debug, Clone)]
pub struct LoadError {
    pub message: String,
    pub line: Option<usize>,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "Zeile {}: {}", l, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for LoadError {}

pub fn parse_definition(yaml: &str) -> Result<Definition, LoadError> {
    let yaml = yaml.strip_prefix('\u{feff}').unwrap_or(yaml);
    serde_norway::from_str(yaml).map_err(|e| LoadError {
        line: e.location().map(|l| l.line()),
        message: e.to_string(),
    })
}

/// Serializes a definition back to YAML, leaving out unset optional keys.
pub fn to_yaml(def: &Definition) -> String {
    fn strip_nulls(v: &mut serde_norway::Value) {
        match v {
            serde_norway::Value::Mapping(m) => {
                m.retain(|_, v| !v.is_null());
                m.values_mut().for_each(strip_nulls);
            }
            serde_norway::Value::Sequence(s) => s.iter_mut().for_each(strip_nulls),
            _ => {}
        }
    }
    // Every field is plain data, so serializing cannot fail.
    let mut v = serde_norway::to_value(def).expect("definition serializes");
    strip_nulls(&mut v);
    serde_norway::to_string(&v).expect("value serializes")
}

/// Parses and validates a definition, collecting every error.
pub fn load(yaml: &str) -> Result<Questionnaire, Vec<LoadError>> {
    validate::validate(parse_definition(yaml).map_err(|e| vec![e])?)
}

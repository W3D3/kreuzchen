pub mod expr;
mod grade;
pub mod schema;
mod text;
mod validate;

pub use grade::{FieldLine, GradeError, Grading, Input, NormValue, ResultRow, RuleOutcome};
pub use text::number;
pub use validate::Questionnaire;

use schema::Definition;
use sha2::{Digest, Sha256};
use std::fmt;

/// Lowercase hex SHA-256 of a definition file; app and Bibliothek compare these to spot updates.
pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

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

/// `"0.4.0"` → `[0, 4, 0]`; anything else is `None`.
fn app_version(v: &str) -> Option<Vec<u64>> {
    v.split('.').map(|p| p.parse().ok()).collect()
}

/// The `min_app` a file asks for when it is newer than `app`, read before the full load,
/// so a file using features this version lacks still says which version it needs.
pub fn needs_newer_app(yaml: &str, app: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Peek {
        min_app: Option<String>,
    }
    let min = serde_norway::from_str::<Peek>(yaml).ok()?.min_app?;
    (app_version(&min)? > app_version(app)?).then_some(min)
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

#[cfg(test)]
mod tests {
    #[test]
    fn hash_is_lowercase_hex_sha256() {
        assert_eq!(
            super::hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}

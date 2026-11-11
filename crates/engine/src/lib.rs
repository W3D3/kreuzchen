pub mod expr;
pub mod schema;
mod validate;

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

/// Parses and validates a definition, collecting every error.
pub fn load(yaml: &str) -> Result<Questionnaire, Vec<LoadError>> {
    validate::validate(parse_definition(yaml).map_err(|e| vec![e])?)
}

pub mod expr;
pub mod schema;

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

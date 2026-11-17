use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub schema: u32,
    pub id: String,
    pub version: String,
    pub title: String,
    #[serde(default)]
    pub authors: Option<String>,
    #[serde(default)]
    pub copyright: Option<String>,
    /// Oldest Kreuzchen version that can open this file, e.g. "0.4.0".
    #[serde(default)]
    pub min_app: Option<String>,
    #[serde(default)]
    pub cutoff_compare: Compare,
    #[serde(default)]
    pub answer_sets: BTreeMap<String, Vec<String>>,
    /// Entry keys per answer set, one character per answer. Unset sets use 0, 1, 2, ….
    #[serde(default)]
    pub shortcuts: BTreeMap<String, String>,
    #[serde(default)]
    pub keys: BTreeMap<String, Vec<i64>>,
    #[serde(default)]
    pub default_answers: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(default)]
    pub scales: Vec<Scale>,
    #[serde(default)]
    pub derived: Vec<Derived>,
    #[serde(default)]
    pub sections: Vec<Section>,
    #[serde(default)]
    pub result: Vec<String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub caveats: Vec<Caveat>,
    #[serde(default)]
    pub verdict: Vec<VerdictEntry>,
    #[serde(default)]
    pub norms: Option<Norms>,
}

/// Norm tables (e.g. T-values) for scales and derived values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Norms {
    /// Shown before the value, e.g. "T".
    #[serde(default = "default_norm_name")]
    pub name: String,
    /// Severity labels by lower bound, ascending; a norm value gets the last band it reaches.
    #[serde(default)]
    pub bands: Vec<Band>,
    /// Scale or derived id to raw score → norm value. A raw score that is not listed takes the
    /// next lower listed one, as with a printed table that skips rows.
    pub tables: BTreeMap<String, BTreeMap<i64, i64>>,
}

fn default_norm_name() -> String {
    "T".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub from: i64,
    pub label: String,
}

impl Norms {
    /// Norm value and band for `raw`, or `None` when `id` has no table or `raw` is below it.
    pub fn lookup(&self, id: &str, raw: f64) -> Option<(i64, Option<String>)> {
        let (_, &value) = self
            .tables
            .get(id)?
            .range(..=raw.floor() as i64)
            .next_back()?;
        let band = self.bands.iter().rev().find(|b| value >= b.from);
        Some((value, band.map(|b| b.label.clone())))
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum Compare {
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    #[default]
    Ge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: FieldKind,
    #[serde(default)]
    pub default: Option<FieldValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Number,
    Date,
    Checkbox,
}

impl FieldKind {
    /// Whether `v` is the value type this kind of field takes.
    pub fn accepts(&self, v: &FieldValue) -> bool {
        matches!(
            (self, v),
            (FieldKind::Checkbox, FieldValue::Bool(_))
                | (FieldKind::Number, FieldValue::Num(_))
                | (FieldKind::Text | FieldKind::Date, FieldValue::Text(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    Bool(bool),
    Num(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub cutoff: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derived {
    pub id: String,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub cutoff: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub title: String,
    #[serde(default)]
    pub intro: Option<String>,
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    #[serde(default)]
    pub n: Option<u32>,
    #[serde(default)]
    pub id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub answers: Option<String>,
    #[serde(default)]
    pub scale: Option<String>,
    #[serde(default)]
    pub key: Option<KeyRef>,
}

impl Item {
    /// `id`, else `n` as a string (empty if neither is set).
    pub fn key_name(&self) -> String {
        match (&self.id, self.n) {
            (Some(id), _) => id.clone(),
            (None, Some(n)) => n.to_string(),
            (None, None) => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyRef {
    Named(String),
    Inline(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub label: String,
    pub when: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caveat {
    pub when: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerdictEntry {
    When {
        when: String,
        text: String,
    },
    Else {
        #[serde(rename = "else")]
        otherwise: String,
    },
}

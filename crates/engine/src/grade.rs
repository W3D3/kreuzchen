use crate::expr::{self, Value};
use crate::schema::*;
use crate::validate::{build_env, EnvSource};
use crate::Questionnaire;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Input {
    pub fields: BTreeMap<String, FieldValue>,
    /// Item key name to answer index.
    pub answers: BTreeMap<String, usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Grading {
    /// Header lines in definition order, formatted for display and copy text.
    #[serde(default)]
    pub fields: Vec<FieldLine>,
    pub rows: Vec<ResultRow>,
    pub rules: Vec<RuleOutcome>,
    pub caveats: Vec<String>,
    pub verdict: Option<String>,
    pub missing: u32,
    pub missing_items: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FieldLine {
    pub id: String,
    pub label: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ResultRow {
    Sum {
        id: String,
        label: String,
        value: f64,
        cutoff: Option<i64>,
        over: bool,
        /// Absent in results saved before norm tables existed.
        #[serde(default)]
        norm: Option<NormValue>,
    },
    Answer {
        id: String,
        label: String,
        answer: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NormValue {
    pub name: String,
    pub value: i64,
    pub band: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RuleOutcome {
    pub id: String,
    pub label: String,
    pub met: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GradeError {
    UnknownItem(String),
    AnswerOutOfRange { item: String, index: usize },
    UnknownField(String),
    WrongFieldType(String),
    Eval(String),
}

impl fmt::Display for GradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownItem(i) => write!(f, "Antwort für unbekannte Frage {i}"),
            Self::AnswerOutOfRange { item, index } => {
                write!(f, "Frage {item}: Antwort {} gibt es nicht", index + 1)
            }
            Self::UnknownField(id) => write!(f, "Unbekanntes Feld „{id}“"),
            Self::WrongFieldType(id) => write!(f, "Feld „{id}“ hat den falschen Typ"),
            Self::Eval(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for GradeError {}

fn eval(e: &expr::Expr, env: &BTreeMap<String, Value>) -> Result<Value, GradeError> {
    expr::eval_short(e, env).map_err(GradeError::Eval)
}

fn truth(v: Value) -> Result<bool, GradeError> {
    expr::want_bool(v).map_err(GradeError::Eval)
}

impl Questionnaire {
    fn items(&self) -> impl Iterator<Item = &Item> {
        self.def.sections.iter().flat_map(|s| &s.items)
    }

    fn answer_set(&self, item: &Item) -> &[String] {
        item.answers
            .as_ref()
            .or(self.def.default_answers.as_ref())
            .and_then(|n| self.def.answer_sets.get(n))
            .map_or(&[], |v| v.as_slice())
    }

    fn derived_expr(&self, id: &str) -> &expr::Expr {
        let e = self.derived.iter().find(|(d, _)| d.id == id);
        &e.expect("validated: every derived value compiled").1
    }

    fn rule_expr(&self, id: &str) -> &expr::Expr {
        let e = self.rules.iter().find(|(r, _)| r.id == id);
        &e.expect("validated: every rule compiled").1
    }

    fn over(&self, value: f64, cutoff: Option<i64>) -> bool {
        cutoff.is_some_and(|c| match self.def.cutoff_compare {
            Compare::Gt => value > c as f64,
            Compare::Ge => value >= c as f64,
        })
    }

    pub fn grade(&self, input: &Input) -> Result<Grading, GradeError> {
        let def = &self.def;
        // Unknown keys, then out-of-range indexes, then unknown fields.
        for k in input.answers.keys() {
            if !self.items().any(|i| &i.key_name() == k) {
                return Err(GradeError::UnknownItem(k.clone()));
            }
        }
        for item in self.items() {
            let key = item.key_name();
            if let Some(&index) = input.answers.get(&key) {
                if index >= self.answer_set(item).len() {
                    return Err(GradeError::AnswerOutOfRange { item: key, index });
                }
            }
        }
        for k in input.fields.keys() {
            if !def.fields.iter().any(|f| &f.id == k) {
                return Err(GradeError::UnknownField(k.clone()));
            }
        }
        for f in &def.fields {
            let ok = input.fields.get(&f.id).is_none_or(|v| f.kind.accepts(v));
            if !ok {
                return Err(GradeError::WrongFieldType(f.id.clone()));
            }
        }

        // Scale sums and missing answers.
        let mut sums: BTreeMap<&str, i64> = def.scales.iter().map(|s| (s.id.as_str(), 0)).collect();
        let mut missing_items = Vec::new();
        let mut scale_missing: BTreeMap<&str, f64> = BTreeMap::new();
        for item in self.items() {
            let Some(scale) = &item.scale else { continue };
            match input.answers.get(&item.key_name()) {
                None => {
                    missing_items.push(item.key_name());
                    *scale_missing.entry(scale).or_default() += 1.0;
                }
                Some(&idx) => {
                    let key = match &item.key {
                        Some(KeyRef::Inline(v)) => Some(v),
                        Some(KeyRef::Named(n)) => def.keys.get(n),
                        None => None,
                    };
                    // Validation guarantees the key length matches the answer set.
                    let key = key.expect("validated: scale items have a key");
                    *sums.entry(scale).or_default() += key[idx];
                }
            }
        }

        let scale_vals: BTreeMap<&str, (f64, bool)> = def
            .scales
            .iter()
            .map(|s| {
                let v = sums[s.id.as_str()] as f64;
                (s.id.as_str(), (v, self.over(v, s.cutoff)))
            })
            .collect();
        let mut derived_vals: BTreeMap<String, (f64, bool)> = BTreeMap::new();
        let missing = missing_items.len() as u32;
        let mut rules = Vec::new();
        let env = build_env(
            def,
            EnvSource {
                scale: &|s| scale_vals[s.id.as_str()],
                scale_missing: &|s| scale_missing.get(s.id.as_str()).copied().unwrap_or(0.0),
                field: &|f| match input.fields.get(&f.id).or(f.default.as_ref()) {
                    Some(FieldValue::Bool(b)) => Some(Value::Bool(*b)),
                    Some(FieldValue::Num(n)) => Some(Value::Num(*n)),
                    Some(FieldValue::Text(t)) => Some(Value::Str(t.clone())),
                    None => match f.kind {
                        FieldKind::Checkbox => Some(Value::Bool(false)),
                        FieldKind::Text | FieldKind::Date => Some(Value::Str(String::new())),
                        FieldKind::Number => None, // referencing it is an Eval error
                    },
                },
                item: &|i| {
                    input
                        .answers
                        .get(&i.key_name())
                        .map(|&a| self.answer_set(i)[a].clone())
                        .unwrap_or_default()
                },
                missing: missing as f64,
                derived: &mut |d, env| {
                    let n = match eval(self.derived_expr(&d.id), env)? {
                        Value::Num(n) if n.is_finite() => n,
                        Value::Num(_) => {
                            return Err(GradeError::Eval(format!(
                                "Abgeleiteter Wert „{}“: Division durch 0",
                                d.id
                            )))
                        }
                        _ => return Err(GradeError::Eval("Ergebnis muss eine Zahl sein".into())),
                    };
                    let r = (n, self.over(n, d.cutoff));
                    derived_vals.insert(d.id.clone(), r);
                    Ok(r)
                },
                rule: &mut |r, env| {
                    let met = truth(eval(self.rule_expr(&r.id), env)?)?;
                    rules.push(RuleOutcome {
                        id: r.id.clone(),
                        label: r.label.clone(),
                        met,
                    });
                    Ok(met)
                },
            },
        )?;
        let answers: BTreeMap<&str, (&str, Option<String>)> = self
            .items()
            .filter_map(|i| {
                let id = i.id.as_ref()?;
                let a = input
                    .answers
                    .get(id)
                    .map(|&a| self.answer_set(i)[a].clone());
                Some((id.as_str(), (i.text.as_str(), a)))
            })
            .collect();

        let mut caveats = Vec::new();
        for (c, e) in &self.caveats {
            if truth(eval(e, &env)?)? {
                caveats.push(c.text.clone());
            }
        }
        let mut verdict = None;
        for (when, text) in &self.verdict {
            if when.as_ref().map_or(Ok(true), |e| truth(eval(e, &env)?))? {
                verdict = Some(text.clone());
                break;
            }
        }

        let rows = def
            .result
            .iter()
            .map(|id| {
                let sum = |label: &String, cutoff, (value, over): (f64, bool)| ResultRow::Sum {
                    id: id.clone(),
                    label: label.clone(),
                    value,
                    cutoff,
                    over,
                    norm: def.norms.as_ref().and_then(|n| {
                        let (value, band) = n.lookup(id, value)?;
                        Some(NormValue {
                            name: n.name.clone(),
                            value,
                            band,
                        })
                    }),
                };
                if let Some(s) = def.scales.iter().find(|s| &s.id == id) {
                    sum(&s.label, s.cutoff, scale_vals[id.as_str()])
                } else if let Some(d) = def.derived.iter().find(|d| &d.id == id) {
                    sum(&d.label, d.cutoff, derived_vals[id.as_str()])
                } else {
                    let (text, answer) = &answers[id.as_str()];
                    ResultRow::Answer {
                        id: id.clone(),
                        label: text.to_string(),
                        answer: answer.clone(),
                    }
                }
            })
            .collect();

        let fields = def
            .fields
            .iter()
            .map(|f| {
                let value = match input.fields.get(&f.id).or(f.default.as_ref()) {
                    Some(FieldValue::Bool(b)) => (if *b { "Ja" } else { "Nein" }).to_string(),
                    Some(FieldValue::Num(n)) => crate::number(*n),
                    Some(FieldValue::Text(s)) if !s.is_empty() => s.clone(),
                    None if matches!(f.kind, FieldKind::Checkbox) => "Nein".into(),
                    _ => "–".into(),
                };
                FieldLine {
                    id: f.id.clone(),
                    label: f.label.clone(),
                    value,
                }
            })
            .collect();

        Ok(Grading {
            fields,
            rows,
            rules,
            caveats,
            verdict,
            missing,
            missing_items,
        })
    }
}

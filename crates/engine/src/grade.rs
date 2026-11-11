use crate::expr::{self, Value};
use crate::schema::*;
use crate::validate::add_scored;
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
    pub rows: Vec<ResultRow>,
    pub rules: Vec<RuleOutcome>,
    pub caveats: Vec<String>,
    pub verdict: Option<String>,
    pub missing: u32,
    pub missing_items: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ResultRow {
    Sum {
        id: String,
        label: String,
        value: i64,
        cutoff: Option<i64>,
        over: bool,
    },
    Answer {
        id: String,
        label: String,
        answer: Option<String>,
    },
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
            Self::Eval(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for GradeError {}

fn eval(e: &expr::Expr, env: &BTreeMap<String, Value>) -> Result<Value, GradeError> {
    expr::eval(e, env).map_err(GradeError::Eval)
}

fn truth(v: Value) -> bool {
    matches!(v, Value::Bool(true))
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

    fn over(&self, value: i64, cutoff: Option<i64>) -> bool {
        cutoff.is_some_and(|c| match self.def.cutoff_compare {
            Compare::Gt => value > c,
            Compare::Ge => value >= c,
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

        // Scale sums and missing answers.
        let mut sums: BTreeMap<&str, i64> = def.scales.iter().map(|s| (s.id.as_str(), 0)).collect();
        let mut missing_items = Vec::new();
        for item in self.items() {
            let Some(scale) = &item.scale else { continue };
            match input.answers.get(&item.key_name()) {
                None => missing_items.push(item.key_name()),
                Some(&idx) => {
                    let key = match &item.key {
                        Some(KeyRef::Inline(v)) => Some(v),
                        Some(KeyRef::Named(n)) => def.keys.get(n),
                        None => None,
                    };
                    // Validation guarantees the key length matches the answer set.
                    *sums.entry(scale).or_default() += key.map_or(0, |k| k[idx]);
                }
            }
        }

        let mut env = BTreeMap::new();
        let mut values: BTreeMap<&str, (i64, Option<i64>, bool)> = BTreeMap::new();
        for s in &def.scales {
            let v = sums[s.id.as_str()];
            let over = self.over(v, s.cutoff);
            add_scored(&mut env, &s.id, v, s.cutoff, over);
            values.insert(&s.id, (v, s.cutoff, over));
        }
        for (d, e) in &self.derived {
            let Value::Num(n) = eval(e, &env)? else {
                return Err(GradeError::Eval("Ergebnis muss eine Zahl sein".into()));
            };
            let v = n as i64;
            let over = self.over(v, d.cutoff);
            add_scored(&mut env, &d.id, v, d.cutoff, over);
            values.insert(&d.id, (v, d.cutoff, over));
        }
        for f in &def.fields {
            let v = match input.fields.get(&f.id).or(f.default.as_ref()) {
                Some(FieldValue::Bool(b)) => Some(Value::Bool(*b)),
                Some(FieldValue::Num(n)) => Some(Value::Num(*n)),
                Some(FieldValue::Text(t)) => Some(Value::Str(t.clone())),
                None => match f.kind {
                    FieldKind::Checkbox => Some(Value::Bool(false)),
                    FieldKind::Text | FieldKind::Date => Some(Value::Str(String::new())),
                    FieldKind::Number => None, // referencing it is an Eval error
                },
            };
            if let Some(v) = v {
                env.insert(f.id.clone(), v);
            }
        }
        let mut answers: BTreeMap<&str, (&str, Option<String>)> = BTreeMap::new();
        for item in self.items() {
            let Some(id) = &item.id else { continue };
            let a = input
                .answers
                .get(id)
                .map(|&i| self.answer_set(item)[i].clone());
            env.insert(id.clone(), Value::Str(a.clone().unwrap_or_default()));
            answers.insert(id, (&item.text, a));
        }
        let missing = missing_items.len() as u32;
        env.insert("fehlend".to_string(), Value::Num(missing as f64));

        let mut rules = Vec::new();
        for (r, e) in &self.rules {
            let met = truth(eval(e, &env)?);
            env.insert(r.id.clone(), Value::Bool(met));
            rules.push(RuleOutcome {
                id: r.id.clone(),
                label: r.label.clone(),
                met,
            });
        }
        let mut caveats = Vec::new();
        for (c, e) in &self.caveats {
            if truth(eval(e, &env)?) {
                caveats.push(c.text.clone());
            }
        }
        let mut verdict = None;
        for (when, text) in &self.verdict {
            if when
                .as_ref()
                .map_or(Ok(true), |e| eval(e, &env).map(truth))?
            {
                verdict = Some(text.clone());
                break;
            }
        }

        let label_of = |id: &str| {
            def.scales
                .iter()
                .find(|s| s.id == id)
                .map(|s| &s.label)
                .or_else(|| def.derived.iter().find(|d| d.id == id).map(|d| &d.label))
        };
        let rows = def
            .result
            .iter()
            .filter_map(|id| {
                if let Some(&(value, cutoff, over)) = values.get(id.as_str()) {
                    Some(ResultRow::Sum {
                        id: id.clone(),
                        label: label_of(id)?.clone(),
                        value,
                        cutoff,
                        over,
                    })
                } else {
                    let (text, answer) = answers.get(id.as_str())?;
                    Some(ResultRow::Answer {
                        id: id.clone(),
                        label: text.to_string(),
                        answer: answer.clone(),
                    })
                }
            })
            .collect();

        Ok(Grading {
            rows,
            rules,
            caveats,
            verdict,
            missing,
            missing_items,
        })
    }
}

use crate::expr::{self, Expr, Value};
use crate::schema::*;
use crate::LoadError;
use std::collections::{BTreeMap, BTreeSet};

/// A validated definition plus its compiled expressions, ready for grading.
pub struct Questionnaire {
    pub def: Definition,
    /// Derived values paired with their compiled `value`, in definition order.
    pub(crate) derived: Vec<(Derived, Expr)>,
    /// Rules paired with their compiled `when`, in definition order (later rules may use earlier ones).
    pub(crate) rules: Vec<(Rule, Expr)>,
    /// Caveats paired with their compiled `when`.
    pub(crate) caveats: Vec<(Caveat, Expr)>,
    /// Verdict lines in order: `Some(when)` or `None` for the `else` line, plus its text.
    pub(crate) verdict: Vec<(Option<Expr>, String)>,
}

impl Questionnaire {
    /// Number of items that feed a scale.
    pub fn item_count(&self) -> usize {
        self.def
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .filter(|i| i.scale.is_some())
            .count()
    }

    pub fn scale_count(&self) -> usize {
        self.def.scales.len()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Want {
    Num,
    Bool,
}

fn err(message: String) -> LoadError {
    LoadError {
        message,
        line: None,
    }
}

/// Parses `src`, type-checks it against `env`; reports at most one error.
fn compile(
    src: &str,
    env: &BTreeMap<String, Value>,
    want: Want,
    wher: &str,
    errors: &mut Vec<LoadError>,
) -> Option<Expr> {
    let res = expr::parse(src).and_then(|e| match (expr::eval(&e, env)?, want) {
        (Value::Num(_), Want::Num) | (Value::Bool(_), Want::Bool) => Ok(e),
        (_, Want::Num) => Err("Ergebnis muss eine Zahl sein".to_string()),
        (_, Want::Bool) => Err("Bedingung muss wahr/falsch ergeben".to_string()),
    });
    match res {
        Ok(e) => Some(e),
        Err(m) => {
            errors.push(err(format!("{wher}: {m}")));
            None
        }
    }
}

/// Exposes a scale or derived value as `id`, plus `id.cutoff` and `id.over` when it has a cutoff.
pub(crate) fn add_scored(
    env: &mut BTreeMap<String, Value>,
    id: &str,
    value: i64,
    cutoff: Option<i64>,
    over: bool,
) {
    env.insert(id.to_string(), Value::Num(value as f64));
    if let Some(c) = cutoff {
        env.insert(format!("{id}.cutoff"), Value::Num(c as f64));
        env.insert(format!("{id}.over"), Value::Bool(over));
    }
}

fn add_cutoff(env: &mut BTreeMap<String, Value>, id: &str, cutoff: Option<i64>) {
    add_scored(env, id, 0, cutoff.map(|_| 0), false);
}

pub(crate) fn validate(def: Definition) -> Result<Questionnaire, Vec<LoadError>> {
    let mut errors = Vec::new();
    if def.schema != 1 {
        errors.push(err(format!("Unbekannte Schema-Version {}", def.schema)));
    }

    // Names: scales, derived, fields, item ids, rules.
    let items: Vec<(&Section, &Item)> = def
        .sections
        .iter()
        .flat_map(|s| s.items.iter().map(move |i| (s, i)))
        .collect();
    let mut seen = BTreeSet::new();
    let names = def
        .scales
        .iter()
        .map(|s| &s.id)
        .chain(def.derived.iter().map(|d| &d.id))
        .chain(def.fields.iter().map(|f| &f.id))
        .chain(items.iter().filter_map(|(_, i)| i.id.as_ref()))
        .chain(def.rules.iter().map(|r| &r.id));
    for n in names {
        if !seen.insert(n) {
            errors.push(err(format!("Name „{n}“ ist doppelt vergeben")));
        }
    }

    // Items.
    let mut keys_seen = BTreeSet::new();
    for (sec, item) in &items {
        let key = item.key_name();
        if key.is_empty() {
            errors.push(err(format!(
                "Item ohne Nummer oder ID in Abschnitt „{}“",
                sec.title
            )));
            continue;
        }
        if !keys_seen.insert(key.clone()) {
            errors.push(err(format!("Item {key} ist doppelt")));
        }
        if let Some(s) = &item.scale {
            if !def.scales.iter().any(|x| &x.id == s) {
                errors.push(err(format!("Item {key}: Skala „{s}“ ist nicht definiert")));
            }
        }
        let set_name = item.answers.as_ref().or(def.default_answers.as_ref());
        let answers = set_name.and_then(|n| def.answer_sets.get(n));
        if answers.is_none() {
            let n = set_name.map_or("", |n| n.as_str());
            errors.push(err(format!(
                "Item {key}: Antwortset „{n}“ ist nicht definiert"
            )));
        }
        let scores = match &item.key {
            Some(KeyRef::Inline(v)) => Some(v),
            Some(KeyRef::Named(n)) => {
                let k = def.keys.get(n);
                if k.is_none() {
                    errors.push(err(format!(
                        "Item {key}: Punkteschlüssel „{n}“ ist nicht definiert"
                    )));
                }
                k
            }
            None => None,
        };
        match (item.scale.is_some(), item.key.is_some()) {
            (true, false) => errors.push(err(format!("Item {key}: Skala ohne Punkteschlüssel"))),
            (false, true) => errors.push(err(format!("Item {key}: Punkteschlüssel ohne Skala"))),
            _ => {}
        }
        if let (Some(k), Some(a)) = (scores, answers) {
            if k.len() != a.len() {
                errors.push(err(format!(
                    "Item {key}: Punkteschlüssel hat {} Werte, das Antwortset hat {} Antworten",
                    k.len(),
                    a.len()
                )));
            }
        }
    }

    // Expressions, against a default environment that grows as names become visible.
    let mut env = BTreeMap::new();
    for s in &def.scales {
        add_cutoff(&mut env, &s.id, s.cutoff);
    }
    let mut derived = Vec::new();
    for d in &def.derived {
        let w = format!("Abgeleiteter Wert „{}“", d.id);
        let e = compile(&d.value, &env, Want::Num, &w, &mut errors);
        add_cutoff(&mut env, &d.id, d.cutoff);
        derived.extend(e.map(|e| (d.clone(), e)));
    }
    for f in &def.fields {
        let v = match f.kind {
            FieldKind::Number => Value::Num(0.0),
            FieldKind::Checkbox => Value::Bool(false),
            FieldKind::Text | FieldKind::Date => Value::Str(String::new()),
        };
        env.insert(f.id.clone(), v);
    }
    for (_, i) in &items {
        if let Some(id) = &i.id {
            env.insert(id.clone(), Value::Str(String::new()));
        }
    }
    env.insert("fehlend".to_string(), Value::Num(0.0));
    let mut rules = Vec::new();
    for r in &def.rules {
        let w = format!("Regel „{}“", r.id);
        let e = compile(&r.when, &env, Want::Bool, &w, &mut errors);
        env.insert(r.id.clone(), Value::Bool(false));
        rules.extend(e.map(|e| (r.clone(), e)));
    }
    let mut caveats = Vec::new();
    for (i, c) in def.caveats.iter().enumerate() {
        let w = format!("Hinweis {}", i + 1);
        let e = compile(&c.when, &env, Want::Bool, &w, &mut errors);
        caveats.extend(e.map(|e| (c.clone(), e)));
    }
    let mut verdict = Vec::new();
    for (i, v) in def.verdict.iter().enumerate() {
        match v {
            VerdictEntry::When { when, text } => {
                let w = format!("Ergebnis-Regel {}", i + 1);
                let e = compile(when, &env, Want::Bool, &w, &mut errors);
                verdict.extend(e.map(|e| (Some(e), text.clone())));
            }
            VerdictEntry::Else { otherwise } => verdict.push((None, otherwise.clone())),
        }
    }

    // Result entries: scale, derived or item id.
    for r in &def.result {
        let known = def.scales.iter().any(|s| &s.id == r)
            || def.derived.iter().any(|d| &d.id == r)
            || items.iter().any(|(_, i)| i.id.as_ref() == Some(r));
        if !known {
            errors.push(err(format!("Ergebnis: „{r}“ ist nicht definiert")));
        }
    }

    if errors.is_empty() {
        Ok(Questionnaire {
            def,
            derived,
            rules,
            caveats,
            verdict,
        })
    } else {
        Err(errors)
    }
}

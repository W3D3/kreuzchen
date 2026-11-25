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
fn add_scored(
    env: &mut BTreeMap<String, Value>,
    id: &str,
    (value, over): (f64, bool),
    cutoff: Option<i64>,
) {
    env.insert(id.to_string(), Value::Num(value));
    if let Some(c) = cutoff {
        env.insert(format!("{id}.cutoff"), Value::Num(c as f64));
        env.insert(format!("{id}.over"), Value::Bool(over));
    }
}

/// How the shared environment gets its values: validation passes stand-ins, grading real ones.
type Env = BTreeMap<String, Value>;
/// Value and over-cutoff flag.
type Scored = (f64, bool);

pub(crate) struct EnvSource<'a, E> {
    /// Sum and over-cutoff flag of a scale.
    pub scale: &'a dyn Fn(&Scale) -> Scored,
    /// Unanswered items of a scale, exposed as `id.missing`.
    pub scale_missing: &'a dyn Fn(&Scale) -> f64,
    /// Value of a field; `None` leaves it out of the environment.
    pub field: &'a dyn Fn(&Field) -> Option<Value>,
    /// Answer text of an item with an `id` (empty when unanswered).
    pub item: &'a dyn Fn(&Item) -> String,
    pub missing: f64,
    /// Value and over-cutoff flag of a derived value, given the environment so far.
    pub derived: &'a mut dyn FnMut(&Derived, &Env) -> Result<Scored, E>,
    /// Outcome of a rule, given the environment so far.
    pub rule: &'a mut dyn FnMut(&Rule, &Env) -> Result<bool, E>,
}

/// Builds the expression environment in the one order both validation and grading use:
/// scales, fields, item ids, `missing`, derived, then rules (later ones see earlier ones).
pub(crate) fn build_env<E>(
    def: &Definition,
    src: EnvSource<'_, E>,
) -> Result<BTreeMap<String, Value>, E> {
    let mut env = BTreeMap::new();
    for s in &def.scales {
        add_scored(&mut env, &s.id, (src.scale)(s), s.cutoff);
        env.insert(
            format!("{}.missing", s.id),
            Value::Num((src.scale_missing)(s)),
        );
    }
    for f in &def.fields {
        let v = (src.field)(f);
        if f.kind == FieldKind::Number {
            env.insert(format!("{}.present", f.id), Value::Bool(v.is_some()));
        }
        if let Some(v) = v {
            env.insert(f.id.clone(), v);
        }
    }
    for i in def.sections.iter().flat_map(|s| &s.items) {
        if let Some(id) = &i.id {
            env.insert(id.clone(), Value::Str((src.item)(i)));
        }
    }
    env.insert("missing".to_string(), Value::Num(src.missing));
    for d in &def.derived {
        let v = (src.derived)(d, &env)?;
        add_scored(&mut env, &d.id, v, d.cutoff);
    }
    for r in &def.rules {
        let met = (src.rule)(r, &env)?;
        env.insert(r.id.clone(), Value::Bool(met));
    }
    Ok(env)
}

/// `^[A-Za-z_][A-Za-z0-9_]*$`, minus expression keywords and `missing`.
fn valid_name(n: &str) -> bool {
    let mut c = n.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
        && !["and", "or", "not", "true", "false", "missing"].contains(&n)
}

pub(crate) fn validate(def: Definition) -> Result<Questionnaire, Vec<LoadError>> {
    let mut errors = Vec::new();
    if def.schema != 1 {
        errors.push(err(format!("Unbekannte Schema-Version {}", def.schema)));
    }
    if let Some(v) = def
        .min_app
        .as_deref()
        .filter(|v| crate::app_version(v).is_none())
    {
        errors.push(err(format!("min_app „{v}“ ist keine Version wie 0.4.0")));
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
        if !valid_name(n) {
            errors.push(err(format!("Name „{n}“ ist nicht erlaubt")));
        }
        if !seen.insert(n) {
            errors.push(err(format!("Name „{n}“ ist doppelt vergeben")));
        }
    }

    for (name, keys) in &def.shortcuts {
        let wher = format!("Tasten für „{name}“");
        let Some(set) = def.answer_sets.get(name) else {
            errors.push(err(format!("{wher}: Antwortset ist nicht definiert")));
            continue;
        };
        let n = keys.chars().count();
        if n != set.len() {
            errors.push(err(format!(
                "{wher}: {n} Tasten, das Antwortset hat {} Antworten",
                set.len()
            )));
        }
        if keys.chars().any(char::is_whitespace) {
            errors.push(err(format!("{wher}: Leerzeichen sind nicht erlaubt")));
        }
        let mut seen = BTreeSet::new();
        for c in keys.chars().filter(|c| !seen.insert(*c)) {
            errors.push(err(format!("{wher}: Taste „{c}“ ist doppelt")));
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

    // Expressions, against a default environment of stand-in values. Compile errors are
    // collected; the stand-ins keep later expressions checkable.
    for f in &def.fields {
        let ok = f.default.as_ref().is_none_or(|d| f.kind.accepts(d));
        if !ok {
            errors.push(err(format!(
                "Feld „{}“: Standardwert passt nicht zum Typ",
                f.id
            )));
        }
    }
    let mut derived = Vec::new();
    let mut rules = Vec::new();
    let errs = std::cell::RefCell::new(std::mem::take(&mut errors));
    let env = build_env::<()>(
        &def,
        EnvSource {
            scale: &|_| (0.0, false),
            scale_missing: &|_| 0.0,
            field: &|f| {
                Some(match f.kind {
                    FieldKind::Number => Value::Num(0.0),
                    FieldKind::Checkbox => Value::Bool(false),
                    FieldKind::Text | FieldKind::Date => Value::Str(String::new()),
                })
            },
            item: &|_| String::new(),
            missing: 0.0,
            derived: &mut |d, env| {
                let w = format!("Abgeleiteter Wert „{}“", d.id);
                let e = compile(&d.value, env, Want::Num, &w, &mut errs.borrow_mut());
                derived.extend(e.map(|e| (d.clone(), e)));
                Ok((0.0, false))
            },
            rule: &mut |r, env| {
                let w = format!("Regel „{}“", r.id);
                let e = compile(&r.when, env, Want::Bool, &w, &mut errs.borrow_mut());
                rules.extend(e.map(|e| (r.clone(), e)));
                Ok(false)
            },
        },
    )
    .expect("stand-in callbacks never fail");
    errors = errs.into_inner();
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

    // Before 0.6.0 derived values saw only scales and earlier derived values, and there was no
    // `count`: such a file must say so, or older apps reject it instead of asking for an update.
    let late: BTreeSet<&str> = def
        .fields
        .iter()
        .map(|f| f.id.as_str())
        .chain(items.iter().filter_map(|(_, i)| i.id.as_deref()))
        .chain(["missing"])
        .collect();
    let reads_late = |e: &Expr| {
        expr::names(e)
            .iter()
            .any(|n| late.contains(n.split('.').next().unwrap_or_default()))
    };
    let mut exprs = (derived.iter().map(|(_, e)| e))
        .chain(rules.iter().map(|(_, e)| e))
        .chain(caveats.iter().map(|(_, e)| e))
        .chain(verdict.iter().filter_map(|(e, _)| e.as_ref()));
    let new_language = derived.iter().any(|(_, e)| reads_late(e)) || exprs.any(expr::counts);
    let min_app = def.min_app.as_deref().and_then(crate::app_version);
    if new_language && min_app.is_none_or(|v| v < vec![0, 6, 0]) {
        errors.push(err(
            "count(…) und Felder, Fragen oder „missing“ in abgeleiteten Werten brauchen min_app: \"0.6.0\""
                .to_string(),
        ));
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

    if let Some(n) = &def.norms {
        for id in n.tables.keys() {
            if !def.scales.iter().any(|s| &s.id == id) && !def.derived.iter().any(|d| &d.id == id) {
                errors.push(err(format!(
                    "Normtabelle „{id}“: keine Skala und kein abgeleiteter Wert"
                )));
            }
        }
        if n.bands.windows(2).any(|w| w[0].from >= w[1].from) {
            errors.push(err(
                "Ausprägungsgrade müssen nach „from“ aufsteigend sortiert sein".to_string(),
            ));
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

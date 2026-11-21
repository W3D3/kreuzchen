//! One-time upgrade of 0.4 definitions, whose names were German, to the English names of 0.5.
//! shortcut: only needed while 0.4 files exist (app data, Bibliothek repos); delete once they are gone.

use crate::schema::{Definition, FieldValue, VerdictEntry};
use crate::{app_version, expr, parse_definition, to_yaml};

/// The first app version that understands the English names; upgraded files ask for it.
pub const MIN_APP: &str = "0.5.0";

/// The field the app treats as the subject id, and the default token that fills it.
const SUBJECT: (&str, &str) = ("kennung", "subject_id");
const ANONYMOUS: (&str, &str) = ("anonym", "anonymous");

fn rename_name(n: &str, subject: bool) -> Option<String> {
    if n == "fehlend" {
        return Some("missing".into());
    }
    if subject && n == SUBJECT.0 {
        return Some(SUBJECT.1.into());
    }
    let (base, attr) = n.split_once('.')?;
    let attr = match attr {
        "fehlend" => "missing",
        "gesetzt" => "present",
        _ => return None,
    };
    let base = if subject && base == SUBJECT.0 {
        SUBJECT.1
    } else {
        base
    };
    Some(format!("{base}.{attr}"))
}

/// Every expression of `def`, mutably, with where it is for error messages.
fn expressions(def: &mut Definition) -> Vec<(String, &mut String)> {
    let mut out: Vec<(String, &mut String)> = Vec::new();
    for d in &mut def.derived {
        out.push((format!("Abgeleiteter Wert „{}“", d.id), &mut d.value));
    }
    for r in &mut def.rules {
        out.push((format!("Regel „{}“", r.id), &mut r.when));
    }
    for (i, c) in def.caveats.iter_mut().enumerate() {
        out.push((format!("Hinweis {}", i + 1), &mut c.when));
    }
    for (i, v) in def.verdict.iter_mut().enumerate() {
        if let VerdictEntry::When { when, .. } = v {
            out.push((format!("Ergebnis-Regel {}", i + 1), when));
        }
    }
    out
}

/// Names a definition declares itself (the ones an upgrade must not rename onto).
fn declared(def: &Definition) -> Vec<&str> {
    let items = def.sections.iter().flat_map(|s| &s.items);
    def.scales
        .iter()
        .map(|s| s.id.as_str())
        .chain(def.derived.iter().map(|d| d.id.as_str()))
        .chain(def.fields.iter().map(|f| f.id.as_str()))
        .chain(items.filter_map(|i| i.id.as_deref()))
        .chain(def.rules.iter().map(|r| r.id.as_str()))
        .collect()
}

/// `def` with the English names; `None` when it has nothing to upgrade.
pub fn upgrade_definition(def: &Definition) -> Result<Option<Definition>, String> {
    let mut new = def.clone();
    let subject = def.fields.iter().any(|f| f.id == SUBJECT.0);
    let names = declared(def);
    let taken = |n: &str| {
        names
            .contains(&n)
            .then(|| format!("Name „{n}“ ist schon vergeben"))
    };
    if let Some(e) = taken("missing") {
        return Err(e);
    }
    if let Some(e) = taken(SUBJECT.1).filter(|_| subject) {
        return Err(e);
    }
    for (wher, e) in expressions(&mut new) {
        *e = expr::rename(e, |n| rename_name(n, subject)).map_err(|m| format!("{wher}: {m}"))?;
    }
    for f in &mut new.fields {
        if f.id == SUBJECT.0 {
            f.id = SUBJECT.1.into();
        }
        if f.default == Some(FieldValue::Text(ANONYMOUS.0.into())) {
            f.default = Some(FieldValue::Text(ANONYMOUS.1.into()));
        }
    }
    if new == *def {
        return Ok(None);
    }
    let old_enough = new
        .min_app
        .as_deref()
        .and_then(app_version)
        .is_some_and(|v| v >= app_version(MIN_APP).unwrap());
    if !old_enough {
        new.min_app = Some(MIN_APP.into());
    }
    Ok(Some(new))
}

/// Replaces `key: old` with `key: new` where `old` is a whole word.
fn replace_value(text: &str, key: &str, old: &str, new: &str) -> String {
    let pat = format!("{key}: {old}");
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(&pat) {
        let after = &rest[i + pat.len()..];
        out.push_str(&rest[..i]);
        if after.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            out.push_str(&pat);
        } else {
            out.push_str(&format!("{key}: {new}"));
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Sets the top-level `min_app`, after `schema:` when there is none yet. An existing line is
/// replaced whole: its comment described the old value.
fn set_min_app(text: &str) -> String {
    let line = format!("min_app: \"{MIN_APP}\"");
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if let Some(l) = lines.iter_mut().find(|l| l.starts_with("min_app:")) {
        *l = line;
    } else if let Some(i) = lines.iter().position(|l| l.starts_with("schema:")) {
        lines.insert(i + 1, line);
    } else {
        lines.insert(0, line);
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// `yaml` upgraded when it is a 0.4 file that can be upgraded, else unchanged: the read-side
/// shortcut for files kept byte-identical (Bibliothek downloads, files arriving after migration).
pub fn upgraded(yaml: &str) -> std::borrow::Cow<'_, str> {
    match upgrade(yaml) {
        Ok(Some(new)) => new.into(),
        _ => yaml.into(),
    }
}

/// Upgrades a 0.4 definition text. `Ok(None)`: nothing to upgrade. `Err`: cannot be upgraded,
/// leave the file as it is. Edits the text in place so comments and layout survive; when that
/// does not reproduce the upgraded definition exactly, it falls back to re-serializing.
pub fn upgrade(yaml: &str) -> Result<Option<String>, String> {
    let def = parse_definition(yaml).map_err(|e| e.to_string())?;
    let Some(new) = upgrade_definition(&def)? else {
        return Ok(None);
    };
    let mut pairs: Vec<(String, String)> = {
        let mut old = def.clone();
        let mut upd = new.clone();
        expressions(&mut old)
            .into_iter()
            .zip(expressions(&mut upd))
            .filter(|((_, a), (_, b))| a != b)
            .map(|((_, a), (_, b))| (a.clone(), b.clone()))
            .collect()
    };
    // Longest first, so an expression that contains another is replaced whole.
    pairs.sort_by_key(|(a, _)| std::cmp::Reverse(a.len()));
    pairs.dedup();
    let mut text = yaml.to_string();
    for (a, b) in &pairs {
        text = text.replace(a.as_str(), b);
    }
    text = replace_value(&text, "id", SUBJECT.0, SUBJECT.1);
    text = replace_value(&text, "default", ANONYMOUS.0, ANONYMOUS.1);
    if new.min_app != def.min_app {
        text = set_min_app(&text);
    }
    if parse_definition(&text).ok().as_ref() == Some(&new) {
        return Ok(Some(text));
    }
    Ok(Some(to_yaml(&new)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "\
# Fictional test questionnaire.
schema: 1
id: t
version: \"1\"
title: T
fields:
  - { id: kennung, label: Kennung, type: text, default: anonym }  # who
  - { id: alter, label: Alter, type: number }
scales:
  - { id: A, label: A, cutoff: 2 }
sections:
  - title: S
    items:
      - { n: 1, scale: A, key: [0, 1], text: \"fehlend?\" }
answer_sets: { j: [n, j] }
default_answers: j
rules:
  - { id: r1, label: R, when: A.fehlend > 0 or fehlend > 1 }
caveats:
  - { when: not alter.gesetzt and kennung == \"anonym\", text: \"fehlend\" }
verdict:
  - { else: ok }
";

    #[test]
    fn upgrades_in_place_keeping_comments() {
        let new = upgrade(OLD).unwrap().unwrap();
        assert!(new.contains("# Fictional test questionnaire."), "{new}");
        assert!(new.contains("default: anonymous }  # who"), "{new}");
        assert!(
            new.contains("schema: 1\nmin_app: \"0.5.0\"\nid: t"),
            "{new}"
        );
        assert!(
            new.contains("when: A.missing > 0 or missing > 1 }"),
            "{new}"
        );
        assert!(new
            .contains("when: not alter.present and subject_id == \"anonym\", text: \"fehlend\" }"));
        assert!(new.contains("{ id: subject_id, label: Kennung,"), "{new}");
        assert!(new.contains("text: \"fehlend?\""), "{new}");
        crate::load(&new).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(upgrade(&new).unwrap(), None, "idempotent");
    }

    #[test]
    fn falls_back_to_reserializing_when_the_text_cannot_be_edited() {
        // A folded expression is not written the way it parses.
        let old = OLD.replace(
            "  - { id: r1, label: R, when: A.fehlend > 0 or fehlend > 1 }",
            "  - id: r1\n    label: R\n    when: >-\n      A.fehlend > 0\n      or fehlend > 1",
        );
        crate::parse_definition(&old).unwrap();
        let new = upgrade(&old).unwrap().unwrap();
        assert!(!new.contains("fehlend >"), "{new}");
        crate::load(&new).unwrap_or_else(|e| panic!("{e:?}"));
    }

    #[test]
    fn refuses_what_it_cannot_upgrade() {
        assert!(upgrade("schema: [").is_err());
        let taken = OLD.replace("id: alter,", "id: missing,");
        assert!(upgrade(&taken).unwrap_err().contains("missing"));
        let bad = OLD.replace("fehlend > 1 }", "fehlend > ä }");
        assert!(upgrade(&bad).unwrap_err().contains("Regel „r1“"));
    }

    #[test]
    fn leaves_english_files_alone() {
        let en = upgrade(OLD).unwrap().unwrap();
        assert_eq!(upgrade(&en).unwrap(), None);
        let plain = "schema: 1\nid: p\nversion: \"1\"\ntitle: P\n";
        assert_eq!(upgrade(plain).unwrap(), None);
    }
}

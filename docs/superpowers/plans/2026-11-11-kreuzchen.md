# Kreuzchen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a German-only Tauri 2 desktop app that grades paper questionnaires from YAML definitions loaded at runtime, starting with the MBAS.

**Architecture:** A pure Rust crate (`kreuzchen-engine`) parses, validates and grades questionnaire YAML and renders the copy text. It has no Tauri dependency and carries all grading tests. The Tauri app crate (`src-tauri`) owns the data folder: it scans definitions, stores saved results and exposes both through commands. A Svelte 5 + TypeScript UI renders four screens and holds no grading logic.

**Tech Stack:** Rust stable, Tauri 2, Svelte 5 + TypeScript + Vite, Vitest, `serde` + `serde_norway` (maintained `serde_yaml` fork), `serde_json`, `sha2`, `tempfile` (dev). Tauri plugins: `dialog`, `opener`, `clipboard-manager`. Fonts: `@fontsource/atkinson-hyperlegible`, `@fontsource/ibm-plex-mono`.

**Spec:** `docs/superpowers/specs/2026-11-11-kreuzchen-design.md`. Read it before starting any task.

## Global Constraints

- All user-facing text is German. There is no i18n layer.
- No network access at runtime. The CSP is `default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'`. No remote URLs, no updater, no telemetry. Fonts are bundled via `@fontsource`.
- File names, paths, IDs and parse errors are visible only while Alt is held, or after toggling with Ctrl+Shift+D (class `tech` on elements; hidden unless `body.tech` is set).
- Default Kennung: `Anonym DD.MM.YYYY HH:MM`, using local time with zero padding, e.g. `Anonym 11.11.2026 22:23`.
- Copyrighted material (the MBAS PDF and `mbas.yaml`) lives only in the gitignored `local/` folder. Never commit or bundle it, and never copy MBAS item texts into tests, fixtures or docs.
- The committed fixture is `examples/definitionen/beispiel.yaml` (Task 1). CI tests use only this file.
- Tauri app identifier `io.github.w3d3.kreuzchen`, product name `Kreuzchen`.
- Data folders: `<app_data_dir>/definitionen/` and `<app_data_dir>/auswertungen/`, created at startup.
- Serialized Rust types use Rust field names (snake_case) on the wire. TypeScript types mirror them exactly.
- Commit unsigned (`git -c commit.gpgsign=false commit ...`); the user approved unsigned commits for this repo. End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No code signing. Release builds are unsigned.

## Review Focus

1. **A definition file changes between opening a form and pressing Auswerten.** Expected: grading refuses with "Der Fragebogen wurde geändert. Bitte neu starten." and never grades against mismatched items. Tested in Task 8.
2. **Windows-made YAML files (UTF-8 BOM, CRLF line endings).** Expected: they load like any other file. Tested in Task 1.
3. **Stray files in `definitionen/`** (`.DS_Store`, `README.txt`, `mbas.yaml.bak`, subfolders). Expected: only `*.yaml`/`*.yml` files are listed; nothing else shows as broken. Tested in Task 8.
4. **Saved or imported answers that no longer fit the definition** (unknown item key, answer index out of range). Expected: `GradeError` with a readable German message, never a panic. Tested in Task 4.
5. **Two saves in the same second, or a Kennung with `/ : *`.** Expected: file names come from timestamp + questionnaire ID, never from the Kennung, and collisions get a `-2`, `-3` suffix. Tested in Task 9.

---

## File Structure

```
Cargo.toml                         workspace: members ["crates/engine", "src-tauri"]
crates/engine/
  Cargo.toml                       name = "kreuzchen-engine"
  src/lib.rs                       pub API: load, Questionnaire, Input, Grading, errors
  src/schema.rs                    serde types for the YAML (Definition and children)
  src/expr.rs                      expression parser + evaluator
  src/validate.rs                  Definition -> compiled Questionnaire, Vec<LoadError>
  src/grade.rs                     Questionnaire::grade
  src/text.rs                      Questionnaire::plain_text
  tests/load.rs  tests/validate.rs  tests/grade.rs  tests/text.rs  tests/mbas.rs
examples/definitionen/beispiel.yaml committed invented fixture
src-tauri/
  src/main.rs  src/lib.rs          Tauri setup, state, command registration
  src/definitions.rs               scan/open definition files, hashing
  src/store.rs                     saved results on disk
  src/commands.rs                  #[tauri::command] wrappers
  capabilities/default.json  tauri.conf.json
src/
  App.svelte                       screen state + top navigation
  app.css                          tokens and base styles
  lib/types.ts  lib/api.ts         wire types + invoke wrappers
  lib/kennung.ts  lib/input.ts  lib/keyboard.ts  lib/tech.svelte.ts
  screens/List.svelte  Entry.svelte  Result.svelte  Saved.svelte
  lib/*.test.ts                    vitest
.github/workflows/ci.yml  release.yml
docs/installation.md  README.md
```

---

### Task 1: Toolchain, workspace and schema loading

**Files:**
- Create: `Cargo.toml`, `crates/engine/Cargo.toml`, `crates/engine/src/lib.rs`, `crates/engine/src/schema.rs`, `examples/definitionen/beispiel.yaml`
- Modify: `.gitignore` (add `target/`, `node_modules/`, `dist/`)
- Test: `crates/engine/tests/load.rs`

**Interfaces:**
- Produces (`schema.rs`, all `#[derive(Debug, Clone, Serialize, Deserialize)]` with `#[serde(deny_unknown_fields)]` on structs):
  - `Definition { schema: u32, id: String, version: String, title: String, authors: Option<String>, copyright: Option<String>, cutoff_compare: Compare, answer_sets: BTreeMap<String, Vec<String>>, keys: BTreeMap<String, Vec<i64>>, default_answers: Option<String>, fields: Vec<Field>, scales: Vec<Scale>, derived: Vec<Derived>, sections: Vec<Section>, result: Vec<String>, rules: Vec<Rule>, caveats: Vec<Caveat>, verdict: Vec<VerdictEntry> }`. Use `#[serde(default)]` on every `Option`, `Vec` and `BTreeMap`.
  - `enum Compare { #[serde(rename = ">")] Gt, #[serde(rename = ">=")] Ge }`
  - `Field { id: String, label: String, #[serde(rename = "type")] kind: FieldKind, default: Option<FieldValue> }`
  - `enum FieldKind { Text, Number, Date, Checkbox }` with `rename_all = "lowercase"`
  - `#[serde(untagged)] enum FieldValue { Bool(bool), Num(f64), Text(String) }`
  - `Scale { id, label, cutoff: Option<i64> }`
  - `Derived { id, label, value: String, cutoff: Option<i64> }`
  - `Section { title, intro: Option<String>, items: Vec<Item> }`
  - `Item { n: Option<u32>, id: Option<String>, text: String, answers: Option<String>, scale: Option<String>, key: Option<KeyRef> }`, with `Item::key_name(&self) -> String` returning `id`, or else `n` as a string.
  - `#[serde(untagged)] enum KeyRef { Named(String), Inline(Vec<i64>) }`
  - `Rule { id, label, when: String }`, `Caveat { when: String, text: String }`
  - `#[serde(untagged)] enum VerdictEntry { When { when: String, text: String }, Else { #[serde(rename = "else")] otherwise: String } }`
- Produces (`lib.rs`):
  - `pub struct LoadError { pub message: String, pub line: Option<usize> }`, implementing `Display` as `"Zeile {line}: {message}"`, or just `message` when `line` is `None`.
  - `pub fn parse_definition(yaml: &str) -> Result<Definition, LoadError>`. It strips a leading `\u{feff}`, and maps the `serde_norway` error location to `line`.

- [ ] **Step 1: Install the toolchain.**
  - Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`.
  - Linux system packages: `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`, or the distro equivalent from the Tauri 2 prerequisites page.
  - If you have no sudo access, stop and ask the user to run the package install.
  - Check: `cargo --version` prints a version and `pkg-config --modversion webkit2gtk-4.1` prints a version.

- [ ] **Step 2: Write `examples/definitionen/beispiel.yaml` exactly:**

```yaml
schema: 1
id: beispiel
version: "1"
title: Beispiel-Fragebogen
cutoff_compare: ">="
answer_sets:
  haeufigkeit: [nie, selten, oft, immer]
  ja_nein: [Nein, Ja]
keys:
  normal: [0, 1, 2, 3]
  umgepolt: [3, 2, 1, 0]
default_answers: haeufigkeit
fields:
  - { id: kennung, label: Kennung, type: text, default: anonym }
  - { id: alter, label: Alter, type: number }
  - { id: erwachsen, label: Volljährig, type: checkbox }
scales:
  - { id: X, label: Skala X, cutoff: 4 }
  - { id: Y, label: Skala Y, cutoff: 3 }
derived:
  - { id: gesamt, label: Gesamtsumme, value: X + Y, cutoff: 7 }
sections:
  - title: Teil 1
    intro: Beispieltext.
    items:
      - { n: 1, scale: X, key: normal, text: "Frage eins?" }
      - { n: 2, scale: X, key: umgepolt, text: "Frage zwei?" }
      - { n: 3, scale: Y, key: [0, 0, 1, 2], text: "Frage drei?" }
  - title: Teil 2
    items:
      - { n: 4, scale: Y, key: normal, text: "Frage vier?" }
      - { id: zusatz, answers: ja_nein, text: "Zusatzfrage" }
result: [gesamt, X, Y, zusatz]
rules:
  - { id: r1, label: "Regel 1: Gesamtsumme über Cut-off", when: gesamt.over }
  - { id: r2, label: "Regel 2: Gesamtsumme und Skala X über Cut-off", when: r1 and X.over }
  - { id: vollstaendig, label: "Höchstens 1 Antwort fehlt", when: fehlend <= 1 }
caveats:
  - { when: erwachsen, text: "Nur für Kinder normiert." }
  - { when: zusatz == "Ja", text: "Zusatzfrage mit Ja beantwortet." }
verdict:
  - { when: not vollstaendig, text: "Nicht auswertbar: zu viele fehlende Antworten" }
  - { when: r2, text: "Auffällig (Regel 2)" }
  - { when: r1, text: "Auffällig (Regel 1)" }
  - { else: "Unauffällig" }
```

- [ ] **Step 3: Write the failing tests in `tests/load.rs`.** Read the fixture with `include_str!("../../../examples/definitionen/beispiel.yaml")`.
  - `parses_beispiel`:
    - `d.id == "beispiel"`
    - `d.sections.len() == 2`
    - `matches!(d.cutoff_compare, Compare::Ge)`
    - `matches!(d.sections[0].items[2].key, Some(KeyRef::Inline(ref v)) if v == &vec![0, 0, 1, 2])`
    - `d.sections[1].items[1].key_name() == "zusatz"`
    - `d.sections[0].items[0].key_name() == "1"`
    - `matches!(d.verdict[3], VerdictEntry::Else { .. })`
  - `parses_with_bom_and_crlf`: prefix `"\u{feff}"` and replace `"\n"` with `"\r\n"`. It still parses and `d.id == "beispiel"`.
  - `syntax_error_has_line`: `parse_definition("schema: 1\nid: [\n")` is an `Err` whose `line` is `Some(_)`.
  - `unknown_field_rejected`: the fixture with `"title:"` replaced by `"titel:"` is an `Err`.

- [ ] **Step 4: Run `cargo test -p kreuzchen-engine --test load`.** Expected: compile failure, because the types don't exist yet.

- [ ] **Step 5: Implement the workspace, `crates/engine/Cargo.toml` and `schema.rs`, and `parse_definition` in `lib.rs`.** Dependencies: `serde` (derive), `serde_norway`, `serde_json`.

- [ ] **Step 6: Run `cargo test -p kreuzchen-engine --test load`.** Expected: 4 passed.

- [ ] **Step 7: Commit:** `feat(engine): workspace and YAML schema types`.

---

### Task 2: Expression language

**Files:**
- Create: `crates/engine/src/expr.rs` (unit tests in a `#[cfg(test)] mod tests` at the bottom)

**Interfaces:**
- Produces:
  - `pub enum Value { Num(f64), Bool(bool), Str(String) }` (`Debug, Clone, PartialEq`)
  - `pub enum Expr` (the private shape is the implementer's choice)
  - `pub fn parse(src: &str) -> Result<Expr, String>`
  - `pub fn eval(e: &Expr, env: &BTreeMap<String, Value>) -> Result<Value, String>`
  - `pub fn names(e: &Expr) -> BTreeSet<String>`

The grammar, lowest precedence first (it fixes operator behaviour, so it is spelled out):

```
expr  := or
or    := and ("or" and)*
and   := not ("and" not)*
not   := "not" not | cmp
cmp   := sum (("==" | "!=" | ">=" | "<=" | ">" | "<") sum)?
sum   := atom ("+" atom)*
atom  := number | "\"" chars "\"" | "true" | "false" | name | "(" expr ")"
name  := [A-Za-z_][A-Za-z0-9_]* ("." [A-Za-z_][A-Za-z0-9_]*)?
```

- `and`, `or` and `not` need booleans.
- `+` and the ordering comparisons need numbers.
- `==` and `!=` need both sides of the same type.
- A name missing from `env` is an error: `„{name}“ hat keinen Wert`.
- A type error is `Typfehler: …`.
- Parse errors name the position: `Unerwartetes Zeichen an Position {i}` or `Ausdruck unvollständig`.

- [ ] **Step 1: Write the failing unit tests:**
  - `precedence`: `eval(parse("1 + 2 >= 3 and not false"), {})` is `Bool(true)`.
  - `dotted_names`: with env `{"A.over": Bool(true), "A": Num(40.0)}`, `"A.over and A > 39"` is `Bool(true)`.
  - `string_equality`: env `{"sev": Str("Ja")}`, `sev == "Ja"` is `Bool(true)` and `sev != "Ja"` is `Bool(false)`.
  - `missing_name`: `eval(parse("x > 1"), {})` is an `Err` containing `„x“ hat keinen Wert`.
  - `type_error`: `"1 and true"` is an `Err` containing `Typfehler`.
  - `parse_errors`: `"r1 and and"`, `"(1 + 2"` and `"1 +"` are all `Err`.
  - `names_collects`: `names(parse("gesamt.over and not k1 or fehlend <= 1"))` is `{"gesamt.over", "k1", "fehlend"}`.

- [ ] **Step 2: Run `cargo test -p kreuzchen-engine expr`.** Expected: compile failure.

- [ ] **Step 3: Implement `parse` as a hand-written tokenizer plus recursive-descent parser, and `eval` and `names`, in `expr.rs`.** Don't use an external crate. Declare `pub mod expr;` in `lib.rs`.

- [ ] **Step 4: Run `cargo test -p kreuzchen-engine expr`.** Expected: 7 passed.

- [ ] **Step 5: Commit:** `feat(engine): expression language`.

---

### Task 3: Validation and `load`

**Files:**
- Create: `crates/engine/src/validate.rs`
- Modify: `crates/engine/src/lib.rs`
- Test: `crates/engine/tests/validate.rs`

**Interfaces:**
- Consumes: `parse_definition` and the schema types (Task 1); `expr::{parse, eval, names, Value}` (Task 2).
- Produces:
  - `pub struct Questionnaire { pub def: Definition, /* compiled expressions, private */ }`
  - `pub fn load(yaml: &str) -> Result<Questionnaire, Vec<LoadError>>`. It calls `parse_definition`, then validates and collects **all** errors.
  - `impl Questionnaire { pub fn item_count(&self) -> usize /* items with a scale */; pub fn scale_count(&self) -> usize /* def.scales.len() */ }`

Validation rules and their exact German messages. Every message names the thing it's about; `line` is `None` for these.

| Check | Message |
|---|---|
| `schema != 1` | `Unbekannte Schema-Version {n}` |
| an item without `n` or `id` | `Item ohne Nummer oder ID in Abschnitt „{title}“` |
| duplicate item key | `Item {key} ist doppelt` |
| duplicate name across scales, derived, fields, item `id`s, rules | `Name „{id}“ ist doppelt vergeben` |
| item scale unknown | `Item {key}: Skala „{scale}“ ist nicht definiert` |
| item answer set unknown, or none and no `default_answers` | `Item {key}: Antwortset „{name}“ ist nicht definiert` |
| named key unknown | `Item {key}: Punkteschlüssel „{name}“ ist nicht definiert` |
| key length ≠ answer count | `Item {key}: Punkteschlüssel hat {k} Werte, das Antwortset hat {a} Antworten` |
| scale set without a key | `Item {key}: Skala ohne Punkteschlüssel` |
| key without a scale | `Item {key}: Punkteschlüssel ohne Skala` |
| `result` entry unknown (must be a scale, derived or item `id`) | `Ergebnis: „{id}“ ist nicht definiert` |
| expression parse or type error | `{where}: {expr error}` |

`{where}` is `Abgeleiteter Wert „{id}“`, `Regel „{id}“`, `Hinweis {i+1}` or `Ergebnis-Regel {i+1}`.

Type-check every expression by evaluating it against a **default environment** that holds:
- every scale and derived ID as `Num(0)`
- `{id}.cutoff` as `Num` and `{id}.over` as `Bool(false)` for each scale or derived with a cutoff
- fields: `number` as `Num(0)`, `checkbox` as `Bool(false)`, `text`/`date` as `Str("")`
- every item `id` as `Str("")`
- `fehlend` as `Num(0)`
- each rule ID as `Bool(false)`, but only **after** that rule, so rules see only earlier rules
- each derived value sees only the scales and the earlier derived values

Results must have the right type: derived values `Num`, `when` expressions `Bool`. The message for a wrong one is `{where}: Ergebnis muss eine Zahl sein` or `{where}: Bedingung muss wahr/falsch ergeben`. An unknown name shows up as eval's `„x“ hat keinen Wert`.

- [ ] **Step 1: Write the failing tests.** Add a helper `fn errs(from: &str, to: &str) -> Vec<String>`. It loads the fixture with one `replacen(from, to, 1)` applied, asserts `Err`, and returns the messages. Tests:
  - `beispiel_is_valid`: `load(fixture).is_ok()`; `item_count() == 4`; `scale_count() == 2`.
  - `unknown_scale`: `errs("scale: Y, key: normal", "scale: Z, key: normal")` contains `"Item 4: Skala „Z“ ist nicht definiert"`.
  - `key_length`: `errs("[0, 0, 1, 2]", "[0, 1]")` contains `"Item 3: Punkteschlüssel hat 2 Werte, das Antwortset hat 4 Antworten"`.
  - `duplicate_item`: `errs("n: 4", "n: 3")` contains `"Item 3 ist doppelt"`.
  - `bad_expression`: `errs("when: r1 and X.over", "when: r1 and and")` has one message starting with `"Regel „r2“: "`.
  - `unknown_name`: `errs("when: gesamt.over", "when: gesamtt.over")` has one message containing `„gesamtt.over“ hat keinen Wert`.
  - `rule_sees_only_earlier_rules`: `errs("when: r1 and X.over", "when: r2 and X.over")` is non-empty.
  - `wrong_type`: `errs("value: X + Y", "value: X > Y")` contains `"Abgeleiteter Wert „gesamt“: Ergebnis muss eine Zahl sein"`.
  - `unknown_answer_set`: `errs("answers: ja_nein", "answers: jn")` contains `"Item zusatz: Antwortset „jn“ ist nicht definiert"`.
  - `bad_compare`: `errs("cutoff_compare: \">=\"", "cutoff_compare: \"=>\"")` is non-empty.
  - `unknown_result`: `errs("result: [gesamt, X, Y, zusatz]", "result: [gesamt, Q]")` contains `"Ergebnis: „Q“ ist nicht definiert"`.
  - `collects_all`: replacing both `scale: Y, key: normal` (with `scale: Z, key: normal`) and `[0, 0, 1, 2]` (with `[0, 1]`) yields ≥ 2 messages.

- [ ] **Step 2: Run `cargo test -p kreuzchen-engine --test validate`.** Expected: compile failure.

- [ ] **Step 3: Implement `validate.rs`.** It is a function `pub(crate) fn validate(def: Definition) -> Result<Questionnaire, Vec<LoadError>>`. Store each parsed `Expr` in `Questionnaire` next to its derived/rule/caveat/verdict entry. `load` lives in `lib.rs`.

- [ ] **Step 4: Run `cargo test -p kreuzchen-engine`.** Expected: all pass.

- [ ] **Step 5: Commit:** `feat(engine): definition validation`.

---

### Task 4: Grading

**Files:**
- Create: `crates/engine/src/grade.rs`
- Modify: `crates/engine/src/lib.rs`
- Test: `crates/engine/tests/grade.rs`

**Interfaces:**
- Consumes: `Questionnaire` (Task 3), `FieldValue` (Task 1), `expr` (Task 2).
- Produces (`Serialize, Deserialize, Debug, Clone, PartialEq` on all of them):
  - `pub struct Input { pub fields: BTreeMap<String, FieldValue>, pub answers: BTreeMap<String /* item key_name */, usize /* answer index */> }`
  - `pub struct Grading { pub rows: Vec<ResultRow>, pub rules: Vec<RuleOutcome>, pub caveats: Vec<String>, pub verdict: Option<String>, pub missing: u32, pub missing_items: Vec<String> }`
  - `#[serde(tag = "kind", rename_all = "lowercase")] pub enum ResultRow { Sum { id: String, label: String, value: i64, cutoff: Option<i64>, over: bool }, Answer { id: String, label: String, answer: Option<String> } }`. The label of an `Answer` row is the item `text`.
  - `pub struct RuleOutcome { pub id: String, pub label: String, pub met: bool }`
  - `pub enum GradeError { UnknownItem(String), AnswerOutOfRange { item: String, index: usize }, UnknownField(String), Eval(String) }`. `Display` messages:
    - `Antwort für unbekannte Frage {item}`
    - `Frage {item}: Antwort {index+1} gibt es nicht`
    - `Unbekanntes Feld „{id}“`
    - the eval message as is
  - `impl Questionnaire { pub fn grade(&self, input: &Input) -> Result<Grading, GradeError> }`

Semantics:
- **Scale sum:** add up the key points of each answered item of that scale.
- **`over`:** compare `value` with `cutoff` using `cutoff_compare`. It is `false` when there is no cutoff.
- **Missing answers:** `missing` and `missing_items` (key names, in document order) count **only items with a scale**.
- **Rows:** in `result` order.
- **Environment:** like Task 3's, but with real values. A field missing from the input takes its `default` from the definition; with no default, a checkbox is `false` and text is `""`. A number with neither is left out of the environment, so referencing it gives an `Eval` error.
- **Unanswered items with an `id`:** they appear as `Str("")` in the environment and as `answer: None` in the row.
- **Verdict:** the first matching `When`, else the `Else` text, else `None`.

- [ ] **Step 1: Write the failing tests.** Helper: `fn grade(answers: &[(&str, usize)], fields: &[(&str, FieldValue)]) -> Grading`. Add a helper `sum(g, id) -> (i64, bool)` that reads a `Sum` row's value and `over`. The answer indexes come from `beispiel.yaml`:

| test | answers (key → index) | fields | assertions |
|---|---|---|---|
| `all_first` | 1→0, 2→0, 3→0, 4→0, zusatz→0 | – | X=(3,false), Y=(0,false), gesamt=(3,false); rules r1 false, r2 false, vollstaendig true; verdict `Unauffällig`; zusatz answer `Some("Nein")`; caveats empty; missing 0 |
| `all_last` | 1→3, 2→3, 3→3, 4→3, zusatz→1 | – | X=(3,false), Y=(5,true), gesamt=(8,true); r1 true, r2 false; verdict `Auffällig (Regel 1)`; caveats == [`Zusatzfrage mit Ja beantwortet.`] |
| `exactly_at_cutoff` | 1→3, 2→2, 3→0, 4→3, zusatz→0 | – | X=(4,true), Y=(3,true), gesamt=(7,true); r2 true; verdict `Auffällig (Regel 2)` |
| `one_below_cutoff` | 1→3, 2→3, 3→0, 4→3, zusatz→0 | erwachsen=Bool(true) | X=(3,false), gesamt=(6,false); verdict `Unauffällig`; caveats == [`Nur für Kinder normiert.`] |
| `missing_answers` | 1→3 | – | X=(3,false), Y=(0,false); missing 3; missing_items ["2","3","4"]; vollstaendig false; verdict `Nicht auswertbar: zu viele fehlende Antworten`; zusatz answer `None` |
| `strict_compare` | as `exactly_at_cutoff`, on the fixture with `">="` replaced by `">"` | – | X=(4,false), gesamt=(7,false); verdict `Unauffällig` |
| `unknown_item` | 9→0 | – | `Err(GradeError::UnknownItem("9"))` |
| `out_of_range` | 1→4 | – | `Err(GradeError::AnswerOutOfRange { item: "1", index: 4 })` |
| `unknown_field` | – | foo=Bool(true) | `Err(GradeError::UnknownField("foo"))` |

- [ ] **Step 2: Run `cargo test -p kreuzchen-engine --test grade`.** Expected: compile failure.

- [ ] **Step 3: Implement `grade.rs`.** Re-export `Input`, `Grading`, `ResultRow`, `RuleOutcome` and `GradeError` from `lib.rs`.

- [ ] **Step 4: Run `cargo test -p kreuzchen-engine`.** Expected: all pass.

- [ ] **Step 5: Commit:** `feat(engine): grading`.

---

### Task 5: Copy text

**Files:**
- Create: `crates/engine/src/text.rs`
- Test: `crates/engine/tests/text.rs`

**Interfaces:**
- Consumes: `Questionnaire`, `Input`, `Grading` (Task 4).
- Produces: `impl Questionnaire { pub fn plain_text(&self, input: &Input, g: &Grading) -> String }`

The format is exact copy, one line per entry:
- **Title line:** `{title} (Version {version})`.
- **One line per field, in definition order:** `{label}: {value}`.
  - Checkboxes print `Ja` or `Nein`.
  - Numbers print without `.0` when they are whole.
  - Empty values print `–`.
- **Blank line.**
- **One line per row:**
  - `Sum` rows: `{label}: {value}`. With a cutoff, append ` (Cut-off {cutoff})`, and if `over`, also append ` – über Cut-off`.
  - `Answer` rows: `{label}: {answer or –}`.
- **Blank line.**
- **One line per rule:** `{label}: erfüllt` or `{label}: nicht erfüllt`.
- **Missing answers:** `Fehlende Antworten: {missing}`. If `missing > 0`, append ` (Fragen {missing_items joined by ", "})`.
- **One line per caveat:** `Hinweis: {text}`.
- **Verdict:** `Ergebnis: {verdict}`, only if there is one.

- [ ] **Step 1: Write the failing test `exactly_at_cutoff_text`.** Use Task 4's `exactly_at_cutoff` answers with fields `kennung=Text("Anonym 11.11.2026 22:23")` and `alter=Num(9.0)`. Assert the output equals:

```
Beispiel-Fragebogen (Version 1)
Kennung: Anonym 11.11.2026 22:23
Alter: 9
Volljährig: Nein

Gesamtsumme: 7 (Cut-off 7) – über Cut-off
Skala X: 4 (Cut-off 4) – über Cut-off
Skala Y: 3 (Cut-off 3) – über Cut-off
Zusatzfrage: Nein

Regel 1: Gesamtsumme über Cut-off: erfüllt
Regel 2: Gesamtsumme und Skala X über Cut-off: erfüllt
Höchstens 1 Antwort fehlt: erfüllt
Fehlende Antworten: 0
Ergebnis: Auffällig (Regel 2)
```

  Also write `missing_text`: with Task 4's `missing_answers` input, the output contains `Fehlende Antworten: 3 (Fragen 2, 3, 4)` and `Zusatzfrage: –`.

- [ ] **Step 2: Run `cargo test -p kreuzchen-engine --test text`.** Expected: FAIL.

- [ ] **Step 3: Implement `text.rs`.**

- [ ] **Step 4: Run `cargo test -p kreuzchen-engine`.** Expected: all pass.

- [ ] **Step 5: Commit:** `feat(engine): plain-text result`.

---

### Task 6: Local MBAS tests

**Files:**
- Test: `crates/engine/tests/mbas.rs`

**Interfaces:**
- Consumes: `load`, `Input`, `grade` (Tasks 3–4).

These tests read `local/definitionen/mbas.yaml` at runtime: `concat!(env!("CARGO_MANIFEST_DIR"), "/../../local/definitionen/mbas.yaml")`. If the file is missing, each test prints `skipped: local/definitionen/mbas.yaml not present` and returns early. They must never embed MBAS content.

The expected values follow from the transcription: 13 reversed items in A, 11 in B, 3 in C and 0 in D.

- [ ] **Step 1: Write the tests:**
  - `mbas_loads`: loads without errors; `item_count() == 51`; `scale_count() == 4`.
  - `mbas_all_niemals`: items 1–51 → index 0, `sev` → 0. Expect A=52, B=44, C=12, D=0, gesamt=108; rules `k1` and `k2` met; verdict `Verdacht auf Asperger-Syndrom (Kriterium 2 erfüllt)`.
  - `mbas_all_immer`: items 1–51 → index 4. Expect A=16, B=8, C=36, D=36, gesamt=96; `k1` not met; verdict `Kein Verdacht nach MBAS`.
  - `mbas_hand_cases`: for each `local/faelle/*.json` file, deserialize `{ "input": Input, "expect": { "<row id>": <value>, ... } }`, grade, and assert each listed `Sum` value. With no files present, it passes.

- [ ] **Step 2: Run `cargo test -p kreuzchen-engine --test mbas -- --nocapture`.** Expected: 4 passed on a machine with `local/`; on others they pass with "skipped".

- [ ] **Step 3: Human step. Tell the user, don't do it yourself.** The user checks `local/definitionen/mbas.yaml` against the printed overlay, especially items 29 and 33–37. They also hand-score at least one real paper case into `local/faelle/fall1.json`.

- [ ] **Step 4: Commit:** `test(engine): local MBAS checks`.

---

### Task 7: Tauri app scaffold

**Files:**
- Create: `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html`, `src/main.ts`, `src/App.svelte`, `src/app.css`, `src-tauri/**` (from the template)
- Modify: `Cargo.toml` (add `src-tauri` to workspace members), `.gitignore`

**Interfaces:**
- Produces: `src-tauri/src/lib.rs` with `pub fn run()`, and a managed state `pub struct Paths { pub definitions: PathBuf, pub results: PathBuf }`, created in `setup` with `create_dir_all`.

- [ ] **Step 1: Scaffold.** In a temp directory, run `npm create tauri-app@latest kreuzchen -- --template svelte-ts --manager npm --identifier io.github.w3d3.kreuzchen`. Move its files into the repo root and keep the existing `docs/`, `crates/`, `examples/`, `local/` and `.gitignore`. Merge the template's `.gitignore` lines into ours.

- [ ] **Step 2: Configure.**
  - `tauri.conf.json`:
    - `productName: "Kreuzchen"`
    - window `title: "Kreuzchen"`, `width: 1000`, `height: 800`
    - `app.security.csp` set to the Global Constraints value
  - Add the plugins `tauri-plugin-dialog`, `tauri-plugin-opener` and `tauri-plugin-clipboard-manager` (Rust and npm).
  - `capabilities/default.json`: `core:default`, `dialog:allow-open`, `dialog:allow-save`, `dialog:allow-ask`, `clipboard-manager:allow-write-text`, and `opener:allow-open-path` scoped to `$APPDATA/**`.
  - Add `kreuzchen-engine = { path = "../crates/engine" }` to `src-tauri/Cargo.toml`.
  - npm dev dependencies: `vitest`, `@fontsource/atkinson-hyperlegible`, `@fontsource/ibm-plex-mono`.
  - Scripts: `"test": "vitest run"`, `"check": "svelte-check"`.

- [ ] **Step 3: Write `src/app.css` with these tokens.**
  - The design direction comes from the mockups: calm, large click targets, light and dark themes via `prefers-color-scheme`.
  - Light: `--bg #eef1f2; --surface #ffffff; --fg #1d2a2f; --muted #5f6f75; --line #d3dbde; --accent #1f6f78; --accent-fg #ffffff; --accent-soft #dcefef; --over #b4531a; --over-soft #fbe9dc`.
  - Dark: `--bg #12191c; --surface #1b2428; --fg #e4ecee; --muted #93a4aa; --line #2d3a3f; --accent #5cbcc6; --accent-fg #0c1a1d; --accent-soft #1f3a3e; --over #f0975c; --over-soft #3a2a1e`.
  - Body font: `"Atkinson Hyperlegible", system-ui, sans-serif`. Numbers: `"IBM Plex Mono", ui-monospace, monospace` with `tabular-nums`.
  - `body:not(.tech) .tech { display: none }`.
  - Replace `App.svelte` with a placeholder heading `Kreuzchen`.

- [ ] **Step 4: Verify.** Run `cargo build --workspace`, `npm run check` and `npm test` (0 tests is fine; add `passWithNoTests`). Run `npm run tauri dev`. Expected: a window titled "Kreuzchen" opens, and `<app_data_dir>/definitionen` and `auswertungen` exist on disk (on Linux, `~/.local/share/io.github.w3d3.kreuzchen/`).

- [ ] **Step 5: Commit:** `feat(app): Tauri scaffold with locked-down config`.

---

### Task 8: Definition files and grading commands

**Files:**
- Create: `src-tauri/src/definitions.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)
- Test: unit tests in `definitions.rs` using `tempfile::tempdir()`

**Interfaces:**
- Consumes: `kreuzchen_engine::{load, Questionnaire, Definition, Input, Grading}`; `Paths` (Task 7).
- Produces (`Serialize` structs):
  - `pub struct Summary { pub id: String, pub title: String, pub version: String, pub item_count: usize, pub scale_count: usize }`
  - `pub struct DefinitionEntry { pub file: String, pub hash: String, pub summary: Option<Summary>, pub error: Option<String> }`. `error` holds every `LoadError` `Display` joined with `"\n"`; a non-UTF-8 file gives `Datei ist nicht UTF-8`.
  - `pub fn scan(dir: &Path) -> Vec<DefinitionEntry>`. It lists only regular files ending in `.yaml`/`.yml` (case-insensitive). Valid entries come first, sorted by title; broken ones follow, sorted by file name.
  - `pub fn open(dir: &Path, file: &str) -> Result<(Questionnaire, String /* hash */), String>`. It rejects a `file` containing `/`, `\` or `..` with `Ungültiger Dateiname`.
  - `pub fn hash(bytes: &[u8]) -> String` (SHA-256, lowercase hex).
  - Commands:
    - `list_definitions() -> Vec<DefinitionEntry>`
    - `get_definition(file: String) -> Result<OpenedDefinition, String>`, where `OpenedDefinition { file: String, hash: String, definition: Definition }`
    - `grade(file: String, hash: String, input: Input) -> Result<Graded, String>`, where `Graded { grading: Grading, text: String }`. If the current hash ≠ `hash`, it returns `Err("Der Fragebogen wurde geändert. Bitte neu starten.")`. A `GradeError` is returned as its `Display` text.
    - `data_dir() -> String` (the definitions path, for the tech view).
    - `open_definitions_folder() -> Result<(), String>`, using `tauri_plugin_opener::open_path`.

- [ ] **Step 1: Write the failing tests:**
  - `scan_filters_and_sorts`: a temp dir holds:
    - `beispiel.yaml` (the fixture)
    - `kaputt.yml` (the fixture with `scale: Y, key: normal` replaced by `scale: Z, key: normal`)
    - `README.txt`, `.DS_Store`, `mbas.yaml.bak`, and a subdirectory `x.yaml/`

    Expect 2 entries: `beispiel.yaml` with a summary (`item_count` 4), then `kaputt.yml` with `summary` `None` and an `error` containing `Item 4: Skala „Z“`.
  - `bom_crlf_file_scans`: a fixture written with a BOM and CRLF has a summary.
  - `open_rejects_traversal`: `open(dir, "../x.yaml")` is `Err("Ungültiger Dateiname")`.
  - `hash_changes_with_content`: different bytes give different hashes; the same bytes give the same hash.
  - `stale_hash_detected`: test a pure helper `fn check_hash(current: &str, expected: &str) -> Result<(), String>` that returns the exact "geändert" message on mismatch. The `grade` command uses it.

- [ ] **Step 2: Run `cargo test -p kreuzchen definitions`** (the package name from the template). Expected: compile failure.

- [ ] **Step 3: Implement `definitions.rs` and `commands.rs`, and register the commands.** Add `sha2` to `src-tauri` and `tempfile` as a dev dependency.

- [ ] **Step 4: Run `cargo test --workspace`.** Expected: all pass.

- [ ] **Step 5: Commit:** `feat(app): definition scanning and grading commands`.

---

### Task 9: Saved results store

**Files:**
- Create: `src-tauri/src/store.rs`
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
- Test: unit tests in `store.rs` with `tempdir`

**Interfaces:**
- Consumes: `Input`, `Grading` (Task 4); `scan`, `DefinitionEntry` (Task 8).
- Produces (`Serialize, Deserialize`):
  - `pub struct QRef { pub id: String, pub title: String, pub version: String, pub hash: String }`
  - `pub struct SavedRecord { pub format: u32 /* 1 */, pub kennung: String, pub created_at: String /* local ISO "2026-11-11T22:23:05" from the UI */, pub questionnaire: QRef, pub input: Input, pub grading: Grading, pub text: String /* copy text at save time */ }`
  - `pub struct SavedSummary { pub name: String, pub kennung: String, pub title: String, pub created_at: String }`
  - Functions:
    - `pub fn save(dir: &Path, rec: &SavedRecord) -> Result<String, String>`. The file name is `{created_at with every char outside [0-9A-Za-z-] replaced by '-'}_{id}.json`. On collision it appends `-2`, `-3`, … before `.json`. It writes to `{name}.tmp`, then renames.
    - `pub fn list(dir: &Path) -> Vec<SavedSummary>`. Newest `created_at` first; files that don't parse are skipped.
    - `pub fn load(dir: &Path, name: &str) -> Result<SavedRecord, String>`
    - `pub fn delete(dir: &Path, name: &str) -> Result<(), String>`
    - `pub fn export(dir: &Path, name: &str, dest: &Path) -> Result<(), String>`
    - `pub fn import(dir: &Path, src: &Path) -> Result<String, String>`. It parses and requires `format == 1`, then calls `save`. Otherwise it returns `Err("Diese Datei ist keine gültige Kreuzchen-Auswertung.")`.
  - Every `name` argument must end in `.json` and contain no `/`, `\` or `..`; otherwise `Err("Ungültiger Dateiname")`.
  - Commands, wrapping the functions with `Paths.results`: `save_result(record)`, `list_results()`, `open_result(name) -> Result<OpenedResult, String>`, `delete_result(name)`, `export_result(name, dest)`, `import_result(src)`.
    - `OpenedResult { record: SavedRecord, current: Option<DefinitionEntry>, outdated: bool }`.
    - `current` is the scanned entry whose `summary.id == record.questionnaire.id`.
    - `outdated` is `current.hash != record.questionnaire.hash`, and `false` when `current` is `None`.

- [ ] **Step 1: Write the failing tests:**
  - `save_load_roundtrip`: the record loaded back equals the saved one.
  - `name_from_timestamp_not_kennung`: with kennung `"A/B: *x"` and created_at `"2026-11-11T22:23:05"`, `save` returns `"2026-11-11T22-23-05_beispiel.json"`.
  - `collision_suffix`: saving the same record 3× returns `…_beispiel.json`, `…_beispiel-2.json`, `…_beispiel-3.json`.
  - `list_newest_first_skips_garbage`: two records plus a `junk.json` holding `{}` → 2 summaries, newest first.
  - `import_rejects_invalid`: a file with `{"format":2}` → the exact "keine gültige" message; a valid export re-imports under a new name.
  - `rejects_bad_names`: `load(dir, "../a.json")` and `delete(dir, "a.txt")` both return `Err("Ungültiger Dateiname")`.

- [ ] **Step 2: Run `cargo test -p kreuzchen store`.** Expected: compile failure.

- [ ] **Step 3: Implement `store.rs` and the commands.**

- [ ] **Step 4: Run `cargo test --workspace`.** Expected: all pass.

- [ ] **Step 5: Commit:** `feat(app): saved results`.

---

### Task 10: UI shell, types and questionnaire list

**Files:**
- Create: `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/tech.svelte.ts`, `src/screens/List.svelte`
- Modify: `src/App.svelte`
- Test: `src/lib/tech.test.ts`

**Interfaces:**
- Consumes: the commands from Tasks 8–9.
- Produces:
  - `types.ts`: TypeScript mirrors of `Definition` and its children, `Input`, `Grading`, `ResultRow` (a union on `kind`), `RuleOutcome`, `DefinitionEntry`, `Summary`, `OpenedDefinition`, `Graded`, `SavedRecord`, `SavedSummary` and `OpenedResult`. `FieldValue = boolean | number | string`.
  - `api.ts`: one async function per command, with the same names in camelCase (`listDefinitions`, `getDefinition`, `grade`, `dataDir`, `openDefinitionsFolder`, `saveResult`, `listResults`, `openResult`, `deleteResult`, `exportResult`, `importResult`), each calling `invoke`.
  - `tech.svelte.ts`:
    - pure `techReducer(s: { held: boolean; toggled: boolean }, ev: { type: 'down' | 'up'; key: string; ctrl: boolean; shift: boolean } | { type: 'blur' }) => { held: boolean; toggled: boolean }`
    - `isTech(s) = s.held || s.toggled`
    - `installTech()`, which wires the window `keydown`/`keyup`/`blur` events and keeps `document.body.classList` `tech` in sync
  - `App.svelte` screen state: `type Screen = { name: 'list' } | { name: 'entry'; opened: OpenedDefinition; input: Input } | { name: 'result'; opened: OpenedDefinition | null; record: SavedRecord | null; input: Input; graded: Graded; notice: string | null } | { name: 'saved' }`.

- [ ] **Step 1: Write the failing tests in `tech.test.ts`:**
  - Alt down → `held` true; Alt up → `held` false.
  - Ctrl+Shift+D down flips `toggled` (pressing D with the key `"D"` or `"d"` both work).
  - `blur` → `held` false, and `toggled` stays.
  - Other keys leave the state unchanged.

- [ ] **Step 2: Run `npm test`.** Expected: FAIL.

- [ ] **Step 3: Implement `types.ts`, `api.ts`, `tech.svelte.ts`, `List.svelte` and the `App.svelte` shell.**
  - **Top navigation:** "Fragebögen" and "Gespeicherte Auswertungen".
  - **`List.svelte`:**
    - One card per valid entry: the title in bold; underneath, `{item_count} Fragen · {scale_count} Skalen · Version {version}`, plus `<span class="tech"> · {file}</span>`; a "Starten" button that calls `getDefinition` and switches to `entry` with the initial input (Task 11's `initialInput`).
    - A broken entry shows the bold title `Unbekannter Fragebogen` and the text `Dieser Fragebogen kann nicht geöffnet werden. Bitte wenden Sie sich an den Support.` The `error` follows in a `<pre class="tech">`. "Starten" is disabled.
    - With no entries at all: `Noch keine Fragebögen installiert. Bitte wenden Sie sich an den Support.`
    - Tech-only footer: `Datenordner: {dataDir()}` with the buttons "Ordner öffnen" and "Neu laden".
  - Call `installTech()` once in `App.svelte`.

- [ ] **Step 4: Run `npm test`, then `npm run check`.** Expected: pass. Then run `npm run tauri dev` with `beispiel.yaml` and a broken copy in the data folder. Expected: one card and one broken card; holding Alt reveals the file names, the error and the folder.

- [ ] **Step 5: Commit:** `feat(ui): shell and questionnaire list`.

---

### Task 11: Answer entry with keyboard

**Files:**
- Create: `src/lib/kennung.ts`, `src/lib/input.ts`, `src/lib/keyboard.ts`, `src/screens/Entry.svelte`
- Test: `src/lib/kennung.test.ts`, `src/lib/input.test.ts`, `src/lib/keyboard.test.ts`

**Interfaces:**
- Consumes: `types.ts`, `api.grade` (Task 10).
- Produces:
  - `defaultKennung(d: Date): string` returns `Anonym DD.MM.YYYY HH:MM`.
  - `initialInput(def: Definition, now: Date): Input` sets each field's `default`, with `"anonym"` turned into `defaultKennung(now)`. Fields without a default are omitted. `answers` is `{}`.
  - `entryItems(def: Definition): { key: string; text: string; options: string[]; section: number }[]` lists items in document order. `options` comes from the answer set: the item's `answers` or `default_answers`.
  - `type EntryState = { focused: number; count: number; options: number; current: number | null }`
  - `type EntryAction = { type: 'answer'; index: number } | { type: 'clear' } | { type: 'focus'; to: number } | { type: 'grade' } | { type: 'none' }`
  - `entryKey(k: { key: string; shift: boolean; ctrl: boolean }, s: EntryState): EntryAction`

Key map (also the bar copy):
- `1`…`9` where n ≤ `options` → `answer` index n−1. The component sets the answer, then focuses the next item.
- `ArrowRight` / `ArrowLeft` → `answer` current±1, clamped. With `current` null, Right gives 0 and Left gives `options−1`. Focus stays.
- `Enter`, `Tab` or `ArrowDown` → `focus` focused+1, clamped to count−1.
- `Shift+Tab` or `ArrowUp` → `focus` focused−1, clamped to 0.
- `Delete` / `Backspace` → `clear`.
- `Ctrl+Enter` → `grade`.
- Anything else → `none`, and the event is not prevented.

- [ ] **Step 1: Write the failing tests:**
  - `defaultKennung(new Date(2026, 10, 11, 22, 23))` is `"Anonym 11.11.2026 22:23"`; `new Date(2026, 0, 5, 7, 4)` gives `"Anonym 05.01.2026 07:04"`.
  - `initialInput(beispielDef, new Date(2026, 10, 11, 22, 23))` is `{ fields: { kennung: "Anonym 11.11.2026 22:23" }, answers: {} }`. Use a hand-built `Definition` object in the test that mirrors the fixture's `fields`.
  - `entryItems` on the same object gives keys `["1","2","3","4","zusatz"]`, and the options of `zusatz` are `["Nein","Ja"]`.
  - `entryKey`, with `s = { focused: 2, count: 5, options: 4, current: null }`:
    - `"3"` → `{type:'answer', index:2}`
    - `"5"` → `none`
    - `ArrowRight` → `answer 0`
    - `ArrowLeft` → `answer 3`
    - `Enter` → `focus 3`
    - `Tab` with shift → `focus 1`
    - `ArrowUp` → `focus 1`
    - `Delete` → `clear`
    - `Enter` with ctrl → `grade`
    - `"a"` → `none`
    - with `focused: 4`, `Enter` → `focus 4`

- [ ] **Step 2: Run `npm test`.** Expected: FAIL.

- [ ] **Step 3: Implement the three lib files and `Entry.svelte`:**
  - **Header fields:** text, number and date inputs, and checkboxes, by `kind`. The Kennung label reads `Kennung (keine Patientennamen)`.
  - **Sticky progress bar:** `{answered} / {item_count} beantwortet`. It counts only items with a scale, matching `fehlend`. It also holds the button "Auswerten".
  - **Sections:** each section title, then the `intro` in muted text.
  - **Each item:** its number or ID, the text, and a row of radio buttons. Each radio has its label and a small key number (`1`…`n`).
  - **Focus:** the focused item gets a clear accent outline and is scrolled into view (`block: 'nearest'`). Clicking an item focuses it.
  - **Keyboard scope:** the handler is active only when focus isn't in a header input. Tab from the last header field focuses item 0.
  - **Marking skipped questions:** after "Auswerten" (or Ctrl+Enter), unanswered scored items get the left `--over` stripe. Grading still runs.
  - **Shortcut bar:** fixed to the bottom, always visible. Text: `1–{options} Antwort · ←/→ Antwort ändern · Enter/Tab nächste Frage · Umschalt+Tab zurück · Entf löschen · Strg+Enter Auswerten`. `{options}` follows the focused item.
  - **"Abbrechen":** asks `Eingaben verwerfen?` via the dialog plugin's `ask` when any answer exists, then goes to the list.
  - **Auswerten:** calls `api.grade(opened.file, opened.hash, input)` and switches to `result`. On error, it shows the message in a banner, e.g. the "geändert" text.

- [ ] **Step 4: Run `npm test` and `npm run check`.** Expected: pass. Then run `npm run tauri dev` and fill in `beispiel` using only the keyboard. Expected: the digits answer and advance, and the bar shows `1–4`, then `1–2` on Zusatzfrage.

- [ ] **Step 5: Commit:** `feat(ui): answer entry with keyboard shortcuts`.

---

### Task 12: Result screen

**Files:**
- Create: `src/screens/Result.svelte`

**Interfaces:**
- Consumes: the `result` screen state (Task 10); `api.saveResult`; `writeText` from `@tauri-apps/plugin-clipboard-manager`.

- [ ] **Step 1: Implement `Result.svelte`:**
  - **Header:** the Kennung in bold and the other fields muted.
  - **One row per `ResultRow`.**
    - `Sum`: the label, then `value` in mono, then ` / {cutoff}` when there is a cutoff, then a pill `über Cut-off` (`--over` colours) when `over`. Below that, a meter: fill width `min(value / (cutoff*2), 1)`, with the cutoff mark at 50%. That puts the cutoff in the same spot on every row, and the bar is a visual aid, not to scale.
    - `Answer`: the label and the answer, or `–`.
  - **Verdict box:**
    - each rule as `✓ {label}` or `✗ {label}`
    - `Fehlende Antworten: {missing}`, plus ` (Fragen …)` when there are any
    - the verdict in bold
    - the box uses `--over` styling when any rule is met, neutral otherwise
  - **Caveats:** each in a dashed box, prefixed `Hinweis:`.
  - **`notice`, if set,** shows as a banner at the top. Task 13 uses it.
  - **Buttons:**
    - "Zurück zu den Antworten" goes to `entry` with the same `input`. It is hidden when `opened` is null.
    - "Als Text kopieren" calls `writeText(graded.text)` and then shows `Kopiert.` (status text, `aria-live="polite"`).
    - "Speichern" builds a `SavedRecord`: `format 1`, `kennung` from `input.fields.kennung`, `created_at` as the local `YYYY-MM-DDTHH:MM:SS`, and `questionnaire` from `opened` (`id`, `title`, `version`, `hash`), `grading` and `text` from `graded`. It calls `saveResult` and shows `Gespeichert.`. It is hidden when viewing a saved record that wasn't re-graded.
    - "Neuer Fragebogen" goes to the list.

- [ ] **Step 2: Verify in `npm run tauri dev`.** Grade beispiel with the answers 1→4, 2→3, 3→1, 4→4 (keys 4, 3, 1, 4). Expected: the screen shows Gesamtsumme `7 / 7 über Cut-off` and verdict `Auffällig (Regel 2)`, matching Task 4's `exactly_at_cutoff`. Paste the copied text into an editor: it equals Task 5's expected block, apart from the Kennung and Alter lines. Save, then confirm the JSON file exists in `auswertungen/`.

- [ ] **Step 3: Run `npm run check`.** Expected: pass.

- [ ] **Step 4: Commit:** `feat(ui): result screen with copy and save`.

---

### Task 13: Saved results screen

**Files:**
- Create: `src/screens/Saved.svelte`

**Interfaces:**
- Consumes: `api.listResults`, `openResult`, `deleteResult`, `exportResult`, `importResult`, `getDefinition` and `grade`; `open` / `save` / `ask` from `@tauri-apps/plugin-dialog`.

- [ ] **Step 1: Implement `Saved.svelte`:**
  - **Table:** Kennung, Fragebogen (title), Datum (`created_at` shown as `DD.MM.YYYY HH:MM`), and actions "Öffnen", "Exportieren" and "Löschen". The file `name` shows as `.tech`.
  - **Empty state:** `Keine gespeicherten Auswertungen.`
  - **"Öffnen":** calls `openResult`, then goes to `result` with `record` and `graded = { grading: record.grading, text: record.text }`. "Als Text kopieren" copies the stored text.
    - If `outdated`: notice `Dieser Fragebogen wurde seit dem Speichern geändert. Angezeigt wird das gespeicherte Ergebnis.` plus a button "Neu auswerten". That button calls `getDefinition(current.file)`, then `grade` with `record.input`, and replaces the screen state with `opened` set, so it can be saved as a new record.
    - If `current` is null: notice `Dieser Fragebogen ist nicht mehr installiert. Angezeigt wird das gespeicherte Ergebnis.`
    - If `grade` fails during "Neu auswerten", show its message in the notice and keep the saved result.
  - **"Exportieren":** opens a `save` dialog with the default path `{name}`, then calls `exportResult`.
  - **"Importieren…":** opens an `open` dialog filtered to `json`, then calls `importResult` and refreshes the list. Errors show in a banner.
  - **"Löschen":** asks `Auswertung „{kennung}“ endgültig löschen?`, then calls `deleteResult` and refreshes.

- [ ] **Step 2: Verify in `npm run tauri dev`:**
  - Open the record saved in Task 12; it matches.
  - Edit `beispiel.yaml` (change `cutoff: 7` to `cutoff: 8`) and reopen. Expected: the "geändert" notice appears. "Neu auswerten" then shows `7 / 8` without the über pill.
  - Export, delete and re-import. Expected: the record is back.

- [ ] **Step 3: Run `npm run check`.** Expected: pass.

- [ ] **Step 4: Commit:** `feat(ui): saved results`.

---

### Task 14: CI, release and documentation

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `docs/installation.md`, `README.md`

- [ ] **Step 1: Write `ci.yml`.**
  - Runs on push and PR, on `ubuntu-22.04`.
  - Installs the Tauri Linux packages (as in Task 1), sets up Rust stable and Node 24, then runs `npm ci`, `npm test`, `npm run check` and `cargo test --workspace`.
  - The MBAS tests skip themselves because `local/` is absent.

- [ ] **Step 2: Write `release.yml`.**
  - Triggers on tags `v*`.
  - Matrix: `ubuntu-22.04`, `windows-latest`, `macos-latest` (args `--target universal-apple-darwin`, with both Apple targets added via rustup).
  - Uses `tauri-apps/tauri-action@v0` with `tagName: ${{ github.ref_name }}` and `releaseDraft: true`.
  - No signing secrets.

- [ ] **Step 3: Write `docs/installation.md` (German) for clinic staff:**
  - Windows: run the `.msi`; on SmartScreen click "Weitere Informationen", then "Trotzdem ausführen".
  - macOS: open the `.dmg`, drag Kreuzchen into Programme, then right-click → "Öffnen", or System Settings → Datenschutz & Sicherheit → "Trotzdem öffnen".
  - Linux: the `.AppImage` (make it executable) or the `.deb`.
  - Adding questionnaires, written for the developer or support: where the data folder is on each OS (`%APPDATA%\io.github.w3d3.kreuzchen\definitionen`, `~/Library/Application Support/io.github.w3d3.kreuzchen/definitionen`, `~/.local/share/io.github.w3d3.kreuzchen/definitionen`), and that holding Alt in the app shows the path.
  - Screenshots get added after the first release is tried on real Windows and macOS machines. Mark those spots with `<!-- Screenshot: … -->` comments.

- [ ] **Step 4: Write `README.md` (English, for developers):**
  - what the app is and the dev commands (`npm run tauri dev`, `cargo test --workspace`, `npm test`)
  - the `local/` convention and why it is gitignored
  - a schema reference that points to `examples/definitionen/beispiel.yaml`, with one paragraph on each top-level key and the expression names (`X`, `X.cutoff`, `X.over`, field IDs, item IDs, rule IDs, `fehlend`)

- [ ] **Step 5: Verify.** Push the branch and confirm CI is green: `gh run watch`.

- [ ] **Step 6: Commit:** `chore: CI, release workflow and docs`.

---

### Task 15: End-to-end check on Linux

- [ ] **Step 1: Build.** Run `npm run tauri build`. Expected: an `.AppImage` and a `.deb` under `target/release/bundle/`.

- [ ] **Step 2: Install and run the built app.** Copy `examples/definitionen/beispiel.yaml` and `local/definitionen/mbas.yaml` into the data folder. Then:
  - Pick MBAS and enter the answers from `local/faelle/fall1.json` with the keyboard only.
  - Grade, and check the sums against the hand-scored sheet.
  - Copy, save, reopen from "Gespeicherte Auswertungen", then export, delete and import.

- [ ] **Step 3: Report.** Tell the user exactly what passed and what didn't. Windows and macOS checks need the user's machines and happen after the first draft release.

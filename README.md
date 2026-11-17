<img src="docs/logo.svg" alt="" width="96" height="96">

# Kreuzchen

Kreuzchen is a German-only desktop app (Tauri 2 + Svelte 5) that grades paper questionnaires. Questionnaires are defined in YAML; the app scores entered answers and shows a verdict. It makes no network requests at runtime. Builds are unsigned.

Install instructions for end users (German): [docs/installation.md](docs/installation.md).

## Development

Prerequisites: Rust stable, Node 24 and the [Tauri 2 Linux dependencies](https://v2.tauri.app/start/prerequisites/) on Linux. On NixOS (or with Nix installed), `nix develop` opens a shell with everything; or run commands as `nix develop -c <cmd>`.

```sh
npm ci
npm run tauri dev       # run the app
npm test                # frontend tests (vitest)
npm run check           # svelte-check
cargo test --workspace  # engine and backend tests
npm run tauri build     # installers under target/release/bundle/
```

Layout: `crates/engine` (YAML parsing and scoring), `src-tauri` (Tauri shell), `src` (Svelte UI).

## CI and releases

- Every push and PR runs the test job (`.github/workflows/ci.yml`).
- Every push to `main` additionally builds Windows, macOS (universal) and Linux bundles and uploads them as workflow artifacts.
- Pushing a `v*` tag publishes a GitHub release with all installers (`.github/workflows/release.yml`).

## The `local/` folder

`local/` is gitignored. It holds copyrighted material (e.g. the MBAS PDF and its `mbas.yaml`) that must never be committed or bundled. Tests that need it skip themselves when it is absent, which is the case in CI. Committed tests use only `examples/definitionen/beispiel.yaml`.

## Questionnaire schema

The reference example is [`examples/definitionen/beispiel.yaml`](examples/definitionen/beispiel.yaml). Top-level keys:

- `schema`, `id`, `version`, `title`: format version (currently `1`), a unique ID, the definition version and the display title.
- `cutoff_compare`: how scores are compared with cut-offs, `">="` or `">"`.
- `answer_sets`: named lists of answer labels, in order; the position is the answer value. `default_answers` names the set used by items that do not set `answers`.
- `shortcuts`: optional entry keys per answer set, one character per answer in order, e.g. `ja_nein: nj`. Sets without an entry use `0`, `1`, `2`, ….
- `keys`: named scoring tables. An item's `key` is a table name or an inline list mapping answer position to points (reverse-keyed items use e.g. `[3, 2, 1, 0]`).
- `fields`: patient/session inputs (`text`, `number`, `checkbox`) with an `id` usable in expressions.
- `scales`: scored groups with an `id`, `label` and optional `cutoff`. Items add their points to their `scale`.
- `derived`: computed values with an `id` and a `value` expression (e.g. `X + Y`), optionally with a `cutoff`.
- `sections`: the questionnaire pages, each with a `title`, optional `intro` and `items`. Items have a number `n` or an `id`, `text`, and optionally `scale`, `key` and `answers`.
- `result`: the IDs shown in the result table, in order.
- `rules`: named boolean expressions (`id`, `label`, `when`) that may reference earlier rules.
- `caveats`: notes shown when their `when` expression is true.
- `verdict`: ordered list; the first entry whose `when` is true gives the text; the entry with `else` is the fallback.

Expression names: `X` is the score of scale or derived value `X`; `X.cutoff` is its cut-off; `X.over` is true when it reaches the cut-off per `cutoff_compare`. Field IDs, item IDs and rule IDs are usable as variables, and `fehlend` is the number of unanswered items. Expressions support arithmetic, comparisons, `and`, `or` and `not`.

# Development

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

## Updates

Installed apps check `releases/latest/download/latest.json` at start and offer to install a newer version. `tauri-action` builds the signed update bundles and uploads `latest.json` with each release. To ship an update, bump the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, then push a new `v*` tag.

Update bundles are signed with a minisign key. The public key is in `src-tauri/tauri.conf.json`. The private key and its password are the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. If the private key is lost, installed apps can no longer update and need a manual reinstall with a new key. On Linux only the AppImage updates itself; `.deb` and `.rpm` installs need a reinstall.

## The `local/` folder

`local/` is gitignored. It holds copyrighted material (source PDFs and their definitions) that must never be committed or bundled. Commits, PRs, docs and code never name specific questionnaires; committed tests and examples use only the invented definitions in `examples/definitionen/`.

`crates/engine/tests/local.rs` checks that every `local/definitionen/*.yaml` loads, and grades the hand-scored cases in `local/faelle/<definition name>/*.json`:

```json
{ "input": { "fields": {}, "answers": { "1": 0, "2": 3 } }, "expect": { "gesamt": 42 }, "verdict": "…" }
```

`verdict` is optional. The test skips itself when `local/` is absent, which is the case in CI.

## Questionnaire format

The reference example is [`examples/definitionen/beispiel.yaml`](examples/definitionen/beispiel.yaml). Top-level keys:

- `schema`, `id`, `version`, `title`: format version (currently `1`), a unique ID, the definition version and the display title.
- `cutoff_compare`: optional, how scores are compared with cut-offs, `">="` (default) or `">"`.
- `answer_sets`: named lists of answer labels, in order; the position is the answer value. `default_answers` names the set used by items that do not set `answers`.
- `shortcuts`: optional entry keys per answer set, one character per answer in order, e.g. `ja_nein: nj`. Sets without an entry use `0`, `1`, `2`, ….
- `keys`: named scoring tables. An item's `key` is a table name or an inline list mapping answer position to points (reverse-keyed items use e.g. `[3, 2, 1, 0]`).
- `fields`: patient/session inputs (`text`, `number`, `checkbox`) with an `id` usable in expressions.
- `scales`: scored groups with an `id`, `label` and optional `cutoff`. Items add their points to their `scale`.
- `derived`: computed values with an `id` and a `value` expression (e.g. `X + Y`), optionally with a `cutoff`.
- `sections`: the questionnaire pages, each with a `title`, optional `intro` and `items`. Items have a number `n` or an `id`, `text`, and optionally `scale`, `key` and `answers`.
- `result`: the IDs shown in the result table, in order.
- `norms`: optional norm tables (e.g. T-values). `name` is the label (default `T`), `bands` the severity labels by lower bound (`{ from: 60, label: mild }`, ascending), and `tables` maps a scale or derived ID to `{ raw score: norm value }`. A raw score that is not listed takes the next lower listed one, as with a printed table that skips rows. Example: [`examples/definitionen/beispiel_normen.yaml`](examples/definitionen/beispiel_normen.yaml).
- `rules`: named boolean expressions (`id`, `label`, `when`) that may reference earlier rules.
- `caveats`: notes shown when their `when` expression is true.
- `verdict`: ordered list; the first entry whose `when` is true gives the text; the entry with `else` is the fallback.

Expression names: `X` is the score of scale or derived value `X`; `X.cutoff` is its cut-off; `X.over` is true when it reaches the cut-off per `cutoff_compare`. Field IDs, item IDs and rule IDs are usable as variables, `fehlend` is the number of unanswered items, and `X.fehlend` the unanswered items of scale `X`. Expressions support `+` and `/`, comparisons, `and`, `or` and `not`. Derived values may have decimals (e.g. a mean `K / 9`); results show at most two, as `3,67`.

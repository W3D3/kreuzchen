# Development

Prerequisites: Rust stable, Node 24 and the [Tauri 2 Linux dependencies](https://v2.tauri.app/start/prerequisites/) on Linux. On NixOS (or with Nix installed), `nix develop` opens a shell with everything, including the GSettings schemas the GTK file dialogs need; or run commands as `nix develop -c <cmd>`.

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
- `min_app`: optional, the oldest Kreuzchen version that can open the file, e.g. `"0.4.0"`. Set it when the file uses a newer feature: older versions then show „Update nötig“ instead of an error. Versions before 0.4.0 do not know this key.
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

Expression names: `X` is the score of scale or derived value `X`; `X.cutoff` is its cut-off; `X.over` is true when it reaches the cut-off per `cutoff_compare`. Field IDs, item IDs and rule IDs are usable as variables, `fehlend` is the number of unanswered items, and `X.fehlend` the unanswered items of scale `X`. A number field `F` that may stay empty is checked with `F.gesetzt`, e.g. `alter.gesetzt and alter > 24`: `and`/`or` stop once the left side decides. Expressions support `+` and `/`, comparisons, `and`, `or` and `not`. Derived values may have decimals (e.g. a mean `K / 9`); results show at most two, as `3,67`.

## Registry server

`crates/registry` (`kreuzchen-registry`) serves questionnaire definitions to practices over HTTP: `GET /healthz`, `GET /v1/fragebogen` and `GET /v1/fragebogen/{id}` (HTTP Basic auth, read-only). With an admin token set, it also serves an admin page and API under `/admin` (see [Admin](#admin)).

The app never shows the word "registry". Users see the server as the **Bibliotheksserver** and its questionnaires as the **Bibliothek**. Internal names (`registry/`, `registry.json`, command names, the binary and image) stay as they are.

Layout of the data directory (`/data` in the container, `--data DIR` otherwise):

```
config.yaml          # practices: user, key_hash, allowed fragebogen ids
definitionen/*.yaml  # questionnaire definitions, one file each
```

A sample is in [`crates/registry/example/`](crates/registry/example/). Its `config.yaml` ships with a placeholder `key_hash` that the server rejects ("ungültiger key_hash") until you replace it with one from `kreuzchen-registry key`. Run it with `kreuzchen-registry serve [--data DIR] [--port N]` (defaults `/data` and `8080`, bound to `0.0.0.0`). The server exits non-zero if the config or any definition is invalid. Each request is logged to stderr as `user path status`; keys are never logged.

Add a practice:

1. `kreuzchen-registry key` prints `key:` (give it to the practice, it is shown once) and `key_hash:` (argon2id).
2. Add an entry to `praxen` in `config.yaml` with a unique `user`, the `key_hash` and the `fragebogen` ids the practice may fetch.
3. Put new definitions in `definitionen/` if needed.
4. Restart the server; hand edits on disk are read only at startup. Changes made through `/admin` apply at once.

### Admin

Set `KREUZCHEN_ADMIN_TOKEN` to a long random secret (`openssl rand -base64 32`) to enable it. Unset or empty, every `/admin` route answers 404 and the server stays read-only, as before. The startup line says `admin on` or `admin off`.

`/admin` is a German page built into the binary. It asks for the token, keeps it in `sessionStorage` (gone when the tab closes), lists all questionnaires with the practices that see them, and can upload, replace and delete questionnaires and tick which practices see each one. It cannot create practices or change keys; that stays a `config.yaml` edit plus restart.

API (`Authorization: Bearer <token>`; practice Basic credentials are rejected here, and the token is rejected on `/v1`):

| Request | Effect |
|---|---|
| `GET /admin/api/fragebogen` | `{"praxen": [user…], "fragebogen": [{id, title, version, hash, praxen}…]}` |
| `POST /admin/api/fragebogen` (YAML body) | Adds `definitionen/<id>.yaml`. `409` if the id exists. |
| `PUT /admin/api/fragebogen/{id}` (YAML body) | Replaces the file; the previous version goes to `<file>.bak`. The id in the file must match. |
| `DELETE /admin/api/fragebogen/{id}` | Removes it from every practice in `config.yaml`, then renames the file to `<file>.bak`. |
| `PUT /admin/api/fragebogen/{id}/praxen` (JSON `["user"…]`) | Sets exactly these practices as the ones that see it. |

Uploads are capped at 1 MiB (`413`) and checked exactly like files at startup (engine parse and validation, safe id); on any error the response is `400` with the message and nothing is written. Each write goes to a `.tmp` file that is then renamed over the target, admin requests run one at a time, and after each write the server reloads the data directory, so practices see the change on their next list or download. Only one backup per file is kept; the next replace or delete overwrites it.

When the admin rewrites `config.yaml`, all practices, key hashes and lists are kept, but **comments and formatting in `config.yaml` are lost**. Keep notes elsewhere.

The data directory must be writable by the server (uid `65532` in the image): `sudo chown -R 65532:65532 <dir>`. A fresh named volume already is. Mounted `:ro`, the practice API works as before and admin writes fail with an error.

The token is a bearer secret: serve `/admin` only over HTTPS (reverse proxy or tunnel), never plain HTTP over the internet. Consider restricting `/admin` at the proxy to your own IPs.

Published image: `ghcr.io/w3d3/kreuzchen-registry` (amd64). `.github/workflows/registry.yml` builds the image for every change to the engine or the registry and publishes it when a `registry-v*` tag is pushed, e.g. `registry-v0.1.0` → tags `0.1.0`, `0.1` and `latest`. The registry is versioned separately from the app. The image contains only the server, never definitions or keys.

Container: `docker build -f crates/registry/Dockerfile -t kreuzchen-registry .` builds a static musl binary into a distroless, non-root image (`EXPOSE 8080`, `VOLUME /data`). [`docker-compose.example.yml`](crates/registry/docker-compose.example.yml) mounts the data directory writable and passes `KREUZCHEN_ADMIN_TOKEN` from the environment or an `.env` file. `docker compose restart` applies hand edits to the data.

The server speaks plain HTTP. Run it behind a Cloudflare tunnel or another HTTPS reverse proxy (required when the admin token is set) and add a rate-limit rule on `/v1/` and `/admin/api/` to throttle guessing.

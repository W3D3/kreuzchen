# Questionnaire registry

Status: draft for approval, 2026-11-17.

## Intent

Practices should get their questionnaire definitions from a server instead of copying YAML files by hand, and keep them current with a click. Many definitions transcribe copyrighted material, so the server must not reveal anything to anyone without valid credentials: not the files, not their names, not even which practices exist.

The operator (one person, comfortable with YAML and git) runs the server as a container at home and manages it by editing a config file. There is no admin UI.

## Goals

- A headless server that serves each practice exactly the definitions it is entitled to, behind username + key.
- In the app: a registry connection (URL, username, key), a screen that lists what the practice may install, and manual install and update.
- Definitions installed from the registry cannot be edited in the app.
- Nothing changes on a practice computer without a click.

## Non-goals

- No admin UI. The operator edits `config.yaml` and the definitions folder.
- No automatic install or update of definitions.
- No revoking: a definition a practice has installed stays usable even if the server stops offering it, the practice disconnects, or it switches accounts. Only the practice removes it, by clicking.
- No encryption beyond HTTPS. Files on the practice computer are readable by anyone with access to that computer, as today.
- One registry per app installation. Several registries are out of scope.

## Anti-goals (a failure even if it works)

- Anyone without valid credentials learns that a definition, or a practice, exists. That includes different responses for "unknown user" and "wrong key", and for "not entitled" and "does not exist".
- A definition on a practice computer changes without the user clicking "Installieren" or "Aktualisieren".
- A broken definition reaches a practice.
- Saved results stop opening because a registry definition changed or disappeared.

## Constraints

- HTTPS terminates at Cloudflare (`cloudflared` tunnel to `kreuzchen.wedenig.dev`). The container speaks plain HTTP on one port inside the home network.
- The repository is public. Code, tests, docs, commits and PRs never name specific questionnaires or customers. Tests use invented definitions only.
- The app's CSP has no `connect-src` for remote hosts. Registry requests go through the Rust backend, not the webview.
- Definitions use the existing format and engine (`crates/engine`). The fingerprint is the existing `definitions::hash`: SHA-256 of the file bytes, lowercase hex.

## Approaches considered

1. **Rust server in the workspace (chosen).** A new crate `crates/registry` with a small HTTP server (axum) and a key-generation command. It validates every definition with the engine at startup. The release is one small container image. Chosen because it is the only option that enforces per-practice entitlement in tested code and reuses the validation the app already trusts.
2. Static files behind a web server with per-practice basic auth or Cloudflare Access. No code, but per-practice entitlement becomes hand-maintained proxy rules or folders, with no validation. A mistake exposes definitions to the wrong practice.
3. TypeScript server. Same shape as 1, but cannot reuse the Rust engine, so validation is skipped or duplicated.

## Server

### Files

```
/data/config.yaml
/data/definitionen/<id>.yaml
```

The container reads `/data` (a mounted volume). Definitions are looked up by their `id` field, not by file name.

```yaml
praxen:
  - user: praxis-mueller
    key_hash: "$argon2id$v=19$m=19456,t=2,p=1$…"
    fragebogen: [fragebogen_a, fragebogen_b]
```

### Startup

The server loads every `definitionen/*.yaml` with `kreuzchen_engine::load`. It refuses to start, logging the reason, when any of these is true:
- a definition fails to load;
- two files share an `id`;
- `config.yaml` names an unknown definition id;
- two practices share a username;
- a key hash does not parse.

Config changes take effect on restart (`docker restart`). No hot reload.

### API

Every request except `/healthz` needs HTTP Basic auth (`Authorization: Basic base64(user:key)`). Missing, unknown user and wrong key all get the same `401` with no body. To keep timing equal, an unknown user is verified against a fixed dummy hash.

| Method and path | Response |
|---|---|
| `GET /healthz` | `200`, empty body. No version, no counts. |
| `GET /v1/fragebogen` | `200`, JSON array of the practice's entitled definitions: `{ "id", "title", "version", "hash" }`. |
| `GET /v1/fragebogen/{id}` | `200`, the raw YAML bytes, `Content-Type: application/yaml`, with header `X-Kreuzchen-Hash: <hash>`. `404` with no body if the id does not exist or is not entitled (indistinguishable). |

Responses carry `Cache-Control: no-store`. Request logs record the username, path and status, never the key.

### Keys

`kreuzchen-registry key` prints a new random key (32 bytes, base64url, no padding) and its argon2id hash, ready to paste into `config.yaml`. The key is shown once; only the hash is stored.

Brute force: argon2 makes each guess slow on the server. Rate limiting is left to Cloudflare (a WAF rule on `/v1/`), set up by the operator.

### Container

A multi-stage `Dockerfile` in `crates/registry/` builds a static binary into a minimal base image (distroless or scratch), non-root user, port `8080`, volume `/data`. A `docker-compose.example.yml` shows the volume and port. The image contains no definitions.

## App

### Connecting

The registry is a normal, user-visible feature (decided): installing and updating questionnaires is everyday practice work, and nothing updates on its own, so staff must see it.

- The start screen ("Fragebögen") gets a button **„Fragebögen aus der Registry“** next to the list. It opens the registry screen. It is always visible; without a connection the screen shows the connection form first.
- The registry screen has a connection section with fields for URL, username and key, plus **Verbinden** (tests the connection by fetching the list) and **Trennen** (asks for confirmation). Once connected, the section collapses to one line ("Registry: user @ host, verbunden") with **Ändern**.

The connection is stored in the app data folder as `registry.json` (URL, username, key). The key field is masked after saving and never shown again in plain text; **Ändern** requires entering it anew.

### Installed definitions

Registry definitions are installed into a separate folder, `registry/`, next to `definitionen/` in the app data folder, as `<id>.yaml`. Local and registry definitions are listed together on the start screen. Registry entries carry a small "Registry" label.

The editor ("Fragebögen bearbeiten") stays in the technical area as today.

`DefinitionEntry` gains `locked: bool`, true for files in `registry/`. Locked entries:
- cannot be opened in the editor (**Bearbeiten** disabled), saved or deleted from "Fragebögen bearbeiten";
- can be duplicated, which creates an editable local copy in `definitionen/` with a fresh id.

Backend commands that take a file name gain a way to address the `registry/` folder. Locked files are refused by the save and delete commands; they are removed only through the registry screen.

### Unique ids across both folders

An id exists at most once across `definitionen/` and `registry/` (decided):
- **Install over a local copy:** if `definitionen/` already holds a definition with the same id (typical: a practice copied it by hand before), **Installieren** asks „Lokale Kopie durch die Version aus der Registry ersetzen?“. On yes, the local file is deleted and the registry file installed; on no, nothing changes.
- **Duplicate** picks an id unused in both folders.
- **Editor save** refuses a local definition whose id is installed from the registry, with a message naming the conflict.
- If both folders still hold the same id (e.g. a file copied in by hand after install), both are listed and the registry one is used for saved results (see below).

### Registry screen

```
Fragebögen aus der Registry                    [Nach Änderungen suchen]
Registry: praxis-mueller @ kreuzchen.wedenig.dev               verbunden

  Fragebogen A    installiert · neue Version verfügbar   [Aktualisieren]
  Fragebogen B    installiert · aktuell
  Fragebogen C    verfügbar                               [Installieren]
  Fragebogen D    nicht mehr auf dem Server               [Löschen…]
```

States, by comparing the server list with `registry/`:
- **verfügbar:** on the server, not installed. **Installieren** downloads it.
- **aktuell:** installed, and the hash matches.
- **neue Version verfügbar:** installed, but the hash differs. **Aktualisieren** replaces the file.
- **nicht mehr auf dem Server:** installed, but not in the server list (also after switching to an account that lacks it).

Every installed registry definition has **Entfernen…** in every state, also when offline or disconnected. It asks for confirmation, then deletes the file. Nothing is removed without that click.

**Trennen** removes only the connection (`registry.json`). Installed registry definitions stay usable and locked, and stay listed on the registry screen with **Entfernen…**.

Before writing a downloaded file, the app checks that its SHA-256 matches `X-Kreuzchen-Hash` and the list entry, and that it loads with the engine. Otherwise it keeps the old file and shows an error.

The app fetches the list only when this screen opens or **Nach Änderungen suchen** is clicked. There is no background polling. When offline, it shows "Registry nicht erreichbar" and lists the installed definitions with **Entfernen…**, without update states.

### Saved results

`open_result` searches both `registry/` and `definitionen/` for the record's id. A result whose definition was updated shows the existing "Fragebogen wurde geändert" notice and re-score offer. A result whose definition was deleted shows the existing "nicht mehr installiert" notice. If both folders hold the same id, the one whose hash matches the record wins, then the registry one.

### README

The network sentence in `README.md` changes: besides the update check, the app contacts a registry only when one is configured, and only when the user opens the registry screen.

## Left to the builder

- HTTP client in the app: reuse whatever the updater plugin already pulls in (`reqwest` with rustls is in `Cargo.lock`), or add it explicitly.
- argon2 parameters: the crate's defaults (OWASP minimum) are fine.
- Exact layout of the registry screen, within the sketch above and the existing styles.
- Whether CI also publishes the image to GHCR. A local `docker build` is enough at first.
- Tests: server tests with an in-memory router and invented definitions (entitled list, 404 for not entitled, identical 401s, startup refusal cases). App tests for the four registry states, install over a local copy, the cross-folder id checks in duplicate and save, and the locked checks in save and delete.
- Ids become file names (`registry/<id>.yaml`): check them with the existing name rules before writing.

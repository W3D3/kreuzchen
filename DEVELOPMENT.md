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

## The `local/` folder

`local/` is gitignored. It holds copyrighted material (e.g. the MBAS PDF and its `mbas.yaml`) that must never be committed or bundled. Tests that need it skip themselves when it is absent, which is the case in CI. Committed tests use only `examples/definitionen/beispiel.yaml`.

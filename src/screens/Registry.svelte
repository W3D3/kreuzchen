<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { registryConnect, registryConnection, registryDisconnect, registryInstall, registryRemove, registryRows } from "../lib/api";
  import { rowView } from "../lib/registry";
  import type { ConnectionInfo, Row, RowsResult } from "../lib/types";

  let conn = $state<ConnectionInfo | null>(null);
  let result = $state<RowsResult | null>(null);
  let editing = $state(false);
  let form = $state({ url: "", user: "", key: "" });
  let banner = $state<string | null>(null);
  let busy = $state(false);

  async function run(fn: () => Promise<unknown>) {
    banner = null;
    busy = true;
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    } finally {
      busy = false;
    }
  }
  const load = () => run(async () => ([conn, result] = await Promise.all([registryConnection(), registryRows()])));
  load();

  // The backend stores a normalized http(s) URL, so parsing cannot fail.
  const host = (url: string) => new URL(url).host;

  const change = () => {
    form = { url: "", user: "", key: "" };
    editing = true;
  };

  const connect = (e: SubmitEvent) => {
    e.preventDefault();
    return run(async () => {
      await registryConnect(form.url, form.user, form.key);
      form.key = "";
      editing = false;
      [conn, result] = await Promise.all([registryConnection(), registryRows()]);
    });
  };

  const disconnect = () =>
    run(async () => {
      if (!(await ask("Verbindung zur Registry trennen? Installierte Fragebögen bleiben erhalten.", { kind: "warning" }))) return;
      await registryDisconnect();
      conn = null;
      result = await registryRows();
    });

  const install = (r: Row) =>
    run(async () => {
      if ((await registryInstall(r.id, false)) === "needs_replace_confirm") {
        if (!(await ask("Lokale Kopie durch die Version aus der Registry ersetzen?", { kind: "warning" }))) return;
        await registryInstall(r.id, true);
      }
      result = await registryRows();
    });

  const remove = (r: Row) =>
    run(async () => {
      if (!(await ask(`Fragebogen „${r.title}“ entfernen? Gespeicherte Auswertungen bleiben erhalten.`, { kind: "warning" }))) return;
      await registryRemove(r.file!);
      result = await registryRows();
    });
</script>

<div class="head">
  <h1>Fragebögen aus der Registry</h1>
  {#if conn}<button onclick={load} disabled={busy}>Nach Änderungen suchen</button>{/if}
</div>
{#if banner}<p class="banner" role="alert">{banner}</p>{/if}

{#if conn && !editing}
  <div class="conn">
    <span>Registry: {conn.user} @ {host(conn.url)}, verbunden</span>
    <span class="actions">
      <button onclick={change} disabled={busy}>Ändern</button>
      <button onclick={disconnect} disabled={busy}>Trennen</button>
    </span>
  </div>
{:else}
  <form class="card form" onsubmit={connect}>
    <label>URL <input bind:value={form.url} required placeholder="registry.example.org" /></label>
    <label>Benutzername <input bind:value={form.user} required autocomplete="username" /></label>
    <label>Schlüssel <input type="password" bind:value={form.key} required autocomplete="current-password" /></label>
    <div class="actions">
      <button class="primary" disabled={busy}>Verbinden</button>
      {#if conn}<button type="button" onclick={() => (editing = false)}>Abbrechen</button>{/if}
    </div>
  </form>
{/if}

{#if conn && result && !result.online}
  <p class="banner" role="status">Registry nicht erreichbar</p>
{/if}

{#each result?.rows ?? [] as r (r.id + (r.file ?? ""))}
  {@const v = rowView(r, result!.online)}
  <div class="card">
    <div>
      <strong>{r.title}</strong>
      {#if v.label}<div class="muted">{v.label}</div>{/if}
    </div>
    <div class="actions">
      {#if v.action}<button class="primary" disabled={busy} onclick={() => install(r)}>{v.action}</button>{/if}
      {#if v.remove}<button disabled={busy} onclick={() => remove(r)}>Entfernen…</button>{/if}
    </div>
  </div>
{/each}

<style>
  .head { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
  .conn { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
  .card { display: flex; align-items: center; justify-content: space-between; gap: 1rem; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 1rem 1.5rem; margin-bottom: 1rem; }
  .card strong { font-size: 1.15rem; }
  .form { flex-direction: column; align-items: stretch; margin-bottom: 1.5rem; }
  label { display: flex; flex-direction: column; gap: .25rem; }
  input { font: inherit; min-height: 44px; padding: 0 .75rem; border-radius: 8px; border: 1px solid var(--line); background: var(--bg); color: var(--fg); }
  .muted { color: var(--muted); }
  .actions { display: flex; flex-wrap: wrap; gap: .5rem; }
  button { font: inherit; min-height: 48px; padding: 0 1.25rem; border-radius: 10px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  button:disabled { opacity: .5; cursor: not-allowed; }
  .banner { background: var(--over-soft); border-left: 4px solid var(--over); padding: .75rem 1rem; }
</style>

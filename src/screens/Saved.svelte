<script lang="ts">
  import { ask, open, save } from "@tauri-apps/plugin-dialog";
  import { deleteResult, exportResult, importResult, listResults, openResult } from "../lib/api";
  import { formatCreatedAt } from "../lib/kennung";
  import type { OpenedResult, SavedSummary } from "../lib/types";

  let { onopen }: { onopen: (res: OpenedResult) => void } = $props();

  let rows = $state<SavedSummary[] | null>(null);
  let banner = $state<string | null>(null);

  // Runs an action; any German error string goes to the banner.
  async function run(fn: () => Promise<unknown>) {
    banner = null;
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    }
  }
  const load = () => run(async () => (rows = await listResults()));
  load();

  const openRow = (name: string) => run(async () => onopen(await openResult(name)));

  const exportRow = (name: string) =>
    run(async () => {
      const dest = await save({ defaultPath: name });
      if (dest) await exportResult(name, dest);
    });

  const importOne = () =>
    run(async () => {
      const src = await open({ filters: [{ name: "Auswertung", extensions: ["json"] }], multiple: false });
      if (!src) return;
      await importResult(src);
      rows = await listResults();
    });

  const remove = (r: SavedSummary) =>
    run(async () => {
      if (!(await ask(`Auswertung „${r.kennung}“ endgültig löschen?`))) return;
      await deleteResult(r.name);
      rows = await listResults();
    });
</script>

<h1>Gespeicherte Auswertungen</h1>
{#if banner}<p class="banner" role="alert">{banner}</p>{/if}
{#if rows?.length === 0}<p class="muted">Keine gespeicherten Auswertungen.</p>{/if}
{#if rows?.length}
  <table>
    <thead><tr><th>Kennung</th><th>Fragebogen</th><th>Datum</th><th><span class="sr">Aktionen</span></th></tr></thead>
    <tbody>
      {#each rows as r (r.name)}
        <tr>
          <td><strong>{r.kennung}</strong><div class="tech">{r.name}</div></td>
          <td>{r.title}</td>
          <td>{formatCreatedAt(r.created_at)}</td>
          <td class="actions">
            <button class="primary" onclick={() => openRow(r.name)}>Öffnen</button>
            <button onclick={() => exportRow(r.name)}>Exportieren</button>
            <button onclick={() => remove(r)}>Löschen</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
<div class="bar"><button onclick={importOne}>Importieren…</button></div>

<style>
  table { width: 100%; border-collapse: collapse; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  th, td { text-align: left; padding: .75rem 1rem; border-bottom: 1px solid var(--line); vertical-align: middle; }
  tr:last-child td { border-bottom: 0; }
  .actions { display: flex; flex-wrap: wrap; gap: .5rem; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  .muted { color: var(--muted); }
  .bar { margin-top: 1.5rem; }
  button { font: inherit; min-height: 48px; padding: 0 1.25rem; border-radius: 10px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  .banner { background: var(--over-soft); border-left: 4px solid var(--over); padding: .75rem 1rem; }
  @media (max-width: 40rem) {
    thead { display: none; }
    table, tbody, tr, td { display: block; }
    tr { border-bottom: 1px solid var(--line); padding: .5rem 0; }
    td { border: 0; padding: .25rem 1rem; }
  }
</style>

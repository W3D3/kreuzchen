<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { deleteDefinition, duplicateDefinition, listDefinitions, readDefinitionSource, yamlToDefinition } from "../lib/api";
  import { newDoc, openDoc, type Doc } from "../lib/editor";
  import type { DefinitionEntry } from "../lib/types";

  let { onedit }: { onedit: (doc: Doc) => void } = $props();

  let entries = $state<DefinitionEntry[] | null>(null);
  let banner = $state<string | null>(null);

  async function run(fn: () => Promise<unknown>) {
    banner = null;
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    }
  }
  const load = () => run(async () => (entries = await listDefinitions()));
  load();

  const edit = (e: DefinitionEntry) =>
    run(async () => {
      const yaml = await readDefinitionSource(e.file, e.locked);
      const def = await yamlToDefinition(yaml).catch(() => null);
      onedit(openDoc(e.file, e.hash, yaml, def));
    });

  const duplicate = (e: DefinitionEntry) =>
    run(async () => {
      await duplicateDefinition(e.file, e.locked);
      entries = await listDefinitions();
    });

  const remove = (e: DefinitionEntry) =>
    run(async () => {
      const title = e.summary?.title ?? e.file;
      if (!(await ask(`Fragebogen „${title}“ endgültig löschen? Gespeicherte Auswertungen bleiben erhalten.`, { kind: "warning" }))) return;
      await deleteDefinition(e.file, e.locked);
      entries = await listDefinitions();
    });
</script>

<h1>Fragebögen bearbeiten</h1>
{#if banner}<p class="banner" role="alert">{banner}</p>{/if}
{#each entries ?? [] as e (e.locked + e.file + e.hash)}
  <div class="card">
    <div>
      <strong>{e.summary?.title ?? "Fehlerhafte Datei"}</strong>{#if e.locked}<span class="pill">Registry</span>{/if}
      <div class="muted">{e.file}{e.summary ? ` · Version ${e.summary.version}` : ""}</div>
    </div>
    <div class="actions">
      <button class="primary" disabled={e.locked || !e.file || !e.hash} onclick={() => edit(e)}>Bearbeiten</button>
      <button disabled={!e.hash} onclick={() => duplicate(e)}>Duplizieren</button>
      <button disabled={e.locked || !e.file} onclick={() => remove(e)}>Löschen</button>
    </div>
  </div>
{/each}
<button class="primary" onclick={() => onedit(newDoc())}>Neuer Fragebogen</button>

<style>
  .card { display: flex; align-items: center; justify-content: space-between; gap: 1rem; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 1rem 1.5rem; margin-bottom: 1rem; }
  .card strong { font-size: 1.15rem; }
  .muted { color: var(--muted); font-family: var(--mono); }
  .pill { margin-left: .5rem; padding: .1rem .5rem; border: 1px solid var(--line); border-radius: 999px; font-size: .8rem; color: var(--muted); vertical-align: middle; }
  .actions { display: flex; flex-wrap: wrap; gap: .5rem; }
  button { font: inherit; min-height: 48px; padding: 0 1.25rem; border-radius: 10px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  button:disabled { opacity: .5; cursor: not-allowed; }
  .banner { background: var(--over-soft); border-left: 4px solid var(--over); padding: .75rem 1rem; }
</style>

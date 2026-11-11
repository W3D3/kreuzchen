<script lang="ts">
  import { dataDir, getDefinition, listDefinitions, openDefinitionsFolder } from "../lib/api";
  import { initialInput } from "../lib/input";
  import type { DefinitionEntry, Input, OpenedDefinition } from "../lib/types";

  let { onstart }: { onstart: (opened: OpenedDefinition, input: Input) => void } = $props();

  let entries = $state<DefinitionEntry[] | null>(null);
  let dir = $state("");
  let banner = $state<string | null>(null);

  async function load() {
    try {
      [entries, dir] = await Promise.all([listDefinitions(), dataDir()]);
    } catch (e) {
      banner = String(e);
    }
  }
  load();

  async function start(file: string) {
    try {
      const opened = await getDefinition(file);
      onstart(opened, initialInput(opened.definition, new Date()));
    } catch (e) {
      banner = String(e);
    }
  }
</script>

<h1>Fragebögen</h1>
{#if banner}<p class="banner" role="alert">{banner}</p>{/if}
{#if entries?.length === 0}
  <p class="muted">Noch keine Fragebögen installiert. Bitte wenden Sie sich an den Support.</p>
{/if}
{#each entries ?? [] as e (e.file + e.hash + (e.error ?? ""))}
  <div class="card">
    <div>
      {#if e.summary}
        <strong>{e.summary.title}</strong>
        <div class="muted">
          {e.summary.item_count} Fragen · {e.summary.scale_count} Skalen · Version {e.summary.version}<span class="tech"> · {e.file}</span>
        </div>
      {:else}
        <strong>Unbekannter Fragebogen</strong>
        <div class="muted">Dieser Fragebogen kann nicht geöffnet werden. Bitte wenden Sie sich an den Support.</div>
        <pre class="tech">{e.file ? e.file + "\n" : ""}{e.error}</pre>
      {/if}
    </div>
    <button class="primary" disabled={!e.summary} onclick={() => start(e.file)}>Starten</button>
  </div>
{/each}
<div class="tech footer">
  Datenordner: {dir}
  <button onclick={() => openDefinitionsFolder().catch((e) => (banner = String(e)))}>Ordner öffnen</button>
  <button onclick={load}>Neu laden</button>
</div>

<style>
  .card { display: flex; align-items: center; justify-content: space-between; gap: 1rem; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 1.25rem 1.5rem; margin-bottom: 1rem; }
  .card strong { font-size: 1.25rem; }
  .muted { color: var(--muted); }
  pre { font-family: var(--mono); white-space: pre-wrap; margin: .5rem 0 0; }
  .footer { margin-top: 2rem; color: var(--muted); }
  button { font: inherit; min-height: 48px; padding: 0 1.5rem; border-radius: 10px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  button:disabled { opacity: .5; cursor: not-allowed; }
  .banner { background: var(--over-soft); border-left: 4px solid var(--over); padding: .75rem 1rem; }
</style>

<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { saveResult } from "../lib/api";
  import { localTimestamp } from "../lib/kennung";
  import type { Graded, Input, OpenedDefinition } from "../lib/types";

  let { opened, input, graded, notice, preview = false, onregrade, onback, onnew }: {
    opened: OpenedDefinition | null;
    input: Input;
    graded: Graded;
    notice: string | null;
    preview?: boolean;
    onregrade?: (() => void) | null;
    onback: () => void;
    onnew: () => void;
  } = $props();

  const g = $derived(graded.grading);
  const kennung = $derived(String(input.fields.kennung ?? ""));
  const anyMet = $derived(g.rules.some((r) => r.met));
  const meter = (value: number, cutoff: number) => Math.min(value / (cutoff * 2), 1) * 100;

  let banner = $state<string | null>(null);
  let status = $state("");
  let saved = $state(false);

  async function copy() {
    banner = null;
    try {
      await writeText(graded.text);
      status = "Kopiert.";
    } catch (e) {
      status = "";
      banner = String(e);
    }
  }

  async function save() {
    if (!opened || saved) return;
    banner = null;
    try {
      await saveResult({
        format: 1,
        kennung,
        created_at: localTimestamp(new Date()),
        questionnaire: {
          id: opened.definition.id,
          title: opened.definition.title,
          version: opened.definition.version,
          hash: opened.hash,
        },
        input,
        grading: g,
        text: graded.text,
      });
      saved = true;
      status = "Gespeichert.";
    } catch (e) {
      banner = String(e);
    }
  }
</script>

{#if notice}<p class="notice">{notice}</p>{/if}
{#if banner}<p class="banner" role="alert">{banner}</p>{/if}

<h1>Ergebnis</h1>
<p class="head">
  {#each g.fields as f, i (f.id)}{#if i > 0}<span class="muted"> · </span>{/if}{#if f.id === "kennung"}<strong>{f.value}</strong>{:else}<span class="muted">{f.label}: {f.value}</span>{/if}{/each}
</p>

<div class="rows">
  {#each g.rows as r (r.id)}
    <div class="row">
      {#if r.kind === "sum"}
        <div class="line">
          <span class="label">{r.label}</span>
          <span class="num">{r.value}{r.cutoff !== null ? ` / ${r.cutoff}` : ""}</span>
          {#if r.over}<span class="pill">über Cut-off</span>{/if}
        </div>
        {#if r.cutoff}
          <div class="meter" aria-hidden="true">
            <div class="fill" class:over={r.over} style:width="{meter(r.value, r.cutoff)}%"></div>
            <div class="mark"></div>
          </div>
        {/if}
      {:else}
        <div class="line"><span class="label">{r.label}</span><span>{r.answer ?? "–"}</span></div>
      {/if}
    </div>
  {/each}
</div>

<div class="verdict" class:over={anyMet}>
  {#each g.rules as r (r.id)}<div>{r.met ? "✓" : "✗"} {r.label}</div>{/each}
  <div>Fehlende Antworten: <span class="num">{g.missing}</span>{g.missing > 0 ? ` (Fragen ${g.missing_items.join(", ")})` : ""}</div>
  {#if g.verdict}<p class="vtext"><strong>{g.verdict}</strong></p>{/if}
</div>

{#each g.caveats as c}<p class="caveat">Hinweis: {c}</p>{/each}

<div class="bar">
  {#if opened}<button onclick={onback}>Zurück zu den Antworten</button>{/if}
  {#if onregrade}<button onclick={onregrade}>Neu auswerten</button>{/if}
  <button onclick={copy}>Als Text kopieren</button>
  {#if opened && !preview}<button onclick={save} disabled={saved}>Speichern</button>{/if}
  <button class="primary" onclick={onnew}>{preview ? "Zurück zum Editor" : "Neuer Fragebogen"}</button>
  <span aria-live="polite">{status}</span>
</div>

<style>
  .muted { color: var(--muted); }
  .head { font-size: 1.1rem; }
  .rows { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: .5rem 1.25rem; }
  .row { padding: .75rem 0; border-bottom: 1px solid var(--line); }
  .row:last-child { border-bottom: 0; }
  .line { display: flex; align-items: baseline; gap: .75rem; font-size: 1.15rem; }
  .label { flex: 1; }
  .pill { background: var(--over-soft); color: var(--over); border: 1px solid var(--over); border-radius: 999px; padding: .1rem .75rem; font-size: .95rem; font-weight: 700; }
  .meter { position: relative; height: .5rem; margin-top: .5rem; background: var(--bg); border-radius: 4px; }
  .fill { height: 100%; background: var(--accent); border-radius: 4px; }
  .fill.over { background: var(--over); }
  .mark { position: absolute; left: 50%; top: -3px; bottom: -3px; width: 2px; background: var(--fg); }
  .verdict { margin-top: 1rem; padding: 1rem 1.25rem; border-radius: 12px; border: 1px solid var(--line); background: var(--surface); font-size: 1.1rem; }
  .verdict.over { background: var(--over-soft); border-color: var(--over); }
  .vtext { margin: .75rem 0 0; font-size: 1.3rem; }
  .caveat { margin: .75rem 0 0; padding: .75rem 1rem; border: 1px dashed var(--muted); border-radius: 10px; }
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: .75rem; margin-top: 1.5rem; }
  button { font: inherit; min-height: 48px; padding: 0 1.5rem; border-radius: 10px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  button:disabled { opacity: .6; cursor: default; }
  .banner, .notice { margin: 0 0 1rem; padding: .75rem 1rem; border-left: 4px solid var(--over); background: var(--over-soft); }
  .notice { border-color: var(--accent); background: var(--accent-soft); }
</style>

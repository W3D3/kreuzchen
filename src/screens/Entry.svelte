<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { grade } from "../lib/api";
  import { entryItems } from "../lib/input";
  import { entryKey } from "../lib/keyboard";
  import type { Graded, Input, OpenedDefinition } from "../lib/types";

  let { opened, input, onback, ongraded }: {
    opened: OpenedDefinition;
    input: Input;
    onback: () => void;
    ongraded: (graded: Graded, input: Input) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  const def = opened.definition;
  const items = entryItems(def);
  // svelte-ignore state_referenced_locally
  let answers = $state<Record<string, number>>({ ...input.answers });
  // svelte-ignore state_referenced_locally
  let fields = $state<Record<string, boolean | number | string>>({ ...input.fields });
  let focused = $state(0);
  let tried = $state(false);
  let banner = $state<string | null>(null);
  let busy = false;
  let els: HTMLElement[] = [];

  const scored = items.filter((i) => i.scored);
  const answered = $derived(scored.filter((i) => answers[i.key] !== undefined).length);
  const options = $derived(items[focused]?.options.length ?? 0);

  function focusItem(i: number) {
    focused = i;
    els[i]?.focus();
    els[i]?.scrollIntoView({ block: "nearest" });
  }

  async function doGrade() {
    if (busy) return;
    busy = true;
    tried = true;
    banner = null;
    const finalInput: Input = { fields: { ...fields }, answers: { ...answers } };
    try {
      ongraded(await grade(opened.file, opened.hash, finalInput), finalInput);
    } catch (e) {
      banner = String(e);
    } finally {
      busy = false;
    }
  }

  async function cancel() {
    if (Object.keys(answers).length === 0 || (await ask("Eingaben verwerfen?", { title: "Abbrechen", kind: "warning" }))) onback();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.altKey || e.metaKey) return;
    const item = items[focused];
    const a = entryKey(
      { key: e.key, shift: e.shiftKey, ctrl: e.ctrlKey },
      { focused, count: items.length, options: item.options.length, current: answers[item.key] ?? null },
    );
    if (a.type === "none") return;
    // At either end Tab leaves the list, so header fields and buttons stay reachable.
    if (e.key === "Tab" && a.type === "focus" && a.to === focused) return;
    e.preventDefault();
    if (a.type === "answer") {
      answers[item.key] = a.index;
      if (/^[1-9]$/.test(e.key)) focusItem(Math.min(focused + 1, items.length - 1));
    } else if (a.type === "clear") delete answers[item.key];
    else if (a.type === "focus") focusItem(a.to);
    else void doGrade();
  }
</script>

<div class="bar top">
  <span class="num">{answered} / {scored.length} beantwortet</span>
  <button onclick={cancel}>Abbrechen</button>
  <button class="primary" onclick={doGrade}>Auswerten</button>
</div>

<h1>{def.title}</h1>
{#if banner}<p class="banner" role="alert">{banner}</p>{/if}

<div class="fields">
  {#each def.fields as f (f.id)}
    <label class:check={f.type === "checkbox"}>
      <span>{f.id === "kennung" ? "Kennung (keine Patientennamen)" : f.label}</span>
      {#if f.type === "checkbox"}
        <input type="checkbox" bind:checked={fields[f.id] as boolean} />
      {:else if f.type === "number"}
        <input type="number" bind:value={fields[f.id] as number} />
      {:else if f.type === "date"}
        <input type="date" bind:value={fields[f.id] as string} />
      {:else}
        <input type="text" bind:value={fields[f.id] as string} />
      {/if}
    </label>
  {/each}
</div>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="items" {onkeydown}>
  {#each items as it, i (it.key)}
    {#if i === 0 || it.section !== items[i - 1].section}
      {@const sec = def.sections[it.section]}
      <h2>{sec.title}</h2>
      {#if sec.intro}<p class="muted">{sec.intro}</p>{/if}
    {/if}
    <div
      class="item"
      class:focus={i === focused}
      class:missing={tried && it.scored && answers[it.key] === undefined}
      role="radiogroup"
      aria-label={it.text}
      tabindex={i === focused ? 0 : -1}
      bind:this={els[i]}
      onfocus={() => (focused = i)}
    >
      <div class="q"><span class="num n">{it.key}</span> {it.text}</div>
      <div class="opts">
        {#each it.options as o, j (j)}
          <button
            type="button"
            role="radio"
            aria-checked={answers[it.key] === j}
            tabindex="-1"
            class:sel={answers[it.key] === j}
            onclick={() => { answers[it.key] = j; focusItem(i); }}
          >
            <span class="num k">{j + 1}</span> {o}
          </button>
        {/each}
      </div>
    </div>
  {/each}
</div>

<div class="bar bottom">
  1–{options} Antwort · ←/→ Antwort ändern · Enter/Tab nächste Frage · Umschalt+Tab zurück · Entf löschen · Strg+Enter Auswerten
</div>

<style>
  .bar { background: var(--surface); border: 1px solid var(--line); padding: .5rem 1rem; }
  .top { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; gap: .75rem; border-radius: 10px; margin-bottom: 1rem; }
  .top span { flex: 1; font-size: 1.1rem; }
  .bottom { position: fixed; left: 0; right: 0; bottom: 0; text-align: center; color: var(--muted); border-width: 1px 0 0; }
  .fields { display: flex; flex-wrap: wrap; gap: 1rem; margin-bottom: 1rem; }
  label { display: flex; flex-direction: column; gap: .25rem; flex: 1 1 14rem; }
  label.check { flex-direction: row; align-items: center; flex: 0 0 auto; }
  input[type="text"], input[type="number"], input[type="date"] { font: inherit; min-height: 48px; padding: 0 .75rem; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--fg); }
  input[type="checkbox"] { width: 1.5rem; height: 1.5rem; }
  .items { padding-bottom: 4rem; }
  .muted { color: var(--muted); }
  .item { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 1rem 1.25rem; margin-bottom: .75rem; }
  .item:focus { outline: none; }
  .item.focus { outline: 3px solid var(--accent); outline-offset: 2px; }
  .item.missing { border-left: 6px solid var(--over); }
  .q { font-size: 1.15rem; margin-bottom: .75rem; }
  .n { color: var(--muted); margin-right: .5rem; }
  .opts { display: flex; flex-wrap: wrap; gap: .5rem; }
  .opts button { font: inherit; min-height: 48px; padding: 0 1.25rem; border-radius: 10px; border: 1px solid var(--line); background: var(--bg); color: var(--fg); cursor: pointer; }
  .opts button.sel { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  .k { font-size: .75rem; opacity: .7; margin-right: .35rem; }
  .bar button { font: inherit; min-height: 48px; padding: 0 1.5rem; border-radius: 10px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  .bar button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  .banner { background: var(--over-soft); border-left: 4px solid var(--over); padding: .75rem 1rem; }
</style>

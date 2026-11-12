<script lang="ts">
  import { definitionToYaml, saveDefinition, validateDefinition, yamlToDefinition } from "../lib/api";
  import {
    errorList, formatDefault, formatKey, formChanged, freshKey, move, numbers, parseDefault, parseItemRef, parseKey,
    renameKey, splitList, type Doc,
  } from "../lib/editor";

  let { doc = $bindable(), onclose }: {
    doc: Doc;
    onclose: () => void;
  } = $props();

  let errors = $state<string[] | null>(null);
  let banner = $state<string | null>(null);
  let status = $state("");
  let seq = 0;

  const opt = (s: string) => s.trim() || null;
  const val = (e: Event) => (e.currentTarget as HTMLInputElement).value;
  // In the form view, the original text is kept (with its comments) until the form changes.
  const currentYaml = () => (formChanged(doc) ? definitionToYaml(doc.def!) : Promise.resolve(doc.yaml));

  // Live validation, debounced; only the newest answer counts.
  $effect(() => {
    void (doc.mode === "form" ? JSON.stringify(doc.def) : doc.yaml);
    const id = ++seq;
    const t = setTimeout(async () => {
      let res: string[] = [];
      try {
        await validateDefinition(await currentYaml());
      } catch (e) {
        res = errorList(e);
      }
      if (id === seq) errors = res;
    }, 400);
    return () => clearTimeout(t);
  });

  async function run(fn: () => Promise<void>) {
    banner = null;
    status = "";
    try {
      await fn();
    } catch (e) {
      banner = errorList(e).join("\n");
    }
  }

  const switchTo = (mode: Doc["mode"]) =>
    run(async () => {
      if (mode === doc.mode) return;
      if (mode === "yaml") {
        if (formChanged(doc)) {
          doc.yaml = await definitionToYaml(doc.def!);
          doc.dirty = true;
        }
      } else {
        const def = await yamlToDefinition(doc.yaml);
        doc.def = def;
        doc.snapshot = JSON.stringify(def);
      }
      doc.mode = mode;
    });

  const save = () =>
    run(async () => {
      const yaml = await currentYaml();
      const file = doc.file ?? `${(await yamlToDefinition(yaml)).id}.yaml`;
      doc.hash = await saveDefinition(file, yaml, doc.hash);
      doc.file = file;
      doc.yaml = yaml;
      doc.dirty = false;
      if (doc.mode === "form") doc.snapshot = JSON.stringify(doc.def);
      status = "Gespeichert.";
    });
</script>

{#snippet tools(list: unknown[], i: number)}
  <span class="tools">
    <button type="button" aria-label="Nach oben" title="Nach oben" disabled={i === 0} onclick={() => move(list, i, -1)}>↑</button>
    <button type="button" aria-label="Nach unten" title="Nach unten" disabled={i === list.length - 1} onclick={() => move(list, i, 1)}>↓</button>
    <button type="button" aria-label="Entfernen" title="Entfernen" onclick={() => list.splice(i, 1)}>✕</button>
  </span>
{/snippet}

<div class="bar">
  <span class="file num">{doc.file ?? "Neue Datei"}</span>
  <span class="tabs" role="group" aria-label="Ansicht">
    <button class:active={doc.mode === "form"} aria-pressed={doc.mode === "form"} onclick={() => switchTo("form")}>Formular</button>
    <button class:active={doc.mode === "yaml"} aria-pressed={doc.mode === "yaml"} onclick={() => switchTo("yaml")}>YAML</button>
  </span>
  <button class="primary" disabled={!!errors?.length} onclick={save}>Speichern</button>
  <button onclick={onclose}>Schließen</button>
  <span aria-live="polite">{status}</span>
</div>
{#if banner}<pre class="banner" role="alert">{banner}</pre>{/if}
<div class="check" aria-live="polite">
  {#if errors?.length}
    <strong>Fehler (Speichern nicht möglich):</strong>
    <ul>{#each errors as e, i (i)}<li>{e}</li>{/each}</ul>
  {:else if errors}
    <span class="ok">Keine Fehler.</span>
  {/if}
</div>

{#if doc.mode === "yaml"}
  <textarea class="yaml" spellcheck="false" aria-label="YAML" bind:value={doc.yaml} oninput={() => (doc.dirty = true)}></textarea>
{:else if doc.def}
  {@const d = doc.def}
  <p class="note">Kommentare in der Datei gehen beim Speichern über das Formular verloren.</p>

  <fieldset>
    <legend>Allgemein</legend>
    <div class="grid">
      <label>ID <input bind:value={d.id} /></label>
      <label>Version <input bind:value={d.version} /></label>
      <label class="wide">Titel <input bind:value={d.title} /></label>
      <label class="wide">Autoren <input value={d.authors ?? ""} onchange={(e) => (d.authors = opt(val(e)))} /></label>
      <label class="wide">Copyright <input value={d.copyright ?? ""} onchange={(e) => (d.copyright = opt(val(e)))} /></label>
      <label>„über Cut-off“ heißt
        <select bind:value={d.cutoff_compare}><option value=">=">≥ (größer gleich)</option><option value=">">&gt; (größer)</option></select>
      </label>
    </div>
  </fieldset>

  <fieldset>
    <legend>Antwortsätze</legend>
    {#each Object.entries(d.answer_sets) as [name, labels] (name)}
      <div class="row">
        <input class="id" aria-label="Name" value={name} onchange={(e) => (d.answer_sets = renameKey(d.answer_sets, name, val(e).trim()))} />
        <input aria-label="Antworten, mit Komma getrennt" value={labels.join(", ")} onchange={(e) => (d.answer_sets[name] = splitList(val(e)))} />
        <button type="button" aria-label="Entfernen" onclick={() => delete d.answer_sets[name]}>✕</button>
      </div>
    {/each}
    <button type="button" onclick={() => (d.answer_sets[freshKey(d.answer_sets, "antworten")] = ["Nein", "Ja"])}>+ Antwortsatz</button>
    <label class="inline">Standard-Antwortsatz
      <select value={d.default_answers ?? ""} onchange={(e) => (d.default_answers = opt(val(e)))}>
        <option value="">(keiner)</option>
        {#each Object.keys(d.answer_sets) as n (n)}<option value={n}>{n}</option>{/each}
      </select>
    </label>
  </fieldset>

  <fieldset>
    <legend>Schlüssel (Punkte je Antwort)</legend>
    {#each Object.entries(d.keys) as [name, points] (name)}
      <div class="row">
        <input class="id" aria-label="Name" value={name} onchange={(e) => (d.keys = renameKey(d.keys, name, val(e).trim()))} />
        <input class="num" aria-label="Punkte, mit Komma getrennt" value={points.join(", ")} onchange={(e) => (d.keys[name] = numbers(val(e)))} />
        <button type="button" aria-label="Entfernen" onclick={() => delete d.keys[name]}>✕</button>
      </div>
    {/each}
    <button type="button" onclick={() => (d.keys[freshKey(d.keys, "schluessel")] = [0, 1])}>+ Schlüssel</button>
  </fieldset>

  <fieldset>
    <legend>Kopffelder</legend>
    {#each d.fields as f, i (f)}
      <div class="row">
        <input class="id" aria-label="ID" bind:value={f.id} />
        <input aria-label="Beschriftung" bind:value={f.label} />
        <select aria-label="Typ" bind:value={f.type}>
          <option value="text">Text</option><option value="number">Zahl</option><option value="date">Datum</option><option value="checkbox">Ankreuzfeld</option>
        </select>
        <input class="id" aria-label="Vorgabe" placeholder="Vorgabe" value={formatDefault(f.default)} onchange={(e) => (f.default = parseDefault(f.type, val(e)))} />
        {@render tools(d.fields, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.fields.push({ id: "", label: "", type: "text", default: null })}>+ Feld</button>
  </fieldset>

  <fieldset>
    <legend>Skalen</legend>
    {#each d.scales as s, i (s)}
      <div class="row">
        <input class="id" aria-label="ID" bind:value={s.id} />
        <input aria-label="Beschriftung" bind:value={s.label} />
        <input class="cut num" type="number" aria-label="Cut-off" placeholder="Cut-off" bind:value={s.cutoff} />
        {@render tools(d.scales, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.scales.push({ id: "", label: "", cutoff: null })}>+ Skala</button>
  </fieldset>

  <fieldset>
    <legend>Abgeleitete Werte</legend>
    {#each d.derived as v, i (v)}
      <div class="row">
        <input class="id" aria-label="ID" bind:value={v.id} />
        <input aria-label="Beschriftung" bind:value={v.label} />
        <input class="num" aria-label="Formel" placeholder="z. B. A + B" bind:value={v.value} />
        <input class="cut num" type="number" aria-label="Cut-off" placeholder="Cut-off" bind:value={v.cutoff} />
        {@render tools(d.derived, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.derived.push({ id: "", label: "", value: "", cutoff: null })}>+ Abgeleiteter Wert</button>
  </fieldset>

  <fieldset>
    <legend>Abschnitte und Fragen</legend>
    <datalist id="keys">{#each Object.keys(d.keys) as k (k)}<option value={k}></option>{/each}</datalist>
    {#each d.sections as sec, si (sec)}
      <div class="section">
        <div class="row">
          <input aria-label="Abschnittstitel" bind:value={sec.title} />
          {@render tools(d.sections, si)}
        </div>
        <input class="intro" aria-label="Einleitung" placeholder="Einleitung (optional)" value={sec.intro ?? ""} onchange={(e) => (sec.intro = opt(val(e)))} />
        {#each sec.items as it, i (it)}
          <div class="row item">
            <input class="ref num" aria-label="Nr. oder ID" value={it.id ?? it.n ?? ""} onchange={(e) => Object.assign(it, parseItemRef(val(e)))} />
            <input aria-label="Fragetext" bind:value={it.text} />
            <select aria-label="Skala" value={it.scale ?? ""} onchange={(e) => (it.scale = opt(val(e)))}>
              <option value="">(keine Skala)</option>
              {#each d.scales as s (s)}<option value={s.id}>{s.id}</option>{/each}
            </select>
            <input class="id num" aria-label="Schlüssel oder Punkte" placeholder="Schlüssel" list="keys" value={formatKey(it.key)} onchange={(e) => (it.key = parseKey(val(e)))} />
            <select aria-label="Antwortsatz" value={it.answers ?? ""} onchange={(e) => (it.answers = opt(val(e)))}>
              <option value="">(Standard)</option>
              {#each Object.keys(d.answer_sets) as n (n)}<option value={n}>{n}</option>{/each}
            </select>
            {@render tools(sec.items, i)}
          </div>
        {/each}
        <button type="button" onclick={() => sec.items.push({ n: null, id: null, text: "", answers: null, scale: null, key: null })}>+ Frage</button>
      </div>
    {/each}
    <button type="button" onclick={() => d.sections.push({ title: "", intro: null, items: [] })}>+ Abschnitt</button>
  </fieldset>

  <fieldset>
    <legend>Ergebnis-Reihenfolge</legend>
    {#each d.result as _, i (i)}
      <div class="row">
        <input class="num" aria-label="ID" bind:value={d.result[i]} />
        {@render tools(d.result, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.result.push("")}>+ Zeile</button>
  </fieldset>

  <fieldset>
    <legend>Regeln</legend>
    {#each d.rules as r, i (r)}
      <div class="row">
        <input class="id" aria-label="ID" bind:value={r.id} />
        <input aria-label="Beschriftung" bind:value={r.label} />
        <input class="num" aria-label="Bedingung" placeholder="Bedingung" bind:value={r.when} />
        {@render tools(d.rules, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.rules.push({ id: "", label: "", when: "" })}>+ Regel</button>
  </fieldset>

  <fieldset>
    <legend>Hinweise</legend>
    {#each d.caveats as c, i (c)}
      <div class="row">
        <input class="num" aria-label="Bedingung" placeholder="Bedingung" bind:value={c.when} />
        <input aria-label="Text" bind:value={c.text} />
        {@render tools(d.caveats, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.caveats.push({ when: "", text: "" })}>+ Hinweis</button>
  </fieldset>

  <fieldset>
    <legend>Befund (der erste zutreffende gilt)</legend>
    {#each d.verdict as v, i (v)}
      <div class="row">
        {#if "else" in v}
          <span class="else">sonst</span>
          <input aria-label="Text" bind:value={v.else} />
        {:else}
          <input class="num" aria-label="Bedingung" placeholder="Bedingung" bind:value={v.when} />
          <input aria-label="Text" bind:value={v.text} />
        {/if}
        {@render tools(d.verdict, i)}
      </div>
    {/each}
    <button type="button" onclick={() => d.verdict.push({ when: "", text: "" })}>+ Befund</button>
    <button type="button" onclick={() => d.verdict.push({ else: "" })}>+ Sonst-Befund</button>
  </fieldset>
{/if}

<style>
  .bar { position: sticky; top: 0; z-index: 2; display: flex; flex-wrap: wrap; align-items: center; gap: .5rem; background: var(--surface); border: 1px solid var(--line); border-radius: 10px; padding: .5rem 1rem; margin-bottom: 1rem; }
  .file { flex: 1; color: var(--muted); }
  .tabs { display: flex; gap: .25rem; }
  .tabs .active { background: var(--accent-soft); font-weight: 700; }
  button { font: inherit; min-height: 40px; padding: 0 1rem; border-radius: 8px; border: 1px solid var(--line); background: var(--surface); color: var(--fg); cursor: pointer; }
  button.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); font-weight: 700; }
  button:disabled { opacity: .4; cursor: default; }
  .banner { white-space: pre-wrap; font-family: var(--font); background: var(--over-soft); border-left: 4px solid var(--over); padding: .75rem 1rem; }
  .check { margin-bottom: 1rem; }
  .check ul { margin: .25rem 0 0; padding-left: 1.25rem; color: var(--over); }
  .ok { color: var(--muted); }
  .note { color: var(--muted); }
  .yaml { box-sizing: border-box; width: 100%; min-height: 70vh; font-family: var(--mono); font-size: .95rem; padding: .75rem; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--fg); }
  fieldset { border: 1px solid var(--line); border-radius: 12px; background: var(--surface); padding: 1rem 1.25rem; margin: 0 0 1rem; }
  legend { font-weight: 700; padding: 0 .5rem; }
  .grid { display: flex; flex-wrap: wrap; gap: .75rem; }
  .grid label { display: flex; flex-direction: column; gap: .25rem; flex: 1 1 10rem; }
  .grid label.wide { flex-basis: 100%; }
  .inline { display: flex; align-items: center; gap: .5rem; margin-top: .75rem; }
  .row { display: flex; align-items: center; gap: .5rem; margin-bottom: .5rem; }
  .row > input { flex: 1 1 8rem; min-width: 0; }
  .row > input.id { flex: 0 1 9rem; }
  .row > input.ref, .row > input.cut { flex: 0 0 5.5rem; }
  .row > input.num { font-family: var(--mono); }
  .tools { display: flex; gap: .25rem; }
  .tools button { padding: 0 .6rem; }
  input, select { font: inherit; min-height: 40px; padding: 0 .5rem; border: 1px solid var(--line); border-radius: 6px; background: var(--bg); color: var(--fg); box-sizing: border-box; }
  .section { border-left: 3px solid var(--accent-soft); padding-left: .75rem; margin-bottom: 1rem; }
  .intro { width: 100%; margin-bottom: .5rem; }
  .item { padding-left: 1rem; }
  .else { flex: 0 1 9rem; color: var(--muted); }
</style>

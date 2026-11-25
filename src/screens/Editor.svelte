<script lang="ts">
  import ArrowDown from "@lucide/svelte/icons/arrow-down";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import FileText from "@lucide/svelte/icons/file-text";
  import Info from "@lucide/svelte/icons/info";
  import Play from "@lucide/svelte/icons/play";
  import Plus from "@lucide/svelte/icons/plus";
  import Save from "@lucide/svelte/icons/save";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Banner from "$lib/components/Banner.svelte";
  import { confirm } from "$lib/components/confirm.svelte";
  import X from "@lucide/svelte/icons/x";
  import type { Snippet } from "svelte";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import IconButton from "$lib/components/IconButton.svelte";
  import SelectBox from "$lib/components/SelectBox.svelte";
  import { definitionToYaml, saveDefinition, validateDefinition, yamlToDefinition } from "../lib/api";
  import {
    errorList, formatDefault, formatKey, formChanged, freshKey, move, numbers, parseDefault, parseItemRef, parseKey,
    renameKey, splitList, type Doc,
  } from "../lib/editor";
  import type { Definition } from "../lib/types";

  let { doc = $bindable(), onclose, onpreview }: {
    doc: Doc;
    onclose: () => void;
    onpreview: (yaml: string, def: Definition) => void;
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
      const id = (await yamlToDefinition(yaml)).id;
      const file = doc.file ?? `${id}.yaml`;
      const idChanged = doc.id !== null && doc.id !== id;
      if (idChanged && !(await confirm({
        title: "ID ändern",
        message: `Die ID ändert sich von „${doc.id}“ zu „${id}“. Entwürfe und gespeicherte Auswertungen mit der alten ID finden diesen Fragebogen dann nicht mehr.`,
        confirmLabel: "ID ändern",
        destructive: true,
      }))) return;
      doc.hash = await saveDefinition(file, false, yaml, doc.hash, idChanged);
      doc.id = id;
      doc.file = file;
      doc.yaml = yaml;
      doc.dirty = false;
      if (doc.mode === "form") doc.snapshot = JSON.stringify(doc.def);
      status = "Gespeichert.";
    });

  const test = () =>
    run(async () => {
      const yaml = await currentYaml();
      await validateDefinition(yaml);
      onpreview(yaml, await yamlToDefinition(yaml));
    });

  const types = [
    { value: "text", label: "Text" }, { value: "number", label: "Zahl" },
    { value: "date", label: "Datum" }, { value: "checkbox", label: "Ankreuzfeld" },
  ];
  const compares = [{ value: ">=", label: "≥ (größer gleich)" }, { value: ">", label: "> (größer)" }];
  const setOptions = $derived(Object.keys(doc.def?.answer_sets ?? {}).map((n) => ({ value: n, label: n })));
  const scaleOptions = $derived([{ value: null, label: "(keine Skala)" }, ...(doc.def?.scales ?? []).filter((s) => s.id.trim()).map((s) => ({ value: s.id, label: s.id }))]);

  const lbl = "text-[14px] font-bold";
  // One grid template per table, shared by its header and rows so the columns line up.
  const cols = {
    sets: "grid-cols-[10rem_minmax(0,1fr)_7rem_2.25rem]",
    keys: "grid-cols-[10rem_minmax(0,1fr)_2.25rem]",
    fields: "grid-cols-[9rem_minmax(0,1fr)_9rem_8rem_7.5rem]",
    scales: "grid-cols-[9rem_minmax(0,1fr)_6rem_7.5rem]",
    derived: "grid-cols-[9rem_minmax(0,1fr)_10rem_6rem_7.5rem]",
    items: "grid-cols-[4.5rem_minmax(0,1fr)_10rem_7rem_8.5rem_7.5rem]",
    result: "grid-cols-[minmax(0,1fr)_7.5rem]",
    rules: "grid-cols-[9rem_minmax(0,1fr)_12rem_7.5rem]",
    cond: "grid-cols-[12rem_minmax(0,1fr)_7.5rem]",
  };
</script>

{#snippet tools(list: unknown[], i: number)}
  <div class="flex justify-end gap-0.5">
    <IconButton label="Nach oben" icon={ArrowUp} disabled={i === 0} onclick={() => move(list, i, -1)} />
    <IconButton label="Nach unten" icon={ArrowDown} disabled={i === list.length - 1} onclick={() => move(list, i, 1)} />
    <IconButton label="Entfernen" icon={Trash2} variant="destructive" onclick={() => list.splice(i, 1)} />
  </div>
{/snippet}

{#snippet head(grid: string, labels: string[])}
  <div class="grid {grid} gap-2 px-0.5 pb-1 text-[13px] font-bold text-muted-foreground">
    {#each labels as l, i (i)}<span>{l}</span>{/each}
  </div>
{/snippet}

{#snippet add(label: string, onclick: () => void)}
  <Button variant="ghost" class="-ml-2.5 self-start text-primary hover:text-primary" {onclick}><Plus />{label}</Button>
{/snippet}

{#snippet card(id: string, title: string, description: string | null, body: Snippet)}
  <Card.Root class="gap-3" role="group" aria-labelledby="ed-{id}">
    <Card.Header>
      <Card.Title id="ed-{id}" class="text-[17px] font-bold">{title}</Card.Title>
      {#if description}<Card.Description class="text-[14px]">{description}</Card.Description>{/if}
    </Card.Header>
    <Card.Content class="flex flex-col gap-2">{@render body()}</Card.Content>
  </Card.Root>
{/snippet}

<!-- Full-bleed bar inside the centred column, as on the entry screen. -->
<header class="sticky top-0 z-20 -mx-6 -mt-6 mb-6 bg-card px-6 [clip-path:inset(0_-100vmax_-1px)] shadow-[0_0_0_100vmax_var(--surface),0_1px_0_100vmax_var(--line)]">
  <div class="flex min-h-16 flex-wrap items-center gap-2 py-2.5">
    <FileText class="size-5 shrink-0 text-muted-foreground" />
    <span class="font-mono text-[15px]">{doc.file ?? "Neue Datei"}</span>
    {#if errors?.length}
      <Badge variant="outline" class="h-6 gap-1 border-over/50 bg-over-soft px-2 text-[13px] text-over"><CircleAlert />{errors.length} Fehler</Badge>
    {:else if errors}
      <Badge variant="secondary" class="h-6 gap-1 px-2 text-[13px]"><CircleCheck />Keine Fehler</Badge>
    {/if}
    <div class="flex-1"></div>
    <span aria-live="polite" class="mr-1 text-[14px] text-muted-foreground">{status}</span>
    <div role="group" aria-label="Ansicht" class="flex rounded-lg bg-muted p-0.5">
      {#each [["form", "Formular"], ["yaml", "YAML"]] as const as [m, label] (m)}
        <button
          type="button"
          aria-pressed={doc.mode === m}
          onclick={() => switchTo(m)}
          class="h-8 rounded-md px-3 text-[15px] text-muted-foreground outline-none focus-visible:ring-3 focus-visible:ring-ring/50 aria-pressed:bg-card aria-pressed:font-bold aria-pressed:text-foreground aria-pressed:shadow-sm"
        >{label}</button>
      {/each}
    </div>
    <Separator orientation="vertical" class="mx-1 h-6!" />
    <Button variant="outline" onclick={test}><Play />Testen</Button>
    <Button disabled={!!errors?.length} onclick={save}><Save />Speichern</Button>
    <Button variant="ghost" onclick={onclose}><X />Schließen</Button>
  </div>
</header>

<div class="flex flex-col gap-4 pb-16">
  {#if banner}<div class="scroll-mt-24" {@attach (el) => el.scrollIntoView({ block: "nearest" })}><Banner>{banner}</Banner></div>{/if}
  <div aria-live="polite">
    {#if errors?.length}
      <div class="rounded-xl border border-over/45 bg-over-soft px-5 py-3 text-[15px]">
        <strong class="flex items-center gap-2"><CircleAlert class="size-4 shrink-0 text-over" />Fehler (Speichern nicht möglich):</strong>
        <ul class="mt-1 list-disc pl-10 text-over">{#each errors as e, i (i)}<li>{e}</li>{/each}</ul>
      </div>
    {:else if errors}
      <span class="sr-only">Keine Fehler.</span>
    {/if}
  </div>

  {#if doc.mode === "yaml"}
    <textarea
      class="min-h-[70vh] w-full rounded-xl border border-input bg-card p-4 font-mono text-[15px] outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
      spellcheck="false" aria-label="YAML" bind:value={doc.yaml} oninput={() => (doc.dirty = true)}></textarea>
  {:else if doc.def}
    {@const d = doc.def}
    <p class="flex items-center gap-2 text-[14px] text-muted-foreground">
      <Info class="size-4 shrink-0" />Kommentare in der Datei gehen beim Speichern über das Formular verloren.
    </p>

    <Card.Root class="gap-4" role="group" aria-labelledby="ed-general">
      <Card.Header><Card.Title id="ed-general" class="text-[17px] font-bold">Allgemein</Card.Title></Card.Header>
      <Card.Content class="grid grid-cols-[1fr_8rem_12rem] gap-x-4 gap-y-3">
        <div class="grid gap-1.5"><Label for="f-title" class={lbl}>Titel</Label><Input id="f-title" bind:value={d.title} /></div>
        <div class="grid gap-1.5"><Label for="f-version" class={lbl}>Version</Label><Input id="f-version" class="font-mono" bind:value={d.version} /></div>
        <div class="grid gap-1.5"><Label for="f-id" class={lbl}>ID</Label><Input id="f-id" class="font-mono" bind:value={d.id} /></div>
        <div class="col-span-2 grid gap-1.5"><Label for="f-authors" class={lbl}>Autoren</Label><Input id="f-authors" value={d.authors ?? ""} onchange={(e) => (d.authors = opt(val(e)))} /></div>
        <div class="grid gap-1.5"><Label for="f-cutoff" class={lbl}>„über Cut-off“ heißt</Label>
          <SelectBox id="f-cutoff" label="„über Cut-off“ heißt" bind:value={d.cutoff_compare as string} options={compares} />
        </div>
        <div class="col-span-3 grid gap-1.5"><Label for="f-copyright" class={lbl}>Copyright</Label><Input id="f-copyright" value={d.copyright ?? ""} onchange={(e) => (d.copyright = opt(val(e)))} /></div>
      </Card.Content>
    </Card.Root>

    {#snippet sets()}
      {@render head(cols.sets, ["Name", "Antworten, mit Komma getrennt", "Tasten", ""])}
      {#each Object.entries(d.answer_sets) as [name, labels] (name)}
        <div class="grid {cols.sets} gap-2">
          <Input aria-label="Name" class="font-mono" value={name} onchange={(e) => {
            const to = val(e).trim();
            const sets = renameKey(d.answer_sets, name, to);
            if (sets === d.answer_sets) return;
            d.answer_sets = sets;
            if (name in d.shortcuts) d.shortcuts = renameKey(d.shortcuts, name, to);
          }} />
          <Input aria-label="Antworten, mit Komma getrennt" value={labels.join(", ")} onchange={(e) => (d.answer_sets[name] = splitList(val(e)))} />
          <Input aria-label="Tasten, ein Zeichen pro Antwort" class="font-mono" placeholder={labels.map((_, j) => j).join("")} value={d.shortcuts[name] ?? ""} onchange={(e) => {
            const s = val(e).trim();
            if (s) d.shortcuts[name] = s;
            else delete d.shortcuts[name];
          }} />
          <IconButton label="Entfernen" icon={Trash2} variant="destructive" onclick={() => { delete d.answer_sets[name]; delete d.shortcuts[name]; }} />
        </div>
      {/each}
      <div class="mt-1 flex items-center justify-between">
        {@render add("Antwortsatz", () => (d.answer_sets[freshKey(d.answer_sets, "answers")] = ["Nein", "Ja"]))}
        <div class="flex items-center gap-3">
          <Label for="f-defans" class={lbl}>Standard-Antwortsatz</Label>
          <SelectBox id="f-defans" label="Standard-Antwortsatz" class="w-44" mono bind:value={d.default_answers} options={[{ value: null, label: "(keiner)" }, ...setOptions]} />
        </div>
      </div>
    {/snippet}
    {@render card("sets", "Antwortsätze", "Die Antworten, die bei einer Frage angekreuzt werden können, in dieser Reihenfolge.", sets)}

    {#snippet keys()}
      {@render head(cols.keys, ["Name", "Punkte, mit Komma getrennt", ""])}
      {#each Object.entries(d.keys) as [name, points] (name)}
        <div class="grid {cols.keys} gap-2">
          <Input aria-label="Name" class="font-mono" value={name} onchange={(e) => (d.keys = renameKey(d.keys, name, val(e).trim()))} />
          <Input aria-label="Punkte, mit Komma getrennt" class="font-mono" value={points.join(", ")} onchange={(e) => (d.keys[name] = numbers(val(e)))} />
          <IconButton label="Entfernen" icon={Trash2} variant="destructive" onclick={() => delete d.keys[name]} />
        </div>
      {/each}
      {@render add("Schlüssel", () => (d.keys[freshKey(d.keys, "key")] = [0, 1]))}
    {/snippet}
    {@render card("keys", "Schlüssel (Punkte je Antwort)", null, keys)}

    {#snippet fields()}
      {@render head(cols.fields, ["ID", "Beschriftung", "Typ", "Vorgabe", ""])}
      {#each d.fields as f, i (f)}
        <div class="grid {cols.fields} gap-2">
          <Input aria-label="ID" class="font-mono" bind:value={f.id} />
          <Input aria-label="Beschriftung" bind:value={f.label} />
          <SelectBox label="Typ" bind:value={f.type as string} options={types} />
          <Input aria-label="Vorgabe" placeholder="Vorgabe" value={formatDefault(f.default)} onchange={(e) => (f.default = parseDefault(f.type, val(e)))} />
          {@render tools(d.fields, i)}
        </div>
      {/each}
      {@render add("Feld", () => d.fields.push({ id: "", label: "", type: "text", default: null }))}
    {/snippet}
    {@render card("fields", "Kopffelder", "Felder über den Fragen, z. B. Kennung oder Alter.", fields)}

    {#snippet scales()}
      {@render head(cols.scales, ["ID", "Beschriftung", "Cut-off", ""])}
      {#each d.scales as s, i (s)}
        <div class="grid {cols.scales} gap-2">
          <Input aria-label="ID" class="font-mono" bind:value={s.id} />
          <Input aria-label="Beschriftung" bind:value={s.label} />
          <Input type="number" aria-label="Cut-off" placeholder="Cut-off" class="text-right font-mono tabular-nums" bind:value={s.cutoff} />
          {@render tools(d.scales, i)}
        </div>
      {/each}
      {@render add("Skala", () => d.scales.push({ id: "", label: "", cutoff: null }))}
    {/snippet}
    {@render card("scales", "Skalen", "Summen über Fragen. Ab dem Cut-off gilt der Wert als auffällig.", scales)}

    {#snippet derived()}
      {@render head(cols.derived, ["ID", "Beschriftung", "Formel", "Cut-off", ""])}
      {#each d.derived as v, i (v)}
        <div class="grid {cols.derived} gap-2">
          <Input aria-label="ID" class="font-mono" bind:value={v.id} />
          <Input aria-label="Beschriftung" bind:value={v.label} />
          <Input aria-label="Formel" placeholder="z. B. A + B" class="font-mono" bind:value={v.value} />
          <Input type="number" aria-label="Cut-off" placeholder="Cut-off" class="text-right font-mono tabular-nums" bind:value={v.cutoff} />
          {@render tools(d.derived, i)}
        </div>
      {/each}
      {@render add("Abgeleiteter Wert", () => d.derived.push({ id: "", label: "", value: "", cutoff: null }))}
    {/snippet}
    {@render card("derived", "Abgeleitete Werte", "Formeln über Skalen, Felder und Fragen, z. B. A + B. count(Bedingung, …) zählt, wie viele Bedingungen zutreffen, z. B. count(f1 == \"Ja\", f2 == \"Ja\") (braucht Kreuzchen 0.6.0; beim Speichern wird min_app automatisch gesetzt).", derived)}

    <Card.Root class="gap-3" role="group" aria-labelledby="ed-sections">
      <Card.Header><Card.Title id="ed-sections" class="text-[17px] font-bold">Abschnitte und Fragen</Card.Title></Card.Header>
      <Card.Content class="flex flex-col gap-4">
        <datalist id="keys">{#each Object.keys(d.keys) as k (k)}<option value={k}></option>{/each}</datalist>
        {#each d.sections as sec, si (sec)}
          {#if si > 0}<Separator />{/if}
          <section class="flex flex-col gap-2">
            <div class="grid grid-cols-[minmax(0,1fr)_7.5rem] gap-2">
              <Input aria-label="Abschnittstitel" class="font-bold" bind:value={sec.title} />
              {@render tools(d.sections, si)}
            </div>
            <Input aria-label="Einleitung" placeholder="Einleitung (optional)" value={sec.intro ?? ""} onchange={(e) => (sec.intro = opt(val(e)))} />
            <div class="mt-1 flex flex-col gap-2 border-l-2 border-secondary pl-3">
              {@render head(cols.items, ["Nr. / ID", "Fragetext", "Skala", "Schlüssel", "Antwortsatz", ""])}
              {#each sec.items as it, i (it)}
                <div class="grid {cols.items} gap-2">
                  <Input aria-label="Nr. oder ID" class="font-mono tabular-nums" value={it.id ?? it.n ?? ""} onchange={(e) => Object.assign(it, parseItemRef(val(e)))} />
                  <Input aria-label="Fragetext" bind:value={it.text} />
                  <SelectBox label="Skala" mono bind:value={it.scale} options={scaleOptions} />
                  <Input aria-label="Schlüssel oder Punkte" placeholder="Schlüssel" list="keys" class="font-mono" value={formatKey(it.key)} onchange={(e) => (it.key = parseKey(val(e)))} />
                  <SelectBox label="Antwortsatz" mono bind:value={it.answers} options={[{ value: null, label: "(Standard)" }, ...setOptions]} />
                  {@render tools(sec.items, i)}
                </div>
              {/each}
              {@render add("Frage", () => sec.items.push({ n: null, id: null, text: "", answers: null, scale: null, key: null }))}
            </div>
          </section>
        {/each}
        {@render add("Abschnitt", () => d.sections.push({ title: "", intro: null, items: [] }))}
      </Card.Content>
    </Card.Root>

    {#snippet result()}
      {@render head(cols.result, ["ID", ""])}
      {#each d.result as _, i (i)}
        <div class="grid {cols.result} gap-2">
          <Input aria-label="ID" class="font-mono" bind:value={d.result[i]} />
          {@render tools(d.result, i)}
        </div>
      {/each}
      {@render add("Zeile", () => d.result.push(""))}
    {/snippet}
    {@render card("result", "Ergebnis-Reihenfolge", null, result)}

    {#snippet rules()}
      {@render head(cols.rules, ["ID", "Beschriftung", "Bedingung", ""])}
      {#each d.rules as r, i (r)}
        <div class="grid {cols.rules} gap-2">
          <Input aria-label="ID" class="font-mono" bind:value={r.id} />
          <Input aria-label="Beschriftung" bind:value={r.label} />
          <Input aria-label="Bedingung" placeholder="Bedingung" class="font-mono" bind:value={r.when} />
          {@render tools(d.rules, i)}
        </div>
      {/each}
      {@render add("Regel", () => d.rules.push({ id: "", label: "", when: "" }))}
    {/snippet}
    {@render card("rules", "Regeln", null, rules)}

    {#snippet validity()}
      {@render head(cols.cond, ["Muss gelten", "Text, wenn nicht", ""])}
      {#each d.validity ?? [] as v, i (v)}
        <div class="grid {cols.cond} gap-2">
          <Input aria-label="Muss gelten" placeholder="Bedingung" class="font-mono" bind:value={v.require} />
          <Input aria-label="Text, wenn nicht" placeholder="… – nicht auswertbar." bind:value={v.invalid} />
          {@render tools(d.validity!, i)}
        </div>
      {/each}
      {@render add("Kriterium", () => (d.validity ??= []).push({ require: "", invalid: "" }))}
    {/snippet}
    {@render card("validity", "Gültigkeit", "Bedingungen, die alle gelten müssen, z. B. missing <= 6. Gilt eine nicht, gibt es keinen Befund und keine Normwerte; stattdessen steht ihr Text da (braucht Kreuzchen 0.7.0; beim Speichern wird min_app automatisch gesetzt).", validity)}

    {#snippet caveats()}
      {@render head(cols.cond, ["Bedingung", "Text", ""])}
      {#each d.caveats as c, i (c)}
        <div class="grid {cols.cond} gap-2">
          <Input aria-label="Bedingung" placeholder="Bedingung" class="font-mono" bind:value={c.when} />
          <Input aria-label="Text" bind:value={c.text} />
          {@render tools(d.caveats, i)}
        </div>
      {/each}
      {@render add("Warnung", () => d.caveats.push({ when: "", text: "" }))}
    {/snippet}
    {@render card("caveats", "Warnungen", "Stehen über dem Befund, wenn ihre Bedingung zutrifft.", caveats)}

    {#snippet verdict()}
      {@render head(cols.cond, ["Bedingung", "Text", ""])}
      {#each d.verdict as v, i (v)}
        <div class="grid {cols.cond} gap-2">
          {#if "else" in v}
            <span class="flex h-9 items-center px-1 text-muted-foreground">sonst</span>
            <Input aria-label="Text" bind:value={v.else} />
          {:else}
            <Input aria-label="Bedingung" placeholder="Bedingung" class="font-mono" bind:value={v.when} />
            <Input aria-label="Text" bind:value={v.text} />
          {/if}
          {@render tools(d.verdict, i)}
        </div>
      {/each}
      <div class="flex gap-2">
        {@render add("Befund", () => d.verdict.push({ when: "", text: "" }))}
        {@render add("Sonst-Befund", () => d.verdict.push({ else: "" }))}
      </div>
    {/snippet}
    {@render card("verdict", "Befund (der erste zutreffende gilt)", null, verdict)}
  {/if}
</div>

<script lang="ts">
  import ArrowDown from "@lucide/svelte/icons/arrow-down";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import FileText from "@lucide/svelte/icons/file-text";
  import Info from "@lucide/svelte/icons/info";
  import Play from "@lucide/svelte/icons/play";
  import Plus from "@lucide/svelte/icons/plus";
  import Save from "@lucide/svelte/icons/save";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import X from "@lucide/svelte/icons/x";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import IconBtn from "./IconBtn.svelte";
  import Sel from "./Sel.svelte";

  // Static copy of examples/definitionen/beispiel.yaml, as far as this form shows it. Nothing is saved.
  const d = $state({
    id: "beispiel", version: "1", title: "Beispiel-Fragebogen", authors: "", copyright: "", cutoff_compare: ">=" as string | null,
    answer_sets: [
      { name: "haeufigkeit", labels: "nie, selten, oft, immer", keys: "" },
      { name: "ja_nein", labels: "Nein, Ja", keys: "nj" },
    ],
    default_answers: "haeufigkeit" as string | null,
    fields: [
      { id: "kennung", label: "Kennung", type: "text" as string | null, default: "anonym", checked: false },
      { id: "alter", label: "Alter", type: "number" as string | null, default: "", checked: false },
      { id: "erwachsen", label: "Volljährig", type: "checkbox" as string | null, default: "", checked: false },
    ],
    scales: [
      { id: "X", label: "Skala X", cutoff: 4 },
      { id: "Y", label: "Skala Y", cutoff: 3 },
    ],
    sections: [
      { title: "Teil 1", intro: "Beispieltext.", items: [
        { ref: "1", text: "Frage eins?", scale: "X" as string | null, key: "normal", answers: null as string | null },
        { ref: "2", text: "Frage zwei?", scale: "X" as string | null, key: "umgepolt", answers: null as string | null },
        { ref: "3", text: "Frage drei?", scale: "Y" as string | null, key: "0, 0, 1, 2", answers: null as string | null },
      ] },
      { title: "Teil 2", intro: "", items: [
        { ref: "4", text: "Frage vier?", scale: "Y" as string | null, key: "normal", answers: null as string | null },
        { ref: "zusatz", text: "Zusatzfrage", scale: null as string | null, key: "", answers: "ja_nein" as string | null },
      ] },
    ],
  });

  let mode = $state<"form" | "yaml">("form");
  let confirmDelete = $state(false);

  const move = (list: unknown[], i: number, by: number) => list.splice(i + by, 0, ...list.splice(i, 1));
  const types = [
    { value: "text", label: "Text" }, { value: "number", label: "Zahl" },
    { value: "date", label: "Datum" }, { value: "checkbox", label: "Ankreuzfeld" },
  ];
  const setOptions = $derived(d.answer_sets.map((s) => ({ value: s.name, label: s.name })));
  const scaleOptions = $derived([{ value: null, label: "keine" }, ...d.scales.map((s) => ({ value: s.id, label: s.id }))]);

  const lbl = "text-[14px] font-bold";

  // One grid template per table, shared by its header and rows so the columns line up.
  const cols = {
    sets: "grid-cols-[11rem_minmax(0,1fr)_7rem_2.25rem]",
    fields: "grid-cols-[10rem_minmax(0,1fr)_9.5rem_10rem_7.5rem]",
    scales: "grid-cols-[10rem_minmax(0,1fr)_7rem_7.5rem]",
    items: "grid-cols-[4.5rem_minmax(0,1fr)_7rem_8.5rem_9.5rem_7.5rem]",
  };
</script>

{#snippet tools(list: unknown[], i: number, what: string)}
  <div class="flex gap-0.5 justify-end">
    <IconBtn label="Nach oben" icon={ArrowUp} disabled={i === 0} onclick={() => move(list, i, -1)} />
    <IconBtn label="Nach unten" icon={ArrowDown} disabled={i === list.length - 1} onclick={() => move(list, i, 1)} />
    <IconBtn label={`${what} entfernen`} icon={Trash2} variant="destructive" onclick={() => list.splice(i, 1)} />
  </div>
{/snippet}

{#snippet head(grid: string, labels: string[])}
  <div class="grid {grid} gap-2 px-0.5 pb-1 text-[13px] font-bold text-muted-foreground">
    {#each labels as l, i (i)}<span>{l}</span>{/each}
  </div>
{/snippet}

{#snippet add(label: string, onclick: () => void)}
  <Button variant="ghost" class="text-primary hover:text-primary -ml-2.5 self-start" {onclick}><Plus />{label}</Button>
{/snippet}

<Tooltip.Provider delayDuration={300}>
  <div class="min-h-screen bg-background text-foreground font-sans">
    <header class="sticky top-0 z-20 border-b border-border bg-card">
      <div class="mx-auto flex h-14 max-w-[1040px] items-center gap-3 px-6">
        <FileText class="size-5 text-muted-foreground" />
        <span class="font-mono text-[15px]">beispiel.yaml</span>
        <Badge variant="secondary" class="h-6 gap-1 px-2 text-[13px]"><CircleCheck />Keine Fehler</Badge>
        <div class="flex-1"></div>
        <div role="group" aria-label="Ansicht" class="flex rounded-lg bg-muted p-0.5">
          {#each [["form", "Formular"], ["yaml", "YAML"]] as [m, label] (m)}
            <button
              type="button"
              aria-pressed={mode === m}
              onclick={() => (mode = m as typeof mode)}
              class="h-8 rounded-md px-3 text-[15px] text-muted-foreground outline-none focus-visible:ring-3 focus-visible:ring-ring/50 aria-pressed:bg-card aria-pressed:font-bold aria-pressed:text-foreground aria-pressed:shadow-sm"
            >{label}</button>
          {/each}
        </div>
        <Separator orientation="vertical" class="mx-1 h-6!" />
        <Button variant="outline"><Play />Testen</Button>
        <Button><Save />Speichern</Button>
        <Button variant="ghost"><X />Schließen</Button>
        <IconBtn label="Fragebogen löschen" icon={Trash2} variant="destructive" onclick={() => (confirmDelete = true)} />
      </div>
    </header>

    <main class="mx-auto flex max-w-[1040px] flex-col gap-4 px-6 py-5">
      <p class="flex items-center gap-2 text-[14px] text-muted-foreground">
        <Info class="size-4" />Kommentare in der Datei gehen beim Speichern über das Formular verloren.
      </p>

      <Card.Root class="gap-4">
        <Card.Header><Card.Title class="text-[17px] font-bold">Allgemein</Card.Title></Card.Header>
        <Card.Content class="grid grid-cols-[1fr_8rem_14rem] gap-x-4 gap-y-3">
          <div class="grid gap-1.5"><Label for="f-title" class={lbl}>Titel</Label><Input id="f-title" bind:value={d.title} /></div>
          <div class="grid gap-1.5"><Label for="f-version" class={lbl}>Version</Label><Input id="f-version" class="font-mono" bind:value={d.version} /></div>
          <div class="grid gap-1.5"><Label for="f-id" class={lbl}>ID</Label><Input id="f-id" class="font-mono" bind:value={d.id} /></div>
          <div class="col-span-2 grid gap-1.5"><Label for="f-authors" class={lbl}>Autoren</Label><Input id="f-authors" placeholder="optional" bind:value={d.authors} /></div>
          <div class="grid gap-1.5"><Label class={lbl}>„über Cut-off“ heißt</Label>
            <Sel label="„über Cut-off“ heißt" bind:value={d.cutoff_compare} options={[{ value: ">=", label: "≥  größer oder gleich" }, { value: ">", label: ">  größer" }]} />
          </div>
          <div class="col-span-3 grid gap-1.5"><Label for="f-copyright" class={lbl}>Copyright</Label><Input id="f-copyright" placeholder="optional" bind:value={d.copyright} /></div>
        </Card.Content>
      </Card.Root>

      <Card.Root class="gap-3">
        <Card.Header>
          <Card.Title class="text-[17px] font-bold">Antwortsätze</Card.Title>
          <Card.Description class="text-[14px]">Die Antworten, die bei einer Frage angekreuzt werden können, in dieser Reihenfolge.</Card.Description>
        </Card.Header>
        <Card.Content class="flex flex-col gap-2">
          {@render head(cols.sets, ["Name", "Antworten, mit Komma getrennt", "Tasten", ""])}
          {#each d.answer_sets as s, i (s)}
            <div class="grid {cols.sets} gap-2">
              <Input aria-label="Name" class="font-mono" bind:value={s.name} />
              <Input aria-label="Antworten" bind:value={s.labels} />
              <Input aria-label="Tasten" class="font-mono" placeholder={s.labels.split(",").map((_, j) => j).join("")} bind:value={s.keys} />
              <IconBtn label="Antwortsatz entfernen" icon={Trash2} variant="destructive" onclick={() => d.answer_sets.splice(i, 1)} />
            </div>
          {/each}
          <div class="mt-1 flex items-center justify-between">
            {@render add("Antwortsatz", () => d.answer_sets.push({ name: "antworten", labels: "Nein, Ja", keys: "" }))}
            <div class="flex items-center gap-3">
              <Label class={lbl}>Standard-Antwortsatz</Label>
              <Sel label="Standard-Antwortsatz" class="w-44" mono bind:value={d.default_answers} options={[{ value: null, label: "keiner" }, ...setOptions]} />
            </div>
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root class="gap-3" id="kopffelder">
        <Card.Header>
          <Card.Title class="text-[17px] font-bold">Kopffelder</Card.Title>
          <Card.Description class="text-[14px]">Felder über den Fragen, z. B. Kennung oder Alter.</Card.Description>
        </Card.Header>
        <Card.Content class="flex flex-col gap-2">
          {@render head(cols.fields, ["ID", "Beschriftung", "Typ", "Vorgabe", ""])}
          {#each d.fields as f, i (f)}
            <div class="grid {cols.fields} gap-2">
              <Input aria-label="ID" class="font-mono" bind:value={f.id} />
              <Input aria-label="Beschriftung" bind:value={f.label} />
              <Sel label="Typ" bind:value={f.type} options={types} />
              {#if f.type === "checkbox"}
                <Label class="flex h-9 items-center gap-2 px-1 text-[15px] font-normal"><Checkbox bind:checked={f.checked} />angekreuzt</Label>
              {:else}
                <Input aria-label="Vorgabe" placeholder="keine Vorgabe" bind:value={f.default} />
              {/if}
              {@render tools(d.fields, i, "Feld")}
            </div>
          {/each}
          {@render add("Feld", () => d.fields.push({ id: "", label: "", type: "text", default: "", checked: false }))}
        </Card.Content>
      </Card.Root>

      <Card.Root class="gap-3">
        <Card.Header>
          <Card.Title class="text-[17px] font-bold">Skalen</Card.Title>
          <Card.Description class="text-[14px]">Summen über Fragen. Ab dem Cut-off gilt der Wert als auffällig.</Card.Description>
        </Card.Header>
        <Card.Content class="flex flex-col gap-2">
          {@render head(cols.scales, ["ID", "Beschriftung", "Cut-off", ""])}
          {#each d.scales as s, i (s)}
            <div class="grid {cols.scales} gap-2">
              <Input aria-label="ID" class="font-mono" bind:value={s.id} />
              <Input aria-label="Beschriftung" bind:value={s.label} />
              <Input aria-label="Cut-off" type="number" class="font-mono tabular-nums text-right" bind:value={s.cutoff} />
              {@render tools(d.scales, i, "Skala")}
            </div>
          {/each}
          {@render add("Skala", () => d.scales.push({ id: "", label: "", cutoff: 0 }))}
        </Card.Content>
      </Card.Root>

      <Card.Root class="gap-3">
        <Card.Header>
          <Card.Title class="text-[17px] font-bold">Abschnitte und Fragen</Card.Title>
          <Card.Description class="text-[14px]">Nr. ist die Nummer im Papierbogen; Fragen ohne Nummer brauchen eine ID.</Card.Description>
        </Card.Header>
        <Card.Content class="flex flex-col gap-4">
          {#each d.sections as sec, si (sec)}
            {#if si > 0}<Separator />{/if}
            <section class="flex flex-col gap-2">
              <div class="grid grid-cols-[minmax(0,1fr)_7.5rem] gap-2">
                <Input aria-label="Abschnittstitel" class="font-bold" bind:value={sec.title} />
                {@render tools(d.sections, si, "Abschnitt")}
              </div>
              <Input aria-label="Einleitung" placeholder="Einleitung (optional)" bind:value={sec.intro} />
              <div class="mt-1 flex flex-col gap-2 border-l-2 border-secondary pl-3">
                {@render head(cols.items, ["Nr. / ID", "Fragetext", "Skala", "Schlüssel", "Antwortsatz", ""])}
                {#each sec.items as it, i (it)}
                  <div class="grid {cols.items} gap-2">
                    <Input aria-label="Nr. oder ID" class="font-mono tabular-nums" bind:value={it.ref} />
                    <Input aria-label="Fragetext" bind:value={it.text} />
                    <Sel label="Skala" mono bind:value={it.scale} options={scaleOptions} />
                    <Input aria-label="Schlüssel oder Punkte" placeholder="—" class="font-mono" bind:value={it.key} />
                    <Sel label="Antwortsatz" mono bind:value={it.answers} options={[{ value: null, label: "Standard" }, ...setOptions]} />
                    {@render tools(sec.items, i, "Frage")}
                  </div>
                {/each}
                {@render add("Frage", () => sec.items.push({ ref: "", text: "", scale: null, key: "", answers: null }))}
              </div>
            </section>
          {/each}
          {@render add("Abschnitt", () => d.sections.push({ title: "", intro: "", items: [] }))}
        </Card.Content>
      </Card.Root>
    </main>
  </div>

  <Dialog.Root bind:open={confirmDelete}>
    <Dialog.Content class="sm:max-w-md gap-5">
      <Dialog.Header>
        <Dialog.Title class="text-[17px] font-bold">Fragebogen löschen?</Dialog.Title>
        <Dialog.Description class="text-[15px] leading-relaxed">
          Die Datei <span class="font-mono text-foreground">beispiel.yaml</span> wird gelöscht. Gespeicherte Ergebnisse bleiben erhalten, lassen sich aber nicht mehr neu auswerten.
        </Dialog.Description>
      </Dialog.Header>
      <Dialog.Footer>
        <Button variant="outline" onclick={() => (confirmDelete = false)}>Abbrechen</Button>
        <Button class="bg-destructive text-background hover:bg-destructive/90" onclick={() => (confirmDelete = false)}><Trash2 />Löschen</Button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>
</Tooltip.Provider>

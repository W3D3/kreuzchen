<script lang="ts">
  import { onMount } from "svelte";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { entryItems } from "../lib/input";
  import { entryKey } from "../lib/keyboard";
  import type { Definition } from "../lib/types";

  let { answers = $bindable(), ongraded }: { answers: Record<string, number>; ongraded: () => void } = $props();

  // examples/definitionen/beispiel_normen.yaml as parsed, plus an age field and a yes/no item with keys n/j. Invented.
  const def: Definition = {
    schema: 1, id: "beispiel_normen", version: "1", title: "Beispiel mit Normwerten",
    authors: null, copyright: null, cutoff_compare: ">=",
    answer_sets: { haeufigkeit: ["nie", "manchmal", "oft", "immer"], ja_nein: ["Nein", "Ja"] },
    shortcuts: { ja_nein: "nj" },
    keys: { normal: [0, 1, 2, 3], umgepolt: [3, 2, 1, 0] },
    default_answers: "haeufigkeit",
    fields: [
      { id: "kennung", label: "Kennung", type: "text", default: "anonym" },
      { id: "alter", label: "Alter", type: "number", default: null },
    ],
    scales: [{ id: "A", label: "Skala A", cutoff: null }, { id: "B", label: "Skala B", cutoff: null }],
    derived: [{ id: "gesamt", label: "Gesamtrohwert", value: "A + B", cutoff: 10 }],
    sections: [
      { title: "Fragen", intro: "Wie oft trifft das in den letzten zwei Wochen zu?", items: [
        { n: 1, id: null, scale: "A", key: "normal", answers: null, text: "Frage eins?" },
        { n: 2, id: null, scale: "A", key: "umgepolt", answers: null, text: "Frage zwei?" },
        { n: 3, id: null, scale: "A", key: "normal", answers: null, text: "Frage drei?" },
        { n: 4, id: null, scale: "B", key: "normal", answers: null, text: "Frage vier?" },
        { n: 5, id: null, scale: "B", key: "normal", answers: null, text: "Frage fünf?" },
        { n: 6, id: null, scale: "B", key: "umgepolt", answers: null, text: "Frage sechs?" },
      ] },
      { title: "Zusatz", intro: null, items: [
        { n: null, id: "zusatz", scale: null, key: null, answers: "ja_nein", text: "Wurde der Bogen selbst ausgefüllt?" },
      ] },
    ],
    result: ["gesamt", "A", "B"], rules: [], caveats: [], verdict: [],
  };

  // Everything from here to the markup is Entry.svelte's logic; only grading is faked.
  const items = entryItems(def);
  let fields = $state<Record<string, boolean | number | string>>({ kennung: "K-2611-0842" });
  let focused = $state(0);
  let tried = $state(false);
  let banner = $state<string | null>(null);
  let els: HTMLElement[] = [];
  onMount(() => els[0]?.focus());

  const scored = items.filter((i) => i.scored);
  const answered = $derived(scored.filter((i) => answers[i.key] !== undefined).length);
  const shortcuts = $derived(items[focused]?.shortcuts.filter(Boolean) ?? []);

  function focusItem(i: number) {
    focused = i;
    els[i]?.focus();
    els[i]?.scrollIntoView({ block: "nearest" });
  }

  // shortcut: stands in for the engine; refuses more than one gap so the error state can be shown.
  function doGrade() {
    tried = true;
    banner = null;
    const gaps = scored.filter((i) => answers[i.key] === undefined).map((i) => i.key);
    if (gaps.length > 1) banner = `Nicht auswertbar: ${gaps.length} Fragen ohne Antwort (${gaps.join(", ")}).`;
    else ongraded();
  }

  function cancel() {
    if (Object.keys(answers).length === 0 || confirm("Eingaben verwerfen?")) for (const k in answers) delete answers[k];
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.altKey || e.metaKey) return;
    if (e.ctrlKey && e.key !== "Enter") return;
    const item = items[focused];
    const a = entryKey(
      { key: e.key, shift: e.shiftKey, ctrl: e.ctrlKey },
      { focused, count: items.length, shortcuts: item.shortcuts, current: answers[item.key] ?? null },
    );
    if (a.type === "none") return;
    // At either end Tab leaves the list, so header fields and buttons stay reachable.
    if (e.key === "Tab" && a.type === "focus" && a.to === focused) return;
    e.preventDefault();
    if (a.type === "answer") {
      answers[item.key] = a.index;
      if (item.shortcuts.includes(e.key)) focusItem(Math.min(focused + 1, items.length - 1));
    } else if (a.type === "clear") delete answers[item.key];
    else if (a.type === "focus") focusItem(a.to);
    else void doGrade();
  }

  const hints = $derived([
    [shortcuts, "Antwort"], [["←", "→"], "Antwort ändern"], [["Enter"], "nächste Frage"],
    [["⇧", "Tab"], "zurück"], [["Entf"], "löschen"], [["Strg", "Enter"], "Auswerten"],
  ] as [string[], string][]);
</script>

{#snippet kbd(k: string, cls = "")}
  <kbd class="inline-grid h-5 min-w-5 place-items-center rounded-[4px] border border-current/25 px-1 font-mono text-[12px]! leading-none {cls}">{k}</kbd>
{/snippet}

<div class="min-h-screen bg-background pb-16 font-sans text-foreground">
  <header class="sticky top-0 z-20 border-b border-border bg-card">
    <div class="mx-auto flex h-16 max-w-[880px] items-center gap-4 px-6">
      <div class="flex flex-col gap-1.5">
        <span class="text-[17px]"><span class="font-mono font-bold tabular-nums">{answered} / {scored.length}</span> beantwortet</span>
        <div class="flex gap-1" aria-hidden="true">
          {#each scored as s (s.key)}
            {@const gap = answers[s.key] === undefined}
            <span class="h-1.5 w-7 rounded-full {gap ? (tried ? 'bg-over/70' : 'bg-border') : 'bg-primary'}"></span>
          {/each}
        </div>
      </div>
      <div class="flex-1"></div>
      <Button variant="ghost" class="h-11 px-4" onclick={cancel}>Abbrechen</Button>
      <Button class="h-11 px-5 font-bold" onclick={doGrade}>Auswerten</Button>
    </div>
    {#if banner}
      <p role="alert" class="mx-auto mb-3 flex max-w-[832px] items-center gap-2 rounded-lg border border-over/40 bg-over-soft px-4 py-2.5 text-[15px]">
        <TriangleAlert class="size-4 shrink-0 text-over" />{banner}
      </p>
    {/if}
  </header>

  <main class="mx-auto flex max-w-[880px] flex-col gap-5 px-6 py-6">
    <h1 class="text-[26px] leading-tight font-bold">{def.title}</h1>

    <Card.Root class="py-4">
      <Card.Content class="flex flex-wrap gap-4">
        {#each def.fields as f (f.id)}
          <div class="grid flex-[1_1_14rem] gap-1.5">
            <Label for="fld-{f.id}" class="text-[14px] font-bold">{f.id === "kennung" ? "Kennung (keine Patientennamen)" : f.label}</Label>
            {#if f.type === "number"}
              <Input id="fld-{f.id}" type="number" class="h-11 font-mono" bind:value={fields[f.id] as number} />
            {:else}
              <Input id="fld-{f.id}" class="h-11 font-mono" bind:value={fields[f.id] as string} />
            {/if}
          </div>
        {/each}
      </Card.Content>
    </Card.Root>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="flex flex-col gap-2.5" {onkeydown}>
      {#each items as it, i (it.key)}
        {#if i === 0 || it.section !== items[i - 1].section}
          {@const sec = def.sections[it.section]}
          <div class="mt-3 mb-0.5">
            <h2 class="text-[19px] font-bold">{sec.title}</h2>
            {#if sec.intro}<p class="text-[15px] text-muted-foreground">{sec.intro}</p>{/if}
          </div>
        {/if}
        {@const missing = tried && it.scored && answers[it.key] === undefined}
        <div
          class="flex scroll-my-28 gap-3 rounded-xl border bg-card px-5 py-4 outline-none focus-within:outline-3 focus-within:outline-solid focus-within:outline-offset-2 focus-within:outline-primary
            {missing ? 'border-l-[6px] border-l-over pl-[15px]' : ''}"
          role="radiogroup"
          aria-invalid={missing ? "true" : undefined}
          aria-label={it.text}
          tabindex={i === focused ? 0 : -1}
          bind:this={els[i]}
          onfocus={() => (focused = i)}
        >
          <span class="min-w-8 pt-0.5 text-right font-mono text-muted-foreground tabular-nums">{it.key}</span>
          <div class="flex flex-1 flex-col items-start gap-3">
            <div class="flex w-full items-baseline gap-3">
              <span class="flex-1 text-[18px]">{it.text}</span>
              {#if missing}<span class="text-[14px] font-bold text-over">Antwort fehlt</span>{/if}
            </div>
            <div class="inline-flex flex-wrap gap-1 rounded-lg bg-muted p-1">
              {#each it.options as o, j (j)}
                {@const sel = answers[it.key] === j}
                <button
                  type="button"
                  role="radio"
                  aria-checked={sel}
                  tabindex="-1"
                  class="flex h-12 min-w-28 items-center gap-2.5 rounded-md px-4 text-[16px]
                    {sel ? 'bg-primary font-bold text-primary-foreground shadow-sm' : 'text-foreground hover:bg-card'}"
                  onclick={() => { answers[it.key] = j; focusItem(i); }}
                >
                  {#if it.shortcuts[j]}{@render kbd(it.shortcuts[j], sel ? "" : "text-muted-foreground")}{/if}{o}
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/each}
    </div>
  </main>

  <footer class="fixed inset-x-0 bottom-0 z-20 border-t border-border bg-card">
    <div class="mx-auto flex h-11 max-w-[1040px] items-center justify-center gap-5 px-6 text-[14px] text-muted-foreground">
      {#each hints as [keys, label] (label)}
        <span class="flex items-center gap-1">{#each keys as k (k)}{@render kbd(k)}{/each}<span class="ml-0.5">{label}</span></span>
      {/each}
    </div>
  </footer>
</div>

<script lang="ts">
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import Check from "@lucide/svelte/icons/check";
  import Copy from "@lucide/svelte/icons/copy";
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import Info from "@lucide/svelte/icons/info";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Save from "@lucide/svelte/icons/save";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import X from "@lucide/svelte/icons/x";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
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
  // Same notation as the engine's copy text: `55`, `3,5`, `3,67`.
  const num = (v: number) => v.toLocaleString("de-DE", { maximumFractionDigits: 2, useGrouping: false });
  // Index into the --band-N colours; any other label gets a neutral chip.
  const bands = ["unauffällig", "mild", "moderat", "schwer"];
  const cols = "grid-cols-[minmax(0,1fr)_5.5rem_13rem_5.5rem_7.5rem]";
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

<!-- Full-bleed bar inside the centred column, as on the entry screen. -->
<header class="sticky top-0 z-20 -mx-6 -mt-6 mb-6 bg-card px-6 [clip-path:inset(0_-100vmax_-1px)] shadow-[0_0_0_100vmax_var(--surface),0_1px_0_100vmax_var(--line)]">
  <div class="flex min-h-16 flex-wrap items-center gap-2 py-2.5">
    {#if opened}<Button variant="ghost" class="-ml-2.5 h-11 px-3" onclick={onback}><ArrowLeft />Zurück zu den Antworten</Button>{/if}
    <div class="flex-1"></div>
    <span aria-live="polite" class="mr-2 text-[14px] text-muted-foreground">{status}</span>
    <Button variant="outline" class="h-11 px-4" onclick={copy}><Copy />Als Text kopieren</Button>
    {#if opened && !preview}<Button variant="outline" class="h-11 px-4" disabled={saved} onclick={save}><Save />Speichern</Button>{/if}
    <Button class="h-11 px-4 font-bold" onclick={onnew}><FilePlus />{preview ? "Zurück zum Editor" : "Neuer Fragebogen"}</Button>
  </div>
  {#if banner}
    <p role="alert" class="mb-3 flex items-center gap-2 rounded-lg border border-over/40 bg-over-soft px-4 py-2.5 text-[15px]">
      <TriangleAlert class="size-4 shrink-0 text-over" />{banner}
    </p>
  {/if}
</header>

<div class="flex flex-col gap-5 pb-16">
  {#if notice}
    <div class="flex flex-wrap items-center gap-3 rounded-xl border border-primary/40 bg-secondary px-5 py-3 text-[15px]">
      <Info class="size-4.5 shrink-0 text-primary" /><p class="min-w-0 flex-1">{notice}</p>
      {#if onregrade}<Button variant="outline" class="h-11 px-4" onclick={onregrade}><RefreshCw />Neu auswerten</Button>{/if}
    </div>
  {/if}

  <div>
    {#if opened}<p class="text-[15px] text-muted-foreground">{opened.definition.title}</p>{/if}
    <h1 class="text-[26px] leading-tight font-bold">Ergebnis</h1>
    <p class="mt-1 text-[16px]">
      {#each g.fields as f, i (f.id)}{#if i > 0}<span class="text-muted-foreground">{" · "}</span>{/if}{#if f.id === "kennung"}<strong class="font-mono">{f.value}</strong>{:else}<span class="text-muted-foreground">{f.label}: {f.value}</span>{/if}{/each}
    </p>
  </div>

  <section class="flex gap-5 rounded-xl border px-5 py-4 {anyMet ? 'border-over/45 bg-over-soft' : 'bg-card'}">
    {#if g.verdict}
      <div class="w-44 shrink-0">
        <p class="text-[14px] text-muted-foreground">Gesamturteil</p>
        <p class="text-[26px] leading-tight font-bold">{g.verdict}</p>
      </div>
    {/if}
    <ul class="flex flex-1 flex-col gap-1.5 text-[16px] {g.verdict ? 'border-l border-current/15 pl-5' : ''}">
      {#each g.rules as r (r.id)}
        <li class="flex items-center gap-2 {r.met ? '' : 'text-muted-foreground'}">
          {#if r.met}<Check class="size-4.5 shrink-0 text-over" strokeWidth={3} aria-label="erfüllt" />{:else}<X class="size-4.5 shrink-0" aria-label="nicht erfüllt" />{/if}
          <span class:font-bold={r.met}>{r.label}</span>
        </li>
      {/each}
      <li class="text-[15px] text-muted-foreground {g.rules.length ? 'mt-1' : ''}">
        Fehlende Antworten: <span class="font-mono text-foreground">{g.missing}</span>{g.missing > 0 ? ` (Fragen ${g.missing_items.join(", ")})` : ""}
      </li>
    </ul>
  </section>

  <Card.Root class="gap-0 py-2">
    <Card.Content>
      <div class="grid {cols} items-center gap-x-4 border-b py-2 text-[13px] font-bold text-muted-foreground">
        <span>Skala</span><span class="text-right">Rohwert</span><span>Cut-off</span><span class="text-right">Normwert</span><span>Einordnung</span>
      </div>
      {#each g.rows as r (r.id)}
        <div class="grid {cols} min-h-14 items-center gap-x-4 border-b py-2.5 last:border-b-0">
          <span class="text-[17px]">{r.label}</span>
          {#if r.kind === "sum"}
            <span class="text-right font-mono tabular-nums">
              <span class="text-[17px]! font-bold">{num(r.value)}</span>{#if r.cutoff !== null}<span class="text-muted-foreground">{` / ${r.cutoff}`}</span>{/if}
            </span>
            {#if r.cutoff}
              <div class="flex items-center gap-2.5">
                <div class="relative h-2 w-20 shrink-0 rounded-full bg-muted" aria-hidden="true">
                  <div class="h-full rounded-full {r.over ? 'bg-over' : 'bg-primary'}" style:width="{meter(r.value, r.cutoff)}%"></div>
                  <div class="absolute inset-y-[-4px] left-1/2 w-0.5 bg-foreground"></div>
                </div>
                {#if r.over}<span class="rounded-full border border-over/50 bg-over-soft px-2.5 py-0.5 text-[13px] font-bold whitespace-nowrap text-over">über Cut-off</span>{/if}
              </div>
            {:else}
              <span class="text-muted-foreground">–</span>
            {/if}
            <span class="text-right font-mono whitespace-nowrap tabular-nums">{r.norm ? `${r.norm.name} = ${r.norm.value}` : "–"}</span>
            {#if r.norm?.band}
              {@const b = bands.indexOf(r.norm.band)}
              <span
                class="justify-self-start rounded-full px-2.5 py-0.5 text-[14px] font-bold {b < 0 ? 'bg-muted text-foreground' : ''}"
                style:background={b < 0 ? null : `var(--band-${b})`}
                style:color={b < 0 ? null : `var(--band-${b}-fg)`}>{r.norm.band}</span>
            {:else}<span></span>{/if}
          {:else}
            <span class="col-span-4 text-[17px] font-bold">{r.answer ?? "–"}</span>
          {/if}
        </div>
      {/each}
    </Card.Content>
  </Card.Root>

  {#each g.caveats as c (c)}
    <p class="flex items-center gap-2.5 rounded-xl border border-dashed border-muted-foreground/50 px-5 py-3 text-[15px]">
      <Info class="size-4.5 shrink-0 text-muted-foreground" />Hinweis: {c}
    </p>
  {/each}
</div>

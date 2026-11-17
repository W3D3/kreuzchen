<script lang="ts">
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import Check from "@lucide/svelte/icons/check";
  import Copy from "@lucide/svelte/icons/copy";
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import Info from "@lucide/svelte/icons/info";
  import Save from "@lucide/svelte/icons/save";
  import X from "@lucide/svelte/icons/x";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import type { Grading } from "../lib/types";

  let { onback, onnew }: { onback: () => void; onnew: () => void } = $props();

  // Invented grading for beispiel_normen, with a four-step band table.
  const g: Grading = {
    fields: [
      { id: "kennung", label: "Kennung", value: "K-2611-0842" },
      { id: "alter", label: "Alter", value: "34" },
    ],
    rows: [
      { kind: "sum", id: "gesamt", label: "Gesamtrohwert", value: 13, cutoff: 10, over: true, norm: { name: "T", value: 66, band: "moderat" } },
      { kind: "sum", id: "A", label: "Skala A", value: 8, cutoff: 6, over: true, norm: { name: "T", value: 70, band: "schwer" } },
      { kind: "sum", id: "B", label: "Skala B", value: 5, cutoff: 6, over: false, norm: { name: "T", value: 56, band: "mild" } },
      { kind: "sum", id: "C", label: "Skala C", value: 2, cutoff: null, over: false, norm: { name: "T", value: 41, band: "unauffällig" } },
      { kind: "answer", id: "zusatz", label: "Bogen selbst ausgefüllt", answer: "Ja" },
    ],
    rules: [
      { id: "r1", label: "Gesamtrohwert über Cut-off", met: true },
      { id: "r2", label: "Skala A und Skala B über Cut-off", met: false },
    ],
    caveats: ["Skala B: eine fehlende Antwort. Den Wert mit Vorsicht lesen."],
    verdict: "Auffällig",
    missing: 1,
    missing_items: ["6"],
  };

  const bands = ["unauffällig", "mild", "moderat", "schwer"];
  const anyMet = g.rules.some((r) => r.met);
  const num = (v: number) => v.toLocaleString("de-DE", { maximumFractionDigits: 2, useGrouping: false });
  const meter = (value: number, cutoff: number) => Math.min(value / (cutoff * 2), 1) * 100;
  const cols = "grid-cols-[minmax(0,1fr)_5.5rem_13rem_4.5rem_7.5rem]";

  let status = $state("");
  let saved = $state(false);
</script>

<div class="min-h-screen bg-background font-sans text-foreground">
  <header class="sticky top-0 z-20 border-b border-border bg-card">
    <div class="mx-auto flex h-16 max-w-[880px] items-center gap-2 px-6">
      <Button variant="ghost" class="-ml-2.5 h-11 px-3" onclick={onback}><ArrowLeft />Zurück zu den Antworten</Button>
      <div class="flex-1"></div>
      <span aria-live="polite" class="mr-2 text-[14px] text-muted-foreground">{status}</span>
      <Button variant="outline" class="h-11 px-4" onclick={() => (status = "Kopiert.")}><Copy />Als Text kopieren</Button>
      <Button variant="outline" class="h-11 px-4" disabled={saved} onclick={() => { saved = true; status = "Gespeichert."; }}><Save />Speichern</Button>
      <Button class="h-11 px-4 font-bold" onclick={onnew}><FilePlus />Neuer Fragebogen</Button>
    </div>
  </header>

  <main class="mx-auto flex max-w-[880px] flex-col gap-5 px-6 py-6">
    <div>
      <p class="text-[15px] text-muted-foreground">Beispiel mit Normwerten</p>
      <h1 class="text-[26px] leading-tight font-bold">Ergebnis</h1>
      <p class="mt-1 text-[16px]">
        {#each g.fields as f, i (f.id)}{#if i > 0}<span class="text-muted-foreground">{", "}</span>{/if}{#if f.id === "kennung"}<strong class="font-mono">{f.value}</strong>{:else}<span class="text-muted-foreground">{f.label} {f.value}</span>{/if}{/each}
      </p>
    </div>

    <section class="flex gap-5 rounded-xl border px-5 py-4 {anyMet ? 'border-over/45 bg-over-soft' : 'bg-card'}">
      <div class="w-44 shrink-0">
        <p class="text-[14px] text-muted-foreground">Gesamturteil</p>
        <p class="text-[26px] leading-tight font-bold">{g.verdict}</p>
      </div>
      <ul class="flex flex-1 flex-col gap-1.5 border-l border-current/15 pl-5 text-[16px]">
        {#each g.rules as r (r.id)}
          <li class="flex items-center gap-2 {r.met ? '' : 'text-muted-foreground'}">
            {#if r.met}<Check class="size-4.5 text-over" strokeWidth={3} />{:else}<X class="size-4.5" />{/if}
            <span class:font-bold={r.met}>{r.label}</span>
          </li>
        {/each}
        <li class="mt-1 text-[15px] text-muted-foreground">
          Fehlende Antworten: <span class="font-mono text-foreground">{g.missing}</span>{g.missing > 0 ? ` (Frage ${g.missing_items.join(", ")})` : ""}
        </li>
      </ul>
    </section>

    <Card.Root class="gap-0 py-2">
      <Card.Content>
        <div class="grid {cols} items-center gap-x-4 border-b py-2 text-[13px] font-bold text-muted-foreground">
          <span>Skala</span><span class="text-right">Rohwert</span><span>Cut-off</span><span class="text-right">T-Wert</span><span>Einordnung</span>
        </div>
        {#each g.rows as r (r.id)}
          <div class="grid {cols} min-h-14 items-center gap-x-4 border-b py-2.5 last:border-b-0">
            <span class="text-[17px]">{r.label}</span>
            {#if r.kind === "sum"}
              <span class="text-right font-mono tabular-nums">
                <span class="text-[17px]! font-bold">{num(r.value)}</span>{#if r.cutoff !== null}<span class="text-muted-foreground"> / {r.cutoff}</span>{/if}
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
              <span class="text-right font-mono tabular-nums">{r.norm ? r.norm.value : "–"}</span>
              {#if r.norm?.band}
                {@const b = bands.indexOf(r.norm.band)}
                <span class="justify-self-start rounded-full px-2.5 py-0.5 text-[14px] font-bold" style:background="var(--band-{b})" style:color="var(--band-{b}-fg)">{r.norm.band}</span>
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
        <Info class="size-4.5 shrink-0 text-muted-foreground" />{c}
      </p>
    {/each}
  </main>
</div>

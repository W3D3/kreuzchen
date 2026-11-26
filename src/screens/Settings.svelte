<script lang="ts">
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { getVersion } from "@tauri-apps/api/app";
  import Download from "@lucide/svelte/icons/download";
  import LifeBuoy from "@lucide/svelte/icons/life-buoy";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import {
    DEFAULT_PDF_COLORS,
    SHADE_ALPHA,
    autoSaveResult,
    dotContrast,
    pdfColors,
  } from "../lib/settings";
  import { loadTheme, saveTheme, type Theme } from "../lib/theme";
  import type { PdfColors } from "../lib/types";
  import { findUpdate, updates } from "../lib/updates.svelte";

  let { oninstall, onsupport }: { oninstall: () => void; onsupport: () => void } = $props();
  let version = $state<string | null>(null);
  getVersion().then((v) => (version = v), () => {});

  let theme = $state<Theme>(loadTheme());
  const choices: [Theme, string][] = [["system", "Wie das System"], ["light", "Hell"], ["dark", "Dunkel"]];
  let autoSave = $state(autoSaveResult.load());

  let colors = $state<PdfColors>(pdfColors.load());
  const colorChoices: [keyof PdfColors, string][] = [
    ["accent", "Mittlerer Bereich (±1 SD)"],
    ["dot", "Profilpunkte"],
    ["line", "Profillinie"],
    ["over", "Hervorhebung auffällig"],
    ["row", "Zeilenhintergrund"],
  ];
  const isDefault = $derived(colorChoices.every(([key]) => colors[key] === DEFAULT_PDF_COLORS[key]));
  const contrast = $derived(dotContrast(colors));
  function setColor(key: keyof PdfColors, value: string) {
    colors = { ...colors, [key]: value };
    pdfColors.save(colors);
  }
  function resetColors() {
    colors = { ...DEFAULT_PDF_COLORS };
    pdfColors.save(null);
  }
  // Mini profile: three rows of 24, axis from 120 to 300, middle range 180–240, markers joined.
  const points: [number, number][] = [[160, 12], [262, 36], [205, 60]];
</script>

{#snippet preview(gray: boolean)}
  <figure class="flex flex-col gap-1">
    <svg viewBox="0 0 320 72" class="w-80 max-w-full rounded-sm border bg-white {gray ? 'grayscale' : ''}" role="img"
      aria-label={gray ? "Vorschau in Graustufen" : "Vorschau in Farbe"}>
      <rect x="0" y="24" width="320" height="24" fill={colors.row} />
      <rect x="180" y="0" width="60" height="72" fill={colors.accent} fill-opacity={SHADE_ALPHA} />
      <line x1="210" y1="0" x2="210" y2="72" stroke={colors.accent} stroke-width="1" />
      {#each [120, 150, 270, 300] as x (x)}<line x1={x} y1="0" x2={x} y2="72" stroke="#d3dbde" stroke-width="0.6" />{/each}
      <text x="8" y="16" font-size="10" fill="#1d2a2f">Skala 1</text>
      <text x="8" y="40" font-size="10" fill="#1d2a2f">Skala 2</text>
      <text x="8" y="64" font-size="10" fill="#1d2a2f">Skala 3</text>
      <text x="70" y="40" font-size="9" font-weight="bold" fill={colors.over}>auffällig</text>
      <polyline points={points.map((p) => p.join(",")).join(" ")} fill="none" stroke={colors.line} stroke-width="1.4" />
      {#each points as [x, y] (y)}<circle cx={x} cy={y} r="4.4" fill={colors.dot} stroke="#ffffff" stroke-width="1" />{/each}
    </svg>
    <figcaption class="text-[13px] text-muted-foreground">{gray ? "Graustufen" : "Farbe"}</figcaption>
  </figure>
{/snippet}

<div class="flex flex-col gap-5">
  <h1 class="text-[26px] leading-tight font-bold">Einstellungen</h1>

  <Card.Root class="gap-4">
    <Card.Header><Card.Title id="theme-label" class="text-[17px] font-bold">Darstellung</Card.Title></Card.Header>
    <Card.Content>
      <RadioGroup.Root
        bind:value={() => theme, (v) => { theme = v as Theme; saveTheme(theme); }}
        aria-labelledby="theme-label"
        class="flex flex-wrap gap-x-8 gap-y-2"
      >
        {#each choices as [value, label] (value)}
          <div class="flex min-h-11 items-center gap-2.5">
            <RadioGroup.Item {value} id="theme-{value}" class="size-5" />
            <Label for="theme-{value}" class="cursor-pointer text-[15px]">{label}</Label>
          </div>
        {/each}
      </RadioGroup.Root>
    </Card.Content>
  </Card.Root>

  <Card.Root class="gap-4">
    <Card.Header><Card.Title class="text-[17px] font-bold">Auswertung</Card.Title></Card.Header>
    <Card.Content>
      <div class="flex min-h-11 items-center gap-2.5">
        <Checkbox id="auto-save" class="size-5" bind:checked={() => autoSave, (v) => { autoSave = v; autoSaveResult.save(v); }} />
        <Label for="auto-save" class="cursor-pointer text-[15px]">Ergebnis beim Auswerten automatisch speichern</Label>
      </div>
    </Card.Content>
  </Card.Root>

  <Card.Root class="gap-4">
    <Card.Header><Card.Title class="text-[17px] font-bold">PDF-Export</Card.Title></Card.Header>
    <Card.Content class="flex flex-col gap-4 text-[15px]">
      <div class="grid gap-x-8 gap-y-2 sm:grid-cols-2">
        {#each colorChoices as [key, label] (key)}
          <div class="flex min-h-11 items-center gap-2.5">
            <input
              type="color"
              id="pdf-{key}"
              class="h-9 w-12 shrink-0 cursor-pointer rounded-md border bg-transparent p-0.5"
              value={colors[key]}
              oninput={(e) => setColor(key, e.currentTarget.value)}
            />
            <Label for="pdf-{key}" class="cursor-pointer text-[15px]">{label}</Label>
          </div>
        {/each}
      </div>
      <div class="flex flex-wrap gap-4">
        {@render preview(false)}
        {@render preview(true)}
      </div>
      {#if contrast < 3}
        <p role="status" class="text-over">
          Profilpunkte und mittlerer Bereich sind in Graustufen kaum zu unterscheiden (Kontrast
          {contrast.toLocaleString("de-DE", { maximumFractionDigits: 1 })} : 1). Schwarz-Weiß-Ausdrucke werden
          schwer lesbar.
        </p>
      {/if}
      <div>
        <Button variant="outline" class="h-10 px-4" onclick={resetColors} disabled={isDefault}>
          <RotateCcw />Zurücksetzen
        </Button>
      </div>
    </Card.Content>
  </Card.Root>

  <Card.Root class="gap-4">
    <Card.Header><Card.Title class="text-[17px] font-bold">Updates</Card.Title></Card.Header>
    <Card.Content class="flex flex-col gap-3 text-[15px]">
      {#if version}<p>Installierte Version: <span class="font-mono">{version}</span></p>{/if}
      <div class="flex flex-wrap items-center gap-3">
        <Button variant="outline" class="h-10 px-4 disabled:cursor-progress" onclick={findUpdate} disabled={updates.checking || updates.installing}>
          <RefreshCw class={updates.checking ? "animate-spin" : ""} />{updates.checking ? "Suche Updates …" : "Nach Updates suchen"}
        </Button>
        <span role="status" class="text-muted-foreground">
          {#if updates.available}
            Version {updates.available.version} ist verfügbar.
          {:else if updates.result === "current"}
            Kreuzchen ist aktuell.
          {:else if updates.result === "offline"}
            Keine Verbindung zum Update-Server.
          {/if}
        </span>
        {#if updates.available}
          <Button class="h-10 px-4 font-bold disabled:cursor-progress" onclick={oninstall} disabled={updates.installing}>
            <Download />{updates.installing ? "Update wird installiert …" : "Jetzt installieren"}
          </Button>
        {/if}
      </div>
      {#if updates.error}<p role="alert" class="text-destructive">{updates.error}</p>{/if}
    </Card.Content>
  </Card.Root>

  <Card.Root class="gap-4">
    <Card.Header><Card.Title class="text-[17px] font-bold">Support & Feedback</Card.Title></Card.Header>
    <Card.Content class="flex flex-col gap-3 text-[15px]">
      <p>Frage stellen, Fehler melden oder Wünsche schicken – über den verbundenen Bibliotheksserver.</p>
      <div><Button variant="outline" class="h-10 px-4" onclick={onsupport}><LifeBuoy />Support & Feedback öffnen</Button></div>
    </Card.Content>
  </Card.Root>
</div>

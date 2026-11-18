<script lang="ts">
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { getVersion } from "@tauri-apps/api/app";
  import Download from "@lucide/svelte/icons/download";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import { loadAutoSaveResult, saveAutoSaveResult } from "../lib/settings";
  import { loadTheme, saveTheme, type Theme } from "../lib/theme";
  import { findUpdate, updates } from "../lib/updates.svelte";

  let { oninstall }: { oninstall: () => void } = $props();
  let version = $state<string | null>(null);
  getVersion().then((v) => (version = v), () => {});

  let theme = $state<Theme>(loadTheme());
  const choices: [Theme, string][] = [["system", "Wie das System"], ["light", "Hell"], ["dark", "Dunkel"]];
  let autoSave = $state(loadAutoSaveResult());
</script>

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
        <Checkbox id="auto-save" class="size-5" bind:checked={() => autoSave, (v) => { autoSave = v; saveAutoSaveResult(v); }} />
        <Label for="auto-save" class="cursor-pointer text-[15px]">Ergebnis beim Auswerten automatisch speichern</Label>
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
</div>

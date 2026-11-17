<script lang="ts">
  import CloudDownload from "@lucide/svelte/icons/cloud-download";
  import Download from "@lucide/svelte/icons/download";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Play from "@lucide/svelte/icons/play";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Banner from "$lib/components/Banner.svelte";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { dataDir, getDefinition, listDefinitions, openDefinitionsFolder } from "../lib/api";
  import { initialInput } from "../lib/input";
  import type { DefinitionEntry, Input, OpenedDefinition } from "../lib/types";

  let { onstart, onregistry, onupdate }: { onstart: (opened: OpenedDefinition, input: Input) => void; onregistry: () => void; onupdate: () => void } = $props();

  let entries = $state<DefinitionEntry[] | null>(null);
  let dir = $state("");
  let banner = $state<string | null>(null);

  async function load() {
    banner = null;
    try {
      [entries, dir] = await Promise.all([listDefinitions(), dataDir()]);
    } catch (e) {
      banner = String(e);
    }
  }
  load();

  async function start(e: DefinitionEntry) {
    try {
      const opened = await getDefinition(e.file, e.locked);
      onstart(opened, initialInput(opened.definition, new Date()));
    } catch (e) {
      banner = String(e);
    }
  }
</script>

<div class="flex flex-col gap-5 pb-16">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <h1 class="text-[26px] leading-tight font-bold">Fragebögen</h1>
    <Button variant="outline" class="h-11 px-4" onclick={onregistry}><CloudDownload />Fragebögen aus der Registry</Button>
  </div>
  {#if banner}
    <Banner>{banner}</Banner>
  {/if}
  {#if entries?.length === 0}
    <p class="rounded-xl border border-dashed px-6 py-10 text-center text-[16px] text-muted-foreground">Noch keine Fragebögen installiert. Bitte wenden Sie sich an den Support.</p>
  {:else if entries}
    <Card.Root class="gap-0 py-0">
      <ul class="divide-y">
        {#each entries as e (e.locked + e.file + e.hash + (e.error ?? ""))}
          <li class="flex items-center gap-4 px-5 py-4">
            <div class="min-w-0 flex-1">
              {#if e.summary}
                <p class="flex flex-wrap items-center gap-2 text-[18px] font-bold">
                  {e.summary.title}{#if e.locked}<Badge variant="outline" class="h-6 px-2 text-[13px] text-muted-foreground">Registry</Badge>{/if}
                </p>
                <p class="mt-0.5 text-[15px] text-muted-foreground">
                  {e.summary.item_count} Fragen · {e.summary.scale_count} Skalen · Version {e.summary.version}<span class="tech"> · <span class="font-mono">{e.file}</span></span>
                </p>
              {:else if e.needs_app}
                <p class="flex items-center gap-2 text-[18px] font-bold"><Download class="size-4.5 shrink-0 text-primary" />Update nötig</p>
                <p class="mt-0.5 text-[15px] text-muted-foreground">Dieser Fragebogen benötigt Kreuzchen {e.needs_app} oder neuer.</p>
                <pre class="tech mt-2 font-mono whitespace-pre-wrap">{e.file}</pre>
              {:else}
                <p class="flex items-center gap-2 text-[18px] font-bold"><TriangleAlert class="size-4.5 shrink-0 text-over" />Unbekannter Fragebogen</p>
                <p class="mt-0.5 text-[15px] text-muted-foreground">Dieser Fragebogen kann nicht geöffnet werden. Bitte wenden Sie sich an den Support.</p>
                <pre class="tech mt-2 font-mono whitespace-pre-wrap">{e.file ? e.file + "\n" : ""}{e.error}</pre>
              {/if}
            </div>
            {#if e.needs_app}
              <Button variant="outline" class="h-11 px-5 font-bold" onclick={onupdate}><Download />Zu den Updates</Button>
            {:else}
              <Button class="h-11 px-5 font-bold" disabled={!e.summary} onclick={() => start(e)}><Play />Starten</Button>
            {/if}
          </li>
        {/each}
      </ul>
    </Card.Root>
  {/if}
  <div class="tech mt-4 flex flex-wrap items-center gap-2 text-[15px] text-muted-foreground">
    <span class="mr-2">Datenordner: <span class="font-mono">{dir}</span></span>
    <Button variant="outline" class="h-11 px-4" onclick={() => openDefinitionsFolder().catch((e) => (banner = String(e)))}><FolderOpen />Ordner öffnen</Button>
    <Button variant="outline" class="h-11 px-4" onclick={load}><RefreshCw />Neu laden</Button>
  </div>
</div>

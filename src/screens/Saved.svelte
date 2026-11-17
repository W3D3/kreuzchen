<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import Download from "@lucide/svelte/icons/download";
  import FileInput from "@lucide/svelte/icons/file-input";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { confirm } from "$lib/components/confirm.svelte";
  import { deleteResult, exportResult, importResult, listResults, openResult } from "../lib/api";
  import { formatCreatedAt } from "../lib/kennung";
  import type { OpenedResult, SavedSummary } from "../lib/types";

  let { onopen }: { onopen: (res: OpenedResult) => void } = $props();

  let rows = $state<SavedSummary[] | null>(null);
  let banner = $state<string | null>(null);

  // Runs an action; any German error string goes to the banner.
  async function run(fn: () => Promise<unknown>) {
    banner = null;
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    }
  }
  const load = () => run(async () => (rows = await listResults()));
  load();

  const openRow = (name: string) => run(async () => onopen(await openResult(name)));

  const exportRow = (name: string) =>
    run(async () => {
      const dest = await save({ defaultPath: name });
      if (dest) await exportResult(name, dest);
    });

  const importOne = () =>
    run(async () => {
      const src = await open({ filters: [{ name: "Auswertung", extensions: ["json"] }], multiple: false });
      if (!src) return;
      await importResult(src);
      rows = await listResults();
    });

  const remove = (r: SavedSummary) =>
    run(async () => {
      if (!(await confirm({ title: "Auswertung löschen", message: `Auswertung „${r.kennung}“ endgültig löschen?`, confirmLabel: "Löschen", destructive: true }))) return;
      await deleteResult(r.name);
      rows = await listResults();
    });
</script>

<div class="flex flex-col gap-5 pb-16">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <h1 class="text-[26px] leading-tight font-bold">Gespeicherte Auswertungen</h1>
    <Button variant="outline" class="h-11 px-4" onclick={importOne}><FileInput />Importieren…</Button>
  </div>
  {#if banner}
    <p role="alert" class="flex items-center gap-2 rounded-lg border border-over/40 bg-over-soft px-4 py-2.5 text-[15px]">
      <TriangleAlert class="size-4 shrink-0 text-over" />{banner}
    </p>
  {/if}
  {#if rows?.length === 0}
    <p class="rounded-xl border border-dashed px-6 py-10 text-center text-[16px] text-muted-foreground">Keine gespeicherten Auswertungen.</p>
  {:else if rows}
    <Card.Root class="gap-0 py-0">
      <ul class="divide-y">
        {#each rows as r (r.name)}
          <li class="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3.5">
            <div class="min-w-0 flex-1">
              <p class="font-mono font-bold">{r.kennung}</p>
              <p class="text-[15px] text-muted-foreground">{r.title} · {formatCreatedAt(r.created_at)}</p>
              <p class="tech font-mono text-muted-foreground">{r.name}</p>
            </div>
            <div class="flex gap-2">
              <Button class="h-11 px-5 font-bold" onclick={() => openRow(r.name)}>Öffnen</Button>
              <Button variant="outline" class="h-11 px-4" onclick={() => exportRow(r.name)}><Download />Exportieren</Button>
              <Button variant="destructive" class="h-11 px-4" onclick={() => remove(r)}><Trash2 />Löschen</Button>
            </div>
          </li>
        {/each}
      </ul>
    </Card.Root>
  {/if}
</div>

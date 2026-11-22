<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import FileBraces from "@lucide/svelte/icons/file-braces";
  import FileInput from "@lucide/svelte/icons/file-input";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Banner from "$lib/components/Banner.svelte";
  import PdfButton from "$lib/components/PdfButton.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { confirm } from "$lib/components/confirm.svelte";
  import { deleteResult, exportResult, importResult, listResults, openResult } from "../lib/api";
  import { formatCreatedAt, pdfName } from "../lib/subject";
  import type { OpenedResult, Report, SavedSummary } from "../lib/types";

  let { onopen }: { onopen: (res: OpenedResult) => void } = $props();

  let rows = $state<SavedSummary[] | null>(null);
  let banner = $state<string | null>(null);
  let status = $state("");

  // Runs an action; any German error string goes to the banner.
  async function run(fn: () => Promise<unknown>) {
    banner = null;
    status = "";
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    }
  }
  const load = () => run(async () => (rows = await listResults()));
  load();

  const openRow = (name: string) => run(async () => onopen(await openResult(name)));

  // Same report as the opened result: evaluation date and the norms of the unchanged definition.
  const report = (name: string) => async (): Promise<Report> => {
    const { record, norms } = await openResult(name);
    return { title: record.questionnaire.title, date: formatCreatedAt(record.created_at), grading: record.grading, norms };
  };

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
      if (!(await confirm({ title: "Auswertung löschen", message: `Auswertung „${r.subject_id}“ endgültig löschen?`, confirmLabel: "Löschen", destructive: true }))) return;
      await deleteResult(r.name);
      rows = await listResults();
    });
</script>

<div class="flex flex-col gap-5 pb-16">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <h1 class="text-[26px] leading-tight font-bold">Gespeicherte Auswertungen</h1>
    <div class="flex items-center gap-3">
      <span aria-live="polite" class="text-[14px] text-muted-foreground">{status}</span>
      <Button variant="outline" class="tech h-11 px-4" onclick={importOne}><FileInput />JSON importieren …</Button>
    </div>
  </div>
  {#if banner}
    <Banner>{banner}</Banner>
  {/if}
  {#if rows?.length === 0}
    <p class="rounded-xl border border-dashed px-6 py-10 text-center text-[16px] text-muted-foreground">Keine gespeicherten Auswertungen.</p>
  {:else if rows}
    <Card.Root class="gap-0 py-0">
      <ul class="divide-y">
        {#each rows as r (r.name)}
          <li class="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3.5">
            <div class="min-w-0 flex-1">
              <p class="font-mono font-bold">{r.subject_id}</p>
              <p class="text-[15px] text-muted-foreground">{r.title} · {formatCreatedAt(r.created_at)}</p>
              <p class="tech font-mono text-muted-foreground">{r.name}</p>
            </div>
            <div class="flex flex-wrap gap-2">
              <Button class="h-11 px-5 font-bold" onclick={() => openRow(r.name)}>Öffnen</Button>
              <PdfButton
                report={report(r.name)}
                name={pdfName(r.subject_id, r.title)}
                onstart={() => { banner = null; status = ""; }}
                onerror={(e) => (banner = e)}
                onstatus={(s) => (status = s)}
              />
              <Button variant="outline" class="tech h-11 px-4" onclick={() => exportRow(r.name)}><FileBraces />JSON exportieren</Button>
              <Button variant="destructive" class="h-11 px-4" onclick={() => remove(r)}><Trash2 />Löschen</Button>
            </div>
          </li>
        {/each}
      </ul>
    </Card.Root>
  {/if}
</div>

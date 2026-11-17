<script lang="ts">
  import Copy from "@lucide/svelte/icons/copy";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Plus from "@lucide/svelte/icons/plus";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { confirm } from "$lib/components/confirm.svelte";
  import { deleteDefinition, duplicateDefinition, listDefinitions, readDefinitionSource, yamlToDefinition } from "../lib/api";
  import { newDoc, openDoc, type Doc } from "../lib/editor";
  import type { DefinitionEntry } from "../lib/types";

  let { onedit }: { onedit: (doc: Doc) => void } = $props();

  let entries = $state<DefinitionEntry[] | null>(null);
  let banner = $state<string | null>(null);

  async function run(fn: () => Promise<unknown>) {
    banner = null;
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    }
  }
  const load = () => run(async () => (entries = await listDefinitions()));
  load();

  const edit = (e: DefinitionEntry) =>
    run(async () => {
      const yaml = await readDefinitionSource(e.file, e.locked);
      const def = await yamlToDefinition(yaml).catch(() => null);
      onedit(openDoc(e.file, e.hash, yaml, def));
    });

  const duplicate = (e: DefinitionEntry) =>
    run(async () => {
      await duplicateDefinition(e.file, e.locked);
      entries = await listDefinitions();
    });

  const remove = (e: DefinitionEntry) =>
    run(async () => {
      const title = e.summary?.title ?? e.file;
      if (!(await confirm({ title: "Fragebogen löschen", message: `Fragebogen „${title}“ endgültig löschen? Gespeicherte Auswertungen bleiben erhalten.`, confirmLabel: "Löschen", destructive: true }))) return;
      await deleteDefinition(e.file, e.locked);
      entries = await listDefinitions();
    });
</script>

<div class="flex flex-col gap-5 pb-16">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <h1 class="text-[26px] leading-tight font-bold">Fragebögen bearbeiten</h1>
    <Button class="h-11 px-5 font-bold" onclick={() => onedit(newDoc())}><Plus />Neuer Fragebogen</Button>
  </div>
  {#if banner}
    <p role="alert" class="flex items-center gap-2 rounded-lg border border-over/40 bg-over-soft px-4 py-2.5 text-[15px]">
      <TriangleAlert class="size-4 shrink-0 text-over" />{banner}
    </p>
  {/if}
  {#if entries?.length}
    <Card.Root class="gap-0 py-0">
      <ul class="divide-y">
        {#each entries as e (e.locked + e.file + e.hash)}
          <li class="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3.5">
            <div class="min-w-0 flex-1">
              <p class="flex flex-wrap items-center gap-2 text-[17px] font-bold">
                {e.summary?.title ?? "Fehlerhafte Datei"}{#if e.locked}<Badge variant="outline" class="h-6 px-2 text-[13px] text-muted-foreground">Registry</Badge>{/if}
              </p>
              <p class="font-mono text-muted-foreground">{e.file}{e.summary ? ` · Version ${e.summary.version}` : ""}</p>
            </div>
            <div class="flex gap-2">
              <Button variant="outline" class="h-11 px-4" disabled={e.locked || !e.file || !e.hash} onclick={() => edit(e)}><Pencil />Bearbeiten</Button>
              <Button variant="outline" class="h-11 px-4" disabled={!e.hash} onclick={() => duplicate(e)}><Copy />Duplizieren</Button>
              <Button variant="destructive" class="h-11 px-4" disabled={e.locked || !e.file} onclick={() => remove(e)}><Trash2 />Löschen</Button>
            </div>
          </li>
        {/each}
      </ul>
    </Card.Root>
  {/if}
</div>

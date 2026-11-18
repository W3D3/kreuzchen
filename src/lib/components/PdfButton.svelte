<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import FileDown from "@lucide/svelte/icons/file-down";
  import FileText from "@lucide/svelte/icons/file-text";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { exportPdf, openPdf } from "../api";
  import type { Report } from "../types";

  let { report, name, primary = false, onstart, onerror, onstatus }: {
    /** Built on demand, since a saved row has to load its record first. */
    report: () => Promise<Report>;
    /** File name of the PDF, also the save dialog's default. */
    name: string;
    primary?: boolean;
    /** An action began; clear old messages. */
    onstart?: () => void;
    onerror: (message: string) => void;
    onstatus?: (message: string) => void;
  } = $props();

  let busy = $state(false);
  const variant = $derived(primary ? "default" : "outline");

  /** Opens the PDF in the system viewer, or saves it to `dest`. */
  async function make(dest: string | null) {
    onstart?.();
    busy = true;
    try {
      const r = await report();
      if (dest) {
        await exportPdf(r, dest);
        onstatus?.("PDF gespeichert.");
      } else await openPdf(r, name);
    } catch (e) {
      onerror(String(e));
    } finally {
      busy = false;
    }
  }

  const open = () => make(null);

  async function saveAs() {
    const dest = await saveDialog({ defaultPath: name, filters: [{ name: "PDF", extensions: ["pdf"] }] }).catch((e) => {
      onerror(String(e));
      return null;
    });
    if (dest) await make(dest);
  }
</script>

<div role="group" aria-label="PDF" aria-busy={busy} class="inline-flex">
  <Button {variant} class="h-11 rounded-r-none px-4 disabled:cursor-progress {primary ? 'font-bold' : ''}" disabled={busy} onclick={open}>
    {#if busy}<LoaderCircle class="animate-spin" />{:else}<FileText />{/if}PDF öffnen
  </Button>
  <DropdownMenu.Root>
    <DropdownMenu.Trigger disabled={busy}>
      {#snippet child({ props })}
        <Button
          {...props}
          {variant}
          aria-label="Weitere PDF-Aktionen"
          class="h-11 w-10 rounded-l-none border-l px-0 disabled:cursor-progress {primary ? 'border-l-primary-foreground/30' : '-ml-px'}"
        ><ChevronDown /></Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content align="end" class="w-auto min-w-56">
      <DropdownMenu.Item class="min-h-10 px-2.5 text-[15px]" onSelect={open}><FileText />PDF öffnen</DropdownMenu.Item>
      <DropdownMenu.Item class="min-h-10 px-2.5 text-[15px]" onSelect={saveAs}><FileDown />Als PDF speichern …</DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
</div>

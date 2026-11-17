<script lang="ts">
  import Entry from "./screens/Entry.svelte";
  import Result from "./screens/Result.svelte";
  import List from "./screens/List.svelte";
  import Saved from "./screens/Saved.svelte";
  import Definitions from "./screens/Definitions.svelte";
  import Editor from "./screens/Editor.svelte";
  import Registry from "./screens/Registry.svelte";
  import Settings from "./screens/Settings.svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import Download from "@lucide/svelte/icons/download";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import { confirm } from "$lib/components/confirm.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { getDefinition, grade } from "./lib/api";
  import { isDirty, type Doc } from "./lib/editor";
  import { initialInput } from "./lib/input";
  import { installTech } from "./lib/tech.svelte";
  import type { Definition, Graded, Input, OpenedDefinition, OpenedResult, SavedRecord } from "./lib/types";

  // `preview` is the editor's draft YAML while it is being tested.
  type Screen =
    | { name: "list" }
    | { name: "entry"; opened: OpenedDefinition; input: Input; preview?: string }
    | { name: "result"; opened: OpenedDefinition | null; record: SavedRecord | null; input: Input; graded: Graded; notice: string | null; onregrade?: (() => void) | null; preview?: string }
    | { name: "saved" }
    | { name: "definitions" }
    | { name: "editor" }
    | { name: "registry" }
    | { name: "settings" };

  let screen = $state<Screen>({ name: "list" });
  let entry = $state<Entry>();
  let draft = $state<Doc | null>(null);
  installTech();

  let update = $state<Update | null>(null);
  let updating = $state(false);
  let updateError = $state<string | null>(null);
  let checking = $state(false);
  /** Result of a manual check that found nothing to install, shown on the button for a moment. */
  let updateNote = $state<string | null>(null);

  // The automatic check at start stays quiet when offline; a manual one says what happened.
  async function findUpdate(manual: boolean) {
    checking = true;
    updateNote = null;
    try {
      update = await check();
      if (!update && manual) updateNote = `Aktuell ✓ ${await getVersion()}`;
    } catch {
      if (manual) updateNote = "Keine Verbindung";
    } finally {
      checking = false;
    }
    if (updateNote) setTimeout(() => (updateNote = null), 4000);
  }
  findUpdate(false);

  /** True when there is no unsaved entry or draft, or the user agrees to discard it. */
  async function mayLeave(): Promise<boolean> {
    if (screen.name === "entry" && !(await entry?.confirmDiscard())) return false;
    return !(draft && isDirty(draft)) || (await confirm({ title: "Fragebogen bearbeiten", message: "Ungespeicherte Änderungen verwerfen?", confirmLabel: "Verwerfen", destructive: true }));
  }

  async function go(next: Screen) {
    if (!(await mayLeave())) return;
    draft = null;
    screen = next;
  }

  async function installUpdate() {
    if (!update || !(await mayLeave())) return;
    updating = true;
    updateError = null;
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      updateError = `Update fehlgeschlagen: ${e}`;
      updating = false;
    }
  }

  function preview(yaml: string, definition: Definition) {
    screen = { name: "entry", opened: { file: "", hash: "", definition, locked: false }, input: initialInput(definition, new Date()), preview: yaml };
  }

  function showSaved(res: OpenedResult, notice: string | null) {
    const { record, current, outdated } = res;
    screen = {
      name: "result", opened: null, record, input: record.input,
      graded: { grading: record.grading, text: record.text },
      notice: notice ?? (outdated
        ? "Dieser Fragebogen wurde seit dem Speichern geändert. Angezeigt wird das gespeicherte Ergebnis."
        : current ? null : "Dieser Fragebogen ist nicht mehr installiert. Angezeigt wird das gespeicherte Ergebnis."),
      onregrade: outdated && current ? () => regrade(res) : null,
    };
  }

  async function regrade(res: OpenedResult) {
    try {
      const opened = await getDefinition(res.current!.file, res.current!.locked);
      const graded = await grade(opened.file, opened.locked, opened.hash, res.record.input);
      screen = { name: "result", opened, record: res.record, input: res.record.input, graded, notice: null };
    } catch (e) {
      showSaved(res, String(e));
    }
  }
</script>

<Tooltip.Provider>
  <header class="border-b border-border bg-card">
    <nav class="mx-auto flex h-16 max-w-[1040px] items-center gap-1 px-6">
      {@render tab("Fragebögen", screen.name === "list" || screen.name === "entry" || screen.name === "registry", () => go({ name: "list" }))}
      {@render tab("Gespeicherte Auswertungen", screen.name === "saved" || screen.name === "result", () => go({ name: "saved" }))}
      {@render tab("Fragebögen bearbeiten", screen.name === "definitions" || screen.name === "editor", () => go({ name: "definitions" }), "tech")}
      <div class="flex-1"></div>
      <Button variant="ghost" class="h-11 px-4 text-muted-foreground disabled:cursor-progress" onclick={() => findUpdate(true)} disabled={checking || updating}>
        <RefreshCw class={checking ? "animate-spin" : ""} />{checking ? "Suche Updates …" : updateNote ?? "Nach Updates suchen"}
      </Button>
      {@render tab("Einstellungen", screen.name === "settings", () => go({ name: "settings" }))}
    </nav>
    {#if update}
      <div class="flex items-center justify-center gap-4 border-t border-border bg-secondary px-6 py-2 text-[15px]" role="status">
        <span>{updateError ?? (updating ? "Update wird installiert …" : `Neue Version ${update.version} verfügbar.`)}</span>
        <Button class="h-9 px-4 font-bold disabled:cursor-progress" onclick={installUpdate} disabled={updating}><Download />Jetzt installieren</Button>
      </div>
    {/if}
  </header>
  <main class="mx-auto max-w-[56rem] p-6">
    {#if screen.name === "list"}
      <List onstart={(opened, input) => (screen = { name: "entry", opened, input })} onregistry={() => (screen = { name: "registry" })} />
    {:else if screen.name === "entry"}
      {@const e = screen}
      <Entry
        bind:this={entry}
        opened={e.opened}
        input={e.input}
        preview={e.preview}
        onback={() => (screen = e.preview == null ? { name: "list" } : { name: "editor" })}
        ongraded={(graded, input) => (screen = { name: "result", opened: e.opened, record: null, input, graded, notice: null, preview: e.preview })}
      />
    {:else if screen.name === "result"}
      {@const r = screen}
      <Result
        opened={r.opened}
        input={r.input}
        graded={r.graded}
        notice={r.notice}
        onregrade={r.onregrade}
        preview={r.preview != null}
        onback={() => r.opened && (screen = { name: "entry", opened: r.opened, input: r.input, preview: r.preview })}
        onnew={() => (screen = r.preview == null ? { name: "list" } : { name: "editor" })}
      />
    {:else if screen.name === "saved"}
      <Saved onopen={(res) => showSaved(res, null)} />
    {:else if screen.name === "definitions"}
      <Definitions onedit={(doc) => { draft = doc; screen = { name: "editor" }; }} />
    {:else if screen.name === "registry"}
      <Registry />
    {:else if screen.name === "editor" && draft}
      <Editor bind:doc={draft} onclose={() => go({ name: "definitions" })} onpreview={preview} />
    {:else if screen.name === "settings"}
      <Settings />
    {/if}
  </main>
  <ConfirmDialog />
</Tooltip.Provider>

{#snippet tab(label: string, active: boolean, onclick: () => void, cls = "")}
  <Button variant="ghost" aria-current={active ? "page" : undefined} class="h-11 px-4 text-[15px] aria-[current=page]:bg-secondary aria-[current=page]:font-bold {cls}" {onclick}>{label}</Button>
{/snippet}

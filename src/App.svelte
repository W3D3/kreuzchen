<script lang="ts">
  import Entry from "./screens/Entry.svelte";
  import Result from "./screens/Result.svelte";
  import List from "./screens/List.svelte";
  import Saved from "./screens/Saved.svelte";
  import Definitions from "./screens/Definitions.svelte";
  import Editor from "./screens/Editor.svelte";
  import Registry from "./screens/Registry.svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { ask } from "@tauri-apps/plugin-dialog";
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
    | { name: "registry" };

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
    return !(draft && isDirty(draft)) || (await ask("Ungespeicherte Änderungen verwerfen?", { title: "Fragebogen bearbeiten", kind: "warning" }));
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

<nav>
  <button class:active={screen.name === "list" || screen.name === "entry" || screen.name === "registry"} onclick={() => go({ name: "list" })}>Fragebögen</button>
  <button class:active={screen.name === "saved" || screen.name === "result"} onclick={() => go({ name: "saved" })}>Gespeicherte Auswertungen</button>
  <button class="tech" class:active={screen.name === "definitions" || screen.name === "editor"} onclick={() => go({ name: "definitions" })}>Fragebögen bearbeiten</button>
  <button class="check" onclick={() => findUpdate(true)} disabled={checking || updating}>{checking ? "Suche Updates …" : updateNote ?? "Nach Updates suchen"}</button>
</nav>
{#if update}
  <div class="update" role="status">
    <span>{updateError ?? (updating ? "Update wird installiert …" : `Neue Version ${update.version} verfügbar.`)}</span>
    <button onclick={installUpdate} disabled={updating}>Jetzt installieren</button>
  </div>
{/if}
<main>
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
  {/if}
</main>

<style>
  nav { display: flex; gap: .5rem; padding: .75rem 1.5rem; background: var(--surface); border-bottom: 1px solid var(--line); }
  nav button { font: inherit; min-height: 48px; padding: 0 1.25rem; border-radius: 10px; border: 0; background: none; color: var(--fg); cursor: pointer; }
  nav button.active { background: var(--accent-soft); font-weight: 700; }
  nav .check { margin-left: auto; color: var(--muted); }
  nav .check:disabled { cursor: progress; }
  .update { display: flex; align-items: center; justify-content: center; gap: 1rem; padding: .5rem 1.5rem; background: var(--accent-soft); border-bottom: 1px solid var(--line); }
  .update button { font: inherit; min-height: 40px; padding: 0 1.25rem; border-radius: 10px; border: 1px solid var(--accent); background: var(--accent); color: var(--accent-fg); font-weight: 700; cursor: pointer; }
  .update button:disabled { opacity: .5; cursor: progress; }
  main { max-width: 56rem; margin: 0 auto; padding: 1.5rem; }
</style>

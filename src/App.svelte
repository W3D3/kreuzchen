<script lang="ts">
  import Entry from "./screens/Entry.svelte";
  import Result from "./screens/Result.svelte";
  import List from "./screens/List.svelte";
  import { installTech } from "./lib/tech.svelte";
  import type { Graded, Input, OpenedDefinition, SavedRecord } from "./lib/types";

  type Screen =
    | { name: "list" }
    | { name: "entry"; opened: OpenedDefinition; input: Input }
    | { name: "result"; opened: OpenedDefinition | null; record: SavedRecord | null; input: Input; graded: Graded; notice: string | null }
    | { name: "saved" };

  let screen = $state<Screen>({ name: "list" });
  installTech();
</script>

<nav>
  <button class:active={screen.name === "list" || screen.name === "entry"} onclick={() => (screen = { name: "list" })}>Fragebögen</button>
  <button class:active={screen.name === "saved" || screen.name === "result"} onclick={() => (screen = { name: "saved" })}>Gespeicherte Auswertungen</button>
</nav>
<main>
  {#if screen.name === "list"}
    <List onstart={(opened, input) => (screen = { name: "entry", opened, input })} />
  {:else if screen.name === "entry"}
    {@const e = screen}
    <Entry
      opened={e.opened}
      input={e.input}
      onback={() => (screen = { name: "list" })}
      ongraded={(graded, input) => (screen = { name: "result", opened: e.opened, record: null, input, graded, notice: null })}
    />
  {:else if screen.name === "result"}
    {@const r = screen}
    <Result
      opened={r.opened}
      input={r.input}
      graded={r.graded}
      notice={r.notice}
      onback={() => r.opened && (screen = { name: "entry", opened: r.opened, input: r.input })}
      onnew={() => (screen = { name: "list" })}
    />
  {:else if screen.name === "saved"}
    <h1>Gespeicherte Auswertungen</h1>
  {/if}
</main>

<style>
  nav { display: flex; gap: .5rem; padding: .75rem 1.5rem; background: var(--surface); border-bottom: 1px solid var(--line); }
  nav button { font: inherit; min-height: 48px; padding: 0 1.25rem; border-radius: 10px; border: 0; background: none; color: var(--fg); cursor: pointer; }
  nav button.active { background: var(--accent-soft); font-weight: 700; }
  main { max-width: 56rem; margin: 0 auto; padding: 1.5rem; }
</style>

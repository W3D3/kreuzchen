<script lang="ts">
  import EntryProto from "./EntryProto.svelte";
  import ResultProto from "./ResultProto.svelte";

  let hash = $state(location.hash);
  // Kept here so "Zurück zu den Antworten" finds them again.
  let answers = $state<Record<string, number>>({});
  const go = (h: string) => (location.hash = h);
</script>

<svelte:window onhashchange={() => (hash = location.hash)} />

{#if hash === "#proto-result"}
  <ResultProto onback={() => go("#proto-entry")} onnew={() => { answers = {}; go("#proto-entry"); }} />
{:else}
  <EntryProto bind:answers ongraded={() => go("#proto-result")} />
{/if}

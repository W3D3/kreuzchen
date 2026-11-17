<script lang="ts">
  import { loadTheme, saveTheme, type Theme } from "../lib/theme";

  let theme = $state<Theme>(loadTheme());
  const choices: [Theme, string][] = [["system", "Wie das System"], ["light", "Hell"], ["dark", "Dunkel"]];
</script>

<h1>Einstellungen</h1>

<fieldset>
  <legend>Darstellung</legend>
  {#each choices as [value, label] (value)}
    <label>
      <input type="radio" name="theme" {value} bind:group={theme} onchange={() => saveTheme(theme)} />
      {label}
    </label>
  {/each}
</fieldset>

<style>
  fieldset { border: 1px solid var(--line); border-radius: 12px; background: var(--surface); padding: 1rem 1.25rem; display: flex; flex-wrap: wrap; gap: .5rem 1.5rem; }
  legend { font-weight: 700; padding: 0 .25rem; }
  label { display: flex; align-items: center; gap: .5rem; min-height: 48px; cursor: pointer; }
  input { width: 1.25rem; height: 1.25rem; accent-color: var(--accent); }
</style>

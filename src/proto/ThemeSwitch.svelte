<script lang="ts">
  import { loadTheme, saveTheme, type Theme } from "../lib/theme";

  let theme = $state<Theme>(loadTheme());
  const choices: [Theme, string][] = [["system", "System"], ["light", "Hell"], ["dark", "Dunkel"]];
</script>

<!-- Prototype only: the normal app's settings screen needs the desktop backend. -->
<div class="fixed bottom-16 left-4 z-50 flex gap-1 rounded-full border border-border bg-card p-1 text-sm shadow-md" role="radiogroup" aria-label="Darstellung">
  {#each choices as [value, label] (value)}
    <button
      type="button"
      role="radio"
      aria-checked={theme === value}
      class="rounded-full px-3 py-1 {theme === value ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}"
      onclick={() => { theme = value; saveTheme(value); }}
    >{label}</button>
  {/each}
</div>

<script lang="ts">
  import * as Card from "$lib/components/ui/card/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
  import { loadTheme, saveTheme, type Theme } from "../lib/theme";

  let theme = $state<Theme>(loadTheme());
  const choices: [Theme, string][] = [["system", "Wie das System"], ["light", "Hell"], ["dark", "Dunkel"]];
</script>

<div class="flex flex-col gap-5">
  <h1 class="text-[26px] leading-tight font-bold">Einstellungen</h1>

  <Card.Root class="gap-4">
    <Card.Header><Card.Title id="theme-label" class="text-[17px] font-bold">Darstellung</Card.Title></Card.Header>
    <Card.Content>
      <RadioGroup.Root
        bind:value={() => theme, (v) => { theme = v as Theme; saveTheme(theme); }}
        aria-labelledby="theme-label"
        class="flex flex-wrap gap-x-8 gap-y-2"
      >
        {#each choices as [value, label] (value)}
          <div class="flex min-h-11 items-center gap-2.5">
            <RadioGroup.Item {value} id="theme-{value}" class="size-5" />
            <Label for="theme-{value}" class="cursor-pointer text-[15px]">{label}</Label>
          </div>
        {/each}
      </RadioGroup.Root>
    </Card.Content>
  </Card.Root>
</div>

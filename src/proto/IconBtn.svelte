<script lang="ts">
  import type { Component } from "svelte";
  import { Button, type ButtonVariant } from "$lib/components/ui/button/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";

  let { label, icon: Icon, variant = "ghost", disabled = false, onclick }: {
    label: string;
    icon: Component;
    variant?: ButtonVariant;
    disabled?: boolean;
    onclick?: () => void;
  } = $props();
</script>

<Tooltip.Root>
  <Tooltip.Trigger>
    {#snippet child({ props })}
      <Button {...props} {variant} size="icon" aria-label={label} {disabled} {onclick}><Icon /></Button>
    {/snippet}
  </Tooltip.Trigger>
  <Tooltip.Content>{label}</Tooltip.Content>
</Tooltip.Root>

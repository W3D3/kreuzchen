<script lang="ts">
  import * as Select from "$lib/components/ui/select/index.js";
  import { cn } from "$lib/utils.js";

  // bits-ui has no empty value, so "no choice" travels as NONE and leaves as null.
  const NONE = "__none";
  let { value = $bindable(), options, label, class: className = "", mono = false, id }: {
    value: string | null;
    options: { value: string | null; label: string }[];
    label: string;
    class?: string;
    mono?: boolean;
    id?: string;
  } = $props();

  const current = $derived(options.find((o) => o.value === value));
</script>

<Select.Root type="single" value={value ?? NONE} onValueChange={(v) => (value = v === NONE ? null : v)}>
  <Select.Trigger {id} aria-label={label} class={cn("w-full", className)}>
    <span class={cn("truncate", mono && "font-mono", current?.value == null && "text-muted-foreground")}>{current?.label ?? ""}</span>
  </Select.Trigger>
  <Select.Content>
    {#each options as o, i (i)}
      <Select.Item value={o.value ?? NONE} label={o.label} class={cn(mono && o.value != null && "font-mono")}>{o.label}</Select.Item>
    {/each}
  </Select.Content>
</Select.Root>

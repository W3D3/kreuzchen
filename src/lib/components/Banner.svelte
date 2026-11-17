<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { cn } from "$lib/utils.js";

  // "error" is announced at once (role=alert); "info" politely (role=status).
  let { kind = "error", icon, class: className = "", children }: {
    kind?: "error" | "info";
    icon?: Component;
    class?: string;
    children: Snippet;
  } = $props();
  const Icon = $derived(icon ?? TriangleAlert);
</script>

<p role={kind === "error" ? "alert" : "status"} class={cn("flex items-start gap-2 rounded-lg border border-over/40 bg-over-soft px-4 py-2.5 text-[15px] whitespace-pre-wrap", className)}>
  <Icon class="mt-[3px] size-4 shrink-0 text-over" /><span>{@render children()}</span>
</p>

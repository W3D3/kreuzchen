<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { answer, confirmState } from "./confirm.svelte";

  const req = $derived(confirmState.request);
</script>

<!-- Esc, overlay click and the close button all go through onOpenChange and count as "no". -->
<Dialog.Root open={confirmState.open} onOpenChange={(open) => !open && answer(false)}>
  <Dialog.Content class="sm:max-w-md gap-5">
    <Dialog.Header>
      <Dialog.Title class="text-[17px] font-bold">{req?.title}</Dialog.Title>
      <Dialog.Description class="text-[15px] leading-relaxed">{req?.message}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => answer(false)}>Abbrechen</Button>
      <Button class={req?.destructive ? "bg-destructive text-background hover:bg-destructive/90" : ""} onclick={() => answer(true)}>{req?.confirmLabel}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<script lang="ts">
  import { onMount } from "svelte";
  import Banner from "$lib/components/Banner.svelte";
  import { confirm } from "$lib/components/confirm.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { Input as TextInput } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { grade, gradeDraft } from "../lib/api";
  import { entryItems } from "../lib/input";
  import { entryKey } from "../lib/keyboard";
  import type { Graded, Input, OpenedDefinition } from "../lib/types";

  let { opened, input, preview = null, onback, ongraded }: {
    opened: OpenedDefinition;
    input: Input;
    /** Draft YAML from the editor; grades it instead of the saved file. */
    preview?: string | null;
    onback: () => void;
    ongraded: (graded: Graded, input: Input) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  const def = opened.definition;
  const items = entryItems(def);
  // svelte-ignore state_referenced_locally
  let answers = $state<Record<string, number>>({ ...input.answers });
  // svelte-ignore state_referenced_locally
  let fields = $state<Record<string, boolean | number | string>>({ ...input.fields });
  let focused = $state(0);
  let tried = $state(false);
  let banner = $state<string | null>(null);
  let busy = false;
  let els: HTMLElement[] = [];
  onMount(() => els[0]?.focus());

  const scored = items.filter((i) => i.scored);
  const answered = $derived(scored.filter((i) => answers[i.key] !== undefined).length);
  const shortcuts = $derived(items[focused]?.shortcuts.filter(Boolean) ?? []);

  function focusItem(i: number) {
    focused = i;
    els[i]?.focus();
    els[i]?.scrollIntoView({ block: "nearest" });
  }

  async function doGrade() {
    if (busy) return;
    busy = true;
    tried = true;
    banner = null;
    // A cleared number field is null, which Rust cannot read as a FieldValue.
    const finalInput: Input = {
      fields: Object.fromEntries(Object.entries(fields).filter(([, v]) => v != null)),
      answers: { ...answers },
    };
    try {
      ongraded(await (preview == null ? grade(opened.file, opened.locked, opened.hash, finalInput) : gradeDraft(preview, finalInput)), finalInput);
    } catch (e) {
      banner = String(e);
    } finally {
      busy = false;
    }
  }

  /** True when nothing is entered or the user agrees to discard it. */
  export async function confirmDiscard(): Promise<boolean> {
    return Object.keys(answers).length === 0 || (await confirm({ title: "Ungespeicherte Eingaben", message: "Eingaben verwerfen?", confirmLabel: "Verwerfen", destructive: true }));
  }

  async function cancel() {
    if (await confirmDiscard()) onback();
  }

  const hints = $derived([
    [shortcuts, "Antwort"], [["←", "→"], "Antwort ändern"], [["Enter", "Tab"], "nächste Frage"],
    [["⇧", "Tab"], "zurück"], [["Entf"], "löschen"], [["Strg", "Enter"], "Auswerten"],
  ] as [string[], string][]);

  function onkeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.altKey || e.metaKey) return;
    if (e.ctrlKey && e.key !== "Enter") return;
    const item = items[focused];
    const a = entryKey(
      { key: e.key, shift: e.shiftKey, ctrl: e.ctrlKey },
      { focused, count: items.length, shortcuts: item.shortcuts, current: answers[item.key] ?? null },
    );
    if (a.type === "none") return;
    // At either end Tab leaves the list, so header fields and buttons stay reachable.
    if (e.key === "Tab" && a.type === "focus" && a.to === focused) return;
    e.preventDefault();
    if (a.type === "answer") {
      answers[item.key] = a.index;
      if (item.shortcuts.includes(e.key)) focusItem(Math.min(focused + 1, items.length - 1));
    } else if (a.type === "clear") delete answers[item.key];
    else if (a.type === "focus") focusItem(a.to);
    else void doGrade();
  }
</script>

{#snippet kbd(k: string, cls = "", hidden = false)}
  <kbd aria-hidden={hidden ? "true" : undefined} class="inline-grid h-5 min-w-5 place-items-center rounded-[4px] border border-current/25 px-1 font-mono text-[12px]! leading-none {cls}">{k}</kbd>
{/snippet}

<!-- Full-bleed bar inside the centred column: the shadows paint the card colour and bottom line out to the window edges. -->
<header class="sticky top-0 z-20 -mx-6 -mt-6 mb-6 bg-card px-6 [clip-path:inset(0_-100vmax_-1px)] shadow-[0_0_0_100vmax_var(--surface),0_1px_0_100vmax_var(--line)]">
  <div class="flex min-h-16 items-center gap-4">
    <div class="flex flex-col gap-1.5">
      <span class="text-[17px]"><span class="font-mono font-bold tabular-nums">{answered} / {scored.length}</span> beantwortet</span>
      <div class="flex flex-wrap gap-1" aria-hidden="true">
        {#each scored as s (s.key)}
          {@const gap = answers[s.key] === undefined}
          <span class="h-1.5 w-7 rounded-full {gap ? (tried ? 'bg-over/70' : 'bg-border') : 'bg-primary'}"></span>
        {/each}
      </div>
    </div>
    <div class="flex-1"></div>
    <Button variant="ghost" class="h-11 px-4" onclick={cancel}>Abbrechen</Button>
    <Button class="h-11 px-5 font-bold" onclick={doGrade}>Auswerten</Button>
  </div>
  {#if banner}
    <Banner class="mb-3">{banner}</Banner>
  {/if}
</header>

<div class="flex flex-col gap-5 pb-16">
  <div>
    <h1 class="text-[26px] leading-tight font-bold">{def.title}</h1>
    {#if preview != null}<p class="mt-1 text-[15px] text-muted-foreground">Test des Entwurfs. Es wird nichts gespeichert.</p>{/if}
  </div>

  {#if def.fields.length}
    <Card.Root class="py-4">
      <Card.Content class="flex flex-wrap gap-4">
        {#each def.fields as f (f.id)}
          {@const label = f.id === "kennung" ? "Kennung (keine Patientennamen)" : f.label}
          {#if f.type === "checkbox"}
            <div class="flex items-center gap-2.5 self-end py-2.5">
              <Checkbox id="fld-{f.id}" class="size-6 [&_svg]:size-5!" bind:checked={fields[f.id] as boolean} />
              <Label for="fld-{f.id}" class="text-[15px] font-bold">{label}</Label>
            </div>
          {:else}
            <div class="grid flex-[1_1_14rem] gap-1.5">
              <Label for="fld-{f.id}" class="text-[14px] font-bold">{label}</Label>
              {#if f.type === "number"}
                <TextInput id="fld-{f.id}" type="number" class="h-11 font-mono" bind:value={fields[f.id] as number} />
              {:else if f.type === "date"}
                <TextInput id="fld-{f.id}" type="date" class="h-11 font-mono" bind:value={fields[f.id] as string} />
              {:else}
                <TextInput id="fld-{f.id}" class="h-11 font-mono" bind:value={fields[f.id] as string} />
              {/if}
            </div>
          {/if}
        {/each}
      </Card.Content>
    </Card.Root>
  {/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="flex flex-col gap-2.5" {onkeydown}>
    {#each items as it, i (it.key)}
      {#if i === 0 || it.section !== items[i - 1].section}
        {@const sec = def.sections[it.section]}
        <div class="mt-3 mb-0.5">
          <h2 class="text-[19px] font-bold">{sec.title}</h2>
          {#if sec.intro}<p class="text-[15px] text-muted-foreground">{sec.intro}</p>{/if}
        </div>
      {/if}
      {@const missing = tried && it.scored && answers[it.key] === undefined}
      <!-- scroll-my-28 keeps a focused question clear of the sticky top bar and the fixed shortcut bar. -->
      <div
        class="flex scroll-my-28 gap-3 rounded-xl border bg-card px-5 py-4 outline-none focus-within:outline-3 focus-within:outline-offset-2 focus-within:outline-primary focus-within:outline-solid
          {missing ? 'border-l-[6px] border-l-over pl-[15px]' : ''}"
        role="radiogroup"
        aria-invalid={missing ? "true" : undefined}
        aria-label={it.text}
        tabindex={i === focused ? 0 : -1}
        bind:this={els[i]}
        onfocus={() => (focused = i)}
      >
        <span class="min-w-8 pt-0.5 text-right font-mono text-muted-foreground tabular-nums">{it.key}</span>
        <div class="flex min-w-0 flex-1 flex-col items-start gap-3">
          <div class="flex w-full items-baseline gap-3">
            <span class="min-w-0 flex-1 text-[18px] break-words">{it.text}</span>
            {#if missing}<span class="shrink-0 text-[14px] font-bold text-over">Antwort fehlt</span>{/if}
          </div>
          <div class="inline-flex max-w-full flex-wrap gap-1 rounded-lg bg-muted p-1">
            {#each it.options as o, j (j)}
              {@const sel = answers[it.key] === j}
              <button
                type="button"
                role="radio"
                aria-checked={sel}
                tabindex="-1"
                class="flex min-h-12 min-w-28 items-center gap-2.5 rounded-md px-4 py-2 text-left text-[16px]
                  {sel ? 'bg-primary font-bold text-primary-foreground shadow-sm' : 'text-foreground hover:bg-card'}"
                onclick={() => { answers[it.key] = j; focusItem(i); }}
              >
                {#if it.shortcuts[j]}{@render kbd(it.shortcuts[j], sel ? "shrink-0" : "shrink-0 text-muted-foreground", true)}{/if}{o}
              </button>
            {/each}
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>

<footer class="fixed inset-x-0 bottom-0 z-20 border-t border-border bg-card">
  <div class="mx-auto flex min-h-11 max-w-[1040px] flex-wrap items-center justify-center gap-x-5 gap-y-1 px-6 py-2 text-[14px] text-muted-foreground">
    {#each hints as [keys, label] (label)}
      <span class="flex items-center gap-1">{#each keys as k, j (j)}{@render kbd(k)}{/each}<span class="ml-0.5">{label}</span></span>
    {/each}
  </div>
</footer>

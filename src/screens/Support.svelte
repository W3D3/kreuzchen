<script lang="ts">
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import Info from "@lucide/svelte/icons/info";
  import Send from "@lucide/svelte/icons/send";
  import Server from "@lucide/svelte/icons/server";
  import Banner from "$lib/components/Banner.svelte";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import { listDefinitions, listTickets, registryConnection, sendTicket } from "../lib/api";
  import { choices, KINDS, kindLabel, statusLabel, ticketForm } from "../lib/support";
  import type { ConnectionInfo, QRef, TicketKind, TicketSummary } from "../lib/types";

  let { questionnaire = null, onback, onregistry }: {
    /** Pre-selected, e.g. from "Feedback zu diesem Fragebogen". */
    questionnaire?: QRef | null;
    onback: () => void;
    onregistry: () => void;
  } = $props();

  let conn = $state<ConnectionInfo | null | undefined>(undefined);
  let options = $state<QRef[]>([]);
  const blank = () => ({ type: "feedback" as TicketKind, subject: "", message: "", reply_email: "" });
  let form = $state(blank());
  // "" is "Allgemein"; otherwise a questionnaire id from `options`. The prop only pre-selects.
  // svelte-ignore state_referenced_locally
  let picked = $state(questionnaire?.id ?? "");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let sent = $state<number | null>(null);
  let tickets = $state<TicketSummary[] | null>(null);
  let listError = $state<string | null>(null);
  const pickedLabel = $derived(options.find((q) => q.id === picked)?.title ?? "Allgemein");

  async function loadTickets() {
    try {
      tickets = await listTickets();
      listError = null;
    } catch (e) {
      listError = String(e);
    }
  }

  registryConnection().then((c) => {
    conn = c;
    if (c) loadTickets();
  });
  listDefinitions().then((entries) => (options = choices(entries, questionnaire)), () => (options = choices([], questionnaire)));

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = null;
    sent = null;
    try {
      sent = await sendTicket(ticketForm(form), options.find((q) => q.id === picked) ?? null);
      form = blank();
      picked = "";
      loadTickets();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="flex flex-col gap-5 pb-16">
  <div>
    <Button variant="ghost" class="-ml-2.5 h-11 px-3" onclick={onback}><ArrowLeft />Zurück</Button>
    <h1 class="text-[26px] leading-tight font-bold">Support & Feedback</h1>
  </div>

  {#if conn === null}
    <Card.Root class="gap-3">
      <Card.Content class="flex flex-col gap-3 text-[15px]">
        <p class="flex items-start gap-2"><Server class="mt-[3px] size-4.5 shrink-0 text-muted-foreground" />Anfragen und Feedback werden über den Bibliotheksserver gesendet. Verbinden Sie Kreuzchen zuerst unter „Bibliothek“ mit Ihrem Benutzernamen und Schlüssel.</p>
        <div><Button class="h-11 px-5 font-bold" onclick={onregistry}>Zur Bibliothek</Button></div>
      </Card.Content>
    </Card.Root>
  {:else if conn}
    {#if sent != null}
      <p role="status" class="flex items-center gap-2.5 rounded-xl border border-primary/40 bg-secondary px-5 py-3 text-[15px]">
        <CircleCheck class="size-4.5 shrink-0 text-primary" />Danke! Ihre Anfrage wurde als Nr. {sent} gesendet.
      </p>
    {/if}
    {#if error}
      <Banner>{error}</Banner>
    {/if}
    <Card.Root class="py-5">
      <Card.Content>
        <form class="grid gap-5" onsubmit={submit}>
          <Banner kind="info" icon={Info}>Bitte keine Patientendaten eintragen. Gesendet wird nur, was Sie hier schreiben, dazu Ihre Praxis, der gewählte Fragebogen, die App-Version und das Betriebssystem.</Banner>
          <div class="grid gap-1.5">
            <span id="kind-label" class="text-[14px] font-bold">Art</span>
            <RadioGroup.Root bind:value={form.type} aria-labelledby="kind-label" class="flex flex-wrap gap-x-8 gap-y-2">
              {#each KINDS as [value, label] (value)}
                <div class="flex min-h-11 items-center gap-2.5">
                  <RadioGroup.Item {value} id="kind-{value}" class="size-5" />
                  <Label for="kind-{value}" class="cursor-pointer text-[15px]">{label}</Label>
                </div>
              {/each}
            </RadioGroup.Root>
          </div>
          <div class="grid gap-1.5">
            <Label for="ticket-questionnaire" class="text-[14px] font-bold">Fragebogen</Label>
            <Select.Root type="single" bind:value={picked}>
              <Select.Trigger id="ticket-questionnaire" class="h-11 w-full text-[15px]">{pickedLabel}</Select.Trigger>
              <Select.Content>
                <Select.Item value="" label="Allgemein" />
                {#each options as q (q.id)}
                  <Select.Item value={q.id} label={q.title} />
                {/each}
              </Select.Content>
            </Select.Root>
          </div>
          <div class="grid gap-1.5">
            <Label for="ticket-subject" class="text-[14px] font-bold">Betreff</Label>
            <Input id="ticket-subject" class="h-11" bind:value={form.subject} required maxlength={200} />
          </div>
          <div class="grid gap-1.5">
            <Label for="ticket-message" class="text-[14px] font-bold">Nachricht</Label>
            <textarea
              id="ticket-message"
              class="min-h-40 w-full rounded-md border border-input bg-transparent p-3 text-[15px] outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
              bind:value={form.message} required maxlength={10000}></textarea>
          </div>
          <div class="grid gap-1.5">
            <Label for="ticket-email" class="text-[14px] font-bold">E-Mail für die Antwort <span class="font-normal text-muted-foreground">(optional)</span></Label>
            <Input id="ticket-email" type="email" class="h-11" bind:value={form.reply_email} maxlength={254} autocomplete="email" />
          </div>
          <div><Button type="submit" class="h-11 px-5 font-bold" disabled={busy}><Send />{busy ? "Wird gesendet …" : "Senden"}</Button></div>
        </form>
      </Card.Content>
    </Card.Root>

    <section class="flex flex-col gap-2.5">
      <h2 class="text-[19px] font-bold">Meine Anfragen</h2>
      {#if listError}
        <p class="text-[15px] text-muted-foreground">{listError}</p>
      {:else if tickets?.length === 0}
        <p class="text-[15px] text-muted-foreground">Noch keine Anfragen gesendet.</p>
      {:else if tickets}
        <Card.Root class="gap-0 py-0">
          <ul class="divide-y">
            {#each tickets as t (t.id)}
              <li class="flex flex-wrap items-center gap-x-4 gap-y-1 px-5 py-3">
                <div class="min-w-0 flex-1">
                  <p class="font-bold">Nr. {t.id} · {t.subject}</p>
                  <p class="text-[15px] text-muted-foreground">{kindLabel(t.type)} · {t.questionnaire?.title ?? "Allgemein"} · {new Date(t.created_at).toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" })}</p>
                </div>
                <Badge variant={t.status === "open" ? "secondary" : "outline"} class="h-6 px-2 text-[13px]">{statusLabel(t.status)}</Badge>
              </li>
            {/each}
          </ul>
        </Card.Root>
      {/if}
    </section>
  {/if}
</div>

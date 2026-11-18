<script lang="ts">
  import CloudOff from "@lucide/svelte/icons/cloud-off";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import Server from "@lucide/svelte/icons/server";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Banner from "$lib/components/Banner.svelte";
  import { confirm } from "$lib/components/confirm.svelte";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { registryConnect, registryConnection, registryDisconnect, registryInstall, registryRemove, registryRows } from "../lib/api";
  import { advance, filterRows, percent, rowView, type Download } from "../lib/registry";
  import type { ConnectionInfo, Row, RowState, RowsResult } from "../lib/types";

  let conn = $state<ConnectionInfo | null>(null);
  let result = $state<RowsResult | null>(null);
  let editing = $state(false);
  let form = $state({ url: "", user: "", key: "" });
  let banner = $state<string | null>(null);
  let busy = $state(false);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  let query = $state("");
  let download = $state<Download | null>(null);
  const shown = $derived(result ? filterRows(result.rows, query) : []);
  // A failed list load, or a connection whose server list could not be fetched (installed rows still show).
  const failure = $derived(loadError ?? (conn && result && !result.online ? (result.error ?? "Registry nicht erreichbar") : null));
  const badgeClass: Partial<Record<RowState, string>> = { gone: "bg-over-soft text-over", current: "text-muted-foreground" };

  async function run(fn: () => Promise<unknown>) {
    banner = null;
    busy = true;
    try {
      await fn();
    } catch (e) {
      banner = String(e);
    } finally {
      busy = false;
    }
  }
  // Separate assignments so a failure in one call still shows the other's result.
  async function refresh() {
    conn = await registryConnection();
    result = await registryRows();
  }
  const load = () =>
    run(async () => {
      loading = true;
      loadError = null;
      try {
        await refresh();
      } catch (e) {
        loadError = String(e);
      } finally {
        loading = false;
      }
    });
  load();

  function host(url: string) {
    try {
      return new URL(url).host;
    } catch {
      return url;
    }
  }

  const change = () => {
    form = { url: "", user: "", key: "" };
    editing = true;
  };

  const connect = (e: SubmitEvent) => {
    e.preventDefault();
    return run(async () => {
      await registryConnect(form.url, form.user, form.key);
      form.key = "";
      editing = false;
      await refresh();
    });
  };

  const disconnect = () =>
    run(async () => {
      if (!(await confirm({ title: "Verbindung trennen", message: "Verbindung zur Registry trennen? Installierte Fragebögen bleiben erhalten.", confirmLabel: "Trennen", destructive: true }))) return;
      await registryDisconnect();
      conn = null;
      result = await registryRows();
    });

  async function fetchRow(r: Row, replaceLocal: boolean) {
    download = { id: r.id, progress: null };
    try {
      return await registryInstall(r.id, replaceLocal, (p) => (download = advance(download, r.id, p)));
    } finally {
      download = null;
    }
  }

  const install = (r: Row) =>
    run(async () => {
      try {
        if ((await fetchRow(r, false)) === "needs_replace_confirm") {
          if (!(await confirm({ title: "Lokale Kopie ersetzen", message: "Lokale Kopie durch die Version aus der Registry ersetzen?", confirmLabel: "Ersetzen" }))) return;
          await fetchRow(r, true);
        }
      } catch (e) {
        banner = `„${r.title}“ konnte nicht heruntergeladen werden: ${e}`;
        return;
      }
      result = await registryRows();
    });

  const remove = (r: Row) =>
    run(async () => {
      if (!(await confirm({ title: "Fragebogen entfernen", message: `Fragebogen „${r.title}“ entfernen? Gespeicherte Auswertungen bleiben erhalten.`, confirmLabel: "Entfernen", destructive: true }))) return;
      await registryRemove(r.file!);
      result = await registryRows();
    });
</script>

<div class="flex flex-col gap-5 pb-16">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <h1 class="text-[26px] leading-tight font-bold">Fragebögen aus der Registry</h1>
    {#if conn}<Button variant="outline" class="h-11 px-4" onclick={load} disabled={busy}><RefreshCw class={loading ? "animate-spin" : ""} />{loading ? "Lädt …" : "Nach Änderungen suchen"}</Button>{/if}
  </div>
  {#if banner}
    <Banner>{banner}</Banner>
  {/if}

  {#if loading && !result}
    <div role="status" class="flex items-center justify-center gap-3 rounded-xl border bg-card px-6 py-10 text-[16px] text-muted-foreground">
      <LoaderCircle class="size-5 animate-spin" />Registry wird geladen …
    </div>
  {:else if conn && !editing}
    <div class="flex flex-wrap items-center gap-3 rounded-xl border bg-card px-5 py-3 text-[15px]">
      <Server class="size-4.5 shrink-0 text-muted-foreground" />
      <span class="min-w-0 flex-1">Registry: <strong>{conn.user}</strong> @ <span class="font-mono">{host(conn.url)}</span>, verbunden</span>
      <Button variant="outline" class="h-11 px-4" onclick={change} disabled={busy}>Ändern</Button>
      <Button variant="destructive" class="h-11 px-4" onclick={disconnect} disabled={busy}>Trennen</Button>
    </div>
  {:else}
    <Card.Root class="py-5">
      <Card.Content>
        <form class="grid gap-4" onsubmit={connect}>
          <div class="grid gap-1.5">
            <Label for="reg-url" class="text-[14px] font-bold">URL</Label>
            <Input id="reg-url" class="h-11 font-mono" bind:value={form.url} required placeholder="registry.example.org" />
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <div class="grid gap-1.5">
              <Label for="reg-user" class="text-[14px] font-bold">Benutzername</Label>
              <Input id="reg-user" class="h-11" bind:value={form.user} required autocomplete="username" />
            </div>
            <div class="grid gap-1.5">
              <Label for="reg-key" class="text-[14px] font-bold">Schlüssel</Label>
              <Input id="reg-key" type="password" class="h-11" bind:value={form.key} required autocomplete="current-password" />
            </div>
          </div>
          <div class="flex gap-2">
            <Button type="submit" class="h-11 px-5 font-bold" disabled={busy}>Verbinden</Button>
            {#if conn}<Button type="button" variant="ghost" class="h-11 px-4" onclick={() => (editing = false)}>Abbrechen</Button>{/if}
          </div>
        </form>
      </Card.Content>
    </Card.Root>
  {/if}

  {#if failure && !loading}
    <div role="alert" class="flex flex-wrap items-center gap-3 rounded-xl border border-over/40 bg-over-soft px-5 py-3 text-[15px]">
      <CloudOff class="size-4.5 shrink-0 text-over" />
      <span class="min-w-0 flex-1"><strong>Liste der Registry konnte nicht geladen werden.</strong> {failure}</span>
      <Button variant="outline" class="h-11 px-4" onclick={load} disabled={busy}><RefreshCw />Erneut versuchen</Button>
    </div>
  {/if}

  {#if result?.rows.length}
    <div class="relative">
      <Search class="pointer-events-none absolute top-1/2 left-3.5 size-4.5 -translate-y-1/2 text-muted-foreground" />
      <Input type="search" class="h-11 bg-card pl-10 text-[16px]" aria-label="Fragebögen durchsuchen" placeholder="Fragebogen suchen …" bind:value={query} />
    </div>
  {:else if conn && result?.online}
    <p class="rounded-xl border border-dashed px-6 py-10 text-center text-[16px] text-muted-foreground">Die Registry bietet derzeit keine Fragebögen an.</p>
  {/if}

  {#if result?.rows.length && !shown.length}
    <p class="rounded-xl border border-dashed px-6 py-10 text-center text-[16px] text-muted-foreground">Kein Fragebogen passt zu „{query.trim()}“.</p>
  {:else if result && shown.length}
    <Card.Root class="gap-0 py-0">
      <ul class="divide-y">
        {#each shown as r (r.id + (r.file ?? ""))}
          {@const v = rowView(r, result.online)}
          <li class="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3.5">
            <p class="flex min-w-0 flex-1 flex-wrap items-center gap-2 text-[17px] font-bold">
              {r.title}
              {#if v.label}
                <Badge variant={r.state === "current" ? "outline" : "secondary"} class="h-6 px-2 text-[13px] {badgeClass[r.state] ?? ''}">{v.label}</Badge>
              {/if}
            </p>
            <div class="flex gap-2">
              {#if v.action}
                {@const pct = download?.id === r.id ? percent(download.progress) : undefined}
                {#if pct === undefined}
                  <Button class="h-11 min-w-40 px-5 font-bold" disabled={busy} onclick={() => install(r)}>{v.action}</Button>
                {:else}
                  <!-- Full opacity while disabled: the fill is the progress display. -->
                  <Button class="relative h-11 min-w-40 overflow-hidden px-5 font-bold tabular-nums disabled:opacity-100 disabled:grayscale-0" disabled aria-busy="true">
                    <span class="absolute inset-y-0 left-0 bg-primary-foreground/25 transition-[width]" style:width="{pct ?? 0}%"></span>
                    <LoaderCircle class="animate-spin" />{pct == null ? "Lädt …" : `Lädt … ${pct} %`}
                  </Button>
                {/if}
              {/if}
              {#if v.remove}<Button variant="destructive" class="h-11 px-4" disabled={busy} onclick={() => remove(r)}><Trash2 />Entfernen</Button>{/if}
            </div>
          </li>
        {/each}
      </ul>
    </Card.Root>
  {/if}
</div>

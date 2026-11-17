<script lang="ts">
  import CloudOff from "@lucide/svelte/icons/cloud-off";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
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
  import { rowView } from "../lib/registry";
  import type { ConnectionInfo, Row, RowState, RowsResult } from "../lib/types";

  let conn = $state<ConnectionInfo | null>(null);
  let result = $state<RowsResult | null>(null);
  let editing = $state(false);
  let form = $state({ url: "", user: "", key: "" });
  let banner = $state<string | null>(null);
  let busy = $state(false);
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
  const load = () => run(refresh);
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

  const install = (r: Row) =>
    run(async () => {
      if ((await registryInstall(r.id, false)) === "needs_replace_confirm") {
        if (!(await confirm({ title: "Lokale Kopie ersetzen", message: "Lokale Kopie durch die Version aus der Registry ersetzen?", confirmLabel: "Ersetzen" }))) return;
        await registryInstall(r.id, true);
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
    {#if conn}<Button variant="outline" class="h-11 px-4" onclick={load} disabled={busy}><RefreshCw />Nach Änderungen suchen</Button>{/if}
  </div>
  {#if banner}
    <Banner>{banner}</Banner>
  {/if}

  {#if conn && !editing}
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

  {#if conn && result && !result.online}
    <Banner kind="info" icon={CloudOff}>{result.error ?? "Registry nicht erreichbar"}</Banner>
  {/if}

  {#if result?.rows.length}
    <Card.Root class="gap-0 py-0">
      <ul class="divide-y">
        {#each result.rows as r (r.id + (r.file ?? ""))}
          {@const v = rowView(r, result.online)}
          <li class="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3.5">
            <p class="flex min-w-0 flex-1 flex-wrap items-center gap-2 text-[17px] font-bold">
              {r.title}
              {#if v.label}
                <Badge variant={r.state === "current" ? "outline" : "secondary"} class="h-6 px-2 text-[13px] {badgeClass[r.state] ?? ''}">{v.label}</Badge>
              {/if}
            </p>
            <div class="flex gap-2">
              {#if v.action}<Button class="h-11 px-5 font-bold" disabled={busy} onclick={() => install(r)}>{v.action}</Button>{/if}
              {#if v.remove}<Button variant="destructive" class="h-11 px-4" disabled={busy} onclick={() => remove(r)}><Trash2 />Entfernen…</Button>{/if}
            </div>
          </li>
        {/each}
      </ul>
    </Card.Root>
  {/if}
</div>

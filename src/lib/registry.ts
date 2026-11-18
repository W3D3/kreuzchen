import type { Progress, Row, RowState } from "./types";

const LABELS: Record<RowState, string> = {
  available: "verfügbar",
  current: "aktuell",
  update: "neue Version verfügbar",
  gone: "nicht mehr in der Bibliothek",
};
const ACTIONS: Partial<Record<RowState, "Installieren" | "Aktualisieren">> = { available: "Installieren", update: "Aktualisieren" };

/** What a registry row shows; offline rows carry no server state, so they only offer Entfernen. */
export const rowView = (row: Row, online: boolean) => ({
  label: online ? LABELS[row.state] : null,
  action: online ? ACTIONS[row.state] ?? null : null,
  remove: row.file != null,
});

/** Rows whose title or id contains `query`, ignoring case; a blank query keeps all. */
export function filterRows(rows: Row[], query: string): Row[] {
  const q = query.trim().toLocaleLowerCase("de");
  return q ? rows.filter((r) => `${r.title}\n${r.id}`.toLocaleLowerCase("de").includes(q)) : rows;
}

/** Whole percent downloaded, `null` while the size is unknown (no progress yet or no length sent). */
export const percent = (p: Progress | null): number | null =>
  p?.total ? Math.min(100, Math.floor((p.received * 100) / p.total)) : null;

/** The download running in the registry screen; one at a time, since every action waits for the last. */
export interface Download { id: string; progress: Progress | null }

/** `d` after a progress message for row `id`; a message arriving after the download ended changes nothing. */
export const advance = (d: Download | null, id: string, progress: Progress): Download | null => (d?.id === id ? { id, progress } : d);

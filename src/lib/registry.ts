import type { Row, RowState } from "./types";

const LABELS: Record<RowState, string> = {
  available: "verfügbar",
  current: "aktuell",
  update: "neue Version verfügbar",
  gone: "nicht mehr auf dem Server",
};
const ACTIONS: Partial<Record<RowState, "Installieren" | "Aktualisieren">> = { available: "Installieren", update: "Aktualisieren" };

/** What a registry row shows; offline rows carry no server state, so they only offer Entfernen…. */
export const rowView = (row: Row, online: boolean) => ({
  label: online ? LABELS[row.state] : null,
  action: online ? ACTIONS[row.state] ?? null : null,
  remove: row.file != null,
});

import type { DefinitionEntry, QRef, TicketForm, TicketKind } from "./types";

export const KINDS: [TicketKind, string][] = [["question", "Frage"], ["bug", "Fehler melden"], ["feedback", "Feedback"], ["wish", "Wunsch"]];
export const kindLabel = (k: TicketKind) => KINDS.find(([v]) => v === k)?.[1] ?? k;
export const statusLabel = (s: "open" | "done") => (s === "open" ? "offen" : "erledigt");

/** The questionnaire reference of an installed definition: id, title, version and hash, nothing else. */
export const qrefOf = (e: DefinitionEntry): QRef | null => (e.summary ? { id: e.summary.id, title: e.summary.title, version: e.summary.version, hash: e.hash } : null);

/** The request form from the inputs, trimmed; a blank email becomes `null`. Builds a fresh object so nothing else rides along. */
export const ticketForm = (f: { type: TicketKind; subject: string; message: string; reply_email: string }): TicketForm => ({
  type: f.type,
  subject: f.subject.trim(),
  message: f.message.trim(),
  reply_email: f.reply_email.trim() || null,
});

/** Installed questionnaires to pick from, plus `extra` (e.g. a saved result's) when it is not installed; by title. */
export function choices(entries: DefinitionEntry[], extra: QRef | null): QRef[] {
  const list = entries.flatMap((e) => qrefOf(e) ?? []);
  if (extra && !list.some((q) => q.id === extra.id)) list.push(extra);
  return list.sort((a, b) => a.title.localeCompare(b.title, "de"));
}

import type { DefinitionEntry, QRef, TicketForm, TicketKind, TicketSummary } from "./types";

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

/** One message of a ticket's thread: the ticket's own text first, then the replies. */
export interface ThreadEntry { key: string; mine: boolean; who: string; created_at: string; message: string }

export const thread = (t: TicketSummary): ThreadEntry[] => [
  { key: "ticket", mine: true, who: "Sie", created_at: t.created_at, message: t.message },
  ...t.replies.map((r) => ({ key: `r${r.id}`, mine: r.author === "practice", who: r.author === "practice" ? "Sie" : "Support", created_at: r.created_at, message: r.message })),
];

/** Per ticket id, the highest admin reply id the user has opened. */
export type Seen = Record<string, number>;

const lastAdminReply = (t: TicketSummary) => Math.max(0, ...t.replies.filter((r) => r.author === "admin").map((r) => r.id));

/** Whether `t` has an admin reply the user has not opened yet. */
export const unread = (t: TicketSummary, seen: Seen) => lastAdminReply(t) > (seen[t.id] ?? 0);

/** `seen` with every admin reply of `t` marked as read. */
export const markSeen = (seen: Seen, t: TicketSummary): Seen => ({ ...seen, [t.id]: Math.max(seen[t.id] ?? 0, lastAdminReply(t)) });

export function parseSeen(raw: string | null): Seen {
  let stored: unknown;
  try {
    stored = JSON.parse(raw ?? "{}");
  } catch {
    return {};
  }
  if (!stored || typeof stored !== "object" || Array.isArray(stored)) return {};
  return Object.fromEntries(Object.entries(stored).filter(([k, v]) => /^\d+$/.test(k) && typeof v === "number" && Number.isInteger(v) && v >= 0));
}

// shortcut: one map for all servers (ticket ids are per server); key it by server URL if practices switch servers.
const SEEN = "kreuzchen.ticketsSeen";

/** Read state of admin replies, kept on this device only. */
export const ticketsSeen = {
  load(): Seen {
    try {
      return parseSeen(localStorage.getItem(SEEN));
    } catch {
      return {};
    }
  },
  save(seen: Seen) {
    try {
      localStorage.setItem(SEEN, JSON.stringify(seen));
    } catch {
      // Storage unavailable: markers come back after a restart.
    }
  },
};

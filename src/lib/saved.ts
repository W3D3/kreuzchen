import { formatCreatedAt } from "./subject";
import type { SavedSummary } from "./types";

const MONTHS = ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];

/** Lower case without accents, so „muller“ finds „Müller“ and „angste“ finds „Ängste“. */
const fold = (s: string) => s.toLocaleLowerCase("de").normalize("NFD").replace(/\p{Diacritic}/gu, "");

/** Rows where every word of `query` occurs in the Kennung, the title or the date as shown; a blank query keeps all. */
export function filterSaved(rows: SavedSummary[], query: string): SavedSummary[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (!words.length) return rows;
  return rows.filter((r) => {
    const text = fold(`${r.subject_id}\n${r.title}\n${formatCreatedAt(r.created_at)}`);
    return words.every((w) => text.includes(w));
  });
}

/**
 * Date group of a saved result relative to `now`: „Heute“, „Gestern“, „Diese Woche“ (since Monday), else the month.
 * `created_at` is the local wall-clock time it was saved at, so its calendar day is read from the text, never converted.
 */
export function dateBucket(createdAt: string, now: Date): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(createdAt);
  if (!m) return "Ohne Datum";
  const [y, mo, d] = [+m[1], +m[2], +m[3]];
  // Whole days between two calendar dates; UTC so a DST switch in between cannot shift the count.
  const days = (Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) - Date.UTC(y, mo - 1, d)) / 86_400_000;
  if (days <= 0) return "Heute";
  if (days === 1) return "Gestern";
  if (days <= (now.getDay() + 6) % 7) return "Diese Woche";
  return `${MONTHS[mo - 1]} ${y}`;
}

/** Rows under one header; `label` is null for the ungrouped list. */
export type DateGroup = { label: string | null; rows: SavedSummary[] };

/** Rows in their date groups, in list order (newest first); groups without rows do not appear. */
export function groupByDate(rows: SavedSummary[], now: Date): DateGroup[] {
  const groups = new Map<string, SavedSummary[]>();
  for (const r of rows) {
    const label = dateBucket(r.created_at, now);
    const group = groups.get(label);
    if (group) group.push(r);
    else groups.set(label, [r]);
  }
  return [...groups].map(([label, rows]) => ({ label, rows }));
}

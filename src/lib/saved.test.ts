import { describe, expect, it } from "vitest";
import { dateBucket, filterSaved, groupByDate } from "./saved";
import type { SavedSummary } from "./types";

const row = (subject_id: string, title: string, created_at: string): SavedSummary => ({ name: `${subject_id}.json`, subject_id, title, created_at });

describe("filterSaved", () => {
  const rows = [
    row("MÜLLER-01", "Beck-Depressions-Inventar", "2026-11-17T14:30:00"),
    row("Anonym 03.02.2026 09:15", "Ängste überprüfen", "2026-02-03T09:15:00"),
    row("K-17", "PHQ-9", "2025-12-31T23:59:00"),
  ];
  const ids = (q: string) => filterSaved(rows, q).map((r) => r.subject_id);

  it("blank query keeps every row in order", () => {
    expect(ids("")).toEqual(["MÜLLER-01", "Anonym 03.02.2026 09:15", "K-17"]);
    expect(ids("  ")).toEqual(ids(""));
  });
  it("matches Kennung, title and the date as shown, ignoring case and umlauts", () => {
    expect(ids("muller")).toEqual(["MÜLLER-01"]);
    expect(ids("müller")).toEqual(["MÜLLER-01"]);
    expect(ids("ANGSTE")).toEqual(["Anonym 03.02.2026 09:15"]);
    expect(ids("phq")).toEqual(["K-17"]);
    expect(ids("17.11.2026")).toEqual(["MÜLLER-01"]);
    expect(ids("31.12.2025 23:59")).toEqual(["K-17"]);
  });
  it("needs every word, in any field", () => {
    expect(ids("beck 11.2026")).toEqual(["MÜLLER-01"]);
    expect(ids("beck 2025")).toEqual([]);
  });
  it("does not match the raw ISO timestamp or the file name", () => {
    expect(ids("2026-11-17")).toEqual([]);
    expect(ids(".json")).toEqual([]);
  });
});

describe("dateBucket", () => {
  // Wednesday, 25 November 2026, shortly after midnight local time.
  const now = new Date(2026, 10, 25, 0, 5);

  it("splits days at local midnight, not at UTC midnight", () => {
    expect(dateBucket("2026-11-25T00:01:00", now)).toBe("Heute");
    expect(dateBucket("2026-11-24T23:59:00", now)).toBe("Gestern");
    expect(dateBucket("2026-11-24T00:00:00", new Date(2026, 10, 25, 23, 59))).toBe("Gestern");
  });
  it("this week reaches back to Monday", () => {
    expect(dateBucket("2026-11-23T08:00:00", now)).toBe("Diese Woche");
    expect(dateBucket("2026-11-22T08:00:00", now)).toBe("November 2026");
  });
  it("on a Monday there is no separate week group", () => {
    const monday = new Date(2026, 10, 23, 9, 0);
    expect(dateBucket("2026-11-22T08:00:00", monday)).toBe("Gestern");
    expect(dateBucket("2026-11-21T08:00:00", monday)).toBe("November 2026");
  });
  it("older results fall into their month, with the year", () => {
    expect(dateBucket("2026-10-31T12:00:00", now)).toBe("Oktober 2026");
    expect(dateBucket("2026-03-01T12:00:00", now)).toBe("März 2026");
    expect(dateBucket("2025-11-25T12:00:00", now)).toBe("November 2025");
  });
  it("across the year change", () => {
    // Friday, 2 January 2026: the week began on Monday, 29 December 2025.
    const jan2 = new Date(2026, 0, 2, 10, 0);
    expect(dateBucket("2026-01-01T23:00:00", jan2)).toBe("Gestern");
    expect(dateBucket("2025-12-31T09:00:00", jan2)).toBe("Diese Woche");
    expect(dateBucket("2025-12-29T09:00:00", jan2)).toBe("Diese Woche");
    expect(dateBucket("2025-12-28T09:00:00", jan2)).toBe("Dezember 2025");
    expect(dateBucket("2025-12-31T23:59:00", new Date(2026, 0, 1, 0, 0))).toBe("Gestern");
  });
  it("a clock set back counts as today; an unreadable date gets its own group", () => {
    expect(dateBucket("2026-11-26T08:00:00", now)).toBe("Heute");
    expect(dateBucket("", now)).toBe("Ohne Datum");
  });
});

describe("groupByDate", () => {
  it("keeps list order and only lists groups that have rows", () => {
    const now = new Date(2026, 10, 25, 12, 0);
    const rows = [row("a", "T", "2026-11-25T10:00:00"), row("b", "T", "2026-11-23T10:00:00"), row("c", "T", "2026-10-02T10:00:00"), row("d", "T", "2026-10-01T10:00:00")];
    expect(groupByDate(rows, now).map((g) => [g.label, g.rows.map((r) => r.subject_id)])).toEqual([
      ["Heute", ["a"]],
      ["Diese Woche", ["b"]],
      ["Oktober 2026", ["c", "d"]],
    ]);
    expect(groupByDate([], now)).toEqual([]);
  });
});

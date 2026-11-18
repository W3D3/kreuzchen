import { describe, expect, it } from "vitest";
import { advance, filterRows, percent, rowView } from "./registry";
import type { Row } from "./types";

const row = (state: Row["state"], file: string | null, id = "fragebogen_a", title = "Fragebogen A"): Row => ({ id, title, state, file });

describe("rowView", () => {
  it("labels_and_actions", () => {
    expect(rowView(row("available", null), true)).toEqual({ label: "verfügbar", action: "Installieren", remove: false });
    expect(rowView(row("current", "fragebogen_a.yaml"), true)).toEqual({ label: "aktuell", action: null, remove: true });
    expect(rowView(row("update", "fragebogen_a.yaml"), true)).toEqual({ label: "neue Version verfügbar", action: "Aktualisieren", remove: true });
    expect(rowView(row("gone", "fragebogen_a.yaml"), true)).toEqual({ label: "nicht mehr in der Bibliothek", action: null, remove: true });
  });
  it("offline shows no state and only Entfernen", () => {
    expect(rowView(row("current", "fragebogen_a.yaml"), false)).toEqual({ label: null, action: null, remove: true });
  });
});

describe("filterRows", () => {
  const rows = [row("available", null, "phq_9", "Gesundheitsfragebogen"), row("current", "bdi.yaml", "bdi_ii", "Beck-Depressions-Inventar"), row("update", "ängste.yaml", "angst", "Ängste ÜBERPRÜFEN")];
  const ids = (q: string) => filterRows(rows, q).map((r) => r.id);
  it("blank query keeps every row in order", () => {
    expect(ids("")).toEqual(["phq_9", "bdi_ii", "angst"]);
    expect(ids("   ")).toEqual(["phq_9", "bdi_ii", "angst"]);
  });
  it("matches title or id, ignoring case and outer spaces", () => {
    expect(ids("depression")).toEqual(["bdi_ii"]);
    expect(ids(" PHQ ")).toEqual(["phq_9"]);
    expect(ids("überprüfen")).toEqual(["angst"]);
    expect(ids("fragebogen")).toEqual(["phq_9"]);
  });
  it("no match gives no rows", () => {
    expect(ids("xyz")).toEqual([]);
  });
});

describe("percent", () => {
  it("is unknown before the first chunk or without a length", () => {
    expect(percent(null)).toBeNull();
    expect(percent({ received: 500, total: null })).toBeNull();
    expect(percent({ received: 0, total: 0 })).toBeNull();
  });
  it("rounds down so 100 means done, and never exceeds 100", () => {
    expect(percent({ received: 0, total: 1000 })).toBe(0);
    expect(percent({ received: 999, total: 1000 })).toBe(99);
    expect(percent({ received: 1000, total: 1000 })).toBe(100);
    expect(percent({ received: 1200, total: 1000 })).toBe(100);
  });
});

describe("advance", () => {
  const p = { received: 10, total: 100 };
  it("updates the running download", () => {
    expect(advance({ id: "a", progress: null }, "a", p)).toEqual({ id: "a", progress: p });
  });
  it("ignores late messages once the download ended or another one runs", () => {
    expect(advance(null, "a", p)).toBeNull();
    const other = { id: "b", progress: null };
    expect(advance(other, "a", p)).toBe(other);
  });
});

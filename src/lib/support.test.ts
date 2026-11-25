import { describe, expect, it } from "vitest";
import { choices, qrefOf, ticketForm } from "./support";
import type { DefinitionEntry } from "./types";

const entry = (id: string, title: string, hash = "h"): DefinitionEntry => ({
  file: `${id}.yaml`, hash, locked: false, error: null, needs_app: null,
  summary: { id, title, version: "2", item_count: 3, scale_count: 1 },
});

describe("ticketForm", () => {
  it("keeps only the typed fields", () => {
    // Extra keys a caller might hold (answers, subject id) must not reach the payload.
    const typed = { type: "bug" as const, subject: " Summe ", message: " falsch \n", reply_email: "  ", input: { items: { 1: 3 } }, subject_id: "K-1" };
    const form = ticketForm(typed);
    expect(form).toEqual({ type: "bug", subject: "Summe", message: "falsch", reply_email: null });
    expect(JSON.stringify(form)).not.toMatch(/K-1|items|subject_id/);
  });
  it("keeps a given email", () => {
    expect(ticketForm({ type: "wish", subject: "s", message: "m", reply_email: " a@b.at " }).reply_email).toBe("a@b.at");
  });
});

describe("qrefOf / choices", () => {
  it("references only id, title, version, hash", () => {
    expect(qrefOf(entry("a", "A", "x"))).toEqual({ id: "a", title: "A", version: "2", hash: "x" });
    expect(qrefOf({ ...entry("a", "A"), summary: null })).toBeNull();
  });
  it("lists installed by title and adds a missing extra once", () => {
    const list = [entry("b", "Beta"), entry("a", "Alpha")];
    expect(choices(list, null).map((q) => q.id)).toEqual(["a", "b"]);
    expect(choices(list, { id: "a", title: "Alpha", version: "1", hash: "old" }).map((q) => q.id)).toEqual(["a", "b"]);
    expect(choices(list, { id: "z", title: "Zeta", version: "1", hash: "h" }).map((q) => q.id)).toEqual(["a", "b", "z"]);
  });
});

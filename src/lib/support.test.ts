import { describe, expect, it } from "vitest";
import { choices, markSeen, parseSeen, qrefOf, thread, ticketForm, unread } from "./support";
import type { DefinitionEntry, TicketReply, TicketSummary } from "./types";

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

const reply = (id: number, author: TicketReply["author"]): TicketReply => ({ id, author, created_at: `2026-11-26T1${id}:00:00Z`, message: `m${id}` });
const tkt = (id: number, replies: TicketReply[]): TicketSummary => ({
  id, created_at: "2026-11-26T09:00:00Z", type: "bug", subject: "s", message: "Erste Nachricht", questionnaire: null, status: "open", replies,
});

describe("thread", () => {
  it("starts with the ticket's own message, then the replies in order", () => {
    const entries = thread(tkt(3, [reply(1, "admin"), reply(2, "practice")]));
    expect(entries.map((e) => [e.who, e.mine, e.message])).toEqual([["Sie", true, "Erste Nachricht"], ["Support", false, "m1"], ["Sie", true, "m2"]]);
    expect(new Set(entries.map((e) => e.key)).size).toBe(3);
  });
});

describe("unread marker", () => {
  it("marks admin replies until the ticket is opened", () => {
    const t = tkt(5, [reply(1, "admin")]);
    expect(unread(t, {})).toBe(true);
    const seen = markSeen({}, t);
    expect(unread(t, seen)).toBe(false);
    // A newer admin reply is new again; the user's own replies never are.
    expect(unread(tkt(5, [reply(1, "admin"), reply(2, "practice")]), seen)).toBe(false);
    expect(unread(tkt(5, [reply(1, "admin"), reply(2, "practice"), reply(3, "admin")]), seen)).toBe(true);
    expect(unread(tkt(6, [reply(1, "practice")]), {})).toBe(false);
    // Per ticket: seeing #5 says nothing about #7.
    expect(unread(tkt(7, [reply(1, "admin")]), seen)).toBe(true);
  });
  it("never lowers the stored state", () => {
    expect(markSeen({ 5: 4 }, tkt(5, [reply(1, "admin")]))).toEqual({ 5: 4 });
  });
  it("parses stored state defensively", () => {
    expect(parseSeen(JSON.stringify(markSeen({}, tkt(5, [reply(2, "admin")]))))).toEqual({ 5: 2 });
    for (const raw of [null, "", "kaputt", "[1]", "null", "3"]) expect(parseSeen(raw)).toEqual({});
    expect(parseSeen('{"5": 1, "x": 2, "6": "3", "7": -1}')).toEqual({ 5: 1 });
  });
});

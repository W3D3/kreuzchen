import { describe, expect, it } from "vitest";
import { techReducer } from "./tech.svelte";

const s0 = { held: false, toggled: false };
const k = (type: "down" | "up", key: string, ctrl = false, shift = false) =>
  ({ type, key, ctrl, shift }) as const;

describe("techReducer", () => {
  it("Alt held", () => {
    const h = techReducer(s0, k("down", "Alt"));
    expect(h.held).toBe(true);
    expect(techReducer(h, k("up", "Alt")).held).toBe(false);
  });
  it("Ctrl+Shift+D toggles, either case", () => {
    const a = techReducer(s0, k("down", "D", true, true));
    expect(a.toggled).toBe(true);
    expect(techReducer(a, k("down", "d", true, true)).toggled).toBe(false);
  });
  it("blur releases held, keeps toggled", () => {
    expect(techReducer({ held: true, toggled: true }, { type: "blur" })).toEqual({ held: false, toggled: true });
  });
  it("other keys change nothing", () => {
    expect(techReducer(s0, k("down", "a"))).toEqual(s0);
    expect(techReducer(s0, k("down", "d"))).toEqual(s0);
    expect(techReducer(s0, k("up", "d", true, true))).toEqual(s0);
  });
});

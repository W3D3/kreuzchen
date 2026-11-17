import { expect, it } from "vitest";
import { entryKey } from "./keyboard";

const s = { focused: 2, count: 5, options: 4, current: null };
const k = (key: string, o: { shift?: boolean; ctrl?: boolean } = {}) => ({ key, shift: !!o.shift, ctrl: !!o.ctrl });

it("maps keys to actions", () => {
  expect(entryKey(k("0"), s)).toEqual({ type: "answer", index: 0 });
  expect(entryKey(k("3"), s)).toEqual({ type: "answer", index: 3 });
  expect(entryKey(k("4"), s)).toEqual({ type: "none" });
  expect(entryKey(k("ArrowRight"), s)).toEqual({ type: "answer", index: 0 });
  expect(entryKey(k("ArrowLeft"), s)).toEqual({ type: "answer", index: 3 });
  expect(entryKey(k("ArrowRight"), { ...s, current: 3 })).toEqual({ type: "answer", index: 3 });
  expect(entryKey(k("ArrowLeft"), { ...s, current: 0 })).toEqual({ type: "answer", index: 0 });
  expect(entryKey(k("Enter"), s)).toEqual({ type: "focus", to: 3 });
  expect(entryKey(k("Tab"), s)).toEqual({ type: "focus", to: 3 });
  expect(entryKey(k("ArrowDown"), s)).toEqual({ type: "focus", to: 3 });
  expect(entryKey(k("Tab", { shift: true }), s)).toEqual({ type: "focus", to: 1 });
  expect(entryKey(k("ArrowUp"), s)).toEqual({ type: "focus", to: 1 });
  expect(entryKey(k("Delete"), s)).toEqual({ type: "clear" });
  expect(entryKey(k("Backspace"), s)).toEqual({ type: "clear" });
  expect(entryKey(k("Enter", { ctrl: true }), s)).toEqual({ type: "grade" });
  expect(entryKey(k("a"), s)).toEqual({ type: "none" });
  expect(entryKey(k("Enter"), { ...s, focused: 4 })).toEqual({ type: "focus", to: 4 });
  expect(entryKey(k("ArrowUp"), { ...s, focused: 0 })).toEqual({ type: "focus", to: 0 });
});

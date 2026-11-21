import { describe, expect, it } from "vitest";
import {
  errorList, formatDefault, formatKey, freshKey, isDirty, move, newDoc, openDoc, parseDefault, parseItemRef, parseKey,
  renameKey, template,
} from "./editor";

describe("move", () => {
  it("swaps with the neighbour", () => {
    const l = ["a", "b", "c"];
    move(l, 1, -1);
    expect(l).toEqual(["b", "a", "c"]);
    move(l, 1, 1);
    expect(l).toEqual(["b", "c", "a"]);
  });
  it("does nothing at the ends", () => {
    const l = ["a", "b"];
    move(l, 0, -1);
    move(l, 1, 1);
    expect(l).toEqual(["a", "b"]);
  });
});

it("renameKey keeps order and never overwrites", () => {
  const r = { a: 1, b: 2, c: 3 };
  expect(Object.keys(renameKey(r, "b", "x"))).toEqual(["a", "x", "c"]);
  expect(renameKey(r, "b", "c")).toBe(r);
});

it("freshKey skips taken names", () => {
  expect(freshKey({ neu: 1, neu2: 1 }, "neu")).toBe("neu3");
  expect(freshKey({}, "neu")).toBe("neu");
});

it("parseKey tells named from inline keys", () => {
  expect(parseKey("normal")).toBe("normal");
  expect(parseKey(" 0, 0,1 ,2")).toEqual([0, 0, 1, 2]);
  expect(parseKey("-1")).toEqual([-1]);
  expect(parseKey("  ")).toBeNull();
  expect(formatKey([3, 2])).toBe("3, 2");
  expect(formatKey(null)).toBe("");
});

it("parseItemRef tells numbers from ids", () => {
  expect(parseItemRef("12")).toEqual({ n: 12, id: null });
  expect(parseItemRef("zusatz")).toEqual({ n: null, id: "zusatz" });
  expect(parseItemRef("")).toEqual({ n: null, id: null });
});

it("parseDefault follows the field type", () => {
  expect(parseDefault("checkbox", "ja")).toBe(true);
  expect(parseDefault("checkbox", "nein")).toBe(false);
  expect(parseDefault("number", "4")).toBe(4);
  expect(parseDefault("text", "anonymous")).toBe("anonymous");
  expect(parseDefault("text", "")).toBeNull();
  expect(formatDefault(true)).toBe("ja");
});

it("errorList accepts arrays and multi-line strings", () => {
  expect(errorList(["a", "b"])).toEqual(["a", "b"]);
  expect(errorList("a\nb")).toEqual(["a", "b"]);
});

describe("dirty tracking", () => {
  it("an opened file is clean until the form changes", () => {
    const d = openDoc("a.yaml", "h", "x", "id: x", template());
    expect(isDirty(d)).toBe(false);
    d.def!.title = "Anders";
    expect(isDirty(d)).toBe(true);
  });
  it("a file that does not parse opens in the YAML view", () => {
    expect(openDoc("a.yaml", "h", null, "id: [", null).mode).toBe("yaml");
  });
  it("a new draft counts as unsaved", () => {
    expect(isDirty(newDoc())).toBe(true);
  });
});

it("template has every part the engine needs, wired together", () => {
  const t = template();
  const item = t.sections[0].items[0];
  expect(t.answer_sets[t.default_answers!]).toHaveLength(2);
  expect(t.keys[item.key as string]).toHaveLength(2);
  expect(t.scales.map((s) => s.id)).toContain(item.scale);
  expect(t.result).toEqual([item.scale]);
  expect(t.verdict).toEqual([{ else: expect.any(String) }]);
});

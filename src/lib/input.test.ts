import { expect, it } from "vitest";
import { entryItems, initialInput } from "./input";
import type { Definition } from "./types";

const def = {
  schema: 1, id: "beispiel", version: "1", title: "Beispiel-Fragebogen", authors: null, copyright: null,
  cutoff_compare: ">=", answer_sets: {}, shortcuts: {}, keys: {}, default_answers: null,
  fields: [
    { id: "kennung", label: "Kennung", type: "text", default: "anonym" },
    { id: "alter", label: "Alter", type: "number", default: null },
    { id: "erwachsen", label: "Volljährig", type: "checkbox", default: null },
  ],
  scales: [], derived: [], sections: [], result: [], rules: [], caveats: [], verdict: [],
} as Definition;

it("applies defaults, anonym becomes the Kennung", () => {
  expect(initialInput(def, new Date(2026, 10, 11, 22, 23))).toEqual({
    fields: { kennung: "Anonym 11.11.2026 22:23" },
    answers: {},
  });
});

it("lists items in order with their options", () => {
  const d = {
    ...def,
    answer_sets: { haeufigkeit: ["nie", "selten", "oft", "immer"], ja_nein: ["Nein", "Ja"] },
    shortcuts: { ja_nein: "nj" },
    default_answers: "haeufigkeit",
    sections: [
      { title: "T1", intro: null, items: [1, 2, 3].map((n) => ({ n, id: null, text: `F${n}`, answers: null, scale: "X", key: "normal" })) },
      { title: "T2", intro: null, items: [
        { n: 4, id: null, text: "F4", answers: null, scale: "Y", key: "normal" },
        { n: null, id: "zusatz", text: "Zusatzfrage", answers: "ja_nein", scale: null, key: null },
      ] },
    ],
  } as Definition;
  const items = entryItems(d);
  expect(items.map((i) => i.key)).toEqual(["1", "2", "3", "4", "zusatz"]);
  expect(items[4]).toMatchObject({ options: ["Nein", "Ja"], shortcuts: ["n", "j"] });
  expect(items[3]).toMatchObject({ options: ["nie", "selten", "oft", "immer"], section: 1, shortcuts: ["0", "1", "2", "3"] });
});

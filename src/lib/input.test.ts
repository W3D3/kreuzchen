import { expect, it } from "vitest";
import { initialInput } from "./input";
import type { Definition } from "./types";

const def = {
  schema: 1, id: "beispiel", version: "1", title: "Beispiel-Fragebogen", authors: null, copyright: null,
  cutoff_compare: ">=", answer_sets: {}, keys: {}, default_answers: null,
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

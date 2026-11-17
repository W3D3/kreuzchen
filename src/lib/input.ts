import { defaultKennung } from "./kennung";
import type { Definition, FieldValue, Input } from "./types";

export function initialInput(def: Definition, now: Date): Input {
  const fields: Record<string, FieldValue> = {};
  for (const f of def.fields) {
    if (f.default == null) continue;
    fields[f.id] = f.default === "anonym" ? defaultKennung(now) : f.default;
  }
  return { fields, answers: {} };
}

/** `shortcuts[j]` is the key that picks `options[j]`; "" when it has none. */
export interface EntryItem { key: string; text: string; options: string[]; shortcuts: string[]; section: number; scored: boolean }

export function entryItems(def: Definition): EntryItem[] {
  return def.sections.flatMap((s, section) =>
    s.items.map((i) => {
      const set = i.answers ?? def.default_answers ?? "";
      const options = def.answer_sets[set] ?? [];
      const keys = def.shortcuts[set];
      return {
        key: i.id ?? String(i.n),
        text: i.text,
        options,
        shortcuts: options.map((_, j) => (keys ? [...keys][j] : j < 10 ? String(j) : "")),
        section,
        scored: i.scale != null,
      };
    }),
  );
}

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

export interface EntryItem { key: string; text: string; options: string[]; section: number; scored: boolean }

export function entryItems(def: Definition): EntryItem[] {
  return def.sections.flatMap((s, section) =>
    s.items.map((i) => ({
      key: i.id ?? String(i.n),
      text: i.text,
      options: def.answer_sets[i.answers ?? def.default_answers ?? ""] ?? [],
      section,
      scored: i.scale != null,
    })),
  );
}

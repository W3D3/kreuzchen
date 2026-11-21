import { defaultSubjectId } from "./subject";
import type { Definition, FieldValue, Input } from "./types";

export function initialInput(def: Definition, now: Date): Input {
  const fields: Record<string, FieldValue> = {};
  for (const f of def.fields) {
    if (f.default == null) continue;
    fields[f.id] = f.default === "anonymous" ? defaultSubjectId(now) : f.default;
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

/** `input` without answers and fields the (changed) definition no longer has; `dropped` counts them. */
export function fitInput(def: Definition, input: Input): { input: Input; dropped: number } {
  const options = new Map(entryItems(def).map((i) => [i.key, i.options.length]));
  const ids = new Set(def.fields.map((f) => f.id));
  const answers = Object.entries(input.answers).filter(([k, v]) => v < (options.get(k) ?? 0));
  const fields = Object.entries(input.fields).filter(([k]) => ids.has(k));
  const dropped = Object.keys(input.answers).length - answers.length + Object.keys(input.fields).length - fields.length;
  return { input: { fields: Object.fromEntries(fields), answers: Object.fromEntries(answers) }, dropped };
}

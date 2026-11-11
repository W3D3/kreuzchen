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

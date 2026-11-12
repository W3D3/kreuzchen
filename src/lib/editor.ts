import type { Definition, FieldValue, KeyRef } from "./types";

/** The editor's draft. It lives in App so it survives the "Testen" preview. */
export interface Doc {
  file: string | null; // null until the first save
  hash: string | null; // hash of the file as last read or written
  mode: "form" | "yaml";
  def: Definition | null; // what the form view edits
  yaml: string; // the YAML view's text; in form view, the text `def` was parsed from
  snapshot: string; // JSON of `def` when the form view was entered ("" for a new draft)
  dirty: boolean; // set once the YAML text was edited
}

export function openDoc(file: string, hash: string, yaml: string, def: Definition | null): Doc {
  return { file, hash, mode: def ? "form" : "yaml", def, yaml, snapshot: def ? JSON.stringify(def) : "", dirty: false };
}

export const newDoc = (): Doc => ({ file: null, hash: null, mode: "form", def: template(), yaml: "", snapshot: "", dirty: false });

/** True when the form no longer matches the text it was parsed from. */
export const formChanged = (d: Doc) => d.mode === "form" && JSON.stringify(d.def) !== d.snapshot;

export const isDirty = (d: Doc) => d.dirty || formChanged(d);

/** Swaps `list[i]` with its neighbour; does nothing at either end. */
export function move<T>(list: T[], i: number, by: -1 | 1): void {
  const j = i + by;
  if (j < 0 || j >= list.length) return;
  [list[i], list[j]] = [list[j], list[i]];
}

/** Renames a key, keeping the order. Refuses to overwrite another key. */
export function renameKey<V>(rec: Record<string, V>, from: string, to: string): Record<string, V> {
  if (from === to || to in rec) return rec;
  return Object.fromEntries(Object.entries(rec).map(([k, v]) => [k === from ? to : k, v]));
}

/** Picks a key name like `neu`, `neu2`, … not yet in `rec`. */
export function freshKey(rec: Record<string, unknown>, base: string): string {
  let k = base;
  for (let n = 2; k in rec; n++) k = `${base}${n}`;
  return k;
}

// shortcut: comma-separated lists can't hold labels with commas; use the YAML view for those.
export const splitList = (s: string) => s.split(",").map((x) => x.trim()).filter((x) => x !== "");
export const numbers = (s: string) => splitList(s).map(Number);

/** "normal" names a key; "0, 1, 2" is an inline point list; "" is none. */
export function parseKey(s: string): KeyRef | null {
  const t = s.trim();
  if (t === "") return null;
  return /^-?\d+(\s*,\s*-?\d+)*$/.test(t) ? numbers(t) : t;
}
export const formatKey = (k: KeyRef | null) => (k == null ? "" : Array.isArray(k) ? k.join(", ") : k);

/** "12" is an item number `n`; anything else is an `id`. */
export function parseItemRef(s: string): { n: number | null; id: string | null } {
  const t = s.trim();
  if (/^\d+$/.test(t)) return { n: Number(t), id: null };
  return { n: null, id: t === "" ? null : t };
}

export function parseDefault(type: string, s: string): FieldValue | null {
  const t = s.trim();
  if (t === "") return null;
  if (type === "checkbox") return t === "ja";
  if (type === "number") return Number(t);
  return t;
}
export const formatDefault = (v: FieldValue | null) => (v == null ? "" : v === true ? "ja" : v === false ? "nein" : String(v));

/** Errors from a command: `Vec<String>` arrives as an array, `String` as a string. */
export const errorList = (e: unknown): string[] => (Array.isArray(e) ? e.map(String) : String(e).split("\n"));

/** The smallest valid questionnaire. */
export function template(): Definition {
  return {
    schema: 1, id: "neu", version: "1", title: "Neuer Fragebogen", authors: null, copyright: null,
    cutoff_compare: ">=",
    answer_sets: { ja_nein: ["Nein", "Ja"] },
    keys: { normal: [0, 1] },
    default_answers: "ja_nein",
    fields: [{ id: "kennung", label: "Kennung", type: "text", default: "anonym" }],
    scales: [{ id: "A", label: "Skala A", cutoff: 1 }],
    derived: [],
    sections: [{ title: "Teil 1", intro: null, items: [{ n: 1, id: null, text: "Frage 1?", answers: null, scale: "A", key: "normal" }] }],
    result: ["A"],
    rules: [],
    caveats: [],
    verdict: [{ else: "Kein Befund" }],
  };
}

// Mirrors of the Rust wire types (snake_case, Option -> null).
export type FieldValue = boolean | number | string;
export type KeyRef = string | number[];
export type Compare = ">" | ">=";

export interface Field { id: string; label: string; type: "text" | "number" | "date" | "checkbox"; default: FieldValue | null }
export interface Scale { id: string; label: string; cutoff: number | null }
export interface Derived { id: string; label: string; value: string; cutoff: number | null }
export interface Item { n: number | null; id: string | null; text: string; answers: string | null; scale: string | null; key: KeyRef | null }
export interface Section { title: string; intro: string | null; items: Item[] }
export interface Rule { id: string; label: string; when: string }
export interface Caveat { when: string; text: string }
export type VerdictEntry = { when: string; text: string } | { else: string };

export interface Definition {
  schema: number; id: string; version: string; title: string;
  authors: string | null; copyright: string | null;
  cutoff_compare: Compare;
  answer_sets: Record<string, string[]>;
  keys: Record<string, number[]>;
  default_answers: string | null;
  fields: Field[]; scales: Scale[]; derived: Derived[]; sections: Section[];
  result: string[]; rules: Rule[]; caveats: Caveat[]; verdict: VerdictEntry[];
}

export interface Input { fields: Record<string, FieldValue>; answers: Record<string, number> }

export type ResultRow =
  | { kind: "sum"; id: string; label: string; value: number; cutoff: number | null; over: boolean }
  | { kind: "answer"; id: string; label: string; answer: string | null };
export interface RuleOutcome { id: string; label: string; met: boolean }
export interface Grading {
  rows: ResultRow[]; rules: RuleOutcome[]; caveats: string[];
  verdict: string | null; missing: number; missing_items: string[];
}

export interface Summary { id: string; title: string; version: string; item_count: number; scale_count: number }
export interface DefinitionEntry { file: string; hash: string; summary: Summary | null; error: string | null }
export interface OpenedDefinition { file: string; hash: string; definition: Definition }
export interface Graded { grading: Grading; text: string }
export interface QRef { id: string; title: string; version: string; hash: string }
export interface SavedRecord {
  format: number; kennung: string; created_at: string;
  questionnaire: QRef; input: Input; grading: Grading; text: string;
}
export interface SavedSummary { name: string; kennung: string; title: string; created_at: string }
export interface OpenedResult { record: SavedRecord; current: DefinitionEntry | null; outdated: boolean }

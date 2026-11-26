import type { PdfColors } from "./settings";

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
/** Must hold for the result to be interpretable; `invalid` is shown when it does not. */
export interface Validity { require: string; invalid: string }
export type VerdictEntry = { when: string; text: string } | { else: string };

export interface Definition {
  schema: number; id: string; version: string; title: string;
  authors: string | null; copyright: string | null; min_app?: string | null;
  cutoff_compare: Compare;
  answer_sets: Record<string, string[]>;
  shortcuts: Record<string, string>;
  keys: Record<string, number[]>;
  default_answers: string | null;
  fields: Field[]; scales: Scale[]; derived: Derived[]; sections: Section[];
  result: string[]; rules: Rule[]; caveats: Caveat[]; verdict: VerdictEntry[];
  /** Left out by the engine when empty (files older than 0.7.0 have none). */
  validity?: Validity[];
  norms?: Norms | null;
}

export interface Input { fields: Record<string, FieldValue>; answers: Record<string, number> }

/** Raw score → norm value per scale or derived id; JSON object keys are the raw scores. */
export interface Norms { name: string; bands: { from: number; label: string }[]; tables: Record<string, Record<string, number>> }
export interface NormValue { name: string; value: number; band: string | null }
export type ResultRow =
  | { kind: "sum"; id: string; label: string; value: number; cutoff: number | null; over: boolean; norm?: NormValue | null }
  | { kind: "answer"; id: string; label: string; answer: string | null };
export interface RuleOutcome { id: string; label: string; met: boolean }
export interface FieldLine { id: string; label: string; value: string }
export interface Grading {
  fields: FieldLine[];
  rows: ResultRow[]; rules: RuleOutcome[]; caveats: string[];
  /** Texts of failed validity criteria; absent in results saved before 0.7.0. */
  invalid?: string[];
  verdict: string | null; missing: number; missing_items: string[];
}

export interface Summary { id: string; title: string; version: string; item_count: number; scale_count: number }
export interface DefinitionEntry { file: string; hash: string; summary: Summary | null; error: string | null; needs_app: string | null; locked: boolean }
export interface OpenedDefinition { file: string; hash: string; definition: Definition; locked: boolean }
export interface Graded { grading: Grading; text: string }
/** Input of the PDF report; `date` is shown as given. */
export interface Report { title: string; date: string; grading: Grading; norms: Norms | null; colors?: PdfColors }
export interface QRef { id: string; title: string; version: string; hash: string }
export interface SavedRecord {
  format: number; subject_id: string; created_at: string;
  questionnaire: QRef; input: Input; grading: Grading; text: string;
}
export interface SavedSummary { name: string; subject_id: string; title: string; created_at: string }
/** A stored record with the installed definition it belongs to; `outdated` when that definition changed since. */
export interface Opened<T> { record: T; current: DefinitionEntry | null; outdated: boolean }
/** A saved result; `norms` are its definition's, while that is unchanged since grading (for the PDF profile). */
export type OpenedResult = Opened<SavedRecord> & { norms: Norms | null };
export interface Draft { saved_at: string; questionnaire: QRef; input: Input }
export interface DraftSummary { name: string; subject_id: string; title: string; saved_at: string; answered: number }

export type RowState = "available" | "current" | "update" | "gone";
export interface Row { id: string; title: string; state: RowState; file: string | null }
export interface RowsResult { online: boolean; rows: Row[]; error: string | null }
export type InstallOutcome = "installed" | "needs_replace_confirm";
export interface Progress { received: number; total: number | null }
export interface ConnectionInfo { url: string; user: string }

export type TicketKind = "question" | "bug" | "feedback" | "wish";
/** Exactly what the support form sends; the app adds only its version and the OS. */
export interface TicketForm { type: TicketKind; subject: string; message: string; reply_email: string | null }
export interface TicketSummary { id: number; created_at: string; type: TicketKind; subject: string; questionnaire: QRef | null; status: "open" | "done" }

export type EntryState = { focused: number; count: number; shortcuts: string[]; current: number | null };
export type EntryAction =
  | { type: "answer"; index: number }
  | { type: "clear" }
  | { type: "focus"; to: number }
  | { type: "grade" }
  | { type: "none" };

export function entryKey(k: { key: string; shift: boolean; ctrl: boolean }, s: EntryState): EntryAction {
  const next = { type: "focus", to: Math.min(s.focused + 1, s.count - 1) } as const;
  const prev = { type: "focus", to: Math.max(s.focused - 1, 0) } as const;
  const options = s.shortcuts.length;
  const hit = s.shortcuts.indexOf(k.key);
  if (hit >= 0) return { type: "answer", index: hit };
  switch (k.key) {
    case "ArrowRight":
      return { type: "answer", index: s.current === null ? 0 : Math.min(s.current + 1, options - 1) };
    case "ArrowLeft":
      return { type: "answer", index: s.current === null ? options - 1 : Math.max(s.current - 1, 0) };
    case "Enter":
      return k.ctrl ? { type: "grade" } : next;
    case "Tab":
      return k.shift ? prev : next;
    case "ArrowDown":
      return next;
    case "ArrowUp":
      return prev;
    case "Delete":
    case "Backspace":
      return { type: "clear" };
    default:
      return { type: "none" };
  }
}

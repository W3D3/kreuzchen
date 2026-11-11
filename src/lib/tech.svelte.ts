export type TechState = { held: boolean; toggled: boolean };
export type TechEvent =
  | { type: "down" | "up"; key: string; ctrl: boolean; shift: boolean }
  | { type: "blur" };

export function techReducer(s: TechState, ev: TechEvent): TechState {
  if (ev.type === "blur") return { ...s, held: false };
  if (ev.key === "Alt") return { ...s, held: ev.type === "down" };
  if (ev.type === "down" && ev.ctrl && ev.shift && ev.key.toLowerCase() === "d") {
    return { ...s, toggled: !s.toggled };
  }
  return s;
}

export const isTech = (s: TechState) => s.held || s.toggled;

export function installTech(): void {
  let s: TechState = { held: false, toggled: false };
  const apply = (ev: TechEvent) => {
    s = techReducer(s, ev);
    document.body.classList.toggle("tech", isTech(s));
  };
  const key = (type: "down" | "up") => (e: KeyboardEvent) => {
    if (e.key === "Alt") e.preventDefault(); // keep the window menu from grabbing focus
    apply({ type, key: e.key, ctrl: e.ctrlKey, shift: e.shiftKey });
  };
  window.addEventListener("keydown", key("down"));
  window.addEventListener("keyup", key("up"));
  window.addEventListener("blur", () => apply({ type: "blur" }));
}

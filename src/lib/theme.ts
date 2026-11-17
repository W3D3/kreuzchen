export type Theme = "system" | "light" | "dark";

const KEY = "kreuzchen.theme";

export const parseTheme = (raw: string | null): Theme => (raw === "light" || raw === "dark" ? raw : "system");

/** "system" removes the attribute, so `prefers-color-scheme` decides. */
export function applyTheme(root: { dataset: DOMStringMap }, theme: Theme) {
  if (theme === "system") delete root.dataset.theme;
  else root.dataset.theme = theme;
}

export function loadTheme(): Theme {
  try {
    return parseTheme(localStorage.getItem(KEY));
  } catch {
    return "system";
  }
}

export function saveTheme(theme: Theme) {
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // Storage unavailable: the choice applies until the app closes.
  }
  applyTheme(document.documentElement, theme);
}

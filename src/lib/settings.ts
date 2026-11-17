const AUTO_SAVE = "kreuzchen.autoSaveResult";

/** On unless explicitly turned off. */
export const parseAutoSaveResult = (raw: string | null): boolean => raw !== "off";

export function loadAutoSaveResult(): boolean {
  try {
    return parseAutoSaveResult(localStorage.getItem(AUTO_SAVE));
  } catch {
    return true;
  }
}

export function saveAutoSaveResult(on: boolean) {
  try {
    localStorage.setItem(AUTO_SAVE, on ? "on" : "off");
  } catch {
    // Storage unavailable: the choice applies until the app closes.
  }
}

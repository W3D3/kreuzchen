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

const DEV_MODE = "kreuzchen.devMode";

/** Off unless explicitly turned on. */
export const parseDevMode = (raw: string | null): boolean => raw === "on";

export function loadDevMode(): boolean {
  try {
    return parseDevMode(localStorage.getItem(DEV_MODE));
  } catch {
    return false;
  }
}

export function saveDevMode(on: boolean) {
  try {
    localStorage.setItem(DEV_MODE, on ? "on" : "off");
  } catch {
    // Storage unavailable: the choice applies until the app closes.
  }
}

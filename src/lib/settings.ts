const AUTO_SAVE = "kreuzchen.autoSaveResult";
const DEV_MODE = "kreuzchen.devMode";

/** On unless explicitly turned off. */
export const parseAutoSaveResult = (raw: string | null): boolean => raw !== "off";
/** Off unless explicitly turned on. */
export const parseDevMode = (raw: string | null): boolean => raw === "on";

// Without storage a switch keeps its default, and a change applies until the app closes.
function load(key: string, parse: (raw: string | null) => boolean): boolean {
  try {
    return parse(localStorage.getItem(key));
  } catch {
    return parse(null);
  }
}

function store(key: string, on: boolean) {
  try {
    localStorage.setItem(key, on ? "on" : "off");
  } catch {
    // See `load`.
  }
}

export const loadAutoSaveResult = () => load(AUTO_SAVE, parseAutoSaveResult);
export const saveAutoSaveResult = (on: boolean) => store(AUTO_SAVE, on);
export const loadDevMode = () => load(DEV_MODE, parseDevMode);
export const saveDevMode = (on: boolean) => store(DEV_MODE, on);

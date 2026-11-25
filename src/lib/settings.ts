/** On unless explicitly turned off. */
export const parseFlag = (raw: string | null): boolean => raw !== "off";

/** An on/off setting in localStorage, on by default. */
const flag = (key: string) => ({
  load(): boolean {
    try {
      return parseFlag(localStorage.getItem(key));
    } catch {
      return true;
    }
  },
  save(on: boolean) {
    try {
      localStorage.setItem(key, on ? "on" : "off");
    } catch {
      // Storage unavailable: the choice applies until the app closes.
    }
  },
});

export const autoSaveResult = flag("kreuzchen.autoSaveResult");
export const groupSavedByDate = flag("kreuzchen.groupSavedByDate");

import { check, type Update } from "@tauri-apps/plugin-updater";

/** Shared between the start check (App) and the manual check (Einstellungen). */
export const updates = $state<{
  available: Update | null;
  checking: boolean;
  /** Outcome of the last check that found nothing: up to date, or server unreachable. */
  result: "current" | "offline" | null;
  installing: boolean;
  error: string | null;
}>({ available: null, checking: false, result: null, installing: false, error: null });

export async function findUpdate() {
  updates.checking = true;
  updates.result = null;
  try {
    updates.available = await check();
    if (!updates.available) updates.result = "current";
  } catch {
    updates.result = "offline";
  } finally {
    updates.checking = false;
  }
}

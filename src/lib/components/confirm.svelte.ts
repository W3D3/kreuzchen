export type ConfirmOptions = { title: string; message: string; confirmLabel: string; destructive?: boolean };

/** `request` stays set after closing, so the dialog keeps its text while it fades out. */
export const confirmState = $state<{ open: boolean; request: (ConfirmOptions & { resolve: (ok: boolean) => void }) | null }>({
  open: false,
  request: null,
});

/** In-app replacement for the native `ask`; ConfirmDialog in App.svelte shows it. */
export function confirm(opts: ConfirmOptions): Promise<boolean> {
  answer(false);
  return new Promise((resolve) => {
    confirmState.request = { ...opts, resolve };
    confirmState.open = true;
  });
}

/** Closes the open request with the user's answer; does nothing when none is open. */
export function answer(ok: boolean) {
  if (!confirmState.open) return;
  confirmState.open = false;
  confirmState.request!.resolve(ok);
}

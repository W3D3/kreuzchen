/**
 * Keeps one stored copy of a changing value: one write at a time, and a burst of
 * changes during a write collapses into a single follow-up write of the latest value.
 * `write` gets the stored name (null before the first write) and returns it.
 */
export function createAutosave<T>(
  name: string | null,
  write: (name: string | null, value: T) => Promise<string>,
  remove: (name: string) => Promise<void>,
  onerror: (e: unknown) => void,
) {
  let next: { value: T } | null = null;
  let running: Promise<void> | null = null;
  let failed: unknown = null;
  let discarded = false;

  async function drain() {
    while (next && !discarded) {
      const { value } = next;
      next = null;
      try {
        name = await write(name, value);
        failed = null;
      } catch (e) {
        failed = e;
        onerror(e);
      }
    }
    running = null;
  }

  return {
    get name() { return name; },
    push(value: T) {
      if (discarded) return;
      next = { value };
      running ??= drain();
    },
    /** Waits for pending writes; rejects when the last one failed. */
    async flush(): Promise<string | null> {
      await running;
      if (failed) throw failed;
      return name;
    },
    /** Stops saving and deletes the stored copy. */
    async discard() {
      discarded = true;
      await running;
      if (name) await remove(name);
      name = null;
    },
  };
}

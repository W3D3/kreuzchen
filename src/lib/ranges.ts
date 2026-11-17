/** "2, 3, 4, 9" → "2–4, 9"; ids that are not numbers stay as they are. */
export function compactItems(items: string[]): string {
  const parts: string[] = [];
  let start: number | null = null;
  let prev: number | null = null;
  const flush = () => {
    if (start !== null) parts.push(start === prev ? String(start) : `${start}–${prev}`);
    start = prev = null;
  };
  for (const item of items) {
    const n = /^\d+$/.test(item) ? Number(item) : null;
    if (n !== null && prev !== null && n === prev + 1) {
      prev = n;
      continue;
    }
    flush();
    if (n === null) parts.push(item);
    else start = prev = n;
  }
  flush();
  return parts.join(", ");
}

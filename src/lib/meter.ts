/** Bar width in percent with the cut-off at the middle; a cut-off of 0 or less cannot be scaled, so only the side shows. */
export function meter(value: number, cutoff: number): number {
  if (cutoff <= 0) return value > cutoff ? 100 : value === cutoff ? 50 : 0;
  return Math.min(Math.max(value / (cutoff * 2), 0), 1) * 100;
}

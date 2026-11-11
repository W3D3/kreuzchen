const p = (n: number) => String(n).padStart(2, "0");

export const defaultKennung = (d: Date): string =>
  `Anonym ${p(d.getDate())}.${p(d.getMonth() + 1)}.${d.getFullYear()} ${p(d.getHours())}:${p(d.getMinutes())}`;

const p = (n: number) => String(n).padStart(2, "0");

export const defaultKennung = (d: Date): string =>
  `Anonym ${p(d.getDate())}.${p(d.getMonth() + 1)}.${d.getFullYear()} ${p(d.getHours())}:${p(d.getMinutes())}`;

export const localTimestamp = (d: Date): string =>
  `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;

export const formatCreatedAt = (s: string): string => {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(s);
  return m ? `${m[3]}.${m[2]}.${m[1]} ${m[4]}:${m[5]}` : s;
};

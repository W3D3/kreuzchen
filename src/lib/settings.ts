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

const PDF_COLORS = "kreuzchen.pdfColors";

export type PdfColors = { accent: string; dot: string; line: string; over: string; row: string };

/** The PDF's own colours, `Colors::default()` in `src-tauri/src/report.rs`. */
export const DEFAULT_PDF_COLORS: PdfColors = {
  accent: "#1f6f78",
  dot: "#b42318",
  line: "#5f6f75",
  over: "#b4531a",
  row: "#ebeff0",
};

/** Stored colours, each `#rrggbb` or its default. */
export function parsePdfColors(raw: string | null): PdfColors {
  let stored: Record<string, unknown> = {};
  try {
    stored = Object(JSON.parse(raw ?? "{}"));
  } catch {
    // Unreadable: the defaults.
  }
  const pick = (key: keyof PdfColors) => {
    const v = stored[key];
    return typeof v === "string" && /^#[0-9a-f]{6}$/i.test(v) ? v.toLowerCase() : DEFAULT_PDF_COLORS[key];
  };
  return { accent: pick("accent"), dot: pick("dot"), line: pick("line"), over: pick("over"), row: pick("row") };
}

/** The PDF colours in localStorage; saving `null` goes back to the defaults. */
export const pdfColors = {
  load(): PdfColors {
    try {
      return parsePdfColors(localStorage.getItem(PDF_COLORS));
    } catch {
      return DEFAULT_PDF_COLORS;
    }
  },
  save(colors: PdfColors | null) {
    try {
      if (colors) localStorage.setItem(PDF_COLORS, JSON.stringify(colors));
      else localStorage.removeItem(PDF_COLORS);
    } catch {
      // Storage unavailable: the choice applies until the app closes.
    }
  },
};

/** Fill opacity of the middle range, `SHADE_ALPHA` in `src-tauri/src/report/typeset.rs`. */
export const SHADE_ALPHA = 0.16;

const channels = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
const luminance = (rgb: number[]) => {
  const [r, g, b] = rgb.map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

/** WCAG contrast of the profile markers against the shaded middle range, on a white and a striped
 *  row; luminance alone, so it is what stays of them in grayscale. */
export function dotContrast(c: PdfColors): number {
  const dot = luminance(channels(c.dot));
  const accent = channels(c.accent);
  return Math.min(
    ...["#ffffff", c.row].map((bg) => {
      const band = luminance(channels(bg).map((v, i) => SHADE_ALPHA * accent[i] + (1 - SHADE_ALPHA) * v));
      return (Math.max(dot, band) + 0.05) / (Math.min(dot, band) + 0.05);
    }),
  );
}

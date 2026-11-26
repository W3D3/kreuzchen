import { expect, it } from "vitest";
import { DEFAULT_PDF_COLORS, dotContrast, parseFlag, parsePdfColors } from "./settings";

it("on/off settings are on unless turned off", () => {
  expect(parseFlag(null)).toBe(true);
  expect(parseFlag("on")).toBe(true);
  expect(parseFlag("off")).toBe(false);
});

it("pdf colours keep each valid hex colour and default the rest", () => {
  expect(parsePdfColors(null)).toEqual(DEFAULT_PDF_COLORS);
  expect(parsePdfColors("not json")).toEqual(DEFAULT_PDF_COLORS);
  expect(parsePdfColors("42")).toEqual(DEFAULT_PDF_COLORS);
  expect(parsePdfColors(JSON.stringify({ dot: "#00AA00", accent: "red", line: "#12345", row: 7 }))).toEqual({
    ...DEFAULT_PDF_COLORS,
    dot: "#00aa00",
  });
});

it("markers too close to the middle range in grayscale have low contrast", () => {
  expect(dotContrast(DEFAULT_PDF_COLORS)).toBeGreaterThan(3);
  // Light grey markers on the light band vanish in a black-and-white print.
  expect(dotContrast({ ...DEFAULT_PDF_COLORS, dot: "#dddddd" })).toBeLessThan(3);
});

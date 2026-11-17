import { expect, it } from "vitest";
import { applyTheme, parseTheme } from "./theme";

it("parses stored values, falling back to system", () => {
  expect(parseTheme("light")).toBe("light");
  expect(parseTheme("dark")).toBe("dark");
  expect(parseTheme("system")).toBe("system");
  expect(parseTheme(null)).toBe("system");
  expect(parseTheme("blau")).toBe("system");
});

it("sets data-theme only for an explicit choice", () => {
  const root = { dataset: {} as Record<string, string> };
  applyTheme(root, "dark");
  expect(root.dataset.theme).toBe("dark");
  applyTheme(root, "system");
  expect(root.dataset.theme).toBeUndefined();
});

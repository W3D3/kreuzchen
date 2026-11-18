import { expect, it } from "vitest";
import { parseAutoSaveResult } from "./settings";

it("auto-saving results is on unless turned off", () => {
  expect(parseAutoSaveResult(null)).toBe(true);
  expect(parseAutoSaveResult("on")).toBe(true);
  expect(parseAutoSaveResult("off")).toBe(false);
});

import { expect, it } from "vitest";
import { parseFlag } from "./settings";

it("on/off settings are on unless turned off", () => {
  expect(parseFlag(null)).toBe(true);
  expect(parseFlag("on")).toBe(true);
  expect(parseFlag("off")).toBe(false);
});

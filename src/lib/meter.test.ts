import { expect, it } from "vitest";
import { meter } from "./meter";

it("puts the cut-off at the middle and clamps", () => {
  expect(meter(5, 10)).toBe(25);
  expect(meter(10, 10)).toBe(50);
  expect(meter(30, 10)).toBe(100);
  expect(meter(-1, 10)).toBe(0);
});

it("stays a number with a cut-off of 0", () => {
  expect(meter(0, 0)).toBe(50);
  expect(meter(3, 0)).toBe(100);
  expect(meter(-1, 0)).toBe(0);
});

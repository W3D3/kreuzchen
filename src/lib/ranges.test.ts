import { expect, it } from "vitest";
import { compactItems } from "./ranges";

it("joins consecutive item numbers into ranges", () => {
  expect(compactItems(["2", "3", "4", "5", "9", "11", "12"])).toBe("2–5, 9, 11–12");
  expect(compactItems(["7"])).toBe("7");
});

it("keeps non-numeric ids as they are, in place", () => {
  expect(compactItems(["1", "2", "3", "zusatz", "5"])).toBe("1–3, zusatz, 5");
});

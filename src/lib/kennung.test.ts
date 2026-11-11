import { expect, it } from "vitest";
import { defaultKennung } from "./kennung";

it("formats with zero padding", () => {
  expect(defaultKennung(new Date(2026, 10, 11, 22, 23))).toBe("Anonym 11.11.2026 22:23");
  expect(defaultKennung(new Date(2026, 0, 5, 7, 4))).toBe("Anonym 05.01.2026 07:04");
});

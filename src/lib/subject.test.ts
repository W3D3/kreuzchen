import { expect, it } from "vitest";
import { defaultSubjectId, formatCreatedAt, localTimestamp, pdfName } from "./subject";

it("formats with zero padding", () => {
  expect(defaultSubjectId(new Date(2026, 10, 11, 22, 23))).toBe("Anonym 11.11.2026 22:23");
  expect(defaultSubjectId(new Date(2026, 0, 5, 7, 4))).toBe("Anonym 05.01.2026 07:04");
});

it("formats a local timestamp", () => {
  expect(localTimestamp(new Date(2026, 0, 5, 7, 4, 9))).toBe("2026-01-05T07:04:09");
});

it("formats created_at for display", () => {
  expect(formatCreatedAt("2026-01-05T07:04:09")).toBe("05.01.2026 07:04");
  expect(formatCreatedAt("yesterday")).toBe("yesterday");
});

it("names the PDF after the subject id and title", () => {
  expect(pdfName("Anonym 05.01.2026 07:04", "Test A/B")).toBe("Anonym 05.01.2026 07-04 – Test A-B.pdf");
  expect(pdfName("", "Test")).toBe("Test.pdf");
});

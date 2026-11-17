import { expect, it } from "vitest";
import { answer, confirm, confirmState } from "./confirm.svelte";

const opts = { title: "Löschen", message: "Wirklich löschen?", confirmLabel: "Löschen" };

it("resolves true on confirm and closes", async () => {
  const p = confirm(opts);
  expect(confirmState.open).toBe(true);
  expect(confirmState.request?.message).toBe("Wirklich löschen?");
  answer(true);
  expect(confirmState.open).toBe(false);
  await expect(p).resolves.toBe(true);
});

it("resolves false on cancel, Esc or overlay", async () => {
  const p = confirm(opts);
  answer(false);
  await expect(p).resolves.toBe(false);
});

it("answers only once", async () => {
  const p = confirm(opts);
  answer(false);
  answer(true);
  await expect(p).resolves.toBe(false);
});

it("a second confirm resolves the first as false", async () => {
  const first = confirm(opts);
  const second = confirm({ ...opts, message: "Zweite Frage?" });
  await expect(first).resolves.toBe(false);
  expect(confirmState.request?.message).toBe("Zweite Frage?");
  answer(true);
  await expect(second).resolves.toBe(true);
});

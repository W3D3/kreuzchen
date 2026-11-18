import { afterEach, expect, it, vi } from "vitest";
import { loadDevMode, parseAutoSaveResult, parseDevMode, saveDevMode } from "./settings";

afterEach(() => vi.unstubAllGlobals());

it("auto-saving results is on unless turned off", () => {
  expect(parseAutoSaveResult(null)).toBe(true);
  expect(parseAutoSaveResult("on")).toBe(true);
  expect(parseAutoSaveResult("off")).toBe(false);
});

it("developer mode is off unless turned on", () => {
  expect(parseDevMode(null)).toBe(false);
  expect(parseDevMode("off")).toBe(false);
  expect(parseDevMode("on")).toBe(true);
});

it("developer mode survives a restart", () => {
  const store = new Map<string, string>();
  vi.stubGlobal("localStorage", { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => store.set(k, v) });
  expect(loadDevMode()).toBe(false);
  saveDevMode(true);
  expect(loadDevMode()).toBe(true);
  saveDevMode(false);
  expect(loadDevMode()).toBe(false);
});

it("developer mode stays off without storage", () => {
  vi.stubGlobal("localStorage", { getItem: () => { throw new Error("denied"); }, setItem: () => { throw new Error("denied"); } });
  expect(() => saveDevMode(true)).not.toThrow();
  expect(loadDevMode()).toBe(false);
});

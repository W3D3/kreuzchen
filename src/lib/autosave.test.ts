import { expect, it } from "vitest";
import { createAutosave } from "./autosave";

/** A fake store whose writes finish only when the test says so. */
function store() {
  const files = new Map<string, number>();
  const waiting: (() => void)[] = [];
  const writes: [string | null, number][] = [];
  let fail = false;
  const write = (name: string | null, v: number) =>
    new Promise<string>((resolve, reject) => {
      writes.push([name, v]);
      waiting.push(() => {
        if (fail) return reject(new Error("voll"));
        const n = name ?? `d${files.size + 1}`;
        files.set(n, v);
        resolve(n);
      });
    });
  const remove = async (n: string) => void files.delete(n);
  const step = async () => { waiting.shift()?.(); await new Promise((r) => setTimeout(r)); };
  return { files, writes, write, remove, step, setFail: (f: boolean) => (fail = f) };
}

it("creates once, then overwrites; a burst collapses into the latest value", async () => {
  const s = store();
  const a = createAutosave<number>(null, s.write, s.remove, () => {});
  a.push(1);
  a.push(2);
  a.push(3);
  await s.step();
  await s.step();
  expect(await a.flush()).toBe("d1");
  expect(s.writes).toEqual([[null, 1], ["d1", 3]]);
  expect([...s.files]).toEqual([["d1", 3]]);
});

it("continues an existing draft under its name", async () => {
  const s = store();
  const a = createAutosave<number>("alt", s.write, s.remove, () => {});
  a.push(7);
  await s.step();
  expect(s.writes).toEqual([["alt", 7]]);
});

it("reports a failed write, and flush rejects until a later write succeeds", async () => {
  const s = store();
  const errors: unknown[] = [];
  const a = createAutosave<number>(null, s.write, s.remove, (e) => errors.push(e));
  s.setFail(true);
  a.push(1);
  await s.step();
  expect(errors).toHaveLength(1);
  await expect(a.flush()).rejects.toThrow("voll");
  s.setFail(false);
  a.push(2);
  await s.step();
  expect(await a.flush()).toBe("d1");
});

it("discard waits for the write in flight, deletes it, and ignores later changes", async () => {
  const s = store();
  const a = createAutosave<number>(null, s.write, s.remove, () => {});
  a.push(1);
  a.push(2);
  const done = a.discard();
  await s.step();
  await done;
  a.push(3);
  expect(s.writes).toEqual([[null, 1]]);
  expect(s.files.size).toBe(0);
  expect(a.name).toBeNull();
});

import { describe, expect, it } from "vitest";
import { rowView } from "./registry";
import type { Row } from "./types";

const row = (state: Row["state"], file: string | null): Row => ({ id: "fragebogen_a", title: "Fragebogen A", state, file });

describe("rowView", () => {
  it("labels_and_actions", () => {
    expect(rowView(row("available", null), true)).toEqual({ label: "verfügbar", action: "Installieren", remove: false });
    expect(rowView(row("current", "fragebogen_a.yaml"), true)).toEqual({ label: "aktuell", action: null, remove: true });
    expect(rowView(row("update", "fragebogen_a.yaml"), true)).toEqual({ label: "neue Version verfügbar", action: "Aktualisieren", remove: true });
    expect(rowView(row("gone", "fragebogen_a.yaml"), true)).toEqual({ label: "nicht mehr auf dem Server", action: null, remove: true });
  });
  it("offline shows no state and only Entfernen", () => {
    expect(rowView(row("current", "fragebogen_a.yaml"), false)).toEqual({ label: null, action: null, remove: true });
  });
});

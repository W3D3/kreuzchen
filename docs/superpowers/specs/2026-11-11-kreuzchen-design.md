# Kreuzchen: design

Date: 2026-11-11
Status: approved 2026-11-11
Mockups: https://claude.ai/artifact/RaX3pvQ7RfQTxQwScT7pfh

## Intent

Clinic staff grade printed psychological questionnaires by hand. They lay a printed scoring overlay over the filled-in form, add up points per scale, compare the sums to cut-offs, and fill out a summary sheet. This app replaces the overlay and the arithmetic:

1. A staff member picks a questionnaire.
2. They copy the answers from the paper form into the app by clicking radio buttons.
3. The app shows the full grading: every scale sum, cut-offs, criteria and verdict.

The grading rules for each questionnaire live in a YAML file that the developer ships. Clinics copy these files into a folder. Changing a grading rule or adding a questionnaire never needs a new app release.

The users are not technical. The app must feel like a simple form, not a tool.

The first questionnaire to support is the **MBAS** (Marburger Beurteilungsskala zum Asperger-Syndrom). Its scoring overlay and summary sheet are the reference for what the config format must express. See "MBAS as the reference case" below.

## Goals

- Enter answers with radio buttons that mirror the paper form.
- Grade from a YAML definition loaded at runtime from the app's data folder.
- Show **every scale sum** with its cut-off and an "über Cut-off" marker. Below them, show criteria, the verdict, caveats and the number of missing answers.
- Copy the result as plain text with one click.
- Optionally save a result, list saved results in the app, and export or import a single result as a file.
- Run offline on Windows, macOS and Linux as a lightweight native app.

## Non-goals (v1)

- An in-app editor for questionnaire definitions. It may come later, so keep the YAML format simple enough to edit through a UI.
- Any language but German. There is no i18n layer. Questionnaire texts come from the YAML.
- Network features of any kind: no sync, accounts, telemetry, auto-update or crash reporting.
- Printing (copy covers it). Saving is optional and low priority compared to entry and grading.

## Anti-goals (failure even if it works)

- **Patient names on disk by default.** A result is identified by "Anonym <Datum> <Uhrzeit>" unless staff deliberately type something else.
- **Technical clutter in front of clinic staff.** File names, paths, YAML errors and IDs are hidden. They show only while Alt is held, or after a fallback shortcut.
- **A grading bug that changes a clinical result silently.** Grading must be covered by tests against hand-checked expected values.
- **Electron.** The app must stay small.

## Users and flow

Clinic staff (psychologists, assistants) with the paper form on the desk.

Screens, matching the mockups:

1. **Fragebogen wählen.** Lists every valid questionnaire in the data folder by title, item count and version. A broken file appears as "Dieser Fragebogen kann nicht geöffnet werden. Bitte wenden Sie sich an den Support." It can't be started. Holding Alt shows the file name, the data folder path and the exact parse or validation error, plus "Ordner öffnen" and "Neu laden" buttons.
2. **Antworten eingeben.**
   - Header fields from the YAML: Kennung, prefilled with "Anonym 11.11.2026 22:23" (current local date and time), plus other fields such as Alter. Applicability checkboxes from the YAML.
   - Items grouped by the sections of the paper form ("Seite 1" …). Each item shows the answer labels ("niemals" … "immer") as radio buttons. **Points are never shown during entry**, because the reversal of an item's scoring happens only in grading.
   - A sticky progress bar ("23 / 51 beantwortet") with the "Auswerten" button.
   - Unanswered items are marked but **never block** grading.
   - **Keyboard entry must be quick and obvious.** One question is focused at a time, with a clear highlight:
     - Number keys pick the answer by position (1 = niemals … 5 = immer; 1 = Nein, 2 = Ja). Points are never typed. Picking an answer moves to the next question.
     - Tab and Enter move to the next question; Shift+Tab goes back. Arrow keys move between answers.
     - Each radio button shows its key number as a small label.
     - A permanent shortcut bar at the bottom of the entry screen lists the keys in German (e.g. "1–5 Antwort · Enter/Tab nächste Frage · Umschalt+Tab zurück · Strg+Enter Auswerten").
3. **Ergebnis.**
   - Kennung and header fields.
   - A row per scale and for the total: sum, cut-off, a bar with a cut-off marker, and an "über" pill when over.
   - Yes/no scales (MBAS scale 5).
   - A verdict box listing each criterion as met or not met, the verdict text, and the missing-answer count.
   - The applicability caveat, when it applies.
   - "Als Text kopieren" copies a plain-text block of all of the above.
   - "Speichern" saves the result.
   - Going back to the answers keeps them.
4. **Gespeicherte Auswertungen.** A list of saved results by Kennung, questionnaire and date. It can open, delete, export to a file, or import from a file.

## The questionnaire definition (YAML)

One file per questionnaire in `<app data dir>/definitionen/`. The format must express everything below. Field names are a sketch; the builder may rename them, but not drop capabilities.

- **Identity:** `id`, `version`, `title`, an optional short description, and the copyright line to show.
- **Answer sets:** named lists of labels, e.g. `haeufigkeit: [niemals, selten, manchmal, häufig, immer]` or `ja_nein: [Nein, Ja]`.
- **Scoring keys:** named point maps per answer index, e.g. `normal: [0,1,2,3,4]` and `umgepolt: [4,3,2,1,0]`. An item may also give an inline point list.
- **Header fields:** text, number, date and checkbox fields with ID, label and optional default. The Kennung field's default is the anonymous timestamp.
- **Sections:** an ordered list with a title and optional instruction text, holding items.
- **Items:** ID or number, text, answer set, scoring key, and the scale it counts toward (or none).
- **Scales:** ID, display label, and an optional cut-off. A scale's value is the sum of its items' points. Yes/no scales are items whose answer set is yes/no and that the result shows as the answer, not a sum.
- **Cut-off comparison:** a questionnaire-wide `cutoff_compare` (`>` or `>=`) defines what "über Cut-off" means for every scale and derived value. It drives the "über" marker on the result. Rules read it as `X.over`, so the comparison is written in exactly one place.
- **Derived values:** named sums over scales, e.g. `gesamt = A + B + C + D`, each with an optional cut-off.
- **Rules:** named conditions that produce a met/not-met line on the result, and caveat or verdict texts. Conditions use a **small expression language**:
  - numbers, booleans, `and` / `or` / `not`, comparisons, and `+`
  - references to scale sums, derived values, cut-offs, header fields, checkboxes, item answers, and the built-in `fehlend` (count of missing answers)
  - no loops, functions, I/O or scripting
  - the parser may be a small existing crate (e.g. `evalexpr`) or hand-written; builder's call
- **Verdict:** an ordered list of `{when, text}`. The first match is shown. An optional fallback text applies when nothing matches.

Validation on load must catch unknown scale or answer-set references, duplicate IDs, scoring keys whose length doesn't match the answer set, and expressions that don't parse or name unknown values. The errors are what the Alt view shows. They should name the item or line.

### MBAS as the reference case

From the Auswertungsblatt and Auswertungsfolien (`docs/reference/MBAS_Auswertungsanleitung.pdf`, in this repo):

- 51 items, answers "niemals, selten, manchmal, häufig, immer".
- Each item is printed "01234" (normal) or "43210" (reversed) on the overlay, and is assigned to scale A, B, C or D.
- Scale cut-offs on the scale sum:

  | Scale | Name | Cut-off |
  |---|---|---|
  | 1 (A) | Theory of Mind, Kontakt- & Spielverhalten | 38 |
  | 2 (B) | geteilte Aufmerksamkeit & Freude, Mimik, Gestik | 21 |
  | 3 (C) | stereotypes und situationsinadäquates Verhalten | 20 |
  | 4 (D) | auffälliger Sprachstil, Sonderinteressen, Motorik | 16 |
  | Gesamtsumme | A + B + C + D | 103 |

- Scale 5 is "Sprachentwicklungsverzögerung" (Ja/Nein). It is not one of the 51 items, so the clinician enters it as an extra yes/no question. It is separate diagnostic information: the result shows it, but **it never feeds a criterion or the verdict**.
- Applicability checkboxes:
  - Lebensalter > 24 Jahre: Ja triggers the caveat.
  - Lebensalter < 6 Jahre: Ja triggers the caveat.
  - Durchschnittliche Intelligenz: Nein triggers the caveat.
  - Caveat text: "Anwendbarkeit der MBAS nur unter Vorbehalt".
- Kriterium 1: Gesamtsumme über Cut-off.
- Kriterium 2: Gesamtsumme und Skala 1 über Cut-off.
- The sheet reports "Anzahl fehlender Antworten" and a "Verdachtsdiagnose".

A full transcription exists at `local/definitionen/mbas.yaml`. It is the reference example of the schema and must be checked by hand against the printed overlay. Items 29 and 33–37 need extra care because the PDF's text layout is broken there. The page-wise "Zwischensumme" on the overlay is a paper aid and is not needed.

## Grading semantics

- For MBAS, "über Cut-off" means **greater than or equal** (`>=`). This is decided, and it is encoded as `cutoff_compare: ">="` in the YAML, not in app code.
- A missing answer adds **0 points** and increases `fehlend`. Whether too many missing answers invalidates the result is a rule in the YAML, not app logic.
- The MBAS verdict texts are not on the sheet. My draft:
  - Kriterium 2 met: "Verdacht auf Asperger-Syndrom (Kriterium 2 erfüllt)".
  - Only Kriterium 1 met: "Verdacht auf Asperger-Syndrom (Kriterium 1 erfüllt)".
  - Otherwise: "Kein Verdacht nach MBAS".
  - These need clinical sign-off.

## Saved results

- Saved under `<app data dir>/auswertungen/`, one JSON file per result.
- Each file contains: Kennung, timestamp, questionnaire `id`, `version`, a hash of the definition file, header field values, every answer, and a snapshot of the computed result.
- Opening a saved result shows the snapshot. If the installed definition for that `id` has a different version or hash, the app says so and offers "Neu auswerten", which grades the stored answers with the current file. The snapshot is never overwritten silently.
- Export writes the same JSON to a file the user picks. Import reads one back into the list.
- No encryption at rest in v1; rely on OS disk encryption. Results hold no names unless staff type one.

## Technical choices (my calls)

- **Tauri 2.** The Rust core holds file access and the grading engine. A small web UI in Svelte + TypeScript.
- **The grading engine is its own Rust crate** with no Tauri dependency: parse, validate, grade. The UI only renders what it returns. This keeps grading in one place and testable without a UI.
- YAML parsing is the builder's choice. Use a maintained crate; `serde_yaml` is archived, so prefer a maintained fork or alternative.
- Lock the webview down: no remote URLs, strict CSP, and only the Tauri capabilities the app uses (file access scoped to the data dir, plus dialogs for export and import).
- Data dir: Tauri's per-OS app data dir. On first start, create `definitionen/` and `auswertungen/` there. **No questionnaire is bundled.** With an empty folder, the list screen says "Noch keine Fragebögen installiert. Bitte wenden Sie sich an den Support." The Alt view shows the folder path and "Ordner öffnen".
- Alt reveal: holding Alt shows technical details. Add a toggle shortcut as a fallback (e.g. Ctrl+Shift+D), because some desktops and webviews swallow Alt.

## Repository and copyrighted material

- Private GitHub repo.
- Copyrighted material stays in the gitignored `local/` folder and is never committed or bundled. That covers the MBAS PDF and `local/definitionen/mbas.yaml`.
- The developer copies definitions to clinic machines by hand.
- A committed, invented questionnaire (`beispiel.yaml`) uses every schema feature: reversed keys, a yes/no item, derived values, rules, caveats and verdicts. It is the fixture that always runs.

## Distribution

- No code signing.
- Ship plain installers per platform: `.msi`/`.exe` for Windows, `.dmg` for macOS, `.AppImage`/`.deb` for Linux.
- Write a short German install guide with screenshots for getting past SmartScreen ("Weitere Informationen" → "Trotzdem ausführen") and Gatekeeper (right-click → "Öffnen", or System Settings → "Trotzdem öffnen").

## Testing

- Engine tests on the committed `beispiel.yaml` always run.
- MBAS tests run when `local/definitionen/mbas.yaml` exists and are skipped otherwise, so they run on the developer's machine but not in CI.
- Answer fixtures produce expected scale sums, criteria and verdicts. At least:
  - all answers 0
  - all answers 4
  - one hand-scored realistic case checked against the paper overlay
  - exactly-at-cut-off cases on both sides
  - missing answers
- Validation tests: each kind of broken YAML produces a readable error naming the item or line.
- One end-to-end check per platform build: pick MBAS, enter answers, grade, copy, save, reopen.

## Risks

- **Copyright.** The MBAS item texts and scoring are copyrighted (© Klinik für Psychiatrie und Psychotherapie des Kindes- und Jugendalters, Universität Marburg). They never enter the repo or the installer. Distributing the YAML to clinics may still need permission.
- **Medical device regulation.** Software that computes a "Verdachtsdiagnose" may count as a medical device under EU MDR. This is not a design decision, but it affects how the app may be distributed and labelled.
- **Transcription errors.** One wrong "01234" vs "43210" changes results. The hand-checked fixture is the guard.
- **Unsigned installers** trigger SmartScreen and Gatekeeper warnings. The install guide covers them, but a future macOS version may block unsigned apps outright.

## Left to the builder

Project layout, crate and library choices within the constraints above, YAML field names, expression parser, keyboard shortcuts, visual styling (the mockups set the direction: calm, readable, large click targets), and the app identifier.

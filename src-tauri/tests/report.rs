//! Renders the example definitions to PDF under `target/tmp/` for a look, and checks their content.
use kreuzchen_engine::{Input, schema::FieldValue};
use kreuzchen_lib::report::{Report, pdf};
use std::{collections::BTreeMap, fs};

fn example(name: &str, answers: &[(&str, usize)]) -> String {
    let yaml = fs::read_to_string(format!("../examples/definitions/{name}.yaml")).unwrap();
    let answers: Vec<_> = answers.iter().map(|&(k, v)| (k.to_string(), v)).collect();
    render(name, &yaml, answers)
}

/// `n` one-item scales with stanine tables and no bands, answered in a zigzag.
fn stanine(name: &str, n: usize) -> String {
    let mut yaml = String::from(
        "schema: 1\nid: stanine\nversion: \"1\"\ntitle: Profilbogen mit Stanine-Werten\n\
         answer_sets: { stufen: [\"1\", \"2\", \"3\", \"4\", \"5\", \"6\", \"7\", \"8\", \"9\"] }\n\
         keys: { k: [0, 1, 2, 3, 4, 5, 6, 7, 8] }\ndefault_answers: stufen\n\
         fields: [{ id: subject_id, label: Kennung, type: text }]\nscales:\n",
    );
    let ids: Vec<String> = (1..=n).map(|i| format!("s{i}")).collect();
    for (i, id) in ids.iter().enumerate() {
        // Every fourth name is long enough to wrap onto more lines.
        let more = if i % 4 == 3 {
            ", der über mehrere Zeilen umbricht"
        } else {
            ""
        };
        yaml += &format!("  - {{ id: {id}, label: \"Skala {id} mit längerem Namen{more}\" }}\n");
    }
    yaml += "sections:\n  - title: Fragen\n    items:\n";
    for (i, id) in ids.iter().enumerate() {
        yaml += &format!(
            "      - {{ n: {}, scale: {id}, key: k, text: \"Frage\" }}\n",
            i + 1
        );
    }
    yaml += &format!(
        "result: [{}]\nnorms:\n  name: Stanine\n  tables:\n",
        ids.join(", ")
    );
    for id in &ids {
        yaml += &format!("    {id}: {{ 0: 1, 1: 2, 2: 3, 3: 4, 4: 5, 5: 6, 6: 7, 7: 8, 8: 9 }}\n");
    }
    let answers = (1..=n)
        .map(|i| (i.to_string(), [4, 6, 3, 5, 8, 4, 1, 5, 7, 2][i % 10]))
        .collect();
    render(name, &yaml, answers)
}

/// Like the SRS-2: five scales of eight items, a total with severity bands and a `≥` rule.
/// Item 40 stays unanswered.
fn srs(name: &str) -> String {
    let scales = [
        ("A", "Soziale Wahrnehmung", 4),
        ("B", "Soziale Kognition", 12),
        ("C", "Soziale Kommunikation", 16),
        ("D", "Soziale Motivation", 20),
        ("E", "Restriktive Interessen und repetitives Verhalten", 18),
    ];
    let mut yaml = String::from(
        "schema: 1\nid: srs\nversion: \"1\"\ntitle: Skala zur Erfassung sozialer Reaktivität\n\
         cutoff_compare: \">=\"\n\
         answer_sets: { haeufigkeit: [nie, manchmal, oft, immer] }\n\
         keys: { k: [0, 1, 2, 3] }\ndefault_answers: haeufigkeit\n\
         fields: [{ id: subject_id, label: Kennung, type: text }]\nscales:\n",
    );
    for (id, label, _) in scales {
        yaml += &format!("  - {{ id: {id}, label: \"{label}\" }}\n");
    }
    yaml += "derived:\n  - { id: gesamt, label: Gesamtrohwert, value: A + B + C + D + E, cutoff: 70 }\n\
             rules:\n  - { id: ass, label: \"ASS-Screening: Gesamtrohwert ≥ 70\", when: gesamt >= 70 }\n\
             verdict:\n  - { when: ass, text: \"Hinweis auf eine Autismus-Spektrum-Störung\" }\n  - { else: \"Unauffällig\" }\n\
             sections:\n  - title: Fragen\n    items:\n";
    let mut answers = Vec::new();
    for (s, (id, _, raw)) in scales.iter().enumerate() {
        for i in 0..8 {
            let n = s * 8 + i + 1;
            yaml += &format!("      - {{ n: {n}, scale: {id}, key: k, text: \"Frage\" }}\n");
            if n < 40 {
                answers.push((n.to_string(), (raw - 3 * i as i64).clamp(0, 3) as usize));
            }
        }
    }
    yaml += "result: [gesamt, A, B, C, D, E]\nnorms:\n  name: T\n  bands:\n\
             \x20   - { from: 0, label: unauffällig }\n    - { from: 60, label: mild }\n\
             \x20   - { from: 66, label: moderat }\n    - { from: 76, label: schwer }\n  tables:\n";
    for (id, _, _) in scales {
        let t: Vec<String> = (0..=24).map(|r| format!("{r}: {}", 38 + 2 * r)).collect();
        yaml += &format!("    {id}: {{ {} }}\n", t.join(", "));
    }
    let t: Vec<String> = (0..=24)
        .map(|r| format!("{}: {}", 5 * r, 38 + 2 * r))
        .collect();
    yaml += &format!("    gesamt: {{ {} }}\n", t.join(", "));
    render(name, &yaml, answers)
}

/// Like the MBAS: cut-offs, no norms. Two scales reach their cut-off exactly, one stays under it,
/// one has none.
fn cutoffs(name: &str, compare: &str) -> String {
    let scales = [
        ("A", "Unzufriedenheit mit der Figur", Some(4), 4),
        ("B", "Unzufriedenheit mit dem Gewicht", Some(6), 6),
        ("C", "Muskelbezogene Sorgen", Some(5), 2),
        ("D", "Körperbezogene Gedanken", None, 5),
    ];
    let mut yaml = format!(
        "schema: 1\nid: koerperbild\nversion: \"1\"\ntitle: Körperbild-Fragebogen (fiktiv)\n\
         cutoff_compare: \"{compare}\"\n\
         answer_sets: {{ stufen: [nie, selten, oft, immer] }}\n\
         keys: {{ k: [0, 1, 2, 3] }}\ndefault_answers: stufen\n\
         fields: [{{ id: subject_id, label: Kennung, type: text }}]\nscales:\n"
    );
    for (id, label, cutoff, _) in scales {
        let cutoff = cutoff.map_or(String::new(), |c| format!(", cutoff: {c}"));
        yaml += &format!("  - {{ id: {id}, label: \"{label}\"{cutoff} }}\n");
    }
    yaml += "sections:\n  - title: Fragen\n    items:\n";
    let mut answers = Vec::new();
    for (s, (id, _, _, raw)) in scales.iter().enumerate() {
        for i in 0..3 {
            let n = s * 3 + i + 1;
            yaml += &format!("      - {{ n: {n}, scale: {id}, key: k, text: \"Frage\" }}\n");
            answers.push((n.to_string(), (raw - 3 * i as i64).clamp(0, 3) as usize));
        }
    }
    yaml += "result: [A, B, C, D]\n";
    render(name, &yaml, answers)
}

fn render(name: &str, yaml: &str, answers: Vec<(String, usize)>) -> String {
    render_with(name, yaml, answers, serde_json::Value::Null)
}

fn render_with(
    name: &str,
    yaml: &str,
    answers: Vec<(String, usize)>,
    colors: serde_json::Value,
) -> String {
    let q = kreuzchen_engine::load(yaml).unwrap();
    let input = Input {
        fields: BTreeMap::from([(
            "subject_id".to_string(),
            FieldValue::Text("Muster 0815".into()),
        )]),
        answers: answers.into_iter().collect(),
    };
    // Through JSON as from the frontend, where norm table keys are strings.
    let report: Report = serde_json::from_value(serde_json::json!({
        "title": q.def.title,
        "date": "17.11.2026 14:30",
        "grading": q.grade(&input).unwrap(),
        "norms": q.def.norms,
        "colors": colors,
    }))
    .unwrap();
    let bytes = pdf(&report);
    fs::write(
        format!("{}/{name}.pdf", env!("CARGO_TARGET_TMPDIR")),
        &bytes,
    )
    .unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    // Content streams are uncompressed, so shown strings are plain in the file.
    String::from_utf8_lossy(&bytes).into_owned()
}

/// How often `pdf` shows exactly the string `s`, as a whole shown string.
fn count(pdf: &str, s: &str) -> usize {
    let hex: String = s.chars().map(|c| format!("{:02X}", c as u32)).collect();
    pdf.matches(&format!("({s}) Tj")).count() + pdf.matches(&format!("<{hex}> Tj")).count()
}

/// Whether `pdf` shows `s`: as is, or hex-encoded when the string has non-ASCII bytes.
fn shows(pdf: &str, s: &str) -> bool {
    let hex: String = s.chars().map(|c| format!("{:02X}", c as u32)).collect();
    pdf.contains(s) || pdf.contains(&hex)
}

#[test]
fn normed_definition_gets_profile_chart() {
    let pdf = example(
        "beispiel_normen",
        &[("1", 3), ("2", 0), ("3", 2), ("4", 1), ("5", 1)],
    );
    assert!(shows(&pdf, "(Profil (T-Werte))"));
    // T axis from the lowest to the highest table value, middle range labelled above it.
    assert!(pdf.contains("(25) Tj") && pdf.contains("(80) Tj"));
    for label in ["(-s) Tj", "(m) Tj", "(+s) Tj"] {
        assert!(pdf.contains(label), "{label}");
    }
    assert!(pdf.contains("(Muster 0815)"));
    assert!(pdf.contains("/BaseFont /Helvetica-Bold"));
}

#[test]
fn definition_without_norms_gets_table_only() {
    let pdf = example(
        "beispiel",
        &[("1", 3), ("2", 0), ("3", 3), ("4", 2), ("zusatz", 1)],
    );
    assert!(!pdf.contains("Profil"));
    assert!(pdf.contains("(Ergebnisse)"));
}

#[test]
fn stanine_profile_marks_the_middle_range() {
    let pdf = stanine("stanine", 14);
    assert!(pdf.contains("(Profil (Stanine-Werte))"));
    assert!(pdf.contains("(+s) Tj"));
    assert!(pdf.contains("(Seite 1 von 1)"));
}

#[test]
fn long_profile_continues_on_the_next_page() {
    let pdf = stanine("stanine_lang", 45);
    assert!(pdf.contains("(Seite 2 von "));
}

#[test]
fn srs_report_shows_signs_degrees_and_the_missing_answer() {
    let pdf = srs("srs");
    assert!(pdf.contains("(ERGEBNIS)"));
    // „≥“ is drawn from the embedded signs font, not replaced by „?“.
    assert!(pdf.contains("/BaseFont /KRZCHN+DejaVuSans"));
    assert!(
        pdf.contains("/F3 10 Tf\n(\\000\\005) Tj"),
        "≥ in the signs font"
    );
    assert!(!pdf.contains("Gesamtrohwert ?"));
    // The profile column wraps „milde Ausprägung“ onto two lines; the mean's band stays as is.
    for band in ["(milde) Tj", "(moderate) Tj", "(schwere) Tj"] {
        assert!(pdf.contains(band), "{band}");
    }
    assert!(shows(&pdf, "Ausprägung") && shows(&pdf, "unauffällig"));
    assert!(!shows(&pdf, "unauffällige"));
    assert!(pdf.contains("(RW 12)"));
    assert!(shows(&pdf, "Fehlende Antworten: 1 (Fragen 40)"));
}

#[test]
fn no_missing_answers_line_when_all_are_answered() {
    let pdf = example(
        "beispiel_normen",
        &[("1", 3), ("2", 0), ("3", 2), ("4", 1), ("5", 1), ("6", 0)],
    );
    assert!(!pdf.contains("Fehlende Antworten"));
}

/// `beispiel_normen` with scale A required complete; its caveats warn about missing answers.
fn validity(name: &str, answers: &[(&str, usize)]) -> String {
    let yaml = fs::read_to_string("../examples/definitions/beispiel_normen.yaml")
        .unwrap()
        .replacen("min_app: \"0.5.0\"", "min_app: \"0.7.0\"", 1)
        .replacen(
            "verdict:\n",
            "validity:\n  - { require: A.missing == 0, invalid: \"Skala A unvollständig.\" }\nverdict:\n",
            1,
        );
    let answers = answers.iter().map(|&(k, v)| (k.to_string(), v)).collect();
    render(name, &yaml, answers)
}

#[test]
fn warnings_come_before_the_verdict() {
    // Items 5 and 6 (scale B) missing: still valid, but the „more than 1 missing“ caveat fires.
    let pdf = validity("warnung", &[("1", 3), ("2", 0), ("3", 2), ("4", 1)]);
    let warn = pdf.find("(WARNUNGEN)").expect("warnings");
    let verdict = pdf.find("(ERGEBNIS)").expect("verdict");
    assert!(warn < verdict);
    assert!(!pdf.contains("(NICHT AUSWERTBAR)"));
    assert!(shows(&pdf, "Mehr als 1 fehlende Antwort."));
    assert!(pdf.contains("(Profil (T-Werte))"));
}

#[test]
fn invalid_result_names_why_and_has_no_verdict_or_profile() {
    let pdf = validity(
        "ungueltig",
        &[("1", 3), ("3", 2), ("4", 1), ("5", 1), ("6", 0)],
    );
    assert!(pdf.contains("(NICHT AUSWERTBAR)"));
    assert!(shows(&pdf, "Skala A unvollständig."));
    assert!(pdf.contains("(Die Werte unten sind nicht interpretierbar.)"));
    assert!(pdf.find("(WARNUNGEN)").unwrap() < pdf.find("(NICHT AUSWERTBAR)").unwrap());
    assert!(!pdf.contains("(ERGEBNIS)"));
    assert!(!pdf.contains("Profil"));
    // gesamt is 10, at its cut-off 10, but nothing is marked as over it.
    assert!(pdf.contains("(10) Tj"));
    assert!(!shows(&pdf, "über Cut-off"));
    assert!(pdf.contains("(Ergebnisse)"));
}

#[test]
fn cutoff_scales_without_norms_read_auffaellig_or_unauffaellig() {
    // Two scores equal their cut-off: „auffällig“ with `>=`, not with `>`.
    for (name, compare, auffaellig, unauffaellig) in [
        ("koerperbild_ge", ">=", 2, 1),
        ("koerperbild_gt", ">", 0, 3),
    ] {
        let pdf = cutoffs(name, compare);
        assert_eq!(count(&pdf, "auffällig"), auffaellig, "{compare}");
        assert_eq!(count(&pdf, "unauffällig"), unauffaellig, "{compare}");
        // Three of the four scales have a cut-off; the wording for normed scales stays out.
        assert!(!shows(&pdf, "über Cut-off"));
    }
    // An invalid result has no Einordnung at all.
    let yaml = fs::read_to_string("../examples/definitions/beispiel.yaml")
        .unwrap()
        .replacen("min_app: \"0.5.0\"", "min_app: \"0.7.0\"", 1)
        .replacen(
            "verdict:\n",
            "validity:\n  - { require: missing == 0, invalid: \"Unvollständig.\" }\nverdict:\n",
            1,
        );
    let answers = [("1", 3), ("2", 0), ("3", 3)].map(|(k, v)| (k.to_string(), v));
    let pdf = render("beispiel_ungueltig", &yaml, answers.to_vec());
    assert!(pdf.contains("(NICHT AUSWERTBAR)"));
    assert_eq!(count(&pdf, "auffällig") + count(&pdf, "unauffällig"), 0);
}

#[test]
fn chosen_colours_reach_the_pdf_and_bad_ones_fall_back() {
    let yaml = fs::read_to_string("../examples/definitions/beispiel_normen.yaml").unwrap();
    let answers = || {
        [("1", 3), ("2", 0), ("3", 2), ("4", 1), ("5", 1)]
            .map(|(k, v)| (k.to_string(), v))
            .to_vec()
    };
    let default = render("farben_standard", &yaml, answers());
    let custom = render_with(
        "farben",
        &yaml,
        answers(),
        serde_json::json!({ "accent": "#00FF00", "dot": "#ff00ff", "line": "#0000ff", "row": "#ffff00", "over": 7 }),
    );
    for rgb in ["0 1 0 rg", "1 0 1 rg", "0 0 1 RG", "1 1 0 rg"] {
        assert!(custom.contains(rgb), "{rgb}");
        assert!(!default.contains(rgb), "{rgb}");
    }
    // Malformed values keep the default colours.
    let bad = render_with(
        "farben_falsch",
        &yaml,
        answers(),
        serde_json::json!({ "accent": "lime", "dot": "#ff00f", "line": "#0000fg", "row": null }),
    );
    assert_eq!(bad, default);
}

//! Renders the example definitions to PDF under `target/tmp/` for a look, and checks their content.
use kreuzchen_engine::{Input, schema::FieldValue};
use kreuzchen_lib::report::{Report, pdf};
use std::{collections::BTreeMap, fs};

fn example(name: &str, answers: &[(&str, usize)]) -> String {
    let yaml = fs::read_to_string(format!("../examples/definitionen/{name}.yaml")).unwrap();
    let answers: Vec<_> = answers.iter().map(|&(k, v)| (k.to_string(), v)).collect();
    render(name, &yaml, answers)
}

/// `n` one-item scales with stanine tables and no bands, answered in a zigzag.
fn stanine(name: &str, n: usize) -> String {
    let mut yaml = String::from(
        "schema: 1\nid: stanine\nversion: \"1\"\ntitle: Profilbogen mit Stanine-Werten\n\
         answer_sets: { stufen: [\"1\", \"2\", \"3\", \"4\", \"5\", \"6\", \"7\", \"8\", \"9\"] }\n\
         keys: { k: [0, 1, 2, 3, 4, 5, 6, 7, 8] }\ndefault_answers: stufen\n\
         fields: [{ id: kennung, label: Kennung, type: text }]\nscales:\n",
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

fn render(name: &str, yaml: &str, answers: Vec<(String, usize)>) -> String {
    let q = kreuzchen_engine::load(yaml).unwrap();
    let input = Input {
        fields: BTreeMap::from([(
            "kennung".to_string(),
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

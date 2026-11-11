use crate::{Grading, Input, Questionnaire, ResultRow};
use std::fmt::Write;

impl Questionnaire {
    /// Plain-text result for copying; layout is documented by `tests/text.rs`.
    pub fn plain_text(&self, _input: &Input, g: &Grading) -> String {
        let d = &self.def;
        let mut lines = vec![format!("{} (Version {})", d.title, d.version)];
        lines.extend(g.fields.iter().map(|f| format!("{}: {}", f.label, f.value)));
        lines.push(String::new());
        for row in &g.rows {
            lines.push(match row {
                ResultRow::Sum {
                    label,
                    value,
                    cutoff,
                    over,
                    ..
                } => {
                    let mut s = format!("{label}: {value}");
                    if let Some(c) = cutoff {
                        write!(s, " (Cut-off {c})").unwrap();
                        if *over {
                            s.push_str(" – über Cut-off");
                        }
                    }
                    s
                }
                ResultRow::Answer { label, answer, .. } => {
                    format!("{label}: {}", answer.as_deref().unwrap_or("–"))
                }
            });
        }
        lines.push(String::new());
        for r in &g.rules {
            let met = if r.met { "erfüllt" } else { "nicht erfüllt" };
            lines.push(format!("{}: {met}", r.label));
        }
        let mut missing = format!("Fehlende Antworten: {}", g.missing);
        if g.missing > 0 {
            write!(missing, " (Fragen {})", g.missing_items.join(", ")).unwrap();
        }
        lines.push(missing);
        lines.extend(g.caveats.iter().map(|c| format!("Hinweis: {c}")));
        lines.extend(g.verdict.iter().map(|v| format!("Ergebnis: {v}")));
        lines.join("\n")
    }
}

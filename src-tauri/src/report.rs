//! Printable A4 report of a graded questionnaire: header, a profile chart for normed scales and a
//! result table. In the app's light-theme colours unless the settings choose others, in the
//! standard PDF fonts (nothing embedded).
use kreuzchen_engine::schema::Norms;
use kreuzchen_engine::{Grading, NormValue, ResultRow, number};
use pdf_writer::{Content, Name};
use serde::Deserialize;
use serde_json::Value;
use typeset::*;

mod typeset;

#[derive(Deserialize)]
pub struct Report {
    pub title: String,
    /// Date of the evaluation as shown, e.g. `17.11.2026 23:48`.
    pub date: String,
    pub grading: Grading,
    /// Norm tables of the definition, when it is at hand; they span the profile axis.
    pub norms: Option<Norms>,
    #[serde(default)]
    pub colors: Colors,
}

/// The colours the settings can change, as `0xrrggbb`.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(from = "Value")]
pub struct Colors {
    /// Middle range (mean ± 1 SD) and its labels.
    pub accent: u32,
    /// Profile markers.
    pub dot: u32,
    /// Line joining the markers.
    pub line: u32,
    /// Classification of a score over its cut-off.
    pub over: u32,
    /// Background of every other profile row.
    pub row: u32,
}

impl Default for Colors {
    fn default() -> Self {
        Colors {
            accent: ACCENT,
            dot: DANGER,
            line: MUTED,
            over: OVER,
            row: ROW,
        }
    }
}

/// From the frontend: `#rrggbb` per key; a missing or malformed value keeps its default.
impl From<Value> for Colors {
    fn from(v: Value) -> Self {
        let d = Colors::default();
        let pick = |key: &str, default: u32| {
            v.get(key)
                .and_then(Value::as_str)
                .and_then(hex)
                .unwrap_or(default)
        };
        Colors {
            accent: pick("accent", d.accent),
            dot: pick("dot", d.dot),
            line: pick("line", d.line),
            over: pick("over", d.over),
            row: pick("row", d.row),
        }
    }
}

fn hex(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    let ok = h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit());
    ok.then(|| u32::from_str_radix(h, 16).expect("six hex digits"))
}

pub fn pdf(r: &Report) -> Vec<u8> {
    let g = &r.grading;
    let mut d = Doc {
        pages: vec![page()],
        y: M,
    };
    header(&mut d, r);
    verdict(&mut d, g);
    // An invalid result gets no profile: plotted norm values would read as a finding.
    let profile = Profile::new(r).filter(|_| g.invalid.is_empty());
    let rest: Vec<&ResultRow> = g
        .rows
        .iter()
        .filter(|row| {
            profile
                .as_ref()
                .is_none_or(|p| !p.rows.iter().any(|m| m.id == row_id(row)))
        })
        .collect();
    if let Some(p) = &profile {
        p.draw(&mut d);
    }
    if !rest.is_empty() {
        // Without norms, every scale with a cut-off reads „auffällig“ or „unauffällig“. An invalid
        // result keeps the norm wording, which the engine has emptied: „unauffällig“ is a finding too.
        let normed = !g.invalid.is_empty()
            || r.norms.is_some()
            || g.rows
                .iter()
                .any(|row| matches!(row, ResultRow::Sum { norm: Some(_), .. }));
        table(
            &mut d,
            &rest,
            normed,
            typical(r.norms.as_ref()),
            r.colors.over,
        );
    }

    let footer = match g.fields.iter().find(|f| f.id == "subject_id") {
        Some(k) => format!("{} · {}", r.title, k.value),
        None => r.title.clone(),
    };
    finish(d.pages, &r.title, &footer)
}

/// Title, then the date and session fields as a label/value grid.
fn header(d: &mut Doc, r: &Report) {
    let g = &r.grading;
    for line in wrap(&r.title, Font::Bold, 18.0, W - 2.0 * M) {
        d.y += 18.0;
        d.text(M, d.y, Font::Bold, 18.0, &line);
        d.y += 4.0;
    }
    d.y += 8.0;
    let mut meta = vec![("Datum der Auswertung", r.date.as_str())];
    meta.extend(
        g.fields
            .iter()
            .map(|f| (f.label.as_str(), f.value.as_str())),
    );
    if let Some(k) = g.fields.iter().position(|f| f.id == "subject_id") {
        meta.swap(0, k + 1);
    }
    let col = (W - 2.0 * M) / 3.0;
    for row in meta.chunks(3) {
        for (i, (label, value)) in row.iter().enumerate() {
            let x = M + i as f32 * col;
            d.ink(MUTED);
            d.text(
                x,
                d.y + 9.0,
                Font::Regular,
                8.0,
                &fit(label, Font::Regular, 8.0, col - 10.0),
            );
            d.ink(FG);
            d.text(
                x,
                d.y + 22.0,
                Font::Bold,
                11.0,
                &fit(value, Font::Bold, 11.0, col - 10.0),
            );
        }
        d.y += 30.0;
    }
    d.y += 4.0;
    d.line(M, d.y, W - M, d.y, 0.8, FG);
    d.y += 8.0;
}

/// Warnings, then the verdict or why there is none, rules and missing answers.
fn verdict(d: &mut Doc, g: &Grading) {
    if !g.caveats.is_empty() {
        label(d, OVER, "WARNUNGEN");
        bullets(d, Font::Regular, FG, &g.caveats);
        d.y += 8.0;
    }
    if !g.invalid.is_empty() {
        label(d, DANGER, "NICHT AUSWERTBAR");
        bullets(d, Font::Bold, DANGER, &g.invalid);
        let note = "Die Werte unten sind nicht interpretierbar.";
        d.room(15.0);
        d.text(M, d.y + 11.0, Font::Regular, 10.0, note);
        d.y += 21.0;
    } else if let Some(v) = &g.verdict {
        d.ink(MUTED);
        d.text(M, d.y + 10.0, Font::Regular, 8.0, "ERGEBNIS");
        d.ink(FG);
        d.text(
            M,
            d.y + 27.0,
            Font::Bold,
            16.0,
            &fit(v, Font::Bold, 16.0, W - 2.0 * M),
        );
        d.y += 34.0;
    }
    for rule in &g.rules {
        d.room(15.0);
        let f = if rule.met && g.invalid.is_empty() {
            Font::Bold
        } else {
            Font::Regular
        };
        let state = if rule.met {
            "erfüllt"
        } else {
            "nicht erfüllt"
        };
        d.text(M, d.y + 11.0, f, 10.0, &format!("{}: {state}", rule.label));
        d.y += 15.0;
    }
    if g.missing == 0 {
        return;
    }
    let missing = format!(
        "Fehlende Antworten: {} (Fragen {})",
        g.missing,
        g.missing_items.join(", ")
    );
    for l in wrap(&missing, Font::Regular, 10.0, W - 2.0 * M) {
        d.room(14.0);
        d.text(M, d.y + 11.0, Font::Regular, 10.0, &l);
        d.y += 14.0;
    }
}

/// A small caps label like „ERGEBNIS“, in `color`.
fn label(d: &mut Doc, color: u32, s: &str) {
    d.room(30.0);
    d.ink(color);
    d.text(M, d.y + 10.0, Font::Bold, 8.0, s);
    d.ink(FG);
    d.y += 14.0;
}

/// Wrapped bullet points in `color`.
fn bullets(d: &mut Doc, font: Font, color: u32, items: &[String]) {
    for c in items {
        for (i, l) in wrap(c, font, 10.0, W - 2.0 * M - 12.0).iter().enumerate() {
            d.room(14.0);
            d.ink(color);
            if i == 0 {
                d.text(M, d.y + 11.0, font, 10.0, "•");
            }
            d.text(M + 12.0, d.y + 11.0, font, 10.0, l);
            d.ink(FG);
            d.y += 14.0;
        }
    }
}

fn heading(d: &mut Doc, s: &str) {
    d.y += 16.0;
    d.room(60.0);
    d.text(M, d.y + 12.0, Font::Bold, 12.0, s);
    d.y += 20.0;
}

/// The normed result rows as a profile sheet: one tall row per scale with its name and values
/// beside a shared norm axis, a red marker at each norm value and the markers joined top to bottom.
struct Profile<'a> {
    name: String,
    rows: Vec<Mark<'a>>,
    axis: Axis,
    middle: Option<[i64; 3]>,
    /// Label of the band that holds the scale mean; see [`class`].
    typical: Option<&'a str>,
    /// Whether any row has a band or is over its cut-off, i.e. needs the „Einordnung“ column.
    classified: bool,
    colors: Colors,
}

/// A profile row; `norm` is `None` when the raw score is below the table.
struct Mark<'a> {
    id: &'a str,
    label: &'a str,
    value: f64,
    over: bool,
    norm: Option<&'a NormValue>,
}

fn row_id(row: &ResultRow) -> &str {
    match row {
        ResultRow::Sum { id, .. } | ResultRow::Answer { id, .. } => id,
    }
}

// Profile columns: wrapped label, values (norm value over raw score), axis, classification.
const P_LABEL: f32 = 116.0;
const P_VALUE: f32 = 204.0;
const P_AXIS: f32 = 218.0;
const P_CLASS: f32 = 62.0;
const P_ROW: f32 = 26.0;
const P_HEAD: f32 = 26.0;
const P_LEADING: f32 = 10.5;

/// A profile row laid out: wrapped label and classification, and the row height.
struct Laid<'a> {
    mark: &'a Mark<'a>,
    label: Vec<String>,
    class: Vec<String>,
    h: f32,
}

impl<'a> Profile<'a> {
    fn new(r: &'a Report) -> Option<Self> {
        // With the tables, every scale that has one; without, the rows that carry a norm value.
        let rows: Vec<Mark> = r
            .grading
            .rows
            .iter()
            .filter_map(|row| match row {
                ResultRow::Sum {
                    id,
                    label,
                    value,
                    over,
                    norm,
                    ..
                } => {
                    let normed = r
                        .norms
                        .as_ref()
                        .map_or(norm.is_some(), |n| n.tables.contains_key(id));
                    normed.then_some(Mark {
                        id,
                        label,
                        value: *value,
                        over: *over,
                        norm: norm.as_ref(),
                    })
                }
                ResultRow::Answer { .. } => None,
            })
            .collect();
        let name = match &r.norms {
            Some(n) => n.name.clone(),
            None => rows.iter().find_map(|m| m.norm.map(|n| n.name.clone()))?,
        };
        let middle = middle(&name);
        let mut values: Vec<i64> = rows
            .iter()
            .filter_map(|m| m.norm.map(|n| n.value))
            .collect();
        match &r.norms {
            Some(n) => {
                let tables = rows.iter().filter_map(|m| n.tables.get(m.id));
                values.extend(tables.flat_map(|t| t.values().copied()));
            }
            // Saved result whose definition is gone or changed: the values and the middle range.
            None => values.extend(middle.iter().flatten().copied()),
        }
        let axis = axis(values)?;
        let typical = typical(r.norms.as_ref());
        let classified = rows
            .iter()
            .any(|m| !class(m.norm, m.over, typical).is_empty());
        Some(Profile {
            name,
            rows,
            axis,
            middle,
            typical,
            classified,
            colors: r.colors,
        })
    }

    fn axis_end(&self) -> f32 {
        if self.classified {
            W - M - P_CLASS - 14.0
        } else {
            W - M
        }
    }

    /// Half the width of one norm value's cell.
    fn half(&self) -> f32 {
        (self.axis_end() - P_AXIS) / (self.axis.hi - self.axis.lo + 1) as f32 / 2.0
    }

    fn x(&self, v: i64) -> f32 {
        P_AXIS + (v - self.axis.lo) as f32 * 2.0 * self.half() + self.half()
    }

    fn draw(&self, d: &mut Doc) {
        heading(d, &format!("Profil ({}-Werte)", self.name));
        let laid: Vec<Laid> = self
            .rows
            .iter()
            .map(|mark| {
                let label = wrap(mark.label, Font::Regular, 9.0, P_LABEL);
                let class = match self.classified {
                    true => wrap(
                        &class(mark.norm, mark.over, self.typical),
                        Font::Regular,
                        8.0,
                        P_CLASS,
                    ),
                    false => vec![],
                };
                let h = P_ROW.max(label.len().max(class.len()) as f32 * P_LEADING + 9.0);
                Laid {
                    mark,
                    label,
                    class,
                    h,
                }
            })
            .collect();
        let mut rest = &laid[..];
        while !rest.is_empty() {
            d.room(P_HEAD + rest[0].h);
            let top = d.y + P_HEAD;
            let (mut n, mut bottom) = (0, top);
            while n < rest.len() && (n == 0 || bottom + rest[n].h <= BOTTOM) {
                bottom += rest[n].h;
                n += 1;
            }
            let (part, next) = rest.split_at(n);
            rest = next;
            self.segment(d.c(), part, top, bottom);
            d.y = bottom;
            if !rest.is_empty() {
                d.new_page();
            }
        }
    }

    /// Rows `top`..`bottom` of one page, with the column titles and axis labels above them.
    fn segment(&self, c: &mut Content, rows: &[Laid], top: f32, bottom: f32) {
        let half = self.half();
        let ticks = (self.axis.lo..=self.axis.hi).step_by(self.axis.step as usize);

        // Alternating row backgrounds under everything else.
        let mut y = top;
        for (i, row) in rows.iter().enumerate() {
            if i % 2 == 1 {
                ink(c, self.colors.row);
                c.rect(M, H - y - row.h, W - 2.0 * M, row.h).fill_nonzero();
            }
            y += row.h;
        }

        // Middle range, translucent so the stripes show through, labelled above the axis.
        if let Some(mid) = self.middle {
            let lo = mid[0].max(self.axis.lo);
            let hi = mid[2].min(self.axis.hi);
            if lo <= hi {
                c.save_state().set_parameters(Name(b"Shade"));
                ink(c, self.colors.accent);
                c.rect(
                    self.x(lo) - half,
                    H - bottom,
                    self.x(hi) - self.x(lo) + 2.0 * half,
                    bottom - top,
                )
                .fill_nonzero();
                c.restore_state();
            }
            ink(c, self.colors.accent);
            for (v, s) in mid.iter().zip(["-s", "m", "+s"]) {
                if (self.axis.lo..=self.axis.hi).contains(v) {
                    let x = self.x(*v) - width(s, Font::Bold, 8.0) / 2.0;
                    text(c, x, top - 15.0, Font::Bold, 8.0, s);
                }
            }
            if (self.axis.lo..=self.axis.hi).contains(&mid[1]) {
                line(
                    c,
                    self.x(mid[1]),
                    top,
                    self.x(mid[1]),
                    bottom,
                    0.8,
                    self.colors.accent,
                );
            }
        }

        // Grid, frame, titles.
        for v in ticks.clone() {
            if Some(v) != self.middle.map(|m| m[1]) {
                line(c, self.x(v), top, self.x(v), bottom, 0.5, LINE);
            }
        }
        ink(c, MUTED);
        for v in ticks {
            let s = v.to_string();
            text(
                c,
                self.x(v) - width(&s, Font::Regular, 7.5) / 2.0,
                top - 4.0,
                Font::Regular,
                7.5,
                &s,
            );
        }
        text(c, M, top - 4.0, Font::Bold, 8.0, "Skala");
        right(
            c,
            P_VALUE,
            top - 4.0,
            Font::Bold,
            8.0,
            &fit(&self.name, Font::Bold, 8.0, P_VALUE - M - P_LABEL - 4.0),
        );
        if self.classified {
            text(c, W - M - P_CLASS, top - 4.0, Font::Bold, 8.0, "Einordnung");
        }
        line(c, M, top, W - M, top, 0.8, FG);
        line(c, M, bottom, W - M, bottom, 0.8, FG);

        // Row content; each block is centred vertically in its row.
        let mut points = Vec::new();
        let mut y = top;
        for row in rows {
            let mid = y + row.h / 2.0;
            let first =
                |lines: usize, size: f32| mid - lines as f32 * P_LEADING / 2.0 + size * 0.8 + 0.5;
            ink(c, FG);
            for (j, l) in row.label.iter().enumerate() {
                text(
                    c,
                    M,
                    first(row.label.len(), 9.0) + j as f32 * P_LEADING,
                    Font::Regular,
                    9.0,
                    l,
                );
            }
            let m = row.mark;
            match m.norm {
                Some(n) => {
                    right(c, P_VALUE, mid, Font::Bold, 10.5, &n.value.to_string());
                    points.push(Some((
                        self.x(n.value.clamp(self.axis.lo, self.axis.hi)),
                        mid,
                    )));
                }
                None => {
                    right(c, P_VALUE, mid, Font::Regular, 10.5, "–");
                    points.push(None);
                }
            }
            ink(c, MUTED);
            right(
                c,
                P_VALUE,
                mid + 9.5,
                Font::Regular,
                7.5,
                &format!("RW {}", number(m.value)),
            );
            let (font, color) = if m.over {
                (Font::Bold, self.colors.over)
            } else {
                (Font::Regular, FG)
            };
            ink(c, color);
            for (j, l) in row.class.iter().enumerate() {
                text(
                    c,
                    W - M - P_CLASS,
                    first(row.class.len(), 8.0) + j as f32 * P_LEADING,
                    font,
                    8.0,
                    l,
                );
            }
            y += row.h;
        }

        // The profile line, then the markers on top.
        stroke(c, self.colors.line);
        c.set_line_width(1.2);
        for pair in points.windows(2) {
            if let [Some(a), Some(b)] = pair {
                c.move_to(a.0, H - a.1).line_to(b.0, H - b.1).stroke();
            }
        }
        ink(c, self.colors.dot);
        stroke(c, WHITE);
        c.set_line_width(0.9);
        for &(x, y) in points.iter().flatten() {
            dot(c, x, H - y, 3.8);
        }
        ink(c, FG);
    }
}

// Table columns: label width, then right edges of raw score, cut-off, norm value; classification.
const T_LABEL: f32 = 250.0;
const T_RAW: f32 = 335.0;
const T_CUT: f32 = 385.0;
const T_NORM: f32 = 440.0;
const T_CLASS: f32 = 452.0;

/// `normed`: whether the definition has norms, which decides the wording of „Einordnung“.
fn table(d: &mut Doc, rows: &[&ResultRow], normed: bool, typical: Option<&str>, over_color: u32) {
    heading(d, "Ergebnisse");
    let norm_column = rows
        .iter()
        .any(|r| matches!(r, ResultRow::Sum { norm: Some(_), .. }));
    let header = |d: &mut Doc| {
        let y = d.y + 11.0;
        let c = d.c();
        ink(c, MUTED);
        text(c, M, y, Font::Bold, 8.0, "Skala");
        right(c, T_RAW, y, Font::Bold, 8.0, "Rohwert");
        right(c, T_CUT, y, Font::Bold, 8.0, "Cut-off");
        if norm_column {
            right(c, T_NORM, y, Font::Bold, 8.0, "Normwert");
        }
        text(c, T_CLASS, y, Font::Bold, 8.0, "Einordnung");
        ink(c, FG);
        d.y += 16.0;
        d.line(M, d.y, W - M, d.y, 0.8, FG);
    };
    header(d);
    for (i, row) in rows.iter().enumerate() {
        let label = match row {
            ResultRow::Sum { label, .. } | ResultRow::Answer { label, .. } => label,
        };
        let label = wrap(label, Font::Regular, 10.0, T_LABEL - 5.0);
        let answer = match row {
            ResultRow::Answer { answer, .. } => wrap(
                answer.as_deref().unwrap_or("–"),
                Font::Bold,
                10.0,
                W - 2.0 * M - T_LABEL,
            ),
            ResultRow::Sum { .. } => vec![],
        };
        let h = label.len().max(answer.len()).max(1) as f32 * 13.0 + 7.0;
        if d.y + h > BOTTOM {
            d.new_page();
            header(d);
        } else if i > 0 {
            d.line(M, d.y, W - M, d.y, 0.4, LINE);
        }
        let base = d.y + 13.5;
        let c = d.c();
        for (j, l) in label.iter().enumerate() {
            text(c, M, base + j as f32 * 13.0, Font::Regular, 10.0, l);
        }
        for (j, l) in answer.iter().enumerate() {
            text(c, M + T_LABEL, base + j as f32 * 13.0, Font::Bold, 10.0, l);
        }
        if let ResultRow::Sum {
            value,
            cutoff,
            over,
            norm,
            ..
        } = row
        {
            right(c, T_RAW, base, Font::Bold, 10.0, &number(*value));
            right(
                c,
                T_CUT,
                base,
                Font::Regular,
                10.0,
                &cutoff.map_or("–".into(), |c| c.to_string()),
            );
            if norm_column {
                right(
                    c,
                    T_NORM,
                    base,
                    Font::Regular,
                    10.0,
                    &norm
                        .as_ref()
                        .map_or("–".into(), |n| format!("{} {}", n.name, n.value)),
                );
            }
            let (font, color) = if *over {
                (Font::Bold, over_color)
            } else {
                (Font::Regular, FG)
            };
            let class = match normed {
                true => class(norm.as_ref(), *over, typical),
                false => cutoff_class(*cutoff, *over).into(),
            };
            ink(c, color);
            text(
                c,
                T_CLASS,
                base,
                font,
                10.0,
                &fit(&class, font, 10.0, W - M - T_CLASS),
            );
            ink(c, FG);
        }
        d.y += h;
    }
    d.line(M, d.y, W - M, d.y, 0.8, FG);
}

/// The „Einordnung“ column: the norm band and whether the score is over its cut-off. Bands
/// other than the `typical` one read as a degree of expression.
fn class(norm: Option<&NormValue>, over: bool, typical: Option<&str>) -> String {
    let mut parts: Vec<String> = norm
        .and_then(|n| n.band.as_deref())
        .map(|b| match typical {
            Some(t) if t != b => degree(b),
            _ => b.to_string(),
        })
        .into_iter()
        .collect();
    if over {
        parts.push("über Cut-off".into());
    }
    parts.join(", ")
}

/// The „Einordnung“ of a scale without norms: only scales with a cut-off have one.
fn cutoff_class(cutoff: Option<i64>, over: bool) -> &'static str {
    match (cutoff, over) {
        (None, _) => "",
        (Some(_), true) => "auffällig",
        (Some(_), false) => "unauffällig",
    }
}

/// Label of the norm band that holds the scale mean, when the bands and the mean are known.
fn typical(norms: Option<&Norms>) -> Option<&str> {
    let n = norms?;
    let [_, mean, _] = middle(&n.name)?;
    let band = n.bands.iter().rev().find(|b| mean >= b.from)?;
    Some(&band.label)
}

/// A band label as a degree of expression: „mild“ → „milde Ausprägung“.
fn degree(label: &str) -> String {
    let stem = match label.strip_suffix("hoch") {
        Some(head) => format!("{head}hoh"),
        None => label.to_string(),
    };
    let e = if stem.ends_with('e') { "" } else { "e" };
    format!("{stem}{e} Ausprägung")
}

/// Norm axis from `lo` to `hi` with a tick every `step`; both ends are multiples of `step`.
#[derive(Debug, PartialEq)]
struct Axis {
    lo: i64,
    hi: i64,
    step: i64,
}

/// The smallest 1-2-5 step that labels the value range with at most 12 ticks.
fn axis(values: impl IntoIterator<Item = i64>) -> Option<Axis> {
    let (lo, hi) = values.into_iter().fold(None, |acc, v| match acc {
        None => Some((v, v)),
        Some((lo, hi)) => Some((v.min(lo), v.max(hi))),
    })?;
    let floor = |v: i64, s: i64| v.div_euclid(s) * s;
    let ceil = |v: i64, s: i64| -floor(-v, s);
    let step = (0..)
        .flat_map(|e| [1, 2, 5].map(|m| m * 10i64.pow(e)))
        .find(|&s| (ceil(hi, s) - floor(lo, s)) / s < 12)
        .expect("a large enough step exists");
    Some(Axis {
        lo: floor(lo, step),
        hi: ceil(hi, step),
        step,
    })
}

/// Mean ± 1 SD as `[-s, m, +s]`, for the norm scales whose mean and SD are fixed.
fn middle(name: &str) -> Option<[i64; 3]> {
    let (mean, sd) = match name {
        "T" => (50, 10),
        "Stanine" => (5, 2),
        _ => return None,
    };
    Some([mean - sd, mean, mean + sd])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_covers_values_with_round_ends_and_few_ticks() {
        assert_eq!(
            axis([1, 9]),
            Some(Axis {
                lo: 1,
                hi: 9,
                step: 1
            })
        );
        assert_eq!(
            axis([25, 80, 31]),
            Some(Axis {
                lo: 25,
                hi: 80,
                step: 5
            })
        );
        assert_eq!(
            axis([1, 99]),
            Some(Axis {
                lo: 0,
                hi: 100,
                step: 10
            })
        );
        assert_eq!(
            axis([55, 145]),
            Some(Axis {
                lo: 50,
                hi: 150,
                step: 10
            })
        );
        assert_eq!(
            axis([-3, 3]),
            Some(Axis {
                lo: -3,
                hi: 3,
                step: 1
            })
        );
        assert_eq!(
            axis([50]),
            Some(Axis {
                lo: 50,
                hi: 50,
                step: 1
            })
        );
        assert_eq!(axis([]), None);
    }

    #[test]
    fn bands_away_from_the_mean_read_as_a_degree_of_expression() {
        for (band, want) in [
            ("mild", "milde Ausprägung"),
            ("moderat", "moderate Ausprägung"),
            ("schwer", "schwere Ausprägung"),
            ("niedrig", "niedrige Ausprägung"),
            ("sehr hoch", "sehr hohe Ausprägung"),
            ("leichte", "leichte Ausprägung"),
        ] {
            assert_eq!(degree(band), want);
        }
        let norms: Norms = serde_json::from_value(serde_json::json!({
            "name": "T",
            "bands": [
                { "from": 0, "label": "unauffällig" },
                { "from": 60, "label": "mild" },
                { "from": 66, "label": "moderat" },
            ],
            "tables": {},
        }))
        .unwrap();
        let typical = typical(Some(&norms));
        assert_eq!(typical, Some("unauffällig"));
        let t = |band: &str| NormValue {
            name: "T".into(),
            value: 0,
            band: Some(band.into()),
        };
        assert_eq!(
            class(Some(&t("unauffällig")), false, typical),
            "unauffällig"
        );
        assert_eq!(
            class(Some(&t("moderat")), true, typical),
            "moderate Ausprägung, über Cut-off"
        );
        // Without the bands, the mean's band is unknown and labels stay as they are.
        assert_eq!(class(Some(&t("mild")), false, None), "mild");
    }

    #[test]
    fn middle_range_is_mean_plus_minus_one_sd() {
        assert_eq!(middle("T"), Some([40, 50, 60]));
        assert_eq!(middle("Stanine"), Some([3, 5, 7]));
        assert_eq!(middle("Prozentrang"), None);
    }
}

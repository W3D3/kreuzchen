//! Page geometry, theme colours and drawing primitives for the PDF report: text in the standard
//! Helvetica fonts measured with their metrics, lines, fills, and the final document assembly.
use pdf_writer::{Content, Name, Pdf, Rect, Ref, Str, TextStr};

// A4 in points; content runs from the top margin down to the footer.
pub(super) const W: f32 = 595.0;
pub(super) const H: f32 = 842.0;
pub(super) const M: f32 = 50.0;
pub(super) const BOTTOM: f32 = H - 64.0;

// Light-theme tokens from `src/app.css`.
pub(super) const FG: u32 = 0x1d2a2f; // --fg
pub(super) const MUTED: u32 = 0x5f6f75; // --muted
pub(super) const LINE: u32 = 0xd3dbde; // --line
pub(super) const ACCENT: u32 = 0x1f6f78; // --accent
pub(super) const ROW: u32 = 0xebeff0; // --color-muted: --line at 45 % on --surface
pub(super) const DANGER: u32 = 0xb42318; // --danger
pub(super) const OVER: u32 = 0xb4531a; // --over
pub(super) const WHITE: u32 = 0xffffff; // --surface
/// Fill opacity of the shaded middle range, as graphics state `Shade`.
pub(super) const SHADE_ALPHA: f32 = 0.16;

pub(super) struct Doc {
    pub(super) pages: Vec<Content>,
    /// Distance of the write position from the top of the page.
    pub(super) y: f32,
}

impl Doc {
    pub(super) fn c(&mut self) -> &mut Content {
        self.pages.last_mut().expect("starts with a page")
    }

    pub(super) fn new_page(&mut self) {
        self.pages.push(page());
        self.y = M;
    }

    pub(super) fn text(&mut self, x: f32, y: f32, font: Font, size: f32, s: &str) {
        text(self.c(), x, y, font, size, s);
    }

    pub(super) fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, w: f32, color: u32) {
        line(self.c(), x1, y1, x2, y2, w, color);
    }

    pub(super) fn ink(&mut self, color: u32) {
        ink(self.c(), color);
    }

    /// Starts a new page unless `h` more points fit on this one.
    pub(super) fn room(&mut self, h: f32) {
        if self.y + h > BOTTOM {
            self.new_page();
        }
    }
}

pub(super) fn finish(pages: Vec<Content>, title: &str, footer: &str) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let (catalog, tree, regular, bold, info, shade) = (
        Ref::new(1),
        Ref::new(2),
        Ref::new(3),
        Ref::new(4),
        Ref::new(5),
        Ref::new(6),
    );
    pdf.catalog(catalog).pages(tree);
    for (id, name) in [
        (regular, Name(b"Helvetica")),
        (bold, Name(b"Helvetica-Bold")),
    ] {
        pdf.type1_font(id)
            .base_font(name)
            .encoding_predefined(Name(b"WinAnsiEncoding"));
    }
    pdf.ext_graphics(shade).non_stroking_alpha(SHADE_ALPHA);
    pdf.document_info(info)
        .title(TextStr(title))
        .creator(TextStr("Kreuzchen"));
    let n = pages.len();
    let ids: Vec<(Ref, Ref)> = (0..n as i32)
        .map(|i| (Ref::new(7 + 2 * i), Ref::new(8 + 2 * i)))
        .collect();
    pdf.pages(tree)
        .kids(ids.iter().map(|p| p.0))
        .count(n as i32);
    for (i, (mut c, &(page_id, content_id))) in pages.into_iter().zip(&ids).enumerate() {
        let y = H - 34.0;
        line(&mut c, M, y - 11.0, W - M, y - 11.0, 0.4, LINE);
        ink(&mut c, MUTED);
        let page = format!("Seite {} von {n}", i + 1);
        let pw = width(&page, Font::Regular, 8.0);
        text(
            &mut c,
            M,
            y,
            Font::Regular,
            8.0,
            &fit(footer, Font::Regular, 8.0, W - 2.0 * M - pw - 20.0),
        );
        right(&mut c, W - M, y, Font::Regular, 8.0, &page);
        let mut p = pdf.page(page_id);
        p.media_box(Rect::new(0.0, 0.0, W, H))
            .parent(tree)
            .contents(content_id);
        // One Resources dict: a second call would replace the first.
        let mut res = p.resources();
        res.fonts()
            .pair(Name(b"F1"), regular)
            .pair(Name(b"F2"), bold);
        res.ext_g_states().pair(Name(b"Shade"), shade);
        drop(res);
        drop(p);
        pdf.stream(content_id, &c.finish());
    }
    pdf.finish()
}

#[derive(Clone, Copy)]
pub(super) enum Font {
    Regular,
    Bold,
}

impl Font {
    fn name(self) -> Name<'static> {
        match self {
            Font::Regular => Name(b"F1"),
            Font::Bold => Name(b"F2"),
        }
    }

    /// Advance widths per 1000 em of WinAnsi codes 32–255, from the Helvetica metrics.
    fn widths(self) -> &'static [u16; 224] {
        match self {
            Font::Regular => &HELVETICA,
            Font::Bold => &HELVETICA_BOLD,
        }
    }
}

/// Text at `x` with its baseline `y` below the top of the page.
pub(super) fn text(c: &mut Content, x: f32, y: f32, font: Font, size: f32, s: &str) {
    c.begin_text()
        .set_font(font.name(), size)
        .next_line(x, H - y)
        .show(Str(&winansi(s)))
        .end_text();
}

pub(super) fn right(c: &mut Content, x: f32, y: f32, font: Font, size: f32, s: &str) {
    text(c, x - width(s, font, size), y, font, size, s);
}

pub(super) fn line(c: &mut Content, x1: f32, y1: f32, x2: f32, y2: f32, w: f32, color: u32) {
    stroke(c, color);
    c.set_line_width(w);
    c.move_to(x1, H - y1).line_to(x2, H - y2).stroke();
}

fn rgb(color: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| ((color >> shift) & 0xff) as f32 / 255.0)
}

/// Fill colour, for text too.
pub(super) fn ink(c: &mut Content, color: u32) {
    let [r, g, b] = rgb(color);
    c.set_fill_rgb(r, g, b);
}

pub(super) fn stroke(c: &mut Content, color: u32) {
    let [r, g, b] = rgb(color);
    c.set_stroke_rgb(r, g, b);
}

/// A fresh page, writing in the text colour.
pub(super) fn page() -> Content {
    let mut c = Content::new();
    ink(&mut c, FG);
    c
}

/// Circle around (`x`, `y`) in PDF coordinates, filled and outlined, from four Bézier arcs.
pub(super) fn dot(c: &mut Content, x: f32, y: f32, r: f32) {
    let k = 0.5523 * r;
    c.move_to(x + r, y)
        .cubic_to(x + r, y + k, x + k, y + r, x, y + r)
        .cubic_to(x - k, y + r, x - r, y + k, x - r, y)
        .cubic_to(x - r, y - k, x - k, y - r, x, y - r)
        .cubic_to(x + k, y - r, x + r, y - k, x + r, y)
        .fill_nonzero_and_stroke();
}

/// Windows-1252 code points 0x80–0x9F; the rest of WinAnsi is Latin-1.
const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

/// `s` in WinAnsiEncoding, the encoding of the standard fonts; anything else becomes `?`.
fn winansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c as u32 {
            0x20..=0x7e | 0xa0..=0xff => c as u8,
            _ => CP1252
                .iter()
                .position(|&u| u == c)
                .map_or(b'?', |i| 0x80 + i as u8),
        })
        .collect()
}

pub(super) fn width(s: &str, font: Font, size: f32) -> f32 {
    let w: u32 = winansi(s)
        .iter()
        .map(|&b| font.widths()[b as usize - 32] as u32)
        .sum();
    w as f32 * size / 1000.0
}

/// `s`, cut with an ellipsis to at most `max` points.
pub(super) fn fit(s: &str, font: Font, size: f32, max: f32) -> String {
    if width(s, font, size) <= max {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() && width(&format!("{out}…"), font, size) > max {
        out.pop();
    }
    format!("{}…", out.trim_end())
}

/// `s` broken into lines of at most `max` points at spaces; overlong words are cut.
pub(super) fn wrap(s: &str, font: Font, size: f32, max: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match lines.last_mut() {
            Some(l) if width(&format!("{l} {word}"), font, size) <= max => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(fit(word, font, size, max)),
        }
    }
    lines
}

#[rustfmt::skip]
const HELVETICA: [u16; 224] = [278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, 350, 556, 350, 222, 556, 333, 1000, 556, 556, 333, 1000, 667, 333, 1000, 350, 611, 350, 350, 222, 221, 333, 333, 350, 556, 1000, 333, 1000, 500, 333, 944, 350, 500, 667, 278, 333, 556, 556, 556, 556, 260, 556, 333, 737, 370, 556, 584, 333, 737, 333, 606, 584, 351, 351, 333, 556, 537, 278, 333, 351, 365, 556, 869, 869, 869, 611, 667, 667, 667, 667, 667, 667, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278, 722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 666, 666, 611, 556, 556, 556, 556, 556, 556, 889, 500, 556, 556, 556, 556, 278, 278, 278, 278, 556, 556, 556, 556, 556, 556, 556, 584, 611, 556, 556, 556, 556, 500, 555, 500];
#[rustfmt::skip]
const HELVETICA_BOLD: [u16; 224] = [278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584, 350, 556, 350, 278, 556, 500, 1000, 556, 556, 333, 1000, 667, 333, 1000, 350, 611, 350, 350, 278, 278, 500, 500, 350, 556, 1000, 333, 1000, 556, 333, 944, 350, 500, 667, 278, 333, 556, 556, 556, 556, 280, 556, 333, 737, 370, 556, 584, 333, 737, 333, 606, 584, 351, 351, 333, 611, 556, 278, 333, 351, 365, 556, 869, 869, 869, 611, 722, 722, 722, 722, 722, 722, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278, 722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611, 556, 556, 556, 556, 556, 556, 889, 556, 556, 556, 556, 556, 278, 278, 278, 278, 611, 611, 611, 611, 611, 611, 611, 584, 611, 611, 611, 611, 611, 556, 611, 556];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_winansi_and_measured_by_helvetica_metrics() {
        assert_eq!(
            winansi("Größe – „ok“ €✓"),
            b"Gr\xf6\xdfe \x96 \x84ok\x93 \x80?"
        );
        assert_eq!(width("Wi", Font::Regular, 10.0), (944.0 + 222.0) / 100.0);
        assert!(
            width(
                &fit("Eine sehr lange Skala", Font::Regular, 10.0, 50.0),
                Font::Regular,
                10.0
            ) <= 50.0
        );
        assert_eq!(wrap("aa bb cc", Font::Regular, 10.0, 26.0), ["aa bb", "cc"]);
    }
}

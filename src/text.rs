//! Text shaping-lite: font loading (device system Roboto), measurement, word wrap
//! and cursor mapping. Pure CPU side; rasterization caching/GL atlas lives in gl.rs.

use fontdue::{Font, FontSettings, Metrics};

const FONT_CANDIDATES: &[&str] = &[
    "/system/fonts/Roboto-Regular.ttf",
    "/system/fonts/NotoSans-Regular.ttf",
    "/system/fonts/DroidSans.ttf",
    "/system/fonts/RobotoStatic-Regular.ttf",
];

pub struct FontBook {
    font: Font,
    pub source: String,
}

impl FontBook {
    /// Roboto ships on every Android device; loading it at runtime avoids
    /// bundling (and licensing) a font in the APK.
    pub fn load() -> Result<FontBook, String> {
        let mut last_err = String::from("no font candidates");
        for path in FONT_CANDIDATES {
            match std::fs::read(path) {
                Ok(bytes) => match Font::from_bytes(bytes, FontSettings::default()) {
                    Ok(font) => {
                        return Ok(FontBook {
                            font,
                            source: (*path).to_string(),
                        })
                    }
                    Err(e) => last_err = format!("fontdue rejected {path}: {e:?}"),
                },
                Err(e) => last_err = format!("cannot read {path}: {e}"),
            }
        }
        Err(last_err)
    }

    pub fn line_height(&self, px: f32) -> f32 {
        match self.font.horizontal_line_metrics(px) {
            Some(m) => m.new_line_size,
            None => px * 1.25,
        }
    }

    pub fn ascent(&self, px: f32) -> f32 {
        match self.font.horizontal_line_metrics(px) {
            Some(m) => m.ascent,
            None => px * 0.9,
        }
    }

    pub fn advance(&self, ch: char, px: f32) -> f32 {
        self.font.metrics(ch, px).advance_width
    }

    pub fn kern(&self, a: char, b: char, px: f32) -> f32 {
        self.font.horizontal_kern(a, b, px).unwrap_or(0.0)
    }

    pub fn rasterize(&self, ch: char, px: f32) -> (Metrics, Vec<u8>) {
        self.font.rasterize(ch, px)
    }
}

/// One wrapped line, as char-index range [start, end).
#[derive(Clone, Copy, Debug)]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
}

/// Greedy word wrap on spaces, hard break for over-long words, honoring '\n'.
pub fn layout_text(fb: &FontBook, text: &str, px: f32, max_w: f32) -> Vec<LineSpan> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![LineSpan { start: 0, end: 0 }];
    }
    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let mut width = 0.0f32;
    let mut last_break: Option<usize> = None; // char index just after a space
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\n' {
            lines.push(LineSpan {
                start: line_start,
                end: i,
            });
            line_start = i + 1;
            width = 0.0;
            last_break = None;
            i += 1;
            continue;
        }
        let adv = fb.advance(ch, px);
        if width + adv > max_w && i > line_start {
            match last_break {
                Some(b) if b > line_start => {
                    lines.push(LineSpan {
                        start: line_start,
                        end: b,
                    });
                    line_start = b;
                }
                _ => {
                    lines.push(LineSpan {
                        start: line_start,
                        end: i,
                    });
                    line_start = i;
                }
            }
            width = 0.0;
            last_break = None;
            continue; // re-measure current char on next iteration
        }
        if ch == ' ' {
            last_break = Some(i + 1);
        }
        width += adv;
        i += 1;
    }
    lines.push(LineSpan {
        start: line_start,
        end: chars.len(),
    });
    lines
}

fn measure(fb: &FontBook, chars: &[char], px: f32) -> f32 {
    let mut w = 0.0;
    for i in 0..chars.len() {
        w += fb.advance(chars[i], px);
        if i + 1 < chars.len() {
            w += fb.kern(chars[i], chars[i + 1], px);
        }
    }
    w
}

pub fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// (x offset within line, line index) for a cursor at char index `cursor`.
pub fn cursor_geometry(
    fb: &FontBook,
    chars: &[char],
    lines: &[LineSpan],
    cursor: usize,
    px: f32,
) -> (f32, usize) {
    for (li, line) in lines.iter().enumerate() {
        if cursor <= line.end {
            let from = line.start.min(cursor);
            let x = measure(fb, &chars[from..cursor], px);
            return (x, li);
        }
    }
    (0.0, lines.len().saturating_sub(1))
}

/// Nearest char index for a tap at (x, y) relative to the text block origin.
pub fn hit_to_cursor(
    fb: &FontBook,
    chars: &[char],
    lines: &[LineSpan],
    x: f32,
    y: f32,
    line_h: f32,
    px: f32,
) -> usize {
    let li = ((y / line_h).floor() as usize).min(lines.len().saturating_sub(1));
    let line = lines[li];
    let mut w = 0.0;
    for i in line.start..line.end {
        let cw = fb.advance(chars[i], px);
        if w + cw * 0.5 >= x {
            return i;
        }
        w += cw;
    }
    line.end.min(chars.len())
}

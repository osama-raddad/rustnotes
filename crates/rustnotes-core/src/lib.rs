//! Rust Notes core — platform-agnostic app state machine, input handling and
//! UI drawing. Shells (android / ios) own the window, GL context and event
//! translation, and drive this code through [`CoreEvent`]s.

pub mod gl;
pub mod notes;
pub mod text;
pub mod util;

pub use gl::{Color, Renderer};

use notes::{Note, NotesDb};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const ACCENT: Color = [0.208, 0.455, 0.957, 1.0];
const TEXT_DARK: Color = [0.10, 0.11, 0.13, 1.0];
const TEXT_GRAY: Color = [0.42, 0.44, 0.50, 1.0];
const CARD_BG: Color = [1.0, 1.0, 1.0, 1.0];
const HEADER_BG: Color = [1.0, 1.0, 1.0, 1.0];
const HAIRLINE: Color = [0.90, 0.90, 0.93, 1.0];
const DANGER: Color = [0.83, 0.19, 0.19, 1.0];
const DANGER_BG: Color = [0.985, 0.91, 0.91, 1.0];
const PLACEHOLDER: Color = [0.62, 0.64, 0.69, 1.0];

/// Services only the platform shell can provide.
pub trait Platform {
    fn show_keyboard(&self);
    fn hide_keyboard(&self);
}

/// Neutral key events (shells translate from their own input systems).
pub enum CoreKey {
    Char(char),
    Enter,
    Backspace,
    Left,
    Right,
    Up,
    Down,
    Back,
}

pub enum CoreEvent {
    /// Coordinates are in pixels, origin top-left, y down.
    TouchDown { x: f32, y: f32 },
    TouchMove { x: f32, y: f32 },
    TouchUp { x: f32, y: f32 },
    TouchCancel,
    Key(CoreKey),
}

pub type Rect = (f32, f32, f32, f32);

fn hit(rect: Option<Rect>, x: f32, y: f32) -> bool {
    matches!(rect, Some((rx, ry, rw, rh)) if x >= rx && x < rx + rw && y >= ry && y < ry + rh)
}

fn in_rect(r: &Rect, x: f32, y: f32) -> bool {
    x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
}

#[derive(PartialEq, Clone, Copy)]
enum Screen {
    List,
    Editor,
}

struct TouchState {
    last: (f32, f32),
    moved: f32,
    dragging: bool,
}

#[derive(Clone, Copy)]
struct EditorGeom {
    x0: f32,
    y0: f32,
    max_w: f32,
    line_h: f32,
    px: f32,
}

pub struct App {
    db: NotesDb,
    screen: Screen,
    notes: Vec<Note>,
    // Editor state
    edit_id: i64, // 0 = new, unsaved
    buffer: String,
    cursor: usize, // char index
    edit_scroll: f32,
    edit_geom: Option<EditorGeom>,
    // List state
    list_scroll: f32,
    list_content_h: f32,
    // Hit targets, refreshed during draw
    hit_add: Option<Rect>,
    hit_back: Option<Rect>,
    hit_rows: Vec<(Rect, usize)>,
    hit_del: Vec<(Rect, usize)>,
    hit_editor: Option<Rect>,
    armed_delete: Option<(usize, Instant)>,
    // Input / animation
    touch: Option<TouchState>,
    blink_on: bool,
    last_blink: Instant,
    pub dirty: bool,
    scale: f32,
    vw: f32,
    vh: f32,
    /// Top content inset (status bar / notch) in px; UI is drawn below this.
    content_top: f32,
    /// Bottom content inset (keyboard / home indicator) in px; 0 = use full height.
    content_bottom: f32,
}

impl App {
    pub fn new(data_dir: PathBuf) -> App {
        let db = NotesDb::open(&data_dir).expect("open notes database");
        let notes = db.list();
        log::info!("loaded {} notes from {}", notes.len(), data_dir.display());
        App {
            db,
            screen: Screen::List,
            notes,
            edit_id: 0,
            buffer: String::new(),
            cursor: 0,
            edit_scroll: 0.0,
            edit_geom: None,
            list_scroll: 0.0,
            list_content_h: 0.0,
            hit_add: None,
            hit_back: None,
            hit_rows: Vec::new(),
            hit_del: Vec::new(),
            hit_editor: None,
            armed_delete: None,
            touch: None,
            blink_on: true,
            last_blink: Instant::now(),
            dirty: true,
            scale: 1.0,
            vw: 1080.0,
            vh: 2280.0,
            content_top: 0.0,
            content_bottom: 0.0,
        }
    }

    pub fn set_content_top(&mut self, px: f32) {
        if px != self.content_top {
            self.content_top = px;
            self.dirty = true;
        }
    }

    pub fn set_content_bottom(&mut self, px: f32) {
        if px != self.content_bottom {
            self.content_bottom = px;
            self.dirty = true;
        }
    }

    fn bottom(&self) -> f32 {
        if self.content_bottom > 0.0 {
            self.content_bottom.min(self.vh)
        } else {
            self.vh
        }
    }

    // ---- editor ----------------------------------------------------------

    fn open_editor(&mut self, id: i64, plat: &dyn Platform) {
        log::info!("editor: open id={id}");
        self.edit_id = id;
        self.buffer = if id == 0 {
            String::new()
        } else {
            self.notes
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.body.clone())
                .unwrap_or_default()
        };
        self.cursor = self.buffer.chars().count();
        self.edit_scroll = 0.0;
        self.screen = Screen::Editor;
        self.blink_on = true;
        self.last_blink = Instant::now();
        plat.show_keyboard();
        self.dirty = true;
    }

    fn close_editor(&mut self, plat: &dyn Platform) {
        let empty = self.buffer.trim().is_empty();
        log::info!("editor: close id={} empty={}", self.edit_id, empty);
        if self.edit_id == 0 {
            if !empty {
                let mut n = Note {
                    id: 0,
                    body: self.buffer.clone(),
                    updated: 0,
                };
                self.db.save(&mut n);
            }
        } else if empty {
            self.db.delete(self.edit_id);
        } else {
            let mut n = Note {
                id: self.edit_id,
                body: self.buffer.clone(),
                updated: 0,
            };
            self.db.save(&mut n);
        }
        self.notes = self.db.list();
        self.screen = Screen::List;
        self.list_scroll = 0.0;
        plat.hide_keyboard();
        self.dirty = true;
    }

    /// Save without leaving the editor (used on app pause).
    pub fn save_current(&mut self) {
        if self.screen != Screen::Editor || self.buffer.trim().is_empty() {
            return;
        }
        let mut n = Note {
            id: self.edit_id,
            body: self.buffer.clone(),
            updated: 0,
        };
        self.db.save(&mut n);
        self.edit_id = n.id;
    }

    fn insert_char(&mut self, c: char) {
        let byte = text::char_to_byte(&self.buffer, self.cursor);
        self.buffer.insert(byte, c);
        self.cursor += 1;
        self.after_edit();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = text::char_to_byte(&self.buffer, self.cursor - 1);
        let end = text::char_to_byte(&self.buffer, self.cursor);
        self.buffer.replace_range(start..end, "");
        self.cursor -= 1;
        self.after_edit();
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.buffer.chars().count() as i32;
        self.cursor = ((self.cursor as i32 + delta).clamp(0, len)) as usize;
        self.after_edit();
    }

    fn after_edit(&mut self) {
        self.blink_on = true;
        self.last_blink = Instant::now();
        self.dirty = true;
    }

    // ---- input ------------------------------------------------------------

    /// Handle one neutral event. Returns whether the shell should consider it
    /// consumed (used e.g. for Android's back key).
    pub fn handle_event(
        &mut self,
        ev: CoreEvent,
        plat: &dyn Platform,
        r: Option<&mut Renderer>,
    ) -> bool {
        match ev {
            CoreEvent::TouchDown { x, y } => {
                self.touch = Some(TouchState {
                    last: (x, y),
                    moved: 0.0,
                    dragging: false,
                });
                true
            }
            CoreEvent::TouchMove { x, y } => {
                if let Some(t) = self.touch.as_mut() {
                    let dx = x - t.last.0;
                    let dy = y - t.last.1;
                    t.moved += dx.abs() + dy.abs();
                    if t.moved > 16.0 * self.scale {
                        t.dragging = true;
                    }
                    if t.dragging {
                        match self.screen {
                            Screen::List => {
                                self.list_scroll -= dy;
                                let max = (self.list_content_h - self.vh).max(0.0);
                                self.list_scroll = self.list_scroll.clamp(0.0, max);
                            }
                            Screen::Editor => {
                                self.edit_scroll = (self.edit_scroll - dy).max(0.0);
                            }
                        }
                        self.dirty = true;
                    }
                    t.last = (x, y);
                }
                true
            }
            CoreEvent::TouchUp { x, y } => {
                if let Some(t) = self.touch.take() {
                    if !t.dragging {
                        self.handle_tap(x, y, plat, r);
                    }
                }
                true
            }
            CoreEvent::TouchCancel => {
                self.touch.take();
                true
            }
            CoreEvent::Key(k) => self.handle_key(k, plat, r),
        }
    }

    fn handle_key(&mut self, k: CoreKey, plat: &dyn Platform, r: Option<&mut Renderer>) -> bool {
        if self.screen == Screen::Editor {
            match k {
                CoreKey::Back => {
                    self.close_editor(plat);
                    true
                }
                CoreKey::Enter => {
                    self.insert_char('\n');
                    true
                }
                CoreKey::Backspace => {
                    self.backspace();
                    true
                }
                CoreKey::Left => {
                    self.move_cursor(-1);
                    true
                }
                CoreKey::Right => {
                    self.move_cursor(1);
                    true
                }
                CoreKey::Up => {
                    self.move_cursor_vertical(-1, r);
                    true
                }
                CoreKey::Down => {
                    self.move_cursor_vertical(1, r);
                    true
                }
                CoreKey::Char(c) => {
                    self.insert_char(c);
                    true
                }
            }
        } else {
            match k {
                CoreKey::Enter => {
                    self.open_editor(0, plat); // quick new note
                    true
                }
                _ => false,
            }
        }
    }

    fn handle_tap(&mut self, x: f32, y: f32, plat: &dyn Platform, r: Option<&mut Renderer>) {
        self.dirty = true;
        match self.screen {
            Screen::List => {
                if hit(self.hit_add, x, y) {
                    self.armed_delete = None;
                    self.open_editor(0, plat);
                    return;
                }
                let dels: Vec<(Rect, usize)> = self.hit_del.clone();
                for (rect, idx) in &dels {
                    if in_rect(rect, x, y) {
                        // Two taps within 2s on ✕ confirms the delete.
                        match self.armed_delete.take() {
                            Some((i, t)) if i == *idx && t.elapsed() < Duration::from_secs(2) => {
                                if let Some(n) = self.notes.get(*idx) {
                                    self.db.delete(n.id);
                                }
                                self.notes = self.db.list();
                            }
                            _ => self.armed_delete = Some((*idx, Instant::now())),
                        }
                        return;
                    }
                }
                let rows: Vec<(Rect, usize)> = self.hit_rows.clone();
                for (rect, idx) in &rows {
                    if in_rect(rect, x, y) {
                        let id = self.notes.get(*idx).map(|n| n.id);
                        if let Some(id) = id {
                            self.armed_delete = None;
                            self.open_editor(id, plat);
                        }
                        return;
                    }
                }
            }
            Screen::Editor => {
                if hit(self.hit_back, x, y) {
                    self.close_editor(plat);
                    return;
                }
                if hit(self.hit_editor, x, y) {
                    if let Some(r) = r {
                        self.place_cursor(x, y, r);
                    }
                    plat.show_keyboard();
                }
            }
        }
    }

    fn place_cursor(&mut self, x: f32, y: f32, r: &mut Renderer) {
        let g = match self.edit_geom {
            Some(g) => g,
            None => return,
        };
        let chars: Vec<char> = self.buffer.chars().collect();
        let lines = text::layout_text(&r.fonts, &self.buffer, g.px, g.max_w);
        let ly = (y - g.y0 + self.edit_scroll).max(0.0);
        self.cursor = text::hit_to_cursor(&r.fonts, &chars, &lines, x - g.x0, ly, g.line_h, g.px)
            .min(chars.len());
        self.after_edit();
    }

    fn move_cursor_vertical(&mut self, dir: i32, r: Option<&mut Renderer>) {
        let Some(r) = r else { return };
        let Some(g) = self.edit_geom else { return };
        let chars: Vec<char> = self.buffer.chars().collect();
        let lines = text::layout_text(&r.fonts, &self.buffer, g.px, g.max_w);
        let (cx, li) = text::cursor_geometry(&r.fonts, &chars, &lines, self.cursor, g.px);
        let ty = (li as f32 + dir as f32).max(0.0) * g.line_h;
        self.cursor = text::hit_to_cursor(&r.fonts, &chars, &lines, cx, ty, g.line_h, g.px)
            .min(chars.len());
        self.after_edit();
    }

    /// Advance animations (cursor blink, armed-delete expiry). Call ~10Hz.
    pub fn tick(&mut self) {
        if self.screen == Screen::Editor && self.last_blink.elapsed() >= Duration::from_millis(530)
        {
            self.blink_on = !self.blink_on;
            self.last_blink = Instant::now();
            self.dirty = true;
        }
        if let Some((_, t)) = self.armed_delete {
            if t.elapsed() > Duration::from_secs(2) {
                self.armed_delete = None;
                self.dirty = true;
            }
        }
    }

    // ---- rendering --------------------------------------------------------

    pub fn draw(&mut self, r: &mut Renderer) {
        self.dirty = false;
        let w = r.w as f32;
        let h = r.h as f32;
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        self.vw = w;
        self.vh = h;
        let top = self.content_top;
        self.scale = (w / 360.0).clamp(0.75, 3.5);
        r.begin();
        match self.screen {
            Screen::List => self.draw_list(r, w, h, top),
            Screen::Editor => self.draw_editor(r, w, h, top),
        }
        r.finish();
    }

    fn draw_list(&mut self, r: &mut Renderer, w: f32, h: f32, top: f32) {
        self.hit_rows.clear();
        self.hit_del.clear();

        let s = self.scale;
        let header_h = 56.0 * s;
        let pad = 16.0 * s;
        let card_m = 12.0 * s;
        let card_pad = 14.0 * s;
        let row_h = 78.0 * s;
        let card_h = row_h - 8.0 * s;
        let title_px = (16.0 * s).round();
        let snip_px = (13.0 * s).round();
        let date_px = (11.0 * s).round();
        let del_r = 16.0 * s;
        let side_w = 64.0 * s; // reserved for the delete button

        // Pre-compute content height and clamp scroll before drawing.
        self.list_content_h =
            top + header_h + 12.0 * s + self.notes.len() as f32 * row_h + 16.0 * s;
        let max_scroll = (self.list_content_h - self.vh).max(0.0);
        self.list_scroll = self.list_scroll.clamp(0.0, max_scroll);

        r.fill_rect(0.0, 0.0, w, h, gl::SCREEN_BG);
        r.fill_rect(0.0, top, w, header_h, HEADER_BG);
        r.fill_rect(0.0, top + header_h - 1.0, w, 1.0, HAIRLINE);

        let base = top + header_h / 2.0 + r.fonts.ascent(title_px) / 2.0;
        r.text_str(pad, base, "Notes", title_px, TEXT_DARK);
        let count = match self.notes.len() {
            0 => "no notes".to_string(),
            1 => "1 note".to_string(),
            n => format!("{n} notes"),
        };
        let tw = r.text_width("Notes", title_px);
        r.text_str(pad + tw + 12.0 * s, base, &count, date_px, TEXT_GRAY);

        // + button (fixed at the right edge)
        let btn_d = 40.0 * s;
        let bx = w - pad - btn_d;
        let by = top + (header_h - btn_d) / 2.0;
        let (cx, cy) = (bx + btn_d / 2.0, by + btn_d / 2.0);
        r.fill_round_rect(bx, by, btn_d, btn_d, btn_d * 0.35, ACCENT);
        let arm = 9.0 * s;
        r.line(cx - arm, cy, cx + arm, cy, 2.5 * s, CARD_BG);
        r.line(cx, cy - arm, cx, cy + arm, 2.5 * s, CARD_BG);
        self.hit_add = Some((bx - 8.0 * s, by - 8.0 * s, btn_d + 16.0 * s, btn_d + 16.0 * s));

        let bottom = self.bottom();
        let mut y = top + header_h + 12.0 * s - self.list_scroll;
        for (idx, note) in self.notes.iter().enumerate() {
            if y + card_h > top + header_h && y < bottom {
                r.fill_round_rect(card_m, y, w - card_m * 2.0, card_h, 10.0 * s, CARD_BG);

                // delete button (right side)
                let del_cx = w - card_m - 32.0 * s;
                let del_cy = y + card_h / 2.0;
                let armed = matches!(self.armed_delete, Some((i, _)) if i == idx);
                r.fill_circle(del_cx, del_cy, del_r, if armed { DANGER } else { DANGER_BG });
                let k = del_r * 0.45;
                let cross = if armed { CARD_BG } else { DANGER };
                r.line(del_cx - k, del_cy - k, del_cx + k, del_cy + k, 2.0 * s, cross);
                r.line(del_cx - k, del_cy + k, del_cx + k, del_cy - k, 2.0 * s, cross);

                let text_w = w - card_m * 2.0 - card_pad * 2.0 - side_w;
                let tx = card_m + card_pad;
                r.text_str_clipped(
                    tx,
                    y + card_pad + r.fonts.ascent(title_px),
                    notes::title_of(&note.body),
                    text_w,
                    title_px,
                    TEXT_DARK,
                );
                if let Some(sn) = notes::snippet_of(&note.body) {
                    r.text_str_clipped(
                        tx,
                        y + card_pad + r.fonts.ascent(title_px) + title_px * 1.45,
                        sn,
                        text_w,
                        snip_px,
                        TEXT_GRAY,
                    );
                }
                let date = util::format_note_date(note.updated);
                let dw = r.text_width(&date, date_px);
                r.text_str(
                    card_m + card_pad + text_w - dw,
                    y + card_h - card_pad - r.fonts.ascent(date_px),
                    &date,
                    date_px,
                    PLACEHOLDER,
                );

                self.hit_rows.push(((card_m, y, w - card_m * 2.0, card_h), idx));
                let hit_d = del_r + 10.0 * s;
                self.hit_del.push((
                    (del_cx - hit_d, del_cy - hit_d, hit_d * 2.0, hit_d * 2.0),
                    idx,
                ));
            }
            y += row_h;
        }

        if self.notes.is_empty() {
            let t1 = "No notes yet";
            let t2 = "Tap + to write one";
            let p1 = (18.0 * s).round();
            let p2 = (14.0 * s).round();
            let mid = top + (bottom - top) * 0.4;
            let w1 = r.text_width(t1, p1);
            let w2 = r.text_width(t2, p2);
            r.text_str((w - w1) / 2.0, mid, t1, p1, TEXT_GRAY);
            r.text_str((w - w2) / 2.0, mid + p1 * 1.4, t2, p2, PLACEHOLDER);
        }
    }

    fn draw_editor(&mut self, r: &mut Renderer, w: f32, h: f32, top: f32) {
        let s = self.scale;
        let header_h = 56.0 * s;
        let pad = 16.0 * s;
        let title_px = (17.0 * s).round();

        r.fill_rect(0.0, 0.0, w, h, CARD_BG);
        r.fill_rect(0.0, top, w, header_h, HEADER_BG);
        r.fill_rect(0.0, top + header_h - 1.0, w, 1.0, HAIRLINE);

        // back chevron
        let cx = 28.0 * s;
        let cy = top + header_h / 2.0;
        r.line(cx - 6.0 * s, cy - 10.0 * s, cx - 12.0 * s, cy, 3.0 * s, TEXT_DARK);
        r.line(cx - 12.0 * s, cy, cx - 6.0 * s, cy + 10.0 * s, 3.0 * s, TEXT_DARK);
        let title = if self.edit_id == 0 { "New note" } else { "Edit note" };
        r.text_str(56.0 * s, cy + r.fonts.ascent(title_px) / 2.0, title, title_px, TEXT_DARK);
        self.hit_back = Some((0.0, top, 52.0 * s, header_h));

        // body
        let px = (17.0 * s).round();
        let line_h = r.fonts.line_height(px);
        let x0 = pad;
        let y0 = top + header_h + 12.0 * s;
        let max_w = w - pad * 2.0;
        let chars: Vec<char> = self.buffer.chars().collect();
        let lines = text::layout_text(&r.fonts, &self.buffer, px, max_w);
        let content_h = lines.len() as f32 * line_h;
        let bottom = self.bottom();
        let area_h = (bottom - y0).max(line_h);

        // keep cursor on screen
        let (cx_off, cli) = text::cursor_geometry(&r.fonts, &chars, &lines, self.cursor, px);
        let cur_top = y0 + cli as f32 * line_h - self.edit_scroll;
        let cur_bottom = cur_top + line_h;
        if cur_bottom > bottom {
            self.edit_scroll += cur_bottom - bottom;
        }
        if cur_top < y0 {
            self.edit_scroll -= y0 - cur_top;
        }
        self.edit_scroll = self
            .edit_scroll
            .clamp(0.0, (content_h - area_h + line_h).max(0.0));

        self.edit_geom = Some(EditorGeom {
            x0,
            y0,
            max_w,
            line_h,
            px,
        });

        if chars.is_empty() {
            r.text_str(x0, y0 + r.fonts.ascent(px), "Start typing...", px, PLACEHOLDER);
        }
        for (li, line) in lines.iter().enumerate() {
            let ly = y0 + li as f32 * line_h - self.edit_scroll;
            if ly + line_h < y0 {
                continue;
            }
            if ly > bottom {
                break;
            }
            let baseline = ly + r.fonts.ascent(px);
            r.text_run(x0, baseline, &chars, line.start, line.end, px, TEXT_DARK);
        }
        if self.blink_on {
            let cy0 = y0 + cli as f32 * line_h - self.edit_scroll;
            r.fill_rect(x0 + cx_off, cy0 + line_h * 0.12, 2.0 * s, line_h * 0.76, ACCENT);
        }
        self.hit_editor = Some((0.0, top + header_h, w, bottom - top - header_h));
    }
}

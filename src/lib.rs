//! Rust Notes — a pure-Rust Android app. No Kotlin, no Java, no Gradle.
//!
//! android-activity (NativeActivity) drives the lifecycle; we render the UI
//! with OpenGL ES 3 (see gl.rs) and store notes in SQLite (see notes.rs).

mod egl;
mod gl;
mod notes;
mod text;
mod util;

use android_activity::input::{InputEvent, KeyAction, MotionAction};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use gl::{Color, Renderer};
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

type Rect = (f32, f32, f32, f32);

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
    down: (f32, f32),
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

struct App {
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
    dirty: bool,
    scale: f32,
    vw: f32,
    vh: f32,
    /// Top content inset (status bar height) — all UI is drawn below this.
    content_top: f32,
    got_insets: bool,
}

impl App {
    fn new(app: &AndroidApp) -> App {
        let dir: PathBuf = app
            .internal_data_path()
            .unwrap_or_else(|| PathBuf::from("/data/local/tmp"));
        let db = NotesDb::open(&dir).expect("open notes database");
        let notes = db.list();
        log::info!("loaded {} notes from {}", notes.len(), dir.display());
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
            got_insets: false,
        }
    }

    // ---- editor ----------------------------------------------------------

    fn open_editor(&mut self, id: i64, app: &AndroidApp) {
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
        app.show_soft_input(false);
        self.dirty = true;
    }

    fn close_editor(&mut self, app: &AndroidApp) {
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
        app.hide_soft_input(false);
        self.dirty = true;
    }

    /// Save without leaving the editor (used on Pause/Destroy).
    fn save_current(&mut self) {
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

    fn handle_input(
        &mut self,
        ev: &InputEvent,
        app: &AndroidApp,
        r: Option<&mut Renderer>,
    ) -> InputStatus {
        match ev {
            InputEvent::MotionEvent(m) => {
                let action = m.action();
                let p = m.pointer_at_index(0);
                let (x, y) = (p.x(), p.y());
                match action {
                    MotionAction::Down => {
                        self.touch = Some(TouchState {
                            down: (x, y),
                            last: (x, y),
                            moved: 0.0,
                            dragging: false,
                        });
                        InputStatus::Handled
                    }
                    MotionAction::Move => {
                        if let Some(t) = self.touch.as_mut() {
                            let dx = x - t.last.0;
                            let dy = y - t.last.1;
                            t.moved += dx.abs() + dy.abs();
                            if t.moved > 16.0 * self.scale {
                                t.dragging = true;
                            }
                            t.last = (x, y);
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
                        }
                        InputStatus::Handled
                    }
                    MotionAction::Up | MotionAction::Cancel => {
                        if let Some(t) = self.touch.take() {
                            if action == MotionAction::Up && !t.dragging {
                                self.handle_tap(t.down.0, t.down.1, app, r);
                            }
                        }
                        InputStatus::Handled
                    }
                    _ => InputStatus::Unhandled,
                }
            }
            InputEvent::KeyEvent(k) => {
                let code: u32 = k.key_code().into();
                log::debug!("key event: {k:?}");
                if k.action() != KeyAction::Down {
                    return InputStatus::Unhandled;
                }
                let code: u32 = k.key_code().into();
                let meta = k.meta_state().0;

                if self.screen == Screen::Editor {
                    match code {
                        4 => {
                            // BACK
                            self.close_editor(app);
                            InputStatus::Handled
                        }
                        66 => {
                            // ENTER
                            self.insert_char('\n');
                            InputStatus::Handled
                        }
                        67 => {
                            // DEL / backspace
                            self.backspace();
                            InputStatus::Handled
                        }
                        21 => {
                            self.move_cursor(-1);
                            InputStatus::Handled
                        }
                        22 => {
                            self.move_cursor(1);
                            InputStatus::Handled
                        }
                        19 => {
                            self.move_cursor_vertical(-1, r);
                            InputStatus::Handled
                        }
                        20 => {
                            self.move_cursor_vertical(1, r);
                            InputStatus::Handled
                        }
                        _ => match key_to_char(code, meta) {
                            Some(c) => {
                                self.insert_char(c);
                                InputStatus::Handled
                            }
                            None => InputStatus::Unhandled,
                        },
                    }
                } else {
                    match code {
                        66 => {
                            self.open_editor(0, app); // ENTER: quick new note
                            InputStatus::Handled
                        }
                        _ => InputStatus::Unhandled,
                    }
                }
            }
            _ => InputStatus::Unhandled,
        }
    }

    fn handle_tap(&mut self, x: f32, y: f32, app: &AndroidApp, r: Option<&mut Renderer>) {
        self.dirty = true;
        match self.screen {
            Screen::List => {
                if hit(self.hit_add, x, y) {
                    self.armed_delete = None;
                    self.open_editor(0, app);
                    return;
                }
                let dels: Vec<(Rect, usize)> = self.hit_del.clone();
                for (rect, idx) in &dels {
                    if in_rect(rect, x, y) {
                        // Two taps within 2s on ✕ confirms the delete.
                        match self.armed_delete.take() {
                            Some((i, t))
                                if i == *idx && t.elapsed() < Duration::from_secs(2) =>
                            {
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
                            self.open_editor(id, app);
                        }
                        return;
                    }
                }
            }
            Screen::Editor => {
                if hit(self.hit_back, x, y) {
                    self.close_editor(app);
                    return;
                }
                if hit(self.hit_editor, x, y) {
                    if let Some(r) = r {
                        self.place_cursor(x, y, r);
                    }
                    app.show_soft_input(false);
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

    fn clamp_list_scroll(&mut self) {
        let max = (self.list_content_h - self.vh).max(0.0);
        self.list_scroll = self.list_scroll.clamp(0.0, max);
    }

    fn clamp_editor_scroll(&mut self) {
        // Uses last-known geometry; recalculated properly during draw.
        self.edit_scroll = self.edit_scroll.max(0.0);
    }

    fn tick(&mut self) {
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

    fn draw(&mut self, r: &mut Renderer) {
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
        let count = format!("{} notes", self.notes.len());
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

        let mut y = top + header_h + 12.0 * s - self.list_scroll;
        for (idx, note) in self.notes.iter().enumerate() {
            if y + card_h > top + header_h && y < h {
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
            let mid = top + (h - top) * 0.4;
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
        let area_h = (h - y0).max(line_h);

        // keep cursor on screen
        let (cx_off, cli) = text::cursor_geometry(&r.fonts, &chars, &lines, self.cursor, px);
        let cur_top = y0 + cli as f32 * line_h - self.edit_scroll;
        let cur_bottom = cur_top + line_h;
        if cur_bottom > h {
            self.edit_scroll += cur_bottom - h;
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
            if ly > h {
                break;
            }
            let baseline = ly + r.fonts.ascent(px);
            r.text_run(x0, baseline, &chars, line.start, line.end, px, TEXT_DARK);
        }
        if self.blink_on {
            let cy0 = y0 + cli as f32 * line_h - self.edit_scroll;
            r.fill_rect(x0 + cx_off, cy0 + line_h * 0.12, 2.0 * s, line_h * 0.76, ACCENT);
        }
        self.hit_editor = Some((0.0, top + header_h, w, h - top - header_h));
    }
}

/// Android AKEYCODE_* values -> ASCII char, honoring shift. Key events are how
/// NativeActivity delivers soft-keyboard input (English/basic latin).
fn key_to_char(code: u32, meta: u32) -> Option<char> {
    let shift = meta & (0x01 | 0x40 | 0x80) != 0; // SHIFT_ON | SHIFT_LEFT_ON | SHIFT_RIGHT_ON
    let c = match code {
        29..=54 => {
            let letter = (b'a' + (code - 29) as u8) as char;
            if shift {
                letter.to_ascii_uppercase()
            } else {
                letter
            }
        }
        7..=16 => {
            let d = (b'0' + (code - 7) as u8) as char;
            if shift {
                match d {
                    '1' => '!',
                    '2' => '@',
                    '3' => '#',
                    '4' => '$',
                    '5' => '%',
                    '6' => '^',
                    '7' => '&',
                    '8' => '*',
                    '9' => '(',
                    _ => ')',
                }
            } else {
                d
            }
        }
        62 => ' ',
        55 => if shift { '<' } else { ',' },
        56 => if shift { '>' } else { '.' },
        68 => if shift { '~' } else { '`' },
        69 => if shift { '_' } else { '-' },
        70 => if shift { '+' } else { '=' },
        71 => if shift { '{' } else { '[' },
        72 => if shift { '}' } else { ']' },
        73 => if shift { '|' } else { '\\' },
        74 => if shift { ':' } else { ';' },
        75 => if shift { '"' } else { '\'' },
        76 => if shift { '?' } else { '/' },
        17 => '*',
        18 => '#',
        77 => '@',
        81 => '+',
        _ => return None,
    };
    Some(c)
}

/// Ask the Activity (on the Java main thread) for the system-bar top inset.
/// NativeActivity lays out under the status bar, so we must offset our UI
/// ourselves. Returns 0 if anything goes wrong.
fn query_top_inset(app: &AndroidApp) -> f32 {
    let (tx, rx) = std::sync::mpsc::channel::<i32>();
    let app2 = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr().cast()) };
        let res = vm.attach_current_thread(|env| -> jni::errors::Result<i32> {
            // Borrow the unowned global ref; forget it so it isn't deleted.
            let activity = unsafe {
                jni::objects::JObject::from_raw(
                    env,
                    app2.activity_as_ptr() as jni::sys::jobject,
                )
            };
            let r = (|| -> jni::errors::Result<i32> {
                let window = env
                    .call_method(
                        &activity,
                        jni::jni_str!("getWindow"),
                        jni::jni_sig!("()Landroid/view/Window;"),
                        &[],
                    )?
                    .l()?;
                let decor = env
                    .call_method(
                        &window,
                        jni::jni_str!("getDecorView"),
                        jni::jni_sig!("()Landroid/view/View;"),
                        &[],
                    )?
                    .l()?;
                let insets = env
                    .call_method(
                        &decor,
                        jni::jni_str!("getRootWindowInsets"),
                        jni::jni_sig!("()Landroid/view/WindowInsets;"),
                        &[],
                    )?
                    .l()?;
                let tclass = env.find_class(jni::jni_str!("android/view/WindowInsets$Type"))?;
                let sysbars = env
                    .call_static_method(
                        &tclass,
                        jni::jni_str!("systemBars"),
                        jni::jni_sig!("()I"),
                        &[],
                    )?
                    .i()?;
                let bars = env
                    .call_method(
                        &insets,
                        jni::jni_str!("getInsets"),
                        jni::jni_sig!("(I)Landroid/graphics/Insets;"),
                        &[jni::JValue::from(sysbars)],
                    )?
                    .l()?;
                Ok(env
                    .get_field(&bars, jni::jni_str!("top"), jni::jni_sig!("I"))?
                    .i()?)
            })();
            std::mem::forget(activity);
            r
        });
        let _ = tx.send(res.unwrap_or(0));
    }));
    rx.recv().map(|v| v.max(0) as f32).unwrap_or(0.0)
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("rustnotes")
            .with_max_level(log::LevelFilter::Info),
    );
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));

    let mut state = App::new(&app);
    let mut renderer: Option<Renderer> = None;
    let mut destroyed = false;
    let mut input_pending = false;

    while !destroyed {
        app.poll_events(Some(Duration::from_millis(120)), |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => {
                let rect = app.content_rect();
                log::info!("content_rect: {rect:?}");
                state.content_top = rect.top.max(0) as f32;
                if let Some(win) = app.native_window() {
                    match Renderer::new(win.ptr().as_ptr() as *mut std::ffi::c_void) {
                        Ok(mut r) => {
                            r.resize();
                            log::info!(
                                "renderer {}x{}, font: {}",
                                r.w,
                                r.h,
                                r.fonts.source
                            );
                            renderer = Some(r);
                            state.dirty = true;
                        }
                        Err(e) => log::error!("renderer init failed: {e}"),
                    }
                }
            }
            PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                if let Some(mut r) = renderer.take() {
                    r.destroy();
                }
            }
            PollEvent::Main(MainEvent::WindowResized { .. })
            | PollEvent::Main(MainEvent::ContentRectChanged { .. }) => {
                let rect = app.content_rect();
                let new_top = rect.top.max(0) as f32;
                if (new_top - state.content_top).abs() > 0.5 {
                    log::info!("content top inset: {new_top} (rect {rect:?})");
                }
                state.content_top = new_top;
                if let Some(r) = renderer.as_mut() {
                    r.resize();
                }
                state.dirty = true;
            }
            PollEvent::Main(MainEvent::RedrawNeeded { .. }) => {
                if let Some(r) = renderer.as_mut() {
                    r.resize();
                    state.draw(r);
                }
            }
            PollEvent::Main(MainEvent::GainedFocus) => {
                if !state.got_insets {
                    state.got_insets = true;
                    let top = query_top_inset(&app);
                    log::info!("system bar top inset: {top}");
                    state.content_top = top;
                    state.dirty = true;
                }
            }
            PollEvent::Main(MainEvent::InputAvailable) => input_pending = true,
            PollEvent::Main(MainEvent::Pause) => {
                state.save_current();
                state.dirty = true;
            }
            PollEvent::Main(MainEvent::Resume { .. }) => state.dirty = true,
            PollEvent::Main(MainEvent::Destroy) => destroyed = true,
            _ => {}
        });

        if destroyed {
            break;
        }

        if input_pending {
            input_pending = false;
            if let Ok(mut iter) = app.input_events_iter() {
                loop {
                    let more = iter.next(|ev| state.handle_input(ev, &app, renderer.as_mut()));
                    if !more {
                        break;
                    }
                }
            }
        }

        state.tick();
        if state.dirty {
            if let Some(r) = renderer.as_mut() {
                state.draw(r);
            }
        }
    }

    state.save_current();
    log::info!("android_main exiting");
}

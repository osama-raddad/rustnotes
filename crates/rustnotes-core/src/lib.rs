//! Rust Notes core — platform-agnostic app state machine, input translation and
//! UI. The UI is egui, painted by egui_glow onto a GLES3 context the shell owns.
//! Shells (android / ios) own the window, GL context and event translation, and
//! drive this code through [`CoreEvent`]s.

pub mod notes;
pub mod util;

use egui::epaint::MarginF32;
use egui::{
    Align, Align2, Color32, Event, FontData, FontDefinitions, FontFamily, FontId, Id, Key, Layout,
    Modifiers, PointerButton, Pos2, RawInput, Rect, SafeAreaInsets, Sense, Stroke, TouchDeviceId,
    TouchId, TouchPhase, Ui, UiBuilder, Vec2, ViewportId, ViewportIdMap, ViewportInfo,
};
use egui_glow::{Painter, ShaderVersion};
use notes::{Note, NotesDb};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const SCREEN_BG: Color32 = Color32::from_rgb(246, 246, 248);
const ACCENT: Color32 = Color32::from_rgb(53, 116, 244);
const TEXT_DARK: Color32 = Color32::from_rgb(26, 28, 33);
const TEXT_GRAY: Color32 = Color32::from_rgb(107, 112, 128);
const CARD_BG: Color32 = Color32::WHITE;
const HEADER_BG: Color32 = Color32::WHITE;
const HAIRLINE: Color32 = Color32::from_rgb(230, 230, 237);
const DANGER: Color32 = Color32::from_rgb(212, 48, 48);
const DANGER_BG: Color32 = Color32::from_rgb(251, 232, 232);
const PLACEHOLDER: Color32 = Color32::from_rgb(158, 163, 176);

/// Fonts load at runtime from the device so the bundle carries no font file.
const FONT_CANDIDATES: &[&str] = &[
    "/system/fonts/Roboto-Regular.ttf",    // Android
    "/system/fonts/NotoSans-Regular.ttf",  // Android fallback
    "/System/Library/Fonts/SFNS.ttf",      // iOS: San Francisco
    "/System/Library/Fonts/Geneva.ttf",    // iOS fallback
    "/System/Library/Fonts/DroidSans.ttf", // Android last resort
];

const EDITOR_ID: &str = "rustnotes.editor";
const DELETE_ARM_WINDOW: Duration = Duration::from_secs(2);

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

#[derive(PartialEq, Clone, Copy)]
enum Screen {
    List,
    Editor,
}

enum ListAction {
    Open(usize),
    Delete(usize),
}

pub struct App {
    db: NotesDb,
    screen: Screen,
    notes: Vec<Note>,
    // Editor state
    edit_id: i64, // 0 = new, unsaved
    buffer: String,
    focus_editor: bool,
    // List state
    reset_list_scroll: bool,
    armed_delete: Option<(usize, Instant)>,
    // egui
    ctx: egui::Context,
    painter: Option<Painter>,
    font_source: Result<String, String>,
    events: Vec<Event>,
    size_px: [u32; 2],
    /// Points-per-pixel scale: 360 "dp" across the width, like the old renderer.
    ppp: f32,
    /// Top content inset (status bar / notch) in px.
    content_top: f32,
    /// Bottom content inset (keyboard / home indicator) in px.
    content_bottom: f32,
    start: Instant,
    next_repaint: Option<Instant>,
    keyboard_shown: bool,
    /// A touch was released since the last frame (re-shows a dismissed keyboard).
    tapped: bool,
    pub dirty: bool,
}

fn load_fonts() -> Result<(FontDefinitions, String), String> {
    let mut last_err = String::from("no font candidates");
    for path in FONT_CANDIDATES {
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut defs = FontDefinitions::empty();
                defs.font_data
                    .insert("system".to_owned(), Arc::new(FontData::from_owned(bytes)));
                defs.families
                    .insert(FontFamily::Proportional, vec!["system".to_owned()]);
                defs.families
                    .insert(FontFamily::Monospace, vec!["system".to_owned()]);
                return Ok((defs, (*path).to_string()));
            }
            Err(e) => last_err = format!("cannot read {path}: {e}"),
        }
    }
    Err(last_err)
}

impl App {
    pub fn new(data_dir: PathBuf) -> App {
        let db = NotesDb::open(&data_dir).expect("open notes database");
        let notes = db.list();
        log::info!("loaded {} notes from {}", notes.len(), data_dir.display());

        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Light);
        ctx.style_mut_of(egui::Theme::Light, |s| {
            s.visuals.panel_fill = SCREEN_BG;
            s.visuals.extreme_bg_color = Color32::TRANSPARENT;
            s.visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
            s.visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);
            s.spacing.item_spacing = Vec2::ZERO;
            s.interaction.selectable_labels = false;
        });
        let font_source = match load_fonts() {
            Ok((defs, src)) => {
                ctx.set_fonts(defs);
                Ok(src)
            }
            Err(e) => {
                log::error!("font load failed: {e}");
                Err(e)
            }
        };

        App {
            db,
            screen: Screen::List,
            notes,
            edit_id: 0,
            buffer: String::new(),
            focus_editor: false,
            reset_list_scroll: false,
            armed_delete: None,
            ctx,
            painter: None,
            font_source,
            events: Vec::new(),
            size_px: [0, 0],
            ppp: 1.0,
            content_top: 0.0,
            content_bottom: 0.0,
            start: Instant::now(),
            next_repaint: None,
            keyboard_shown: false,
            tapped: false,
            dirty: true,
        }
    }

    // ---- GL / viewport -----------------------------------------------------

    /// Attach the shell's (current) GL context. Fails if no system font loaded.
    pub fn attach_gl(&mut self, gl: Arc<glow::Context>) -> Result<(), String> {
        let src = self.font_source.clone()?;
        let painter = Painter::new(gl, "", Some(ShaderVersion::Es300), false)
            .map_err(|e| format!("{e:?}"))?;
        log::info!(
            "egui_glow painter ready (max texture {}), font: {src}",
            painter.max_texture_side()
        );
        self.painter = Some(painter);
        self.dirty = true;
        Ok(())
    }

    /// Release GL resources before the shell destroys the context.
    pub fn detach_gl(&mut self) {
        if let Some(mut p) = self.painter.take() {
            p.destroy();
        }
    }

    pub fn font_source(&self) -> &str {
        match &self.font_source {
            Ok(s) => s,
            Err(_) => "<none>",
        }
    }

    pub fn set_viewport(&mut self, w: i32, h: i32) {
        let size = [w.max(0) as u32, h.max(0) as u32];
        if size != self.size_px {
            self.size_px = size;
            self.ppp = (size[0] as f32 / 360.0).clamp(0.75, 3.5);
            self.dirty = true;
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
        self.screen = Screen::Editor;
        self.focus_editor = true;
        plat.show_keyboard();
        self.keyboard_shown = true;
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
        self.reset_list_scroll = true;
        plat.hide_keyboard();
        self.keyboard_shown = false;
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

    // ---- input ------------------------------------------------------------

    fn pos(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(x / self.ppp, y / self.ppp)
    }

    fn touch(&mut self, phase: TouchPhase, pos: Pos2) {
        self.events.push(Event::Touch {
            device_id: TouchDeviceId(0),
            id: TouchId(0),
            phase,
            pos,
            force: None,
        });
    }

    fn push_key(&mut self, key: Key) {
        for pressed in [true, false] {
            self.events.push(Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Modifiers::NONE,
            });
        }
    }

    /// Handle one neutral event. Returns whether the shell should consider it
    /// consumed (used e.g. for Android's back key).
    pub fn handle_event(&mut self, ev: CoreEvent, plat: &dyn Platform) -> bool {
        self.dirty = true;
        match ev {
            CoreEvent::TouchDown { x, y } => {
                let p = self.pos(x, y);
                self.touch(TouchPhase::Start, p);
                self.events.push(Event::PointerMoved(p));
                self.events.push(Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                });
                true
            }
            CoreEvent::TouchMove { x, y } => {
                let p = self.pos(x, y);
                self.touch(TouchPhase::Move, p);
                self.events.push(Event::PointerMoved(p));
                true
            }
            CoreEvent::TouchUp { x, y } => {
                let p = self.pos(x, y);
                self.touch(TouchPhase::End, p);
                self.events.push(Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                });
                self.events.push(Event::PointerGone);
                self.tapped = true;
                true
            }
            CoreEvent::TouchCancel => {
                self.touch(TouchPhase::Cancel, Pos2::ZERO);
                self.events.push(Event::PointerGone);
                true
            }
            CoreEvent::Key(k) => self.handle_key(k, plat),
        }
    }

    fn handle_key(&mut self, k: CoreKey, plat: &dyn Platform) -> bool {
        if self.screen == Screen::Editor {
            match k {
                CoreKey::Back => self.close_editor(plat),
                CoreKey::Char(c) => self.events.push(Event::Text(c.to_string())),
                CoreKey::Enter => self.push_key(Key::Enter),
                CoreKey::Backspace => self.push_key(Key::Backspace),
                CoreKey::Left => self.push_key(Key::ArrowLeft),
                CoreKey::Right => self.push_key(Key::ArrowRight),
                CoreKey::Up => self.push_key(Key::ArrowUp),
                CoreKey::Down => self.push_key(Key::ArrowDown),
            }
            true
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

    /// Fire due repaints (cursor blink, scroll momentum, armed-delete expiry).
    /// Call ~10Hz or faster.
    pub fn tick(&mut self) {
        if let Some(t) = self.next_repaint {
            if Instant::now() >= t {
                self.next_repaint = None;
                self.dirty = true;
            }
        }
    }

    // ---- frame ------------------------------------------------------------

    /// Run one egui frame and paint it into the currently bound framebuffer.
    pub fn frame(&mut self, plat: &dyn Platform) {
        self.dirty = false;
        let [w, h] = self.size_px;
        let Some(max_texture_side) = self.painter.as_ref().map(|p| p.max_texture_side()) else {
            self.events.clear();
            return;
        };
        if w == 0 || h == 0 {
            return;
        }
        let ppp = self.ppp;
        let screen_rect =
            Rect::from_min_size(Pos2::ZERO, Vec2::new(w as f32 / ppp, h as f32 / ppp));
        let mut viewports = ViewportIdMap::default();
        viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                native_pixels_per_point: Some(ppp),
                ..Default::default()
            },
        );
        let raw = RawInput {
            viewport_id: ViewportId::ROOT,
            viewports,
            safe_area_insets: Some(SafeAreaInsets(MarginF32 {
                left: 0.0,
                right: 0.0,
                top: self.content_top / ppp,
                bottom: self.content_bottom / ppp,
            })),
            screen_rect: Some(screen_rect),
            max_texture_side: Some(max_texture_side),
            time: Some(self.start.elapsed().as_secs_f64()),
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };

        let ctx = self.ctx.clone();
        let mut out = ctx.run_ui(raw, |ui| self.ui(ui, plat));

        // Soft keyboard follows egui's "is a text field being edited" signal.
        // A tap while editing re-shows a keyboard the OS dismissed.
        let editing = out.platform_output.ime.is_some();
        let tapped = std::mem::take(&mut self.tapped);
        if editing {
            if tapped || !self.keyboard_shown {
                plat.show_keyboard();
                self.keyboard_shown = true;
            }
        } else if self.keyboard_shown {
            plat.hide_keyboard();
            self.keyboard_shown = false;
        }

        // Repaint scheduling: egui asks for 0 (animations / multi-pass) or a
        // delay (cursor blink); Duration::MAX means "idle until input".
        let delay = out
            .viewport_output
            .get(&ViewportId::ROOT)
            .map(|v| v.repaint_delay)
            .unwrap_or(Duration::MAX);
        if delay.is_zero() {
            self.dirty = true;
            self.next_repaint = None;
        } else if delay < Duration::from_secs(3600) {
            self.next_repaint = Some(Instant::now() + delay);
        } else {
            self.next_repaint = None;
        }

        let primitives = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        let painter = self.painter.as_mut().expect("painter attached");
        let [r, g, b, _] = SCREEN_BG.to_normalized_gamma_f32();
        painter.clear([w, h], [r, g, b, 1.0]);
        painter.paint_and_update_textures(
            [w, h],
            out.pixels_per_point,
            &primitives,
            &mut out.textures_delta,
        );
    }

    // ---- UI ---------------------------------------------------------------

    fn ui(&mut self, ui: &mut Ui, plat: &dyn Platform) {
        let ctx = ui.ctx().clone();
        let full = ctx.viewport_rect();
        let content = ctx.content_rect();
        let bg = match self.screen {
            Screen::List => SCREEN_BG,
            Screen::Editor => CARD_BG,
        };
        ui.painter().rect_filled(full, 0.0, bg);
        let mut body = ui.new_child(
            UiBuilder::new()
                .max_rect(content)
                .layout(Layout::top_down(Align::Min)),
        );
        match self.screen {
            Screen::List => self.list_ui(&mut body, plat),
            Screen::Editor => self.editor_ui(&mut body, plat),
        }
    }

    /// Header bar: fills `header_h`, returns its rect (caller paints contents).
    fn header(ui: &mut Ui, header_h: f32) -> Rect {
        let (header, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), header_h), Sense::hover());
        let p = ui.painter();
        p.rect_filled(header, 0.0, HEADER_BG);
        p.line_segment(
            [header.left_bottom(), header.right_bottom()],
            Stroke::new(1.0, HAIRLINE),
        );
        header
    }

    fn list_ui(&mut self, ui: &mut Ui, plat: &dyn Platform) {
        let pad = 16.0;
        let header = Self::header(ui, 56.0);
        let p = ui.painter();
        let title = p.text(
            Pos2::new(header.left() + pad, header.center().y),
            Align2::LEFT_CENTER,
            "Notes",
            FontId::proportional(16.0),
            TEXT_DARK,
        );
        let count = match self.notes.len() {
            0 => "no notes".to_string(),
            1 => "1 note".to_string(),
            n => format!("{n} notes"),
        };
        p.text(
            Pos2::new(title.right() + 12.0, header.center().y),
            Align2::LEFT_CENTER,
            count,
            FontId::proportional(11.0),
            TEXT_GRAY,
        );

        // + button (fixed at the right edge)
        let btn_d = 40.0;
        let btn = Rect::from_center_size(
            Pos2::new(header.right() - pad - btn_d / 2.0, header.center().y),
            Vec2::splat(btn_d),
        );
        let add = ui.interact(btn.expand(8.0), Id::new("add"), Sense::click());
        let p = ui.painter();
        p.rect_filled(btn, btn_d * 0.35, ACCENT);
        let c = btn.center();
        let arm = 9.0;
        p.line_segment(
            [c - Vec2::new(arm, 0.0), c + Vec2::new(arm, 0.0)],
            Stroke::new(2.5, CARD_BG),
        );
        p.line_segment(
            [c - Vec2::new(0.0, arm), c + Vec2::new(0.0, arm)],
            Stroke::new(2.5, CARD_BG),
        );
        if add.clicked() {
            self.armed_delete = None;
            self.open_editor(0, plat);
            return;
        }

        // Armed delete expires after 2 s without a repaint-triggering input.
        if let Some((_, t)) = self.armed_delete {
            let elapsed = t.elapsed();
            if elapsed > DELETE_ARM_WINDOW {
                self.armed_delete = None;
            } else {
                ui.ctx()
                    .request_repaint_after(DELETE_ARM_WINDOW - elapsed);
            }
        }

        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("list")
            .auto_shrink(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if std::mem::take(&mut self.reset_list_scroll) {
            scroll = scroll.vertical_scroll_offset(0.0);
        }

        let notes = &self.notes;
        let armed_delete = self.armed_delete;
        let mut action: Option<ListAction> = None;
        scroll.show(ui, |ui| {
            let view = ui.available_rect_before_wrap();
            ui.add_space(12.0);
            let w = ui.available_width();
            let card_m = 12.0;
            let card_pad = 14.0;
            let row_h = 78.0;
            let card_h = row_h - 8.0;
            let del_r = 16.0;
            let side_w = 64.0; // reserved for the delete button
            let title_px = 16.0;

            for (idx, note) in notes.iter().enumerate() {
                let (row, _) = ui.allocate_exact_size(Vec2::new(w, row_h), Sense::hover());
                let card = Rect::from_min_size(
                    Pos2::new(row.left() + card_m, row.top()),
                    Vec2::new(w - card_m * 2.0, card_h),
                );
                // Delete is registered after the row so it wins the hit test.
                let row_resp = ui.interact(card, Id::new(("row", note.id)), Sense::click());
                let del_c = Pos2::new(card.right() - 32.0, card.center().y);
                let del_hit = Rect::from_center_size(del_c, Vec2::splat((del_r + 10.0) * 2.0));
                let del_resp = ui.interact(del_hit, Id::new(("del", note.id)), Sense::click());
                if del_resp.clicked() {
                    action = Some(ListAction::Delete(idx));
                } else if row_resp.clicked() {
                    action = Some(ListAction::Open(idx));
                }
                if !ui.is_rect_visible(card) {
                    continue;
                }

                let p = ui.painter();
                p.rect_filled(card, 10.0, CARD_BG);

                let armed = matches!(armed_delete, Some((i, _)) if i == idx);
                p.circle_filled(del_c, del_r, if armed { DANGER } else { DANGER_BG });
                let k = del_r * 0.45;
                let cross = if armed { CARD_BG } else { DANGER };
                p.line_segment(
                    [del_c + Vec2::new(-k, -k), del_c + Vec2::new(k, k)],
                    Stroke::new(2.0, cross),
                );
                p.line_segment(
                    [del_c + Vec2::new(-k, k), del_c + Vec2::new(k, -k)],
                    Stroke::new(2.0, cross),
                );

                let text_w = card.width() - card_pad * 2.0 - side_w;
                let tx = card.left() + card_pad;
                let clip = Rect::from_min_size(
                    Pos2::new(tx, card.top()),
                    Vec2::new(text_w, card_h),
                );
                let pc = p.with_clip_rect(clip);
                pc.text(
                    Pos2::new(tx, card.top() + card_pad),
                    Align2::LEFT_TOP,
                    notes::title_of(&note.body),
                    FontId::proportional(title_px),
                    TEXT_DARK,
                );
                if let Some(sn) = notes::snippet_of(&note.body) {
                    pc.text(
                        Pos2::new(tx, card.top() + card_pad + title_px * 1.45),
                        Align2::LEFT_TOP,
                        sn,
                        FontId::proportional(13.0),
                        TEXT_GRAY,
                    );
                }
                p.text(
                    Pos2::new(tx + text_w, card.bottom() - card_pad),
                    Align2::RIGHT_BOTTOM,
                    util::format_note_date(note.updated),
                    FontId::proportional(11.0),
                    PLACEHOLDER,
                );
            }
            ui.add_space(16.0);

            if notes.is_empty() {
                let mid = view.top() + view.height() * 0.4;
                let p = ui.painter();
                let t1 = p.text(
                    Pos2::new(view.center().x, mid),
                    Align2::CENTER_TOP,
                    "No notes yet",
                    FontId::proportional(18.0),
                    TEXT_GRAY,
                );
                p.text(
                    Pos2::new(view.center().x, t1.bottom() + 6.0),
                    Align2::CENTER_TOP,
                    "Tap + to write one",
                    FontId::proportional(14.0),
                    PLACEHOLDER,
                );
            }
        });

        match action {
            Some(ListAction::Open(idx)) => {
                if let Some(id) = self.notes.get(idx).map(|n| n.id) {
                    self.armed_delete = None;
                    self.open_editor(id, plat);
                }
            }
            Some(ListAction::Delete(idx)) => {
                // Two taps within 2 s on ✕ confirms the delete.
                match self.armed_delete.take() {
                    Some((i, t)) if i == idx && t.elapsed() < DELETE_ARM_WINDOW => {
                        if let Some(n) = self.notes.get(idx) {
                            self.db.delete(n.id);
                        }
                        self.notes = self.db.list();
                    }
                    _ => self.armed_delete = Some((idx, Instant::now())),
                }
            }
            None => {}
        }
    }

    fn editor_ui(&mut self, ui: &mut Ui, plat: &dyn Platform) {
        let header = Self::header(ui, 56.0);
        let back = ui.interact(
            Rect::from_min_size(header.left_top(), Vec2::new(52.0, header.height())),
            Id::new("back"),
            Sense::click(),
        );
        let p = ui.painter();
        // back chevron
        let c = Pos2::new(header.left() + 28.0, header.center().y);
        let stroke = Stroke::new(3.0, TEXT_DARK);
        p.line_segment([c + Vec2::new(-6.0, -10.0), c + Vec2::new(-12.0, 0.0)], stroke);
        p.line_segment([c + Vec2::new(-12.0, 0.0), c + Vec2::new(-6.0, 10.0)], stroke);
        let title = if self.edit_id == 0 { "New note" } else { "Edit note" };
        p.text(
            Pos2::new(header.left() + 56.0, header.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(17.0),
            TEXT_DARK,
        );
        if back.clicked() {
            self.close_editor(plat);
            return;
        }

        let id = Id::new(EDITOR_ID);
        if std::mem::take(&mut self.focus_editor) {
            ui.memory_mut(|m| m.request_focus(id));
        }
        let body_h = ui.available_height();
        egui::ScrollArea::vertical()
            .id_salt("editor")
            .auto_shrink(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.add_space(12.0);
                // The field fills the whole body so a tap anywhere below the
                // header lands on it; TextEdit scrolls its cursor into view.
                let min = Vec2::new(ui.available_width(), (body_h - 12.0).max(0.0));
                ui.add(
                    egui::TextEdit::multiline(&mut self.buffer)
                        .id(id)
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                            left: 16,
                            right: 16,
                            top: 0,
                            bottom: 0,
                        }))
                        .desired_width(f32::INFINITY)
                        .min_size(min)
                        .font(FontId::proportional(17.0))
                        .text_color(TEXT_DARK)
                        .hint_text("Start typing...")
                        .lock_focus(true),
                );
            });
    }
}

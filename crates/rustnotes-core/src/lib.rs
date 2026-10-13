//! Rust Notes core — platform-agnostic app state machine, input translation and
//! UI. The UI is egui, painted by egui_glow onto a GLES3 context the shell owns.
//! Shells (android / ios) own the window, GL context and event translation, and
//! drive this code through [`CoreEvent`]s.

pub mod layout;
pub mod notes;
mod screens;
pub mod util;

use egui::epaint::MarginF32;
use egui::{
    Event, FontData, FontDefinitions, FontFamily, Key, Modifiers, PointerButton, Pos2, RawInput,
    Rect, SafeAreaInsets, TouchDeviceId, TouchId, TouchPhase, Vec2, ViewportId, ViewportIdMap,
    ViewportInfo,
};
use egui_glow::{Painter, ShaderVersion};
use layout::{Metrics, Palette};
use notes::{Note, NotesDb};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Fonts load at runtime from the device so the bundle carries no font file.
const FONT_CANDIDATES: &[&str] = &[
    "/system/fonts/Roboto-Regular.ttf",    // Android
    "/system/fonts/NotoSans-Regular.ttf",  // Android fallback
    "/System/Library/Fonts/SFNS.ttf",      // iOS: San Francisco
    "/System/Library/Fonts/Geneva.ttf",    // iOS fallback
    "/System/Library/Fonts/DroidSans.ttf", // Android last resort
];

pub(crate) const EDITOR_ID: &str = "rustnotes.editor";
pub(crate) const DELETE_ARM_WINDOW: Duration = Duration::from_secs(2);

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
pub(crate) enum Screen {
    List,
    Editor,
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
    /// Effective pixels per point: OS density x text scale.
    ppp: f32,
    /// OS density in pixels per point (Android density/160, iOS screen scale).
    density_ppp: f32,
    /// User text scale (Android fontScale, iOS Dynamic Type); 1.0 = default.
    text_scale: f32,
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
        Metrics::DEFAULT.apply(&ctx);
        Palette::apply_all(&ctx);
        // Until a shell reports the OS appearance, `System` resolves to light.
        ctx.options_mut(|o| o.fallback_theme = egui::Theme::Light);
        ctx.all_styles_mut(|s| {
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
            density_ppp: 1.0,
            text_scale: 1.0,
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

    /// `pixels_per_point`: the OS scale (Android: configuration density / 160,
    /// iOS: UIScreen scale). `None` falls back to a width-based guess so
    /// desktop builds and tests still lay out sensibly.
    pub fn set_viewport(&mut self, w: i32, h: i32, pixels_per_point: Option<f32>) {
        let size = [w.max(0) as u32, h.max(0) as u32];
        let density = pixels_per_point
            .map(|p| p.clamp(1.0, 4.5))
            .unwrap_or_else(|| (size[0] as f32 / 360.0).clamp(0.75, 3.5));
        if size != self.size_px || (density - self.density_ppp).abs() > 1e-3 {
            self.size_px = size;
            self.density_ppp = density;
            self.ppp = density * self.text_scale;
            self.dirty = true;
        }
    }

    /// User text scale from the OS (Android `fontScale`, iOS Dynamic Type).
    /// Scales every point-sized thing uniformly, bars and targets included.
    pub fn set_text_scale(&mut self, scale: f32) {
        let scale = scale.clamp(0.8, 2.0);
        if (scale - self.text_scale).abs() > 1e-3 {
            self.text_scale = scale;
            self.ppp = self.density_ppp * scale;
            self.dirty = true;
        }
    }

    /// OS dark/light appearance. winit reports no theme on mobile, so the
    /// shells feed it and egui's `System` preference resolves to it.
    pub fn set_appearance(&mut self, dark: bool) {
        let theme = if dark { egui::Theme::Dark } else { egui::Theme::Light };
        if self.ctx.options(|o| o.fallback_theme) != theme {
            self.ctx.options_mut(|o| o.fallback_theme = theme);
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

    /// Save the buffer to the database and refresh the list. An empty new note
    /// is dropped; emptying an existing note deletes it.
    fn persist_buffer(&mut self) {
        let empty = self.buffer.trim().is_empty();
        log::info!("editor: persist id={} empty={}", self.edit_id, empty);
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
    }

    fn open_editor(&mut self, id: i64, plat: &dyn Platform) {
        log::info!("editor: open id={id}");
        if self.screen == Screen::Editor {
            // Two-pane: switching notes saves the one being edited first.
            self.persist_buffer();
        }
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
        self.persist_buffer();
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
        let handled = self.handle_event_inner(ev, plat);
        if self.painter.is_none() {
            // No frame will drain these until a GL context is attached.
            self.events.clear();
        }
        handled
    }

    fn handle_event_inner(&mut self, ev: CoreEvent, plat: &dyn Platform) -> bool {
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
        let [r, g, b, _] = Palette::of(&ctx).bg.to_normalized_gamma_f32();
        painter.clear([w, h], [r, g, b, 1.0]);
        painter.paint_and_update_textures(
            [w, h],
            out.pixels_per_point,
            &primitives,
            &mut out.textures_delta,
        );
    }
}

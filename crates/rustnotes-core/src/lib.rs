//! Rust Notes core — the notes app itself: state, screens, storage and the
//! mobile layout layer. It is pure egui and knows nothing about windows, GL
//! or platforms; the `rustnotes` crate hosts it with eframe on Android, iOS
//! and desktop.
//!
//! Host contract (all a host needs to call):
//! - [`App::new`] once, with the data directory, the egui context and an
//!   optional platform-resolved system font.
//! - [`App::filter_input`] from `eframe::App::raw_input_hook`, every frame.
//! - [`App::on_back`] when the platform back key arrives.
//! - [`App::ui`] every frame.
//! - [`App::save_current`] when the app loses focus or exits.

pub mod layout;
pub mod notes;
mod screens;
pub mod util;

use egui::{Event, FontData, FontDefinitions, FontFamily, RawInput, TouchPhase, Vec2};
use layout::{Metrics, Nav, Palette};
use notes::{Note, NotesDb};
use screens::{Cmd, Swipe};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Fonts load at runtime from the device so the bundle carries no font file.
/// iOS devices resolve theirs through CoreText in the host instead: the
/// device has no readable font at these paths (the simulator masks it,
/// because its reads resolve against the macOS host file system).
const FONT_CANDIDATES: &[&str] = &[
    "/system/fonts/Roboto-Regular.ttf",                // Android
    "/system/fonts/NotoSans-Regular.ttf",              // Android fallback
    "/System/Library/Fonts/SFNS.ttf",                  // macOS desktop, iOS simulator
    "/System/Library/Fonts/Geneva.ttf",                // macOS fallback
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", // Linux desktop
];

pub(crate) const EDITOR_ID: &str = "rustnotes.editor";
pub(crate) const DELETE_ARM_WINDOW: Duration = Duration::from_secs(2);

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Screen {
    List,
    Editor,
}

/// Top-level destinations in the bottom navigation bar.
#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Tab {
    Notes,
    Settings,
}

/// An iOS-style edge swipe in progress (drives an interactive pop).
struct EdgeSwipe {
    start_x: f32,
    start_y: f32,
    last_x: f32,
    last_t: f64,
    /// Horizontal velocity in points per second.
    vel: f32,
    dragging: bool,
}

/// How far from the left edge, in points, an edge swipe may start.
const EDGE_ZONE: f32 = 20.0;
/// Movement, in points, before an edge touch becomes a swipe.
const EDGE_SLOP: f32 = 8.0;
/// Fraction of the width past which releasing commits the pop.
const EDGE_COMMIT_FRACTION: f32 = 0.35;
/// Release velocity, in points per second, that commits regardless of distance.
const EDGE_COMMIT_VELOCITY: f32 = 600.0;

pub struct App {
    db: NotesDb,
    nav: Nav<Screen>,
    tab: Tab,
    /// Navigation changes queued by screens while drawing.
    cmds: Vec<Cmd>,
    /// Single-pane layout slides between screens; two-pane does not.
    two_pane: bool,
    reduce_motion: bool,
    edge_swipe_enabled: bool,
    edge: Option<EdgeSwipe>,
    /// Drop the simulated pointer events of a touch the edge swipe claimed.
    swallow_pointer: bool,
    // Pull to refresh
    pull_raw: f32,
    pull_armed: bool,
    pull_active: bool,
    /// List scroll offset from the previous frame.
    list_offset: f32,
    // Swipe to delete
    swipe: Option<Swipe>,
    notes: Vec<Note>,
    // Editor state
    edit_id: i64, // 0 = new, unsaved
    buffer: String,
    focus_editor: bool,
    // List state
    reset_list_scroll: bool,
    armed_delete: Option<(usize, Instant)>,
    /// egui's clock (`RawInput::time`), latest seen.
    time: f64,
    font_source: Result<String, String>,
}

/// egui font definitions naming a single installed font "system", used for
/// every family. `source` is the diagnostic origin (path or platform API).
fn font_defs_from(bytes: Vec<u8>, source: &str) -> (FontDefinitions, String) {
    let mut defs = FontDefinitions::empty();
    defs.font_data
        .insert("system".to_owned(), Arc::new(FontData::from_owned(bytes)));
    defs.families
        .insert(FontFamily::Proportional, vec!["system".to_owned()]);
    defs.families
        .insert(FontFamily::Monospace, vec!["system".to_owned()]);
    (defs, source.to_owned())
}

/// First readable candidate wins. Every miss is logged, because a silent
/// fall-through is how the iOS 26 device shipped a black screen.
fn load_fonts_from(candidates: &[&str]) -> Result<(FontDefinitions, String), String> {
    let mut last_err = String::from("no font candidates");
    for path in candidates {
        match std::fs::read(path) {
            Ok(bytes) => return Ok(font_defs_from(bytes, path)),
            Err(e) => {
                log::info!("font candidate miss: {path}: {e}");
                last_err = format!("cannot read {path}: {e}");
            }
        }
    }
    Err(last_err)
}

/// Install the UI font: the platform-resolved one if the host has it
/// (iOS: CoreText), else the first readable candidate path. Without either,
/// egui's bundled default fonts stay in place, so the UI still renders.
fn install_fonts(
    ctx: &egui::Context,
    system_font: Option<(Vec<u8>, String)>,
) -> Result<String, String> {
    let loaded = match system_font {
        Some((bytes, src)) => {
            log::info!("system font from {src}");
            Ok(font_defs_from(bytes, &src))
        }
        None => load_fonts_from(FONT_CANDIDATES),
    };
    match loaded {
        Ok((defs, src)) => {
            ctx.set_fonts(defs);
            Ok(src)
        }
        Err(e) => {
            log::error!("font load failed, using egui's bundled fonts: {e}");
            Err(e)
        }
    }
}

impl App {
    /// Open the notes database in `data_dir` and set up the egui context:
    /// metrics, palettes, fonts.
    pub fn new(
        data_dir: PathBuf,
        ctx: &egui::Context,
        system_font: Option<(Vec<u8>, String)>,
    ) -> App {
        let db = NotesDb::open(&data_dir).expect("open notes database");
        let notes = db.list();
        log::info!("loaded {} notes from {}", notes.len(), data_dir.display());

        Metrics::DEFAULT.apply(ctx);
        Palette::apply_all(ctx);
        // Until the host reports the OS appearance, `System` resolves to light.
        ctx.options_mut(|o| o.fallback_theme = egui::Theme::Light);
        ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = Vec2::ZERO;
            s.interaction.selectable_labels = false;
        });
        let font_source = install_fonts(ctx, system_font);

        App {
            db,
            nav: Nav::new(Screen::List),
            tab: Tab::Notes,
            cmds: Vec::new(),
            two_pane: false,
            reduce_motion: false,
            edge_swipe_enabled: false,
            edge: None,
            swallow_pointer: false,
            pull_raw: 0.0,
            pull_armed: false,
            pull_active: false,
            list_offset: 0.0,
            swipe: None,
            notes,
            edit_id: 0,
            buffer: String::new(),
            focus_editor: false,
            reset_list_scroll: false,
            armed_delete: None,
            time: 0.0,
            font_source,
        }
    }

    /// Where the UI font came from, for logs and diagnostics.
    pub fn font_source(&self) -> &str {
        match &self.font_source {
            Ok(s) => s,
            Err(_) => "<egui default>",
        }
    }

    pub(crate) fn screen(&self) -> Screen {
        *self.nav.top()
    }

    /// egui's clock; navigation animations run on it.
    fn now(&self) -> f64 {
        self.time
    }

    fn animate_nav(&self) -> bool {
        !self.two_pane && !self.reduce_motion
    }

    /// Honour the OS "reduce motion" preference: screens change without sliding.
    pub fn set_reduce_motion(&mut self, reduce: bool) {
        self.reduce_motion = reduce;
    }

    /// iOS only: a swipe from the left edge pops the editor interactively.
    /// Android keeps the system back gesture, which arrives as the back key.
    pub fn set_edge_swipe_back(&mut self, enabled: bool) {
        self.edge_swipe_enabled = enabled;
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

    fn open_editor(&mut self, id: i64) {
        log::info!("editor: open id={id}");
        let was_editor = self.screen() == Screen::Editor;
        if was_editor {
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
        if !was_editor {
            let (now, animate) = (self.now(), self.animate_nav());
            self.nav.push(Screen::Editor, now, animate);
        }
        // Focusing the editor makes egui-winit show the soft keyboard.
        self.focus_editor = true;
    }

    fn close_editor(&mut self) {
        self.persist_buffer();
        self.reset_list_scroll = true;
        let (now, animate) = (self.now(), self.animate_nav());
        self.nav.pop(now, animate);
        // The editor is no longer drawn, so egui drops its focus next frame
        // and egui-winit hides the keyboard.
    }

    /// Save without leaving the editor (the host calls this on pause/exit).
    pub fn save_current(&mut self) {
        if self.screen() != Screen::Editor || self.buffer.trim().is_empty() {
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

    /// The platform back key (Android's back button or gesture). Returns false
    /// when there is nothing to go back to, so the host can leave the app.
    pub fn on_back(&mut self) -> bool {
        if self.tab == Tab::Settings {
            self.tab = Tab::Notes;
            return true;
        }
        if self.nav.is_dragging() {
            return true;
        }
        if self.screen() == Screen::Editor {
            // Single-pane: back to the list. Two-pane: closes the detail.
            self.close_editor();
            return true;
        }
        false
    }

    fn edge_swipe_eligible(&self) -> bool {
        self.edge_swipe_enabled
            && self.tab == Tab::Notes
            && !self.two_pane
            && self.screen() == Screen::Editor
            && self.nav.can_pop()
            && !self.nav.is_animating()
    }

    /// Call from `raw_input_hook` every frame. Records egui's clock and claims
    /// touches that start at the left edge for the interactive back swipe:
    /// those touches (and the pointer events egui-winit simulates for them)
    /// never reach the widgets. A claimed touch that never moves is replayed
    /// as a plain tap, so the back button under the edge still works.
    pub fn filter_input(&mut self, raw: &mut RawInput) {
        if let Some(t) = raw.time {
            self.time = t;
        }
        if !self.edge_swipe_enabled {
            return;
        }
        let width = raw.screen_rect.map_or(400.0, |r| r.width()).max(1.0);
        let events = std::mem::take(&mut raw.events);
        for ev in events {
            if let Event::Touch { phase, pos, .. } = &ev {
                match phase {
                    TouchPhase::Start => {
                        self.swallow_pointer = false;
                        if self.edge_swipe_eligible() && pos.x <= EDGE_ZONE {
                            self.edge = Some(EdgeSwipe {
                                start_x: pos.x,
                                start_y: pos.y,
                                last_x: pos.x,
                                last_t: self.time,
                                vel: 0.0,
                                dragging: false,
                            });
                            self.swallow_pointer = true;
                            continue;
                        }
                    }
                    TouchPhase::Move => {
                        if let Some(mut e) = self.edge.take() {
                            let dx = pos.x - e.start_x;
                            if !e.dragging && dx > EDGE_SLOP {
                                e.dragging = self.nav.begin_drag();
                            }
                            if e.dragging {
                                e.vel =
                                    (pos.x - e.last_x) / ((self.time - e.last_t) as f32).max(1e-3);
                                self.nav.drag_to(dx / width);
                            }
                            e.last_x = pos.x;
                            e.last_t = self.time;
                            self.edge = Some(e);
                            continue;
                        }
                    }
                    TouchPhase::End | TouchPhase::Cancel => {
                        if let Some(e) = self.edge.take() {
                            if e.dragging {
                                let commit = *phase == TouchPhase::End
                                    && ((pos.x - e.start_x) / width > EDGE_COMMIT_FRACTION
                                        || e.vel > EDGE_COMMIT_VELOCITY);
                                if commit {
                                    self.persist_buffer();
                                    self.reset_list_scroll = true;
                                }
                                self.nav.end_drag(commit, self.time);
                            } else if *phase == TouchPhase::End {
                                // Never became a swipe: replay it as a tap.
                                let p = egui::pos2(e.start_x, e.start_y);
                                raw.events.push(Event::PointerMoved(p));
                                for pressed in [true, false] {
                                    raw.events.push(Event::PointerButton {
                                        pos: p,
                                        button: egui::PointerButton::Primary,
                                        pressed,
                                        modifiers: egui::Modifiers::NONE,
                                    });
                                }
                                raw.events.push(Event::PointerGone);
                            }
                            continue;
                        }
                    }
                }
            }
            if self.swallow_pointer
                && matches!(
                    ev,
                    Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone
                )
            {
                continue;
            }
            raw.events.push(ev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("rn-core-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The seam the iOS 26 black screen came through: the loader must fall
    /// through unreadable candidates to a readable one.
    #[test]
    fn font_fallback_picks_first_readable_candidate() {
        let dir = temp("fonts");
        let missing = dir.join("missing.ttf");
        let present = dir.join("present.ttf");
        std::fs::write(&present, b"fake-font-bytes").unwrap();
        let (m, p) = (missing.to_str().unwrap(), present.to_str().unwrap());

        let (defs, src) = load_fonts_from(&[m, p]).expect("second candidate loads");
        assert_eq!(src, p);
        assert!(defs.font_data.contains_key("system"));
        let _ = std::fs::remove_file(&present);
    }

    #[test]
    fn font_fallback_reports_last_error_when_all_missing() {
        let err = load_fonts_from(&["/nonexistent/a.ttf", "/nonexistent/b.ttf"]).unwrap_err();
        assert!(
            err.contains("b.ttf"),
            "error should name the last candidate"
        );
    }

    /// A host-resolved font (CoreText on iOS) wins over the candidate lookup.
    #[test]
    fn system_font_overrides_candidate_lookup() {
        let ctx = egui::Context::default();
        let app = App::new(
            temp("app").join("data"),
            &ctx,
            Some((b"bytes".to_vec(), "coretext://SF".to_owned())),
        );
        assert_eq!(app.font_source(), "coretext://SF");
    }

    fn touch(phase: TouchPhase, x: f32) -> Event {
        Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(0),
            phase,
            pos: egui::pos2(x, 300.0),
            force: None,
        }
    }

    fn frame(events: Vec<Event>, t: f64) -> RawInput {
        RawInput {
            time: Some(t),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 800.0),
            )),
            events,
            ..Default::default()
        }
    }

    /// An edge swipe past the commit fraction pops the editor, and egui never
    /// sees the touch or its simulated pointer events.
    #[test]
    fn edge_swipe_pops_the_editor_and_hides_the_touch() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("edge").join("data"), &ctx, None);
        app.set_edge_swipe_back(true);
        app.open_editor(0);
        app.nav.settle();
        assert!(app.screen() == Screen::Editor);

        let mut f = frame(
            vec![
                touch(TouchPhase::Start, 5.0),
                Event::PointerMoved(egui::pos2(5.0, 300.0)),
            ],
            1.0,
        );
        app.filter_input(&mut f);
        assert!(f.events.is_empty(), "claimed touch must not reach egui");

        let mut f = frame(vec![touch(TouchPhase::Move, 250.0)], 1.2);
        app.filter_input(&mut f);
        let mut f = frame(vec![touch(TouchPhase::End, 250.0), Event::PointerGone], 1.3);
        app.filter_input(&mut f);
        assert!(f.events.is_empty());
        assert!(
            app.screen() == Screen::List,
            "past 35% of the width commits"
        );
    }

    /// A touch that starts in the edge zone but never moves is a tap.
    #[test]
    fn edge_tap_is_replayed_as_a_click() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("tap").join("data"), &ctx, None);
        app.set_edge_swipe_back(true);
        app.open_editor(0);
        app.nav.settle();

        let mut f = frame(vec![touch(TouchPhase::Start, 10.0)], 1.0);
        app.filter_input(&mut f);
        let mut f = frame(vec![touch(TouchPhase::End, 10.0), Event::PointerGone], 1.1);
        app.filter_input(&mut f);
        let presses = f
            .events
            .iter()
            .filter(|e| matches!(e, Event::PointerButton { .. }))
            .count();
        assert_eq!(presses, 2, "replayed as press + release");
        assert!(app.screen() == Screen::Editor);
    }

    #[test]
    fn back_closes_the_editor_then_reports_the_root() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("back").join("data"), &ctx, None);
        app.open_editor(0);
        assert!(app.on_back());
        assert!(app.screen() == Screen::List);
        assert!(!app.on_back(), "nothing left: the host may leave the app");
    }
}

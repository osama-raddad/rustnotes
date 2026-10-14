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

mod editor_touch;
pub mod layout;
pub mod notes;
mod screens;
pub mod util;

use editor_touch::EditorTouch;
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
    /// Which finger claimed the swipe; only this touch drives or ends it
    /// (LOW-3: a second finger must not commit the pop).
    touch: egui::TouchId,
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

/// How long after the last keystroke the autosave fires (HIGH-2).
const AUTOSAVE_DELAY: Duration = Duration::from_millis(1500);

/// Prefixes that classify `App::db_error` entries so a later success of the
/// same operation clears exactly its own error (HIGH-1b).
const DB_LIST_ERR: &str = "couldn't load notes";
const DB_SAVE_ERR: &str = "save failed";
const DB_DELETE_ERR: &str = "delete failed";

/// What the autosave logic should do this frame. Pure data so the decision
/// (`debounce`) is table-testable without a live egui context.
#[derive(PartialEq, Debug)]
enum DebounceAction {
    /// Nothing to do (not dirty, or no edit recorded yet).
    None,
    /// Dirty, but the quiet period hasn't elapsed; repaint after this long
    /// so the save fires without user input.
    Wait(Duration),
    /// The quiet period elapsed — save now.
    Save,
}

/// Debounced save-on-edit decision (HIGH-2): fire `AUTOSAVE_DELAY` after the
/// last change to a dirty buffer.
fn debounce(last_edit: Option<Instant>, dirty: bool, now: Instant) -> DebounceAction {
    if !dirty {
        return DebounceAction::None;
    }
    match last_edit {
        None => DebounceAction::None,
        Some(t) => {
            let elapsed = now.saturating_duration_since(t);
            if elapsed >= AUTOSAVE_DELAY {
                DebounceAction::Save
            } else {
                DebounceAction::Wait(AUTOSAVE_DELAY - elapsed)
            }
        }
    }
}

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
    /// Touch intent in the editor: a swipe scrolls, a tap places the caret,
    /// a long press selects (see `editor_touch`).
    editor_touch: EditorTouch,
    /// Where the editor's scroll area was drawn, set by `editor_drawn` each
    /// frame the editor draws and taken by the next `filter_input`, so it
    /// is `None` there whenever the editor did not draw last frame.
    editor_rect: Option<egui::Rect>,
    // Pull to refresh
    pull_raw: f32,
    pull_armed: bool,
    pull_active: bool,
    /// List scroll offset from the previous frame.
    list_offset: f32,
    // Swipe to delete
    swipe: Option<Swipe>,
    notes: Vec<Note>,
    /// Formatted row dates, parallel to `notes` — built when the list loads
    /// or the UTC offset changes, not per frame (MED-5).
    row_dates: Vec<String>,
    // Editor state
    edit_id: i64, // 0 = new, unsaved
    buffer: String,
    /// The note body exactly as last loaded or saved; `buffer` differing
    /// from it means unsaved edits (LOW-2's dirty check, HIGH-2's flag).
    loaded_body: String,
    /// The buffer as of the previous frame, to detect edits without hashing.
    last_seen_buffer: String,
    /// When the buffer last changed; drives the autosave debounce (HIGH-2).
    last_edit: Option<Instant>,
    focus_editor: bool,
    // List state
    reset_list_scroll: bool,
    /// Delete confirmation, keyed by note id (MED-1: an index goes stale
    /// when the list reorders between the two taps).
    armed_delete: Option<(i64, Instant)>,
    /// Two clocks, deliberately: egui's `time` (`RawInput::time`, seconds
    /// since the loop started) drives the nav animation because it stays in
    /// step with the frames egui paints, while `Instant` drives the
    /// delete-arm window and autosave debounce, which must keep running
    /// (and fire) with no input at all. Don't mix them (NIT-3).
    time: f64,
    font_source: Result<String, String>,
    /// The latest database failure, if any: drives the editor's "save
    /// failed" indicator, the list's error state and the Settings
    /// diagnostics row (HIGH-1b). Cleared when the same operation later
    /// succeeds.
    db_error: Option<String>,
    /// Local zone offset east of UTC, pushed by the host (MED-2); note
    /// dates render shifted by this.
    utc_offset_secs: i32,
    /// Cached settings diagnostics, rebuilt only when a value changes
    /// (NIT-2).
    diag: Option<(screens::DiagKey, Vec<(&'static str, String)>)>,
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
        // HIGH-3: a corrupt or unwritable database must degrade to an
        // in-memory fallback instead of panicking — on mobile that is a
        // startup crash loop until the user clears app data.
        let (db, mut db_error) = match NotesDb::open(&data_dir) {
            Ok(db) => (db, None),
            Err(e) => {
                log::error!("open notes database failed: {e}; falling back to in-memory storage");
                let msg = format!("notes.db unavailable ({e}) — in-memory, notes will not persist");
                match NotesDb::open_in_memory() {
                    Ok(db) => (db, Some(msg)),
                    // An in-memory open has no filesystem dependency, so this
                    // is not realistically reachable.
                    Err(e2) => panic!("open in-memory notes database: {e2}"),
                }
            }
        };
        let (notes, list_err) = match db.list() {
            Ok(notes) => (notes, None),
            Err(e) => {
                log::error!("initial note list failed: {e}");
                (Vec::new(), Some(format!("{DB_LIST_ERR}: {e}")))
            }
        };
        if db_error.is_none() {
            db_error = list_err;
        }
        log::info!("loaded {} notes from {}", notes.len(), data_dir.display());
        let utc_offset_secs = 0;
        let row_dates = notes
            .iter()
            .map(|n| util::format_note_date(n.updated, utc_offset_secs))
            .collect();

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
            editor_touch: EditorTouch::default(),
            editor_rect: None,
            pull_raw: 0.0,
            pull_armed: false,
            pull_active: false,
            list_offset: 0.0,
            swipe: None,
            notes,
            row_dates,
            edit_id: 0,
            buffer: String::new(),
            loaded_body: String::new(),
            last_seen_buffer: String::new(),
            last_edit: None,
            focus_editor: false,
            reset_list_scroll: false,
            armed_delete: None,
            time: 0.0,
            font_source,
            db_error,
            utc_offset_secs,
            diag: None,
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

    /// Render note dates in the local zone: `offset_secs` east of UTC, as
    /// polled from the OS by the host (core is pure egui and cannot read
    /// the zone itself — MED-2). Clamped to ±24 h.
    pub fn set_utc_offset_secs(&mut self, offset_secs: i32) {
        self.utc_offset_secs = offset_secs.clamp(-86_400, 86_400);
        self.recompute_dates();
    }

    /// Rebuild the cached row dates (MED-5) from `notes` and the offset.
    fn recompute_dates(&mut self) {
        let offset = self.utc_offset_secs;
        self.row_dates = self
            .notes
            .iter()
            .map(|n| util::format_note_date(n.updated, offset))
            .collect();
    }

    /// Reload the list from the database. Every re-list clears the armed
    /// delete — rows may have reordered, so the arm must not survive onto a
    /// different note (MED-1) — and rebuilds the cached dates.
    fn refresh_notes(&mut self) {
        match self.db.list() {
            Ok(rows) => {
                self.notes = rows;
                self.armed_delete = None;
                self.recompute_dates();
                self.clear_error_if(DB_LIST_ERR);
            }
            Err(e) => {
                log::error!("list notes failed: {e}");
                self.db_error = Some(format!("{DB_LIST_ERR}: {e}"));
            }
        }
    }

    /// Clear `db_error` when it came from the operation that just succeeded.
    fn clear_error_if(&mut self, prefix: &str) {
        if matches!(self.db_error.as_deref(), Some(e) if e.starts_with(prefix)) {
            self.db_error = None;
        }
    }

    // ---- editor ----------------------------------------------------------

    /// Save the buffer to the database and refresh the list. An empty new note
    /// is dropped; emptying an existing note deletes it. Failures are
    /// surfaced through `db_error` instead of being swallowed (HIGH-1).
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
                match self.db.save(&mut n) {
                    Ok(()) => {
                        // LOW-5: track the insert so a later save UPDATEs
                        // instead of duplicating the note.
                        self.edit_id = n.id;
                        self.loaded_body.clone_from(&self.buffer);
                        self.clear_error_if(DB_SAVE_ERR);
                    }
                    Err(e) => self.db_error = Some(format!("{DB_SAVE_ERR}: {e}")),
                }
            }
        } else if empty {
            match self.db.delete(self.edit_id) {
                Ok(()) => self.clear_error_if(DB_DELETE_ERR),
                Err(e) => self.db_error = Some(format!("{DB_DELETE_ERR}: {e}")),
            }
        } else if self.buffer != self.loaded_body {
            // LOW-2: a clean buffer is not written, so closing without
            // editing no longer bumps `updated` and reorders the list.
            let mut n = Note {
                id: self.edit_id,
                body: self.buffer.clone(),
                updated: 0,
            };
            match self.db.save(&mut n) {
                Ok(()) => {
                    self.loaded_body.clone_from(&self.buffer);
                    self.clear_error_if(DB_SAVE_ERR);
                }
                Err(e) => self.db_error = Some(format!("{DB_SAVE_ERR}: {e}")),
            }
        }
        self.last_edit = None;
        self.refresh_notes();
    }

    fn open_editor(&mut self, id: i64) {
        log::info!("editor: open id={id}");
        let was_editor = self.screen() == Screen::Editor;
        if was_editor {
            // Two-pane: switching notes saves the one being edited first.
            self.persist_buffer();
        }
        self.edit_id = id;
        // The list only carries a body prefix (MED-5), so fetch the full
        // note from the database.
        self.buffer = if id == 0 {
            String::new()
        } else {
            match self.db.get(id) {
                Ok(Some(n)) => n.body,
                Ok(None) => String::new(),
                Err(e) => {
                    self.db_error = Some(format!("{DB_LIST_ERR}: {e}"));
                    String::new()
                }
            }
        };
        self.loaded_body.clone_from(&self.buffer);
        self.last_seen_buffer.clone_from(&self.buffer);
        self.last_edit = None;
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
    /// A buffer that matches what was loaded is skipped so an unedited note
    /// is neither written nor bumped (LOW-2).
    pub fn save_current(&mut self) {
        if self.screen() != Screen::Editor
            || self.buffer.trim().is_empty()
            || self.buffer == self.loaded_body
        {
            return;
        }
        let mut n = Note {
            id: self.edit_id,
            body: self.buffer.clone(),
            updated: 0,
        };
        match self.db.save(&mut n) {
            Ok(()) => {
                self.edit_id = n.id;
                self.loaded_body.clone_from(&self.buffer);
                self.clear_error_if(DB_SAVE_ERR);
            }
            Err(e) => self.db_error = Some(format!("{DB_SAVE_ERR}: {e}")),
        }
        self.last_edit = None;
    }

    /// True while the editor holds unsaved changes.
    fn dirty(&self) -> bool {
        self.screen() == Screen::Editor
            && !self.buffer.trim().is_empty()
            && self.buffer != self.loaded_body
    }

    /// The debounced save (HIGH-2): the editor stays open, the buffer is
    /// written and the failure state is surfaced. A failed attempt retries
    /// after another quiet period, so a transient error (disk full, then
    /// freed) recovers on its own.
    fn autosave(&mut self) {
        if !self.dirty() {
            self.last_edit = None;
            return;
        }
        let mut n = Note {
            id: self.edit_id,
            body: self.buffer.clone(),
            updated: 0,
        };
        match self.db.save(&mut n) {
            Ok(()) => {
                self.edit_id = n.id;
                self.loaded_body.clone_from(&self.buffer);
                self.last_edit = None;
                self.clear_error_if(DB_SAVE_ERR);
            }
            Err(e) => {
                self.db_error = Some(format!("{DB_SAVE_ERR}: {e}"));
                self.last_edit = Some(Instant::now());
            }
        }
        self.refresh_notes();
    }

    // ---- input ------------------------------------------------------------

    /// The platform back key (Android's back button or gesture). Returns false
    /// when there is nothing to go back to, so the host can leave the app.
    pub fn on_back(&mut self) -> bool {
        if self.tab == Tab::Settings {
            self.tab = Tab::Notes;
            return true;
        }
        // LOW-1: while a push/pop animation plays or a drag is in flight,
        // swallow the press — a second back during the pop animation would
        // otherwise report "root" and quit the app mid-slide.
        if self.nav.is_dragging() || self.nav.is_animating() {
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

    /// The editor may own a touch: it is the notes tab's current route and
    /// nothing slides it (`is_animating` covers an edge drag too).
    fn editor_owns_touches(&self) -> bool {
        self.tab == Tab::Notes && self.screen() == Screen::Editor && !self.nav.is_animating()
    }

    /// The editor calls this each frame it draws, right before its scroll
    /// area: the touch filter learns where that area is (a touch must start
    /// there to be the editor's, and the wheel must land there), and a touch
    /// waiting out the long-press window is woken when the window ends,
    /// whether the finger moves or not.
    pub(crate) fn editor_drawn(&mut self, ui: &egui::Ui) {
        self.editor_rect = Some(ui.available_rect_before_wrap().intersect(ui.clip_rect()));
        if let Some(wait) = self.editor_touch.wake_after(self.time) {
            ui.ctx().request_repaint_after(wait);
        }
    }

    /// Call from `raw_input_hook` every frame. Records egui's clock and runs
    /// the two touch filters, the edge swipe first:
    ///
    /// - The edge swipe (iOS) claims touches that start at the left edge for
    ///   the interactive back swipe: those touches (and the pointer events
    ///   egui-winit simulates for them) never reach the widgets. A claimed
    ///   touch that never moves is replayed as a plain tap, so the back
    ///   button under the edge still works.
    /// - The editor's touch intent (every platform, `editor_touch`): a touch
    ///   on the editor scrolls it when it moves, places the caret when it
    ///   taps, and starts a selection when held still. It never sees a touch
    ///   the edge swipe claimed.
    ///
    /// TODO(verify-on-device): multi-touch interleavings (second finger
    /// during a claim, OS-cancelled touches) can't be produced headlessly —
    /// check the TouchId gating on hardware.
    pub fn filter_input(&mut self, raw: &mut RawInput) {
        if let Some(t) = raw.time {
            self.time = t;
        }
        let drawn = self.editor_rect.take();
        if self.edge_swipe_enabled {
            self.filter_edge_swipe(raw);
        }
        // Checked after the edge swipe, which may just have started a drag
        // or popped the editor.
        let editor = drawn.filter(|_| self.editor_owns_touches());
        // While the edge swipe holds a finger, a new finger is not the one
        // egui-winit mirrors as the pointer: the editor must not claim it.
        let claim = self.edge.is_none();
        let events = std::mem::take(&mut raw.events);
        raw.events = self.editor_touch.filter(events, self.time, editor, claim);
    }

    /// The edge swipe's part of `filter_input`.
    fn filter_edge_swipe(&mut self, raw: &mut RawInput) {
        let width = raw.screen_rect.map_or(400.0, |r| r.width()).max(1.0);
        let events = std::mem::take(&mut raw.events);
        for ev in events {
            if let Event::Touch { phase, pos, id, .. } = &ev {
                match phase {
                    TouchPhase::Start => {
                        self.swallow_pointer = false;
                        if self.edge_swipe_eligible() && pos.x <= EDGE_ZONE {
                            self.edge = Some(EdgeSwipe {
                                touch: *id,
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
                        // Only the finger that claimed the swipe drives it
                        // (LOW-3); any other touch falls through to egui.
                        let claimed = matches!(self.edge.as_ref(), Some(e) if e.touch == *id);
                        if claimed {
                            if let Some(e) = self.edge.as_mut() {
                                let dx = pos.x - e.start_x;
                                if !e.dragging && dx > EDGE_SLOP {
                                    e.dragging = self.nav.begin_drag();
                                }
                                if e.dragging {
                                    e.vel = (pos.x - e.last_x)
                                        / ((self.time - e.last_t) as f32).max(1e-3);
                                    self.nav.drag_to(dx / width);
                                }
                                e.last_x = pos.x;
                                e.last_t = self.time;
                                continue;
                            }
                        }
                    }
                    TouchPhase::End | TouchPhase::Cancel => {
                        if self.edge.as_ref().is_some_and(|e| e.touch == *id) {
                            if let Some(e) = self.edge.take() {
                                // Keep eating this touch's synthetic pointer
                                // stream (its trailing PointerGone too);
                                // the next TouchPhase::Start resets the
                                // flag (see above).
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
        touch_at(0, phase, egui::pos2(x, 300.0))
    }

    /// The one `Touch` builder the touch helpers share.
    fn touch_at(id: u64, phase: TouchPhase, pos: egui::Pos2) -> Event {
        Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(id),
            phase,
            pos,
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

    fn finger(phase: TouchPhase, x: f32, id: u64) -> Event {
        touch_at(id, phase, egui::pos2(x, 300.0))
    }

    /// LOW-3: a second finger sweeping right during a claimed edge swipe
    /// must not drive or commit the pop; only the claiming touch ends it.
    #[test]
    fn second_finger_cannot_commit_the_edge_swipe() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("twofinger").join("data"), &ctx, None);
        app.set_edge_swipe_back(true);
        app.open_editor(0);
        app.nav.settle();

        // Finger 0 claims the edge; finger 1 starts elsewhere.
        let mut f = frame(vec![touch(TouchPhase::Start, 5.0)], 1.0);
        app.filter_input(&mut f);
        assert!(f.events.is_empty(), "claim swallows the touch");

        // Finger 1 sweeps past the commit fraction: it must pass through
        // untouched and not move the claimed swipe.
        let mut f = frame(vec![finger(TouchPhase::Start, 100.0, 1)], 1.05);
        app.filter_input(&mut f);
        assert_eq!(f.events.len(), 1, "the other finger reaches egui");
        let mut f = frame(vec![finger(TouchPhase::Move, 300.0, 1)], 1.1);
        app.filter_input(&mut f);
        assert_eq!(f.events.len(), 1, "its moves reach egui too");

        // The claimed finger releases near its start: no commit, and the
        // stationary claim replays as a tap.
        let mut f = frame(vec![touch(TouchPhase::End, 10.0)], 1.2);
        app.filter_input(&mut f);
        let presses = f
            .events
            .iter()
            .filter(|e| matches!(e, Event::PointerButton { .. }))
            .count();
        assert_eq!(presses, 2, "claim ends as a replayed tap");
        assert!(
            app.screen() == Screen::Editor,
            "the second finger did not pop"
        );
    }

    #[test]
    fn back_closes_the_editor_then_reports_the_root() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("back").join("data"), &ctx, None);
        app.open_editor(0);
        app.nav.settle();
        assert!(app.on_back());
        app.nav.settle();
        assert!(app.screen() == Screen::List);
        assert!(!app.on_back(), "nothing left: the host may leave the app");
    }

    /// LOW-1: a second back while the pop is still animating must be
    /// swallowed, not reported as "root" (which quits the app mid-slide).
    #[test]
    fn back_during_the_pop_animation_is_swallowed() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("backanim").join("data"), &ctx, None);
        app.open_editor(0);
        app.nav.settle();
        assert!(app.on_back(), "starts the pop");
        assert!(
            app.on_back(),
            "still animating — swallow instead of exiting"
        );
        app.nav.settle();
        assert!(!app.on_back(), "settled: the host may leave the app");
    }

    /// HIGH-2's debounce decision, as a table: pure, no egui context.
    #[test]
    fn debounce_waits_then_saves() {
        let t0 = Instant::now();
        assert_eq!(debounce(None, true, t0), DebounceAction::None);
        assert_eq!(debounce(Some(t0), false, t0), DebounceAction::None);
        match debounce(Some(t0), true, t0 + Duration::from_millis(100)) {
            DebounceAction::Wait(left) => {
                assert!(left > Duration::ZERO && left < AUTOSAVE_DELAY);
            }
            other => panic!("expected Wait, got {other:?}"),
        }
        assert_eq!(
            debounce(Some(t0), true, t0 + AUTOSAVE_DELAY),
            DebounceAction::Save
        );
        assert_eq!(
            debounce(Some(t0), true, t0 + AUTOSAVE_DELAY + Duration::from_secs(5)),
            DebounceAction::Save
        );
    }

    /// HIGH-2 wiring: a frame that runs more than the quiet period after the
    /// last edit saves the note without leaving the editor.
    #[test]
    fn autosave_persists_after_the_quiet_period() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("autosave").join("data"), &ctx, None);
        app.open_editor(0);
        app.nav.settle();
        app.buffer = "typed on the train".to_owned();
        // Pretend the change was detected two seconds ago.
        app.last_seen_buffer = app.buffer.clone();
        app.last_edit = Some(Instant::now() - Duration::from_secs(2));

        ctx.run_ui(Default::default(), |root| {
            // eframe hands App::ui an edge-to-edge root; run_ui's root is
            // the same shape, unlike a margined CentralPanel.
            app.ui(root);
        })
        .drop_without_applying_deltas();

        assert!(app.last_edit.is_none(), "the save consumed the edit");
        assert_eq!(app.loaded_body, "typed on the train");
        assert_eq!(app.db.list().unwrap().len(), 1);
        assert!(app.db_error.is_none());
    }

    /// HIGH-3: a corrupt database degrades to in-memory storage with the
    /// failure surfaced, instead of panicking into a startup crash loop.
    #[test]
    fn corrupt_database_degrades_to_memory_with_an_error_state() {
        let dir = temp("corrupt");
        std::fs::write(dir.join("notes.db"), b"definitely not a sqlite database").unwrap();
        let ctx = egui::Context::default();
        let mut app = App::new(dir, &ctx, None);
        assert!(
            app.db_error.is_some(),
            "open failure is surfaced, not panicked"
        );
        // The app still works against the fallback: edit and persist.
        app.open_editor(0);
        app.buffer = "recovered".to_owned();
        app.persist_buffer();
        assert_ne!(app.edit_id, 0);
        assert_eq!(app.db.list().unwrap().len(), 1);
        // The open failure is permanent — it must not be cleared by a
        // successful in-memory write.
        assert!(app.db_error.is_some());
    }

    /// MED-1: the delete arm is keyed by note id and any re-list clears it.
    #[test]
    fn refresh_clears_the_armed_delete() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("armed").join("data"), &ctx, None);
        app.armed_delete = Some((42, Instant::now()));
        app.refresh_notes();
        assert!(app.armed_delete.is_none(), "a re-list invalidates the arm");
    }

    /// LOW-2: `save_current` skips a buffer that matches what was loaded, so
    /// an unedited note is not written (or timestamp-bumped) on focus loss.
    #[test]
    fn save_current_skips_a_clean_buffer() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("clean").join("data"), &ctx, None);
        app.open_editor(0);
        app.nav.settle();
        app.buffer = "already saved".to_owned();
        app.loaded_body = "already saved".to_owned();
        app.save_current();
        assert_eq!(app.edit_id, 0, "clean buffer must not insert");
        assert!(app.db.list().unwrap().is_empty());
        // A real change does save.
        app.buffer = "edited".to_owned();
        app.save_current();
        assert_ne!(app.edit_id, 0);
        assert_eq!(app.db.list().unwrap().len(), 1);
    }

    /// LOW-5: after inserting a new note, the editor tracks its id so a
    /// later save UPDATEs instead of duplicating the INSERT.
    #[test]
    fn persist_assigns_the_new_note_id() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("low5").join("data"), &ctx, None);
        app.open_editor(0);
        app.buffer = "hello".to_owned();
        app.persist_buffer();
        assert_ne!(app.edit_id, 0, "edit_id tracks the insert");
        assert_eq!(app.notes.len(), 1);
    }

    /// MED-5: the list rows carry only a body prefix; opening the editor
    /// fetches the full body from the database.
    #[test]
    fn open_editor_loads_the_full_body() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("fullbody").join("data"), &ctx, None);
        app.open_editor(0);
        app.nav.settle();
        app.buffer = format!("{}\nand a second line", "y".repeat(200));
        app.persist_buffer();
        let id = app.edit_id;

        let listed = app.db.list().unwrap();
        assert!(listed[0].body.len() <= 120, "the list keeps only a preview");

        app.open_editor(id);
        assert_eq!(app.buffer.len(), 200 + "\nand a second line".len());
    }

    /// MED-2: the host-pushed UTC offset is clamped to ±24 h and shifts the
    /// cached row dates.
    #[test]
    fn utc_offset_is_clamped_and_feeds_row_dates() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("tz").join("data"), &ctx, None);
        app.set_utc_offset_secs(1_000_000);
        assert_eq!(app.utc_offset_secs, 86_400, "clamped to +24 h");
        app.set_utc_offset_secs(-9_999_999);
        assert_eq!(app.utc_offset_secs, -86_400, "clamped to -24 h");

        app.open_editor(0);
        app.buffer = "tz".to_owned();
        app.persist_buffer();
        app.set_utc_offset_secs(0);
        let before = app.row_dates.clone();
        // One hour east always moves the rendered HH:MM of a fresh note.
        app.set_utc_offset_secs(3600);
        assert_ne!(app.row_dates, before, "row dates shift with the zone");
        assert_eq!(app.row_dates.len(), app.notes.len());
    }

    // ---- grafted from the parallel fix branch (PR #11) ----
    // These drive the app through a real egui pass, so list widgets are
    // real, hit-testable widgets.

    /// One egui pass, as the host runs it: the app draws into a root `Ui`
    /// covering the screen between begin_pass/end_pass.
    fn run_pass(app: &mut App, ctx: &egui::Context, events: Vec<Event>, t: f64) {
        pass(app, ctx, frame(events, t));
    }

    /// The pass itself, for input that is already built (and filtered).
    fn pass(app: &mut App, ctx: &egui::Context, raw: RawInput) -> egui::FullOutput {
        let screen = raw.screen_rect.unwrap();
        ctx.begin_pass(raw);
        let mut root = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("test-root"),
            egui::UiBuilder::new().max_rect(screen),
        );
        app.ui(&mut root);
        // No renderer applies the glyph-atlas delta; clear it or epaint
        // panics on the drop.
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        output
    }

    /// A tap at `pos`: move, press, release, all in one pass.
    fn click(pos: egui::Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            button(pos, true),
            button(pos, false),
        ]
    }

    /// A primary press (or release) at `pos`, for `click` and the touch and
    /// drag helpers alike.
    fn button(pos: egui::Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// The editor round trip: a new note saves on close, reopening loads
    /// its body, re-editing updates it in place, and closing returns to
    /// the list.
    #[test]
    fn editor_round_trip_creates_then_updates_a_note() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("editor-roundtrip").join("data"), &ctx, None);

        app.open_editor(0);
        app.buffer = "first draft".to_owned();
        app.close_editor();
        assert!(app.screen() == Screen::List, "closing returns to the list");
        assert_eq!(app.notes.len(), 1);
        assert_eq!(db_body(&app, app.notes[0].id), "first draft");
        let id = app.notes[0].id;

        app.open_editor(id);
        assert_eq!(
            app.buffer, "first draft",
            "the editor loads the stored body"
        );
        app.buffer = "second draft".to_owned();
        app.close_editor();
        assert_eq!(app.notes.len(), 1, "editing updates in place, no duplicate");
        assert_eq!(app.notes[0].id, id);
        assert_eq!(db_body(&app, id), "second draft");
    }

    #[test]
    fn closing_an_empty_new_note_discards_it() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("editor-empty-new").join("data"), &ctx, None);
        app.open_editor(0);
        app.close_editor();
        assert!(app.notes.is_empty(), "an empty new note is never saved");
    }

    #[test]
    fn emptying_an_existing_note_deletes_it() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("editor-empty-out").join("data"), &ctx, None);
        app.open_editor(0);
        app.buffer = "will be emptied".to_owned();
        app.close_editor();
        assert_eq!(app.notes.len(), 1);

        let id = app.notes[0].id;
        app.open_editor(id);
        app.buffer.clear();
        app.close_editor();
        assert!(app.notes.is_empty(), "emptying an existing note deletes it");
    }

    /// Tapping a row's ✕ once arms the delete; only a second tap within
    /// the 2 s window removes the note. (The expiry after 2 s runs on
    /// `Instant` and is not injectable, so it is not tested here.) The arm
    /// is keyed by note id, not list position (MED-1).
    #[test]
    fn delete_cross_needs_two_taps() {
        let ctx = egui::Context::default();
        let mut app = App::new(temp("delete-two-tap").join("data"), &ctx, None);
        app.open_editor(0);
        app.buffer = "to delete".to_owned();
        app.close_editor();
        let id = app.notes[0].id;

        // Lay the list out once, then read the cross's rect back from the
        // widget registry — the same rects egui hit-tests against.
        run_pass(&mut app, &ctx, vec![], 1.0);
        let cross = ctx
            .read_response(egui::Id::new(("del", id)))
            .expect("the delete cross is laid out")
            .rect
            .center();

        run_pass(&mut app, &ctx, click(cross), 2.0);
        assert_eq!(
            app.armed_delete.map(|(armed, _)| armed),
            Some(id),
            "first tap arms by note id"
        );
        assert_eq!(app.notes.len(), 1, "arming alone must not delete");

        run_pass(&mut app, &ctx, click(cross), 2.2);
        assert_eq!(app.armed_delete, None, "the second tap consumes the arm");
        assert!(app.notes.is_empty(), "second tap within the window deletes");
    }

    /// The full stored body of a note (the list rows only carry a prefix).
    fn db_body(app: &App, id: i64) -> String {
        app.db.get(id).unwrap().expect("row exists").body
    }

    // ---- editor touch intent ---------------------------------------------
    // A finger in the editor: a swipe scrolls, a tap places the caret, a
    // press held still past the long-press window starts a selection. These
    // run the host's order every frame: `filter_input`, then the pass.

    /// What egui-winit's `on_touch` emits for the primary finger: the touch
    /// plus the pointer events it mirrors for the first finger down.
    fn winit_finger(phase: TouchPhase, pos: egui::Pos2) -> Vec<Event> {
        let touch = touch_at(0, phase, pos);
        match phase {
            TouchPhase::Start => vec![touch, Event::PointerMoved(pos), button(pos, true)],
            TouchPhase::Move => vec![touch, Event::PointerMoved(pos)],
            TouchPhase::End => vec![touch, button(pos, false), Event::PointerGone],
            TouchPhase::Cancel => vec![touch, Event::PointerGone],
        }
    }

    /// One host frame: eframe's `raw_input_hook` (`filter_input`), then the
    /// pass. Returns the events egui received.
    fn host_frame(app: &mut App, ctx: &egui::Context, events: Vec<Event>, t: f64) -> Vec<Event> {
        let mut raw = frame(events, t);
        app.filter_input(&mut raw);
        let seen = raw.events.clone();
        pass(app, ctx, raw);
        seen
    }

    /// A 200-line note open in the editor, its layout settled.
    fn long_note_in_editor(name: &str) -> (egui::Context, App) {
        let ctx = egui::Context::default();
        let mut app = App::new(temp(name).join("data"), &ctx, None);
        app.open_editor(0);
        app.buffer = (1..=200)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.close_editor();
        let id = app.notes[0].id;
        app.open_editor(id);
        app.nav.settle();
        for k in 0..3 {
            host_frame(&mut app, &ctx, vec![], 0.5 + 0.016 * k as f64);
        }
        (ctx, app)
    }

    /// The editor's text field as last laid out. It moves up as the editor
    /// scrolls down, so its top tracks the scroll offset point for point.
    fn editor_field(ctx: &egui::Context) -> egui::Rect {
        ctx.read_response(egui::Id::new(EDITOR_ID))
            .expect("the editor is laid out")
            .rect
    }

    /// The editor's caret or selection.
    fn editor_cursor(ctx: &egui::Context) -> egui::text::CCursorRange {
        egui::text_edit::TextEditState::load(ctx, egui::Id::new(EDITOR_ID))
            .and_then(|s| s.cursor.char_range())
            .expect("the editor has a cursor")
    }

    /// The pointer presses and releases in `events`, in order.
    fn buttons(events: &[Event]) -> Vec<(egui::Pos2, bool)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::PointerButton { pos, pressed, .. } => Some((*pos, *pressed)),
                _ => None,
            })
            .collect()
    }

    /// A quick upward swipe scrolls the note: the wheel carries the finger's
    /// travel with its sign (finger up, dy < 0, content up, offset grows),
    /// nothing is pressed, and no text gets selected.
    #[test]
    fn swipe_in_the_editor_scrolls_instead_of_selecting() {
        let (ctx, mut app) = long_note_in_editor("touch-swipe");
        let top_before = editor_field(&ctx).top();
        let caret_before = editor_cursor(&ctx);

        let x = 200.0;
        let mut t = 1.0;
        let start = winit_finger(TouchPhase::Start, egui::pos2(x, 600.0));
        let mut seen = host_frame(&mut app, &ctx, start, t);
        for k in 1..=15 {
            t += 0.016;
            let y = 600.0 - 20.0 * k as f32;
            let finger = winit_finger(TouchPhase::Move, egui::pos2(x, y));
            seen.extend(host_frame(&mut app, &ctx, finger, t));
        }
        t += 0.016;
        let lift = winit_finger(TouchPhase::End, egui::pos2(x, 300.0));
        seen.extend(host_frame(&mut app, &ctx, lift, t));
        // egui repaints once more after a frame with input.
        seen.extend(host_frame(&mut app, &ctx, vec![], t + 0.016));

        assert!(buttons(&seen).is_empty(), "a swipe never presses: {seen:?}");
        let wheel: f32 = seen
            .iter()
            .filter_map(|e| match e {
                Event::MouseWheel { delta, .. } => Some(delta.y),
                _ => None,
            })
            .sum();
        assert!(
            (wheel + 300.0).abs() < 0.5,
            "the wheel carries the finger's 300 pt upward travel: {wheel}"
        );
        let moved = top_before - editor_field(&ctx).top();
        assert!(
            (moved - 300.0).abs() < 0.5,
            "the content follows the finger 1:1 (offset +300): moved {moved}"
        );
        let cursor = editor_cursor(&ctx);
        assert!(cursor.is_empty(), "no text selected: {cursor:?}");
        assert_eq!(cursor.primary, caret_before.primary, "the caret stays put");
    }

    /// A quick tap is replayed as a click at the touch point, in one frame,
    /// and the caret moves there (from the end, where opening put it).
    #[test]
    fn tap_in_the_editor_places_the_caret() {
        let (ctx, mut app) = long_note_in_editor("touch-tap");
        let len = app.buffer.chars().count();
        assert_eq!(
            editor_cursor(&ctx).primary.index.0,
            len,
            "opening puts the caret at the end"
        );
        let field = editor_field(&ctx);
        let p = egui::pos2(field.left() + 40.0, field.top() + 10.0); // line 1

        let down = host_frame(&mut app, &ctx, winit_finger(TouchPhase::Start, p), 1.0);
        assert!(buttons(&down).is_empty(), "the press waits for the intent");
        let up = host_frame(&mut app, &ctx, winit_finger(TouchPhase::End, p), 1.08);
        assert_eq!(
            buttons(&up),
            vec![(p, true), (p, false)],
            "replayed as press + release at the touch point, in one frame"
        );
        let cursor = editor_cursor(&ctx);
        assert!(cursor.is_empty(), "a tap selects nothing: {cursor:?}");
        assert!(
            cursor.primary.index.0 <= "line 1".len(),
            "the caret moved to the tapped first line: {cursor:?}"
        );
    }

    /// A press held still past the long-press window is delivered late, at
    /// the touch point; a drag after it selects text, as egui does today.
    #[test]
    fn press_held_still_starts_a_selection() {
        let (ctx, mut app) = long_note_in_editor("touch-hold");
        let field = editor_field(&ctx);
        let p = egui::pos2(field.left() + 40.0, field.top() + 10.0);

        let mut seen = host_frame(&mut app, &ctx, winit_finger(TouchPhase::Start, p), 1.0);
        let wait = app
            .editor_touch
            .wake_after(1.0)
            .expect("the editor asks to be woken");
        assert!((wait.as_secs_f64() - editor_touch::LONG_PRESS).abs() < 1e-6);
        for t in [1.1, 1.2, 1.3] {
            seen.extend(host_frame(&mut app, &ctx, vec![], t));
        }
        assert!(
            buttons(&seen).is_empty(),
            "still undecided inside the window"
        );

        // The frame that wake-up brings at the deadline: no new events, the
        // finger still down.
        let late = host_frame(&mut app, &ctx, vec![], 1.0 + editor_touch::LONG_PRESS);
        assert_eq!(buttons(&late), vec![(p, true)], "the withheld press, now");

        // egui's own long-touch fires 0.8 s after a press held still and
        // ends the drag, so the drag comes well inside that.
        let mut t = 1.45;
        for k in 1..=4 {
            let q = p + egui::vec2(0.0, 30.0 * k as f32);
            host_frame(&mut app, &ctx, winit_finger(TouchPhase::Move, q), t);
            t += 0.016;
        }
        let lift = winit_finger(TouchPhase::End, p + egui::vec2(0.0, 120.0));
        host_frame(&mut app, &ctx, lift, t);
        let cursor = editor_cursor(&ctx);
        assert!(!cursor.is_empty(), "the drag after the hold selected text");
    }

    /// While a touch waits out the long-press window, the editor asks egui
    /// for a frame when the window ends: a finger held still sends nothing
    /// that would bring one.
    #[test]
    fn a_pending_touch_asks_for_a_frame_at_the_window_end() {
        let (ctx, mut app) = long_note_in_editor("touch-wake");
        let field = editor_field(&ctx);
        let p = egui::pos2(field.left() + 40.0, field.top() + 10.0);
        host_frame(&mut app, &ctx, winit_finger(TouchPhase::Start, p), 1.0);

        // No input, and an unfocused window so the caret's blink asks for
        // no repaint of its own.
        let mut idle = |t| {
            let mut raw = RawInput {
                focused: false,
                ..frame(vec![], t)
            };
            app.filter_input(&mut raw);
            pass(&mut app, &ctx, raw).viewport_output[&egui::ViewportId::ROOT].repaint_delay
        };
        // egui answers a frame with input with two repaints, so the first
        // frame after the touch reports zero whatever the editor asks: let
        // it pass, and read the next.
        idle(1.05);
        let delay = idle(1.1);
        assert_ne!(delay, Duration::MAX, "a held finger must still get a frame");
        // egui trims the predicted frame time (1/60 s) off a requested delay.
        assert!(
            (delay.as_secs_f64() - (editor_touch::LONG_PRESS - 0.1)).abs() < 0.05,
            "woken when the window ends, about 0.3 s on: {delay:?}"
        );
    }

    /// A touch that starts on the top bar, and any touch while the list is
    /// on top, reach egui exactly as egui-winit sent them.
    #[test]
    fn touches_off_the_editor_pass_through_untouched() {
        let (ctx, mut app) = long_note_in_editor("touch-elsewhere");
        let bar = egui::pos2(200.0, 20.0);
        for (phase, pos, t) in [
            (TouchPhase::Start, bar, 1.0),
            (TouchPhase::Move, bar + egui::vec2(0.0, 40.0), 1.02),
            (TouchPhase::End, bar + egui::vec2(0.0, 40.0), 1.04),
        ] {
            let sent = winit_finger(phase, pos);
            let seen = host_frame(&mut app, &ctx, sent.clone(), t);
            assert_eq!(seen, sent, "a top-bar touch is not the editor's");
        }

        app.close_editor();
        app.nav.settle();
        host_frame(&mut app, &ctx, vec![], 1.2);
        let body = egui::pos2(200.0, 600.0);
        for (phase, pos, t) in [
            (TouchPhase::Start, body, 1.3),
            (TouchPhase::Move, body - egui::vec2(0.0, 40.0), 1.32),
            (TouchPhase::End, body - egui::vec2(0.0, 40.0), 1.34),
        ] {
            let sent = winit_finger(phase, pos);
            let seen = host_frame(&mut app, &ctx, sent.clone(), t);
            assert_eq!(seen, sent, "the list's touches are egui's");
        }
    }

    /// The edge swipe keeps priority over the editor (iOS): a touch it
    /// claims never reaches the editor's filter, its replayed tap still
    /// clicks, and its swipe pops without scrolling anything.
    #[test]
    fn the_edge_swipe_keeps_its_touches() {
        let (ctx, mut app) = long_note_in_editor("touch-edge");
        app.set_edge_swipe_back(true);
        let edge = egui::pos2(5.0, 400.0); // inside the editor's body too

        let down = host_frame(&mut app, &ctx, winit_finger(TouchPhase::Start, edge), 1.0);
        assert!(down.is_empty(), "the edge claims the whole touch");
        let up = host_frame(&mut app, &ctx, winit_finger(TouchPhase::End, edge), 1.05);
        assert_eq!(
            buttons(&up),
            vec![(edge, true), (edge, false)],
            "the edge filter's replayed tap passes the editor's untouched"
        );

        let far = egui::pos2(250.0, 400.0);
        let mut seen = host_frame(&mut app, &ctx, winit_finger(TouchPhase::Start, edge), 2.0);
        seen.extend(host_frame(
            &mut app,
            &ctx,
            winit_finger(TouchPhase::Move, far),
            2.1,
        ));
        seen.extend(host_frame(
            &mut app,
            &ctx,
            winit_finger(TouchPhase::End, far),
            2.2,
        ));
        assert!(
            !seen.iter().any(|e| matches!(e, Event::MouseWheel { .. })),
            "the edge swipe scrolls nothing"
        );
        assert!(app.screen() == Screen::List, "the swipe popped the editor");
    }

    /// A mouse sends no touches, so the filter never engages: dragging in
    /// the editor still selects text on desktop.
    #[test]
    fn a_mouse_drag_in_the_editor_still_selects() {
        let (ctx, mut app) = long_note_in_editor("mouse-drag");
        let field = editor_field(&ctx);
        let p = egui::pos2(field.left() + 40.0, field.top() + 10.0);

        // `click` releases in the same pass; a drag holds the press.
        let down = vec![Event::PointerMoved(p), button(p, true)];
        assert_eq!(host_frame(&mut app, &ctx, down.clone(), 1.0), down);
        let mut t = 1.0;
        for k in 1..=4 {
            t += 0.016;
            let q = p + egui::vec2(0.0, 30.0 * k as f32);
            host_frame(&mut app, &ctx, vec![Event::PointerMoved(q)], t);
        }
        let end = p + egui::vec2(0.0, 120.0);
        host_frame(&mut app, &ctx, vec![button(end, false)], t + 0.016);
        assert!(!editor_cursor(&ctx).is_empty(), "the mouse drag selected");
    }
}

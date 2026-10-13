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

    /// Call from `raw_input_hook` every frame. Records egui's clock and claims
    /// touches that start at the left edge for the interactive back swipe:
    /// those touches (and the pointer events egui-winit simulates for them)
    /// never reach the widgets. A claimed touch that never moves is replayed
    /// as a plain tap, so the back button under the edge still works.
    /// TODO(verify-on-device): multi-touch interleavings (second finger
    /// during a claim, OS-cancelled touches) can't be produced headlessly —
    /// check the TouchId gating on hardware.
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

    fn finger(phase: TouchPhase, x: f32, id: u64) -> Event {
        Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(id),
            phase,
            pos: egui::pos2(x, 300.0),
            force: None,
        }
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
        let raw = frame(events, t);
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
        ctx.end_pass().textures_delta.clear();
    }

    /// A tap at `pos`: move, press, release, all in one pass.
    fn click(pos: egui::Pos2) -> Vec<Event> {
        let button = |pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![Event::PointerMoved(pos), button(true), button(false)]
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
}

//! Demo Mobile — the feature-gallery consumer of the `rustnotes` host.
//!
//! `hello-mobile` proves the seam exists by doing the least honest thing
//! with every hook; this crate proves what the hooks *carry* by doing the
//! most: each of the host's nine calls lands somewhere observable in the
//! UI, and the four identity consts are this app's own. Like hello-mobile
//! there is no `rustnotes-core` dependency — only the framework seam.
//!
//! What each hook demonstrates here:
//!
//! - `new`  — creates its data dir, installs the host-resolved
//!   platform font (reporting it through `font_source`), and restores
//!   previously persisted state.
//! - `set_edge_swipe_back` — records the host's enabling of the iOS edge
//!   gesture, which `filter_input` then actually drives.
//! - `font_source`  — the real provenance of the installed font.
//! - `set_reduce_motion` — freezes the gallery's animation and is shown
//!   live, with a count of how often the host re-polled.
//! - `set_utc_offset_secs` — drives a live local-time clock.
//! - `filter_input` — counts every event class crossing the seam, and
//!   claims left-edge touches to pop the page stack (the same trick
//!   `rustnotes-core` plays).
//! - `save_current` — persists the editor buffer and counters to disk;
//!   the host fires it on focus loss, backgrounding and exit, so state
//!   survives relaunches.
//! - `on_back`  — pops the page stack, returning false at the root
//!   (on Android the host then leaves the app).
//! - `ui`  — the gallery itself: identity/host facts, live preferences,
//!   input telemetry, an autosaving editor and a pushable page stack.

use egui::{
    Event, FontData, FontDefinitions, FontFamily, Pos2, RawInput, TouchId, TouchPhase, Vec2,
};
use std::path::PathBuf;
use std::sync::Arc;

// The seam itself, in scope for its consts and methods on DemoApp.
use rustnotes::App as _;

/// Left-edge band (points) in which a touch starts the back gesture.
const EDGE_X: f32 = 24.0;
/// Horizontal travel (points) that commits the back gesture.
const SWIPE_MIN: f32 = 48.0;
const STATE_FILE: &str = "demo-state.txt";

/// The gallery's pages. The stack always has the home page at the bottom.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Page {
    Home,
    Preferences,
    Input,
    /// A pushable detail page; the number is its depth for display.
    Detail(u32),
}

/// The app: every seam hook writes somewhere in here.
pub struct DemoApp {
    data_dir: PathBuf,
    font_source: String,
    stack: Vec<Page>,

    // Host-polled preferences (set_* hooks).
    reduce_motion: bool,
    reduce_motion_updates: u32,
    utc_offset_secs: Option<i32>,
    utc_updates: u32,
    edge_swipe_back: Option<bool>,

    // Input pipeline telemetry (filter_input).
    events_seen: u64,
    touches: u64,
    pointers: u64,
    keys: u64,
    texts: u64,
    zooms: u64,
    /// The edge touch currently claimed, if any: (touch id, start pos).
    edge_touch: Option<(TouchId, Pos2)>,

    // Editor + persistence (save_current).
    buffer: String,
    saved_buffer: String,
    save_count: u32,
    back_count: u32,

    /// Animation driver, frozen while reduce motion is on.
    phase: f32,
}

impl rustnotes::App for DemoApp {
    const APP_NAME: &'static str = "Demo Mobile";
    const DESKTOP_DATA_DIR: &'static str = ".demo-mobile";
    const ANDROID_DATA_DIR_FALLBACK: &'static str = "/data/data/com.example.demomobile/files";
    const ANDROID_LOG_TAG: &'static str = "demomobile";

    /// Open the data dir, install the platform font when the host has
    /// one, and restore any state a previous run persisted.
    fn new(
        data_dir: PathBuf,
        ctx: &egui::Context,
        platform_font: Option<(Vec<u8>, String)>,
    ) -> Self {
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            log::error!("create data dir {}: {e}", data_dir.display());
        }
        let font_source = install_font(ctx, platform_font);
        let mut app = DemoApp {
            data_dir,
            font_source,
            stack: vec![Page::Home],
            reduce_motion: false,
            reduce_motion_updates: 0,
            utc_offset_secs: None,
            utc_updates: 0,
            edge_swipe_back: None,
            events_seen: 0,
            touches: 0,
            pointers: 0,
            keys: 0,
            texts: 0,
            zooms: 0,
            edge_touch: None,
            buffer: String::new(),
            saved_buffer: String::new(),
            save_count: 0,
            back_count: 0,
            phase: 0.0,
        };
        app.load_state();
        log::info!("starting in {}", app.data_dir.display());
        app
    }

    /// The host enables this on iOS; the gesture is then driven by the
    /// edge-touch claiming in `filter_input`.
    fn set_edge_swipe_back(&mut self, enabled: bool) {
        self.edge_swipe_back = Some(enabled);
    }

    fn font_source(&self) -> &str {
        &self.font_source
    }

    /// Recorded and shown live; the gallery's orbit freezes while set.
    fn set_reduce_motion(&mut self, reduce: bool) {
        self.reduce_motion = reduce;
        self.reduce_motion_updates += 1;
    }

    /// Recorded and shown as a live local-time clock.
    fn set_utc_offset_secs(&mut self, offset_secs: i32) {
        self.utc_offset_secs = Some(offset_secs);
        self.utc_updates += 1;
    }

    /// Counts every event class, and claims left-edge touches to pop the
    /// page stack while the host has the gesture enabled — the claimed
    /// events never reach egui, so the gesture cannot also scroll.
    fn filter_input(&mut self, raw: &mut RawInput) {
        for e in &raw.events {
            self.events_seen += 1;
            match e {
                Event::Touch { .. } => self.touches += 1,
                Event::PointerButton { pressed: true, .. } => self.pointers += 1,
                Event::Key { pressed: true, .. } => self.keys += 1,
                Event::Text(_) => self.texts += 1,
                Event::Zoom(_) => self.zooms += 1,
                _ => {}
            }
        }
        if self.edge_swipe_back != Some(true) {
            return;
        }
        let mut i = 0;
        while i < raw.events.len() {
            let claimed = match &raw.events[i] {
                Event::Touch { id, phase, pos, .. } => match self.edge_touch {
                    None => {
                        if *phase == TouchPhase::Start && pos.x <= EDGE_X {
                            self.edge_touch = Some((*id, *pos));
                            true
                        } else {
                            false
                        }
                    }
                    Some((tracked, start)) if tracked == *id => {
                        if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
                            if *phase == TouchPhase::End && pos.x - start.x >= SWIPE_MIN {
                                self.pop_page();
                            }
                            self.edge_touch = None;
                        }
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if claimed {
                raw.events.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Persist the buffer and counters; the host fires this on focus
    /// loss, backgrounding and exit, which is the whole demonstration.
    fn save_current(&mut self) {
        self.save_count += 1;
        self.saved_buffer = self.buffer.clone();
        let path = self.state_path();
        let body = format!(
            "buffer={}\nsave_count={}\nback_count={}\nevents_seen={}\n",
            escape(&self.buffer),
            self.save_count,
            self.back_count,
            self.events_seen,
        );
        let tmp = path.with_extension("tmp");
        if let Err(e) = std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, &path)) {
            log::error!("persist state: {e}");
        }
    }

    /// Pop a page; false at the root so the host may leave the app
    /// (Android's back-at-root exit).
    fn on_back(&mut self) -> bool {
        self.back_count += 1;
        self.stack.len() > 1 && self.stack.pop().is_some()
    }

    /// The gallery.
    fn ui(&mut self, root: &mut egui::Ui) {
        // Drive the animation off egui's clock, frozen under reduce
        // motion; request repaints only while it runs.
        let now = root.ctx().input(|i| i.time);
        if !self.reduce_motion {
            self.phase = (now as f32) % 4.0;
            root.ctx().request_repaint();
        }
        let page = *self.stack.last().expect("stack is never empty");
        egui::containers::Panel::top("demo_header")
            .show_separator_line(false)
            .show(root, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let can_pop = self.stack.len() > 1;
                    ui.add_enabled_ui(can_pop, |ui| {
                        if ui.button("‹ back").clicked() {
                            self.pop_page();
                        }
                    });
                    ui.heading(page.title());
                });
                ui.add_space(6.0);
            });
        egui::CentralPanel::default().show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);
                match page {
                    Page::Home => self.ui_home(ui, now),
                    Page::Preferences => self.ui_preferences(ui, now),
                    Page::Input => self.ui_input(ui),
                    Page::Detail(n) => self.ui_detail(ui, n),
                }
            });
        });
    }
}

impl DemoApp {
    fn state_path(&self) -> PathBuf {
        self.data_dir.join(STATE_FILE)
    }

    /// Pop one page (shared by `on_back` and the committed edge swipe).
    fn pop_page(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.back_count += 1;
        }
    }

    fn load_state(&mut self) {
        let Ok(body) = std::fs::read_to_string(self.state_path()) else {
            return;
        };
        for line in body.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "buffer" => {
                    self.buffer = unescape(value);
                    self.saved_buffer = self.buffer.clone();
                }
                "save_count" => self.save_count = value.parse().unwrap_or(0),
                "back_count" => self.back_count = value.parse().unwrap_or(0),
                "events_seen" => self.events_seen = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    fn ui_home(&mut self, ui: &mut egui::Ui, now: f64) {
        let theme = ui.ctx().options(|o| o.fallback_theme);
        ui.heading("Identity (from the trait consts)");
        ui.label(format!("app name: {}", Self::APP_NAME));
        ui.label(format!("data dir: {}", self.data_dir.display()));
        ui.label(format!("font source: {}", self.font_source));
        ui.add_space(12.0);

        ui.heading("Host facts (live)");
        ui.label(format!("appearance: {theme:?}"));
        ui.label(format!(
            "zoom factor (Dynamic Type): {:.2}",
            ui.ctx().zoom_factor()
        ));
        let clock = self
            .local_time(now)
            .unwrap_or_else(|| "— (no offset yet)".into());
        ui.label(format!(
            "utc offset: {}s → local {clock}",
            self.utc_offset_secs()
        ));
        ui.label(format!("edge swipe back: {:?}", self.edge_swipe_back));
        ui.label(format!("events seen: {}", self.events_seen));
        ui.add_space(12.0);

        ui.heading("Gallery");
        if ui.button("Preferences →").clicked() {
            self.stack.push(Page::Preferences);
        }
        if ui.button("Input & editor →").clicked() {
            self.stack.push(Page::Input);
        }
        if ui.button("Push a detail page →").clicked() {
            self.stack.push(Page::Detail(2));
        }
        ui.add_space(8.0);
        ui.small(
            "Back pops a page (button, Android back key, or an iOS edge \
             swipe); at the root the host leaves the app.",
        );
    }

    fn ui_preferences(&mut self, ui: &mut egui::Ui, now: f64) {
        ui.heading("Host-polled preferences");
        ui.label(format!(
            "reduce motion: {} ({} update{})",
            self.reduce_motion,
            self.reduce_motion_updates,
            if self.reduce_motion_updates == 1 {
                ""
            } else {
                "s"
            },
        ));
        ui.label(format!(
            "utc offset: {}s ({} update{}) → local {}",
            self.utc_offset_secs(),
            self.utc_updates,
            if self.utc_updates == 1 { "" } else { "s" },
            self.local_time(now).unwrap_or_else(|| "—".into()),
        ));
        ui.label(format!("edge swipe back: {:?}", self.edge_swipe_back));
        ui.add_space(12.0);

        ui.heading("Motion demo");
        ui.label("The dot orbits while motion is allowed and freezes when the OS asks for less:");
        let (rect, _resp) = ui.allocate_exact_size(Vec2::new(200.0, 80.0), egui::Sense::hover());
        let center = rect.center();
        let angle = self.phase * std::f32::consts::FRAC_PI_2;
        let pos = center + Vec2::new(angle.cos(), angle.sin()) * 28.0;
        ui.painter()
            .circle_filled(pos, 6.0, ui.visuals().widgets.inactive.fg_stroke.color);
        ui.add_space(8.0);
        ui.small("Toggle the OS setting (or Xcode Simulator ▸ Features ▸ Toggle Appearance for the theme) and watch the values above follow.");
    }

    fn ui_input(&mut self, ui: &mut egui::Ui) {
        ui.heading("Input pipeline (filter_input)");
        ui.label(format!("events seen: {}", self.events_seen));
        ui.label(format!(
            "touches {} · pointers {} · keys {} · text {} · zoom {}",
            self.touches, self.pointers, self.keys, self.texts, self.zooms,
        ));
        ui.add_space(12.0);

        ui.heading("Editor (autosave via save_current)");
        ui.add(
            egui::TextEdit::multiline(&mut self.buffer)
                .desired_rows(4)
                .hint_text("type, then background or refocus the app"),
        );
        let clean = self.buffer == self.saved_buffer;
        ui.label(format!(
            "buffer {} · saves {}{}",
            if clean { "saved" } else { "dirty" },
            self.save_count,
            if clean {
                ""
            } else {
                " — changes persist on focus loss, backgrounding or exit"
            },
        ));
        if ui.button("Save now").clicked() {
            self.save_current();
        }
    }

    fn ui_detail(&mut self, ui: &mut egui::Ui, n: u32) {
        ui.heading(format!("Detail page {n}"));
        ui.label(format!("stack depth: {}", self.stack.len()));
        ui.label(format!("backs so far: {}", self.back_count));
        if ui.button(format!("Push detail page {} →", n + 1)).clicked() {
            self.stack.push(Page::Detail(n + 1));
        }
        if ui.button("‹ Pop").clicked() {
            self.pop_page();
        }
        ui.add_space(8.0);
        ui.small("On iOS, swipe from the left edge to pop (when the host has the gesture enabled); on Android, the back key.");
    }

    fn utc_offset_secs(&self) -> i32 {
        self.utc_offset_secs.unwrap_or(0)
    }

    /// egui's clock plus the polled offset, as a wall-clock string.
    fn local_time(&self, now: f64) -> Option<String> {
        let offset = self.utc_offset_secs?;
        let secs = (now as i64 + offset as i64).rem_euclid(86_400);
        Some(format!(
            "{:02}:{:02}:{:02}",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60,
        ))
    }
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Page::Home => "Demo Mobile",
            Page::Preferences => "Preferences",
            Page::Input => "Input & editor",
            Page::Detail(_) => "Detail",
        }
    }
}

/// Install the platform font the host resolved (iOS: CoreText), or leave
/// egui's bundled defaults in place — the same choice `rustnotes-core`
/// makes, in miniature.
fn install_font(ctx: &egui::Context, platform_font: Option<(Vec<u8>, String)>) -> String {
    match platform_font {
        Some((bytes, source)) => {
            let mut defs = FontDefinitions::empty();
            defs.font_data
                .insert("system".to_owned(), Arc::new(FontData::from_owned(bytes)));
            defs.families
                .insert(FontFamily::Proportional, vec!["system".to_owned()]);
            defs.families
                .insert(FontFamily::Monospace, vec!["system".to_owned()]);
            ctx.set_fonts(defs);
            source
        }
        None => "egui default".to_owned(),
    }
}

/// One-line escaping for the state file: backslashes, then newlines.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

/// The inverse of [`escape`].
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The demo's own Android entry — the only `android_main` linked into
/// its cdylib, because the `rustnotes` dependency is built with
/// `default-features = false`. Everything past this line is host
/// framework code.
#[cfg(target_os = "android")]
pub mod android {
    use android_activity::AndroidApp;

    #[unsafe(no_mangle)]
    fn android_main(app: AndroidApp) {
        rustnotes::android::start::<crate::DemoApp>(app);
    }
}

// Keep the state file a single line per field; newlines are escaped.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_round_trips_newlines_and_backslashes() {
        let cases = [
            "",
            "plain",
            "line one\nline two",
            "back\\slash",
            "mix\\\n\r\t",
        ];
        for c in cases {
            assert_eq!(unescape(&escape(c)), c);
        }
    }

    #[test]
    fn local_time_applies_the_polled_offset() {
        let mut app = demo_app();
        assert_eq!(app.local_time(0.0), None);
        app.set_utc_offset_secs(3600);
        assert_eq!(app.local_time(0.0).as_deref(), Some("01:00:00"));
        app.set_utc_offset_secs(-1);
        assert_eq!(app.local_time(0.0).as_deref(), Some("23:59:59"));
    }

    #[test]
    fn back_pops_until_the_root_then_refuses() {
        let mut app = demo_app();
        app.stack.push(Page::Detail(2));
        assert!(app.on_back());
        assert_eq!(app.stack, vec![Page::Home]);
        assert!(!app.on_back());
    }

    #[test]
    fn edge_swipe_end_pops_only_when_enabled_and_far_enough() {
        let mut app = demo_app();
        app.stack.push(Page::Detail(2));
        // Not enabled: the touch passes through untouched.
        let mut raw = raw_with_touch(TouchPhase::Start, egui::Pos2::new(2.0, 50.0), 7);
        app.filter_input(&mut raw);
        assert_eq!(raw.events.len(), 1);
        assert_eq!(app.stack.len(), 2);
        // Enabled: start in the edge band, end past the threshold → pop,
        // and both events are claimed from egui.
        app.set_edge_swipe_back(true);
        let mut raw = raw_with_touch(TouchPhase::Start, egui::Pos2::new(2.0, 50.0), 7);
        app.filter_input(&mut raw);
        assert!(raw.events.is_empty());
        let mut raw = raw_with_touch(TouchPhase::End, egui::Pos2::new(60.0, 50.0), 7);
        app.filter_input(&mut raw);
        assert!(raw.events.is_empty());
        assert_eq!(app.stack, vec![Page::Home]);
        // A short swipe does not pop.
        app.stack.push(Page::Detail(2));
        let mut raw = raw_with_touch(TouchPhase::Start, egui::Pos2::new(2.0, 50.0), 8);
        app.filter_input(&mut raw);
        let mut raw = raw_with_touch(TouchPhase::End, egui::Pos2::new(30.0, 50.0), 8);
        app.filter_input(&mut raw);
        assert_eq!(app.stack.len(), 2);
    }

    #[test]
    fn save_current_persists_across_reopen() {
        let dir = std::env::temp_dir().join(format!("demo-mobile-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = demo_app_in(dir.clone());
        app.buffer = "hello\nworld".into();
        app.save_current();
        let reopened = demo_app_in(dir.clone());
        assert_eq!(reopened.buffer, "hello\nworld");
        assert_eq!(reopened.save_count, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn demo_app() -> DemoApp {
        demo_app_in(std::env::temp_dir().join("demo-mobile-test-unused"))
    }

    fn demo_app_in(dir: PathBuf) -> DemoApp {
        let ctx = egui::Context::default();
        DemoApp::new(dir, &ctx, None)
    }

    fn raw_with_touch(phase: TouchPhase, pos: egui::Pos2, id: u64) -> RawInput {
        let mut raw = RawInput::default();
        raw.events.push(Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: TouchId(id),
            phase,
            pos,
            force: None,
        });
        raw
    }
}

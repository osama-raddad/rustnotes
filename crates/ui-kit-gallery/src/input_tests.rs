//! Headless input tests for the gallery: the real app, built through
//! the `rustnotes::App` seam the host drives (`new`, then
//! `filter_input` and `ui` every frame), at phone size on ONE
//! persistent context. Fingers arrive the way egui-winit delivers them
//! from a touch screen, so these pin what a phone sees: the page, the
//! sideways specimen rows and the tab strip scroll under the finger
//! through egui's own drag-to-scroll from the very first swipe, taps
//! keep landing once touches have been seen, and the bar's theme menu
//! works by touch. The context must persist across frames: egui
//! resolves a click from a press in one pass and its release in a
//! later one, and a drag from the frames between.
//!
//! Two oracles: the page's scroll offset, read back from egui's memory
//! (the page area's `scroll_area::State`), and where a known string
//! was painted, read from the frame's shapes (what a screenshot would
//! show, without the screenshot).

use std::path::PathBuf;

use eframe::egui;
use rustnotes::App as _;
use ui_kit::gallery_copy as copy;
use ui_kit::theme::ThemeId;

use crate::GalleryApp;

/// The phone the tests hold: under the gallery's 600pt compact
/// breakpoint, so the phone layout is the one under test.
const PHONE: egui::Vec2 = egui::vec2(384.0, 855.0);

/// One frame's length, the 60Hz the hosts run at: egui derives the
/// pointer's velocity (and with it the fling) from the clock.
const FRAME: f64 = 1.0 / 60.0;

/// The persistent-context harness: the app, the context, the window,
/// the clock and the strings the last frame painted.
struct Harness {
    ctx: egui::Context,
    app: GalleryApp,
    window: egui::Vec2,
    time: f64,
    /// Every string the last frame painted, with its rect.
    strings: Vec<(String, egui::Rect)>,
}

impl Harness {
    /// The app on the phone.
    fn phone() -> Self {
        Self::window(PHONE)
    }

    /// The app as the host creates it, in a `window`-sized viewport,
    /// warmed for a few frames (the sizing passes settle and the widget
    /// ids register).
    fn window(window: egui::Vec2) -> Self {
        let ctx = egui::Context::default();
        let app = GalleryApp::new(PathBuf::new(), &ctx, None);
        let mut harness = Self {
            ctx,
            app,
            window,
            time: 1.0,
            strings: Vec::new(),
        };
        harness.idle(3);
        harness
    }

    /// One frame with `events`, through the app's input filter first,
    /// as the host's `raw_input_hook` runs it.
    fn frame(&mut self, events: Vec<egui::Event>) {
        self.time += FRAME;
        let mut raw = egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.window)),
            time: Some(self.time),
            ..Default::default()
        };
        self.app.filter_input(&mut raw);
        let out = self.ctx.run_ui(raw, |ui| self.app.ui(ui));
        self.strings = painted_strings(&out.shapes);
        out.drop_without_applying_deltas();
    }

    /// `frames` frames with no input.
    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame(Vec::new());
        }
    }

    /// A tap: one finger down and up again at `pos`.
    fn tap(&mut self, pos: egui::Pos2) {
        self.frame(finger_down(pos));
        self.frame(finger_up(pos));
        self.idle(1);
    }

    /// A swipe: one finger down at `from`, moved to `to` over `steps`
    /// frames, and lifted there (a lift at speed, so egui flings).
    fn swipe(&mut self, from: egui::Pos2, to: egui::Pos2, steps: usize) {
        self.frame(finger_down(from));
        for step in 1..=steps {
            self.frame(finger_move(from.lerp(to, step as f32 / steps as f32)));
        }
        self.frame(finger_up(to));
    }

    /// A spot on the page's left margin, low on the window: nothing
    /// there is interactive, so a tap on it lands on nothing.
    fn margin(&self) -> egui::Pos2 {
        egui::pos2(8.0, self.window.y * 0.7)
    }

    /// A first touch on the page margin: egui treats the screen as a
    /// touch screen only once it has seen a touch, the state a phone is
    /// in after its first tap.
    fn touch_screen(&mut self) {
        self.tap(self.margin());
        assert!(
            self.ctx.input(|i| i.has_touch_screen()),
            "the touches must reach egui untouched"
        );
    }

    /// The page's live scroll offset, from the page area's state.
    fn page_offset(&self) -> egui::Vec2 {
        let id = self
            .app
            .gallery
            .page_scroll_id
            .expect("the chrome has shown the page");
        egui::scroll_area::State::load(&self.ctx, id)
            .expect("the page area stores its state")
            .offset
    }

    /// Where the last frame painted `text` (its topmost occurrence), if
    /// it painted it at all: off-screen text is culled.
    fn painted(&self, text: &str) -> Option<egui::Rect> {
        self.strings
            .iter()
            .filter(|(painted, _)| painted == text)
            .map(|(_, rect)| *rect)
            .min_by(|a, b| a.top().total_cmp(&b.top()))
    }

    /// Wheel the page down until `text` is painted inside `band` (a y
    /// range), then let the smoothed wheel settle; `None` when it never
    /// shows.
    fn scroll_to(&mut self, text: &str, band: std::ops::Range<f32>) -> Option<egui::Rect> {
        let over_page = egui::pos2(self.window.x / 2.0, self.window.y * 0.6);
        for _ in 0..600 {
            self.frame(vec![
                egui::Event::PointerMoved(over_page),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -12.0),
                    modifiers: Default::default(),
                    phase: egui::TouchPhase::Move,
                },
            ]);
            if self
                .painted(text)
                .is_some_and(|rect| band.contains(&rect.top()))
            {
                self.idle(60);
                return self.painted(text);
            }
        }
        None
    }
}

/// Every string a frame painted, with its rect, where its clip lets it
/// show.
fn painted_strings(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
    fn walk(shape: &egui::Shape, clip: egui::Rect, out: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => {
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                if rect.intersects(clip) {
                    out.push((text.galley.text().to_owned(), rect));
                }
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, clip, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        walk(&clipped.shape, clipped.clip_rect, &mut out);
    }
    out
}

/// A finger landing at `pos`, as egui-winit's `on_touch` delivers it:
/// the touch itself, then (the first finger drives the pointer) the
/// pointer moved there and pressed.
fn finger_down(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        touch(egui::TouchPhase::Start, pos),
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        },
    ]
}

/// The finger moving to `pos`: the touch, then the pointer follows.
fn finger_move(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        touch(egui::TouchPhase::Move, pos),
        egui::Event::PointerMoved(pos),
    ]
}

/// The finger lifting at `pos`: the touch ends, the pointer releases,
/// then vanishes (no hover is left behind on a touch screen).
fn finger_up(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        touch(egui::TouchPhase::End, pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        },
        egui::Event::PointerGone,
    ]
}

fn touch(phase: egui::TouchPhase, pos: egui::Pos2) -> egui::Event {
    egui::Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(0),
        phase,
        pos,
        force: None,
    }
}

/// A finger swiping up the Brand page scrolls it natively: the page
/// follows the finger 1:1 while it is down and keeps going after the
/// lift (egui's kinetic fling). The swipe is the first touch after
/// launch, the one egui's touch-armed default lets land on nothing.
#[test]
fn touch_drag_scrolls_the_page() {
    let mut h = Harness::phone();
    let before = h.page_offset().y;
    h.swipe(egui::pos2(190.0, 700.0), egui::pos2(190.0, 300.0), 10);
    let released = h.page_offset().y;
    h.idle(30);
    let settled = h.page_offset().y;
    assert!(
        released >= before + 300.0,
        "the page must follow a 400pt swipe: {before} -> {released}"
    );
    assert!(
        settled > released + 50.0,
        "the lift must fling the page on: {released} -> {settled}"
    );
}

/// A sideways swipe over the Foundations state tiles scrolls that row
/// under the finger (the row's own horizontal scroll area takes the
/// drag) and leaves the page where it was. The wheel brings the tiles
/// into view, so the swipe is the first touch after launch.
#[test]
fn sideways_touch_drag_scrolls_a_specimen_row() {
    let mut h = Harness::phone();
    h.app.gallery.tab = 1;
    h.idle(3);
    // The pressed tile's caption sits mid-row: on screen before the
    // swipe and after it.
    let caption = copy::STATE_CAPTIONS[2];
    let before = h
        .scroll_to(caption, 250.0..600.0)
        .expect("the state tiles scroll into view");
    let page = h.page_offset().y;
    let y = before.center().y;
    h.swipe(egui::pos2(340.0, y), egui::pos2(60.0, y), 10);
    h.idle(30);
    let after = h.painted(caption).expect("the caption stays on screen");
    assert!(
        after.left() < before.left() - 100.0,
        "the row must follow the finger sideways: x {} -> {}",
        before.left(),
        after.left()
    );
    assert!(
        (h.page_offset().y - page).abs() < 1.0,
        "a sideways swipe must leave the page alone: {page} -> {}",
        h.page_offset().y
    );
}

/// Once touches have been seen (egui's drag-to-scroll is armed), taps
/// still land: one on the tab strip switches panels, one inside the
/// scrolling page dismisses a banner, and the restore button appears
/// only then.
#[test]
fn taps_still_land_after_touches_have_been_seen() {
    let mut h = Harness::phone();
    h.touch_screen();

    let foundations = h.painted(copy::TABS[1]).expect("the tab strip is painted");
    h.tap(foundations.center());
    assert_eq!(h.app.gallery.tab, 1, "a tap on Foundations switches panels");

    let feedback = h.painted(copy::TABS[3]).expect("the tab strip is painted");
    h.tap(feedback.center());
    assert_eq!(h.app.gallery.tab, 3, "a tap on Feedback switches panels");
    h.idle(2);
    assert!(
        h.painted(copy::BANNERS_RESTORE).is_none(),
        "all four banners show at first, so there is nothing to restore"
    );

    // The info banner's dismiss is the topmost close glyph on the page.
    let close = ui_kit::icons::Icon::Close.as_char().to_string();
    let dismiss = h.painted(&close).expect("the info banner's dismiss");
    h.tap(dismiss.center());
    h.idle(2);
    assert!(
        h.painted(copy::BANNERS_RESTORE).is_some(),
        "a tap inside the page dismisses the banner and offers the restore"
    );
}

/// The first touch after launch can be a sideways swipe on the tab
/// strip, which outgrows a phone: the strip follows that very swipe,
/// bringing its clipped last tab in, and the swipe is no tap on the tab
/// under the finger. The taps after it still land: one on the tab the
/// swipe brought in switches panels, one inside the page dismisses a
/// banner.
#[test]
fn a_first_swipe_on_the_tab_strip_scrolls_it_and_taps_still_land() {
    let mut h = Harness::phone();
    // The strip ends at the content panel's inset; Feedback, its last
    // tab, runs past that edge until the strip scrolls.
    let edge = h.window.x - ui_kit::tokens::spacing::SPACE_6;
    let before = h.painted(copy::TABS[3]).expect("the tab strip is painted");
    assert!(
        before.right() > edge,
        "the four tabs must outgrow a phone's strip: Feedback ends at {}, the strip at {edge}",
        before.right()
    );
    let y = before.center().y;
    h.swipe(egui::pos2(300.0, y), egui::pos2(40.0, y), 10);
    h.idle(30);
    let after = h.painted(copy::TABS[3]).expect("the tab strip is painted");
    assert!(
        after.right() <= edge,
        "the strip must follow the first swipe: Feedback ends at {} -> {} (the strip at {edge})",
        before.right(),
        after.right()
    );
    assert_eq!(h.app.gallery.tab, 0, "a swipe is not a tap on a tab");

    h.tap(after.center());
    assert_eq!(h.app.gallery.tab, 3, "a tap on Feedback switches panels");
    h.idle(2);
    // The info banner's dismiss is the topmost close glyph on the page.
    let close = ui_kit::icons::Icon::Close.as_char().to_string();
    let dismiss = h.painted(&close).expect("the info banner's dismiss");
    h.tap(dismiss.center());
    h.idle(2);
    assert!(
        h.painted(copy::BANNERS_RESTORE).is_some(),
        "a tap inside the page dismisses the banner and offers the restore"
    );
}

/// The wheel still scrolls the page, and the offset accumulates across
/// frames rather than resetting each one.
#[test]
fn wheel_events_scroll_the_page() {
    let mut h = Harness::phone();
    let before = h.page_offset().y;
    for _ in 0..30 {
        h.frame(vec![
            egui::Event::PointerMoved(egui::pos2(180.0, 500.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -5.0),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
            },
        ]);
    }
    h.idle(30);
    let after = h.page_offset().y;
    assert!(
        after > before + 100.0,
        "30 wheel steps of 5pt must scroll the page: {before} -> {after}"
    );
}

/// The bar's theme menu is the kit's, and it works by touch: a tap on
/// the trigger opens the popover, a tap on a theme switches the live
/// theme (the gallery's labels stay unselectable under it) and leaves
/// the popover open, a tap on a toggle flips its flag, and a tap
/// outside the popover closes it.
#[test]
fn the_theme_menu_switches_themes_and_closes_on_a_tap_outside() {
    let mut h = Harness::phone();
    // The Feedback panel paints no theme names of its own, so every
    // name on screen is the menu's.
    h.app.gallery.tab = 3;
    h.idle(3);
    h.touch_screen();
    let dark = ThemeId::Dark.name();
    assert!(h.painted(dark).is_none(), "the menu starts closed");

    let trigger = h
        .painted(copy::THEME_MENU)
        .expect("the bar's theme trigger");
    h.tap(trigger.center());
    h.idle(2);
    let choice = h.painted(dark).expect("the open menu lists every theme");
    h.tap(choice.center());
    assert_eq!(
        h.app.gallery.theme,
        ThemeId::Dark,
        "the tap picks the theme"
    );
    assert_eq!(h.ctx.theme(), egui::Theme::Dark, "and applies it");
    assert!(
        !h.ctx
            .style_of(egui::Theme::Dark)
            .interaction
            .selectable_labels,
        "labels stay unselectable under the new theme"
    );

    h.idle(2);
    let motion = h
        .painted(copy::REDUCED_MOTION)
        .expect("the menu stays open after a choice");
    let reduced = h.app.gallery.flags.reduced_motion;
    h.tap(motion.center());
    assert_ne!(
        h.app.gallery.flags.reduced_motion, reduced,
        "the toggle flips reduced motion"
    );

    h.tap(h.margin());
    h.idle(2);
    assert!(
        h.painted(dark).is_none(),
        "a tap outside the popover closes the menu"
    );
}

/// Where the bar has room, the theme trigger names the live theme
/// after the menu's label; a phone's bar keeps the bare label (the one
/// the menu test above taps).
#[test]
fn the_theme_trigger_names_the_live_theme_on_a_wide_window() {
    let h = Harness::window(egui::vec2(1200.0, 800.0));
    let named = format!("{} · {}", copy::THEME_MENU, ThemeId::Light.name());
    assert!(
        h.painted(&named).is_some(),
        "a wide bar's trigger reads {named:?}"
    );
    assert!(
        h.painted(copy::THEME_MENU).is_none(),
        "a wide bar's trigger is not the bare label"
    );
}

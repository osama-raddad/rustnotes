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
//! Three oracles: the page's scroll offset, read back from egui's
//! memory (the page area's `scroll_area::State`); where a known string
//! was painted, read from the frame's shapes (what a screenshot would
//! show, without the screenshot); and the accessibility tree, which
//! names each control and gives its widget's own rect (what a screen
//! reader is told).

use std::path::PathBuf;

use eframe::egui;
use eframe::egui::accesskit::Role;
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
/// the clock, and what the last frame painted and told a screen reader.
struct Harness {
    ctx: egui::Context,
    app: GalleryApp,
    window: egui::Vec2,
    time: f64,
    /// Every string the last frame painted, with its rect.
    strings: Vec<(String, egui::Rect)>,
    /// Every labelled node of the last frame's accessibility tree: its
    /// role, its label and its widget's rect.
    nodes: Vec<(Role, String, egui::Rect)>,
    /// The node the last frame's accessibility tree held keyboard focus
    /// on, with its role, label and rect; `None` while nothing is
    /// focused. A screen reader is told the same.
    focus: Option<(Role, String, egui::Rect)>,
    /// The bounds of every mesh the last frame painted.
    meshes: Vec<egui::Rect>,
    /// Every shape the last frame painted, with its clip.
    shapes: Vec<egui::epaint::ClippedShape>,
    /// How many passes the last frame took: one, unless the app asked
    /// egui to lay the frame out again (`request_discard`).
    passes: usize,
    /// Every frame run so far.
    frames: usize,
    /// How many of those frames took a second pass.
    multipass_frames: usize,
}

impl Harness {
    /// The app on the phone.
    fn phone() -> Self {
        Self::window(PHONE)
    }

    /// The app as the host creates it, in a `window`-sized viewport,
    /// warmed for a few frames (the sizing passes settle and the widget
    /// ids register). The accessibility tree is on, as a screen reader
    /// turns it on: its nodes carry each control's name and rect.
    fn window(window: egui::Vec2) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let app = GalleryApp::new(PathBuf::new(), &ctx, None);
        let mut harness = Self {
            ctx,
            app,
            window,
            time: 1.0,
            strings: Vec::new(),
            nodes: Vec::new(),
            focus: None,
            meshes: Vec::new(),
            shapes: Vec::new(),
            passes: 0,
            frames: 0,
            multipass_frames: 0,
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
        self.nodes = labelled_nodes(&out.platform_output);
        self.focus = focused_node(&out.platform_output);
        self.meshes = painted_meshes(&out.shapes);
        self.shapes = out.shapes.clone();
        self.passes = out.platform_output.num_completed_passes;
        self.frames += 1;
        self.multipass_frames += usize::from(1 < self.passes);
        out.drop_without_applying_deltas();
    }

    /// `frames` frames with no input.
    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame(Vec::new());
        }
    }

    /// A key press: `key` down and up again, a frame each, with no
    /// modifiers (a keyboard user's Tab, which moves focus).
    fn press(&mut self, key: egui::Key) {
        for pressed in [true, false] {
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
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

    /// `steps` wheel notches of `delta` points each over the page, then
    /// time for the smoothed wheel to settle.
    fn wheel_page(&mut self, steps: usize, delta: f32) {
        let over_page = egui::pos2(self.window.x / 2.0, self.window.y * 0.6);
        for _ in 0..steps {
            self.frame(vec![
                egui::Event::PointerMoved(over_page),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, delta),
                    modifiers: Default::default(),
                    phase: egui::TouchPhase::Move,
                },
            ]);
        }
        self.idle(60);
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

    /// The rect of the topmost `role` node in the last frame's
    /// accessibility tree whose label starts with `label` (a name can
    /// run on: the theme trigger's carries the live theme after the
    /// menu's label), if there is one. The rect is the widget's own,
    /// whatever its label paints.
    fn node(&self, role: Role, label: &str) -> Option<egui::Rect> {
        self.nodes
            .iter()
            .filter(|(node_role, name, _)| *node_role == role && name.starts_with(label))
            .map(|(_, _, rect)| *rect)
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

/// Every node of a frame's accessibility tree that carries a label and
/// a rect: what a screen reader is told, and where.
fn labelled_nodes(output: &egui::PlatformOutput) -> Vec<(Role, String, egui::Rect)> {
    let Some(update) = &output.accesskit_update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let bounds = node.bounds()?;
            let rect = egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            );
            Some((node.role(), node.label()?.to_owned(), rect))
        })
        .collect()
}

/// The node of a frame's accessibility tree that holds keyboard focus,
/// with its role, label and rect; `None` when the focus is on no
/// labelled control (a clean start has it on the window itself).
fn focused_node(output: &egui::PlatformOutput) -> Option<(Role, String, egui::Rect)> {
    let update = output.accesskit_update.as_ref()?;
    let (_, node) = update.nodes.iter().find(|(id, _)| *id == update.focus)?;
    let bounds = node.bounds()?;
    let rect = egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    );
    Some((node.role(), node.label()?.to_owned(), rect))
}

/// The bounds of every mesh a frame painted.
fn painted_meshes(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Rect> {
    fn walk(shape: &egui::Shape, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Mesh(mesh) => out.push(mesh.calc_bounds()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        walk(&clipped.shape, &mut out);
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
    // The tiles are as wide as their labels, so the pressed tile's
    // caption starts near the phone's right edge: on screen before the
    // swipe, and pulled left, still on screen after it.
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

/// A vertical swipe that starts on a specimen row scrolls the page under
/// the finger, and the row holds still. Before the row owned its gesture's
/// axis, its horizontal area claimed the whole drag and the page stayed
/// put; the first few points of travel now pick the axis, and a vertical
/// gesture hands its movement to the page.
#[test]
fn vertical_touch_drag_on_a_specimen_row_scrolls_the_page() {
    let mut h = Harness::phone();
    h.app.gallery.tab = 1;
    h.idle(3);
    let caption = copy::STATE_CAPTIONS[2];
    let before = h
        .scroll_to(caption, 250.0..600.0)
        .expect("the state tiles scroll into view");
    let page = h.page_offset().y;
    // The tiles are as wide as their labels, so the pressed tile's
    // caption starts near the phone's right edge and its centre is off
    // screen: press on the window's centre line instead. First park the
    // pointer 186pt to its left, so the touch's first delta jumps 186pt
    // sideways, as a press on the caption's centre did. That jump runs
    // rightward and the row sits at its left end, so the row cannot
    // follow it; the axis must come from the travel since the press.
    let x = h.window.x / 2.0;
    let y = before.center().y;
    h.frame(vec![egui::Event::PointerMoved(egui::pos2(x - 186.0, y))]);
    h.swipe(egui::pos2(x, y), egui::pos2(x, y - 200.0), 10);
    h.idle(30);
    assert!(
        h.page_offset().y >= page + 150.0,
        "a vertical swipe on the row must scroll the page: {page} -> {}",
        h.page_offset().y
    );
    let after = h.painted(caption).expect("the caption is still on screen");
    assert!(
        (after.left() - before.left()).abs() < 1.0,
        "the row must hold still on a vertical swipe: x {} -> {}",
        before.left(),
        after.left()
    );
}

/// Dismiss the Feedback panel's info banner by touch, the topmost close
/// glyph on the page, and check that the panel then offers to restore
/// it: a tap inside the scrolling page lands.
fn dismiss_the_info_banner(h: &mut Harness) {
    let close = ui_kit::icons::Icon::Close.as_char().to_string();
    let dismiss = h.painted(&close).expect("the info banner's dismiss");
    h.tap(dismiss.center());
    h.idle(2);
    assert!(
        h.painted(copy::BANNERS_RESTORE).is_some(),
        "a tap inside the page dismisses the banner and offers the restore"
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

    dismiss_the_info_banner(&mut h);
}

/// The first touch after launch can be a sideways swipe on the tab
/// strip, which outgrows a phone, the narrowest common one (360pt) the
/// most: the strip follows that very swipe, bringing its clipped last
/// tab in, and the swipe, pressed on Components, is no tap on it. The
/// taps after it still land: one on the tab the swipe brought in
/// switches panels, one inside the page dismisses a banner.
#[test]
fn a_first_swipe_on_the_tab_strip_scrolls_it_and_taps_still_land() {
    let mut h = Harness::window(egui::vec2(360.0, PHONE.y));
    // The strip ends at the content panel's gutter; Feedback, its last
    // tab, runs past that edge until the strip scrolls.
    let edge = h.window.x - crate::gallery::PAGE_GUTTER;
    let before = h.painted(copy::TABS[3]).expect("the tab strip is painted");
    assert!(
        before.right() > edge,
        "the four tabs must outgrow a phone's strip: Feedback ends at {}, the strip at {edge}",
        before.right()
    );
    // The finger lands on a tab, Components, at its painted centre.
    let press = h
        .painted(copy::TABS[2])
        .expect("the tab strip is painted")
        .center();
    h.swipe(press, press - egui::vec2(200.0, 0.0), 10);
    h.idle(30);
    let after = h.painted(copy::TABS[3]).expect("the tab strip is painted");
    assert!(
        after.right() <= edge,
        "the strip must follow the first swipe: Feedback ends at {} -> {} (the strip at {edge})",
        before.right(),
        after.right()
    );
    assert_eq!(
        h.app.gallery.tab, 0,
        "a swipe is not a tap on the tab under the finger"
    );

    h.tap(after.center());
    assert_eq!(h.app.gallery.tab, 3, "a tap on Feedback switches panels");
    h.idle(2);
    dismiss_the_info_banner(&mut h);
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

/// Every panel keeps its own page scroll offset. Scroll Brand down and
/// switch to Foundations: the panel opens at its top, not at Brand's
/// offset. Scroll Foundations to a different depth, and each panel is
/// where its own reader left it when they switch back.
#[test]
fn each_panel_keeps_its_own_page_scroll_offset() {
    let mut h = Harness::phone();
    h.wheel_page(60, -10.0);
    let brand = h.page_offset().y;
    assert!(brand > 300.0, "Brand must scroll down first: {brand}");

    h.app.gallery.tab = 1;
    h.idle(3);
    assert!(
        h.page_offset().y < 1.0,
        "Foundations is first shown, so it opens at its top, not at Brand's {brand}: {}",
        h.page_offset().y
    );

    h.wheel_page(30, -10.0);
    let foundations = h.page_offset().y;
    assert!(
        foundations > 100.0 && (foundations - brand).abs() > 100.0,
        "Foundations must scroll to a depth of its own: {foundations} against Brand's {brand}"
    );

    h.app.gallery.tab = 0;
    h.idle(3);
    assert!(
        (h.page_offset().y - brand).abs() < 1.0,
        "Brand keeps the offset it was left at: {brand} -> {}",
        h.page_offset().y
    );

    h.app.gallery.tab = 1;
    h.idle(3);
    assert!(
        (h.page_offset().y - foundations).abs() < 1.0,
        "Foundations keeps the offset it was left at: {foundations} -> {}",
        h.page_offset().y
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
        .node(Role::Button, copy::THEME_MENU)
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

/// The theme popover's rows are touch targets, and its two groups read
/// as one list. Each of the six theme rows and the two flag rows
/// registers a rect for interaction at least the 44pt touch floor
/// tall (read off the accessibility tree, which carries each widget's
/// own rect, not its glyph), the rows stack at least 44pt apart so no
/// two targets overlap, and the radio labels and the toggle labels
/// start at one x. The rows once registered the kit's 20pt radio row
/// (a 32pt pitch with its 12pt gap) and 26pt toggle row (about 41pt
/// apart), and the toggle labels sat 28pt right of the radio labels.
/// A tap at the edge of each reported rect lands, so the rect is the
/// live target and not only a number.
#[test]
fn the_theme_popovers_rows_are_44pt_and_their_labels_align() {
    use ui_kit::components::button::TOUCH_TARGET_MIN;

    // Both layouts: the phone, and a desktop window where the bar
    // shows the named trigger.
    let mut off = Vec::new();
    for window in [PHONE, egui::vec2(1280.0, 900.0)] {
        let mut h = Harness::window(window);
        // The Feedback panel paints no theme names or flag labels of its
        // own, so every one on screen is the menu's.
        h.app.gallery.tab = 3;
        h.idle(3);
        h.touch_screen();
        let at = format!("at {}pt", window.x);

        let trigger = h
            .node(Role::Button, copy::THEME_MENU)
            .expect("the bar's theme trigger");
        h.tap(trigger.center());
        h.idle(2);

        // The six theme rows then the two flag rows, in reading order,
        // each with the rect it registered and the x its label painted
        // at.
        let radios = ThemeId::ALL.iter().map(|theme| theme.name());
        let toggles = [copy::REDUCED_MOTION, copy::REDUCED_TRANSPARENCY];
        let mut rows: Vec<(&str, egui::Rect, f32, bool)> = Vec::new();
        for (name, role, is_radio) in radios
            .map(|name| (name, Role::RadioButton, true))
            .chain(toggles.map(|name| (name, Role::CheckBox, false)))
        {
            let rect = h
                .node(role, name)
                .unwrap_or_else(|| panic!("{at}: the open menu has no {role:?} row {name:?}"));
            let label = h
                .painted(name)
                .unwrap_or_else(|| panic!("{at}: the open menu paints no {name:?}"));
            rows.push((name, rect, label.left(), is_radio));
        }

        for (name, rect, _, _) in &rows {
            if rect.height() < TOUCH_TARGET_MIN - 0.01 {
                off.push(format!(
                    "{at}: the {name:?} row registers {}pt tall, under the {TOUCH_TARGET_MIN}pt \
                     touch floor ({rect:?})",
                    rect.height()
                ));
            }
        }
        for pair in rows.windows(2) {
            let ((above, a, _, _), (below, b, _, _)) = (&pair[0], &pair[1]);
            let pitch = b.top() - a.top();
            if pitch < TOUCH_TARGET_MIN - 0.01 {
                off.push(format!(
                    "{at}: {above:?} and {below:?} sit {pitch}pt apart, under the \
                     {TOUCH_TARGET_MIN}pt touch floor"
                ));
            }
        }
        let radio_x = rows[0].2;
        for (name, _, x, is_radio) in &rows {
            if (x - radio_x).abs() > 0.01 {
                off.push(format!(
                    "{at}: the {} label {name:?} starts at x {x}, not the radio labels' x \
                     {radio_x}",
                    if *is_radio { "radio" } else { "toggle" }
                ));
            }
        }

        // The rect's edges are live: a tap 2pt inside the top of a theme
        // row picks it, and 2pt inside the bottom of a flag row flips
        // it.
        let dark = rows
            .iter()
            .find(|row| row.0 == ThemeId::Dark.name())
            .expect("the Dark row");
        h.tap(egui::pos2(dark.1.center().x, dark.1.top() + 2.0));
        if h.app.gallery.theme != ThemeId::Dark {
            off.push(format!(
                "{at}: a tap 2pt inside the top of the {:?} row ({:?}) did not pick it",
                dark.0, dark.1
            ));
        }
        h.idle(2);
        let motion = h
            .node(Role::CheckBox, copy::REDUCED_MOTION)
            .expect("the Reduced motion row");
        let reduced = h.app.gallery.flags.reduced_motion;
        h.tap(egui::pos2(motion.center().x, motion.bottom() - 2.0));
        if h.app.gallery.flags.reduced_motion == reduced {
            off.push(format!(
                "{at}: a tap 2pt inside the bottom of the Reduced motion row ({motion:?}) \
                 did not flip it"
            ));
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// The bar's search popover closes on a tap outside it, as the theme
/// menu's does, and the bar holds one popover open at a time: opening
/// either one closes the other.
#[test]
fn the_search_popover_closes_on_a_tap_outside_and_gives_way_to_the_theme_menu() {
    let mut h = Harness::phone();
    // The Feedback panel paints neither a theme name nor the search
    // field's label of its own, so any on screen are the popovers'.
    h.app.gallery.tab = 3;
    h.idle(3);
    h.touch_screen();
    let field = copy::SEARCH_FIELD_LABEL;
    let dark = ThemeId::Dark.name();
    let search = h
        .node(Role::Button, copy::SEARCH_BUTTON_NAME)
        .expect("the bar's search button");
    let trigger = h
        .node(Role::Button, copy::THEME_MENU)
        .expect("the bar's theme trigger");

    h.tap(search.center());
    h.idle(2);
    assert!(
        h.painted(field).is_some(),
        "a tap on the search button opens its popover"
    );
    h.tap(h.margin());
    h.idle(2);
    assert!(
        h.painted(field).is_none(),
        "a tap outside the search popover closes it"
    );

    h.tap(search.center());
    h.idle(2);
    h.tap(trigger.center());
    h.idle(2);
    assert!(
        h.painted(field).is_none() && h.painted(dark).is_some(),
        "opening the theme menu closes the search popover"
    );
    h.tap(search.center());
    h.idle(2);
    assert!(
        h.painted(dark).is_none() && h.painted(field).is_some(),
        "opening the search popover closes the theme menu"
    );
}

/// The rect of the one popover open over the page: the area rect of the
/// visible foreground layer, which is the popover's frame (the kit's
/// frame adds no outer margin, so the area is the painted box). The
/// harness holds one popover open at a time and the Feedback panel
/// opens none of its own, so a second foreground layer fails the lookup.
fn open_popover(h: &Harness) -> egui::Rect {
    let rects: Vec<egui::Rect> = h.ctx.memory(|memory| {
        memory
            .areas()
            .visible_layer_ids()
            .into_iter()
            .filter(|layer| layer.order == egui::Order::Foreground)
            .filter_map(|layer| memory.area_rect(layer.id))
            .collect()
    });
    assert_eq!(rects.len(), 1, "one popover is open: {rects:?}");
    rects[0]
}

/// The bar's two popovers open below the bar, hang from their triggers
/// by one rule, and share one inner padding.
///
/// The rule: a popover starts the `BAR_POPOVER_GAP` under the bar's
/// bottom edge, and its right edge stands on its trigger's right edge.
/// Where the popover is wider than the room between that edge and the
/// bar's left edge (the search popover at a phone's width), it is held
/// inside the bar instead: its left edge stands on the bar's left edge.
/// The content stands the inner margin and the frame's border in from
/// the popover's edge, and the popovers keep their widths. Both
/// popovers are asserted at a phone, the narrowest phone, the compact
/// breakpoint and a desktop, and the run must reach the held case and
/// the plain case, or it fails.
#[test]
fn the_bars_popovers_open_below_the_bar_and_hang_from_their_triggers() {
    use crate::gallery::{BAR_POPOVER_GAP, BAR_POPOVER_PAD, SEARCH_POPOVER_WIDTH};

    // Painted to the pixel, laid out in whole points: a point's slack.
    const SLACK: f32 = 1.0;
    // The theme popover's width, as it was and stays: the widest row
    // (the "Reduced transparency" toggle) and the frame, as measured.
    // `THEME_MENU_WIDTH` (248pt) is only a ceiling, and the area
    // shrinks to its content.
    const THEME_WIDTH: f32 = 226.156;
    // The kit popover frame's 1pt border: the content stands this far
    // further in than the frame's inner margin.
    const BORDER: f32 = 1.0;

    let mut off = Vec::new();
    let (mut hung, mut held) = (0, 0);
    for window in [
        PHONE,
        egui::vec2(320.0, 800.0),
        egui::vec2(600.0, 800.0),
        egui::vec2(1280.0, 900.0),
    ] {
        let mut h = Harness::window(window);
        // The Feedback panel paints neither popover's content of its
        // own, and no popover of its own.
        h.app.gallery.tab = 3;
        h.idle(3);
        h.touch_screen();
        let at = format!("at {}pt", window.x);
        let pane = bar_pane(&h, ThemeId::Light);
        let search = h
            .node(Role::Button, copy::SEARCH_BUTTON_NAME)
            .expect("the bar's search button");
        let trigger = h
            .node(Role::Button, copy::THEME_MENU)
            .expect("the bar's theme trigger");

        // Each popover opens with a tap on its trigger; opening the
        // second closes the first.
        for (name, button, width) in [
            ("search", search, SEARCH_POPOVER_WIDTH),
            ("theme", trigger, THEME_WIDTH),
        ] {
            h.tap(button.center());
            h.idle(2);
            let popover = open_popover(&h);
            let at = format!("{at} the {name} popover {popover:?}");

            if (popover.width() - width).abs() > SLACK {
                off.push(format!("{at} is {}pt wide, not {width}pt", popover.width()));
            }
            if !h.ctx.content_rect().contains_rect(popover) {
                off.push(format!("{at} leaves the window"));
            }

            // Below the bar, a gap clear of its border.
            let gap = popover.top() - pane.bottom();
            if (gap - BAR_POPOVER_GAP).abs() > SLACK {
                off.push(format!(
                    "{at} starts {gap}pt under the bar's bottom edge {}, not {BAR_POPOVER_GAP}pt",
                    pane.bottom()
                ));
            }

            // Hung from the trigger's right edge, or held inside the bar.
            if popover.width() <= button.right() - pane.left() {
                hung += 1;
                if (popover.right() - button.right()).abs() > SLACK {
                    off.push(format!(
                        "{at} ends at x {}, not on its trigger's right edge {}",
                        popover.right(),
                        button.right()
                    ));
                }
            } else {
                held += 1;
                if (popover.left() - pane.left()).abs() > SLACK {
                    off.push(format!(
                        "{at} starts at x {}, not on the bar's left edge {} (its trigger \
                         ends at {}, too near the bar's left edge to hang from)",
                        popover.left(),
                        pane.left(),
                        button.right()
                    ));
                }
            }

            // The padding: the content stands the 16pt inner margin and
            // the border in from the popover's edge. The search field's
            // control is the one 44pt box the TextField paints (the
            // accessibility tree gives the text input no label to find
            // it by); the theme menu's flag rows start at the content's
            // edge.
            let (content, left) = if name == "search" {
                let control = ui_kit::components::text_field::CONTROL_HEIGHT;
                let input = painted_rects(&h)
                    .into_iter()
                    .map(|(shape, _)| shape.rect)
                    .filter(|rect| {
                        popover.contains_rect(*rect) && (rect.height() - control).abs() < 0.01
                    })
                    .min_by(|a, b| a.left().total_cmp(&b.left()))
                    .unwrap_or_else(|| panic!("{at}: paints no search input"));
                ("input", input.left())
            } else {
                let row = h
                    .node(Role::CheckBox, copy::REDUCED_MOTION)
                    .unwrap_or_else(|| panic!("{at}: has no Reduced motion row"));
                ("flag row", row.left())
            };
            let inset = left - popover.left();
            if (inset - (BAR_POPOVER_PAD + BORDER)).abs() > SLACK {
                off.push(format!(
                    "{at}: its {content} starts {inset}pt in from the popover's edge, not the \
                     {BAR_POPOVER_PAD}pt inner margin and the {BORDER}pt border"
                ));
            }
        }
    }
    // The floor of four hung popovers is both popovers at the two
    // wider windows (600 and 1280pt); the narrow windows hang the theme
    // popover and hold the search popover.
    if hung < 4 || held == 0 {
        off.push(format!(
            "the run hung {hung} popovers from their triggers and held {held} inside the bar; \
             it must hang at least four and hold one"
        ));
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// The bar keeps its working controls clear of the rest of its row
/// from the narrowest phones to a wide desktop (320, the first iPhone
/// SE and the small Androids; 600, the compact breakpoint; 768 and 834,
/// iPad mini and iPad Air portrait; 1024, iPad landscape; 1200 and
/// 1440, desktops, the widest holding the fullest bar) in every theme,
/// each with its own faces and name: the theme trigger and the search
/// button keep at least the bar's action gap from every nav link and
/// from the wordmark. Touching counts as crowding: laid out without
/// measuring, the search button came within half a point of the last
/// nav link at 1024.
#[test]
fn the_bar_keeps_its_controls_clear_of_the_nav_links_and_the_wordmark() {
    let gap = ui_kit::components::top_app_bar::ACTIONS_GAP;
    let mut crowded = Vec::new();
    // One app, resized as a window is: the bar measures its row afresh
    // every frame. It is the same bar over every panel, and the
    // Feedback panel is the cheapest one to lay out beneath it.
    let mut h = Harness::window(egui::vec2(320.0, 800.0));
    h.app.gallery.tab = 3;
    for width in [320.0, 600.0, 768.0, 834.0, 1024.0, 1200.0, 1440.0] {
        h.window = egui::vec2(width, 800.0);
        for theme in ThemeId::ALL {
            h.app.gallery.theme = *theme;
            crate::gallery::apply_theme(*theme, &h.ctx);
            h.idle(2);
            let trigger = h
                .node(Role::Button, copy::THEME_MENU)
                .expect("the bar's theme trigger");
            let search = h
                .node(Role::Button, copy::SEARCH_BUTTON_NAME)
                .expect("the bar's search button");
            if width >= 1440.0 {
                // The widest window holds the fullest bar, so the check
                // below covers it: the trigger naming the theme and the
                // call to action beside it.
                let named = format!("{} · {}", copy::THEME_MENU, theme.name());
                let holds = |name: &str| {
                    h.nodes
                        .iter()
                        .any(|(role, label, _)| *role == Role::Button && label == name)
                };
                assert!(
                    holds(&named) && holds(copy::START_A_PROJECT),
                    "at {width}pt in {} the bar holds everything",
                    theme.name()
                );
            }
            let mut neighbours = nav_links(&h);
            neighbours.push(("the wordmark".to_owned(), wordmark(&h, trigger)));
            for (control, rect) in [("theme trigger", trigger), ("search button", search)] {
                let reach = rect.expand2(egui::vec2(gap, 0.0));
                for (name, neighbour) in &neighbours {
                    if reach.intersects(*neighbour) {
                        crowded.push(format!(
                            "at {width}pt in {} the {control} {rect:?} crowds {name} {neighbour:?}",
                            theme.name()
                        ));
                    }
                }
            }
        }
    }
    assert!(crowded.is_empty(), "{}", crowded.join("\n"));
}

/// The window widths the bar's geometry tests run at (the centring and
/// padding test, the bottom edge test, the tab strip's place): the
/// narrowest phone, a phone, the compact breakpoint, a tablet and a
/// wide desktop. The tiers they reach are listed with
/// `the_bar_centres_its_wordmark_and_pads_its_ends`.
const BAR_WIDTHS: [f32; 5] = [320.0, 384.0, 600.0, 1024.0, 1440.0];

/// The bar's content sits on one centre line, and the bar pads its ends
/// alike but for one deliberate difference. The wordmark, the nav
/// links, the call to action and the two controls each centre on the
/// pane's centre line. The first thing the bar shows on its left (the
/// wordmark) and the last on its right (the call to action's fill, or
/// the theme trigger's icon where the call to action is shed) each
/// stand the bar's own padding in from the pane's edge, from the
/// narrowest phone to a wide desktop in every theme. The exception is
/// the labelled trigger's icon (the chevron): its box stands 4pt
/// further in, 28pt against the wordmark's 24pt, so that the trigger's
/// focus ring, which reaches 4pt past its box, stays inside the bar
/// (`the_theme_triggers_focus_ring_stays_inside_the_bar`). That is a
/// known limit of the bar. The icon button's icon and the call to
/// action stand the full 24pt in.
///
/// The row once began at egui's 18pt interact height, so the 24pt
/// wordmark, placed first, was pinned to the row's top and sat 10pt
/// above the 44pt controls' centre line; and the ghost theme trigger
/// kept its own button padding (20pt, the icon button's 10pt) inside
/// the bar's, so its icon stood that much further in than the wordmark
/// does. An icon is measured by its painted box, the 20pt or 24pt cell
/// the glyph sits in; the glyph's ink sits a few points inside that.
/// The bar's 68pt height is asserted too, so a fix cannot buy its
/// centring by growing the bar.
///
/// The widths are `BarFit` tiers: 320 is the compact bar with the icon
/// trigger (the Tune icon ends the row), 384 (a phone) and 600 (the
/// compact breakpoint, where the full bar still does not fit) the
/// compact bar with the bare label (the chevron ends the row), and
/// 1024 and 1440 the full bar, whose call to action ends the row. The
/// run must reach all three trailing cases, or it fails: a breakpoint
/// change cannot drop one of them unseen. The tier with the nav links
/// and no call to action is not sampled.
#[test]
fn the_bar_centres_its_wordmark_and_pads_its_ends() {
    use std::collections::BTreeSet;

    use ui_kit::components::badge::ICON_BUTTON_SIZE;
    use ui_kit::components::top_app_bar::{bar_height, BAR_PAD_X, CONTENT_HEIGHT};
    use ui_kit::states::FOCUS_RING_REACH;

    // Painted to the pixel, laid out in whole points: a point's slack.
    const SLACK: f32 = 1.0;
    const CTA: &str = "the call to action";
    const CHEVRON: &str = "the bare chevron";
    const TUNE: &str = "the tune icon";
    let mut off = Vec::new();
    let mut ends = BTreeSet::new();
    let mut h = Harness::window(egui::vec2(320.0, 800.0));
    h.app.gallery.tab = 3;
    for width in BAR_WIDTHS {
        h.window = egui::vec2(width, 800.0);
        for theme in ThemeId::ALL {
            h.app.gallery.theme = *theme;
            crate::gallery::apply_theme(*theme, &h.ctx);
            h.idle(2);
            let at = format!("at {width}pt in {}", theme.name());
            let pane = bar_pane(&h, *theme);
            let trigger = h
                .node(Role::Button, copy::THEME_MENU)
                .expect("the bar's theme trigger");
            let search = h
                .node(Role::Button, copy::SEARCH_BUTTON_NAME)
                .expect("the bar's search button");
            let mark = wordmark(&h, trigger);
            let cta = h.node(Role::Button, copy::START_A_PROJECT);

            if pane.height() != bar_height(CONTENT_HEIGHT) {
                off.push(format!("{at} the bar is {}pt tall", pane.height()));
            }

            // One centre line.
            let line = pane.center().y;
            let mut centred = vec![
                ("the wordmark".to_owned(), mark),
                ("the theme trigger".to_owned(), trigger),
                ("the search button".to_owned(), search),
            ];
            centred.extend(cta.map(|rect| (CTA.to_owned(), rect)));
            centred.extend(nav_links(&h));
            for (name, rect) in centred {
                if (rect.center().y - line).abs() > SLACK {
                    off.push(format!(
                        "{at} {name} {rect:?} is centred on y {}, not the bar's {line}",
                        rect.center().y
                    ));
                }
            }

            // Both ends: the wordmark's gap, and the gap of whatever
            // ends the row.
            let leading = mark.left() - pane.left();
            let (end, trailing) = match cta {
                // A filled button: its fill is what shows.
                Some(cta) => (CTA, pane.right() - cta.right()),
                // A ghost button or a bare icon: the icon is what shows.
                // The icon button is the 44pt circle; the ghost button
                // with the chevron is wider.
                None => {
                    let end = if trigger.width() == ICON_BUTTON_SIZE {
                        TUNE
                    } else {
                        CHEVRON
                    };
                    (end, pane.right() - trigger_icon(&h, trigger).right())
                }
            };
            ends.insert(end);
            if (leading - BAR_PAD_X).abs() > SLACK {
                off.push(format!(
                    "{at} the leading gap is {leading}pt, not the bar's {BAR_PAD_X}pt padding"
                ));
            }
            // The chevron's icon box stands the padding plus the focus
            // ring's reach in; the other icon boxes (and the call to
            // action's fill) stand the padding in.
            let (wanted, why) = if end == CHEVRON {
                (
                    BAR_PAD_X + FOCUS_RING_REACH,
                    format!(
                        "the bar's {BAR_PAD_X}pt padding and {FOCUS_RING_REACH}pt for the focus ring"
                    ),
                )
            } else {
                (BAR_PAD_X, format!("the bar's {BAR_PAD_X}pt padding"))
            };
            if (trailing - wanted).abs() > SLACK {
                off.push(format!(
                    "{at} the trailing gap to {end} is {trailing}pt, not {wanted}pt ({why})"
                ));
            }
        }
    }
    for end in [CTA, CHEVRON, TUNE] {
        if !ends.contains(end) {
            off.push(format!("no width ended the bar with {end}"));
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// The bar's bottom edge is its own border and nothing beside it: no
/// straight rule runs along the edge, under the pane's rounded corners
/// or against its flat part. egui's top panel paints a 1pt separator
/// line across its whole edge by default (`Panel::show_separator_line`),
/// and the bar's pane stands flush on that edge: on the flat part the
/// line lay against the pane's border and doubled it, at each rounded
/// corner the curve lifted off the line and left a notch, and the line
/// ran on past the corners to the window's edges. The pane's border is
/// the one bottom edge, so the test finds the pane by that border (a bar
/// that lost it fails the lookup) and looks for any line segment or
/// thin rect, wide enough to cover the pane's flat bottom, within
/// `NEAR` of the edge.
#[test]
fn the_bars_bottom_edge_is_its_border_alone() {
    use ui_kit::tokens::radii::RADIUS_LG;

    // A rule this near the bottom edge lies on the 1pt border (drawn
    // inside the pane) or against it.
    const NEAR: f32 = 2.0;
    // The thickest a filled rect can be and still read as a rule.
    const RULE: f32 = 2.0;

    // Two line ends at the same height, to within this, make a
    // horizontal line.
    const LEVEL: f32 = 0.01;

    let mut off = Vec::new();
    for width in BAR_WIDTHS {
        let mut h = Harness::window(egui::vec2(width, 800.0));
        // The Feedback panel: the page chrome only, as the other bar
        // tests run it.
        h.app.gallery.tab = 3;
        h.idle(3);
        let at = format!("at {width}pt");
        let pane = bar_pane(&h, ThemeId::Light);
        // The pane's flat bottom: its width between the rounded corners.
        let flat = pane.shrink2(egui::vec2(RADIUS_LG, 0.0));

        let mut rules: Vec<(&str, egui::Rect)> = Vec::new();
        for (points, stroke) in painted_segments(&h) {
            if (points[0].y - points[1].y).abs() < LEVEL {
                let line = egui::Rect::from_two_pos(points[0], points[1]);
                // A stroke is centred on its line: the painted band
                // reaches half the stroke's width to either side.
                rules.push(("a line", line.expand2(egui::vec2(0.0, stroke.width / 2.0))));
            }
        }
        for (shape, _) in painted_rects(&h) {
            if shape.rect.height() <= RULE {
                rules.push(("a thin rect", shape.rect));
            }
        }
        for (kind, rule) in rules {
            let along_the_edge =
                rule.top() < pane.bottom() + NEAR && rule.bottom() > pane.bottom() - NEAR;
            let covers_the_flat = rule.left() <= flat.left() && flat.right() <= rule.right();
            if along_the_edge && covers_the_flat {
                off.push(format!(
                    "{at} {kind} {rule:?} runs along the bar's bottom edge y {}, x {} to {}",
                    pane.bottom(),
                    pane.left(),
                    pane.right()
                ));
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// The tab strip stands where it stood under the bar when egui's
/// separator line ran along the bar's bottom edge: its 1pt rule, the
/// strip's lowest line, lies 85pt under the bar's bottom edge. The line
/// had reserved 1pt of room in the top panel's outer margin; turning it
/// off (`the_bars_bottom_edge_is_its_border_alone`) gives that room up
/// unless the clearance keeps it, and the page below would slide up 1pt.
/// The rule is the 1pt-tall rect as wide as the bar, under it. The 85pt
/// is a golden value, as `the_bars_controls_keep_their_places` has: a
/// deliberate change to the strip's height or to the bar's clearance
/// re-records it, copying `under` from the failure output.
#[test]
fn the_tab_strip_keeps_its_place_under_the_bar() {
    const SLACK: f32 = 0.02;
    const RECORDED: f32 = 85.0;

    let mut off = Vec::new();
    for width in BAR_WIDTHS {
        let mut h = Harness::window(egui::vec2(width, 800.0));
        h.app.gallery.tab = 3;
        h.idle(3);
        let pane = bar_pane(&h, ThemeId::Light);
        let rules: Vec<egui::Rect> = painted_rects(&h)
            .into_iter()
            .map(|(shape, _)| shape.rect)
            .filter(|rect| {
                (rect.height() - 1.0).abs() <= SLACK
                    && rect.top() > pane.bottom()
                    && (rect.left() - pane.left()).abs() <= SLACK
                    && (rect.right() - pane.right()).abs() <= SLACK
            })
            .collect();
        match rules.as_slice() {
            [rule] => {
                let under = rule.top() - pane.bottom();
                if (under - RECORDED).abs() > SLACK {
                    off.push(format!(
                        "at {width}pt the tab strip's rule {rule:?} lies {under}pt under the \
                         bar's bottom edge {}, not the recorded {RECORDED}pt",
                        pane.bottom()
                    ));
                }
            }
            _ => off.push(format!(
                "at {width}pt the tab strip paints {} rules as wide as the bar under it, \
                 not one: {rules:?}",
                rules.len()
            )),
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// Keyboard focus on the bar's theme trigger rings it inside the bar.
/// With the call to action shed the labelled trigger ends the bar's
/// row and its box runs on into the bar's padding (`ThemeTrigger::
/// overhang`). Its focus ring reaches 4pt past the box (the 2pt offset
/// and the 2pt outline), so a box 4pt short of the pane's edge put the
/// ring's outer edge on the bar's border, where the pane's clip cuts
/// it. The ring must lie inside the pane, whole under its clip, and
/// keep its own reach (4pt) clear of the pane's right edge. The
/// compact bars with the bare label are 384 (a phone) and 600 (the
/// compact breakpoint) wide, and the bare label is the one labelled
/// trigger that ends the row there (the named label comes with the
/// call to action, which ends it instead); the icon trigger and the
/// call to action sit further in. Focus comes from two Tab presses:
/// the bar, first in the tab order, answers the first with its leftmost
/// control, the search button, and the second with the trigger beside
/// it (`tab_reaches_the_bars_controls_in_visual_order`). A third would
/// move on past it.
#[test]
fn the_theme_triggers_focus_ring_stays_inside_the_bar() {
    use ui_kit::states::{FOCUS_RING_OFFSET, FOCUS_RING_REACH};

    let mut off = Vec::new();
    for width in [384.0, 600.0] {
        for theme in ThemeId::ALL {
            let mut h = Harness::window(egui::vec2(width, 800.0));
            h.app.gallery.tab = 3;
            h.app.gallery.theme = *theme;
            crate::gallery::apply_theme(*theme, &h.ctx);
            h.idle(2);
            let at = format!("at {width}pt in {}", theme.name());
            let pane = bar_pane(&h, *theme);
            let trigger = h
                .node(Role::Button, copy::THEME_MENU)
                .expect("the bar's theme trigger");

            // The ring around the trigger: the focus-ring stroke, at full
            // strength (it fades in over 120ms, so the frames after the
            // Tab let it land), on the trigger's rect grown by the
            // ring's offset to within half a point. `outer` below
            // assumes the kit's stroke lies outside that rect.
            let colour = theme.colors().focus_ring;
            let around_trigger = |h: &Harness| {
                painted_rects(h).into_iter().find(|(shape, _)| {
                    let inner = shape.rect.shrink(FOCUS_RING_OFFSET);
                    shape.stroke.color == colour
                        && trigger.expand(0.5).contains_rect(inner)
                        && inner.expand(0.5).contains_rect(trigger)
                })
            };
            h.press(egui::Key::Tab);
            h.press(egui::Key::Tab);
            h.idle(12);
            let Some((shape, clip)) = around_trigger(&h) else {
                off.push(format!(
                    "{at} the second Tab press drew no focus ring round the trigger \
                     {trigger:?}: focus did not reach it, or the ring's geometry \
                     or colour changed"
                ));
                continue;
            };

            let outer = shape.visual_bounding_rect();
            let clear = pane.right() - outer.right();
            if clear < FOCUS_RING_REACH - 0.01 {
                off.push(format!(
                    "{at} the focus ring {outer:?} ends {clear}pt from the bar's right edge {}, \
                     not the {FOCUS_RING_REACH}pt it needs",
                    pane.right()
                ));
            }
            if !pane.contains_rect(outer) {
                off.push(format!(
                    "{at} the focus ring {outer:?} leaves the bar {pane:?}"
                ));
            }
            if !clip.contains_rect(outer) {
                off.push(format!(
                    "{at} the focus ring {outer:?} is cut by its clip {clip:?}"
                ));
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// The rounding allowance of the strip's rings: egui snaps painted
/// edges, and a scrolled row's origin, to whole device pixels, which is
/// half a point at the harness's one-to-one scale. The 4pt cut the ring
/// once had is eight times that.
const PIXEL_SNAP: f32 = 0.5;

/// The focus ring round one tab of the tab strip, as the last frame
/// painted it.
struct StripRing {
    /// The tab's label.
    label: &'static str,
    /// The ring's outline shape.
    shape: egui::epaint::RectShape,
    /// The clip the ring was painted under: the strip's clip, widened
    /// along x by the ring's reach and kept inside the host's.
    clip: egui::Rect,
    /// The tab's label rect, from which the tab's own box follows.
    label_rect: egui::Rect,
}

impl StripRing {
    /// The tab's own box: the label with its 4pt side paddings, and the
    /// 14pt paddings over and under its line box.
    fn tab(&self) -> egui::Rect {
        use ui_kit::components::tabs::{TAB_PAD_X, TAB_PAD_Y, TAB_TEXT_SIZE};
        use ui_kit::tokens::leading::LEADING_NORMAL;

        egui::Rect::from_center_size(
            self.label_rect.center(),
            egui::vec2(
                self.label_rect.width() + 2.0 * TAB_PAD_X,
                2.0 * TAB_PAD_Y + TAB_TEXT_SIZE * LEADING_NORMAL,
            ),
        )
    }

    /// Whether the ring is whole: drawn at its full reach round the tab
    /// (not pulled in to fit, give or take [`PIXEL_SNAP`]) and lying
    /// inside the clip it was painted under, give or take `slack`.
    /// Pushes what is wrong, if anything, onto `off`.
    fn check_whole(&self, at: &str, slack: f32, off: &mut Vec<String>) {
        use ui_kit::states::FOCUS_RING_REACH;

        let outer = self.shape.visual_bounding_rect();
        let wanted = self.tab().expand(FOCUS_RING_REACH);
        if (outer.min - wanted.min).abs().max_elem() > PIXEL_SNAP
            || (outer.max - wanted.max).abs().max_elem() > PIXEL_SNAP
        {
            off.push(format!(
                "{at} the {} tab's focus ring {outer:?} is not drawn at its full reach \
                 ({FOCUS_RING_REACH}pt) round the tab: {wanted:?}",
                self.label
            ));
        }
        if !self.clip.expand(slack).contains_rect(outer) {
            off.push(format!(
                "{at} the {} tab's focus ring {outer:?} is cut by the clip it was painted \
                 under, {:?}",
                self.label, self.clip
            ));
        }
    }
}

/// The ring round the strip's tab `label` in the last frame, if one is
/// painted: the focus-ring stroke whose rect holds the tab's label,
/// below the bar. (At the wide tiers the bar's nav links carry the same
/// names, which is why the label is looked for below the bar's pane.)
fn strip_ring(h: &Harness, theme: ThemeId, label: &'static str) -> Option<StripRing> {
    let below = bar_pane(h, theme).bottom();
    let label_rect = h
        .strings
        .iter()
        .filter(|(text, rect)| text == label && rect.top() > below)
        .map(|(_, rect)| *rect)
        .min_by(|a, b| a.top().total_cmp(&b.top()))?;
    let colour = theme.colors().focus_ring;
    painted_rects(h)
        .into_iter()
        .find(|(shape, _)| shape.stroke.color == colour && shape.rect.contains_rect(label_rect))
        .map(|(shape, clip)| StripRing {
            label,
            shape,
            clip,
            label_rect,
        })
}

/// From a clean start, press Tab (the bar's controls first, then the
/// strip's tabs left to right) until each of `labels` has been ringed,
/// and return each tab's ring as it first stood at full strength. A tab
/// the presses never reached is reported onto `off` (under the context
/// `at`) and is missing from the result.
fn tab_to_strip_rings(
    h: &mut Harness,
    theme: ThemeId,
    labels: &[&'static str],
    at: &str,
    off: &mut Vec<String>,
) -> Vec<StripRing> {
    // The widest bar tier holds seven stops (four nav links, the search,
    // the theme trigger and the call to action) ahead of the strip.
    const MOST_PRESSES: usize = 16;
    let mut rings: Vec<Option<StripRing>> = labels.iter().map(|_| None).collect();
    for _ in 0..MOST_PRESSES {
        h.press(egui::Key::Tab);
        // The ring fades in over 120ms; let it land.
        h.idle(12);
        for (label, ring) in labels.iter().zip(&mut rings) {
            if ring.is_none() {
                *ring = strip_ring(h, theme, label);
            }
        }
        if rings.iter().all(Option::is_some) {
            break;
        }
    }
    labels
        .iter()
        .zip(rings)
        .filter_map(|(label, ring)| {
            if ring.is_none() {
                off.push(format!(
                    "{at} no Tab press drew a focus ring round the {label} tab"
                ));
            }
            ring
        })
        .collect()
}

/// Keyboard focus on the tab strip's first tab, Brand, rings it whole.
/// The strip is a sideways scroll area, which clips its content to its
/// own rect, and that rect starts at the content margin, where the
/// first tab starts: the ring reaches 4pt past the tab (the 2pt offset
/// and the 2pt outline), so the clip cut the ring's left stroke and
/// both its left corners off (on a Pixel, from x 48px). The ring must
/// be drawn at its full reach and lie inside the clip it was painted
/// under, and the tab must still start on the content margin, where it
/// stood before. Focus comes from real Tab presses: the bar, first in
/// the tab order, answers the first of them, the strip follows its
/// controls. Run at a phone's width and a desktop's, in every theme.
#[test]
fn the_tab_strip_keeps_the_first_tabs_focus_ring_whole() {
    use crate::gallery::PAGE_GUTTER;

    let brand = copy::TABS[0];
    let mut off = Vec::new();
    for width in [384.0, 1200.0] {
        for theme in ThemeId::ALL {
            let mut h = Harness::window(egui::vec2(width, 800.0));
            h.app.gallery.theme = *theme;
            crate::gallery::apply_theme(*theme, &h.ctx);
            h.idle(2);
            let at = format!("at {width}pt in {}", theme.name());
            for ring in tab_to_strip_rings(&mut h, *theme, &[brand], &at, &mut off) {
                ring.check_whole(&at, 0.0, &mut off);
                // The fix makes room for the ring; it must not move the tab.
                if (ring.tab().left() - PAGE_GUTTER).abs() > 0.01 {
                    off.push(format!(
                        "{at} the {brand} tab starts at x {}, not on the content margin \
                         {PAGE_GUTTER}",
                        ring.tab().left()
                    ));
                }
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// Every tab of the strip rings whole, not only the first. On a desktop
/// the four tabs sit inside the strip; on a phone the strip outgrows
/// its width, and at the end of its scroll the last tab, Feedback, is
/// the one that reaches the clip's right edge (as Brand reaches the
/// left at the start), so a swipe takes the strip to its end before the
/// presses, and Feedback is the tab held to that edge. That scroll is
/// not exact: egui rounds a scrolled row's origin to whole device
/// pixels, so the last tab can stand up to [`PIXEL_SNAP`] past the
/// strip's edge, and only that case gets the allowance on its clip
/// check. The swipe must have reached the strip's end, or the case
/// would test the wrong edge.
#[test]
fn the_tab_strip_rings_every_tab_whole() {
    use crate::gallery::PAGE_GUTTER;

    let mut off = Vec::new();
    for (width, scrolled) in [(1200.0, false), (384.0, true)] {
        for theme in ThemeId::ALL {
            let mut h = Harness::window(egui::vec2(width, 800.0));
            h.app.gallery.theme = *theme;
            crate::gallery::apply_theme(*theme, &h.ctx);
            h.idle(2);
            let at = format!(
                "at {width}pt in {}{}",
                theme.name(),
                if scrolled {
                    ", scrolled to the end"
                } else {
                    ""
                }
            );
            let labels: Vec<&'static str> = if scrolled {
                // The tabs scrolled out of sight are not the edge's.
                vec![copy::TABS[3]]
            } else {
                copy::TABS.to_vec()
            };
            if scrolled {
                let press = h
                    .painted(copy::TABS[2])
                    .expect("the tab strip is painted")
                    .center();
                h.swipe(press, press - egui::vec2(200.0, 0.0), 10);
                h.idle(30);
            }
            for ring in tab_to_strip_rings(&mut h, *theme, &labels, &at, &mut off) {
                ring.check_whole(&at, if scrolled { PIXEL_SNAP } else { 0.0 }, &mut off);
                if scrolled {
                    // The strip ends at the content panel's gutter.
                    let end = h.window.x - PAGE_GUTTER;
                    if (ring.tab().right() - end).abs() > PIXEL_SNAP {
                        off.push(format!(
                            "{at} the swipe left the {} tab ending at x {}, not at the strip's \
                             end {end}",
                            ring.label,
                            ring.tab().right()
                        ));
                    }
                }
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// What the bar offers a keyboard, each control with its rect and in
/// the order the eye reads them (by left edge): the nav links where the
/// row has room for them, the search button, the theme trigger, and the
/// call to action where the row has room for that. The wordmark is not
/// one: it is a picture, not a control.
fn bar_controls(h: &Harness) -> Vec<(String, egui::Rect)> {
    let mut controls = nav_links(h);
    for (name, label) in [
        ("the search button", copy::SEARCH_BUTTON_NAME),
        ("the theme trigger", copy::THEME_MENU),
        ("the call to action", copy::START_A_PROJECT),
    ] {
        if let Some(rect) = h.node(Role::Button, label) {
            controls.push((name.to_owned(), rect));
        }
    }
    controls.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    controls
}

/// The widths the bar's Tab-order and pinned-rect tests run at, each a
/// different `BarFit` tier: 320 is the compact bar with the icon
/// trigger, 384 (a phone) the compact bar with the bare label, 900 the
/// nav links with no call to action, and 1280 the fullest bar: nav
/// links, the call to action and the trigger naming the theme. Whether
/// the bar holds the nav links and the call to action is part of what
/// the Tab-order test expects (the pinned-rect test finds its controls
/// by name), so that a breakpoint change cannot drop a tier unseen.
const BAR_TIERS: [(f32, bool, bool); 4] = [
    // (window width, the nav links, the call to action)
    (320.0, false, false),
    (384.0, false, false),
    (900.0, true, false),
    (1280.0, true, true),
];

/// Tab walks the bar's controls in the order they stand on the screen,
/// left to right: from a clean start the first press focuses the first
/// control on the left and each press moves one control right (on a
/// phone: the search, then the theme trigger beside it; where the row
/// has room for them, the nav links first and the call to action last).
/// egui hands keyboard focus along in the order the widgets are created
/// in a frame, not the order they stand in, so a bar whose actions were
/// built from the right (the layout runs right to left, to end the row
/// flush with its padding) put the theme trigger ahead of the search it
/// stands beside, and the call to action ahead of both. The wordmark is
/// not a stop: n presses visit exactly the bar's n controls. Run at
/// every tier of the bar, in every theme (the label widths change with
/// the faces).
#[test]
fn tab_reaches_the_bars_controls_in_visual_order() {
    let mut off = Vec::new();
    for (width, nav, cta) in BAR_TIERS {
        for theme in ThemeId::ALL {
            // A clean start: the app as the host creates it, no input
            // yet, and nothing focused.
            let mut h = Harness::window(egui::vec2(width, 800.0));
            h.app.gallery.tab = 3;
            h.app.gallery.theme = *theme;
            crate::gallery::apply_theme(*theme, &h.ctx);
            h.idle(2);
            let at = format!("at {width}pt in {}", theme.name());

            // What the bar holds at this tier, in the order it stands.
            let mut wanted: Vec<String> = Vec::new();
            if nav {
                wanted.extend(copy::TABS.iter().map(|label| format!("the {label} link")));
            }
            wanted.push("the search button".to_owned());
            wanted.push("the theme trigger".to_owned());
            if cta {
                wanted.push("the call to action".to_owned());
            }
            let controls = bar_controls(&h);
            let held: Vec<&str> = controls.iter().map(|(name, _)| name.as_str()).collect();
            if held != wanted {
                off.push(format!(
                    "{at} the bar holds {held:?} from left to right, not the tier's {wanted:?}"
                ));
                continue;
            }
            if let Some((role, name, _)) = &h.focus {
                off.push(format!("{at} starts with {role:?} {name:?} focused"));
                continue;
            }

            for (stop, (name, rect)) in controls.iter().enumerate() {
                h.press(egui::Key::Tab);
                match &h.focus {
                    Some((_, _, focused))
                        if (focused.min - rect.min).abs().max_elem() < 0.5
                            && (focused.max - rect.max).abs().max_elem() < 0.5 => {}
                    Some((role, label, focused)) => off.push(format!(
                        "{at} Tab press {} focused {role:?} {label:?} {focused:?}, \
                         not {name} {rect:?}",
                        stop + 1
                    )),
                    None => off.push(format!(
                        "{at} Tab press {} focused nothing, not {name} {rect:?}",
                        stop + 1
                    )),
                }
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// Where the bar's search button, theme trigger and call to action stand
/// in the light theme at each of the `BAR_TIERS`, as recorded before the
/// controls were built left to right to put Tab in their visual order:
/// the order they are created in changed, the places they stand in must
/// not. (The bar's centring and padding are pinned by
/// `the_bar_centres_its_wordmark_and_pads_its_ends`; this pins the
/// controls' own rects, to a fraction of a point.) The faces are the
/// kit's own, so the widths are the same on every machine.
///
/// `RECORDED` is a golden table, not a derived expectation: it holds
/// the rects as they stood, and it fails on any move, so a deliberate
/// change to the bar's geometry (`ACTIONS_GAP`, the button padding, the
/// faces or their sizes) must re-record it. Do that from this test's
/// failure output, which prints each control's new rect, in the table's
/// own form, beside the recorded one: check that the move is the one
/// intended, then copy the new values in.
#[test]
fn the_bars_controls_keep_their_places() {
    /// Below egui's rounding step of a 32nd of a point (about 0.031): it
    /// absorbs only the three-decimal rounding of the golden table's
    /// values, and any real move, which is a whole step or more, fails.
    const SLACK: f32 = 0.02;
    const SEARCH: &str = "the search button";
    const TRIGGER: &str = "the theme trigger";
    const CTA: &str = "the call to action";
    // The golden table, in the light theme:
    // (window width, control, [left, top, right, bottom])
    const RECORDED: [(f32, &str, [f32; 4]); 9] = [
        (320.0, SEARCH, [186.0, 24.0, 230.0, 68.0]),
        (320.0, TRIGGER, [246.0, 24.0, 290.0, 68.0]),
        (384.0, SEARCH, [189.281, 24.0, 233.281, 68.0]),
        (384.0, TRIGGER, [249.281, 24.0, 360.0, 68.0]),
        (900.0, SEARCH, [705.281, 24.0, 749.281, 68.0]),
        (900.0, TRIGGER, [765.281, 24.0, 876.0, 68.0]),
        (1280.0, SEARCH, [858.219, 24.0, 902.219, 68.0]),
        (1280.0, TRIGGER, [918.219, 24.0, 1113.0, 68.0]),
        (1280.0, CTA, [1129.0, 28.0, 1240.0, 64.0]),
    ];
    let mut off = Vec::new();
    let mut seen = 0;
    for (width, _, _) in BAR_TIERS {
        let mut h = Harness::window(egui::vec2(width, 800.0));
        h.app.gallery.tab = 3;
        h.app.gallery.theme = ThemeId::Light;
        crate::gallery::apply_theme(ThemeId::Light, &h.ctx);
        h.idle(2);
        for (name, rect) in bar_controls(&h) {
            let Some((_, _, [l, t, r, b])) = RECORDED
                .iter()
                .find(|(w, recorded, _)| *w == width && *recorded == name)
            else {
                continue;
            };
            seen += 1;
            let recorded = egui::Rect::from_min_max(egui::pos2(*l, *t), egui::pos2(*r, *b));
            if (rect.min - recorded.min).abs().max_elem() > SLACK
                || (rect.max - recorded.max).abs().max_elem() > SLACK
            {
                // In the table's own form, to three decimals, so that a
                // deliberate move can be copied in as it stands.
                let table = |rect: egui::Rect| {
                    format!(
                        "[{:.3}, {:.3}, {:.3}, {:.3}]",
                        rect.min.x, rect.min.y, rect.max.x, rect.max.y
                    )
                };
                off.push(format!(
                    "at {width}pt {name} stands at {}, not where it was recorded {}",
                    table(rect),
                    table(recorded)
                ));
            }
        }
    }
    assert_eq!(seen, RECORDED.len(), "every recorded control was found");
    assert!(off.is_empty(), "{}", off.join("\n"));
}

/// The bar's nav links: the `Link` nodes named after the panels, each
/// with its rect.
fn nav_links(h: &Harness) -> Vec<(String, egui::Rect)> {
    h.nodes
        .iter()
        .filter(|(role, name, _)| *role == Role::Link && copy::TABS.contains(&name.as_str()))
        .map(|(_, name, rect)| (format!("the {name} link"), *rect))
        .collect()
}

/// The wordmark: the one mesh painted level with the bar's `row` (the
/// logo is drawn as a mesh, the bar's text and controls as shapes of
/// their own). A second mesh there fails the lookup rather than
/// widening the wordmark.
fn wordmark(h: &Harness, row: egui::Rect) -> egui::Rect {
    let level: Vec<egui::Rect> = h
        .meshes
        .iter()
        .copied()
        .filter(|mesh| mesh.top() < row.bottom() && row.top() < mesh.bottom())
        .collect();
    assert_eq!(
        level.len(),
        1,
        "the bar paints one mesh, the wordmark: {level:?}"
    );
    level[0]
}

/// Every rect shape the last frame painted, with the clip it was
/// painted under.
fn painted_rects(h: &Harness) -> Vec<(egui::epaint::RectShape, egui::Rect)> {
    fn walk(
        shape: &egui::Shape,
        clip: egui::Rect,
        out: &mut Vec<(egui::epaint::RectShape, egui::Rect)>,
    ) {
        match shape {
            egui::Shape::Rect(rect) => out.push((rect.clone(), clip)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, clip, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.shapes {
        walk(&clipped.shape, clipped.clip_rect, &mut out);
    }
    out
}

/// Every straight line segment the last frame painted (`hline`, `vline`
/// and `line_segment` all paint one), with its stroke.
fn painted_segments(h: &Harness) -> Vec<([egui::Pos2; 2], egui::Stroke)> {
    fn walk(shape: &egui::Shape, out: &mut Vec<([egui::Pos2; 2], egui::Stroke)>) {
        match shape {
            egui::Shape::LineSegment { points, stroke } => out.push((*points, *stroke)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// The bar's pane: the topmost rect the last frame painted with the
/// bar's 1px `outline-variant` border at `radius-lg`.
fn bar_pane(h: &Harness, theme: ThemeId) -> egui::Rect {
    let border = ui_kit::components::top_app_bar::bar_border(theme);
    let radius = egui::CornerRadius::same(ui_kit::tokens::radii::RADIUS_LG as u8);
    painted_rects(h)
        .into_iter()
        .filter(|(shape, _)| shape.stroke == border && shape.corner_radius == radius)
        .map(|(shape, _)| shape.rect)
        .min_by(|a, b| a.top().total_cmp(&b.top()))
        .expect("the frame paints the bar's pane")
}

/// The theme trigger's icon as the last frame painted it: the box the
/// glyph sits in (the chevron on the labelled trigger, the tune icon
/// on the icon button), found among the painted strings inside the
/// trigger's rect. The rightmost, when there are several.
fn trigger_icon(h: &Harness, trigger: egui::Rect) -> egui::Rect {
    use ui_kit::icons::Icon;

    // The rect is the trigger's own, give or take rounding.
    const WITHIN: f32 = 1.0;
    let icons = [Icon::ExpandMore, Icon::Tune].map(|icon| icon.as_char().to_string());
    h.strings
        .iter()
        .filter(|(text, rect)| icons.contains(text) && trigger.expand(WITHIN).contains_rect(*rect))
        .map(|(_, rect)| *rect)
        .max_by(|a, b| a.right().total_cmp(&b.right()))
        .expect("the theme trigger paints its icon")
}

/// On the narrowest phones the theme trigger is its icon alone, named
/// for a screen reader by the menu's own label and nothing more, and a
/// tap on it still opens the menu.
#[test]
fn the_theme_trigger_is_its_icon_alone_on_the_narrowest_phones() {
    let mut h = Harness::window(egui::vec2(320.0, 640.0));
    // The Feedback panel paints no theme names of its own, so every
    // name on screen is the menu's.
    h.app.gallery.tab = 3;
    h.idle(3);
    h.touch_screen();
    assert!(
        h.nodes
            .iter()
            .any(|(role, name, _)| *role == Role::Button && name == copy::THEME_MENU),
        "the icon trigger is named {:?} and nothing more",
        copy::THEME_MENU
    );
    assert!(
        h.painted(copy::THEME_MENU).is_none(),
        "the trigger paints no label at 320pt"
    );
    let trigger = h
        .node(Role::Button, copy::THEME_MENU)
        .expect("the bar's theme trigger");
    h.tap(trigger.center());
    h.idle(2);
    assert!(
        h.painted(ThemeId::Dark.name()).is_some(),
        "a tap on the icon trigger opens the theme menu"
    );
}

/// Where the bar has room, the theme trigger names the live theme
/// after the menu's label; most phones' bars keep the bare label (the
/// one the menu test above taps), and the narrowest the icon alone.
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

/// The Region select on the Components panel, scrolled into the window's
/// upper half: its box's rect, a field gap under its label and
/// `CONTROL_HEIGHT` tall.
fn region_select(h: &mut Harness) -> egui::Rect {
    h.app.gallery.tab = 2;
    h.idle(3);
    h.touch_screen();
    // The band for the label's top edge, in the 855pt phone window. The
    // box ends about 66pt under the label's top, so at the band's lower
    // edge (420pt) it ends near 486pt, leaving over 350pt under it: room
    // for the popover (three rows of `OPTION_HEIGHT` and a small gap) to
    // open below the box. Its upper edge (280pt) keeps the label well
    // clear of the bar and the tab strip at the top.
    let label = h
        .scroll_to(copy::SELECT_REGION_LABEL, 280.0..420.0)
        .expect("the Region select scrolls into view");
    egui::Rect::from_min_size(
        egui::pos2(
            label.left(),
            label.bottom() + ui_kit::components::text_field::FIELD_GAP,
        ),
        egui::vec2(
            h.window.x - 2.0 * label.left(),
            ui_kit::components::text_field::CONTROL_HEIGHT,
        ),
    )
}

/// Whether the last frame painted `value` as a select's shown value: a
/// field padding in from the box's left edge, `box_left`. (The radio
/// group below paints the same names, but past its circles.)
fn shows_value(h: &Harness, value: &str, box_left: f32) -> bool {
    let x = box_left + ui_kit::components::text_field::FIELD_PAD_X;
    h.strings
        .iter()
        .any(|(painted, at)| painted == value && (at.left() - x).abs() < 0.5)
}

/// A tap on the Region select's box by touch opens its popover, and a
/// tap on plain, non-interactive text outside it closes it again with
/// the choice unchanged: the caption says a click outside closes the
/// list.
#[test]
fn a_touch_tap_on_plain_text_closes_the_open_select() {
    let mut h = Harness::phone();
    let select_box = region_select(&mut h);
    assert!(
        !egui::Popup::is_any_open(&h.ctx),
        "the select starts closed"
    );

    assert!(
        shows_value(&h, "Nordics", select_box.left()),
        "the select starts on Nordics"
    );

    h.tap(select_box.center());
    h.idle(2);
    assert!(
        egui::Popup::is_any_open(&h.ctx),
        "a tap on the select's box opens its popover"
    );

    // The group's own label: plain text, under no popover.
    let text = h
        .painted(copy::SELECTS_LABEL)
        .expect("the selects group's label is painted");
    assert_ne!(
        h.ctx.layer_id_at(text.center()).map(|layer| layer.order),
        Some(egui::Order::Foreground),
        "the label lies outside the popover"
    );
    h.tap(text.center());
    h.idle(2);
    assert!(
        !egui::Popup::is_any_open(&h.ctx),
        "a tap on plain text outside the popover closes it"
    );
    assert!(
        shows_value(&h, "Nordics", select_box.left()),
        "closing by a tap outside leaves the choice alone"
    );
}

/// A window the height of the Pixel's page (the emulator's 448pt width,
/// a little short of its 997pt screen once the system bars are taken
/// off): the Feedback page overflows it by a few points only, so at the
/// page's end the info banner and its dismiss control are still on
/// screen.
const PIXEL_PAGE: egui::Vec2 = egui::vec2(448.0, 940.0);

/// The Feedback page on [`PIXEL_PAGE`], swiped to its end: the offset
/// is the page's maximum, and a second swipe cannot take it further.
fn feedback_at_its_end() -> Harness {
    let mut h = Harness::window(PIXEL_PAGE);
    h.app.gallery.tab = 3;
    h.idle(3);
    h.touch_screen();
    let up = |h: &mut Harness| {
        h.swipe(egui::pos2(190.0, 700.0), egui::pos2(190.0, 100.0), 10);
        h.idle(90);
    };
    up(&mut h);
    let end = h.page_offset().y;
    assert!(
        end > 20.0,
        "the Feedback page must overflow the window: {end}"
    );
    up(&mut h);
    assert!(
        (h.page_offset().y - end).abs() < PIXEL_SNAP,
        "the first swipe must take the page to its end: {end} -> {}",
        h.page_offset().y
    );
    h
}

/// Where the last frame painted three marks on the Feedback page, top
/// edges: the banners' label above the banners, the restore button
/// among them, and the snackbars' label far below. `None` for a mark
/// the frame did not paint.
fn feedback_marks(h: &Harness) -> [Option<f32>; 3] {
    [
        copy::BANNERS_LABEL,
        copy::BANNERS_RESTORE,
        copy::SNACKBARS_LABEL,
    ]
    .map(|text| h.painted(text).map(|rect| rect.top()))
}

/// The marks of two frames are the same page: each is painted in both
/// at the same place, or in neither.
fn same_marks(a: [Option<f32>; 3], b: [Option<f32>; 3]) -> bool {
    a.iter().zip(&b).all(|pair| match pair {
        (Some(a), Some(b)) => (a - b).abs() < PIXEL_SNAP,
        (None, None) => true,
        _ => false,
    })
}

/// Tap the info banner's dismiss control, the topmost close glyph on the
/// page: the finger down and up again, a frame each, and no frame after
/// them (unlike [`dismiss_the_info_banner`], which settles and checks
/// the restore button), so the caller reads the frames that follow.
fn dismiss_info_banner(h: &mut Harness) {
    let close = ui_kit::icons::Icon::Close.as_char().to_string();
    let dismiss = h
        .painted(&close)
        .expect("the info banner's dismiss is on screen")
        .center();
    h.frame(finger_down(dismiss));
    h.frame(finger_up(dismiss));
}

/// Dismissing the info banner at the end of the Feedback page shortens
/// the page below the offset it is scrolled to. egui lays the content out
/// at the offset it stored, and clamps that offset only after the content
/// is placed, so the first frame with the shorter page draws it
/// over-scrolled: the "Banners" label half under the tab strip and blank
/// space below the content. It is the frame the Pixel then held on screen
/// (finding 3-A9). The page lays that frame out again, in the same frame,
/// so what reaches the screen is the page where it settles.
#[test]
fn a_banner_dismissed_at_the_pages_end_leaves_the_page_clamped_in_its_first_frame() {
    let mut h = feedback_at_its_end();
    let before = feedback_marks(&h);

    // The tap: the banner is still drawn in the frame that releases it,
    // and the restore button joins it, so the page grows.
    dismiss_info_banner(&mut h);
    assert_eq!(h.passes, 1, "the release grows the page and needs one pass");

    // The first frame with the banner gone: no input, no idle frames
    // before it is read.
    h.frame(Vec::new());
    let first = feedback_marks(&h);
    let first_passes = h.passes;

    h.idle(3);
    let settled = feedback_marks(&h);
    assert!(
        !same_marks(before, settled),
        "dismissing the banner must move the page: {before:?} -> {settled:?}"
    );
    assert!(
        same_marks(first, settled),
        "the first frame without the banner must paint the page where it settles: \
         {first:?} against {settled:?}"
    );
    assert_eq!(
        first_passes, 2,
        "the first frame with the shorter page takes a second pass"
    );
    assert_eq!(h.passes, 1, "an idle frame after it takes one pass");
}

/// Restoring the banners lengthens the page, so the offset it is at stays
/// inside it and nothing clamps: the first frame with the banners back is
/// already the page where it settles, in one pass, with no special case
/// for the restore button.
#[test]
fn restoring_the_banners_needs_no_second_pass() {
    let mut h = feedback_at_its_end();
    dismiss_info_banner(&mut h);
    h.idle(4);
    let dismissed_at = h.page_offset().y;
    let restore = h
        .painted(copy::BANNERS_RESTORE)
        .expect("the restore button is offered after a dismissal")
        .center();

    let multipass = h.multipass_frames;
    h.frame(finger_down(restore));
    h.frame(finger_up(restore));
    h.frame(Vec::new());
    let first = feedback_marks(&h);
    h.idle(3);
    assert!(
        h.painted(copy::BANNERS_RESTORE).is_none(),
        "the restore brings both banners back, so nothing is left to restore"
    );
    assert_eq!(
        h.multipass_frames, multipass,
        "no frame of the restore takes a second pass"
    );
    assert!(
        same_marks(first, feedback_marks(&h)),
        "the first frame with the banners back is the page where it settles"
    );
    assert!(
        (h.page_offset().y - dismissed_at).abs() < PIXEL_SNAP,
        "a longer page leaves the offset alone: {dismissed_at} -> {}",
        h.page_offset().y
    );
}

/// A reader's own scrolling of a page that does not change (a swipe past
/// the end and the fling back, the wheel up and down) takes exactly one
/// pass in every frame, the clamp at the end included.
///
/// This guards against a second pass on every scroll or clamp, not the
/// page's height gate: on an unchanged page the stored offset is already
/// clamped, so the gate's offset condition is never true here, and the
/// test passes with the height comparison deleted.
#[test]
fn scrolling_an_unchanged_page_takes_one_pass_per_frame() {
    let mut h = Harness::window(PIXEL_PAGE);
    h.app.gallery.tab = 3;
    h.idle(3);
    h.touch_screen();
    h.idle(3);
    let frames = h.frames;
    let multipass = h.multipass_frames;

    // Up past the end with a fling, then back down to the top.
    h.swipe(egui::pos2(190.0, 700.0), egui::pos2(190.0, 100.0), 10);
    h.idle(90);
    let end = h.page_offset().y;
    assert!(
        end > 20.0,
        "the swipe must scroll the page to its end: {end}"
    );
    h.swipe(egui::pos2(190.0, 200.0), egui::pos2(190.0, 800.0), 10);
    h.idle(90);
    assert!(
        h.page_offset().y < end - 20.0,
        "the swipe back must scroll the page up: {end} -> {}",
        h.page_offset().y
    );
    // And the wheel, down and up again.
    h.wheel_page(30, -10.0);
    h.wheel_page(30, 10.0);

    assert!(
        200 < h.frames - frames,
        "the scrolling must run a few hundred frames: {}",
        h.frames - frames
    );
    assert_eq!(
        h.multipass_frames,
        multipass,
        "{} of the {} frames of scrolling took a second pass",
        h.multipass_frames - multipass,
        h.frames - frames
    );
}

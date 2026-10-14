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
    /// The bounds of every mesh the last frame painted.
    meshes: Vec<egui::Rect>,
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
            meshes: Vec::new(),
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
        self.meshes = painted_meshes(&out.shapes);
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

/// The bar keeps its working controls clear of the rest of its row
/// from the narrowest phones to a laptop (320, the first iPhone SE and
/// the small Androids; 600, the compact breakpoint; 768 and 834, iPad
/// mini and iPad Air portrait; 1024, iPad landscape) with the longest
/// theme name live: the theme trigger and the search button keep at
/// least the bar's action gap from every nav link and from the
/// wordmark. Touching counts as crowding: laid out without measuring,
/// the search button came within half a point of the last nav link at
/// 1024.
#[test]
fn the_bar_keeps_its_controls_clear_of_the_nav_links_and_the_wordmark() {
    let longest = *ThemeId::ALL
        .iter()
        .max_by_key(|id| id.name().len())
        .expect("the gallery's themes");
    let gap = ui_kit::components::top_app_bar::ACTIONS_GAP;
    let mut crowded = Vec::new();
    for width in [320.0, 600.0, 768.0, 834.0, 1024.0] {
        let mut h = Harness::window(egui::vec2(width, 800.0));
        h.app.gallery.theme = longest;
        crate::gallery::apply_theme(longest, &h.ctx);
        h.idle(2);
        let trigger = h
            .node(Role::Button, copy::THEME_MENU)
            .expect("the bar's theme trigger");
        let search = h
            .node(Role::Button, copy::SEARCH_BUTTON_NAME)
            .expect("the bar's search button");
        let mut neighbours = nav_links(&h);
        neighbours.push(("the wordmark".to_owned(), wordmark(&h, trigger)));
        for (control, rect) in [("theme trigger", trigger), ("search button", search)] {
            let reach = rect.expand2(egui::vec2(gap, 0.0));
            for (name, neighbour) in &neighbours {
                if reach.intersects(*neighbour) {
                    crowded.push(format!(
                        "at {width}pt the {control} {rect:?} crowds {name} {neighbour:?}"
                    ));
                }
            }
        }
    }
    assert!(crowded.is_empty(), "{}", crowded.join("\n"));
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

//! Headless input tests for the gallery chrome: synthetic taps through
//! the real `chrome()` at phone dimensions on ONE persistent context,
//! asserting the interactive chrome actually responds. These exist to
//! catch "the UI renders but input never lands" regressions on any
//! platform, headlessly. The context must persist across frames: egui
//! resolves a click from a press in one pass and its release in a later
//! pass of the same context.

use eframe::egui;

use crate::gallery::Gallery;

/// A persistent-context harness around the gallery chrome at phone
/// size. `frame` advances one pass with the given events.
struct Harness {
    ctx: egui::Context,
    gallery: Gallery,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        ui_kit::icon::install(&ctx);
        for id in ui_kit::theme::ThemeId::ALL {
            ui_kit::typography::install(*id, &ctx);
        }
        ui_kit::theme::ThemeId::Light.apply(&ctx);
        Self {
            ctx,
            gallery: Gallery::new(),
        }
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        let out = self.ctx.run_ui(
            egui::RawInput {
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(384.0, 855.0),
                )),
                time: Some(1.0),
                ..Default::default()
            },
            |ui| {
                let viewport = ui.ctx().input(|i| i.viewport_rect());
                let canvas = self.gallery.theme.colors().background;
                ui.painter().rect_filled(viewport, 0.0, canvas);
                self.gallery.chrome(ui);
            },
        );
        out.drop_without_applying_deltas();
    }

    /// Warm the layout for a few passes (sizing passes settle, widget
    /// ids register), then tap: move, press, release.
    fn tap(&mut self, pos: (f32, f32)) {
        let (x, y) = pos;
        for _ in 0..3 {
            self.frame(vec![]);
        }
        self.frame(vec![egui::Event::PointerMoved(egui::pos2(x, y))]);
        self.frame(vec![egui::Event::PointerButton {
            pos: egui::pos2(x, y),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }]);
        self.frame(vec![egui::Event::PointerButton {
            pos: egui::pos2(x, y),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);
    }
}

#[test]
fn tapping_the_foundations_tab_switches_panels() {
    // Scan the tab strip's y at phone width: headless chrome carries no
    // safe-area insets, so the strip sits higher than on device.
    for y in [90.0, 110.0, 130.0, 150.0, 170.0, 190.0, 210.0, 230.0] {
        let mut h = Harness::new();
        h.tap((120.0, y));
        if h.gallery.tab == 1 {
            return; // found the strip; the tap works
        }
    }
    panic!("no tap y switched the Foundations tab: the chrome eats input");
}

/// The wheel path must scroll the page: synthetic MouseWheel events
/// move the main scroll area's offset.
#[test]
fn wheel_events_scroll_the_page() {
    let mut h = Harness::new();
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let before = h.gallery.last_scroll_offset.y;
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
    let after = h.gallery.last_scroll_offset.y;
    assert!(
        (after - before).abs() > 1.0,
        "wheel must scroll: before {before} after {after}"
    );
}

/// The touch-scroll shim: finger drags through `filter_input` move the
/// page's scroll offset.
#[test]
fn touch_drag_scrolls_through_the_shim() {
    let ctx = egui::Context::default();
    ui_kit::icon::install(&ctx);
    for id in ui_kit::theme::ThemeId::ALL {
        ui_kit::typography::install(*id, &ctx);
    }
    ui_kit::theme::ThemeId::Light.apply(&ctx);
    let mut gallery = Gallery::new();
    let frame = |ctx: &egui::Context, gallery: &mut Gallery, events: Vec<egui::Event>| {
        let mut app = crate::test_app(std::mem::replace(gallery, Gallery::new()), ctx.clone());
        let mut raw = egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(384.0, 855.0),
            )),
            time: Some(1.0),
            ..Default::default()
        };
        app.filter_input_shim(&mut raw);
        let out = ctx.run_ui(raw, |ui| {
            let viewport = ui.ctx().input(|i| i.viewport_rect());
            let canvas = app.gallery.theme.colors().background;
            ui.painter().rect_filled(viewport, 0.0, canvas);
            app.gallery.chrome(ui);
        });
        out.drop_without_applying_deltas();
        *gallery = app.gallery;
    };
    // warm up: the chrome registers the scroll id
    for _ in 0..3 {
        frame(&ctx, &mut gallery, vec![]);
    }
    let before = gallery.last_scroll_offset.y;
    // a finger drag up: Start, Moves, End
    frame(
        &ctx,
        &mut gallery,
        vec![touch(egui::TouchPhase::Start, 300.0)],
    );
    let mut y = 280.0f32;
    while y > 200.0 {
        frame(&ctx, &mut gallery, vec![touch(egui::TouchPhase::Move, y)]);
        y -= 10.0;
    }
    frame(
        &ctx,
        &mut gallery,
        vec![touch(egui::TouchPhase::End, 200.0)],
    );
    let after = gallery.last_scroll_offset.y;
    assert!(
        after > before + 10.0,
        "the touch drag must scroll the page: {before} -> {after}"
    );
}

fn touch(phase: egui::TouchPhase, y: f32) -> egui::Event {
    egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase,
        pos: egui::pos2(200.0, y),
        force: None,
    }
}

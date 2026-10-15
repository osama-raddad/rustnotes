use super::{Metrics, Palette, WindowClass};
use egui::{pos2, vec2, Align, Layout, Rect, Stroke, Ui, UiBuilder};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BarLine {
    /// Hairline on the bottom edge (a top bar).
    Bottom,
    /// Hairline on the top edge (a bottom bar).
    Top,
}

/// Paint a bar into `rect`, extending its fill `extend_up` / `extend_down`
/// points past the content area so the status-bar and home-indicator strips
/// take the bar colour, then run `add` in a child UI covering `rect`.
pub fn bar(
    ui: &mut Ui,
    rect: Rect,
    extend_up: f32,
    extend_down: f32,
    line: BarLine,
    pal: &Palette,
    add: impl FnOnce(&mut Ui),
) {
    let fill = Rect::from_min_max(
        pos2(rect.left(), rect.top() - extend_up),
        pos2(rect.right(), rect.bottom() + extend_down),
    );
    let p = ui.painter();
    p.rect_filled(fill, 0.0, pal.bar);
    let y = match line {
        BarLine::Bottom => rect.bottom() - 0.5,
        BarLine::Top => rect.top() + 0.5,
    };
    p.line_segment(
        [pos2(rect.left(), y), pos2(rect.right(), y)],
        Stroke::new(1.0, pal.hairline),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    add(&mut child);
}

/// The rects a scaffold used, in points.
#[derive(Clone, Copy, Debug)]
pub struct ScaffoldRects {
    pub top_bar: Option<Rect>,
    pub bottom_bar: Option<Rect>,
    pub body: Rect,
}

type Slot<'a> = Option<Box<dyn FnOnce(&mut Ui) + 'a>>;

/// Safe-area-aware frame every screen renders into.
///
/// The host carries two duties egui cannot express for us:
///
/// * Fold the soft keyboard into `RawInput::safe_area_insets` — egui-winit
///   only feeds the notch and home-indicator insets, so a custom shell must
///   add the keyboard height for `content_rect()` to exclude it.
/// * Hand [`Scaffold::show`] an edge-to-edge root `Ui` whose `max_rect` is
///   the viewport (eframe's root). A navigation slide may translate that
///   root sideways — the scaffold follows via `full.min - viewport.min` —
///   but a root that is already inset (a panel, a margined container)
///   would double-apply the safe areas. As a backstop the derived content
///   rect is clamped into the root.
///
/// Between those duties, `content_rect()` drives the layout, top to bottom:
/// top bar, body, bottom bar, with an optional floating action button over
/// the body's corner. Its horizontal insets are mirrored first (the larger
/// of the left and right insets on both sides), so a cutout on one side of
/// a landscape phone leaves the layout centred; the margins beside it are
/// the canvas the scaffold paints behind everything.
pub struct Scaffold<'a> {
    pal: &'a Palette,
    m: &'a Metrics,
    top_bar: Slot<'a>,
    bottom_bar: Slot<'a>,
    fab: Slot<'a>,
    background: Option<egui::Color32>,
    limit_body_width: bool,
}

impl<'a> Scaffold<'a> {
    pub fn new(pal: &'a Palette, m: &'a Metrics) -> Self {
        Scaffold {
            pal,
            m,
            top_bar: None,
            bottom_bar: None,
            fab: None,
            background: None,
            limit_body_width: false,
        }
    }

    pub fn top_bar(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.top_bar = Some(Box::new(f));
        self
    }

    pub fn bottom_bar(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom_bar = Some(Box::new(f));
        self
    }

    pub fn fab(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.fab = Some(Box::new(f));
        self
    }

    pub fn background(mut self, c: egui::Color32) -> Self {
        self.background = Some(c);
        self
    }

    /// Center the body and cap it at `Metrics::max_content_width`.
    pub fn limit_body_width(mut self) -> Self {
        self.limit_body_width = true;
        self
    }

    pub fn show(self, root: &mut Ui, body: impl FnOnce(&mut Ui, WindowClass)) -> ScaffoldRects {
        let ctx = root.ctx().clone();
        // Relative to the UI we were given: a navigation transition hands us
        // a viewport-sized rect shifted sideways, and everything follows it.
        let full = root.max_rect();
        debug_assert_eq!(
            full.min.y,
            ctx.viewport_rect().min.y,
            "Scaffold expects an edge-to-edge root: a nav slide shifts it \
             horizontally only, so a vertical offset means the host already \
             inset the root and the safe areas will not line up"
        );
        let shift = full.min - ctx.viewport_rect().min;
        let viewport = ctx.viewport_rect().translate(shift);
        let mut content = ctx.content_rect().translate(shift);
        // The horizontal insets are mirrored, as the gallery's chrome does: a
        // cutout on one side of a landscape phone would otherwise push the
        // bars and the body to the other side, so both sides keep `side`, the
        // larger of the two. The margin it leaves beside a bar or the body
        // is the canvas painted behind them. Capped at half the window, so a
        // window narrower than its two gutters has an empty body rather than
        // an inverted one.
        let side = (content.min.x - viewport.min.x)
            .max(viewport.max.x - content.max.x)
            .min(viewport.width() / 2.0);
        content.min.x = viewport.min.x + side;
        content.max.x = viewport.max.x - side;
        // Backstop for a host that hands us an already-inset root anyway:
        // clamp so the safe areas can neither apply twice nor spill outside
        // the root we were given.
        content.min = content.min.max(full.min);
        content.max = content.max.min(full.max);
        let class = WindowClass::of(&ctx);
        let (pal, m) = (self.pal, self.m);

        root.painter()
            .rect_filled(full, 0.0, self.background.unwrap_or(pal.bg));

        let mut body_rect = content;
        let mut rects = ScaffoldRects {
            top_bar: None,
            bottom_bar: None,
            body: content,
        };

        if let Some(f) = self.top_bar {
            let r = Rect::from_min_size(content.min, vec2(content.width(), m.bar_height));
            bar(
                root,
                r,
                content.top() - full.top(),
                0.0,
                BarLine::Bottom,
                pal,
                f,
            );
            body_rect.min.y = r.bottom();
            rects.top_bar = Some(r);
        }
        if let Some(f) = self.bottom_bar {
            let top = (content.bottom() - m.bar_height).max(body_rect.min.y);
            let r = Rect::from_min_max(pos2(content.left(), top), content.right_bottom());
            bar(
                root,
                r,
                0.0,
                full.bottom() - content.bottom(),
                BarLine::Top,
                pal,
                f,
            );
            body_rect.max.y = r.top();
            rects.bottom_bar = Some(r);
        }

        if self.limit_body_width && body_rect.width() > m.max_content_width {
            let x = body_rect.center().x - m.max_content_width / 2.0;
            body_rect = Rect::from_min_max(
                pos2(x, body_rect.min.y),
                pos2(x + m.max_content_width, body_rect.max.y),
            );
        }
        rects.body = body_rect;

        let mut body_ui = root.new_child(
            UiBuilder::new()
                .max_rect(body_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        body_ui.set_clip_rect(body_rect.intersect(root.clip_rect()));
        body(&mut body_ui, class);

        if let Some(f) = self.fab {
            let inset = m.sp(3);
            // Clamp into the body so a body narrower or shorter than the
            // button plus its insets keeps the button inside, degenerate as
            // that may look.
            let min = (body_rect.right_bottom() - vec2(m.fab_size + inset, m.fab_size + inset))
                .max(body_rect.left_top() + vec2(inset, inset));
            let r = Rect::from_min_size(min, vec2(m.fab_size, m.fab_size));
            let r = Rect::from_min_max(
                r.min,
                r.max.min(body_rect.right_bottom() - vec2(inset, inset)),
            );
            let mut fab_ui = root.new_child(UiBuilder::new().max_rect(r));
            fab_ui.set_clip_rect(r.intersect(root.clip_rect()));
            f(&mut fab_ui);
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput, SafeAreaInsets, Vec2};

    fn raw(size: Vec2, top: f32, bottom: f32) -> RawInput {
        raw_sides(size, 0.0, 0.0, top, bottom)
    }

    fn raw_sides(size: Vec2, left: f32, right: f32, top: f32, bottom: f32) -> RawInput {
        RawInput {
            time: Some(0.0),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            safe_area_insets: Some(SafeAreaInsets(egui::epaint::MarginF32 {
                left,
                right,
                top,
                bottom,
            })),
            ..Default::default()
        }
    }

    fn run<T>(raw: RawInput, mut f: impl FnMut(&mut Ui) -> T) -> T {
        let ctx = egui::Context::default();
        let mut out = None;
        ctx.run_ui(raw, |root| out = Some(f(root)))
            .drop_without_applying_deltas();
        out.expect("the callback ran")
    }

    /// The bars sit between the safe areas, the body between the bars.
    #[test]
    fn bars_and_body_honour_safe_areas() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        let r = run(raw(vec2(400.0, 800.0), 59.0, 34.0), |root| {
            Scaffold::new(&pal, &m)
                .top_bar(|_| {})
                .bottom_bar(|_| {})
                .show(root, |_, _| {})
        });
        assert_eq!(
            r.top_bar.expect("top bar"),
            Rect::from_min_size(pos2(0.0, 59.0), vec2(400.0, m.bar_height))
        );
        assert_eq!(
            r.bottom_bar.expect("bottom bar"),
            Rect::from_min_max(pos2(0.0, 710.0), pos2(400.0, 766.0))
        );
        assert_eq!(
            r.body,
            Rect::from_min_max(pos2(0.0, 115.0), pos2(400.0, 710.0))
        );
    }

    /// A cutout on one side of a landscape phone insets the other side by the
    /// same amount: both sides keep the larger of the two insets, whichever
    /// side it is on, so the bars and the body stand centred in the window.
    #[test]
    fn horizontal_insets_are_mirrored() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        // A Pixel in landscape, its cutout 53 pt wide.
        let size = vec2(997.0, 448.0);
        for (left, right) in [
            (53.0, 0.0),
            (0.0, 53.0),
            (53.0, 20.0),
            (20.0, 53.0),
            (53.0, 53.0),
        ] {
            let r = run(raw_sides(size, left, right, 0.0, 0.0), |root| {
                Scaffold::new(&pal, &m)
                    .top_bar(|_| {})
                    .bottom_bar(|_| {})
                    .show(root, |_, _| {})
            });
            let at = format!("insets {left} / {right}");
            assert_eq!(
                r.top_bar.expect("top bar"),
                Rect::from_min_max(pos2(53.0, 0.0), pos2(944.0, 56.0)),
                "{at}"
            );
            assert_eq!(
                r.bottom_bar.expect("bottom bar"),
                Rect::from_min_max(pos2(53.0, 392.0), pos2(944.0, 448.0)),
                "{at}"
            );
            assert_eq!(
                r.body,
                Rect::from_min_max(pos2(53.0, 56.0), pos2(944.0, 392.0)),
                "{at}"
            );
        }
    }

    /// The mirrored body is still capped and centred in what is left, so it
    /// stands the same distance from either edge of the window.
    #[test]
    fn mirrored_inset_leaves_a_centred_capped_body() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        let size = vec2(997.0, 448.0);
        let r = run(raw_sides(size, 53.0, 0.0, 0.0, 0.0), |root| {
            Scaffold::new(&pal, &m)
                .limit_body_width()
                .show(root, |_, _| {})
        });
        assert_eq!(r.body.width(), m.max_content_width);
        assert_eq!(r.body.left(), size.x - r.body.right());
    }

    /// A window narrower than its two gutters has an empty body, not an
    /// inverted one: the mirror may not push the edges past each other.
    #[test]
    fn mirrored_inset_in_a_narrow_window_does_not_invert() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        let r = run(raw_sides(vec2(60.0, 400.0), 53.0, 0.0, 0.0, 0.0), |root| {
            Scaffold::new(&pal, &m)
                .top_bar(|_| {})
                .fab(|_| {})
                .show(root, |_, _| {})
        });
        let bar = r.top_bar.expect("top bar");
        for rect in [r.body, bar] {
            assert!(
                rect.width() >= 0.0 && rect.height() >= 0.0,
                "not inverted: {rect:?}"
            );
        }
    }

    /// `limit_body_width` centers the capped body in a wide window.
    #[test]
    fn limited_body_width_centers() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        let r = run(raw(vec2(1000.0, 800.0), 0.0, 0.0), |root| {
            Scaffold::new(&pal, &m)
                .limit_body_width()
                .show(root, |_, _| {})
        });
        assert_eq!(
            r.body,
            Rect::from_min_max(pos2(180.0, 0.0), pos2(820.0, 800.0))
        );
    }

    /// A zero-size first frame produces degenerate but finite rects, with
    /// the bars' overlap clamp keeping the body empty rather than inverted.
    #[test]
    fn zero_size_first_frame_is_survivable() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        let r = run(raw(vec2(0.0, 0.0), 0.0, 0.0), |root| {
            Scaffold::new(&pal, &m)
                .top_bar(|_| {})
                .bottom_bar(|_| {})
                .fab(|_| {})
                .show(root, |_, _| {})
        });
        let mut rects = vec![r.body];
        rects.extend(r.top_bar);
        rects.extend(r.bottom_bar);
        for rect in rects {
            assert!(
                rect.min.x.is_finite()
                    && rect.min.y.is_finite()
                    && rect.max.x.is_finite()
                    && rect.max.y.is_finite(),
                "degenerate frame must not poison rects: {rect:?}"
            );
        }
    }

    /// The FAB sits in the body's bottom-right corner, clear of the insets.
    #[test]
    fn fab_sits_in_the_body_corner() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        run(raw(vec2(400.0, 800.0), 0.0, 0.0), |root| {
            let mut fab = None;
            let rects = Scaffold::new(&pal, &m)
                .fab(|ui| fab = Some(ui.max_rect()))
                .show(root, |_, _| {});
            let fab = fab.expect("fab drawn");
            let body = rects.body;
            assert_eq!(
                fab,
                Rect::from_min_max(pos2(328.0, 728.0), pos2(384.0, 784.0))
            );
            assert!(fab.min.x >= body.min.x && fab.max.x <= body.max.x);
            assert!(fab.min.y >= body.min.y && fab.max.y <= body.max.y);
        });
    }

    /// A body narrower than the button plus its insets still contains it.
    #[test]
    fn fab_stays_inside_a_degenerate_body() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        run(raw(vec2(60.0, 400.0), 0.0, 0.0), |root| {
            let mut fab = None;
            let rects = Scaffold::new(&pal, &m)
                .fab(|ui| fab = Some(ui.max_rect()))
                .show(root, |_, _| {});
            let fab = fab.expect("fab drawn");
            let body = rects.body;
            assert!(fab.min.x >= body.min.x, "left in: {:?}", fab);
            assert!(fab.max.x <= body.max.x, "right in: {:?}", fab);
            assert!(fab.min.y >= body.min.y, "top in: {:?}", fab);
            assert!(fab.max.y <= body.max.y, "bottom in: {:?}", fab);
        });
    }

    /// A root that is already inset (a side pane) gets its content clamped
    /// into the root instead of double-applying the safe areas.
    #[test]
    fn inset_root_does_not_double_apply_safe_areas() {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        run(raw(vec2(400.0, 800.0), 59.0, 34.0), |root| {
            let inner = Rect::from_min_max(pos2(100.0, 0.0), pos2(300.0, 800.0));
            let mut pane = root.new_child(UiBuilder::new().max_rect(inner));
            let body = Scaffold::new(&pal, &m).show(&mut pane, |_, _| {}).body;
            assert_eq!(
                body,
                Rect::from_min_max(pos2(100.0, 59.0), pos2(300.0, 766.0))
            );
        });
    }
}

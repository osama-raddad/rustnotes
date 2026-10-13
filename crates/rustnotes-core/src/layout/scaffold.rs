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
/// Reads `content_rect()` (which already excludes status bar, home indicator
/// and the keyboard) and lays out, top to bottom: top bar, body, bottom bar,
/// with an optional floating action button over the body's corner.
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
        let full = ctx.viewport_rect();
        let content = ctx.content_rect();
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
            bar(root, r, content.top() - full.top(), 0.0, BarLine::Bottom, pal, f);
            body_rect.min.y = r.bottom();
            rects.top_bar = Some(r);
        }
        if let Some(f) = self.bottom_bar {
            let top = (content.bottom() - m.bar_height).max(body_rect.min.y);
            let r = Rect::from_min_max(pos2(content.left(), top), content.right_bottom());
            bar(root, r, 0.0, full.bottom() - content.bottom(), BarLine::Top, pal, f);
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
            let r = Rect::from_min_size(
                pos2(
                    body_rect.right() - inset - m.fab_size,
                    body_rect.bottom() - inset - m.fab_size,
                ),
                vec2(m.fab_size, m.fab_size),
            );
            let mut fab_ui = root.new_child(UiBuilder::new().max_rect(r));
            f(&mut fab_ui);
        }
        rects
    }
}

use super::{Metrics, Palette};
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Rect, Sense, Stroke, Ui};

/// One destination in a [`bottom_nav`]: a painted icon and a label.
pub struct NavItem {
    pub label: &'static str,
    /// Paint a 24-point icon centred in the given square.
    pub icon: fn(&Painter, Rect, Color32),
}

/// Equal-width tabs filling the bar the scaffold's `bottom_bar` slot gave us.
/// Each tab is at least `touch_target` wide and the full bar tall. Returns the
/// index of a tab tapped this frame that is not already selected.
pub fn bottom_nav(
    ui: &mut Ui,
    items: &[NavItem],
    selected: usize,
    pal: &Palette,
    m: &Metrics,
) -> Option<usize> {
    let bar = ui.max_rect();
    if items.is_empty() {
        return None;
    }
    let w = bar.width() / items.len() as f32;
    let mut tapped = None;
    for (i, item) in items.iter().enumerate() {
        let cell = Rect::from_min_size(
            pos2(bar.left() + w * i as f32, bar.top()),
            vec2(w.max(m.touch_target), bar.height()),
        );
        let resp = ui.interact(cell, Id::new(("layout.tab", i)), Sense::click());
        let active = i == selected;
        let color = if active { pal.accent } else { pal.text_weak };
        let p = ui.painter();
        let icon_c = pos2(cell.center().x, cell.top() + m.sp(2) + 12.0);
        (item.icon)(p, Rect::from_center_size(icon_c, vec2(24.0, 24.0)), color);
        p.text(
            pos2(cell.center().x, cell.bottom() - m.sp(1)),
            Align2::CENTER_BOTTOM,
            item.label,
            m.font(m.text.caption),
            color,
        );
        if resp.clicked() && !active {
            tapped = Some(i);
        }
    }
    tapped
}

/// Page with text lines.
pub fn icon_notes(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(2.0, c);
    p.rect_stroke(r.shrink2(vec2(3.0, 1.0)), 3.0, s, egui::StrokeKind::Inside);
    for k in 0..3 {
        let y = r.top() + 8.0 + k as f32 * 5.0;
        p.line_segment([pos2(r.left() + 7.0, y), pos2(r.right() - 7.0, y)], s);
    }
}

/// Two slider tracks.
pub fn icon_settings(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(2.0, c);
    for (k, knob) in [0.35_f32, 0.7].into_iter().enumerate() {
        let y = r.top() + 7.0 + k as f32 * 10.0;
        p.line_segment([pos2(r.left() + 2.0, y), pos2(r.right() - 2.0, y)], s);
        p.circle_filled(pos2(r.left() + 2.0 + (r.width() - 4.0) * knob, y), 4.0, c);
    }
}

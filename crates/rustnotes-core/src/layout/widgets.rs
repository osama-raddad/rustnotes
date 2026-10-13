use super::{Metrics, Palette};
use egui::{pos2, vec2, Align2, Id, Rect, Response, Sense, Stroke, Ui};

/// Chevron on the left of a bar. The hit area is the full bar height by
/// `touch_target + 4` wide.
pub fn back_button(ui: &mut Ui, pal: &Palette, m: &Metrics) -> Response {
    let bar = ui.max_rect();
    let hit = Rect::from_min_size(bar.left_top(), vec2(m.touch_target + 4.0, bar.height()));
    let resp = ui.interact(hit, Id::new("layout.back"), Sense::click());
    let c = pos2(bar.left() + 28.0, bar.center().y);
    let stroke = Stroke::new(3.0, pal.text);
    let p = ui.painter();
    p.line_segment([c + vec2(-6.0, -10.0), c + vec2(-12.0, 0.0)], stroke);
    p.line_segment([c + vec2(-12.0, 0.0), c + vec2(-6.0, 10.0)], stroke);
    resp
}

/// Rounded accent "+" button on the right of a bar.
pub fn add_button(ui: &mut Ui, pal: &Palette, m: &Metrics) -> Response {
    let bar = ui.max_rect();
    let d = m.touch_target - 8.0;
    let btn = Rect::from_center_size(
        pos2(bar.right() - m.sp(3) - d / 2.0, bar.center().y),
        vec2(d, d),
    );
    let resp = ui.interact(btn.expand(8.0), Id::new("layout.add"), Sense::click());
    let p = ui.painter();
    p.rect_filled(btn, d * 0.35, pal.accent);
    let c = btn.center();
    let arm = 9.0;
    let stroke = Stroke::new(2.5, pal.on_accent);
    p.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], stroke);
    p.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], stroke);
    resp
}

/// Left-aligned bar title starting `left` points from the bar's left edge.
/// Returns the text's rect so callers can place a subtitle after it.
pub fn bar_title(ui: &mut Ui, left: f32, text: &str, pal: &Palette, m: &Metrics) -> Rect {
    let bar = ui.max_rect();
    ui.painter().text(
        pos2(bar.left() + left, bar.center().y),
        Align2::LEFT_CENTER,
        text,
        m.font(m.text.title),
        pal.text,
    )
}

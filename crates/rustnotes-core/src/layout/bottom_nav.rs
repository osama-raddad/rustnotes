use super::{Metrics, Palette};
use egui::{pos2, vec2, Align2, Color32, Painter, Rect, Sense, Stroke, Ui};

/// One destination in a [`bottom_nav`]: a painted icon and a label.
// TODO(framework-extraction): `label: &'static str` precludes localization
// and runtime-built labels; take an owned/`Arc<str>` label at extraction
// time.
pub struct NavItem {
    pub label: &'static str,
    /// Paint a 24-point icon centred in the given square.
    pub icon: fn(&Painter, Rect, Color32),
}

/// Equal-width tabs filling the bar the scaffold's `bottom_bar` slot gave
/// us: `bar.width() / n` each, the full bar tall. When the bar is narrower
/// than `touch_target` per tab (a split-screen remainder), the tabs narrow
/// instead of overlapping each other or spilling past the bar's edge.
/// Returns the index of a tab tapped this frame that is not already
/// selected.
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
            vec2(w, bar.height()),
        );
        // Scoped to this bar instance so a second `bottom_nav` in the same
        // frame cannot collide with this one.
        let resp = ui.interact(cell, ui.auto_id_with(("layout.tab", i)), Sense::click());
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
// TODO(framework-extraction): a notes-page icon is an app concept; keep the
// `fn(&Painter, Rect, Color32)` icon type here and move the painter to the
// app crate.
pub fn icon_notes(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(2.0, c);
    p.rect_stroke(r.shrink2(vec2(3.0, 1.0)), 3.0, s, egui::StrokeKind::Inside);
    for k in 0..3 {
        let y = r.top() + 8.0 + k as f32 * 5.0;
        p.line_segment([pos2(r.left() + 7.0, y), pos2(r.right() - 7.0, y)], s);
    }
}

/// Two slider tracks.
// TODO(framework-extraction): a settings icon is an app concept; move it to
// the app crate with `icon_notes`.
pub fn icon_settings(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(2.0, c);
    for (k, knob) in [0.35_f32, 0.7].into_iter().enumerate() {
        let y = r.top() + 7.0 + k as f32 * 10.0;
        p.line_segment([pos2(r.left() + 2.0, y), pos2(r.right() - 2.0, y)], s);
        p.circle_filled(pos2(r.left() + 2.0 + (r.width() - 4.0) * knob, y), 4.0, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Event, Modifiers, PointerButton, Pos2, RawInput, UiBuilder};

    fn icon(_: &Painter, _: Rect, _: Color32) {}

    fn items() -> Vec<NavItem> {
        vec![NavItem { label: "A", icon }, NavItem { label: "B", icon }]
    }

    fn raw(t: f64, events: Vec<Event>) -> RawInput {
        RawInput {
            time: Some(t),
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 56.0))),
            events,
            ..Default::default()
        }
    }

    /// Draw a two-tab bar in an 80-point slot (two 40-point tabs — below the
    /// 48-point touch target) and click at `p`; a tap needs one frame to
    /// register the tabs, one to press and one to release.
    fn tap(ctx: &egui::Context, p: Pos2, selected: usize) -> Option<usize> {
        let (pal, m) = (Palette::LIGHT, Metrics::DEFAULT);
        let bar = egui::Rect::from_min_size(Pos2::ZERO, vec2(80.0, 56.0));
        let run = |t: f64, events: Vec<Event>| -> Option<usize> {
            let mut tapped = None;
            ctx.run_ui(raw(t, events), |root| {
                let mut ui = root.new_child(UiBuilder::new().max_rect(bar));
                tapped = bottom_nav(&mut ui, &items(), selected, &pal, &m);
            })
            .drop_without_applying_deltas();
            tapped
        };
        run(1.0, vec![]); // register the tab rects
        run(2.0, vec![Event::PointerMoved(p)]);
        run(
            3.0,
            vec![Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
        );
        run(
            // egui only counts a release as a click within
            // `max_click_duration` (0.8 s), so keep the press short.
            3.2,
            vec![
                Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerGone,
            ],
        )
    }

    /// Cells tile the bar exactly: a tap inside the narrow bar lands on a
    /// tab, a tap outside it (where the old 48-point cells overflowed) hits
    /// nothing.
    #[test]
    fn narrow_bar_cells_tile_without_overflowing() {
        let ctx = egui::Context::default();
        assert_eq!(
            tap(&ctx, pos2(60.0, 28.0), 0),
            Some(1),
            "second tab, inside"
        );
        // Tap the first tab while the SECOND is selected: a hit on a
        // not-already-selected tab.
        assert_eq!(tap(&ctx, pos2(20.0, 28.0), 1), Some(0), "first tab, inside");
        assert_eq!(
            tap(&ctx, pos2(90.0, 28.0), 0),
            None,
            "10 pt past the 80 pt bar is outside every cell"
        );
    }
}

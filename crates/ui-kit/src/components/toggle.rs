//! Toggle: the design system's takes-effect-at-once switch, the looks
//! of `components/Toggle` and the rules its preview carries (the
//! `.toggle` rules live in the preview, not in `colors_and_type.css`).
//!
//! A 44 by 26px pill track (`radius-full`) in `outline` when off and
//! `primary` when on — the README's own note: primary, never the
//! tertiary accent, which falls below the 3:1 a control needs on the
//! teal-light surface. The 20px thumb is `surface` when off and
//! `on-primary` when on, carrying `shadow-l1`, and slides over 200ms
//! on the house curve `cubic-bezier(0.2, 0.8, 0.2, 1)`. The label sits
//! beside the track in the 14px body face with a 12px gap, and a click
//! on the words flips the switch too.
//!
//! Motion inside egui's per-frame model: the slide is remembered per
//! widget id as (from, target, started-at) in context data and driven
//! by the input clock, a fresh crossing starting from wherever the
//! knob happens to be so a mid-flight reversal turns around naturally;
//! the widget asks for repaints until it lands. Under
//! [`UiKitFlags::reduced_motion`] the 200ms is a spatial transition
//! and drops to zero — the knob snaps, the spec's own example use of
//! the flag.
//!
//! States otherwise follow the shared helpers: the 8% and 12% overlays
//! over the track's own on-colour, the shared focus ring at the
//! track's own pill radius, disabled at the 38/12 pair (thumb 38,
//! track 12) with the label in `text-muted`.
//!
//! Documented deviations. egui's `epaint` circles carry no shadow, so
//! `shadow-l1` paints as its layers' own offset translucent circles
//! behind the thumb; and egui's accessibility slots have no switch, so
//! the state reports through the checkbox slot (checked = on).

use egui::epaint::CircleShape;
use egui::{Color32, FontId, Id, Pos2, Response, Sense, Ui, Vec2};

use crate::components::FocusRing;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::spacing;
use crate::typography::TypeStyle;

/// The track: 44 by 26px.
pub const TRACK_W: f32 = 44.0;
/// The track's height.
pub const TRACK_H: f32 = 26.0;
/// The thumb: 20px.
pub const KNOB_SIZE: f32 = 20.0;
/// The thumb's inset from the track's edges: 3px.
pub const KNOB_INSET: f32 = 3.0;
/// How far the thumb slides: the track minus the insets and the thumb,
/// the preview's `left: 3px` to `left: 21px`.
pub const KNOB_TRAVEL: f32 = TRACK_W - 2.0 * KNOB_INSET - KNOB_SIZE;
/// The slide's duration: ~200ms, a step on the motion duration scale
/// (`--duration-short`).
pub const TRAVEL_MS: f32 = 200.0;
/// The gap between the track and the label: the preview's `.setting`
/// gap (`--space-3`).
pub const LABEL_GAP: f32 = spacing::SPACE_3;
/// The label: 14px `text-body-sm` in the body face.
pub const LABEL_SIZE: f32 = crate::components::checkbox::LABEL_SIZE;

/// The house curve `cubic-bezier(0.2, 0.8, 0.2, 1)`
/// ([`crate::tokens::motion::EASING_STANDARD`], near-equivalent to
/// Material 3 emphasized-decelerate) evaluated at `fraction` of the
/// timeline: the fraction as timeline x, returned as the curve's y.
///
/// The evaluation is WebKit's `UnitBezier`: Newton-Raphson on the
/// curve's x, bisection as the fallback, so the result is the curve's
/// own y and not an approximation of it.
pub fn ease_vynatix(fraction: f32) -> f32 {
    let [x1, y1, x2, y2] = crate::tokens::motion::EASING_STANDARD;
    let bezier = UnitBezier::new(x1, y1, x2, y2);
    bezier.solve(fraction.clamp(0.0, 1.0), 1e-6)
}

/// A CSS timing-function cubic bezier with x in [0, 1] and y in
/// [0, 1], solved for y at a given x (WebKit's `UnitBezier`).
struct UnitBezier {
    /// The x polynomial's coefficients, descending.
    ax: f32,
    bx: f32,
    cx: f32,
    /// The y polynomial's coefficients, descending.
    ay: f32,
    by: f32,
    cy: f32,
}

impl UnitBezier {
    fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        // The Bernstein form collapsed to powers of t.
        let cx = 3.0 * x1;
        let bx = 3.0 * (x2 - x1) - cx;
        let ax = 1.0 - cx - bx;
        let cy = 3.0 * y1;
        let by = 3.0 * (y2 - y1) - cy;
        let ay = 1.0 - cy - by;
        Self {
            ax,
            bx,
            cx,
            ay,
            by,
            cy,
        }
    }

    fn sample_x(&self, t: f32) -> f32 {
        ((self.ax * t + self.bx) * t + self.cx) * t
    }

    fn sample_y(&self, t: f32) -> f32 {
        ((self.ay * t + self.by) * t + self.cy) * t
    }

    fn sample_derivative_x(&self, t: f32) -> f32 {
        (3.0 * self.ax * t + 2.0 * self.bx) * t + self.cx
    }

    /// y at the t whose x is `x`: Newton first, bisection as the
    /// fallback when Newton stalls.
    fn solve(&self, x: f32, epsilon: f32) -> f32 {
        // Newton-Raphson, from a linear-lerp first guess.
        let mut t = x;
        for _ in 0..8 {
            let x2 = self.sample_x(t);
            if (x - x2).abs() < epsilon {
                return self.sample_y(t);
            }
            let d = self.sample_derivative_x(t);
            if d.abs() < 1e-6 {
                break;
            }
            t -= (x2 - x) / d;
        }
        // Bisection on x, bracketed to [0, 1].
        let (mut lo, mut hi, mut t) = (0.0, 1.0, x);
        while hi - lo > epsilon {
            let x2 = self.sample_x(t);
            if (x - x2).abs() < epsilon {
                return self.sample_y(t);
            }
            if x2 > x {
                hi = t;
            } else {
                lo = t;
            }
            t = (lo + hi) / 2.0;
        }
        self.sample_y(t)
    }
}

/// How far through the slide `elapsed_ms` of a `duration_ms` crossing
/// is, on the house curve: clamped, eased, and 1 for a zero duration —
/// a spatial transition under reduced motion snaps rather than slides.
pub fn travel_fraction(elapsed_ms: f32, duration_ms: f32) -> f32 {
    if duration_ms <= 0.0 {
        return 1.0;
    }
    ease_vynatix((elapsed_ms / duration_ms).clamp(0.0, 1.0))
}

/// The thumb's left edge inside `track` at `fraction` of the travel.
pub fn knob_left(track: egui::Rect, fraction: f32) -> f32 {
    track.left() + KNOB_INSET + KNOB_TRAVEL * fraction.clamp(0.0, 1.0)
}

/// The thumb's rect inside `track` at `fraction` of the travel: a
/// [`KNOB_SIZE`] square whose left edge is [`knob_left`] and which is
/// centred on the track's midline. The painter takes the thumb's centre
/// from this rect, so the slide's two ends sit [`KNOB_INSET`] in from the
/// track's edges.
pub fn knob_rect(track: egui::Rect, fraction: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        Pos2::new(
            knob_left(track, fraction),
            track.center().y - KNOB_SIZE / 2.0,
        ),
        Vec2::splat(KNOB_SIZE),
    )
}

/// The track's rect inside its row: flush left, centred on the row.
pub fn track_rect(row: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        Pos2::new(row.left(), row.center().y - TRACK_H / 2.0),
        Vec2::new(TRACK_W, TRACK_H),
    )
}

/// The toggle's resolved paints for one state: pure data off the
/// tokens, testable without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToggleLook {
    /// The track: `outline` off, `primary` on, an overlay composited
    /// in on hover and press.
    pub track: Color32,
    /// The thumb: `surface` off, `on-primary` on.
    pub knob: Color32,
    /// The label colour: `text-body`, `text-muted` when disabled.
    pub label: Color32,
}

/// Resolve one toggle in one state, per theme: `on` the switch's
/// state. Hover and press are the shared state layers over the track's
/// own on-colour; disabled is the 38/12 pair with the label in
/// `text-muted`.
pub fn toggle_look(on: bool, widget: &WidgetState, theme: ThemeId) -> ToggleLook {
    let colors = theme.colors();
    let (track, knob, on_track) = if on {
        (colors.primary, colors.on_primary, colors.on_primary)
    } else {
        (colors.outline, colors.surface, colors.on_surface)
    };
    let mut look = ToggleLook {
        track,
        knob,
        label: colors.text_body(),
    };

    if widget.disabled {
        // The shared pair over the switch: the track at 12%, the thumb
        // at 38%, the label in `text-muted`.
        let pair = states::disabled(knob, track);
        look.track = pair.container;
        look.knob = pair.content;
        look.label = colors.text_muted();
        return look;
    }

    // The state layers over the track's own on-colour, press
    // outranking hover.
    let layer = if widget.pressed {
        states::pressed_overlay(on_track)
    } else if widget.hovered {
        states::hover_overlay(on_track)
    } else {
        return look;
    };
    look.track = states::composite_over(layer, track);
    look
}

/// The remembered state of one slide: where the knob came from, where
/// it is going, and when the crossing started (input time, seconds).
#[derive(Clone, Copy, Debug)]
struct KnobAnimation {
    from: f32,
    target: f32,
    started_at: f64,
}

/// The knob's position this frame: the remembered crossing eased
/// forward to `now`, a new target turning it around from wherever the
/// knob is, and repaints requested until it lands. `duration_ms` of
/// zero (reduced motion) snaps.
fn knob_position(ctx: &egui::Context, id: Id, target: f32, duration_ms: f32) -> f32 {
    let now = ctx.input(|input| input.time);
    let mut anim = ctx.data_mut(|data| {
        *data.get_temp_mut_or_insert_with(id, || KnobAnimation {
            from: target,
            target,
            started_at: now,
        })
    });
    if anim.target != target {
        // A new crossing starts from wherever the knob is.
        anim = KnobAnimation {
            from: knob_at(anim, now, duration_ms),
            target,
            started_at: now,
        };
    }
    let position = knob_at(anim, now, duration_ms);
    ctx.data_mut(|data| data.insert_temp(id, anim));
    if position != anim.target {
        // Play the slide out: repaint until it lands.
        let remaining = duration_ms / 1000.0 - (now - anim.started_at) as f32;
        ctx.request_repaint_after_secs(remaining.max(0.0) + 0.004);
    }
    position
}

/// The knob's position at `now` along one crossing.
fn knob_at(anim: KnobAnimation, now: f64, duration_ms: f32) -> f32 {
    if duration_ms <= 0.0 {
        return anim.target;
    }
    let elapsed_ms = (((now - anim.started_at) * 1000.0) as f32).max(0.0);
    anim.from + (anim.target - anim.from) * travel_fraction(elapsed_ms, duration_ms)
}

/// The toggle widget: a pill switch with its label beside it, for a
/// setting that takes effect at once.
///
/// ```
/// use ui_kit::components::toggle::Toggle;
///
/// let motion = Toggle::new("Reduced motion").checked(true);
/// let locked = Toggle::new("Sync over mobile data").disabled();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Toggle<'a> {
    label: &'a str,
    checked: bool,
    enabled: bool,
}

impl Default for Toggle<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<'a> Toggle<'a> {
    /// An off toggle labelled `label`.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            checked: false,
            enabled: true,
        }
    }

    /// Set the switch's state.
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set enabled: a disabled toggle drops to the 38/12 pair and
    /// neither clicks nor takes focus.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the toggle.
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Show the toggle and hand back its typed response: the row's
    /// egui response, the state a flip landed on (if any), and the
    /// knob's position along its slide this frame (0 off, 1 on).
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> ToggleResponse {
        let font_id = FontId {
            size: LABEL_SIZE,
            family: TypeStyle::Body.family(theme),
        };
        let galley = ui.ctx().fonts_mut(|fonts| {
            fonts.layout_delayed_color(self.label.to_owned(), font_id, f32::INFINITY)
        });

        let height = TRACK_H.max(galley.size().y);
        let size = Vec2::new(TRACK_W + LABEL_GAP + galley.size().x, height);
        let id = ui.next_auto_id().with("vynatix-toggle");
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(
            rect,
            id,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let mut widget = WidgetState::of(&response);
        if !self.enabled {
            widget.disabled = true;
        }
        let look = toggle_look(self.checked, &widget, theme);
        let painter = ui.painter().clone().with_layer_id(response.layer_id);

        // A click (mouse, touch, or Space/Enter through egui's focused
        // widget activation) takes the focus and flips the switch; the
        // knob then slides toward the new state.
        let mut toggled = None;
        if response.clicked() {
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(response.id));
            toggled = Some(!self.checked);
        }
        let target = toggled.unwrap_or(self.checked);
        let duration = flags.spatial_ms(TRAVEL_MS);
        let position = knob_position(ui.ctx(), id, if target { 1.0 } else { 0.0 }, duration);

        let track = track_rect(rect);
        let radius = (TRACK_H / 2.0) as u8;
        painter.rect_filled(track, egui::CornerRadius::same(radius), look.track);
        // `knob_left` is the thumb's left edge, not its centre: the
        // circle is centred on the thumb's rect.
        let center = knob_rect(track, position).center();
        // shadow-l1 as its layers' own offset translucent circles.
        for layer in theme.shadows().l1 {
            if !layer.inset {
                painter.add(CircleShape::filled(
                    Pos2::new(center.x + layer.x, center.y + layer.y),
                    KNOB_SIZE / 2.0 + layer.blur / 2.0 + layer.spread,
                    layer.color,
                ));
            }
        }
        painter.circle_filled(center, KNOB_SIZE / 2.0, look.knob);
        let label_pos = crate::typography::left_center(
            rect.left() + TRACK_W + LABEL_GAP,
            rect.center().y,
            galley.size(),
        );
        painter.galley(label_pos, galley, look.label);

        // The switch semantics; egui's accessibility slots carry no
        // switch, so the state reports through the checkbox slot.
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                self.enabled,
                self.checked,
                self.label,
            )
        });

        // The ring belongs to the track, the click target to the row.
        FocusRing::new()
            .radius(egui::CornerRadius::same(radius))
            .paint_at(ui, &response, track, theme, flags);

        ToggleResponse {
            response,
            toggled,
            position,
        }
    }
}

/// The toggle's response: the row's egui response, the typed state a
/// flip landed on (if any), and the knob's position this frame.
#[derive(Clone, Debug)]
pub struct ToggleResponse {
    /// The wrapped egui response.
    pub response: Response,
    /// The new state, if the switch was flipped this frame.
    pub toggled: Option<bool>,
    /// The knob's position along its slide, 0 (off) to 1 (on).
    pub position: f32,
}

#[cfg(test)]
mod tests {
    use crate::states::{self, UiKitFlags, WidgetState};
    use crate::theme::ThemeId;
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        ease_vynatix, knob_left, knob_rect, toggle_look, track_rect, travel_fraction, Toggle,
        KNOB_INSET, KNOB_SIZE, KNOB_TRAVEL, LABEL_GAP, TRACK_H, TRACK_W, TRAVEL_MS,
    };

    /// Install the faces the toggle paints with: the icon families and
    /// the theme's type faces bind at the pass boundary, so a warm-up
    /// pass makes them resolvable for the widget passes that follow.
    fn install_faces(ctx: &egui::Context, theme: ThemeId) {
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
    }

    /// A state with only one flag set, or the plain rest state.
    fn state(flag: &str) -> WidgetState {
        match flag {
            "rest" => WidgetState::default(),
            "hover" => WidgetState {
                hovered: true,
                ..Default::default()
            },
            "pressed" => WidgetState {
                pressed: true,
                ..Default::default()
            },
            "disabled" => WidgetState {
                disabled: true,
                ..Default::default()
            },
            other => panic!("unknown state {other}"),
        }
    }

    /// The look table per the Toggle preview, in every theme: off is
    /// an `outline` track with a `surface` thumb; on is a `primary`
    /// track (never the tertiary accent) with an `on-primary` thumb;
    /// disabled is the 38/12 pair with the label in `text-muted`.
    #[test]
    fn looks_match_the_preview_table_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let rest = state("rest");

            let off = toggle_look(false, &rest, *theme);
            assert_eq!(off.track, colors.outline, "{name}: outline track off");
            assert_eq!(off.knob, colors.surface, "{name}: surface thumb off");

            let on = toggle_look(true, &rest, *theme);
            assert_eq!(on.track, colors.primary, "{name}: primary track on");
            assert_ne!(
                on.track, colors.tertiary,
                "{name}: never the tertiary accent"
            );
            assert_eq!(on.knob, colors.on_primary, "{name}: on-primary thumb on");

            let disabled = toggle_look(true, &state("disabled"), *theme);
            assert_eq!(
                disabled.track,
                states::disabled(colors.on_primary, colors.primary).container,
                "{name}: the track at 12%"
            );
            assert_eq!(
                disabled.knob,
                states::disabled(colors.on_primary, colors.primary).content,
                "{name}: the thumb at 38%"
            );
            assert_eq!(disabled.label, colors.text_muted(), "{name}: muted label");
        }
    }

    /// Hover and press are the shared state layers over the track's
    /// own on-colour: `on-surface` over the outline track off,
    /// `on-primary` over the primary track on.
    #[test]
    fn hover_and_press_apply_the_state_layers() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let off_hover = toggle_look(false, &state("hover"), *theme);
            assert_eq!(
                off_hover.track,
                states::composite_over(states::hover_overlay(colors.on_surface), colors.outline),
                "{name}: off hover composites"
            );

            let on_press = toggle_look(true, &state("pressed"), *theme);
            assert_eq!(
                on_press.track,
                states::composite_over(states::pressed_overlay(colors.on_primary), colors.primary),
                "{name}: on press composites"
            );
        }
    }

    /// The geometry: a 44 by 26px track, a 20px thumb inset 3px that
    /// travels 18px (the preview's `left: 3px` to `left: 21px`), and
    /// the 12px gap to the label.
    #[test]
    fn geometry_matches_the_spec() {
        assert_eq!(TRACK_W, 44.0);
        assert_eq!(TRACK_H, 26.0);
        assert_eq!(KNOB_SIZE, 20.0);
        assert_eq!(KNOB_INSET, 3.0);
        assert_eq!(KNOB_TRAVEL, 18.0);
        assert_eq!(LABEL_GAP, 12.0);
        assert_eq!(TRAVEL_MS, 200.0);

        let track = track_rect(egui::Rect::from_min_size(
            egui::Pos2::new(10.0, 50.0),
            egui::Vec2::new(200.0, TRACK_H),
        ));
        assert_eq!(track.min, egui::Pos2::new(10.0, 50.0));
        assert_eq!(track.size(), egui::Vec2::new(TRACK_W, TRACK_H));
        assert_eq!(knob_left(track, 0.0), 13.0);
        assert_eq!(knob_left(track, 1.0), 31.0);
        assert_eq!(knob_left(track, 1.5), 31.0, "clamped");
    }

    /// The thumb's rect keeps the thumb's size and stays inside the
    /// track the whole way along: `KNOB_INSET` from the left edge at
    /// 0 (off), `KNOB_INSET` from the right edge at 1 (on), and between
    /// the two for the middle of the slide.
    #[test]
    fn knob_rect_stays_inside_the_track_with_the_inset_at_both_ends() {
        let track = track_rect(egui::Rect::from_min_size(
            egui::Pos2::new(10.0, 50.0),
            egui::Vec2::new(200.0, TRACK_H),
        ));
        for fraction in [0.0, 0.5, 1.0] {
            let knob = knob_rect(track, fraction);
            assert_eq!(
                knob.size(),
                egui::Vec2::splat(KNOB_SIZE),
                "fraction {fraction}: the thumb keeps its size"
            );
            assert!(
                track.contains_rect(knob),
                "fraction {fraction}: thumb {knob:?} leaves track {track:?}"
            );
            assert_eq!(
                knob.center().y,
                track.center().y,
                "fraction {fraction}: centred on the track's midline"
            );
        }
        assert_eq!(
            knob_rect(track, 0.0).left() - track.left(),
            KNOB_INSET,
            "off: the left gap is the inset"
        );
        assert_eq!(
            track.right() - knob_rect(track, 1.0).right(),
            KNOB_INSET,
            "on: the right gap is the inset"
        );
    }

    /// Every shape a frame painted, with `Shape::Vec` groups walked
    /// into, in paint order.
    fn flat_shapes(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Shape> {
        fn walk(shape: &egui::Shape, found: &mut Vec<egui::Shape>) {
            match shape {
                egui::Shape::Vec(nested) => nested.iter().for_each(|shape| walk(shape, found)),
                other => found.push(other.clone()),
            }
        }
        let mut found = Vec::new();
        for clipped in shapes {
            walk(&clipped.shape, &mut found);
        }
        found
    }

    /// What the toggle really paints, in both themes and both states:
    /// the thumb's circle lies inside the painted track with the inset
    /// gap at its end (left when off, right when on), and it is the
    /// circle `knob_rect` describes. The paint once took the thumb's
    /// left edge for its centre, which left half the off thumb outside
    /// the pill and stopped the on thumb short of the right end.
    #[test]
    fn the_painted_knob_sits_inside_the_painted_track_at_both_ends() {
        for theme in ThemeId::ALL {
            let name = theme.name();
            for on in [false, true] {
                // A fresh context per case: the knob seeds its slide at
                // the target, so the first frame paints it settled.
                let ctx = egui::Context::default();
                install_faces(&ctx, *theme);
                let flags = UiKitFlags::default();

                let (row, shapes) = paint_pass(&ctx, |ui| {
                    Toggle::new("Reduced motion")
                        .checked(on)
                        .ui(ui, *theme, &flags)
                        .response
                        .rect
                });
                let look = toggle_look(on, &WidgetState::default(), *theme);
                let painted = flat_shapes(&shapes);

                let tracks: Vec<egui::Rect> = painted
                    .iter()
                    .filter_map(|shape| match shape {
                        egui::Shape::Rect(rect) if rect.fill == look.track => Some(rect.rect),
                        _ => None,
                    })
                    .collect();
                let [track] = tracks.as_slice() else {
                    panic!("{name}, on={on}: expected one painted track, got {tracks:?}");
                };
                assert_eq!(*track, track_rect(row), "{name}, on={on}: the track");

                let knobs: Vec<&egui::epaint::CircleShape> = painted
                    .iter()
                    .filter_map(|shape| match shape {
                        egui::Shape::Circle(circle) if circle.fill == look.knob => Some(circle),
                        _ => None,
                    })
                    .collect();
                let knob = knobs
                    .last()
                    .unwrap_or_else(|| panic!("{name}, on={on}: no thumb circle was painted"));
                assert_eq!(knob.radius, KNOB_SIZE / 2.0, "{name}, on={on}: thumb size");
                let knob_box =
                    egui::Rect::from_center_size(knob.center, egui::Vec2::splat(KNOB_SIZE));

                assert!(
                    track.contains_rect(knob_box),
                    "{name}, on={on}: thumb {knob_box:?} leaves track {track:?}"
                );
                if on {
                    assert_eq!(
                        track.right() - knob_box.right(),
                        KNOB_INSET,
                        "{name}: on, the right gap is the inset"
                    );
                } else {
                    assert_eq!(
                        knob_box.left() - track.left(),
                        KNOB_INSET,
                        "{name}: off, the left gap is the inset"
                    );
                }
                assert_eq!(
                    knob_box,
                    knob_rect(*track, if on { 1.0 } else { 0.0 }),
                    "{name}, on={on}: the paint follows knob_rect"
                );
            }
        }
    }

    /// The house curve: identity at the ends, a strong decelerate
    /// through the middle (the timeline half-eased at 94.6 percent,
    /// the quarter at 76.5), monotone, and the snap under a zero
    /// duration.
    #[test]
    fn the_knob_travel_math() {
        assert_eq!(ease_vynatix(0.0), 0.0);
        assert_eq!(ease_vynatix(1.0), 1.0);
        assert!(
            (ease_vynatix(0.5) - 0.946).abs() < 0.01,
            "{}",
            ease_vynatix(0.5)
        );
        assert!(
            (ease_vynatix(0.25) - 0.765).abs() < 0.01,
            "{}",
            ease_vynatix(0.25)
        );
        let mut previous = 0.0;
        for step in 1..=20 {
            let eased = ease_vynatix(step as f32 / 20.0);
            assert!(eased >= previous, "monotone");
            previous = eased;
        }

        // t = 0, 0.5, 1 for the known 200ms duration, the clamp past
        // the end, and the reduced-motion snap at a zero duration.
        assert_eq!(travel_fraction(0.0, 200.0), 0.0);
        assert!((travel_fraction(100.0, 200.0) - 0.946).abs() < 0.01);
        assert_eq!(travel_fraction(200.0, 200.0), 1.0);
        assert_eq!(travel_fraction(250.0, 200.0), 1.0);
        assert_eq!(travel_fraction(100.0, 0.0), 1.0, "reduced motion snaps");
    }

    /// The state machine: a click flips off to on and back, Space
    /// activates the focused toggle, and a disabled one reports
    /// nothing.
    #[test]
    fn toggles_flip_off_on_and_off_again() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();

        let mut centers = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let motion = Toggle::new("Reduced motion").ui(ui, ThemeId::Light, &flags);
            let dark = Toggle::new("Dark theme").ui(ui, ThemeId::Light, &flags);
            centers = vec![motion.response.rect.center(), dark.response.rect.center()];
        })
        .drop_without_applying_deltas();

        let off = click(&ctx, centers[0], |ui| {
            Toggle::new("Reduced motion")
                .ui(ui, ThemeId::Light, &flags)
                .toggled
        });
        assert_eq!(off, Some(true), "off flips on");

        let back = click(&ctx, centers[0], |ui| {
            Toggle::new("Reduced motion")
                .checked(true)
                .ui(ui, ThemeId::Light, &flags)
                .toggled
        });
        assert_eq!(back, Some(false), "on flips off");

        // Space activates the focused toggle.
        let mut seen_id = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Toggle::new("Dark theme").ui(ui, ThemeId::Light, &flags);
            seen_id = Some(response.response.id);
        })
        .drop_without_applying_deltas();
        ctx.memory_mut(|memory| memory.request_focus(seen_id.unwrap()));
        let keyed = press_key(&ctx, egui::Key::Space, |ui| {
            Toggle::new("Dark theme")
                .ui(ui, ThemeId::Light, &flags)
                .toggled
        });
        assert_eq!(keyed, Some(true));

        // Disabled: no click, no keyboard activation.
        let locked = click(&ctx, centers[1], |ui| {
            Toggle::new("Dark theme")
                .disabled()
                .ui(ui, ThemeId::Light, &flags)
                .toggled
        });
        assert_eq!(locked, None);
    }

    /// The slide over the input clock: the flip frame holds the knob
    /// where it was, the half-duration sits at the curve's 94.6
    /// percent, the full duration lands, and reduced motion snaps on
    /// the flip frame itself.
    #[test]
    fn the_knob_slides_over_the_input_clock() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();

        let position_at = |ctx: &egui::Context, checked: bool, time: f64, flags: &UiKitFlags| {
            let input = egui::RawInput {
                time: Some(time),
                ..Default::default()
            };
            let mut position = None;
            let out = ctx.run_ui(input, |ui| {
                position = Some(
                    Toggle::new("Reduced motion")
                        .checked(checked)
                        .ui(ui, ThemeId::Light, flags)
                        .position,
                );
            });
            out.drop_without_applying_deltas();
            position.unwrap()
        };

        let flags = UiKitFlags::default();
        // Off, settled: the knob rests at 0.
        assert_eq!(position_at(&ctx, false, 0.0, &flags), 0.0);
        // The flip at t = 100ms: the knob is still where it was…
        assert_eq!(position_at(&ctx, true, 0.1, &flags), 0.0);
        // …100ms in sits at the curve's half…
        assert!(
            (position_at(&ctx, true, 0.2, &flags) - 0.946).abs() < 0.01,
            "{}",
            position_at(&ctx, true, 0.2, &flags)
        );
        // …and the full 200ms lands.
        assert_eq!(position_at(&ctx, true, 0.3, &flags), 1.0);

        // Reduced motion: the flip frame itself snaps.
        let reduced = UiKitFlags {
            reduced_motion: true,
            ..Default::default()
        };
        assert_eq!(position_at(&ctx, true, 0.1, &reduced), 1.0);
    }

    /// The label starts after the 44px track and its 12px gap. The paint
    /// once also subtracted half the label's width from that left
    /// anchor, so the label ran back under its own track.
    #[test]
    fn the_label_starts_after_the_track_and_its_gap() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();

        let (row, shapes) = paint_pass(&ctx, |ui| {
            Toggle::new("Reduced motion")
                .ui(ui, theme, &flags)
                .response
                .rect
        });
        let pos = assert_painted_inside(&shapes, "Reduced motion", row);
        let after_track = row.left() + TRACK_W + LABEL_GAP;
        assert!(
            (pos.x - after_track).abs() < 0.01,
            "starts at x {} but after the track and its gap is {after_track}",
            pos.x
        );
    }

    /// Run a press then a release at `pos`, returning what `widget`
    /// reported on the release frame.
    fn click<T>(
        ctx: &egui::Context,
        pos: egui::Pos2,
        mut widget: impl FnMut(&mut egui::Ui) -> T,
    ) -> T {
        let mut result = None;
        for pressed in [true, false] {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            let mut outcome = None;
            ctx.run_ui(input, |ui| {
                outcome = Some(widget(ui));
            })
            .drop_without_applying_deltas();
            if !pressed {
                result = outcome;
            }
        }
        result.expect("the release frame ran")
    }

    /// Send one key press, returning what `widget` reported on that
    /// frame.
    fn press_key<T>(
        ctx: &egui::Context,
        key: egui::Key,
        mut widget: impl FnMut(&mut egui::Ui) -> T,
    ) -> T {
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        });
        let mut outcome = None;
        ctx.run_ui(input, |ui| {
            outcome = Some(widget(ui));
        })
        .drop_without_applying_deltas();
        outcome.expect("the key frame ran")
    }
}

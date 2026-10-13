//! Interaction states: the overlay composites, the press scale, the
//! disabled opacities, the focus-ring geometry, and the kit flags that
//! carry the host's motion and transparency preferences.
//!
//! Source: the design system README, "Hover, press, focus". Hover is an
//! 8% on-colour overlay (Material 3's state layer), press 12% plus a
//! small scale, keyboard focus a 2px `focus-ring` outline at a 2px
//! offset over a 4px canvas-coloured halo, disabled 38% content over a
//! 12% container. The FocusRing component wraps the geometry as a
//! widget response treatment ([`crate::components::FocusRing`]).
//!
//! Compositing. egui's [`Color32`] is premultiplied sRGBA and the
//! renderer blends `(ONE, ONE_MINUS_SRC_ALPHA)`, so [`composite_over`]
//! works channel-wise in that space: the integer twin of what the GPU
//! does when it layers an overlay rect over an opaque fill, which is
//! the web's `rgba(...)` over an opaque background. The spec's
//! percentages live in sRGB, not linear: `Color32::linear_multiply`
//! would gamma-convert them, so nothing here touches it.

use egui::epaint::{CornerRadius, RectShape, StrokeKind};
use egui::{Color32, Painter, Rect, Response, Stroke};

use crate::theme::ThemeId;
use crate::tokens::{radii, ThemeColors};

/// A spec percentage as a byte alpha: `round(percent * 255)`, the same
/// quantization the generated shadow tints use (0.08 to 20, 0.16 to 41).
const fn alpha_from_percent(percent: f32) -> u8 {
    (percent * 255.0 + 0.5) as u8
}

/// Hover: the on-colour state layer at 8% (Material 3).
pub const HOVER_OVERLAY_ALPHA: u8 = alpha_from_percent(0.08);
/// Pressed: 12%.
pub const PRESSED_OVERLAY_ALPHA: u8 = alpha_from_percent(0.12);
/// Disabled content: 38% opacity.
pub const DISABLED_CONTENT_ALPHA: u8 = alpha_from_percent(0.38);
/// Disabled container: 12%.
pub const DISABLED_CONTAINER_ALPHA: u8 = alpha_from_percent(0.12);

/// Press scale: the button specimen's 98.5%, inside the README's 98 to
/// 99% band, held over the press duration and returning on release.
pub const PRESS_SCALE: f32 = 0.985;
/// The press duration: ~100ms (a step on the motion duration scale).
pub const PRESS_DURATION_MS: f32 = 100.0;

/// The focus-ring outline: 2px of the `focus-ring` role (README
/// "Borders": 2px for focus rings and selection indicators).
pub const FOCUS_RING_WIDTH: f32 = 2.0;
/// The outline's 2px offset.
pub const FOCUS_RING_OFFSET: f32 = 2.0;
/// The canvas-coloured halo: a 4px `surface` band filling the offset
/// gap under the ring, so the ring reads against any fill.
pub const FOCUS_HALO_WIDTH: f32 = 4.0;
/// The ring fades in over ~120ms (README "Hover, press, focus"). Not a
/// duration-scale step: the focus rule names it directly.
pub const FOCUS_FADE_MS: f32 = 120.0;

/// The largest opacity fade kept under reduced motion.
pub const REDUCED_FADE_MAX_MS: f32 = 100.0;

/// The hover state layer: `on`, the fill's on-colour, at 8% alpha.
/// Composite it over the fill with [`composite_over`].
pub fn hover_overlay(on: Color32) -> Color32 {
    state_layer(on, HOVER_OVERLAY_ALPHA)
}

/// The pressed state layer: `on` at 12% alpha.
pub fn pressed_overlay(on: Color32) -> Color32 {
    state_layer(on, PRESSED_OVERLAY_ALPHA)
}

/// `color` at `alpha`: the on-colours arrive opaque from the tokens, so
/// their channels are the straight sRGB values and re-premultiplying
/// at the state layer's alpha is exact.
fn state_layer(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// Composite `overlay` over an opaque `fill`, channel-wise in
/// premultiplied space: `src + dst * (1 - a)`, each part rounded to the
/// nearest byte. What egui's blend does when it stacks the two.
pub fn composite_over(overlay: Color32, fill: Color32) -> Color32 {
    let alpha = overlay.a() as u16;
    let over = |src: u8, dst: u8| -> u8 {
        let dst = (dst as u16 * (255 - alpha) + 127) / 255;
        (src as u16 + dst).min(255) as u8
    };
    Color32::from_rgb(
        over(overlay.r(), fill.r()),
        over(overlay.g(), fill.g()),
        over(overlay.b(), fill.b()),
    )
}

/// The disabled pair of one widget: its content colour at 38% opacity
/// over its container colour at 12%.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisabledColors {
    /// Icons, labels, text: 38%.
    pub content: Color32,
    /// The fill the content sits on: 12%.
    pub container: Color32,
}

/// Disabled: content at 38% opacity, container at 12%, from the
/// widget's rest colours.
pub fn disabled(content: Color32, container: Color32) -> DisabledColors {
    DisabledColors {
        content: state_layer(content, DISABLED_CONTENT_ALPHA),
        container: state_layer(container, DISABLED_CONTAINER_ALPHA),
    }
}

/// An egui [`Response`] boiled down to the four states the design
/// system styles, consuming egui's own interaction sensing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetState {
    /// The pointer hovers the widget, unobstructed.
    pub hovered: bool,
    /// The pointer button is held down on the widget (a click or a
    /// drag in progress): the press state.
    pub pressed: bool,
    /// The widget holds keyboard focus (Tab order, clicks into text).
    pub has_focus: bool,
    /// The widget was added disabled.
    pub disabled: bool,
}

impl WidgetState {
    /// Read the four states off `response`.
    pub fn of(response: &Response) -> Self {
        Self {
            hovered: response.hovered(),
            pressed: response.is_pointer_button_down_on(),
            has_focus: response.has_focus(),
            disabled: !response.enabled(),
        }
    }
}

/// The focus treatment's two shapes, in paint order (halo first, the
/// ring over it), exactly as [`focus_ring`] paints them.
#[derive(Clone, Debug)]
pub struct FocusRingGeometry {
    /// The halo: the widget's own rect stroked `Outside` in 4px of the
    /// `surface` role, so the band runs from the widget's edge to 4px
    /// out and fills the offset gap.
    pub halo: RectShape,
    /// The ring: the widget's rect grown by the 2px offset, stroked
    /// `Outside` in 2px of the `focus-ring` role.
    pub ring: RectShape,
}

impl FocusRingGeometry {
    /// The outer edge of the whole treatment: the widget rect grown by
    /// the 2px offset plus the 2px outline, 4px in total.
    pub fn outer_edge(&self) -> Rect {
        self.ring.rect.expand(FOCUS_RING_WIDTH)
    }

    /// Both shapes at `fraction` of their opacity (0 to 1): the ring's
    /// fade-in.
    pub fn faded(&self, fraction: f32) -> Self {
        Self {
            halo: fade_shape(&self.halo, fraction),
            ring: fade_shape(&self.ring, fraction),
        }
    }
}

/// One ring shape at `fraction` of its opacity: the stroke's colour
/// scaled, geometry untouched.
fn fade_shape(shape: &RectShape, fraction: f32) -> RectShape {
    let mut faded = shape.clone();
    faded.stroke.color = scale_opacity(shape.stroke.color, fraction);
    faded
}

/// A premultiplied colour at `fraction` of its opacity: the same
/// colour, every channel (alpha included) scaled.
fn scale_opacity(color: Color32, fraction: f32) -> Color32 {
    let scale = |channel: u8| (channel as f32 * fraction).round().clamp(0.0, 255.0) as u8;
    let [r, g, b, a] = color.to_array();
    Color32::from_rgba_premultiplied(scale(r), scale(g), scale(b), scale(a))
}

/// Build the focus treatment's shapes for `rect`, following the
/// element's own corner radius (the outline follows the radius, per the
/// FocusRing component spec).
pub fn focus_ring_geometry(
    rect: Rect,
    radius: impl Into<CornerRadius>,
    colors: &ThemeColors,
) -> FocusRingGeometry {
    let radius = radius.into();
    FocusRingGeometry {
        halo: RectShape::stroke(
            rect,
            radius,
            Stroke::new(FOCUS_HALO_WIDTH, colors.surface),
            StrokeKind::Outside,
        ),
        ring: RectShape::stroke(
            rect.expand(FOCUS_RING_OFFSET),
            radius,
            Stroke::new(FOCUS_RING_WIDTH, colors.focus_ring),
            StrokeKind::Outside,
        ),
    }
}

/// Paint the focus treatment around `rect`: the 2px `focus-ring`
/// outline at a 2px offset plus the 4px canvas-coloured halo under it.
///
/// The radius is the default widget radius (md); [`focus_ring_at`]
/// takes the element's own. This paints the full, static treatment;
/// [`crate::components::FocusRing`] fades it in over ~120ms.
pub fn focus_ring(painter: &Painter, rect: Rect, theme: ThemeId) {
    focus_ring_at(painter, rect, radii::RADIUS_MD as u8, theme);
}

/// [`focus_ring`] with an explicit corner radius.
pub fn focus_ring_at(
    painter: &Painter,
    rect: Rect,
    radius: impl Into<CornerRadius>,
    theme: ThemeId,
) {
    let geometry = focus_ring_geometry(rect, radius, &theme.colors());
    painter.add(geometry.halo);
    painter.add(geometry.ring);
}

/// The host's accessibility preferences: the kit's stand-in for the web
/// system's `prefers-reduced-motion` and `prefers-reduced-transparency`
/// media queries.
///
/// Documented deviation: egui exposes no media queries, so the host
/// senses the platform (the rustnotes host already reads iOS
/// reduce-motion) and passes the flags down beside the theme; the
/// gallery hosts them as fields next to its switcher. Default: both
/// off. Helpers here and the components consult them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiKitFlags {
    /// `prefers-reduced-motion: reduce`: spatial transitions go, opacity
    /// fades at or under 100ms stay.
    pub reduced_motion: bool,
    /// `prefers-reduced-transparency: reduce`: glass and tints give way
    /// to the equivalent solid tokens (consumed by issues 08 and 09).
    pub reduced_transparency: bool,
}

impl UiKitFlags {
    /// The duration a spatial transition may take: none at all under
    /// reduced motion, `duration` otherwise.
    pub fn spatial_ms(&self, duration: f32) -> f32 {
        if self.reduced_motion {
            0.0
        } else {
            duration
        }
    }

    /// The duration an opacity fade may take: clamped to 100ms under
    /// reduced motion, `duration` otherwise.
    pub fn fade_ms(&self, duration: f32) -> f32 {
        if self.reduced_motion {
            duration.min(REDUCED_FADE_MAX_MS)
        } else {
            duration
        }
    }

    /// True when glass and tints must give way to solid tokens.
    pub fn solid_surfaces(&self) -> bool {
        self.reduced_transparency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The state layers carry the spec's percentages as byte alphas and
    /// keep the on-colour's straight channels.
    #[test]
    fn state_layers_carry_the_spec_alphas() {
        assert_eq!(HOVER_OVERLAY_ALPHA, 20);
        assert_eq!(PRESSED_OVERLAY_ALPHA, 31);
        assert_eq!(DISABLED_CONTENT_ALPHA, 97);
        assert_eq!(DISABLED_CONTAINER_ALPHA, 31);

        let cream = ThemeColors::LIGHT.on_primary;
        assert_eq!(
            hover_overlay(cream),
            Color32::from_rgba_unmultiplied(239, 232, 218, 20)
        );
        assert_eq!(
            pressed_overlay(cream),
            Color32::from_rgba_unmultiplied(239, 232, 218, 31)
        );
    }

    /// 8% of Cream over Forest: the overlay premultiplies Cream to
    /// (19, 18, 17) at alpha 20, the fill contributes
    /// round(channel * 235/255) = (28, 53, 43), and the sum lands on
    /// the composite the GPU would paint.
    #[test]
    fn hover_overlay_composites_cream_over_forest() {
        let light = ThemeColors::LIGHT;
        let hover = composite_over(hover_overlay(light.on_primary), light.primary);
        assert_eq!(hover, Color32::from_rgb(47, 71, 60));
        assert_eq!(hover.a(), 255, "over an opaque fill, fully opaque");
    }

    /// 12% of Cream over Forest: premultiplied (29, 28, 27) at alpha
    /// 31 over round(channel * 224/255) = (26, 51, 41).
    #[test]
    fn pressed_overlay_composites_cream_over_forest() {
        let light = ThemeColors::LIGHT;
        let pressed = composite_over(pressed_overlay(light.on_primary), light.primary);
        assert_eq!(pressed, Color32::from_rgb(55, 79, 68));
    }

    /// The disabled pair: content at 38, container at 12, straight
    /// channels preserved for opaque inputs.
    #[test]
    fn disabled_sets_both_opacities() {
        let light = ThemeColors::LIGHT;
        let pair = disabled(light.on_surface, light.surface_container);
        assert_eq!(
            pair.content,
            Color32::from_rgba_unmultiplied(34, 34, 34, 97)
        );
        assert_eq!(
            pair.container,
            Color32::from_rgba_unmultiplied(239, 232, 218, 31)
        );
    }

    /// The press scale is the button specimen's 98.5%, inside the
    /// README's 98 to 99% band, held for ~100ms.
    #[test]
    fn press_scale_sits_in_the_spec_band() {
        assert_eq!(PRESS_SCALE, 0.985);
        assert!((0.98..=0.99).contains(&PRESS_SCALE));
        assert_eq!(PRESS_DURATION_MS, 100.0);
    }

    /// A Response maps onto the four states: a plain button neither
    /// hovers nor presses nor holds focus; a requested focus lands the
    /// next pass; a disabled button reports disabled and nothing else.
    #[test]
    fn widget_state_reads_the_response() {
        let ctx = egui::Context::default();
        let mut seen_id = None;
        let mut focused_pass = WidgetState::default();
        for pass in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let response = ui.add(egui::Button::new("Start a project"));
                if pass == 0 {
                    seen_id = Some(response.id);
                    let plain = WidgetState::of(&response);
                    assert!(!plain.hovered && !plain.pressed && !plain.has_focus);
                    assert!(!plain.disabled);
                } else {
                    focused_pass = WidgetState::of(&response);
                }
            })
            .drop_without_applying_deltas();
            if pass == 0 {
                ctx.memory_mut(|memory| memory.request_focus(seen_id.unwrap()));
            }
        }
        assert!(focused_pass.has_focus, "requested focus holds");
        assert!(!focused_pass.hovered && !focused_pass.pressed);
        assert!(!focused_pass.disabled);

        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = ui.add_enabled(false, egui::Button::new("Start a project"));
            let state = WidgetState::of(&response);
            assert!(state.disabled);
            assert!(!state.hovered && !state.pressed && !state.has_focus);
        })
        .drop_without_applying_deltas();
    }

    /// The ring: a 2px `focus-ring` outline whose path sits at the 2px
    /// offset (stroke `Outside`, so its outer edge is 4px out), over a
    /// 4px `surface` halo on the widget's own rect.
    #[test]
    fn focus_ring_geometry_matches_the_spec() {
        let rect = Rect::from_min_size(egui::Pos2::new(100.0, 200.0), egui::Vec2::new(80.0, 24.0));
        let colors = ThemeColors::LIGHT;
        let geometry = focus_ring_geometry(rect, 8, &colors);

        assert_eq!(geometry.halo.rect, rect);
        assert_eq!(geometry.halo.stroke.width, FOCUS_HALO_WIDTH);
        assert_eq!(geometry.halo.stroke.width, 4.0);
        assert_eq!(geometry.halo.stroke.color, colors.surface);
        assert_eq!(geometry.halo.stroke_kind, StrokeKind::Outside);

        assert_eq!(geometry.ring.rect, rect.expand(FOCUS_RING_OFFSET));
        assert_eq!(geometry.ring.stroke.width, FOCUS_RING_WIDTH);
        assert_eq!(geometry.ring.stroke.width, 2.0);
        assert_eq!(geometry.ring.stroke.color, colors.focus_ring);
        assert_eq!(geometry.ring.stroke_kind, StrokeKind::Outside);

        assert_eq!(
            geometry.outer_edge(),
            rect.expand(4.0),
            "the treatment ends 4px out: 2px offset plus 2px outline"
        );
    }

    /// Fading scales opacity only: every channel (alpha included) by
    /// the same fraction, so the colours stay themselves.
    #[test]
    fn focus_ring_geometry_fades_both_shapes() {
        let rect = Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(80.0, 24.0));
        let colors = ThemeColors::LIGHT;
        let geometry = focus_ring_geometry(rect, 8, &colors);

        let half = geometry.faded(0.5);
        assert_eq!(half.halo.stroke.color.a(), 128);
        assert_eq!(half.ring.stroke.color.a(), 128);
        // Forest's ring is (139, 111, 71): half of each channel.
        assert_eq!(half.ring.stroke.color.r(), 70);
        assert_eq!(half.ring.stroke.color.g(), 56);
        assert_eq!(half.ring.stroke.color.b(), 36);

        assert_eq!(geometry.faded(0.25).ring.stroke.color.a(), 64);
        let full = geometry.faded(1.0);
        assert_eq!(full.ring.stroke.color, colors.focus_ring);
    }

    /// The flags default off, keep durations as given, and under
    /// reduced motion drop spatial transitions while clamping fades to
    /// 100ms: the focus fade's 120ms becomes 100.
    #[test]
    fn kit_flags_default_off_and_clamp_under_reduction() {
        let flags = UiKitFlags::default();
        assert!(!flags.reduced_motion && !flags.reduced_transparency);
        assert!(!flags.solid_surfaces());
        assert_eq!(flags.fade_ms(FOCUS_FADE_MS), 120.0);
        assert_eq!(flags.fade_ms(50.0), 50.0);
        assert_eq!(flags.spatial_ms(240.0), 240.0);

        let mut reduced = flags;
        reduced.reduced_motion = true;
        assert_eq!(reduced.fade_ms(FOCUS_FADE_MS), 100.0);
        assert_eq!(reduced.fade_ms(50.0), 50.0, "already under the cap");
        assert_eq!(reduced.spatial_ms(240.0), 0.0, "spatial transitions go");

        let mut solid = flags;
        solid.reduced_transparency = true;
        assert!(solid.solid_surfaces());
    }
}

//! Button: the design system's action component, the four emphases of
//! `components/Button` and section 13 of `colors_and_type.css`.
//!
//! Buttons trigger actions: quiet at rest, clearly responsive on
//! interaction. Primary fills `primary` with an `on-primary` label;
//! secondary sits transparent behind a 1px `outline` border with a
//! `primary` label; tertiary fills `tertiary` with an `on-tertiary`
//! label and is reserved for the one accent moment on a surface (a
//! rule of taste the kit documents here but cannot enforce); ghost sits
//! transparent with a `primary` label. Sizes run 36px (label, 13px),
//! 44px (body-sm, the default) and 52px (body-md), every size at
//! `radius-md`, the body face at weight 600, a `space-2` gap beside a
//! 20px icon.
//!
//! States follow the shared helpers ([`crate::states`]): hover is the
//! 8% on-colour overlay, press 12% plus the 0.985 scale held for as long
//! as the primary button stays down on the button, however long the hold
//! (a press is cancelled when the pointer leaves the button or the press
//! turns into a drag; see [`crate::states::held_press`]), keyboard focus
//! the FocusRing at `radius-md`, and
//! disabled the 38% content over a 12% container where the emphasis has
//! a container. The label is set in the theme's body face at weight 600
//! ([`crate::typography::body_semibold_family`]), 13/14/16px by size,
//! tracked open at the CSS's `letter-spacing: 0.01em`
//! ([`LABEL_TRACKING_EM`]), and icons ride at [`IconSize::Md`], with
//! `arrow_forward` trailing a call to action and `arrow_outward`
//! marking a link that leaves the app. Labels are verb-first sentence
//! case ("Start a project"); an
//! icon-only button names its action for assistive technology through
//! the required label of [`Button::icon_only`], carried on the egui
//! response's widget info.
//!
//! Documented deviations. The CSS hovers the emphases three ways (a 6%
//! brighten, a darken for tertiary, container fills for the transparent
//! two); the kit follows the README's shared state layer instead, the
//! same 8% on-colour overlay everywhere, so one helper drives every
//! component. The press scale and the overlays apply on the frame the
//! state lands (egui paints no tweened fills; under reduced motion the
//! scale goes and the overlays snap, per the flags). And the touch
//! floor: sizes sm is 36px, legitimate on pointer-precise surfaces, so
//! [`ButtonSize::touch_target`] exposes the 44px check the README asks
//! hosts to honour on touch surfaces rather than asserting it.

use egui::{Color32, Response, Sense, Stroke, Ui, Vec2};

use crate::components::FocusRing;
use crate::icon::IconSize;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale};

/// The touch floor the README sets: touch targets at least 44px tall,
/// so the default or large size on touch surfaces.
pub const TOUCH_TARGET_MIN: f32 = 44.0;

/// The CSS's `letter-spacing: 0.01em` on `.btn` labels: the hairline
/// of open tracking the label job carries between its glyphs.
pub const LABEL_TRACKING_EM: f32 = 0.01;

/// Which button, from the Button spec's one default and three
/// modifiers. Tertiary is the one accent moment per surface: never two
/// on the same screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonEmphasis {
    /// `btn`: primary, the main action on a surface. `primary` fill,
    /// `on-primary` label.
    #[default]
    Primary,
    /// `btn btn--secondary`: beside a primary action. Transparent, 1px
    /// `outline` border, `primary` label.
    Secondary,
    /// `btn btn--tertiary`: the one accent moment on a surface.
    /// `tertiary` fill, `on-tertiary` label (4.7:1, the lowest pair).
    Tertiary,
    /// `btn btn--ghost`: lowest emphasis, Cancel and inline actions.
    /// Transparent, `primary` label.
    Ghost,
}

impl ButtonEmphasis {
    /// Every emphasis, stylesheet order.
    pub const ALL: &[ButtonEmphasis] = &[
        ButtonEmphasis::Primary,
        ButtonEmphasis::Secondary,
        ButtonEmphasis::Tertiary,
        ButtonEmphasis::Ghost,
    ];

    /// The classes the emphasis takes.
    pub const fn name(self) -> &'static str {
        match self {
            ButtonEmphasis::Primary => "btn",
            ButtonEmphasis::Secondary => "btn btn--secondary",
            ButtonEmphasis::Tertiary => "btn btn--tertiary",
            ButtonEmphasis::Ghost => "btn btn--ghost",
        }
    }
}

/// One button size, from the spec's `btn--sm`, default and `btn--lg`:
/// 36px with `text-label`, 44px with `text-body-sm` (the default), 52px
/// with `text-body-md`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// `btn--sm`: 36px tall, 13px `text-label`, 8px 14px padding. For
    /// dense, pointer-precise surfaces; under the touch floor.
    Sm,
    /// The default: 44px tall, 14px `text-body-sm`, 12px 20px padding.
    /// The touch floor itself.
    #[default]
    Default,
    /// `btn--lg`: 52px tall, 16px `text-body-md`, 16px 28px padding.
    Lg,
}

/// Where an icon rides relative to the label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum IconSlot {
    /// Ahead of the label.
    #[default]
    Leading,
    /// After the label: `arrow_forward` trails a call to action.
    Trailing,
}

impl ButtonSize {
    /// Every size, stylesheet order.
    pub const ALL: &[ButtonSize] = &[ButtonSize::Sm, ButtonSize::Default, ButtonSize::Lg];

    /// The `min-height` in px.
    pub const fn height(self) -> f32 {
        match self {
            ButtonSize::Sm => 36.0,
            ButtonSize::Default => 44.0,
            ButtonSize::Lg => 52.0,
        }
    }

    /// The label's font size from the type scale.
    pub const fn text_size(self) -> f32 {
        match self {
            ButtonSize::Sm => type_scale::TEXT_LABEL,
            ButtonSize::Default => type_scale::TEXT_BODY_SM,
            ButtonSize::Lg => type_scale::TEXT_BODY_MD,
        }
    }

    /// The horizontal padding in px.
    pub const fn padding_x(self) -> f32 {
        match self {
            ButtonSize::Sm => 14.0,
            ButtonSize::Default => 20.0,
            ButtonSize::Lg => 28.0,
        }
    }

    /// The vertical padding in px.
    pub const fn padding_y(self) -> f32 {
        match self {
            ButtonSize::Sm => 8.0,
            ButtonSize::Default => 12.0,
            ButtonSize::Lg => 16.0,
        }
    }

    /// The classes the size takes.
    pub const fn name(self) -> &'static str {
        match self {
            ButtonSize::Sm => "btn btn--sm",
            ButtonSize::Default => "btn",
            ButtonSize::Lg => "btn btn--lg",
        }
    }

    /// Whether the size clears the README's 44px touch floor: the
    /// default and large sizes do, small does not, by design.
    pub const fn touch_target(self) -> bool {
        self.height() >= TOUCH_TARGET_MIN
    }
}

/// The button's resolved paints for one state: pure data off the tokens,
/// testable without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonLook {
    /// The fill: `primary` or `tertiary` for the filled emphases,
    /// `Color32::TRANSPARENT` for secondary and ghost, an overlay
    /// composited in on hover and press.
    pub fill: Color32,
    /// The label colour: the fill's on-colour, or `primary` on the
    /// transparent emphases.
    pub label: Color32,
    /// The secondary emphasis's 1px `outline` border.
    pub border: Option<Stroke>,
    /// `radius-md` for every emphasis and size.
    pub corner_radius: f32,
}

/// Resolve one emphasis's paints in one state, per theme. Hover and
/// press are the shared state layers (8% and 12% on-colour, composited
/// over a fill, the layer itself on a transparent one); disabled is the
/// 38% content over a 12% container where the emphasis has a container,
/// the content alone at 38% where it does not.
pub fn emphasis_look(emphasis: ButtonEmphasis, state: &WidgetState, theme: ThemeId) -> ButtonLook {
    let colors = theme.colors();
    // The emphasis's rest pair: the fill, its on-colour (which doubles
    // as the state layer's colour), and the secondary's border.
    let (fill, on, label, border, filled) = match emphasis {
        ButtonEmphasis::Primary => (
            colors.primary,
            colors.on_primary,
            colors.on_primary,
            None,
            true,
        ),
        ButtonEmphasis::Secondary => (
            Color32::TRANSPARENT,
            colors.primary,
            colors.primary,
            Some(Stroke::new(1.0, colors.outline)),
            false,
        ),
        ButtonEmphasis::Tertiary => (
            colors.tertiary,
            colors.on_tertiary,
            colors.on_tertiary,
            None,
            true,
        ),
        ButtonEmphasis::Ghost => (
            Color32::TRANSPARENT,
            colors.primary,
            colors.primary,
            None,
            false,
        ),
    };
    let mut look = ButtonLook {
        fill,
        label,
        border,
        corner_radius: radii::RADIUS_MD,
    };

    if state.disabled {
        // 38% content over a 12% container where the emphasis has a
        // container; the content alone (the secondary's border
        // included) where it does not.
        let pair = states::disabled(label, fill);
        look.label = pair.content;
        if filled {
            look.fill = pair.container;
        }
        if let Some(border) = &mut look.border {
            border.color = states::disabled_content(border.color);
        }
        return look;
    }

    // The state layers: the on-colour at 8% (hover) or 12% (press),
    // composited over a fill, the layer itself on a transparent one.
    let layer = if state.pressed {
        states::pressed_overlay(on)
    } else if state.hovered {
        states::hover_overlay(on)
    } else {
        return look;
    };
    look.fill = if filled {
        states::composite_over(layer, fill)
    } else {
        layer
    };
    look
}

/// The button's minimum width for measured content: horizontal padding
/// both sides, the 20px icon when one rides, the `space-2` gap between
/// an icon and a label. An icon-only button is square: its height.
pub fn min_width(size: ButtonSize, label_width: f32, icon: bool, icon_only: bool) -> f32 {
    if icon_only {
        // Square: the icon-only button is its own touch target.
        return size.height();
    }
    let icon_width = if icon {
        IconSize::Md.px() + spacing::SPACE_2
    } else {
        0.0
    };
    2.0 * size.padding_x() + icon_width + label_width
}

/// The label's layout job: the given face, tracked open at the CSS's
/// `0.01em`, in the placeholder colour the paint pass replaces with
/// the state-resolved label colour — the same delayed colour
/// `layout_delayed_color` produces, with the tracking riding the job.
fn label_job(label: &str, font_id: egui::FontId) -> egui::text::LayoutJob {
    crate::typography::tracked_text(
        label,
        font_id,
        LABEL_TRACKING_EM,
        None,
        Color32::PLACEHOLDER,
        None,
    )
}

/// The button widget: custom-painted (not `egui::Button`) so the states
/// follow the spec exactly, sensing interaction through `ui.interact`.
///
/// ```
/// use ui_kit::components::button::{Button, ButtonEmphasis};
///
/// let primary = Button::new("Start a project");
/// let off_site = Button::new("Visit the studio")
///     .icon_trailing(ui_kit::icons::Icon::ArrowOutward);
/// let search = Button::new("").icon_only(
///     ui_kit::icons::Icon::Search,
///     "Search the archive",
/// );
/// let quiet = Button::new("Cancel").emphasis(ButtonEmphasis::Ghost);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Button<'a> {
    label: &'a str,
    accessibility_label: Option<&'a str>,
    emphasis: ButtonEmphasis,
    size: ButtonSize,
    icon: Option<Icon>,
    slot: IconSlot,
    icon_only: bool,
    enabled: bool,
}

impl Default for Button<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<'a> Button<'a> {
    /// A primary button at the default size, labelled `label`.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            accessibility_label: None,
            emphasis: ButtonEmphasis::Primary,
            size: ButtonSize::Default,
            icon: None,
            slot: IconSlot::Leading,
            icon_only: false,
            enabled: true,
        }
    }

    /// Set the emphasis.
    pub const fn emphasis(mut self, emphasis: ButtonEmphasis) -> Self {
        self.emphasis = emphasis;
        self
    }

    /// The small size: 36px, for dense pointer surfaces.
    pub const fn small(self) -> Self {
        self.size(ButtonSize::Sm)
    }

    /// The large size: 52px.
    pub const fn large(self) -> Self {
        self.size(ButtonSize::Lg)
    }

    /// Set the size.
    pub const fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// An icon ahead of the label.
    pub const fn icon_leading(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.slot = IconSlot::Leading;
        self
    }

    /// An icon after the label: `arrow_forward` trails a call to
    /// action, `arrow_outward` a link that leaves the app.
    pub const fn icon_trailing(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.slot = IconSlot::Trailing;
        self
    }

    /// An icon-only button. The accessibility label is required by the
    /// signature (the spec: an icon-only button needs an `aria-label`
    /// naming the action); it rides the response's widget info.
    pub const fn icon_only(mut self, icon: Icon, accessibility_label: &'a str) -> Self {
        self.icon = Some(icon);
        self.accessibility_label = Some(accessibility_label);
        self.icon_only = true;
        self
    }

    /// Set enabled: a disabled button drops to 38% content over a 12%
    /// container and neither clicks nor takes focus.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the button.
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// The button's emphasis.
    pub const fn emphasis_kind(&self) -> ButtonEmphasis {
        self.emphasis
    }

    /// The button's size.
    pub const fn size_kind(&self) -> ButtonSize {
        self.size
    }

    /// Show the button in `ui` and hand back its typed response: the
    /// egui response, whether it fired this frame, and its four states.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> ButtonResponse {
        let size = self.size;
        let font_id = egui::FontId {
            size: size.text_size(),
            family: crate::typography::body_semibold_family(theme),
        };
        let galley = if self.icon_only {
            None
        } else {
            let job = label_job(self.label, font_id);
            Some(ui.ctx().fonts_mut(|fonts| fonts.layout_job(job)))
        };
        let label_width = galley.as_ref().map_or(0.0, |galley| galley.size().x);
        // The icon only rides when there is something beside it, or
        // when the button is icon-only by construction.
        let has_icon = self.icon.is_some() && (self.icon_only || galley.is_some());
        let width = min_width(size, label_width, has_icon, self.icon_only);
        let height = size.height();

        let id = ui.next_auto_id().with("vynatix-button");
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        // A disabled button neither clicks nor takes focus: it senses
        // nothing. egui's own add_enabled would multiply the painter's
        // opacity by Visuals::disabled_alpha on top of the spec's 38%,
        // so the kit resolves the disabled pair itself instead.
        let response = ui.interact(
            rect,
            id,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let mut state = WidgetState::of(&response);
        // egui drops its press once the hold outlives the click window;
        // the helper keeps the press while the primary button stays down.
        let hold = states::held_press(ui, &response);
        state.pressed |= hold.held;
        if !self.enabled {
            state.disabled = true;
        }
        let look = emphasis_look(self.emphasis, &state, theme);

        // The press scale, held while the pointer is down and dropped
        // under reduced motion (spatial effects go, the overlay stays).
        let scale = if state.pressed
            && !state.disabled
            && flags.spatial_ms(states::PRESS_DURATION_MS) > 0.0
        {
            states::PRESS_SCALE
        } else {
            1.0
        };
        let body = scaled_around_center(rect, scale);

        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        painter.rect_filled(body, look.corner_radius as u8, look.fill);
        if let Some(border) = look.border {
            painter.rect_stroke(
                body,
                look.corner_radius as u8,
                border,
                egui::StrokeKind::Inside,
            );
        }

        // The content: the icon at 20px and the label, a space-2 gap
        // between them, ordered by the slot, both centred in the body.
        // The icon takes room only when one rides, as in `min_width`: a
        // label-only button centres its label alone.
        let icon_px = IconSize::Md.px();
        let gap = if has_icon && !self.icon_only {
            spacing::SPACE_2
        } else {
            0.0
        };
        let icon_width = if has_icon { icon_px } else { 0.0 };
        let content_width = icon_width + gap + label_width;
        let start = body.center().x - content_width / 2.0;
        let icon = self.icon.filter(|_| has_icon);
        let (icon_x, label_x) = match (icon, galley.is_some()) {
            (Some(_), true) if self.slot == IconSlot::Trailing => {
                // The label first, the icon trailing it.
                (Some(start + label_width + gap), Some(start))
            }
            (Some(_), true) => (Some(start), Some(start + icon_px + gap)),
            (Some(_), false) => (Some(start), None),
            (None, _) => (None, Some(start)),
        };
        if let Some(icon) = icon {
            painter.text(
                egui::Pos2::new(icon_x.unwrap() + icon_px / 2.0, body.center().y),
                egui::Align2::CENTER_CENTER,
                icon.as_char().to_string(),
                egui::FontId {
                    size: icon_px,
                    family: crate::icon::family_for_theme(theme, false),
                },
                look.label,
            );
        }
        if let Some(galley) = galley {
            painter.galley(
                crate::typography::left_center(label_x.unwrap(), body.center().y, galley.size()),
                galley,
                look.label,
            );
        }

        // The accessibility name: the required label on an icon-only
        // button, the label text otherwise.
        let name = self.accessibility_label.unwrap_or(self.label);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, self.enabled, name)
        });

        FocusRing::new().paint(ui, &response, theme, flags);

        ButtonResponse {
            clicked: response.clicked() || hold.clicked,
            response,
            state,
        }
    }
}

/// `rect` scaled by `scale` around its centre: the press treatment.
fn scaled_around_center(rect: egui::Rect, scale: f32) -> egui::Rect {
    let center = rect.center();
    let size = rect.size() * scale;
    egui::Rect::from_center_size(center, size)
}

/// The button's response: the egui response plus the kit's typed reads.
#[derive(Clone, Debug)]
pub struct ButtonResponse {
    /// Whether the button fired this frame: a click, or a release over the
    /// button after a hold longer than egui's click window. Read this
    /// rather than `response.clicked()`, which does not see a long hold.
    pub clicked: bool,
    /// The wrapped egui response. Its `clicked()` misses the release after
    /// a long hold, so read [`Self::clicked`] instead.
    pub response: Response,
    /// The four states as of this frame.
    pub state: WidgetState,
}

#[cfg(test)]
mod tests {
    use egui::Color32;

    use crate::icon::IconSize;
    use crate::icons::Icon;
    use crate::states::{self, UiKitFlags, WidgetState};
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing, type_scale};
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        emphasis_look, label_job, min_width, Button, ButtonEmphasis, ButtonResponse, ButtonSize,
        LABEL_TRACKING_EM, TOUCH_TARGET_MIN,
    };

    /// Install the faces the button paints with: the icon families and
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

    /// The size table per the stylesheet: sm 36px with 13px text-label
    /// and 8x14 padding, the default 44px with 14px body-sm and 12x20,
    /// lg 52px with 16px body-md and 16x28. The touch floor holds for
    /// the default and large sizes; sm sits under it by design, for
    /// pointer-precise surfaces.
    #[test]
    fn sizes_match_the_stylesheet_table() {
        let expected: &[(ButtonSize, f32, f32, f32, f32, bool)] = &[
            (ButtonSize::Sm, 36.0, 13.0, 8.0, 14.0, false),
            (ButtonSize::Default, 44.0, 14.0, 12.0, 20.0, true),
            (ButtonSize::Lg, 52.0, 16.0, 16.0, 28.0, true),
        ];
        for (size, height, text, pad_y, pad_x, touch) in expected {
            assert_eq!(size.height(), *height, "{}: min-height", size.name());
            assert_eq!(size.text_size(), *text, "{}: font-size", size.name());
            assert_eq!(
                size.padding_y(),
                *pad_y,
                "{}: vertical padding",
                size.name()
            );
            assert_eq!(
                size.padding_x(),
                *pad_x,
                "{}: horizontal padding",
                size.name()
            );
            assert_eq!(
                size.touch_target(),
                *touch,
                "{}: the 44px touch floor",
                size.name()
            );
        }
        assert_eq!(TOUCH_TARGET_MIN, 44.0);
        assert_eq!(type_scale::TEXT_LABEL, 13.0);
        assert_eq!(type_scale::TEXT_BODY_SM, 14.0);
        assert_eq!(type_scale::TEXT_BODY_MD, 16.0);
    }

    /// The emphasis table per the Button spec, in every theme: primary
    /// fills primary with an on-primary label and no border; secondary
    /// sits transparent behind a 1px outline border with a primary
    /// label; tertiary fills tertiary with an on-tertiary label; ghost
    /// sits transparent with a primary label and no border. Every
    /// emphasis corners at radius-md.
    #[test]
    fn emphases_resolve_their_colour_pairs_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let rest = state("rest");

            let primary = emphasis_look(ButtonEmphasis::Primary, &rest, *theme);
            assert_eq!(primary.fill, colors.primary, "{name}: primary fill");
            assert_eq!(primary.label, colors.on_primary, "{name}: on-primary label");
            assert_eq!(primary.border, None, "{name}: no border");

            let secondary = emphasis_look(ButtonEmphasis::Secondary, &rest, *theme);
            assert_eq!(
                secondary.fill,
                Color32::TRANSPARENT,
                "{name}: secondary is transparent"
            );
            assert_eq!(secondary.label, colors.primary, "{name}: primary label");
            let border = secondary
                .border
                .expect("{name}: secondary carries the 1px outline");
            assert_eq!(border.width, 1.0, "{name}: 1px");
            assert_eq!(border.color, colors.outline, "{name}: outline colour");

            let tertiary = emphasis_look(ButtonEmphasis::Tertiary, &rest, *theme);
            assert_eq!(tertiary.fill, colors.tertiary, "{name}: tertiary fill");
            assert_eq!(
                tertiary.label, colors.on_tertiary,
                "{name}: on-tertiary label"
            );
            assert_eq!(tertiary.border, None, "{name}: no border");

            let ghost = emphasis_look(ButtonEmphasis::Ghost, &rest, *theme);
            assert_eq!(
                ghost.fill,
                Color32::TRANSPARENT,
                "{name}: ghost is transparent"
            );
            assert_eq!(ghost.label, colors.primary, "{name}: primary label");
            assert_eq!(ghost.border, None, "{name}: no border");

            for emphasis in ButtonEmphasis::ALL {
                let look = emphasis_look(*emphasis, &rest, *theme);
                assert_eq!(look.corner_radius, radii::RADIUS_MD, "{name}: radius-md");
            }
        }
    }

    /// Hover and press are the shared state layers: over a filled
    /// emphasis the 8% and 12% on-colour overlays composite into the
    /// fill (the same bytes the states tests pin: Cream over Forest at
    /// 8% is (47, 71, 60), at 12% (55, 79, 68)); over a transparent
    /// emphasis the layer itself becomes the fill, the on-colour at the
    /// state's alpha.
    #[test]
    fn hover_and_press_apply_the_state_layers() {
        let light = ThemeId::Light;
        let colors = light.colors();

        let hover = emphasis_look(ButtonEmphasis::Primary, &state("hover"), light);
        assert_eq!(
            hover.fill,
            states::composite_over(states::hover_overlay(colors.on_primary), colors.primary)
        );
        assert_eq!(hover.fill, Color32::from_rgb(47, 71, 60));

        let pressed = emphasis_look(ButtonEmphasis::Primary, &state("pressed"), light);
        assert_eq!(
            pressed.fill,
            states::composite_over(states::pressed_overlay(colors.on_primary), colors.primary)
        );
        assert_eq!(pressed.fill, Color32::from_rgb(55, 79, 68));

        let ghost_hover = emphasis_look(ButtonEmphasis::Ghost, &state("hover"), light);
        assert_eq!(ghost_hover.fill, states::hover_overlay(colors.primary));
        let ghost_press = emphasis_look(ButtonEmphasis::Ghost, &state("pressed"), light);
        assert_eq!(ghost_press.fill, states::pressed_overlay(colors.primary));

        // Press outranks hover when both land.
        let both = WidgetState {
            hovered: true,
            pressed: true,
            ..Default::default()
        };
        let pressed_look = emphasis_look(ButtonEmphasis::Primary, &both, light);
        assert_eq!(pressed_look.fill, pressed.fill, "press outranks hover");
    }

    /// Disabled is the 38% content over a 12% container where the
    /// emphasis has a container (primary, tertiary), and the content
    /// alone at 38% where it does not (secondary's border included,
    /// ghost's label).
    #[test]
    fn disabled_applies_the_spec_opacities() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let disabled = state("disabled");

            let primary = emphasis_look(ButtonEmphasis::Primary, &disabled, *theme);
            assert_eq!(
                primary.fill,
                states::disabled(colors.on_primary, colors.primary).container,
                "{name}: primary container at 12%"
            );
            assert_eq!(
                primary.label,
                states::disabled(colors.on_primary, colors.primary).content,
                "{name}: on-primary at 38%"
            );

            let tertiary = emphasis_look(ButtonEmphasis::Tertiary, &disabled, *theme);
            assert_eq!(
                tertiary.fill,
                states::disabled(colors.on_tertiary, colors.tertiary).container,
                "{name}: tertiary container at 12%"
            );

            let ghost = emphasis_look(ButtonEmphasis::Ghost, &disabled, *theme);
            assert_eq!(ghost.fill, Color32::TRANSPARENT, "{name}: no container");
            assert_eq!(
                ghost.label,
                states::disabled_content(colors.primary),
                "{name}: primary label at 38%"
            );

            let secondary = emphasis_look(ButtonEmphasis::Secondary, &disabled, *theme);
            assert_eq!(secondary.fill, Color32::TRANSPARENT);
            assert_eq!(secondary.label, states::disabled_content(colors.primary));
            let border = secondary.border.expect("{name}: the border stays");
            assert_eq!(border.width, 1.0);
            assert_eq!(border.color, states::disabled_content(colors.outline));
        }
    }

    /// The width math: horizontal padding both sides, the 20px icon and
    /// the space-2 gap when one rides beside a label, and the height
    /// alone for an icon-only button (it is square, its own touch
    /// target).
    #[test]
    fn min_width_accounts_for_padding_icon_and_gap() {
        for size in ButtonSize::ALL {
            let s = *size;
            assert_eq!(
                min_width(s, 100.0, false, false),
                2.0 * s.padding_x() + 100.0,
                "{}: text only",
                s.name()
            );
            assert_eq!(
                min_width(s, 100.0, true, false),
                2.0 * s.padding_x() + 20.0 + spacing::SPACE_2 + 100.0,
                "{}: icon beside the label",
                s.name()
            );
            assert_eq!(
                min_width(s, 0.0, true, true),
                s.height(),
                "{}: icon-only is square",
                s.name()
            );
        }
    }

    /// The label voice: the theme's body face at weight 600, which every
    /// theme installs (Aptos SemiBold in Forest, Geist SemiBold in Teal
    /// and Monochrome).
    #[test]
    fn labels_take_the_body_face_at_weight_600() {
        use crate::typography;
        for theme in ThemeId::ALL {
            let expected = match theme.name() {
                "Forest Light" | "Forest Dark" => "Aptos SemiBold",
                _ => "Geist SemiBold",
            };
            assert_eq!(
                typography::body_semibold_family_name(*theme),
                expected,
                "{}: --font-body at weight 600",
                theme.name()
            );
        }
    }

    /// The label job carries the stylesheet's `letter-spacing: 0.01em`
    /// on `.btn` labels: 0.13/0.14/0.16pt at the three sizes' 13/14/16px
    /// type, between the label's glyphs.
    #[test]
    fn label_jobs_carry_the_css_tracking() {
        for size in ButtonSize::ALL {
            let font_id = egui::FontId {
                size: size.text_size(),
                family: crate::typography::body_semibold_family(ThemeId::Light),
            };
            let job = label_job("Start a project", font_id);
            assert!(
                (job.sections[0].format.extra_letter_spacing
                    - LABEL_TRACKING_EM * size.text_size())
                .abs()
                    < 1e-6,
                "{}: 0.01em at {}px",
                size.name(),
                size.text_size()
            );
        }
    }

    /// A button lays out at its size's height, senses the click, takes
    /// focus, and reports its typed response; a disabled one renders at
    /// the same size, never fires and never focuses.
    #[test]
    fn button_renders_clicks_and_fits_the_touch_floor() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let flags = UiKitFlags::default();
        let mut id = None;
        let mut pos = None;
        let mut sizes = Vec::new();
        let mut focused = Vec::new();
        let mut clicked = Vec::new();

        // Pass 0, idle: record where the button landed and its id.
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Button::new("Start a project").ui(ui, ThemeId::Light, &flags);
            id = Some(response.response.id);
            pos = Some(response.response.rect.center());
            sizes.push(response.response.rect.size());
            focused.push(response.response.has_focus());
            clicked.push(response.clicked);
        })
        .drop_without_applying_deltas();
        ctx.memory_mut(|memory| memory.request_focus(id.unwrap()));

        // Pass 1, idle: the requested focus holds, nothing fires.
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Button::new("Start a project").ui(ui, ThemeId::Light, &flags);
            sizes.push(response.response.rect.size());
            focused.push(response.response.has_focus());
            clicked.push(response.clicked);
        })
        .drop_without_applying_deltas();

        // Pass 2 presses at the recorded centre; the press alone does
        // not fire. Pass 3 releases: the click fires.
        for pass in 2..4 {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::PointerMoved(pos.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: pos.unwrap(),
                button: egui::PointerButton::Primary,
                pressed: pass == 2,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let response = Button::new("Start a project").ui(ui, ThemeId::Light, &flags);
                sizes.push(response.response.rect.size());
                focused.push(response.response.has_focus());
                clicked.push(response.clicked);
            })
            .drop_without_applying_deltas();
        }

        assert_eq!(focused, vec![false, true, true, true], "Tab focus holds");
        assert_eq!(
            clicked,
            vec![false, false, false, true],
            "press then release fires, nothing sooner"
        );
        assert_eq!(
            sizes,
            vec![egui::Vec2::new(sizes[0].x, 44.0); 4],
            "the default size is the 44px touch floor, steady across states"
        );

        // A disabled button, the same two events: never fires.
        let mut fired = 0;
        for pass in 2..4 {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::PointerMoved(pos.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: pos.unwrap(),
                button: egui::PointerButton::Primary,
                pressed: pass == 2,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let response =
                    Button::new("Start a project")
                        .disabled()
                        .ui(ui, ThemeId::Light, &flags);
                if response.clicked {
                    fired += 1;
                }
                assert!(response.state.disabled, "the host flag reads through");
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(fired, 0, "a disabled button never fires");
    }

    /// One frame of a press script: `events` at `time` seconds (the clock
    /// egui's click window reads), the button shown, its response back.
    fn press_button(
        ctx: &egui::Context,
        time: f64,
        events: Vec<egui::Event>,
        button: Button<'_>,
    ) -> ButtonResponse {
        let flags = UiKitFlags::default();
        let input = egui::RawInput {
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut shown = None;
        ctx.run_ui(input, |ui| {
            shown = Some(button.ui(ui, ThemeId::Light, &flags));
        })
        .drop_without_applying_deltas();
        shown.expect("the button is shown every frame")
    }

    /// One frame of a press script on the default button.
    fn press_frame(ctx: &egui::Context, time: f64, events: Vec<egui::Event>) -> ButtonResponse {
        press_button(ctx, time, events, Button::new("Start a project"))
    }

    /// The pointer moved to `pos`.
    fn moved(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerMoved(pos)
    }

    /// The primary button pressed or released at `pos`.
    fn primary(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }

    /// The button's rect at rest, from an idle frame at 10s. The press
    /// scripts run from 11s.
    fn rest_rect(ctx: &egui::Context) -> egui::Rect {
        press_frame(ctx, 10.0, Vec::new()).response.rect
    }

    /// `quarters` idle frames a quarter second apart from `from`: the
    /// press held in place. Hands back every frame's response.
    fn hold(ctx: &egui::Context, from: f64, quarters: usize) -> Vec<ButtonResponse> {
        (1..=quarters)
            .map(|step| press_frame(ctx, from + 0.25 * step as f64, Vec::new()))
            .collect()
    }

    /// A press held two seconds, well past egui's 0.8s click window: the
    /// pressed look (and the scale that rides it) stays on through the
    /// whole hold, though egui has by then let the press go.
    #[test]
    fn a_long_hold_stays_pressed() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();

        let press = press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        assert!(press.state.pressed, "the press lands");
        assert!(!press.clicked, "a press is not a click");

        let frames = hold(&ctx, 11.0, 8);
        for frame in &frames {
            assert!(frame.state.pressed, "still pressed through the hold");
            assert!(!frame.clicked, "a hold is not a click before release");
        }
        let last = frames.last().expect("the hold has frames");
        assert!(
            !last.response.is_pointer_button_down_on(),
            "egui let the press go past its click window; the helper holds it"
        );
    }

    /// Releasing over the button after a long hold is a click on that
    /// release frame, and the button reads unpressed from then on.
    #[test]
    fn releasing_over_the_button_after_a_hold_clicks() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        hold(&ctx, 11.0, 8);

        let release = press_frame(&ctx, 13.25, vec![primary(at, false)]);
        assert!(
            release.clicked,
            "a release over the button after a hold clicks"
        );
        assert!(!release.state.pressed, "the release frame is unpressed");

        let after = press_frame(&ctx, 13.5, Vec::new());
        assert!(!after.clicked, "the click fires once");
        assert!(!after.state.pressed);
    }

    /// A hold dragged off the button and released there does not click,
    /// and the press reads unpressed from the moment the pointer leaves.
    #[test]
    fn a_hold_moved_off_the_button_and_released_does_not_click() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();
        let off = at + egui::Vec2::new(0.0, 60.0);
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        hold(&ctx, 11.0, 8);

        let away = press_frame(&ctx, 13.25, vec![moved(off)]);
        assert!(
            !away.state.pressed,
            "the pointer left the button: cancelled"
        );

        let release = press_frame(&ctx, 13.5, vec![primary(off, false)]);
        assert!(!release.clicked, "a release off the button does not click");
        assert!(!release.state.pressed);
    }

    /// A hold released off the button with no move first does not click
    /// either: the release position is what counts.
    #[test]
    fn a_hold_released_off_the_button_does_not_click() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();
        let off = at + egui::Vec2::new(0.0, 60.0);
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        hold(&ctx, 11.0, 8);

        let release = press_frame(&ctx, 13.25, vec![primary(off, false)]);
        assert!(!release.clicked, "a release off the button does not click");
        assert!(!release.state.pressed);
    }

    /// A press dragged past the click distance early on never comes back
    /// as a hold, however long the button stays down.
    #[test]
    fn a_press_dragged_early_never_comes_back_as_a_hold() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let rect = rest_rect(&ctx);
        let at = rect.center();
        let max_click_dist = ctx.options(|options| options.input_options.max_click_dist);
        let dragged = at + egui::Vec2::new(max_click_dist + 4.0, 0.0);
        assert!(rect.contains(dragged), "the drag stays on the button");

        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        let drag = press_frame(&ctx, 11.25, vec![moved(dragged)]);
        assert!(!drag.state.pressed, "egui drops the press on the drag");

        for frame in hold(&ctx, 11.25, 7) {
            assert!(!frame.state.pressed, "the dragged press never comes back");
            assert!(!frame.clicked, "a dragged press does not click");
        }
        let release = press_frame(&ctx, 13.25, vec![primary(dragged, false)]);
        assert!(
            !release.clicked,
            "a dragged press does not click on release"
        );
    }

    /// A drag that starts only after a long hold cancels the held press:
    /// the press clears on the drag, and the release does not click.
    #[test]
    fn a_drag_after_a_long_hold_cancels_the_press() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let rect = rest_rect(&ctx);
        let at = rect.center();
        let max_click_dist = ctx.options(|options| options.input_options.max_click_dist);
        let dragged = at + egui::Vec2::new(max_click_dist + 4.0, 0.0);
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        let held = hold(&ctx, 11.0, 8);
        assert!(held.last().is_some_and(|frame| frame.state.pressed));

        let drag = press_frame(&ctx, 13.25, vec![moved(dragged)]);
        assert!(!drag.state.pressed, "the drag cancels the held press");

        let release = press_frame(&ctx, 13.5, vec![primary(dragged, false)]);
        assert!(!release.clicked, "a cancelled press does not click");
    }

    /// A drag whose last move and release share one frame does not click:
    /// the move cancels the press before the release can count.
    #[test]
    fn a_drag_released_in_its_last_moves_frame_does_not_click() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let rect = rest_rect(&ctx);
        let at = rect.center();
        let max_click_dist = ctx.options(|options| options.input_options.max_click_dist);
        let dragged = at + egui::Vec2::new(max_click_dist + 4.0, 0.0);
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        hold(&ctx, 11.0, 8);

        let release = press_frame(&ctx, 13.25, vec![moved(dragged), primary(dragged, false)]);
        assert!(rect.contains(dragged), "the release lands on the button");
        assert!(
            !release.clicked,
            "a drag released in one frame does not click"
        );
        assert!(!release.state.pressed);
    }

    /// A wobble well inside the click distance during a long hold keeps
    /// the press, and the release over the button still clicks.
    #[test]
    fn a_wobble_inside_the_click_distance_still_clicks() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();
        let max_click_dist = ctx.options(|options| options.input_options.max_click_dist);
        let wobble_at = at + egui::Vec2::new(max_click_dist / 3.0, 0.0);
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        hold(&ctx, 11.0, 4);

        let wobble = press_frame(&ctx, 12.25, vec![moved(wobble_at)]);
        assert!(wobble.state.pressed, "a small wobble is not a drag");

        let release = press_frame(&ctx, 12.5, vec![primary(wobble_at, false)]);
        assert!(release.clicked, "the release over the button clicks");
    }

    /// Once a hold leaves the button the press stays cancelled, even when
    /// the pointer comes back before the release: no revival, no click.
    #[test]
    fn leaving_and_returning_does_not_revive_a_cancelled_hold() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();
        let off = at + egui::Vec2::new(0.0, 60.0);
        press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        hold(&ctx, 11.0, 8);

        let away = press_frame(&ctx, 13.25, vec![moved(off)]);
        assert!(!away.state.pressed);

        let back = press_frame(&ctx, 13.5, vec![moved(at)]);
        assert!(!back.state.pressed, "back over the button, still cancelled");

        let release = press_frame(&ctx, 13.75, vec![primary(at, false)]);
        assert!(!release.clicked, "a cancelled press does not click");
    }

    /// A quick tap clicks as before: pressed on the press frame, clicked
    /// on the release, unpressed after.
    #[test]
    fn a_quick_tap_still_clicks() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let at = rest_rect(&ctx).center();

        let press = press_frame(&ctx, 11.0, vec![moved(at), primary(at, true)]);
        assert!(press.state.pressed && !press.clicked);

        let release = press_frame(&ctx, 11.1, vec![primary(at, false)]);
        assert!(release.clicked, "a quick tap clicks on release");
        assert!(!release.state.pressed);
    }

    /// A disabled button takes no press: no pressed look, no click, however
    /// long the primary button stays down on it.
    #[test]
    fn a_disabled_button_holds_no_press() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let disabled = Button::new("Start a project").disabled();
        // Lay the disabled button out first: egui's hit test reads the
        // previous pass's senses, so an enabled warm-up would leak a click.
        let at = press_button(&ctx, 10.0, Vec::new(), disabled)
            .response
            .rect
            .center();

        let press = press_button(&ctx, 11.0, vec![moved(at), primary(at, true)], disabled);
        assert!(!press.state.pressed, "a disabled button takes no press");
        for step in 1..=8 {
            let frame = press_button(&ctx, 11.0 + 0.25 * step as f64, Vec::new(), disabled);
            assert!(!frame.state.pressed && !frame.clicked);
        }

        let release = press_button(&ctx, 13.25, vec![primary(at, false)], disabled);
        assert!(!release.clicked, "a disabled button never clicks");
    }

    /// An icon-only button is square, carries its required accessibility
    /// label, and centres the icon without a label.
    #[test]
    fn icon_only_buttons_are_square_and_labelled() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::TealDark);
        let flags = UiKitFlags::default();
        let mut sizes = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Button::new("")
                .icon_only(crate::icons::Icon::Search, "Search the archive")
                .ui(ui, ThemeId::TealDark, &flags);
            sizes.push(response.response.rect.size());
            assert_eq!(response.response.rect.width(), 44.0);
        })
        .drop_without_applying_deltas();
        assert!(sizes.iter().all(|size| size.x == size.y), "square");
    }

    /// The label starts where the button's padding puts it: one padding
    /// in from the left edge when it stands alone or leads a trailing
    /// icon, and after the 20px icon and its space-2 gap when the icon
    /// leads. The paint once also subtracted half the label's width from
    /// that left anchor, so "Start a project" read "a project" with its
    /// text running out of the button's left edge, and "Read the case
    /// study" read "e case study" over the button before it. And a
    /// label-only button once centred a block as wide as its label plus
    /// the 20px icon slot with no icon in it, which left the label 10px
    /// left of centre.
    #[test]
    fn labels_start_a_padding_in_from_the_button_edge() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();
        let padding = ButtonSize::Default.padding_x();

        let check = |what: &str, label: &str, button: Button, offset: f32| {
            let (rect, shapes) = paint_pass(&ctx, |ui| button.ui(ui, theme, &flags).response.rect);
            let pos = assert_painted_inside(&shapes, label, rect);
            let expected = rect.left() + offset;
            assert!(
                (pos.x - expected).abs() < 0.01,
                "{what}: starts at x {} but the padding puts it at {expected}",
                pos.x
            );
        };
        check(
            "label only",
            "Start a project",
            Button::new("Start a project"),
            padding,
        );
        check(
            "leading icon",
            "Read the case study",
            Button::new("Read the case study").icon_leading(Icon::ArrowForward),
            padding + IconSize::Md.px() + spacing::SPACE_2,
        );
        check(
            "trailing icon",
            "Visit the studio",
            Button::new("Visit the studio").icon_trailing(Icon::ArrowOutward),
            padding,
        );
    }
}

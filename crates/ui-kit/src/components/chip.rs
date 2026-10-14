//! Chip: the design system's compact filter pill, the looks of
//! `components/Chip` and the rules its preview carries (the `.chip`
//! rules live in the preview, not in `colors_and_type.css`).
//!
//! At rest a chip is transparent behind a 1px `outline` border with
//! `on-surface` text at 13px and weight 500; selected (an active
//! filter) it fills `secondary-container` with `on-secondary-container`
//! text plus the `check` icon ahead of the label, so the state never
//! rests on colour alone; the accent variant fills `tertiary-container`
//! with `on-tertiary-container` text and counts as the surface's one
//! tertiary accent. Every chip corners at `radius-full` with 6px 12px
//! padding, a 6px gap beside a 16px icon, and one-to-two-word
//! sentence-case labels ("Android Automotive", "Platform").
//!
//! A toggling chip is a control: it senses clicks, carries its pressed
//! state through `WidgetInfo::selected` (the stand-in for the spec's
//! `aria-pressed`) and takes the shared focus ring at its own pill
//! radius. Clicks come back as typed events ([`ChipEvent::Toggled`]);
//! a dismissible chip appends the 16px `close` icon and reports
//! [`ChipEvent::Dismissed`] when the click lands on it, the host
//! removing the chip from its list. States otherwise follow the shared
//! helpers: the 8% and 12% overlays, disabled at 38% content over a
//! 12% container where filled.
//!
//! Documented deviations. The spec's weight 500 ships as the theme's
//! Regular body face (Aptos 400 in Forest, Geist 400 in Teal and
//! Monochrome): no 500 instance of either is prepared, and taking the
//! 600 would read as a badge; the line box is set at 1.5, giving the
//! preview's ~32px pill. The label's 13px is `text-label`; the 6px gaps
//! are the preview's own values, not spacing tokens.

use egui::{Color32, CornerRadius, Response, Sense, Stroke, Ui, Vec2};

use crate::components::FocusRing;
use crate::icon::IconSize;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::type_scale;

/// The vertical padding: 6px, the preview's own value.
pub const CHIP_PAD_Y: f32 = 6.0;
/// The horizontal padding: 12px, the preview's own value.
pub const CHIP_PAD_X: f32 = 12.0;
/// The gap between an icon and the label: 6px, the preview's own value.
pub const CHIP_GAP: f32 = 6.0;
/// The label's line height: 1.5 over the 13px `text-label`.
pub const CHIP_LEADING: f32 = 1.5;

/// What a click on the chip meant, typed for the host's filter logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipEvent {
    /// The filter toggled; the bool is the new selected state.
    Toggled(bool),
    /// The click landed on the dismiss icon: remove the chip.
    Dismissed,
}

/// The chip's resolved paints for one state: pure data off the tokens,
/// testable without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChipLook {
    /// The fill: transparent at rest, `secondary-container` when
    /// selected, `tertiary-container` when accented, an overlay
    /// composited in on hover and press.
    pub fill: Color32,
    /// The label colour: `on-surface` at rest, the container's on-colour
    /// when selected or accented.
    pub label: Color32,
    /// The rest look's 1px `outline` border; the filled looks carry
    /// none (the preview sets the border to the fill).
    pub border: Option<Stroke>,
}

/// Resolve one chip look in one state, per theme: `selected` an active
/// filter, `accent` a featured label, both false the rest look. Hover
/// and press are the shared state layers over the look's own on-colour
/// (its label colour); disabled is 38% content over a 12% container
/// where filled.
pub fn chip_look(selected: bool, accent: bool, state: &WidgetState, theme: ThemeId) -> ChipLook {
    let colors = theme.colors();
    // The rest pair, its fill, label and border; the fill's on-colour
    // doubles as the state layer's colour.
    let (fill, label, border) = if selected {
        (
            colors.secondary_container,
            colors.on_secondary_container,
            None,
        )
    } else if accent {
        (
            colors.tertiary_container,
            colors.on_tertiary_container,
            None,
        )
    } else {
        (
            Color32::TRANSPARENT,
            colors.on_surface,
            Some(Stroke::new(1.0, colors.outline)),
        )
    };
    let filled = selected || accent;
    let mut look = ChipLook {
        fill,
        label,
        border,
    };

    if state.disabled {
        // 38% content over a 12% container where filled; the content
        // alone (the rest border included) where transparent.
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

    // The state layers over the look's own on-colour.
    let layer = if state.pressed {
        states::pressed_overlay(label)
    } else if state.hovered {
        states::hover_overlay(label)
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

/// The chip's height: the 6px padding twice over the 13px label's 1.5
/// line box, the preview's ~32px pill.
pub const fn chip_height() -> f32 {
    2.0 * CHIP_PAD_Y + type_scale::TEXT_LABEL * CHIP_LEADING
}

/// The chip's corner radius: `radius-full`, as a pill's half-height
/// clamped into a [`CornerRadius`] byte.
pub fn chip_radius() -> CornerRadius {
    CornerRadius::same((chip_height() / 2.0) as u8)
}

/// The chip's minimum width for a measured label and its icons: the
/// 12px padding twice, the leading icon and the dismiss icon at 16px
/// each with their 6px gaps.
pub fn chip_min_width(label_width: f32, leading_icon: bool, dismissible: bool) -> f32 {
    let icon_px = IconSize::Sm.px();
    let icons = if leading_icon {
        icon_px + CHIP_GAP
    } else {
        0.0
    } + if dismissible { icon_px + CHIP_GAP } else { 0.0 };
    2.0 * CHIP_PAD_X + icons + label_width
}

/// The chip widget: a pill-shaped filter or label with an optional
/// leading icon.
///
/// ```
/// use ui_kit::components::chip::Chip;
///
/// let rest = Chip::new("Platform");
/// let picked = Chip::new("Android Automotive").selected();
/// let featured = Chip::new("Featured").accent();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Chip<'a> {
    label: &'a str,
    selected: bool,
    accent: bool,
    icon: Option<Icon>,
    dismissible: bool,
    enabled: bool,
}

impl Default for Chip<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<'a> Chip<'a> {
    /// A chip at rest, labelled `label`.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            selected: false,
            accent: false,
            icon: None,
            dismissible: false,
            enabled: true,
        }
    }

    /// The selected look: an active filter, the `check` icon ahead of
    /// the label.
    pub const fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    /// The accent look: a featured label, `tertiary-container` with
    /// `on-tertiary-container` text. Counts as the surface's one
    /// tertiary accent.
    pub const fn accent(mut self) -> Self {
        self.accent = true;
        self
    }

    /// Set the selected flag.
    pub const fn set_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// A leading 16px icon.
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Append the dismiss icon; a click on it reports
    /// [`ChipEvent::Dismissed`] and the host removes the chip.
    pub const fn dismissible(mut self) -> Self {
        self.dismissible = true;
        self
    }

    /// Set enabled: a disabled chip drops to 38% content and neither
    /// clicks nor takes focus.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show the chip and hand back its typed response: the egui
    /// response plus the event a click meant, if any.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> ChipResponse {
        // The selected look puts the check ahead of the label so the
        // state does not rest on colour alone; the rest look keeps the
        // caller's icon, if any.
        let leading_icon = if self.selected {
            Some(Icon::Check)
        } else {
            self.icon
        };
        let font_id = egui::FontId {
            size: type_scale::TEXT_LABEL,
            family: crate::typography::TypeStyle::Body.family(theme),
        };
        let galley = ui.ctx().fonts_mut(|fonts| {
            fonts.layout_delayed_color(self.label.to_owned(), font_id, f32::INFINITY)
        });
        let label_width = galley.size().x;
        let width = chip_min_width(label_width, leading_icon.is_some(), self.dismissible);
        let height = chip_height();

        let id = ui.next_auto_id().with("vynatix-chip");
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
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
        if !self.enabled {
            state.disabled = true;
        }
        let look = chip_look(self.selected, self.accent, &state, theme);
        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        let radius = chip_radius();
        painter.rect_filled(rect, radius, look.fill);
        if let Some(border) = look.border {
            painter.rect_stroke(rect, radius, border, egui::StrokeKind::Inside);
        }

        // The content: the leading icon, the label, the dismiss icon,
        // 6px gaps between them, all centred.
        let icon_px = IconSize::Sm.px();
        let mut cursor = rect.left() + CHIP_PAD_X;
        if let Some(icon) = leading_icon {
            painter.text(
                egui::Pos2::new(cursor + icon_px / 2.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                icon.as_char().to_string(),
                egui::FontId {
                    size: icon_px,
                    family: crate::icon::family_for_theme(theme, self.selected),
                },
                look.label,
            );
            cursor += icon_px + CHIP_GAP;
        }
        painter.galley(
            crate::typography::left_center(cursor, rect.center().y, galley.size()),
            galley,
            look.label,
        );
        cursor += label_width + CHIP_GAP;
        let dismiss_rect = if self.dismissible {
            let rect = egui::Rect::from_min_size(
                egui::Pos2::new(cursor, rect.center().y - icon_px / 2.0),
                Vec2::splat(icon_px),
            );
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                Icon::Close.as_char().to_string(),
                egui::FontId {
                    size: icon_px,
                    family: crate::icon::family_for_theme(theme, false),
                },
                look.label,
            );
            Some(rect)
        } else {
            None
        };

        // The pressed (aria-pressed) semantics of a toggling filter.
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                self.enabled,
                self.selected,
                self.label,
            )
        });

        FocusRing::new()
            .radius(chip_radius())
            .paint(ui, &response, theme, flags);

        // The typed event: a click on the dismiss icon dismisses,
        // anything else toggles the filter.
        let event = if response.clicked() {
            if self.dismissible
                && dismiss_rect.is_some_and(|rect| {
                    response
                        .interact_pointer_pos()
                        .is_some_and(|pos| rect.contains(pos))
                })
            {
                Some(ChipEvent::Dismissed)
            } else {
                Some(ChipEvent::Toggled(!self.selected))
            }
        } else {
            None
        };

        ChipResponse { response, event }
    }
}

/// The chip's response: the egui response plus the typed event a click
/// meant, if any.
#[derive(Clone, Debug)]
pub struct ChipResponse {
    /// The wrapped egui response.
    pub response: Response,
    /// What a click meant, if the chip was clicked this frame.
    pub event: Option<ChipEvent>,
}

#[cfg(test)]
mod tests {
    use egui::Color32;

    use crate::icon::IconSize;
    use crate::states::{self, UiKitFlags, WidgetState};
    use crate::theme::ThemeId;
    use crate::tokens::{radii, type_scale};
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        chip_height, chip_look, chip_min_width, chip_radius, Chip, ChipEvent, CHIP_GAP,
        CHIP_LEADING, CHIP_PAD_X, CHIP_PAD_Y,
    };

    /// Install the faces the chip paints with: the icon families and the
    /// theme's type faces bind at the pass boundary, so a warm-up pass
    /// makes them resolvable for the widget passes that follow.
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

    /// The look table per the Chip preview, in every theme: rest is
    /// transparent behind a 1px `outline` border with `on-surface`
    /// text; selected fills `secondary-container` with
    /// `on-secondary-container` text and no border; accent fills
    /// `tertiary-container` with `on-tertiary-container` text.
    #[test]
    fn looks_match_the_preview_table_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let rest = state("rest");

            let unselected = chip_look(false, false, &rest, *theme);
            assert_eq!(unselected.fill, Color32::TRANSPARENT, "{name}: transparent");
            assert_eq!(
                unselected.label, colors.on_surface,
                "{name}: on-surface text"
            );
            let border = unselected
                .border
                .expect("{name}: the rest look carries the border");
            assert_eq!(border.width, 1.0, "{name}: 1px");
            assert_eq!(border.color, colors.outline, "{name}: outline colour");

            let selected = chip_look(true, false, &rest, *theme);
            assert_eq!(
                selected.fill, colors.secondary_container,
                "{name}: selected fills secondary-container"
            );
            assert_eq!(
                selected.label, colors.on_secondary_container,
                "{name}: on-secondary-container text"
            );
            assert_eq!(selected.border, None, "{name}: border folds into the fill");

            let accent = chip_look(false, true, &rest, *theme);
            assert_eq!(
                accent.fill, colors.tertiary_container,
                "{name}: accent fills tertiary-container"
            );
            assert_eq!(
                accent.label, colors.on_tertiary_container,
                "{name}: on-tertiary-container text"
            );
            assert_eq!(accent.border, None);
        }
    }

    /// Hover and press are the shared state layers over the look's own
    /// on-colour: over a filled look they composite (Cream's 8% over
    /// Forest's selected container), over the transparent rest look the
    /// layer itself becomes the fill, `on-surface` at the state's alpha.
    #[test]
    fn hover_and_press_apply_the_state_layers() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let selected_hover = chip_look(true, false, &state("hover"), *theme);
            assert_eq!(
                selected_hover.fill,
                states::composite_over(
                    states::hover_overlay(colors.on_secondary_container),
                    colors.secondary_container
                ),
                "{name}: selected hover composites"
            );

            let rest_hover = chip_look(false, false, &state("hover"), *theme);
            assert_eq!(
                rest_hover.fill,
                states::hover_overlay(colors.on_surface),
                "{name}: rest hover is the layer itself"
            );

            let rest_press = chip_look(false, false, &state("pressed"), *theme);
            assert_eq!(
                rest_press.fill,
                states::pressed_overlay(colors.on_surface),
                "{name}: rest press is the layer itself"
            );
        }
    }

    /// Disabled is 38% content over a 12% container where filled, the
    /// content alone at 38% (border included) where transparent.
    #[test]
    fn disabled_applies_the_spec_opacities() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let disabled = state("disabled");

            let selected = chip_look(true, false, &disabled, *theme);
            assert_eq!(
                selected.fill,
                states::disabled(colors.on_secondary_container, colors.secondary_container)
                    .container,
                "{name}: selected container at 12%"
            );
            assert_eq!(
                selected.label,
                states::disabled(colors.on_secondary_container, colors.secondary_container).content,
                "{name}: selected label at 38%"
            );

            let rest = chip_look(false, false, &disabled, *theme);
            assert_eq!(rest.fill, Color32::TRANSPARENT, "{name}: no container");
            assert_eq!(
                rest.label,
                states::disabled_content(colors.on_surface),
                "{name}: label at 38%"
            );
            let border = rest.border.expect("{name}: the border stays");
            assert_eq!(border.color, states::disabled_content(colors.outline));
        }
    }

    /// The geometry: 6px 12px padding over the 13px label's 1.5 line
    /// box (the preview's ~32px pill), a pill radius of half the
    /// height, and the width accounting for the 16px icons and their
    /// 6px gaps.
    #[test]
    fn geometry_matches_the_preview() {
        assert_eq!(CHIP_PAD_Y, 6.0);
        assert_eq!(CHIP_PAD_X, 12.0);
        assert_eq!(CHIP_GAP, 6.0);
        assert_eq!(CHIP_LEADING, 1.5);
        assert_eq!(type_scale::TEXT_LABEL, 13.0);
        assert_eq!(chip_height(), 2.0 * CHIP_PAD_Y + 13.0 * CHIP_LEADING);
        assert_eq!(chip_height(), 31.5);
        assert_eq!(chip_radius().nw, (chip_height() / 2.0) as u8);
        assert!(
            radii::RADIUS_FULL > chip_height(),
            "the pill is radius-full"
        );

        assert_eq!(
            chip_min_width(100.0, false, false),
            2.0 * CHIP_PAD_X + 100.0
        );
        assert_eq!(
            chip_min_width(100.0, true, false),
            2.0 * CHIP_PAD_X + 16.0 + CHIP_GAP + 100.0
        );
        assert_eq!(
            chip_min_width(100.0, true, true),
            2.0 * CHIP_PAD_X + 16.0 + CHIP_GAP + 100.0 + CHIP_GAP + 16.0
        );
    }

    /// A chip toggles as a typed event: the click comes back
    /// `Toggled(true)` from rest and `Toggled(false)` from selected (the
    /// check icon riding the selected look). A click on a dismissible
    /// chip's trailing icon comes back `Dismissed`, a click on its label
    /// `Toggled`.
    #[test]
    fn chips_toggle_and_dismiss_as_typed_events() {
        let ctx = egui::Context::default();
        // The icon families and the theme's faces bind at the pass
        // boundary: a warm-up pass makes them resolvable.
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();

        // Both filter chips in every frame, so the layout (and the id
        // each chip draws) stays put while the pointer works on them.
        let mut positions = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let rest = Chip::new("Platform").ui(ui, ThemeId::Light, &flags);
            let picked = Chip::new("Android Automotive")
                .selected()
                .ui(ui, ThemeId::Light, &flags);
            positions = vec![rest.response.rect.center(), picked.response.rect.center()];
        })
        .drop_without_applying_deltas();

        let events: Vec<Option<ChipEvent>> = positions
            .iter()
            .map(|&pos| {
                click(&ctx, pos, |ui| {
                    let rest = Chip::new("Platform").ui(ui, ThemeId::Light, &flags);
                    let picked =
                        Chip::new("Android Automotive")
                            .selected()
                            .ui(ui, ThemeId::Light, &flags);
                    if rest.response.hovered() {
                        rest.event
                    } else {
                        picked.event
                    }
                })
            })
            .collect();
        assert_eq!(
            events,
            vec![
                Some(ChipEvent::Toggled(true)),
                Some(ChipEvent::Toggled(false))
            ],
            "the toggle reports the new state, not the old"
        );

        // A dismissible chip: the click on the trailing icon dismisses,
        // a click on the label toggles.
        let mut dismiss_center = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Chip::new("Client work")
                .dismissible()
                .ui(ui, ThemeId::Light, &flags);
            let rect = response.response.rect;
            dismiss_center = Some(egui::Pos2::new(
                rect.right() - CHIP_PAD_X - 16.0 / 2.0,
                rect.center().y,
            ));
        })
        .drop_without_applying_deltas();

        let on_icon = click(&ctx, dismiss_center.unwrap(), |ui| {
            Chip::new("Client work")
                .dismissible()
                .ui(ui, ThemeId::Light, &flags)
                .event
        });
        assert_eq!(on_icon, Some(ChipEvent::Dismissed));

        let on_label = click(&ctx, positions[0], |ui| {
            Chip::new("Client work")
                .dismissible()
                .ui(ui, ThemeId::Light, &flags)
                .event
        });
        assert_eq!(on_label, Some(ChipEvent::Toggled(true)));
    }

    /// The label starts one chip padding in from the pill's left edge,
    /// or after the leading icon and its gap when one rides ahead of it
    /// (the selected look's check). The paint once also subtracted half
    /// the label's width from that left anchor, so the label ran out of
    /// the pill's left edge.
    #[test]
    fn labels_start_a_padding_in_from_the_pill_edge() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();

        let (rect, shapes) = paint_pass(&ctx, |ui| {
            Chip::new("Platform").ui(ui, theme, &flags).response.rect
        });
        let pos = assert_painted_inside(&shapes, "Platform", rect);
        assert!(
            (pos.x - (rect.left() + CHIP_PAD_X)).abs() < 0.01,
            "bare label: starts at x {} but one padding in is {}",
            pos.x,
            rect.left() + CHIP_PAD_X
        );

        let (rect, shapes) = paint_pass(&ctx, |ui| {
            Chip::new("Android Automotive")
                .selected()
                .ui(ui, theme, &flags)
                .response
                .rect
        });
        let pos = assert_painted_inside(&shapes, "Android Automotive", rect);
        let after_icon = rect.left() + CHIP_PAD_X + IconSize::Sm.px() + CHIP_GAP;
        assert!(
            (pos.x - after_icon).abs() < 0.01,
            "leading check: starts at x {} but after the icon and its gap is {after_icon}",
            pos.x
        );
    }

    /// Run a press then a release at `pos`, returning what `chip`
    /// reported on the release frame.
    fn click<T>(
        ctx: &egui::Context,
        pos: egui::Pos2,
        mut chip: impl FnMut(&mut egui::Ui) -> T,
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
                outcome = Some(chip(ui));
            })
            .drop_without_applying_deltas();
            if !pressed {
                result = outcome;
            }
        }
        result.expect("the release frame ran")
    }
}

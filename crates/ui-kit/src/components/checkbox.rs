//! Checkbox: the design system's yes-or-no (or any-number-from-a-group)
//! control, the looks of `components/Checkbox` and the rules its preview
//! carries (the `.check` rules live in the preview, not in
//! `colors_and_type.css`).
//!
//! An 18px box corners at `radius-xs` behind a 2px `outline` border on
//! `surface-container-low`; checked and indeterminate fill `primary`
//! and draw the Material Symbols `check` / `remove` glyph in
//! `on-primary`; the label sits beside the box in the 14px body face
//! with a 10px gap, its 20px line box setting the row's height. Rows
//! stack 12px apart inside a group whose legend, hint and error rows
//! come from the shared field helpers
//! ([`crate::components::text_field`]).
//!
//! Tri-state. The spec defines indeterminate — "a parent whose children
//! are partly chosen", a script-set property with no HTML attribute —
//! so [`CheckboxState`] carries it, [`CheckboxState::toggle`] follows
//! the HTML click model (indeterminate lands on checked), and
//! [`CheckboxState::from_children`] resolves a parent over its
//! children, the preview's own sync logic as a pure function. The host
//! owns the sync, exactly as the preview's script does.
//!
//! States otherwise follow the shared helpers: the 8% and 12% overlays
//! over the fill's own on-colour, the shared focus ring at the box's
//! own `radius-xs` (the box is the ring's rect; the label stays part of
//! the click target), `error` (`aria-invalid`) turning the border
//! `error`, and disabled dropping the box to the 38/12 pair with the
//! label in `text-muted`.
//!
//! Documented deviations. The spec draws the mark at wght 600 "so the
//! mark stays clear at this size", but the kit's icon faces are
//! instanced at the brand's single wght 300 (see
//! [`crate::icon`]), so the mark ships unfilled at 300 — the same
//! nearest-weight deviation the Chip records for its 500. egui's
//! accessibility slots have no mixed state, so an indeterminate box
//! reports unchecked through the checkbox slot; the `remove` glyph
//! carries the state visually.

use egui::{Color32, CornerRadius, FontId, Pos2, Response, Sense, Stroke, Ui, Vec2};

use crate::components::FocusRing;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::{radii, type_scale};
use crate::typography::TypeStyle;

/// The box: 18px square.
pub const BOX_SIZE: f32 = 18.0;
/// The border: 2px, `outline` at rest.
pub const BOX_BORDER: f32 = 2.0;
/// The box's corner radius: `radius-xs`.
pub const BOX_RADIUS: f32 = radii::RADIUS_XS;
/// The gap between the box and the label: the preview's `column-gap`.
pub const LABEL_GAP: f32 = 10.0;
/// The label: 14px `text-body-sm` in the body face, the shared
/// control-text convention.
pub const LABEL_SIZE: f32 = type_scale::TEXT_BODY_SM;
/// The label's 20px line box, the row's height.
pub const ROW_HEIGHT: f32 = 20.0;
/// The gap between rows inside a group: the preview's `.options` gap.
pub const ROW_GAP: f32 = 12.0;

/// The checkbox's three states, the spec's unchecked / checked /
/// indeterminate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckboxState {
    /// Off.
    #[default]
    Unchecked,
    /// On.
    Checked,
    /// A parent whose children are partly chosen.
    Indeterminate,
}

impl CheckboxState {
    /// The click model: unchecked and checked swap; indeterminate lands
    /// on checked (the HTML behaviour — clicking a mixed box selects
    /// everything it stands for).
    pub const fn toggle(self) -> Self {
        match self {
            Self::Unchecked | Self::Indeterminate => Self::Checked,
            Self::Checked => Self::Unchecked,
        }
    }

    /// True only when checked: what `input.checked` and the checkbox
    /// accessibility slot report.
    pub const fn is_checked(self) -> bool {
        matches!(self, Self::Checked)
    }

    /// A parent's state over its children: all checked, none, or
    /// partly — the preview's sync script as a pure function.
    pub const fn from_children(total: usize, checked: usize) -> Self {
        if checked == 0 {
            Self::Unchecked
        } else if checked == total {
            Self::Checked
        } else {
            Self::Indeterminate
        }
    }
}

/// The box's resolved paints for one state: pure data off the tokens,
/// testable without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxLook {
    /// The fill: `surface-container-low` at rest, `primary` when
    /// checked or indeterminate, an overlay composited in on hover and
    /// press.
    pub fill: Color32,
    /// The 2px border: `outline`, `primary` when filled, `error` on an
    /// error (which wins over both, per the preview's rule order).
    pub border: Stroke,
    /// The mark the box draws, if any: `check` or `remove` in
    /// `on-primary`.
    pub mark: Option<(Icon, Color32)>,
    /// The label colour: `text-body`, `text-muted` when disabled.
    pub label: Color32,
}

/// Resolve one checkbox in one state, per theme: `state` the tri-state
/// (the look of checked and indeterminate differs only in the glyph),
/// `error` the invalid flag. Hover and press are the shared state
/// layers over the fill's own on-colour; disabled is the 38/12 pair
/// over the box with the label in `text-muted`.
pub fn checkbox_look(
    state: CheckboxState,
    error: bool,
    widget: &WidgetState,
    theme: ThemeId,
) -> CheckboxLook {
    let colors = theme.colors();

    // The rest pair: the box on its container, the border outline, the
    // mark over the primary fill when one draws.
    let filled = !matches!(state, CheckboxState::Unchecked);
    let (fill, on_fill, mark) = if filled {
        (
            colors.primary,
            colors.on_primary,
            Some((
                if state.is_checked() {
                    Icon::Check
                } else {
                    Icon::Remove
                },
                colors.on_primary,
            )),
        )
    } else {
        (colors.surface_container_low, colors.on_surface, None)
    };
    let mut look = CheckboxLook {
        fill,
        border: Stroke::new(
            BOX_BORDER,
            if filled {
                colors.primary
            } else {
                colors.outline
            },
        ),
        mark,
        label: colors.text_body(),
    };

    if widget.disabled {
        // The shared pair over the box — the fill at 12%, the border
        // and mark colours at 38% — with the label in `text-muted`.
        let pair = states::disabled(on_fill, fill);
        look.fill = pair.container;
        look.border.color = states::disabled_content(look.border.color);
        if let Some((_, colour)) = &mut look.mark {
            *colour = states::disabled_content(colors.on_primary);
        }
        look.label = colors.text_muted();
        return look;
    }

    // The border is the error signal and wins over the checked fill's
    // own (the preview's rule order).
    if error {
        look.border.color = colors.error;
    }

    // The state layers over the fill's own on-colour, press outranking
    // hover.
    let layer = if widget.pressed {
        states::pressed_overlay(on_fill)
    } else if widget.hovered {
        states::hover_overlay(on_fill)
    } else {
        return look;
    };
    look.fill = states::composite_over(layer, fill);
    look
}

/// The box's rect inside its row: flush left, centred on the row's
/// 20px line box (the preview's 1px top margin centres the 18px box
/// against the label's line box).
pub fn box_rect(row: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        Pos2::new(row.left(), row.center().y - BOX_SIZE / 2.0),
        Vec2::splat(BOX_SIZE),
    )
}

/// The row's minimum width for a measured label: the box, the 10px
/// gap, the label.
pub fn row_min_width(label_width: f32) -> f32 {
    BOX_SIZE + LABEL_GAP + label_width
}

/// The checkbox widget: an 18px box with its label beside it, one row
/// of a group or standing alone.
///
/// ```
/// use ui_kit::components::checkbox::{Checkbox, CheckboxState};
///
/// let digest = Checkbox::new("Weekly digest");
/// let mixed = Checkbox::new("All practice areas").state(CheckboxState::Indeterminate);
/// let invalid = Checkbox::new("Email").error(true);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Checkbox<'a> {
    label: &'a str,
    state: CheckboxState,
    error: bool,
    enabled: bool,
}

impl Default for Checkbox<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<'a> Checkbox<'a> {
    /// An unchecked checkbox labelled `label`.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            state: CheckboxState::Unchecked,
            error: false,
            enabled: true,
        }
    }

    /// Set the tri-state explicitly.
    pub const fn state(mut self, state: CheckboxState) -> Self {
        self.state = state;
        self
    }

    /// Set the two-state flag: true is checked.
    pub const fn checked(mut self, checked: bool) -> Self {
        self.state = if checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        self
    }

    /// The invalid state (`aria-invalid`): the border turns `error`.
    pub const fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    /// Set enabled: a disabled box drops to the 38/12 pair, its label
    /// to `text-muted`, and neither clicks nor takes focus.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the checkbox.
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Show the checkbox and hand back its typed response: the row's
    /// egui response plus the state a click or a keyboard activation
    /// landed on, if any.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> CheckboxResponse {
        let font_id = FontId {
            size: LABEL_SIZE,
            family: TypeStyle::Body.family(theme),
        };
        let galley = ui.ctx().fonts_mut(|fonts| {
            fonts.layout_delayed_color(self.label.to_owned(), font_id, f32::INFINITY)
        });

        let size = Vec2::new(row_min_width(galley.size().x), ROW_HEIGHT);
        let id = ui.next_auto_id().with("vynatix-checkbox");
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
        let look = checkbox_look(self.state, self.error, &widget, theme);
        let painter = ui.painter().clone().with_layer_id(response.layer_id);

        let box_rect = box_rect(rect);
        let radius = CornerRadius::same(BOX_RADIUS as u8);
        painter.rect_filled(box_rect, radius, look.fill);
        painter.rect_stroke(box_rect, radius, look.border, egui::StrokeKind::Inside);
        if let Some((icon, color)) = look.mark {
            painter.text(
                box_rect.center(),
                egui::Align2::CENTER_CENTER,
                icon.as_char().to_string(),
                FontId {
                    size: BOX_SIZE,
                    family: crate::icon::family_for_theme(theme, false),
                },
                color,
            );
        }
        let label_pos = crate::typography::left_center(
            rect.left() + BOX_SIZE + LABEL_GAP,
            rect.center().y,
            galley.size(),
        );
        painter.galley(label_pos, galley, look.label);

        // The checkbox semantics; indeterminate reports unchecked
        // (egui's accessibility slots carry no mixed state).
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                self.enabled,
                self.state.is_checked(),
                self.label,
            )
        });

        // The ring belongs to the box, the click target to the row.
        FocusRing::new()
            .radius(radius)
            .paint_at(ui, &response, box_rect, theme, flags);

        // A click (mouse, touch, or Space/Enter through egui's focused
        // widget activation) takes the focus and reports the new state.
        let mut toggled = None;
        if response.clicked() {
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(response.id));
            toggled = Some(self.state.toggle());
        }

        CheckboxResponse { response, toggled }
    }
}

/// The checkbox's response: the row's egui response plus the typed
/// state a click landed on, if any.
#[derive(Clone, Debug)]
pub struct CheckboxResponse {
    /// The wrapped egui response.
    pub response: Response,
    /// The new [`CheckboxState`], if the box was clicked this frame.
    pub toggled: Option<CheckboxState>,
}

#[cfg(test)]
mod tests {
    use crate::states::{self, UiKitFlags, WidgetState};
    use crate::theme::ThemeId;
    use crate::tokens::{radii, type_scale};
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        box_rect, checkbox_look, row_min_width, Checkbox, CheckboxState, BOX_BORDER, BOX_RADIUS,
        BOX_SIZE, LABEL_GAP, LABEL_SIZE, ROW_GAP, ROW_HEIGHT,
    };

    /// Install the faces the checkbox paints with: the icon families and
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

    /// The look table per the Checkbox preview, in every theme: rest is
    /// `surface-container-low` behind a 2px `outline` border with no
    /// mark; checked and indeterminate fill `primary`, border
    /// `primary`, and draw their glyph in `on-primary`; an error turns
    /// the border `error` over both.
    #[test]
    fn looks_match_the_preview_table_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let rest = state("rest");

            let unchecked = checkbox_look(CheckboxState::Unchecked, false, &rest, *theme);
            assert_eq!(
                unchecked.fill, colors.surface_container_low,
                "{name}: rest fill"
            );
            assert_eq!(unchecked.border.width, BOX_BORDER, "{name}: 2px");
            assert_eq!(unchecked.border.color, colors.outline, "{name}: outline");
            assert_eq!(unchecked.mark, None, "{name}: no mark at rest");

            for (mixed, glyph) in [
                (CheckboxState::Checked, crate::icons::Icon::Check),
                (CheckboxState::Indeterminate, crate::icons::Icon::Remove),
            ] {
                let filled = checkbox_look(mixed, false, &rest, *theme);
                assert_eq!(filled.fill, colors.primary, "{name}: primary fill");
                assert_eq!(
                    filled.border.color, colors.primary,
                    "{name}: border folds into the fill"
                );
                let (icon, colour) = filled.mark.expect("{name}: the mark draws");
                assert_eq!(icon, glyph, "{name}: the right glyph");
                assert_eq!(colour, colors.on_primary, "{name}: on-primary mark");
            }

            let invalid = checkbox_look(CheckboxState::Unchecked, true, &rest, *theme);
            assert_eq!(invalid.border.color, colors.error, "{name}: error border");
            let invalid_filled = checkbox_look(CheckboxState::Checked, true, &rest, *theme);
            assert_eq!(
                invalid_filled.border.color, colors.error,
                "{name}: the error wins over the checked border"
            );
        }
    }

    /// Hover and press are the shared state layers over the fill's own
    /// on-colour: `on-surface` over the container at rest, `on-primary`
    /// over the primary fill when checked.
    #[test]
    fn hover_and_press_apply_the_state_layers() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let rest_hover =
                checkbox_look(CheckboxState::Unchecked, false, &state("hover"), *theme);
            assert_eq!(
                rest_hover.fill,
                states::composite_over(
                    states::hover_overlay(colors.on_surface),
                    colors.surface_container_low
                ),
                "{name}: rest hover composites"
            );

            let checked_press =
                checkbox_look(CheckboxState::Checked, false, &state("pressed"), *theme);
            assert_eq!(
                checked_press.fill,
                states::composite_over(states::pressed_overlay(colors.on_primary), colors.primary),
                "{name}: checked press composites"
            );
        }
    }

    /// Disabled is the shared pair over the box — the fill at 12%, the
    /// border and mark colours at 38% — with the label in `text-muted`.
    #[test]
    fn disabled_applies_the_spec_opacities() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let disabled = state("disabled");

            let rest = checkbox_look(CheckboxState::Unchecked, false, &disabled, *theme);
            assert_eq!(
                rest.fill,
                states::disabled(colors.on_surface, colors.surface_container_low).container,
                "{name}: the box fill at 12%"
            );
            assert_eq!(
                rest.border.color,
                states::disabled_content(colors.outline),
                "{name}: the border at 38%"
            );
            assert_eq!(rest.label, colors.text_muted(), "{name}: muted label");

            let checked = checkbox_look(CheckboxState::Checked, false, &disabled, *theme);
            assert_eq!(
                checked.fill,
                states::disabled(colors.on_primary, colors.primary).container,
                "{name}: the primary fill at 12%"
            );
            let (_, mark) = checked.mark.expect("{name}: the mark stays");
            assert_eq!(
                mark,
                states::disabled_content(colors.on_primary),
                "{name}: the mark at 38%"
            );
        }
    }

    /// The tri-state click model: unchecked and checked swap,
    /// indeterminate lands on checked (the HTML behaviour), and only
    /// `Checked` reports checked to the accessibility slot.
    #[test]
    fn tri_state_cycles_per_the_html_model() {
        assert_eq!(CheckboxState::Unchecked.toggle(), CheckboxState::Checked);
        assert_eq!(CheckboxState::Checked.toggle(), CheckboxState::Unchecked);
        assert_eq!(
            CheckboxState::Indeterminate.toggle(),
            CheckboxState::Checked
        );
        assert!(!CheckboxState::Unchecked.is_checked());
        assert!(!CheckboxState::Indeterminate.is_checked());
        assert!(CheckboxState::Checked.is_checked());
    }

    /// A parent over its children, the preview's sync script: all
    /// checked, none, or partly — the script's `n === kids.length`,
    /// `n > 0 && n < kids.length`.
    #[test]
    fn parents_resolve_over_their_children() {
        assert_eq!(CheckboxState::from_children(3, 0), CheckboxState::Unchecked);
        assert_eq!(CheckboxState::from_children(3, 3), CheckboxState::Checked);
        assert_eq!(
            CheckboxState::from_children(3, 1),
            CheckboxState::Indeterminate
        );
        assert_eq!(
            CheckboxState::from_children(3, 2),
            CheckboxState::Indeterminate
        );
    }

    /// The geometry: an 18px box at `radius-xs` flush left in a 20px
    /// row, the 10px gap and the 14px label beside it, rows stacking
    /// 12px apart.
    #[test]
    fn geometry_matches_the_spec() {
        assert_eq!(BOX_SIZE, 18.0);
        assert_eq!(BOX_BORDER, 2.0);
        assert_eq!(BOX_RADIUS, radii::RADIUS_XS);
        assert_eq!(LABEL_SIZE, type_scale::TEXT_BODY_SM);
        assert_eq!(LABEL_SIZE, 14.0);
        assert_eq!(ROW_HEIGHT, 20.0);
        assert_eq!(LABEL_GAP, 10.0);
        assert_eq!(ROW_GAP, 12.0);

        let row = egui::Rect::from_min_size(
            egui::Pos2::new(40.0, 100.0),
            egui::Vec2::new(row_min_width(120.0), ROW_HEIGHT),
        );
        let class = box_rect(row);
        // Centred on the row's 20px line box: 1px over and under.
        assert_eq!(
            class.min,
            egui::Pos2::new(40.0, row.center().y - BOX_SIZE / 2.0)
        );
        assert_eq!(class.size(), egui::Vec2::splat(BOX_SIZE));
        assert_eq!(row_min_width(120.0), BOX_SIZE + LABEL_GAP + 120.0);
    }

    /// A checkbox reports the state it lands on: a click on an
    /// unchecked box reports checked, on a checked one unchecked, on an
    /// indeterminate parent checked; Space on a focused box activates
    /// it through egui's focused-widget click; a disabled box reports
    /// nothing.
    #[test]
    fn checkboxes_report_the_state_they_land_on() {
        let ctx = egui::Context::default();
        // The icon families and the theme's faces bind at the pass
        // boundary: a warm-up pass makes them resolvable.
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();

        // Both rows in every frame, so the layout (and the id each
        // widget draws) stays put while the pointer works on them.
        let mut centers = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let digest = Checkbox::new("Weekly digest").ui(ui, ThemeId::Light, &flags);
            let picked = Checkbox::new("Blocked builds").ui(ui, ThemeId::Light, &flags);
            centers = vec![digest.response.rect.center(), picked.response.rect.center()];
        })
        .drop_without_applying_deltas();

        let off = click(&ctx, centers[0], |ui| {
            let digest = Checkbox::new("Weekly digest").ui(ui, ThemeId::Light, &flags);
            Checkbox::new("Blocked builds").ui(ui, ThemeId::Light, &flags);
            digest.toggled
        });
        assert_eq!(off, Some(CheckboxState::Checked));

        let on = click(&ctx, centers[1], |ui| {
            Checkbox::new("Weekly digest").ui(ui, ThemeId::Light, &flags);
            Checkbox::new("Blocked builds")
                .checked(true)
                .ui(ui, ThemeId::Light, &flags)
                .toggled
        });
        assert_eq!(on, Some(CheckboxState::Unchecked));

        let mixed = click(&ctx, centers[0], |ui| {
            let digest = Checkbox::new("All practice areas")
                .state(CheckboxState::Indeterminate)
                .ui(ui, ThemeId::Light, &flags);
            Checkbox::new("Blocked builds").ui(ui, ThemeId::Light, &flags);
            digest.toggled
        });
        assert_eq!(mixed, Some(CheckboxState::Checked));

        // Space activates the focused box, egui's focused-widget click.
        let mut seen_id = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Checkbox::new("Weekly digest").ui(ui, ThemeId::Light, &flags);
            seen_id = Some(response.response.id);
            assert_eq!(response.toggled, None);
        })
        .drop_without_applying_deltas();
        ctx.memory_mut(|memory| memory.request_focus(seen_id.unwrap()));
        let keyed = press_key(&ctx, egui::Key::Space, |ui| {
            Checkbox::new("Weekly digest")
                .ui(ui, ThemeId::Light, &flags)
                .toggled
        });
        assert_eq!(keyed, Some(CheckboxState::Checked));

        // Disabled: no click, no keyboard activation.
        let off = click(&ctx, centers[0], |ui| {
            let digest = Checkbox::new("Weekly digest")
                .disabled()
                .ui(ui, ThemeId::Light, &flags);
            Checkbox::new("Blocked builds").ui(ui, ThemeId::Light, &flags);
            digest.toggled
        });
        assert_eq!(off, None);
    }

    /// The label starts after the 18px box and its 10px gap. The paint
    /// once also subtracted half the label's width from that left
    /// anchor, so the label ran back over its own box.
    #[test]
    fn the_label_starts_after_the_box_and_its_gap() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();

        let (row, shapes) = paint_pass(&ctx, |ui| {
            Checkbox::new("Weekly digest")
                .ui(ui, theme, &flags)
                .response
                .rect
        });
        let pos = assert_painted_inside(&shapes, "Weekly digest", row);
        let after_box = row.left() + BOX_SIZE + LABEL_GAP;
        assert!(
            (pos.x - after_box).abs() < 0.01,
            "starts at x {} but after the box and its gap is {after_box}",
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

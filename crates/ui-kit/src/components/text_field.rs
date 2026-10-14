//! TextField: the design system's labelled text input, the forms rules
//! of section 12 in `colors_and_type.css` and the `.field` rules the
//! `components/TextField` preview carries (the stylesheet styles the
//! native controls; the field wrapper lives in the preview).
//!
//! A field is a column with a 6px gap: the always-visible label (13px
//! SemiBold in `text-display`), the control, and below it either the
//! hint or the error message, never both. The control fills
//! `surface-container-low` behind a 1px `outline` border at `radius-md`
//! with 12px by 14px padding, full width, and holds the 44px touch
//! floor. Focus moves the border to `focus-ring` and adds the shared
//! 2px ring over the 2px `surface` halo (the field's main state
//! signal); an error turns the border `error` and swaps the hint for
//! the message, the 16px `error` icon beside a 12px caption that says
//! what is wrong and how to fix it, never colour alone. A search field
//! leads with the 20px `search` icon in `text-muted` at 14px inside
//! the control, over 44px of left padding. Disabled is the kit's shared
//! 38% content over the 12% container.
//!
//! The kit wraps egui's [`TextEdit`]` rather than reimplementing text
//! editing: the edit ships frameless with its own margins, the kit
//! owns the chrome around it. The placeholder rides the edit's own
//! `hint_text`, which the applied theme already sets in `text-muted`.
//! Validation is the host's: the response reports
//! [`TextFieldResponse::lost_focus`], the "validate when the person
//! leaves the field, not while they type" moment, and the host hands
//! the message back through [`TextField::error`].
//!
//! Documented deviations. The stylesheet computes the control height
//! from the 14px control text's inherited line box; the kit pins the
//! 44px touch floor the `.btn` rule states and the README repeats.
//! The chrome resolves off the edit's previous-frame response (the
//! fill must paint under the text), so the border's focus and hover
//! colours land a frame late; the ring, painted over from the live
//! response, does not lag. egui has no `label for`, so the visual
//! label's association is positional only. And the field zeroes the
//! ui's item spacing for its own column so the spec's 6px gap is
//! exact rather than stacked on egui's default 8px.

use egui::{Align, FontSelection, TextEdit};
use egui::{
    Align2, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Response, Sense, Stroke, Ui,
    Vec2,
};

use crate::components::FocusRing;
use crate::icon::IconSize;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale};
use crate::typography::{self, TypeStyle};

/// The control's vertical padding: 12px, the stylesheet's forms rule.
pub const FIELD_PAD_Y: f32 = 12.0;
/// The control's horizontal padding: 14px.
pub const FIELD_PAD_X: f32 = 14.0;
/// The gap between the label, the control and the message: the
/// preview's `.field` column gap.
pub const FIELD_GAP: f32 = 6.0;
/// The single-line control's height: the 44px touch floor.
pub const CONTROL_HEIGHT: f32 = 44.0;
/// The control text: 14px `text-body-sm` in the theme's body face,
/// the button's control-text convention.
pub const CONTROL_TEXT_SIZE: f32 = type_scale::TEXT_BODY_SM;
/// The leading icon's inset from the control's left edge: 14px inside
/// the control, the search rule's own number.
pub const LEADING_ICON_INSET: f32 = 14.0;
/// The text's left inset when a leading icon rides: the search rule's
/// 44px of left padding.
pub const LEADING_TEXT_INSET: f32 = 44.0;

/// The field's resolved paints for one state: pure data off the tokens,
/// testable without a context. The rest pair is shared with Select;
/// only the disabled treatments differ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldLook {
    /// The fill: `surface-container-low`, an overlay composited in on
    /// hover and press, the 12% container when disabled.
    pub fill: Color32,
    /// The control text: `text-body`, the disabled content pair when
    /// disabled.
    pub text: Color32,
    /// The 1px border: `outline`, `focus-ring` under focus, `error` on
    /// an error (which holds under focus).
    pub border: Stroke,
    /// `radius-md`.
    pub corner_radius: f32,
}

/// Resolve one control box in one state, per theme: `error` names the
/// invalid state (`aria-invalid`), and hover and press are the shared
/// state layers over the fill. Disabled is the kit's 38% content over
/// the 12% container. The error border holds under focus, per the
/// Select spec's rule; the ring still paints in the `focus-ring`
/// colour.
pub fn field_look(error: bool, state: &WidgetState, theme: ThemeId) -> FieldLook {
    let colors = theme.colors();
    let mut look = FieldLook {
        fill: colors.surface_container_low,
        text: colors.text_body(),
        border: Stroke::new(1.0, colors.outline),
        corner_radius: radii::RADIUS_MD,
    };

    if state.disabled {
        // The kit's shared pair: the 12% container under the 38%
        // content, the border's own colour at 38%.
        let pair = states::disabled(colors.on_surface, colors.surface_container_low);
        return FieldLook {
            fill: pair.container,
            text: pair.content,
            border: Stroke::new(1.0, states::disabled_content(colors.outline)),
            corner_radius: radii::RADIUS_MD,
        };
    }

    // The border is the state signal: focus-ring under focus, error on
    // an error, and the error holds under focus.
    if error {
        look.border.color = colors.error;
    } else if state.has_focus {
        look.border.color = colors.focus_ring;
    }

    // The state layers: the shared 8% and 12% on-surface overlays
    // composited over the fill, press outranking hover.
    let layer = if state.pressed {
        states::pressed_overlay(colors.on_surface)
    } else if state.hovered {
        states::hover_overlay(colors.on_surface)
    } else {
        return look;
    };
    look.fill = states::composite_over(layer, look.fill);
    look
}

/// The leading icon's centre: the 20px glyph at 14px inside the
/// control, on its vertical centre.
pub fn leading_icon_center(rect: Rect) -> Pos2 {
    Pos2::new(
        rect.left() + LEADING_ICON_INSET + IconSize::Md.px() / 2.0,
        rect.center().y,
    )
}

/// The multiline control's height: the padding twice over `rows` of
/// the font's row height, the textarea rule.
pub fn multiline_height(rows: usize, row_height: f32) -> f32 {
    2.0 * FIELD_PAD_Y + (rows.max(1) as f32) * row_height
}

/// Paint the field's label row and hand back its rect: 13px SemiBold
/// in `text-display`, always visible. Shared with Select, whose
/// legends are set the same way.
pub fn label_row(ui: &mut Ui, theme: ThemeId, label: &str) -> Rect {
    let font = FontId {
        size: type_scale::TEXT_LABEL,
        family: typography::body_semibold_family(theme),
    };
    let galley = ui
        .ctx()
        .fonts_mut(|fonts| fonts.layout_delayed_color(label.to_owned(), font, f32::INFINITY));
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter()
        .galley(rect.left_top(), galley, theme.colors().text_display());
    rect
}

/// Paint the field's hint row and hand back its rect: a 12px caption
/// in `text-muted`.
pub fn hint_row(ui: &mut Ui, theme: ThemeId, hint: &str) -> Rect {
    let galley = ui.ctx().fonts_mut(|fonts| {
        fonts.layout_delayed_color(
            hint.to_owned(),
            TypeStyle::Caption.font_id(theme),
            f32::INFINITY,
        )
    });
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter()
        .galley(rect.left_top(), galley, theme.colors().text_muted());
    rect
}

/// Paint the field's error row and hand back its rect: the 16px
/// `error` icon beside a 12px caption in the `error` role, a 4px gap
/// between them, per the preview's `.field .error`.
pub fn error_row(ui: &mut Ui, theme: ThemeId, message: &str) -> Rect {
    let colors = theme.colors();
    let galley = ui.ctx().fonts_mut(|fonts| {
        fonts.layout_delayed_color(
            message.to_owned(),
            TypeStyle::Caption.font_id(theme),
            f32::INFINITY,
        )
    });
    let icon_px = IconSize::Sm.px();
    let gap = spacing::SPACE_1;
    let size = Vec2::new(
        icon_px + gap + galley.size().x,
        galley.size().y.max(icon_px),
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().text(
        Pos2::new(rect.left() + icon_px / 2.0, rect.center().y),
        Align2::CENTER_CENTER,
        Icon::Error.as_char().to_string(),
        FontId {
            size: icon_px,
            family: crate::icon::family_for_theme(theme, false),
        },
        colors.error,
    );
    let text = typography::left_center(rect.left() + icon_px + gap, rect.center().y, galley.size());
    ui.painter().galley(text, galley, colors.error);
    rect
}

/// The labelled text field: a `.field` column wrapping a restyled egui
/// [`TextEdit`], the kit owning the chrome.
///
/// ```
/// use ui_kit::components::text_field::TextField;
///
/// let mut email = String::new();
/// let field = TextField::new("Work email", &mut email)
///     .placeholder("name@company.se")
///     .hint("We will never sell or share your email.");
/// ```
#[derive(Debug)]
pub struct TextField<'a> {
    label: &'a str,
    text: &'a mut String,
    hint: Option<&'a str>,
    error: Option<&'a str>,
    placeholder: Option<&'a str>,
    leading_icon: Option<Icon>,
    rows: usize,
    enabled: bool,
}

impl<'a> TextField<'a> {
    /// A single-line field labelled `label`, editing `text`.
    pub const fn new(label: &'a str, text: &'a mut String) -> Self {
        Self {
            label,
            text,
            hint: None,
            error: None,
            placeholder: None,
            leading_icon: None,
            rows: 1,
            enabled: true,
        }
    }

    /// The hint below the control, 12px in `text-muted`. The error
    /// message replaces it when the field is invalid.
    pub const fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// The error state: the border turns `error` and the message
    /// replaces the hint, leading with the `error` icon. Say what is
    /// wrong and how to fix it.
    pub const fn error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }

    /// The placeholder inside the empty control, `text-muted`: a
    /// format hint, never a substitute for the label.
    pub const fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// A leading 20px icon in `text-muted`: the `search` icon, whose
    /// text then starts at the rule's 44px inset.
    pub const fn leading_icon(mut self, icon: Icon) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// The textarea variant: `rows` lines tall, the same chrome.
    pub const fn multiline(mut self, rows: usize) -> Self {
        self.rows = if rows < 1 { 1 } else { rows };
        self
    }

    /// Set enabled: a disabled field drops to the 38/12 pair and
    /// neither edits nor takes focus.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the field.
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Show the field in `ui` and hand back its typed response.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> TextFieldResponse {
        let colors = theme.colors();
        let font = FontId {
            size: CONTROL_TEXT_SIZE,
            family: TypeStyle::Body.family(theme),
        };

        // The field's column runs on exact gaps: egui's item spacing
        // (8px by default) would sit under the spec's 6px, so the
        // field zeroes it for its own run and restores it after.
        let saved_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        let label_rect = label_row(ui, theme, self.label);
        ui.add_space(FIELD_GAP);

        // The control box: full width, the touch floor tall (or the
        // textarea's rows). The id is read at a fixed structural point
        // so the edit's memory keys stay stable across frames.
        let id = ui.next_auto_id().with("vynatix-text-field");
        let edit_id = id.with("edit");
        let width = ui.available_width();
        let height = if self.rows > 1 {
            let row_height = ui.ctx().fonts_mut(|fonts| fonts.row_height(&font));
            multiline_height(self.rows, row_height)
        } else {
            CONTROL_HEIGHT
        };
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

        // The chrome resolves off the edit's previous-frame response:
        // the fill and border must paint under the text, so focus and
        // hover land a frame late here (the ring, painted over from
        // the live response, does not).
        let mut state = ui
            .ctx()
            .read_response(edit_id)
            .map(|response| WidgetState::of(&response))
            .unwrap_or_default();
        if !self.enabled {
            state = WidgetState {
                disabled: true,
                ..WidgetState::default()
            };
        }
        let look = field_look(self.error.is_some(), &state, theme);
        let radius = CornerRadius::same(radii::RADIUS_MD as u8);
        let painter = ui.painter().clone();
        painter.rect_filled(rect, radius, look.fill);
        painter.rect_stroke(rect, radius, look.border, egui::StrokeKind::Inside);
        if let Some(icon) = self.leading_icon {
            painter.text(
                leading_icon_center(rect),
                Align2::CENTER_CENTER,
                icon.as_char().to_string(),
                FontId {
                    size: IconSize::Md.px(),
                    family: crate::icon::family_for_theme(theme, false),
                },
                colors.text_muted(),
            );
        }

        // The edit itself: egui's TextEdit, restyled. No frame of its
        // own; the kit's chrome stands around it, the margins keeping
        // the text at the spec's insets (the search rule's 44px with a
        // leading icon, 14px without).
        let margin = Margin {
            left: (if self.leading_icon.is_some() {
                LEADING_TEXT_INSET
            } else {
                FIELD_PAD_X
            }) as i8,
            right: FIELD_PAD_X as i8,
            top: if self.rows > 1 { FIELD_PAD_Y as i8 } else { 0 },
            bottom: if self.rows > 1 { FIELD_PAD_Y as i8 } else { 0 },
        };
        let mut edit = if self.rows > 1 {
            TextEdit::multiline(self.text).desired_rows(self.rows)
        } else {
            TextEdit::singleline(self.text).vertical_align(Align::Center)
        };
        edit = edit
            .id(edit_id)
            .frame(Frame::NONE)
            .margin(margin)
            .min_size(rect.size())
            .font(FontSelection::FontId(font))
            .text_color(look.text)
            .interactive(self.enabled);
        if let Some(placeholder) = self.placeholder {
            edit = edit.hint_text(placeholder);
        }
        let response = ui.put(rect, edit);

        // The ring: the live response, the box's own rect.
        FocusRing::new().text_input().paint(ui, &response, theme, flags);

        // The message row: the error replaces the hint, the preview
        // pointing the field at one or the other.
        ui.add_space(FIELD_GAP);
        let message_rect = if let Some(error) = self.error {
            Some(error_row(ui, theme, error))
        } else {
            self.hint.map(|hint| hint_row(ui, theme, hint))
        };

        ui.spacing_mut().item_spacing = saved_spacing;

        let live = WidgetState::of(&response);
        TextFieldResponse {
            changed: response.changed(),
            lost_focus: response.lost_focus(),
            state: WidgetState {
                disabled: !self.enabled,
                ..live
            },
            response,
            label_rect,
            control_rect: rect,
            message_rect,
        }
    }
}

/// The field's response: the wrapped edit's egui response plus the
/// kit's typed reads and the row geometry (for hosts anchoring
/// popovers or describing the field to assistive technology).
#[derive(Debug)]
pub struct TextFieldResponse {
    /// The wrapped [`TextEdit`] response: focus, clicks, drags.
    pub response: Response,
    /// The text changed this frame.
    pub changed: bool,
    /// The field lost keyboard focus this frame: the validate-on-leave
    /// moment the spec names.
    pub lost_focus: bool,
    /// The four states as of this frame.
    pub state: WidgetState,
    /// The label row's rect.
    pub label_rect: Rect,
    /// The control box's rect.
    pub control_rect: Rect,
    /// The message row's rect: the error when invalid, otherwise the
    /// hint, `None` when the field carries neither.
    pub message_rect: Option<Rect>,
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, RawInput, Rect, Stroke, Vec2};

    use crate::icon::IconSize;
    use crate::states::{self, UiKitFlags, WidgetState};
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing, type_scale};
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        field_look, leading_icon_center, multiline_height, TextField, CONTROL_HEIGHT, FIELD_GAP,
        FIELD_PAD_X, FIELD_PAD_Y, LEADING_ICON_INSET, LEADING_TEXT_INSET,
    };

    /// Install the faces the field paints with: the icon families and
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
            "focus" => WidgetState {
                has_focus: true,
                ..Default::default()
            },
            "disabled" => WidgetState {
                disabled: true,
                ..Default::default()
            },
            other => panic!("unknown state {other}"),
        }
    }

    /// The box numbers per the stylesheet's forms rule and the
    /// preview's `.field` rules: 12px by 14px padding, the 6px column
    /// gap, radius-md, the 44px touch floor, 14px control text, the
    /// 20px leading icon at 14px inside over 44px of left padding
    /// (a 10px gap between the icon and the text).
    #[test]
    fn box_metrics_match_the_spec_numbers() {
        assert_eq!(FIELD_PAD_Y, 12.0, "12px vertical padding");
        assert_eq!(FIELD_PAD_X, 14.0, "14px horizontal padding");
        assert_eq!(FIELD_GAP, 6.0, "the .field column gap");
        assert_eq!(CONTROL_HEIGHT, 44.0, "the touch floor");
        assert_eq!(super::CONTROL_TEXT_SIZE, type_scale::TEXT_BODY_SM);
        assert_eq!(radii::RADIUS_MD, 8.0);
        assert_eq!(LEADING_ICON_INSET, 14.0, "the icon 14px inside");
        assert_eq!(LEADING_TEXT_INSET, 44.0, "44px of left padding");
        assert_eq!(IconSize::Md.px(), 20.0, "the 20px search icon");
        assert_eq!(
            LEADING_TEXT_INSET - LEADING_ICON_INSET - IconSize::Md.px(),
            10.0,
            "the gap between the icon and the text"
        );
        // The textarea: the padding twice over three rows.
        assert_eq!(multiline_height(3, 20.0), 2.0 * FIELD_PAD_Y + 60.0);
        assert_eq!(multiline_height(1, 20.0), 2.0 * FIELD_PAD_Y + 20.0);
    }

    /// The state colours per theme: rest fills surface-container-low
    /// behind the 1px outline border with text-body text; focus moves
    /// the border to focus-ring; error turns it error and holds under
    /// focus; hover and press are the shared 8% and 12% on-surface
    /// overlays composited over the fill; disabled is the 38% content
    /// over the 12% container, the border's colour at 38%.
    #[test]
    fn looks_resolve_the_state_colours_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let rest = field_look(false, &state("rest"), *theme);
            assert_eq!(rest.fill, colors.surface_container_low, "{name}: fill");
            assert_eq!(rest.text, colors.text_body(), "{name}: text");
            assert_eq!(
                rest.border,
                Stroke::new(1.0, colors.outline),
                "{name}: border"
            );
            assert_eq!(rest.corner_radius, radii::RADIUS_MD, "{name}: radius-md");

            let focus = field_look(false, &state("focus"), *theme);
            assert_eq!(
                focus.border.color, colors.focus_ring,
                "{name}: focus border"
            );
            assert_eq!(
                focus.fill, colors.surface_container_low,
                "{name}: fill holds"
            );

            let error = field_look(true, &state("rest"), *theme);
            assert_eq!(error.border.color, colors.error, "{name}: error border");
            let focused_error = field_look(true, &state("focus"), *theme);
            assert_eq!(
                focused_error.border.color, colors.error,
                "{name}: the error border holds under focus"
            );

            let hover = field_look(false, &state("hover"), *theme);
            assert_eq!(
                hover.fill,
                states::composite_over(
                    states::hover_overlay(colors.on_surface),
                    colors.surface_container_low
                ),
                "{name}: the 8% on-surface overlay"
            );
            let pressed = field_look(false, &state("pressed"), *theme);
            assert_eq!(
                pressed.fill,
                states::composite_over(
                    states::pressed_overlay(colors.on_surface),
                    colors.surface_container_low
                ),
                "{name}: the 12% on-surface overlay"
            );

            let disabled = field_look(false, &state("disabled"), *theme);
            let pair = states::disabled(colors.on_surface, colors.surface_container_low);
            assert_eq!(disabled.fill, pair.container, "{name}: the 12% container");
            assert_eq!(disabled.text, pair.content, "{name}: the 38% content");
            assert_eq!(
                disabled.border.color,
                states::disabled_content(colors.outline),
                "{name}: the border at 38%"
            );
        }
    }

    /// The leading icon's seat: centred on the control's mid line, its
    /// centre 24px from the left edge (the 14px inset plus the glyph's
    /// half).
    #[test]
    fn leading_icon_placement_math() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 200.0), Vec2::new(300.0, 44.0));
        assert_eq!(leading_icon_center(rect), Pos2::new(124.0, 222.0));
    }

    /// The field lays out its column: the label above the control at
    /// the 6px gap, the control at the touch floor and full width, the
    /// message below at the gap. The error replaces the hint, and a
    /// field with neither message ends at the control. The textarea
    /// grows past the touch floor.
    #[test]
    fn field_lays_out_label_control_and_message() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let flags = UiKitFlags::default();
        let theme = ThemeId::Light;

        let mut name = String::new();
        let mut hinted = None;
        let mut hinted_bottom = 0.0;
        ctx.run_ui(RawInput::default(), |ui| {
            let field = TextField::new("Project name", &mut name)
                .hint("As it appears in the proposal.")
                .ui(ui, theme, &flags);
            hinted = Some((field.label_rect, field.control_rect, field.message_rect));
            hinted_bottom = ui.min_rect().bottom();
        })
        .drop_without_applying_deltas();

        let (label, control, message) = hinted.expect("the rows are laid out");
        assert!(label.width() > 0.0, "the label paints");
        assert_eq!(
            label.bottom() + FIELD_GAP,
            control.top(),
            "the label sits the 6px gap above the control"
        );
        assert_eq!(control.height(), CONTROL_HEIGHT, "the touch floor");
        assert!(control.width() > 100.0, "full width");
        let message = message.expect("the hint row is laid out");
        assert_eq!(
            control.bottom() + FIELD_GAP,
            message.top(),
            "the message sits the 6px gap below the control"
        );

        // No hint, no error: the field ends at the control.
        let mut plain_bottom = 0.0;
        ctx.run_ui(RawInput::default(), |ui| {
            let field = TextField::new("Project name", &mut name).ui(ui, theme, &flags);
            assert!(field.message_rect.is_none(), "no message row");
            plain_bottom = ui.min_rect().bottom();
        })
        .drop_without_applying_deltas();
        assert!(
            plain_bottom < hinted_bottom,
            "the hintless field is shorter"
        );

        // The error replaces the hint and stands at least the icon tall.
        let mut errored = None;
        ctx.run_ui(RawInput::default(), |ui| {
            let field = TextField::new("Work email", &mut name)
                .hint("We will never sell or share your email.")
                .error("That address is missing its @ sign.")
                .ui(ui, theme, &flags);
            errored = field.message_rect;
        })
        .drop_without_applying_deltas();
        let error = errored.expect("the error row replaces the hint");
        assert!(
            error.height() >= IconSize::Sm.px(),
            "the error row stands at least the 16px icon tall"
        );

        // The textarea grows past the floor by its rows.
        let mut brief = String::new();
        let mut textarea = None;
        let mut measured = None;
        ctx.run_ui(RawInput::default(), |ui| {
            let font = egui::FontId {
                size: super::CONTROL_TEXT_SIZE,
                family: crate::typography::TypeStyle::Body.family(theme),
            };
            let row_height = ui.ctx().fonts_mut(|fonts| fonts.row_height(&font));
            measured = Some(row_height);
            let field = TextField::new("What are you trying to ship?", &mut brief)
                .multiline(3)
                .ui(ui, theme, &flags);
            textarea = Some(field.control_rect);
        })
        .drop_without_applying_deltas();
        let textarea = textarea.expect("the textarea is laid out");
        assert_eq!(
            textarea.height(),
            multiline_height(3, measured.expect("the row height is measured")),
            "the padding twice over three rows of the font's row height"
        );
        assert!(
            textarea.height() > CONTROL_HEIGHT + 2.0 * FIELD_PAD_Y,
            "three rows grow well past one"
        );
    }

    /// The message rows start where their rect says: the hint's text at
    /// the row's left edge, and the error's after the 16px error icon
    /// and its 4px gap. The error row once subtracted half the message's
    /// width from that left anchor, so the text ran back over its icon
    /// and out of the field's left edge (the Select and Radio error
    /// rows are this row).
    #[test]
    fn message_rows_start_where_their_rect_says() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();

        let hint = "As it appears in the proposal.";
        let mut name = String::new();
        let (row, shapes) = paint_pass(&ctx, |ui| {
            TextField::new("Project name", &mut name)
                .hint(hint)
                .ui(ui, theme, &flags)
                .message_rect
                .expect("the hint row is laid out")
        });
        let pos = assert_painted_inside(&shapes, hint, row);
        assert!(
            (pos.x - row.left()).abs() < 0.01,
            "hint: starts at x {} but the row's left edge is {}",
            pos.x,
            row.left()
        );

        let message = "That address is missing its @ sign.";
        let mut email = String::new();
        let (row, shapes) = paint_pass(&ctx, |ui| {
            TextField::new("Work email", &mut email)
                .error(message)
                .ui(ui, theme, &flags)
                .message_rect
                .expect("the error row is laid out")
        });
        let pos = assert_painted_inside(&shapes, message, row);
        let after_icon = row.left() + IconSize::Sm.px() + spacing::SPACE_1;
        assert!(
            (pos.x - after_icon).abs() < 0.01,
            "error: starts at x {} but after the icon and its gap is {after_icon}",
            pos.x
        );
    }

    /// The field edits through egui's TextEdit: requested focus holds,
    /// typed text lands in the buffer and reports changed, and leaving
    /// reports lost_focus exactly once.
    #[test]
    fn typing_edits_the_buffer_and_reports_change() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let flags = UiKitFlags::default();
        let theme = ThemeId::Light;
        let mut email = String::new();
        let mut id = None;

        ctx.run_ui(RawInput::default(), |ui| {
            let field = TextField::new("Work email", &mut email)
                .placeholder("name@company.se")
                .ui(ui, theme, &flags);
            id = Some(field.response.id);
            assert!(!field.changed, "nothing changes before typing");
            assert!(!field.lost_focus);
        })
        .drop_without_applying_deltas();
        ctx.memory_mut(|memory| memory.request_focus(id.unwrap()));

        let mut input = RawInput::default();
        input.events.push(egui::Event::Text("a".into()));
        ctx.run_ui(input, |ui| {
            let field = TextField::new("Work email", &mut email).ui(ui, theme, &flags);
            assert!(field.response.has_focus(), "the edit holds the focus");
            assert!(field.changed, "typing reports changed");
        })
        .drop_without_applying_deltas();
        assert_eq!(email, "a", "the typed text lands in the buffer");

        ctx.memory_mut(|memory| memory.surrender_focus(id.unwrap()));
        ctx.run_ui(RawInput::default(), |ui| {
            let field = TextField::new("Work email", &mut email).ui(ui, theme, &flags);
            assert!(field.lost_focus, "leaving reports the validate moment");
            assert!(!field.changed);
        })
        .drop_without_applying_deltas();
    }

    /// A disabled field neither takes focus nor edits: the state reads
    /// disabled and typed text goes nowhere.
    #[test]
    fn disabled_field_never_focuses_or_edits() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::TealDark);
        let flags = UiKitFlags::default();
        let theme = ThemeId::TealDark;
        let mut reference = String::from("A-2026-114");

        let mut input = RawInput::default();
        input.events.push(egui::Event::Text("x".into()));
        ctx.run_ui(input, |ui| {
            let field = TextField::new("Reference number", &mut reference)
                .disabled()
                .hint("Set by your contract.")
                .ui(ui, theme, &flags);
            assert!(field.state.disabled, "the state reads disabled");
            assert!(!field.response.has_focus(), "no focus");
            assert!(!field.changed, "no edits");
        })
        .drop_without_applying_deltas();
        assert_eq!(reference, "A-2026-114", "the buffer is untouched");
    }
}

//! Select: the design system's one-of-many picker, the forms rules of
//! section 12 in `colors_and_type.css` and the `.select` rules the
//! `components/Select` preview carries.
//!
//! A select is a text field's box with a trailing chevron that picks
//! one option from a list too long for radios: `surface-container-low`
//! behind the 1px `outline` border at `radius-md`, 12px by 14px
//! padding, 44px reserved on the right, where the 24px `expand_more`
//! icon sits 12px from the edge in `on-surface-variant`. The
//! unselected value shows the placeholder in `text-muted` (the
//! stylesheet's `select:invalid`); focus and error follow the field's
//! treatment, the error border holding under focus. Disabled is the
//! Select spec's own pair: `surface-container-high` fill,
//! `outline-variant` border, the value in `text-muted`, the icon at
//! 38%, and the hint saying why.
//!
//! The spec keeps the native `<select>` for its keyboard, screen
//! reader and mobile behaviour; egui has none, so the kit builds the
//! popover the deviation demands and keeps the behaviour native-shaped:
//! a click (or Enter, Space or ArrowDown on the focused box) opens the
//! list under the box; the arrow keys move a hover that wraps; Enter,
//! Space or a click commits and closes; Escape or a click outside
//! closes without committing. The list caps at eight rows and scrolls
//! inside that. The selected option carries the 20px `check` icon in
//! `primary`, so the state never rests on colour alone. The choice
//! comes back typed: [`SelectResponse::committed`] hands the host the
//! option value it was built from.
//!
//! The interaction is a pure machine ([`SelectInteraction`]) driven by
//! the widget, so the open-hover-commit-dismiss flow and its bounds
//! are testable without a context. The popover chrome follows the
//! box's tokens: `surface-container-low` at `radius-md` behind a 1px
//! `outline-variant` hairline and shadow-l2, the elevated card's
//! treatment.
//!
//! Documented deviations. egui 0.36 moves keyboard focus
//! directionally on the arrow keys, so while the list is open the
//! system focus drifts onto the option rows and the box's ring rests;
//! the choice returns the focus to the box as the list closes, and
//! the box takes it on a click the way a native select receives
//! Escape and Enter afterwards (egui hands focus only to Tab traversal
//! and explicit requests). And the rows paint square inside the
//! popover's rounded frame: their hover fill meets the frame's own
//! hairline, as Material menus do.

use egui::containers::{Popup, PopupCloseBehavior};
use egui::{
    Align2, CornerRadius, FontId, Frame, Key, Margin, Pos2, Rect, Response, ScrollArea, Sense,
    Stroke, Ui, Vec2,
};

use crate::components::focus_ring::FocusRing;
use crate::components::text_field::{
    error_row, field_look, hint_row, label_row, FieldLook, CONTROL_HEIGHT, CONTROL_TEXT_SIZE,
    FIELD_GAP, FIELD_PAD_X,
};
use crate::icon::IconSize;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing};
use crate::typography::TypeStyle;

/// The chevron's inset from the box's right edge: 12px, the Select
/// preview's own number.
pub const SELECT_ICON_INSET: f32 = 12.0;
/// The space reserved right of the value: 44px, the preview's
/// `padding-right`.
pub const SELECT_TEXT_INSET_RIGHT: f32 = 44.0;
/// One option row's height: the touch floor.
pub const OPTION_HEIGHT: f32 = CONTROL_HEIGHT;
/// The most rows the list shows before it scrolls.
pub const MAX_VISIBLE_OPTIONS: usize = 8;

/// The select's interaction as pure logic: whether the list is open
/// and which option the hover marks. The widget drives it from clicks
/// and keys; the whole open-hover-commit-dismiss flow and its bounds
/// are testable without a context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectInteraction {
    open: bool,
    hovered: Option<usize>,
}

impl SelectInteraction {
    /// The closed, hoverless state.
    pub const fn new() -> Self {
        Self {
            open: false,
            hovered: None,
        }
    }

    /// Whether the list is open.
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// The option the hover marks, if any.
    pub const fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    /// Open with the hover on the selected option: nothing hovered
    /// when nothing is selected or the index sits outside the list.
    pub fn open_at(&mut self, selected: Option<usize>, len: usize) {
        self.open = true;
        self.hovered = selected.filter(|index| *index < len);
    }

    /// Close without committing: Escape, a click outside, a toggle on
    /// the open box.
    pub fn dismiss(&mut self) {
        self.open = false;
        self.hovered = None;
    }

    /// Mark `index` hovered. Only while the list is open and the index
    /// sits inside it; reports whether the hover landed.
    pub fn hover(&mut self, len: usize, index: usize) -> bool {
        if !self.open || index >= len {
            return false;
        }
        self.hovered = Some(index);
        true
    }

    /// Step the hover by `delta` options, wrapping around the list.
    /// From nothing hovered, stepping down lands on the first option
    /// and stepping up on the last. Reports whether the hover moved.
    pub fn step(&mut self, len: usize, delta: i32) -> bool {
        if !self.open || len == 0 {
            return false;
        }
        let start = self
            .hovered
            .map(|index| index as i32)
            .unwrap_or(if delta > 0 { -1 } else { len as i32 });
        self.hovered = Some((start + delta).rem_euclid(len as i32) as usize);
        true
    }

    /// Commit the hovered option: hands back its index and closes the
    /// list. `None` when the list is closed or nothing is hovered, and
    /// nothing changes.
    pub fn commit(&mut self) -> Option<usize> {
        let index = self.hovered?;
        self.dismiss();
        Some(index)
    }
}

/// Resolve the select box in one state, per theme. The rest, focus,
/// error and overlay states are the field's own
/// ([`field_look`]); disabled is the Select spec's pair:
/// `surface-container-high` fill, `outline-variant` border, the value
/// in `text-muted` (the chevron drops to 38% in the widget).
pub fn select_look(error: bool, state: &WidgetState, theme: ThemeId) -> FieldLook {
    if state.disabled {
        let colors = theme.colors();
        return FieldLook {
            fill: colors.surface_container_high,
            text: colors.text_muted(),
            border: Stroke::new(1.0, colors.outline_variant),
            corner_radius: radii::RADIUS_MD,
        };
    }
    field_look(error, state, theme)
}

/// The chevron's centre: the 24px glyph's centre 12px in from the
/// box's right edge, on its vertical centre.
pub fn chevron_center(rect: Rect) -> Pos2 {
    Pos2::new(
        rect.right() - SELECT_ICON_INSET - IconSize::Lg.px() / 2.0,
        rect.center().y,
    )
}

/// The value text's width budget: the box's width less the 14px left
/// padding and the 44px the chevron reserves.
pub fn value_max_width(rect: Rect) -> f32 {
    (rect.width() - FIELD_PAD_X - SELECT_TEXT_INSET_RIGHT).max(0.0)
}

/// The list's height: the options' rows, capped at the eight visible
/// before the list scrolls.
pub fn popup_list_height(option_count: usize) -> f32 {
    option_count.min(MAX_VISIBLE_OPTIONS) as f32 * OPTION_HEIGHT
}

/// The popover's frame: the box's tokens elevated, `radius-md`, a 1px
/// `outline-variant` hairline and shadow-l2. The kit's token-correct
/// popover chrome, shared by the select's option list (issue 07) and
/// the top app bar's search field (issue 09).
pub fn popup_frame(theme: ThemeId) -> Frame {
    let colors = theme.colors();
    Frame {
        inner_margin: Margin::same(4),
        fill: colors.surface_container_low,
        stroke: Stroke::new(1.0, colors.outline_variant),
        corner_radius: CornerRadius::same(radii::RADIUS_MD as u8),
        shadow: crate::theme::egui_shadow(theme.shadows().l2),
        ..Default::default()
    }
}

/// The select widget: a text field's box with a chevron, opening a
/// popover list over whatever sits below.
///
/// ```
/// use ui_kit::components::select::Select;
///
/// #[derive(Clone, Copy, PartialEq)]
/// enum Region {
///     Nordics,
///     Europe,
///     Global,
/// }
///
/// const REGIONS: &[(Region, &str)] = &[
///     (Region::Nordics, "Nordics"),
///     (Region::Europe, "Europe"),
///     (Region::Global, "Global"),
/// ];
///
/// let mut region = Some(Region::Nordics);
/// // let picked = Select::new("Region")
/// //     .options(REGIONS)
/// //     .selected(region)
/// //     .hint("Where the team will work from.")
/// //     .ui(ui, theme, flags);
/// // if let Some(choice) = picked.committed {
/// //     region = Some(choice);
/// // }
/// ```
pub struct Select<'a, T: Copy + Eq> {
    label: &'a str,
    options: &'a [(T, &'a str)],
    selected: Option<T>,
    placeholder: Option<&'a str>,
    hint: Option<&'a str>,
    error: Option<&'a str>,
    enabled: bool,
}

impl<'a, T: Copy + Eq> Select<'a, T> {
    /// A select labelled `label`, carrying no options yet.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            options: &[],
            selected: None,
            placeholder: None,
            hint: None,
            error: None,
            enabled: true,
        }
    }

    /// The options, each with the label it shows.
    pub const fn options(mut self, options: &'a [(T, &'a str)]) -> Self {
        self.options = options;
        self
    }

    /// The current choice, `None` for the placeholder state.
    pub const fn selected(mut self, selected: Option<T>) -> Self {
        self.selected = selected;
        self
    }

    /// The placeholder shown while nothing is chosen, `text-muted`:
    /// "Choose a team size".
    pub const fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// The hint below the box; the error message replaces it.
    pub const fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// The error state: the border turns `error` and the message
    /// replaces the hint.
    pub const fn error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }

    /// Set enabled: a disabled select shows its own pair and neither
    /// opens nor takes focus; the hint says why.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the select.
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Show the select in `ui` and hand back its typed response.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> SelectResponse<T> {
        let colors = theme.colors();
        let options = self.options;
        let selected = self.selected;
        let len = options.len();

        // The field's column runs on exact gaps, like TextField.
        let saved_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        label_row(ui, theme, self.label);
        ui.add_space(FIELD_GAP);

        // The box: the field's own, full width at the touch floor.
        let id = ui.next_auto_id().with("vynatix-select");
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, CONTROL_HEIGHT), Sense::hover());
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
            state = WidgetState {
                disabled: true,
                ..WidgetState::default()
            };
        }
        let look = select_look(self.error.is_some(), &state, theme);
        let radius = CornerRadius::same(radii::RADIUS_MD as u8);
        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        painter.rect_filled(rect, radius, look.fill);
        painter.rect_stroke(rect, radius, look.border, egui::StrokeKind::Inside);

        // The value, or the placeholder set muted like select:invalid.
        let chosen = selected.filter(|value| options.iter().any(|(opt, _)| opt == value));
        let font = FontId {
            size: CONTROL_TEXT_SIZE,
            family: TypeStyle::Body.family(theme),
        };
        let shown = chosen
            .and_then(|value| options.iter().find(|(opt, _)| *opt == value))
            .map(|(_, name)| (*name).to_owned())
            .or_else(|| self.placeholder.map(str::to_owned))
            .unwrap_or_default();
        let value_color = if chosen.is_some() {
            look.text
        } else {
            colors.text_muted()
        };
        let galley = ui
            .ctx()
            .fonts_mut(|fonts| fonts.layout_delayed_color(shown, font, value_max_width(rect)));
        let value_pos = Pos2::new(rect.left() + FIELD_PAD_X, rect.center().y) - galley.size() / 2.0;
        painter.galley(value_pos, galley, value_color);

        // The chevron: 24px expand_more, 12px from the right edge, in
        // on-surface-variant, 38% when disabled.
        let chevron = if state.disabled {
            states::disabled_content(colors.on_surface_variant)
        } else {
            colors.on_surface_variant
        };
        painter.text(
            chevron_center(rect),
            Align2::CENTER_CENTER,
            Icon::ExpandMore.as_char().to_string(),
            FontId {
                size: IconSize::Lg.px(),
                family: crate::icon::family_for_theme(theme, false),
            },
            chevron,
        );

        FocusRing::new().paint(ui, &response, theme, flags);

        // The interaction: the pure machine, driven by clicks and keys,
        // its openness synced with egui's popup memory (the popup's own
        // outside-click and Escape closures are the source of truth).
        let ctx = ui.ctx().clone();
        let popup_id = id.with("popup");
        let mut interaction: SelectInteraction =
            ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(id, SelectInteraction::new));
        if !Popup::is_id_open(&ctx, popup_id) {
            interaction.dismiss();
        }

        let selected_index = options.iter().position(|(opt, _)| Some(*opt) == chosen);
        let mut committed: Option<T> = None;
        if self.enabled {
            if response.clicked() {
                // A click opens (or toggles) the list and takes the
                // focus, the way a native select receives Escape and
                // Enter after a click: egui only hands focus to Tab
                // traversal and explicit requests.
                ctx.memory_mut(|memory| memory.request_focus(id));
                if interaction.is_open() {
                    interaction.dismiss();
                } else {
                    interaction.open_at(selected_index, len);
                }
            }
            let (down, up, enter, escape, space) = ctx.input(|input| {
                (
                    input.key_pressed(Key::ArrowDown),
                    input.key_pressed(Key::ArrowUp),
                    input.key_pressed(Key::Enter),
                    input.key_pressed(Key::Escape),
                    input.key_pressed(Key::Space),
                )
            });
            if interaction.is_open() {
                if down {
                    interaction.step(len, 1);
                }
                if up {
                    interaction.step(len, -1);
                }
                if enter || space {
                    if let Some(index) = interaction.commit() {
                        committed = Some(options[index].0);
                        // The native shape: the choice lands and the
                        // focus returns to the box. egui's arrow-key
                        // focus traversal moves the system focus onto
                        // the rows while the list is open (and Escape
                        // clears it outright), so the box takes it
                        // back as the list closes.
                        ctx.memory_mut(|memory| memory.request_focus(id));
                    }
                }
                if escape {
                    interaction.dismiss();
                    ctx.memory_mut(|memory| memory.request_focus(id));
                }
            } else if response.has_focus() && (down || enter || space) {
                interaction.open_at(selected_index, len);
            }
        } else {
            interaction.dismiss();
        }

        if interaction.is_open() {
            Popup::open_id(&ctx, popup_id);
            let popup = Popup::from_response(&response)
                .id(popup_id)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .width(rect.width())
                .gap(4.0)
                .frame(popup_frame(theme));
            popup.show(|ui| {
                // The rows stack flush inside the popover.
                ui.spacing_mut().item_spacing.y = 0.0;
                ScrollArea::vertical()
                    .max_height(popup_list_height(len))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (index, (option, name)) in options.iter().enumerate() {
                            let width = ui.available_width().min(rect.width());
                            let (row_rect, _) = ui.allocate_exact_size(
                                Vec2::new(width, OPTION_HEIGHT),
                                Sense::hover(),
                            );
                            let row_response = ui.interact(
                                row_rect,
                                popup_id.with("option").with(index),
                                Sense::click(),
                            );
                            if row_response.hovered() {
                                interaction.hover(len, index);
                            }
                            if interaction.hovered() == Some(index) {
                                ui.painter().rect_filled(
                                    row_rect,
                                    CornerRadius::same(0),
                                    colors.surface_container,
                                );
                                ui.scroll_to_rect(row_rect, Some(egui::Align::Center));
                            }
                            if Some(*option) == chosen {
                                ui.painter().text(
                                    Pos2::new(
                                        row_rect.left() + FIELD_PAD_X + IconSize::Md.px() / 2.0,
                                        row_rect.center().y,
                                    ),
                                    Align2::CENTER_CENTER,
                                    Icon::Check.as_char().to_string(),
                                    FontId {
                                        size: IconSize::Md.px(),
                                        family: crate::icon::family_for_theme(theme, true),
                                    },
                                    colors.primary,
                                );
                            }
                            let galley = ui.ctx().fonts_mut(|fonts| {
                                fonts.layout_delayed_color(
                                    (*name).to_owned(),
                                    FontId {
                                        size: CONTROL_TEXT_SIZE,
                                        family: TypeStyle::Body.family(theme),
                                    },
                                    f32::INFINITY,
                                )
                            });
                            let label_x = row_rect.left()
                                + FIELD_PAD_X
                                + IconSize::Md.px()
                                + spacing::SPACE_2;
                            ui.painter().galley(
                                Pos2::new(label_x, row_rect.center().y) - galley.size() / 2.0,
                                galley,
                                colors.text_body(),
                            );
                            if row_response.clicked() {
                                interaction.hover(len, index);
                                if let Some(picked) = interaction.commit() {
                                    committed = Some(options[picked].0);
                                }
                            }
                        }
                    });
            });
            if !interaction.is_open() {
                Popup::close_id(&ctx, popup_id);
            }
        } else {
            Popup::close_id(&ctx, popup_id);
        }

        // egui's popup memory is the truth after the closing click; the
        // response reports it.
        let open = interaction.is_open() && Popup::is_id_open(&ctx, popup_id);
        if !open {
            interaction.dismiss();
        }
        ctx.data_mut(|data| data.insert_temp(id, interaction));

        // The message row, like the field's: the error replaces the hint.
        ui.add_space(FIELD_GAP);
        if let Some(error) = self.error {
            error_row(ui, theme, error);
        } else if let Some(hint) = self.hint {
            hint_row(ui, theme, hint);
        }

        ui.spacing_mut().item_spacing = saved_spacing;

        SelectResponse {
            response,
            committed,
            open,
        }
    }
}

/// The select's response: the box's egui response plus the typed
/// commit.
#[derive(Debug)]
pub struct SelectResponse<T> {
    /// The box's response: focus, clicks, hover.
    pub response: Response,
    /// The option committed this frame, if a choice landed.
    pub committed: Option<T>,
    /// Whether the list is open after this frame.
    pub open: bool,
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, RawInput, Rect, Stroke, Vec2};

    use crate::icon::IconSize;
    use crate::states::{UiKitFlags, WidgetState};
    use crate::theme::ThemeId;

    use super::{
        chevron_center, popup_list_height, select_look, value_max_width, Select, SelectInteraction,
        MAX_VISIBLE_OPTIONS, OPTION_HEIGHT, SELECT_ICON_INSET, SELECT_TEXT_INSET_RIGHT,
    };

    /// The Radio preview's example domain: the region.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Region {
        Nordics,
        Europe,
        Global,
    }

    const REGIONS: &[(Region, &str)] = &[
        (Region::Nordics, "Nordics"),
        (Region::Europe, "Europe"),
        (Region::Global, "Global"),
    ];

    /// Install the faces the select paints with.
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

    /// The machine the issue names: open at the selection, hover an
    /// option, step with the arrows (wrapping), commit on Enter.
    /// Committing hands back the index and closes; a closed machine
    /// commits nothing.
    #[test]
    fn interaction_opens_hovers_steps_and_commits() {
        let mut select = SelectInteraction::new();
        assert!(!select.is_open(), "starts closed");

        select.open_at(Some(1), 3);
        assert!(select.is_open());
        assert_eq!(select.hovered(), Some(1), "opens at the selection");

        assert!(select.step(3, 1));
        assert_eq!(select.hovered(), Some(2));
        assert!(select.step(3, 1));
        assert_eq!(select.hovered(), Some(0), "stepping wraps");

        assert_eq!(select.commit(), Some(0), "Enter commits the hover");
        assert!(!select.is_open(), "committing closes");
        assert_eq!(select.hovered(), None, "the hover clears");
        assert_eq!(select.commit(), None, "a closed machine commits nothing");
    }

    /// Escape and the outside click close without committing, from any
    /// hover.
    #[test]
    fn esc_and_outside_click_dismiss_without_committing() {
        let mut select = SelectInteraction::new();
        select.open_at(None, 3);
        assert!(select.hover(3, 2), "the pointer hovers an option");

        select.dismiss(); // Escape
        assert!(!select.is_open());
        assert_eq!(select.hovered(), None);
        assert_eq!(select.commit(), None, "no commit on dismissal");

        select.open_at(Some(0), 3);
        select.dismiss(); // the click outside
        assert!(!select.is_open());
        assert_eq!(select.hovered(), None);
    }

    /// The bounds hold the hover: out-of-range indexes never land, an
    /// out-of-range selection opens hoverless, stepping from nothing
    /// lands on the first or last option, and an empty list neither
    /// hovers, steps nor commits.
    #[test]
    fn bounds_hold_the_hover() {
        let mut select = SelectInteraction::new();
        select.open_at(Some(5), 3);
        assert_eq!(
            select.hovered(),
            None,
            "an out-of-range selection opens hoverless"
        );
        assert!(
            !select.hover(3, 3),
            "index 3 sits outside a three-option list"
        );
        assert_eq!(select.hovered(), None);
        assert!(select.hover(3, 2));
        assert_eq!(select.hovered(), Some(2));

        let mut down = SelectInteraction::new();
        down.open_at(None, 3);
        assert!(down.step(3, 1));
        assert_eq!(
            down.hovered(),
            Some(0),
            "from nothing, down lands on the first"
        );

        let mut up = SelectInteraction::new();
        up.open_at(None, 3);
        assert!(up.step(3, -1));
        assert_eq!(up.hovered(), Some(2), "from nothing, up lands on the last");

        let mut empty = SelectInteraction::new();
        empty.open_at(None, 0);
        assert!(empty.is_open(), "an empty list still opens");
        assert!(!empty.step(0, 1));
        assert_eq!(empty.commit(), None);

        let mut closed = SelectInteraction::new();
        assert!(!closed.hover(3, 0), "nothing hovers while closed");
        assert!(!closed.step(3, 1), "nothing steps while closed");
    }

    /// The box's states per theme: rest, focus and error are the
    /// field's own; disabled is the Select spec's pair,
    /// `surface-container-high` under an `outline-variant` border with
    /// the value in `text-muted`.
    #[test]
    fn looks_match_the_select_spec_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let rest = select_look(false, &state("rest"), *theme);
            assert_eq!(
                rest,
                crate::components::text_field::field_look(false, &state("rest"), *theme),
                "{name}: rest shares the field box"
            );
            let focus = select_look(false, &state("focus"), *theme);
            assert_eq!(
                focus.border.color, colors.focus_ring,
                "{name}: the focus border"
            );
            let error = select_look(true, &state("rest"), *theme);
            assert_eq!(error.border.color, colors.error, "{name}: the error border");

            let disabled = select_look(false, &state("disabled"), *theme);
            assert_eq!(
                disabled.fill, colors.surface_container_high,
                "{name}: the container-high fill"
            );
            assert_eq!(
                disabled.border,
                Stroke::new(1.0, colors.outline_variant),
                "{name}: the outline-variant border"
            );
            assert_eq!(
                disabled.text,
                colors.text_muted(),
                "{name}: the muted value"
            );
        }
    }

    /// The chevron's seat and the value's budget: the 24px glyph 12px
    /// from the right edge, the 14px left padding and the 44px
    /// reservation taken off the value's width. The list caps at eight
    /// rows of the touch floor, so nine options scroll.
    #[test]
    fn chevron_and_value_placement_math() {
        assert_eq!(IconSize::Lg.px(), 24.0, "the 24px expand_more");
        let rect = Rect::from_min_size(Pos2::new(100.0, 200.0), Vec2::new(300.0, 44.0));
        assert_eq!(chevron_center(rect), Pos2::new(376.0, 222.0));
        assert_eq!(SELECT_ICON_INSET, 12.0);
        assert_eq!(SELECT_TEXT_INSET_RIGHT, 44.0);
        assert_eq!(value_max_width(rect), 300.0 - 14.0 - 44.0);

        assert_eq!(OPTION_HEIGHT, 44.0, "rows at the touch floor");
        assert_eq!(MAX_VISIBLE_OPTIONS, 8);
        assert_eq!(popup_list_height(5), 5.0 * 44.0);
        assert_eq!(
            popup_list_height(9),
            8.0 * 44.0,
            "nine options scroll inside eight rows"
        );
    }

    /// The widget round trip, headless: a click opens the list over the
    /// box, the arrows move the hover from the selection, Enter commits
    /// the typed choice and closes, and the host's selection moves.
    #[test]
    fn select_opens_navigates_and_commits_by_keyboard() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let flags = UiKitFlags::default();
        let theme = ThemeId::Light;
        let mut region = Some(Region::Nordics);
        let mut center = None;

        // Pass 0: the closed box.
        ctx.run_ui(RawInput::default(), |ui| {
            let picked = Select::new("Region")
                .options(REGIONS)
                .selected(region)
                .hint("Where the team will work from.")
                .ui(ui, theme, &flags);
            assert!(!picked.open, "starts closed");
            assert_eq!(picked.committed, None);
            center = Some(picked.response.rect.center());
        })
        .drop_without_applying_deltas();

        // Passes 1 and 2: press and release on the box; the release
        // fires the click and the list opens.
        for pressed in [true, false] {
            let mut input = RawInput::default();
            input
                .events
                .push(egui::Event::PointerMoved(center.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: center.unwrap(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            let mut opened = false;
            ctx.run_ui(input, |ui| {
                let picked = Select::new("Region")
                    .options(REGIONS)
                    .selected(region)
                    .ui(ui, theme, &flags);
                opened = picked.open;
            })
            .drop_without_applying_deltas();
            if !pressed {
                assert!(opened, "the click opens the list");
            }
        }

        // Pass 3: ArrowDown moves the hover off the selection.
        let key = |key: egui::Key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        let mut input = RawInput::default();
        input.events.push(key(egui::Key::ArrowDown));
        ctx.run_ui(input, |ui| {
            let picked = Select::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, theme, &flags);
            assert!(picked.open, "the list stays open through the arrows");
        })
        .drop_without_applying_deltas();

        // Pass 4: Enter commits the hovered option, typed.
        let mut input = RawInput::default();
        input.events.push(key(egui::Key::Enter));
        let mut committed = None;
        ctx.run_ui(input, |ui| {
            let picked = Select::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, theme, &flags);
            assert!(!picked.open, "committing closes the list");
            committed = picked.committed;
        })
        .drop_without_applying_deltas();
        assert_eq!(
            committed,
            Some(Region::Europe),
            "the typed choice comes back"
        );
        region = committed.or(region);
        assert_eq!(region, Some(Region::Europe), "the host's selection moves");

        // Pass 5: Escape on a reopened list commits nothing.
        ctx.run_ui(RawInput::default(), |ui| {
            Select::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, theme, &flags);
        })
        .drop_without_applying_deltas();
        let mut input = RawInput::default();
        input.events.push(key(egui::Key::ArrowDown));
        ctx.run_ui(input, |ui| {
            let picked = Select::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, theme, &flags);
            assert!(picked.open);
        })
        .drop_without_applying_deltas();
        let mut input = RawInput::default();
        input.events.push(key(egui::Key::Escape));
        ctx.run_ui(input, |ui| {
            let picked = Select::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, theme, &flags);
            assert!(!picked.open, "Escape closes");
            assert_eq!(picked.committed, None, "without committing");
        })
        .drop_without_applying_deltas();
    }

    /// A disabled select never opens: clicks do nothing and the state
    /// reads disabled.
    #[test]
    fn disabled_select_never_opens() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::MonoLight);
        let flags = UiKitFlags::default();
        let theme = ThemeId::MonoLight;
        let mut center = None;

        ctx.run_ui(RawInput::default(), |ui| {
            let picked = Select::new("Invoice currency")
                .options(&[(0, "SEK")])
                .selected(Some(0))
                .disabled()
                .hint("Set by your contract.")
                .ui(ui, theme, &flags);
            center = Some(picked.response.rect.center());
        })
        .drop_without_applying_deltas();

        for pressed in [true, false] {
            let mut input = RawInput::default();
            input
                .events
                .push(egui::Event::PointerMoved(center.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: center.unwrap(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let picked = Select::new("Invoice currency")
                    .options(&[(0, "SEK")])
                    .selected(Some(0))
                    .disabled()
                    .ui(ui, theme, &flags);
                assert!(!picked.open, "a disabled select never opens");
                assert_eq!(picked.committed, None);
            })
            .drop_without_applying_deltas();
        }
    }
}

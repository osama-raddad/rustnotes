//! Radio group: the design system's pick-exactly-one control, the looks
//! of `components/Radio` and the rules its preview carries (the
//! `.radio` rules live in the preview, not in `colors_and_type.css`).
//!
//! An 18px circle behind a 2px `outline` border on
//! `surface-container-low`; selected, the border and an 8px dot turn
//! `primary`. The label sits beside the circle in the 14px body face
//! with a 10px gap, rows stack 12px apart, and the group's legend,
//! hint and error rows come from the shared field helpers — the spec
//! sets legends exactly like TextField labels.
//!
//! Group semantics. One value is selected, exactly one ([`Option`] by
//! construction), and a click on any row commits that row's value — a
//! click on the selected row commits nothing, the native radio
//! behaviour. The arrow keys move the choice through the enabled
//! options, wrapping, and the focus rides along; Space and Enter
//! activate the focused row through egui's focused-widget click.
//!
//! States otherwise follow the shared helpers: the 8% and 12% overlays
//! over the fill's own on-colour, the shared focus ring at the
//! circle's own full radius, `error` (`aria-invalid` on every option,
//! as the preview places it) turning the borders `error`, and disabled
//! dropping the circle to the 38/12 pair with the label in
//! `text-muted`.
//!
//! Documented deviations. The web group traps all four arrows and Tabs
//! in at the selected option; egui's tab order is declaration order
//! and its directional focus runs on its own, so the kit moves the
//! choice on the vertical arrows (the group's own axis; the
//! horizontals stay with egui's directional focus) and lets Tab walk
//! the rows. The web forced-colours mode has no egui counterpart, so
//! the dot's `currentColor` rule has no translation here.

use egui::{Color32, CornerRadius, FontId, Pos2, Response, Sense, Stroke, Ui, Vec2};

use crate::components::text_field::{error_row, hint_row, label_row};
use crate::components::FocusRing;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::spacing;
use crate::typography::TypeStyle;

/// The circle: 18px, the checkbox box's own geometry.
pub const CIRCLE_SIZE: f32 = crate::components::checkbox::BOX_SIZE;
/// The border: 2px, `outline` at rest.
pub const CIRCLE_BORDER: f32 = 2.0;
/// The selected dot: 8px, `primary`.
pub const DOT_SIZE: f32 = 8.0;
/// The gap between the circle and the label: the preview's
/// `column-gap`.
pub const LABEL_GAP: f32 = 10.0;
/// The label: 14px `text-body-sm` in the body face.
pub const LABEL_SIZE: f32 = crate::components::checkbox::LABEL_SIZE;
/// The row's 20px height, the label's line box.
pub const ROW_HEIGHT: f32 = crate::components::checkbox::ROW_HEIGHT;
/// The gap between rows: the preview's `.options` gap.
pub const ROW_GAP: f32 = spacing::SPACE_3;
/// The group hint's 4px gap under the legend, the preview's own value.
pub const HINT_GAP: f32 = spacing::SPACE_1;
/// The options' 12px top gap under the legend and hint.
pub const OPTIONS_GAP: f32 = spacing::SPACE_3;

/// One option of a radio group: its value, its label, and whether it
/// can be picked.
///
/// ```
/// use ui_kit::components::radio::{option, RadioGroup};
/// # #[derive(Clone, Copy, PartialEq, Eq)] enum Region { Nordics, Europe, Global }
/// const REGIONS: &[ui_kit::components::radio::RadioOption<Region>] = &[
///     option(Region::Nordics, "Nordics"),
///     option(Region::Europe, "Europe"),
/// ];
/// # let _ = RadioGroup::new("Region").options(REGIONS);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RadioOption<'a, T> {
    /// The value this row commits.
    pub value: T,
    /// The row's label.
    pub label: &'a str,
    /// Whether the row can be picked.
    pub enabled: bool,
}

/// One selectable option, for building the options list.
pub const fn option<T>(value: T, label: &str) -> RadioOption<'_, T> {
    RadioOption {
        value,
        label,
        enabled: true,
    }
}

impl<'a, T> RadioOption<'a, T> {
    /// Disable this option: its circle drops to the 38/12 pair and the
    /// arrows skip it.
    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// The circle's resolved paints for one option in one state: pure data
/// off the tokens, testable without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadioLook {
    /// The fill: `surface-container-low`, an overlay composited in on
    /// hover and press.
    pub fill: Color32,
    /// The 2px border: `outline`, `primary` when selected, `error` on
    /// an error.
    pub border: Stroke,
    /// The selected dot's colour, if the dot draws.
    pub dot: Option<Color32>,
    /// The label colour: `text-body`, `text-muted` when disabled.
    pub label: Color32,
}

/// Resolve one option in one state, per theme: `selected` whether this
/// row holds the group's choice, `error` the group's invalid flag.
/// Hover and press are the shared state layers over the fill's own
/// on-colour; disabled is the 38/12 pair with the label in
/// `text-muted`.
pub fn radio_look(selected: bool, error: bool, widget: &WidgetState, theme: ThemeId) -> RadioLook {
    let colors = theme.colors();
    let mut look = RadioLook {
        fill: colors.surface_container_low,
        border: if selected {
            Stroke::new(CIRCLE_BORDER, colors.primary)
        } else {
            Stroke::new(CIRCLE_BORDER, colors.outline)
        },
        dot: selected.then_some(colors.primary),
        label: colors.text_body(),
    };

    if widget.disabled {
        // The shared pair over the circle — the fill at 12%, the
        // border and dot at 38% — with the label in `text-muted`.
        let pair = states::disabled(colors.on_surface, colors.surface_container_low);
        look.fill = pair.container;
        look.border.color = states::disabled_content(look.border.color);
        look.dot = look.dot.map(|_| states::disabled_content(colors.primary));
        look.label = colors.text_muted();
        return look;
    }

    // The border is the error signal and wins over selection (the
    // preview places `aria-invalid` on every option).
    if error {
        look.border.color = colors.error;
    }

    // The state layers over the fill's own on-colour, press outranking
    // hover.
    let layer = if widget.pressed {
        states::pressed_overlay(colors.on_surface)
    } else if widget.hovered {
        states::hover_overlay(colors.on_surface)
    } else {
        return look;
    };
    look.fill = states::composite_over(layer, look.fill);
    look
}

/// The circle's rect inside its row, centred on the 20px line box.
pub fn circle_rect(row: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        Pos2::new(row.left(), row.center().y - CIRCLE_SIZE / 2.0),
        Vec2::splat(CIRCLE_SIZE),
    )
}

/// The row's minimum width for a measured label.
pub fn row_min_width(label_width: f32) -> f32 {
    CIRCLE_SIZE + LABEL_GAP + label_width
}

/// The option the arrows move the choice to: from the focused `index`,
/// `delta` rows away (wrapping), skipping disabled options. `None`
/// when every option is disabled or the list is empty.
pub fn step_selection<T>(
    options: &[RadioOption<'_, T>],
    index: usize,
    delta: i32,
) -> Option<usize> {
    let len = options.len();
    if len == 0 {
        return None;
    }
    // One lap at most: every step either finds an enabled option or
    // comes back to where it started.
    let mut step = index;
    for _ in 0..len {
        // Euclidean stepping, wrapping in either direction.
        step = if delta < 0 {
            (step + len - (delta.unsigned_abs() as usize % len)) % len
        } else {
            (step + delta as usize) % len
        };
        if options[step].enabled {
            return Some(step);
        }
    }
    None
}

/// The radio group widget: a legend, an optional hint, the option rows,
/// and an optional error message — one control that picks exactly one
/// value.
///
/// ```
/// use ui_kit::components::radio::{option, RadioGroup};
/// # #[derive(Clone, Copy, PartialEq, Eq)] enum Region { Nordics, Europe, Global }
/// const REGIONS: &[ui_kit::components::radio::RadioOption<Region>] = &[
///     option(Region::Nordics, "Nordics"),
///     option(Region::Europe, "Europe"),
///     option(Region::Global, "Global"),
/// ];
/// let group = RadioGroup::new("Region")
///     .options(REGIONS)
///     .selected(Some(Region::Nordics))
///     .hint("Where the team will work from.");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RadioGroup<'a, T: Copy + Eq> {
    label: &'a str,
    options: &'a [RadioOption<'a, T>],
    selected: Option<T>,
    hint: Option<&'a str>,
    error: Option<&'a str>,
    enabled: bool,
}

impl<'a, T: Copy + Eq> RadioGroup<'a, T> {
    /// A radio group asking `label`, with no options yet.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            options: &[],
            selected: None,
            hint: None,
            error: None,
            enabled: true,
        }
    }

    /// The group's options, in row order.
    pub const fn options(mut self, options: &'a [RadioOption<'a, T>]) -> Self {
        self.options = options;
        self
    }

    /// The currently selected value, if a choice is made.
    pub const fn selected(mut self, selected: Option<T>) -> Self {
        self.selected = selected;
        self
    }

    /// The group's hint, under the legend.
    pub const fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// The group's error: the borders turn `error` and the message
    /// replaces the hint's place below the options.
    pub const fn error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }

    /// Set enabled: a disabled group disables every row.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the whole group.
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Show the group and hand back its typed response: the selected
    /// row's egui response plus the value a choice landed on, if any.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> RadioResponse<T> {
        // The group's column runs on exact gaps, like the fields.
        let saved_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        label_row(ui, theme, self.label);
        if let Some(hint) = self.hint {
            ui.add_space(HINT_GAP);
            hint_row(ui, theme, hint);
        }
        ui.add_space(OPTIONS_GAP);

        let ctx = ui.ctx().clone();
        let invalid = self.error.is_some();
        let mut committed: Option<T> = None;
        let mut responses: Vec<Response> = Vec::new();
        let mut row_ids: Vec<egui::Id> = Vec::new();

        for (index, option) in self.options.iter().enumerate() {
            if index > 0 {
                ui.add_space(ROW_GAP);
            }
            let font_id = FontId {
                size: LABEL_SIZE,
                family: TypeStyle::Body.family(theme),
            };
            let galley = ui.ctx().fonts_mut(|fonts| {
                fonts.layout_delayed_color(option.label.to_owned(), font_id, f32::INFINITY)
            });
            let size = Vec2::new(row_min_width(galley.size().x), ROW_HEIGHT);
            let id = ui.next_auto_id().with("vynatix-radio").with(index);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let enabled = self.enabled && option.enabled;
            let response = ui.interact(
                rect,
                id,
                if enabled {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            let mut widget = WidgetState::of(&response);
            if !enabled {
                widget.disabled = true;
            }
            let selected = self.selected == Some(option.value);
            let look = radio_look(selected, invalid, &widget, theme);
            let painter = ui.painter().clone().with_layer_id(response.layer_id);

            let circle = circle_rect(rect);
            let radius = CornerRadius::same((CIRCLE_SIZE / 2.0) as u8);
            painter.rect_filled(circle, radius, look.fill);
            painter.rect_stroke(circle, radius, look.border, egui::StrokeKind::Inside);
            if let Some(colour) = look.dot {
                painter.circle_filled(circle.center(), DOT_SIZE / 2.0, colour);
            }
            let label_pos = crate::typography::left_center(
                rect.left() + CIRCLE_SIZE + LABEL_GAP,
                rect.center().y,
                galley.size(),
            );
            painter.galley(label_pos, galley, look.label);

            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::RadioButton,
                    enabled,
                    selected,
                    option.label,
                )
            });
            FocusRing::new()
                .radius(radius)
                .paint_at(ui, &response, circle, theme, flags);

            // A click commits a new choice (and takes the focus); a
            // click on the selected row is the native no-op.
            if response.clicked() {
                ctx.memory_mut(|memory| memory.request_focus(response.id));
                if !selected {
                    committed = Some(option.value);
                }
            }

            // The group traps the vertical arrows on whichever row
            // holds the focus (egui's focus lock), so the choice, not
            // the system focus, is what they move.
            ctx.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });

            row_ids.push(id);
            responses.push(response);
        }

        // The arrows move the choice through the enabled options,
        // wrapping, the focus riding along.
        let (down, up) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::ArrowUp),
            )
        });
        if (down || up) && self.enabled {
            if let Some(focused) = responses.iter().position(|r| r.has_focus()) {
                let delta = if down { 1 } else { -1 };
                if let Some(target) = step_selection(self.options, focused, delta) {
                    if Some(self.options[target].value) != self.selected {
                        committed = Some(self.options[target].value);
                    }
                    ctx.memory_mut(|memory| memory.request_focus(row_ids[target]));
                }
            }
        }

        // The error message, below the options.
        if let Some(error) = self.error {
            ui.add_space(OPTIONS_GAP);
            error_row(ui, theme, error);
        }

        ui.spacing_mut().item_spacing = saved_spacing;

        // The selected row's response (the first row's when no choice
        // is made): the row a host inspects for focus and hover. A
        // group with no options has no rows; the ui's own response
        // stands in.
        let selected_index = self
            .selected
            .and_then(|value| self.options.iter().position(|o| o.value == value))
            .unwrap_or(0);
        let response = responses
            .into_iter()
            .nth(selected_index)
            .unwrap_or_else(|| ui.response());

        RadioResponse {
            response,
            committed,
        }
    }
}

/// The radio group's response: the selected row's egui response plus
/// the typed value a choice landed on, if any.
#[derive(Clone, Debug)]
pub struct RadioResponse<T> {
    /// The wrapped egui response.
    pub response: Response,
    /// The option committed this frame, if a choice landed.
    pub committed: Option<T>,
}

#[cfg(test)]
mod tests {
    use crate::states::{self, UiKitFlags, WidgetState};
    use crate::theme::ThemeId;
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        circle_rect, option, radio_look, row_min_width, step_selection, RadioGroup, CIRCLE_BORDER,
        CIRCLE_SIZE, DOT_SIZE, LABEL_GAP, ROW_HEIGHT,
    };

    /// Install the faces the group paints with: the icon families and
    /// the theme's type faces bind at the pass boundary, so a warm-up
    /// pass makes them resolvable for the widget passes that follow.
    fn install_faces(ctx: &egui::Context, theme: ThemeId) {
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
    }

    /// The region options, the Radio preview's own example domain.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Region {
        Nordics,
        Europe,
        Global,
    }

    const REGIONS: &[super::RadioOption<Region>] = &[
        option(Region::Nordics, "Nordics"),
        option(Region::Europe, "Europe"),
        option(Region::Global, "Global"),
    ];

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

    /// The look table per the Radio preview, in every theme: rest is
    /// `surface-container-low` behind a 2px `outline` border with no
    /// dot; selected borders `primary` and draws the 8px dot in
    /// `primary`; an error turns the border `error` over both.
    #[test]
    fn looks_match_the_preview_table_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();
            let rest = state("rest");

            let unselected = radio_look(false, false, &rest, *theme);
            assert_eq!(
                unselected.fill, colors.surface_container_low,
                "{name}: rest fill"
            );
            assert_eq!(unselected.border.width, CIRCLE_BORDER, "{name}: 2px");
            assert_eq!(unselected.border.color, colors.outline, "{name}: outline");
            assert_eq!(unselected.dot, None, "{name}: no dot at rest");

            let selected = radio_look(true, false, &rest, *theme);
            assert_eq!(selected.fill, colors.surface_container_low, "{name}: fill");
            assert_eq!(
                selected.border.color, colors.primary,
                "{name}: selected border"
            );
            assert_eq!(selected.dot, Some(colors.primary), "{name}: the dot");

            let invalid = radio_look(false, true, &rest, *theme);
            assert_eq!(invalid.border.color, colors.error, "{name}: error border");
            let invalid_selected = radio_look(true, true, &rest, *theme);
            assert_eq!(
                invalid_selected.border.color, colors.error,
                "{name}: the error wins over the selected border"
            );
        }
    }

    /// Hover and press are the shared state layers over the fill's own
    /// on-colour; disabled is the 38/12 pair over the circle with the
    /// label in `text-muted` and the dot at 38%.
    #[test]
    fn states_follow_the_shared_helpers() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let hover = radio_look(false, false, &state("hover"), *theme);
            assert_eq!(
                hover.fill,
                states::composite_over(
                    states::hover_overlay(colors.on_surface),
                    colors.surface_container_low
                ),
                "{name}: hover composites"
            );

            let disabled = radio_look(true, false, &state("disabled"), *theme);
            assert_eq!(
                disabled.fill,
                states::disabled(colors.on_surface, colors.surface_container_low).container,
                "{name}: the circle fill at 12%"
            );
            assert_eq!(
                disabled.border.color,
                states::disabled_content(colors.primary),
                "{name}: the border at 38%"
            );
            assert_eq!(
                disabled.dot,
                Some(states::disabled_content(colors.primary)),
                "{name}: the dot at 38%"
            );
            assert_eq!(disabled.label, colors.text_muted(), "{name}: muted label");
        }
    }

    /// The arrows step through the enabled options, wrapping, and skip
    /// the disabled ones from either direction.
    #[test]
    fn arrows_step_wrap_and_skip() {
        let options: &[super::RadioOption<Region>] = &[
            option(Region::Nordics, "Nordics"),
            option(Region::Europe, "Europe").disabled(),
            option(Region::Global, "Global"),
        ];
        assert_eq!(step_selection(options, 0, 1), Some(2), "skips the disabled");
        assert_eq!(step_selection(options, 2, 1), Some(0), "wraps");
        assert_eq!(step_selection(options, 0, -1), Some(2), "wraps back");
        assert_eq!(step_selection(options, 2, -1), Some(0), "skips back");
        assert_eq!(step_selection(REGIONS, 1, 1), Some(2));
        assert_eq!(step_selection(REGIONS, 1, -1), Some(0));

        let none: &[super::RadioOption<Region>] = &[option(Region::Nordics, "Nordics").disabled()];
        assert_eq!(step_selection(none, 0, 1), None, "all disabled");
        let empty: &[super::RadioOption<Region>] = &[];
        assert_eq!(step_selection(empty, 0, 1), None, "no options");
    }

    /// The geometry: an 18px circle flush left in a 20px row, the 8px
    /// dot centred, the 10px gap and the label beside.
    #[test]
    fn geometry_matches_the_spec() {
        assert_eq!(CIRCLE_SIZE, 18.0);
        assert_eq!(CIRCLE_BORDER, 2.0);
        assert_eq!(DOT_SIZE, 8.0);
        assert_eq!(ROW_HEIGHT, 20.0);
        assert_eq!(LABEL_GAP, 10.0);
        let row = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(row_min_width(100.0), ROW_HEIGHT),
        );
        let circle = circle_rect(row);
        // Centred on the row's 20px line box: 1px over and under.
        assert_eq!(
            circle.min,
            egui::Pos2::new(0.0, row.center().y - CIRCLE_SIZE / 2.0)
        );
        assert_eq!(circle.size(), egui::Vec2::splat(CIRCLE_SIZE));
        assert_eq!(row_min_width(100.0), CIRCLE_SIZE + LABEL_GAP + 100.0);
    }

    /// One selected, exactly one, by construction: a click commits the
    /// clicked row, the host's single value moves, and the group then
    /// reports exactly one selected row through its looks; a click on
    /// the selected row commits nothing, and a disabled group commits
    /// nothing at all.
    #[test]
    fn selection_is_exclusive_across_the_group() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();

        // egui hit-tests a press against the previous pass's widgets, so
        // the measuring pass renders the real group; the third row
        // centre follows from the first row's rect by the row height
        // and gap (20 + 12px each).
        let mut first_row = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = RadioGroup::new("Region")
                .options(REGIONS)
                .selected(Some(Region::Nordics))
                .ui(ui, ThemeId::Light, &flags);
            first_row = Some(response.response.rect);
        })
        .drop_without_applying_deltas();
        let first = first_row.expect("the group laid out");
        let row_centers: Vec<egui::Pos2> = (0..3)
            .map(|index| {
                egui::Pos2::new(
                    first.left() + super::CIRCLE_SIZE / 2.0,
                    first.center().y + index as f32 * (ROW_HEIGHT + super::ROW_GAP),
                )
            })
            .collect();

        let mut region = Some(Region::Nordics);
        let picked = click(&ctx, row_centers[2], |ui| {
            RadioGroup::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, ThemeId::Light, &flags)
                .committed
        });
        assert_eq!(picked, Some(Region::Global), "the click commits the row");
        region = picked;

        // The host's single value is the whole selection: exactly one
        // row's look carries the dot.
        let selected_rows = REGIONS
            .iter()
            .filter(|option| {
                radio_look(
                    Some(option.value) == region,
                    false,
                    &state("rest"),
                    ThemeId::Light,
                )
                .dot
                .is_some()
            })
            .count();
        assert_eq!(selected_rows, 1, "exactly one row selected");

        // A click on the selected row commits nothing.
        let again = click(&ctx, row_centers[2], |ui| {
            RadioGroup::new("Region")
                .options(REGIONS)
                .selected(region)
                .ui(ui, ThemeId::Light, &flags)
                .committed
        });
        assert_eq!(again, None);

        // A disabled group commits nothing.
        let disabled = click(&ctx, row_centers[1], |ui| {
            RadioGroup::new("Region")
                .options(REGIONS)
                .selected(region)
                .disabled()
                .ui(ui, ThemeId::Light, &flags)
                .committed
        });
        assert_eq!(disabled, None);
    }

    /// The arrows move the choice from the focused row, wrapping, the
    /// focus riding along; Space activates the focused row through
    /// egui's focused-widget click.
    #[test]
    fn keyboard_moves_and_activates_the_choice() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();

        let render = |ctx: &egui::Context, region: Option<Region>, input: egui::RawInput| {
            let mut committed = None;
            let mut focused = None;
            let out = ctx.run_ui(input, |ui| {
                let response = RadioGroup::new("Region")
                    .options(REGIONS)
                    .selected(region)
                    .ui(ui, ThemeId::Light, &flags);
                committed = response.committed;
                focused = Some(response.response.has_focus());
            });
            out.drop_without_applying_deltas();
            (committed, focused.unwrap())
        };

        // Focus the selected row (the group's returned response is the
        // selected row's): one frame to gain it, one to settle the
        // focus lock, then the arrows are the group's own.
        let mut first_row = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = RadioGroup::new("Region")
                .options(REGIONS)
                .selected(Some(Region::Nordics))
                .ui(ui, ThemeId::Light, &flags);
            first_row = Some(response.response.id);
        })
        .drop_without_applying_deltas();
        ctx.memory_mut(|memory| memory.request_focus(first_row.unwrap()));
        let (_, focused) = render(&ctx, Some(Region::Nordics), egui::RawInput::default());
        assert!(focused, "the selected row holds the focus");

        let mut region = Some(Region::Nordics);
        for (key, expected) in [
            (egui::Key::ArrowDown, Region::Europe),
            (egui::Key::ArrowDown, Region::Global),
            (egui::Key::ArrowDown, Region::Nordics), // wraps
            (egui::Key::ArrowUp, Region::Global),    // and back
        ] {
            let (committed, _) = render(&ctx, region, key_input(key));
            assert_eq!(committed, Some(expected), "the choice moves on {key:?}");
            region = Some(expected);
            // The focus rides along: the settled frame's selected row
            // (the group's returned response) holds it.
            let (_, focused) = render(&ctx, region, egui::RawInput::default());
            assert!(focused, "the focus rides along onto {expected:?}");
        }

        // Space activates the focused row: with no choice made yet,
        // the focused first row commits.
        let mut first_row = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = RadioGroup::new("Region")
                .options(REGIONS)
                .selected(None)
                .ui(ui, ThemeId::Light, &flags);
            first_row = Some(response.response.id);
        })
        .drop_without_applying_deltas();
        ctx.memory_mut(|memory| memory.request_focus(first_row.unwrap()));
        render(&ctx, None, egui::RawInput::default());
        let (committed, _) = render(&ctx, None, key_input(egui::Key::Space));
        assert_eq!(committed, Some(Region::Nordics));
    }

    /// Each option's label starts after the circle and its 10px gap, on
    /// the label's own row. The paint once also subtracted half the
    /// label's width from that left anchor, so the label ran back over
    /// its circle. The group answers with the selected row's rect, so
    /// each option is selected in turn to read its row.
    #[test]
    fn labels_start_after_the_circle_and_its_gap() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();

        for option in REGIONS {
            let (row, shapes) = paint_pass(&ctx, |ui| {
                RadioGroup::new("Region")
                    .options(REGIONS)
                    .selected(Some(option.value))
                    .ui(ui, theme, &flags)
                    .response
                    .rect
            });
            let pos = assert_painted_inside(&shapes, option.label, row);
            let after_circle = row.left() + CIRCLE_SIZE + LABEL_GAP;
            assert!(
                (pos.x - after_circle).abs() < 0.01,
                "{}: starts at x {} but after the circle and its gap is {after_circle}",
                option.label,
                pos.x
            );
        }
    }

    /// One raw input carrying a single key press.
    fn key_input(key: egui::Key) -> egui::RawInput {
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        });
        input
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
}

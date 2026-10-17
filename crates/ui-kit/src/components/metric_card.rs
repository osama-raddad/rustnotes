//! MetricCard: one specific, sourced number, per
//! `components/MetricCard` and the preview's `.metric` rules.
//!
//! A `text-accent` label tracked at +18% above a large display-voice
//! figure with an optional smaller unit span, and a line of context
//! below saying what the number counts and when. The container is the
//! plain elevated [`Card`]: `surface-container-low`, `radius-lg`, 24px
//! padding.
//!
//! The label reuses the eyebrow machinery ([`TrackedLabel`]) at the
//! metric card's tighter +0.18em (the eyebrow's own +0.24em stays the
//! signature); the figure is the display voice in bold at 56px, tracked
//! −4%, line height 1, resolving through
//! [`display_bold_family`](crate::typography::display_bold_family) per
//! theme (Fraunces Metric in Forest, Instrument Serif in Teal, Geist
//! SemiBold in Monochrome, the documented deviations). The unit span
//! rides inline in the figure's job at the type scale's h4 (32px),
//! baseline-aligned by the shaper. Real, falsifiable numbers only: the
//! context line carries the source and the date.

use egui::text::LayoutJob;
use egui::{Color32, FontId, Response, Ui};

use crate::components::{Card, CardPadding};
use crate::theme::ThemeId;
use crate::tokens::{leading, spacing, type_scale};
use crate::typography::{tracked_text, TrackedLabel, TypeStyle};

/// The narrowest a metric card gets, padding included: `space-40`
/// (160px) of content between the two `space-6` paddings, which holds
/// the widest gallery figure on one line ("19 mo" in the Monochrome
/// face, 148px) and every label and context word. A row that would
/// squeeze its cards (a horizontal scroll caps its content at the
/// visible width) then scrolls instead of breaking "100%" apart.
pub const MIN_WIDTH: f32 = spacing::SPACE_40 + 2.0 * CardPadding::Standard.spacing();

/// The label's tracking: +18%, tighter than the eyebrow's +24%.
pub const LABEL_TRACKING_EM: f32 = 0.18;
/// The figure's tracking: −4%, the display track at metric size.
pub const FIGURE_TRACKING_EM: f32 = -0.04;

/// The metric card's label font: the body face at weight 600, 11px,
/// uppercased by the tracked label. Weight 600 is headline 4's family,
/// the body voice's semibold.
pub fn label_font_id(theme: ThemeId) -> FontId {
    FontId {
        size: type_scale::TEXT_EYEBROW,
        family: TypeStyle::Headline4.family(theme),
    }
}

/// The figure font: the display voice in bold at the type scale's h2
/// (56px, `--text-h2` in the preview).
pub fn figure_font_id(theme: ThemeId) -> FontId {
    FontId {
        size: type_scale::TEXT_H2,
        family: crate::typography::display_bold_family(theme),
    }
}

/// The unit span's font: the figure's family at the type scale's h4
/// (32px, `--text-h4`), smaller than the figure it sits in.
pub fn unit_font_id(theme: ThemeId) -> FontId {
    FontId {
        size: type_scale::TEXT_H4,
        family: crate::typography::display_bold_family(theme),
    }
}

/// The context font: the body face at the type scale's label size
/// (13px, `--text-label`).
pub fn context_font_id(theme: ThemeId) -> FontId {
    FontId {
        size: type_scale::TEXT_LABEL,
        family: TypeStyle::Body.family(theme),
    }
}

/// The figure's line box: line height 1 at the figure's size, per the
/// preview's `.val` rule.
pub const fn figure_line_height() -> f32 {
    type_scale::TEXT_H2 * 1.0
}

/// The figure over the context line: the metric card's own display
/// pairing, which must clear the system's 2.5x floor (56 over 13 sits
/// at 4.3).
pub fn figure_to_context_ratio() -> f32 {
    type_scale::TEXT_H2 / type_scale::TEXT_LABEL
}

/// The figure's laid-out text with its optional unit span inline:
/// `100` plus `%` is one job, the unit smaller but baseline-aligned by
/// the shaper, both tracked −4% in `text-display`.
pub fn value_job(value: &str, unit: Option<&str>, theme: ThemeId, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let line = figure_line_height();
    job.append(
        value,
        0.0,
        egui::text::TextFormat {
            font_id: figure_font_id(theme),
            extra_letter_spacing: FIGURE_TRACKING_EM * type_scale::TEXT_H2,
            line_height: Some(line),
            color,
            ..Default::default()
        },
    );
    if let Some(unit) = unit {
        job.append(
            unit,
            0.0,
            egui::text::TextFormat {
                font_id: unit_font_id(theme),
                // The span inherits the figure's −4% at its own size,
                // and its line box, so the shaper baseline-aligns it.
                extra_letter_spacing: FIGURE_TRACKING_EM * type_scale::TEXT_H4,
                line_height: Some(line),
                color,
                ..Default::default()
            },
        );
    }
    job
}

/// The context line: 13px body in `text-muted` at the normal leading,
/// wrapping in the card.
pub fn context_job(context: &str, theme: ThemeId, color: Color32) -> LayoutJob {
    tracked_text(
        context,
        context_font_id(theme),
        0.0,
        Some(type_scale::TEXT_LABEL * leading::LEADING_NORMAL),
        color,
        None,
    )
}

/// The metric card widget: label, figure (with its optional unit) and
/// context, on the plain elevated card. A metric card states a number;
/// it is not a control, so it takes no flags and paints no focus ring.
///
/// ```
/// use ui_kit::components::MetricCard;
///
/// let trucks = MetricCard::new(
///     "Trucks running our code",
///     "714k",
///     "Volvo FH, FM, FMX. April 2026 fleet count.",
/// );
/// let senior = MetricCard::new(
///     "Senior people, percent",
///     "100",
///     "No juniors on client work. Ever.",
/// )
/// .unit("%");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MetricCard<'a> {
    label: &'a str,
    value: &'a str,
    unit: Option<&'a str>,
    context: &'a str,
}

impl<'a> MetricCard<'a> {
    /// The card: `label` above the `value` figure, `context` below.
    pub const fn new(label: &'a str, value: &'a str, context: &'a str) -> Self {
        Self {
            label,
            value,
            unit: None,
            context,
        }
    }

    /// The unit span inside the figure, like the percent sign in the
    /// preview: smaller, baseline-aligned.
    pub const fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Show the card, sized by its own copy and no narrower than
    /// [`MIN_WIDTH`].
    pub fn ui(self, ui: &mut Ui, theme: ThemeId) -> Response {
        let colors = theme.colors();
        let card = Card::new();
        card.ui(ui, theme, &Default::default(), |ui| {
            // The floor: a horizontal row hands the card whatever width
            // is left of its slot, which the label, figure and context
            // would wrap at. Raising the content's minimum also raises
            // the width they wrap at.
            ui.set_min_width(MIN_WIDTH - 2.0 * card.padding_step().spacing());
            ui.add(TrackedLabel::new(
                self.label,
                label_font_id(theme),
                LABEL_TRACKING_EM,
                colors.text_accent,
            ));
            // The preview's `.val` margins: 12px above the figure, 8px
            // below it.
            ui.add_space(spacing::SPACE_3);
            ui.add(egui::Label::new(value_job(
                self.value,
                self.unit,
                theme,
                colors.text_display(),
            )));
            ui.add_space(spacing::SPACE_2);
            ui.add(egui::Label::new(context_job(
                self.context,
                theme,
                colors.text_muted(),
            )));
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        context_font_id, context_job, figure_font_id, figure_line_height, figure_to_context_ratio,
        label_font_id, unit_font_id, value_job, MetricCard, FIGURE_TRACKING_EM, LABEL_TRACKING_EM,
        MIN_WIDTH,
    };
    use crate::components::CardPadding;
    use crate::gallery_copy::{
        METRIC_FLEET_FIGURE, METRIC_FLEET_LABEL, METRIC_FLEET_SOURCE, METRIC_SENIOR_FIGURE,
        METRIC_SENIOR_LABEL, METRIC_SENIOR_SOURCE, METRIC_SENIOR_UNIT, METRIC_TENURE_FIGURE,
        METRIC_TENURE_LABEL, METRIC_TENURE_SOURCE,
    };
    use crate::theme::ThemeId;
    use crate::tokens::type_scale;
    use crate::typography::{
        display_bold_family_name, paint_pass, painted_text, tracked_text, TypeStyle,
    };
    use egui::{Color32, FontFamily, Rect, UiBuilder, Vec2};

    /// The label is the eyebrow machinery at the metric card's own
    /// track: 11px (`--text-eyebrow`), the body face at weight 600 in
    /// every theme, tracked +18% (not the eyebrow's +24%).
    #[test]
    fn label_is_the_tracked_body_semi_bold_at_eighteen_percent() {
        assert_eq!(LABEL_TRACKING_EM, 0.18);
        assert_ne!(LABEL_TRACKING_EM, TypeStyle::Eyebrow.tracking());
        for theme in ThemeId::ALL {
            let font_id = label_font_id(*theme);
            assert_eq!(font_id.size, type_scale::TEXT_EYEBROW, "11px");
            assert_eq!(font_id.size, 11.0);
            // Weight 600 of the body voice is headline 4's family.
            assert_eq!(
                font_id.family,
                TypeStyle::Headline4.family(*theme),
                "{}: the body face at 600",
                theme.name()
            );
        }
    }

    /// The figure is the display voice in bold at 56px (`--text-h2`)
    /// with line height 1, tracked −4%.
    #[test]
    fn figure_is_the_bold_display_voice_at_56px() {
        assert_eq!(FIGURE_TRACKING_EM, -0.04);
        for theme in ThemeId::ALL {
            let font_id = figure_font_id(*theme);
            assert_eq!(font_id.size, type_scale::TEXT_H2, "56px");
            assert_eq!(font_id.size, 56.0);
            assert_eq!(
                font_id.family,
                FontFamily::Name(display_bold_family_name(*theme).into()),
                "{}: the theme's bold display family",
                theme.name()
            );
        }
        assert_eq!(figure_line_height(), 56.0, "line height 1");
    }

    /// The unit span is smaller than its figure: the type scale's h4
    /// (32px) against the figure's h2 (56px), in the same bold display
    /// family.
    #[test]
    fn unit_span_is_smaller_than_the_figure() {
        assert_eq!(unit_font_id(ThemeId::Light).size, type_scale::TEXT_H4);
        assert_eq!(unit_font_id(ThemeId::Light).size, 32.0);
        assert!(unit_font_id(ThemeId::Light).size < figure_font_id(ThemeId::Light).size);
        for theme in ThemeId::ALL {
            assert_eq!(
                unit_font_id(*theme).family,
                figure_font_id(*theme).family,
                "{}: the unit rides in the figure's family",
                theme.name()
            );
        }
    }

    /// The figure's job carries its tracking in points, keeps the
    /// figure's case (no uppercasing: 714k stays 714k), and appends the
    /// unit as a second, smaller section on the same baseline when
    /// there is one.
    #[test]
    fn value_job_tracks_and_appends_the_unit_inline() {
        let plain = value_job("714k", None, ThemeId::Light, Color32::BLACK);
        assert_eq!(plain.text, "714k", "figures keep their case");
        assert_eq!(plain.sections.len(), 1);
        let format = &plain.sections[0].format;
        assert_eq!(format.font_id.size, 56.0);
        assert!((format.extra_letter_spacing - (-0.04 * 56.0)).abs() < 1e-4);
        assert_eq!(format.line_height, Some(56.0));
        assert_eq!(format.color, Color32::BLACK);

        let with_unit = value_job("100", Some("%"), ThemeId::TealLight, Color32::WHITE);
        assert_eq!(with_unit.text, "100%");
        assert_eq!(with_unit.sections.len(), 2, "the unit is an inline span");
        let unit_format = &with_unit.sections[1].format;
        assert_eq!(unit_format.font_id.size, 32.0);
        assert_eq!(unit_format.font_id, unit_font_id(ThemeId::TealLight));
        assert!((unit_format.extra_letter_spacing - (-0.04 * 32.0)).abs() < 1e-4);
        // The span shares the figure's line box, so it sits on the same
        // baseline rather than the row's centre.
        assert_eq!(unit_format.line_height, Some(56.0));
    }

    /// The context line is 13px (`--text-label`) in the body face at
    /// the normal leading, in the muted text colour.
    #[test]
    fn context_is_thirteen_px_body_at_the_normal_leading() {
        for theme in ThemeId::ALL {
            let font_id = context_font_id(*theme);
            assert_eq!(font_id.size, type_scale::TEXT_LABEL);
            assert_eq!(font_id.size, 13.0);
            assert_eq!(
                font_id.family,
                TypeStyle::Body.family(*theme),
                "{}: the body face",
                theme.name()
            );
        }
        let job = context_job(
            "Across 27 engagements since 2022.",
            ThemeId::Dark,
            Color32::GRAY,
        );
        assert_eq!(job.sections[0].format.line_height, Some(13.0 * 1.5));
        assert_eq!(job.sections[0].format.extra_letter_spacing, 0.0);
    }

    /// The figure's pairing with its context line clears the system's
    /// 2.5x floor with room: 56 over 13.
    #[test]
    fn figure_pairs_with_context_at_two_and_a_half_times() {
        assert!((figure_to_context_ratio() - 56.0 / 13.0).abs() < 1e-4);
        assert!(figure_to_context_ratio() >= 2.5);
    }

    /// The card paints headless in every theme, with and without a unit
    /// span, without touching the output deltas. The theme's faces and
    /// the icon families are installed first, as a host would.
    #[test]
    fn metric_card_paints_headless() {
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            crate::typography::install(*theme, &ctx);
            crate::icon::install(&ctx);
            ctx.run_ui(egui::RawInput::default(), |ui| {
                MetricCard::new(
                    "Trucks running our code",
                    "714k",
                    "Volvo FH, FM, FMX. April 2026 fleet count.",
                )
                .ui(ui, *theme);
                MetricCard::new(
                    "Senior people, percent",
                    "100",
                    "No juniors on client work. Ever.",
                )
                .unit("%")
                .ui(ui, *theme);
            })
            .drop_without_applying_deltas();
        }
    }

    /// One metric card's copy: (label, figure, unit, context).
    type CardCopy = (
        &'static str,
        &'static str,
        Option<&'static str>,
        &'static str,
    );

    /// The gallery's three metric cards.
    fn gallery_cards() -> [CardCopy; 3] {
        [
            (
                METRIC_FLEET_LABEL,
                METRIC_FLEET_FIGURE,
                None,
                METRIC_FLEET_SOURCE,
            ),
            (
                METRIC_TENURE_LABEL,
                METRIC_TENURE_FIGURE,
                None,
                METRIC_TENURE_SOURCE,
            ),
            (
                METRIC_SENIOR_LABEL,
                METRIC_SENIOR_FIGURE,
                Some(METRIC_SENIOR_UNIT),
                METRIC_SENIOR_SOURCE,
            ),
        ]
    }

    /// A context carrying `theme`'s faces and the icon families: they
    /// bind at the pass boundary, so a warm-up pass makes them resolvable
    /// for the passes that follow.
    fn context_with_faces(theme: ThemeId) -> egui::Context {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
        ctx
    }

    /// Lay `card` out the way the gallery's metric row does: in a
    /// horizontal row inside a horizontal scroll area, in a slot only
    /// `slot_width` wide. The test context's screen is huge, so the slot
    /// is what narrows the row. Returns the card's rect and the shapes
    /// the pass painted.
    fn card_in_a_scrolling_slot(
        ctx: &egui::Context,
        theme: ThemeId,
        slot_width: f32,
        card: MetricCard<'_>,
    ) -> (Rect, Vec<egui::epaint::ClippedShape>) {
        paint_pass(ctx, |ui| {
            let slot = Rect::from_min_size(ui.cursor().min, Vec2::new(slot_width, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(slot), |ui| {
                egui::ScrollArea::horizontal()
                    .show(ui, |ui| ui.horizontal(|ui| card.ui(ui, theme).rect).inner)
                    .inner
            })
            .inner
        })
    }

    /// A horizontal scroll caps its content at the visible width, so a
    /// row hands each card what is left of that width, and a card that
    /// sets no floor shrinks to it: 78pt and 40pt on a phone, the label
    /// one letter to a line. The card now holds [`MIN_WIDTH`] however
    /// narrow its slot, in every theme, and the row scrolls to reach it.
    #[test]
    fn a_card_keeps_its_minimum_width_in_a_narrow_scrolling_slot() {
        for theme in ThemeId::ALL {
            let ctx = context_with_faces(*theme);
            for (label, value, unit, context) in gallery_cards() {
                let mut card = MetricCard::new(label, value, context);
                if let Some(unit) = unit {
                    card = card.unit(unit);
                }
                let (rect, _) = card_in_a_scrolling_slot(&ctx, *theme, 40.0, card);
                assert!(
                    rect.width() >= MIN_WIDTH - 0.5,
                    "{}, {label:?}: the card is {}pt wide in a 40pt slot; the minimum is \
                     {MIN_WIDTH}pt",
                    theme.name(),
                    rect.width()
                );
            }
        }
    }

    /// The minimum is a floor and not a size: in a slot as wide as a
    /// desktop window the card is still as wide as its own copy asks.
    #[test]
    fn a_card_in_a_wide_slot_is_sized_by_its_copy() {
        let theme = ThemeId::Light;
        let ctx = context_with_faces(theme);
        for (label, value, _, context) in gallery_cards() {
            let (rect, _) = card_in_a_scrolling_slot(
                &ctx,
                theme,
                1000.0,
                MetricCard::new(label, value, context),
            );
            // Each gallery card's copy asks for more than the floor, and
            // for far less than the slot.
            assert!(
                rect.width() > MIN_WIDTH && rect.width() < 400.0,
                "{label:?}: the card is {}pt wide in a 1000pt slot",
                rect.width()
            );
        }
    }

    /// The words painted on each row of the galley `text` was laid out
    /// as, in order: what the card showed, whatever the rows broke.
    fn painted_words(shapes: &[egui::epaint::ClippedShape], text: &str) -> Vec<String> {
        painted_text(shapes, text)
            .galley
            .rows
            .iter()
            .flat_map(|placed| {
                placed
                    .row
                    .text()
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The label, the figure and the context keep their words whole in a
    /// narrow slot: the rows together read as the copy's own words, with
    /// none broken in two ("ENGAGEM" over "ENT", "100" over "%").
    #[test]
    fn narrow_cards_keep_their_words_whole() {
        for theme in ThemeId::ALL {
            let ctx = context_with_faces(*theme);
            for (label, value, unit, context) in gallery_cards() {
                let mut card = MetricCard::new(label, value, context);
                let mut figure = value.to_owned();
                if let Some(unit) = unit {
                    card = card.unit(unit);
                    figure.push_str(unit);
                }
                let (_, shapes) = card_in_a_scrolling_slot(&ctx, *theme, 40.0, card);
                for text in [label.to_uppercase(), figure, context.to_owned()] {
                    let words: Vec<&str> = text.split_whitespace().collect();
                    assert_eq!(
                        painted_words(&shapes, &text),
                        words,
                        "{}: {text:?} was broken inside a word",
                        theme.name()
                    );
                }
            }
        }
    }

    /// [`MIN_WIDTH`]'s content width holds what it is chosen for, in
    /// every theme's faces: each gallery figure on one line, and every
    /// word of every label and context line, so none is broken inside.
    #[test]
    fn the_minimum_holds_the_widest_figure_and_every_word() {
        let content = MIN_WIDTH - 2.0 * CardPadding::Standard.spacing();
        for theme in ThemeId::ALL {
            let ctx = context_with_faces(*theme);
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let width = |job: egui::text::LayoutJob| {
                    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job).size().x)
                };
                for (label, value, unit, context) in gallery_cards() {
                    let figure = width(value_job(value, unit, *theme, Color32::BLACK));
                    assert!(
                        figure <= content,
                        "{}: the figure {value:?} is {figure}pt, over the {content}pt content",
                        theme.name()
                    );
                    for word in label.to_uppercase().split_whitespace() {
                        let job = tracked_text(
                            word,
                            label_font_id(*theme),
                            LABEL_TRACKING_EM,
                            None,
                            Color32::BLACK,
                            None,
                        );
                        assert!(
                            width(job) <= content,
                            "{}: the label word {word:?} is over the {content}pt content",
                            theme.name()
                        );
                    }
                    for word in context.split_whitespace() {
                        assert!(
                            width(context_job(word, *theme, Color32::BLACK)) <= content,
                            "{}: the context word {word:?} is over the {content}pt content",
                            theme.name()
                        );
                    }
                }
            });
            out.drop_without_applying_deltas();
        }
    }
}

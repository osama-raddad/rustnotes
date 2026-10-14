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

use crate::components::Card;
use crate::theme::ThemeId;
use crate::tokens::{leading, spacing, type_scale};
use crate::typography::{tracked_text, TrackedLabel, TypeStyle};

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

    /// Show the card, sized by its own copy.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId) -> Response {
        let colors = theme.colors();
        Card::new().ui(ui, theme, &Default::default(), |ui| {
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
    };
    use crate::theme::ThemeId;
    use crate::tokens::type_scale;
    use crate::typography::{display_bold_family_name, TypeStyle};
    use egui::{Color32, FontFamily};

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
}

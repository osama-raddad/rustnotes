//! ThemesShowcase: the six themes side by side as miniature re-skinned
//! specimens, `components/Themes`' preview ported as a gallery section
//! (issue 10).
//!
//! The whole point of the page is the one-system-six-skins claim, so
//! every card paints ITS theme's tokens whatever the gallery's ambient
//! theme is: the kit's components take a [`ThemeId`] and resolve their
//! own tokens from it, so the card passes its theme down and nothing
//! reads the ambient visuals. The tokens are plain data; the section
//! copy (eyebrow, intro, captions) stays in the ambient theme.
//!
//! Per card, per the preview: the V mark and the `tokens.json` id pill;
//! the theme's display name and its character sentence; the six anchor
//! swatches (primary, on-primary, surface, surface-container-low,
//! tertiary, outline) with mono labels; the direction's ratio bar and
//! legend (60/30/10 in Forest, 60/30/9/1 in Teal and Monochrome); the
//! display specimen in the theme's own display face with its faces
//! line; and a live specimen row, a primary and a secondary Button, a
//! selected Chip and a small Card, all re-skinned.
//!
//! The copy lives in consts and the [`CARDS`] table so the voice lint
//! (issue 11) can scan it.

use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::components::brand_showcase::card_rows;
use crate::components::button::{Button, ButtonEmphasis};
use crate::components::card::{Card, CardPadding};
use crate::components::chip::Chip;
use crate::logo::Logo;
use crate::states::UiKitFlags;
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale, ThemeColors};
use crate::typography::{
    body_semibold_family, specimen_job, tracked_text, TrackedLabel, TypeStyle,
};

/// The theme card's content width where cards share a line (the card's
/// own padding rides on top; three cards and their gutters fit the
/// gallery's page). Under the compact breakpoint the cards stack and each
/// takes the whole column instead (`brand_showcase::card_width`).
pub const CARD_WIDTH: f32 = 264.0;

/// The roles the swatch grid and the ratio bar paint, resolved from
/// the card's own tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowcaseRole {
    /// `primary`.
    Primary,
    /// `on-primary`.
    OnPrimary,
    /// `surface`, the canvas.
    Surface,
    /// `surface-container-low`, the card's own rest.
    SurfaceContainerLow,
    /// `secondary`.
    Secondary,
    /// `on-surface`, the ink.
    OnSurface,
    /// `tertiary`, the accent.
    Tertiary,
    /// `outline`.
    Outline,
}

impl ShowcaseRole {
    /// The swatch grid's six roles, the preview's order.
    pub const SWATCHES: &[ShowcaseRole] = &[
        ShowcaseRole::Primary,
        ShowcaseRole::OnPrimary,
        ShowcaseRole::Surface,
        ShowcaseRole::SurfaceContainerLow,
        ShowcaseRole::Tertiary,
        ShowcaseRole::Outline,
    ];

    /// The role's value in a theme.
    pub const fn resolve(self, colors: &ThemeColors) -> Color32 {
        match self {
            ShowcaseRole::Primary => colors.primary,
            ShowcaseRole::OnPrimary => colors.on_primary,
            ShowcaseRole::Surface => colors.surface,
            ShowcaseRole::SurfaceContainerLow => colors.surface_container_low,
            ShowcaseRole::Secondary => colors.secondary,
            ShowcaseRole::OnSurface => colors.on_surface,
            ShowcaseRole::Tertiary => colors.tertiary,
            ShowcaseRole::Outline => colors.outline,
        }
    }

    /// The role's css name, the preview's mono swatch labels.
    pub const fn name(self) -> &'static str {
        match self {
            ShowcaseRole::Primary => "primary",
            ShowcaseRole::OnPrimary => "on-primary",
            ShowcaseRole::Surface => "surface",
            ShowcaseRole::SurfaceContainerLow => "surface-container-low",
            ShowcaseRole::Secondary => "secondary",
            ShowcaseRole::OnSurface => "on-surface",
            ShowcaseRole::Tertiary => "tertiary",
            ShowcaseRole::Outline => "outline",
        }
    }
}

/// One theme card's copy: the character sentence, the ratio bar's
/// segments (share out of one hundred and the role each paints) and
/// the legend naming the direction's anchors.
///
/// The legend entries are paint names and shares ("60 Cloud Dancer",
/// "30 Cloud Dancer + Teal"), not prose: every word a proper noun or
/// a digit, so the gallery voice lint's sentence-case heuristic steps
/// aside for them (the documented exemption at
/// `gallery_copy::voice_failures`); the punctuation checks still run.
pub struct ThemeCardCopy {
    /// The theme this card paints: its tokens, its faces, its name.
    pub theme: ThemeId,
    /// The one-line character sentence under the name.
    pub character: &'static str,
    /// The ratio bar's segments: share and role.
    pub ratio: &'static [(u32, ShowcaseRole)],
    /// The legend under the ratio bar, one entry per anchor group.
    pub legend: &'static [&'static str],
    /// The faces line under the display specimen.
    pub faces: &'static str,
}

/// The six cards, the preview's arrangement: the three light themes,
/// then the three dark ones.
pub const CARDS: &[ThemeCardCopy] = &[
    ThemeCardCopy {
        theme: ThemeId::Light,
        character: "Deep Hunter Green on Warm Cream: Nordic forest and editorial print.",
        ratio: &[
            (60, ShowcaseRole::Surface),
            (30, ShowcaseRole::Primary),
            (5, ShowcaseRole::Tertiary),
            (5, ShowcaseRole::OnSurface),
        ],
        legend: &["60 Cream", "30 Forest", "10 Bronze + Charcoal"],
        faces: "Fraunces display, Aptos body",
    },
    ThemeCardCopy {
        theme: ThemeId::TealLight,
        character: "Deep Teal on Cloud Dancer, with Champagne as the one guest accent.",
        ratio: &[
            (60, ShowcaseRole::Surface),
            (30, ShowcaseRole::OnSurface),
            (9, ShowcaseRole::Secondary),
            (1, ShowcaseRole::Tertiary),
        ],
        legend: &[
            "60 Cloud Dancer",
            "30 Ink + Teal",
            "9 Warm Grey",
            "1 Champagne",
        ],
        faces: "Instrument Serif display, Geist body",
    },
    ThemeCardCopy {
        theme: ThemeId::MonoLight,
        character: "Warm Charcoal on Paper. Stone connects; Bronze is a guest.",
        ratio: &[
            (60, ShowcaseRole::Surface),
            (30, ShowcaseRole::Primary),
            (9, ShowcaseRole::Secondary),
            (1, ShowcaseRole::Tertiary),
        ],
        legend: &["60 Paper", "30 Charcoal", "9 Stone", "1 Bronze"],
        faces: "Geist display and body, Fraunces editorial",
    },
    ThemeCardCopy {
        theme: ThemeId::Dark,
        character: "A warm near-black canvas; Forest and Bronze rise to lighter tones.",
        ratio: &[
            (60, ShowcaseRole::Surface),
            (30, ShowcaseRole::Primary),
            (5, ShowcaseRole::Tertiary),
            (5, ShowcaseRole::OnSurface),
        ],
        legend: &["60 Neutral 10", "30 Forest 80", "10 Bronze 70 + text"],
        faces: "Fraunces display, Aptos body",
    },
    ThemeCardCopy {
        theme: ThemeId::TealDark,
        character: "Ink Teal-Black canvas, Cloud Dancer type; teal and Champagne brighten.",
        ratio: &[
            (60, ShowcaseRole::Surface),
            (30, ShowcaseRole::OnSurface),
            (9, ShowcaseRole::Secondary),
            (1, ShowcaseRole::Tertiary),
        ],
        legend: &[
            "60 Ink",
            "30 Cloud Dancer + Teal",
            "9 Warm Grey",
            "1 Champagne",
        ],
        faces: "Instrument Serif display, Geist body",
    },
    ThemeCardCopy {
        theme: ThemeId::MonoDark,
        character: "Obsidian and Parchment: a parallel environment, not an inverted light theme.",
        ratio: &[
            (60, ShowcaseRole::Surface),
            (30, ShowcaseRole::Primary),
            (9, ShowcaseRole::Secondary),
            (1, ShowcaseRole::Tertiary),
        ],
        legend: &[
            "60 Obsidian",
            "30 Parchment",
            "9 Ash Stone",
            "1 Warm Bronze",
        ],
        faces: "Geist display and body, Fraunces editorial",
    },
];

/// The section's eyebrow.
pub const SECTION_EYEBROW: &str = "Brand · Themes · One system, six skins";

/// The section's intro, the Themes README's opening sentence.
pub const INTRO: &str =
    "Six themes re-skin the one system: three brand directions, each with a light and a dark theme, over one shared set of colour roles.";

/// The note under the cards: the explicit-tokens mechanic.
pub const NOTE: &str = "Each card paints its own theme's tokens whatever the gallery's theme is: the tokens are plain data the card passes down, never an ambient lookup.";

/// The light row's label.
pub const LIGHT_ROW: &str = "Light themes";

/// The dark row's label.
pub const DARK_ROW: &str = "Dark themes";

/// The display specimen every card sets in its own display face.
pub const SPECIMEN_LINE: &str = "Quiet is the new loud.";

/// The specimen row's primary action, the preview's own button.
pub const PRIMARY_ACTION: &str = "Book a call";

/// The specimen row's secondary action.
pub const SECONDARY_ACTION: &str = "Our work";

/// The specimen row's selected filter chip.
pub const CHIP_LABEL: &str = "Platform";

/// The themes showcase: the six miniature re-skinned specimens.
///
/// ```
/// use ui_kit::components::ThemesShowcase;
///
/// let showcase = ThemesShowcase::new();
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ThemesShowcase;

impl ThemesShowcase {
    /// The showcase.
    pub const fn new() -> Self {
        Self
    }

    /// Paint the showcase in the ambient `theme`: the eyebrow, intro
    /// and captions ambient, each card carrying its own theme's tokens.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> Response {
        let colors = theme.colors();
        ui.add(TrackedLabel::new(
            SECTION_EYEBROW,
            TypeStyle::Eyebrow.font_id(theme),
            TypeStyle::Eyebrow.tracking(),
            colors.text_accent,
        ));
        ui.add_space(spacing::SPACE_4);
        ui.add(egui::Label::new(specimen_job(
            INTRO,
            TypeStyle::Lede,
            theme,
            colors.text_muted(),
            None,
        )));
        ui.add_space(spacing::SPACE_6);

        // The preview's two rows: the light three, then the dark three,
        // each breaking into as many cards as fit and stacked one to a
        // row on a phone (see `card_rows`).
        for (row, group) in [(LIGHT_ROW, &CARDS[..3]), (DARK_ROW, &CARDS[3..])] {
            ui.label(row);
            ui.add_space(spacing::SPACE_2);
            card_rows(
                ui,
                CARD_WIDTH + 2.0 * CardPadding::Compact.spacing(),
                spacing::SPACE_3,
                group.len(),
                group,
                |ui, copy| theme_card(ui, flags, copy),
            );
            ui.add_space(spacing::SPACE_6);
        }

        ui.add(egui::Label::new(specimen_job(
            NOTE,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            None,
        )))
    }
}

/// Paint one theme card: everything inside resolves from `copy`'s own
/// theme, the whole point of the page, so the card re-skins whatever
/// the ambient theme is.
fn theme_card(ui: &mut Ui, flags: &UiKitFlags, copy: &ThemeCardCopy) {
    let card_theme = copy.theme;
    let colors = card_theme.colors();
    Card::new().compact().ui(ui, card_theme, flags, |ui| {
        // The cell's width less the card's padding: `CARD_WIDTH` where
        // the cards share a line, the column less the padding on a phone.
        let inner = ui.available_width();
        ui.set_min_width(inner);

        // The mark in this theme's primary, the id pill beside it.
        ui.horizontal(|ui| {
            ui.add(Logo::mark().height(spacing::SPACE_6).color(colors.primary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                egui::Frame::default()
                    .stroke(Stroke::new(1.0, colors.outline))
                    .corner_radius(CornerRadius::same(radii::RADIUS_FULL.min(255.0) as u8))
                    .inner_margin(egui::Margin {
                        left: spacing::SPACE_2 as i8,
                        right: spacing::SPACE_2 as i8,
                        top: spacing::SPACE_0_5 as i8,
                        bottom: spacing::SPACE_0_5 as i8,
                    })
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(tracked_text(
                            card_theme.id(),
                            FontId {
                                size: type_scale::TEXT_CAPTION,
                                family: TypeStyle::Mono.family(card_theme),
                            },
                            0.0,
                            None,
                            colors.text_muted(),
                            None,
                        )));
                    });
            });
        });

        // The tokens.json name and the character sentence.
        ui.add_space(spacing::SPACE_2);
        ui.add(egui::Label::new(tracked_text(
            card_theme.name(),
            FontId {
                size: type_scale::TEXT_BODY_LG,
                family: body_semibold_family(card_theme),
            },
            0.0,
            Some(type_scale::TEXT_BODY_LG * 1.25),
            colors.text_body(),
            None,
        )));
        ui.add(egui::Label::new(tracked_text(
            copy.character,
            FontId {
                size: type_scale::TEXT_BODY_SM,
                family: TypeStyle::Body.family(card_theme),
            },
            0.0,
            Some(20.0),
            colors.text_muted(),
            None,
        )));

        // The anchor swatches, one a row with its mono label beside:
        // the full token names need the row's width on a phone card.
        ui.add_space(spacing::SPACE_2);
        for pair in ShowcaseRole::SWATCHES.chunks(1) {
            ui.horizontal(|ui| {
                for role in pair {
                    let swatch_radius = CornerRadius::same(radii::RADIUS_SM as u8);
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::splat(spacing::SPACE_6), Sense::hover());
                    ui.painter()
                        .rect_filled(rect, swatch_radius, role.resolve(&colors));
                    // The edge is `outline`, the border role that holds
                    // 3:1 on the card in every theme: `outline-variant`
                    // is a 1.31 to 1.84:1 hairline, translucent in the
                    // Teal themes, and the surface-coloured swatches
                    // vanished into the card behind it.
                    ui.painter().rect_stroke(
                        rect,
                        swatch_radius,
                        Stroke::new(1.0, colors.outline),
                        egui::StrokeKind::Outside,
                    );
                    ui.add(egui::Label::new(tracked_text(
                        role.name(),
                        FontId {
                            size: type_scale::TEXT_EYEBROW,
                            family: TypeStyle::Mono.family(card_theme),
                        },
                        0.0,
                        None,
                        colors.text_muted(),
                        None,
                    )));
                    ui.add_space(spacing::SPACE_3);
                }
            });
        }

        // The ratio bar, bordered, and its legend.
        ui.add_space(spacing::SPACE_1);
        let (bar, _) = ui.allocate_exact_size(Vec2::new(inner, spacing::SPACE_4), Sense::hover());
        let total: u32 = copy.ratio.iter().map(|(share, _)| share).sum();
        let mut x = bar.left();
        for &(share, role) in copy.ratio {
            let w = bar.width() * share as f32 / total as f32;
            ui.painter().rect_filled(
                Rect::from_min_max(Pos2::new(x, bar.top()), Pos2::new(x + w, bar.bottom())),
                CornerRadius::same(0),
                role.resolve(&colors),
            );
            x += w;
        }
        ui.painter().rect_stroke(
            bar,
            CornerRadius::same(radii::RADIUS_SM as u8),
            Stroke::new(1.0, colors.outline_variant),
            egui::StrokeKind::Inside,
        );
        ui.add(egui::Label::new(tracked_text(
            &copy.legend.join(" · "),
            FontId {
                size: type_scale::TEXT_CAPTION,
                family: TypeStyle::Body.family(card_theme),
            },
            0.0,
            None,
            colors.text_muted(),
            None,
        )));

        // The display specimen in this theme's own faces, on a small
        // filled card: the small card of the issue's specimen row.
        ui.add_space(spacing::SPACE_2);
        Card::new()
            .filled()
            .compact()
            .ui(ui, card_theme, flags, |ui| {
                ui.set_min_width(ui.available_width());
                ui.add(egui::Label::new(tracked_text(
                    SPECIMEN_LINE,
                    FontId {
                        size: 22.0,
                        family: TypeStyle::Display.family(card_theme),
                    },
                    -0.01,
                    Some(26.0),
                    colors.text_display(),
                    None,
                )));
                ui.add(egui::Label::new(tracked_text(
                    copy.faces,
                    FontId {
                        size: type_scale::TEXT_CAPTION,
                        family: TypeStyle::Body.family(card_theme),
                    },
                    0.0,
                    None,
                    colors.text_muted(),
                    None,
                )));
            });

        // The live specimen row: two buttons and a selected chip, all
        // in this theme's tokens.
        ui.add_space(spacing::SPACE_2);
        ui.horizontal_wrapped(|ui| {
            Button::new(PRIMARY_ACTION)
                .small()
                .ui(ui, card_theme, flags);
            ui.add_space(spacing::SPACE_2);
            Button::new(SECONDARY_ACTION)
                .small()
                .emphasis(ButtonEmphasis::Secondary)
                .ui(ui, card_theme, flags);
            ui.add_space(spacing::SPACE_2);
            Chip::new(CHIP_LABEL)
                .set_selected(true)
                .ui(ui, card_theme, flags);
        });
    });
}

#[cfg(test)]
mod tests {
    use crate::components::brand_showcase::copy_passes_voice_checks;
    use crate::components::card::Card;
    use crate::states::composite_over;
    use crate::theme::ThemeId;
    use crate::tokens::spacing;

    use super::{
        ShowcaseRole, CARDS, CARD_WIDTH, CHIP_LABEL, DARK_ROW, INTRO, LIGHT_ROW, NOTE,
        PRIMARY_ACTION, SECONDARY_ACTION, SECTION_EYEBROW, SPECIMEN_LINE,
    };

    /// The showcase covers every theme exactly once: each of the six
    /// `ThemeId`s appears in `CARDS` once and only once, the light
    /// three before the dark three, as the preview's two rows group
    /// them.
    #[test]
    fn cards_cover_every_theme_exactly_once() {
        assert_eq!(CARDS.len(), ThemeId::ALL.len(), "six cards, no strays");
        for theme in ThemeId::ALL {
            let seen = CARDS.iter().filter(|card| card.theme == *theme).count();
            assert_eq!(seen, 1, "{} appears exactly once", theme.name());
        }
        for card in &CARDS[..3] {
            assert!(!card.theme.is_dark(), "the first row is the light three");
        }
        for card in &CARDS[3..] {
            assert!(card.theme.is_dark(), "the second row is the dark three");
        }
    }

    /// The ratio bars carry the directions' splits: every card's
    /// segments sum to one hundred, the surface canvas keeps the sixty
    /// share, and the tertiary guest stays at or under ten shares in
    /// every theme.
    #[test]
    fn ratio_segments_carry_the_directions_splits() {
        for card in CARDS {
            let name = card.theme.name();
            let sum: u32 = card.ratio.iter().map(|(share, _)| share).sum();
            assert_eq!(sum, 100, "{name}: shares sum to one hundred");
            let canvas = card
                .ratio
                .iter()
                .find(|(_, role)| *role == ShowcaseRole::Surface)
                .map(|(share, _)| *share);
            assert_eq!(canvas, Some(60), "{name}: the canvas holds the sixty share");
            let guest: u32 = card
                .ratio
                .iter()
                .filter(|(_, role)| *role == ShowcaseRole::Tertiary)
                .map(|(share, _)| share)
                .sum();
            assert!(guest <= 10, "{name}: the accent stays a guest");
            assert!(
                card.legend
                    .first()
                    .is_some_and(|entry| entry.starts_with("60")),
                "{name}: the legend names the canvas share"
            );
        }
    }

    /// Every copy string passes the voice checks: no em dashes, no
    /// ellipses, no exclamation marks, no emoji; middle dots only as
    /// the separator. The shape of issue 11's gallery-wide lint.
    #[test]
    fn copy_passes_the_voice_checks() {
        let mut texts = vec![
            SECTION_EYEBROW,
            INTRO,
            NOTE,
            LIGHT_ROW,
            DARK_ROW,
            SPECIMEN_LINE,
            PRIMARY_ACTION,
            SECONDARY_ACTION,
            CHIP_LABEL,
        ];
        for card in CARDS {
            texts.push(card.character);
            texts.push(card.faces);
            for entry in card.legend {
                texts.push(entry);
            }
        }
        for text in texts {
            assert!(copy_passes_voice_checks(text), "{text}");
        }
    }

    /// The showcase fits its column at every width, and the light three
    /// fall into the rows the width allows, read off where their names
    /// land: on a 360pt phone's page column (312pt inside the gallery's
    /// margins) they stack one to a row; on a 700pt window's (652pt) two
    /// share a row and the third wraps; on a desktop's (1152pt) the three
    /// share the preview's one row. (The cards once ran 1304pt past the
    /// phone's column: each card's right-aligned id pill claimed the rest
    /// of its line, and a wrapped row never breaks before a card.)
    #[test]
    fn cards_fit_the_column_and_share_rows_where_they_fit() {
        let flags = crate::states::UiKitFlags::default();
        let names = [ThemeId::Light, ThemeId::TealLight, ThemeId::MonoLight].map(ThemeId::name);
        for (width, rows) in [(312.0, [0, 1, 2]), (652.0, [0, 0, 1]), (1152.0, [0, 0, 0])] {
            let (size, painted) = crate::components::brand_showcase::lay_out(width, |ui| {
                super::ThemesShowcase::new().ui(ui, ThemeId::Light, &flags);
            });
            assert!(
                size.x <= width + 0.5,
                "at {width}pt the showcase runs {}pt past its column",
                size.x - width
            );
            assert_eq!(
                crate::components::brand_showcase::rows_of(&painted, &names),
                rows,
                "the light three's rows at {width}pt"
            );
        }
    }

    /// The showcase paints headless under every ambient theme (fonts
    /// for all six installed in a warm-up pass): the cards render
    /// whatever the gallery's theme is, each in its own tokens.
    #[test]
    fn paints_headless_under_every_ambient_theme() {
        let ctx = egui::Context::default();
        let flags = crate::states::UiKitFlags::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            for theme in ThemeId::ALL {
                crate::typography::install(*theme, ui.ctx());
            }
        });
        out.drop_without_applying_deltas();
        for ambient in ThemeId::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                super::ThemesShowcase::new().ui(ui, *ambient, &flags);
            });
            out.drop_without_applying_deltas();
        }
    }

    /// WCAG relative luminance of one opaque colour (a copy of the
    /// helper in `tokens/tests.rs`, private there).
    fn luminance(c: egui::Color32) -> f64 {
        let channel = |v: u8| {
            let s = f64::from(v) / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    /// WCAG contrast ratio between two opaque colours (1 to 21).
    fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Every rect `copy`'s card paints, headless, on the last of a few
    /// passes (so the card's sizing passes settle).
    fn painted_rects(copy: &super::ThemeCardCopy) -> Vec<egui::epaint::RectShape> {
        fn walk(shape: &egui::Shape, out: &mut Vec<egui::epaint::RectShape>) {
            match shape {
                egui::Shape::Rect(rect) => out.push(rect.clone()),
                egui::Shape::Vec(nested) => nested.iter().for_each(|shape| walk(shape, out)),
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        let flags = crate::states::UiKitFlags::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            for theme in ThemeId::ALL {
                crate::typography::install(*theme, ui.ctx());
            }
        })
        .drop_without_applying_deltas();
        let mut rects = Vec::new();
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(CARD_WIDTH + 64.0, 10_000.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| super::theme_card(ui, &flags, copy));
            rects.clear();
            for clipped in std::mem::take(&mut out.shapes) {
                walk(&clipped.shape, &mut rects);
            }
            out.drop_without_applying_deltas();
        }
        rects
    }

    /// Each card's role swatches, as the card paints them: the swatch
    /// keeps a border that holds 3:1 against the card it sits on, in all
    /// six themes. A role close to the card (the surfaces, on-primary in
    /// Teal Light) vanishes without one, and the hairline `outline-variant`
    /// (1.31 to 1.84:1, translucent in the Teal themes) is too faint to be
    /// the edge. 3:1 is what WCAG 1.4.11 asks of a graphic that carries
    /// meaning.
    #[test]
    fn every_swatch_on_every_card_keeps_a_visible_border() {
        let swatch_radius = egui::CornerRadius::same(crate::tokens::radii::RADIUS_SM as u8);
        let mut off = Vec::new();
        for copy in CARDS {
            let theme = copy.theme;
            let colors = theme.colors();
            // The card paints on its own elevated surface.
            let card = Card::new().compact().look(false, theme).surface;
            let rects = painted_rects(copy);
            let swatches: Vec<_> = rects
                .iter()
                .filter(|shape| {
                    shape.rect.size() == egui::Vec2::splat(spacing::SPACE_6)
                        && shape.corner_radius == swatch_radius
                        && shape.fill != egui::Color32::TRANSPARENT
                })
                .collect();
            assert_eq!(
                swatches.len(),
                ShowcaseRole::SWATCHES.len(),
                "{theme:?}: one swatch per role"
            );
            for (shape, role) in swatches.iter().zip(ShowcaseRole::SWATCHES) {
                assert_eq!(
                    shape.fill,
                    role.resolve(&colors),
                    "{theme:?}: the swatches run in role order, each in its own colour"
                );
                let border = rects
                    .iter()
                    .filter(|edge| {
                        edge.rect == shape.rect
                            && edge.fill == egui::Color32::TRANSPARENT
                            && edge.stroke.width >= 1.0
                    })
                    .map(|edge| contrast(composite_over(edge.stroke.color, card), card))
                    .fold(None, |best: Option<f64>, ratio| {
                        Some(best.map_or(ratio, |b| b.max(ratio)))
                    });
                match border {
                    None => off.push(format!("{theme:?} {}: no border", role.name())),
                    Some(ratio) if ratio < 3.0 => off.push(format!(
                        "{theme:?} {}: border {ratio:.2}:1 on the card surface, under 3:1",
                        role.name()
                    )),
                    Some(_) => {}
                }
            }
        }
        assert!(
            off.is_empty(),
            "{} swatches:\n{}",
            off.len(),
            off.join("\n")
        );
    }
}

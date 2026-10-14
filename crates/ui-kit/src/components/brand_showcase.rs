//! BrandShowcase: the brand-book's Brand page as a gallery section
//! (issue 10): the governing principle, the four tones of voice, the
//! three approved headline modes and the photography direction, per
//! `components/Brand`'s preview.
//!
//! Every colour is a role, so the page follows the theme: the
//! principle numeral and the voice cards' top rules take `indicator`,
//! the labels `text-accent`, and the do and don't pairs the `success`
//! and error containers with an icon each, so the verdict never rests
//! on colour alone. The photographs are flat placeholders in role
//! colours with a camera icon and a caption: the system's mock rule
//! (no invented illustration) until approved documentary photography
//! exists.
//!
//! The copy is lifted from the spec pages and lives in consts here, so
//! the voice lint (issue 11) can scan it. One documented adaptation:
//! the don't tile for the cliché opening drops the source's trailing
//! ellipsis (the voice forbids ellipses outside operational busy
//! patterns), and the don't strings generally are quoted
//! anti-patterns, exempt from the vocabulary discipline by design
//! while still passing the punctuation checks.

use egui::{Color32, CornerRadius, FontId, Response, Sense, Ui, Vec2};

use crate::components::card::Card;
use crate::icon::{family_for_theme, icon, IconSize};
use crate::icons::Icon;
use crate::states::UiKitFlags;
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale, ThemeColors};
use crate::typography::{display_bold_family, specimen_job, tracked_text, TrackedLabel, TypeStyle};

/// One voice card's content width; four of them and their gutters fit
/// the gallery's page.
pub const VOICE_CARD_WIDTH: f32 = 184.0;

/// One do or don't tile's content width; a pair fills the page.
pub const MODE_TILE_WIDTH: f32 = 380.0;

/// The lead photo tile's size: tall, spanning the four small tiles'
/// two rows.
pub const PHOTO_TILE_WIDE: Vec2 = Vec2::new(300.0, 220.0);

/// The small photo tiles' size.
pub const PHOTO_TILE_SMALL: Vec2 = Vec2::new(160.0, 104.0);

// ---- the voice checks -----------------------------------------------------

/// Whether `text` passes the voice checks these sections' copy is held
/// to: no em dashes (U+2014), no ellipses (U+2026) outside operational
/// busy patterns, no exclamation marks, no emoji, so the middle dot
/// stays the only metadata separator. The shape of issue 11's
/// gallery-wide voice lint, kept public here so that lint can grow
/// from it.
pub fn copy_passes_voice_checks(text: &str) -> bool {
    !text.contains('\u{2014}')
        && !text.contains('\u{2026}')
        && !text.contains('!')
        && !text.chars().any(is_emoji)
}

/// The emoji and symbol blocks the checklist forbids anywhere.
fn is_emoji(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF   // emoji, symbols, transport, supplemental
        | 0x2600..=0x27BF   // misc symbols and dingbats
        | 0x2B00..=0x2BFF   // arrows and stars
        | 0xFE0F            // variation selector
        | 0x200D) // zero-width joiner
}

// ---- the copy --------------------------------------------------------------

/// The section's eyebrow.
pub const SECTION_EYEBROW: &str = "Brand · Voice, headlines, photography";

/// The principle panel's eyebrow.
pub const PRINCIPLE_EYEBROW: &str = "Governing principle";

/// The principle panel's numeral, in `indicator`.
pub const PRINCIPLE_NUMERAL: &str = "01";

/// The principle's headline.
pub const PRINCIPLE_HEADLINE: &str = "The discipline of the system is the luxury signal.";

/// The principle's body.
pub const PRINCIPLE_BODY: &str = "Not ornament. Not saturation. Not motion. Four colors do the entire load. When the urge to add a fifth appears, the answer is more whitespace. Premium isn't a price point. It's a discipline.";

/// The voice section's heading.
pub const VOICE_HEADING: &str = "Voice: one constant, four tones";

/// The voice section's intro.
pub const VOICE_INTRO: &str =
    "Voice is considered, direct, warm, confident-never-boastful. Tone shifts by surface; commit to one position, never sit between two.";

/// The four tone positions: label, register word, one example sentence
/// each, from the Brand preview and the README's tone spectrum.
pub const VOICE_POSITIONS: &[(&str, &str, &str)] = &[
    (
        "Editorial",
        "Reflective.",
        "We rebuilt the truck radio in sixteen weeks. The hard part wasn't the code. It was the conversation about what 'done' meant.",
    ),
    (
        "Professional",
        "Direct.",
        "We do platform engineering and design-engineering for large Nordic enterprises. Senior people. Permanent contracts. No subcontracting.",
    ),
    (
        "Operational",
        "Short.",
        "Saved. Eight changes since 14:32. Refresh to see them.",
    ),
    (
        "Reassuring",
        "Soft.",
        "We couldn't reach the server just now. Your work is held locally and will sync when you reconnect.",
    ),
];

/// The headlines section's heading.
pub const HEADLINES_HEADING: &str = "Headlines: three approved modes";

/// The three approved modes as do and don't pairs: the tile's label and
/// its example. The don't strings are quoted anti-patterns.
pub const HEADLINE_MODES: &[(&str, &str)] = &[
    (
        "Do · the specific claim",
        "We rebuilt Volvo's truck radio in sixteen weeks.",
    ),
    (
        "Don't · the vague aspiration",
        "Unleashing the power of innovation.",
    ),
    (
        "Do · the considered observation",
        "Premium isn't a price point. It's a discipline.",
    ),
    (
        "Don't · the noun-pair",
        "Strategic Outcomes. Engineered Trust.",
    ),
    (
        "Do · the plain question",
        "Why does most consulting writing sound the same?",
    ),
    (
        "Don't · the cliché opening",
        "In today's rapidly evolving digital landscape",
    ),
];

/// The photography section's heading.
pub const PHOTO_HEADING: &str = "Photography: documentary, naturally lit";

/// The photography section's intro.
pub const PHOTO_INTRO: &str = "People doing identifiable real work. Available light, Nordic temperament, restrained saturation. No staged handshakes, no laughing-at-laptops, no AI-generated people, ever.";

/// The note under the tiles: the mock rule.
pub const PHOTO_NOTE: &str = "Flat role tints stand in until approved photography exists: the system's mock rule, so no illustration is invented.";

/// Which role pair one photo tile paints: the ground and the ink over
/// it, the preview's five grounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoGround {
    /// `primary` under `on-primary`: the tall lead tile.
    Primary,
    /// `tertiary-container` under `on-tertiary-container`.
    TertiaryContainer,
    /// `surface-container-highest` under `on-surface`.
    SurfaceContainerHighest,
    /// `inverse-surface` under `inverse-on-surface`.
    InverseSurface,
    /// `secondary-container` under `on-secondary-container`.
    SecondaryContainer,
}

impl PhotoGround {
    /// The ground's fill and the ink over it, in a theme.
    pub const fn pair(self, colors: &ThemeColors) -> (Color32, Color32) {
        match self {
            PhotoGround::Primary => (colors.primary, colors.on_primary),
            PhotoGround::TertiaryContainer => {
                (colors.tertiary_container, colors.on_tertiary_container)
            }
            PhotoGround::SurfaceContainerHighest => {
                (colors.surface_container_highest, colors.on_surface)
            }
            PhotoGround::InverseSurface => (colors.inverse_surface, colors.inverse_on_surface),
            PhotoGround::SecondaryContainer => {
                (colors.secondary_container, colors.on_secondary_container)
            }
        }
    }
}

/// The five photo tiles: their ground and their caption, the preview's
/// own captions with middle-dot metadata.
pub const PHOTO_TILES: &[(PhotoGround, &str)] = &[
    (
        PhotoGround::Primary,
        "Volvo Trucks workshop · Lundby · 06:42",
    ),
    (
        PhotoGround::TertiaryContainer,
        "Bronze foil cover · folio v9",
    ),
    (
        PhotoGround::SurfaceContainerHighest,
        "Whiteboard · architecture review",
    ),
    (
        PhotoGround::InverseSurface,
        "After-hours debug · Gothenburg HQ",
    ),
    (
        PhotoGround::SecondaryContainer,
        "Field notebook · Aptos Serif print",
    ),
];

// ---- the widget -------------------------------------------------------------

/// The brand showcase: the principle, the tone spectrum, the headline
/// modes and the photography direction.
///
/// ```
/// use ui_kit::components::BrandShowcase;
///
/// let showcase = BrandShowcase::new();
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct BrandShowcase;

impl BrandShowcase {
    /// The showcase.
    pub const fn new() -> Self {
        Self
    }

    /// Paint the showcase in the ambient `theme`, every colour a role
    /// so the page follows it: the principle, the tone spectrum, the
    /// headline modes, the photography direction.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> Response {
        let colors = theme.colors();
        ui.add(TrackedLabel::new(
            SECTION_EYEBROW,
            TypeStyle::Eyebrow.font_id(theme),
            TypeStyle::Eyebrow.tracking(),
            colors.text_accent,
        ));
        ui.add_space(spacing::SPACE_4);

        // The governing principle: the numeral in indicator beside the
        // eyebrow, headline and body, the preview's one panel.
        Card::new().large().ui(ui, theme, flags, |ui| {
            // Captured while the Ui is still the card's bounded
            // interior: inside the horizontal below, available width is
            // unbounded.
            let panel_width = ui.available_width();
            ui.horizontal(|ui| {
                let _ = panel_width;
                ui.add(egui::Label::new(tracked_text(
                    PRINCIPLE_NUMERAL,
                    FontId {
                        size: type_scale::TEXT_DISPLAY_LG,
                        family: display_bold_family(theme),
                    },
                    -0.05,
                    Some(type_scale::TEXT_DISPLAY_LG * 0.9),
                    colors.indicator,
                    None,
                )));
                ui.add_space(spacing::SPACE_8);
                ui.vertical(|ui| {
                    let column_width = panel_width - type_scale::TEXT_DISPLAY_LG - spacing::SPACE_8;
                    ui.add(TrackedLabel::new(
                        PRINCIPLE_EYEBROW,
                        TypeStyle::Eyebrow.font_id(theme),
                        TypeStyle::Eyebrow.tracking(),
                        colors.text_accent,
                    ));
                    ui.add_space(spacing::SPACE_2);
                    ui.add(egui::Label::new(tracked_text(
                        PRINCIPLE_HEADLINE,
                        FontId {
                            size: type_scale::TEXT_H4,
                            family: TypeStyle::Headline3.family(theme),
                        },
                        -0.02,
                        Some(type_scale::TEXT_H4 * 1.15),
                        colors.text_display(),
                        Some(column_width),
                    )));
                    ui.add_space(spacing::SPACE_2);
                    ui.add(egui::Label::new(specimen_job(
                        PRINCIPLE_BODY,
                        TypeStyle::Body,
                        theme,
                        colors.text_body(),
                        Some(column_width),
                    )));
                });
            });
        });

        // Voice: one constant, four tones. Each card carries the
        // indicator top rule, the label in text-accent, the register
        // word and one example sentence.
        ui.add_space(spacing::SPACE_6);
        ui.label(VOICE_HEADING);
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            VOICE_INTRO,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(VOICE_CARD_WIDTH - spacing::SPACE_8),
        )));
        ui.add_space(spacing::SPACE_2);
        // Wrapped, not one long row: the cards stack on a phone.
        ui.horizontal_wrapped(|ui| {
            for (label, register, example) in VOICE_POSITIONS {
                Card::new().filled().compact().ui(ui, theme, flags, |ui| {
                    ui.set_min_width(VOICE_CARD_WIDTH);
                    // The indicator rule across the card's top, the
                    // preview's 2px border-top.
                    let (rule, _) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 2.0), Sense::hover());
                    ui.painter()
                        .rect_filled(rule, CornerRadius::same(0), colors.indicator);
                    ui.add_space(spacing::SPACE_2);
                    ui.add(TrackedLabel::new(
                        label,
                        TypeStyle::Eyebrow.font_id(theme),
                        TypeStyle::Eyebrow.tracking(),
                        colors.text_accent,
                    ));
                    ui.add_space(spacing::SPACE_1);
                    ui.add(egui::Label::new(tracked_text(
                        register,
                        FontId {
                            size: 22.0,
                            family: TypeStyle::Headline3.family(theme),
                        },
                        -0.01,
                        Some(26.0),
                        colors.text_display(),
                        Some(VOICE_CARD_WIDTH - spacing::SPACE_8),
                    )));
                    ui.add_space(spacing::SPACE_1);
                    ui.add(egui::Label::new(specimen_job(
                        example,
                        TypeStyle::Caption,
                        theme,
                        colors.text_body(),
                        Some(VOICE_CARD_WIDTH - spacing::SPACE_8),
                    )));
                });
                ui.add_space(spacing::SPACE_3);
            }
        });

        // Headlines: the three approved modes as do and don't pairs,
        // the verdict carried by the icon and the word, never colour
        // alone.
        ui.add_space(spacing::SPACE_6);
        ui.label(HEADLINES_HEADING);
        ui.add_space(spacing::SPACE_2);
        egui::Grid::new("brand-headline-modes")
            .num_columns(2)
            .spacing([spacing::SPACE_3, spacing::SPACE_3])
            .show(ui, |ui| {
                for (index, (label, example)) in HEADLINE_MODES.iter().enumerate() {
                    let is_do = label.starts_with("Do");
                    let (fill, ink, glyph) = if is_do {
                        (
                            colors.success_container,
                            colors.on_success_container,
                            Icon::CheckCircle,
                        )
                    } else {
                        (
                            colors.error_container,
                            colors.on_error_container,
                            Icon::Block,
                        )
                    };
                    egui::Frame::default()
                        .fill(fill)
                        .corner_radius(radii::RADIUS_MD as u8)
                        .inner_margin(spacing::SPACE_4)
                        .show(ui, |ui| {
                            ui.set_min_width(MODE_TILE_WIDTH);
                            ui.horizontal(|ui| {
                                icon(ui, glyph, IconSize::Sm, ink);
                                ui.add_space(spacing::SPACE_2);
                                ui.add(TrackedLabel::new(
                                    label,
                                    TypeStyle::Eyebrow.font_id(theme),
                                    0.18,
                                    ink,
                                ));
                            });
                            ui.add_space(spacing::SPACE_2);
                            ui.add(egui::Label::new(tracked_text(
                                example,
                                FontId {
                                    size: 18.0,
                                    family: TypeStyle::Headline3.family(theme),
                                },
                                -0.01,
                                Some(24.0),
                                ink,
                                Some(VOICE_CARD_WIDTH - spacing::SPACE_8),
                            )));
                        });
                    if index % 2 == 1 {
                        ui.end_row();
                    }
                }
            });

        // Photography: the direction copy, then the five tinted
        // placeholder tiles, the lead one tall beside a two-by-two.
        ui.add_space(spacing::SPACE_6);
        ui.label(PHOTO_HEADING);
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            PHOTO_INTRO,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(VOICE_CARD_WIDTH - spacing::SPACE_8),
        )));
        ui.add_space(spacing::SPACE_2);
        ui.horizontal(|ui| {
            let (lead_ground, lead_caption) = PHOTO_TILES[0];
            photo_tile(ui, theme, lead_ground, lead_caption, PHOTO_TILE_WIDE);
            ui.add_space(spacing::SPACE_3);
            ui.vertical(|ui| {
                for pair in PHOTO_TILES[1..].chunks(2) {
                    ui.horizontal(|ui| {
                        for (ground, caption) in pair {
                            photo_tile(ui, theme, *ground, caption, PHOTO_TILE_SMALL);
                            ui.add_space(spacing::SPACE_3);
                        }
                    });
                    ui.add_space(spacing::SPACE_3);
                }
            });
        });
        ui.add_space(spacing::SPACE_2);
        ui.add(egui::Label::new(specimen_job(
            PHOTO_NOTE,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(VOICE_CARD_WIDTH - spacing::SPACE_8),
        )))
    }
}

/// Paint one photo placeholder tile: a flat role tint, the camera glyph
/// at feature size and the caption in the ground's ink, the mock rule
/// until approved photography exists.
fn photo_tile(ui: &mut Ui, theme: ThemeId, ground: PhotoGround, caption: &str, size: Vec2) {
    let colors = theme.colors();
    let (fill, ink) = ground.pair(&colors);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let radius = CornerRadius::same(radii::RADIUS_MD as u8);
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().text(
        rect.left_top() + Vec2::new(spacing::SPACE_3, spacing::SPACE_3),
        egui::Align2::LEFT_TOP,
        Icon::PhotoCamera.as_char().to_string(),
        FontId {
            size: IconSize::Xl.px(),
            family: family_for_theme(theme, false),
        },
        ink,
    );
    ui.painter().text(
        rect.left_bottom() + Vec2::new(spacing::SPACE_3, -spacing::SPACE_3),
        egui::Align2::LEFT_BOTTOM,
        caption,
        FontId {
            size: type_scale::TEXT_CAPTION,
            family: TypeStyle::Body.family(theme),
        },
        ink,
    );
}

#[cfg(test)]
mod tests {
    use super::{
        copy_passes_voice_checks, PhotoGround, HEADLINES_HEADING, HEADLINE_MODES, PHOTO_HEADING,
        PHOTO_INTRO, PHOTO_NOTE, PHOTO_TILES, PRINCIPLE_BODY, PRINCIPLE_EYEBROW,
        PRINCIPLE_HEADLINE, PRINCIPLE_NUMERAL, SECTION_EYEBROW, VOICE_HEADING, VOICE_INTRO,
        VOICE_POSITIONS,
    };
    use crate::theme::ThemeId;

    /// The tone spectrum is the four positions the README names, each
    /// with its register word and exactly one example sentence.
    #[test]
    fn voice_positions_are_the_four_tones() {
        assert_eq!(VOICE_POSITIONS.len(), 4, "one constant, four tones");
        let labels: Vec<&str> = VOICE_POSITIONS.iter().map(|(label, _, _)| *label).collect();
        assert_eq!(
            labels,
            ["Editorial", "Professional", "Operational", "Reassuring"]
        );
        for (label, register, example) in VOICE_POSITIONS {
            assert!(register.ends_with('.'), "{label}: the register is a word");
            assert!(
                example.split('.').count() >= 2,
                "{label}: one example sentence at least"
            );
        }
    }

    /// The headline modes are three do and don't pairs, alternating,
    /// the dos naming the README's three approved modes.
    #[test]
    fn headline_modes_pair_three_dos_with_three_donts() {
        assert_eq!(
            HEADLINE_MODES.len(),
            6,
            "three approved modes, do and don't"
        );
        for (index, (label, _)) in HEADLINE_MODES.iter().enumerate() {
            let is_do = label.starts_with("Do ·");
            assert_eq!(
                is_do,
                index % 2 == 0,
                "{label}: dos and don'ts alternate, dos first"
            );
        }
        assert_eq!(
            HEADLINE_MODES
                .iter()
                .filter(|(l, _)| l.starts_with("Do ·"))
                .count(),
            3
        );
        assert_eq!(
            HEADLINE_MODES
                .iter()
                .filter(|(l, _)| l.starts_with("Don't ·"))
                .count(),
            3
        );
    }

    /// The photography direction shows five tiles over five distinct
    /// role grounds, each caption carrying middle-dot metadata.
    #[test]
    fn photo_tiles_span_five_distinct_grounds() {
        assert_eq!(PHOTO_TILES.len(), 5, "the preview's five tiles");
        let mut grounds: Vec<PhotoGround> = PHOTO_TILES.iter().map(|(ground, _)| *ground).collect();
        grounds.sort_by_key(|ground| *ground as i32);
        grounds.dedup();
        assert_eq!(grounds.len(), 5, "five distinct grounds");
        for (ground, caption) in PHOTO_TILES {
            assert!(
                caption.contains(" · "),
                "{ground:?}: metadata separated by middle dots"
            );
        }
    }

    /// Every copy string passes the voice checks: no em dashes, no
    /// ellipses, no exclamation marks, no emoji; the middle dot the
    /// only separator. The prepared shape of issue 11's gallery-wide
    /// lint.
    #[test]
    fn copy_passes_the_voice_checks() {
        let mut texts = vec![
            SECTION_EYEBROW,
            PRINCIPLE_EYEBROW,
            PRINCIPLE_NUMERAL,
            PRINCIPLE_HEADLINE,
            PRINCIPLE_BODY,
            VOICE_HEADING,
            VOICE_INTRO,
            HEADLINES_HEADING,
            PHOTO_HEADING,
            PHOTO_INTRO,
            PHOTO_NOTE,
        ];
        for (_, register, example) in VOICE_POSITIONS {
            texts.push(register);
            texts.push(example);
        }
        for (label, example) in HEADLINE_MODES {
            texts.push(label);
            texts.push(example);
        }
        for (_, caption) in PHOTO_TILES {
            texts.push(caption);
        }
        for text in texts {
            assert!(copy_passes_voice_checks(text), "{text}");
        }
    }

    /// The principle body keeps the README's own words: the governing
    /// principle's seven sentences, lifted verbatim from the Brand
    /// preview, opening on the negations and closing on the discipline.
    #[test]
    fn principle_body_is_the_specs_own_copy() {
        let sentences: Vec<&str> = PRINCIPLE_BODY.split(". ").collect();
        assert_eq!(
            sentences.len(),
            7,
            "seven sentences, verbatim from the preview"
        );
        assert!(PRINCIPLE_BODY.starts_with("Not ornament."));
        assert!(PRINCIPLE_BODY.contains("the answer is more whitespace"));
        assert!(PRINCIPLE_BODY.ends_with("It's a discipline."));
    }

    /// The showcase paints headless in every theme (fonts installed in
    /// a warm-up pass), every colour a role so the page follows the
    /// theme it is given.
    #[test]
    fn paints_headless_in_every_theme() {
        let ctx = egui::Context::default();
        let flags = crate::states::UiKitFlags::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            for theme in ThemeId::ALL {
                crate::typography::install(*theme, ui.ctx());
            }
        });
        out.drop_without_applying_deltas();
        for theme in ThemeId::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                super::BrandShowcase::new().ui(ui, *theme, &flags);
            });
            out.drop_without_applying_deltas();
        }
    }
}

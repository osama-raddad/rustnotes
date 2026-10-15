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

use std::borrow::Cow;
use std::sync::Arc;

use egui::{Color32, CornerRadius, FontId, Galley, Painter, Response, Sense, Ui, Vec2};

use crate::components::card::{Card, CardPadding};
use crate::icon::{family_for_theme, icon, IconSize};
use crate::icons::Icon;
use crate::states::UiKitFlags;
use crate::theme::ThemeId;
use crate::tokens::{breakpoints, radii, spacing, type_scale, ThemeColors};
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

// ---- the card rows ----------------------------------------------------------

/// How many cards `width` wide sit side by side, `gap` apart, in
/// `available`: one under the compact breakpoint (`breakpoint-tablet`,
/// where the design system's mobile layout ends), so a phone stacks
/// them; above it as many as fit, up to the section's own row of `max`,
/// and never fewer than one.
pub(crate) fn card_columns(available: f32, width: f32, gap: f32, max: usize) -> usize {
    if available < breakpoints::BREAKPOINT_TABLET {
        return 1;
    }
    let fit = ((available + gap) / (width + gap)).floor().max(1.0) as usize;
    fit.min(max.max(1))
}

/// Lay `items` out as cards in rows of [`card_columns`], `gap` apart
/// and `gap` between the rows, each card placed at its `width` up
/// front. egui's `horizontal_wrapped` moves a widget to the next line
/// only when it knows the widget's size before laying it out, which a
/// card (a child `Ui`) never offers, so a wrapped row of cards runs on
/// past the window's edge. The up-front width is what lets the rows
/// break, and it bounds a card whose own rows reach for all the width
/// they are given (a right-aligned pill, a full-width rule). Rows hang
/// from their tops, the cards' heights their own. The gap is between
/// the cards only: a child `Ui` clones its parent's style, so each cell
/// hands its card the spacing the caller laid out with, and the card's
/// own rows keep egui's gaps rather than the row's.
pub(crate) fn card_rows<T>(
    ui: &mut Ui,
    width: f32,
    gap: f32,
    max: usize,
    items: &[T],
    mut card: impl FnMut(&mut Ui, &T),
) {
    let inner = ui.spacing().item_spacing;
    let per_row = card_columns(ui.available_width(), width, gap, max);
    for (index, row) in items.chunks(per_row).enumerate() {
        if index > 0 {
            // The item spacing already rides after the row above.
            ui.add_space((gap - ui.spacing().item_spacing.y).max(0.0));
        }
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for item in row {
                ui.allocate_ui_with_layout(
                    Vec2::new(width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing = inner;
                        card(ui, item);
                    },
                );
            }
        });
    }
}

/// Lay a showcase out headless in a column `width` wide, every theme's
/// faces installed in a warm-up pass, and hand back its laid-out size
/// beside every text it painted with that text's painted rect: the
/// showcases' layout tests read their rows off where the card titles
/// land, and the gaps inside a card off where its texts sit. The
/// column runs tall so nothing is culled.
#[cfg(test)]
pub(crate) fn lay_out(
    width: f32,
    mut paint: impl FnMut(&mut Ui),
) -> (Vec2, Vec<(String, egui::Rect)>) {
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => {
                found.push((
                    text.galley.text().to_owned(),
                    text.galley.rect.translate(text.pos.to_vec2()),
                ));
            }
            egui::Shape::Vec(nested) => nested.iter().for_each(|shape| walk(shape, found)),
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::icon::install(ui.ctx());
        for theme in ThemeId::ALL {
            crate::typography::install(*theme, ui.ctx());
        }
    })
    .drop_without_applying_deltas();
    let mut size = Vec2::ZERO;
    let mut found = Vec::new();
    // A few passes, so the cards' sizing passes settle.
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(width, 10_000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            paint(ui);
            size = ui.min_rect().size();
        });
        found.clear();
        for clipped in std::mem::take(&mut out.shapes) {
            walk(&clipped.shape, &mut found);
        }
        out.drop_without_applying_deltas();
    }
    (size, found)
}

/// The row each of `wanted` sits in, counting the distinct top edges
/// among them from the top (half a point of slack): how the layout
/// tests read a grid of cards. A text the layout never painted panics.
#[cfg(test)]
pub(crate) fn rows_of(painted: &[(String, egui::Rect)], wanted: &[&str]) -> Vec<usize> {
    let tops: Vec<f32> = wanted
        .iter()
        .map(|text| {
            painted
                .iter()
                .find(|(seen, _)| seen == text)
                .map(|(_, rect)| rect.top())
                .unwrap_or_else(|| panic!("{text:?} was never painted"))
        })
        .collect();
    let mut distinct: Vec<f32> = Vec::new();
    for top in &tops {
        if !distinct.iter().any(|seen| (seen - top).abs() < 0.5) {
            distinct.push(*top);
        }
    }
    distinct.sort_by(f32::total_cmp);
    tops.iter()
        .map(|top| {
            distinct
                .iter()
                .position(|seen| (seen - top).abs() < 0.5)
                .expect("every top is among the distinct ones")
        })
        .collect()
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
        // The four tones in one row where they fit, stacked one to a row
        // on a phone (see `card_rows`).
        card_rows(
            ui,
            VOICE_CARD_WIDTH + 2.0 * CardPadding::Compact.spacing(),
            spacing::SPACE_3,
            VOICE_POSITIONS.len(),
            VOICE_POSITIONS,
            |ui, (label, register, example)| {
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
            },
        );

        // Headlines: the three approved modes as do and don't pairs,
        // the verdict carried by the icon and the word, never colour
        // alone. A pair to a row where two tiles fit; on a phone the
        // tiles stack, each no wider than the page.
        ui.add_space(spacing::SPACE_6);
        ui.label(HEADLINES_HEADING);
        ui.add_space(spacing::SPACE_2);
        let tile_width = (MODE_TILE_WIDTH + 2.0 * spacing::SPACE_4).min(ui.available_width());
        card_rows(
            ui,
            tile_width,
            spacing::SPACE_3,
            2,
            HEADLINE_MODES,
            |ui, (label, example)| {
                // The verdict word and its separator: a bare "Do" also
                // opens every "Don't".
                let is_do = label.starts_with("Do ·");
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
                        ui.set_min_width(tile_width - 2.0 * spacing::SPACE_4);
                        ui.horizontal(|ui| {
                            icon(ui, glyph, IconSize::Sm, ink);
                            ui.add_space(spacing::SPACE_2);
                            // A row's labels keep one line by default; this
                            // one wraps beside the icon instead, where the
                            // tile runs out of width (a phone's tile is no
                            // wider than the page, and "the considered
                            // observation" outruns it).
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
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
            },
        );

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
        let (lead_ground, lead_caption) = PHOTO_TILES[0];
        // The lead tile beside the two-by-two where that composition
        // fits; on a phone, and wherever it would not, the lead tile
        // leads and the small tiles follow in rows of what fits, one to
        // a row under the compact breakpoint. The composition's
        // footprint is the lead tile and two small ones across, each
        // followed by its gap: the row's item spacing and the 12px
        // space.
        let gap = spacing::SPACE_3 + ui.spacing().item_spacing.x;
        let composition = PHOTO_TILE_WIDE.x + 2.0 * PHOTO_TILE_SMALL.x + 3.0 * gap;
        let available = ui.available_width();
        if available < breakpoints::BREAKPOINT_TABLET || available < composition {
            photo_tile(ui, theme, lead_ground, lead_caption, PHOTO_TILE_WIDE);
            ui.add_space(spacing::SPACE_3);
            card_rows(
                ui,
                PHOTO_TILE_SMALL.x,
                spacing::SPACE_3,
                2,
                &PHOTO_TILES[1..],
                |ui, (ground, caption)| photo_tile(ui, theme, *ground, caption, PHOTO_TILE_SMALL),
            );
        } else {
            ui.horizontal(|ui| {
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
        }
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
    // The caption wraps inside the tile, the tile's inset kept on both
    // sides, and hangs from the bottom inset so its last line sits where
    // the one-line caption did.
    let font = FontId {
        size: type_scale::TEXT_CAPTION,
        family: TypeStyle::Body.family(theme),
    };
    let max_width = size.x - 2.0 * spacing::SPACE_3;
    let galley = caption_galley(ui.painter(), caption, font, ink, max_width);
    let corner = rect.left_bottom() + Vec2::new(spacing::SPACE_3, -spacing::SPACE_3);
    let at = egui::Align2::LEFT_BOTTOM.anchor_size(corner, galley.size());
    ui.painter().galley(at.min, galley, ink);
}

/// `caption` in `font` laid out to wrap within `max_width`. egui breaks
/// a word wider than the line mid-word onto the next row, so such a
/// word is cut short with an ellipsis first and the rest of the caption
/// kept: every word then fits, the wrap only ever falls between words,
/// and no row runs past `max_width`.
fn caption_galley(
    painter: &Painter,
    caption: &str,
    font: FontId,
    ink: Color32,
    max_width: f32,
) -> Arc<Galley> {
    let fits = |text: &str| {
        painter
            .layout_no_wrap(text.to_owned(), font.clone(), ink)
            .size()
            .x
            <= max_width
    };
    let fitted: Vec<Cow<str>> = caption
        .split(' ')
        .map(|word| {
            if fits(word) {
                Cow::Borrowed(word)
            } else {
                Cow::Owned(ellipsize(word, fits))
            }
        })
        .collect();
    painter.layout(fitted.join(" "), font, ink, max_width)
}

/// `word` cut from its end until what is left and an ellipsis `fits`,
/// at least one letter kept.
fn ellipsize(word: &str, fits: impl Fn(&str) -> bool) -> String {
    let ends: Vec<usize> = word.char_indices().map(|(at, _)| at).skip(1).collect();
    let cut = |end: usize| format!("{}\u{2026}", &word[..end]);
    ends.iter()
        .rev()
        .map(|&end| cut(end))
        .find(|cut| fits(cut))
        .or_else(|| ends.first().map(|&end| cut(end)))
        .unwrap_or_else(|| word.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        copy_passes_voice_checks, PhotoGround, HEADLINES_HEADING, HEADLINE_MODES, PHOTO_HEADING,
        PHOTO_INTRO, PHOTO_NOTE, PHOTO_TILES, PHOTO_TILE_SMALL, PHOTO_TILE_WIDE, PRINCIPLE_BODY,
        PRINCIPLE_EYEBROW, PRINCIPLE_HEADLINE, PRINCIPLE_NUMERAL, SECTION_EYEBROW, VOICE_HEADING,
        VOICE_INTRO, VOICE_POSITIONS,
    };
    use crate::icon::IconSize;
    use crate::icons::Icon;
    use crate::theme::ThemeId;
    use crate::tokens::spacing;

    /// The gap between `label` and the `glyph` icon painted on its line,
    /// read off the painted texts: the label's left edge less the right
    /// edge of the icon's cell, a small icon's square centred on the
    /// glyph. The icon is the one whose centre falls inside the label's
    /// own height, so the pairing holds wherever else the glyph shows. A
    /// label with no such icon beside it panics.
    fn icon_to_label(painted: &[(String, egui::Rect)], glyph: Icon, label: &str) -> f32 {
        let text = painted
            .iter()
            .find(|(seen, _)| seen == label)
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| panic!("{label:?} was never painted"));
        let glyph_text = glyph.as_char().to_string();
        let cell = painted
            .iter()
            .filter(|(seen, _)| *seen == glyph_text)
            .map(|(_, rect)| *rect)
            .find(|rect| text.y_range().contains(rect.center().y))
            .unwrap_or_else(|| panic!("no {glyph:?} icon beside {label:?}"));
        text.left() - (cell.center().x + IconSize::Sm.px() / 2.0)
    }

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

    /// The showcase fits its column at every width, and its cards fall
    /// into the rows the width allows, read off where their labels land:
    /// on a 360pt phone's page column (312pt inside the gallery's
    /// margins) the four voice cards and the first do and don't pair
    /// stack one to a row; on a 700pt window's (652pt) two voice cards
    /// share a row and the pair, too wide for two, still stacks; on a
    /// desktop's (1152pt) the four tones share one row and the pair
    /// another. (The cards once ran 1044pt past the phone's column: a
    /// wrapped row never breaks before a card, each voice card's top
    /// rule claimed the rest of its line, and the do and don't grid
    /// kept two 412pt tiles side by side.) Inside the cards the spacing
    /// stays the caller's at every width, the rows' 12pt gap kept to
    /// the gaps between the cards: each tile's icon sits egui's default
    /// item spacing and its `space-2` from its label, the do tile's a
    /// check and the don't tile's a block.
    #[test]
    fn cards_fit_the_column_and_share_rows_where_they_fit() {
        let flags = crate::states::UiKitFlags::default();
        let voices: Vec<String> = VOICE_POSITIONS
            .iter()
            .map(|(label, _, _)| label.to_uppercase())
            .collect();
        let voices: Vec<&str> = voices.iter().map(String::as_str).collect();
        let pair: Vec<String> = HEADLINE_MODES[..2]
            .iter()
            .map(|(label, _)| label.to_uppercase())
            .collect();
        let pair: Vec<&str> = pair.iter().map(String::as_str).collect();
        for (width, voice_rows, pair_rows) in [
            (312.0, [0, 1, 2, 3], [0, 1]),
            (652.0, [0, 0, 1, 1], [0, 1]),
            (1152.0, [0, 0, 0, 0], [0, 0]),
        ] {
            let (size, painted) = super::lay_out(width, |ui| {
                super::BrandShowcase::new().ui(ui, ThemeId::Light, &flags);
            });
            assert!(
                size.x <= width + 0.5,
                "at {width}pt the showcase runs {}pt past its column",
                size.x - width
            );
            assert_eq!(
                super::rows_of(&painted, &voices),
                voice_rows,
                "the voice cards' rows at {width}pt"
            );
            assert_eq!(
                super::rows_of(&painted, &pair),
                pair_rows,
                "the first do and don't pair's rows at {width}pt"
            );
            let inner = egui::Spacing::default().item_spacing.x + spacing::SPACE_2;
            for (label, glyph) in [(pair[0], Icon::CheckCircle), (pair[1], Icon::Block)] {
                let gap = icon_to_label(&painted, glyph, label);
                assert!(
                    (gap - inner).abs() < 0.5,
                    "at {width}pt the {glyph:?} icon sits {gap}pt from {label:?}, not {inner}pt"
                );
            }
        }
    }

    /// Lay the five photo tiles out headless in `theme`, each at the
    /// size the showcase gives it (the lead tile wide, the other four
    /// small), and hand back every tile's rect beside the texts the
    /// layout painted.
    fn lay_out_photo_tiles(theme: ThemeId) -> (Vec<egui::Rect>, Vec<(String, egui::Rect)>) {
        let mut tiles = Vec::new();
        let (_, painted) = super::lay_out(PHOTO_TILE_WIDE.x, |ui| {
            tiles.clear();
            for (index, (ground, caption)) in PHOTO_TILES.iter().enumerate() {
                let size = if index == 0 {
                    PHOTO_TILE_WIDE
                } else {
                    PHOTO_TILE_SMALL
                };
                let tile = ui
                    .scope(|ui| super::photo_tile(ui, theme, *ground, caption, size))
                    .response
                    .rect;
                assert_eq!(tile.size(), size, "the scope is the tile and nothing else");
                tiles.push(tile);
            }
        });
        (tiles, painted)
    }

    /// The rect the painted text `text` covers; a text the layout never
    /// painted panics.
    fn painted_rect(painted: &[(String, egui::Rect)], text: &str) -> egui::Rect {
        painted
            .iter()
            .find(|(seen, _)| seen == text)
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| panic!("{text:?} was never painted"))
    }

    /// Every photo tile's caption stays inside its tile, at the size the
    /// showcase gives the tile and in every theme's face. The small
    /// 160pt tiles' captions are wider than the tile on one line; they
    /// wrap onto more lines instead of running on past its edge. Each
    /// caption is painted whole (a caption cut short would not be found
    /// under its own text), keeps the tile's left and bottom inset it
    /// always had, and the lines the wrap adds, which grow upward from
    /// that inset, stop short of the camera glyph above them.
    #[test]
    fn photo_captions_stay_inside_their_tiles() {
        let camera = Icon::PhotoCamera.as_char().to_string();
        let mut problems = Vec::new();
        for theme in ThemeId::ALL {
            let (tiles, painted) = lay_out_photo_tiles(*theme);
            for ((_, caption), tile) in PHOTO_TILES.iter().zip(&tiles) {
                let text = painted_rect(&painted, caption);
                if !tile.expand(0.01).contains_rect(text) {
                    problems.push(format!(
                        "{theme:?}: {caption:?} paints {text:?}, outside its {tile:?} tile"
                    ));
                }
                let (left, bottom) = (text.left() - tile.left(), tile.bottom() - text.bottom());
                if (left - spacing::SPACE_3).abs() > 0.01
                    || (bottom - spacing::SPACE_3).abs() > 0.01
                {
                    problems.push(format!(
                        "{theme:?}: {caption:?} sits {left}pt from the tile's left and {bottom}pt \
                         from its bottom, not {}pt",
                        spacing::SPACE_3
                    ));
                }
                let glyph = painted
                    .iter()
                    .filter(|(seen, _)| *seen == camera)
                    .map(|(_, rect)| *rect)
                    .find(|rect| tile.contains(rect.center()))
                    .unwrap_or_else(|| panic!("{theme:?}: no camera glyph in the {tile:?} tile"));
                if glyph.bottom() > text.top() + 0.01 {
                    problems.push(format!(
                        "{theme:?}: {caption:?} (top {}) runs up into the camera glyph (bottom {})",
                        text.top(),
                        glyph.bottom()
                    ));
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// A word wider than the tile is the one thing wrapping cannot fit:
    /// it is cut short with an ellipsis, the rest of the caption kept,
    /// and still nothing paints outside the tile.
    #[test]
    fn photo_caption_word_wider_than_the_tile_ends_in_an_ellipsis() {
        const WORD: &str = "Antidisestablishmentarianism";
        let caption = format!("{WORD} · Lundby");
        for theme in ThemeId::ALL {
            let mut tile = egui::Rect::NOTHING;
            let (_, painted) = super::lay_out(PHOTO_TILE_SMALL.x, |ui| {
                tile = ui
                    .scope(|ui| {
                        super::photo_tile(
                            ui,
                            *theme,
                            PhotoGround::SecondaryContainer,
                            &caption,
                            PHOTO_TILE_SMALL,
                        )
                    })
                    .response
                    .rect;
            });
            let cut: Vec<_> = painted
                .iter()
                .filter(|(seen, _)| seen.contains('\u{2026}'))
                .collect();
            assert_eq!(cut.len(), 1, "{theme:?}: one caption, one ellipsis");
            let (text, rect) = cut[0];
            assert!(
                text.starts_with("Anti") && text.ends_with("\u{2026} · Lundby"),
                "{theme:?}: only the over-wide word is cut, got {text:?}"
            );
            assert!(
                tile.expand(0.01).contains_rect(*rect),
                "{theme:?}: {text:?} paints {rect:?}, outside its {tile:?} tile"
            );
        }
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

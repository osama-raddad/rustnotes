//! Cover: the brand-book cover composition, `components/Cover`'s
//! preview adapted as the gallery's opening screen (issue 10).
//!
//! The preview is a 960-unit card: a top band of unequal blocks bled
//! off the top edge, weighted like the 60/30/10 rule — `primary` leads
//! at 416 units, `primary-container` (the relief) takes 224, `on-surface`
//! 160, and `tertiary`, the guest at the edge, 96 — with three
//! surface-coloured hairlines cut across so the band reads as ruled
//! stock (the editorial-serif row). Below sits the name in the display
//! face at 64px in `primary`, and the tag in the body face at 14px in
//! `text-muted`. Corners at `radius-lg`, gutters `space-4`, rule pitch
//! `space-6`.
//!
//! The adaptation (documented judgment): the words gain the eyebrow the
//! issue's acceptance names, and the V mark at its brand-placement
//! floor of 96px stands above it, so the composition reads mark,
//! eyebrow, display headline, standfirst. The canvas leads (the band
//! blocks are the only fills), the accent stays a guest (the tertiary
//! block is the narrowest, 10% of the grid), and the headline clears
//! the README's 2.5x pairing floor over the standfirst through
//! [`crate::typography::pairing_holds`]. The roles re-skin per theme:
//! Forest, mint, Charcoal, Bronze; Deep Teal, teal tint, Ink,
//! Champagne; Charcoal, warm grey, Charcoal, Bronze.
//!
//! The copy lives in consts so the voice lint (issue 11) can scan it.

use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::logo::{Logo, MARK_MIN_BRAND};
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale, ThemeColors};
use crate::typography::{specimen_job, tracked_text, TrackedLabel, TypeStyle};

/// The display headline's size: headline 1, the preview's 64px `.name`.
pub const HEADLINE_SIZE: f32 = 64.0;

/// The standfirst's size: the preview's 14px `.tag` (body-sm).
pub const STANDFIRST_SIZE: f32 = type_scale::TEXT_BODY_SM;

/// The visible band height in px (the blocks bleed 16px above it).
pub const BAND_HEIGHT: f32 = 96.0;

/// How far the blocks bleed past the band's top, px.
pub const BAND_BLEED: f32 = 16.0;

/// The 960-unit grid the preview's band is drawn on: the denominator
/// of each block's share (the guest's 96 units are the 10% the ratio
/// rule gives the accent). The laid-out band spans the 944 units its
/// blocks and gutters take, scaled to the column ([`band_layout`]).
pub const BAND_UNITS: f32 = 960.0;

/// The band's left margin, in grid units: none. The preview's 32-unit
/// margin was its page padding, and the page's own gutter now supplies
/// that edge, so the lead block starts on the text's left edge rather
/// than indented beside it.
pub const BAND_MARGIN_UNITS: f32 = 0.0;

/// The gutter between blocks, in grid units (`space-4`).
pub const BAND_GUTTER_UNITS: f32 = 16.0;

/// The pitch of the three hairlines cut across the band (`space-6`).
pub const BAND_RULE_PITCH: f32 = 24.0;

/// The V mark's height: the brand-placement floor.
pub const COVER_MARK_HEIGHT: f32 = MARK_MIN_BRAND;

/// Which role one band block fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandRole {
    /// The lead block: `primary`.
    Primary,
    /// The relief: `primary-container`.
    PrimaryContainer,
    /// The ink block: `on-surface`.
    OnSurface,
    /// The guest at the edge: `tertiary`, the accent.
    Tertiary,
}

impl BandRole {
    /// The role's value in a theme.
    pub const fn resolve(self, colors: &ThemeColors) -> Color32 {
        match self {
            BandRole::Primary => colors.primary,
            BandRole::PrimaryContainer => colors.primary_container,
            BandRole::OnSurface => colors.on_surface,
            BandRole::Tertiary => colors.tertiary,
        }
    }
}

/// The band's blocks: width in grid units and the role each fills, in
/// the preview's arrangement order.
pub const BAND_BLOCKS: &[(f32, BandRole)] = &[
    (416.0, BandRole::Primary),
    (224.0, BandRole::PrimaryContainer),
    (160.0, BandRole::OnSurface),
    (96.0, BandRole::Tertiary),
];

/// The band's blocks laid out over `width`: each block's x offset and
/// width in px. The blocks and the gutters between them (the margin,
/// none, in front) span the band's grid extent, 944 units, and that
/// extent is scaled to the whole `width`, so the lead block starts on
/// the column's left edge and the guest ends on its right edge. The
/// blocks keep the 960-unit shares of [`BAND_BLOCKS`] against one
/// another and against the gutters. Pure geometry, so the proportions
/// test without a context.
pub fn band_layout(width: f32) -> Vec<(f32, f32)> {
    // The last block's right edge in grid units, which the width fills.
    let extent = BAND_MARGIN_UNITS
        + BAND_BLOCKS.iter().map(|(units, _)| units).sum::<f32>()
        + BAND_GUTTER_UNITS * (BAND_BLOCKS.len() - 1) as f32;
    let scale = width / extent;
    let mut x = BAND_MARGIN_UNITS;
    let mut out = Vec::with_capacity(BAND_BLOCKS.len());
    for (units, _) in BAND_BLOCKS {
        out.push((x * scale, units * scale));
        x += units + BAND_GUTTER_UNITS;
    }
    out
}

/// The eyebrow line, metadata separated by middle dots.
pub const EYEBROW: &str = "Design system · Gothenburg · 2026";

/// The display headline, the preview's `.name`.
pub const HEADLINE: &str = "Vynatix Design System";

/// The standfirst, the preview's `.tag` (the README's blockquote).
pub const STANDFIRST: &str = "The discipline is the luxury. The tokens are the discipline.";

/// The caption under the composition.
pub const CAPTION: &str = "The brand-book cover: the canvas leads, the band is weighted like the 60/30/10 rule, and the accent block is a guest at the edge. The roles re-skin it in every theme.";

/// The cover: the gallery's opening composition, painted in the theme
/// it is given.
///
/// ```
/// use ui_kit::components::Cover;
/// use ui_kit::theme::ThemeId;
///
/// let cover = Cover::new(); // the brand-book composition
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Cover;

impl Cover {
    /// The cover composition.
    pub const fn new() -> Self {
        Self
    }

    /// Paint the cover: the band bled off the top, then the V mark at
    /// its brand-placement floor, the eyebrow, the display headline and
    /// the standfirst, closing on the pairing caption. Scaled to the
    /// available width; the blocks and rules clip where they leave the
    /// panel, the bleed the preview draws.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId) -> Response {
        let colors = theme.colors();
        let width = ui.available_width();

        // The band: a strip of unequal blocks bled off the top edge,
        // weighted like the 60/30/10 rule, the tertiary guest last and
        // narrowest. The painter clips above the panel, which is the
        // bleed the preview draws.
        let (band, _) = ui.allocate_exact_size(Vec2::new(width, BAND_HEIGHT), Sense::hover());
        let block_radius = CornerRadius::same(radii::RADIUS_LG as u8);
        for ((x, w), (_, role)) in band_layout(width).iter().zip(BAND_BLOCKS) {
            let rect = Rect::from_min_size(
                Pos2::new(band.left() + x, band.top() - BAND_BLEED),
                Vec2::new(*w, BAND_HEIGHT + BAND_BLEED),
            );
            ui.painter()
                .rect_filled(rect, block_radius, role.resolve(&colors));
        }
        // Three hairlines cut from the surface ground across the strip,
        // the editorial ruled-stock pattern.
        let margin = band_layout(width)[0].0;
        for line in 1..=3u32 {
            let y = band.top() + BAND_RULE_PITCH * line as f32 + 0.5;
            ui.painter().hline(
                (band.left() + margin)..=band.right(),
                y,
                Stroke::new(1.0, colors.surface),
            );
        }

        // The words: the mark at its brand-placement floor, then the
        // eyebrow, the display headline and the standfirst. The canvas
        // carries it; the headline clears the 2.5x pairing floor over
        // the standfirst beside it.
        ui.add_space(spacing::SPACE_16);
        ui.add(Logo::mark().height(COVER_MARK_HEIGHT).color(colors.primary));
        ui.add_space(spacing::SPACE_4);
        ui.add(TrackedLabel::new(
            EYEBROW,
            TypeStyle::Eyebrow.font_id(theme),
            TypeStyle::Eyebrow.tracking(),
            colors.text_accent,
        ));
        ui.add_space(spacing::SPACE_2);
        ui.add(egui::Label::new(specimen_job(
            HEADLINE,
            TypeStyle::Headline1,
            theme,
            colors.primary,
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_2);
        ui.add(egui::Label::new(tracked_text(
            STANDFIRST,
            FontId {
                size: STANDFIRST_SIZE,
                family: TypeStyle::Body.family(theme),
            },
            0.0,
            Some(20.0),
            colors.text_muted(),
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_4);
        let ratio = HEADLINE_SIZE / STANDFIRST_SIZE;
        ui.add(egui::Label::new(specimen_job(
            &format!(
                "{CAPTION} Headline {:.0} over standfirst {:.0}: {:.1}x the 2.5x floor.",
                HEADLINE_SIZE, STANDFIRST_SIZE, ratio,
            ),
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(ui.available_width()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::components::brand_showcase::copy_passes_voice_checks;
    use egui::Rect;

    use crate::theme::ThemeId;
    use crate::typography::{pairing_holds, TypeStyle};

    use super::{
        band_layout, BandRole, BAND_BLEED, BAND_BLOCKS, BAND_HEIGHT, BAND_MARGIN_UNITS, BAND_UNITS,
        CAPTION, COVER_MARK_HEIGHT, EYEBROW, HEADLINE, HEADLINE_SIZE, STANDFIRST, STANDFIRST_SIZE,
    };

    /// The composition pairs its display headline with its standfirst
    /// over the README's 2.5x floor, through the same helper the
    /// specimen's own pairing goes through: 64 over 14, the preview's
    /// sizes.
    #[test]
    fn headline_pairs_with_the_standfirst_over_the_floor() {
        assert_eq!(HEADLINE_SIZE, TypeStyle::Headline1.size());
        assert_eq!(STANDFIRST_SIZE, 14.0);
        assert!(pairing_holds(HEADLINE_SIZE, STANDFIRST_SIZE));
        let ratio = HEADLINE_SIZE / STANDFIRST_SIZE;
        assert!(
            (2.5..5.0).contains(&ratio),
            "clears the floor without shouting it"
        );
    }

    /// The band is the preview's arrangement: four blocks whose shares
    /// of the 960-unit grid put primary in the lead, the tertiary guest
    /// last and narrowest at the 10% the ratio rule gives the whole
    /// accent, the blocks together about a third of the grid.
    #[test]
    fn band_weights_primary_and_keeps_the_accent_a_guest() {
        assert_eq!(
            BAND_BLOCKS.len(),
            4,
            "four blocks: lead, relief, ink, guest"
        );
        let shares: Vec<f32> = BAND_BLOCKS
            .iter()
            .map(|(units, _)| units / BAND_UNITS)
            .collect();
        assert_eq!(
            BAND_BLOCKS[0].1,
            BandRole::Primary,
            "primary leads the band"
        );
        assert_eq!(
            BAND_BLOCKS[3].1,
            BandRole::Tertiary,
            "the guest sits at the edge"
        );
        let narrowest = shares.iter().copied().fold(f32::INFINITY, f32::min);
        assert_eq!(shares[3], narrowest, "the guest is the narrowest block");
        assert!(
            shares[3] <= 0.10 + 1e-6,
            "the accent keeps to its 10% share"
        );
        assert!(
            shares.iter().sum::<f32>() < 0.96,
            "the band is a strip, not a wall"
        );
    }

    /// The band layout scales the grid to any width, gutters riding
    /// along, and the last block ends on the width itself. The blocks
    /// and gutters take 944 units (416 + 224 + 160 + 96 and three
    /// gutters of 16), so at 472px the scale is one half: the lead
    /// block starts at 0 and runs 208px wide, the second starts a
    /// gutter later at 216px, and the guest ends at 472px. The band
    /// once ended at 944 of the 960 units, short of the column.
    #[test]
    fn band_layout_scales_the_grid() {
        let layout = band_layout(472.0);
        assert_eq!(layout.len(), BAND_BLOCKS.len());
        assert!(
            (layout[0].0 - BAND_MARGIN_UNITS / 2.0).abs() < 1e-4,
            "the margin scales"
        );
        assert!(
            (layout[0].1 - 416.0 / 2.0).abs() < 1e-4,
            "lead width scales"
        );
        assert!((layout[1].0 - 432.0 / 2.0).abs() < 1e-4, "gutters scale");
        for width in [472.0, 320.5, 361.7, 960.0, 1280.0] {
            let layout = band_layout(width);
            let end = layout[3].0 + layout[3].1;
            assert!(
                (end - width).abs() < 1e-3,
                "at {width}px the guest ends at {end}px, the column's right edge"
            );
        }
    }

    /// The band's visible height carries the preview's bleed, the
    /// hairlines pitch at `space-6`, and the mark sits at the
    /// brand-placement floor the logo rules set.
    #[test]
    fn band_height_and_mark_floor_match_the_rules() {
        assert_eq!(BAND_HEIGHT, 96.0);
        assert_eq!(BAND_BLEED, 16.0);
        assert_eq!(BAND_HEIGHT + BAND_BLEED, 112.0, "the blocks stand 112 tall");
        assert_eq!(super::BAND_RULE_PITCH, 24.0, "three rules, space-6 apart");
        assert_eq!(COVER_MARK_HEIGHT, 96.0, "the V mark at the brand floor");
    }

    /// The copy passes the voice checks: no em dashes, no ellipses, no
    /// exclamation marks, no emoji, middle dots as the separator. The
    /// shape of issue 11's gallery-wide lint.
    #[test]
    fn copy_passes_the_voice_checks() {
        for text in [EYEBROW, HEADLINE, STANDFIRST, CAPTION] {
            assert!(copy_passes_voice_checks(text), "{text}");
        }
        assert!(EYEBROW.contains(" · "), "metadata separated by middle dots");
    }

    /// The cover paints headless in every theme: fonts installed in a
    /// warm-up pass, then one pass per theme without panicking, the
    /// composition sized to the dummy window.
    #[test]
    fn cover_paints_headless_in_every_theme() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            for theme in ThemeId::ALL {
                crate::typography::install(*theme, ui.ctx());
            }
        });
        out.drop_without_applying_deltas();
        for theme in ThemeId::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                super::Cover::new().ui(ui, *theme);
            });
            out.drop_without_applying_deltas();
        }
    }

    /// What the cover really paints into a page's content column near
    /// `column`, in `theme`: the column the layout actually gave it (egui
    /// snaps a requested rect to its own grid, so the request is not the
    /// column), and every shape the pass drew, groups walked into, in
    /// paint order. The fonts bind at a pass boundary, so a warm-up pass
    /// installs them first.
    fn painted_cover(theme: ThemeId, column: Rect) -> (Rect, Vec<egui::Shape>) {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let (laid_out, shapes) = crate::typography::paint_pass(&ctx, |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                let column = ui.max_rect();
                super::Cover::new().ui(ui, theme);
                column
            })
            .inner
        });
        fn walk(shape: &egui::Shape, found: &mut Vec<egui::Shape>) {
            match shape {
                egui::Shape::Vec(nested) => nested.iter().for_each(|shape| walk(shape, found)),
                other => found.push(other.clone()),
            }
        }
        let mut found = Vec::new();
        for clipped in &shapes {
            walk(&clipped.shape, &mut found);
        }
        (laid_out, found)
    }

    /// The band as painted: four blocks, each one filled rect, in the
    /// band's roles, with the same gutter between neighbours, the lead
    /// block on the column's left edge and the guest block on its right
    /// edge. The band once ended at 944 of 960 grid units, so it stopped
    /// short of the content column's right edge by 16 units: 6 pt on a
    /// phone. The columns start off the whole point and are fractional
    /// wide, as a phone's are, since that is where an edge that was
    /// computed rather than placed would show.
    #[test]
    fn the_painted_band_fills_the_column_in_four_whole_blocks() {
        let columns = [
            Rect::from_min_size(egui::pos2(20.3, 30.0), egui::vec2(361.7, 700.0)),
            Rect::from_min_size(egui::pos2(16.0, 0.0), egui::vec2(320.5, 700.0)),
            Rect::from_min_size(egui::pos2(0.25, 12.5), egui::vec2(960.0, 700.0)),
        ];
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            for requested in columns {
                let (column, painted) = painted_cover(*theme, requested);
                let name = format!("{}, column {column:?}", theme.name());

                // The band's blocks are the rects that stand BAND_HEIGHT
                // plus the bleed tall with their top at the bleed; the
                // role colours alone cannot pick them out, since a theme
                // may give two roles one colour.
                let mut blocks: Vec<&egui::epaint::RectShape> = painted
                    .iter()
                    .filter_map(|shape| match shape {
                        egui::Shape::Rect(rect)
                            if rect.rect.top() == column.top() - BAND_BLEED
                                && rect.rect.height() == BAND_HEIGHT + BAND_BLEED =>
                        {
                            Some(rect)
                        }
                        _ => None,
                    })
                    .collect();
                blocks.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
                assert_eq!(
                    blocks.len(),
                    BAND_BLOCKS.len(),
                    "{name}: one filled rect per block, got {blocks:?}"
                );
                for (block, (_, role)) in blocks.iter().zip(BAND_BLOCKS) {
                    assert_eq!(
                        block.fill,
                        role.resolve(&colors),
                        "{name}: the {role:?} block's fill"
                    );
                }

                // Edge to edge: the lead on the column's left, the guest
                // on its right. The layout snaps the band's origin to
                // egui's 1/32 pt grid, so the edges agree to that step
                // and no further.
                let snap = egui::emath::GUI_ROUNDING;
                let first = blocks[0].rect;
                let last = blocks[blocks.len() - 1].rect;
                assert!(
                    (first.left() - column.left()).abs() <= snap,
                    "{name}: the band starts at {}, the column at {}",
                    first.left(),
                    column.left()
                );
                assert!(
                    (last.right() - column.right()).abs() <= snap,
                    "{name}: the band ends at {}, the column at {}",
                    last.right(),
                    column.right()
                );

                // The gutters: the same gap between every pair of
                // neighbours, open, never overlapping.
                let gutters: Vec<f32> = blocks
                    .windows(2)
                    .map(|pair| pair[1].rect.left() - pair[0].rect.right())
                    .collect();
                for gutter in &gutters {
                    assert!(*gutter > 0.0, "{name}: blocks overlap, gutters {gutters:?}");
                    assert!(
                        (gutter - gutters[0]).abs() < 1e-3,
                        "{name}: uneven gutters {gutters:?}"
                    );
                }

                // The proportions: each block's width keeps its share of
                // the lead block's.
                for (block, (units, _)) in blocks.iter().zip(BAND_BLOCKS) {
                    let want = blocks[0].rect.width() * units / BAND_BLOCKS[0].0;
                    assert!(
                        (block.rect.width() - want).abs() < 1e-2,
                        "{name}: a {units}-unit block is {} wide, wanted {want}",
                        block.rect.width()
                    );
                }
            }
        }
    }
}

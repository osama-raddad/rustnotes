//! The Vynatix icon runtime: the four instanced Material Symbols
//! subsets, their families, and the one-glyph widget that paints them.
//!
//! The data lives in the generated [`crate::icons`] module (the
//! catalogue enum, the PUA codepoints, the fill metadata); this module
//! carries everything that runs: the embedded faces under
//! `assets/fonts/` (instanced and subsetted by `tools/prepare_icons.py`
//! from the design system's variable `MaterialSymbolsOutlined.woff2`),
//! their additive installation beside the type faces, the grade and
//! fill selection, and [`icon`] / [`icon_active`], the widgets.
//!
//! Families. Four faces, one per (grade, fill) combination the design
//! system uses: `Vynatix Icons` (GRAD 0, FILL 0), `Vynatix Icons Dark`
//! (GRAD −25, FILL 0), `Vynatix Icons Fill` (FILL 1, GRAD 0) and
//! `Vynatix Icons Fill Dark` (FILL 1, GRAD −25), every one instanced at
//! wght 300, the brand's single icon weight. Grade is the dark-theme
//! correction (−25 thins light-on-dark glyphs so they don't bloom) and
//! fill is the active state (the selected navigation item, a toggled
//! control): [`family`] picks the face from both, and
//! [`family_for_theme`] resolves the grade straight from a
//! [`ThemeId`]. All four install together through [`install`], which
//! merges by family name like the type install and never displaces it.
//!
//! Painting. An icon is one private-use character laid out in the
//! selected family at the token size, centred in a square allocation.
//! [`icon`] draws the rest outline; [`icon_active`] draws the filled
//! glyph for the icons whose catalogue entry carries one. The glyph
//! takes the colour it is handed: primary for icons that act, the text
//! colours beside their words (the routing rules land with the
//! components, issue 05).
//!
//! Documented deviations, all pinned in the generator: the optical
//! size axis is pinned at opsz 24 in every subset (egui has no
//! variation-axis control, so the per-size opsz the tokens describe, 20
//! through 48, ships as the single 24 instance); `location_on` has no
//! glyph in font 0.47.5, so the generated enum aliases it to `push_pin`
//!'s codepoint (the catalogue's pin), documented on the variant; and
//! the ligature-based web usage does not apply, the codepoints are
//! addressed directly.

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{Color32, Context, FontData, FontFamily, FontId, Response, Ui};

use crate::icons::Icon;
use crate::theme::ThemeId;
use crate::tokens::icons as tokens;
use crate::typography::Face;

// ---- the embedded faces ---------------------------------------------------

/// Material Symbols Outlined instanced at wght 300, opsz 24, GRAD 0,
/// FILL 0: the rest face for the light themes.
const OUTLINE: Face = Face {
    family: "Vynatix Icons",
    file: "MaterialSymbols-Outline.ttf",
    bytes: include_bytes!("../assets/fonts/MaterialSymbols-Outline.ttf"),
};

/// GRAD −25, FILL 0: the rest face for the dark themes.
const OUTLINE_DARK: Face = Face {
    family: "Vynatix Icons Dark",
    file: "MaterialSymbols-OutlineDark.ttf",
    bytes: include_bytes!("../assets/fonts/MaterialSymbols-OutlineDark.ttf"),
};

/// FILL 1, GRAD 0: the active face for the light themes.
const FILL: Face = Face {
    family: "Vynatix Icons Fill",
    file: "MaterialSymbols-Fill.ttf",
    bytes: include_bytes!("../assets/fonts/MaterialSymbols-Fill.ttf"),
};

/// FILL 1, GRAD −25: the active face for the dark themes.
const FILL_DARK: Face = Face {
    family: "Vynatix Icons Fill Dark",
    file: "MaterialSymbols-FillDark.ttf",
    bytes: include_bytes!("../assets/fonts/MaterialSymbols-FillDark.ttf"),
};

/// The four icon faces, grade-major, fill-minor.
pub const FACES: &[Face] = &[OUTLINE, OUTLINE_DARK, FILL, FILL_DARK];

/// Install the four icon families into a context. Additive by design,
/// the same [`Context::add_font`] merge the type install uses: the
/// icon families and the text families coexist whichever installs
/// first, and a second call changes nothing.
pub fn install(ctx: &Context) {
    for face in FACES {
        ctx.add_font(FontInsert::new(
            face.family,
            FontData::from_static(face.bytes),
            vec![InsertFontFamily {
                family: FontFamily::Name(face.family.into()),
                priority: FontPriority::Highest,
            }],
        ));
    }
}

/// The rest face for the theme's grade: GRAD 0 on light canvases,
/// GRAD −25 on dark ones.
pub fn icon_font(dark: bool) -> &'static Face {
    if dark {
        &OUTLINE_DARK
    } else {
        &OUTLINE
    }
}

/// The active (FILL 1) face for the theme's grade.
pub fn fill_font(dark: bool) -> &'static Face {
    if dark {
        &FILL_DARK
    } else {
        &FILL
    }
}

/// The family name for the grade and fill: one of the four installed
/// icon families.
pub fn family_name(dark: bool, filled: bool) -> &'static str {
    match (dark, filled) {
        (false, false) => OUTLINE.family,
        (true, false) => OUTLINE_DARK.family,
        (false, true) => FILL.family,
        (true, true) => FILL_DARK.family,
    }
}

/// The family for the grade and fill, as a [`FontFamily::Name`].
pub fn family(dark: bool, filled: bool) -> FontFamily {
    FontFamily::Name(family_name(dark, filled).into())
}

/// The icon family for a theme and state: dark themes take the −25
/// grade automatically, active states the filled face.
pub fn family_for_theme(theme: ThemeId, filled: bool) -> FontFamily {
    family(theme.is_dark(), filled)
}

// ---- the token sizes ------------------------------------------------------

/// One of the five icon sizes from the `icon` tokens. The only sizes
/// the widget accepts: construct a variant (or [`try_from_px`], which
/// returns `None` for any other value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSize {
    /// `icon-size-sm`: 16px, inline with dense text.
    Sm,
    /// `icon-size-md`: 20px, dense UI.
    Md,
    /// `icon-size-lg`: 24px, the default.
    Lg,
    /// `icon-size-xl`: 32px, feature lists and empty states.
    Xl,
    /// `icon-size-2xl`: 48px, hero moments.
    Xxl,
}

impl IconSize {
    /// Every size, token order.
    pub const ALL: &[IconSize] = &[
        IconSize::Sm,
        IconSize::Md,
        IconSize::Lg,
        IconSize::Xl,
        IconSize::Xxl,
    ];

    /// The size in px.
    pub const fn px(self) -> f32 {
        match self {
            IconSize::Sm => tokens::ICON_SIZE_SM,
            IconSize::Md => tokens::ICON_SIZE_MD,
            IconSize::Lg => tokens::ICON_SIZE_LG,
            IconSize::Xl => tokens::ICON_SIZE_XL,
            IconSize::Xxl => tokens::ICON_SIZE_2XL,
        }
    }

    /// The `tokens.json` name.
    pub const fn name(self) -> &'static str {
        match self {
            IconSize::Sm => "icon-size-sm",
            IconSize::Md => "icon-size-md",
            IconSize::Lg => "icon-size-lg",
            IconSize::Xl => "icon-size-xl",
            IconSize::Xxl => "icon-size-2xl",
        }
    }

    /// The size for a px value, only when it is one of the tokens.
    pub fn try_from_px(px: f32) -> Option<Self> {
        IconSize::ALL.iter().copied().find(|size| size.px() == px)
    }
}

// ---- the widget -----------------------------------------------------------

/// Paint `icon` at `size` in `color`: the rest outline, one private-use
/// glyph from the grade the theme's darkness picks, centred in a square
/// allocation of the token size.
pub fn icon(ui: &mut Ui, icon: Icon, size: IconSize, color: Color32) -> Response {
    icon_variant(ui, icon, size, color, false)
}

/// Paint `icon` in its active form: the filled glyph, for the icons
/// whose catalogue entry carries a `-fill.svg`. Icons without one draw
/// their outline (the font's FILL axis only shifts the glyphs that have
/// a filled cut).
pub fn icon_active(ui: &mut Ui, icon: Icon, size: IconSize, color: Color32) -> Response {
    icon_variant(ui, icon, size, color, true)
}

/// The shared painter behind [`icon`] and [`icon_active`].
fn icon_variant(ui: &mut Ui, icon: Icon, size: IconSize, color: Color32, filled: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::Vec2::splat(size.px()), egui::Sense::hover());
    let dark = ui.visuals().dark_mode;
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.as_char().to_string(),
        FontId {
            size: size.px(),
            family: family(dark, filled),
        },
        color,
    );
    response
}

#[cfg(test)]
mod tests {
    use egui::FontFamily;

    use crate::icons::{Icon, CATALOGUE_COUNT, CATALOGUE_NAMES};
    use crate::theme::ThemeId;
    use crate::tokens::icons as tokens;

    use super::{family_name, icon, icon_active, icon_font, install, IconSize, FACES};

    /// The enum covers the catalogue exactly: one variant per name, the
    /// names in `catalogue.json` order, no duplicates, no strays. The
    /// count const pins the design system's number so a catalogue
    /// change forces a regeneration.
    #[test]
    fn enum_covers_exactly_the_catalogue() {
        assert_eq!(CATALOGUE_COUNT, 162, "the design system's 162 icons");
        let names: Vec<&str> = Icon::ALL.iter().map(|icon| icon.name()).collect();
        assert_eq!(names, CATALOGUE_NAMES, "variant names in catalogue order");
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "no duplicate variants");
        assert_eq!(Icon::ALL.len(), CATALOGUE_COUNT);
    }

    /// Every icon's character sits in the Unicode private-use area the
    /// subsetted fonts address, and the samples round-trip against the
    /// source font's cmap: home at U+E9B2, search at U+EF7A, and the
    /// documented `location_on` alias borrowing `push_pin`'s codepoint
    /// (no glyph for it exists in font 0.47.5).
    #[test]
    fn codepoints_are_private_use_and_round_trip() {
        for icon in Icon::ALL {
            let c = icon.as_char();
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&c),
                "{}: {c:?} outside the private-use area",
                icon.name()
            );
        }
        assert_eq!(Icon::Home.as_char(), '\u{e9b2}');
        assert_eq!(Icon::Search.as_char(), '\u{ef7a}');
        assert_eq!(
            Icon::LocationOn.as_char(),
            Icon::PushPin.as_char(),
            "location_on aliases push_pin's codepoint"
        );
    }

    /// The size enum carries exactly the five icon tokens and nothing
    /// else: `try_from_px` resolves each token value and rejects every
    /// non-token, including the neighbouring and out-of-range values.
    #[test]
    fn sizes_come_only_from_the_icon_tokens() {
        let expected = [
            ("icon-size-sm", tokens::ICON_SIZE_SM),
            ("icon-size-md", tokens::ICON_SIZE_MD),
            ("icon-size-lg", tokens::ICON_SIZE_LG),
            ("icon-size-xl", tokens::ICON_SIZE_XL),
            ("icon-size-2xl", tokens::ICON_SIZE_2XL),
        ];
        assert_eq!(IconSize::ALL.len(), expected.len());
        for (size, (name, px)) in IconSize::ALL.iter().zip(expected) {
            assert_eq!(size.px(), px, "{name}");
            assert_eq!(size.name(), name);
            assert_eq!(IconSize::try_from_px(px), Some(*size), "{name} resolves");
        }
        for rejected in [15.0, 17.0, 24.5, 0.0, -4.0, 64.0, 96.0] {
            assert_eq!(
                IconSize::try_from_px(rejected),
                None,
                "{rejected}px is not an icon token"
            );
        }
    }

    /// Four faces cover the grade x fill grid: light and dark grades at
    /// rest and active, unique family names, real embedded bytes from
    /// their prepared files.
    #[test]
    fn four_faces_cover_grade_and_fill() {
        assert_eq!(FACES.len(), 4, "grade-major, fill-minor");
        let mut families: Vec<&str> = Vec::new();
        for face in FACES {
            assert!(!face.bytes.is_empty(), "{} embeds bytes", face.file);
            families.push(face.family);
        }
        families.sort_unstable();
        families.dedup();
        assert_eq!(families.len(), 4, "one family per variant");
        assert_eq!(family_name(false, false), "Vynatix Icons");
        assert_eq!(family_name(true, false), "Vynatix Icons Dark");
        assert_eq!(family_name(false, true), "Vynatix Icons Fill");
        assert_eq!(family_name(true, true), "Vynatix Icons Fill Dark");
        assert_eq!(icon_font(false).family, "Vynatix Icons");
        assert_eq!(icon_font(true).family, "Vynatix Icons Dark");
    }

    /// Dark themes pick the −25 grade straight from their id, at rest
    /// and active; light themes the 0 grade.
    #[test]
    fn dark_themes_pick_the_neg25_grade() {
        for theme in ThemeId::ALL {
            let expected = super::family_name(theme.is_dark(), false);
            assert_eq!(
                super::family_for_theme(*theme, false),
                FontFamily::Name(expected.into()),
                "{}: rest grade",
                theme.name()
            );
            let expected = super::family_name(theme.is_dark(), true);
            assert_eq!(
                super::family_for_theme(*theme, true),
                FontFamily::Name(expected.into()),
                "{}: active grade",
                theme.name()
            );
        }
    }

    /// `install` registers the four families on a live context and is
    /// additive with the type install: every icon family and every
    /// specimen family stays listed whichever order they install in.
    #[test]
    fn install_registers_the_four_icon_families() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let families = ui.ctx().fonts(|fonts| fonts.families());
            for face in FACES {
                let family = FontFamily::Name(face.family.into());
                assert!(families.contains(&family), "{} installed", face.family);
            }
            for style in crate::typography::TypeStyle::ALL {
                let family = style.family(ThemeId::Light);
                assert!(
                    families.contains(&family),
                    "{} survives the icon install",
                    style.family_name(ThemeId::Light)
                );
            }
        });
        out.drop_without_applying_deltas();
    }

    /// The widget lays out as a square of the token size and paints its
    /// glyph (headless: the fonts install in one pass, the painting
    /// happens in the next, because `add_font` binds families at the
    /// pass boundary), at rest and active.
    #[test]
    fn icon_paints_a_square_of_the_token_size() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            install(ui.ctx());
        });
        out.drop_without_applying_deltas();
        for theme in ThemeId::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                theme.apply(ui.ctx());
                for size in IconSize::ALL {
                    let response = icon(ui, Icon::Search, *size, egui::Color32::BLACK);
                    assert_eq!(
                        response.rect.size(),
                        egui::Vec2::splat(size.px()),
                        "{}: {} at rest",
                        theme.name(),
                        size.name()
                    );
                    let response = icon_active(ui, Icon::Home, *size, egui::Color32::BLACK);
                    assert_eq!(
                        response.rect.size(),
                        egui::Vec2::splat(size.px()),
                        "{}: {} active",
                        theme.name(),
                        size.name()
                    );
                }
            });
            out.drop_without_applying_deltas();
        }
    }
}

//! The Vynatix type system: the eleven specimen styles, the per-theme
//! face mapping, and the self-hosted fonts they install.
//!
//! The style table carries `tokens.json` `type.groups` (sizes, leadings,
//! trackings, nominal weights). egui has no weight axis and no
//! letter-spacing: each weight is its own named family (the faces under
//! `assets/fonts/`, instanced by `tools/prepare_fonts.py` and embedded
//! with `include_bytes!`), and tracking is painted where it matters.
//!
//! Letter spacing. egui 0.36 carries letter spacing in
//! [`egui::text::TextFormat::extra_letter_spacing`] (points, applied at
//! cluster boundaries by the shaper, kerning kept within clusters), so
//! the tracking travels with the text instead of a per-glyph painter:
//! [`specimen_job`] sets every style's tracking (the eyebrow's +0.24em
//! signature and the headlines' negative tracks alike) and the tokens'
//! line heights, and [`TrackedLabel`] is the eyebrow widget on top of
//! it, uppercasing its text as the style specifies.
//! [`tracked_text_width`] is the advance model the spacing follows:
//! glyph widths plus one track between each pair of glyphs, none after
//! the last, exactly as the shaper spaces the clusters — not CSS
//! letter-spacing, which adds a trailing track too.
//!
//! Installing. [`install`] registers the theme's faces through
//! [`Context::add_font`], which merges by name into the live font
//! definitions instead of replacing them: families other modules install
//! later (the icon font, issue 03) survive, our families survive theirs,
//! and re-installing a theme is a no-op upstream.
//!
//! Documented deviations, all pinned here and in the script: headline 2's
//! opsz 96 reuses the display's 144 instance (one optical size fewer,
//! imperceptible at 48px); the lede's weight 350 ships as the Regular 400
//! face (no 350 static exists); Teal's display styles name weight 500 but
//! set Instrument Serif, which ships one weight (the Teal book turns
//! synthesis off, so no faux bold); the eyebrow's leading is the tokens'
//! absolute 16px over its 11px size; and the two component styles that
//! name weight 700 (the metric figure, the editorial headline) resolve
//! through [`display_bold_family`], where Forest instanced a dedicated
//! Fraunces 700 while Teal keeps Instrument Serif's single weight and
//! Monochrome's Geist takes its 600 instance.

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{Color32, Context, FontData, FontFamily, FontId, Response, Ui, Widget};

use crate::theme::ThemeId;

// ---- the embedded faces ---------------------------------------------------

/// One self-hosted face: the named family it installs as, the prepared
/// file it comes from (`assets/fonts/`), and the file's bytes.
pub struct Face {
    /// The family name the face installs as, and the name the type
    /// styles reference. One weight (and style) per name: egui has no
    /// weight axis.
    pub family: &'static str,
    /// The prepared file the bytes come from, for tracing a face back to
    /// `tools/prepare_fonts.py`'s output.
    pub file: &'static str,
    /// The embedded TTF.
    pub bytes: &'static [u8],
}

/// Fraunces at the display coordinates: wght 500, opsz 144, SOFT 30.
const FRAUNCES_DISPLAY: Face = Face {
    family: "Fraunces Display",
    file: "Fraunces-Display.ttf",
    bytes: include_bytes!("../assets/fonts/Fraunces-Display.ttf"),
};

/// Fraunces for headline 3: wght 600, opsz 36, SOFT 0.
const FRAUNCES_HEADLINE: Face = Face {
    family: "Fraunces Headline",
    file: "Fraunces-Headline.ttf",
    bytes: include_bytes!("../assets/fonts/Fraunces-Headline.ttf"),
};

/// Fraunces at the metric figure's coordinates: wght 700, opsz 144,
/// SOFT 30. The one bold display instance the components need (issue
/// 08): the metric card's 56px figure and the editorial card's 24px
/// headline, both of which the specs set in the display family at
/// weight 700.
const FRAUNCES_METRIC: Face = Face {
    family: "Fraunces Metric",
    file: "Fraunces-Metric.ttf",
    bytes: include_bytes!("../assets/fonts/Fraunces-Metric.ttf"),
};

/// Fraunces italic for editorial pull-quotes: wght 400, opsz 22, SOFT 30.
const FRAUNCES_EDITORIAL: Face = Face {
    family: "Fraunces Editorial",
    file: "Fraunces-EditorialItalic.ttf",
    bytes: include_bytes!("../assets/fonts/Fraunces-EditorialItalic.ttf"),
};

/// Geist at 400: body, lede and caption in Teal and Monochrome.
const GEIST: Face = Face {
    family: "Geist",
    file: "Geist-Regular.ttf",
    bytes: include_bytes!("../assets/fonts/Geist-Regular.ttf"),
};

/// Geist at 500: Monochrome's display weight (standing in for Söhne).
const GEIST_MEDIUM: Face = Face {
    family: "Geist Medium",
    file: "Geist-Medium.ttf",
    bytes: include_bytes!("../assets/fonts/Geist-Medium.ttf"),
};

/// Geist at 600: eyebrows everywhere; H3 and H4 in Teal and Monochrome.
const GEIST_SEMI_BOLD: Face = Face {
    family: "Geist SemiBold",
    file: "Geist-SemiBold.ttf",
    bytes: include_bytes!("../assets/fonts/Geist-SemiBold.ttf"),
};

/// Aptos Regular: body, lede and caption in Forest.
const APTOS: Face = Face {
    family: "Aptos",
    file: "Aptos.ttf",
    bytes: include_bytes!("../assets/fonts/Aptos.ttf"),
};

/// Aptos SemiBold: headline 4 in Forest.
const APTOS_SEMI_BOLD: Face = Face {
    family: "Aptos SemiBold",
    file: "Aptos-SemiBold.ttf",
    bytes: include_bytes!("../assets/fonts/Aptos-SemiBold.ttf"),
};

/// Instrument Serif Regular: Teal's display face.
const INSTRUMENT: Face = Face {
    family: "Instrument Serif",
    file: "InstrumentSerif-Regular.ttf",
    bytes: include_bytes!("../assets/fonts/InstrumentSerif-Regular.ttf"),
};

/// Instrument Serif Italic: Teal's editorial face.
const INSTRUMENT_ITALIC: Face = Face {
    family: "Instrument Serif Italic",
    file: "InstrumentSerif-Italic.ttf",
    bytes: include_bytes!("../assets/fonts/InstrumentSerif-Italic.ttf"),
};

/// JetBrains Mono Regular: the mono style, every theme.
const JETBRAINS_MONO: Face = Face {
    family: "JetBrains Mono",
    file: "JetBrainsMono-Regular.ttf",
    bytes: include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
};

/// Forest Heritage's faces: Fraunces display and editorial, Aptos body,
/// Geist eyebrows, JetBrains Mono.
const FOREST_FACES: &[Face] = &[
    FRAUNCES_DISPLAY,
    FRAUNCES_HEADLINE,
    FRAUNCES_METRIC,
    FRAUNCES_EDITORIAL,
    APTOS,
    APTOS_SEMI_BOLD,
    GEIST_SEMI_BOLD,
    JETBRAINS_MONO,
];

/// Transformative Teal's faces: Instrument Serif display and editorial,
/// Geist for body and for H3 and smaller, JetBrains Mono.
const TEAL_FACES: &[Face] = &[
    INSTRUMENT,
    INSTRUMENT_ITALIC,
    GEIST,
    GEIST_SEMI_BOLD,
    JETBRAINS_MONO,
];

/// Stockholm Monochrome's faces: Geist display (standing in for Söhne)
/// and body, Fraunces editorial, JetBrains Mono.
const MONO_FACES: &[Face] = &[
    GEIST_MEDIUM,
    GEIST_SEMI_BOLD,
    GEIST,
    FRAUNCES_EDITORIAL,
    JETBRAINS_MONO,
];

/// Which face group a theme draws from: the README's per-theme faces
/// table has three rows, one per direction, each covering that
/// direction's light and dark theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceGroup {
    Forest,
    Teal,
    Mono,
}

impl ThemeId {
    /// The faces the theme's styles draw from, one entry per prepared
    /// file the theme needs.
    pub const fn faces(self) -> &'static [Face] {
        match self.face_group() {
            FaceGroup::Forest => FOREST_FACES,
            FaceGroup::Teal => TEAL_FACES,
            FaceGroup::Mono => MONO_FACES,
        }
    }

    const fn face_group(self) -> FaceGroup {
        match self {
            ThemeId::Light | ThemeId::Dark => FaceGroup::Forest,
            ThemeId::TealLight | ThemeId::TealDark => FaceGroup::Teal,
            ThemeId::MonoLight | ThemeId::MonoDark => FaceGroup::Mono,
        }
    }
}

/// Install the theme's families into a context. Additive by design:
/// [`Context::add_font`] merges each face by name into the live font
/// definitions and never replaces them, so anything installed before or
/// after (other themes' faces, the icon family from issue 03) survives,
/// and installing an already-installed theme changes nothing.
pub fn install(theme: ThemeId, ctx: &Context) {
    for face in theme.faces() {
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

// ---- the component display voices ----------------------------------------

/// The display voice in bold (the specs' weight 700) per theme, as a
/// named family. Two component styles set it: the metric card's 56px
/// figure and the editorial card's 24px headline, each of which pairs
/// the display family with body copy on the same surface.
///
/// Documented deviations, mirroring the specimen's: Forest instanced a
/// dedicated wght 700, opsz 144, SOFT 30 Fraunces ("Fraunces Metric",
/// added by issue 08); Teal's Instrument Serif ships one weight (the
/// Teal book turns synthesis off, so no faux bold); Monochrome's Geist
/// display uses its 600 instance, the nearest prepared Geist to the
/// spec's 700, since one component pair did not justify a fourth Geist
/// instance.
pub fn display_bold_family_name(theme: ThemeId) -> &'static str {
    match theme.face_group() {
        FaceGroup::Forest => FRAUNCES_METRIC.family,
        FaceGroup::Teal => INSTRUMENT.family,
        FaceGroup::Mono => GEIST_SEMI_BOLD.family,
    }
}

/// [`display_bold_family_name`] as a [`FontFamily::Name`].
pub fn display_bold_family(theme: ThemeId) -> FontFamily {
    FontFamily::Name(display_bold_family_name(theme).into())
}

/// The body face at weight 600, as a family name: the components'
/// label voice, the Button spec's `--font-body` at weight 600 and the
/// Badge's 12px at 600. The same per-theme mapping as headline 4,
/// whose specimen is the body face at 600: Aptos SemiBold in Forest,
/// Geist SemiBold in Teal and Monochrome.
pub fn body_semibold_family_name(theme: ThemeId) -> &'static str {
    TypeStyle::Headline4.family_name(theme)
}

/// [`body_semibold_family_name`] as a [`FontFamily::Name`].
pub fn body_semibold_family(theme: ThemeId) -> FontFamily {
    FontFamily::Name(body_semibold_family_name(theme).into())
}

/// The body face at its medium stand-in, as a family name: the
/// components' weight-500 voice — the Tabs' rest labels and the
/// TopAppBar's links, both `--font-body` at weight 500 in their specs.
/// Only Monochrome ships a medium instance of the body face (Geist
/// Medium), so Forest and Teal rest at the regular 400 and the
/// deviation lives in the components' docs; this resolver picks the
/// nearest installed instance per theme.
pub fn body_medium_family_name(theme: ThemeId) -> &'static str {
    match theme.face_group() {
        FaceGroup::Mono => GEIST_MEDIUM.family,
        FaceGroup::Forest => APTOS.family,
        FaceGroup::Teal => GEIST.family,
    }
}

/// [`body_medium_family_name`] as a [`FontFamily::Name`].
pub fn body_medium_family(theme: ThemeId) -> FontFamily {
    FontFamily::Name(body_medium_family_name(theme).into())
}

// ---- the type styles ------------------------------------------------------

/// One of the design system's eleven specimen styles, from
/// `tokens.json` `type.groups`. Sizes, leadings and trackings are the
/// groups' values; `weight` is the nominal weight (the shipped face may
/// be the nearest available, see the module docs' deviations).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeStyle {
    /// `type-display`: Fraunces 500 at opsz 144, 96/1.02, tracked −4%.
    /// Hero headlines.
    Display,
    /// `type-headline-1`: 64/1.08, tracked −3.5%.
    Headline1,
    /// `type-headline-2`: 48/1.12, tracked −3% (opsz 96 reuses the 144
    /// instance).
    Headline2,
    /// `type-headline-3`: 32/1.18, weight 600, tracked −2%.
    Headline3,
    /// `type-headline-4`: the body face at 600, 22/1.3, tracked −1%.
    Headline4,
    /// `type-lede`: 22/1.5 at the nominal 350 (ships as the Regular 400
    /// face).
    Lede,
    /// `type-body`: 17/1.65, the reading voice.
    Body,
    /// `type-editorial`: the display family's italic, 22/1.55, for
    /// pull-quotes.
    Editorial,
    /// `type-eyebrow`: Geist SemiBold 11/16, tracked +0.24em, set in
    /// uppercase in text-accent. The signature label.
    Eyebrow,
    /// `type-mono`: JetBrains Mono, 14/1.6.
    Mono,
    /// `type-caption`: 12/1.5, tracked +2%, for credits and metadata.
    Caption,
}

impl TypeStyle {
    /// Every style, in `type.groups` specimen order.
    pub const ALL: &'static [TypeStyle] = &[
        TypeStyle::Display,
        TypeStyle::Headline1,
        TypeStyle::Headline2,
        TypeStyle::Headline3,
        TypeStyle::Headline4,
        TypeStyle::Lede,
        TypeStyle::Body,
        TypeStyle::Editorial,
        TypeStyle::Eyebrow,
        TypeStyle::Mono,
        TypeStyle::Caption,
    ];

    /// The style's `tokens.json` name.
    pub const fn name(self) -> &'static str {
        match self {
            TypeStyle::Display => "type-display",
            TypeStyle::Headline1 => "type-headline-1",
            TypeStyle::Headline2 => "type-headline-2",
            TypeStyle::Headline3 => "type-headline-3",
            TypeStyle::Headline4 => "type-headline-4",
            TypeStyle::Lede => "type-lede",
            TypeStyle::Body => "type-body",
            TypeStyle::Editorial => "type-editorial",
            TypeStyle::Eyebrow => "type-eyebrow",
            TypeStyle::Mono => "type-mono",
            TypeStyle::Caption => "type-caption",
        }
    }

    /// The size in px from the group's `fontSize`.
    pub const fn size(self) -> f32 {
        match self {
            TypeStyle::Display => 96.0,
            TypeStyle::Headline1 => 64.0,
            TypeStyle::Headline2 => 48.0,
            TypeStyle::Headline3 => 32.0,
            TypeStyle::Headline4 | TypeStyle::Lede | TypeStyle::Editorial => 22.0,
            TypeStyle::Body => 17.0,
            TypeStyle::Eyebrow => 11.0,
            TypeStyle::Mono => 14.0,
            TypeStyle::Caption => 12.0,
        }
    }

    /// The unitless leading from the group's `lineHeight` (the eyebrow's
    /// is its absolute 16px over 11px).
    pub const fn leading(self) -> f32 {
        match self {
            TypeStyle::Display => 1.02,
            TypeStyle::Headline1 => 1.08,
            TypeStyle::Headline2 => 1.12,
            TypeStyle::Headline3 => 1.18,
            TypeStyle::Headline4 => 1.3,
            TypeStyle::Lede => 1.5,
            TypeStyle::Body => 1.65,
            TypeStyle::Editorial => 1.55,
            TypeStyle::Eyebrow => 16.0 / 11.0,
            TypeStyle::Mono => 1.6,
            TypeStyle::Caption => 1.5,
        }
    }

    /// The tracking in em from the group's `letterSpacing`.
    pub const fn tracking(self) -> f32 {
        match self {
            TypeStyle::Display => -0.04,
            TypeStyle::Headline1 => -0.035,
            TypeStyle::Headline2 => -0.03,
            TypeStyle::Headline3 => -0.02,
            TypeStyle::Headline4 => -0.01,
            TypeStyle::Lede | TypeStyle::Body | TypeStyle::Editorial | TypeStyle::Mono => 0.0,
            TypeStyle::Eyebrow => crate::tokens::tracking::TRACKING_EYEBROW,
            TypeStyle::Caption => 0.02,
        }
    }

    /// The nominal weight from the group's `fontWeight`. The shipped face
    /// is the nearest available one (the lede's 350 ships as 400;
    /// Instrument Serif ships one weight).
    pub const fn weight(self) -> u16 {
        match self {
            TypeStyle::Display | TypeStyle::Headline1 | TypeStyle::Headline2 => 500,
            TypeStyle::Headline3 | TypeStyle::Headline4 | TypeStyle::Eyebrow => 600,
            TypeStyle::Lede => 350,
            TypeStyle::Body | TypeStyle::Editorial | TypeStyle::Mono | TypeStyle::Caption => 400,
        }
    }

    /// The face family the style sets in each theme, per the README's
    /// faces table (Forest, Teal, Monochrome).
    const fn families(self) -> (&'static str, &'static str, &'static str) {
        match self {
            TypeStyle::Display | TypeStyle::Headline1 | TypeStyle::Headline2 => {
                ("Fraunces Display", "Instrument Serif", "Geist Medium")
            }
            TypeStyle::Headline3 => ("Fraunces Headline", "Geist SemiBold", "Geist SemiBold"),
            TypeStyle::Headline4 => ("Aptos SemiBold", "Geist SemiBold", "Geist SemiBold"),
            // The eyebrow is the system sans in every theme: Geist
            // SemiBold, tracked open, in text-accent.
            TypeStyle::Eyebrow => ("Geist SemiBold", "Geist SemiBold", "Geist SemiBold"),
            TypeStyle::Lede | TypeStyle::Body | TypeStyle::Caption => ("Aptos", "Geist", "Geist"),
            TypeStyle::Editorial => (
                "Fraunces Editorial",
                "Instrument Serif Italic",
                "Fraunces Editorial",
            ),
            TypeStyle::Mono => ("JetBrains Mono", "JetBrains Mono", "JetBrains Mono"),
        }
    }

    /// The named family the style sets in the theme.
    pub fn family_name(self, theme: ThemeId) -> &'static str {
        let (forest, teal, mono) = self.families();
        match theme.face_group() {
            FaceGroup::Forest => forest,
            FaceGroup::Teal => teal,
            FaceGroup::Mono => mono,
        }
    }

    /// The style's family in the theme, as a [`FontFamily::Name`].
    pub fn family(self, theme: ThemeId) -> FontFamily {
        FontFamily::Name(self.family_name(theme).into())
    }

    /// The style as an egui [`FontId`]: the specimen size in the theme's
    /// family. The weight lives in the family name (egui has no weight
    /// axis); the tracking is separate ([`TrackedLabel`] for the
    /// eyebrow's).
    pub fn font_id(self, theme: ThemeId) -> FontId {
        FontId {
            size: self.size(),
            family: self.family(theme),
        }
    }

    /// The line box in px: size times leading (the eyebrow's absolute
    /// 16px included through its ratio).
    pub const fn line_height(self) -> f32 {
        self.size() * self.leading()
    }
}

// ---- the pairing rule -----------------------------------------------------

/// The display-to-body floor the README's pairing rule states: display
/// type must sit at least 2.5x the body type beside it on the same
/// surface, or hierarchy collapses.
pub const DISPLAY_TO_BODY_MIN: f32 = 2.5;

/// Whether a display size pairs with a body size under the README's
/// 2.5x rule. The specimen's own display and body go through
/// [`display_pairing_holds`]; components that set their own sizes (the
/// cover's 64px headline over its 14px standfirst) check through here,
/// so every pairing answers to the one floor.
pub fn pairing_holds(display_size: f32, body_size: f32) -> bool {
    display_size / body_size >= DISPLAY_TO_BODY_MIN
}

/// The display-to-body ratio the README's pairing rule states: Fraunces
/// display must sit at least 2.5x the Aptos body beside it, or hierarchy
/// collapses.
pub fn display_to_body_ratio() -> f32 {
    TypeStyle::Display.size() / TypeStyle::Body.size()
}

/// Whether the specimen's display and body clear the 2.5x pairing rule.
pub fn display_pairing_holds() -> bool {
    pairing_holds(TypeStyle::Display.size(), TypeStyle::Body.size())
}

// ---- tracked text ---------------------------------------------------------

/// The total advance of tracked text: the glyph widths plus one track
/// between each pair of glyphs — `sum + (n-1)·track` — the model egui's
/// `extra_letter_spacing` paints (a track before every cluster that is
/// not the first of its section, epaint `text_layout.rs`'s
/// `is_first_glyph_in_section` rule). Not CSS letter-spacing, which
/// adds a trailing track after the last glyph as well.
pub fn tracked_text_width(widths: &[f32], tracking_em: f32, size: f32) -> f32 {
    let track = tracking_em * size;
    let between = widths.len().saturating_sub(1) as f32;
    widths.iter().sum::<f32>() + between * track
}

/// A single-section [`LayoutJob`] carrying tracking and an optional line
/// height: the kit's one path from a style's values to laid-out text.
/// The tracking travels in points (em times the font size) through
/// egui's cluster-boundary letter spacing, kerning kept within clusters.
fn tracked_job(
    text: &str,
    font_id: FontId,
    tracking_em: f32,
    line_height: Option<f32>,
    color: Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let spacing = tracking_em * font_id.size;
    job.append(
        text,
        0.0,
        egui::text::TextFormat {
            font_id,
            extra_letter_spacing: spacing,
            line_height,
            color,
            ..Default::default()
        },
    );
    job
}

/// `text` set as the style in the theme: the specimen's own font, size,
/// tracking and line height, in `color`. The eyebrow usually goes
/// through [`TrackedLabel`] instead, which uppercases; this job is for
/// every other style and for multi-line copy.
pub fn specimen_job(
    text: &str,
    style: TypeStyle,
    theme: ThemeId,
    color: Color32,
) -> egui::text::LayoutJob {
    tracked_job(
        text,
        style.font_id(theme),
        style.tracking(),
        Some(style.line_height()),
        color,
    )
}

/// `text` in `font_id`, advanced by one `tracking_em` track between
/// each pair of glyphs at an optional line height, in `color`: the same
/// path [`specimen_job`] takes, for the component styles that are not
/// one of the specimen's (the metric figure's −4% at 56px, the
/// editorial headline's −2% at 24px). Unlike [`TrackedLabel`] this
/// never uppercases: figures and headlines keep their case. The job
/// wraps at the containing `Ui`'s wrap width, as any label does.
pub fn tracked_text(
    text: &str,
    font_id: FontId,
    tracking_em: f32,
    line_height: Option<f32>,
    color: Color32,
) -> egui::text::LayoutJob {
    tracked_job(text, font_id, tracking_em, line_height, color)
}

/// The eyebrow widget: a single-line label tracked open at the style's
/// +0.24em, set in uppercase in `text-accent`, the brand's signature
/// label. Use it with [`TypeStyle::Eyebrow`]'s font id and tracking.
///
/// ```
/// use egui::Color32;
/// use ui_kit::theme::ThemeId;
/// use ui_kit::typography::{TrackedLabel, TypeStyle};
///
/// let label = TrackedLabel::new(
///     "Field notes",
///     TypeStyle::Eyebrow.font_id(ThemeId::Light),
///     TypeStyle::Eyebrow.tracking(),
///     Color32::BLACK,
/// );
/// ```
pub struct TrackedLabel<'a> {
    text: &'a str,
    font_id: FontId,
    tracking_em: f32,
    color: Color32,
}

impl<'a> TrackedLabel<'a> {
    /// The label: `text` in `font_id`, advanced by one `tracking_em`
    /// track between each pair of glyphs, painted in `color`. Use the
    /// eyebrow's font id and [`TypeStyle::tracking`] for the signature
    /// label.
    pub const fn new(text: &'a str, font_id: FontId, tracking_em: f32, color: Color32) -> Self {
        Self {
            text,
            font_id,
            tracking_em,
            color,
        }
    }

    /// The label's laid-out text, uppercased and tracked: exposed for
    /// tests and for callers measuring the label before painting it.
    fn job(&self) -> egui::text::LayoutJob {
        tracked_job(
            &self.text.to_uppercase(),
            self.font_id.clone(),
            self.tracking_em,
            None,
            self.color,
        )
    }
}

impl Widget for TrackedLabel<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.add(egui::Label::new(self.job()))
    }
}

#[cfg(test)]
mod tests {
    use egui::{Color32, FontFamily, FontId};

    use crate::theme::ThemeId;
    use crate::tokens::tracking;

    use super::{
        display_pairing_holds, display_to_body_ratio, install, tracked_text_width, TypeStyle,
    };

    /// The style table matches `tokens.json` `type.groups` exactly: the
    /// eleven specimen styles with their sizes, leadings, trackings and
    /// nominal weights. The eyebrow's leading is the tokens' absolute 16px
    /// over its 11px size.
    #[test]
    fn style_table_matches_tokens_json_groups() {
        // (style, tokens name, size, leading, tracking em, nominal weight)
        let expected: &[(TypeStyle, &str, f32, f32, f32, u16)] = &[
            (TypeStyle::Display, "type-display", 96.0, 1.02, -0.04, 500),
            (
                TypeStyle::Headline1,
                "type-headline-1",
                64.0,
                1.08,
                -0.035,
                500,
            ),
            (
                TypeStyle::Headline2,
                "type-headline-2",
                48.0,
                1.12,
                -0.03,
                500,
            ),
            (
                TypeStyle::Headline3,
                "type-headline-3",
                32.0,
                1.18,
                -0.02,
                600,
            ),
            (
                TypeStyle::Headline4,
                "type-headline-4",
                22.0,
                1.3,
                -0.01,
                600,
            ),
            (TypeStyle::Lede, "type-lede", 22.0, 1.5, 0.0, 350),
            (TypeStyle::Body, "type-body", 17.0, 1.65, 0.0, 400),
            (TypeStyle::Editorial, "type-editorial", 22.0, 1.55, 0.0, 400),
            (
                TypeStyle::Eyebrow,
                "type-eyebrow",
                11.0,
                16.0 / 11.0,
                0.24,
                600,
            ),
            (TypeStyle::Mono, "type-mono", 14.0, 1.6, 0.0, 400),
            (TypeStyle::Caption, "type-caption", 12.0, 1.5, 0.02, 400),
        ];
        assert_eq!(TypeStyle::ALL.len(), expected.len(), "all eleven styles");
        for (style, (expected_style, name, size, leading, tracking_em, weight)) in
            TypeStyle::ALL.iter().zip(expected)
        {
            assert_eq!(style, expected_style, "specimen order from type.groups");
            assert_eq!(style.name(), *name);
            assert_eq!(style.size(), *size, "{name}: size");
            assert_eq!(style.leading(), *leading, "{name}: leading");
            assert_eq!(style.tracking(), *tracking_em, "{name}: tracking");
            assert_eq!(style.weight(), *weight, "{name}: nominal weight");
        }
    }

    /// The eyebrow sets 16px of line over its 11px size, per the tokens'
    /// absolute lineHeight, and its tracking is the signature +0.24em the
    /// tracking tokens carry.
    #[test]
    fn eyebrow_line_is_16px_tracked_a_quarter_em() {
        assert!((TypeStyle::Eyebrow.line_height() - 16.0).abs() < 1e-4);
        assert_eq!(TypeStyle::Eyebrow.tracking(), tracking::TRACKING_EYEBROW);
        assert_eq!(tracking::TRACKING_EYEBROW, 0.24);
    }

    /// Every style of every theme resolves to a family that theme
    /// installs, and the `FontId` carries the style's size.
    #[test]
    fn every_style_of_every_theme_maps_to_an_installed_face() {
        for theme in ThemeId::ALL {
            let faces = theme.faces();
            let name = theme.name();
            let installed: Vec<&str> = faces.iter().map(|face| face.family).collect();
            assert!(!installed.is_empty(), "{name}: carries faces");
            for style in TypeStyle::ALL {
                let family = style.family_name(*theme);
                assert!(
                    installed.contains(&family),
                    "{name}: {} maps to {family}, which the theme does not install",
                    style.name()
                );
                let font_id = style.font_id(*theme);
                assert_eq!(font_id.size, style.size(), "{name}: FontId size");
                assert_eq!(
                    font_id.family,
                    egui::FontFamily::Name(family.into()),
                    "{name}: FontId family"
                );
            }
            // Every face embeds real bytes from its prepared file.
            for face in faces {
                assert!(!face.bytes.is_empty(), "{name}: {} has bytes", face.file);
            }
        }
    }

    /// The three themes together install exactly the twelve prepared
    /// faces from `assets/fonts/`, each theme's set free of duplicates.
    #[test]
    fn theme_faces_are_exactly_the_prepared_font_set() {
        let expected: Vec<(&str, &str)> = vec![
            ("Aptos", "Aptos.ttf"),
            ("Aptos SemiBold", "Aptos-SemiBold.ttf"),
            ("Fraunces Display", "Fraunces-Display.ttf"),
            ("Fraunces Editorial", "Fraunces-EditorialItalic.ttf"),
            ("Fraunces Headline", "Fraunces-Headline.ttf"),
            ("Fraunces Metric", "Fraunces-Metric.ttf"),
            ("Geist", "Geist-Regular.ttf"),
            ("Geist Medium", "Geist-Medium.ttf"),
            ("Geist SemiBold", "Geist-SemiBold.ttf"),
            ("Instrument Serif", "InstrumentSerif-Regular.ttf"),
            ("Instrument Serif Italic", "InstrumentSerif-Italic.ttf"),
            ("JetBrains Mono", "JetBrainsMono-Regular.ttf"),
        ];
        let mut union: Vec<(&str, &str)> = Vec::new();
        for theme in ThemeId::ALL {
            let mut families: Vec<&str> = Vec::new();
            for face in theme.faces() {
                assert!(
                    !families.contains(&face.family),
                    "{}: no duplicate families",
                    theme.name()
                );
                families.push(face.family);
                if !union.iter().any(|(f, _)| *f == face.family) {
                    union.push((face.family, face.file));
                }
            }
        }
        union.sort_unstable();
        let mut expected = expected;
        expected.sort_unstable();
        assert_eq!(union, expected, "the twelve prepared faces, no strays");
    }

    /// The bold display voice resolves per theme to a family that theme
    /// installs: Fraunces Metric in Forest (the issue 08 instance at
    /// wght 700), Instrument Serif in Teal (one weight, no faux bold)
    /// and Geist SemiBold in Monochrome (the nearest prepared Geist to
    /// the spec's 700).
    #[test]
    fn display_bold_resolves_to_an_installed_face_per_theme() {
        let expected: &[(ThemeId, &str)] = &[
            (ThemeId::Light, "Fraunces Metric"),
            (ThemeId::Dark, "Fraunces Metric"),
            (ThemeId::TealLight, "Instrument Serif"),
            (ThemeId::TealDark, "Instrument Serif"),
            (ThemeId::MonoLight, "Geist SemiBold"),
            (ThemeId::MonoDark, "Geist SemiBold"),
        ];
        for (theme, family) in expected {
            assert_eq!(super::display_bold_family_name(*theme), *family);
            assert_eq!(
                super::display_bold_family(*theme),
                FontFamily::Name((*family).into())
            );
            let installed: Vec<&str> = theme.faces().iter().map(|face| face.family).collect();
            assert!(
                installed.contains(family),
                "{}: display bold maps to {family}, which the theme does not install",
                theme.name()
            );
        }
    }

    /// `tracked_text` lays out its text verbatim (no uppercasing, unlike
    /// [`super::TrackedLabel`]) with the requested tracking in points
    /// and optional line height: the metric figure's −4% at 56px and the
    /// editorial headline's −2% at 24px go through it.
    #[test]
    fn tracked_text_keeps_case_and_carries_its_tracking() {
        let job = super::tracked_text(
            "714k",
            FontId {
                size: 56.0,
                family: super::display_bold_family(ThemeId::Light),
            },
            -0.04,
            Some(56.0),
            Color32::BLACK,
        );
        assert_eq!(job.text, "714k", "figures keep their case");
        assert_eq!(job.sections.len(), 1);
        let format = &job.sections[0].format;
        assert!((format.extra_letter_spacing - (-0.04 * 56.0)).abs() < 1e-4);
        assert_eq!(format.line_height, Some(56.0));
        assert_eq!(format.color, Color32::BLACK);
    }

    /// The body face at weight 600 resolves per theme to a family that
    /// theme installs: Aptos SemiBold in Forest, Geist SemiBold in Teal
    /// and Monochrome. The components' label voice (button and badge
    /// labels) draws on it.
    #[test]
    fn body_semibold_resolves_to_an_installed_face_per_theme() {
        let expected: &[(ThemeId, &str)] = &[
            (ThemeId::Light, "Aptos SemiBold"),
            (ThemeId::Dark, "Aptos SemiBold"),
            (ThemeId::TealLight, "Geist SemiBold"),
            (ThemeId::TealDark, "Geist SemiBold"),
            (ThemeId::MonoLight, "Geist SemiBold"),
            (ThemeId::MonoDark, "Geist SemiBold"),
        ];
        for (theme, family) in expected {
            assert_eq!(super::body_semibold_family_name(*theme), *family);
            assert_eq!(
                super::body_semibold_family(*theme),
                FontFamily::Name((*family).into())
            );
            let installed: Vec<&str> = theme.faces().iter().map(|face| face.family).collect();
            assert!(
                installed.contains(family),
                "{}: the label voice maps to {family}, which the theme does not install",
                theme.name()
            );
        }
    }

    /// The per-theme face mapping follows the README table: Forest sets
    /// display and editorial in Fraunces with Aptos for body; Teal sets
    /// display and editorial in Instrument Serif with Geist for body and
    /// for H3 and smaller; Monochrome sets display in Geist (standing in
    /// for Söhne) with Fraunces for editorial.
    #[test]
    fn face_mapping_follows_the_readme_table() {
        let cases: &[(ThemeId, TypeStyle, &str)] = &[
            (ThemeId::Light, TypeStyle::Display, "Fraunces Display"),
            (ThemeId::Dark, TypeStyle::Headline2, "Fraunces Display"),
            (ThemeId::Light, TypeStyle::Headline3, "Fraunces Headline"),
            (ThemeId::Light, TypeStyle::Headline4, "Aptos SemiBold"),
            (ThemeId::Light, TypeStyle::Body, "Aptos"),
            (ThemeId::Dark, TypeStyle::Editorial, "Fraunces Editorial"),
            (ThemeId::TealLight, TypeStyle::Display, "Instrument Serif"),
            (ThemeId::TealDark, TypeStyle::Headline2, "Instrument Serif"),
            // H3 and smaller in Geist, per the Teal section.
            (ThemeId::TealLight, TypeStyle::Headline3, "Geist SemiBold"),
            (ThemeId::TealDark, TypeStyle::Headline4, "Geist SemiBold"),
            (ThemeId::TealLight, TypeStyle::Body, "Geist"),
            (
                ThemeId::TealDark,
                TypeStyle::Editorial,
                "Instrument Serif Italic",
            ),
            // Geist stands in for the spec's Söhne display.
            (ThemeId::MonoLight, TypeStyle::Display, "Geist Medium"),
            (ThemeId::MonoDark, TypeStyle::Headline2, "Geist Medium"),
            (ThemeId::MonoLight, TypeStyle::Headline3, "Geist SemiBold"),
            (ThemeId::MonoDark, TypeStyle::Body, "Geist"),
            (
                ThemeId::MonoLight,
                TypeStyle::Editorial,
                "Fraunces Editorial",
            ),
        ];
        for (theme, style, family) in cases {
            assert_eq!(
                style.family_name(*theme),
                *family,
                "{}: {}",
                theme.name(),
                style.name()
            );
        }
        // The eyebrow is Geist SemiBold and the mono style JetBrains Mono
        // in every theme.
        for theme in ThemeId::ALL {
            assert_eq!(
                TypeStyle::Eyebrow.family_name(*theme),
                "Geist SemiBold",
                "{}: eyebrow",
                theme.name()
            );
            assert_eq!(
                TypeStyle::Mono.family_name(*theme),
                "JetBrains Mono",
                "{}: mono",
                theme.name()
            );
        }
    }

    /// The specimen jobs carry their style's tracking (em times size, in
    /// points, through egui's cluster-boundary letter spacing) and line
    /// height, in the theme's family, text intact.
    #[test]
    fn specimen_jobs_carry_their_tracking_and_line_height() {
        for theme in ThemeId::ALL {
            for style in TypeStyle::ALL {
                let job =
                    super::specimen_job("Quiet is the new loud.", *style, *theme, Color32::BLACK);
                let name = style.name();
                assert_eq!(job.text, "Quiet is the new loud.", "{name}: text intact");
                assert_eq!(job.sections.len(), 1, "{name}: one section");
                let format = &job.sections[0].format;
                assert_eq!(format.font_id, style.font_id(*theme), "{name}: font id");
                assert!(
                    (format.extra_letter_spacing - style.tracking() * style.size()).abs() < 1e-4,
                    "{name}: tracking in points"
                );
                assert_eq!(
                    format.line_height,
                    Some(style.line_height()),
                    "{name}: line height"
                );
                assert_eq!(format.color, Color32::BLACK, "{name}: colour");
            }
        }
    }

    /// The tracked label sets its text in uppercase and advances it by
    /// its tracking: the eyebrow's signature +0.24em at 11px.
    #[test]
    fn tracked_label_uppercases_and_tracks() {
        let label = super::TrackedLabel::new(
            "Field notes · 2026",
            TypeStyle::Eyebrow.font_id(ThemeId::Light),
            TypeStyle::Eyebrow.tracking(),
            Color32::BLACK,
        );
        let job = label.job();
        assert_eq!(job.text, "FIELD NOTES · 2026");
        let format = &job.sections[0].format;
        assert!((format.extra_letter_spacing - 0.24 * 11.0).abs() < 1e-4);
    }

    /// Tracked text advances by its glyph widths plus one track
    /// between each pair of glyphs — `sum + (n-1)·track` — the model
    /// egui's `extra_letter_spacing` paints (a track before every
    /// cluster that is not the first of its section, epaint
    /// `text_layout.rs`'s `is_first_glyph_in_section` rule). Not CSS
    /// letter-spacing, which adds a trailing track after the last
    /// glyph too.
    #[test]
    fn tracked_width_adds_one_track_between_glyphs() {
        // 10 + 20 + 30 = 60 of glyphs, two tracks of 0.24em at 11px
        // (2.64px each): 65.28 total.
        let width = tracked_text_width(&[10.0, 20.0, 30.0], 0.24, 11.0);
        assert!((width - 65.28).abs() < 1e-4);
        // A single glyph carries no track: nothing follows it.
        assert!((tracked_text_width(&[10.0], 0.24, 11.0) - 10.0).abs() < 1e-4);
        // Zero tracking collapses to the plain advance.
        assert!((tracked_text_width(&[10.0, 20.0, 30.0], 0.0, 11.0) - 60.0).abs() < 1e-4);
    }

    /// The model against the engine itself: a real galley of the
    /// eyebrow's tracked style measures what the helper predicts from
    /// that galley's own glyph advances — one track between glyphs,
    /// none after the last. Galley widths round to the pixel grid,
    /// hence the 1px tolerance; the trailing track the CSS model would
    /// add is 2.64px here, well outside it.
    #[test]
    fn tracked_width_matches_a_real_egui_galley() {
        let ctx = egui::Context::default();
        let install_pass = ctx.run_ui(egui::RawInput::default(), |ui| {
            super::install(ThemeId::Light, ui.ctx())
        });
        install_pass.drop_without_applying_deltas();
        let layout_pass = ctx.run_ui(egui::RawInput::default(), |ui| {
            let job = super::specimen_job(
                "TRACKEDWIDTH",
                TypeStyle::Eyebrow,
                ThemeId::Light,
                Color32::BLACK,
            );
            let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));
            let advances: Vec<f32> = galley.rows[0]
                .glyphs
                .iter()
                .map(|glyph| glyph.advance_width)
                .collect();
            assert_eq!(
                advances.len(),
                "TRACKEDWIDTH".chars().count(),
                "one glyph per character, one cluster each"
            );
            let expected = tracked_text_width(
                &advances,
                TypeStyle::Eyebrow.tracking(),
                TypeStyle::Eyebrow.size(),
            );
            assert!(
                (galley.size().x - expected).abs() <= 1.0,
                "engine width {} vs the helper's {}",
                galley.size().x,
                expected
            );
        });
        layout_pass.drop_without_applying_deltas();
    }

    /// The pairing rule: Fraunces display must sit at least 2.5x the body
    /// beside it. The specimen's 96 over 17 clears it with room.
    #[test]
    fn display_pairs_with_body_at_two_and_a_half_times() {
        assert!((display_to_body_ratio() - 96.0 / 17.0).abs() < 1e-4);
        assert!(display_to_body_ratio() >= 2.5);
        assert!(display_pairing_holds());
    }

    /// The pairing rule generalises beyond the specimen: any display
    /// size over any body size checks against the same 2.5x floor. The
    /// cover's 64 over 14 clears it; a 34 over 17 does not; equal sizes
    /// never do.
    #[test]
    fn pairing_helper_checks_any_display_body_pair() {
        assert_eq!(super::DISPLAY_TO_BODY_MIN, 2.5);
        assert!(super::pairing_holds(
            TypeStyle::Display.size(),
            TypeStyle::Body.size()
        ));
        assert!(super::pairing_holds(64.0, 14.0));
        assert!(!super::pairing_holds(34.0, 17.0));
        assert!(!super::pairing_holds(17.0, 17.0));
    }

    /// `install` registers the theme's named families on a live context:
    /// after a pass, egui lists every family the theme's styles (and the
    /// components' bold display voice) reference. The install is
    /// additive, so each theme's families accumulate in the one context
    /// rather than replace each other.
    #[test]
    fn install_registers_the_named_families_with_a_live_context() {
        let ctx = egui::Context::default();
        let mut seen: Vec<egui::FontFamily> = Vec::new();
        for theme in ThemeId::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| install(*theme, ui.ctx()));
            out.drop_without_applying_deltas();
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let families = ui.ctx().fonts(|fonts| fonts.families());
                let mut referenced: Vec<egui::FontFamily> = TypeStyle::ALL
                    .iter()
                    .map(|style| style.family(*theme))
                    .collect();
                referenced.push(super::display_bold_family(*theme));
                for family in referenced {
                    assert!(
                        families.contains(&family),
                        "{}: {family} resolves to an installed family",
                        theme.name()
                    );
                    if !seen.contains(&family) {
                        seen.push(family);
                    }
                }
            });
            out.drop_without_applying_deltas();
        }
        // Additive across themes: twelve families, none of them dropped
        // by a later theme's install.
        assert_eq!(seen.len(), 12, "every prepared family stays installed");
    }

    /// The FontId constructors resolve per theme without allocating
    /// surprises: same style, same theme, same id.
    #[test]
    fn font_ids_are_stable_per_theme() {
        let a = TypeStyle::Display.font_id(ThemeId::Light);
        let b = TypeStyle::Display.font_id(ThemeId::Light);
        assert_eq!(a, b);
        assert_eq!(
            a,
            FontId {
                size: 96.0,
                family: egui::FontFamily::Name("Fraunces Display".into())
            }
        );
    }
}

//! LiquidGlass: translucent chrome for top app bars, sheets, popovers
//! and modal headers, per `components/LiquidGlass`, the preview's five
//! thickness tiles and the README's "Transparency, blur, Liquid Glass".
//!
//! Five thicknesses (ultra-thin through chrome) come as blur and
//! saturation equivalents from `colors_and_type.css` section 15: blur
//! 20, 32, 48, 64 and 80px at 140, 160 and then 180 percent
//! saturation. The tints are the theme's glass tint tokens; the 1px
//! specular edge (the inner top highlight from the glass-specular
//! tokens) is mandatory, without it the surface reads as flat
//! transparency, not glass; the elevation is shadow L2. Glass is for
//! chrome and overlays only, never body content, and never two panes
//! of the same thickness stacked: the host supplies the backdrop.
//!
//! Documented deviation: egui composites no backdrop blur, so the pane
//! is a semi-transparent tint approximation. The tint and its opacity
//! carry the effect over whatever the host paints beneath (the gallery
//! places panes over palette content), and the thickness a pane was
//! built at reaches the host through [`GlassRecipe::thickness`] for
//! labelling, since the blur itself cannot render.
//!
//! Under [`UiKitFlags::reduced_transparency`] the pane turns solid
//! (`surface-container`, `primary` or `tertiary` per the tint) and the
//! specular edge goes, exactly the web system's
//! `prefers-reduced-transparency` swap, and the primary and accent
//! labels move to their on-colours.

use egui::epaint::CornerRadius;
use egui::{Color32, Painter, Pos2, Rect, Response, Stroke, Ui};

use crate::states::UiKitFlags;
use crate::theme::{egui_shadow, ThemeId};
use crate::tokens::{radii, shadows, shadows::ShadowLayer, spacing, ThemeColors};

/// One of the five thicknesses, from `--glass-blur-*` in
/// `colors_and_type.css` section 15.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlassThickness {
    /// `glass--ultra-thin`: blur 20 at 140% saturation. Image
    /// captions.
    UltraThin,
    /// `glass--thin`: blur 32 at 160%. Navigation over imagery.
    Thin,
    /// The plain `.glass`: blur 48 at 180%. Sheets and popovers.
    #[default]
    Regular,
    /// `glass--thick`: blur 64 at 180%. Full-sheet covers.
    Thick,
    /// `glass--chrome`: blur 80 at 180%. System chrome that must
    /// isolate content.
    Chrome,
}

impl GlassThickness {
    /// Every thickness, thin to chrome.
    pub const ALL: &'static [GlassThickness] = &[
        GlassThickness::UltraThin,
        GlassThickness::Thin,
        GlassThickness::Regular,
        GlassThickness::Thick,
        GlassThickness::Chrome,
    ];

    /// The `tokens.json`-side class name.
    pub const fn name(self) -> &'static str {
        match self {
            GlassThickness::UltraThin => "glass--ultra-thin",
            GlassThickness::Thin => "glass--thin",
            GlassThickness::Regular => "glass",
            GlassThickness::Thick => "glass--thick",
            GlassThickness::Chrome => "glass--chrome",
        }
    }

    /// The blur, in px, the thickness stands for.
    pub const fn blur_px(self) -> f32 {
        match self {
            GlassThickness::UltraThin => 20.0,
            GlassThickness::Thin => 32.0,
            GlassThickness::Regular => 48.0,
            GlassThickness::Thick => 64.0,
            GlassThickness::Chrome => 80.0,
        }
    }

    /// The saturation, in percent, the thickness stands for.
    pub const fn saturation_percent(self) -> f32 {
        match self {
            GlassThickness::UltraThin => 140.0,
            GlassThickness::Thin => 160.0,
            GlassThickness::Regular | GlassThickness::Thick | GlassThickness::Chrome => 180.0,
        }
    }

    /// What the thickness is for, from the component README.
    pub const fn usage(self) -> &'static str {
        match self {
            GlassThickness::UltraThin => "Image captions",
            GlassThickness::Thin => "Nav over imagery",
            GlassThickness::Regular => "Sheets, popovers",
            GlassThickness::Thick => "Full-sheet covers",
            GlassThickness::Chrome => "System chrome",
        }
    }
}

/// The tint: which glass tint token the pane fills with, and therefore
/// what its labels take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlassTint {
    /// The plain `.glass`: `glass-tint-surface`, labels in the body
    /// text colour. The default chrome tint.
    #[default]
    Surface,
    /// `glass--primary`: `glass-tint-primary`, labels in
    /// `glass-on-primary`. Keep it over the canvas or photography.
    Primary,
    /// `glass--accent`: `glass-tint-accent`, labels `on-surface`. Rare,
    /// never the main chrome tint (accent-tinted glass reads as
    /// jaundice).
    Accent,
}

impl GlassTint {
    /// Every tint, in stylesheet order.
    pub const ALL: &'static [GlassTint] =
        &[GlassTint::Surface, GlassTint::Primary, GlassTint::Accent];

    /// The stylesheet class.
    pub const fn name(self) -> &'static str {
        match self {
            GlassTint::Surface => "glass",
            GlassTint::Primary => "glass--primary",
            GlassTint::Accent => "glass--accent",
        }
    }
}

/// One pane's resolved paints: the fill (the tint with its token alpha,
/// or the solid equivalent under reduced transparency), the label
/// colour, the mandatory 1px specular edge (gone under the flag), the
/// L2 elevation, and the thickness the pane was built at.
#[derive(Clone, Copy, Debug)]
pub struct GlassRecipe {
    /// The pane's fill.
    pub fill: Color32,
    /// The colour the pane's labels take.
    pub label: Color32,
    /// The specular edge's stroke colour, straight from the
    /// glass-specular tokens; `None` under reduced transparency.
    pub specular: Option<Color32>,
    /// The elevation: shadow L2, every pane.
    pub shadow: &'static [ShadowLayer],
    /// The thickness the pane was built at, for the host's labelling
    /// (the blur itself cannot render, see the module docs).
    pub thickness: GlassThickness,
}

/// Resolve one pane's recipe: the tint's fill and label colour, the
/// theme's specular edge and L2 shadow, or the solid fallback under
/// [`UiKitFlags::solid_surfaces`].
pub fn glass_recipe(
    thickness: GlassThickness,
    tint: GlassTint,
    theme: ThemeId,
    flags: &UiKitFlags,
) -> GlassRecipe {
    let colors = theme.colors();
    // The tint's fill and label colour, or the solid swap when the host
    // asked for solid surfaces.
    let (fill, label) = if flags.solid_surfaces() {
        solid(tint, &colors)
    } else {
        match tint {
            GlassTint::Surface => (colors.glass_tint_surface, colors.text_body()),
            GlassTint::Primary => (colors.glass_tint_primary, colors.glass_on_primary),
            GlassTint::Accent => (colors.glass_tint_accent, colors.on_surface),
        }
    };
    GlassRecipe {
        fill,
        label,
        // The mandatory 1px specular edge from the theme's
        // glass-specular token, dropped under reduced transparency.
        specular: (!flags.solid_surfaces()).then(|| {
            let edge = if theme.is_dark() {
                shadows::GLASS_SPECULAR_DARK
            } else {
                shadows::GLASS_SPECULAR_LIGHT
            };
            edge[0].color
        }),
        shadow: theme.shadows().l2,
        thickness,
    }
}

/// The specular edge as a hairline: the inner top highlight runs 1px
/// down, spanning between the corner rounds.
pub fn specular_line(rect: Rect, radius: f32) -> [Pos2; 2] {
    let y = rect.min.y + 1.0;
    [
        Pos2::new(rect.min.x + radius, y),
        Pos2::new(rect.max.x - radius, y),
    ]
}

/// Paint one pane at `rect`: the L2 shadow, the fill, and (when the
/// recipe carries it) the specular hairline. The pane's chrome radius
/// is `radius-md` per the preview's `.pane`; hosts with their own shape
/// pass their own. This is the entry point for chrome that paints
/// itself (issue 09's top app bar); [`LiquidGlass::ui`] wraps it for
/// panes that lay content out.
pub fn paint_glass(
    painter: &Painter,
    rect: Rect,
    radius: impl Into<CornerRadius>,
    recipe: &GlassRecipe,
) {
    let radius = radius.into();
    painter.add(egui_shadow(recipe.shadow).as_shape(rect, radius));
    painter.rect_filled(rect, radius, recipe.fill);
    if let Some(color) = recipe.specular {
        let [from, to] = specular_line(rect, radius.nw as f32);
        painter.line_segment([from, to], Stroke::new(1.0, color));
    }
}

/// The LiquidGlass pane widget: a chrome container whose content is one
/// closure, laid out at the pane's own 12px padding, painted through
/// [`glass_recipe`] and [`paint_glass`].
///
/// ```
/// use ui_kit::components::LiquidGlass;
/// use ui_kit::components::liquid_glass::GlassThickness;
///
/// let regular = LiquidGlass::new();
/// let chrome = LiquidGlass::new().thickness(GlassThickness::Chrome);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct LiquidGlass {
    thickness: GlassThickness,
    tint: GlassTint,
}

impl Default for LiquidGlass {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidGlass {
    /// A regular surface-tinted pane.
    pub const fn new() -> Self {
        Self {
            thickness: GlassThickness::Regular,
            tint: GlassTint::Surface,
        }
    }

    /// Set the thickness.
    pub const fn thickness(mut self, thickness: GlassThickness) -> Self {
        self.thickness = thickness;
        self
    }

    /// Set the tint.
    pub const fn tint(mut self, tint: GlassTint) -> Self {
        self.tint = tint;
        self
    }

    /// The pane's recipe in the theme under the flags.
    pub fn recipe(self, theme: ThemeId, flags: &UiKitFlags) -> GlassRecipe {
        glass_recipe(self.thickness, self.tint, theme, flags)
    }

    /// Show the pane with `add` as its content. The pane is chrome, not
    /// a control: it senses nothing, and its labels should take
    /// [`GlassRecipe::label`].
    pub fn ui<R>(
        self,
        ui: &mut Ui,
        theme: ThemeId,
        flags: &UiKitFlags,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> Response {
        let recipe = self.recipe(theme, flags);
        let radius = CornerRadius::same(radii::RADIUS_MD as u8);
        let frame = egui::Frame::default()
            .inner_margin(spacing::SPACE_3)
            .corner_radius(radius)
            .fill(recipe.fill)
            .shadow(egui_shadow(recipe.shadow));
        let inner = frame.show(ui, |ui| add(ui));
        if let Some(color) = recipe.specular {
            let [from, to] = specular_line(inner.response.rect, radii::RADIUS_MD);
            ui.painter()
                .line_segment([from, to], Stroke::new(1.0, color));
        }
        inner.response
    }
}

/// The solid fills and label colours the reduced-transparency swap
/// calls for, per tint: the pane fills the equivalent solid token and
/// the primary and accent labels move to their on-colours.
fn solid(tint: GlassTint, colors: &ThemeColors) -> (Color32, Color32) {
    match tint {
        GlassTint::Surface => (colors.surface_container, colors.text_body()),
        GlassTint::Primary => (colors.primary, colors.on_primary),
        GlassTint::Accent => (colors.tertiary, colors.on_tertiary),
    }
}

#[cfg(test)]
mod tests {
    use super::{glass_recipe, paint_glass, specular_line, GlassThickness, GlassTint, LiquidGlass};
    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::shadows;
    use egui::{Pos2, Rect};

    /// The thickness table matches `colors_and_type.css` section 15:
    /// blur 20, 32, 48, 64 and 80px at 140, 160 and then 180 percent
    /// saturation, with the usages the component README names.
    #[test]
    fn thickness_table_matches_the_stylesheet() {
        let expected: &[(GlassThickness, &str, f32, f32, &str)] = &[
            (
                GlassThickness::UltraThin,
                "glass--ultra-thin",
                20.0,
                140.0,
                "Image captions",
            ),
            (
                GlassThickness::Thin,
                "glass--thin",
                32.0,
                160.0,
                "Nav over imagery",
            ),
            (
                GlassThickness::Regular,
                "glass",
                48.0,
                180.0,
                "Sheets, popovers",
            ),
            (
                GlassThickness::Thick,
                "glass--thick",
                64.0,
                180.0,
                "Full-sheet covers",
            ),
            (
                GlassThickness::Chrome,
                "glass--chrome",
                80.0,
                180.0,
                "System chrome",
            ),
        ];
        assert_eq!(GlassThickness::ALL.len(), expected.len());
        for (thickness, (kind, name, blur, saturation, usage)) in
            GlassThickness::ALL.iter().zip(expected)
        {
            assert_eq!(thickness, kind);
            assert_eq!(thickness.name(), *name);
            assert_eq!(thickness.blur_px(), *blur);
            assert_eq!(thickness.saturation_percent(), *saturation);
            assert_eq!(thickness.usage(), *usage);
        }
    }

    /// The recipe resolves per theme from the tokens: the fill is the
    /// tint token with its own alpha, the labels follow the tint, the
    /// specular edge is the theme's (light 56, dark 20 of alpha) and
    /// the elevation is L2.
    #[test]
    fn recipes_resolve_from_the_tokens_per_theme() {
        let flags = UiKitFlags::default();
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let surface = glass_recipe(GlassThickness::Regular, GlassTint::Surface, *theme, &flags);
            assert_eq!(
                surface.fill, colors.glass_tint_surface,
                "{name}: the tint token"
            );
            assert_eq!(
                surface.fill.a(),
                colors.glass_tint_surface.a(),
                "{name}: its alpha"
            );
            assert_eq!(
                surface.label,
                colors.text_body(),
                "{name}: body-text labels"
            );

            let primary = glass_recipe(GlassThickness::Thin, GlassTint::Primary, *theme, &flags);
            assert_eq!(
                primary.fill, colors.glass_tint_primary,
                "{name}: primary tint"
            );
            assert_eq!(
                primary.label, colors.glass_on_primary,
                "{name}: light labels"
            );

            let accent = glass_recipe(GlassThickness::Chrome, GlassTint::Accent, *theme, &flags);
            assert_eq!(accent.fill, colors.glass_tint_accent, "{name}: accent tint");
            assert_eq!(accent.label, colors.on_surface, "{name}: on-surface labels");

            for recipe in [surface, primary, accent] {
                let specular = recipe.specular.expect("{name}: the edge is mandatory");
                let edge = if theme.is_dark() {
                    shadows::GLASS_SPECULAR_DARK[0].color
                } else {
                    shadows::GLASS_SPECULAR_LIGHT[0].color
                };
                assert_eq!(specular, edge, "{name}: the glass-specular token");
                assert_eq!(specular.a(), edge.a(), "{name}: the token's alpha");
                assert_eq!(recipe.shadow, theme.shadows().l2, "{name}: shadow L2");
            }

            // The thickness travels with the recipe for the host's
            // labelling.
            assert_eq!(
                glass_recipe(GlassThickness::Thick, GlassTint::Surface, *theme, &flags).thickness,
                GlassThickness::Thick
            );
        }
    }

    /// Under reduced transparency every pane turns solid, the specular
    /// edge goes, and the primary and accent labels move to their
    /// on-colours: the web system's swap, flag-driven.
    #[test]
    fn reduced_transparency_swaps_to_solid_tokens() {
        let flags = UiKitFlags {
            reduced_transparency: true,
            ..Default::default()
        };
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let surface = glass_recipe(GlassThickness::Regular, GlassTint::Surface, *theme, &flags);
            assert_eq!(
                surface.fill, colors.surface_container,
                "{name}: solid surface-container"
            );
            assert_eq!(surface.fill.a(), 255, "{name}: fully opaque");
            assert_eq!(surface.specular, None, "{name}: no edge");
            assert_eq!(
                surface.label,
                colors.text_body(),
                "{name}: body labels stay"
            );

            let primary = glass_recipe(GlassThickness::Regular, GlassTint::Primary, *theme, &flags);
            assert_eq!(primary.fill, colors.primary, "{name}: solid primary");
            assert_eq!(
                primary.label, colors.on_primary,
                "{name}: on-primary labels"
            );
            assert_eq!(primary.specular, None);

            let accent = glass_recipe(GlassThickness::Regular, GlassTint::Accent, *theme, &flags);
            assert_eq!(accent.fill, colors.tertiary, "{name}: solid tertiary");
            assert_eq!(
                accent.label, colors.on_tertiary,
                "{name}: on-tertiary labels"
            );
            assert_eq!(accent.specular, None);

            // The L2 elevation stays: the pane is still chrome.
            assert_eq!(surface.shadow, theme.shadows().l2);
        }
    }

    /// The specular hairline runs 1px down from the pane's top,
    /// spanning between the corner rounds so it blends into the shape.
    #[test]
    fn specular_hairline_sits_inside_the_top_edge() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::Vec2::new(120.0, 40.0));
        let [from, to] = specular_line(rect, 8.0);
        assert_eq!(from, Pos2::new(18.0, 21.0));
        assert_eq!(to, Pos2::new(122.0, 21.0));
    }

    /// The pane paints headless without touching the output deltas:
    /// shadow, fill and hairline all go through the painter.
    #[test]
    fn pane_paints_headless() {
        let ctx = egui::Context::default();
        let flags = UiKitFlags::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let recipe = LiquidGlass::new().recipe(ThemeId::Light, &flags);
            assert_eq!(recipe.fill.a(), 209, "the light surface tint's alpha");
            paint_glass(
                ui.painter(),
                Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(100.0, 30.0)),
                8,
                &recipe,
            );
        })
        .drop_without_applying_deltas();
    }

    /// A pane with no specular (the solid fallback) paints without the
    /// hairline and stays fully opaque.
    #[test]
    fn solid_pane_paints_without_the_edge() {
        let ctx = egui::Context::default();
        let flags = UiKitFlags {
            reduced_transparency: true,
            ..Default::default()
        };
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let recipe = LiquidGlass::new().recipe(ThemeId::Dark, &flags);
            assert_eq!(recipe.fill, ThemeId::Dark.colors().surface_container);
            assert!(recipe.fill.is_opaque());
            paint_glass(
                ui.painter(),
                Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(100.0, 30.0)),
                8,
                &recipe,
            );
        })
        .drop_without_applying_deltas();
    }

    /// The pane widget lays its content out headless in every theme,
    /// every thickness and every tint, without touching the output
    /// deltas.
    #[test]
    fn pane_widget_paints_headless() {
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            let flags = UiKitFlags::default();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                for thickness in GlassThickness::ALL {
                    for tint in GlassTint::ALL {
                        LiquidGlass::new().thickness(*thickness).tint(*tint).ui(
                            ui,
                            *theme,
                            &flags,
                            |ui| {
                                ui.label("Sheets, popovers, modal headers");
                            },
                        );
                    }
                }
            })
            .drop_without_applying_deltas();
        }
    }
}

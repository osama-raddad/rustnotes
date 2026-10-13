//! The six Vynatix themes: ids, display names, resolved colour roles and
//! their mapping onto egui [`Visuals`], following the pattern of
//! `rustnotes-core`'s `layout::Palette`.

use egui::{Color32, Context, CornerRadius, Stroke, Theme, ThemePreference, Visuals};

use crate::tokens::{radii, shadows, ThemeColors};

/// The six themes of the design system, one per `color.themes` entry in
/// `tokens.json`: Forest Heritage, Transformative Teal and Stockholm
/// Monochrome, light and dark each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    /// Forest Heritage light, the default canvas (Warm Cream).
    Light,
    /// Forest Heritage dark.
    Dark,
    /// Transformative Teal light (Cloud Dancer canvas).
    TealLight,
    /// Transformative Teal dark (Ink Teal-Black canvas).
    TealDark,
    /// Stockholm Monochrome + Bronze light (Paper canvas).
    MonoLight,
    /// Stockholm Monochrome + Bronze dark (Obsidian canvas).
    MonoDark,
}

impl ThemeId {
    /// Every theme, in `tokens.json` (`color.themes`) order.
    pub const ALL: &'static [ThemeId] = &[
        ThemeId::Light,
        ThemeId::Dark,
        ThemeId::TealLight,
        ThemeId::TealDark,
        ThemeId::MonoLight,
        ThemeId::MonoDark,
    ];

    /// The theme's `tokens.json` id, the value the design system's
    /// `data-theme` attribute takes ("light", "teal-dark", ...).
    pub const fn id(self) -> &'static str {
        match self {
            ThemeId::Light => "light",
            ThemeId::Dark => "dark",
            ThemeId::TealLight => "teal-light",
            ThemeId::TealDark => "teal-dark",
            ThemeId::MonoLight => "mono-light",
            ThemeId::MonoDark => "mono-dark",
        }
    }

    /// The display name from `tokens.json` ("Forest Light", ...).
    pub const fn name(self) -> &'static str {
        match self {
            ThemeId::Light => "Forest Light",
            ThemeId::Dark => "Forest Dark",
            ThemeId::TealLight => "Teal Light",
            ThemeId::TealDark => "Teal Dark",
            ThemeId::MonoLight => "Monochrome Light",
            ThemeId::MonoDark => "Monochrome Dark",
        }
    }

    /// True for the three dark themes.
    pub const fn is_dark(self) -> bool {
        matches!(self, ThemeId::Dark | ThemeId::TealDark | ThemeId::MonoDark)
    }

    /// The theme's 54 resolved colour roles.
    pub const fn colors(self) -> ThemeColors {
        match self {
            ThemeId::Light => ThemeColors::LIGHT,
            ThemeId::Dark => ThemeColors::DARK,
            ThemeId::TealLight => ThemeColors::TEAL_LIGHT,
            ThemeId::TealDark => ThemeColors::TEAL_DARK,
            ThemeId::MonoLight => ThemeColors::MONO_LIGHT,
            ThemeId::MonoDark => ThemeColors::MONO_DARK,
        }
    }

    /// The theme's shadow scale.
    pub const fn shadows(self) -> shadows::ThemeShadows {
        match self {
            ThemeId::Light => shadows::LIGHT,
            ThemeId::Dark => shadows::DARK,
            ThemeId::TealLight => shadows::TEAL_LIGHT,
            ThemeId::TealDark => shadows::TEAL_DARK,
            ThemeId::MonoLight => shadows::MONO_LIGHT,
            ThemeId::MonoDark => shadows::MONO_DARK,
        }
    }

    /// The egui light/dark slot the theme lives under.
    const fn slot(self) -> Theme {
        if self.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    /// egui [`Visuals`] skinned from the theme's roles: canvas, text
    /// colours, selection, hyperlink, widget fills and shadows.
    pub fn visuals(self) -> Visuals {
        let c = self.colors();
        let mut v = if self.is_dark() {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        v.dark_mode = self.is_dark();
        // The canvas is the hero: panels take the surface token, windows
        // and faint fills rest on surface-container-low, code on -high.
        v.panel_fill = c.surface;
        v.window_fill = c.surface_container_low;
        v.faint_bg_color = c.surface_container_low;
        v.extreme_bg_color = if self.is_dark() {
            c.surface_container_highest
        } else {
            c.surface_bright
        };
        v.code_bg_color = c.surface_container_high;
        // Text: body from on-surface, muted from on-surface-variant.
        v.override_text_color = Some(c.on_surface);
        v.weak_text_color = Some(c.on_surface_variant);
        // Links hover in the primary family, one step deeper or lighter
        // than the primary itself.
        v.hyperlink_color = c.primary_hover;
        v.error_fg_color = c.error;
        v.warn_fg_color = c.warning;
        // Selection carries the design system's selection pair.
        v.selection.bg_fill = c.selection_bg;
        v.selection.stroke = Stroke::new(1.0, c.selection_fg);
        v.text_cursor.stroke.color = c.primary;
        // Radii: the default widget radius is md, windows and menus xl/md.
        let radius_md = CornerRadius::same(radii::RADIUS_MD as u8);
        v.window_corner_radius = CornerRadius::same(radii::RADIUS_XL as u8);
        v.menu_corner_radius = radius_md;
        v.window_stroke.color = c.outline_variant;
        // Widgets: rest on the container scale, hover one step up, press
        // fills with the primary (text on-primary).
        v.widgets.noninteractive.bg_fill = c.surface;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.outline_variant);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.on_surface_variant);
        v.widgets.noninteractive.corner_radius = radius_md;
        v.widgets.inactive.bg_fill = c.surface_container_low;
        v.widgets.inactive.weak_bg_fill = c.surface_container_low;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, c.outline);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, c.on_surface);
        v.widgets.inactive.corner_radius = radius_md;
        v.widgets.hovered.bg_fill = c.surface_container;
        v.widgets.hovered.weak_bg_fill = c.surface_container;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.outline);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, c.primary_hover);
        v.widgets.hovered.corner_radius = radius_md;
        v.widgets.active.bg_fill = c.primary;
        v.widgets.active.weak_bg_fill = c.primary;
        v.widgets.active.bg_stroke = Stroke::new(1.0, c.primary);
        v.widgets.active.fg_stroke = Stroke::new(1.0, c.on_primary);
        v.widgets.active.corner_radius = radius_md;
        v.widgets.open.bg_fill = c.surface_container_high;
        v.widgets.open.weak_bg_fill = c.surface_container_high;
        v.widgets.open.bg_stroke = Stroke::new(1.0, c.outline);
        v.widgets.open.fg_stroke = Stroke::new(1.0, c.on_surface);
        v.widgets.open.corner_radius = radius_md;
        v.window_shadow = egui_shadow(self.shadows().l2);
        v.popup_shadow = egui_shadow(self.shadows().l1);
        v
    }

    /// Install the theme on a context: the visuals go into the theme's
    /// light/dark slot and the theme preference switches to it, so a
    /// live switch re-skins immediately. The id is remembered for
    /// [`ThemeId::current`], the role lookup the kit's widgets use when
    /// they colour themselves from the live theme.
    pub fn apply(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(theme_key(), self));
        ctx.set_theme(ThemePreference::from(self.slot()));
        ctx.set_visuals_of(self.slot(), self.visuals());
    }

    /// The theme [`apply`](Self::apply) last installed on the context:
    /// `None` before the first apply. The kit's theme-following widgets
    /// (the logo) resolve their default colours through this rather
    /// than taking a theme parameter.
    pub fn current(ctx: &Context) -> Option<ThemeId> {
        ctx.data(|data| data.get_temp(theme_key()))
    }
}

/// The context-data key [`ThemeId::apply`] stores the live theme under.
fn theme_key() -> egui::Id {
    egui::Id::new("ui-kit theme")
}

/// One CSS layer set as a single egui shadow: egui paints one layer with
/// unsigned blur/spread, so the widest layer stands in for the set and a
/// negative spread (the monochrome shrink) clamps away. The kit's own
/// visuals and the gallery's elevation specimens both go through here.
pub fn egui_shadow(layers: &[shadows::ShadowLayer]) -> egui::Shadow {
    let widest = layers
        .iter()
        .max_by_key(|layer| layer.blur.round() as u32)
        .copied()
        .unwrap_or(shadows::ShadowLayer {
            inset: false,
            x: 0.0,
            y: 0.0,
            blur: 0.0,
            spread: 0.0,
            color: Color32::TRANSPARENT,
        });
    egui::Shadow {
        offset: [
            widest.x.round().clamp(-128.0, 127.0) as i8,
            widest.y.round().clamp(-128.0, 127.0) as i8,
        ],
        blur: widest.blur.round().clamp(0.0, 255.0) as u8,
        spread: widest.spread.round().clamp(0.0, 255.0) as u8,
        color: widest.color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ThemeId::ALL` carries all six themes in `tokens.json` order, each
    /// reporting its `tokens.json` id and display name.
    #[test]
    fn all_lists_six_themes_in_tokens_json_order() {
        let expected: &[(&str, &str)] = &[
            ("light", "Forest Light"),
            ("dark", "Forest Dark"),
            ("teal-light", "Teal Light"),
            ("teal-dark", "Teal Dark"),
            ("mono-light", "Monochrome Light"),
            ("mono-dark", "Monochrome Dark"),
        ];
        assert_eq!(
            ThemeId::ALL.len(),
            expected.len(),
            "all six themes, in tokens.json order"
        );
        for (theme, (id, name)) in ThemeId::ALL.iter().zip(expected) {
            assert_eq!(theme.id(), *id);
            assert_eq!(theme.name(), *name);
        }
    }

    /// Dark flags mark exactly the dark themes.
    #[test]
    fn dark_flags_mark_exactly_the_dark_themes() {
        for theme in ThemeId::ALL {
            assert_eq!(theme.is_dark(), theme.id().ends_with("dark"));
        }
    }

    /// The visuals are skinned from the theme's own roles: the canvas is
    /// the surface token, text comes from the text roles, links hover and
    /// selection uses the selection pair.
    #[test]
    fn visuals_are_skinned_from_the_roles() {
        for theme in ThemeId::ALL {
            let c = theme.colors();
            let v = theme.visuals();
            let name = theme.name();
            assert_eq!(v.dark_mode, theme.is_dark(), "{name}: dark flag");
            assert_eq!(v.panel_fill, c.surface, "{name}: the canvas is surface");
            assert_eq!(v.window_fill, c.surface_container_low, "{name}: windows");
            assert_eq!(
                v.code_bg_color, c.surface_container_high,
                "{name}: code backgrounds"
            );
            assert_eq!(
                v.override_text_color,
                Some(c.on_surface),
                "{name}: body text"
            );
            assert_eq!(
                v.weak_text_color,
                Some(c.on_surface_variant),
                "{name}: muted text"
            );
            assert_eq!(
                v.hyperlink_color, c.primary_hover,
                "{name}: links shift to primary-hover"
            );
            assert_eq!(
                v.selection.bg_fill, c.selection_bg,
                "{name}: selection fill"
            );
            assert_eq!(v.error_fg_color, c.error, "{name}: error text");
        }
    }

    /// `apply` puts the theme's visuals in the right egui slot, makes
    /// it the live theme, and remembers its id: `current` reports the
    /// last applied theme, `None` on a fresh context and after each
    /// apply the one that went in.
    #[test]
    fn apply_installs_the_theme_on_the_context() {
        let ctx = egui::Context::default();
        assert_eq!(ThemeId::current(&ctx), None, "fresh context carries none");
        for theme in ThemeId::ALL {
            theme.apply(&ctx);
            assert_eq!(ctx.theme(), theme.slot(), "{}: live slot", theme.name());
            assert_eq!(
                ctx.global_style().visuals.panel_fill,
                theme.colors().surface,
                "{}: canvas applied",
                theme.name()
            );
            assert_eq!(
                ThemeId::current(&ctx),
                Some(*theme),
                "{}: remembered",
                theme.name()
            );
        }
    }
}

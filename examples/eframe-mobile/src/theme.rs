//! Theme management on top of egui's own `Visuals`.
//!
//! Why not egui-thematic: as of 0.1.1 it is compiled against egui 0.33 (two
//! egui versions in one binary means its `Visuals` is a different type from
//! eframe 0.36's) and it depends on `rfd` for file dialogs, which has no
//! Android backend. When a release targets egui 0.36 without `rfd`, swap
//! `Theme::visuals()` for `ThemeConfig::to_visuals()`; nothing else changes.
//!
//! Each preset is a named `Visuals`. Applying one is `ctx.set_visuals`,
//! which is cheap but still rebuilds style state, so `App` applies it only
//! when the selection changes, never per frame.

use egui::{Color32, Visuals};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    Light,
    Dark,
    /// Follow the OS setting reported by winit (eframe forwards it).
    System,
    Nord,
    CatppuccinMocha,
}

impl Theme {
    pub const ALL: &[Theme] = &[
        Theme::System,
        Theme::Light,
        Theme::Dark,
        Theme::Nord,
        Theme::CatppuccinMocha,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::Nord => "Nord",
            Theme::CatppuccinMocha => "Catppuccin Mocha",
        }
    }

    /// Apply this theme to the context. `System` hands control back to
    /// egui's theme preference so OS dark/light switches are followed live.
    pub fn apply(self, ctx: &egui::Context) {
        match self {
            Theme::System => ctx.set_theme(egui::ThemePreference::System),
            other => {
                ctx.set_theme(egui::ThemePreference::from(other.base()));
                ctx.set_visuals(other.visuals());
            }
        }
    }

    /// Whether the preset is fundamentally a dark or light scheme.
    fn base(self) -> egui::Theme {
        match self {
            Theme::Light => egui::Theme::Light,
            _ => egui::Theme::Dark,
        }
    }

    fn visuals(self) -> Visuals {
        match self {
            Theme::Light => Visuals::light(),
            Theme::Dark | Theme::System => Visuals::dark(),
            Theme::Nord => palette(
                Visuals::dark(),
                Color32::from_rgb(0x2e, 0x34, 0x40), // polar night
                Color32::from_rgb(0x3b, 0x42, 0x52),
                Color32::from_rgb(0xd8, 0xde, 0xe9), // snow storm
                Color32::from_rgb(0x88, 0xc0, 0xd0), // frost
            ),
            Theme::CatppuccinMocha => palette(
                Visuals::dark(),
                Color32::from_rgb(0x1e, 0x1e, 0x2e), // base
                Color32::from_rgb(0x31, 0x32, 0x44), // surface0
                Color32::from_rgb(0xcd, 0xd6, 0xf4), // text
                Color32::from_rgb(0x89, 0xb4, 0xfa), // blue
            ),
        }
    }
}

/// Recolour a base `Visuals` with a four-colour palette: window background,
/// widget surface, text, accent. Enough for a coherent look; extend the
/// mapping here when a design system asks for more.
fn palette(mut v: Visuals, bg: Color32, surface: Color32, text: Color32, accent: Color32) -> Visuals {
    v.panel_fill = bg;
    v.window_fill = bg;
    v.extreme_bg_color = surface;
    v.faint_bg_color = surface;
    v.override_text_color = Some(text);
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent.gamma_multiply(0.4);
    v.widgets.noninteractive.bg_fill = surface;
    v.widgets.inactive.bg_fill = surface;
    v.widgets.inactive.weak_bg_fill = surface;
    v.widgets.hovered.bg_fill = accent.gamma_multiply(0.5);
    v.widgets.hovered.weak_bg_fill = accent.gamma_multiply(0.5);
    v.widgets.active.bg_fill = accent;
    v.widgets.active.weak_bg_fill = accent;
    v
}

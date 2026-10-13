use egui::{Color32, Context, Theme, Visuals};

/// Colour tokens. One light and one dark instance; screens read tokens, never
/// literal colours.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Screen background behind cards.
    pub bg: Color32,
    /// Editor / card surface.
    pub surface: Color32,
    /// Top and bottom bars.
    pub bar: Color32,
    pub text: Color32,
    pub text_weak: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub danger_bg: Color32,
    pub hairline: Color32,
    /// Selected row in a list (two-pane mode).
    pub selected: Color32,
}

impl Palette {
    pub const LIGHT: Palette = Palette {
        bg: Color32::from_rgb(246, 246, 248),
        surface: Color32::WHITE,
        bar: Color32::WHITE,
        text: Color32::from_rgb(26, 28, 33),
        text_weak: Color32::from_rgb(107, 112, 128),
        text_faint: Color32::from_rgb(158, 163, 176),
        accent: Color32::from_rgb(53, 116, 244),
        on_accent: Color32::WHITE,
        danger: Color32::from_rgb(212, 48, 48),
        danger_bg: Color32::from_rgb(251, 232, 232),
        hairline: Color32::from_rgb(230, 230, 237),
        selected: Color32::from_rgb(232, 240, 254),
    };

    pub const DARK: Palette = Palette {
        bg: Color32::from_rgb(16, 17, 20),
        surface: Color32::from_rgb(28, 30, 35),
        bar: Color32::from_rgb(22, 23, 27),
        text: Color32::from_rgb(232, 234, 240),
        text_weak: Color32::from_rgb(160, 165, 180),
        text_faint: Color32::from_rgb(110, 115, 130),
        accent: Color32::from_rgb(110, 156, 255),
        on_accent: Color32::from_rgb(10, 20, 45),
        danger: Color32::from_rgb(240, 98, 98),
        danger_bg: Color32::from_rgb(64, 32, 34),
        hairline: Color32::from_rgb(44, 47, 55),
        selected: Color32::from_rgb(34, 46, 74),
    };

    pub fn for_theme(theme: Theme) -> Palette {
        match theme {
            Theme::Light => Self::LIGHT,
            Theme::Dark => Self::DARK,
        }
    }

    /// The palette for the context's currently resolved theme.
    pub fn of(ctx: &Context) -> Palette {
        Self::for_theme(ctx.theme())
    }

    fn visuals(&self, theme: Theme) -> Visuals {
        let mut v = match theme {
            Theme::Light => Visuals::light(),
            Theme::Dark => Visuals::dark(),
        };
        v.panel_fill = self.bg;
        v.window_fill = self.surface;
        v.extreme_bg_color = Color32::TRANSPARENT;
        v.override_text_color = Some(self.text);
        v.weak_text_color = Some(self.text_faint);
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = self.accent.gamma_multiply(0.35);
        v.selection.stroke.color = self.accent;
        v.text_cursor.stroke.color = self.accent;
        v.text_cursor.stroke.width = 2.0;
        v
    }

    /// Write both themes' visuals so a live light/dark switch is correct.
    /// Call once at startup.
    pub fn apply_all(ctx: &Context) {
        ctx.set_visuals_of(Theme::Light, Self::LIGHT.visuals(Theme::Light));
        ctx.set_visuals_of(Theme::Dark, Self::DARK.visuals(Theme::Dark));
    }
}

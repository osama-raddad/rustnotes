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
    /// Most de-emphasised text (hints, empty states). Kept at ≥3:1 on every
    /// background it lands on (bg, surface, selected); see the contrast tests
    /// below.
    pub text_faint: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub danger_bg: Color32,
    pub hairline: Color32,
    /// Highlighted/selected surface.
    // TODO(framework-extraction): callers use this for chosen rows and
    // selected text; keep the token generic ("selected surface") when the
    // app concepts move out.
    pub selected: Color32,
}

impl Palette {
    pub const LIGHT: Palette = Palette {
        bg: Color32::from_rgb(246, 246, 248),
        surface: Color32::WHITE,
        bar: Color32::WHITE,
        text: Color32::from_rgb(26, 28, 33),
        text_weak: Color32::from_rgb(102, 107, 123),
        text_faint: Color32::from_rgb(130, 136, 150),
        accent: Color32::from_rgb(49, 107, 224),
        on_accent: Color32::WHITE,
        danger: Color32::from_rgb(204, 44, 44),
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
        text_faint: Color32::from_rgb(120, 125, 140),
        accent: Color32::from_rgb(110, 156, 255),
        on_accent: Color32::from_rgb(10, 20, 45),
        danger: Color32::from_rgb(246, 112, 112),
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
        // Name-match the slot: egui's weak text takes the weak token, not
        // the faintest one.
        v.weak_text_color = Some(self.text_weak);
        v.hyperlink_color = self.accent;
        // egui recolours selected text itself: the glyphs take
        // `selection.stroke.color` and sit on `selection.bg_fill`
        // (`text_selection/visuals.rs`). Selected glyphs keep the text
        // colour, as on iOS and Android, so the fill only has to sit well
        // under the text: the light theme takes the light `selected`
        // surface, the dark theme a darkened accent. That one is translucent
        // (`gamma_multiply` scales alpha too), so whatever it is painted
        // over lifts it. The editor has no frame (`Frame::NONE`), so no
        // focus outline reads this stroke.
        v.selection.bg_fill = match theme {
            Theme::Light => self.selected,
            Theme::Dark => self.accent.gamma_multiply(0.35),
        };
        v.selection.stroke.color = self.text;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2.x relative luminance of one channel.
    fn channel(v: u8) -> f64 {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(c: Color32) -> f64 {
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    /// WCAG contrast ratio (1..=21, bigger is better).
    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    fn at_least(fg: Color32, bg: Color32, ratio: f64, what: &str) {
        let got = contrast(fg, bg);
        assert!(
            got >= ratio,
            "{what}: contrast {got:.2} < {ratio} for {fg:?} on {bg:?}"
        );
    }

    /// `fill` laid over an opaque `backdrop` with the usual source-over
    /// blend. egui stores colours premultiplied, so the fill's own RGB is
    /// already scaled by its alpha. The dark selection fill is translucent:
    /// its raw RGB is far darker than what reaches the screen, so contrast
    /// has to be measured after this blend, not against the raw value.
    fn over(fill: Color32, backdrop: Color32) -> Color32 {
        let keep = 1.0 - f32::from(fill.a()) / 255.0;
        let mix = |f: u8, b: u8| (f32::from(f) + f32::from(b) * keep).round().min(255.0) as u8;
        Color32::from_rgb(
            mix(fill.r(), backdrop.r()),
            mix(fill.g(), backdrop.g()),
            mix(fill.b(), backdrop.b()),
        )
    }

    /// Every text token must be readable on the backgrounds it lands on:
    /// body-level tokens at WCAG AA (4.5:1), de-emphasised and accent pairings
    /// at the large-text/UI minimum (3:1). The `selected` row (the chosen note in
    /// the two-pane list) holds `text`, `text_weak` and `danger` to the same AA
    /// bar as the plain surfaces, and `text_faint` to its 3:1. `danger` also
    /// writes caption-size text (the storage-error strip, the armed-delete
    /// hint) and fills the swipe-to-delete reveal under an `on_accent` label,
    /// so those pairs are held to AA too. So is `accent`, which is text as
    /// well as fill (the active bottom-nav label, links): AA on `bg`,
    /// `surface` and `bar`, and AA for the `on_accent` labels that sit on it.
    #[test]
    fn text_tokens_read_on_their_backgrounds() {
        for pal in [Palette::LIGHT, Palette::DARK] {
            at_least(pal.text, pal.bg, 4.5, "text on bg");
            at_least(pal.text, pal.surface, 4.5, "text on surface");
            at_least(pal.text, pal.selected, 4.5, "text on selected");
            at_least(pal.text_weak, pal.bg, 4.5, "weak text on bg");
            at_least(pal.text_weak, pal.surface, 4.5, "weak text on surface");
            at_least(pal.text_weak, pal.selected, 4.5, "weak text on selected");
            at_least(pal.text_faint, pal.bg, 3.0, "faint text on bg");
            at_least(pal.text_faint, pal.surface, 3.0, "faint text on surface");
            at_least(pal.text_faint, pal.selected, 3.0, "faint text on selected");
            at_least(pal.on_accent, pal.accent, 4.5, "on_accent on accent");
            at_least(pal.accent, pal.bg, 4.5, "accent text on bg");
            at_least(pal.accent, pal.surface, 4.5, "accent text on surface");
            at_least(pal.accent, pal.bar, 4.5, "accent text on bar");
            at_least(pal.on_accent, pal.danger, 4.5, "on_accent on danger");
            at_least(pal.danger, pal.danger_bg, 3.0, "danger on danger_bg");
            at_least(pal.danger, pal.bg, 4.5, "danger text on bg");
            at_least(pal.danger, pal.surface, 4.5, "danger text on surface");
            at_least(pal.danger, pal.selected, 4.5, "danger text on selected");
        }
    }

    /// egui recolours selected text itself: the glyphs take
    /// `selection.stroke.color` and sit on `selection.bg_fill`
    /// (`text_selection/visuals.rs`). Selected glyphs keep the text colour, as
    /// on iOS and Android, so that pair has to clear AA in both themes. The
    /// fill is measured as the user sees it, composited over each backdrop it
    /// can land on: the dark fill is translucent. (Accent glyphs on these
    /// fills scored only 3.3 to 3.7:1.)
    #[test]
    fn selected_text_stays_readable_in_both_themes() {
        for (pal, theme) in [(Palette::LIGHT, Theme::Light), (Palette::DARK, Theme::Dark)] {
            let v = pal.visuals(theme);
            for (backdrop, name) in [(pal.bg, "bg"), (pal.surface, "surface")] {
                let fill = over(v.selection.bg_fill, backdrop);
                let what = format!("{theme:?} selection over {name}");
                at_least(
                    v.selection.stroke.color,
                    fill,
                    4.5,
                    &format!("{what}: glyphs"),
                );
            }
            assert_eq!(
                v.selection.stroke.color, pal.text,
                "{theme:?}: selected glyphs keep the text colour"
            );
        }
    }

    /// The weak-text slot is fed by the weak token, not the faintest one.
    #[test]
    fn weak_text_slot_uses_the_weak_token() {
        for (pal, theme) in [(Palette::LIGHT, Theme::Light), (Palette::DARK, Theme::Dark)] {
            assert_eq!(pal.visuals(theme).weak_text_color, Some(pal.text_weak));
        }
    }
}

use egui::{Context, FontFamily, FontId, TextStyle, Vec2};

/// Type sizes in points.
#[derive(Clone, Copy, Debug)]
pub struct TextScale {
    pub caption: f32,
    pub body: f32,
    pub body_large: f32,
    pub title: f32,
    pub headline: f32,
}

/// Layout constants, in points. One table for every screen.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// Spacing scale: 4, 8, 12, 16, 24.
    pub space: [f32; 5],
    pub bar_height: f32,
    /// Minimum touch target. 48 satisfies Material and the iOS HIG (44).
    pub touch_target: f32,
    pub fab_size: f32,
    pub corner: f32,
    pub text: TextScale,
    /// Body width cap in Medium/Expanded windows so lines stay readable.
    pub max_content_width: f32,
}

impl Metrics {
    pub const DEFAULT: Metrics = Metrics {
        space: [4.0, 8.0, 12.0, 16.0, 24.0],
        bar_height: 56.0,
        touch_target: 48.0,
        fab_size: 56.0,
        corner: 10.0,
        text: TextScale {
            caption: 11.0,
            body: 14.0,
            body_large: 16.0,
            title: 17.0,
            headline: 22.0,
        },
        max_content_width: 640.0,
    };

    /// `space[i]`, clamped to the scale.
    pub fn sp(&self, i: usize) -> f32 {
        self.space[i.min(self.space.len() - 1)]
    }

    pub fn font(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }

    /// Write the touch target and type scale into egui's styles. Call once
    /// at startup, not per frame.
    pub fn apply(&self, ctx: &Context) {
        let m = *self;
        ctx.all_styles_mut(move |s| {
            s.spacing.interact_size = Vec2::new(m.touch_target, m.touch_target);
            s.spacing.button_padding = Vec2::new(m.sp(3), m.sp(1));
            s.text_styles.insert(TextStyle::Small, m.font(m.text.caption));
            s.text_styles.insert(TextStyle::Body, m.font(m.text.body_large));
            s.text_styles.insert(TextStyle::Button, m.font(m.text.body_large));
            s.text_styles.insert(TextStyle::Heading, m.font(m.text.headline));
        });
    }
}

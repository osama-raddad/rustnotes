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
    ///
    /// An out-of-scale index is a typo somewhere, so it is loud in debug
    /// builds; release builds keep the silent clamp.
    pub fn sp(&self, i: usize) -> f32 {
        debug_assert!(
            i < self.space.len(),
            "space index {i} is off the scale (0..={})",
            self.space.len() - 1
        );
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
            s.text_styles
                .insert(TextStyle::Small, m.font(m.text.caption));
            s.text_styles
                .insert(TextStyle::Body, m.font(m.text.body_large));
            s.text_styles
                .insert(TextStyle::Button, m.font(m.text.body_large));
            s.text_styles
                .insert(TextStyle::Heading, m.font(m.text.headline));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sp_reads_the_scale() {
        let m = Metrics::DEFAULT;
        assert_eq!(m.sp(0), 4.0);
        assert_eq!(m.sp(1), 8.0);
        assert_eq!(m.sp(2), 12.0);
        assert_eq!(m.sp(3), 16.0);
        assert_eq!(m.sp(4), 24.0);
    }

    /// Off-scale indices assert in debug builds and clamp in release ones.
    #[test]
    fn off_scale_index_is_loud_in_debug() {
        if cfg!(debug_assertions) {
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let panicked = std::panic::catch_unwind(|| Metrics::DEFAULT.sp(7));
            std::panic::set_hook(prev);
            assert!(panicked.is_err(), "sp(7) must debug-assert");
        } else {
            assert_eq!(Metrics::DEFAULT.sp(7), 24.0, "release clamps to the top");
        }
    }
}

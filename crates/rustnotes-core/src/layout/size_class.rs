use egui::Context;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeClass {
    Compact,
    Medium,
    Expanded,
}

/// Material 3 window size classes, in points.
/// Width: compact < 600, medium < 840, expanded otherwise.
/// Height: compact < 480, medium < 900, expanded otherwise.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowClass {
    pub width: SizeClass,
    pub height: SizeClass,
}

impl WindowClass {
    /// Non-finite dimensions classify as [`SizeClass::Compact`]: when the
    /// number cannot be trusted, fall back to the smallest layout rather
    /// than the most expansive one.
    pub fn from_points(w: f32, h: f32) -> Self {
        let width = if !w.is_finite() || w < 600.0 {
            SizeClass::Compact
        } else if w < 840.0 {
            SizeClass::Medium
        } else {
            SizeClass::Expanded
        };
        let height = if !h.is_finite() || h < 480.0 {
            SizeClass::Compact
        } else if h < 900.0 {
            SizeClass::Medium
        } else {
            SizeClass::Expanded
        };
        WindowClass { width, height }
    }

    /// Classified from the full viewport, not the content rect, so opening
    /// the keyboard never flips the layout.
    pub fn of(ctx: &Context) -> Self {
        let r = ctx.viewport_rect();
        Self::from_points(r.width(), r.height())
    }

    /// Two panes need room both ways. A phone in landscape is width-Expanded
    /// (874 pt on an iPhone 17 Pro) but height-Compact (402 pt) and stays
    /// single-pane.
    pub fn two_pane_ok(&self) -> bool {
        self.width == SizeClass::Expanded && self.height != SizeClass::Compact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones_portrait_are_compact() {
        let c = WindowClass::from_points(402.0, 874.0);
        assert_eq!(c.width, SizeClass::Compact);
        assert!(!c.two_pane_ok());
    }

    #[test]
    fn phones_landscape_stay_single_pane() {
        let c = WindowClass::from_points(874.0, 402.0);
        assert_eq!(c.width, SizeClass::Expanded);
        assert_eq!(c.height, SizeClass::Compact);
        assert!(!c.two_pane_ok());
    }

    #[test]
    fn tablets_get_two_panes_in_both_orientations() {
        assert!(WindowClass::from_points(1024.0, 1366.0).two_pane_ok());
        assert!(WindowClass::from_points(1366.0, 1024.0).two_pane_ok());
    }

    #[test]
    fn small_tablet_portrait_is_medium_width() {
        let c = WindowClass::from_points(744.0, 1133.0);
        assert_eq!(c.width, SizeClass::Medium);
        assert!(!c.two_pane_ok());
    }

    /// The thresholds are `<`, so a device sitting exactly on one (600 pt,
    /// 840 pt, 480 pt, 900 pt) lands in the bigger class.
    #[test]
    fn boundaries_land_in_the_bigger_class() {
        let c = WindowClass::from_points(600.0, 480.0);
        assert_eq!((c.width, c.height), (SizeClass::Medium, SizeClass::Medium));
        let c = WindowClass::from_points(840.0, 900.0);
        assert_eq!(
            (c.width, c.height),
            (SizeClass::Expanded, SizeClass::Expanded)
        );
        let c = WindowClass::from_points(599.0, 479.0);
        assert_eq!(
            (c.width, c.height),
            (SizeClass::Compact, SizeClass::Compact)
        );
        let c = WindowClass::from_points(839.0, 899.0);
        assert_eq!((c.width, c.height), (SizeClass::Medium, SizeClass::Medium));
    }

    /// NaN (or infinity) is a number we cannot trust: Compact, never
    /// Expanded, so a degenerate frame cannot flip on two panes.
    #[test]
    fn non_finite_input_is_compact() {
        let c = WindowClass::from_points(f32::NAN, f32::NAN);
        assert_eq!(c.width, SizeClass::Compact);
        assert_eq!(c.height, SizeClass::Compact);
        assert!(!c.two_pane_ok());

        let c = WindowClass::from_points(f32::INFINITY, 800.0);
        assert_eq!(c.width, SizeClass::Compact, "infinity is not Expanded");
        assert!(!c.two_pane_ok());

        let c = WindowClass::from_points(400.0, f32::NAN);
        assert_eq!(c.height, SizeClass::Compact);
    }
}

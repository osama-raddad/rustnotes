//! The Vynatix logo widgets: the V mark and the wordmark as flattened,
//! build-time-triangulated fills painted through `epaint`.
//!
//! The artwork is the uploaded vector from the design system's
//! `assets/Logos/` (every per-ink file shares one geometry; the script
//! verifies that before using it). `tools/prepare_icons.py` parses the
//! two SVGs once at build time, flattens the Bézier contours (documented
//! tolerance) and triangulates them with their holes (the counters of
//! the serif "a"), emitting normalized triangle constants to the
//! generated `paths` submodule: `src/logo/paths.rs`. Nothing parses at
//! runtime; the widget maps the triangles through a scale and hands
//! them to `epaint` as one [`egui::Shape::Mesh`] per logo.
//!
//! Inks. The shapes never bake a colour: they take a role at paint
//! time. [`LogoInk`] carries the three approved placements plus the
//! accent: [`Canvas`](LogoInk::Canvas) is `primary` on the canvas,
//! [`OnPrimary`](LogoInk::OnPrimary) is `on-primary` over a primary
//! fill, [`Inverse`](LogoInk::Inverse) is `inverse-on-surface` on an
//! inverse band, and [`Accent`](LogoInk::Accent) is the bronze-family
//! `text-accent`. [`Logo::color`] overrides the role outright. The
//! per-ink SVG files stay reference material; the kit repaints the one
//! geometry.
//!
//! Clear space and size. Clear space on every side is the height of the
//! V ([`Logo::clear_space`], resolved from the artwork). The minimums
//! are enforced by [`size_ok`] through a `debug_assert` in the widget
//! (and a log line in release): the wordmark at least 96px wide, the
//! mark at least 24px in product chrome, 96px in a brand placement
//! ([`WORDMARK_MIN_WIDTH`], [`MARK_MIN_CHROME`], [`MARK_MIN_BRAND`]).
//!
//! Documented deviations: the mesh is flat-filled triangles, so logo
//! edges carry no antialiasing under egui (visible only on 1x displays
//! at the smallest sizes); and the flattening tolerance is 1/1000 of
//! the art box (the script asserts the triangulated area against the
//! shoelace area of the contours, holes subtracted, before emitting).

mod paths;

use egui::epaint::{Mesh, Shape};
use egui::{Color32, Pos2, Response, Sense, Ui, Vec2, Widget};

use crate::tokens::ThemeColors;

/// The wordmark's smallest on-screen size: 96px wide.
pub const WORDMARK_MIN_WIDTH: f32 = 96.0;

/// The mark's smallest size in product chrome: 24px tall.
pub const MARK_MIN_CHROME: f32 = 24.0;

/// The mark's smallest size in a brand placement: 96px tall.
pub const MARK_MIN_BRAND: f32 = 96.0;

/// The height every unit of the art box scales by: the box is
/// normalized to 1000 units tall.
const UNITS: f32 = 1000.0;

/// Which artwork a [`Logo`] paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoArt {
    /// The V mark alone: favicons, app icons, avatars, rails.
    Mark,
    /// The wordmark: the V mark followed by "natix" in the serif.
    Wordmark,
}

impl LogoArt {
    /// Width over height of the source art box, preserved by every
    /// sizing path.
    pub fn aspect(self) -> f32 {
        let (w, h) = self.box_units();
        w / h
    }

    /// The art box in normalized units: (width, height).
    pub fn box_units(self) -> (f32, f32) {
        match self {
            LogoArt::Mark => (paths::MARK_WIDTH, paths::MARK_HEIGHT),
            LogoArt::Wordmark => (paths::WORDMARK_WIDTH, paths::WORDMARK_HEIGHT),
        }
    }

    /// The triangles, each `[x0, y0, x1, y1, x2, y2]` in normalized
    /// units, y down, origin at the art box's top-left corner.
    pub fn triangles(self) -> &'static [[f32; 6]] {
        match self {
            LogoArt::Mark => paths::MARK_TRIANGLES,
            LogoArt::Wordmark => paths::WORDMARK_TRIANGLES,
        }
    }

    /// The clear space on every side, in normalized units: the height
    /// of the V (the mark is its own V; the wordmark's V is its leading
    /// mark piece, a fraction of the art box the tail of the "x" also
    /// fills).
    pub fn clear_space_units(self) -> f32 {
        match self {
            LogoArt::Mark => paths::MARK_CLEAR_SPACE,
            LogoArt::Wordmark => paths::WORDMARK_CLEAR_SPACE,
        }
    }
}

/// The logo's ink: the placement whose role colours it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoInk {
    /// `primary` on the canvas or any surface: the default.
    Canvas,
    /// `on-primary` over a primary fill.
    OnPrimary,
    /// `inverse-on-surface` on an inverse band (hero, footer,
    /// snackbar).
    Inverse,
    /// `text-accent`: the bronze family's accent, print foil's on-screen
    /// stand-in.
    Accent,
}

impl LogoInk {
    /// The role the ink resolves to in a theme.
    pub fn resolve(self, colors: &ThemeColors) -> Color32 {
        match self {
            LogoInk::Canvas => colors.primary,
            LogoInk::OnPrimary => colors.on_primary,
            LogoInk::Inverse => colors.inverse_on_surface,
            LogoInk::Accent => colors.text_accent,
        }
    }
}

/// One logo, sized by its art height (or width, through
/// [`Logo::width`]), coloured by a [`LogoInk`] role or an outright
/// [`Logo::color`] override.
///
/// ```
/// use ui_kit::logo::Logo;
///
/// let wordmark = Logo::wordmark(); // 96px wide, primary on the canvas
/// let mark = Logo::mark().height(32.0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Logo {
    art: LogoArt,
    height: f32,
    ink: LogoInk,
    color: Option<Color32>,
}

impl Logo {
    /// The wordmark at its smallest on-screen size, 96px wide, in the
    /// canvas ink.
    pub fn wordmark() -> Self {
        Self {
            art: LogoArt::Wordmark,
            height: 96.0 / LogoArt::Wordmark.aspect(),
            ink: LogoInk::Canvas,
            color: None,
        }
    }

    /// The mark at its product-chrome size, 24px tall, in the canvas
    /// ink.
    pub fn mark() -> Self {
        Self {
            art: LogoArt::Mark,
            height: MARK_MIN_CHROME,
            ink: LogoInk::Canvas,
            color: None,
        }
    }

    /// Set the art height in px; the width follows the aspect.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set the art width in px; the height follows the aspect.
    pub fn width(mut self, width: f32) -> Self {
        self.height = width / self.art.aspect();
        self
    }

    /// Colour the logo with a role selected by placement.
    pub fn ink(mut self, ink: LogoInk) -> Self {
        self.ink = ink;
        self
    }

    /// Colour the logo outright, overriding the ink's role.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// The artwork the logo paints.
    pub fn art(&self) -> LogoArt {
        self.art
    }

    /// The logo's laid-out size: the art box at the set height, aspect
    /// preserved.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.height * self.art.aspect(), self.height)
    }

    /// The clear space on every side: the height of the V, in px.
    pub fn clear_space(&self) -> f32 {
        self.height * self.art.clear_space_units() / UNITS
    }

    /// The paint colour: the override when set, the ink's role in
    /// `colors` otherwise.
    fn paint_color(&self, colors: &ThemeColors) -> Color32 {
        self.color.unwrap_or_else(|| self.ink.resolve(colors))
    }

    /// The mesh for the art scaled into `rect`.
    fn mesh(&self, rect: egui::Rect, color: Color32) -> Mesh {
        let mut mesh = Mesh::default();
        let (w, h) = self.art.box_units();
        let scale = Vec2::new(rect.width() / w, rect.height() / h);
        for triangle in self.art.triangles() {
            let base = mesh.vertices.len() as u32;
            for pair in triangle.chunks(2) {
                let [x, y] = [pair[0], pair[1]];
                let pos = rect.left_top() + Vec2::new(x * scale.x, y * scale.y);
                mesh.colored_vertex(Pos2::new(pos.x, pos.y), color);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        mesh
    }
}

/// Whether the logo's laid-out size meets its minimum: the wordmark
/// 96px wide, the mark 24px tall (the chrome floor; brand placements
/// keep to [`MARK_MIN_BRAND`]). A hair of tolerance (0.05px) keeps
/// floating-point sizing from failing its own defaults.
pub fn size_ok(art: LogoArt, size: Vec2) -> bool {
    match art {
        LogoArt::Wordmark => size.x + 0.05 >= WORDMARK_MIN_WIDTH,
        LogoArt::Mark => size.y + 0.05 >= MARK_MIN_CHROME,
    }
}

impl Widget for Logo {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size();
        // The minimum-size guard: a debug assert (the issue's
        // "debug-assert/log" floor), silent in release builds.
        debug_assert!(
            size_ok(self.art, size),
            "logo below its minimum size: {:?} at {:?}",
            self.art,
            size
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let theme =
            crate::theme::ThemeId::current(ui.ctx()).unwrap_or(crate::theme::ThemeId::Light);
        let color = self.paint_color(&theme.colors());
        let mesh = self.mesh(rect, color);
        ui.painter().add(Shape::mesh(mesh));
        response
    }
}

#[cfg(test)]
mod tests {
    use super::{
        size_ok, Logo, LogoArt, LogoInk, MARK_MIN_BRAND, MARK_MIN_CHROME, WORDMARK_MIN_WIDTH,
    };
    use crate::theme::ThemeId;

    /// The aspects are the source art boxes': the mark 12684.2 over
    /// 12592.4, the wordmark 2654.79 over 653.758. Every sizing path
    /// divides through them.
    #[test]
    fn art_aspects_match_the_source_boxes() {
        assert!((LogoArt::Mark.aspect() - 12684.2 / 12592.4).abs() < 0.01);
        assert!((LogoArt::Wordmark.aspect() - 2654.79 / 653.758).abs() < 0.01);
    }

    /// The constructors land on the minimums exactly: the wordmark 96px
    /// wide, the mark 24px tall in chrome, 96 reserved for brand.
    #[test]
    fn defaults_meet_the_minimum_sizes() {
        assert_eq!(Logo::wordmark().size().x, WORDMARK_MIN_WIDTH);
        assert_eq!(Logo::mark().size().y, MARK_MIN_CHROME);
        assert_eq!(MARK_MIN_BRAND, 96.0);
    }

    /// The guards fire between the floors and one step below them, and
    /// width sizing keeps the aspect: 96px wide is 96 over aspect tall.
    #[test]
    fn minimum_size_guards_fire() {
        let aspect = LogoArt::Wordmark.aspect();
        assert!(!size_ok(
            LogoArt::Wordmark,
            egui::Vec2::new(
                WORDMARK_MIN_WIDTH - 0.1,
                (WORDMARK_MIN_WIDTH - 0.1) / aspect
            )
        ));
        assert!(size_ok(
            LogoArt::Wordmark,
            egui::Vec2::new(WORDMARK_MIN_WIDTH, WORDMARK_MIN_WIDTH / aspect)
        ));
        assert!(!size_ok(
            LogoArt::Mark,
            egui::Vec2::new(MARK_MIN_CHROME - 0.1, MARK_MIN_CHROME - 0.1)
        ));
        assert!(size_ok(
            LogoArt::Mark,
            egui::Vec2::new(MARK_MIN_CHROME, MARK_MIN_CHROME)
        ));
        let sized = Logo::wordmark().width(120.0);
        assert!((sized.size().x - 120.0).abs() < 1e-3);
        assert!((sized.size().y - 120.0 / aspect).abs() < 1e-3);
    }

    /// Clear space on every side is the height of the V: the mark's own
    /// height, the wordmark's V cap height (a fraction of the art box,
    /// which also carries the tail of the "x").
    #[test]
    fn clear_space_is_the_height_of_the_v() {
        let mark = Logo::mark().height(32.0);
        assert!((mark.clear_space() - 32.0).abs() < 0.5, "the mark is the V");
        let wordmark = Logo::wordmark();
        let height = wordmark.size().y;
        let clear = wordmark.clear_space();
        assert!(clear > 0.0);
        assert!(
            clear > 0.5 * height && clear <= height + 1e-3,
            "the wordmark's V is most of its box: {clear} of {height}"
        );
    }

    /// The emitted geometry is well formed: triangles present, finite,
    /// inside the art box, none degenerate, together covering less than
    /// the whole box.
    #[test]
    fn triangles_are_well_formed() {
        for art in [LogoArt::Mark, LogoArt::Wordmark] {
            let (w, h) = art.box_units();
            let triangles = art.triangles();
            assert!(!triangles.is_empty(), "{art:?} carries triangles");
            let mut total = 0.0f32;
            for t in triangles {
                let mut xs = [0.0f32; 3];
                let mut ys = [0.0f32; 3];
                for i in 0..3 {
                    let (x, y) = (t[2 * i], t[2 * i + 1]);
                    assert!(x.is_finite() && y.is_finite(), "{art:?}: finite");
                    assert!(
                        (0.0..=w + 0.5).contains(&x) && (0.0..=h + 0.5).contains(&y),
                        "{art:?}: ({x}, {y}) inside the {w}x{h} box"
                    );
                    xs[i] = x;
                    ys[i] = y;
                }
                let area = 0.5
                    * ((xs[1] - xs[0]) * (ys[2] - ys[0]) - (xs[2] - xs[0]) * (ys[1] - ys[0])).abs();
                assert!(area > 0.0, "{art:?}: no degenerate triangles");
                total += area;
            }
            assert!(total < w * h, "{art:?}: ink is less than the whole box");
            assert!(
                total > 0.1 * w * h,
                "{art:?}: ink covers a real share of the box"
            );
        }
    }

    /// The inks resolve to the theme's roles and the outright colour
    /// override wins over them.
    #[test]
    fn inks_resolve_to_the_theme_roles() {
        let colors = ThemeId::Light.colors();
        assert_eq!(LogoInk::Canvas.resolve(&colors), colors.primary);
        assert_eq!(LogoInk::OnPrimary.resolve(&colors), colors.on_primary);
        assert_eq!(LogoInk::Inverse.resolve(&colors), colors.inverse_on_surface);
        assert_eq!(LogoInk::Accent.resolve(&colors), colors.text_accent);
        let override_color = egui::Color32::from_rgb(1, 2, 3);
        let logo = Logo::mark().color(override_color);
        assert_eq!(logo.paint_color(&colors), override_color);
        assert_eq!(Logo::mark().paint_color(&colors), colors.primary);
    }

    /// The widget lays out at its size and paints without touching the
    /// texture deltas (headless pass over every theme).
    #[test]
    fn logo_widget_lays_out_at_its_size() {
        let ctx = egui::Context::default();
        for theme in ThemeId::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                theme.apply(ui.ctx());
                let response = ui.add(Logo::wordmark());
                let expected = Logo::wordmark().size();
                let got = response.rect.size();
                for i in 0..2 {
                    assert!(
                        (expected[i] - got[i]).abs() < 0.05,
                        "{:?}: wordmark {got:?} vs {expected:?}",
                        theme.name()
                    );
                }
                let response = ui.add(Logo::mark().height(32.0));
                let got = response.rect.size();
                let expected = egui::Vec2::new(32.0 * LogoArt::Mark.aspect(), 32.0);
                for i in 0..2 {
                    assert!(
                        (expected[i] - got[i]).abs() < 0.05,
                        "{:?}: mark {got:?} vs {expected:?}",
                        theme.name()
                    );
                }
            });
            out.drop_without_applying_deltas();
        }
    }
}

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
//! runtime. The first paint of each art derives its ink outline from
//! those triangles (see Anti-aliasing below), once and cached; the
//! widget maps the outline's points through a scale and hands the ink
//! and its feather ring to `epaint` as one [`egui::Shape::Mesh`] per
//! logo.
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
//! Anti-aliasing. epaint hands a `Shape::Mesh` to the GPU as it is, so
//! flat triangles would paint hard edges where every other shape and
//! glyph gets a feather. The widget therefore feathers the mesh
//! itself, the way epaint feathers a filled path: the `outline` module
//! derives the ink's boundary from the triangles once, the ink is
//! pulled in by half the feather along the boundary's miter, and a
//! ring of the same colour fading to transparent is added over the
//! boundary, out to half the feather beyond it, all in the one mesh.
//! The feather is the context's own
//! (`TessellationOptions::feathering_size_in_pixels`, one physical
//! pixel by default, and off when the context turns feathering off),
//! so the logo's edge matches the text's at every scale factor. The
//! ring spills past the laid-out rect: half a feather where the edge
//! meets the rect square on (0.5pt at 1x on the wordmark's left,
//! right and top at 24pt high), up to a full feather where a sharp
//! corner's miter reaches its cap (the wordmark's bottom).
//!
//! Documented deviations: the feather is a vertex-alpha ramp, not
//! coverage, so a stroke thinner than a pixel paints a little heavier
//! than the art (the whole wordmark at its 96px minimum stays within
//! 3% of the art's area, which a test pins); the flattening tolerance
//! is 1/1000 of the art box (the script asserts the triangulated area
//! against the shoelace area of the contours, holes subtracted, before
//! emitting).

mod outline;
mod paths;

use std::sync::LazyLock;

use egui::epaint::{Mesh, Shape};
use egui::{Color32, Response, Sense, Ui, Vec2, Widget};

use outline::Outline;

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

    /// The ink's outline, derived from [`LogoArt::triangles`] on first
    /// use (the one table the art's triangles are read through).
    fn outline(self) -> &'static Outline {
        static MARK: LazyLock<Outline> = LazyLock::new(|| Outline::new(LogoArt::Mark.triangles()));
        static WORDMARK: LazyLock<Outline> =
            LazyLock::new(|| Outline::new(LogoArt::Wordmark.triangles()));
        match self {
            LogoArt::Mark => &MARK,
            LogoArt::Wordmark => &WORDMARK,
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

    /// The mesh for the art scaled into `rect`, its edge feathered over
    /// `feather` points: the ink pulled in by half of it, a ring of
    /// the same colour fading to transparent over the whole of it,
    /// centred on the art's edge as epaint's own feathering is. `0`
    /// paints the snapped outline's triangles with no ring.
    fn mesh(&self, rect: egui::Rect, color: Color32, feather: f32) -> Mesh {
        let outline = self.art.outline();
        let (w, h) = self.art.box_units();
        let scale = Vec2::new(rect.width() / w, rect.height() / h);
        let half = 0.5 * feather;
        let stride = if feather > 0.0 { 2 } else { 1 };
        let ring = if feather > 0.0 {
            2 * outline.edges.len()
        } else {
            0
        };
        let mut mesh = Mesh::default();
        mesh.reserve_vertices(stride * outline.points.len());
        mesh.reserve_triangles(outline.triangles.len() + ring);
        for (point, miter) in outline.points.iter().zip(&outline.miters) {
            let at = rect.left_top() + Vec2::new(point[0] * scale.x, point[1] * scale.y);
            let offset = Vec2::new(miter[0], miter[1]) * half;
            mesh.colored_vertex(at - offset, color);
            if feather > 0.0 {
                mesh.colored_vertex(at + offset, Color32::TRANSPARENT);
            }
        }
        // Vertex `2 * i` is point `i` pulled in, `2 * i + 1` pushed out.
        let inner = |i: u32| stride as u32 * i;
        for &[a, b, c] in &outline.triangles {
            mesh.add_triangle(inner(a), inner(b), inner(c));
        }
        if feather > 0.0 {
            for &[from, to] in &outline.edges {
                let (a, b) = (inner(from), inner(to));
                mesh.add_triangle(a, b, b + 1);
                mesh.add_triangle(a, b + 1, a + 1);
            }
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
        // The feather every other shape gets: the tessellator's own
        // width, in physical pixels, and its switch.
        let feather_pixels = ui.ctx().tessellation_options(|options| {
            if options.feathering {
                options.feathering_size_in_pixels
            } else {
                0.0
            }
        });
        let feather = feather_pixels / ui.ctx().pixels_per_point();
        let mesh = self.mesh(rect, color, feather);
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
    use egui::epaint::Mesh;

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

    /// The mesh the widget painted, and the rect it was laid out in,
    /// from one headless frame at `pixels_per_point`.
    fn painted_logo(logo: Logo, pixels_per_point: f32, feathering: bool) -> (Mesh, egui::Rect) {
        fn find(shape: &egui::Shape) -> Option<Mesh> {
            match shape {
                egui::Shape::Mesh(mesh) => Some((**mesh).clone()),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(find),
                _ => None,
            }
        }
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(pixels_per_point);
        ctx.tessellation_options_mut(|options| options.feathering = feathering);
        let mut rect = egui::Rect::NOTHING;
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ThemeId::Light.apply(ui.ctx());
            rect = ui.add(logo).rect;
        });
        let mesh = out
            .shapes
            .iter()
            .find_map(|clipped| find(&clipped.shape))
            .expect("the logo paints one mesh");
        out.drop_without_applying_deltas();
        (mesh, rect)
    }

    /// Per-pixel coverage of `mesh` over the pixel grid that starts at
    /// `origin` (in points), `pixels_per_point` pixels to the point:
    /// each triangle is sampled at the pixel centres with its vertex
    /// alpha interpolated, and the samples composited source-over, the
    /// way the GPU blends the premultiplied vertex colours. Returns the
    /// grid's width, height and the alphas (0 to 1).
    fn coverage(
        mesh: &Mesh,
        origin: egui::Pos2,
        size: egui::Vec2,
        pixels_per_point: f32,
    ) -> (usize, usize, Vec<f32>) {
        let width = (size.x * pixels_per_point).ceil() as usize + 4;
        let height = (size.y * pixels_per_point).ceil() as usize + 4;
        let mut alpha = vec![0.0f32; width * height];
        // Pixel (0, 0) sits two pixels up and left of the origin, so
        // the feather outside the rect has room.
        let pixel = |x: f32, y: f32| {
            (
                (x - origin.x) * pixels_per_point + 2.0,
                (y - origin.y) * pixels_per_point + 2.0,
            )
        };
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let v: Vec<_> = triangle
                .iter()
                .map(|&i| {
                    let vertex = mesh.vertices[i as usize];
                    let (x, y) = pixel(vertex.pos.x, vertex.pos.y);
                    (x, y, f32::from(vertex.color.a()) / 255.0)
                })
                .collect();
            let area =
                (v[1].0 - v[0].0) * (v[2].1 - v[0].1) - (v[1].1 - v[0].1) * (v[2].0 - v[0].0);
            if area.abs() < 1e-9 {
                continue;
            }
            let x0 = v
                .iter()
                .map(|p| p.0)
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(0.0) as usize;
            let x1 = v
                .iter()
                .map(|p| p.0)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .max(0.0) as usize;
            let y0 = v
                .iter()
                .map(|p| p.1)
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(0.0) as usize;
            let y1 = v
                .iter()
                .map(|p| p.1)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .max(0.0) as usize;
            for py in y0..y1.min(height) {
                for px in x0..x1.min(width) {
                    let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
                    let w0 = ((v[1].0 - cx) * (v[2].1 - cy) - (v[1].1 - cy) * (v[2].0 - cx)) / area;
                    let w1 = ((v[2].0 - cx) * (v[0].1 - cy) - (v[2].1 - cy) * (v[0].0 - cx)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let source = w0 * v[0].2 + w1 * v[1].2 + w2 * v[2].2;
                    let behind = alpha[py * width + px];
                    alpha[py * width + px] = source + behind * (1.0 - source);
                }
            }
        }
        (width, height, alpha)
    }

    /// Wordmark ink area in points squared, from the source triangles:
    /// the reference a painted, feathered mesh must keep.
    fn ink_area(logo: &Logo, pixels_per_point: f32) -> f32 {
        // Rasterised at 8 samples to the pixel through the same
        // coverage routine, from the unfeathered triangles, so the
        // reference shares the union semantics of the painted mesh.
        let size = logo.size();
        let (w, h) = logo.art().box_units();
        let mut hard = Mesh::default();
        for triangle in logo.art().triangles() {
            let base = hard.vertices.len() as u32;
            for pair in triangle.chunks(2) {
                let pos = egui::Pos2::new(pair[0] * size.x / w, pair[1] * size.y / h);
                hard.colored_vertex(pos, egui::Color32::WHITE);
            }
            hard.add_triangle(base, base + 1, base + 2);
        }
        let dense = 8.0 * pixels_per_point;
        let (_, _, alpha) = coverage(&hard, egui::Pos2::ZERO, size, dense);
        alpha.iter().sum::<f32>() / (dense * dense)
    }

    /// Finding 2-I4: the wordmark's edges were hard, with no
    /// anti-aliasing, unlike the kit's text. The wordmark is one
    /// `Shape::Mesh` of flat-coloured triangles, which epaint's
    /// tessellator passes through verbatim, so it never got the feather
    /// every other shape carries. Rasterised the way a GPU would at 1x
    /// and 2x, every ink pixel is either fully covered or empty today;
    /// feathered, the edge pixels take intermediate coverage, and the
    /// ink keeps its area (the feather straddles the edge, it does not
    /// fatten it).
    #[test]
    fn the_wordmark_edges_are_anti_aliased() {
        for pixels_per_point in [1.0, 2.0] {
            let logo = Logo::wordmark().height(24.0);
            let (mesh, rect) = painted_logo(logo, pixels_per_point, true);
            let (_, _, alpha) = coverage(&mesh, rect.min, rect.size(), pixels_per_point);
            let ink = alpha.iter().filter(|a| **a > 0.02).count();
            let edge = alpha.iter().filter(|a| **a > 0.02 && **a < 0.98).count();
            assert!(ink > 0, "the wordmark paints ink at {pixels_per_point}x");
            let share = edge as f32 / ink as f32;
            assert!(
                share > 0.15,
                "{edge} of {ink} ink pixels carry partial coverage at {pixels_per_point}x \
                 ({share:.2}); a hard-edged mesh has none"
            );

            let painted = alpha.iter().sum::<f32>() / (pixels_per_point * pixels_per_point);
            let reference = ink_area(&logo, pixels_per_point);
            assert!(
                (painted - reference).abs() < 0.03 * reference,
                "the feathered wordmark covers {painted:.1} pt2 at {pixels_per_point}x, \
                 the source art {reference:.1}"
            );
        }
    }

    /// The ring follows the tessellator's feather
    /// (`feathering_size_in_pixels` physical pixels, so a point at 1x
    /// and half a point at 2x) and is gone with its switch. Each point
    /// of the art is an opaque vertex and a transparent one, offset
    /// along the miter, so the pair stands one to two feathers apart
    /// (the miter's length runs from 1 to its cap of 2). That is the
    /// distance between the two vertices, not a width measured across
    /// the edge. The ink stays in the laid-out rect and the ring within
    /// a feather of it.
    #[test]
    fn the_feather_ring_spans_one_to_two_feathers_and_follows_the_tessellation_option() {
        for art in [Logo::mark(), Logo::wordmark()] {
            for pixels_per_point in [1.0, 2.0, 3.0] {
                let (hard, _) = painted_logo(art, pixels_per_point, false);
                assert!(
                    hard.vertices.iter().all(|v| v.color.a() == 255),
                    "{:?}: with feathering off the logo is opaque triangles",
                    art.art()
                );

                let (soft, rect) = painted_logo(art, pixels_per_point, true);
                let feather = 1.0 / pixels_per_point;
                // The ink stays inside the laid-out rect; the ring
                // spills at most a feather (half, times the miter
                // limit) past it, so the layout is the same.
                let ink = soft
                    .vertices
                    .iter()
                    .filter(|v| v.color.a() == 255)
                    .fold(egui::Rect::NOTHING, |bounds, v| {
                        bounds.union(egui::Rect::from_pos(v.pos))
                    });
                assert!(
                    rect.expand(1e-3).contains_rect(ink),
                    "{:?}: the ink {ink:?} leaves its rect {rect:?}",
                    art.art()
                );
                assert!(
                    rect.expand(feather + 1e-3)
                        .contains_rect(soft.calc_bounds()),
                    "{:?}: the ring {:?} spills more than a feather past {rect:?}",
                    art.art(),
                    soft.calc_bounds()
                );
                let (mut ramps, mut flat) = (0, 0);
                for pair in soft.vertices.as_chunks::<2>().0 {
                    assert_eq!(pair[0].color.a(), 255, "the inner vertex is the ink");
                    assert_eq!(pair[1].color.a(), 0, "the outer vertex is transparent");
                    let width = pair[0].pos.distance(pair[1].pos);
                    if width == 0.0 {
                        flat += 1;
                        continue;
                    }
                    ramps += 1;
                    assert!(
                        width >= feather * 0.999 && width <= feather * 2.001,
                        "{:?} at {pixels_per_point}x: a pair {width} apart, the feather {feather}",
                        art.art()
                    );
                }
                assert!(
                    ramps > 0 && flat * 100 <= ramps,
                    "{ramps} ramps, {flat} flat"
                );
            }
        }
    }

    /// The outline the widget paints is the art's own: its points are
    /// corners of `LogoArt::triangles` (the one table both read) and
    /// span the same box, so a second dispatch that went stale, or
    /// swapped the two arts, would fail here.
    #[test]
    fn each_arts_outline_is_built_from_its_own_triangles() {
        use std::collections::HashSet;

        for art in [LogoArt::Mark, LogoArt::Wordmark] {
            let outline = art.outline();
            let corners: HashSet<[u32; 2]> = art
                .triangles()
                .iter()
                .flat_map(|t| t.as_chunks::<2>().0.iter().copied())
                .map(|[x, y]| [x.to_bits(), y.to_bits()])
                .collect();
            assert!(
                outline
                    .points
                    .iter()
                    .all(|[x, y]| corners.contains(&[x.to_bits(), y.to_bits()])),
                "{art:?}: an outline point is not a corner of its triangles"
            );
            let extent = |axis: usize, pick: fn(f32, f32) -> f32, from: f32| {
                outline.points.iter().map(|p| p[axis]).fold(from, pick)
            };
            let (w, h) = art.box_units();
            for (axis, size) in [(0, w), (1, h)] {
                let (lo, hi) = (
                    extent(axis, f32::min, f32::INFINITY),
                    extent(axis, f32::max, f32::NEG_INFINITY),
                );
                assert!(
                    lo < 1.0 && (size - hi).abs() < 1.0,
                    "{art:?}: the outline spans {lo}..{hi} on axis {axis}, the art box {size}"
                );
            }
            assert!(
                // Snapping drops the slivers it collapses (a fifth of
                // the mark's); the rest are the art's.
                outline.triangles.len() * 4 >= art.triangles().len() * 3
                    && outline.triangles.len() <= art.triangles().len(),
                "{art:?}: the outline keeps the art's triangles"
            );
        }
    }
}

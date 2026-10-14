//! The grid: a responsive layout of equal cells for the kit's catalogues
//! (the icon sheet, the token and swatch rows, any run of specimens).
//!
//! [`Grid::show`] lays the items out in rows. The column count follows
//! the width the page gives it: as many cells of at least
//! `min_cell_width` as fit with the gap between them, and never fewer
//! than one. At the default gap of [`spacing::SPACE_4`] (16pt) and a
//! `min_cell_width` of 96, a 360pt phone gets three cells of 109.33pt, a
//! 448pt window four of 100pt and a 1200pt desktop ten of 105.6pt. The
//! cells fill the width edge to edge. A short last row keeps the same
//! cell width and leaves its trailing columns empty, as CSS `auto-fill`
//! does.
//!
//! Each row is as tall as its tallest cell, and every cell in the row
//! takes that height. The grid measures before it places. The cell
//! closure runs once per cell in a sizing pass (an invisible child Ui
//! for which [`egui::Ui::is_sizing_pass`] is true), the rows take their
//! heights, and the closure runs again for real with each cell's final
//! size. So each cell is laid out twice per frame. The sizing pass paints
//! nothing and its responses are dropped, so only the real pass responds
//! to input. The cells are placed in one allocation for the whole grid,
//! so the gap between rows is the grid's own and not the layout's item
//! spacing.
//!
//! [`egui::Grid`] is not used: its columns size to their content, so a
//! catalogue's columns would not line up across its rows.
//!
//! Captions belong to the cell closure, and the grid does not draw them.
//! The closure gets the cell's own Ui, exactly one cell wide. In a vertical
//! layout a [`egui::Label`] wraps to the Ui's available width, which is the
//! cell's width, so a caption wraps inside its cell as it stands. The kit's
//! [`crate::typography::specimen_job`] sets a caption in the caption type.
//! The label's wrap width replaces the job's, so passing `Some(size.x)`
//! states the same width without changing the result. The row then grows to
//! the tallest caption, so a two-line caption is never clipped.
//!
//! The cell closure receives `(ui, index, item, size)`. `size` is the
//! cell's width and its row's height. In the sizing pass `size.y` is `0`,
//! so content measures at its natural height: lay art and caption out at
//! their own height, and paint into `size` as you like, since the sizing
//! pass paints nothing. Do not reserve `size.y` as content. A cell that
//! does measures as zero and leaves its row to the other cells.
//!
//! The grid belongs in a vertical layout, such as a page or a scroll
//! area. It takes the rest of the current line's width.

use egui::{pos2, vec2, Align, Layout, Rect, Response, Sense, Ui, UiBuilder, Vec2};

use crate::tokens::spacing;

/// A responsive grid of equal cells. Build one with [`Grid::new`] and
/// lay items out with [`Grid::show`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    min_cell_width: f32,
    gap: f32,
}

/// What [`Grid::show`] laid out.
#[derive(Clone, Debug)]
pub struct GridResponse {
    /// The grid's own response. It spans the width the grid filled and
    /// is as tall as its rows and the gaps between them.
    pub response: Response,
    /// The number of columns at this width. Set even for an empty slice.
    pub columns: usize,
    /// The width of every cell.
    pub cell_width: f32,
    /// Each cell's rect, in item order. Cells in a row share a top and a
    /// height.
    pub cells: Vec<Rect>,
}

impl Grid {
    /// A grid whose cells are at least `min_cell_width` wide, with the
    /// default gap of [`spacing::SPACE_4`] (16pt). The gap is the kit's
    /// phone page margin, so cells sit as far apart as the page sits from
    /// the screen edge.
    #[must_use]
    pub const fn new(min_cell_width: f32) -> Self {
        Self {
            min_cell_width,
            gap: spacing::SPACE_4,
        }
    }

    /// The gap between cells, horizontally and vertically.
    #[must_use]
    pub const fn gap(self, gap: f32) -> Self {
        Self { gap, ..self }
    }

    /// Lay `items` out as rows of equal cells in the rest of `ui`'s
    /// current line, one cell per item, and draw each through `cell`.
    /// `cell` gets the item's index, the item and the cell's size; see
    /// the module docs for how `size` behaves in the sizing pass.
    ///
    /// An empty slice lays out nothing and takes no space. An unbounded
    /// width, such as a horizontal scroll area, lays out one cell of
    /// `min_cell_width`.
    pub fn show<T>(
        self,
        ui: &mut Ui,
        items: &[T],
        mut cell: impl FnMut(&mut Ui, usize, &T, Vec2),
    ) -> GridResponse {
        let line = ui.available_rect_before_wrap();
        let origin = line.min;
        let width = fill_width(line.width(), self.min_cell_width);
        let columns = columns_for(width, self.min_cell_width, self.gap);
        let cell_width = cell_width_for(width, columns, self.gap);

        if items.is_empty() {
            // Nothing is allocated, so the cursor stays where it was. The
            // response is a zero-height interaction that moves nothing.
            let id = ui.auto_id_with("grid-empty");
            let response = ui.interact(Rect::from_min_size(origin, Vec2::ZERO), id, Sense::hover());
            return GridResponse {
                response,
                columns,
                cell_width,
                cells: Vec::new(),
            };
        }

        // The sizing and paint children take their ids from `base` and
        // the item's index, under distinct tags, so no two widgets in this
        // frame share an id (egui flags a clash on any widget it creates).
        let base = ui.next_auto_id();

        // Pass one: each row is as tall as its tallest cell. The sizing
        // child is unbounded in height, so no cell is cut short, and it is
        // invisible and unresponsive, so only the paint pass is live.
        let mut row_heights = Vec::with_capacity(items.len().div_ceil(columns));
        for (row, chunk) in items.chunks(columns).enumerate() {
            let mut tallest = 0.0_f32;
            for (col, item) in chunk.iter().enumerate() {
                let index = row * columns + col;
                let bounds = Rect::from_min_size(origin, vec2(cell_width, f32::INFINITY));
                let mut sizing = ui.new_child(
                    UiBuilder::new()
                        .id_salt((base, "sizing", index))
                        .max_rect(bounds)
                        .layout(Layout::top_down(Align::Center))
                        .sizing_pass()
                        .invisible(),
                );
                cell(&mut sizing, index, item, vec2(cell_width, 0.0));
                tallest = tallest.max(sizing.min_rect().height());
            }
            row_heights.push(tallest);
        }
        let gaps = self.gap * row_heights.len().saturating_sub(1) as f32;
        let height = row_heights.iter().sum::<f32>() + gaps;

        // Claim the grid's whole rect at once, so the rows sit exactly one
        // gap apart and the layout adds nothing between them.
        let response = ui.allocate_rect(
            Rect::from_min_size(origin, vec2(width, height)),
            Sense::hover(),
        );

        // Pass two: paint each cell at its final rect, with its row's
        // height as the size.
        let mut cells = Vec::with_capacity(items.len());
        let mut top = origin.y;
        for (row, (chunk, &row_height)) in items.chunks(columns).zip(&row_heights).enumerate() {
            for (col, item) in chunk.iter().enumerate() {
                let index = row * columns + col;
                let left = origin.x + col as f32 * (cell_width + self.gap);
                let rect = Rect::from_min_size(pos2(left, top), vec2(cell_width, row_height));
                let mut painted = ui.new_child(
                    UiBuilder::new()
                        .id_salt((base, "cell", index))
                        .max_rect(rect)
                        .layout(Layout::top_down(Align::Center)),
                );
                cell(&mut painted, index, item, rect.size());
                cells.push(rect);
            }
            top += row_height + self.gap;
        }

        GridResponse {
            response,
            columns,
            cell_width,
            cells,
        }
    }
}

/// The width the grid fills: the rest of the line, never negative. An
/// unbounded line (a horizontal scroll area) fills one cell instead, so
/// the grid never builds an infinite rect.
fn fill_width(line: f32, min_cell_width: f32) -> f32 {
    if line.is_finite() {
        line.max(0.0)
    } else {
        min_cell_width.max(0.0)
    }
}

/// `max(1, floor((available + gap) / (min + gap)))`: how many cells of at
/// least `min` width, `gap` apart, fit in `available`. A non-positive
/// stride (zero minimum and zero gap) gives one column, so the division
/// never runs.
fn columns_for(available: f32, min: f32, gap: f32) -> usize {
    let stride = min + gap;
    if stride > 0.0 {
        (((available + gap) / stride).floor() as usize).max(1)
    } else {
        1
    }
}

/// `(available - gap * (columns - 1)) / columns`: the width each of
/// `columns` equal cells takes so the row fills `available` exactly.
fn cell_width_for(available: f32, columns: usize, gap: f32) -> f32 {
    (available - gap * (columns - 1) as f32) / columns as f32
}

#[cfg(test)]
mod tests {
    use egui::{Color32, Context, Pos2, RawInput, Rect, Sense, Vec2};

    use super::{cell_width_for, columns_for, Grid, GridResponse};
    use crate::theme::ThemeId;
    use crate::typography::{specimen_job, TypeStyle};

    /// Float tolerance for cell geometry. Cells are placed at unrounded
    /// positions, so exact equality is not promised.
    const EPS: f32 = 1e-3;
    const MIN: f32 = 96.0;
    const GAP: f32 = 16.0;

    fn input(width: f32) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 800.0))),
            ..Default::default()
        }
    }

    /// Lay one cell per height out at `width`, each cell drawing a 10pt
    /// wide block of its height.
    fn lay_out(width: f32, heights: &[f32]) -> GridResponse {
        let ctx = Context::default();
        let mut out = None;
        ctx.run_ui(input(width), |ui| {
            out = Some(
                Grid::new(MIN)
                    .gap(GAP)
                    .show(ui, heights, |ui, _, height, _| {
                        let _ = ui.allocate_exact_size(Vec2::new(10.0, *height), Sense::hover());
                    }),
            );
        })
        .drop_without_applying_deltas();
        out.expect("the grid laid out in the frame")
    }

    /// The column count at the phone, tablet and desktop widths, and the
    /// cell width each gives, worked by hand:
    /// - 360pt: floor((360 + 16) / 112) = floor(3.36) = 3 columns of
    ///   (360 - 2 * 16) / 3 = 109.33pt.
    /// - 448pt: floor((448 + 16) / 112) = floor(4.14) = 4 columns of
    ///   (448 - 3 * 16) / 4 = 100pt.
    /// - 1200pt: floor((1200 + 16) / 112) = floor(10.86) = 10 columns of
    ///   (1200 - 9 * 16) / 10 = 105.6pt.
    #[test]
    fn grid_column_counts_at_phone_tablet_and_desktop_widths() {
        let heights = [20.0_f32; 30];
        for (width, columns, cell_width) in [
            (360.0, 3, 328.0 / 3.0),
            (448.0, 4, 100.0),
            (1200.0, 10, 105.6),
        ] {
            let out = lay_out(width, &heights);
            assert_eq!(out.columns, columns, "columns at {width}pt");
            assert!(
                (out.cell_width - cell_width).abs() < EPS,
                "cell width at {width}pt is {}, expected {cell_width}",
                out.cell_width
            );
            let first_top = out.cells[0].top();
            let in_first_row = out
                .cells
                .iter()
                .filter(|rect| (rect.top() - first_top).abs() < EPS)
                .count();
            assert_eq!(in_first_row, columns, "cells in the first row at {width}pt");
        }
    }

    /// The column count switches exactly where a cell and its gap fill
    /// the width: two 96pt cells and their 16pt gap need 208pt.
    #[test]
    fn grid_columns_switch_exactly_at_a_cell_and_gap_boundary() {
        assert_eq!(columns_for(96.0, MIN, GAP), 1);
        assert_eq!(columns_for(207.9, MIN, GAP), 1);
        assert_eq!(columns_for(208.0, MIN, GAP), 2);
        assert_eq!(columns_for(0.0, MIN, GAP), 1, "never fewer than one");
        assert!((cell_width_for(208.0, 2, GAP) - 96.0).abs() < EPS);
    }

    /// Four columns at 448pt give 100pt cells, and every cell in every
    /// row, the short last row included, is that wide.
    #[test]
    fn grid_cells_share_one_width_in_a_row_and_across_rows() {
        let out = lay_out(448.0, &[20.0_f32; 10]);
        assert_eq!(out.cells.len(), 10, "three rows: 4, 4 and 2");
        for (index, rect) in out.cells.iter().enumerate() {
            assert!(
                (rect.width() - 100.0).abs() < EPS,
                "cell {index} is {}pt wide, expected 100",
                rect.width()
            );
        }
    }

    /// Cells stay within the width the grid was given, full rows run
    /// edge to edge, and each row starts exactly one gap below the last.
    #[test]
    fn grid_no_cell_extends_past_the_available_width_and_rows_never_overlap() {
        let heights: Vec<f32> = (0..30).map(|i| 12.0 + (i % 5) as f32 * 8.0).collect();
        for width in [360.0_f32, 448.0, 1200.0] {
            let out = lay_out(width, &heights);
            for (index, rect) in out.cells.iter().enumerate() {
                assert!(
                    (-EPS..=width + EPS).contains(&rect.left())
                        && (-EPS..=width + EPS).contains(&rect.right()),
                    "cell {index} at {width}pt spans {}..{}",
                    rect.left(),
                    rect.right()
                );
            }
            let rows: Vec<&[Rect]> = out.cells.chunks(out.columns).collect();
            for row in &rows {
                if row.len() == out.columns {
                    let right = row.last().map_or(0.0, Rect::right);
                    assert!(
                        (right - width).abs() < EPS,
                        "a full row ends at {right}, not {width}"
                    );
                }
            }
            for pair in rows.windows(2) {
                let above_bottom = pair[0].iter().map(Rect::bottom).fold(f32::MIN, f32::max);
                let below_top = pair[1][0].top();
                assert!(
                    (below_top - (above_bottom + GAP)).abs() < EPS,
                    "rows at {width}pt: the next row starts {below_top}, not one gap below {above_bottom}"
                );
            }
        }
    }

    /// Each row is as tall as its tallest cell and every cell in the row
    /// takes that height. The cell closure also gets that height as
    /// `size.y` in the real pass.
    #[test]
    fn grid_each_row_is_as_tall_as_its_tallest_cell() {
        // Four columns at 448pt. The rows' tallest cells are 36, 40 and 30.
        let heights = [20.0, 36.0, 28.0, 12.0, 40.0, 8.0, 16.0, 24.0, 30.0, 10.0];
        let expected = [36.0_f32, 40.0, 30.0];
        let ctx = Context::default();
        let mut out = None;
        let mut painted_sizes = Vec::new();
        ctx.run_ui(input(448.0), |ui| {
            out = Some(
                Grid::new(MIN)
                    .gap(GAP)
                    .show(ui, &heights, |ui, _, height, size| {
                        let _ = ui.allocate_exact_size(Vec2::new(10.0, *height), Sense::hover());
                        if !ui.is_sizing_pass() {
                            painted_sizes.push(size);
                        }
                    }),
            );
        })
        .drop_without_applying_deltas();
        let out = out.expect("the grid laid out in the frame");

        let mut top = out.cells[0].top();
        for (row, chunk) in out.cells.chunks(4).enumerate() {
            for rect in chunk {
                assert!(
                    (rect.top() - top).abs() < EPS,
                    "row {row}: its cells share a top"
                );
                assert!(
                    (rect.height() - expected[row]).abs() < EPS,
                    "row {row} is {}pt tall, expected {}",
                    rect.height(),
                    expected[row]
                );
            }
            top += expected[row] + GAP;
        }
        for (index, size) in painted_sizes.iter().enumerate() {
            let rect = out.cells[index];
            assert!(
                (size.x - rect.width()).abs() < EPS && (size.y - rect.height()).abs() < EPS,
                "cell {index} was handed {size:?}, its rect is {:?}",
                rect.size()
            );
        }
    }

    /// An empty slice runs no cell, allocates no space and does not
    /// panic. The column count still follows the width.
    #[test]
    fn grid_empty_slice_lays_out_nothing_and_does_not_panic() {
        let ctx = Context::default();
        let mut calls = 0;
        ctx.run_ui(input(360.0), |ui| {
            let before = ui.next_widget_position();
            let out = Grid::new(MIN).show(ui, &[] as &[f32], |_, _, _, _| calls += 1);
            assert!(out.cells.is_empty(), "no cells for an empty slice");
            assert_eq!(out.columns, 3, "the column count still follows the width");
            assert_eq!(ui.next_widget_position(), before, "no space is taken");
        })
        .drop_without_applying_deltas();
        assert_eq!(calls, 0, "the cell closure never runs");
    }

    /// A caption with no spaces in it (a real icon name) wraps inside its
    /// cell, and its row grows to the two lines instead of clipping them.
    #[test]
    fn grid_caption_wraps_inside_its_cell_and_grows_the_row() {
        // A short caption, and a long icon name with no spaces in it, which
        // has to break to fit a 96pt cell.
        let captions = ["home", "keyboard_double_arrow_right_outlined_long"];
        let theme = ThemeId::Light;
        let ctx = Context::default();
        // Install the kit faces the captions paint with, as the tab tests do.
        ctx.run_ui(RawInput::default(), |ui| {
            crate::typography::install(theme, ui.ctx());
        })
        .drop_without_applying_deltas();

        let mut labels = Vec::new();
        let mut out = None;
        ctx.run_ui(input(360.0), |ui| {
            out = Some(
                Grid::new(MIN)
                    .gap(GAP)
                    .show(ui, &captions, |ui, _, caption, size| {
                        // The caption call the gallery makes: the kit caption
                        // type. The label wraps to the cell's own width, which
                        // is its available width in this vertical layout.
                        let job = specimen_job(
                            caption,
                            TypeStyle::Caption,
                            theme,
                            Color32::BLACK,
                            Some(size.x),
                        );
                        let label = ui.add(egui::Label::new(job));
                        if !ui.is_sizing_pass() {
                            labels.push(label.rect);
                        }
                    }),
            );
        })
        .drop_without_applying_deltas();
        let out = out.expect("the grid laid out in the frame");

        assert_eq!(labels.len(), captions.len());
        for (index, label) in labels.iter().enumerate() {
            assert!(
                out.cells[index].expand(EPS).contains_rect(*label),
                "caption {index} spans {label:?}, outside its cell {:?}",
                out.cells[index]
            );
        }
        let short = labels[0].height();
        let long = labels[1].height();
        assert!(
            long > short * 1.5,
            "the long caption wraps to more lines: {long} against {short}"
        );
        assert!(
            out.cells[0].height() >= long - EPS,
            "the row grows to the two-line caption"
        );
    }
}

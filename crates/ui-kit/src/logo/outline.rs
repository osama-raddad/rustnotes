//! The outline of a logo's ink, derived once from its triangles.
//!
//! The artwork ships as a soup of triangles (even-odd scanline slabs:
//! see `paths.rs`), which epaint paints verbatim, so the edge of the ink
//! is hard. To feather that edge the widget needs the edge: this module
//! finds it. Triangle corners within [`SNAP`] of each other are one
//! point (the generator rounds each corner on its own, so a corner two
//! slabs share can land a hundredth of a unit apart); the triangles are
//! oriented alike; the horizontal edges between slabs are split where a
//! corner sits on them (a slab ends where two begin); and every
//! directed edge whose opposite is also present cancels. What is left
//! is the boundary of the ink, ink on the left of each edge, in closed
//! loops.
//!
//! Each point also gets a miter vector, the direction the feather
//! offsets it along: the average of the outward normals of the boundary
//! edges that meet there, stretched by 1 over the cosine of half the
//! corner so that the offset edges sit a constant distance from the
//! originals, and capped at [`MITER_LIMIT`] so the tip of a sharp
//! corner does not spike. A point on no boundary edge has a zero miter.

use std::collections::{HashMap, HashSet};

/// Two triangle corners closer than this, in art units on both axes,
/// are the same point. The art box is 1000 units tall, so a quarter
/// unit is a quarter of a pixel at a 1000px wordmark, and far below a
/// pixel at the sizes the kit paints it.
pub(super) const SNAP: f32 = 0.25;

/// The longest a miter vector may get, in multiples of the offset: a
/// right-angle corner needs 1.41, a corner of 60 degrees needs 2.
pub(super) const MITER_LIMIT: f32 = 2.0;

/// The ink's points, triangles and boundary, in art units.
pub(super) struct Outline {
    /// The distinct points, `[x, y]`, y down.
    pub points: Vec<[f32; 2]>,
    /// The outward miter vector at each point (zero off the boundary).
    pub miters: Vec<[f32; 2]>,
    /// The triangles over `points`, all of one orientation.
    pub triangles: Vec<[u32; 3]>,
    /// The boundary edges over `points`, ink on the left.
    pub edges: Vec<[u32; 2]>,
}

impl Outline {
    /// The outline of a triangle soup, each triangle
    /// `[x0, y0, x1, y1, x2, y2]`.
    pub(super) fn new(soup: &[[f32; 6]]) -> Self {
        let mut points: Vec<[f32; 2]> = Vec::new();
        let mut grid: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        let mut corner = |x: f32, y: f32| -> u32 {
            let (cx, cy) = ((x / SNAP).floor() as i32, (y / SNAP).floor() as i32);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for &id in grid.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                        let p = points[id as usize];
                        if (p[0] - x).abs() <= SNAP && (p[1] - y).abs() <= SNAP {
                            return id;
                        }
                    }
                }
            }
            let id = points.len() as u32;
            points.push([x, y]);
            grid.entry((cx, cy)).or_default().push(id);
            id
        };

        let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(soup.len());
        for t in soup {
            let ids = [corner(t[0], t[1]), corner(t[2], t[3]), corner(t[4], t[5])];
            triangles.push(ids);
        }
        // Snapping can collapse a sliver to a line: drop it, and turn
        // the rest to one orientation.
        triangles.retain_mut(|ids| {
            let [a, b, c] = ids.map(|i| points[i as usize]);
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if area == 0.0 {
                return false;
            }
            if area < 0.0 {
                ids.swap(1, 2);
            }
            true
        });

        let edges = boundary(&points, &triangles);
        let miters = miters(&points, &edges);
        Self {
            points,
            miters,
            triangles,
            edges,
        }
    }
}

/// The boundary of the triangles' union: their directed edges, the
/// horizontal ones split at the corners lying on them, less the pairs
/// that cancel.
fn boundary(points: &[[f32; 2]], triangles: &[[u32; 3]]) -> Vec<[u32; 2]> {
    // The points by height, to find the corners on a horizontal edge.
    let mut by_height: Vec<u32> = (0..points.len() as u32).collect();
    by_height.sort_by(|&a, &b| points[a as usize][1].total_cmp(&points[b as usize][1]));

    let mut directed: Vec<[u32; 2]> = Vec::with_capacity(triangles.len() * 3);
    for &[a, b, c] in triangles {
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let (p, q) = (points[from as usize], points[to as usize]);
            if p[1] != q[1] {
                directed.push([from, to]);
                continue;
            }
            // Level (a seam between slabs; a sloped edge is a side,
            // however shallow): the corners strictly between its ends
            // at its height split it.
            let level = p[1];
            let (lo, hi) = (p[0].min(q[0]), p[0].max(q[0]));
            let first = by_height.partition_point(|&i| points[i as usize][1] < level);
            let mut on: Vec<u32> = by_height[first..]
                .iter()
                .copied()
                .take_while(|&i| points[i as usize][1] == level)
                .filter(|&i| i != from && i != to)
                .filter(|&i| lo < points[i as usize][0] && points[i as usize][0] < hi)
                .collect();
            on.sort_by(|&i, &j| points[i as usize][0].total_cmp(&points[j as usize][0]));
            if p[0] > q[0] {
                on.reverse();
            }
            let mut previous = from;
            for next in on.into_iter().chain([to]) {
                directed.push([previous, next]);
                previous = next;
            }
        }
    }

    let present: HashSet<[u32; 2]> = directed.iter().copied().collect();
    let mut seen = HashSet::new();
    directed
        .into_iter()
        .filter(|&[from, to]| !present.contains(&[to, from]) && seen.insert([from, to]))
        .collect()
}

/// The outward miter vector at each point, from the boundary edges
/// that meet there.
fn miters(points: &[[f32; 2]], edges: &[[u32; 2]]) -> Vec<[f32; 2]> {
    let mut sums = vec![([0.0f32; 2], 0u32); points.len()];
    for &[from, to] in edges {
        let (p, q) = (points[from as usize], points[to as usize]);
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        let length = dx.hypot(dy);
        if length == 0.0 {
            continue;
        }
        // Ink is on the left of (dx, dy); outward is the right.
        let normal = [dy / length, -dx / length];
        for id in [from, to] {
            let (sum, count) = &mut sums[id as usize];
            sum[0] += normal[0];
            sum[1] += normal[1];
            *count += 1;
        }
    }
    sums.into_iter()
        .map(|(sum, count)| {
            if count == 0 {
                return [0.0, 0.0];
            }
            let mean = [sum[0] / count as f32, sum[1] / count as f32];
            let squared = mean[0] * mean[0] + mean[1] * mean[1];
            if squared < 1e-6 {
                // Edges doubling back on themselves: nothing to offset.
                return [0.0, 0.0];
            }
            // mean / |mean|^2 is the unit normal over the cosine of
            // half the corner.
            let mut miter = [mean[0] / squared, mean[1] / squared];
            let length = miter[0].hypot(miter[1]);
            if length > MITER_LIMIT {
                miter = [
                    miter[0] * MITER_LIMIT / length,
                    miter[1] * MITER_LIMIT / length,
                ];
            }
            miter
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Outline, MITER_LIMIT};
    use crate::logo::paths;
    use std::collections::HashMap;

    fn outlines() -> [(&'static str, Outline); 2] {
        [
            ("mark", Outline::new(paths::MARK_TRIANGLES)),
            ("wordmark", Outline::new(paths::WORDMARK_TRIANGLES)),
        ]
    }

    fn inside_ink(outline: &Outline, at: [f32; 2]) -> bool {
        outline.triangles.iter().any(|&[a, b, c]| {
            let [a, b, c] = [a, b, c].map(|i| outline.points[i as usize]);
            let side = |p: [f32; 2], q: [f32; 2]| {
                (q[0] - p[0]) * (at[1] - p[1]) - (q[1] - p[1]) * (at[0] - p[0])
            };
            // The triangles are all of one orientation: inside is the
            // left of every edge.
            side(a, b) > 0.0 && side(b, c) > 0.0 && side(c, a) > 0.0
        })
    }

    /// The boundary is the edge of the ink: it closes into loops (every
    /// point meets as many edges coming in as going out), and for all
    /// but a hair slit or two (an edge pair 0.07 units apart, which
    /// reads as ink on both sides) the ink lies a unit to the left of
    /// an edge's middle and the page a unit to the right. An edge that
    /// failed to cancel against its twin across a slab seam would fail
    /// the second half, and be feathered across the middle of a stem.
    #[test]
    fn the_outline_is_the_edge_of_the_ink() {
        for (name, outline) in outlines() {
            assert!(!outline.edges.is_empty(), "{name}: has an outline");
            let mut balance: HashMap<u32, i32> = HashMap::new();
            for &[from, to] in &outline.edges {
                *balance.entry(from).or_default() -= 1;
                *balance.entry(to).or_default() += 1;
            }
            assert!(
                balance.values().all(|b| *b == 0),
                "{name}: the outline closes into loops"
            );

            let mut astray = 0;
            for &[from, to] in &outline.edges {
                let (p, q) = (outline.points[from as usize], outline.points[to as usize]);
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                let length = dx.hypot(dy);
                let middle = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
                // Left of (dx, dy) is (-dy, dx): the ink side.
                let left = [middle[0] - dy / length, middle[1] + dx / length];
                let right = [middle[0] + dy / length, middle[1] - dx / length];
                if !inside_ink(&outline, left) || inside_ink(&outline, right) {
                    astray += 1;
                }
            }
            assert!(
                astray * 100 <= outline.edges.len(),
                "{name}: {astray} of {} outline edges are not between ink and page",
                outline.edges.len()
            );
        }
    }

    /// The feather offsets every outline point along its miter, a unit
    /// or more (so the ramp is at least the feather wide) and at most
    /// the miter limit.
    #[test]
    fn the_miters_are_at_least_a_unit_and_capped() {
        for (name, outline) in outlines() {
            let mut on_outline = vec![false; outline.points.len()];
            for &[from, to] in &outline.edges {
                on_outline[from as usize] = true;
                on_outline[to as usize] = true;
            }
            let mut slits = 0;
            for (miter, on) in outline.miters.iter().zip(on_outline) {
                let length = miter[0].hypot(miter[1]);
                if !on {
                    assert_eq!(length, 0.0, "{name}: an inner point is not offset");
                } else if length == 0.0 {
                    slits += 1;
                } else {
                    assert!(
                        (1.0 - 1e-4..=MITER_LIMIT + 1e-4).contains(&length),
                        "{name}: miter {length}"
                    );
                }
            }
            assert!(
                slits * 100 <= outline.points.len(),
                "{name}: {slits} points double back on themselves"
            );
        }
    }
}

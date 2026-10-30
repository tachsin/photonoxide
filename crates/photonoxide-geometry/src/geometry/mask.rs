//! Masks: a 2D array of fill values from 0 to 1 on a grid of pixels, as a density-based design
//! gives them (inverse design, 0.7) or as a file holds them.
//!
//! A mask is two things, and each question is answered by the one it is about:
//!
//! - **a density** ρ, constant over each pixel. [`Region::fill`] is ∫ρ over the cell, exactly
//!   (each pixel's value times the area it shares with the cell), and [`Region::area`] ∫ρ over
//!   the mask, so that averaging over a grid's cells sees the grey pixels as the fractions they
//!   are, and the fills of a grid sum to the area;
//! - **the design binarised at ρ ≥ ½**, the shape as it would be made: [`Region::locate`] (and so
//!   `contains`, which point sampling reads), [`Region::distance`], [`Region::polygons`] and
//!   [`Region::perimeter`] are those of the union of the pixels with ρ ≥ ½, exactly (its
//!   outline is the pixels' edges; pixels that meet only at a corner are separate polygons).
//!
//! For a mask of zeros and ones the two are the same region. [`Region::bounds`] is the box of
//! the pixels with ρ > 0, which holds both.

use std::sync::Arc;

use super::boundary::{self, P, Piece};
use super::curves::Frame;
use super::index::Index;
use super::primitives::um;
use super::region::{Distance, Location, Polygons, Region, Tolerance, assemble};
use super::transform::Transform;
use super::{Bounds, Point, bounds_um};
use crate::units::Length;
use crate::{Error, Result};

/// A mask: fill values from 0 to 1 on a grid of pixels. See the module for what it answers.
/// Cheap to clone.
#[derive(Clone, Debug)]
pub struct Mask {
    data: Arc<Data>,
}

#[derive(Clone, Debug, PartialEq)]
struct Spec {
    /// The lower left corner of the grid, in the mask's own frame.
    corner: P,
    pixel: [f64; 2],
    nx: usize,
    ny: usize,
    /// x fastest, from the bottom row up.
    values: Vec<f64>,
    frame: Frame,
}

impl PartialEq for Mask {
    fn eq(&self, other: &Mask) -> bool {
        self.data.spec == other.data.spec
    }
}

#[derive(Debug)]
struct Data {
    spec: Spec,
    /// The binarised design's outline, loop by loop, in the plane, the region on the left.
    loops: Vec<Vec<P>>,
    groups: Vec<(usize, Vec<usize>)>,
    /// Every edge of the outline, (loop, vertex), and their boxes.
    edges: Vec<(usize, usize)>,
    index: Index,
    bounds: Bounds,
    area: f64,
}

fn mask_error(reason: impl Into<String>) -> Error {
    Error::invalid("mask", reason.into())
}

/// Whether the closed ring holds `p` (even-odd).
fn holds(ring: &[P], p: P) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn shoelace(ring: &[P]) -> f64 {
    let n = ring.len();
    let o = ring[0];
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            (a[0] - o[0]) * (b[1] - o[1]) - (b[0] - o[0]) * (a[1] - o[1])
        })
        .sum::<f64>()
        / 2.0
}

impl Spec {
    fn value(&self, i: usize, j: usize) -> f64 {
        self.values[j * self.nx + i]
    }

    /// Whether pixel (i, j) is in the binarised design; outside the grid, not.
    fn on(&self, i: isize, j: isize) -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < self.nx
            && (j as usize) < self.ny
            && self.value(i as usize, j as usize) >= 0.5
    }

    /// The grid's node (i, j) in the plane.
    fn node(&self, i: usize, j: usize) -> P {
        self.frame.to.map_um([
            self.corner[0] + i as f64 * self.pixel[0],
            self.corner[1] + j as f64 * self.pixel[1],
        ])
    }
}

impl Mask {
    /// The mask with lower left corner `corner` and pixels of `pixel` (along x, along y), `nx`
    /// pixels along x, and the fill `values`, x fastest from the bottom row up (pixel (i, j) is
    /// `values[j nx + i]`).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the corner is finite, the pixels' sides positive and
    /// finite, there are nx ≥ 1 pixels along x and a whole number of rows, every value is from 0
    /// to 1, and one at least is ½ or more (the binarised design isn't empty).
    pub fn new(corner: Point, pixel: [Length; 2], nx: usize, values: Vec<f64>) -> Result<Mask> {
        let c = um(corner);
        let (dx, dy) = (pixel[0].to_um(), pixel[1].to_um());
        if !(c[0].is_finite() && c[1].is_finite()) {
            return Err(mask_error("needs a finite corner"));
        }
        if !(dx.is_finite() && dy.is_finite() && dx > 0.0 && dy > 0.0) {
            return Err(mask_error(format!(
                "needs pixels of positive size, got {} x {}",
                pixel[0], pixel[1]
            )));
        }
        if nx == 0 || values.is_empty() || !values.len().is_multiple_of(nx) {
            return Err(mask_error(format!(
                "needs whole rows of {nx} pixels, got {} values",
                values.len()
            )));
        }
        if let Some((k, v)) = values
            .iter()
            .enumerate()
            .find(|(_, v)| !(0.0..=1.0).contains(*v))
        {
            return Err(mask_error(format!(
                "its fill values must be from 0 to 1: pixel ({}, {}) is {v}",
                k % nx,
                k / nx
            )));
        }
        if !values.iter().any(|v| *v >= 0.5) {
            return Err(mask_error(
                "no pixel is 1/2 or more: its binarised design, what it draws, is empty",
            ));
        }
        let ny = values.len() / nx;
        Mask::build(Spec {
            corner: c,
            pixel: [dx, dy],
            nx,
            ny,
            values,
            frame: Frame::new(Transform::IDENTITY),
        })
    }

    /// A mask from text: one line per row of pixels, the top row first (as the picture reads),
    /// values separated by spaces, tabs or commas; empty lines and lines starting with `#` are
    /// skipped. As [`Mask::new`] otherwise.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a value that isn't a number, rows of different lengths, or
    /// what [`Mask::new`] refuses.
    pub fn parse(text: &str, corner: Point, pixel: [Length; 2]) -> Result<Mask> {
        let mut rows: Vec<Vec<f64>> = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let row = line
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse::<f64>()
                        .map_err(|_| mask_error(format!("line {}: \"{s}\" isn't a number", n + 1)))
                })
                .collect::<Result<Vec<f64>>>()?;
            if let Some(first) = rows.first()
                && first.len() != row.len()
            {
                return Err(mask_error(format!(
                    "line {}: a row of {} values, where the first has {}",
                    n + 1,
                    row.len(),
                    first.len()
                )));
            }
            rows.push(row);
        }
        Mask::from_rows(&rows, corner, pixel)
    }

    /// A mask from its rows, the top row first (as the picture reads). As [`Mask::new`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for rows of different lengths, or what [`Mask::new`] refuses.
    pub fn from_rows(rows: &[Vec<f64>], corner: Point, pixel: [Length; 2]) -> Result<Mask> {
        let nx = rows.first().map_or(0, Vec::len);
        if let Some(r) = rows.iter().find(|r| r.len() != nx) {
            return Err(mask_error(format!(
                "its rows must have the same length: {} and {nx}",
                r.len()
            )));
        }
        let values: Vec<f64> = rows.iter().rev().flatten().copied().collect();
        Mask::new(corner, pixel, nx, values)
    }

    fn build(spec: Spec) -> Result<Mask> {
        let (nx, ny) = (spec.nx, spec.ny);
        let vertex = |i: usize, j: usize| j * (nx + 1) + i;
        // the binarised design's boundary edges, region on their left: (from, to, direction)
        let mut edges: Vec<(usize, usize, u8)> = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let (ii, jj) = (i as isize, j as isize);
                if !spec.on(ii, jj) {
                    continue;
                }
                if !spec.on(ii, jj - 1) {
                    edges.push((vertex(i, j), vertex(i + 1, j), 0));
                }
                if !spec.on(ii + 1, jj) {
                    edges.push((vertex(i + 1, j), vertex(i + 1, j + 1), 1));
                }
                if !spec.on(ii, jj + 1) {
                    edges.push((vertex(i + 1, j + 1), vertex(i, j + 1), 2));
                }
                if !spec.on(ii - 1, jj) {
                    edges.push((vertex(i, j + 1), vertex(i, j), 3));
                }
            }
        }
        let vertices = (nx + 1) * (ny + 1);
        let mut out = vec![[u32::MAX; 2]; vertices];
        for (e, &(from, _, _)) in edges.iter().enumerate() {
            let slot = if out[from][0] == u32::MAX { 0 } else { 1 };
            out[from][slot] = u32::try_from(e).map_err(|_| mask_error("too many pixels"))?;
        }
        let mut used = vec![false; edges.len()];
        let mut local_loops: Vec<Vec<usize>> = Vec::new();
        for first in 0..edges.len() {
            if used[first] {
                continue;
            }
            let mut ring: Vec<usize> = Vec::new();
            let mut e = first;
            loop {
                used[e] = true;
                let (from, to, dir) = edges[e];
                ring.push(from);
                // at `to`: the edge turning left if there are two
                let options: Vec<usize> = out[to]
                    .iter()
                    .filter(|&&o| o != u32::MAX && !used[o as usize])
                    .map(|&o| o as usize)
                    .collect();
                let next = match options.as_slice() {
                    [] => break,
                    [one] => *one,
                    many => *many
                        .iter()
                        .find(|&&o| edges[o].2 == (dir + 1) % 4)
                        .unwrap_or(&many[0]),
                };
                e = next;
            }
            // only the corners: drop each vertex where the boundary runs straight on
            let n = ring.len();
            let at = |v: usize| (v % (nx + 1), v / (nx + 1));
            let kept: Vec<usize> = (0..n)
                .filter(|&k| {
                    let (a, b, c) = (
                        at(ring[(k + n - 1) % n]),
                        at(ring[k]),
                        at(ring[(k + 1) % n]),
                    );
                    let d1 = (b.0 as isize - a.0 as isize, b.1 as isize - a.1 as isize);
                    let d2 = (c.0 as isize - b.0 as isize, c.1 as isize - b.1 as isize);
                    d1.0 * d2.1 - d1.1 * d2.0 != 0
                })
                .map(|k| ring[k])
                .collect();
            local_loops.push(kept);
        }
        let mirrored = spec.frame.mirrored();
        let loops: Vec<Vec<P>> = local_loops
            .iter()
            .map(|l| {
                let mut r: Vec<P> = l
                    .iter()
                    .map(|&v| spec.node(v % (nx + 1), v / (nx + 1)))
                    .collect();
                if mirrored {
                    r.reverse();
                }
                r
            })
            .collect();
        let areas: Vec<f64> = loops.iter().map(|l| shoelace(l)).collect();
        let outers: Vec<usize> = (0..loops.len()).filter(|&l| areas[l] > 0.0).collect();
        let mut groups: Vec<(usize, Vec<usize>)> =
            outers.iter().map(|&o| (o, Vec::new())).collect();
        let quarter = 0.25 * spec.pixel[0].min(spec.pixel[1]) * spec.frame.to.factor();
        for h in (0..loops.len()).filter(|&l| areas[l] <= 0.0) {
            // a point a quarter pixel to the left of the hole's first edge: in the region
            let (a, b) = (loops[h][0], loops[h][1 % loops[h].len()]);
            let e = [b[0] - a[0], b[1] - a[1]];
            let l = e[0].hypot(e[1]);
            let p = [
                0.5 * (a[0] + b[0]) - quarter * e[1] / l,
                0.5 * (a[1] + b[1]) + quarter * e[0] / l,
            ];
            let holder = outers
                .iter()
                .enumerate()
                .filter(|(_, o)| holds(&loops[**o], p))
                .min_by(|x, y| areas[*x.1].total_cmp(&areas[*y.1]));
            if let Some((g, _)) = holder {
                groups[g].1.push(h);
            }
        }
        let mut edge_list = Vec::new();
        let mut boxes = Vec::new();
        for (l, ring) in loops.iter().enumerate() {
            let n = ring.len();
            for k in 0..n {
                edge_list.push((l, k));
                boxes.push(bounds_um(&[ring[k], ring[(k + 1) % n]]));
            }
        }
        let index = Index::new(&boxes);
        // the support's bounds: the corners of every pixel with ρ > 0
        let mut corners: Vec<P> = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                if spec.value(i, j) > 0.0 {
                    corners.extend([
                        spec.node(i, j),
                        spec.node(i + 1, j),
                        spec.node(i + 1, j + 1),
                        spec.node(i, j + 1),
                    ]);
                }
            }
        }
        let bounds = bounds_um(&corners);
        let f = spec.frame.to.factor();
        let area = spec.values.iter().sum::<f64>() * spec.pixel[0] * spec.pixel[1] * f * f;
        Ok(Mask {
            data: Arc::new(Data {
                spec,
                loops,
                groups,
                edges: edge_list,
                index,
                bounds,
                area,
            }),
        })
    }

    /// The lower left corner of the grid, in the mask's own frame (before any transform).
    pub fn corner(&self) -> Point {
        let c = self.data.spec.corner;
        Point::um(c[0], c[1])
    }

    /// The pixels' sides, along x and y (before any transform).
    pub fn pixel(&self) -> [Length; 2] {
        let p = self.data.spec.pixel;
        [Length::um(p[0]), Length::um(p[1])]
    }

    /// The number of pixels along x and along y.
    pub fn size(&self) -> [usize; 2] {
        [self.data.spec.nx, self.data.spec.ny]
    }

    /// The fill values, x fastest from the bottom row up.
    pub fn values(&self) -> &[f64] {
        &self.data.spec.values
    }

    /// The binarised design's area, µm²: the pixels with ρ ≥ ½.
    pub fn binarised_area(&self) -> f64 {
        let s = &self.data.spec;
        let f = s.frame.to.factor();
        s.values.iter().filter(|v| **v >= 0.5).count() as f64 * s.pixel[0] * s.pixel[1] * f * f
    }

    /// The mask's image under `t`: the same pixels in a frame moved by `t`.
    ///
    /// # Panics
    ///
    /// Never for a valid mask but on overflow.
    pub fn transformed(&self, t: &Transform) -> Mask {
        Mask::build(Spec {
            frame: self.data.spec.frame.then(t),
            ..self.data.spec.clone()
        })
        .expect("a transformed mask is a mask")
    }

    fn nearest(&self, p: P) -> (f64, P, P) {
        let d = &self.data;
        let first = d.loops[0][0];
        let ub = (p[0] - first[0]).hypot(p[1] - first[1]);
        let r = ub * (1.0 + 1e-12) + 1e-12;
        let window = Bounds {
            min: Point::um(p[0] - r, p[1] - r),
            max: Point::um(p[0] + r, p[1] + r),
        };
        let mut best = (f64::INFINITY, p, [1.0, 0.0]);
        for e in d.index.meeting(window) {
            let (l, k) = d.edges[e];
            let ring = &d.loops[l];
            let c = Piece::Line(ring[k], ring[(k + 1) % ring.len()]).nearest(p);
            if c.0 < best.0 {
                best = c;
            }
        }
        best
    }
}

impl Region for Mask {
    /// The binarised design's: inside where the pixel's ρ ≥ ½, on the boundary on an edge
    /// between such a pixel and one that isn't (or the grid's edge).
    fn locate(&self, p: Point) -> Location {
        let s = &self.data.spec;
        let q = s.frame.from.map_um(um(p));
        let fx = (q[0] - s.corner[0]) / s.pixel[0];
        let fy = (q[1] - s.corner[1]) / s.pixel[1];
        if !(fx >= 0.0 && fy >= 0.0 && fx <= s.nx as f64 && fy <= s.ny as f64) {
            return Location::Outside;
        }
        let around = |f: f64| -> Vec<isize> {
            let i = f.floor() as isize;
            if f == f.floor() {
                vec![i - 1, i]
            } else {
                vec![i]
            }
        };
        let (is, js) = (around(fx), around(fy));
        let mut states = is
            .iter()
            .flat_map(|&i| js.iter().map(move |&j| (i, j)))
            .map(|(i, j)| s.on(i, j));
        let first = states.next().unwrap_or(false);
        if states.any(|v| v != first) {
            Location::Boundary
        } else if first {
            Location::Inside
        } else {
            Location::Outside
        }
    }

    /// To the binarised design's outline, exactly.
    fn distance(&self, p: Point) -> Distance {
        let q = um(p);
        let inside = self.locate(p) != Location::Outside;
        Distance::from(boundary::signed(self.nearest(q), q, inside))
    }

    /// The box of the pixels with ρ > 0.
    fn bounds(&self) -> Bounds {
        self.data.bounds
    }

    /// The binarised design's outline, exactly (snapped to the tolerance's grid).
    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let d = &self.data;
        let contours = d
            .groups
            .iter()
            .map(|(o, holes)| {
                (
                    d.loops[*o].clone(),
                    holes.iter().map(|h| d.loops[*h].clone()).collect(),
                )
            })
            .collect();
        assemble(contours, 0.0, tolerance)
    }

    /// ∫ρ: each pixel's value times its area.
    fn area(&self) -> f64 {
        self.data.area
    }

    /// The binarised design's outline's length.
    fn perimeter(&self) -> Option<Length> {
        let total: f64 = self
            .data
            .loops
            .iter()
            .map(|r| {
                let n = r.len();
                (0..n)
                    .map(|k| {
                        let (a, b) = (r[k], r[(k + 1) % n]);
                        (b[0] - a[0]).hypot(b[1] - a[1])
                    })
                    .sum::<f64>()
            })
            .sum();
        Some(Length::um(total))
    }

    /// ∫ρ over the cell, exactly: each pixel's value times the area it shares with the cell.
    fn fill(&self, cell: Bounds) -> f64 {
        let d = &self.data;
        let s = &d.spec;
        if !d.bounds.overlaps(&cell) {
            return 0.0;
        }
        // the pixels the cell can meet: those under its corners' box in the mask's frame
        let [lo, hi] = cell.corners();
        let local: Vec<P> = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]]
            .iter()
            .map(|c| s.frame.from.map_um(*c))
            .collect();
        let b = bounds_um(&local);
        let [blo, bhi] = b.corners();
        let range = |lo: f64, hi: f64, o: f64, d: f64, n: usize| {
            let a = ((lo - o) / d).floor().max(0.0);
            let b = ((hi - o) / d).ceil().min(n as f64);
            (a as usize, (b as usize).max(a as usize))
        };
        let (i0, i1) = range(blo[0], bhi[0], s.corner[0], s.pixel[0], s.nx);
        let (j0, j1) = range(blo[1], bhi[1], s.corner[1], s.pixel[1], s.ny);
        let aligned = s.frame.to.axis_aligned();
        let mut sum = 0.0;
        for j in j0..j1 {
            for i in i0..i1 {
                let v = s.value(i, j);
                if v == 0.0 {
                    continue;
                }
                let corners = [
                    s.node(i, j),
                    s.node(i + 1, j),
                    s.node(i + 1, j + 1),
                    s.node(i, j + 1),
                ];
                let shared = if aligned {
                    bounds_um(&corners).overlap(&cell)
                } else {
                    let pieces: Vec<Piece> = (0..4)
                        .map(|k| Piece::Line(corners[k], corners[(k + 1) % 4]))
                        .collect();
                    boundary::fill(&[pieces], [lo, hi]).abs()
                };
                sum += v * shared;
            }
        }
        sum
    }
}

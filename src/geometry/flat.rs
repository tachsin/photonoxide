//! Polygons answering as a region: the inside test (even-odd), the nearest point of their edges
//! through a bounding-volume hierarchy, their area, perimeter and bounds, and their area inside
//! a cell by clipping. The composed regions ([`super::compose`]) keep their polygons within
//! [`super::FILL_TOLERANCE`] of their size as one of these, for what they can't answer exactly.

use super::boundary::{self, P, Piece};
use super::clip::{Part, perimeter, shoelace};
use super::index::Index;
use super::primitives::point;
use super::region::Location;
use super::{Bounds, Point};

/// Polygons with holes as a region.
#[derive(Clone, Debug)]
pub(crate) struct Flat {
    parts: Vec<Part>,
    /// Every ring as segments, the region on their left.
    loops: Vec<Vec<Piece>>,
    /// Each ring's box and signed area.
    boxes: Vec<Bounds>,
    areas: Vec<f64>,
    /// The edges, (ring, edge), in the hierarchy's order of items.
    edges: Vec<(usize, usize)>,
    index: Index,
    /// The rings' boxes, so that a point or a cell meets only the rings near it: many holes (a
    /// slab less a lattice of them) don't make every query visit every hole.
    rings: Index,
    area: f64,
    bounds: Bounds,
    /// How far the polygons are from the region they stand for, µm.
    pub deviation: f64,
}

fn bounds_of(points: impl Iterator<Item = P>) -> Bounds {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in points {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    Bounds {
        min: point(lo),
        max: point(hi),
    }
}

impl Flat {
    /// The region `parts` bound, `deviation` µm from what they stand for; `None` if empty.
    pub(crate) fn new(parts: Vec<Part>, deviation: f64) -> Option<Flat> {
        let rings: Vec<&Vec<P>> = parts
            .iter()
            .flat_map(|(o, h)| std::iter::once(o).chain(h))
            .collect();
        if rings.is_empty() {
            return None;
        }
        let loops: Vec<Vec<Piece>> = rings
            .iter()
            .map(|r| {
                let n = r.len();
                (0..n).map(|i| Piece::Line(r[i], r[(i + 1) % n])).collect()
            })
            .collect();
        let boxes: Vec<Bounds> = rings.iter().map(|r| bounds_of(r.iter().copied())).collect();
        let areas: Vec<f64> = rings.iter().map(|r| shoelace(r)).collect();
        let area = areas.iter().sum::<f64>();
        if area <= 0.0 {
            return None;
        }
        let mut edges = Vec::new();
        let mut edge_boxes = Vec::new();
        for (l, pieces) in loops.iter().enumerate() {
            for (k, piece) in pieces.iter().enumerate() {
                let Piece::Line(a, b) = piece else { continue };
                edges.push((l, k));
                edge_boxes.push(bounds_of([*a, *b].into_iter()));
            }
        }
        let index = Index::new(&edge_boxes);
        let rings = Index::new(&boxes);
        let bounds = bounds_of(parts.iter().flat_map(|(o, _)| o.iter().copied()));
        Some(Flat {
            parts,
            loops,
            boxes,
            areas,
            edges,
            index,
            rings,
            area,
            bounds,
            deviation,
        })
    }

    pub(crate) fn area(&self) -> f64 {
        self.area
    }

    pub(crate) fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub(crate) fn perimeter(&self) -> f64 {
        self.parts
            .iter()
            .flat_map(|(o, h)| std::iter::once(o).chain(h))
            .map(|r| perimeter(r))
            .sum()
    }

    /// Inside, on an edge or outside: even-odd over every ring, the edges near `p` tried first.
    pub(crate) fn locate(&self, p: Point) -> Location {
        let q = [p.x.to_um(), p.y.to_um()];
        if !self.bounds.contains(p) {
            return Location::Outside;
        }
        let on = self.index.any_at(p, |e| {
            let (l, k) = self.edges[e];
            let Piece::Line(a, b) = self.loops[l][k] else {
                return false;
            };
            let c = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
            c == 0.0
                && q[0] >= a[0].min(b[0])
                && q[0] <= a[0].max(b[0])
                && q[1] >= a[1].min(b[1])
                && q[1] <= a[1].max(b[1])
        });
        if on {
            return Location::Boundary;
        }
        // a ray along +x crosses a closed ring whose box doesn't hold p an even number of times
        // (both its ends are outside the ring), so only the rings whose boxes hold p count
        let mut inside = false;
        self.rings.any_at(p, |l| {
            super::array::tested();
            let b = &self.boxes[l];
            if q[1] < b.min.y.to_um() || q[1] > b.max.y.to_um() || q[0] > b.max.x.to_um() {
                return false;
            }
            for piece in &self.loops[l] {
                let Piece::Line(a, c) = piece else { continue };
                if (a[1] > q[1]) != (c[1] > q[1])
                    && q[0] < (c[0] - a[0]) * (q[1] - a[1]) / (c[1] - a[1]) + a[0]
                {
                    inside = !inside;
                }
            }
            false
        });
        if inside {
            Location::Inside
        } else {
            Location::Outside
        }
    }

    /// The nearest point of the edges to `p` (µm): (distance, point, outward normal there), the
    /// edges in a window about `p` tried, the window widened until it holds the answer.
    pub(crate) fn nearest(&self, p: P, guess: f64) -> (f64, P, P) {
        let size = (self.bounds.max.x - self.bounds.min.x)
            .to_um()
            .max((self.bounds.max.y - self.bounds.min.y).to_um());
        let mut r = guess.abs().max(1e-6 * size).max(f64::MIN_POSITIVE);
        // a window this wide holds every edge
        let [lo, hi] = self.bounds.corners();
        let all = (p[0] - lo[0])
            .abs()
            .max((hi[0] - p[0]).abs())
            .max((p[1] - lo[1]).abs())
            .max((hi[1] - p[1]).abs());
        loop {
            let window = Bounds {
                min: point([p[0] - r, p[1] - r]),
                max: point([p[0] + r, p[1] + r]),
            };
            let mut best = (f64::INFINITY, p, [1.0, 0.0]);
            for e in self.index.meeting(window) {
                let (l, k) = self.edges[e];
                let c = self.loops[l][k].nearest(p);
                if c.0 < best.0 {
                    best = c;
                }
            }
            // every point within r of p is in the window: a nearest point found within r is
            // the nearest
            if best.0 <= r || r >= all {
                return best;
            }
            r *= 4.0;
        }
    }

    /// The signed distance (negative inside) and outward normal at `p`.
    pub(crate) fn distance(&self, p: Point, guess: f64) -> (f64, P) {
        let q = [p.x.to_um(), p.y.to_um()];
        let inside = self.locate(p) != Location::Outside;
        boundary::signed(self.nearest(q, guess), q, inside)
    }

    /// The area inside `cell`: each ring clipped to it, rings whose boxes miss the cell skipped
    /// and rings inside it taken whole.
    pub(crate) fn fill(&self, cell: Bounds) -> f64 {
        if !self.bounds.overlaps(&cell) {
            return 0.0;
        }
        if cell.encloses(&self.bounds) {
            return self.area;
        }
        let corners = cell.corners();
        let mut sum = 0.0;
        // the rings whose boxes meet the cell, in their own order, so the sum is the one taken
        // over every ring (a ring whose box misses the cell bounds none of it)
        let mut near = self.rings.meeting(cell);
        near.sort_unstable();
        for l in near {
            super::array::tested();
            let b = &self.boxes[l];
            if cell.encloses(b) {
                sum += self.areas[l];
            } else if b.overlaps(&cell) {
                sum += boundary::fill(std::slice::from_ref(&self.loops[l]), corners);
            }
        }
        sum
    }
}

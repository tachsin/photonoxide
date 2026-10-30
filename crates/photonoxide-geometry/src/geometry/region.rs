//! What every region in the plane answers: inside or not, how far from its boundary and in
//! which direction, its bounds, area and perimeter, polygons within a tolerance, and its area
//! inside a cell.

use std::cmp::Ordering;

use super::boundary::{self, P, Piece};
use super::primitives::{point, um};
use super::{Bounds, Point};
use crate::units::Length;
use crate::{Error, Result};

/// Where a point is with respect to a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    /// Inside, off the boundary.
    Inside,
    /// On the boundary, as far as the definition's arithmetic can say.
    Boundary,
    /// Outside.
    Outside,
}

impl Location {
    /// From a level function's comparison with its value on the boundary: less inside.
    pub(crate) fn of(order: Ordering) -> Location {
        match order {
            Ordering::Less => Location::Inside,
            Ordering::Equal => Location::Boundary,
            Ordering::Greater => Location::Outside,
        }
    }
}

/// The signed distance from a point to a region's boundary and the outward normal there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Distance {
    /// The distance to the nearest point of the boundary: negative inside, positive outside.
    pub signed: Length,
    /// The unit normal pointing out of the region at that point: along the line from the
    /// point to it (away from the region) when the point is off the boundary.
    pub normal: [f64; 2],
}

impl From<(f64, P)> for Distance {
    fn from((d, n): (f64, P)) -> Distance {
        Distance {
            signed: Length::um(d),
            normal: n,
        }
    }
}

/// How closely polygons follow a curved boundary, and the grid their vertices are snapped to.
///
/// Every point of the polygons is within `geometric` of the true boundary, and every point of
/// the boundary within `geometric` of the polygons: the curves are flattened to within
/// `geometric − grid/√2`, and snapping a vertex to the grid moves it by at most `grid/√2`. A
/// grid of zero snaps nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    geometric: Length,
    grid: Length,
}

impl Default for Tolerance {
    /// 1 nm, on a grid of 1 nm (GDSII's usual database unit).
    fn default() -> Tolerance {
        Tolerance {
            geometric: Length::nm(1.0),
            grid: Length::nm(1.0),
        }
    }
}

impl Tolerance {
    /// Within `geometric` of the boundary, on a grid of `grid` (zero: no grid).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both are finite, the grid at least zero and the
    /// geometric tolerance larger than the grid's own error, grid/√2.
    pub fn new(geometric: Length, grid: Length) -> Result<Tolerance> {
        let (g, d) = (geometric.to_um(), grid.to_um());
        if !(g.is_finite() && d.is_finite() && d >= 0.0 && g > d * std::f64::consts::FRAC_1_SQRT_2)
        {
            return Err(Error::invalid(
                "tolerance",
                format!(
                    "needs a finite grid of at least zero and a geometric tolerance larger than \
                     the grid's own error (grid/sqrt 2), got {geometric} on a grid of {grid}"
                ),
            ));
        }
        Ok(Tolerance { geometric, grid })
    }

    /// Within `geometric` of the boundary, with no grid.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless it is positive and finite.
    pub fn unsnapped(geometric: Length) -> Result<Tolerance> {
        Tolerance::new(geometric, Length::ZERO)
    }

    /// The largest distance between the polygons and the boundary.
    pub fn geometric(&self) -> Length {
        self.geometric
    }

    /// The grid the vertices are snapped to; zero for none.
    pub fn grid(&self) -> Length {
        self.grid
    }

    /// What the flattening of curves may take, µm: the tolerance less the snapping's error.
    pub(crate) fn budget(&self) -> f64 {
        self.geometric.to_um() - self.grid.to_um() * std::f64::consts::FRAC_1_SQRT_2
    }
}

/// A polygon with holes: its outer boundary counterclockwise and its holes clockwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Contour {
    /// The outer boundary, counterclockwise, the first vertex not repeated.
    pub outer: Vec<Point>,
    /// The holes, each clockwise.
    pub holes: Vec<Vec<Point>>,
}

impl Contour {
    /// The area, µm²: the outer boundary's less the holes'.
    pub fn area(&self) -> f64 {
        shoelace(&self.outer) + self.holes.iter().map(|h| shoelace(h)).sum::<f64>()
    }
}

/// The shoelace formula: positive counterclockwise, µm².
fn shoelace(v: &[Point]) -> f64 {
    let n = v.len();
    (0..n)
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % n]);
            a.x.to_um() * b.y.to_um() - b.x.to_um() * a.y.to_um()
        })
        .sum::<f64>()
        / 2.0
}

/// A region as polygons, with the tolerance asked for and the distance reached.
#[derive(Clone, Debug, PartialEq)]
pub struct Polygons {
    /// The polygons, each with its holes.
    pub contours: Vec<Contour>,
    /// The tolerance they were asked for.
    pub tolerance: Tolerance,
    /// The largest distance between them and the true boundary: within the tolerance, unless a
    /// curve needed more than 2²⁰ vertices for it.
    pub deviation: Length,
}

impl Polygons {
    /// Their area, µm².
    pub fn area(&self) -> f64 {
        self.contours.iter().map(Contour::area).sum()
    }
}

/// A region in the plane: what the solvers, the layout and the studio ask of a shape.
///
/// Coordinates are those of [`Point`], areas µm².
pub trait Region {
    /// Whether `p` is inside, on the boundary or outside: exactly as far as the definition's
    /// arithmetic allows.
    fn locate(&self, p: Point) -> Location;

    /// Whether the closed region contains `p`.
    fn contains(&self, p: Point) -> bool {
        self.locate(p) != Location::Outside
    }

    /// The signed distance from `p` to the boundary, negative inside, and the outward normal at
    /// the nearest point.
    fn distance(&self, p: Point) -> Distance;

    /// The smallest axis-aligned box holding the region.
    fn bounds(&self) -> Bounds;

    /// The region as polygons with holes, every point within `tolerance` of the true boundary,
    /// snapped to its grid.
    fn polygons(&self, tolerance: Tolerance) -> Polygons;

    /// The area, µm².
    fn area(&self) -> f64;

    /// The length of the boundary, where it is defined.
    fn perimeter(&self) -> Option<Length>;

    /// The area of the region inside the axis-aligned `cell`, µm²: what averaging over a grid's
    /// cells needs.
    fn fill(&self, cell: Bounds) -> f64;
}

impl Bounds {
    /// The box with corners `a` and `b`, in either order.
    pub fn new(a: Point, b: Point) -> Bounds {
        Bounds {
            min: Point {
                x: Length::um(a.x.to_um().min(b.x.to_um())),
                y: Length::um(a.y.to_um().min(b.y.to_um())),
            },
            max: Point {
                x: Length::um(a.x.to_um().max(b.x.to_um())),
                y: Length::um(a.y.to_um().max(b.y.to_um())),
            },
        }
    }

    /// Whether the two boxes share any point, edges included.
    pub fn overlaps(&self, other: &Bounds) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    /// Whether this box holds all of `other`.
    pub fn encloses(&self, other: &Bounds) -> bool {
        self.min.x <= other.min.x
            && self.min.y <= other.min.y
            && self.max.x >= other.max.x
            && self.max.y >= other.max.y
    }

    /// The area, µm².
    pub fn area(&self) -> f64 {
        (self.max.x - self.min.x).to_um() * (self.max.y - self.min.y).to_um()
    }

    /// The area this box shares with `other`, µm².
    pub fn overlap(&self, other: &Bounds) -> f64 {
        let w = self.max.x.to_um().min(other.max.x.to_um())
            - self.min.x.to_um().max(other.min.x.to_um());
        let h = self.max.y.to_um().min(other.max.y.to_um())
            - self.min.y.to_um().max(other.min.y.to_um());
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    }

    pub(crate) fn corners(&self) -> [P; 2] {
        [um(self.min), um(self.max)]
    }
}

/// A region's area inside `cell` from its boundary's pieces, with the cases that need no
/// clipping first.
pub(crate) fn fill_of(loops: &[Vec<Piece>], bounds: Bounds, area: f64, cell: Bounds) -> f64 {
    if !bounds.overlaps(&cell) {
        return 0.0;
    }
    if cell.encloses(&bounds) {
        return area;
    }
    boundary::fill(loops, cell.corners())
}

/// A region's polygons from its boundary's pieces: the first loop the outer boundary, the
/// others its holes.
pub(crate) fn polygons_of(loops: &[Vec<Piece>], tolerance: Tolerance) -> Polygons {
    let budget = tolerance.budget();
    let mut dev = 0.0f64;
    let mut rings = loops.iter().map(|l| {
        let (v, d) = boundary::polygon(l, budget);
        dev = dev.max(d);
        v
    });
    let outer = rings.next().unwrap_or_default();
    let holes: Vec<Vec<P>> = rings.collect();
    assemble(vec![(outer, holes)], dev, tolerance)
}

/// Polygons from their vertices (µm), snapped to the tolerance's grid: consecutive vertices
/// that snap together merge, and a ring left with fewer than 3 vertices or no area goes.
pub(crate) fn assemble(
    contours: Vec<(Vec<P>, Vec<Vec<P>>)>,
    deviation: f64,
    tolerance: Tolerance,
) -> Polygons {
    let grid = tolerance.grid().to_um();
    let snap = |ring: Vec<P>| -> Option<Vec<Point>> {
        let mut out: Vec<P> = Vec::with_capacity(ring.len());
        for v in ring {
            let v = if grid > 0.0 {
                [(v[0] / grid).round() * grid, (v[1] / grid).round() * grid]
            } else {
                v
            };
            if out.last() != Some(&v) {
                out.push(v);
            }
        }
        while out.len() > 1 && out.first() == out.last() {
            out.pop();
        }
        let points: Vec<Point> = out.into_iter().map(point).collect();
        (points.len() >= 3 && shoelace(&points) != 0.0).then_some(points)
    };
    let contours = contours
        .into_iter()
        .filter_map(|(outer, holes)| {
            Some(Contour {
                outer: snap(outer)?,
                holes: holes.into_iter().filter_map(snap).collect(),
            })
        })
        .collect();
    let snapped = if grid > 0.0 {
        grid * std::f64::consts::FRAC_1_SQRT_2
    } else {
        0.0
    };
    Polygons {
        contours,
        tolerance,
        deviation: Length::um(deviation + snapped),
    }
}

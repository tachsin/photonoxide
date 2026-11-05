//! A device's geometry cut into the sections of a 3D eigenmode expansion ([`crate::eme::Eme3d`]):
//! its cross-section at each x, a plane of Yee's grid across y and z.
//!
//! The device runs along x, its ports facing ±x. A section's plane holds the device's y as the
//! plane's x and its z as the plane's y, so that the expansion's z is the device's x (the three
//! axes keep their handedness: (y, z, x) is a turn of (x, y, z)). The cut is made at every x where
//! an outline turns (each vertex of every shape's polygons, on every layer); between two such x a
//! cross-section whose outlines run along x is one section, and a run of sloped outlines (a
//! taper's sides, a curve's polygon) is cut into equal steps no longer than a stated length, each
//! the cross-section at its middle (the midpoint rule: second order in the step). The two ends are
//! the ports' guides, led straight out of the device, as sections of no length: the S-matrix is
//! between their modes, at the device's ports.
//!
//! Each value of E sees the structure's permittivity averaged over its cell of the plane's dual
//! grid from 8 × 8 points ([`Plane::new`]), as 3D FDFD averages a cell of a structure uniform
//! along x ([`crate::fdfd::Solver3d::from_structure`] by [`crate::averaging::Averaging::Samples`]).

use std::cell::RefCell;

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use crate::eme::{Plane, PlaneGrid, Section3d, Wall, Walls};
use crate::geometry::{Point, Region, Tolerance};
use crate::stack::Structure;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::Geometry;

/// How a device is cut into sections: the plane's cells across (`step`, along the device's y)
/// and up (`step_z`), the cladding kept beside the core's shapes and below and above its layer,
/// the longest step of a sloped run (`along`), and whether only the half y ≥ 0 is kept, an
/// electric wall on the device's axis (`mirror`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Cut {
    /// The cells' width across the device (its y), µm.
    pub step: f64,
    /// Their height (its z), µm.
    pub step_z: f64,
    /// The cladding kept beside the core layer's shapes, µm.
    pub beside: f64,
    /// The cladding kept below and above the core layer, µm.
    pub vertical: f64,
    /// The longest step of a run of sloped outlines, µm.
    pub along: f64,
    /// Only y ≥ 0, an electric wall at y = 0: a device mirror-symmetric about its axis, the
    /// modes whose E_y is even about it (the TE-like ones).
    pub mirror: bool,
    /// Cut at every x where the plane's averaging sees a sloped outline cross one of its
    /// sampling points, instead of in steps of `along`: each section is then the discretized
    /// cross-section over its whole length, and the cut has no step error.
    pub exact: bool,
}

impl Cut {
    /// Cells `step` µm across and `step_z` µm high, steps of sloped runs at most `along` µm,
    /// 0.5 µm of cladding beside and below and above, the whole window.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a step that isn't positive and finite.
    pub fn new(step: f64, step_z: f64, along: f64) -> Result<Cut> {
        let cut = Cut {
            step,
            step_z,
            beside: 0.5,
            vertical: 0.5,
            along,
            mirror: false,
            exact: false,
        };
        cut.check()?;
        Ok(cut)
    }

    /// The same cut made at every x where the plane changes (the field `exact`): where a sloped
    /// outline crosses one of the points each cell's permittivity is averaged from (8 a cell
    /// across, at odd sixteenths of it), so that every section's cross-section holds over its
    /// whole length. Exact for outlines whose walls are vertical; a run of curves cut so takes
    /// as many sections as its outline crosses those points.
    #[must_use]
    pub fn exactly(self) -> Cut {
        Cut {
            exact: true,
            ..self
        }
    }

    /// The same cut with `beside` µm of cladding beside the core's shapes and `vertical` below
    /// and above its layer.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a margin that is negative or not finite.
    pub fn with_margins(self, beside: f64, vertical: f64) -> Result<Cut> {
        let cut = Cut {
            beside,
            vertical,
            ..self
        };
        cut.check()?;
        Ok(cut)
    }

    /// The same cut on the half y ≥ 0, an electric wall on the device's axis.
    #[must_use]
    pub fn mirrored(self) -> Cut {
        Cut {
            mirror: true,
            ..self
        }
    }

    fn check(&self) -> Result<()> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let margin = |v: f64| v.is_finite() && v >= 0.0;
        if positive(self.step)
            && positive(self.step_z)
            && positive(self.along)
            && margin(self.beside)
            && margin(self.vertical)
        {
            Ok(())
        } else {
            Err(Error::invalid(
                "eme cut",
                format!("needs positive steps and margins of at least 0, got {self:?}"),
            ))
        }
    }

    /// The plane every section of `structure` is on: across the core layer's shapes and
    /// `beside` more (from y = 0 when mirrored; else an even number of cells centred on them),
    /// and up from `vertical` below the layer to `vertical` above it, its faces on nodes when
    /// the layer is a whole number of `step_z` thick.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a stack without the layer `core` or a layer without shapes.
    pub fn grid(&self, structure: &Structure, core: &str) -> Result<PlaneGrid> {
        self.check()?;
        let (_, bottom, top) = structure
            .stack()
            .layer(core)
            .ok_or_else(|| Error::invalid("eme cut", format!("the stack has no layer {core}")))?;
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for shape in structure.shapes(core) {
            let b = shape.bounds();
            lo = lo.min(b.min.y.to_um());
            hi = hi.max(b.max.y.to_um());
        }
        if !(lo.is_finite() && hi.is_finite()) {
            return Err(Error::invalid(
                "eme cut",
                format!("the layer {core} has no shapes to cut"),
            ));
        }
        // a count of cells, a whole number to within round-off taken as it is
        let cells = |v: f64| (v - 1e-9).ceil().max(1.0) as usize;
        let h = self.step;
        let (nx, x0) = if self.mirror {
            (cells((hi + self.beside) / h), 0.0)
        } else {
            let n = 2 * cells((hi - lo + 2.0 * self.beside) / (2.0 * h));
            (n, 0.5 * (lo + hi) - 0.5 * n as f64 * h)
        };
        let hz = self.step_z;
        let (b, t) = (bottom.to_um(), top.to_um());
        let below = if self.vertical > 0.0 {
            cells(self.vertical / hz)
        } else {
            0
        };
        let core_cells = ((t - b) / hz).round().max(1.0) as usize;
        let grid = PlaneGrid::new(
            nx,
            core_cells + 2 * below,
            (h, hz),
            (x0, b - below as f64 * hz),
        );
        Ok(if self.mirror {
            grid.with_walls(Walls {
                west: Wall::Electric,
                ..Walls::default()
            })
        } else {
            grid
        })
    }

    /// `geometry` cut into sections at `wavelength`, from its ports on the left (facing 180°)
    /// to those on the right (facing 0°), its ports' guides led straight out on the layer of
    /// its first port.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for ports that don't face along x or not on both sides; and
    /// [`Cut::structure`]'s.
    pub fn geometry(&self, geometry: &Geometry, wavelength: Wavelength) -> Result<Sections> {
        let (structure, core, ends) = led(geometry)?;
        self.structure(&structure, &core, ends, wavelength)
    }

    /// The plane [`Cut::geometry`] cuts `geometry` on, without cutting it.
    ///
    /// # Errors
    ///
    /// As [`Cut::geometry`], but for the materials'.
    pub fn geometry_grid(&self, geometry: &Geometry) -> Result<PlaneGrid> {
        let (structure, core, _) = led(geometry)?;
        self.grid(&structure, &core)
    }
}

/// `geometry`'s structure with its ports' guides led out on its first port's layer, that layer,
/// and its left and right ports' x (µm).
fn led(geometry: &Geometry) -> Result<(Structure, String, (f64, f64))> {
    {
        let core = geometry
            .ports
            .first()
            .map(|p| p.layer.clone())
            .ok_or_else(|| Error::invalid("eme cut", "a device without ports"))?;
        let (mut left, mut right) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &geometry.ports {
            let x = p.position.x.to_um();
            if p.direction == 180.0 {
                left = left.min(x);
            } else if p.direction == 0.0 {
                right = right.max(x);
            } else {
                return Err(Error::invalid(
                    "eme cut",
                    format!(
                        "a device cut along x takes ports facing 0 or 180 degrees, not {}",
                        p.direction
                    ),
                ));
            }
        }
        if !(left.is_finite() && right.is_finite() && right > left) {
            return Err(Error::invalid(
                "eme cut",
                "a device cut along x needs ports on its left and on its right",
            ));
        }
        let structure = super::solve::led(geometry, &core, LEAD)?;
        Ok((structure, core, (left, right)))
    }
}

impl Cut {
    /// `structure` cut into sections from x = `a` to `b` µm at `wavelength`: the cross-section
    /// just outside each end as a section of no length (a port's guide, which must run on
    /// straight), and between them the sections and steps of the module's docs.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for `b` not after `a`, bodies over the stack (whose outlines the
    /// cut doesn't read), a structure that isn't mirror-symmetric where the cut is mirrored, and
    /// [`Cut::grid`]'s and the materials' errors ([`Error::OutsideValidity`] for one without data
    /// at `wavelength`).
    pub fn structure(
        &self,
        structure: &Structure,
        core: &str,
        (a, b): (f64, f64),
        wavelength: Wavelength,
    ) -> Result<Sections> {
        if !(a.is_finite() && b.is_finite() && b > a) {
            return Err(Error::invalid(
                "eme cut",
                format!("cuts from one x to a larger one, got {a} to {b} um"),
            ));
        }
        if !structure.bodies().is_empty() {
            return Err(Error::invalid(
                "eme cut",
                "a structure with bodies over its stack: the cut reads the layers' outlines only",
            ));
        }
        let grid = self.grid(structure, core)?;
        let pieces = pieces(structure, (a, b), self.along, self.exact.then_some(grid));
        let end = END * self.step.min(self.along);
        let mut sections = Vec::with_capacity(pieces.len() + 2);
        let mut edges = Vec::with_capacity(pieces.len() + 1);
        sections.push(Section3d::new(
            self.plane(structure, grid, a - end, wavelength)?,
            Length::ZERO,
        ));
        edges.push(a);
        // the planes side by side, collected in order: the same bits on any number of threads
        let planes: Vec<Plane> = pieces
            .par_iter()
            .map(|&(x0, x1)| self.plane(structure, grid, 0.5 * (x0 + x1), wavelength))
            .collect::<Result<_>>()?;
        for (&(x0, x1), plane) in pieces.iter().zip(planes) {
            sections.push(Section3d::new(plane, Length::um(x1 - x0)));
            edges.push(x1);
        }
        sections.push(Section3d::new(
            self.plane(structure, grid, b + end, wavelength)?,
            Length::ZERO,
        ));
        Ok(Sections {
            sections,
            edges,
            grid,
        })
    }

    /// The cross-section of `structure` at x on `grid`.
    fn plane(
        &self,
        structure: &Structure,
        grid: PlaneGrid,
        x: f64,
        wavelength: Wavelength,
    ) -> Result<Plane> {
        let failed: RefCell<Option<Error>> = RefCell::new(None);
        let eps = |y: f64, z: f64| -> c64 {
            match structure.permittivity_at(Point::um(x, y), Length::um(z), wavelength) {
                Ok(e) => e,
                Err(e) => {
                    failed.borrow_mut().get_or_insert(e);
                    c64::new(1.0, 0.0)
                }
            }
        };
        if self.mirror {
            // the half stands for the whole only if the whole is its mirror image: compared at
            // each cell's centre, a point on an outline (closed on one side, open on the other
            // by its snapped vertices) compared a picometre either way too
            let d = 1e-6;
            let differ = |y: f64, z: f64| {
                eps(y, z) != eps(-y, z)
                    && eps(y + d, z) != eps(-y - d, z)
                    && eps(y - d, z) != eps(-y + d, z)
            };
            for i in 0..=grid.nx {
                for j in 0..=grid.ny {
                    let (y, z) = (grid.x(i) + 0.5 * grid.dx, grid.y(j) + 0.5 * grid.dy);
                    if differ(y, z) {
                        return Err(Error::invalid(
                            "eme cut",
                            format!(
                                "mirrored about y = 0, but the structure at x = {x} um isn't its \
                                 mirror image (at y = {y}, z = {z} um)"
                            ),
                        ));
                    }
                }
            }
        }
        let plane = Plane::new(grid, eps)?;
        match failed.into_inner() {
            Some(e) => Err(e),
            None => Ok(plane),
        }
    }
}

/// The length a port's guide is led out of a device before the cut reads its cross-section, µm.
const LEAD: f64 = 1.0;

/// How far outside each end the end sections' cross-sections are read, in the smaller of the
/// cell's width and the longest step.
const END: f64 = 0.5;

/// A structure cut into sections ([`Cut::structure`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Sections {
    /// The sections in order along x: the first and last of no length (the ports' guides),
    /// between them each piece's cross-section at its middle.
    pub sections: Vec<Section3d>,
    /// Where each section ends, µm along x: the first's is the cut's start, the last's its end.
    pub edges: Vec<f64>,
    /// The plane they share.
    pub grid: PlaneGrid,
}

/// The pieces between `a` and `b` (start, end, µm): each interval between two x where an
/// outline turns, outlines along x across it, as one; and each run of such intervals with a
/// sloped outline cut into equal steps no longer than `along`; or, `exact` on a grid, every
/// interval between the x where a sloped outline crosses one of the grid's sampling points
/// across (odd sixteenths of a cell from its low wall) as one.
fn pieces(
    structure: &Structure,
    (a, b): (f64, f64),
    along: f64,
    exact: Option<PlaneGrid>,
) -> Vec<(f64, f64)> {
    // every edge of every shape's polygons, on every layer
    let mut edges: Vec<[[f64; 2]; 2]> = Vec::new();
    for layer in structure.stack().layers() {
        for shape in structure.shapes(&layer.name) {
            for contour in shape.polygons(Tolerance::default()).contours {
                for ring in std::iter::once(&contour.outer).chain(&contour.holes) {
                    for (k, p) in ring.iter().enumerate() {
                        let q = ring[(k + 1) % ring.len()];
                        edges.push([[p.x.to_um(), p.y.to_um()], [q.x.to_um(), q.y.to_um()]]);
                    }
                }
            }
        }
    }
    // the x where an outline turns, inside (a, b), merged within a picometre
    let close = 1e-6;
    let mut xs: Vec<f64> = edges
        .iter()
        .map(|e| e[0][0])
        .filter(|&x| x > a + close && x < b - close)
        .collect();
    xs.push(a);
    xs.push(b);
    if let Some(g) = exact {
        // the sampling points across sit at x0 + (2m + 1) dx/16 (the plane's x, the device's
        // y); those past a wall read their mirror image inside, so only the window's count
        let level = g.dx / 16.0;
        let (low, high) = (g.x0, g.x(g.nx));
        for e in &edges {
            let ([xa, ya], [xb, yb]) = (e[0], e[1]);
            if ya == yb || (xa - xb).abs() <= close {
                continue;
            }
            let (lo, hi) = (ya.min(yb).max(low), ya.max(yb).min(high));
            let mut m = (((lo - g.x0) / level - 1.0) / 2.0).floor() as i64;
            loop {
                let y = g.x0 + (2 * m + 1) as f64 * level;
                if y >= hi {
                    break;
                }
                if y > lo {
                    let x = xa + (y - ya) * (xb - xa) / (yb - ya);
                    if x > a + close && x < b - close {
                        xs.push(x);
                    }
                }
                m += 1;
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|x, y| (*x - *y).abs() < close);
    if exact.is_some() {
        return xs.windows(2).map(|w| (w[0], w[1])).collect();
    }
    // an interval is sloped where an edge across all of it isn't along x
    let sloped = |x0: f64, x1: f64| {
        edges.iter().any(|e| {
            let (lo, hi) = (e[0][0].min(e[1][0]), e[0][0].max(e[1][0]));
            lo <= x0 + close && hi >= x1 - close && hi - lo > close && e[0][1] != e[1][1]
        })
    };
    let mut pieces = Vec::new();
    let mut run: Option<(f64, f64)> = None;
    let flush = |run: &mut Option<(f64, f64)>, pieces: &mut Vec<(f64, f64)>| {
        if let Some((s, e)) = run.take() {
            let n = ((e - s) / along - 1e-9).ceil().max(1.0) as usize;
            let h = (e - s) / n as f64;
            for k in 0..n {
                let end = if k + 1 == n {
                    e
                } else {
                    s + (k + 1) as f64 * h
                };
                pieces.push((s + k as f64 * h, end));
            }
        }
    };
    for w in xs.windows(2) {
        let (x0, x1) = (w[0], w[1]);
        if sloped(x0, x1) {
            run = Some(run.map_or((x0, x1), |(s, _)| (s, x1)));
        } else {
            flush(&mut run, &mut pieces);
            pieces.push((x0, x1));
        }
    }
    flush(&mut run, &mut pieces);
    pieces
}

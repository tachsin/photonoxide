//! A layer's feature sizes, measured with the geometry kernel's offsets: its narrowest line and
//! its narrowest gap, which decide the lithographies that can write it.
//!
//! The shapes are joined into one region first. A line w wide breaks or disappears when the
//! region is shrunk by w/2; a gap g wide closes when it is grown by g/2. So the narrowest line is
//! twice the least shrinking that changes the region's pieces or holes, and the narrowest gap
//! twice the least growing that does; both are found by bisection and given to
//! [`RESOLUTION`]. A short stub that vanishes without changing the pieces or holes is not seen.

use serde::{Deserialize, Serialize};

use crate::geometry::{Join, Operation, Polygons, Region, Shape, Tolerance};
use crate::units::Length;
use crate::{Error, Result};

/// What a line or a gap is given to, µm (0.01 nm): the bisection brackets it a hundred times
/// closer, and the result is rounded to it, so a drawn width on that grid is measured exactly.
pub const RESOLUTION: f64 = 1e-5;

/// How closely the bisection brackets a half-width, µm.
const STEP: f64 = RESOLUTION / 100.0;

fn rounded(v: f64) -> f64 {
    (v / RESOLUTION).round() * RESOLUTION
}

/// A layer's narrowest line and narrowest gap, µm; `None` for a gap where nothing is beside
/// anything.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Features {
    /// The narrowest line, µm.
    pub line_um: f64,
    /// The narrowest gap, µm.
    pub gap_um: Option<f64>,
}

impl Features {
    /// The smaller of the two: what a lithography must resolve.
    pub fn smallest_um(&self) -> f64 {
        self.gap_um.map_or(self.line_um, |g| g.min(self.line_um))
    }
}

fn tolerance() -> Tolerance {
    Tolerance::unsnapped(Length::um(STEP / 10.0)).unwrap_or_default()
}

/// The region's pieces and holes.
fn topology(p: &Polygons) -> (usize, usize) {
    (
        p.contours.len(),
        p.contours.iter().map(|c| c.holes.len()).sum(),
    )
}

/// The least of `lo..hi` at which `changed` holds, by bisection to `STEP` (`changed`
/// false at `lo`, true at `hi`).
fn bisect(mut lo: f64, mut hi: f64, changed: impl Fn(f64) -> Result<bool>) -> Result<f64> {
    while hi - lo > STEP {
        let mid = 0.5 * (lo + hi);
        if changed(mid)? {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(0.5 * (lo + hi))
}

/// The narrowest line and gap of `shapes` joined.
///
/// # Errors
///
/// [`Error::InvalidValue`] with no shapes.
pub fn features(shapes: &[Shape]) -> Result<Features> {
    let t = tolerance();
    let mut joined: Option<Polygons> = None;
    for s in shapes {
        let p = s.polygons(t);
        joined = Some(match joined {
            None => p.boolean(&p, Operation::Union, t),
            Some(j) => j.boolean(&p, Operation::Union, t),
        });
    }
    let joined = joined.ok_or_else(|| Error::invalid("fabrication", "no shapes to measure"))?;
    let start = topology(&joined);
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for c in &joined.contours {
        for p in &c.outer {
            x0 = x0.min(p.x.to_um());
            x1 = x1.max(p.x.to_um());
            y0 = y0.min(p.y.to_um());
            y1 = y1.max(p.y.to_um());
        }
    }
    let span = (x1 - x0).max(y1 - y0);
    let offset = |d: f64| joined.offset(Length::um(d), Join::Square, t);
    let shrunk = |d: f64| -> Result<bool> { Ok(topology(&offset(-d)?) != start) };
    let half_line = bisect(0.0, span, shrunk)?;
    let grown = |d: f64| -> Result<bool> { Ok(topology(&offset(d)?) != start) };
    let gap = if grown(span)? {
        Some(rounded(2.0 * bisect(0.0, span, grown)?))
    } else {
        None
    };
    Ok(Features {
        line_um: rounded(2.0 * half_line),
        gap_um: gap,
    })
}

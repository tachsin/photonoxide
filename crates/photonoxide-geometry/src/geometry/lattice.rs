//! Lattices: the sites an [`super::Array`] places its elements on, each with its index (i, j),
//! and the defects a photonic crystal's cavities and guides are made of (sites removed,
//! shifted or resized).
//!
//! - **Rectangular:** site (i, j) at (i aₓ, j a_y).
//! - **Hexagonal** (triangular, as a photonic crystal's holes): rows along x, row j at
//!   y = j a √3/2, its sites at x = (i + ½ (j mod 2)) a. Every site has six nearest neighbours
//!   a apart; a patch of i₀ ≤ i ≤ i₁, j₀ ≤ j ≤ j₁ is a rectangle of rows, the odd rows half a
//!   pitch to the right, as photonic-crystal slabs are drawn (Γ–K along x).
//! - **Polar:** rings j = 0, 1, … of radius r₀ + j Δr, ring j holding n_j sites at the angles
//!   θ₀ + 2π i / n_j. A ring of radius zero holds one site, the centre.
//!
//! Positions are in the lattice's own frame, site (0, 0) at the origin; an array is placed
//! with a [`super::Transform`].

use std::f64::consts::TAU;

use super::Point;
use super::transform::sin_cos;
use crate::units::Length;
use crate::{Error, Result};

/// Which lattice, and its spacings.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LatticeKind {
    /// Site (i, j) at (i aₓ, j a_y).
    Rectangular {
        /// The pitches along x and y.
        pitch: [Length; 2],
        /// The range of i, both ends included.
        i: [i64; 2],
        /// The range of j, both ends included.
        j: [i64; 2],
    },
    /// Rows along x, a apart within a row and a √3/2 between rows, the odd rows shifted by a/2.
    Hexagonal {
        /// The distance between nearest neighbours.
        pitch: Length,
        /// The range of i along a row, both ends included.
        i: [i64; 2],
        /// The range of rows j, both ends included.
        j: [i64; 2],
    },
    /// Rings about the origin.
    Polar {
        /// Ring 0's radius.
        inner: Length,
        /// The rings' spacing.
        pitch: Length,
        /// How many sites ring j holds, j from 0.
        counts: Vec<usize>,
        /// The angle of each ring's site 0, radians.
        start: f64,
    },
}

/// A site of a lattice: its index and where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Site {
    /// (i, j): along a row and the row, or around a ring and the ring.
    pub index: [i64; 2],
    /// Its position in the lattice's frame, a shift included.
    pub at: Point,
    /// How much its element is scaled about it: 1 but where a defect resizes it.
    pub scale: f64,
}

/// The sites of a rectangular, hexagonal or polar lattice, with its defects. Build one with
/// [`Lattice::rectangular`], [`Lattice::hexagonal`] or [`Lattice::polar`], then remove, shift
/// or resize sites by their index.
#[derive(Clone, Debug, PartialEq)]
pub struct Lattice {
    kind: LatticeKind,
    sites: Vec<Site>,
    removed: Vec<[i64; 2]>,
}

/// A lattice can't have more sites than this (an array of 10⁷ elements is far past any
/// structure drawn element by element).
pub const MAX_SITES: usize = 10_000_000;

fn positive(what: &str, l: Length) -> Result<f64> {
    let v = l.to_um();
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(Error::invalid(
            "lattice",
            format!("needs a positive, finite {what}, got {l}"),
        ))
    }
}

fn range(name: &str, r: [i64; 2]) -> Result<usize> {
    if r[0] > r[1] {
        return Err(Error::invalid(
            "lattice",
            format!(
                "{name} runs from {} to {}: the first can't be above the last",
                r[0], r[1]
            ),
        ));
    }
    usize::try_from(r[1].abs_diff(r[0]))
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| Error::invalid("lattice", format!("{name}'s range is too long")))
}

fn too_many(n: usize) -> Error {
    Error::invalid("lattice", format!("has {n} sites, more than {MAX_SITES}"))
}

impl Lattice {
    /// A rectangular lattice: site (i, j) at (i aₓ, j a_y), for i from `i[0]` to `i[1]` and j
    /// from `j[0]` to `j[1]`, row by row (j, then i).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the pitches are positive and finite, each range runs
    /// upwards, and there are at most [`MAX_SITES`] sites.
    pub fn rectangular(pitch: [Length; 2], i: [i64; 2], j: [i64; 2]) -> Result<Lattice> {
        let (ax, ay) = (positive("pitch", pitch[0])?, positive("pitch", pitch[1])?);
        let count = range("i", i)?
            .checked_mul(range("j", j)?)
            .filter(|&n| n <= MAX_SITES)
            .ok_or_else(|| too_many(range("i", i).unwrap_or(0) * range("j", j).unwrap_or(0)))?;
        let mut sites = Vec::with_capacity(count);
        for b in j[0]..=j[1] {
            for a in i[0]..=i[1] {
                sites.push(Site {
                    index: [a, b],
                    at: Point::um(a as f64 * ax, b as f64 * ay),
                    scale: 1.0,
                });
            }
        }
        Ok(Lattice {
            kind: LatticeKind::Rectangular { pitch, i, j },
            sites,
            removed: Vec::new(),
        })
    }

    /// A hexagonal (triangular) lattice of nearest-neighbour distance `pitch`: rows along x,
    /// row j at y = j a √3/2, its sites at x = (i + ½ (j mod 2)) a, for i from `i[0]` to `i[1]`
    /// and j from `j[0]` to `j[1]`, row by row.
    ///
    /// # Errors
    ///
    /// As [`Lattice::rectangular`].
    pub fn hexagonal(pitch: Length, i: [i64; 2], j: [i64; 2]) -> Result<Lattice> {
        let a = positive("pitch", pitch)?;
        let count = range("i", i)?
            .checked_mul(range("j", j)?)
            .filter(|&n| n <= MAX_SITES)
            .ok_or_else(|| too_many(range("i", i).unwrap_or(0) * range("j", j).unwrap_or(0)))?;
        let row = a * 3f64.sqrt() / 2.0;
        let mut sites = Vec::with_capacity(count);
        for b in j[0]..=j[1] {
            let odd = 0.5 * b.rem_euclid(2) as f64;
            for c in i[0]..=i[1] {
                sites.push(Site {
                    index: [c, b],
                    at: Point::um((c as f64 + odd) * a, b as f64 * row),
                    scale: 1.0,
                });
            }
        }
        Ok(Lattice {
            kind: LatticeKind::Hexagonal { pitch, i, j },
            sites,
            removed: Vec::new(),
        })
    }

    /// A polar lattice: ring j (from 0) of radius `inner` + j `pitch` holding `counts[j]`
    /// sites, site i at the angle `start` + 2π i / `counts[j]`, ring by ring.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `inner` is at least zero and finite, `pitch` positive and
    /// finite, `start` finite, there is a ring, every ring holds a site, a ring of radius zero
    /// only one, and there are at most [`MAX_SITES`] sites.
    pub fn polar(inner: Length, pitch: Length, counts: &[usize], start: f64) -> Result<Lattice> {
        let r0 = inner.to_um();
        if !(r0.is_finite() && r0 >= 0.0) || !start.is_finite() {
            return Err(Error::invalid(
                "lattice",
                format!(
                    "a polar lattice needs a finite inner radius of at least zero and a finite \
                     start angle, got {inner} and {start} rad"
                ),
            ));
        }
        let dr = positive("pitch", pitch)?;
        if counts.is_empty() {
            return Err(Error::invalid("lattice", "a polar lattice needs a ring"));
        }
        let total = counts.iter().try_fold(0usize, |s, &n| s.checked_add(n));
        if total.is_none_or(|n| n > MAX_SITES) {
            return Err(too_many(total.unwrap_or(usize::MAX)));
        }
        let mut sites = Vec::with_capacity(total.unwrap_or(0));
        for (j, &n) in counts.iter().enumerate() {
            let r = r0 + j as f64 * dr;
            if n == 0 || (r == 0.0 && n != 1) {
                return Err(Error::invalid(
                    "lattice",
                    format!(
                        "ring {j} (radius {r} um) holds {n} sites: a ring holds at least one, \
                         and a ring of radius zero only one"
                    ),
                ));
            }
            for i in 0..n {
                let (s, c) = sin_cos(start + TAU * i as f64 / n as f64);
                sites.push(Site {
                    index: [i as i64, j as i64],
                    at: Point::um(r * c, r * s),
                    scale: 1.0,
                });
            }
        }
        Ok(Lattice {
            kind: LatticeKind::Polar {
                inner,
                pitch,
                counts: counts.to_vec(),
                start,
            },
            sites,
            removed: Vec::new(),
        })
    }

    /// Which lattice it is.
    pub fn kind(&self) -> &LatticeKind {
        &self.kind
    }

    /// The sites left, in order: row by row, or ring by ring.
    pub fn sites(&self) -> &[Site] {
        &self.sites
    }

    /// How many sites are left.
    pub fn len(&self) -> usize {
        self.sites.len()
    }

    /// Whether every site was removed.
    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// The site of index (i, j), unless it was removed or is outside the lattice.
    pub fn site(&self, index: [i64; 2]) -> Option<&Site> {
        self.position(index).map(|k| &self.sites[k])
    }

    /// The sites removed, in the order they were.
    pub fn removed(&self) -> &[[i64; 2]] {
        &self.removed
    }

    /// Where site `index` is in `sites`: found directly from its index, the sites being in
    /// order, while none is removed; by search after.
    fn position(&self, index: [i64; 2]) -> Option<usize> {
        let [a, b] = index;
        let direct = match &self.kind {
            LatticeKind::Rectangular { i, j, .. } | LatticeKind::Hexagonal { i, j, .. } => {
                if a < i[0] || a > i[1] || b < j[0] || b > j[1] {
                    return None;
                }
                let row = usize::try_from(i[1] - i[0] + 1).ok()?;
                usize::try_from(b - j[0]).ok()? * row + usize::try_from(a - i[0]).ok()?
            }
            LatticeKind::Polar { counts, .. } => {
                let ring = usize::try_from(b).ok()?;
                let n = *counts.get(ring)?;
                let i = usize::try_from(a).ok().filter(|&i| i < n)?;
                counts[..ring].iter().sum::<usize>() + i
            }
        };
        if self.sites.get(direct).is_some_and(|s| s.index == index) {
            return Some(direct);
        }
        self.sites.iter().position(|s| s.index == index)
    }

    /// The site `index`, for a defect named `what`: an error naming it when it isn't there.
    fn found(&self, index: [i64; 2], what: &str) -> Result<usize> {
        if let Some(k) = self.position(index) {
            return Ok(k);
        }
        let [a, b] = index;
        let why = if self.removed.contains(&index) {
            "it was removed".to_owned()
        } else {
            match &self.kind {
                LatticeKind::Rectangular { i, j, .. } | LatticeKind::Hexagonal { i, j, .. } => {
                    format!(
                        "i runs from {} to {} and j from {} to {}",
                        i[0], i[1], j[0], j[1]
                    )
                }
                LatticeKind::Polar { counts, .. } => format!(
                    "j is a ring from 0 to {}, and i from 0 to one less than its count ({})",
                    counts.len() - 1,
                    counts
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        };
        Err(Error::invalid(
            "lattice",
            format!("can't {what} site ({a}, {b}): {why}"),
        ))
    }

    /// The lattice without the site `index`: a vacancy (an L3 cavity's three, a W1 guide's
    /// row).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] naming the site when there is no such site, or it was removed.
    pub fn remove(mut self, index: [i64; 2]) -> Result<Lattice> {
        let k = self.found(index, "remove")?;
        self.sites.remove(k);
        self.removed.push(index);
        Ok(self)
    }

    /// The lattice with the site `index` moved by `by`, in the lattice's frame (an L3 cavity's
    /// end holes moved out).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] naming the site when there is no such site, or the shift isn't
    /// finite.
    pub fn shift(mut self, index: [i64; 2], by: [Length; 2]) -> Result<Lattice> {
        let k = self.found(index, "shift")?;
        let (dx, dy) = (by[0].to_um(), by[1].to_um());
        if !(dx.is_finite() && dy.is_finite()) {
            return Err(Error::invalid(
                "lattice",
                format!(
                    "can't shift site ({}, {}) by ({}, {}): not finite",
                    index[0], index[1], by[0], by[1]
                ),
            ));
        }
        let s = &mut self.sites[k];
        s.at = Point::um(s.at.x.to_um() + dx, s.at.y.to_um() + dy);
        Ok(self)
    }

    /// The lattice with the element at site `index` scaled by `factor` about the site (a
    /// smaller or larger hole).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] naming the site when there is no such site, or the factor isn't
    /// positive and finite.
    pub fn resize(mut self, index: [i64; 2], factor: f64) -> Result<Lattice> {
        let k = self.found(index, "resize")?;
        if !(factor.is_finite() && factor > 0.0) {
            return Err(Error::invalid(
                "lattice",
                format!(
                    "can't resize site ({}, {}) by {factor}: the factor must be positive and \
                     finite",
                    index[0], index[1]
                ),
            ));
        }
        self.sites[k].scale *= factor;
        Ok(self)
    }
}

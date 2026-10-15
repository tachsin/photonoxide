//! Averaging: the permittivity a solver's cell sees, from the share of it each material fills.
//!
//! A grid's cell that an interface crosses holds two materials (or more). Sampling it at a point
//! puts the interface where the samples fall, so a shape moves in steps of the grid
//! (staircasing) and a solver's error falls only as the step does, first order, and erratically.
//! [`Averaging`] says how a cell is read instead:
//!
//! - [`Averaging::Samples`], the solvers' default: 8 × 8 (in 3D 8 × 8 × 8) samples over the
//!   component's cell, harmonic along the component and arithmetic across it;
//! - [`Averaging::Point`]: the material at the component's own point, plain staircasing;
//! - [`Averaging::Mean`]: from the fill fractions ([`crate::geometry::Region::fill`], exact for
//!   polygons and to a stated tolerance for curves), the mean ⟨ε⟩ = Σ f ε for every component;
//! - [`Averaging::Diagonal`]: from the fill fractions and the interface's normal n
//!   ([`crate::geometry::Region::distance`]), the diagonal of A. Farjadpour et al.'s tensor
//!   (Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972, Eq. 1),
//!   ε̃⁻¹ = P ⟨ε⁻¹⟩ + (1 − P) ⟨ε⟩⁻¹, P = n nᵀ: the mean for a component along the interface, the
//!   harmonic mean for one across it (their "VP-EP" scheme);
//! - [`Averaging::Tensor`]: the whole tensor, its off-diagonal part too, which only the 2D
//!   problem with H along z couples ([`crate::fdfd::Solver2d::from_plane`]).
//!
//! Farjadpour et al. show why: a smoothing perturbs the interface's fields by Δε |E∥|² −
//! Δ(ε⁻¹) |D⊥|², and only their tensor makes that vanish to first order on an interface of any
//! slope, so only it keeps the scheme's second order there; the mean is right for E along every
//! interface (E along z in 2D), and the diagonal for interfaces along the grid's axes.
//! docs/methods/averaging.md has the method and what was measured.
//!
//! A [`Plane`] is what a 2D solver reads: shapes drawn on a background, each with its
//! permittivity, later ones on top. [`Shares`] is what fills one cell of it.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;

use crate::geometry::{Bounds, Index, Point, Region, Shape};
use crate::{Error, Result};

/// How a solver's cell is given its permittivity: by samples (the default) or from the fill
/// fractions of what is drawn. See the module's docs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Averaging {
    /// 8 samples along each axis of the component's cell, harmonic along the component and
    /// arithmetic across it: what the solvers have always done.
    #[default]
    Samples,
    /// The material at the component's own point: staircasing.
    Point,
    /// The fill-weighted mean ⟨ε⟩ for every component.
    Mean,
    /// The diagonal of Farjadpour et al.'s tensor, from the fill fractions and the interface's
    /// normal.
    Diagonal,
    /// Farjadpour et al.'s whole tensor, its off-diagonal part coupled where the problem has one
    /// (2D, H along z).
    Tensor,
}

impl Averaging {
    /// Every kind, in the order of the docs.
    pub const ALL: [Averaging; 5] = [
        Averaging::Samples,
        Averaging::Point,
        Averaging::Mean,
        Averaging::Diagonal,
        Averaging::Tensor,
    ];

    /// Its name in job files: `"samples"`, `"point"`, `"mean"`, `"diagonal"` or `"tensor"`.
    pub fn name(self) -> &'static str {
        match self {
            Averaging::Samples => "samples",
            Averaging::Point => "point",
            Averaging::Mean => "mean",
            Averaging::Diagonal => "diagonal",
            Averaging::Tensor => "tensor",
        }
    }

    /// The kind named `name` ([`Averaging::name`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for any other name.
    pub fn parse(name: &str) -> Result<Averaging> {
        Averaging::ALL
            .into_iter()
            .find(|a| a.name() == name)
            .ok_or_else(|| {
                Error::invalid(
                    "averaging",
                    format!(
                        "\"{name}\" isn't one of \"samples\", \"point\", \"mean\", \"diagonal\" or \
                         \"tensor\""
                    ),
                )
            })
    }

    /// Whether it reads the fill fractions (mean, diagonal, tensor) rather than points.
    pub fn reads_fill(self) -> bool {
        matches!(
            self,
            Averaging::Mean | Averaging::Diagonal | Averaging::Tensor
        )
    }
}

/// A share of a cell this close to 0 or 1 is taken as 0 or 1, so that a cell inside a curved
/// shape, whose fill comes from clipping its boundary, is that shape's material to the bit.
pub const SHARE_SNAP: f64 = 1e-12;

/// What fills a cell: each material (`K`) and the share of the cell it fills, adding up to 1,
/// and the unit normal of the interface that crosses it, if one does.
#[derive(Clone, Debug, PartialEq)]
pub struct Shares<K> {
    /// The materials and their shares, none of them 0.
    pub parts: Vec<(K, f64)>,
    /// The normal of the interface nearest the cell's centre, when the cell holds more than one
    /// material.
    pub normal: Option<[f64; 2]>,
}

impl<K: Copy> Shares<K> {
    /// A cell all of `k`.
    pub fn uniform(k: K) -> Shares<K> {
        Shares {
            parts: vec![(k, 1.0)],
            normal: None,
        }
    }

    /// Whether one material fills the whole cell.
    pub fn is_uniform(&self) -> bool {
        self.parts.len() == 1
    }

    /// The same shares with each material mapped by `f`.
    pub fn map<L>(&self, f: impl Fn(K) -> L) -> Shares<L> {
        Shares {
            parts: self.parts.iter().map(|&(k, s)| (f(k), s)).collect(),
            normal: self.normal,
        }
    }
}

impl Shares<c64> {
    /// ⟨ε⟩ = Σ f ε; the one material's ε, to the bit, in a uniform cell.
    pub fn mean(&self) -> c64 {
        if let [(e, _)] = self.parts[..] {
            return e;
        }
        self.parts.iter().map(|&(e, f)| e * f).sum()
    }

    /// ⟨ε⁻¹⟩⁻¹ = (Σ f/ε)⁻¹, the harmonic mean; the one material's ε, to the bit, in a uniform
    /// cell.
    pub fn harmonic(&self) -> c64 {
        if let [(e, _)] = self.parts[..] {
            return e;
        }
        1.0 / self.parts.iter().map(|&(e, f)| f / e).sum::<c64>()
    }

    /// The inverse tensor ε̃⁻¹ = P ⟨ε⁻¹⟩ + (1 − P) ⟨ε⟩⁻¹ in the plane, P = n nᵀ (Farjadpour et
    /// al., Eq. 1), as [xx, yy, xy]. Without an interface, ε⁻¹ on the diagonal.
    pub fn inverse_tensor(&self) -> [c64; 3] {
        let (a, b) = (1.0 / self.harmonic(), 1.0 / self.mean());
        match self.normal {
            Some([nx, ny]) if !self.is_uniform() => [
                a * nx * nx + b * (1.0 - nx * nx),
                a * ny * ny + b * (1.0 - ny * ny),
                (a - b) * nx * ny,
            ],
            _ => [b, b, c64::new(0.0, 0.0)],
        }
    }

    /// The permittivity a field component along `along` (0: x, 1: y, 2: z, out of the plane)
    /// sees by `averaging`, which must read the fill: the mean, or for the diagonal and the
    /// tensor 1 / (ε̃⁻¹)_aa; along z, which every interface of a plane is parallel to, the mean.
    pub fn component(&self, along: usize, averaging: Averaging) -> c64 {
        if self.is_uniform() || averaging == Averaging::Mean || along >= 2 {
            return self.mean();
        }
        let t = self.inverse_tensor();
        1.0 / t[along]
    }
}

/// Shapes drawn on a background, each with its material `K`, later ones on top: a plane as the
/// averaging reads it. Cells are answered through an [`Index`] when there are many shapes.
pub(crate) struct Layout<K> {
    background: K,
    shapes: Vec<Shape>,
    keys: Vec<K>,
    index: OnceLock<Index>,
    /// Unions of sets of shapes (`false`) and the part of a shape that the ones after it leave
    /// visible (`true`), made once each; `None` where the boolean is empty or couldn't be made.
    combined: Mutex<Combined>,
}

/// The booleans a [`Layout`] made, by whether each is a visible part and its shapes.
type Combined = BTreeMap<(bool, Vec<usize>), Option<Shape>>;

/// With more shapes than this, cells and points are answered through the index.
const INDEXED: usize = 8;

impl<K: Copy + PartialEq> Layout<K> {
    pub(crate) fn new(background: K) -> Layout<K> {
        Layout {
            background,
            shapes: Vec::new(),
            keys: Vec::new(),
            index: OnceLock::new(),
            combined: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn push(&mut self, shape: Shape, key: K) {
        self.shapes.push(shape);
        self.keys.push(key);
        self.index = OnceLock::new();
        self.combined = Mutex::new(BTreeMap::new());
    }

    pub(crate) fn background(&self) -> K {
        self.background
    }

    pub(crate) fn shapes(&self) -> impl Iterator<Item = (&Shape, K)> {
        self.shapes.iter().zip(self.keys.iter().copied())
    }

    fn index(&self) -> &Index {
        self.index.get_or_init(|| Index::of(&self.shapes))
    }

    /// The shapes whose boxes meet `cell`, in drawing order.
    fn near(&self, cell: Bounds) -> Vec<usize> {
        if self.shapes.len() <= INDEXED {
            return (0..self.shapes.len())
                .filter(|&k| self.shapes[k].bounds().overlaps(&cell))
                .collect();
        }
        let mut found = self.index().meeting(cell);
        found.sort_unstable();
        found
    }

    /// The material at `p`: the last shape drawn that holds it, or the background.
    pub(crate) fn at(&self, p: Point) -> K {
        let cell = Bounds { min: p, max: p };
        self.near(cell)
            .into_iter()
            .rev()
            .find(|&k| self.shapes[k].contains(p))
            .map_or(self.background, |k| self.keys[k])
    }

    /// What fills `cell`: each material's share, from the shapes' fills, and the normal of the
    /// boundary nearest its centre.
    ///
    /// One shape crossing the cell gives its fill exactly. Where several do, their union's fill
    /// (all of one material) or each one's visible part, what the shapes drawn after it leave
    /// (a [`crate::geometry::Boolean`], whose fill comes from its polygons within
    /// [`crate::geometry::FILL_TOLERANCE`] of its size) is taken, made once per set of shapes.
    pub(crate) fn shares(&self, cell: Bounds) -> Shares<K> {
        let area = cell.area();
        if area.is_nan() || area <= 0.0 {
            let centre = centre_of(cell);
            return Shares::uniform(self.at(centre));
        }
        let touched: Vec<(usize, f64)> = self
            .near(cell)
            .into_iter()
            .map(|k| (k, self.shapes[k].fill(cell) / area))
            .filter(|&(_, f)| f > SHARE_SNAP)
            .collect();
        let snap = |f: f64| {
            if f >= 1.0 - SHARE_SNAP {
                1.0
            } else if f <= SHARE_SNAP {
                0.0
            } else {
                f
            }
        };
        let mut parts: Vec<(K, f64)> = Vec::with_capacity(3);
        let mut add = |k: K, f: f64| {
            let f = snap(f);
            if f == 0.0 {
                return;
            }
            match parts.iter_mut().find(|(key, _)| *key == k) {
                Some((_, s)) => *s += f,
                None => parts.push((k, f)),
            }
        };
        match touched[..] {
            [] => return Shares::uniform(self.background),
            [(k, f)] => {
                let f = snap(f);
                if f == 1.0 {
                    return Shares::uniform(self.keys[k]);
                }
                add(self.keys[k], f);
                add(self.background, 1.0 - f);
            }
            _ => {
                let set: Vec<usize> = touched.iter().map(|&(k, _)| k).collect();
                let one = self.keys[set[0]];
                if set.iter().all(|&k| self.keys[k] == one) {
                    let f = match self.combine(false, &set) {
                        Some(u) => u.fill(cell) / area,
                        None => return self.sampled(cell),
                    };
                    let f = snap(f.min(1.0));
                    add(one, f);
                    add(self.background, 1.0 - f);
                } else {
                    // each shape's visible part, the last drawn whole
                    let mut covered = 0.0;
                    for (n, &k) in set.iter().enumerate() {
                        let f = if n + 1 == set.len() {
                            touched[n].1
                        } else {
                            match self.combine(true, &set[n..]) {
                                Some(v) => v.fill(cell) / area,
                                None => 0.0,
                            }
                        };
                        covered += f;
                        add(self.keys[k], f);
                    }
                    add(self.background, (1.0 - covered).max(0.0));
                }
            }
        }
        // the shares add up to 1, as the snaps and the booleans' tolerance may not have
        let total: f64 = parts.iter().map(|&(_, f)| f).sum();
        if parts.len() == 1 {
            return Shares::uniform(parts[0].0);
        }
        for (_, f) in &mut parts {
            *f /= total;
        }
        let centre = centre_of(cell);
        let normal = touched
            .iter()
            .map(|&(k, _)| self.shapes[k].distance(centre))
            .min_by(|a, b| a.signed.to_um().abs().total_cmp(&b.signed.to_um().abs()))
            .map(|d| d.normal);
        Shares { parts, normal }
    }

    /// The union of the shapes `set` (`visible` false), or the first of them less the union of
    /// the rest (`visible` true), made once.
    fn combine(&self, visible: bool, set: &[usize]) -> Option<Shape> {
        let key = (visible, set.to_vec());
        if let Some(found) = self.lock().get(&key) {
            return found.clone();
        }
        let union = |ks: &[usize]| -> Option<Shape> {
            let mut acc = self.shapes[ks[0]].clone();
            for &k in &ks[1..] {
                acc = acc.union(&self.shapes[k]).ok()?;
            }
            Some(acc)
        };
        let made = if visible {
            union(&set[1..]).and_then(|rest| self.shapes[set[0]].difference(&rest).ok())
        } else {
            union(set)
        };
        self.lock().insert(key, made.clone());
        made
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Combined> {
        // a poisoned cache only lost an entry another thread was adding
        self.combined
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The shares from 32 × 32 samples, where the booleans couldn't be made.
    fn sampled(&self, cell: Bounds) -> Shares<K> {
        let n = 32;
        let (x0, y0) = (cell.min.x.to_um(), cell.min.y.to_um());
        let (w, h) = (cell.max.x.to_um() - x0, cell.max.y.to_um() - y0);
        let mut parts: Vec<(K, f64)> = Vec::new();
        for b in 0..n {
            for a in 0..n {
                let p = Point::um(
                    x0 + (a as f64 + 0.5) / n as f64 * w,
                    y0 + (b as f64 + 0.5) / n as f64 * h,
                );
                let k = self.at(p);
                let share = 1.0 / (n * n) as f64;
                match parts.iter_mut().find(|(key, _)| *key == k) {
                    Some((_, s)) => *s += share,
                    None => parts.push((k, share)),
                }
            }
        }
        if parts.len() == 1 {
            return Shares::uniform(parts[0].0);
        }
        let centre = centre_of(cell);
        let normal = self
            .near(cell)
            .into_iter()
            .map(|k| self.shapes[k].distance(centre))
            .min_by(|a, b| a.signed.to_um().abs().total_cmp(&b.signed.to_um().abs()))
            .map(|d| d.normal);
        Shares { parts, normal }
    }
}

fn centre_of(cell: Bounds) -> Point {
    Point::um(
        0.5 * (cell.min.x.to_um() + cell.max.x.to_um()),
        0.5 * (cell.min.y.to_um() + cell.max.y.to_um()),
    )
}

impl<K: Copy + PartialEq> Clone for Layout<K> {
    fn clone(&self) -> Layout<K> {
        Layout {
            background: self.background,
            shapes: self.shapes.clone(),
            keys: self.keys.clone(),
            index: OnceLock::new(),
            combined: Mutex::new(BTreeMap::new()),
        }
    }
}

/// A plane for a 2D solver: a background permittivity, and shapes drawn on it each with its own,
/// later ones on top. It answers a point ([`Plane::at`]) and a cell ([`Plane::shares`]).
///
/// ```
/// use num_complex::Complex64 as c64;
/// use photonoxide::averaging::Plane;
/// use photonoxide::geometry::{Bounds, Point, Shape};
/// use photonoxide::units::Length;
///
/// let mut plane = Plane::new(c64::new(1.0, 0.0));
/// let disk = Shape::circle(Point::um(0.0, 0.0), Length::um(1.0)).unwrap();
/// plane.draw(disk, c64::new(12.0, 0.0));
/// // a cell across the disk's edge
/// let cell = Bounds::new(Point::um(0.9, -0.05), Point::um(1.1, 0.05));
/// let shares = plane.shares(cell);
/// assert_eq!(shares.parts.len(), 2);
/// // the disk fills 0.4979 of it (its edge bows in), so the mean is 1 + 11 × 0.4979
/// assert!((shares.mean().re - 6.477).abs() < 1e-3);
/// ```
#[derive(Clone)]
pub struct Plane {
    layout: Layout<c64>,
}

impl Plane {
    /// A plane of relative permittivity `background`, nothing drawn on it.
    pub fn new(background: c64) -> Plane {
        Plane {
            layout: Layout::new(background),
        }
    }

    /// Draws `shape` with relative permittivity `eps`, over what was drawn before.
    pub fn draw(&mut self, shape: Shape, eps: c64) -> &mut Plane {
        self.layout.push(shape, eps);
        self
    }

    /// The background's permittivity.
    pub fn background(&self) -> c64 {
        self.layout.background()
    }

    /// The shapes drawn, in order, with their permittivities.
    pub fn shapes(&self) -> impl Iterator<Item = (&Shape, c64)> {
        self.layout.shapes()
    }

    /// The permittivity at (x, y), µm: the last shape drawn that holds it, or the background.
    pub fn at(&self, x: f64, y: f64) -> c64 {
        self.layout.at(Point::um(x, y))
    }

    /// What fills `cell`: one shape's fill exactly, several shapes' union or each one's visible
    /// part from a boolean, and the normal of the boundary nearest the centre (see the module's
    /// docs).
    pub fn shares(&self, cell: Bounds) -> Shares<c64> {
        self.layout.shares(cell)
    }

    /// The cell centred on (x, y) of `size` (µm), as [`Plane::shares`] has it.
    pub(crate) fn shares_at(&self, (x, y): (f64, f64), size: (f64, f64)) -> Shares<c64> {
        self.shares(Bounds::new(
            Point::um(x - size.0 / 2.0, y - size.1 / 2.0),
            Point::um(x + size.0 / 2.0, y + size.1 / 2.0),
        ))
    }
}

impl fmt::Debug for Plane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Plane")
            .field("background", &self.layout.background)
            .field("shapes", &self.layout.shapes.len())
            .finish()
    }
}

impl PartialEq for Plane {
    fn eq(&self, other: &Plane) -> bool {
        self.layout.background == other.layout.background
            && self.layout.shapes == other.layout.shapes
            && self.layout.keys == other.layout.keys
    }
}

pub(crate) mod checks;
#[cfg(test)]
mod tests;

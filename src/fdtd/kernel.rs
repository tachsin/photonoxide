//! The Yee updates' kernel, in f32 and f64: the curl updates of E and H̃ and the CPML's
//! convolutions in their slabs, row by row.
//!
//! Along a row of the grid (i, the fastest axis), the neighbours along y and z are a whole row
//! away, the same offset for every value, and those along x are the next value but at the row's
//! ends. Each row is then three plain loops over slices of equal length: the compiler
//! vectorizes them over i, and the ends, where a wall, a periodic side or a Bloch side decides
//! the neighbour, are done one value at a time. Every value is computed by the same operations
//! in the same order as the plain loops it replaces (kept as [`reference`]), so in f64 the
//! fields are the same bits; IEEE 754 makes a vector lane's sum or product the scalar one.
//! The z-planes are shared among rayon's threads, each value written by one of them: the same
//! bits on any number of threads.
//!
//! The neighbours' offsets and the 1/(κΔ) factors are tabulated once per field ([`Stencil`]),
//! and nothing is allocated while stepping.

use std::ops::{Add, Div, Mul, Sub};

use rayon::prelude::*;

use super::Field;
use crate::fdfd::{Axis, Grid3d};

/// A floating-point type the kernel runs in: f32 or f64.
pub trait Real:
    Copy
    + Send
    + Sync
    + PartialEq
    + std::fmt::Debug
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + 'static
{
    /// Zero.
    const ZERO: Self;
    /// `v`, rounded to this type.
    fn of(v: f64) -> Self;
    /// This value as an f64, exactly.
    fn to_f64(self) -> f64;
}

impl Real for f64 {
    const ZERO: f64 = 0.0;
    fn of(v: f64) -> f64 {
        v
    }
    fn to_f64(self) -> f64 {
        self
    }
}

impl Real for f32 {
    const ZERO: f32 = 0.0;
    fn of(v: f64) -> f32 {
        v as f32
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

/// One field's differences: for each axis, the offset to each index's neighbour (forward for
/// H̃'s update, which differences E, back for E's), `None` beyond a wall, and the factor κ/Δ
/// there.
#[derive(Clone, Debug)]
pub(crate) struct Stencil<T> {
    /// Forward differences (H̃'s update) or back (E's).
    forward: bool,
    offsets: [Vec<Option<isize>>; 3],
    factors: [Vec<T>; 3],
    /// A row of zeros: the field beyond a wall.
    zeros: Vec<T>,
}

impl<T: Real> Stencil<T> {
    /// This stencil in another precision.
    pub(crate) fn to<U: Real>(&self) -> Stencil<U> {
        Stencil {
            forward: self.forward,
            offsets: self.offsets.clone(),
            factors: std::array::from_fn(|a| {
                self.factors[a].iter().map(|f| U::of(f.to_f64())).collect()
            }),
            zeros: vec![U::ZERO; self.zeros.len()],
        }
    }

    /// From the neighbours' `offsets` along each axis and `factors`, κ/Δ per index.
    pub(crate) fn new(
        grid: Grid3d,
        forward: bool,
        offsets: [Vec<Option<isize>>; 3],
        factors: &[Vec<f64>; 3],
    ) -> Stencil<T> {
        Stencil {
            forward,
            offsets,
            factors: std::array::from_fn(|a| factors[a].iter().map(|&f| T::of(f)).collect()),
            zeros: vec![T::ZERO; grid.nx],
        }
    }
}

/// The difference along an axis other than x for one row: the neighbour row (zeros beyond a
/// wall) and the factor, the same along the row.
struct Across<'a, T> {
    other: &'a [T],
    factor: T,
}

/// The neighbour row of the row starting at `base` along `axis` (y or z), at index `m` on it.
fn across<'a, T: Real>(
    stencil: &'a Stencil<T>,
    f: &'a [T],
    axis: Axis,
    m: usize,
    base: usize,
    nx: usize,
) -> Across<'a, T> {
    let a = axis.index();
    let other = match stencil.offsets[a][m] {
        Some(o) => {
            let start = (base as isize + o) as usize;
            &f[start..start + nx]
        }
        None => &stencil.zeros[..nx],
    };
    Across {
        other,
        factor: stencil.factors[a][m],
    }
}

/// The difference of `f` (the row `row`, starting at `base` in `f`) along x at index `i`, the
/// neighbour from the stencil: the rows' ends.
#[inline(always)]
fn along_x_at<T: Real>(stencil: &Stencil<T>, f: &[T], row: &[T], base: usize, i: usize) -> T {
    let other =
        stencil.offsets[0][i].map_or(T::ZERO, |o| f[(base as isize + i as isize + o) as usize]);
    if stencil.forward {
        other - row[i]
    } else {
        row[i] - other
    }
}

/// How a row's values are updated from the curl: E ← keep E + scale curl, or H̃ ← H̃ − Δt curl.
#[derive(Clone, Copy)]
enum Apply<'a, T> {
    E { keep: &'a [T], scale: &'a [T] },
    H { dt: T },
}

impl<T: Real> Apply<'_, T> {
    #[inline(always)]
    fn at(&self, v: T, i: usize, curl: T) -> T {
        match *self {
            Apply::E { keep, scale } => keep[i] * v + scale[i] * curl,
            Apply::H { dt } => v - dt * curl,
        }
    }
}

/// E's update coefficients ca and cb, per component.
pub(crate) type Coefficients<'a, T> = (&'a [Vec<T>; 3], &'a [Vec<T>; 3]);

/// One field's update from the curl of the other, `f`, all three components in one pass over
/// the grid, row by row: (∇ × F)_c = ∂F_b/∂a − ∂F_a/∂b for each c, (a, b) = c.others(), each
/// difference times its factor κ/Δ; E ← ca E + cb (∇ × H̃) with `coefficients` (ca, cb) per
/// component, or H̃ ← H̃ − Δt (∇ × E) without. One pass reads each row of the three
/// components of each field once, where three would read them three times.
pub(crate) fn curl_update<T: Real>(
    grid: Grid3d,
    out: &mut [Vec<T>; 3],
    f: &[Vec<T>; 3],
    stencil: &Stencil<T>,
    coefficients: Option<Coefficients<'_, T>>,
    dt: T,
) {
    let (nx, ny) = (grid.nx, grid.ny);
    let plane = nx * ny;
    let [ox, oy, oz] = out;
    ox.par_chunks_mut(plane)
        .zip(oy.par_chunks_mut(plane))
        .zip(oz.par_chunks_mut(plane))
        .enumerate()
        .for_each(|(k, ((px, py), pz))| {
            for j in 0..ny {
                let base = k * plane + j * nx;
                let span = j * nx..(j + 1) * nx;
                let apply = |c: usize| match coefficients {
                    Some((keep, scale)) => Apply::E {
                        keep: &keep[c][base..base + nx],
                        scale: &scale[c][base..base + nx],
                    },
                    None => Apply::H { dt },
                };
                let row = |c: usize| &f[c][base..base + nx];
                // c = x: ∂F_z/∂y − ∂F_y/∂z, both across rows
                let ta = across(stencil, &f[2], Axis::Y, j, base, nx);
                let tb = across(stencil, &f[1], Axis::Z, k, base, nx);
                if stencil.forward {
                    rows_across::<T, true>(
                        &mut px[span.clone()],
                        row(2),
                        row(1),
                        &ta,
                        &tb,
                        apply(0),
                    );
                } else {
                    rows_across::<T, false>(
                        &mut px[span.clone()],
                        row(2),
                        row(1),
                        &ta,
                        &tb,
                        apply(0),
                    );
                }
                // c = y: ∂F_x/∂z − ∂F_z/∂x
                let ta = across(stencil, &f[0], Axis::Z, k, base, nx);
                row_mixed(
                    stencil,
                    &mut py[span.clone()],
                    &f[2],
                    row(2),
                    row(0),
                    &ta,
                    base,
                    apply(1),
                    false,
                );
                // c = z: ∂F_y/∂x − ∂F_x/∂y
                let tb = across(stencil, &f[0], Axis::Y, j, base, nx);
                row_mixed(
                    stencil,
                    &mut pz[span],
                    &f[1],
                    row(1),
                    row(0),
                    &tb,
                    base,
                    apply(2),
                    true,
                );
            }
        });
}

/// A row of E_x or H̃_x: both differences across rows.
#[inline(always)]
fn rows_across<T: Real, const FORWARD: bool>(
    row: &mut [T],
    rfb: &[T],
    rfa: &[T],
    ta: &Across<'_, T>,
    tb: &Across<'_, T>,
    apply: Apply<'_, T>,
) {
    let n = row.len();
    let (rfb, rfa, ob, oa) = (&rfb[..n], &rfa[..n], &ta.other[..n], &tb.other[..n]);
    let (fa_, fb_) = (ta.factor, tb.factor);
    let diff = |o: T, f: T| if FORWARD { o - f } else { f - o };
    match apply {
        Apply::E { keep, scale } => {
            let (keep, scale) = (&keep[..n], &scale[..n]);
            for i in 0..n {
                let curl = diff(ob[i], rfb[i]) * fa_ - diff(oa[i], rfa[i]) * fb_;
                row[i] = keep[i] * row[i] + scale[i] * curl;
            }
        }
        Apply::H { dt } => {
            for i in 0..n {
                let curl = diff(ob[i], rfb[i]) * fa_ - diff(oa[i], rfa[i]) * fb_;
                row[i] = row[i] - dt * curl;
            }
        }
    }
}

/// A row of E_y, H̃_y (one difference along x, of F_z, subtracted: `x_first` false) or of
/// E_z, H̃_z (the difference along x of F_y first, `x_first` true). `fx` is the field
/// differenced along x (whole, `rx` its row), `ry` the one differenced across rows by `t`.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn row_mixed<T: Real>(
    stencil: &Stencil<T>,
    row: &mut [T],
    fx: &[T],
    rx: &[T],
    ry: &[T],
    t: &Across<'_, T>,
    base: usize,
    apply: Apply<'_, T>,
    x_first: bool,
) {
    let n = row.len();
    let fac = &stencil.factors[0][..n];
    let (rx, ry, other) = (&rx[..n], &ry[..n], &t.other[..n]);
    let ft = t.factor;
    let forward = stencil.forward;
    // the curl from the x difference dx and the across-rows one dy, in the order of the plain
    // loops: (∂F_b/∂a) κ_a/Δ_a − (∂F_a/∂b) κ_b/Δ_b
    let curl = |dx: T, i: usize, dy: T| {
        if x_first {
            dx * fac[i] - dy * ft
        } else {
            dy * ft - dx * fac[i]
        }
    };
    let across = |i: usize| {
        if forward {
            other[i] - ry[i]
        } else {
            ry[i] - other[i]
        }
    };
    // the interior along x: the next value (forward) or the one before
    if n > 1 {
        if forward {
            for i in 0..n - 1 {
                let c = curl(rx[i + 1] - rx[i], i, across(i));
                row[i] = apply.at(row[i], i, c);
            }
        } else {
            for i in 1..n {
                let c = curl(rx[i] - rx[i - 1], i, across(i));
                row[i] = apply.at(row[i], i, c);
            }
        }
    }
    // the row's end, its neighbour from the stencil
    let i = if forward { n - 1 } else { 0 };
    let c = curl(along_x_at(stencil, fx, rx, base, i), i, across(i));
    row[i] = apply.at(row[i], i, c);
}

/// One CPML slab's auxiliary field for one component and one axis: ψ over the slab's cells.
#[derive(Clone, Debug)]
pub(crate) struct Slab<T> {
    pub(crate) field: Field,
    /// The component updated.
    pub(crate) component: Axis,
    /// The axis of the PML, and of the derivative ψ convolves.
    pub(crate) axis: Axis,
    /// The slab's first index along `axis`, and its thickness.
    pub(crate) from: usize,
    pub(crate) count: usize,
    /// The recursion's coefficients along `axis`, `count` of each.
    pub(crate) b: Vec<T>,
    pub(crate) a: Vec<T>,
    pub(crate) psi: Vec<T>,
}

impl<T: Real> Slab<T> {
    /// This slab in another precision.
    pub(crate) fn to<U: Real>(&self) -> Slab<U> {
        let convert = |v: &Vec<T>| v.iter().map(|x| U::of(x.to_f64())).collect();
        Slab {
            field: self.field,
            component: self.component,
            axis: self.axis,
            from: self.from,
            count: self.count,
            b: convert(&self.b),
            a: convert(&self.a),
            psi: convert(&self.psi),
        }
    }
}

/// The CPML's convolutions for `field`'s updates: each slab's ψ ← bψ + a ∂F/∂w (F the other
/// field's component differenced along w, the slab's axis), then ± ψ into the updated field,
/// E += cb ψ or H̃ −= Δt ψ, the sign that of the derivative in the curl.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_slabs<T: Real>(
    grid: Grid3d,
    slabs: &mut [Slab<T>],
    field: Field,
    target: &mut [Vec<T>; 3],
    source: &[Vec<T>; 3],
    cb: &[Vec<T>; 3],
    stencil: &Stencil<T>,
    dt: T,
) {
    let (nx, ny) = (grid.nx, grid.ny);
    let plane = nx * ny;
    for slab in slabs.iter_mut().filter(|s| s.field == field) {
        let c = slab.component;
        let w = slab.axis;
        let (a, _) = c.others();
        // (∇ × F)_c = ∂F_b/∂a − ∂F_a/∂b: the slab's axis is a or b
        let (differentiated, sign) = if w == a {
            (c.others().1, T::of(1.0))
        } else {
            (a, T::of(-1.0))
        };
        let h = T::of(grid.step(w));
        let f = &source[differentiated.index()];
        let mut from = [0usize; 3];
        let mut count = [nx, ny, grid.nz];
        from[w.index()] = slab.from;
        count[w.index()] = slab.count;
        let local_plane = count[0] * count[1];
        let (bs, as_) = (&slab.b, &slab.a);
        let start = slab.from;
        let forward = stencil.forward;
        let offsets = &stencil.offsets[w.index()];
        let cb = &cb[c.index()];
        // ψ's planes and the field's planes they update, together: each row's ψ, then the field
        target[c.index()]
            .par_chunks_mut(plane)
            .skip(from[2])
            .take(count[2])
            .zip(slab.psi.par_chunks_mut(local_plane))
            .enumerate()
            .for_each(|(lk, (values, psi))| {
                let k = from[2] + lk;
                let n = count[0];
                for lj in 0..count[1] {
                    let j = from[1] + lj;
                    let at = j * nx + from[0];
                    let row_base = k * plane + at;
                    let psi = &mut psi[lj * n..(lj + 1) * n];
                    let values = &mut values[at..at + n];
                    let rf = &f[row_base..row_base + n];
                    if w == Axis::X {
                        // the slab's coefficients vary along the row; the neighbour is the next
                        // value but at the grid's ends
                        let (bs, as_) = (&bs[..n], &as_[..n]);
                        for li in 0..n {
                            let i = from[0] + li;
                            let other = offsets[i].map_or(T::ZERO, |o| {
                                f[(row_base as isize + li as isize + o) as usize]
                            });
                            let d = if forward {
                                other - rf[li]
                            } else {
                                rf[li] - other
                            } / h;
                            psi[li] = bs[li] * psi[li] + as_[li] * d;
                        }
                    } else {
                        let mw = [0, j, k][w.index()];
                        let s = mw - start;
                        let (bw, aw) = (bs[s], as_[s]);
                        let other = match offsets[mw] {
                            Some(o) => {
                                let at = (row_base as isize + o) as usize;
                                &f[at..at + n]
                            }
                            None => &stencil.zeros[..n],
                        };
                        if forward {
                            for li in 0..n {
                                let d = (other[li] - rf[li]) / h;
                                psi[li] = bw * psi[li] + aw * d;
                            }
                        } else {
                            for li in 0..n {
                                let d = (rf[li] - other[li]) / h;
                                psi[li] = bw * psi[li] + aw * d;
                            }
                        }
                    }
                    match field {
                        Field::E => {
                            let cb = &cb[row_base..row_base + n];
                            for li in 0..n {
                                let u = sign * psi[li];
                                values[li] = values[li] + cb[li] * u;
                            }
                        }
                        Field::H => {
                            for li in 0..n {
                                let u = sign * psi[li];
                                values[li] = values[li] - dt * u;
                            }
                        }
                    }
                }
            });
    }
}

pub(crate) mod reference;
mod yee;
pub(crate) use yee::Yee;

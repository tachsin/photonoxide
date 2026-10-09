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
//! The rows are shared among rayon's threads in chunks fixed by the grid (so a 2D grid, one plane
//! thick, is shared too), each value written by one of them: the same bits on any number of
//! threads.
//!
//! The neighbours' offsets and the 1/(κΔ) factors are tabulated once per field ([`Stencil`]),
//! and nothing is allocated while stepping the whole grid.
//!
//! Beyond the caches the grid is stepped by tiles instead ([`blocked`]), one step or several at
//! a time, each row by the same [`curl_row`] and [`slab_row`]: the same bits.

use std::ops::{Add, Div, Mul, Sub};

use rayon::prelude::*;

use super::Field;
use crate::fdfd::{Axis, Grid3d};

/// About how many values of a component one task of the curl's update takes: whole rows, as
/// many as make this many values, so that a grid one plane thick is shared among the threads.
/// Each row's update is alone, so the fields are the same bits however they are shared.
const ROW_CELLS: usize = 4096;

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

/// The rows of the other field, F, that one row's update reads: F's three components on the
/// row, and the rows across it that the curl differences along y and z (zeros beyond a wall).
pub(crate) struct Rows<'a, T> {
    pub(crate) f: [&'a [T]; 3],
    /// F_z and F_x on the neighbouring row along y: j + 1 for H̃'s update, j − 1 for E's.
    pub(crate) zy: &'a [T],
    pub(crate) xy: &'a [T],
    /// F_y and F_x on the neighbouring row along z: k + 1 for H̃'s update, k − 1 for E's.
    pub(crate) yz: &'a [T],
    pub(crate) xz: &'a [T],
}

/// The neighbour row along `axis` (y or z) of the row starting at `base` in `f`, at index `m`
/// on that axis: zeros beyond a wall.
fn neighbour<'a, T: Real>(
    stencil: &'a Stencil<T>,
    f: &'a [T],
    axis: usize,
    m: usize,
    base: usize,
    nx: usize,
) -> &'a [T] {
    match stencil.offsets[axis][m] {
        Some(o) => {
            let start = (base as isize + o) as usize;
            &f[start..start + nx]
        }
        None => &stencil.zeros[..nx],
    }
}

/// The difference along an axis other than x for one row: the neighbour row (zeros beyond a
/// wall) and the factor, the same along the row.
struct Across<'a, T> {
    other: &'a [T],
    factor: T,
}

/// The difference of a row along x at index `i`, the neighbour from the stencil: the rows'
/// ends. The neighbour along x is on the same row, across a periodic or Bloch side too.
#[inline(always)]
fn along_x_at<T: Real>(stencil: &Stencil<T>, row: &[T], i: usize) -> T {
    let other = stencil.offsets[0][i].map_or(T::ZERO, |o| row[(i as isize + o) as usize]);
    if stencil.forward {
        other - row[i]
    } else {
        row[i] - other
    }
}

/// How a row's values are updated from the curl: E ← keep E + scale curl, or H̃ ← H̃ − Δt curl.
#[derive(Clone, Copy)]
pub(crate) enum Apply<'a, T> {
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

/// How each component's row starting at `base` is updated: by E's coefficients there, or by Δt.
pub(crate) fn applies<'a, T: Real>(
    coefficients: Option<Coefficients<'a, T>>,
    dt: T,
    base: usize,
    nx: usize,
) -> [Apply<'a, T>; 3] {
    std::array::from_fn(|c| match coefficients {
        Some((keep, scale)) => Apply::E {
            keep: &keep[c][base..base + nx],
            scale: &scale[c][base..base + nx],
        },
        None => Apply::H { dt },
    })
}

/// One row's update, (j, k), of a field's three components (`out`) from the curl of the other
/// field (`rows`): (∇ × F)_c = ∂F_b/∂a − ∂F_a/∂b for each c, (a, b) = c.others(), each
/// difference times its factor κ/Δ, then `apply`. The blocked and the unblocked paths both
/// update every row by this, so they compute every value by the same operations.
#[inline]
pub(crate) fn curl_row<T: Real>(
    stencil: &Stencil<T>,
    (j, k): (usize, usize),
    out: [&mut [T]; 3],
    rows: &Rows<'_, T>,
    apply: [Apply<'_, T>; 3],
) {
    let [px, py, pz] = out;
    let (fy, fz) = (stencil.factors[1][j], stencil.factors[2][k]);
    // c = x: ∂F_z/∂y − ∂F_y/∂z, both across rows
    let ta = Across {
        other: rows.zy,
        factor: fy,
    };
    let tb = Across {
        other: rows.yz,
        factor: fz,
    };
    if stencil.forward {
        rows_across::<T, true>(px, rows.f[2], rows.f[1], &ta, &tb, apply[0]);
    } else {
        rows_across::<T, false>(px, rows.f[2], rows.f[1], &ta, &tb, apply[0]);
    }
    // c = y: ∂F_x/∂z − ∂F_z/∂x
    let ta = Across {
        other: rows.xz,
        factor: fz,
    };
    row_mixed(stencil, py, rows.f[2], rows.f[0], &ta, apply[1], false);
    // c = z: ∂F_y/∂x − ∂F_x/∂y
    let tb = Across {
        other: rows.xy,
        factor: fy,
    };
    row_mixed(stencil, pz, rows.f[1], rows.f[0], &tb, apply[2], true);
}

/// One field's update from the curl of the other, `f`, all three components in one pass over
/// the grid, row by row ([`curl_row`]): E ← ca E + cb (∇ × H̃) with `coefficients` (ca, cb)
/// per component, or H̃ ← H̃ − Δt (∇ × E) without. One pass reads each row of the three
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
    // the rows in chunks of about ROW_CELLS values, fixed by the grid: a 2D grid, one plane
    // thick, is shared among the threads as a 3D one is
    let rows = ROW_CELLS.div_ceil(nx).max(1);
    let [ox, oy, oz] = out;
    ox.par_chunks_mut(rows * nx)
        .zip(oy.par_chunks_mut(rows * nx))
        .zip(oz.par_chunks_mut(rows * nx))
        .enumerate()
        .for_each(|(chunk, ((px, py), pz))| {
            for local in 0..px.len() / nx {
                let r = chunk * rows + local;
                let (j, k) = (r % ny, r / ny);
                let base = r * nx;
                let span = local * nx..(local + 1) * nx;
                let reads = Rows {
                    f: std::array::from_fn(|c| &f[c][base..base + nx]),
                    zy: neighbour(stencil, &f[2], 1, j, base, nx),
                    xy: neighbour(stencil, &f[0], 1, j, base, nx),
                    yz: neighbour(stencil, &f[1], 2, k, base, nx),
                    xz: neighbour(stencil, &f[0], 2, k, base, nx),
                };
                curl_row(
                    stencil,
                    (j, k),
                    [&mut px[span.clone()], &mut py[span.clone()], &mut pz[span]],
                    &reads,
                    applies(coefficients, dt, base, nx),
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
/// E_z, H̃_z (the difference along x of F_y first, `x_first` true). `rx` is the row of the
/// field differenced along x, `ry` that of the one differenced across rows by `t`.
#[inline(always)]
fn row_mixed<T: Real>(
    stencil: &Stencil<T>,
    row: &mut [T],
    rx: &[T],
    ry: &[T],
    t: &Across<'_, T>,
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
    let c = curl(along_x_at(stencil, rx, i), i, across(i));
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

/// What a slab's rows read besides ψ: which field and component it updates, the derivative it
/// convolves and its sign in the curl, and the recursion's coefficients.
#[derive(Clone, Copy)]
pub(crate) struct SlabRows<'a, T> {
    pub(crate) field: Field,
    pub(crate) component: usize,
    pub(crate) axis: Axis,
    pub(crate) from: usize,
    pub(crate) count: usize,
    /// The other field's component differenced along `axis`.
    pub(crate) differentiated: usize,
    /// +1 or −1: the derivative's sign in the curl.
    pub(crate) sign: T,
    /// The step along `axis`.
    pub(crate) h: T,
    pub(crate) b: &'a [T],
    pub(crate) a: &'a [T],
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

    /// What its rows read, and ψ to update.
    pub(crate) fn rows(&mut self, grid: Grid3d) -> (SlabRows<'_, T>, &mut [T]) {
        let c = self.component;
        let w = self.axis;
        let (a, b) = c.others();
        // (∇ × F)_c = ∂F_b/∂a − ∂F_a/∂b: the slab's axis is a or b
        let (differentiated, sign) = if w == a {
            (b, T::of(1.0))
        } else {
            (a, T::of(-1.0))
        };
        (
            SlabRows {
                field: self.field,
                component: c.index(),
                axis: w,
                from: self.from,
                count: self.count,
                differentiated: differentiated.index(),
                sign,
                h: T::of(grid.step(w)),
                b: &self.b,
                a: &self.a,
            },
            &mut self.psi,
        )
    }
}

/// One row of a CPML slab: over the row's values in the slab, ψ ← bψ + a ∂F/∂w, then
/// ± ψ into the updated field, E += cb ψ or H̃ −= Δt ψ. `rf` is F's whole row, `other` its
/// neighbouring row along w when w is y or z (zeros beyond a wall), `m` the row's index along
/// w; `values` and `cb` are the updated field's whole row and its cb, `psi` ψ on the row.
#[allow(clippy::too_many_arguments)]
#[inline]
pub(crate) fn slab_row<T: Real>(
    slab: &SlabRows<'_, T>,
    stencil: &Stencil<T>,
    psi: &mut [T],
    values: &mut [T],
    rf: &[T],
    other: &[T],
    m: usize,
    cb: &[T],
    dt: T,
) {
    let n = psi.len();
    let x0 = if slab.axis == Axis::X { slab.from } else { 0 };
    let h = slab.h;
    let forward = stencil.forward;
    if slab.axis == Axis::X {
        // the slab's coefficients vary along the row; the neighbour is the next value but at
        // the grid's ends
        let (bs, as_, rx) = (&slab.b[..n], &slab.a[..n], &rf[x0..x0 + n]);
        for li in 0..n {
            let i = x0 + li;
            let o = stencil.offsets[0][i].map_or(T::ZERO, |o| rf[(i as isize + o) as usize]);
            let d = if forward { o - rx[li] } else { rx[li] - o } / h;
            psi[li] = bs[li] * psi[li] + as_[li] * d;
        }
    } else {
        let s = m - slab.from;
        let (bw, aw) = (slab.b[s], slab.a[s]);
        let (rf, other) = (&rf[..n], &other[..n]);
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
    let values = &mut values[x0..x0 + n];
    let sign = slab.sign;
    match slab.field {
        Field::E => {
            let cb = &cb[x0..x0 + n];
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

/// The CPML's convolutions for `field`'s updates: each slab's ψ ← bψ + a ∂F/∂w (F the other
/// field's component differenced along w, the slab's axis), then ± ψ into the updated field,
/// E += cb ψ or H̃ −= Δt ψ, the sign that of the derivative in the curl ([`slab_row`]).
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
        let (rows, psi) = slab.rows(grid);
        let w = rows.axis;
        let mut from = [0usize; 3];
        let mut count = [nx, ny, grid.nz];
        from[w.index()] = rows.from;
        count[w.index()] = rows.count;
        let local_plane = count[0] * count[1];
        let f = &source[rows.differentiated];
        let cb = &cb[rows.component];
        // ψ's planes and the field's planes they update, together: each row's ψ, then the field
        target[rows.component]
            .par_chunks_mut(plane)
            .skip(from[2])
            .take(count[2])
            .zip(psi.par_chunks_mut(local_plane))
            .enumerate()
            .for_each(|(lk, (values, psi))| {
                let k = from[2] + lk;
                for lj in 0..count[1] {
                    let j = from[1] + lj;
                    let base = (k * ny + j) * nx;
                    let m = [0, j, k][w.index()];
                    let other = if w == Axis::X {
                        &stencil.zeros[..0]
                    } else {
                        neighbour(stencil, f, w.index(), m, base, nx)
                    };
                    slab_row(
                        &rows,
                        stencil,
                        &mut psi[lj * count[0]..(lj + 1) * count[0]],
                        &mut values[j * nx..(j + 1) * nx],
                        &f[base..base + nx],
                        other,
                        m,
                        &cb[base..base + nx],
                        dt,
                    );
                }
            });
    }
}

pub(crate) mod blocked;
pub(crate) use blocked::{Blocking, Injection, Tap, Tiling};
pub(crate) mod reference;
mod yee;
pub(crate) use yee::Yee;

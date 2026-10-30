//! The interfaces' S-matrices from the modes' overlaps, and the structure's by the star product
//! (the module's third and fourth points).

use faer::linalg::matmul::matmul;
use faer::{Accum, Mat, Par};
use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::modes::Mode;
use super::{Eme, Scattering, Section};
use crate::{Error, Result};

/// The 20-point Gauss–Legendre rule's positive nodes and their weights.
const GL20: [(f64, f64); 10] = [
    (0.076_526_521_133_497_33, 0.152_753_387_130_725_85),
    (0.227_785_851_141_645_08, 0.149_172_986_472_603_75),
    (0.373_706_088_715_419_56, 0.142_096_109_318_382_05),
    (0.510_867_001_950_827_1, 0.131_688_638_449_176_63),
    (0.636_053_680_726_515, 0.118_194_531_961_518_42),
    (0.746_331_906_460_150_8, 0.101_930_119_817_240_44),
    (0.839_116_971_822_218_8, 0.083_276_741_576_704_75),
    (0.912_234_428_251_326, 0.062_672_048_334_109_06),
    (0.963_971_927_277_913_8, 0.040_601_429_800_386_94),
    (0.993_128_599_185_094_9, 0.017_614_007_139_152_12),
];

/// The widest phase a 20-point panel integrates, ω h: the rule is exact for polynomials of
/// degree 39, and e^(iωt) on a panel ωh = 10 wide is within 1e-17 of its Taylor polynomial.
const PANEL_PHASE: f64 = 10.0;

/// A quadrature point across the window: where, its weight along the complex coordinate
/// (dx̃ = stretch dx), and the layer it is in in each of the two slices.
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) weight: c64,
    pub(crate) layer_a: usize,
    pub(crate) layer_b: usize,
}

/// The points that integrate products of a mode of `a`'s slice and one of `b`'s across the
/// window: 20-point Gauss–Legendre panels on each piece between the two slices' layer
/// boundaries, as many as the fastest of the two modes' oscillation there needs.
pub(crate) fn points(a: &[&Mode], b: &[&Mode]) -> Vec<Point> {
    let (la, lb) = (&a[0].layers, &b[0].layers);
    let mut edges: Vec<f64> = la
        .iter()
        .chain(lb.iter())
        .flat_map(|l| [l.x0, l.x0 + l.width])
        .collect();
    edges.sort_by(f64::total_cmp);
    edges.dedup_by(|x, y| (*x - *y).abs() <= 1e-12 * y.abs().max(1.0));
    let fastest =
        |modes: &[&Mode], j: usize| modes.iter().map(|m| m.wavenumber(j)).fold(0.0, f64::max);
    let find = |layers: &[super::modes::Layer], x: f64| {
        layers.iter().rposition(|l| l.x0 <= x).unwrap_or(0)
    };
    let mut out = Vec::new();
    for w in edges.windows(2) {
        let (lo, hi) = (w[0], w[1]);
        if hi - lo <= 1e-12 {
            continue;
        }
        let mid = 0.5 * (lo + hi);
        let (ja, jb) = (find(la, mid), find(lb, mid));
        let stretch = la[ja].stretch;
        let omega = (fastest(a, ja) + fastest(b, jb)) * stretch.norm();
        let panels = ((omega * (hi - lo) / PANEL_PHASE).ceil() as usize).max(1);
        let h = (hi - lo) / panels as f64;
        for p in 0..panels {
            let centre = lo + (p as f64 + 0.5) * h;
            for &(node, weight) in &GL20 {
                for x in [centre - 0.5 * h * node, centre + 0.5 * h * node] {
                    out.push(Point {
                        x,
                        weight: stretch * (0.5 * h * weight),
                        layer_a: ja,
                        layer_b: jb,
                    });
                }
            }
        }
    }
    out
}

/// O_ij = ⟨E_i^a, H_j^b⟩ = ∫ (E_i^a × H_j^b)·ẑ dx̃, across the window.
pub(crate) fn overlaps(a: &[Mode], b: &[Mode]) -> Mat<c64> {
    let ra: Vec<&Mode> = a.iter().collect();
    let rb: Vec<&Mode> = b.iter().collect();
    let pts = points(&ra, &rb);
    // e of a's modes times the weights, and h of b's, at the points
    let e: Vec<Vec<c64>> = a
        .par_iter()
        .map(|m| {
            pts.iter()
                .map(|q| q.weight * m.e_h(q.layer_a, q.x).0)
                .collect()
        })
        .collect();
    let h: Vec<Vec<c64>> = b
        .par_iter()
        .map(|m| pts.iter().map(|q| m.e_h(q.layer_b, q.x).1).collect())
        .collect();
    let left = Mat::from_fn(a.len(), pts.len(), |i, q| e[i][q]);
    let right = Mat::from_fn(pts.len(), b.len(), |q, j| h[j][q]);
    product(&left, &right)
}

/// a b, on this thread: the same bits on any number of threads.
fn product(a: &Mat<c64>, b: &Mat<c64>) -> Mat<c64> {
    let mut out = Mat::zeros(a.nrows(), b.ncols());
    matmul(
        out.as_mut(),
        Accum::Replace,
        a.as_ref(),
        b.as_ref(),
        c64::new(1.0, 0.0),
        Par::Seq,
    );
    out
}

/// a⁻¹ b, by Gaussian elimination with partial pivoting.
fn solve(a: &Mat<c64>, b: &Mat<c64>) -> Mat<c64> {
    let n = a.nrows();
    let mut m = a.clone();
    let mut x = b.clone();
    for c in 0..n {
        let pivot = (c..n)
            .max_by(|&i, &j| m[(i, c)].norm().total_cmp(&m[(j, c)].norm()))
            .unwrap_or(c);
        if pivot != c {
            for j in 0..n {
                let t = m[(c, j)];
                m[(c, j)] = m[(pivot, j)];
                m[(pivot, j)] = t;
            }
            for j in 0..x.ncols() {
                let t = x[(c, j)];
                x[(c, j)] = x[(pivot, j)];
                x[(pivot, j)] = t;
            }
        }
        let inv = 1.0 / m[(c, c)];
        for i in c + 1..n {
            let f = m[(i, c)] * inv;
            if f == c64::new(0.0, 0.0) {
                continue;
            }
            for j in c..n {
                let v = m[(c, j)];
                m[(i, j)] -= f * v;
            }
            for j in 0..x.ncols() {
                let v = x[(c, j)];
                x[(i, j)] -= f * v;
            }
        }
    }
    for c in (0..n).rev() {
        let inv = 1.0 / m[(c, c)];
        for j in 0..x.ncols() {
            let mut v = x[(c, j)];
            for i in c + 1..n {
                v -= m[(c, i)] * x[(i, j)];
            }
            x[(c, j)] = v * inv;
        }
    }
    x
}

fn identity(n: usize) -> Mat<c64> {
    Mat::from_fn(n, n, |i, j| {
        if i == j {
            c64::new(1.0, 0.0)
        } else {
            c64::new(0.0, 0.0)
        }
    })
}

fn scaled(a: &Mat<c64>, f: f64) -> Mat<c64> {
    Mat::from_fn(a.nrows(), a.ncols(), |i, j| f * a[(i, j)])
}

fn transpose(a: &Mat<c64>) -> Mat<c64> {
    Mat::from_fn(a.ncols(), a.nrows(), |i, j| a[(j, i)])
}

/// A two-port's S-matrix in blocks: 11 and 22 reflections, 21 from port 1 to port 2.
#[derive(Clone, Debug)]
pub(crate) struct Blocks {
    pub(crate) s11: Mat<c64>,
    pub(crate) s12: Mat<c64>,
    pub(crate) s21: Mat<c64>,
    pub(crate) s22: Mat<c64>,
}

/// The interface whose overlaps are `o` (O_ij = ⟨E_i^I, H_j^II⟩), as the ideal transformer of
/// the module's third point: R = (1 + OOᵀ)⁻¹(1 − OOᵀ), T = 2Oᵀ(1 + OOᵀ)⁻¹, and from section II,
/// R' = (1 + OᵀO)⁻¹(OᵀO − 1), T' = 2O(1 + OᵀO)⁻¹.
pub(crate) fn interface(o: &Mat<c64>) -> Blocks {
    let ot = transpose(o);
    let (n1, n2) = (o.nrows(), o.ncols());
    let m = product(o, &ot);
    let mt = product(&ot, o);
    let plus = |m: &Mat<c64>, n: usize| &identity(n) + m;
    let minus = |m: &Mat<c64>, n: usize| &identity(n) - m;
    let s11 = solve(&plus(&m, n1), &minus(&m, n1));
    let s22 = scaled(&solve(&plus(&mt, n2), &minus(&mt, n2)), -1.0);
    // (1 + OOᵀ) and (1 + OᵀO) are symmetric: T = 2 ((1 + OOᵀ)⁻¹ O)ᵀ, T' = 2 ((1 + OᵀO)⁻¹ Oᵀ)ᵀ
    let s21 = scaled(&transpose(&solve(&plus(&m, n1), o)), 2.0);
    let s12 = scaled(&transpose(&solve(&plus(&mt, n2), &ot)), 2.0);
    Blocks { s11, s12, s21, s22 }
}

/// Redheffer's star product a ⋆ b: `a` then `b` along z (Bienstman and Baets's Eq. 7).
pub(crate) fn star(a: &Blocks, b: &Blocks) -> Blocks {
    let n = a.s22.nrows();
    let f = &identity(n) - &product(&b.s11, &a.s22);
    let g = &identity(n) - &product(&a.s22, &b.s11);
    let x = solve(&f, &product(&b.s11, &a.s21));
    let y = solve(&f, &b.s12);
    let z = solve(&g, &a.s21);
    let v = solve(&g, &product(&a.s22, &b.s12));
    Blocks {
        s11: &a.s11 + &product(&a.s12, &x),
        s12: product(&a.s12, &y),
        s21: product(&b.s21, &z),
        s22: &b.s22 + &product(&b.s21, &v),
    }
}

/// `s` followed by a length of guide whose modes advance by `p` (diag(e^(iβL))).
fn then_propagate(s: &Blocks, p: &[c64]) -> Blocks {
    let n = p.len();
    Blocks {
        s11: s.s11.clone(),
        s12: Mat::from_fn(s.s12.nrows(), n, |i, j| s.s12[(i, j)] * p[j]),
        s21: Mat::from_fn(n, s.s21.ncols(), |i, j| p[i] * s.s21[(i, j)]),
        s22: Mat::from_fn(n, n, |i, j| p[i] * s.s22[(i, j)] * p[j]),
    }
}

fn rows(m: &Mat<c64>) -> Vec<Vec<c64>> {
    (0..m.nrows())
        .map(|i| (0..m.ncols()).map(|j| m[(i, j)]).collect())
        .collect()
}

/// The structure's S-matrix.
pub(super) fn structure(eme: &Eme, sections: &[Section]) -> Result<Scattering> {
    let Some(first) = sections.first() else {
        return Err(Error::invalid("eme", "needs at least one section"));
    };
    let width = first.slice.width().to_um();
    for s in sections {
        let w = s.slice.width().to_um();
        if (w - width).abs() > 1e-9 * width {
            return Err(Error::invalid(
                "eme",
                format!("every section's window is as wide: {w} um against {width} um"),
            ));
        }
        let l = s.length.to_um();
        if !(l.is_finite() && l >= 0.0) {
            return Err(Error::invalid(
                "eme",
                format!("a section's length must be at least 0, got {l} um"),
            ));
        }
    }
    // neighbours of the same slice are one section
    let mut merged: Vec<(&super::Slice, f64)> = Vec::new();
    for s in sections {
        match merged.last_mut() {
            Some((slice, l)) if **slice == s.slice => *l += s.length.to_um(),
            _ => merged.push((&s.slice, s.length.to_um())),
        }
    }
    let modes: Vec<Vec<Mode>> = merged
        .iter()
        .map(|(slice, _)| eme.modes(slice))
        .collect::<Result<_>>()?;
    let k = eme.wavelength.wavenumber();
    let advance = |m: &[Mode], l: f64| -> Vec<c64> {
        m.iter()
            .map(|m| (c64::new(0.0, k * l) * m.effective_index()).exp())
            .collect()
    };
    let interfaces: Vec<Blocks> = modes
        .par_windows(2)
        .map(|w| interface(&overlaps(&w[0], &w[1])))
        .collect();
    let n = eme.modes;
    let zero = Mat::<c64>::zeros(n, n);
    let p0 = advance(&modes[0], merged[0].1);
    let diag = Mat::from_fn(n, n, |i, j| if i == j { p0[i] } else { c64::new(0.0, 0.0) });
    let mut total = Blocks {
        s11: zero.clone(),
        s12: diag.clone(),
        s21: diag,
        s22: zero,
    };
    for (i, face) in interfaces.iter().enumerate() {
        total = star(&total, face);
        total = then_propagate(&total, &advance(&modes[i + 1], merged[i + 1].1));
    }
    let indices = |m: &[Mode]| m.iter().map(Mode::effective_index).collect();
    Ok(Scattering {
        s11: rows(&total.s11),
        s12: rows(&total.s12),
        s21: rows(&total.s21),
        s22: rows(&total.s22),
        first: indices(&modes[0]),
        last: indices(&modes[modes.len() - 1]),
    })
}

/// Bienstman and Baets's Eq. 3, both continuities tested with section I's modes: for comparison.
#[cfg(test)]
pub(crate) fn interface_eq3(o12: &Mat<c64>, o21: &Mat<c64>) -> (Mat<c64>, Mat<c64>) {
    let sum = o12 + &transpose(o21);
    let t = scaled(&solve(&sum, &identity(o12.nrows())), 2.0);
    let r = scaled(&product(&(&transpose(o21) - o12), &t), 0.5);
    (r, t)
}

//! Vector fitting: responses sampled in frequency as rational functions in pole–residue form,
//! with common poles.
//!
//! B. Gustavsen, A. Semlyen, "Rational approximation of frequency domain responses by vector
//! fitting", IEEE Trans. Power Deliv. 14, 1052 (1999),
//! [doi:10.1109/61.772353](https://doi.org/10.1109/61.772353). Each response is fitted as
//! (their Eq. 2)
//!
//! f(s) ≈ Σₙ cₙ/(s − aₙ) + d + s h,
//!
//! in two linear stages, both with known poles:
//!
//! 1. **Pole relocation** (their Stage 1, Eqs. 3–8 and Appendix A): with starting poles āₙ, an
//!    unknown σ(s) = Σₙ c̃ₙ/(s − āₙ) + 1 is fitted together with (σf)(s) =
//!    Σₙ cₙ/(s − āₙ) + d + s h, from the linear least squares (σf)(s) − σ(s) f(s) ≈ 0. The zeros
//!    of σ are the new poles: the eigenvalues of A − b c̃ᵀ (Appendix B, Eq. B.1). Several
//!    responses share σ, and so their poles (the vector formulation of their discussion's
//!    reply, Eqs. 15–21).
//! 2. **Residue identification** (Stage 2): with the new poles fixed, the residues, d and h of
//!    each response by linear least squares.
//!
//! Repeated with the new poles as starting poles, the poles converge (their Section 4.3). Poles
//! that come out unstable have their real parts' signs inverted, as the paper does (Section 2,
//! and the reply to Watson).
//!
//! **Real and complex models.** A real system's response satisfies f(s̄) = f(s)‾: its poles are
//! real or complex-conjugate pairs and its residues alike, and d and h are real. That is the
//! paper's model, [`Symmetry::Real`], with its real formulation of the least squares (A.5–A.8)
//! and of the zeros (B.2). An optical spectrum is sampled over a band far from zero frequency
//! and never at negative frequencies, and a model there needn't be a real system's:
//! [`Symmetry::Complex`] lets every pole, residue and d be any complex number, the same equations
//! (A.1–A.4, B.1) solved in complex arithmetic without the conjugate pairing.
//!
//! **The solve.** Stage 1's least squares holds every response's own unknowns (cₙ, d, h) and the
//! shared c̃ₙ. Each response's own unknowns are eliminated exactly by a QR factorization of its
//! block [A B b] = QR: what remains for c̃ is the lower-right block of R, and those blocks of every
//! response are stacked and solved together. The columns are scaled to unit norm before every
//! least squares.
//!
//! The Laplace variable here is the paper's s. Under photonoxide's e^(−iωt) a field goes as
//! e^(st) for s = −iω, so a stable pole has a negative real part in either convention;
//! [`super::model`] maps wavelengths to s.

use faer::Mat;
use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// Whether a model is a real system's (poles and residues in conjugate pairs) or any complex
/// function's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Symmetry {
    /// f(s̄) = f(s)‾: real or conjugate-pair poles and residues, real d and h. The paper's
    /// model, and a real system's, needed for a time-domain model.
    Real,
    /// Any complex poles, residues and d: for a band far from zero frequency, such as an
    /// optical carrier's.
    Complex,
}

/// How a response's known delay is taken out before it is fitted.
#[derive(Clone, Debug, PartialEq)]
pub enum Delay {
    /// No delay.
    None,
    /// The same delay τ for every response: f(s) = e^(−sτ) R(s), and R is fitted.
    Fixed(f64),
    /// Each response's own delay, estimated from the slope of its unwrapped phase
    /// ([`estimate_delay`]).
    Estimate,
}

/// The settings of a fit.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The number of poles N: for [`Symmetry::Real`] it must be even (pairs).
    pub poles: usize,
    /// At most this many pole relocations (Stage 1); the fit with the smallest error is kept.
    pub iterations: usize,
    /// Relocations stop once no pole moves by more than this, relative to its magnitude.
    pub tolerance: f64,
    /// A real system's model or a complex one.
    pub symmetry: Symmetry,
    /// Fit the constant term d.
    pub constant: bool,
    /// Fit the proportional term s h (a response that grows without bound at high frequency;
    /// never for an S-matrix).
    pub proportional: bool,
    /// The delay taken out first.
    pub delay: Delay,
    /// The starting poles; `None` spreads them over the band as the paper's Section 3.2 does
    /// (see [`starting_poles`]).
    pub starting_poles: Option<Vec<c64>>,
}

impl Options {
    /// `poles` poles, up to 20 relocations to a relative tolerance of 1e-10, a complex model with
    /// a constant term, no delay, and starting poles from the paper's recipe.
    pub fn new(poles: usize) -> Options {
        Options {
            poles,
            iterations: 20,
            tolerance: 1e-10,
            symmetry: Symmetry::Complex,
            constant: true,
            proportional: false,
            delay: Delay::None,
            starting_poles: None,
        }
    }
}

/// A fit's error against its samples, over every response and sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitError {
    /// The root mean square of |f − f_fit|.
    pub rms: f64,
    /// The largest |f − f_fit|.
    pub max: f64,
    /// `rms` over the root mean square of |f|.
    pub rms_relative: f64,
    /// `max` over the largest |f|: relative to the response's scale, so a notch where f is
    /// near zero doesn't make it infinite.
    pub max_relative: f64,
}

/// A fitted rational model: common poles, and each response's residues, constant, proportional
/// term and delay.
///
/// Response e is f_e(s) = e^(−s τ_e) (Σₙ r_en/(s − aₙ) + d_e + s h_e).
#[derive(Clone, Debug, PartialEq)]
pub struct Rational {
    /// The poles aₙ, each conjugate pair as both its poles (a real model's).
    pub poles: Vec<c64>,
    /// `residues[e][n]`: response e's residue at pole n.
    pub residues: Vec<Vec<c64>>,
    /// Each response's constant term d.
    pub constant: Vec<c64>,
    /// Each response's proportional term h.
    pub proportional: Vec<c64>,
    /// Each response's delay τ.
    pub delays: Vec<f64>,
    /// The error against the samples it was fitted to.
    pub error: FitError,
    /// The pole relocations made.
    pub iterations: usize,
}

impl Rational {
    /// Response `e` at `s`.
    ///
    /// # Panics
    ///
    /// If `e` isn't a response of the model.
    pub fn evaluate(&self, e: usize, s: c64) -> c64 {
        let sum: c64 = self
            .poles
            .iter()
            .zip(&self.residues[e])
            .map(|(&a, &r)| r / (s - a))
            .sum();
        (sum + self.constant[e] + s * self.proportional[e]) * (-s * self.delays[e]).exp()
    }

    /// The number of responses.
    pub fn responses(&self) -> usize {
        self.residues.len()
    }
}

/// A pole of the model being fitted, with its share of the least squares' real unknowns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Pole {
    /// A real pole: one real residue.
    Real(f64),
    /// A complex-conjugate pair a, ā (stored with Im a > 0): residues c, c̄ from two real
    /// unknowns c′, c″ (the paper's A.5–A.6).
    Pair(c64),
    /// A lone complex pole: a complex residue from two real unknowns.
    Complex(c64),
}

impl Pole {
    pub(crate) fn unknowns(self) -> usize {
        match self {
            Pole::Real(_) => 1,
            Pole::Pair(_) | Pole::Complex(_) => 2,
        }
    }

    pub(crate) fn value(self) -> c64 {
        match self {
            Pole::Real(a) => c64::new(a, 0.0),
            Pole::Pair(a) | Pole::Complex(a) => a,
        }
    }
}

const I: c64 = c64 { re: 0.0, im: 1.0 };

/// The basis functions of `poles`, a column each per real unknown, at `s`.
pub(crate) fn partial_fractions(poles: &[Pole], s: c64, out: &mut Vec<c64>) {
    for &p in poles {
        match p {
            Pole::Real(a) => out.push(1.0 / (s - a)),
            Pole::Pair(a) => {
                let (u, v) = (1.0 / (s - a), 1.0 / (s - a.conj()));
                out.push(u + v);
                out.push(I * u - I * v);
            }
            Pole::Complex(a) => {
                let u = 1.0 / (s - a);
                out.push(u);
                out.push(I * u);
            }
        }
    }
}

/// The derivatives d/ds of [`partial_fractions`]' columns at `s`.
pub(crate) fn partial_fraction_slopes(poles: &[Pole], s: c64, out: &mut Vec<c64>) {
    for &p in poles {
        match p {
            Pole::Real(a) => out.push(-1.0 / ((s - a) * (s - a))),
            Pole::Pair(a) => {
                let (u, v) = (1.0 / (s - a), 1.0 / (s - a.conj()));
                out.push(-u * u - v * v);
                out.push(-I * u * u + I * v * v);
            }
            Pole::Complex(a) => {
                let u = 1.0 / (s - a);
                out.push(-u * u);
                out.push(-I * u * u);
            }
        }
    }
}

/// The complex coefficients of `poles` (each pair's two poles) from the real unknowns `x`.
pub(crate) fn residues_of(poles: &[Pole], x: &[f64]) -> Vec<c64> {
    let mut out = Vec::new();
    let mut k = 0;
    for &p in poles {
        match p {
            Pole::Real(_) => {
                out.push(c64::new(x[k], 0.0));
                k += 1;
            }
            Pole::Pair(_) => {
                out.push(c64::new(x[k], x[k + 1]));
                out.push(c64::new(x[k], -x[k + 1]));
                k += 2;
            }
            Pole::Complex(_) => {
                out.push(c64::new(x[k], x[k + 1]));
                k += 2;
            }
        }
    }
    out
}

pub(crate) fn expanded(poles: &[Pole]) -> Vec<c64> {
    poles
        .iter()
        .flat_map(|&p| match p {
            Pole::Pair(a) => vec![a, a.conj()],
            other => vec![other.value()],
        })
        .collect()
}

/// The basis columns of the terms d and s h, for `symmetry`.
pub(crate) fn polynomial_terms(options: &Options, s: c64, out: &mut Vec<c64>) {
    let complex = options.symmetry == Symmetry::Complex;
    if options.constant {
        out.push(c64::new(1.0, 0.0));
        if complex {
            out.push(I);
        }
    }
    if options.proportional {
        out.push(s);
        if complex {
            out.push(I * s);
        }
    }
}

/// Singular values below this, relative to the largest, are dropped from a least squares: the
/// directions the data can't determine, as when more poles are asked for than the response has.
const RCOND: f64 = 1e-13;

/// Real least squares min ‖A x − b‖, columns scaled to unit norm first, and solved through the
/// singular values of R (A = QR), the smallest dropped below [`RCOND`]: the minimum-norm solution
/// when the problem is rank-deficient.
pub(crate) fn least_squares(a: Mat<f64>, b: &Mat<f64>) -> Vec<f64> {
    let (m, n) = (a.nrows(), a.ncols());
    let scale: Vec<f64> = (0..n)
        .map(|j| {
            let v = a.col(j).norm_l2();
            if v > 0.0 { 1.0 / v } else { 1.0 }
        })
        .collect();
    // [A D, b] = QR: R's first n columns and its last column, Qᵀb
    let augmented = Mat::from_fn(m, n + 1, |i, j| {
        if j < n {
            a[(i, j)] * scale[j]
        } else {
            b[(i, 0)]
        }
    });
    let qr = augmented.qr();
    let r = qr.thin_R();
    let rows = r.nrows().min(n);
    let rn = Mat::from_fn(rows, n, |i, j| r[(i, j)]);
    let qtb: Vec<f64> = (0..rows).map(|i| r[(i, n)]).collect();
    let svd = rn.thin_svd().expect("the SVD of a small dense matrix");
    let (u, s, v) = (svd.U(), svd.S().column_vector(), svd.V());
    let top = s.iter().copied().fold(0.0, f64::max);
    let mut x = vec![0.0; n];
    for k in 0..s.nrows() {
        let sk = s[k];
        if sk <= RCOND * top || sk == 0.0 {
            continue;
        }
        let coef = (0..rows).map(|i| u[(i, k)] * qtb[i]).sum::<f64>() / sk;
        for (j, xj) in x.iter_mut().enumerate() {
            *xj += coef * v[(j, k)];
        }
    }
    x.iter().zip(&scale).map(|(x, w)| x * w).collect()
}

/// Rows of a real least squares from complex equations: real parts, then imaginary parts.
pub(crate) fn real_rows(rows: &[Vec<c64>]) -> Mat<f64> {
    let (m, n) = (rows.len(), rows.first().map_or(0, Vec::len));
    Mat::from_fn(2 * m, n, |i, j| {
        if i < m {
            rows[i][j].re
        } else {
            rows[i - m][j].im
        }
    })
}

/// The paper's starting poles (Section 3.2, Eqs. 9–10) for `count` poles over the samples `s`:
/// complex, their imaginary parts β spread linearly over the samples' range of Im s, and real
/// parts −β/100. A real model's come in conjugate pairs (`count`/2 values of β over the range of
/// |Im s|); a complex model's take the sign of the samples' Im s.
///
/// For a band far from zero frequency, β/100 is much larger than the poles' spacing; the
/// real parts are then −min(β/100, Δβ/2), Δβ the spacing, which keeps each pole's reach within
/// its share of the band as the paper intends.
pub fn starting_poles(s: &[c64], count: usize, symmetry: Symmetry) -> Vec<c64> {
    let im: Vec<f64> = s.iter().map(|z| z.im).collect();
    let (lo, hi) = match symmetry {
        Symmetry::Real => {
            let a: Vec<f64> = im.iter().map(|v| v.abs()).collect();
            (
                a.iter().copied().fold(f64::INFINITY, f64::min),
                a.iter().copied().fold(0.0, f64::max),
            )
        }
        Symmetry::Complex => (
            im.iter().copied().fold(f64::INFINITY, f64::min),
            im.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ),
    };
    let n = match symmetry {
        Symmetry::Real => count / 2,
        Symmetry::Complex => count,
    };
    let step = if n > 1 {
        (hi - lo) / (n - 1) as f64
    } else {
        0.0
    };
    let beta = |k: usize| {
        if n > 1 {
            lo + step * k as f64
        } else {
            0.5 * (lo + hi)
        }
    };
    let alpha = |b: f64| {
        let paper = b.abs() / 100.0;
        if step > 0.0 {
            paper.min(step / 2.0)
        } else {
            paper
        }
        .max(f64::MIN_POSITIVE)
    };
    (0..n)
        .flat_map(|k| {
            let b = beta(k);
            let a = c64::new(-alpha(b), b);
            match symmetry {
                Symmetry::Real => vec![a, a.conj()],
                Symmetry::Complex => vec![a],
            }
        })
        .collect()
}

/// The delay of a response sampled at `s`: minus the least-squares slope of its unwrapped phase
/// against Im s. A delay τ is e^(−sτ), whose phase falls by τ per unit of Im s.
pub fn estimate_delay(s: &[c64], values: &[c64]) -> f64 {
    if s.len() < 2 {
        return 0.0;
    }
    // unwrap in the order of Im s
    let mut order: Vec<usize> = (0..s.len()).collect();
    order.sort_by(|&a, &b| s[a].im.total_cmp(&s[b].im));
    let mut phase = Vec::with_capacity(s.len());
    let mut last: Option<f64> = None;
    for &k in &order {
        let mut p = values[k].arg();
        if let Some(prev) = last {
            p += std::f64::consts::TAU * ((prev - p) / std::f64::consts::TAU).round();
        }
        phase.push(p);
        last = Some(p);
    }
    let x: Vec<f64> = order.iter().map(|&k| s[k].im).collect();
    let (mx, mp) = (
        x.iter().sum::<f64>() / x.len() as f64,
        phase.iter().sum::<f64>() / x.len() as f64,
    );
    let (sxy, sxx) = x.iter().zip(&phase).fold((0.0, 0.0), |(a, b), (&xi, &pi)| {
        (a + (xi - mx) * (pi - mp), b + (xi - mx) * (xi - mx))
    });
    if sxx == 0.0 { 0.0 } else { -sxy / sxx }
}

/// Fits `responses[e][k]`, sampled at `s[k]`, with common poles: the paper's two stages,
/// repeated (see the [module docs](self)).
///
/// # Errors
///
/// [`Error::InvalidValue`] for no samples or responses, responses of different lengths,
/// non-finite values, an odd number of poles for a real model, fewer samples than unknowns per
/// response, or starting poles that don't match the options.
pub fn vector_fit(s: &[c64], responses: &[Vec<c64>], options: &Options) -> Result<Rational> {
    let bad = |reason: String| Err(Error::invalid("vector fit", reason));
    let k = s.len();
    if k == 0 || responses.is_empty() {
        return bad("needs samples and at least one response".into());
    }
    if let Some(r) = responses.iter().find(|r| r.len() != k) {
        return bad(format!(
            "every response needs {k} samples, one has {}",
            r.len()
        ));
    }
    if s.iter()
        .chain(responses.iter().flatten())
        .any(|z| !(z.re.is_finite() && z.im.is_finite()))
    {
        return bad("samples and responses must be finite".into());
    }
    if options.poles == 0 {
        return bad("needs at least one pole".into());
    }
    if options.symmetry == Symmetry::Real && !options.poles.is_multiple_of(2) {
        return bad(format!(
            "a real model's poles come in pairs: {} is odd",
            options.poles
        ));
    }
    let start = match &options.starting_poles {
        Some(p) => p.clone(),
        None => starting_poles(s, options.poles, options.symmetry),
    };
    let mut poles = classify(&start, options.symmetry)?;
    if expanded(&poles).len() != options.poles {
        return bad(format!(
            "{} starting poles for a fit of {} poles",
            expanded(&poles).len(),
            options.poles
        ));
    }
    // per response: own unknowns (residues and polynomial terms) as real numbers
    let mut terms = Vec::new();
    polynomial_terms(options, c64::new(1.0, 0.0), &mut terms);
    // (a real model has one real unknown per pole; a complex model two)
    let own = poles.iter().map(|p| p.unknowns()).sum::<usize>() + terms.len();
    if 2 * k < own + 1 {
        return bad(format!(
            "{k} samples can't determine {own} real unknowns per response: use more samples or fewer poles"
        ));
    }
    // the delays, taken out
    let delays: Vec<f64> = match &options.delay {
        Delay::None => vec![0.0; responses.len()],
        Delay::Fixed(t) => vec![*t; responses.len()],
        Delay::Estimate => responses.iter().map(|r| estimate_delay(s, r)).collect(),
    };
    let data: Vec<Vec<c64>> = responses
        .iter()
        .zip(&delays)
        .map(|(r, &t)| r.iter().zip(s).map(|(&v, &z)| v * (z * t).exp()).collect())
        .collect();

    let mut best: Option<Rational> = None;
    let mut iterations = 0;
    for _ in 0..options.iterations.max(1) {
        let new = relocate(s, &data, &poles, options)?;
        iterations += 1;
        let moved = max_move(&expanded(&poles), &expanded(&new));
        poles = new;
        let fit = residues(s, &data, &poles, options, &delays, responses, iterations)?;
        if best.as_ref().is_none_or(|b| fit.error.rms < b.error.rms) {
            best = Some(fit);
        }
        if moved <= options.tolerance {
            break;
        }
    }
    let mut best = best.ok_or_else(|| Error::invalid("vector fit", "made no relocation"))?;
    best.iterations = iterations;
    Ok(best)
}

/// The largest move of a pole of `old` to its nearest in `new`, relative to its magnitude.
fn max_move(old: &[c64], new: &[c64]) -> f64 {
    old.iter()
        .map(|a| {
            new.iter()
                .map(|b| (a - b).norm())
                .fold(f64::INFINITY, f64::min)
                / a.norm().max(f64::MIN_POSITIVE)
        })
        .fold(0.0, f64::max)
}

/// Starting poles as the fit's poles: a real model's in conjugate pairs or real.
pub(crate) fn classify(poles: &[c64], symmetry: Symmetry) -> Result<Vec<Pole>> {
    match symmetry {
        Symmetry::Complex => Ok(poles.iter().map(|&a| Pole::Complex(a)).collect()),
        Symmetry::Real => {
            let mut out = Vec::new();
            let mut used = vec![false; poles.len()];
            for (i, &a) in poles.iter().enumerate() {
                if used[i] {
                    continue;
                }
                used[i] = true;
                if a.im == 0.0 {
                    out.push(Pole::Real(a.re));
                    continue;
                }
                let Some(j) = (0..poles.len()).find(|&j| !used[j] && poles[j] == a.conj()) else {
                    return Err(Error::invalid(
                        "vector fit",
                        format!(
                            "a real model's complex starting poles come in conjugate pairs: {a} has none"
                        ),
                    ));
                };
                used[j] = true;
                out.push(Pole::Pair(if a.im > 0.0 { a } else { a.conj() }));
            }
            Ok(out)
        }
    }
}

/// The rows of a least squares' R that hold its shared unknowns, after the block's own unknowns
/// are eliminated: `m` is one block [A B b], `own` columns of A (unknowns of this block alone),
/// `shared` of B (shared with other blocks) and the right side b. The QR factorization of the
/// column-scaled block leaves, below A's rows of R, the least squares that remains for the
/// shared unknowns: B's and b's columns there, the scaling of B's undone. The rows are pushed
/// onto `out`, `shared` + 1 values each.
pub(crate) fn eliminate(mut m: Mat<f64>, own: usize, shared: usize, out: &mut Vec<Vec<f64>>) {
    let scale: Vec<f64> = (0..m.ncols())
        .map(|j| {
            let v = m.col(j).norm_l2();
            if v > 0.0 { 1.0 / v } else { 1.0 }
        })
        .collect();
    for (j, &w) in scale.iter().enumerate().take(m.ncols() - 1) {
        m.col_mut(j).iter_mut().for_each(|v| *v *= w);
    }
    let qr = m.qr();
    let r = qr.thin_R();
    for i in own..r.nrows().min(own + shared) {
        out.push(
            (own..own + shared + 1)
                .map(|j| {
                    let v = r[(i, j)];
                    if j < own + shared { v / scale[j] } else { v }
                })
                .collect(),
        );
    }
}

/// The least squares of stacked rows from [`eliminate`]: the shared unknowns.
pub(crate) fn solve_stacked(stacked: &[Vec<f64>], shared: usize) -> Vec<f64> {
    let a = Mat::from_fn(stacked.len(), shared, |i, j| stacked[i][j]);
    let b = Mat::from_fn(stacked.len(), 1, |i, _| stacked[i][shared]);
    least_squares(a, &b)
}

/// The zeros of σ(s) = 1 + Σ c̃ φ(s) over the basis of `poles`, `c_tilde` its real unknowns:
/// the eigenvalues of A − b c̃ᵀ (the paper's B.1), a real model's pairs as real 2 × 2 blocks
/// (B.2), so that its zeros are real or exact conjugate pairs. Not made stable.
pub(crate) fn sigma_zeros(poles: &[Pole], c_tilde: &[f64]) -> Result<Vec<c64>> {
    let failed = |e: faer::linalg::evd::EvdError| {
        Error::invalid("vector fit", format!("the zeros of sigma: {e:?}"))
    };
    if poles.iter().all(|p| !matches!(p, Pole::Complex(_))) {
        let n = c_tilde.len();
        let mut h = Mat::<f64>::zeros(n, n);
        let mut b = vec![0.0; n];
        let mut at = 0;
        for &p in poles {
            match p {
                Pole::Real(a) => {
                    h[(at, at)] = a;
                    b[at] = 1.0;
                    at += 1;
                }
                Pole::Pair(a) => {
                    h[(at, at)] = a.re;
                    h[(at, at + 1)] = a.im;
                    h[(at + 1, at)] = -a.im;
                    h[(at + 1, at + 1)] = a.re;
                    b[at] = 2.0;
                    at += 2;
                }
                Pole::Complex(_) => unreachable!("checked: no lone complex poles"),
            }
        }
        for i in 0..n {
            for j in 0..n {
                h[(i, j)] -= b[i] * c_tilde[j];
            }
        }
        h.eigenvalues().map_err(failed)
    } else {
        // every pole complex, two real unknowns each
        let n = poles.len();
        let c: Vec<c64> = (0..n)
            .map(|j| c64::new(c_tilde[2 * j], c_tilde[2 * j + 1]))
            .collect();
        let h = Mat::<c64>::from_fn(n, n, |i, j| {
            let diag = if i == j {
                poles[i].value()
            } else {
                c64::new(0.0, 0.0)
            };
            diag - c[j]
        });
        h.eigenvalues().map_err(failed)
    }
}

/// New poles from σ's zeros, unstable ones flipped into the left half plane (the paper's
/// Section 2); a real model's as real poles and pairs.
pub(crate) fn stable_poles(zeros: Vec<c64>, symmetry: Symmetry) -> Vec<Pole> {
    let stable = |a: c64| c64::new(-a.re.abs(), a.im);
    match symmetry {
        Symmetry::Complex => zeros
            .into_iter()
            .map(|a| Pole::Complex(stable(a)))
            .collect(),
        Symmetry::Real => zeros
            .into_iter()
            .filter_map(|a| {
                // eigenvalues of a real matrix: real, or in exact conjugate pairs
                if a.im == 0.0 {
                    Some(Pole::Real(-a.re.abs()))
                } else if a.im > 0.0 {
                    Some(Pole::Pair(stable(a)))
                } else {
                    None
                }
            })
            .collect(),
    }
}

/// Stage 1: the zeros of σ, fitted with every response, as the new poles (stable).
fn relocate(s: &[c64], data: &[Vec<c64>], poles: &[Pole], options: &Options) -> Result<Vec<Pole>> {
    let k = s.len();
    let n_sigma: usize = poles.iter().map(|p| p.unknowns()).sum();
    // the basis at every sample
    let basis: Vec<Vec<c64>> = s
        .iter()
        .map(|&z| {
            let mut row = Vec::new();
            partial_fractions(poles, z, &mut row);
            row
        })
        .collect();
    let mut stacked: Vec<Vec<f64>> = Vec::new();
    for f in data {
        // [A B b]: the response's own columns (σf's), σ's columns −f φ, and f
        let rows: Vec<Vec<c64>> = (0..k)
            .map(|i| {
                let mut row = basis[i].clone();
                polynomial_terms(options, s[i], &mut row);
                row.extend(basis[i].iter().map(|&p| -f[i] * p));
                row.push(f[i]);
                row
            })
            .collect();
        let m = real_rows(&rows);
        let own = m.ncols() - n_sigma - 1;
        eliminate(m, own, n_sigma, &mut stacked);
    }
    let c_tilde = solve_stacked(&stacked, n_sigma);
    Ok(stable_poles(
        sigma_zeros(poles, &c_tilde)?,
        options.symmetry,
    ))
}

/// Stage 2: each response's residues and terms, with `poles` fixed, and the fit's error.
pub(crate) fn residues(
    s: &[c64],
    data: &[Vec<c64>],
    poles: &[Pole],
    options: &Options,
    delays: &[f64],
    original: &[Vec<c64>],
    iterations: usize,
) -> Result<Rational> {
    let k = s.len();
    let rows: Vec<Vec<c64>> = s
        .iter()
        .map(|&z| {
            let mut row = Vec::new();
            partial_fractions(poles, z, &mut row);
            polynomial_terms(options, z, &mut row);
            row
        })
        .collect();
    let a = real_rows(&rows);
    let n_res: usize = poles.iter().map(|p| p.unknowns()).sum();
    let mut model = Rational {
        poles: expanded(poles),
        residues: Vec::new(),
        constant: Vec::new(),
        proportional: Vec::new(),
        delays: delays.to_vec(),
        error: FitError {
            rms: 0.0,
            max: 0.0,
            rms_relative: 0.0,
            max_relative: 0.0,
        },
        iterations,
    };
    for f in data {
        let b = Mat::from_fn(2 * k, 1, |i, _| if i < k { f[i].re } else { f[i - k].im });
        let x = least_squares(a.clone(), &b);
        model.residues.push(residues_of(poles, &x[..n_res]));
        let mut rest = x[n_res..].iter();
        let mut next_term = || -> c64 {
            match options.symmetry {
                Symmetry::Real => c64::new(*rest.next().unwrap_or(&0.0), 0.0),
                Symmetry::Complex => {
                    let re = *rest.next().unwrap_or(&0.0);
                    c64::new(re, *rest.next().unwrap_or(&0.0))
                }
            }
        };
        let d = if options.constant {
            next_term()
        } else {
            c64::new(0.0, 0.0)
        };
        let h = if options.proportional {
            next_term()
        } else {
            c64::new(0.0, 0.0)
        };
        model.constant.push(d);
        model.proportional.push(h);
    }
    model.error = error_of(&model, s, original);
    Ok(model)
}

/// The model's error against `responses` at `s`.
pub(crate) fn error_of(model: &Rational, s: &[c64], responses: &[Vec<c64>]) -> FitError {
    let (mut sum_e, mut sum_f, mut max_e, mut max_f, mut count) =
        (0.0, 0.0, 0.0f64, 0.0f64, 0usize);
    for (e, f) in responses.iter().enumerate() {
        for (&z, &v) in s.iter().zip(f) {
            let d = (model.evaluate(e, z) - v).norm();
            sum_e += d * d;
            sum_f += v.norm_sqr();
            max_e = max_e.max(d);
            max_f = max_f.max(v.norm());
            count += 1;
        }
    }
    let rms = (sum_e / count as f64).sqrt();
    let rms_f = (sum_f / count as f64).sqrt();
    FitError {
        rms,
        max: max_e,
        rms_relative: if rms_f > 0.0 { rms / rms_f } else { rms },
        max_relative: if max_f > 0.0 { max_e / max_f } else { max_e },
    }
}

#[cfg(test)]
mod tests;

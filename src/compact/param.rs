//! Compact models over a component's parameters: rational in frequency, with numerator and
//! denominator coefficients that depend on the parameters.
//!
//! The model is the parameterized form of P. Triverio, S. Grivet-Talocia and M. S. Nakhla
//! (IEEE Trans. Adv. Packag. 32, 205 (2009), doi:10.1109/TADVP.2008.2007913, Eqs. 6, 7 and 11):
//!
//! H_e(s, p) = N_e(s, p) / D(s, p),
//! N_e = Σ_l w_l(p) (Σₙ c_enl/(s − aₙ) + d_el),
//! D = 1 + Σ_l w_l(p) Σₙ c̃_nl/(s − aₙ),
//!
//! numerator and denominator on one set of fixed basis poles aₙ, which cancel between them
//! (their Section IV-B): the model's poles at p are the zeros of D(·, p), the eigenvalues of
//! A − b c̃(p)ᵀ (B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999),
//! doi:10.1109/61.772353, Appendix B), and their residues N(z)/D′(z). D's constant term is fixed
//! at 1, Triverio's free r₀ (their Theorem 1), so D never vanishes at high frequency and no pole
//! goes to infinity. The weights w_l(p) are one of two kinds ([`Interpolation`]):
//!
//! - **Piecewise linear**, Triverio's (their Eqs. 8–9): the samples on a grid, each fitted on its
//!   own by vector fitting, its poles stable, and its pole–residue model rewritten exactly on the
//!   basis poles (their Theorem 1, Eqs. 24–26); between samples the coefficients are linear
//!   (multilinear for several parameters). The basis poles are spread linearly over the band
//!   (their Section III-C). At the samples the model is stable by construction (their Section
//!   III-D), and with one parameter [`ParametricModel::uniform_stability`] finds every value where
//!   a pole crosses into the right half plane, without sampling.
//! - **Polynomial**: monomials of total degree at most D in the parameters, each scaled to
//!   [−1, 1] over the samples' range, the basis poles every sample's common poles by vector
//!   fitting, and N − f D = 0 solved at every sample and frequency as one linear least squares
//!   (the responses' own unknowns eliminated block by block as in [`super::fit`]). That least
//!   squares weights each equation by |D|, small at resonances, so it is solved again with each
//!   equation divided by the last solve's |D|: the iteration of C. K. Sanathanan and J. Koerner
//!   (IEEE Trans. Autom. Control 8, 56 (1963), doi:10.1109/TAC.1963.1105517, Eqs. 5–7), whose
//!   first solve, with no weight, is their Eq. 3. They ran 10 iterations; here up to 10, stopping
//!   when three in a row don't improve the largest error at the samples, and keeping the best.
//!
//! **Which.** On an all-pass ring whose resonances shift by 0.8 of a free spectral range, about
//! 35 linewidths, over its samples, the piecewise-linear model fails between samples (|ΔS| of 7
//! to 24, however many samples): a straight line between two denominators whose zeros lie more
//! than a linewidth apart doesn't move the zero, it makes new ones. The polynomial model, fitted
//! to all samples at once, gets 2e-7 there (degree 6, 13 samples). Over the ring's coupling,
//! which changes no resonance's position, the piecewise-linear model is within 9e-4 from 5
//! samples and the polynomial one within 3e-10 (Eq. 1 is a ratio of functions linear in r). The
//! polynomial model's poles aren't constrained, though, and some fall in the right half plane,
//! out of the band; [`ParametricModel::stability`] samples them and
//! [`ParametricModel::stable_rational`] makes a stable model at a point. (Both measured in the
//! tests and the validation report.)
//!
//! The model's error is measured at the samples ([`ParametricModel::error`]) and should be
//! measured again at parameter values that weren't fitted ([`ParametricModel::error_against`]).

use faer::Mat;
use num_complex::Complex64 as c64;

use super::fit::{
    self, Delay, FitError, Options, Pole, Rational, Symmetry, classify, eliminate, expanded,
    least_squares, partial_fraction_slopes, partial_fractions, polynomial_terms, real_rows,
    residues, sequential, sigma_realization, sigma_zeros, solve_stacked, stable_poles,
    starting_poles,
};
use super::model::{band_of, check_band, laplace, responses_of};
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, Spectrum, ports};
use crate::units::Wavelength;
use crate::{Error, Result};

/// At most this many least squares, the first unweighted and the rest reweighted by 1/|D|
/// (Sanathanan and Koerner ran 10).
const SOLVES: usize = 10;

/// A spectrum at one point of a component's parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The parameters' values, in the order of the model's parameters.
    pub values: Vec<f64>,
    /// The spectrum there.
    pub spectrum: Spectrum,
}

/// How a [`ParametricModel`]'s coefficients depend on the parameters (see the
/// [module docs](self)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    /// Polynomials of total degree `degree`, all samples fitted together, reweighted as
    /// Sanathanan and Koerner do. Follows resonances that move with the parameters.
    Polynomial {
        /// The total degree.
        degree: usize,
    },
    /// Triverio et al.'s: each sample, on a grid, fitted alone and its stable model rewritten
    /// on the basis poles; linear (multilinear) between samples. Stable at the samples, and its
    /// stability between them can be decided ([`ParametricModel::uniform_stability`]); for
    /// responses that change by less than a linewidth between samples.
    PiecewiseLinear,
}

/// The parameter weights w_l(p).
#[derive(Clone, Debug, PartialEq)]
enum Weights {
    /// Monomials in the scaled parameters: each parameter's exponent.
    Monomials(Vec<Vec<usize>>),
    /// Multilinear hat functions on a grid: each parameter's sample values, increasing. The
    /// nodes are numbered with the first parameter fastest.
    Grid(Vec<Vec<f64>>),
}

/// Where a pole of a [`ParametricModel`] crosses the imaginary axis: at parameter value
/// `value`, at angular frequency ω = `omega` (rad/µm; s = −iω). Beyond it, along the parameter,
/// the model is unstable until another crossing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    /// The parameter's value.
    pub value: f64,
    /// The angular frequency of the pole on the axis, ω in s = −iω.
    pub omega: f64,
}

/// A rational model in frequency whose numerator and denominator coefficients depend on the
/// component's parameters (see the [module docs](self)): a [`Component`] of
/// [`Fidelity::Compact`] with parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct ParametricModel {
    kind: String,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    band: (Wavelength, Wavelength),
    /// The samples' range of each parameter, as its centre and half-width.
    centre: Vec<f64>,
    half: Vec<f64>,
    weights: Weights,
    symmetry: Symmetry,
    constant: bool,
    basis: Vec<Pole>,
    /// Response e's real unknowns: basis column j and weight l at j L + l, then the constant
    /// term's columns, each times the weights.
    numerator: Vec<Vec<f64>>,
    /// D's real unknowns, basis column j and weight l at j L + l.
    denominator: Vec<f64>,
    delay: f64,
    error: FitError,
    basis_error: FitError,
    source: String,
}

/// The monomials of total degree at most `degree` in `vars` variables, constant first.
fn monomials(vars: usize, degree: usize) -> Vec<Vec<usize>> {
    fn fill(i: usize, left: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if i + 1 == current.len() {
            current[i] = left;
            out.push(current.clone());
            return;
        }
        for e in (0..=left).rev() {
            current[i] = e;
            fill(i + 1, left - e, current, out);
        }
    }
    let mut out = vec![vec![0; vars]];
    if vars > 0 {
        for d in 1..=degree {
            fill(0, d, &mut vec![0; vars], &mut out);
        }
    }
    out
}

fn evaluate_monomials(exponents: &[Vec<usize>], x: &[f64]) -> Vec<f64> {
    exponents
        .iter()
        .map(|e| e.iter().zip(x).map(|(&k, &v)| v.powi(k as i32)).product())
        .collect()
}

/// The multilinear hat-function weights of the grid's nodes at `values` (within the grid).
fn grid_weights(axes: &[Vec<f64>], values: &[f64]) -> Vec<f64> {
    let count: usize = axes.iter().map(Vec::len).product();
    let mut w = vec![0.0; count];
    // per parameter: the segment and the position in it
    let segments: Vec<(usize, f64)> = axes
        .iter()
        .zip(values)
        .map(|(axis, &v)| {
            let k = axis.partition_point(|&a| a <= v).clamp(1, axis.len() - 1);
            let t = ((v - axis[k - 1]) / (axis[k] - axis[k - 1])).clamp(0.0, 1.0);
            (k - 1, t)
        })
        .collect();
    for corner in 0..(1usize << axes.len()) {
        let (mut index, mut stride, mut weight) = (0, 1, 1.0);
        for (d, (axis, &(k, t))) in axes.iter().zip(&segments).enumerate() {
            let upper = corner >> d & 1 == 1;
            index += (k + usize::from(upper)) * stride;
            weight *= if upper { t } else { 1.0 - t };
            stride *= axis.len();
        }
        w[index] += weight;
    }
    w
}

/// The error of `value(sample, k, e)` against the samples' responses.
fn error_over(
    samples: &[(Vec<Vec<c64>>, Vec<c64>)],
    value: impl Fn(usize, usize, usize) -> c64,
) -> FitError {
    let (mut sum_e, mut sum_f, mut max_e, mut max_f, mut count) =
        (0.0, 0.0, 0.0f64, 0.0f64, 0usize);
    for (i, (responses, _)) in samples.iter().enumerate() {
        for (e, f) in responses.iter().enumerate() {
            for (k, &v) in f.iter().enumerate() {
                let d = (value(i, k, e) - v).norm();
                sum_e += d * d;
                sum_f += v.norm_sqr();
                max_e = max_e.max(d);
                max_f = max_f.max(v.norm());
                count += 1;
            }
        }
    }
    let n = count.max(1) as f64;
    let (rms, rms_f) = ((sum_e / n).sqrt(), (sum_f / n).sqrt());
    FitError {
        rms,
        max: max_e,
        rms_relative: if rms_f > 0.0 { rms / rms_f } else { rms },
        max_relative: if max_f > 0.0 { max_e / max_f } else { max_e },
    }
}

/// `values`, one per expanded basis pole, as the basis's real unknowns: a pair's from its pole
/// with positive imaginary part, a lone complex pole's as its real and imaginary parts.
fn unknowns_of(basis: &[Pole], values: &[c64]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut k = 0;
    for &p in basis {
        match p {
            Pole::Real(_) => {
                out.push(values[k].re);
                k += 1;
            }
            Pole::Pair(_) => {
                out.push(values[k].re);
                out.push(values[k].im);
                k += 2;
            }
            Pole::Complex(_) => {
                out.push(values[k].re);
                out.push(values[k].im);
                k += 1;
            }
        }
    }
    out
}

/// Triverio et al.'s Theorem 1 with r₀ = 1: a stable pole–residue model `local` rewritten as
/// N/D on `basis`: D's coefficients from D(p_l) = 0 at every local pole (their Eq. 24), N's
/// from Rₙ = rₙ H(aₙ) (Eq. 25) and R₀ = Q₀ (Eq. 26). Returns D's real unknowns and each
/// response's N's.
fn rewrite(
    local: &Rational,
    basis: &[Pole],
    symmetry: Symmetry,
    constant: bool,
) -> Result<(Vec<f64>, Vec<Vec<f64>>)> {
    use faer::linalg::solvers::Solve;
    let a = expanded(basis);
    let p = &local.poles;
    let n = a.len();
    if p.len() != n {
        return Err(Error::invalid(
            "parametric model",
            format!("a sample's fit has {} poles for {n} basis poles", p.len()),
        ));
    }
    // Σₙ rₙ/(p_l − aₙ) = −1, a Cauchy matrix
    let m = Mat::<c64>::from_fn(n, n, |l, k| 1.0 / (p[l] - a[k]));
    let r = m
        .partial_piv_lu()
        .solve(Mat::<c64>::from_fn(n, 1, |_, _| c64::new(-1.0, 0.0)));
    let r: Vec<c64> = (0..n).map(|k| r[(k, 0)]).collect();
    if r.iter().any(|v| !(v.re.is_finite() && v.im.is_finite())) {
        return Err(Error::invalid(
            "parametric model",
            "a sample's poles can't be rewritten on the basis poles (a pole on a basis pole)",
        ));
    }
    let den = unknowns_of(basis, &r);
    let num = (0..local.responses())
        .map(|e| {
            let big_r: Vec<c64> = (0..n).map(|k| r[k] * local.evaluate(e, a[k])).collect();
            let mut x = unknowns_of(basis, &big_r);
            if constant {
                let q0 = local.constant[e];
                x.push(q0.re);
                if symmetry == Symmetry::Complex {
                    x.push(q0.im);
                }
            }
            x
        })
        .collect();
    Ok((den, num))
}

impl ParametricModel {
    /// Fits `samples`, spectra at values of `parameters` sharing their ports and wavelengths:
    /// `options` for the rational part (the number of poles, symmetry, constant term and
    /// delay) and `interpolation` for the parameters (see the [module docs](self)).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a sample's values don't match the parameters or aren't
    /// allowed by them, if the spectra don't share their ports and wavelengths, if a parameter
    /// doesn't vary over the samples, if there are fewer samples than polynomial coefficients,
    /// if [`Interpolation::PiecewiseLinear`]'s samples aren't a full grid, for a proportional
    /// term or an estimated delay (a fixed one is allowed), or for a fit's own reasons
    /// ([`fit::vector_fit`]).
    pub fn fit(
        parameters: Vec<Parameter>,
        samples: &[Sample],
        options: &Options,
        interpolation: Interpolation,
    ) -> Result<ParametricModel> {
        sequential(|| Self::fit_on_one_thread(parameters, samples, options, interpolation))
    }

    fn fit_on_one_thread(
        parameters: Vec<Parameter>,
        samples: &[Sample],
        options: &Options,
        interpolation: Interpolation,
    ) -> Result<ParametricModel> {
        let bad = |reason: String| Err(Error::invalid("parametric model", reason));
        let Some(first) = samples.first() else {
            return bad("needs samples".into());
        };
        if options.proportional {
            return bad("has no proportional term: an S-matrix stays bounded".into());
        }
        let delay = match options.delay {
            Delay::None => 0.0,
            Delay::Fixed(t) => t,
            Delay::Estimate => {
                return bad("takes a fixed delay or none, not an estimated one".into());
            }
        };
        let vars = parameters.len();
        for s in samples {
            if s.values.len() != vars {
                return bad(format!(
                    "a sample has {} values for {vars} parameters",
                    s.values.len()
                ));
            }
            if let Some((p, v)) = parameters
                .iter()
                .zip(&s.values)
                .find(|(p, v)| !p.allows(**v))
            {
                return bad(format!(
                    "{} = {v} is outside its range, {} to {}",
                    p.name, p.min, p.max
                ));
            }
            if s.spectrum.ports() != first.spectrum.ports()
                || s.spectrum.wavelengths() != first.spectrum.wavelengths()
            {
                return bad(
                    "every sample's spectrum must have the same ports and wavelengths".into(),
                );
            }
        }
        let wavelengths = first.spectrum.wavelengths();
        if wavelengths.len() < 2 || first.spectrum.ports().is_empty() {
            return bad("needs spectra of at least two wavelengths and a port".into());
        }
        let (mut centre, mut half) = (Vec::new(), Vec::new());
        for (i, p) in parameters.iter().enumerate() {
            let lo = samples
                .iter()
                .map(|s| s.values[i])
                .fold(f64::INFINITY, f64::min);
            let hi = samples
                .iter()
                .map(|s| s.values[i])
                .fold(f64::NEG_INFINITY, f64::max);
            if hi <= lo {
                return bad(format!("{} doesn't vary over the samples", p.name));
            }
            centre.push(0.5 * (lo + hi));
            half.push(0.5 * (hi - lo));
        }
        let weights = match interpolation {
            Interpolation::Polynomial { degree } => {
                let exponents = monomials(vars, degree);
                if samples.len() < exponents.len() {
                    return bad(format!(
                        "{} samples can't determine a polynomial of degree {degree} in {vars} parameters ({} coefficients)",
                        samples.len(),
                        exponents.len()
                    ));
                }
                Weights::Monomials(exponents)
            }
            Interpolation::PiecewiseLinear => {
                let axes: Vec<Vec<f64>> = (0..vars)
                    .map(|i| {
                        let mut v: Vec<f64> = samples.iter().map(|s| s.values[i]).collect();
                        v.sort_by(f64::total_cmp);
                        v.dedup();
                        v
                    })
                    .collect();
                let nodes: usize = axes.iter().map(Vec::len).product();
                if nodes != samples.len() {
                    return bad(format!(
                        "piecewise-linear interpolation needs the samples on a full grid: {} samples, but the parameters' values make {nodes} combinations",
                        samples.len()
                    ));
                }
                Weights::Grid(axes)
            }
        };
        let s: Vec<c64> = wavelengths.iter().map(|&w| laplace(w)).collect();
        let names: Vec<&str> = first.spectrum.ports().iter().map(String::as_str).collect();
        let mut model = ParametricModel {
            kind: "compact model".into(),
            ports: ports(&names),
            parameters,
            band: band_of(wavelengths),
            centre,
            half,
            weights,
            symmetry: options.symmetry,
            constant: options.constant,
            basis: Vec::new(),
            numerator: Vec::new(),
            denominator: Vec::new(),
            delay,
            error: FitError {
                rms: 0.0,
                max: 0.0,
                rms_relative: 0.0,
                max_relative: 0.0,
            },
            basis_error: FitError {
                rms: 0.0,
                max: 0.0,
                rms_relative: 0.0,
                max_relative: 0.0,
            },
            source: format!("spectra at {} parameter samples", samples.len()),
        };
        // each sample's responses, the delay taken out, and its weights
        let data: Vec<(Vec<Vec<c64>>, Vec<c64>)> = samples
            .iter()
            .map(|smp| {
                let responses = responses_of(&smp.spectrum)
                    .into_iter()
                    .map(|r| {
                        r.iter()
                            .zip(&s)
                            .map(|(&v, &z)| v * (z * delay).exp())
                            .collect()
                    })
                    .collect();
                Ok((responses, model.weights_at(&smp.values)?))
            })
            .collect::<Result<_>>()?;
        let local = Options {
            delay: Delay::None,
            ..options.clone()
        };
        match &model.weights {
            Weights::Monomials(_) => {
                // the basis: every sample's spectrum fitted with common poles
                let all: Vec<Vec<c64>> = data.iter().flat_map(|(r, _)| r.iter().cloned()).collect();
                let shared = fit::vector_fit(&s, &all, &local)?;
                model.basis = classify(&shared.poles, options.symmetry)?;
                model.basis_error = shared.error;
                model.solve(&s, &data)?;
            }
            Weights::Grid(axes) => {
                // each sample fitted alone, rewritten on poles spread over the band
                model.basis = classify(
                    &starting_poles(&s, options.poles, options.symmetry),
                    options.symmetry,
                )?;
                let nodes: usize = axes.iter().map(Vec::len).product();
                let n_den: usize = model.basis.iter().map(|p| p.unknowns()).sum();
                let n_terms = usize::from(options.constant)
                    * if options.symmetry == Symmetry::Complex {
                        2
                    } else {
                        1
                    };
                let n_resp = data[0].0.len();
                model.denominator = vec![0.0; n_den * nodes];
                model.numerator = vec![vec![0.0; (n_den + n_terms) * nodes]; n_resp];
                let mut worst = FitError {
                    rms: 0.0,
                    max: 0.0,
                    rms_relative: 0.0,
                    max_relative: 0.0,
                };
                for (responses, w) in &data {
                    let node = w
                        .iter()
                        .position(|x| (x.re - 1.0).abs() < 1e-12)
                        .ok_or_else(|| {
                            Error::invalid("parametric model", "a sample isn't a node of its grid")
                        })?;
                    let fit = fit::vector_fit(&s, responses, &local)?;
                    let (den, num) =
                        rewrite(&fit, &model.basis, options.symmetry, options.constant)?;
                    for (j, v) in den.into_iter().enumerate() {
                        model.denominator[j * nodes + node] = v;
                    }
                    for (e, x) in num.into_iter().enumerate() {
                        for (j, v) in x.into_iter().enumerate() {
                            model.numerator[e][j * nodes + node] = v;
                        }
                    }
                    worst = FitError {
                        rms: worst.rms.max(fit.error.rms),
                        max: worst.max.max(fit.error.max),
                        rms_relative: worst.rms_relative.max(fit.error.rms_relative),
                        max_relative: worst.max_relative.max(fit.error.max_relative),
                    };
                }
                model.basis_error = worst;
            }
        }
        // the error against the original responses, delay and all
        let originals: Vec<(Vec<Vec<c64>>, Vec<c64>)> = samples
            .iter()
            .zip(&data)
            .map(|(smp, (_, m))| (responses_of(&smp.spectrum), m.clone()))
            .collect();
        model.error = error_over(&originals, |i, k, e| {
            model.evaluate(e, s[k], &originals[i].1) * (-s[k] * delay).exp()
        });
        Ok(model)
    }

    /// The columns of N's and D's real unknowns at `s` and weights `m`.
    fn columns(&self, s: c64, m: &[c64]) -> (Vec<c64>, Vec<c64>) {
        let mut phi = Vec::new();
        partial_fractions(&self.basis, s, &mut phi);
        let mut terms = Vec::new();
        polynomial_terms(&self.options(), s, &mut terms);
        let d: Vec<c64> = phi
            .iter()
            .flat_map(|&p| m.iter().map(move |&x| p * x))
            .collect();
        let mut n = d.clone();
        n.extend(terms.iter().flat_map(|&t| m.iter().map(move |&x| t * x)));
        (n, d)
    }

    fn options(&self) -> Options {
        Options {
            symmetry: self.symmetry,
            constant: self.constant,
            proportional: false,
            ..Options::new(1)
        }
    }

    /// N_e/D at `s`, weights `m`, the delay not included.
    fn evaluate(&self, e: usize, s: c64, m: &[c64]) -> c64 {
        let (n, d) = self.columns(s, m);
        let num: c64 = n.iter().zip(&self.numerator[e]).map(|(c, x)| c * x).sum();
        let den: c64 = d
            .iter()
            .zip(&self.denominator)
            .map(|(c, x)| c * x)
            .sum::<c64>()
            + 1.0;
        num / den
    }

    /// The least squares N − f D = 0, reweighted by 1/|D| (Sanathanan and Koerner), the best
    /// of the solves kept.
    fn solve(&mut self, s: &[c64], data: &[(Vec<Vec<c64>>, Vec<c64>)]) -> Result<()> {
        let n_resp = data[0].0.len();
        let k = s.len();
        let mut weights = vec![vec![1.0; k]; data.len()];
        let mut best: Option<(f64, Vec<Vec<f64>>, Vec<f64>)> = None;
        let mut since_best = 0;
        for _ in 0..SOLVES {
            // the shared unknowns (D's) from every response's eliminated block
            let mut stacked = Vec::new();
            let (mut own, mut n_shared) = (0, 0);
            for e in 0..n_resp {
                let mut rows = Vec::with_capacity(data.len() * k);
                for (i, (responses, m)) in data.iter().enumerate() {
                    for (j, &z) in s.iter().enumerate() {
                        let w = weights[i][j];
                        let f = responses[e][j];
                        let (n, d) = self.columns(z, m);
                        own = n.len();
                        n_shared = d.len();
                        let mut row: Vec<c64> = n.iter().map(|&v| v * w).collect();
                        row.extend(d.iter().map(|&v| -f * v * w));
                        row.push(f * w);
                        rows.push(row);
                    }
                }
                eliminate(real_rows(&rows), own, n_shared, &mut stacked);
            }
            let c_tilde = solve_stacked(&stacked, n_shared);
            // each response's own unknowns with D fixed: N ≈ f D
            let mut numerator = Vec::with_capacity(n_resp);
            for e in 0..n_resp {
                let mut rows = Vec::with_capacity(data.len() * k);
                let mut rhs = Vec::with_capacity(data.len() * k);
                for (i, (responses, m)) in data.iter().enumerate() {
                    for (j, &z) in s.iter().enumerate() {
                        let w = weights[i][j];
                        let (n, d) = self.columns(z, m);
                        let den: c64 =
                            d.iter().zip(&c_tilde).map(|(c, x)| c * x).sum::<c64>() + 1.0;
                        rows.push(n.iter().map(|&v| v * w).collect::<Vec<c64>>());
                        rhs.push(responses[e][j] * den * w);
                    }
                }
                let a = real_rows(&rows);
                let r = rhs.len();
                let b = Mat::from_fn(
                    2 * r,
                    1,
                    |i, _| if i < r { rhs[i].re } else { rhs[i - r].im },
                );
                numerator.push(least_squares(a, &b));
            }
            self.numerator = numerator;
            self.denominator = c_tilde;
            let err = error_over(data, |i, j, e| self.evaluate(e, s[j], &data[i].1)).max;
            if best.as_ref().is_none_or(|b| err < b.0) {
                best = Some((err, self.numerator.clone(), self.denominator.clone()));
                since_best = 0;
            } else {
                since_best += 1;
                if since_best == 3 {
                    break;
                }
            }
            // reweight by the last denominator (their Eq. 5: the error over |Q_(L−1)|)
            for (i, (_, m)) in data.iter().enumerate() {
                for (j, &z) in s.iter().enumerate() {
                    let (_, d) = self.columns(z, m);
                    let den: c64 = d
                        .iter()
                        .zip(&self.denominator)
                        .map(|(c, x)| c * x)
                        .sum::<c64>()
                        + 1.0;
                    weights[i][j] = 1.0 / den.norm().max(f64::MIN_POSITIVE);
                }
            }
        }
        let (_, numerator, denominator) = best.expect("at least one solve");
        self.numerator = numerator;
        self.denominator = denominator;
        Ok(())
    }

    /// The same model, its kind named `kind`.
    #[must_use]
    pub fn with_kind(mut self, kind: impl Into<String>) -> ParametricModel {
        self.kind = kind.into();
        self
    }

    /// The same model, saying what its spectra came from, for its [`Provenance`].
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> ParametricModel {
        self.source = source.into();
        self
    }

    /// The model's error against the samples it was fitted to.
    pub fn error(&self) -> FitError {
        self.error
    }

    /// For [`Interpolation::Polynomial`], the error of the common basis poles' own fit: every
    /// sample with the same poles and residues of their own, the model a shared-pole
    /// interpolation starts from. For [`Interpolation::PiecewiseLinear`], the worst of the
    /// samples' own fits.
    pub fn basis_error(&self) -> FitError {
        self.basis_error
    }

    /// The band.
    pub fn band(&self) -> (Wavelength, Wavelength) {
        self.band
    }

    /// The weights at parameter `values`, checked against the sampled range.
    fn weights_at(&self, values: &[f64]) -> Result<Vec<c64>> {
        if values.len() != self.parameters.len() {
            return Err(Error::invalid(
                "parametric model",
                format!(
                    "takes {} parameter values, not {}",
                    self.parameters.len(),
                    values.len()
                ),
            ));
        }
        let mut x = Vec::with_capacity(values.len());
        for ((v, (c, h)), p) in values
            .iter()
            .zip(self.centre.iter().zip(&self.half))
            .zip(&self.parameters)
        {
            let u = (v - c) / h;
            if u.is_nan() || u.abs() > 1.0 + 1e-12 {
                return Err(Error::invalid(
                    "parametric model",
                    format!(
                        "{} was sampled from {} to {}, not at {v}",
                        p.name,
                        c - h,
                        c + h
                    ),
                ));
            }
            x.push(u);
        }
        let w = match &self.weights {
            Weights::Monomials(exponents) => evaluate_monomials(exponents, &x),
            Weights::Grid(axes) => grid_weights(axes, values),
        };
        Ok(w.into_iter().map(|v| c64::new(v, 0.0)).collect())
    }

    /// D's coefficients at weights `m`.
    fn sigma_at(&self, m: &[c64]) -> Vec<f64> {
        let l = m.len();
        self.denominator
            .chunks(l)
            .map(|c| c.iter().zip(m).map(|(a, b)| a * b.re).sum())
            .collect()
    }

    /// The model at parameter `values` in pole–residue form: the poles are the zeros of D, the
    /// residues N(z)/D′(z), the constant N's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for the wrong number of values, values outside the range the
    /// samples cover (the model says nothing outside it), or if D's zeros can't be found.
    pub fn rational(&self, values: &[f64]) -> Result<Rational> {
        let m = self.weights_at(values)?;
        let c = self.sigma_at(&m);
        let poles = sigma_zeros(&self.basis, &c)?;
        let n_resp = self.numerator.len();
        let mut residues = vec![Vec::with_capacity(poles.len()); n_resp];
        for &z in &poles {
            let (n, _) = self.columns(z, &m);
            let mut slopes = Vec::new();
            partial_fraction_slopes(&self.basis, z, &mut slopes);
            let d_prime: c64 = slopes.iter().zip(&c).map(|(a, b)| a * b).sum();
            for (e, res) in residues.iter_mut().enumerate() {
                let num: c64 = n.iter().zip(&self.numerator[e]).map(|(c, x)| c * x).sum();
                res.push(num / d_prime);
            }
        }
        let n_pf: usize = self.basis.iter().map(|p| p.unknowns()).sum::<usize>() * m.len();
        let constant = (0..n_resp)
            .map(|e| {
                let x = &self.numerator[e][n_pf..];
                match (self.constant, self.symmetry) {
                    (false, _) => c64::new(0.0, 0.0),
                    (true, Symmetry::Real) => {
                        c64::new(x.iter().zip(&m).map(|(a, b)| a * b.re).sum(), 0.0)
                    }
                    (true, Symmetry::Complex) => {
                        let (re, im) = x.split_at(m.len());
                        c64::new(
                            re.iter().zip(&m).map(|(a, b)| a * b.re).sum(),
                            im.iter().zip(&m).map(|(a, b)| a * b.re).sum(),
                        )
                    }
                }
            })
            .collect();
        Ok(Rational {
            poles,
            residues,
            constant,
            proportional: vec![c64::new(0.0, 0.0); n_resp],
            delays: vec![self.delay; n_resp],
            error: self.error,
            iterations: 0,
        })
    }

    /// The model at parameter `values` as a **stable** pole–residue model: D's zeros, those in
    /// the right half plane flipped into the left as the paper does (its Section 2), and the
    /// residues and constant identified again by the paper's Stage 2 against the model's own
    /// response at `points` wavelengths over the band, evenly spaced in frequency. Its error
    /// is that fit's, against the parametric model: what stability costs at these values.
    ///
    /// # Errors
    ///
    /// As [`ParametricModel::rational`], and for fewer points than the fit needs.
    pub fn stable_rational(&self, values: &[f64], points: usize) -> Result<Rational> {
        let m = self.weights_at(values)?;
        let poles = stable_poles(sigma_zeros(&self.basis, &self.sigma_at(&m))?, self.symmetry);
        if points < 2 {
            return Err(Error::invalid(
                "parametric model",
                "a refit needs at least two points",
            ));
        }
        let (f0, f1) = (1.0 / self.band.1.to_um(), 1.0 / self.band.0.to_um());
        let s: Vec<c64> = (0..points)
            .map(|k| {
                c64::new(
                    0.0,
                    -std::f64::consts::TAU * (f0 + (f1 - f0) * k as f64 / (points - 1) as f64),
                )
            })
            .collect();
        let n_resp = self.numerator.len();
        let data: Vec<Vec<c64>> = (0..n_resp)
            .map(|e| s.iter().map(|&z| self.evaluate(e, z, &m)).collect())
            .collect();
        let original: Vec<Vec<c64>> = data
            .iter()
            .map(|f| {
                f.iter()
                    .zip(&s)
                    .map(|(&v, &z)| v * (-z * self.delay).exp())
                    .collect()
            })
            .collect();
        let unknowns: usize = poles.iter().map(|p| p.unknowns()).sum::<usize>() + 2;
        if 2 * points < unknowns + 1 {
            return Err(Error::invalid(
                "parametric model",
                format!("{points} points can't determine {unknowns} unknowns per response"),
            ));
        }
        residues(
            &s,
            &data,
            &poles,
            &self.options(),
            &vec![self.delay; n_resp],
            &original,
            0,
        )
    }

    /// The model's error against `samples`, e.g. spectra at parameter values it wasn't fitted
    /// to.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values outside the sampled range, wavelengths outside the
    /// band, or a spectrum whose ports don't match the model's.
    pub fn error_against(&self, samples: &[Sample]) -> Result<FitError> {
        let mut data = Vec::with_capacity(samples.len());
        for smp in samples {
            if smp.spectrum.ports().len() != self.ports.len() {
                return Err(Error::invalid(
                    "parametric model",
                    "a sample's ports don't match the model's",
                ));
            }
            for &w in smp.spectrum.wavelengths() {
                check_band("parametric model", self.band, w)?;
            }
            data.push((responses_of(&smp.spectrum), self.weights_at(&smp.values)?));
        }
        let s: Vec<Vec<c64>> = samples
            .iter()
            .map(|smp| {
                smp.spectrum
                    .wavelengths()
                    .iter()
                    .map(|&w| laplace(w))
                    .collect()
            })
            .collect();
        Ok(error_over(&data, |i, k, e| {
            let z = s[i][k];
            self.evaluate(e, z, &data[i].1) * (-z * self.delay).exp()
        }))
    }

    /// The largest Re a/|a| of the model's poles over a grid of `points` values per parameter
    /// across the sampled range (at most 10⁴ points in all): negative when every pole is stable
    /// at those points. A sample, not a proof: see [`ParametricModel::uniform_stability`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than two points, or if D's zeros can't be found.
    pub fn stability(&self, points: usize) -> Result<f64> {
        if points < 2 {
            return Err(Error::invalid(
                "stability check",
                "needs at least two points",
            ));
        }
        let vars = self.parameters.len();
        let per = points.min((1e4f64.powf(1.0 / vars.max(1) as f64)).floor().max(2.0) as usize);
        let total = per.pow(vars as u32);
        let mut worst = f64::NEG_INFINITY;
        for k in 0..total {
            let mut idx = k;
            let values: Vec<f64> = (0..vars)
                .map(|d| {
                    let i = idx % per;
                    idx /= per;
                    let u = -1.0 + 2.0 * i as f64 / (per - 1) as f64;
                    (self.centre[d] + self.half[d] * u)
                        .clamp(self.centre[d] - self.half[d], self.centre[d] + self.half[d])
                })
                .collect();
            let m = self.weights_at(&values)?;
            for a in sigma_zeros(&self.basis, &self.sigma_at(&m))? {
                worst = worst.max(a.re / a.norm());
            }
        }
        Ok(worst)
    }

    /// Every point where a pole crosses into the right half plane, over the whole sampled range
    /// of a [`Interpolation::PiecewiseLinear`] model's one parameter: empty when the model is
    /// stable at every value, not only at sampled ones. The poles at each sample are checked
    /// first (a pole already unstable there is reported at the sample, with its ω).
    ///
    /// Between samples k and k + 1 the denominator's coefficients are c(t) = c_k + t δ, so the
    /// poles are the eigenvalues of H − t b δᵀ, H = A − b c_kᵀ: a rank-one change. A pole is on
    /// the axis at s = jω when 1 + t g(jω) = 0, g(s) = δᵀ(sI − H)⁻¹ b, so where g(jω) is real
    /// and at most −1, with t = −1/g. g is real on the axis where F(s) = g(s) − conj(g(−s̄))
    /// vanishes, a rational function realized on blkdiag(H, −H̄) whose zeros are the finite
    /// generalized eigenvalues of its system pencil: every candidate ω at once, without
    /// sampling. Each candidate's t, estimated from g, is pinned by bisection on the real part of
    /// the pole nearest the axis point, on both sides of the estimate, and kept only if that pole
    /// is on the axis there. Triverio et al.'s own test (their Theorem 2) solves linear matrix
    /// inequalities, which needs a semidefinite solver photonoxide doesn't have; this one is
    /// exact for one parameter in exact arithmetic, and in floating point as good as the poles:
    /// for narrow resonances the rewriting (their Theorem 1) is ill-conditioned, and a pole
    /// within about 1e-4 of the axis can't be placed on either side.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the model is piecewise linear in exactly one parameter,
    /// or if an eigenvalue problem fails.
    pub fn uniform_stability(&self) -> Result<Vec<Crossing>> {
        let Weights::Grid(axes) = &self.weights else {
            return Err(Error::invalid(
                "uniform stability",
                "decides piecewise-linear models only: a polynomial model's poles are sampled by stability()",
            ));
        };
        if axes.len() != 1 {
            return Err(Error::invalid(
                "uniform stability",
                "decides models of one parameter only",
            ));
        }
        let axis = &axes[0];
        let failed = |e: String| Error::invalid("uniform stability", e);
        let node = |k: usize| -> Vec<f64> {
            let m: Vec<c64> = (0..axis.len())
                .map(|l| c64::new(f64::from(u8::from(l == k)), 0.0))
                .collect();
            self.sigma_at(&m)
        };
        let mut crossings = Vec::new();
        for (k, &value) in axis.iter().enumerate() {
            for z in sigma_zeros(&self.basis, &node(k))? {
                if z.re >= 0.0 {
                    crossings.push(Crossing {
                        value,
                        omega: -z.im,
                    });
                }
            }
        }
        for k in 0..axis.len() - 1 {
            let (a, b, ck) = sigma_realization(&self.basis, &node(k));
            let (_, _, ck1) = sigma_realization(&self.basis, &node(k + 1));
            let n = b.len();
            let h = Mat::<c64>::from_fn(n, n, |i, j| a[(i, j)] - b[i] * ck[j]);
            let delta: Vec<c64> = ck1.iter().zip(&ck).map(|(x, y)| x - y).collect();
            // F(s) = δᵀ(sI − H)⁻¹b + δ̄ᵀ(sI + H̄)⁻¹b̄: its zeros, the finite eigenvalues of the
            // pencil ([[Â, B], [Cᵀ, 0]], [[I, 0], [0, 0]])
            let size = 2 * n + 1;
            let pencil_a = Mat::<c64>::from_fn(size, size, |i, j| match (i < 2 * n, j < 2 * n) {
                (true, true) => {
                    if i < n && j < n {
                        h[(i, j)]
                    } else if i >= n && j >= n {
                        -h[(i - n, j - n)].conj()
                    } else {
                        c64::new(0.0, 0.0)
                    }
                }
                (true, false) => {
                    if i < n {
                        b[i]
                    } else {
                        b[i - n].conj()
                    }
                }
                (false, true) => {
                    if j < n {
                        delta[j]
                    } else {
                        delta[j - n].conj()
                    }
                }
                (false, false) => c64::new(0.0, 0.0),
            });
            let pencil_b = Mat::<c64>::from_fn(size, size, |i, j| {
                c64::new(f64::from(u8::from(i == j && i < 2 * n)), 0.0)
            });
            let eig = pencil_a
                .generalized_eigen(&pencil_b)
                .map_err(|e| failed(format!("the axis crossings: {e:?}")))?;
            let (alpha, beta) = (eig.S_a(), eig.S_b());
            let scale = (0..n).map(|i| h[(i, i)].norm()).fold(1.0, f64::max);
            for i in 0..size {
                if beta[i].norm() <= 1e-12 * alpha[i].norm() {
                    continue; // infinite
                }
                let z = alpha[i] / beta[i];
                if z.re.abs() > 1e-4 * scale {
                    continue;
                }
                // t ≈ −1/g(z), where g is nearly real and negative: next to a nearly marginal
                // pole g turns fast, so this is only where to start looking
                let g =
                    {
                        use faer::linalg::solvers::Solve;
                        let x = Mat::<c64>::from_fn(n, n, |r, c| {
                            if r == c { z - h[(r, c)] } else { -h[(r, c)] }
                        })
                        .partial_piv_lu()
                        .solve(Mat::<c64>::from_fn(n, 1, |r, _| b[r]));
                        (0..n).map(|r| delta[r] * x[(r, 0)]).sum::<c64>()
                    };
                if !(g.re < 0.0 && g.re.is_finite()) {
                    continue;
                }
                let t0 = (-1.0 / g.re).clamp(0.0, 1.0);
                // pinned by bisection, on each side of the estimate: the real part of the
                // eigenvalue of H − t b δᵀ nearest the axis point must change sign
                let target = c64::new(0.0, z.im);
                let nearest = |t: f64| -> Result<c64> {
                    let m = Mat::<c64>::from_fn(n, n, |r, c| h[(r, c)] - t * b[r] * delta[c]);
                    let values = m
                        .eigenvalues()
                        .map_err(|e| failed(format!("the poles along a segment: {e:?}")))?;
                    values
                        .into_iter()
                        .min_by(|x, y| (x - target).norm().total_cmp(&(y - target).norm()))
                        .ok_or_else(|| failed("a segment without poles".into()))
                };
                let f0 = nearest(t0)?.re;
                for side in [-1.0, 1.0] {
                    let mut bracket = None;
                    for width in [1e-8, 1e-6, 1e-4, 1e-2, 1e-1] {
                        let t1 = (t0 + side * width).clamp(0.0, 1.0);
                        if t1 == t0 {
                            break;
                        }
                        if nearest(t1)?.re * f0 <= 0.0 {
                            bracket = Some((t0.min(t1), t0.max(t1)));
                            break;
                        }
                    }
                    let Some((mut lo, mut hi)) = bracket else {
                        continue;
                    };
                    let f_lo = nearest(lo)?.re;
                    for _ in 0..60 {
                        let mid = 0.5 * (lo + hi);
                        if nearest(mid)?.re * f_lo > 0.0 {
                            lo = mid;
                        } else {
                            hi = mid;
                        }
                    }
                    let t = 0.5 * (lo + hi);
                    let pole = nearest(t)?;
                    // a sign change of the nearest eigenvalue that isn't on the axis is two
                    // poles trading places, not a crossing
                    if pole.re.abs() > 1e-6 * pole.norm() {
                        continue;
                    }
                    let crossing = Crossing {
                        value: axis[k] + t * (axis[k + 1] - axis[k]),
                        omega: -pole.im,
                    };
                    // the same crossing found from two eigenvalues
                    let span = axis[k + 1] - axis[k];
                    if !crossings.iter().any(|c: &Crossing| {
                        (c.value - crossing.value).abs() <= 1e-9 * span
                            && (c.omega - crossing.omega).abs()
                                <= 1e-9 * crossing.omega.abs().max(1.0)
                    }) {
                        crossings.push(crossing);
                    }
                }
            }
        }
        crossings.sort_by(|x, y| x.value.total_cmp(&y.value));
        Ok(crossings)
    }
}

impl Component for ParametricModel {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check_band("parametric model", self.band, wavelength)?;
        let m = self.weights_at(values)?;
        let s = laplace(wavelength);
        let delay = (-s * self.delay).exp();
        let n = self.ports.len();
        Ok(SMatrix::from_fn(n, |q, p| {
            self.evaluate(q * n + p, s, &m) * delay
        }))
    }

    fn provenance(&self) -> Provenance {
        let dependence = match &self.weights {
            Weights::Monomials(exponents) => format!(
                "polynomial of degree {} in the parameters (Sanathanan and Koerner's iteration, IEEE Trans. Autom. Control 8, 56 (1963), doi:10.1109/TAC.1963.1105517)",
                exponents.iter().map(|e| e.iter().sum::<usize>()).max().unwrap_or(0)
            ),
            Weights::Grid(_) => "piecewise linear in the parameters (P. Triverio et al., IEEE Trans. Adv. Packag. 32, 205 (2009), doi:10.1109/TADVP.2008.2007913)".into(),
        };
        Provenance {
            fidelity: Fidelity::Compact,
            source: format!(
                "a rational model on {} basis poles (vector fitting, B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999), doi:10.1109/61.772353), numerator and denominator {dependence}, fitted to {}",
                expanded(&self.basis).len(),
                self.source
            ),
            error: Some(self.error.max),
            validity: Some((self.band.0.to_um(), self.band.1.to_um())),
        }
    }
}

#[cfg(test)]
mod tests;

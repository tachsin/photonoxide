//! Compact models over a component's parameters: rational in frequency, polynomial in the
//! parameters.
//!
//! The model is the paper's Stage-1 pair itself, (σf) and σ, with coefficients that are
//! polynomials in the parameters p:
//!
//! H_e(s, p) = N_e(s, p) / D(s, p),
//! N_e = Σₙ Σ_l c_enl ξ_l(p)/(s − aₙ) + Σ_l d_el ξ_l(p),
//! D = 1 + Σₙ Σ_l c̃_nl ξ_l(p)/(s − aₙ),
//!
//! with ξ_l the monomials of total degree at most D in the parameters, each scaled to [−1, 1]
//! over the samples' range. The basis poles aₙ are fixed: the common poles of every sample's
//! spectrum, fitted together by vector fitting ([`super::fit`]; the paper's vector formulation).
//! They cancel between N and D, and the model's poles at p are the zeros of D(·, p), the
//! eigenvalues of A − b c̃(p)ᵀ (B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052
//! (1999), doi:10.1109/61.772353, Appendix B). Their residues are N(z)/D′(z) there.
//!
//! N − f D = 0, written at every sample and frequency, is linear in the c, d and c̃: one least
//! squares, the responses' own unknowns eliminated block by block as in [`super::fit`]. It
//! weights each equation by |D|, small at resonances, so it is solved again with each equation
//! divided by the last solve's |D|, the iteration of C. K. Sanathanan and J. Koerner (IEEE
//! Trans. Autom. Control 8, 56 (1963), doi:10.1109/TAC.1963.1105517), here three times, keeping
//! the best. Parameterizing σf and σ this way, with a fixed basis, is the approach of P.
//! Triverio, S. Grivet-Talocia and M. S. Nakhla (IEEE Trans. Adv. Packag. 32, 205 (2009),
//! doi:10.1109/TADVP.2008.2007913); neither paper has been checked against yet.
//!
//! **Why a parametric denominator.** A resonance moves with a device's parameters, and with it
//! its pole. A shared set of poles with interpolated residues can't move a resonance by more
//! than its width: an all-pass ring whose resonances shift by 0.8 of a free spectral range over
//! the samples is fitted by 16 shared poles only to 1.3 (measured in the tests). Fitting each
//! sample on its own and interpolating its poles follows the resonances in the band, but the
//! poles a fit places outside the band, to stand for the resonances beyond it, change from
//! sample to sample and don't interpolate: off by 0.2 to 0.8 between samples on the same ring
//! (measured while choosing the method). Here the poles are the zeros of a denominator whose
//! coefficients are polynomials, so they move continuously with p: 5.8e-7 between samples on
//! that ring with degree 6 and 13 samples.
//!
//! The model's error is measured at the samples ([`ParametricModel::error`]) and should be
//! measured again at parameter values that weren't fitted ([`ParametricModel::error_against`]).
//! The poles' stability is checked over a grid of the parameters' range
//! ([`ParametricModel::stability`]); unlike a single fit's, they aren't flipped.

use num_complex::Complex64 as c64;

use super::fit::{
    self, Delay, FitError, Options, Pole, Rational, Symmetry, classify, eliminate, least_squares,
    partial_fraction_slopes, partial_fractions, polynomial_terms, real_rows, residues, sigma_zeros,
    solve_stacked, stable_poles,
};
use super::model::{band_of, check_band, laplace, responses_of};
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, Spectrum, ports};
use crate::units::Wavelength;
use crate::{Error, Result};

/// How many times the least squares is reweighted by 1/|D| (Sanathanan and Koerner).
const REWEIGHTINGS: usize = 3;

/// A spectrum at one point of a component's parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The parameters' values, in the order of the model's parameters.
    pub values: Vec<f64>,
    /// The spectrum there.
    pub spectrum: Spectrum,
}

/// A rational model in frequency whose numerator and denominator coefficients are polynomials
/// in the component's parameters (see the [module docs](self)): a [`Component`] of
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
    /// The monomials: each parameter's exponent.
    exponents: Vec<Vec<usize>>,
    symmetry: Symmetry,
    constant: bool,
    basis: Vec<Pole>,
    /// Response e's real unknowns: basis column j and monomial l at j L + l, then the constant
    /// term's columns, each times the monomials.
    numerator: Vec<Vec<f64>>,
    /// D's real unknowns, basis column j and monomial l at j L + l.
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

impl ParametricModel {
    /// Fits `samples`, spectra at values of `parameters` sharing their ports and wavelengths:
    /// `options` for the common basis poles (their number, symmetry, constant term and
    /// delay), and polynomials of total degree `degree` in the parameters (see the
    /// [module docs](self)).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a sample's values don't match the parameters or aren't
    /// allowed by them, if the spectra don't share their ports and wavelengths, if a parameter
    /// doesn't vary over the samples, if there are fewer samples than polynomial coefficients,
    /// for a proportional term or an estimated delay (a fixed one is allowed), or for the basis
    /// fit's own reasons ([`fit::vector_fit`]).
    pub fn fit(
        parameters: Vec<Parameter>,
        samples: &[Sample],
        options: &Options,
        degree: usize,
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
        let exponents = monomials(vars, degree);
        if samples.len() < exponents.len() {
            return bad(format!(
                "{} samples can't determine a polynomial of degree {degree} in {vars} parameters ({} coefficients)",
                samples.len(),
                exponents.len()
            ));
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
        let s: Vec<c64> = wavelengths.iter().map(|&w| laplace(w)).collect();
        // each sample's responses, the delay taken out, and its monomials
        let data: Vec<(Vec<Vec<c64>>, Vec<c64>)> = samples
            .iter()
            .map(|smp| {
                let x: Vec<f64> = smp
                    .values
                    .iter()
                    .zip(&centre)
                    .zip(&half)
                    .map(|((v, c), h)| (v - c) / h)
                    .collect();
                let m = evaluate_monomials(&exponents, &x);
                let responses = responses_of(&smp.spectrum)
                    .into_iter()
                    .map(|r| {
                        r.iter()
                            .zip(&s)
                            .map(|(&v, &z)| v * (z * delay).exp())
                            .collect()
                    })
                    .collect();
                (responses, m.into_iter().map(|v| c64::new(v, 0.0)).collect())
            })
            .collect();

        // the basis: every sample's spectrum fitted with common poles
        let all: Vec<Vec<c64>> = data.iter().flat_map(|(r, _)| r.iter().cloned()).collect();
        let shared = fit::vector_fit(
            &s,
            &all,
            &Options {
                delay: Delay::None,
                ..options.clone()
            },
        )?;
        let basis = classify(&shared.poles, options.symmetry)?;
        let mut model = ParametricModel {
            kind: "compact model".into(),
            ports: ports(
                &first
                    .spectrum
                    .ports()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            ),
            parameters,
            band: band_of(wavelengths),
            centre,
            half,
            exponents,
            symmetry: options.symmetry,
            constant: options.constant,
            basis,
            numerator: Vec::new(),
            denominator: Vec::new(),
            delay,
            error: shared.error,
            basis_error: shared.error,
            source: format!("spectra at {} parameter samples", samples.len()),
        };
        model.solve(&s, &data)?;
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

    /// The columns of N's and D's real unknowns at `s` and monomials `m`.
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

    /// N_e/D at `s`, monomials `m`, the delay not included.
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

    /// The least squares N − f D = 0, reweighted by 1/|D|, the best of the solves kept.
    fn solve(&mut self, s: &[c64], data: &[(Vec<Vec<c64>>, Vec<c64>)]) -> Result<()> {
        let n_resp = data[0].0.len();
        let k = s.len();
        let mut weights = vec![vec![1.0; k]; data.len()];
        let mut best: Option<(f64, Vec<Vec<f64>>, Vec<f64>)> = None;
        for _ in 0..=REWEIGHTINGS {
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
                let b = faer::Mat::from_fn(
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
            }
            // reweight by the last denominator
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

    /// The error of the common basis poles' own fit: every sample with the same poles and
    /// residues of their own, the model a shared-pole interpolation starts from.
    pub fn basis_error(&self) -> FitError {
        self.basis_error
    }

    /// The band.
    pub fn band(&self) -> (Wavelength, Wavelength) {
        self.band
    }

    /// The monomials at parameter `values`, checked against the sampled range.
    fn monomials_at(&self, values: &[f64]) -> Result<Vec<c64>> {
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
        Ok(evaluate_monomials(&self.exponents, &x)
            .into_iter()
            .map(|v| c64::new(v, 0.0))
            .collect())
    }

    /// D's coefficients at monomials `m`.
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
    /// samples cover (a polynomial says nothing outside it), or if D's zeros can't be found.
    pub fn rational(&self, values: &[f64]) -> Result<Rational> {
        let m = self.monomials_at(values)?;
        let poles = sigma_zeros(&self.basis, &self.sigma_at(&m))?;
        let n_resp = self.numerator.len();
        let mut residues = vec![Vec::with_capacity(poles.len()); n_resp];
        for &z in &poles {
            let (n, _) = self.columns(z, &m);
            let mut slopes = Vec::new();
            partial_fraction_slopes(&self.basis, z, &mut slopes);
            let c = self.sigma_at(&m);
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
        let m = self.monomials_at(values)?;
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
            data.push((responses_of(&smp.spectrum), self.monomials_at(&smp.values)?));
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
    /// there.
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
            let x: Vec<f64> = (0..vars)
                .map(|_| {
                    let i = idx % per;
                    idx /= per;
                    -1.0 + 2.0 * i as f64 / (per - 1) as f64
                })
                .collect();
            let m: Vec<c64> = evaluate_monomials(&self.exponents, &x)
                .into_iter()
                .map(|v| c64::new(v, 0.0))
                .collect();
            for a in sigma_zeros(&self.basis, &self.sigma_at(&m))? {
                worst = worst.max(a.re / a.norm());
            }
        }
        Ok(worst)
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
        let m = self.monomials_at(values)?;
        let s = laplace(wavelength);
        let delay = (-s * self.delay).exp();
        let n = self.ports.len();
        Ok(SMatrix::from_fn(n, |q, p| {
            self.evaluate(q * n + p, s, &m) * delay
        }))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Compact,
            source: format!(
                "a rational model on {} basis poles (vector fitting, B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999), doi:10.1109/61.772353), numerator and denominator polynomial of degree {} in the parameters, fitted to {}",
                self.basis
                    .iter()
                    .map(|p| if matches!(p, Pole::Pair(_)) { 2 } else { 1 })
                    .sum::<usize>(),
                self.exponents
                    .iter()
                    .map(|e| e.iter().sum::<usize>())
                    .max()
                    .unwrap_or(0),
                self.source
            ),
            error: Some(self.error.max),
            validity: Some((self.band.0.to_um(), self.band.1.to_um())),
        }
    }
}

#[cfg(test)]
mod tests;

//! A component's S-matrix spectrum as a rational function of frequency: vector fitting
//! ([`super::fit`]) applied to every element of S with common poles.
//!
//! A wavelength λ is the Laplace variable s = −iω with ω = 2π/λ, in rad/µm (ω/c: frequencies in
//! photonoxide are in units of c/µm, [`crate::units`]). Under photonoxide's e^(−iωt) a pole a
//! gives a field e^(at), so a stable pole has Re a < 0, and Im a = −ω_p, minus its resonance's
//! angular frequency: a resonance of quality factor Q at ω_p has a ≈ −ω_p/(2Q) − iω_p. A delay
//! τ (µm of light travel, n_g L for a guide) is e^(−sτ) = e^(iωτ), the phase of light that
//! travels.

use std::f64::consts::TAU;
use std::path::Path;

use num_complex::Complex64 as c64;

use super::fit::{self, FitError, Options, Rational};
use super::touchstone::{Convention, Touchstone};
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, Spectrum, ports};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The Laplace variable of a wavelength: s = −iω, ω = 2π/λ in rad/µm.
pub fn laplace(wavelength: Wavelength) -> c64 {
    c64::new(0.0, -TAU / wavelength.to_um())
}

/// The shortest and longest of `wavelengths` (not empty).
pub(crate) fn band_of(wavelengths: &[Wavelength]) -> (Wavelength, Wavelength) {
    wavelengths
        .iter()
        .fold((wavelengths[0], wavelengths[0]), |(a, b), &w| {
            (if w < a { w } else { a }, if w > b { w } else { b })
        })
}

/// `wavelength` is within the band, to round-off.
pub(crate) fn check_band(
    what: &'static str,
    (lo, hi): (Wavelength, Wavelength),
    wavelength: Wavelength,
) -> Result<()> {
    let (lo, hi, w) = (lo.to_um(), hi.to_um(), wavelength.to_um());
    let slack = 1e-12 * hi;
    if w < lo - slack || w > hi + slack {
        return Err(Error::invalid(
            what,
            format!("covers {lo} to {hi} um, not {w} um"),
        ));
    }
    Ok(())
}

/// The responses of a spectrum, response q n + p being S_qp over the wavelengths.
pub(crate) fn responses_of(spectrum: &Spectrum) -> Vec<Vec<c64>> {
    let n = spectrum.ports().len();
    (0..n * n).map(|e| spectrum.element(e / n, e % n)).collect()
}

/// A fitted S-matrix model: S_qp(λ) a rational function of frequency, every element with the
/// same poles, valid over the band it was fitted on. A [`Component`] of
/// [`Fidelity::Compact`], whose error is the fit's largest |ΔS_qp|.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactModel {
    kind: String,
    ports: Vec<Port>,
    /// Responses q n + p: S_qp.
    rational: Rational,
    band: (Wavelength, Wavelength),
    source: String,
}

/// Where a model's S-matrix isn't passive: the largest singular value of S over a grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Passivity {
    /// The largest singular value of S found, and the wavelength it is at.
    pub largest: (Wavelength, f64),
    /// Every grid point where it exceeds 1: (wavelength, singular value).
    pub violations: Vec<(Wavelength, f64)>,
    /// The number of wavelengths checked.
    pub points: usize,
}

impl Passivity {
    /// Whether S is passive at every point checked, its singular values at most 1 + `tolerance`.
    pub fn passive(&self, tolerance: f64) -> bool {
        self.largest.1 <= 1.0 + tolerance
    }
}

/// The largest singular value of `s(λ)` at `points` wavelengths evenly spaced in frequency over
/// `band`, and where it exceeds 1.
pub(crate) fn passivity_of(
    band: (Wavelength, Wavelength),
    points: usize,
    s: impl Fn(Wavelength) -> Result<SMatrix>,
) -> Result<Passivity> {
    if points < 2 {
        return Err(Error::invalid(
            "passivity check",
            "needs at least two points",
        ));
    }
    let (f0, f1) = (1.0 / band.1.to_um(), 1.0 / band.0.to_um());
    let mut largest = (band.0, 0.0);
    let mut violations = Vec::new();
    for k in 0..points {
        let f = f0 + (f1 - f0) * k as f64 / (points - 1) as f64;
        let w = Wavelength::um(1.0 / f)?;
        let top = s(w)?.largest_singular_value()?;
        if top > largest.1 {
            largest = (w, top);
        }
        if top > 1.0 {
            violations.push((w, top));
        }
    }
    Ok(Passivity {
        largest,
        violations,
        points,
    })
}

impl CompactModel {
    /// Fits `spectrum` (photonoxide's e^(−iωt) convention, at two wavelengths or more).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than two wavelengths or no ports, or the fit's own
    /// reasons ([`fit::vector_fit`]).
    pub fn fit(spectrum: &Spectrum, options: &Options) -> Result<CompactModel> {
        let w = spectrum.wavelengths();
        if w.len() < 2 || spectrum.ports().is_empty() {
            return Err(Error::invalid(
                "compact model",
                format!(
                    "needs a spectrum of at least two wavelengths and a port, not {} and {}",
                    w.len(),
                    spectrum.ports().len()
                ),
            ));
        }
        let s: Vec<c64> = w.iter().map(|&w| laplace(w)).collect();
        let rational = fit::vector_fit(&s, &responses_of(spectrum), options)?;
        let names: Vec<&str> = spectrum.ports().iter().map(String::as_str).collect();
        Ok(CompactModel {
            kind: "compact model".into(),
            ports: ports(&names),
            band: band_of(w),
            source: format!("a spectrum at {} wavelengths", w.len()),
            rational,
        })
    }

    /// Fits the S-parameters of a Touchstone file in `convention` (see
    /// [`Touchstone::spectrum`]).
    ///
    /// # Errors
    ///
    /// As [`Touchstone::spectrum`] and [`CompactModel::fit`].
    pub fn from_touchstone(
        file: &Touchstone,
        convention: Convention,
        options: &Options,
    ) -> Result<CompactModel> {
        CompactModel::fit(&file.spectrum(convention)?, options)
    }

    /// The same model, its kind named `kind` (e.g. `"ring"`).
    #[must_use]
    pub fn with_kind(mut self, kind: impl Into<String>) -> CompactModel {
        self.kind = kind.into();
        self
    }

    /// The same model, saying what its spectrum came from (e.g. a solver and its grid, or a
    /// file), for its [`Provenance`].
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> CompactModel {
        self.source = source.into();
        self
    }

    /// The fitted rational function: response q n + p is S_qp.
    pub fn rational(&self) -> &Rational {
        &self.rational
    }

    /// The fit's error against the spectrum it was fitted to.
    pub fn error(&self) -> FitError {
        self.rational.error
    }

    /// The band it was fitted on: the shortest and longest wavelengths.
    pub fn band(&self) -> (Wavelength, Wavelength) {
        self.band
    }

    /// S at `wavelength`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] outside the band the model was fitted on: a rational fit says
    /// nothing outside it.
    pub fn at(&self, wavelength: Wavelength) -> Result<SMatrix> {
        check_band("compact model", self.band, wavelength)?;
        let s = laplace(wavelength);
        let n = self.ports.len();
        Ok(SMatrix::from_fn(n, |q, p| {
            self.rational.evaluate(q * n + p, s)
        }))
    }

    /// The largest singular value of S at `points` wavelengths evenly spaced in frequency over
    /// the band, and every point where it exceeds 1: a passive device's S never does. The
    /// check doesn't enforce passivity; a model that fails it needs more poles or samples.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than two points, or if a singular value decomposition
    /// fails.
    pub fn passivity(&self, points: usize) -> Result<Passivity> {
        passivity_of(self.band, points, |w| self.at(w))
    }

    /// Writes the model's spectrum at `points` wavelengths over its band, evenly spaced in
    /// frequency, as a Touchstone file in `convention`.
    ///
    /// # Errors
    ///
    /// As [`Touchstone::from_spectrum`], and for fewer than two points.
    pub fn to_touchstone(&self, points: usize, convention: Convention) -> Result<Touchstone> {
        if points < 2 {
            return Err(Error::invalid(
                "compact model",
                "a spectrum needs at least two points",
            ));
        }
        let (f0, f1) = (1.0 / self.band.1.to_um(), 1.0 / self.band.0.to_um());
        let wavelengths = (0..points)
            .map(|k| Wavelength::um(1.0 / (f0 + (f1 - f0) * k as f64 / (points - 1) as f64)))
            .collect::<Result<Vec<_>>>()?;
        let spectrum = Spectrum::of(self, &wavelengths, &[])?;
        Touchstone::from_spectrum(&spectrum, convention)
    }
}

impl Component for CompactModel {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &[]
    }

    fn s_matrix(&self, wavelength: Wavelength, _: &[f64]) -> Result<SMatrix> {
        self.at(wavelength)
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Compact,
            source: format!(
                "{} poles fitted by vector fitting (B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999), doi:10.1109/61.772353) to {}",
                self.rational.poles.len(),
                self.source
            ),
            error: Some(self.rational.error.max),
            validity: Some((self.band.0.to_um(), self.band.1.to_um())),
        }
    }
}

/// A measured (or any tabulated) S-matrix spectrum as a [`Component`] of
/// [`Fidelity::Measured`]: S between the samples interpolated linearly in frequency, element by
/// element, and an error outside the band. Linear interpolation needs samples dense against
/// the spectrum's features; a resonant device's measurement is better fitted
/// ([`CompactModel::fit`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    kind: String,
    ports: Vec<Port>,
    /// Frequencies (c/µm), increasing, and S at each.
    frequencies: Vec<f64>,
    matrices: Vec<SMatrix>,
    band: (Wavelength, Wavelength),
    source: String,
}

impl Measured {
    /// A measured spectrum, from `source` (e.g. a file and its instrument).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than two wavelengths, or two at the same wavelength.
    pub fn new(spectrum: &Spectrum, source: impl Into<String>) -> Result<Measured> {
        let w = spectrum.wavelengths();
        let mut order: Vec<usize> = (0..w.len()).collect();
        order.sort_by(|&a, &b| w[b].to_um().total_cmp(&w[a].to_um()));
        let frequencies: Vec<f64> = order.iter().map(|&k| 1.0 / w[k].to_um()).collect();
        if frequencies.len() < 2 || frequencies.windows(2).any(|f| f[1] <= f[0]) {
            return Err(Error::invalid(
                "measured spectrum",
                "needs at least two wavelengths, all different",
            ));
        }
        let names: Vec<&str> = spectrum.ports().iter().map(String::as_str).collect();
        Ok(Measured {
            kind: "measured".into(),
            ports: ports(&names),
            frequencies,
            matrices: order
                .iter()
                .map(|&k| spectrum.matrices()[k].clone())
                .collect(),
            band: band_of(w),
            source: source.into(),
        })
    }

    /// The S-parameters of a Touchstone file in `convention`, its path the source.
    ///
    /// # Errors
    ///
    /// As [`Touchstone::read`], [`Touchstone::spectrum`] and [`Measured::new`].
    pub fn read(path: &Path, convention: Convention) -> Result<Measured> {
        let file = Touchstone::read(path)?;
        Measured::new(&file.spectrum(convention)?, path.display().to_string())
    }

    /// The same component, its kind named `kind`.
    #[must_use]
    pub fn with_kind(mut self, kind: impl Into<String>) -> Measured {
        self.kind = kind.into();
        self
    }

    /// The band: the shortest and longest wavelengths.
    pub fn band(&self) -> (Wavelength, Wavelength) {
        self.band
    }
}

impl Component for Measured {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &[]
    }

    fn s_matrix(&self, wavelength: Wavelength, _: &[f64]) -> Result<SMatrix> {
        check_band("measured spectrum", self.band(), wavelength)?;
        let f = (1.0 / wavelength.to_um()).clamp(
            self.frequencies[0],
            self.frequencies[self.frequencies.len() - 1],
        );
        let k = self
            .frequencies
            .partition_point(|&g| g <= f)
            .clamp(1, self.frequencies.len() - 1);
        let (f0, f1) = (self.frequencies[k - 1], self.frequencies[k]);
        let t = (f - f0) / (f1 - f0);
        let (a, b) = (&self.matrices[k - 1], &self.matrices[k]);
        Ok(SMatrix::from_fn(self.ports.len(), |q, p| {
            a[(q, p)] * (1.0 - t) + b[(q, p)] * t
        }))
    }

    fn provenance(&self) -> Provenance {
        let (lo, hi) = self.band();
        Provenance {
            fidelity: Fidelity::Measured,
            source: format!(
                "{} ({} wavelengths, linear in frequency between them)",
                self.source,
                self.frequencies.len()
            ),
            error: None,
            validity: Some((lo.to_um(), hi.to_um())),
        }
    }
}

#[cfg(test)]
mod tests;

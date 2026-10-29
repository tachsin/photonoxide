//! A component from a sampled spectrum, e.g. a 2D FDFD run, interpolated in wavelength.

use std::f64::consts::{PI, TAU};

use num_complex::Complex64 as c64;

use crate::circuit::{Component, Parameter, Port, Provenance, SMatrix, Spectrum};
use crate::units::Wavelength;
use crate::{Error, Result};

/// A component whose S-matrices were sampled at wavelengths (by a solver, or measured) and are
/// interpolated between them: each S_qp linearly in its magnitude and in its phase, the phase
/// unwrapped between neighbouring samples (taken to turn by less than half a turn between
/// them, so the sampling must resolve the device's delays and resonances). No parameters.
///
/// Outside the sampled wavelengths it refuses. Where a smooth model is wanted, fit the samples
/// instead: [`crate::compact::CompactModel::fit`] of [`Sampled::spectrum`], a rational model in
/// wavelength by vector fitting, with its fit error.
#[derive(Clone, Debug, PartialEq)]
pub struct Sampled {
    kind: String,
    ports: Vec<Port>,
    spectrum: Spectrum,
    provenance: Provenance,
}

impl Sampled {
    /// A component of kind `kind` with `ports`, interpolating `spectrum`; `provenance` says
    /// where the samples come from (its validity is set to the sampled range).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no samples, wavelengths that don't increase, or ports that
    /// don't match the spectrum's.
    pub fn new(
        kind: impl Into<String>,
        ports: Vec<Port>,
        spectrum: Spectrum,
        mut provenance: Provenance,
    ) -> Result<Sampled> {
        let w = spectrum.wavelengths();
        if w.is_empty() {
            return Err(Error::invalid(
                "sampled component",
                "needs at least one sample",
            ));
        }
        if w.windows(2).any(|p| p[1].to_um() <= p[0].to_um()) {
            return Err(Error::invalid(
                "sampled component",
                "the wavelengths must increase",
            ));
        }
        if ports.len() != spectrum.ports().len() {
            return Err(Error::invalid(
                "sampled component",
                format!(
                    "{} ports for a spectrum of {}",
                    ports.len(),
                    spectrum.ports().len()
                ),
            ));
        }
        provenance.validity = Some((w[0].to_um(), w[w.len() - 1].to_um()));
        Ok(Sampled {
            kind: kind.into(),
            ports,
            spectrum,
            provenance,
        })
    }

    /// The samples.
    pub fn spectrum(&self) -> &Spectrum {
        &self.spectrum
    }
}

/// Interpolates between `a` (at t = 0) and `b` (at t = 1): the magnitude linearly, the phase
/// along the shorter way round.
fn interpolate(a: c64, b: c64, t: f64) -> c64 {
    let (ma, pa) = a.to_polar();
    let (mb, pb) = b.to_polar();
    let mut turn = (pb - pa) % TAU;
    if turn > PI {
        turn -= TAU;
    } else if turn < -PI {
        turn += TAU;
    }
    c64::from_polar(ma + t * (mb - ma), pa + t * turn)
}

impl Component for Sampled {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &[]
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        if !values.is_empty() {
            return Err(Error::invalid(
                "component",
                format!("a {} takes no parameters, got {}", self.kind, values.len()),
            ));
        }
        let w: Vec<f64> = self
            .spectrum
            .wavelengths()
            .iter()
            .map(|w| w.to_um())
            .collect();
        let x = wavelength.to_um();
        let (lo, hi) = (w[0], w[w.len() - 1]);
        if !(lo..=hi).contains(&x) {
            return Err(Error::invalid(
                "component",
                format!(
                    "a {} is sampled from {lo} to {hi} um, not at {x} um",
                    self.kind
                ),
            ));
        }
        let m = self.spectrum.matrices();
        let k = w.partition_point(|&v| v <= x).clamp(1, w.len().max(2) - 1);
        if w.len() == 1 || w[k - 1] == x {
            return Ok(m[k - 1].clone());
        }
        let t = (x - w[k - 1]) / (w[k] - w[k - 1]);
        let (a, b) = (&m[k - 1], &m[k]);
        Ok(SMatrix::from_fn(a.size(), |q, p| {
            interpolate(a[(q, p)], b[(q, p)], t)
        }))
    }

    fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }

    fn derivatives(&self, _: Wavelength, _: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        Ok(Some(Vec::new()))
    }
}

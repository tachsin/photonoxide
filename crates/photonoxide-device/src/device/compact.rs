//! A device's compact fidelity: a [`ParametricModel`] fitted to the device's own runs at a
//! source fidelity (2D or 3D, or a closed form), over some of its parameters, and the device
//! offering it as [`Fidelity::Compact`] at the source's accuracy.
//!
//! - [`FitPlan`]: the parameters to cover and their ranges ([`FitAxis`]), the samples (a grid,
//!   or a Latin hypercube, [`Sampling`]), how many samples are kept out of the fit to measure
//!   its error, and the model's options: its poles and how it depends on the parameters
//!   ([`Interpolation`]).
//! - [`Fitted`]: the model with what it was fitted from (the device, its design, the source
//!   fidelity and its grid, the values of the parameters it doesn't cover) and how good it is:
//!   its largest |ΔS| at its samples and at the held-out ones, its poles' stability and its
//!   passivity, sampled over its range. It is a file (JSON, [`Fitted::to_json`]), saved next to
//!   the design; a design names it in its `[device]` table, `compact = "ring.compact.json"`.
//! - A design with a compact model has one more fidelity, [`Fidelity::Compact`], whose
//!   [`super::Method`] names its source fidelity and states its held-out error against it.
//!   Its component is the model, its parameters the ones it covers, with exact derivatives in
//!   them; the parameters it doesn't cover must keep the values it was fitted at.
//!
//! The samples are run as a device's study is ([`crate::job`], `kind = "fit"`): each point
//! the design with the covered parameters set, computed at the source fidelity, on rayon's
//! threads or a farm's workers. docs/methods/devices.md describes the device fits.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::{Built, Device, Figure, Geometry, Method, Range, Stated, Values};
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix};
use crate::compact::{Interpolation, Options, ParametricModel, Sample};
use crate::stack::LayerStack;
use crate::units::Wavelength;
use crate::{Error, Result};

/// The format of a [`Fitted`] file this photonoxide writes, and the only one it reads.
const FORMAT: u32 = 1;

/// The most parameters a compact model covers.
pub const MAX_PARAMETERS: usize = 4;

/// The most samples a fit takes, held-out ones included.
pub const MAX_SAMPLES: usize = 10_000;

/// How a fit's samples are placed over its parameters' ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Sampling {
    /// Every combination of each axis's points, evenly spaced from its `from` to its `to`.
    Grid,
    /// `points` samples on a Latin hypercube (M. D. McKay, R. J. Beckman, W. J. Conover,
    /// Technometrics 21, 239 (1979), doi:10.1080/00401706.1979.10489755): each parameter takes
    /// `points` levels evenly spaced from its `from` to its `to`, each level once, in an order
    /// drawn from `seed`. The ends are levels, so the samples span the range the model covers.
    LatinHypercube {
        /// How many samples.
        points: usize,
        /// The seed of the levels' orders.
        seed: u64,
    },
}

/// A parameter a compact model covers: one of the device's own, from one value to another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FitAxis {
    /// The device's parameter ([`Device::parameters`]).
    pub parameter: String,
    /// The smallest value, in the parameter's unit.
    pub from: f64,
    /// The largest.
    pub to: f64,
    /// A grid's points along it ([`Sampling::Grid`]); a Latin hypercube's are its own.
    pub points: usize,
}

impl FitAxis {
    /// `parameter` from `from` to `to`, `points` along it on a grid.
    pub fn new(parameter: &str, from: f64, to: f64, points: usize) -> FitAxis {
        FitAxis {
            parameter: parameter.to_owned(),
            from,
            to,
            points,
        }
    }
}

/// What a device's compact model covers and how it is fitted.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FitPlan {
    /// The parameters it covers, one to [`MAX_PARAMETERS`], each with its range.
    pub axes: Vec<FitAxis>,
    /// Where the samples are.
    pub sampling: Sampling,
    /// How many samples are kept out of the fit to measure its error: on a Latin hypercube of
    /// their own, each level in the middle of a stratum, so inside the range and between the
    /// fit's samples.
    pub held_out: usize,
    /// The basis poles of the rational model ([`Options::poles`]).
    pub poles: usize,
    /// How its coefficients depend on the parameters.
    pub interpolation: Interpolation,
}

impl FitPlan {
    /// The parameters `axes` on a grid, 8 held out (on a Latin hypercube of seed 1), 12 poles,
    /// and a polynomial of degree 2.
    pub fn new(axes: Vec<FitAxis>) -> FitPlan {
        FitPlan {
            axes,
            sampling: Sampling::Grid,
            held_out: 8,
            poles: 12,
            interpolation: Interpolation::Polynomial { degree: 2 },
        }
    }

    /// The same plan, its samples placed by `sampling`.
    #[must_use]
    pub fn sampled(self, sampling: Sampling) -> FitPlan {
        FitPlan { sampling, ..self }
    }

    /// The same plan with `held_out` samples kept out of the fit.
    #[must_use]
    pub fn holding_out(self, held_out: usize) -> FitPlan {
        FitPlan { held_out, ..self }
    }

    /// The same plan with `poles` basis poles.
    #[must_use]
    pub fn with_poles(self, poles: usize) -> FitPlan {
        FitPlan { poles, ..self }
    }

    /// The same plan, its coefficients depending on the parameters by `interpolation`.
    #[must_use]
    pub fn interpolated(self, interpolation: Interpolation) -> FitPlan {
        FitPlan {
            interpolation,
            ..self
        }
    }

    /// The seed the held-out samples are drawn from: the Latin hypercube's own plus one, or 1.
    fn held_out_seed(&self) -> u64 {
        match self.sampling {
            Sampling::LatinHypercube { seed, .. } => seed.wrapping_add(1),
            Sampling::Grid => 1,
        }
    }

    /// The samples the model is fitted to, each the covered parameters' values in the axes'
    /// order: a grid's with the last axis fastest.
    pub fn samples(&self) -> Vec<Vec<f64>> {
        let at = |a: &FitAxis, u: f64| a.from + (a.to - a.from) * u;
        match self.sampling {
            Sampling::Grid => {
                let mut out: Vec<Vec<f64>> = vec![Vec::new()];
                for a in &self.axes {
                    let n = a.points.max(2);
                    out = out
                        .into_iter()
                        .flat_map(|p| {
                            (0..n).map(move |k| {
                                let mut q = p.clone();
                                q.push(at(a, k as f64 / (n - 1) as f64));
                                q
                            })
                        })
                        .collect();
                }
                out
            }
            Sampling::LatinHypercube { points, seed } => {
                let n = points.max(2);
                latin_hypercube(n, self.axes.len(), seed, |k| k as f64 / (n - 1) as f64)
                    .into_iter()
                    .map(|u| self.axes.iter().zip(u).map(|(a, u)| at(a, u)).collect())
                    .collect()
            }
        }
    }

    /// The held-out samples: a Latin hypercube of [`FitPlan::held_out`] levels in the middle of
    /// their strata, `(k + ½)/n` of each range.
    pub fn held_out_samples(&self) -> Vec<Vec<f64>> {
        let n = self.held_out;
        if n == 0 {
            return Vec::new();
        }
        latin_hypercube(n, self.axes.len(), self.held_out_seed(), |k| {
            (k as f64 + 0.5) / n as f64
        })
        .into_iter()
        .map(|u| {
            self.axes
                .iter()
                .zip(u)
                .map(|(a, u)| a.from + (a.to - a.from) * u)
                .collect()
        })
        .collect()
    }

    /// What it samples, in words: "a 5 × 3 grid" or "25 points of a Latin hypercube (seed 1)".
    pub fn describe_sampling(&self) -> String {
        match self.sampling {
            Sampling::Grid => format!(
                "a {} grid",
                self.axes
                    .iter()
                    .map(|a| a.points.to_string())
                    .collect::<Vec<_>>()
                    .join(" × ")
            ),
            Sampling::LatinHypercube { points, seed } => {
                format!("{points} points of a Latin hypercube (seed {seed})")
            }
        }
    }

    /// Refuses a plan that isn't one for `device` with `source` as its source fidelity: no axis
    /// or more than [`MAX_PARAMETERS`], a parameter that isn't the device's or is covered
    /// twice, a range that isn't the parameter's or has no width, a grid with fewer than 2
    /// points on an axis, a Latin hypercube of fewer than 2 points, more than [`MAX_SAMPLES`]
    /// samples, no held-out sample, fewer samples than the polynomial's coefficients, a
    /// piecewise-linear model on anything but a grid, no poles, or a source fidelity the
    /// device hasn't or that is the compact one.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] then, saying which.
    pub fn check(&self, device: &dyn Device, source: Fidelity) -> Result<()> {
        let bad = |reason: String| Err(Error::invalid("compact model", reason));
        if source == Fidelity::Compact || !device.fidelities().iter().any(|m| m.fidelity == source)
        {
            let have: Vec<String> = device
                .fidelities()
                .iter()
                .filter(|m| m.fidelity != Fidelity::Compact)
                .map(|m| m.fidelity.to_string())
                .collect();
            return bad(format!(
                "a {}'s compact model is fitted to one of its own models ({}), not its {source} one",
                device.kind(),
                have.join(", ")
            ));
        }
        if self.axes.is_empty() || self.axes.len() > MAX_PARAMETERS {
            return bad(format!(
                "covers 1 to {MAX_PARAMETERS} parameters, not {}",
                self.axes.len()
            ));
        }
        for (k, a) in self.axes.iter().enumerate() {
            let Some(p) = device.parameters().iter().find(|p| p.name == a.parameter) else {
                let names: Vec<&str> = device
                    .parameters()
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect();
                return bad(format!(
                    "a {} has no parameter {}: {}",
                    device.kind(),
                    a.parameter,
                    names.join(", ")
                ));
            };
            if self.axes[..k].iter().any(|b| b.parameter == a.parameter) {
                return bad(format!("covers {} twice", a.parameter));
            }
            if !(a.from.is_finite() && a.to.is_finite() && a.from < a.to) {
                return bad(format!(
                    "{} needs a range from a smaller to a larger value, got {} to {}",
                    a.parameter, a.from, a.to
                ));
            }
            if !(p.allows(a.from) && p.allows(a.to)) {
                return bad(format!(
                    "a {}'s {} must be from {} to {} {}, got {} to {}",
                    device.kind(),
                    a.parameter,
                    p.min,
                    p.max,
                    p.unit,
                    a.from,
                    a.to
                ));
            }
            if self.sampling == Sampling::Grid && a.points < 2 {
                return bad(format!(
                    "a grid takes 2 points or more along {}, got {}",
                    a.parameter, a.points
                ));
            }
        }
        let samples = match self.sampling {
            Sampling::Grid => self
                .axes
                .iter()
                .map(|a| a.points)
                .try_fold(1usize, usize::checked_mul)
                .unwrap_or(usize::MAX),
            Sampling::LatinHypercube { points, .. } => {
                if points < 2 {
                    return bad(format!(
                        "a Latin hypercube takes 2 points or more, got {points}"
                    ));
                }
                points
            }
        };
        if samples.saturating_add(self.held_out) > MAX_SAMPLES {
            return bad(format!(
                "takes at most {MAX_SAMPLES} samples, held-out ones included, got {}",
                samples.saturating_add(self.held_out)
            ));
        }
        if self.held_out == 0 {
            return bad(
                "keeps at least one sample out of the fit, to measure its error where it wasn't \
                 fitted"
                    .into(),
            );
        }
        if self.poles == 0 {
            return bad("needs poles".into());
        }
        match self.interpolation {
            Interpolation::Polynomial { degree } => {
                let coefficients = binomial(self.axes.len() + degree, degree);
                if samples < coefficients {
                    return bad(format!(
                        "{samples} samples can't determine a polynomial of degree {degree} in {} \
                         parameters ({coefficients} coefficients)",
                        self.axes.len()
                    ));
                }
            }
            Interpolation::PiecewiseLinear => {
                if self.sampling != Sampling::Grid {
                    return bad(
                        "a piecewise-linear model is fitted on a grid, not a Latin hypercube"
                            .into(),
                    );
                }
            }
        }
        Ok(())
    }
}

/// n choose k, saturating.
fn binomial(n: usize, k: usize) -> usize {
    let mut c: usize = 1;
    for i in 0..k {
        c = c.saturating_mul(n - i) / (i + 1);
    }
    c
}

/// SplitMix64 (G. L. Steele, D. Lea, C. H. Flood, OOPSLA 2014, doi:10.1145/2660193.2660195):
/// the same numbers from the same seed on every system.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number from 0 to `n` − 1, by rejection, so every one is as likely.
    fn below(&mut self, n: usize) -> usize {
        let n = n as u64;
        let zone = u64::MAX - u64::MAX % n;
        loop {
            let x = self.next();
            if x < zone {
                return (x % n) as usize;
            }
        }
    }
}

/// `n` points of a Latin hypercube in `dims` dimensions: in each, the levels `level(0)` to
/// `level(n − 1)` each once, in an order of their own drawn from `seed` (Fisher–Yates).
fn latin_hypercube(
    n: usize,
    dims: usize,
    seed: u64,
    level: impl Fn(usize) -> f64,
) -> Vec<Vec<f64>> {
    let mut rng = SplitMix(seed);
    let orders: Vec<Vec<usize>> = (0..dims)
        .map(|_| {
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            order
        })
        .collect();
    (0..n)
        .map(|k| orders.iter().map(|o| level(o[k])).collect())
        .collect()
}

/// A device's compact model, fitted to its own runs, with what it was fitted from and how good
/// it is: the content of the file saved next to its design ([`Fitted::to_json`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Fitted {
    /// The file's format.
    pub format: u32,
    /// The device's kind, e.g. `"all-pass ring"`.
    pub kind: String,
    /// The design it was fitted from, as its `[device]` table (TOML), at the source fidelity.
    pub design: String,
    /// The fidelity of the runs it was fitted to: `"analytic"`, `"2D"` or `"3D"`.
    pub source: String,
    /// The grid of those runs, as their figures name it.
    pub grid: String,
    /// The parameters it covers, in the model's order.
    pub parameters: Vec<String>,
    /// Each one's unit.
    pub units: Vec<String>,
    /// Each one's range, (smallest, largest).
    pub ranges: Vec<(f64, f64)>,
    /// The device's other parameters and the values they were fixed at.
    pub fixed: Vec<(String, f64)>,
    /// The band, µm: (shortest, longest).
    pub band_um: (f64, f64),
    /// How the samples were placed, in words.
    pub sampling: String,
    /// How many samples it was fitted to.
    pub samples: usize,
    /// How many were held out.
    pub held_out: usize,
    /// Its largest |ΔS_qp| at its samples.
    pub fitted_error: f64,
    /// Its largest |ΔS_qp| at the held-out samples: its error where it wasn't fitted.
    pub held_out_error: f64,
    /// The largest Re a/|a| of its poles, sampled over its range: negative when every pole
    /// found is stable ([`ParametricModel::stability`]).
    pub stability: f64,
    /// The largest singular value of its S, sampled over its band and range: at most 1 for a
    /// passive model ([`ParametricModel::passivity`]).
    pub passivity: f64,
    /// The model.
    pub model: ParametricModel,
}

impl Fitted {
    /// Fits `spec`'s device as `plan` says: `samples` at [`FitPlan::samples`]'s values and
    /// `held` at [`FitPlan::held_out_samples`]', each the source fidelity's spectrum (`spec`'s
    /// fidelity) on `grid`, its values those of the covered parameters. Its stability and
    /// passivity are sampled at 21 values per parameter (at most about 10⁴ in all) and, for the
    /// passivity, 201 wavelengths.
    ///
    /// # Errors
    ///
    /// The errors of [`FitPlan::check`] and of [`super::Spec::build`], [`Error::InvalidValue`]
    /// when the samples aren't the plan's, and the fit's ([`ParametricModel::fit`]).
    pub fn fit(
        spec: &super::Spec,
        plan: &FitPlan,
        grid: &str,
        samples: &[Sample],
        held: &[Sample],
    ) -> Result<Fitted> {
        let (device, design) = spec.build()?;
        plan.check(device.as_ref(), spec.fidelity)?;
        if samples.len() != plan.samples().len() || held.len() != plan.held_out {
            return Err(Error::invalid(
                "compact model",
                format!(
                    "the plan takes {} samples and {} held out, got {} and {}",
                    plan.samples().len(),
                    plan.held_out,
                    samples.len(),
                    held.len()
                ),
            ));
        }
        let covered: Vec<&Parameter> = plan
            .axes
            .iter()
            .map(|a| {
                device
                    .parameters()
                    .iter()
                    .find(|p| p.name == a.parameter)
                    .unwrap_or_else(|| unreachable!("checked: a device parameter"))
            })
            .collect();
        let parameters: Vec<Parameter> = covered
            .iter()
            .zip(&plan.axes)
            .map(|(p, a)| {
                let default = design.values().get(&p.name).unwrap_or(a.from);
                Parameter::new(
                    p.name.clone(),
                    p.unit.clone(),
                    default.clamp(a.from, a.to),
                    a.from,
                    a.to,
                )
            })
            .collect();
        let options = Options::new(plan.poles);
        let model = ParametricModel::fit(parameters, samples, &options, plan.interpolation)?
            .with_kind(device.kind())
            .with_source(format!(
                "the {} model's spectra ({grid}) at {} samples, {}",
                spec.fidelity,
                samples.len(),
                plan.describe_sampling()
            ));
        let held_out_error = model.error_against(held)?.max;
        let stability = model.stability(21)?;
        let passivity = model.passivity(201, 21)?.largest;
        let band = model.band();
        let fixed = device
            .parameters()
            .iter()
            .filter(|p| !plan.axes.iter().any(|a| a.parameter == p.name))
            .map(|p| {
                (
                    p.name.clone(),
                    design.values().get(&p.name).unwrap_or(p.default),
                )
            })
            .collect();
        let mut nominal = spec.clone();
        nominal.compact = None;
        let design_text = toml::to_string(&nominal.to_table()).map_err(|e| Error::Parse {
            what: "compact model".into(),
            reason: e.to_string(),
        })?;
        Ok(Fitted {
            format: FORMAT,
            kind: device.kind().to_owned(),
            design: design_text,
            source: spec.fidelity.to_string(),
            grid: grid.to_owned(),
            parameters: plan.axes.iter().map(|a| a.parameter.clone()).collect(),
            units: covered.iter().map(|p| p.unit.clone()).collect(),
            ranges: plan.axes.iter().map(|a| (a.from, a.to)).collect(),
            fixed,
            band_um: (band.0.to_um(), band.1.to_um()),
            sampling: plan.describe_sampling(),
            samples: samples.len(),
            held_out: held.len(),
            fitted_error: model.error().max,
            held_out_error,
            stability,
            passivity,
            model,
        })
    }

    /// The source fidelity.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a file whose source isn't a fidelity.
    pub fn source_fidelity(&self) -> Result<Fidelity> {
        super::fidelity(&self.source)
    }

    /// The file's text: JSON, every number to the last bit ([`Fitted::from_json`] reads it
    /// back to the same model).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a number JSON can't hold.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| Error::Parse {
            what: "compact model".into(),
            reason: e.to_string(),
        })
    }

    /// The compact model `text` holds.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for text that isn't one, of a format other than this one's, or whose
    /// parameters don't match its model's.
    pub fn from_json(text: &str) -> Result<Fitted> {
        let f: Fitted = serde_json::from_str(text).map_err(|e| Error::Parse {
            what: "compact model".into(),
            reason: e.to_string(),
        })?;
        let bad = |reason: String| {
            Err(Error::Parse {
                what: "compact model".into(),
                reason,
            })
        };
        if f.format != FORMAT {
            return bad(format!(
                "is format {}; this photonoxide reads format {FORMAT}",
                f.format
            ));
        }
        let names: Vec<&str> = f
            .model
            .parameters()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        if names != f.parameters.iter().map(String::as_str).collect::<Vec<_>>()
            || f.ranges.len() != f.parameters.len()
            || f.units.len() != f.parameters.len()
        {
            return bad("its parameters aren't its model's".into());
        }
        f.source_fidelity()?;
        Ok(f)
    }

    /// Reads the compact model in the file at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it can't be read, and those of [`Fitted::from_json`].
    pub fn read(path: &Path) -> Result<Fitted> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        Fitted::from_json(&text).map_err(|e| match e {
            Error::Parse { reason, .. } => Error::Parse {
                what: path.display().to_string(),
                reason,
            },
            other => other,
        })
    }

    /// Writes it to `path`.
    ///
    /// # Errors
    ///
    /// Those of [`Fitted::to_json`], and [`Error::Io`] if it can't be written.
    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_json()?).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// The compact fidelity's [`Method`]: what it was fitted to and how, its held-out error
    /// stated against its source fidelity, and where it holds (its range, its band, and the
    /// values of the parameters it doesn't cover).
    fn method(&self, source: Fidelity) -> Method {
        let interpolation = match self.model.interpolation() {
            Interpolation::Polynomial { degree } => format!("polynomial of degree {degree}"),
            Interpolation::PiecewiseLinear => "piecewise linear".into(),
        };
        let mut validity: Vec<Range> = self
            .parameters
            .iter()
            .zip(&self.ranges)
            .map(|(n, r)| Range::new(n, r.0, r.1))
            .collect();
        validity.push(Range::new("wavelength", self.band_um.0, self.band_um.1));
        // a device's own `wavelength` (what its figures are at) is held fixed as any other, but
        // listed once, as the band
        validity.extend(
            self.fixed
                .iter()
                .filter(|(n, _)| n != "wavelength")
                .map(|(n, v)| Range::new(n, *v, *v)),
        );
        Method {
            fidelity: Fidelity::Compact,
            name: format!(
                "a compact model of the {source} model's runs ({}): rational in frequency on {} \
                 poles, {interpolation} in {}, from {} samples ({}), its error measured at {} \
                 held out",
                self.grid,
                self.model.basis_poles(),
                self.parameters.join(" and "),
                self.samples,
                self.sampling,
                self.held_out
            ),
            write_up: "compact".into(),
            papers: vec![
                "B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999), \
                 doi:10.1109/61.772353"
                    .into(),
                "P. Triverio, S. Grivet-Talocia, M. S. Nakhla, IEEE Trans. Adv. Packag. 32, 205 \
                 (2009), doi:10.1109/TADVP.2008.2007913"
                    .into(),
                "C. K. Sanathanan, J. Koerner, IEEE Trans. Autom. Control 8, 56 (1963), \
                 doi:10.1109/TAC.1963.1105517"
                    .into(),
            ],
            error: Some(Stated {
                quantity: "S".into(),
                bound: self.held_out_error,
                unit: String::new(),
                against: source,
            }),
            validity,
        }
    }
}

/// The compact models read so far, by path, with the file's time and length when it was read:
/// a design's model is read once, not at every build of its device.
type Cache = Mutex<HashMap<PathBuf, (Option<SystemTime>, u64, Arc<Fitted>)>>;

/// The compact model in the file at `path`, read once while the file doesn't change.
///
/// # Errors
///
/// Those of [`Fitted::read`].
pub(crate) fn load(path: &Path) -> Result<Arc<Fitted>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let meta = std::fs::metadata(path).map_err(|e| Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;
    let stamp = (meta.modified().ok(), meta.len());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some((time, len, f)) = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(path)
        && (*time, *len) == stamp
    {
        return Ok(f.clone());
    }
    let f = Arc::new(Fitted::read(path)?);
    cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(path.to_path_buf(), (stamp.0, stamp.1, f.clone()));
    Ok(f)
}

/// A device with a compact model: the device's own fidelities and [`Fidelity::Compact`].
pub(crate) struct Compacted {
    inner: Box<dyn Device>,
    fitted: Arc<Fitted>,
    methods: Vec<Method>,
    /// Why the model can't be used with this design's device at all (a stack it wasn't fitted
    /// on), if it can't.
    stale: Option<String>,
}

impl fmt::Debug for Compacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compacted")
            .field("inner", &self.inner)
            .field("kind", &self.fitted.kind)
            .field("parameters", &self.fitted.parameters)
            .finish_non_exhaustive()
    }
}

impl Compacted {
    /// `inner` with `fitted`'s model, fitted to a design of `stack` (its `[device]` table's
    /// stack, as TOML).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a model of another kind of device, or of parameters the
    /// device hasn't.
    pub(crate) fn new(
        inner: Box<dyn Device>,
        fitted: Arc<Fitted>,
        stack: &toml::Value,
    ) -> Result<Compacted> {
        if fitted.kind != inner.kind() {
            return Err(Error::invalid(
                "compact model",
                format!("it is a {}'s, not a {}'s", fitted.kind, inner.kind()),
            ));
        }
        for name in fitted
            .parameters
            .iter()
            .chain(fitted.fixed.iter().map(|(n, _)| n))
        {
            if !inner.parameters().iter().any(|p| p.name == *name) {
                return Err(Error::invalid(
                    "compact model",
                    format!("a {} has no parameter {name}", inner.kind()),
                ));
            }
        }
        let source = fitted.source_fidelity()?;
        let mut methods: Vec<Method> = inner
            .fidelities()
            .iter()
            .filter(|m| m.fidelity != Fidelity::Compact)
            .cloned()
            .collect();
        let at = methods
            .iter()
            .position(|m| rank(m.fidelity) > rank(Fidelity::Compact))
            .unwrap_or(methods.len());
        methods.insert(at, fitted.method(source));
        let fitted_stack = fitted_stack(&fitted.design);
        let stale = (fitted_stack.as_ref() != Some(stack))
            .then(|| "it was fitted on another stack: fit it again on this one".to_owned());
        Ok(Compacted {
            inner,
            fitted,
            methods,
            stale,
        })
    }

    /// The model's component at `values`, the device's: refused for values of the parameters
    /// it doesn't cover other than those it was fitted at, or outside its range.
    fn compact(&self, values: &Values) -> Result<Built> {
        values.check(self.inner.as_ref())?;
        let f = &self.fitted;
        let bad = |reason: String| Err(Error::invalid("compact model", reason));
        if let Some(why) = &self.stale {
            return bad(why.clone());
        }
        let unit = |n: &str| {
            self.inner
                .parameters()
                .iter()
                .find(|p| p.name == n)
                .map_or_else(String::new, |p| p.unit.clone())
        };
        for (name, v) in &f.fixed {
            let x = values.at(name);
            if (x - v).abs() > 1e-12 * v.abs().max(1.0) {
                return bad(format!(
                    "it was fitted at {name} = {v} {}, and this design's is {x}: fit it again, or \
                     over {name} too",
                    unit(name)
                ));
            }
        }
        let at: Vec<f64> = f.parameters.iter().map(|n| values.at(n)).collect();
        for ((name, &(lo, hi)), &x) in f.parameters.iter().zip(&f.ranges).zip(&at) {
            if !(x >= lo - 1e-12 * lo.abs().max(1.0) && x <= hi + 1e-12 * hi.abs().max(1.0)) {
                return bad(format!(
                    "it covers {name} from {lo} to {hi} {}, not {x}",
                    unit(name)
                ));
            }
        }
        let derivatives = self
            .inner
            .parameters()
            .iter()
            .map(|p| {
                f.parameters.iter().position(|n| *n == p.name).map(|k| {
                    let mut d = vec![0.0; f.parameters.len()];
                    d[k] = 1.0;
                    d
                })
            })
            .collect();
        let figures = vec![Figure {
            name: "largest |ΔS| at the held-out samples".into(),
            symbol: "dS_held_out".into(),
            value: f.held_out_error,
            unit: String::new(),
            fidelity: Fidelity::Compact,
            grid: format!("a compact model of the {} model ({})", f.source, f.grid),
        }];
        Ok(Built {
            component: Box::new(Shared(self.fitted.clone())),
            values: at,
            derivatives,
            figures,
        })
    }
}

/// The stack of a fitted design's `[device]` table (TOML), as a value, if it parses.
fn fitted_stack(design: &str) -> Option<toml::Value> {
    let table: toml::Table = toml::from_str(design).ok()?;
    let spec = super::Spec::from_table(&table).ok()?;
    Some(spec.stack.to_value())
}

/// A fidelity's place on the ladder, cheapest first.
fn rank(f: Fidelity) -> u8 {
    match f {
        Fidelity::Analytic => 0,
        Fidelity::Compact => 1,
        Fidelity::TwoD => 2,
        Fidelity::ThreeD => 3,
        Fidelity::Measured => 4,
    }
}

impl Device for Compacted {
    fn kind(&self) -> &str {
        self.inner.kind()
    }

    fn about(&self) -> &str {
        self.inner.about()
    }

    fn parameters(&self) -> &[Parameter] {
        self.inner.parameters()
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        self.inner.stack(values)
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        self.inner.geometry(values)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        if fidelity == Fidelity::Compact {
            self.compact(values)
        } else {
            self.inner.component(values, fidelity)
        }
    }

    fn derived(&self, values: &Values, fidelity: Fidelity) -> Result<Vec<Figure>> {
        if fidelity == Fidelity::Compact {
            Ok(self.compact(values)?.figures)
        } else {
            self.inner.derived(values, fidelity)
        }
    }

    fn objectives(&self) -> Vec<super::Objective> {
        self.inner.objectives()
    }

    fn constraints(&self) -> Vec<super::Constraint> {
        self.inner.constraints()
    }
}

/// A fitted model shared between the components built from it, as a [`Component`].
#[derive(Debug)]
struct Shared(Arc<Fitted>);

impl Component for Shared {
    fn kind(&self) -> &str {
        self.0.model.kind()
    }

    fn ports(&self) -> &[Port] {
        self.0.model.ports()
    }

    fn parameters(&self) -> &[Parameter] {
        self.0.model.parameters()
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        self.0.model.s_matrix(wavelength, values)
    }

    fn provenance(&self) -> Provenance {
        self.0.model.provenance()
    }

    fn reciprocal(&self) -> bool {
        self.0.model.reciprocal()
    }

    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        self.0.model.derivatives(wavelength, values)
    }
}

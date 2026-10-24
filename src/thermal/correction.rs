//! Crosstalk correction posed for an optimizer: the heaters' powers that give each phase
//! shifter its wanted phase through their thermal crosstalk, after V. Gurses et al.,
//! "Large-scale crosstalk-corrected thermo-optic phase shifter arrays in silicon photonics",
//! IEEE J. Sel. Top. Quantum Electron. 28(6), 1 (2022), doi:10.1109/JSTQE.2022.3189965.
//! docs/methods/thermal-compact.md derives it.

use std::f64::consts::{PI, TAU};

use super::RcModel;
use super::rc::solve;
use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("crosstalk correction", reason)
}

/// The correction of an array of heated phase shifters: their steady response C (rad/W,
/// `response[shifter][heater]`), each shifter's phase with no power φ₀ and its wanted phase φ*.
/// At powers P the phases are φ = φ₀ + C P, and the objective
///
/// F(P) = Σ_i (1 − cos(φ_i − φ*_i)),  ∂F/∂P_j = Σ_i sin(φ_i − φ*_i) C_ij,
///
/// is 0 exactly where every phase is its target to a whole turn, smooth everywhere, and its
/// gradient exact: what an optimizer (genoxide's) takes, the powers bounded below by 0 and above
/// by what the heaters stand. Gurses et al.'s correction is its zero at P = C⁻¹(φ* − φ₀)
/// (their Eq. 13, [`Correction::inverse`]), when that P is in bounds; the optimizer finds one
/// with every power in bounds, the targets moved by whole turns where it must.
#[derive(Clone, Debug, PartialEq)]
pub struct Correction {
    response: Vec<Vec<f64>>,
    bias: Vec<f64>,
    target: Vec<f64>,
}

impl Correction {
    /// The correction of shifters of steady response `response` (rad/W, `[shifter][heater]`) to
    /// the phases `target` (rad), with no phase at no power.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no shifter or heater, rows of different lengths, a target
    /// per shifter missing, or a value that isn't finite.
    pub fn new(response: Vec<Vec<f64>>, target: Vec<f64>) -> Result<Correction> {
        let heaters = response.first().map_or(0, Vec::len);
        if heaters == 0 || response.iter().any(|r| r.len() != heaters) {
            return Err(invalid(
                "a response needs a shifter and a heater, every row a value per heater",
            ));
        }
        if target.len() != response.len() {
            return Err(invalid(format!(
                "{} shifters need {} targets, not {}",
                response.len(),
                response.len(),
                target.len()
            )));
        }
        if response
            .iter()
            .flatten()
            .chain(&target)
            .any(|v| !v.is_finite())
        {
            return Err(invalid("a response and a target must be finite"));
        }
        let bias = vec![0.0; response.len()];
        Ok(Correction {
            response,
            bias,
            target,
        })
    }

    /// The correction through a compact model's steady response, Z(0)
    /// ([`RcModel::steady`]), its outputs the shifters' phases.
    ///
    /// # Errors
    ///
    /// As [`Correction::new`].
    pub fn of_model(model: &RcModel, target: Vec<f64>) -> Result<Correction> {
        Correction::new(model.steady(), target)
    }

    /// Gurses et al.'s array (their Eq. 13): `count` shifters in a row, each heater's own phase
    /// π per `p_pi_w` W, and its n-th neighbour's (n = 1, 2, …) π per `neighbours_p_pi_w[n − 1]`
    /// W, none beyond; C_ii = π/P_π, C_i,i±n = π/P_π^(n).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no shifter, a power that isn't finite and positive, and as
    /// [`Correction::new`].
    pub fn row(
        count: usize,
        p_pi_w: f64,
        neighbours_p_pi_w: &[f64],
        target: Vec<f64>,
    ) -> Result<Correction> {
        if count == 0 {
            return Err(invalid("a row needs a shifter"));
        }
        if std::iter::once(&p_pi_w)
            .chain(neighbours_p_pi_w)
            .any(|p| !(p.is_finite() && *p > 0.0))
        {
            return Err(invalid("a power per π must be finite and positive"));
        }
        let response = (0..count)
            .map(|i| {
                (0..count)
                    .map(|j| match i.abs_diff(j) {
                        0 => PI / p_pi_w,
                        n => neighbours_p_pi_w.get(n - 1).map_or(0.0, |p| PI / p),
                    })
                    .collect()
            })
            .collect();
        Correction::new(response, target)
    }

    /// The shifters' phases with no power, rad (0 by default).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another number than one per shifter, or one not finite.
    pub fn with_bias(mut self, bias: Vec<f64>) -> Result<Correction> {
        if bias.len() != self.response.len() || bias.iter().any(|b| !b.is_finite()) {
            return Err(invalid("a bias needs a finite phase per shifter"));
        }
        self.bias = bias;
        Ok(self)
    }

    /// The steady response, rad/W, `[shifter][heater]`.
    pub fn response(&self) -> &[Vec<f64>] {
        &self.response
    }

    /// The wanted phases, rad.
    pub fn target(&self) -> &[f64] {
        &self.target
    }

    /// How many heaters.
    pub fn heaters(&self) -> usize {
        self.response[0].len()
    }

    /// How many shifters.
    pub fn shifters(&self) -> usize {
        self.response.len()
    }

    fn check(&self, powers: &[f64]) -> Result<()> {
        if powers.len() != self.heaters() || powers.iter().any(|p| !p.is_finite()) {
            return Err(invalid(format!(
                "the array takes {} finite powers, one per heater",
                self.heaters()
            )));
        }
        Ok(())
    }

    /// The phases at `powers` (W): φ₀ + C P.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another number of powers than heaters, or one not finite.
    pub fn phases(&self, powers: &[f64]) -> Result<Vec<f64>> {
        self.check(powers)?;
        Ok(self
            .response
            .iter()
            .zip(&self.bias)
            .map(|(row, b)| b + row.iter().zip(powers).map(|(c, p)| c * p).sum::<f64>())
            .collect())
    }

    /// Each shifter's phase error at `powers`, φ_i − φ*_i taken into (−π, π].
    ///
    /// # Errors
    ///
    /// As [`Correction::phases`].
    pub fn errors(&self, powers: &[f64]) -> Result<Vec<f64>> {
        Ok(self
            .phases(powers)?
            .iter()
            .zip(&self.target)
            .map(|(p, t)| {
                let e = (p - t).rem_euclid(TAU);
                if e > PI { e - TAU } else { e }
            })
            .collect())
    }

    /// The objective F at `powers` and its gradient ∂F/∂P (per W).
    ///
    /// # Errors
    ///
    /// As [`Correction::phases`].
    pub fn objective(&self, powers: &[f64]) -> Result<(f64, Vec<f64>)> {
        let d: Vec<f64> = self
            .phases(powers)?
            .iter()
            .zip(&self.target)
            .map(|(p, t)| p - t)
            .collect();
        let f = d.iter().map(|e| 1.0 - e.cos()).sum();
        let g = (0..self.heaters())
            .map(|j| {
                d.iter()
                    .zip(&self.response)
                    .map(|(e, row)| e.sin() * row[j])
                    .sum()
            })
            .collect();
        Ok((f, g))
    }

    /// The powers that ignore the crosstalk: each heater set for its own shifter alone,
    /// P_j = ((φ*_j − φ₀_j) mod 2π)/C_jj. What a correction starts from.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless there is a heater per shifter, each moving its own.
    pub fn uncorrected(&self) -> Result<Vec<f64>> {
        self.square()?;
        Ok((0..self.shifters())
            .map(|j| (self.target[j] - self.bias[j]).rem_euclid(TAU) / self.response[j][j])
            .collect())
    }

    /// Gurses et al.'s correction, P = C⁻¹ (φ* − φ₀) (their Eq. 13): the phases exactly the
    /// targets as given, whether or not every power is positive.
    ///
    /// # Errors
    ///
    /// As [`Correction::uncorrected`], and [`Error::InvalidValue`] for a response that isn't
    /// invertible.
    pub fn inverse(&self) -> Result<Vec<f64>> {
        self.square()?;
        let b: Vec<f64> = self
            .target
            .iter()
            .zip(&self.bias)
            .map(|(t, b)| t - b)
            .collect();
        let p = solve(self.response.clone(), b);
        let back = self.phases(&p)?;
        let worst = back
            .iter()
            .zip(&self.target)
            .fold(0.0f64, |m, (a, t)| m.max((a - t).abs()));
        if worst.is_nan()
            || worst > 1e-9 * (1.0 + self.target.iter().fold(0.0f64, |m, t| m.max(t.abs())))
        {
            return Err(invalid("the response isn't invertible"));
        }
        Ok(p)
    }

    fn square(&self) -> Result<()> {
        if self.heaters() != self.shifters()
            || (0..self.shifters()).any(|j| self.response[j][j] == 0.0)
        {
            return Err(invalid(
                "this needs a heater per shifter, each moving its own shifter's phase",
            ));
        }
        Ok(())
    }
}

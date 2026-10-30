//! A circuit whose phase shifters are heated: each one's phase from its own heater's power and
//! its neighbours' through a dynamic thermal compact model ([`crate::thermal::RcModel`], Coenen
//! et al. 2022), in time and in frequency. docs/methods/thermal-compact.md derives it.

use num_complex::Complex64 as c64;

use super::{Circuit, Fidelity, Provenance, SMatrix};
use crate::thermal::RcModel;
use crate::units::Wavelength;
use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("heated circuit", reason)
}

/// A circuit, the instances whose `phase` the heaters drive, and the thermal model from the
/// heaters' powers (W) to those phases (rad): instance k's phase is its bias plus output k of
/// the model. The optics follow the heat at once (the heat's microseconds against the light's
/// picoseconds), so each instant is a circuit solve at that instant's phases.
#[derive(Clone, Debug)]
pub struct Heated {
    circuit: Circuit,
    shifters: Vec<String>,
    /// Each shifter's `phase` among the circuit's parameters.
    index: Vec<usize>,
    model: RcModel,
    bias: Vec<f64>,
    error: Option<f64>,
}

/// A heated circuit's response in time ([`Heated::transient`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HeatedTransient {
    /// The times, seconds.
    pub times_s: Vec<f64>,
    /// `phases[shifter][time]`, rad, bias included.
    pub phases: Vec<Vec<f64>>,
    /// The circuit's S-matrix at each time.
    pub s: Vec<SMatrix>,
}

impl Heated {
    /// `circuit` with the phases of instances `shifters` driven through `model`, the k-th
    /// shifter its k-th output, each at its current phase with no power (its bias).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a shifter named twice, one that isn't an instance with a
    /// `phase`, or a model with another number of outputs than shifters.
    pub fn new(circuit: Circuit, shifters: &[&str], model: RcModel) -> Result<Heated> {
        if model.outputs() != shifters.len() {
            return Err(invalid(format!(
                "{} shifters need a model of {} outputs, not {}",
                shifters.len(),
                shifters.len(),
                model.outputs()
            )));
        }
        let mut index = Vec::with_capacity(shifters.len());
        for (k, s) in shifters.iter().enumerate() {
            if shifters[..k].contains(s) {
                return Err(invalid(format!("the shifter {s} is named twice")));
            }
            let i = circuit.parameter(&format!("{s}.phase")).ok_or_else(|| {
                invalid(format!(
                    "{s} isn't an instance with a phase (a phase shifter's `phase`)"
                ))
            })?;
            index.push(i);
        }
        let bias = index.iter().map(|&i| circuit.values()[i]).collect();
        Ok(Heated {
            circuit,
            shifters: shifters.iter().map(|s| s.to_string()).collect(),
            index,
            model,
            bias,
            error: None,
        })
    }

    /// The shifters' phases with no power, rad (by default their values in the circuit).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another number than one per shifter, or one not finite.
    pub fn with_bias(mut self, phases: &[f64]) -> Result<Heated> {
        if phases.len() != self.shifters.len() || phases.iter().any(|p| !p.is_finite()) {
            return Err(invalid("a bias needs a finite phase per shifter"));
        }
        self.bias = phases.to_vec();
        Ok(self)
    }

    /// The model's error against what it was fitted to, for the provenance
    /// ([`crate::thermal::Fitted::error`]).
    pub fn with_error(mut self, error: f64) -> Heated {
        self.error = Some(error);
        self
    }

    /// The circuit.
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// The thermal model.
    pub fn model(&self) -> &RcModel {
        &self.model
    }

    /// The shifters, in the model's order.
    pub fn shifters(&self) -> &[String] {
        &self.shifters
    }

    /// Where the shifters' phases come from: a compact model, with its fit's error.
    pub fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Compact,
            source: format!(
                "a dynamic thermal compact model of {} poles from {} heaters to {} phases \
                 (Coenen et al. 2022, doi:10.1109/TCPMT.2022.3195699)",
                self.model.time_constants_s().len(),
                self.model.inputs(),
                self.model.outputs()
            ),
            error: self.error,
            validity: None,
        }
    }

    /// The circuit's values with the shifters at `phases`, each taken into [0, 2π).
    fn values(&self, phases: &[f64]) -> Vec<f64> {
        let mut v = self.circuit.values().to_vec();
        for (&i, p) in self.index.iter().zip(phases) {
            v[i] = p.rem_euclid(std::f64::consts::TAU);
        }
        v
    }

    fn check_powers(&self, powers: &[f64]) -> Result<()> {
        if powers.len() != self.model.inputs() || powers.iter().any(|p| !p.is_finite()) {
            return Err(invalid(format!(
                "the model takes {} finite powers, one per heater",
                self.model.inputs()
            )));
        }
        Ok(())
    }

    /// The shifters' steady phases at heater powers `powers` (W): bias + Z(0) P.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another number of powers than the model's inputs, or one not
    /// finite.
    pub fn phases(&self, powers: &[f64]) -> Result<Vec<f64>> {
        self.check_powers(powers)?;
        let z = self.model.steady();
        Ok(self
            .bias
            .iter()
            .zip(&z)
            .map(|(b, row)| b + row.iter().zip(powers).map(|(c, p)| c * p).sum::<f64>())
            .collect())
    }

    /// The circuit's steady S-matrix at `wavelength` with the heaters at `powers` (W).
    ///
    /// # Errors
    ///
    /// As [`Heated::phases`] and [`Circuit::s_matrix_with`].
    pub fn s_matrix(&self, wavelength: Wavelength, powers: &[f64]) -> Result<SMatrix> {
        let phases = self.phases(powers)?;
        self.circuit
            .s_matrix_with(wavelength, &self.values(&phases))
    }

    /// The circuit in time at `wavelength`: from rest (no power before), heater j at
    /// `powers[j][m]` W from `times_s[m]` to the next time, held ([`RcModel::simulate`]), the
    /// circuit solved at each time's phases.
    ///
    /// # Errors
    ///
    /// As [`RcModel::simulate`] and [`Circuit::s_matrix_with`].
    pub fn transient(
        &self,
        wavelength: Wavelength,
        times_s: &[f64],
        powers: &[Vec<f64>],
    ) -> Result<HeatedTransient> {
        let rise = self.model.simulate(times_s, powers)?;
        let phases: Vec<Vec<f64>> = rise
            .iter()
            .zip(&self.bias)
            .map(|(r, b)| r.iter().map(|v| b + v).collect())
            .collect();
        let s = (0..times_s.len())
            .map(|m| {
                let at: Vec<f64> = phases.iter().map(|p| p[m]).collect();
                self.circuit.s_matrix_with(wavelength, &self.values(&at))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(HeatedTransient {
            times_s: times_s.to_vec(),
            phases,
            s,
        })
    }

    /// The small-signal response of the power from external port `p` to port `q`, |S_qp|², to
    /// heater `heater`'s power about the steady state at `powers`, at each of `frequencies_hz`:
    /// per W of a modulation at f, its complex amplitude Σ_k (∂|S_qp|²/∂φ_k) Z_k,heater(i 2πf),
    /// ∂|S_qp|²/∂φ_k = 2 Re(conj(S_qp) ∂S_qp/∂φ_k) by the circuit adjoint ([`Circuit::jacobian`])
    /// and Z the model's.
    ///
    /// # Errors
    ///
    /// As [`Heated::phases`] and [`Circuit::jacobian`], and [`Error::InvalidValue`] for a heater
    /// the model hasn't or a port the circuit hasn't.
    pub fn modulation(
        &self,
        wavelength: Wavelength,
        powers: &[f64],
        (q, p): (usize, usize),
        heater: usize,
        frequencies_hz: &[f64],
    ) -> Result<Modulation> {
        if heater >= self.model.inputs() {
            return Err(invalid(format!(
                "the model has {} heaters, not a heater {heater}",
                self.model.inputs()
            )));
        }
        let phases = self.phases(powers)?;
        let (s, jacobian) = self.circuit.jacobian(wavelength, &self.values(&phases))?;
        if q >= s.size() || p >= s.size() {
            return Err(invalid(format!(
                "the circuit has {} external ports, not ports {q} and {p}",
                s.size()
            )));
        }
        let by_phase: Vec<f64> = self
            .index
            .iter()
            .map(|&i| 2.0 * (s[(q, p)].conj() * jacobian[i][(q, p)]).re)
            .collect();
        let per_watt = frequencies_hz
            .iter()
            .map(|&f| {
                by_phase
                    .iter()
                    .enumerate()
                    .map(|(k, g)| *g * self.model.response(k, heater, f))
                    .sum::<c64>()
            })
            .collect();
        Ok(Modulation {
            power: s[(q, p)].norm_sqr(),
            by_phase,
            per_watt,
        })
    }
}

/// A heated circuit's small-signal response ([`Heated::modulation`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Modulation {
    /// |S_qp|² at the steady state.
    pub power: f64,
    /// ∂|S_qp|²/∂φ_k for each shifter, per rad.
    pub by_phase: Vec<f64>,
    /// At each frequency, |S_qp|²'s complex amplitude per W of the heater's modulation.
    pub per_watt: Vec<c64>,
}

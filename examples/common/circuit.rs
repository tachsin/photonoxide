//! Closed-form components for the circuit examples, each with its derivatives for the circuit
//! adjoint: a waveguide, a lossless directional coupler and a phase shifter. A component is
//! anything that implements `photonoxide::circuit::Component`.

use std::f64::consts::{LN_10, TAU};

use photonoxide::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, ports};
use photonoxide::units::Wavelength;
use photonoxide::{Complex64 as c64, Result};

fn zero() -> c64 {
    c64::new(0.0, 0.0)
}

/// A 2-port that transmits `t` both ways and reflects nothing.
fn two_port(t: c64) -> SMatrix {
    SMatrix::from_fn(2, |q, p| if q == p { zero() } else { t })
}

/// A straight waveguide, ports `a` and `b`: t = 10^(−αL/20) e^(i 2π n(λ) L/λ), its effective
/// index first order in λ about λ₀, n(λ) = n₀ + (n₀ − n_g)(λ − λ₀)/λ₀. Parameters: `length` (µm)
/// and `loss` (α, dB/cm).
#[derive(Debug)]
pub struct Waveguide {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    /// n₀ at λ₀.
    pub index: f64,
    /// n_g.
    pub group_index: f64,
    /// λ₀, µm.
    pub reference_um: f64,
}

impl Waveguide {
    /// A waveguide of effective index `index` and group index `group_index` at
    /// `reference_um`, 10 µm long and lossless by default.
    pub fn new(index: f64, group_index: f64, reference_um: f64) -> Waveguide {
        Waveguide {
            ports: ports(&["a", "b"]),
            parameters: vec![
                Parameter::new("length", "µm", 10.0, 0.0, 1e5),
                Parameter::new("loss", "dB/cm", 0.0, 0.0, 1e3),
            ],
            index,
            group_index,
            reference_um,
        }
    }

    /// n(λ).
    pub fn effective_index(&self, wavelength_um: f64) -> f64 {
        self.index
            + (self.index - self.group_index) * (wavelength_um - self.reference_um)
                / self.reference_um
    }

    /// The transmission of `length` µm with a loss of `loss` dB/cm at `wavelength_um`.
    pub fn transmission(&self, wavelength_um: f64, length: f64, loss: f64) -> c64 {
        let amplitude = 10f64.powf(-loss * length * 1e-4 / 20.0);
        amplitude
            * c64::from_polar(
                1.0,
                TAU * self.effective_index(wavelength_um) * length / wavelength_um,
            )
    }
}

impl Component for Waveguide {
    fn kind(&self) -> &str {
        "waveguide"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        Ok(two_port(self.transmission(
            wavelength.to_um(),
            values[0],
            values[1],
        )))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "a uniform waveguide's phase and loss, first order in wavelength".into(),
            error: Some(0.0),
            validity: None,
        }
    }

    /// ∂t/∂L = t (i 2π n/λ − α ln 10 · 1e-4/20), ∂t/∂α = −t L ln 10 · 1e-4/20.
    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let w = wavelength.to_um();
        let (length, loss) = (values[0], values[1]);
        let t = self.transmission(w, length, loss);
        let decay = 1e-4 * LN_10 / 20.0;
        let by_length = t * c64::new(-loss * decay, TAU * self.effective_index(w) / w);
        let by_loss = t * (-length * decay);
        Ok(Some(vec![two_port(by_length), two_port(by_loss)]))
    }
}

/// A lossless directional coupler, inputs `a1`, `a2` and outputs `b1`, `b2`: straight through
/// r = √(1 − κ²), across iκ. Parameter: `kappa2`, the power coupled across.
#[derive(Debug)]
pub struct Coupler {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl Coupler {
    /// A 50:50 coupler by default.
    pub fn new() -> Coupler {
        Coupler {
            ports: ports(&["a1", "a2", "b1", "b2"]),
            parameters: vec![Parameter::new("kappa2", "", 0.5, 0.0, 1.0)],
        }
    }
}

/// The coupler's matrix with `through` straight through and `across` across.
fn coupler(through: c64, across: c64) -> SMatrix {
    let z = zero();
    SMatrix::from_rows(vec![
        vec![z, z, through, across],
        vec![z, z, across, through],
        vec![through, across, z, z],
        vec![across, through, z, z],
    ])
    .expect("square")
}

impl Component for Coupler {
    fn kind(&self) -> &str {
        "directional coupler"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        let k2 = values[0];
        Ok(coupler(
            c64::new((1.0 - k2).sqrt(), 0.0),
            c64::new(0.0, k2.sqrt()),
        ))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "a lossless symmetric coupler, r² + κ² = 1".into(),
            error: Some(0.0),
            validity: None,
        }
    }

    /// ∂r/∂κ² = −1/(2r), ∂(iκ)/∂κ² = i/(2κ).
    fn derivatives(&self, _: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let k2 = values[0];
        Ok(Some(vec![coupler(
            c64::new(-0.5 / (1.0 - k2).sqrt(), 0.0),
            c64::new(0.0, 0.5 / k2.sqrt()),
        )]))
    }
}

/// An ideal phase shifter, ports `a` and `b`: t = e^(iφ). Parameter: `phase`, φ in rad.
#[derive(Debug)]
pub struct PhaseShifter {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl PhaseShifter {
    /// No phase by default; from −2π to 2π.
    pub fn new() -> PhaseShifter {
        PhaseShifter {
            ports: ports(&["a", "b"]),
            parameters: vec![Parameter::new("phase", "rad", 0.0, -TAU, TAU)],
        }
    }
}

impl Component for PhaseShifter {
    fn kind(&self) -> &str {
        "phase shifter"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        Ok(two_port(c64::from_polar(1.0, values[0])))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "an ideal phase shifter, e^(iφ)".into(),
            error: Some(0.0),
            validity: None,
        }
    }

    /// ∂t/∂φ = i t.
    fn derivatives(&self, _: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        Ok(Some(vec![two_port(
            c64::new(0.0, 1.0) * c64::from_polar(1.0, values[0]),
        )]))
    }
}

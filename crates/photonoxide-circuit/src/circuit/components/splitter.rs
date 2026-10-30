//! The Y-branch: an ideal 50/50 splitter with an excess loss.

use super::{Dual, Entries, amplitude, both, check, closed_form, closed_form_derivatives};
use crate::Result;
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, ports};
use crate::units::Wavelength;

/// An ideal Y-branch: ports `o1` (the stem), `o2` and `o3` (the arms), the stem's power split
/// equally, S₂₁ = S₃₁ = 10^(−loss/20)/√2, the same back, no reflection and no phase.
/// Parameter `excess_loss`, dB.
///
/// It is reciprocal and passive but never lossless, whatever the loss: light coming back in the
/// arms' odd combination (S₂ = −S₃) has nowhere to go in the stem's single mode and radiates,
/// so S has a zero singular value. A Mach–Zehnder interferometer of two Y-branches
/// ([`super::mzi_y`]) loses its out-of-phase light that way.
#[derive(Clone, Debug, PartialEq)]
pub struct YBranch {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl YBranch {
    /// A Y-branch, its excess loss starting at 0 dB.
    pub fn new() -> YBranch {
        YBranch {
            ports: ports(&["o1", "o2", "o3"]),
            parameters: vec![Parameter::new("excess_loss", "dB", 0.0, 0.0, 10.0)],
        }
    }

    fn entries(values: &[Dual]) -> Entries {
        let s = amplitude(values[0]) * std::f64::consts::FRAC_1_SQRT_2;
        let mut e = Vec::with_capacity(4);
        e.extend(both(1, 0, s));
        e.extend(both(2, 0, s));
        e
    }
}

impl Default for YBranch {
    fn default() -> YBranch {
        YBranch::new()
    }
}

impl Component for YBranch {
    fn kind(&self) -> &str {
        "Y-branch"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        closed_form(3, values, |v| Ok(YBranch::entries(v)))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "an ideal Y-branch: the stem's power split equally between the arms, with an \
                     excess loss"
                .into(),
            error: Some(0.0),
            validity: None,
        }
    }

    fn derivatives(&self, _: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        closed_form_derivatives(3, values, |v| Ok(YBranch::entries(v)))
    }
}

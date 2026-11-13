//! Photon pairs born inside a circuit: sources pumped through one part of it, their pairs
//! interfering in the rest.
//!
//! Weakly pumped sources at ports k of the rest of the circuit (each pair's signal entering at
//! s_k and its idler at i_k) make the state Σ_k ξ_k â†_s_k â†_i_k |0⟩, ξ_k the source's pair
//! amplitude: for four-wave mixing, the product of the two pump photons' field amplitudes at the
//! source, each from the pump's path through the first part of the circuit. Through the rest,
//! S_s at the signal's wavelength and S_i at the idler's, the amplitude to find the signal at
//! port o₁ and the idler at o₂ is
//!
//! A(o₁, o₂) = Σ_k ξ_k S_s[o₁, s_k] S_i[o₂, i_k];
//!
//! for degenerate photons (one wavelength, indistinguishable) one photon at o₁ and one at o₂
//! (o₁ ≠ o₂) has Σ_k ξ_k (S[o₁, s_k] S[o₂, i_k] + S[o₂, s_k] S[o₁, i_k]). A coincidence rate is
//! |A|², up to the pair rate. Two sources whose amplitudes differ by a phase φ make a fringe
//! |x + y e^(iφ)|² from (|x| − |y|)² to (|x| + |y|)², visibility (N_max − N_min)/N_max =
//! 4|x||y|/(|x| + |y|)² ([`two_path_visibility`]); J. W. Silverstone et al., Nat. Photonics 8, 104
//! (2013), doi:10.1038/nphoton.2013.339, Eq. 1, the on-chip fringe of two such sources.

use num_complex::Complex64 as c64;

use super::transfer::Transfer;
use crate::portable;
use crate::{Error, Result};

/// A source of photon pairs inside a circuit: the ports by which its signal and idler enter the
/// rest of the circuit, and its pair amplitude ξ.
#[derive(Clone, Debug, PartialEq)]
pub struct PairBirth {
    /// The port its signal enters by.
    pub signal: String,
    /// The port its idler enters by.
    pub idler: String,
    /// ξ: the pair amplitude, the product of its pump photons' amplitudes there.
    pub amplitude: c64,
}

/// The amplitude to find a pair's signal at port `outputs[0]` and its idler at `outputs[1]`,
/// through `signal` (the rest of the circuit at the signal's wavelength) and `idler` (at the
/// idler's), from the sources `births`. With `degenerate`, the photons are indistinguishable
/// (`signal` and `idler` the same transfer) and the amplitude is for one photon at each of the two
/// outputs, which must differ.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a port either transfer doesn't have, transfers with different
/// ports, or equal outputs of degenerate photons.
pub fn pair_amplitude(
    signal: &Transfer,
    idler: &Transfer,
    births: &[PairBirth],
    outputs: [&str; 2],
    degenerate: bool,
) -> Result<c64> {
    if signal.ports() != idler.ports() {
        return Err(invalid(
            "the signal's and the idler's transfers have different ports",
        ));
    }
    let port = |name: &str| {
        signal.port(name).ok_or_else(|| {
            invalid(format!(
                "no port {name}; the ports are {}",
                signal.ports().join(", ")
            ))
        })
    };
    let (o1, o2) = (port(outputs[0])?, port(outputs[1])?);
    if degenerate && o1 == o2 {
        return Err(invalid(
            "a coincidence of degenerate photons needs two different outputs",
        ));
    }
    let (s, i) = (signal.matrix(), idler.matrix());
    let mut sum = c64::new(0.0, 0.0);
    for b in births {
        let (ps, pi) = (port(&b.signal)?, port(&b.idler)?);
        let mut term = s[(o1, ps)] * i[(o2, pi)];
        if degenerate {
            term += s[(o2, ps)] * i[(o1, pi)];
        }
        sum += b.amplitude * term;
    }
    Ok(sum)
}

/// The visibility (N_max − N_min)/N_max of a fringe |x + y e^(iφ)|² over φ: 4|x||y|/(|x| + |y|)²,
/// 1 when the two paths are equally likely, and 0 with no light.
pub fn two_path_visibility(x: c64, y: c64) -> f64 {
    let (a, b) = (portable::complex::abs(x), portable::complex::abs(y));
    if a + b == 0.0 {
        return 0.0;
    }
    4.0 * a * b / ((a + b) * (a + b))
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("photon pairs in a circuit", reason)
}

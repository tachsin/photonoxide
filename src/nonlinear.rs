//! Nonlinear integrated optics (0.6.1): second- and third-order processes in crystals and
//! waveguides.
//!
//! - [`phase`]: phase matching. The wave-vector mismatch Δk of any three- or four-wave process
//!   from any index source (a catalogue model's axis with its temperature, a material, or a
//!   mode's effective indices given as data); quasi-phase matching, its period, order and
//!   duty cycle; birefringent phase matching of types 0, I and II, noncritical or angle tuned;
//!   the undepleted response sinc²(ΔkL/2); and the acceptance bandwidths in wavelength,
//!   temperature and angle, after Fejer et al. 1992.
//! - [`coupled`]: the coupled-amplitude equations of SHG, sum- and difference-frequency
//!   generation and parametric amplification along a guide, depleted or not, with loss
//!   (Armstrong et al. 1962).
//! - [`pairs`]: photon pairs by spontaneous parametric down-conversion and spontaneous four-wave
//!   mixing: the joint spectral
//!   amplitude and its Schmidt decomposition, the pairs' purity (Grice and Walmsley 1997).
//! - [`guide`]: all of it in a waveguide, from its modes: the mismatch, the poling period and
//!   the group indices from the mode solver's effective indices over wavelength, and the coupling
//!   κ of the coupled equations from the full-vector fields and the crystal's d tensor turned
//!   into the guide's frame (the normalized efficiency in %/(W·cm²)), and the pairs' rate.
//! - [`pulse`]: pulses along a Kerr guide, the nonlinear Schrödinger equation by split-step
//!   Fourier: β₂, β₃, β₄, … from the guide's effective index over wavelength, γ from its
//!   effective area and the catalogue's n₂; solitons and self-phase modulation (Agrawal 2019).
//! - [`fwm`]: four-wave mixing with one pump, its parametric gain and conversion with the
//!   mismatch from the guide's dispersion, closed form and along the guide, depleted.
//! - [`resonator`]: Kerr microresonators by the Lugiato–Lefever equation: the homogeneous
//!   steady states and their bistability, modulation instability, and dissipative Kerr
//!   solitons, a ring's equation from its waveguide (Lugiato and Lefever 1987, Coen et al.
//!   2012, Herr et al. 2013).
//! - [`miller`]: Miller's rule, a d coefficient carried from the wavelength it was measured at
//!   to another through the three waves' linear susceptibilities (Miller 1964).
//!
//! The materials' third-order data, n₂ and two-photon absorption, are the catalogue's
//! ([`crate::material::catalogue::kerr`]); a guide's γ is [`crate::mode::quantities`]'.
//!
//! The write-up is `docs/methods/nonlinear.md`.

pub(crate) mod checks;
pub mod coupled;
pub(crate) mod data_checks;
mod fft;
pub mod fwm;
pub mod guide;
pub(crate) mod guide_checks;
pub(crate) mod kerr_checks;
pub mod miller;
pub mod pairs;
pub mod phase;
pub mod pulse;
pub mod resonator;

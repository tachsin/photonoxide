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
//! - [`pairs`]: photon pairs by spontaneous parametric down-conversion: the joint spectral
//!   amplitude and its Schmidt decomposition, the pairs' purity (Grice and Walmsley 1997).
//!
//! The write-up is `docs/methods/nonlinear.md`.

pub(crate) mod checks;
pub mod coupled;
pub mod pairs;
pub mod phase;

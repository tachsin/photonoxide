//! Linear-optical statistics: photons on a circuit's ports, and what comes out.
//!
//! A linear circuit's quantum description is its S-matrix. photonoxide's S-matrices are
//! power-normalized in the e^(−iωt) convention, so they transform the ports' annihilation
//! operators: the outgoing modes are b̂ = S â for a lossless circuit. A lossy circuit needs noise
//! modes to keep the commutators, b̂ = S â + L ĉ with L L† = I − S S†, the vacuum entering
//! through the loss ([`Transfer`]). Everything else follows from S:
//!
//! - n single photons in input modes T leave in modes R with probability
//!   |Per(S_RT)|² / (∏ m_t! ∏ m_r!) for occupations m (S. Aaronson, A. Arkhipov, Theory Comput.
//!   9, 143 (2013), doi:10.4086/toc.2013.v009a004): [`Distribution`], [`probability`];
//! - the permanent by Ryser's formula or Glynn's, in Gray-code order, threaded and the same bit
//!   for bit on any number of threads ([`permanent`]);
//! - partly distinguishable photons through their overlaps ([`Photons::Overlaps`]);
//! - an exact sampler, Clifford and Clifford's algorithm B, with an explicit seed ([`sample`]);
//! - two-photon interference with the photons' spectra and the circuit's own S(ω) from its
//!   sweep: the Hong–Ou–Mandel dip ([`hom_dip`]);
//! - Haar-random unitaries and the Reck and Clements decompositions of a unitary into a mesh
//!   of beam splitters ([`Mesh`]).
//!
//! With real components (Q3): many photons with spectra through any S(ω), a closed form's, a
//! compact model's, a solver's or a Touchstone file's ([`SpectralInput`]); photon pairs born
//! inside a circuit and their fringes ([`pair_amplitude`]); loss budgets per path, photon and
//! event, along the routes a photon can take, against published thresholds and Qi et al.'s
//! simulability test ([`Budget`], [`routes`], [`Threshold`], [`qi_simulable_error`]). The
//! emitters' and detectors' electromagnetics are the solvers': [`crate::fdtd::emitter`] for β
//! and Purcell factors,
//! [`crate::mode::multilayer::Multilayer::absorption`] for a detector stack's absorption and the
//! full-vector modes' loss for a waveguide detector's.
//!
//! Broadband pulsed squeezing at any gain, whose squeezed modes change with the gain: the
//! Bogoliubov map of a source cut into slices along its length ([`Broadband`]).
//!
//! docs/methods/quantum.md derives each formula and lists the validation cases.

mod broadband;
mod budget;
mod fock;
mod gaussian;
mod hafnian;
mod hom;
mod intermodal;
mod matrix;
mod mesh;
mod pairs;
mod permanent;
mod ring;
mod rng;
mod sampler;
mod source;
mod spectral;
mod squeezer;
mod transfer;
mod walk;

pub(crate) mod checks;
pub(crate) mod light_checks;
#[cfg(test)]
mod light_tests;
pub(crate) mod real_checks;
#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod tests;

pub use broadband::Broadband;
pub use budget::{
    Budget, Hop, Stage, Threshold, event, fusion_loss, qi_exact_transmission, qi_fidelity,
    qi_simulable_error, routes, varnava_loss,
};
pub use fock::{Distribution, Fock, Photons, patterns, probability};
pub use gaussian::{Gaussian, anti_squeezed_quadrature, db_of_squeezing, squeezed_quadrature};
pub use hafnian::{hafnian, loop_hafnian, torontonian};
pub use hom::{Dip, Wavepacket, coincidence, hom_dip};
pub use intermodal::{Beta, DualPump, dual_pump_spectrum, shaped_dual_pump_spectrum};
pub use matrix::Matrix;
pub use mesh::{Arrangement, Cell, Mesh, fidelity, haar_unitary};
pub use pairs::{PairBirth, pair_amplitude, two_path_visibility};
pub use permanent::{Formula, permanent, permanent_by};
pub use ring::{RingGrid, ring_spectrum};
pub use sampler::sample;
pub use source::{Source, heralding_through, hom_visibility, path};
pub use spectral::{SpectralInput, SpectralPhoton};
pub use squeezer::{Squeezer, degenerate, four_wave, non_degenerate};
pub use transfer::Transfer;
pub use walk::coupled_array;

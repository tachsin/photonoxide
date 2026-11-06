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
//! docs/methods/quantum.md derives each formula and lists the validation cases.

mod fock;
mod hom;
mod matrix;
mod mesh;
mod permanent;
mod rng;
mod sampler;
mod transfer;
mod walk;

pub(crate) mod checks;
#[cfg(test)]
mod tests;

pub use fock::{Distribution, Fock, Photons, patterns, probability};
pub use hom::{Dip, Wavepacket, coincidence, hom_dip};
pub use matrix::Matrix;
pub use mesh::{Arrangement, Cell, Mesh, fidelity, haar_unitary};
pub use permanent::{Formula, permanent, permanent_by};
pub use sampler::sample;
pub use transfer::Transfer;
pub use walk::coupled_array;

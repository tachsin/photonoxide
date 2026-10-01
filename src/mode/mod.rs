//! Waveguide modes: the field patterns that propagate unchanged along a waveguide, and their
//! effective indices.
//!
//! - [`bend`]: the exact modes of a bent slab, and their radiation loss.
//! - [`dispersion`]: group index, dispersion and loss from the effective index, and one mode
//!   followed across wavelength.
//! - [`fields`]: a full-vector mode's six field components, its power, and its coupling into
//!   another mode.
//! - [`eim`]: the effective index method, a ridge's mode from two slab problems.
//! - [`marcatili`]: Marcatili's approximation for rectangular guides.
//! - [`multilayer`]: the bound modes and leaky waves of any planar stack, exactly, by
//!   transfer matrices.
//! - [`slab_fd`]: any planar profile (graded, lossy, leaky) by 1D finite differences.
//! - [`slab`]: the modes of a three-layer slab, exactly (TE and TM).
//! - [`vector`]: the full-vector modes of any cross-section, by finite differences.

pub mod bend;
pub mod dispersion;
pub mod eim;
pub mod fields;
pub mod marcatili;
pub mod multilayer;
pub mod slab;
pub mod slab_fd;
pub mod vector;

/// The polarization of a slab mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Polarization {
    /// Transverse electric: the electric field lies in the slab's plane, across the direction of
    /// propagation.
    Te,
    /// Transverse magnetic: the magnetic field lies in the slab's plane, across the direction
    /// of propagation.
    Tm,
}

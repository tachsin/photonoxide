//! Waveguide modes: the field patterns that propagate unchanged along a waveguide, and their
//! effective indices.
//!
//! - [`slab`]: the modes of a three-layer slab, exactly (TE and TM).

pub mod slab;

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

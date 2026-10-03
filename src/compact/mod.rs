//! Compact models and Touchstone files: a device's S-matrix spectrum as a few numbers.
//!
//! - [`touchstone`]: Touchstone (`.sNp`) files, read and written, Version 1 and 2.0: measured
//!   and simulated S-parameters in the format microwave and photonics tools exchange them in,
//!   with the conversions between frequency and wavelength and between time conventions.
//!
//! The method's page is docs/methods/compact.md.

pub mod touchstone;

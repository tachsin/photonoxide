//! # photonoxide-device
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::device`, `photonoxide::generator`, `photonoxide::thermal`).
//!
//! photonoxide's device layer (docs/architecture.md): components that know their geometry, the
//! component generators, and the thermo-optic phase shifter, with `thermal`, the statics crate's
//! heat solver and the shifter under one path. It names only the layers below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

#[doc(hidden)]
pub mod cases;
pub mod device;
pub mod generator;
mod shifter;
pub mod thermal;

// the layers' modules below, at the `crate::` paths the modules here were written with
use photonoxide_circuit::{circuit, compact};
use photonoxide_core::{Error, Result, convergence, expr, parallel, portable, stop, units};
use photonoxide_fdfd::fdfd;
use photonoxide_geometry::{averaging, geometry, material, raster, stack};
use photonoxide_mode::mode;
use photonoxide_statics::{electro, statics};
// the validation types, which the checks here name by photonoxide's path for them
// (`photonoxide::validation` re-exports `verdict`'s)
use photonoxide_core::verdict as validation;
// the netlist's errors, re-exported by circuit at its paths for them

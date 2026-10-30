//! # photonoxide-statics
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::electro`, and `photonoxide::thermal` with the thermo-optic
//! shifter beside it).
//!
//! photonoxide's statics layer (docs/architecture.md): the static solver on a cross-section,
//! heat conduction and the electrostatics and travelling-wave lines on it. It names only the
//! core, geometry and linear algebra layers below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub mod electro;
pub mod statics;
pub mod thermal;

// the layers' modules below, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, convergence, units, verdict};
use photonoxide_geometry::{averaging, geometry, raster, stack};
use photonoxide_linalg::{eigen, sparse};

//! # photonoxide-circuit
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::circuit`, `photonoxide::compact`).
//!
//! photonoxide's circuit layer (docs/architecture.md): components, netlists and their solve and
//! adjoint, and compact models. It names only the core, geometry, linear algebra, mode and
//! statics layers below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod circuit;
pub mod compact;

// the layers' modules below, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, lstsq, portable, units};
use photonoxide_geometry::material;
use photonoxide_mode::mode;
use photonoxide_statics::thermal;
// the validation types, which the checks here name by photonoxide's path for them
// (`photonoxide::validation` re-exports `verdict`'s)
// the netlist's errors, re-exported by circuit at its paths for them
use photonoxide_core::error;

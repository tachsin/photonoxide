//! # photonoxide-mode
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::mode`, `photonoxide::eme`, `photonoxide::sweep`).
//!
//! photonoxide's mode layer (docs/architecture.md): the mode solvers, eigenmode expansion and
//! adaptive sweeps. It names only the core, geometry and linear algebra layers below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub mod eme;
pub mod mode;
pub mod sweep;

// the layers' modules below, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, convergence, parallel, portable, units, verdict};
use photonoxide_geometry::material;
use photonoxide_linalg::eigen;
// the validation types, which the checks here name by photonoxide's path for them
// (`photonoxide::validation` re-exports `verdict`'s)
use photonoxide_core::verdict as validation;

//! # photonoxide-geometry
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::geometry`, `photonoxide::material`, ...).
//!
//! photonoxide's geometry layer (docs/architecture.md): shapes and bodies, materials and their
//! catalogue, layer stacks and their etches, subpixel averaging and rasters. It names only the
//! core layer (photonoxide-core) below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod averaging;
mod etch;
pub mod geometry;
pub mod material;
pub mod raster;
pub mod stack;

// the core layer's modules, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, expr, units, verdict};

// and the one only the tests use
#[cfg(test)]
use photonoxide_core::parallel;

// the validation types, which the checks here name by photonoxide's path for them
// (`photonoxide::validation` re-exports `verdict`'s)
use photonoxide_core::verdict as validation;

//! # photonoxide-fdfd
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports this module at the
//! same path (`photonoxide::fdfd`).
//!
//! photonoxide's FDFD layer (docs/architecture.md): frequency-domain finite differences in 2D and
//! 3D. It names only the core, geometry and linear algebra layers below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod fdfd;

// the layers' modules below, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, traffic, units};
use photonoxide_geometry::{averaging, geometry, material, stack};
use photonoxide_linalg::{backend, eigen};

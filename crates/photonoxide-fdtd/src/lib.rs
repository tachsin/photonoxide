//! # photonoxide-fdtd
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports this module at the
//! same path (`photonoxide::fdtd`).
//!
//! photonoxide's FDTD layer (docs/architecture.md): the Yee scheme in 2D and 3D, on the CPU and,
//! with the `gpu` feature, the GPU. It names only the core, geometry, linear algebra and FDFD
//! layers below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod fdtd;

// the layers' modules below, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, lstsq, portable, units};
use photonoxide_fdfd::fdfd;
use photonoxide_geometry::{geometry, material, stack};

// and the one only the tests use
#[cfg(test)]
use photonoxide_core::traffic;

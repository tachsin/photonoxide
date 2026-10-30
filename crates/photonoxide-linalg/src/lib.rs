//! # photonoxide-linalg
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::backend`).
//!
//! photonoxide's linear algebra layer (docs/architecture.md): the dense kernels, the sparse
//! factorizations, the solver backends with the Krylov solvers, and the eigensolvers. It names
//! only the core layer (photonoxide-core) below it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub mod backend;
pub mod dense;
#[doc(hidden)]
pub mod eigen;
#[doc(hidden)]
pub mod sparse;

// `backend` keeps the Krylov solvers' and the iterative backend's modules private, as they were:
// their types that photonoxide re-exports (`photonoxide::fdfd::Convergence`) are named here, and
// the Krylov module's items, for photonoxide's FDFD, through a hidden module (hiding the module
// itself would hide those types)
pub use backend::iterative::CycleShape;
pub use backend::krylov::{BlockConvergence, Convergence, Stopping};
#[doc(hidden)]
pub mod krylov {
    pub use crate::backend::krylov::*;
}

// the core layer's modules, at the `crate::` paths the modules here were written with
use photonoxide_core::{Error, Result, traffic};

// and the one only the tests use
#[cfg(test)]
use photonoxide_core::portable;

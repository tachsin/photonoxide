//! # photonoxide
//!
//! Validated, fabrication-ready photonics for Rust: mode solvers, FDFD, FDTD, inverse design,
//! layout and PDKs, with a studio to watch every run live.
//!
//! **Pre-alpha:** the API is being built milestone by milestone. See the
//! [roadmap](https://github.com/tachsin/photonoxide/blob/main/ROADMAP.md) for what is planned.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod error;
pub mod geometry;
pub mod material;
pub mod run;
pub mod stack;
pub mod units;

pub use error::{Error, Result};
pub use num_complex::Complex64;

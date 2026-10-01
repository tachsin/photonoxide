//! # photonoxide
//!
//! Validated, fabrication-ready photonics for Rust: mode solvers, FDFD, FDTD, inverse design,
//! layout and PDKs, with a studio to watch every run live.
//!
//! **Alpha:** the API is being built milestone by milestone, and will change. See the
//! [roadmap](https://github.com/tachsin/photonoxide/blob/main/ROADMAP.md) for what is planned.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

mod eigen;
pub mod error;
pub mod geometry;
pub mod job;
pub mod material;
pub mod mode;
pub mod raster;
pub mod run;
pub mod stack;
#[cfg(feature = "studio")]
pub mod studio;
pub mod units;
pub mod validation;

pub use error::{Error, Result};
pub use num_complex::Complex64;

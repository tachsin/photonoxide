//! # photonoxide-core
//!
//! Internal; use [photonoxide](https://docs.rs/photonoxide), which re-exports these modules at
//! the same paths (`photonoxide::units`, `photonoxide::expr`, ...).
//!
//! The lowest layer of photonoxide (docs/architecture.md): units, expressions, the portable
//! maths, parallel collection, the error type, the validation verdicts, convergence and the
//! stop signal. It names nothing above itself.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod convergence;
pub mod error;
pub mod expr;
#[doc(hidden)]
pub mod lstsq;
pub mod parallel;
#[doc(hidden)]
pub mod portable;
pub mod stop;
pub mod traffic;
pub mod units;
pub mod verdict;

pub use error::{Error, Result};

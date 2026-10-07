//! # photonoxide
//!
//! Validated photonics for Rust: materials with provenance, mode solvers, 2D and 3D FDFD with
//! S-parameters and adjoints, components and circuits, and compact models, with a studio to
//! watch every run live. Every method is checked against an analytic solution or a published
//! result ([`validation`]).
//!
//! - [`material`]: dispersion models with their source, validity range and temperature, and a
//!   catalogue of photonic materials, every number read from its paper.
//! - [`mode`]: waveguide modes, from the exact slab and multilayers to full-vector finite
//!   differences of a cross-section, with bends, dispersion and loss.
//! - [`fdfd`]: frequency-domain finite differences in 2D and 3D on Yee's grid, with PMLs, mode
//!   ports, S-parameters and adjoint gradients.
//! - [`circuit`]: components with ports and S-matrices, connected into netlists, solved as one
//!   sparse system and differentiated by the circuit adjoint.
//! - [`compact`]: compact models by vector fitting, over parameters, and Touchstone files.
//! - [`run`] and [`job`]: jobs as TOML, JSON or YAML files, and runs recorded as events that
//!   replay exactly; the studio, the `photonoxide` program attached to each release, runs and
//!   shows them.
//! - [`bench`](mod@bench): fixed problems that time the solvers at a stated accuracy, which
//!   `photonoxide bench` runs.
//! - [`parallel`]: independent problems (a sweep's points) side by side, collected in order.
//!
//! **Alpha:** the API is being built milestone by milestone, and will change. FDTD, thermal and
//! electro-optic devices, inverse design, layout and PDKs are planned, not here yet: see the
//! [roadmap](https://github.com/tachsin/photonoxide/blob/main/ROADMAP.md).

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/logo-32.png"
)]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod backend;
pub mod bench;
pub mod circuit;
pub mod compact;
mod eigen;
pub mod error;
pub mod fdfd;
pub mod geometry;
pub mod job;
pub mod material;
pub mod mode;
pub mod parallel;
pub mod raster;
pub mod run;
mod sparse;
pub mod stack;
mod traffic;
pub mod units;
pub mod validation;

pub use error::{Error, Result};

/// photonoxide's version, e.g. `"0.2.0"`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub use num_complex::Complex64;

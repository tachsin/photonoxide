//! Compact models and Touchstone files: a device's S-matrix spectrum as a few numbers.
//!
//! - [`fit`]: vector fitting (B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052
//!   (1999), doi:10.1109/61.772353): sampled responses as rational functions with common poles,
//!   stable, with the fit's error.
//! - [`model`]: a component's S-matrix spectrum as a fitted rational model ([`CompactModel`]),
//!   evaluated at any wavelength in its band, its passivity checked; and a measured spectrum as a
//!   component ([`Measured`]).
//! - [`param`]: a model over the component's parameters ([`ParametricModel`]): rational in
//!   frequency, its numerator and denominator polynomial in the parameters.
//! - [`touchstone`]: Touchstone (`.sNp`) files, read and written, Version 1 and 2.0: measured
//!   and simulated S-parameters in the format microwave and photonics tools exchange them in,
//!   with the conversions between frequency and wavelength and between time conventions.
//!
//! [`CompactModel`], [`ParametricModel`] and [`Measured`] are circuit components
//! ([`crate::circuit::Component`]). The method's page is docs/methods/compact.md.
//!
//! ```
//! use photonoxide::circuit::{Component, SMatrix, Spectrum};
//! use photonoxide::compact::{CompactModel, Options};
//! use photonoxide::units::Wavelength;
//! use photonoxide::Complex64 as c64;
//!
//! // a resonance at 1.55 um, about 1 nm wide, sampled at 101 wavelengths
//! let k0 = std::f64::consts::TAU / 1.55;
//! let through = |w: f64| {
//!     let k = std::f64::consts::TAU / w;
//!     1.0 - 0.002 / c64::new(0.002, -(k - k0))
//! };
//! let wavelengths = (0..101)
//!     .map(|i| Wavelength::um(1.54 + 0.0002 * i as f64))
//!     .collect::<Result<Vec<_>, _>>()?;
//! let matrices = wavelengths
//!     .iter()
//!     .map(|w| SMatrix::from_fn(1, |_, _| through(w.to_um())))
//!     .collect();
//! let spectrum = Spectrum::new(vec!["o1".into()], wavelengths, matrices)?;
//!
//! let model = CompactModel::fit(&spectrum, &Options::new(2))?;
//! assert!(model.error().max < 1e-12);
//! let s = model.s_matrix(Wavelength::um(1.5501)?, &[])?;
//! assert!((s[(0, 0)] - through(1.5501)).norm() < 1e-12);
//! # Ok::<(), photonoxide::Error>(())
//! ```

pub(crate) mod checks;
pub mod fit;
pub mod model;
pub mod param;
pub mod touchstone;

pub use fit::{Delay, FitError, Options, Rational, Symmetry};
pub use model::{CompactModel, Measured, Passivity};
pub use param::{ParametricModel, Sample};

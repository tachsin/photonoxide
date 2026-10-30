//! Heat conduction on a waveguide's cross-section: −∇·(k∇T) = Q, steady, and in time,
//! ρc_p ∂T/∂t = ∇·(k∇T) + Q ([`Heat::transient`], [`Heat::response`], a pulse by
//! [`Heat::pulse`] and what it does to a phase-change film by [`Switch`]), per unit length along
//! the guide.
//!
//! A [`Heat`] problem is a [`Section`] (what fills the cross-section: shapes drawn with media,
//! or a [`Structure`](crate::stack::Structure)'s cut with its stack, etches and walls) on a
//! [`Grid`], each medium's thermal conductivity k in W/(m·K) ([`Tensor`]: isotropic, or
//! diagonal in the section's frame), the window's sides ([`Boundary`]: a fixed temperature, a
//! flux, or a convective film), heat sinks held at a temperature, and heaters that give a
//! power per unit length (W/m), or the Joule heat of a current ([`crate::electro::Current`]).
//! Its solution is a [`Field`] of temperatures at the cells' centres, with the heat each sink
//! takes and what leaves through each side; a 2D number, per metre of a straight section.
//!
//! Temperatures are in whatever unit the problem's are (K or °C); a rise above a sink at 0 is
//! a temperature difference.
//!
//! The method, shared with [`crate::electro`], is the static solver of docs/methods/static.md:
//! cell-centred finite volumes on a graded grid, exact for layered media on the grid's lines,
//! second order elsewhere, solved by photonoxide's multifrontal solver. Its validation cases
//! (`static/` in docs/validation.md) check it against closed forms: a slab and a two-layer
//! stack, a line source above an isothermal substrate by images, and the order of the error.
//!
//! ```
//! # mod photonoxide { pub use photonoxide_core::*; pub use photonoxide_geometry::*; pub use photonoxide_mode::*; pub use photonoxide_statics::electro; pub use photonoxide_fdfd::*; pub use photonoxide_circuit::*; pub use photonoxide_device::*; pub use num_complex::Complex64; } // the crates users name photonoxide
//! use photonoxide::geometry::{Point, Shape};
//! use photonoxide::thermal::{Boundary, Features, Grading, Grid, Heat, Section, Side, Tensor};
//! use photonoxide::units::Length;
//!
//! // 2 µm of oxide on an isothermal substrate, a 1 W/m heater 1 µm up
//! let mut section = Section::new("oxide");
//! let heater = Shape::rect(Point::um(0.0, 1.0), Length::um(0.5), Length::um(0.1))?;
//! let mut features = section.features().clone();
//! features.shape(&heater);
//! let grid = Grid::fitted([-20.0, 20.0], [0.0, 2.0], &Grading::new(0.5, 0.02, 1.2)?, &features)?;
//! let mut heat = Heat::new(section, grid);
//! heat.conductivity("oxide", Tensor::isotropic(1.4))
//!     .boundary(Side::Bottom, Boundary::Fixed(0.0))
//!     .heater(heater, 1.0);
//! let t = heat.solve()?;
//! // all the heat leaves through the substrate
//! assert!((t.outflow(Side::Bottom) - 1.0).abs() < 1e-9);
//! assert!(t.max().0 > 0.0);
//! # Ok::<(), photonoxide::Error>(())
//! ```
//!
//! The thermo-optic phase shifter on it ([`PhaseShifter`], [`Guide`], [`Optics`], [`Drive`]:
//! a heat solve's rise in a guide's mode) is here too.

// the statics crate's module (photonoxide-statics), and the shifter, which joins it to the mode
// solver and so sits in this, the device layer: one path for both, as before the split (#508)
#[doc(inline)]
pub use photonoxide_statics::thermal::*;

pub use crate::shifter::{Drive, Efficiency, Guide, Optics, PhaseShifter, StepResponses, Timing};
// what the tests took from the module's own imports when they sat in it
#[cfg(test)]
use crate::geometry::Shape;

// the checks and tests of the heat solver that use the shifter
#[path = "cases/thermal/correction_checks.rs"]
#[doc(hidden)]
pub mod correction_checks;
#[cfg(test)]
#[path = "cases/thermal/correction_tests.rs"]
mod correction_tests;
#[path = "cases/thermal/crosstalk_checks.rs"]
#[doc(hidden)]
pub mod crosstalk_checks;
#[path = "cases/thermal/rc_checks.rs"]
#[doc(hidden)]
pub mod rc_checks;
#[path = "cases/thermal/shifter_checks.rs"]
#[doc(hidden)]
pub mod shifter_checks;
#[cfg(test)]
#[path = "cases/thermal/shifter_tests.rs"]
mod shifter_tests;

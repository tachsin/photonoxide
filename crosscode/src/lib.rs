//! Other codes run as external programs on photonoxide's cases, for comparison: the cross-code
//! references of the static and RF solvers, as Meep and MPB are for the optics (ROADMAP 0.6,
//! decision 6; issue #422).
//!
//! Nothing of theirs is linked, built, shipped or read: a program the user has installed is
//! found ([`openems::OpenEms::find`]), given an input file this crate writes from the program's
//! documented format, run in a folder of its own, and its output files read. CI and the
//! released program never need one, and never build this crate; its tests that need one skip,
//! saying so, where it isn't found.
//!
//! - [`openems`]: openEMS (3D FDTD, GPL-3.0), found through `PHOTONOXIDE_OPENEMS` or `PATH`;
//!   its XML input, its runs with their progress, its probes' files.
//! - [`line`]: a uniform line's propagation constant and impedance from voltages and currents
//!   at planes along it, whatever its reflections.
//! - [`cpw`]: the shared case, a coplanar waveguide on a finite substrate in a shielding box,
//!   drawn on the same mesh lines for openEMS's FDTD and photonoxide's quasi-TEM line.
//!
//! The setup, the licences and the results are in docs/cross-code.md and
//! docs/validation-openems.md (`just openems` from the repository's root, where openEMS is
//! installed). A crate of its own, outside photonoxide's workspace: CI never builds or runs it.

#![forbid(unsafe_code)]

pub mod cpw;
pub mod line;
pub mod openems;

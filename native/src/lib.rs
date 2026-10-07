//! External libraries for photonoxide, found and loaded at run time: the one crate of
//! photonoxide's that calls C, and the only one allowed `unsafe`.
//!
//! Nothing is linked at build time and nothing is redistributed: a library the user has
//! installed is found ([`discover`]), loaded ([`Library`], [`load`]), its version read, and a
//! backend built on it is offered to photonoxide's registry only after its smoke test, a small
//! complex system solved and compared with photonoxide's own answer ([`offer`]). The plan is
//! `docs/plans/backends.md`.
//!
//! - **Discovery** looks, in order, at the caller's setting, the library's environment
//!   variables (photonoxide's own, as `PHOTONOXIDE_CUDSS`, then the vendor's), the conda
//!   environment, the vendor's install folders, its Python wheels' folders and the system's
//!   library paths, and records every candidate and what became of it.
//! - **Safety:** each call into C is one `unsafe` block that states what the C side requires.
//!   A library's errors become photonoxide's [`Error`](photonoxide::Error). An abort inside a
//!   library can't be caught in-process, so the benchmark runs backends in a child process.
//!
//! The libraries known so far are NVIDIA's ([`nvidia`]); their backends, and the others', are
//! each one's own issue (#175 to #190).

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::undocumented_unsafe_blocks)]

mod discovery;
mod library;
pub mod nvidia;
mod smoke;

pub use discovery::{Candidate, Discovery, Source, Spec, Status, discover, versioned};
pub use library::{Library, load};
pub use smoke::{TOLERANCE, offer, smoke_test};

/// What was found of one library.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Probe {
    /// Every candidate, and the one used.
    pub discovery: Discovery,
    /// The version read from the library loaded.
    pub version: Option<String>,
    /// What else the library says (the GPUs, their memory).
    pub details: Vec<String>,
}

impl std::fmt::Display for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = &self.discovery;
        match d.used() {
            Some(used) => write!(
                f,
                "{} {}: {} (from {})",
                d.library,
                self.version.as_deref().unwrap_or("(version unknown)"),
                used.path.display(),
                used.source
            )?,
            None => write!(f, "{}: {}", d.library, d.reason())?,
        }
        for detail in &self.details {
            write!(f, "\n  {detail}")?;
        }
        Ok(())
    }
}

/// Every library photonoxide knows how to look for, as found on this machine. Nothing is
/// registered yet: each library's backend registers itself here when it lands.
pub fn register_all() -> Vec<Probe> {
    nvidia::probe()
}

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
//! The libraries known so far are NVIDIA's ([`nvidia`]) and Intel's oneMKL ([`intel`]); their
//! backends, and the others', are each one's own issue (#175 to #190).

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::undocumented_unsafe_blocks)]

pub mod accelerate;
mod cuda;
mod cudss;
mod discovery;
mod gpu_multigrid;
mod gpu_qmr;
pub mod intel;
mod library;
pub mod mumps;
pub mod nvidia;
mod pardiso;
mod smoke;
pub mod superlu;

pub use accelerate::Accelerate;
pub use cudss::Cudss;
pub use discovery::{Candidate, Discovery, Source, Spec, Status, discover, versioned};
pub use gpu_qmr::GpuQmr;
pub use library::{Library, load};
pub use mumps::Mumps;
pub use pardiso::Pardiso;
pub use smoke::{TOLERANCE, offer, offer_iterative, smoke_test, smoke_test_iterative};
pub use superlu::SuperLu;

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

/// Every library photonoxide knows how to look for, as found on this machine. Each backend
/// whose library is found is offered to photonoxide's registry after its smoke test
/// ([`offer`]); one whose library isn't is registered as unavailable, with the reason.
pub fn register_all() -> Vec<Probe> {
    // the registry refuses only names it doesn't take, and "cudss" is one it takes
    let _ = match Cudss::load() {
        Ok(cudss) => offer(std::sync::Arc::new(cudss)).map(drop),
        Err(reason) => photonoxide::backend::register_unavailable("cudss", reason),
    };
    let _ = match GpuQmr::load() {
        Ok(qmr) => offer_iterative(std::sync::Arc::new(qmr)).map(drop),
        Err(reason) => photonoxide::backend::register_iterative_unavailable("cusparse", reason),
    };
    let _ = match Pardiso::load() {
        Ok(pardiso) => offer(std::sync::Arc::new(pardiso)).map(drop),
        Err(reason) => photonoxide::backend::register_unavailable("pardiso", reason),
    };
    let _ = match Mumps::load() {
        Ok(mumps) => {
            // its block low-rank factorization, a backend of its own
            let tolerance = std::env::var("PHOTONOXIDE_MUMPS_BLR")
                .ok()
                .and_then(|text| text.trim().parse().ok())
                .unwrap_or(mumps::BLR_TOLERANCE);
            let _ = match mumps.block_low_rank(tolerance) {
                Ok(blr) => offer(std::sync::Arc::new(blr)).map(drop),
                Err(reason) => photonoxide::backend::register_unavailable("mumps-blr", reason),
            };
            offer(std::sync::Arc::new(mumps)).map(drop)
        }
        Err(reason) => {
            let _ = photonoxide::backend::register_unavailable("mumps-blr", reason.clone());
            photonoxide::backend::register_unavailable("mumps", reason)
        }
    };
    let _ = match SuperLu::load() {
        Ok(superlu) => offer(std::sync::Arc::new(superlu)).map(drop),
        Err(reason) => photonoxide::backend::register_unavailable("superlu", reason),
    };
    // (off macOS it isn't listed at all: there is nothing a user could do about it)
    if cfg!(target_os = "macos") {
        let _ = match Accelerate::load() {
            Ok(accelerate) => offer(std::sync::Arc::new(accelerate)).map(drop),
            Err(reason) => photonoxide::backend::register_unavailable("accelerate", reason),
        };
    }
    let mut probes = nvidia::probe();
    probes.push(intel::probe());
    probes.push(mumps::probe());
    probes.push(superlu::probe());
    if cfg!(target_os = "macos") {
        probes.push(accelerate::probe());
    }
    probes
}

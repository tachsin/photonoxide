//! The checks and tests of the lower crates' modules that reach above their layer
//! (docs/architecture.md, #508): a check of the geometry that runs FDTD, a material's that
//! solves a mode, convergence's that drive the solvers. Their module's crate can't name the
//! solvers, so they live here, each in a module named after the one it checks, which sees that
//! module's items as the checks' `super` did when they sat inside it.
//!
//! The other modules' checks and tests that reach above their module stay in `src/cases/` too,
//! declared by their modules (`#[path]`), which are in this crate.

/// Convergence to a tolerance, driving the slab mode solver, 2D FDFD and eigenmode expansion.
pub(crate) mod convergence {
    #[allow(unused_imports)]
    use crate::convergence::*;

    pub(crate) mod checks;
}

/// The expressions' spline against the geometry's own (a test of the core crate's `expr` that
/// needs the geometry crate).
#[cfg(test)]
mod expr {
    mod spline_tests;
}

/// The bodies, checked by FDTD's smoothing and by FDFD.
pub(crate) mod geometry {
    #[allow(unused_imports)]
    use crate::geometry::*;

    pub(crate) mod body_checks;
    #[cfg(test)]
    mod body_tests;
}

/// The materials, checked by the mode solvers, FDFD and FDTD.
pub(crate) mod material {
    #[allow(unused_imports)]
    use crate::material::*;

    pub(crate) mod graded_checks;

    /// The phase-change materials, checked by the slab mode solvers.
    pub(crate) mod catalogue {
        #[allow(unused_imports)]
        use crate::material::catalogue::*;

        pub(crate) mod phase_change_checks;
    }

    /// The crystals' tensors, checked by the mode solvers.
    pub(crate) mod crystal {
        #[allow(unused_imports)]
        use crate::material::crystal::*;
        // what the tests took from the crystal module's own imports
        #[cfg(test)]
        use num_complex::Complex64;

        pub(crate) mod checks;
        #[cfg(test)]
        mod tests;
    }
}

/// The stacks' etches, checked by the full-vector mode solver and Hadley's.
pub(crate) mod stack {
    #[allow(unused_imports)]
    use crate::stack::*;

    pub(crate) mod checks;
}

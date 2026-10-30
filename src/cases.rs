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

/// The eigensolvers, checked on the mode solvers' problems.
pub(crate) mod eigen {
    #[allow(unused_imports)]
    use crate::eigen::*;

    pub(crate) mod checks;
}

/// The travelling-wave lines, checked with the Pockels cases' lithium niobate.
pub(crate) mod electro {
    #[allow(unused_imports)]
    use crate::electro::*;

    pub(crate) mod full_wave_checks;
    pub(crate) mod line_checks;
    #[cfg(test)]
    mod line_tests;
}

/// 2D and 3D FDFD, checked against the mode solvers' modes.
pub(crate) mod fdfd {
    #[allow(unused_imports)]
    use crate::fdfd::*;
    // what the checks and tests took from the module's own imports
    #[allow(unused_imports)]
    use crate::units::Wavelength;
    #[allow(unused_imports)]
    use num_complex::Complex64 as c64;

    pub(crate) mod checks;
    pub(crate) mod crystal_checks;
    #[cfg(test)]
    mod crystal_tests;
    pub(crate) mod fill_checks;
    #[cfg(test)]
    mod tests;

    /// The fill of 2D FDFD's cells.
    #[cfg(test)]
    mod fill {
        #[allow(unused_imports)]
        use crate::fdfd::fill::*;
        // what the tests took from the module's own imports
        use crate::averaging::{Averaging, Plane};
        use crate::backend::Choice;
        use crate::fdfd::*;
        use crate::units::Wavelength;

        mod tests;
    }

    /// 3D FDFD.
    pub(crate) mod three {
        #[allow(unused_imports)]
        use crate::fdfd::three_items::*;
        // what the checks and tests took from the module's own imports
        #[allow(unused_imports)]
        use crate::units::Wavelength;
        #[allow(unused_imports)]
        use num_complex::Complex64 as c64;
        // the multigrid, ports and iterative modules are private to their crate: what the
        // checks and tests use of them
        #[allow(unused_imports)]
        use crate::fdfd::three_items::{
            iterative_items as iterative, multigrid_items as multigrid, ports_items as ports,
        };

        pub(crate) mod block_checks;
        pub(crate) mod checks;
        pub(crate) mod port_checks;
        #[cfg(test)]
        mod port_tests;
        pub(crate) mod recycle_checks;
        pub(crate) mod stencil_checks;
        #[cfg(test)]
        mod tests;
    }
}

/// FDTD, checked against the mode solvers' Bloch modes and the crystals' slab.
pub(crate) mod fdtd {
    #[allow(unused_imports)]
    use crate::fdtd::*;
    // what the checks took from the module's own imports
    use crate::fdfd::{Axis, Edges, Grid3d};
    use crate::units::Frequency;
    use num_complex::Complex64 as c64;

    pub(crate) mod bloch_checks;
    pub(crate) mod crystal_checks;
    #[cfg(test)]
    mod crystal_tests;
    #[cfg(test)]
    mod media_tests;
}

/// Eigenmode expansion, checked against 2D FDFD and the validation report's rows.
pub(crate) mod eme {
    #[allow(unused_imports)]
    use crate::eme::*;
    // what the tests took from the module's own imports
    #[cfg(test)]
    use crate::mode::Polarization;
    #[cfg(test)]
    use crate::units::{Length, Wavelength};
    #[cfg(test)]
    use num_complex::Complex64 as c64;

    pub(crate) mod checks;
    #[cfg(test)]
    mod tests;
}

/// The mode solvers' perturbations, checked with the static solver's fields, and the roughness
/// loss, checked through the devices.
pub(crate) mod mode {
    #[allow(unused_imports)]
    use crate::mode::*;

    pub(crate) mod thermo_checks;

    /// The Pockels effect, checked with the static solver's field.
    pub(crate) mod pockels {
        #[allow(unused_imports)]
        use crate::mode::pockels::*;

        pub(crate) mod checks;
    }

    /// The roughness loss, through the strip waveguide device.
    pub(crate) mod roughness {
        #[allow(unused_imports)]
        use crate::mode::roughness::*;
        // what the tests took from the module's own imports
        #[cfg(test)]
        use crate::mode::vector::{Boundary, CrossSection};
        #[cfg(test)]
        use crate::units::{Length, Wavelength};
        #[cfg(test)]
        use num_complex::Complex64 as c64;
        #[cfg(test)]
        use std::f64::consts::PI;

        #[cfg(test)]
        mod tests;
    }
}

/// The multifrontal solver, measured on the benchmark's systems.
#[cfg(test)]
mod sparse {
    mod multifrontal {
        #[allow(unused_imports)]
        use crate::sparse::multifrontal::*;
        // what the measurement took from the module's own imports
        use faer::sparse::{SparseColMat, Triplet};
        use num_complex::Complex64 as c64;
        use std::sync::Arc;

        mod measure;
    }
}

/// The static solver, checked against the full-vector mode solver.
pub(crate) mod statics {
    #[allow(unused_imports)]
    use crate::statics::*;

    pub(crate) mod checks;
    #[cfg(test)]
    mod tests;
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
        // `phase_change` is private to its crate: what the checks use of it
        use crate::material::catalogue::phase_change_items as phase_change;

        pub(crate) mod phase_change_checks;
    }

    /// The crystals' tensors, checked by the mode solvers.
    pub(crate) mod crystal {
        // the crystal module is private to its crate: its public types
        use crate::material::{CrystalAxis, Orientation};
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

//! The checks of the statics and mode crates' modules that the devices use and that join the two
//! layers (the TFLN modulator's line and Pockels section): declared here, in the lowest crate
//! that sees both, each in a module named after the one it checks, which sees that module's
//! items as the checks' `super` did when they sat inside it. photonoxide's `cases` re-exports
//! them at its paths for them.

/// The travelling-wave lines, checked with the Pockels cases' lithium niobate.
pub mod electro {
    #[allow(unused_imports)]
    use crate::electro::*;

    pub mod line_checks;
}

/// The mode solvers' Pockels effect, checked with the static solver's field.
pub mod mode {
    /// The Pockels effect.
    pub mod pockels {
        #[allow(unused_imports)]
        use crate::mode::pockels::*;

        pub mod checks;
    }
}

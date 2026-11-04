//! The rules (#544) of eigenmode expansion's cases (#432): the one-mode step and the uniform
//! media's step against closed forms, a lossless structure's reciprocity and power at round-off,
//! and a stepped taper against 2D FDFD with the expansion's own convergence in its number of
//! modes.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "eme/step-one-mode",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the same mode-matching on the exact slab modes and integrals in closed form, against the solver's roots and Gauss–Legendre sums; the walls 3 µm from the core move the modes by under 1e-14: round-off",
            ),
        ),
        (
            "eme/step-fresnel-te",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "each mode meets only its own, with Fresnel's coefficient at its angle exactly: round-off",
            ),
        ),
        (
            "eme/step-fresnel-tm",
            Rule::new(Absolute, RoundOff, 1e-12, Exact, "as eme/step-fresnel-te"),
        ),
        (
            "eme/lossless-reciprocity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "S symmetric for any number of modes, by construction: round-off over five interfaces' products",
            ),
        ),
        (
            "eme/lossless-power",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "complex power conserved at each interface for any number of modes: round-off",
            ),
        ),
        (
            "eme/taper-steps-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the midpoint staircase's second order, within 0.1: the observed order falls from 2.18 (10 to 40 sections) to 2.07 (20 to 80)",
            ),
        ),
        (
            "eme/taper-modes",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "the convergence gate's estimate of the mode count's error: the gate a number of the expansion passes before eme/taper-fdfd compares it, 1e-4",
            ),
        ),
        (
            "eme/taper-fdfd",
            Rule::new(
                Relative,
                Discretisation,
                2e-4,
                Noted,
                "FDFD's grid limit by Richardson (its estimate at 2.5 nm 5.7e-5) against the expansion, whose own error is its change from 160 to 200 modes: three times the larger, rounded up, 2e-4",
            ),
        ),
        (
            "eme/taper-adaptive",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Noted,
                "the tolerance the adaptive steps are driven to, on S's fundamental entries (at most 1): their true error against the equal steps' Richardson limit, whose own error is its change from 2560 steps",
            ),
        ),
    ]
}

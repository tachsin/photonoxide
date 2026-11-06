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
        (
            "eme/wavelength-taper",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the estimate's claim, as sweep/estimate-bounds-error: the interpolant's true error at most its estimate, a ratio of at most 1; the case measures that ratio itself",
            )
            .varying(),
        ),
        (
            "eme/wavelength-mmi",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "as eme/wavelength-taper",
            )
            .varying(),
        ),
        (
            "eme/wavelength-reciprocity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a linear relation every solve keeps, kept by entries that share a denominator: round-off, 1e-12 as eme/lossless-reciprocity",
            )
            .varying(),
        ),
        (
            "eme/wavelength-power",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the bound the entries' estimates give a column's power: a ratio of at most 1 wherever they bound the entries' errors",
            )
            .varying(),
        ),
        (
            "eme/3d-orthonormal",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "an exact discrete identity, held to the eigensolver's own tolerance on each pair's residual, 1e-10",
            ),
        ),
        (
            "eme/3d-ports",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the same eigenproblem by two eigensolvers, each pair held to a residual of 1e-10",
            ),
        ),
        (
            "eme/3d-hadley",
            Rule::new(
                Relative,
                Discretisation,
                1.2e-5,
                Noted,
                "Hadley's series to ±1e-8; our limit's error bounded by the finest grid's own estimate, at most 3.8e-6 relative (problem 4's convex corner): three times it, rounded up",
            ),
        ),
        (
            "eme/3d-symmetry",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same equations restricted to a symmetry: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-fresnel",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "each mode meets only its own, with Fresnel's coefficient at its discrete angle exactly: round-off of a dense eigensolve of 236 unknowns whose β² reach 1e3 µm⁻², 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-lossless-reciprocity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "S symmetric for any number of modes, by construction: round-off over products of 728 × 728 matrices, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-lossless-power",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "with every mode the interfaces are exact: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-extruded-separable",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same discrete eigenproblem in both: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-extruded-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "Yee's second order, within 0.1 as eme/taper-steps-order",
            ),
        ),
        (
            "eme/3d-extruded-slab",
            Rule::new(
                Relative,
                Discretisation,
                3e-4,
                Noted,
                "three times the finest grid's own error by Richardson (9.3e-5 of the modulus of S11), rounded up: that error bounds the limit's",
            ),
        ),
        (
            "eme/3d-taper-fdfd",
            Rule::new(
                Absolute,
                Discretisation,
                4e-5,
                Noted,
                "three times FDFD's own error, the change from PMLs 0.5 µm thick and leads 1 µm long to 1 and 1.5 µm (1.1e-5 on the transmission; the PML alone from 1 to 2 µm moves it 4.6e-7), rounded up; the expansion with every mode is exact across the plane's equations, its own error its fit's residual",
            ),
        ),
        (
            "eme/3d-cut-extruded",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same planes, their lengths differences of the outlines' x: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-cut-taper-exact",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same 33 planes over the same lengths, the steps' merged: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-cut-taper-fdfd",
            Rule::new(
                Absolute,
                Discretisation,
                4e-4,
                Noted,
                "three times FDFD's own error, rounded up: its limit moves 1.3e-4 with its coarsest step (0.05 µm) in the fit or out, its sub-cell sampling of the sloped wall along x scattering its values by about 1e-4 about their trend (the PML from 1 to 2 µm moves it 1.4e-5); the expansion cut exactly has no step error, its own error the fit's residual",
            ),
        ),
        (
            "eme/3d-cut-mmi-fdfd",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Noted,
                "three times FDFD's own error, rounded up: the PML from 1 to 2 µm moves its transmission 6.3e-5 at 25 nm (its limit moves 3.3e-5 with its coarsest step in the fit or out); the expansion with every mode is exact on the plane's equations, and the MMI's three cross-sections have no step error",
            ),
        ),
        (
            "eme/3d-cut-mmi-reflection",
            Rule::new(
                Absolute,
                Discretisation,
                1.4e-5,
                Noted,
                "three times FDFD's own error, rounded up: its limit moves 4.5e-6 with its coarsest step in the fit or out (the PML from 1 to 2 µm, 7.4e-7)",
            ),
        ),
        (
            "eme/3d-cut-mmi-reciprocity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "S symmetric for any number of modes, by construction: round-off over products of 728 × 728 matrices, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-cut-mmi-power",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "with every mode the interfaces are exact: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
    ]
}

//! The rules (#544) of eigenmode expansion's cases (#432): the one-mode step and the uniform
//! media's step against closed forms, a lossless structure's reciprocity and power at round-off,
//! and a stepped taper against 2D FDFD with the expansion's own convergence in its number of
//! modes; periodic sections' Bloch modes against the two-layer closed form and N periods by
//! repeated squaring at round-off; bends against the exact bent slab within our own error, two
//! bends against one at round-off.

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
            "eme/bloch-quarter-wave",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "an exact identity for one uniform mode a layer: round-off over a 2 × 2 eigenproblem and two Fresnel steps, 1e-12 as eme/step-fresnel-te",
            ),
        ),
        (
            "eme/bloch-quarter-wave-modes",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "as eme/bloch-quarter-wave, each mode its own stack: round-off, relative to the magnitude of cos KΛ where it is above 1 (an evanescent mode's grows as its decay per period)",
            ),
        ),
        (
            "eme/bloch-band-edges",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "bisection to neighbouring floats on a function at round-off whose slope at the edges is about 1: round-off, 1e-12",
            ),
        ),
        (
            "eme/grating-squaring",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the same star products in another order: round-off, 1e-12 as eme/lossless-reciprocity",
            ),
        ),
        (
            "eme/bloch-grating-lossless",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "power conserved at each interface for any number of modes in a closed window, so a propagating Bloch mode keeps its λ on the unit circle: round-off, 1e-12 as eme/lossless-power",
            ),
        ),
        (
            "eme/bloch-grating-pml-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "a weak PML's first order in its strength, within 0.1 as eme/taper-steps-order: 1.00 with 20 and 40 modes at 0.0125 and 0.00625, 0.90 and 1.06 between 0.2 and 0.1, where the next order shows",
            ),
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
        (
            "eme/3d-drawing-extruded",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same planes as eme/3d-cut-extruded's, the job's S copied from the library's: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-drawing-taper",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same 33 planes over the same lengths: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/3d-drawing-gds",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same strips from whole nanometres, so the same planes as eme/3d-drawing-extruded's: round-off, 1e-10 as eme/3d-orthonormal",
            ),
        ),
        (
            "eme/bend-index",
            Rule::new(
                Absolute,
                Discretisation,
                7e-10,
                Noted,
                "three times our own error, rounded up: the largest of Richardson's limit's change to the next halving's (2.2e-10, TM at 2 µm), its change with a PML twice as strong (2.2e-10) and the exact bend's own (its Runge–Kutta steps' and its inner start's, 1.9e-11)",
            ),
        ),
        (
            "eme/bend-loss",
            Rule::new(
                Absolute,
                Discretisation,
                9e-5,
                Noted,
                "three times our own error relative to the loss, rounded up: the largest, a PML twice as strong moving Im n_eff by 7.8e-12 of 2.9e-7 (TM at 5 µm), which is the error measured",
            ),
        ),
        (
            "eme/bend-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the midpoint staircase's second order, within 0.1 as eme/taper-steps-order",
            ),
        ),
        (
            "eme/bend-halves",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a joint between a section's own modes is the identity to round-off: 1e-12 as eme/grating-squaring",
            ),
        ),
        (
            "eme/bend-no-angle",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the truncation of two bases, which falls with the modes: the expansion's mode-count gate, 1e-4 as eme/taper-modes",
            ),
        ),
        (
            "eme/bend-joint",
            Rule::new(
                Absolute,
                Discretisation,
                4e-9,
                Noted,
                "three times our own error, rounded up: the exact field's Runge–Kutta steps' (1.3e-9 at 2 µm TE, from steps twice as long), above the staircase's limit's change to the next halving's (3.4e-12) and the departure's change with a PML twice as strong (6.1e-14)",
            ),
        ),
        (
            "eme/bend-joint-leaky",
            Rule::new(
                Absolute,
                Discretisation,
                3e-9,
                Noted,
                "three times our own error, rounded up: the largest, TM at 2 µm, the staircase's limit's change to the next halving's and the departure's change with a PML twice as strong (both 6.9e-10), above the exact field's Runge–Kutta steps'",
            ),
        ),
        (
            "eme/beam-fresnel",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "each mode is a plane wave at its own angle and meets the step with Fresnel's coefficient exactly (eme/step-fresnel-te), and the modes sample the beam's angular spectrum, whose sum and integral differ by its images in the walls, exp(-89) here: round-off, 1e-12 as eme/step-fresnel-te",
            ),
        ),
        (
            "eme/beam-film",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the same modes through two interfaces and the film's propagation, each mode on its own: round-off, 1e-12 as eme/beam-fresnel",
            ),
        ),
        (
            "eme/beam-slab-coupling",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the expansion's fundamental mode is the exact slab's to exp(-q d) with the walls 15 µm from the core (q about 10 per µm), and both integrals are Gauss-Legendre sums to round-off: round-off, 1e-12",
            ),
        ),
    ]
}

//! The rules (#544) of the mode solvers', the thermo-optic and the electro-optic (Pockels)
//! cases, by the case's id. A published value far more precise than its case's tolerance (Hadley's
//! series to 1e-8, Chilwell and Hodgkinson's tables to five and six decimals, CAMFR's seven
//! digits) leaves our own grid's error as what the case measures: their basis is the grid's.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- slabs and strips against the book ----
        (
            "mode/slab-te-book",
            Rule::new(
                Absolute,
                Reference,
                5e-4,
                Exact,
                "the book's three decimals: half a unit of the third",
            ),
        ),
        (
            "mode/slab-tm-book",
            Rule::new(
                Absolute,
                Reference,
                5e-4,
                Exact,
                "the book's three decimals: half a unit of the third",
            ),
        ),
        (
            "mode/vector-slab-limit-te",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the full-vector scheme's second order against the exact slab, tested at 20, 10 and 5 nm",
            ),
        ),
        (
            "mode/vector-slab-limit-tm",
            Rule::new(Absolute, Discretisation, 1e-4, Measured, "as the TE case's"),
        ),
        (
            "mode/strip-book",
            Rule::new(
                Absolute,
                Reference,
                3e-3,
                Stated(9e-4, "against 2.5 nm, first order at the convex corners"),
                "Lumerical MODE's 20 nm mesh, to about 1e-3 by the book's Fig. 3.9, and its three decimals",
            ),
        ),
        (
            "mode/strip-group-index-book",
            Rule::new(
                Absolute,
                Reference,
                2e-2,
                Noted,
                "the plot read to ±0.005, and the book's 20 nm mesh",
            ),
        ),
        (
            "mode/slab-group-index",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "differences over ±2 nm against the exact field's weighting: their error, second order in the step",
            ),
        ),
        (
            "mode/eim-strip-book",
            Rule::new(
                Absolute,
                Reference,
                1e-3,
                Exact,
                "the book's three decimals, and its lateral slab's own rounding (2.488558)",
            ),
        ),
        // ---- Hadley's problems: his series to 1e-8, so what remains is our grid's ----
        (
            "mode/hadley-box-low",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "Hadley's series to ±1e-8: our 12.5 nm grid's error, about first order at convex corners",
            ),
        ),
        (
            "mode/hadley-box-high",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "as mode/hadley-box-low",
            ),
        ),
        (
            "mode/hadley-corner-low",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "Hadley's series to ±2e-9: our 12.5 nm grid's error, 1.4 to 1.8 at concave corners",
            ),
        ),
        (
            "mode/hadley-corner-high",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "as mode/hadley-corner-low",
            ),
        ),
        (
            "mode/hadley-uniform-box",
            Rule::new(
                Absolute,
                Discretisation,
                1e-9,
                Measured,
                "Hadley's sixth-order equations at 250 nm (the standard scheme's error there 1.6e-4)",
            ),
        ),
        (
            "mode/hadley-uniform-order",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "his equations' sixth order, within 0.2",
            ),
        ),
        (
            "mode/hadley-interface",
            Rule::new(
                Absolute,
                Discretisation,
                5e-9,
                Measured,
                "his interface equations' fifth order on this grid (the standard scheme's error 5.4e-6)",
            ),
        ),
        (
            "mode/hadley-interface-order",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "his interface equations' fifth order, diluted by a line of interface nodes: sixth within 0.3",
            ),
        ),
        (
            "mode/hadley-interface-turned",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a vertical interface is a horizontal one with x and y exchanged: round-off",
            ),
        ),
        (
            "mode/hadley-corners-box-low",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "Hadley's ±1e-8: our 7.8 nm grid's error with his corner equations (the standard scheme's 3.8e-5)",
            ),
        ),
        (
            "mode/hadley-corners-box-high",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "as mode/hadley-corners-box-low (the standard scheme's 9.0e-6)",
            ),
        ),
        (
            "mode/hadley-corners-impinged-low",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "Hadley's ±2e-9: our 7.8 nm grid's error with his corner equations (the standard scheme's 1.4e-5)",
            ),
        ),
        (
            "mode/hadley-corners-impinged-high",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "Hadley's ±5e-9: as mode/hadley-corners-impinged-low (the standard scheme's 1.6e-5)",
            ),
        ),
        (
            "mode/hadley-corners-order",
            Rule::new(
                Absolute,
                Order,
                0.25,
                Measured,
                "second order with his corner equations, where the standard scheme's is about first, within 0.25",
            ),
        ),
        // ---- multilayers and Chilwell and Hodgkinson's tables ----
        (
            "mode/multilayer-bound-chilwell",
            Rule::new(
                Absolute,
                Reference,
                5e-7,
                Exact,
                "Table 3's six decimals: half a unit of the sixth",
            ),
        ),
        (
            "mode/multilayer-leaky-chilwell",
            Rule::new(
                Absolute,
                Reference,
                1.5e-5,
                Exact,
                "Table 2's five decimals: one value a unit off in the last place (1.1e-5), the others ours rounded",
            ),
        ),
        (
            "mode/multilayer-power-chilwell",
            Rule::new(
                Absolute,
                Reference,
                6e-2,
                Exact,
                "Table 3's powers to 0.1 %, in percentage points",
            ),
        ),
        (
            "mode/multilayer-fresnel",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a single interface's transfer matrix is Fresnel's: round-off",
            ),
        ),
        (
            "mode/multilayer-bragg",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a quarter-wave stack's closed form from its admittance: round-off",
            ),
        ),
        (
            "mode/multilayer-derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-9,
                Measured,
                "the exact derivatives against five-point differences, whose error is O(h⁴) at h = 1e-4",
            ),
        ),
        (
            "mode/multilayer-quarter-wave-stationary",
            Rule::new(
                Absolute,
                RoundOff,
                1e-11,
                Exact,
                "a quarter-wave stack's reflectance is stationary in its thicknesses: zero to round-off",
            ),
        ),
        (
            "mode/slab-fd-chilwell",
            Rule::new(
                Absolute,
                Discretisation,
                2e-6,
                Measured,
                "Table 3's six decimals (±5e-7): the rest the finite-difference scheme's second order",
            ),
        ),
        // ---- PMLs and leaky waves ----
        (
            "mode/pml-soi-leakage-te",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "the PML's and the grid's error in the leakage, against the exact multilayer",
            ),
        ),
        (
            "mode/pml-soi-leakage-tm",
            Rule::new(Relative, Discretisation, 1e-3, Measured, "as the TE case's"),
        ),
        (
            "mode/pml-leaky-chilwell",
            Rule::new(
                Absolute,
                Discretisation,
                5e-5,
                Measured,
                "Table 2's five decimals (±5e-6): the rest our 2.5 nm grid's and the PML's",
            ),
        ),
        (
            "mode/leaky-wire-bienstman",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "CAMFR's and the aperiodic Fourier modal method's seven digits: our extrapolated grids' error (order 0.67, the corners')",
            ),
        ),
        (
            "mode/leaky-wire-bienstman-loss",
            Rule::new(
                Absolute,
                Discretisation,
                6e-2,
                Measured,
                "CAMFR's 2.9135 and the aperiodic method's 2.91348: our extrapolated leakage's error, in 1e-8",
            ),
        ),
        // ---- bends ----
        (
            "mode/bend-slab-te",
            Rule::new(
                Absolute,
                Discretisation,
                3e-5,
                Measured,
                "second order against Harris's exact bent slab",
            ),
        ),
        (
            "mode/bend-slab-te-loss",
            Rule::new(
                Relative,
                Discretisation,
                2e-3,
                Measured,
                "the bend's loss against the exact bent slab, the PML outside the turning point",
            ),
        ),
        (
            "mode/bend-slab-tm",
            Rule::new(
                Absolute,
                Discretisation,
                2e-3,
                Measured,
                "an isotropic medium scaled for E in the bend's plane, where the exact one is anisotropic: its error falls as the radius grows",
            ),
        ),
        (
            "mode/bend-marcuse",
            Rule::new(
                Relative,
                Model,
                0.1,
                Exact,
                "Marcuse's large-radius approximation, its deviation falling as 1/R (0.084 at 120 µm)",
            ),
        ),
        // ---- fields, Marcatili, graded index ----
        (
            "mode/fields-butt-coupling",
            Rule::new(
                Absolute,
                Discretisation,
                5e-5,
                Measured,
                "the modes' fields on the grid against the TE slabs' closed-form overlap, 0.994662",
            ),
        ),
        (
            "mode/marcatili-closed-form",
            Rule::new(
                Relative,
                Model,
                5e-2,
                Exact,
                "Marcatili: 'within a few percent of the exact value' there",
            ),
        ),
        (
            "mode/marcatili-vector",
            Rule::new(
                Absolute,
                Model,
                5e-4,
                Noted,
                "Marcatili's equations meet the full-vector mode within 4e-5 at B = 3, where his approximation holds",
            ),
        ),
        (
            "mode/graded-parabolic",
            Rule::new(
                Absolute,
                Discretisation,
                1e-7,
                Measured,
                "the planar scheme against the harmonic oscillator's exact modes",
            ),
        ),
        (
            "mode/graded-parabolic-order",
            Rule::new(
                Absolute,
                Order,
                0.25,
                Measured,
                "second order for a smooth medium, within 0.25",
            ),
        ),
        // ---- the eigensolvers ----
        (
            "mode/contour-uniform-box",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "contour integrals capture a multiplicity whole: the eigenvalues to round-off",
            ),
        ),
        (
            "mode/contour-dense",
            Rule::new(
                Absolute,
                RoundOff,
                2e-9,
                Exact,
                "two solvers' backward errors, each near rounding on a far-from-normal matrix, leave 6e-10 between them",
            ),
        ),
        (
            "mode/contour-strip-shift-invert",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the same matrix by shift-and-invert Arnoldi, its residual below 1e-9",
            ),
        ),
        (
            "mode/contour-leaky-shift-invert",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "the same non-Hermitian matrix by shift-and-invert, each to its residual",
            ),
        ),
        (
            "mode/contour-chilwell-bound",
            Rule::new(
                Absolute,
                Discretisation,
                2e-6,
                Measured,
                "Table 3's six decimals (±5e-7): the rest our grid's",
            ),
        ),
        (
            "mode/contour-chilwell-leaky",
            Rule::new(
                Absolute,
                Discretisation,
                2e-5,
                Measured,
                "Table 2's five decimals (±5e-6): the grid adds about 1e-5 at m = 7",
            ),
        ),
        (
            "mode/contour-exponential",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "exponential convergence in the points: 1, within 0.2 (a power of N would be 1/2)",
            ),
        ),
        (
            "mode/contour-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same computation on any thread: no difference",
            ),
        ),
        (
            "mode/contour-sakurai-sugiura",
            Rule::new(
                Absolute,
                Reference,
                1e5,
                Exact,
                "their ratio from two printed digits each, 1.52e6 to 1.72e6",
            ),
        ),
        (
            "mode/krylov-schur-box",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "a restart keeps each wanted direction, a double eigenvalue twice: round-off",
            ),
        ),
        (
            "mode/krylov-schur-dense",
            Rule::new(
                Absolute,
                RoundOff,
                3e-8,
                Exact,
                "the eigenvalues' condition times the backward error leaves up to 1.4e-8 between the two solvers",
            ),
        ),
        (
            "mode/krylov-schur-growing",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "both accept an eigenpair at a residual of 1e-9",
            ),
        ),
        (
            "mode/krylov-schur-chilwell",
            Rule::new(
                Absolute,
                Discretisation,
                2e-6,
                Measured,
                "Table 3's six decimals (±5e-7): the rest our grid's",
            ),
        ),
        (
            "mode/krylov-schur-convergence",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "the error goes as the residual squared: 2, within 0.3",
            ),
        ),
        (
            "mode/krylov-schur-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "each step in a fixed order on one thread: no difference",
            ),
        ),
        (
            "mode/every-guided-mode",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "the same modes by Krylov–Schur, its residual below 1e-9",
            ),
        ),
        (
            "mode/every-guided-mode-chilwell",
            Rule::new(
                Absolute,
                Discretisation,
                2e-6,
                Measured,
                "Table 3's six decimals (±5e-7): the rest our grid's",
            ),
        ),
        // ---- temperature ----
        (
            "thermo/slab-first-order",
            Rule::new(
                Absolute,
                Order,
                0.05,
                Measured,
                "first-order perturbation leaves an error of second order in ΔT: 2, within 0.05",
            ),
        ),
        (
            "thermo/slab-vector",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "the full-vector scheme against the slab's closed form at 292.5 and 293.5 K",
            ),
        ),
        (
            "thermo/slab-vector-order",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "the solver's second order, within 0.3",
            ),
        ),
        (
            "thermo/wire-perturbation",
            Rule::new(
                Relative,
                Model,
                5e-3,
                Stated(2e-6, "the re-solves, 10 against 5 nm"),
                "first-order perturbation against the re-solve: the field's singular corners converge slowly in a cell's overlap",
            ),
        ),
        (
            "thermo/dwivedi-470",
            Rule::new(
                Absolute,
                Reference,
                0.184,
                Noted,
                "the plot's error bar and reading, and Li's ±0.15e-4 per K carried through the wire, in 1e-4 per K",
            ),
        ),
        (
            "thermo/dwivedi-602",
            Rule::new(Absolute, Reference, 0.182, Noted, "as thermo/dwivedi-470"),
        ),
        (
            "thermo/dwivedi-805",
            Rule::new(Absolute, Reference, 0.179, Noted, "as thermo/dwivedi-470"),
        ),
        (
            "thermo/heater-field",
            Rule::new(
                Relative,
                Model,
                1e-2,
                Exact,
                "first-order perturbation against the re-solve of the warmed cross-section, on the same grid",
            ),
        ),
        // ---- Pockels ----
        (
            "pockels/bulk-x-cut",
            Rule::new(
                Relative,
                Model,
                1e-4,
                Exact,
                "the index ellipsoid to first order in E: second order left at 0.1 V/µm",
            ),
        ),
        (
            "pockels/bulk-z-cut",
            Rule::new(Relative, Model, 1e-4, Exact, "as pockels/bulk-x-cut"),
        ),
        (
            "pockels/bulk-y-cut",
            Rule::new(Relative, Model, 1e-4, Exact, "as pockels/bulk-x-cut"),
        ),
        (
            "pockels/plates-x-cut",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "first-order perturbation on the 5 nm full-vector grid against the slab solved again exactly",
            ),
        ),
        (
            "pockels/plates-z-cut",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "as pockels/plates-x-cut",
            ),
        ),
        (
            "pockels/resolve-tfln",
            Rule::new(
                Relative,
                Model,
                2e-3,
                Exact,
                "first-order perturbation against the re-solve, on the same 20 nm cells",
            ),
        ),
        (
            "pockels/resolve-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the perturbation's error second order in Δε, so first relative to Δn_eff: 1, within 0.1",
            ),
        ),
        (
            "pockels/static-order",
            Rule::new(
                Absolute,
                Order,
                0.5,
                Measured,
                "about first at the electrodes' edges, as the static solver's at a thin strip (0.82): 1, within 0.5",
            ),
        ),
        (
            "pockels/optical-convergence",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "the grid study's estimate at 10 nm",
            ),
        ),
        (
            "pockels/overlap-gamma",
            Rule::new(
                Relative,
                Model,
                2e-2,
                Exact,
                "the papers' overlap, scaled by n_eff/n_e, against the perturbation on the same grids: E's other components and r₁₃ left out",
            ),
        ),
        (
            "pockels/wang-2018",
            Rule::new(
                Relative,
                Reference,
                0.15 / 1.8,
                Stated(5e-3, "its grids gated at 0.5 %"),
                "r₃₃'s ±0.9 pm/V, the sidewall from 60 to 90°, and the paper's two digits",
            ),
        ),
        (
            "pockels/zhang-2022",
            Rule::new(
                Relative,
                Reference,
                0.15 / 2.3,
                Stated(5e-3, "its grids gated at 0.5 %"),
                "r₃₃'s ±0.9 pm/V, the sidewall from 60 to 90°, and the paper's two digits",
            ),
        ),
        (
            "pockels/valdez-2023",
            Rule::new(
                Relative,
                Reference,
                0.05 / 0.8,
                Stated(5e-3, "its grids gated at 0.5 %"),
                "the paper's one digit, 0.8 V·cm: half a unit of it",
            ),
        ),
        (
            "pockels/bto-angle",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the tensor turned in closed form: round-off",
            ),
        ),
        (
            "pockels/bto-deng",
            Rule::new(Absolute, Reference, 1.0, Exact, "their last digit, in pm/V"),
        ),
        (
            "pockels/bto-eltes",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "their two digits: half a unit of the second, in V·cm",
            ),
        ),
        (
            "pockels/litao3-x-cut",
            Rule::new(
                Relative,
                Model,
                1e-4,
                Exact,
                "the index ellipsoid to first order in E, as pockels/bulk-x-cut",
            ),
        ),
        (
            "pockels/bto-abel-response",
            Rule::new(
                Relative,
                Reference,
                0.1 / 3.4,
                Stated(5e-3, "its grids gated at 0.5 %"),
                "their two digits and their 33 nm FDTD grid, 3 %",
            ),
        ),
        (
            "pockels/bto-abel",
            Rule::new(
                Relative,
                Reference,
                0.015 / 0.45,
                Stated(5e-3, "its grids gated at 0.5 %"),
                "as pockels/bto-abel-response",
            ),
        ),
    ]
}

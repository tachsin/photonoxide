//! The rules (#544) of the devices', the circuits', the compact models', the jobs' and the phase
//! shifters' cases, by the case's id. Most are exact: a netlist against its closed form, a
//! device against the library's component or its job, a count of bits that differ. A fit or an
//! interpolation measures its own residual, and the finite differences their own truncation:
//! their basis is discretisation, as a grid's is. A lower fidelity against the next one up is a
//! model, the stated error of the device's approximation, with the higher fidelity's own grid
//! error beside it (measured on finer grids, or zero where both share the grid). The published
//! numbers are the papers' stated uncertainties, or what they don't state, with our grids gated
//! or studied beside them.

use super::{Basis::*, Measure::*, Numerical, Numerical::*, Rule};

/// The 2D thermal models' own error: each figure on the device's grids and twice as fine,
/// gated at 1 %.
const GATED: Numerical = Stated(1e-2, "its grids gated at 1 %, twice as fine");

/// The 2D thermal models' times: their grids and steps, gated at 1 % in the published cases.
const TIME_GRID: Numerical = Stated(
    1e-2,
    "the 2D grids and steps gated at 1 % (shifter/jacques-*-time, device/thermo-harris-bandwidth)",
);

/// The 3D MMI's own error: its grid study at 44, 31.4 and 22 nm, extrapolated at second order,
/// spanning first order to the 4.0 the grids show.
const MMI_3D: Numerical = Stated(
    6e-3,
    "the 3D grid study at 44, 31.4 and 22 nm, extrapolated at second order and spanning first order to the 4.0 the grids show (docs/methods/devices.md)",
);

/// The 3D MMI's best length's own error, relative: the length study's sweeps at 44 and 31.4 nm,
/// spanning the 31.4 nm grid's best to the second-order extrapolation's, with the fits' spread.
const MMI_BEST_LENGTH: Numerical = Stated(
    1.5e-2,
    "the 3D length study: sweeps at 44 and 31.4 nm, from the finer grid's best to the second-order extrapolation's (docs/methods/devices.md)",
);

/// The 3D MMI's most power's own error: the 31.4 nm sweep's, the 22 nm point's change at the
/// same length and the fits' spread.
const MMI_BEST_POWER: Numerical = Stated(
    8e-4,
    "the 3D length study: the 31.4 nm sweep, the 22 nm point and the fits' spread (docs/methods/devices.md)",
);

/// The 2D thermal models' grids at the three fabricated designs (device/thermo-2d-grid).
const THERMO_GRID: Numerical = Stated(
    8.0e-4,
    "the 2D grids against twice as fine, device/thermo-2d-grid",
);

/// The 3D bend's own error: the straight strip's on cells fitted about 10 nm (4/3 of its change
/// to 5 nm, 3.3e-5, at second order) and the conformal map's shift's (device/bend-3d-grid).
const BEND_3D: Numerical = Stated(
    7.2e-5,
    "the straight strip's 10 nm cells against 5 nm, second order, and the shift's 2.8e-5",
);

/// The rings' and couplers' circuits against their closed forms: the circuit solve's round-off.
const NETLIST: &str = "the circuit solve against its closed form: round-off";

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- the circuit solve ----
        (
            "circuit/series-waveguides",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "two guides in series are one as long: the circuit solve's round-off",
            ),
        ),
        (
            "circuit/mzi-closed-form",
            Rule::new(Absolute, RoundOff, 1e-13, Exact, NETLIST),
        ),
        (
            "circuit/ring-all-pass-bogaerts",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "Bogaerts's Eqs. 1 and 2 are the netlist's own closed form: round-off",
            ),
        ),
        (
            "circuit/ring-add-drop-bogaerts",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "Bogaerts's Eqs. 5 and 6, as the all-pass ring's",
            ),
        ),
        (
            "circuit/ring-fsr-bogaerts",
            Rule::new(
                Absolute,
                Model,
                1e-4,
                Exact,
                "Eq. 9 is first order in the FSR over λ: its next term, 7.8e-5 nm here, and the search's, nm",
            ),
        ),
        (
            "circuit/sub-network-growth",
            Rule::new(
                Absolute,
                RoundOff,
                1e-13,
                Exact,
                "the same netlist solved by another order of elimination: round-off",
            ),
        ),
        (
            "circuit/reciprocity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-13,
                Exact,
                "reciprocal components make a symmetric S: round-off",
            ),
        ),
        (
            "circuit/unitarity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-13,
                Exact,
                "lossless components make a unitary S: round-off",
            ),
        ),
        (
            "circuit/adjoint-mzi",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "the adjoint against fourth-order differences: their truncation and round-off over the step",
            ),
        ),
        (
            "circuit/adjoint-ring",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "as circuit/adjoint-mzi, smaller steps on the resonance",
            ),
        ),
        (
            "circuit/adjoint-mesh",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "as circuit/adjoint-mzi",
            ),
        ),
        (
            "circuit/adjoint-nested",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "as circuit/adjoint-ring",
            ),
        ),
        (
            "circuit/adjoint-differences",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "differences of each component inside the adjoint against differences of the whole",
            ),
        ),
        (
            "circuit/component-differences",
            Rule::new(
                Relative,
                Discretisation,
                1e-9,
                Measured,
                "a component's fourth-order differences, one-sided at its bounds, against its closed form",
            ),
        ),
        // ---- the components ----
        (
            "components/ring-all-pass-bogaerts",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "Bogaerts's Eq. 1 and the netlist of a coupler and a guide: round-off",
            ),
        ),
        (
            "components/ring-add-drop-bogaerts",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "Bogaerts's Eqs. 5 and 6 and the netlist: round-off",
            ),
        ),
        (
            "components/ring-extremes-bogaerts",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "Eqs. 11 to 16 are exact on and off resonance: round-off",
            ),
        ),
        (
            "components/ring-linewidth-bogaerts",
            Rule::new(
                Relative,
                Model,
                1e-3,
                Exact,
                "Eqs. 7 and 8, the linewidth to first order in the round trip's loss and coupling",
            ),
        ),
        (
            "components/ring-fsr-fdfd",
            Rule::new(
                Absolute,
                Discretisation,
                0.5,
                Measured,
                "the 25 nm grid's staircased ring and its numerical dispersion, nm",
            ),
        ),
        (
            "components/mzi-closed-form",
            Rule::new(Absolute, RoundOff, 1e-13, Exact, NETLIST),
        ),
        (
            "components/mzi-unitarity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-13,
                Exact,
                "lossless parts make a unitary S: round-off",
            ),
        ),
        (
            "components/unitarity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "each lossless closed form is unitary: round-off",
            ),
        ),
        (
            "components/reciprocity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-15,
                Exact,
                "each closed form is symmetric: round-off",
            ),
        ),
        (
            "components/passivity",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "no singular value above 1 but for round-off",
            ),
        ),
        (
            "components/derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "central differences at the best of four steps: their truncation and round-off",
            ),
        ),
        (
            "components/directional-coupler-power",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the closed form's own: the round-off of phases up to 1000 rad",
            ),
        ),
        (
            "components/mmi-beat-length-soldano",
            Rule::new(
                Absolute,
                Model,
                0.5,
                Exact,
                "Eq. 6 is paraxial: its next term 1 % (0.23 µm), about twice it, µm",
            ),
        ),
        (
            "components/mmi-fdfd",
            Rule::new(
                Absolute,
                Model,
                3e-2,
                Stated(
                    5.8e-3,
                    "the job's 20 nm grid against 10 and 5 nm, Richardson's estimate (order 2.1)",
                ),
                "the guided modes leave out the radiation and the faces' reflections",
            ),
        ),
        (
            "components/mzi-neff-dwivedi-470",
            Rule::new(
                Absolute,
                Reference,
                0.061,
                Stated(5.4e-5, "10 nm against 5 nm cells, second order"),
                "Table I's uncertainty and the width's ±20 nm and thickness's ±5 nm through Eq. 5",
            ),
        ),
        (
            "components/mzi-ng-dwivedi-470",
            Rule::new(
                Absolute,
                Reference,
                0.049,
                Stated(8.1e-5, "10 nm against 5 nm cells, second order"),
                "as components/mzi-neff-dwivedi-470",
            ),
        ),
        (
            "components/mzi-neff-dwivedi-602",
            Rule::new(
                Absolute,
                Reference,
                0.042,
                Stated(2.3e-5, "10 nm against 5 nm cells, second order"),
                "as components/mzi-neff-dwivedi-470",
            ),
        ),
        (
            "components/mzi-ng-dwivedi-602",
            Rule::new(
                Absolute,
                Reference,
                0.032,
                Stated(3.8e-5, "10 nm against 5 nm cells, second order"),
                "as components/mzi-neff-dwivedi-470",
            ),
        ),
        (
            "components/mzi-neff-dwivedi-805",
            Rule::new(
                Absolute,
                Reference,
                0.031,
                Stated(9.2e-6, "10 nm against 5 nm cells, second order"),
                "as components/mzi-neff-dwivedi-470",
            ),
        ),
        (
            "components/mzi-ng-dwivedi-805",
            Rule::new(
                Absolute,
                Reference,
                0.021,
                Stated(1.5e-5, "10 nm against 5 nm cells, second order"),
                "as components/mzi-neff-dwivedi-470",
            ),
        ),
        // ---- the devices ----
        (
            "device/library-components",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same components to the bit",
            ),
        ),
        (
            "device/ring-closed-forms",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the ring's figures are Bogaerts's closed forms: round-off",
            ),
        ),
        (
            "device/coupler-closed-forms",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the coupler's figures are coupled-mode theory's closed forms: round-off",
            ),
        ),
        (
            "device/derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "the exact derivatives against fourth-order differences: their truncation and round-off",
            ),
        ),
        (
            "device/waveguide-fit-neff",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the fit's interpolation between its nodes and its anchor's 5e-5, on the 3D model's own grids",
            ),
        ),
        (
            "device/waveguide-fit-ng",
            Rule::new(
                Absolute,
                Model,
                2e-2,
                Exact,
                "the analytic model's stated error: its anchor at the book's silicon, 0.0164 from Li's; the same grids",
            ),
        ),
        (
            "device/waveguide-3d-order",
            Rule::new(
                Absolute,
                Order,
                0.5,
                Measured,
                "Hadley's second order at the corners, shown to two decimals",
            ),
        ),
        (
            "device/bend-analytic",
            Rule::new(
                Absolute,
                Model,
                3e-3,
                BEND_3D,
                "the analytic model's stated error from 5 µm: the bend's shift it leaves out",
            ),
        ),
        (
            "device/bend-2d",
            Rule::new(
                Absolute,
                Model,
                6e-2,
                BEND_3D,
                "the 2D model's stated error: the effective index method's, +1.8 % for this strip",
            ),
        ),
        (
            "device/bend-3d-grid",
            Rule::new(
                Absolute,
                Discretisation,
                5e-5,
                Measured,
                "the shift's change halving the cells, the corners' error cancelling in it",
            ),
        ),
        (
            "device/coupler-fit",
            Rule::new(
                Absolute,
                Model,
                3.0,
                Exact,
                "the analytic model's stated error: its coupling fixed at 1.55 µm's, its fit and anchor; the same grids, %",
            ),
        ),
        (
            "device/coupler-3d-order",
            Rule::new(
                Absolute,
                Order,
                0.5,
                Measured,
                "the standard scheme's first order at convex corners, shown to two decimals",
            ),
        ),
        (
            "device/ring-fsr",
            Rule::new(
                Absolute,
                Model,
                0.5,
                Exact,
                "the analytic model's stated error, its group index's anchor; the same grids, %",
            ),
        ),
        (
            "device/threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same bits on any number of threads",
            ),
        ),
        (
            "device/bend-ring-slab",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same bent slab, to the bit",
            ),
        ),
        (
            "device/strip-chrostowski",
            Rule::new(
                Absolute,
                Reference,
                3e-3,
                Stated(9e-4, "against 2.5 nm, as mode/strip-book on the same grid"),
                "Lumerical MODE's 20 nm mesh, to about 1e-3, and its three decimals, as mode/strip-book",
            ),
        ),
        (
            "device/ring-q-bogaerts",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Exact,
                "'approximately 10 mm': a unit of its last digit, mm",
            ),
        ),
        (
            "device/wire-ng-dwivedi",
            Rule::new(
                Absolute,
                Reference,
                0.032,
                Stated(3.8e-5, "10 nm against 5 nm cells, second order"),
                "as components/mzi-ng-dwivedi-602",
            ),
        ),
        // ---- the rib, the MMIs, the Y-branch and the MZIs ----
        (
            "device/mmi-library",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same components to the bit",
            ),
        ),
        (
            "device/mmi-self-imaging",
            Rule::new(
                Absolute,
                Model,
                1.2,
                Exact,
                "Eq. 6 is paraxial, about 1 % short at 3 µm; the 2D model's exact slab modes, %",
            ),
        ),
        (
            "device/mzi-closed-form",
            Rule::new(Absolute, RoundOff, 1e-14, Exact, NETLIST),
        ),
        (
            "device/mzi-derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "as device/derivatives",
            ),
        ),
        (
            "device/rib-wide-slab",
            Rule::new(
                Absolute,
                Discretisation,
                2e-5,
                Measured,
                "the 50 nm grid's error (5e-6) and the extrapolation's in 1/W²",
            ),
        ),
        (
            "device/rib-eim",
            Rule::new(
                Absolute,
                Model,
                7e-2,
                Stated(
                    1.0e-4,
                    "the 3D model's 20 nm cells against 10 nm, second order",
                ),
                "the effective index method's error, the 2D model's stated one over its range",
            ),
        ),
        (
            "device/rib-hadley",
            Rule::new(
                Absolute,
                Reference,
                2e-6,
                Stated(
                    1.7e-7,
                    "the grid study at 50, 25 and 12.5 nm, stack/rib-hadley-table",
                ),
                "Hadley's Table I, his ±2–3e-6 at its lower end (ours 1.4e-6 from it, converged)",
            ),
        ),
        (
            "device/fdfd-straight",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the 3D PMLs' reflection on 100 nm cells",
            ),
        ),
        (
            "device/fdfd-3d-iterative",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "the iterative solve's residual, 1e-9, which the S-matrix's difference follows (4.7e-10 on a 2 µm MMI on 80 nm cells)",
            )
            .varying(),
        ),
        (
            "device/mmi-2d-grid",
            Rule::new(
                Absolute,
                Discretisation,
                5e-3,
                Measured,
                "the change halving the grid, order 1.8, bounds what is left",
            ),
        ),
        (
            "device/mmi-guided-2d",
            Rule::new(
                Absolute,
                Model,
                5e-3,
                Stated(
                    7.0e-4,
                    "Richardson's estimate from 20, 10 and 5 nm (order 1.8), device/mmi-2d-grid",
                ),
                "the guided modes leave out the junctions' radiation and the faces' reflections",
            ),
        ),
        (
            "device/y-branch-2d-grid",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "the change halving the grid bounds what is left",
            ),
        ),
        (
            "device/mmi-guided-3d",
            Rule::new(
                Absolute,
                Model,
                5e-2,
                MMI_3D,
                "the guided modes' stated error against 3D: the difference measured and the 3D value's uncertainty, rounded up",
            ),
        ),
        (
            "device/mmi-fdfd-3d",
            Rule::new(
                Absolute,
                Model,
                5e-2,
                MMI_3D,
                "2D FDFD's stated error against 3D: the difference measured and the 3D value's uncertainty, rounded up",
            ),
        ),
        (
            "device/mmi-best-length-3d",
            Rule::new(
                Relative,
                Model,
                7e-2,
                MMI_BEST_LENGTH,
                "the guided modes' stated error against 3D in their best length: the difference measured and the 3D length's uncertainty, rounded up",
            ),
        ),
        (
            "device/mmi-best-power-3d",
            Rule::new(
                Absolute,
                Model,
                6e-3,
                MMI_BEST_POWER,
                "the guided modes' stated error against 3D at their best lengths: the difference measured and the 3D value's uncertainty, rounded up",
            ),
        ),
        (
            "device/y-branch-2d-3d",
            Rule::new(
                Absolute,
                Model,
                2e-2,
                Stated(
                    3e-4,
                    "the 3D grid study at 44, 31.4 and 22 nm, extrapolated at second order and spanning first order to the 4.4 the grids show (docs/methods/devices.md)",
                ),
                "the 2D model's stated error against 3D: the difference measured and the 3D value's uncertainty, rounded up",
            ),
        ),
        (
            "device/y-branch-ideal-2d",
            Rule::new(
                Absolute,
                Model,
                5e-2,
                Stated(
                    9e-5,
                    "Richardson's estimate from 40, 20 and 10 nm (order 2.1), device/y-branch-2d-grid",
                ),
                "the ideal model's stated error: what the junction scatters and the arms shed",
            ),
        ),
        // ---- the devices' studies ----
        (
            "device/study-sweep-analytic",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same computation, to the bit",
            ),
        ),
        (
            "device/study-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same bits on any number of threads",
            ),
        ),
        (
            "device/study-corners-closed-form",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "a straight guide's transmission in closed form: round-off",
            ),
        ),
        (
            "device/coupler-chrostowski",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Noted,
                "the book's Eq. 4.14 at 200 nm, a fit to their 10 nm mesh's points, whose own error is about 1 µm: within 1 µm",
            ),
        ),
        (
            "device/study-coupler-chrostowski",
            Rule::new(
                Relative,
                Reference,
                5e-2,
                Stated(
                    5.8e-4,
                    "twice the change from 10 to 5 nm cells, device/study-coupler-grid",
                ),
                "their fit's own departure from their 37.5 µm at 200 nm",
            ),
        ),
        (
            "device/study-coupler-grid",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "the cells' first-order error in L_x, by nearly the same factor at every gap, cancels in the slope",
            ),
        ),
        // ---- the devices' compact models ----
        (
            "device/compact-ring-closed-form",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "the fit's own error between its samples",
            )
            .varying(),
        ),
        (
            "device/compact-ring-held-out",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "the fit's own error at its held-out samples",
            )
            .varying(),
        ),
        (
            "device/compact-ring-convergence",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the error falling at least tenfold with each step in samples and degree",
            )
            .varying(),
        ),
        (
            "device/compact-ring-derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "central differences of 1e-6 µm: their truncation and round-off",
            ),
        ),
        (
            "device/compact-ring-passive",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "the fit's own error allowed above 1",
            ),
        ),
        (
            "device/compact-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same bits on any number of threads, and from the file",
            ),
        ),
        (
            "device/compact-waveguide-3d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "the compact fidelity's stated error against its source, at held-out samples",
            )
            .varying(),
        ),
        (
            "device/compact-bend-2d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "as device/compact-waveguide-3d",
            )
            .varying(),
        ),
        (
            "device/compact-coupler-3d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "as device/compact-waveguide-3d",
            ),
        ),
        (
            "device/compact-all-pass-3d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "as device/compact-waveguide-3d",
            ),
        ),
        (
            "device/compact-add-drop-3d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "as device/compact-waveguide-3d",
            ),
        ),
        (
            "device/compact-rib-2d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "as device/compact-waveguide-3d",
            )
            .varying(),
        ),
        (
            "device/compact-mmi-1x2-2d",
            Rule::new(
                Absolute,
                Discretisation,
                5e-3,
                Measured,
                "as device/compact-waveguide-3d",
            )
            .varying(),
        ),
        (
            "device/compact-mmi-2x2-2d",
            Rule::new(
                Absolute,
                Discretisation,
                5e-3,
                Measured,
                "as device/compact-waveguide-3d",
            ),
        ),
        (
            "device/compact-y-branch-2d",
            Rule::new(
                Absolute,
                Discretisation,
                5e-2,
                Measured,
                "as device/compact-waveguide-3d",
            ),
        ),
        (
            "device/compact-mzi-3d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "as device/compact-waveguide-3d",
            )
            .varying(),
        ),
        // ---- optimizations and chips ----
        (
            "optimize/derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "fourth-order differences at 5e-6 of each value: their truncation and round-off",
            ),
        ),
        (
            "optimize/coupler-3db",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "sin²(π/4) at L_x/2 whatever the loss: round-off",
            ),
        ),
        (
            "optimize/threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same bits on any number of threads",
            ),
        ),
        (
            "optimize/hypervolume",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "sums of whole numbers, exact",
            ),
        ),
        (
            "chip/mzi-closed-form",
            Rule::new(Absolute, RoundOff, 1e-13, Exact, NETLIST),
        ),
        (
            "chip/derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "as optimize/derivatives",
            ),
        ),
        (
            "chip/adjoint",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "the same derivative by forward mode and by the adjoint: round-off",
            ),
        ),
        (
            "chip/study-points",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same netlist solve, to the bit",
            ),
        ),
        (
            "chip/threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same bits on any number of threads",
            ),
        ),
        // ---- vector fitting and compact models ----
        (
            "compact/vf-paper-rms",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the paper's 3.8e-12: round-off on a response that peaks near 200",
            ),
        ),
        (
            "compact/vf-paper-poles",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the paper's Table 3: its poles recovered to round-off",
            ),
        ),
        (
            "compact/vf-paper-residues",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "the paper's Table 3: its residues recovered to round-off",
            ),
        ),
        (
            "compact/vf-paper-real-start",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "the paper's Table 4 after three relocations: round-off",
            ),
        ),
        (
            "compact/vf-fast-equivalence",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "the same least squares, reduced: round-off",
            ),
        ),
        (
            "compact/vf-ring-allpass",
            Rule::new(
                Absolute,
                Discretisation,
                1e-8,
                Measured,
                "12 poles for a ring's many: the fit's own error",
            ),
        ),
        (
            "compact/vf-ring-poles",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "as compact/vf-ring-allpass, the poles in the band",
            ),
        ),
        (
            "compact/fdfd-ring-held-out",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "the fit's own error at the wavelengths it left out",
            ),
        ),
        (
            "compact/fdfd-ring-passive",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "a bound: no singular value above 1",
            ),
        ),
        (
            "compact/param-ring-coupling",
            Rule::new(
                Absolute,
                RoundOff,
                1e-8,
                Exact,
                "degree 1 represents the ring's dependence on r exactly: the fit's round-off",
            ),
        ),
        (
            "compact/param-ring-shift",
            Rule::new(
                Absolute,
                Discretisation,
                5e-6,
                Measured,
                "the polynomial's own error between its samples",
            ),
        ),
        (
            "compact/param-ring-stable",
            Rule::new(
                Absolute,
                Discretisation,
                5e-6,
                Measured,
                "as compact/param-ring-shift, its poles flipped",
            ),
        ),
        (
            "compact/param-ring-derivatives",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "the differences' own truncation and round-off",
            ),
        ),
        (
            "compact/param-ring-passive",
            Rule::new(
                Absolute,
                RoundOff,
                1e-8,
                Exact,
                "a bound, the exact fit's round-off allowed above 1",
            ),
        ),
        (
            "compact/param-ring-piecewise",
            Rule::new(
                Absolute,
                Discretisation,
                2e-3,
                Measured,
                "a straight line's error between samples",
            ),
        ),
        (
            "compact/param-ring-piecewise-stable",
            Rule::new(Absolute, RoundOff, 0.0, Exact, "a count of crossings: none"),
        ),
        (
            "compact/param-ring-piecewise-crossings",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "a count of crossings unconfirmed or missed: none",
            ),
        ),
        // ---- leaky modes through jobs ----
        (
            "job/leaky-film-loss",
            Rule::new(
                Relative,
                Discretisation,
                3e-3,
                Measured,
                "the 5 nm grid's second-order error, 1.7e-3, its order job/leaky-film-grid-order's",
            ),
        ),
        (
            "job/leaky-film-grid-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "second order at interfaces, shown to two decimals",
            ),
        ),
        (
            "job/leaky-film-pml",
            Rule::new(
                Relative,
                Discretisation,
                5e-4,
                Measured,
                "the PML's own discretisation, 1.8e-4 at 50 cells",
            ),
        ),
        (
            "job/leaky-wire-bienstman",
            Rule::new(
                Absolute,
                Discretisation,
                5e-3,
                Measured,
                "CAMFR's seven digits: our 5 nm grid's error, +4.0e-3, the corners'",
            ),
        ),
        (
            "job/leaky-wire-bienstman-loss",
            Rule::new(
                Absolute,
                Discretisation,
                0.15,
                Measured,
                "CAMFR's 2.9135: our 5 nm grid's error in the leakage, in 1e-8",
            ),
        ),
        // ---- the phase shifter ----
        (
            "shifter/slab-sheet",
            Rule::new(
                Relative,
                Discretisation,
                2e-3,
                Measured,
                "the mode's 5 nm grid in the perturbation's overlap, its order shifter/slab-sheet-order's",
            ),
        ),
        (
            "shifter/slab-sheet-order",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "the overlap's second order, within 0.3",
            ),
        ),
        (
            "shifter/line-source",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "the heat's grid from 5 nm, its order shifter/line-source-order's",
            ),
        ),
        (
            "shifter/line-source-order",
            Rule::new(
                Absolute,
                Order,
                0.5,
                Measured,
                "the static solver's second order and the interpolation's, within 0.5",
            ),
        ),
        (
            "shifter/jacques-tin",
            Rule::new(
                Relative,
                Reference,
                0.1,
                GATED,
                "10 % for what the paper doesn't state, as static/jacques-tin-heater",
            ),
        ),
        (
            "shifter/jacques-npp",
            Rule::new(
                Relative,
                Reference,
                0.1,
                GATED,
                "10 % for what the paper doesn't state: the cladding above",
            ),
        ),
        (
            "shifter/jacques-tin-time",
            Rule::new(
                Absolute,
                Reference,
                1.1,
                Stated(0.1, "its grids gated at 1 %, of 9.5 µs"),
                "10 % for what the paper doesn't state, and Fig. 1(c)'s switch read to ±0.2 µs",
            ),
        ),
        (
            "shifter/jacques-npp-time",
            Rule::new(
                Absolute,
                Reference,
                0.66,
                Stated(0.05, "its grids gated at 1 %, of 4.3 µs"),
                "as shifter/jacques-tin-time",
            ),
        ),
        (
            "shifter/jacques-time-step",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "BDF2's steps, the convergence gate's estimate from three",
            ),
        ),
        (
            "shifter/line-source-crosstalk",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "as shifter/line-source",
            ),
        ),
        (
            "shifter/jacques-crosstalk-5",
            Rule::new(
                Absolute,
                Reference,
                0.056,
                Stated(2e-3, "its grids gated at 1 %"),
                "40 % for the reading of the gap and the plot, the victim's far arm and the length",
            ),
        ),
        (
            "shifter/jacques-crosstalk-15",
            Rule::new(
                Absolute,
                Reference,
                0.017,
                Stated(6e-4, "its grids gated at 1 %"),
                "as shifter/jacques-crosstalk-5",
            ),
        ),
        (
            "shifter/jacques-trench",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Stated(0.035, "its grids gated at 1 %"),
                "the trench's depth and width, which the paper doesn't give, and the plots",
            ),
        ),
        (
            "shifter/harris-p-pi",
            Rule::new(
                Absolute,
                Reference,
                2.9,
                Stated(0.27, "its grids gated at 1 %, of 26.4 mW"),
                "10 % and the measurement's ±0.43 mW, for what the paper doesn't give and the device's 3D",
            ),
        ),
        (
            "shifter/harris-bandwidth",
            Rule::new(
                Absolute,
                Reference,
                32.0,
                Stated(1.1, "its grids gated at 1 %, of 109 kHz"),
                "20 % and the measurement's ±5.59 kHz, kHz",
            ),
        ),
        (
            "shifter/harris-bounds",
            Rule::new(
                Absolute,
                Model,
                0.5,
                Stated(
                    0.02,
                    "the sections' grids as the averaged one's, gated at 1 %, over their 16.8 mW",
                ),
                "a 2D section's bounds on the periodic heater: inside them, 0 to 1",
            ),
        ),
        // ---- the thermal compact models ----
        (
            "thermal-rc/known-sum",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "the fit's own convergence to a known sum",
            ),
        ),
        (
            "thermal-rc/coenen-cauer-ucut",
            Rule::new(
                Relative,
                Reference,
                2e-2,
                Exact,
                "Table I's three or four digits, the last elements' sensitivity to them",
            ),
        ),
        (
            "thermal-rc/coenen-cauer",
            Rule::new(Relative, Reference, 1e-2, Exact, "Table I's digits"),
        ),
        (
            "thermal-rc/coenen-200khz",
            Rule::new(
                Absolute,
                Reference,
                2.0,
                Exact,
                "'-30 dB', read with Fig. 13's coupling included, dB",
            ),
        ),
        (
            "thermal-rc/coenen-start-up",
            Rule::new(
                Absolute,
                Reference,
                4.0,
                Exact,
                "'40 µs', read with the neighbours' share: 10 %, µs",
            ),
        ),
        (
            "thermal-rc/jacques-poles",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "the 8-pole fit's own error over the record",
            )
            .varying(),
        ),
        (
            "thermal-rc/jacques-cutoff",
            Rule::new(
                Relative,
                Discretisation,
                2e-3,
                Measured,
                "the transient's 0.1 µs steps the model is fitted to",
            )
            .varying(),
        ),
        (
            "circuit/heated-mzi-transient",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "the 6-pole fit's own error, carried through the MZI",
            ),
        ),
        (
            "circuit/heated-mzi-modulation",
            Rule::new(
                Relative,
                Discretisation,
                1e-2,
                Measured,
                "the transient's 0.2 µs steps the model is fitted to",
            ),
        ),
        // ---- the crosstalk correction ----
        (
            "correction/gurses-cutoff",
            Rule::new(
                Relative,
                Model,
                5e-2,
                Exact,
                "their Eq. 33 leaves out the heater node's divider: 2.5 Gτ where the network gives 2.4 Gτ",
            ),
        ),
        (
            "correction/gurses-neighbour",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "the three digits of G and G_SS",
            ),
        ),
        (
            "correction/gurses-array",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "P = C⁻¹Φ sets each phase exactly: round-off, rad",
            ),
        ),
        (
            "correction/gradient",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "central differences of 1e-6 of each power: their truncation, 1e-12 of the step's square",
            ),
        ),
        // ---- the thermo-optic phase shifter devices ----
        (
            "device/thermo-analytic-metal",
            Rule::new(
                Relative,
                Model,
                0.15,
                THERMO_GRID,
                "the analytic fidelity's stated error: the heat thermally all oxide, Marcatili's mode",
            ),
        ),
        (
            "device/thermo-analytic-doped",
            Rule::new(
                Relative,
                Model,
                0.15,
                THERMO_GRID,
                "as device/thermo-analytic-metal, the slab's silicon left out",
            ),
        ),
        (
            "device/thermo-analytic-time-metal",
            Rule::new(
                Relative,
                Model,
                5e-2,
                TIME_GRID,
                "the closed form's stated error in time: the oxide's slab eigenfunctions",
            ),
        ),
        (
            "device/thermo-analytic-time-doped",
            Rule::new(
                Relative,
                Model,
                0.6,
                TIME_GRID,
                "what the closed form states for a doped heater's time: the slab's silicon left out",
            ),
        ),
        (
            "device/thermo-jacques-tin",
            Rule::new(Relative, Reference, 0.1, GATED, "as shifter/jacques-tin"),
        ),
        (
            "device/thermo-jacques-npp",
            Rule::new(Relative, Reference, 0.1, GATED, "as shifter/jacques-npp"),
        ),
        (
            "device/thermo-tin-measured",
            Rule::new(
                Relative,
                Model,
                0.15,
                GATED,
                "the 2D fidelity's stated error against the made device",
            ),
        ),
        (
            "device/thermo-npp-measured",
            Rule::new(
                Relative,
                Model,
                0.15,
                GATED,
                "as device/thermo-tin-measured",
            ),
        ),
        (
            "device/thermo-analytic-in-guide",
            Rule::new(
                Relative,
                Model,
                0.15,
                THERMO_GRID,
                "the analytic fidelity's stated error: the silicon a fin, its heat straight across",
            ),
        ),
        (
            "device/thermo-analytic-time-in-guide",
            Rule::new(
                Relative,
                Model,
                0.3,
                TIME_GRID,
                "what the closed form states for the in-guide heater's time",
            ),
        ),
        (
            "device/thermo-harris",
            Rule::new(
                Absolute,
                Reference,
                2.9,
                Stated(0.27, "its grids gated at 1 %, of 26.4 mW"),
                "as shifter/harris-p-pi",
            ),
        ),
        (
            "device/thermo-harris-bandwidth",
            Rule::new(
                Absolute,
                Reference,
                32.0,
                Stated(1.1, "its grids gated at 1 %, of 109 kHz"),
                "as shifter/harris-bandwidth",
            ),
        ),
        (
            "device/thermo-in-guide-measured",
            Rule::new(
                Relative,
                Model,
                0.15,
                GATED,
                "as device/thermo-tin-measured",
            ),
        ),
        (
            "device/thermo-2d-grid",
            Rule::new(
                Relative,
                Discretisation,
                1e-2,
                Measured,
                "the static solver's and the mode's second order: halving the cells moves P_π under 1 %",
            ),
        ),
        (
            "device/thermo-jobs",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same computation, to the bit",
            ),
        ),
    ]
}

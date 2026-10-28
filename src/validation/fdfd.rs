//! The rules (#544) of the 2D and 3D FDFD cases, by the case's id: most of them are in
//! [`super::cases`], the fill fractions' beside their module (`crate::fdfd::fill_checks`). A
//! closed form against our grid (the exact stack, the exact slab) leaves the grid's own error as
//! what the case measures, its order checked by another case; the scheme's exact identities (its
//! flux, reciprocity, a mirror, a uniform guide) and the solvers' agreement with each other are
//! round-off, the iterative solvers' within their residual times the problem's condition; the
//! published comparisons are with the papers' own discrete problems (Shin and Fan's square,
//! Freund and Malhotra's cube, Parks et al.'s square, Erlangga et al.'s symbol), which leave no
//! grid of ours to converge.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- 2D: the exact stack, the flux and the PML ----
        (
            "fdfd/slab-reflection-ez",
            Rule::new(
                Absolute,
                Discretisation,
                5e-5,
                Measured,
                "the exact stack (Chilwell and Hodgkinson): our 2.5 nm grid's error, second order (3.5e-5)",
            ),
        ),
        (
            "fdfd/slab-reflection-hz",
            Rule::new(
                Absolute,
                Discretisation,
                5e-5,
                Measured,
                "as fdfd/slab-reflection-ez (3.1e-5)",
            ),
        ),
        (
            "fdfd/flux-conservation",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the scheme's own flux is conserved exactly: round-off over the rows' sums",
            ),
        ),
        (
            "fdfd/pml-reflection",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "Shin and Fan's grading: the round trip keeps (1e-8)^1.38, an amplitude of 3e-6, and the 20 nm grid's own reflection",
            ),
        ),
        // ---- 2D ports and the cut crystals' slabs ----
        (
            "fdfd/port-mode-te",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "the exact slab: our 2.5 nm grid's error, second order (1.6e-4)",
            ),
        ),
        (
            "fdfd/port-mode-tm",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "the exact slab: our 2.5 nm grid's error, second order (1.5e-4)",
            ),
        ),
        (
            "fdfd/crystal-x-cut-slab-te",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "the exact slab: our 2.5 nm grid's error, as fdfd/port-mode-te's (3.4e-5 to 4.0e-5 here)",
            ),
        ),
        (
            "fdfd/crystal-x-cut-slab-tm",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "as fdfd/crystal-x-cut-slab-te",
            ),
        ),
        (
            "fdfd/crystal-z-cut-slab-te",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "as fdfd/crystal-x-cut-slab-te",
            ),
        ),
        (
            "fdfd/crystal-z-cut-slab-tm",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "the uniaxial slab's closed form: as fdfd/crystal-x-cut-slab-te",
            ),
        ),
        (
            "fdfd/crystal-slab-order",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "the scheme's second order, within 0.2",
            ),
        ),
        (
            "fdfd/crystal-turned",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the mirror maps the grid, the source and the tensor onto themselves: two factorizations' round-off",
            ),
        ),
        (
            "fdfd/crystal-turned-3d",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "as fdfd/crystal-turned, each component's entry of the tensor mapped onto itself",
            ),
        ),
        (
            "fdfd/straight-guide",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the grid's own port modes and a total-field source: a uniform guide transmits them whole, to round-off",
            ),
        ),
        (
            "fdfd/reciprocity",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "S symmetric with the stretches as weights and the unconjugated Lorentz form: round-off",
            ),
        ),
        (
            "fdfd/step-reflection-te",
            Rule::new(
                Absolute,
                Model,
                2e-5,
                Noted,
                "Fresnel's formula on the effective indices, which leaves out the modes' mismatch and the 1.9e-4 radiated: within 2e-5, 1.7 % of it",
            ),
        ),
        (
            "fdfd/adjoint-gradient-ez",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "fourth-order differences at δ = 1e-3: the error they resolve, the powers' precision through their weights (6.6e-7 and 8.4e-7 here)",
            ),
        ),
        (
            "fdfd/adjoint-gradient-hz",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "as fdfd/adjoint-gradient-ez",
            ),
        ),
        // ---- 3D: the exact stack, the flux, the PML and the 2D scheme ----
        (
            "fdfd3d/film-reflection-te",
            Rule::new(
                Absolute,
                Discretisation,
                6e-5,
                Measured,
                "the exact stack (Chilwell and Hodgkinson): our 2.5 nm grid's error, second order (4.6e-5)",
            ),
        ),
        (
            "fdfd3d/film-reflection-tm",
            Rule::new(
                Absolute,
                Discretisation,
                5e-5,
                Measured,
                "as fdfd3d/film-reflection-te (3.1e-5)",
            ),
        ),
        (
            "fdfd3d/flux-conservation",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the scheme's own flux is conserved exactly: round-off over the planes' sums",
            ),
        ),
        (
            "fdfd3d/pml-reflection",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "as fdfd/pml-reflection",
            ),
        ),
        (
            "fdfd3d/two-d-agreement",
            Rule::new(
                Relative,
                RoundOff,
                1e-11,
                Exact,
                "the same equations, one eliminating H and the other E, each factorized: round-off (2e-13 measured)",
            ),
        ),
        // ---- 3D ports ----
        (
            "fdfd3d/port-mode-slab-te",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the exact slab: our 2.5 nm grid's error, second order (6.0e-5)",
            ),
        ),
        (
            "fdfd3d/port-mode-slab-tm",
            Rule::new(
                Absolute,
                Discretisation,
                2e-4,
                Measured,
                "the exact slab: our 2.5 nm grid's error, second order (1.5e-4)",
            ),
        ),
        (
            "fdfd3d/port-mode-strip",
            Rule::new(
                Absolute,
                Discretisation,
                3e-3,
                Measured,
                "Hadley's equations extrapolated, to 7e-6: our port's 20 nm grid's error, second order (2.4e-3)",
            ),
        ),
        (
            "fdfd3d/straight-strip",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "as fdfd/straight-guide (1.8e-15 measured)",
            ),
        ),
        (
            "fdfd3d/adjoint-gradient",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "fourth-order differences at δ = 1e-3: their round-off, about 1e-10/δ",
            ),
        ),
        (
            "fdfd3d/reciprocity",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "as fdfd/reciprocity (2e-15 measured)",
            ),
        ),
        (
            "fdfd3d/closed-guide-energy",
            Rule::new(
                Absolute,
                Discretisation,
                5e-6,
                Measured,
                "S†S = 1 but for the evanescent modes' power across the ports, falling as e^(−2κd): 2e-6 at 1 µm",
            ),
        ),
        (
            "fdfd3d/two-d-s-matrix",
            Rule::new(
                Absolute,
                Discretisation,
                5e-8,
                Measured,
                "the 2D scheme but for the PMLs half a cell apart (8.9e-9 with E along z) and the eigensolver's tolerance (2.1e-10 with H)",
            ),
        ),
        // ---- the iterative and direct solvers: the same system two ways ----
        (
            "fdfd3d/qmr-direct",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "QMR to a residual of 1e-10, the direct solver's far below: within the condition times it (1.3e-10 measured)",
            ),
        ),
        (
            "fdfd3d/qmr-symmetric-direct",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "as fdfd3d/qmr-direct (6.9e-12 measured)",
            ),
        ),
        (
            "fdfd3d/ldlt-lu",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "both refined to a residual of 1e-12: within the condition times it (1.2e-15 measured)",
            ),
        ),
        (
            "fdfd3d/qmr-plateau",
            Rule::new(
                Absolute,
                Model,
                5e-3,
                Exact,
                "Shin and Fan's floor, the residual's share in the near-null eigenspace (1/√2 to three digits), which the rest of the residual keeps it above",
            ),
        ),
        (
            "fdfd3d/qmr-iterations-curl-curl",
            Rule::new(
                Absolute,
                Reference,
                5.0,
                Exact,
                "Fig. 3 read off the plot to ±5 iterations: the curves' width, at 3.4 pixels an iteration",
            ),
        ),
        (
            "fdfd3d/qmr-iterations-shin-fan",
            Rule::new(
                Absolute,
                Reference,
                5.0,
                Exact,
                "as fdfd3d/qmr-iterations-curl-curl",
            ),
        ),
        (
            "fdfd3d/pml-reflection-stretched",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "a stretch with Im s > 0 absorbs without reflecting (Chew and Weedon): the 20 nm grid's reflection, within the plain PML's",
            ),
        ),
        (
            "fdfd3d/qmr-ilu-direct",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "QMR to a residual of 1e-10: within the condition times it (2.4e-10 measured)",
            ),
        ),
        (
            "fdfd3d/gmres-multigrid-direct",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "GMRES to a residual of 1e-10: within the condition times it (2.0e-10 measured)",
            ),
        ),
        // ---- block QMR ----
        (
            "fdfd3d/block-qmr-s-matrix",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "both to a residual of 1e-10: their S-matrices within the condition times it (1.7e-11 measured)",
            ),
        ),
        (
            "fdfd3d/block-qmr-ilu-s-matrix",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "as fdfd3d/block-qmr-s-matrix (6.6e-11 and 1.1e-10 measured)",
            ),
        ),
        (
            "fdfd3d/block-qmr-iterations",
            Rule::new(
                Absolute,
                Order,
                0.6,
                Measured,
                "a block Krylov space serves all 8 systems, at best 1/8 of the products: under 0.6",
            ),
        ),
        (
            "fdfd3d/block-qmr-dependent",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "deflated systems recovered as exact combinations: round-off (6.2e-15 measured)",
            ),
        ),
        (
            "fdfd3d/block-qmr-convergence",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "the error falls as the tolerance, κ(A) times it: a slope of 1, within 0.2",
            ),
        ),
        (
            "fdfd3d/block-qmr-freund-malhotra",
            Rule::new(
                Absolute,
                Reference,
                0.5,
                Exact,
                "their counts' last digits (±0.15 in the ratio) and their stopping test, which we can't read and which moves the ratio to 4.7",
            ),
        ),
        (
            "fdfd3d/block-qmr-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same computation on any number of threads: no bit differs",
            ),
        ),
        // ---- recycling ----
        (
            "fdfd3d/recycle-s-matrix",
            Rule::new(
                Absolute,
                RoundOff,
                1e-10,
                Exact,
                "ROADMAP's 0.5.2 line, the single solves' S-matrices to 1e-10: the fields to 1e-11 and 1e-10, within the condition times it (2.1e-11 measured)",
            ),
        ),
        (
            "fdfd3d/recycle-deflation",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "Parks et al.'s Theorem 3.1 with δ = 0: the deflated problem's residuals step by step, to the rounding of a run to 1e-10 (2.2e-11 measured)",
            ),
        ),
        (
            "fdfd3d/recycle-convergence",
            Rule::new(
                Absolute,
                Order,
                0.35,
                Measured,
                "each pair of solutions kept raises the start's order: about a third of the plain sweep's iterations (0.23 measured)",
            ),
        ),
        (
            "fdfd3d/recycle-adjoint",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "both solved to 1e-11: within the condition times it (7.8e-11 measured)",
            ),
        ),
        (
            "fdfd3d/recycle-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "as fdfd3d/block-qmr-threads",
            ),
        ),
        (
            "fdfd3d/qmr-matrix-free-sweep",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "the product the stored matrix's to rounding, every field to a residual of 1e-10: within the condition times it (2.4e-11 measured)",
            ),
        ),
        (
            "fdfd3d/qmr-matrix-free-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "as fdfd3d/block-qmr-threads",
            ),
        ),
        // ---- the multigrid without its finest matrix ----
        (
            "fdfd3d/multigrid-stencil-direct",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "as fdfd3d/gmres-multigrid-direct (3.2e-10 measured)",
            ),
        ),
        (
            "fdfd3d/multigrid-stencil-s-matrix",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "a preconditioner changes the iterations, not the solution: every field to 1e-11, within the condition times it (8.4e-12 measured)",
            ),
        ),
        (
            "fdfd3d/multigrid-stencil-threads",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "as fdfd3d/block-qmr-threads",
            ),
        ),
        (
            "fdfd3d/multigrid-stencil-refinement",
            Rule::new(
                Absolute,
                Order,
                1.5,
                Measured,
                "a multigrid's iterations stay level as the cells halve, a ratio of 1, where a step-dependent method's double: within half of that",
            ),
        ),
        (
            "fdfd3d/multigrid-stencil-plane-waves",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "each plane wave an eigenvector, its damping the Fourier symbol's: round-off",
            ),
        ),
        (
            "fdfd3d/multigrid-stencil-smoothing-factors",
            Rule::new(
                Absolute,
                Reference,
                1e-2,
                Exact,
                "Table 5's two decimals: a unit of the second, as two of theirs (0.75, 0.35) are a unit off their own symbol's",
            ),
        ),
        (
            "fdfd3d/recycle-parks-table",
            Rule::new(
                Absolute,
                Reference,
                1e-6,
                Exact,
                "Table 4.3's five cosines at 1 and five at 0: their departures (to 5.9e-9) and ours (to 7.2e-8) each the run's rounding",
            ),
        ),
        (
            "fdfd3d/recycle-parks-rerun",
            Rule::new(
                Absolute,
                Order,
                0.9,
                Measured,
                "Fig. 4.9: the second run faster than full GMRES, under 0.9 of its products (0.58 measured)",
            ),
        ),
        // ---- the fill fractions ----
        (
            "fdfd/fill-slab-te-mean",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "Yee's second order with the right average, within 0.1",
            ),
        ),
        (
            "fdfd/fill-slab-te-point",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "staircasing's first order, within 0.1",
            ),
        ),
        (
            "fdfd/fill-slab-tm-diagonal",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "second order with Farjadpour et al.'s average, within 0.2",
            ),
        ),
        (
            "fdfd/fill-slab-tm-mean",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the wrong average's first order, within 0.1",
            ),
        ),
        (
            "fdfd3d/fill-film-te-diagonal",
            Rule::new(Absolute, Order, 0.1, Measured, "as fdfd/fill-slab-te-mean"),
        ),
        (
            "fdfd3d/fill-film-tm-diagonal",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "as fdfd/fill-slab-tm-diagonal",
            ),
        ),
        (
            "fdfd3d/fill-film-tm-point",
            Rule::new(Absolute, Order, 0.1, Measured, "as fdfd/fill-slab-te-point"),
        ),
        (
            "fdfd/fill-crystal-te-tensor",
            Rule::new(
                Absolute,
                Order,
                0.4,
                Measured,
                "Farjadpour et al.'s Fig. 1: second order, with the scatter of an order fitted at 10 to 56 cells, within 0.4",
            ),
        ),
        (
            "fdfd/fill-crystal-te-others",
            Rule::new(
                Absolute,
                Order,
                0.4,
                Measured,
                "Farjadpour et al.'s Fig. 1: first order or erratic, the largest within 0.4 of the first",
            ),
        ),
        (
            "fdfd/fill-crystal-tm-mean",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "Farjadpour et al.: second order for TM, within 0.2",
            ),
        ),
        (
            "fdfd/fill-derivative-ez",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "central differences over ±0.1 nm, of the power and of the matrices: their second-order error and round-off",
            ),
        ),
        (
            "fdfd/fill-derivative-hz",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "as fdfd/fill-derivative-ez",
            ),
        ),
        (
            "fdfd/fill-seam",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a seam is no interface: the pieces' fill is the whole strip's, so its S is the same and symmetric, to round-off",
            ),
        ),
        (
            "fdfd3d/fill-seam",
            Rule::new(Absolute, RoundOff, 1e-12, Exact, "as fdfd/fill-seam"),
        ),
    ]
}

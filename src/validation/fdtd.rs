//! The rules (#544) of FDTD's cases, by the case's id: most of them are in [`super::cases`]
//! itself, the graded medium's beside it (`material::graded_checks`).
//!
//! FDTD against FDFD on the same grid at the leapfrog's frequency is an exact identity, so its
//! cases are round-off, the direct solve's condition included; where the CPMLs come in, what is
//! left is the CPML against FDFD's PML in discrete time, first order in Δt, and the case measures
//! that. Against a closed form (Airy's slab, Fresnel, the transfer matrices, the ring's exact
//! poles, Mie's series) what the case measures is our grid's error, its order checked by
//! another case. A fit of a material by Lorentz terms is treated as a grid: its residual is what
//! the case measures. An adjoint gradient against finite differences is judged by the
//! differences' own error: the transforms' settling over the step, with the step's truncation
//! below it. Below that the digits are each platform's arithmetic, so an error under it is
//! shown as it, and the row reads the same everywhere.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- the scheme ----
        (
            "fdtd/dispersion",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the leapfrog keeps Taflove and Brodwin's relation exactly: round-off over 400 steps",
            ),
        ),
        (
            "fdtd/energy",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "Yee's two curls each other's transposes: the invariant to round-off over 300 steps",
            ),
        ),
        (
            "fdtd/fdfd-lossy",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the steady state solves Yee's equations at the leapfrog's frequency exactly: the direct solve's round-off",
            ),
        ),
        (
            "fdtd/fdfd-cpml",
            Rule::new(
                Relative,
                Discretisation,
                5e-4,
                Measured,
                "the CPML's convolution against FDFD's stretch, first order in Δt: 6.6e-4 at 64 steps a period, 3.5e-4 at 128",
            ),
        ),
        (
            "fdtd/cpml-thickness",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "what a CPML reflects, falling as it thickens: 16 cells below 8 cells' 1.2e-4",
            ),
        ),
        (
            "fdtd/cpml-late-growth",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "a passive system's energy only falls, so the ratio is at most 1; the grid's own waves grow in the CPMLs on 50 nm cells (4.4e14), not on 25",
            ),
        ),
        // ---- sources ----
        (
            "fdtd/spectrum",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "once the fields have died the transforms solve Yee's equations at the leapfrog's frequency exactly: the solves' round-off",
            ),
        ),
        (
            "fdtd/tfsf-leakage",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the incident field solves the grid's own equations: round-off",
            ),
        ),
        (
            "fdtd/tfsf-slab",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "Airy's slab against the 5 nm grid, second order: 5.4e-3, 1.3e-3 and 3.3e-4 at 20, 10 and 5 nm",
            ),
        ),
        (
            "fdtd/mode-source-fdfd",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "FDFD's mode source in the time domain, the same equations: the solves' round-off",
            ),
        ),
        (
            "fdtd/mode-source-backward",
            Rule::new(
                Relative,
                Discretisation,
                1e-5,
                Measured,
                "one way exactly in FDFD's PMLs: what is left is the CPMLs', first order in Δt",
            ),
        ),
        (
            "fdtd/mode-source-power",
            Rule::new(
                Relative,
                Discretisation,
                5e-4,
                Measured,
                "the mode's power on FDFD's grid, its flux through the CPMLs' guide in discrete time",
            ),
        ),
        (
            "fdtd/beam-waist",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "the beam's plane waves on 50 nm cells, its width's moments fitted from 0 to 12 µm, in µm",
            ),
        ),
        (
            "fdtd/beam-divergence",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "exact for a sum of plane waves: the width's moments sampled on 50 nm cells and fitted",
            ),
        ),
        (
            "fdtd/beam-tilt",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "exact for a sum of plane waves: the centre sampled on 50 nm cells from 2 to 14 µm",
            ),
        ),
        // ---- dispersive media and Bloch sides ----
        (
            "fdtd/ade-fdfd",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "Okoniewski et al.'s scheme solves FDFD's equations with the leapfrog's χ exactly: the solve's round-off",
            ),
        ),
        (
            "fdtd/drude-fresnel",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "Fresnel's against the 5 nm grid, second order: 6.9e-3, 1.6e-3 and 3.0e-4 at 20, 10 and 5 nm",
            ),
        ),
        (
            "fdtd/lorentz-group-delay",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "the first pass's exact delay against the 2.5 nm grid, second order to a floor of about 1e-4: 3.0e-3, 8.8e-4 and 3.6e-4 at 10, 5 and 2.5 nm",
            ),
        ),
        (
            "fdtd/bloch-fdfd",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the real and imaginary parts coupled across the Bloch sides as FDFD's differences are: the solves' round-off",
            ),
        ),
        (
            "fdtd/bloch-multilayer",
            Rule::new(
                Absolute,
                Discretisation,
                3e-3,
                Measured,
                "the transfer matrices against the 5 nm grid, second order: 2.0e-2, 5.0e-3 and 1.3e-3 at 20, 10 and 5 nm",
            ),
        ),
        (
            "fdtd/bloch-bands",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "the analytic bands against 80 cells a period, second order: 5.0e-3, 1.2e-3 and 3.1e-4 at 20, 40 and 80",
            ),
        ),
        (
            "fdtd/fit-lossless",
            Rule::new(
                Relative,
                Discretisation,
                1e-4,
                Measured,
                "the fit's residual with two undamped terms, ε's own accuracy for FDTD",
            ),
        ),
        (
            "fdtd/fit-lossy",
            Rule::new(
                Relative,
                Discretisation,
                1e-2,
                Measured,
                "the fit's residual with 8 damped terms on a fixed ladder of resonances",
            ),
        ),
        (
            "fdtd/fitted-slab",
            Rule::new(
                Absolute,
                Discretisation,
                5e-3,
                Measured,
                "Airy's slab against the 5 nm grid, second order toward the fit's 6e-5: 2.9e-2, 7.3e-3 and 2.0e-3 at 20, 10 and 5 nm",
            ),
        ),
        // ---- subpixel smoothing ----
        (
            "fdtd/smoothing-slab",
            Rule::new(
                Absolute,
                Discretisation,
                1.5e-3,
                Measured,
                "Airy's slab against cells of 1/160 µm, second order with Farjadpour et al.'s average: 3.0e-3 at 1/80, 6.9e-4 at 1/160",
            ),
        ),
        (
            "fdtd/smoothing-anisotropic-slab",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "the slab's exact reflection along its principal axes against cells of 1/160 µm, second order: 1.2e-3 at 1/80, 3.0e-4 at 1/160",
            ),
        ),
        (
            "fdtd/smoothing-oblique",
            Rule::new(
                Relative,
                Discretisation,
                4e-4,
                Measured,
                "the exact layers against 128 cells a µm, second order with Farjadpour et al.'s placement: error times n² −3.5",
            ),
        ),
        (
            "fdtd/smoothing-oblique-nodes",
            Rule::new(
                Relative,
                Discretisation,
                3e-3,
                Measured,
                "the exact layers against 128 cells a µm, first order with Werner and Cary's placement: error times n −0.26",
            ),
        ),
        (
            "fdtd/smoothing-energy",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "ε̃⁻¹ symmetric with the entries at the nodes: the energy to round-off over 10⁵ steps",
            ),
        ),
        (
            "fdtd/smoothing-contrast",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "ε̃⁻¹ symmetric and positive definite at ε = 40: the energy to round-off over 10⁵ steps, and so the fields bounded",
            ),
        ),
        (
            "fdtd/smoothing-oskooi",
            Rule::new(
                Relative,
                Reference,
                0.1,
                Noted,
                "Fig. 2's lowest error 'often by an order of magnitude': a tenth of the smaller of the others'",
            ),
        ),
        (
            "fdtd/smoothing-triplets-oblique",
            Rule::new(
                Relative,
                Discretisation,
                1.6e-3,
                Measured,
                "the exact layers against 128 cells a µm, first order as every symmetric effective dielectric: error times n −0.14",
            ),
        ),
        (
            "fdtd/smoothing-triplets-order",
            Rule::new(
                Absolute,
                Order,
                0.35,
                Measured,
                "first order, as Werner, Bauer and Cary find, from second at coarse grids: within 0.35",
            ),
        ),
        (
            "fdtd/smoothing-diagonal-oblique",
            Rule::new(
                Relative,
                Discretisation,
                6e-3,
                Measured,
                "the exact layers against 128 cells a µm, the dropped coupling first order: error times n 0.57",
            ),
        ),
        (
            "fdtd/smoothing-diagonal-order",
            Rule::new(
                Absolute,
                Order,
                0.35,
                Measured,
                "first order, not yet asymptotic at 64 to 128 cells (error times n still rising): within 0.35",
            ),
        ),
        (
            "fdtd/smoothing-bauer-order",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "Bauer et al.'s unsymmetric tensors exact for constant fields at a plane: second order, within 0.3",
            ),
        ),
        (
            "fdtd/smoothing-triplets-contrast",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "ε̃⁻¹ symmetric and positive definite at any contrast: the energy to round-off over 10⁵ steps",
            ),
        ),
        (
            "fdtd/smoothing-triplets-lattices",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "ε̃⁻¹ symmetric: the energy to round-off over 10⁵ steps",
            ),
        ),
        (
            "fdtd/smoothing-wc07-growth",
            Rule::new(
                Absolute,
                Reference,
                0.5,
                Noted,
                "their 'γ ≈ 3c/a' and '≈ 6c/a', one digit each: half a unit, in c/a",
            ),
        ),
        (
            "fdtd/smoothing-crystal",
            Rule::new(
                Relative,
                Discretisation,
                3e-3,
                Measured,
                "Table 1 to eight digits by Richardson's extrapolation at 96 and 128 cells: our 48 cells' error, 1.1e-3 to 1.8e-3 by band",
            ),
        ),
        (
            "fdtd/smoothing-crystal-order",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "Bauer et al.'s second order (their Fig. 8), within 0.3",
            ),
        ),
        (
            "fdtd/crystal-turned",
            Rule::new(
                Relative,
                RoundOff,
                1e-13,
                Exact,
                "the mirror maps the grid, the source and the tensor onto themselves: rounding only",
            ),
        ),
        // ---- monitors and harmonic inversion ----
        (
            "fdtd/monitor-transforms",
            Rule::new(
                Relative,
                RoundOff,
                1e-15,
                Exact,
                "the same sum in the same order: the same bits",
            ),
        ),
        (
            "fdtd/monitor-flux",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "the transforms are FDFD's fields at the leapfrog's frequency: the direct solve's round-off and the fields' decay to 1e-17",
            ),
        ),
        (
            "fdtd/monitor-flux-box",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "summation by parts leaves only the faces: round-off once the fields have died",
            ),
        ),
        (
            "fdtd/monitor-modes",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "FDFD's own Lorentz form on FDFD's fields: the direct solve's round-off and the fields' decay to 1e-15",
            ),
        ),
        (
            "fdtd/monitor-guides",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the CPML against FDFD's PML in discrete time on 50 nm cells: the straight guide's reflections, 1.3e-4 by both",
            ),
        ),
        (
            "fdtd/harmonic-inversion",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "filter diagonalization is exact for finitely many terms: the small eigenproblem's round-off",
            ),
        ),
        (
            "fdtd/cavity-resonances",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the leapfrog's own modes, their factors u of magnitude near 1: round-off",
            ),
        ),
        (
            "fdtd/slab-resonance",
            Rule::new(
                Absolute,
                Order,
                0.05,
                Measured,
                "second order, the scheme's dispersion alone with the faces between nodes: within 0.05",
            ),
        ),
        // ---- Mie's series and the sphere ----
        (
            "fdtd/mie-table",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "his three decimals, and gold's m′² to two: half a unit of the second carried through 𝔞₁, up to 5e-3 near the resonance",
            ),
        ),
        (
            "fdtd/mie-balance",
            Rule::new(
                Relative,
                RoundOff,
                1e-11,
                Exact,
                "each coefficient's real part its squared magnitude for a real index: round-off over sums of up to 1100 terms",
            ),
        ),
        (
            "fdtd/mie-terms",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the terms past α + 4α^(1/3) + 10 fall below rounding: round-off in Q_ext",
            ),
        ),
        (
            "fdtd/mie-sphere",
            Rule::new(
                Absolute,
                Order,
                0.15,
                Measured,
                "second order, the scheme's dispersion with the sphere smoothed: within 0.15",
            ),
        ),
        (
            "fdtd/mie-sphere-staircase",
            Rule::new(
                Relative,
                Discretisation,
                1e-2,
                Measured,
                "Mie's series against the staircase on 16 cells a radius, irregular in the grid: 8.1e-3 at 12, 6.0e-3 at 16",
            ),
        ),
        (
            "fdtd/mie-drude",
            Rule::new(
                Relative,
                Discretisation,
                3e-2,
                Measured,
                "Mie's series against a staircased surface, first order: 3.0e-2 on 8 cells a radius, 1.6e-2 on 16",
            ),
        ),
        // ---- PMLs, rings and FDFD over a band ----
        (
            "fdtd/meep-pml-rates",
            Rule::new(
                Absolute,
                Order,
                0.4,
                Measured,
                "Oskooi et al.'s 1/L^(2d+4), approached from above: 0.32 off at L = 4 (0.08 at L = 8), within 0.4",
            ),
        ),
        (
            "fdtd/ring-wronskian",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "the power series lose up to e^r/2πr of their precision to cancellation, r = 15 at most",
            ),
        ),
        (
            "fdtd/ring-resonances",
            Rule::new(
                Relative,
                Discretisation,
                1.5e-3,
                Measured,
                "the ring's exact poles against 20 cells a µm, second order (fdtd/ring-order)",
            ),
        ),
        (
            "fdtd/ring-order",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "second order with E_z smoothed along the faces, within 0.3",
            ),
        ),
        (
            "fdtd/fdfd-band-2d",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the CPML against FDFD's PML in discrete time on 50 nm cells, falling to 1.7e-7 on 25",
            ),
        ),
        (
            "fdtd/fdfd-band-2d-fine",
            Rule::new(
                Absolute,
                Discretisation,
                1e-6,
                Measured,
                "the CPML against FDFD's PML on 25 nm cells: what each reflects, falling as the same thickness takes more cells",
            ),
        ),
        (
            "fdtd/fdfd-band-3d-strip",
            Rule::new(
                Absolute,
                Discretisation,
                5e-4,
                Measured,
                "the CPML against FDFD's PML, converging as it thickens: 2.2e-4, 6.5e-5 and 7.9e-6 with 8, 10 and 12 cells",
            ),
        ),
        (
            "fdtd/fdfd-band-3d-bend",
            Rule::new(
                Absolute,
                Discretisation,
                5e-4,
                Measured,
                "the CPML of 8 cells against FDFD's PML, as fdtd/fdfd-band-3d-strip",
            ),
        ),
        (
            "fdtd/fdfd-smoothed",
            Rule::new(
                Absolute,
                Order,
                0.3,
                Measured,
                "first order at curved faces for both averages, not yet asymptotic (0.97 from 25 to 12.5 nm): within 0.3",
            ),
        ),
        // ---- adjoint gradients ----
        (
            "fdtd/adjoint-modes",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "fourth-order differences at δ = 1e-3, which resolve 1.5e-8 (the transforms settled to 1e-11, over δ; the truncation δ⁴ below it), an error under that shown as it",
            ),
        ),
        (
            "fdtd/adjoint-flux",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "as fdtd/adjoint-modes",
            ),
        ),
        (
            "fdtd/adjoint-fdfd",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the transforms are FDFD's fields at the leapfrog's frequency, and its adjoint the same solve's: round-off",
            ),
        ),
        (
            "fdtd/adjoint-fdfd-cpml",
            Rule::new(
                Relative,
                Discretisation,
                5e-5,
                Measured,
                "the CPML against FDFD's PML, first order in Δt: 1.3e-4, 5.1e-5 and 2.3e-5 at 64, 128 and 256 steps a period",
            ),
        ),
        (
            "fdtd/adjoint-slab",
            Rule::new(
                Relative,
                Discretisation,
                1e-3,
                Measured,
                "Airy's dT/dε against cells of 1/160 µm, second order: 1.2e-2, 2.9e-3 and 7.2e-4 at 1/40, 1/80 and 1/160",
            ),
        ),
        (
            "fdtd/adjoint-slab-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the scheme's second order, within 0.1",
            ),
        ),
        (
            "fdtd/adjoint-backward-mode",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "Lalau-Keraly et al.'s identity holds on the grid with FDFD's Lorentz form: round-off",
            ),
        ),
        // ---- a graded medium ----
        (
            "fdtd/graded-parabolic",
            Rule::new(
                Relative,
                Discretisation,
                2e-4,
                Measured,
                "the harmonic oscillator against the 12.5 nm grid, second order (fdtd/graded-parabolic-order)",
            ),
        ),
        (
            "fdtd/graded-parabolic-order",
            Rule::new(
                Absolute,
                Order,
                0.25,
                Measured,
                "Yee's second order for a smooth medium, within 0.25",
            ),
        ),
    ]
}

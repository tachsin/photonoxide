//! The rules (#544) of the transmission lines' cases: the static solver's lines against closed
//! forms and conformal mapping, the full-wave RF mode against the exact plates and the
//! quasi-TEM line, and the published lines (Kharel et al., Zhang et al., Shin et al., Heinrich,
//! Frankel et al.). Our own error beside each published number comes from its grid study: the
//! static grids' gate at 0.5% (8, 4 and 2 nm), the full-wave study's estimate (1.3e-3), and the
//! cases' own grid cases (Kharel's segmented 6.7e-4, Heinrich's 1.4e-3, Frankel's 1.6e-4). Ghione
//! and Naldi's line, on one 4 nm grid, was run again at 1 nm (2026-10-29).

use super::{Basis::*, Measure::*, Numerical, Numerical::*, Rule};

/// The static grids' study, which every published static line passes (`gated`, 8, 4 and 2 nm).
const STATIC_GATE: Numerical = Stated(5e-3, "the static grids' study gates the line at 0.5%");
/// The full-wave study of Zhang et al.'s line (line/full-wave-grid's estimate).
const FULL_WAVE: Numerical = Stated(1.3e-3, "line/full-wave-grid, Richardson's estimate");
const SEGMENTED: Numerical = Stated(6.7e-4, "line/kharel-2021-segmented-grid");
const HEINRICH: Numerical = Stated(1.4e-3, "line/heinrich-1993-grid");
const FRANKEL: Numerical = Stated(1.6e-4, "line/frankel-1991-grid");

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- the static solver's lines ----
        (
            "line/holloway-kuester-current",
            Rule::new(
                Relative,
                Discretisation,
                2e-3,
                Measured,
                "the edge's r^-1/2 current against Holloway and Kuester's closed form, on a 1 nm edge step: first order",
            ),
        ),
        (
            "line/holloway-kuester-order",
            Rule::new(
                Absolute,
                Order,
                0.35,
                Measured,
                "first order at edges singular as r^-1/2 (static/coplanar-edges-order); the singularity comes in slowly: within 0.35",
            ),
        ),
        (
            "line/ghione-naldi-thick",
            Rule::new(
                Relative,
                Model,
                1e-2,
                Stated(1.0e-5, "the 4 nm grid against 1 nm"),
                "conformal mapping against their Fig. 2's variational bounds, close together at h/b = 1: 1%",
            ),
        ),
        (
            "line/ghione-naldi-thin",
            Rule::new(
                Relative,
                Model,
                2.5e-2,
                Stated(1.3e-5, "the 4 nm grid against 1 nm"),
                "Eq. 2 near the upper of the variational bounds at h/b = 0.5, 8% apart: 2.5%",
            ),
        ),
        (
            "line/ghione-naldi-impedance",
            Rule::new(
                Relative,
                Model,
                5e-3,
                Stated(1.0e-4, "the 4 nm grid against 1 nm"),
                "C0 exact, so half Eq. 2's 1%",
            ),
        ),
        (
            "line/wheeler-plates",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "Wheeler's surface integral on uniform plates, 2/w: round-off",
            ),
        ),
        (
            "line/wheeler-holloway-kuester",
            Rule::new(
                Relative,
                Model,
                0.1,
                Stated(
                    1.0e-4,
                    "the static solver's 4 nm grid against 1 nm, as Ghione and Naldi's line",
                ),
                "Wheeler's rule fails within a skin depth of an edge, where Holloway and Kuester's stopping distance doesn't: the two within 10%",
            ),
        ),
        (
            "line/filling-plates",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "capacitors in series, the interfaces on the grid's lines: round-off",
            ),
        ),
        // ---- the electro-optic response ----
        (
            "line/response-walk-off",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "Ghione's U = 1.39: half a unit of its last digit",
            ),
        ),
        (
            "line/response-loss-3db",
            Rule::new(
                Absolute,
                Reference,
                1e-2,
                Exact,
                "Ghione's 6.393 dB, from αL = 0.736 Np to its last digit (±4.3e-3 dB); 1e-2",
            ),
        ),
        (
            "line/response-loss-6db",
            Rule::new(
                Absolute,
                Reference,
                5e-2,
                Exact,
                "Zhang et al.'s 13.8 dB: half a unit of its last digit",
            ),
        ),
        // ---- published lines, quasi-TEM ----
        (
            "line/kharel-2021-impedance",
            Rule::new(
                Relative,
                Reference,
                0.1,
                STATIC_GATE,
                "what the paper leaves out (±3%, ±1.5%) and its 7 µm row's 4–5% off the trend: 10%",
            ),
        ),
        (
            "line/kharel-2021-index",
            Rule::new(
                Relative,
                Reference,
                0.04,
                STATIC_GATE,
                "the unstated etch and cladding (±2.5%) and the 5 µm row's 3%: 4%",
            ),
        ),
        (
            "line/kharel-2021-loss",
            Rule::new(
                Relative,
                Reference,
                0.12,
                STATIC_GATE,
                "the gold's ±5%, the geometry's ±3%, Holloway and Kuester's 5–10%: 12%",
            ),
        ),
        (
            "line/zhang-2022-index",
            Rule::new(
                Absolute,
                Reference,
                0.04,
                Stated(1.1e-2, "the static grids' 0.5% gate on 2.26"),
                "their measured index 0.02 and 0.04 above the design: 0.04",
            ),
        ),
        (
            "line/zhang-2022-impedance",
            Rule::new(
                Absolute,
                Reference,
                1.5,
                Stated(0.21, "the static grids' 0.5% gate on 41 Ω"),
                "41.5 Ω extracted from the measurement against 40.5 Ω simulated: 1.5 Ω",
            ),
        ),
        (
            "line/zhang-2022-loss",
            Rule::new(
                Absolute,
                Reference,
                3.0,
                Stated(0.08, "the static grids' 0.5% gate on 15 dB/cm"),
                "1.3 dB/mm, their figure's 16 and their fit's 12.9 dB/cm: 3 dB/cm",
            ),
        ),
        (
            "line/zhang-2022-rf-vpi-short",
            Rule::new(
                Relative,
                Reference,
                0.2,
                STATIC_GATE,
                "their raw points' ±15% at 250 GHz and the line's loss: 20%",
            ),
        ),
        (
            "line/zhang-2022-rf-vpi-long",
            Rule::new(
                Relative,
                Reference,
                0.2,
                STATIC_GATE,
                "as the short electrode's: 20%",
            ),
        ),
        // ---- periodic lines ----
        (
            "line/periodic-uniform",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "a uniform line is periodic with any period: round-off over the sections' product",
            ),
        ),
        (
            "line/periodic-dispersion",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the half trace of two sections' ABCD matrices: round-off",
            ),
        ),
        (
            "line/periodic-band-edge",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "the band edge where (A + D)/2 reaches ±1, in closed form: round-off",
            ),
        ),
        (
            "line/periodic-shin-limit",
            Rule::new(
                Relative,
                Discretisation,
                1e-5,
                Measured,
                "the Bloch wave meets Shin et al.'s long-wave limit to second order in β_BΛ, at the case's period: 1e-5",
            ),
        ),
        (
            "line/shin-2005-velocity",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Stated(
                    0.075,
                    "the static grids' 0.5% gate, in their uncertainty (6.0 ± 0.4)",
                ),
                "their measured uncertainty: within one of it",
            ),
        ),
        (
            "line/shin-2005-impedance",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Stated(
                    0.072,
                    "the static grids' 0.5% gate, in their uncertainty (35.9 ± 2.5 Ω)",
                ),
                "their measured uncertainty: within one of it",
            ),
        ),
        (
            "line/kharel-2021-quartz-index",
            Rule::new(
                Relative,
                Reference,
                0.04,
                STATIC_GATE,
                "as their lines on silicon: 4%",
            ),
        ),
        (
            "line/kharel-2021-segmented-impedance",
            Rule::new(
                Relative,
                Reference,
                0.1,
                SEGMENTED,
                "the 2D sections' fringing (2%), the unstated etch and cladding (±3%) and ground (±2.5%): 10%",
            ),
        ),
        (
            "line/kharel-2021-segmented-index",
            Rule::new(
                Relative,
                Reference,
                0.05,
                SEGMENTED,
                "the 2D sections' fringing bound (+2.1%) and the unstated etch and cladding (±2.5%): 5%",
            ),
        ),
        (
            "line/kharel-2021-segmented-measured-index",
            Rule::new(
                Absolute,
                Reference,
                0.05,
                Stated(1.5e-3, "line/kharel-2021-segmented-grid's 6.7e-4 on 2.23"),
                "between the 2D sections' bounds, 0.047 above the lower, with their ±0.01: 0.05",
            ),
        ),
        (
            "line/kharel-2021-segmented-measured-loss",
            Rule::new(
                Relative,
                Reference,
                0.15,
                SEGMENTED,
                "the gold's ±5%, Holloway and Kuester's 5–10%, and the stems' charging current: 15%",
            ),
        ),
        (
            "line/kharel-2021-segmented-grid",
            Rule::new(
                Relative,
                Discretisation,
                5e-3,
                Measured,
                "Richardson's estimate from the three grids: the gate the published rows pass, 0.5%",
            ),
        ),
        // ---- the full-wave RF mode ----
        (
            "line/full-wave-plates",
            Rule::new(
                Relative,
                Discretisation,
                3e-5,
                Measured,
                "the Yee lattice's second order on 5 nm against the plates' exact TM mode",
            ),
        ),
        (
            "line/full-wave-plates-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "centred differences, the interfaces on the lattice's lines: second order, within 0.1",
            ),
        ),
        (
            "line/full-wave-balance",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "Poynting's theorem on the lattice, kept to the eigensolver's 1e-10 on β² over its conditioning",
            ),
        ),
        (
            "line/full-wave-quasi-tem-index",
            Rule::new(
                Relative,
                Model,
                5e-3,
                FULL_WAVE,
                "quasi-TEM with Holloway and Kuester's impedance, the line a two-hundredth of the wavelength (Heinrich: under 1.5%): 0.5%",
            ),
        ),
        (
            "line/full-wave-quasi-tem-impedance",
            Rule::new(
                Relative,
                Model,
                1e-2,
                FULL_WAVE,
                "Heinrich's quasi-TEM within 3% of full-wave, the two impedances one for a TEM line: 1%",
            ),
        ),
        (
            "line/full-wave-quasi-tem-loss",
            Rule::new(
                Relative,
                Model,
                5e-2,
                FULL_WAVE,
                "Holloway and Kuester's conductor loss within 5–10% of full-wave methods: 5%",
            ),
        ),
        (
            "line/full-wave-grid",
            Rule::new(
                Relative,
                Discretisation,
                5e-3,
                Measured,
                "Richardson's estimate from the three grids: the gate the published numbers pass, 0.5%",
            ),
        ),
        (
            "line/zhang-2022-full-wave-index",
            Rule::new(
                Absolute,
                Reference,
                0.04,
                Stated(2.9e-3, "line/full-wave-grid's 1.3e-3 on 2.26"),
                "their measured index 0.04 above the design at 250 GHz: 0.04",
            ),
        ),
        (
            "line/zhang-2022-full-wave-impedance",
            Rule::new(
                Absolute,
                Reference,
                1.5,
                Stated(5.4e-2, "line/full-wave-grid's 1.3e-3 on 41 Ω"),
                "41.5 Ω extracted from the measurement against 40.5 Ω simulated: 1.5 Ω",
            ),
        ),
        (
            "line/zhang-2022-full-wave-loss",
            Rule::new(
                Absolute,
                Reference,
                3.0,
                Stated(2.1e-2, "line/full-wave-grid's 1.3e-3 on 16 dB/cm"),
                "their figure's 16 dB/cm, their transmission's 13 and their fit's 12.9: 3 dB/cm",
            ),
        ),
        // ---- Heinrich's coplanar line ----
        (
            "line/heinrich-1993-external",
            Rule::new(
                Relative,
                Model,
                1e-2,
                HEINRICH,
                "his approximations within 1% of C and L_e∞",
            ),
        ),
        (
            "line/heinrich-1993-resistance-1ghz",
            Rule::new(
                Relative,
                Model,
                0.1,
                HEINRICH,
                "his Section VI: the resistance his model's main error, an order of 10%",
            ),
        ),
        (
            "line/heinrich-1993-resistance-10ghz",
            Rule::new(Relative, Model, 0.1, HEINRICH, "his Section VI: 10%"),
        ),
        (
            "line/heinrich-1993-resistance-100ghz",
            Rule::new(
                Relative,
                Model,
                0.1,
                HEINRICH,
                "his Section VI: the constant skin depth overestimates R at the edges, by an order of 10%",
            ),
        ),
        (
            "line/heinrich-1993-inductance-1ghz",
            Rule::new(
                Relative,
                Model,
                0.03,
                HEINRICH,
                "his Section VI: ε_eff and Z within 3% of his full-wave analysis",
            ),
        ),
        (
            "line/heinrich-1993-inductance-10ghz",
            Rule::new(Relative, Model, 0.03, HEINRICH, "his Section VI: 3%"),
        ),
        (
            "line/heinrich-1993-inductance-100ghz",
            Rule::new(Relative, Model, 0.03, HEINRICH, "his Section VI: 3%"),
        ),
        (
            "line/heinrich-1993-grid",
            Rule::new(
                Relative,
                Discretisation,
                5e-3,
                Measured,
                "Richardson's estimate from the three grids, faces and corners graded: the gate, 0.5%",
            ),
        ),
        // ---- Frankel's radiation ----
        (
            "line/frankel-1991-radiation-100ghz",
            Rule::new(
                Relative,
                Model,
                0.1,
                FRANKEL,
                "Eq. (9a) for wavelengths above s + 2w, and the window's own up to 3%: 10%",
            ),
        ),
        (
            "line/frankel-1991-radiation-200ghz",
            Rule::new(Relative, Model, 0.1, FRANKEL, "as at 100 GHz: 10%"),
        ),
        (
            "line/frankel-1991-radiation-400ghz",
            Rule::new(
                Relative,
                Model,
                0.1,
                FRANKEL,
                "the substrate's wavelength three times the line's at 400 GHz: 10%",
            ),
        ),
        (
            "line/frankel-1991-grid",
            Rule::new(
                Relative,
                Discretisation,
                5e-3,
                Measured,
                "the two finest graded grids' study, at least first order: the gate, 0.5%",
            ),
        ),
    ]
}

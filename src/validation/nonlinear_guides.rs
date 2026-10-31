//! The rules (#544) of the cases of second-order processes in waveguides (0.6.1,
//! [`crate::nonlinear::guide`]): the plane-wave identities to round-off and to the
//! interpolants' tolerance, the slab's overlap and group index to their grids' errors, the
//! coupled equations to RK4's, and the published guide to half the last printed digit, each with
//! our own grid study beside it.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let mut rules = modal_rules();
    rules.extend(overlap_rules());
    rules.extend(pair_rules());
    rules
}

/// Modal phase matching: the plane-wave limit and Zhao et al.'s guide.
fn modal_rules() -> Vec<(&'static str, Rule)> {
    let printed = |why| Rule::new(Absolute, Reference, 0.05, Noted, why);
    vec![
        (
            "nonlinear/guide-bulk-limit",
            Rule::new(
                Relative,
                Discretisation,
                3e-9,
                Measured,
                "the interpolants' 1e-10 in n_eff through Δk = (4π/λ)(n_2ω − n_ω), n_2ω − n_ω = 0.078: 2.6e-9, rounded up; a uniform field is the scheme's exact mode, so the grid adds nothing",
            ),
        ),
        (
            "nonlinear/guide-zhao-period",
            printed(
                "half a unit of the last digit of the printed 2.8 µm: the paper states no other uncertainty",
            ),
        ),
        (
            "nonlinear/guide-zhao-wavelength",
            Rule::new(
                Absolute,
                Reference,
                0.038,
                Noted,
                "the printed 2.8 µm's half digit: from 2.75 to 2.85 µm the matched fundamental moves by ±0.0389 µm in this guide (10 nm cells), rounded down",
            ),
        ),
        (
            "nonlinear/guide-zhao-area-signal",
            printed(
                "half a unit of the last digit of the printed 1.1 µm²: the paper states no other uncertainty",
            ),
        ),
        (
            "nonlinear/guide-zhao-area-pump",
            printed(
                "half a unit of the last digit of the printed 0.4 µm²: the paper states no other uncertainty",
            ),
        ),
    ]
}

/// The overlap's κ and the efficiency along a guide.
fn overlap_rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "nonlinear/guide-plane-wave",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "uniform fields make the overlap the closed form's product exactly: sums of a few hundred terms",
            ),
        ),
        (
            "nonlinear/guide-slab-overlap",
            Rule::new(
                Relative,
                Discretisation,
                1e-5,
                Noted,
                "the error on 5 nm cells, 7.1e-6, rounded up: second order (guide-slab-order), so 2.5 nm's is a quarter of it",
            ),
        ),
        (
            "nonlinear/guide-slab-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "second order within 0.1: the scheme's where the interfaces are straight and on nodes",
            ),
        ),
        (
            "nonlinear/guide-slab-coupled",
            Rule::new(
                Relative,
                Discretisation,
                1e-10,
                Measured,
                "RK4's error on 500 steps, 7.2e-11, rounded up: fourth order (1.2e-9, 7.2e-11, 4.5e-12 and 2.8e-13 on 250 to 2000 steps), so 1000's is a sixteenth of it",
            )
            .varying(),
        ),
    ]
}

/// The group index the joint spectrum takes.
fn pair_rules() -> Vec<(&'static str, Rule)> {
    vec![(
        "nonlinear/guide-group-index",
        Rule::new(
            Relative,
            Discretisation,
            2e-5,
            Noted,
            "the error on 5 nm cells, 1.6e-5, rounded up: second order (6.5e-5, 1.6e-5 and 4.0e-6 on 10, 5 and 2.5 nm)",
        ),
    )]
}

//! The rules (#544) of a mode's quantities' cases (#432): the effective area, the confinement
//! and the orthogonality of full-vector modes. A case against a closed form on our grid takes as
//! its tolerance the error on the grid twice as coarse (5 nm), rounded up to one digit: the bound
//! a converging scheme's 2.5 nm error must stay under, and its order case checks it is second.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "mode/effective-area-gaussian",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "a Gaussian's midpoint sums at 20 cells a width are exact to round-off, and the mirror images double the halves exactly",
            )
            .varying(),
        ),
        (
            "mode/effective-area-slab-te",
            Rule::new(
                Relative,
                Discretisation,
                3e-4,
                Measured,
                "the 5 nm grid's error, 2.16e-4, rounded up: the 2.5 nm grid's (5.4e-5) is a quarter of it at the second order mode/effective-area-slab-te-order checks",
            ),
        ),
        (
            "mode/effective-area-slab-te-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "second order within 0.1: the two halvings give 2.0006 and 2.0003, and 0.1 leaves room for the eigensolver's own convergence on the finest grid",
            ),
        ),
        (
            "mode/confinement-slab-te",
            Rule::new(
                Relative,
                Discretisation,
                8e-5,
                Measured,
                "the 5 nm grid's error, 7.4e-5, rounded up: the 2.5 nm grid's (1.8e-5) is a quarter of it at the second order mode/confinement-slab-order checks",
            ),
        ),
        (
            "mode/confinement-slab-tm",
            Rule::new(
                Relative,
                Discretisation,
                4e-4,
                Measured,
                "the 5 nm grid's error, 3.2e-4, rounded up: the 2.5 nm grid's is 7.4e-5, the halvings' orders 2.30, 2.17 and 2.10 from 20 nm, nearing second",
            ),
        ),
        (
            "mode/confinement-slab-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "second order within 0.1: the two halvings give 2.00 and 2.00, and 0.1 leaves room for the eigensolver's own convergence on the finest grid",
            ),
        ),
        (
            "mode/orthogonality-slab",
            Rule::new(
                Absolute,
                Discretisation,
                8e-5,
                Measured,
                "the 5 nm grid's largest product, 7.4e-5, rounded up: the 2.5 nm grid's (1.85e-5) is a quarter of it at the second order mode/orthogonality-slab-order checks",
            ),
        ),
        (
            "mode/orthogonality-slab-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "second order within 0.1: the two halvings give 2.003 and 2.001, and 0.1 leaves room for the eigensolver's own convergence on the finest grid",
            ),
        ),
    ]
}

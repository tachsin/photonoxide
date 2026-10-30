//! The rules (#544) of 0.6.1's data cases ([`crate::nonlinear::data_checks`]): undoped congruent
//! LiTaO₃'s extraordinary index against temperature, each against what its paper prints.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "material/litao3-barboza-thg-period",
            Rule::new(
                Absolute,
                Reference,
                0.005,
                Exact,
                "half a unit of the last digit of the printed 14.78 µm: the paper states no other uncertainty",
            ),
        ),
        (
            "material/litao3-barboza-tables",
            Rule::new(
                Relative,
                Reference,
                5e-4,
                Exact,
                "the paper's δn/n ≈ 5e-4 of the indices both tables were fitted to (p. 454): Table 2's own printed digits (A to three decimals moves n by 1.2e-4) don't reach Table 1's",
            ),
        ),
        (
            "material/litao3-bruner-slt-633",
            Rule::new(
                Absolute,
                Reference,
                5e-4,
                Exact,
                "half a unit of the last digit of the printed 2.177; from 20 to 30 °C the equation gives 2.1769 to 2.1770, so the unstated room temperature doesn't matter",
            ),
        ),
    ]
}

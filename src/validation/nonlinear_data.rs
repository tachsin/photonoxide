//! The rules (#544) of 0.6.1's data cases ([`crate::nonlinear::data_checks`]): undoped congruent
//! LiTaO₃'s extraordinary index against temperature and the Kerr data, each against what its
//! paper prints (half a unit of a printed digit, or a stated uncertainty); the unit conversions
//! and γ against their closed forms (round-off); Miller's rule against measured d, to the rule's
//! own stated spread; and a transparency window's band edge against the range its paper prints.

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
        (
            "material/kerr-desalvo-esu",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Exact,
                "half a unit of each value's last printed digit, the error measured in those halves: the paper states the pairs as the same numbers, to two digits",
            ),
        ),
        (
            "material/kerr-units-round-trip",
            Rule::new(
                Relative,
                RoundOff,
                1e-15,
                Exact,
                "a product and a quotient each way, each rounded once in f64 (2⁻⁵³ ≈ 1.1e-16): four roundings at most",
            ),
        ),
        (
            "material/kerr-bristow-readings",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Stated(0.27, "half a pixel of Fig. 3, 0.013 × 10⁻¹⁴ cm²/W, in halves of the last digit; Fig. 2's is 0.046"),
                "half a unit of the last digit of the printed 2.1, 0.4, 4.7 and 2.0, the error measured in those halves",
            ),
        ),
        (
            "nonlinear/kerr-gamma-gaussian",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "a Gaussian's midpoint sums at 20 cells a width are exact to round-off (as mode/effective-area-gaussian), and γ is that area's closed form",
            )
            .varying(),
        ),
        (
            "nonlinear/miller-linbo3-d33",
            miller(),
        ),
        (
            "nonlinear/miller-linbo3-d31",
            miller(),
        ),
        (
            "nonlinear/miller-linbo3-mgo-d33",
            miller(),
        ),
        (
            "nonlinear/miller-linbo3-mgo-d31",
            miller(),
        ),
    ]
}

/// The transparency windows' case.
pub(crate) fn transparency_rules() -> Vec<(&'static str, Rule)> {
    vec![(
        "material/transparency-gaas-gap",
        Rule::new(
            Absolute,
            Reference,
            0.0075,
            Exact,
            "the range of reported values Table I prints at 300 K, 1.420 to 1.435 eV: its half-width about its middle",
        ),
    )]
}

/// Miller's rule against measured d: the model's stated accuracy, Miller's δ "all equal to within
/// ±50 %" for one coefficient among crystals of a class (p. 17), which the observed deviation is
/// shown against; never fitted.
fn miller() -> Rule {
    Rule::new(
        Relative,
        Model,
        0.5,
        Exact,
        "Miller 1964's ±50 % for δ of one coefficient within a symmetry class (p. 17), the rule's own stated spread; the deviation shown is the finding",
    )
}

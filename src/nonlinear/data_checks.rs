//! The validation cases of 0.6.1's data: undoped congruent LiTaO₃'s extraordinary index against
//! temperature (Barboza & Cudney 2009, Bruner et al. 2002), each against the numbers its paper
//! prints.

use crate::material::Material;
use crate::material::catalogue::litao3_ne::{
    BARBOZA_DN_OVER_N, BARBOZA_PRISM_LAMBDAS, BRUNER_SLT, BRUNER_SLT_633, barboza, barboza_table_1,
    bruner_n2,
};
use crate::units::Wavelength;
use crate::verdict::{Case, Outcome, Tier};

fn n(material: &Material, lam: f64) -> f64 {
    Wavelength::um(lam)
        .and_then(|w| material.refractive_index(w))
        .map_or(f64::NAN, |n| n.re)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// Barboza & Cudney's period for Luo et al.'s third harmonic of 1.342 µm, by third-order
/// quasi-phase-matched sum-frequency mixing of the fundamental and its second harmonic at
/// 85.6 °C: Λ = 3 / (n₃/λ₃ − n₂/λ₂ − n₁/λ₁), µm.
pub(crate) fn barboza_thg_period() -> f64 {
    let Ok(m) = barboza(85.6 + 273.15) else {
        return f64::NAN;
    };
    let l1 = 1.342;
    let (l2, l3) = (l1 / 2.0, l1 / 3.0);
    3.0 / (n(&m, l3) / l3 - n(&m, l2) / l2 - n(&m, l1) / l1)
}

fn barboza_thg_case() -> Outcome {
    // "Our equation predicts that at 85.6 °C the required periodicity to obtain third-harmonic
    // generation is 14.78 µm" (p. 457)
    outcome(barboza_thg_period(), 14.78, 0.005)
}

/// Barboza & Cudney's Table 2 at 23 °C against their Table 1, the room-temperature fit, at the
/// seven wavelengths measured by minimum deviation: the largest relative difference.
pub(crate) fn barboza_tables() -> f64 {
    let (Ok(t2), Ok(t1)) = (barboza(296.15), barboza_table_1()) else {
        return f64::NAN;
    };
    BARBOZA_PRISM_LAMBDAS
        .iter()
        .map(|&lam| ((n(&t2, lam) - n(&t1, lam)) / n(&t1, lam)).abs())
        .fold(0.0, f64::max)
}

fn barboza_tables_case() -> Outcome {
    outcome(barboza_tables(), 0.0, BARBOZA_DN_OVER_N)
}

/// Bruner et al.'s equation for the stoichiometric crystal (Eq. (5), Table 1's SLT row) at
/// 633 nm and 25 °C: the supplier's index the fit was held to.
pub(crate) fn bruner_slt_633() -> f64 {
    let [a, b, c, d, e, f, g, h, kb, kc] = BRUNER_SLT;
    bruner_n2([a, b, c, d, e, f], (g, h), (kb, kc), 0.633, 25.0).sqrt()
}

fn bruner_slt_case() -> Outcome {
    outcome(bruner_slt_633(), BRUNER_SLT_633, 5e-4)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "material/litao3-barboza-thg-period",
            title: r"Congruent lithium tantalate (Barboza & Cudney, $n_e(\lambda, T)$): the period of third-order quasi-phase-matched sum-frequency mixing of 1.342 µm and its second harmonic into the third, at 85.6 °C, $\Lambda = 3/(n_3/\lambda_3 - n_2/\lambda_2 - n_1/\lambda_1)$ (µm, shown)",
            tier: Tier::Published,
            source: r"N. A. Barboza, R. S. Cudney, Appl. Phys. B 95, 453 (2009), doi:10.1007/s00340-009-3434-y, p. 457: '14.78 µm' for Luo et al.'s simultaneous second and third harmonics (the text's '1.342 nm' read as µm), half a unit of its last digit",
            run: barboza_thg_case,
        },
        Case {
            id: "material/litao3-barboza-tables",
            title: r"Congruent lithium tantalate: Barboza & Cudney's Table 2 (with temperature) at 23 °C against their Table 1 (the 23 °C fit), at the seven wavelengths measured by minimum deviation, 457.9 to 1064.2 nm (largest relative difference shown)",
            tier: Tier::Published,
            source: r"N. A. Barboza, R. S. Cudney, Appl. Phys. B 95, 453 (2009), doi:10.1007/s00340-009-3434-y, Eq. (7), Tables 1 and 2 (p. 455); the indices' stated $\delta n/n \approx 5 \times 10^{-4}$ (p. 454)",
            run: barboza_tables_case,
        },
        Case {
            id: "material/litao3-bruner-slt-633",
            title: r"Bruner et al.'s equation for stoichiometric lithium tantalate (Eq. (5), Table 1's SLT row, $b$ and $c$ in $(T + 273.15)^2$) at 633 nm and 25 °C, the supplier's index their fit was held to (shown): the check of our reading of the form the congruent row shares",
            tier: Tier::Published,
            source: r"A. Bruner et al., Proc. SPIE 4628, 66 (2002), doi:10.1117/12.460817, p. 70: 'The refractive index for SLT at 633 nm was measured by the supplier, and is 2.177. We used this value as a constant reference point', half a unit of its last digit",
            run: bruner_slt_case,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_passes_and_the_numbers() {
        for case in cases() {
            let o = (case.run)();
            assert!(o.passed(), "{}: {o:?}", case.id);
        }
        // the third harmonic's period, 14.7826 µm against the printed 14.78
        assert!((barboza_thg_period() - 14.7826).abs() < 1e-4);
        assert!(
            (barboza_tables() - 1.12e-4).abs() < 1e-6,
            "{}",
            barboza_tables()
        );
        assert!((bruner_slt_633() - 2.17697).abs() < 1e-5);
    }
}

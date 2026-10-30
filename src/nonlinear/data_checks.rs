//! The validation cases of 0.6.1's data: undoped congruent LiTaO₃'s extraordinary index against
//! temperature (Barboza & Cudney 2009, Bruner et al. 2002), each against the numbers its paper
//! prints; the Kerr data's units, readings and γ; Miller's rule against the catalogue's own d
//! coefficients measured at several wavelengths; and GaAs's band edge against the reported range.

use crate::material::Material;
use crate::material::catalogue::kerr::{BRISTOW_1220, BRISTOW_FIGURES};
use crate::material::catalogue::litao3_ne::{
    BARBOZA_DN_OVER_N, BARBOZA_PRISM_LAMBDAS, BRUNER_SLT, BRUNER_SLT_633, barboza, barboza_table_1,
    bruner_n2,
};
use crate::material::catalogue::{
    self, Axis, Conditions, KerrQuantity, TensorKind, beta_from_cm_per_gw, beta_to_cm_per_gw, kerr,
    n2_from_cm2_per_w, n2_from_esu, n2_to_cm2_per_w, n2_to_esu,
};
use crate::nonlinear::miller::miller_shg;
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

// ---------------------------------------------------------------------------------------------
// The Kerr data
// ---------------------------------------------------------------------------------------------

/// DeSalvo et al.'s two examples of their unit conversion, BaF₂ at 532 and 266 nm (p. 1327):
/// (n₂ in cm²/W, n₂ in esu, n₀ of Table II, the half units of the last printed digits of each).
pub(crate) const DESALVO_EXAMPLES: [(f64, f64, f64, f64, f64); 2] = [
    (2.1e-16, 7.5e-14, 1.49, 0.05e-16, 0.05e-14),
    (3.1e-16, 1.1e-13, 1.5, 0.05e-16, 0.05e-13),
];

/// Each example converted both ways against the other printed value: the largest difference in
/// halves of that value's last printed digit.
pub(crate) fn desalvo_esu() -> f64 {
    DESALVO_EXAMPLES
        .iter()
        .map(|&(si, esu, n0, half_si, half_esu)| {
            let to_esu = (n2_to_esu(n2_from_cm2_per_w(si), n0) - esu).abs() / half_esu;
            let to_si = (n2_to_cm2_per_w(n2_from_esu(esu, n0)) - si).abs() / half_si;
            to_esu.max(to_si)
        })
        .fold(0.0, f64::max)
}

fn desalvo_esu_case() -> Outcome {
    outcome(desalvo_esu(), 0.0, 1.0)
}

/// Every Kerr value of the catalogue taken to its printed unit and back (esu with n₀ = 2.2,
/// cm²/W, cm/GW): the largest relative difference from the value.
pub(crate) fn units_round_trip() -> f64 {
    ["si", "si3n4", "linbo3", "aln"]
        .iter()
        .flat_map(|id| kerr(id))
        .map(|k| {
            let v = k.value;
            let back = match k.quantity {
                KerrQuantity::NonlinearIndex => [
                    n2_from_cm2_per_w(n2_to_cm2_per_w(v)),
                    n2_from_esu(n2_to_esu(v, 2.2), 2.2),
                ],
                _ => [beta_from_cm_per_gw(beta_to_cm_per_gw(v)); 2],
            };
            back.iter().map(|b| ((b - v) / v).abs()).fold(0.0, f64::max)
        })
        .fold(0.0, f64::max)
}

fn units_round_trip_case() -> Outcome {
    outcome(units_round_trip(), 0.0, 1e-15)
}

/// Bristow et al.'s figures read at 1220 nm, where their text prints the values: β and its bar
/// (Fig. 2) and n₂ and its bar (Fig. 3) against 2.1 ± 0.4 cm/GW and (4.7 ± 2.0) × 10⁻¹⁴ cm²/W,
/// the largest difference in halves of the printed last digit (0.05 in each).
pub(crate) fn bristow_readings() -> f64 {
    let [beta, _, n2, _] = BRISTOW_FIGURES;
    let [(b, db), (n, dn)] = BRISTOW_1220;
    [
        beta.value() - b,
        beta.half_bar() - db,
        n2.value() - n,
        n2.half_bar() - dn,
    ]
    .iter()
    .map(|d| d.abs() / 0.05)
    .fold(0.0, f64::max)
}

fn bristow_readings_case() -> Outcome {
    outcome(bristow_readings(), 0.0, 1.0)
}

/// A Gaussian mode of area π µm² (E = e^(−r²/w²), w = 1 µm, on 50 nm cells) with the catalogue's silicon
/// nitride n₂ at its wavelength: γ by [`crate::mode::fields::Fields::kerr_coefficient`] against
/// 2π n₂ / (λ A_eff) in SI, relative difference.
pub(crate) fn gamma_gaussian() -> f64 {
    let k = &kerr("si3n4")[0];
    let Ok(lam) = Wavelength::um(k.wavelength) else {
        return f64::NAN;
    };
    let f =
        crate::mode::quantities_checks::gaussian(1.0, 0.05, [-6.0, -6.0], [240, 240], [None, None]);
    let gamma = f.kerr_coefficient(lam, k).unwrap_or(f64::NAN);
    let exact =
        std::f64::consts::TAU * k.value / (k.wavelength * 1e-6 * std::f64::consts::PI * 1e-12);
    (gamma / exact - 1.0).abs()
}

fn gamma_gaussian_case() -> Outcome {
    outcome(gamma_gaussian(), 0.0, 1e-10)
}

// ---------------------------------------------------------------------------------------------
// Miller's rule
// ---------------------------------------------------------------------------------------------

/// The fundamental the catalogue's d is carried from, µm, and the ones it is compared at.
pub(crate) const MILLER_FROM: f64 = 1.064;
pub(crate) const MILLER_TO: [f64; 2] = [1.313, 0.852];

/// Miller's rule from the entry's d at 1.064 µm to 1.313 and 0.852 µm, with the entry's default
/// index model, against the entry's own d there: each prediction's relative deviation, d₃₃
/// (`d33`) or d₃₁.
pub(crate) fn miller_deviations(entry: &str, d33: bool) -> Vec<(f64, f64, f64)> {
    let Some(e) = catalogue::entry(entry) else {
        return vec![(f64::NAN, f64::NAN, f64::NAN)];
    };
    let Some(model) = e.default_model() else {
        return vec![(f64::NAN, f64::NAN, f64::NAN)];
    };
    let Ok(materials) = model.materials(Conditions::default()) else {
        return vec![(f64::NAN, f64::NAN, f64::NAN)];
    };
    let axis = |a: Axis| {
        model
            .axes
            .iter()
            .position(|x| *x == a)
            .map(|k| &materials[k])
    };
    let (Some(o), Some(ex)) = (axis(Axis::Ordinary), axis(Axis::Extraordinary)) else {
        return vec![(f64::NAN, f64::NAN, f64::NAN)];
    };
    let col = if d33 { 3 } else { 1 };
    let d_at = |lam: f64| {
        e.tensors
            .iter()
            .filter(|t| t.kind == TensorKind::SecondOrder && t.wavelength == Some(lam))
            .find_map(|t| t.value(3, col))
            .unwrap_or(f64::NAN)
    };
    let fundamental = if d33 { [ex, ex] } else { [o, o] };
    let from = d_at(MILLER_FROM);
    MILLER_TO
        .iter()
        .map(|&lam| {
            let predicted = match (Wavelength::um(MILLER_FROM), Wavelength::um(lam)) {
                (Ok(a), Ok(b)) => miller_shg(from, a, b, ex, fundamental).unwrap_or(f64::NAN),
                _ => f64::NAN,
            };
            let measured = d_at(lam);
            (lam, predicted, (predicted / measured - 1.0).abs())
        })
        .collect()
}

fn miller_case(entry: &str, d33: bool) -> Outcome {
    let worst = miller_deviations(entry, d33)
        .iter()
        .map(|r| r.2)
        .fold(
            0.0,
            |w: f64, e| if e.is_nan() { f64::NAN } else { w.max(e) },
        );
    outcome(worst, 0.0, 0.5)
}

fn miller_linbo3_d33() -> Outcome {
    miller_case("linbo3", true)
}

fn miller_linbo3_d31() -> Outcome {
    miller_case("linbo3", false)
}

fn miller_mgo_d33() -> Outcome {
    miller_case("linbo3-mgo", true)
}

fn miller_mgo_d31() -> Outcome {
    miller_case("linbo3-mgo", false)
}

// ---------------------------------------------------------------------------------------------
// Transparency windows
// ---------------------------------------------------------------------------------------------

/// GaAs's band edge at 300 K from Vurgaftman et al.'s Table I (the Varshni form), in eV: hc/λ
/// of the catalogue's edge.
pub(crate) fn gaas_gap_300() -> f64 {
    catalogue::transparency(
        "gaas",
        Conditions {
            temperature: Some(300.0),
            composition: None,
        },
    )
    .and_then(|t| t.band_edge.wavelength)
    .map_or(f64::NAN, |lam| 1.239_841_98 / lam)
}

fn gaas_gap_case() -> Outcome {
    // Table I's range of reported values at 300 K, 1.420 to 1.435 eV: its middle and half-width
    outcome(gaas_gap_300(), 1.4275, 0.0075)
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
        Case {
            id: "material/kerr-desalvo-esu",
            title: r"Kerr coefficients in esu: $n_2(\mathrm{esu}) = (c n_0/40\pi)\thinspace n_2(\mathrm{m^2/W})$ both ways on DeSalvo et al.'s two printed examples, BaF₂ at 532 nm ((2.1 ± 0.4) × 10⁻¹⁶ cm²/W as (7.5 ± 1.5) × 10⁻¹⁴ esu, $n_0$ = 1.49) and at 266 nm ((3.1 ± 0.6) × 10⁻¹⁶ cm²/W as (1.1 ± 0.2) × 10⁻¹³ esu, $n_0$ = 1.5) (largest difference shown, in halves of each value's last printed digit)",
            tier: Tier::Published,
            source: "R. DeSalvo et al., IEEE J. Quantum Electron. 32, 1324 (1996), doi:10.1109/3.511545, p. 1327, with Table II's indices; each value to half a unit of its last printed digit",
            run: desalvo_esu_case,
        },
        Case {
            id: "material/kerr-units-round-trip",
            title: r"Every Kerr value of the catalogue (silicon, silicon nitride, lithium niobate, aluminium nitride) taken from SI to cm²/W, to esu ($n_0$ = 2.2) or to cm/GW and back (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"1 cm²/W = 1e-4 m²/W, 1 cm/GW = 1e-11 m/W, and the esu relation's inverse: exact identities",
            run: units_round_trip_case,
        },
        Case {
            id: "material/kerr-bristow-readings",
            title: r"Silicon's $\beta$ and $n_2$ read off Bristow et al.'s Figs. 2 and 3 (the PDF's bitmaps, against the axes' ticks) at 1220 nm, where the text prints them: the markers' centres and half their error bars against 2.1 ± 0.4 cm/GW and (4.7 ± 2.0) × 10⁻¹⁴ cm²/W, the check of the reading the 1550 nm values come from (largest difference shown, in halves of the last printed digit)",
            tier: Tier::Published,
            source: "A. D. Bristow, N. Rotenberg, H. M. van Driel, Appl. Phys. Lett. 90, 191104 (2007), doi:10.1063/1.2737359, p. 191104-2 and Figs. 2 and 3; half a unit of the last printed digit",
            run: bristow_readings_case,
        },
        Case {
            id: "nonlinear/kerr-gamma-gaussian",
            title: r"A guide's nonlinear coefficient from a catalogue $n_2$: a Gaussian mode of area π µm² with silicon nitride's $n_2$ (Ikeda et al., 2.4 × 10⁻¹⁵ cm²/W at 1548 nm), $\gamma$ by the mode's effective area against $2\pi n_2/(\lambda A_\text{eff})$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\gamma = 2\pi n_2/(\lambda A_\text{eff})$ (Afshar & Monro, Opt. Express 17, 2298 (2009), doi:10.1364/OE.17.002298, Eq. 42 with $n_2$ uniform); the Gaussian's area $\pi w^2$",
            run: gamma_gaussian_case,
        },
        Case {
            id: "nonlinear/miller-linbo3-d33",
            title: r"Miller's rule, congruent lithium niobate's $d_{33}$ carried from 1.064 µm to 1.313 and 0.852 µm with Zelmon's $n_e$, $d \propto \chi(2\omega)\chi(\omega)^2$, against Shoji's $d_{33}$ measured there (largest relative deviation shown)",
            tier: Tier::Published,
            source: "R. C. Miller, Appl. Phys. Lett. 5, 17 (1964), doi:10.1063/1.1754022, Eq. (4) and its ±50 % (p. 17); I. Shoji et al., J. Opt. Soc. Am. B 14, 2268 (1997), doi:10.1364/JOSAB.14.002268, Table 10 (25.2, 19.5 and 25.7 pm/V)",
            run: miller_linbo3_d33,
        },
        Case {
            id: "nonlinear/miller-linbo3-d31",
            title: r"Miller's rule, congruent lithium niobate's $d_{31}$ carried from 1.064 µm to 1.313 and 0.852 µm with Zelmon's $n_e$ at the harmonic and $n_o$ at the fundamental, against Shoji's $d_{31}$ measured there (largest relative deviation shown)",
            tier: Tier::Published,
            source: "R. C. Miller, Appl. Phys. Lett. 5, 17 (1964), doi:10.1063/1.1754022, Eq. (4) and its ±50 % (p. 17); I. Shoji et al., J. Opt. Soc. Am. B 14, 2268 (1997), doi:10.1364/JOSAB.14.002268, Table 10 (4.6, 3.2 and 4.8 pm/V)",
            run: miller_linbo3_d31,
        },
        Case {
            id: "nonlinear/miller-linbo3-mgo-d33",
            title: r"Miller's rule, 5% MgO-doped lithium niobate's $d_{33}$ carried from 1.064 µm to 1.313 and 0.852 µm with Zelmon's $n_e$ (Table 2, columns exchanged), against Shoji's $d_{33}$ measured there (largest relative deviation shown)",
            tier: Tier::Published,
            source: "R. C. Miller, Appl. Phys. Lett. 5, 17 (1964), doi:10.1063/1.1754022, Eq. (4) and its ±50 % (p. 17); I. Shoji et al., J. Opt. Soc. Am. B 14, 2268 (1997), doi:10.1364/JOSAB.14.002268, Table 10 (25.0, 20.3 and 28.4 pm/V)",
            run: miller_mgo_d33,
        },
        Case {
            id: "nonlinear/miller-linbo3-mgo-d31",
            title: r"Miller's rule, 5% MgO-doped lithium niobate's $d_{31}$ carried from 1.064 µm to 1.313 and 0.852 µm, $n_e$ at the harmonic and $n_o$ at the fundamental, against Shoji's $d_{31}$ measured there (largest relative deviation shown)",
            tier: Tier::Published,
            source: "R. C. Miller, Appl. Phys. Lett. 5, 17 (1964), doi:10.1063/1.1754022, Eq. (4) and its ±50 % (p. 17); I. Shoji et al., J. Opt. Soc. Am. B 14, 2268 (1997), doi:10.1364/JOSAB.14.002268, Table 10 (4.4, 3.4 and 4.9 pm/V)",
            run: miller_mgo_d31,
        },
        Case {
            id: "material/transparency-gaas-gap",
            title: r"GaAs's band edge at 300 K, the catalogue's transparency window: the direct gap from Vurgaftman et al.'s Varshni parameters ($E_g(0)$ = 1.519 eV, $\alpha$ = 0.5405 meV/K, $\beta$ = 204 K) against the range of reported values their Table I gives at 300 K, 1.420 to 1.435 eV (eV, shown)",
            tier: Tier::Published,
            source: "I. Vurgaftman, J. R. Meyer, L. R. Ram-Mohan, J. Appl. Phys. 89, 5815 (2001), doi:10.1063/1.1368156, Table I (p. 5825) and Eq. (2.13) (p. 5818); the range's middle and half-width",
            run: gaas_gap_case,
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
        // Miller's rule: d33 of congruent LiNbO₃ predicted 23.6 pm/V at 1.313 µm, where 19.5 is
        // measured (+21 %); 28.1 at 0.852, where 25.7 (+9 %)
        let d = miller_deviations("linbo3", true);
        assert!(
            (d[0].1 - 23.58).abs() < 0.005 && (d[1].1 - 28.12).abs() < 0.005,
            "{d:?}"
        );
        // the write-up's table: the deviations at 1.313 and 0.852 µm, in percent
        for (entry, d33, at_1313, at_852) in [
            ("linbo3", true, 20.9, 9.4),
            ("linbo3", false, 34.2, 7.3),
            ("linbo3-mgo", true, 15.4, 2.0),
            ("linbo3-mgo", false, 20.9, 0.4),
        ] {
            let d = miller_deviations(entry, d33);
            assert!(
                (100.0 * d[0].2 - at_1313).abs() < 0.05,
                "{entry} {d33} {d:?}"
            );
            assert!(
                (100.0 * d[1].2 - at_852).abs() < 0.05,
                "{entry} {d33} {d:?}"
            );
        }
    }
}

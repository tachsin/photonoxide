//! The validation cases of the microwave permittivities and loss tangents: Krupka et al.'s
//! Table I through their Eq. (3), and Lamb's Table I's ε′ against its n.

use super::rf::{RfValue, rf_dielectric};
use crate::validation::Outcome;

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// Krupka et al.'s Table I, room temperature: the frequency (GHz), the unloaded Q factor Q_u,
/// the parasitic losses' Q_p, the sample's electric energy filling factor p_e and tan δ, as
/// printed.
pub(crate) const KRUPKA_TABLE_I: [(f64, f64, f64, f64, f64); 3] = [
    (6.685, 550.0, 38_950.0, 0.9605, 0.001866),
    (6.797, 1134.0, 18_030.0, 0.4346, 0.001901),
    (4.824, 4036.0, 16_200.0, 0.0885, 0.002101),
];

/// Their Eq. (3): tan δ = (1/Q_u − 1/Q_p)/p_e.
pub(crate) fn krupka_eq_3(q_u: f64, q_p: f64, p_e: f64) -> f64 {
    (1.0 / q_u - 1.0 / q_p) / p_e
}

/// The catalogue's values of `id` from the paper `key`.
fn values(id: &str, key: &str) -> Vec<RfValue> {
    rf_dielectric(id)
        .map(|m| {
            m.values
                .into_iter()
                .filter(|v| v.source.reference == key)
                .collect()
        })
        .unwrap_or_default()
}

/// Eq. (3) on each row of Table I's Q factors and filling factor, against the catalogue's
/// tan δ at that frequency (the table's): the largest relative deviation.
pub(crate) fn krupka_table_1() -> Outcome {
    let catalogue = values("si", "krupka-2006");
    let worst = KRUPKA_TABLE_I
        .iter()
        .map(|&(f, q_u, q_p, p_e, _)| {
            catalogue
                .iter()
                .find(|v| v.frequency_ghz.0 == f)
                .map_or(f64::NAN, |v| {
                    (krupka_eq_3(q_u, q_p, p_e) / v.loss_tangent.0 - 1.0).abs()
                })
        })
        .fold(0.0, |w: f64, d| if d.is_nan() { d } else { w.max(d) });
    outcome(worst, 0.0, 1e-3)
}

/// The refractive index Lamb prints beside each ε′ the catalogue takes from his Table I, in the
/// catalogue's order (fused silica, quartz, sapphire, silicon): the frequency (GHz), n at the
/// ends of its range (equal for one value), and the decimals of n and of ε′ as printed.
pub(crate) const LAMB_N: [(f64, (f64, f64), u32, u32); 22] = [
    // fused silica: Spectrosil (10 GHz), QI, QV, QU (150), Dynasil, Spectrosil WF (245), Spectrosil
    (10.0, (1.944, 1.944), 3, 2),
    (150.0, (1.952, 1.952), 3, 4),
    (150.0, (1.953, 1.953), 3, 4),
    (150.0, (1.958, 1.958), 3, 4),
    (245.0, (1.955, 1.955), 3, 3),
    (245.0, (1.9516, 1.9516), 4, 4),
    (2.0, (1.962, 1.962), 3, 2),
    // crystalline quartz: o 35, o and e 140, o and e 245, o and e 30–900 GHz
    (35.0, (2.105, 2.105), 3, 2),
    (140.0, (2.1076, 2.1076), 4, 4),
    (140.0, (2.1550, 2.1550), 4, 4),
    (245.0, (2.1059, 2.1059), 4, 4),
    (245.0, (2.1533, 2.1533), 4, 4),
    (300.0, (2.106, 2.106), 3, 3),
    (300.0, (2.154, 2.154), 3, 3),
    // sapphire: z-cut 100, α o and e 140, z-cut 200 and 300 GHz
    (100.0, (3.06396, 3.06396), 5, 4),
    (140.0, (3.066, 3.071), 3, 3),
    (140.0, (3.400, 3.405), 3, 3),
    (200.0, (3.06356, 3.06356), 5, 4),
    (300.0, (3.06350, 3.06350), 5, 4),
    // silicon: 100, 245, 250 GHz
    (100.0, (3.4464, 3.4464), 4, 3),
    (245.0, (3.4182, 3.4182), 4, 3),
    (250.0, (3.4471, 3.4471), 4, 3),
];

/// Lamb's Eq. (6), ε′ = n² for a low-loss material, on every row the catalogue takes from his
/// Table I (both ends of a range): |n² − ε′| over the printed digits' bound, half a unit of ε′'s
/// last and 2n times half a unit of n's; the largest.
pub(crate) fn lamb_table_1() -> Outcome {
    let rows: Vec<RfValue> = ["sio2", "quartz", "sapphire", "si"]
        .iter()
        .flat_map(|id| values(id, "lamb-1996"))
        .filter(|v| v.permittivity.is_some())
        .collect();
    if rows.len() != LAMB_N.len() {
        return outcome(f64::NAN, 0.0, 1.0);
    }
    let worst = rows
        .iter()
        .zip(LAMB_N)
        .map(|(v, (f, (n0, n1), dn, de))| {
            let Some((e0, e1)) = v.permittivity else {
                return f64::NAN;
            };
            if v.frequency_ghz.0 != f {
                return f64::NAN;
            }
            let bound = |n: f64| 0.5 * 10f64.powi(-(de as i32)) + n * 10f64.powi(-(dn as i32));
            ((n0 * n0 - e0).abs() / bound(n0)).max((n1 * n1 - e1).abs() / bound(n1))
        })
        .fold(0.0, |w: f64, d| if d.is_nan() { d } else { w.max(d) });
    outcome(worst, 0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_passes() {
        for (name, o) in [("krupka", krupka_table_1()), ("lamb", lamb_table_1())] {
            assert!(o.passed(), "{name}: {o:?}");
        }
    }

    #[test]
    fn the_numbers() {
        // Eq. (3) gives 1.8663e-3, 1.9015e-3 and 2.1022e-3: the third 5.6e-4 above its print
        let tan: Vec<f64> = KRUPKA_TABLE_I
            .iter()
            .map(|&(_, q_u, q_p, p_e, _)| krupka_eq_3(q_u, q_p, p_e))
            .collect();
        for (t, e) in tan.iter().zip([1.8663e-3, 1.9015e-3, 2.1022e-3]) {
            assert!((t - e).abs() < 1e-7, "{t}");
        }
        let o = krupka_table_1();
        assert!((o.measured - 5.6e-4).abs() < 1e-5, "{o:?}");
        // the largest of Lamb's rows, z-cut sapphire at 100 GHz: 3.06396² = 9.38785 against
        // 9.3879, 0.61 of the printed digits' bound
        let o = lamb_table_1();
        assert!((o.measured - 0.609).abs() < 1e-3, "{o:?}");
        // the row left out: fused silica at 393 GHz (his ref. 20), n 1.9469 against ε′ 3.801,
        // whose root is 1.9496, the same digits with two exchanged
        assert!((1.9469f64 * 1.9469 - 3.801).abs() > 0.01);
        assert!((3.801f64.sqrt() - 1.9496).abs() < 5e-5);
    }
}

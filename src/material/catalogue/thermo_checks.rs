//! The validation cases of temperature in the catalogue: silicon's and silica's n(λ, T) and
//! dn/dT against the tables their papers print.

use super::checks::{FREY_TABLE_2_T, LEVITON_TABLE_4_T};
use super::{Conditions, entry, thermo};
use crate::material::Material;
use crate::units::Wavelength;
use crate::validation::Outcome;

fn n(material: &Material, lam: f64) -> f64 {
    Wavelength::um(lam)
        .and_then(|w| material.refractive_index(w))
        .map_or(f64::NAN, |n| n.re)
}

fn largest(errors: impl Iterator<Item = f64>) -> f64 {
    errors.fold(
        0.0,
        |a: f64, e| if e.is_nan() { f64::NAN } else { a.max(e) },
    )
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn deviation(largest: f64, tolerance: f64) -> Outcome {
    outcome(largest, 0.0, tolerance)
}

/// dn/dT, per K, of the catalogue's model `model` of `id` at λ (µm) and T (K).
pub(crate) fn dndt(id: &str, model: &str, lam: f64, t: f64) -> f64 {
    entry(id)
        .and_then(|e| e.index.into_iter().find(|m| m.id == model))
        .and_then(|m| {
            let c = Conditions {
                temperature: Some(t),
                ..Conditions::default()
            };
            Wavelength::um(lam).and_then(|w| m.thermo_optic(w, c)).ok()
        })
        .map_or(f64::NAN, |d| d[0])
}

/// The temperatures (K) of Li's Tables 1 and 2.
pub(crate) const LI_TABLE_T: [f64; 14] = [
    100.0, 150.0, 200.0, 250.0, 293.0, 350.0, 400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0,
    750.0,
];

/// Li's Table 1: n of silicon at seven of its wavelengths (µm) and every temperature, as
/// printed (the fourth decimal "insignificant", for smoothness). The rows from 1.32 to 1.60 µm,
/// with entries struck through on the page, are left out.
#[rustfmt::skip]
pub(crate) const LI_TABLE_1: [(f64, [f64; 14]); 7] = [
    (1.2, [3.4845, 3.4915, 3.4995, 3.5084, 3.5167, 3.5284, 3.5393, 3.5508, 3.5627, 3.5749, 3.5873, 3.5999, 3.6126, 3.6252]),
    (1.3, [3.4706, 3.4773, 3.4850, 3.4936, 3.5016, 3.5130, 3.5236, 3.5348, 3.5463, 3.5582, 3.5704, 3.5827, 3.5950, 3.6074]),
    (2.0, [3.4240, 3.4297, 3.4363, 3.4439, 3.4510, 3.4612, 3.4708, 3.4809, 3.4915, 3.5023, 3.5135, 3.5247, 3.5360, 3.5473]),
    (3.0, [3.4048, 3.4101, 3.4163, 3.4234, 3.4302, 3.4399, 3.4491, 3.4588, 3.4689, 3.4794, 3.4900, 3.5009, 3.5117, 3.5226]),
    (5.0, [3.3950, 3.4000, 3.4060, 3.4129, 3.4195, 3.4290, 3.4380, 3.4474, 3.4573, 3.4675, 3.4780, 3.4886, 3.4992, 3.5099]),
    (10.0, [3.3909, 3.3958, 3.4017, 3.4085, 3.4150, 3.4244, 3.4332, 3.4426, 3.4524, 3.4625, 3.4729, 3.4834, 3.4940, 3.5045]),
    (14.0, [3.3902, 3.3951, 3.4010, 3.4078, 3.4142, 3.4236, 3.4325, 3.4418, 3.4516, 3.4617, 3.4721, 3.4826, 3.4931, 3.5036]),
];

/// Li's Table 2: dn/dT of silicon, in units of 1e-4 per K, at seven wavelengths (µm) and every
/// temperature, as printed (the third decimal "insignificant").
#[rustfmt::skip]
pub(crate) const LI_TABLE_2: [(f64, [f64; 14]); 7] = [
    (1.2, [1.305, 1.509, 1.693, 1.857, 1.983, 2.132, 2.244, 2.335, 2.411, 2.468, 2.508, 2.530, 2.535, 2.523]),
    (1.28, [1.254, 1.458, 1.642, 1.807, 1.934, 2.083, 2.194, 2.287, 2.361, 2.418, 2.457, 2.479, 2.484, 2.472]),
    (2.0, [1.033, 1.238, 1.423, 1.589, 1.717, 1.866, 1.977, 2.070, 2.144, 2.200, 2.238, 2.259, 2.262, 2.248]),
    (3.0, [0.947, 1.151, 1.337, 1.504, 1.632, 1.781, 1.892, 1.985, 2.058, 2.114, 2.152, 2.172, 2.175, 2.160]),
    (5.0, [0.902, 1.107, 1.293, 1.460, 1.588, 1.737, 1.848, 1.940, 2.014, 2.070, 2.108, 2.127, 2.130, 2.114]),
    (10.0, [0.883, 1.088, 1.274, 1.441, 1.570, 1.719, 1.830, 1.922, 1.996, 2.051, 2.089, 2.108, 2.110, 2.095]),
    (14.0, [0.880, 1.085, 1.271, 1.438, 1.567, 1.716, 1.827, 1.919, 1.992, 2.048, 2.086, 2.105, 2.107, 2.092]),
];

/// Li's Table 1 from his Table 1 at 293 K moved by Eq. (22) (the default silicon from 250 K
/// up): the largest difference over the 98 entries, within three units of the printed fourth
/// decimal (the table at 293 K rounded, Eq. (22) to the other columns rounded).
pub(crate) fn li_table_1() -> Outcome {
    let errors = LI_TABLE_T.iter().enumerate().flat_map(|(j, &t)| {
        let m = thermo::li_eq22_silicon(t).ok();
        LI_TABLE_1
            .iter()
            .map(move |(lam, row)| m.as_ref().map_or(f64::NAN, |m| (n(m, *lam) - row[j]).abs()))
            .collect::<Vec<_>>()
    });
    deviation(largest(errors), 3e-4)
}

/// Li's Table 2 from Eq. (22)'s derivative (the default silicon's dn/dT from 250 K up): the
/// largest difference over the 98 entries, in units of 1e-4 per K, within five units of the
/// printed third decimal.
pub(crate) fn li_table_2() -> Outcome {
    let errors = LI_TABLE_T.iter().enumerate().flat_map(|(j, &t)| {
        LI_TABLE_2
            .iter()
            .map(move |(lam, row)| (thermo::li_eq22(*lam, t).1 * 1e4 - row[j]).abs())
    });
    deviation(largest(errors), 0.005)
}

/// The order at which central differences of Eq. (22), 1 and 0.5 K wide, approach its dn/dT in
/// closed form at 1.55 µm and 400 K (2 for a central difference).
pub(crate) fn li_dndt_order() -> Outcome {
    let (lam, t) = (1.55, 400.0);
    let exact = thermo::li_eq22(lam, t).1;
    let error = |h: f64| {
        let d = (thermo::li_eq22(lam, t + h).0 - thermo::li_eq22(lam, t - h).0) / (2.0 * h);
        (d - exact).abs()
    };
    outcome((error(1.0) / error(0.5)).log2(), 2.0, 0.05)
}

/// Frey et al.'s Table 4: measured dn/dT of silicon (per K) at their Table 2's wavelengths (µm)
/// and temperatures ([`FREY_TABLE_2_T`]), printed to three figures.
#[rustfmt::skip]
pub(crate) const FREY_TABLE_4: [(f64, [f64; 12]); 13] = [
    (1.1, [6.70E-06, 1.79E-05, 2.91E-05, 4.04E-05, 5.16E-05, 6.28E-05, 7.40E-05, 8.52E-05, 1.30E-04, 1.59E-04, 1.88E-04, 2.15E-04]),
    (1.2, [6.40E-06, 1.72E-05, 2.80E-05, 3.86E-05, 4.95E-05, 6.08E-05, 7.18E-05, 8.14E-05, 1.26E-04, 1.52E-04, 1.78E-04, 2.01E-04]),
    (1.3, [6.46E-06, 1.70E-05, 2.75E-05, 3.80E-05, 4.85E-05, 5.90E-05, 6.95E-05, 8.00E-05, 1.23E-04, 1.48E-04, 1.72E-04, 1.94E-04]),
    (1.4, [6.01E-06, 1.64E-05, 2.68E-05, 3.72E-05, 4.76E-05, 5.80E-05, 6.84E-05, 7.88E-05, 1.21E-04, 1.45E-04, 1.69E-04, 1.90E-04]),
    (1.5, [5.86E-06, 1.61E-05, 2.64E-05, 3.67E-05, 4.70E-05, 5.72E-05, 6.75E-05, 7.78E-05, 1.19E-04, 1.42E-04, 1.66E-04, 1.87E-04]),
    (2.0, [5.39E-06, 1.53E-05, 2.53E-05, 3.52E-05, 4.51E-05, 5.51E-05, 6.50E-05, 7.50E-05, 1.14E-04, 1.36E-04, 1.58E-04, 1.78E-04]),
    (2.5, [5.51E-06, 1.52E-05, 2.50E-05, 3.47E-05, 4.44E-05, 5.41E-05, 6.39E-05, 7.36E-05, 1.12E-04, 1.33E-04, 1.55E-04, 1.74E-04]),
    (3.0, [5.53E-06, 1.51E-05, 2.47E-05, 3.44E-05, 4.40E-05, 5.36E-05, 6.32E-05, 7.28E-05, 1.11E-04, 1.32E-04, 1.53E-04, 1.71E-04]),
    (3.5, [5.72E-06, 1.53E-05, 2.48E-05, 3.44E-05, 4.40E-05, 5.35E-05, 6.31E-05, 7.27E-05, 1.10E-04, 1.31E-04, 1.52E-04, 1.71E-04]),
    (4.0, [5.90E-06, 1.53E-05, 2.48E-05, 3.42E-05, 4.36E-05, 5.30E-05, 6.25E-05, 7.19E-05, 1.09E-04, 1.30E-04, 1.51E-04, 1.70E-04]),
    (4.5, [5.20E-06, 1.47E-05, 2.43E-05, 3.38E-05, 4.34E-05, 5.29E-05, 6.25E-05, 7.20E-05, 1.10E-04, 1.30E-04, 1.51E-04, 1.70E-04]),
    (5.0, [6.79E-06, 1.60E-05, 2.52E-05, 3.44E-05, 4.36E-05, 5.28E-05, 6.20E-05, 7.13E-05, 1.09E-04, 1.30E-04, 1.51E-04, 1.69E-04]),
    (5.5, [7.11E-06, 1.62E-05, 2.54E-05, 3.45E-05, 4.36E-05, 5.28E-05, 6.19E-05, 7.10E-05, 1.09E-04, 1.30E-04, 1.51E-04, 1.69E-04]),
];

/// Frey et al.'s measured dn/dT (Table 4) from their formula (Table 5), 30 to 250 K, every
/// wavelength: the largest difference, in units of 1e-4 per K, within the formula's residual in
/// n (1e-4) over Table 4's finest step in temperature (10 K). At 295 K, the end of its range,
/// the formula falls 0.06e-4 to 0.24e-4 per K below the measurements, which is why the default
/// silicon takes Li's above 250 K ([`li_dndt_join`]).
pub(crate) fn frey_dndt() -> Outcome {
    let errors = FREY_TABLE_2_T
        .iter()
        .enumerate()
        .filter(|(_, t)| **t <= thermo::JOIN_T)
        .flat_map(|(j, &t)| {
            FREY_TABLE_4
                .iter()
                .map(move |(lam, row)| (dndt("si", "frey-2006", *lam, t) - row[j]).abs() * 1e4)
        });
    deviation(largest(errors), 0.1)
}

/// The default silicon's dn/dT (Li's Eq. (22) from 250 K up) against Frey et al.'s measured
/// Table 4 at 250 and 295 K, 1.2 to 5.5 µm: the largest difference, in units of 1e-4 per K,
/// within Li's stated ±0.15e-4.
pub(crate) fn li_dndt_join() -> Outcome {
    let errors = FREY_TABLE_2_T
        .iter()
        .enumerate()
        .filter(|(_, t)| **t >= thermo::JOIN_T)
        .flat_map(|(j, &t)| {
            FREY_TABLE_4
                .iter()
                .filter(|(lam, _)| *lam >= 1.2)
                .map(move |(lam, row)| (dndt("si", "li-1980", *lam, t) - row[j]).abs() * 1e4)
        });
    deviation(largest(errors), 0.15)
}

/// Leviton & Frey's Table 6: measured dn/dT of Corning 7980 (per K) at their Table 4's
/// wavelengths (µm) and temperatures ([`LEVITON_TABLE_4_T`]), printed to three figures.
#[rustfmt::skip]
pub(crate) const LEVITON_TABLE_6: [(f64, [f64; 13]); 16] = [
    (0.4, [1.25E-06, 1.60E-06, 1.95E-06, 2.30E-06, 3.00E-06, 3.70E-06, 4.67E-06, 6.02E-06, 7.11E-06, 8.20E-06, 9.16E-06, 9.70E-06, 9.84E-06]),
    (0.5, [1.28E-06, 1.61E-06, 1.95E-06, 2.28E-06, 2.94E-06, 3.60E-06, 4.47E-06, 5.69E-06, 6.70E-06, 7.72E-06, 8.61E-06, 9.12E-06, 9.25E-06]),
    (0.6, [1.22E-06, 1.55E-06, 1.88E-06, 2.20E-06, 2.86E-06, 3.51E-06, 4.35E-06, 5.52E-06, 6.51E-06, 7.50E-06, 8.36E-06, 8.86E-06, 8.98E-06]),
    (0.7, [1.26E-06, 1.58E-06, 1.89E-06, 2.20E-06, 2.83E-06, 3.46E-06, 4.27E-06, 5.43E-06, 6.40E-06, 7.36E-06, 8.21E-06, 8.69E-06, 8.81E-06]),
    (0.8, [1.16E-06, 1.48E-06, 1.80E-06, 2.13E-06, 2.77E-06, 3.42E-06, 4.23E-06, 5.36E-06, 6.32E-06, 7.28E-06, 8.12E-06, 8.60E-06, 8.72E-06]),
    (0.9, [1.17E-06, 1.49E-06, 1.81E-06, 2.13E-06, 2.76E-06, 3.40E-06, 4.21E-06, 5.33E-06, 6.27E-06, 7.22E-06, 8.05E-06, 8.52E-06, 8.64E-06]),
    (1.0, [1.07E-06, 1.40E-06, 1.74E-06, 2.07E-06, 2.73E-06, 3.39E-06, 4.21E-06, 5.30E-06, 6.24E-06, 7.18E-06, 8.01E-06, 8.48E-06, 8.60E-06]),
    (1.2, [8.86E-07, 1.26E-06, 1.64E-06, 2.01E-06, 2.76E-06, 3.52E-06, 4.29E-06, 5.25E-06, 6.19E-06, 7.12E-06, 7.94E-06, 8.40E-06, 8.52E-06]),
    (1.5, [9.92E-07, 1.34E-06, 1.69E-06, 2.04E-06, 2.73E-06, 3.43E-06, 4.19E-06, 5.22E-06, 6.18E-06, 7.13E-06, 7.97E-06, 8.45E-06, 8.57E-06]),
    (1.6, [9.31E-07, 1.30E-06, 1.66E-06, 2.02E-06, 2.75E-06, 3.48E-06, 4.27E-06, 5.23E-06, 6.14E-06, 7.05E-06, 7.84E-06, 8.29E-06, 8.41E-06]),
    (1.8, [9.16E-07, 1.27E-06, 1.62E-06, 1.98E-06, 2.68E-06, 3.39E-06, 4.19E-06, 5.22E-06, 6.16E-06, 7.09E-06, 7.91E-06, 8.38E-06, 8.50E-06]),
    (2.0, [8.22E-07, 1.17E-06, 1.51E-06, 1.85E-06, 2.54E-06, 3.22E-06, 4.03E-06, 5.14E-06, 6.13E-06, 7.12E-06, 7.98E-06, 8.48E-06, 8.60E-06]),
    (2.2, [1.01E-06, 1.37E-06, 1.73E-06, 2.09E-06, 2.82E-06, 3.54E-06, 4.35E-06, 5.28E-06, 6.12E-06, 6.96E-06, 7.69E-06, 8.11E-06, 8.22E-06]),
    (2.4, [1.14E-06, 1.45E-06, 1.77E-06, 2.08E-06, 2.71E-06, 3.34E-06, 4.12E-06, 5.21E-06, 6.15E-06, 7.08E-06, 7.91E-06, 8.38E-06, 8.49E-06]),
    (2.5, [1.06E-06, 1.40E-06, 1.74E-06, 2.08E-06, 2.77E-06, 3.46E-06, 4.08E-06, 5.10E-06, 6.17E-06, 7.24E-06, 8.18E-06, 8.71E-06, 8.85E-06]),
    (2.6, [1.03E-06, 1.38E-06, 1.72E-06, 2.07E-06, 2.76E-06, 3.45E-06, 4.22E-06, 5.25E-06, 6.20E-06, 7.16E-06, 8.00E-06, 8.48E-06, 8.60E-06]),
];

/// Leviton & Frey's measured dn/dT (Table 6) from the default silica's (their formula's,
/// Table 3), 100 to 300 K, every wavelength: the largest difference, in units of 1e-6 per K,
/// within 1e-6 per K. That is what a guide needs of its cladding: silica holds a strip's field
/// at a tenth of silicon's weight or less, so 1e-6 per K in silica moves its dn_eff/dT by
/// 1e-7 per K, 0.05% of silicon's 1.8e-4.
pub(crate) fn leviton_dndt() -> Outcome {
    let errors = LEVITON_TABLE_4_T
        .iter()
        .enumerate()
        .filter(|(_, t)| **t >= 100.0)
        .flat_map(|(j, &t)| {
            LEVITON_TABLE_6.iter().map(move |(lam, row)| {
                (dndt("sio2", "malitson-1965", *lam, t) - row[j]).abs() * 1e6
            })
        });
    deviation(largest(errors), 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::catalogue::models;

    #[test]
    fn every_case_passes() {
        for (name, o) in [
            ("li_table_1", li_table_1()),
            ("li_table_2", li_table_2()),
            ("li_dndt_order", li_dndt_order()),
            ("frey_dndt", frey_dndt()),
            ("li_dndt_join", li_dndt_join()),
            ("leviton_dndt", leviton_dndt()),
        ] {
            println!("{name}: {o:?}");
            assert!(o.passed(), "{name}: {o:?}");
        }
    }

    #[test]
    fn the_defaults_are_the_room_temperature_models_bit_for_bit() {
        let at = |m: Material| m.refractive_index(Wavelength::um(1.55).unwrap()).unwrap();
        assert_eq!(
            at(thermo::li_silicon(thermo::LI_T0).unwrap()),
            at(crate::material::silicon())
        );
        assert_eq!(
            at(thermo::malitson_silica(thermo::MALITSON_T0).unwrap()),
            at(crate::material::silica())
        );
        // and a degree away they move by dn/dT
        let si = |t| n(&thermo::li_silicon(t).unwrap(), 1.55);
        let d = (si(294.0) - si(292.0)) / 2.0;
        // within 1e-6: Li's two expansion branches meet at 293 K 4e-6 apart in ΔL/L
        assert!((d - dndt("si", "li-1980", 1.55, 293.0)).abs() < 1e-6, "{d}");
        let ox = |t| n(&thermo::malitson_silica(t).unwrap(), 1.55);
        let d = (ox(294.15) - ox(292.15)) / 2.0;
        assert!(
            (d - dndt("sio2", "malitson-1965", 1.55, 293.15)).abs() < 1e-8,
            "{d}"
        );
    }

    #[test]
    fn the_default_silicon_is_continuous_where_it_joins_frey() {
        let si = |t| n(&thermo::li_silicon(t).unwrap(), 1.55);
        let j = thermo::JOIN_T;
        assert!((si(j) - si(j - 1e-6)).abs() < 1e-9);
        // below the join it is Frey et al.'s change of index, from 20 K
        let frey = |t| n(&models::frey_silicon(t).unwrap(), 1.55);
        let below = si(100.0) - si(j);
        assert!((below - (frey(100.0) - frey(j))).abs() < 1e-9, "{below}");
        // and the cold silicon ends at 5 µm, inside Frey et al.'s range
        assert!(thermo::li_silicon(100.0).unwrap().range().1.to_um() == 5.0);
    }

    #[test]
    fn silica_continues_above_300_k_at_its_dn_dt_there() {
        let ox = |t| n(&thermo::malitson_silica(t).unwrap(), 1.55);
        let slope = dndt("sio2", "malitson-1965", 1.55, 300.0);
        assert!((ox(350.0) - ox(300.0) - 50.0 * slope).abs() < 1e-9);
        assert!(dndt("sio2", "malitson-1965", 1.55, 350.0) == slope);
        assert!(thermo::malitson_silica(401.0).is_err());
        assert!(thermo::li_silicon(751.0).is_err());
        assert!(thermo::li_silicon(19.0).is_err());
    }
}

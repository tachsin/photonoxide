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

// ---------------------------------------------------------------------------------------------
// Silicon below 40 K: Komma et al. 2012
// ---------------------------------------------------------------------------------------------

/// Frey et al.'s measured dn/dT at 1.5 µm as Komma et al.'s Fig. 4 plots it beside their own
/// points (its crosses), read off the figure as the points are ([`thermo::KOMMA_FIG_4`]): Frey
/// et al.'s temperatures (K) and the values read, per K, from the main plot and (30, 40 and
/// 50 K) the inset.
#[rustfmt::skip]
pub(crate) const KOMMA_FIG_4_FREY: [(f64, f64); 15] = [
    (30.0, 5.879e-6), (40.0, 1.616e-5), (50.0, 2.647e-5), (60.0, 3.658e-5), (70.0, 4.700e-5),
    (80.0, 5.708e-5), (90.0, 6.764e-5), (100.0, 7.761e-5), (150.0, 1.192e-4), (200.0, 1.424e-4),
    (250.0, 1.660e-4), (295.0, 1.874e-4),
    (30.0, 5.855e-6), (40.0, 1.609e-5), (50.0, 2.641e-5),
];

/// Komma et al.'s error bar at 35 K, the last the figure draws: its half-width, per K (the bar
/// runs from 7.74e-6 to 1.289e-5 about the point's 1.05e-5).
pub(crate) const KOMMA_BAR_35: f64 = 2.57e-6;

/// Our reading of Komma et al.'s Fig. 4, checked on what it plots from a table: its crosses,
/// Frey et al.'s Table 4 at 1.5 µm, read with the axes the points are read with, against the
/// table as printed. The largest relative deviation, within 1 %.
pub(crate) fn komma_frey_reading() -> Outcome {
    let row = FREY_TABLE_4
        .iter()
        .find(|(lam, _)| *lam == 1.5)
        .map(|(_, row)| row);
    let worst = KOMMA_FIG_4_FREY
        .iter()
        .map(|&(t, read)| {
            let printed = row.and_then(|row| {
                FREY_TABLE_2_T
                    .iter()
                    .position(|&tt| tt == t)
                    .map(|j| row[j])
            });
            printed.map_or(f64::NAN, |p| (read / p - 1.0).abs())
        })
        .fold(
            0.0,
            |a: f64, e| if e.is_nan() { f64::NAN } else { a.max(e) },
        );
    deviation(worst, 0.01)
}

/// Where the default silicon passes from Komma et al.'s dn/dT to Frey et al.'s formula, 40 K at
/// 1.55 µm: the formula's against Komma et al.'s point, in units of 1e-6 per K, within the
/// half-width of Komma et al.'s last error bar.
pub(crate) fn komma_join() -> Outcome {
    let t = thermo::COLD_JOIN_T;
    let komma = thermo::KOMMA_FIG_4
        .iter()
        .find(|(tt, _)| *tt == t)
        .map_or(f64::NAN, |(_, d)| *d);
    outcome(
        dndt("si", "frey-2006", 1.55, t) * 1e6,
        komma * 1e6,
        KOMMA_BAR_35 * 1e6,
    )
}

/// The default silicon's dn/dT at 1.55 µm and 300 K (Li's Eq. (22)) against the 1.8e-4 per K
/// Komma et al. measured there (their abstract), in units of 1e-4 per K, within Li's ±0.15e-4.
pub(crate) fn komma_room() -> Outcome {
    outcome(dndt("si", "li-1980", 1.55, 300.0) * 1e4, 1.8, 0.15)
}

/// The default silicon's dn/dT at 1.55 µm at every kelvin from 5 to 750 K: how many are not
/// positive (none; Frey et al.'s formula alone turns negative below 27 K).
pub(crate) fn silicon_dndt_positive() -> Outcome {
    let (lo, hi) = thermo::SILICON_T;
    let count = (lo as usize..=hi as usize)
        .map(|t| dndt("si", "li-1980", 1.55, t as f64))
        .filter(|d| d.is_nan() || *d <= 0.0)
        .count();
    outcome(count as f64, 0.0, 0.0)
}

// ---------------------------------------------------------------------------------------------
// Silicon nitride: Arbabi & Goddard 2013, Elshaari et al. 2016
// ---------------------------------------------------------------------------------------------

/// Elshaari et al.'s Eq. (3), dn/dT of their PECVD SiN as printed: the coefficients of T⁰ … T⁴.
/// Not taken (it lies above the figure's points); a test keeps the comparison.
#[cfg(test)]
pub(crate) const ELSHAARI_EQ_3: [f64; 5] = [3.211e-7, -1.990e-8, 8.856e-10, -1.375e-12, -1.105e-15];
/// Elshaari et al.'s Eq. (4), dn/dT of their PECVD oxide as printed.
pub(crate) const ELSHAARI_EQ_4: [f64; 5] = [-1.167e-7, 1.727e-8, 1.861e-10, -5.781e-13, 4.221e-16];

/// Elshaari et al.'s Fig. 4(b), the oxide's dn/dT from 40 K, read off the figure as Fig. 4(a)'s
/// points are ([`thermo::ELSHAARI_FIG_4A`]): T (K) and dn/dT (per K) to four figures.
#[rustfmt::skip]
pub(crate) const ELSHAARI_FIG_4B: [(f64, f64); 25] = [
    (40.65, 8.607e-7), (45.32, 1.000e-6), (50.08, 1.145e-6), (55.15, 1.310e-6), (59.90, 1.461e-6),
    (68.17, 1.749e-6), (76.43, 2.047e-6), (84.61, 2.343e-6), (92.64, 2.651e-6), (100.36, 2.945e-6),
    (110.81, 3.358e-6), (120.00, 3.730e-6), (130.06, 4.127e-6), (140.12, 4.533e-6), (149.94, 4.923e-6),
    (164.91, 5.509e-6), (178.70, 6.050e-6), (192.35, 6.546e-6), (209.26, 7.163e-6), (223.61, 7.634e-6),
    (239.12, 8.137e-6), (252.14, 8.511e-6), (269.68, 8.969e-6), (283.79, 9.313e-6), (298.99, 9.596e-6),
];

fn quartic(c: &[f64; 5], t: f64) -> f64 {
    c.iter().rev().fold(0.0, |acc, ci| acc * t + ci)
}

/// Our reading of Elshaari et al.'s Fig. 4, checked where the paper gives the curve in closed
/// form: Fig. 4(b)'s points, read with that figure's axes as Fig. 4(a)'s are, against their
/// Eq. (4) from 40 to 300 K. The largest relative deviation, within 1 %.
pub(crate) fn elshaari_reading() -> Outcome {
    let worst = ELSHAARI_FIG_4B
        .iter()
        .map(|&(t, read)| (read / quartic(&ELSHAARI_EQ_4, t) - 1.0).abs())
        .fold(0.0, f64::max);
    deviation(worst, 0.01)
}

/// Elshaari et al.'s points of Fig. 4(a) between their neighbours: each inner point against the
/// power law in T of the two about it, the largest relative deviation, within the ±0.08e-5 of
/// their 2.51e-5 at room temperature (3.2 %).
pub(crate) fn elshaari_between() -> Outcome {
    let p = &thermo::ELSHAARI_FIG_4A;
    let worst = p
        .windows(3)
        .map(|w| (thermo::between_rows(&[w[0], w[2]], w[1].0) / w[1].1 - 1.0).abs())
        .fold(0.0, f64::max);
    deviation(worst, 0.08 / 2.51)
}

/// Where the two nitrides meet: the default's dn/dT at 295 K (Arbabi & Goddard's LPCVD Si₃N₄),
/// in units of 1e-5 per K, against the 2.51 ± 0.08 Elshaari et al. measured on their PECVD SiN
/// at room temperature, within the two uncertainties summed (±0.09 and ±0.08).
pub(crate) fn nitride_room() -> Outcome {
    outcome(dndt("si3n4", "luke-2015", 1.55, 295.0) * 1e5, 2.51, 0.17)
}

/// Arbabi & Goddard's Table 2: per ring, the simulated sensitivities ∂λ/∂n (nm per RIU) of the
/// TE and TM resonances to the core and the cladding, and the measured shifts dλ/dT (pm per K)
/// with their uncertainties, as printed: (TE core, TE clad, TM core, TM clad, dλ_TE/dT ±,
/// dλ_TM/dT ±).
pub(crate) const ARBABI_TABLE_2: [[f64; 8]; 2] = [
    [555.0, 316.0, 361.0, 578.0, 16.81, 0.39, 14.55, 0.32],
    [591.0, 265.0, 386.0, 545.0, 16.47, 0.5, 13.99, 0.68],
];
/// Table 2's computed dn/dT of Si₃N₄ and SiO₂ for rings one and two, with their uncertainties,
/// and Eqs. (3a) and (3b), the two combined: (Si₃N₄, ±, SiO₂, ±), in units of 1e-5 per K.
pub(crate) const ARBABI_COMPUTED: [[f64; 4]; 3] = [
    [2.48, 0.12, 0.97, 0.11],
    [2.39, 0.15, 0.87, 0.20],
    [2.45, 0.09, 0.95, 0.10],
];

/// Arbabi & Goddard's Eqs. (2) solved with Table 2's sensitivities and measured shifts, ring by
/// ring, and the two rings combined by inverse-variance weights (Table 2's uncertainties): the
/// six dn/dT of Table 2 and Eqs. (3), and the two combined uncertainties, against the printed
/// values, in units of 1e-5 per K. The largest deviation, within a unit of the printed second
/// decimal.
pub(crate) fn arbabi_table_2() -> Outcome {
    let mut errors = vec![];
    for (ring, computed) in ARBABI_TABLE_2.iter().zip(ARBABI_COMPUTED) {
        let [a, b, c, d, te, _, tm, _] = *ring;
        let det = a * d - b * c;
        // dλ/dT in pm per K over ∂λ/∂n in nm per RIU: 1e-3 per K, so 100 in units of 1e-5
        let core = (te * d - b * tm) / det * 100.0;
        let clad = (a * tm - c * te) / det * 100.0;
        errors.push((core - computed[0]).abs());
        errors.push((clad - computed[2]).abs());
    }
    let combine = |k: usize| {
        let (sum, weight) = ARBABI_COMPUTED[..2].iter().fold((0.0, 0.0), |(s, w), r| {
            let v = 1.0 / (r[k + 1] * r[k + 1]);
            (s + v * r[k], w + v)
        });
        (sum / weight, 1.0 / weight.sqrt())
    };
    for k in [0, 2] {
        let (value, sigma) = combine(k);
        errors.push((value - ARBABI_COMPUTED[2][k]).abs());
        errors.push((sigma - ARBABI_COMPUTED[2][k + 1]).abs());
    }
    deviation(largest(errors.into_iter()), 0.01)
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
            ("komma_frey_reading", komma_frey_reading()),
            ("komma_join", komma_join()),
            ("komma_room", komma_room()),
            ("silicon_dndt_positive", silicon_dndt_positive()),
            ("elshaari_reading", elshaari_reading()),
            ("elshaari_between", elshaari_between()),
            ("nitride_room", nitride_room()),
            ("arbabi_table_2", arbabi_table_2()),
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
        assert!(thermo::li_silicon(4.9).is_err());
    }

    #[test]
    fn the_default_silicon_below_40_k_moves_by_komma_et_al_s_dn_dt() {
        let si = |t| n(&thermo::li_silicon(t).unwrap(), 1.55);
        let j = thermo::COLD_JOIN_T;
        assert!((si(j) - si(j - 1e-6)).abs() < 1e-9);
        // its change of index is Komma et al.'s dn/dT integrated, at every wavelength
        let rise = thermo::between_rows_integral(&thermo::KOMMA_FIG_4, 5.0, j);
        assert!((si(j) - si(5.0) - rise).abs() < 1e-12, "{rise}");
        assert!(rise > 1.4e-4 && rise < 1.6e-4, "{rise}");
        let at3 = |t| n(&thermo::li_silicon(t).unwrap(), 3.0);
        assert!((at3(j) - at3(5.0) - rise).abs() < 1e-12);
        // dn/dT a degree apart from the change of index, and at the points themselves
        let d = (si(20.5) - si(19.5)) / 1.0;
        assert!(
            (d / dndt("si", "li-1980", 1.55, 20.0) - 1.0).abs() < 0.02,
            "{d}"
        );
        for (t, k) in thermo::KOMMA_FIG_4.iter().filter(|(t, _)| *t < j) {
            assert!(
                (dndt("si", "li-1980", 1.55, *t) / k - 1.0).abs() < 1e-12,
                "{t}"
            );
        }
        // the integral of the power law against the midpoint rule on 10⁴ points
        let (a, b) = (7.0, 33.0);
        let m = 10_000;
        let h = (b - a) / m as f64;
        let quadrature: f64 = (0..m)
            .map(|i| thermo::between_rows(&thermo::KOMMA_FIG_4, a + (i as f64 + 0.5) * h) * h)
            .sum();
        let exact = thermo::between_rows_integral(&thermo::KOMMA_FIG_4, a, b);
        assert!(
            (quadrature / exact - 1.0).abs() < 1e-6,
            "{quadrature} {exact}"
        );
        // Frey et al.'s formula alone turns negative below 27 K
        assert!(dndt("si", "frey-2006", 1.55, 25.0) < 0.0);
        assert!(dndt("si", "frey-2006", 1.55, 28.0) > 0.0);
    }

    #[test]
    fn the_nitride_moves_by_its_dn_dt_and_is_luke_et_al_s_bit_for_bit_at_295_k() {
        let at = |m: Material| m.refractive_index(Wavelength::um(1.55).unwrap()).unwrap();
        let t0 = thermo::NITRIDE_T0;
        assert_eq!(
            at(thermo::luke_nitride(t0).unwrap()),
            at(crate::material::silicon_nitride())
        );
        let luke = crate::material::silicon_nitride();
        // the spline through the table against Luke et al.'s formula plus the shift, between
        // its points: finely in the UV, where the formula bends most, coarsely beyond
        let probes: Vec<f64> = (0..414)
            .map(|i| 0.3102 + 0.0007 * i as f64)
            .chain((0..377).map(|i| 0.6 + 0.013 * i as f64))
            .collect();
        for t in [19.4, 100.0, 270.0, 350.0, 400.0] {
            let m = thermo::luke_nitride(t).unwrap();
            let shift = thermo::nitride_shift(t);
            let worst = probes
                .iter()
                .map(|&lam| (n(&m, lam) - n(&luke, lam) - shift).abs())
                .fold(0.0, f64::max);
            assert!(worst < 1e-6, "{t}: {worst}");
        }
        // dn/dT continuous where Elshaari et al.'s meets Arbabi & Goddard's, within 0.5 %
        let j = thermo::NITRIDE_JOIN_T;
        let below = thermo::nitride_dndt(j - 1e-9);
        assert!((below / thermo::ARBABI_DNDT - 1.0).abs() < 0.006, "{below}");
        // the shift continuous there, and its slope the dn/dT
        assert!((thermo::nitride_shift(j) - thermo::nitride_shift(j - 1e-9)).abs() < 1e-12);
        let d = |t: f64| thermo::nitride_shift(t + 0.5) - thermo::nitride_shift(t - 0.5);
        for t in [30.0, 150.0, 260.0, 330.0] {
            assert!(
                (d(t) / dndt("si3n4", "luke-2015", 1.55, t) - 1.0).abs() < 2e-3,
                "{t}"
            );
        }
        // 3.5e-3 below room temperature's at Elshaari et al.'s coldest
        let cold = thermo::nitride_shift(thermo::NITRIDE_T.0);
        assert!((cold + 3.5e-3).abs() < 5e-5, "{cold}");
        assert!(thermo::luke_nitride(19.3).is_err());
        assert!(thermo::luke_nitride(400.5).is_err());
    }

    #[test]
    fn elshaari_et_al_s_eq_3_as_printed_lies_above_their_points() {
        // 2 to 9 % above Fig. 4(a)'s points from 60 to 300 K; 2.80e-5 at 300 K against the
        // text's 2.51e-5
        let p = &thermo::ELSHAARI_FIG_4A;
        for &(t, k) in p.iter().filter(|(t, _)| *t >= 60.0) {
            let r = quartic(&ELSHAARI_EQ_3, t) / k;
            assert!(r > 1.015 && r < 1.095, "{t}: {r}");
        }
        assert!((quartic(&ELSHAARI_EQ_3, 300.0) - 2.80e-5).abs() < 0.005e-5);
        // and Eq. (4) is the text's 0.96e-5 for the oxide at 300 K
        assert!((quartic(&ELSHAARI_EQ_4, 300.0) - 0.96e-5).abs() < 0.005e-5);
    }
}

//! Temperature in the default models of silicon, silica and silicon nitride: their indices at a
//! temperature and their thermo-optic coefficients dn/dT.
//!
//! - Silicon: Li's table (293 K) moved to a temperature by his own Eq. (22), n(λ, T) for 1.2 to
//!   14 µm and 100 to 750 K (H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980),
//!   doi:10.1063/1.555624), from which he computed his Tables 1 and 2; below 250 K on by Frey,
//!   Leviton & Madison's formula (Proc. SPIE 6273, 62732J (2006), doi:10.1117/12.672850), 20 to
//!   300 K, which their measurements there follow more closely ([`JOIN_T`]); below 40 K by
//!   Komma et al.'s measured dn/dT at 1550 nm (Appl. Phys. Lett. 101, 041905 (2012),
//!   doi:10.1063/1.4738989), 5 to 300 K, where Frey et al.'s formula no longer follows their own
//!   measurements ([`COLD_JOIN_T`]).
//! - Silica: Malitson's formula (20 °C) moved by Leviton & Frey's change of index with
//!   temperature (Proc. SPIE 6273, 62732K (2006), doi:10.1117/12.672853), 30 to 300 K, and
//!   from 300 to 400 K by their dn/dT at 300 K, held: an extrapolation.
//! - Silicon nitride: Luke et al.'s formula (taken at 295 K) moved by Arbabi & Goddard's dn/dT
//!   of LPCVD Si₃N₄ at 1550 nm (Opt. Lett. 38, 3878 (2013), doi:10.1364/OL.38.003878), measured
//!   near room temperature and held from 270 to 400 K, and below 270 K by Elshaari et al.'s
//!   measured dn/dT of PECVD SiN from 18 to 300 K (IEEE Photonics J. 8, 2701009 (2016),
//!   doi:10.1109/JPHOT.2016.2561622), which meets Arbabi & Goddard's there ([`NITRIDE_JOIN_T`]).

use super::models::{self, check_range};
use crate::Result;
use crate::material::{self, Material, Model, Provenance, Table};
use crate::units::Wavelength;

// ---------------------------------------------------------------------------------------------
// Silicon: Li 1980, Eq. (22)
// ---------------------------------------------------------------------------------------------

/// Eq. (22)'s range: 100 to 750 K (and 1.2 to 14 µm).
pub(crate) const LI_T: (f64, f64) = (100.0, 750.0);
/// The temperature of Li's Table 1 as the default model uses it, and of his ΔL/L₂₉₃.
pub(crate) const LI_T0: f64 = 293.0;

/// ε(T) of Eq. (22): 11.4445 + 2.7739e-4 T + 1.7050e-6 T² − 8.1347e-10 T³.
pub(crate) const LI_EPS: [f64; 4] = [11.4445, 2.7739e-4, 1.7050e-6, -8.1347e-10];
/// A₀, A₁, A₂ of Eq. (22).
pub(crate) const LI_A: [f64; 3] = [0.8948, 4.3977e-4, 7.3835e-8];
/// ΔL(T)/L₂₉₃ from 20 to 293 K (p. 568, from Touloukian et al.), as a fraction. The paper prints
/// the constant as −0.021: the percent of the thermal-expansion tables it comes from, beside
/// coefficients of T already divided by 100. As printed ΔL/L₂₉₃ would be −2.1e-2 at 293 K, where
/// it is zero by definition, and Eq. (22) would miss Table 1 by 0.015; read as −2.1e-4 it is
/// 1.5e-6 at 293 K and Eq. (22) gives Tables 1 and 2 (the validation cases).
pub(crate) const LI_EXPANSION_LOW: [f64; 4] = [-2.1e-4, -4.149e-7, -4.620e-10, 1.482e-11];
/// ΔL(T)/L₂₉₃ from 293 to 1600 K, the constant printed as −0.071 read as −7.1e-4 likewise.
pub(crate) const LI_EXPANSION_HIGH: [f64; 4] = [-7.1e-4, 1.887e-6, 1.934e-9, -4.544e-13];

/// The printed coefficients of Eq. (22), for display: (symbol, its terms in T⁰ … T³).
pub(crate) const LI_PRINTED: [(&str, [&str; 4]); 4] = [
    (
        r"$\varepsilon(T)$",
        ["11.4445", "2.7739E-04", "1.7050E-06", "-8.1347E-10"],
    ),
    (
        r"$A_0 + A_1 T + A_2 T^2$",
        ["0.8948", "4.3977E-04", "7.3835E-08", ""],
    ),
    (
        r"$\Delta L/L_{293}$, 20–293 K",
        ["-0.021", "-4.149E-07", "-4.620E-10", "1.482E-11"],
    ),
    (
        r"$\Delta L/L_{293}$, 293–1600 K",
        ["-0.071", "1.887E-06", "1.934E-09", "-4.544E-13"],
    ),
];

fn cubic(c: &[f64; 4], t: f64) -> (f64, f64) {
    (
        c[0] + t * (c[1] + t * (c[2] + t * c[3])),
        c[1] + t * (2.0 * c[2] + 3.0 * t * c[3]),
    )
}

/// n and dn/dT (per K) of silicon at λ (µm) and T (K) by Li's Eq. (22):
/// n² = ε(T) + L(T)/λ² (A₀ + A₁T + A₂T²), L(T) = exp(−3 ΔL(T)/L₂₉₃). No range check.
pub(crate) fn li_eq22(lam: f64, t: f64) -> (f64, f64) {
    let (eps, deps) = cubic(&LI_EPS, t);
    let expansion = if t < LI_T0 {
        &LI_EXPANSION_LOW
    } else {
        &LI_EXPANSION_HIGH
    };
    let (dl, ddl) = cubic(expansion, t);
    let l = (-3.0 * dl).exp();
    let a = LI_A[0] + t * (LI_A[1] + t * LI_A[2]);
    let da = LI_A[1] + 2.0 * t * LI_A[2];
    let l2 = lam * lam;
    let n = (eps + l / l2 * a).sqrt();
    let dn2 = deps + (-3.0 * ddl * l * a + l * da) / l2;
    (n, dn2 / (2.0 * n))
}

/// Where the default silicon passes from Frey et al.'s formula (below) to Li's Eq. (22) (above).
/// Against Frey et al.'s measured dn/dT (their Table 4), their formula is within 0.08e-4 per K
/// from 30 to 250 K but 0.06e-4 to 0.24e-4 below at 295 K, the end of its range, and Li's is
/// within 0.10e-4 at 250 and 295 K (inside his ±0.15e-4) but exceeds them below 200 K (0.36e-4
/// at 100 K and 1.55 µm).
pub(crate) const JOIN_T: f64 = 250.0;
/// The default silicon's temperatures: Komma et al.'s lowest to Li's highest.
pub(crate) const SILICON_T: (f64, f64) = (KOMMA_FIG_4[0].0, LI_T.1);

/// Komma et al.'s measured dn/dT of silicon at 1550 nm, per K, from 5 to 40 K: the points of
/// their Fig. 4's inset, read off the figure's drawing (each point's centre against the axes'
/// ticks, which the paper's PDF holds as vector paths), to three figures. The paper prints no
/// table; its text gives 1e-8 at 5 K (±2e-8) and 1.8e-4 at 300 K.
pub(crate) const KOMMA_FIG_4: [(f64, f64); 8] = [
    (5.0, 9.99e-9),
    (10.0, 8.32e-8),
    (15.0, 4.65e-7),
    (20.0, 1.53e-6),
    (25.0, 2.97e-6),
    (30.0, 7.03e-6),
    (35.0, 1.05e-5),
    (40.0, 1.56e-5),
];
/// Where the default silicon passes from Komma et al.'s dn/dT (below) to Frey et al.'s formula
/// (above). There Frey et al.'s formula gives 1.556e-5 per K at 1.55 µm, Komma et al.'s
/// 1.56e-5; below, the formula's dn/dT falls under their own measurement (3.5e-6 against
/// 5.86e-6 per K at 30 K and 1.5 µm, their Table 4) and turns negative below 27 K, where
/// Komma et al.'s falls smoothly to 1e-8 at 5 K.
pub(crate) const COLD_JOIN_T: f64 = 40.0;

/// A table's value at `t` as a power law of T between the rows about it (linear in log y
/// against log T); outside the rows, the nearest end's power law.
pub(crate) fn between_rows(rows: &[(f64, f64)], t: f64) -> f64 {
    let k = rows
        .windows(2)
        .position(|w| t <= w[1].0)
        .unwrap_or(rows.len() - 2);
    let ((t0, y0), (t1, y1)) = (rows[k], rows[k + 1]);
    y0 * (t / t0).powf((y1 / y0).ln() / (t1 / t0).ln())
}

/// The integral over T from `a` to `b` (a ≤ b) of [`between_rows`], segment by segment in
/// closed form: ∫ y₀ (T/T₀)^m dT = y₀ T₀ ((b/T₀)^{m+1} − (a/T₀)^{m+1}) / (m + 1).
pub(crate) fn between_rows_integral(rows: &[(f64, f64)], a: f64, b: f64) -> f64 {
    let last = rows.len() - 2;
    rows.windows(2)
        .enumerate()
        .map(|(i, w)| {
            let ((t0, y0), (t1, y1)) = (w[0], w[1]);
            // the first and last segments reach past the rows, as between_rows does
            let lo = if i == 0 { a } else { a.max(t0) };
            let hi = if i == last { b } else { b.min(t1) };
            if lo >= hi {
                return 0.0;
            }
            let m1 = (y1 / y0).ln() / (t1 / t0).ln() + 1.0;
            if m1.abs() < 1e-12 {
                y0 * t0 * (hi / lo).ln()
            } else {
                y0 * t0 * ((hi / t0).powf(m1) - (lo / t0).powf(m1)) / m1
            }
        })
        .sum()
}

/// Komma et al.'s dn/dT at T (K), per K.
fn komma_dndt(t: f64) -> f64 {
    between_rows(&KOMMA_FIG_4, t)
}
/// The longest wavelength, µm, of the default silicon below [`JOIN_T`]: the last of Li's points
/// inside Frey et al.'s range (1.1 to 5.6 µm).
pub(crate) const COLD_LONGEST: f64 = 5.0;

/// Frey et al.'s index at λ (µm) and T (K).
fn frey(lam: f64, t: f64) -> Result<f64> {
    Ok(models::frey_silicon(t)?
        .refractive_index(Wavelength::um(lam)?)?
        .re)
}

/// Li's Table 1 (293 K) moved to `t` at its points up to `longest` (µm) by `shift(λ)`, splined.
fn li_moved(
    t: f64,
    longest: f64,
    shift: impl Fn(f64) -> Result<f64>,
    how: &str,
) -> Result<Material> {
    let base = material::silicon();
    let Model::Tabulated(table) = base.model() else {
        unreachable!("Li's silicon is a table")
    };
    let wavelengths: Vec<f64> = table
        .wavelengths()
        .iter()
        .copied()
        .filter(|&lam| lam <= longest)
        .collect();
    let n = wavelengths
        .iter()
        .map(|&lam| Ok(base.refractive_index(Wavelength::um(lam)?)?.re + shift(lam)?))
        .collect::<Result<Vec<f64>>>()?;
    let p = base.provenance();
    Material::new(
        "Si",
        Model::Tabulated(Table::new(wavelengths, n, None)?),
        base.range().0,
        Wavelength::um(longest)?,
        Provenance {
            temperature: Some(t),
            notes: format!(
                "crystalline silicon; Li's Table 1 (293 K) moved to {t} K {how}; uncertainty \
                 ±2e-3 (Li)"
            ),
            ..p.clone()
        },
    )
}

/// Li's Table 1 (293 K) moved to `t` (K) by Eq. (22) alone, at all 35 points: Li's own n(λ, T),
/// from which he computed his Table 1 at every temperature.
pub(crate) fn li_eq22_silicon(t: f64) -> Result<Material> {
    check_range("temperature", t, LI_T.0, LI_T.1)?;
    li_moved(
        t,
        14.0,
        |lam| Ok(li_eq22(lam, t).0 - li_eq22(lam, LI_T0).0),
        "by his Eq. (22)",
    )
}

/// The default silicon at a temperature (K): Li's Table 1 at 293 K ([`material::silicon`], bit
/// for bit at 293 K), each of its points moved by Li's Eq. (22) from 293 K down to [`JOIN_T`]
/// and up to 750 K, below by Frey et al.'s formula on from there to [`COLD_JOIN_T`] (at Li's
/// points up to 5 µm), and below that by Komma et al.'s dn/dT at 1550 nm, integrated, at every
/// point (to 5 K); splined as the table is.
pub(crate) fn li_silicon(t: f64) -> Result<Material> {
    if t == LI_T0 {
        return Ok(material::silicon());
    }
    check_range("temperature", t, SILICON_T.0, SILICON_T.1)?;
    if t >= JOIN_T {
        return li_eq22_silicon(t);
    }
    if t >= COLD_JOIN_T {
        return li_moved(
            t,
            COLD_LONGEST,
            |lam| {
                Ok(
                    li_eq22(lam, JOIN_T).0 - li_eq22(lam, LI_T0).0 + frey(lam, t)?
                        - frey(lam, JOIN_T)?,
                )
            },
            "by his Eq. (22) to 250 K and below by Frey et al.'s formula (doi:10.1117/12.672850)",
        );
    }
    let cold = between_rows_integral(&KOMMA_FIG_4, t, COLD_JOIN_T);
    li_moved(
        t,
        COLD_LONGEST,
        |lam| {
            Ok(
                li_eq22(lam, JOIN_T).0 - li_eq22(lam, LI_T0).0 + frey(lam, COLD_JOIN_T)?
                    - frey(lam, JOIN_T)?
                    - cold,
            )
        },
        "by his Eq. (22) to 250 K, by Frey et al.'s formula (doi:10.1117/12.672850) to 40 K and \
         below by Komma et al.'s dn/dT at 1550 nm (doi:10.1063/1.4738989)",
    )
}

/// dn/dT of the default silicon, per K, at λ (µm) and T (K): Eq. (22)'s derivative (as Li's
/// Table 2 tabulates it) from [`JOIN_T`] up, Frey et al.'s formula's down to [`COLD_JOIN_T`],
/// Komma et al.'s at 1550 nm below (at every wavelength).
pub(crate) fn li_dndt(lam: f64, t: f64) -> Result<Vec<f64>> {
    check_range("temperature", t, SILICON_T.0, SILICON_T.1)?;
    if t >= JOIN_T {
        check_range("wavelength", lam, 1.2, 14.0)?;
        Ok(vec![li_eq22(lam, t).1])
    } else if t < COLD_JOIN_T {
        check_range("wavelength", lam, 1.2, COLD_LONGEST)?;
        Ok(vec![komma_dndt(t)])
    } else {
        check_range("wavelength", lam, 1.2, COLD_LONGEST)?;
        by_differences(
            |t| models::frey_silicon(t).map(|m| vec![m]),
            models::FREY_SI_T,
            lam,
            t,
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Silica: Malitson 1965, moved by Leviton & Frey 2006
// ---------------------------------------------------------------------------------------------

/// Malitson's temperature, 20 °C.
pub(crate) const MALITSON_T0: f64 = 293.15;
/// The temperatures silica's default model takes: Leviton & Frey's 30 to 300 K, and their dn/dT
/// at 300 K held to 400 K.
pub(crate) const SILICA_T: (f64, f64) = (models::LEVITON_T.0, 400.0);
/// The wavelengths, µm, silica takes away from 20 °C: Leviton & Frey's, at every 20 nm.
const SILICA_STEP: f64 = 0.02;

/// Leviton & Frey's index at λ (µm) and T (K) inside their range.
fn leviton(lam: f64, t: f64) -> Result<f64> {
    Ok(models::leviton_frey(t)?
        .refractive_index(Wavelength::um(lam)?)?
        .re)
}

/// Leviton & Frey's dn/dT at λ (µm) and T (K), per K, by differences.
fn leviton_dndt(lam: f64, t: f64) -> Result<f64> {
    by_differences(
        |t| models::leviton_frey(t).map(|m| vec![m]),
        models::LEVITON_T,
        lam,
        t,
    )
    .map(|v| v[0])
}

/// Leviton & Frey's change of index from Malitson's 20 °C to `t`, at λ (µm): their own above
/// 300 K continued at their dn/dT at 300 K.
fn silica_shift(lam: f64, t: f64) -> Result<f64> {
    let top = models::LEVITON_T.1;
    let at = if t <= top {
        leviton(lam, t)?
    } else {
        leviton(lam, top)? + (t - top) * leviton_dndt(lam, top)?
    };
    Ok(at - leviton(lam, MALITSON_T0)?)
}

/// Fused silica at a temperature (K): Malitson's formula ([`material::silica`], bit for bit at
/// 20 °C), elsewhere moved by Leviton & Frey's change of index from 20 °C, on their wavelengths
/// (0.4 to 2.6 µm) every 20 nm, splined.
pub(crate) fn malitson_silica(t: f64) -> Result<Material> {
    if t == MALITSON_T0 {
        return Ok(material::silica());
    }
    check_range("temperature", t, SILICA_T.0, SILICA_T.1)?;
    let base = material::silica();
    let (lo, hi) = models::LEVITON_LAMBDA;
    let count = ((hi - lo) / SILICA_STEP).round() as usize;
    let wavelengths: Vec<f64> = (0..=count)
        .map(|i| lo + (hi - lo) * i as f64 / count as f64)
        .collect();
    let n = wavelengths
        .iter()
        .map(|&lam| Ok(base.refractive_index(Wavelength::um(lam)?)?.re + silica_shift(lam, t)?))
        .collect::<Result<Vec<f64>>>()?;
    let p = base.provenance();
    let extrapolated = if t > models::LEVITON_T.1 {
        ", above 300 K at their dn/dT at 300 K (an extrapolation)"
    } else {
        ""
    };
    Material::new(
        "SiO2",
        Model::Tabulated(Table::new(wavelengths, n, None)?),
        Wavelength::um(lo)?,
        Wavelength::um(hi)?,
        Provenance {
            reference: format!(
                "{}; D. B. Leviton, B. J. Frey, Proc. SPIE 6273, 62732K (2006)",
                p.reference
            ),
            temperature: Some(t),
            notes: format!(
                "fused silica, Malitson's Eq. (1) at 20 °C moved to {t} K by Leviton & Frey's \
                 Table 3 (doi:10.1117/12.672853){extrapolated}, every 20 nm from 0.4 to 2.6 µm"
            ),
            ..p.clone()
        },
    )
}

/// dn/dT of the default silica, per K, at λ (µm) and T (K): Leviton & Frey's.
pub(crate) fn malitson_dndt(lam: f64, t: f64) -> Result<Vec<f64>> {
    check_range("temperature", t, SILICA_T.0, SILICA_T.1)?;
    check_range(
        "wavelength",
        lam,
        models::LEVITON_LAMBDA.0,
        models::LEVITON_LAMBDA.1,
    )?;
    Ok(vec![leviton_dndt(lam, t.min(models::LEVITON_T.1))?])
}

// ---------------------------------------------------------------------------------------------
// Silicon nitride: Luke 2015, moved by Arbabi & Goddard 2013 and Elshaari et al. 2016
// ---------------------------------------------------------------------------------------------

/// Arbabi & Goddard's dn/dT of stoichiometric LPCVD Si₃N₄ at 1550 nm, per K: their Eq. (3a),
/// (2.45 ± 0.09) × 10⁻⁵, from rings measured between about 18 and 32 °C.
pub(crate) const ARBABI_DNDT: f64 = 2.45e-5;
/// Elshaari et al.'s measured dn/dT of PECVD SiN (unannealed, 850 to 880 nm), per K, from 18 to
/// 300 K: the 30 points of their Fig. 4(a), read off the figure's drawing as Komma et al.'s
/// ([`KOMMA_FIG_4`]), T to 0.1 K and dn/dT to three figures. The first plots at 19.4 K, where
/// the text says 18 K. Their Eq. (3), a quartic fitted to the same data, lies 2 to 9 % above
/// these points from 60 to 300 K as printed (2.80e-5 at 300 K, against the text's 2.51e-5), so
/// the points are taken; their Eq. (4) for the oxide lies on Fig. 4(b)'s within 0.7 % from 40 K,
/// which checks the reading (the case `material/silicon-nitride-elshaari-reading`).
#[rustfmt::skip]
pub(crate) const ELSHAARI_FIG_4A: [(f64, f64); 30] = [
    (19.4, 2.00e-7), (20.6, 2.24e-7), (22.0, 2.58e-7), (24.3, 3.16e-7), (34.1, 6.29e-7),
    (40.6, 9.02e-7), (45.3, 1.12e-6), (50.0, 1.36e-6), (55.1, 1.66e-6), (59.9, 1.95e-6),
    (68.1, 2.55e-6), (76.4, 3.20e-6), (84.6, 3.92e-6), (92.5, 4.70e-6), (100.3, 5.50e-6),
    (110.7, 6.63e-6), (120.0, 7.74e-6), (130.0, 8.99e-6), (140.0, 1.03e-5), (149.9, 1.16e-5),
    (164.8, 1.36e-5), (178.7, 1.54e-5), (192.3, 1.72e-5), (209.3, 1.93e-5), (223.5, 2.09e-5),
    (239.1, 2.24e-5), (252.1, 2.35e-5), (269.6, 2.46e-5), (283.7, 2.53e-5), (298.9, 2.57e-5),
];
/// Where the default nitride passes from Elshaari et al.'s dn/dT (below) to Arbabi & Goddard's
/// (above): Elshaari et al.'s reach 2.45e-5 at 267 K, 2.462e-5 at 270 K, 0.5 % above.
pub(crate) const NITRIDE_JOIN_T: f64 = 270.0;
/// The temperature Luke et al.'s formula is taken at: the paper states none; room temperature,
/// inside Arbabi & Goddard's measurements.
pub(crate) const NITRIDE_T0: f64 = 295.0;
/// The default nitride's temperatures: Elshaari et al.'s first point to 400 K, Arbabi &
/// Goddard's dn/dT held above their 305 K (an extrapolation, as a heater needs).
pub(crate) const NITRIDE_T: (f64, f64) = (ELSHAARI_FIG_4A[0].0, 400.0);
/// The wavelengths the nitride takes away from [`NITRIDE_T0`]: Luke et al.'s formula at steps
/// of [`NITRIDE_STEP_UV`] λ² (µm), at most [`NITRIDE_STEP`]: 0.75 nm at 0.31 µm, where its
/// index bends most toward the resonance at 135 nm, 20 nm from 1.6 µm. The spline through them
/// is the formula plus the shift within 1e-6 across the range.
const NITRIDE_STEP: f64 = 0.02;
/// The step's factor on λ², per µm.
const NITRIDE_STEP_UV: f64 = 0.0078;

/// The wavelengths, µm, of the moved nitride's table: from `lo` to `hi` by [`NITRIDE_STEP_UV`]
/// λ², at most [`NITRIDE_STEP`], the last step shortened to end at `hi`.
fn nitride_wavelengths(lo: f64, hi: f64) -> Vec<f64> {
    let mut w = vec![lo];
    loop {
        let l = w[w.len() - 1];
        let next = l + NITRIDE_STEP.min(NITRIDE_STEP_UV * l * l);
        if next >= hi - 1e-9 {
            w.push(hi);
            return w;
        }
        w.push(next);
    }
}

/// dn/dT of the default nitride at T (K), per K: Elshaari et al.'s below [`NITRIDE_JOIN_T`],
/// Arbabi & Goddard's above.
pub(crate) fn nitride_dndt(t: f64) -> f64 {
    if t < NITRIDE_JOIN_T {
        between_rows(&ELSHAARI_FIG_4A, t)
    } else {
        ARBABI_DNDT
    }
}

/// The default nitride's change of index from [`NITRIDE_T0`] to `t`: its dn/dT integrated.
pub(crate) fn nitride_shift(t: f64) -> f64 {
    let j = NITRIDE_JOIN_T;
    if t >= j {
        ARBABI_DNDT * (t - NITRIDE_T0)
    } else {
        ARBABI_DNDT * (j - NITRIDE_T0) - between_rows_integral(&ELSHAARI_FIG_4A, t, j)
    }
}

/// Silicon nitride at a temperature (K): Luke et al.'s formula ([`material::silicon_nitride`],
/// bit for bit at [`NITRIDE_T0`]), elsewhere moved by [`nitride_shift`] at the wavelengths of
/// [`nitride_wavelengths`] across its range, 0.31 to 5.504 µm, splined.
pub(crate) fn luke_nitride(t: f64) -> Result<Material> {
    if t == NITRIDE_T0 {
        return Ok(material::silicon_nitride());
    }
    check_range("temperature", t, NITRIDE_T.0, NITRIDE_T.1)?;
    let base = material::silicon_nitride();
    let (lo, hi) = base.range();
    let (lo, hi) = (lo.to_um(), hi.to_um());
    let wavelengths = nitride_wavelengths(lo, hi);
    let count = wavelengths.len();
    let shift = nitride_shift(t);
    let n = wavelengths
        .iter()
        .map(|&lam| Ok(base.refractive_index(Wavelength::um(lam)?)?.re + shift))
        .collect::<Result<Vec<f64>>>()?;
    let p = base.provenance();
    let how = if t < NITRIDE_JOIN_T {
        "by Arbabi & Goddard's dn/dT of LPCVD Si3N4 at 1550 nm (doi:10.1364/OL.38.003878) to \
         270 K and below by Elshaari et al.'s measured dn/dT of PECVD SiN (Fig. 4(a), \
         doi:10.1109/JPHOT.2016.2561622)"
    } else if t > 305.0 {
        "by Arbabi & Goddard's dn/dT of LPCVD Si3N4 at 1550 nm (doi:10.1364/OL.38.003878), held \
         above their 305 K (an extrapolation)"
    } else {
        "by Arbabi & Goddard's dn/dT of LPCVD Si3N4 at 1550 nm (doi:10.1364/OL.38.003878)"
    };
    Material::new(
        "Si3N4",
        Model::Tabulated(Table::new(wavelengths, n, None)?),
        Wavelength::um(lo)?,
        Wavelength::um(hi)?,
        Provenance {
            reference: format!(
                "{}; A. Arbabi, L. L. Goddard, Opt. Lett. 38, 3878 (2013); A. W. Elshaari et al., \
                 IEEE Photonics J. 8, 2701009 (2016)",
                p.reference
            ),
            temperature: Some(t),
            notes: format!(
                "Luke et al.'s Eq. (1), taken at {NITRIDE_T0} K, moved to {t} K {how}, at \
                 {count} wavelengths from 0.31 to 5.504 µm"
            ),
            ..p.clone()
        },
    )
}

/// dn/dT of the default nitride, per K, at λ (µm) and T (K): [`nitride_dndt`], the same at every
/// wavelength of Luke et al.'s range.
pub(crate) fn luke_dndt(lam: f64, t: f64) -> Result<Vec<f64>> {
    check_range("temperature", t, NITRIDE_T.0, NITRIDE_T.1)?;
    check_range("wavelength", lam, 0.31, 5.504)?;
    Ok(vec![nitride_dndt(t)])
}

/// dn/dT, per K, of a model's materials at λ (µm) and T (K) by central differences 0.5 K wide
/// (one-sided within 0.5 K of the range's ends; second order either way), for a model with no
/// closed form.
pub(crate) fn by_differences(
    materials: impl Fn(f64) -> Result<Vec<Material>>,
    range: (f64, f64),
    lam: f64,
    t: f64,
) -> Result<Vec<f64>> {
    check_range("temperature", t, range.0, range.1)?;
    let w = Wavelength::um(lam)?;
    let at = |t: f64| -> Result<Vec<f64>> {
        materials(t)?
            .iter()
            .map(|m| Ok(m.refractive_index(w)?.re))
            .collect()
    };
    let h = 0.5;
    let (points, weights): (Vec<f64>, [f64; 3]) = if t - h < range.0 {
        (vec![t, t + h, t + 2.0 * h], [-3.0, 4.0, -1.0])
    } else if t + h > range.1 {
        (vec![t, t - h, t - 2.0 * h], [3.0, -4.0, 1.0])
    } else {
        (vec![t + h, t - h], [1.0, -1.0, 0.0])
    };
    let values = points.iter().map(|&p| at(p)).collect::<Result<Vec<_>>>()?;
    Ok((0..values[0].len())
        .map(|axis| {
            values
                .iter()
                .zip(weights)
                .map(|(v, w)| w * v[axis])
                .sum::<f64>()
                / (2.0 * h)
        })
        .collect())
}

//! Temperature in the default models of silicon and silica: their indices at a temperature and
//! their thermo-optic coefficients dn/dT.
//!
//! - Silicon: Li's table (293 K) moved to a temperature by his own Eq. (22), n(λ, T) for 1.2 to
//!   14 µm and 100 to 750 K (H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980),
//!   doi:10.1063/1.555624), from which he computed his Tables 1 and 2; below 250 K on by Frey,
//!   Leviton & Madison's formula (Proc. SPIE 6273, 62732J (2006), doi:10.1117/12.672850), 20 to
//!   300 K, which their measurements there follow more closely ([`JOIN_T`]).
//! - Silica: Malitson's formula (20 °C) moved by Leviton & Frey's change of index with
//!   temperature (Proc. SPIE 6273, 62732K (2006), doi:10.1117/12.672853), 30 to 300 K, and
//!   from 300 to 400 K by their dn/dT at 300 K, held: an extrapolation.

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
/// The default silicon's temperatures: Frey et al.'s lowest to Li's highest.
pub(crate) const SILICON_T: (f64, f64) = (models::FREY_SI_T.0, LI_T.1);
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
/// and up to 750 K, and below by Frey et al.'s formula on from there (to 20 K, at Li's points up
/// to 5 µm), splined as the table is.
pub(crate) fn li_silicon(t: f64) -> Result<Material> {
    if t == LI_T0 {
        return Ok(material::silicon());
    }
    check_range("temperature", t, SILICON_T.0, SILICON_T.1)?;
    if t >= JOIN_T {
        return li_eq22_silicon(t);
    }
    li_moved(
        t,
        COLD_LONGEST,
        |lam| {
            Ok(li_eq22(lam, JOIN_T).0 - li_eq22(lam, LI_T0).0 + frey(lam, t)? - frey(lam, JOIN_T)?)
        },
        "by his Eq. (22) to 250 K and below by Frey et al.'s formula (doi:10.1117/12.672850)",
    )
}

/// dn/dT of the default silicon, per K, at λ (µm) and T (K): Eq. (22)'s derivative (as Li's
/// Table 2 tabulates it) from [`JOIN_T`] up, Frey et al.'s formula's below.
pub(crate) fn li_dndt(lam: f64, t: f64) -> Result<Vec<f64>> {
    check_range("temperature", t, SILICON_T.0, SILICON_T.1)?;
    if t >= JOIN_T {
        check_range("wavelength", lam, 1.2, 14.0)?;
        Ok(vec![li_eq22(lam, t).1])
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

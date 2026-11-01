//! Miller's rule: a second-order coefficient carried from the wavelengths it was measured at to
//! others, through the linear susceptibilities of the three waves.
//!
//! R. C. Miller (Appl. Phys. Lett. 5, 17 (1964), doi:10.1063/1.1754022, Eq. (4)) wrote
//! d_ijk = χ_ii(ω₃) χ_jj(ω₁) χ_kk(ω₂) δ_ijk and found δ nearly the same for every crystal he
//! tabulated: "always within a factor of two of the average value", and for one coefficient
//! among crystals of one symmetry class "all equal to within ±50 %" (p. 17). R. W. Boyd
//! (*Nonlinear Optics*, 4th ed., Academic Press (2020), Sec. 1.4.2, Eqs. (1.4.25) and (1.4.28),
//! pp. 26–27) derives it from the anharmonic oscillator, whose χ⁽²⁾ is exactly such a product
//! with a constant δ; real crystals follow it approximately. With δ held, d at new frequencies
//! is d at measured ones times the ratio of the products ([`miller_scale`]). The susceptibilities
//! are χ = n² − 1 of the index each wave's polarization sees, the real part, so the rule is for
//! light below the band edge (a nonresonant model).

use crate::Result;
use crate::material::Material;
use crate::units::Wavelength;

/// χ⁽¹⁾ = n² − 1 of `material` at `wavelength`, from the real part of its index.
///
/// # Errors
///
/// Those of [`Material::refractive_index`]: a wavelength outside the material's range.
pub fn susceptibility(material: &Material, wavelength: Wavelength) -> Result<f64> {
    let n = material.refractive_index(wavelength)?.re;
    Ok(n * n - 1.0)
}

/// Miller's δ = d / (χ₃ χ₁ χ₂): `chi` the susceptibilities of the generated wave (ω₃ = ω₁ + ω₂)
/// and the two driving ones, in that order. In the units of d.
pub fn miller_delta(d: f64, chi: [f64; 3]) -> f64 {
    d / (chi[0] * chi[1] * chi[2])
}

/// d at new frequencies from d at measured ones, δ held: d · (χ₃′χ₁′χ₂′)/(χ₃χ₁χ₂), each array
/// the generated wave's susceptibility, then the driving waves'.
pub fn miller_scale(d: f64, measured: [f64; 3], target: [f64; 3]) -> f64 {
    d * (target[0] * target[1] * target[2]) / (measured[0] * measured[1] * measured[2])
}

/// The SHG coefficient d_ijk at the fundamental `to` from its value `d` at the fundamental
/// `from`: `harmonic` is the index the second harmonic's polarization (i) sees, `fundamental`
/// those of the two fundamental fields (j, k). For d₃₃ of a uniaxial crystal all three are n_e;
/// for d₃₁, n_e at the harmonic and n_o twice at the fundamental.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a wavelength, or its half, outside a material's range.
pub fn miller_shg(
    d: f64,
    from: Wavelength,
    to: Wavelength,
    harmonic: &Material,
    fundamental: [&Material; 2],
) -> Result<f64> {
    let chis = |lam: Wavelength| -> Result<[f64; 3]> {
        let half = Wavelength::um(0.5 * lam.to_um())?;
        Ok([
            susceptibility(harmonic, half)?,
            susceptibility(fundamental[0], lam)?,
            susceptibility(fundamental[1], lam)?,
        ])
    };
    Ok(miller_scale(d, chis(from)?, chis(to)?))
}

/// χ⁽¹⁾ = n² − 1 of `material` along its crystal's x, y and z at `wavelength` (an isotropic
/// material's three the same), each n the real part of the square root of the principal
/// permittivity.
///
/// # Errors
///
/// Those of [`Material::principal_permittivity`]: a wavelength outside the material's range.
pub fn principal_susceptibilities(material: &Material, wavelength: Wavelength) -> Result<[f64; 3]> {
    let eps = material.principal_permittivity(wavelength)?;
    Ok(eps.map(|e| {
        let n = e.sqrt().re;
        n * n - 1.0
    }))
}

/// The contracted index pairs (j, k) of the columns 1 to 6 of a d tensor in Voigt notation.
const PAIRS: [(usize, usize); 6] = [(0, 0), (1, 1), (2, 2), (1, 2), (0, 2), (0, 1)];

/// Miller's ratio for each element d_il of a d tensor in its crystal's axes (rows 1 to 3,
/// columns 11, 22, 33, 23, 13, 12), carrying it from the frequencies it was measured at to
/// others: `measured` and `target` each hold the principal susceptibilities (along the
/// crystal's x, y, z) of the generated wave ω₃ = ω₁ + ω₂ and of the two driving waves, in that
/// order.
///
/// d_ijk = δ χ_ii(ω₃) χ_jj(ω₁) χ_kk(ω₂) with δ held (Miller's Eq. (4)). Voigt notation keeps one
/// value for d_ijk and d_ikj, which Miller's product tells apart when ω₁ ≠ ω₂ and the crystal
/// isn't isotropic: the element takes the mean of the two orderings, χ_jj(ω₁)χ_kk(ω₂) and
/// χ_kk(ω₁)χ_jj(ω₂), which is each of them where j = k or the two driving waves are one
/// (second-harmonic generation).
pub fn miller_tensor_ratios(measured: [[f64; 3]; 3], target: [[f64; 3]; 3]) -> [[f64; 6]; 3] {
    let product = |chi: &[[f64; 3]; 3], i: usize, (j, k): (usize, usize)| {
        chi[0][i] * 0.5 * (chi[1][j] * chi[2][k] + chi[1][k] * chi[2][j])
    };
    std::array::from_fn(|i| {
        std::array::from_fn(|l| product(&target, i, PAIRS[l]) / product(&measured, i, PAIRS[l]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Model;
    use num_complex::Complex64 as c64;

    fn constant(n: f64) -> Material {
        Material::new(
            "n",
            Model::Constant(c64::new(n, 0.0)),
            Wavelength::um(0.2).unwrap(),
            Wavelength::um(5.0).unwrap(),
            crate::material::Provenance {
                reference: "a test".into(),
                doi: String::new(),
                data: String::new(),
                temperature: None,
                notes: String::new(),
            },
        )
        .unwrap()
    }

    #[test]
    fn a_dispersionless_medium_keeps_d_and_delta_is_the_ratio() {
        let m = constant(2.0);
        let (a, b) = (Wavelength::um(1.0).unwrap(), Wavelength::um(1.5).unwrap());
        assert_eq!(miller_shg(10.0, a, b, &m, [&m, &m]).unwrap(), 10.0);
        // δ = 27 / 3³ = 1
        assert_eq!(miller_delta(27.0, [3.0; 3]), 1.0);
        assert!((miller_scale(5.0, [1.0, 2.0, 3.0], [2.0, 2.0, 3.0]) - 10.0).abs() < 1e-15);
        assert!(miller_shg(1.0, a, Wavelength::um(0.3).unwrap(), &m, [&m, &m]).is_err());
    }

    #[test]
    fn each_element_takes_the_product_of_its_own_axes() {
        // a uniaxial crystal's χ: o, o, e at the generated wave and the two driving ones
        let measured = [[3.0, 3.0, 2.5], [2.8, 2.8, 2.4], [2.8, 2.8, 2.4]];
        let target = [[2.9, 2.9, 2.45], [2.7, 2.7, 2.3], [2.6, 2.6, 2.2]];
        let r = miller_tensor_ratios(measured, target);
        // d33 = d_zzz: e, e, e
        let d33 = (2.45 * 2.3 * 2.2) / (2.5 * 2.4 * 2.4);
        assert!((r[2][2] - d33).abs() < 1e-15);
        // d31 = d_zxx: e at ω₃, o, o
        assert!((r[2][0] - (2.45 * 2.7 * 2.6) / (2.5 * 2.8 * 2.8)).abs() < 1e-15);
        // d15 = d_xxz: the mean of the two orderings of the driving waves
        let d15 = (2.9 * 0.5 * (2.7 * 2.2 + 2.3 * 2.6)) / (3.0 * 2.8 * 2.4);
        assert!((r[0][4] - d15).abs() < 1e-15);
        // one medium at the measured and target frequencies: every ratio 1
        assert!(
            miller_tensor_ratios(measured, measured)
                .iter()
                .flatten()
                .all(|&x| x == 1.0)
        );
        // an isotropic constant index: χ along every axis n² − 1
        let m = constant(2.0);
        let chi = principal_susceptibilities(&m, Wavelength::um(1.0).unwrap()).unwrap();
        assert!(chi.iter().all(|&c| (c - 3.0).abs() < 1e-15));
    }
}

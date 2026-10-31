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
}

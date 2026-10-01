//! Optical materials: dispersion models, and data with their provenance.
//!
//! A [`Material`] is a dispersion [`Model`], the range of vacuum wavelengths it is valid for, and
//! its [`Provenance`]: the paper, the data source, and the temperature. Asking for a wavelength
//! outside the range is an error ([`Error::OutsideValidity`]): a model fitted to 1.2–14 µm says
//! nothing reliable at 1 µm.
//!
//! Every model follows the crate's e^(−iωt) convention ([`crate::units`]): a lossy material has a
//! positive imaginary permittivity.
//!
//! The built-in materials ([`silicon`], [`silica`], [`silicon_nitride`], [`vacuum`]) come from
//! the [refractiveindex.info database](https://github.com/polyanskiy/refractiveindex.info-database),
//! which is in the public domain (CC0 1.0), and each cites its paper.

use std::f64::consts::TAU;
use std::fmt;

use num_complex::Complex64;

use crate::units::{Frequency, Wavelength, refractive_index};
use crate::{Error, Result};

/// Where a material's data comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    /// The publication, e.g. `"I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965)"`.
    pub reference: String,
    /// Its DOI, e.g. `"10.1364/JOSA.55.001205"`, or empty when it has none.
    pub doi: String,
    /// Where the numbers were taken from, and under which license.
    pub data: String,
    /// The temperature the data holds for, in kelvin, when the source states one.
    pub temperature: Option<f64>,
    /// Anything else a user should know, e.g. the sample the data was measured on.
    pub notes: String,
}

/// One term B·λ²/(λ² − C²) of a [`Model::Sellmeier`] formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SellmeierTerm {
    /// The strength B, dimensionless.
    pub b: f64,
    /// The resonance wavelength C, in micrometres.
    pub c: f64,
}

/// One resonance of a [`Model::Lorentz`] permittivity:
/// Δε·ω₀² / (ω₀² − ω² − iγω).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LorentzPole {
    /// The strength Δε, dimensionless.
    pub strength: f64,
    /// The resonance frequency f₀ (ω₀ = 2πf₀).
    pub resonance: Frequency,
    /// The damping γ/2π, as a frequency; zero for a lossless resonance.
    pub damping: f64,
}

/// A dispersion model: how the permittivity depends on the wavelength.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Model {
    /// A constant refractive index.
    Constant(Complex64),
    /// The Sellmeier formula n² = A + Σ B·λ²/(λ² − C²), λ in micrometres; lossless.
    Sellmeier {
        /// The constant term A: 1 in the usual form n² − 1 = Σ ….
        a: f64,
        /// The terms.
        terms: Vec<SellmeierTerm>,
    },
    /// Cauchy's formula n = A₀ + A₁/λ² + A₂/λ⁴ + …, λ in micrometres; lossless.
    Cauchy(Vec<f64>),
    /// A Drude metal: ε = ε∞ − ω_p² / (ω² + iγω).
    Drude {
        /// The permittivity at high frequency, ε∞.
        eps_inf: f64,
        /// The plasma frequency f_p (ω_p = 2πf_p).
        plasma: Frequency,
        /// The collision rate γ/2π, as a frequency in c/µm.
        damping: f64,
    },
    /// Lorentz oscillators: ε = ε∞ + Σ Δε·ω₀² / (ω₀² − ω² − iγω).
    Lorentz {
        /// The permittivity at high frequency, ε∞.
        eps_inf: f64,
        /// The resonances.
        poles: Vec<LorentzPole>,
    },
    /// Measured n (and k) at given wavelengths, interpolated by natural cubic splines.
    Tabulated(Table),
}

/// Tabulated n and k, interpolated by natural cubic splines (continuous first and second
/// derivatives, so group indices are smooth too). Build it with [`Table::new`].
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    n: Spline,
    k: Option<Spline>,
}

impl Table {
    /// A table of wavelengths (micrometres, strictly increasing), refractive indices n and,
    /// optionally, extinction coefficients k (zero when `None`).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with fewer than two points, lengths that differ, wavelengths
    /// that aren't positive and strictly increasing, or values that aren't finite.
    pub fn new(wavelengths: Vec<f64>, n: Vec<f64>, k: Option<Vec<f64>>) -> Result<Table> {
        if wavelengths.len() < 2 {
            return Err(Error::invalid(
                "table",
                format!("needs at least 2 points, got {}", wavelengths.len()),
            ));
        }
        if !wavelengths.iter().all(|w| w.is_finite() && *w > 0.0) {
            return Err(Error::invalid(
                "table",
                "wavelengths must be positive and finite",
            ));
        }
        if wavelengths.windows(2).any(|w| w[1] <= w[0]) {
            return Err(Error::invalid(
                "table",
                "wavelengths must be strictly increasing",
            ));
        }
        let check = |name: &str, v: &[f64]| -> Result<()> {
            if v.len() != wavelengths.len() {
                return Err(Error::invalid(
                    "table",
                    format!(
                        "{name} has {} values for {} wavelengths",
                        v.len(),
                        wavelengths.len()
                    ),
                ));
            }
            if !v.iter().all(|x| x.is_finite()) {
                return Err(Error::invalid(
                    "table",
                    format!("{name} values must be finite"),
                ));
            }
            Ok(())
        };
        check("n", &n)?;
        if let Some(k) = &k {
            check("k", k)?;
        }
        Ok(Table {
            n: Spline::natural(wavelengths.clone(), n),
            k: k.map(|k| Spline::natural(wavelengths, k)),
        })
    }

    /// The wavelengths of the table, in micrometres.
    pub fn wavelengths(&self) -> &[f64] {
        &self.n.x
    }

    /// The complex index n + ik and its derivative with respect to λ (µm), at λ in µm.
    fn at(&self, lam: f64) -> (Complex64, Complex64) {
        let (n, dn) = self.n.eval(lam);
        let (k, dk) = self.k.as_ref().map_or((0.0, 0.0), |s| s.eval(lam));
        (Complex64::new(n, k), Complex64::new(dn, dk))
    }
}

/// A natural cubic spline through (x, y).
#[derive(Clone, Debug, PartialEq)]
struct Spline {
    x: Vec<f64>,
    y: Vec<f64>,
    /// the second derivatives at the knots
    m: Vec<f64>,
}

impl Spline {
    /// The natural spline (zero second derivative at both ends): the tridiagonal system for
    /// the knots' second derivatives, solved by the Thomas algorithm.
    fn natural(x: Vec<f64>, y: Vec<f64>) -> Spline {
        let n = x.len();
        let mut m = vec![0.0; n];
        if n > 2 {
            let mut diag = vec![0.0; n];
            let mut rhs = vec![0.0; n];
            let mut upper = vec![0.0; n];
            for i in 1..n - 1 {
                let (h0, h1) = (x[i] - x[i - 1], x[i + 1] - x[i]);
                let lower = h0 / 6.0;
                diag[i] = (h0 + h1) / 3.0;
                upper[i] = h1 / 6.0;
                rhs[i] = (y[i + 1] - y[i]) / h1 - (y[i] - y[i - 1]) / h0;
                if i > 1 {
                    let w = lower / diag[i - 1];
                    diag[i] -= w * upper[i - 1];
                    rhs[i] -= w * rhs[i - 1];
                }
            }
            for i in (1..n - 1).rev() {
                let next = if i + 1 < n - 1 { m[i + 1] } else { 0.0 };
                m[i] = (rhs[i] - upper[i] * next) / diag[i];
            }
        }
        Spline { x, y, m }
    }

    /// The value and the first derivative at `t`, which must lie within the knots.
    fn eval(&self, t: f64) -> (f64, f64) {
        let (x, y, m) = (&self.x, &self.y, &self.m);
        let i = match x.partition_point(|&v| v <= t) {
            0 => 0,
            p if p >= x.len() => x.len() - 2,
            p => p - 1,
        };
        let h = x[i + 1] - x[i];
        let (a, b) = ((x[i + 1] - t) / h, (t - x[i]) / h);
        let value = a * y[i]
            + b * y[i + 1]
            + ((a * a * a - a) * m[i] + (b * b * b - b) * m[i + 1]) * h * h / 6.0;
        let slope = (y[i + 1] - y[i]) / h
            + ((1.0 - 3.0 * a * a) * m[i] + (3.0 * b * b - 1.0) * m[i + 1]) * h / 6.0;
        (value, slope)
    }
}

impl Model {
    /// The relative permittivity at λ (µm), without a range check.
    fn permittivity(&self, lam: f64) -> Complex64 {
        let w = TAU / lam; // angular frequency in c/µm
        match self {
            Model::Constant(n) => n * n,
            Model::Sellmeier { a, terms } => {
                let l2 = lam * lam;
                let eps = a + terms
                    .iter()
                    .map(|t| t.b * l2 / (l2 - t.c * t.c))
                    .sum::<f64>();
                Complex64::new(eps, 0.0)
            }
            Model::Cauchy(coefficients) => {
                let inv2 = 1.0 / (lam * lam);
                let n: f64 = coefficients.iter().rev().fold(0.0, |acc, c| acc * inv2 + c);
                Complex64::new(n * n, 0.0)
            }
            Model::Drude {
                eps_inf,
                plasma,
                damping,
            } => {
                let wp = plasma.angular();
                let g = TAU * damping;
                eps_inf - wp * wp / Complex64::new(w * w, g * w)
            }
            Model::Lorentz { eps_inf, poles } => {
                let mut eps = Complex64::new(*eps_inf, 0.0);
                for p in poles {
                    let w0 = p.resonance.angular();
                    let g = TAU * p.damping;
                    eps += p.strength * w0 * w0 / Complex64::new(w0 * w0 - w * w, -g * w);
                }
                eps
            }
            Model::Tabulated(table) => {
                let (n, _) = table.at(lam);
                n * n
            }
        }
    }

    /// The complex index and its derivative with respect to λ (µm), without a range check.
    fn index_and_slope(&self, lam: f64) -> (Complex64, Complex64) {
        if let Model::Tabulated(table) = self {
            return table.at(lam);
        }
        // central difference: the models are smooth, and an O(h²) error at h = 1e-4·λ is
        // about 1e-9 relative, well below any data's accuracy
        let h = 1e-4 * lam;
        let n = |l: f64| refractive_index(self.permittivity(l));
        (n(lam), (n(lam + h) - n(lam - h)) / (2.0 * h))
    }
}

/// An optical material: a dispersion model, its range of validity, and where it comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    name: String,
    model: Model,
    range: (Wavelength, Wavelength),
    provenance: Provenance,
}

impl Material {
    /// A material valid for vacuum wavelengths from `shortest` to `longest`. A tabulated model
    /// must cover the whole range.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the range is empty, or a table doesn't cover it.
    pub fn new(
        name: impl Into<String>,
        model: Model,
        shortest: Wavelength,
        longest: Wavelength,
        provenance: Provenance,
    ) -> Result<Material> {
        let name = name.into();
        if shortest > longest {
            return Err(Error::invalid(
                "material",
                format!("{name}: the range {shortest} to {longest} is empty"),
            ));
        }
        if let Model::Tabulated(table) = &model {
            let w = table.wavelengths();
            if shortest.to_um() < w[0] || longest.to_um() > w[w.len() - 1] {
                return Err(Error::invalid(
                    "material",
                    format!(
                        "{name}: the table covers {} to {} um, not {shortest} to {longest}",
                        w[0],
                        w[w.len() - 1]
                    ),
                ));
            }
        }
        Ok(Material {
            name,
            model,
            range: (shortest, longest),
            provenance,
        })
    }

    /// The material's name, e.g. `"Si"`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The dispersion model.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The shortest and longest vacuum wavelengths the model is valid for.
    pub fn range(&self) -> (Wavelength, Wavelength) {
        self.range
    }

    /// Where the data comes from.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    fn check(&self, wavelength: Wavelength) -> Result<f64> {
        let (lo, hi) = self.range;
        if wavelength < lo || wavelength > hi {
            return Err(Error::OutsideValidity {
                material: self.name.clone(),
                wavelength_um: wavelength.to_um(),
                shortest_um: lo.to_um(),
                longest_um: hi.to_um(),
            });
        }
        Ok(wavelength.to_um())
    }

    /// The relative permittivity at a vacuum wavelength.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] outside the material's range.
    pub fn permittivity(&self, wavelength: Wavelength) -> Result<Complex64> {
        Ok(self.model.permittivity(self.check(wavelength)?))
    }

    /// The complex refractive index n + iκ at a vacuum wavelength, κ ≥ 0.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] outside the material's range.
    pub fn refractive_index(&self, wavelength: Wavelength) -> Result<Complex64> {
        let lam = self.check(wavelength)?;
        Ok(match &self.model {
            Model::Tabulated(table) => table.at(lam).0,
            model => refractive_index(model.permittivity(lam)),
        })
    }

    /// The group index n_g = n − λ·dn/dλ of the bulk material (real parts), at a vacuum
    /// wavelength.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] outside the material's range.
    pub fn group_index(&self, wavelength: Wavelength) -> Result<f64> {
        let lam = self.check(wavelength)?;
        let (n, slope) = self.model.index_and_slope(lam);
        Ok(n.re - lam * slope.re)
    }
}

impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} to {}; {})",
            self.name, self.range.0, self.range.1, self.provenance.reference
        )
    }
}

/// A built-in wavelength: positive and finite by construction.
const fn um(value: f64) -> Wavelength {
    Wavelength::from_um_unchecked(value)
}

const CC0: &str = "refractiveindex.info database (public domain, CC0 1.0)";

/// Vacuum: n = 1 at every wavelength.
pub fn vacuum() -> Material {
    Material {
        name: "vacuum".into(),
        model: Model::Constant(Complex64::new(1.0, 0.0)),
        range: (um(f64::MIN_POSITIVE), um(f64::MAX)),
        provenance: Provenance {
            reference: "by definition".into(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    }
}

/// Crystalline silicon at 293 K, 1.2–14 µm: H. H. Li's tabulated refractive index,
/// interpolated by a natural cubic spline. Lossless (below the band gap).
///
/// H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980),
/// [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624). The table is the
/// refractiveindex.info database's `main/Si/nk/Li-293K.yml` (CC0); its reference gives the
/// year as 1993, the DOI's record says 1980.
pub fn silicon() -> Material {
    const DATA: [(f64, f64); 35] = [
        (1.20, 3.5167),
        (1.22, 3.5133),
        (1.24, 3.5102),
        (1.26, 3.5072),
        (1.28, 3.5043),
        (1.30, 3.5016),
        (1.32, 3.4990),
        (1.34, 3.4965),
        (1.36, 3.4941),
        (1.38, 3.4918),
        (1.40, 3.4896),
        (1.45, 3.4845),
        (1.50, 3.4799),
        (1.55, 3.4757),
        (1.60, 3.4719),
        (1.65, 3.4684),
        (1.70, 3.4653),
        (1.80, 3.4597),
        (1.90, 3.4550),
        (2.00, 3.4510),
        (2.25, 3.4431),
        (2.50, 3.4375),
        (2.75, 3.4334),
        (3.00, 3.4302),
        (4.00, 3.4229),
        (5.00, 3.4195),
        (6.00, 3.4177),
        (7.00, 3.4165),
        (8.00, 3.4158),
        (9.00, 3.4153),
        (10.0, 3.4150),
        (11.0, 3.4147),
        (12.0, 3.4145),
        (13.0, 3.4144),
        (14.0, 3.4142),
    ];
    // valid by construction (tested): positive, increasing, finite
    let table = Table {
        n: Spline::natural(
            DATA.iter().map(|d| d.0).collect(),
            DATA.iter().map(|d| d.1).collect(),
        ),
        k: None,
    };
    Material {
        name: "Si".into(),
        model: Model::Tabulated(table),
        range: (um(1.2), um(14.0)),
        provenance: Provenance {
            reference: "H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980)".into(),
            doi: "10.1063/1.555624".into(),
            data: format!("{CC0}: main/Si/nk/Li-293K.yml"),
            temperature: Some(293.0),
            notes: "crystalline silicon; tabulated to 4 decimals".into(),
        },
    }
}

/// Fused silica (SiO₂) at 20 °C, 0.21–3.71 µm: Malitson's three-term Sellmeier formula.
///
/// I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965),
/// [doi:10.1364/JOSA.55.001205](https://doi.org/10.1364/JOSA.55.001205). Coefficients from the
/// refractiveindex.info database's `main/SiO2/nk/Malitson.yml` (CC0). The range is the one
/// Malitson reports; the database extends it to 6.7 µm on a later paper's authority.
pub fn silica() -> Material {
    Material {
        name: "SiO2".into(),
        model: Model::Sellmeier {
            a: 1.0,
            terms: vec![
                SellmeierTerm {
                    b: 0.6961663,
                    c: 0.0684043,
                },
                SellmeierTerm {
                    b: 0.4079426,
                    c: 0.1162414,
                },
                SellmeierTerm {
                    b: 0.8974794,
                    c: 9.896161,
                },
            ],
        },
        range: (um(0.21), um(3.71)),
        provenance: Provenance {
            reference: "I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965)".into(),
            doi: "10.1364/JOSA.55.001205".into(),
            data: format!("{CC0}: main/SiO2/nk/Malitson.yml"),
            temperature: Some(293.15),
            notes: "fused silica; thermal and deposited oxides differ slightly".into(),
        },
    }
}

/// Stoichiometric silicon nitride (Si₃N₄), 0.31–5.504 µm: Luke et al.'s two-term Sellmeier
/// formula, for LPCVD films.
///
/// K. Luke et al., Opt. Lett. 40, 4823 (2015),
/// [doi:10.1364/OL.40.004823](https://doi.org/10.1364/OL.40.004823). Coefficients from the
/// refractiveindex.info database's `main/Si3N4/nk/Luke.yml` (CC0).
pub fn silicon_nitride() -> Material {
    Material {
        name: "Si3N4".into(),
        model: Model::Sellmeier {
            a: 1.0,
            terms: vec![
                SellmeierTerm {
                    b: 3.0249,
                    c: 0.1353406,
                },
                SellmeierTerm {
                    b: 40314.0,
                    c: 1239.842,
                },
            ],
        },
        range: (um(0.31), um(5.504)),
        provenance: Provenance {
            reference: "K. Luke et al., Opt. Lett. 40, 4823 (2015)".into(),
            doi: "10.1364/OL.40.004823".into(),
            data: format!("{CC0}: main/Si3N4/nk/Luke.yml"),
            temperature: None,
            notes: "340 nm LPCVD Si3N4 on 3.1 um of thermal SiO2 on silicon".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lam(value: f64) -> Wavelength {
        Wavelength::um(value).unwrap()
    }

    fn n(material: &Material, at: f64) -> f64 {
        material.refractive_index(lam(at)).unwrap().re
    }

    #[test]
    fn silica_matches_the_standard_values() {
        let sio2 = silica();
        // the d line (587.56 nm) of fused silica: n_d = 1.4585, the glass makers' value
        assert!((n(&sio2, 0.5875618) - 1.4585).abs() < 1e-4);
        // 1.55 um: 1.4440
        assert!((n(&sio2, 1.55) - 1.4440).abs() < 1e-4);
    }

    #[test]
    fn silicon_nitride_matches_its_formula_at_1550_nm() {
        // n^2 = 1 + 3.0249 l^2/(l^2 - 0.1353406^2) + 40314 l^2/(l^2 - 1239.842^2) = 1.99628...
        assert!((n(&silicon_nitride(), 1.55) - 1.9963).abs() < 1e-4);
    }

    #[test]
    fn silicon_passes_through_lis_table() {
        let si = silicon();
        for (at, expected) in [(1.2, 3.5167), (1.55, 3.4757), (14.0, 3.4142)] {
            assert!((n(&si, at) - expected).abs() < 1e-12, "{at}");
        }
        // the built-in table passes the checks a user's table gets
        let Model::Tabulated(table) = si.model() else {
            panic!("silicon is tabulated")
        };
        let n_values: Vec<f64> = table.wavelengths().iter().map(|&w| n(&si, w)).collect();
        assert!(Table::new(table.wavelengths().to_vec(), n_values, None).is_ok());
        // between knots the spline stays between its neighbours (n falls with wavelength here)
        let mid = n(&si, 1.31);
        assert!(mid < 3.5016 && mid > 3.4990, "{mid}");
        assert_eq!(si.permittivity(lam(1.55)).unwrap().im, 0.0);
    }

    #[test]
    fn the_group_index_of_bulk_silicon_at_1550_nm_is_about_3_6() {
        // n_g = n - l dn/dl, with dn/dl of about -0.08 /um from the table
        let ng = silicon().group_index(lam(1.55)).unwrap();
        assert!((3.58..3.62).contains(&ng), "{ng}");
        // a formula and a table give the same kind of number for silica: 1.4626 at 1.55 um
        let ng = silica().group_index(lam(1.55)).unwrap();
        assert!((ng - 1.4626).abs() < 2e-3, "{ng}");
    }

    #[test]
    fn a_wavelength_outside_the_range_is_an_error() {
        let si = silicon();
        let e = si.refractive_index(lam(1.0)).unwrap_err();
        assert!(matches!(e, Error::OutsideValidity { .. }), "{e}");
        assert!(e.to_string().contains("Si"), "{e}");
        assert!(silica().permittivity(lam(4.0)).is_err());
        assert!(silicon_nitride().group_index(lam(0.3)).is_err());
    }

    #[test]
    fn every_built_in_material_cites_its_source() {
        for m in [silicon(), silica(), silicon_nitride()] {
            let p = m.provenance();
            assert!(p.doi.starts_with("10."), "{}", m.name());
            assert!(p.data.contains("CC0"), "{}", m.name());
            assert!(!p.reference.is_empty());
        }
    }

    #[test]
    fn lossy_models_have_a_positive_imaginary_permittivity() {
        let any = Provenance {
            reference: "test".into(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        };
        let drude = Material::new(
            "metal",
            Model::Drude {
                eps_inf: 1.0,
                plasma: Frequency::natural(7.0).unwrap(),
                damping: 0.05,
            },
            lam(0.4),
            lam(2.0),
            any.clone(),
        )
        .unwrap();
        let eps = drude.permittivity(lam(1.55)).unwrap();
        assert!(eps.re < 0.0 && eps.im > 0.0, "{eps}");
        let lorentz = Material::new(
            "oscillator",
            Model::Lorentz {
                eps_inf: 2.0,
                poles: vec![LorentzPole {
                    strength: 1.5,
                    resonance: Frequency::natural(2.0).unwrap(),
                    damping: 0.1,
                }],
            },
            lam(0.1),
            lam(100.0),
            any,
        )
        .unwrap();
        assert!(lorentz.permittivity(lam(0.5)).unwrap().im > 0.0);
        // far below resonance the static permittivity is eps_inf + strength
        let eps = lorentz.permittivity(lam(100.0)).unwrap();
        assert!((eps.re - 3.5).abs() < 1e-3, "{eps}");
        assert!(lorentz.refractive_index(lam(0.5)).unwrap().im > 0.0);
    }

    #[test]
    fn a_constant_index_has_no_dispersion() {
        let m = Material::new(
            "n=2",
            Model::Constant(Complex64::new(2.0, 0.0)),
            lam(0.5),
            lam(2.0),
            silica().provenance().clone(),
        )
        .unwrap();
        assert!((m.group_index(lam(1.0)).unwrap() - 2.0).abs() < 1e-9);
        assert_eq!(
            vacuum().refractive_index(lam(1.55)).unwrap(),
            Complex64::new(1.0, 0.0)
        );
    }

    #[test]
    fn cauchy_evaluates_its_series() {
        let m = Material::new(
            "cauchy",
            Model::Cauchy(vec![1.5, 0.004, 0.0001]),
            lam(0.4),
            lam(2.0),
            silica().provenance().clone(),
        )
        .unwrap();
        let expected = 1.5 + 0.004 / 0.25 + 0.0001 / 0.0625;
        assert!((n(&m, 0.5) - expected).abs() < 1e-15);
    }

    #[test]
    fn a_natural_spline_reproduces_a_straight_line() {
        let table = Table::new(vec![1.0, 1.5, 2.5, 3.0], vec![2.0, 2.5, 3.5, 4.0], None).unwrap();
        for t in [1.0, 1.2, 1.9, 2.75, 3.0] {
            let (v, d) = table.at(t);
            assert!(
                (v.re - (t + 1.0)).abs() < 1e-14 && (d.re - 1.0).abs() < 1e-13,
                "{t}"
            );
        }
    }

    #[test]
    fn bad_tables_are_errors() {
        assert!(Table::new(vec![1.0], vec![2.0], None).is_err());
        assert!(Table::new(vec![1.0, 1.0], vec![2.0, 2.0], None).is_err());
        assert!(Table::new(vec![2.0, 1.0], vec![2.0, 2.0], None).is_err());
        assert!(Table::new(vec![1.0, 2.0], vec![2.0], None).is_err());
        assert!(Table::new(vec![1.0, 2.0], vec![2.0, f64::NAN], None).is_err());
        assert!(Table::new(vec![1.0, 2.0], vec![2.0, 2.0], Some(vec![0.0])).is_err());
        let table = Table::new(vec![1.0, 2.0], vec![2.0, 2.0], None).unwrap();
        // a table must cover the material's range
        let e = Material::new(
            "short",
            Model::Tabulated(table),
            lam(0.5),
            lam(2.0),
            silica().provenance().clone(),
        );
        assert!(e.is_err());
    }
}

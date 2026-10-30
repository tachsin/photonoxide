//! A barium titanate film's dispersion: Suceava et al.'s ellipsometry of a strained film on
//! GdScO₃ (Adv. Mater. 38, e07564 (2025), doi:10.1002/adma.202507564, Supporting Note S3, Table
//! S1), its ordinary index from their Gaussian-oscillator model, and Tian et al.'s films on
//! silicon at 1550 nm (arXiv:2604.23259 (2026), Fig. 4(a), read off the figure).

use super::entries::{src, strings, table};
use super::{Axis, IndexModel};
use crate::Result;
use crate::material::{Material, Model, Provenance, Table};
use crate::units::Wavelength;
use crate::verdict::Outcome;

/// hc in eV·µm: the photon energy E = HC/λ.
const HC: f64 = 1.239842;

/// An oscillator of Table S1: its amplitude Amp, width Br (eV) and centre En (eV).
type Oscillator = (f64, f64, f64);

/// Table S1's Gaussian oscillators, room temperature.
pub(crate) const SUCEAVA_GAUSSIAN: [Oscillator; 4] = [
    (0.668082, 5.0302, 1.873),
    (7.428887, 0.2808, 3.756),
    (5.664778, 0.3357, 4.112),
    (8.632928, 0.6736, 4.677),
];

/// Table S1's Lorentz oscillators, room temperature: their other model of the same data.
pub(crate) const SUCEAVA_LORENTZ: [Oscillator; 4] = [
    (0.324128, 5.0491, 1.828),
    (4.259748, 0.1964, 3.781),
    (4.377518, 0.4444, 4.118),
    (7.828694, 0.6844, 4.705),
];

/// Both models' other terms, room temperature: ε∞, the IR pole's Amp_IR (En_IR = 0) and the UV
/// pole's Amp_UV and En_UV (eV); the film 36.53 nm thick.
pub(crate) const SUCEAVA_POLES: (f64, f64, f64, f64) = (0.0, 0.2831, 438.5576, 11.125);
pub(crate) const SUCEAVA_THICKNESS_UM: f64 = 0.03653;

/// Their GdScO₃ substrate's Cauchy model, n = A + B/λ² + C/λ⁴ (λ in µm).
pub(crate) const SUCEAVA_SUBSTRATE: [f64; 3] = [2.001, 0.01125, 0.00029005];

/// The wavelengths the catalogue's model spans, µm: from 0.5 µm, where the film's absorption edge
/// has passed, to the end of their data, 1689.3 nm.
pub(crate) const SUCEAVA_RANGE: (f64, f64) = (0.5, 1.6893);

/// Dawson's integral F(x) = e^{−x²} ∫₀ˣ e^{t²} dt: Simpson's rule on 2000 intervals up to |x| = 8,
/// its asymptotic series beyond.
pub(crate) fn dawson(x: f64) -> f64 {
    if x.abs() > 8.0 {
        let y = 1.0 / (x * x);
        return 0.5 / x * (1.0 + 0.5 * y + 0.75 * y * y + 1.875 * y * y * y);
    }
    let n = 2000;
    let h = x / n as f64;
    let s: f64 = (0..=n)
        .map(|i| {
            let t = i as f64 * h;
            let w = if i == 0 || i == n {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            w * (t * t - x * x).exp()
        })
        .sum();
    s * h / 3.0
}

/// The film's ordinary ε (real part, imaginary part) at photon energy `e` (eV) by Table S1's
/// model: ε∞ + Amp_UV/(En_UV² − E²) − Amp_IR/E² plus the oscillators, Gaussian or Lorentz.
pub(crate) fn suceava_epsilon(e: f64, gaussian: bool) -> (f64, f64) {
    let (eps_inf, amp_ir, amp_uv, en_uv) = SUCEAVA_POLES;
    let mut re = eps_inf + amp_uv / (en_uv * en_uv - e * e) - amp_ir / (e * e);
    let mut im = 0.0;
    let oscillators = if gaussian {
        SUCEAVA_GAUSSIAN
    } else {
        SUCEAVA_LORENTZ
    };
    for (amp, br, en) in oscillators {
        if gaussian {
            // ε₂ = Amp[e^{−((E−En)/σ)²} − e^{−((E+En)/σ)²}], σ = Br/(2√ln 2), and its
            // Kramers–Kronig ε₁ = (2Amp/√π)[F((E+En)/σ) − F((E−En)/σ)]
            let s = br / (2.0 * std::f64::consts::LN_2.sqrt());
            let (a, b) = ((e - en) / s, (e + en) / s);
            re += 2.0 * amp / std::f64::consts::PI.sqrt() * (dawson(b) - dawson(a));
            im += amp * ((-a * a).exp() - (-b * b).exp());
        } else {
            // Amp·Br·En/(En² − E² − iE·Br)
            let (dr, di) = (en * en - e * e, -e * br);
            let d2 = dr * dr + di * di;
            re += amp * br * en * dr / d2;
            im -= amp * br * en * di / d2;
        }
    }
    (re, im)
}

/// The film's ordinary n and k at `lam` µm by Table S1's model.
pub(crate) fn suceava_index(lam: f64, gaussian: bool) -> (f64, f64) {
    let (re, im) = suceava_epsilon(HC / lam, gaussian);
    let n = num_complex::Complex64::new(re, im).sqrt();
    (n.re, n.im)
}

/// The catalogue's model: the Gaussian model's n (not its k) every 10 nm over
/// [`SUCEAVA_RANGE`], through natural splines.
fn suceava_material() -> Result<Vec<Material>> {
    let (lo, hi) = SUCEAVA_RANGE;
    let steps = ((hi - lo) / 0.01).ceil() as usize;
    let w: Vec<f64> = (0..=steps)
        .map(|i| (lo + 0.01 * i as f64).min(hi))
        .collect();
    let n: Vec<f64> = w.iter().map(|&l| suceava_index(l, true).0).collect();
    Ok(vec![Material::new(
        "BaTiO3 strained film on GdScO3 (o)",
        Model::Tabulated(Table::new(w, n, None)?),
        Wavelength::um(lo)?,
        Wavelength::um(hi)?,
        Provenance {
            reference: "A. Suceava et al., Adv. Mater. 38, e07564 (2025)".into(),
            doi: "10.1002/adma.202507564".into(),
            data: "our evaluation of the Gaussian-oscillator model of Supporting Note S3, Table \
                   S1, every 10 nm"
                .into(),
            temperature: Some(295.0),
            notes: "the ordinary index of a 36.53 nm BaTiO3 film strained on GdScO3, at room \
                    temperature; lossless (the model's k isn't taken)"
                .into(),
        },
    )?])
}

/// Suceava et al.'s film as an index model.
pub(super) fn suceava_model() -> IndexModel {
    let rows = SUCEAVA_GAUSSIAN
        .iter()
        .enumerate()
        .map(|(i, (a, b, e))| {
            vec![
                format!("{}", i + 1),
                format!("{a}"),
                format!("{b}"),
                format!("{e}"),
            ]
        })
        .chain([
            strings(&["ε∞", "0", "", ""]),
            strings(&["IR pole", "0.2831", "", "0"]),
            strings(&["UV pole", "438.5576", "", "11.125"]),
        ])
        .collect();
    IndexModel {
        id: "suceava-2025".into(),
        name: "Suceava et al. 2025, strained film on GdScO₃".into(),
        axes: vec![Axis::Ordinary],
        equation: r"\varepsilon = \varepsilon_\infty + \frac{A_{UV}}{E_{UV}^2 - E^2} - \frac{A_{IR}}{E^2} + \sum_{i=1}^{4} A_i\left\{\frac{2}{\sqrt\pi}\left[F\!\left(\tfrac{E+E_i}{\sigma_i}\right) - F\!\left(\tfrac{E-E_i}{\sigma_i}\right)\right] + i\left[e^{-((E-E_i)/\sigma_i)^2} - e^{-((E+E_i)/\sigma_i)^2}\right]\right\}, \quad n_o = \operatorname{Re}\sqrt{\varepsilon}".into(),
        symbols: r"$E = 1.239842/\lambda$ in eV ($\lambda$ in µm); $A_i$, $E_i$ and the width $B_i$ (eV, full width at half maximum) of Gaussian oscillator $i$, $\sigma_i = B_i/(2\sqrt{\ln 2})$; $F$ Dawson's integral, which gives the Kramers–Kronig $\varepsilon_1$ of the Gaussian $\varepsilon_2$".into(),
        coefficients: vec![table(
            "Supporting Information, Note S3, Table S1: the Gaussian oscillator model, room temperature (film 36.53 nm)",
            &["oscillator", "$A_i$", "$B_i$ (eV)", "$E_i$ (eV)"],
            rows,
        )],
        wavelength: SUCEAVA_RANGE,
        temperature: None,
        composition: None,
        accuracy: "none stated for n; the paper's error budget for the electro-optic coefficient takes the index's change from 295 to 5 K, 3.4 %, as its largest error. The catalogue's n in the paper's thin-film reflectance at 1550 nm gives 0.1240 against the 0.124 they measured".into(),
        dndt: None,
        sources: vec![src(
            "suceava-2025",
            "Supporting Information, Note S3 and Table S1; main text, Fig. 2(e)",
        )],
        default: false,
        notes: "the ordinary index (light polarized in the film's plane) of a 36.53 nm BaTiO3 film grown on (110) GdScO3, strained, at room temperature, by variable-angle spectroscopic ellipsometry (Woollam M-2000XI at 45°, 55° and 65°, 271 to 1689 nm). 2.317 at 1550 nm; their Lorentz model of the same data gives 2.3174. The catalogue takes n only: the two models' k at 1550 nm, 0.061 and 0.120, differ twofold where their n agree within 2e-4, so the data don't fix it. The extraordinary index, the ordinary plus an extended-Cauchy difference whose form the paper leaves to the analysis software's manual, isn't reproduced. A strained film on an oxide substrate, not on silicon".into(),
        eval: |_, _| suceava_material(),
    }
}

/// Tian et al.'s three films' n at 1550 nm, read off Fig. 4(a) (a bitmap; a pixel at 10× is
/// 0.001 in n): PLD, hMBE_1, hMBE_2.
pub(crate) const TIAN_1550: [(&str, f64); 3] =
    [("PLD", 2.281), ("hMBE_1", 2.262), ("hMBE_2", 2.309)];

/// The reflectance at normal incidence of a film of index `n1`, `d` µm thick, on a substrate of
/// index `n2`, at `lam` µm, from air (both interfaces and the film's multiple reflections).
pub(crate) fn film_reflectance(n1: f64, d: f64, n2: f64, lam: f64) -> f64 {
    use num_complex::Complex64 as c;
    let r01 = (1.0 - n1) / (1.0 + n1);
    let r12 = (n1 - n2) / (n1 + n2);
    let phase = c::new(0.0, 4.0 * std::f64::consts::PI * n1 * d / lam).exp();
    let r = (c::new(r01, 0.0) + r12 * phase) / (c::new(1.0, 0.0) + r01 * r12 * phase);
    r.norm_sqr()
}

/// The catalogue's film (its n at 1550 nm), 36.53 nm on their GdScO₃ (their Cauchy model), at
/// normal incidence: the reflectance against the 0.124 they measured at 4° (Note S3).
pub(crate) fn suceava_reflectance() -> Outcome {
    let lam = 1.55;
    let n = suceava_material()
        .ok()
        .and_then(|m| {
            Wavelength::um(lam)
                .and_then(|w| m[0].refractive_index(w))
                .ok()
        })
        .map_or(f64::NAN, |n| n.re);
    let [a, b, cc] = SUCEAVA_SUBSTRATE;
    let substrate = a + b / (lam * lam) + cc / lam.powi(4);
    let r = film_reflectance(n, SUCEAVA_THICKNESS_UM, substrate, lam);
    Outcome {
        measured: r,
        expected: 0.124,
        tolerance: 5e-4,
        error: (r - 0.124).abs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dawson_takes_its_known_values() {
        for (x, f) in [
            (0.5, 0.424_436_383_502_022_3),
            (1.0, 0.538_079_506_912_768_4),
            (3.0, 0.178_271_030_610_558_3),
        ] {
            assert!((dawson(x) - f).abs() < 1e-10, "{x}: {}", dawson(x));
            assert!((dawson(-x) + f).abs() < 1e-10);
        }
        assert!((dawson(9.0) - 0.055_9).abs() < 1e-3);
    }

    #[test]
    fn the_models_and_the_check() {
        // the two models of the same data agree in n at 1550 nm, not in k
        let (ng, kg) = suceava_index(1.55, true);
        let (nl, kl) = suceava_index(1.55, false);
        assert!(
            (ng - 2.3172).abs() < 1e-4 && (nl - 2.3174).abs() < 1e-4,
            "{ng} {nl}"
        );
        assert!(
            (kg - 0.0612).abs() < 1e-4 && (kl - 0.1197).abs() < 1e-4,
            "{kg} {kl}"
        );
        let o = suceava_reflectance();
        assert!(o.passed(), "{o:?}");
        assert!((o.measured - 0.1240).abs() < 1e-4, "{o:?}");
        // with the models' k the reflectance is 0.1294 (Gaussian) and 0.1345 (Lorentz), where
        // the paper computes 0.1237 and 0.1292
        let sub = 2.001 + 0.01125 / 1.55f64.powi(2) + 0.00029005 / 1.55f64.powi(4);
        let lossy = |n: f64, k: f64| {
            use num_complex::Complex64 as c;
            let n1 = c::new(n, k);
            let r01 = (1.0 - n1) / (1.0 + n1);
            let r12 = (n1 - sub) / (n1 + sub);
            let ph = (c::new(
                0.0,
                4.0 * std::f64::consts::PI * SUCEAVA_THICKNESS_UM / 1.55,
            ) * n1)
                .exp();
            ((r01 + r12 * ph) / (1.0 + r01 * r12 * ph)).norm_sqr()
        };
        assert!((lossy(ng, kg) - 0.1294).abs() < 1e-4);
        assert!((lossy(nl, kl) - 0.1345).abs() < 1e-4);
        // the tabulated model follows the closed form between its points
        let m = suceava_material().unwrap();
        let at = |l: f64| {
            m[0].refractive_index(Wavelength::um(l).unwrap())
                .unwrap()
                .re
        };
        for l in [0.555, 0.987, 1.3105, 1.6845] {
            assert!((at(l) - suceava_index(l, true).0).abs() < 1e-5, "{l}");
        }
    }
}

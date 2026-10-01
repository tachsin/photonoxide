//! The validation harness: every check of photonoxide against something it must agree with.
//!
//! A [`Case`] compares one number photonoxide computes with what it should be, on one of three
//! [`Tier`]s:
//!
//! - **analytic:** a closed-form solution;
//! - **cross-code:** an established code on the same structure;
//! - **published:** a published measurement or result.
//!
//! [`cases`] lists them all, and [`report`] runs them and writes the markdown report that
//! `photonoxide validate` keeps in `docs/validation.md`. CI fails when a case fails or the
//! committed report is out of date. The report shows values to six significant digits and the
//! tolerance, not the raw error, so it reads the same on every platform.

use std::f64::consts::TAU;
use std::fmt::Write as _;

use num_complex::Complex64;

use crate::material::{self, Model, Table};
use crate::units::{Wavelength, refractive_index};

/// What a case is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// A closed-form solution.
    Analytic,
    /// An established code on the same structure.
    CrossCode,
    /// A published measurement or result.
    Published,
}

impl Tier {
    fn label(self) -> &'static str {
        match self {
            Tier::Analytic => "analytic",
            Tier::CrossCode => "cross-code",
            Tier::Published => "published",
        }
    }
}

/// What a case measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// The value photonoxide computed, as reported.
    pub measured: f64,
    /// The value it should be.
    pub expected: f64,
    /// The largest error allowed: absolute, in the value's own unit.
    pub tolerance: f64,
    /// The actual error, |measured − expected| or a norm over several values.
    pub error: f64,
}

impl Outcome {
    /// Whether the error is within the tolerance.
    pub fn passed(&self) -> bool {
        self.error.is_finite() && self.error <= self.tolerance
    }
}

/// One validation case.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    /// A short, stable identifier, e.g. `"material/silicon-li-table"`.
    pub id: &'static str,
    /// What is checked, in a sentence.
    pub title: &'static str,
    /// What it is checked against.
    pub tier: Tier,
    /// The source of the expected value: a reference with its DOI, or the closed form.
    pub source: &'static str,
    /// Runs the case.
    pub run: fn() -> Outcome,
}

/// Every validation case, in report order.
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "units/amplitude-convention",
            title: "The amplitude of a real signal Re(A e^(-iwt)) is recovered with the kernel e^(+iwt) (magnitude shown)",
            tier: Tier::Analytic,
            source: "the e^(-iwt) convention: (2/T) int Re(A e^(-iwt)) e^(iwt) dt = A over whole periods",
            run: amplitude_convention,
        },
        Case {
            id: "units/lossy-attenuation",
            title: "A wave in a medium with Im(eps) > 0 decays over one wavelength by exp(-2 pi kappa) (decay shown)",
            tier: Tier::Analytic,
            source: "e^(i n k0 x) with n = n' + i kappa, kappa >= 0",
            run: lossy_attenuation,
        },
        Case {
            id: "material/spline-line",
            title: "A natural cubic spline through points on a line is that line (largest deviation shown)",
            tier: Tier::Analytic,
            source: "a line has zero second derivative, which the natural end conditions impose",
            run: spline_line,
        },
        Case {
            id: "material/silicon-li-table",
            title: "Silicon's index passes through Li's table at all 35 points (largest deviation shown)",
            tier: Tier::Published,
            source: "H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), doi:10.1063/1.555624, 293 K",
            run: silicon_table,
        },
    ]
}

fn amplitude_convention() -> Outcome {
    let a = Complex64::new(0.7, -0.4);
    let w = TAU * 0.65;
    let n = 20_000;
    let t_end = 20.0 * TAU / w;
    let dt = t_end / n as f64;
    let mut sum = Complex64::new(0.0, 0.0);
    for i in 0..n {
        let t = (i as f64 + 0.5) * dt;
        let s = (a * Complex64::new(0.0, -w * t).exp()).re;
        sum += s * Complex64::new(0.0, w * t).exp() * dt;
    }
    let got = sum * (2.0 / t_end);
    Outcome {
        measured: got.norm(),
        expected: a.norm(),
        tolerance: 1e-9,
        error: (got - a).norm(),
    }
}

fn lossy_attenuation() -> Outcome {
    let n = refractive_index(Complex64::new(12.0, 0.5));
    let lam = Wavelength::um(1.0).map_or(1.0, |l| l.to_um());
    let k0 = TAU / lam;
    let field = |x: f64| (Complex64::i() * n * k0 * x).exp().norm();
    let measured = field(lam) / field(0.0);
    let expected = (-TAU * n.im).exp();
    Outcome {
        measured,
        expected,
        tolerance: 1e-12,
        error: (measured - expected).abs(),
    }
}

fn spline_line() -> Outcome {
    let x = vec![1.0, 1.3, 2.0, 2.2, 3.5];
    let y: Vec<f64> = x.iter().map(|v| 0.25 * v + 1.5).collect();
    let table = Table::new(x, y, None);
    let worst = match table {
        Ok(table) => {
            let m = material::Material::new(
                "line",
                Model::Tabulated(table),
                um(1.0),
                um(3.5),
                material::silica().provenance().clone(),
            );
            match m {
                Ok(m) => (0..=50)
                    .map(|i| {
                        let t = 1.0 + 2.5 * i as f64 / 50.0;
                        let n = m.refractive_index(um(t)).map_or(f64::NAN, |n| n.re);
                        (n - (0.25 * t + 1.5)).abs()
                    })
                    .fold(0.0, f64::max),
                Err(_) => f64::NAN,
            }
        }
        Err(_) => f64::NAN,
    };
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-13,
        error: worst,
    }
}

fn silicon_table() -> Outcome {
    // Li's table as the refractiveindex.info database holds it (main/Si/nk/Li-293K.yml, CC0)
    const LI: [(f64, f64); 35] = [
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
    let si = material::silicon();
    let worst = LI
        .iter()
        .map(|&(l, n)| {
            let got = si.refractive_index(um(l)).map_or(f64::NAN, |v| v.re);
            (got - n).abs()
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn um(value: f64) -> Wavelength {
    Wavelength::from_um_unchecked(value)
}

/// Six significant digits; values below 1e-300 in magnitude as 0.
fn sig(v: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    if v.abs() < 1e-300 {
        return "0".into();
    }
    let digits = 6 - 1 - v.abs().log10().floor() as i32;
    if (-4..6).contains(&(5 - digits)) {
        format!("{:.*}", digits.max(0) as usize, v)
    } else {
        format!("{v:.5e}")
    }
}

/// A value that is ideally zero: "0" up to the tolerance, else its order of magnitude, so the
/// report doesn't change with the last bits of a platform's math library.
fn small(v: f64, tolerance: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    if v <= tolerance {
        format!("≤ {tolerance:e}")
    } else {
        format!("{v:.1e}")
    }
}

/// Runs every case and returns the markdown report, and whether every case passed.
pub fn report() -> (String, bool) {
    let mut all = true;
    let mut rows = String::new();
    for case in cases() {
        let o = (case.run)();
        let passed = o.passed();
        all &= passed;
        let (measured, expected) = if o.expected == 0.0 {
            (small(o.measured, o.tolerance), "0".to_owned())
        } else {
            (sig(o.measured), sig(o.expected))
        };
        debug_assert!(!case.title.contains('|') && !case.source.contains('|'));
        let _ = writeln!(
            rows,
            "| `{}` | {} | {} | {} | {} | {} | {:e} | {} |",
            case.id,
            case.tier.label(),
            case.title,
            case.source,
            measured,
            expected,
            o.tolerance,
            if passed { "pass" } else { "**FAIL**" }
        );
    }
    let text = format!(
        "# Validation report\n\
         \n\
         Written by `photonoxide validate`; don't edit it by hand. CI fails when a case fails or \
         this file is out of date. Values have six significant digits; a value that should be \
         zero shows as \"≤ tolerance\" when it is within it.\n\
         \n\
         | Case | Tier | What | Against | Measured | Expected | Tolerance | Result |\n\
         |---|---|---|---|---|---|---|---|\n\
         {rows}"
    );
    (text, all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_passes() {
        for case in cases() {
            let o = (case.run)();
            assert!(o.passed(), "{}: {o:?}", case.id);
        }
    }

    #[test]
    fn no_case_text_breaks_the_markdown_table() {
        for case in cases() {
            assert!(
                !case.title.contains('|') && !case.source.contains('|'),
                "{}",
                case.id
            );
        }
    }

    #[test]
    fn case_ids_are_unique() {
        let ids: Vec<&str> = cases().iter().map(|c| c.id).collect();
        for (i, id) in ids.iter().enumerate() {
            assert!(!ids[..i].contains(id), "{id}");
        }
    }

    #[test]
    fn the_report_has_a_row_per_case() {
        let (text, all) = report();
        assert!(all);
        assert_eq!(text.matches("| pass |").count(), cases().len());
    }

    #[test]
    fn significant_digits() {
        assert_eq!(sig(0.806226), "0.806226");
        assert_eq!(sig(3.47570001), "3.47570");
        assert_eq!(sig(123456.7), "123457");
        assert_eq!(sig(1.5e-9), "1.50000e-9");
        assert_eq!(small(3e-14, 1e-12), "≤ 1e-12");
        assert_eq!(small(3e-10, 1e-12), "3.0e-10");
    }

    #[test]
    fn a_failing_outcome_fails() {
        let o = Outcome {
            measured: 1.0,
            expected: 0.0,
            tolerance: 0.5,
            error: 1.0,
        };
        assert!(!o.passed());
        let nan = Outcome {
            error: f64::NAN,
            ..o
        };
        assert!(!nan.passed());
    }
}

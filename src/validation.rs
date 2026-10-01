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
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::units::{Length, Wavelength, refractive_index};

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
            source: "H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), doi:10.1063/1.555624, Table 1, 293 K",
            run: silicon_table,
        },
        Case {
            id: "material/silica-malitson-formula",
            title: "Silica's index equals Malitson's computed index at his 60 wavelengths (largest deviation shown)",
            tier: Tier::Published,
            source: "I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965), doi:10.1364/JOSA.55.001205, Table I, computed index to 6 decimals",
            run: silica_formula,
        },
        Case {
            id: "material/silica-malitson-measured",
            title: "Silica's index matches the measured mean of three specimens at 60 wavelengths, to five decimals (largest deviation shown)",
            tier: Tier::Published,
            source: "I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965), doi:10.1364/JOSA.55.001205, Table I, computed index plus the C-D-G.E. residual",
            run: silica_measured,
        },
        Case {
            id: "mode/slab-te-book",
            title: "The TE mode of 220 nm of silicon (3.473) in oxide (1.444) at 1550 nm (effective index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Section 3.2.2: 2.845 (3 decimals)",
            run: slab_te_book,
        },
        Case {
            id: "mode/slab-tm-book",
            title: "The TM mode of 220 nm of silicon (3.473) in oxide (1.444) at 1550 nm (effective index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Section 3.2.2: 2.051 (3 decimals)",
            run: slab_tm_book,
        },
        Case {
            id: "mode/vector-slab-limit-te",
            title: "The full-vector solver on the book's slab, uniform along one axis, at a 2.5 nm mesh: TE (error against the exact slab shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab); the scheme converges at second order, tested at 20, 10 and 5 nm",
            run: vector_slab_te,
        },
        Case {
            id: "mode/vector-slab-limit-tm",
            title: "The full-vector solver on the book's slab, uniform along one axis, at a 2.5 nm mesh: TM (error against the exact slab shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab); the scheme converges at second order, tested at 20, 10 and 5 nm",
            run: vector_slab_tm,
        },
        Case {
            id: "mode/strip-book",
            title: "The TE-like mode of a 500 x 220 nm silicon strip in oxide at 1550 nm, at a 5 nm mesh (effective index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Fig. 3.14: 2.443 (Lumerical MODE, 20 nm conformal mesh, accurate to about 1e-3 by its Fig. 3.9); ours converges at about first order at the convex corners (as on Hadley's corner problems below), 2.4435 at 2.5 nm",
            run: strip_book,
        },
        Case {
            id: "mode/hadley-box-low",
            title: "Hadley's corner problem 1: a box, eps 2.25 in a quarter of the 1 x 1 um domain, at 1.5 um, on an 80 x 80 grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 4: 1.27627404 +- 1e-8 (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_1,
        },
        Case {
            id: "mode/hadley-box-high",
            title: "Hadley's corner problem 2: a box, eps 8 in a quarter of the 1 x 1 um domain, at 1.5 um, on an 80 x 80 grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 5: 2.65679692 +- 1e-8 (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_2,
        },
        Case {
            id: "mode/hadley-corner-low",
            title: "Hadley's corner problem 3: an impinged corner, eps 2.25 in three quarters of the 1 x 1 um domain, at 1.5 um, on an 80 x 80 grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 6: 1.387926425 +- 2e-9 (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_3,
        },
        Case {
            id: "mode/hadley-corner-high",
            title: "Hadley's corner problem 4: an impinged corner, eps 8 in three quarters of the 1 x 1 um domain, at 1.5 um, on an 80 x 80 grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 7: 2.761465320 +- 5e-9 (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_4,
        },
        Case {
            id: "mode/slab-group-index",
            title: "The group index of a TE slab (220 nm of 3.473 between 1.444 and air) at 1.55 um, from differences of its effective index over +-2 nm (error shown)",
            tier: Tier::Analytic,
            source: "Hellmann-Feynman, no material dispersion: n_g = <eps>/n_eff, <eps> weighted by E^2 of the exact field",
            run: slab_group_index,
        },
        Case {
            id: "mode/strip-group-index-book",
            title: "The group index of a 500 x 220 nm strip at 1.55 um, with the book's dispersive silicon and 1.444 oxide, on a 6.25 x 5 nm grid (group index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Fig. 3.22b: about 4.18, read off the plot to +-0.005 (Lumerical MODE, 20 nm mesh); materials from its Listing 3.1",
            run: strip_group_index_book,
        },
        Case {
            id: "mode/multilayer-bound-chilwell",
            title: "The 8 bound modes (TE and TM 0-3) of a four-layer guide, 1.0 / 1.66, 1.53, 1.60, 1.66 (500 nm each) / 1.50 at 632.8 nm, exact (largest deviation shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 3: effective indices to 6 decimals",
            run: multilayer_bound,
        },
        Case {
            id: "mode/multilayer-leaky-chilwell",
            title: "The 5 TE leaky waves (m = 4-8) of the same guide, complex effective indices, exact (largest deviation of a real or imaginary part shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 2: to 5 decimals; m = 5's real part, 1.38250, is ours (1.3824892) plus 1.1e-5, one unit in the last place, the other nine our values rounded",
            run: multilayer_leaky,
        },
        Case {
            id: "mode/multilayer-power-chilwell",
            title: "The share of each bound mode's power in the cover, each film and the substrate, 48 percentages (largest deviation shown, in percentage points)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 3: to 0.1 %",
            run: multilayer_power,
        },
        Case {
            id: "mode/pml-soi-leakage-te",
            title: "The loss of 220 nm SOI's TE mode leaking through 0.5 um of buried oxide into the substrate, full-vector with a PML (1 um, strength 3), 2.5 nm grid (relative error in Im n_eff shown)",
            tier: Tier::Analytic,
            source: "the exact leaky mode of the same stack by transfer matrices (mode::multilayer); PML by complex coordinate stretching, W. C. Chew et al., Microw. Opt. Technol. Lett. 15, 363 (1997)",
            run: pml_soi_te,
        },
        Case {
            id: "mode/pml-soi-leakage-tm",
            title: "The same for the TM mode (relative error in Im n_eff shown)",
            tier: Tier::Analytic,
            source: "the exact leaky mode of the same stack by transfer matrices (mode::multilayer); PML by complex coordinate stretching, W. C. Chew et al., Microw. Opt. Technol. Lett. 15, 363 (1997)",
            run: pml_soi_tm,
        },
        Case {
            id: "mode/pml-leaky-chilwell",
            title: "Chilwell and Hodgkinson's TE leaky waves m = 4-7, full-vector with a PML (2 um, strength 5) in the substrate, 2.5 nm grid (largest deviation of a real or imaginary part shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 2, to 5 decimals; m = 8 (Re 1.00304, just above the cover's 1.0) has a slowly decaying, inward-phased field in the cover and is checked only in the leaky_waves example, within 2e-4",
            run: pml_leaky_chilwell,
        },
        Case {
            id: "mode/eim-strip-book",
            title: "The effective index method on a 500 x 220 nm silicon strip (3.473 in 1.444) at 1550 nm, TE-like, exact slabs (effective index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Section 3.2.5: 2.489, from the slab index rounded to 2.845 and a 10 nm 1D mesh (on that input the exact lateral slab gives 2.488558); the method: G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), doi:10.1364/AO.16.000113",
            run: eim_strip_book,
        },
        Case {
            id: "mode/bend-slab-te",
            title: "A slab (2.845, 500 nm, in 1.444) bent at 1 um, E normal to the bend plane, full-vector on a conformally mapped 2.5 nm grid with a PML: effective index along the arc (error shown)",
            tier: Tier::Analytic,
            source: "the exact bent slab (mode::bend: radial shooting matched to the outgoing Hankel function, D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x, Eq. 10); the map: M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975), doi:10.1109/JQE.1975.1068563, exact for this polarization; second order",
            run: bend_te_index,
        },
        Case {
            id: "mode/bend-slab-te-loss",
            title: "The same bend's radiation loss, Im n_eff = 9.29e-4 (relative error shown)",
            tier: Tier::Analytic,
            source: "the exact bent slab (mode::bend); the PML starts at 2.5 um, outside the bend's turning point",
            run: bend_te_loss,
        },
        Case {
            id: "mode/bend-slab-tm",
            title: "The same bend with E in the bend plane, where scaling an isotropic permittivity is an approximation (error shown)",
            tier: Tier::Analytic,
            source: "the exact bent slab (mode::bend); the exact equivalent medium would be anisotropic in both permittivity and permeability; the error falls as the radius grows (1.3e-4 at 3 um)",
            run: bend_tm_index,
        },
        Case {
            id: "mode/bend-marcuse",
            title: "Marcuse's bending-loss formula against the exact loss of a slab (1.6 in 1.5, 1 um, at 1 um) bent at 120 um (ratio minus one shown)",
            tier: Tier::Published,
            source: "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x, Eqs. 32-33, an approximation for large radii: its deviation falls as 1/R, 0.14 at 80 um, 0.084 at 120 um and 0.061 at 160 um",
            run: bend_marcuse,
        },
        Case {
            id: "mode/leaky-wire-bienstman",
            title: "The leaky SOI wire benchmark (500 x 220 nm Si 3.5 on 1 um SiO2 1.45 on Si, air above, 1.55 um), TE: Re n_eff, Richardson-extrapolated from core grids of 5, 2.5 and 1.25 nm (order ~0.67, the corners'), with a PML in the substrate",
            tier: Tier::Published,
            source: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), doi:10.1007/s11082-006-9025-9, Table 6: 2.412372, from CAMFR and the aperiodic Fourier modal method (7 digits); raw errors +4.0e-3, +2.5e-3, +1.5e-3",
            run: bienstman_re,
        },
        Case {
            id: "mode/leaky-wire-bienstman-loss",
            title: "The same wire's substrate leakage, Im n_eff x 1e8, extrapolated alike",
            tier: Tier::Published,
            source: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), doi:10.1007/s11082-006-9025-9, Table 6: 2.9135 (CAMFR) and 2.91348 (aperiodic Fourier modal method); the raw results are 0.97, 0.98 and 0.99 of it",
            run: bienstman_im,
        },
        Case {
            id: "mode/fields-butt-coupling",
            title: "The power a 220 nm silicon slab's TE mode launches into a 300 nm slab's (3.473 in 1.444, 1.55 um), from the full-vector fields on a 5 nm grid (error shown)",
            tier: Tier::Analytic,
            source: "the exact slab fields (mode::slab): for TE slabs H is proportional to E, so the coupling is (int E1 E2)^2 / (int E1^2 int E2^2) = 0.994662",
            run: fields_butt_coupling,
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

/// Malitson's Table I: wavelength (µm), the index computed by his Eq. (1) (6 decimals), and the
/// residual of the measured mean of the Corning, Dynasil and General Electric specimens
/// (measured − computed, ×10⁻⁶), as printed. The residuals' mean absolute value is the paper's
/// 10.5 × 10⁻⁶ (tested), a check on the transcription.
const MALITSON_TABLE_I: [(f64, f64, i32); 60] = [
    (0.213856, 1.534307, -27),
    (0.214438, 1.533722, -2),
    (0.226747, 1.522750, 70),
    (0.230209, 1.520081, -21),
    (0.237833, 1.514729, 1),
    (0.239938, 1.513367, 3),
    (0.248272, 1.508398, 2),
    (0.265204, 1.500029, -29),
    (0.269885, 1.498047, 3),
    (0.275278, 1.495913, -3),
    (0.280347, 1.494039, 1),
    (0.289360, 1.490990, 20),
    (0.296728, 1.488734, -14),
    (0.302150, 1.487194, -4),
    (0.330259, 1.480539, -9),
    (0.334148, 1.479763, -3),
    (0.340365, 1.478584, 6),
    (0.346620, 1.477468, 2),
    (0.361051, 1.475129, 1),
    (0.365015, 1.474539, -19),
    (0.404656, 1.469618, 2),
    (0.435835, 1.466693, -3),
    (0.467816, 1.464292, 8),
    (0.486133, 1.463126, 4),
    (0.508582, 1.461863, 7),
    (0.546074, 1.460078, 2),
    (0.576959, 1.458846, 4),
    (0.579065, 1.458769, 1),
    (0.587561, 1.458464, 6),
    (0.589262, 1.458404, -4),
    (0.643847, 1.456704, 6),
    (0.656272, 1.456367, 3),
    (0.667815, 1.456067, 3),
    (0.706519, 1.455145, 5),
    (0.852111, 1.452465, 5),
    (0.894350, 1.451835, 5),
    (1.01398, 1.450242, 8),
    (1.08297, 1.449405, -5),
    (1.12866, 1.448869, 1),
    (1.3622, 1.446212, -12),
    (1.39506, 1.445836, 4),
    (1.4695, 1.444975, -5),
    (1.52952, 1.444268, 2),
    (1.6606, 1.442670, -20),
    (1.681, 1.442414, 6),
    (1.6932, 1.442260, 0),
    (1.70913, 1.442057, 3),
    (1.81307, 1.440699, 21),
    (1.97009, 1.438519, 1),
    (2.0581, 1.437224, -4),
    (2.1526, 1.435769, -29),
    (2.32542, 1.432928, -18),
    (2.4374, 1.430954, -24),
    (3.2439, 1.413118, 32),
    (3.2668, 1.412505, 25),
    (3.3026, 1.411535, 25),
    (3.422, 1.408180, 20),
    (3.5070, 1.405676, -16),
    (3.5564, 1.404174, -24),
    (3.7067, 1.399389, -19),
];

/// The largest |n − reference| over Malitson's wavelengths.
fn silica_worst(reference: impl Fn(f64, i32) -> f64) -> f64 {
    let sio2 = material::silica();
    MALITSON_TABLE_I
        .iter()
        .map(|&(l, computed, residual)| {
            let n = sio2.refractive_index(um(l)).map_or(f64::NAN, |v| v.re);
            (n - reference(computed, residual)).abs()
        })
        .fold(0.0, f64::max)
}

fn silica_formula() -> Outcome {
    // the printed index has 6 decimals (±5e-7), and some wavelengths only 4 or 5 digits
    let worst = silica_worst(|computed, _| computed);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-6,
        error: worst,
    }
}

fn silica_measured() -> Outcome {
    // Malitson: the formula interpolates the measurements to five decimal places
    let worst = silica_worst(|computed, residual| computed + f64::from(residual) * 1e-6);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-4,
        error: worst,
    }
}

/// The fundamental slab mode of the book's example, against its printed effective index.
fn slab_book(polarization: Polarization, expected: f64) -> Outcome {
    let n = Slab::new(1.444, 3.473, 1.444, Length::nm(220.0))
        .ok()
        .and_then(|slab| slab.modes(polarization, um(1.55)).first().copied())
        .map_or(f64::NAN, |m| m.effective_index());
    Outcome {
        measured: n,
        expected,
        // printed to 3 decimals
        tolerance: 5e-4,
        error: (n - expected).abs(),
    }
}

fn slab_te_book() -> Outcome {
    slab_book(Polarization::Te, 2.845)
}

fn slab_tm_book() -> Outcome {
    slab_book(Polarization::Tm, 2.051)
}

fn vector_slab(polarization: Polarization, te_like: bool) -> Outcome {
    let error = crate::mode::vector::slab_limit_error(polarization, te_like, um(1.55), 0.0025);
    Outcome {
        measured: error.abs(),
        expected: 0.0,
        // second order: 2.5 nm leaves about 4e-5 (TE) and 5e-6 (TM)
        tolerance: 1e-4,
        error: error.abs(),
    }
}

fn vector_slab_te() -> Outcome {
    // the slab's TE mode has its magnetic field mostly along x here
    vector_slab(Polarization::Te, false)
}

fn vector_slab_tm() -> Outcome {
    vector_slab(Polarization::Tm, true)
}

fn strip_book() -> Outcome {
    let n = crate::mode::vector::modes(&crate::mode::vector::strip(0.005), um(1.55), 1, None)
        .ok()
        .and_then(|m| m.first().map(|m| m.effective_index().re))
        .unwrap_or(f64::NAN);
    Outcome {
        measured: n,
        expected: 2.443,
        // the book's value is good to about 1e-3; ours at 5 nm is within about 2e-3 of its
        // converged value (the corner singularities)
        tolerance: 3e-3,
        error: (n - 2.443).abs(),
    }
}

/// Hadley's corner problem `problem` at N = 80, against his modal index.
fn hadley(problem: usize) -> Outcome {
    let (cs, expected) = crate::mode::vector::hadley_problem(problem, 80);
    let n = crate::mode::vector::modes(&cs, um(1.5), 1, None)
        .ok()
        .and_then(|m| m.first().map(|m| m.effective_index().re))
        .unwrap_or(f64::NAN);
    Outcome {
        measured: n,
        expected,
        // at N = 80 the errors are 4.9e-5, 5.8e-6, 2.5e-5 and 3.3e-5
        tolerance: 1e-4,
        error: (n - expected).abs(),
    }
}

fn hadley_1() -> Outcome {
    hadley(1)
}

fn hadley_2() -> Outcome {
    hadley(2)
}

fn hadley_3() -> Outcome {
    hadley(3)
}

fn hadley_4() -> Outcome {
    hadley(4)
}

fn slab_group_index() -> Outcome {
    let (differences, exact) = crate::mode::dispersion::te_slab_group_index();
    Outcome {
        measured: (differences - exact).abs(),
        expected: 0.0,
        // the five-point differences' truncation is about 1e-8
        tolerance: 1e-6,
        error: (differences - exact).abs(),
    }
}

fn strip_group_index_book() -> Outcome {
    use crate::material::{LorentzPole, Material, Model, Provenance};
    use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
    use faer::c64;
    // the book's Lorentz silicon (its Eq. 3.2 and Listing 3.1), ω₀ in rad/s as THz
    let silicon = Material::new(
        "Si",
        Model::Lorentz {
            eps_inf: 7.987_374_92,
            poles: vec![LorentzPole {
                strength: 3.687_991_43,
                resonance: crate::units::Frequency::thz(3.932_824_66e15 / TAU / 1e12)
                    .expect("a positive frequency"),
                damping: 0.0,
            }],
        },
        um(1.15),
        um(1.8),
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Eq. 3.2"
                .into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: "Listing 3.1".into(),
            temperature: Some(300.0),
            notes: String::new(),
        },
    )
    .expect("a valid material");
    // the TE-like mode's quarter, behind an electric wall at x = 0 and a magnetic one at y = 0
    let strip = |l: Wavelength| {
        let si = silicon.permittivity(l)?;
        CrossSection::uniform((0.0, 1.05, 168), (0.0, 0.75, 150), |x, y| {
            Permittivity::isotropic(if x < 0.25 && y < 0.11 {
                si
            } else {
                c64::new(1.444 * 1.444, 0.0)
            })
        })?
        .with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            south: Boundary::MagneticWall,
            ..Boundaries::default()
        })
    };
    let wavelengths = [um(1.54), um(1.55), um(1.56)];
    let ng = crate::mode::dispersion::track(strip, &wavelengths, None, 3)
        .ok()
        .and_then(|m| {
            let n: Vec<f64> = m.iter().map(|m| m.effective_index().re).collect();
            crate::mode::dispersion::group_index(&wavelengths, &n).ok()
        })
        .map_or(f64::NAN, |ng| ng[1]);
    Outcome {
        measured: ng,
        expected: 4.18,
        // the plot's reading (±0.005), the book's 20 nm mesh and our corners' convergence
        // (4.1651, 4.1729 at 12.5 and 6.25 nm)
        tolerance: 0.02,
        error: (ng - 4.18).abs(),
    }
}

/// Chilwell and Hodgkinson's Table 3: each bound mode's effective index and the % of its power
/// in the cover, films 1–4 and the substrate; TE0, TE1, TE2, TE3, then TM0 … TM3.
const CHILWELL_TABLE_3: [(f64, [f64; 6]); 8] = [
    (1.622729, [0.0, 0.1, 0.4, 19.3, 76.3, 3.9]),
    (1.605276, [1.0, 87.1, 11.4, 0.2, 0.2, 0.0]),
    (1.557136, [0.0, 1.4, 12.3, 59.0, 20.7, 6.6]),
    (1.503587, [0.3, 6.3, 28.0, 14.7, 18.6, 32.1]),
    (1.620031, [0.0, 0.0, 0.5, 21.4, 74.3, 3.8]),
    (1.594788, [0.5, 83.4, 15.4, 0.4, 0.3, 0.0]),
    (1.554981, [0.0, 2.2, 12.0, 57.3, 21.3, 7.3]),
    (1.501818, [0.1, 4.2, 22.7, 13.0, 15.1, 45.0]),
];

/// The four-layer guide's bound modes, TE then TM, fundamental first.
fn chilwell_bound() -> Vec<crate::mode::multilayer::MultilayerMode> {
    let (stack, w) = crate::mode::multilayer::chilwell_four_layer();
    [Polarization::Te, Polarization::Tm]
        .iter()
        .flat_map(|&p| stack.bound_modes(p, w).unwrap_or_default())
        .collect()
}

fn multilayer_bound() -> Outcome {
    let modes = chilwell_bound();
    let worst = if modes.len() == 8 {
        modes
            .iter()
            .zip(CHILWELL_TABLE_3)
            .map(|(m, (n, _))| (m.effective_index().re - n).abs())
            .fold(0.0, f64::max)
    } else {
        f64::NAN
    };
    Outcome {
        measured: worst,
        expected: 0.0,
        // printed to 6 decimals
        tolerance: 5e-7,
        error: worst,
    }
}

fn multilayer_leaky() -> Outcome {
    use crate::mode::multilayer::Region;
    let printed = [
        (1.46186, 0.00716),
        (1.38250, 0.01817),
        (1.28136, 0.03588),
        (1.14231, 0.05288),
        (1.00304, 0.07077),
    ];
    let (stack, w) = crate::mode::multilayer::chilwell_four_layer();
    let region = Region {
        re_min: 1.0,
        re_max: 1.499,
        im_min: 0.0,
        im_max: 0.1,
    };
    let found = stack.modes_in(Polarization::Te, w, region, 120);
    let worst = if found.len() == 5 {
        found
            .iter()
            .zip(printed)
            .map(|(m, (a, b))| {
                let n = m.effective_index();
                (n.re - a).abs().max((n.im - b).abs())
            })
            .fold(0.0, f64::max)
    } else {
        f64::NAN
    };
    Outcome {
        measured: worst,
        expected: 0.0,
        // half a unit in the 5th decimal, plus the one value a unit off (see the source)
        tolerance: 1.5e-5,
        error: worst,
    }
}

fn multilayer_power() -> Outcome {
    let modes = chilwell_bound();
    let worst = if modes.len() == 8 {
        modes
            .iter()
            .zip(CHILWELL_TABLE_3)
            .flat_map(|(m, (_, shares))| {
                m.power_fractions()
                    .into_iter()
                    .zip(shares)
                    .map(|(got, p)| (100.0 * got - p).abs())
                    .collect::<Vec<_>>()
            })
            .fold(0.0, f64::max)
    } else {
        f64::NAN
    };
    Outcome {
        measured: worst,
        expected: 0.0,
        // printed to 0.1 %
        tolerance: 0.06,
        error: worst,
    }
}

fn pml_soi(polarization: Polarization) -> Outcome {
    let (found, exact) = crate::mode::vector::soi_leakage(polarization, 0.5, 0.0025);
    let relative = (found.im / exact.im - 1.0).abs();
    Outcome {
        measured: relative,
        expected: 0.0,
        // 0.1 % of the loss: at 2.5 nm the grid leaves 0.04 % (TE) and 0.01 % (TM)
        tolerance: 1e-3,
        error: relative,
    }
}

fn pml_soi_te() -> Outcome {
    pml_soi(Polarization::Te)
}

fn pml_soi_tm() -> Outcome {
    pml_soi(Polarization::Tm)
}

fn pml_leaky_chilwell() -> Outcome {
    let printed = [
        num_complex::Complex64::new(1.46186, 0.00716),
        num_complex::Complex64::new(1.38250, 0.01817),
        num_complex::Complex64::new(1.28136, 0.03588),
        num_complex::Complex64::new(1.14231, 0.05288),
    ];
    let worst = crate::mode::vector::chilwell_leaky(0.0025, &printed)
        .iter()
        .zip(printed)
        .map(|(n, p)| (n.re - p.re).abs().max((n.im - p.im).abs()))
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        // the table's 5 decimals, the grid (3.8e-5 at 2.5 nm for m = 7) and the PML
        tolerance: 5e-5,
        error: worst,
    }
}

fn eim_strip_book() -> Outcome {
    let n =
        crate::mode::eim::Ridge::strip(1.444, 3.473, 1.444, Length::nm(220.0), Length::nm(500.0))
            .and_then(|r| r.mode(Polarization::Te, um(1.55)))
            .map_or(f64::NAN, |m| m.effective_index);
    Outcome {
        measured: n,
        expected: 2.489,
        // 3 printed decimals, plus the book's rounded input (1.9e-4) and mesh
        tolerance: 1e-3,
        error: (n - 2.489).abs(),
    }
}

fn bend_te_index() -> Outcome {
    let (found, exact) = crate::mode::vector::bent_slab(Polarization::Te, 1.0, 0.0025);
    Outcome {
        measured: (found.re - exact.re).abs(),
        expected: 0.0,
        // second order: 2.5 nm leaves 1.4e-5
        tolerance: 3e-5,
        error: (found.re - exact.re).abs(),
    }
}

fn bend_te_loss() -> Outcome {
    let (found, exact) = crate::mode::vector::bent_slab(Polarization::Te, 1.0, 0.0025);
    let relative = (found.im / exact.im - 1.0).abs();
    Outcome {
        measured: relative,
        expected: 0.0,
        tolerance: 2e-3,
        error: relative,
    }
}

fn bend_tm_index() -> Outcome {
    let (found, exact) = crate::mode::vector::bent_slab(Polarization::Tm, 1.0, 0.0025);
    Outcome {
        measured: (found.re - exact.re).abs(),
        expected: 0.0,
        // the approximation's error, 1.3e-3 at 1 um
        tolerance: 2e-3,
        error: (found.re - exact.re).abs(),
    }
}

fn bend_marcuse() -> Outcome {
    use crate::mode::bend::{SlabBend, marcuse_loss};
    let (core, clad, t, r) = (1.6, 1.5, Length::um(1.0), Length::um(120.0));
    let w = um(1.0);
    let straight = Slab::new(clad, core, clad, t)
        .ok()
        .and_then(|s| {
            s.modes(Polarization::Te, w)
                .first()
                .map(|m| m.effective_index())
        })
        .unwrap_or(f64::NAN);
    let exact = SlabBend::new(r, clad, &[(core, t)], clad, Length::um(-0.5))
        .and_then(|b| b.fundamental(Polarization::Te, w))
        .map_or(f64::NAN, |n| n.im);
    let deviation = marcuse_loss(core, clad, t, r, w, straight) / exact - 1.0;
    Outcome {
        measured: deviation.abs(),
        expected: 0.0,
        // an approximation of order d/R: 0.084 here; at larger radii the loss nears round-off
        tolerance: 0.1,
        error: deviation.abs(),
    }
}

/// The leaky wire extrapolated from its three grids, computed once for both cases.
fn bienstman() -> num_complex::Complex64 {
    static LIMIT: std::sync::OnceLock<num_complex::Complex64> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| {
        let v = [0.005, 0.0025, 0.00125].map(crate::mode::vector::bienstman_wire);
        crate::mode::vector::richardson(v).0
    })
}

fn bienstman_re() -> Outcome {
    let n = bienstman().re;
    Outcome {
        measured: n,
        expected: 2.412372,
        // 8e-5 after extrapolation
        tolerance: 2e-4,
        error: (n - 2.412372).abs(),
    }
}

fn bienstman_im() -> Outcome {
    let im = bienstman().im * 1e8;
    Outcome {
        measured: im,
        expected: 2.9135,
        // 2 %: 0.25 % after extrapolation
        tolerance: 0.06,
        error: (im - 2.9135).abs(),
    }
}

fn fields_butt_coupling() -> Outcome {
    let (got, exact) = crate::mode::fields::slab_butt_coupling(0.005);
    Outcome {
        measured: (got - exact).abs(),
        expected: 0.0,
        // 6.8e-5 at 10 nm
        tolerance: 5e-5,
        error: (got - exact).abs(),
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
    #[cfg_attr(
        debug_assertions,
        ignore = "runs the full-vector solves; CI's validation job runs every case in release"
    )]
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
    fn malitsons_table_is_transcribed_as_printed() {
        // the paper's average of absolute residuals for the C-D-G.E. column is 10.5e-6
        let mean = MALITSON_TABLE_I
            .iter()
            .map(|r| f64::from(r.2.abs()))
            .sum::<f64>()
            / MALITSON_TABLE_I.len() as f64;
        assert!((mean - 10.5).abs() < 0.05, "{mean}");
        // wavelengths increase, within the material's range
        assert!(MALITSON_TABLE_I.windows(2).all(|w| w[1].0 > w[0].0));
    }

    #[test]
    fn case_ids_are_unique() {
        let ids: Vec<&str> = cases().iter().map(|c| c.id).collect();
        for (i, id) in ids.iter().enumerate() {
            assert!(!ids[..i].contains(id), "{id}");
        }
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "runs the full-vector solves; CI's validation job runs every case in release"
    )]
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

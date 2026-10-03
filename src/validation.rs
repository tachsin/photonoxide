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

use crate::material::catalogue::checks as catalogue;
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
///
/// Its `title` and `source` are cells of the report's Markdown table: their math is inline TeX
/// between single dollar signs, which GitHub, the studio and the project site render. A cell can't
/// hold a `|` (an absolute value is `\lvert x \rvert`), and GitHub drops the backslash before
/// punctuation, so `\,` and `\{` don't survive (`\thinspace`, `\lbrace` do), nor does a `>` (`\gt`).
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
            title: r"The amplitude of a real signal $\operatorname{Re}(A e^{-i\omega t})$ is recovered with the kernel $e^{+i\omega t}$ (magnitude shown)",
            tier: Tier::Analytic,
            source: r"the $e^{-i\omega t}$ convention: $\frac{2}{T}\int_0^T \operatorname{Re}(A e^{-i\omega t})\thinspace e^{i\omega t}\thinspace dt = A$ over whole periods",
            run: amplitude_convention,
        },
        Case {
            id: "units/lossy-attenuation",
            title: r"A wave in a medium with $\operatorname{Im}\varepsilon \gt 0$ decays over one wavelength by $\exp(-2\pi\kappa)$ (decay shown)",
            tier: Tier::Analytic,
            source: r"$e^{i n k_0 x}$ with $n = n' + i\kappa$, $\kappa \geq 0$",
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
            id: "material/silica-leviton-table",
            title: r"Fused silica's $n(\lambda, T)$ (Corning 7980) from the authors' Table 3 reproduces their Table 4: 16 wavelengths from 0.4 to 2.6 µm at 13 temperatures from 30 to 300 K (largest deviation shown)",
            tier: Tier::Published,
            source: "D. B. Leviton, B. J. Frey, Proc. SPIE 6273, 62732K (2006), doi:10.1117/12.672853, Table 4, printed to 5 decimals",
            run: catalogue::leviton_table,
        },
        Case {
            id: "material/linbo3-zelmon-633",
            title: r"Congruent lithium niobate (Zelmon, Table 1) at 633 nm against the $n_o = 2.2864$ and $n_e = 2.2022$ at which Jazbinšek and Zgonik give its tensors (larger deviation shown)",
            tier: Tier::Published,
            source: "M. Jazbinšek, M. Zgonik, Appl. Phys. B 74, 407 (2002), doi:10.1007/s003400200818, Table 5's caption (their ref. 27); Zelmon's fit is within 2e-4 of its data",
            run: catalogue::zelmon_633,
        },
        Case {
            id: "material/linbo3-zelmon-opo",
            title: r"Congruent lithium niobate (Zelmon, $n_e$): the idler of a 1.064 µm-pumped PPLN OPO with a 30 µm grating, from $n_p/\lambda_p - n_s/\lambda_s - n_i/\lambda_i = 1/\Lambda$ (µm, shown)",
            tier: Tier::Published,
            source: "D. E. Zelmon, D. L. Small, D. Jundt, J. Opt. Soc. Am. B 14, 3319 (1997), doi:10.1364/JOSAB.14.003319, Fig. 3, the predicted points: 3.35 µm, read off the plot to 0.03",
            run: catalogue::zelmon_opo,
        },
        Case {
            id: "material/linbo3-jundt-opo",
            title: r"Congruent lithium niobate (Jundt, $n_e(\lambda, T)$): the idler of a 1.064 µm-pumped PPLN OPO at 250 °C with a 25.5 µm grating, expanded by Jundt's Eq. (3) (µm, shown)",
            tier: Tier::Published,
            source: "D. H. Jundt, Opt. Lett. 22, 1553 (1997), doi:10.1364/OL.22.001553, Fig. 1, the 250 °C fit at 25.5 µm: 4.755 µm, read off the plot to 0.02",
            run: catalogue::jundt_opo,
        },
        Case {
            id: "material/linbo3-shoji-miller",
            title: r"Congruent lithium niobate: Miller's $\Delta_{33} = d_{33}/[(n_e^2(2\omega) - 1)(n_e^2(\omega) - 1)^2]$ from Shoji's $d_{33}$ and Zelmon's $n_e$ at the fundamentals 1.313, 1.064 and 0.852 µm (largest relative deviation shown)",
            tier: Tier::Published,
            source: "I. Shoji et al., J. Opt. Soc. Am. B 14, 2268 (1997), doi:10.1364/JOSAB.14.002268, Tables 10 and 12 (3.92, 4.73 and 4.34, in units of 1e-13 m/V), Eq. (1); the paper's own indices differ from Zelmon's",
            run: catalogue::shoji_miller,
        },
        Case {
            id: "material/linbo3-mgo-gayer-zelmon",
            title: r"5% MgO-doped lithium niobate at 21 °C: Gayer's $n_e$ (0.5 to 3 µm) and $n_o$ (0.5 to 1.62 µm) against Zelmon's Table 2 with its columns exchanged, as Gayer et al. find they must be (largest difference shown)",
            tier: Tier::Published,
            source: r"O. Gayer et al., Appl. Phys. B 91, 343 (2008), doi:10.1007/s00340-008-2998-2, Sec. 4.3.1: within 3.1e-4 ($n_e$) and 2.2e-4 ($n_o$); the tolerance adds Zelmon's own 2e-4",
            run: catalogue::gayer_zelmon,
        },
        Case {
            id: "material/gaas-gehrsitz-gap",
            title: r"The direct gap of GaAs from Gehrsitz et al.'s Eq. (11) against the $E_0^2$ of their Table II at 298, 185 and 103 K (in µm⁻², largest deviation shown)",
            tier: Tier::Published,
            source: "S. Gehrsitz et al., J. Appl. Phys. 87, 7825 (2000), doi:10.1063/1.373462, Table II, GaAs Fit 2, printed to 6 decimals",
            run: catalogue::gehrsitz_gap,
        },
        Case {
            id: "material/gaas-gehrsitz-n-inf",
            title: r"GaAs: $n_\infty^2 = A + C_1/E_1^2 + C_0/E_0^2$ (Eq. (8)) from the temperature forms of Table II against its columns at 298, 185 and 103 K (largest deviation shown)",
            tier: Tier::Published,
            source: "S. Gehrsitz et al., J. Appl. Phys. 87, 7825 (2000), doi:10.1063/1.373462, Table II, GaAs Fit 2; the temperature forms are themselves fits to the three columns",
            run: catalogue::gehrsitz_n_inf,
        },
        Case {
            id: "material/algaas-gehrsitz-samples",
            title: r"AlGaAs: the analytic $n(x, \lambda)$ of Table IV against each of the nine samples' own fits (Table III) from 0.73 to 0.83 µm at 23 °C, in units of the sum of the two fits' $\Delta n_\mathrm{max}$ (largest shown)",
            tier: Tier::Published,
            source: "S. Gehrsitz et al., J. Appl. Phys. 87, 7825 (2000), doi:10.1063/1.373462, Tables III and IV (quality of fit)",
            run: catalogue::gehrsitz_samples,
        },
        Case {
            id: "material/algaas-papatryfonos",
            title: r"AlGaAs (Gehrsitz) against MBE layers measured by ellipsometry, $x$ from 0 to 0.452 at 825, 1300 and 1550 nm: the 17 points below the gap (largest relative deviation shown)",
            tier: Tier::Published,
            source: "K. Papatryfonos et al., AIP Adv. 11, 025327 (2021), doi:10.1063/5.0039631, Table III; the paper reports differences of the order of 1% from the reference models",
            run: catalogue::gehrsitz_papatryfonos,
        },
        Case {
            id: "material/algaas-afromowitz-eta",
            title: r"GaAs in Afromowitz's model: $\eta = \pi E_d / [2E_0^3(E_0^2 - E_\Gamma^2)]$ with $E_0 = 3.65$, $E_d = 36.1$ and $E_\Gamma = 1.424$ eV (shown)",
            tier: Tier::Published,
            source: "M. A. Afromowitz, Solid State Commun. 15, 59 (1974), doi:10.1016/0038-1098(74)90014-3, p. 60: 0.1032",
            run: catalogue::afromowitz_eta,
        },
        Case {
            id: "material/gaas-skauli-shg",
            title: r"GaAs (Skauli, the Pikhtin form) at 21 °C: the first-order QPM periods $\Lambda/m = \lambda_\omega / 2(n_{2\omega} - n_\omega)$ of the seven measured SHG wavelengths inside its range (largest relative deviation shown)",
            tier: Tier::Published,
            source: "T. Skauli et al., J. Appl. Phys. 94, 6447 (2003), doi:10.1063/1.1621740, Table I; a difference of indices is accurate to 0.2% of itself, and 61.2 µm is printed to 0.08%",
            run: catalogue::skauli_shg,
        },
        Case {
            id: "material/gaas-skauli-dndt",
            title: r"GaAs (Skauli, the Pikhtin form): $dn/dT$ at 1.5 µm and 22 °C (in units of 1e-4 per K, shown)",
            tier: Tier::Published,
            source: "T. Skauli et al., J. Appl. Phys. 94, 6447 (2003), doi:10.1063/1.1621740, Sec. V: 2.33e-4 per K, from their fits",
            run: catalogue::skauli_dndt,
        },
        Case {
            id: "material/ingap-tanaka-ueno",
            title: r"InGaP (Tanaka's single oscillator, $E_0 = 3.39$ and $E_d = 28.07$ eV) at 1.579 µm, the index Ueno et al. take from it for their $d_{14}$ (shown)",
            tier: Tier::Published,
            source: "Y. Ueno, V. Ricci, G. I. Stegeman, J. Opt. Soc. Am. B 14, 1428 (1997), doi:10.1364/JOSAB.14.001428, Table 2: 3.12, citing Tanaka et al. 1986",
            run: catalogue::tanaka_ueno,
        },
        Case {
            id: "material/inp-pettit-turner-suzuki",
            title: "InP (Pettit and Turner, 298 K) at 1.064, 1.208, 1.306 and 1.50 µm against the indices Suzuki and Tada list beside their electro-optic coefficients (largest deviation shown)",
            tier: Tier::Published,
            source: "N. Suzuki, K. Tada, Jpn. J. Appl. Phys. 23, 291 (1984), doi:10.1143/JJAP.23.291, Table I (3.29, 3.23, 3.20, 3.17); Pettit and Turner's fit is within 0.007 of their data",
            run: catalogue::pettit_turner_suzuki,
        },
        Case {
            id: "material/inp-suzuki-tada-voltages",
            title: r"InP: the half-wave voltages $V_{\lambda/2} = \lambda_0 / 2n_0^3 r_{41}^T$ from the catalogue's $r_{41}^T$ against those Suzuki and Tada print, at four wavelengths (largest relative deviation shown)",
            tier: Tier::Published,
            source: "N. Suzuki, K. Tada, Jpn. J. Appl. Phys. 23, 291 (1984), doi:10.1143/JJAP.23.291, Table I (11.4, 12.0, 12.9 and 14.4 kV); its indices and coefficients have three digits",
            run: catalogue::suzuki_tada_voltages,
        },
        Case {
            id: "material/aln-majkic-d33",
            title: r"AlN: $d_{33}$ from the measured ratio $0.169\thinspace d_{33}(\mathrm{LiNbO_3})$ and the catalogue's $d_{33}$ of congruent lithium niobate at 1.064 µm (pm/V, shown)",
            tier: Tier::Published,
            source: "A. Majkić et al., Phys. Status Solidi B 254, 1700077 (2017), doi:10.1002/pssb.201700077, p. 4 and Table 1: 4.3 pm/V at 1030 nm, from Shoji et al.'s 25.2 pm/V",
            run: catalogue::majkic_d33,
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
            title: r"The TE-like mode of a $500 \times 220$ nm silicon strip in oxide at 1550 nm, at a 5 nm mesh (effective index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Fig. 3.14: 2.443 (Lumerical MODE, 20 nm conformal mesh, accurate to about 1e-3 by its Fig. 3.9); ours converges at about first order at the convex corners (as on Hadley's corner problems below), 2.4435 at 2.5 nm",
            run: strip_book,
        },
        Case {
            id: "mode/hadley-box-low",
            title: r"Hadley's corner problem 1: a box, $\varepsilon = 2.25$ in a quarter of the $1 \times 1$ µm domain, at 1.5 µm, on an $80 \times 80$ grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 4: $1.27627404 \pm 10^{-8}$ (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_1,
        },
        Case {
            id: "mode/hadley-box-high",
            title: r"Hadley's corner problem 2: a box, $\varepsilon = 8$ in a quarter of the $1 \times 1$ µm domain, at 1.5 µm, on an $80 \times 80$ grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 5: $2.65679692 \pm 10^{-8}$ (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_2,
        },
        Case {
            id: "mode/hadley-corner-low",
            title: r"Hadley's corner problem 3: an impinged corner, $\varepsilon = 2.25$ in three quarters of the $1 \times 1$ µm domain, at 1.5 µm, on an $80 \times 80$ grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 6: $1.387926425 \pm 2 \times 10^{-9}$ (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_3,
        },
        Case {
            id: "mode/hadley-corner-high",
            title: r"Hadley's corner problem 4: an impinged corner, $\varepsilon = 8$ in three quarters of the $1 \times 1$ µm domain, at 1.5 µm, on an $80 \times 80$ grid (12.5 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 7: $2.761465320 \pm 5 \times 10^{-9}$ (series expansion); ours converges at about first order at convex corners and 1.4-1.8 at concave ones",
            run: hadley_4,
        },
        Case {
            id: "mode/slab-group-index",
            title: r"The group index of a TE slab (220 nm of 3.473 between 1.444 and air) at 1.55 µm, from differences of its effective index over $\pm 2$ nm (error shown)",
            tier: Tier::Analytic,
            source: r"Hellmann-Feynman, no material dispersion: $n_g = \langle\varepsilon\rangle/n_\text{eff}$, $\langle\varepsilon\rangle$ weighted by $E^2$ of the exact field",
            run: slab_group_index,
        },
        Case {
            id: "mode/strip-group-index-book",
            title: r"The group index of a $500 \times 220$ nm strip at 1.55 µm, with the book's dispersive silicon and 1.444 oxide, on a $6.25 \times 5$ nm grid (group index shown)",
            tier: Tier::Published,
            source: r"L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Fig. 3.22b: about 4.18, read off the plot to $\pm 0.005$ (Lumerical MODE, 20 nm mesh); materials from its Listing 3.1",
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
            title: r"The 5 TE leaky waves ($m = 4, \ldots, 8$) of the same guide, complex effective indices, exact (largest deviation of a real or imaginary part shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 2: to 5 decimals; $m = 5$'s real part, 1.38250, is ours (1.3824892) plus 1.1e-5, one unit in the last place, the other nine our values rounded",
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
            id: "mode/multilayer-fresnel",
            title: "A plane wave's reflection coefficient at one interface, 1.0 to 1.5, TE and TM at 0, 20, 45 and 70 degrees, by the transfer matrices (largest difference in $r$ shown)",
            tier: Tier::Analytic,
            source: r"Fresnel's equations: $r_s = (n_1\cos\theta_1 - n_2\cos\theta_2)/(n_1\cos\theta_1 + n_2\cos\theta_2)$, $r_p = (n_2\cos\theta_1 - n_1\cos\theta_2)/(n_2\cos\theta_1 + n_1\cos\theta_2)$; J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, say Eq. 13 reduces to them",
            run: multilayer_fresnel,
        },
        Case {
            id: "mode/multilayer-bragg",
            title: "The reflectance of 8 quarter-wave pairs, 2.3 / 1.38 on 1.52 at 550 nm, normal incidence, by the transfer matrices (shown)",
            tier: Tier::Analytic,
            source: r"the quarter-wave stack's closed form, $R = \left(\frac{1 - q}{1 + q}\right)^2$ with $q = (n_s/n_0)(n_H/n_L)^{2N}$, from its admittance",
            run: multilayer_bragg,
        },
        Case {
            id: "mode/pml-soi-leakage-te",
            title: r"The loss of 220 nm SOI's TE mode leaking through 0.5 µm of buried oxide into the substrate, full-vector with a PML (1 µm, strength 3), 2.5 nm grid (relative error in $\operatorname{Im} n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: "the exact leaky mode of the same stack by transfer matrices (mode::multilayer); PML by complex coordinate stretching, W. C. Chew et al., Microw. Opt. Technol. Lett. 15, 363 (1997)",
            run: pml_soi_te,
        },
        Case {
            id: "mode/pml-soi-leakage-tm",
            title: r"The same for the TM mode (relative error in $\operatorname{Im} n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: "the exact leaky mode of the same stack by transfer matrices (mode::multilayer); PML by complex coordinate stretching, W. C. Chew et al., Microw. Opt. Technol. Lett. 15, 363 (1997)",
            run: pml_soi_tm,
        },
        Case {
            id: "mode/pml-leaky-chilwell",
            title: r"Chilwell and Hodgkinson's TE leaky waves $m = 4, \ldots, 7$, full-vector with a PML (2 µm, strength 5) in the substrate, 2.5 nm grid (largest deviation of a real or imaginary part shown)",
            tier: Tier::Published,
            source: r"J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 2, to 5 decimals; $m = 8$ ($\operatorname{Re} n_\text{eff} = 1.00304$, just above the cover's 1.0) has a slowly decaying, inward-phased field in the cover and is checked only in the leaky_waves example, within 2e-4",
            run: pml_leaky_chilwell,
        },
        Case {
            id: "mode/eim-strip-book",
            title: r"The effective index method on a $500 \times 220$ nm silicon strip (3.473 in 1.444) at 1550 nm, TE-like, exact slabs (effective index shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Section 3.2.5: 2.489, from the slab index rounded to 2.845 and a 10 nm 1D mesh (on that input the exact lateral slab gives 2.488558); the method: G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), doi:10.1364/AO.16.000113",
            run: eim_strip_book,
        },
        Case {
            id: "mode/bend-slab-te",
            title: "A slab (2.845, 500 nm, in 1.444) bent at 1 µm, $E$ normal to the bend plane, full-vector on a conformally mapped 2.5 nm grid with a PML: effective index along the arc (error shown)",
            tier: Tier::Analytic,
            source: "the exact bent slab (mode::bend: radial shooting matched to the outgoing Hankel function, D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x, Eq. 10); the map: M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975), doi:10.1109/JQE.1975.1068563, exact for this polarization; second order",
            run: bend_te_index,
        },
        Case {
            id: "mode/bend-slab-te-loss",
            title: r"The same bend's radiation loss, $\operatorname{Im} n_\text{eff} = 9.29 \times 10^{-4}$ (relative error shown)",
            tier: Tier::Analytic,
            source: "the exact bent slab (mode::bend); the PML starts at 2.5 µm, outside the bend's turning point",
            run: bend_te_loss,
        },
        Case {
            id: "mode/bend-slab-tm",
            title: "The same bend with $E$ in the bend plane, where scaling an isotropic permittivity is an approximation (error shown)",
            tier: Tier::Analytic,
            source: "the exact bent slab (mode::bend); the exact equivalent medium would be anisotropic in both permittivity and permeability; the error falls as the radius grows (1.3e-4 at 3 µm)",
            run: bend_tm_index,
        },
        Case {
            id: "mode/bend-marcuse",
            title: "Marcuse's bending-loss formula against the exact loss of a slab (1.6 in 1.5, 1 µm, at 1 µm) bent at 120 µm (ratio minus one shown)",
            tier: Tier::Published,
            source: "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x, Eqs. 32-33, an approximation for large radii: its deviation falls as $1/R$, 0.14 at 80 µm, 0.084 at 120 µm and 0.061 at 160 µm",
            run: bend_marcuse,
        },
        Case {
            id: "mode/leaky-wire-bienstman",
            title: r"The leaky SOI wire benchmark ($500 \times 220$ nm Si 3.5 on 1 µm SiO₂ 1.45 on Si, air above, 1.55 µm), TE: $\operatorname{Re} n_\text{eff}$, Richardson-extrapolated from core grids of 5, 2.5 and 1.25 nm (order $\approx 0.67$, the corners'), with a PML in the substrate",
            tier: Tier::Published,
            source: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), doi:10.1007/s11082-006-9025-9, Table 6: 2.412372, from CAMFR and the aperiodic Fourier modal method (7 digits); raw errors +4.0e-3, +2.5e-3, +1.5e-3",
            run: bienstman_re,
        },
        Case {
            id: "mode/leaky-wire-bienstman-loss",
            title: r"The same wire's substrate leakage, $\operatorname{Im} n_\text{eff} \times 10^8$, extrapolated alike",
            tier: Tier::Published,
            source: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), doi:10.1007/s11082-006-9025-9, Table 6: 2.9135 (CAMFR) and 2.91348 (aperiodic Fourier modal method); the raw results are 0.97, 0.98 and 0.99 of it",
            run: bienstman_im,
        },
        Case {
            id: "mode/fields-butt-coupling",
            title: "The power a 220 nm silicon slab's TE mode launches into a 300 nm slab's (3.473 in 1.444, 1.55 µm), from the full-vector fields on a 5 nm grid (error shown)",
            tier: Tier::Analytic,
            source: r"the exact slab fields (mode::slab): for TE slabs $H$ is proportional to $E$, so the coupling is $(\int E_1 E_2)^2 / (\int E_1^2 \int E_2^2) = 0.994662$",
            run: fields_butt_coupling,
        },
        Case {
            id: "mode/marcatili-closed-form",
            title: r"Marcatili's closed-form approximation against his transcendental equations, $E^x_{11}$ and $E^y_{11}$ of his guide $a = 2b$, $n_1/n_4 = 1.05$, where $(k_z^2 - k_4^2)/(k_1^2 - k_4^2) \geq 0.5$ (largest relative difference shown)",
            tier: Tier::Published,
            source: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969), doi:10.1002/j.1538-7305.1969.tb01166.x, p. 2083: 'within a few percent of the exact value' there; 4.1 % here",
            run: marcatili_closed_form,
        },
        Case {
            id: "mode/marcatili-vector",
            title: r"Marcatili's approximation (his transcendental equations) against the full-vector solver, $E^x_{11}$ of his guide $a = 2b$, $n_1/n_4 = 1.05$, at $(2b/\lambda)(n_1^2 - n_4^2)^{1/2} = 3$, far from cutoff (difference in the normalized constant shown)",
            tier: Tier::Published,
            source: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969), doi:10.1002/j.1538-7305.1969.tb01166.x, Eqs. 3, 6-7, 20-21; Fig. 6b's regime: 1e-4 apart at $B = 3$ and 4, 1.2e-3 at 1.5, 9e-3 at 1 near cutoff, where the corners Marcatili ignores hold field",
            run: marcatili_vector,
        },
        Case {
            id: "mode/slab-fd-chilwell",
            title: "The 8 bound modes of Chilwell and Hodgkinson's four-layer guide by 1D finite differences on a 1 nm grid (largest deviation shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 3: effective indices to 6 decimals; the scheme is second order (tested against the exact slab)",
            run: slab_fd_chilwell,
        },
        Case {
            id: "fdfd/slab-reflection-ez",
            title: "2D FDFD, $E$ along $z$: the reflectance of 220 nm of silicon (3.476) on oxide (1.444) under air, 30 degrees, 1.55 µm, from the fluxes on a 2.5 nm grid (shown)",
            tier: Tier::Analytic,
            source: "the exact stack by transfer matrices (mode::multilayer, J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Eqs. 13-16, TE); second order: 2.1e-3, 5.5e-4, 1.4e-4, 3.5e-5 at 20, 10, 5, 2.5 nm",
            run: fdfd_slab_ez,
        },
        Case {
            id: "fdfd/slab-reflection-hz",
            title: "The same with $H$ along $z$ (shown)",
            tier: Tier::Analytic,
            source: "the exact stack by transfer matrices (TM); second order: 1.9e-3, 4.9e-4, 1.2e-4, 3.1e-5 at 20, 10, 5, 2.5 nm",
            run: fdfd_slab_hz,
        },
        Case {
            id: "fdfd/flux-conservation",
            title: "2D FDFD: the power through every row from the oxide through the silicon into the air, both polarizations at 0, 30 and 60 degrees, 10 nm grid (largest relative spread shown)",
            tier: Tier::Analytic,
            source: "Poynting's theorem: no power is lost or made in a lossless region without sources; the scheme's own flux keeps this exactly",
            run: fdfd_flux_conservation,
        },
        Case {
            id: "fdfd/pml-reflection",
            title: "2D FDFD: what a 20-cell PML graded to $R = 10^{-8}$ ($m = 3$) sends back of a plane wave 17 degrees off its normal, in oxide on a 20 nm grid, both polarizations (largest amplitude shown)",
            tier: Tier::Analytic,
            source: "W. Shin, S. Fan, J. Comput. Phys. 231, 3406 (2012), doi:10.1016/j.jcp.2012.01.013, Eqs. 2.7-2.9: graded for $R = 10^{-8}$ in vacuum at normal incidence; in oxide 17 degrees off, the round trip absorbs to $(10^{-8})^{1.38}$, an amplitude of 3e-6; measured 2.5e-6",
            run: fdfd_pml_reflection,
        },
        Case {
            id: "fdfd/port-mode-te",
            title: "2D FDFD ports: the fundamental mode of a 220 nm silicon slab (3.476 in 1.444) at 1.55 µm, $E$ along $z$, solved on a port column of a 2.5 nm grid: effective index (shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab); the port's 1D operator is the 2D scheme's own, second order: 2.6e-3, 6.5e-4, 1.6e-4 at 10, 5, 2.5 nm",
            run: fdfd_port_mode_te,
        },
        Case {
            id: "fdfd/port-mode-tm",
            title: "The same with $H$ along $z$ (shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab); second order: 2.5e-3, 6.1e-4, 1.5e-4 at 10, 5, 2.5 nm",
            run: fdfd_port_mode_tm,
        },
        Case {
            id: "fdfd/straight-guide",
            title: r"2D FDFD ports: a straight silicon slab between two ports 1.4 µm apart, both polarizations, 20 nm grid: largest of the magnitudes of $S_{11}$ and $S_{22}$ and of the errors of $S_{21}$ and $S_{12}$ against $\exp(i\beta L)$ (shown)",
            tier: Tier::Analytic,
            source: r"a uniform guide transmits its mode whole with phase $\beta L$; the port modes are the grid's own and the source is total-field/scattered-field (R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012), doi:10.2528/PIERB11092006, Eq. 55)",
            run: fdfd_straight_guide,
        },
        Case {
            id: "fdfd/reciprocity",
            title: "2D FDFD ports: a slab stepping from 220 to 300 nm, both polarizations, 10 nm grid: $S_{21}$ against $S_{12}$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "Lorentz reciprocity: $S$ is symmetric for a reciprocal device; the scheme keeps it with the PMLs' stretches as weights and the modes normalized by the unconjugated Lorentz form",
            run: fdfd_reciprocity,
        },
        Case {
            id: "fdfd/step-reflection-te",
            title: "The same step's reflection of the 220 nm slab's TE mode, $E$ along $z$ (shown)",
            tier: Tier::Analytic,
            source: r"Fresnel's formula on the two modes' effective indices, $((n_1 - n_2)/(n_1 + n_2))^2 = 1.16503 \times 10^{-3}$ for 2.84742 and 3.04866: the TE modal impedance is the effective index; an approximation, good here to 0.07 %",
            run: fdfd_step_reflection_te,
        },
        Case {
            id: "fdfd/adjoint-gradient-ez",
            title: r"2D FDFD, $E$ along $z$: the adjoint gradient of the power a silicon slab with a bump beside it delivers into its right port's mode, against fourth-order central finite differences ($\delta = 10^{-3}$) on a cell each in the bump, the core and the oxide (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the adjoint variable method, G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2-4: $\nabla F = -2\operatorname{Re}(\lambda^T \mathrm{d}A\thinspace u)$, $A^T\lambda = \mathrm{d}F/\mathrm{d}u$; the finite differences' own round-off is about $10^{-10}/\delta$",
            run: fdfd_adjoint_ez,
        },
        Case {
            id: "fdfd/adjoint-gradient-hz",
            title: r"The same with $H$ along $z$, where the permittivity enters through the faces' $1/\varepsilon$ (shown)",
            tier: Tier::Analytic,
            source: "the adjoint variable method, as above; the faces' permittivity the mean of their two cells",
            run: fdfd_adjoint_hz,
        },
        Case {
            id: "mode/hadley-uniform-box",
            title: r"Hadley's high-accuracy equations in a uniform region: the box of Hadley I, Fig. 5 ($n = 3.44$, $2 \times 2$ µm, 1.15 µm) on an $8 \times 8$ grid (250 nm; effective index shown)",
            tier: Tier::Analytic,
            source: r"exact, $\sqrt{\varepsilon - ((\pi/4)^2 + (\pi/2)^2)/k^2}$ for $H_y = \cos(\pi x/4) \sin(\pi y/2)$; the equations are G. R. Hadley, J. Lightwave Technol. 20, 1210 (2002), doi:10.1109/JLT.2002.800361, Eqs. 7-9; the standard scheme's error on this grid is 1.6e-4",
            run: hadley_uniform_box,
        },
        Case {
            id: "mode/hadley-uniform-order",
            title: r"The same box: the order of convergence of the effective index from $4 \times 4$ to $8 \times 8$ grids (shown to two decimals)",
            tier: Tier::Analytic,
            source: r"sixth order: Hadley I, Fig. 5, slope 6.03; errors 1.7e-8 and 2.4e-10 here (6.03 from $8 \times 8$ to $16 \times 16$, where 3.7e-12 nears round-off)",
            run: hadley_uniform_order,
        },
        Case {
            id: "mode/hadley-interface",
            title: r"Hadley's interface equations: the two-dielectric box of Hadley I, Fig. 6 ($\varepsilon = 1$ over $\varepsilon = 11.8336$, 1.5 µm wide, 0.975 µm) on a 31.25 nm grid (effective index shown)",
            tier: Tier::Analytic,
            source: r"exact: separable, $H_y = \sin(\pi x/W)\thinspace Y(y)$ with $Y$ and $Y'$ continuous, so $k_b \tan(k_b L_b) + k_t \tan(k_t L_t) = 0$ with $k^2 = k_0^2 (\varepsilon - n_\text{eff}^2) - (\pi/W)^2$; the equations are Hadley I, Eqs. 20-24 and 43; the standard scheme's error on this grid is 5.4e-6",
            run: hadley_interface,
        },
        Case {
            id: "mode/hadley-interface-order",
            title: "The same box: the order of convergence of the effective index from 62.5 to 31.25 nm grids (shown to two decimals)",
            tier: Tier::Analytic,
            source: "sixth order: Hadley I, Fig. 7 (fifth-order interface equations, diluted by a line of interface nodes); errors 9.7e-8 and 1.6e-9 here",
            run: hadley_interface_order,
        },
        Case {
            id: "mode/hadley-interface-turned",
            title: "The same box turned on its side, so that $H_x$ is the component normal to the interface, at 62.5 nm (difference of the effective indices shown)",
            tier: Tier::Analytic,
            source: "symmetry: Hadley derives the equations for a horizontal interface; a vertical one is the same with $x$ and $y$ exchanged",
            run: hadley_interface_turned,
        },
        Case {
            id: "mode/hadley-corners-box-low",
            title: r"Hadley's corner problem 1 (a box, $\varepsilon = 2.25$) by his high-accuracy equations on a $128 \times 128$ grid (7.8 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Fig. 4: $1.27627404 \pm 10^{-8}$ (series expansion); the corner equations are its Eqs. 50 and 52, their misprinted $\theta \sin\theta$ read as $\theta \sin 2\theta$ from Eq. 47; the standard scheme's error on this grid is 3.8e-5",
            run: hadley_corners_1,
        },
        Case {
            id: "mode/hadley-corners-box-high",
            title: r"Hadley's corner problem 2 (a box, $\varepsilon = 8$) by his high-accuracy equations on a $128 \times 128$ grid (7.8 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley (2002), part II, Fig. 5: $2.65679692 \pm 10^{-8}$; the standard scheme's error on this grid is 9.0e-6",
            run: hadley_corners_2,
        },
        Case {
            id: "mode/hadley-corners-impinged-low",
            title: r"Hadley's corner problem 3 (an impinged corner, $\varepsilon = 2.25$) by his high-accuracy equations on a $128 \times 128$ grid (7.8 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley (2002), part II, Fig. 6: $1.387926425 \pm 2 \times 10^{-9}$; the standard scheme's error on this grid is 1.4e-5",
            run: hadley_corners_3,
        },
        Case {
            id: "mode/hadley-corners-impinged-high",
            title: r"Hadley's corner problem 4 (an impinged corner, $\varepsilon = 8$) by his high-accuracy equations on a $128 \times 128$ grid (7.8 nm; effective index shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley (2002), part II, Fig. 7: $2.761465320 \pm 5 \times 10^{-9}$; the standard scheme's error on this grid is 1.6e-5",
            run: hadley_corners_4,
        },
        Case {
            id: "mode/hadley-corners-order",
            title: r"Hadley's corner problem 1 by his equations: the order of convergence of the effective index from $32 \times 32$ to $128 \times 128$ grids (shown to two decimals)",
            tier: Tier::Published,
            source: "G. R. Hadley (2002), part II, Section IV: second order for most cases (Figs. 8-11), where the standard scheme's is about first; errors 9.1e-6, 2.1e-6, 5.2e-7 here",
            run: hadley_corners_order,
        },
        Case {
            id: "fdfd3d/film-reflection-te",
            title: "3D FDFD, s (TE) polarized: the reflectance of 220 nm of silicon (3.476) on oxide (1.444) under air, 30 degrees from the normal in a plane 30 degrees from $x$, 1.55 µm, from the fluxes on a 2.5 nm grid (shown)",
            tier: Tier::Analytic,
            source: "the exact stack by transfer matrices (mode::multilayer, J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Eqs. 13-16, TE); second order: 2.8e-3, 7.3e-4, 1.9e-4, 4.6e-5 at 20, 10, 5, 2.5 nm",
            run: fdfd3d_film_te,
        },
        Case {
            id: "fdfd3d/film-reflection-tm",
            title: "The same, p (TM) polarized (shown)",
            tier: Tier::Analytic,
            source: "the exact stack by transfer matrices (TM); second order: 1.9e-3, 4.9e-4, 1.2e-4, 3.1e-5 at 20, 10, 5, 2.5 nm",
            run: fdfd3d_film_tm,
        },
        Case {
            id: "fdfd3d/flux-conservation",
            title: "3D FDFD: the power through every plane from the oxide through the silicon into the air, both polarizations at 0, 30 and 60 degrees, 10 nm grid (largest relative spread shown)",
            tier: Tier::Analytic,
            source: "Poynting's theorem: no power is lost or made in a lossless region without sources; the scheme's own flux (tangential $E$ averaged across the plane, $H$ on it) keeps this exactly",
            run: fdfd3d_flux_conservation,
        },
        Case {
            id: "fdfd3d/pml-reflection",
            title: "3D FDFD: what a 20-cell PML graded to $R = 10^{-8}$ ($m = 3$) sends back of a plane wave 17 degrees off its normal in a plane 30 degrees from $x$, in oxide on a 20 nm grid, both polarizations (largest amplitude shown)",
            tier: Tier::Analytic,
            source: "W. Shin, S. Fan, J. Comput. Phys. 231, 3406 (2012), doi:10.1016/j.jcp.2012.01.013, Eqs. 2.5-2.9: graded for $R = 10^{-8}$ in vacuum at normal incidence; in oxide 17 degrees off, the round trip absorbs to $(10^{-8})^{1.38}$, an amplitude of 3e-6; measured 2.5e-6, as in 2D",
            run: fdfd3d_pml_reflection,
        },
        Case {
            id: "fdfd3d/two-d-agreement",
            title: "3D FDFD on a structure invariant along $z$ (a silicon rod in lossy oxide, Bloch-periodic in $x$ and $y$, 25 nm grid, one cell along $z$) against the 2D solver, $E$ along $z$ and $H$ along $z$ (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"with $\partial/\partial z = 0$ Maxwell's equations split into the two 2D polarizations (K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966), doi:10.1109/TAP.1966.1138693); on the same grid and averaging the two discrete systems are the same equations, one eliminating $H$ and the other $E$",
            run: fdfd3d_two_d_agreement,
        },
        Case {
            id: "fdfd3d/qmr-direct",
            title: "3D FDFD by QMR, on the curl-curl operator and on Shin and Fan's ($s = -1$), to a relative residual of 1e-10, against the sparse direct solver: a silicon strip in oxide, $16^3$ cells of 40 nm, PMLs all round (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: "the same system solved two ways: QMR, R. W. Freund, N. M. Nachtigal, Numer. Math. 60, 315 (1991), doi:10.1007/BF01385726, Algorithm 3.1 without look-ahead; Shin and Fan's operator, Opt. Express 21, 22578 (2013), doi:10.1364/OE.21.022578, Eq. 7, has the same solution; measured 1.1e-11 and 1.3e-10",
            run: fdfd3d_qmr_direct,
        },
        Case {
            id: "fdfd3d/qmr-plateau",
            title: r"Shin and Fan's vacuum square (their Fig. 1: $50 \times 50$ cells of 2 nm, periodic, uniform along $z$, an $x$-polarized dipole at its centre, 1.55 µm), QMR on the curl-curl operator ($s = 0$): the relative residual where it stagnates, at iteration 20 (shown)",
            tier: Tier::Published,
            source: "W. Shin, S. Fan, Opt. Express 21, 22578 (2013), doi:10.1364/OE.21.022578, Section 3 and Fig. 3: the residual's part in the near-null eigenspace, 0.707, holds the residual there initially (GMRES; QMR is GMRES for this real symmetric matrix)",
            run: fdfd3d_qmr_plateau,
        },
        Case {
            id: "fdfd3d/qmr-iterations-curl-curl",
            title: "The same square, $s = 0$: QMR iterations to a relative residual of 1e-6 (shown)",
            tier: Tier::Published,
            source: r"W. Shin, S. Fan, Opt. Express 21, 22578 (2013), doi:10.1364/OE.21.022578, Fig. 3: the $s = 0$ curve crosses 1e-6 at about $m = 114$, read off the plot to $\pm 5$",
            run: fdfd3d_qmr_iterations_curl_curl,
        },
        Case {
            id: "fdfd3d/qmr-iterations-shin-fan",
            title: "The same square, $s = -1$: QMR iterations to a relative residual of 1e-6 (shown)",
            tier: Tier::Published,
            source: r"W. Shin, S. Fan, Opt. Express 21, 22578 (2013), doi:10.1364/OE.21.022578, Fig. 3: the $s = -1$ curve crosses 1e-6 at about $m = 77$, read off the plot to $\pm 5$",
            run: fdfd3d_qmr_iterations_shin_fan,
        },
        Case {
            id: "circuit/series-waveguides",
            title: r"Circuits: two waveguides, 12.5 and 30.25 µm, in series are one of 42.75 µm ($n_\text{eff} = 2.4$, $n_g = 4.2$, 3 dB/cm), 1.54 to 1.56 µm (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"$e^{i\phi_1} e^{i\phi_2} = e^{i(\phi_1 + \phi_2)}$: a connection joins two reference planes with nothing between them; round-off of the 400 rad phases, about $10^{-13}$",
            run: circuit_series,
        },
        Case {
            id: "circuit/mzi-closed-form",
            title: r"Circuits: a Mach-Zehnder interferometer of two lossless couplers ($\kappa^2$ = 0.5 and 0.3) and arms of 100 and 120 µm, 1.54 to 1.56 µm: its four transmissions (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"the product of the transfer matrices, $C \operatorname{diag}(t_1, t_2)\thinspace C$ with $C_{11} = C_{22} = r$ and $C_{12} = C_{21} = i\kappa$, $r^2 + \kappa^2 = 1$",
            run: circuit_mzi,
        },
        Case {
            id: "circuit/ring-all-pass-bogaerts",
            title: r"Circuits: an all-pass ring, a coupler ($\kappa^2$ = 0.1 and 0.02) with one output fed back through 62.8 µm of waveguide, 1.54 to 1.56 µm: the through field (largest error shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eq. 1: $e^{i(\pi + \phi)} (a - r e^{-i\phi}) / (1 - r a e^{i\phi})$, and its square, Eq. 2",
            run: circuit_ring_all_pass,
        },
        Case {
            id: "circuit/ring-add-drop-bogaerts",
            title: r"Circuits: an add-drop ring, two couplers ($\kappa^2$ = 0.1 and 0.05) joined by two halves of a 62.8 µm ring, 1.54 to 1.56 µm: the through and drop powers (largest error shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eqs. 5 and 6",
            run: circuit_ring_add_drop,
        },
        Case {
            id: "circuit/ring-fsr-bogaerts",
            title: r"Circuits: the all-pass ring ($\kappa^2 = 0.1$, 62.8 µm, $n_g = 4.2$): the spacing of its two resonances either side of 1.55 µm, found as minima of the through power (nm, shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eq. 9: $\lambda^2 / (n_g L)$ at the resonances' midpoint; first order, its error here $(\Delta\lambda / 2\lambda)^2$ relative, 8e-5 nm",
            run: circuit_ring_fsr,
        },
        Case {
            id: "circuit/sub-network-growth",
            title: r"Circuits: the sparse solve against sub-network growth on a netlist of 11 instances (two nested rings and an MZI, three reflective multiports, loops), 6 external ports, 1.54 to 1.56 µm (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"G. Filipsson, 11th European Microwave Conference, 700 (1981), doi:10.1109/EUMA.1981.332972, Eq. 6, one connection at a time: the same S to round-off",
            run: circuit_growth,
        },
        Case {
            id: "circuit/reciprocity",
            title: r"Circuits: the same netlist, reciprocal parts: $S$ against $S^\mathsf{T}$ (largest $\lvert S_{qp} - S_{pq} \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"a netlist of reciprocal components is reciprocal, $S = S^\mathsf{T}$",
            run: circuit_reciprocity,
        },
        Case {
            id: "circuit/unitarity",
            title: r"Circuits: a lossless netlist (an add-drop ring, an all-pass ring and an MZI), 4 external ports, 1.54 to 1.56 µm: $S^\dagger S$ against $I$ (largest entry of the difference shown)",
            tier: Tier::Analytic,
            source: r"a netlist of lossless components is lossless, $S^\dagger S = I$",
            run: circuit_unitarity,
        },
        Case {
            id: "circuit/adjoint-mzi",
            title: r"Circuit adjoint: an MZI's bar power summed over five wavelengths, 1.549 to 1.551 µm, its gradient with respect to both couplers' $\kappa^2$ and both arms' lengths against fourth-order central finite differences of the whole circuit ($\delta$ = 1e-3 and 1e-4 µm; the largest difference relative to the largest component shown)",
            tier: Tier::Analytic,
            source: r"the adjoint variable method, G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2-4, on the circuit's system $M = I - S_b \Gamma$: $\partial F/\partial \theta = 2 \operatorname{Re} \operatorname{tr}(\Lambda^\mathsf{T} \thinspace \partial_\theta S_b \thinspace A)$ with $M^\mathsf{T} \Lambda = E G$; the differences' own round-off and truncation, about 1e-10",
            run: circuit_adjoint_mzi,
        },
        Case {
            id: "circuit/adjoint-ring",
            title: r"Circuit adjoint: an add-drop ring ($\kappa^2$ = 0.1 and 0.05, 62.8 µm) on the flank of a resonance, 1.5521 µm: the gradient of $(\lvert S_{41} \rvert^2 - 0.5)^2$ with respect to its four parameters, the same way ($\delta$ = 1e-4 and 1e-5 µm; shown as above)",
            tier: Tier::Analytic,
            source: "the adjoint variable method, as above; the differences' round-off, about 3e-11",
            run: circuit_adjoint_ring,
        },
        Case {
            id: "circuit/adjoint-mesh",
            title: r"Circuit adjoint: a 4 × 4 mesh of six MZIs in Clements et al.'s rectangular arrangement, 24 parameters (every coupler's $\kappa^2$ and every phase): the gradient of $\sum_{qp} \lvert S_{qp} - T_{qp} \rvert^2$ for a complex target $T$, the same way ($\delta$ = 1e-3; shown as above)",
            tier: Tier::Analytic,
            source: r"the adjoint variable method, as above, with a sensitivity $G = \overline{S - T}$ that depends on the phases; the mesh is W. R. Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Fig. 1(b); the differences' round-off, about 1e-11",
            run: circuit_adjoint_mesh,
        },
        Case {
            id: "circuit/adjoint-nested",
            title: r"Circuit adjoint: a netlist of circuits (an add-drop ring and an MZI as instances, a reflective 3-port in a loop, a waveguide), $\sum_\lambda \lvert S_{41} \rvert^2$ at three wavelengths, its 9 parameters through the inner circuits' own adjoints, the same way ($\delta$ = 1e-4 and 1e-5 µm; shown as above)",
            tier: Tier::Analytic,
            source: r"the adjoint variable method, as above; a circuit's derivatives are its Jacobian $\partial S/\partial \theta = Z^\mathsf{T} \partial_\theta S_b \thinspace A$, $M^\mathsf{T} Z = E$; the differences' round-off, about 5e-10",
            run: circuit_adjoint_nested,
        },
        Case {
            id: "circuit/adjoint-differences",
            title: r"Circuit adjoint: the circuit solve's netlist of 11 instances, whose components give no derivatives, differentiated each by fourth-order differences of its S-matrix (step $\epsilon^{1/3} \max(\lvert \theta \rvert, 1)$): $\sum_\lambda \lvert S_{41} \rvert^2$ at three wavelengths, its 11 parameters, the same way ($\delta$ = 1e-4 and 1e-5 µm; shown as above)",
            tier: Tier::Analytic,
            source: "the adjoint variable method, as above; the components' differences add about 1e-11, the circuit's differences' round-off about 5e-10",
            run: circuit_adjoint_differences,
        },
        Case {
            id: "circuit/component-differences",
            title: r"A component's $\partial S/\partial \theta$ by fourth-order differences (central; one-sided inwards at a bound), against its closed form: a waveguide's length at 100 µm and at 0, a coupler's $\kappa^2$ at 0.3, a phase shifter at its bounds (the largest difference relative to the largest entry shown)",
            tier: Tier::Analytic,
            source: r"$\partial t/\partial L = t\thinspace(i 2\pi n/\lambda - \alpha \ln 10/20)$, $\partial r/\partial \kappa^2 = -1/2r$, $\partial \kappa/\partial \kappa^2 = 1/2\kappa$, $\partial e^{i\phi}/\partial \phi = i e^{i\phi}$; the round-off of a step of $6 \times 10^{-6} \max(\lvert \theta \rvert, 1)$, about 1e-11",
            run: circuit_component_differences,
        },
    ]
}

fn circuit_adjoint(check: crate::circuit::adjoint::checks::Check) -> Outcome {
    let error = check.error();
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 1e-8,
        error,
    }
}

fn circuit_adjoint_mzi() -> Outcome {
    circuit_adjoint(crate::circuit::adjoint::checks::Check::Mzi)
}

fn circuit_adjoint_ring() -> Outcome {
    circuit_adjoint(crate::circuit::adjoint::checks::Check::Ring)
}

fn circuit_adjoint_mesh() -> Outcome {
    circuit_adjoint(crate::circuit::adjoint::checks::Check::Mesh)
}

fn circuit_adjoint_nested() -> Outcome {
    circuit_adjoint(crate::circuit::adjoint::checks::Check::Nested)
}

fn circuit_adjoint_differences() -> Outcome {
    circuit_adjoint(crate::circuit::adjoint::checks::Check::Fallback)
}

fn circuit_component_differences() -> Outcome {
    let error = crate::circuit::adjoint::checks::fallback_error();
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 1e-9,
        error,
    }
}

fn circuit_series() -> Outcome {
    use crate::circuit::{Component, Netlist, ideal};
    use std::sync::Arc;
    let guide = ideal::wire();
    let mut n = Netlist::new();
    let ok = "a valid netlist";
    for (name, length) in [("x", 12.5), ("y", 30.25)] {
        n.add(name, Arc::new(guide.clone())).expect(ok);
        n.set(name, "length", length).expect(ok);
    }
    n.connect("x.b", "y.a").expect(ok);
    n.expose("in", "x.a").expect(ok);
    n.expose("out", "y.b").expect(ok);
    let circuit = n.compile().expect(ok);
    let worst = ideal::sweep()
        .into_iter()
        .map(|w| {
            let s = circuit.s_matrix(w).expect(ok);
            let one = guide.s_matrix(w, &[42.75]).expect(ok);
            s.max_difference(&one).expect(ok)
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn circuit_mzi() -> Outcome {
    use crate::circuit::ideal;
    let guide = ideal::wire();
    let mut worst: f64 = 0.0;
    for kappa2 in [0.5, 0.3] {
        let circuit = ideal::mzi(&guide, 100.0, 120.0, kappa2).expect("a valid netlist");
        for w in ideal::sweep() {
            let s = circuit.s_matrix(w).expect("a solvable netlist");
            let t = |l| guide.transmission(w.to_um(), l);
            for (q, p) in [(2, 0), (3, 0), (2, 1), (3, 1)] {
                let exact = ideal::mzi_closed_form(t(100.0), t(120.0), kappa2, q - 2, p);
                worst = worst.max((s[(q, p)] - exact).norm());
            }
        }
    }
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-13,
        error: worst,
    }
}

/// The all-pass ring of the validation cases: 62.8 µm round (a 10 µm radius).
const RING_LENGTH: f64 = std::f64::consts::TAU * 10.0;

fn circuit_ring_all_pass() -> Outcome {
    use crate::circuit::ideal;
    let guide = ideal::wire();
    let mut worst: f64 = 0.0;
    for kappa2 in [0.1, 0.02] {
        let circuit = ideal::all_pass(&guide, RING_LENGTH, kappa2).expect("a valid netlist");
        let r = (1.0 - kappa2).sqrt();
        for w in ideal::sweep() {
            let t = guide.transmission(w.to_um(), RING_LENGTH);
            let (a, phi) = (t.norm(), t.arg());
            let s = circuit.s_matrix(w).expect("a solvable netlist");
            worst = worst
                .max((s[(1, 0)] - ideal::bogaerts_eq1(r, a, phi)).norm())
                .max((s.power(1, 0) - ideal::bogaerts_eq2(r, a, phi)).abs());
        }
    }
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn circuit_ring_add_drop() -> Outcome {
    use crate::circuit::ideal;
    let guide = ideal::wire();
    let (k1, k2) = (0.1, 0.05);
    let circuit = ideal::add_drop(&guide, RING_LENGTH, k1, k2).expect("a valid netlist");
    let (r1, r2) = ((1.0 - k1).sqrt(), (1.0 - k2).sqrt());
    let mut worst: f64 = 0.0;
    for w in ideal::sweep() {
        let t = guide.transmission(w.to_um(), RING_LENGTH);
        let (a, phi) = (t.norm(), t.arg());
        let s = circuit.s_matrix(w).expect("a solvable netlist");
        worst = worst
            .max((s.power(1, 0) - ideal::bogaerts_eq5(r1, r2, a, phi)).abs())
            .max((s.power(3, 0) - ideal::bogaerts_eq6(r1, r2, a, phi)).abs());
    }
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn circuit_ring_fsr() -> Outcome {
    use crate::circuit::ideal;
    let guide = ideal::wire();
    let circuit = ideal::all_pass(&guide, RING_LENGTH, 0.1).expect("a valid netlist");
    let through = |um: f64| {
        circuit
            .s_matrix(ideal::um(um))
            .expect("a solvable netlist")
            .power(1, 0)
    };
    // the minima of the through power on a 0.1 nm grid, refined by golden sections
    let grid: Vec<f64> = (0..400).map(|i| 1.53 + 1e-4 * f64::from(i)).collect();
    let values: Vec<f64> = grid.iter().map(|&w| through(w)).collect();
    let minima: Vec<f64> = (1..grid.len() - 1)
        .filter(|&i| values[i] < values[i - 1] && values[i] <= values[i + 1])
        .map(|i| golden_minimum(&through, grid[i - 1], grid[i + 1]))
        .collect();
    let below = minima.iter().rev().find(|&&w| w < 1.55);
    let above = minima.iter().find(|&&w| w >= 1.55);
    let (Some(&below), Some(&above)) = (below, above) else {
        return Outcome {
            measured: f64::NAN,
            expected: 0.0,
            tolerance: 0.0,
            error: f64::NAN,
        };
    };
    let mid = 0.5 * (below + above);
    let expected = mid * mid / (guide.group_index * RING_LENGTH) * 1e3;
    let measured = (above - below) * 1e3;
    Outcome {
        measured,
        expected,
        // Eq. 9's own first-order error, 7.8e-5 nm, with room for the search's
        tolerance: 1e-4,
        error: (measured - expected).abs(),
    }
}

/// The minimum of `f` between `a` and `b` by golden sections, to 1e-12.
fn golden_minimum(f: &impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
    let (mut fc, mut fd) = (f(c), f(d));
    while b - a > 1e-12 {
        if fc < fd {
            b = d;
            (d, fd) = (c, fc);
            c = b - g * (b - a);
            fc = f(c);
        } else {
            a = c;
            (c, fc) = (d, fd);
            d = a + g * (b - a);
            fd = f(d);
        }
    }
    0.5 * (a + b)
}

fn circuit_growth() -> Outcome {
    use crate::circuit::ideal;
    let circuit = ideal::tangle();
    let worst = ideal::sweep()
        .into_iter()
        .map(|w| {
            let s = circuit.s_matrix(w).expect("a solvable netlist");
            let grown = circuit.s_matrix_by_growth(w).expect("a solvable netlist");
            s.max_difference(&grown).expect("the same ports")
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-13,
        error: worst,
    }
}

fn circuit_reciprocity() -> Outcome {
    use crate::circuit::ideal;
    let circuit = ideal::tangle();
    let worst = ideal::sweep()
        .into_iter()
        .map(|w| {
            circuit
                .s_matrix(w)
                .expect("a solvable netlist")
                .reciprocity_error()
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-13,
        error: worst,
    }
}

fn circuit_unitarity() -> Outcome {
    use crate::circuit::ideal;
    let circuit = ideal::lossless();
    let worst = ideal::sweep()
        .into_iter()
        .map(|w| {
            circuit
                .s_matrix(w)
                .expect("a solvable netlist")
                .unitarity_error()
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-13,
        error: worst,
    }
}

fn fdfd3d_qmr_direct() -> Outcome {
    use crate::fdfd::Formulation;
    use crate::fdfd::checks3d::qmr_against_direct;
    let worst = qmr_against_direct(Formulation::CurlCurl)
        .0
        .max(qmr_against_direct(Formulation::ShinFan).0);
    Outcome {
        measured: worst,
        expected: 0.0,
        // a residual of 1e-10 bounds the error by the condition number times 1e-10: measured
        // 1.1e-11 (curl-curl) and 1.3e-10 (Shin and Fan)
        tolerance: 1e-9,
        error: worst,
    }
}

fn fdfd3d_qmr_plateau() -> Outcome {
    let how = crate::fdfd::checks3d::shin_fan_square(0.0, 1e-6);
    let measured = how.history[19];
    Outcome {
        measured,
        expected: 0.707,
        // their 0.707 is the residual's projection on the eigenvalue nearest zero, an
        // approximate floor; measured 0.709 from iteration 5 to 40
        tolerance: 5e-3,
        error: (measured - 0.707).abs(),
    }
}

fn fdfd3d_qmr_iterations(s: f64, read: f64) -> Outcome {
    let measured = crate::fdfd::checks3d::shin_fan_square(s, 1e-6).iterations as f64;
    Outcome {
        measured,
        expected: read,
        // read off the plot: 3.4 pixels per iteration at 500 dpi, the curves' width about 5
        tolerance: 5.0,
        error: (measured - read).abs(),
    }
}

fn fdfd3d_qmr_iterations_curl_curl() -> Outcome {
    fdfd3d_qmr_iterations(0.0, 114.0)
}

fn fdfd3d_qmr_iterations_shin_fan() -> Outcome {
    fdfd3d_qmr_iterations(-1.0, 77.0)
}

fn fdfd3d_film(kind: Polarization, tolerance: f64) -> Outcome {
    use crate::fdfd::checks3d::{PML, film_ratios, film_run};
    let run = film_run(kind, 0.0025, 30f64.to_radians(), (PML, 1e-8));
    let ((r, _), (exact, _)) = film_ratios(&run);
    Outcome {
        measured: r,
        expected: exact,
        tolerance,
        error: (r - exact).abs(),
    }
}

fn fdfd3d_film_te() -> Outcome {
    // second order from 1.9e-4 at 5 nm predicts 4.6e-5, which it is
    fdfd3d_film(Polarization::Te, 6e-5)
}

fn fdfd3d_film_tm() -> Outcome {
    // second order from 1.2e-4 at 5 nm predicts 3.1e-5, which it is
    fdfd3d_film(Polarization::Tm, 5e-5)
}

fn fdfd3d_flux_conservation() -> Outcome {
    use crate::fdfd::checks3d::{PML, film_run, flux_spread};
    let mut worst: f64 = 0.0;
    for kind in [Polarization::Te, Polarization::Tm] {
        for deg in [0.0, 30.0, 60.0_f64] {
            let run = film_run(kind, 0.01, deg.to_radians(), (PML, 1e-8));
            worst = worst.max(flux_spread(&run));
        }
    }
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-10,
        error: worst,
    }
}

fn fdfd3d_pml_reflection() -> Outcome {
    use crate::fdfd::checks3d::pml_reflection;
    let worst = pml_reflection(Polarization::Te).max(pml_reflection(Polarization::Tm));
    Outcome {
        measured: worst,
        expected: 0.0,
        // as in 2D: the round trip keeps (1e-8)^1.38 of the power, an amplitude of 3e-6
        tolerance: 1e-5,
        error: worst,
    }
}

fn fdfd3d_two_d_agreement() -> Outcome {
    use crate::fdfd::checks3d::two_d_difference;
    let worst = two_d_difference(crate::fdfd::Polarization::Ez)
        .max(two_d_difference(crate::fdfd::Polarization::Hz));
    Outcome {
        measured: worst,
        expected: 0.0,
        // the same equations factorized two ways: round-off, 2e-13 measured
        tolerance: 1e-11,
        error: worst,
    }
}

fn fdfd_slab(polarization: crate::fdfd::Polarization) -> Outcome {
    use crate::fdfd::checks::{PML, slab_ratios, slab_run};
    let run = slab_run(polarization, 0.0025, 30f64.to_radians(), (PML, 1e-8));
    let ((r, _), (exact, _)) = slab_ratios(&run);
    Outcome {
        measured: r,
        expected: exact,
        tolerance: 5e-5,
        error: (r - exact).abs(),
    }
}

fn fdfd_slab_ez() -> Outcome {
    fdfd_slab(crate::fdfd::Polarization::Ez)
}

fn fdfd_slab_hz() -> Outcome {
    fdfd_slab(crate::fdfd::Polarization::Hz)
}

fn fdfd_flux_conservation() -> Outcome {
    use crate::fdfd::Polarization;
    use crate::fdfd::checks::{PML, flux_spread, slab_run};
    let mut worst: f64 = 0.0;
    for polarization in [Polarization::Ez, Polarization::Hz] {
        for deg in [0.0, 30.0, 60.0_f64] {
            let run = slab_run(polarization, 0.01, deg.to_radians(), (PML, 1e-8));
            worst = worst.max(flux_spread(&run));
        }
    }
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-10,
        error: worst,
    }
}

fn fdfd_pml_reflection() -> Outcome {
    use crate::fdfd::Polarization;
    use crate::fdfd::checks::pml_reflection;
    let worst = pml_reflection(Polarization::Ez).max(pml_reflection(Polarization::Hz));
    Outcome {
        measured: worst,
        expected: 0.0,
        // in oxide at 17 degrees the wave crosses the PML 1.38 times as fast as in vacuum, so its
        // round trip keeps (1e-8)^1.38 of the power: an amplitude of 3e-6
        tolerance: 1e-5,
        error: worst,
    }
}

fn fdfd_port_mode(polarization: crate::fdfd::Polarization) -> Outcome {
    let (got, exact) = crate::fdfd::checks::port_mode_index(polarization, 0.0025);
    Outcome {
        measured: got,
        expected: exact,
        tolerance: 2e-4,
        error: (got - exact).abs(),
    }
}

fn fdfd_port_mode_te() -> Outcome {
    fdfd_port_mode(crate::fdfd::Polarization::Ez)
}

fn fdfd_port_mode_tm() -> Outcome {
    fdfd_port_mode(crate::fdfd::Polarization::Hz)
}

fn fdfd_straight_guide() -> Outcome {
    use crate::fdfd::Polarization;
    use crate::fdfd::checks::straight_guide_error;
    let worst = straight_guide_error(Polarization::Ez).max(straight_guide_error(Polarization::Hz));
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn fdfd_reciprocity() -> Outcome {
    use crate::fdfd::Polarization;
    use crate::fdfd::checks::step;
    let worst = [Polarization::Ez, Polarization::Hz]
        .into_iter()
        .map(|p| {
            let (s, _, _) = step(p);
            (s[1][0] - s[0][1]).norm() / s[1][0].norm()
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn fdfd_step_reflection_te() -> Outcome {
    let (s, n1, n2) = crate::fdfd::checks::step(crate::fdfd::Polarization::Ez);
    let measured = s[0][0].norm_sqr();
    let expected = ((n1 - n2) / (n1 + n2)).powi(2);
    Outcome {
        measured,
        expected,
        tolerance: 2e-5,
        error: (measured - expected).abs(),
    }
}

fn fdfd_adjoint(polarization: crate::fdfd::Polarization) -> Outcome {
    let error = crate::fdfd::checks::gradient_check(polarization);
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 1e-6,
        error,
    }
}

fn fdfd_adjoint_ez() -> Outcome {
    fdfd_adjoint(crate::fdfd::Polarization::Ez)
}

fn fdfd_adjoint_hz() -> Outcome {
    fdfd_adjoint(crate::fdfd::Polarization::Hz)
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

fn multilayer_fresnel() -> Outcome {
    use crate::mode::multilayer::Multilayer;
    let (n1, n2) = (1.0, 1.5);
    let one = |n: f64| Complex64::new(n, 0.0);
    let worst = Multilayer::new(one(n1), &[(one(n2), Length::nm(300.0))], one(n2))
        .and_then(|stack| {
            let lam = Wavelength::um(0.6328)?;
            let mut worst: f64 = 0.0;
            for deg in [0.0, 20.0, 45.0, 70.0_f64] {
                let t1 = deg.to_radians();
                let t2 = (n1 * t1.sin() / n2).asin();
                let (c1, c2) = (t1.cos(), t2.cos());
                let rs = (n1 * c1 - n2 * c2) / (n1 * c1 + n2 * c2);
                let rp = (n2 * c1 - n1 * c2) / (n2 * c1 + n1 * c2);
                let te = stack.reflection(Polarization::Te, lam, t1)?;
                let tm = stack.reflection(Polarization::Tm, lam, t1)?;
                worst = worst.max((te.r - rs).norm()).max((tm.r - rp).norm());
            }
            Ok(worst)
        })
        .unwrap_or(f64::NAN);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

fn multilayer_bragg() -> Outcome {
    use crate::mode::multilayer::Multilayer;
    let (n0, nh, nl, ns, pairs) = (1.0, 2.3, 1.38, 1.52, 8);
    let one = |n: f64| Complex64::new(n, 0.0);
    let mut films = Vec::new();
    for _ in 0..pairs {
        films.push((one(nh), Length::um(0.55 / (4.0 * nh))));
        films.push((one(nl), Length::um(0.55 / (4.0 * nl))));
    }
    let q = ns / n0 * (nh / nl).powi(2 * pairs);
    let expected = ((1.0 - q) / (1.0 + q)).powi(2);
    let measured = Multilayer::new(one(n0), &films, one(ns))
        .and_then(|stack| stack.reflection(Polarization::Te, Wavelength::um(0.55)?, 0.0))
        .map_or(f64::NAN, |r| r.reflectance);
    Outcome {
        measured,
        expected,
        tolerance: 1e-12,
        error: (measured - expected).abs(),
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

fn marcatili_closed_form() -> Outcome {
    let worst = crate::mode::marcatili::closed_form_deviation(1.5, 1.5 / 1.05);
    Outcome {
        measured: worst,
        expected: 0.0,
        // "a few percent"
        tolerance: 0.05,
        error: worst,
    }
}

fn marcatili_vector() -> Outcome {
    let (v, m) = crate::mode::marcatili::against_vector(3.0, crate::mode::marcatili::Family::Ex);
    Outcome {
        measured: (v - m).abs(),
        expected: 0.0,
        tolerance: 5e-4,
        error: (v - m).abs(),
    }
}

fn slab_fd_chilwell() -> Outcome {
    let profile = crate::mode::slab_fd::chilwell_profile(0.001);
    let w = um(0.6328);
    let worst = [(Polarization::Te, 0), (Polarization::Tm, 4)]
        .iter()
        .map(|&(pol, offset)| {
            let mut got: Vec<f64> = profile
                .modes(pol, w, 4, Some(1.63))
                .map(|m| m.iter().map(|m| m.effective_index.re).collect())
                .unwrap_or_default();
            got.sort_by(|a, b| b.total_cmp(a));
            if got.len() < 4 {
                return f64::NAN;
            }
            got.iter()
                .zip(&CHILWELL_TABLE_3[offset..offset + 4])
                .map(|(g, (n, _))| (g - n).abs())
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        // the table's 6 decimals and the grid
        tolerance: 2e-6,
        error: worst,
    }
}

/// The first mode near `exact` by Hadley's high-accuracy equations: its effective index.
fn hadley_index(cs: &crate::mode::vector::CrossSection, w: Wavelength, near: Option<f64>) -> f64 {
    crate::mode::hadley::modes(cs, w, 1, near)
        .ok()
        .and_then(|m| m.first().map(|m| m.effective_index().re))
        .unwrap_or(f64::NAN)
}

/// Hadley I's uniform box on `n` × `n` cells: (found, exact).
fn hadley_box(n: usize) -> (f64, f64) {
    let (cs, w, exact) = crate::mode::hadley::uniform_box(n);
    (hadley_index(&cs, w, Some(exact)), exact)
}

fn hadley_uniform_box() -> Outcome {
    let (n, exact) = hadley_box(8);
    Outcome {
        measured: n,
        expected: exact,
        // 2.4e-10
        tolerance: 1e-9,
        error: (n - exact).abs(),
    }
}

fn hadley_uniform_order() -> Outcome {
    let errors = [4, 8].map(|n| {
        let (found, exact) = hadley_box(n);
        (found - exact).abs()
    });
    let order = (errors[0] / errors[1]).log2();
    Outcome {
        // two decimals: the report reads the same on every platform
        measured: (order * 100.0).round() / 100.0,
        expected: 6.0,
        tolerance: 0.2,
        error: (order - 6.0).abs(),
    }
}

/// Hadley I's two-dielectric box, high contrast, `n` cells across the top layer: (found, exact).
fn hadley_slab(n: usize, vertical: bool) -> (f64, f64) {
    let (cs, w, exact) = crate::mode::hadley::two_dielectric_box(true, n, vertical);
    (hadley_index(&cs, w, Some(exact)), exact)
}

fn hadley_interface() -> Outcome {
    let (n, exact) = hadley_slab(16, false);
    Outcome {
        measured: n,
        expected: exact,
        // 1.6e-9
        tolerance: 5e-9,
        error: (n - exact).abs(),
    }
}

fn hadley_interface_order() -> Outcome {
    let errors = [8, 16].map(|n| {
        let (found, exact) = hadley_slab(n, false);
        (found - exact).abs()
    });
    let order = (errors[0] / errors[1]).log2();
    Outcome {
        // two decimals: the report reads the same on every platform
        measured: (order * 100.0).round() / 100.0,
        expected: 6.0,
        tolerance: 0.3,
        error: (order - 6.0).abs(),
    }
}

fn hadley_interface_turned() -> Outcome {
    let (flat, _) = hadley_slab(8, false);
    let (side, _) = hadley_slab(8, true);
    let d = (flat - side).abs();
    Outcome {
        measured: d,
        expected: 0.0,
        tolerance: 1e-12,
        error: d,
    }
}

/// Hadley II's corner problem `problem` by his equations on `n` × `n` cells: (found, exact).
fn hadley_corner(problem: usize, n: usize) -> (f64, f64) {
    let (cs, exact) = crate::mode::vector::hadley_problem(problem, n);
    (hadley_index(&cs, um(1.5), None), exact)
}

fn hadley_corners(problem: usize) -> Outcome {
    let (n, exact) = hadley_corner(problem, 128);
    Outcome {
        measured: n,
        expected: exact,
        // 5.2e-7, 2.3e-7, 2.5e-7 and 1.9e-7
        tolerance: 1e-6,
        error: (n - exact).abs(),
    }
}

fn hadley_corners_1() -> Outcome {
    hadley_corners(1)
}

fn hadley_corners_2() -> Outcome {
    hadley_corners(2)
}

fn hadley_corners_3() -> Outcome {
    hadley_corners(3)
}

fn hadley_corners_4() -> Outcome {
    hadley_corners(4)
}

fn hadley_corners_order() -> Outcome {
    let errors = [32, 128].map(|n| {
        let (found, exact) = hadley_corner(1, n);
        (found - exact).abs()
    });
    // two halvings of the spacing
    let order = (errors[0] / errors[1]).log2() / 2.0;
    Outcome {
        // two decimals: the report reads the same on every platform
        measured: (order * 100.0).round() / 100.0,
        expected: 2.0,
        tolerance: 0.25,
        error: (order - 2.0).abs(),
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
    fn the_cases_math_survives_github() {
        for case in cases() {
            for text in [case.title, case.source] {
                let dollars = text.matches('$').count();
                assert!(dollars.is_multiple_of(2), "{}: an unclosed $", case.id);
                let mut chars = text.chars().peekable();
                while let Some(c) = chars.next() {
                    // GitHub drops a backslash before ASCII punctuation, even in math, and
                    // escapes a > in math twice
                    let next = chars.peek().copied().unwrap_or(' ');
                    assert!(
                        !(c == '\\' && next.is_ascii_punctuation()) && c != '>' && c != '<',
                        "{}: {c}{next} in {text}",
                        case.id
                    );
                }
            }
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

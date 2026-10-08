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

use crate::circuit::components::checks as components;
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
    let mut cases = vec![
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
            id: "material/inp-suzuki-tada-faust-henry",
            title: r"InP: the Pockels nonlinearity $d_{41}^{EO} = -n_0^4 r_{41}^S / 4$ from the catalogue's $r_{41}^S$ at 1.064 µm, and its electronic and lattice parts through the Faust–Henry coefficient $C = -0.53$, against the 39, 83 and −44 pm/V Suzuki and Tada print (largest relative deviation shown)",
            tier: Tier::Published,
            source: "N. Suzuki, K. Tada, Jpn. J. Appl. Phys. 23, 291 (1984), doi:10.1143/JJAP.23.291, Eqs. (6)–(7) and Table II (n₀ = 3.29); the printed values have two digits",
            run: catalogue::suzuki_tada_faust_henry,
        },
        Case {
            id: "material/aln-majkic-d33",
            title: r"AlN: $d_{33}$ from the measured ratio $0.169\thinspace d_{33}(\mathrm{LiNbO_3})$ and the catalogue's $d_{33}$ of congruent lithium niobate at 1.064 µm (pm/V, shown)",
            tier: Tier::Published,
            source: "A. Majkić et al., Phys. Status Solidi B 254, 1700077 (2017), doi:10.1002/pssb.201700077, p. 4 and Table 1: 4.3 pm/V at 1030 nm, from Shoji et al.'s 25.2 pm/V",
            run: catalogue::majkic_d33,
        },
        Case {
            id: "material/aln-rigler-se",
            title: r"AlN, Al-polar and N-polar (Rigler et al. 2015, $n^2 = 1 + A\lambda^2/(\lambda^2 - B^2)$): $n_o$ and $n_e$ at 658 nm against the ellipsometric values their Table I lists beside the fit (largest deviation shown)",
            tier: Tier::Published,
            source: "M. Rigler et al., Appl. Phys. Express 8, 042603 (2015), doi:10.7567/APEX.8.042603, Table I, the 658 nm SE column, to three decimals with A and B rounded",
            run: catalogue::rigler_2015_se,
        },
        Case {
            id: "material/aln-rigler-maie",
            title: r"AlN, Al-polar and N-polar (Rigler et al. 2015): $n_o$ and $n_e$ at 658 nm against the independent multi-angle ellipsometry of the same films (largest deviation shown)",
            tier: Tier::Published,
            source: "M. Rigler et al., Appl. Phys. Express 8, 042603 (2015), doi:10.7567/APEX.8.042603, Table I, the 658 nm MAIE column, each to 0.01",
            run: catalogue::rigler_2015_maie,
        },
        Case {
            id: "material/ingap-ferrini-table-consistency",
            title: r"InGaP above the gap (Ferrini et al. 2002, Table 3): $\varepsilon_1 = n^2 - k^2$ and $\varepsilon_2 = 2nk$ from the table's $n$ and $k$ against its printed $\varepsilon$, all 37 rows (largest deviation shown)",
            tier: Tier::Published,
            source: "R. Ferrini et al., Eur. Phys. J. B 27, 449 (2002), doi:10.1140/epjb/e2002-00177-x, Table 3; n and k have three decimals, which allows about 0.006 in the product",
            run: catalogue::ferrini_table_consistency,
        },
        Case {
            id: "material/ingap-ferrini-table-knots",
            title: r"InGaP above the gap: the catalogue's $n + ik$ at Ferrini et al.'s 37 photon energies against their Table 3 (largest deviation shown)",
            tier: Tier::Published,
            source: "R. Ferrini et al., Eur. Phys. J. B 27, 449 (2002), doi:10.1140/epjb/e2002-00177-x, Table 3 (the second row printed 4.2 eV read as 4.1); a natural spline passes through its knots",
            run: catalogue::ferrini_table_knots,
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
            id: "fdfd3d/port-mode-slab-te",
            title: "3D FDFD ports: the fundamental TE mode of a 220 nm silicon slab (3.476 in 1.444) at 1.55 µm, uniform along $y$, solved on a port's plane normal to $x$ on a 2.5 nm grid: effective index (shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab); the port's eigenproblem is the 3D scheme's own on the plane, second order: 9.6e-4, 2.4e-4, 6.0e-5 at 10, 5, 2.5 nm",
            run: fdfd3d_port_mode_slab_te,
        },
        Case {
            id: "fdfd3d/port-mode-slab-tm",
            title: "The same, the TM mode (shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab); second order: 2.5e-3, 6.1e-4, 1.5e-4 at 10, 5, 2.5 nm, the 2D ports' errors",
            run: fdfd3d_port_mode_slab_tm,
        },
        Case {
            id: "fdfd3d/port-mode-strip",
            title: r"3D FDFD ports: the TE-like mode of a $500 \times 220$ nm silicon strip in oxide at 1.55 µm on a port's plane of a 20 nm grid, inside walls $2.02 \times 3.5$ µm: effective index (shown)",
            tier: Tier::CrossCode,
            source: "Hadley's high-accuracy equations (mode::hadley, G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371) on 20, 10 and 5 nm grids, extrapolated: 2.445380; the port converges to it at second order (2.4e-3, 6.4e-4, 1.7e-4; its own limit 2.445387), and Fallahkhair et al.'s scheme (mode::vector) at about first, slowed by the corners",
            run: fdfd3d_port_mode_strip,
        },
        Case {
            id: "fdfd3d/straight-strip",
            title: r"3D FDFD ports: a straight silicon strip ($500 \times 220$ nm in oxide) between two ports 0.25 µm apart, PMLs close around it, 50 nm grid: largest of the magnitudes of $S_{11}$ and $S_{22}$ and of the errors of $S_{21}$ and $S_{12}$ against $\exp(i\beta L)$ (shown)",
            tier: Tier::Analytic,
            source: r"a uniform guide transmits its mode whole with phase $\beta L$; the port modes are the grid's own and the source is total-field/scattered-field (R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012), doi:10.2528/PIERB11092006, Eq. 55)",
            run: fdfd3d_straight_strip,
        },
        Case {
            id: "fdfd3d/adjoint-gradient",
            title: r"3D FDFD adjoint gradients: a rectangular guide ($\varepsilon = 12$, $0.4 \times 0.3$ µm in 2.1) with a block of $\varepsilon = 6$ beside it, $14 \times 12 \times 24$ cells of 50 nm, PMLs of 4, 1.55 µm: the gradient of the power its mode carries ahead with respect to the permittivity at eight values of E (each component, in the block, the core and the cladding) against fourth-order central differences ($\delta = 10^{-3}$) (largest difference relative to the largest gradient shown)",
            tier: Tier::Analytic,
            source: r"the adjoint variable method, G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2-4, with $A^T = V A V^{-1}$ ($V$ the product of the PMLs' stretches) and the Lorentz form's weights as $\partial F/\partial u$; measured 2.8e-10",
            run: fdfd3d_adjoint_gradient,
        },
        Case {
            id: "fdfd3d/reciprocity",
            title: "3D FDFD ports: a silicon strip stepping from 400 to 600 nm wide, off the grid's axis, PMLs close around it, 50 nm grid: $S_{21}$ against $S_{12}$ (relative difference shown)",
            tier: Tier::Analytic,
            source: "Lorentz reciprocity: $S$ is symmetric for a reciprocal device; the scheme keeps it, with the PMLs' stretches as weights and the modes normalized by the unconjugated Lorentz form",
            run: fdfd3d_reciprocity,
        },
        Case {
            id: "fdfd3d/closed-guide-energy",
            title: r"3D FDFD ports: a silicon strip in a closed metal box of oxide ($0.6 \times 0.4$ µm), stepping from 300 to 400 nm wide, ports 1 µm from the step, 50 nm grid: the $S$-matrix between all the propagating modes on both sides (3 and 3), its distance from unitary (shown)",
            tier: Tier::Analytic,
            source: r"Poynting's theorem: a closed lossless guide loses no power, so $S^\dagger S = 1$ but for the power the evanescent modes carry across the ports' planes, which falls as $e^{-2\kappa d}$: 4e-4, 1e-4, 2.6e-5, 7e-6, 2e-6 for ports 0.2 to 1 µm from the step",
            run: fdfd3d_closed_guide_energy,
        },
        Case {
            id: "fdfd3d/two-d-s-matrix",
            title: "3D FDFD ports on a structure invariant along $z$ (a silicon slab stepping from 220 to 300 nm, one periodic cell along $z$, 20 nm grid) against the 2D solver's $S$-matrix, $E$ along $z$ and $H$ along $z$ (largest difference shown)",
            tier: Tier::Analytic,
            source: r"with $\partial/\partial z = 0$ the 3D scheme is the 2D one (fdfd3d/two-d-agreement); with $H$ along $z$ the two agree to the eigensolver's tolerance (2e-10), with $E$ along $z$ to 9e-9, the PMLs half a cell apart in the two grids; both take a mode's backward twin with the same tangential $E$, so the reflections agree in sign",
            run: fdfd3d_two_d_s_matrix,
        },
        Case {
            id: "fdfd3d/qmr-direct",
            title: "3D FDFD by QMR, on the curl-curl operator and on Shin and Fan's ($s = -1$), to a relative residual of 1e-10, against the sparse direct solver: a silicon strip in oxide, $16^3$ cells of 40 nm, PMLs all round (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: "the same system solved two ways: QMR, R. W. Freund, N. M. Nachtigal, Numer. Math. 60, 315 (1991), doi:10.1007/BF01385726, Algorithm 3.1 without look-ahead (on the curl-curl operator, its form for complex symmetric matrices: fdfd3d/qmr-symmetric-direct); Shin and Fan's operator, Opt. Express 21, 22578 (2013), doi:10.1364/OE.21.022578, Eq. 7, has the same solution; measured 6.9e-12 and 1.3e-10",
            run: fdfd3d_qmr_direct,
        },
        Case {
            id: "fdfd3d/qmr-symmetric-direct",
            title: "3D FDFD on the curl-curl operator by QMR for complex symmetric matrices, on its diagonal similarity $B = S A S^{-1}$, which is complex symmetric, to a relative residual of 1e-10 of $A x = b$, against the sparse direct solver: the strip of fdfd3d/qmr-direct (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same system solved two ways: R. W. Freund, SIAM J. Sci. Stat. Comput. 13, 425 (1992), doi:10.1137/0913023, Algorithm 3.2 on his complex symmetric Lanczos process (Algorithm 2.1): one product with $B$ an iteration where QMR takes one with $A$ and one with $A^\mathsf{T}$; measured 6.9e-12 in 498 iterations",
            run: fdfd3d_qmr_symmetric_direct,
        },
        Case {
            id: "fdfd3d/ldlt-lu",
            title: "3D FDFD by the sparse direct solver two ways: $L D L^\\mathsf{T}$ of the curl-curl operator's complex symmetric similarity $B = S A S^{-1}$ (the solver's own choice) and $LU$ of $A$, both multifrontal with static pivoting after a maximum-product matching and scaling, each refined to a relative residual of 1e-12: the strip of fdfd3d/qmr-direct (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same system solved two ways: static pivoting after X. S. Li, J. W. Demmel, ACM Trans. Math. Softw. 29, 110 (2003), doi:10.1145/779359.779361, on I. S. Duff, J. Koster, SIAM J. Matrix Anal. Appl. 22, 973 (2001), doi:10.1137/S0895479899358443's matching and scaling; the fronts after I. S. Duff, J. K. Reid, ACM Trans. Math. Softw. 9, 302 (1983), doi:10.1145/356044.356047; measured 1.2e-15",
            run: fdfd3d_ldlt_lu,
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
            id: "fdfd3d/pml-reflection-stretched",
            title: r"3D FDFD: what a 20-cell PML graded to $R = 10^{-8}$ ($m = 3$) and stretched as much as it absorbs, $s = 1 + (1 + i)\sigma$, sends back of a plane wave 17 degrees off its normal, in oxide on a 20 nm grid, both polarizations (largest amplitude shown)",
            tier: Tier::Analytic,
            source: r"W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994), doi:10.1002/mop.4650071304: any stretch with $\operatorname{Im} s \gt 0$ absorbs without reflecting, so the real part changes only the discretization's reflection: 3.6e-6, against 2.5e-6 without it (fdfd3d/pml-reflection); thinner, it reflects more (10 cells of 10 nm: 2.3e-4 against 4e-5)",
            run: fdfd3d_pml_reflection_stretched,
        },
        Case {
            id: "fdfd3d/qmr-ilu-direct",
            title: r"3D FDFD by QMR on Shin and Fan's operator preconditioned by its ILU(0), to a relative residual of 1e-10, against the sparse direct solver: a silicon strip in oxide, $24 \times 20 \times 16$ cells of 40 nm, stretched PMLs of 6 cells all round (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same system solved two ways; ILU(0) as in Y. Saad, Iterative Methods for Sparse Linear Systems, 2nd ed., SIAM (2003), doi:10.1137/1.9780898718003, from the right, so the residual QMR stops on is the system's own; measured 2.4e-10, in 160 iterations against 548 without it",
            run: fdfd3d_qmr_ilu_direct,
        },
        Case {
            id: "fdfd3d/gmres-multigrid-direct",
            title: r"3D FDFD by GMRES on Shin and Fan's operator preconditioned by a multigrid cycle, to a relative residual of 1e-10, against the sparse direct solver: the same strip, $24 \times 20 \times 16$ cells of 40 nm, stretched PMLs of 6 cells all round (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same system solved two ways; the cycle after B. Reps, W. Vanroose, H. bin Zubair, J. Comput. Phys. 229, 8384 (2010), doi:10.1016/j.jcp.2010.07.022 (V(0, 1), ILU(0) smoothing, Galerkin coarse operators) with the complex shift of Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci. Comput. 27, 1471 (2006), doi:10.1137/040615195; GMRES as in Y. Saad, Iterative Methods for Sparse Linear Systems, 2nd ed., SIAM (2003), doi:10.1137/1.9780898718003, from the right; measured 2.0e-10, in 24 iterations against ILU(0)'s 160",
            run: fdfd3d_gmres_multigrid_direct,
        },
        Case {
            id: "fdtd/dispersion",
            title: r"FDTD's numerical dispersion: a plane wave's $E_z$ on a periodic Yee grid ($24 \times 18$ cells of $50 \times 40$ nm), started from rest, at four wave vectors (along an axis, the diagonal, between) and Courant numbers 0.5 to 1: its frequency per step against Taflove and Brodwin's relation (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"A. Taflove, M. E. Brodwin, IEEE Trans. Microw. Theory Tech. 23, 623 (1975), doi:10.1109/TMTT.1975.1128640: $\sin^2(\omega\Delta t/2)/\Delta t^2 = \sum_i \sin^2(k_i\Delta_i/2)/\Delta_i^2$, which the leapfrog on Yee's grid (K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966), doi:10.1109/TAP.1966.1138693) keeps exactly; measured 3.6e-15",
            run: fdtd_dispersion,
        },
        Case {
            id: "fdtd/energy",
            title: r"FDTD's energy in a closed conducting box ($10 \times 9 \times 8$ cells of 50 nm, a block of $\varepsilon = 12$ in $\varepsilon = 2$, random fields, Courant number 0.95), over 300 steps: the largest change of the leapfrog's invariant $\tfrac12\sum \varepsilon E^2 + \tfrac12\sum \tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$, relative to it (shown)",
            tier: Tier::Analytic,
            source: r"the leapfrog's discrete energy is conserved exactly when the two curls are each other's transposes, as Yee's differences are (Yee 1966, doi:10.1109/TAP.1966.1138693); measured 2.4e-15",
            run: fdtd_energy,
        },
        Case {
            id: "fdtd/fdfd-lossy",
            title: r"FDTD against FDFD: a continuous current in a closed box of a lossy medium ($16 \times 14 \times 12$ cells of 50 nm, $\varepsilon = 2.1$, $\sigma = 2$/µm, 1.55 µm, 64 steps a period), its steady amplitude against `Solver3d`'s field for the same current at the leapfrog's frequency $\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$ with $\varepsilon + i\sigma\cos(\omega\Delta t/2)/\tilde\omega$ (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same equations: the leapfrog's steady state at $\omega$ solves Yee's frequency-domain equations at $\tilde\omega$ exactly, the conductivity averaged over the step; at $\omega$ itself the difference is 4.1e-4, the leapfrog's dispersion; measured 2.3e-14",
            run: fdtd_fdfd_lossy,
        },
        Case {
            id: "fdtd/fdfd-cpml",
            title: r"FDTD against FDFD with open boundaries: a continuous current beside a silicon block ($16 \times 14 \times 12$ cells of 50 nm, a CPML of 4 cells, 1.55 µm, 128 steps a period), its steady amplitude against `Solver3d`'s field with its PML (largest field difference outside the PMLs relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"with $\kappa = 1$ and $\alpha = 0$ the convolutional PML (J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000); see docs/methods/fdtd.md for its DOI) is FDFD's stretched coordinate $s = 1 + i\sigma/\omega$, its recursive convolution to first order in $\Delta t$: 6.6e-4 at 64 steps a period, 3.5e-4 at 128 (measured)",
            run: fdtd_fdfd_cpml,
        },
        Case {
            id: "fdtd/cpml-thickness",
            title: r"FDTD's CPML in 2D: a pulse from a dipole in vacuum (cells of 50 nm, $\lambda = 1$ µm), recorded 2 cells from a CPML of 16 cells, against a grid 120 cells larger: the largest difference over the run, relative to the largest field (shown)",
            tier: Tier::Analytic,
            source: r"a CPML graded as FDFD's ($R = 10^{-8}$, order 3) reflects less as it thickens: measured 2.6e-2 at 4 cells, 1.2e-4 at 8 and 6.1e-6 at 16. Roden and Gedney's own case, a plate in soil, is the cpml_roden_gedney example",
            run: fdtd_cpml_thickness,
        },
        Case {
            id: "fdtd/spectrum",
            title: r"FDTD's sources normalized: a Gaussian pulse from an electric point current and from a magnetic dipole between values, in a closed box of a lossy medium ($16 \times 14 \times 12$ cells of 50 nm, $\varepsilon = 2.1$, $\sigma = 2$/µm), E's DFT at the carrier (1.55 µm) and 0.12 c/µm off it against `Solver3d`'s field for the sources' own transforms at the leapfrog's frequency (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same equations: each field's transform at its own times solves Yee's frequency-domain equations at $\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$ for the currents' transforms, once the fields have died away; the dipole restricted to the grid as A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010), doi:10.1016/j.cpc.2009.11.008 restrict it; measured 6.5e-13",
            run: fdtd_spectrum,
        },
        Case {
            id: "fdtd/tfsf-leakage",
            title: r"FDTD's total-field/scattered-field box with nothing in it: a plane-wave pulse in vacuum at normal and oblique incidence (2D, 80 × 80 cells of 50 nm, along $(1, 0)$, $(2, 1)$ and $(1, -3)$ in units of the cells, both polarizations; 3D, $36^3$ cells, along $(0, 0, 1)$ and $(2, 1, 1)$), the largest field in the scattered region over the run relative to the largest incident field (largest of the five shown)",
            tier: Tier::Analytic,
            source: r"K. Umashankar, A. Taflove, IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982), doi:10.1109/TEMC.1982.304054: the box is exact when the incident field solves the grid's own equations, which the auxiliary 1D run along the direction does at any angle whose components are whole numbers of cells; measured 1.3e-15",
            run: fdtd_tfsf_leakage,
        },
        Case {
            id: "fdtd/tfsf-slab",
            title: r"FDTD's total-field/scattered-field box at normal incidence on a slab of $\varepsilon = 4$, 0.3 µm thick, on cells of 5 nm: the power reflection from the scattered field in front at 0.8, 1 and 1.2 c/µm against Airy's formula (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the slab's exact reflection, $r = r_{12}(1 - e^{2i\delta})/(1 - r_{12}^2 e^{2i\delta})$, $\delta = n k_0 d$; second order in the cells: 5.4e-3 at 20 nm, 1.3e-3 at 10, 3.3e-4 at 5 (measured)",
            run: fdtd_tfsf_slab,
        },
        Case {
            id: "fdtd/mode-source-fdfd",
            title: r"FDTD's mode source against FDFD's: a rectangular guide's fundamental mode (core of $\varepsilon = 12$, $0.4 \times 0.3$ µm, in $\varepsilon = 2.1$) launched forward by a continuous wave in a closed box of a lossy medium ($16 \times 14 \times 24$ cells of 50 nm, $\sigma = 4$/µm, 1.55 µm, 64 steps a period), its steady amplitude against `Solver3d`'s field for its mode source of the same mode at the leapfrog's frequency (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same equations: the mode is FDFD's on the grid, carried along the guide as the grid carries it, and its currents on the plane are total-field/scattered-field's (Umashankar and Taflove 1982, doi:10.1109/TEMC.1982.304054), FDFD's own (R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012), doi:10.2528/PIERB11092006) in the time domain; measured 2.5e-14",
            run: fdtd_mode_source_fdfd,
        },
        Case {
            id: "fdtd/mode-source-backward",
            title: r"FDTD's mode source one way: the same guide's fundamental mode launched forward by a continuous wave in open space ($40 \times 36 \times 40$ cells of 50 nm, CPMLs of 8, 1.55 µm), the power going backward behind the source relative to the power going forward ahead (shown)",
            tier: Tier::Analytic,
            source: r"a mode launched by total-field/scattered-field goes one way only; what is left is the CPMLs', which differ from FDFD's PMLs the mode was solved in to first order in $\Delta t$; measured 1.5e-7. With a pulse 0.1 c/µm wide, the carrier's mode sends back 4.1e-4 and 3.8e-4 of the forward power at half that either side (docs/methods/fdtd.md)",
            run: fdtd_mode_source_backward,
        },
        Case {
            id: "fdtd/mode-source-power",
            title: r"FDTD's mode source's power: the forward flux ahead of the source in the same open guide, relative to the mode's own power at unit amplitude (shown)",
            tier: Tier::Analytic,
            source: r"a mode at unit amplitude carries its power; measured 1.000048",
            run: fdtd_mode_source_power,
        },
        Case {
            id: "fdtd/beam-waist",
            title: r"FDTD's Gaussian beam: a 2D beam ($E_z$, 1 µm in vacuum, $w_0 = 1.5$ µm, its focus 2 µm ahead of its plane) on cells of 50 nm, its width $2\sqrt{\langle (y - \bar y)^2 \rangle}$ at 0 to 12 µm past the focus fitted by $w^2 = w_0^2 + \theta^2 (d - d_0)^2$: the waist, µm (shown)",
            tier: Tier::Analytic,
            source: r"the paraxial Gaussian beam's waist $w_0$, the beam launched one way from its field on a plane decomposed into the grid's own plane waves; the focus fitted at 1.94 µm from the plane (2 asked)",
            run: fdtd_beam_waist,
        },
        Case {
            id: "fdtd/beam-divergence",
            title: r"FDTD's Gaussian beam's divergence: the same beam's $\theta$ from the same fit, against $2\sqrt{\langle (dk_x/dk_y)^2 \rangle}$ over its plane waves with the grid's dispersion (relative difference shown)",
            tier: Tier::Analytic,
            source: r"for a sum of plane waves the width grows exactly as $w^2 = w_0^2 + \theta^2 d^2$, $\theta$ from the waves' directions; measured 2.7e-6 (0.218825). The paraxial $\lambda/(\pi w_0) = 0.2122$ is 3.1 % below: the continuum's exact 0.2160 is 1.8 % above it (the beam isn't paraxial at $w_0 = 1.5\lambda$), and the grid's dispersion at 20 cells a wavelength adds 1.3 %",
            run: fdtd_beam_divergence,
        },
        Case {
            id: "fdtd/beam-tilt",
            title: r"FDTD's tilted Gaussian beam: the same beam tilted by 10°, the slope of its centre $d\bar y/dx$ from 2 to 14 µm past its plane against $\langle -dk_x/dk_y \rangle$ over its plane waves with the grid's dispersion (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the centre of a sum of plane waves moves in a straight line at their mean direction; measured 1.5e-4. The slope, 0.1825, is above $\tan 10° = 0.1763$ by the beam's spread and the grid's dispersion",
            run: fdtd_beam_tilt,
        },
        Case {
            id: "fdtd/ade-fdfd",
            title: r"FDTD's dispersive media against FDFD: a continuous current in a closed box filled with a Drude term and a Lorentz term ($\varepsilon_\infty = 2$, $f_p = 1.2$ c/µm; $\Delta\varepsilon = 1.5$ at $f_0 = 0.9$ c/µm; dampings 0.3 c/µm) and $\sigma = 2$/µm ($16 \times 14 \times 12$ cells of 50 nm, 1.55 µm, 64 steps a period), its steady amplitude against `Solver3d`'s field for the same current at the leapfrog's frequency with the leapfrog's permittivity (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same equations: M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave Lett. 7, 121 (1997), doi:10.1109/75.569723 (their synchronized Lorentz scheme) solves FDFD's equations at $\tilde\omega$ with each term's $\chi$ replaced by $(i\cos(\omega\Delta t/2)/\tilde\omega)\thinspace\gamma(z - 1/z)/(2\Delta t(z - \alpha - \xi/z))$, $z = e^{-i\omega\Delta t}$; with the medium's own $\varepsilon(\tilde\omega)$ the difference is 2.4e-3, the scheme's second-order error; measured 3.3e-14",
            run: fdtd_ade_fdfd,
        },
        Case {
            id: "fdtd/drude-fresnel",
            title: r"FDTD's Drude metal: a pulse at normal incidence on a half-space of $\varepsilon = 1 - \omega_p^2/(\omega^2 + i\gamma\omega)$ ($f_p = 1$ c/µm, $\gamma/2\pi = 0.05$ c/µm) in a column of 5 nm cells, its face between two values of E: the reflection coefficient from the runs with and without the metal at 0.4 to 1.6 c/µm, below and above the plasma frequency, against Fresnel's (largest $\lvert \Delta r \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"Fresnel's $r = (1 - n)/(1 + n)$, $n = \sqrt{\varepsilon(\omega)}$, the phase referred to the face with the grid's own wavenumber; second order in the cells: 6.9e-3 at 20 nm, 1.6e-3 at 10, 3.0e-4 at 5 (measured)",
            run: fdtd_drude_fresnel,
        },
        Case {
            id: "fdtd/lorentz-group-delay",
            title: r"FDTD's Lorentz medium: a pulse through a slab 10 µm thick of $\varepsilon = 2.25 + \omega_0^2/(\omega_0^2 - \omega^2 - i\gamma\omega)$ ($f_0 = 2$ c/µm, $\gamma/2\pi = 0.02$ c/µm) in a column of 2.5 nm cells: the first pass's group delay, $d \arg/d\omega$ of its DFT less the empty run's, as a group index $1 + \tau/d$, at 0.8 to 1.2 c/µm, against the first pass's exact delay (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the delay of $t_{12} t_{21} e^{i(n - 1)\omega d}$, which is $(n_g - 1)d$ with $n_g = \operatorname{Re} d(n\omega)/d\omega$ to 3e-5: $n_g$ from 1.977 to 2.402; second order in the cells to a floor of about 1e-4: 3.0e-3 at 10 nm, 8.8e-4 at 5, 3.6e-4 at 2.5 (measured)",
            run: fdtd_lorentz_group_delay,
        },
        Case {
            id: "fdtd/bloch-fdfd",
            title: r"FDTD with Bloch-periodic sides against FDFD: a continuous current and a dipole restricted across two Bloch sides ($k_x = 1.3$, $k_y = -2.1$ rad/µm, walls along z, $16 \times 14 \times 12$ cells of 50 nm, 1.55 µm, 64 steps a period), in a lossy medium ($\varepsilon = 2.1$, $\sigma = 2$/µm) and in the Drude and Lorentz medium of `fdtd/ade-fdfd`, the complex run's steady amplitude against `Solver3d`'s Bloch field at the leapfrog's frequency (largest field difference relative to the largest field shown)",
            tier: Tier::Analytic,
            source: r"the same equations: the real and imaginary parts step with Yee's real update, coupled across the Bloch sides by $e^{\pm ikL}$, as FDFD's differences are; measured 2.9e-14",
            run: fdtd_bloch_fdfd,
        },
        Case {
            id: "fdtd/bloch-multilayer",
            title: r"FDTD at oblique incidence through Bloch sides: a pulse at a fixed $k_x = 2\pi \times 0.2$ rad/µm on 4 pairs of $n = 2$ (0.12 µm) and $n = 1.5$ (0.16 µm) in vacuum, in a column one cell across of 5 nm cells, TE and TM: the transmittance at 0.7 to 1.3 c/µm, each at its own angle $\sin\theta = k_x/\omega$ (16.6° to 8.8°), against the transfer matrices' (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the exact stack by transfer matrices (mode::multilayer, J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Eqs. 13-16); second order in the cells: TE 2.0e-2, 4.9e-3, 1.2e-3 and TM 2.0e-2, 5.0e-3, 1.3e-3 at 20, 10 and 5 nm (measured)",
            run: fdtd_bloch_multilayer,
        },
        Case {
            id: "fdtd/bloch-bands",
            title: r"FDTD's band structure of a 1D photonic crystal: one period (0.2 µm of $n = 2$, 0.3 µm of vacuum) between Bloch sides at $k = 0.3\pi/\Lambda$, 80 cells a period, its three bands below 2 c/µm from a complex run's probe (a broadband run's peaks, then each band alone by a narrow pulse and the probe's recurrence), against the analytic ones (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\cos k\Lambda = \cos k_1 d_1 \cos k_2 d_2 - \tfrac12(n_1/n_2 + n_2/n_1)\sin k_1 d_1 \sin k_2 d_2$, half the trace of a period's transfer matrix: 0.20139, 1.21847, 1.64523 c/µm; second order in the cells: 5.0e-3, 1.2e-3, 3.1e-4 at 20, 40, 80 cells a period (measured)",
            run: fdtd_bloch_bands,
        },
        Case {
            id: "fdtd/fit-lossless",
            title: r"Catalogue fits for FDTD where the material is transparent: silicon (Li 1980's table) over 1.2 to 1.7 µm and silica (Malitson 1965's Sellmeier formula) over 0.4 to 1.6 µm, each fitted by $\varepsilon_\infty \ge 1$ and undamped Lorentz terms of nonnegative strength (largest relative error in $\varepsilon$ over the band shown)",
            tier: Tier::Analytic,
            source: r"the catalogue's own $\varepsilon(\lambda)$ at 256 wavelengths across the band; nonnegative least squares (C. L. Lawson, R. J. Hanson, Solving Least Squares Problems, SIAM (1995), doi:10.1137/1.9781611971217, Ch. 23); two terms each; measured 6.0e-5 and 6.0e-5",
            run: fdtd_fit_lossless,
        },
        Case {
            id: "fdtd/fit-lossy",
            title: r"A catalogue fit for FDTD where the material absorbs: In$_{0.49}$Ga$_{0.51}$P above its gap (Ferrini 2002's table of n and k) over 0.4 to 0.6 µm, fitted by $\varepsilon_\infty \ge 1$ and damped Lorentz terms of nonnegative strength (largest relative error in $\varepsilon$ over the band shown)",
            tier: Tier::Analytic,
            source: r"the catalogue's own $\varepsilon(\lambda)$ at 256 wavelengths across the band; nonnegative least squares over resonances from half the band's lowest frequency to twice its highest at three dampings each; 8 terms; measured 8.1e-3",
            run: fdtd_fit_lossy,
        },
        Case {
            id: "fdtd/fitted-slab",
            title: r"FDTD with a catalogue fit: a slab of silicon 0.4 µm thick in vacuum, Li 1980's table fitted over 1.2 to 1.7 µm, lit by a pulse at normal incidence in a column of 5 nm cells: the power reflection at 1.3, 1.45 and 1.6 µm against Airy's formula with the catalogue's index (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the slab's exact reflection, $r = r_{12}(1 - e^{2i\delta})/(1 - r_{12}^2 e^{2i\delta})$, $\delta = n k_0 d$, $n$ the catalogue's; second order in the cells toward the fit's 6e-5: 2.9e-2 at 20 nm, 7.3e-3 at 10, 2.0e-3 at 5 (measured)",
            run: fdtd_fitted_slab,
        },
        Case {
            id: "fdtd/smoothing-slab",
            title: r"FDTD's subpixel smoothing at an interface between grid points: a slab of $\varepsilon = 4$, 0.3 µm thick, its faces at 1.0137 and 1.3137 µm, lit by a pulse at normal incidence in a column of cells of 1/160 µm: the complex reflection at 0.8, 1 and 1.2 c/µm at the slab's face against Airy's formula (largest $\lvert \Delta r \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"the slab's exact reflection, $r = r_{12}(1 - e^{2i\delta})/(1 - r_{12}^2 e^{2i\delta})$, $\delta = n k_0 d$; with $\langle\varepsilon\rangle$ along the faces (A. Farjadpour et al., Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972, Eq. 1) second order wherever the faces fall: 5.8e-2, 1.2e-2, 3.0e-3, 6.9e-4 at 1/20 to 1/160 µm; sampled at the values, erratic: 1.6e-1, 1.1e-2, 4.8e-2, 1.8e-2 (measured)",
            run: fdtd_smoothing_slab,
        },
        Case {
            id: "fdtd/smoothing-anisotropic-slab",
            title: r"FDTD's subpixel smoothing of an anisotropic slab: principal values 2, 3 and 4.5 along axes turned from the grid's (every entry of $\varepsilon$ non-zero), the same slab and grid, an $x$-polarized pulse: $r_{xx}$ and $r_{yx}$ at 0.8, 1 and 1.2 c/µm (largest $\lvert \Delta r \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"exact at normal incidence: $D_z = 0$, so $E_x$ and $E_y$ see $\varepsilon_t = \varepsilon_{tt} - \varepsilon_{tz}\varepsilon_{zt}/\varepsilon_{zz}$, the transverse block of Kottke et al.'s $\tau$ (Phys. Rev. E 77, 036611 (2008), doi:10.1103/PhysRevE.77.036611, Eq. 4), and along each of its principal axes the slab is Airy's (the 4 × 4 transfer matrices split in two). photonoxide has no 4 × 4 transfer matrices for oblique incidence. Second order: 2.9e-2, 9.5e-3, 1.2e-3, 3.0e-4 at 1/20 to 1/160 µm; sampled 1.1e-1, 1.0e-2, 3.2e-2, 1.2e-2 (measured)",
            run: fdtd_smoothing_anisotropic_slab,
        },
        Case {
            id: "fdtd/smoothing-oblique",
            title: r"FDTD's subpixel smoothing at interfaces oblique to the grid: layers of $\varepsilon = 12$ and 1, half a period each, the period $1/\sqrt 5$ µm along $(1, 2)/\sqrt 5$, periodic on a square of 1 µm, 128 cells a µm; the lowest mode along the layers with $E$ across them, $\tilde H_z \propto \cos 2\pi(2x - y)$, the off-diagonal entries of $\tilde\varepsilon^{-1}$ at each value of $E$ (`Coupling::Points`): its frequency against the transfer matrices (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the exact layers by their transfer matrices of $(\tilde H_z, \varepsilon^{-1}\partial_s \tilde H_z)$, $\frac12 \operatorname{tr} M_1 M_2 = 1$, 0.885677 c/µm; second order with Farjadpour et al.'s placement, error times $n^2$ $-2.1$, $-2.8$, $-3.3$, $-3.5$ at $n$ = 16 to 128; with Werner and Cary's (`Coupling::Nodes`) first order, error times $n$ $-0.56$, $-0.39$, $-0.28$, $-0.26$, like the mean $\langle\varepsilon\rangle$ ($-0.23$ at 128); sampled, erratic (measured; docs/methods/fdtd.md)",
            run: fdtd_smoothing_oblique,
        },
        Case {
            id: "fdtd/smoothing-oblique-nodes",
            title: r"FDTD's subpixel smoothing at the same oblique layers with the off-diagonal entries at the nodes (`Coupling::Nodes`, the default before 0.5), 128 cells a µm: the mode's frequency against the transfer matrices (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the same exact frequency; first order: each row of $\tilde\varepsilon^{-1}$ mixes cells about different points (G. R. Werner, J. R. Cary, J. Comput. Phys. 226, 1085 (2007), doi:10.1016/j.jcp.2007.05.008, Eq. 39, as A. F. Oskooi, C. Kottke, S. G. Johnson, Opt. Lett. 34, 2778 (2009), doi:10.1364/OL.34.002778, Fig. 1, place them); measured 2.0e-3, against 2.1e-4 with the entries at the points",
            run: fdtd_smoothing_oblique_nodes,
        },
        Case {
            id: "fdtd/smoothing-energy",
            title: r"FDTD's energy with a smoothed tensor: a closed box ($10 \times 9 \times 8$ cells of 50 nm) holding an ellipsoid of the anisotropic crystal above at an angle to the grid, in $\varepsilon = 2$, random D and H, $10^5$ steps: the largest change of $\tfrac12\sum E\cdot D + \tfrac12\sum \tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$, relative to it (shown)",
            tier: Tier::Analytic,
            source: r"with the off-diagonal entries at the nodes $\tilde\varepsilon^{-1}$ on the grid is symmetric (Werner and Cary 2007), so the leapfrog conserves this energy exactly; measured 2.0e-15. At the points it isn't: 6.3e-2 over 300 steps",
            run: fdtd_smoothing_energy,
        },
        Case {
            id: "fdtd/smoothing-contrast",
            title: r"FDTD's smoothed tensor at high contrast: a closed box ($20 \times 18 \times 16$ cells of 50 nm) holding an isotropic ellipsoid at an angle to the grid in vacuum, the off-diagonal entries at the nodes, random D and H at Courant number 0.99: at $\varepsilon = 40$ the largest $\lVert E\rVert$ over $10^5$ steps relative to the first (shown); refused at $\varepsilon = 50$, its $\tilde\varepsilon^{-1}$ on the grid not positive definite",
            tier: Tier::Analytic,
            source: r"symmetric, the leapfrog's energy is conserved, but it bounds the fields only while $\tilde\varepsilon^{-1}$ is positive definite, and Werner and Cary's 2007 scheme isn't at every contrast: here its least eigenvalue turns negative between $\varepsilon = 40$ and 45 ($-4.6 \times 10^{-3}$ at 50), and $\varepsilon = 50$ let through grows by $10^{149}$ in $10^4$ steps. The check is exact (a Cholesky factorization of the coupled block) but sufficient, not sharp: 45 let through stays bounded over $10^5$ steps",
            run: fdtd_smoothing_contrast,
        },
        Case {
            id: "fdtd/smoothing-oskooi",
            title: r"FDTD's subpixel smoothing of anisotropic media, Oskooi et al.'s 2D lattice: ellipses (semi-axes 0.355 and 0.305 of the period, as their inset) of principal values 1.45, 2.81 and 4.98 in 8.49, 8.78 and 11.52, the axes ours; the lowest mode at $k = (\tfrac12, 0)\thinspace 2\pi/a$ on 32 cells a period: the error of $\tau$'s average relative to the smaller of the harmonic mean's and no smoothing's (shown)",
            tier: Tier::Published,
            source: r"A. F. Oskooi, C. Kottke, S. G. Johnson, Opt. Lett. 34, 2778 (2009), doi:10.1364/OL.34.002778, Fig. 2: the new smoothing has the lowest error, often by an order of magnitude. Against the mean of $s = 1$ and $s = 2$ at 64 cells (which converge from either side), 0.159381 c/µm: 2.3e-4 smoothed, 4.5e-3 harmonic mean, 3.7e-3 none; but 2.5e-4 for the mean $\langle\varepsilon\rangle$, which their figure has 6 times the new one's (not reproduced here: at this $k$ and with our axes the mean is as good)",
            run: fdtd_smoothing_oskooi,
        },
        Case {
            id: "fdtd/smoothing-triplets-oblique",
            title: r"FDTD's subpixel smoothing at the oblique layers above ($\varepsilon = 12$ and 1 at 26.6° to the grid, 128 cells a µm) with $\tilde\varepsilon^{-1}$ from each node's eight triplets (`Coupling::Triplets`): the mode's frequency against the transfer matrices (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the same exact frequency, 0.885677 c/µm; G. R. Werner, C. A. Bauer, J. R. Cary, J. Comput. Phys. 255, 436 (2013), doi:10.1016/j.jcp.2013.08.009 (Secs. 4 and 7), each triplet's tensor C. A. Bauer, G. R. Werner, J. R. Cary's (J. Comput. Phys. 230, 2060 (2011), doi:10.1016/j.jcp.2010.12.005) made symmetric. First order, as Werner et al. find every symmetric effective dielectric: error times $n$ $-0.38$, $-0.23$, $-0.15$, $-0.14$ at $n$ = 16 to 128; measured 1.06e-3, about half the nodes' 2.0e-3",
            run: fdtd_smoothing_triplets_oblique,
        },
        Case {
            id: "fdtd/smoothing-triplets-order",
            title: r"The order of convergence at the oblique layers with `Coupling::Triplets`, from 64 to 128 cells a µm: $\log_2$ of the ratio of the errors (shown)",
            tier: Tier::Analytic,
            source: r"against the transfer matrices; first order, as Werner, Bauer and Cary (2013) find for every symmetric effective dielectric at sharp interfaces (their Figs. 4, 7, 8: second order at coarse grids, first beyond a resolution that falls as the contrast rises); measured 1.15",
            run: fdtd_smoothing_triplets_order,
        },
        Case {
            id: "fdtd/smoothing-bauer-order",
            title: r"The order of convergence at the oblique layers with Bauer, Werner and Cary's triplet tensors as they are, not made symmetric (a check, not a `Coupling`: not stable), from 64 to 128 cells a µm (shown)",
            tier: Tier::Analytic,
            source: r"C. A. Bauer, G. R. Werner, J. R. Cary, J. Comput. Phys. 230, 2060 (2011), doi:10.1016/j.jcp.2010.12.005: exact for constant fields at a plane interface, hence second order, but not symmetric; error times $n^2$ $-5.7$, $-5.7$, $-6.3$, $-6.9$ at 16 to 128 cells, 4.2e-4 at 128; measured order 1.87. Making them symmetric is what costs the order",
            run: fdtd_smoothing_bauer_order,
        },
        Case {
            id: "fdtd/smoothing-triplets-contrast",
            title: r"`Coupling::Triplets` at high contrast: the closed box of `fdtd/smoothing-contrast` with the ellipsoid at $\varepsilon = 100$, where the nodes' $\tilde\varepsilon^{-1}$ isn't positive definite, random D and H at Courant number 0.99, $10^5$ steps: the largest change of $\tfrac12\sum E\cdot D + \tfrac12\sum \tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$ relative to it (shown), and $\lVert E\rVert$ bounded",
            tier: Tier::Analytic,
            source: r"Werner, Bauer and Cary (2013), Sec. 4: $\tilde\varepsilon^{-1}$ the mean of eight block-diagonal matrices of symmetric positive-definite 3 × 3 blocks, so symmetric and positive definite at any contrast, and the leapfrog's energy, conserved, bounds the fields; measured: energy to 1e-14 and $\lVert E\rVert$ at most 1.1 times its first",
            run: fdtd_smoothing_triplets_contrast,
        },
        Case {
            id: "fdtd/smoothing-triplets-lattices",
            title: r"`Coupling::Triplets` over long runs where Farjadpour et al.'s placement grows: one period of the elliptical holes in $\varepsilon = 12$ and one of Oskooi et al.'s anisotropic lattice, 16 cells a period, random D and H, $10^5$ steps each: the larger change of the leapfrog's energy relative to it (shown)",
            tier: Tier::Analytic,
            source: r"symmetric, so the energy is conserved to round-off; `Coupling::Points` grows by $10^3$ over $4 \times 10^5$ steps in the holes and by $10^{16}$ within $2 \times 10^4$ in the anisotropic lattice (docs/methods/fdtd.md); measured 4.3e-15 and 5.0e-15",
            run: fdtd_smoothing_triplets_lattices,
        },
        Case {
            id: "fdtd/smoothing-wc07-growth",
            title: r"Werner and Cary's 2007 scheme (`Coupling::Nodes`, let through unchecked) unstable at high contrast: a square lattice of isotropic discs of $\varepsilon = 100$, radius $0.37a$, in vacuum, TE, 32 cells a period, from random fields: the rate $\gamma$ at which $\lVert E\rVert$ grows as $e^{\gamma t}$, c/a (shown); 64 cells too, and `Coupling::Triplets` bounded at both",
            tier: Tier::Published,
            source: r"G. R. Werner, C. A. Bauer, J. R. Cary, J. Comput. Phys. 255, 436 (2013), doi:10.1016/j.jcp.2013.08.009, Sec. 5: at contrast 100, $\gamma \approx 3c/a$ on $32^2$ cells and $\approx 6c/a$ on $64^2$; measured 2.85 and 6.11. Their contrast-60 case ($\gamma \approx 0.5c/a$ on $64^2$) isn't reproduced: no growth here over $3000a/c$ (the disc's place on the grid isn't given). The triplets stay within 1.1 times the first $\lVert E\rVert$",
            run: fdtd_smoothing_wc07_growth,
        },
        Case {
            id: "fdtd/smoothing-crystal",
            title: r"`Coupling::Triplets` in Bauer, Werner and Cary's 3D photonic crystal: an orthorhombic lattice ($1.2 \times 1.5 \times 1.8$ µm) of ellipsoids (semi-axes 0.45, 0.60, 0.75 µm, turned) of an anisotropic tensor (principal values 8, 10, 12, turned) in vacuum, 48 cells along each lattice vector: the nine lowest bands at $k = 0$ against their Table 1 (largest relative difference shown)",
            tier: Tier::Published,
            source: r"C. A. Bauer, G. R. Werner, J. R. Cary, J. Comput. Phys. 230, 2060 (2011), doi:10.1016/j.jcp.2010.12.005, Sec. 4.3 and Table 1 (Richardson's extrapolation of their second-order frequency-domain results at 96 and 128 cells); their tensor's rotations read as passive, the one reading of the two under which all nine bands converge to the table. Measured 1.1e-3 to 1.8e-3 by band with the triplets, 1.4e-3 to 2.3e-3 with the nodes",
            run: fdtd_smoothing_crystal,
        },
        Case {
            id: "fdtd/smoothing-crystal-order",
            title: r"The same crystal with Bauer et al.'s triplet tensors as they are (not symmetric; a run of 400 µm/c, before round-off grows): the order of convergence of the nine bands' RMS error against their Table 1 from 32 to 48 cells (shown)",
            tier: Tier::Published,
            source: r"Bauer, Werner and Cary (2011), Fig. 8: second order; measured 1.98, the RMS error 1.2e-3 at 48 cells",
            run: fdtd_smoothing_crystal_order,
        },
        Case {
            id: "fdtd/monitor-transforms",
            title: r"FDTD's transform monitors: $E$ over a box of $12 \times 7 \times 10$ values in a closed lossy box ($\varepsilon = 2.1$, $\sigma = 2$/µm, $16 \times 14 \times 12$ cells of 50 nm) after a pulse, at two frequencies, against $\sum_n E(n\Delta t) e^{i\omega n\Delta t}\Delta t$ summed by hand (largest difference relative to the largest transform shown)",
            tier: Tier::Analytic,
            source: r"the same sum in the same order; measured 0, the same bits",
            run: fdtd_monitor_transforms,
        },
        Case {
            id: "fdtd/monitor-flux",
            title: r"FDTD's flux monitors: the same lossy box, the flux through a whole plane and out of a box around the source at 1.55 µm, from the transforms divided by the pulse's spectrum, against FDFD's field for a unit current at the leapfrog's frequency (largest difference relative to the plane's flux shown)",
            tier: Tier::Analytic,
            source: r"once the fields have died away, the transforms solve FDFD's equations at $\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$, $\tilde H = \nabla \times E/(i\tilde\omega)$ exactly, so their flux $\tfrac12 \operatorname{Re}(E \times \tilde H^*)$ on Yee's grid is FDFD's; measured 1.2e-11",
            run: fdtd_monitor_flux,
        },
        Case {
            id: "fdtd/monitor-flux-box",
            title: r"FDTD's flux out of a closed box with no source and no loss in it (a block of $\varepsilon = 4$; outside it $\sigma = 3$/µm and a pulse), at two frequencies, run until the fields have died away (largest net flux relative to the largest face's shown)",
            tier: Tier::Analytic,
            source: r"summation by parts of $\sum \tilde H^*\cdot(\nabla \times E) - \sum E\cdot(\nabla \times \tilde H)^*$ over the box, each value weighted by its share of it, leaves only the faces' fluxes, and the sums are imaginary in a lossless region without sources; measured 1.5e-15",
            run: fdtd_monitor_flux_box,
        },
        Case {
            id: "fdtd/monitor-modes",
            title: r"FDTD's mode monitors: a rectangular guide ($\varepsilon = 12$, $0.4 \times 0.3$ µm in 2.1) in a closed lossy box ($\sigma = 4$/µm, $16 \times 14 \times 24$ cells of 50 nm), its mode launched forward by a pulse: the forward and backward amplitudes behind and ahead of the source at 1.55 µm, divided by the source's amplitude, against FDFD's for its own mode source (largest difference relative to the largest amplitude shown)",
            tier: Tier::Analytic,
            source: r"the monitors project the transforms by FDFD's own Lorentz reciprocity form, and the transforms are FDFD's fields at $\tilde\omega$; measured 1.5e-11",
            run: fdtd_monitor_modes,
        },
        Case {
            id: "fdtd/monitor-guides",
            title: r"FDTD's S-parameters in 2D: a guide of $\varepsilon = 12$ and 0.25 µm in air (cells of 50 nm, CPMLs of 10 cells), straight and bent by 90° at a sharp corner, its mode launched by a pulse: transmission into the output's mode and reflection into the input's at 1.55 µm, against FDFD with its PMLs (largest difference shown)",
            tier: Tier::Analytic,
            source: r"FDFD on the same grid at the leapfrog's frequency; the CPML ($\kappa = 1$, $\alpha = 0$) and FDFD's PML differ in discrete time: straight, $\lvert T \rvert$ 0.9999991 against 1.0000002; bent, 0.315408 against 0.315415, $\lvert \Delta R \rvert$ 1.4e-5; measured 5.8e-5, the straight guide's reflections (1.3e-4 by both)",
            run: fdtd_monitor_guides,
        },
        Case {
            id: "fdtd/harmonic-inversion",
            title: r"Harmonic inversion by filter diagonalization: 400 samples of three decaying terms, two of them 0.01 c/µm apart where the Fourier transform resolves 0.025, the window 0.9 to 1.1 c/µm (largest error of their frequencies, decay rates and relative amplitudes shown)",
            tier: Tier::Analytic,
            source: r"V. A. Mandelshtam, H. S. Taylor, J. Chem. Phys. 107, 6756 (1997), doi:10.1063/1.475324: the generalized eigenproblem $U^{(1)}B = u\thinspace U^{(0)}B$ of their Eq. 25 is exact for a signal of finitely many terms, so they are recovered to round-off; measured 2.1e-12",
            run: fdtd_harmonic_inversion,
        },
        Case {
            id: "fdtd/cavity-resonances",
            title: r"FDTD's resonances by harmonic inversion: a 2D cavity with walls ($24 \times 18$ cells of 50 nm, $\varepsilon = 2$, $\sigma = 0.05$/µm) after a pulse, 4000 steps of $E_z$ at one point, 0.4 to 1.2 c/µm: the resonances found against the leapfrog's own (largest $\lvert u - u_\text{exact} \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"each mode $\sin(m\pi i/n_x)\sin(n\pi j/n_y)$ evolves as $u^n$ with $u^2 - (1 + c_a - c_b\Delta t\lambda)u + c_a = 0$, $\lambda = (4/\Delta^2)(\sin^2\frac{m\pi}{2n_x} + \sin^2\frac{n\pi}{2n_y})$; 4 found, to 3.4e-15",
            run: fdtd_cavity_resonances,
        },
        Case {
            id: "fdtd/slab-resonance",
            title: r"FDTD's resonance and Q in 1D: a slab of $\varepsilon = 4$, 0.5 µm thick, in vacuum (CPMLs of 30 cells), its second resonance by harmonic inversion on cells of 20, 10 and 5 nm at Courant number 0.5, against the continuum's $\omega = (2\pi + i \ln r)/(nL)$, $r = 1/3$ (1 c/µm, $Q = 2.86$): the order of convergence of the frequency and of the decay rate (the smaller shown)",
            tier: Tier::Analytic,
            source: r"the slab's poles $r^2 e^{2inkL} = 1$; second order as the scheme's dispersion: relative errors 1.8e-3, 4.5e-4 and 1.1e-4 of the frequency, 1.4e-2, 3.6e-3 and 8.9e-4 of the decay rate: orders 2.00 and 2.01",
            run: fdtd_slab_resonance,
        },
        Case {
            id: "fdtd/mie-table",
            title: r"Mie's series against Mie's own Table I: $\mathfrak{a}_1 = a_1/2\alpha^3$ for a perfectly conducting sphere and gold spheres in water at 420 to 650 nm, $\alpha^2$ from 0 to 2.5, 66 entries (the median $\lvert \Delta\mathfrak{a}_1 \rvert$ shown)",
            tier: Tier::Published,
            source: r"G. Mie, Ann. Phys. 330, 377 (1908), doi:10.1002/andp.19083300302, Table I with gold's $m'^2$ from p. 417, his Eq. 55 in photonoxide's convention; 61 of 66 within 0.016 of his three digits, median 3.0e-3. The other five, near gold's resonance where his series in $\alpha^2$ by hand converge worst, differ from his by 0.04 to 0.40 and agree with $a_1$ from the closed forms of $\psi_1$ and $\xi_1$ to 1e-13",
            run: fdtd_mie_table,
        },
        Case {
            id: "fdtd/mie-balance",
            title: r"Mie's series: a lossless sphere takes from the wave what it scatters, $Q_\text{ext} = Q_\text{sca}$, for $\alpha$ from 0.1 to 1000 and $m$ from 1.05 to 3.5 (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"energy conservation: Mie's §26, his parts II and III of the flux through a large sphere, $\operatorname{Re}(a_\nu) = \lvert a_\nu \rvert^2$ and the same for $b_\nu$ when $m$ is real; round-off",
            run: fdtd_mie_balance,
        },
        Case {
            id: "fdtd/mie-terms",
            title: r"Mie's series converges in the number of terms: $Q_\text{ext}$ of the first $\alpha + 4\alpha^{1/3} + 10$ terms against the converged sum, for $\alpha$ = 5, 50 and 200 (largest error shown)",
            tier: Tier::Analytic,
            source: r"the terms fall faster than exponentially once $\nu$ passes $\alpha$ (Mie's §15); half as many terms are off by more than 1e-2; measured 7.0e-14, round-off",
            run: fdtd_mie_terms,
        },
        Case {
            id: "fdtd/mie-sphere",
            title: r"FDTD against Mie's series: a smoothed sphere of $\varepsilon = 4$ and radius 1 µm in vacuum, a TF/SF plane wave, its scattering cross-section from a flux box over $\alpha$ = 1 to 3, on 8 and 16 cells a radius: the order of convergence of the mean relative error",
            tier: Tier::Analytic,
            source: r"G. Mie, Ann. Phys. 330, 377 (1908), doi:10.1002/andp.19083300302, Eq. 55 and §26's part III; mean errors 2.8e-2 and 7.2e-3 (6, 12 and 20 cells: 5.1e-2, 1.2e-2, 4.7e-3), second order as the scheme's dispersion",
            run: fdtd_mie_sphere,
        },
        Case {
            id: "fdtd/mie-sphere-staircase",
            title: r"The same sphere without smoothing, $\varepsilon$ sampled at each value of E, 16 cells a radius: the mean relative error of the scattering cross-section",
            tier: Tier::Analytic,
            source: r"Mie's series; the staircase's error is irregular in the grid, 2.6e-2, 1.7e-2, 8.1e-3, 6.0e-3 and 4.8e-3 on 6, 8, 12, 16 and 20 cells, not smaller than the smoothed sphere's here because the dispersion's dominates at the top of the band",
            run: fdtd_mie_sphere_staircase,
        },
        Case {
            id: "fdtd/mie-drude",
            title: r"FDTD against Mie's series for a damped Drude metal sphere ($f_p$ = 0.5 c/µm, $\gamma/2\pi$ = 0.2 c/µm, radius 1 µm: Re ε from −2.8 to 0.3), sampled at each value of E, 16 cells a radius: the mean relative error of the scattering and absorption cross-sections, from flux boxes outside and inside the TF/SF box",
            tier: Tier::Analytic,
            source: r"Mie's series with the leapfrog's permittivity at each frequency; first order, as a staircased surface: 3.0e-2 on 8 cells, 1.6e-2 on 16",
            run: fdtd_mie_drude,
        },
        Case {
            id: "fdtd/meep-pml-rates",
            title: r"Oskooi et al.'s PML convergence: a point source of $E_z$ in 2D vacuum, 20 cells a wavelength, a cell of 4 wavelengths inside CPMLs of L wavelengths ($\kappa = 1$, $\alpha = 0$, round-trip reflection $10^{-15}$), $\sigma$ graded as $(x/L)^d$: the rate at which $\lvert E^{L+1} - E^L \rvert^2$ a wavelength from the source falls with L at L = 4, against $1/L^{2d+4}$ for d = 1, 2 and 3 (largest difference of the rates shown)",
            tier: Tier::Published,
            source: r"A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010), doi:10.1016/j.cpc.2009.11.008, Section 4.2 and Fig. 8: field convergence as $1/L^6$, $1/L^8$ and $1/L^{10}$; measured 6.14, 8.22 and 10.32 at L = 4 (each slope between the differences' midpoints), approaching from above: 6.03, 8.05 and 10.08 at L = 8, the `pml_oskooi` example",
            run: fdtd_meep_pml_rates,
        },
        Case {
            id: "fdtd/ring-wronskian",
            title: r"A 2D ring's exact resonances: the Bessel functions $J_m$ and $Y_m$ of complex argument by their power series, orders 0 to 11 across $\lvert z \rvert \le 15$: the Wronskian $J_m Y_m' - J_m' Y_m$ against $2/\pi z$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"M. Abramowitz, I. A. Stegun, Handbook of Mathematical Functions, NBS (1964), Eqs. 9.1.10, 9.1.11 and 9.1.16; the series lose about $e^{\lvert z \rvert}/2\pi\lvert z \rvert$ of their precision to cancellation; measured 1.0e-10",
            run: fdtd_ring_wronskian,
        },
        Case {
            id: "fdtd/ring-resonances",
            title: r"FDTD's resonances of a dielectric ring: Oskooi et al.'s Fig. 11 ring ($\varepsilon = 11.56$) between radii 1 and 2 µm in vacuum, 2D, $E_z$, smoothed, 20 cells a µm (CPMLs of 2 µm, 1 µm clear), a pulse at 0.15 c/µm, 300 µm/c of one point's field by harmonic inversion: the three resonances from 0.1 to 0.2 c/µm against the exact ones (largest relative error in frequency shown)",
            tier: Tier::Analytic,
            source: r"the zeros of the determinant of $E_z$'s and $\partial E_z/\partial r$'s continuity with $J_m$ in the hole, $J_m$ and $Y_m$ in the ring and the outgoing $H^{(1)}_m$ outside, by the secant method: m = 3, 4 and 5 at 0.118192, 0.147431 and 0.175779 c/µm, Q 77.26, 343.92 and 1634.2 (Oskooi et al. 2010 give no numbers for their ring, nor its size); measured 2.8e-4, 5.2e-4 and 7.7e-4, Q 77.36, 345.21 and 1646.2 (7.3e-3)",
            run: fdtd_ring_resonances,
        },
        Case {
            id: "fdtd/ring-order",
            title: r"The ring's resonances on 10 and 20 cells a µm: the order at which the frequencies' and the Q factors' errors fall (the smallest of the six shown)",
            tier: Tier::Analytic,
            source: r"second order: smoothed, $E_z$ sees the mean $\varepsilon$ across each face, the right average for a field along an interface (A. Farjadpour et al., Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972); measured 1.99 to 2.02 in frequency, 1.95 to 2.05 in Q",
            run: fdtd_ring_order,
        },
        Case {
            id: "fdtd/fdfd-band-2d",
            title: r"FDTD against FDFD over a band in 2D: a guide of $\varepsilon = 12$ and 0.3 µm in air (E in the plane; a square of 3.5 µm in 50 nm cells, CPMLs of 0.5 µm), straight, and bent by 90° around a quarter circle of 1 µm, its mode launched by one pulse at 1.55 µm: $S_{21}$ and $S_{11}$ at five frequencies across ±5 % from the run's mode monitors, against `Solver3d` at each one's leapfrog frequency (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"FDFD on the same grid at $\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$ for the run's own currents, the mode source's J and M each times its transform at the times it is applied (the carrier's mode at every frequency, as the run launches it); $S_{21} = a^+_\text{out}/a^+_\text{in}$ and $S_{11} = a^-_\text{in}/a^+_\text{in}$ on the same planes. The CPML ($\kappa = 1$, $\alpha = 0$) and FDFD's PML differ in discrete time: measured 3.2e-5, the straight guide's $\lvert S_{11} \rvert$ 9.0e-5 against 8.4e-5; the bend's $\lvert S_{21} \rvert$ about 0.995",
            run: fdtd_fdfd_band_2d,
        },
        Case {
            id: "fdtd/fdfd-band-2d-fine",
            title: r"The same 2D guides on 25 nm cells (the same CPMLs of 0.5 µm, 20 cells): the largest $\lvert \Delta S \rvert$ over the band against FDFD (shown)",
            tier: Tier::Analytic,
            source: r"FDFD on the same grid at the leapfrog's frequency; the CPML's difference from FDFD's PML lies in what each reflects, which falls as the same thickness takes more cells: 3.2e-5 on 50 nm cells, 1.7e-7 on 25",
            run: fdtd_fdfd_band_2d_fine,
        },
        Case {
            id: "fdtd/fdfd-band-3d-strip",
            title: r"FDTD against FDFD over a band in 3D: a strip of $\varepsilon = 12$, $0.4 \times 0.25$ µm, in $\varepsilon = 2.1$ (50 nm cells, $24 \times 20 \times 28$ inside CPMLs of 8 cells), its fundamental mode launched by one pulse at 1.55 µm: $S_{21}$ over 0.95 µm of guide and $S_{11}$ at the carrier and 5 % either side, against `Solver3d` at each one's leapfrog frequency (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"FDFD on the same grid at the leapfrog's frequency for the run's own currents, as `fdtd/fdfd-band-2d`; the CPML against FDFD's PML, converging as it thickens: 2.2e-4 with 8 cells, 6.5e-5 with 10 and 7.9e-6 with 12 ($\lvert S_{11} \rvert$, what the CPML reflects, 4.2e-4, 8.7e-5 and 4.2e-5); measured 2.2e-4",
            run: fdtd_fdfd_band_3d_strip,
        },
        Case {
            id: "fdtd/fdfd-band-3d-bend",
            title: r"FDTD against FDFD over a band in 3D: the same strip along x bent by 90° into y around a quarter circle of 0.6 µm (50 nm cells, $34 \times 34 \times 16$ inside CPMLs of 8 cells), one pulse at 1.55 µm: $S_{21}$ into the output's mode and $S_{11}$ at the carrier and 5 % either side, against `Solver3d` at each one's leapfrog frequency (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"FDFD on the same grid at the leapfrog's frequency for the run's own currents, as `fdtd/fdfd-band-2d`; $\lvert S_{21} \rvert$ 0.78 to 0.85 and $\lvert S_{11} \rvert$ about 0.01 by both; measured 1.9e-4, the CPML against FDFD's PML",
            run: fdtd_fdfd_band_3d_bend,
        },
        Case {
            id: "fdtd/fdfd-smoothed",
            title: r"FDTD with subpixel smoothing against FDFD: the 2D bend of `fdtd/fdfd-band-2d` smoothed by Werner, Bauer and Cary's triplets (`Coupling::Triplets`) for FDTD, averaged as FDFD averages it for FDFD, on 50 and 25 nm cells: the order at which their largest $\lvert \Delta S \rvert$ over the band falls (shown)",
            tier: Tier::Analytic,
            source: r"both converge to the continuum's S; at the bend the two averages differ, FDFD's (harmonic along each component and arithmetic across, from samples) first order at its curved faces, as are the triplets (G. R. Werner, C. A. Bauer, J. R. Cary, J. Comput. Phys. 255, 436 (2013), doi:10.1016/j.jcp.2013.08.009): $\lvert \Delta S \rvert$ 0.56, 0.31 and 0.16 on 50, 25 and 12.5 nm cells, mostly $S_{21}$'s phase over 3.6 µm of guide ($\lvert S_{21} \rvert$ within 7e-4); measured 0.85 from 50 to 25 nm, 0.97 from 25 to 12.5",
            run: fdtd_fdfd_smoothed,
        },
        Case {
            id: "fdtd/adjoint-modes",
            title: r"FDTD adjoint gradients in 3D: a strip guide (core $\varepsilon = 6$, $0.3 \times 0.3$ µm, in vacuum; $36 \times 20 \times 20$ cells of 50 nm, CPMLs of 6) with a design of $3 \times 2 \times 2$ cells in its core at uneven densities ($\varepsilon$ 1 to 6), its mode launched by a pulse; $F = \lvert a_+(f_0)\rvert^2 + \lvert a_+(f_1)\rvert^2 - \frac12\lvert a_-(f_0)\rvert^2$ at 1 and 1.06 c/µm: the gradient with respect to every cell's density against fourth-order central differences ($\delta = 10^{-3}$) (largest difference relative to the largest gradient shown)",
            tier: Tier::Analytic,
            source: r"the run's transforms solve $K x = b$ exactly; $K^T D = D K$, $D$ the CPMLs' stretches, so the adjoint is a run; $\partial F/\partial\varepsilon = \sum 2\operatorname{Re}(i\tilde\omega\thinspace\hat E_\text{adj}\hat E)$ (G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288; C. M. Lalau-Keraly et al., Opt. Express 21, 21693 (2013), doi:10.1364/OE.21.021693, Eq. 5); the mode's complex profile whole (dropped, the gradient is wrong by orders of magnitude); measured 5.5e-10, and 0.49 with the imaginary parts dropped",
            run: fdtd_adjoint_modes,
        },
        Case {
            id: "fdtd/adjoint-flux",
            title: r"The same strip with the flux through a plane ahead (its window reaching into the CPMLs) at 1 c/µm plus $\lvert E_y\rvert^2$ at a point ahead at 1.06 c/µm (shown)",
            tier: Tier::Analytic,
            source: r"as above; the flux $\frac12\operatorname{Re}(E \times \tilde H^*)$ gives sources of both $J$ and $M$; measured 1.5e-9",
            run: fdtd_adjoint_flux,
        },
        Case {
            id: "fdtd/adjoint-fdfd",
            title: r"FDTD's adjoint gradient against FDFD's: a rectangular guide ($\varepsilon = 12$, $0.4 \times 0.3$ µm in 2.1) in a closed box of a lossy medium ($\sigma = 4$/µm, $16 \times 14 \times 24$ cells of 50 nm, 64 steps a period of 1.55 µm), a design of $3 \times 3 \times 3$ cells across its core's edge, lossless: the gradient of the forward power ahead with respect to the permittivity at its 144 values of E, divided by the source's amplitude squared, against `Solver3d`'s adjoint on the same values at $\tilde\omega$ (largest difference relative to the largest gradient shown)",
            tier: Tier::Analytic,
            source: r"the transforms are FDFD's fields at $\tilde\omega$ with $\varepsilon + i\sigma\cos(\omega\Delta t/2)/\tilde\omega$, exactly, and FDFD's adjoint is $\lambda = V A^{-1} V^{-1}\partial F/\partial u$; measured 7.5e-14",
            run: fdtd_adjoint_fdfd,
        },
        Case {
            id: "fdtd/adjoint-fdfd-cpml",
            title: r"The same box open along the guide, CPMLs of 8 cells against FDFD's PMLs of the same grading, at 64, 128 and 256 steps a period: the difference at 256 steps (shown)",
            tier: Tier::Analytic,
            source: r"the CPML ($\kappa = 1$, $\alpha = 0$; J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000), see docs/methods/fdtd.md for its DOI) is FDFD's PML to first order in $\Delta t$: 1.3e-4, 5.1e-5 and 2.3e-5 at 64, 128 and 256 steps a period (measured)",
            run: fdtd_adjoint_fdfd_cpml,
        },
        Case {
            id: "fdtd/adjoint-slab",
            title: r"FDTD's adjoint gradient against a closed form: a slab of $\varepsilon = 4$, 0.3 µm thick, in vacuum, at 1 c/µm (1D, cells of 1/160 µm, its faces on nodes), $F = \lvert\hat E_x\rvert^2$ behind it: the gradient summed over the slab's cells per unit incident $\lvert\hat E_x\rvert^2$, against $dT/d\varepsilon$ by Airy's formula (relative error shown)",
            tier: Tier::Analytic,
            source: r"$T = 1/(1 + \frac{(\varepsilon - 1)^2}{4\varepsilon}\sin^2(\sqrt\varepsilon k_0 d))$, differentiated; the grid's dispersion makes the difference second order: 1.2e-2, 2.9e-3 and 7.2e-4 on cells of 1/40, 1/80 and 1/160 µm (measured)",
            run: fdtd_adjoint_slab,
        },
        Case {
            id: "fdtd/adjoint-slab-order",
            title: r"The same: the order of convergence from 1/80 to 1/160 µm (shown to two decimals)",
            tier: Tier::Analytic,
            source: r"second order, as the scheme; 2.06 from 1/40 to 1/80 (measured)",
            run: fdtd_adjoint_slab_order,
        },
        Case {
            id: "fdtd/adjoint-backward-mode",
            title: r"Lalau-Keraly et al.'s adjoint field: in the closed lossy box above, the adjoint run's transforms over the design for $F = \lvert a_+\rvert^2$ against a run launching the mode backwards from the monitor's plane, $\hat E_\text{adj} / (\bar a\thinspace\hat E_\text{back})$ against $\Delta V/4$ (largest relative difference shown)",
            tier: Tier::Published,
            source: r"C. M. Lalau-Keraly, S. Bhargava, O. D. Miller, E. Yablonovitch, Opt. Express 21, 21693 (2013), doi:10.1364/OE.21.021693, Eqs. 8-9: the adjoint of a mode's transmission is the mode sent backwards into the device, its amplitude the forward overlap's conjugate; with FDFD's Lorentz form, $\Delta V/(4ik_0)$ in FDFD's units, $\Delta V/4$ in a run's; measured 5.5e-13",
            run: fdtd_adjoint_backward_mode,
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
        Case {
            id: "components/ring-all-pass-bogaerts",
            title: r"Components: the all-pass ring's closed form ($\kappa^2$ = 0.1 and 0.02, 62.8 µm, $n_g = 4.2$, 3 dB/cm), 1.54 to 1.56 µm, against Bogaerts's through field and against its netlist, a coupler fed back through a waveguide, solved as a circuit (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eq. 1: $e^{i(\pi + \phi)} (a - r e^{-i\phi}) / (1 - r a e^{i\phi})$",
            run: components::ring_all_pass,
        },
        Case {
            id: "components/ring-add-drop-bogaerts",
            title: r"Components: the add-drop ring's closed form ($\kappa_1^2 = 0.1$, $\kappa_2^2 = 0.05$, 62.8 µm), 1.54 to 1.56 µm, against Bogaerts's through and drop powers and against its netlist of two couplers and two half rings (largest difference shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eqs. 5 and 6",
            run: components::ring_add_drop,
        },
        Case {
            id: "components/ring-extremes-bogaerts",
            title: r"Components: the rings' through and drop powers on resonance ($\phi = 2\pi m$) and off it ($\phi = 2\pi m + \pi$), all-pass and add-drop, $\kappa^2$ = 0.05 and 0.2 (largest difference shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eqs. 11 to 16, e.g. $R_\text{min} = (r - a)^2 / (1 - r a)^2$",
            run: components::ring_extremes,
        },
        Case {
            id: "components/ring-linewidth-bogaerts",
            title: r"Components: the full width at half maximum measured on the rings' spectra (all-pass through, add-drop drop; $\kappa^2$ = 0.02 and 0.05, 62.8 µm, 3 dB/cm) against Bogaerts's formulas (largest relative difference shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eqs. 7 and 8, $(1 - ra)\lambda^2 / (\pi n_g L \sqrt{ra})$: a Lorentzian line, $\cos \phi \approx 1 - \phi^2 / 2$ across it, good to about $(1 - ra)^2$, 8e-4 at most here",
            run: components::ring_linewidth,
        },
        Case {
            id: "components/ring-fsr-fdfd",
            title: r"Components: the free spectral range of jobs/ring-fdfd.toml's all-pass ring (radius 2 µm, 2D FDFD by the effective index method, 25 nm grid), from its two resonances near 1.52 and 1.567 µm (nm shown)",
            tier: Tier::Published,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eq. 9, $\lambda^2 / (n_g L)$ at the resonances' midpoint, $n_g$ of the exact bent slab of the same plane (radial shooting, Marcuse 1971, doi:10.1002/j.1538-7305.1971.tb02620.x); the tolerance is the 25 nm grid's",
            run: components::ring_fsr_fdfd,
        },
        Case {
            id: "components/mzi-closed-form",
            title: r"Components: a Mach-Zehnder interferometer netlist of two directional couplers (17 and 23 µm) and arms of 150 and 50 µm (3 dB/cm), 1.54 to 1.56 µm, against its transfer matrices (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"$C_2 \operatorname{diag}(t_\text{lower}, t_\text{upper})\thinspace C_1$, each coupler's $C$ its through and across fields",
            run: components::mzi_netlist,
        },
        Case {
            id: "components/mzi-unitarity",
            title: r"Components: the same interferometer without loss, 1.54 to 1.56 µm: $S^\dagger S$ against $I$ (largest entry of the difference shown)",
            tier: Tier::Analytic,
            source: r"a netlist of lossless components is lossless, $S^\dagger S = I$",
            run: components::mzi_unitarity,
        },
        Case {
            id: "components/unitarity",
            title: r"Components: the lossless waveguide, bend, coupler, directional coupler, all-pass and add-drop rings, 1.54 to 1.56 µm: $S^\dagger S$ against $I$ (largest entry of the difference shown)",
            tier: Tier::Analytic,
            source: r"a lossless component conserves power, $S^\dagger S = I$",
            run: components::unitarity,
        },
        Case {
            id: "components/reciprocity",
            title: r"Components: every first component, lossy, and the 1 x 2 and 2 x 2 MMIs, 1.54 to 1.56 µm: $S$ against $S^\mathsf{T}$ (largest $\lvert S_{qp} - S_{pq} \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"reciprocity, $S = S^\mathsf{T}$, as the circuits' conventions state it (docs/design/components.md)",
            run: components::reciprocity,
        },
        Case {
            id: "components/passivity",
            title: r"Components: the same components: the largest singular value of $S$ (shown), at most 1",
            tier: Tier::Analytic,
            source: r"a passive component gains no power: every singular value of $S$ at most 1, the lossless ones exactly 1",
            run: components::passivity,
        },
        Case {
            id: "components/derivatives",
            title: r"Components: the closed forms' exact parameter derivatives $\partial S / \partial \theta$ (by dual numbers) against central differences (largest difference relative to the derivative's largest entry shown)",
            tier: Tier::Analytic,
            source: r"$(S(\theta + h) - S(\theta - h)) / 2h$, the best of $h/\theta$ = 1e-5 to 1e-8: its truncation and round-off, about 1e-8",
            run: components::derivatives,
        },
        Case {
            id: "components/directional-coupler-power",
            title: r"Components: a directional coupler's power across and through from its supermodes ($\Delta n$ from $C = 0.04$ per µm), 5 to 100 µm long, 1.54 to 1.56 µm (largest difference shown)",
            tier: Tier::Published,
            source: r"L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Eqs. 4.1 to 4.3: $\sin^2(\pi \Delta n L / \lambda)$ and $\cos^2$; the round-off of phases up to 1000 rad, about 1e-13",
            run: components::coupler_power,
        },
        Case {
            id: "components/mmi-beat-length-soldano",
            title: r"Components: the beat length $L_\pi$ of a 3 µm wide multimode section on 220 nm SOI at 1.55 µm, seen from above (TM across it), from its two lowest exact slab modes (µm shown)",
            tier: Tier::Published,
            source: r"L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995), doi:10.1109/50.372474, Eq. 6, $4 n_r W_e^2 / 3\lambda_0$ with Eq. 4's effective width; the paraxial expansion's next term is 1 %, 0.23 µm, the tolerance about twice that",
            run: components::mmi_beat_length,
        },
        Case {
            id: "components/mmi-fdfd",
            title: r"Components: the 1 x 2 MMI of jobs/mmi-fdfd.toml (3 x 8.55 µm, 0.5 µm guides) at 1.55 µm by guided-mode propagation against 2D FDFD of the same plane (20 nm grid): the power in each output (the first shown)",
            tier: Tier::Analytic,
            source: r"L. B. Soldano, E. C. M. Pennings (1995), doi:10.1109/50.372474, Eqs. 8 to 12 with the exact slab modes, against the FDFD solver; what the guided modes leave out (radiation modes, the faces' reflections) sets the tolerance",
            run: components::mmi_fdfd,
        },
        Case {
            id: "components/mzi-neff-dwivedi-470",
            title: r"Components: measured Mach-Zehnder interferometers of a 470 x 211 nm silicon wire in oxide (the SEM's cross-section, a rectangle; Hadley's equations, about 10 nm grids), $\Delta L$ designed for the drawn 450 x 215 nm wire, two ideal splitters, the spectrum read as the paper reads it: $n_\text{eff}$ at 1550 nm from the $m$ = 15 interferometer's peak, $n_\text{eff} L = m \lambda$ (Eq. 3), carried to 1550 nm by $n_g$",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 2.355 ± 0.002; the tolerance is their Eq. 5 with ±20 nm of width and ±5 nm of thickness (their Fig. 1's process), by the solver's derivatives, plus that uncertainty",
            run: crate::circuit::components::measured::dwivedi_neff_470,
        },
        Case {
            id: "components/mzi-ng-dwivedi-470",
            title: r"Components: measured Mach-Zehnder interferometers of a 470 x 211 nm silicon wire in oxide (the SEM's cross-section, a rectangle; Hadley's equations, about 10 nm grids), $\Delta L$ designed for the drawn 450 x 215 nm wire, two ideal splitters, the spectrum read as the paper reads it: $n_g$ at 1550 nm from the $M$ = 110 interferometer's peaks, $\lambda_1 \lambda_2 / ((\lambda_2 - \lambda_1) \Delta L)$ (Eq. 8), fitted by a line in $\lambda$ (Eq. 10)",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 4.2739 ± 0.0042; the tolerance is their Eq. 5 with ±20 nm of width and ±5 nm of thickness (their Fig. 1's process), by the solver's derivatives, plus that uncertainty",
            run: crate::circuit::components::measured::dwivedi_ng_470,
        },
        Case {
            id: "components/mzi-neff-dwivedi-602",
            title: r"Components: measured Mach-Zehnder interferometers of a 602 x 211 nm silicon wire in oxide (the SEM's cross-section, a rectangle; Hadley's equations, about 10 nm grids), $\Delta L$ designed for the drawn 600 x 215 nm wire, two ideal splitters, the spectrum read as the paper reads it: $n_\text{eff}$ at 1550 nm from the $m$ = 15 interferometer's peak, $n_\text{eff} L = m \lambda$ (Eq. 3), carried to 1550 nm by $n_g$",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 2.534 ± 0.0035; the tolerance is their Eq. 5 with ±20 nm of width and ±5 nm of thickness (their Fig. 1's process), by the solver's derivatives, plus that uncertainty",
            run: crate::circuit::components::measured::dwivedi_neff_602,
        },
        Case {
            id: "components/mzi-ng-dwivedi-602",
            title: r"Components: measured Mach-Zehnder interferometers of a 602 x 211 nm silicon wire in oxide (the SEM's cross-section, a rectangle; Hadley's equations, about 10 nm grids), $\Delta L$ designed for the drawn 600 x 215 nm wire, two ideal splitters, the spectrum read as the paper reads it: $n_g$ at 1550 nm from the $M$ = 110 interferometer's peaks, $\lambda_1 \lambda_2 / ((\lambda_2 - \lambda_1) \Delta L)$ (Eq. 8), fitted by a line in $\lambda$ (Eq. 10)",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 4.0453 ± 0.0045; the tolerance is their Eq. 5 with ±20 nm of width and ±5 nm of thickness (their Fig. 1's process), by the solver's derivatives, plus that uncertainty",
            run: crate::circuit::components::measured::dwivedi_ng_602,
        },
        Case {
            id: "components/mzi-neff-dwivedi-805",
            title: r"Components: measured Mach-Zehnder interferometers of a 805 x 211 nm silicon wire in oxide (the SEM's cross-section, a rectangle; Hadley's equations, about 10 nm grids), $\Delta L$ designed for the drawn 800 x 215 nm wire, two ideal splitters, the spectrum read as the paper reads it: $n_\text{eff}$ at 1550 nm from the $m$ = 15 interferometer's peak, $n_\text{eff} L = m \lambda$ (Eq. 3), carried to 1550 nm by $n_g$",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 2.67 ± 0.004; the tolerance is their Eq. 5 with ±20 nm of width and ±5 nm of thickness (their Fig. 1's process), by the solver's derivatives, plus that uncertainty",
            run: crate::circuit::components::measured::dwivedi_neff_805,
        },
        Case {
            id: "components/mzi-ng-dwivedi-805",
            title: r"Components: measured Mach-Zehnder interferometers of a 805 x 211 nm silicon wire in oxide (the SEM's cross-section, a rectangle; Hadley's equations, about 10 nm grids), $\Delta L$ designed for the drawn 800 x 215 nm wire, two ideal splitters, the spectrum read as the paper reads it: $n_g$ at 1550 nm from the $M$ = 110 interferometer's peaks, $\lambda_1 \lambda_2 / ((\lambda_2 - \lambda_1) \Delta L)$ (Eq. 8), fitted by a line in $\lambda$ (Eq. 10)",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 3.8902 ± 0.005; the tolerance is their Eq. 5 with ±20 nm of width and ±5 nm of thickness (their Fig. 1's process), by the solver's derivatives, plus that uncertainty",
            run: crate::circuit::components::measured::dwivedi_ng_805,
        },
    ];
    // the compact models' cases live with them
    cases.extend(crate::compact::checks::cases());
    // and the geometry kernel's
    cases.extend(crate::geometry::checks::cases());
    cases
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

fn fdfd3d_port_mode_slab(kind: Polarization, tolerance: f64) -> Outcome {
    let (got, exact) = crate::fdfd::port_checks3d::slab_port_index(kind, 0.0025);
    Outcome {
        measured: got,
        expected: exact,
        tolerance,
        error: (got - exact).abs(),
    }
}

fn fdfd3d_port_mode_slab_te() -> Outcome {
    // second order from 2.4e-4 at 5 nm predicts 6.0e-5, which it is
    fdfd3d_port_mode_slab(Polarization::Te, 1e-4)
}

fn fdfd3d_port_mode_slab_tm() -> Outcome {
    // second order from 6.1e-4 at 5 nm predicts 1.5e-4, which it is
    fdfd3d_port_mode_slab(Polarization::Tm, 2e-4)
}

fn fdfd3d_port_mode_strip() -> Outcome {
    let [te, _] = crate::fdfd::port_checks3d::strip_port_indices(0.02);
    // Hadley's equations on 20, 10 and 5 nm grids (2.44558833, 2.44543164, 2.44539267),
    // extrapolated at their fitted order (2.01)
    let limit = 2.445380;
    Outcome {
        measured: te,
        expected: limit,
        // the port's own discretization error at 20 nm, 2.4e-3, second order
        tolerance: 3e-3,
        error: (te - limit).abs(),
    }
}

fn fdfd3d_straight_strip() -> Outcome {
    let (worst, _) = crate::fdfd::port_checks3d::straight_strip();
    Outcome {
        measured: worst,
        expected: 0.0,
        // round-off: 1.8e-15 measured
        tolerance: 1e-12,
        error: worst,
    }
}

fn fdfd3d_reciprocity() -> Outcome {
    let s = crate::fdfd::port_checks3d::width_step();
    let asymmetry = (s[1][0] - s[0][1]).norm() / s[1][0].norm();
    Outcome {
        measured: asymmetry,
        expected: 0.0,
        // round-off: 2e-15 measured
        tolerance: 1e-12,
        error: asymmetry,
    }
}

fn fdfd3d_closed_guide_energy() -> Outcome {
    use crate::fdfd::port_checks3d::{closed_step, unitarity};
    let (s, _) = closed_step(20);
    let distance = unitarity(&s);
    Outcome {
        measured: distance,
        expected: 0.0,
        // the evanescent modes' share at 1 um from the step: 2.0e-6 measured
        tolerance: 5e-6,
        error: distance,
    }
}

fn fdfd3d_two_d_s_matrix() -> Outcome {
    use crate::fdfd::port_checks3d::two_d_s_difference;
    let worst = two_d_s_difference(crate::fdfd::Polarization::Ez)
        .max(two_d_s_difference(crate::fdfd::Polarization::Hz));
    Outcome {
        measured: worst,
        expected: 0.0,
        // E along z: the PMLs half a cell apart, 8.9e-9 measured; H along z, 2.1e-10
        tolerance: 5e-8,
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
        // 6.9e-12 (curl-curl) and 1.3e-10 (Shin and Fan)
        tolerance: 1e-9,
        error: worst,
    }
}

fn fdfd3d_qmr_symmetric_direct() -> Outcome {
    use crate::fdfd::Formulation;
    use crate::fdfd::checks3d::qmr_against_direct;
    // the curl-curl operator with PMLs is solved by QMR for symmetric matrices
    let worst = qmr_against_direct(Formulation::CurlCurl).0;
    Outcome {
        measured: worst,
        expected: 0.0,
        // as fdfd3d/qmr-direct's: measured 6.9e-12
        tolerance: 1e-9,
        error: worst,
    }
}

fn fdfd3d_ldlt_lu() -> Outcome {
    let (worst, symmetric) = crate::fdfd::checks3d::ldlt_against_lu();
    // the solver must have taken L D Lᵀ for the comparison to mean anything
    let measured = if symmetric { worst } else { f64::INFINITY };
    Outcome {
        measured,
        expected: 0.0,
        // both refined to a residual of 1e-12: their difference is bounded by the condition
        // number times that; measured 1.2e-15
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_dispersion() -> Outcome {
    let measured = [((3, 0), 0.9), ((2, 2), 0.9), ((1, 3), 0.5), ((4, 1), 1.0)]
        .into_iter()
        .map(|(m, courant)| {
            let (got, theory) = crate::fdtd::checks::dispersion(m, courant);
            ((got - theory) / theory).abs()
        })
        .fold(0.0f64, f64::max);
    Outcome {
        measured,
        expected: 0.0,
        // round-off over 400 steps: measured 3.6e-15
        tolerance: 1e-12,
        error: measured,
    }
}

fn fdtd_energy() -> Outcome {
    let measured = crate::fdtd::checks::energy_drift();
    Outcome {
        measured,
        expected: 0.0,
        // round-off: measured 2.4e-15
        tolerance: 1e-12,
        error: measured,
    }
}

fn fdtd_fdfd_lossy() -> Outcome {
    let measured = crate::fdtd::checks::against_fdfd(64, Some(2.0)).0;
    Outcome {
        measured,
        expected: 0.0,
        // the solves' rounding: measured 2.3e-14
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_fdfd_cpml() -> Outcome {
    let measured = crate::fdtd::checks::against_fdfd(128, None).0;
    Outcome {
        measured,
        expected: 0.0,
        // the CPML's convolution, first order in Δt: measured 3.5e-4
        tolerance: 5e-4,
        error: measured,
    }
}

fn fdtd_cpml_thickness() -> Outcome {
    let measured = crate::fdtd::checks::cpml_error(16);
    Outcome {
        measured,
        expected: 0.0,
        // measured 6.1e-6: −104 dB
        tolerance: 1e-4,
        error: measured,
    }
}

fn fdtd_spectrum() -> Outcome {
    let measured = crate::fdtd::checks::spectrum_against_fdfd();
    Outcome {
        measured,
        expected: 0.0,
        // the solves' rounding: measured 6.5e-13
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_tfsf_leakage() -> Outcome {
    let measured = [
        ((1, 0, 0), [0.0, 0.0, 1.0], false),
        ((2, 1, 0), [0.0, 0.0, 1.0], false),
        ((1, -3, 0), [1.0, 0.0, 0.0], false),
        ((0, 0, 1), [1.0, 0.0, 0.0], true),
        ((2, 1, 1), [1.0, -1.0, 0.0], true),
    ]
    .into_iter()
    .map(|(direction, polarization, three_d)| {
        crate::fdtd::checks::tfsf_leakage(direction, polarization, three_d)
    })
    .fold(0.0f64, f64::max);
    Outcome {
        measured,
        expected: 0.0,
        // round-off: measured 1.3e-15
        tolerance: 1e-12,
        error: measured,
    }
}

fn fdtd_tfsf_slab() -> Outcome {
    let measured = crate::fdtd::checks::tfsf_slab(0.005);
    Outcome {
        measured,
        expected: 0.0,
        // second order in the cells: measured 3.3e-4 at 5 nm (5.4e-3 at 20 nm, 1.3e-3 at 10)
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_mode_source_fdfd() -> Outcome {
    let measured = crate::fdtd::checks::mode_source_against_fdfd();
    Outcome {
        measured,
        expected: 0.0,
        // the solves' rounding: measured 2.5e-14
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_mode_source_backward() -> Outcome {
    let measured = crate::fdtd::checks::mode_source_open(false)[0].1;
    Outcome {
        measured,
        expected: 0.0,
        // the CPMLs against FDFD's PMLs: measured 1.5e-7
        tolerance: 1e-5,
        error: measured,
    }
}

fn fdtd_mode_source_power() -> Outcome {
    let measured = crate::fdtd::checks::mode_source_open(false)[0].2;
    Outcome {
        measured,
        expected: 1.0,
        // measured 1.000048
        tolerance: 5e-4,
        error: (measured - 1.0).abs(),
    }
}

fn fdtd_beam_waist() -> Outcome {
    let (measured, _, _) = crate::fdtd::checks::beam_fit(&crate::fdtd::checks::beam(0.0));
    Outcome {
        measured,
        expected: 1.5,
        // measured 1.49999
        tolerance: 1e-3,
        error: (measured - 1.5).abs(),
    }
}

fn fdtd_beam_divergence() -> Outcome {
    use crate::fdtd::checks::{beam, beam_divergence, beam_fit};
    let (_, _, divergence) = beam_fit(&beam(0.0));
    let predicted = beam_divergence(1.5, 1.0, Some((0.05, 1.0 / 32.0)));
    let measured = (divergence - predicted).abs() / predicted;
    Outcome {
        measured,
        expected: 0.0,
        // measured 2.7e-6 (0.218825)
        tolerance: 1e-4,
        error: measured,
    }
}

fn fdtd_beam_tilt() -> Outcome {
    use crate::fdtd::checks::{beam, beam_centre_slope};
    let tilted = beam(10f64.to_radians());
    let (d0, _, y0) = tilted.columns[0];
    let (d1, _, y1) = tilted.columns[4];
    let slope = (y1 - y0) / (d1 - d0);
    let predicted = beam_centre_slope(1.5, 10f64.to_radians(), 1.0, 0.05, 1.0 / 32.0);
    let measured = (slope - predicted).abs() / predicted;
    Outcome {
        measured,
        expected: 0.0,
        // measured 1.5e-4
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_ade_fdfd() -> Outcome {
    let measured = crate::fdtd::media_checks::against_fdfd().0;
    Outcome {
        measured,
        expected: 0.0,
        // the solve's rounding: measured 3.3e-14
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_drude_fresnel() -> Outcome {
    let measured = crate::fdtd::media_checks::drude_reflection(0.005).0;
    Outcome {
        measured,
        expected: 0.0,
        // second order in the cells: measured 3.0e-4 at 5 nm
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_lorentz_group_delay() -> Outcome {
    let measured = crate::fdtd::media_checks::lorentz_delay(0.0025).0;
    Outcome {
        measured,
        expected: 0.0,
        // second order to a floor near 1e-4: measured 3.6e-4 at 2.5 nm
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_bloch_fdfd() -> Outcome {
    use crate::fdtd::bloch_checks::against_fdfd;
    let measured = against_fdfd(false).max(against_fdfd(true));
    Outcome {
        measured,
        expected: 0.0,
        // the solves' rounding: measured 2.9e-14
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_bloch_multilayer() -> Outcome {
    use crate::fdtd::bloch_checks::multilayer;
    let measured = multilayer(0.005, false).0.max(multilayer(0.005, true).0);
    Outcome {
        measured,
        expected: 0.0,
        // second order in the cells: measured 1.3e-3 at 5 nm
        tolerance: 3e-3,
        error: measured,
    }
}

fn fdtd_bloch_bands() -> Outcome {
    use crate::fdtd::bloch_checks::{analytic_bands, bands};
    let k = 0.3 * std::f64::consts::PI / 0.5;
    let exact = analytic_bands(k, 2.0);
    let got = bands(0.00625, k);
    let measured = if got.len() == exact.len() {
        got.iter()
            .zip(&exact)
            .fold(0.0f64, |m, (a, b)| m.max((a - b).abs() / b))
    } else {
        f64::INFINITY
    };
    Outcome {
        measured,
        expected: 0.0,
        // second order in the cells: measured 3.1e-4 at 80 cells a period
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_fit_lossless() -> Outcome {
    let fits = crate::fdtd::media_checks::fits();
    let measured = fits[0].0.max(fits[1].0);
    Outcome {
        measured,
        expected: 0.0,
        // measured 6.0e-5
        tolerance: 1e-4,
        error: measured,
    }
}

fn fdtd_fit_lossy() -> Outcome {
    let measured = crate::fdtd::media_checks::fits()[2].0;
    Outcome {
        measured,
        expected: 0.0,
        // measured 8.1e-3
        tolerance: 1e-2,
        error: measured,
    }
}

fn fdtd_fitted_slab() -> Outcome {
    let measured = crate::fdtd::media_checks::fitted_slab(0.005).0;
    Outcome {
        measured,
        expected: 0.0,
        // second order in the cells: measured 2.0e-3 at 5 nm
        tolerance: 5e-3,
        error: measured,
    }
}

fn fdtd_smoothing_slab() -> Outcome {
    use crate::fdtd::smoothing::checks::slab_error;
    let eps = crate::fdtd::Permittivity::isotropic(4.0).expect("ε");
    let measured = slab_error(1.0 / 160.0, eps, crate::fdtd::Smoothing::default());
    Outcome {
        measured,
        expected: 0.0,
        // second order: measured 6.9e-4 at 1/160 µm
        tolerance: 1.5e-3,
        error: measured,
    }
}

fn fdtd_smoothing_anisotropic_slab() -> Outcome {
    use crate::fdtd::smoothing::checks::{slab_error, tilted_crystal};
    let smoothing = crate::fdtd::Smoothing::default();
    let measured = slab_error(1.0 / 160.0, tilted_crystal(), smoothing);
    Outcome {
        measured,
        expected: 0.0,
        // second order: measured 3.0e-4 at 1/160 µm
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_smoothing_oblique() -> Outcome {
    use crate::fdtd::{Coupling, Smoothing};
    let points = Smoothing {
        coupling: Coupling::Points,
        ..Smoothing::default()
    };
    let measured = crate::fdtd::smoothing::checks::oblique(points, 128).abs();
    Outcome {
        measured,
        expected: 0.0,
        // second order: measured 2.1e-4 at 128 cells a µm
        tolerance: 4e-4,
        error: measured,
    }
}

fn fdtd_smoothing_oblique_nodes() -> Outcome {
    let smoothing = crate::fdtd::Smoothing::with(crate::fdtd::Average::Subpixel);
    let measured = crate::fdtd::smoothing::checks::oblique(smoothing, 128).abs();
    Outcome {
        measured,
        expected: 0.0,
        // first order: measured 2.0e-3 at 128 cells a µm
        tolerance: 3e-3,
        error: measured,
    }
}

fn fdtd_smoothing_energy() -> Outcome {
    use crate::fdtd::Coupling;
    let measured = crate::fdtd::smoothing::checks::tensor_energy_drift(Coupling::Nodes, 100_000);
    Outcome {
        measured,
        expected: 0.0,
        // round-off: measured 2.0e-15 over 10⁵ steps
        tolerance: 1e-12,
        error: measured,
    }
}

fn fdtd_monitor_transforms() -> Outcome {
    let measured = crate::fdtd::monitors_checks::transforms_against_hand();
    Outcome {
        measured,
        expected: 0.0,
        // the same sum in the same order
        tolerance: 1e-15,
        error: measured,
    }
}

fn fdtd_monitor_flux() -> Outcome {
    let measured = crate::fdtd::monitors_checks::flux_against_fdfd();
    Outcome {
        measured,
        expected: 0.0,
        // FDFD's direct solve and the fields' decay to 1e-17
        tolerance: 1e-9,
        error: measured,
    }
}

fn fdtd_monitor_flux_box() -> Outcome {
    let (net, largest) = crate::fdtd::monitors_checks::box_balance(false);
    let measured = net / largest;
    Outcome {
        measured,
        expected: 0.0,
        // round-off once the fields have died away
        tolerance: 1e-12,
        error: measured,
    }
}

fn fdtd_monitor_modes() -> Outcome {
    let measured = crate::fdtd::monitors_checks::modes_against_fdfd();
    Outcome {
        measured,
        expected: 0.0,
        // FDFD's direct solve and the fields' decay to 1e-15
        tolerance: 1e-9,
        error: measured,
    }
}

fn fdtd_monitor_guides() -> Outcome {
    let measured = [false, true]
        .into_iter()
        .map(|bend| {
            let s = crate::fdtd::monitors_checks::guide_2d(bend);
            s.fdtd
                .iter()
                .zip(&s.fdfd)
                .map(|(a, b)| (a - b).norm())
                .fold(0.0f64, f64::max)
        })
        .fold(0.0f64, f64::max);
    Outcome {
        measured,
        expected: 0.0,
        // the CPML against FDFD's PML: measured 5.8e-5 (the straight guide's reflection)
        tolerance: 1e-4,
        error: measured,
    }
}

fn fdtd_meep_pml_rates() -> Outcome {
    let rates = crate::fdtd::meep_checks::pml_rates(4);
    let measured = rates
        .iter()
        .zip([6.0, 8.0, 10.0])
        .map(|(r, e)| (r - e).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured,
        expected: 0.0,
        // the rates near L = 4 are still above their limits: measured 0.32 (d = 3)
        tolerance: 0.4,
        error: measured,
    }
}

fn fdtd_ring_wronskian() -> Outcome {
    let measured = crate::fdtd::ring::wronskian_error();
    Outcome {
        measured,
        expected: 0.0,
        // measured 1.0e-10
        tolerance: 1e-9,
        error: measured,
    }
}

/// The ring's relative errors in frequency and in Q at `resolution` cells a µm, by order 3, 4
/// and 5; infinite unless exactly those three are found.
fn ring_errors(resolution: usize) -> Vec<(f64, f64)> {
    use crate::fdtd::ring::ring_q;
    let modes = crate::fdtd::meep_checks::ring_resonances(resolution);
    if modes.iter().map(|m| m.order).collect::<Vec<_>>() != [3, 4, 5] {
        return vec![(f64::INFINITY, f64::INFINITY)];
    }
    modes
        .iter()
        .map(|m| {
            (
                (m.fdtd.re - m.exact.re).abs() / m.exact.re,
                (ring_q(m.fdtd) - ring_q(m.exact)).abs() / ring_q(m.exact),
            )
        })
        .collect()
}

fn fdtd_ring_resonances() -> Outcome {
    let measured = ring_errors(20).iter().map(|e| e.0).fold(0.0, f64::max);
    Outcome {
        measured,
        expected: 0.0,
        // measured 7.7e-4 (m = 5)
        tolerance: 1.5e-3,
        error: measured,
    }
}

fn fdtd_ring_order() -> Outcome {
    let (coarse, fine) = (ring_errors(10), ring_errors(20));
    let measured = coarse
        .iter()
        .zip(&fine)
        .flat_map(|(c, f)| [(c.0 / f.0).log2(), (c.1 / f.1).log2()])
        .fold(f64::INFINITY, f64::min);
    let measured = if measured.is_finite() { measured } else { 0.0 };
    Outcome {
        measured,
        expected: 2.0,
        tolerance: 0.3,
        error: (measured - 2.0).abs(),
    }
}

fn fdtd_fdfd_band_2d() -> Outcome {
    let measured = crate::fdtd::agreement_checks::guides_2d(0.05);
    Outcome {
        measured,
        expected: 0.0,
        // the CPML against FDFD's PML: measured 3.2e-5
        tolerance: 1e-4,
        error: measured,
    }
}

fn fdtd_fdfd_band_2d_fine() -> Outcome {
    let measured = crate::fdtd::agreement_checks::guides_2d(0.025);
    Outcome {
        measured,
        expected: 0.0,
        // measured 1.7e-7
        tolerance: 1e-6,
        error: measured,
    }
}

fn fdtd_fdfd_band_3d_strip() -> Outcome {
    let measured = crate::fdtd::agreement_checks::strip_3d(8).largest_difference();
    Outcome {
        measured,
        expected: 0.0,
        // the CPML against FDFD's PML: measured 2.2e-4
        tolerance: 5e-4,
        error: measured,
    }
}

fn fdtd_fdfd_band_3d_bend() -> Outcome {
    let measured = crate::fdtd::agreement_checks::bend_3d(8).largest_difference();
    Outcome {
        measured,
        expected: 0.0,
        // the CPML against FDFD's PML: measured 1.9e-4
        tolerance: 5e-4,
        error: measured,
    }
}

fn fdtd_fdfd_smoothed() -> Outcome {
    use crate::fdtd::agreement_checks::smoothed_bend_2d;
    let measured = (smoothed_bend_2d(0.05) / smoothed_bend_2d(0.025)).log2();
    Outcome {
        measured,
        expected: 1.0,
        // first order, not yet asymptotic: 0.85, then 0.97 from 25 to 12.5 nm
        tolerance: 0.3,
        error: (measured - 1.0).abs(),
    }
}

fn fdtd_harmonic_inversion() -> Outcome {
    let measured = crate::fdtd::monitors_checks::harmonic_synthetic();
    Outcome {
        measured,
        expected: 0.0,
        // round-off in the small eigenproblem
        tolerance: 1e-9,
        error: measured,
    }
}

fn fdtd_cavity_resonances() -> Outcome {
    let (count, measured) = crate::fdtd::monitors_checks::cavity_resonances();
    Outcome {
        measured,
        expected: 0.0,
        // round-off; at least four modes found
        tolerance: 1e-12,
        error: if count >= 4 { measured } else { f64::INFINITY },
    }
}

fn fdtd_slab_resonance() -> Outcome {
    use crate::fdtd::monitors_checks::slab_resonance;
    use rayon::prelude::*;
    let errors: Vec<(f64, f64)> = [0.02, 0.01, 0.005]
        .par_iter()
        .map(|&h| slab_resonance(h))
        .collect();
    let order = |a: f64, b: f64| (a / b).log2();
    let measured = order(errors[1].0, errors[2].0).min(order(errors[1].1, errors[2].1));
    Outcome {
        measured,
        expected: 2.0,
        // measured 2.00 and 2.01
        tolerance: 0.05,
        error: (measured - 2.0).abs(),
    }
}

fn fdtd_smoothing_contrast() -> Outcome {
    use crate::fdtd::smoothing::checks::contrast_growth;
    let refused = contrast_growth(50.0, 0, true).is_none();
    let measured = contrast_growth(40.0, 100_000, true).unwrap_or(f64::INFINITY);
    Outcome {
        measured,
        expected: 1.0,
        // bounded: E and H trade energy, and ‖E‖ may rise above its first value
        tolerance: 1.0,
        error: if refused {
            (measured - 1.0).abs()
        } else {
            f64::INFINITY
        },
    }
}

fn fdtd_smoothing_oskooi() -> Outcome {
    use crate::fdtd::smoothing::checks::oskooi;
    use crate::fdtd::{Average, Smoothing};
    use rayon::prelude::*;
    let with = |average, diameter| Smoothing {
        average,
        diameter,
        coupling: crate::fdtd::Coupling::Nodes,
    };
    let runs = [
        (with(Average::Subpixel, 1.0), 64),
        (with(Average::Subpixel, 2.0), 64),
        (with(Average::Subpixel, 1.0), 32),
        (with(Average::InverseMean, 1.0), 32),
        (with(Average::Sampled, 1.0), 32),
    ];
    let f: Vec<f64> = runs.par_iter().map(|&(s, n)| oskooi(s, n)).collect();
    let reference = 0.5 * (f[0] + f[1]);
    let error = |v: f64| ((v - reference) / reference).abs();
    let measured = error(f[2]) / error(f[3]).min(error(f[4]));
    Outcome {
        measured,
        expected: 0.0,
        // "often by 1 order of magnitude": measured 0.063
        tolerance: 0.1,
        error: measured,
    }
}

fn fdtd_smoothing_triplets_oblique() -> Outcome {
    use crate::fdtd::Coupling;
    let measured = crate::fdtd::smoothing::checks::oblique(Coupling::Triplets, 128).abs();
    Outcome {
        measured,
        expected: 0.0,
        // first order: measured 1.06e-3 at 128 cells a µm
        tolerance: 1.6e-3,
        error: measured,
    }
}

/// log₂ of the oblique layers' error at 64 cells a µm over that at 128, by `scheme`.
fn oblique_order(scheme: crate::fdtd::smoothing::checks::Scheme) -> f64 {
    let (coarse, fine) = rayon::join(
        || crate::fdtd::smoothing::checks::oblique(scheme, 64),
        || crate::fdtd::smoothing::checks::oblique(scheme, 128),
    );
    (coarse / fine).abs().log2()
}

fn fdtd_smoothing_triplets_order() -> Outcome {
    let measured = oblique_order(crate::fdtd::Coupling::Triplets.into());
    Outcome {
        measured,
        expected: 1.0,
        // first order: measured 1.15
        tolerance: 0.35,
        error: (measured - 1.0).abs(),
    }
}

fn fdtd_smoothing_bauer_order() -> Outcome {
    let measured = oblique_order(crate::fdtd::smoothing::checks::Scheme::Accurate);
    Outcome {
        measured,
        expected: 2.0,
        // second order: measured 1.87
        tolerance: 0.3,
        error: (measured - 2.0).abs(),
    }
}

fn fdtd_smoothing_triplets_contrast() -> Outcome {
    use crate::fdtd::Coupling;
    use crate::fdtd::smoothing::checks::{contrast_drift, contrast_growth_with};
    let (drift, grown) = rayon::join(
        || contrast_drift(Coupling::Triplets, 100.0, 100_000),
        || contrast_growth_with(Coupling::Triplets, 100.0, 100_000, true),
    );
    let bounded = grown.is_some_and(|g| g < 2.0);
    Outcome {
        measured: drift,
        expected: 0.0,
        // round-off over 10⁵ steps, and ‖E‖ bounded
        tolerance: 1e-12,
        error: if bounded { drift } else { f64::INFINITY },
    }
}

fn fdtd_smoothing_triplets_lattices() -> Outcome {
    use crate::fdtd::Coupling;
    use crate::fdtd::smoothing::checks::{
        anisotropic_lattice, drift, hole_lattice, lattice_simulation,
    };
    let run = |structure: crate::fdtd::Structure| {
        let s = lattice_simulation(Coupling::Triplets, &structure, [1, 1], 16, true)
            .expect("the lattice");
        drift(s, 100_000)
    };
    let (holes, anisotropic) = rayon::join(
        || run(hole_lattice([1, 1])),
        || run(anisotropic_lattice([1, 1])),
    );
    let measured = holes.max(anisotropic);
    Outcome {
        measured,
        expected: 0.0,
        // round-off: measured 4.3e-15 and 5.0e-15 over 10⁵ steps
        tolerance: 1e-12,
        error: measured,
    }
}

fn fdtd_smoothing_wc07_growth() -> Outcome {
    use crate::fdtd::Coupling;
    use crate::fdtd::smoothing::checks::disc_growth;
    use rayon::prelude::*;
    let runs = [
        (Coupling::Nodes, 32),
        (Coupling::Nodes, 64),
        (Coupling::Triplets, 32),
        (Coupling::Triplets, 64),
    ];
    let found: Vec<(f64, Option<f64>)> = runs
        .par_iter()
        .map(|&(coupling, n)| disc_growth(coupling, 100.0, n, 300.0))
        .collect();
    let rate = |q: usize| found[q].1.unwrap_or(0.0);
    let measured = rate(0);
    // the paper's "≈ 3" and "≈ 6", to a fifth; the triplets bounded
    let fine = (rate(1) - 6.0).abs() < 1.2;
    let bounded = found[2].0 < 2.0 && found[3].0 < 2.0;
    Outcome {
        measured,
        expected: 3.0,
        // measured 2.85 (32 cells) and 6.11 (64)
        tolerance: 0.6,
        error: if fine && bounded {
            (measured - 3.0).abs()
        } else {
            f64::INFINITY
        },
    }
}

fn fdtd_smoothing_crystal() -> Outcome {
    use crate::fdtd::Coupling;
    use crate::fdtd::smoothing::checks::{ELLIPSOID_BANDS, ellipsoid_bands};
    let bands = ellipsoid_bands(Coupling::Triplets, 48);
    let measured = bands
        .iter()
        .zip(ELLIPSOID_BANDS)
        .map(|(f, r)| ((f - r) / r).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured,
        expected: 0.0,
        // measured 1.8e-3 at 48 cells
        tolerance: 3e-3,
        error: measured,
    }
}

fn fdtd_smoothing_crystal_order() -> Outcome {
    use crate::fdtd::smoothing::checks::{ELLIPSOID_BANDS, Scheme, ellipsoid_bands};
    let rms = |n: usize| {
        let bands = ellipsoid_bands(Scheme::Accurate, n);
        let sum: f64 = bands
            .iter()
            .zip(ELLIPSOID_BANDS)
            .map(|(f, r)| ((f - r) / r).powi(2))
            .sum();
        (sum / ELLIPSOID_BANDS.len() as f64).sqrt()
    };
    let (coarse, fine) = rayon::join(|| rms(32), || rms(48));
    let measured = (coarse / fine).ln() / 1.5f64.ln();
    Outcome {
        measured,
        expected: 2.0,
        // second order: measured 1.98
        tolerance: 0.3,
        error: (measured - 2.0).abs(),
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

fn fdfd3d_pml_reflection_stretched() -> Outcome {
    use crate::fdfd::checks3d::pml_reflection_stretched;
    let worst = pml_reflection_stretched(Polarization::Te, 1.0)
        .max(pml_reflection_stretched(Polarization::Tm, 1.0));
    Outcome {
        measured: worst,
        expected: 0.0,
        // 3.6e-6 measured, as the plain PML's 2.5e-6 and the grading's 3e-6
        tolerance: 1e-5,
        error: worst,
    }
}

fn fdfd3d_qmr_ilu_direct() -> Outcome {
    let (worst, _, _) = crate::fdfd::checks3d::ilu_against_direct();
    Outcome {
        measured: worst,
        expected: 0.0,
        // a residual of 1e-10 bounds the error by the condition number times 1e-10: 2.4e-10
        tolerance: 1e-8,
        error: worst,
    }
}

fn fdfd3d_gmres_multigrid_direct() -> Outcome {
    let (worst, _) = crate::fdfd::checks3d::multigrid_against_direct();
    Outcome {
        measured: worst,
        expected: 0.0,
        // as QMR + ILU(0)'s: a residual of 1e-10 bounds the error by the condition number times
        // 1e-10, 2.0e-10 measured
        tolerance: 1e-8,
        error: worst,
    }
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
    let deviation = marcuse_loss(core, clad, t, r, w, straight).unwrap_or(f64::NAN) / exact - 1.0;
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

fn fdtd_mie_table() -> Outcome {
    let (median, known) = crate::fdtd::mie_checks::table_one_agreement();
    Outcome {
        measured: median,
        expected: 0.0,
        // Mie's three digits; all his 66 entries within 0.016 but five known ones
        tolerance: 0.005,
        error: if known { median } else { f64::INFINITY },
    }
}

fn fdtd_mie_balance() -> Outcome {
    let measured = crate::fdtd::mie_checks::lossless_balance();
    Outcome {
        measured,
        expected: 0.0,
        // measured 1.4e-13, round-off over sums of up to 1100 terms
        tolerance: 1e-11,
        error: measured,
    }
}

fn fdtd_mie_terms() -> Outcome {
    use crate::fdtd::mie_checks::truncation_error;
    use num_complex::Complex64 as c64;
    let mut measured: f64 = 0.0;
    let mut short: f64 = f64::INFINITY;
    for (size, index) in [
        (5.0, c64::new(1.5, 0.0)),
        (50.0, c64::new(1.33, 0.0)),
        (200.0, c64::new(1.5, 0.05)),
    ] {
        let terms = (size + 4.0 * f64::cbrt(size) + 10.0) as usize;
        measured = measured.max(truncation_error(size, index, terms));
        short = short.min(truncation_error(size, index, (size / 2.0) as usize));
    }
    Outcome {
        measured,
        expected: 0.0,
        // round-off: the partial sum steps its D_ν down from a different start
        tolerance: 1e-12,
        error: if short > 1e-2 {
            measured
        } else {
            f64::INFINITY
        },
    }
}

fn fdtd_mie_sphere() -> Outcome {
    use crate::fdtd::Average;
    use crate::fdtd::mie_checks::{Material, SPHERE_TIME, sphere_run};
    let material = Material::Dielectric(4.0, Average::Subpixel);
    let coarse = sphere_run(8, &material, SPHERE_TIME);
    let fine = sphere_run(16, &material, SPHERE_TIME);
    let measured = (coarse.mean_error(false) / fine.mean_error(false)).log2();
    Outcome {
        measured,
        expected: 2.0,
        // measured 1.97; and no spurious absorption in the lossless sphere
        tolerance: 0.15,
        error: if fine.spurious_absorption() < 2e-3 {
            (measured - 2.0).abs()
        } else {
            f64::INFINITY
        },
    }
}

fn fdtd_mie_sphere_staircase() -> Outcome {
    use crate::fdtd::Average;
    use crate::fdtd::mie_checks::{Material, SPHERE_TIME, sphere_run};
    let run = sphere_run(
        16,
        &Material::Dielectric(4.0, Average::Sampled),
        SPHERE_TIME,
    );
    let measured = run.mean_error(false);
    Outcome {
        measured,
        expected: 0.0,
        // measured 6.0e-3
        tolerance: 1e-2,
        error: measured,
    }
}

fn fdtd_mie_drude() -> Outcome {
    use crate::fdtd::mie_checks::{Material, SPHERE_TIME, damped_drude, sphere_run};
    let run = sphere_run(16, &Material::Dispersive(damped_drude()), SPHERE_TIME);
    let measured = run.mean_error(true);
    Outcome {
        measured,
        expected: 0.0,
        // measured 1.6e-2; 3.0e-2 on 8 cells
        tolerance: 3e-2,
        error: measured,
    }
}

fn fdfd3d_adjoint_gradient() -> Outcome {
    let (measured, _) = crate::fdfd::checks3d_adjoint::gradient_against_differences();
    Outcome {
        measured,
        expected: 0.0,
        // the differences' own round-off, about 1e-10/δ
        tolerance: 1e-7,
        error: measured,
    }
}

fn fdtd_adjoint_modes() -> Outcome {
    use crate::fdtd::adjoint_checks::{Objective, strip_against_differences};
    let (measured, _) = strip_against_differences(Objective::Modes);
    Outcome {
        measured,
        expected: 0.0,
        // the differences' round-off and the transforms' settling, 1e-13 a block
        tolerance: 1e-7,
        error: measured,
    }
}

fn fdtd_adjoint_flux() -> Outcome {
    use crate::fdtd::adjoint_checks::{Objective, strip_against_differences};
    let (measured, _) = strip_against_differences(Objective::Flux);
    Outcome {
        measured,
        expected: 0.0,
        tolerance: 1e-7,
        error: measured,
    }
}

fn fdtd_adjoint_fdfd() -> Outcome {
    let (measured, _) = crate::fdtd::adjoint_checks::against_fdfd_closed();
    Outcome {
        measured,
        expected: 0.0,
        // FDFD's direct solve and the transforms' settling
        tolerance: 1e-10,
        error: measured,
    }
}

fn fdtd_adjoint_fdfd_cpml() -> Outcome {
    let measured = crate::fdtd::adjoint_checks::against_fdfd_open(256);
    Outcome {
        measured,
        expected: 0.0,
        tolerance: 5e-5,
        error: measured,
    }
}

fn fdtd_adjoint_slab() -> Outcome {
    use crate::fdtd::adjoint_checks::{airy, slab};
    let (_, exact) = airy();
    let measured = ((slab(1.0 / 160.0).1 - exact) / exact).abs();
    Outcome {
        measured,
        expected: 0.0,
        // measured 7.2e-4
        tolerance: 1e-3,
        error: measured,
    }
}

fn fdtd_adjoint_slab_order() -> Outcome {
    use crate::fdtd::adjoint_checks::{airy, slab};
    use rayon::prelude::*;
    let (_, exact) = airy();
    let errors: Vec<f64> = [80.0, 160.0]
        .par_iter()
        .map(|&n| ((slab(1.0 / n).1 - exact) / exact).abs())
        .collect();
    let measured = (errors[0] / errors[1]).log2();
    Outcome {
        measured,
        expected: 2.0,
        // measured 2.02
        tolerance: 0.1,
        error: (measured - 2.0).abs(),
    }
}

fn fdtd_adjoint_backward_mode() -> Outcome {
    let (measured, _) = crate::fdtd::adjoint_checks::adjoint_is_the_mode_sent_backward();
    Outcome {
        measured,
        expected: 0.0,
        tolerance: 1e-10,
        error: measured,
    }
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

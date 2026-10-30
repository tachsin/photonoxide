//! Third-order (χ⁽³⁾) data: the Kerr coefficient n₂ and the two-photon absorption coefficient β,
//! each value with its wavelength, its uncertainty as the paper states it, and how it was
//! obtained: measured and printed, read from a figure where the paper prints no number, or
//! derived from another measurement through a model.
//!
//! - **Silicon:** A. D. Bristow, N. Rotenberg, H. M. van Driel, Appl. Phys. Lett. 90, 191104
//!   (2007), doi:10.1063/1.2737359: z-scan of a 125 µm intrinsic (001) wafer at 300 K with
//!   200 fs pulses, 850 to 2200 nm. The text prints β and n₂ at 1220 nm and n₂'s peak at
//!   1800 nm; at 1550 nm the values are read from Figs. 2 and 3 ([`BRISTOW_FIGURES`]).
//! - **Silicon nitride:** K. Ikeda, R. E. Saperstein, N. Alic, Y. Fainman, Opt. Express 16,
//!   12987 (2008), doi:10.1364/OE.16.012987: n₂ of a PECVD film from a ring's pump–probe
//!   response at 1 GHz near 1548 nm, no uncertainty given.
//! - **Lithium niobate:** R. DeSalvo et al., IEEE J. Quantum Electron. 32, 1324 (1996),
//!   doi:10.1109/3.511545: z-scan with 30 ps pulses at 1064 and 532 nm, extraordinary
//!   polarization; n₂ printed in esu ([`n2_from_esu`]).
//! - **Aluminium nitride:** H. Jung, C. Xiong, K. Y. Fong, X. Zhang, H. X. Tang, Opt. Lett. 38,
//!   2810 (2013), doi:10.1364/OL.38.002810: n₂ derived from a microring's comb threshold, not
//!   measured directly.
//!
//! The units: n₂ in m²/W, with n = n₀ + n₂I; β in m/W, with dI/dz = −βI². The nonlinear
//! coefficient of a guide is γ = 2π n₂ / (λ A_eff) ([`crate::mode::quantities`]).

use serde::Serialize;

use super::Source;
use super::entries::src;
use crate::units::SPEED_OF_LIGHT;

/// Which third-order coefficient.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum KerrQuantity {
    /// The Kerr coefficient n₂, in m²/W: n = n₀ + n₂I.
    NonlinearIndex,
    /// The degenerate two-photon absorption coefficient β, in m/W: dI/dz = −βI².
    TwoPhotonAbsorption,
}

/// How a value was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Obtained {
    /// Measured, and printed in the paper's text or a table.
    Printed,
    /// Measured, but printed only as a point of a figure: read off the figure, with the reading's
    /// own uncertainty stated in [`Kerr::method`].
    ReadFromFigure,
    /// Derived from another measurement through a model (a comb threshold, say), not measured
    /// directly.
    Derived,
}

/// One third-order coefficient of a material, from one paper.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Kerr {
    /// n₂ or β.
    pub quantity: KerrQuantity,
    /// The value as the paper prints it, with its unit, e.g. `"(4.7 ± 2.0) × 10⁻⁵ cm²/GW"`.
    pub printed: String,
    /// The value in SI: m²/W for n₂, m/W for β.
    pub value: f64,
    /// Its uncertainty in SI, as the paper states it; `None` where it states none.
    pub uncertainty: Option<f64>,
    /// The vacuum wavelength, µm.
    pub wavelength: f64,
    /// Printed, read from a figure, or derived.
    pub obtained: Obtained,
    /// How it was measured or derived, and what the uncertainty covers.
    pub method: String,
    /// The sample: crystal or film, its orientation and the light's polarization.
    pub sample: String,
    /// The paper and the place in it.
    pub source: Source,
}

impl Kerr {
    /// n₂ in m²/W, or `None` for a two-photon absorption coefficient.
    pub fn n2(&self) -> Option<f64> {
        (self.quantity == KerrQuantity::NonlinearIndex).then_some(self.value)
    }
}

// ---------------------------------------------------------------------------------------------
// Units
// ---------------------------------------------------------------------------------------------

/// n₂ in m²/W from n₂ in esu, for n = n₀ + n₂⟨E²⟩: n₂(esu) = (c n₀ / 40π) n₂(m²/W), c in m/s
/// (DeSalvo et al., p. 1327).
pub fn n2_from_esu(esu: f64, n0: f64) -> f64 {
    40.0 * std::f64::consts::PI * esu / (SPEED_OF_LIGHT * n0)
}

/// n₂ in esu from n₂ in m²/W, the inverse of [`n2_from_esu`].
pub fn n2_to_esu(n2: f64, n0: f64) -> f64 {
    n2 * SPEED_OF_LIGHT * n0 / (40.0 * std::f64::consts::PI)
}

/// n₂ in m²/W from n₂ in cm²/W.
pub fn n2_from_cm2_per_w(value: f64) -> f64 {
    value * 1e-4
}

/// n₂ in cm²/W from n₂ in m²/W.
pub fn n2_to_cm2_per_w(n2: f64) -> f64 {
    n2 * 1e4
}

/// β in m/W from β in cm/GW.
pub fn beta_from_cm_per_gw(value: f64) -> f64 {
    value * 1e-11
}

/// β in cm/GW from β in m/W.
pub fn beta_to_cm_per_gw(beta: f64) -> f64 {
    beta * 1e11
}

// ---------------------------------------------------------------------------------------------
// Bristow et al.'s figures
// ---------------------------------------------------------------------------------------------

/// A point read off a figure's bitmap (the PDF's embedded image, in its own pixels): the
/// figure's axes by two ticks each, and the point's error bar.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reading {
    /// Two ticks of the x axis: (pixel, nm).
    pub(crate) x_ticks: [(f64, f64); 2],
    /// Two ticks of the y axis: (pixel, value in the axis' unit).
    pub(crate) y_ticks: [(f64, f64); 2],
    /// The error bar's column, its marker's centre row and its two caps' rows.
    pub(crate) bar: (f64, f64, [f64; 2]),
}

impl Reading {
    fn y(&self, row: f64) -> f64 {
        let [(p0, v0), (p1, v1)] = self.y_ticks;
        v0 + (row - p0) * (v1 - v0) / (p1 - p0)
    }

    /// The wavelength, nm.
    pub(crate) fn wavelength(&self) -> f64 {
        let [(p0, v0), (p1, v1)] = self.x_ticks;
        v0 + (self.bar.0 - p0) * (v1 - v0) / (p1 - p0)
    }

    /// The value at the marker's centre.
    pub(crate) fn value(&self) -> f64 {
        self.y(self.bar.1)
    }

    /// Half the error bar.
    pub(crate) fn half_bar(&self) -> f64 {
        0.5 * (self.y(self.bar.2[0]) - self.y(self.bar.2[1])).abs()
    }

    /// Half a pixel in the axis' unit: the reading's resolution.
    pub(crate) fn resolution(&self) -> f64 {
        let [(p0, v0), (p1, v1)] = self.y_ticks;
        0.5 * ((v1 - v0) / (p1 - p0)).abs()
    }
}

/// Fig. 2 (β, cm/GW): the bitmap is 897 × 813 pixels; 800 nm at column 112 and 2000 nm at 780.5,
/// 0 at row 715.5 and 2.5 at 171.5.
const FIG_2_X: [(f64, f64); 2] = [(112.0, 800.0), (780.5, 2000.0)];
const FIG_2_Y: [(f64, f64); 2] = [(715.5, 0.0), (171.5, 2.5)];
/// Fig. 3 (n₂, 10⁻¹⁴ cm²/W): 917 × 768 pixels; 1200 nm at column 280 and 2000 nm at 785, 3 at
/// row 547 and 15 at 98.
const FIG_3_X: [(f64, f64); 2] = [(280.0, 1200.0), (785.0, 2000.0)];
const FIG_3_Y: [(f64, f64); 2] = [(547.0, 3.0), (98.0, 15.0)];

/// Bristow et al.'s points read off their figures: β at 1220 and 1550 nm (Fig. 2), n₂ at 1220
/// and 1550 nm (Fig. 3). The 1550 nm square of Fig. 2 is partly hidden behind another paper's
/// point; its centre is the bar's midpoint, which the visible edges agree with.
pub(crate) const BRISTOW_FIGURES: [Reading; 4] = [
    Reading {
        x_ticks: FIG_2_X,
        y_ticks: FIG_2_Y,
        bar: (346.0, 262.0, [177.5, 347.5]),
    },
    Reading {
        x_ticks: FIG_2_X,
        y_ticks: FIG_2_Y,
        bar: (530.5, 544.0, [458.0, 629.0]),
    },
    Reading {
        x_ticks: FIG_3_X,
        y_ticks: FIG_3_Y,
        bar: (293.0, 483.0, [408.0, 557.0]),
    },
    Reading {
        x_ticks: FIG_3_X,
        y_ticks: FIG_3_Y,
        bar: (501.0, 478.0, [403.0, 552.5]),
    },
];

/// The values the text prints at 1220 nm: β = 2.1 ± 0.4 cm/GW and n₂ = (4.7 ± 2.0) × 10⁻¹⁴
/// cm²/W (p. 191104-2), for the check of the readings.
pub(crate) const BRISTOW_1220: [(f64, f64); 2] = [(2.1, 0.4), (4.7, 2.0)];

// ---------------------------------------------------------------------------------------------
// The data
// ---------------------------------------------------------------------------------------------

/// DeSalvo et al.'s linear indices (Table II) at 1064 and 532 nm, which their esu values take.
pub(crate) const DESALVO_N0: [f64; 2] = [2.2, 2.23];
/// DeSalvo et al.'s Table I for LiNbO₃: n₂ × 10¹⁴ esu at 1064 and 532 nm with ±, and β (cm/GW)
/// at 532 nm with ±.
pub(crate) const DESALVO_LINBO3: ([(f64, f64); 2], (f64, f64)) =
    ([(48.0, 7.0), (440.0, 70.0)], (0.38, 0.08));

const BRISTOW_SAMPLE: &str = "bulk intrinsic crystalline silicon, a 125 µm double-side polished wafer cut along (001), 300 K, linearly polarized light";
const BRISTOW_METHOD: &str = "z-scan with nominally 200 fs pulses at 1 kHz from an optical parametric amplifier, 75 µm FWHM focus; the error 'mainly related to obtaining values of peak laser intensity and small differences in beam shape with tuning' (p. 191104-2)";

#[allow(clippy::too_many_arguments)]
fn value(
    quantity: KerrQuantity,
    printed: &str,
    value: f64,
    uncertainty: Option<f64>,
    wavelength: f64,
    obtained: Obtained,
    method: String,
    sample: &str,
    source: Source,
) -> Kerr {
    Kerr {
        quantity,
        printed: printed.into(),
        value,
        uncertainty,
        wavelength,
        obtained,
        method,
        sample: sample.into(),
        source,
    }
}

fn silicon() -> Vec<Kerr> {
    use KerrQuantity::*;
    let [_, b1550, _, n1550] = BRISTOW_FIGURES;
    let read = |r: Reading, what: &str| {
        format!(
            "{BRISTOW_METHOD}. The paper prints no number here: read off {what} at {:.0} nm, the marker's centre against the axis' ticks on the PDF's bitmap, to half a pixel ({:.1e} in the axis' unit); the uncertainty is half the plotted error bar. The same reading at 1220 nm gives the text's printed values within half their last digit (material/kerr-bristow-readings)",
            r.wavelength(),
            r.resolution()
        )
    };
    let beta = |r: Reading| beta_from_cm_per_gw(r.value());
    let n2 = |r: Reading| n2_from_cm2_per_w(r.value() * 1e-14);
    vec![
        value(
            TwoPhotonAbsorption,
            "2.1 ± 0.4 cm/GW",
            beta_from_cm_per_gw(2.1),
            Some(beta_from_cm_per_gw(0.4)),
            1.22,
            Obtained::Printed,
            format!("open-aperture {BRISTOW_METHOD}; peak intensity 7.8 GW/cm² (Fig. 1)"),
            BRISTOW_SAMPLE,
            src("bristow-2007", "p. 191104-2, the fits of Fig. 1"),
        ),
        value(
            NonlinearIndex,
            "(4.7 ± 2.0) × 10⁻⁵ cm²/GW",
            n2_from_cm2_per_w(4.7e-14),
            Some(n2_from_cm2_per_w(2.0e-14)),
            1.22,
            Obtained::Printed,
            format!("closed-aperture {BRISTOW_METHOD}; peak intensity 7.8 GW/cm² (Fig. 1)"),
            BRISTOW_SAMPLE,
            src("bristow-2007", "p. 191104-2, the fits of Fig. 1"),
        ),
        value(
            NonlinearIndex,
            "1.2 × 10⁻¹³ cm²/W",
            n2_from_cm2_per_w(1.2e-13),
            None,
            1.8,
            Obtained::Printed,
            format!(
                "closed-aperture {BRISTOW_METHOD}; the peak of n₂ over 850 to 2200 nm, which 'varies by a factor of 4'; no uncertainty printed with it (Fig. 3's bar there is about ±2 × 10⁻¹⁴ cm²/W)"
            ),
            BRISTOW_SAMPLE,
            src("bristow-2007", "abstract, p. 191104-1"),
        ),
        value(
            TwoPhotonAbsorption,
            "0.79 ± 0.39 cm/GW (read from Fig. 2)",
            beta(b1550),
            Some(beta_from_cm_per_gw(b1550.half_bar())),
            1.55,
            Obtained::ReadFromFigure,
            read(b1550, "Fig. 2"),
            BRISTOW_SAMPLE,
            src("bristow-2007", "Fig. 2, p. 191104-2"),
        ),
        value(
            NonlinearIndex,
            "(4.8 ± 2.0) × 10⁻¹⁴ cm²/W (read from Fig. 3)",
            n2(n1550),
            Some(n2_from_cm2_per_w(n1550.half_bar() * 1e-14)),
            1.55,
            Obtained::ReadFromFigure,
            read(n1550, "Fig. 3"),
            BRISTOW_SAMPLE,
            src("bristow-2007", "Fig. 3, p. 191104-3"),
        ),
    ]
}

fn silicon_nitride() -> Vec<Kerr> {
    vec![value(
        KerrQuantity::NonlinearIndex,
        "2.4 × 10⁻¹⁵ cm²/W",
        n2_from_cm2_per_w(2.4e-15),
        None,
        1.548,
        Obtained::Printed,
        "n₂ = Δn/I from the shift of an all-pass ring's resonance (r = 20 µm, g = 600 nm, Q about 35,600) under a pump modulated at 1 GHz, too fast for the thermal response: a 0.001 nm shift, Δn = 1.2 × 10⁻⁶, at 2.6 W peak in the ring; pump near 1548 nm and probe near 1558 nm, as in the paper's thermal test (p. 12990). No uncertainty is given".into(),
        "PECVD silicon nitride (dual-frequency, 350 °C; film index about 1.99), a 500 nm × 1 µm core in PECVD oxide, TE-like mode: not this entry's stoichiometric LPCVD film",
        src("ikeda-2008", "Sec. 4, p. 12993; abstract"),
    )]
}

fn lithium_niobate() -> Vec<Kerr> {
    let ([(n1064, d1064), (n532, d532)], (b, db)) = DESALVO_LINBO3;
    let [n0_1064, n0_532] = DESALVO_N0;
    let method = |n0: f64| {
        format!(
            "z-scan with single 30 ps (FWHM) pulses of a Q-switched, mode-locked Nd:YAG laser and its second harmonic; the uncertainty Table I's, 'the overall experimental uncertainty is approximately ±20 %' (p. 1326), mostly the irradiance's calibration. Printed in esu for n = n₀ + n₂⟨E²⟩, converted with n₀ = {n0} (Table II) by n₂(esu) = (c n₀/40π) n₂(m²/W) (p. 1327)"
        )
    };
    let sample = "congruent LiNbO₃ from Crystal Technology, x cut, the light polarized along the optic axis (extraordinary) (footnote 2, p. 1325)";
    vec![
        value(
            KerrQuantity::NonlinearIndex,
            "(48 ± 7) × 10⁻¹⁴ esu",
            n2_from_esu(n1064 * 1e-14, n0_1064),
            Some(n2_from_esu(d1064 * 1e-14, n0_1064)),
            1.064,
            Obtained::Printed,
            method(n0_1064),
            sample,
            src("desalvo-1996", "Table I (p. 1326), Table II"),
        ),
        value(
            KerrQuantity::NonlinearIndex,
            "(440 ± 70) × 10⁻¹⁴ esu",
            n2_from_esu(n532 * 1e-14, n0_532),
            Some(n2_from_esu(d532 * 1e-14, n0_532)),
            0.532,
            Obtained::Printed,
            method(n0_532),
            sample,
            src("desalvo-1996", "Table I (p. 1326), Table II"),
        ),
        value(
            KerrQuantity::TwoPhotonAbsorption,
            "0.38 ± 0.08 cm/GW",
            beta_from_cm_per_gw(b),
            Some(beta_from_cm_per_gw(db)),
            0.532,
            Obtained::Printed,
            "open-aperture z-scan with single 30 ps (FWHM) pulses at 532 nm; 'the overall experimental uncertainty is approximately ±20 %' (p. 1326). At 1064 nm the photon energy is below half the gap (E_g = 3.9 eV, Table I), so no two-photon absorption".into(),
            sample,
            src("desalvo-1996", "Table I (p. 1326)"),
        ),
    ]
}

fn aluminium_nitride() -> Vec<Kerr> {
    vec![value(
        KerrQuantity::NonlinearIndex,
        "(2.3 ± 1.5) × 10⁻¹⁵ cm²/W",
        n2_from_cm2_per_w(2.3e-15),
        Some(n2_from_cm2_per_w(1.5e-15)),
        1.55,
        Obtained::Derived,
        "not measured directly: derived from the comb threshold, about 210 mW in the 3.5 µm wide guide, through the threshold formula of the paper's Eq. (1) (Kippenberg et al.), with A_eff = 1.34 µm² and n_eff = 1.96 from finite-element simulation, Q₀ = 800,000, K = 0.3 and Δω = 3 ± 1.5 GHz from the transmission spectrum, and a correction factor C(Γ) between 1 and 2; pumped near 1550 nm".into(),
        "an AlN film 650 nm thick, guides 3.5 µm wide at the top with slanted sidewalls, on 2 µm of thermal oxide under 3.5 µm of PECVD oxide annealed at 1000 °C; the TE-like mode in the (001) plane",
        src("jung-2013", "Eq. (1), p. 2812"),
    )]
}

/// The third-order coefficients the catalogue has for an entry (by its id), or none.
pub fn kerr(entry: &str) -> Vec<Kerr> {
    match entry {
        "si" => silicon(),
        "si3n4" => silicon_nitride(),
        "linbo3" => lithium_niobate(),
        "aln" => aluminium_nitride(),
        _ => Vec::new(),
    }
}

impl super::Entry {
    /// The material's third-order coefficients, n₂ and β, beside its index, when the catalogue
    /// has them ([`kerr`]).
    pub fn kerr(&self) -> Vec<Kerr> {
        kerr(&self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_printed_values_are_the_ones_kept() {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b.abs();
        let si = kerr("si");
        // the leading number of each printed value times its unit is the value kept
        assert!(close(si[0].value, 2.1e-11) && close(si[1].value, 4.7e-18));
        assert!(close(si[2].value, 1.2e-17));
        assert!(close(kerr("si3n4")[0].value, 2.4e-19));
        assert!(close(kerr("aln")[0].value, 2.3e-19));
        assert!(close(kerr("aln")[0].uncertainty.unwrap(), 1.5e-19));
        let ln = kerr("linbo3");
        assert!(close(n2_to_esu(ln[0].value, 2.2), 48e-14));
        assert!(close(n2_to_esu(ln[1].value, 2.23), 440e-14));
        assert!(close(ln[2].value, 3.8e-12));
        for k in si.iter().chain(&ln) {
            assert!(
                k.printed.contains('±') || k.uncertainty.is_none(),
                "{}",
                k.printed
            );
        }
        // the readings at 1550 nm, as the printed strings round them
        let [_, b, _, n] = BRISTOW_FIGURES;
        assert!((b.value() - 0.79).abs() < 0.005 && (b.half_bar() - 0.39).abs() < 0.005);
        assert!((n.value() - 4.8).abs() < 0.05 && (n.half_bar() - 2.0).abs() < 0.05);
        assert!((b.wavelength() - 1550.0).abs() < 2.0 && (n.wavelength() - 1550.0).abs() < 2.0);
    }

    #[test]
    fn units() {
        // 1 cm²/W is 1e-4 m²/W; 1 cm/GW is 1e-11 m/W
        assert_eq!(n2_from_cm2_per_w(1.0), 1e-4);
        assert_eq!(beta_from_cm_per_gw(1.0), 1e-11);
        // LiNbO₃ at 1064 nm: 9.1e-20 m²/W
        let n2 = kerr("linbo3")[0].value;
        assert!((n2 - 9.146e-20).abs() < 1e-23, "{n2}");
        assert!(kerr("gaas").is_empty());
        assert_eq!(kerr("aln")[0].n2(), Some(kerr("aln")[0].value));
        assert_eq!(kerr("linbo3")[2].n2(), None);
    }
}

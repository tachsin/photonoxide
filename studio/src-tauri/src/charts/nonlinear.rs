//! The charts of the lesson on nonlinear integrated optics, each on the library's `nonlinear`
//! module or the mode solver's fields:
//!
//! - **Miller's rule** (`nl-miller`): lithium niobate's d₃₃ and d₃₁ carried from one of Shoji et
//!   al.'s measured fundamentals to the others by `nonlinear::miller`, against the catalogue's
//!   measurements.
//! - **The coupled-amplitude equations** (`nl-coupled`): SHG or parametric amplification along a
//!   guide by RK4 (`nonlinear::coupled`), against the undepleted closed forms and Armstrong et
//!   al.'s tanh².
//! - **Quasi-phase matching** (`nl-qpm`): the harmonic's field summed domain by domain, exactly,
//!   in the studio, with no grating, a grating of order m and duty D, and phase matched; Fejer et
//!   al.'s Fourier coefficient from `nonlinear::phase`.
//! - **The acceptance bandwidths** (`nl-acceptance`): sinc²(ΔkL/2) of SHG in bulk periodically
//!   poled congruent lithium niobate against the fundamental or the temperature, with Fejer et
//!   al.'s widths (`nonlinear::phase::Device`).
//! - **A thin-film ridge's modes and overlap** (`nl-ridge`): Zhao et al. 2020's x-cut MgO:LN
//!   ridge, its TE₀ modes at a fundamental and its harmonic on 20 nm cells, the poling period,
//!   the effective areas and the normalized efficiency from the overlap (`nonlinear::guide`).
//! - **Self-phase modulation** (`nl-spm`) and **solitons** (`nl-soliton`): the NLSE by split-step
//!   (`nonlinear::pulse`), against Agrawal's RMS broadening and the soliton's return.
//! - **Four-wave mixing** (`nl-fwm`): the parametric gain against the signal's detuning
//!   (`nonlinear::fwm`, Agrawal's Eq. 10.4.7 with Δk from β₂ and β₄).
//! - **A Kerr cavity's uniform states** (`nl-lle-states`) and **a dissipative soliton's comb**
//!   (`nl-comb`): the Lugiato–Lefever equation's homogeneous roots and a soliton settled by
//!   split-step (`nonlinear::resonator`).
//! - **Photon pairs** (`nl-jsa`): a down-converter's joint spectral intensity and its Schmidt
//!   purity (`nonlinear::pairs`), against the Gaussian closed form.

use std::collections::HashMap;
use std::f64::consts::{LN_2, PI, TAU};
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::material::catalogue::{self, Axis, Conditions, TensorKind};
use photonoxide::material::{CrystalAxis, Orientation, vacuum};
use photonoxide::mode::pockels::Along;
use photonoxide::nonlinear::coupled::{
    SecondHarmonic, ThreeWave, coupling_from_efficiency, undepleted_lossy_shg,
};
use photonoxide::nonlinear::fwm;
use photonoxide::nonlinear::guide::{
    Family, Grid, ModeField, Ridge, SecondOrder, fundamental, percent_per_watt_cm2,
    second_harmonic_coupling,
};
use photonoxide::nonlinear::miller::miller_shg;
use photonoxide::nonlinear::pairs::{
    Matching, Pump, gaussian_gamma, gaussian_purity, joint_spectrum,
};
use photonoxide::nonlinear::phase::{
    Device, Expansion, Index, Polarization, Process, State, Type, Uniaxial, Variable, Wave,
    fourier_coefficient, sinc2,
};
use photonoxide::nonlinear::pulse::{
    Dispersion, Guide, Pulse, dispersion_length, soliton_period, soliton_power, spm_rms_broadening,
};
use photonoxide::nonlinear::resonator::{
    Resonator, homogeneous_powers, instability_rate, max_detuning, peak_width,
};
use photonoxide::raster::Raster;
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};

use super::{ChartData, ChartSpec, Curve, FieldMap, figure, invalid, param, sig};

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// `n` points from `a` to `b`, both included.
fn span(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| a + (b - a) * k as f64 / (n - 1) as f64)
        .collect()
}

fn um(lam: f64) -> photonoxide::Result<Wavelength> {
    Wavelength::um(lam)
}

fn empty(x_label: &str, y_label: &str) -> ChartData {
    ChartData {
        x_label: x_label.to_owned(),
        x_length: false,
        y_label: y_label.to_owned(),
        y_log: false,
        y_range: None,
        series: Vec::new(),
        marker: None,
        figures: Vec::new(),
        note: String::new(),
        maps: Vec::new(),
    }
}

/// A small cache, cleared when it grows past 200 entries.
struct Cache<T> {
    map: Mutex<HashMap<Vec<i64>, T>>,
}

impl<T: Clone> Cache<T> {
    fn new() -> Cache<T> {
        Cache {
            map: Mutex::new(HashMap::new()),
        }
    }

    fn get_or(
        &self,
        key: &[f64],
        make: impl FnOnce() -> photonoxide::Result<T>,
    ) -> photonoxide::Result<T> {
        // to a millionth: a slider's values are far coarser
        let k: Vec<i64> = key.iter().map(|v| (v * 1e6).round() as i64).collect();
        if let Some(v) = self.map.lock().ok().and_then(|m| m.get(&k).cloned()) {
            return Ok(v);
        }
        let v = make()?;
        if let Ok(mut m) = self.map.lock() {
            if m.len() >= 200 {
                m.clear();
            }
            m.insert(k, v.clone());
        }
        Ok(v)
    }
}

/// The fundamentals of Shoji et al.'s Table 10, µm: the catalogue's measured d of both lithium
/// niobates.
const SHOJI: [f64; 3] = [0.852, 1.064, 1.313];

/// Zhao et al. 2020's d₃₃, pm/V (p. 1), which the ridge chart takes, as the job's cases do.
const ZHAO_D33: f64 = 27.0;

/// The chart specs.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "nl-miller",
            title: "Lithium niobate's d coefficients by Miller's rule",
            about: "d₃₃ and d₃₁ of lithium niobate for SHG against the fundamental, carried by Miller's rule from Shoji et al.'s measurement at one fundamental, beside the catalogue's measurements at all three.",
            computed_by: "nonlinear::miller::miller_shg (Miller's δ held, χ = n² − 1 of each wave's index from the entry's default model) on the catalogue's Shoji et al. 1997 tensors (material::catalogue)",
            params: vec![
                param(
                    "crystal",
                    "Crystal",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1: congruent LiNbO₃; 2: 5 mol% MgO-doped LiNbO₃.",
                ),
                param(
                    "from",
                    "Carried from",
                    "",
                    (1.0, 3.0, 2.0),
                    1.0,
                    false,
                    "The measured fundamental the rule starts from: 1: 0.852 µm, 2: 1.064 µm, 3: 1.313 µm.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-coupled",
            title: "SHG and parametric gain along a guide",
            about: "The coupled-amplitude equations integrated along a guide of normalized efficiency η: the harmonic's share of the power for SHG of 1.55 µm, or the signal's gain for parametric amplification of 1.50 µm by a 0.775 µm pump, beside the undepleted closed forms.",
            computed_by: "nonlinear::coupled::SecondHarmonic and ThreeWave (RK4, 2000 steps over the length), coupling_from_efficiency, undepleted_lossy_shg; Armstrong et al.'s tanh² and the gain 1 + (Γ/g)² sinh²(gz)",
            params: vec![
                param(
                    "process",
                    "Process",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1: second-harmonic generation; 2: parametric amplification.",
                ),
                param(
                    "efficiency",
                    "Efficiency $\\eta$ (%/W/cm²)",
                    "",
                    (10.0, 10000.0, 4500.0),
                    10.0,
                    true,
                    "The undepleted normalized efficiency, %/(W·cm²): η = κ².",
                ),
                param(
                    "power",
                    "Power in $P$ (mW)",
                    "",
                    (1.0, 10000.0, 100.0),
                    1.0,
                    true,
                    "The fundamental's power (SHG) or the pump's (amplification), mW.",
                ),
                param(
                    "length",
                    "Length $L$",
                    "um",
                    (500.0, 20000.0, 4000.0),
                    100.0,
                    false,
                    "The guide's length.",
                ),
                param(
                    "phase",
                    "Mismatch $\\Delta kL/2$",
                    "",
                    (0.0, 6.0, 0.0),
                    0.05,
                    false,
                    "The phase mismatch over the whole length, rad.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-qpm",
            title: "How a harmonic grows: phase matched, mismatched and quasi-phase matched",
            about: "The second harmonic's power along a crystal in coherence lengths, with no grating, with the nonlinear coefficient's sign flipped by a grating of order m and duty cycle D, and phase matched, each driven polarization's e^(−iΔkz) integrated exactly over every domain.",
            computed_by: "the integral of d(z) e^(−iΔkz) over each domain, summed in the studio; Fejer et al.'s Fourier coefficient G_m (nonlinear phase's fourier_coefficient)",
            params: vec![
                param(
                    "order",
                    "Order $m$",
                    "",
                    (1.0, 3.0, 1.0),
                    1.0,
                    false,
                    "The grating's order: its period is 2m coherence lengths.",
                ),
                param(
                    "duty",
                    "Duty cycle $D$",
                    "",
                    (0.05, 0.95, 0.5),
                    0.01,
                    false,
                    "The share of each period at +d.",
                ),
                param(
                    "span",
                    "Length in $l_c$",
                    "",
                    (2.0, 40.0, 12.0),
                    1.0,
                    false,
                    "The crystal's length in coherence lengths l_c = π/Δk.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-acceptance",
            title: "SHG in periodically poled lithium niobate: its acceptance",
            about: "sinc²(ΔkL/2) of d₃₃ SHG in bulk congruent lithium niobate, first-order quasi-phase matched at 25 °C by a grating cut for the fundamental, against the fundamental's wavelength or the temperature, with Fejer et al.'s linear estimates and the widths between the half maxima.",
            computed_by: "nonlinear::phase::Uniaxial::catalogue (Edwards and Lawrence's n_e(T)), Device::quasi_phase_matched with Jundt's thermal expansion, Device::phase_tuned, Device::response and Device::acceptance",
            params: vec![
                param(
                    "fundamental",
                    "Fundamental $\\lambda$",
                    "um",
                    (0.86, 2.2, 1.064),
                    0.001,
                    false,
                    "The fundamental's vacuum wavelength the grating is cut for.",
                ),
                param(
                    "length",
                    "Length $L$",
                    "um",
                    (100.0, 50000.0, 1000.0),
                    10.0,
                    true,
                    "The crystal's length.",
                ),
                param(
                    "variable",
                    "Against",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1: the fundamental's wavelength at 25 °C; 2: the temperature at the design wavelength.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-ridge",
            title: "A thin-film lithium niobate ridge: its modes, period and efficiency",
            about: "Zhao et al. 2020's x-cut ridge (300 nm of MgO:LiNbO₃ on silica, air above, 75° sidewalls, along the crystal's y), its TE₀ modes at a fundamental and at its second harmonic on 20 nm cells: |Eₓ| of each, the effective indices, the first-order poling period, the effective areas, and the SHG efficiency from the modes' overlap through d₃₃ = 27 pm/V, poled through at a 50 % duty cycle.",
            computed_by: "nonlinear::guide: Ridge::cross_section on 20 nm cells behind a mirror wall, fundamental (the full-vector mode solver), ModeField, SecondOrder turned onto the chip, second_harmonic_coupling and percent_per_watt_cm2; the areas by Afshar and Monro's Eq. 41 (Fields::effective_area)",
            params: vec![
                param(
                    "width",
                    "Ridge width $w$",
                    "um",
                    (0.8, 2.0, 1.24),
                    0.01,
                    false,
                    "The ridge's top width.",
                ),
                param(
                    "etch",
                    "Etch depth $e$",
                    "nm",
                    (20.0, 200.0, 50.0),
                    5.0,
                    false,
                    "How deep the 300 nm film is etched beside the ridge.",
                ),
                param(
                    "fundamental",
                    "Fundamental $\\lambda$",
                    "um",
                    (1.45, 1.7, 1.57),
                    0.005,
                    false,
                    "The fundamental's vacuum wavelength; the harmonic is at half.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-spm",
            title: "Self-phase modulation of a Gaussian pulse",
            about: "A Gaussian pulse's spectrum before and after a Kerr guide without dispersion or loss, for a peak nonlinear phase φ_max = γP₀L: the spectrum broadens and splits into peaks while the pulse keeps its shape.",
            computed_by: "nonlinear::pulse::Guide::propagate (the split-step Fourier method; without dispersion the Kerr step is exact) on Pulse::gaussian, Pulse::spectrum, Pulse::rms_bandwidth; Agrawal's Eq. 4.1.16 by spm_rms_broadening",
            params: vec![param(
                "phase",
                "Peak phase $\\varphi_\\mathrm{max}$ (rad)",
                "",
                (0.0, 20.0, 14.1372),
                0.05,
                false,
                "γP₀L, the phase the pulse's peak gains, rad: 4.5π is 14.14.",
            )],
        },
        ChartSpec {
            id: "nl-soliton",
            title: "A soliton of the nonlinear Schrödinger equation",
            about: "A sech pulse of soliton order N along a guide of anomalous dispersion (β₂ = −0.05 ps²/m, γ = 1 /(W·m), T₀ = 100 fs), its power against time at the start and at a distance in soliton periods: order 1 keeps its shape, higher orders narrow, split and return at every period.",
            computed_by: "nonlinear::pulse::Guide::propagate (the symmetrized split-step Fourier method, 2000 steps a soliton period, 1024 points across 50 T₀) on Pulse::sech, soliton_power, soliton_period",
            params: vec![
                param(
                    "order",
                    "Order $N$",
                    "",
                    (0.5, 4.0, 3.0),
                    0.1,
                    false,
                    "(γP₀T₀²/|β₂|)^½: the input's peak is N² times the fundamental soliton's.",
                ),
                param(
                    "distance",
                    "Distance $z/z_0$",
                    "",
                    (0.0, 2.0, 0.5),
                    0.01,
                    false,
                    "How far, in soliton periods z₀ = (π/2)T₀²/|β₂|.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-fwm",
            title: "Four-wave mixing's parametric gain",
            about: "The signal's gain and the idler's conversion against the signal's detuning from a CW pump at 1.55 µm, undepleted, with no idler in: the mismatch Δk = β₂Ω² + β₄Ω⁴/12 from the dispersion and the pump's Kerr shift 2γP.",
            computed_by: "nonlinear::fwm::signal_gain, conversion and peak_detuning (Agrawal's Eqs. 10.2.19–10.2.21, 10.4.6–10.4.7), nonlinear::pulse::Dispersion::four_wave_mismatch",
            params: vec![
                param(
                    "power",
                    "Pump $P$ (W)",
                    "",
                    (0.01, 10.0, 1.0),
                    0.01,
                    true,
                    "The pump's power in the guide, W.",
                ),
                param(
                    "gamma",
                    "$\\gamma$ (1/W/m)",
                    "",
                    (0.1, 1000.0, 1.0),
                    0.1,
                    true,
                    "The guide's nonlinear coefficient 2πn₂/(λA_eff), 1/(W·m).",
                ),
                param(
                    "length",
                    "Length $L$ (m)",
                    "",
                    (0.001, 10.0, 2.0),
                    0.001,
                    true,
                    "The guide's length, m.",
                ),
                param(
                    "beta2",
                    "$\\beta_2$ (ps²/m)",
                    "",
                    (-1.0, 1.0, -0.05),
                    0.001,
                    false,
                    "The group-velocity dispersion at the pump, ps²/m (1 ps²/m is 1000 fs²/mm).",
                ),
                param(
                    "beta4",
                    "$\\beta_4$ (10⁻⁴ ps⁴/m)",
                    "",
                    (-10.0, 10.0, 0.0),
                    0.01,
                    false,
                    "The fourth-order dispersion at the pump, in units of 10⁻⁴ ps⁴/m.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-lle-states",
            title: "A Kerr cavity's uniform states",
            about: "The intracavity power X = |ψ|² of the Lugiato–Lefever equation's uniform steady states against the pump's detuning ζ₀ (half linewidths) at a drive f²: one state below ζ₀ = √3, three on the S-shaped stretch above it, the middle one unstable; above X = 1 a uniform state is unstable to a pair of comb lines when the dispersion is anomalous.",
            computed_by: "nonlinear::resonator::homogeneous_powers (Lugiato and Lefever's Eq. 2, f² = X(1 + (X − ζ₀)²)) and instability_rate (their Eq. 4)",
            params: vec![
                param(
                    "drive",
                    "Drive $f^2$",
                    "",
                    (0.5, 30.0, 4.0),
                    0.1,
                    false,
                    "The pump's power in units where |ψ|² = 1 shifts the resonance by half a linewidth.",
                ),
                param(
                    "detuning",
                    "Detuning $\\zeta_0$",
                    "",
                    (-2.0, 30.0, 3.0),
                    0.05,
                    false,
                    "The pump's detuning from the cold resonance in half linewidths, positive redder: the marker.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-comb",
            title: "A dissipative Kerr soliton's comb",
            about: "The comb lines of a single soliton of the Lugiato–Lefever equation with D₂ alone, settled by split-step from the approximate sech soliton on 512 lines, beside the approximation's lines: their power against the line number μ.",
            computed_by: "nonlinear::resonator::Resonator::quadratic, soliton and soliton_field, Resonator::settle (Strang splitting, steps of 0.002, settled to 10⁻⁹ of the peak per unit time), peak_width, max_detuning; the lines by a discrete Fourier transform",
            params: vec![
                param(
                    "detuning",
                    "Detuning $\\zeta_0$",
                    "",
                    (4.0, 14.0, 10.0),
                    0.1,
                    false,
                    "The pump's detuning in half linewidths.",
                ),
                param(
                    "drive",
                    "Drive $f$",
                    "",
                    (2.0, 6.0, 3.0),
                    0.05,
                    false,
                    "The drive's amplitude.",
                ),
                param(
                    "d2",
                    "Dispersion $d_2 = D_2/\\kappa$",
                    "",
                    (0.001, 0.02, 0.004),
                    0.0005,
                    true,
                    "The resonances' second-order departure from an even comb, in linewidths.",
                ),
            ],
        },
        ChartSpec {
            id: "nl-jsa",
            title: "Photon pairs' joint spectrum and purity",
            about: "The joint spectral intensity |f(λₛ, λᵢ)|² of degenerate down-conversion of 0.775 µm into 1.55 µm by a quasi-phase-matched guide whose waves have group indices 2.30 (signal), 2.40 (pump) and gᵢ (idler), pumped by a Gaussian pulse, and the pairs' Schmidt purity.",
            computed_by: "nonlinear::pairs::joint_spectrum (Grice and Walmsley's f = α(ν_s + ν_i)Φ(ν_s, ν_i)) and JointSpectrum::schmidt on 201 × 201 frequencies, Device::phase_at_frequencies for the picture, gaussian_purity",
            params: vec![
                param(
                    "pump",
                    "Pump width (nm)",
                    "",
                    (0.2, 3.0, 0.6),
                    0.01,
                    true,
                    "The FWHM of the pump's spectrum in wavelength, nm.",
                ),
                param(
                    "length",
                    "Length $L$",
                    "um",
                    (2000.0, 30000.0, 10000.0),
                    500.0,
                    false,
                    "The guide's length.",
                ),
                param(
                    "idler",
                    "Idler's group index $g_i$",
                    "",
                    (2.2, 2.6, 2.25),
                    0.005,
                    false,
                    "The idler's group index: 2.25 a type-0 guide's, above the pump's 2.40 a group-velocity-matched type II's.",
                ),
                param(
                    "matching",
                    "Phase matching",
                    "",
                    (1.0, 2.0, 2.0),
                    1.0,
                    false,
                    "1: sinc(ΔkL/2), the device's own; 2: its Gaussian stand-in, whose purity has a closed form.",
                ),
            ],
        },
    ]
}

// ---- Miller's rule ----

/// d₃₃ and d₃₁ of lithium niobate (`crystal` 1) or MgO:LN (2) by Miller's rule from Shoji et
/// al.'s value at `from` (1, 2, 3: 0.852, 1.064, 1.313 µm), against the fundamental.
pub fn miller(crystal: f64, from: f64) -> photonoxide::Result<ChartData> {
    let (id, name) = if crystal.round() as i64 == 2 {
        ("linbo3-mgo", "MgO:LiNbO₃")
    } else {
        ("linbo3", "LiNbO₃")
    };
    let start = SHOJI[(from.round() as usize).clamp(1, 3) - 1];
    let missing = || invalid("catalogue", format!("the entry {id} lacks its index model"));
    let e = catalogue::entry(id).ok_or_else(missing)?;
    let model = e.default_model().ok_or_else(missing)?;
    let materials = model.materials(Conditions::default())?;
    let axis = |a: Axis| {
        model
            .axes
            .iter()
            .position(|x| *x == a)
            .map(|k| materials[k].clone())
            .ok_or_else(missing)
    };
    let (o, ex) = (axis(Axis::Ordinary)?, axis(Axis::Extraordinary)?);
    let d_at = |lam: f64, col: usize| -> Option<f64> {
        e.tensors
            .iter()
            .filter(|t| t.kind == TensorKind::SecondOrder && t.wavelength == Some(lam))
            .find_map(|t| t.value(3, col))
            .map(f64::abs)
    };
    let grid = span(0.82, 2.2, 139);
    let mut series = Vec::new();
    let mut figures = Vec::new();
    for (k, (col, label, fundamental)) in [(3, "d₃₃", [&ex, &ex]), (1, "d₃₁", [&o, &o])]
        .into_iter()
        .enumerate()
    {
        let d0 = d_at(start, col).ok_or_else(missing)?;
        let carried = |lam: f64| -> photonoxide::Result<f64> {
            miller_shg(d0, um(start)?, um(lam)?, &ex, fundamental)
        };
        let points: Vec<[f64; 2]> = grid
            .iter()
            .filter_map(|&l| carried(l).ok().map(|d| [l, d]))
            .collect();
        series.push(curve(
            &format!("{label}, Miller's rule from {start} µm"),
            points,
            false,
            k,
        ));
        let measured: Vec<[f64; 2]> = SHOJI
            .iter()
            .filter_map(|&l| d_at(l, col).map(|d| [l, d]))
            .collect();
        series.push(curve(
            &format!("{label}, Shoji et al. measured"),
            measured,
            true,
            k,
        ));
        for &lam in SHOJI.iter().filter(|&&l| l != start) {
            let (Some(m), Ok(p)) = (d_at(lam, col), carried(lam)) else {
                continue;
            };
            figures.push(figure(
                &format!("{label} at {lam} µm"),
                p,
                4,
                "pm/V",
                &format!(
                    "measured {}: Miller's {:+.1} %",
                    sig(m, 3),
                    100.0 * (p / m - 1.0)
                ),
            ));
        }
    }
    Ok(ChartData {
        x_label: "fundamental λ (µm)".to_owned(),
        y_label: "|d| (pm/V)".to_owned(),
        y_range: Some([0.0, 40.0]),
        series,
        marker: Some(start),
        figures,
        note: format!(
            "Exact, no grid: {name}'s d carried with Miller's δ held, each χ = n² − 1 from the \
             catalogue entry's default index model (Zelmon et al.'s), the harmonic's polarization \
             along c (n_e), the fundamental's along c for d₃₃ and across it for d₃₁ (n_o). The \
             dashed lines join the three measurements at {}, {} and {} µm.",
            SHOJI[0], SHOJI[1], SHOJI[2]
        ),
        ..empty("", "")
    })
}

// ---- the coupled-amplitude equations ----

/// SHG of 1.55 µm (`process` 1) or parametric amplification of 1.50 µm by 0.775 µm (2) along a
/// guide of `efficiency` %/(W·cm²), `power_mw` in, `length_mm` long, mismatched by
/// ΔkL/2 = `phase`.
pub fn coupled(
    process: f64,
    efficiency: f64,
    power_mw: f64,
    length_mm: f64,
    phase: f64,
) -> photonoxide::Result<ChartData> {
    let kappa = coupling_from_efficiency(efficiency);
    let (l, p0) = (length_mm * 1e3, power_mw * 1e-3);
    let dk = 2.0 * phase / l;
    let zs = span(l / 200.0, l, 200);
    let steps = |z: f64| ((2000.0 * z / l).ceil() as usize).max(20);
    let real = |v: f64| c64::new(v, 0.0);
    if process.round() as i64 == 2 {
        let (ns, np) = (1.0 / 1.5, 1.0 / 0.775);
        let ni = np - ns;
        let tw = ThreeWave {
            frequencies: [ns, ni],
            coupling: kappa,
            mismatch: dk,
            loss: [0.0; 3],
        };
        let seed = 1e-9 * p0;
        let gamma = kappa * (ns * ni * p0).sqrt() / np;
        let g2 = gamma * gamma - 0.25 * dk * dk;
        let closed = |z: f64| {
            let s = if g2 > 0.0 {
                (g2.sqrt() * z).sinh() / g2.sqrt()
            } else if g2 < 0.0 {
                ((-g2).sqrt() * z).sin() / (-g2).sqrt()
            } else {
                z
            };
            1.0 + gamma * gamma * s * s
        };
        let mut ours = vec![[0.0, 1.0]];
        let mut undepleted = vec![[0.0, 1.0]];
        for &z in &zs {
            let a = tw.propagate([real(seed.sqrt()), real(0.0), real(p0.sqrt())], z, steps(z))?;
            ours.push([z * 1e-3, a[0].norm_sqr() / seed]);
            undepleted.push([z * 1e-3, closed(z)]);
        }
        let end = ours.last().map_or(1.0, |p| p[1]);
        return Ok(ChartData {
            x_label: "position z (mm)".to_owned(),
            y_label: "signal gain P_s(z)/P_s(0)".to_owned(),
            y_log: true,
            series: vec![
                curve("the coupled equations (RK4)", ours, false, 0),
                curve("undepleted: 1 + (Γ/g)² sinh² gz", undepleted, true, 1),
            ],
            figures: vec![
                figure(
                    "Gain at $L$",
                    end,
                    6,
                    "",
                    &format!("{} dB", sig(10.0 * end.log10(), 4)),
                ),
                figure("Undepleted", closed(l), 6, "", "cosh² ΓL when matched"),
                figure("$\\Gamma L$", gamma * l, 4, "", "Γ = κ(ν_sν_iP)^½/ν_p"),
            ],
            note: format!(
                "RK4 on 2000 steps over the length, its error falling as the step's fourth \
                 power: a 0.775 µm pump of {} mW amplifying 1.50 µm from 10⁻⁹ of it, the idler \
                 at 1.603 µm, η = {} %/(W·cm²), no loss. The pump depletes only once the gain \
                 reaches about 10⁹.",
                sig(power_mw, 3),
                sig(efficiency, 3)
            ),
            ..empty("", "")
        });
    }
    let shg = SecondHarmonic {
        coupling: kappa,
        mismatch: dk,
        loss: [0.0; 2],
    };
    let mut ours = vec![[0.0, 0.0]];
    let mut undepleted = vec![[0.0, 0.0]];
    let mut tanh = vec![[0.0, 0.0]];
    for &z in &zs {
        let a = shg.propagate([real(p0.sqrt()), real(0.0)], z, steps(z))?;
        ours.push([z * 1e-3, a[1].norm_sqr() / p0]);
        let u = undepleted_lossy_shg(&shg, p0, z) / p0;
        if u <= 1.1 {
            undepleted.push([z * 1e-3, u]);
        }
        tanh.push([z * 1e-3, (kappa * p0.sqrt() * z).tanh().powi(2)]);
    }
    let end = ours.last().map_or(0.0, |p| p[1]);
    let mut series = vec![
        curve("the coupled equations (RK4)", ours, false, 0),
        curve("undepleted: ηPz² sinc²(Δkz/2)", undepleted, true, 1),
    ];
    let x = kappa * p0.sqrt() * l;
    let mut figures = vec![
        figure("$P_{2\\omega}/P$ at $L$", end, 6, "", "depleted"),
        figure(
            "Undepleted",
            undepleted_lossy_shg(&shg, p0, l) / p0,
            6,
            "",
            "ηPL² sinc²(ΔkL/2)",
        ),
        figure("$\\kappa\\sqrt{P}\\,L$", x, 4, "", "κ = √η"),
    ];
    if phase == 0.0 {
        series.push(curve("Armstrong et al.: tanh² κ√P z", tanh, true, 2));
        figures.push(figure(
            "$\\tanh^2\\kappa\\sqrt{P}L$",
            x.tanh().powi(2),
            6,
            "",
            "matched",
        ));
    }
    Ok(ChartData {
        x_label: "position z (mm)".to_owned(),
        y_label: "harmonic's share P_2ω(z)/P".to_owned(),
        y_range: Some([0.0, 1.05]),
        series,
        figures,
        note: format!(
            "RK4 on 2000 steps over the length, its error falling as the step's fourth power: \
             {} mW of 1.55 µm in, η = {} %/(W·cm²), no loss. The undepleted curve stops where \
             it passes 1.1.",
            sig(power_mw, 3),
            sig(efficiency, 3)
        ),
        ..empty("", "")
    })
}

// ---- quasi-phase matching ----

/// The harmonic's power along `span` coherence lengths: no grating, a grating of `order` and
/// `duty`, and phase matched, in units of the matched power after one coherence length.
pub fn qpm(order: f64, duty: f64, span_lc: f64) -> photonoxide::Result<ChartData> {
    let m = order.round().clamp(1.0, 3.0) as u32;
    let period = 2.0 * f64::from(m);
    let dk = PI;
    // ∫_a^b e^{−iΔkz} dz
    let piece = |a: f64, b: f64| {
        (c64::from_polar(1.0, -dk * a) - c64::from_polar(1.0, -dk * b)) / c64::new(0.0, dk)
    };
    let sign = |z: f64| {
        if (z / period).rem_euclid(1.0) < duty {
            1.0
        } else {
            -1.0
        }
    };
    let n = (span_lc * 40.0).round() as usize;
    let h = span_lc / n as f64;
    let (mut none, mut poled) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
    let g = fourier_coefficient(m, duty);
    let mut curves = [
        vec![[0.0, 0.0]],
        vec![[0.0, 0.0]],
        vec![[0.0, 0.0]],
        vec![[0.0, 0.0]],
    ];
    let mut highest_none: f64 = 0.0;
    for k in 0..n {
        let (z0, z1) = (k as f64 * h, (k + 1) as f64 * h);
        none += piece(z0, z1);
        // split the step at the domains' edges, kΛ and (k + D)Λ
        let mut edges = vec![z0];
        let first = (z0 / period).floor() as i64;
        for j in first..=first + 1 {
            for e in [j as f64 * period, (j as f64 + duty) * period] {
                if e > z0 && e < z1 {
                    edges.push(e);
                }
            }
        }
        edges.push(z1);
        edges.sort_by(f64::total_cmp);
        for w in edges.windows(2) {
            poled += sign(0.5 * (w[0] + w[1])) * piece(w[0], w[1]);
        }
        highest_none = highest_none.max(none.norm_sqr());
        curves[0].push([z1, z1 * z1]);
        curves[1].push([z1, none.norm_sqr()]);
        curves[2].push([z1, poled.norm_sqr()]);
        curves[3].push([z1, g * g * z1 * z1]);
    }
    let [matched, unmatched, grating, average] = curves;
    let ratio = poled.norm_sqr() / (span_lc * span_lc);
    Ok(ChartData {
        x_label: "position z/l_c".to_owned(),
        y_label: "harmonic's power, in the matched power at z = l_c".to_owned(),
        y_log: false,
        y_range: Some([0.0, (span_lc * span_lc).max(1.0) * 1.02]),
        series: vec![
            curve("phase matched: (z/l_c)²", matched, false, 0),
            curve("no grating", unmatched, false, 1),
            curve(
                &format!("poled, order {m}, duty {duty:.2}"),
                grating,
                false,
                2,
            ),
            curve("G_m² (z/l_c)²", average, true, 2),
        ],
        marker: None,
        figures: vec![
            figure("$G_m$", g, 4, "", "(2/mπ) sin(mπD), Fejer et al.'s Eq. 22"),
            figure(
                "$G_m^2$",
                g * g,
                4,
                "",
                "the poled growth's share of the matched",
            ),
            figure(
                "Poled over matched at the end",
                ratio,
                4,
                "",
                "the sum's, tending to G_m² as the length grows",
            ),
            figure(
                "No grating's largest",
                highest_none,
                4,
                "",
                "(2/π)² = 0.4053 at z = l_c, exact",
            ),
        ],
        note: format!(
            "Exact, no grid: the polarization's e^(−iΔkz) integrated in closed form over every \
             domain and every step of l_c/40, Δk = π/l_c, the grating's period 2m l_c = \
             {period} l_c. Powers are |∫ d(z) e^(−iΔkz) dz|² in units of l_c²."
        ),
        ..empty("", "")
    })
}

// ---- the acceptance bandwidths ----

/// d₃₃ SHG of `fundamental` µm over `length_mm` of congruent LiNbO₃, first-order QPM at 25 °C.
fn ln_shg(fundamental: f64, length_mm: f64) -> photonoxide::Result<Device> {
    let ln = Uniaxial::catalogue(
        "linbo3",
        Some("edwards-lawrence-1984"),
        Some("edwards-lawrence-1984"),
    )?;
    let state = State::celsius(25.0);
    let shg = ln.second_harmonic(
        um(fundamental)?,
        Type::Zero(Polarization::Extraordinary),
        &state,
    )?;
    Ok(
        Device::quasi_phase_matched(shg, length_mm * 1e3, state, 1, 0.5)?
            .with_expansion(Expansion::LINBO3_JUNDT),
    )
}

/// The response of [`ln_shg`] against the fundamental (`variable` 1) or the temperature (2).
pub fn acceptance(
    fundamental: f64,
    length_mm: f64,
    variable: f64,
) -> photonoxide::Result<ChartData> {
    let device = ln_shg(fundamental, length_mm)?;
    let state = *device.state();
    let lc = device.process().coherence_length(&state)?;
    let period = device.grating().map_or(f64::NAN, |g| g.period());
    let dl = device.acceptance(Variable::Wavelength)?;
    let dt = device.acceptance(Variable::Temperature)?;
    let width = |a: &photonoxide::nonlinear::phase::Acceptance| {
        if a.numerical.is_finite() {
            a.numerical
        } else {
            a.linear
        }
    };
    let (points, x_label, marker) = if variable.round() as i64 == 2 {
        let w = width(&dt);
        let ts = span((25.0 - 2.5 * w).max(0.0), (25.0 + 2.5 * w).min(500.0), 401);
        let points: Vec<[f64; 2]> = ts
            .iter()
            .filter_map(|&t| device.response(&State::celsius(t)).ok().map(|r| [t, r]))
            .collect();
        (points, "temperature T (°C)", 25.0)
    } else {
        let w = width(&dl);
        let ls = span(fundamental - 2.5 * w, fundamental + 2.5 * w, 401);
        let points: Vec<[f64; 2]> = ls
            .iter()
            .filter_map(|&l| {
                device
                    .phase_tuned(l, &state)
                    .ok()
                    .map(|p| [l * 1e3, sinc2(p)])
            })
            .collect();
        (points, "fundamental λ (nm)", fundamental * 1e3)
    };
    Ok(ChartData {
        x_label: x_label.to_owned(),
        y_label: "response sinc²(ΔkL/2)".to_owned(),
        y_range: Some([0.0, 1.05]),
        series: vec![curve("sinc²(ΔkL/2)", points, false, 0)],
        marker: Some(marker),
        figures: vec![
            figure("Coherence length $l_c$", lc, 4, "µm", "π/|Δk₀|, no grating"),
            figure(
                "Period $\\Lambda$",
                period,
                4,
                "µm",
                "first order, 2l_c at 25 °C",
            ),
            figure(
                "$\\delta\\lambda$, Fejer et al.'s Eq. 25",
                dl.linear * 1e3,
                4,
                "nm",
                "the FWHM from ∂Δk/∂λ",
            ),
            figure(
                "$\\delta\\lambda$ between the half maxima",
                dl.numerical * 1e3,
                4,
                "nm",
                "",
            ),
            figure(
                "$\\delta T$, Fejer et al.'s Eq. 44",
                dt.linear,
                4,
                "K",
                "with the expansion",
            ),
            figure(
                "$\\delta T$ between the half maxima",
                dt.numerical,
                4,
                "K",
                if dt.numerical.is_finite() {
                    ""
                } else {
                    "none: a half maximum lies outside Edwards and Lawrence's 0–500 °C"
                },
            ),
        ],
        note: format!(
            "Exact, no grid: Edwards and Lawrence's n_e(T) (the catalogue's \
             edwards-lawrence-1984), a first-order grating at a 50 % duty cycle cut for SHG of \
             {} µm at 25 °C, Jundt's thermal expansion along it, {} mm long.",
            sig(fundamental, 4),
            sig(length_mm, 3)
        ),
        ..empty("", "")
    })
}

// ---- the thin-film ridge ----

#[derive(Clone)]
struct RidgeModes {
    n_eff: [f64; 2],
    areas: [f64; 2],
    eta: f64,
    maps: [Raster; 2],
    outline: Vec<[f64; 2]>,
}

/// The film, µm, and the sidewalls, degrees: Zhao et al.'s.
const FILM: f64 = 0.3;
const SIDEWALL: f64 = 75.0;

fn ridge_of(width: f64, etch: f64) -> photonoxide::Result<Ridge> {
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z))?;
    let missing = || invalid("catalogue", "an entry is missing");
    let silica = catalogue::entry("sio2")
        .ok_or_else(missing)?
        .default_model()
        .ok_or_else(missing)?
        .materials(Conditions::default())?
        .remove(0);
    Ok(Ridge {
        film: FILM,
        etch,
        width,
        angle: SIDEWALL,
        core: catalogue::entry("linbo3-mgo")
            .ok_or_else(missing)?
            .oriented(None, Conditions::default(), x_cut)?,
        substrate: silica,
        cladding: vacuum(),
        along: Along::Y,
    })
}

fn ridge_modes(width: f64, etch: f64, lam: f64) -> photonoxide::Result<RidgeModes> {
    static CACHE: OnceLock<Cache<RidgeModes>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(&[width, etch, lam], || {
            let ridge = ridge_of(width, etch)?;
            let grid = Grid::new(0.02).halved();
            let mut solved = Vec::new();
            for l in [lam, 0.5 * lam] {
                let w = um(l)?;
                let cs = ridge.cross_section(w, &grid, Family::Te)?;
                let mode = fundamental(&cs, w, Family::Te, 4)?;
                solved.push((cs, mode, w));
            }
            let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z))?;
            let mut v = [[0.0; 6]; 3];
            v[2][2] = ZHAO_D33;
            let d = SecondOrder::from_voigt(v)?
                .rotated(&x_cut)
                .in_section(Along::Y);
            let (cs_f, mode_f, w_f) = &solved[0];
            let (cs_h, mode_h, w_h) = &solved[1];
            let ef = ModeField::new(mode_f, cs_f, *w_f)?;
            let eh = ModeField::new(mode_h, cs_h, *w_h)?;
            let share = ridge.film_share(cs_f, |_, _| true);
            let kappa = second_harmonic_coupling(&eh, &ef, &d, &share)?;
            let eta = percent_per_watt_cm2(kappa * fourier_coefficient(1, 0.5));
            // the pictures: |E_x| over the ridge and around it, cells of 10 nm, the half window
            // mirrored
            let (x0, x1, y0, y1) = (-1.6, 1.6, -0.7, 0.9);
            let (nx, ny) = (320, 160);
            let mut maps = Vec::new();
            let mut areas = [0.0; 2];
            for (k, (cs, mode, _)) in solved.iter().enumerate() {
                let fields = mode.fields_one_sided(cs)?;
                areas[k] = fields.effective_area();
                let (xs, ys) = (cs.x(), cs.y());
                let cell = |v: &[f64], u: f64| {
                    v.partition_point(|&node| node <= u)
                        .saturating_sub(1)
                        .min(v.len() - 2)
                };
                let mut values = vec![0.0f64; nx * ny];
                for j in 0..ny {
                    let y = y0 + (j as f64 + 0.5) * (y1 - y0) / ny as f64;
                    let cj = cell(ys, y);
                    for i in 0..nx {
                        let x = x0 + (i as f64 + 0.5) * (x1 - x0) / nx as f64;
                        values[j * nx + i] = fields.e(cell(xs, x.abs()), cj)[0].norm();
                    }
                }
                let peak = values
                    .iter()
                    .copied()
                    .fold(0.0, f64::max)
                    .max(f64::MIN_POSITIVE);
                maps.push(Raster {
                    nx,
                    ny,
                    x0,
                    x1,
                    y0,
                    y1,
                    values: values
                        .iter()
                        .map(|v| ((v / peak) * 1e3).round() as f32 / 1e3)
                        .collect(),
                });
            }
            let slab = FILM - etch;
            let foot = 0.5 * width + etch / SIDEWALL.to_radians().tan();
            let outline = vec![
                [x0, 0.0],
                [x1, 0.0],
                [x1, slab],
                [foot, slab],
                [0.5 * width, FILM],
                [-0.5 * width, FILM],
                [-foot, slab],
                [x0, slab],
                [x0, 0.0],
            ];
            let [m0, m1]: [Raster; 2] = maps
                .try_into()
                .map_err(|_| invalid("ridge chart", "two pictures"))?;
            Ok(RidgeModes {
                n_eff: [mode_f.effective_index().re, mode_h.effective_index().re],
                areas,
                eta,
                maps: [m0, m1],
                outline,
            })
        })
}

/// Zhao et al.'s ridge, `width` µm wide and etched `etch_nm`, at the fundamental `lam` µm.
pub fn ridge(width: f64, etch_nm: f64, lam: f64) -> photonoxide::Result<ChartData> {
    let r = ridge_modes(width, etch_nm * 1e-3, lam)?;
    let period = 0.5 * lam / (r.n_eff[1] - r.n_eff[0]);
    let names = [
        format!("|Eₓ| of TE₀ at {} µm", sig(lam, 4)),
        format!("|Eₓ| of TE₀ at {} µm", sig(0.5 * lam, 4)),
    ];
    let maps = r
        .maps
        .iter()
        .zip(&names)
        .map(|(m, name)| FieldMap {
            label: name.clone(),
            raster: m.clone(),
            outline: vec![r.outline.clone()],
        })
        .collect();
    Ok(ChartData {
        x_label: "x".to_owned(),
        x_length: true,
        y_label: "y".to_owned(),
        maps,
        figures: vec![
            figure("$n_\\text{eff}$ at $\\lambda$", r.n_eff[0], 6, "", "TE₀"),
            figure("$n_\\text{eff}$ at $\\lambda/2$", r.n_eff[1], 6, "", "TE₀"),
            figure(
                "Period $\\Lambda$",
                period,
                5,
                "µm",
                "first order, (λ/2)/(n_2ω − n_ω)",
            ),
            figure(
                "$A_\\text{eff}$ at $\\lambda$",
                r.areas[0],
                4,
                "µm²",
                "Afshar and Monro's Eq. 41",
            ),
            figure("$A_\\text{eff}$ at $\\lambda/2$", r.areas[1], 4, "µm²", ""),
            figure(
                "Efficiency $\\eta$",
                r.eta,
                5,
                "%/W/cm²",
                "d₃₃ = 27 pm/V, poled through, 50 % duty",
            ),
        ],
        note: "Zhao et al.'s 300 nm x-cut MgO:LiNbO₃ film on silica, air above, 75° sidewalls, \
               the guide along the crystal's y so its c axis lies across; the full-vector mode \
               solver on 20 nm cells over the film and ridge, half the window behind a mirror \
               wall, the indices Zelmon et al.'s; each picture's colours run to its own peak. On \
               these cells the defaults give 2.8172 µm, 1.0788 and 0.3969 µm² and \
               4160 %/(W·cm²), where 10 and 5 nm cells give 2.8150 and 2.8142 µm and 4164 and \
               4165 %/(W·cm²)."
            .to_owned(),
        ..empty("", "")
    })
}

// ---- self-phase modulation ----

/// A Gaussian pulse (T₀ = 1 ps, 1 W, γ = 1 /(W·m)) over a dispersionless guide whose peak
/// phase is `phase` rad: its spectrum before and after.
pub fn spm(phase: f64) -> photonoxide::Result<ChartData> {
    let (t0, p0, gamma) = (1.0, 1.0, 1.0);
    let start = Pulse::gaussian(p0, t0, 1024, 64.0)?;
    let end = if phase > 0.0 {
        Guide::new(Dispersion::new(1.55, &[0.0])?, gamma, 0.0).propagate(
            &start,
            phase / (gamma * p0),
            8,
        )?
    } else {
        start.clone()
    };
    let spectrum = |p: &Pulse| -> Vec<[f64; 2]> {
        let mut s: Vec<[f64; 2]> = p
            .frequencies()
            .iter()
            .zip(p.spectrum())
            .map(|(w, a)| [w * t0, a.norm_sqr()])
            .filter(|q| q[0].abs() <= 25.0)
            .collect();
        s.sort_by(|a, b| a[0].total_cmp(&b[0]));
        s
    };
    let (a, b) = (spectrum(&start), spectrum(&end));
    let top = a.iter().map(|q| q[1]).fold(0.0, f64::max);
    let scale =
        |v: Vec<[f64; 2]>| -> Vec<[f64; 2]> { v.into_iter().map(|q| [q[0], q[1] / top]).collect() };
    let broadening = end.rms_bandwidth() / start.rms_bandwidth();
    Ok(ChartData {
        x_label: "detuning (ω − ω₀)T₀".to_owned(),
        y_label: "spectral intensity, in the input's peak".to_owned(),
        series: vec![
            curve("the input", scale(a), true, 1),
            curve("after the guide", scale(b), false, 0),
        ],
        figures: vec![
            figure("$\\varphi_\\mathrm{max}/\\pi$", phase / PI, 4, "", "γP₀L"),
            figure("RMS broadening", broadening, 7, "", "the split-step's"),
            figure(
                "Agrawal's Eq. 4.1.16",
                spm_rms_broadening(phase),
                7,
                "",
                "(1 + 4φ_max²/(3√3))^½, exact for the RMS width",
            ),
        ],
        note: "1024 points across 64 T₀ (T₀ = 1 ps), no dispersion or loss, γ = 1 /(W·m) and \
               1 W, the length γP₀L = φ_max: the Kerr step is exact, so the error is the time \
               grid's, round-off at these points."
            .to_owned(),
        ..empty("", "")
    })
}

// ---- solitons ----

/// The NLSE's guide for the soliton chart: β₂ = −0.05 ps²/m, γ = 1 /(W·m), T₀ = 100 fs.
const SOLITON: (f64, f64, f64) = (-0.05, 1.0, 0.1);

/// A sech pulse of soliton order `order` after `distance` soliton periods.
pub fn soliton(order: f64, distance: f64) -> photonoxide::Result<ChartData> {
    let (beta2, gamma, t0) = SOLITON;
    let p1 = soliton_power(gamma, t0, beta2);
    let z0 = soliton_period(t0, beta2);
    let guide = Guide::new(Dispersion::new(1.55, &[beta2])?, gamma, 0.0);
    let start = Pulse::sech(order * order * p1, t0, 1024, 50.0 * t0)?;
    let run = |d: f64| -> photonoxide::Result<Pulse> {
        if d <= 0.0 {
            return Ok(start.clone());
        }
        guide.propagate(&start, d * z0, ((2000.0 * d).ceil() as usize).max(1))
    };
    let end = run(distance)?;
    let profile = |p: &Pulse| -> Vec<[f64; 2]> {
        p.times()
            .iter()
            .zip(p.power())
            .map(|(t, w)| [t / t0, w / p1])
            .filter(|q| q[0].abs() <= 4.0)
            .collect()
    };
    let back = run(1.0)?;
    let returned = back
        .power()
        .iter()
        .zip(start.power())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
        / start.peak_power();
    Ok(ChartData {
        x_label: "time T/T₀".to_owned(),
        y_label: "power, in the fundamental soliton's peak P₁".to_owned(),
        series: vec![
            curve("at z = 0", profile(&start), true, 1),
            curve(
                &format!("at z = {} z₀", sig(distance, 3)),
                profile(&end),
                false,
                0,
            ),
        ],
        figures: vec![
            figure(
                "Peak at $z$",
                end.peak_power() / start.peak_power(),
                5,
                "of the input's",
                "",
            ),
            figure("$P_1$", p1, 4, "W", "|β₂|/(γT₀²)"),
            figure("$L_D$", dispersion_length(t0, beta2), 4, "m", "T₀²/|β₂|"),
            figure("$z_0$", z0, 4, "m", "(π/2)L_D"),
            figure(
                "Back at $z_0$",
                returned,
                3,
                "",
                "the largest change of |A|² from the input's, over its peak: small for whole N",
            ),
        ],
        note: "The symmetrized split-step Fourier method on 2000 steps a soliton period, 1024 \
               points across 50 T₀; its error falls as the step's square (4 × 10⁻⁵ at N = 1 \
               after 5z₀ on 4000 steps)."
            .to_owned(),
        ..empty("", "")
    })
}

// ---- four-wave mixing ----

/// The gain of a signal detuned from a CW pump of `power` W, in a guide of `gamma`, `length` m,
/// `beta2` ps²/m and `beta4` × 10⁻⁴ ps⁴/m.
pub fn fwm_gain(
    power: f64,
    gamma: f64,
    length: f64,
    beta2: f64,
    beta4: f64,
) -> photonoxide::Result<ChartData> {
    let disp = Dispersion::new(1.55, &[beta2, 0.0, beta4 * 1e-4])?;
    let peak = fwm::peak_detuning(gamma, power, beta2);
    // where β₂Ω² or β₄Ω⁴/12 alone would cancel the Kerr shift 2γP
    let reach = (2.0 * gamma * power / beta2.abs().max(1e-6))
        .sqrt()
        .min((24.0 * gamma * power / (beta4.abs() * 1e-4).max(1e-12)).powf(0.25));
    let edge = (2.5 * peak.unwrap_or(reach)).clamp(0.5, 300.0);
    let mut gain = Vec::new();
    let mut idler = Vec::new();
    for w in span(-edge, edge, 801) {
        let dk = disp.four_wave_mismatch(w);
        let f = w / TAU;
        gain.push([f, 10.0 * fwm::signal_gain(gamma, power, dk, length).log10()]);
        idler.push([
            f,
            10.0 * fwm::conversion(gamma, power, dk, length).max(1e-30).log10(),
        ]);
    }
    let gpl = gamma * power * length;
    let mut figures = vec![figure("$\\gamma PL$", gpl, 4, "", "")];
    if let Some(p) = peak {
        let g = fwm::signal_gain(gamma, power, disp.four_wave_mismatch(p), length);
        figures.push(figure(
            "Peak detuning $\\Omega$",
            p,
            4,
            "rad/ps",
            &format!(
                "{} THz, {} nm at 1.55 µm",
                sig(p / TAU, 4),
                sig(
                    1.55e-6 * 1.55e-6 * p * 1e12 / (TAU * SPEED_OF_LIGHT) * 1e9,
                    4
                )
            ),
        ));
        figures.push(figure(
            "Gain there",
            g,
            6,
            "",
            &format!("{} dB", sig(10.0 * g.log10(), 4)),
        ));
    }
    figures.push(figure(
        "$\\cosh^2\\gamma PL$",
        gpl.cosh().powi(2),
        6,
        "",
        "the gain where κ = 0",
    ));
    let bottom = idler
        .iter()
        .chain(&gain)
        .map(|q| q[1])
        .fold(f64::INFINITY, f64::min)
        .max(-40.0);
    let top = gain.iter().map(|q| q[1]).fold(0.0, f64::max);
    Ok(ChartData {
        x_label: "signal's detuning Ω/2π (THz)".to_owned(),
        y_label: "gain and conversion (dB)".to_owned(),
        y_range: Some([bottom.floor(), (top + 1.0).ceil()]),
        series: vec![
            curve("signal gain P_s(L)/P_s(0)", gain, false, 0),
            curve("idler conversion P_i(L)/P_s(0)", idler, true, 1),
        ],
        marker: peak.map(|p| p / TAU),
        figures,
        note: format!(
            "Exact, no grid: the undepleted closed forms with Δk = β₂Ω² + β₄Ω⁴/12 and the \
             effective mismatch Δk + 2γP; P = {} W, γ = {} /(W·m), L = {} m.",
            sig(power, 3),
            sig(gamma, 3),
            sig(length, 3)
        ),
        ..empty("", "")
    })
}

// ---- the Lugiato–Lefever equation ----

/// The uniform states' powers against the detuning at the drive `drive2` = f².
pub fn lle_states(drive2: f64, detuning: f64) -> photonoxide::Result<ChartData> {
    let f = drive2.sqrt();
    let zs = span(-4.0, (drive2 + 6.0).max(10.0), 1201);
    let roots: Vec<Vec<f64>> = zs.iter().map(|&z| homogeneous_powers(z, f)).collect();
    let three: Vec<usize> = (0..zs.len()).filter(|&k| roots[k].len() >= 3).collect();
    let mut upper = Vec::new();
    let mut middle = Vec::new();
    let mut lower = Vec::new();
    for (k, (&z, r)) in zs.iter().zip(&roots).enumerate() {
        let (Some(&lo), Some(&hi)) = (r.first(), r.last()) else {
            continue;
        };
        match (three.first(), three.last()) {
            (Some(&a), Some(&b)) => {
                if k <= b {
                    upper.push([z, hi]);
                }
                if k >= a {
                    lower.push([z, lo]);
                }
                if r.len() >= 3 {
                    middle.push([z, r[1]]);
                }
            }
            _ => upper.push([z, hi]),
        }
    }
    let top = upper.iter().map(|q| q[1]).fold(1.0, f64::max);
    let mut series = vec![curve("stable uniform state", upper, false, 0)];
    if !lower.is_empty() {
        series.push(curve("stable uniform state, lower", lower, false, 0));
        series.push(curve("unstable middle state", middle, true, 0));
    }
    series.push(curve(
        "X = 1: unstable to comb lines above",
        vec![[zs[0], 1.0], [zs[zs.len() - 1], 1.0]],
        true,
        1,
    ));
    let here = homogeneous_powers(detuning, f);
    let mut figures = Vec::new();
    for (k, &x) in here.iter().enumerate() {
        let rate = instability_rate(detuning, x, (2.0 * x - detuning).max(0.0));
        figures.push(figure(
            &format!("State {}: $X$", k + 1),
            x,
            5,
            "",
            &if rate > 0.0 {
                format!("unstable to comb lines, rate {} per τ", sig(rate, 3))
            } else {
                "stable to every line pair".to_owned()
            },
        ));
    }
    figures.push(figure(
        "Bistable above $\\zeta_0$",
        3f64.sqrt(),
        4,
        "",
        "√3: the response turns S shaped",
    ));
    Ok(ChartData {
        x_label: "detuning ζ₀ (half linewidths)".to_owned(),
        y_label: "intracavity power X = |ψ|²".to_owned(),
        y_range: Some([0.0, (top * 1.08).ceil()]),
        series,
        marker: Some(detuning),
        figures,
        note: format!(
            "Exact, no grid: the real roots of f² = X(1 + (X − ζ₀)²) at f² = {}, each state's \
             stability to the line pair of detuning 2X − ζ₀ (the most unstable, reached with \
             anomalous dispersion) by Lugiato and Lefever's Eq. 4. The middle state is \
             unstable to the uniform perturbation too.",
            sig(drive2, 3)
        ),
        ..empty("", "")
    })
}

#[derive(Clone)]
struct Settled {
    lines: Vec<[f64; 2]>,
    approx_lines: Vec<[f64; 2]>,
    peak: f64,
    approx_peak: f64,
    width: f64,
    approx_width: f64,
    decayed: bool,
    time: f64,
}

/// |ψ_μ|² of a field around the ring, by its discrete Fourier transform, line μ = −n/2 … n/2 − 1.
fn lines_of(field: &[c64]) -> Vec<[f64; 2]> {
    let n = field.len();
    let half = n as i64 / 2;
    (-half..half)
        .map(|mu| {
            let s: c64 = field
                .iter()
                .enumerate()
                .map(|(j, v)| v * c64::from_polar(1.0, -TAU * mu as f64 * j as f64 / n as f64))
                .sum();
            [mu as f64, (s / n as f64).norm_sqr()]
        })
        .collect()
}

fn settled(detuning: f64, drive: f64, d2: f64) -> photonoxide::Result<Option<Settled>> {
    static CACHE: OnceLock<Cache<Option<Settled>>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(&[detuning, drive, d2], || {
            let r = Resonator::quadratic(detuning, drive, d2, 512)?;
            let Some(s) = r.soliton(d2) else {
                return Ok(None);
            };
            let start = r.soliton_field(&s);
            let (end, time) = r.settle(&start, 2e-3, 1.0, 1e-9, 400.0)?;
            let peak_of = |f: &[c64]| f.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
            let background = s.background.norm_sqr();
            let peak = peak_of(&end);
            let decayed = peak < 1.5 * background.max(1e-12);
            Ok(Some(Settled {
                lines: lines_of(&end),
                approx_lines: lines_of(&start),
                peak,
                approx_peak: peak_of(&start),
                width: if decayed { f64::NAN } else { peak_width(&end)? },
                // the sech²'s own FWHM, 2 acosh(√2) W, as the write-up's table takes it
                approx_width: 2.0 * 2f64.sqrt().acosh() * s.width,
                decayed,
                time,
            }))
        })
}

/// A single soliton's comb at `detuning`, `drive` and `d2`.
pub fn comb(detuning: f64, drive: f64, d2: f64) -> photonoxide::Result<ChartData> {
    let limit = max_detuning(drive);
    let mut data = ChartData {
        x_label: "comb line μ".to_owned(),
        y_label: "line power |ψ_μ|² (dB, of the strongest line)".to_owned(),
        y_range: Some([-80.0, 2.0]),
        ..empty("", "")
    };
    let Some(s) = settled(detuning, drive, d2)? else {
        data.note = format!(
            "No soliton: the approximation's balance of loss and drive has no root above \
             ζ₀ = π²f²/8 = {}.",
            sig(limit, 4)
        );
        data.figures = vec![figure(
            "$\\pi^2 f^2/8$",
            limit,
            4,
            "",
            "the approximation's limit",
        )];
        return Ok(data);
    };
    let db = |lines: &[[f64; 2]]| -> Vec<[f64; 2]> {
        let top = lines.iter().map(|q| q[1]).fold(0.0, f64::max);
        lines
            .iter()
            .map(|q| [q[0], 10.0 * (q[1] / top).max(1e-12).log10()])
            .filter(|q| q[0].abs() <= 200.0)
            .collect()
    };
    let lines = db(&s.lines);
    let within = lines.iter().filter(|q| q[1] >= -30.0).count();
    data.series = vec![
        curve("settled soliton", lines, false, 0),
        curve("the approximate sech", db(&s.approx_lines), true, 1),
    ];
    data.figures = vec![
        figure(
            "Peak $|\\psi|^2$",
            s.peak,
            4,
            "",
            &format!("the approximation's {}", sig(s.approx_peak, 4)),
        ),
        figure(
            "FWHM",
            s.width,
            4,
            "rad",
            &format!(
                "of the ring's angle; the approximation's {}",
                sig(s.approx_width, 4)
            ),
        ),
        figure(
            "Lines within 30 dB",
            within as f64,
            1,
            "",
            "of the strongest, the pump's included",
        ),
        figure(
            "$\\pi^2 f^2/8$",
            limit,
            4,
            "",
            "the approximation's largest ζ₀",
        ),
    ];
    data.note = if s.decayed {
        format!(
            "The soliton decayed to the uniform state within τ = {}: a 512-line split-step from \
             the approximate soliton, steps of 0.002.",
            sig(s.time, 3)
        )
    } else {
        format!(
            "Strang splitting on 512 lines, steps of 0.002, from the approximate sech soliton \
             until |ψ|² changes by less than 10⁻⁹ of its peak per unit time (τ = {}); its \
             steady state is second order in the step.",
            sig(s.time, 3)
        )
    };
    Ok(data)
}

// ---- photon pairs ----

/// A linear index n = a + bλ: its group index is a.
fn linear_index(a: f64, b: f64) -> photonoxide::Result<Index> {
    let w: Vec<f64> = (0..=27).map(|k| 0.3 + 0.1 * f64::from(k)).collect();
    let n = w.iter().map(|l| a + b * l).collect();
    Index::effective(w, n)
}

/// The group indices of the signal and the pump, and the pump's wavelength, µm.
const PAIRS: (f64, f64, f64) = (2.30, 2.40, 0.775);

/// Degenerate down-conversion over `length_mm` pumped by `pump_nm`, the idler's group index
/// `idler`, sinc (`matching` 1) or Gaussian (2) phase matching.
pub fn jsa(
    pump_nm: f64,
    length_mm: f64,
    idler: f64,
    matching: f64,
) -> photonoxide::Result<ChartData> {
    let (gs, gp, lp) = PAIRS;
    let length = length_mm * 1e3;
    let process = Process::difference_frequency(
        Wave::new(um(lp)?, linear_index(gp, -0.25)?),
        Wave::new(um(2.0 * lp)?, linear_index(gs, -0.08)?),
        linear_index(idler, -0.06)?,
    )?;
    let device = Device::quasi_phase_matched(process, length, State::default(), 1, 0.5)?;
    let bandwidth = pump_nm * 1e-3 / (lp * lp);
    let pump = Pump {
        center: 1.0 / lp,
        bandwidth,
    };
    let gaussian = matching.round() as i64 == 2;
    let kind = if gaussian {
        Matching::Gaussian
    } else {
        Matching::Sinc
    };
    // the Gaussian amplitude's covariance sets the window: ln f = −p(δs + δi)² − g(Aδs + Bδi)²
    let p = 2.0 * LN_2 / (bandwidth * bandwidth);
    let g = gaussian_gamma() * (PI * length).powi(2);
    let (a, b) = (gp - gs, gp - idler);
    let q = [
        [p + g * a * a, p + g * a * b],
        [p + g * a * b, p + g * b * b],
    ];
    let det = q[0][0] * q[1][1] - q[0][1] * q[0][1];
    // |f|² = exp(−2 zᵀqz): the variances are q⁻¹/4
    let sigma = [(q[1][1] / det / 4.0).sqrt(), (q[0][0] / det / 4.0).sqrt()];
    let reach = 6.0 * sigma[0].max(sigma[1]);
    let n = 201;
    let jsa = joint_spectrum(&device, pump, kind, reach, n)?;
    let schmidt = jsa.schmidt()?;
    let purity = schmidt.purity();
    // the picture, on even wavelengths: f from the device's phase and the pump's amplitude
    let state = *device.state();
    let nu0 = 0.5 / lp;
    // the picture: half the purity's window, which holds the amplitude's tails
    let shown = 0.5 * reach;
    let (near, far) = (1.0 / (nu0 + shown), 1.0 / (nu0 - shown));
    let cells = 160;
    let mut values = vec![0.0f64; cells * cells];
    let pick = |x: f64| {
        if gaussian {
            (-gaussian_gamma() * x * x).exp()
        } else if x.abs() < 1e-4 {
            1.0 - x * x / 6.0
        } else {
            x.sin() / x
        }
    };
    for j in 0..cells {
        let li = near + (far - near) * (j as f64 + 0.5) / cells as f64;
        for i in 0..cells {
            let ls = near + (far - near) * (i as f64 + 0.5) / cells as f64;
            let (s, id) = (1.0 / ls, 1.0 / li);
            let x = device.phase_at_frequencies(&[s, id, s + id], &state)?;
            values[j * cells + i] = (pump.amplitude(s + id) * pick(x)).powi(2);
        }
    }
    let top = values
        .iter()
        .copied()
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    let raster = Raster {
        nx: cells,
        ny: cells,
        x0: near,
        x1: far,
        y0: near,
        y1: far,
        values: values
            .iter()
            .map(|v| ((v / top) * 1e3).round() as f32 / 1e3)
            .collect(),
    };
    // the signal's marginal spectrum's FWHM, nm
    let ni = jsa.idler.len();
    let marginal: Vec<f64> = (0..jsa.signal.len())
        .map(|k| {
            jsa.amplitude[k * ni..(k + 1) * ni]
                .iter()
                .map(|v| v.norm_sqr())
                .sum()
        })
        .collect();
    let half = 0.5 * marginal.iter().copied().fold(0.0, f64::max);
    let above: Vec<usize> = (0..marginal.len())
        .filter(|&k| marginal[k] >= half)
        .collect();
    let fwhm = match (above.first(), above.last()) {
        (Some(&i), Some(&j)) => (1.0 / jsa.signal[i] - 1.0 / jsa.signal[j]).abs() * 1e3,
        _ => f64::NAN,
    };
    let mut figures = vec![
        figure(
            "Purity $P$",
            purity,
            6,
            "",
            "Σλₖ², the Schmidt decomposition's",
        ),
        figure("Schmidt number $K$", 1.0 / purity, 4, "", "1/P"),
    ];
    if gaussian {
        figures.push(figure(
            "Closed form",
            gaussian_purity(q[0][0], q[1][1], q[0][1]),
            6,
            "",
            "(1 − c²/ab)^½ of the Gaussian",
        ));
    }
    figures.push(figure(
        "Signal's FWHM",
        fwhm,
        3,
        "nm",
        "of its marginal spectrum, on the grid",
    ));
    Ok(ChartData {
        x_label: "signal wavelength λₛ".to_owned(),
        x_length: true,
        y_label: "idler wavelength λᵢ".to_owned(),
        maps: vec![FieldMap {
            label: "|f(λₛ, λᵢ)|², the joint spectral intensity".to_owned(),
            raster,
            outline: Vec::new(),
        }],
        figures,
        note: format!(
            "The library's joint spectrum on 201 × 201 frequencies ±{} /µm about degeneracy, \
             its singular values for the purity; the picture on even wavelengths, its colours to \
             its own peak. A model guide: indices linear in the wavelength, group indices \
             {gs} (signal), {gp} (pump) and {} (idler), a grating cut for 1.55 µm, {} mm; a \
             Gaussian pump {} nm wide at 0.775 µm.",
            sig(reach, 3),
            sig(idler, 4),
            sig(length_mm, 3),
            sig(pump_nm, 3)
        ),
        ..empty("", "")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap_or(f64::INFINITY)
    }

    /// Carried from 1.064 µm, congruent LiNbO₃'s d₃₃ is 23.58 pm/V at 1.313 µm and 28.12 at
    /// 0.852 (docs/methods/nonlinear.md, "Miller's rule"), MgO:LN's d₃₁ 4.11 and 4.92.
    #[test]
    fn miller_is_the_write_ups() {
        let ln = miller(1.0, 2.0).unwrap();
        assert!((fig(&ln, "d₃₃ at 1.313") - 23.58).abs() < 5e-3);
        assert!((fig(&ln, "d₃₃ at 0.852") - 28.12).abs() < 5e-3);
        let mgo = miller(2.0, 2.0).unwrap();
        assert!((fig(&mgo, "d₃₁ at 1.313") - 4.11).abs() < 5e-3);
        assert!((fig(&mgo, "d₃₁ at 0.852") - 4.92).abs() < 5e-3);
        // the rule is the identity at its own fundamental: the curve passes the measurement
        let at = |c: &Curve, l: f64| {
            c.points
                .iter()
                .min_by(|a, b| (a[0] - l).abs().total_cmp(&(b[0] - l).abs()))
                .unwrap()[1]
        };
        assert!(ln.series[0].points.len() > 100);
        assert!((at(&ln.series[0], 1.064) - 25.2).abs() < 0.1);
    }

    /// Matched, the depleted harmonic is Armstrong et al.'s tanh²(κ√P z) and the undepleted
    /// ηPz²; matched parametric gain is cosh² ΓL, mismatched 1 + (Γ/g)² sinh² gL, as long as
    /// the pump doesn't deplete.
    #[test]
    fn coupled_is_armstrongs() {
        // κ√P L = 2 at 4.5e-7 /(W µm²) and 4 mm: P = 1/(κL/2)² W
        let kappa = coupling_from_efficiency(4500.0);
        let p = (2.0 / (kappa * 4000.0)).powi(2);
        let d = coupled(1.0, 4500.0, p * 1e3, 4.0, 0.0).unwrap();
        assert!((fig(&d, "$P_{2\\omega}/P$") - 2f64.tanh().powi(2)).abs() < 1e-9);
        assert!((fig(&d, "$\\kappa") - 2.0).abs() < 1e-12);
        for (a, b) in d.series[0].points.iter().zip(&d.series[2].points) {
            assert!((a[1] - b[1]).abs() < 1e-9, "{a:?} {b:?}");
        }
        let weak = coupled(1.0, 4500.0, 1e-3, 4.0, 1.0).unwrap();
        let u = fig(&weak, "Undepleted");
        assert!(
            (fig(&weak, "$P_{2\\omega}/P$") / u - 1.0).abs() < 1e-5,
            "{u}"
        );
        for (phase, tolerance) in [(0.0, 1e-9), (1.0, 1e-7)] {
            let g = coupled(2.0, 4500.0, 500.0, 4.0, phase).unwrap();
            let (ours, closed) = (fig(&g, "Gain at"), fig(&g, "Undepleted"));
            assert!(
                (ours / closed - 1.0).abs() < tolerance,
                "{phase}: {ours} {closed}"
            );
            if phase == 0.0 {
                let gl = fig(&g, "$\\Gamma L$");
                assert!((closed - gl.cosh().powi(2)).abs() < 1e-9 * closed);
            }
        }
    }

    /// Without a grating the harmonic peaks at (2/π)² after one coherence length; a first-order
    /// grating at 50 % grows on average as (2/π)² of the matched; the second order at 50 % not at
    /// all; Wang et al.'s 39 % cuts G₁ by 5.91 %.
    #[test]
    fn qpm_is_fejers() {
        let d = qpm(1.0, 0.5, 40.0).unwrap();
        assert!((fig(&d, "No grating's") - 4.0 / (PI * PI)).abs() < 1e-12);
        let g2 = (2.0 / PI).powi(2);
        assert!((fig(&d, "$G_m^2$") - g2).abs() < 1e-12);
        let ratio = fig(&d, "Poled over");
        assert!((ratio / g2 - 1.0).abs() < 0.05, "{ratio}");
        // at the end of every whole period the poled field is G·z exactly for D = ½ and m = 1:
        // |∫| over one period is 4/π = (2/π)·2
        assert!((ratio - g2).abs() < 1e-9, "{ratio}");
        let even = qpm(2.0, 0.5, 40.0).unwrap();
        assert!(fig(&even, "Poled over") < 1e-3);
        let wang = qpm(1.0, 0.39, 40.0).unwrap();
        assert!((fig(&wang, "$G_m$") / (2.0 / PI) - (1.0 - 0.0591)).abs() < 1e-4);
    }

    /// At 1.064 µm over 1 mm the chart's numbers are Fejer et al.'s Table I as `qpm_fejer`
    /// prints them: l_c 3.40124 µm, δλ 2.03597 nm, δT 25.7704 K; the period 6.802 µm; and the
    /// curve passes ½ at the numerical width's ends.
    #[test]
    fn acceptance_is_fejers() {
        let d = acceptance(1.064, 1.0, 1.0).unwrap();
        assert!((fig(&d, "Coherence") - 3.40124).abs() < 1e-4);
        assert!((fig(&d, "Period") - 6.802).abs() < 5e-4);
        assert!((fig(&d, "$\\delta\\lambda$, Fejer") - 2.03597).abs() < 1e-4);
        assert!((fig(&d, "$\\delta T$, Fejer") - 25.7704).abs() < 1e-3);
        let w = fig(&d, "$\\delta\\lambda$ between");
        assert!((w - 2.036).abs() < 0.01, "{w}");
        let device = ln_shg(1.064, 1.0).unwrap();
        let a = device.acceptance(Variable::Wavelength).unwrap();
        for edge in [a.lower, a.upper] {
            let r = sinc2(device.phase_tuned(edge, device.state()).unwrap());
            assert!((r - 0.5).abs() < 1e-6, "{r}");
        }
        let long = acceptance(2.12, 1.0, 1.0).unwrap();
        assert!((fig(&long, "$\\delta\\lambda$, Fejer") - 61.574).abs() < 1e-2);
        let t = acceptance(1.064, 1.0, 2.0).unwrap();
        assert!(t.series[0].points.len() > 300);
    }

    /// Zhao et al.'s ridge on 20 nm cells: the period 2.8172 µm, the areas 1.0788 and 0.3969
    /// µm² (docs/methods/nonlinear.md, "Zhao et al.'s guide") and η 4160.1 %/(W·cm²) (the
    /// job's case `job/nonlinear-ridge-zhao-efficiency`).
    #[test]
    fn ridge_is_zhaos() {
        let d = ridge(1.24, 50.0, 1.57).unwrap();
        assert!((fig(&d, "Period") - 2.8172).abs() < 5e-5);
        assert!((fig(&d, "$A_\\text{eff}$ at $\\lambda$") - 1.0788).abs() < 5e-5);
        assert!((fig(&d, "$A_\\text{eff}$ at $\\lambda/2") - 0.3969).abs() < 5e-5);
        assert!((fig(&d, "Efficiency") - 4160.1).abs() < 0.05);
        assert_eq!(d.maps.len(), 2);
        for m in &d.maps {
            assert_eq!(m.raster.values.iter().copied().fold(0.0, f32::max), 1.0);
        }
    }

    /// The RMS broadening is Agrawal's Eq. 4.1.16 to round-off: 12.443951 at 4.5π.
    #[test]
    fn spm_is_agrawals() {
        let d = spm(4.5 * PI).unwrap();
        let ours = fig(&d, "RMS");
        assert!((ours - 12.443951).abs() < 1e-6, "{ours}");
        assert!((ours - fig(&d, "Agrawal")).abs() < 1e-8);
    }

    /// The fundamental soliton keeps its peak; the third-order one returns to its input after
    /// a soliton period, to the split-step's 2.0 × 10⁻⁴ of its peak on 2000 steps.
    #[test]
    fn solitons_return() {
        let one = soliton(1.0, 1.0).unwrap();
        assert!((fig(&one, "Peak at") - 1.0).abs() < 1e-4);
        assert!((fig(&one, "$P_1$") - 5.0).abs() < 1e-12);
        let three = soliton(3.0, 0.5).unwrap();
        assert!(fig(&three, "Back at") < 1e-3, "{}", fig(&three, "Back at"));
        assert!(
            fig(&three, "Peak at") > 2.0,
            "a third-order soliton narrows at z₀/2"
        );
    }

    /// The gain's peak, at Ω = (2γP/|β₂|)^½ = 6.32 rad/ps, is cosh² γPL = 14.154116 at γPL = 2
    /// (`nonlinear/fwm-gain-matched`); β₄ enters as β₄Ω⁴/12.
    #[test]
    fn fwm_is_agrawals() {
        let d = fwm_gain(1.0, 1.0, 2.0, -0.05, 0.0).unwrap();
        assert!((fig(&d, "Peak detuning") - 40f64.sqrt()).abs() < 1e-12);
        assert!((fig(&d, "Gain there") - 14.154116).abs() < 1e-6);
        let top = d.series[0].points.iter().map(|q| q[1]).fold(0.0, f64::max);
        assert!((top - 10.0 * 14.154116f64.log10()).abs() < 0.01, "{top}");
        let disp = Dispersion::new(1.55, &[-0.05, 0.0, 1e-4]).unwrap();
        let w: f64 = 5.0;
        assert!(
            (disp.four_wave_mismatch(w) - (-0.05 * w * w + 1e-4 * w.powi(4) / 12.0)).abs() < 1e-12
        );
    }

    /// At ζ₀ = 3 and f² = 4 the uniform states are 2 − √2, 2 and 2 + √2 (docs/methods/nonlinear.md,
    /// "Steady states and bistability"), the middle and upper unstable to comb lines.
    #[test]
    fn lle_states_are_lugiato_lefevers() {
        let d = lle_states(4.0, 3.0).unwrap();
        let s2 = 2f64.sqrt();
        for (k, x) in [2.0 - s2, 2.0, 2.0 + s2].iter().enumerate() {
            assert!((fig(&d, &format!("State {}", k + 1)) - x).abs() < 1e-9);
        }
        assert!(d.figures[0].note.starts_with("stable"));
        assert!(d.figures[2].note.starts_with("unstable"));
        assert_eq!(d.series.len(), 4);
    }

    /// At f = 3, d₂ = 0.004 and ζ₀ = 10 the settled soliton peaks at 20.74 and is 0.03423 rad
    /// wide, the approximation 19.50 and 0.03525 (docs/methods/nonlinear.md, "Dissipative Kerr
    /// solitons"); beyond π²f²/8 there is none.
    #[test]
    fn comb_is_the_write_ups() {
        let d = comb(10.0, 3.0, 0.004).unwrap();
        assert!(
            (fig(&d, "Peak") - 20.74).abs() < 0.02,
            "{}",
            fig(&d, "Peak")
        );
        assert!(
            (fig(&d, "FWHM") - 0.03423).abs() < 5e-5,
            "{}",
            fig(&d, "FWHM")
        );
        assert!(d.figures[0].note.contains("19.5"), "{}", d.figures[0].note);
        assert!(
            d.figures[1].note.contains("0.03525"),
            "{}",
            d.figures[1].note
        );
        let none = comb(13.0, 3.0, 0.004).unwrap();
        assert!(none.series.is_empty());
    }

    /// The Gaussian source of `nonlinear/pairs-gaussian-purity` has the purity 0.190323, the
    /// closed form; group-velocity matching the idler opposite the signal raises it.
    #[test]
    fn jsa_is_the_closed_form() {
        // 1e-3 /µm wide at 0.775 µm, the case's pump
        let d = jsa(1e-3 * 0.775 * 0.775 * 1e3, 10.0, 2.25, 2.0).unwrap();
        let p = fig(&d, "Purity");
        assert!((p - 0.190323).abs() < 1e-6, "{p}");
        assert!((p / fig(&d, "Closed form") - 1.0).abs() < 1e-8);
        let matched = jsa(0.6, 10.0, 2.5, 2.0).unwrap();
        assert!(fig(&matched, "Purity") > 0.8, "{}", fig(&matched, "Purity"));
        let sinc = jsa(0.6, 10.0, 2.25, 1.0).unwrap();
        assert!(sinc.figures.iter().all(|f| f.label != "Closed form"));
        assert_eq!(d.maps[0].raster.values.len(), 160 * 160);
    }
}

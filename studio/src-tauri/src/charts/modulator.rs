//! The charts of the lesson on modulators and phase shifters, each on one of the library's active
//! devices or on the travelling-wave line's response:
//!
//! - **A heater's power and time constant** (`heater-power`): the thermo-optic phase shifters'
//!   closed forms (`ThermoShifter`'s analytic fidelity, `device::thermo_closed`), Pπ and the 1/e
//!   time against the heater's height over a TiN heater's guide or the doped heaters' distance
//!   from a rib, over the analytic model's validity range.
//! - **A Mach–Zehnder modulator's transmission** (`mzm-transfer`): the TFLN Mach–Zehnder
//!   modulator's component (`TflnModulator`, its 2D model at Zhang et al. 2022's design) against
//!   the drive voltage, at a bias and a frequency.
//! - **A travelling-wave electrode's response** (`modulator-response`): Ghione's Eq. (6.31)
//!   (`electro::line::response`) for a line of a given RF index, impedance and loss between
//!   50 Ω ends, with its two limits, the walk-off's |sin U/U| and the loss's (1 − e^(−αL))/αL.
//! - **A phase-change cell's index change** (`pcm-shift`): the phase-change phase shifters'
//!   effective index method (`PcmShifter`'s analytic fidelity), Δn_eff from the amorphous to the
//!   crystalline state against the film's thickness, for Sb₂Se₃, Sb₂S₃ and GST.

use photonoxide::portable;
use std::f64::consts::{LN_10, PI};

use num_complex::Complex64 as c64;
use photonoxide::circuit::Fidelity;
use photonoxide::device::{Device, PcmShifter, TflnModulator, ThermoShifter, Values};
use photonoxide::electro::line::{Drive, falls_to, response};
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The modulator's wavelength, µm: where the thin film's r₃₃ is given.
const LAMBDA: f64 = 1.55;
/// The light's group index in the response chart: Zhang et al. 2022's TE mode, simulated (their
/// Section 2.A); their measured line runs about 0.04 above it (docs/methods/travelling-wave.md).
const GROUP_INDEX: f64 = 2.26;
/// dB in a neper.
const DB_PER_NEPER: f64 = 20.0 / LN_10;
/// The frequency the responses are referred to, Hz, as Zhang et al. refer theirs.
const REFERENCE: f64 = 1e9;

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

/// The figure `symbol` of `figures`, its value and grid.
fn read(
    figures: &[photonoxide::device::Figure],
    symbol: &str,
) -> photonoxide::Result<(f64, String)> {
    figures
        .iter()
        .find(|f| f.symbol == symbol)
        .map(|f| (f.value, f.grid.clone()))
        .ok_or_else(|| invalid("device", format!("the model gives no {symbol}")))
}

/// The chart specs.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "heater-power",
            title: "A heater's Pπ and time constant",
            about: "A thermo-optic phase shifter's power for a phase of π and the 1/e time of its phase turned on, against the heater's distance from the guide: a TiN heater 7.5 µm wide above a silicon strip, or two N++ strips in the slab beside a rib, at Jacques et al. 2019's designs otherwise, by the device's closed forms.",
            computed_by: "device::ThermoShifter::metal and ::doped at the analytic fidelity (Device::derived): the oxide as a slab between an isothermal substrate and an insulated top, its eigenfunction series in closed form, steady, at a frequency and in time; Marcatili's dn_eff/dT",
            params: vec![
                param(
                    "kind",
                    "Heater",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1: a TiN heater above a strip, against its height; 2: N++ strips in the slab, against their distance from the rib.",
                ),
                param(
                    "strip",
                    "Guide width $w$",
                    "um",
                    (0.4, 0.6, 0.5),
                    0.01,
                    false,
                    "The silicon guide's width, µm, 220 nm thick: the closed forms' validity range.",
                ),
            ],
        },
        ChartSpec {
            id: "mzm-transfer",
            title: "A Mach–Zehnder modulator's transmission against its voltage",
            about: "The thin-film lithium niobate Mach–Zehnder modulator at Zhang et al. 2022's cross-section, driven in push and pull: the power it transmits against the voltage on its electrode, at a bias and a drive frequency. Its Vπ·L is the 2D model's at DC; at a frequency the electrode's response m(f) scales the voltage.",
            computed_by: "device::TflnModulator at the 2D fidelity (Device::component and its S-matrix): the static solver's field and the full-vector mode's perturbation for Vπ·L (mode::pockels), the quasi-TEM line (electro::line) and Ghione's Eq. 6.31 for m(f)",
            params: vec![
                param(
                    "length",
                    "Electrode length $L$",
                    "um",
                    (1000.0, 20000.0, 5800.0),
                    100.0,
                    false,
                    "The electrode's length, µm: Zhang et al. made 5.8 and 10.8 mm.",
                ),
                param(
                    "bias",
                    "Bias $\\varphi_0/\\pi$",
                    "",
                    (-1.0, 1.0, 0.5),
                    0.01,
                    false,
                    "The arms' phase difference with no voltage, in units of π: 0.5 is quadrature.",
                ),
                param(
                    "frequency",
                    "Frequency $f$ (GHz)",
                    "",
                    (0.0, 100.0, 0.0),
                    1.0,
                    false,
                    "The drive's frequency, GHz, 0 for DC: the quasi-TEM line holds to about 100 GHz.",
                ),
            ],
        },
        ChartSpec {
            id: "modulator-response",
            title: "A travelling-wave electrode's electro-optic response",
            about: "The modulation a travelling-wave electrode gives against the frequency, referred to 1 GHz: a line of RF index n_m, impedance Z₀ and a loss growing as a√f + bf, between a 50 Ω source and a 50 Ω load, beside light of group index 2.26. Dashed, its two limits: the walk-off alone on a matched lossless line, and the loss alone on a matched line at the light's speed.",
            computed_by: "electro::line::response (Ghione 2009, Eq. 6.31; Zhang et al. 2022's Eq. 1 for equal ends) and electro::line::falls_to; the limits |sin U/U| and (1 − e^(−αL))/(αL) of Ghione's Section 6.5.2",
            params: vec![
                param(
                    "mismatch",
                    "Index mismatch $n_m - n_o$",
                    "",
                    (-2.5, 2.5, 0.04),
                    0.005,
                    false,
                    "The RF index above the light's group index: Zhang et al.'s line is 0.04 above; a bulk crystal's about 2.",
                ),
                param(
                    "loss",
                    "Conductor loss $a$ (dB cm⁻¹ GHz⁻½)",
                    "",
                    (0.0, 1.5, 0.42),
                    0.01,
                    false,
                    "The loss's √f part, dB/cm at 1 GHz: Zhang et al.'s fit 0.42.",
                ),
                param(
                    "dielectric",
                    "Dielectric loss $b$ (dB cm⁻¹ GHz⁻¹)",
                    "",
                    (0.0, 0.1, 0.025),
                    0.001,
                    false,
                    "The loss's f part, dB/cm per GHz: Zhang et al.'s fit 0.025.",
                ),
                param(
                    "impedance",
                    "Line impedance $Z_0$ (Ω)",
                    "",
                    (20.0, 80.0, 41.5),
                    0.5,
                    false,
                    "The line's characteristic impedance, Ω: Zhang et al.'s 41.5, between 50 Ω ends.",
                ),
                param(
                    "length",
                    "Electrode length $L$",
                    "um",
                    (1000.0, 30000.0, 5800.0),
                    100.0,
                    false,
                    "The electrode's length, µm.",
                ),
            ],
        },
        ChartSpec {
            id: "pcm-shift",
            title: "A phase-change cell's index change against its thickness",
            about: "A film of Sb₂Se₃, Sb₂S₃ or GST on a 220 nm silicon strip at 1.55 µm, TE-like: how far the guide's effective index moves when the film turns from amorphous to crystalline, against the film's thickness, by the device's effective index method with exact complex slabs.",
            computed_by: "device::PcmShifter::sb2se3, ::sb2s3 and ::gst at the analytic fidelity (Device::derived): the effective index method with exact complex slabs (mode::multilayer), the materials our fits to Delaney et al. 2020's ellipsometry",
            params: vec![
                param(
                    "thickness",
                    "Film thickness $t_f$",
                    "nm",
                    (10.0, 50.0, 25.0),
                    1.0,
                    false,
                    "The film whose figures are given, nm: Delaney et al.'s 25 nm.",
                ),
                param(
                    "width",
                    "Guide width $w$",
                    "um",
                    (0.4, 0.6, 0.5),
                    0.01,
                    false,
                    "The silicon strip's width, µm.",
                ),
            ],
        },
    ]
}

/// Pπ and the 1/e time of a TiN heater (`kind` 1) against its height over the strip, 1 to 3
/// µm, or of doped heaters (`kind` 2) against their distance from the rib, 0.4 to 1.6 µm, the
/// guide `strip` µm wide; the figures at Jacques et al.'s design (2 and 0.8 µm).
pub fn heater_power(kind: f64, strip: f64) -> photonoxide::Result<ChartData> {
    let (device, key, xs, design, x_label, what) = if kind < 1.5 {
        (
            ThermoShifter::metal(),
            "height",
            span(1.0, 3.0, 21),
            2.0,
            "the heater's height above the strip h (µm)",
            "a TiN heater 7.5 µm wide above",
        )
    } else {
        (
            ThermoShifter::doped(),
            "buffer",
            span(0.4, 1.6, 25),
            0.8,
            "the N++ strips' distance from the rib d (µm)",
            "1.0 µm N++ strips in the 90 nm slab beside",
        )
    };
    let at = |x: f64| -> photonoxide::Result<Vec<photonoxide::device::Figure>> {
        let values = Values::new(&device, &[(key, x), ("width", strip)])?;
        device.derived(&values, Fidelity::Analytic)
    };
    let (mut power, mut time) = (Vec::new(), Vec::new());
    for &x in &xs {
        let f = at(x)?;
        power.push([x, read(&f, "P_pi")?.0]);
        time.push([x, read(&f, "tau")?.0]);
    }
    let here = at(design)?;
    let (p, _) = read(&here, "P_pi")?;
    let (tau, _) = read(&here, "tau")?;
    let (f3, _) = read(&here, "f_3dB")?;
    let top = power.iter().chain(&time).map(|q| q[1]).fold(0.0, f64::max);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: x_label.to_owned(),
        x_length: false,
        y_label: "Pπ (mW) and τ (µs)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.1 * top]),
        series: vec![
            curve("Pπ (mW)", power, false, 0),
            curve("τ, the 1/e time (µs)", time, false, 1),
        ],
        marker: Some(design),
        figures: vec![
            figure(
                "$P_\\pi$ at the design",
                p,
                4,
                "mW",
                "the analytic model; exact: no grid",
            ),
            figure(
                "$\\tau$, the phase's 1/e time",
                tau,
                3,
                "µs",
                "turned on from the switch",
            ),
            figure(
                "$f_{3\\,\\mathrm{dB}}$",
                f3,
                4,
                "kHz",
                "the phase's small-signal −3 dB frequency",
            ),
            figure(
                "$P_\\pi\\tau$",
                p * tau,
                3,
                "nJ",
                "Jacques et al.'s figure of merit: smaller is better",
            ),
            figure(
                "$2\\pi f_{3\\,\\mathrm{dB}}\\,\\tau$",
                2.0 * PI * f3 * 1e3 * tau * 1e-6,
                3,
                "",
                "1 for a single pole: heat's response is many",
            ),
        ],
        note: format!(
            "Exact, no grid: the device's closed forms at the analytic fidelity, {what} a \
             {} µm × 220 nm silicon guide on 2 µm of buried oxide, 320 µm long, Jacques et al.'s \
             materials at 300 K, 1.55 µm. Pπ is a straight section's, whatever its length (a 2D \
             number). The analytic Pπ is within 15 % of the 2D model's over this range.",
            sig(strip, 3)
        ),
    })
}

/// The TFLN Mach–Zehnder modulator's transmission |S₂₁|² against the voltage, `length_mm`
/// long, biased at `bias_pi` π, driven at `frequency_ghz` (0 for DC), by its 2D model.
pub fn mzm_transfer(
    length_mm: f64,
    bias_pi: f64,
    frequency_ghz: f64,
) -> photonoxide::Result<ChartData> {
    let device = TflnModulator::new();
    let w = Wavelength::um(LAMBDA)?;
    let base = [
        ("length", length_mm * 1e3),
        ("bias", bias_pi * PI),
        ("frequency", frequency_ghz),
    ];
    let dc = Values::new(
        &device,
        &[("length", length_mm * 1e3), ("bias", bias_pi * PI)],
    )?;
    let dc_figures = device.derived(&dc, Fidelity::TwoD)?;
    let (vpi_l, grid) = read(&dc_figures, "VpiL")?;
    let (vpi, _) = read(&dc_figures, "Vpi")?;
    let transmission = |volts: f64, frequency: f64| -> photonoxide::Result<f64> {
        let values = Values::new(
            &device,
            &[
                ("length", length_mm * 1e3),
                ("bias", bias_pi * PI),
                ("frequency", frequency),
                ("voltage", volts),
            ],
        )?;
        Ok(device
            .component(&values, Fidelity::TwoD)?
            .s_matrix(w)?
            .power(1, 0))
    };
    let volts = span(-2.0 * vpi, 2.0 * vpi, 81);
    let mut series = Vec::new();
    let mut points = Vec::with_capacity(volts.len());
    for &v in &volts {
        points.push([v, transmission(v, frequency_ghz)?]);
    }
    let here = device.derived(&Values::new(&device, &base)?, Fidelity::TwoD)?;
    let (m_db, _) = read(&here, "m_dB")?;
    let m = portable::powf(10f64, m_db / 20.0);
    series.push(curve(
        &if frequency_ghz > 0.0 {
            format!("at {} GHz", sig(frequency_ghz, 3))
        } else {
            "at DC".to_owned()
        },
        points,
        false,
        0,
    ));
    if frequency_ghz > 0.0 {
        let mut dc_points = Vec::with_capacity(volts.len());
        for &v in &volts {
            dc_points.push([v, transmission(v, 0.0)?]);
        }
        series.push(curve("at DC", dc_points, true, 1));
    }
    // the slope at the bias, dT/dV = −(π|m|/2Vπ) sin(φ₀): the component's T = cos²(φ/2)
    let slope = -(PI * m / (2.0 * vpi)) * portable::sin(bias_pi * PI);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "drive voltage V (V)".to_owned(),
        x_length: false,
        y_label: "transmission |S₂₁|²".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05]),
        series,
        marker: Some(0.0),
        figures: vec![
            figure(
                "$V_\\pi L$, push–pull",
                vpi_l,
                4,
                "V·cm",
                &format!("2D, DC: {grid}"),
            ),
            figure("$V_\\pi$ at DC", vpi, 4, "V", "Vπ·L over the length"),
            figure(
                "$\\lvert m(f)\\rvert$, the electrode's response",
                m,
                4,
                "",
                "from DC, by the 2D model's quasi-TEM line",
            ),
            figure("$V_\\pi$ at the frequency", vpi / m, 4, "V", "Vπ / |m(f)|"),
            figure(
                "$dT/dV$ at the bias",
                slope,
                3,
                "1/V",
                "steepest at quadrature, φ₀ = ±π/2",
            ),
        ],
        note: format!(
            "2D: Zhang et al. 2022's cross-section (a 600 nm x-cut film etched 300 nm to ribs \
             1.2 µm wide, 5 µm gaps beside a 20 µm signal, 0.83 µm of copper), Vπ·L by the \
             static solver's field and the full-vector mode's perturbation, at 1.55 µm, the \
             thin film's r₃₃ = 26.9 pm/V; ideal Y-branches, no propagation loss. Grids: {grid}."
        ),
    })
}

/// The response of a line of RF index n_o + `mismatch`, impedance `impedance` Ω and loss
/// `loss`√f + `dielectric` f dB/cm (f in GHz), `length_mm` long between 50 Ω ends, from 1 to
/// 300 GHz, referred to 1 GHz; and its walk-off and loss limits.
pub fn modulator_response(
    mismatch: f64,
    loss: f64,
    dielectric: f64,
    impedance: f64,
    length_mm: f64,
) -> photonoxide::Result<ChartData> {
    let length = length_mm * 1e-3;
    let drive = Drive::new(50.0, GROUP_INDEX, length)?;
    // γ = α + jβ, 1/m, of a line of index n and loss a√f + bf dB/cm at `hz`
    let gamma = |hz: f64, n: f64, a: f64, b: f64| {
        let ghz = hz * 1e-9;
        let alpha = (a * ghz.sqrt() + b * ghz) * 100.0 / DB_PER_NEPER;
        c64::new(alpha, 2.0 * PI * hz * n / SPEED_OF_LIGHT)
    };
    let n_m = GROUP_INDEX + mismatch;
    let full = |hz: f64| {
        response(
            gamma(hz, n_m, loss, dielectric),
            c64::new(impedance, 0.0),
            hz,
            &drive,
        )
    };
    // the limits: a lossless matched line of the mismatch, and a matched one at the light's speed
    let walk_off = |hz: f64| {
        let u = 2.0 * PI * hz * mismatch * length / (2.0 * SPEED_OF_LIGHT);
        if u.abs() < 1e-12 {
            1.0
        } else {
            (portable::sin(u) / u).abs()
        }
    };
    let lossy = |hz: f64| {
        let al = gamma(hz, GROUP_INDEX, loss, dielectric).re * length;
        if al < 1e-12 {
            1.0
        } else {
            (1.0 - portable::exp(-al)) / al
        }
    };
    let db = |m: &dyn Fn(f64) -> f64, hz: f64| 20.0 * portable::log10(m(hz) / m(REFERENCE));
    let ghz = span(1.0, 300.0, 300);
    let line_of = |m: &dyn Fn(f64) -> f64| -> Vec<[f64; 2]> {
        ghz.iter().map(|&g| [g, db(m, g * 1e9)]).collect()
    };
    let band = |m: &dyn Fn(f64) -> f64, level: f64| -> photonoxide::Result<Option<f64>> {
        Ok(falls_to(|hz| Ok(db(m, hz)), level, REFERENCE, 1e12)?.map(|hz| hz * 1e-9))
    };
    let text = |f: Option<f64>| f.unwrap_or(f64::INFINITY);
    let f3 = band(&full, -3.0)?;
    let f6 = band(&full, -6.0)?;
    let f3_walk = band(&walk_off, -3.0)?;
    let f3_loss = band(&lossy, -3.0)?;
    let low = 20.0 * portable::log10(full(REFERENCE));
    let series = vec![
        curve("the line", line_of(&full), false, 0),
        curve("walk-off alone: |sin U/U|", line_of(&walk_off), true, 1),
        curve("loss alone: (1 − e^(−αL))/(αL)", line_of(&lossy), true, 2),
    ];
    let bottom = series
        .iter()
        .flat_map(|c| c.points.iter().map(|p| p[1]))
        .fold(0.0, f64::min)
        .max(-20.0);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "frequency f (GHz)".to_owned(),
        x_length: false,
        y_label: "response 20 log₁₀ |m(f)/m(1 GHz)| (dB)".to_owned(),
        y_log: false,
        y_range: Some([bottom.floor(), 1.0]),
        series,
        marker: f3,
        figures: vec![
            figure(
                "3 dB bandwidth",
                text(f3),
                4,
                "GHz",
                "the line's, from 1 GHz",
            ),
            figure(
                "6 dB bandwidth",
                text(f6),
                4,
                "GHz",
                "the line's, from 1 GHz",
            ),
            figure(
                "the walk-off's 3 dB",
                text(f3_walk),
                4,
                "GHz",
                "where U = πf|n_m − n_o|L/c reaches 1.389",
            ),
            figure(
                "the loss's 3 dB",
                text(f3_loss),
                4,
                "GHz",
                "where the line loses 6.39 dB over its length",
            ),
            figure(
                "from DC to 1 GHz",
                low,
                3,
                "dB",
                "the line's impedance against the 50 Ω ends, and its loss there",
            ),
        ],
        note: format!(
            "Exact, no grid: Ghione's Eq. (6.31) for a line of RF index {}, Z₀ = {} Ω and \
             {} √f + {} f dB/cm (f in GHz), {} mm long, between a 50 Ω source and load, light of \
             group index {GROUP_INDEX}. Zhang et al. 2022's fit to their measured line is the \
             default; the response is referred to 1 GHz, as they refer theirs.",
            sig(n_m, 4),
            sig(impedance, 3),
            sig(loss, 3),
            sig(dielectric, 3),
            sig(length_mm, 3)
        ),
    })
}

/// The index change of a 25 nm (or `thickness_nm`) film of each phase-change material on a
/// strip `width` µm wide, from amorphous to crystalline, against the film's thickness from 10
/// to 50 nm; the figures at `thickness_nm`.
pub fn pcm_shift(thickness_nm: f64, width: f64) -> photonoxide::Result<ChartData> {
    let cells = [
        ("Sb₂Se₃", PcmShifter::sb2se3()),
        ("Sb₂S₃", PcmShifter::sb2s3()),
        ("GST", PcmShifter::gst()),
    ];
    let thicknesses = span(10.0, 50.0, 17);
    let mut series = Vec::new();
    let mut figures = Vec::new();
    for (k, (name, device)) in cells.iter().enumerate() {
        let at = |t: f64| -> photonoxide::Result<Vec<photonoxide::device::Figure>> {
            let values = Values::new(device, &[("film_thickness", t * 1e-3), ("width", width)])?;
            device.derived(&values, Fidelity::Analytic)
        };
        let mut points = Vec::with_capacity(thicknesses.len());
        for &t in &thicknesses {
            points.push([t, read(&at(t)?, "dn_eff_ac")?.0.abs()]);
        }
        series.push(curve(name, points, false, k));
        let here = at(thickness_nm)?;
        let (dn, _) = read(&here, "dn_eff_ac")?;
        let (alpha, _) = read(&here, "alpha_c")?;
        let (l_pi, _) = read(&here, "L_pi")?;
        figures.push(figure(
            &format!("{name}: $\\Delta n_\\mathrm{{eff}}$"),
            dn,
            4,
            "",
            &format!("crystalline loss {} dB/µm", sig(alpha, 3)),
        ));
        let fom = read(&here, "FOM").map(|f| f.0).unwrap_or(f64::INFINITY);
        figures.push(figure(
            &format!("{name}: $L_\\pi$"),
            l_pi,
            4,
            "µm",
            &if fom.is_finite() {
                format!("figure of merit {} rad/dB", sig(fom, 3))
            } else {
                "the fit absorbs nothing: no figure of merit".to_owned()
            },
        ));
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "film thickness t_f (nm)".to_owned(),
        x_length: false,
        y_label: "Δn_eff, amorphous to crystalline".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: Some(thickness_nm),
        figures,
        note: format!(
            "Exact, no grid: the effective index method with exact complex slabs, a film over \
             the whole top of a {} µm × 220 nm silicon strip on oxide under oxide, TE-like, at \
             1.55 µm. The analytic Δn_eff is within 15 % of the 2D model's from 10 to 50 nm on \
             guides 0.4 to 0.6 µm wide.",
            sig(width, 3)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::portable;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap_or(f64::INFINITY)
    }

    /// The curves are the device's analytic figures at every point, and at Jacques et al.'s
    /// designs they are what `thermo_jacques` prints: 29.851 mW and 11.15 µs (TiN), 27.607 mW
    /// and 6.06 µs (N++). Pπ and τ rise as the heater moves away from the guide.
    #[test]
    fn the_heaters_are_the_devices() {
        for (kind, device, key, p_pi, tau) in [
            (1.0, ThermoShifter::metal(), "height", 29.851, 11.15),
            (2.0, ThermoShifter::doped(), "buffer", 27.607, 6.06),
        ] {
            let data = heater_power(kind, 0.5).unwrap();
            assert!((fig(&data, "$P_\\pi$ at") - p_pi).abs() < 1e-3, "{key}");
            assert!((fig(&data, "$\\tau$") - tau).abs() < 5e-3, "{key}");
            let power = &data.series[0].points;
            for q in power.iter().step_by(5) {
                let values = Values::new(&device, &[(key, q[0]), ("width", 0.5)]).unwrap();
                let f = device.derived(&values, Fidelity::Analytic).unwrap();
                assert_eq!(q[1], read(&f, "P_pi").unwrap().0);
            }
            assert!(
                power.windows(2).all(|w| w[1][1] > w[0][1]),
                "{key}: Pπ rises"
            );
        }
    }

    /// The transmission is cos²(φ/2) with φ = πV|m|/Vπ + φ₀ (the component's), Vπ·L the 2D
    /// model's 2.372 V·cm at Zhang et al.'s design (`tfln_zhang` prints it); quadrature at
    /// φ₀ = π/2 transmits half; a frequency lowers |m| and widens the swing's Vπ.
    #[test]
    fn the_modulator_transmits_cos_squared() {
        let data = mzm_transfer(5.8, 0.5, 0.0).unwrap();
        let vpi_l = fig(&data, "$V_\\pi L$");
        assert!((vpi_l - 2.372).abs() < 1e-3, "{vpi_l}");
        let vpi = fig(&data, "$V_\\pi$ at DC");
        for q in &data.series[0].points {
            let phi = PI * q[0] / vpi + 0.5 * PI;
            assert!(
                (q[1] - portable::powi(portable::cos(phi / 2.0), 2)).abs() < 1e-9,
                "{q:?}"
            );
        }
        let at_zero = data.series[0].points[40];
        assert!(at_zero[0].abs() < 1e-12 && (at_zero[1] - 0.5).abs() < 1e-9);
        let ac = mzm_transfer(5.8, 0.5, 50.0).unwrap();
        let m = fig(&ac, "$\\lvert m(f)\\rvert$");
        assert!(m < 1.0 && m > 0.3, "{m}");
        assert_eq!(ac.series.len(), 2);
    }

    /// The limits are Ghione's closed forms, and `response` meets them: a matched lossless line
    /// falls by 3 dB at U = 1.3916, a matched line at the light's speed where it loses 6.39 dB.
    /// Zhang et al.'s fitted line, 41.5 Ω between 50 Ω ends, 0.04 above the light, falls by 3 dB
    /// at 128 GHz at 5.8 mm and 62 at 10.8 mm (docs/methods/travelling-wave.md).
    #[test]
    fn the_response_is_ghiones() {
        // walk-off alone through `response`: a matched lossless line
        let walk = modulator_response(0.2, 0.0, 0.0, 50.0, 10.0).unwrap();
        for (a, b) in walk.series[0].points.iter().zip(&walk.series[1].points) {
            assert!((a[1] - b[1]).abs() < 1e-6, "{a:?} {b:?}");
        }
        // −3 dB from 1 GHz, where |sin U/U| = 10^(−3/20): U = 1.389 (1.3916 at 1/√2, Ghione's 1.39)
        let f3 = fig(&walk, "the walk-off's 3 dB");
        let u = PI * f3 * 1e9 * 0.2 * 0.01 / SPEED_OF_LIGHT;
        let u1 = PI * 1e9 * 0.2 * 0.01 / SPEED_OF_LIGHT;
        let db = 20.0 * portable::log10((portable::sin(u) / u) / (portable::sin(u1) / u1));
        assert!(
            (db + 3.0).abs() < 1e-6 && (u - 1.389).abs() < 1e-3,
            "{u}: {db} dB"
        );
        // loss alone through `response`: a matched line at the light's speed
        let lossy = modulator_response(0.0, 0.6, 0.0, 50.0, 10.0).unwrap();
        for (a, b) in lossy.series[0].points.iter().zip(&lossy.series[2].points) {
            assert!((a[1] - b[1]).abs() < 1e-6, "{a:?} {b:?}");
        }
        // −3 dB from 1 GHz on (1 − e^(−αL))/(αL), αL = 0.6√f dB over 1 cm
        let f = fig(&lossy, "the loss's 3 dB");
        let m = |ghz: f64| {
            let al = 0.6 * ghz.sqrt() / DB_PER_NEPER;
            (1.0 - portable::exp(-al)) / al
        };
        let db = 20.0 * portable::log10(m(f) / m(1.0));
        assert!((db + 3.0).abs() < 1e-6, "{f} GHz: {db} dB");
        // Zhang et al.'s fitted line
        let short = fig(
            &modulator_response(0.04, 0.42, 0.025, 41.5, 5.8).unwrap(),
            "3 dB",
        );
        let long = fig(
            &modulator_response(0.04, 0.42, 0.025, 41.5, 10.8).unwrap(),
            "3 dB",
        );
        assert!((short - 128.0).abs() < 2.0, "{short}");
        assert!((long - 62.0).abs() < 2.0, "{long}");
    }

    /// The curves are the devices' analytic Δn_eff, and at 25 nm on the 500 nm strip they are
    /// what `pcm_delaney` prints: 0.0533 (Sb₂Se₃), 0.0296 (Sb₂S₃), 0.8020 (GST).
    #[test]
    fn the_cells_are_the_devices() {
        let data = pcm_shift(25.0, 0.5).unwrap();
        for (name, dn) in [("Sb₂Se₃", 0.05333), ("Sb₂S₃", 0.02957), ("GST", 0.8020)] {
            let got = fig(&data, &format!("{name}: $\\Delta"));
            assert!((got - dn).abs() < 1e-4, "{name}: {got}");
            let c = data.series.iter().find(|c| c.label == name).unwrap();
            assert!(
                c.points.windows(2).all(|w| w[1][1] > w[0][1]),
                "{name} rises"
            );
        }
    }
}

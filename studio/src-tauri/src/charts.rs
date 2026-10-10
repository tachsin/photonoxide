//! The Academy's interactive charts: each a named function of typed parameters (units, ranges,
//! defaults) that returns curves and figures computed by photonoxide's own validated code, the
//! circuit components' closed forms and the multilayer's transfer matrices. The window draws
//! them with a slider for each parameter and asks again as a slider moves; nothing of the physics
//! is computed in the window.
//!
//! A lesson places a chart with a block (academy/README.md):
//!
//! ```text
//! ::chart ring-spectrum{radius=10um, coupling=0.05, loss=3dB/cm}
//! ```

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::circuit::components::{
    AddDropRing, AllPassRing, Coupler, Dispersion, PhaseShifter, Waveguide, mzi_closed_form,
};
use photonoxide::circuit::{Circuit, Component, Netlist};
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::units::{Length, Wavelength};
use serde::Serialize;

/// A chart's parameter: what a slider sets.
#[derive(Serialize, Clone, Debug)]
pub struct Param {
    /// Its name in a chart block, e.g. `radius`.
    pub key: &'static str,
    /// Its name for people; may hold TeX between dollars.
    pub label: &'static str,
    /// Its unit as a block writes it: `um`, `nm`, `dB/cm`, or empty for a number.
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// The slider's step (on a linear slider); 1 for a whole number.
    pub step: f64,
    /// Whether the slider moves on a logarithmic scale.
    pub log: bool,
    /// What it is, in a sentence.
    pub about: &'static str,
}

/// A chart: its parameters and what it computes with.
#[derive(Serialize, Clone, Debug)]
pub struct ChartSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    /// The library functions it calls, for the reader who wants to look.
    pub computed_by: &'static str,
    pub params: Vec<Param>,
}

/// A curve: (x, y) points.
#[derive(Serialize, Clone, Debug)]
pub struct Curve {
    pub label: String,
    pub points: Vec<[f64; 2]>,
    pub dashed: bool,
}

/// A number the chart reports beside its curves.
#[derive(Serialize, Clone, Debug)]
pub struct Figure {
    /// Its name; may hold TeX between dollars.
    pub label: String,
    /// The value, for tests and copying; `None` when there is none (an infinite extinction).
    pub value: Option<f64>,
    /// The value as shown.
    pub text: String,
    pub unit: String,
    /// A remark: the regime, or where it comes from.
    pub note: String,
}

/// What a chart draws.
#[derive(Serialize, Clone, Debug)]
pub struct ChartData {
    pub x_label: String,
    /// Whether x is a length in µm, which the window shows in the unit chosen app-wide.
    pub x_length: bool,
    pub y_label: String,
    pub y_range: Option<[f64; 2]>,
    pub series: Vec<Curve>,
    /// An x to mark with a line: a resonance, the Bragg wavelength, critical coupling.
    pub marker: Option<f64>,
    pub figures: Vec<Figure>,
    /// What the curves are, in a sentence: the grid, or "exact".
    pub note: String,
}

/// A parameter, its range and default as `(min, max, default)`.
const fn param(
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    (min, max, default): (f64, f64, f64),
    step: f64,
    log: bool,
    about: &'static str,
) -> Param {
    Param {
        key,
        label,
        unit,
        min,
        max,
        default,
        step,
        log,
        about,
    }
}

/// The silicon wire the ring charts take, as the ring examples do: n_eff 2.4 at 1.55 µm.
const RING_INDEX: f64 = 2.4;
/// The wavelength the ring charts centre on, µm.
const RING_CENTRE: f64 = 1.55;

/// Every chart, by id.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "ring-spectrum",
            title: "A ring's through and drop spectra",
            about: "An all-pass ring (drop coupling 0) or an add-drop ring: the power at its through and drop ports against wavelength, around 1.55 µm, with the Lorentzian of the resonance nearest it.",
            computed_by: "circuit::components::AllPassRing and AddDropRing: s_matrix, resonance, fsr, fwhm, q_factor, finesse, extremes (Bogaerts et al. 2011, Eqs. 1-23)",
            params: vec![
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (2.0, 200.0, 10.0),
                    0.1,
                    true,
                    "The ring's radius: its round trip is 2πR.",
                ),
                param(
                    "coupling",
                    "Input coupling $\\kappa_1^2$",
                    "",
                    (1e-4, 0.5, 0.01),
                    1e-4,
                    true,
                    "The share of the power the input bus's coupler crosses over.",
                ),
                param(
                    "drop",
                    "Drop coupling $\\kappa_2^2$",
                    "",
                    (0.0, 0.5, 0.0),
                    0.001,
                    false,
                    "The drop bus's coupler; 0 leaves an all-pass ring.",
                ),
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.0, 50.0, 3.0),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
                param(
                    "group_index",
                    "Group index $n_g$",
                    "",
                    (1.5, 5.0, 4.2),
                    0.01,
                    false,
                    "The waveguide's group index, which sets the free spectral range.",
                ),
                param(
                    "window",
                    "Window",
                    "nm",
                    (0.5, 100.0, 20.0),
                    0.1,
                    true,
                    "The span of wavelengths shown, centred on 1.55 µm.",
                ),
            ],
        },
        ChartSpec {
            id: "ring-coupling",
            title: "On resonance, against the coupling",
            about: "A ring's through and drop powers on resonance, and its loaded Q as a share of its intrinsic Q, as the input coupling goes from nothing through critical coupling to strong over-coupling.",
            computed_by: "circuit::components::AllPassRing and AddDropRing: extremes, q_factor (Bogaerts et al. 2011, Eqs. 11-22)",
            params: vec![
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (2.0, 200.0, 10.0),
                    0.1,
                    true,
                    "The ring's radius.",
                ),
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.1, 50.0, 3.0),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
                param(
                    "drop",
                    "Drop coupling $\\kappa_2^2$",
                    "",
                    (0.0, 0.5, 0.0),
                    0.001,
                    false,
                    "The drop bus's coupler; 0 leaves an all-pass ring.",
                ),
            ],
        },
        ChartSpec {
            id: "bragg-reflectance",
            title: "A Bragg mirror's reflectance",
            about: "N pairs of quarter-wave layers, high index first, between a cover and a substrate, at normal incidence: the share of the power reflected against wavelength.",
            computed_by: "mode::multilayer::Multilayer::reflection (Chilwell and Hodgkinson 1984, Eqs. 10-16)",
            params: vec![
                param(
                    "high",
                    "High index $n_H$",
                    "",
                    (1.0, 4.0, 2.3),
                    0.01,
                    false,
                    "The high-index layers' refractive index.",
                ),
                param(
                    "low",
                    "Low index $n_L$",
                    "",
                    (1.0, 4.0, 1.38),
                    0.01,
                    false,
                    "The low-index layers' refractive index.",
                ),
                param(
                    "pairs",
                    "Pairs $N$",
                    "",
                    (1.0, 60.0, 8.0),
                    1.0,
                    false,
                    "How many high-low pairs.",
                ),
                param(
                    "period",
                    "Period $\\Lambda$",
                    "nm",
                    (50.0, 1000.0, 160.0),
                    0.5,
                    false,
                    "One pair's thickness, split so that each layer is a quarter wave at the Bragg wavelength.",
                ),
                param(
                    "cover",
                    "Cover $n_0$",
                    "",
                    (1.0, 4.0, 1.0),
                    0.01,
                    false,
                    "The medium the light comes from.",
                ),
                param(
                    "substrate",
                    "Substrate $n_s$",
                    "",
                    (1.0, 4.0, 1.52),
                    0.01,
                    false,
                    "The medium beyond the stack.",
                ),
                param(
                    "from",
                    "From",
                    "nm",
                    (200.0, 3000.0, 350.0),
                    1.0,
                    false,
                    "The shortest wavelength shown.",
                ),
                param(
                    "to",
                    "To",
                    "nm",
                    (200.0, 3000.0, 950.0),
                    1.0,
                    false,
                    "The longest wavelength shown.",
                ),
            ],
        },
        ChartSpec {
            id: "bragg-bandwidth",
            title: "The stop band's width against the contrast",
            about: "The width of an endless quarter-wave stack's stop band against its index contrast: its edges found from one period's transmission by transfer matrices (Bloch's theorem), beside the closed form derived in the lesson and the coupled-mode estimate.",
            computed_by: "mode::multilayer::Multilayer::reflection (Chilwell and Hodgkinson 1984): one period's transmission t, and cos KΛ = Re(1/t)",
            params: vec![
                param(
                    "low",
                    "Low index $n_L$",
                    "",
                    (1.0, 3.5, 1.45),
                    0.01,
                    false,
                    "The low-index layers' refractive index.",
                ),
                param(
                    "up_to",
                    "Largest contrast",
                    "",
                    (0.05, 2.5, 2.0),
                    0.01,
                    false,
                    "The largest n_H − n_L shown.",
                ),
            ],
        },
        ChartSpec {
            id: "mzi-spectrum",
            title: "A Mach–Zehnder interferometer's two outputs",
            about: "Two ideal couplers joined by two arms of a silicon wire, the upper longer by ΔL and with a phase shifter on it, solved as a circuit: the power at the bar and cross outputs against wavelength, around 1.55 µm.",
            computed_by: "circuit::Netlist of components::Coupler, Waveguide and PhaseShifter, solved by Circuit::spectrum; the extremes by components::mzi_closed_form",
            params: vec![
                param(
                    "delta",
                    "Path difference $\\Delta L$",
                    "um",
                    (5.0, 2000.0, 50.0),
                    0.01,
                    true,
                    "How much longer the upper arm is than the lower (100 µm).",
                ),
                param(
                    "split",
                    "Splitter $\\kappa_1^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.005,
                    false,
                    "The share of the power the first coupler crosses over; 0.5 splits it evenly.",
                ),
                param(
                    "combine",
                    "Combiner $\\kappa_2^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.005,
                    false,
                    "The share of the power the second coupler crosses over.",
                ),
                param(
                    "loss",
                    "Arm loss $\\alpha$",
                    "dB/cm",
                    (0.0, 100.0, 3.0),
                    0.1,
                    false,
                    "Both arms' propagation loss: the longer arm loses more.",
                ),
                param(
                    "phase",
                    "Phase shifter $\\varphi / \\pi$",
                    "",
                    (-1.0, 1.0, 0.0),
                    0.01,
                    false,
                    "The phase the shifter on the upper arm adds, in units of π: a heater's or a modulator's.",
                ),
                param(
                    "group_index",
                    "Group index $n_g$",
                    "",
                    (1.5, 5.0, 4.2),
                    0.01,
                    false,
                    "The wire's group index, which sets the free spectral range.",
                ),
                param(
                    "window",
                    "Window",
                    "nm",
                    (0.5, 200.0, 40.0),
                    0.1,
                    true,
                    "The span of wavelengths shown, centred on 1.55 µm.",
                ),
            ],
        },
        ChartSpec {
            id: "mzi-extinction",
            title: "The extinction against the splitter",
            about: "How deep the fringes of each output are, the brightest over the darkest as the arms' phase difference turns, as the first coupler goes from keeping all the light to crossing all of it over.",
            computed_by: "circuit::components::mzi_closed_form, with Coupler's and Waveguide's S-matrices",
            params: vec![
                param(
                    "combine",
                    "Combiner $\\kappa_2^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.005,
                    false,
                    "The share of the power the second coupler crosses over.",
                ),
                param(
                    "loss",
                    "Arm loss $\\alpha$",
                    "dB/cm",
                    (0.0, 100.0, 3.0),
                    0.1,
                    false,
                    "Both arms' propagation loss.",
                ),
                param(
                    "delta",
                    "Path difference $\\Delta L$",
                    "um",
                    (5.0, 2000.0, 50.0),
                    0.01,
                    true,
                    "How much longer, and so how much lossier, the upper arm is.",
                ),
            ],
        },
    ]
}

/// The chart `id`'s spec.
pub fn spec(id: &str) -> Option<ChartSpec> {
    specs().into_iter().find(|s| s.id == id)
}

/// Every parameter's value: those given, checked against the spec, and the defaults for the
/// rest.
pub fn resolve(
    spec: &ChartSpec,
    given: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, f64>, String> {
    for key in given.keys() {
        if !spec.params.iter().any(|p| p.key == key) {
            return Err(format!("{}: no parameter {key}", spec.id));
        }
    }
    let mut values = BTreeMap::new();
    for p in &spec.params {
        let v = given.get(p.key).copied().unwrap_or(p.default);
        if !(v.is_finite() && v >= p.min && v <= p.max) {
            return Err(format!(
                "{}: {} must be from {} to {}{}, got {v}",
                spec.id,
                p.key,
                p.min,
                p.max,
                if p.unit.is_empty() {
                    String::new()
                } else {
                    format!(" {}", p.unit)
                }
            ));
        }
        if p.step == 1.0 && v.fract() != 0.0 {
            return Err(format!(
                "{}: {} must be a whole number, got {v}",
                spec.id, p.key
            ));
        }
        values.insert(p.key.to_owned(), v);
    }
    Ok(values)
}

/// A block's value, `10um` or `0.2` or `3dB/cm`, in the parameter's unit: a length may be
/// written in nm, µm or mm whatever unit the parameter keeps it in; any other unit must be the
/// parameter's own, or left out.
pub fn parse_value(param: &Param, text: &str) -> Result<f64, String> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(text.len());
    // an exponent's e is a digit's; a unit's first letter isn't (no unit starts with e)
    let (number, unit) = text.split_at(split);
    let value: f64 = number
        .parse()
        .map_err(|_| format!("{}: {text:?} isn't a number", param.key))?;
    let unit = unit.trim();
    let length = |u: &str| match u {
        "nm" => Some(1e-3),
        "um" | "µm" => Some(1.0),
        "mm" => Some(1e3),
        _ => None,
    };
    if unit.is_empty() || unit == param.unit {
        return Ok(value);
    }
    match (length(unit), length(param.unit)) {
        (Some(from), Some(to)) => Ok(value * from / to),
        _ => Err(format!(
            "{}: {text:?} is in {unit}, the parameter is in {}",
            param.key,
            if param.unit.is_empty() {
                "no unit"
            } else {
                param.unit
            }
        )),
    }
}

/// A chart block's `{key=value, ...}`, each value in its parameter's unit.
pub fn parse_values(spec: &ChartSpec, text: &str) -> Result<BTreeMap<String, f64>, String> {
    let mut out = BTreeMap::new();
    for pair in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("{}: {pair:?} isn't key=value", spec.id))?;
        let key = key.trim();
        let p = spec
            .params
            .iter()
            .find(|p| p.key == key)
            .ok_or_else(|| format!("{}: no parameter {key}", spec.id))?;
        if out.insert(key.to_owned(), parse_value(p, value)?).is_some() {
            return Err(format!("{}: {key} given twice", spec.id));
        }
    }
    resolve(spec, &out)?;
    Ok(out)
}

/// The chart `id` at `given` (missing parameters at their defaults).
pub fn evaluate(id: &str, given: &BTreeMap<String, f64>) -> Result<ChartData, String> {
    let spec = spec(id).ok_or_else(|| format!("no chart {id}"))?;
    let v = resolve(&spec, given)?;
    let at = |k: &str| v[k];
    let result = match id {
        "ring-spectrum" => ring_spectrum(
            at("radius"),
            at("coupling"),
            at("drop"),
            at("loss"),
            at("group_index"),
            at("window"),
        ),
        "ring-coupling" => ring_coupling(at("radius"), at("loss"), at("drop")),
        "bragg-reflectance" => bragg_reflectance(
            &Stack {
                high: at("high"),
                low: at("low"),
                pairs: at("pairs") as usize,
                period_nm: at("period"),
                cover: at("cover"),
                substrate: at("substrate"),
            },
            (at("from"), at("to")),
        ),
        "bragg-bandwidth" => bragg_bandwidth(at("low"), at("up_to")),
        "mzi-spectrum" => mzi_spectrum(
            &Mzi {
                delta: at("delta"),
                split: at("split"),
                combine: at("combine"),
                loss: at("loss"),
                phase: at("phase"),
                group_index: at("group_index"),
            },
            at("window"),
        ),
        "mzi-extinction" => mzi_extinction(at("combine"), at("loss"), at("delta")),
        _ => return Err(format!("chart {id} has no function")),
    };
    result.map_err(|e| format!("{id}: {e}"))
}

/// `x` to `digits` significant digits, as people write it: 1.234e5 as 1.234×10⁵.
pub fn sig(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return "∞".to_owned();
    }
    if x == 0.0 {
        return "0".to_owned();
    }
    let e = x.abs().log10().floor() as i32;
    if (-2..4).contains(&e) {
        let decimals = (digits as i32 - 1 - e).max(0) as usize;
        format!("{x:.decimals$}")
    } else {
        let mantissa = x / 10f64.powi(e);
        let sup: String = e
            .to_string()
            .chars()
            .map(|c| match c {
                '-' => '⁻',
                d => ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹']
                    [d.to_digit(10).unwrap_or(0) as usize],
            })
            .collect();
        format!("{:.*}×10{sup}", digits.saturating_sub(1), mantissa)
    }
}

fn figure(label: &str, value: f64, digits: usize, unit: &str, note: &str) -> Figure {
    Figure {
        label: label.to_owned(),
        value: value.is_finite().then_some(value),
        text: sig(value, digits),
        unit: unit.to_owned(),
        note: note.to_owned(),
    }
}

fn invalid(what: &'static str, reason: impl Into<String>) -> photonoxide::Error {
    photonoxide::Error::InvalidValue {
        what,
        reason: reason.into(),
    }
}

/// The ring charts' ring: all-pass when `drop` is 0, add-drop otherwise.
enum Ring {
    AllPass(AllPassRing),
    AddDrop(AddDropRing),
}

impl Ring {
    fn new(loss: f64, group_index: f64, drop: f64) -> photonoxide::Result<Ring> {
        let guide =
            Dispersion::new(Wavelength::um(RING_CENTRE)?, RING_INDEX, group_index).with_loss(loss);
        Ok(if drop > 0.0 {
            Ring::AddDrop(AddDropRing::new(guide)?)
        } else {
            Ring::AllPass(AllPassRing::new(guide)?)
        })
    }

    fn guide(&self) -> &Dispersion {
        match self {
            Ring::AllPass(r) => r.guide(),
            Ring::AddDrop(r) => r.guide(),
        }
    }

    /// The component's values: `[length, coupling]` or `[length, coupling, coupling_drop]`.
    fn values(&self, length: f64, coupling: f64, drop: f64) -> Vec<f64> {
        match self {
            Ring::AllPass(_) => vec![length, coupling],
            Ring::AddDrop(_) => vec![length, coupling, drop],
        }
    }

    /// Through and drop power at λ (the drop's 0 for an all-pass ring).
    fn powers(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<(f64, f64)> {
        Ok(match self {
            Ring::AllPass(r) => (r.s_matrix(w, values)?.power(1, 0), 0.0),
            Ring::AddDrop(r) => {
                let s = r.s_matrix(w, values)?;
                (s.power(1, 0), s.power(3, 0))
            }
        })
    }

    fn resonance(&self, near: Wavelength, length: f64) -> photonoxide::Result<Wavelength> {
        match self {
            Ring::AllPass(r) => r.resonance(near, length),
            Ring::AddDrop(r) => r.resonance(near, length),
        }
    }

    fn fsr(&self, w: Wavelength, length: f64) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.fsr(w, length),
            Ring::AddDrop(r) => r.fsr(w, length),
        }
    }

    fn fwhm(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.fwhm(w, values),
            Ring::AddDrop(r) => r.fwhm(w, values),
        }
    }

    fn q_factor(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.q_factor(w, values),
            Ring::AddDrop(r) => r.q_factor(w, values),
        }
    }

    fn finesse(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.finesse(w, values),
            Ring::AddDrop(r) => r.finesse(w, values),
        }
    }

    /// Through power off and on resonance, and drop power on resonance (0 for all-pass).
    fn extremes(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<(f64, f64, f64)> {
        Ok(match self {
            Ring::AllPass(r) => {
                let (off, on) = r.extremes(w, values)?;
                (off, on, 0.0)
            }
            Ring::AddDrop(r) => {
                let [off, on, dropped, _] = r.extremes(w, values)?;
                (off, on, dropped)
            }
        })
    }

    /// The single-pass amplitude a over a round trip of `length`: e^(−Im γ L), from the guide's
    /// propagation constant.
    fn single_pass(&self, w: Wavelength, length: f64) -> f64 {
        (-self.guide().propagation(w).im * length).exp()
    }
}

/// The coupling κ₁² at which the through port goes dark on resonance: r₁ = r₂a (Bogaerts Eqs.
/// 12 and 14), so κ₁² = 1 − (1 − κ₂²)a².
fn critical(a: f64, drop: f64) -> f64 {
    1.0 - (1.0 - drop) * a * a
}

fn regime(coupling: f64, critical: f64) -> &'static str {
    if (coupling - critical).abs() <= 0.02 * critical {
        "critically coupled: the through port is dark on resonance"
    } else if coupling < critical {
        "under-coupled: the ring loses more per round trip than it couples out"
    } else {
        "over-coupled: the ring couples out more per round trip than it loses"
    }
}

fn ring_spectrum(
    radius: f64,
    coupling: f64,
    drop: f64,
    loss: f64,
    group_index: f64,
    window_nm: f64,
) -> photonoxide::Result<ChartData> {
    let ring = Ring::new(loss, group_index, drop)?;
    let length = TAU * radius;
    let values = ring.values(length, coupling, drop);
    let centre = Wavelength::um(RING_CENTRE)?;
    let half = window_nm * 1e-3 / 2.0;
    let (lo, hi) = (RING_CENTRE - half, RING_CENTRE + half);
    let nearest = ring.resonance(centre, length)?;
    let fsr = ring.fsr(nearest, length)?;
    let fwhm = ring.fwhm(nearest, &values)?;

    // the wavelengths: even across the window, and dense across each resonance's line
    let mut xs: Vec<f64> = (0..=800)
        .map(|k| lo + (hi - lo) * f64::from(k) / 800.0)
        .collect();
    let count = (hi - lo) / fsr;
    if count < 400.0 {
        // points either side of each line's centre: fewer when there are many lines
        let per = ((3000.0 / count) as i32).clamp(10, 40);
        let first = ((lo - nearest.to_um()) / fsr).floor() as i64 - 1;
        let last = ((hi - nearest.to_um()) / fsr).ceil() as i64 + 1;
        for m in first..=last {
            let guess = nearest.to_um() + m as f64 * fsr;
            let Ok(near) = Wavelength::um(guess) else {
                continue;
            };
            let Ok(res) = ring.resonance(near, length) else {
                continue;
            };
            let width = ring.fwhm(res, &values).unwrap_or(fwhm);
            for k in -per..=per {
                // tangent spacing: fine at the centre of the line, coarse in its wings
                let t = (f64::from(k) / f64::from(per) * 1.45).tan() * 4.0;
                xs.push(res.to_um() + t * width);
            }
        }
    }
    xs.retain(|x| (lo..=hi).contains(x));
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-13);

    let mut through = Vec::with_capacity(xs.len());
    let mut dropped = Vec::with_capacity(xs.len());
    for &x in &xs {
        let (t, d) = ring.powers(Wavelength::um(x)?, &values)?;
        through.push([x, t]);
        dropped.push([x, d]);
    }
    let (off, on, peak) = ring.extremes(nearest, &values)?;
    let mut series = vec![Curve {
        label: "through".to_owned(),
        points: through,
        dashed: false,
    }];
    if drop > 0.0 {
        series.push(Curve {
            label: "drop".to_owned(),
            points: dropped,
            dashed: false,
        });
    }
    // the Lorentzian the line is close to, by its width from Eq. 7 (or 8)
    let reach = (6.0 * fwhm).min(fsr / 2.0);
    let lorentzian: Vec<[f64; 2]> = (-120..=120)
        .map(|k| nearest.to_um() + reach * f64::from(k) / 120.0)
        .filter(|x| (lo..=hi).contains(x))
        .map(|x| {
            let u = 2.0 * (x - nearest.to_um()) / fwhm;
            [x, off - (off - on) / (1.0 + u * u)]
        })
        .collect();
    if lorentzian.len() > 2 {
        series.push(Curve {
            label: "Lorentzian of the FWHM formula".to_owned(),
            points: lorentzian,
            dashed: true,
        });
    }

    let a = ring.single_pass(nearest, length);
    let crit = critical(a, drop);
    let order = (ring.guide().effective_index_at(nearest) * length / nearest.to_um()).round();
    let mut figures = vec![
        figure("Resonance nearest 1.55 µm", nearest.to_um(), 7, "µm", ""),
        Figure {
            label: "Order $m$".to_owned(),
            value: Some(order),
            text: format!("{order}"),
            unit: String::new(),
            note: "$n_\\text{eff} L = m\\lambda$".to_owned(),
        },
        figure(
            "Free spectral range",
            fsr * 1e3,
            4,
            "nm",
            "$\\lambda^2 / (n_g L)$",
        ),
        figure("Linewidth (FWHM)", fwhm * 1e6, 4, "pm", ""),
        figure(
            "Loaded $Q$",
            ring.q_factor(nearest, &values)?,
            4,
            "",
            "$\\lambda / \\text{FWHM}$",
        ),
        figure(
            "Finesse",
            ring.finesse(nearest, &values)?,
            4,
            "",
            "FSR / FWHM",
        ),
        figure("Single-pass amplitude $a$", a, 6, "", ""),
        figure(
            "Critical coupling $\\kappa_1^2$",
            crit,
            4,
            "",
            regime(coupling, crit),
        ),
    ];
    let extinction = 10.0 * (off / on).log10();
    figures.push(Figure {
        label: "Through extinction".to_owned(),
        value: extinction.is_finite().then_some(extinction),
        text: if on <= 0.0 || extinction > 99.0 {
            "> 99".to_owned()
        } else {
            sig(extinction, 3)
        },
        unit: "dB".to_owned(),
        note: "off resonance over on resonance".to_owned(),
    });
    if drop > 0.0 {
        figures.push(figure(
            "Drop on resonance",
            peak,
            4,
            "",
            "share of the input power",
        ));
    }
    Ok(ChartData {
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_range: Some([-0.02, 1.02]),
        series,
        marker: Some(nearest.to_um()),
        figures,
        note: format!(
            "Closed forms, exact: no grid. A silicon wire, n_eff {RING_INDEX} at {RING_CENTRE} µm, n_g {group_index}, first order in wavelength."
        ),
    })
}

fn ring_coupling(radius: f64, loss: f64, drop: f64) -> photonoxide::Result<ChartData> {
    let ring = Ring::new(loss, 4.2, drop)?;
    let length = TAU * radius;
    let w = Wavelength::um(RING_CENTRE)?;
    let a = ring.single_pass(w, length);
    let crit = critical(a, drop);
    // the intrinsic Q: the ring alone, nothing coupled out
    let intrinsic = AllPassRing::new(*ring.guide())?.q_factor(w, &[length, 0.0])?;
    let upto = (5.0 * crit).clamp(0.01, 0.99);
    let mut on = Vec::new();
    let mut peak = Vec::new();
    let mut q = Vec::new();
    for k in 0..=400 {
        let kappa2 = upto * f64::from(k) / 400.0;
        let values = ring.values(length, kappa2, drop);
        let (_, t, d) = ring.extremes(w, &values)?;
        on.push([kappa2, t]);
        peak.push([kappa2, d]);
        q.push([kappa2, ring.q_factor(w, &values)? / intrinsic]);
    }
    let mut series = vec![Curve {
        label: "through, on resonance".to_owned(),
        points: on,
        dashed: false,
    }];
    if drop > 0.0 {
        series.push(Curve {
            label: "drop, on resonance".to_owned(),
            points: peak,
            dashed: false,
        });
    }
    series.push(Curve {
        label: "loaded Q / intrinsic Q".to_owned(),
        points: q,
        dashed: true,
    });
    let at_critical = ring.q_factor(w, &ring.values(length, crit, drop))?;
    Ok(ChartData {
        x_label: "input coupling κ₁²".to_owned(),
        x_length: false,
        y_label: "share".to_owned(),
        y_range: Some([-0.02, 1.02]),
        series,
        marker: Some(crit),
        figures: vec![
            figure(
                "Single-pass amplitude $a$",
                a,
                6,
                "",
                "$e^{-\\alpha L / 2}$",
            ),
            figure(
                "Critical coupling $\\kappa_1^2$",
                crit,
                4,
                "",
                "$1 - (1 - \\kappa_2^2)a^2$",
            ),
            figure("Intrinsic $Q$", intrinsic, 4, "", "no coupling"),
            figure("Loaded $Q$ at critical coupling", at_critical, 4, "", ""),
        ],
        note: format!(
            "Closed forms, exact: no grid. A silicon wire, n_eff {RING_INDEX}, n_g 4.2 at {RING_CENTRE} µm."
        ),
    })
}

/// A quarter-wave stack: N pairs of high and low layers, high first.
struct Stack {
    high: f64,
    low: f64,
    pairs: usize,
    period_nm: f64,
    cover: f64,
    substrate: f64,
}

impl Stack {
    /// The Bragg wavelength, nm: each layer a quarter wave, n_H d_H = n_L d_L = λ_B/4.
    fn bragg_nm(&self) -> f64 {
        4.0 * self.period_nm * self.high * self.low / (self.high + self.low)
    }

    /// The layers' thicknesses, nm.
    fn thicknesses(&self) -> (f64, f64) {
        let q = self.bragg_nm() / 4.0;
        (q / self.high, q / self.low)
    }

    fn multilayer(&self) -> photonoxide::Result<Multilayer> {
        let (dh, dl) = self.thicknesses();
        let one = |n: f64| c64::new(n, 0.0);
        let mut films = Vec::with_capacity(2 * self.pairs);
        for _ in 0..self.pairs {
            films.push((one(self.high), Length::nm(dh)));
            films.push((one(self.low), Length::nm(dl)));
        }
        Multilayer::new(one(self.cover), &films, one(self.substrate))
    }
}

/// The reflectance of `stack` at λ (nm), normal incidence.
fn reflectance(stack: &Multilayer, nm: f64) -> photonoxide::Result<f64> {
    Ok(stack
        .reflection(Polarization::Te, Wavelength::nm(nm)?, 0.0)?
        .reflectance)
}

/// The infinite quarter-wave stack's stop band, as a share of its centre frequency:
/// (4/π) arcsin(|n_H − n_L|/(n_H + n_L)), derived in academy/bragg-gratings.md.
pub fn band_width(high: f64, low: f64) -> f64 {
    4.0 / PI * ((high - low).abs() / (high + low)).asin()
}

fn bragg_reflectance(s: &Stack, (from, to): (f64, f64)) -> photonoxide::Result<ChartData> {
    if to <= from {
        return Err(invalid(
            "the window",
            format!("it must run from a shorter to a longer wavelength, got {from} to {to} nm"),
        ));
    }
    let stack = s.multilayer()?;
    let mut points = Vec::with_capacity(1501);
    for k in 0..=1500 {
        let nm = from + (to - from) * f64::from(k) / 1500.0;
        points.push([nm * 1e-3, reflectance(&stack, nm)?]);
    }
    let bragg = s.bragg_nm();
    let (dh, dl) = s.thicknesses();
    let peak = reflectance(&stack, bragg)?;
    // the quarter-wave stack's closed form, from its admittance (the validation case's)
    let q = s.substrate / s.cover * (s.high / s.low).powi(2 * s.pairs as i32);
    let closed = ((1.0 - q) / (1.0 + q)).powi(2);
    // the endless stack's band, from one period's Bloch phase
    let width = bloch_band(s.high, s.low)?;
    let (short, long) = (bragg / (1.0 + width / 2.0), bragg / (1.0 - width / 2.0));
    Ok(ChartData {
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "reflectance R".to_owned(),
        y_range: Some([-0.02, 1.02]),
        series: vec![Curve {
            label: format!("R, {} pairs", s.pairs),
            points,
            dashed: false,
        }],
        marker: Some(bragg * 1e-3),
        figures: vec![
            figure(
                "Bragg wavelength $\\lambda_B$",
                bragg,
                5,
                "nm",
                "$4 n_H d_H = 4 n_L d_L$",
            ),
            figure("High layer $d_H$", dh, 4, "nm", ""),
            figure("Low layer $d_L$", dl, 4, "nm", ""),
            figure("$R$ at $\\lambda_B$, transfer matrices", peak, 6, "", ""),
            figure(
                "$R$ at $\\lambda_B$, closed form",
                closed,
                6,
                "",
                "$((1 - q)/(1 + q))^2$, $q = (n_s/n_0)(n_H/n_L)^{2N}$",
            ),
            figure(
                "Stop band, infinite stack",
                width * 100.0,
                4,
                "%",
                "of the centre frequency, where $\\cos K\\Lambda \\lt -1$",
            ),
            Figure {
                label: "Band edges, infinite stack".to_owned(),
                value: Some(short),
                text: format!("{} – {}", sig(short, 5), sig(long, 5)),
                unit: "nm".to_owned(),
                note: String::new(),
            },
        ],
        note: "Transfer matrices, exact: no grid. Normal incidence.".to_owned(),
    })
}

/// cos KΛ, the Bloch phase of an endless stack of `cell` repeated, at `nm`: one period's
/// transmission t between media of the cell's outer index, cos KΛ = Re(1/t), which holds for a
/// lossless, symmetric period (its transfer matrix's half trace, Bloch's theorem).
fn bloch_cos(cell: &Multilayer, nm: f64) -> photonoxide::Result<f64> {
    let t = cell
        .reflection(Polarization::Te, Wavelength::nm(nm)?, 0.0)?
        .t;
    Ok((1.0 / t).re)
}

/// The stop band of the endless quarter-wave stack of `high` and `low`, as a share of its
/// centre frequency: its edges, where cos KΛ = −1, found by bisection either side of the
/// centre, the Bloch phase from one period's transfer matrices.
pub fn bloch_band(high: f64, low: f64) -> photonoxide::Result<f64> {
    let bragg = 1000.0;
    let (dh, dl) = (bragg / (4.0 * high), bragg / (4.0 * low));
    let one = |n: f64| c64::new(n, 0.0);
    // the period made symmetric, half a low layer either side of the high one, in the low index
    let cell = Multilayer::new(
        one(low),
        &[
            (one(low), Length::nm(dl / 2.0)),
            (one(high), Length::nm(dh)),
            (one(low), Length::nm(dl / 2.0)),
        ],
        one(low),
    )?;
    // x is the frequency's offset from the centre, as a share of it: λ = λ_B/(1 + x); the band
    // is where cos KΛ < −1, around x = 0, and the quarter-wave stack's next band edge is at x = ±1
    let below =
        |x: f64| -> photonoxide::Result<bool> { Ok(bloch_cos(&cell, bragg / (1.0 + x))? < -1.0) };
    // equal indices: no band, cos KΛ = −1 at the centre, to round-off
    if bloch_cos(&cell, bragg)? > -1.0 - 1e-12 {
        return Ok(0.0);
    }
    let mut width = 0.0;
    for sign in [-1.0, 1.0] {
        let (mut inside, mut outside) = (0.0, sign * 0.999);
        for _ in 0..60 {
            let mid = (inside + outside) / 2.0;
            if below(mid)? {
                inside = mid;
            } else {
                outside = mid;
            }
        }
        width += ((inside + outside) / 2.0).abs();
    }
    Ok(width)
}

fn bragg_bandwidth(low: f64, up_to: f64) -> photonoxide::Result<ChartData> {
    let mut measured = Vec::new();
    let mut exact = Vec::new();
    let mut coupled = Vec::new();
    for k in 1..=100 {
        let dn = up_to * f64::from(k) / 100.0;
        let high = low + dn;
        measured.push([dn, 100.0 * bloch_band(high, low)?]);
        exact.push([dn, 100.0 * band_width(high, low)]);
        // coupled modes: κ = 2Δn/λ for a square grating, a band 2κ wide in β: Δω/ω = 2Δn/(π n̄)
        coupled.push([dn, 100.0 * 2.0 * dn / (PI * (high + low) / 2.0)]);
    }
    let last = |c: &[[f64; 2]]| c.last().map_or(f64::NAN, |p| p[1]);
    let figures = vec![
        figure(
            "Band at the largest contrast, transfer matrices",
            last(&measured),
            5,
            "%",
            "",
        ),
        figure(
            "The same, closed form",
            last(&exact),
            5,
            "%",
            r"$(4/\pi) \arcsin((n_H - n_L)/(n_H + n_L))$",
        ),
        figure(
            "The same, coupled modes",
            last(&coupled),
            5,
            "%",
            r"$2 \Delta n/(\pi \bar n)$",
        ),
    ];
    Ok(ChartData {
        x_label: "index contrast Δn = nH − nL".to_owned(),
        x_length: false,
        y_label: "stop band (% of centre frequency)".to_owned(),
        y_range: None,
        series: vec![
            Curve {
                label: "band edges, transfer matrices: cos KΛ = −1".to_owned(),
                points: measured,
                dashed: false,
            },
            Curve {
                label: "closed form: (4/π) arcsin(Δn/(nH + nL))".to_owned(),
                points: exact,
                dashed: true,
            },
            Curve {
                label: "coupled modes: 2Δn/(π n̄)".to_owned(),
                points: coupled,
                dashed: true,
            },
        ],
        marker: None,
        figures,
        note: "Transfer matrices, exact: no grid. An endless stack of quarter-wave layers at normal incidence.".to_owned(),
    })
}

/// The lower arm's length in the MZI charts, µm; the upper is longer by ΔL.
const MZI_ARM: f64 = 100.0;
/// The largest extinction the extinction chart draws, dB: a balanced interferometer's is
/// unbounded.
const MZI_CAP: f64 = 60.0;

/// The MZI charts' interferometer: path difference µm, the couplers' κ², the arms' loss dB/cm,
/// the phase shifter's phase over π, and the wire's group index.
struct Mzi {
    delta: f64,
    split: f64,
    combine: f64,
    loss: f64,
    phase: f64,
    group_index: f64,
}

impl Mzi {
    /// The arms' wire: n_eff 2.4 at 1.55 µm, as the ring charts', first order in λ.
    fn guide(&self) -> photonoxide::Result<Dispersion> {
        Ok(
            Dispersion::new(Wavelength::um(RING_CENTRE)?, RING_INDEX, self.group_index)
                .with_loss(self.loss),
        )
    }

    /// The interferometer as a netlist, solved by the circuit solve: two ideal couplers, the
    /// upper arm (`MZI_ARM` + ΔL µm) and its phase shifter, the lower arm (`MZI_ARM` µm), as
    /// [`photonoxide::circuit::components::mzi`] joins them. Ports `o1` (the input, the lower
    /// guide), `o2`, `o3` (cross, the upper guide) and `o4` (bar, the lower guide).
    fn circuit(&self) -> photonoxide::Result<Circuit> {
        let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
        let wire: Arc<dyn Component> = Arc::new(Waveguide::new(self.guide()?));
        let mut n = Netlist::new();
        n.add("splitter", coupler.clone())?;
        n.add("combiner", coupler)?;
        n.add("upper", wire.clone())?;
        n.add("shifter", Arc::new(PhaseShifter::new()))?;
        n.add("lower", wire)?;
        n.connect("splitter.o3", "upper.o1")?;
        n.connect("upper.o2", "shifter.o1")?;
        n.connect("shifter.o2", "combiner.o2")?;
        n.connect("splitter.o4", "lower.o1")?;
        n.connect("lower.o2", "combiner.o1")?;
        for (port, at) in [
            ("o1", "splitter.o1"),
            ("o2", "splitter.o2"),
            ("o3", "combiner.o3"),
            ("o4", "combiner.o4"),
        ] {
            n.expose(port, at)?;
        }
        n.set("splitter", "coupling", self.split)?;
        n.set("combiner", "coupling", self.combine)?;
        n.set("upper", "length", MZI_ARM + self.delta)?;
        n.set("lower", "length", MZI_ARM)?;
        n.set("shifter", "phase", self.phase * PI)?;
        n.compile()
    }

    /// The upper and lower arms' field amplitudes, |e^(iγL)|, from the waveguide's S-matrix.
    fn arms(&self) -> photonoxide::Result<(f64, f64)> {
        let wire = Waveguide::new(self.guide()?);
        let w = Wavelength::um(RING_CENTRE)?;
        let loss = self.loss;
        let amplitude = |length: f64| -> photonoxide::Result<f64> {
            Ok(wire.s_matrix(w, &[length, loss])?[(1, 0)].norm())
        };
        Ok((amplitude(MZI_ARM + self.delta)?, amplitude(MZI_ARM)?))
    }
}

/// A coupler's through and across fields at κ², from [`Coupler`]'s S-matrix.
fn coupler_fields(kappa2: f64) -> photonoxide::Result<(c64, c64)> {
    let s = Coupler::new().s_matrix(Wavelength::um(RING_CENTRE)?, &[kappa2])?;
    Ok((s[(3, 0)], s[(2, 0)]))
}

/// The bar and cross outputs' brightest and darkest powers as the arms' phase difference turns,
/// `[[bar max, bar min], [cross max, cross min]]`, light entering the lower guide: the closed form
/// with the arms' fields in phase and then opposite (each output adds two paths, and its
/// extremes are where they line up), for couplers `split` and `combine` and arms of amplitude
/// `upper` and `lower`.
fn mzi_extremes(
    split: f64,
    combine: f64,
    upper: f64,
    lower: f64,
) -> photonoxide::Result<[[f64; 2]; 2]> {
    let (s, c) = (coupler_fields(split)?, coupler_fields(combine)?);
    let lower = c64::new(lower, 0.0);
    let one = mzi_closed_form(s, c, c64::new(upper, 0.0), lower);
    let other = mzi_closed_form(s, c, c64::new(-upper, 0.0), lower);
    let pair = |q: usize| {
        let (a, b) = (one[q][0].norm_sqr(), other[q][0].norm_sqr());
        [a.max(b), a.min(b)]
    };
    Ok([pair(0), pair(1)])
}

/// The extinction of `[max, min]`, dB, as a figure: unbounded when the darkest is dark.
fn extinction_figure(label: &str, [max, min]: [f64; 2]) -> Figure {
    let db = 10.0 * (max / min).log10();
    Figure {
        label: label.to_owned(),
        value: db.is_finite().then_some(db),
        text: if max <= 0.0 {
            "–".to_owned()
        } else if min <= 0.0 || db > 99.0 {
            "> 99".to_owned()
        } else {
            sig(db, 3)
        },
        unit: "dB".to_owned(),
        note: "brightest over darkest".to_owned(),
    }
}

/// The cross port's power at `l` µm.
fn cross(c: &Circuit, l: f64) -> photonoxide::Result<f64> {
    Ok(c.s_matrix(Wavelength::um(l)?)?.power(2, 0))
}

/// The cross port's peaks either side of 1.55 µm, λ₁ < 1.55 ≤ λ₂, found on the circuit's
/// spectrum: sampled over ±1.6 free spectral ranges (`fsr`, µm), each refined by golden section
/// to 1e-12 µm, as the `mzi_dwivedi` example reads a spectrum.
fn neighbouring_peaks(c: &Circuit, fsr: f64) -> photonoxide::Result<Option<(f64, f64)>> {
    let (from, to) = (RING_CENTRE - 1.6 * fsr, RING_CENTRE + 1.6 * fsr);
    let n = 480;
    let x = |i: usize| from + (to - from) * i as f64 / n as f64;
    let t: Vec<f64> = (0..=n)
        .map(|i| cross(c, x(i)))
        .collect::<photonoxide::Result<_>>()?;
    let spread = t.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - t.iter().copied().fold(f64::INFINITY, f64::min);
    if spread < 1e-9 {
        return Ok(None);
    }
    let mut peaks = Vec::new();
    for i in 1..n {
        if t[i] > t[i - 1] && t[i] >= t[i + 1] {
            let (mut lo, mut hi) = (x(i - 1), x(i + 1));
            let g = (5f64.sqrt() - 1.0) / 2.0;
            while hi - lo > 1e-12 {
                let (a, b) = (hi - g * (hi - lo), lo + g * (hi - lo));
                if cross(c, a)? > cross(c, b)? {
                    hi = b;
                } else {
                    lo = a;
                }
            }
            peaks.push((lo + hi) / 2.0);
        }
    }
    let below = peaks.iter().copied().rfind(|&p| p < RING_CENTRE);
    let above = peaks.iter().copied().find(|&p| p >= RING_CENTRE);
    Ok(below.zip(above))
}

fn mzi_spectrum(m: &Mzi, window_nm: f64) -> photonoxide::Result<ChartData> {
    let circuit = m.circuit()?;
    let guide = m.guide()?;
    let centre = Wavelength::um(RING_CENTRE)?;
    let index = |p: &str| {
        circuit
            .ports()
            .iter()
            .position(|q| q.name == p)
            .ok_or_else(|| invalid("the interferometer", format!("no port {p}")))
    };
    let (input, cross_port, bar_port) = (index("o1")?, index("o3")?, index("o4")?);
    let fsr_ng = RING_CENTRE * RING_CENTRE / (m.group_index * m.delta);
    let fsr_neff = RING_CENTRE * RING_CENTRE / (RING_INDEX * m.delta);

    // even across the window, at least 16 points a fringe
    let half = window_nm * 1e-3 / 2.0;
    let (lo, hi) = (RING_CENTRE - half, RING_CENTRE + half);
    let count = ((16.0 * (hi - lo) / fsr_ng).ceil() as usize).clamp(1200, 16000);
    let wavelengths: Vec<Wavelength> = (0..=count)
        .map(|k| Wavelength::um(lo + (hi - lo) * k as f64 / count as f64))
        .collect::<photonoxide::Result<_>>()?;
    let spectrum = circuit.spectrum(&wavelengths)?;
    let (mut bar, mut crossed, mut total) = (Vec::new(), Vec::new(), Vec::new());
    for (w, s) in wavelengths.iter().zip(spectrum.matrices()) {
        let (b, c) = (s.power(bar_port, input), s.power(cross_port, input));
        bar.push([w.to_um(), b]);
        crossed.push([w.to_um(), c]);
        total.push([w.to_um(), b + c]);
    }

    let peaks = neighbouring_peaks(&circuit, fsr_ng)?;
    let (upper, lower) = m.arms()?;
    let [bar_extremes, cross_extremes] = mzi_extremes(m.split, m.combine, upper, lower)?;
    let none = |label: &str, note: &str| Figure {
        label: label.to_owned(),
        value: None,
        text: "–".to_owned(),
        unit: String::new(),
        note: note.to_owned(),
    };
    let mut figures = Vec::new();
    match peaks {
        Some((l1, l2)) => {
            figures.push(figure(
                "Free spectral range, between peaks",
                (l2 - l1) * 1e3,
                5,
                "nm",
                "the cross port's peaks $\\lambda_1$, $\\lambda_2$ either side of 1.55 µm",
            ));
            figures.push(figure(
                "$n_g$ read from the peaks",
                l1 * l2 / ((l2 - l1) * m.delta),
                6,
                "",
                "$\\lambda_1 \\lambda_2 / ((\\lambda_2 - \\lambda_1) \\Delta L)$",
            ));
        }
        None => {
            figures.push(none(
                "Free spectral range, between peaks",
                "no fringes: one arm carries no light",
            ));
            figures.push(none("$n_g$ read from the peaks", ""));
        }
    }
    figures.extend([
        figure(
            "$\\lambda^2 / (n_g \\Delta L)$",
            fsr_ng * 1e3,
            5,
            "nm",
            "at 1.55 µm",
        ),
        figure(
            "$\\lambda^2 / (n_\\text{eff} \\Delta L)$",
            fsr_neff * 1e3,
            5,
            "nm",
            "with the effective index instead: not the spacing",
        ),
        figure(
            "Order at 1.55 µm",
            guide.effective_index_at(centre) * m.delta / RING_CENTRE,
            5,
            "",
            "$n_\\text{eff} \\Delta L / \\lambda$",
        ),
        extinction_figure("Extinction, cross", cross_extremes),
        extinction_figure("Extinction, bar", bar_extremes),
        figure(
            "The longer arm loses more by",
            m.loss * m.delta * 1e-4,
            3,
            "dB",
            "$\\alpha \\Delta L$",
        ),
    ]);
    let marker = peaks.map(|(l1, l2)| {
        if (l1 - RING_CENTRE).abs() < (l2 - RING_CENTRE).abs() {
            l1
        } else {
            l2
        }
    });
    Ok(ChartData {
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_range: Some([-0.02, 1.02]),
        series: vec![
            Curve {
                label: "cross".to_owned(),
                points: crossed,
                dashed: false,
            },
            Curve {
                label: "bar".to_owned(),
                points: bar,
                dashed: false,
            },
            Curve {
                label: "both outputs together".to_owned(),
                points: total,
                dashed: true,
            },
        ],
        marker: marker.filter(|x| (lo..=hi).contains(x)),
        figures,
        note: format!(
            "The netlist solved as a circuit, exact: no grid. Ideal couplers; arms of {MZI_ARM} and {MZI_ARM} + ΔL µm of a silicon wire, n_eff {RING_INDEX} at {RING_CENTRE} µm and n_g {}, first order in wavelength.",
            m.group_index
        ),
    })
}

/// The κ₁² at which an output goes dark, its two paths equal, r₁x = k₁y: x the amplitude the
/// path through the lower arm has after the splitter, y the upper's (the bar's r₁r₂a_l =
/// k₁k₂a_u: x = r₂a_l, y = k₂a_u; the cross's r₁k₂a_l = k₁r₂a_u: x = k₂a_l, y = r₂a_u).
fn balance(lower_path: f64, upper_path: f64) -> Option<f64> {
    // r₁² = 1 − κ₁² and k₁² = κ₁², so (1 − κ₁²)x² = κ₁²y²
    let (x2, y2) = (lower_path * lower_path, upper_path * upper_path);
    (x2 + y2 > 0.0).then(|| x2 / (x2 + y2))
}

fn mzi_extinction(combine: f64, loss: f64, delta: f64) -> photonoxide::Result<ChartData> {
    let m = Mzi {
        delta,
        split: 0.5,
        combine,
        loss,
        phase: 0.0,
        group_index: 4.2,
    };
    let (upper, lower) = m.arms()?;
    let capped = |[max, min]: [f64; 2]| {
        let db = 10.0 * (max / min).log10();
        if db.is_nan() { 0.0 } else { db.min(MZI_CAP) }
    };
    let (mut bar, mut crossed) = (Vec::new(), Vec::new());
    // denser near the middle, where the dips are narrow
    for k in 0..=800 {
        let u = f64::from(k) / 800.0;
        let kappa2 = 0.5 - 0.5 * (PI * u).cos();
        let [b, c] = mzi_extremes(kappa2, combine, upper, lower)?;
        bar.push([kappa2, capped(b)]);
        crossed.push([kappa2, capped(c)]);
    }
    let (r2, k2) = {
        let (t, x) = coupler_fields(combine)?;
        (t.norm(), x.norm())
    };
    let bar_dark = balance(r2 * lower, k2 * upper);
    let cross_dark = balance(k2 * lower, r2 * upper);
    let at = |kappa2: f64| mzi_extremes(kappa2, combine, upper, lower);
    let dark = |label: &str, v: Option<f64>, note: &str| match v {
        Some(v) => figure(label, v, 5, "", note),
        None => Figure {
            label: label.to_owned(),
            value: None,
            text: "–".to_owned(),
            unit: String::new(),
            note: "no light reaches it".to_owned(),
        },
    };
    let figures = vec![
        dark(
            "Bar dark at $\\kappa_1^2$",
            bar_dark,
            "$r_1 r_2 a_l = k_1 k_2 a_u$",
        ),
        dark(
            "Cross dark at $\\kappa_1^2$",
            cross_dark,
            "$r_1 k_2 a_l = k_1 r_2 a_u$",
        ),
        extinction_figure("Bar, a 50:50 splitter", at(0.5)?[0]),
        extinction_figure("Cross, a 50:50 splitter", at(0.5)?[1]),
        extinction_figure("Bar, a 55:45 splitter", at(0.55)?[0]),
        extinction_figure("Cross, a 55:45 splitter", at(0.55)?[1]),
    ];
    Ok(ChartData {
        x_label: "splitter κ₁²".to_owned(),
        x_length: false,
        y_label: "extinction (dB)".to_owned(),
        y_range: Some([-1.0, MZI_CAP + 2.0]),
        series: vec![
            Curve {
                label: "cross".to_owned(),
                points: crossed,
                dashed: false,
            },
            // dashed, so that the cross curve shows beneath it where they agree (an even combiner)
            Curve {
                label: "bar".to_owned(),
                points: bar,
                dashed: true,
            },
        ],
        marker: bar_dark,
        figures,
        note: format!(
            "The closed form, exact: no grid. Ideal couplers; arms of {MZI_ARM} and {MZI_ARM} + ΔL µm, each losing α; an extinction above {MZI_CAP} dB is drawn at {MZI_CAP}."
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> BTreeMap<String, f64> {
        BTreeMap::new()
    }

    fn with(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|&(k, v)| (k.to_owned(), v)).collect()
    }

    fn fig<'a>(data: &'a ChartData, label: &str) -> &'a Figure {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    fn close(a: f64, b: f64, tol: f64) {
        assert!(
            (a - b).abs() <= tol * b.abs().max(1e-300),
            "{a} against {b}"
        );
    }

    #[test]
    fn every_spec_is_sound_and_evaluates_at_its_defaults() {
        let specs = specs();
        for (k, s) in specs.iter().enumerate() {
            assert!(specs[..k].iter().all(|o| o.id != s.id), "{} twice", s.id);
            assert!(!s.params.is_empty(), "{}", s.id);
            for p in &s.params {
                assert!(
                    p.min < p.max && (p.min..=p.max).contains(&p.default),
                    "{}.{}",
                    s.id,
                    p.key
                );
                assert!(
                    p.step > 0.0 && (!p.log || p.min > 0.0),
                    "{}.{}",
                    s.id,
                    p.key
                );
                assert!(
                    ["", "um", "nm", "dB/cm"].contains(&p.unit),
                    "{}.{}",
                    s.id,
                    p.key
                );
            }
            let data = evaluate(s.id, &none()).unwrap();
            assert!(
                !data.series.is_empty() && !data.figures.is_empty(),
                "{}",
                s.id
            );
            for c in &data.series {
                assert!(c.points.len() > 10, "{}: {}", s.id, c.label);
                assert!(
                    c.points
                        .iter()
                        .all(|p| p[0].is_finite() && p[1].is_finite()),
                    "{}: {}",
                    s.id,
                    c.label
                );
            }
        }
    }

    #[test]
    fn values_out_of_range_or_unknown_are_refused() {
        assert!(evaluate("ring-spectrum", &with(&[("radius", 1.0)])).is_err());
        assert!(evaluate("ring-spectrum", &with(&[("radius", 500.0)])).is_err());
        assert!(evaluate("ring-spectrum", &with(&[("coupling", f64::NAN)])).is_err());
        assert!(evaluate("ring-spectrum", &with(&[("width", 1.0)])).is_err());
        assert!(evaluate("bragg-reflectance", &with(&[("pairs", 2.5)])).is_err());
        assert!(
            evaluate(
                "bragg-reflectance",
                &with(&[("from", 900.0), ("to", 400.0)])
            )
            .is_err()
        );
        assert!(evaluate("no-such-chart", &none()).is_err());
    }

    #[test]
    fn block_values_carry_their_units() {
        let ring = spec("ring-spectrum").unwrap();
        let v = parse_values(
            &ring,
            "radius=10um, coupling=0.2, loss=2dB/cm, window=0.03um",
        )
        .unwrap();
        assert_eq!(v["radius"], 10.0);
        assert_eq!(v["coupling"], 0.2);
        assert_eq!(v["loss"], 2.0);
        close(v["window"], 30.0, 1e-12);
        close(
            parse_values(&ring, "radius=5000nm").unwrap()["radius"],
            5.0,
            1e-12,
        );
        close(
            parse_values(&ring, "radius = 1.5e1").unwrap()["radius"],
            15.0,
            1e-12,
        );
        assert!(parse_values(&ring, "loss=2dB/m").is_err());
        assert!(parse_values(&ring, "coupling=0.2um").is_err());
        assert!(parse_values(&ring, "radius=10um, radius=12um").is_err());
        assert!(parse_values(&ring, "radius").is_err());
        assert!(
            parse_values(&ring, "radius=1um").is_err(),
            "below its range"
        );
        let bragg = spec("bragg-reflectance").unwrap();
        close(
            parse_values(&bragg, "period=0.32um").unwrap()["period"],
            320.0,
            1e-12,
        );
    }

    /// The all-pass ring the chart draws is photonoxide's AllPassRing: the radius in µm makes a
    /// round trip of 2πR µm, the loss is in dB/cm, x is in µm and the window in nm.
    #[test]
    fn ring_spectrum_is_the_all_pass_ring() {
        let (radius, coupling, loss, ng) = (12.0, 0.08, 4.0, 4.0);
        let data = evaluate(
            "ring-spectrum",
            &with(&[
                ("radius", radius),
                ("coupling", coupling),
                ("loss", loss),
                ("group_index", ng),
                ("window", 30.0),
            ]),
        )
        .unwrap();
        let ring = AllPassRing::new(
            Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, ng).with_loss(loss),
        )
        .unwrap();
        let values = [TAU * radius, coupling];
        let through = &data.series[0];
        assert_eq!(through.label, "through");
        assert_eq!(
            data.series.len(),
            2,
            "no drop port: through and the Lorentzian"
        );
        let xs: Vec<f64> = through.points.iter().map(|p| p[0]).collect();
        close(xs[0], 1.55 - 0.015, 1e-12);
        close(*xs.last().unwrap(), 1.55 + 0.015, 1e-12);
        for p in through.points.iter().step_by(37) {
            let s = ring
                .s_matrix(Wavelength::um(p[0]).unwrap(), &values)
                .unwrap();
            assert_eq!(p[1], s.power(1, 0));
        }
        let res = ring
            .resonance(Wavelength::um(1.55).unwrap(), TAU * radius)
            .unwrap();
        assert_eq!(data.marker, Some(res.to_um()));
        close(
            fig(&data, "Free spectral range").value.unwrap(),
            ring.fsr(res, TAU * radius).unwrap() * 1e3,
            1e-15,
        );
        close(
            fig(&data, "Linewidth").value.unwrap(),
            ring.fwhm(res, &values).unwrap() * 1e6,
            1e-15,
        );
        close(
            fig(&data, "Loaded").value.unwrap(),
            ring.q_factor(res, &values).unwrap(),
            1e-15,
        );
        close(
            fig(&data, "Finesse").value.unwrap(),
            ring.finesse(res, &values).unwrap(),
            1e-15,
        );
        // a, from the loss: 4 dB/cm over 2π·12 µm
        let a = 10f64.powf(-loss * TAU * radius * 1e-4 / 20.0);
        close(fig(&data, "Single-pass").value.unwrap(), a, 1e-12);
        close(fig(&data, "Critical").value.unwrap(), 1.0 - a * a, 1e-12);
        // the line is sampled well: its dip reaches the on-resonance power
        let (_, on) = ring.extremes(res, &values).unwrap();
        let lowest = through
            .points
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        assert!(lowest - on < 1e-3 * (1.0 - on), "{lowest} against {on}");
    }

    #[test]
    fn ring_spectrum_with_a_drop_is_the_add_drop_ring() {
        let data = evaluate("ring-spectrum", &with(&[("drop", 0.05), ("coupling", 0.1)])).unwrap();
        let ring = AddDropRing::new(
            Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2).with_loss(3.0),
        )
        .unwrap();
        let values = [TAU * 10.0, 0.1, 0.05];
        assert_eq!(data.series[1].label, "drop");
        for (t, d) in data.series[0]
            .points
            .iter()
            .zip(&data.series[1].points)
            .step_by(53)
        {
            let s = ring
                .s_matrix(Wavelength::um(t[0]).unwrap(), &values)
                .unwrap();
            assert_eq!((t[1], d[1]), (s.power(1, 0), s.power(3, 0)));
        }
        let res = ring
            .resonance(Wavelength::um(1.55).unwrap(), TAU * 10.0)
            .unwrap();
        close(
            fig(&data, "Drop on").value.unwrap(),
            ring.extremes(res, &values).unwrap()[2],
            1e-15,
        );
        close(
            fig(&data, "Loaded").value.unwrap(),
            ring.q_factor(res, &values).unwrap(),
            1e-15,
        );
    }

    #[test]
    fn ring_coupling_crosses_zero_at_critical_coupling() {
        let data = evaluate("ring-coupling", &with(&[("radius", 20.0), ("loss", 5.0)])).unwrap();
        let ring = AllPassRing::new(
            Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2).with_loss(5.0),
        )
        .unwrap();
        let w = Wavelength::um(1.55).unwrap();
        for p in data.series[0].points.iter().step_by(41) {
            assert_eq!(p[1], ring.extremes(w, &[TAU * 20.0, p[0]]).unwrap().1);
        }
        let crit = data.marker.unwrap();
        assert!(ring.extremes(w, &[TAU * 20.0, crit]).unwrap().1 < 1e-20);
        let intrinsic = ring.q_factor(w, &[TAU * 20.0, 0.0]).unwrap();
        close(fig(&data, "Intrinsic").value.unwrap(), intrinsic, 1e-15);
        // at critical coupling the loaded Q is about half the intrinsic one
        let ratio = fig(&data, "Loaded").value.unwrap() / intrinsic;
        assert!((ratio - 0.5).abs() < 0.01, "{ratio}");
        // an add-drop ring is critically coupled at r₁ = r₂a
        let add_drop = evaluate("ring-coupling", &with(&[("drop", 0.02)])).unwrap();
        let a = fig(&add_drop, "Single-pass").value.unwrap();
        close(add_drop.marker.unwrap(), 1.0 - 0.98 * a * a, 1e-12);
    }

    /// The validation case's stack (mode/multilayer-bragg): 8 pairs of 2.3 and 1.38 on 1.52,
    /// quarter waves at 550 nm; the period in nm, x in µm.
    #[test]
    fn bragg_reflectance_is_the_multilayer() {
        let period = 550.0 / 4.0 * (1.0 / 2.3 + 1.0 / 1.38);
        let data = evaluate("bragg-reflectance", &with(&[("period", period)])).unwrap();
        close(fig(&data, "Bragg").value.unwrap(), 550.0, 1e-12);
        close(
            fig(&data, "High layer").value.unwrap(),
            550.0 / (4.0 * 2.3),
            1e-12,
        );
        close(data.marker.unwrap(), 0.55, 1e-12);
        let q = 1.52 * (2.3f64 / 1.38).powi(16);
        let closed = ((1.0 - q) / (1.0 + q)).powi(2);
        close(
            fig(&data, "$R$ at $\\lambda_B$, transfer").value.unwrap(),
            closed,
            1e-12,
        );
        close(
            fig(&data, "$R$ at $\\lambda_B$, closed").value.unwrap(),
            closed,
            1e-15,
        );
        // the curve is Multilayer::reflection at its wavelengths
        let one = |n: f64| c64::new(n, 0.0);
        let mut films = Vec::new();
        for _ in 0..8 {
            films.push((one(2.3), Length::nm(550.0 / (4.0 * 2.3))));
            films.push((one(1.38), Length::nm(550.0 / (4.0 * 1.38))));
        }
        let stack = Multilayer::new(one(1.0), &films, one(1.52)).unwrap();
        let curve = &data.series[0].points;
        close(curve[0][0], 0.35, 1e-12);
        close(curve.last().unwrap()[0], 0.95, 1e-12);
        for p in curve.iter().step_by(97) {
            let r = stack
                .reflection(Polarization::Te, Wavelength::um(p[0]).unwrap(), 0.0)
                .unwrap();
            close(p[1], r.reflectance, 1e-12);
        }
    }

    #[test]
    fn bragg_bandwidth_finds_the_band_edges_from_one_period() {
        let data = evaluate("bragg-bandwidth", &with(&[("low", 1.5), ("up_to", 1.0)])).unwrap();
        let (measured, exact) = (&data.series[0].points, &data.series[1].points);
        assert_eq!(measured.len(), 100);
        close(measured.last().unwrap()[0], 1.0, 1e-15);
        // the period's transmission at the Bragg wavelength: the half trace of M_H M_L there,
        // −(ρ + 1/ρ)/2, from Multilayer::reflection
        let one = |n: f64| c64::new(n, 0.0);
        let cell = Multilayer::new(
            one(1.5),
            &[
                (one(1.5), Length::nm(1000.0 / 12.0)),
                (one(2.5), Length::nm(100.0)),
                (one(1.5), Length::nm(1000.0 / 12.0)),
            ],
            one(1.5),
        )
        .unwrap();
        let rho = 2.5 / 1.5;
        close(
            bloch_cos(&cell, 1000.0).unwrap(),
            -(rho + 1.0 / rho) / 2.0,
            1e-12,
        );
        // the edges where cos KΛ = −1 are the closed form's, at every contrast, in percent
        for (m, e) in measured.iter().zip(exact) {
            close(m[1], e[1], 1e-9);
        }
        close(
            exact.last().unwrap()[1],
            100.0 * band_width(2.5, 1.5),
            1e-15,
        );
        // and the coupled-mode estimate agrees with them for a weak contrast
        let (c, e0) = (data.series[2].points[0][1], exact[0][1]);
        assert!((c - e0).abs() < 1e-3 * e0, "{c} against {e0}");
        assert_eq!(bloch_band(1.5, 1.5).unwrap(), 0.0, "equal indices: no band");
    }

    /// The interferometer the chart solves is the closed form's, the product of the couplers' and
    /// the arms' transfer matrices (components/mzi-closed-form), with the phase shifter's e^(iφ)
    /// on the upper arm: µm, dB/cm and φ in units of π.
    #[test]
    fn mzi_spectrum_is_the_interferometer() {
        let (delta, split, combine, loss, phase) = (80.0, 0.4, 0.6, 10.0, 0.3);
        let data = evaluate(
            "mzi-spectrum",
            &with(&[
                ("delta", delta),
                ("split", split),
                ("combine", combine),
                ("loss", loss),
                ("phase", phase),
                ("window", 30.0),
            ]),
        )
        .unwrap();
        let guide = Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2).with_loss(loss);
        let wire = Waveguide::new(guide);
        let fields = |k: f64| {
            let s = Coupler::new()
                .s_matrix(Wavelength::um(1.55).unwrap(), &[k])
                .unwrap();
            (s[(3, 0)], s[(2, 0)])
        };
        let (crossed, bar, total) = (&data.series[0], &data.series[1], &data.series[2]);
        assert_eq!(
            (crossed.label.as_str(), bar.label.as_str()),
            ("cross", "bar")
        );
        close(crossed.points[0][0], 1.55 - 0.015, 1e-12);
        close(crossed.points.last().unwrap()[0], 1.55 + 0.015, 1e-12);
        for ((c, b), t) in crossed
            .points
            .iter()
            .zip(&bar.points)
            .zip(&total.points)
            .step_by(61)
        {
            let w = Wavelength::um(c[0]).unwrap();
            let arm = |l: f64| wire.s_matrix(w, &[l, loss]).unwrap()[(1, 0)];
            let shifter = c64::from_polar(1.0, phase * PI);
            let m = mzi_closed_form(
                fields(split),
                fields(combine),
                arm(100.0 + delta) * shifter,
                arm(100.0),
            );
            assert!((b[1] - m[0][0].norm_sqr()).abs() < 1e-12, "bar at {}", c[0]);
            assert!(
                (c[1] - m[1][0].norm_sqr()).abs() < 1e-12,
                "cross at {}",
                c[0]
            );
            assert!((t[1] - b[1] - c[1]).abs() < 1e-15);
        }
        // the longer arm loses α ΔL more
        close(
            fig(&data, "The longer arm").value.unwrap(),
            loss * delta * 1e-4,
            1e-12,
        );
    }

    /// The fringes are a free spectral range λ²/(n_g ΔL) apart, not λ²/(n_eff ΔL), and the
    /// peaks either side of 1.55 µm give back the wire's n_g by Dwivedi et al.'s Eq. 8, exactly
    /// for a wire first order in λ (whose n_g doesn't change with λ).
    #[test]
    fn mzi_fringes_give_back_the_group_index() {
        let data = evaluate(
            "mzi-spectrum",
            &with(&[("group_index", 4.27), ("delta", 73.0)]),
        )
        .unwrap();
        close(fig(&data, "$n_g$ read").value.unwrap(), 4.27, 1e-8);
        close(
            fig(&data, "$\\lambda^2 / (n_g").value.unwrap(),
            1.55 * 1.55 / (4.27 * 73.0) * 1e3,
            1e-12,
        );
        close(
            fig(&data, "$\\lambda^2 / (n_\\text{eff}").value.unwrap(),
            1.55 * 1.55 / (2.4 * 73.0) * 1e3,
            1e-12,
        );
        let between = fig(&data, "Free spectral range").value.unwrap();
        let formula = fig(&data, "$\\lambda^2 / (n_g").value.unwrap();
        assert!((between - formula).abs() < 0.02 * formula);
        // the marker is the cross port's peak nearest 1.55 µm, where it carries all the light
        let peak = data.marker.unwrap();
        assert!((peak - 1.55).abs() <= between * 1e-3 / 2.0 + 1e-9);
        let m = Mzi {
            delta: 73.0,
            split: 0.5,
            combine: 0.5,
            loss: 3.0,
            phase: 0.0,
            group_index: 4.27,
        };
        let (upper, lower) = m.arms().unwrap();
        let brightest = mzi_extremes(0.5, 0.5, upper, lower).unwrap()[1][0];
        close(
            cross(&m.circuit().unwrap(), peak).unwrap(),
            brightest,
            1e-12,
        );
    }

    /// Two 50:50 couplers and lossless arms: the cross port carries cos²(Δφ/2) and the bar
    /// sin²(Δφ/2), both dark at their minima; a shifter's π swaps them.
    #[test]
    fn a_balanced_lossless_mzi_is_cos_squared() {
        let data = evaluate("mzi-spectrum", &with(&[("loss", 0.0), ("phase", 0.5)])).unwrap();
        for (c, b) in data.series[0]
            .points
            .iter()
            .zip(&data.series[1].points)
            .step_by(47)
        {
            let phi = TAU * 2.4 * 50.0 / c[0]
                + TAU * (2.4 - 4.2) / 1.55 * (c[0] - 1.55) * 50.0 / c[0]
                + 0.5 * PI;
            close(c[1], (phi / 2.0).cos().powi(2), 1e-9);
            close(b[1] + c[1], 1.0, 1e-12);
        }
        assert_eq!(fig(&data, "Extinction, cross").text, "> 99");
        assert_eq!(fig(&data, "Extinction, bar").text, "> 99");
        // no light in the upper arm: no fringes to read
        let none = evaluate("mzi-spectrum", &with(&[("split", 0.0)])).unwrap();
        assert!(fig(&none, "Free spectral range").value.is_none());
        assert!(none.marker.is_none());
    }

    #[test]
    fn mzi_extinction_falls_as_the_splitter_leaves_50_50() {
        let data = evaluate("mzi-extinction", &none()).unwrap();
        // both outputs are dark where their two paths are equal: just past 0.5, the upper arm
        // being the lossier
        let at = data.marker.unwrap();
        let m = Mzi {
            delta: 50.0,
            split: at,
            combine: 0.5,
            loss: 3.0,
            phase: 0.0,
            group_index: 4.2,
        };
        let (upper, lower) = m.arms().unwrap();
        close(at, lower * lower / (lower * lower + upper * upper), 1e-15);
        assert!(at > 0.5 && at < 0.501, "{at}");
        let [bar, crossed] = mzi_extremes(at, 0.5, upper, lower).unwrap();
        assert!(bar[1] < 1e-20 && crossed[1] < 1e-20, "{bar:?} {crossed:?}");
        // a 55:45 splitter: ((A + B)/(A − B))², A = r₁r₂a_l and B = k₁k₂a_u
        let (a, b) = (
            (0.45f64 * 0.5).sqrt() * lower,
            (0.55f64 * 0.5).sqrt() * upper,
        );
        close(
            fig(&data, "Bar, a 55:45").value.unwrap(),
            20.0 * ((a + b) / (b - a)).log10(),
            1e-9,
        );
        // the curve is capped, rises to its cap at the dark point, and is 0 dB at either end
        let bar_curve = &data.series[1].points;
        assert_eq!(bar_curve[0][1], 0.0);
        assert!(bar_curve.iter().all(|p| p[1] <= MZI_CAP));
        assert!(bar_curve.iter().any(|p| p[1] > 40.0));
        // equal combiner and lossless arms: dark at 0.5 exactly
        let lossless = evaluate("mzi-extinction", &with(&[("loss", 0.0)])).unwrap();
        close(lossless.marker.unwrap(), 0.5, 1e-15);
    }

    #[test]
    fn numbers_are_written_as_people_write_them() {
        assert_eq!(sig(1.55, 4), "1.550");
        assert_eq!(sig(9.081, 4), "9.081");
        assert_eq!(sig(134_130.0, 4), "1.341×10⁵");
        assert_eq!(sig(0.0004318, 4), "4.318×10⁻⁴");
        assert_eq!(sig(f64::INFINITY, 4), "∞");
    }
}

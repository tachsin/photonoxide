//! The Materials page's source designer: a photon-pair source as the library computes it
//! (`photonoxide::nonlinear::pairs`, `photonoxide::quantum`), beside the nonlinear jobs.
//!
//! The source is one of four:
//!
//! - **a guide** given by each wave's group index and dispersion at its centre, phase matched at
//!   the centres, for down-conversion or four-wave mixing (`pairs::joint_spectrum`);
//! - **a crystal** of the catalogue, each wave on an axis, quasi-phase-matched by a grating
//!   designed for the centres (`pairs::joint_spectrum`);
//! - **a ring** point-coupled to its bus, the pump on a resonance and the pairs on the resonances
//!   a number of free spectral ranges either side (`quantum::ring_spectrum`);
//! - **a guide pumped in two modes**, one pump delayed (`quantum::dual_pump_spectrum`).
//!
//! For each it returns the joint spectral amplitude and intensity on the grid, the Schmidt
//! weights with the purity and the Schmidt number (`JointSpectrum::schmidt`), the brightness where
//! the library defines one (down-conversion: `guide::pair_rate_density`), the heralding through
//! each arm's filter and coupler from their S-matrices (`quantum::path`,
//! `JointSpectrum::heralding`), and the pairs at a mean number per pulse (`quantum::Source`):
//! the multi-pair probability and the unheralded g⁽²⁾.
//!
//! The library does the physics; this module checks the design against its limits, hands it to
//! the library, and returns what comes back as the view draws it.

use std::f64::consts::{LN_2, TAU};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::circuit::components::{AddDropRing, AllPassRing, Coupler, Dispersion};
use photonoxide::material::catalogue::Axis;
use photonoxide::nonlinear::coupled::coupling_from_efficiency;
use photonoxide::nonlinear::guide::pair_rate_density;
use photonoxide::nonlinear::pairs::{self, JointSpectrum, Matching, Pump};
use photonoxide::nonlinear::phase::{Device, Index, Process, State, Wave};
use photonoxide::quantum::{self, DualPump, RingGrid, Source};
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};
use serde::{Deserialize, Serialize};

/// The fewest points across each axis of the grid: fewer don't resolve a Schmidt decomposition.
pub const FEWEST_POINTS: usize = 11;
/// The most: an SVD of 301 × 301 and its maps stay live as the form changes.
pub const MOST_POINTS: usize = 301;
/// The most Schmidt weights returned to be drawn, largest first.
pub const SHOWN_WEIGHTS: usize = 30;
/// The most pairs counted in a pulse's distribution.
pub const MOST_PAIRS: usize = 5;
/// The most mean pairs per pulse: the Schmidt modes are the low-gain ones, which a higher gain
/// reshapes (docs/methods/quantum.md, "Pair sources").
pub const MOST_MEAN_PAIRS: f64 = 1.0;
/// Above this many mean pairs per pulse, the view says the modes may no longer be the low-gain
/// ones.
pub const LOW_GAIN: f64 = 0.1;
/// The largest share of the signal's or the idler's frequency the grid may span either side.
pub const MOST_HALF: f64 = 0.15;
/// The pump's integral's points for a ring, as the published ring cases take it.
pub const RING_PUMP_POINTS: usize = 4001;
/// The share of the peak the amplitude may still have at the grid's edge before the view says
/// the spectrum is cut.
pub const EDGE: f64 = 0.01;

/// Hertz to 1/µm and back: ν = f/c.
fn hz(per_um: f64) -> f64 {
    per_um * SPEED_OF_LIGHT * 1e6
}

/// The pump: its centre, its bandwidth and, for a guide pumped in two modes, its delay.
#[derive(Clone, Debug, Deserialize)]
pub struct PumpSpec {
    /// The centre's vacuum wavelength, µm.
    pub wavelength_um: f64,
    /// The FWHM of its intensity spectrum, nm.
    pub bandwidth_nm: f64,
}

/// The process of a guide: down-conversion of one pump photon or four-wave mixing of two.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PairProcess {
    Spdc,
    Sfwm,
}

/// The phase matching's shape: the device's sinc, or the Gaussian of the same width.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Sinc,
    Gaussian,
}

impl Shape {
    fn matching(self) -> Matching {
        match self {
            Shape::Sinc => Matching::Sinc,
            Shape::Gaussian => Matching::Gaussian,
        }
    }
}

/// One mode of a guide pumped in two modes, at the pump's wavelength.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ModeSpec {
    pub effective_index: f64,
    pub group_index: f64,
    /// D, ps/(nm·km).
    pub dispersion: f64,
}

/// Where the pairs are made.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Medium {
    /// A guide given by each wave's group index and dispersion (pump, signal, idler) at its
    /// centre, phase matched at the centres.
    Guide {
        process: PairProcess,
        group_index: [f64; 3],
        /// D of each wave, ps/(nm·km).
        dispersion: [f64; 3],
        length_mm: f64,
        matching: Shape,
        /// η, %/(W·cm²), for down-conversion's brightness.
        efficiency: Option<f64>,
    },
    /// A catalogue crystal, each wave (pump, signal, idler) on an axis, quasi-phase-matched at
    /// the centres.
    Crystal {
        id: String,
        model: Option<String>,
        axes: [String; 3],
        temperature_c: f64,
        length_mm: f64,
        matching: Shape,
        efficiency: Option<f64>,
    },
    /// A ring point-coupled to its bus: the pump on the resonance nearest its wavelength, the
    /// signal and the idler `order` free spectral ranges above and below it.
    Ring {
        radius_um: f64,
        effective_index: f64,
        group_index: f64,
        dispersion: f64,
        /// κ², the power coupling to the bus.
        coupling: f64,
        loss_db_per_cm: f64,
        /// The share of the pump's power a second channel takes per round trip (Vernon et
        /// al.'s dual coupling), which widens the pump's resonance alone.
        pump_extra: f64,
        order: usize,
        /// The grid's half width, in the signal's linewidths.
        span: f64,
    },
    /// A guide pumped in two modes (a and b), pump a delayed; the signal in mode b and the
    /// idler in mode a.
    DualPump {
        modes: [ModeSpec; 2],
        length_mm: f64,
        delay_ps: f64,
    },
}

/// A filter in an arm: none, or an add-drop ring whose drop port takes the photon, a resonance
/// on the arm's centre.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Filter {
    None,
    Ring {
        radius_um: f64,
        group_index: f64,
        /// κ² of both its couplers.
        coupling: f64,
        loss_db_per_cm: f64,
    },
}

/// What a photon passes on its way to its detector.
#[derive(Clone, Debug, Deserialize)]
pub struct Arm {
    pub filter: Filter,
    /// A coupler whose cross port (its share κ²) leads on, if any.
    pub coupler: Option<f64>,
    /// The coupler's excess loss, dB.
    pub coupler_loss_db: f64,
    /// The detector's efficiency.
    pub detector: f64,
}

/// A photon-pair source to compute.
#[derive(Clone, Debug, Deserialize)]
pub struct Design {
    pub medium: Medium,
    pub pump: PumpSpec,
    /// The signal's wavelength, µm: by default twice the pump's for down-conversion; a guide's
    /// four-wave mixing and a guide pumped in two modes need it.
    pub signal_um: Option<f64>,
    /// The grid's half width in the signal's wavelength, nm (a ring's is `span`).
    pub half_nm: f64,
    pub points: usize,
    /// μ, the mean number of pairs per pulse.
    pub mean_pairs: f64,
    /// The signal's arm and the idler's.
    pub arms: [Arm; 2],
}

/// A number about the source the library gives, with its unit and where it comes from.
#[derive(Debug, Serialize)]
pub struct Fact {
    label: String,
    value: f64,
    unit: String,
    about: String,
}

fn fact(label: &str, value: f64, unit: &str, about: &str) -> Fact {
    Fact {
        label: label.into(),
        value,
        unit: unit.into(),
        about: about.into(),
    }
}

/// Down-conversion's brightness, `guide::pair_rate_density` at the guide's coupling.
#[derive(Debug, Serialize)]
pub struct Brightness {
    /// At the signal's centre, pairs/(s·GHz·mW).
    spectral: f64,
    /// Over the grid's signal frequencies for a continuous pump, pairs/(s·mW).
    rate: f64,
}

/// What a heralded photon and its herald pass: the heralding efficiencies through the arms'
/// filters and couplers, without and with the detectors, and the heralded photons' purity.
#[derive(Debug, Serialize)]
pub struct Heralded {
    /// η_s and η_i from the arms' S-matrices (`JointSpectrum::heralding`).
    efficiency: [f64; 2],
    /// The same times each photon's own detector's efficiency.
    detected: [f64; 2],
    /// The filtered spectrum's purity and Schmidt number.
    purity: f64,
    schmidt_number: f64,
    /// Each arm's power transmission at its centre.
    centre: [f64; 2],
    facts: Vec<Fact>,
}

/// A source as the view draws it.
#[derive(Debug, Serialize)]
pub struct Designed {
    pump_um: f64,
    signal_um: f64,
    idler_um: f64,
    /// The maps' axes: `"GHz"` (detunings from each centre) or `"nm"` (wavelengths).
    unit: String,
    signal_axis: Vec<f64>,
    idler_axis: Vec<f64>,
    /// |f| / max |f|, row by row: the i-th signal and the j-th idler at i × points + j.
    amplitude: Vec<f64>,
    /// |f|² / max |f|².
    intensity: Vec<f64>,
    points: usize,
    /// The Schmidt weights λ_k above 10⁻⁶, largest first, at most [`SHOWN_WEIGHTS`].
    weights: Vec<f64>,
    /// How many weights are above 10⁻⁶.
    modes: usize,
    purity: f64,
    schmidt_number: f64,
    brightness: Option<Brightness>,
    /// Why there is no brightness, when there isn't.
    brightness_note: Option<String>,
    heralding: Heralded,
    mean_pairs: f64,
    /// P(0), P(1), … P([`MOST_PAIRS`]) pairs in a pulse.
    pairs: Vec<f64>,
    multi_pair: f64,
    g2: f64,
    facts: Vec<Fact>,
    notes: Vec<String>,
    seconds: f64,
}

/// A source's joint spectrum and what was found on the way.
struct Built {
    spectrum: JointSpectrum,
    pump_um: f64,
    signal_um: f64,
    idler_um: f64,
    ghz: bool,
    brightness: Option<Brightness>,
    brightness_note: Option<String>,
    facts: Vec<Fact>,
    notes: Vec<String>,
}

/// The photon-pair source `design`, computed.
#[tauri::command(async)]
pub fn pair_source(design: Design) -> Result<Designed, String> {
    designed(&design)
}

fn within(value: f64, low: f64, high: f64, what: &str, unit: &str) -> Result<f64, String> {
    if value.is_finite() && value >= low && value <= high {
        Ok(value)
    } else {
        Err(format!(
            "{what} must be from {low} to {high}{unit}, not {value}{unit}"
        ))
    }
}

fn positive(value: f64, high: f64, what: &str, unit: &str) -> Result<f64, String> {
    if value.is_finite() && value > 0.0 && value <= high {
        Ok(value)
    } else {
        Err(format!(
            "{what} must be above 0 and at most {high}{unit}, not {value}{unit}"
        ))
    }
}

fn um(value: f64) -> Result<Wavelength, String> {
    Wavelength::um(value).map_err(|e| e.to_string())
}

fn err(e: photonoxide::Error) -> String {
    e.to_string()
}

pub(crate) fn designed(d: &Design) -> Result<Designed, String> {
    let start = Instant::now();
    if !(FEWEST_POINTS..=MOST_POINTS).contains(&d.points) {
        return Err(format!(
            "the grid takes {FEWEST_POINTS} to {MOST_POINTS} points across each axis, not {}: \
             fewer don't resolve a Schmidt decomposition, and more aren't live",
            d.points
        ));
    }
    within(
        d.pump.wavelength_um,
        0.3,
        5.0,
        "the pump's wavelength",
        " µm",
    )?;
    positive(
        d.mean_pairs,
        MOST_MEAN_PAIRS,
        "the mean number of pairs per pulse",
        "",
    )?;
    for (arm, name) in d.arms.iter().zip(["signal", "idler"]) {
        positive(
            arm.detector,
            1.0,
            &format!("the {name}'s detector's efficiency"),
            "",
        )?;
        if let Some(k2) = arm.coupler {
            positive(k2, 1.0, &format!("the {name}'s coupler's share"), "")?;
        }
        within(
            arm.coupler_loss_db,
            0.0,
            20.0,
            &format!("the {name}'s coupler's excess loss"),
            " dB",
        )?;
    }
    let lp = d.pump.wavelength_um;
    let bandwidth = positive(
        d.pump.bandwidth_nm,
        0.1 * lp * 1e3,
        "the pump's bandwidth",
        " nm",
    )? * 1e-3
        / (lp * lp);
    let mut built = match &d.medium {
        Medium::Guide { .. } | Medium::Crystal { .. } => made_in_guide(d, bandwidth)?,
        Medium::Ring { .. } => made_in_ring(d, bandwidth)?,
        Medium::DualPump { .. } => made_in_two_modes(d, bandwidth)?,
    };
    built.facts.insert(
        0,
        fact(
            "pump's pulse",
            4.0 * LN_2 / (TAU * hz(bandwidth)) * 1e12,
            "ps",
            "the FWHM of a transform-limited Gaussian pulse of this bandwidth, 0.4413/Δν",
        ),
    );
    let s = &built.spectrum;
    let schmidt = s.schmidt().map_err(err)?;
    let source = Source::new(&schmidt.weights, d.mean_pairs).map_err(err)?;
    if d.mean_pairs > LOW_GAIN {
        built.notes.push(format!(
            "at {} pairs per pulse the gain may reshape the Schmidt modes, which are the low-gain \
             ones here: the pairs' numbers are then a guide",
            d.mean_pairs
        ));
    }
    let heralding = herald(s, &d.arms, [built.signal_um, built.idler_um])?;

    // the maps, and whether the grid holds the spectrum
    let n = d.points;
    let magnitude: Vec<f64> = s.amplitude.iter().map(|z| z.norm()).collect();
    let peak = magnitude.iter().copied().fold(0.0, f64::max);
    let amplitude: Vec<f64> = magnitude.iter().map(|m| m / peak).collect();
    let edge = (0..n)
        .flat_map(|k| [k, (n - 1) * n + k, k * n, k * n + n - 1])
        .map(|k| amplitude[k])
        .fold(0.0, f64::max);
    if edge > EDGE {
        built.notes.push(format!(
            "the amplitude is still {:.1}% of its peak at the grid's edge: widen the grid, or the \
             purity is a cut spectrum's",
            edge * 100.0
        ));
    }
    let (row, col) = {
        let at = amplitude
            .iter()
            .enumerate()
            .fold(0, |b, (k, v)| if *v > amplitude[b] { k } else { b });
        (at / n, at % n)
    };
    let across = |f: &dyn Fn(usize) -> f64| (0..n).filter(|&k| f(k) >= 0.5).count();
    let narrowest = across(&|k| amplitude[row * n + k]).min(across(&|k| amplitude[k * n + col]));
    if narrowest < 3 {
        built.notes.push(format!(
            "the amplitude is above half its peak on {narrowest} point{} across: narrow the grid \
             or add points, or the purity is the grid's",
            if narrowest == 1 { "" } else { "s" }
        ));
    }
    let axis = |nu: &[f64], centre_um: f64| -> Vec<f64> {
        if built.ghz {
            nu.iter().map(|v| hz(v - 1.0 / centre_um) * 1e-9).collect()
        } else {
            nu.iter().map(|v| 1e3 / v).collect()
        }
    };
    let weights = &schmidt.weights;
    Ok(Designed {
        pump_um: built.pump_um,
        signal_um: built.signal_um,
        idler_um: built.idler_um,
        unit: if built.ghz { "GHz" } else { "nm" }.into(),
        signal_axis: axis(&s.signal, built.signal_um),
        idler_axis: axis(&s.idler, built.idler_um),
        intensity: amplitude.iter().map(|a| a * a).collect(),
        amplitude,
        points: n,
        weights: weights
            .iter()
            .copied()
            .filter(|w| *w > 1e-6)
            .take(SHOWN_WEIGHTS)
            .collect(),
        modes: weights.iter().filter(|w| **w > 1e-6).count(),
        purity: schmidt.purity(),
        schmidt_number: schmidt.number(),
        brightness: built.brightness,
        brightness_note: built.brightness_note,
        heralding,
        mean_pairs: source.mean_pairs(),
        pairs: source.pairs(MOST_PAIRS),
        multi_pair: source.multi_pair(),
        g2: source.unheralded_g2(),
        facts: built.facts,
        notes: built.notes,
        seconds: start.elapsed().as_secs_f64(),
    })
}

/// The grid's half width (1/µm) from `half_nm` in the signal's wavelength, within
/// [`MOST_HALF`] of both photons' frequencies.
fn half_of(d: &Design, signal_um: f64, idler_um: f64) -> Result<f64, String> {
    let half =
        positive(d.half_nm, 1e4, "the grid's half width", " nm")? * 1e-3 / (signal_um * signal_um);
    let most = MOST_HALF / signal_um.max(idler_um);
    if half > most {
        return Err(format!(
            "the grid's half width, {} nm, spans more than {}% of the {}'s frequency: at most \
             {:.4} nm",
            d.half_nm,
            MOST_HALF * 100.0,
            if idler_um > signal_um {
                "idler"
            } else {
                "signal"
            },
            most * signal_um * signal_um * 1e3
        ));
    }
    Ok(half)
}

fn axis_of(name: &str) -> Result<Axis, String> {
    Ok(match name {
        "isotropic" => Axis::Isotropic,
        "ordinary" | "o" => Axis::Ordinary,
        "extraordinary" | "e" => Axis::Extraordinary,
        "x" => Axis::X,
        "y" => Axis::Y,
        "z" => Axis::Z,
        _ => {
            return Err(format!(
                "no axis {name:?}: isotropic, ordinary, extraordinary, x, y or z"
            ));
        }
    })
}

/// The pairs' wavelengths of a pump at `lp`: the signal given (or twice the pump's for
/// down-conversion) and the idler energy conservation leaves.
fn wavelengths(d: &Design, four: bool) -> Result<(f64, f64), String> {
    let lp = d.pump.wavelength_um;
    let ls = match d.signal_um {
        Some(ls) => within(ls, 0.3, 10.0, "the signal's wavelength", " µm")?,
        None if !four => 2.0 * lp,
        None => {
            return Err(
                "give the signal's wavelength: four-wave mixing's pairs sit either side of the \
                 pump"
                    .into(),
            );
        }
    };
    let nu_i = if four { 2.0 / lp } else { 1.0 / lp } - 1.0 / ls;
    if nu_i <= 0.0 {
        return Err(if four {
            format!(
                "the signal at {ls} µm leaves the idler no frequency: four-wave mixing needs it \
                 longer than half the pump's wavelength, {} µm",
                lp / 2.0
            )
        } else {
            format!("the signal at {ls} µm must be longer than the pump at {lp} µm")
        });
    }
    let li = 1.0 / nu_i;
    within(li, 0.3, 10.0, "the idler's wavelength", " µm")?;
    Ok((ls, li))
}

/// A guide's or a crystal's pairs: `pairs::joint_spectrum` of its device.
fn made_in_guide(d: &Design, bandwidth: f64) -> Result<Built, String> {
    let lp = d.pump.wavelength_um;
    let (process, length_mm, matching, efficiency) = match &d.medium {
        Medium::Guide {
            process,
            length_mm,
            matching,
            efficiency,
            ..
        } => (*process, *length_mm, *matching, *efficiency),
        Medium::Crystal {
            length_mm,
            matching,
            efficiency,
            ..
        } => (PairProcess::Spdc, *length_mm, *matching, *efficiency),
        _ => unreachable!("a guide or a crystal"),
    };
    let four = process == PairProcess::Sfwm;
    let (ls, li) = wavelengths(d, four)?;
    let half = half_of(d, ls, li)?;
    let length = positive(length_mm, 100.0, "the length", " mm")? * 1e3;
    let mut facts = Vec::new();
    let mut notes = Vec::new();
    let device = match &d.medium {
        Medium::Guide {
            group_index,
            dispersion,
            ..
        } => {
            for (n, wave) in group_index.iter().zip(["pump", "signal", "idler"]) {
                within(*n, 1.0, 5.0, &format!("the {wave}'s group index"), "")?;
            }
            for (x, wave) in dispersion.iter().zip(["pump", "signal", "idler"]) {
                within(*x, -1e5, 1e5, &format!("the {wave}'s D"), " ps/(nm·km)")?;
            }
            // one phase index at every centre: matched there, each wave's group index and D
            // around it (the circuit library's model, Dispersion)
            let phase = group_index.iter().copied().fold(0.0, f64::max);
            let index = |l0: f64, ng: f64, dd: f64| -> Result<Index, String> {
                let model = Dispersion::new(um(l0)?, phase, ng).with_dispersion(dd);
                let lams: Vec<f64> = (0..=80).map(|k| l0 * (0.7 + 0.01 * k as f64)).collect();
                let n = lams
                    .iter()
                    .map(|&l| um(l).map(|w| model.effective_index_at(w)))
                    .collect::<Result<Vec<f64>, String>>()?;
                Index::effective(lams, n).map_err(err)
            };
            let pump = Wave::new(um(lp)?, index(lp, group_index[0], dispersion[0])?);
            let signal = Wave::new(um(ls)?, index(ls, group_index[1], dispersion[1])?);
            let idler = index(li, group_index[2], dispersion[2])?;
            let process = if four {
                Process::four_wave(pump.clone(), pump, signal, idler)
            } else {
                Process::difference_frequency(pump, signal, idler)
            }
            .map_err(err)?;
            Device::new(process, length, State::default()).map_err(err)?
        }
        Medium::Crystal {
            id,
            model,
            axes,
            temperature_c,
            ..
        } => {
            within(*temperature_c, -50.0, 300.0, "the temperature", " °C")?;
            let index = |axis: &str| -> Result<Index, String> {
                Index::catalogue(id, model.as_deref(), axis_of(axis)?).map_err(err)
            };
            let process = Process::difference_frequency(
                Wave::new(um(lp)?, index(&axes[0])?),
                Wave::new(um(ls)?, index(&axes[1])?),
                index(&axes[2])?,
            )
            .map_err(err)?;
            let state = State::celsius(*temperature_c);
            let mismatch = process.mismatch(&state).map_err(err)?;
            facts.push(fact(
                "coherence length",
                process.coherence_length(&state).map_err(err)?,
                "µm",
                "π/|Δk₀| without a grating",
            ));
            match Device::quasi_phase_matched(process.clone(), length, state, 1, 0.5) {
                Ok(device) => {
                    if let Some(g) = device.grating() {
                        facts.push(fact(
                            "poling period",
                            g.period(),
                            "µm",
                            "first order, duty 0.5, designed for the centres (Grating::for_mismatch)",
                        ));
                    }
                    device
                }
                Err(e) => {
                    notes.push(format!(
                        "no grating: {e} (Δk₀ = {mismatch:.3e} rad/µm), so the pairs are as the \
                         crystal matches them"
                    ));
                    Device::new(process, length, state).map_err(err)?
                }
            }
        }
        _ => unreachable!("a guide or a crystal"),
    };
    let mut pump = Pump {
        center: 1.0 / lp,
        bandwidth,
    };
    if four {
        pump = pump.two_photon();
    }
    let spectrum =
        pairs::joint_spectrum(&device, pump, matching.matching(), half, d.points).map_err(err)?;
    let (brightness, brightness_note) = if four {
        (
            None,
            Some(
                "the library gives a pair rate for down-conversion only: for four-wave mixing \
                 the mean number of pairs per pulse is set here"
                    .to_owned(),
            ),
        )
    } else if let Some(eta) = efficiency {
        positive(eta, 1e6, "η", " %/(W·cm²)")?;
        (Some(brightness(&device, eta, &spectrum)?), None)
    } else {
        (
            None,
            Some("give η, the guide's normalized efficiency, for the pairs' rate".to_owned()),
        )
    };
    Ok(Built {
        spectrum,
        pump_um: lp,
        signal_um: ls,
        idler_um: li,
        ghz: false,
        brightness,
        brightness_note,
        facts,
        notes,
    })
}

/// Down-conversion's brightness at the normalized efficiency `eta` (%/(W·cm²)), as the
/// nonlinear job gives it (`guide::pair_rate_density` at κ = √(η·10⁻¹⁰)): at the signal's
/// centre, and summed over the grid's signal frequencies for a continuous pump at the centre.
fn brightness(device: &Device, eta: f64, spectrum: &JointSpectrum) -> Result<Brightness, String> {
    let kappa = coupling_from_efficiency(eta);
    let waves = device.process().waves();
    let (ns, ni) = (
        1.0 / waves[0].wavelength.to_um(),
        1.0 / waves[1].wavelength.to_um(),
    );
    let np = ns + ni;
    let length = device.length();
    let state = *device.state();
    // per W and per Hz, so per mW and per GHz at once: × 1e-3 × 1e9
    let spectral = pair_rate_density(kappa, ns, ni, 1.0, length, 0.0) * 1e6;
    let density = |s: f64| -> Result<f64, String> {
        let x = device
            .phase_at_frequencies(&[s, np - s, np], &state)
            .map_err(err)?;
        Ok(pair_rate_density(kappa, s, np - s, 1.0, length, x))
    };
    let nu = &spectrum.signal;
    let step = hz(nu[1] - nu[0]);
    let mut rate = 0.0;
    for (k, &s) in nu.iter().enumerate() {
        let w = if k == 0 || k + 1 == nu.len() {
            0.5
        } else {
            1.0
        };
        rate += w * density(s)?;
    }
    Ok(Brightness {
        spectral,
        rate: rate * step * 1e-3,
    })
}

/// A ring's pairs: `quantum::ring_spectrum` of its field enhancements.
fn made_in_ring(d: &Design, bandwidth: f64) -> Result<Built, String> {
    let Medium::Ring {
        radius_um,
        effective_index,
        group_index,
        dispersion,
        coupling,
        loss_db_per_cm,
        pump_extra,
        order,
        span,
    } = &d.medium
    else {
        unreachable!("a ring")
    };
    within(*radius_um, 1.0, 2000.0, "the ring's radius", " µm")?;
    within(*effective_index, 1.0, 5.0, "the ring's effective index", "")?;
    within(*group_index, 1.0, 6.0, "the ring's group index", "")?;
    within(*dispersion, -1e5, 1e5, "the ring's D", " ps/(nm·km)")?;
    if !(coupling.is_finite() && *coupling > 0.0 && *coupling < 1.0) {
        return Err(format!(
            "the ring's power coupling κ² must be between 0 and 1, not {coupling}"
        ));
    }
    within(*loss_db_per_cm, 0.0, 50.0, "the ring's loss", " dB/cm")?;
    within(*pump_extra, 0.0, 0.99, "the pump's extra coupling", "")?;
    if !(1..=50).contains(order) {
        return Err(format!(
            "the pairs sit 1 to 50 free spectral ranges from the pump, not {order}"
        ));
    }
    within(*span, 1.0, 50.0, "the grid's half width", " linewidths")?;
    let lp = d.pump.wavelength_um;
    let length = TAU * radius_um;
    let guide = Dispersion::new(um(lp)?, *effective_index, *group_index)
        .with_dispersion(*dispersion)
        .with_loss(*loss_db_per_cm);
    let ring = AllPassRing::new(guide).map_err(err)?;
    // the second channel takes a share of the pump's power per round trip: a loss of the pump's
    let pumped = ring
        .clone()
        .with_round_trip_loss(-10.0 * (1.0 - pump_extra).log10())
        .map_err(err)?;
    let values = [length, *coupling];
    let resonance = |nu: f64| -> Result<f64, String> {
        Ok(1.0 / ring.resonance(um(1.0 / nu)?, length).map_err(err)?.to_um())
    };
    let np = resonance(1.0 / lp)?;
    let fsr = ring.fsr(um(1.0 / np)?, length).map_err(err)? * np * np;
    let ns = resonance(np + *order as f64 * fsr)?;
    let ni = resonance(np - *order as f64 * fsr)?;
    let width = |r: &AllPassRing, nu: f64| -> Result<f64, String> {
        Ok(r.fwhm(um(1.0 / nu)?, &values).map_err(err)? * nu * nu)
    };
    let linewidth = width(&ring, ns)?;
    let pump_linewidth = width(&pumped, np)?;
    let enhance = |r: &AllPassRing, nu: f64| -> photonoxide::Result<c64> {
        r.field_enhancement(Wavelength::um(1.0 / nu)?, &values)
    };
    let fp = |nu: f64| enhance(&pumped, nu);
    let fs = |nu: f64| enhance(&ring, nu);
    let pump = Pump {
        center: np,
        bandwidth,
    };
    let grid = RingGrid {
        half: span * linewidth,
        points: d.points,
        pump_half: (8.0 * bandwidth).min(100.0 * pump_linewidth),
        pump_points: RING_PUMP_POINTS,
    };
    let spectrum = quantum::ring_spectrum(pump, [&fp, &fs, &fs], [ns, ni], grid).map_err(err)?;
    let at = um(1.0 / ns)?;
    let facts = vec![
        fact(
            "linewidth",
            hz(linewidth) * 1e-9,
            "GHz",
            "the signal's resonance's FWHM (AllPassRing::fwhm)",
        ),
        fact(
            "pump's linewidth",
            hz(pump_linewidth) * 1e-9,
            "GHz",
            "the pump's resonance's FWHM, with its extra coupling",
        ),
        fact(
            "loaded Q",
            ring.q_factor(at, &values).map_err(err)?,
            "",
            "λ/FWHM at the signal (AllPassRing::q_factor)",
        ),
        fact(
            "free spectral range",
            hz(fsr) * 1e-9,
            "GHz",
            "at the pump (AllPassRing::fsr)",
        ),
        fact(
            "pump/ring bandwidth",
            bandwidth / pump_linewidth,
            "",
            "the pump's bandwidth over its resonance's: far above 1, the pairs take the rings' \
             widths",
        ),
    ];
    Ok(Built {
        spectrum,
        pump_um: 1.0 / np,
        signal_um: 1.0 / ns,
        idler_um: 1.0 / ni,
        ghz: true,
        brightness: None,
        brightness_note: Some(
            "the library gives a pair rate for down-conversion only: for a ring the mean number \
             of pairs per pulse is set here"
                .into(),
        ),
        facts,
        notes: Vec::new(),
    })
}

/// A guide pumped in two modes: `quantum::dual_pump_spectrum` of the modes' propagation
/// constants, each mode's the circuit library's model (`Dispersion`) at the pump.
fn made_in_two_modes(d: &Design, bandwidth: f64) -> Result<Built, String> {
    let Medium::DualPump {
        modes,
        length_mm,
        delay_ps,
    } = &d.medium
    else {
        unreachable!("a guide pumped in two modes")
    };
    for (m, name) in modes.iter().zip(["a", "b"]) {
        within(
            m.effective_index,
            1.0,
            5.0,
            &format!("mode {name}'s effective index"),
            "",
        )?;
        within(
            m.group_index,
            1.0,
            6.0,
            &format!("mode {name}'s group index"),
            "",
        )?;
        within(
            m.dispersion,
            -1e5,
            1e5,
            &format!("mode {name}'s D"),
            " ps/(nm·km)",
        )?;
    }
    let length = positive(*length_mm, 100.0, "the length", " mm")? * 1e3;
    within(*delay_ps, -100.0, 100.0, "the delay", " ps")?;
    let lp = d.pump.wavelength_um;
    let (ls, li) = wavelengths(d, true)?;
    let half = half_of(d, ls, li)?;
    let model =
        |m: &ModeSpec| -> Result<Dispersion, String> {
            Ok(Dispersion::new(um(lp)?, m.effective_index, m.group_index)
                .with_dispersion(m.dispersion))
        };
    let (ma, mb) = (model(&modes[0])?, model(&modes[1])?);
    let beta = |m: Dispersion| {
        move |nu: f64| -> f64 {
            Wavelength::um(1.0 / nu).map_or(f64::NAN, |w| TAU * nu * m.effective_index_at(w))
        }
    };
    let (ba, bb) = (beta(ma), beta(mb));
    let guide = DualPump {
        beta: [&ba, &bb, &bb, &ba],
        length,
        delay: *delay_ps,
        pump: Pump {
            center: 1.0 / lp,
            bandwidth,
        },
    };
    let spectrum = quantum::dual_pump_spectrum(
        &guide,
        [1.0 / ls, 1.0 / li],
        half,
        d.points,
        4.0 * bandwidth,
        2 * d.points + 1,
    )
    .map_err(err)?;
    let mut facts = Vec::new();
    let mut notes = Vec::new();
    let np = 1.0 / lp;
    let phase = 0.5 * length * (ba(np) + bb(np) - bb(1.0 / ls) - ba(1.0 / li));
    facts.push(fact(
        "ΔkL/2 at the centres",
        phase,
        "rad",
        "β_a + β_b of the pumps less β_b of the signal and β_a of the idler, at the centres",
    ));
    if phase.abs() > std::f64::consts::PI {
        notes.push(format!(
            "the centres aren't phase matched (ΔkL/2 = {phase:.2} rad): the pairs sit off the \
             grid's middle, or not on it"
        ));
    }
    let dn = modes[1].group_index - modes[0].group_index;
    if dn != 0.0 {
        let z = SPEED_OF_LIGHT * 1e-6 * delay_ps / dn;
        facts.push(fact(
            "pumps' overlap centred at",
            z * 1e-3,
            "mm",
            "z = cτ/(n_g,b − n_g,a): where the delayed pump has caught up",
        ));
        if !(0.0..=length).contains(&z) {
            notes.push(format!(
                "the pumps meet at {:.2} mm, outside the guide's {:.2} mm",
                z * 1e-3,
                length * 1e-3
            ));
        }
    }
    Ok(Built {
        spectrum,
        pump_um: lp,
        signal_um: ls,
        idler_um: li,
        ghz: false,
        brightness: None,
        brightness_note: Some(
            "the library gives a pair rate for down-conversion only: for four-wave mixing the \
             mean number of pairs per pulse is set here"
                .into(),
        ),
        facts,
        notes,
    })
}

/// Each arm's amplitude transmissions at its photon's frequencies, through its filter and its
/// coupler (`quantum::path` of each), and what the filter is.
fn arm_path(
    arm: &Arm,
    frequencies: &[f64],
    centre_um: f64,
    name: &str,
    facts: &mut Vec<Fact>,
) -> Result<Vec<c64>, String> {
    let mut t = vec![c64::new(1.0, 0.0); frequencies.len()];
    if let Filter::Ring {
        radius_um,
        group_index,
        coupling,
        loss_db_per_cm,
    } = &arm.filter
    {
        within(
            *radius_um,
            1.0,
            2000.0,
            &format!("the {name}'s filter's radius"),
            " µm",
        )?;
        within(
            *group_index,
            1.0,
            6.0,
            &format!("the {name}'s filter's group index"),
            "",
        )?;
        if !(coupling.is_finite() && *coupling > 0.0 && *coupling < 1.0) {
            return Err(format!(
                "the {name}'s filter's power coupling κ² must be between 0 and 1, not {coupling}"
            ));
        }
        within(
            *loss_db_per_cm,
            0.0,
            50.0,
            &format!("the {name}'s filter's loss"),
            " dB/cm",
        )?;
        // a resonance on the photon's centre: n_eff L = m λ there
        let length = TAU * radius_um;
        let order = (group_index * length / centre_um).round().max(1.0);
        let ring = AddDropRing::new(
            Dispersion::new(um(centre_um)?, order * centre_um / length, *group_index)
                .with_loss(*loss_db_per_cm),
        )
        .map_err(err)?;
        let values = [length, *coupling, *coupling];
        let through = quantum::path(&ring, &values, "in", "drop", frequencies).map_err(err)?;
        for (a, b) in t.iter_mut().zip(&through) {
            *a *= b;
        }
        let nu = 1.0 / centre_um;
        facts.push(fact(
            &format!("{name}'s filter's width"),
            hz(ring.fwhm(um(centre_um)?, &values).map_err(err)? * nu * nu) * 1e-9,
            "GHz",
            "the add-drop ring's FWHM at the photon's centre (AddDropRing::fwhm)",
        ));
    }
    if let Some(k2) = arm.coupler {
        let coupler = Coupler::new()
            .with_excess_loss(arm.coupler_loss_db)
            .map_err(err)?;
        let across = quantum::path(&coupler, &[k2], "o1", "o3", frequencies).map_err(err)?;
        for (a, b) in t.iter_mut().zip(&across) {
            *a *= b;
        }
    }
    Ok(t)
}

/// The heralding through both arms.
fn herald(s: &JointSpectrum, arms: &[Arm; 2], centres_um: [f64; 2]) -> Result<Heralded, String> {
    let mut facts = Vec::new();
    let ts = arm_path(&arms[0], &s.signal, centres_um[0], "signal", &mut facts)?;
    let ti = arm_path(&arms[1], &s.idler, centres_um[1], "idler", &mut facts)?;
    let h = s.heralding(&ts, &ti).map_err(err)?;
    let filtered = s.filtered(&ts, &ti).map_err(err)?.schmidt().map_err(err)?;
    let mid = |t: &[c64]| t[t.len() / 2].norm_sqr();
    Ok(Heralded {
        efficiency: [h.signal, h.idler],
        detected: [h.signal * arms[0].detector, h.idler * arms[1].detector],
        purity: filtered.purity(),
        schmidt_number: filtered.number(),
        centre: [mid(&ts), mid(&ti)],
        facts,
    })
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;

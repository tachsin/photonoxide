//! A nonlinear process in a guide or a crystal: a task alone of kind `"nonlinear"`
//! ([`crate::nonlinear`]).
//!
//! The task names the process (`"shg"`, `"sfg"`, `"dfg"`, `"opa"`, `"spdc"` or `"fwm"`), the
//! waveguide its waves travel in, the grating that poles it, and the pump (and the signal). The
//! waveguide is one of three:
//!
//! - **a crystal** (`[task.waveguide.crystal]`): plane waves in a catalogue crystal, each wave's
//!   index one axis of one of its index models, rebuilt at the task's temperature;
//! - **effective indices given as data** (`[[task.waveguide.n_eff]]`): one table of n_eff over
//!   wavelength for every wave, or one per wave in the process's order;
//! - **a ridge** (`[task.waveguide.ridge]`): a film of a `[task.stack]`-like table etched to a
//!   ridge, each wave's index its fundamental mode of a family by the full-vector solver over the
//!   wavelengths the task needs ([`crate::nonlinear::guide::ModeIndex`]), and the coupling of a
//!   second-order process from the modes' overlap through the crystal's d tensor turned onto the
//!   chip ([`crate::nonlinear::guide`]).
//!
//! The waves, in each process's order: SHG's fundamental (the pump) and harmonic; SFG's first
//! input (the pump), its second (the signal) and their sum; DFG's, OPA's and SPDC's pump, signal
//! and idler; FWM's pump, signal and idler, the pump degenerate.
//!
//! The run records, after its start ([`Event::Started`]):
//!
//! - [`Event::NonlinearSetup`]: the waves with their wavelengths and indices, the mismatch Δk₀
//!   and the coherence length, the grating (given, or designed to cancel Δk₀ at the operating
//!   point), the residual Δk and the response sinc²(ΔkL/2), the normalized efficiency η and
//!   where it comes from, the acceptance bandwidths in wavelength and temperature, the
//!   phase-matched wavelength inside the map, and notes: what wasn't computed and why, and
//!   each wave that lies outside a material's transparency window
//!   ([`crate::material::catalogue::transparency`]);
//! - [`Event::PhaseMap`]: Δk and sinc²(ΔkL/2) against the tuned wave's wavelength (SHG's
//!   fundamental, every other process's signal, the pump held) and, with `width_um` or
//!   `temperature_c` in `[task.map]`, the ridge's width or the temperature;
//! - [`Event::NonlinearEfficiency`]: the generated power against the length, depleted by the
//!   coupled-amplitude equations with the losses ([`crate::nonlinear::coupled`]) beside the
//!   undepleted closed form; for OPA the signal's gain, for SPDC the pairs' spectral brightness,
//!   for FWM the signal's gain and the idler's conversion ([`crate::nonlinear::fwm`]);
//! - [`Event::PairSpectrum`] for SPDC and FWM: the joint spectral amplitude, with its purity and
//!   Schmidt number ([`crate::nonlinear::pairs`]).
//!
//! A run with a folder writes the same numbers beside its record: `nonlinear.json` (the four
//! events' contents), `phase_map.csv`, `efficiency.csv` and `pair_spectrum.csv`.
//!
//! ```yaml
//! name: ppln-shg
//! timeout_minutes: 1
//!
//! task:
//!   kind: nonlinear
//!   process: shg               # shg, sfg, dfg, opa, spdc or fwm
//!   length_mm: 1.0
//!   temperature_c: 25.0
//!   waveguide:
//!     crystal:                 # or n_eff (tables), or ridge (a film's modes)
//!       id: linbo3
//!       model: edwards-lawrence-1984
//!       axes: [extraordinary]  # each wave's axis, or one for all: d33, type 0
//!     efficiency_percent_per_w_cm2: 100.0
//!     expansion: linbo3-jundt-1997
//!   poling: { order: 1, duty: 0.5 }   # the period designed for the pump, or period_um
//!   pump: { wavelength_um: 1.064, power_mw: 100.0 }
//!   map:
//!     wavelength_um: [1.06, 1.068]
//!     points: 201
//!     temperature_c: [15.0, 35.0]
//!     across_points: 21
//! ```

use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;
use std::path::Path;

use num_complex::Complex64 as c64;
use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Serialize};

use super::execute::solver_run;
use super::kind::TaskKind;
use super::{Event, task_error};
use crate::material::catalogue::{self, Axis, Conditions, TensorKind};
use crate::material::{Material, Orientation};
use crate::mode::pockels::Along;
use crate::nonlinear::coupled::{
    SecondHarmonic, ThreeWave, coupling_from_efficiency, loss_from_db_per_cm, undepleted_lossy_shg,
};
use crate::nonlinear::fwm;
use crate::nonlinear::guide::{
    Family, Grid, ModeField, ModeIndex, Ridge, SecondOrder, fundamental, pair_rate_density,
    percent_per_watt_cm2, second_harmonic_coupling, sum_frequency_coupling,
};
use crate::nonlinear::pairs::{self, Matching, Pump};
use crate::nonlinear::phase::{
    Device, Expansion, Grating, Index, Process, State, Variable, Wave, sinc2,
};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::{Error, Result};

// -------------------------------------------------------------------------------------------
// The task
// -------------------------------------------------------------------------------------------

/// A `"nonlinear"` task: a process, its waveguide, its grating and its pump.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NonlinearTask {
    /// `"nonlinear"`.
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    /// `"shg"`, `"sfg"`, `"dfg"`, `"opa"`, `"spdc"` or `"fwm"`.
    process: String,
    /// The waveguide (or crystal) the waves travel in.
    waveguide: WaveguideSpec,
    /// The pump: SHG's fundamental, SFG's first input, the others' pump.
    pump: PumpSpec,
    /// The signal: SFG's second input, the signal of DFG, OPA, SPDC (twice the pump's wavelength
    /// by default: degenerate pairs) and FWM.
    signal: Option<SignalSpec>,
    /// The grating that quasi-phase-matches a second-order process; none for birefringent or
    /// modal phase matching.
    poling: Option<PolingSpec>,
    /// The device's length, mm.
    length_mm: f64,
    /// The operating temperature, °C (25 by default).
    temperature_c: Option<f64>,
    /// The phase-matching map: against the tuned wave's wavelength, and the width or the
    /// temperature.
    map: Option<MapSpec>,
    /// SPDC's and FWM's joint spectrum.
    pairs: Option<PairsSpec>,
    /// The points along the length the efficiency is given at (101 by default).
    points: Option<usize>,
}

/// `[task.waveguide]`: where the waves' indices come from, and the guide's coefficients.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WaveguideSpec {
    /// Plane waves in a catalogue crystal.
    crystal: Option<CrystalSpec>,
    /// Effective indices given as data: one table for every wave, or one per wave.
    n_eff: Option<Vec<EffectiveSpec>>,
    /// A ridge etched into a film, its modes solved.
    ridge: Option<RidgeSpec>,
    /// The device's normalized efficiency η, %/(W·cm²), its grating's Fourier coefficient in
    /// it: a second-order process's undepleted P₃ = η P₁P₂L² sinc²(ΔkL/2). A ridge's comes
    /// from its modes' overlap when this isn't given.
    efficiency_percent_per_w_cm2: Option<f64>,
    /// FWM's nonlinear coefficient γ, 1/(W·m).
    gamma_per_w_m: Option<f64>,
    /// The propagation loss, dB/cm: one for every wave, or one per wave.
    loss_db_per_cm: Option<Losses>,
    /// The thermal expansion of the length and the grating: `"none"` (the default) or
    /// `"linbo3-jundt-1997"`.
    expansion: Option<String>,
}

/// `[task.waveguide.crystal]`: a catalogue crystal's index model.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CrystalSpec {
    /// The catalogue's id, e.g. `"linbo3"`.
    id: String,
    /// The index model's id; the entry's default when not given.
    model: Option<String>,
    /// The composition (an alloy's fraction).
    composition: Option<f64>,
    /// Each wave's axis, `"isotropic"`, `"ordinary"`, `"extraordinary"`, `"x"`, `"y"` or `"z"`:
    /// one per wave, or one for all.
    axes: Vec<String>,
}

/// `[[task.waveguide.n_eff]]`: a mode's effective indices over wavelength.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EffectiveSpec {
    /// The wavelengths, µm, increasing.
    wavelengths_um: Vec<f64>,
    /// n_eff at each.
    n_eff: Vec<f64>,
}

/// `[task.waveguide.ridge]`: a film of a stack etched to a ridge.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RidgeSpec {
    /// The stack, as a `[task.stack]` table: a substrate, a cladding and layers.
    #[schemars(with = "crate::job::schema::Layers")]
    stack: toml::Table,
    /// The layer etched: the film. It must be the top layer; the medium below it is the layer
    /// under it (or the substrate), the one above and beside the ridge its background, which
    /// must be the stack's cladding.
    layer: String,
    /// The ridge's top width, µm.
    width_um: f64,
    /// The etch's depth, µm: the film's thickness (a strip) by default.
    etch_um: Option<f64>,
    /// The sidewalls' angle from the plane, degrees (90 by default).
    sidewall_deg: Option<f64>,
    /// The chip's axis the guide runs along, `"x"` (the default) or `"y"`.
    along: Option<String>,
    /// The grid's finest spacing, nm (20 by default).
    step_nm: Option<f64>,
    /// Each wave's mode family, `"te"` or `"tm"`: one per wave, or one for all (TE by default).
    #[serde(default)]
    families: Vec<String>,
    /// How many of the highest modes the fundamental of a family is picked from (4).
    modes: Option<usize>,
    /// The tolerance of each mode's index interpolant, in n_eff (1e-8).
    tolerance: Option<f64>,
    /// Whether to solve half the window behind a mirror wall (false): only for a symmetric
    /// ridge whose crystal's axes lie along the cross-section's.
    mirror: Option<bool>,
    /// Whether Kleinman's symmetry may complete the film's d tensor where its source leaves an
    /// element out (d₁₅ = d₃₁ in 3m, say): set it only where the symmetry holds, the waves far
    /// from the crystal's resonances (false).
    kleinman: Option<bool>,
    /// The film's d tensor in its crystal's axes, pm/V (rows 1 to 3, columns 11, 22, 33, 23, 13,
    /// 12), in place of the catalogue's: for a film whose catalogue entry has none, or leaves an
    /// element out.
    d_pm_per_v: Option<[[f64; 6]; 3]>,
}

/// One loss for every wave, or one per wave.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
enum Losses {
    One(f64),
    Each(Vec<f64>),
}

/// `[task.pump]`.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PumpSpec {
    /// The vacuum wavelength, µm.
    wavelength_um: f64,
    /// The power, mW (100 by default).
    power_mw: Option<f64>,
    /// The FWHM of a pulsed pump's spectrum, nm, for a joint spectrum (0.1 by default).
    bandwidth_nm: Option<f64>,
}

/// `[task.signal]`.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SignalSpec {
    /// The vacuum wavelength, µm.
    wavelength_um: Option<f64>,
    /// The power in, mW (1 by default).
    power_mw: Option<f64>,
}

/// `[task.poling]`: a periodically poled grating.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PolingSpec {
    /// The period, µm; designed to cancel the mismatch at the operating point when not given.
    period_um: Option<f64>,
    /// The quasi-phase-matching order (1).
    order: Option<u32>,
    /// The duty cycle, inside (0, 1) (0.5).
    duty: Option<f64>,
}

/// `[task.map]`: the phase-matching map's axes.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MapSpec {
    /// The tuned wave's wavelengths, µm: from and to.
    wavelength_um: [f64; 2],
    /// Their points (201).
    points: Option<usize>,
    /// The temperatures, °C, from and to: the map's other axis.
    temperature_c: Option<[f64; 2]>,
    /// A ridge's widths, µm, from and to: the map's other axis, each width's modes solved.
    width_um: Option<[f64; 2]>,
    /// The other axis's points (21 temperatures, 5 widths).
    across_points: Option<usize>,
}

/// `[task.pairs]`: the joint spectrum's grid.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PairsSpec {
    /// The signal's span, nm, centred on it: three wavelength acceptances by default.
    span_nm: Option<f64>,
    /// The points a side (101).
    points: Option<usize>,
    /// The phase matching, `"sinc"` (the device's own, the default) or `"gaussian"`.
    matching: Option<String>,
}

// -------------------------------------------------------------------------------------------
// The record
// -------------------------------------------------------------------------------------------

/// A nonlinear process as its run sets it up ([`Event::NonlinearSetup`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NonlinearSetup {
    /// `"shg"`, `"sfg"`, `"dfg"`, `"opa"`, `"spdc"` or `"fwm"`.
    pub process: String,
    /// What the waves' indices come from: `"crystal"`, `"n_eff"` or `"ridge"`.
    pub waveguide: String,
    /// The process and its guide in a sentence.
    pub about: String,
    /// The waves, in the process's order.
    pub waves: Vec<NonlinearWave>,
    /// The tuned wave: the map's wavelength axis and the wavelength acceptance are its.
    pub tuned: String,
    /// The length, mm.
    pub length_mm: f64,
    /// The operating temperature, °C.
    pub temperature_c: f64,
    /// Δk₀ = Σ k(high side) − Σ k(other side), without the grating, rad/µm.
    pub mismatch_per_um: f64,
    /// The coherence length π/|Δk₀|, µm.
    pub coherence_length_um: f64,
    /// The grating, if the process is poled.
    pub grating: Option<PoledGrating>,
    /// Δk = Δk₀ − K_m at the operating point, rad/µm.
    pub residual_per_um: f64,
    /// sinc²(ΔkL/2) at the operating point.
    pub response: f64,
    /// The device's normalized efficiency η used, %/(W·cm²), the grating's Fourier coefficient
    /// in it.
    pub efficiency_percent_per_w_cm2: Option<f64>,
    /// A ridge's η from its modes' overlap, unpoled (the grating's coefficient not in it).
    pub overlap_percent_per_w_cm2: Option<f64>,
    /// Where η comes from.
    pub efficiency_from: String,
    /// FWM's γ, 1/(W·m).
    pub gamma_per_w_m: Option<f64>,
    /// The FWHM acceptance in the tuned wave's wavelength, nm.
    pub wavelength_acceptance_nm: Option<Accepted>,
    /// The FWHM acceptance in temperature, K.
    pub temperature_acceptance_k: Option<Accepted>,
    /// The tuned wave's phase-matched wavelength inside the map's range, µm.
    pub matched_um: Option<f64>,
    /// What wasn't computed and why; waves outside a material's transparency window.
    pub notes: Vec<String>,
}

/// One wave of a process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NonlinearWave {
    /// Its role: `"fundamental"`, `"harmonic"`, `"pump"`, `"signal"`, `"idler"`, `"first"`,
    /// `"second"` or `"sum"`.
    pub role: String,
    /// The vacuum wavelength, µm.
    pub wavelength_um: f64,
    /// Its (effective) index at the operating point.
    pub index: f64,
    /// A ridge mode's group index.
    pub group_index: Option<f64>,
    /// A ridge mode's family, `"te"` or `"tm"`; a crystal's axis.
    pub polarization: Option<String>,
    /// Its power in, mW, if it is an input.
    pub power_mw: Option<f64>,
}

/// A grating as a run uses it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PoledGrating {
    /// The period, µm, at the expansion's reference temperature.
    pub period_um: f64,
    /// The order.
    pub order: u32,
    /// The duty cycle.
    pub duty: f64,
    /// G_m = (2/(mπ)) sin(mπD).
    pub fourier_coefficient: f64,
    /// Whether the run designed it (the task gave no period).
    pub designed: bool,
}

/// An acceptance bandwidth, two ways ([`crate::nonlinear::phase::Acceptance`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Accepted {
    /// Fejer et al.'s linear estimate, Eq. (25); or the noncritical one where the first
    /// derivative vanishes.
    pub estimate: f64,
    /// The width between the half-maximum points, found numerically, if both are inside the
    /// indices' ranges.
    pub numerical: Option<f64>,
}

/// The phase-matching map ([`Event::PhaseMap`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PhaseMap {
    /// The tuned wave, whose wavelength is the map's first axis.
    pub tuned: String,
    /// Its wavelengths, µm.
    pub wavelengths_um: Vec<f64>,
    /// The other axis: `"width_um"`, `"temperature_c"`, or none (one row at the operating point).
    pub across: Option<String>,
    /// Its values.
    pub across_values: Vec<f64>,
    /// Δk with the grating, rad/µm, row by row (a row per value across, a column per
    /// wavelength); none where an index has no value.
    pub mismatch_per_um: Vec<Option<f64>>,
    /// sinc²(ΔkL/2), the same way.
    pub response: Vec<Option<f64>>,
}

/// What a process gives along its length ([`Event::NonlinearEfficiency`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EfficiencyCurve {
    /// The lengths, mm, from 0 to the device's.
    pub lengths_mm: Vec<f64>,
    /// The curves.
    pub series: Vec<EfficiencySeries>,
}

/// One curve against the length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EfficiencySeries {
    /// What it is.
    pub label: String,
    /// Its unit.
    pub unit: String,
    /// Its values at the lengths; none where it has none (a gain in dB at 0).
    pub values: Vec<Option<f64>>,
}

/// A joint spectral amplitude ([`Event::PairSpectrum`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PairSpectrum {
    /// The signal's vacuum wavelengths, µm (decreasing: the grid is even in frequency).
    pub signal_um: Vec<f64>,
    /// The idler's.
    pub idler_um: Vec<f64>,
    /// |f| at the i-th signal and j-th idler wavelength, at i × (the idler's count) + j,
    /// scaled to a peak of 1.
    pub amplitude: Vec<f64>,
    /// The pairs' purity Σλ_k².
    pub purity: f64,
    /// The Schmidt number 1/purity.
    pub schmidt_number: f64,
    /// The pump's FWHM, nm.
    pub pump_bandwidth_nm: f64,
    /// `"sinc"` or `"gaussian"`.
    pub matching: String,
}

// -------------------------------------------------------------------------------------------
// The processes
// -------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Shg,
    Sfg,
    Dfg,
    Opa,
    Spdc,
    Fwm,
}

impl Kind {
    fn parse(name: &str) -> Result<Kind> {
        Ok(match name {
            "shg" => Kind::Shg,
            "sfg" => Kind::Sfg,
            "dfg" => Kind::Dfg,
            "opa" => Kind::Opa,
            "spdc" => Kind::Spdc,
            "fwm" => Kind::Fwm,
            other => {
                return Err(task_error(format!(
                    "unknown process \"{other}\": shg, sfg, dfg, opa, spdc or fwm"
                )));
            }
        })
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Shg => "shg",
            Kind::Sfg => "sfg",
            Kind::Dfg => "dfg",
            Kind::Opa => "opa",
            Kind::Spdc => "spdc",
            Kind::Fwm => "fwm",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Kind::Shg => "second-harmonic generation",
            Kind::Sfg => "sum-frequency generation",
            Kind::Dfg => "difference-frequency generation",
            Kind::Opa => "optical parametric amplification",
            Kind::Spdc => "spontaneous parametric down-conversion",
            Kind::Fwm => "four-wave mixing",
        }
    }

    fn roles(self) -> &'static [&'static str] {
        match self {
            Kind::Shg => &["fundamental", "harmonic"],
            Kind::Sfg => &["first", "second", "sum"],
            _ => &["pump", "signal", "idler"],
        }
    }

    /// The wave the map and the wavelength acceptance are in: SHG's fundamental, the others'
    /// second wave (the signal), the pump held.
    fn tuned(self) -> usize {
        usize::from(self != Kind::Shg)
    }

    /// Each wave's wavelength (µm) with the pump at `pump` and the tuned wave at `tuned`.
    fn wavelengths(self, pump: f64, tuned: f64) -> Result<Vec<f64>> {
        let rest = |nu: f64, what: &str| -> Result<f64> {
            if nu > 0.0 && nu.is_finite() {
                Ok(1.0 / nu)
            } else {
                Err(task_error(format!(
                    "the {what}'s frequency isn't positive with the pump at {pump} um and the \
                     signal at {tuned} um"
                )))
            }
        };
        Ok(match self {
            Kind::Shg => vec![tuned, tuned / 2.0],
            Kind::Sfg => vec![pump, tuned, 1.0 / (1.0 / pump + 1.0 / tuned)],
            Kind::Dfg | Kind::Opa | Kind::Spdc => {
                vec![pump, tuned, rest(1.0 / pump - 1.0 / tuned, "idler")?]
            }
            Kind::Fwm => vec![pump, tuned, rest(2.0 / pump - 1.0 / tuned, "idler")?],
        })
    }

    /// The process of these waves' indices, at the wavelengths `waves` (µm, in the roles' order).
    fn process(self, indices: &[Index], waves: &[f64]) -> Result<Process> {
        let w = |k: usize| Wavelength::um(waves[k]);
        let wave = |k: usize| -> Result<Wave> { Ok(Wave::new(w(k)?, indices[k].clone())) };
        match self {
            Kind::Shg => Ok(Process::second_harmonic(
                w(0)?,
                indices[0].clone(),
                indices[0].clone(),
                indices[1].clone(),
            )),
            Kind::Sfg => Process::sum_frequency(wave(0)?, wave(1)?, indices[2].clone()),
            Kind::Dfg | Kind::Opa | Kind::Spdc => {
                Process::difference_frequency(wave(0)?, wave(1)?, indices[2].clone())
            }
            Kind::Fwm => Process::four_wave(wave(0)?, wave(0)?, wave(1)?, indices[2].clone()),
        }
    }
}

// -------------------------------------------------------------------------------------------
// The guide
// -------------------------------------------------------------------------------------------

/// A ridge as a task gives it, its stack read.
#[derive(Clone, Debug)]
struct RidgeGuide {
    ridge: Ridge,
    grid: Grid,
    families: Vec<Family>,
    candidates: usize,
    tolerance: f64,
    /// The film's catalogue id, its orientation and conditions, if it is a catalogue crystal.
    film: Option<(String, Option<Orientation>, Conditions)>,
    /// Whether Kleinman's symmetry may complete the film's d tensor.
    kleinman: bool,
    /// The film's d tensor as the task gives it, in its crystal's axes.
    d: Option<SecondOrder>,
}

/// Where the waves' indices come from.
#[derive(Clone, Debug)]
enum Guide {
    Crystal {
        indices: Vec<Index>,
        axes: Vec<String>,
        id: String,
        composition: Option<f64>,
    },
    Effective(Vec<Index>),
    Ridge(Box<RidgeGuide>),
}

/// One per wave, or one for all.
fn per_wave<T: Clone>(values: &[T], waves: usize, what: &str) -> Result<Vec<T>> {
    match values.len() {
        1 => Ok(vec![values[0].clone(); waves]),
        n if n == waves => Ok(values.to_vec()),
        n => Err(task_error(format!(
            "[task.waveguide] {what}: give one, or one per wave ({waves}), not {n}"
        ))),
    }
}

fn axis(name: &str) -> Result<Axis> {
    Ok(match name {
        "isotropic" => Axis::Isotropic,
        "ordinary" | "o" => Axis::Ordinary,
        "extraordinary" | "e" => Axis::Extraordinary,
        "x" => Axis::X,
        "y" => Axis::Y,
        "z" => Axis::Z,
        other => {
            return Err(task_error(format!(
                "[task.waveguide.crystal] unknown axis \"{other}\": isotropic, ordinary, \
                 extraordinary, x, y or z"
            )));
        }
    })
}

fn family(name: &str) -> Result<Family> {
    match name {
        "te" => Ok(Family::Te),
        "tm" => Ok(Family::Tm),
        other => Err(task_error(format!(
            "[task.waveguide.ridge] unknown family \"{other}\": te or tm"
        ))),
    }
}

fn family_name(f: Family) -> &'static str {
    if f == Family::Te { "te" } else { "tm" }
}

/// The catalogue id, orientation and conditions of a stack medium's value, if it is a
/// catalogue entry's.
fn catalogued(
    v: &toml::Value,
    at: &str,
) -> Result<Option<(String, Option<Orientation>, Conditions)>> {
    let number = |t: &toml::Table, key: &str| -> Option<f64> {
        t.get(key)
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    };
    Ok(match v {
        toml::Value::String(id) => Some((id.clone(), None, Conditions::default())),
        toml::Value::Table(t) => {
            let Some(id) = t.get("id").and_then(toml::Value::as_str) else {
                return Ok(None);
            };
            let orientation = match t.get("orientation") {
                Some(o) => Some(Orientation::from_value(o, &format!("{at}, orientation"))?),
                None => None,
            };
            Some((
                id.to_owned(),
                orientation,
                Conditions {
                    temperature: number(t, "temperature"),
                    composition: number(t, "composition"),
                },
            ))
        }
        _ => None,
    })
}

impl RidgeGuide {
    fn of(spec: &RidgeSpec, waves: usize) -> Result<RidgeGuide> {
        let at = "[task.waveguide.ridge]";
        let stack = crate::device::spec::task_stack(&spec.stack).map_err(|e| match e {
            Error::InvalidValue { reason, .. } => task_error(format!("{at} stack: {reason}")),
            Error::Parse { reason, .. } => task_error(format!("{at} stack: {reason}")),
            other => other,
        })?;
        let layers = stack.layers();
        let k = layers
            .iter()
            .position(|l| l.name == spec.layer)
            .ok_or_else(|| {
                task_error(format!(
                    "{at} layer \"{}\" isn't one of the stack's: {}",
                    spec.layer,
                    layers
                        .iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;
        if k + 1 != layers.len() {
            return Err(task_error(format!(
                "{at} the film \"{}\" must be the stack's top layer: a ridge has a film, the \
                 medium under it and one cladding",
                spec.layer
            )));
        }
        let film = &layers[k];
        if film.background.name() != stack.cladding().name() {
            return Err(task_error(format!(
                "{at} beside the ridge and above it is one medium: the film's background ({}) \
                 and the stack's cladding ({}) differ",
                film.background.name(),
                stack.cladding().name()
            )));
        }
        let under: Material = if k == 0 {
            stack.substrate().clone()
        } else {
            layers[k - 1].material.clone()
        };
        let thickness = film.thickness.to_um();
        let along = match spec.along.as_deref().unwrap_or("x") {
            "x" => Along::X,
            "y" => Along::Y,
            other => {
                return Err(task_error(format!(
                    "{at} along \"{other}\": the chip's x or y"
                )));
            }
        };
        let ridge = Ridge {
            film: thickness,
            etch: spec.etch_um.unwrap_or(thickness),
            width: spec.width_um,
            angle: spec.sidewall_deg.unwrap_or(90.0),
            core: film.material.clone(),
            substrate: under,
            cladding: stack.cladding().clone(),
            along,
        };
        let step = spec.step_nm.unwrap_or(20.0);
        if !(step.is_finite() && step > 0.0) {
            return Err(task_error(format!(
                "{at} step_nm must be positive, not {step}"
            )));
        }
        let mut grid = Grid::new(step / 1000.0);
        if spec.mirror.unwrap_or(false) {
            grid = grid.halved();
        }
        let families = if spec.families.is_empty() {
            vec![Family::Te; waves]
        } else {
            per_wave(&spec.families, waves, "families")?
                .iter()
                .map(|f| family(f))
                .collect::<Result<_>>()?
        };
        let candidates = spec.modes.unwrap_or(4);
        if !(1..=20).contains(&candidates) {
            return Err(task_error(format!(
                "{at} modes must be 1 to 20, not {candidates}"
            )));
        }
        let tolerance = spec.tolerance.unwrap_or(1e-8);
        if !(tolerance.is_finite() && tolerance > 0.0) {
            return Err(task_error(format!(
                "{at} tolerance must be positive, not {tolerance}"
            )));
        }
        let film_value = spec
            .stack
            .get("layer")
            .and_then(toml::Value::as_array)
            .and_then(|a| a.get(k))
            .and_then(|l| l.get("material"));
        let film = match film_value {
            Some(v) => catalogued(v, &format!("{at} stack's {}", spec.layer))?,
            None => None,
        };
        Ok(RidgeGuide {
            ridge,
            grid,
            families,
            candidates,
            tolerance,
            film,
            kleinman: spec.kleinman.unwrap_or(false),
            d: spec
                .d_pm_per_v
                .map(SecondOrder::from_voigt)
                .transpose()
                .map_err(|e| task_error(format!("{at} d_pm_per_v: {e}")))?,
        })
    }

    fn at_width(&self, width: f64) -> Ridge {
        Ridge {
            width,
            ..self.ridge.clone()
        }
    }

    /// Each wave's index over its range (`ranges`, µm), for the ridge `ridge`: one mode
    /// interpolant per family and range, the waves of one family whose ranges overlap sharing
    /// one.
    fn indices(
        &self,
        ridge: &Ridge,
        ranges: &[(f64, f64)],
    ) -> Result<(Vec<Index>, Vec<ModeIndex>)> {
        // the merged ranges of each family, and which of them each wave takes
        let mut groups: Vec<(Family, (f64, f64))> = Vec::new();
        for (&f, &(lo, hi)) in self.families.iter().zip(ranges) {
            match groups
                .iter_mut()
                .find(|(g, (a, b))| *g == f && lo <= *b && hi >= *a)
            {
                Some((_, r)) => *r = (r.0.min(lo), r.1.max(hi)),
                None => groups.push((f, (lo, hi))),
            }
        }
        // (a merge may make two groups of a family overlap: merge again until none do)
        loop {
            let mut merged = false;
            'outer: for i in 0..groups.len() {
                for j in i + 1..groups.len() {
                    let ((fi, (a, b)), (fj, (c, d))) = (groups[i], groups[j]);
                    if fi == fj && a <= d && c <= b {
                        groups[i].1 = (a.min(c), b.max(d));
                        groups.remove(j);
                        merged = true;
                        break 'outer;
                    }
                }
            }
            if !merged {
                break;
            }
        }
        let modes = groups
            .iter()
            .map(|&(f, (lo, hi))| {
                ModeIndex::of_ridge(
                    ridge,
                    &self.grid,
                    f,
                    self.candidates,
                    (lo, hi),
                    self.tolerance,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let mut indices = Vec::with_capacity(ranges.len());
        let mut of_wave = Vec::with_capacity(ranges.len());
        for (&f, &(lo, hi)) in self.families.iter().zip(ranges) {
            let g = groups
                .iter()
                .position(|&(gf, (a, b))| gf == f && a <= lo && hi <= b)
                .unwrap_or(0);
            indices.push(modes[g].index()?);
            of_wave.push(modes[g].clone());
        }
        Ok((indices, of_wave))
    }
}

/// The task read and checked: what every part of the run is built from.
#[derive(Clone, Debug)]
struct Setup {
    kind: Kind,
    guide: Guide,
    /// Each wave's wavelength at the operating point, µm.
    waves: Vec<f64>,
    /// The inputs' powers, W: the pump's and the signal's.
    pump_w: f64,
    signal_w: f64,
    pump_bandwidth_nm: f64,
    length_um: f64,
    state: State,
    expansion: Expansion,
    poling: Option<(Option<f64>, u32, f64)>,
    /// Each wave's loss, 1/µm.
    loss: Vec<f64>,
    lossy: bool,
    efficiency: Option<f64>,
    gamma: Option<f64>,
    map: Option<Map>,
    pairs: Option<Pairs>,
    points: usize,
}

#[derive(Clone, Debug)]
struct Map {
    wavelengths: Vec<f64>,
    across: Across,
}

#[derive(Clone, Debug)]
enum Across {
    None,
    Temperature(Vec<f64>),
    Width(Vec<f64>),
}

#[derive(Clone, Copy, Debug)]
struct Pairs {
    span_nm: Option<f64>,
    points: usize,
    matching: Matching,
}

fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![a];
    }
    (0..n)
        .map(|k| a + (b - a) * k as f64 / (n - 1) as f64)
        .collect()
}

fn positive(v: f64, what: &str) -> Result<f64> {
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(task_error(format!("{what} must be positive, not {v}")))
    }
}

fn count(
    n: Option<usize>,
    default: usize,
    range: std::ops::RangeInclusive<usize>,
    what: &str,
) -> Result<usize> {
    let n = n.unwrap_or(default);
    if range.contains(&n) {
        Ok(n)
    } else {
        Err(task_error(format!(
            "{what} must be {} to {}, not {n}",
            range.start(),
            range.end()
        )))
    }
}

fn task_of(job: &Job) -> Result<NonlinearTask> {
    job.task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))
}

impl Setup {
    fn of(job: &Job) -> Result<Setup> {
        let task = task_of(job)?;
        let kind = Kind::parse(&task.process)?;
        let roles = kind.roles().len();
        let pump = positive(task.pump.wavelength_um, "[task.pump] wavelength_um")?;
        let signal = task.signal.as_ref().and_then(|s| s.wavelength_um);
        let tuned = match kind {
            Kind::Shg => {
                if task.signal.is_some() {
                    return Err(task_error(
                        "SHG has one input, the pump: [task.signal] is for the other processes",
                    ));
                }
                pump
            }
            Kind::Spdc => signal.unwrap_or(2.0 * pump),
            _ => signal.ok_or_else(|| {
                task_error(format!(
                    "{} needs [task.signal] with its wavelength_um",
                    kind.title()
                ))
            })?,
        };
        let tuned = positive(tuned, "[task.signal] wavelength_um")?;
        if matches!(kind, Kind::Dfg | Kind::Opa | Kind::Spdc) && tuned <= pump {
            return Err(task_error(
                "the signal must be longer than the pump: the idler is their difference",
            ));
        }
        let waves = kind.wavelengths(pump, tuned)?;
        let pump_w = positive(task.pump.power_mw.unwrap_or(100.0), "[task.pump] power_mw")? * 1e-3;
        let signal_w = match task.signal.as_ref().and_then(|s| s.power_mw) {
            Some(p) => positive(p, "[task.signal] power_mw")? * 1e-3,
            None => 1e-3,
        };
        let pump_bandwidth_nm = positive(
            task.pump.bandwidth_nm.unwrap_or(0.1),
            "[task.pump] bandwidth_nm",
        )?;
        let length_um = positive(task.length_mm, "length_mm")? * 1000.0;
        let celsius = task.temperature_c.unwrap_or(25.0);
        if !(celsius.is_finite() && celsius > -273.15) {
            return Err(task_error(format!(
                "temperature_c must be above absolute zero, not {celsius}"
            )));
        }
        let state = State::celsius(celsius);
        let g = &task.waveguide;
        let expansion = match g.expansion.as_deref().unwrap_or("none") {
            "none" => Expansion::NONE,
            "linbo3-jundt-1997" => Expansion::LINBO3_JUNDT,
            other => {
                return Err(task_error(format!(
                    "[task.waveguide] unknown expansion \"{other}\": none or linbo3-jundt-1997"
                )));
            }
        };
        let poling = match &task.poling {
            None => None,
            Some(_) if kind == Kind::Fwm => {
                return Err(task_error(
                    "four-wave mixing is a third-order process: a grating of the second-order \
                     coefficient doesn't phase-match it; leave [task.poling] out",
                ));
            }
            Some(p) => {
                let order = p.order.unwrap_or(1);
                let duty = p.duty.unwrap_or(0.5);
                let period = p
                    .period_um
                    .map(|v| positive(v, "[task.poling] period_um"))
                    .transpose()?;
                // what Grating refuses, said here
                Grating::new(period.unwrap_or(1.0), order, duty)
                    .map_err(|e| task_error(format!("[task.poling] {e}")))?;
                Some((period, order, duty))
            }
        };
        let loss_db = match &g.loss_db_per_cm {
            None => vec![0.0; roles],
            Some(Losses::One(v)) => vec![*v; roles],
            Some(Losses::Each(v)) => per_wave(v, roles, "loss_db_per_cm")?,
        };
        if loss_db.iter().any(|v| !(v.is_finite() && *v >= 0.0)) {
            return Err(task_error(
                "[task.waveguide] loss_db_per_cm must be finite and not negative",
            ));
        }
        let lossy = loss_db.iter().any(|v| *v > 0.0);
        let loss = loss_db.iter().map(|&v| loss_from_db_per_cm(v)).collect();
        let efficiency = g
            .efficiency_percent_per_w_cm2
            .map(|v| positive(v, "[task.waveguide] efficiency_percent_per_w_cm2"))
            .transpose()?;
        let gamma = g
            .gamma_per_w_m
            .map(|v| positive(v, "[task.waveguide] gamma_per_w_m"))
            .transpose()?;
        if kind == Kind::Fwm && efficiency.is_some() {
            return Err(task_error(
                "four-wave mixing takes gamma_per_w_m, not a second-order efficiency",
            ));
        }
        if kind != Kind::Fwm && gamma.is_some() {
            return Err(task_error(
                "gamma_per_w_m is four-wave mixing's; a second-order process takes \
                 efficiency_percent_per_w_cm2",
            ));
        }
        let guide = match (&g.crystal, &g.n_eff, &g.ridge) {
            (Some(c), None, None) => {
                let entry = catalogue::entry(&c.id).ok_or_else(|| {
                    task_error(format!(
                        "[task.waveguide.crystal] no material \"{}\" in the catalogue",
                        c.id
                    ))
                })?;
                let model = match &c.model {
                    None => entry.default_model(),
                    Some(id) => entry.index.iter().find(|m| m.id == *id),
                }
                .ok_or_else(|| {
                    task_error(format!(
                        "[task.waveguide.crystal] {} has no index model {}",
                        c.id,
                        c.model.as_deref().unwrap_or("")
                    ))
                })?;
                if c.axes.is_empty() {
                    return Err(task_error(
                        "[task.waveguide.crystal] needs axes: one per wave, or one for all",
                    ));
                }
                let axes = per_wave(&c.axes, roles, "crystal axes")?;
                let indices = axes
                    .iter()
                    .map(|a| {
                        Index::model(model.clone(), axis(a)?, c.composition)
                            .map_err(|e| task_error(format!("[task.waveguide.crystal] {e}")))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Guide::Crystal {
                    indices,
                    axes,
                    id: c.id.clone(),
                    composition: c.composition,
                }
            }
            (None, Some(tables), None) => {
                if tables.is_empty() {
                    return Err(task_error("[task.waveguide] n_eff holds no table"));
                }
                let tables = per_wave(tables, roles, "n_eff")?;
                Guide::Effective(
                    tables
                        .into_iter()
                        .map(|t| {
                            Index::effective(t.wavelengths_um, t.n_eff)
                                .map_err(|e| task_error(format!("[task.waveguide] n_eff: {e}")))
                        })
                        .collect::<Result<_>>()?,
                )
            }
            (None, None, Some(r)) => Guide::Ridge(Box::new(RidgeGuide::of(r, roles)?)),
            _ => {
                return Err(task_error(
                    "[task.waveguide] needs one of crystal, n_eff or ridge",
                ));
            }
        };
        let map = match &task.map {
            None => None,
            Some(m) => {
                let [a, b] = m.wavelength_um;
                if !(a.is_finite() && b.is_finite() && a > 0.0 && a < b) {
                    return Err(task_error(format!(
                        "[task.map] wavelength_um must be increasing and positive, not [{a}, {b}]"
                    )));
                }
                let points = count(m.points, 201, 2..=2001, "[task.map] points")?;
                let across = match (m.temperature_c, m.width_um) {
                    (None, None) => {
                        if m.across_points.is_some() {
                            return Err(task_error(
                                "[task.map] across_points needs temperature_c or width_um",
                            ));
                        }
                        Across::None
                    }
                    (Some(_), Some(_)) => {
                        return Err(task_error(
                            "[task.map] takes temperature_c or width_um, not both",
                        ));
                    }
                    (Some([t0, t1]), None) => {
                        if !(t0.is_finite() && t1.is_finite() && t0 < t1 && t0 > -273.15) {
                            return Err(task_error(format!(
                                "[task.map] temperature_c must be increasing, not [{t0}, {t1}]"
                            )));
                        }
                        let n = count(m.across_points, 21, 2..=201, "[task.map] across_points")?;
                        Across::Temperature(linspace(t0, t1, n))
                    }
                    (None, Some([w0, w1])) => {
                        if !matches!(guide, Guide::Ridge(_)) {
                            return Err(task_error(
                                "[task.map] width_um is a ridge's: give [task.waveguide.ridge]",
                            ));
                        }
                        if !(w0.is_finite() && w1.is_finite() && w0 > 0.0 && w0 < w1) {
                            return Err(task_error(format!(
                                "[task.map] width_um must be increasing and positive, not [{w0}, {w1}]"
                            )));
                        }
                        let n = count(m.across_points, 5, 2..=21, "[task.map] across_points")?;
                        Across::Width(linspace(w0, w1, n))
                    }
                };
                // each wave's wavelength at the map's ends must exist
                kind.wavelengths(pump, a)?;
                kind.wavelengths(pump, b)?;
                Some(Map {
                    wavelengths: linspace(a, b, points),
                    across,
                })
            }
        };
        let pairs = match (&task.pairs, kind) {
            (Some(_), k) if !matches!(k, Kind::Spdc | Kind::Fwm) => {
                return Err(task_error("[task.pairs] is for photon pairs: spdc or fwm"));
            }
            (p, Kind::Spdc | Kind::Fwm) => {
                let p = p.clone().unwrap_or(PairsSpec {
                    span_nm: None,
                    points: None,
                    matching: None,
                });
                let matching = match p.matching.as_deref().unwrap_or("sinc") {
                    "sinc" => Matching::Sinc,
                    "gaussian" => Matching::Gaussian,
                    other => {
                        return Err(task_error(format!(
                            "[task.pairs] matching \"{other}\": sinc or gaussian"
                        )));
                    }
                };
                Some(Pairs {
                    span_nm: p
                        .span_nm
                        .map(|v| positive(v, "[task.pairs] span_nm"))
                        .transpose()?,
                    points: count(p.points, 101, 2..=401, "[task.pairs] points")?,
                    matching,
                })
            }
            _ => None,
        };
        let points = count(task.points, 101, 2..=2001, "points")?;
        Ok(Setup {
            kind,
            guide,
            waves,
            pump_w,
            signal_w,
            pump_bandwidth_nm,
            length_um,
            state,
            expansion,
            poling,
            loss,
            lossy,
            efficiency,
            gamma,
            map,
            pairs,
            points,
        })
    }

    /// The signal's span for the joint spectrum, µm of half width in frequency (1/µm).
    fn pair_half(&self, accepted_nm: Option<f64>) -> f64 {
        let signal = self.waves[1];
        let span_nm = self
            .pairs
            .and_then(|p| p.span_nm)
            .or_else(|| accepted_nm.map(|a| 3.0 * a))
            .unwrap_or(0.02 * signal * 1000.0)
            .min(0.4 * signal * 1000.0);
        0.5 * span_nm * 1e-3 / (signal * signal)
    }

    /// Each wave's wavelengths that the run asks its index for, as (shortest, longest), µm:
    /// the operating point, the map's ends, and the joint spectrum's corners.
    fn ranges(&self, half: f64) -> Result<Vec<(f64, f64)>> {
        let kind = self.kind;
        let pump = self.waves[0];
        let tuned = self.waves[kind.tuned()];
        let mut sets: Vec<Vec<f64>> = vec![kind.wavelengths(pump, tuned)?];
        if let Some(m) = &self.map {
            for &l in [m.wavelengths[0], m.wavelengths[m.wavelengths.len() - 1]].iter() {
                sets.push(kind.wavelengths(pump, l)?);
            }
        }
        if self.pairs.is_some() {
            let (s, i) = (1.0 / self.waves[1], 1.0 / self.waves[2]);
            for (ds, di) in [(-half, -half), (half, half), (-half, half), (half, -half)] {
                let (ns, ni) = (s + ds, i + di);
                if ns <= 0.0 || ni <= 0.0 {
                    continue;
                }
                let np = if kind == Kind::Fwm {
                    0.5 * (ns + ni)
                } else {
                    ns + ni
                };
                sets.push(vec![1.0 / np, 1.0 / ns, 1.0 / ni]);
            }
        }
        Ok((0..kind.roles().len())
            .map(|k| {
                let (lo, hi) = sets
                    .iter()
                    .map(|s| s[k])
                    .fold((f64::INFINITY, 0.0_f64), |(a, b), v| (a.min(v), b.max(v)));
                // at least 1 % wide, and a little beyond what is asked
                let mid = 0.5 * (lo + hi);
                let half = (0.5 * (hi - lo)).max(0.005 * mid) * 1.02;
                (mid - half, mid + half)
            })
            .collect())
    }
}

/// The guide's indices at the operating point's width (or `width`), and a ridge's mode
/// interpolants.
fn indices_at(
    setup: &Setup,
    width: Option<f64>,
    ranges: &[(f64, f64)],
) -> Result<(Vec<Index>, Vec<ModeIndex>)> {
    match &setup.guide {
        Guide::Crystal { indices, .. } => Ok((indices.clone(), Vec::new())),
        Guide::Effective(indices) => Ok((indices.clone(), Vec::new())),
        Guide::Ridge(r) => {
            let ridge = width.map_or_else(|| r.ridge.clone(), |w| r.at_width(w));
            r.indices(&ridge, ranges)
        }
    }
}

/// The device of these indices at the operating point, with `grating`.
fn device_of(setup: &Setup, indices: &[Index], grating: Option<Grating>) -> Result<Device> {
    let process = setup.kind.process(indices, &setup.waves)?;
    let device =
        Device::new(process, setup.length_um, setup.state)?.with_expansion(setup.expansion);
    Ok(match grating {
        Some(g) => device.with_grating(g),
        None => device,
    })
}

/// The grating the task asks for, from the mismatch Δk₀ at the operating point: its period, or
/// the one that cancels Δk₀ there; its wave vector along Δk₀.
fn grating_of(setup: &Setup, mismatch: f64) -> Result<Option<(Grating, bool)>> {
    let Some((period, order, duty)) = setup.poling else {
        return Ok(None);
    };
    let strain = 1.0 + setup.expansion.strain(setup.state.temperature);
    let sign = if mismatch < 0.0 { -1.0 } else { 1.0 };
    Ok(Some(match period {
        Some(p) => (
            Grating::for_mismatch(sign * TAU * f64::from(order) / p, order, duty)?,
            false,
        ),
        // the period at the reference temperature that, expanded to the operating one, cancels Δk₀
        None => (Grating::for_mismatch(mismatch * strain, order, duty)?, true),
    }))
}

// -------------------------------------------------------------------------------------------
// The run
// -------------------------------------------------------------------------------------------

/// What a run computes, in its record's order.
#[derive(Clone, Debug)]
struct Computed {
    setup: NonlinearSetup,
    map: Option<PhaseMap>,
    efficiency: Option<EfficiencyCurve>,
    pairs: Option<PairSpectrum>,
}

fn accepted(
    device: &Device,
    variable: Variable,
    scale: f64,
) -> std::result::Result<Accepted, String> {
    let a = device.acceptance(variable).map_err(|e| e.to_string())?;
    let estimate = if a.linear.is_finite() {
        a.linear
    } else {
        a.noncritical
    };
    Ok(Accepted {
        estimate: estimate * scale,
        numerical: a.numerical.is_finite().then_some(a.numerical * scale),
    })
}

fn ok(v: Result<f64>) -> Option<f64> {
    v.ok().filter(|x| x.is_finite())
}

/// The waves outside a material's transparency window, as notes.
fn transparency_notes(
    id: &str,
    conditions: Conditions,
    roles: &[&str],
    waves: &[f64],
) -> Vec<String> {
    let Some(t) = catalogue::transparency(id, conditions) else {
        return Vec::new();
    };
    let name = catalogue::entry(id).map_or_else(|| id.to_owned(), |e| e.name);
    let mut notes = Vec::new();
    for (role, &l) in roles.iter().zip(waves) {
        if let Some(edge) = t.band_edge.wavelength.filter(|&e| l < e) {
            notes.push(format!(
                "the {role} at {l:.4} µm lies below {name}'s band edge, {edge:.4} µm: absorbed"
            ));
        } else if let Some(edge) = t.two_photon.wavelength.filter(|&e| l < e) {
            notes.push(format!(
                "the {role} at {l:.4} µm lies below {edge:.4} µm, where two-photon absorption \
                 sets in in {name}"
            ));
        }
        if let Some(edge) = t.multiphonon.wavelength.filter(|&e| l > e) {
            notes.push(format!(
                "the {role} at {l:.4} µm lies beyond {name}'s multiphonon edge, {edge:.4} µm"
            ));
        }
    }
    notes
}

fn compute(job: &Job, stop: &Stop) -> Result<Computed> {
    let setup = Setup::of(job)?;
    let kind = setup.kind;
    let roles = kind.roles();
    let mut notes: Vec<String> = Vec::new();

    // a first pass at the operating point for the acceptance that sizes the joint spectrum
    let half_guess = setup.pair_half(None);
    let ranges = setup.ranges(half_guess)?;
    let (indices, modes) = indices_at(&setup, None, &ranges)?;
    let bare = device_of(&setup, &indices, None)?;
    let mismatch0 = bare.process().mismatch(&setup.state)?;
    let grating = grating_of(&setup, mismatch0)?;
    let device = device_of(&setup, &indices, grating.map(|g| g.0))?;
    let state = setup.state;
    let residual = device.mismatch(&state)?;
    let phase = device.phase(&state)?;

    let nm = 1000.0;
    let wavelength_acceptance = match accepted(&device, Variable::Wavelength, nm) {
        Ok(a) => Some(a),
        Err(e) => {
            notes.push(format!("no wavelength acceptance: {e}"));
            None
        }
    };
    let temperature_dependent = device
        .process()
        .waves()
        .iter()
        .any(|w| w.index.depends_on_temperature());
    let temperature_acceptance = if temperature_dependent {
        match accepted(&device, Variable::Temperature, 1.0) {
            Ok(a) => Some(a),
            Err(e) => {
                notes.push(format!("no temperature acceptance: {e}"));
                None
            }
        }
    } else {
        None
    };
    let matched = setup.map.as_ref().and_then(|m| {
        let (a, b) = (m.wavelengths[0], m.wavelengths[m.wavelengths.len() - 1]);
        device.phase_matching_wavelength(a, b).ok()
    });

    // the efficiency: given, or a ridge's overlap
    let gm = grating.map_or(1.0, |(g, _)| g.fourier_coefficient());
    let mut overlap = None;
    let (efficiency, efficiency_from) = if kind == Kind::Fwm {
        (None, "four-wave mixing: γ".to_owned())
    } else if let Some(eta) = setup.efficiency {
        (Some(eta), "given by the task".to_owned())
    } else if let Guide::Ridge(r) = &setup.guide {
        match ridge_overlap(&setup, r) {
            Ok((eta, how)) => {
                overlap = Some(eta);
                (Some(eta * gm * gm), how)
            }
            Err(e) => {
                notes.push(format!(
                    "no efficiency from the modes' overlap: {e}; give efficiency_percent_per_w_cm2"
                ));
                (None, "none".to_owned())
            }
        }
    } else {
        notes.push(
            "no efficiency: give efficiency_percent_per_w_cm2 (a crystal's or a table's waves \
             have no fields to overlap)"
                .to_owned(),
        );
        (None, "none".to_owned())
    };
    if kind == Kind::Fwm && setup.gamma.is_none() {
        notes.push("no gain: give gamma_per_w_m".to_owned());
    }
    if let Guide::Ridge(_) = &setup.guide {
        notes.push(
            "the ridge's materials are taken at their stack's conditions: the task's \
             temperature moves only the grating's expansion"
                .to_owned(),
        );
    }

    // the waves
    let polarization: Vec<Option<String>> = match &setup.guide {
        Guide::Crystal { axes, .. } => axes.iter().map(|a| Some(a.clone())).collect(),
        Guide::Ridge(r) => r
            .families
            .iter()
            .map(|f| Some(family_name(*f).to_owned()))
            .collect(),
        Guide::Effective(_) => vec![None; roles.len()],
    };
    let input_power = |k: usize| -> Option<f64> {
        match (kind, k) {
            (_, 0) => Some(setup.pump_w * 1e3),
            (Kind::Sfg | Kind::Dfg | Kind::Opa | Kind::Fwm, 1) => Some(setup.signal_w * 1e3),
            _ => None,
        }
    };
    let waves = roles
        .iter()
        .enumerate()
        .map(|(k, role)| {
            Ok(NonlinearWave {
                role: (*role).to_owned(),
                wavelength_um: setup.waves[k],
                index: indices[k].at(setup.waves[k], &state)?,
                group_index: modes.get(k).map(|m| m.group_index(setup.waves[k])),
                polarization: polarization[k].clone(),
                power_mw: input_power(k),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // the transparency of the crystal, or the ridge's film
    match &setup.guide {
        Guide::Crystal {
            id, composition, ..
        } => notes.extend(transparency_notes(
            id,
            Conditions {
                temperature: Some(state.temperature),
                composition: *composition,
            },
            roles,
            &setup.waves,
        )),
        Guide::Ridge(r) => {
            if let Some((id, _, c)) = &r.film {
                notes.extend(transparency_notes(id, *c, roles, &setup.waves));
            }
        }
        Guide::Effective(_) => {}
    }

    let about = format!(
        "{} {} over {} mm{}",
        kind.title(),
        match kind {
            Kind::Shg => format!("of {} µm", setup.waves[0]),
            Kind::Sfg => format!("of {} and {} µm", setup.waves[0], setup.waves[1]),
            _ => format!(
                "pumped at {} µm, the signal at {} µm",
                setup.waves[0], setup.waves[1]
            ),
        },
        setup.length_um / 1000.0,
        match &setup.guide {
            Guide::Crystal { id, .. } => format!(", plane waves in {id}"),
            Guide::Effective(_) => ", the effective indices given".to_owned(),
            Guide::Ridge(r) => format!(
                ", the fundamental modes of a {} µm ridge in {} µm of film",
                r.ridge.width, r.ridge.film
            ),
        }
    );

    let kappa = efficiency.map(coupling_from_efficiency);
    let efficiency_curve = efficiency_curve(&setup, &device, residual, kappa, &mut notes)?;

    let pairs = match setup.pairs {
        Some(p) => {
            // a ridge's modes were solved over the first guess's span, a crystal's or a table's
            // indices take the acceptance's
            let half = if matches!(setup.guide, Guide::Ridge(_)) {
                half_guess
            } else {
                setup.pair_half(wavelength_acceptance.as_ref().map(|a| a.estimate))
            };
            Some(pair_spectrum(&setup, &device, p, half)?)
        }
        None => None,
    };

    let map = match &setup.map {
        None => None,
        Some(m) => Some(phase_map(&setup, m, &device, grating.map(|g| g.0), stop)?),
    };

    let setup_record = NonlinearSetup {
        process: kind.name().to_owned(),
        waveguide: match setup.guide {
            Guide::Crystal { .. } => "crystal",
            Guide::Effective(_) => "n_eff",
            Guide::Ridge(_) => "ridge",
        }
        .to_owned(),
        about,
        waves,
        tuned: roles[kind.tuned()].to_owned(),
        length_mm: setup.length_um / 1000.0,
        temperature_c: state.temperature - 273.15,
        mismatch_per_um: mismatch0,
        coherence_length_um: PI / mismatch0.abs(),
        grating: grating.map(|(g, designed)| PoledGrating {
            period_um: g.period(),
            order: g.order(),
            duty: g.duty(),
            fourier_coefficient: g.fourier_coefficient(),
            designed,
        }),
        residual_per_um: residual,
        response: sinc2(phase),
        efficiency_percent_per_w_cm2: efficiency,
        overlap_percent_per_w_cm2: overlap,
        efficiency_from,
        gamma_per_w_m: setup.gamma,
        wavelength_acceptance_nm: wavelength_acceptance,
        temperature_acceptance_k: temperature_acceptance,
        matched_um: matched,
        notes,
    };
    Ok(Computed {
        setup: setup_record,
        map,
        efficiency: efficiency_curve,
        pairs,
    })
}

/// A ridge's η from its modes' overlap at the operating point, unpoled, %/(W·cm²), and what it
/// took.
fn ridge_overlap(setup: &Setup, r: &RidgeGuide) -> Result<(f64, String)> {
    if let Some(d) = r.d {
        let d = match r.film.as_ref().and_then(|f| f.1.as_ref()) {
            Some(o) => d.rotated(o),
            None => d,
        };
        let kappa = coupling(setup, r, &d.in_section(r.ridge.along))?;
        return Ok((
            percent_per_watt_cm2(kappa),
            "the modes' overlap through the d tensor the task gives, turned onto the chip"
                .to_owned(),
        ));
    }
    let Some((id, orientation, _)) = &r.film else {
        return Err(task_error(
            "the film isn't a catalogue crystal: give its d_pm_per_v",
        ));
    };
    let entry = catalogue::entry(id)
        .ok_or_else(|| task_error(format!("no material \"{id}\" in the catalogue")))?;
    let tensors: Vec<_> = entry
        .tensors
        .iter()
        .filter(|t| t.kind == TensorKind::SecondOrder)
        .collect();
    let (tensor, rest) = tensors
        .split_first()
        .ok_or_else(|| task_error(format!("{} has no d tensor in the catalogue", entry.name)))?;
    // the first tensor, its unknown elements from the others, and by Kleinman's symmetry if the
    // task says it holds
    let mut d = SecondOrder::completed(tensor, rest, r.kleinman).map_err(|e| {
        task_error(format!(
            "{e}{}",
            if r.kleinman {
                ""
            } else {
                " (set kleinman = true where Kleinman's symmetry holds, or give d_pm_per_v)"
            }
        ))
    })?;
    if let Some(o) = orientation {
        d = d.rotated(o);
    }
    let kappa = coupling(setup, r, &d.in_section(r.ridge.along))?;
    Ok((
        percent_per_watt_cm2(kappa),
        format!(
            "the modes' overlap through {}'s d tensor ({}), as the catalogue gives it, turned onto the chip",
            entry.name, tensor.label
        ),
    ))
}

/// The coupling κ of the process from the ridge's modes at the operating point, through d
/// in the guide's frame.
fn coupling(setup: &Setup, r: &RidgeGuide, d: &SecondOrder) -> Result<f64> {
    let fields = setup
        .waves
        .iter()
        .zip(&r.families)
        .map(|(&l, &f)| {
            let w = Wavelength::um(l)?;
            let cs = r.ridge.cross_section(w, &r.grid, f)?;
            let mode = fundamental(&cs, w, f, r.candidates)?;
            Ok((ModeField::new(&mode, &cs, w)?, cs))
        })
        .collect::<Result<Vec<_>>>()?;
    let share = r.ridge.film_share(&fields[0].1, |_, _| true);
    match setup.kind {
        Kind::Shg => second_harmonic_coupling(&fields[1].0, &fields[0].0, d, &share),
        Kind::Sfg => sum_frequency_coupling(&fields[2].0, &fields[0].0, &fields[1].0, d, &share),
        // the pump is the sum of the signal and the idler
        _ => sum_frequency_coupling(&fields[0].0, &fields[1].0, &fields[2].0, d, &share),
    }
}

/// sinh(gL)/g from g², continued to sin(|g|L)/|g| and L.
fn sinhc(g2: f64, length: f64) -> f64 {
    if g2 > 0.0 {
        let g = g2.sqrt();
        (g * length).sinh() / g
    } else if g2 < 0.0 {
        let g = (-g2).sqrt();
        (g * length).sin() / g
    } else {
        length
    }
}

/// The RK4 steps over the whole length.
const STEPS: usize = 2000;

fn efficiency_curve(
    setup: &Setup,
    device: &Device,
    mismatch: f64,
    kappa: Option<f64>,
    notes: &mut Vec<String>,
) -> Result<Option<EfficiencyCurve>> {
    let n = setup.points;
    let lengths: Vec<f64> = linspace(0.0, setup.length_um, n);
    let lengths_mm = lengths.iter().map(|z| z / 1000.0).collect();
    let series = |label: &str, unit: &str, values: Vec<Option<f64>>| EfficiencySeries {
        label: label.to_owned(),
        unit: unit.to_owned(),
        values,
    };
    let per_segment = STEPS.div_ceil(n - 1).max(1);
    let segment = setup.length_um / (n - 1) as f64;
    let (p0, ps) = (setup.pump_w, setup.signal_w);
    let nu: Vec<f64> = setup.waves.iter().map(|l| 1.0 / l).collect();
    let mw = |w: f64| Some(w * 1e3);
    let db = |g: f64| (g > 0.0).then(|| 10.0 * g.log10());
    let lossless =
        |s: EfficiencySeries| -> Option<EfficiencySeries> { (!setup.lossy).then_some(s) };
    let out = match setup.kind {
        Kind::Fwm => {
            let Some(gamma) = setup.gamma else {
                return Ok(None);
            };
            if setup.lossy {
                notes.push(
                    "four-wave mixing's gain is lossless, in closed form: its losses aren't in it"
                        .to_owned(),
                );
            }
            let dk = device.process().mismatch(&setup.state)? * 1e6;
            let gain: Vec<Option<f64>> = lengths
                .iter()
                .map(|z| db(fwm::signal_gain(gamma, p0, dk, z * 1e-6)))
                .collect();
            let conversion: Vec<Option<f64>> = lengths
                .iter()
                .map(|z| db(fwm::conversion(gamma, p0, dk, z * 1e-6)))
                .collect();
            vec![
                series("signal gain (undepleted pump, closed form)", "dB", gain),
                series(
                    "idler conversion (undepleted pump, closed form)",
                    "dB",
                    conversion,
                ),
            ]
        }
        _ => {
            let Some(kappa) = kappa else {
                return Ok(None);
            };
            let i = c64::new(0.0, 1.0);
            let rotate = |a: c64, z0: f64, s: f64| a * (i * (s * mismatch * z0)).exp();
            match setup.kind {
                Kind::Shg => {
                    let shg = SecondHarmonic {
                        coupling: kappa,
                        mismatch,
                        loss: [setup.loss[0], setup.loss[1]],
                    };
                    let mut a = [c64::new(p0.sqrt(), 0.0), c64::new(0.0, 0.0)];
                    let mut depleted = vec![Some(0.0)];
                    for k in 1..n {
                        let z0 = lengths[k - 1];
                        let b =
                            shg.propagate([a[0], rotate(a[1], z0, 1.0)], segment, per_segment)?;
                        a = [b[0], rotate(b[1], z0, -1.0)];
                        depleted.push(mw(a[1].norm_sqr()));
                    }
                    let undepleted = lengths
                        .iter()
                        .map(|&z| {
                            if z > 0.0 {
                                mw(undepleted_lossy_shg(&shg, p0, z))
                            } else {
                                Some(0.0)
                            }
                        })
                        .collect();
                    let conversion = depleted
                        .iter()
                        .map(|p| p.map(|p| 100.0 * p * 1e-3 / p0))
                        .collect();
                    vec![
                        series("harmonic (depleted, RK4)", "mW", depleted),
                        series("harmonic (undepleted, closed form)", "mW", undepleted),
                        series("conversion", "%", conversion),
                    ]
                }
                Kind::Sfg | Kind::Dfg | Kind::Opa | Kind::Spdc => {
                    // ThreeWave's waves: SFG's first, second and sum; DFG's signal, idler and pump
                    let sum = setup.kind == Kind::Sfg;
                    let (f1, l) = if sum {
                        (
                            [nu[0], nu[1]],
                            [setup.loss[0], setup.loss[1], setup.loss[2]],
                        )
                    } else {
                        (
                            [nu[1], nu[2]],
                            [setup.loss[1], setup.loss[2], setup.loss[0]],
                        )
                    };
                    let three = ThreeWave {
                        frequencies: f1,
                        coupling: kappa,
                        mismatch,
                        loss: l,
                    };
                    let nu3 = f1[0] + f1[1];
                    let sinc2z = |z: f64| sinc2(0.5 * mismatch * z);
                    if setup.kind == Kind::Spdc {
                        let p_mw = p0 * 1e3;
                        let bright = lengths
                            .iter()
                            .map(|&z| {
                                Some(
                                    pair_rate_density(
                                        kappa,
                                        f1[0],
                                        f1[1],
                                        p0,
                                        z,
                                        0.5 * mismatch * z,
                                    ) * 1e9
                                        / p_mw,
                                )
                            })
                            .collect();
                        if setup.lossy {
                            notes.push(
                                "the pairs' brightness is lossless: their losses aren't in it"
                                    .to_owned(),
                            );
                        }
                        vec![series(
                            "spectral brightness at the signal's centre (undepleted)",
                            "pairs/(s·GHz·mW)",
                            bright,
                        )]
                    } else {
                        let start = if sum {
                            [
                                c64::new(p0.sqrt(), 0.0),
                                c64::new(ps.sqrt(), 0.0),
                                c64::new(0.0, 0.0),
                            ]
                        } else {
                            [
                                c64::new(ps.sqrt(), 0.0),
                                c64::new(0.0, 0.0),
                                c64::new(p0.sqrt(), 0.0),
                            ]
                        };
                        let mut a = start;
                        let mut path = vec![a];
                        for k in 1..n {
                            let z0 = lengths[k - 1];
                            let b = three.propagate(
                                [a[0], a[1], rotate(a[2], z0, 1.0)],
                                segment,
                                per_segment,
                            )?;
                            a = [b[0], b[1], rotate(b[2], z0, -1.0)];
                            path.push(a);
                        }
                        let power = |j: usize| -> Vec<Option<f64>> {
                            path.iter().map(|a| mw(a[j].norm_sqr())).collect()
                        };
                        match setup.kind {
                            Kind::Sfg => {
                                let closed = lengths
                                    .iter()
                                    .map(|&z| mw(kappa * kappa * p0 * ps * z * z * sinc2z(z)))
                                    .collect();
                                [
                                    Some(series("sum (depleted, RK4)", "mW", power(2))),
                                    lossless(series(
                                        "sum (undepleted, lossless closed form)",
                                        "mW",
                                        closed,
                                    )),
                                ]
                                .into_iter()
                                .flatten()
                                .collect()
                            }
                            Kind::Dfg => {
                                let r = f1[1] / nu3;
                                let closed = lengths
                                    .iter()
                                    .map(|&z| {
                                        mw(kappa * kappa * r * r * p0 * ps * z * z * sinc2z(z))
                                    })
                                    .collect();
                                [
                                    Some(series("idler (depleted, RK4)", "mW", power(1))),
                                    lossless(series(
                                        "idler (undepleted, lossless closed form)",
                                        "mW",
                                        closed,
                                    )),
                                    Some(series("signal (depleted, RK4)", "mW", power(0))),
                                ]
                                .into_iter()
                                .flatten()
                                .collect()
                            }
                            _ => {
                                // OPA: the signal's gain, and the undepleted closed form
                                let gain: Vec<Option<f64>> =
                                    path.iter().map(|a| db(a[0].norm_sqr() / ps)).collect();
                                let big = kappa * kappa * f1[0] * f1[1] / (nu3 * nu3) * p0;
                                let g2 = big - 0.25 * mismatch * mismatch;
                                let closed = lengths
                                    .iter()
                                    .map(|&z| db(1.0 + big * sinhc(g2, z).powi(2)))
                                    .collect();
                                [
                                    Some(series("signal gain (depleted, RK4)", "dB", gain)),
                                    lossless(series(
                                        "signal gain (undepleted, lossless closed form)",
                                        "dB",
                                        closed,
                                    )),
                                    Some(series("idler (depleted, RK4)", "mW", power(1))),
                                ]
                                .into_iter()
                                .flatten()
                                .collect()
                            }
                        }
                    }
                }
                Kind::Fwm => unreachable!("handled above"),
            }
        }
    };
    Ok(Some(EfficiencyCurve {
        lengths_mm,
        series: out,
    }))
}

fn pair_spectrum(setup: &Setup, device: &Device, p: Pairs, half: f64) -> Result<PairSpectrum> {
    let pump = setup.waves[0];
    let bandwidth = setup.pump_bandwidth_nm * 1e-3 / (pump * pump);
    let mut spectrum = Pump {
        center: 1.0 / pump,
        bandwidth,
    };
    if setup.kind == Kind::Fwm {
        spectrum = spectrum.two_photon();
    }
    let jsa = pairs::joint_spectrum(device, spectrum, p.matching, half, p.points)?;
    let schmidt = jsa.schmidt()?;
    let magnitudes: Vec<f64> = jsa.amplitude.iter().map(|z| z.norm()).collect();
    let peak = magnitudes.iter().copied().fold(0.0, f64::max);
    Ok(PairSpectrum {
        signal_um: jsa.signal.iter().map(|nu| 1.0 / nu).collect(),
        idler_um: jsa.idler.iter().map(|nu| 1.0 / nu).collect(),
        amplitude: magnitudes
            .iter()
            .map(|m| if peak > 0.0 { m / peak } else { 0.0 })
            .collect(),
        purity: schmidt.purity(),
        schmidt_number: schmidt.number(),
        pump_bandwidth_nm: setup.pump_bandwidth_nm,
        matching: match p.matching {
            Matching::Gaussian => "gaussian",
            _ => "sinc",
        }
        .to_owned(),
    })
}

fn phase_map(
    setup: &Setup,
    m: &Map,
    device: &Device,
    grating: Option<Grating>,
    stop: &Stop,
) -> Result<PhaseMap> {
    let length = setup.length_um;
    let row = |device: &Device, state: &State| -> (Vec<Option<f64>>, Vec<Option<f64>>) {
        let s = 1.0 + device.expansion().strain(state.temperature);
        m.wavelengths
            .iter()
            .map(|&l| {
                let phase = ok(device.phase_tuned(l, state));
                (phase.map(|p| 2.0 * p / (length * s)), phase.map(sinc2))
            })
            .unzip()
    };
    let mut mismatch = Vec::new();
    let mut response = Vec::new();
    let (across, values) = match &m.across {
        Across::None => {
            let (a, b) = row(device, &setup.state);
            mismatch.extend(a);
            response.extend(b);
            (None, Vec::new())
        }
        Across::Temperature(ts) => {
            for &t in ts {
                let state = State::celsius(t);
                let (a, b) = row(device, &state);
                mismatch.extend(a);
                response.extend(b);
            }
            (Some("temperature_c".to_owned()), ts.clone())
        }
        Across::Width(ws) => {
            let ranges = setup.ranges(setup.pair_half(None))?;
            let mut done = Vec::new();
            for &w in ws {
                if stop.reason().is_some() {
                    break;
                }
                let (indices, _) = indices_at(setup, Some(w), &ranges)?;
                let device = device_of(setup, &indices, grating)?;
                let (a, b) = row(&device, &setup.state);
                mismatch.extend(a);
                response.extend(b);
                done.push(w);
            }
            (Some("width_um".to_owned()), done)
        }
    };
    Ok(PhaseMap {
        tuned: setup.kind.roles()[setup.kind.tuned()].to_owned(),
        wavelengths_um: m.wavelengths.clone(),
        across,
        across_values: values,
        mismatch_per_um: mismatch,
        response,
    })
}

// -------------------------------------------------------------------------------------------
// The files
// -------------------------------------------------------------------------------------------

fn num(v: f64) -> String {
    format!("{v:?}")
}

fn opt(v: Option<f64>) -> String {
    v.map(num).unwrap_or_default()
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// The run's numbers beside its record: `nonlinear.json` and its tables as CSV.
fn write_files(dir: &Path, c: &Computed) -> Result<()> {
    let io = |path: &Path, e: std::io::Error| Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    };
    let write = |name: &str, text: String| -> Result<()> {
        let path = dir.join(name);
        std::fs::write(&path, text).map_err(|e| io(&path, e))
    };
    #[derive(Serialize)]
    struct File<'a> {
        setup: &'a NonlinearSetup,
        phase_map: &'a Option<PhaseMap>,
        efficiency: &'a Option<EfficiencyCurve>,
        pair_spectrum: &'a Option<PairSpectrum>,
    }
    let json = serde_json::to_string_pretty(&File {
        setup: &c.setup,
        phase_map: &c.map,
        efficiency: &c.efficiency,
        pair_spectrum: &c.pairs,
    })
    .map_err(|e| Error::Parse {
        what: "nonlinear.json".into(),
        reason: e.to_string(),
    })?;
    write("nonlinear.json", json + "\n")?;
    if let Some(m) = &c.map {
        let mut text = String::new();
        let across = m.across.clone().unwrap_or_default();
        if across.is_empty() {
            text.push_str("wavelength_um,mismatch_rad_per_um,response\n");
        } else {
            let _ = writeln!(text, "{across},wavelength_um,mismatch_rad_per_um,response");
        }
        let n = m.wavelengths_um.len();
        let rows = m.mismatch_per_um.len() / n.max(1);
        for r in 0..rows {
            for (j, &l) in m.wavelengths_um.iter().enumerate() {
                let k = r * n + j;
                if !across.is_empty() {
                    let _ = write!(text, "{},", num(m.across_values[r]));
                }
                let _ = writeln!(
                    text,
                    "{},{},{}",
                    num(l),
                    opt(m.mismatch_per_um[k]),
                    opt(m.response[k])
                );
            }
        }
        write("phase_map.csv", text)?;
    }
    if let Some(e) = &c.efficiency {
        let mut text = String::from("length_mm");
        for s in &e.series {
            let _ = write!(text, ",{}", csv_field(&format!("{} ({})", s.label, s.unit)));
        }
        text.push('\n');
        for (k, &z) in e.lengths_mm.iter().enumerate() {
            text.push_str(&num(z));
            for s in &e.series {
                let _ = write!(text, ",{}", opt(s.values[k]));
            }
            text.push('\n');
        }
        write("efficiency.csv", text)?;
    }
    if let Some(p) = &c.pairs {
        let mut text = String::from("signal_um,idler_um,amplitude\n");
        let n = p.idler_um.len();
        for (i, &s) in p.signal_um.iter().enumerate() {
            for (j, &l) in p.idler_um.iter().enumerate() {
                let _ = writeln!(
                    text,
                    "{},{},{}",
                    num(s),
                    num(l),
                    num(p.amplitude[i * n + j])
                );
            }
        }
        write("pair_spectrum.csv", text)?;
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------
// The kind
// -------------------------------------------------------------------------------------------

/// The `"nonlinear"` kind: a second- or third-order process in a guide or a crystal.
pub(super) struct NonlinearKind;

impl TaskKind for NonlinearKind {
    fn name(&self) -> &'static str {
        "nonlinear"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        super::schema::task::<NonlinearTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        let setup = Setup::of(job)?;
        match &setup.guide {
            // the cross-sections drawn at each wave's wavelength, without solving them
            Guide::Ridge(r) => {
                for (&l, &f) in setup.waves.iter().zip(&r.families) {
                    r.ridge
                        .cross_section(Wavelength::um(l)?, &r.grid, f)
                        .map_err(|e| task_error(format!("[task.waveguide.ridge] {e}")))?;
                }
                Ok(())
            }
            // a crystal's or a table's process built at the operating point
            _ => {
                let ranges = setup.ranges(setup.pair_half(None))?;
                let (indices, _) = indices_at(&setup, None, &ranges)?;
                let device = device_of(&setup, &indices, None)?;
                device
                    .process()
                    .mismatch(&setup.state)
                    .map(|_| ())
                    .map_err(|e| task_error(e.to_string()))
            }
        }
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        self.check(job)?;
        if matches!(Setup::of(job)?.guide, Guide::Ridge(_)) {
            return Err(task_error(
                "a ridge's process is set up from its modes, which its run solves",
            ));
        }
        let c = compute(job, &Stop::new(None))?;
        Ok(Event::NonlinearSetup {
            setup: Box::new(c.setup),
        })
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        solver_run(job, run, stop, self.name(), |job, run, stop| {
            self.check(job)?;
            let c = compute(job, stop)?;
            run.record(&Event::NonlinearSetup {
                setup: Box::new(c.setup.clone()),
            })?;
            if let Some(m) = &c.map {
                run.record(&Event::PhaseMap {
                    map: Box::new(m.clone()),
                })?;
            }
            if let Some(e) = &c.efficiency {
                run.record(&Event::NonlinearEfficiency {
                    curve: Box::new(e.clone()),
                })?;
            }
            if let Some(p) = &c.pairs {
                run.record(&Event::PairSpectrum {
                    spectrum: Box::new(p.clone()),
                })?;
            }
            if !run.dir().as_os_str().is_empty() {
                write_files(run.dir(), &c)?;
            }
            Ok(())
        })
    }

    fn farms(&self, _: &Job) -> bool {
        false
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        ("a nonlinear job", &[])
    }
}

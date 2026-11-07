//! Photons through a chip, as a job: a task of kind `"quantum"` beside a `[circuit]`
//! ([`crate::quantum`]), the studio's quantum view of the Chip page run from the command line
//! and recorded.
//!
//! At one wavelength, through the chip's S-matrix with its loss dilated to loss modes
//! ([`Transfer`]), the task computes any of:
//!
//! - **statistics** of the photons on the ports (`[[task.photons]]`): every pattern at the ports
//!   for photons alike and told apart, the share where every photon arrives, bunching, and each
//!   port's mean ([`Event::QuantumStatistics`]);
//! - **samples** of them by the exact sampler (`[task.sample]`), counted against the exact
//!   distribution: the total variation distance and what a correct sampler is expected to reach,
//!   ½ Σ √(2p(1 − p)/(πN)) ([`Event::QuantumSamples`]);
//! - **a Hong–Ou–Mandel dip** (`[task.dip]`): two photons with Gaussian spectra through the
//!   chip's S(ω) over a band, the coincidences between two ports against the delay
//!   ([`Event::QuantumDip`]);
//! - **a loss budget** (`[[task.budget]]`): each photon's best route between two ports, stage by
//!   stage, the n-photon event and the published thresholds ([`Event::QuantumBudget`]);
//! - **squeezed light** (`[[task.squeezed]]`): squeezed, displaced vacuum on ports through the
//!   chip, each port's squeezing after it and the state's purity ([`Event::QuantumSqueezed`]).
//!
//! ```yaml
//! name: coupler-quantum
//! timeout_minutes: 1
//! circuit:                         # a chip, as a "spectrum" job's
//!   connections: []
//!   instance:
//!     - { name: x, component: coupler, values: { coupling: 0.5 } }
//!   port:
//!     - { name: in1, at: x.o1 }
//!     - { name: in2, at: x.o2 }
//!     - { name: out1, at: x.o3 }
//!     - { name: out2, at: x.o4 }
//! task:
//!   kind: quantum
//!   wavelength_um: 1.55
//!   photons: [{ port: in1, count: 1 }, { port: in2, count: 1 }]
//!   sample: { samples: 10000, seed: 1 }
//!   dip:
//!     inputs: [in1, in2]
//!     outputs: [out1, out2]
//!     center_um: [1.55, 1.55]
//!     fwhm_nm: [1.0, 1.0]
//!     from_um: 1.5
//!     to_um: 1.6
//!     points: 1001
//!   budget: [{ from: in1, to: out1 }]
//!   squeezed: [{ port: in1, squeezing_db: 6.0 }]
//! ```

use std::collections::BTreeMap;
use std::f64::consts::PI;

use num_complex::Complex64 as c64;
use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::chip::{Assembled, CHIP_EXPORTS, ChipDesign, chip_job, chip_of};
use super::kind::TaskKind;
use super::{Event, reason, task_error};
use crate::circuit::Spectrum;
use crate::quantum::{
    self, Budget, Distribution, Fock, Gaussian, Photons, Threshold, Transfer, Wavepacket,
};
use crate::run::{Job, Run, Stop};
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

/// The most photons on the ports: each pattern is a permanent of that size.
pub const MOST_PHOTONS: usize = 8;
/// The most external ports.
pub const MOST_PORTS: usize = 16;
/// The most patterns over the ports and the loss modes.
pub const MOST_PATTERNS: usize = 100_000;
/// The most samples a job draws.
pub const MOST_SAMPLES: usize = 1_000_000;
/// The most delays of a dip.
pub const MOST_DELAYS: usize = 2001;
/// The fewest steps of the band across a photon's width: fewer don't resolve its spectrum.
pub const POINTS_PER_WIDTH: f64 = 4.0;
/// The most routes looked at for a photon of a budget.
pub const MOST_ROUTES: usize = 32;
/// The most squeezing on a port, dB.
pub const MOST_SQUEEZING_DB: f64 = 20.0;

// ---- the task ----

/// A `"quantum"` task: its wavelength, and what it computes there.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct QuantumTask {
    /// `"quantum"`.
    #[allow(dead_code)] // looked up by the registry; kept for serde and the schema
    kind: String,
    /// The wavelength of the statistics, the samples, the budget and the squeezed light, µm.
    wavelength_um: f64,
    /// Photons on the external ports: their statistics, and their samples.
    #[serde(default)]
    photons: Vec<PhotonsOn>,
    /// Samples of those photons, counted against their exact distribution.
    sample: Option<SampleTask>,
    /// A Hong–Ou–Mandel dip through the chip's S(ω).
    dip: Option<DipTask>,
    /// A loss budget: each photon's best route from a port to another.
    #[serde(default)]
    budget: Vec<LegTask>,
    /// Squeezed light on ports.
    #[serde(default)]
    squeezed: Vec<SqueezedTask>,
    /// What the run writes beside its record: none of a chip's.
    #[allow(dead_code)] // read by super::export_of, checked by serde
    export: Option<super::ExportSpec>,
}

/// Photons on a port.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotonsOn {
    /// The port.
    pub port: String,
    /// How many photons.
    pub count: usize,
}

/// Samples of the photons.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SampleTask {
    /// How many.
    pub samples: usize,
    /// The seed: the same seed draws the same samples.
    pub seed: u64,
}

/// A Hong–Ou–Mandel dip: photon 1 into `inputs[0]`, delayed, photon 2 into `inputs[1]`, the
/// coincidences between `outputs`.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DipTask {
    /// The ports the photons enter by.
    pub inputs: [String; 2],
    /// The ports whose coincidences are counted.
    pub outputs: [String; 2],
    /// Each photon's centre, µm.
    pub center_um: [f64; 2],
    /// The FWHM of each photon's power spectrum, nm (a transform-limited Gaussian).
    pub fwhm_nm: [f64; 2],
    /// The band the chip's S(ω) is computed over, µm, and its points.
    pub from_um: f64,
    /// Its end.
    pub to_um: f64,
    /// Its points.
    pub points: usize,
    /// How many delays (201 by default).
    pub delays: Option<usize>,
    /// The delays from −span to +span, ps (four times the photons' coherence by default).
    pub span_ps: Option<f64>,
}

/// A photon of a budget: from a port to another, along its best route.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegTask {
    /// The port it enters by.
    pub from: String,
    /// The port it leaves by.
    pub to: String,
}

/// Squeezed vacuum on a port, then displaced.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SqueezedTask {
    /// The port.
    pub port: String,
    /// 10 log₁₀ e^{2r}: the squeezed quadrature's variance below the vacuum's, dB.
    pub squeezing_db: f64,
    /// φ of ξ = r e^{iφ}, degrees: x squeezed at 0, p at 180.
    #[serde(default)]
    pub angle_deg: f64,
    /// α, its real and imaginary parts.
    #[serde(default)]
    pub displacement: [f64; 2],
}

// ---- what a run records ----

/// A pattern at the ports with its probability for photons alike and told apart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumPattern {
    /// Photons per port, in the ports' order.
    pub occupations: Vec<usize>,
    /// [alike, told apart].
    pub probability: [f64; 2],
}

/// The statistics of photons on a chip's ports ([`Event::QuantumStatistics`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumStatistics {
    /// The wavelength, µm.
    pub wavelength_um: f64,
    /// The ports, in the patterns' order.
    pub ports: Vec<String>,
    /// The photons put on each port.
    pub input: Vec<usize>,
    /// The loss modes the chip's loss is dilated to.
    pub loss_modes: usize,
    /// Every pattern at the ports, a pattern with fewer photons one where some were lost.
    pub patterns: Vec<QuantumPattern>,
    /// The probability that every photon arrives, [alike, told apart].
    pub all_arrive: [f64; 2],
    /// The probability that a port holds two photons or more.
    pub bunching: [f64; 2],
    /// Each port's mean photon number, [alike, told apart].
    pub means: Vec<[f64; 2]>,
}

/// A pattern and how many samples fell on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumCount {
    /// Photons per port.
    pub occupations: Vec<usize>,
    /// Samples.
    pub count: usize,
}

/// Samples of photons on a chip's ports against their exact distribution
/// ([`Event::QuantumSamples`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumSamples {
    /// How many, and their seed.
    pub samples: usize,
    /// The seed.
    pub seed: u64,
    /// Each pattern sampled, with its count.
    pub counts: Vec<QuantumCount>,
    /// ½ Σ |p̂ − p|.
    pub distance: f64,
    /// ½ Σ √(2p(1 − p)/(πN)).
    pub expected: f64,
}

/// A Hong–Ou–Mandel dip through a chip ([`Event::QuantumDip`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumDip {
    /// The ports the photons enter by.
    pub inputs: [String; 2],
    /// The ports whose coincidences are counted.
    pub outputs: [String; 2],
    /// Photon 1's delays, ps.
    pub delays_ps: Vec<f64>,
    /// The coincidence probability at each.
    pub coincidences: Vec<f64>,
    /// The coincidence probability of photons told apart.
    pub distinguishable: f64,
    /// 1 − the fewest coincidences over those of photons told apart.
    pub visibility: f64,
}

/// A stage of a route.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumStage {
    /// The instance, `port → port`.
    pub name: String,
    /// The share of photons it keeps.
    pub transmission: f64,
    /// Where that comes from.
    pub source: String,
}

/// A photon's best route through a chip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumLeg {
    /// The port it enters by.
    pub from: String,
    /// The port it leaves by.
    pub to: String,
    /// How many routes there are (up to 32 looked at).
    pub routes: usize,
    /// The best one's instances, its stages and its transmission; none when no route keeps
    /// power.
    pub through: Vec<String>,
    /// Its stages.
    pub stages: Vec<QuantumStage>,
    /// Its transmission.
    pub transmission: f64,
}

/// A published threshold on a photon's transmission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumThreshold {
    /// Whose.
    pub name: String,
    /// The transmission it asks of each photon.
    pub transmission: f64,
    /// What it assumes.
    pub assumptions: String,
}

/// A loss budget through a chip ([`Event::QuantumBudget`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumBudget {
    /// The wavelength, µm.
    pub wavelength_um: f64,
    /// Each photon's best route.
    pub legs: Vec<QuantumLeg>,
    /// The n-photon event: the product of the routes' transmissions.
    pub event: f64,
    /// The published thresholds.
    pub thresholds: Vec<QuantumThreshold>,
}

/// Squeezed light through a chip ([`Event::QuantumSqueezed`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantumSqueezed {
    /// The wavelength, µm.
    pub wavelength_um: f64,
    /// The ports.
    pub ports: Vec<String>,
    /// The squeezing given each, dB.
    pub given_db: Vec<f64>,
    /// The smallest quadrature variance of the light leaving each, dB from the vacuum's.
    pub squeezed_db: Vec<f64>,
    /// The largest.
    pub anti_squeezed_db: Vec<f64>,
    /// Its mean photon number.
    pub mean_photons: Vec<f64>,
    /// The whole state's purity.
    pub purity: f64,
}

// ---- checking ----

fn task_of(job: &Job) -> Result<QuantumTask> {
    job.task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))
}

/// C(n + k − 1, k), saturating.
fn patterns_of(modes: usize, photons: usize) -> usize {
    let mut c: u128 = 1;
    for i in 1..=photons as u128 {
        c = c * (modes as u128 + i - 1) / i;
        if c > usize::MAX as u128 {
            return usize::MAX;
        }
    }
    c as usize
}

/// The task read and checked against the chip, its chip built and its transfer at the
/// wavelength.
struct Prepared {
    task: QuantumTask,
    chip: ChipDesign,
    assembled: Assembled,
    ports: Vec<String>,
    transfer: Transfer,
    wavelength: Wavelength,
}

fn prepare(job: &Job) -> Result<Prepared> {
    let chip = chip_of(job)?;
    let task = task_of(job)?;
    if task.photons.is_empty()
        && task.dip.is_none()
        && task.budget.is_empty()
        && task.squeezed.is_empty()
    {
        return Err(task_error(
            "a quantum task computes something: photons ([[task.photons]]), a dip ([task.dip]), \
             a budget ([[task.budget]]) or squeezed light ([[task.squeezed]])",
        ));
    }
    let wavelength = Wavelength::um(task.wavelength_um)?;
    let assembled = chip.assemble()?;
    let ports = chip.port_names();
    if ports.len() > MOST_PORTS {
        return Err(task_error(format!(
            "{} external ports: a quantum task takes up to {MOST_PORTS}",
            ports.len()
        )));
    }
    let s = assembled.s_matrix(task.wavelength_um)?;
    let transfer = Transfer::new(&s, ports.clone())?;
    let port = |name: &str| -> Result<usize> {
        transfer.port(name).ok_or_else(|| {
            task_error(format!(
                "no port {name}: the chip's ports are {}",
                ports.join(", ")
            ))
        })
    };
    let n: usize = task.photons.iter().map(|p| p.count).sum();
    for p in &task.photons {
        port(&p.port)?;
    }
    if !task.photons.is_empty() {
        if n == 0 || n > MOST_PHOTONS {
            return Err(task_error(format!(
                "the photons on the ports number 1 to {MOST_PHOTONS}, not {n}"
            )));
        }
        let made = patterns_of(ports.len() + transfer.loss_modes(), n);
        if made > MOST_PATTERNS {
            return Err(task_error(format!(
                "{n} photons in {} ports and {} loss modes make {made} patterns: up to \
                 {MOST_PATTERNS}",
                ports.len(),
                transfer.loss_modes()
            )));
        }
    }
    if let Some(s) = task.sample {
        if task.photons.is_empty() {
            return Err(task_error(
                "samples are of the photons on the ports: give them ([[task.photons]])",
            ));
        }
        if s.samples == 0 || s.samples > MOST_SAMPLES {
            return Err(task_error(format!(
                "a task draws 1 to {MOST_SAMPLES} samples, not {}",
                s.samples
            )));
        }
    }
    if let Some(d) = &task.dip {
        for p in d.inputs.iter().chain(&d.outputs) {
            port(p)?;
        }
        dip_plan(d)?;
    }
    for leg in &task.budget {
        port(&leg.from)?;
        port(&leg.to)?;
    }
    if task.budget.len() > MOST_PHOTONS {
        return Err(task_error(format!(
            "a budget takes up to {MOST_PHOTONS} photons, not {}",
            task.budget.len()
        )));
    }
    let mut seen = Vec::new();
    for q in &task.squeezed {
        let m = port(&q.port)?;
        if seen.contains(&m) {
            return Err(task_error(format!(
                "port {} is given squeezed light twice",
                q.port
            )));
        }
        seen.push(m);
        if !(q.squeezing_db.is_finite() && (0.0..=MOST_SQUEEZING_DB).contains(&q.squeezing_db)) {
            return Err(task_error(format!(
                "a port's squeezing is 0 to {MOST_SQUEEZING_DB} dB, not {} dB at {}",
                q.squeezing_db, q.port
            )));
        }
        if !(q.angle_deg.is_finite() && q.displacement.iter().all(|v| v.is_finite())) {
            return Err(task_error(format!(
                "the angle and the displacement at {} must be numbers",
                q.port
            )));
        }
    }
    Ok(Prepared {
        task,
        chip,
        assembled,
        ports,
        transfer,
        wavelength,
    })
}

/// A dip's band, its photons and its delays (ps), checked.
fn dip_plan(d: &DipTask) -> Result<(Vec<Wavelength>, [Wavepacket; 2], Vec<f64>)> {
    let wavelengths = super::chip::band(d.from_um, d.to_um, Some(d.points), d.points)?
        .into_iter()
        .map(Wavelength::um)
        .collect::<Result<Vec<_>>>()?;
    if wavelengths.len() < 2 {
        return Err(task_error("a dip's band takes two points or more"));
    }
    let step_um = (d.to_um - d.from_um) / (d.points - 1) as f64;
    let mut widest_ps: f64 = 0.0;
    let mut packets = Vec::with_capacity(2);
    for k in 0..2 {
        let (c, w) = (d.center_um[k], d.fwhm_nm[k]);
        if !(c >= d.from_um && c <= d.to_um) {
            return Err(task_error(format!(
                "photon {}'s centre, {c} µm, is outside the band, {} to {} µm",
                k + 1,
                d.from_um,
                d.to_um
            )));
        }
        if !(w.is_finite() && w * 1e-3 >= POINTS_PER_WIDTH * step_um) {
            return Err(task_error(format!(
                "photon {}'s width, {w} nm, spans fewer than {POINTS_PER_WIDTH} of the band's \
                 steps ({:.4} nm)",
                k + 1,
                step_um * 1e3
            )));
        }
        let packet = Wavepacket::gaussian(Wavelength::um(c)?, w)?;
        if let Wavepacket::Gaussian { width, .. } = packet {
            widest_ps = widest_ps.max(1e12 / width);
        }
        packets.push(packet);
    }
    let shortest = d.from_um * 1e-6;
    let largest_step =
        2.0 * PI * SPEED_OF_LIGHT * (step_um * 1e-6) / (shortest * (shortest + step_um * 1e-6));
    let resolved_ps = PI / largest_step * 1e12;
    let span = match d.span_ps {
        Some(s) if !(s.is_finite() && s > 0.0) || s > resolved_ps => {
            return Err(task_error(format!(
                "the delays' span must be above 0 and at most ±{resolved_ps:.3} ps (π over the \
                 band's largest angular step), not {s} ps"
            )));
        }
        Some(s) => s,
        None => (4.0 * widest_ps).min(resolved_ps),
    };
    let count = d.delays.unwrap_or(201);
    if !(2..=MOST_DELAYS).contains(&count) {
        return Err(task_error(format!(
            "a dip takes 2 to {MOST_DELAYS} delays, not {count}"
        )));
    }
    let last = (count - 1) as f64;
    let delays = (0..count)
        .map(|k| span * (2.0 * k as f64 - last) / last)
        .collect();
    let [a, b]: [Wavepacket; 2] = packets
        .try_into()
        .map_err(|_| Error::invalid("dip", "two photons"))?;
    Ok((wavelengths, [a, b], delays))
}

// ---- running ----

fn statistics(p: &Prepared) -> Result<(QuantumStatistics, Fock, Distribution)> {
    let given: Vec<(&str, usize)> = p
        .task
        .photons
        .iter()
        .map(|q| (q.port.as_str(), q.count))
        .collect();
    let input = p.transfer.input(&given)?;
    let n = input.photons();
    let alike = p
        .transfer
        .distribution(&input, &Photons::Indistinguishable)?;
    let apart = p.transfer.distribution(&input, &Photons::Distinguishable)?;
    let apart_of: BTreeMap<&Fock, f64> = apart
        .patterns()
        .iter()
        .zip(apart.probabilities())
        .map(|(f, &q)| (f, q))
        .collect();
    let patterns: Vec<QuantumPattern> = alike
        .patterns()
        .iter()
        .zip(alike.probabilities())
        .map(|(f, &q)| QuantumPattern {
            occupations: f.occupations().to_vec(),
            probability: [q, apart_of.get(f).copied().unwrap_or(0.0)],
        })
        .collect();
    let arrive = |d: &Distribution| -> f64 {
        d.patterns()
            .iter()
            .zip(d.probabilities())
            .filter(|(f, _)| f.photons() == n)
            .map(|(_, q)| q)
            .sum()
    };
    let stats = QuantumStatistics {
        wavelength_um: p.wavelength.to_um(),
        ports: p.ports.clone(),
        input: input.occupations().to_vec(),
        loss_modes: p.transfer.loss_modes(),
        patterns,
        all_arrive: [arrive(&alike), arrive(&apart)],
        bunching: [alike.bunching(), apart.bunching()],
        means: (0..p.ports.len())
            .map(|k| [alike.mean(k), apart.mean(k)])
            .collect(),
    };
    Ok((stats, input, alike))
}

fn samples(
    p: &Prepared,
    input: &Fock,
    exact: &Distribution,
    task: SampleTask,
) -> Result<QuantumSamples> {
    let drawn = p.transfer.sample(input, task.samples, task.seed)?;
    let mut counts: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
    for f in &drawn {
        *counts.entry(f.occupations().to_vec()).or_default() += 1;
    }
    let counted: Vec<(Fock, usize)> = counts
        .iter()
        .map(|(o, &c)| (Fock::new(o.clone()), c))
        .collect();
    let distance = exact.total_variation(&Distribution::from_counts(&counted));
    let expected = 0.5
        * exact
            .probabilities()
            .iter()
            .map(|&q| (2.0 * q * (1.0 - q) / (PI * task.samples as f64)).sqrt())
            .sum::<f64>();
    Ok(QuantumSamples {
        samples: task.samples,
        seed: task.seed,
        counts: counts
            .into_iter()
            .map(|(occupations, count)| QuantumCount { occupations, count })
            .collect(),
        distance,
        expected,
    })
}

fn dip(p: &Prepared, d: &DipTask) -> Result<QuantumDip> {
    let (wavelengths, packets, delays_ps) = dip_plan(d)?;
    let spectrum = Spectrum::of(&p.assembled.circuit, &wavelengths, &p.assembled.values)?;
    let delays: Vec<f64> = delays_ps.iter().map(|t| t * 1e-12).collect();
    let dip = quantum::hom_dip(
        &spectrum,
        [&d.inputs[0], &d.inputs[1]],
        [&d.outputs[0], &d.outputs[1]],
        [&packets[0], &packets[1]],
        &delays,
    )?;
    Ok(QuantumDip {
        inputs: d.inputs.clone(),
        outputs: d.outputs.clone(),
        visibility: dip.visibility(),
        distinguishable: dip.distinguishable,
        coincidences: dip.coincidences,
        delays_ps,
    })
}

fn budget(p: &Prepared) -> Result<QuantumBudget> {
    let mut legs = Vec::with_capacity(p.task.budget.len());
    let mut event = 1.0;
    for leg in &p.task.budget {
        let found = quantum::routes(
            &p.assembled.circuit,
            &leg.from,
            &leg.to,
            p.wavelength,
            MOST_ROUTES,
        )?;
        let mut best: Option<(Vec<String>, Budget)> = None;
        for hops in &found {
            let b = Budget::along(&p.assembled.circuit, hops, p.wavelength)?;
            if best
                .as_ref()
                .is_none_or(|(_, kept)| b.transmission() > kept.transmission())
            {
                best = Some((hops.iter().map(|h| h.instance.clone()).collect(), b));
            }
        }
        let (through, stages, transmission) = match best {
            Some((through, b)) => (
                through,
                b.stages()
                    .iter()
                    .map(|s| QuantumStage {
                        name: s.name.clone(),
                        transmission: s.transmission,
                        source: s.source.clone(),
                    })
                    .collect(),
                b.transmission(),
            ),
            None => (Vec::new(), Vec::new(), 0.0),
        };
        event *= transmission;
        legs.push(QuantumLeg {
            from: leg.from.clone(),
            to: leg.to.clone(),
            routes: found.len(),
            through,
            stages,
            transmission,
        });
    }
    let thresholds = [
        (Threshold::Varnava2008, "Varnava, Browne & Rudolph 2008"),
        (Threshold::Bartolucci2023, "Bartolucci et al. 2023"),
    ]
    .into_iter()
    .map(|(t, name)| QuantumThreshold {
        name: name.into(),
        transmission: t.transmission(),
        assumptions: t.assumptions().into(),
    })
    .collect();
    Ok(QuantumBudget {
        wavelength_um: p.wavelength.to_um(),
        legs,
        event,
        thresholds,
    })
}

fn squeezed(p: &Prepared) -> Result<QuantumSqueezed> {
    let n = p.ports.len();
    let mut given = vec![0.0; n];
    let mut state = Gaussian::vacuum(n);
    for q in &p.task.squeezed {
        let m = p
            .transfer
            .port(&q.port)
            .ok_or_else(|| task_error(format!("no port {}", q.port)))?;
        given[m] = q.squeezing_db;
        let r = q.squeezing_db * std::f64::consts::LN_10 / 20.0;
        state = state
            .squeezed(m, r, q.angle_deg.to_radians())?
            .displaced(m, c64::new(q.displacement[0], q.displacement[1]))?;
    }
    let out = state.through(&p.transfer)?;
    let db = |v: f64| 10.0 * v.log10();
    let mut squeezed_db = Vec::with_capacity(n);
    let mut anti_squeezed_db = Vec::with_capacity(n);
    let mut mean_photons = Vec::with_capacity(n);
    for m in 0..n {
        let (low, high) = out.quadrature_extremes(m)?;
        squeezed_db.push(db(low));
        anti_squeezed_db.push(db(high));
        mean_photons.push(out.mean_photons(m)?);
    }
    Ok(QuantumSqueezed {
        wavelength_um: p.wavelength.to_um(),
        ports: p.ports.clone(),
        given_db: given,
        squeezed_db,
        anti_squeezed_db,
        mean_photons,
        purity: out.purity(),
    })
}

fn run_quantum(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let p = prepare(job)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: "quantum".into(),
    })?;
    run.record(&Event::Chip {
        instances: p.chip.parts(&p.assembled),
        ports: p.ports.clone(),
    })?;
    if !p.task.photons.is_empty() && stop.reason().is_none() {
        let (stats, input, exact) = statistics(&p)?;
        run.record(&Event::QuantumStatistics {
            statistics: Box::new(stats),
        })?;
        if let Some(s) = p.task.sample
            && stop.reason().is_none()
        {
            run.record(&Event::QuantumSamples {
                samples: Box::new(samples(&p, &input, &exact, s)?),
            })?;
        }
    }
    if let Some(d) = &p.task.dip
        && stop.reason().is_none()
    {
        run.record(&Event::QuantumDip {
            dip: Box::new(dip(&p, d)?),
        })?;
    }
    if !p.task.budget.is_empty() && stop.reason().is_none() {
        run.record(&Event::QuantumBudget {
            budget: Box::new(budget(&p)?),
        })?;
    }
    if !p.task.squeezed.is_empty() && stop.reason().is_none() {
        run.record(&Event::QuantumSqueezed {
            squeezed: Box::new(squeezed(&p)?),
        })?;
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

/// The `"quantum"` kind beside a `[circuit]`.
pub(super) struct QuantumKind;

impl TaskKind for QuantumKind {
    fn name(&self) -> &'static str {
        "quantum"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        super::schema::task::<QuantumTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        prepare(job).map(|_| ())
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        super::check(job)?;
        super::fdfd::preview(job)
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        run_quantum(job, run, stop)
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        CHIP_EXPORTS
    }
}

/// The job that computes photons through `chip` as `task` says (its `[task]` table without its
/// kind: `wavelength_um`, `photons`, `sample`, `dip`, `budget`, `squeezed`), named `name`, with
/// a hard limit of `timeout_minutes`. It is checked by [`super::check`].
///
/// # Errors
///
/// The errors of [`super::check`] for the job it makes.
pub fn quantum_from_chip(
    name: &str,
    chip: &ChipDesign,
    task: &Table,
    timeout_minutes: f64,
) -> Result<Job> {
    let mut t = task.clone();
    t.insert("kind".into(), Value::String("quantum".into()));
    chip_job(name, chip, t, timeout_minutes)
}

#[cfg(test)]
#[path = "quantum_tests.rs"]
mod tests;

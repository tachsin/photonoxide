//! The Chip page's quantum view: photons on the chip's external ports, through the chip's own
//! S-matrix (`photonoxide::quantum`).
//!
//! - **Statistics:** a Fock state on the ports at one wavelength of the sweep, its loss dilated
//!   to loss modes ([`Transfer`]): every output pattern's probability for indistinguishable and
//!   for distinguishable photons, each port's photon numbers, bunching, and the share of events
//!   where every photon arrives ([`chip_quantum_statistics`]).
//! - **The Hong–Ou–Mandel dip:** two photons with Gaussian spectra through the chip's S(ω) over
//!   its sweep, the coincidences between two ports against the delay ([`chip_hom_dip`]).
//! - **The loss budget:** each photon's routes from its input to its output port, stage by
//!   stage, with a source and a detector, the n-photon event and the published thresholds
//!   ([`chip_loss_budget`]).
//!
//! The library does the physics; this module checks what the view asks against its limits,
//! hands it to the library, and returns what comes back as the view draws it.

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::path::Path;
use std::time::Instant;

use photonoxide::circuit::{Circuit, Component};
use photonoxide::quantum::{self, Budget, Fock, Photons, Threshold, Transfer, Wavepacket};
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};
use serde::{Deserialize, Serialize};

use super::{Chip, complete};
use crate::studio::Studio;

/// The most photons the view puts on the ports: each output pattern is a permanent of that size,
/// and their number grows as a binomial in them.
pub const MOST_PHOTONS: usize = 8;
/// The most external ports a chip may have for the view's statistics.
pub const MOST_PORTS: usize = 16;
/// The most output patterns (over the ports and the loss modes) the statistics compute.
pub const MOST_PATTERNS: usize = 100_000;
/// The most patterns returned to be drawn, most likely first; the rest are summed.
pub const SHOWN_PATTERNS: usize = 40;
/// The most delays in a dip.
pub const MOST_DELAYS: usize = 2001;
/// The fewest sweep points across a photon's full width at half maximum: fewer don't resolve its
/// spectrum, and the dip is the sweep's, not the photon's.
pub const POINTS_PER_WIDTH: f64 = 4.0;
/// The most routes listed for one photon.
pub const MOST_ROUTES: usize = 32;

/// Photons on one external port.
#[derive(Clone, Debug, Deserialize)]
pub struct Input {
    pub port: String,
    pub photons: usize,
}

/// An output pattern: photons per port, with its probability for indistinguishable and for
/// distinguishable photons.
#[derive(Debug, Serialize)]
pub struct Pattern {
    occupations: Vec<usize>,
    probability: [f64; 2],
}

/// A port's photon numbers: P(k photons there), and the mean, for both kinds of photons.
#[derive(Debug, Serialize)]
pub struct Marginal {
    port: String,
    probabilities: [Vec<f64>; 2],
    mean: [f64; 2],
}

/// The statistics of photons on a chip's ports at one wavelength. Each pair is
/// [indistinguishable, distinguishable].
#[derive(Debug, Serialize)]
pub struct Statistics {
    wavelength_um: f64,
    ports: Vec<String>,
    /// |S_qp|, rows the outputs and columns the inputs, in the ports' order.
    magnitude: Vec<Vec<f64>>,
    /// arg S_qp, rad.
    phase: Vec<Vec<f64>>,
    /// The channels that lose power: the dilation's loss modes.
    loss_modes: usize,
    /// How far the dilation is from exact: round-off.
    dilation_error: f64,
    photons: usize,
    /// The patterns computed, over the ports and the loss modes.
    computed: usize,
    /// The patterns where every photon arrives, most likely first (for indistinguishable
    /// photons), at most [`SHOWN_PATTERNS`].
    patterns: Vec<Pattern>,
    /// How many patterns of every photon arriving there are, and the probability of those not
    /// in `patterns`.
    arriving_patterns: usize,
    others: [f64; 2],
    /// The probability that every photon arrives at a port.
    all_arrive: [f64; 2],
    /// The probability that some port holds two photons or more.
    bunching: [f64; 2],
    marginals: Vec<Marginal>,
    seconds: f64,
}

/// The wavelength `um` checked against the chip's sweep: the view's results are the sweep's.
fn in_sweep(chip: &Chip, um: f64) -> Result<Wavelength, String> {
    let (from, to) = (chip.sweep.from_um, chip.sweep.to_um);
    if !(um.is_finite() && um >= from && um <= to) {
        return Err(format!(
            "the wavelength must be in the chip's sweep, {from} to {to} µm, not {um} µm"
        ));
    }
    Wavelength::um(um).map_err(|e| e.to_string())
}

/// C(n + k − 1, k): the patterns of k photons in n modes, saturating.
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

/// The ports' names of a compiled circuit.
fn port_names(circuit: &Circuit) -> Vec<String> {
    circuit.ports().iter().map(|p| p.name.clone()).collect()
}

/// The statistics of photons on `chip`'s external ports at `wavelength_um`.
#[tauri::command(async)]
pub fn chip_quantum_statistics(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    wavelength_um: f64,
    inputs: Vec<Input>,
) -> Result<Statistics, String> {
    statistics(&chip, &state.workspace(), wavelength_um, &inputs)
}

pub(crate) fn statistics(
    chip: &Chip,
    root: &Path,
    wavelength_um: f64,
    inputs: &[Input],
) -> Result<Statistics, String> {
    let n: usize = inputs.iter().map(|i| i.photons).sum();
    if n == 0 {
        return Err("put a photon on a port first".into());
    }
    if n > MOST_PHOTONS {
        return Err(format!(
            "{n} photons: the view takes up to {MOST_PHOTONS} (the library's permanents take more, \
             from a program)"
        ));
    }
    let wavelength = in_sweep(chip, wavelength_um)?;
    let circuit = complete(chip, root)?.compile().map_err(|e| e.to_string())?;
    let ports = port_names(&circuit);
    if ports.len() > MOST_PORTS {
        return Err(format!(
            "{} external ports: the view's statistics take up to {MOST_PORTS}",
            ports.len()
        ));
    }
    let started = Instant::now();
    let transfer = Transfer::of(&circuit, wavelength).map_err(|e| e.to_string())?;
    let modes = ports.len() + transfer.loss_modes();
    let computed = patterns_of(modes, n);
    if computed > MOST_PATTERNS {
        return Err(format!(
            "{n} photons in {} ports and {} loss modes make {computed} patterns; the view computes \
             up to {MOST_PATTERNS}: take fewer photons or ports",
            ports.len(),
            transfer.loss_modes()
        ));
    }
    let given: Vec<(&str, usize)> = inputs
        .iter()
        .filter(|i| i.photons > 0)
        .map(|i| (i.port.as_str(), i.photons))
        .collect();
    let input = transfer.input(&given).map_err(|e| e.to_string())?;
    let alike = transfer
        .distribution(&input, &Photons::Indistinguishable)
        .map_err(|e| e.to_string())?;
    let apart = transfer
        .distribution(&input, &Photons::Distinguishable)
        .map_err(|e| e.to_string())?;
    let apart_of: BTreeMap<&Fock, f64> = apart
        .patterns()
        .iter()
        .zip(apart.probabilities())
        .map(|(p, &q)| (p, q))
        .collect();
    let mut arriving: Vec<Pattern> = alike
        .patterns()
        .iter()
        .zip(alike.probabilities())
        .filter(|(p, _)| p.photons() == n)
        .map(|(p, &q)| Pattern {
            occupations: p.occupations().to_vec(),
            probability: [q, apart_of.get(p).copied().unwrap_or(0.0)],
        })
        .collect();
    let all_arrive = [0, 1].map(|k| arriving.iter().map(|p| p.probability[k]).sum::<f64>());
    arriving.sort_by(|a, b| b.probability[0].total_cmp(&a.probability[0]));
    let arriving_patterns = arriving.len();
    let others = [0, 1].map(|k| {
        arriving
            .iter()
            .skip(SHOWN_PATTERNS)
            .map(|p| p.probability[k])
            .sum::<f64>()
    });
    arriving.truncate(SHOWN_PATTERNS);
    let marginals = ports
        .iter()
        .enumerate()
        .map(|(k, port)| Marginal {
            port: port.clone(),
            probabilities: [alike.marginal(k), apart.marginal(k)],
            mean: [alike.mean(k), apart.mean(k)],
        })
        .collect();
    let s = transfer.matrix();
    let size = ports.len();
    Ok(Statistics {
        wavelength_um,
        magnitude: (0..size)
            .map(|q| (0..size).map(|p| s[(q, p)].norm()).collect())
            .collect(),
        phase: (0..size)
            .map(|q| (0..size).map(|p| s[(q, p)].arg()).collect())
            .collect(),
        ports,
        loss_modes: transfer.loss_modes(),
        dilation_error: transfer.dilation_error(),
        photons: n,
        computed,
        patterns: arriving,
        arriving_patterns,
        others,
        all_arrive,
        bunching: [alike.bunching(), apart.bunching()],
        marginals,
        seconds: started.elapsed().as_secs_f64(),
    })
}

/// A photon with a Gaussian spectrum: its centre and its power spectrum's full width at half
/// maximum.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Photon {
    pub center_um: f64,
    pub fwhm_nm: f64,
}

/// What a dip is asked for: photon 1 into `inputs[0]` delayed, photon 2 into `inputs[1]`, the
/// coincidences between `outputs`; the delays from −span to +span (a span of the photons' own
/// when none is given), `points` of them.
#[derive(Clone, Debug, Deserialize)]
pub struct DipRequest {
    pub inputs: [String; 2],
    pub outputs: [String; 2],
    pub photons: [Photon; 2],
    pub span_ps: Option<f64>,
    pub points: usize,
}

/// A Hong–Ou–Mandel dip through the chip.
#[derive(Debug, Serialize)]
pub struct DipView {
    delays_ps: Vec<f64>,
    coincidences: Vec<f64>,
    /// The coincidence probability of distinguishable photons, which a long delay reaches.
    distinguishable: f64,
    visibility: f64,
    /// The delay of the fewest coincidences.
    deepest_ps: f64,
    span_ps: f64,
    /// The longest delay the sweep's step resolves: π over its largest angular step.
    resolved_ps: f64,
    /// What the dip should be read with: a spectrum cut by the sweep's ends.
    notes: Vec<String>,
    seconds: f64,
}

/// The coincidences between two of `chip`'s ports against the delay between two photons, through
/// its swept S(ω).
#[tauri::command(async)]
pub fn chip_hom_dip(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    request: DipRequest,
) -> Result<DipView, String> {
    hom(&chip, &state.workspace(), &request)
}

pub(crate) fn hom(chip: &Chip, root: &Path, request: &DipRequest) -> Result<DipView, String> {
    if !(2..=MOST_DELAYS).contains(&request.points) {
        return Err(format!(
            "a dip takes 2 to {MOST_DELAYS} delays, not {}",
            request.points
        ));
    }
    let sweep = chip.sweep;
    let wavelengths = sweep.wavelengths()?;
    let step_um = (sweep.to_um - sweep.from_um) / (sweep.points - 1) as f64;
    let mut notes = Vec::new();
    let mut packets = Vec::with_capacity(2);
    let mut widest_ps: f64 = 0.0;
    for (k, photon) in request.photons.iter().enumerate() {
        let centre =
            in_sweep(chip, photon.center_um).map_err(|e| format!("photon {}: {e}", k + 1))?;
        let width_um = photon.fwhm_nm * 1e-3;
        if !(photon.fwhm_nm.is_finite() && width_um >= POINTS_PER_WIDTH * step_um) {
            return Err(format!(
                "photon {}'s width, {} nm, spans fewer than {POINTS_PER_WIDTH} of the sweep's \
                 steps ({:.4} nm): add points to the sweep, or widen the photon",
                k + 1,
                photon.fwhm_nm,
                step_um * 1e3
            ));
        }
        if photon.center_um - 2.0 * width_um < sweep.from_um
            || photon.center_um + 2.0 * width_um > sweep.to_um
        {
            notes.push(format!(
                "photon {}'s spectrum runs past the sweep's ends, which cut it within two widths \
                 of its centre: the dip is the cut photon's",
                k + 1
            ));
        }
        let packet = Wavepacket::gaussian(centre, photon.fwhm_nm).map_err(|e| e.to_string())?;
        if let Wavepacket::Gaussian { width, .. } = packet {
            // the dip's 1/e half-width in delay, 1/σ
            widest_ps = widest_ps.max(1e12 / width);
        }
        packets.push(packet);
    }
    // the trapezoid's phase e^(iωτ) turns by Δω τ over a step: beyond π it aliases
    let shortest = sweep.from_um * 1e-6;
    let largest_step =
        2.0 * PI * SPEED_OF_LIGHT * (step_um * 1e-6) / (shortest * (shortest + step_um * 1e-6));
    let resolved_ps = PI / largest_step * 1e12;
    let span_ps = match request.span_ps {
        Some(s) if !(s.is_finite() && s > 0.0) => {
            return Err(format!("the delays' span must be positive, not {s} ps"));
        }
        Some(s) if s > resolved_ps => {
            return Err(format!(
                "delays beyond ±{resolved_ps:.3} ps turn the phase by more than π between the \
                 sweep's points: add points to the sweep, or shorten the span"
            ));
        }
        Some(s) => s,
        None => (4.0 * widest_ps).min(resolved_ps),
    };
    let delays_ps: Vec<f64> = (0..request.points)
        .map(|k| -span_ps + 2.0 * span_ps * k as f64 / (request.points - 1) as f64)
        .collect();
    let started = Instant::now();
    let netlist = complete(chip, root)?;
    let spectrum = netlist
        .compile()
        .and_then(|c| c.spectrum(&wavelengths))
        .map_err(|e| e.to_string())?;
    let delays: Vec<f64> = delays_ps.iter().map(|d| d * 1e-12).collect();
    let dip = quantum::hom_dip(
        &spectrum,
        [&request.inputs[0], &request.inputs[1]],
        [&request.outputs[0], &request.outputs[1]],
        [&packets[0], &packets[1]],
        &delays,
    )
    .map_err(|e| e.to_string())?;
    let deepest = dip
        .coincidences
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(k, _)| k);
    Ok(DipView {
        deepest_ps: delays_ps[deepest],
        visibility: dip.visibility(),
        distinguishable: dip.distinguishable,
        coincidences: dip.coincidences,
        delays_ps,
        span_ps,
        resolved_ps,
        notes,
        seconds: started.elapsed().as_secs_f64(),
    })
}

/// A photon of a budget: from an input port to an output port, along one of its routes (the one
/// that keeps the most when none is chosen).
#[derive(Clone, Debug, Deserialize)]
pub struct Leg {
    pub from: String,
    pub to: String,
    pub route: Option<usize>,
}

/// What a budget is asked for: its photons, at one wavelength of the sweep, with a source's and
/// a detector's efficiency (stated by the user; 1 leaves them out).
#[derive(Clone, Debug, Deserialize)]
pub struct BudgetRequest {
    pub wavelength_um: f64,
    pub photons: Vec<Leg>,
    pub source: f64,
    pub detector: f64,
}

/// A stage of a budget as the waterfall draws it.
#[derive(Debug, Serialize)]
pub struct StageView {
    name: String,
    transmission: f64,
    loss_db: f64,
    source: String,
}

/// A route's budget.
#[derive(Debug, Serialize)]
pub struct RouteView {
    /// The instances it crosses, in order.
    through: Vec<String>,
    stages: Vec<StageView>,
    transmission: f64,
    loss_db: f64,
}

/// One photon's routes and the one taken.
#[derive(Debug, Serialize)]
pub struct LegView {
    from: String,
    to: String,
    routes: Vec<RouteView>,
    /// The route taken, if there is any.
    chosen: Option<usize>,
    /// Whether there are more routes than those listed.
    more: bool,
    /// |S_to,from|² of the whole circuit, with every route's interference.
    circuit: f64,
}

/// A published threshold on a photon's transmission.
#[derive(Debug, Serialize)]
pub struct ThresholdView {
    name: &'static str,
    doi: &'static str,
    transmission: f64,
    loss_db: f64,
    assumptions: &'static str,
}

/// A chip's loss budget.
#[derive(Debug, Serialize)]
pub struct BudgetView {
    photons: Vec<LegView>,
    /// The probability that every photon arrives, each along its route: 0 when one has none.
    event: f64,
    /// As a loss in dB; none when a photon has no route.
    event_db: Option<f64>,
    thresholds: Vec<ThresholdView>,
    seconds: f64,
}

/// −10 log₁₀ t: a transmission as a loss in dB.
fn db(t: f64) -> f64 {
    -10.0 * t.log10()
}

/// The loss budget of photons through `chip`: each one's routes, stage by stage.
#[tauri::command(async)]
pub fn chip_loss_budget(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    request: BudgetRequest,
) -> Result<BudgetView, String> {
    budget(&chip, &state.workspace(), &request)
}

pub(crate) fn budget(
    chip: &Chip,
    root: &Path,
    request: &BudgetRequest,
) -> Result<BudgetView, String> {
    if request.photons.is_empty() {
        return Err("give a photon a port to start from and one to arrive at".into());
    }
    if request.photons.len() > MOST_PHOTONS {
        return Err(format!(
            "{} photons: the view's budget takes up to {MOST_PHOTONS}",
            request.photons.len()
        ));
    }
    for (name, eta) in [("source", request.source), ("detector", request.detector)] {
        if !(0.0..=1.0).contains(&eta) || eta == 0.0 {
            return Err(format!(
                "a {name}'s efficiency is above 0 and at most 1, not {eta}"
            ));
        }
    }
    let wavelength = in_sweep(chip, request.wavelength_um)?;
    let started = Instant::now();
    let circuit = complete(chip, root)?.compile().map_err(|e| e.to_string())?;
    let ports = port_names(&circuit);
    let s = circuit.s_matrix(wavelength).map_err(|e| e.to_string())?;
    let stated = |b: Budget, name: &str, eta: f64| {
        if eta < 1.0 {
            b.with(name, eta, "stated").map_err(|e| e.to_string())
        } else {
            Ok(b)
        }
    };
    let mut legs = Vec::with_capacity(request.photons.len());
    let mut event = 1.0;
    for leg in &request.photons {
        let found = quantum::routes(&circuit, &leg.from, &leg.to, wavelength, MOST_ROUTES + 1)
            .map_err(|e| e.to_string())?;
        let more = found.len() > MOST_ROUTES;
        let mut routes = Vec::with_capacity(found.len().min(MOST_ROUTES));
        for hops in found.iter().take(MOST_ROUTES) {
            let path = Budget::along(&circuit, hops, wavelength).map_err(|e| e.to_string())?;
            let mut b = stated(Budget::new(), "source", request.source)?;
            for stage in path.stages() {
                b = b
                    .with(&stage.name, stage.transmission, &stage.source)
                    .map_err(|e| e.to_string())?;
            }
            let b = stated(b, "detector", request.detector)?;
            routes.push(RouteView {
                through: hops.iter().map(|h| h.instance.clone()).collect(),
                stages: b
                    .stages()
                    .iter()
                    .map(|st| StageView {
                        name: st.name.clone(),
                        transmission: st.transmission,
                        loss_db: db(st.transmission),
                        source: st.source.clone(),
                    })
                    .collect(),
                transmission: b.transmission(),
                loss_db: b.loss_db(),
            });
        }
        let best = routes
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.transmission.total_cmp(&b.1.transmission))
            .map(|(k, _)| k);
        let chosen = match leg.route {
            Some(k) if k < routes.len() => Some(k),
            _ => best,
        };
        event *= chosen.map_or(0.0, |k| routes[k].transmission);
        let index = |name: &str| ports.iter().position(|p| p == name);
        let circuit_power = match (index(&leg.from), index(&leg.to)) {
            (Some(p), Some(q)) => s.power(q, p),
            _ => 0.0,
        };
        legs.push(LegView {
            from: leg.from.clone(),
            to: leg.to.clone(),
            routes,
            chosen,
            more,
            circuit: circuit_power,
        });
    }
    let thresholds = [
        (
            Threshold::Varnava2008,
            "Varnava, Browne & Rudolph 2008",
            "10.1103/PhysRevLett.100.060502",
        ),
        (
            Threshold::Bartolucci2023,
            "Bartolucci et al. 2023",
            "10.1038/s41467-023-36493-1",
        ),
    ]
    .into_iter()
    .map(|(t, name, doi)| ThresholdView {
        name,
        doi,
        transmission: t.transmission(),
        loss_db: db(t.transmission()),
        assumptions: t.assumptions(),
    })
    .collect();
    Ok(BudgetView {
        photons: legs,
        event,
        event_db: (event > 0.0).then(|| db(event)),
        thresholds,
        seconds: started.elapsed().as_secs_f64(),
    })
}

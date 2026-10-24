//! A static job's `line` table: the conductors as a travelling-wave electrode, its quasi-TEM
//! index, impedance and loss at each frequency, and a modulator's electro-optic response through
//! it ([`crate::electro::line`], docs/methods/travelling-wave.md).
//!
//! The job is an `"electrostatic"` static job whose `[[task.conductor]]` rectangles are the
//! line's metal: `signal` (counted from 1, as the record names them `conductor k`) is the signal,
//! the others its grounds; their potentials don't enter. `mirrored: true` when the window holds
//! half the line, its left side (insulated, as by default) the line's symmetry plane through the
//! signal: the numbers are then the whole line's. The metal's `conductivity_s_per_m` and
//! `thickness_um` (by default the signal rectangle's height) give the conductor loss by Holloway
//! and Kuester's method, its edges `square` (the default) or `bevelled` (45°); `loss_tangent`
//! gives each medium's tan δ by its name (`Si`, `SiO2`, a crystal's, a region's), for the
//! dielectric loss. The run records the capacitances, the effective permittivity, the
//! quasi-static index and impedance, each medium's share of the electric energy and the line's
//! loss tangent, and at each of `frequencies_ghz` the RF index, Z₀ and the losses (dB/cm), the
//! conductor loss by Wheeler's incremental-inductance rule beside it where the metal is at
//! least twice its skin depth thick. The
//! quasi-TEM numbers hold up to about 100 GHz; a frequency above that is marked "beyond
//! quasi-TEM".
//!
//! `segmented` makes the line a segmented (T-rail) electrode, the conductors drawn the
//! cross-section through its rails: `rail` lists which `[[task.conductor]]`s are rails, each
//! with the conductor it hangs from by its stem (`{ conductor: 3, of: 1 }`, counted from 1; give
//! a rail its channel's potential for the field the electro-optic table takes), `rail_um` each
//! rail's length along the line and `period_um` the period. The other conductors are the
//! channel line, whose numbers the `line:` entries are. A period is the channel line's
//! (period − rail)/2, the line loaded by the rails for rail_um, and the other half
//! ([`crate::electro::line::Periodic`], the loaded part [`QuasiTem::loaded`]: the rails'
//! capacitance, the channels' inductance and current). The run records the loaded capacitance
//! and, at each frequency, the Bloch wave's index, Z_B and losses, and the index and impedance
//! with the gaps between rails loaded as the rails too (the bound a 2D section gives on the
//! rails' ends fringing across them); the response and the grid study then take the segmented
//! line.
//!
//! `response` (`length_mm`, `group_index`, and `source_ohm` and `load_ohm`, 50 Ω by default)
//! records the modulator's electro-optic response 20 log₁₀ m at each frequency, referred to
//! `reference_ghz` (1 GHz by default, as the papers measure it), and its 3 dB and 6 dB
//! bandwidths up to `up_to_ghz` (400 by default).
//!
//! `convergence: { tolerance: 0.01 }` (relative) checks the quasi-static index and impedance (a
//! segmented line's Bloch index and Re Z_B at the highest frequency) and the loss at the highest
//! frequency on grids twice and four times as coarse (`grids: 2` for one coarser);
//! `require: true` fails the run when one hasn't converged. A line's numbers are a
//! cross-section's, per unit length of a straight line: 2D numbers.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Deserialize;

use super::statics::StaticValue;
use super::{Event, task_error};
use crate::electro::Electrostatic;
use crate::electro::line::{
    DB_PER_NEPER, Drive, Edge, Metal, Periodic, QuasiTem, Wave, falls_to, response,
};
use crate::run::{Run, Stop};
use crate::{Error, Result};

/// The highest frequency quasi-TEM is taken to hold to, GHz.
const QUASI_TEM_GHZ: f64 = 100.0;

/// `line`: see the module's docs.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct LineSpec {
    /// The signal's conductor, counted from 1.
    signal: usize,
    /// Whether the window holds half the line, mirrored in its left side.
    #[serde(default)]
    mirrored: bool,
    /// The metal's conductivity, S/m.
    conductivity_s_per_m: f64,
    /// The metal's thickness, µm (by default the signal rectangle's height).
    thickness_um: Option<f64>,
    /// `"square"` (the default) or `"bevelled"` (45°) edges.
    edge: Option<String>,
    /// Each medium's loss tangent, by name.
    #[serde(default)]
    loss_tangent: BTreeMap<String, f64>,
    /// The frequencies, GHz.
    frequencies_ghz: Vec<f64>,
    /// A segmented (T-rail) electrode: its rails, their length and the period.
    segmented: Option<SegmentedSpec>,
    /// The electro-optic response of a modulator driven through the line.
    response: Option<ResponseSpec>,
    /// The grid study.
    convergence: Option<LineConvergence>,
}

/// `line.segmented`.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct SegmentedSpec {
    /// The rails, each a rectangle of the section and the conductor it hangs from.
    rail: Vec<RailSpec>,
    /// Each rail's length along the line, µm.
    rail_um: f64,
    /// The period along the line, µm.
    period_um: f64,
}

/// `line.segmented.rail`: a `[[task.conductor]]` that is a rail, and the conductor it hangs
/// from by its stem, both counted from 1.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct RailSpec {
    /// The rail's conductor, counted from 1.
    conductor: usize,
    /// The conductor it hangs from, counted from 1.
    of: usize,
}

/// `line.response`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ResponseSpec {
    /// The electrode's length, mm.
    length_mm: f64,
    /// The light's group index.
    group_index: f64,
    /// The source's resistance, Ω (50 by default).
    source_ohm: Option<f64>,
    /// The load's resistance, Ω (50 by default).
    load_ohm: Option<f64>,
    /// The frequency the response is referred to, GHz (1 by default).
    reference_ghz: Option<f64>,
    /// The highest frequency the bandwidths are looked for to, GHz (400 by default).
    up_to_ghz: Option<f64>,
}

/// `line.convergence`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct LineConvergence {
    /// The tolerance on each number's estimated relative error.
    tolerance: f64,
    /// How many grids, the task's among them: 2 or 3 (the default).
    grids: Option<usize>,
    /// Fail the run when a number's grid hasn't converged.
    require: Option<bool>,
}

fn err(reason: impl Into<String>) -> Error {
    task_error(format!("line: {}", reason.into()))
}

impl LineSpec {
    fn edge(&self) -> Result<Edge> {
        match self.edge.as_deref() {
            None | Some("square") => Ok(Edge::Square),
            Some("bevelled") => Ok(Edge::Bevelled),
            Some(other) => Err(err(format!(
                "edge = \"{other}\": \"square\" or \"bevelled\""
            ))),
        }
    }

    fn tangents(&self) -> Vec<(&str, f64)> {
        self.loss_tangent
            .iter()
            .map(|(m, t)| (m.as_str(), *t))
            .collect()
    }

    fn drive(&self) -> Result<Option<(Drive, f64, f64)>> {
        let Some(r) = self.response else {
            return Ok(None);
        };
        let reference = r.reference_ghz.unwrap_or(1.0);
        let up_to = r.up_to_ghz.unwrap_or(400.0);
        if !(reference.is_finite() && reference > 0.0 && up_to > reference && up_to.is_finite()) {
            return Err(err(format!(
                "response.reference_ghz must be above 0 and below up_to_ghz, got {reference} and \
                 {up_to}"
            )));
        }
        let drive = Drive::new(50.0, r.group_index, r.length_mm * 1e-3)
            .and_then(|d| d.with_ends(r.source_ohm.unwrap_or(50.0), r.load_ohm.unwrap_or(50.0)))
            .map_err(|e| err(e.to_string()))?;
        Ok(Some((drive, reference, up_to)))
    }

    fn top(&self) -> f64 {
        self.frequencies_ghz
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// Checks the table against the job's conductors (how many), their heights (µm) and the
/// section's media: the metal it gives.
pub(super) fn check(
    spec: &LineSpec,
    conductors: usize,
    heights: &[f64],
    media: &[String],
) -> Result<Metal> {
    if spec.signal == 0 || spec.signal > conductors {
        return Err(err(format!(
            "signal = {}: one of the {conductors} [[task.conductor]] rectangles, counted from 1",
            spec.signal
        )));
    }
    if conductors < 2 {
        return Err(err(
            "a line needs a signal and a ground among the conductors",
        ));
    }
    if let Some(m) = spec.loss_tangent.keys().find(|m| !media.contains(m)) {
        return Err(err(format!(
            "loss_tangent names {m}, which the section hasn't (it has {})",
            media.join(", ")
        )));
    }
    if spec.frequencies_ghz.is_empty()
        || spec
            .frequencies_ghz
            .iter()
            .any(|f| !(f.is_finite() && *f > 0.0))
    {
        return Err(err(
            "frequencies_ghz must list one frequency or more, each above 0",
        ));
    }
    for (m, t) in &spec.loss_tangent {
        if !(t.is_finite() && *t >= 0.0) {
            return Err(err(format!("loss_tangent {m} must be at least 0, got {t}")));
        }
    }
    if let Some(s) = &spec.segmented {
        if s.rail.is_empty() {
            return Err(err("segmented needs one rail or more"));
        }
        let rails: Vec<usize> = s.rail.iter().map(|r| r.conductor).collect();
        for (k, r) in s.rail.iter().enumerate() {
            let fine = |c: usize| c >= 1 && c <= conductors;
            if !(fine(r.conductor) && fine(r.of)) {
                return Err(err(format!(
                    "segmented.rail #{}: conductor = {} and of = {} must each be one of the \
                     {conductors} [[task.conductor]] rectangles, counted from 1",
                    k + 1,
                    r.conductor,
                    r.of
                )));
            }
            if r.conductor == spec.signal || rails.contains(&r.of) {
                return Err(err(format!(
                    "segmented.rail #{}: a rail is a conductor of its own, hanging from the \
                     signal or a ground that isn't a rail",
                    k + 1
                )));
            }
            if rails.iter().filter(|&&c| c == r.conductor).count() > 1 {
                return Err(err(format!(
                    "segmented.rail: conductor {} is listed twice",
                    r.conductor
                )));
            }
        }
        if !s.rail.iter().any(|r| r.of == spec.signal) {
            return Err(err("segmented needs a rail hanging from the signal"));
        }
        if conductors - rails.len() < 2 {
            return Err(err(
                "a segmented line needs a signal and a ground besides its rails",
            ));
        }
        if !(s.period_um.is_finite()
            && s.rail_um.is_finite()
            && s.rail_um > 0.0
            && s.rail_um < s.period_um)
        {
            return Err(err(format!(
                "segmented.rail_um must be above 0 and below period_um, got {} and {}",
                s.rail_um, s.period_um
            )));
        }
    }
    if let Some(c) = &spec.convergence {
        if !(c.tolerance.is_finite() && c.tolerance > 0.0) {
            return Err(err(format!(
                "convergence.tolerance must be above 0, got {}",
                c.tolerance
            )));
        }
        if !matches!(c.grids.unwrap_or(3), 2 | 3) {
            return Err(err("convergence.grids is 2 or 3"));
        }
    }
    spec.drive()?;
    let thickness = spec
        .thickness_um
        .unwrap_or_else(|| heights.get(spec.signal - 1).copied().unwrap_or(0.0));
    let edge = spec.edge()?;
    Metal::new(spec.conductivity_s_per_m, thickness)
        .map(|m| m.with_edge(edge))
        .map_err(|e| err(e.to_string()))
}

/// The line the table makes of `problem`: the conductors' quasi-TEM line, and a segmented
/// line's loaded part and period.
struct Line {
    channel: QuasiTem,
    segmented: Option<(QuasiTem, Periodic)>,
}

impl Line {
    fn new(spec: &LineSpec, problem: &Electrostatic) -> Result<Line> {
        let signal = spec.signal - 1;
        let rails: Vec<usize> = spec
            .segmented
            .iter()
            .flat_map(|s| s.rail.iter().map(|r| r.conductor - 1))
            .collect();
        // the channels: the conductors without the rails, the signal among them
        let channels = problem.without_conductors(&rails);
        let at = signal - rails.iter().filter(|&&k| k < signal).count();
        let channel = if spec.mirrored {
            QuasiTem::mirrored(&channels, at)?
        } else {
            QuasiTem::new(&channels, at)?
        };
        let segmented = match &spec.segmented {
            None => None,
            Some(s) => {
                let mut signals = vec![signal];
                signals.extend(
                    s.rail
                        .iter()
                        .filter(|r| r.of == spec.signal)
                        .map(|r| r.conductor - 1),
                );
                let loaded = channel.loaded(problem, &signals)?;
                let half = 0.5 * (s.period_um - s.rail_um);
                let period = Periodic::new(vec![
                    (channel.clone(), half),
                    (loaded.clone(), s.rail_um),
                    (channel.clone(), half),
                ])?;
                Some((loaded, period))
            }
        };
        Ok(Line { channel, segmented })
    }

    /// The wave the modulator sees at `frequency_hz`: the channel line's, or the segmented
    /// line's Bloch wave.
    fn wave(&self, frequency_hz: f64, metal: &Metal, tangents: &[(&str, f64)]) -> Result<Wave> {
        match &self.segmented {
            None => self.channel.wave(frequency_hz, metal, tangents),
            Some((_, p)) => p.bloch(frequency_hz, metal, tangents),
        }
    }

    /// The three numbers a grid study checks: the quasi-static index and impedance (a
    /// segmented line's Bloch index and Re Z_B at the highest frequency), and the loss (dB/cm)
    /// at the highest frequency.
    fn studied(&self, spec: &LineSpec, metal: &Metal) -> Result<[f64; 3]> {
        let w = self.wave(spec.top() * 1e9, metal, &spec.tangents())?;
        Ok(match self.segmented {
            None => [
                self.channel.index(),
                self.channel.impedance(),
                w.db_per_cm(),
            ],
            Some(_) => [w.index(), w.impedance.re, w.db_per_cm()],
        })
    }
}

/// The line on the run's grid, its numbers, and the grid study when the table asks for one
/// (`coarser(k)` the electrostatic problem on grids 2^k as coarse).
pub(super) fn run(
    spec: &LineSpec,
    metal: &Metal,
    problem: &Electrostatic,
    coarser: impl Fn(u32) -> Result<Electrostatic>,
    run: &mut Run,
    stop: &Stop,
) -> Result<Option<Vec<StaticValue>>> {
    let value = |name: String, value: f64, unit: &str| StaticValue {
        name,
        value,
        unit: unit.to_owned(),
    };
    let built = Line::new(spec, problem)?;
    let line = &built.channel;
    let tangents = spec.tangents();
    let mut values = vec![
        value("line: C".into(), line.capacitance(), "F/m"),
        value("line: C0 (vacuum)".into(), line.vacuum_capacitance(), "F/m"),
        value(
            "line: effective permittivity".into(),
            line.effective_permittivity(),
            "",
        ),
        value("line: n (quasi-static)".into(), line.index(), ""),
        value("line: Z0 (quasi-static)".into(), line.impedance(), "ohm"),
        value(
            "line: loss tangent (the media's, by energy)".into(),
            line.loss_tangent(&tangents)?,
            "",
        ),
        value("line: quasi-TEM up to".into(), QUASI_TEM_GHZ, "GHz"),
    ];
    for (m, q) in line.shares() {
        values.push(value(format!("line: {m}'s share of the energy"), *q, ""));
    }
    if let (Some((loaded, p)), Some(s)) = (&built.segmented, &spec.segmented) {
        values.extend([
            value(
                "segmented line: C where loaded".into(),
                loaded.capacitance(),
                "F/m",
            ),
            value("segmented line: period".into(), p.period_um(), "um"),
            value("segmented line: rail length".into(), s.rail_um, "um"),
            value(
                "segmented line: n (quasi-static, loaded as Shin et al.'s)".into(),
                (line.inductance()
                    * (loaded.capacitance() * s.rail_um
                        + line.capacitance() * (s.period_um - s.rail_um))
                    / s.period_um)
                    .sqrt()
                    * crate::units::SPEED_OF_LIGHT,
                "",
            ),
        ]);
    }
    let drive = spec.drive()?;
    let reference = match &drive {
        Some((d, f0, _)) => {
            let w = built.wave(f0 * 1e9, metal, &tangents)?;
            Some(response(w.gamma, w.impedance, f0 * 1e9, d))
        }
        None => None,
    };
    for &f in &spec.frequencies_ghz {
        let w = line.wave(f * 1e9, metal, &tangents)?;
        let at = if f > QUASI_TEM_GHZ {
            format!("{f} GHz (beyond quasi-TEM)")
        } else {
            format!("{f} GHz")
        };
        values.extend([
            value(format!("line at {at}: n_m"), w.index(), ""),
            value(format!("line at {at}: Re Z0"), w.impedance.re, "ohm"),
            value(
                format!("line at {at}: Im Z0 (e^jwt)"),
                w.impedance.im,
                "ohm",
            ),
            value(format!("line at {at}: loss"), w.db_per_cm(), "dB/cm"),
            value(
                format!("line at {at}: conductor loss"),
                w.conductor * DB_PER_NEPER / 100.0,
                "dB/cm",
            ),
            value(
                format!("line at {at}: dielectric loss"),
                w.dielectric * DB_PER_NEPER / 100.0,
                "dB/cm",
            ),
        ]);
        // Wheeler's rule beside it, where the metal is thick enough for it
        if let Ok(a) = line.wheeler_loss(f * 1e9, metal) {
            values.push(value(
                format!("line at {at}: conductor loss by Wheeler's rule"),
                a * DB_PER_NEPER / 100.0,
                "dB/cm",
            ));
        }
        if let Some((loaded, p)) = &built.segmented {
            let b = p.bloch(f * 1e9, metal, &tangents)?;
            let full = loaded.wave(f * 1e9, metal, &tangents)?;
            values.extend([
                value(format!("segmented line at {at}: n_B"), b.index(), ""),
                value(
                    format!("segmented line at {at}: Re Z_B"),
                    b.impedance.re,
                    "ohm",
                ),
                value(
                    format!("segmented line at {at}: Im Z_B (e^jwt)"),
                    b.impedance.im,
                    "ohm",
                ),
                value(
                    format!("segmented line at {at}: loss"),
                    b.db_per_cm(),
                    "dB/cm",
                ),
                value(
                    format!("segmented line at {at}: conductor loss"),
                    b.conductor * DB_PER_NEPER / 100.0,
                    "dB/cm",
                ),
                value(
                    format!("segmented line at {at}: dielectric loss"),
                    b.dielectric * DB_PER_NEPER / 100.0,
                    "dB/cm",
                ),
                value(
                    format!("segmented line at {at}: n_B with the gaps loaded too (2D bound)"),
                    full.index(),
                    "",
                ),
                value(
                    format!("segmented line at {at}: Re Z_B with the gaps loaded too (2D bound)"),
                    full.impedance.re,
                    "ohm",
                ),
            ]);
        }
        if let (Some((d, _, _)), Some(r0)) = (&drive, reference) {
            let w = built.wave(f * 1e9, metal, &tangents)?;
            let m = response(w.gamma, w.impedance, f * 1e9, d);
            values.push(value(
                format!("line at {at}: EO response"),
                20.0 * (m / r0).log10(),
                "dB",
            ));
        }
    }
    if let (Some((d, f0, up_to)), Some(r0)) = (&drive, reference) {
        let db = |f: f64| -> Result<f64> {
            let w = built.wave(f, metal, &tangents)?;
            Ok(20.0 * (response(w.gamma, w.impedance, f, d) / r0).log10())
        };
        for level in [3.0, 6.0] {
            let name = format!("line: EO {level} dB bandwidth");
            match falls_to(db, -level, f0 * 1e9, up_to * 1e9)? {
                Some(f) => values.push(value(name, f / 1e9, "GHz")),
                None => values.push(value(format!("{name} (above up_to_ghz)"), *up_to, "GHz")),
            }
        }
    }
    if let Some(c) = spec.convergence {
        let grids = c.grids.unwrap_or(3);
        let step = problem
            .grid()
            .x()
            .steps()
            .0
            .min(problem.grid().y().steps().0);
        let names = if spec.segmented.is_some() {
            [
                "segmented line n_B at the highest frequency",
                "segmented line Re Z_B at the highest frequency, ohm",
                "segmented line loss, dB/cm",
            ]
        } else {
            [
                "line n (quasi-static)",
                "line Z0 (quasi-static), ohm",
                "line loss, dB/cm",
            ]
        };
        let mut studies: Vec<crate::convergence::Study> = names
            .iter()
            .map(|n| crate::convergence::Study::new(n))
            .collect();
        for k in (1..grids as u32).rev() {
            if stop.reason().is_some() {
                return Ok(None);
            }
            let scale = f64::from(1u32 << k);
            let coarse = Line::new(spec, &coarser(k)?)?;
            for (s, v) in studies.iter_mut().zip(coarse.studied(spec, metal)?) {
                *s = s.clone().on(step * scale, v)?;
            }
        }
        let fine = built.studied(spec, metal)?;
        let mut failed = Vec::new();
        for (s, v) in studies.into_iter().zip(fine) {
            let check = s.on(step, v)?.check(c.tolerance * v.abs())?;
            if !check.converged() {
                failed.push(format!("{}: {}", check.quantity, check.reason));
            }
            run.record(&Event::GridConvergence {
                label: check.quantity.clone(),
                wavelength_um: 0.0,
                check,
            })?;
        }
        if c.require == Some(true) && !failed.is_empty() {
            return Err(Error::invalid(
                "grid convergence",
                format!(
                    "line.convergence requires the line's grids to converge: {}",
                    failed.join("; ")
                ),
            ));
        }
    }
    Ok(Some(values))
}

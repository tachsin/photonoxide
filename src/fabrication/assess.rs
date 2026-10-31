//! What the rules find in a design and its flow beyond the flow itself: each rule's check
//! ([`Check`]: resolution, aspect ratio, walls, selectivity, thermal budget, registration,
//! planarization, film thickness), each step's alternatives ranked with what each would gain or
//! cost ([`Ranked`]), and the accuracy of each fabricated dimension ([`Dimension`]): its
//! contributions from the processes that make it, each with its source or "not specified", and
//! their combination by a stated rule ([`super::rules::COMBINATION`]).

use serde::{Deserialize, Serialize};

use crate::geometry::{Region, Shape, Tolerance};
use crate::{Error, Result};

use super::flow::{Depth, Flow, Operation, Step, Tone};
use super::generate::{conformal, coverage, selectivity_over, thermal};
use super::library::{Process, StepType, process};
use super::measure::features;
use super::physical::Physical;
use super::rayleigh::{Optics, Reach};
use super::rules::{
    COMBINATION, example_non_uniformity, k1_limit, k1_typical, over_etch, registration, rie_lag,
};
use super::stated::{Capability, Quantity, Source, Stated, Unit, number};

/// Which rule a check applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Rule {
    /// A lithography's resolution against the layer's narrowest feature.
    Resolution,
    /// An etch's depth over the narrowest opening.
    AspectRatio,
    /// The design's walls against the etch's cited angles.
    Sidewall,
    /// The mask's and the film below's selectivity.
    Selectivity,
    /// A step's temperature against the films already there.
    Thermal,
    /// A layer's registration to the one below.
    Overlay,
    /// A flat surface under a patterned layer.
    Planarization,
    /// A film's thickness against what its process makes.
    Thickness,
}

/// What a check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Verdict {
    /// Within what the sources show.
    Within,
    /// Outside it, or at its edge: the text says what follows.
    Warning,
    /// A number it needs is not specified by any source in hand.
    NotSpecified,
}

/// One rule's check on one layer or step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Check {
    /// The rule.
    pub rule: Rule,
    /// The layer it is about, if one.
    pub layer: Option<String>,
    /// The step it is about (from 0), if one.
    pub step: Option<usize>,
    /// What it found.
    pub verdict: Verdict,
    /// In plain words, with the numbers and their sources.
    pub text: String,
}

/// A process that could do a step, ranked: what it would gain and cost against the chosen one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Ranked {
    /// The process's id.
    pub process: String,
    /// Its name.
    pub name: String,
    /// Whether it is the flow's choice.
    pub chosen: bool,
    /// Whether it can do the step for this design at all.
    pub feasible: bool,
    /// Where it is better than the chosen one.
    pub better: Vec<String>,
    /// Where it is worse, or why it can't do the step.
    pub worse: Vec<String>,
    /// What choosing it would cost or gain in feature size and accuracy, in a phrase.
    pub cost: String,
}

/// A step's processes, best first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Ranking {
    /// The step (from 0).
    pub step: usize,
    /// The chosen process and its alternatives, ranked.
    pub ranked: Vec<Ranked>,
}

/// What kind of dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Kind {
    /// A film's thickness.
    Thickness,
    /// A line's width at its top.
    Width,
    /// A line's width at the etch's floor.
    FloorWidth,
    /// An etch's depth.
    Depth,
    /// The walls' angle.
    Sidewall,
    /// A layer's position against the layer it is aligned to.
    Placement,
}

/// One contribution to a dimension's uncertainty: a bound either way, with its source, or not
/// specified.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Contribution {
    /// What it is, in plain words.
    pub what: String,
    /// The bound, ± in the dimension's unit; `None` when not specified.
    pub bound: Option<f64>,
    /// Where the number comes from.
    pub source: Option<Source>,
    /// The conditions the source states it for, and how it was carried to this dimension.
    pub conditions: String,
}

/// A fabricated dimension and how accurately the flow's processes make it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Dimension {
    /// What it is, e.g. "the narrowest line of Si".
    pub what: String,
    /// The layer.
    pub layer: String,
    /// Its kind.
    pub kind: Kind,
    /// The design's value.
    pub nominal: f64,
    /// Its unit (nm or degrees).
    pub unit: Unit,
    /// The contributions.
    pub contributions: Vec<Contribution>,
    /// The stated contributions combined as the root of the sum of their squares, ±.
    pub combined: Option<f64>,
    /// Their sum, ±: every one at its bound at once.
    pub worst: Option<f64>,
    /// Whether every contribution is stated.
    pub complete: bool,
    /// The read-out in plain words: "the 500 nm line is made to ± z nm".
    pub text: String,
}

impl Dimension {
    fn new(what: String, layer: &str, kind: Kind, nominal: f64, unit: Unit) -> Dimension {
        Dimension {
            what,
            layer: layer.into(),
            kind,
            nominal,
            unit,
            contributions: Vec::new(),
            combined: None,
            worst: None,
            complete: true,
            text: String::new(),
        }
    }

    fn add(&mut self, what: &str, bound: Option<f64>, source: Option<&Source>, conditions: String) {
        self.contributions.push(Contribution {
            what: what.into(),
            bound,
            source: source.cloned(),
            conditions,
        });
    }

    /// Combines the contributions and writes the read-out.
    fn close(mut self) -> Dimension {
        let stated: Vec<f64> = self.contributions.iter().filter_map(|c| c.bound).collect();
        self.complete = self.contributions.iter().all(|c| c.bound.is_some());
        let unit = |v: f64| match self.unit {
            Unit::Degree => format!("{}°", number(v)),
            _ => format!("{} nm", number(v)),
        };
        if !stated.is_empty() {
            self.combined = Some(stated.iter().map(|b| b * b).sum::<f64>().sqrt());
            self.worst = Some(stated.iter().sum());
        }
        self.text = match (self.combined, self.worst) {
            (Some(c), Some(w)) if self.complete => format!(
                "{}, {}: made to ± {} (at worst ± {})",
                self.what,
                unit(self.nominal),
                unit(c),
                unit(w)
            ),
            (Some(c), Some(w)) => format!(
                "{}, {}: made to ± {} from what the sources state (at worst ± {}); not \
                 specified: {}",
                self.what,
                unit(self.nominal),
                unit(c),
                unit(w),
                self.contributions
                    .iter()
                    .filter(|c| c.bound.is_none())
                    .map(|c| c.what.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => format!(
                "{}, {}: not specified by the sources in hand ({})",
                self.what,
                unit(self.nominal),
                self.contributions
                    .iter()
                    .map(|c| c.what.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        self
    }
}

/// What the rules find in a design and its flow.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Assessment {
    /// Each rule's checks.
    pub checks: Vec<Check>,
    /// Each step's processes, ranked.
    pub ranking: Vec<Ranking>,
    /// Each fabricated dimension's accuracy.
    pub accuracy: Vec<Dimension>,
    /// How the contributions are combined, with its sources.
    pub combination: String,
    /// The optics of the flow's projection lithographies.
    pub optics: Vec<Optics>,
}

fn nm(v_um: f64) -> String {
    format!("{} nm", number(v_um * 1000.0))
}

fn cited(c: &Capability) -> String {
    match c.value.source() {
        Some(s) => format!(
            "{} ({}; {}, {})",
            c.value, c.conditions, s.reference, s.location
        ),
        None => format!("not specified ({})", c.conditions),
    }
}

/// A stated value in nm, its larger end (a bound either way), or `None`.
fn nm_of(s: &Stated) -> Option<f64> {
    let v = s.high().or(s.low())?;
    match s.unit()? {
        Unit::Nm => Some(v),
        Unit::Um => Some(v * 1000.0),
        _ => None,
    }
}

/// The largest stated length of `p`'s capabilities of `q`, nm, with the capability.
fn largest_nm<'a>(p: &'a Process, q: &'a Quantity) -> Option<(f64, &'a Capability)> {
    p.capabilities_of(q)
        .filter_map(|c| nm_of(&c.value).map(|v| (v, c)))
        .fold(None, |best, (v, c)| match best {
            Some((b, _)) if b >= v => best,
            _ => Some((v, c)),
        })
}

/// The largest stated length of `p`'s uniformities, nm.
fn uniformity(p: &Process) -> Option<(f64, &Capability)> {
    largest_nm(p, &Quantity::Uniformity)
}

/// The smallest feature `p` resolves, nm: its first stated minimum feature's lower end (the
/// generator's choice reads the same).
fn min_feature(p: &Process) -> Option<(f64, &Capability)> {
    let c = p
        .capabilities_of(&Quantity::MinFeature)
        .find(|c| c.value.is_stated())?;
    let v = c.value.low().or(c.value.high())?;
    let nm = match c.value.unit()? {
        Unit::Nm => v,
        Unit::Um => v * 1000.0,
        _ => return None,
    };
    Some((nm, c))
}

/// A stated percentage of `p`'s capabilities of `q`, with the capability.
fn percent<'a>(p: &'a Process, q: &'a Quantity) -> Option<(f64, &'a Capability)> {
    p.capabilities_of(q)
        .find(|c| c.value.unit() == Some(Unit::Percent) && c.value.is_stated())
        .and_then(|c| c.value.high().or(c.value.low()).map(|v| (v, c)))
}

/// The cited sidewall ranges of `p`, degrees.
fn walls(p: &Process) -> Vec<(f64, f64, &Capability)> {
    p.capabilities_of(&Quantity::Sidewall)
        .filter_map(|c| match (c.value.low(), c.value.high()) {
            (Some(lo), Some(hi)) => Some((lo, hi, c)),
            _ => None,
        })
        .collect()
}

/// The y-intervals `shapes` cover on the line x = `x0`, µm, sorted.
fn intervals_at(shapes: &[Shape], x0: f64) -> Vec<(f64, f64)> {
    let t = Tolerance::default();
    let mut out = Vec::new();
    for s in shapes {
        let p = s.polygons(t);
        for c in &p.contours {
            let mut ys = Vec::new();
            for ring in std::iter::once(&c.outer).chain(c.holes.iter()) {
                let n = ring.len();
                for k in 0..n {
                    let (a, b) = (&ring[k], &ring[(k + 1) % n]);
                    let (xa, ya, xb, yb) = (a.x.to_um(), a.y.to_um(), b.x.to_um(), b.y.to_um());
                    if (xa <= x0 && xb > x0) || (xb <= x0 && xa > x0) {
                        ys.push(ya + (x0 - xa) / (xb - xa) * (yb - ya));
                    }
                }
            }
            ys.sort_by(f64::total_cmp);
            for pair in ys.chunks(2) {
                if let [lo, hi] = pair {
                    out.push((*lo, *hi));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The smallest distance, µm, between an edge of `a`'s intervals and one of `b`'s on the line
/// x = `x0`: how far a layer may shift before an edge of it meets an edge of the other.
fn clearance(a: &[Shape], b: &[Shape], x0: f64) -> Option<f64> {
    let ea: Vec<f64> = intervals_at(a, x0)
        .iter()
        .flat_map(|i| [i.0, i.1])
        .collect();
    let eb: Vec<f64> = intervals_at(b, x0)
        .iter()
        .flat_map(|i| [i.0, i.1])
        .collect();
    ea.iter()
        .flat_map(|x| eb.iter().map(move |y| (x - y).abs()))
        .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
}

/// The middle of the design's shapes along x, µm: where the placement is read.
fn middle_x(design: &Physical) -> f64 {
    let s = design.structure();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for l in s.stack().layers() {
        for shape in s.shapes(&l.name) {
            let b = shape.bounds();
            lo = lo.min(b.min.x.to_um());
            hi = hi.max(b.max.x.to_um());
        }
    }
    if lo.is_finite() { 0.5 * (lo + hi) } else { 0.0 }
}

/// A layer's index in the stack.
fn index(design: &Physical, layer: &str) -> Option<usize> {
    design
        .structure()
        .stack()
        .layers()
        .iter()
        .position(|l| l.name == layer)
}

/// What a patterned layer was made with: its lithography step, its etch or lift-off step.
struct Patterned<'a> {
    layer: String,
    litho: (usize, &'a Step),
    transfer: (usize, &'a Step),
}

fn patterned(flow: &Flow) -> Vec<Patterned<'_>> {
    let mut out = Vec::new();
    let mut last: Option<(usize, &Step)> = None;
    for (k, s) in flow.steps.iter().enumerate() {
        match &s.operation {
            Operation::Pattern { .. } => last = Some((k, s)),
            Operation::Etch { layer, .. } | Operation::LiftOff { layer } => {
                if let Some(l) = last.take() {
                    out.push(Patterned {
                        layer: layer.clone(),
                        litho: l,
                        transfer: (k, s),
                    });
                }
            }
            _ => {}
        }
    }
    out
}

fn must(id: &str) -> Result<&'static Process> {
    process(id).ok_or_else(|| Error::invalid("fabrication", format!("no process {id}")))
}

/// What the rules find in `design` and its `flow`.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a flow naming a process the library hasn't, or a layer the
/// design hasn't.
pub fn assess(design: &Physical, flow: &Flow) -> Result<Assessment> {
    let mut a = Assessment {
        checks: Vec::new(),
        ranking: Vec::new(),
        accuracy: Vec::new(),
        combination: COMBINATION.into(),
        optics: Vec::new(),
    };
    let s = design.structure();
    let layers = s.stack().layers();
    let films = design.layers();
    let x0 = middle_x(design);
    let mut narrowest: Vec<(usize, f64)> = Vec::new();
    // the wafer's layers' thickness
    let mut wafer_layers: Vec<String> = Vec::new();
    for step in &flow.steps {
        if let Operation::Wafer { layers: laid, .. } = &step.operation {
            let p = must(&step.process)?;
            let spread = largest_nm(p, &Quantity::Uniformity);
            for (l, f) in laid {
                wafer_layers.push(l.name.clone());
                if f.name == "Si" || f.name == "LiNbO3" {
                    let mut d = Dimension::new(
                        format!("the {} film's thickness", f.name),
                        &l.name,
                        Kind::Thickness,
                        l.thickness.to_um() * 1000.0,
                        Unit::Nm,
                    );
                    match spread {
                        Some((v, c)) => d.add(
                            "the wafer's film thickness spread",
                            Some(v),
                            c.value.source(),
                            format!(
                                "{} ({}), read as a bound either way; the largest the \
                                 wafer's sources state",
                                c.value, c.conditions
                            ),
                        ),
                        None => d.add(
                            "the wafer's film thickness spread",
                            None,
                            None,
                            format!("no source in hand states it for {}", p.name),
                        ),
                    }
                    a.accuracy.push(d.close());
                }
            }
        }
    }
    // each patterned layer
    for pat in patterned(flow) {
        let (lk, lstep) = pat.litho;
        let (tk, tstep) = pat.transfer;
        let litho = must(&lstep.process)?;
        let transfer = must(&tstep.process)?;
        let Some(i) = index(design, &pat.layer) else {
            continue;
        };
        let layer = &layers[i];
        let (mask, tone) = match &lstep.operation {
            Operation::Pattern { mask, tone, .. } => (mask.clone(), *tone),
            _ => continue,
        };
        let f = features(&mask)?;
        narrowest.push((lk, f.smallest_um()));
        let line_nm = f.line_um * 1000.0;
        // resolution
        let smallest = f.smallest_um();
        if let Some(o) = Optics::of(litho) {
            let k1 = o.k1(smallest * 1000.0);
            a.checks.push(Check {
                rule: Rule::Resolution,
                layer: Some(pat.layer.clone()),
                step: Some(lk),
                verdict: if o.reach(smallest * 1000.0) == Reach::Typical {
                    Verdict::Within
                } else {
                    Verdict::Warning
                },
                text: format!(
                    "{}: the narrowest feature, {}, asks {} ({} nm, NA {} to {}) for k₁ = {} \
                     at its highest NA; k₁ is typically {} and at least {} (campbell-2008, Eq. \
                     7.18, p. 183): it prints {} down to {}, {} at the limit",
                    pat.layer,
                    nm(smallest),
                    litho.name,
                    number(o.wavelength_nm),
                    number(o.na[0]),
                    number(o.na[1]),
                    number(k1),
                    k1_typical(),
                    k1_limit(),
                    "as a matter of course",
                    nm(o.typical_nm / 1000.0),
                    nm(o.limit_nm / 1000.0)
                ),
            });
            if !a.optics.iter().any(|x| x.process == o.process) {
                a.optics.push(o);
            }
        } else {
            let min = min_feature(litho);
            a.checks.push(Check {
                rule: Rule::Resolution,
                layer: Some(pat.layer.clone()),
                step: Some(lk),
                verdict: if min.is_some() {
                    Verdict::Within
                } else {
                    Verdict::NotSpecified
                },
                text: match min {
                    Some((_, c)) => format!(
                        "{}: the narrowest feature, {}, against {}'s cited minimum feature, {}",
                        pat.layer,
                        nm(smallest),
                        litho.name,
                        cited(c)
                    ),
                    None => format!(
                        "{}: {}'s resolution is not specified by the sources in hand",
                        pat.layer, litho.name
                    ),
                },
            });
        }
        // the line's width at its top: the lithography's linewidth control, and the etch's
        let mut width = Dimension::new(
            format!("the narrowest line of {}", pat.layer),
            &pat.layer,
            Kind::Width,
            line_nm,
            Unit::Nm,
        );
        let cd = litho
            .capabilities_of(&Quantity::Uniformity)
            .find(|c| c.value.is_stated());
        match cd.and_then(|c| nm_of(&c.value).map(|v| (v, c))) {
            Some((v, c)) => width.add(
                "the lithography's linewidth control",
                Some(v),
                c.value.source(),
                format!("{} ({})", c.value, c.conditions),
            ),
            None => width.add(
                "the lithography's linewidth control",
                None,
                None,
                format!("no source in hand states it for {}", litho.name),
            ),
        }
        if matches!(tstep.operation, Operation::Etch { .. }) {
            width.add(
                "the etch's linewidth change",
                None,
                None,
                format!("no source in hand states it for {}", transfer.name),
            );
        }
        a.accuracy.push(width.close());
        // overlay: this layer against the patterned layer below it
        let below_pattern = (0..i)
            .rev()
            .find(|&j| !s.shapes(&layers[j].name).is_empty() && j < i);
        if let Some(j) = below_pattern {
            let below_name = &layers[j].name;
            let margin = clearance(&mask, s.shapes(below_name), x0);
            let overlay = largest_nm(litho, &Quantity::Overlay);
            let mut d = Dimension::new(
                format!("{}'s place against {}", pat.layer, below_name),
                &pat.layer,
                Kind::Placement,
                margin.map_or(0.0, |m| m * 1000.0),
                Unit::Nm,
            );
            match overlay {
                Some((v, c)) => d.add(
                    "the lithography's overlay",
                    Some(v),
                    c.value.source(),
                    format!("{} ({})", c.value, c.conditions),
                ),
                None => d.add(
                    "the lithography's overlay",
                    None,
                    None,
                    format!(
                        "no source in hand states {}'s overlay{}",
                        litho.name,
                        litho
                            .facts
                            .iter()
                            .find(|f| f.text.contains("operator"))
                            .map_or(String::new(), |f| format!(
                                ": {} ({}, {})",
                                f.text, f.source.reference, f.source.location
                            ))
                    ),
                ),
            }
            let mut d = d.close();
            d.what = format!(
                "the clearance between {}'s and {}'s edges",
                pat.layer, below_name
            );
            d.text = d.text.replacen(
                &format!("{}'s place against {}", pat.layer, below_name),
                &d.what,
                1,
            );
            let (verdict, text) = match (margin, overlay) {
                (Some(m), Some((v, c))) => {
                    let reg = registration();
                    let third = reg.high().unwrap_or(1.0 / 3.0) * smallest * 1000.0;
                    if v / 1000.0 <= m && v <= third + 1e-9 {
                        (
                            Verdict::Within,
                            format!(
                                "{}: its edges sit {} from {}'s at the cut x = {} µm; {}'s \
                                 overlay, {}, is within that, and within a third of the \
                                 layer's narrowest feature ({}; registration a quarter to a \
                                 third of the resolution, campbell-2008, p. 194)",
                                pat.layer,
                                nm(m),
                                below_name,
                                number(x0),
                                litho.name,
                                cited(c),
                                nm(smallest)
                            ),
                        )
                    } else {
                        (
                            Verdict::Warning,
                            format!(
                                "{}: its edges sit {} from {}'s at the cut x = {} µm; {}'s \
                                 overlay, {}, {}",
                                pat.layer,
                                nm(m),
                                below_name,
                                number(x0),
                                litho.name,
                                cited(c),
                                if v / 1000.0 > m {
                                    "could move an edge onto the other layer's".to_string()
                                } else {
                                    format!(
                                        "is above a third of the layer's narrowest feature, {} \
                                         (campbell-2008, p. 194)",
                                        nm(third / 1000.0)
                                    )
                                }
                            ),
                        )
                    }
                }
                (Some(m), None) => (
                    Verdict::NotSpecified,
                    format!(
                        "{}: its edges sit {} from {}'s at the cut x = {} µm; {}'s overlay is \
                         not specified by the sources in hand",
                        pat.layer,
                        nm(m),
                        below_name,
                        number(x0),
                        litho.name
                    ),
                ),
                (None, _) => (
                    Verdict::NotSpecified,
                    format!(
                        "{}: no edge of it meets the cut x = {} µm with one of {}",
                        pat.layer,
                        number(x0),
                        below_name
                    ),
                ),
            };
            a.checks.push(Check {
                rule: Rule::Overlay,
                layer: Some(pat.layer.clone()),
                step: Some(lk),
                verdict,
                text,
            });
            if margin.is_some() {
                a.accuracy.push(d);
            }
        }
        // the etch: depth, walls, aspect ratio, selectivity
        if let Operation::Etch {
            depth, sidewall, ..
        } = &tstep.operation
        {
            let deg = sidewall.to_degrees();
            let thickness_nm = layer.thickness.to_um() * 1000.0;
            let depth_nm = match depth {
                Depth::ToStop => thickness_nm,
                Depth::Timed(d) => d.to_um() * 1000.0,
            };
            // walls
            let cited_walls = walls(transfer);
            let mut wall = Dimension::new(
                format!("the walls of {}", pat.layer),
                &pat.layer,
                Kind::Sidewall,
                deg,
                Unit::Degree,
            );
            let mut floor = Dimension::new(
                format!("the narrowest line of {} at its floor", pat.layer),
                &pat.layer,
                Kind::FloorWidth,
                line_nm + 2.0 * depth_nm / sidewall.tan(),
                Unit::Nm,
            );
            if cited_walls.is_empty() {
                wall.add(
                    "the etch's wall angle",
                    None,
                    None,
                    format!("no source in hand states {}'s walls", transfer.name),
                );
                floor.add(
                    "the walls' angle, carried to the floor",
                    None,
                    None,
                    "the etch's walls are not specified".into(),
                );
            } else {
                let lo = cited_walls
                    .iter()
                    .map(|w| w.0)
                    .fold(f64::INFINITY, f64::min);
                let hi = cited_walls
                    .iter()
                    .map(|w| w.1)
                    .fold(f64::NEG_INFINITY, f64::max);
                let c = cited_walls[0].2;
                let off = (lo - deg).abs().max((hi - deg).abs());
                wall.add(
                    "the etch's cited wall angles",
                    Some(off),
                    c.value.source(),
                    format!(
                        "{} to {}° across its sources, against the design's {}°",
                        number(lo),
                        number(hi),
                        number(deg)
                    ),
                );
                let cot = |d: f64| 1.0 / d.to_radians().tan();
                let shift = |d: f64| 2.0 * depth_nm * (cot(d) - cot(deg));
                let bound = shift(lo).abs().max(shift(hi).abs());
                floor.add(
                    "the walls' angle, carried to the floor",
                    Some(bound),
                    c.value.source(),
                    format!(
                        "two walls {} deep at {} to {}° instead of {}°: 2·d·(cot θ − cot θ₀)",
                        nm(depth_nm / 1000.0),
                        number(lo),
                        number(hi),
                        number(deg)
                    ),
                );
                let inside = deg >= lo - 1e-9 && deg <= hi + 1e-9;
                a.checks.push(Check {
                    rule: Rule::Sidewall,
                    layer: Some(pat.layer.clone()),
                    step: Some(tk),
                    verdict: if inside {
                        Verdict::Within
                    } else {
                        Verdict::Warning
                    },
                    text: if inside {
                        format!(
                            "{}: the design's walls at {}° lie within {}'s cited {} to {}°",
                            pat.layer,
                            number(deg),
                            transfer.name,
                            number(lo),
                            number(hi)
                        )
                    } else {
                        let near = if (lo - deg).abs() < (hi - deg).abs() {
                            lo
                        } else {
                            hi
                        };
                        format!(
                            "{}: the design's walls at {}° are outside {}'s cited {} to {}°; at \
                             its {}°, the narrowest line's floor would be {} {} than drawn \
                             (2·d·(cot θ − cot θ₀), d = {})",
                            pat.layer,
                            number(deg),
                            transfer.name,
                            number(lo),
                            number(hi),
                            number(near),
                            nm(shift(near).abs() / 1000.0),
                            if shift(near) > 0.0 {
                                "wider"
                            } else {
                                "narrower"
                            },
                            nm(depth_nm / 1000.0)
                        )
                    },
                });
            }
            if let Some(w) = a
                .accuracy
                .iter()
                .find(|d| d.kind == Kind::Width && d.layer == pat.layer)
            {
                let mut all = w.contributions.clone();
                all.append(&mut floor.contributions);
                floor.contributions = all;
            }
            a.accuracy.push(wall.close());
            a.accuracy.push(floor.close());
            // depth
            match depth {
                Depth::Timed(d) => {
                    let mut dd = Dimension::new(
                        format!("the etch depth into {}", pat.layer),
                        &pat.layer,
                        Kind::Depth,
                        d.to_um() * 1000.0,
                        Unit::Nm,
                    );
                    match largest_nm(transfer, &Quantity::Uniformity) {
                        Some((v, c)) => dd.add(
                            "the etch's depth variation",
                            Some(v),
                            c.value.source(),
                            format!(
                                "{} ({}), the largest its sources state, read as a bound \
                                 either way",
                                c.value, c.conditions
                            ),
                        ),
                        None => {
                            let ex = example_non_uniformity();
                            let pct = ex.low().unwrap_or(0.0);
                            dd.add(
                                "the etch's rate non-uniformity",
                                Some(pct / 100.0 * d.to_um() * 1000.0),
                                ex.source(),
                                format!(
                                    "{}'s own is not stated: the textbook's example, 'say 5 %', \
                                     of the {} depth",
                                    transfer.name,
                                    nm(d.to_um())
                                ),
                            );
                        }
                    }
                    a.accuracy.push(dd.close());
                }
                Depth::ToStop => {
                    // the layer below loses the over-etch over the selectivity
                    let below_name = if i == 0 {
                        "the substrate".to_string()
                    } else {
                        layers[i - 1].name.clone()
                    };
                    let below_film = if i == 0 {
                        design.substrate().name.clone()
                    } else {
                        films[i - 1].0.name.clone()
                    };
                    let oe = over_etch();
                    match selectivity_over(transfer, &below_film) {
                        Some(c) => {
                            let sel = c.value.low().or(c.value.high()).unwrap_or(1.0);
                            let loss = oe.high().unwrap_or(0.0) / 100.0 * thickness_nm / sel;
                            a.checks.push(Check {
                                rule: Rule::Selectivity,
                                layer: Some(pat.layer.clone()),
                                step: Some(tk),
                                verdict: Verdict::Within,
                                text: format!(
                                    "{}: the etch is ended on {} ({below_film}); its over-etch, \
                                     {} of the film's etch (franssila-2010, pp. 138–139), at a \
                                     selectivity of at least {} ({}), takes at most {} off it \
                                     where the film is etched away",
                                    pat.layer,
                                    below_name,
                                    oe,
                                    number(sel),
                                    cited(c),
                                    nm(loss / 1000.0)
                                ),
                            });
                            if i > 0 {
                                let mut bd = Dimension::new(
                                    format!("{} beside {}'s shapes", below_name, pat.layer),
                                    &below_name,
                                    Kind::Thickness,
                                    layers[i - 1].thickness.to_um() * 1000.0,
                                    Unit::Nm,
                                );
                                bd.add(
                                    "the over-etch's loss",
                                    Some(loss),
                                    c.value.source(),
                                    format!(
                                        "{} of the {} film's etch over a selectivity of {} \
                                         ({}); the over-etch from franssila-2010, pp. 138–139",
                                        oe,
                                        nm(thickness_nm / 1000.0),
                                        number(sel),
                                        c.conditions
                                    ),
                                );
                                a.accuracy.push(bd.close());
                            }
                        }
                        None => a.checks.push(Check {
                            rule: Rule::Selectivity,
                            layer: Some(pat.layer.clone()),
                            step: Some(tk),
                            verdict: Verdict::NotSpecified,
                            text: format!(
                                "{}: the etch stops on {} ({below_film}), as its source says, \
                                 but no source in hand gives the selectivity, so how much of \
                                 it the over-etch takes is not specified",
                                pat.layer, below_name
                            ),
                        }),
                    }
                }
            }
            // aspect ratio
            if tone == Tone::Keep
                && let Some(gap) = f.gap_um
            {
                let ar = depth_nm / (gap * 1000.0);
                let lag = rie_lag();
                let lag_at = lag.low().unwrap_or(2.0);
                let limit = transfer
                    .capabilities_of(&Quantity::AspectRatio)
                    .find(|c| c.value.is_stated());
                let (verdict, text) = match limit {
                    Some(c) if ar > c.value.high().unwrap_or(f64::INFINITY) => (
                        Verdict::Warning,
                        format!(
                            "{}: the narrowest opening, {}, etched {} deep, has an aspect ratio \
                             of {}:1, beyond {}'s {}",
                            pat.layer,
                            nm(gap),
                            nm(depth_nm / 1000.0),
                            number(ar),
                            transfer.name,
                            cited(c)
                        ),
                    ),
                    _ if ar >= lag_at => (
                        Verdict::Warning,
                        format!(
                            "{}: the narrowest opening, {}, etched {} deep, has an aspect ratio \
                             of {}:1; around 2:1 narrow openings etch slower than wide ones (RIE \
                             lag; franssila-2010, Section 21.7.1, p. 266): {}",
                            pat.layer,
                            nm(gap),
                            nm(depth_nm / 1000.0),
                            number(ar),
                            if matches!(depth, Depth::Timed(_)) {
                                "a timed depth comes out shallower there"
                            } else {
                                "reaching the stop there needs a longer over-etch"
                            }
                        ),
                    ),
                    _ => (
                        Verdict::Within,
                        format!(
                            "{}: the narrowest opening, {}, etched {} deep, has an aspect ratio \
                             of {}:1, below the 2:1 around which narrow openings start to etch \
                             slower (franssila-2010, Section 21.7.1, p. 266)",
                            pat.layer,
                            nm(gap),
                            nm(depth_nm / 1000.0),
                            number(ar)
                        ),
                    ),
                };
                a.checks.push(Check {
                    rule: Rule::AspectRatio,
                    layer: Some(pat.layer.clone()),
                    step: Some(tk),
                    verdict,
                    text,
                });
            }
            // the mask
            if let Some(resist) = &litho.resist {
                let m = transfer.masks.iter().find(|m| &m.resist == resist);
                let text = match m
                    .and_then(|m| m.selectivity.low().or(m.selectivity.high()).map(|s| (m, s)))
                {
                    Some((m, sel)) => format!(
                        "{}: the {} etch through {resist} at a selectivity of {} ({}) needs at \
                         least {} of it",
                        pat.layer,
                        nm(depth_nm / 1000.0),
                        m.selectivity,
                        m.selectivity.source().map_or(String::new(), |s| format!(
                            "{}, {}",
                            s.reference, s.location
                        )),
                        nm(depth_nm / sel / 1000.0)
                    ),
                    None => format!(
                        "{}: no source in hand gives {}'s selectivity over {resist}",
                        pat.layer, transfer.name
                    ),
                };
                a.checks.push(Check {
                    rule: Rule::Selectivity,
                    layer: Some(pat.layer.clone()),
                    step: Some(lk),
                    verdict: if m.is_some_and(|m| m.selectivity.is_stated()) {
                        Verdict::Within
                    } else {
                        Verdict::NotSpecified
                    },
                    text,
                });
            }
        }
    }
    // deposited layers: thickness, polish, step coverage, thickness limits; thermal budgets
    let mut present: Vec<String> = vec![design.substrate().name.clone()];
    for (k, step) in flow.steps.iter().enumerate() {
        let p = must(&step.process)?;
        if let Some((t, c)) = p.temperature() {
            let check = thermal(p, &present);
            let (verdict, text) = if !check.violations.is_empty() {
                (
                    Verdict::Warning,
                    format!(
                        "step {}, {}: {}",
                        k + 1,
                        p.name,
                        check.violations.join("; ")
                    ),
                )
            } else if !check.unspecified.is_empty() {
                (
                    Verdict::NotSpecified,
                    format!(
                        "step {}, {}, reaches {} °C ({}): {}",
                        k + 1,
                        p.name,
                        number(t),
                        cited(c),
                        check.unspecified.join("; ")
                    ),
                )
            } else {
                (
                    Verdict::Within,
                    format!(
                        "step {}, {}, reaches {} °C ({}); every film already there is shown to \
                         survive it: {}",
                        k + 1,
                        p.name,
                        number(t),
                        cited(c),
                        check.shown.join(", ")
                    ),
                )
            };
            a.checks.push(Check {
                rule: Rule::Thermal,
                layer: None,
                step: Some(k),
                verdict,
                text,
            });
        }
        match &step.operation {
            Operation::Wafer { layers: laid, .. } => {
                for (_, f) in laid {
                    if !present.contains(&f.name) {
                        present.push(f.name.clone());
                    }
                }
            }
            Operation::Deposit {
                film, layers: laid, ..
            } => {
                if !present.contains(&film.name) {
                    present.push(film.name.clone());
                }
                let polish = flow.steps.get(k + 1).and_then(|n| match n.operation {
                    Operation::Planarize { removed } => Some((k + 1, removed)),
                    _ => None,
                });
                for l in laid {
                    let mut d = Dimension::new(
                        format!("{}'s thickness", l.name),
                        &l.name,
                        Kind::Thickness,
                        l.thickness.to_um() * 1000.0,
                        Unit::Nm,
                    );
                    match largest_nm(p, &Quantity::Uniformity) {
                        Some((v, c)) => d.add(
                            "the deposition's thickness spread",
                            Some(v),
                            c.value.source(),
                            format!("{} ({})", c.value, c.conditions),
                        ),
                        None => d.add(
                            "the deposition's thickness spread",
                            None,
                            None,
                            format!("no source in hand states it for {}", p.name),
                        ),
                    }
                    if let Some((pk, removed)) = polish {
                        let cmp = must(&flow.steps[pk].process)?;
                        if let Some((v, c)) = largest_nm(cmp, &Quantity::Uniformity) {
                            d.add(
                                "the polish's flatness",
                                Some(v),
                                c.value.source(),
                                format!("{} ({})", c.value, c.conditions),
                            );
                        }
                        if let Some((pct, c)) = percent(cmp, &Quantity::Uniformity) {
                            let r = removed.to_um() * 1000.0;
                            d.add(
                                "the polish's removal across the wafer",
                                Some(pct / 100.0 * r),
                                c.value.source(),
                                format!(
                                    "{} ({}) of the {} it removes",
                                    c.value,
                                    c.conditions,
                                    nm(r / 1000.0)
                                ),
                            );
                        }
                    }
                    // a cited thickness limit of the process
                    if let Some(c) = p.capabilities_of(&Quantity::Thickness).find(|c| {
                        matches!(c.value, Stated::AtMost { .. })
                            && c.conditions.contains("nitride")
                                == matches!(film.name.as_str(), "Si3N4" | "SiN")
                    }) && let Some(max) = nm_of(&c.value)
                    {
                        let t = l.thickness.to_um() * 1000.0;
                        a.checks.push(Check {
                            rule: Rule::Thickness,
                            layer: Some(l.name.clone()),
                            step: Some(k),
                            verdict: if t <= max {
                                Verdict::Within
                            } else {
                                Verdict::Warning
                            },
                            text: format!(
                                "{}: {} of {} by {}, against at most {} ({})",
                                l.name,
                                nm(t / 1000.0),
                                film.name,
                                p.name,
                                nm(max / 1000.0),
                                cited(c)
                            ),
                        });
                    }
                    a.accuracy.push(d.close());
                }
            }
            Operation::Evaporate { film, thickness } => {
                if !present.contains(&film.name) {
                    present.push(film.name.clone());
                }
                let how = if step.sputtered() {
                    ("sputtered", "sputtering")
                } else {
                    ("evaporated", "evaporation")
                };
                let mut d = Dimension::new(
                    format!("the {} {}'s thickness", how.0, film.name),
                    &film.name,
                    Kind::Thickness,
                    thickness.to_um() * 1000.0,
                    Unit::Nm,
                );
                d.add(
                    &format!("the {}'s thickness spread", how.1),
                    None,
                    None,
                    format!("no source in hand states it for {}", p.name),
                );
                a.accuracy.push(d.close());
            }
            _ => {}
        }
    }
    // planarization: a patterned layer above an etched step
    for pat in patterned(flow) {
        let (lk, _) = pat.litho;
        let etched_before = flow.steps[..lk].iter().any(|s| {
            matches!(
                s.operation,
                Operation::Etch { .. } | Operation::LiftOff { .. }
            )
        });
        if !etched_before {
            continue;
        }
        let polished = flow.steps[..lk]
            .iter()
            .rposition(|s| matches!(s.operation, Operation::Planarize { .. }));
        let last_etch = flow.steps[..lk].iter().rposition(|s| {
            matches!(
                s.operation,
                Operation::Etch { .. } | Operation::LiftOff { .. }
            )
        });
        let flat = matches!((polished, last_etch), (Some(p), Some(e)) if p > e);
        a.checks.push(Check {
            rule: Rule::Planarization,
            layer: Some(pat.layer.clone()),
            step: Some(lk),
            verdict: if flat {
                Verdict::Within
            } else {
                Verdict::Warning
            },
            text: if flat {
                format!(
                    "{}: patterned on a surface polished flat after the etch below it",
                    pat.layer
                )
            } else {
                format!(
                    "{}: patterned over the etched surface below it, not polished: the resist \
                     and the film lie over its steps (the 2D profile shows them); lift-off needs \
                     a smooth or planarized surface (campbell-2008, Section 11.9, pp. 310–311)",
                    pat.layer
                )
            },
        });
    }
    // conformality of a film filling etched openings
    for (k, step) in flow.steps.iter().enumerate() {
        if let Operation::Deposit { .. } = step.operation {
            let p = must(&step.process)?;
            let filling = step.reason.starts_with("It fills");
            if filling {
                let text = match coverage(p) {
                    Some(c) => format!(
                        "step {}, {}: it fills the etched openings with a step coverage of {}{}",
                        k + 1,
                        p.name,
                        cited(c),
                        if conformal(p) {
                            ": conformal, it follows walls and floors evenly"
                        } else {
                            ": thinner on the walls than on the flat, so narrow openings may \
                             close over a void"
                        }
                    ),
                    None => format!(
                        "step {}, {}: it fills the etched openings; its step coverage is not \
                         specified by the sources in hand",
                        k + 1,
                        p.name
                    ),
                };
                a.checks.push(Check {
                    rule: Rule::Planarization,
                    layer: None,
                    step: Some(k),
                    verdict: if conformal(p) {
                        Verdict::Within
                    } else if coverage(p).is_some() {
                        Verdict::Warning
                    } else {
                        Verdict::NotSpecified
                    },
                    text,
                });
            }
        }
    }
    // the rankings
    for (k, step) in flow.steps.iter().enumerate() {
        if step.alternatives.is_empty() {
            continue;
        }
        a.ranking.push(Ranking {
            step: k,
            ranked: rank(
                flow,
                k,
                narrowest.iter().find(|(j, _)| *j == k).map(|(_, v)| *v),
            )?,
        });
    }
    Ok(a)
}

/// The processes of step `k`, ranked: the chosen first, then the feasible alternatives by what
/// they reach, then the rest.
fn rank(flow: &Flow, k: usize, narrowest: Option<f64>) -> Result<Vec<Ranked>> {
    let step = &flow.steps[k];
    let chosen = must(&step.process)?;
    let mut out = vec![Ranked {
        process: chosen.id.clone(),
        name: chosen.name.clone(),
        chosen: true,
        feasible: true,
        better: Vec::new(),
        worse: Vec::new(),
        cost: "the flow's choice".into(),
    }];
    let mut rest: Vec<(usize, Ranked)> = Vec::new();
    for alt in &step.alternatives {
        let Some(p) = process(&alt.process) else {
            rest.push((
                3,
                Ranked {
                    process: alt.process.clone(),
                    name: alt.name.clone(),
                    chosen: false,
                    feasible: false,
                    better: Vec::new(),
                    worse: vec![alt.why_not.clone()],
                    cost: "another route".into(),
                },
            ));
            continue;
        };
        let refused = refused(&alt.why_not);
        let mut r = Ranked {
            process: p.id.clone(),
            name: p.name.clone(),
            chosen: false,
            feasible: !refused,
            better: Vec::new(),
            worse: Vec::new(),
            cost: String::new(),
        };
        let mut score = if refused { 2 } else { 1 };
        match step.step {
            StepType::Lithography => {
                let (cf, pf) = (min_feature(chosen), min_feature(p));
                let mut cost = Vec::new();
                if let (Some((c, _)), Some((v, pc))) = (cf, pf) {
                    if v < c {
                        r.better.push(format!(
                            "finer: it resolves down to {} against {} ({})",
                            nm(v / 1000.0),
                            nm(c / 1000.0),
                            cited(pc)
                        ));
                    } else if v > c {
                        r.worse.push(format!(
                            "coarser: it resolves down to {} against {} ({})",
                            nm(v / 1000.0),
                            nm(c / 1000.0),
                            cited(pc)
                        ));
                    }
                    cost.push(format!(
                        "smallest feature {} instead of {}",
                        nm(v / 1000.0),
                        nm(c / 1000.0)
                    ));
                }
                let (co, po) = (
                    largest_nm(chosen, &Quantity::Overlay),
                    largest_nm(p, &Quantity::Overlay),
                );
                match (co, po) {
                    (Some((c, _)), Some((v, pc))) => {
                        if v < c {
                            r.better.push(format!(
                                "tighter overlay: {} against {} ({})",
                                nm(v / 1000.0),
                                nm(c / 1000.0),
                                cited(pc)
                            ));
                        } else if v > c {
                            r.worse.push(format!(
                                "looser overlay: {} against {} ({})",
                                nm(v / 1000.0),
                                nm(c / 1000.0),
                                cited(pc)
                            ));
                        }
                        cost.push(format!(
                            "overlay {} instead of {}",
                            nm(v / 1000.0),
                            nm(c / 1000.0)
                        ));
                    }
                    (None, Some((v, pc))) => {
                        r.better.push(format!("a stated overlay, {}", cited(pc)));
                        cost.push(format!("overlay {}", nm(v / 1000.0)));
                    }
                    (Some(_), None) => r.worse.push("its overlay is not specified".into()),
                    (None, None) => {}
                }
                if let (None, Some((v, pc))) = (uniformity(chosen), uniformity(p)) {
                    r.better.push(format!(
                        "a stated linewidth control, ± {} ({})",
                        nm(v / 1000.0),
                        cited(pc)
                    ));
                }
                if p.id.starts_with("ebl-") {
                    r.worse.push(
                        "it writes point by point with no mask: hours per wafer for a full \
                         pattern (campbell-2008, p. 168), so it suits prototypes"
                            .into(),
                    );
                } else if chosen.id.starts_with("ebl-") {
                    r.better
                        .push("a mask exposes a whole field at once: for volume".into());
                }
                if let Some(o) = Optics::of(p) {
                    // the layer this lithography patterns
                    if let Some(f) = narrowest {
                        match o.reach(f * 1000.0) {
                            Reach::Typical => {}
                            Reach::Enhanced => {
                                score = score.max(1);
                                r.worse.push(format!(
                                    "the layer asks it for k₁ = {}, below the typical 0.4: \
                                     only with resolution enhancement (campbell-2008, Eq. \
                                     7.18)",
                                    number(o.k1(f * 1000.0))
                                ));
                            }
                            Reach::Beyond => {
                                r.feasible = false;
                                score = 2;
                            }
                        }
                    }
                }
                r.cost = if cost.is_empty() {
                    "not comparable: the sources state neither's resolution or overlay".into()
                } else {
                    cost.join("; ")
                };
            }
            StepType::PatternTransfer => {
                if let Operation::Etch { sidewall, .. } = step.operation {
                    let deg = sidewall.to_degrees();
                    let off = |x: &Process| {
                        walls(x)
                            .iter()
                            .map(|(lo, hi, _)| {
                                if deg < *lo {
                                    lo - deg
                                } else if deg > *hi {
                                    deg - hi
                                } else {
                                    0.0
                                }
                            })
                            .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
                    };
                    match (off(chosen), off(p)) {
                        (Some(c), Some(v)) if v < c => r.better.push(format!(
                            "its cited walls come {}° closer to the design's {}°",
                            number(c - v),
                            number(deg)
                        )),
                        (Some(c), Some(v)) if v > c => r.worse.push(format!(
                            "its cited walls lie {}° further from the design's {}°",
                            number(v - c),
                            number(deg)
                        )),
                        (Some(_), None) => r.worse.push("its walls are not specified".into()),
                        (None, Some(_)) => r.better.push("its walls are stated".into()),
                        _ => {}
                    }
                    match (uniformity(chosen), uniformity(p)) {
                        (None, Some((v, pc))) => r.better.push(format!(
                            "a stated depth variation, {} ({})",
                            nm(v / 1000.0),
                            cited(pc)
                        )),
                        (Some(_), None) => {
                            r.worse.push("its depth variation is not specified".into())
                        }
                        _ => {}
                    }
                    r.cost = match off(p) {
                        Some(v) => format!(
                            "walls {}° from the design's: the floor of a line moves by up to \
                             2·d·|cot θ − cot θ₀|",
                            number(v)
                        ),
                        None => "walls not specified".into(),
                    };
                } else {
                    r.cost = "another route".into();
                }
                if refused || (r.better.is_empty() && r.worse.is_empty()) {
                    r.worse.push(alt.why_not.clone());
                }
            }
            StepType::Deposition => {
                let t = |x: &Process| x.temperature().map(|(t, _)| t);
                match (t(chosen), t(p)) {
                    (Some(c), Some(v)) if v > c => r.worse.push(format!(
                        "hotter: up to {} °C against {} °C",
                        number(v),
                        number(c)
                    )),
                    (Some(c), Some(v)) if v < c => r.better.push(format!(
                        "cooler: up to {} °C against {} °C",
                        number(v),
                        number(c)
                    )),
                    _ => {}
                }
                match (conformal(chosen), conformal(p)) {
                    (false, true) => r.better.push(format!(
                        "conformal ({})",
                        coverage(p).map_or(String::new(), cited)
                    )),
                    (true, false) => r.worse.push(format!(
                        "not conformal: step coverage {}",
                        coverage(p).map_or("not specified".into(), cited)
                    )),
                    _ => {}
                }
                r.worse.push(alt.why_not.clone());
                r.cost = match (coverage(p), t(p)) {
                    (Some(c), Some(t)) => {
                        format!("step coverage {}, up to {} °C", c.value, number(t))
                    }
                    (None, Some(t)) => {
                        format!("up to {} °C, step coverage not specified", number(t))
                    }
                    _ => "not specified".into(),
                };
            }
            _ => {
                r.worse.push(alt.why_not.clone());
                r.cost = "another route".into();
            }
        }
        // the generator's own reason, why it isn't the choice
        if !r.worse.contains(&alt.why_not) {
            r.worse.push(alt.why_not.clone());
        }
        rest.push((score, r));
    }
    // the feasible first, then by how many gains outweigh losses; ties in the flow's order
    rest.sort_by_key(|(s, r)| (*s, r.worse.len() as i64 - r.better.len() as i64));
    out.extend(rest.into_iter().map(|(_, r)| r));
    Ok(out)
}

/// Whether an alternative's reason says it can't do the step at all.
fn refused(why_not: &str) -> bool {
    why_not.contains("is above the layer's narrowest feature")
        || why_not.contains("below the theoretical limit")
        || why_not.contains("not specified by the sources in hand")
        || (why_not.contains("above ") && why_not.contains("'s limit"))
        || why_not.contains("grows oxide only from silicon")
        || why_not.contains("lasts only")
}

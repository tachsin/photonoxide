//! A flow's frames: the wafer at each moment the studio's Fabrication page plays, from the bare
//! wafer to the design. Most steps are one frame, the executor's stage after it
//! ([`crate::fabrication::stages`]); a lithography is three, the resist spun on, the pattern
//! written into it (pressed, for nanoimprint) and the resist developed.
//!
//! What the planar stages can't hold is drawn as films over them ([`FrameFilm`],
//! [`super::fabrication_films`]):
//!
//! - **The resist**, which the stages leave out (it changes no layer of the design), from its
//!   lithography to the etch or lift-off that uses it and the strip that removes it. Spun on, it
//!   fills the regions etched open and its top is flat, its thickness over the highest top. It is
//!   drawn with silica's optics, which are near any resist's (its index is between 1.4 and 1.5 at
//!   1.55 µm), and named for itself, so a picture colours it apart from the oxides. A resist whose
//!   thickness no source in hand states is drawn [`UNSTATED_RESIST_UM`] thick, and its frames say
//!   so.
//! - **The exposure**: the part of the resist the light or the beam falls on, by the resist's
//!   polarity ([`crate::fabrication::Polarity`]) and the part that must stay: a positive resist
//!   is exposed where it will dissolve, a negative one where it will stay. Where the library
//!   names no polarity, the frames draw a positive resist and say so. Nanoimprint exposes
//!   nothing: its mould presses the pattern in, and a thin layer left in the recesses is cleared.
//! - **A film deposited over an etched step**: conformal, its thickness on the flat the
//!   deposition's, on the walls that times the process's step coverage (the middle of a stated
//!   range; fully conformal, the frames' default, where no source states it). The frame after it
//!   is the executor's stage again: flat after a polish, or as the design draws it.
//! - **A film evaporated for a lift-off**, on the resist and in its openings: where an opening
//!   lies over a region etched open, on that region's floor. From the lift-off on, the film it
//!   leaves there is drawn where it landed, its planar layer as what fills it.
//! - **A wafer the flow makes for itself** (Smart Cut or SIMOX), whose stages keep the design's
//!   film throughout: what each step of the route does, from its numbers (`start_films`): the
//!   implanted hydrogen's plane, the rest of the donor bonded on and the bonded interface until
//!   the anneal, the film's excess until the polish and the sacrificial oxide, that oxide until
//!   the strip; for SIMOX, the damaged silicon over the implanted oxide until the anneal.
//!
//! The last frame is the last stage, the structure the round trip checks, but for a film lifted
//! off onto an etched floor, which it draws there.

use serde::Serialize;

use super::Event;
use super::fabrication::named;
use super::fabrication_films::{
    Area, FrameFilm, Open, Paint, and, conformal, fill, minus, open_layers, prism, reach, rect,
    stand_in, top_of, union,
};
use super::scene::{SceneEtched, drawn_eps, scene_drawn, scene_etched};
use crate::Result;
use crate::fabrication::{Flow, Made, Operation, Polarity, Quantity, Tone, Unit, process};
use crate::material::{Material, silica};
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};

/// The thickness a resist is drawn at when no source in hand states it, µm.
pub const UNSTATED_RESIST_UM: f64 = 0.3;

/// The share of its thickness a nanoimprinted resist is drawn keeping in the mould's recesses,
/// which no source in hand states.
pub const IMPRINT_RESIDUE: f64 = 0.1;

/// The step coverage a deposition is drawn with when no source in hand states its own: fully
/// conformal, the same thickness on the walls as on the flat.
pub const UNSTATED_COVERAGE: f64 = 1.0;

/// The wafer at one moment of its flow.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FlowFrame {
    /// The step it belongs to, from 0.
    pub step: usize,
    /// What happens to the wafer: the step's operation (`"wafer"`, `"deposit"`, `"etch"`, …, as
    /// [`crate::fabrication::StepRecord::operation`] names it), or a lithography's `"coat"`,
    /// `"expose"` (`"imprint"` for nanoimprint) and `"develop"` (`"residue"`: nanoimprint's
    /// recesses cleared).
    pub phase: String,
    /// What happens, in a few words.
    pub label: String,
    /// The layer it acts on: the layer made, etched or lifted off, `"resist"` for a
    /// lithography's phases and the strip; none for a step that changes no layer.
    pub layer: Option<String>,
    /// The wafer's planar layers as it leaves it ([`Event::Scene`]), its media named by the films
    /// there.
    pub scene: Event,
    /// The etched layers' shapes with their etches ([`scene_etched`]).
    pub etched: Vec<SceneEtched>,
    /// The films over the planar layers, which they can't hold: the resist, a film deposited
    /// over an etched step, a film evaporated onto a stencil. A picture draws them over the
    /// layers' backgrounds and under their shapes.
    pub films: Vec<FrameFilm>,
    /// What the picture can't take from a source, and what it assumes in its place: a resist's
    /// thickness, its polarity, a film's step coverage.
    pub note: Option<String>,
}

/// A resist on the wafer: its film's name, its thickness drawn, whether a source states it, the
/// shapes it is patterned on, its tone, how it is patterned, and the topography it was spun over.
struct Resist {
    name: String,
    thickness: f64,
    stated: bool,
    tone: Tone,
    /// The polarity drawn, and whether the library names it (else the frames' default).
    polarity: Polarity,
    named: bool,
    /// The source that states the polarity, if one in hand does.
    source: Option<crate::fabrication::Source>,
    /// Whether a mould presses the pattern in (nanoimprint) instead of light or a beam.
    imprint: bool,
    /// Whether an electron beam writes it, else light through a mask.
    beam: bool,
    /// The wafer's top when it was spun on, and the layers it filled.
    top: f64,
    open: Vec<Open>,
    /// The window, the shapes' union and the rest of the window.
    window: Area,
    shapes: Area,
    rest: Area,
    count: usize,
    /// The permittivity it is drawn with: silica's.
    eps: f64,
}

/// The resist's film name for the library's resist id.
fn resist_name(id: &str) -> String {
    match id {
        "hsq" => "HSQ resist".into(),
        "pmma" => "PMMA resist".into(),
        "ma-n" => "ma-N resist".into(),
        "photoresist" => "photoresist".into(),
        "" => "resist".into(),
        other => format!("{other} resist"),
    }
}

/// `n` shapes, in words.
fn shapes_text(n: usize) -> String {
    format!("{n} shape{}", if n == 1 { "" } else { "s" })
}

/// A number in a note: to four significant digits, without trailing zeros.
fn num(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let digits = usize::try_from((3 - v.abs().log10().floor() as i32).max(0)).unwrap_or(0);
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

impl Resist {
    fn paint(&self, film: &str) -> Paint {
        Paint {
            layer: "resist".into(),
            film: film.into(),
            eps: self.eps,
        }
    }

    /// The resist over `area`, `t` thick over the top, filling the regions open below.
    fn over(&self, film: &str, area: &Area, t: f64) -> Vec<FrameFilm> {
        let p = self.paint(film);
        let mut out = fill(&p, &self.open, area);
        out.extend(prism(&p, area, self.top, self.top + t));
        out
    }

    /// The part that stays after developing: on the shapes (an etch mask) or around them (a
    /// stencil).
    fn kept(&self) -> &Area {
        match self.tone {
            Tone::Keep => &self.shapes,
            Tone::Open => &self.rest,
        }
    }

    /// The part that goes.
    fn gone(&self) -> &Area {
        match self.tone {
            Tone::Keep => &self.rest,
            Tone::Open => &self.shapes,
        }
    }

    /// Spun on: over the whole window.
    fn coat(&self) -> Vec<FrameFilm> {
        self.over(&self.name, &self.window, self.thickness)
    }

    /// Exposed (or imprinted): the part the light or the beam falls on, by the polarity; or, for
    /// nanoimprint, the pattern pressed in, a thin layer left in the recesses.
    fn written(&self) -> Vec<FrameFilm> {
        if self.imprint {
            let mut out = self.over(&self.name, self.kept(), self.thickness);
            out.extend(self.over(&self.name, self.gone(), IMPRINT_RESIDUE * self.thickness));
            return out;
        }
        let exposed_name = format!("{}, exposed", self.name);
        let (exposed, unexposed) = self.exposed();
        let mut out = self.over(&self.name, unexposed, self.thickness);
        out.extend(self.over(&exposed_name, exposed, self.thickness));
        out
    }

    /// The parts exposed and not: a positive resist is exposed where it goes, a negative one
    /// where it stays.
    fn exposed(&self) -> (&Area, &Area) {
        match self.polarity {
            Polarity::Positive => (self.gone(), self.kept()),
            Polarity::Negative => (self.kept(), self.gone()),
        }
    }

    /// Developed: what stays.
    fn developed(&self) -> Vec<FrameFilm> {
        self.over(&self.name, self.kept(), self.thickness)
    }

    /// A stencil with `film`, `t` thick, put down over it: in its openings where it lands
    /// (`landed`, [`Resist::landed`]), and on the resist around them.
    fn evaporated(&self, film: &Paint, t: f64, landed: &[FrameFilm]) -> Vec<FrameFilm> {
        let mut out = self.developed();
        out.extend_from_slice(landed);
        out.extend(prism(
            film,
            self.kept(),
            self.top + self.thickness,
            self.top + self.thickness + t,
        ));
        out
    }

    /// A film `t` thick evaporated into the stencil's openings, where it lands: on the wafer's
    /// top, and where an opening lies over a region of the top layer etched open (around a
    /// ridge, or in a trench), on that region's floor; with the open layer whose floor it
    /// reaches, if any. Evaporation's poor step coverage, which lift-off relies on (Campbell,
    /// Section 11.9), puts nothing on the walls. Around a sloped ridge the piece on the floor
    /// runs to the ridge's top outline, over the wall's foot, which the picture draws over it.
    fn landed(&self, film: &Paint, t: f64) -> (Vec<FrameFilm>, Option<&Open>) {
        let openings = self.gone();
        let surface = self
            .open
            .iter()
            .find(|o| (o.z[1] - self.top).abs() < 1e-9 && o.floor < o.z[1] - 1e-12);
        let Some(o) = surface else {
            return (prism(film, openings, self.top, self.top + t), None);
        };
        let shapes = union(&o.shapes);
        let (on_top, on_floor) = if o.ridge {
            (and(openings, &shapes), minus(openings, &shapes))
        } else {
            (minus(openings, &shapes), and(openings, &shapes))
        };
        let floor = prism(film, &on_floor, o.floor, o.floor + t);
        if floor.is_empty() {
            return (prism(film, openings, self.top, self.top + t), None);
        }
        let mut out = prism(film, &on_top, self.top, self.top + t);
        out.extend(floor);
        (out, Some(o))
    }

    /// Its frames' notes: the thickness drawn, and how it lies over etched layers.
    fn note(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.stated {
            parts.push(format!(
                "No source in hand states the {}'s thickness: it is drawn {UNSTATED_RESIST_UM} µm \
                 thick.",
                self.name
            ));
        }
        if !self.open.is_empty() {
            parts.push(format!(
                "Spun over the etched {}, the resist fills the etched regions and its top is \
                 flat, {} µm over the highest top (the frames' model of a spin-on film).",
                self.open
                    .iter()
                    .map(|o| o.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" and "),
                num(self.thickness)
            ));
        }
        (!parts.is_empty()).then(|| parts.join(" "))
    }

    /// The exposure's label: what is exposed, and what the polarity does with it.
    fn written_label(&self) -> String {
        let shapes = shapes_text(self.count);
        if self.imprint {
            return format!(
                "imprint: the mould presses the {shapes}' pattern into the {}; no light, no beam",
                self.name
            );
        }
        // a positive resist is exposed where it goes, a negative one where it stays
        let on_shapes = matches!(
            (self.polarity, self.tone),
            (Polarity::Positive, Tone::Open) | (Polarity::Negative, Tone::Keep)
        );
        let what = match (self.beam, on_shapes) {
            (true, true) => format!("the beam writes the {shapes} into the {}", self.name),
            (true, false) => format!(
                "the beam writes everything around the {shapes} into the {}",
                self.name
            ),
            (false, true) => format!(
                "light falls through the mask onto the {shapes} of the {}",
                self.name
            ),
            (false, false) => format!(
                "light falls through the mask around the {shapes} of the {}",
                self.name
            ),
        };
        let does = match self.polarity {
            Polarity::Positive => "positive: the exposed part will dissolve",
            Polarity::Negative => "negative: the exposed part will stay",
        };
        format!("expose: {what}; it is {does}")
    }

    /// The exposure's note: the polarity's source, or the assumption in its place.
    fn written_note(&self) -> Option<String> {
        let mut parts: Vec<String> = self.note().into_iter().collect();
        if self.imprint {
            parts.push(format!(
                "No source in hand states how thick a layer the mould leaves in its recesses: it \
                 is drawn as {} of the resist's thickness.",
                num(IMPRINT_RESIDUE)
            ));
        } else if let Some(s) = &self.source {
            let citation = crate::fabrication::reference(&s.reference)
                .map_or_else(|| s.reference.clone(), |r| r.citation);
            parts.push(format!(
                "The {} is {}: {citation}, {}.",
                self.name,
                self.polarity_word(),
                s.location
            ));
        } else if self.named {
            parts.push(format!(
                "No source in hand states the {}'s polarity by its place: it is drawn {}, as the \
                 library's description of the process names it.",
                self.name,
                self.polarity_word()
            ));
        } else {
            parts.push(format!(
                "No source in hand states whether the {} is positive or negative: it is drawn \
                 positive, the frames' default.",
                self.name
            ));
        }
        Some(parts.join(" "))
    }

    fn polarity_word(&self) -> &'static str {
        match self.polarity {
            Polarity::Positive => "positive",
            Polarity::Negative => "negative",
        }
    }
}

/// `made` with a bare wafer's stand-in air layer taken away, so films lie on the wafer itself.
fn bare(made: &Made) -> Result<Made> {
    let s = &made.structure;
    let stack = s.stack();
    let keep: Vec<usize> = (0..stack.layers().len())
        .filter(|&i| !stand_in(&stack.layers()[i], &made.layers[i], s))
        .collect();
    if keep.len() == stack.layers().len() {
        return Ok(made.clone());
    }
    let layers = keep.iter().map(|&i| stack.layers()[i].clone()).collect();
    let names = keep.iter().map(|&i| made.layers[i].clone()).collect();
    let mut structure = Structure::new(LayerStack::new(
        stack.substrate().clone(),
        layers,
        stack.cladding().clone(),
    )?);
    for &i in &keep {
        let name = &stack.layers()[i].name;
        for (shape, etch) in s.shapes(name).iter().zip(s.etches(name)) {
            structure.draw_etched(name, shape.clone(), *etch)?;
        }
    }
    Ok(Made {
        structure,
        substrate: made.substrate.clone(),
        layers: names,
        cladding: made.cladding.clone(),
    })
}

/// A deposition's step coverage as the frames draw it: the share of its thickness on the walls,
/// and the note that says where it comes from.
fn coverage(process_id: &str) -> (f64, String) {
    let stated = process(process_id).and_then(|p| {
        p.capabilities_of(&Quantity::StepCoverage)
            .find(|c| c.value.is_stated() && c.value.unit() == Some(Unit::Percent))
            .map(|c| (c.clone(), p.name.clone()))
    });
    match stated {
        Some((c, name)) => {
            let (lo, hi) = (
                c.value.low().unwrap_or(100.0),
                c.value.high().unwrap_or(100.0),
            );
            let s = c.value.source();
            let cite = s.map_or(String::new(), |s| {
                format!(" ({}, {})", s.reference, s.location)
            });
            let mid = 0.5 * (lo + hi);
            let text = if (hi - lo).abs() < 1e-12 {
                format!("{name}'s step coverage is {} %{cite}", num(lo))
            } else {
                format!(
                    "{name}'s step coverage is {} to {} %{cite}: it is drawn at {} %, the \
                     middle",
                    num(lo),
                    num(hi),
                    num(mid)
                )
            };
            (mid / 100.0, text)
        }
        None => (
            UNSTATED_COVERAGE,
            format!(
                "No source in hand states this deposition's step coverage: it is drawn fully \
                 conformal ({} %), the frames' default",
                num(100.0 * UNSTATED_COVERAGE)
            ),
        ),
    }
}

/// A film deposited over the single open layer of `before`: what it is, how thick on the flat,
/// how thick on the walls.
struct Draped<'a> {
    open: Open,
    film: &'a crate::fabrication::Film,
    layer: String,
    t: f64,
    n: f64,
    coverage: String,
}

/// The finite film `step` deposits over the open layer of `before`, when it deposits one over
/// exactly one open layer (a cladding, over everything, is drawn as the stage draws it).
fn draped<'a>(step: &'a crate::fabrication::Step, before: &Made) -> Option<Draped<'a>> {
    let Operation::Deposit {
        film,
        layers,
        cladding: false,
        extra,
    } = &step.operation
    else {
        return None;
    };
    let first = layers.first()?;
    let mut open = open_layers(before);
    if open.len() != 1 {
        return None;
    }
    let t = layers.iter().map(|l| l.thickness.to_um()).sum::<f64>() + extra.to_um();
    let (s, coverage) = coverage(&step.process);
    Some(Draped {
        open: open.remove(0),
        film,
        layer: first.name.clone(),
        t,
        n: s * t,
        coverage,
    })
}

/// The frames of `flow`, whose stages are `stages`, over the window `x` × `y` (grown where a
/// conformal film reaches more than 0.5 µm short of its edge, the 1 µm the window keeps around
/// the shapes) from `z_bottom` to 1 µm above the highest top any frame or `design_top` reaches,
/// at `wavelength`.
///
/// # Errors
///
/// A structure the frames can't build (none for a flow the executor ran), and a material
/// without data at `wavelength`.
pub(super) fn frames(
    flow: &Flow,
    stages: &[Made],
    x: [f64; 2],
    y: [f64; 2],
    z_bottom: f64,
    design_top: f64,
    wavelength: Wavelength,
) -> Result<Vec<FlowFrame>> {
    let (moments, x, y) = moments(flow, stages, x, y, wavelength)?;
    let top = moments.iter().map(Moment::top).fold(design_top, f64::max);
    // deep enough for a film inside the substrate (a Smart Cut donor's implanted plane)
    let low = moments
        .iter()
        .flat_map(|m| &m.films)
        .map(|f| f.z_um[0] - 0.3)
        .fold(z_bottom, f64::min);
    let z = [low, top + 1.0];
    moments
        .into_iter()
        .map(|m| {
            let mut drawn = scene_drawn(&m.made.structure, x, y, z, wavelength)?;
            named(&mut drawn, &m.made);
            Ok(FlowFrame {
                step: m.step,
                phase: m.phase,
                label: m.label,
                layer: m.layer,
                scene: drawn,
                etched: scene_etched(&m.made.structure),
                films: m.films,
                note: m.note,
            })
        })
        .collect()
}

/// A moment of a flow, as a frame draws it: the step it belongs to, what happens, the wafer's
/// planar layers and the films over them ([`FlowFrame`] has each field's meaning).
pub(crate) struct Moment {
    pub step: usize,
    pub phase: String,
    pub label: String,
    pub layer: Option<String>,
    pub made: Made,
    pub films: Vec<FrameFilm>,
    pub note: Option<String>,
}

impl Moment {
    /// The highest the wafer stands, its films included, µm.
    pub fn top(&self) -> f64 {
        self.films
            .iter()
            .map(|f| f.z_um[1])
            .fold(self.made.structure.stack().thickness().to_um(), f64::max)
    }
}

/// The moments [`frames`] draws, and the window `x` × `y` grown to hold every conformal film.
///
/// # Errors
///
/// [`frames`]'s.
pub(crate) fn moments(
    flow: &Flow,
    stages: &[Made],
    x: [f64; 2],
    y: [f64; 2],
    wavelength: Wavelength,
) -> Result<(Vec<Moment>, [f64; 2], [f64; 2])> {
    // the window, grown to hold every conformal film
    let mut grow: f64 = 0.0;
    for (k, step) in flow.steps.iter().enumerate().skip(1) {
        if let Some(d) = draped(step, &stages[k - 1]) {
            grow = grow.max(reach(&d.open, d.t, d.n) + 0.5 - 1.0);
        }
    }
    let (x, y) = ([x[0] - grow, x[1] + grow], [y[0] - grow, y[1] + grow]);
    let window = rect(x, y);
    let eps = |m: &Material| -> Result<f64> { drawn_eps(m, wavelength) };
    let records = flow.record(None).steps;
    let mut moments: Vec<Moment> = Vec::new();
    let mut resist: Option<Resist> = None;
    // a film evaporated whose pieces lie on an etched floor, then the step that lifts it off
    let mut lifted: Option<Lifted> = None;
    let mut lifted_at: Option<(usize, Lifted)> = None;
    for (k, (step, made)) in flow.steps.iter().zip(stages).enumerate() {
        let record = &records[k];
        let mut push = |phase: &str,
                        label: String,
                        layer: Option<String>,
                        made: Made,
                        films: Vec<FrameFilm>,
                        note: Option<String>| {
            moments.push(Moment {
                step: k,
                phase: phase.into(),
                label,
                layer,
                made,
                films,
                note,
            });
        };
        let before = (k > 0).then(|| &stages[k - 1]);
        match &step.operation {
            Operation::Pattern {
                resist: id,
                thickness,
                mask,
                tone,
            } => {
                // the wafer as the resist is spun on it: the stage before (a lithography changes
                // no layer)
                let under = before.unwrap_or(made);
                let litho = process(&step.process);
                let polarity = litho.and_then(crate::fabrication::Process::polarity);
                let shapes = union(mask);
                let all: Area = window.clone();
                let r = Resist {
                    name: resist_name(id),
                    thickness: thickness.map_or(UNSTATED_RESIST_UM, Length::to_um),
                    stated: thickness.is_some(),
                    tone: *tone,
                    polarity: polarity.as_ref().map_or(Polarity::Positive, |p| p.polarity),
                    named: polarity.is_some(),
                    source: polarity.and_then(|p| p.source),
                    imprint: litho.is_some_and(crate::fabrication::Process::imprints),
                    beam: step.process.starts_with("ebl"),
                    top: top_of(under),
                    open: open_layers(under),
                    rest: minus(&all, &shapes),
                    window: all,
                    shapes,
                    count: mask.len(),
                    eps: eps(&silica())?,
                };
                let layer = Some("resist".to_owned());
                let shown = bare(made)?;
                push(
                    "coat",
                    format!("spin the {} on", r.name),
                    layer.clone(),
                    shown.clone(),
                    r.coat(),
                    r.note(),
                );
                push(
                    if r.imprint { "imprint" } else { "expose" },
                    r.written_label(),
                    layer.clone(),
                    shown.clone(),
                    r.written(),
                    r.written_note(),
                );
                let shapes = shapes_text(mask.len());
                push(
                    if r.imprint { "residue" } else { "develop" },
                    match (r.imprint, tone) {
                        (true, _) => format!(
                            "the thin {} left in the mould's recesses etched away",
                            r.name
                        ),
                        (false, Tone::Keep) => {
                            format!("develop: the {} stays on the {shapes}", r.name)
                        }
                        (false, Tone::Open) => {
                            format!("develop: the {} opens over the {shapes}", r.name)
                        }
                    },
                    layer,
                    shown,
                    r.developed(),
                    r.note(),
                );
                resist = Some(r);
            }
            Operation::Evaporate { film, thickness } => {
                let (films, note) = match &resist {
                    Some(r) => {
                        // the layer the lift-off after it makes, which its pieces belong to
                        let layer = flow.steps[k + 1..]
                            .iter()
                            .find_map(|s| match &s.operation {
                                Operation::LiftOff { layer } => Some(layer.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| film.name.clone());
                        let paint = Paint {
                            layer: layer.clone(),
                            film: film.name.clone(),
                            eps: eps(&film.optical)?,
                        };
                        let t = thickness.to_um();
                        let (landed, floor) = r.landed(&paint, t);
                        let mut note: Vec<String> = r.note().into_iter().collect();
                        if let Some(o) = floor {
                            note.push(format!(
                                "Where an opening lies over the region etched open around the \
                                 {}, the {} lands on its floor, {} µm below the top: it is drawn \
                                 there, and so is what the lift-off leaves, though the design \
                                 (planar, as the model has every layer) draws its {} layer on \
                                 the top.",
                                o.name,
                                film.name,
                                num(o.z[1] - o.floor),
                                layer
                            ));
                            lifted = Some(Lifted {
                                layer,
                                pieces: landed.clone(),
                            });
                        }
                        (
                            r.evaporated(&paint, t, &landed),
                            (!note.is_empty()).then(|| note.join(" ")),
                        )
                    }
                    None => (Vec::new(), None),
                };
                push(
                    &record.operation,
                    record.does.clone(),
                    record.layer.clone(),
                    bare(made)?,
                    films,
                    note,
                );
            }
            Operation::LiftOff { .. } | Operation::Strip => {
                let layer = match step.operation {
                    Operation::Strip => Some("resist".to_owned()),
                    _ => record.layer.clone(),
                };
                if let (Operation::LiftOff { .. }, Some(l)) = (&step.operation, lifted.take()) {
                    lifted_at = Some((k, l));
                }
                push(
                    &record.operation,
                    record.does.clone(),
                    layer,
                    made.clone(),
                    Vec::new(),
                    None,
                );
                resist = None;
            }
            _ => {
                let drape = match (&resist, before) {
                    (None, Some(b)) => draped(step, b),
                    _ => None,
                };
                let conformal_films = match &drape {
                    Some(d) => conformal(
                        &Paint {
                            layer: d.layer.clone(),
                            film: d.film.name.clone(),
                            eps: eps(&d.film.optical)?,
                        },
                        &d.open,
                        &window,
                        d.t,
                        d.n,
                    )
                    .map(|f| (f, d)),
                    None => None,
                };
                match (conformal_films, &resist) {
                    (Some((films, d)), _) => {
                        let polished = matches!(
                            flow.steps.get(k + 1).map(|s| &s.operation),
                            Some(Operation::Planarize { .. })
                        );
                        let note = format!(
                            "Over the {} µm step of the etched {}, the {} follows the surface: \
                             {} µm on the flat and {} µm on the walls, measured normal to them. \
                             {}. {}",
                            num(d.open.z[1] - d.open.floor),
                            d.open.name,
                            d.film.name,
                            num(d.t),
                            num(d.n),
                            d.coverage,
                            if polished {
                                "The polish after it leaves it flat, as the design has it."
                            } else {
                                "The design draws it flat, and so do the frames after this one: \
                                 no polish follows it (the flow's planarization check says why)."
                            }
                        );
                        push(
                            &record.operation,
                            record.does.clone(),
                            record.layer.clone(),
                            bare(before.unwrap_or(made))?,
                            films,
                            Some(note),
                        );
                    }
                    (None, Some(r)) => {
                        push(
                            &record.operation,
                            record.does.clone(),
                            record.layer.clone(),
                            bare(made)?,
                            r.developed(),
                            r.note(),
                        );
                    }
                    (None, None) => {
                        let (films, note) = start_films(flow, k, made, x, y, wavelength)?;
                        push(
                            &record.operation,
                            record.does.clone(),
                            record.layer.clone(),
                            made.clone(),
                            films,
                            note,
                        );
                    }
                }
            }
        }
    }
    // from the lift-off on, the film it leaves is drawn where it landed, its planar layer as
    // what fills it (the stages, which the round trip checks, keep the design's planar layer)
    if let Some((at, l)) = lifted_at {
        for m in moments.iter_mut().filter(|m| m.step >= at) {
            m.made = unlifted(&m.made, &l.layer)?;
            m.films.extend(l.pieces.iter().cloned());
        }
    }
    Ok((moments, x, y))
}

/// A film evaporated for a lift-off whose pieces lie on an etched floor: the layer the lift-off
/// makes, and its pieces as they landed.
struct Lifted {
    layer: String,
    pieces: Vec<FrameFilm>,
}

/// `made` with `layer`'s shapes taken off, the layer drawn as what fills it around them: the
/// frames draw that layer's film as pieces where it landed instead.
fn unlifted(made: &Made, layer: &str) -> Result<Made> {
    let s = &made.structure;
    let stack = s.stack();
    let Some(i) = stack.layers().iter().position(|l| l.name == layer) else {
        return Ok(made.clone());
    };
    let mut layers = stack.layers().to_vec();
    layers[i].material = layers[i].background.clone();
    let mut names = made.layers.clone();
    names[i].0 = names[i].1.clone();
    let mut structure = Structure::new(LayerStack::new(
        stack.substrate().clone(),
        layers,
        stack.cladding().clone(),
    )?);
    for l in stack.layers().iter().filter(|l| l.name != layer) {
        for (shape, etch) in s.shapes(&l.name).iter().zip(s.etches(&l.name)) {
            structure.draw_etched(&l.name, shape.clone(), *etch)?;
        }
    }
    Ok(Made {
        structure,
        substrate: made.substrate.clone(),
        layers: names,
        cladding: made.cladding.clone(),
    })
}

/// How thick the rest of a Smart Cut donor is drawn above its implanted plane, µm: cut short, as
/// a wafer is hundreds of micrometres thick.
pub(crate) const DONOR_DRAWN_UM: f64 = 0.8;

/// The thinnest a plane is drawn, µm: a bonded interface, or an implanted peak narrower than it.
pub(crate) const PLANE_DRAWN_UM: f64 = 0.03;

/// How far a plane inside the wafer (an implanted peak, a bonded interface) is drawn past the
/// window's sides, µm, so a 3D view shows it where it would otherwise be hidden in the silicon.
const PLANE_OVERHANG_UM: f64 = 0.2;

/// The film names of what a flow that makes its own SOI wafer adds to its frames.
pub(crate) const HYDROGEN: &str = "implanted hydrogen";
pub(crate) const BOND_INTERFACE: &str = "bond interface";
pub(crate) const DAMAGED_SI: &str = "Si, implant-damaged";

/// What the planar stages of a flow that makes its own SOI wafer (Smart Cut or SIMOX) leave out
/// at step `k`, drawn for its frame from the route's own numbers, with what the frame shows: the
/// implanted hydrogen's plane, the rest of the donor bonded on and the bonded interface until the
/// anneal strengthens it, the film's excess until the polish and the sacrificial oxide take it
/// off, that oxide until the hydrofluoric acid strips it; for SIMOX, the silicon over the
/// implanted oxide until the anneal heals it. The stages keep the design's film throughout (the
/// round trip compares them, not the frames). Nothing for any other step or flow.
fn start_films(
    flow: &Flow,
    k: usize,
    made: &Made,
    x: [f64; 2],
    y: [f64; 2],
    wavelength: Wavelength,
) -> Result<(Vec<FrameFilm>, Option<String>)> {
    use crate::fabrication::{simox, smart_cut};
    // only the route's own steps, the run of them the flow starts with: a later step of the
    // same process (a silicon polish of the design's own) draws nothing of the route's
    const ROUTE: [&str; 10] = [
        "si-wafer",
        "thermal-oxidation",
        "h-implant",
        "direct-bonding",
        "smart-cut-split",
        "anneal-bond",
        "cmp-si",
        "hf-oxide-strip",
        "o-implant",
        "simox-anneal",
    ];
    let route_end = flow
        .steps
        .iter()
        .position(|s| !ROUTE.contains(&s.process.as_str()))
        .unwrap_or(flow.steps.len());
    if k >= route_end {
        return Ok((Vec::new(), None));
    }
    let step = &flow.steps[k];
    // the layer a step of the route deposits, and its film
    let deposit = |id: &str| {
        flow.steps.iter().find_map(|s| match &s.operation {
            Operation::Deposit { film, layers, .. } if s.process == id => {
                layers.first().map(|l| (l.clone(), film.clone()))
            }
            _ => None,
        })
    };
    let eps = |m: &Material| -> Result<f64> { drawn_eps(m, wavelength) };
    let paint = |layer: &str, film: &str, eps: f64| Paint {
        layer: layer.into(),
        film: film.into(),
        eps,
    };
    let window = rect(x, y);
    let past = rect(
        [x[0] - PLANE_OVERHANG_UM, x[1] + PLANE_OVERHANG_UM],
        [y[0] - PLANE_OVERHANG_UM, y[1] + PLANE_OVERHANG_UM],
    );
    let top = top_of(made);
    let thinning = matches!(step.operation, Operation::Planarize { .. })
        && step.process == "thermal-oxidation";
    let um = |v: f64| format!("{} µm", num(v));
    let mut films = Vec::new();

    if let (Some((film, si)), Some((oxide, ox))) =
        (deposit("direct-bonding"), deposit("thermal-oxidation"))
    {
        let r = smart_cut::route(film.thickness.to_um(), oxide.thickness.to_um())?;
        let o = r.oxide;
        let silicon = paint(&film.name, &si.name, eps(&si.optical)?);
        let donor = paint("donor", &si.name, eps(&si.optical)?);
        let hydrogen = paint("implant", HYDROGEN, eps(&si.optical)?);
        let bond = paint("bond", BOND_INTERFACE, eps(&ox.optical)?);
        let half = r.straggle.max(PLANE_DRAWN_UM / 2.0);
        let plane = |z: f64| prism(&hydrogen, &past, z - half, z + half);
        // the bond is between the handle's silicon (below z = 0) and the donor's oxide
        let interface = || prism(&bond, &past, -PLANE_DRAWN_UM / 2.0, PLANE_DRAWN_UM / 2.0);
        let note = match step.process.as_str() {
            "h-implant" => {
                films.extend(plane(-r.split));
                Some(format!(
                    "The hydrogen's peak, {} below the oxide, drawn one straggle ({}) to each \
                     side; past the window's sides, so it shows in 3D.",
                    um(r.split),
                    um(r.straggle)
                ))
            }
            "direct-bonding" => {
                films.extend(interface());
                films.extend(prism(&silicon, &window, top, o + r.split));
                films.extend(prism(
                    &donor,
                    &window,
                    o + r.split,
                    o + r.split + DONOR_DRAWN_UM,
                ));
                films.extend(plane(o + r.split));
                Some(format!(
                    "The donor turned over onto the handle, its oxide bonded to the handle's \
                     silicon (the line at the oxide's foot). Above the design's film, its {} of \
                     excess up to the implanted plane, and the rest of the donor, drawn {} thick: \
                     a wafer is hundreds of micrometres.",
                    um(r.excess),
                    um(DONOR_DRAWN_UM)
                ))
            }
            "smart-cut-split" => {
                films.extend(interface());
                films.extend(prism(&silicon, &window, top, o + r.split));
                Some(format!(
                    "Split along the implanted plane: the rest of the donor is gone, and {} of \
                     silicon stays, {} more than the design's film. The bond is still weak.",
                    um(r.split),
                    um(r.excess)
                ))
            }
            "anneal-bond" => {
                films.extend(prism(&silicon, &window, top, o + r.split));
                Some("The bond strengthened: its line is gone.".into())
            }
            "cmp-si" => {
                films.extend(prism(&silicon, &window, top, o + r.split - r.polish));
                Some(format!(
                    "{} polished off; {} of excess left for the sacrificial oxide.",
                    um(r.polish),
                    um(r.excess - r.polish)
                ))
            }
            _ if thinning => {
                let sacrificial = paint("sacrificial oxide", &ox.name, eps(&ox.optical)?);
                films.extend(prism(&sacrificial, &window, top, top + r.sacrificial));
                Some(format!(
                    "The sacrificial oxide, {}: it has grown into the silicon by {}, down to \
                     the design's film.",
                    um(r.sacrificial),
                    um(r.consumed)
                ))
            }
            _ => None,
        };
        return Ok((films, note));
    }

    if let (Some((oxide, ox)), Some((film, si))) = (deposit("o-implant"), deposit("simox-anneal")) {
        let r = simox::route(film.thickness.to_um(), oxide.thickness.to_um())?;
        let silicon = paint(&film.name, &si.name, eps(&si.optical)?);
        let note = match step.process.as_str() {
            "o-implant" => {
                let damaged = paint(&film.name, DAMAGED_SI, eps(&si.optical)?);
                films.extend(prism(&damaged, &window, top, top + r.made));
                Some(format!(
                    "Over the buried oxide, about {} of silicon the oxygen passed through, \
                     damaged until the anneal heals it.",
                    um(r.made)
                ))
            }
            "simox-anneal" if r.thinned > 0.0 => {
                films.extend(prism(&silicon, &window, top, top + r.thinned));
                Some(format!(
                    "The film healed, {} more than the design's: what the sacrificial oxide \
                     takes off.",
                    um(r.thinned)
                ))
            }
            _ if thinning => {
                let sacrificial = paint("sacrificial oxide", &ox.name, eps(&ox.optical)?);
                films.extend(prism(&sacrificial, &window, top, top + r.sacrificial));
                Some(format!(
                    "The sacrificial oxide, {}: it has grown into the silicon by {}, down to \
                     the design's film.",
                    um(r.sacrificial),
                    um(r.thinned)
                ))
            }
            _ => None,
        };
        return Ok((films, note));
    }
    Ok((films, None))
}

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
//! - **A film evaporated for a lift-off**, on the resist and in its openings.
//!
//! The last frame is the last stage, the structure the round trip checks.

use serde::Serialize;

use super::Event;
use super::fabrication::named;
use super::fabrication_films::{
    Area, FrameFilm, Open, Paint, conformal, fill, minus, open_layers, prism, reach, rect,
    stand_in, top_of, union,
};
use super::scene::{SceneEtched, scene, scene_etched};
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

    /// A stencil with `film`, `t` thick, put down over it: in its openings, at the top the
    /// design draws the layer at, and on the resist around them.
    fn evaporated(&self, film: &Paint, t: f64) -> Vec<FrameFilm> {
        let mut out = self.developed();
        out.extend(prism(film, self.gone(), self.top, self.top + t));
        out.extend(prism(
            film,
            self.kept(),
            self.top + self.thickness,
            self.top + self.thickness + t,
        ));
        out
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
    struct Moment {
        step: usize,
        phase: String,
        label: String,
        layer: Option<String>,
        made: Made,
        films: Vec<FrameFilm>,
        note: Option<String>,
    }
    // the window, grown to hold every conformal film
    let mut grow: f64 = 0.0;
    for (k, step) in flow.steps.iter().enumerate().skip(1) {
        if let Some(d) = draped(step, &stages[k - 1]) {
            grow = grow.max(reach(&d.open, d.t, d.n) + 0.5 - 1.0);
        }
    }
    let (x, y) = ([x[0] - grow, x[1] + grow], [y[0] - grow, y[1] + grow]);
    let window = rect(x, y);
    let eps = |m: &Material| -> Result<f64> { Ok(m.permittivity(wavelength)?.re) };
    let records = flow.record(None).steps;
    let mut moments: Vec<Moment> = Vec::new();
    let mut resist: Option<Resist> = None;
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
                let polarity = litho.and_then(|p| p.polarity.clone());
                let shapes = union(mask);
                let all: Area = window.clone();
                let r = Resist {
                    name: resist_name(id),
                    thickness: thickness.map_or(UNSTATED_RESIST_UM, Length::to_um),
                    stated: thickness.is_some(),
                    tone: *tone,
                    polarity: polarity.as_ref().map_or(Polarity::Positive, |p| p.polarity),
                    named: polarity.is_some(),
                    imprint: litho.is_some_and(|p| p.imprints),
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
                        let paint = Paint {
                            layer: film.name.clone(),
                            film: film.name.clone(),
                            eps: eps(&film.optical)?,
                        };
                        let mut note: Vec<String> = r.note().into_iter().collect();
                        if !fill(&paint, &r.open, r.gone()).is_empty() {
                            note.push(format!(
                                "The design draws the {} on the top of the etched {}, over the \
                                 regions etched open, as the planar model has every layer: it is \
                                 drawn there, though in an opening over an etched region it would \
                                 land on the region's floor.",
                                film.name,
                                r.open
                                    .iter()
                                    .map(|o| o.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" and ")
                            ));
                        }
                        (
                            r.evaporated(&paint, thickness.to_um()),
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
                        push(
                            &record.operation,
                            record.does.clone(),
                            record.layer.clone(),
                            made.clone(),
                            Vec::new(),
                            None,
                        );
                    }
                }
            }
        }
    }
    let top = moments
        .iter()
        .map(|m| {
            m.films
                .iter()
                .map(|f| f.z_um[1])
                .fold(m.made.structure.stack().thickness().to_um(), f64::max)
        })
        .fold(design_top, f64::max);
    let z = [z_bottom, top + 1.0];
    moments
        .into_iter()
        .map(|m| {
            let mut drawn = scene(&m.made.structure, x, y, z, wavelength)?;
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

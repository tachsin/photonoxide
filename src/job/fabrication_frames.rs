//! A flow's frames: the wafer at each moment the studio's Fabrication page plays, from the bare
//! wafer to the design. Most steps are one frame, the executor's stage after it
//! ([`crate::fabrication::stages`]); a lithography is three, the resist spun on, the pattern
//! written into it and the resist developed. The resist, which the stages leave out (it changes
//! no layer of the design), is drawn here as a layer on the wafer's top from its lithography to
//! the etch or lift-off that uses it and the strip that removes it; an evaporation for a lift-off
//! draws its film on the resist and in its openings.
//!
//! The resist lies flat on the wafer's top, as the planar model has every layer; it is drawn with
//! silica's optics, which are near any resist's (its index is between 1.4 and 1.5 at 1.55 µm),
//! and named for itself, so a cross-section colours it apart from the oxides. A resist whose
//! thickness no source in hand states is drawn [`UNSTATED_RESIST_UM`] thick, and its frames say so.

use serde::Serialize;

use super::Event;
use super::fabrication::named;
use super::scene::{SceneEtched, scene, scene_etched};
use crate::Result;
use crate::fabrication::{Flow, Made, Operation, Tone};
use crate::geometry::Shape;
use crate::material::{Material, silica, vacuum};
use crate::stack::{Etch, Layer, LayerStack, Structure};
use crate::units::{Length, Wavelength};

/// The thickness a resist is drawn at when no source in hand states it, µm.
pub const UNSTATED_RESIST_UM: f64 = 0.3;

/// The wafer at one moment of its flow.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FlowFrame {
    /// The step it belongs to, from 0.
    pub step: usize,
    /// What happens to the wafer: the step's operation (`"wafer"`, `"deposit"`, `"etch"`, …, as
    /// [`crate::fabrication::StepRecord::operation`] names it), or a lithography's `"coat"`,
    /// `"expose"` and `"develop"`.
    pub phase: String,
    /// What happens, in a few words.
    pub label: String,
    /// The layer it acts on: the layer made, etched or lifted off, `"resist"` for a
    /// lithography's phases and the strip; none for a step that changes no layer.
    pub layer: Option<String>,
    /// The wafer as it leaves it ([`Event::Scene`]), its media named by the films there.
    pub scene: Event,
    /// The etched layers' shapes with their etches ([`scene_etched`]).
    pub etched: Vec<SceneEtched>,
    /// What the picture can't take from a source, if anything: a resist's thickness.
    pub note: Option<String>,
}

/// A resist on the wafer: its film's name, its thickness drawn, whether a source states it, the
/// shapes it is patterned on and its tone.
struct Resist {
    name: String,
    thickness: Length,
    stated: bool,
    mask: Vec<Shape>,
    tone: Tone,
}

/// A layer drawn over a stage: its name, thickness, the film in its shapes and around them (each
/// with the optics drawn), and its shapes.
struct Extra {
    name: String,
    thickness: Length,
    inside: (String, Material),
    outside: (String, Material),
    shapes: Vec<Shape>,
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

fn air() -> (String, Material) {
    ("air".into(), vacuum())
}

impl Resist {
    fn film(&self) -> (String, Material) {
        (self.name.clone(), silica())
    }

    /// The resist spun on: a flat film over the wafer.
    fn coat(&self) -> Vec<Extra> {
        vec![Extra {
            name: "resist".into(),
            thickness: self.thickness,
            inside: self.film(),
            outside: self.film(),
            shapes: Vec::new(),
        }]
    }

    /// The pattern written into it: the shapes in a lighter shade.
    fn written(&self) -> Vec<Extra> {
        vec![Extra {
            name: "resist".into(),
            thickness: self.thickness,
            inside: (format!("{}, written", self.name), silica()),
            outside: self.film(),
            shapes: self.mask.clone(),
        }]
    }

    /// Developed: on the shapes (an etch mask) or everywhere but the shapes (a stencil).
    fn developed(&self) -> Vec<Extra> {
        let (inside, outside) = match self.tone {
            Tone::Keep => (self.film(), air()),
            Tone::Open => (air(), self.film()),
        };
        vec![Extra {
            name: "resist".into(),
            thickness: self.thickness,
            inside,
            outside,
            shapes: self.mask.clone(),
        }]
    }

    /// A stencil with `film`, `t` thick, put down over it: in its openings, on the wafer, and on
    /// the resist around them.
    fn evaporated(&self, film: &(String, Material), t: Length) -> Vec<Extra> {
        let low = if t < self.thickness {
            t
        } else {
            self.thickness
        };
        let mut out = vec![Extra {
            name: "resist".into(),
            thickness: low,
            inside: film.clone(),
            outside: self.film(),
            shapes: self.mask.clone(),
        }];
        if self.thickness > low {
            out.push(Extra {
                name: "resist top".into(),
                thickness: self.thickness - low,
                inside: air(),
                outside: self.film(),
                shapes: self.mask.clone(),
            });
        }
        out.push(Extra {
            name: format!("{} on resist", film.0),
            thickness: t,
            inside: air(),
            outside: film.clone(),
            shapes: self.mask.clone(),
        });
        out
    }

    fn note(&self) -> Option<String> {
        (!self.stated).then(|| {
            format!(
                "No source in hand states the {}'s thickness: it is drawn {UNSTATED_RESIST_UM} µm \
                 thick.",
                self.name
            )
        })
    }
}

/// `made` with `extra` laid on its top (a bare wafer's stand-in air layer taken away, so they lie
/// on the wafer itself).
fn topped(made: &Made, extra: Vec<Extra>) -> Result<Made> {
    if extra.is_empty() {
        return Ok(made.clone());
    }
    let s = &made.structure;
    let stack = s.stack();
    let keep: Vec<usize> = (0..stack.layers().len())
        .filter(|&i| {
            !(stack.layers()[i].name == "air"
                && made.layers[i].0 == "air"
                && made.layers[i].1 == "air"
                && s.shapes("air").is_empty())
        })
        .collect();
    let mut layers: Vec<Layer> = keep.iter().map(|&i| stack.layers()[i].clone()).collect();
    let mut names: Vec<(String, String)> = keep.iter().map(|&i| made.layers[i].clone()).collect();
    for e in &extra {
        layers.push(Layer {
            name: e.name.clone(),
            thickness: e.thickness,
            material: e.inside.1.clone(),
            background: e.outside.1.clone(),
        });
        names.push((e.inside.0.clone(), e.outside.0.clone()));
    }
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
    for e in extra {
        for shape in e.shapes {
            structure.draw_etched(&e.name, shape, Etch::FULL)?;
        }
    }
    Ok(Made {
        structure,
        substrate: made.substrate.clone(),
        layers: names,
        cladding: made.cladding.clone(),
    })
}

/// The frames of `flow`, whose stages are `stages`, over the window `x` × `y` from `z_bottom`
/// to 1 µm above the highest top any frame or `design_top` reaches, at `wavelength`.
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
        note: Option<String>,
    }
    let records = flow.record(None).steps;
    let mut moments: Vec<Moment> = Vec::new();
    let mut resist: Option<Resist> = None;
    for (k, (step, made)) in flow.steps.iter().zip(stages).enumerate() {
        let record = &records[k];
        let mut push = |phase: &str, label: String, layer: Option<String>, made: Made, note| {
            moments.push(Moment {
                step: k,
                phase: phase.into(),
                label,
                layer,
                made,
                note,
            });
        };
        match &step.operation {
            Operation::Pattern {
                resist: id,
                thickness,
                mask,
                tone,
            } => {
                let r = Resist {
                    name: resist_name(id),
                    thickness: thickness.unwrap_or(Length::um(UNSTATED_RESIST_UM)),
                    stated: thickness.is_some(),
                    mask: mask.clone(),
                    tone: *tone,
                };
                let n = mask.len();
                let shapes = format!("{n} shape{}", if n == 1 { "" } else { "s" });
                let layer = Some("resist".to_owned());
                push(
                    "coat",
                    format!("spin the {} on", r.name),
                    layer.clone(),
                    topped(made, r.coat())?,
                    r.note(),
                );
                push(
                    "expose",
                    format!("write the {shapes}' pattern into the {}", r.name),
                    layer.clone(),
                    topped(made, r.written())?,
                    r.note(),
                );
                push(
                    "develop",
                    match tone {
                        Tone::Keep => format!("develop: the {} stays on the {shapes}", r.name),
                        Tone::Open => format!("develop: the {} opens over the {shapes}", r.name),
                    },
                    layer,
                    topped(made, r.developed())?,
                    r.note(),
                );
                resist = Some(r);
            }
            Operation::Evaporate { film, thickness } => {
                let (extra, note) = match &resist {
                    Some(r) => (
                        r.evaporated(&(film.name.clone(), film.optical.clone()), *thickness),
                        r.note(),
                    ),
                    None => (Vec::new(), None),
                };
                push(
                    &record.operation,
                    record.does.clone(),
                    record.layer.clone(),
                    topped(made, extra)?,
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
                    None,
                );
                resist = None;
            }
            _ => {
                let (extra, note) = match &resist {
                    Some(r) => (r.developed(), r.note()),
                    None => (Vec::new(), None),
                };
                push(
                    &record.operation,
                    record.does.clone(),
                    record.layer.clone(),
                    topped(made, extra)?,
                    note,
                );
            }
        }
    }
    let top = moments
        .iter()
        .map(|m| m.made.structure.stack().thickness().to_um())
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
                note: m.note,
            })
        })
        .collect()
}

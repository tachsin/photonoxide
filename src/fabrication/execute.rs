//! A flow executed on the library's own geometry, and the round trip: the structure the flow
//! makes against the design's.
//!
//! The exact model is a [`Structure`]: the wafer's layers, a deposition's layers on top (filling
//! every region etched open so far, which sets those layers' background), an etch's shapes drawn
//! on its layer with its depth and sidewall, a lift-off's shapes drawn on the layer it makes. A
//! polish leaves the planar model as it is, since it is flat by construction.

use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Shape};
use crate::stack::{Etch, Layer, LayerStack, Structure};
use crate::units::Length;
use crate::{Error, Result};

use super::flow::{Depth, Flow, Operation, Tone};
use super::physical::{Film, Physical};

/// What a flow makes: its structure, and the films there.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    /// The structure, its materials the optical ones.
    pub structure: Structure,
    /// The substrate's film.
    pub substrate: String,
    /// Each layer's drawn shapes' film and its background's, bottom to top.
    pub layers: Vec<(String, String)>,
    /// The cladding's film.
    pub cladding: String,
}

struct Building {
    name: String,
    thickness: Length,
    film: Film,
    background: Option<Film>,
    shapes: Vec<Shape>,
    etches: Vec<Etch>,
}

fn invalid(message: String) -> Error {
    Error::invalid("fabrication", message)
}

/// Runs `flow`'s operations from the bare wafer.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a flow that doesn't hold together: an etch with no resist on the
/// wafer or of a layer that isn't on top, a lift-off without an evaporation into an opened resist,
/// a polish that doesn't take off what was deposited for it, a layer etched open that nothing
/// fills, or no wafer or cladding.
pub fn execute(flow: &Flow) -> Result<Made> {
    run(&flow.steps, false)
}

/// The wafer after each of `flow`'s steps, for the cross-section of each: what is made so far,
/// with air where a region is etched open and above the top until the cladding goes on (the
/// resist isn't drawn). The last is [`execute`]'s.
///
/// # Errors
///
/// [`execute`]'s, for the steps up to each.
pub fn stages(flow: &Flow) -> Result<Vec<Made>> {
    (1..=flow.steps.len())
        .map(|k| run(&flow.steps[..k], k < flow.steps.len()))
        .collect()
}

/// The air a stage shows where nothing is yet.
fn air() -> Film {
    Film {
        name: "air".into(),
        optical: crate::material::vacuum(),
    }
}

/// Runs `steps`; `partial`, the wafer so far, with air where nothing is yet.
fn run(steps: &[super::flow::Step], partial: bool) -> Result<Made> {
    let mut substrate: Option<Film> = None;
    let mut cladding: Option<Film> = None;
    let mut layers: Vec<Building> = Vec::new();
    let mut resist: Option<(Vec<Shape>, Tone)> = None;
    let mut evaporated: Option<(Film, Length)> = None;
    let mut extra = Length::ZERO;
    for (k, step) in steps.iter().enumerate() {
        let at = |what: &str| invalid(format!("step {} ({}): {what}", k + 1, step.name));
        match &step.operation {
            Operation::Wafer {
                substrate: s,
                layers: laid,
            } => {
                if substrate.is_some() {
                    return Err(at("a second wafer"));
                }
                substrate = Some(s.clone());
                for (l, film) in laid {
                    layers.push(Building {
                        name: l.name.clone(),
                        thickness: l.thickness,
                        film: film.clone(),
                        background: None,
                        shapes: Vec::new(),
                        etches: Vec::new(),
                    });
                }
            }
            Operation::Deposit {
                film,
                layers: laid,
                cladding: clad,
                extra: more,
            } => {
                if substrate.is_none() || cladding.is_some() {
                    return Err(at("a deposition with no wafer, or over the cladding"));
                }
                for l in &mut layers {
                    if !l.shapes.is_empty() && l.background.is_none() {
                        l.background = Some(film.clone());
                    }
                }
                for l in laid {
                    layers.push(Building {
                        name: l.name.clone(),
                        thickness: l.thickness,
                        film: film.clone(),
                        background: None,
                        shapes: Vec::new(),
                        etches: Vec::new(),
                    });
                }
                if *clad {
                    cladding = Some(film.clone());
                }
                extra = *more;
            }
            Operation::Planarize { removed } => {
                if *removed != extra {
                    return Err(at(&format!(
                        "the polish takes off {removed} where {extra} was deposited for it"
                    )));
                }
                extra = Length::ZERO;
            }
            Operation::Pattern { mask, tone, .. } => {
                if resist.is_some() {
                    return Err(at("resist over resist"));
                }
                resist = Some((mask.clone(), *tone));
            }
            Operation::Etch {
                layer,
                depth,
                sidewall,
            } => {
                let Some((mask, Tone::Keep)) = &resist else {
                    return Err(at("an etch with no resist kept on the shapes"));
                };
                let top = layers
                    .last_mut()
                    .filter(|l| &l.name == layer)
                    .ok_or_else(|| at(&format!("{layer} isn't the top layer")))?;
                let etch = match depth {
                    Depth::ToStop => Etch::FULL.with_sidewall(*sidewall),
                    Depth::Timed(d) => Etch::partial(*d).with_sidewall(*sidewall),
                };
                for s in mask {
                    top.shapes.push(s.clone());
                    top.etches.push(etch);
                }
            }
            Operation::Evaporate { film, thickness } => {
                if !matches!(resist, Some((_, Tone::Open))) {
                    return Err(at("an evaporation for lift-off with no opened resist"));
                }
                evaporated = Some((film.clone(), *thickness));
            }
            Operation::LiftOff { layer } => {
                let (Some((mask, Tone::Open)), Some((film, thickness))) =
                    (resist.take(), evaporated.take())
                else {
                    return Err(at("a lift-off with no evaporated film on an opened resist"));
                };
                let n = mask.len();
                layers.push(Building {
                    name: layer.clone(),
                    thickness,
                    film,
                    background: None,
                    shapes: mask,
                    etches: vec![Etch::FULL; n],
                });
            }
            Operation::Strip => {
                if resist.take().is_none() {
                    return Err(at("no resist to remove"));
                }
            }
            Operation::Undrawn { .. } => {}
        }
    }
    let substrate = substrate.ok_or_else(|| invalid("the flow has no wafer".into()))?;
    let cladding = match cladding {
        Some(c) => c,
        None if partial => air(),
        None => return Err(invalid("the flow never covers the wafer".into())),
    };
    if !partial && (resist.is_some() || extra != Length::ZERO) {
        return Err(invalid(
            "the flow ends with resist on the wafer or a deposition not polished".into(),
        ));
    }
    let mut stack_layers = Vec::with_capacity(layers.len());
    let mut names = Vec::with_capacity(layers.len());
    for l in &layers {
        let background = match (&l.background, l.shapes.is_empty()) {
            (Some(b), _) => b.clone(),
            (None, true) => l.film.clone(),
            (None, false) if partial => air(),
            (None, false) => {
                return Err(invalid(format!(
                    "{} is etched open and nothing fills it",
                    l.name
                )));
            }
        };
        names.push((l.film.name.clone(), background.name.clone()));
        stack_layers.push(Layer {
            name: l.name.clone(),
            thickness: l.thickness,
            material: l.film.optical.clone(),
            background: background.optical,
        });
    }
    let stack = LayerStack::new(
        substrate.optical.clone(),
        stack_layers,
        cladding.optical.clone(),
    )?;
    let mut structure = Structure::new(stack);
    for l in &layers {
        for (s, e) in l.shapes.iter().zip(&l.etches) {
            structure.draw_etched(&l.name, s.clone(), *e)?;
        }
    }
    Ok(Made {
        structure,
        substrate: substrate.name,
        layers: names,
        cladding: cladding.name,
    })
}

/// The round trip's result: whether the flow's structure is the design's, to the bit, with the
/// same films, and on how many of the points compared (a grid over the design's shapes, through
/// every layer) the material or the film differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundTrip {
    /// Whether the structure equals the design's (`PartialEq`, to the bit).
    pub structure_equal: bool,
    /// Whether every film is the design's.
    pub films_equal: bool,
    /// The points compared.
    pub points: usize,
    /// The points whose material or film differs.
    pub differing: usize,
}

impl RoundTrip {
    /// Whether the flow makes the design exactly: equal structures and films, and no point
    /// differing.
    pub fn exact(&self) -> bool {
        self.structure_equal && self.films_equal && self.differing == 0
    }
}

/// Points across a grid: `n` along each side.
const GRID: usize = 48;

/// The film at (p, z) of a structure whose films are `films`.
fn film_at<'a>(
    s: &Structure,
    substrate: &'a str,
    layers: &'a [(String, String)],
    cladding: &'a str,
    p: Point,
    z: f64,
) -> &'a str {
    if z < 0.0 {
        return substrate;
    }
    let mut bottom = 0.0;
    for (l, (drawn, background)) in s.stack().layers().iter().zip(layers) {
        let top = bottom + l.thickness.to_um();
        if z < top {
            let kept = s.kept(&l.name, p).map_or(0.0, Length::to_um);
            return if z - bottom < kept { drawn } else { background };
        }
        bottom = top;
    }
    cladding
}

/// Executes `flow` and compares what it makes with `design`.
///
/// # Errors
///
/// [`execute`]'s.
pub fn round_trip(flow: &Flow, design: &Physical) -> Result<RoundTrip> {
    let made = execute(flow)?;
    let want = design.structure();
    let (substrate, layers, cladding) = design.names();
    let films_equal =
        made.substrate == substrate && made.layers == layers && made.cladding == cladding;
    // the window: every shape of either, and 1 µm around
    let (mut x0, mut x1, mut y0, mut y1) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut first = true;
    for s in [want, &made.structure] {
        for l in s.stack().layers() {
            for shape in s.shapes(&l.name) {
                let b = shape.bounds();
                let (a, c) = (b.min, b.max);
                if first {
                    (x0, x1, y0, y1) = (a.x.to_um(), c.x.to_um(), a.y.to_um(), c.y.to_um());
                    first = false;
                }
                x0 = x0.min(a.x.to_um());
                x1 = x1.max(c.x.to_um());
                y0 = y0.min(a.y.to_um());
                y1 = y1.max(c.y.to_um());
            }
        }
    }
    let (x0, x1, y0, y1) = (x0 - 1.0, x1 + 1.0, y0 - 1.0, y1 + 1.0);
    // heights: below the stack, three in each layer (of the taller of the two), above it
    let mut zs = vec![-0.1];
    let mut bottom = 0.0;
    for l in want.stack().layers() {
        let t = l.thickness.to_um();
        zs.extend([bottom + 0.05 * t, bottom + 0.5 * t, bottom + 0.95 * t]);
        bottom += t;
    }
    zs.push(bottom.max(made.structure.stack().thickness().to_um()) + 0.1);
    let mut points = 0;
    let mut differing = 0;
    for i in 0..GRID {
        let x = x0 + (x1 - x0) * (i as f64 + 0.5) / GRID as f64;
        for j in 0..GRID {
            let y = y0 + (y1 - y0) * (j as f64 + 0.5) / GRID as f64;
            let p = Point::um(x, y);
            for &z in &zs {
                points += 1;
                let same_material = want.material_at(p, Length::um(z))
                    == made.structure.material_at(p, Length::um(z));
                let same_film = film_at(want, &substrate, &layers, &cladding, p, z)
                    == film_at(
                        &made.structure,
                        &made.substrate,
                        &made.layers,
                        &made.cladding,
                        p,
                        z,
                    );
                if !(same_material && same_film) {
                    differing += 1;
                }
            }
        }
    }
    Ok(RoundTrip {
        structure_equal: made.structure == *want,
        films_equal,
        points,
        differing,
    })
}

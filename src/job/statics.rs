//! The `"static"` job: heat conduction, electrostatics or current continuity on a cross-section
//! of a stack and its shapes ([`crate::thermal`], [`crate::electro`]).
//!
//! The section is the plane x = `cut_x_um` (light travels along x, as in every kind of job):
//! across it y, up it z, over the window `y_um` × `z_um`. The grid is fitted to the stack's
//! interfaces, the shapes' edges along the cut and every rectangle the task draws in the
//! section, its cells `fine_nm` at their corners (by default `step_nm`) growing by `growth` (1.2
//! by default) to `step_nm`.
//!
//! - `physics = "heat"`: `[task.thermal_conductivity]` gives each medium's k in W/(m·K), by its
//!   name (a catalogue material's, as `Si` and `SiO2` in the presets, or a region's); a number,
//!   or `[across, up]`. A medium it doesn't name takes the catalogue's conductivity
//!   ([`crate::material::catalogue::conductivity`]: Si, SiO2, W, Ti, Pt) at `temperature_k`
//!   (300 K by default): a stack layer's material with a film model (silicon) the film's at
//!   that layer's thickness, and a substrate of the same material, drawn apart as
//!   `"Si (substrate)"`, the bulk's. The run records each value it took and where from.
//!   `[[task.heater]]` rectangles give W/m, `[[task.sink]]` rectangles hold a temperature.
//! - `physics = "electrostatic"`: `[task.permittivity]` the relative static permittivities;
//!   `[[task.conductor]]` rectangles held at a potential (V), whose capacitance matrix (F/m) the
//!   run records. With `[task.electro_optic]` the stack may hold an oriented crystal, and the
//!   run takes the field into a mode of the same cut by the Pockels effect and records
//!   dn_eff/dV and Vπ·L ([`super::pockels`], docs/methods/electro-optic.md).
//! - `physics = "current"`: `[task.electrical_conductivity]` in S/m, the `[[task.conductor]]`
//!   rectangles its contacts; with a `[task.thermal_conductivity]` too, the Joule heat is then
//!   conducted as a heat problem on the same grid.
//!
//! `[task.boundary]` holds each side (`left`, `right`, `bottom`, `top`): `"insulated"` (the
//! default: no flux, or a mirror), `{ fixed = 0.0 }`, `{ flux = 0.0 }` (W/m² or C/m² into the
//! window) or `{ convective = 10.0, ambient = 0.0 }` (W/(m²·K)). `[[task.region]]` rectangles
//! draw a named `medium` over the cut (a heater's metal); `[[task.probe]]` rectangles name the
//! means the run records.
//!
//! **In time** (heat only): `[task.transient]` turns the heaters on at t = 0 from the steady
//! state without them (`turn = "on"`, the default), or off from the steady state with them
//! (`"off"`), and steps `duration_us` by `step_ns`, implicitly: `scheme = "crank-nicolson"` (the
//! default, the more accurate of the two at the same step on the slab's series solution) or
//! `"bdf2"` ([`crate::thermal::Scheme`]). `[task.heat_capacity]` gives each medium's ρc_p in
//! J/(m³·K). The run records each probe's mean as the steps go ([`Event::StaticStep`]), the
//! last field, and each probe's 1/e and 10–90 % times and the −3 dB frequency of its
//! small-signal response to the heaters' power.
//!
//! **A pulse:** `pulse_ns` in `[task.transient]` turns the heaters on at t = 0 for that long, a
//! whole number of steps, then off to `duration_us` ([`crate::thermal::Heat::pulse`]; BDF2
//! unless the scheme is given, which takes the heat exactly to the pulse's end). The run
//! records each probe's hottest and coldest cell at their highest (the cells at least half
//! inside it), and for each `[[task.switching]]` (`probe`, and `material`, a phase-change
//! material of the catalogue: `sb2se3`, …) its crystallization and melting temperatures and
//! what the pulse does to the film there ([`crate::thermal::Switch`]: `"film after the pulse:
//! amorphized"`, …). The temperatures are absolute, so the job holds its sides or sinks at the
//! ambient (293.15 K, say). A medium named by a phase-change state's catalogue id
//! (`sb2se3-crystalline`) takes the catalogue's conductivity and ρc_p unless the job gives
//! them. jobs/pcm-switching.yaml amorphizes 25 nm of Sb₂Se₃ on a silicon strip.
//!
//! **A phase shifter** (heat, or a current's Joule heat): `[task.shifter]` (`wavelength_um`,
//! `step_nm` of the optical grid, and `index` and `dn_dt` tables by medium in place of the
//! catalogue's, whose default models give the rest at `temperature_k`) with `[[task.guide]]`
//! rectangles, each a guide's mode window ([`crate::thermal::PhaseShifter`]). Each heater is a
//! drive (a current's Joule heat the one drive), the k-th heater's own guide the k-th guide. The
//! run records each guide's n_eff and dn_eff/dT, each guide's ∂n_eff/∂P′ by each drive (m/W),
//! each drive's P_π on its own guide (W, a 2D number: any length of a straight section) and its
//! crosstalk on the others (the share of its own phase), and with `[task.transient]` each
//! drive's phase's 1/e and 10–90 % times, −3 dB frequency and P_π times its 1/e time. With
//! `poles` too, a dynamic thermal compact model of that many poles is fitted to every drive's
//! step response of every guide's phase ([`crate::thermal::RcModel`], Coenen et al. 2022): the
//! run records each fit's error from 1 pole up, and the model's time constants and residues
//! (rad/W), what [`crate::circuit::Heated`] takes.
//!
//! ```yaml
//! name: heater
//! timeout_minutes: 5
//!
//! task:
//!   kind: static
//!   physics: heat
//!   stack: soi_220
//!   y_um: [-20.0, 20.0]
//!   z_um: [-10.0, 6.0]
//!   step_nm: 1000.0
//!   fine_nm: 20.0
//!
//!   rect:
//!     - { layer: Si, center_um: [0.0, 0.0], size_um: [10.0, 0.5] }
//!
//!   thermal_conductivity: { Si: 148.0, SiO2: 1.4, TiN: 28.0 }
//!
//!   boundary:
//!     bottom: { fixed: 0.0 }
//!     top: { convective: 10.0, ambient: 0.0 }
//!
//!   region:
//!     - { y_um: [-1.0, 1.0], z_um: [4.22, 4.34], medium: TiN }
//!
//!   heater:
//!     - { y_um: [-1.0, 1.0], z_um: [4.22, 4.34], power_w_per_m: 10.0 }
//!
//!   probe:
//!     - { name: guide, y_um: [-0.25, 0.25], z_um: [2.0, 2.22] }
//!
//!   # in time: with these, the heater's turn-on
//!   heat_capacity: { Si: 1.657e6, SiO2: 1.562e6, TiN: 3.134e6 }
//!
//!   transient:
//!     step_ns: 200.0
//!     duration_us: 40.0
//! ```
//!
//! The run records an [`Event::StaticField`] per field (the temperature or the potential, and a
//! current's Joule heat), each in its unit on pixels of about the finest step, and an
//! [`Event::StaticResult`] with the grid and the numbers: the probes' means, the largest value,
//! the heat or charge of each held rectangle, what leaves through each side, the balance's
//! imbalance, and a capacitance matrix or the contacts' currents and Joule power.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    CircleSpec, Event, PathSpec, RectSpec, RingSpec, StackField, draw, draw_paths, named_stack,
    task_error,
};
use crate::Result;
use crate::electro::{Conductor, Current, Electrostatic};
use crate::geometry::{Point, Shape};
use crate::run::{Job, Run, Stop};
use crate::statics::{Boundary, Cut, Features, Field, Grading, Grid, Section, Side, Tensor};
use crate::thermal::{Heat, Scheme, Stepping, Switch};
use crate::units::Length;

/// The most cells a static job's grid may have.
const MOST_CELLS: usize = 4_000_000;

/// The most time steps a transient may take.
const MOST_STEPS: usize = 100_000;

/// About how many of a transient's steps a run records.
const RECORDED_STEPS: usize = 2000;

/// The most pixels a recorded field has along a side.
const PIXELS: usize = 400;

/// A number of a static job's result, with its unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticValue {
    /// What it is, e.g. `"mean over guide"`.
    pub name: String,
    /// Its value.
    pub value: f64,
    /// Its unit, e.g. `"K"`, `"W/m"`, `"F/m"`.
    pub unit: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum CoefficientSpec {
    Isotropic(f64),
    Diagonal([f64; 2]),
}

impl CoefficientSpec {
    fn tensor(&self) -> Tensor {
        match *self {
            CoefficientSpec::Isotropic(v) => Tensor::isotropic(v),
            CoefficientSpec::Diagonal([a, b]) => Tensor::diagonal(a, b),
        }
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum SideSpec {
    Named(String),
    Fixed { fixed: f64 },
    Flux { flux: f64 },
    Convective { convective: f64, ambient: f64 },
}

impl SideSpec {
    fn boundary(&self, side: &str) -> Result<Boundary> {
        Ok(match self {
            SideSpec::Named(n) if n == "insulated" => Boundary::INSULATED,
            SideSpec::Named(n) => {
                return Err(task_error(format!(
                    "[task.boundary] {side} = \"{n}\": a side is \"insulated\", {{ fixed = … }}, \
                     {{ flux = … }} or {{ convective = …, ambient = … }}"
                )));
            }
            SideSpec::Fixed { fixed } => Boundary::Fixed(*fixed),
            SideSpec::Flux { flux } => Boundary::Flux(*flux),
            SideSpec::Convective {
                convective,
                ambient,
            } => Boundary::Convective {
                coefficient: *convective,
                ambient: *ambient,
            },
        })
    }
}

#[derive(Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "StaticBoundaries")]
struct BoundarySpec {
    left: Option<SideSpec>,
    right: Option<SideSpec>,
    bottom: Option<SideSpec>,
    top: Option<SideSpec>,
}

/// A rectangle in the section's plane: y across, z up, µm.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Area {
    y_um: [f64; 2],
    z_um: [f64; 2],
    medium: Option<String>,
    power_w_per_m: Option<f64>,
    temperature: Option<f64>,
    potential_v: Option<f64>,
    name: Option<String>,
}

impl Area {
    fn shape(&self, what: &str, k: usize) -> Result<Shape> {
        let ([y0, y1], [z0, z1]) = (self.y_um, self.z_um);
        if !(y0.is_finite()
            && y1.is_finite()
            && z0.is_finite()
            && z1.is_finite()
            && y0 < y1
            && z0 < z1)
        {
            return Err(task_error(format!(
                "[[task.{what}]] #{}: y_um and z_um must go from a smaller to a larger number",
                k + 1
            )));
        }
        Shape::rect(
            Point::um(0.5 * (y0 + y1), 0.5 * (z0 + z1)),
            Length::um(y1 - y0),
            Length::um(z1 - z0),
        )
    }

    fn need(&self, what: &str, k: usize, field: &str, value: Option<f64>) -> Result<f64> {
        value
            .filter(|v| v.is_finite())
            .ok_or_else(|| task_error(format!("[[task.{what}]] #{} needs a finite {field}", k + 1)))
    }
}

/// `[task.transient]`.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransientSpec {
    step_ns: f64,
    duration_us: f64,
    scheme: Option<String>,
    turn: Option<String>,
    /// The heaters on from t = 0 for this long, then off: a pulse, a whole number of steps.
    pulse_ns: Option<f64>,
}

/// A transient as a run takes it: its steps, whether the heaters turn on (or off), and a
/// pulse's length (s).
struct Transient {
    stepping: Stepping,
    on: bool,
    pulse_s: Option<f64>,
}

/// `[[task.switching]]`: a probe over a phase-change film and the material's id (`sb2se3`),
/// whose crystallization and melting temperatures the pulse's are set against.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SwitchingSpec {
    probe: String,
    material: String,
}

/// The phase-change material and state whose catalogue entry id is `medium`.
fn phase_state(
    medium: &str,
) -> Option<(
    crate::material::catalogue::PhaseChange,
    crate::material::catalogue::State,
)> {
    crate::material::catalogue::phase_changes()
        .into_iter()
        .find_map(|p| {
            let s = p.state_of(medium)?;
            Some((p, s))
        })
}

impl TransientSpec {
    fn transient(&self) -> Result<Transient> {
        let (step, duration) = (self.step_ns, self.duration_us);
        if !(step.is_finite() && step > 0.0 && duration.is_finite() && duration > 0.0) {
            return Err(task_error(format!(
                "[task.transient] needs a positive step_ns and duration_us, got {step} and \
                 {duration}"
            )));
        }
        let steps = (duration * 1e3 / step).round().max(1.0);
        if steps > MOST_STEPS as f64 {
            return Err(task_error(format!(
                "[task.transient] would take {steps} steps, more than {MOST_STEPS}: take a longer \
                 step_ns or a shorter duration_us"
            )));
        }
        let scheme = match self.scheme.as_deref() {
            // a pulse by BDF2 unless asked: its peak at the pulse's end to second order
            None if self.pulse_ns.is_some() => Scheme::Bdf2,
            None | Some("crank-nicolson") => Scheme::CrankNicolson,
            Some("bdf2") => Scheme::Bdf2,
            Some(other) => {
                return Err(task_error(format!(
                    "[task.transient] scheme = \"{other}\": a scheme is \"crank-nicolson\" or \
                     \"bdf2\""
                )));
            }
        };
        let on = match self.turn.as_deref() {
            None | Some("on") => true,
            Some("off") => false,
            Some(other) => {
                return Err(task_error(format!(
                    "[task.transient] turn = \"{other}\": the heaters turn \"on\" or \"off\""
                )));
            }
        };
        let pulse_s = match self.pulse_ns {
            None => None,
            Some(p) => {
                let k = p / step;
                if !(p.is_finite() && p > 0.0)
                    || (k - k.round()).abs() > 1e-6 * k.max(1.0)
                    || k.round() >= steps
                {
                    return Err(task_error(format!(
                        "[task.transient] pulse_ns = {p} must be a whole number of steps of \
                         {step} ns, shorter than duration_us"
                    )));
                }
                if !on {
                    return Err(task_error(
                        "[task.transient] a pulse turns the heaters on, then off: leave turn out",
                    ));
                }
                Some(p * 1e-9)
            }
        };
        Ok(Transient {
            stepping: Stepping {
                step_s: step * 1e-9,
                steps: steps as usize,
                scheme,
            },
            on,
            pulse_s,
        })
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct StaticTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    physics: String,
    stack: StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    cut_x_um: Option<f64>,
    y_um: [f64; 2],
    z_um: [f64; 2],
    step_nm: f64,
    fine_nm: Option<f64>,
    growth: Option<f64>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    #[serde(default)]
    path: Vec<PathSpec>,
    #[serde(default)]
    shape: Vec<super::shapes::ShapeSpec>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Component>")]
    component: Vec<toml::Table>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Body>")]
    body: Vec<toml::Table>,
    #[serde(default)]
    thermal_conductivity: BTreeMap<String, CoefficientSpec>,
    /// The temperature (K) at which a medium [task.thermal_conductivity] gives no value takes
    /// the catalogue's conductivity; 300 K by default.
    temperature_k: Option<f64>,
    #[serde(default)]
    electrical_conductivity: BTreeMap<String, CoefficientSpec>,
    #[serde(default)]
    permittivity: BTreeMap<String, CoefficientSpec>,
    #[serde(default)]
    heat_capacity: BTreeMap<String, f64>,
    transient: Option<TransientSpec>,
    #[serde(default)]
    switching: Vec<SwitchingSpec>,
    #[serde(default)]
    boundary: BoundarySpec,
    #[serde(default)]
    region: Vec<Area>,
    #[serde(default)]
    heater: Vec<Area>,
    #[serde(default)]
    sink: Vec<Area>,
    #[serde(default)]
    conductor: Vec<Area>,
    #[serde(default)]
    probe: Vec<Area>,
    /// The Pockels effect the electrostatic field makes in a mode, and Vπ·L
    /// ([`super::pockels`]).
    electro_optic: Option<super::pockels::ElectroOpticSpec>,
    /// What the run writes beside its record: the tables, and the temperature and the
    /// potential on the grid as an npz when asked ([`crate::export`]).
    #[allow(dead_code)] // read by super::export_of, checked by serde
    export: Option<super::ExportSpec>,
    /// `[task.shifter]`: the heaters as a thermo-optic phase shifter's drives.
    shifter: Option<ShifterSpec>,
    #[serde(default)]
    guide: Vec<Area>,
}

/// `[task.shifter]`: the optics of the guides `[[task.guide]]` draws (each a window around a
/// waveguide in the section), the heaters (or a current's Joule heat) as what drives them.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShifterSpec {
    /// The wavelength, µm.
    wavelength_um: f64,
    /// The optical grid's step, nm.
    step_nm: f64,
    /// Media's refractive indices, by name, in place of the catalogue's.
    #[serde(default)]
    index: BTreeMap<String, f64>,
    /// Media's dn/dT (1/K), by name, in place of the catalogue's.
    #[serde(default)]
    dn_dt: BTreeMap<String, f64>,
    /// With a `[task.transient]`: a dynamic thermal compact model of this many poles (1 to
    /// 12), fitted to every drive's step response of every guide's phase.
    poles: Option<usize>,
}

/// A rectangle of the section, y and z, µm.
type Window = ([f64; 2], [f64; 2]);

/// A phase shifter as a run takes it: its optics, the guides' windows and names, the drives.
struct Shifter {
    optics: crate::thermal::Optics,
    guides: Vec<(String, Window)>,
    heaters: Vec<(String, Shape, f64)>,
    /// Where each medium's optics came from, for the record.
    record: Vec<StaticValue>,
    /// The thermal compact model's poles, if one is asked for.
    poles: Option<usize>,
}

/// The fields of a static job as an npz ([`crate::export::Npz`]): `y_um`, `z_um` (the cells'
/// centres), `y_nodes_um`, `z_nodes_um` (their edges, the grid graded), and each of `fields`
/// that `export` keeps, `(nz, ny)` in its unit.
fn export_fields(
    path: &std::path::Path,
    meta: &crate::export::Meta,
    export: &super::ExportSpec,
    grid: &Grid,
    fields: &[(&str, &Field)],
) -> Result<u64> {
    let (x, y) = (grid.x(), grid.y());
    let centres =
        |a: &crate::statics::Axis| -> Vec<f64> { (0..a.cells()).map(|i| a.centre(i)).collect() };
    let mut npz = export.npz_file()?;
    npz.exact("y_um", &[x.cells()], &centres(x));
    npz.exact("z_um", &[y.cells()], &centres(y));
    npz.exact("y_nodes_um", &[x.nodes().len()], x.nodes());
    npz.exact("z_nodes_um", &[y.nodes().len()], y.nodes());
    for (name, field) in fields {
        if export.keeps(name) {
            npz.field(name, &[y.cells(), x.cells()], field.values());
        }
    }
    npz.meta(
        meta,
        &crate::export::Described {
            grid: "the cross-section across the guide (y) and up (z), graded: y_um and z_um \
                   name each cell's centre, where the values sit (crate::statics::Field), \
                   y_nodes_um and z_nodes_um its edges; each field (nz, ny), row j at z_um[j], \
                   column i at y_um[i]",
            units: "um for lengths; temperature in K, potential in V",
            normalization: "none: the solver's own values, held regions at their own",
        },
    );
    npz.write(path)
}

/// What a static job sets up: the section, its grid and the physics' problems.
struct Setup {
    physics: String,
    structure: crate::stack::Structure,
    cut: f64,
    grid: Grid,
    window: ([f64; 2], [f64; 2]),
    pixels: (usize, usize),
    heat: Option<Heat>,
    electrostatic: Option<Electrostatic>,
    current: Option<Current>,
    probes: Vec<(String, Shape)>,
    held: Vec<String>,
    transient: Option<Transient>,
    /// Each [[task.switching]]: its probe's index, the material's id, T_c and T_m (K).
    switching: Vec<(usize, String, f64, f64)>,
    /// The conductivities the catalogue gave, for the record.
    catalogued: Vec<StaticValue>,
    /// The electro-optic chain, with what it takes from the task.
    electro_optic: Option<super::pockels::Chain>,
    shifter: Option<Shifter>,
}

/// The name a substrate takes in a heat problem when its material is also a film's, whose
/// conductivity differs: the material's name and this.
const SUBSTRATE: &str = " (substrate)";

/// The conductivities the catalogue gives a heat problem's media that
/// `[task.thermal_conductivity]` doesn't: each medium's, W/(m·K), with the model it comes from;
/// and the substrate's material, when the substrate is to be drawn as a medium of its own
/// ([`SUBSTRATE`]) because that material is also a film's.
struct Catalogued {
    values: Vec<(String, f64, String)>,
    substrate: Option<String>,
}

/// [`Catalogued`] for the media `media` of a cut of `structure`. A medium that is the material
/// of a stack's layer, and has a film model in the catalogue (silicon), takes the film's
/// conductivity at that layer's thickness; its substrate, when of the same material, the bulk's.
fn catalogued(
    task: &StaticTask,
    structure: &crate::stack::Structure,
    media: &[String],
) -> Result<Catalogued> {
    use crate::material::catalogue;
    let kelvin = task.temperature_k.unwrap_or(300.0);
    if !(kelvin.is_finite() && kelvin > 0.0) {
        return Err(task_error(format!(
            "temperature_k must be above 0 K, got {kelvin}"
        )));
    }
    let stack = structure.stack();
    let mut out = Catalogued {
        values: Vec::new(),
        substrate: None,
    };
    let model_error = |m: &str, e: crate::Error| {
        task_error(format!(
            "[task.thermal_conductivity] has no value for the medium {m}, and the catalogue's \
             can't be taken: {e}"
        ))
    };
    for m in media {
        if task.thermal_conductivity.contains_key(m) {
            continue;
        }
        // a phase-change state named by its entry id, e.g. sb2se3-crystalline
        if let Some((p, state)) = phase_state(m) {
            let k = p.conductivity(state).ok_or_else(|| {
                task_error(format!(
                    "[task.thermal_conductivity] has no value for the medium {m}, and the \
                     catalogue has no conductivity for it yet: give it one"
                ))
            })?;
            out.values.push((
                m.clone(),
                k.value,
                format!("{}, {}", p.formula, k.source.reference),
            ));
            continue;
        }
        let Some(c) = catalogue::conductivity(m) else {
            let known: Vec<String> = catalogue::conductivities()
                .into_iter()
                .map(|c| c.formula.replace('₂', "2"))
                .collect();
            return Err(task_error(format!(
                "[task.thermal_conductivity] has no value for the medium {m} (the section holds \
                 {}), and the catalogue has no conductivity for it (it has {})",
                media.join(", "),
                known.join(", ")
            )));
        };
        let bulk = c.default_model();
        let films: Vec<f64> = stack
            .layers()
            .iter()
            .filter(|l| l.material.name() == m)
            .map(|l| l.thickness.to_um())
            .collect();
        let Some((film, &d)) = c.film().zip(films.first()) else {
            let k = bulk
                .conductivity(Some(kelvin), None)
                .map_err(|e| model_error(m, e))?;
            out.values.push((m.clone(), k, bulk.name.clone()));
            continue;
        };
        let ambiguous = |why: &str| {
            task_error(format!(
                "[task.thermal_conductivity] has no value for the medium {m}, whose conductivity \
                 the catalogue gives for a film by its thickness, and {why}: give {m} a value"
            ))
        };
        if films.iter().any(|t| (t - d).abs() > 1e-9) {
            return Err(ambiguous(
                "the stack has layers of it of different thicknesses",
            ));
        }
        let elsewhere = stack.cladding().name() == m
            || stack.layers().iter().any(|l| l.background.name() == m)
            || structure
                .bodies()
                .iter()
                .any(|(_, b)| matches!(b, crate::stack::Medium::Material(b) if b.name() == m));
        if elsewhere {
            return Err(ambiguous("it also fills the stack in bulk"));
        }
        if stack.substrate().name() == m {
            if !structure.bodies().is_empty() {
                return Err(ambiguous(
                    "it is the substrate too, which a structure with bodies can't draw apart",
                ));
            }
            let substrate = format!("{m}{SUBSTRATE}");
            if !task.thermal_conductivity.contains_key(&substrate) {
                let k = bulk
                    .conductivity(Some(kelvin), None)
                    .map_err(|e| model_error(&substrate, e))?;
                out.values.push((substrate, k, bulk.name.clone()));
            }
            out.substrate = Some(m.clone());
        }
        let k = film
            .conductivity(Some(kelvin), Some(d))
            .map_err(|e| model_error(m, e))?;
        out.values.push((
            m.clone(),
            k,
            format!("{}, {} nm film", film.name, (d * 1e4).round() / 10.0),
        ));
    }
    Ok(out)
}

fn sides(spec: &BoundarySpec) -> Result<Vec<(Side, Boundary)>> {
    let mut out = Vec::new();
    for (side, s) in [
        (Side::Left, &spec.left),
        (Side::Right, &spec.right),
        (Side::Bottom, &spec.bottom),
        (Side::Top, &spec.top),
    ] {
        if let Some(s) = s {
            out.push((side, s.boundary(side.name())?));
        }
    }
    Ok(out)
}

fn coefficients(
    section: &Section,
    table: &BTreeMap<String, CoefficientSpec>,
    what: &str,
) -> Result<Vec<(String, Tensor)>> {
    let mut out = Vec::new();
    for m in section.media() {
        let t = table.get(m).ok_or_else(|| {
            task_error(format!(
                "[task.{what}] has no value for the medium {m} (the section holds {})",
                section.media().join(", ")
            ))
        })?;
        out.push((m.clone(), checked(m, t, what)?));
    }
    Ok(out)
}

/// The coefficient `spec` gives the medium `m`, positive and finite.
fn checked(m: &str, spec: &CoefficientSpec, what: &str) -> Result<Tensor> {
    let t = spec.tensor();
    if !(t.xx.is_finite() && t.yy.is_finite() && t.xx > 0.0 && t.yy > 0.0) {
        return Err(task_error(format!(
            "[task.{what}] {m} must be positive and finite"
        )));
    }
    Ok(t)
}

fn setup(job: &Job) -> Result<Setup> {
    setup_scaled(job, 1.0)
}

/// The job's setup on a grid whose steps are `scale` times the task's (a grid study's coarser
/// grids).
fn setup_scaled(job: &Job, scale: f64) -> Result<Setup> {
    let task: StaticTask = super::params::task(job)?;
    super::check_window("y_um", task.y_um)?;
    super::check_window("z_um", task.z_um)?;
    super::check_step(task.step_nm)?;
    let fine = task.fine_nm.unwrap_or(task.step_nm);
    if !(fine.is_finite() && fine > 0.0 && fine <= task.step_nm) {
        return Err(task_error(format!(
            "fine_nm must be positive and at most step_nm, got {fine}"
        )));
    }
    let growth = task.growth.unwrap_or(1.2);
    let grading = Grading::new(task.step_nm * 1e-3 * scale, fine * 1e-3 * scale, growth)
        .map_err(|e| task_error(e.to_string()))?;
    // a crystal's tensor is taken by the electro-optic chain only
    let stack = match &task.electro_optic {
        None => named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?,
        Some(_) => {
            if task.physics != "electrostatic" {
                return Err(task_error(
                    "electro_optic is for physics = \"electrostatic\": the field of the \
                     conductors in a crystal",
                ));
            }
            if !(task.path.is_empty()
                && task.shape.is_empty()
                && task.component.is_empty()
                && task.body.is_empty())
            {
                return Err(task_error(
                    "electro_optic takes the shapes a modes job does: rect, circle and ring",
                ));
            }
            super::tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?
        }
    };
    let chain_stack = stack.clone();
    let structure = draw_paths(
        draw(stack, &task.rect, &task.circle, &task.ring)?,
        &task.path,
        &task.shape,
        &task.component,
    )
    .and_then(|s| super::bodies::place(s, &task.body))?;
    let cut = task.cut_x_um.unwrap_or(0.0);
    if !cut.is_finite() {
        return Err(task_error("cut_x_um must be a number"));
    }
    let cut_section =
        Section::cut(&structure, Cut::X(cut)).map_err(|e| task_error(e.to_string()))?;
    let regions = |section: &mut Section| -> Result<()> {
        for (k, r) in task.region.iter().enumerate() {
            let shape = r.shape("region", k)?;
            let medium = r
                .medium
                .as_deref()
                .ok_or_else(|| task_error(format!("[[task.region]] #{} needs a medium", k + 1)))?;
            section.draw(shape, medium);
        }
        Ok(())
    };
    let mut section = cut_section.clone();
    let mut features = Features::new();
    features.extend(section.features());
    regions(&mut section)?;
    features.extend(section.features());
    for (what, list) in [
        ("heater", &task.heater),
        ("sink", &task.sink),
        ("conductor", &task.conductor),
        ("probe", &task.probe),
        ("guide", &task.guide),
    ] {
        for (k, a) in list.iter().enumerate() {
            features.shape(&a.shape(what, k)?);
        }
    }
    let window = (task.y_um, task.z_um);
    let grid = Grid::fitted(task.y_um, task.z_um, &grading, &features)
        .map_err(|e| task_error(e.to_string()))?;
    if grid.cells() > MOST_CELLS {
        return Err(task_error(format!(
            "the grid would have {} cells, more than {MOST_CELLS}: take a larger fine_nm, \
             step_nm or growth, or a smaller window",
            grid.cells()
        )));
    }
    let (ny, nz) = (
        (((task.y_um[1] - task.y_um[0]) / (fine * 1e-3)).ceil() as usize).clamp(1, PIXELS),
        (((task.z_um[1] - task.z_um[0]) / (fine * 1e-3)).ceil() as usize).clamp(1, PIXELS),
    );
    let boundaries = sides(&task.boundary)?;
    let mut probes = Vec::new();
    for (k, p) in task.probe.iter().enumerate() {
        let name = p.name.clone().unwrap_or_else(|| format!("probe {}", k + 1));
        probes.push((name, p.shape("probe", k)?));
    }
    let mut setup = Setup {
        physics: task.physics.clone(),
        structure: structure.clone(),
        cut,
        grid: grid.clone(),
        window,
        pixels: (ny, nz),
        heat: None,
        electrostatic: None,
        current: None,
        probes,
        held: Vec::new(),
        transient: None,
        switching: Vec::new(),
        catalogued: Vec::new(),
        electro_optic: None,
        shifter: None,
    };
    // the heat problem's section, its media's conductivities (the job's, else the catalogue's)
    // and what the catalogue gave
    let heat = || -> Result<(Heat, Section, Vec<StaticValue>)> {
        let found = catalogued(&task, &structure, section.media())?;
        let heat_section = match &found.substrate {
            None => section.clone(),
            Some(m) => {
                // the substrate apart from the film of the same material, under the regions
                let mut s = cut_section.clone();
                s.layer(task.z_um[0].min(0.0) - 1.0, 0.0, &format!("{m}{SUBSTRATE}"))?;
                regions(&mut s)?;
                s
            }
        };
        let mut heat = Heat::new(heat_section.clone(), grid.clone());
        for m in heat_section.media() {
            let t = match task.thermal_conductivity.get(m) {
                Some(spec) => checked(m, spec, "thermal_conductivity")?,
                None => {
                    let (_, k, _) = found
                        .values
                        .iter()
                        .find(|(n, _, _)| n == m)
                        .expect("catalogued gives every medium the job doesn't");
                    Tensor::isotropic(*k)
                }
            };
            heat.conductivity(m, t);
        }
        let record = found
            .values
            .iter()
            .map(|(m, k, from)| {
                value(
                    format!("thermal conductivity of {m} ({from})"),
                    *k,
                    "W/(m*K)",
                )
            })
            .collect();
        for &(side, b) in &boundaries {
            heat.boundary(side, b);
        }
        for (k, h) in task.heater.iter().enumerate() {
            let p = h.need("heater", k, "power_w_per_m", h.power_w_per_m)?;
            heat.heater(h.shape("heater", k)?, p);
        }
        for (k, s) in task.sink.iter().enumerate() {
            let t = s.need("sink", k, "temperature", s.temperature)?;
            heat.sink(s.shape("sink", k)?, t);
        }
        Ok((heat, heat_section, record))
    };
    // something must hold the solution's level: a fixed or convective side, or a held rectangle
    let held_side = boundaries
        .iter()
        .any(|(_, b)| !matches!(b, Boundary::Flux(_)));
    let held = match task.physics.as_str() {
        "heat" => held_side || !task.sink.is_empty(),
        _ => held_side || !task.conductor.is_empty(),
    };
    if !held {
        return Err(task_error(
            "nothing holds the solution's level: give [task.boundary] a fixed or convective side, \
             or the task a [[task.sink]] or [[task.conductor]]",
        ));
    }
    if task.transient.is_some() && task.physics != "heat" {
        return Err(task_error(
            "[task.transient] is for physics = \"heat\": a transient of the temperature",
        ));
    }
    if !task.switching.is_empty() && task.transient.is_none() {
        return Err(task_error(
            "[[task.switching]] sets a pulse's temperatures against a film's: give the task a \
             [task.transient] with a pulse_ns",
        ));
    }
    match task.physics.as_str() {
        "heat" => {
            setup.held = (1..=task.sink.len()).map(|k| format!("sink {k}")).collect();
            let (mut h, heat_section, record) = heat()?;
            setup.catalogued = record;
            if let Some(spec) = &task.transient {
                let transient = spec.transient()?;
                if transient.pulse_s.is_some() && task.shifter.is_some() {
                    return Err(task_error(
                        "[task.transient] pulse_ns is for a pulse's temperatures, not a \
                         [task.shifter]'s phase",
                    ));
                }
                if !task.switching.is_empty() && transient.pulse_s.is_none() {
                    return Err(task_error(
                        "[[task.switching]] sets a pulse's temperatures against a film's: give \
                         [task.transient] a pulse_ns",
                    ));
                }
                for (k, s) in task.switching.iter().enumerate() {
                    let probe = setup
                        .probes
                        .iter()
                        .position(|(n, _)| *n == s.probe)
                        .ok_or_else(|| {
                            task_error(format!(
                                "[[task.switching]] #{} names the probe {:?}, which no \
                                 [[task.probe]] is",
                                k + 1,
                                s.probe
                            ))
                        })?;
                    let p =
                        crate::material::catalogue::phase_change(&s.material).ok_or_else(|| {
                            task_error(format!(
                                "[[task.switching]] #{} names {:?}: a phase-change material is \
                                 sb2se3, sb2s3 or gst",
                                k + 1,
                                s.material
                            ))
                        })?;
                    let (Some(tc), Some(tm)) = (&p.crystallization, &p.melting) else {
                        return Err(task_error(format!(
                            "[[task.switching]] #{}: the catalogue has no crystallization and \
                             melting temperatures for {} yet",
                            k + 1,
                            p.id
                        )));
                    };
                    setup
                        .switching
                        .push((probe, p.id.clone(), tc.value, tm.value));
                }
                setup.transient = Some(transient);
                for m in heat_section.media() {
                    // a substrate drawn apart takes its material's, unless given its own
                    // and a phase-change state the catalogue's ρc_p
                    let given = task
                        .heat_capacity
                        .get(m)
                        .or_else(|| {
                            m.strip_suffix(SUBSTRATE)
                                .and_then(|n| task.heat_capacity.get(n))
                        })
                        .copied();
                    let c = given
                        .or_else(|| {
                            let (p, _) = phase_state(m)?;
                            let c = p.heat_capacity()?;
                            setup.catalogued.push(value(
                                format!("heat capacity of {m} ({}, catalogue)", p.formula),
                                c,
                                "J/(m^3*K)",
                            ));
                            Some(c)
                        })
                        .ok_or_else(|| {
                            task_error(format!(
                                "[task.heat_capacity] has no value for the medium {m} (the \
                                 section holds {}), which a transient needs",
                                heat_section.media().join(", ")
                            ))
                        })?;
                    if !(c.is_finite() && c > 0.0) {
                        return Err(task_error(format!(
                            "[task.heat_capacity] {m} must be positive and finite"
                        )));
                    }
                    h.heat_capacity(m, c);
                }
            }
            setup.heat = Some(h);
        }
        "electrostatic" => {
            let mut e = Electrostatic::new(section.clone(), grid.clone());
            let mut permittivity = task.permittivity.clone();
            if let Some(spec) = &task.electro_optic {
                let (crystal, eps) = super::pockels::check(spec, &chain_stack)?;
                permittivity
                    .entry(crystal)
                    .or_insert(CoefficientSpec::Diagonal(eps));
                setup.electro_optic = Some(super::pockels::Chain {
                    spec: spec.clone(),
                    stack: chain_stack.clone(),
                    rect: task.rect.clone(),
                    circle: task.circle.clone(),
                    ring: task.ring.clone(),
                    cut,
                });
            }
            for (m, t) in coefficients(&section, &permittivity, "permittivity")? {
                e.permittivity(&m, t);
            }
            for &(side, b) in &boundaries {
                e.boundary(side, b);
            }
            for (k, c) in task.conductor.iter().enumerate() {
                let v = c.need("conductor", k, "potential_v", c.potential_v)?;
                e.conductor(Conductor::shape(c.shape("conductor", k)?, v));
            }
            setup.held = (1..=task.conductor.len())
                .map(|k| format!("conductor {k}"))
                .collect();
            setup.electrostatic = Some(e);
        }
        "current" => {
            let mut c = Current::new(section.clone(), grid.clone());
            for (m, t) in coefficients(
                &section,
                &task.electrical_conductivity,
                "electrical_conductivity",
            )? {
                c.conductivity(&m, t);
            }
            for &(side, b) in &boundaries {
                if matches!(b, Boundary::Fixed(_) | Boundary::Flux(_)) {
                    c.boundary(side, b);
                }
            }
            for (k, a) in task.conductor.iter().enumerate() {
                let v = a.need("conductor", k, "potential_v", a.potential_v)?;
                c.contact(Conductor::shape(a.shape("conductor", k)?, v));
            }
            if task.conductor.is_empty() {
                return Err(task_error(
                    "a current job needs [[task.conductor]] contacts at potentials",
                ));
            }
            setup.held = (1..=task.conductor.len())
                .map(|k| format!("contact {k}"))
                .collect();
            setup.current = Some(c);
            if !task.thermal_conductivity.is_empty() || task.shifter.is_some() {
                let (h, _, record) = heat()?;
                setup.heat = Some(h);
                setup.catalogued = record;
            }
        }
        other => {
            return Err(task_error(format!(
                "physics = \"{other}\": a static job's physics is \"heat\", \"electrostatic\" or \
                 \"current\""
            )));
        }
    }
    if let Some(spec) = &task.shifter {
        setup.shifter = Some(shifter(&task, spec, &section)?);
    } else if !task.guide.is_empty() {
        return Err(task_error(
            "[[task.guide]] draws a phase shifter's guides: give it a [task.shifter]",
        ));
    }
    Ok(setup)
}

/// The catalogue's index and dn/dT of medium `m` at `kelvin` and `wavelength`, from its entry's
/// default model, with the model's id; `None` if it has none, or none that takes a temperature.
fn catalogue_optics(
    m: &str,
    kelvin: f64,
    wavelength: crate::units::Wavelength,
) -> Option<(f64, f64, String)> {
    use crate::material::catalogue;
    let id: String = m
        .strip_suffix(SUBSTRATE)
        .unwrap_or(m)
        .chars()
        .map(|c| match c {
            '₀'..='₉' => char::from(b'0' + (c as u32 - '₀' as u32) as u8),
            c => c,
        })
        .collect::<String>()
        .to_lowercase();
    let entry = catalogue::entry(&id)?;
    let model = entry.default_model()?;
    let conditions = catalogue::Conditions {
        temperature: Some(kelvin),
        composition: None,
    };
    let n = model
        .materials(conditions)
        .ok()?
        .first()?
        .refractive_index(wavelength)
        .ok()?
        .re;
    let d = *model.thermo_optic(wavelength, conditions).ok()?.first()?;
    Some((n, d, model.id.clone()))
}

/// The phase shifter of `[task.shifter]` on `section`: each medium's optics from the task, else
/// the catalogue's at `temperature_k`; the guides' windows; the heaters as drives.
fn shifter(task: &StaticTask, spec: &ShifterSpec, section: &Section) -> Result<Shifter> {
    if task.physics == "electrostatic" {
        return Err(task_error(
            "[task.shifter] is for physics = \"heat\" or \"current\": a heater's phase",
        ));
    }
    let (w, step) = (spec.wavelength_um, spec.step_nm);
    let wavelength = crate::units::Wavelength::um(w)
        .map_err(|e| task_error(format!("[task.shifter] wavelength_um = {w}: {e}")))?;
    let mut optics = crate::thermal::Optics::new(wavelength, step * 1e-3)
        .map_err(|e| task_error(format!("[task.shifter] step_nm = {step}: {e}")))?;
    if task.guide.is_empty() {
        return Err(task_error(
            "[task.shifter] needs a guide: draw one with [[task.guide]] (y_um and z_um around the \
             waveguide, its mode's window)",
        ));
    }
    let kelvin = task.temperature_k.unwrap_or(300.0);
    let mut record = Vec::new();
    // a heat problem may draw the substrate apart: its optics are its material's
    let mut media: Vec<String> = section.media().to_vec();
    let apart: Vec<String> = media.iter().map(|m| format!("{m}{SUBSTRATE}")).collect();
    media.extend(apart);
    for m in &media {
        let given = (spec.index.get(m), spec.dn_dt.get(m));
        let found = match given {
            (Some(&n), Some(&d)) => Some((n, d, None)),
            _ => catalogue_optics(m, kelvin, wavelength).map(|(n, d, model)| {
                (
                    given.0.copied().unwrap_or(n),
                    given.1.copied().unwrap_or(d),
                    Some(model),
                )
            }),
        };
        let Some((n, d, model)) = found else {
            continue;
        };
        if !(n.is_finite() && n >= 1.0 && d.is_finite()) {
            return Err(task_error(format!(
                "[task.shifter] {m}'s index must be at least 1 and its dn/dT finite, got {n} and \
                 {d}"
            )));
        }
        optics.medium(m, n, d);
        if let Some(model) = model.filter(|_| !m.ends_with(SUBSTRATE)) {
            record.push(value(format!("index of {m} ({model})"), n, ""));
            record.push(value(format!("dn/dT of {m} ({model})"), d, "1/K"));
        }
    }
    let mut guides = Vec::new();
    for (k, g) in task.guide.iter().enumerate() {
        let name = g.name.clone().unwrap_or_else(|| format!("guide {}", k + 1));
        g.shape("guide", k)?;
        guides.push((name, (g.y_um, g.z_um)));
    }
    let mut heaters = Vec::new();
    for (k, h) in task.heater.iter().enumerate() {
        let name = h
            .name
            .clone()
            .unwrap_or_else(|| format!("heater {}", k + 1));
        let p = h.need("heater", k, "power_w_per_m", h.power_w_per_m)?;
        heaters.push((name, h.shape("heater", k)?, p));
    }
    if task.physics == "heat" && heaters.is_empty() {
        return Err(task_error(
            "[task.shifter] needs a [[task.heater]] to drive it (or physics = \"current\", its \
             Joule heat)",
        ));
    }
    if let Some(n) = spec.poles {
        if !(1..=12).contains(&n) {
            return Err(task_error(format!(
                "[task.shifter] poles = {n}: a thermal compact model has 1 to 12 poles"
            )));
        }
        if task.transient.is_none() {
            return Err(task_error(
                "[task.shifter] poles fits a thermal compact model to the drives' step \
                 responses: give a [task.transient] to take them",
            ));
        }
    }
    Ok(Shifter {
        optics,
        guides,
        heaters,
        record,
        poles: spec.poles,
    })
}

/// Checks a static job without solving it: its task, the stack and shapes, the grid's size,
/// every medium's coefficient and every boundary.
pub(super) fn check(job: &Job) -> Result<()> {
    setup(job).map(|_| ())
}

fn value(name: impl Into<String>, value: f64, unit: &str) -> StaticValue {
    StaticValue {
        name: name.into(),
        value,
        unit: unit.to_owned(),
    }
}

impl Setup {
    fn picture(&self, field: &Field) -> crate::raster::Raster {
        field.raster(self.window.0, self.window.1, self.pixels.0, self.pixels.1)
    }

    /// The structure in 3D for the studio's view: as long along x as the window is wide,
    /// about the cut, its media's permittivities at 1.55 µm (`None` if a material has no data
    /// there: the view is the optics', the problem doesn't need it).
    fn scene(&self) -> Option<Event> {
        let (y, z) = self.window;
        let half = 0.5 * (y[1] - y[0]);
        let wavelength = crate::units::Wavelength::um(1.55).ok()?;
        super::scene(
            &self.structure,
            [self.cut - half, self.cut + half],
            y,
            z,
            wavelength,
        )
        .ok()
    }
}

/// A static job's scene, for the job builder's preview.
pub(super) fn preview(job: &Job) -> Result<Event> {
    let setup = setup(job)?;
    setup.scene().ok_or_else(|| {
        task_error("the stack's materials have no permittivity at 1.55 um to draw it with")
    })
}

/// A field's numbers: the probes' means, its largest value and where, what each held rectangle
/// (`held`, by name) gives, what leaves through each side, and the imbalance.
fn summary(
    field: &Field,
    probes: &[(String, Shape)],
    held: &[String],
    (unit, flow): (&str, &str),
    out: &mut Vec<StaticValue>,
) -> Result<()> {
    for (name, shape) in probes {
        out.push(value(format!("mean over {name}"), field.mean(shape)?, unit));
    }
    let (max, at) = field.max();
    out.push(value("largest", max, unit));
    out.push(value("largest at y", at[0], "um"));
    out.push(value("largest at z", at[1], "um"));
    for (name, q) in held.iter().zip(field.held()) {
        out.push(value(format!("from {name}"), *q, flow));
    }
    for side in Side::ALL {
        out.push(value(
            format!("out through {}", side.name()),
            field.outflow(side),
            flow,
        ));
    }
    out.push(value("imbalance", field.imbalance(), ""));
    Ok(())
}

/// A heat job's transient: each probe's mean recorded as the steps go, the last field, and
/// each probe's 1/e and 10–90 % times and −3 dB frequency.
fn in_time(
    setup: &Setup,
    heat: &Heat,
    transient: &Transient,
    run: &mut Run,
    stop: &Stop,
    values: &mut Vec<StaticValue>,
) -> Result<()> {
    if let Some(pulse_s) = transient.pulse_s {
        return in_pulse(setup, heat, transient, pulse_s, run, stop, values);
    }
    let (from, to) = if transient.on { (0.0, 1.0) } else { (1.0, 0.0) };
    let shapes: Vec<Shape> = setup.probes.iter().map(|(_, s)| s.clone()).collect();
    let means =
        |field: &Field| -> Result<Vec<f64>> { shapes.iter().map(|s| field.mean(s)).collect() };
    let ends = means(&heat.steady_at(to)?)?;
    let stepping = &transient.stepping;
    let every = stepping.steps.div_ceil(RECORDED_STEPS).max(1);
    let step_event = |time_s: f64, means: &[f64]| Event::StaticStep {
        time_us: time_s * 1e6,
        means: setup
            .probes
            .iter()
            .zip(means)
            .map(|((name, _), m)| value(format!("mean over {name}"), *m, "K"))
            .collect(),
    };
    run.record(&step_event(0.0, &means(&heat.steady_at(from)?)?))?;
    let history = heat.transient_observed(stepping, from, &|_| to, &shapes, &mut |step| {
        if step.index % every == 0 || step.index == stepping.steps {
            run.record(&step_event(step.time_s, &step.means))?;
        }
        Ok(stop.reason().is_none())
    })?;
    let last = history.times_s.last().copied().unwrap_or(0.0);
    run.record(&Event::StaticField {
        label: format!("temperature at {} us", (last * 1e9).round() / 1e3),
        unit: "K".into(),
        axes: ["y".to_owned(), "z".to_owned()],
        raster: setup.picture(&history.last),
    })?;
    values.push(value("time step", stepping.step_s, "s"));
    values.push(value("steps", (history.times_s.len() - 1) as f64, ""));
    let e = 1.0 - (-1.0f64).exp();
    for (k, (name, shape)) in setup.probes.iter().enumerate() {
        let start = history.watched[k][0];
        if let Some(t) = history.crossing(k, start, ends[k], e) {
            values.push(value(format!("1/e time of {name}"), t, "s"));
        }
        if let (Some(a), Some(b)) = (
            history.crossing(k, start, ends[k], 0.1),
            history.crossing(k, start, ends[k], 0.9),
        ) {
            values.push(value(format!("10-90 % time of {name}"), b - a, "s"));
        }
        if stop.reason().is_none()
            && let Ok(f) = heat.cutoff(shape)
        {
            values.push(value(format!("-3 dB frequency of {name}"), f, "Hz"));
        }
    }
    Ok(())
}

/// A heat job's pulse ([`Heat::pulse`]): each probe's mean recorded as the steps go, the last
/// field, each probe's hottest and coldest cell at their highest, and what the pulse does to
/// each `[[task.switching]]` film.
fn in_pulse(
    setup: &Setup,
    heat: &Heat,
    transient: &Transient,
    pulse_s: f64,
    run: &mut Run,
    stop: &Stop,
    values: &mut Vec<StaticValue>,
) -> Result<()> {
    let shapes: Vec<Shape> = setup.probes.iter().map(|(_, s)| s.clone()).collect();
    let stepping = &transient.stepping;
    let every = stepping.steps.div_ceil(RECORDED_STEPS).max(1);
    let step_event = |time_s: f64, means: &[f64]| Event::StaticStep {
        time_us: time_s * 1e6,
        means: setup
            .probes
            .iter()
            .zip(means)
            .map(|((name, _), m)| value(format!("mean over {name}"), *m, "K"))
            .collect(),
    };
    let start = heat.steady_at(0.0)?;
    let first: Vec<f64> = shapes
        .iter()
        .map(|s| start.mean(s))
        .collect::<Result<_>>()?;
    run.record(&step_event(0.0, &first))?;
    let pulse = heat.pulse_observed(stepping, pulse_s, &shapes, &mut |step| {
        if step.index % every == 0 || step.index == stepping.steps {
            run.record(&step_event(step.time_s, &step.means))?;
        }
        Ok(stop.reason().is_none())
    })?;
    let last = pulse.history.times_s.last().copied().unwrap_or(0.0);
    run.record(&Event::StaticField {
        label: format!("temperature at {} us", (last * 1e9).round() / 1e3),
        unit: "K".into(),
        axes: ["y".to_owned(), "z".to_owned()],
        raster: setup.picture(&pulse.history.last),
    })?;
    values.push(value("time step", stepping.step_s, "s"));
    values.push(value("steps", (pulse.history.times_s.len() - 1) as f64, ""));
    values.push(value("pulse", pulse_s, "s"));
    for (k, (name, _)) in setup.probes.iter().enumerate() {
        values.push(value(
            format!("highest of {name}'s hottest cell"),
            pulse.hottest[k],
            "K",
        ));
        values.push(value(
            format!("highest of {name}'s coldest cell"),
            pulse.coldest[k],
            "K",
        ));
        values.push(value(
            format!("time of {name}'s coldest cell's highest"),
            pulse.coldest_at_s[k],
            "s",
        ));
    }
    for (k, material, tc, tm) in &setup.switching {
        let name = &setup.probes[*k].0;
        values.push(value(format!("T_c of {material}"), *tc, "K"));
        values.push(value(format!("T_m of {material}"), *tm, "K"));
        let switch = Switch::of(pulse.coldest[*k], pulse.hottest[*k], *tc, *tm);
        values.push(value(
            format!("{name} after the pulse: {}", switch.name()),
            1.0,
            "",
        ));
    }
    Ok(())
}

/// A `[task.shifter]`'s numbers: each guide's effective index and dn_eff/dT, each guide's
/// ∂n_eff/∂P′ by each drive (the heaters, or a current's Joule heat), each drive's P_π on its
/// own guide (the k-th heater's is the k-th guide) and its crosstalk on the others; with a
/// `[task.transient]`, each drive's phase's 1/e and 10–90 % times and −3 dB frequency on its own
/// guide, and P_π times the 1/e time.
fn phase_shifter(
    setup: &Setup,
    shifter: &Shifter,
    heat: Heat,
    flow: Option<&crate::electro::Flow>,
    stop: &Stop,
    values: &mut Vec<StaticValue>,
) -> Result<()> {
    use crate::thermal::{Drive, Guide, PhaseShifter};
    let drives: Vec<Drive> = match flow {
        Some(flow) => vec![Drive::joule("Joule heat", flow)],
        None => shifter
            .heaters
            .iter()
            .map(|(name, shape, p)| Drive::heater(name, shape.clone(), *p))
            .collect(),
    };
    let mut guides = Vec::new();
    for (name, window) in &shifter.guides {
        guides.push(
            Guide::new(name, heat.section(), *window, &shifter.optics).map_err(|e| {
                task_error(format!(
                    "[[task.guide]] {name}: {e} (give [task.shifter] index and dn_dt for a \
                     medium the catalogue has no temperature model of)"
                ))
            })?,
        );
    }
    let ps = PhaseShifter::new(heat, drives, guides)?;
    let e = ps.efficiency()?;
    values.extend(shifter.record.iter().cloned());
    for g in ps.guides() {
        values.push(value(
            format!("n_eff of {}", g.name()),
            g.effective_index(),
            "",
        ));
        values.push(value(
            format!("dn_eff/dT of {}", g.name()),
            g.thermo_optic(),
            "1/K",
        ));
    }
    let (guides, drives) = (ps.guides(), ps.drives());
    for (i, g) in guides.iter().enumerate() {
        for (j, d) in drives.iter().enumerate() {
            values.push(value(
                format!("dn_eff/dP of {} by {}", g.name(), d.name()),
                e.slopes[i][j],
                "m/W",
            ));
        }
    }
    for (j, d) in drives.iter().enumerate().take(guides.len()) {
        values.push(value(
            format!("P_pi of {} on {}", d.name(), guides[j].name()),
            e.p_pi(j, j),
            "W",
        ));
        for (i, g) in guides.iter().enumerate() {
            if i != j {
                values.push(value(
                    format!("crosstalk of {} on {}", d.name(), g.name()),
                    e.crosstalk(i, j, j),
                    "",
                ));
            }
        }
    }
    if let Some(transient) = &setup.transient {
        for (j, d) in drives.iter().enumerate().take(guides.len()) {
            if stop.reason().is_some() {
                break;
            }
            let g = guides[j].name();
            let t = ps.times(j, j, &transient.stepping, transient.on)?;
            values.push(value(
                format!("phase 1/e time of {g} by {}", d.name()),
                t.one_over_e_s,
                "s",
            ));
            values.push(value(
                format!("phase 10-90 % time of {g} by {}", d.name()),
                t.ten_ninety_s,
                "s",
            ));
            values.push(value(
                format!("P_pi * 1/e time of {}", d.name()),
                e.p_pi(j, j) * t.one_over_e_s,
                "W*s",
            ));
            if let Ok(f) = ps.cutoff(j, j) {
                values.push(value(
                    format!("phase -3 dB frequency of {g} by {}", d.name()),
                    f,
                    "Hz",
                ));
            }
        }
        if let Some(n) = shifter.poles
            && stop.reason().is_none()
        {
            thermal_model(&ps, &transient.stepping, n, values)?;
        }
    }
    Ok(())
}

/// A `[task.shifter]`'s thermal compact model of `poles` poles, fitted to every drive's step
/// response of every guide's phase ([`crate::thermal::RcModel::fits`]): the error of each fit
/// from 1 pole to `poles`, and the last fit's time constants and residues (rad/W).
fn thermal_model(
    ps: &crate::thermal::PhaseShifter,
    stepping: &Stepping,
    poles: usize,
    values: &mut Vec<StaticValue>,
) -> Result<()> {
    use crate::thermal::{RcFit, RcModel};
    let r = ps.step_responses(stepping)?;
    let fits = RcModel::fits(&r.times_s, &r.steps, Some(&r.steady), &RcFit::new(poles))?;
    for (k, f) in fits.iter().enumerate() {
        values.push(value(
            format!("thermal model error, {} poles", k + 1),
            f.error,
            "",
        ));
    }
    let model = &fits.last().expect("a fit per pole").model;
    for (k, (t, r)) in model
        .time_constants_s()
        .iter()
        .zip(model.residues())
        .enumerate()
    {
        values.push(value(
            format!("thermal model time constant {}", k + 1),
            *t,
            "s",
        ));
        for (i, g) in ps.guides().iter().enumerate() {
            for (j, d) in ps.drives().iter().enumerate() {
                values.push(value(
                    format!(
                        "thermal model residue {} of {} by {}",
                        k + 1,
                        g.name(),
                        d.name()
                    ),
                    r[i][j],
                    "rad/W",
                ));
            }
        }
    }
    Ok(())
}

/// Runs a static job into `run`.
pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let setup = setup(job)?;
    if let Some(scene) = setup.scene() {
        run.record(&scene)?;
    }
    let (smallest_x, largest_x) = setup.grid.x().steps();
    let (smallest_y, largest_y) = setup.grid.y().steps();
    let mut values = Vec::new();
    let axes = ["y".to_owned(), "z".to_owned()];
    // the fields solved, for [task.export]
    let export = super::export_of(job)?;
    let mut solved: Vec<(&str, Field)> = Vec::new();
    if let Some(e) = &setup.electrostatic {
        let field = e.solve()?;
        if export.npz() {
            solved.push(("potential", field.clone()));
        }
        run.record(&Event::StaticField {
            label: "potential".into(),
            unit: "V".into(),
            axes: axes.clone(),
            raster: setup.picture(&field),
        })?;
        summary(
            &field,
            &setup.probes,
            &setup.held,
            ("V", "C/m"),
            &mut values,
        )?;
        if !e.conductors().is_empty() && e.conductors().len() <= 8 && stop.reason().is_none() {
            let c = e.capacitance()?;
            for (i, row) in c.iter().enumerate() {
                for (j, v) in row.iter().enumerate() {
                    values.push(value(format!("C[{}][{}]", i + 1, j + 1), *v, "F/m"));
                }
            }
        }
        if let Some(chain) = &setup.electro_optic {
            let coarser = |k: u32| -> Result<Electrostatic> {
                setup_scaled(job, f64::from(1u32 << k))?
                    .electrostatic
                    .ok_or_else(|| task_error("an electrostatic job has an electrostatic problem"))
            };
            match super::pockels::run(chain, e, coarser, run, stop)? {
                Some(more) => values.extend(more),
                None => return Ok(()),
            }
        }
    }
    let mut joule = None;
    if let Some(c) = &setup.current {
        let flow = c.solve()?;
        if export.npz() {
            solved.push(("potential", flow.field().clone()));
        }
        run.record(&Event::StaticField {
            label: "potential".into(),
            unit: "V".into(),
            axes: axes.clone(),
            raster: setup.picture(flow.field()),
        })?;
        summary(
            flow.field(),
            &setup.probes,
            &setup.held,
            ("V", "A/m"),
            &mut values,
        )?;
        values.push(value("Joule power", flow.power(), "W/m"));
        joule = Some(flow);
    }
    if let Some(h) = &setup.heat
        && stop.reason().is_none()
    {
        let mut h = h.clone();
        let plain = h.clone();
        if let Some(flow) = &joule {
            h.joule(flow)?;
        }
        let field = h.solve()?;
        if export.npz() {
            solved.push(("temperature", field.clone()));
        }
        run.record(&Event::StaticField {
            label: "temperature".into(),
            unit: "K".into(),
            axes,
            raster: setup.picture(&field),
        })?;
        // a current's contacts hold the potential, not the temperature
        let held: &[String] = if joule.is_some() { &[] } else { &setup.held };
        let mut heat_values = Vec::new();
        summary(&field, &setup.probes, held, ("K", "W/m"), &mut heat_values)?;
        if joule.is_some() {
            for v in &mut heat_values {
                v.name = format!("heat: {}", v.name);
            }
        }
        values.extend(heat_values);
        values.extend(setup.catalogued.iter().cloned());
        if let Some(transient) = &setup.transient {
            in_time(&setup, &h, transient, run, stop, &mut values)?;
        }
        if let Some(shifter) = &setup.shifter
            && stop.reason().is_none()
        {
            phase_shifter(&setup, shifter, plain, joule.as_ref(), stop, &mut values)?;
        }
    }
    if export.npz() && !solved.is_empty() && !run.dir().as_os_str().is_empty() {
        let fields: Vec<(&str, &Field)> = solved.iter().map(|(n, f)| (*n, f)).collect();
        export_fields(
            &run.dir().join("static.npz"),
            &crate::export::Meta::of(job, run),
            &export,
            &setup.grid,
            &fields,
        )?;
    }
    run.record(&Event::StaticResult {
        physics: setup.physics.clone(),
        cells: [setup.grid.x().cells(), setup.grid.y().cells()],
        step_um: [smallest_x.min(smallest_y), largest_x.max(largest_y)],
        values,
    })
}

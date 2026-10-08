//! The `"fdtd"` job: a structure stepped in time by FDTD ([`crate::fdtd`]), in 2D (the layer's
//! plane, each point's permittivity its slab mode's effective index squared, as an `"fdfd"`
//! job's) or in 3D (the stack itself), with sources, monitors and a stopping rule, the field
//! recorded as it propagates.
//!
//! Times are c·t in µm (µm/c): light in vacuum crosses 1 µm in 1 µm/c, 3.336 fs. Every source
//! shares one waveform, a Gaussian pulse on a carrier at `wavelength_um` wide enough to cover
//! the `[task.spectrum]`; the materials' permittivities are taken at the carrier, so the run is
//! non-dispersive. The monitors' spectra are referred to the sources' incident power at each
//! wavelength: for mode sources the power the source's guide carries forward in its own mode
//! there, measured on a plane just after the source; for plane waves the power through the
//! total-field box's face; for beams the paraxial beam's power. Dipoles have no incident power,
//! and a run with one is referred to the pulse's spectrum instead.

use std::time::Instant;

use num_complex::Complex64 as c64;
use serde::{Deserialize, Serialize};

use super::{
    CircleSpec, Event, RectSpec, RingSpec, check_cells, check_materials, check_step, check_window,
    draw, named_stack, scene, task_error,
};
use crate::fdfd::{
    Axis, Boundaries3d, Direction, Edges, Formulation, Grid3d, IterativeSolver3d, PortMode3d,
};
use crate::fdtd::{
    BeamPolarization, Boundaries, Cpml, Dipole, Field, FluxPlane, GaussianBeam, PlaneWave,
    Simulation, Waveform, harmonic_inversion,
};
use crate::geometry::{Point, Shape};
use crate::material::Material;
use crate::run::{Job, Run, Stop};
use crate::stack::Structure;
use crate::units::{Frequency, Length, Wavelength};
use crate::{Error, Result};

/// The most cells an FDTD job's grid may have in all: about 2 GB of fields and coefficients.
const MOST_CELLS: f64 = 2e7;

/// The longest side, in pixels, of a recorded frame: a grid with more cells along a side is
/// averaged over blocks of cells.
const FRAME_SIDE: usize = 240;

/// The frames a run records at most; after every [`FRAMES_BEFORE_DOUBLING`] the interval
/// between them doubles, so a long run's record stays bounded.
const MOST_FRAMES: usize = 500;

/// See [`MOST_FRAMES`].
const FRAMES_BEFORE_DOUBLING: usize = 100;

/// The share of a run's time its frames may take: the interval doubles when they take more.
const FRAME_SHARE: f64 = 0.1;

/// The progress is recorded at least this often, seconds of the clock, frames or not.
const PROGRESS_SECONDS: f64 = 1.0;

/// The spectra are recorded at most this often, seconds of the clock (and no more often than
/// ten times what they take), and at the end.
const SPECTRA_SECONDS: f64 = 2.0;

/// The samples of a resonance monitor's field harmonic inversion takes at most.
const RESONANCE_SAMPLES: usize = 4000;

/// The modes solved on a port's plane, among which the job's polarization is picked.
const PORT_MODES: usize = 4;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct BoundarySpec {
    x: Option<String>,
    y: Option<String>,
    z: Option<String>,
    /// The Bloch wavenumbers along x, y and z, rad/µm, for the periodic sides.
    bloch: Option<[f64; 3]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpectrumSpec {
    from_um: f64,
    to_um: f64,
    points: usize,
}

/// A source or a monitor, as the job gives it: its type and the fields that type reads.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ItemSpec {
    #[serde(rename = "type")]
    kind: String,
    name: Option<String>,
    normal: Option<String>,
    at_um: Option<f64>,
    direction: Option<String>,
    x_um: Option<[f64; 2]>,
    y_um: Option<[f64; 2]>,
    z_um: Option<[f64; 2]>,
    position_um: Option<Vec<f64>>,
    component: Option<String>,
    polarization: Option<String>,
    center_um: Option<Vec<f64>>,
    waist_um: Option<f64>,
    focus_um: Option<f64>,
    angle_deg: Option<f64>,
    wavelengths_um: Option<Vec<f64>>,
}

impl ItemSpec {
    /// The fields given, by name.
    fn given(&self) -> Vec<&'static str> {
        let mut g = Vec::new();
        let mut put = |name: &'static str, given: bool| {
            if given {
                g.push(name);
            }
        };
        put("name", self.name.is_some());
        put("normal", self.normal.is_some());
        put("at_um", self.at_um.is_some());
        put("direction", self.direction.is_some());
        put("x_um", self.x_um.is_some());
        put("y_um", self.y_um.is_some());
        put("z_um", self.z_um.is_some());
        put("position_um", self.position_um.is_some());
        put("component", self.component.is_some());
        put("polarization", self.polarization.is_some());
        put("center_um", self.center_um.is_some());
        put("waist_um", self.waist_um.is_some());
        put("focus_um", self.focus_um.is_some());
        put("angle_deg", self.angle_deg.is_some());
        put("wavelengths_um", self.wavelengths_um.is_some());
        g
    }

    /// Refuses the fields a type doesn't read, rather than ignoring them.
    fn only(&self, what: &str, allowed: &[&str]) -> Result<()> {
        if let Some(extra) = self.given().into_iter().find(|g| !allowed.contains(g)) {
            return Err(task_error(format!(
                "{what}: a {} doesn't take {extra} (it takes {})",
                self.kind,
                allowed.join(", ")
            )));
        }
        Ok(())
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct StopSpec {
    /// `"decay"` (the default) or `"time"`.
    until: Option<String>,
    /// For `"decay"`: the share of its peak |field|² the monitors' points must stay below over a
    /// whole check.
    fraction: Option<f64>,
    /// For `"time"`: how long, µm/c.
    time_um: Option<f64>,
    /// For `"decay"`: the longest the run may go on, µm/c.
    limit_um: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ViewSpec {
    /// The field the frames show: a component, `"|E|^2"` or `"|H|^2"`.
    field: Option<String>,
    /// 3D: the height of the plane seen from above, µm.
    z_um: Option<f64>,
    /// A frame every this much time, µm/c.
    every_um: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FdtdTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    /// 2 (the default) or 3.
    dimensions: Option<u8>,
    /// 2D: `"te"` (the default) or `"tm"`.
    polarization: Option<String>,
    x_um: [f64; 2],
    y_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    step_nm: f64,
    courant: Option<f64>,
    pml_cells: Option<usize>,
    boundaries: Option<BoundarySpec>,
    spectrum: Option<SpectrumSpec>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    #[serde(default)]
    source: Vec<ItemSpec>,
    #[serde(default)]
    monitor: Vec<ItemSpec>,
    stop: Option<StopSpec>,
    view: Option<ViewSpec>,
}

/// A spectrum's series: its values at the spectrum's wavelengths.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectrumSeries {
    /// What it is, e.g. `"towards +x"`.
    pub label: String,
    /// Its values, one per wavelength.
    pub values: Vec<f64>,
}

/// A resonance an FDTD run found by harmonic inversion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoundResonance {
    /// The vacuum wavelength 1/f, µm.
    pub wavelength_um: f64,
    /// Re ω / 2π, c/µm.
    pub frequency: f64,
    /// The quality factor ω/(2γ).
    pub q: f64,
    /// γ, per µm/c: the amplitude goes as e^(−γt).
    pub decay: f64,
    /// |amplitude| at the signal's first sample.
    pub amplitude: f64,
    /// Harmonic inversion's own error for the term: near round-off for a term the signal holds.
    pub error: f64,
}

/// The field the frames show.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Component(Field, Axis),
    /// |E|² or |H̃|².
    Squared(Field),
}

impl Shown {
    fn parse(name: &str) -> Option<Shown> {
        Some(match name {
            "|E|^2" => Shown::Squared(Field::E),
            "|H|^2" => Shown::Squared(Field::H),
            _ => {
                let (field, component) = component(name)?;
                Shown::Component(field, component)
            }
        })
    }

    fn name(self) -> String {
        match self {
            Shown::Component(f, c) => component_name(f, c),
            Shown::Squared(Field::E) => "|E|^2".into(),
            Shown::Squared(Field::H) => "|H|^2".into(),
        }
    }
}

/// A component's name, e.g. `"Ez"`, as a field and an axis.
fn component(name: &str) -> Option<(Field, Axis)> {
    let mut c = name.chars();
    let field = match c.next()? {
        'E' => Field::E,
        'H' => Field::H,
        _ => return None,
    };
    let axis = axis_of(&c.collect::<String>())?;
    Some((field, axis))
}

fn component_name(field: Field, axis: Axis) -> String {
    format!(
        "{}{}",
        if field == Field::E { "E" } else { "H" },
        ["x", "y", "z"][axis.index()]
    )
}

fn axis_of(name: &str) -> Option<Axis> {
    match name {
        "x" => Some(Axis::X),
        "y" => Some(Axis::Y),
        "z" => Some(Axis::Z),
        _ => None,
    }
}

fn axis_name(axis: Axis) -> &'static str {
    ["x", "y", "z"][axis.index()]
}

/// A source as the run adds it.
enum SourcePlan {
    Mode {
        axis: Axis,
        plane: usize,
        direction: Direction,
        window: [std::ops::Range<usize>; 2],
        te: bool,
    },
    Dipole(Dipole),
    PlaneWave(PlaneWave),
    Beam(GaussianBeam),
}

/// A monitor as the run adds it.
enum MonitorPlan {
    Mode {
        name: String,
        axis: Axis,
        plane: usize,
        window: [std::ops::Range<usize>; 2],
        te: bool,
    },
    Flux {
        name: String,
        plane: FluxPlane,
    },
    /// The faces of a box, each with its sign: outwards is +.
    FluxBox {
        name: String,
        faces: Vec<(FluxPlane, f64)>,
    },
    Field {
        wavelengths_um: Vec<f64>,
    },
    Resonance {
        name: String,
        field: Field,
        component: Axis,
        at: (usize, usize, usize),
        position_um: Vec<f64>,
    },
}

/// How the run ends.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Until {
    /// When |field|² at the monitors' points has stayed below `fraction` of its peak over a
    /// whole check, or at `limit` µm/c.
    Decay { fraction: f64, limit: f64 },
    /// At `time` µm/c.
    Time(f64),
}

/// An `"fdtd"` job, checked and placed on its grid, ready to run.
struct Setup {
    task: FdtdTask,
    structure: Structure,
    /// The layer's bottom and top.
    layer: (Length, Length),
    three: bool,
    /// 2D: the slab's TE mode (H along z); TM otherwise. 3D: unused.
    te: bool,
    grid: Grid3d,
    boundaries: Boundaries,
    courant: f64,
    /// The spectrum's wavelengths, µm.
    wavelengths: Vec<f64>,
    /// The pulse's carrier and bandwidth, c/µm.
    carrier: f64,
    bandwidth: f64,
    sources: Vec<SourcePlan>,
    monitors: Vec<MonitorPlan>,
    until: Until,
    shown: Shown,
    /// The plane the frames show (3D: its node along z).
    view_k: usize,
    view_z_um: f64,
    every: Option<f64>,
    /// The cells the pictures leave out along x and y at each end: the CPMLs and two more.
    margins: [usize; 2],
}

/// The task, its parameters and the shapes' expressions resolved as the other kinds' are.
fn parse(job: &Job) -> Result<FdtdTask> {
    super::params::task(job)
}

impl Setup {
    /// The job checked as far as it can be without solving: the windows, the grid, the
    /// boundaries, the spectrum, each source's and monitor's fields and place, the stopping rule
    /// and the view.
    fn new(job: &Job) -> Result<Setup> {
        let task = parse(job)?;
        check_window("x_um", task.x_um)?;
        check_window("y_um", task.y_um)?;
        check_step(task.step_nm)?;
        check_cells("x_um", task.x_um, task.step_nm)?;
        check_cells("y_um", task.y_um, task.step_nm)?;
        let three = match task.dimensions.unwrap_or(2) {
            2 => false,
            3 => true,
            d => return Err(task_error(format!("dimensions must be 2 or 3, got {d}"))),
        };
        let te = match (three, task.polarization.as_deref()) {
            (false, None | Some("te")) => true,
            (false, Some("tm")) => false,
            (false, Some(other)) => {
                return Err(task_error(format!(
                    "unknown polarization \"{other}\": te or tm"
                )));
            }
            (true, Some(_)) => {
                return Err(task_error(
                    "polarization is for a 2D job: a 3D job's sources and monitors each say \
                     theirs",
                ));
            }
            (true, None) => true,
        };
        if !three && task.z_um.is_some() {
            return Err(task_error(
                "z_um is for a 3D job (dimensions = 3): a 2D job is the layer's plane",
            ));
        }
        let structure = draw(
            named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?,
            &task.rect,
            &task.circle,
            &task.ring,
        )?;
        let (_, bottom, top) = structure
            .stack()
            .layer(&task.layer)
            .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
        let z_window = match task.z_um {
            Some(z) => {
                check_window("z_um", z)?;
                check_cells("z_um", z, task.step_nm)?;
                z
            }
            None => [bottom.to_um() - 1.0, top.to_um() + 1.0],
        };
        let courant = task.courant.unwrap_or(0.9);
        if !(courant > 0.0 && courant <= 1.0) {
            return Err(task_error(format!(
                "courant must be in (0, 1], got {courant}"
            )));
        }
        let grid = grid_of(&task, three, (bottom, top), z_window);
        if grid.cells() as f64 > MOST_CELLS {
            return Err(task_error(format!(
                "the grid is {} x {} x {} = {} cells, more than {MOST_CELLS:e}: take a larger \
                 step_nm or a smaller window",
                grid.nx,
                grid.ny,
                grid.nz,
                grid.cells()
            )));
        }
        let pml = task.pml_cells.unwrap_or(20);
        let boundaries = boundaries_of(&task, three, pml, &grid)?;
        // the leapfrog's step, as the simulation will take it
        let dt = time_step(&grid, &boundaries, courant);
        let wavelengths = match &task.spectrum {
            None => vec![task.wavelength_um],
            Some(sp) => {
                if sp.points == 0
                    || !(sp.from_um.is_finite() && sp.to_um.is_finite())
                    || sp.from_um <= 0.0
                    || sp.to_um <= 0.0
                {
                    return Err(task_error(
                        "the spectrum needs positive from_um and to_um and at least 1 point",
                    ));
                }
                if sp.points > 1000 {
                    return Err(task_error(format!(
                        "the spectrum's points must be at most 1000, got {}",
                        sp.points
                    )));
                }
                if sp.points == 1 {
                    vec![sp.from_um]
                } else {
                    (0..sp.points)
                        .map(|k| {
                            sp.from_um + (sp.to_um - sp.from_um) * k as f64 / (sp.points - 1) as f64
                        })
                        .collect()
                }
            }
        };
        Wavelength::um(task.wavelength_um)?;
        let carrier = 1.0 / task.wavelength_um;
        let reach = wavelengths
            .iter()
            .map(|w| (1.0 / w - carrier).abs())
            .fold(0.0, f64::max);
        let bandwidth = (2.0 * reach).max(0.05 * carrier);
        let nyquist = 0.5 / dt;
        for f in wavelengths
            .iter()
            .map(|w| 1.0 / w)
            .chain([carrier + bandwidth])
        {
            if f >= 0.8 * nyquist {
                return Err(task_error(format!(
                    "the pulse and the spectrum reach {f:.4} c/um, too near the time step's \
                     Nyquist frequency {nyquist:.4} c/um: take a smaller step_nm or a narrower \
                     spectrum"
                )));
            }
        }
        let until = match &task.stop {
            None => Until::Decay {
                fraction: 1e-8,
                limit: 2000.0,
            },
            Some(stop) => stop_of(stop)?,
        };
        let view = task.view.as_ref();
        let shown_name = view.and_then(|v| v.field.clone()).unwrap_or_else(|| {
            match (three, te) {
                (true, _) => "Ey",
                (false, true) => "Hz",
                (false, false) => "Ez",
            }
            .into()
        });
        let shown = Shown::parse(&shown_name).ok_or_else(|| {
            task_error(format!(
                "unknown view field \"{shown_name}\": Ex, Ey, Ez, Hx, Hy, Hz, |E|^2 or |H|^2"
            ))
        })?;
        if let Shown::Component(f, c) = shown {
            in_polarization(three, te, f, c, "the view's field")?;
        }
        let (view_k, view_z_um) = if three {
            let z = view
                .and_then(|v| v.z_um)
                .unwrap_or(0.5 * (bottom.to_um() + top.to_um()));
            let k = ((z - grid.z0) / grid.dz).floor();
            if !(z.is_finite() && k >= 0.0 && (k as usize) < grid.nz) {
                return Err(task_error(format!(
                    "the view's z_um, {z}, is outside the window's z, {} to {} um",
                    z_window[0], z_window[1]
                )));
            }
            (k as usize, z)
        } else {
            if view.is_some_and(|v| v.z_um.is_some()) {
                return Err(task_error(
                    "the view's z_um is for a 3D job: a 2D job is seen in its plane",
                ));
            }
            (0, top.to_um())
        };
        let every = view.and_then(|v| v.every_um);
        if let Some(e) = every
            && !(e.is_finite() && e > 0.0)
        {
            return Err(task_error(format!(
                "the view's every_um must be positive, got {e}"
            )));
        }
        let margins = [Axis::X, Axis::Y].map(|a| match edges(&boundaries, a) {
            Edges::Pml { low, .. } if low > 0 => low + 2,
            _ => 0,
        });
        let mut setup = Setup {
            task,
            structure,
            layer: (bottom, top),
            three,
            te,
            grid,
            boundaries,
            courant,
            wavelengths,
            carrier,
            bandwidth,
            sources: Vec::new(),
            monitors: Vec::new(),
            until,
            shown,
            view_k,
            view_z_um,
            every,
            margins,
        };
        if setup.task.source.is_empty() {
            return Err(task_error("an fdtd job needs at least one [[task.source]]"));
        }
        let complex = setup.complex();
        for k in 0..setup.task.source.len() {
            let plan = setup.source(k)?;
            if complex && !matches!(plan, SourcePlan::Dipole(_)) {
                return Err(task_error(format!(
                    "source {}: with a Bloch wavenumber the fields are complex, and only dipoles \
                     can drive them",
                    k + 1
                )));
            }
            setup.sources.push(plan);
        }
        for k in 0..setup.task.monitor.len() {
            let plan = setup.monitor(k)?;
            if complex && matches!(plan, MonitorPlan::Mode { .. }) {
                return Err(task_error(format!(
                    "monitor {}: a guide's modes need real fields, without a Bloch wavenumber",
                    k + 1
                )));
            }
            setup.monitors.push(plan);
        }
        let mut names: Vec<&str> = Vec::new();
        for m in &setup.monitors {
            if let Some(n) = m.name() {
                if names.contains(&n) {
                    return Err(task_error(format!(
                        "two monitors are named \"{n}\": each needs its own name"
                    )));
                }
                names.push(n);
            }
        }
        let mut all = setup.wavelengths.clone();
        all.push(setup.task.wavelength_um);
        check_materials(setup.structure.stack(), &all)?;
        Ok(setup)
    }

    /// Whether a Bloch wavenumber makes the fields complex.
    fn complex(&self) -> bool {
        [self.boundaries.x, self.boundaries.y, self.boundaries.z]
            .iter()
            .any(|e| matches!(e, Edges::Bloch { k } if *k != 0.0))
    }

    fn origin(&self, axis: Axis) -> f64 {
        [self.grid.x0, self.grid.y0, self.grid.z0][axis.index()]
    }

    /// The node nearest `v` along `axis`.
    fn node(&self, axis: Axis, v: f64, what: &str) -> Result<usize> {
        let m = ((v - self.origin(axis)) / self.grid.step(axis)).round();
        if !(v.is_finite() && m >= 0.0 && m <= self.grid.n(axis) as f64) {
            return Err(task_error(format!(
                "{what}: {} = {v} um is outside the window",
                axis_name(axis)
            )));
        }
        Ok(m as usize)
    }

    /// The CPML's cells at each end of `axis`.
    fn cpml(&self, axis: Axis) -> (usize, usize) {
        match edges(&self.boundaries, axis) {
            Edges::Pml { low, high } => (low, high),
            Edges::Bloch { .. } => (0, 0),
        }
    }

    /// A plane of nodes normal to `axis` at `v`, at least three cells clear of the CPMLs and of
    /// the window's ends (a mode's plane, the next and the source's or monitor's own around it).
    fn clear_plane(&self, axis: Axis, v: f64, what: &str) -> Result<usize> {
        if matches!(edges(&self.boundaries, axis), Edges::Bloch { .. }) {
            return Err(task_error(format!(
                "{what}: a plane normal to {} needs ends along it, not periodic ones",
                axis_name(axis)
            )));
        }
        let p = self.node(axis, v, what)?;
        let (low, high) = self.cpml(axis);
        let n = self.grid.n(axis);
        if p < low + 6 || p + 9 + high > n {
            return Err(task_error(format!(
                "{what}: its plane at {} = {v} um must be at least 6 cells clear of the CPML or \
                 the window's end before it ({low} cells of CPML) and 9 of those after it \
                 ({high})",
                axis_name(axis)
            )));
        }
        Ok(p)
    }

    /// The cells from `w[0]` to `w[1]` along `axis` (the whole axis when `None`), at least 3.
    fn cells(&self, axis: Axis, w: Option<[f64; 2]>, what: &str) -> Result<std::ops::Range<usize>> {
        let n = self.grid.n(axis);
        let Some(w) = w else {
            return Ok(0..n);
        };
        check_window(&format!("{what}'s {}_um", axis_name(axis)), w)?;
        let at = |v: f64| ((v - self.origin(axis)) / self.grid.step(axis)).round();
        let (a, b) = (at(w[0]), at(w[1]));
        if a < 0.0 || b > n as f64 {
            return Err(task_error(format!(
                "{what}: its {}_um, {} to {}, must lie inside the window",
                axis_name(axis),
                w[0],
                w[1]
            )));
        }
        let (a, b) = (a as usize, b as usize);
        if b < a + 3 {
            return Err(task_error(format!(
                "{what}: its {}_um, {} to {}, covers fewer than 3 cells",
                axis_name(axis),
                w[0],
                w[1]
            )));
        }
        Ok(a..b)
    }

    /// The axis of a plane normal to x or y (or z, in 3D, when `z` is allowed).
    fn normal(&self, spec: &ItemSpec, what: &str, z: bool) -> Result<Axis> {
        let name = spec
            .normal
            .as_deref()
            .ok_or_else(|| task_error(format!("{what}: needs a normal, \"x\" or \"y\"")))?;
        match axis_of(name) {
            Some(Axis::Z) if z && self.three => Ok(Axis::Z),
            Some(a @ (Axis::X | Axis::Y)) => Ok(a),
            _ => Err(task_error(format!(
                "{what}: unknown normal \"{name}\": x or y{}",
                if z && self.three { " or z" } else { "" }
            ))),
        }
    }

    /// The window of a plane normal to `axis` along its two other axes, `axis.others()`.
    fn window(
        &self,
        spec: &ItemSpec,
        axis: Axis,
        what: &str,
    ) -> Result<[std::ops::Range<usize>; 2]> {
        let given = |a: Axis| match a {
            Axis::X => spec.x_um,
            Axis::Y => spec.y_um,
            Axis::Z => spec.z_um,
        };
        if given(axis).is_some() {
            return Err(task_error(format!(
                "{what}: its plane is normal to {}: give at_um, and windows along the other axes",
                axis_name(axis)
            )));
        }
        if !self.three && spec.z_um.is_some() {
            return Err(task_error(format!("{what}: z_um is for a 3D job")));
        }
        let (b, c) = axis.others();
        Ok([
            self.cells(b, given(b), what)?,
            self.cells(c, given(c), what)?,
        ])
    }

    /// A 2D job's or a 3D source's polarization, as TE (true) or TM.
    fn polarization(&self, spec: &ItemSpec, what: &str) -> Result<bool> {
        match (self.three, spec.polarization.as_deref()) {
            (false, None) => Ok(self.te),
            (false, Some(_)) => Err(task_error(format!(
                "{what}: a 2D job's polarization is the task's; leave it out here"
            ))),
            (true, None | Some("te")) => Ok(true),
            (true, Some("tm")) => Ok(false),
            (true, Some(other)) => Err(task_error(format!(
                "{what}: unknown polarization \"{other}\": te or tm"
            ))),
        }
    }

    /// A position (x, y), and z in 3D (the layer's middle by default in 2D, where z doesn't
    /// count), inside the window.
    fn position(&self, p: Option<&Vec<f64>>, what: &str) -> Result<[f64; 3]> {
        let p = p.ok_or_else(|| task_error(format!("{what}: needs position_um")))?;
        let (x, y, z) = match (self.three, p.as_slice()) {
            (false, [x, y]) => (*x, *y, self.grid.z0 + 0.5 * self.grid.dz),
            (true, [x, y, z]) => (*x, *y, *z),
            _ => {
                return Err(task_error(format!(
                    "{what}: position_um must be [x, y]{}",
                    if self.three { ", z" } else { "" }
                )));
            }
        };
        for (axis, v) in Axis::ALL.into_iter().zip([x, y, z]) {
            let lo = self.origin(axis);
            let hi = lo + self.grid.n(axis) as f64 * self.grid.step(axis);
            if !(v.is_finite() && v >= lo && v <= hi) {
                return Err(task_error(format!(
                    "{what}: its {} = {v} um is outside the window",
                    axis_name(axis)
                )));
            }
        }
        Ok([x, y, z])
    }

    fn source(&self, k: usize) -> Result<SourcePlan> {
        let spec = &self.task.source[k];
        let what = format!("source {} ({})", k + 1, spec.kind);
        let what = what.as_str();
        match spec.kind.as_str() {
            "mode" => {
                spec.only(
                    what,
                    &[
                        "normal",
                        "at_um",
                        "direction",
                        "x_um",
                        "y_um",
                        "z_um",
                        "polarization",
                    ],
                )?;
                let axis = self.normal(spec, what, false)?;
                let at = spec
                    .at_um
                    .ok_or_else(|| task_error(format!("{what}: needs at_um, its plane")))?;
                let plane = self.clear_plane(axis, at, what)?;
                let direction = direction_of(spec.direction.as_deref(), what)?;
                let window = self.window(spec, axis, what)?;
                let te = self.polarization(spec, what)?;
                Ok(SourcePlan::Mode {
                    axis,
                    plane,
                    direction,
                    window,
                    te,
                })
            }
            "dipole" => {
                spec.only(what, &["position_um", "component"])?;
                let position = self.position(spec.position_um.as_ref(), what)?;
                let name = spec
                    .component
                    .as_deref()
                    .ok_or_else(|| task_error(format!("{what}: needs a component, e.g. \"Ez\"")))?;
                let (field, component) = component(name).ok_or_else(|| {
                    task_error(format!(
                        "{what}: unknown component \"{name}\": Ex, Ey, Ez, Hx, Hy or Hz"
                    ))
                })?;
                in_polarization(self.three, self.te, field, component, what)?;
                Ok(SourcePlan::Dipole(Dipole {
                    field,
                    component,
                    position,
                    amplitude: c64::new(1.0, 0.0),
                    waveform: self.waveform(),
                }))
            }
            "plane_wave" => {
                spec.only(what, &["x_um", "y_um", "z_um", "direction", "polarization"])?;
                let (axis, sign) = signed_axis(spec.direction.as_deref(), what, self.three)?;
                let mut low = [0usize; 3];
                let mut high = [0usize; 3];
                for (a, w) in Axis::ALL.into_iter().zip([spec.x_um, spec.y_um, spec.z_um]) {
                    if a == Axis::Z && !self.three {
                        if w.is_some() {
                            return Err(task_error(format!("{what}: z_um is for a 3D job")));
                        }
                        low[2] = 0;
                        high[2] = self.grid.nz;
                        continue;
                    }
                    let w = w.ok_or_else(|| {
                        task_error(format!(
                            "{what}: needs its box's {}_um, the total field inside",
                            axis_name(a)
                        ))
                    })?;
                    check_window(&format!("{what}'s {}_um", axis_name(a)), w)?;
                    let (lo, hi) = (self.node(a, w[0], what)?, self.node(a, w[1], what)?);
                    let (pl, ph) = self.cpml(a);
                    let n = self.grid.n(a);
                    if lo < pl + 2 || hi + ph + 2 > n || hi <= lo {
                        return Err(task_error(format!(
                            "{what}: its box's {}_um, {} to {}, must be clear of the CPMLs \
                             ({pl} and {ph} cells) by two cells",
                            axis_name(a),
                            w[0],
                            w[1]
                        )));
                    }
                    low[a.index()] = lo;
                    high[a.index()] = hi;
                }
                let polarization = match (self.three, spec.polarization.as_deref()) {
                    (false, None) => {
                        if self.te {
                            // in the plane, across the direction
                            if axis == Axis::X {
                                [0.0, 1.0, 0.0]
                            } else {
                                [1.0, 0.0, 0.0]
                            }
                        } else {
                            [0.0, 0.0, 1.0]
                        }
                    }
                    (false, Some(_)) => {
                        return Err(task_error(format!(
                            "{what}: a 2D job's polarization is the task's; leave it out here"
                        )));
                    }
                    (true, p) => {
                        let p = p.ok_or_else(|| {
                            task_error(format!(
                                "{what}: needs a polarization, E's direction: x, y or z"
                            ))
                        })?;
                        match axis_of(p) {
                            Some(e) if e != axis => {
                                let mut v = [0.0; 3];
                                v[e.index()] = 1.0;
                                v
                            }
                            _ => {
                                return Err(task_error(format!(
                                    "{what}: its polarization, E's direction, must be x, y or z \
                                     across its direction, got \"{p}\""
                                )));
                            }
                        }
                    }
                };
                let mut direction = [0i64; 3];
                direction[axis.index()] = sign;
                Ok(SourcePlan::PlaneWave(PlaneWave {
                    low: (low[0], low[1], low[2]),
                    high: (high[0], high[1], high[2]),
                    direction: (direction[0], direction[1], direction[2]),
                    polarization,
                    // the medium's, read off the grid when the source is added
                    eps: 1.0,
                    waveform: self.waveform(),
                }))
            }
            "beam" => {
                spec.only(
                    what,
                    &[
                        "normal",
                        "at_um",
                        "direction",
                        "center_um",
                        "waist_um",
                        "focus_um",
                        "angle_deg",
                        "polarization",
                    ],
                )?;
                let axis = self.normal(spec, what, false)?;
                let at = spec
                    .at_um
                    .ok_or_else(|| task_error(format!("{what}: needs at_um, its plane")))?;
                let plane = self.clear_plane(axis, at, what)?;
                let direction = direction_of(spec.direction.as_deref(), what)?;
                let (b, c) = axis.others();
                // the in-plane axis across the beam, and z
                let across = if axis == Axis::X { Axis::Y } else { Axis::X };
                let mid = self.grid.z0 + 0.5 * self.grid.nz as f64 * self.grid.dz;
                let (a_value, z_value) = match (self.three, spec.center_um.as_deref()) {
                    (false, Some([a])) => (*a, mid),
                    (true, Some([a, z])) => (*a, *z),
                    _ => {
                        return Err(task_error(format!(
                            "{what}: center_um must be where the beam crosses its plane: [{}]{}",
                            axis_name(across),
                            if self.three { " and z" } else { "" }
                        )));
                    }
                };
                let value = |t: Axis| if t == Axis::Z { z_value } else { a_value };
                let waist = spec
                    .waist_um
                    .ok_or_else(|| task_error(format!("{what}: needs waist_um")))?;
                let angle = spec.angle_deg.unwrap_or(0.0);
                if !(angle.is_finite() && angle.abs() < 90.0) {
                    return Err(task_error(format!(
                        "{what}: angle_deg must be between -90 and 90, got {angle}"
                    )));
                }
                let polarization = match (self.three, spec.polarization.as_deref()) {
                    (false, None) => {
                        if self.te {
                            BeamPolarization::Parallel
                        } else {
                            BeamPolarization::Perpendicular
                        }
                    }
                    (true, None | Some("s")) => BeamPolarization::Perpendicular,
                    (true, Some("p")) => BeamPolarization::Parallel,
                    (false, Some(_)) => {
                        return Err(task_error(format!(
                            "{what}: a 2D job's polarization is the task's; leave it out here"
                        )));
                    }
                    (true, Some(other)) => {
                        return Err(task_error(format!(
                            "{what}: unknown polarization \"{other}\": s (E across the plane \
                             of incidence) or p"
                        )));
                    }
                };
                Ok(SourcePlan::Beam(GaussianBeam {
                    axis,
                    plane,
                    direction,
                    centre: (value(b), value(c)),
                    waist,
                    focus: spec.focus_um.unwrap_or(0.0),
                    tilt: across,
                    angle: angle.to_radians(),
                    polarization,
                    // the medium's, read off the grid when the source is added
                    eps: 1.0,
                }))
            }
            other => Err(task_error(format!(
                "source {}: unknown type \"{other}\": mode, dipole, plane_wave or beam",
                k + 1
            ))),
        }
    }

    fn monitor(&self, k: usize) -> Result<MonitorPlan> {
        let spec = &self.task.monitor[k];
        let what = format!("monitor {} ({})", k + 1, spec.kind);
        let what = what.as_str();
        let name = spec
            .name
            .clone()
            .unwrap_or_else(|| format!("{} {}", spec.kind.replace('_', " "), k + 1));
        match spec.kind.as_str() {
            "mode" => {
                spec.only(
                    what,
                    &[
                        "name",
                        "normal",
                        "at_um",
                        "x_um",
                        "y_um",
                        "z_um",
                        "polarization",
                    ],
                )?;
                let axis = self.normal(spec, what, false)?;
                let at = spec
                    .at_um
                    .ok_or_else(|| task_error(format!("{what}: needs at_um, its plane")))?;
                let plane = self.clear_plane(axis, at, what)?;
                let window = self.window(spec, axis, what)?;
                let te = self.polarization(spec, what)?;
                Ok(MonitorPlan::Mode {
                    name,
                    axis,
                    plane,
                    window,
                    te,
                })
            }
            "flux" => {
                spec.only(what, &["name", "normal", "at_um", "x_um", "y_um", "z_um"])?;
                let axis = self.normal(spec, what, true)?;
                let at = spec
                    .at_um
                    .ok_or_else(|| task_error(format!("{what}: needs at_um, its plane")))?;
                let plane = self.half_plane(axis, at, what)?;
                let (b, c) = axis.others();
                let given = |a: Axis| match a {
                    Axis::X => spec.x_um,
                    Axis::Y => spec.y_um,
                    Axis::Z => spec.z_um,
                };
                if given(axis).is_some() || (!self.three && spec.z_um.is_some()) {
                    return Err(task_error(format!(
                        "{what}: give at_um, and windows along the plane's other axes{}",
                        if self.three {
                            ""
                        } else {
                            " (x or y: z is for a 3D job)"
                        }
                    )));
                }
                let window = [b, c].map(|a| given(a).map(|w| (a, w)));
                let mut plane = FluxPlane::new(axis, plane);
                for (slot, w) in window.into_iter().enumerate() {
                    if let Some((a, w)) = w {
                        check_window(&format!("{what}'s {}_um", axis_name(a)), w)?;
                        let lo = self.half_plane(a, w[0], what)?;
                        let hi = self.half_plane(a, w[1], what)?;
                        if hi <= lo {
                            return Err(task_error(format!(
                                "{what}: its {}_um covers less than a cell",
                                axis_name(a)
                            )));
                        }
                        plane.window[slot] = Some((lo, hi));
                    }
                }
                Ok(MonitorPlan::Flux { name, plane })
            }
            "flux_box" => {
                spec.only(what, &["name", "x_um", "y_um", "z_um"])?;
                let mut planes = [(0usize, 0usize); 3];
                for (a, w) in Axis::ALL.into_iter().zip([spec.x_um, spec.y_um, spec.z_um]) {
                    if a == Axis::Z && !self.three {
                        if w.is_some() {
                            return Err(task_error(format!("{what}: z_um is for a 3D job")));
                        }
                        continue;
                    }
                    let w = w.ok_or_else(|| {
                        task_error(format!("{what}: needs its box's {}_um", axis_name(a)))
                    })?;
                    check_window(&format!("{what}'s {}_um", axis_name(a)), w)?;
                    let (lo, hi) = (
                        self.half_plane(a, w[0], what)?,
                        self.half_plane(a, w[1], what)?,
                    );
                    if hi <= lo {
                        return Err(task_error(format!(
                            "{what}: its {}_um covers less than a cell",
                            axis_name(a)
                        )));
                    }
                    planes[a.index()] = (lo, hi);
                }
                let axes: &[Axis] = if self.three {
                    &Axis::ALL
                } else {
                    &[Axis::X, Axis::Y]
                };
                let mut faces = Vec::new();
                for &axis in axes {
                    let (lo, hi) = planes[axis.index()];
                    let (b, c) = axis.others();
                    for (node, sign) in [(lo, -1.0), (hi, 1.0)] {
                        let mut face = FluxPlane::new(axis, node);
                        for (slot, t) in [b, c].into_iter().enumerate() {
                            if axes.contains(&t) {
                                face.window[slot] = Some(planes[t.index()]);
                            }
                        }
                        faces.push((face, sign));
                    }
                }
                Ok(MonitorPlan::FluxBox { name, faces })
            }
            "field" => {
                spec.only(what, &["name", "wavelengths_um"])?;
                let wavelengths_um = spec
                    .wavelengths_um
                    .clone()
                    .unwrap_or_else(|| vec![self.task.wavelength_um]);
                if wavelengths_um.is_empty() || wavelengths_um.len() > 20 {
                    return Err(task_error(format!(
                        "{what}: wavelengths_um must hold 1 to 20 wavelengths"
                    )));
                }
                let lowest = 1.0 / self.wavelengths.iter().copied().fold(f64::MIN, f64::max);
                let highest = 1.0 / self.wavelengths.iter().copied().fold(f64::MAX, f64::min);
                let (lo, hi) = (
                    lowest.min(self.carrier - self.bandwidth),
                    highest.max(self.carrier + self.bandwidth),
                );
                for &w in &wavelengths_um {
                    Wavelength::um(w)?;
                    if !(lo..=hi).contains(&(1.0 / w)) {
                        return Err(task_error(format!(
                            "{what}: {w} um is outside the pulse's band, {:.4} to {:.4} um",
                            1.0 / hi,
                            1.0 / lo
                        )));
                    }
                }
                Ok(MonitorPlan::Field { wavelengths_um })
            }
            "resonance" => {
                spec.only(what, &["name", "position_um", "component"])?;
                let position = self.position(spec.position_um.as_ref(), what)?;
                let name_c = spec.component.clone().unwrap_or_else(|| {
                    match (self.three, self.te) {
                        (true, _) => "Ey",
                        (false, true) => "Hz",
                        (false, false) => "Ez",
                    }
                    .into()
                });
                let (field, comp) = component(&name_c).ok_or_else(|| {
                    task_error(format!(
                        "{what}: unknown component \"{name_c}\": Ex, Ey, Ez, Hx, Hy or Hz"
                    ))
                })?;
                in_polarization(self.three, self.te, field, comp, what)?;
                let at = self.cell_of(position);
                Ok(MonitorPlan::Resonance {
                    name,
                    field,
                    component: comp,
                    at,
                    position_um: if self.three {
                        position.to_vec()
                    } else {
                        position[..2].to_vec()
                    },
                })
            }
            other => Err(task_error(format!(
                "monitor {}: unknown type \"{other}\": mode, flux, flux_box, field or resonance",
                k + 1
            ))),
        }
    }

    /// The half-plane nearest `v` along `axis`: the node before it, so that the plane is
    /// halfway between it and the next; inside the grid and clear of the CPMLs.
    fn half_plane(&self, axis: Axis, v: f64, what: &str) -> Result<usize> {
        let m = ((v - self.origin(axis)) / self.grid.step(axis) - 0.5).round();
        let (low, high) = self.cpml(axis);
        let n = self.grid.n(axis);
        if !(v.is_finite()
            && m >= low as f64
            && m + 1.0 + (high as f64) <= n as f64
            && m + 1.0 < n as f64)
        {
            return Err(task_error(format!(
                "{what}: {} = {v} um is outside the window or inside its CPMLs ({low} and \
                 {high} cells)",
                axis_name(axis)
            )));
        }
        Ok(m as usize)
    }

    /// The cell holding a point.
    fn cell_of(&self, p: [f64; 3]) -> (usize, usize, usize) {
        let at = |a: Axis| {
            let m = ((p[a.index()] - self.origin(a)) / self.grid.step(a)).floor();
            (m.max(0.0) as usize).min(self.grid.n(a) - 1)
        };
        (at(Axis::X), at(Axis::Y), at(Axis::Z))
    }

    fn waveform(&self) -> Waveform {
        // (the carrier and the bandwidth are positive: checked above)
        Waveform::pulse(
            Frequency::natural(self.carrier).expect("a positive carrier"),
            self.bandwidth,
        )
        .expect("a positive bandwidth")
    }

    /// The permittivity at (x, y, z): in 2D the effective index squared of the slab under the
    /// point, as an `"fdfd"` job's; in 3D the stack's own, both at the carrier.
    fn permittivity(&self) -> Result<Permittivity> {
        let lam = Wavelength::um(self.task.wavelength_um)?;
        let stack = self.structure.stack().clone();
        let layers = stack.layers().len();
        if layers > 16 {
            return Err(task_error("an fdtd job's stack may have at most 16 layers"));
        }
        let shapes: Vec<Vec<Shape>> = stack
            .layers()
            .iter()
            .map(|l| self.structure.shapes(&l.name).to_vec())
            .collect();
        let tops: Vec<f64> = stack
            .layers()
            .iter()
            .scan(0.0, |z, l| {
                *z += l.thickness.to_um();
                Some(*z)
            })
            .collect();
        if self.three {
            let real = |m: &Material| -> Result<f64> {
                let e = m.permittivity(lam)?;
                if e.im.abs() > 1e-9 * e.re.abs() {
                    return Err(task_error(format!(
                        "{} is lossy at {} um (eps = {e}): an fdtd job's materials must be \
                         lossless",
                        m.name(),
                        lam.to_um()
                    )));
                }
                Ok(e.re)
            };
            // each layer's two materials, and the substrate and cladding
            let mut table = Vec::with_capacity(layers);
            for l in stack.layers() {
                table.push((real(&l.material)?, real(&l.background)?));
            }
            let (below, above) = (real(stack.substrate())?, real(stack.cladding())?);
            Ok(Box::new(move |x: f64, y: f64, z: f64| {
                if z < 0.0 {
                    return below;
                }
                match tops.iter().position(|&t| z < t) {
                    Some(i) => {
                        let p = Point::um(x, y);
                        if shapes[i].iter().any(|shape| shape.contains(p)) {
                            table[i].0
                        } else {
                            table[i].1
                        }
                    }
                    None => above,
                }
            }))
        } else {
            let kind = if self.te {
                crate::mode::Polarization::Te
            } else {
                crate::mode::Polarization::Tm
            };
            // the slab's index for each set of layers with a shape over a point
            let mut table = Vec::with_capacity(1 << layers);
            for mask in 0..1usize << layers {
                let n = super::fdfd::slab_index(
                    |z| column(&stack, &tops, mask, z.to_um()),
                    self.layer,
                    kind,
                    lam,
                )?;
                table.push(n * n);
            }
            Ok(Box::new(move |x: f64, y: f64, _: f64| {
                let p = Point::um(x, y);
                let mask = shapes
                    .iter()
                    .enumerate()
                    .filter(|(_, sh)| sh.iter().any(|shape| shape.contains(p)))
                    .fold(0, |m, (i, _)| m | (1 << i));
                table[mask]
            }))
        }
    }

    /// The scene: the part of the window the pictures show, and its height.
    fn scene(&self) -> Result<Event> {
        let [x, y] = self.inner();
        let z = if self.three {
            [
                self.grid.z0,
                self.grid.z0 + self.grid.nz as f64 * self.grid.dz,
            ]
        } else {
            [self.layer.0.to_um() - 1.0, self.layer.1.to_um() + 1.0]
        };
        scene(
            &self.structure,
            x,
            y,
            z,
            Wavelength::um(self.task.wavelength_um)?,
        )
    }

    /// The part of the window the pictures show along x and y, µm.
    fn inner(&self) -> [[f64; 2]; 2] {
        [Axis::X, Axis::Y].map(|a| {
            let m = self.margins[a.index()];
            let lo = self.origin(a) + m as f64 * self.grid.step(a);
            let hi = self.origin(a) + (self.grid.n(a) - m) as f64 * self.grid.step(a);
            [lo, hi]
        })
    }

    /// The components whose |field|² is summed at each of [`Setup::probe_points`].
    fn probe_components(&self) -> Vec<(Field, Axis)> {
        match (self.three, self.te) {
            (true, _) => Axis::ALL.iter().map(|&a| (Field::E, a)).collect(),
            (false, true) => vec![(Field::H, Axis::Z)],
            (false, false) => vec![(Field::E, Axis::Z)],
        }
    }

    /// Where the decay is judged: each monitor's middle (each resonance's point), or the
    /// middle of the window without monitors.
    fn probe_points(&self) -> Vec<(usize, usize, usize)> {
        let k_mid = self.grid.nz / 2;
        let mut points = Vec::new();
        let mid = |r: &std::ops::Range<usize>| (r.start + r.end) / 2;
        let at = |axis: Axis, plane: usize, b: usize, c: usize| {
            let mut p = [0usize; 3];
            let (ab, ac) = axis.others();
            p[axis.index()] = plane;
            p[ab.index()] = b;
            p[ac.index()] = c;
            (p[0], p[1], p[2].min(self.grid.nz - 1))
        };
        for m in &self.monitors {
            match m {
                MonitorPlan::Mode {
                    axis,
                    plane,
                    window,
                    ..
                } => points.push(at(*axis, *plane, mid(&window[0]), mid(&window[1]))),
                MonitorPlan::Flux { plane, .. } => {
                    let (b, c) = plane.axis.others();
                    let centre = |slot: usize, t: Axis| match plane.window[slot] {
                        Some((lo, hi)) => (lo + hi) / 2,
                        None => {
                            if t == Axis::Z {
                                k_mid
                            } else {
                                self.grid.n(t) / 2
                            }
                        }
                    };
                    points.push(at(plane.axis, plane.plane, centre(0, b), centre(1, c)));
                }
                MonitorPlan::FluxBox { faces, .. } => {
                    for (face, _) in faces {
                        let (b, c) = face.axis.others();
                        let centre = |slot: usize, t: Axis| match face.window[slot] {
                            Some((lo, hi)) => (lo + hi) / 2,
                            None => {
                                if t == Axis::Z {
                                    k_mid
                                } else {
                                    self.grid.n(t) / 2
                                }
                            }
                        };
                        points.push(at(face.axis, face.plane, centre(0, b), centre(1, c)));
                    }
                }
                MonitorPlan::Resonance { at, .. } => points.push(*at),
                MonitorPlan::Field { .. } => {}
            }
        }
        if points.is_empty() {
            points.push((self.grid.nx / 2, self.grid.ny / 2, k_mid));
        }
        points
    }
}

/// A job's permittivity, ε(x, y, z), real.
type Permittivity = Box<dyn Fn(f64, f64, f64) -> f64 + Sync>;

/// The material at height `z` (µm) of a column of `stack` where the layers in `mask` have a
/// shape: the substrate below the stack, the cladding above it, and within a layer its
/// material or its background, as [`Structure::material_at`] says.
fn column<'a>(
    stack: &'a crate::stack::LayerStack,
    tops: &[f64],
    mask: usize,
    z: f64,
) -> &'a Material {
    if z < 0.0 {
        return stack.substrate();
    }
    match tops.iter().position(|&t| z < t) {
        Some(i) => {
            let layer = &stack.layers()[i];
            if mask & (1 << i) != 0 {
                &layer.material
            } else {
                &layer.background
            }
        }
        None => stack.cladding(),
    }
}

fn edges(b: &Boundaries, axis: Axis) -> Edges {
    [b.x, b.y, b.z][axis.index()]
}

/// Whether a component belongs to a 2D job's polarization: TE has E in the plane and H along
/// z, TM the other three.
fn in_polarization(three: bool, te: bool, field: Field, c: Axis, what: &str) -> Result<()> {
    if three {
        return Ok(());
    }
    let in_plane = c != Axis::Z;
    let te_component = (field == Field::E) == in_plane;
    if te_component != te {
        return Err(task_error(format!(
            "{what}: {} isn't one of a 2D {} job's components ({})",
            component_name(field, c),
            if te { "TE" } else { "TM" },
            if te { "Ex, Ey, Hz" } else { "Ez, Hx, Hy" }
        )));
    }
    Ok(())
}

fn direction_of(d: Option<&str>, what: &str) -> Result<Direction> {
    match d.unwrap_or("+") {
        "+" => Ok(Direction::Forward),
        "-" => Ok(Direction::Backward),
        other => Err(task_error(format!(
            "{what}: unknown direction \"{other}\": \"+\" (towards +normal) or \"-\""
        ))),
    }
}

fn signed_axis(d: Option<&str>, what: &str, three: bool) -> Result<(Axis, i64)> {
    let d = d.ok_or_else(|| task_error(format!("{what}: needs a direction, e.g. \"+x\"")))?;
    let (sign, rest) = match d.split_at_checked(1) {
        Some(("+", r)) => (1, r),
        Some(("-", r)) => (-1, r),
        _ => (0, d),
    };
    match axis_of(rest) {
        Some(a) if sign != 0 && (three || a != Axis::Z) => Ok((a, sign)),
        _ => Err(task_error(format!(
            "{what}: unknown direction \"{d}\": +x, -x, +y or -y{}",
            if three { ", +z or -z" } else { "" }
        ))),
    }
}

fn stop_of(stop: &StopSpec) -> Result<Until> {
    let positive = |name: &str, v: f64| {
        if v.is_finite() && v > 0.0 {
            Ok(v)
        } else {
            Err(task_error(format!("{name} must be positive, got {v}")))
        }
    };
    match stop.until.as_deref().unwrap_or("decay") {
        "decay" => {
            if stop.time_um.is_some() {
                return Err(task_error(
                    "time_um is for until = \"time\"; a run until the fields decay takes \
                     fraction and limit_um",
                ));
            }
            let fraction = stop.fraction.unwrap_or(1e-8);
            if !(fraction > 0.0 && fraction < 1.0) {
                return Err(task_error(format!(
                    "fraction must be between 0 and 1, got {fraction}"
                )));
            }
            Ok(Until::Decay {
                fraction,
                limit: positive("limit_um", stop.limit_um.unwrap_or(2000.0))?,
            })
        }
        "time" => {
            if stop.fraction.is_some() || stop.limit_um.is_some() {
                return Err(task_error(
                    "fraction and limit_um are for until = \"decay\"; a run for a time takes \
                     time_um",
                ));
            }
            let t = stop
                .time_um
                .ok_or_else(|| task_error("until = \"time\" needs time_um"))?;
            Ok(Until::Time(positive("time_um", t)?))
        }
        other => Err(task_error(format!(
            "unknown until \"{other}\": decay or time"
        ))),
    }
}

/// The grid of cells of about `step_nm` filling the window; one cell thick along z in 2D,
/// at the layer's middle.
fn grid_of(task: &FdtdTask, three: bool, layer: (Length, Length), z: [f64; 2]) -> Grid3d {
    let h = task.step_nm / 1000.0;
    let cells = |w: [f64; 2]| ((w[1] - w[0]) / h).round().max(1.0) as usize;
    let (nx, ny) = (cells(task.x_um), cells(task.y_um));
    let (nz, dz, z0) = if three {
        let nz = cells(z);
        (nz, (z[1] - z[0]) / nz as f64, z[0])
    } else {
        (1, h, 0.5 * (layer.0.to_um() + layer.1.to_um()) - 0.5 * h)
    };
    Grid3d {
        nx,
        ny,
        nz,
        dx: (task.x_um[1] - task.x_um[0]) / nx as f64,
        dy: (task.y_um[1] - task.y_um[0]) / ny as f64,
        dz,
        x0: task.x_um[0],
        y0: task.y_um[0],
        z0,
    }
}

fn boundaries_of(task: &FdtdTask, three: bool, pml: usize, grid: &Grid3d) -> Result<Boundaries> {
    let spec = task.boundaries.as_ref();
    let default = BoundarySpec::default();
    let spec = spec.unwrap_or(&default);
    if !three && spec.z.is_some() {
        return Err(task_error(
            "boundaries.z is for a 3D job: a 2D job is periodic along z",
        ));
    }
    let bloch = spec.bloch.unwrap_or([0.0; 3]);
    let mut out = [Edges::Pml { low: 0, high: 0 }; 3];
    for (axis, given) in Axis::ALL.into_iter().zip([&spec.x, &spec.y, &spec.z]) {
        let k = bloch[axis.index()];
        if !k.is_finite() {
            return Err(task_error(format!(
                "the Bloch wavenumber along {} must be a number, got {k}",
                axis_name(axis)
            )));
        }
        if axis == Axis::Z && !three {
            if k != 0.0 {
                return Err(task_error(
                    "a 2D job has no Bloch wavenumber along z: it is uniform there",
                ));
            }
            out[2] = Edges::Bloch { k: 0.0 };
            continue;
        }
        let kind = given.as_deref().unwrap_or("cpml");
        out[axis.index()] = match kind {
            "cpml" => {
                if 2 * pml + 4 > grid.n(axis) {
                    return Err(task_error(format!(
                        "CPMLs of {pml} cells at each end leave nothing of the {} cells along {}",
                        grid.n(axis),
                        axis_name(axis)
                    )));
                }
                Edges::Pml {
                    low: pml,
                    high: pml,
                }
            }
            "wall" => Edges::Pml { low: 0, high: 0 },
            "periodic" => Edges::Bloch { k },
            other => {
                return Err(task_error(format!(
                    "unknown boundary \"{other}\" along {}: cpml, wall or periodic",
                    axis_name(axis)
                )));
            }
        };
        if k != 0.0 && kind != "periodic" {
            return Err(task_error(format!(
                "a Bloch wavenumber along {} needs periodic sides there",
                axis_name(axis)
            )));
        }
    }
    Ok(Boundaries {
        x: out[0],
        y: out[1],
        z: out[2],
        cpml: Cpml::default(),
    })
}

/// The leapfrog's Δt at Courant number `courant`: C/√(Σ 1/Δ²) over the axes the field varies
/// along (the library's own rule, before it builds anything).
fn time_step(grid: &Grid3d, b: &Boundaries, courant: f64) -> f64 {
    let sum: f64 = Axis::ALL
        .into_iter()
        .filter(|&a| !(grid.n(a) == 1 && matches!(edges(b, a), Edges::Bloch { .. })))
        .map(|a| 1.0 / (grid.step(a) * grid.step(a)))
        .sum();
    courant / sum.sqrt()
}

impl MonitorPlan {
    fn name(&self) -> Option<&str> {
        match self {
            MonitorPlan::Mode { name, .. }
            | MonitorPlan::Flux { name, .. }
            | MonitorPlan::FluxBox { name, .. }
            | MonitorPlan::Resonance { name, .. } => Some(name),
            MonitorPlan::Field { .. } => None,
        }
    }
}

/// [`super::check`] for an `"fdtd"` job.
pub(super) fn check(job: &Job) -> Result<()> {
    Setup::new(job).map(|_| ())
}

/// [`super::preview`] for an `"fdtd"` job.
pub(super) fn preview(job: &Job) -> Result<Event> {
    Setup::new(job)?.scene()
}

/// The fundamental mode of the job's polarization on a plane, solved by FDFD on a few planes
/// cut out of the grid around it at the wavelength whose leapfrog frequency is `frequency`'s
/// (as a mode source and a mode monitor need), and put back on the plane.
#[allow(clippy::too_many_arguments)]
fn port_mode(
    sim: &Simulation,
    eps: &(dyn Fn(f64, f64, f64) -> f64 + Sync),
    boundaries: &Boundaries,
    axis: Axis,
    plane: usize,
    window: &[std::ops::Range<usize>; 2],
    te: bool,
    frequency: Frequency,
) -> Result<PortMode3d> {
    let g = sim.grid();
    let mut cut = g;
    // nodes plane − 3 to plane + 5 along the axis
    match axis {
        Axis::X => {
            cut.nx = 8;
            cut.x0 = g.node(Axis::X, plane - 3);
        }
        Axis::Y => {
            cut.ny = 8;
            cut.y0 = g.node(Axis::Y, plane - 3);
        }
        Axis::Z => {
            cut.nz = 8;
            cut.z0 = g.node(Axis::Z, plane - 3);
        }
    }
    let mut fdfd = Boundaries3d {
        x: boundaries.x,
        y: boundaries.y,
        z: boundaries.z,
        reflection: boundaries.cpml.reflection,
        order: boundaries.cpml.order,
        real_stretch: 0.0,
    };
    match axis {
        Axis::X => fdfd.x = Edges::Pml { low: 0, high: 0 },
        Axis::Y => fdfd.y = Edges::Pml { low: 0, high: 0 },
        Axis::Z => fdfd.z = Edges::Pml { low: 0, high: 0 },
    }
    let wavelength = sim.fdfd_wavelength(frequency)?;
    let solver = IterativeSolver3d::new(
        cut,
        wavelength,
        |x, y, z| c64::new(eps(x, y, z), 0.0),
        fdfd,
        Formulation::CurlCurl,
    )?;
    let modes =
        solver.port_modes_within(axis, 3, (window[0].clone(), window[1].clone()), PORT_MODES)?;
    let (b, c) = axis.others();
    // the in-plane axis across the guide: TE has its E there
    let across = if axis == Axis::X { Axis::Y } else { Axis::X };
    let (size, _) = modes
        .first()
        .ok_or_else(|| task_error("no mode found"))?
        .shape();
    let share = |m: &PortMode3d| {
        let (mut along, mut total) = (0.0, 0.0);
        for v in 0..size[1] {
            for u in 0..size[0] {
                for t in [b, c] {
                    let e = m.e(t, (u, v)).norm_sqr();
                    total += e;
                    if t == across {
                        along += e;
                    }
                }
            }
        }
        along / total.max(f64::MIN_POSITIVE)
    };
    modes
        .into_iter()
        .find(|m| (share(m) > 0.5) == te)
        .map(|m| m.moved_to(plane))
        .ok_or_else(|| {
            task_error(format!(
                "no {} mode guides on the plane at {} node {plane}",
                if te { "TE" } else { "TM" },
                axis_name(axis)
            ))
        })
}

/// What a run measures, added to its simulation.
struct Monitors {
    /// Per mode source: its incident mode monitor, the direction it launches, and the modes at
    /// the spectrum's frequencies.
    incident: Vec<(usize, Direction)>,
    /// Per monitor of the job, what the simulation numbers it.
    added: Vec<Added>,
    /// The spectrum's frequencies, c/µm, as the monitors take them.
    frequencies: Vec<Frequency>,
}

enum Added {
    Mode {
        name: String,
        n: usize,
        axis: Axis,
    },
    Flux {
        name: String,
        n: usize,
        axis: Axis,
    },
    FluxBox {
        name: String,
        faces: Vec<(usize, f64)>,
    },
    Field {
        n: usize,
        wavelengths_um: Vec<f64>,
    },
    Resonance {
        name: String,
        probe: usize,
        component: String,
        position_um: Vec<f64>,
    },
}

/// The values of `field` on a 3D grid's plane k, as rows from y0 up.
fn plane_values(sim: &Simulation, shown: Shown, k: usize) -> Vec<f64> {
    let g = sim.grid();
    let plane = g.nx * g.ny;
    let range = k * plane..(k + 1) * plane;
    match shown {
        Shown::Component(f, c) => {
            let v = if f == Field::E { sim.e(c) } else { sim.h(c) };
            v[range].to_vec()
        }
        Shown::Squared(f) => {
            let mut out = vec![0.0; plane];
            for c in Axis::ALL {
                let v = if f == Field::E { sim.e(c) } else { sim.h(c) };
                for (o, x) in out.iter_mut().zip(&v[range.clone()]) {
                    *o += x * x;
                }
            }
            out
        }
    }
}

/// The base64 of `bytes` (RFC 4648, with padding).
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (k, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if k <= chunk.len() {
                out.push(TABLE[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A frame of the field on the pictured part of plane k: averaged over blocks of cells so that
/// its longer side is at most [`FRAME_SIDE`] pixels, and quantized to signed bytes of its peak.
fn frame(sim: &Simulation, setup: &Setup, step: usize, time: f64) -> Event {
    let g = sim.grid();
    let values = plane_values(sim, setup.shown, setup.view_k);
    let [mx, my] = setup.margins;
    let (ix, iy) = (g.nx - 2 * mx, g.ny - 2 * my);
    let block = ix.max(iy).div_ceil(FRAME_SIDE).max(1);
    let (px, py) = (ix.div_ceil(block), iy.div_ceil(block));
    let mut pixels = vec![0.0f64; px * py];
    for (pj, row) in pixels.chunks_mut(px).enumerate() {
        for (pi, p) in row.iter_mut().enumerate() {
            let (mut sum, mut count) = (0.0, 0usize);
            for j in pj * block..((pj + 1) * block).min(iy) {
                for i in pi * block..((pi + 1) * block).min(ix) {
                    sum += values[(my + j) * g.nx + mx + i];
                    count += 1;
                }
            }
            *p = sum / count.max(1) as f64;
        }
    }
    let peak = pixels.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let scale = if peak > 0.0 { 127.0 / peak } else { 0.0 };
    let bytes: Vec<u8> = pixels
        .iter()
        .map(|v| (v * scale).round().clamp(-127.0, 127.0) as i8 as u8)
        .collect();
    let [x, y] = setup.inner();
    Event::FdtdFrame {
        step,
        time_um: time,
        field: setup.shown.name(),
        z_um: setup.view_z_um,
        nx: px,
        ny: py,
        x_um: [x[0], x[0] + (px * block) as f64 * g.dx],
        y_um: [y[0], y[0] + (py * block) as f64 * g.dy],
        peak,
        data: base64(&bytes),
    }
}

/// The run's spectra now: each monitor's, referred to the incident power (or to the pulse's
/// spectrum, when a dipole drives the run).
fn spectra(sim: &Simulation, setup: &Setup, m: &Monitors, last: bool) -> Vec<Event> {
    let steps = sim.steps();
    let dt = sim.dt();
    let waveform = setup.waveform();
    let fs = &m.frequencies;
    // the pulse is over (to e^(−36)) six widths after its peak: its transform is that far's
    let Waveform::Gaussian { width, delay, .. } = waveform else {
        unreachable!("a pulse")
    };
    let pulse = steps.min(((delay + 6.0 * width) / dt).ceil() as usize + 1);
    // each mode source's incident amplitudes, at every frequency
    let launched: Vec<Vec<(c64, c64)>> = m
        .incident
        .iter()
        .map(|&(n, _)| sim.mode_amplitudes(n))
        .collect();
    // the incident power at each frequency, or the pulse's |spectrum|² for a dipole's run
    let incident: Vec<f64> = (0..fs.len())
        .map(|k| {
            let w = waveform.spectrum(Field::H, dt, pulse, fs[k]).norm_sqr();
            let mut total = 0.0;
            for (s, plan) in setup.sources.iter().enumerate() {
                total += match plan {
                    SourcePlan::Mode { .. } => {
                        let j = mode_index(setup, s);
                        let a = launched[j][k];
                        let a = if m.incident[j].1 == Direction::Forward {
                            a.0
                        } else {
                            a.1
                        };
                        a.norm_sqr()
                    }
                    SourcePlan::PlaneWave(p) => {
                        let g = sim.grid();
                        let along = [p.direction.0, p.direction.1, p.direction.2]
                            .iter()
                            .position(|&d| d != 0)
                            .unwrap_or(0);
                        let span = |a: usize| -> f64 {
                            let (lo, hi) = (
                                [p.low.0, p.low.1, p.low.2][a],
                                [p.high.0, p.high.1, p.high.2][a],
                            );
                            (hi - lo) as f64 * g.step(Axis::ALL[a])
                        };
                        let area: f64 = (0..3).filter(|&a| a != along).map(span).product();
                        0.5 * p.eps.sqrt() * w * area
                    }
                    SourcePlan::Beam(b) => {
                        let g = sim.grid();
                        let width = if setup.three {
                            std::f64::consts::PI * b.waist * b.waist / 2.0
                        } else {
                            b.waist * (std::f64::consts::PI / 2.0).sqrt() * g.dz
                        };
                        0.5 * b.eps.sqrt() * w * width
                    }
                    SourcePlan::Dipole(_) => f64::NAN,
                };
            }
            if total.is_nan() { w } else { total }
        })
        .collect();
    let referred = !setup
        .sources
        .iter()
        .any(|s| matches!(s, SourcePlan::Dipole(_)));
    let quantity = |what: &str| {
        if referred {
            format!("{what} / incident power")
        } else {
            format!("{what} per unit source spectrum, |field|^2 um^2")
        }
    };
    let wavelengths_um: Vec<f64> = fs.iter().map(|f| 1.0 / f.to_natural()).collect();
    let time_um = sim.time();
    let mut out = Vec::new();
    let per = |v: Vec<f64>| -> Vec<f64> { v.iter().zip(&incident).map(|(a, b)| a / b).collect() };
    // each mode source's reflection, back into its guide
    for (s, plan) in setup.sources.iter().enumerate() {
        if let SourcePlan::Mode { axis, .. } = plan {
            let (n, d) = m.incident[mode_index(setup, s)];
            let amplitudes = sim.mode_amplitudes(n);
            let (forward, back): (Vec<f64>, Vec<f64>) = amplitudes
                .iter()
                .map(|a| {
                    let (f, b) = if d == Direction::Forward {
                        (a.0, a.1)
                    } else {
                        (a.1, a.0)
                    };
                    (f.norm_sqr(), b.norm_sqr())
                })
                .unzip();
            out.push(Event::FdtdSpectrum {
                monitor: format!("source {}", s + 1),
                kind: "reflection".into(),
                quantity: "power back into the source's mode / power the source launches into it"
                    .into(),
                wavelengths_um: wavelengths_um.clone(),
                series: vec![SpectrumSeries {
                    label: format!("back along {}", axis_name(*axis)),
                    values: back.iter().zip(&forward).map(|(b, f)| b / f).collect(),
                }],
                time_um,
                last,
            });
        }
    }
    for a in &m.added {
        match a {
            Added::Mode { name, n, axis } => {
                let amplitudes = sim.mode_amplitudes(*n);
                let (plus, minus): (Vec<f64>, Vec<f64>) = amplitudes
                    .iter()
                    .map(|a| (a.0.norm_sqr(), a.1.norm_sqr()))
                    .unzip();
                out.push(Event::FdtdSpectrum {
                    monitor: name.clone(),
                    kind: "mode".into(),
                    quantity: quantity("power in the guide's mode"),
                    wavelengths_um: wavelengths_um.clone(),
                    series: vec![
                        SpectrumSeries {
                            label: format!("towards +{}", axis_name(*axis)),
                            values: per(plus),
                        },
                        SpectrumSeries {
                            label: format!("towards -{}", axis_name(*axis)),
                            values: per(minus),
                        },
                    ],
                    time_um,
                    last,
                });
            }
            Added::Flux { name, n, axis } => out.push(Event::FdtdSpectrum {
                monitor: name.clone(),
                kind: "flux".into(),
                quantity: quantity("flux"),
                wavelengths_um: wavelengths_um.clone(),
                series: vec![SpectrumSeries {
                    label: format!("towards +{}", axis_name(*axis)),
                    values: per(sim.flux(*n)),
                }],
                time_um,
                last,
            }),
            Added::FluxBox { name, faces } => {
                let mut total = vec![0.0; fs.len()];
                for &(n, sign) in faces {
                    for (t, f) in total.iter_mut().zip(sim.flux(n)) {
                        *t += sign * f;
                    }
                }
                out.push(Event::FdtdSpectrum {
                    monitor: name.clone(),
                    kind: "flux_box".into(),
                    quantity: quantity("flux"),
                    wavelengths_um: wavelengths_um.clone(),
                    series: vec![SpectrumSeries {
                        label: "out of the box".into(),
                        values: per(total),
                    }],
                    time_um,
                    last,
                });
            }
            Added::Field { .. } | Added::Resonance { .. } => {}
        }
    }
    out
}

/// The index among the job's mode sources of source `s`.
fn mode_index(setup: &Setup, s: usize) -> usize {
    setup.sources[..s]
        .iter()
        .filter(|p| matches!(p, SourcePlan::Mode { .. }))
        .count()
}

pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let mut setup = Setup::new(job)?;
    run.record(&setup.scene()?)?;
    let g = setup.grid;
    let eps = setup.permittivity()?;
    let mut sim = Simulation::new_parallel(g, &eps, setup.boundaries, setup.courant)?;
    let dt = sim.dt();
    let waveform = setup.waveform();
    let frequencies: Vec<Frequency> = setup
        .wavelengths
        .iter()
        .map(|w| Frequency::natural(1.0 / w))
        .collect::<Result<_>>()?;
    let carrier = Frequency::natural(setup.carrier)?;
    let at = |what: String| {
        move |e: Error| match e {
            Error::InvalidValue { reason, .. } => task_error(format!("{what}: {reason}")),
            other => other,
        }
    };
    // the sources, and each mode source's incident monitor just after it
    let mut incident = Vec::new();
    for k in 0..setup.sources.len() {
        let what = format!("source {} ({})", k + 1, setup.task.source[k].kind);
        match &mut setup.sources[k] {
            SourcePlan::Mode {
                axis,
                plane,
                direction,
                window,
                te,
            } => {
                let mode = port_mode(
                    &sim,
                    &eps,
                    &setup.boundaries,
                    *axis,
                    *plane,
                    window,
                    *te,
                    carrier,
                )
                .map_err(at(what.clone()))?;
                sim.add_mode_source(&mode, *direction, waveform)
                    .map_err(at(what.clone()))?;
                let after = match direction {
                    Direction::Forward => *plane + 3,
                    Direction::Backward => *plane - 3,
                };
                let modes = frequencies
                    .iter()
                    .map(|&f| {
                        port_mode(&sim, &eps, &setup.boundaries, *axis, after, window, *te, f)
                    })
                    .collect::<Result<Vec<_>>>()
                    .map_err(at(what.clone()))?;
                incident.push((
                    sim.add_mode_monitor(&modes).map_err(at(what.clone()))?,
                    *direction,
                ));
            }
            SourcePlan::Dipole(d) => sim.add_dipole(*d).map_err(at(what))?,
            SourcePlan::PlaneWave(p) => {
                let corner = g.e_position(Axis::X, p.low);
                p.eps = eps(corner[0], corner[1], corner[2]);
                sim.add_plane_wave(*p).map_err(at(what))?;
            }
            SourcePlan::Beam(b) => {
                let mut centre = [0.0; 3];
                let (bb, cc) = b.axis.others();
                centre[b.axis.index()] = g.node(b.axis, b.plane);
                centre[bb.index()] = b.centre.0;
                centre[cc.index()] = b.centre.1;
                b.eps = eps(centre[0], centre[1], centre[2]);
                sim.add_beam(b, waveform).map_err(at(what))?;
            }
        }
    }
    let mut added = Vec::new();
    for (k, plan) in setup.monitors.iter().enumerate() {
        let what = format!("monitor {} ({})", k + 1, setup.task.monitor[k].kind);
        added.push(match plan {
            MonitorPlan::Mode {
                name,
                axis,
                plane,
                window,
                te,
            } => {
                let modes = frequencies
                    .iter()
                    .map(|&f| {
                        port_mode(&sim, &eps, &setup.boundaries, *axis, *plane, window, *te, f)
                    })
                    .collect::<Result<Vec<_>>>()
                    .map_err(at(what.clone()))?;
                Added::Mode {
                    name: name.clone(),
                    n: sim.add_mode_monitor(&modes).map_err(at(what))?,
                    axis: *axis,
                }
            }
            MonitorPlan::Flux { name, plane } => Added::Flux {
                name: name.clone(),
                n: sim.add_flux(*plane, &frequencies).map_err(at(what))?,
                axis: plane.axis,
            },
            MonitorPlan::FluxBox { name, faces } => Added::FluxBox {
                name: name.clone(),
                faces: faces
                    .iter()
                    .map(|&(face, sign)| Ok((sim.add_flux(face, &frequencies)?, sign)))
                    .collect::<Result<Vec<_>>>()
                    .map_err(at(what))?,
            },
            MonitorPlan::Field { wavelengths_um } => {
                let fs = wavelengths_um
                    .iter()
                    .map(|w| Frequency::natural(1.0 / w))
                    .collect::<Result<Vec<_>>>()?;
                let k = setup.view_k;
                Added::Field {
                    n: sim
                        .add_dft((0, 0, k), (g.nx - 1, g.ny - 1, k), &fs)
                        .map_err(at(what))?,
                    wavelengths_um: wavelengths_um.clone(),
                }
            }
            MonitorPlan::Resonance {
                name,
                field,
                component,
                at: cell,
                position_um,
            } => Added::Resonance {
                name: name.clone(),
                probe: sim.add_probe(*field, *component, *cell).map_err(at(what))?,
                component: component_name(*field, *component),
                position_um: position_um.clone(),
            },
        });
    }
    let monitors = Monitors {
        incident,
        added,
        frequencies,
    };
    // the points the decay is judged at
    let points = setup.probe_points();
    let components = setup.probe_components();
    // each point's value in a field's components
    let probes: Vec<usize> = points
        .iter()
        .map(|&(i, j, k)| (k * g.ny + j) * g.nx + i)
        .collect();
    let Waveform::Gaussian { width, delay, .. } = waveform else {
        unreachable!("a pulse")
    };
    let pulse_end = delay + 6.0 * width;
    let period = 1.0 / setup.carrier;
    let check = 10.0 * period;
    let limit = match setup.until {
        Until::Decay { limit, .. } => limit,
        Until::Time(t) => t,
    };
    // the time light takes to cross the window's longest side at the slowest speed in it
    let crossing = {
        let mut slowest: f64 = 1.0;
        for k in [0usize, g.cells() / 3, g.cells() / 2, 2 * g.cells() / 3] {
            let (i, j, kk) = (k % g.nx, (k / g.nx) % g.ny, k / (g.nx * g.ny));
            let p = g.e_position(Axis::X, (i, j, kk));
            slowest = slowest.max(eps(p[0], p[1], p[2]).sqrt());
        }
        let longest = [Axis::X, Axis::Y, Axis::Z]
            .iter()
            .map(|&a| g.n(a) as f64 * g.step(a))
            .fold(0.0, f64::max);
        longest * slowest
    };
    let mut every = setup
        .every
        .unwrap_or_else(|| (pulse_end + crossing) / 80.0)
        .max(dt);
    record_solver(run, &setup, &sim, every, pulse_end)?;
    // the run, frame by frame
    let mut peaks = vec![0.0f64; points.len()];
    let mut recent = vec![0.0f64; points.len()];
    let mut block = vec![0.0f64; points.len()];
    let mut block_end = pulse_end + check;
    let mut next_frame = every;
    let mut frames = 0usize;
    let mut frame_seconds = 0.0;
    let mut step_seconds = 0.0;
    let mut since = Instant::now();
    let mut since_frame = 0.0;
    let mut last_progress = Instant::now();
    let mut last_spectra = Instant::now();
    let mut spectra_seconds = 0.0;
    let mut decayed = false;
    let mut last_decay = None;
    // |field|² at a point now, both parts of a complex field
    let level = |s: &Simulation, r: usize| -> f64 {
        components
            .iter()
            .map(|&(f, c)| {
                let (re, im) = match f {
                    Field::E => (s.e(c)[r], s.e_imaginary(c).map_or(0.0, |v| v[r])),
                    Field::H => (s.h(c)[r], s.h_imaginary(c).map_or(0.0, |v| v[r])),
                };
                re * re + im * im
            })
            .sum()
    };
    loop {
        if stop.reason().is_some() {
            break;
        }
        sim.step();
        let t = sim.time();
        for (k, &r) in probes.iter().enumerate() {
            let v = level(&sim, r);
            peaks[k] = peaks[k].max(v);
            recent[k] = recent[k].max(v);
            if t > pulse_end {
                block[k] = block[k].max(v);
            }
        }
        if let Until::Decay { fraction, .. } = setup.until
            && t >= block_end
        {
            // (a point the light has never reached holds nothing up)
            let worst = peaks
                .iter()
                .zip(&block)
                .map(|(&p, &b)| if p > 0.0 { b / p } else { 0.0 })
                .fold(0.0, f64::max);
            last_decay = Some(worst);
            if worst <= fraction && peaks.iter().any(|&p| p > 0.0) {
                decayed = true;
            }
            block.iter_mut().for_each(|b| *b = 0.0);
            block_end += check;
        }
        let done = decayed || t >= limit;
        let framed = t >= next_frame;
        // the frames on the run's time, the progress at least every PROGRESS_SECONDS of the
        // clock, and the spectra no more often than SPECTRA_SECONDS nor than ten times what
        // they last took
        if framed || done || last_progress.elapsed().as_secs_f64() >= PROGRESS_SECONDS {
            let stepped = since.elapsed().as_secs_f64();
            step_seconds += stepped;
            let began = Instant::now();
            let decay = peaks
                .iter()
                .zip(&recent)
                .map(|(&p, &r)| if p > 0.0 { r / p } else { 0.0 })
                .fold(0.0, f64::max);
            recent.iter_mut().for_each(|r| *r = 0.0);
            since_frame += stepped;
            if framed && frames < MOST_FRAMES {
                run.record(&frame(&sim, &setup, sim.steps(), t))?;
                frames += 1;
                // fewer frames as the run goes on, and when one costs too much against the
                // steps since the last
                if frames.is_multiple_of(FRAMES_BEFORE_DOUBLING)
                    || began.elapsed().as_secs_f64() > FRAME_SHARE * since_frame
                {
                    every *= 2.0;
                }
                since_frame = 0.0;
            }
            if !done
                && last_spectra.elapsed().as_secs_f64()
                    >= SPECTRA_SECONDS.max(10.0 * spectra_seconds)
            {
                let at = Instant::now();
                for e in spectra(&sim, &setup, &monitors, false) {
                    run.record(&e)?;
                }
                spectra_seconds = at.elapsed().as_secs_f64();
                last_spectra = Instant::now();
            }
            frame_seconds += began.elapsed().as_secs_f64();
            last_progress = Instant::now();
            run.record(&Event::FdtdProgress {
                step: sim.steps(),
                time_um: t,
                until_um: limit,
                decay,
                fraction: match setup.until {
                    Until::Decay { fraction, .. } => Some(fraction),
                    Until::Time(_) => None,
                },
                steps_seconds: step_seconds,
                frames_seconds: frame_seconds,
                cell_updates_per_second: (g.cells() * sim.steps()) as f64 / step_seconds.max(1e-9),
            })?;
            if framed {
                next_frame = t + every;
            }
            since = Instant::now();
        }
        if done {
            break;
        }
    }
    // what the run measured
    for e in spectra(&sim, &setup, &monitors, true) {
        run.record(&e)?;
    }
    for a in &monitors.added {
        match a {
            Added::Resonance {
                name,
                probe,
                component,
                position_um,
            } => {
                let start = ((pulse_end / dt).ceil() as usize).min(sim.probe(*probe).len());
                let signal: Vec<c64> = sim.probe_complex(*probe)[start..].to_vec();
                let (lowest, highest) = monitors
                    .frequencies
                    .iter()
                    .map(|f| f.to_natural())
                    .fold((f64::MAX, f64::MIN), |(a, b), f| (a.min(f), b.max(f)));
                let (lo, hi) = if highest > lowest {
                    (lowest, highest)
                } else {
                    (
                        setup.carrier - 0.5 * setup.bandwidth,
                        setup.carrier + 0.5 * setup.bandwidth,
                    )
                };
                // the field is narrow-band, the step oversamples it many times over: every
                // q-th sample, four a period of the band's highest frequency, and at most
                // RESONANCE_SAMPLES of them, which harmonic inversion takes without its powers
                // of the eigenvalues running away
                let q = ((0.25 / (hi * dt)).floor() as usize).max(1);
                let signal: Vec<c64> = signal
                    .iter()
                    .step_by(q)
                    .take(RESONANCE_SAMPLES)
                    .copied()
                    .collect();
                let resonances = if signal.len() >= 8 {
                    harmonic_inversion(
                        &signal,
                        dt * q as f64,
                        Frequency::natural(lo)?,
                        Frequency::natural(hi)?,
                    )?
                } else {
                    Vec::new()
                };
                // the terms the signal holds (a spurious one's error is large, and its
                // amplitude may be any size), decaying, and not lost in the largest's noise
                let held = |r: &&crate::fdtd::Resonance| r.error < 1e-2 && r.decay > 0.0;
                let largest = resonances
                    .iter()
                    .filter(held)
                    .map(|r| r.amplitude.norm())
                    .fold(0.0, f64::max);
                let found = resonances
                    .iter()
                    .filter(held)
                    .filter(|r| r.amplitude.norm() > 1e-6 * largest)
                    .map(|r| FoundResonance {
                        wavelength_um: 1.0 / r.frequency,
                        frequency: r.frequency,
                        q: r.q(),
                        decay: r.decay,
                        amplitude: r.amplitude.norm(),
                        error: r.error,
                    })
                    .collect();
                run.record(&Event::FdtdResonances {
                    monitor: name.clone(),
                    position_um: position_um.clone(),
                    component: component.clone(),
                    resonances: found,
                })?;
            }
            Added::Field { n, wavelengths_um } => {
                let dft = sim.dft(*n);
                let [mx, my] = setup.margins;
                let (ix, iy) = (g.nx - 2 * mx, g.ny - 2 * my);
                let [x, y] = setup.inner();
                for (f, &w) in wavelengths_um.iter().enumerate() {
                    let mut values = vec![0.0f64; ix * iy];
                    for j in 0..iy {
                        for i in 0..ix {
                            let at = (mx + i, my + j, setup.view_k);
                            values[j * ix + i] = Axis::ALL
                                .iter()
                                .map(|&c| {
                                    dft.value(Field::E, c, at, f).map_or(0.0, |v| v.norm_sqr())
                                })
                                .sum();
                        }
                    }
                    let peak = values
                        .iter()
                        .copied()
                        .fold(0.0, f64::max)
                        .max(f64::MIN_POSITIVE);
                    run.record(&Event::Field {
                        label: if setup.three {
                            format!("|E|^2 by FDTD's transform, on z = {} um", setup.view_z_um)
                        } else {
                            "|E|^2 by FDTD's transform, 2D by the effective index method".into()
                        },
                        wavelength_um: w,
                        z_um: setup.view_z_um,
                        intensity: crate::raster::Raster {
                            nx: ix,
                            ny: iy,
                            x0: x[0],
                            x1: x[1],
                            y0: y[0],
                            y1: y[1],
                            values: values.iter().map(|v| (v / peak) as f32).collect(),
                        },
                    })?;
                }
            }
            Added::Mode { .. } | Added::Flux { .. } | Added::FluxBox { .. } => {}
        }
    }
    if let Some(d) = last_decay {
        run.record(&Event::SolveError {
            point: None,
            value: setup.task.wavelength_um,
            measure: "field left at the monitors (|field|^2 over its peak, last check)".into(),
            error: d,
        })?;
    }
    Ok(())
}

/// The run's [`Event::Solver`]: the grid, the time step and what the run does.
fn record_solver(
    run: &mut Run,
    setup: &Setup,
    sim: &Simulation,
    every: f64,
    pulse_end: f64,
) -> Result<()> {
    let g = sim.grid();
    let boundary = |a: Axis| match edges(&setup.boundaries, a) {
        Edges::Pml { low: 0, .. } => "walls".to_owned(),
        Edges::Pml { low, .. } => format!("CPMLs of {low} cells"),
        Edges::Bloch { k: 0.0 } => "periodic".to_owned(),
        Edges::Bloch { k } => format!("Bloch-periodic, k = {k} rad/um"),
    };
    let mut details = vec![
        [
            "dimensions".to_owned(),
            if setup.three {
                format!("3D: {} x {} x {} cells", g.nx, g.ny, g.nz)
            } else {
                format!(
                    "2D by the effective index method ({}): {} x {} cells",
                    if setup.te {
                        "TE: H along z, E in the plane"
                    } else {
                        "TM: E along z"
                    },
                    g.nx,
                    g.ny
                )
            },
        ],
        [
            "time step".into(),
            format!("{:.6} um/c, Courant number {}", sim.dt(), setup.courant),
        ],
        ["x".into(), boundary(Axis::X)],
        ["y".into(), boundary(Axis::Y)],
    ];
    if setup.three {
        details.push(["z".into(), boundary(Axis::Z)]);
    }
    details.extend([
        [
            "pulse".into(),
            format!(
                "Gaussian on a carrier at {} um, {:.4} c/um wide at half power, over by {:.2} um/c",
                setup.task.wavelength_um, setup.bandwidth, pulse_end
            ),
        ],
        [
            "permittivity".into(),
            if setup.three {
                format!("the stack's materials at {} um (non-dispersive)", setup.task.wavelength_um)
            } else {
                format!(
                    "each point's slab mode index squared at {} um (non-dispersive)",
                    setup.task.wavelength_um
                )
            },
        ],
        [
            "spectrum".into(),
            format!(
                "{} wavelength{} from {} to {} um",
                setup.wavelengths.len(),
                if setup.wavelengths.len() == 1 { "" } else { "s" },
                setup.wavelengths.first().copied().unwrap_or(0.0),
                setup.wavelengths.last().copied().unwrap_or(0.0)
            ),
        ],
        [
            "stops".into(),
            match setup.until {
                Until::Decay { fraction, limit } => format!(
                    "when |field|^2 at the monitors stays below {fraction:e} of its peak for {:.1} um/c, or at {limit} um/c",
                    10.0 / setup.carrier
                ),
                Until::Time(t) => format!("at {t} um/c"),
            },
        ],
        [
            "frames".into(),
            format!(
                "{} every {every:.3} um/c at first, fewer as the run goes on (at most {MOST_FRAMES})",
                setup.shown.name()
            ),
        ],
        ["sources".into(), setup.sources.len().to_string()],
        ["monitors".into(), setup.monitors.len().to_string()],
    ]);
    run.record(&Event::Solver {
        module: "fdtd".into(),
        cells: [g.nx, g.ny],
        step_um: setup.task.step_nm / 1000.0,
        unknowns: 6 * g.cells(),
        details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_is_rfc_4648s() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0x80, 0x7f]), "/4B/");
    }

    /// A job's events, run headless into a scratch directory.
    fn run_job(text: &str, tag: &str) -> Vec<Event> {
        let job = Job::parse(text).unwrap();
        let root =
            std::env::temp_dir().join(format!("photonoxide-fdtd-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut run = Run::create(&root, &job).unwrap();
        super::super::execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events = crate::run::replay(run.dir()).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        events
    }

    /// A monitor's last spectrum: its wavelengths and its first series.
    fn last_spectrum(events: &[Event], name: &str) -> (Vec<f64>, Vec<f64>) {
        events
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::FdtdSpectrum {
                    monitor,
                    wavelengths_um,
                    series,
                    last: true,
                    ..
                } if monitor == name => Some((wavelengths_um.clone(), series[0].values.clone())),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no spectrum of {name}"))
    }

    /// A short 2D guide with one of each monitor, its pulse run for a fixed time.
    const SMALL: &str = r#"
name = "small"
[task]
kind = "fdtd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.5, 1.5]
y_um = [-1.0, 1.0]
step_nm = 50.0
pml_cells = 8
[task.spectrum]
from_um = 1.5
to_um = 1.6
points = 3
[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]
[[task.source]]
type = "mode"
normal = "x"
at_um = -0.8
[[task.monitor]]
type = "mode"
name = "out"
normal = "x"
at_um = 0.6
[[task.monitor]]
type = "flux"
normal = "x"
at_um = 0.9
y_um = [-0.55, 0.55]
[[task.monitor]]
type = "flux_box"
x_um = [-0.2, 0.2]
y_um = [-0.5, 0.5]
[[task.monitor]]
type = "resonance"
position_um = [0.0, 0.0]
[[task.monitor]]
type = "field"
wavelengths_um = [1.55]
[task.stop]
until = "time"
time_um = 160.0
"#;

    #[test]
    fn a_job_records_frames_progress_spectra_resonances_and_fields() {
        let events = run_job(SMALL, "small");
        let frames: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::FdtdFrame { .. }))
            .collect();
        assert!(frames.len() > 10, "{} frames", frames.len());
        // the frames' bytes are the picture's pixels
        for f in &frames {
            let Event::FdtdFrame { nx, ny, data, .. } = f else {
                unreachable!()
            };
            assert_eq!(data.len(), (nx * ny).div_ceil(3) * 4);
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FdtdProgress { .. }))
        );
        for name in ["out", "flux 2", "flux box 3", "source 1"] {
            let (w, v) = last_spectrum(&events, name);
            assert_eq!(w.len(), 3);
            assert!(v.iter().all(|x| x.is_finite()), "{name}: {v:?}");
        }
        // a straight guide: what goes in comes out, the same through the flux plane
        let (_, out) = last_spectrum(&events, "out");
        let (_, flux) = last_spectrum(&events, "flux 2");
        let (_, box_out) = last_spectrum(&events, "flux box 3");
        for k in 0..3 {
            assert!((out[k] - 1.0).abs() < 0.05, "{out:?}");
            assert!((flux[k] - out[k]).abs() < 0.05, "{flux:?} against {out:?}");
            // nothing is lost or made inside the box of the lossless guide
            assert!(box_out[k].abs() < 0.02, "{box_out:?}");
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FdtdResonances { .. }))
        );
        assert!(events.iter().any(|e| matches!(e, Event::Field { .. })));
    }

    #[test]
    fn a_job_is_the_same_bits_on_any_number_of_threads() {
        // what the run measured: when it records its frames and its spectra on the way follows
        // the clock (they are fewer when they cost too much), but each frame is its step's
        let on = |threads: usize| {
            let events = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| run_job(SMALL, &format!("threads-{threads}")));
            let frames: Vec<(usize, String)> = events
                .iter()
                .filter_map(|e| match e {
                    Event::FdtdFrame { step, data, .. } => Some((*step, data.clone())),
                    _ => None,
                })
                .collect();
            let measured: Vec<Event> = events
                .into_iter()
                .filter(|e| match e {
                    Event::FdtdSpectrum { last, .. } => *last,
                    Event::FdtdResonances { .. } | Event::Field { .. } => true,
                    _ => false,
                })
                .collect();
            (frames, measured)
        };
        let (frames, measured) = on(1);
        assert!(!measured.is_empty());
        for threads in [4, 20] {
            let (other_frames, other) = on(threads);
            assert!(other == measured, "{threads} threads differ from one");
            for (step, data) in &other_frames {
                if let Some((_, same)) = frames.iter().find(|(s, _)| s == step) {
                    assert!(same == data, "{threads} threads, step {step}");
                }
            }
        }
    }

    #[test]
    fn the_check_refuses_what_the_run_would() {
        for (from, to, says) in [
            (
                "type = \"mode\"\nnormal",
                "type = \"laser\"\nnormal",
                "unknown type \"laser\"",
            ),
            ("at_um = -0.8", "at_um = -1.4", "clear of the CPML"),
            (
                "position_um = [0.0, 0.0]",
                "position_um = [0.0, 0.0]\ncomponent = \"Ez\"",
                "isn't one of a 2D TE job's",
            ),
            (
                "step_nm = 50.0",
                "step_nm = 50.0\nz_um = [1.0, 3.0]",
                "z_um is for a 3D job",
            ),
            (
                "step_nm = 50.0",
                "step_nm = 50.0\ncourant = 1.5",
                "courant must be in (0, 1]",
            ),
            (
                "time_um = 160.0",
                "time_um = 160.0\nfraction = 1e-6",
                "fraction and limit_um are for",
            ),
            (
                "name = \"out\"",
                "name = \"flux 2\"",
                "two monitors are named",
            ),
            (
                "[[task.source]]\ntype = \"mode\"\nnormal = \"x\"\nat_um = -0.8\n",
                "",
                "at least one [[task.source]]",
            ),
            ("at_um = 0.9", "at_um = 1.4", "inside its CPMLs"),
        ] {
            let text = SMALL.replace(from, to);
            assert_ne!(text, SMALL, "{from}");
            let job = Job::parse(&text).unwrap();
            let refused = super::super::check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{says}: {refused}");
        }
        super::super::check(&Job::parse(SMALL).unwrap()).unwrap();
        for text in [
            include_str!("../../jobs/mmi-fdtd.toml"),
            include_str!("../../jobs/ring-fdtd.toml"),
        ] {
            super::super::check(&Job::parse(text).unwrap()).unwrap();
        }
    }

    #[test]
    fn a_3d_job_runs_on_the_stack() {
        let text = r#"
name = "strip-3d"
[task]
kind = "fdtd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
dimensions = 3
x_um = [-1.2, 1.2]
y_um = [-1.0, 1.0]
z_um = [1.3, 2.9]
step_nm = 80.0
pml_cells = 4
[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]
[[task.source]]
type = "mode"
normal = "x"
at_um = -0.4
[[task.monitor]]
type = "mode"
name = "out"
normal = "x"
at_um = -0.1
[task.stop]
until = "time"
time_um = 20.0
"#;
        let events = run_job(text, "3d");
        assert!(events.iter().any(|e| matches!(e, Event::FdtdFrame { .. })));
        let (_, out) = last_spectrum(&events, "out");
        assert!(out[0].is_finite() && out[0] > 0.0, "{out:?}");
    }

    /// A guide cut by a 0.3 µm gap, 2D TE on 40 nm cells with absorbers of 20, as an `"fdtd"`
    /// job (`kind` and its own lines) or an `"fdfd"` one with ports where the FDTD job's mode
    /// source and monitor are.
    fn gap(kind: &str, own: &str) -> String {
        format!(
            r#"
name = "gap-{kind}"
[task]
kind = "{kind}"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
polarization = "te"
x_um = [-2.4, 2.4]
y_um = [-1.4, 1.4]
step_nm = 40.0
pml_cells = 20
[[task.rect]]
layer = "Si"
center_um = [-2.0, 0.0]
size_um = [3.7, 0.5]
[[task.rect]]
layer = "Si"
center_um = [2.0, 0.0]
size_um = [3.7, 0.5]
{own}
"#
        )
    }

    /// The FDTD job's spectra against FDFD's S-parameters for the same structure on the same
    /// grid, with the permittivity of the carrier, at the leapfrog's frequencies: at each
    /// wavelength, the difference in transmitted and in reflected power.
    fn against_fdfd(fdtd: &str, fdfd: &str, out: &str) -> Vec<(f64, f64, f64)> {
        let events = run_job(fdtd, "against");
        let (wavelengths, through) = last_spectrum(&events, out);
        let (_, back) = last_spectrum(&events, "source 1");
        let setup = Setup::new(&Job::parse(fdtd).unwrap()).unwrap();
        let dt = time_step(&setup.grid, &setup.boundaries, setup.courant);
        let tilde: Vec<f64> = wavelengths
            .iter()
            .map(|w| {
                let omega = std::f64::consts::TAU / w;
                std::f64::consts::TAU / (2.0 / dt * (0.5 * omega * dt).sin())
            })
            .collect();
        let s = super::super::fdfd::fdfd_s_parameters_frozen(
            &Job::parse(fdfd).unwrap(),
            &tilde,
            setup.task.wavelength_um,
        )
        .unwrap();
        (0..wavelengths.len())
            .map(|k| {
                let (t, r) = (s.s[k][1][0].norm_sqr(), s.s[k][0][0].norm_sqr());
                println!(
                    "{:.4} um: through {:.6} against {t:.6}, back {:.6} against {r:.6}",
                    wavelengths[k], through[k], back[k]
                );
                (wavelengths[k], (through[k] - t).abs(), (back[k] - r).abs())
            })
            .collect()
    }

    /// A gap's transmission and reflection by an `"fdtd"` job, against the `"fdfd"` job of the
    /// same gap. At the carrier they agree to the CPMLs' difference from FDFD's PMLs; away from
    /// it the mode source, which launches the carrier's mode at every frequency, puts in a
    /// little that isn't the guide's own mode there, and some of it reaches the output: 1.3e-4
    /// at the carrier, 2.4e-3 at 3 % from it.
    #[test]
    fn a_gaps_transmission_and_reflection_are_fdfds() {
        let fdtd = gap(
            "fdtd",
            r#"
[task.spectrum]
from_um = 1.5
to_um = 1.6
points = 5
[[task.source]]
type = "mode"
normal = "x"
at_um = -1.2
[[task.monitor]]
type = "mode"
name = "out"
normal = "x"
at_um = 1.0
[task.stop]
fraction = 1e-10
"#,
        );
        let fdfd = gap(
            "fdfd",
            r#"
[[task.port]]
x_um = -1.2
side = "left"
[[task.port]]
x_um = 1.0
side = "right"
"#,
        );
        let differences = against_fdfd(&fdtd, &fdfd, "out");
        for (w, t, r) in differences {
            let off = (1.0 / w - 1.0 / 1.55).abs() * 1.55;
            assert!(
                t < 2e-4 + 0.08 * off && r < 1e-4 + 0.02 * off,
                "{w}: {t} {r}"
            );
        }
    }

    /// The built-in MMI job against the `"fdfd"` job of the same device, 217 500 cells for some
    /// 80 000 steps: about two minutes on 20 threads, so run on demand, in release:
    /// `cargo test --release --lib job::fdtd -- --ignored`.
    #[test]
    #[ignore]
    fn the_mmi_job_reproduces_the_fdfd_jobs_s_parameters() {
        let text = include_str!("../../jobs/mmi-fdtd.toml");
        let started = Instant::now();
        let events = run_job(text, "mmi");
        let (wavelengths, upper) = last_spectrum(&events, "upper output");
        let (_, lower) = last_spectrum(&events, "lower output");
        let (_, reflected) = last_spectrum(&events, "source 1");
        // FDFD at the leapfrog's own frequencies, on the same grid
        let setup = Setup::new(&Job::parse(text).unwrap()).unwrap();
        let dt = time_step(&setup.grid, &setup.boundaries, setup.courant);
        let tilde: Vec<f64> = wavelengths
            .iter()
            .map(|w| {
                let omega = std::f64::consts::TAU / w;
                std::f64::consts::TAU / (2.0 / dt * (0.5 * omega * dt).sin())
            })
            .collect();
        println!("fdtd in {:.1} s", started.elapsed().as_secs_f64());
        let fdfd = Job::parse(include_str!("../../jobs/mmi-fdfd.toml")).unwrap();
        let s = super::super::fdfd::fdfd_s_parameters_frozen(&fdfd, &tilde, 1.55).unwrap();
        let (mut worst, mut back): (f64, f64) = (0.0, 0.0);
        for k in 0..wavelengths.len() {
            let p = |q: usize| s.s[k][q][0].norm_sqr();
            println!(
                "{:.4} um: upper {:.6} against {:.6}, lower {:.6} against {:.6}, reflected {:.3e} \
                 against {:.3e}",
                wavelengths[k],
                upper[k],
                p(1),
                lower[k],
                p(2),
                reflected[k],
                p(0)
            );
            worst = worst
                .max((upper[k] - p(1)).abs())
                .max((lower[k] - p(2)).abs());
            back = back.max((reflected[k] - p(0)).abs());
        }
        println!(
            "worst {worst:.3e}, reflection {back:.3e}, in {:.1} s",
            started.elapsed().as_secs_f64()
        );
        // 4.3e-4 at the carrier, 1.1e-3 at the band's edge
        assert!(worst < 1.5e-3 && back < 5e-4, "{worst} {back}");
    }

    /// A ring of 1.5 µm beside its bus, 2D TE on 40 nm cells: harmonic inversion of the field
    /// in the ring finds its resonances where the bus's transmission, by its transforms, dips.
    #[test]
    fn a_rings_resonances_are_where_its_bus_dips() {
        let text = r#"
name = "small-ring"
[task]
kind = "fdtd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-3.0, 3.0]
y_um = [-2.6, 2.6]
step_nm = 40.0
pml_cells = 12
[task.spectrum]
from_um = 1.5
to_um = 1.6
points = 41
[[task.rect]]
layer = "Si"
center_um = [0.0, -1.7]
size_um = [8.0, 0.5]
[[task.ring]]
layer = "Si"
center_um = [0.0, 0.4]
radius_um = 1.5
width_um = 0.5
[[task.source]]
type = "mode"
normal = "x"
at_um = -2.0
y_um = [-2.6, -0.9]
[[task.monitor]]
type = "mode"
name = "through"
normal = "x"
at_um = 1.8
y_um = [-2.6, -0.9]
[[task.monitor]]
type = "resonance"
name = "ring"
position_um = [0.0, 1.9]
[task.stop]
until = "time"
time_um = 800.0
"#;
        let events = run_job(text, "ring");
        let (wavelengths, through) = last_spectrum(&events, "through");
        let found = events
            .iter()
            .find_map(|e| match e {
                Event::FdtdResonances { resonances, .. } => Some(resonances.clone()),
                _ => None,
            })
            .unwrap();
        assert!(!found.is_empty());
        // the deepest dip, and a resonance within a step of the spectrum of it, its Q a ring's
        let deepest = (0..through.len())
            .min_by(|&a, &b| through[a].total_cmp(&through[b]))
            .unwrap();
        assert!(through[deepest] < 0.5, "{through:?}");
        let near = found
            .iter()
            .find(|r| (r.wavelength_um - wavelengths[deepest]).abs() < 2.5e-3)
            .unwrap_or_else(|| panic!("{found:?} against a dip at {}", wavelengths[deepest]));
        assert!(
            near.q > 100.0 && near.q < 1e4 && near.error < 1e-5,
            "{near:?}"
        );
    }
}

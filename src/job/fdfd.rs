//! The `"fdfd"` job: a device on one layer, seen from above, by 2D FDFD with ports.
//!
//! The 2D problem is the layer's plane. Each point's permittivity is its effective index
//! squared: the fundamental mode of the three-layer slab it stands on (the layer's material at
//! that point between the materials just below and above), solved exactly by
//! [`crate::mode::slab`], or the material's own index where no slab mode guides. This is the
//! effective index method, so the results are 2D estimates, not a device's 3D performance. The
//! slab's TE mode has E in the plane, so `polarization = "te"` is H along z in the 2D problem,
//! and `"tm"` is E along z.

use num_complex::Complex64 as c64;
use serde::Deserialize;

use super::{
    CircleSpec, Event, RectSpec, RingSpec, check_finite, check_materials, check_step, check_sweep,
    check_window, draw, named_stack, scene, task_error,
};
use crate::fdfd::{Boundaries, Direction, Grid, Polarization, Port, Side, Solver2d};
use crate::geometry::Point;
use crate::mode::slab::Slab;
use crate::raster::Raster;
use crate::run::{Job, Run, Stop};
use crate::stack::Structure;
use crate::units::{Length, Wavelength, refractive_index};
use crate::{Error, Result};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PortSpec {
    /// Where the port's column is, µm.
    x_um: f64,
    /// `"left"` (light goes in towards +x) or `"right"`.
    side: String,
    /// The port's window along y, µm: one guide of several; the whole column by default.
    y_um: Option<[f64; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WavelengthSweep {
    #[allow(dead_code)] // only the wavelength is swept; checked below
    parameter: String,
    from: f64,
    to: f64,
    points: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FdfdTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    /// `"te"` (the default) or `"tm"`: the slab mode's polarization.
    polarization: Option<String>,
    x_um: [f64; 2],
    y_um: [f64; 2],
    step_nm: f64,
    /// PML cells on each side, inside the window: 20 by default.
    pml_cells: Option<usize>,
    /// The field is recorded at the swept wavelength nearest this (µm): the first by default.
    field_um: Option<f64>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    #[serde(default)]
    port: Vec<PortSpec>,
    sweep: Option<WavelengthSweep>,
}

impl FdfdTask {
    /// What [`check`] and the run both refuse before solving anything: the window, the step,
    /// the ports' places and the sweep.
    fn validate(&self) -> Result<()> {
        check_window("x_um", self.x_um)?;
        check_window("y_um", self.y_um)?;
        check_step(self.step_nm)?;
        check_finite("field_um", self.field_um)?;
        for p in &self.port {
            check_finite("a port's x_um", Some(p.x_um))?;
            if let Some(y) = p.y_um {
                check_window("a port's y_um", y)?;
            }
        }
        if let Some(sw) = &self.sweep {
            if sw.parameter != "wavelength" {
                return Err(task_error(format!(
                    "an fdfd job sweeps the wavelength, not \"{}\"",
                    sw.parameter
                )));
            }
            check_sweep(sw.from, sw.to, sw.points)?;
            Wavelength::um(sw.from)?;
            Wavelength::um(sw.to)?;
        }
        Ok(())
    }
}

/// The effective index of the slab at `p`: the layer's material there between what lies just
/// below and above it, or the material's own index where the slab guides nothing.
fn effective_index(
    s: &Structure,
    p: Point,
    (bottom, top): (Length, Length),
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<f64> {
    let n = |z: Length| -> Result<f64> {
        Ok(refractive_index(s.material_at(p, z).permittivity(wavelength)?).re)
    };
    let tiny = Length::nm(1e-3);
    let (below, core, above) = (n(bottom - tiny)?, n((bottom + top) * 0.5)?, n(top + tiny)?);
    if core <= below.max(above) {
        return Ok(core);
    }
    let slab = Slab::new(below, core, above, top - bottom)?;
    Ok(slab
        .modes(kind, wavelength)
        .first()
        .map_or(core, crate::mode::slab::SlabMode::effective_index))
}

/// The in-plane permittivity at `wavelength` on `grid`. A point's column is fixed by which
/// layers have a shape over it; the few distinct columns are found at the cells' centres and each
/// one's effective index solved once.
fn plane(
    s: &Structure,
    grid: &Grid,
    layer: (Length, Length),
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<impl Fn(f64, f64) -> c64 + use<>> {
    let s = s.clone();
    let names: Vec<String> = s.stack().layers().iter().map(|l| l.name.clone()).collect();
    let key = move |s: &Structure, p: Point| -> Vec<bool> {
        names
            .iter()
            .map(|n| s.shapes(n).iter().any(|shape| shape.contains(p)))
            .collect()
    };
    let mut table: Vec<(Vec<bool>, f64)> = Vec::new();
    for j in 0..grid.ny {
        for i in 0..grid.nx {
            let p = Point::um(grid.x(i), grid.y(j));
            let k = key(&s, p);
            if !table.iter().any(|(t, _)| *t == k) {
                let n = effective_index(&s, p, layer, kind, wavelength)?;
                table.push((k, n));
            }
        }
    }
    Ok(move |x: f64, y: f64| {
        let k = key(&s, Point::um(x, y));
        // a column only a sub-cell sample meets takes the first one's index
        let n = table.iter().find(|(t, _)| *t == k).unwrap_or(&table[0]).1;
        c64::new(n * n, 0.0)
    })
}

/// The scene of an `"fdfd"` job: its window, 1 µm below and above its layer.
fn scene_of(task: &FdfdTask, s: &Structure) -> Result<Event> {
    let (_, bottom, top) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    scene(
        s,
        task.x_um,
        task.y_um,
        [bottom.to_um() - 1.0, top.to_um() + 1.0],
        Wavelength::um(task.wavelength_um)?,
    )
}

/// [`super::preview`] for an `"fdfd"` job.
pub(super) fn preview(job: &Job) -> Result<Event> {
    let task: FdfdTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let s = draw(
        named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?,
        &task.rect,
        &task.circle,
        &task.ring,
    )?;
    scene_of(&task, &s)
}

/// [`super::check`] for an `"fdfd"` job.
pub(super) fn check(job: &Job) -> Result<()> {
    let task: FdfdTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    task.validate()?;
    let s = draw(
        named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?,
        &task.rect,
        &task.circle,
        &task.ring,
    )?;
    s.stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    match task.polarization.as_deref().unwrap_or("te") {
        "te" | "tm" => {}
        other => {
            return Err(task_error(format!(
                "unknown polarization \"{other}\": te or tm"
            )));
        }
    }
    if task.port.is_empty() {
        return Err(task_error("an fdfd job needs at least one [[task.port]]"));
    }
    let mut wavelengths = vec![task.wavelength_um];
    if let Some(sw) = &task.sweep {
        wavelengths.extend([sw.from, sw.to]);
    }
    check_materials(s.stack(), &wavelengths)
}

/// An `"fdfd"` job's device, drawn and gridded, ready to solve at any wavelength.
struct Device {
    task: FdfdTask,
    structure: Structure,
    /// The layer's bottom and top.
    layer: (Length, Length),
    polarization: Polarization,
    kind: crate::mode::Polarization,
    field_name: &'static str,
    grid: Grid,
    boundaries: Boundaries,
}

impl Device {
    /// The device `job` describes, checked: the task, the structure, the polarization and the
    /// ports, and the grid of cells of about `step_nm` filling the window, PMLs inside it.
    fn new(job: &Job) -> Result<Device> {
        let task: FdfdTask = job
            .task()
            .clone()
            .try_into()
            .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
        task.validate()?;
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
        let (polarization, kind, field_name) = match task.polarization.as_deref().unwrap_or("te") {
            "te" => (Polarization::Hz, crate::mode::Polarization::Te, "|H_z|^2"),
            "tm" => (Polarization::Ez, crate::mode::Polarization::Tm, "|E_z|^2"),
            other => {
                return Err(task_error(format!(
                    "unknown polarization \"{other}\": te or tm"
                )));
            }
        };
        if task.port.is_empty() {
            return Err(task_error("an fdfd job needs at least one [[task.port]]"));
        }
        let h = task.step_nm / 1000.0;
        // (the window and the step are validated above)
        let (wx, wy) = (task.x_um[1] - task.x_um[0], task.y_um[1] - task.y_um[0]);
        let (nx, ny) = (
            (wx / h).round().max(1.0) as usize,
            (wy / h).round().max(1.0) as usize,
        );
        let grid = Grid {
            nx,
            ny,
            dx: wx / nx as f64,
            dy: wy / ny as f64,
            x0: task.x_um[0],
            y0: task.y_um[0],
        };
        let boundaries = Boundaries::pml(task.pml_cells.unwrap_or(20));
        Ok(Device {
            task,
            structure,
            layer: (bottom, top),
            polarization,
            kind,
            field_name,
            grid,
            boundaries,
        })
    }

    /// The job's wavelengths, µm: its sweep's, or its one wavelength.
    fn wavelengths(&self) -> Vec<f64> {
        match &self.task.sweep {
            None => vec![self.task.wavelength_um],
            // (validated: the wavelength, from 2 points between distinct ends)
            Some(sw) => (0..sw.points)
                .map(|k| sw.from + (sw.to - sw.from) * k as f64 / (sw.points - 1) as f64)
                .collect(),
        }
    }

    /// The ports' names, in the job's order.
    fn port_names(&self) -> Vec<String> {
        self.task
            .port
            .iter()
            .enumerate()
            .map(|(k, p)| format!("{} ({}, x = {} um)", k + 1, p.side, p.x_um))
            .collect()
    }

    /// The problem at `wavelength`, its ports and its S-matrix; `first`, the problem at an
    /// earlier wavelength, lends it its analysed sparsity.
    fn solve(
        &self,
        wavelength: Wavelength,
        first: Option<&Solver2d>,
    ) -> Result<(Solver2d, Vec<Port>, Vec<Vec<c64>>)> {
        let eps = plane(
            &self.structure,
            &self.grid,
            self.layer,
            self.kind,
            wavelength,
        )?;
        let solver = match first {
            None => Solver2d::new(
                self.grid,
                self.polarization,
                wavelength,
                eps,
                self.boundaries,
            )?,
            Some(first) => first.reuse(wavelength, eps)?,
        };
        let ports = self
            .task
            .port
            .iter()
            .map(|p| port(&solver, p))
            .collect::<Result<Vec<Port>>>()?;
        let s = solver.s_matrix(&ports)?;
        Ok((solver, ports, s))
    }
}

/// The S-parameters of an `"fdfd"` job's device ([`fdfd_s_parameters`]): one S-matrix per
/// wavelength, in the conventions of [`crate::fdfd`] (power-normalized, `s[q][p]` from port p
/// into port q, the phases referred to the ports' columns).
#[derive(Clone, Debug, PartialEq)]
pub struct FdfdSParameters {
    /// The ports' names, in the job's order, e.g. `"1 (left, x = -3.5 um)"`.
    pub ports: Vec<String>,
    /// The wavelengths, µm.
    pub wavelengths_um: Vec<f64>,
    /// The S-matrix at each wavelength, `s[k][q][p]`.
    pub s: Vec<Vec<Vec<c64>>>,
    /// Each port mode's effective index at each wavelength, `effective_indices[k][p]`.
    pub effective_indices: Vec<Vec<f64>>,
    /// The grid's cell, dx × dy, µm.
    pub cell_um: (f64, f64),
    /// The slab mode's polarization the plane's indices come from (the job's `polarization`):
    /// TE is H along z in the plane, TM is E along z.
    pub polarization: crate::mode::Polarization,
}

/// Solves an `"fdfd"` job's device as [`execute`](super::execute) runs it, without recording
/// a run: its S-matrix at `wavelengths_um` (µm), or at the job's own wavelengths (its sweep, or
/// its one wavelength) when `None`. The sparsity is analysed once.
///
/// # Errors
///
/// The errors [`check`](super::check) finds, a wavelength that isn't positive and finite, and
/// those of the solve (a port inside the PML, a material without data at a wavelength).
pub fn fdfd_s_parameters(job: &Job, wavelengths_um: Option<&[f64]>) -> Result<FdfdSParameters> {
    let device = Device::new(job)?;
    let wavelengths = wavelengths_um.map_or_else(|| device.wavelengths(), <[f64]>::to_vec);
    let mut first: Option<Solver2d> = None;
    let mut s = Vec::with_capacity(wavelengths.len());
    let mut effective_indices = Vec::with_capacity(wavelengths.len());
    for &w in &wavelengths {
        let (solver, ports, sm) = device.solve(Wavelength::um(w)?, first.as_ref())?;
        s.push(sm);
        effective_indices.push(ports.iter().map(|p| p.mode.effective_index().re).collect());
        if first.is_none() {
            first = Some(solver);
        }
    }
    Ok(FdfdSParameters {
        ports: device.port_names(),
        wavelengths_um: wavelengths,
        s,
        effective_indices,
        cell_um: (device.grid.dx, device.grid.dy),
        polarization: device.kind,
    })
}

/// The pixels a sweep's fields may take in all, over its points.
const SWEEP_PIXELS: usize = 5_000_000;

/// The side, in cells, of the blocks a sweep's fields are averaged over: 2, or the smallest
/// that keeps the `points` pictures of an `nx` × `ny` grid within [`SWEEP_PIXELS`].
pub(super) fn sweep_block(nx: usize, ny: usize, points: usize) -> usize {
    let most = nx.min(ny).max(1);
    (2..most)
        .find(|b| (nx / b) * (ny / b) * points <= SWEEP_PIXELS)
        .unwrap_or(most)
        .min(most)
}

/// A sweep point's picture of `values` (|field|² on `grid`, row by row): their means over
/// blocks of `block` × `block` cells, scaled to a peak of 1 and rounded to three decimals. The
/// cells left over at the right and the top, fewer than a block, are left out.
fn coarse(values: &[f64], grid: &Grid, block: usize) -> Raster {
    let (nx, ny) = (grid.nx / block, grid.ny / block);
    let mut means: Vec<f64> = (0..ny)
        .flat_map(|j| (0..nx).map(move |i| (i, j)))
        .map(|(i, j)| {
            let sum: f64 = (0..block)
                .flat_map(|b| (0..block).map(move |a| (a, b)))
                .map(|(a, b)| values[(j * block + b) * grid.nx + i * block + a])
                .sum();
            sum / (block * block) as f64
        })
        .collect();
    let peak = means
        .iter()
        .copied()
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    for v in &mut means {
        *v = (*v / peak * 1000.0).round() / 1000.0;
    }
    Raster {
        nx,
        ny,
        x0: grid.x0,
        x1: grid.x0 + (nx * block) as f64 * grid.dx,
        y0: grid.y0,
        y1: grid.y0 + (ny * block) as f64 * grid.dy,
        values: means.iter().map(|&v| v as f32).collect(),
    }
}

pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let device = Device::new(job)?;
    let task = &device.task;
    // the 3D view draws the 2D field on the layer's top face
    let z_face = device.layer.1.to_um();
    run.record(&scene_of(task, &device.structure)?)?;
    let Grid { nx, ny, .. } = device.grid;
    let wavelengths = device.wavelengths();
    let names = device.port_names();
    // the step whose field is recorded: the wavelength nearest field_um, else the first
    let field_step = task.field_um.map_or(0, |f| {
        (0..wavelengths.len())
            .min_by(|&a, &b| {
                (wavelengths[a] - f)
                    .abs()
                    .total_cmp(&(wavelengths[b] - f).abs())
            })
            .unwrap_or(0)
    });
    let label = format!(
        "{} from port 1, 2D by the effective index method",
        device.field_name
    );
    // a sweep's points each record their field, on blocks of cells
    run.record(&Event::Solver {
        module: "fdfd".into(),
        cells: [nx, ny],
        step_um: task.step_nm / 1000.0,
        unknowns: nx * ny,
        details: vec![
            [
                "field".into(),
                match device.kind {
                    crate::mode::Polarization::Te => {
                        "H along z: the slab's TE mode, E in the plane"
                    }
                    crate::mode::Polarization::Tm => "E along z: the slab's TM mode",
                }
                .into(),
            ],
            [
                "permittivity".into(),
                "the effective index method: each point's slab mode index, squared".into(),
            ],
            [
                "PML".into(),
                format!(
                    "{} cells on each side, inside the window",
                    task.pml_cells.unwrap_or(20)
                ),
            ],
            ["ports".into(), task.port.len().to_string()],
            [
                "linear solve".into(),
                "sparse LU with one step of iterative refinement; a sweep analyses the \
                 sparsity once"
                    .into(),
            ],
        ],
    })?;
    let pml_cells = task.pml_cells.unwrap_or(20);
    let block = match &task.sweep {
        Some(sw) => {
            run.record(&Event::Sweep {
                parameter: "wavelength".into(),
                from: sw.from,
                to: sw.to,
                points: sw.points,
            })?;
            Some(sweep_block(nx, ny, sw.points))
        }
        None => None,
    };
    let mut solver: Option<Solver2d> = None;
    for (step, &w) in wavelengths.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let value = w;
        let w = Wavelength::um(w)?;
        let (current, ports, sm) = device.solve(w, solver.as_ref())?;
        let mut residual = None;
        // how far S is from reciprocal: the largest |S_qp − S_pq|
        let reciprocity = (0..sm.len())
            .flat_map(|q| (0..q).map(move |p| (q, p)))
            .map(|(q, p)| (sm[q][p] - sm[p][q]).norm())
            .fold(None, |most: Option<f64>, d| {
                Some(most.map_or(d, |m| m.max(d)))
            });
        run.record(&Event::SParameters {
            wavelength_um: w.to_um(),
            ports: names.clone(),
            effective_indices: ports.iter().map(|p| p.mode.effective_index().re).collect(),
            s: sm
                .iter()
                .map(|row| row.iter().map(|v| [v.re, v.im]).collect())
                .collect(),
        })?;
        // the field from the first port: in full at the wavelength chosen, and coarser at each
        // point of a sweep
        if step == field_step || block.is_some() {
            let first = &ports[0];
            let direction = match first.side {
                Side::Left => Direction::Forward,
                Side::Right => Direction::Backward,
            };
            // launched from the window's end of port 1's guide, so the picture shows the wave
            // from where the guide comes in (S is still referred to the port's own column)
            let launched = launch(&current, &task.port[0], &first.mode, pml_cells);
            let source = current.mode_source(launched.as_ref().unwrap_or(&first.mode), direction);
            let field = current.solve_system(&source)?;
            residual = Some(current.residual(&field, &source)?);
            let values: Vec<f64> = (0..ny)
                .flat_map(|j| (0..nx).map(move |i| (i, j)))
                .map(|(i, j)| field.at(i, j).norm_sqr())
                .collect();
            if step == field_step {
                let peak = values
                    .iter()
                    .copied()
                    .fold(0.0, f64::max)
                    .max(f64::MIN_POSITIVE);
                run.record(&Event::Field {
                    label: label.clone(),
                    wavelength_um: w.to_um(),
                    z_um: z_face,
                    intensity: Raster {
                        nx,
                        ny,
                        x0: task.x_um[0],
                        x1: task.x_um[1],
                        y0: task.y_um[0],
                        y1: task.y_um[1],
                        values: values.iter().map(|v| (v / peak) as f32).collect(),
                    },
                })?;
            }
            if let Some(block) = block {
                run.record(&Event::SweepField {
                    point: step,
                    value,
                    label: label.clone(),
                    wavelength_um: w.to_um(),
                    z_um: z_face,
                    intensity: coarse(&values, &device.grid, block),
                })?;
            }
        }
        let point = block.map(|_| step);
        for (measure, error) in [("linear residual", residual), ("reciprocity", reciprocity)] {
            if let Some(error) = error {
                run.record(&Event::SolveError {
                    point,
                    value,
                    measure: measure.into(),
                    error,
                })?;
            }
        }
        if solver.is_none() {
            solver = Some(current);
        }
    }
    Ok(())
}

/// The mode of `spec`'s guide at the column nearest the window's end on the port's side, two
/// cells clear of the PML: where the recorded field is launched from, so that it shows the wave
/// along the whole guide and not only from the port on. `None` when the guide there isn't the
/// port's (no mode, or one of another effective index): the field is then launched at the port.
fn launch(
    solver: &Solver2d,
    spec: &PortSpec,
    at_port: &crate::fdfd::PortMode,
    pml_cells: usize,
) -> Option<crate::fdfd::PortMode> {
    let g = solver.grid();
    let column = match spec.side.as_str() {
        "left" => pml_cells + 2,
        _ => g.nx.checked_sub(pml_cells + 3)?,
    };
    let row = |y: f64| (((y - g.y0) / g.dy).round().max(0.0) as usize).min(g.ny);
    let rows = spec.y_um.map_or(0..g.ny, |[a, b]| row(a)..row(b));
    let mode = solver
        .port_modes_within(column, rows, 1)
        .ok()?
        .into_iter()
        .next()?;
    let same = (mode.effective_index() - at_port.effective_index()).norm() < 1e-8;
    same.then_some(mode)
}

/// The port a spec describes: its column, its window, its fundamental mode.
fn port(solver: &Solver2d, spec: &PortSpec) -> Result<Port> {
    let g = solver.grid();
    let column = ((spec.x_um - g.x0) / g.dx).floor();
    if !(column >= 0.0 && column < g.nx as f64) {
        return Err(task_error(format!(
            "the port at x = {} um is outside the window",
            spec.x_um
        )));
    }
    let side = match spec.side.as_str() {
        "left" => Side::Left,
        "right" => Side::Right,
        other => {
            return Err(task_error(format!(
                "unknown port side \"{other}\": left or right"
            )));
        }
    };
    let row = |y: f64| (((y - g.y0) / g.dy).round().max(0.0) as usize).min(g.ny);
    let rows = spec.y_um.map_or(0..g.ny, |[a, b]| row(a)..row(b));
    let mut modes = solver
        .port_modes_within(column as usize, rows, 1)
        .map_err(|e| match e {
            Error::InvalidValue { reason, .. } => {
                task_error(format!("the port at x = {} um: {reason}", spec.x_um))
            }
            other => other,
        })?;
    Ok(Port {
        mode: modes.remove(0),
        side,
    })
}

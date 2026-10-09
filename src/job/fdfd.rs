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
    CircleSpec, Event, RectSpec, RingSpec, check_cells, check_finite, check_materials, check_step,
    check_sweep, check_window, draw, named_stack, scene, task_error,
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
        check_cells("x_um", self.x_um, self.step_nm)?;
        check_cells("y_um", self.y_um, self.step_nm)?;
        check_finite("field_um", self.field_um)?;
        if self.pml_cells == Some(0) {
            return Err(task_error(
                "pml_cells must be at least 1: without PMLs the window's walls send everything \
                 back, and the S-parameters are a closed box's",
            ));
        }
        for (k, p) in self.port.iter().enumerate() {
            check_finite("a port's x_um", Some(p.x_um))?;
            if let Some(y) = p.y_um {
                check_window("a port's y_um", y)?;
                // (a hair's slack for a bound typed as the window's own)
                let slack = 1e-9;
                if y[0] < self.y_um[0] - slack || y[1] > self.y_um[1] + slack {
                    return Err(task_error(format!(
                        "the port at x = {} um: its y_um, {} to {}, must lie inside the window's, \
                         {} to {}",
                        p.x_um, y[0], y[1], self.y_um[0], self.y_um[1]
                    )));
                }
            }
            if let Some(same) = self.port[..k]
                .iter()
                .position(|q| q.x_um == p.x_um && q.side == p.side && q.y_um == p.y_um)
            {
                return Err(task_error(format!(
                    "ports {} and {} are the same (x = {} um, {}): each port needs its own place",
                    same + 1,
                    k + 1,
                    p.x_um,
                    p.side
                )));
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
    layer: (Length, Length),
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<f64> {
    slab_index(|z| s.material_at(p, z), layer, kind, wavelength)
}

/// [`effective_index`] of the column whose material at each height z is `material(z)`: the
/// layer's, between what lies just below and above it.
pub(super) fn slab_index<'a>(
    material: impl Fn(Length) -> &'a crate::material::Material,
    (bottom, top): (Length, Length),
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<f64> {
    let n = |z: Length| -> Result<f64> {
        Ok(refractive_index(material(z).permittivity(wavelength)?).re)
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

/// The grid of cells of about `step_nm` filling the task's window (validated before).
fn grid_of(task: &FdfdTask) -> Grid {
    let h = task.step_nm / 1000.0;
    let (wx, wy) = (task.x_um[1] - task.x_um[0], task.y_um[1] - task.y_um[0]);
    let (nx, ny) = (
        (wx / h).round().max(1.0) as usize,
        (wy / h).round().max(1.0) as usize,
    );
    Grid {
        nx,
        ny,
        dx: wx / nx as f64,
        dy: wy / ny as f64,
        x0: task.x_um[0],
        y0: task.y_um[0],
    }
}

/// The part of the grid the run's pictures show, and how many cells it leaves out on each side
/// along x and y: the window without its PMLs and the two cells beside them, where the field
/// is launched. The PMLs are where the solver absorbs the light, not part of the device, so a
/// guide drawn into them would look as if the light started late and ended early. A grid too
/// small to leave anything is shown whole.
fn pictured(grid: &Grid, pml_cells: usize) -> (Grid, [usize; 2]) {
    let margin = pml_cells + 2;
    let mx = if grid.nx > 2 * margin { margin } else { 0 };
    let my = if grid.ny > 2 * margin { margin } else { 0 };
    (
        Grid {
            nx: grid.nx - 2 * mx,
            ny: grid.ny - 2 * my,
            x0: grid.x0 + mx as f64 * grid.dx,
            y0: grid.y0 + my as f64 * grid.dy,
            ..*grid
        },
        [mx, my],
    )
}

/// The scene of an `"fdfd"` job: the part of its window the pictures show ([`pictured`]),
/// 1 µm below and above its layer.
fn scene_of(task: &FdfdTask, s: &Structure) -> Result<Event> {
    let (inner, _) = pictured(&grid_of(task), task.pml_cells.unwrap_or(20));
    let (_, bottom, top) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    scene(
        s,
        [inner.x0, inner.x0 + inner.nx as f64 * inner.dx],
        [inner.y0, inner.y0 + inner.ny as f64 * inner.dy],
        [bottom.to_um() - 1.0, top.to_um() + 1.0],
        Wavelength::um(task.wavelength_um)?,
    )
}

/// [`super::preview`] for an `"fdfd"` job.
pub(super) fn preview(job: &Job) -> Result<Event> {
    let task: FdfdTask = super::params::task(job)?;
    let s = draw(
        named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?,
        &task.rect,
        &task.circle,
        &task.ring,
    )?;
    scene_of(&task, &s)
}

/// The backend a choice resolves to for the device, as the run's record names it: its name
/// and version; and if the job named none, that `auto` chose it and why (what the solver
/// decides too, from the same measurements: [`crate::backend::auto::decide`]).
fn direct_solver(device: &Device) -> Result<String> {
    use crate::backend::{Choice, direct};
    if device.direct != Choice::Auto {
        let c = direct(&device.direct)?.capabilities();
        return Ok(format!("{} {}", c.name, c.version));
    }
    let problem = crate::fdfd::problem_2d(device.grid, &device.boundaries);
    let decision = crate::backend::auto::decide(problem);
    let c = Choice::parse(&decision.backend)
        .and_then(|c| direct(&c))
        .or_else(|_| direct(&Choice::Photonoxide))?
        .capabilities();
    Ok(format!(
        "{} {} (auto: {})",
        c.name, c.version, decision.reason
    ))
}

/// [`super::check`] for an `"fdfd"` job.
pub(super) fn check(job: &Job) -> Result<()> {
    // the task, the structure, the layer, the polarization, that there are ports, and the grid
    let device = Device::new(job)?;
    let task = &device.task;
    // what the solver would refuse before solving anything: PMLs that leave no room, and each
    // port's place
    crate::fdfd::check(device.grid, &device.boundaries)?;
    for p in &task.port {
        place(&device.grid, &device.boundaries, p)?;
    }
    let mut wavelengths = vec![task.wavelength_um];
    if let Some(sw) = &task.sweep {
        wavelengths.extend([sw.from, sw.to]);
    }
    check_materials(device.structure.stack(), &wavelengths)
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
    /// The direct solver the job asks for.
    direct: crate::backend::Choice,
    /// The wavelength the permittivity is taken at, if not each solve's own: an `"fdtd"` job's
    /// non-dispersive plane, to compare it with.
    frozen: Option<Wavelength>,
}

impl Device {
    /// The device `job` describes, checked: the task, the structure, the polarization and the
    /// ports, and the grid of cells of about `step_nm` filling the window, PMLs inside it.
    fn new(job: &Job) -> Result<Device> {
        let task: FdfdTask = super::params::task(job)?;
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
        // (the window and the step are validated above)
        let grid = grid_of(&task);
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
            direct: job.direct().clone(),
            frozen: None,
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
            self.frozen.unwrap_or(wavelength),
        )?;
        let solver = match first {
            None => Solver2d::new_on(
                self.grid,
                self.polarization,
                wavelength,
                eps,
                self.boundaries,
                &self.direct,
            )?,
            Some(first) => first.reuse(wavelength, eps)?,
        };
        let ports = self
            .task
            .port
            .iter()
            .map(|p| port(&solver, &self.boundaries, p))
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
/// its one wavelength) when `None`. The sparsity is analysed once, at the first wavelength; the
/// others are solved side by side on rayon's threads, and come out in order, the same whatever
/// the threads.
///
/// # Errors
///
/// The errors [`check`](super::check) finds, a wavelength that isn't positive and finite, and
/// those of the solve (a port inside the PML, a material without data at a wavelength).
pub fn fdfd_s_parameters(job: &Job, wavelengths_um: Option<&[f64]>) -> Result<FdfdSParameters> {
    s_parameters(Device::new(job)?, wavelengths_um)
}

/// [`fdfd_s_parameters`] with the permittivity taken at `permittivity_um` whatever the
/// wavelength: the non-dispersive plane an `"fdtd"` job steps in time, its carrier's.
#[cfg(test)]
pub(super) fn fdfd_s_parameters_frozen(
    job: &Job,
    wavelengths_um: &[f64],
    permittivity_um: f64,
) -> Result<FdfdSParameters> {
    let mut device = Device::new(job)?;
    device.frozen = Some(Wavelength::um(permittivity_um)?);
    s_parameters(device, Some(wavelengths_um))
}

fn s_parameters(device: Device, wavelengths_um: Option<&[f64]>) -> Result<FdfdSParameters> {
    use rayon::prelude::*;
    let wavelengths = wavelengths_um.map_or_else(|| device.wavelengths(), <[f64]>::to_vec);
    let mut s = Vec::with_capacity(wavelengths.len());
    let mut effective_indices = Vec::with_capacity(wavelengths.len());
    let indices = |ports: &[Port]| ports.iter().map(|p| p.mode.effective_index().re).collect();
    if let Some((&w, rest)) = wavelengths.split_first() {
        let (first, ports, sm) = device.solve(Wavelength::um(w)?, None)?;
        s.push(sm);
        effective_indices.push(indices(&ports));
        // as the run's sweep: about 5 kB a cell for each point in flight
        let batch = super::in_flight((device.grid.nx * device.grid.ny) as f64 * 5e3);
        for ws in rest.chunks(batch) {
            let done: Vec<Result<_>> = ws
                .par_iter()
                .map(|&w| {
                    let (_, ports, sm) = device.solve(Wavelength::um(w)?, Some(&first))?;
                    Ok((sm, indices(&ports)))
                })
                .collect();
            for point in done {
                let (sm, n) = point?;
                s.push(sm);
                effective_indices.push(n);
            }
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
            ["direct solver".into(), direct_solver(&device)?],
        ],
    })?;
    let pml_cells = task.pml_cells.unwrap_or(20);
    // the pictures show the window without its PMLs
    let (inner, [mx, my]) = pictured(&device.grid, pml_cells);
    let block = match &task.sweep {
        Some(sw) => {
            run.record(&Event::Sweep {
                parameter: "wavelength".into(),
                from: sw.from,
                to: sw.to,
                points: sw.points,
            })?;
            Some(sweep_block(inner.nx, inner.ny, sw.points))
        }
        None => None,
    };
    // one point's events, and its solver: the first point's is reused by the others, whose
    // sparsity it has analysed
    let solve_point = |step: usize, reuse: Option<&Solver2d>| -> Result<(Vec<Event>, Solver2d)> {
        let mut events = Vec::new();
        let w = wavelengths[step];
        let value = w;
        let w = Wavelength::um(w)?;
        let (current, ports, sm) = device.solve(w, reuse)?;
        let mut residual = None;
        // how far S is from reciprocal: the largest |S_qp − S_pq|
        let reciprocity = (0..sm.len())
            .flat_map(|q| (0..q).map(move |p| (q, p)))
            .map(|(q, p)| (sm[q][p] - sm[p][q]).norm())
            .fold(None, |most: Option<f64>, d| {
                Some(most.map_or(d, |m| m.max(d)))
            });
        events.push(Event::SParameters {
            wavelength_um: w.to_um(),
            ports: names.clone(),
            effective_indices: ports.iter().map(|p| p.mode.effective_index().re).collect(),
            s: sm
                .iter()
                .map(|row| row.iter().map(|v| [v.re, v.im]).collect())
                .collect(),
        });
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
            let values: Vec<f64> = (0..inner.ny)
                .flat_map(|j| (0..inner.nx).map(move |i| (i, j)))
                .map(|(i, j)| field.at(mx + i, my + j).norm_sqr())
                .collect();
            if step == field_step {
                let peak = values
                    .iter()
                    .copied()
                    .fold(0.0, f64::max)
                    .max(f64::MIN_POSITIVE);
                events.push(Event::Field {
                    label: label.clone(),
                    wavelength_um: w.to_um(),
                    z_um: z_face,
                    intensity: Raster {
                        nx: inner.nx,
                        ny: inner.ny,
                        x0: inner.x0,
                        x1: inner.x0 + inner.nx as f64 * inner.dx,
                        y0: inner.y0,
                        y1: inner.y0 + inner.ny as f64 * inner.dy,
                        values: values.iter().map(|v| (v / peak) as f32).collect(),
                    },
                });
            }
            if let Some(block) = block {
                events.push(Event::SweepField {
                    point: step,
                    value,
                    label: label.clone(),
                    wavelength_um: w.to_um(),
                    z_um: z_face,
                    intensity: coarse(&values, &inner, block),
                });
            }
        }
        let point = block.map(|_| step);
        for (measure, error) in [("linear residual", residual), ("reciprocity", reciprocity)] {
            if let Some(error) = error {
                events.push(Event::SolveError {
                    point,
                    value,
                    measure: measure.into(),
                    error,
                });
            }
        }
        Ok((events, current))
    };
    if wavelengths.is_empty() || stop.reason().is_some() {
        return Ok(());
    }
    let (events, first) = solve_point(0, None)?;
    for e in &events {
        run.record(e)?;
    }
    // the others side by side: a point takes about 5 kB a cell (measured: 690 MB at 149 600
    // cells, `photonoxide bench`'s fdfd2d/slab-lu)
    let batch = super::in_flight((nx * ny) as f64 * 5e3);
    super::sweep_points(run, stop, 1..wavelengths.len(), batch, |k| {
        Ok(Some(solve_point(k, Some(&first))?.0))
    })
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

/// Where a spec puts its port on the grid: its column, its rows and its side. Refused as the
/// solve would refuse it ([`Solver2d::port_modes_within`]), in the same words, before any
/// solve: a column outside the window or less than two cells clear of the PMLs, an unknown
/// side, a window of fewer than 3 rows.
fn place(
    g: &Grid,
    boundaries: &Boundaries,
    spec: &PortSpec,
) -> Result<(usize, std::ops::Range<usize>, Side)> {
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
    let column = column as usize;
    let row = |y: f64| (((y - g.y0) / g.dy).round().max(0.0) as usize).min(g.ny);
    let rows = spec.y_um.map_or(0..g.ny, |[a, b]| row(a)..row(b));
    let refused =
        |reason: String| task_error(format!("the port at x = {} um: {reason}", spec.x_um));
    if rows.len() < 3 {
        return Err(refused(match spec.y_um {
            // in the job's own terms: its y_um against the window's
            Some([a, b]) => format!(
                "its y_um [{a}, {b}] covers {} {} of the window (y from {} to {} um, {} rows); \
                 a port needs 3 rows or more",
                rows.len(),
                if rows.len() == 1 { "row" } else { "rows" },
                // to the picometre, without the grid's rounding
                (g.y0 * 1e6).round() / 1e6,
                ((g.y0 + g.dy * g.ny as f64) * 1e6).round() / 1e6,
                g.ny
            ),
            None => format!(
                "the window, rows {} to {}, must have 3 rows or more on the grid's {}",
                rows.start, rows.end, g.ny
            ),
        }));
    }
    let (low, high) = boundaries.x.pml();
    if !(column >= low + 2 && column + 3 + high <= g.nx) {
        return Err(refused(format!(
            "column {column} must be two cells clear of the PMLs ({low} and {high} cells) and \
             the ends of {} columns",
            g.nx
        )));
    }
    Ok((column, rows, side))
}

/// The port a spec describes: its column, its window, its fundamental mode.
fn port(solver: &Solver2d, boundaries: &Boundaries, spec: &PortSpec) -> Result<Port> {
    let (column, rows, side) = place(&solver.grid(), boundaries, spec)?;
    let mut modes = solver
        .port_modes_within(column, rows, 1)
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

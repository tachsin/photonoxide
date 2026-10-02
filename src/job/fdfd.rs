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

use super::{CircleSpec, Event, RectSpec, RingSpec, draw, named_stack, scene, task_error};
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
    Wavelength::um(task.wavelength_um)?;
    Ok(())
}

pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
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
    let (_, bottom, top) = s
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
    // the 3D view draws the 2D field on the layer's top face
    let z_face = top.to_um();
    run.record(&scene_of(&task, &s)?)?;

    // the grid: cells of about step_nm filling the window, PMLs inside it
    let h = task.step_nm / 1000.0;
    let (wx, wy) = (task.x_um[1] - task.x_um[0], task.y_um[1] - task.y_um[0]);
    if !(h > 0.0 && wx > 0.0 && wy > 0.0) {
        return Err(task_error("the window and the step must be positive"));
    }
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
    let wavelengths: Vec<f64> = match &task.sweep {
        None => vec![task.wavelength_um],
        Some(sw) => {
            if sw.parameter != "wavelength" {
                return Err(task_error(format!(
                    "an fdfd job sweeps the wavelength, not \"{}\"",
                    sw.parameter
                )));
            }
            if sw.points < 2 || !(sw.from.is_finite() && sw.to.is_finite()) {
                return Err(task_error("a sweep needs from, to and at least 2 points"));
            }
            (0..sw.points)
                .map(|k| sw.from + (sw.to - sw.from) * k as f64 / (sw.points - 1) as f64)
                .collect()
        }
    };
    let names: Vec<String> = task
        .port
        .iter()
        .enumerate()
        .map(|(k, p)| format!("{} ({}, x = {} um)", k + 1, p.side, p.x_um))
        .collect();
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
    let mut solver: Option<Solver2d> = None;
    for (step, &w) in wavelengths.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let w = Wavelength::um(w)?;
        let eps = plane(&s, &grid, (bottom, top), kind, w)?;
        let current = match &solver {
            None => Solver2d::new(grid, polarization, w, eps, boundaries)?,
            Some(first) => first.reuse(w, eps)?,
        };
        let ports = task
            .port
            .iter()
            .map(|p| port(&current, p))
            .collect::<Result<Vec<Port>>>()?;
        let sm = current.s_matrix(&ports)?;
        run.record(&Event::SParameters {
            wavelength_um: w.to_um(),
            ports: names.clone(),
            effective_indices: ports.iter().map(|p| p.mode.effective_index().re).collect(),
            s: sm
                .iter()
                .map(|row| row.iter().map(|v| [v.re, v.im]).collect())
                .collect(),
        })?;
        // the field from the first port, at the wavelength chosen
        if step == field_step {
            let first = &ports[0];
            let direction = match first.side {
                Side::Left => Direction::Forward,
                Side::Right => Direction::Backward,
            };
            let field = current.solve_system(&current.mode_source(&first.mode, direction))?;
            let values: Vec<f64> = (0..ny)
                .flat_map(|j| (0..nx).map(move |i| (i, j)))
                .map(|(i, j)| field.at(i, j).norm_sqr())
                .collect();
            let peak = values
                .iter()
                .copied()
                .fold(0.0, f64::max)
                .max(f64::MIN_POSITIVE);
            run.record(&Event::Field {
                label: format!("{field_name} from port 1, 2D by the effective index method"),
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
        if solver.is_none() {
            solver = Some(current);
        }
    }
    Ok(())
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

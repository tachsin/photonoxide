//! The `"eme"` job in 3D (`[task.plane]`): each section's cross-section a plane of Yee's grid
//! ([`crate::eme::Eme3d`]). A task alone gives each section as a strip on the plane, its width
//! `core_um` (a taper's `[from, to]`, cut into equal steps, each the strip at its middle) and its
//! height `[task.plane]`'s; beside a `[device]` the sections are cut from the device's geometry
//! along x ([`crate::device::Cut`]), and a task's own drawing (a `stack`, its `layer`, and
//! rectangles, circles, rings, paths and shapes drawn on it, as a structure job's) is cut the same
//! way from `x_um`'s start to its end. The run records the S-parameters between the guided modes
//! at the two ends, as a 2D job's.

use num_complex::Complex64 as c64;
use schemars::JsonSchema;
use serde::Deserialize;

use super::{
    EmeTask, Resolved, Shape, Solved, bloch_point, check_of, interpolated, period_range,
    ports_event, quantity,
};
use crate::Result;
use crate::convergence::{Refined, ToTolerance, Tolerance};
use crate::device::Cut;
use crate::eme::{Eme3d, Plane, PlaneGrid, Pml3d, Scattering, Section3d, Wall, Walls};
use crate::job::{Event, task_error};
use crate::run::{Job, Run, Stop};
use crate::stack::Structure;
use crate::units::{Length, Wavelength};

/// A `[task.plane]`: the sections are 3D, each cross-section a plane of Yee's grid.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PlaneSpec {
    /// The cells' size across (x) and up (y), µm; beside a `[device]` or with a drawing, across
    /// its y and up its z (the device runs along x).
    step_um: [f64; 2],
    /// A task alone: the cells across and up, the window centred on the strips' axis (from the
    /// planes of symmetry when `symmetry` asks).
    cells: Option<[usize; 2]>,
    /// A task alone: the strips' height, µm, centred in the window.
    height_um: Option<f64>,
    /// `"none"` (the default), `"half"` (x ≥ 0: an electric wall at x = 0, the modes whose E_x
    /// is even about it, the TE-like ones) or, for a task alone, `"quarter"` (x ≥ 0 and y ≥ 0:
    /// a magnetic wall at y = 0 too).
    symmetry: Option<String>,
    /// Beside a `[device]` or with a drawing: the cladding kept beside its core's shapes and
    /// below and above its layer, µm; [0.5, 0.5] by default.
    margins_um: Option<[f64; 2]>,
    /// Beside a `[device]` or with a drawing: the longest step of a run of sloped outlines (a
    /// taper's sides, a curve), µm; 0.1 by default.
    along_um: Option<f64>,
    /// Beside a `[device]` or with a drawing: cut at every x where the plane changes (where a
    /// sloped outline crosses one of the points each cell is averaged from), so that each
    /// section holds over its whole length and the cut has no step error, instead of in steps
    /// of `along_um`; false by default.
    exact: Option<bool>,
}

/// The planes of symmetry a 3D job keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Symmetry {
    None,
    Half,
    Quarter,
}

impl PlaneSpec {
    fn symmetry(&self) -> Result<Symmetry> {
        match self.symmetry.as_deref().unwrap_or("none") {
            "none" => Ok(Symmetry::None),
            "half" => Ok(Symmetry::Half),
            "quarter" => Ok(Symmetry::Quarter),
            other => Err(task_error(format!(
                "[task.plane] symmetry is \"none\", \"half\" or \"quarter\", got \"{other}\""
            ))),
        }
    }

    /// A task alone's plane: `cells` of `step_um`, centred on the axis, or from the planes of
    /// symmetry; its walls electric but a quarter's magnetic one at y = 0.
    fn grid(&self) -> Result<PlaneGrid> {
        let [nx, ny] = self
            .cells
            .ok_or_else(|| task_error("[task.plane] needs cells = [across, up]"))?;
        let [dx, dy] = self.step_um;
        let symmetry = self.symmetry()?;
        let x0 = if symmetry == Symmetry::None {
            -0.5 * nx as f64 * dx
        } else {
            0.0
        };
        let y0 = if symmetry == Symmetry::Quarter {
            0.0
        } else {
            -0.5 * ny as f64 * dy
        };
        let grid = PlaneGrid::new(nx, ny, (dx, dy), (x0, y0));
        Ok(if symmetry == Symmetry::Quarter {
            grid.with_walls(Walls {
                south: Wall::Magnetic,
                ..Walls::default()
            })
        } else {
            grid
        })
    }

    /// A `[device]`'s or a drawing's cut.
    fn cut(&self) -> Result<Cut> {
        let [beside, vertical] = self.margins_um.unwrap_or([0.5, 0.5]);
        let mut cut = Cut::new(
            self.step_um[0],
            self.step_um[1],
            self.along_um.unwrap_or(0.1),
        )?
        .with_margins(beside, vertical)?;
        if self.exact() {
            cut = cut.exactly();
        }
        Ok(match self.symmetry()? {
            Symmetry::None => cut,
            _ => cut.mirrored(),
        })
    }

    fn exact(&self) -> bool {
        self.exact.unwrap_or(false)
    }

    /// How a cut's sections are made, for the solver's settings.
    fn cut_details(&self, from: &str, sections: usize) -> String {
        if self.exact() {
            format!(
                "cut from the {from}: {sections} (its guides at the ends), at every x where the \
                 plane changes"
            )
        } else {
            format!(
                "cut from the {from}: {sections} (its guides at the ends), sloped runs in steps \
                 of at most {} um",
                self.along_um.unwrap_or(0.1)
            )
        }
    }
}

/// A task's drawing: its stack with its shapes drawn on it, its core layer, and the cut's ends
/// (µm along x); refused unless the core layer's shapes run on past both ends.
fn drawing(task: &EmeTask) -> Result<(Structure, String, (f64, f64))> {
    let stack = task
        .stack
        .as_ref()
        .ok_or_else(|| task_error("a drawing needs its stack"))?;
    let structure = crate::job::structure::draw_paths(
        crate::job::structure::draw(
            crate::job::structure::named_stack(stack, task.core_nm, task.bottom_oxide_um)?,
            &task.rect,
            &task.circle,
            &task.ring,
        )?,
        &task.path,
        &task.shape,
        &[],
    )?;
    let core = task
        .layer
        .clone()
        .ok_or_else(|| task_error("a drawing needs its layer, the core its plane spans"))?;
    if structure.stack().layer(&core).is_none() {
        return Err(task_error(format!("the stack has no layer {core}")));
    }
    let [a, b] = task
        .x_um
        .ok_or_else(|| task_error("a drawing needs x_um, where it is cut from and to along x"))?;
    if !(a.is_finite() && b.is_finite() && b > a) {
        return Err(task_error(format!(
            "x_um runs from one x to a larger one, got [{a}, {b}]"
        )));
    }
    let (lo, hi) = extent(&structure, &core)
        .ok_or_else(|| task_error(format!("the layer {core} has no shapes to cut")))?;
    if !(lo < a && hi > b) {
        return Err(task_error(format!(
            "the shapes on {core} run from x = {lo} to {hi} um: they must run on past both of \
             x_um's ends, [{a}, {b}], whose cross-sections are the ports' guides"
        )));
    }
    Ok((structure, core, (a, b)))
}

/// The x the shapes on `layer` run over, µm.
fn extent(structure: &Structure, layer: &str) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for shape in structure.shapes(layer) {
        let b = shape.bounds();
        lo = lo.min(b.min.x.to_um());
        hi = hi.max(b.max.x.to_um());
    }
    (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
}

/// What a 3D job refuses before solving anything; `device` when it has a `[device]`.
pub(super) fn validate(task: &EmeTask, plane: &PlaneSpec, device: bool) -> Result<()> {
    if !plane.step_um.iter().all(|h| h.is_finite() && *h > 0.0) {
        return Err(task_error(format!(
            "[task.plane] step_um must be two positive sizes, got {:?}",
            plane.step_um
        )));
    }
    let symmetry = plane.symmetry()?;
    let two_d = [
        ("polarization", task.polarization.is_some()),
        ("walls", task.walls.is_some()),
        ("symmetric", task.symmetric.is_some()),
        ("window_um", task.window_um.is_some()),
    ];
    if let Some((name, _)) = two_d.iter().find(|(_, given)| *given) {
        return Err(task_error(format!(
            "{name} is a 2D job's: a 3D job's modes are full-vector, its window [task.plane]'s \
             cells, its walls electric and its symmetry [task.plane] symmetry"
        )));
    }
    if task.adaptive.is_some() {
        return Err(task_error(
            "[task.adaptive] places a 2D job's steps: a 3D job's tapers take equal steps",
        ));
    }
    if task.pml.as_ref().is_some_and(|p| p.sides.is_some()) {
        return Err(task_error(
            "a 3D job's PML is inside every wall but its planes of symmetry: leave sides out",
        ));
    }
    let drawn = task.drawing_field();
    if device && let Some(name) = drawn {
        return Err(task_error(format!(
            "{name} is a drawing's: a [device] draws its own geometry"
        )));
    }
    if device || drawn.is_some() {
        let what = if device { "a [device]" } else { "a drawing" };
        if !task.section.is_empty() || task.cladding.is_some() || task.core.is_some() {
            return Err(task_error(format!(
                "{what}'s sections are cut from its geometry, its media its stack's: leave \
                 [[task.section]], cladding and core out"
            )));
        }
        if plane.cells.is_some() || plane.height_um.is_some() {
            return Err(task_error(format!(
                "{what}'s plane is cut from its geometry: [task.plane] takes step_um, \
                 margins_um, along_um, exact and symmetry, not cells or height_um"
            )));
        }
        if symmetry == Symmetry::Quarter {
            return Err(task_error(format!(
                "{what} is cut whole or by its half (symmetry = \"half\"), not a quarter"
            )));
        }
        if plane.exact() && plane.along_um.is_some() {
            return Err(task_error(
                "along_um is the steps of a cut that isn't exact: leave it out with exact = true",
            ));
        }
        plane.cut()?;
        if !device {
            drawing(task)?;
        }
        return Ok(());
    }
    if plane.margins_um.is_some() || plane.along_um.is_some() || plane.exact.is_some() {
        return Err(task_error(
            "margins_um, along_um and exact are a [device]'s or a drawing's cut: a task alone \
             gives cells and height_um",
        ));
    }
    let grid = plane.grid()?;
    if grid.nx == 0 || grid.ny == 0 || grid.unknowns() == 0 {
        return Err(task_error(format!(
            "[task.plane] cells must hold a field between its walls, got {:?}",
            plane.cells
        )));
    }
    let height = plane
        .height_um
        .ok_or_else(|| task_error("[task.plane] needs height_um, the strips' height"))?;
    let up = grid.ny as f64 * grid.dy;
    let room = if symmetry == Symmetry::Quarter {
        up
    } else {
        0.5 * up
    };
    if !(height.is_finite() && height > 0.0 && 0.5 * height < room) {
        return Err(task_error(format!(
            "[task.plane] height_um must be positive and inside the window ({up} um high), got \
             {height}"
        )));
    }
    task.cladding()?;
    if symmetry != Symmetry::None && task.section.iter().any(|s| s.center_um.is_some()) {
        return Err(task_error(
            "a symmetric job's strips are centred on its plane of symmetry: leave center_um out",
        ));
    }
    task.validate_sections(grid.nx as f64 * grid.dx, symmetry != Symmetry::None, false)
}

/// The PML inside the walls that aren't planes of symmetry.
fn pml(task: &EmeTask, plane: &PlaneSpec) -> Result<Option<Pml3d>> {
    let Some(p) = &task.pml else {
        return Ok(None);
    };
    let t = p.thickness_um;
    let (west, south) = match plane.symmetry()? {
        Symmetry::None => (t, t),
        Symmetry::Half => (0.0, t),
        Symmetry::Quarter => (0.0, 0.0),
    };
    Ok(Some(Pml3d::new(west, t, south, t, p.strength)?))
}

/// A strip section's plane a share `u` along it, `height` µm high, on `grid`.
fn strip(grid: PlaneGrid, r: &Resolved, u: f64, height: f64) -> Result<Plane> {
    match &r.shape {
        &Shape::Core {
            core,
            cladding,
            center,
            from,
            to,
            profile,
            ..
        } => {
            let w = profile.at(from, to, u);
            let (inside, outside) = (core * core, cladding * cladding);
            Plane::new(grid, |x, y| {
                if (x - center).abs() < 0.5 * w && y.abs() < 0.5 * height {
                    inside
                } else {
                    outside
                }
            })
        }
        Shape::Layers(_) => Err(task_error("a 3D section is a strip: layers are 2D's")),
    }
}

/// How a run's sections are made at a wavelength: from the task's strips, a device's cut, or
/// the cut of the task's drawing.
enum Source {
    Strips {
        grid: PlaneGrid,
        height: f64,
    },
    Device {
        geometry: Box<crate::device::Geometry>,
        plane: PlaneSpec,
        grid: PlaneGrid,
    },
    Drawn {
        structure: Box<Structure>,
        core: String,
        ends: (f64, f64),
        plane: PlaneSpec,
        grid: PlaneGrid,
    },
}

impl Source {
    fn of(job: &Job, task: &EmeTask, plane: &PlaneSpec) -> Result<Source> {
        if job.device().is_some() {
            let (device, design) = crate::job::device::spec_of(job)?.build()?;
            let geometry = device.geometry(design.values())?;
            let grid = plane.cut()?.geometry_grid(&geometry)?;
            return Ok(Source::Device {
                geometry: Box::new(geometry),
                plane: plane.clone(),
                grid,
            });
        }
        if task.drawing_field().is_some() {
            let (structure, core, ends) = drawing(task)?;
            let grid = plane.cut()?.grid(&structure, &core)?;
            return Ok(Source::Drawn {
                structure: Box::new(structure),
                core,
                ends,
                plane: plane.clone(),
                grid,
            });
        }
        Ok(Source::Strips {
            grid: plane.grid()?,
            height: plane.height_um.unwrap_or(f64::NAN),
        })
    }

    fn grid(&self) -> PlaneGrid {
        match self {
            Source::Strips { grid, .. }
            | Source::Device { grid, .. }
            | Source::Drawn { grid, .. } => *grid,
        }
    }

    /// The sections at `wavelength` (µm), each taper's steps times `scale` (a device's sloped
    /// runs in steps `scale` times shorter), and the steps the tapers have.
    fn sections(
        &self,
        task: &EmeTask,
        wavelength: f64,
        scale: usize,
    ) -> Result<(Vec<Section3d>, usize)> {
        match self {
            Source::Device {
                geometry, plane, ..
            } => {
                let mut cut = plane.cut()?;
                cut.along /= scale as f64;
                let cut = cut.geometry(geometry, Wavelength::um(wavelength)?)?;
                let steps = cut.sections.len().saturating_sub(2);
                Ok((cut.sections, steps))
            }
            Source::Drawn {
                structure,
                core,
                ends,
                plane,
                ..
            } => {
                let mut cut = plane.cut()?;
                cut.along /= scale as f64;
                let cut = cut.structure(structure, core, *ends, Wavelength::um(wavelength)?)?;
                let steps = cut.sections.len().saturating_sub(2);
                Ok((cut.sections, steps))
            }
            &Source::Strips { grid, height } => {
                let resolved = task.sections(wavelength)?;
                let mut sections = Vec::new();
                let mut steps = 0;
                for r in &resolved {
                    if r.is_taper() {
                        let n = r.steps() * scale;
                        steps += n;
                        let l = r.length;
                        sections.extend(Section3d::sampled(Length::um(l), n, |z| {
                            strip(grid, r, z.to_um() / l, height)
                        })?);
                    } else {
                        sections.push(Section3d::new(
                            strip(grid, r, 0.0, height)?,
                            Length::um(r.length),
                        ));
                    }
                }
                Ok((sections, steps))
            }
        }
    }
}

/// One solve at `wavelength` with `modes` modes (at most the plane's unknowns) and each taper's
/// steps times `scale`.
fn solve(
    task: &EmeTask,
    plane: &PlaneSpec,
    source: &Source,
    wavelength: f64,
    (modes, scale): (usize, usize),
) -> Result<Solved> {
    let (sections, steps) = source.sections(task, wavelength, scale)?;
    let eme = settings(task, plane, source, wavelength, modes)?;
    // a task alone's strips only: a [device]'s sections have no [task.period]
    let range = match task.period {
        Some(_) => period_range(task, &task.sections(wavelength)?, scale),
        None => None,
    };
    let s = match range {
        Some((range, count)) => eme.grating(&sections, range, count)?,
        None => eme.s_matrix(&sections)?,
    };
    Ok((s, None, steps))
}

/// The expansion's settings at `wavelength` µm with `modes` modes (at most the plane's
/// unknowns).
fn settings(
    task: &EmeTask,
    plane: &PlaneSpec,
    source: &Source,
    wavelength: f64,
    modes: usize,
) -> Result<Eme3d> {
    let mut eme = Eme3d::new(
        Wavelength::um(wavelength)?,
        modes.min(source.grid().unknowns()),
    )?;
    if let Some(p) = pml(task, plane)? {
        eme = eme.with_pml(p);
    }
    Ok(eme)
}

/// The Bloch modes of a `[task.period]`'s period (a task alone's strips) at `wavelength` µm, as
/// a 2D job's ([`super::bloch_event`]), their TE fractions not given (the modes are hybrid).
fn bloch_event(
    task: &EmeTask,
    plane: &PlaneSpec,
    source: &Source,
    wavelength: f64,
    (modes, scale): (usize, usize),
) -> Result<Option<Event>> {
    if task.period.is_none() {
        return Ok(None);
    }
    let Some((range, _)) = period_range(task, &task.sections(wavelength)?, scale) else {
        return Ok(None);
    };
    let (sections, _) = source.sections(task, wavelength, scale)?;
    let eme = settings(task, plane, source, wavelength, modes)?;
    let bloch = eme.bloch(&sections[range])?;
    Ok(Some(bloch_point(&bloch, wavelength, None)))
}

/// What a 3D job's check refuses besides [`validate`]: the materials at each wavelength, the
/// device's design and cut, the PML.
pub(super) fn check(job: &Job, task: &EmeTask) -> Result<()> {
    let Some(plane) = &task.plane else {
        return Ok(());
    };
    pml(task, plane)?;
    let source = Source::of(job, task, plane)?;
    if let Source::Drawn { structure, .. } = &source {
        crate::job::check_materials(structure.stack(), &task.wavelengths())?;
    }
    for w in task.wavelengths() {
        if let Source::Strips { .. } = source {
            task.sections(w)?;
        }
        Wavelength::um(w)?;
    }
    Ok(())
}

/// A task alone's scene: its strips seen from above, as a 2D job's cores; or its drawing over
/// its core layer's shapes along x and its plane across and up (both halves of a mirrored one).
pub(super) fn scene(task: &EmeTask, plane: &PlaneSpec) -> Result<Event> {
    if task.drawing_field().is_some() {
        let (structure, core, _) = drawing(task)?;
        let grid = plane.cut()?.grid(&structure, &core)?;
        let (lo, hi) = extent(&structure, &core)
            .ok_or_else(|| task_error(format!("the layer {core} has no shapes to cut")))?;
        let right = grid.x(grid.nx);
        let across = if plane.symmetry()? == Symmetry::None {
            [grid.x0, right]
        } else {
            [-right, right]
        };
        return crate::job::scene::scene(
            &structure,
            [lo, hi],
            across,
            [grid.y0, grid.y(grid.ny)],
            Wavelength::um(task.wavelength_um)?,
        );
    }
    let grid = plane.grid()?;
    super::scene_across(
        task,
        grid.nx as f64 * grid.dx,
        plane.symmetry()? != Symmetry::None,
    )
}

/// The lowest real permittivity on `plane`'s values of E_x: its cladding's.
fn lowest(plane: &Plane) -> f64 {
    let g = plane.grid();
    (0..=g.ny)
        .flat_map(|j| (0..g.nx).map(move |i| (i, j)))
        .map(|(i, j)| plane.eps_x(i, j).re)
        .fold(f64::INFINITY, f64::min)
}

/// The guided modes of an end: those with a real index above its cladding's (at least the
/// first), up to 4.
fn guided(indices: &[c64], cladding: f64) -> usize {
    indices
        .iter()
        .take(4)
        .take_while(|n| n.re > cladding && n.im.abs() <= 1e-9 * n.re)
        .count()
        .max(1)
}

/// The planes cut from a `[device]` or a drawing as the run records its sections, `"cut 1"` to
/// the last, each where it starts along x from the first.
fn cut_sections(sections: &[Section3d]) -> Event {
    let mut z = 0.0;
    let sections = sections
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let length = s.length.to_um();
            let record = super::EmeSection {
                name: format!("cut {}", k + 1),
                kind: "cut".into(),
                start_um: z,
                length_um: length,
                steps: 1,
                radius_um: None,
                angle_deg: None,
                repeats: 1,
            };
            z += length;
            record
        })
        .collect();
    Event::EmeSections { sections }
}

/// The run of a 3D job: its scene (a task alone's; a device's is recorded before), the
/// solver's settings, and at each wavelength the S-parameters and any convergence study.
pub(super) fn run(
    job: &Job,
    task: &EmeTask,
    plane: &PlaneSpec,
    run: &mut Run,
    stop: &Stop,
) -> Result<()> {
    let device = job.device().is_some();
    if !device {
        run.record(&scene(task, plane)?)?;
    }
    let source = Source::of(job, task, plane)?;
    let grid = source.grid();
    let wavelengths = task.wavelengths();
    let (first, base_steps) = source.sections(task, wavelengths[0], 1)?;
    let modes = task.modes.min(grid.unknowns());
    let mut details = vec![
        [
            "plane".to_owned(),
            format!(
                "{} × {} cells of {:.1} × {:.1} nm, {} unknowns",
                grid.nx,
                grid.ny,
                1e3 * grid.dx,
                1e3 * grid.dy,
                grid.unknowns()
            ),
        ],
        [
            "modes".to_owned(),
            if modes == grid.unknowns() {
                format!("{modes}, every one of the plane's")
            } else {
                modes.to_string()
            },
        ],
        [
            "symmetry".to_owned(),
            plane.symmetry.clone().unwrap_or_else(|| "none".into()),
        ],
        [
            "sections".to_owned(),
            match source {
                Source::Device { .. } => plane.cut_details("device", first.len()),
                Source::Drawn { .. } => plane.cut_details("drawing", first.len()),
                Source::Strips { .. } => {
                    format!("{} ({base_steps} of them the tapers' steps)", first.len())
                }
            },
        ],
    ];
    if let Some(d) = super::period_details(task) {
        details.push(d);
    }
    if let Some(p) = &task.pml {
        details.push([
            "PML".into(),
            format!(
                "{} um of strength {} inside the walls but the planes of symmetry",
                p.thickness_um, p.strength
            ),
        ]);
    }
    run.record(&Event::Solver {
        module: "eme 3d".into(),
        cells: [grid.nx, grid.ny],
        step_um: grid.dx.max(grid.dy),
        unknowns: grid.unknowns(),
        details,
    })?;
    run.record(&match source {
        Source::Strips { .. } => super::sections_event(task, &task.sections(wavelengths[0])?),
        Source::Device { .. } | Source::Drawn { .. } => cut_sections(&first),
    })?;
    if let Some(s) = &task.sweep {
        run.record(&Event::Sweep {
            parameter: "wavelength".into(),
            from: s.from,
            to: s.to,
            points: s.points,
        })?;
    }
    // the guided modes at each end at a wavelength
    let ports = |wl: f64, s: &Scattering| -> Result<(usize, usize)> {
        let (sections, _) = source.sections(task, wl, 1)?;
        let ends = [&sections[0].plane, &sections[sections.len() - 1].plane]
            .map(|p| lowest(p).max(0.0).sqrt());
        Ok((guided(&s.first, ends[0]), guided(&s.last, ends[1])))
    };
    if task.interpolate.is_some() {
        // the discretisation settled once, at the job's wavelength, and kept at every solve
        let at = match &task.convergence {
            None => (task.modes, 1),
            Some(c) => converged(task, plane, &source, task.wavelength_um, c, base_steps, run)?.0,
        };
        if let Some(e) = bloch_event(task, plane, &source, task.wavelength_um, at)? {
            run.record(&e)?;
        }
        return interpolated(
            task,
            run,
            stop,
            |wl| Ok(solve(task, plane, &source, wl, at)?.0),
            ports,
            |m| format!("m{m}"),
        );
    }
    for &wl in &wavelengths {
        if stop.reason().is_some() {
            return Ok(());
        }
        let (at, (s, _, _)) = match &task.convergence {
            None => (
                (task.modes, 1),
                solve(task, plane, &source, wl, (task.modes, 1))?,
            ),
            Some(c) => converged(task, plane, &source, wl, c, base_steps, run)?,
        };
        let counts = ports(wl, &s)?;
        run.record(&ports_event(&s, wl, counts, |m| format!("m{m}")))?;
        if let Some(e) = bloch_event(task, plane, &source, wl, at)? {
            run.record(&e)?;
        }
    }
    Ok(())
}

/// A wavelength's solve driven by `[task.convergence]`: the modes doubled (and a task alone's
/// tapers' steps with them) until the quantity changes by less than its tolerance, each study
/// recorded; where it ended, (modes, the steps' scale), and the solve there.
fn converged(
    task: &EmeTask,
    plane: &PlaneSpec,
    source: &Source,
    wl: f64,
    c: &super::ConvergenceSpec,
    base_steps: usize,
    run: &mut Run,
) -> Result<((usize, usize), Solved)> {
    let (name, read) = quantity(c.quantity.as_deref())?;
    let label = format!("{name} of the fundamental modes at {wl} um");
    let driver = ToTolerance::new(&label, Tolerance::Relative(c.tolerance))?;
    let mut solved: Vec<((usize, usize), Solved)> = Vec::new();
    let strips_with_tapers = matches!(source, Source::Strips { .. }) && base_steps > 0;
    let (studies, at): (Vec<Refined>, (usize, usize)) = if strips_with_tapers {
        let r = driver.eme(task.modes, base_steps, |m, n| {
            let s = solve(task, plane, source, wl, (m, n / base_steps))?;
            let v = read(&s.0);
            solved.push(((m, n / base_steps), s));
            Ok(v)
        })?;
        let at = (r.modes, r.steps / base_steps);
        (vec![r.mode_study, r.step_study, r.together], at)
    } else {
        let r = driver.counts(task.modes, |m| {
            let s = solve(task, plane, source, wl, (m, 1))?;
            let v = read(&s.0);
            solved.push(((m, 1), s));
            Ok(v)
        })?;
        let m = (1.0 / r.grids[r.grids.len() - 1]).round() as usize;
        (vec![r], (m, 1))
    };
    for r in &studies {
        run.record(&Event::GridConvergence {
            label: r.quantity.clone(),
            wavelength_um: wl,
            check: check_of(r),
        })?;
    }
    let s = match solved.into_iter().find(|(k, _)| *k == at) {
        Some((_, s)) => s,
        None => solve(task, plane, source, wl, at)?,
    };
    Ok((at, s))
}

//! The jobs `photonoxide run` knows, and the events they record.
//!
//! A job file's `[solver]` table, if it has one, names the direct solver of an `"fdfd"` job
//! ([`crate::backend`]): `direct = "auto"` (the default: photonoxide's own), `"photonoxide"`,
//! or a backend registered under another name. A backend that isn't registered or isn't
//! available is refused by [`check`] and by the run, with the reason; the run's record names
//! the backend that solved.
//!
//! **Parameters and expressions.** A task may have a `[task.parameters]` table of named
//! parameters, each an expression ([`crate::expr`]) of numbers with units and of the other
//! parameters, in any order (a cycle is refused). Wherever a shape table (`[[task.rect]]`,
//! `[[task.circle]]`, `[[task.ring]]`) takes a length, it takes an expression too: its `center`,
//! `size`, `radius` and `width` keys hold expressions with units (`"500 nm"`, `"w/2 + gap"`; a
//! bare `0` needs none), and its `_um` keys numbers in µm as they always have, or expressions,
//! read in µm when they have no unit. A numeric `_um` key reads exactly as before.
//!
//! ```toml
//! [task.parameters]
//! w = "500 nm"
//! gap = "w/2.5"
//! radius = "5 um"
//!
//! [[task.ring]]
//! layer = "Si"
//! center = [0, "radius + w + gap"]
//! radius = "radius"
//! width = "w"
//! ```
//!
//! A modes job's sweep can step through any parameter, `parameter = "gap"`, from `from` to `to`
//! (numbers in µm for a length and radians for an angle, or expressions with units: `"150 nm"`);
//! at each point the parameters that use it follow, and the point records its shapes as a width
//! sweep's do. [`check`] refuses an expression that doesn't parse, an unknown name, units that
//! don't add up, a parameter named `wavelength` or `width` (a sweep's own) or a coordinate in a
//! shape's field, naming the table and field: `[[task.rect]] #2, size: value 1: column 3:
//! unknown name "wdth"`; and a parameter's sweep whose shapes at either end aren't shapes.
//!
//! A job file's `[task]` table names its kind. The kinds so far:
//!
//! - `"structure"`: a layer stack with shapes drawn on it, recorded as pictures of its
//!   permittivity, seen from above and on a vertical cut.
//!
//! ```toml
//! name = "strip"
//! timeout_minutes = 5
//!
//! [task]
//! kind = "structure"
//! stack = "soi_220"          # or "soi" / "nitride" with core_nm and bottom_oxide_um
//! wavelength_um = 1.55
//! layer = "Si"               # the layer seen from above
//! x_um = [-3.0, 3.0]
//! y_um = [-2.0, 2.0]
//! step_nm = 20.0
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [6.0, 0.5]
//!
//! [[task.circle]]
//! layer = "Si"
//! center_um = [1.5, 1.2]
//! radius_um = 0.4
//!
//! [[task.ring]]              # a ring resonator's waveguide
//! layer = "Si"
//! center_um = [-1.5, 0.9]
//! radius_um = 0.8            # to the waveguide's centre line
//! width_um = 0.4
//! ```
//!
//! - `"modes"`: the guided modes of a waveguide's cross-section, by the full-vector solver
//!   ([`crate::mode::vector`]) on a uniform grid of `step_nm`; recorded as a picture of the
//!   cross-section and, for each mode, one of its |E|² and one of its signed transverse field,
//!   and optionally swept over the wavelength or a rectangle's width, each point recorded as it
//!   is solved, with its modes' pictures and, for a width sweep, its shapes.
//!
//!   Light travels along x in every kind of job: a modes job with `propagation = "x"` cuts the
//!   stack and shapes at x = `cut_x_um` (0 by default) over the window `y_um` across the guide,
//!   as an `"fdfd"` job's ports are columns normal to x, so the same rectangle is the same guide
//!   in both. A job without `propagation` is an older one, its modes travelling along y: it is
//!   cut at y = `cut_y_um` over the window `x_um`, as `propagation = "y"` says, and runs exactly
//!   as before. Either way the cross-section's pictures have the axis across the guide (y for
//!   modes along x, x for modes along y) across and z up, and a width sweep's `"width"` is the
//!   swept rectangle's size across the guide: along y for modes along x, along x for modes along
//!   y. A job that names its `propagation` records an [`Event::Cut`] after its scene, naming
//!   the cut's plane.
//!
//! ```toml
//! name = "strip-modes"
//! timeout_minutes = 10
//!
//! [task]
//! kind = "modes"
//! stack = "soi_220"
//! wavelength_um = 1.55
//! layer = "Si"               # the window's height defaults to 1 µm around it
//! propagation = "x"          # the modes travel along x; "y" (the default) for an older job
//! y_um = [-1.2, 1.2]         # the window across the guide (x_um for "y")
//! cut_x_um = 0.0             # where the cross-section is cut (cut_y_um for "y")
//! step_nm = 20.0
//! modes = 2                  # how many (1 to 50), from the highest effective index
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [10.0, 0.5]      # 500 nm wide, along x
//!
//! [task.sweep]
//! parameter = "wavelength"   # "width": rect `rect`'s size across the guide; or a parameter
//! from = 1.5
//! to = 1.6
//! points = 11
//! ```
//!
//! - `"fdfd"`: a device on one layer, seen from above, by 2D FDFD ([`crate::fdfd`]) with
//!   ports: each point's permittivity is its slab's effective index squared (the effective
//!   index method, so the results are 2D estimates, not a device's 3D performance). Recorded
//!   as the S-parameters at each wavelength, and the field from the first port at the first.
//!   The ports are columns normal to x, so the light travels along x.
//!
//! ```toml
//! name = "mmi"
//! timeout_minutes = 20
//!
//! [task]
//! kind = "fdfd"
//! stack = "soi_220"
//! wavelength_um = 1.55
//! layer = "Si"
//! polarization = "te"        # the slab's TE mode, H along z in the 2D problem; or "tm"
//! x_um = [-2.0, 12.5]        # PMLs (pml_cells, 20 by default) inside the window
//! field_um = 1.55            # the field is recorded at the swept wavelength nearest this
//! y_um = [-3.0, 3.0]
//! step_nm = 20.0
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [4.275, 0.0]
//! size_um = [8.55, 3.0]
//!
//! [[task.port]]
//! x_um = -1.0
//! side = "left"              # light comes in towards +x
//!
//! [[task.port]]
//! x_um = 11.0
//! side = "right"
//! y_um = [0.0, 3.0]          # one guide of several: the port's window
//!
//! [task.sweep]
//! parameter = "wavelength"
//! from = 1.5
//! to = 1.6
//! points = 11
//! ```
//!
//! - `"fdtd"`: a structure stepped in time by FDTD ([`crate::fdtd`]): in 2D (`dimensions = 2`,
//!   the default) the layer's plane, each point's permittivity its slab mode's effective index
//!   squared as an `"fdfd"` job's (so its results are 2D estimates, not a device's 3D
//!   performance), or in 3D the stack itself over `z_um`. The materials are taken at
//!   `wavelength_um`, the carrier of the one Gaussian pulse every source shares, which is wide
//!   enough to cover the `[task.spectrum]` (1 point at the carrier by default). Times are c·t
//!   in µm (µm/c). The run records the field on a plane as it propagates (frames at intervals
//!   that double as the run goes on, at most 500), its progress, each monitor's spectrum as
//!   it accumulates and at the end, the resonances, and the transforms' |E|² pictures.
//!
//!   Planes are given by their `normal` and `at_um`, with windows along the other axes
//!   (`y_um`, `x_um`, and `z_um` in 3D; the whole window by default). Sources: `"mode"` (a
//!   guide's fundamental mode of the job's polarization, or a 3D source's own `"te"` or `"tm"`,
//!   launched one way, `direction = "+"` or `"-"` along the normal), `"dipole"` (`position_um`
//!   and `component`, e.g. `"Ez"` or `"Hz"`), `"plane_wave"` (on a total-field/scattered-field
//!   box `x_um`, `y_um` (`z_um`), `direction` `"+x"` to `"-z"`, in 3D a `polarization`, E's
//!   axis) and `"beam"` (a Gaussian beam from a plane: `center_um` across it, `waist_um`,
//!   `focus_um` ahead, `angle_deg` of tilt in the layer's plane, in 3D `"s"` or `"p"`).
//!   Monitors: `"mode"` (the power in a guide's mode each way, through a plane), `"flux"`
//!   (through a plane), `"flux_box"` (out of a box), `"field"` (|E|² of the transforms on the
//!   view's plane at `wavelengths_um`) and `"resonance"` (a point's field after the pulse, by
//!   harmonic inversion: wavelengths and Q). Their spectra are referred to the incident power:
//!   a mode source's own mode's, measured just after it; a plane wave's through its box's face;
//!   a beam's, paraxial. With a dipole they are per unit source spectrum instead. A mode
//!   source also records its reflection, back into its own mode. Boundaries are `"cpml"`
//!   (`pml_cells` thick, 20 by default, inside the window), `"wall"` or `"periodic"` along each
//!   axis, with Bloch wavenumbers (complex fields, driven by dipoles only); a 2D job is
//!   uniform along z. The run stops when |field|² at the monitors' middles has stayed below
//!   `fraction` (1e-8) of its peak over ten carrier periods, or at `limit_um` (2000), or at
//!   `time_um` with `until = "time"`, or at the job's timeout.
//!
//! ```toml
//! name = "mmi-fdtd"
//! timeout_minutes = 20
//!
//! [task]
//! kind = "fdtd"
//! stack = "soi_220"
//! wavelength_um = 1.55       # the pulse's carrier, and the materials' wavelength
//! layer = "Si"
//! dimensions = 2             # 2: the layer's plane by the effective index method; 3: the stack
//! polarization = "te"        # 2D: "te" (H along z, E in the plane) or "tm" (E along z)
//! x_um = [-2.0, 12.5]
//! y_um = [-3.0, 3.0]
//! step_nm = 20.0
//! courant = 0.9              # the time step, C/sqrt(sum of 1/step^2), 0 < C <= 1
//! pml_cells = 20             # each CPML's cells, inside the window
//!
//! [task.boundaries]          # "cpml" (the default), "wall" or "periodic" along each axis
//! x = "cpml"
//! y = "cpml"
//!
//! [task.spectrum]            # the wavelengths the monitors report
//! from_um = 1.5
//! to_um = 1.6
//! points = 11
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [4.275, 0.0]
//! size_um = [8.55, 3.0]
//!
//! [[task.source]]
//! type = "mode"
//! normal = "x"
//! at_um = -1.0
//! direction = "+"            # towards +x
//!
//! [[task.monitor]]
//! type = "mode"
//! name = "upper output"
//! normal = "x"
//! at_um = 11.0
//! y_um = [0.0, 3.0]          # one guide of two: its window across
//!
//! [[task.monitor]]
//! type = "resonance"
//! position_um = [4.0, 0.5]
//! component = "Hz"
//!
//! [task.stop]
//! until = "decay"            # or "time", with time_um
//! fraction = 1e-10
//! limit_um = 2000.0
//!
//! [task.view]                # the field the frames show
//! field = "Hz"               # a component, "|E|^2" or "|H|^2"
//! ```

use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Shape};
use crate::raster::Raster;
use crate::run::{Job, Run, Stop, StopReason};
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

mod fdfd;
mod fdtd;
mod params;

pub use fdfd::{FdfdSParameters, fdfd_s_parameters};
pub use fdtd::{FoundResonance, SpectrumSeries};

/// The memory a sweep's points in flight may take in all, by their estimate: what bounds them
/// when the machine has more threads than a sweep's points can share it with.
const IN_FLIGHT_BYTES: f64 = 8e9;

/// How many of a sweep's points to solve at once: one per thread of rayon's pool, but no more
/// than [`IN_FLIGHT_BYTES`] holds at `bytes_per_point` each, and at least one.
fn in_flight(bytes_per_point: f64) -> usize {
    let by_memory = (IN_FLIGHT_BYTES / bytes_per_point.max(1.0)).floor() as usize;
    rayon::current_num_threads().min(by_memory).max(1)
}

/// A sweep's points solved side by side, `batch` at a time on rayon's threads, and recorded in
/// their order: `point(k)` makes point k's events without the run (`None` when the stop came
/// first), and each batch's events go into `run` point by point, so the record is the one a loop
/// over the points writes, whatever the threads. A stop ends the sweep at its first point not
/// done; an error at a point ends it there, after the points before it.
fn sweep_points(
    run: &mut Run,
    stop: &Stop,
    points: std::ops::Range<usize>,
    batch: usize,
    point: impl Fn(usize) -> Result<Option<Vec<Event>>> + Sync,
) -> Result<()> {
    use rayon::prelude::*;
    let all: Vec<usize> = points.collect();
    for ks in all.chunks(batch.max(1)) {
        if stop.reason().is_some() {
            return Ok(());
        }
        let done: Vec<Result<Option<Vec<Event>>>> = ks.par_iter().map(|&k| point(k)).collect();
        for events in done {
            let Some(events) = events? else {
                return Ok(());
            };
            for e in &events {
                run.record(e)?;
            }
        }
    }
    Ok(())
}

/// An event of a run's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The run started.
    Started {
        /// The job's name.
        job: String,
        /// The task's kind, e.g. `"structure"`.
        kind: String,
    },
    /// The structure in 3D, for the studio's 3D view: the stack's layers and every shape's
    /// outline, over the window the run looks at.
    Scene {
        /// The window along x, µm.
        x_um: [f64; 2],
        /// Along y, µm.
        y_um: [f64; 2],
        /// Along z (height), µm.
        z_um: [f64; 2],
        /// The vacuum wavelength the permittivities are at, µm.
        wavelength_um: f64,
        /// Below the stack.
        substrate: SceneMedium,
        /// Above it.
        cladding: SceneMedium,
        /// The layers, bottom to top.
        layers: Vec<SceneLayer>,
        /// Every shape drawn on a layer, as an outline.
        shapes: Vec<SceneShape>,
    },
    /// A picture of the permittivity (real part).
    Permittivity {
        /// What the picture shows, e.g. `"top view of layer Si"`.
        view: String,
        /// The axis labels, e.g. `["x", "y"]`.
        axes: [String; 2],
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The picture.
        raster: Raster,
    },
    /// A guided mode of a cross-section.
    Mode {
        /// What it is, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y (see
        /// [`crate::mode::vector::VectorMode::te_fraction`]): near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|² at the cells' centres, its largest value 1; across the guide (x, or y for modes
        /// travelling along x: see [`Event::Cut`]) and z up.
        intensity: Raster,
        /// Where the cross-section was cut along the guide, µm: the y of the cut, or its x when
        /// the run's [`Event::Cut`] is normal to x (the modes travel along x).
        #[serde(default)]
        cut_y_um: f64,
    },
    /// A point of a sweep: the modes' effective indices at one value of the parameter.
    SweepPoint {
        /// `"wavelength"` (µm), `"width"` (µm) or a parameter's name (µm for a length, radians
        /// for an angle).
        parameter: String,
        /// The parameter's value.
        value: f64,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The modes' effective indices, highest first, real and imaginary parts.
        effective_indices: Vec<[f64; 2]>,
        /// Their TE fractions, in the same order.
        te_fractions: Vec<f64>,
    },
    /// A field of a 2D FDFD run, seen from above, over the window without its PMLs (as the
    /// run's [`Event::Scene`] is: the PMLs absorb the light and aren't part of the device, so
    /// the pictures leave them and the two cells beside them out). It is the field port 1's
    /// mode excites, launched from
    /// the window's end of port 1's guide (just inside the PML) when the guide is the same
    /// there as at the port, else from the port's column, so that the picture shows the wave
    /// along the whole guide. The S-parameters are referred to the ports' own columns either
    /// way.
    Field {
        /// What it shows, e.g. `"|H_z|^2 from port 1, 2D by the effective index method"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The height the 3D view draws it at, µm: the layer's top face.
        z_um: f64,
        /// |field|², its largest value 1; x across, y up.
        intensity: Raster,
    },
    /// A 2D FDFD run's S-parameters at one wavelength.
    SParameters {
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The ports' names, in order.
        ports: Vec<String>,
        /// Each port's mode's effective index.
        effective_indices: Vec<f64>,
        /// The power-normalized S-matrix, `s[q][p]` from port p into port q, real and
        /// imaginary parts.
        s: Vec<Vec<[f64; 2]>>,
    },
    /// The run ended.
    Finished {
        /// Why it ended early, if it did: `"timeout"` or `"requested"`.
        stopped: Option<String>,
        /// Its duration, seconds.
        seconds: f64,
    },
    /// A guided mode's signed field on its cross-section, recorded right after its
    /// [`Event::Mode`]: the transverse component of E that carries most of |E|², with the
    /// mode's global phase chosen to make that component real and positive where its magnitude
    /// peaks, and then its real part. For a lossless guided mode the component is then real
    /// everywhere up to round-off, so along the guide (the modes travel along +y, or +x when the
    /// run's [`Event::Cut`] is normal to x; s below is that coordinate) the field is this
    /// picture times cos(β (s − cut) − ωt), with β = k₀ Re n_eff and the cut at
    /// [`Event::Mode`]'s `cut_y_um`, and a lossy mode's also decays as e^(−k₀ Im n_eff (s − cut)).
    /// Unlike |E|², it shows where a higher-order mode changes sign.
    ModeField {
        /// The [`Event::Mode`]'s label, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The component, in the job's axes: `"Ex"` or `"Ey"` (across the guide, for modes
        /// travelling along y or x) or `"Ez"` (up).
        component: String,
        /// Its real part at the cells' centres, scaled so that its largest magnitude is 1 (from
        /// −1 to 1, positive at the peak); across the guide and z up, on the [`Event::Mode`]'s
        /// grid.
        values: Raster,
    },
    /// The shapes at a point of a width sweep, recorded after its [`Event::SweepPoint`]: the
    /// [`Event::Scene`]'s shapes with the swept rectangle at that width (the window and the
    /// layers stay the scene's).
    SweepShapes {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there, µm.
        value: f64,
        /// Every shape, as the scene lists them.
        shapes: Vec<SceneShape>,
    },
    /// The cross-section's permittivity at a point of a width sweep, recorded after its
    /// [`Event::SweepShapes`]: what the job's own [`Event::Permittivity`] of the cross-section
    /// shows, with the swept rectangle at that width, on pixels twice the grid's step and to
    /// three decimals. (A wavelength sweep changes ε only through dispersion and records none.)
    SweepPermittivity {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there, µm.
        value: f64,
        /// What the picture shows, e.g. `"cross-section at x = 0 um"`.
        view: String,
        /// The axis labels: across the guide and up, `["y", "z"]` for modes travelling along x
        /// and `["x", "z"]` along y.
        axes: [String; 2],
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The picture: Re ε.
        raster: Raster,
    },
    /// A mode at a point of a sweep, recorded after its [`Event::SweepPoint`] (and
    /// [`Event::SweepShapes`]): what [`Event::Mode`] and [`Event::ModeField`] record for the job's
    /// own configuration, on pixels twice the grid's step and to three decimals, so a sweep's
    /// record stays small.
    SweepMode {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there.
        value: f64,
        /// What it is, e.g. `"mode 1 of 2"`, ranked by effective index at that point.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|², its largest value 1; across the guide, z up.
        intensity: Raster,
        /// The signed field's component, as [`Event::ModeField`] names it: `"Ex"`, `"Ey"` or
        /// `"Ez"`.
        component: String,
        /// That component's real part, from −1 to 1, as [`Event::ModeField`] records it.
        field: Raster,
        /// Where the cross-section was cut along the guide, µm, as [`Event::Mode`] records it.
        cut_y_um: f64,
    },
    /// A sweep is about to run, recorded before its first point: what it steps through, so that
    /// a viewer can say how far along a running one is.
    Sweep {
        /// `"wavelength"` (µm), `"width"` (µm) or a parameter's name (µm for a length, radians
        /// for an angle).
        parameter: String,
        /// The first point's value.
        from: f64,
        /// The last point's value.
        to: f64,
        /// How many points, evenly spaced from `from` to `to`.
        points: usize,
    },
    /// The field at a point of a 2D FDFD run's wavelength sweep, recorded after the point's
    /// [`Event::SParameters`]: what [`Event::Field`] records at the one wavelength the job asks
    /// for, averaged over blocks of cells (2 × 2, or larger for a sweep long enough that all its
    /// pictures would pass five million pixels) and to three decimals, so a sweep's record stays
    /// small.
    SweepField {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there: the vacuum wavelength, µm.
        value: f64,
        /// What it shows, as [`Event::Field`]'s label.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The height the 3D view draws it at, µm: the layer's top face.
        z_um: f64,
        /// |field|², its largest value 1 (each point's own peak, so the pictures compare in
        /// shape, not in strength); x across, y up.
        intensity: Raster,
    },
    /// The plane a modes job cut its cross-section on, recorded right after its
    /// [`Event::Scene`] by a job that names its `propagation`: the modes travel along the
    /// plane's normal, towards +x or +y. A modes run without one (an older job) cut normal to
    /// y, at its [`Event::Mode`]s' `cut_y_um`.
    Cut {
        /// The axis the plane is normal to and the modes travel along: `"x"` or `"y"`.
        normal: String,
        /// Where the plane crosses that axis, µm.
        at_um: f64,
    },
    /// How the run solves, recorded once before its first solve: the method and its grid.
    Solver {
        /// The library module that solves, e.g. `"mode::vector"` or `"fdfd"`.
        module: String,
        /// The grid's cells along its two axes: across the guide and up for a modes job, x and
        /// y for an fdfd job.
        cells: [usize; 2],
        /// The grid's step as the job asks for it, µm.
        step_um: f64,
        /// The unknowns of one solve.
        unknowns: usize,
        /// Further facts, each a name and its value, e.g. `["PML", "20 cells on each side"]`.
        details: Vec<[String; 2]>,
    },
    /// A measure of a solve's numerical error, recorded after what the solve gave: a mode's
    /// eigen-residual ([`crate::mode::vector::VectorMode::residual`]), an FDFD field's linear
    /// residual ([`crate::fdfd::Solver2d::residual`]), or how far an S-matrix is from
    /// reciprocal (the largest |S_qp − S_pq|). These say how well the discrete problem is
    /// solved, not how well the grid resolves the device.
    SolveError {
        /// The sweep point's index, from 0, or `None` for the job's own configuration.
        point: Option<usize>,
        /// The swept parameter's value there, or the job's wavelength, µm.
        value: f64,
        /// What is measured, e.g. `"eigen-residual, mode 1 of 2"`, `"linear residual"` or
        /// `"reciprocity"`.
        measure: String,
        /// Its value: smaller is better.
        error: f64,
    },
    /// A picture of an FDTD run's field at one moment, seen from above on the plane the run's
    /// view names, over the part of the window its [`Event::Scene`] shows: the field averaged
    /// over blocks of cells (so that its longer side is at most 240 pixels) and quantized to
    /// bytes of its peak. Frames are recorded at intervals of the run's time that double as the
    /// run goes on and when a frame takes more than a tenth of the steps' time since the last, at
    /// most 500 of them. Which steps have a frame follows the clock; each frame is its step's.
    FdtdFrame {
        /// The steps taken.
        step: usize,
        /// The time of E, c·t in µm (µm/c).
        time_um: f64,
        /// The field shown: a component (`"Ez"`, `"Hz"`, …), `"|E|^2"` or `"|H|^2"`.
        field: String,
        /// The height of the plane, µm (in 2D, the layer's top face, where the 3D view draws it).
        z_um: f64,
        /// The pixels along x.
        nx: usize,
        /// Along y.
        ny: usize,
        /// The pixels' extent along x, µm.
        x_um: [f64; 2],
        /// Along y.
        y_um: [f64; 2],
        /// The largest magnitude among the pixels, in the field's units (E or H̃ = η₀H, the
        /// sources' currents at unit amplitude; their square for `"|E|^2"`).
        peak: f64,
        /// The pixels row by row from the low y up, each a signed byte from −127 to 127 (0 to
        /// 127 for a square) that times `peak` / 127 is its value, in base64.
        data: String,
    },
    /// How far an FDTD run has gone, recorded with each [`Event::FdtdFrame`] and at least every
    /// second.
    FdtdProgress {
        /// The steps taken.
        step: usize,
        /// The time of E, µm/c.
        time_um: f64,
        /// The time the run stops at, at the latest, µm/c.
        until_um: f64,
        /// The largest |field|² at the monitors' points since the last frame, over its peak
        /// so far: what the run's stopping rule watches.
        decay: f64,
        /// For a run until the fields decay: the share of the peak they must stay below.
        fraction: Option<f64>,
        /// The time the steps have taken so far, seconds.
        steps_seconds: f64,
        /// The time the frames, the spectra and their records have taken so far, seconds.
        frames_seconds: f64,
        /// Cells times steps over `steps_seconds`.
        cell_updates_per_second: f64,
    },
    /// An FDTD monitor's spectrum: its transforms so far, recorded every two seconds or so as they
    /// accumulate, and once more at the end (`last`). For a mode source, also its reflection
    /// back into its own guide's mode.
    FdtdSpectrum {
        /// The monitor's name, or `"source 1"` for a mode source's reflection.
        monitor: String,
        /// `"mode"`, `"flux"`, `"flux_box"` or `"reflection"`.
        kind: String,
        /// What the values are, e.g. `"flux / incident power"`.
        quantity: String,
        /// The vacuum wavelengths, µm.
        wavelengths_um: Vec<f64>,
        /// The values over the wavelengths, one series per direction.
        series: Vec<SpectrumSeries>,
        /// The time the transforms have reached, µm/c.
        time_um: f64,
        /// Whether it is the run's last.
        last: bool,
    },
    /// The resonances an FDTD run's resonance monitor found at the end, by harmonic inversion
    /// of its point's field after the sources had ended.
    FdtdResonances {
        /// The monitor's name.
        monitor: String,
        /// Its point, µm: (x, y), and z in 3D.
        position_um: Vec<f64>,
        /// The component recorded, e.g. `"Hz"`.
        component: String,
        /// The resonances in the spectrum's band, by wavelength: those with harmonic
        /// inversion's error below 1e-2, decaying, and above 1e-6 of the largest amplitude.
        resonances: Vec<FoundResonance>,
    },
    // New variants go here, at the end after the last one, so that the earlier variants keep
    // their discriminants (inserting one in between renumbers every variant after it).
}

/// A medium of a [`Event::Scene`]: its material and the real part of its permittivity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneMedium {
    /// The material's name, e.g. `"SiO2"`.
    pub material: String,
    /// Re ε at the scene's wavelength.
    pub eps: f64,
}

/// A layer of a [`Event::Scene`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneLayer {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its bottom and top, µm.
    pub z_um: [f64; 2],
    /// What shapes on it are made of.
    pub material: SceneMedium,
    /// What fills it elsewhere.
    pub background: SceneMedium,
}

/// A shape of a [`Event::Scene`]: its layer and its outline, counterclockwise, µm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneShape {
    /// The layer it is drawn on.
    pub layer: String,
    /// Its outline's vertices, (x, y), counterclockwise; a circle as a 64-gon. A shape with a
    /// hole (a ring) is one outline that also walks the hole: the outer boundary
    /// counterclockwise and back to its first vertex, then the hole clockwise from its first
    /// vertex and back to it, the two joined by a cut of zero width. Its first vertex
    /// recurring marks where the hole starts.
    pub outline: Vec<[f64; 2]>,
}

/// A shape's outline, counterclockwise: a rectangle's corners, a circle as a 64-gon, a ring as
/// its two 64-gons joined by a cut (see [`SceneShape::outline`]).
fn outline(shape: &Shape) -> Vec<[f64; 2]> {
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            let (cx, cy, w, h) = (
                center.x.to_um(),
                center.y.to_um(),
                width.to_um() / 2.0,
                height.to_um() / 2.0,
            );
            vec![
                [cx - w, cy - h],
                [cx + w, cy - h],
                [cx + w, cy + h],
                [cx - w, cy + h],
            ]
        }
        Shape::Circle { center, radius } => polygon64(*center, radius.to_um(), 1.0),
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let mut o = polygon64(*center, outer.to_um(), 1.0);
            o.push(o[0]);
            let hole = polygon64(*center, inner.to_um(), -1.0);
            o.extend(&hole);
            o.push(hole[0]);
            o
        }
        Shape::Polygon(p) => p
            .vertices()
            .iter()
            .map(|v| [v.x.to_um(), v.y.to_um()])
            .collect(),
        // the other primitives (not yet in jobs): their polygons within 1 nm, holes joined as
        // a ring's are
        other => {
            use crate::geometry::Region;
            let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
            let mut o = Vec::new();
            for c in other
                .polygons(crate::geometry::Tolerance::default())
                .contours
            {
                o.extend(c.outer.iter().map(xy));
                if let Some(first) = c.outer.first()
                    && !c.holes.is_empty()
                {
                    o.push(xy(first));
                }
                for hole in &c.holes {
                    o.extend(hole.iter().map(xy));
                    if let Some(first) = hole.first() {
                        o.push(xy(first));
                    }
                }
            }
            o
        }
    }
}

/// A circle about `center` as a 64-gon from angle 0, counterclockwise for `turn` 1, clockwise for
/// −1.
fn polygon64(center: Point, radius: f64, turn: f64) -> Vec<[f64; 2]> {
    (0..64)
        .map(|k| {
            let a = turn * std::f64::consts::TAU * f64::from(k) / 64.0;
            [
                center.x.to_um() + radius * a.cos(),
                center.y.to_um() + radius * a.sin(),
            ]
        })
        .collect()
}

/// Every shape of `s`, layer by layer from the bottom, as a scene lists them.
fn scene_shapes(s: &Structure) -> Vec<SceneShape> {
    s.stack()
        .layers()
        .iter()
        .flat_map(|layer| {
            s.shapes(&layer.name).iter().map(|shape| SceneShape {
                layer: layer.name.clone(),
                outline: outline(shape),
            })
        })
        .collect()
}

/// The scene of a structure over a window, its permittivities at `wavelength`.
fn scene(
    s: &Structure,
    x: [f64; 2],
    y: [f64; 2],
    z: [f64; 2],
    wavelength: Wavelength,
) -> Result<Event> {
    let medium = |m: &crate::material::Material| -> Result<SceneMedium> {
        Ok(SceneMedium {
            material: m.name().to_owned(),
            eps: m.permittivity(wavelength)?.re,
        })
    };
    let stack = s.stack();
    let mut layers = Vec::new();
    let mut bottom = 0.0;
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        layers.push(SceneLayer {
            name: layer.name.clone(),
            z_um: [bottom, top],
            material: medium(&layer.material)?,
            background: medium(&layer.background)?,
        });
        bottom = top;
    }
    let shapes = scene_shapes(s);
    Ok(Event::Scene {
        x_um: x,
        y_um: y,
        z_um: z,
        wavelength_um: wavelength.to_um(),
        substrate: medium(stack.substrate())?,
        cladding: medium(stack.cladding())?,
        layers,
        shapes,
    })
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RectSpec {
    layer: String,
    center_um: [f64; 2],
    size_um: [f64; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CircleSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
}

/// A ring: its centre line's radius and its waveguide's width ([`Shape::ring`]).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RingSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
    width_um: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructureTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    x_um: [f64; 2],
    y_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    side_y_um: Option<f64>,
    step_nm: f64,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
}

fn task_error(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "task".into(),
        reason: reason.into(),
    }
}

/// A window along one axis, named as the job names it (`"x_um"`): finite, from a smaller to a
/// larger value.
fn check_window(name: &str, w: [f64; 2]) -> Result<()> {
    if w[0].is_finite() && w[1].is_finite() && w[1] > w[0] {
        Ok(())
    } else {
        Err(task_error(format!(
            "{name} must go from a smaller to a larger number, got [{}, {}]",
            w[0], w[1]
        )))
    }
}

/// The grid's step: finite and positive.
fn check_step(step_nm: f64) -> Result<()> {
    if step_nm.is_finite() && step_nm > 0.0 {
        Ok(())
    } else {
        Err(task_error(format!(
            "step_nm must be positive, got {step_nm}"
        )))
    }
}

/// The most cells a job's grid may have along one side: what a picture may have
/// ([`crate::raster`]), and far beyond what a solve can take.
const MOST_CELLS: f64 = 10_000.0;

/// The fewest cells a job's grid may have along one side: a cross-section needs 3 nodes, and a
/// picture of one cell shows nothing.
const FEWEST_CELLS: f64 = 2.0;

/// A window's cells at the job's step: at most [`MOST_CELLS`], so that a step mistyped a
/// thousand times too small is refused instead of filling the memory, and at least
/// [`FEWEST_CELLS`], so that one a thousand times too large is refused instead of solving a
/// window of one cell.
fn check_cells(name: &str, window: [f64; 2], step_nm: f64) -> Result<()> {
    let exact = (window[1] - window[0]) / (step_nm / 1000.0);
    let cells = exact.round();
    if cells > MOST_CELLS {
        // a number, not a line of three hundred digits
        let shown = if cells < 1e12 {
            cells.to_string()
        } else {
            format!("{cells:.1e}")
        };
        return Err(task_error(format!(
            "{name} at a step of {step_nm} nm is {shown} cells, more than {MOST_CELLS}: take a \
             larger step_nm or a smaller window"
        )));
    }
    if exact < FEWEST_CELLS {
        // to the picometre, without the subtraction's rounding
        let width = window[1] - window[0];
        let shown = if width >= 1e-3 {
            ((width * 1e6).round() / 1e6).to_string()
        } else {
            format!("{width:.1e}")
        };
        return Err(task_error(format!(
            "{name} is {shown} um across, less than {FEWEST_CELLS} cells at a step of {step_nm} \
             nm: take a smaller step_nm or a larger window"
        )));
    }
    Ok(())
}

/// The most modes a modes job may ask for. The eigensolver keeps twice as many vectors of the
/// grid's size and 20 more, and its time grows with them: 500 modes ran six minutes past a
/// one-minute limit before the solve heeded it.
const MOST_MODES: usize = 50;

/// An optional number the run uses as a coordinate: finite when given.
fn check_finite(name: &str, v: Option<f64>) -> Result<()> {
    match v {
        Some(v) if !v.is_finite() => Err(task_error(format!("{name} must be a number, got {v}"))),
        _ => Ok(()),
    }
}

/// Every material of `stack` (its substrate, its cladding, each layer's and what surrounds it)
/// must have data at each of `wavelengths_um`; for a sweep, its two ends, a material's validity
/// being one range.
fn check_materials(stack: &LayerStack, wavelengths_um: &[f64]) -> Result<()> {
    let mut materials = vec![stack.substrate(), stack.cladding()];
    for layer in stack.layers() {
        materials.push(&layer.material);
        materials.push(&layer.background);
    }
    for &w in wavelengths_um {
        let w = Wavelength::um(w)?;
        for m in &materials {
            m.permittivity(w)?;
        }
    }
    Ok(())
}

/// A sweep's range: finite ends that differ, and at least 2 points.
fn check_sweep(from: f64, to: f64, points: usize) -> Result<()> {
    if points < 2 || !(from.is_finite() && to.is_finite()) {
        return Err(task_error("a sweep needs from, to and at least 2 points"));
    }
    if from == to {
        return Err(task_error(format!(
            "a sweep's from and to must differ, got {from} for both"
        )));
    }
    Ok(())
}

impl StructureTask {
    /// What [`check`] and the run both refuse before drawing anything: the windows and the step.
    fn validate(&self) -> Result<()> {
        check_window("x_um", self.x_um)?;
        check_window("y_um", self.y_um)?;
        if let Some(z) = self.z_um {
            check_window("z_um", z)?;
        }
        check_finite("side_y_um", self.side_y_um)?;
        // the side view is cut inside the window, or it shows what isn't drawn
        let side = self.side_y_um.unwrap_or(0.0);
        if !(self.y_um[0]..=self.y_um[1]).contains(&side) {
            return Err(task_error(format!(
                "the side view's cut, side_y_um = {side}{}, is outside y_um [{}, {}]: give a \
                 side_y_um inside it",
                if self.side_y_um.is_some() {
                    ""
                } else {
                    " by default"
                },
                self.y_um[0],
                self.y_um[1]
            )));
        }
        check_step(self.step_nm)?;
        check_cells("x_um", self.x_um, self.step_nm)?;
        check_cells("y_um", self.y_um, self.step_nm)?;
        match self.z_um {
            Some(z) => check_cells("z_um", z, self.step_nm),
            None => Ok(()),
        }
    }
}

impl ModesTask {
    /// The axis the modes travel along: `propagation`, y when it isn't given (an older job).
    fn along(&self) -> Result<Along> {
        match self.propagation.as_deref() {
            None | Some("y") => Ok(Along::Y),
            Some("x") => Ok(Along::X),
            Some(other) => Err(task_error(format!(
                "unknown propagation \"{other}\": x or y"
            ))),
        }
    }

    /// The window across the guide, µm: `y_um` for modes along x, `x_um` along y.
    fn across(&self) -> Result<[f64; 2]> {
        let (window, name) = match self.along()? {
            Along::X => (self.y_um, "y_um"),
            Along::Y => (self.x_um, "x_um"),
        };
        window.ok_or_else(|| {
            task_error(format!(
                "a modes job along {} needs {name}, its window across the guide",
                self.along().map_or("y", Along::name)
            ))
        })
    }

    /// Where the cross-section is cut along the guide, µm: `cut_x_um` or `cut_y_um`, 0 by default.
    fn cut(&self) -> Result<f64> {
        Ok(match self.along()? {
            Along::X => self.cut_x_um,
            Along::Y => self.cut_y_um,
        }
        .unwrap_or(0.0))
    }

    /// What [`check`] and the run both refuse before solving anything: the propagation, the
    /// windows, the step, the cut, the number of modes and the sweep.
    fn validate(&self) -> Result<()> {
        // (none is refused below)
        if let Some(modes) = self.modes
            && modes > MOST_MODES
        {
            return Err(task_error(format!(
                "modes must be at most {MOST_MODES}, got {modes}: the solve's time and memory \
                 grow with the modes asked for, and a guide has a few"
            )));
        }
        let along = self.along()?;
        // the other propagation's fields would be read as nothing: refused, not ignored
        let (window, cut, other) = match along {
            Along::X => (
                ("x_um", self.x_um.is_some()),
                ("cut_y_um", self.cut_y_um.is_some()),
                "y",
            ),
            Along::Y => (
                ("y_um", self.y_um.is_some()),
                ("cut_x_um", self.cut_x_um.is_some()),
                "x",
            ),
        };
        for (name, given) in [window, cut] {
            if given {
                return Err(task_error(format!(
                    "{name} is for a modes job along {other}; this one's modes travel along {} \
                     (propagation = \"{}\")",
                    along.name(),
                    along.name()
                )));
            }
        }
        check_window(
            match along {
                Along::X => "y_um",
                Along::Y => "x_um",
            },
            self.across()?,
        )?;
        if let Some(z) = self.z_um {
            check_window("z_um", z)?;
        }
        check_finite("cut_x_um", self.cut_x_um)?;
        check_finite("cut_y_um", self.cut_y_um)?;
        if self.modes == Some(0) {
            return Err(task_error("modes must be at least 1, got 0"));
        }
        check_step(self.step_nm)?;
        check_cells(
            match along {
                Along::X => "y_um",
                Along::Y => "x_um",
            },
            self.across()?,
            self.step_nm,
        )?;
        if let Some(z) = self.z_um {
            check_cells("z_um", z, self.step_nm)?;
        }
        let Some(sweep) = &self.sweep else {
            return Ok(());
        };
        check_sweep(sweep.from, sweep.to, sweep.points)?;
        match sweep.parameter.as_str() {
            "wavelength" => {
                Wavelength::um(sweep.from)?;
                Wavelength::um(sweep.to)?;
            }
            "width" => {
                let index = sweep.rect.unwrap_or(0);
                if index >= self.rect.len() {
                    // the studio numbers the rectangles from 1, the job from 0: say both
                    return Err(task_error(format!(
                        "the width sweep's rect {index} isn't there: rect counts from 0, so it is \
                         rectangle {} of the job's {}",
                        index + 1,
                        self.rect.len()
                    )));
                }
                if sweep.from <= 0.0 || sweep.to <= 0.0 {
                    return Err(task_error(format!(
                        "a width sweep's widths must be positive, got {} to {}",
                        sweep.from, sweep.to
                    )));
                }
            }
            // a parameter of [task.parameters]: its shapes at each point are checked as drawn
            other if self.parameters.iter().any(|p| p == other) => {
                if sweep.rect.is_some() {
                    return Err(task_error(format!(
                        "the sweep of the parameter \"{other}\" takes no rect: rect is for a \
                         width sweep"
                    )));
                }
            }
            other => {
                return Err(task_error(format!(
                    "unknown sweep parameter \"{other}\": wavelength, width or a parameter of \
                     [task.parameters]"
                )));
            }
        }
        Ok(())
    }
}

/// A named stack: `soi_220`, or `soi` / `nitride` with their core and bottom oxide.
fn named_stack(
    stack: &str,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let thicknesses = || match (core_nm, bottom_oxide_um) {
        (Some(core), Some(oxide)) => Ok((Length::nm(core), Length::um(oxide))),
        _ => Err(task_error(format!(
            "stack \"{stack}\" needs core_nm and bottom_oxide_um"
        ))),
    };
    match stack {
        "soi_220" => Ok(LayerStack::soi_220()),
        "soi" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::soi(core, oxide)
        }
        "nitride" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::nitride(core, oxide)
        }
        other => Err(task_error(format!(
            "unknown stack \"{other}\": soi_220, soi or nitride"
        ))),
    }
}

/// The stack with the shapes drawn on it.
fn draw(
    stack: LayerStack,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
) -> Result<Structure> {
    let mut s = Structure::new(stack);
    for r in rects {
        let shape = Shape::rect(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.size_um[0]),
            Length::um(r.size_um[1]),
        )?;
        s.draw(&r.layer, shape)?;
    }
    for c in circles {
        let shape = Shape::circle(
            Point::um(c.center_um[0], c.center_um[1]),
            Length::um(c.radius_um),
        )?;
        s.draw(&c.layer, shape)?;
    }
    for r in rings {
        let shape = Shape::ring(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.radius_um),
            Length::um(r.width_um),
        )?;
        s.draw(&r.layer, shape)?;
    }
    Ok(s)
}

impl StructureTask {
    fn structure(&self) -> Result<Structure> {
        draw(
            named_stack(&self.stack, self.core_nm, self.bottom_oxide_um)?,
            &self.rect,
            &self.circle,
            &self.ring,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SweepSpec {
    parameter: String,
    from: f64,
    to: f64,
    points: usize,
    /// For a width sweep: which rectangle, in the order given (0 by default).
    rect: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModesTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    /// `"x"` or `"y"` (the default, an older job): the axis the modes travel along.
    propagation: Option<String>,
    /// Along y: the window across the guide.
    x_um: Option<[f64; 2]>,
    /// Along x: the window across the guide.
    y_um: Option<[f64; 2]>,
    z_um: Option<[f64; 2]>,
    /// Along x: where the cross-section is cut.
    cut_x_um: Option<f64>,
    /// Along y: where the cross-section is cut.
    cut_y_um: Option<f64>,
    step_nm: f64,
    modes: Option<usize>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    sweep: Option<SweepSpec>,
    /// The names of `[task.parameters]`, which a sweep may step through: set after reading.
    #[serde(skip)]
    parameters: Vec<String>,
}

/// The axis a modes job's modes travel along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Along {
    X,
    Y,
}

impl Along {
    fn name(self) -> &'static str {
        match self {
            Along::X => "x",
            Along::Y => "y",
        }
    }

    /// The axis across the guide, in the plane.
    fn across(self) -> &'static str {
        match self {
            Along::X => "y",
            Along::Y => "x",
        }
    }
}

/// A modes job's shapes and cut in the frame the cross-section is solved in, where the modes
/// travel along y and x is across the guide. Along y that is the job's own frame; along x it is
/// the job's turned a quarter turn clockwise, (x, y) → (y, −x): the job's y is the frame's x, so
/// the pictures have the job's y across, and its cut x = c is the frame's y = −c. The turn is
/// exact (a swap and a sign), so a guide along x has the modes of the same guide along y.
struct Frame {
    rect: Vec<RectSpec>,
    circle: Vec<CircleSpec>,
    ring: Vec<RingSpec>,
    cut: f64,
}

impl Frame {
    fn new(
        along: Along,
        rects: &[RectSpec],
        circles: &[CircleSpec],
        rings: &[RingSpec],
        cut: f64,
    ) -> Frame {
        let turn = |c: [f64; 2]| match along {
            Along::X => [c[1], -c[0]],
            Along::Y => c,
        };
        Frame {
            rect: rects
                .iter()
                .map(|r| RectSpec {
                    layer: r.layer.clone(),
                    center_um: turn(r.center_um),
                    size_um: match along {
                        Along::X => [r.size_um[1], r.size_um[0]],
                        Along::Y => r.size_um,
                    },
                })
                .collect(),
            circle: circles
                .iter()
                .map(|c| CircleSpec {
                    layer: c.layer.clone(),
                    center_um: turn(c.center_um),
                    radius_um: c.radius_um,
                })
                .collect(),
            ring: rings
                .iter()
                .map(|r| RingSpec {
                    layer: r.layer.clone(),
                    center_um: turn(r.center_um),
                    radius_um: r.radius_um,
                    width_um: r.width_um,
                })
                .collect(),
            cut: match along {
                Along::X => -cut,
                Along::Y => cut,
            },
        }
    }

    /// The frame's structure: the stack with its shapes drawn on it.
    fn draw(&self, stack: LayerStack) -> Result<Structure> {
        draw(stack, &self.rect, &self.circle, &self.ring)
    }
}

/// The cross-section of `s` cut at y over `x` × `z` (µm) on a grid of about `step`, with every
/// shape's edge on the cut and every layer interface on a node, so a sweep over a width varies
/// it smoothly; each cell the permittivity at its centre at `wavelength`.
#[allow(clippy::too_many_arguments)]
fn cross_section(
    s: &Structure,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
    cut_y: f64,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    wavelength: Wavelength,
) -> Result<crate::mode::vector::CrossSection> {
    // every material the cut can meet must know this wavelength
    check_materials(s.stack(), &[wavelength.to_um()])?;
    // the edges the cut crosses, and the layers' interfaces
    let mut x_edges = Vec::new();
    for r in rects {
        if (cut_y - r.center_um[1]).abs() < r.size_um[1] / 2.0 {
            x_edges.extend([
                r.center_um[0] - r.size_um[0] / 2.0,
                r.center_um[0] + r.size_um[0] / 2.0,
            ]);
        }
    }
    let mut circle_edges = |center: [f64; 2], radius: f64| {
        let d = cut_y - center[1];
        if d.abs() < radius {
            let half = (radius * radius - d * d).sqrt();
            x_edges.extend([center[0] - half, center[0] + half]);
        }
    };
    for c in circles {
        circle_edges(c.center_um, c.radius_um);
    }
    for r in rings {
        circle_edges(r.center_um, r.radius_um - r.width_um / 2.0);
        circle_edges(r.center_um, r.radius_um + r.width_um / 2.0);
    }
    let mut z_edges = vec![0.0];
    let mut top = 0.0;
    for layer in s.stack().layers() {
        top += layer.thickness.to_um();
        z_edges.push(top);
    }
    let grid = |a: f64, b: f64, fixed: &[f64]| {
        crate::mode::vector::graded_nodes(a, b, (a, b), step, 0.0, step, fixed)
    };
    let (xs, zs) = (grid(x[0], x[1], &x_edges), grid(z[0], z[1], &z_edges));
    let mut cells = Vec::with_capacity((xs.len() - 1) * (zs.len() - 1));
    for i in 0..xs.len() - 1 {
        let xc = 0.5 * (xs[i] + xs[i + 1]);
        for j in 0..zs.len() - 1 {
            let zc = 0.5 * (zs[j] + zs[j + 1]);
            let eps = s
                .material_at(Point::um(xc, cut_y), Length::um(zc))
                .permittivity(wavelength)
                .unwrap_or(num_complex::Complex64::new(f64::NAN, 0.0));
            cells.push(crate::mode::vector::Permittivity::isotropic(eps));
        }
    }
    crate::mode::vector::CrossSection::new(xs, zs, cells)
}

/// `value(i, j)` of the cross-section's cells as a picture of about `step` per pixel over
/// `x` × `z`, each pixel the cell it falls in (the grid needn't be uniform), divided by its
/// largest magnitude.
fn picture(
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    value: impl Fn(usize, usize) -> f64,
) -> Raster {
    let (nx, ny) = (
        ((x[1] - x[0]) / step).round().max(1.0) as usize,
        ((z[1] - z[0]) / step).round().max(1.0) as usize,
    );
    // the cell holding a coordinate, along one axis
    let cell =
        |nodes: &[f64], v: f64| nodes.partition_point(|&n| n <= v).clamp(1, nodes.len() - 1) - 1;
    let mut values = vec![0.0f64; nx * ny];
    for j in 0..ny {
        let zv = z[0] + (j as f64 + 0.5) * (z[1] - z[0]) / ny as f64;
        let cj = cell(cs.y(), zv);
        for i in 0..nx {
            let xv = x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
            values[j * nx + i] = value(cell(cs.x(), xv), cj);
        }
    }
    let peak = values
        .iter()
        .map(|v| v.abs())
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    Raster {
        nx,
        ny,
        x0: x[0],
        x1: x[1],
        y0: z[0],
        y1: z[1],
        values: values.iter().map(|v| (v / peak) as f32).collect(),
    }
}

/// A mode's |E|² as a [`picture`], its largest value 1.
fn intensity(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
) -> Raster {
    picture(cs, x, z, step, |i, j| {
        fields.e(i, j).iter().map(|c| c.norm_sqr()).sum()
    })
}

/// A mode's signed field as [`Event::ModeField`] records it: the name of the transverse
/// component of E with the larger share of ∫|E|² dA, and the real part of that component, its
/// global phase fixed to make it real and positive at its largest magnitude, as a [`picture`].
fn signed_field(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    across: &str,
) -> (String, Raster) {
    let (ni, nj) = (fields.x().len(), fields.y().len());
    let mut share = [0.0f64; 2];
    for i in 0..ni {
        for j in 0..nj {
            let (e, area) = (fields.e(i, j), fields.area[i * nj + j]);
            share[0] += e[0].norm_sqr() * area;
            share[1] += e[1].norm_sqr() * area;
        }
    }
    // the solver's (x, y) cross-section is the job's (across, z)
    let (c, name) = if share[0] >= share[1] {
        (0, format!("E{across}"))
    } else {
        (1, "Ez".to_owned())
    };
    let mut peak = num_complex::Complex64::new(0.0, 0.0);
    for i in 0..ni {
        for j in 0..nj {
            let v = fields.e(i, j)[c];
            if v.norm_sqr() > peak.norm_sqr() {
                peak = v;
            }
        }
    }
    // e^(−i arg) of the peak: it, and with it the whole component, made real
    let turn = if peak.norm() > 0.0 {
        peak.conj() / peak.norm()
    } else {
        num_complex::Complex64::new(1.0, 0.0)
    };
    let raster = picture(cs, x, z, step, |i, j| (fields.e(i, j)[c] * turn).re);
    (name, raster)
}

fn reason(stop: &Stop) -> Option<String> {
    stop.reason().map(|r| match r {
        StopReason::Timeout => "timeout".to_owned(),
        StopReason::Requested => "requested".to_owned(),
    })
}

/// Runs `job` into `run`, recording its [`Event`]s, and stops early when `stop` says so.
///
/// # Errors
///
/// [`Error::Parse`] for a task that isn't a known kind or doesn't fit it, the errors of the
/// task itself (an invalid shape, a wavelength a material has no data for), and
/// [`Error::Io`] if the record can't be written.
pub fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?
        .to_owned();
    check_direct(job, &kind)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: kind.clone(),
    })?;
    match kind.as_str() {
        "structure" => structure(job, run, stop)?,
        "modes" => modes(job, run, stop)?,
        "fdfd" => fdfd::run(job, run, stop)?,
        "fdtd" => fdtd::run(job, run, stop)?,
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

/// Checks `job` without running it: its kind is known, its task fits that kind, the stack is
/// known, the shapes are valid, the layer is in the stack, the wavelength is valid, the windows
/// run from smaller to larger numbers, the step is positive, a sweep has distinct ends, at least
/// 2 points and a parameter the kind sweeps, the coordinates are numbers, and every material of
/// the stack has data at the wavelength and at both ends of a wavelength sweep; for an
/// `"fdfd"` job, the polarization, that the PMLs leave room, and each port's place: a known
/// side, a column inside the window and clear of the PMLs, a window of rows on the grid. No
/// window, the default height (1 µm below and above the layer) included, may be more than
/// 10 000 cells of the step across or fewer than 2; a modes job asks for at least one mode, and
/// a structure job's side view is cut inside its window. What only a solve finds (a port
/// whose column guides no mode, modes that don't converge) is left to the run.
///
/// # Errors
///
/// The errors [`execute`] would return for those, before recording anything.
pub fn check(job: &Job) -> Result<()> {
    check_inner(job)
}

/// The job's direct solver: one that is registered and available, and asked of a job that has
/// a direct solve to give it. What [`check`] and the run both refuse, in the same words.
fn check_direct(job: &Job, kind: &str) -> Result<()> {
    crate::backend::direct(job.direct())?;
    if kind != "fdfd" && *job.direct() != crate::backend::Choice::Auto {
        return Err(task_error(format!(
            "[solver] direct = \"{}\" is for fdfd jobs: a {kind} job has no direct solve to \
             give a backend",
            job.direct()
        )));
    }
    Ok(())
}

fn check_inner(job: &Job) -> Result<()> {
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?;
    check_direct(job, kind)?;
    let mut wavelengths = Vec::new();
    let (s, layer, wavelength_um) = match kind {
        "structure" => {
            let task: StructureTask = params::task(job)?;
            task.validate()?;
            let s = task.structure()?;
            window_z(task.z_um, &task.layer, &s, task.step_nm)?;
            (s, task.layer, task.wavelength_um)
        }
        "modes" => {
            let task = modes_task(job, None)?;
            task.validate()?;
            let stack = named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            let s = draw(stack, &task.rect, &task.circle, &task.ring)?;
            let z = modes_z(&task, &s)?;
            modes_guide(&task, &s, z)?;
            if let Some(sw) = task.sweep.iter().find(|sw| sw.parameter == "wavelength") {
                wavelengths.extend([sw.from, sw.to]);
            }
            // a parameter's sweep: its shapes at both ends must be shapes
            if let Some(sw) = task
                .sweep
                .iter()
                .find(|sw| task.parameters.contains(&sw.parameter))
            {
                for end in [sw.from, sw.to] {
                    let at = modes_task(job, Some((&sw.parameter, end)))?;
                    let stack = named_stack(&at.stack, at.core_nm, at.bottom_oxide_um)?;
                    draw(stack, &at.rect, &at.circle, &at.ring).map_err(|e| {
                        task_error(format!("at the sweep's {} = {end}: {e}", sw.parameter))
                    })?;
                }
            }
            (s, task.layer, task.wavelength_um)
        }
        "fdfd" => return fdfd::check(job),
        "fdtd" => return fdtd::check(job),
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    };
    s.stack()
        .layer(&layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    wavelengths.insert(0, wavelength_um);
    check_materials(s.stack(), &wavelengths)
}

fn structure(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: StructureTask = params::task(job)?;
    task.validate()?;
    let s = task.structure()?;
    let lam = Wavelength::um(task.wavelength_um)?;
    run.record(&structure_scene(&task, &s)?)?;
    let step = Length::nm(task.step_nm);
    let x = (Length::um(task.x_um[0]), Length::um(task.x_um[1]));
    let y = (Length::um(task.y_um[0]), Length::um(task.y_um[1]));
    let top = s.top_view(&task.layer, x, y, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("top view of layer {}", task.layer),
        axes: ["x".into(), "y".into()],
        wavelength_um: lam.to_um(),
        raster: top,
    })?;
    if stop.reason().is_some() {
        return Ok(());
    }
    let [z0, z1] = window_z(task.z_um, &task.layer, &s, task.step_nm)?;
    let z = (Length::um(z0), Length::um(z1));
    let side_y = Length::um(task.side_y_um.unwrap_or(0.0));
    let side = s.side_view(side_y, x, z, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("side view at y = {} um", side_y.to_um()),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: side,
    })
}

/// A structure job's scene: the window, 1 µm above and below the layer seen from above unless
/// `z_um` says otherwise.
fn structure_scene(task: &StructureTask, s: &Structure) -> Result<Event> {
    let z = window_z(task.z_um, &task.layer, s, task.step_nm)?;
    scene(
        s,
        task.x_um,
        task.y_um,
        z,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// What a modes job needs for there to be a guide in its cross-section: a z window that
/// reaches its layer, and a cut that meets a shape within the window across. Without either
/// the solver would find the modes of bare cladding in a box, and call them the job's.
fn modes_guide(task: &ModesTask, s: &Structure, z: [f64; 2]) -> Result<()> {
    let (_, bottom, top) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    let (bottom, top) = (bottom.to_um(), top.to_um());
    if z[1] <= bottom || z[0] >= top {
        return Err(task_error(format!(
            "z_um, {} to {}, doesn't reach the layer {} ({bottom} to {top} um): the \
             cross-section would hold no guide",
            z[0], z[1], task.layer
        )));
    }
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    // along the cut, across the window, a quarter of the step apart
    let step = task.step_nm / 4000.0;
    let count = ((across[1] - across[0]) / step).ceil() as usize;
    let meets = (0..=count).any(|k| {
        let a = (across[0] + k as f64 * step).min(across[1]);
        let p = match along {
            Along::X => Point::um(cut, a),
            Along::Y => Point::um(a, cut),
        };
        s.stack()
            .layers()
            .iter()
            .any(|l| s.shapes(&l.name).iter().any(|shape| shape.contains(p)))
    });
    if !meets {
        return Err(task_error(format!(
            "the cross-section at {} = {cut} um meets no shape between {} = {} and {} um: there \
             is no guide there to solve",
            along.name(),
            along.across(),
            across[0],
            across[1]
        )));
    }
    Ok(())
}

/// A modes job's window height: `z_um`, else 1 µm below and above its layer.
fn modes_z(task: &ModesTask, s: &Structure) -> Result<[f64; 2]> {
    window_z(task.z_um, &task.layer, s, task.step_nm)
}

/// A job's window height: `z_um`, else 1 µm below and above its layer, of as many cells as
/// [`check_cells`] allows a window the job gives (a layer a metre thick is refused here, not
/// as a picture too large to draw).
fn window_z(z_um: Option<[f64; 2]>, layer: &str, s: &Structure, step_nm: f64) -> Result<[f64; 2]> {
    if let Some(z) = z_um {
        // (checked with the job's other windows)
        return Ok(z);
    }
    let (_, bottom, top) = s
        .stack()
        .layer(layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    let z = [bottom.to_um() - 1.0, top.to_um() + 1.0];
    check_cells(
        "the height, 1 um below and above the layer as z_um isn't given,",
        z,
        step_nm,
    )?;
    Ok(z)
}

/// A modes job's scene: a block behind the cut along the guide, as deep as the window across it
/// is wide.
fn modes_scene(task: &ModesTask, s: &Structure) -> Result<Event> {
    let cut = task.cut()?;
    let across = task.across()?;
    let behind = [cut - (across[1] - across[0]), cut];
    let (x, y) = match task.along()? {
        Along::X => (behind, across),
        Along::Y => (across, behind),
    };
    scene(
        s,
        x,
        y,
        modes_z(task, s)?,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// The scene `job`'s run records first, its structure over its window, without running it: what
/// the studio draws of a job before it runs. Checked as [`check`] does.
///
/// # Errors
///
/// The errors of [`check`].
pub fn preview(job: &Job) -> Result<Event> {
    check(job)?;
    match job.task().get("kind").and_then(|k| k.as_str()) {
        Some("structure") => {
            let task: StructureTask = params::task(job)?;
            structure_scene(&task, &task.structure()?)
        }
        Some("modes") => {
            let task = modes_task(job, None)?;
            let stack = named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            modes_scene(&task, &draw(stack, &task.rect, &task.circle, &task.ring)?)
        }
        Some("fdtd") => fdtd::preview(job),
        _ => fdfd::preview(job),
    }
}

/// A modes job's task, its parameters and expressions resolved, with `set` (a parameter and its
/// value) if given.
fn modes_task(job: &Job, set: Option<(&str, f64)>) -> Result<ModesTask> {
    let mut task: ModesTask = params::at(job.task(), set)?;
    task.parameters = params::names(job.task());
    Ok(task)
}

fn modes(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task = modes_task(job, None)?;
    task.validate()?;
    let stack = || named_stack(&task.stack, task.core_nm, task.bottom_oxide_um);
    // (validated: at least 1 and at most MOST_MODES)
    let count = task.modes.unwrap_or(2);
    let lam = Wavelength::um(task.wavelength_um)?;
    let step = task.step_nm / 1000.0;
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    // the pictures' axes, across the guide and up, and how they name the cut
    let axes = || [along.across().to_owned(), "z".to_owned()];
    let view = format!("cross-section at {} = {cut} um", along.name());
    let s = draw(stack()?, &task.rect, &task.circle, &task.ring)?;
    let z = modes_z(&task, &s)?;
    modes_guide(&task, &s, z)?;
    run.record(&modes_scene(&task, &s)?)?;
    if task.propagation.is_some() {
        run.record(&Event::Cut {
            normal: along.name().to_owned(),
            at_um: cut,
        })?;
    }
    // the structure the cross-section is solved in, the modes travelling along its y
    let frame = Frame::new(along, &task.rect, &task.circle, &task.ring, cut);
    let fs = frame.draw(stack()?)?;
    // the cross-section, as the structure job shows a side view
    run.record(&Event::Permittivity {
        view: view.clone(),
        axes: axes(),
        wavelength_um: lam.to_um(),
        raster: fs.side_view(
            Length::um(frame.cut),
            (Length::um(across[0]), Length::um(across[1])),
            (Length::um(z[0]), Length::um(z[1])),
            Length::um(step),
            lam,
        )?,
    })?;
    let cs = cross_section(
        &fs,
        &frame.rect,
        &frame.circle,
        &frame.ring,
        frame.cut,
        across,
        z,
        step,
        lam,
    )?;
    run.record(&Event::Solver {
        module: "mode::vector".into(),
        cells: [cs.x().len() - 1, cs.y().len() - 1],
        step_um: step,
        unknowns: cs.unknowns(),
        details: vec![
            ["modes asked for".into(), count.to_string()],
            [
                "eigen-solver".into(),
                "shift-and-invert Arnoldi about the highest index, sparse LU of the shifted \
                 matrix; an eigenpair is accepted at a residual of 1e-9"
                    .into(),
            ],
            [
                "nodes".into(),
                "on every shape's edge and layer interface, cells of about the step between \
                 them"
                    .into(),
            ],
        ],
    })?;
    // the solve itself heeds the stop and the time limit, at each of its steps
    let stopped = || stop.reason().is_some();
    let Some(found) = crate::mode::vector::modes_until(&cs, lam, count, None, stopped)? else {
        return Ok(());
    };
    let mut sorted: Vec<_> = found.iter().collect();
    sorted.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
    for (k, m) in sorted.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let n = m.effective_index();
        let fields = m.fields(&cs)?;
        let label = format!("mode {} of {}", k + 1, sorted.len());
        run.record(&Event::Mode {
            label: label.clone(),
            wavelength_um: lam.to_um(),
            effective_index: [n.re, n.im],
            te_fraction: m.te_fraction(),
            intensity: intensity(&fields, &cs, across, z, step),
            cut_y_um: cut,
        })?;
        let (component, values) = signed_field(&fields, &cs, across, z, step, along.across());
        run.record(&Event::ModeField {
            label,
            wavelength_um: lam.to_um(),
            component,
            values,
        })?;
    }
    for (k, m) in sorted.iter().enumerate() {
        run.record(&Event::SolveError {
            point: None,
            value: lam.to_um(),
            measure: format!("eigen-residual, mode {} of {}", k + 1, sorted.len()),
            error: m.residual(&cs)?,
        })?;
    }
    // (validated above: at least 2 points, finite and distinct ends, a known parameter)
    let Some(sweep) = &task.sweep else {
        return Ok(());
    };
    run.record(&Event::Sweep {
        parameter: sweep.parameter.clone(),
        from: sweep.from,
        to: sweep.to,
        points: sweep.points,
    })?;
    let point = |k: usize| -> Result<Option<Vec<Event>>> {
        if stop.reason().is_some() {
            return Ok(None);
        }
        let mut events = Vec::new();
        let value = sweep.from + (sweep.to - sweep.from) * k as f64 / (sweep.points - 1) as f64;
        // the point's shapes, for a width or a parameter's sweep
        let mut shapes = None;
        let (cs, w) = match sweep.parameter.as_str() {
            "wavelength" => {
                let w = Wavelength::um(value)?;
                (
                    cross_section(
                        &fs,
                        &frame.rect,
                        &frame.circle,
                        &frame.ring,
                        frame.cut,
                        across,
                        z,
                        step,
                        w,
                    )?,
                    w,
                )
            }
            other => {
                // the swept rectangle at the point's width, or the shapes at the parameter's value
                let at: ModesTask;
                let (rects, circles, rings): (Vec<RectSpec>, &[CircleSpec], &[RingSpec]) = if other
                    == "width"
                {
                    let index = sweep.rect.unwrap_or(0);
                    let mut rects: Vec<RectSpec> = task.rect.iter().map(RectSpec::clone).collect();
                    let r = rects.get_mut(index).ok_or_else(|| {
                        task_error(format!("the width sweep's rect {index} isn't there"))
                    })?;
                    // its size across the guide
                    match along {
                        Along::X => r.size_um[1] = value,
                        Along::Y => r.size_um[0] = value,
                    }
                    (rects, &task.circle, &task.ring)
                } else if task.parameters.iter().any(|p| p == other) {
                    at = modes_task(job, Some((other, value)))?;
                    (at.rect.clone(), &at.circle, &at.ring)
                } else {
                    return Err(task_error(format!(
                        "unknown sweep parameter \"{other}\": wavelength, width or a \
                             parameter of [task.parameters]"
                    )));
                };
                let swept = draw(stack()?, &rects, circles, rings)?;
                let swept_frame = Frame::new(along, &rects, circles, rings, cut);
                let swept_fs = swept_frame.draw(stack()?)?;
                let picture = swept_fs.side_view(
                    Length::um(swept_frame.cut),
                    (Length::um(across[0]), Length::um(across[1])),
                    (Length::um(z[0]), Length::um(z[1])),
                    Length::um(2.0 * step),
                    lam,
                )?;
                shapes = Some((scene_shapes(&swept), picture));
                (
                    cross_section(
                        &swept_fs,
                        &swept_frame.rect,
                        &swept_frame.circle,
                        &swept_frame.ring,
                        swept_frame.cut,
                        across,
                        z,
                        step,
                        lam,
                    )?,
                    lam,
                )
            }
        };
        let Some(mut found) = crate::mode::vector::modes_until(&cs, w, count, None, stopped)?
        else {
            return Ok(None);
        };
        found.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
        events.push(Event::SweepPoint {
            parameter: sweep.parameter.clone(),
            value,
            wavelength_um: w.to_um(),
            effective_indices: found
                .iter()
                .map(|m| [m.effective_index().re, m.effective_index().im])
                .collect(),
            te_fractions: found
                .iter()
                .map(crate::mode::vector::VectorMode::te_fraction)
                .collect(),
        });
        // the point's pictures, coarser than the job's own: twice the step, three decimals
        let coarse = |mut r: Raster| {
            for v in &mut r.values {
                // (+ 0.0 makes −0 zero)
                *v = (*v * 1000.0).round() / 1000.0 + 0.0;
            }
            r
        };
        if let Some((shapes, picture)) = shapes {
            events.push(Event::SweepShapes {
                point: k,
                value,
                shapes,
            });
            events.push(Event::SweepPermittivity {
                point: k,
                value,
                view: view.clone(),
                axes: axes(),
                wavelength_um: w.to_um(),
                raster: coarse(picture),
            });
        }
        for (rank, m) in found.iter().enumerate() {
            if stop.reason().is_some() {
                return Ok(None);
            }
            let fields = m.fields(&cs)?;
            let (component, field) =
                signed_field(&fields, &cs, across, z, 2.0 * step, along.across());
            let n = m.effective_index();
            events.push(Event::SweepMode {
                point: k,
                value,
                label: format!("mode {} of {}", rank + 1, found.len()),
                wavelength_um: w.to_um(),
                effective_index: [n.re, n.im],
                te_fraction: m.te_fraction(),
                intensity: coarse(intensity(&fields, &cs, across, z, 2.0 * step)),
                component,
                field: coarse(field),
                cut_y_um: cut,
            });
        }
        for (rank, m) in found.iter().enumerate() {
            events.push(Event::SolveError {
                point: Some(k),
                value,
                measure: format!("eigen-residual, mode {} of {}", rank + 1, found.len()),
                error: m.residual(&cs)?,
            });
        }
        Ok(Some(events))
    };
    // a point takes about 6 kB per unknown of its cross-section (measured: 1.45 GB at 253 k
    // unknowns, examples/strip_waveguide.rs on its 5 nm grid)
    let batch = in_flight(cs.unknowns() as f64 * 6e3);
    sweep_points(run, stop, 0..sweep.points, batch, point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::replay;
    use std::path::PathBuf;

    const JOB: &str = r#"
name = "strip"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
y_um = [-1.0, 1.0]
step_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [2.0, 0.5]
"#;

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp(tag: &str) -> TempDir {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-job-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TempDir(dir)
    }

    #[test]
    fn a_structure_job_records_a_top_and_a_side_view() {
        let root = temp("views");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        assert_eq!(events.len(), 5);
        assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "structure"));
        let Event::Scene { layers, shapes, .. } = &events[1] else {
            panic!("{:?}", events[1])
        };
        // soi_220: the buried oxide and the silicon layer; the strip a rectangle on it
        assert_eq!(layers.len(), 2);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].outline.len(), 4);
        assert!(layers[1].material.eps > 11.0 && layers[1].background.eps < 2.2);
        let Event::Permittivity { raster, axes, .. } = &events[2] else {
            panic!("{:?}", events[2])
        };
        assert_eq!(axes, &["x".to_owned(), "y".to_owned()]);
        assert_eq!((raster.nx, raster.ny), (40, 40));
        let Event::Permittivity { raster, axes, .. } = &events[3] else {
            panic!("{:?}", events[3])
        };
        assert_eq!(axes[1], "z");
        // the default cut spans 1 um below and above the 220 nm layer
        assert!((raster.y1 - raster.y0 - 2.22).abs() < 1e-9);
        assert!(matches!(&events[4], Event::Finished { stopped: None, .. }));
    }

    #[test]
    fn a_stopped_job_says_why() {
        let root = temp("stopped");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request();
        execute(&job, &mut run, &stop).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the scene and the top view, then it stops before the side view
        assert_eq!(events.len(), 4);
        assert!(matches!(
            &events[3],
            Event::Finished { stopped: Some(r), .. } if r == "requested"
        ));
    }

    const MODES: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [0.5, 10.0]
"#;

    #[test]
    fn a_modes_job_records_the_strips_modes() {
        let root = temp("modes");
        let job = Job::parse(MODES).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // started, the scene, the cross-section, two modes and their fields, finished
        assert_eq!(events.len(), 8, "{events:?}");
        let Event::Mode {
            effective_index,
            te_fraction,
            intensity,
            ..
        } = &events[3]
        else {
            panic!("{:?}", events[3])
        };
        // the 500 x 220 nm strip's TE-like mode, first; its field peaks in the core
        assert!(
            effective_index[0] > 2.3 && effective_index[0] < 2.6,
            "{effective_index:?}"
        );
        assert!(*te_fraction > 0.9);
        let (lo, hi) = intensity.range();
        assert!(lo >= 0.0 && (hi - 1.0).abs() < 1e-6);
        // its signed field: E_x, of one sign, on the same grid, peaking where |E|² does
        let Event::ModeField {
            label,
            component,
            values,
            ..
        } = &events[4]
        else {
            panic!("{:?}", events[4])
        };
        assert_eq!(label, "mode 1 of 2");
        assert_eq!(component, "Ex");
        assert_eq!((values.nx, values.ny), (intensity.nx, intensity.ny));
        assert!((values.range().1 - 1.0).abs() < 1e-6);
        // of one sign, but for a cell or two at the core's corners, where E_x is singular
        for j in 0..values.ny {
            for i in 0..values.nx {
                let v = values.at(i, j);
                let x = values.x0 + (i as f64 + 0.5) * (values.x1 - values.x0) / values.nx as f64;
                let z = values.y0 + (j as f64 + 0.5) * (values.y1 - values.y0) / values.ny as f64;
                let corner = [-0.25, 0.25]
                    .iter()
                    .any(|&cx| [2.0, 2.22].iter().any(|&cz| (x - cx).hypot(z - cz) < 0.06));
                assert!(v > -0.02 || corner, "{v} at ({x}, {z})");
            }
        }
        let top = values.values.iter().position(|&v| v == 1.0).unwrap();
        assert!(intensity.values[top] > 0.8, "{}", intensity.values[top]);
        // E_x is most of |E|² in the fundamental TE mode: its square follows |E|² closely
        let squares: Vec<f64> = values.values.iter().map(|&v| f64::from(v * v)).collect();
        let total: Vec<f64> = intensity.values.iter().map(|&v| f64::from(v)).collect();
        let mean = |a: &[f64]| a.iter().sum::<f64>() / a.len() as f64;
        let (ma, mb) = (mean(&squares), mean(&total));
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for (a, b) in squares.iter().zip(&total) {
            ab += (a - ma) * (b - mb);
            aa += (a - ma) * (a - ma);
            bb += (b - mb) * (b - mb);
        }
        let correlation = ab / (aa * bb).sqrt();
        assert!(correlation > 0.95, "{correlation}");
        let Event::Mode {
            effective_index: second,
            te_fraction,
            ..
        } = &events[5]
        else {
            panic!("{:?}", events[5])
        };
        assert!(second[0] < effective_index[0] && *te_fraction < 0.1);
        // the TM-like mode's field is mostly vertical
        assert!(matches!(&events[6], Event::ModeField { component, .. } if component == "Ez"));
    }

    /// MODES, its modes travelling along x: the same strip turned a quarter turn.
    #[test]
    fn a_modes_job_finds_eight_modes() {
        // a strip guides two or three modes; the ones after them are the window's, close
        // together. Asking for six or more failed ("didn't converge in 20 restarts") while
        // each restart of the eigensolver kept a Krylov space of one size
        let events = run_events("modes-eight", &MODES.replace("modes = 2", "modes = 8"));
        let indices: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    effective_index, ..
                } => Some(effective_index[0]),
                _ => None,
            })
            .collect();
        assert_eq!(indices.len(), 8, "{indices:?}");
        // from the highest index down, each a different mode, the first two the strip's own
        assert!(
            indices.windows(2).all(|p| p[0] - p[1] > 1e-9),
            "{indices:?}"
        );
        let two = run_events("modes-eight-two", MODES);
        let first: Vec<f64> = two
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    effective_index, ..
                } => Some(effective_index[0]),
                _ => None,
            })
            .collect();
        assert_eq!(first.len(), 2);
        for (a, b) in first.iter().zip(&indices) {
            assert!((a - b).abs() < 1e-8, "{a} vs {b}");
        }
    }

    const MODES_X: &str = r#"
name = "strip-modes-x"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]
"#;

    fn run_events(tag: &str, text: &str) -> Vec<Event> {
        let root = temp(tag);
        let job = Job::parse(text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        replay(run.dir()).unwrap()
    }

    /// Every mode's effective index, TE fraction and pictures, in the order recorded.
    fn modes_of(events: &[Event]) -> Vec<([f64; 2], f64, Raster, String, Raster)> {
        let mut out = Vec::new();
        for (k, e) in events.iter().enumerate() {
            if let Event::Mode {
                effective_index,
                te_fraction,
                intensity,
                ..
            } = e
            {
                let Event::ModeField {
                    component, values, ..
                } = &events[k + 1]
                else {
                    panic!("{:?}", events[k + 1])
                };
                out.push((
                    *effective_index,
                    *te_fraction,
                    intensity.clone(),
                    component.clone(),
                    values.clone(),
                ));
            }
        }
        out
    }

    #[test]
    fn a_guide_along_x_has_the_modes_of_the_same_guide_along_y() {
        // an asymmetric pair of strips, a ring off to one side, a cut off the middle; along y it
        // is the same structure turned a quarter turn: (x, y) along x is (y, −x) along y
        let along_x = r#"
name = "pair-x"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.2, 1.0]
cut_x_um = 0.7
step_nm = 40.0
modes = 3

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.1]
size_um = [10.0, 0.5]

[[task.rect]]
layer = "Si"
center_um = [0.5, -0.55]
size_um = [6.0, 0.3]

[[task.ring]]
layer = "Si"
center_um = [2.5, 0.0]
radius_um = 2.0
width_um = 0.3
"#;
        let along_y = r#"
name = "pair-y"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.2, 1.0]
cut_y_um = -0.7
step_nm = 40.0
modes = 3

[[task.rect]]
layer = "Si"
center_um = [0.1, 0.0]
size_um = [0.5, 10.0]

[[task.rect]]
layer = "Si"
center_um = [-0.55, -0.5]
size_um = [0.3, 6.0]

[[task.ring]]
layer = "Si"
center_um = [0.0, -2.5]
radius_um = 2.0
width_um = 0.3
"#;
        let (x, y) = (
            run_events("along-x", along_x),
            run_events("along-y", along_y),
        );
        let (mx, my) = (modes_of(&x), modes_of(&y));
        assert_eq!(mx.len(), 3);
        assert_eq!(my.len(), 3);
        for (a, b) in mx.iter().zip(&my) {
            for c in 0..2 {
                assert!((a.0[c] - b.0[c]).abs() < 1e-12, "{:?} {:?}", a.0, b.0);
            }
            assert!((a.1 - b.1).abs() < 1e-12);
            // the same pictures, across the guide (y along x, x along y) and up
            assert_eq!(a.2, b.2);
            assert_eq!(a.4, b.4);
            // the component across the guide is E_y along x and E_x along y
            match a.3.as_str() {
                "Ey" => assert_eq!(b.3, "Ex"),
                other => assert_eq!((other, b.3.as_str()), ("Ez", "Ez")),
            }
        }
        // the cut: named along x, where the job says; the pictures' axes and the scene follow
        assert!(x.iter().any(|e| matches!(
            e,
            Event::Cut { normal, at_um } if normal == "x" && *at_um == 0.7
        )));
        assert!(!y.iter().any(|e| matches!(e, Event::Cut { .. })));
        assert!(x.iter().any(|e| matches!(
            e,
            Event::Mode { cut_y_um, .. } if *cut_y_um == 0.7
        )));
        let Some(Event::Permittivity { view, axes, .. }) =
            x.iter().find(|e| matches!(e, Event::Permittivity { .. }))
        else {
            panic!("no cross-section")
        };
        assert_eq!(view, "cross-section at x = 0.7 um");
        assert_eq!(axes, &["y".to_owned(), "z".to_owned()]);
        let Event::Scene { x_um, y_um, .. } = &x[1] else {
            panic!("{:?}", x[1])
        };
        // a block behind the cut along x, as deep as the window across is wide
        assert!(
            (x_um[0] - (0.7 - 2.2)).abs() < 1e-12 && x_um[1] == 0.7,
            "{x_um:?}"
        );
        assert_eq!(y_um, &[-1.2, 1.0]);
    }

    #[test]
    fn an_older_job_runs_as_before() {
        // a job without `propagation` is along y, as it always was: naming it changes nothing
        // but the cut it records
        let old = run_events("old", MODES);
        let named = run_events(
            "named-y",
            &MODES.replace(
                "layer = \"Si\"\nx_um",
                "layer = \"Si\"\npropagation = \"y\"\nx_um",
            ),
        );
        assert!(!old.iter().any(|e| matches!(e, Event::Cut { .. })));
        let same = |events: &[Event]| -> Vec<Event> {
            events
                .iter()
                .filter(|e| !matches!(e, Event::Cut { .. } | Event::Finished { .. }))
                .cloned()
                .collect()
        };
        assert_eq!(same(&old), same(&named));
        assert!(named.iter().any(|e| matches!(
            e,
            Event::Cut { normal, at_um } if normal == "y" && *at_um == 0.0
        )));
        // and the pictures name their axes as before
        assert!(old.iter().any(|e| matches!(
            e,
            Event::Permittivity { view, axes, .. }
                if view == "cross-section at y = 0 um" && axes[0] == "x"
        )));
        assert!(
            old.iter()
                .any(|e| matches!(e, Event::ModeField { component, .. } if component == "Ex"))
        );
    }

    #[test]
    fn the_strip_jobs_along_x_find_what_they_found_along_y() {
        // jobs/strip-modes.toml and jobs/strip-width-sweep.toml as they were before light
        // travelled along x, and their effective indices then (0.4.0, this machine's last
        // digits aside): the jobs along x find the same
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        let file = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
        let older = |text: &str| {
            text.replace("propagation = \"x\"", "")
                .replace("y_um = [", "x_um = [")
                .replace("size_um = [10.0, 0.5]", "size_um = [0.5, 10.0]")
        };
        for (name, first, last) in [
            (
                "strip-modes.toml",
                [2.4497971939172727, 1.7853122219332593],
                [2.394612987693227, 1.7260069902533284],
            ),
            (
                "strip-width-sweep.toml",
                [2.4497972033509092, 1.7853414527523084],
                [2.68762994950837, 2.1757624280681673],
            ),
        ] {
            let text = file(name);
            assert!(text.contains("propagation = \"x\""), "{name}");
            // a short sweep: its first and last points only
            let short = |t: String| t.replace("points = 11", "points = 2");
            for (tag, events) in [
                ("now", run_events("jobs-now", &short(text.clone()))),
                ("then", run_events("jobs-then", &short(older(&text)))),
            ] {
                let indices: Vec<f64> = modes_of(&events).iter().map(|m| m.0[0]).collect();
                let points: Vec<Vec<f64>> = events
                    .iter()
                    .filter_map(|e| match e {
                        Event::SweepPoint {
                            effective_indices, ..
                        } => Some(effective_indices.iter().map(|n| n[0]).collect()),
                        _ => None,
                    })
                    .collect();
                let close = |a: &[f64], b: &[f64]| {
                    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
                };
                assert!(close(&indices, &first), "{name}, {tag}: {indices:?}");
                assert!(close(&points[1], &last), "{name}, {tag}: {points:?}");
            }
        }
    }

    #[test]
    fn a_width_sweep_along_x_widens_the_strip_along_y() {
        let text = format!(
            "{MODES_X}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 2\n"
        );
        let events = run_events("sweep-x", &text.replace("modes = 2", "modes = 1"));
        let spans: Vec<(f64, f64, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepShapes { value, shapes, .. } => {
                    let span = |c: usize| {
                        let v = shapes[0].outline.iter().map(|p| p[c]);
                        v.clone().fold(f64::MIN, f64::max) - v.fold(f64::MAX, f64::min)
                    };
                    Some((*value, span(0), span(1)))
                }
                _ => None,
            })
            .collect();
        assert_eq!(spans.len(), 2);
        for (value, along, across) in spans {
            // still 10 µm long, its width across the guide the sweep's
            assert!((along - 10.0).abs() < 1e-9 && (across - value).abs() < 1e-9);
        }
        let points: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPoint {
                    effective_indices, ..
                } => Some(effective_indices[0][0]),
                _ => None,
            })
            .collect();
        assert!(points[1] > points[0], "{points:?}");
        assert!(events.iter().any(|e| matches!(
            e,
            Event::SweepPermittivity { view, axes, .. }
                if view == "cross-section at x = 0 um" && axes[0] == "y"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::SweepMode { component, .. } if component == "Ey"
        )));
    }

    #[test]
    fn a_higher_order_modes_field_changes_sign_across_the_core() {
        let root = temp("te1");
        // a 1 µm strip guides TE0, TE1 and TM0; TE1 is odd in x
        let text = MODES
            .replace("size_um = [0.5, 10.0]", "size_um = [1.0, 10.0]")
            .replace("x_um = [-1.0, 1.0]", "x_um = [-1.2, 1.2]")
            .replace("step_nm = 25.0", "step_nm = 40.0")
            .replace("modes = 2", "modes = 3");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let fields: Vec<(&String, &Raster)> = events
            .iter()
            .filter_map(|e| match e {
                Event::ModeField {
                    component, values, ..
                } => Some((component, values)),
                _ => None,
            })
            .collect();
        assert_eq!(fields.len(), 3);
        // E_x along the row through the core's middle, a quarter of the width either side
        let across = |r: &Raster, x: f64| {
            let i = (((x - r.x0) / (r.x1 - r.x0)) * r.nx as f64) as usize;
            // the silicon is 2 to 2.22 µm up, on the buried oxide
            let j = (((2.11 - r.y0) / (r.y1 - r.y0)) * r.ny as f64) as usize;
            r.values[j * r.nx + i]
        };
        let (_, te0) = fields[0];
        assert!(across(te0, -0.25) > 0.3 && across(te0, 0.25) > 0.3);
        let odd = fields.iter().any(|(c, r)| {
            let (a, b) = (across(r, -0.25), across(r, 0.25));
            *c == "Ex" && a * b < 0.0 && a.abs() > 0.3 && b.abs() > 0.3
        });
        assert!(odd, "no mode changes sign across the core");
    }

    #[test]
    fn a_width_sweep_records_rising_indices() {
        let root = temp("sweep");
        // 450 to 550 nm: the index rises ever more slowly (a_cross_sections_nodes_hold_its_edges
        // checks the edges are on nodes, so the widths don't snap)
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 3\n"
        );
        let job = Job::parse(&text.replace("modes = 2", "modes = 1")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let points: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPoint {
                    effective_indices, ..
                } => Some(effective_indices[0][0]),
                _ => None,
            })
            .collect();
        assert_eq!(points.len(), 3);
        assert!(points.windows(2).all(|w| w[1] > w[0]), "{points:?}");
        // rising ever more slowly, as a widening strip does
        assert!(points[1] - points[0] > points[2] - points[1], "{points:?}");
        // each point's structure: the strip at that width
        let widths: Vec<(usize, f64, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepShapes {
                    point,
                    value,
                    shapes,
                } => {
                    let xs = shapes[0].outline.iter().map(|p| p[0]);
                    let span = xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min);
                    Some((*point, *value, span))
                }
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 3);
        for (k, (point, value, span)) in widths.iter().enumerate() {
            assert_eq!(*point, k);
            assert!((value - (0.45 + 0.05 * k as f64)).abs() < 1e-9);
            assert!((span - value).abs() < 1e-9, "{span} {value}");
        }
        // and its mode, with its pictures, at that point's index and n_eff
        let modes: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::SweepMode { .. }))
            .collect();
        assert_eq!(modes.len(), 3);
        for (k, e) in modes.iter().enumerate() {
            let Event::SweepMode {
                point,
                label,
                effective_index,
                intensity,
                component,
                field,
                ..
            } = e
            else {
                unreachable!()
            };
            assert_eq!((*point, label.as_str()), (k, "mode 1 of 1"));
            assert_eq!(effective_index[0], points[k]);
            assert_eq!(component, "Ex");
            // on pixels twice the step: the 2 µm window in 40 pixels of 50 nm
            assert_eq!((intensity.nx, field.nx), (40, 40));
            assert!(
                (intensity.range().1 - 1.0).abs() < 1e-6 && (field.range().1 - 1.0).abs() < 1e-6
            );
            // to three decimals
            assert!(
                field
                    .values
                    .iter()
                    .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
            );
        }
        // the point's events come after its SweepPoint, the first after the sweep's own
        let first = events
            .iter()
            .position(|e| matches!(e, Event::SweepPoint { .. }))
            .unwrap();
        assert!(matches!(
            &events[first - 1],
            Event::Sweep { parameter, from, to, points: 3 }
                if parameter == "width" && (*from, *to) == (0.45, 0.55)
        ));
        assert!(matches!(
            &events[first + 1],
            Event::SweepShapes { point: 0, .. }
        ));
        assert!(matches!(
            &events[first + 2],
            Event::SweepPermittivity { point: 0, .. }
        ));
        assert!(matches!(
            &events[first + 3],
            Event::SweepMode { point: 0, .. }
        ));
        // each point's cross-section: silicon across the strip's width, wider point by point
        let silicon: Vec<usize> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPermittivity { raster, .. } => {
                    // the row through the middle of the 220 nm silicon, 2 to 2.22 µm up
                    let j =
                        ((2.11 - raster.y0) / (raster.y1 - raster.y0) * raster.ny as f64) as usize;
                    Some((0..raster.nx).filter(|&i| raster.at(i, j) > 11.0).count())
                }
                _ => None,
            })
            .collect();
        assert_eq!(silicon.len(), 3);
        assert!(silicon.windows(2).all(|w| w[1] > w[0]), "{silicon:?}");
    }

    #[test]
    fn a_wavelength_sweep_records_each_points_modes() {
        let root = temp("wavelength-sweep");
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // no shapes: the structure stays the job's
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::SweepShapes { .. }))
        );
        let modes: Vec<(usize, f64, String, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepMode {
                    point,
                    wavelength_um,
                    label,
                    te_fraction,
                    ..
                } => Some((*point, *wavelength_um, label.clone(), *te_fraction)),
                _ => None,
            })
            .collect();
        assert_eq!(modes.len(), 4, "{modes:?}");
        for (k, (point, wavelength, label, te)) in modes.iter().enumerate() {
            assert_eq!(*point, k / 2);
            assert!((wavelength - [1.5, 1.6][k / 2]).abs() < 1e-12);
            assert_eq!(label, &format!("mode {} of 2", k % 2 + 1));
            // TE-like first, then TM-like, at both wavelengths
            assert!(if k % 2 == 0 { *te > 0.9 } else { *te < 0.1 });
        }
    }

    #[test]
    fn a_cross_sections_nodes_hold_its_edges_and_interfaces() {
        // a 450 nm strip on a 20 nm grid from −1 µm: ±0.225 µm aren't multiples of the step, so
        // a uniform grid would snap the width; they, and the layers' interfaces, must be nodes
        let rect = RectSpec {
            layer: "Si".into(),
            center_um: [0.0, 0.0],
            size_um: [0.45, 10.0],
        };
        let s = draw(LayerStack::soi_220(), std::slice::from_ref(&rect), &[], &[]).unwrap();
        let cs = cross_section(
            &s,
            &[rect],
            &[],
            &[],
            0.0,
            [-1.0, 1.0],
            [-1.0, 1.22],
            0.02,
            Wavelength::um(1.55).unwrap(),
        )
        .unwrap();
        let has = |nodes: &[f64], v: f64| nodes.iter().any(|&n| (n - v).abs() < 1e-12);
        assert!(has(cs.x(), -0.225) && has(cs.x(), 0.225), "{:?}", cs.x());
        assert!(has(cs.y(), 0.0) && has(cs.y(), 0.22), "{:?}", cs.y());
        // and the spacing stays about the step
        assert!(
            cs.x()
                .windows(2)
                .all(|w| w[1] - w[0] <= 0.02 + 1e-12 && w[1] - w[0] > 0.005)
        );
    }

    #[test]
    fn a_circles_outline_is_a_counterclockwise_64_gon() {
        let c = Shape::circle(Point::um(1.0, 2.0), Length::um(0.5)).unwrap();
        let o = outline(&c);
        assert_eq!(o.len(), 64);
        // counterclockwise: a positive signed area, close to the disk's
        let area: f64 = (0..64)
            .map(|k| {
                let (a, b) = (o[k], o[(k + 1) % 64]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0;
        assert!(
            (area - std::f64::consts::PI * 0.25).abs() < 0.01 * area,
            "{area}"
        );
    }

    #[test]
    fn a_preview_is_the_scene_its_run_records() {
        let root = temp("preview");
        let ring = "
[[task.ring]]
layer = \"Si\"
center_um = [0.0, 0.6]
radius_um = 0.5
width_um = 0.3
";
        for text in [
            format!("{JOB}{ring}"),
            format!("{MODES}{ring}"),
            format!("{MODES_X}{ring}"),
            format!("{FDFD}{ring}"),
        ] {
            let job = Job::parse(&text).unwrap();
            let preview = preview(&job).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let stop = Stop::new(None);
            stop.request(); // the scene comes first; nothing more is needed
            let _ = execute(&job, &mut run, &stop);
            let events: Vec<Event> = crate::run::replay(run.dir()).unwrap();
            let recorded = events
                .iter()
                .find(|e| matches!(e, Event::Scene { .. }))
                .unwrap();
            assert_eq!(&preview, recorded);
            let Event::Scene { shapes, .. } = preview else {
                unreachable!()
            };
            assert_eq!(
                shapes.last().unwrap().outline.len(),
                130,
                "the ring, with its hole"
            );
        }
    }

    #[test]
    fn a_rings_outline_walks_its_hole_through_a_cut() {
        let r = Shape::ring(Point::um(0.0, 0.0), Length::um(1.0), Length::um(0.5)).unwrap();
        let o = outline(&r);
        assert_eq!(o.len(), 64 + 1 + 64 + 1);
        // the first vertex recurs where the hole starts, and the hole closes on itself
        assert_eq!(o[64], o[0]);
        assert_eq!(o[129], o[65]);
        let area = |v: &[[f64; 2]]| {
            (0..v.len())
                .map(|k| {
                    let (a, b) = (v[k], v[(k + 1) % v.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum::<f64>()
                / 2.0
        };
        // outer counterclockwise, hole clockwise, and the whole walk the ring's area
        assert!(area(&o[..64]) > 0.0 && area(&o[65..129]) < 0.0);
        let exact = std::f64::consts::PI * (1.25 * 1.25 - 0.75 * 0.75);
        assert!((area(&o) - exact).abs() < 0.01 * exact, "{}", area(&o));
    }

    #[test]
    fn the_check_refuses_a_wavelength_a_material_has_no_data_at() {
        // 5 µm is beyond the stack's oxide: the check says what the run would
        let sweep = |from: f64, to: f64| {
            format!(
                "[task.sweep]
parameter = \"wavelength\"
from = {from}
to = {to}
points = 2
"
            )
        };
        let fdfd_sweep = sweep(1.5, 1.6);
        assert!(FDFD.contains(&fdfd_sweep));
        for (what, text) in [
            (
                "structure",
                JOB.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
            ),
            (
                "modes",
                MODES.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
            ),
            (
                "modes sweep",
                format!(
                    "{MODES}
{}",
                    sweep(1.5, 5.0)
                ),
            ),
            (
                "fdfd",
                FDFD.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
            ),
            ("fdfd sweep", FDFD.replace(&fdfd_sweep, &sweep(5.0, 1.6))),
        ] {
            let job = Job::parse(&text).unwrap();
            let refused = check(&job).unwrap_err();
            assert!(
                matches!(&refused, Error::OutsideValidity { wavelength_um, .. } if *wavelength_um == 5.0),
                "{what}: {refused}"
            );
            // the run's own words
            let root = temp("range");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused.to_string(), failed.to_string(), "{what}");
        }
        // a width sweep's ends are widths, not wavelengths
        let width = format!(
            "{MODES}
[task.sweep]
parameter = \"width\"
from = 0.4
to = 5.0
points = 2
"
        );
        check(&Job::parse(&width).unwrap()).unwrap();
    }

    /// The run's events without how it solved ([`Event::Solver`], [`Event::SolveError`]):
    /// what it found, for the tests that read events by position.
    fn found(run: &Run) -> Vec<Event> {
        replay(run.dir())
            .unwrap()
            .into_iter()
            .filter(|e| !matches!(e, Event::Solver { .. } | Event::SolveError { .. }))
            .collect()
    }

    #[test]
    fn a_modes_job_records_its_solver_and_each_modes_residual() {
        let root = temp("modes-solver");
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the solver, once, before the first mode: the 2 µm window in cells of 25 nm
        let at = events
            .iter()
            .position(|e| matches!(e, Event::Solver { .. }))
            .unwrap();
        let first_mode = events
            .iter()
            .position(|e| matches!(e, Event::Mode { .. }))
            .unwrap();
        assert!(at < first_mode);
        let Event::Solver {
            module,
            cells,
            step_um,
            unknowns,
            details,
        } = &events[at]
        else {
            unreachable!()
        };
        assert_eq!((module.as_str(), *step_um), ("mode::vector", 0.025));
        assert!(cells[0] >= 80 && cells[1] >= 80, "{cells:?}");
        // H_x and H_y at each node, less the walls'
        let nodes = (cells[0] + 1) * (cells[1] + 1);
        assert!(*unknowns <= 2 * nodes && *unknowns > nodes, "{unknowns}");
        assert!(details.iter().any(|[name, _]| name == "eigen-solver"));
        // each mode's residual, the job's own and each point's: within the solver's tolerance
        let errors: Vec<(Option<usize>, f64, String, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SolveError {
                    point,
                    value,
                    measure,
                    error,
                } => Some((*point, *value, measure.clone(), *error)),
                _ => None,
            })
            .collect();
        let places: Vec<(Option<usize>, f64)> = errors.iter().map(|e| (e.0, e.1)).collect();
        assert_eq!(
            places,
            [
                (None, 1.55),
                (None, 1.55),
                (Some(0), 1.5),
                (Some(0), 1.5),
                (Some(1), 1.6),
                (Some(1), 1.6)
            ]
        );
        for (k, (_, _, measure, error)) in errors.iter().enumerate() {
            assert_eq!(*measure, format!("eigen-residual, mode {} of 2", k % 2 + 1));
            assert!(*error > 0.0 && *error < 1e-9, "{measure}: {error:e}");
        }
    }

    #[test]
    fn an_fdfd_job_records_its_solver_and_each_solves_errors() {
        let root = temp("fdfd-solver");
        let job = Job::parse(FDFD).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the solver right after the scene: the 2.8 µm window in cells of 40 nm
        let Event::Solver {
            module,
            cells,
            step_um,
            unknowns,
            details,
        } = &events[2]
        else {
            panic!("{:?}", events[2])
        };
        assert_eq!(
            (module.as_str(), *cells, *step_um, *unknowns),
            ("fdfd", [70, 70], 0.04, 4900)
        );
        assert!(
            details
                .iter()
                .any(|[name, value]| name == "PML" && value.starts_with("20 cells"))
        );
        // at each of the two wavelengths, the field's residual and S's reciprocity: a direct
        // solve's rounding, and a straight strip's S symmetric to it
        let errors: Vec<(Option<usize>, f64, &str, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SolveError {
                    point,
                    value,
                    measure,
                    error,
                } => Some((*point, *value, measure.as_str(), *error)),
                _ => None,
            })
            .collect();
        let places: Vec<(Option<usize>, f64, &str)> =
            errors.iter().map(|e| (e.0, e.1, e.2)).collect();
        assert_eq!(
            places,
            [
                (Some(0), 1.5, "linear residual"),
                (Some(0), 1.5, "reciprocity"),
                (Some(1), 1.6, "linear residual"),
                (Some(1), 1.6, "reciprocity")
            ]
        );
        for (_, _, measure, error) in &errors {
            assert!(*error >= 0.0 && *error < 1e-9, "{measure}: {error:e}");
        }
        // a job at one wavelength: the errors are the job's own
        let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
        let job = Job::parse(&FDFD.replace(sweep, "")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let single: Vec<(Option<usize>, f64)> = replay(run.dir())
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Event::SolveError { point, value, .. } => Some((*point, *value)),
                _ => None,
            })
            .collect();
        assert_eq!(single, [(None, 1.55), (None, 1.55)]);
    }

    #[test]
    fn a_job_names_its_direct_solver_and_the_record_says_which_solved() {
        use crate::backend::tests::Recording;
        let direct_of = |events: &[Event]| -> String {
            events
                .iter()
                .find_map(|e| match e {
                    Event::Solver { details, .. } => details
                        .iter()
                        .find(|[name, _]| name == "direct solver")
                        .map(|[_, value]| value.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let version = env!("CARGO_PKG_VERSION");
        let with = |name: &str| format!("{FDFD}\n[solver]\ndirect = \"{name}\"\n");
        // none named: photonoxide's own, as its choice
        let plain = run_events("fdfd-direct-auto", FDFD);
        assert_eq!(
            direct_of(&plain),
            format!("photonoxide {version} (auto: no benchmark records on this machine)")
        );
        // photonoxide's by name: the same run
        let named = run_events("fdfd-direct-own", &with("photonoxide"));
        assert_eq!(direct_of(&named), format!("photonoxide {version}"));
        let s = |events: &[Event]| -> Vec<Event> {
            events
                .iter()
                .filter(|e| matches!(e, Event::SParameters { .. }))
                .cloned()
                .collect()
        };
        assert!(!s(&plain).is_empty());
        assert_eq!(s(&plain), s(&named));
        // a backend registered at run time: the job's solves go to it (two wavelengths: one
        // analysis, two factorizations), and give the same S-matrices
        let recording = Recording::register("recording-job");
        let routed = run_events("fdfd-direct-recording", &with("recording-job"));
        assert_eq!(direct_of(&routed), format!("recording-job {version}"));
        let [analyses, factorizations, solves, _] = recording.counts();
        assert_eq!((analyses, factorizations), (1, 2));
        assert!(solves >= 4, "{solves}");
        assert_eq!(s(&plain), s(&routed));
    }

    #[test]
    fn a_job_that_names_no_backend_records_what_auto_chose_and_why() {
        use crate::backend::Form;
        use crate::backend::auto::{Measured, Outcome, with};
        let direct_of = |events: &[Event]| -> String {
            events
                .iter()
                .find_map(|e| match e {
                    Event::Solver { details, .. } => details
                        .iter()
                        .find(|[name, _]| name == "direct solver")
                        .map(|[_, value]| value.clone()),
                    _ => None,
                })
                .unwrap()
        };
        // the job's 70 × 70 cells: faer's LU measured 1.5 times as fast near that size
        let record = |backend: &str, seconds: f64| Measured {
            family: "fdfd2d".into(),
            form: Form::Symmetric,
            unknowns: 5_000,
            backend: backend.into(),
            threads: rayon::current_num_threads(),
            seconds,
            peak_bytes: None,
            outcome: Outcome::Accurate,
            unix_seconds: 1_791_763_200,
        };
        let records = vec![record("photonoxide", 3.0), record("faer", 2.0)];
        let chosen = with(records.clone(), None, || {
            run_events("fdfd-auto-chosen", FDFD)
        });
        let said = direct_of(&chosen);
        assert!(said.starts_with("faer "), "{said}");
        assert!(
            said.contains("(auto: 1.5 times faster than photonoxide's own at 5000 unknowns on")
                && said.ends_with("on this machine, measured 2026-10-12)"),
            "{said}"
        );
        // a job that pins its backend keeps it, whatever the records
        let pinned = format!("{FDFD}\n[solver]\ndirect = \"photonoxide\"\n");
        let pinned = with(records, None, || run_events("fdfd-auto-pinned", &pinned));
        assert_eq!(
            direct_of(&pinned),
            format!("photonoxide {}", env!("CARGO_PKG_VERSION"))
        );
        // the same device either way: the S-parameters agree to rounding
        let s = |events: &[Event]| -> Vec<f64> {
            events
                .iter()
                .filter(|e| matches!(e, Event::SParameters { .. }))
                .filter_map(|e| serde_json::to_value(e).ok())
                .flat_map(|v| numbers(&v))
                .collect()
        };
        let (a, b) = (s(&chosen), s(&pinned));
        assert!(
            !a.is_empty() && a.len() == b.len(),
            "{} {}",
            a.len(),
            b.len()
        );
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-9 * (1.0 + y.abs()), "{x} vs {y}");
        }
    }

    /// Every number in a JSON value, in order.
    fn numbers(v: &serde_json::Value) -> Vec<f64> {
        match v {
            serde_json::Value::Number(n) => n.as_f64().into_iter().collect(),
            serde_json::Value::Array(a) => a.iter().flat_map(numbers).collect(),
            serde_json::Value::Object(o) => o.values().flat_map(numbers).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn the_check_and_the_run_refuse_a_direct_solver_that_cant_be_had() {
        let with = |text: &str, name: &str| format!("{text}\n[solver]\ndirect = \"{name}\"\n");
        crate::backend::register_unavailable("job-test-missing", "not installed").unwrap();
        let cases = [
            // nothing registered under the name
            (
                with(FDFD, "job-test-nothing"),
                "no direct solver is registered as \"job-test-nothing\"",
            ),
            // known, and not to be had: with the reason
            (
                with(FDFD, "job-test-missing"),
                "the direct solver \"job-test-missing\" isn't available: not installed",
            ),
            // a job without a direct solve
            (
                with(MODES, "photonoxide"),
                "[solver] direct = \"photonoxide\" is for fdfd jobs: a modes job has no direct \
                 solve to give a backend",
            ),
        ];
        for (k, (text, want)) in cases.iter().enumerate() {
            let job = Job::parse(text).unwrap();
            let checked = check(&job).unwrap_err().to_string();
            assert!(checked.contains(want), "{checked}");
            // the run says the same, before recording anything
            let root = temp(&format!("direct-refused-{k}"));
            let mut run = Run::create(&root.0, &job).unwrap();
            let ran = execute(&job, &mut run, &Stop::new(None))
                .unwrap_err()
                .to_string();
            assert_eq!(ran, checked);
            assert!(replay::<Event>(run.dir()).unwrap().is_empty());
        }
        // auto on any job is no choice at all
        check(&Job::parse(&with(MODES, "auto")).unwrap()).unwrap();
    }

    #[test]
    fn the_check_refuses_what_the_run_would_before_it_solves() {
        // each of these passed the check and failed in the run: the check now says what the
        // run says, and the run still says it
        let port = "[[task.port]]\nx_um = -0.4\nside = \"left\"";
        let with_port = |to: &str| FDFD.replace(port, to);
        let cases = [
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 40.0\npml_cells = 60"),
                "PMLs of 60 and 60 cells leave nothing of 70 cells",
            ),
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 3000.0"),
                "x_um is 2.8 um across, less than 2 cells at a step of 3000 nm",
            ),
            (
                with_port("[[task.port]]\nx_um = -9.0\nside = \"left\""),
                "the port at x = -9 um is outside the window",
            ),
            (
                with_port("[[task.port]]\nx_um = -1.4\nside = \"left\""),
                "column 0 must be two cells clear of the PMLs (20 and 20 cells)",
            ),
            (
                with_port("[[task.port]]\nx_um = -0.4\nside = \"up\""),
                "unknown port side \"up\": left or right",
            ),
            (
                with_port("[[task.port]]\nx_um = -0.4\nside = \"left\"\ny_um = [0.0, 0.04]"),
                "its y_um [0, 0.04] covers 1 row of the window (y from -1.4 to 1.4 um, 70 rows); a \
                 port needs 3 rows or more",
            ),
            (
                with_port("[[task.port]]\nx_um = -0.4\nside = \"left\"\ny_um = [5.0, 6.0]"),
                "its y_um, 5 to 6, must lie inside the window's, -1.4 to 1.4",
            ),
            // a step a thousand times too small, in each kind: refused, not allocated
            (
                JOB.replace("step_nm = 50.0", "step_nm = 0.05"),
                "x_um at a step of 0.05 nm is 40000 cells, more than 10000",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 50.0\nz_um = [0.0, 600.0]"),
                "z_um at a step of 50 nm is 12000 cells",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 0.025"),
                "x_um at a step of 0.025 nm is 80000 cells",
            ),
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 0.04"),
                "x_um at a step of 0.04 nm is 70000 cells",
            ),
            // a window as wide as a thousand of the step: a number, not three hundred digits
            (
                JOB.replace("x_um = [-1.0, 1.0]", "x_um = [-1e300, 1e300]"),
                "x_um at a step of 50 nm is 4.0e301 cells, more than 10000",
            ),
            // a step a thousand times too large: refused, not solved on a window of one cell
            (
                MODES.replace("step_nm = 25.0", "step_nm = 5000.0"),
                "x_um is 2 um across, less than 2 cells at a step of 5000 nm",
            ),
            (
                MODES.replace("modes = 2", "modes = 2\nz_um = [1.0, 1.01]"),
                "z_um is 0.01 um across, less than 2 cells at a step of 25 nm",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 3000.0"),
                "x_um is 2 um across, less than 2 cells at a step of 3000 nm",
            ),
            // the height the job doesn't give, of a layer a metre thick
            (
                MODES.replace(
                    "stack = \"soi_220\"",
                    "stack = \"soi\"\ncore_nm = 1e9\nbottom_oxide_um = 2.0",
                ),
                "the height, 1 um below and above the layer as z_um isn't given, at a step of 25 \
                 nm is",
            ),
            (
                MODES.replace("modes = 2", "modes = 0"),
                "modes must be at least 1, got 0",
            ),
            // a side view cut outside the window, given or by default
            (
                JOB.replace("step_nm = 50.0", "step_nm = 50.0\nside_y_um = 5.0"),
                "the side view's cut, side_y_um = 5, is outside y_um [-1, 1]",
            ),
            (
                JOB.replace("y_um = [-1.0, 1.0]", "y_um = [2.0, 4.0]"),
                "the side view's cut, side_y_um = 0 by default, is outside y_um [2, 4]",
            ),
        ];
        for (text, says) in cases {
            let job = Job::parse(&text).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{refused}");
            let root = temp("refused");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        // the largest grid allowed still passes: 10 000 cells across, and as many up (the
        // default height, 1 um below and above the layer, would be 11 100: its side view was
        // refused only when the run drew it)
        let job =
            Job::parse(&JOB.replace("step_nm = 50.0", "step_nm = 0.2\nz_um = [1.0, 3.0]")).unwrap();
        check(&job).unwrap();
        let job = Job::parse(&JOB.replace("step_nm = 50.0", "step_nm = 0.2")).unwrap();
        let refused = check(&job).unwrap_err().to_string();
        assert!(refused.contains("is 11100 cells"), "{refused}");
    }

    #[test]
    fn the_check_refuses_jobs_that_ran_on_something_other_than_what_they_say() {
        // each of these ran, on a clipped port, a port counted twice, a closed box, or bare
        // cladding: the check and the run now refuse them, in the same words
        let left = "[[task.port]]\nx_um = -0.4\nside = \"left\"";
        let right = "[[task.port]]\nx_um = 0.4\nside = \"right\"";
        assert!(FDFD.contains(left) && FDFD.contains(right));
        let cases = [
            (
                FDFD.replace(left, &format!("{left}\ny_um = [0.0, 5.0]")),
                "the port at x = -0.4 um: its y_um, 0 to 5, must lie inside the window's, -1.4 to 1.4",
            ),
            (
                FDFD.replace(right, left),
                "ports 1 and 2 are the same (x = -0.4 um, left)",
            ),
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 40.0\npml_cells = 0"),
                "pml_cells must be at least 1",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = 50.0"),
                "the cross-section at y = 50 um meets no shape between x = -1 and 1 um",
            ),
            (
                MODES.split("[[task.rect]]").next().unwrap().to_owned(),
                "the cross-section at y = 0 um meets no shape",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\nz_um = [5.0, 6.0]"),
                "z_um, 5 to 6, doesn't reach the layer Si (2 to 2.22 um)",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = 50.0"),
                "the cross-section at x = 50 um meets no shape between y = -1 and 1 um",
            ),
        ];
        for (text, says) in cases {
            let job = Job::parse(&text).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{refused}");
            let root = temp("guideless");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        // what is still a job: a port's window that is the window's own, a z window that
        // holds the layer and nothing else, a strip narrower than a cell of the sampling
        for text in [
            FDFD.replace(left, &format!("{left}\ny_um = [-1.4, 1.4]")),
            MODES.replace("step_nm = 25.0", "step_nm = 25.0\nz_um = [1.9, 2.3]"),
            MODES.replace("size_um = [0.5, 10.0]", "size_um = [0.01, 10.0]"),
        ] {
            check(&Job::parse(&text).unwrap()).unwrap();
        }
    }

    #[test]
    fn a_modes_job_asks_for_at_most_50_modes() {
        // hundreds ran minutes past a job's time limit: refused by the check and the run, in
        // the same words (none is refused too, by the_check_refuses_what_the_run_would…)
        for (modes, says) in [
            ("modes = 51", "modes must be at most 50, got 51"),
            ("modes = 500", "modes must be at most 50, got 500"),
        ] {
            let job = Job::parse(&MODES.replace("modes = 2", modes)).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{refused}");
            let root = temp("many-modes");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        for modes in ["modes = 1", "modes = 50"] {
            check(&Job::parse(&MODES.replace("modes = 2", modes)).unwrap()).unwrap();
        }
    }

    #[test]
    fn bad_tasks_are_errors() {
        let root = temp("bad");
        for (text, says) in [
            (
                JOB.replace("kind = \"structure\"", "kind = \"fem\""),
                "unknown kind",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"glass\""),
                "unknown stack",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"soi\""),
                "core_nm",
            ),
            (JOB.replace("step_nm", "step"), "step"),
            (
                JOB.replace("layer = \"Si\"\ncenter", "layer = \"M1\"\ncenter"),
                "M1",
            ),
            // windows that run backwards or are empty, a step that isn't positive
            (
                JOB.replace("x_um = [-1.0, 1.0]", "x_um = [1.0, -1.0]"),
                "x_um must go from a smaller to a larger number",
            ),
            (
                JOB.replace("y_um = [-1.0, 1.0]", "y_um = [1.0, 1.0]"),
                "y_um must go",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 50.0\nz_um = [3.0, 1.0]"),
                "z_um must go",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 0.0"),
                "step_nm must be positive",
            ),
            (
                MODES.replace("x_um = [-1.0, 1.0]", "x_um = [1.0, -1.0]"),
                "x_um must go",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = -25.0"),
                "step_nm must be positive",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = nan"),
                "cut_y_um must be a number",
            ),
            // the propagation, and the other propagation's window or cut
            (
                MODES_X.replace("propagation = \"x\"", "propagation = \"z\""),
                "unknown propagation \"z\"",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\nx_um = [-1.0, 1.0]"),
                "x_um is for a modes job along y",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = 0.5"),
                "cut_y_um is for a modes job along y",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = 0.5"),
                "cut_x_um is for a modes job along x",
            ),
            (
                MODES_X.replace("y_um = [-1.0, 1.0]\n", ""),
                "a modes job along x needs y_um",
            ),
            (
                MODES_X.replace("y_um = [-1.0, 1.0]", "y_um = [1.0, -1.0]"),
                "y_um must go",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = inf"),
                "cut_x_um must be a number",
            ),
            // sweeps the run can't take
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.55\nto = 1.55\npoints = 3\n"
                ),
                "must differ",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 1\n"
                ),
                "at least 2 points",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"height\"\nfrom = 0.2\nto = 0.3\npoints = 3\n"
                ),
                "unknown sweep parameter",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.4\nto = 0.6\npoints = 3\nrect = 2\n"
                ),
                "the width sweep's rect 2 isn't there: rect counts from 0, so it is rectangle 3 of \
                 the job's 1",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = -0.1\nto = 0.6\npoints = 3\n"
                ),
                "widths must be positive",
            ),
            (
                FDFD.replace("y_um = [-1.4, 1.4]", "y_um = [1.4, -1.4]"),
                "y_um must go",
            ),
            (FDFD.replace("to = 1.6", "to = 1.5"), "must differ"),
            (
                FDFD.replace("x_um = -0.4", "x_um = nan"),
                "a port's x_um must be a number",
            ),
        ] {
            let job = Job::parse(&text).unwrap();
            // found by the check, without running, as by the run
            let e = check(&job).unwrap_err();
            assert!(e.to_string().contains(says), "check, {says}: {e}");
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(says), "{says}: {e}");
        }
    }

    #[test]
    fn the_repositorys_jobs_pass_the_check() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|x| x == "toml") {
                check(&Job::load(&path).unwrap()).unwrap_or_else(|e| panic!("{path:?}: {e}"));
                count += 1;
            }
        }
        assert!(count >= 4);
        assert!(check(&Job::parse(FDFD).unwrap()).is_ok());
        let portless = FDFD.split("[[task.port]]").next().unwrap();
        let e = check(&Job::parse(portless).unwrap()).unwrap_err();
        assert!(e.to_string().contains("port"), "{e}");
    }

    const FDFD: &str = r#"
name = "strip-fdfd"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.4, 1.4]
y_um = [-1.4, 1.4]
step_nm = 40.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]

[[task.port]]
x_um = -0.4
side = "left"

[[task.port]]
x_um = 0.4
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 2
"#;

    #[test]
    fn an_fdfd_job_records_s_parameters_and_a_field() {
        let root = temp("fdfd");
        let job = Job::parse(FDFD).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // started, the scene, the sweep, S at 1.5 um, the field and the point's, S at 1.6 um,
        // its point's field, finished
        assert_eq!(events.len(), 9, "{events:?}");
        // the scene and the pictures leave out the PMLs (20 cells of 40 nm) and the two cells
        // beside them: the 2.8 µm window's middle 1.04 µm, 26 cells
        let Event::Scene { x_um, y_um, .. } = &events[1] else {
            panic!("{:?}", events[1])
        };
        for (shown, edge) in [
            (x_um[0], -0.52),
            (x_um[1], 0.52),
            (y_um[0], -0.52),
            (y_um[1], 0.52),
        ] {
            assert!((shown - edge).abs() < 1e-12, "{x_um:?} {y_um:?}");
        }
        assert!(matches!(
            &events[2],
            Event::Sweep { parameter, from, to, points: 2 }
                if parameter == "wavelength" && (*from, *to) == (1.5, 1.6)
        ));
        let Event::SParameters {
            wavelength_um,
            ports,
            effective_indices,
            s,
        } = &events[3]
        else {
            panic!("{:?}", events[3])
        };
        assert_eq!(*wavelength_um, 1.5);
        assert_eq!(ports.len(), 2);
        // a straight strip: everything through, in a mode between the oxide's and the slab's
        // index (the slab's TE mode, 2.85, in oxide, 1.444)
        let through = s[1][0][0].hypot(s[1][0][1]);
        assert!((through - 1.0).abs() < 1e-6, "{s:?}");
        assert!(
            effective_indices.iter().all(|&n| n > 1.444 && n < 2.85),
            "{effective_indices:?}"
        );
        let Event::Field { intensity, .. } = &events[4] else {
            panic!("{:?}", events[4])
        };
        assert_eq!((intensity.nx, intensity.ny), (26, 26));
        assert!((intensity.x0 - x_um[0]).abs() < 1e-12 && (intensity.x1 - x_um[1]).abs() < 1e-12);
        assert!(
            matches!(&events[6], Event::SParameters { wavelength_um, .. } if *wavelength_um == 1.6)
        );
        // each point's field after its S: the job's own on 2 x 2 blocks, its peak 1, to three
        // decimals, over the same window
        for (k, at) in [(0, 5), (1, 7)] {
            let Event::SweepField {
                point,
                value,
                wavelength_um,
                intensity: coarse,
                ..
            } = &events[at]
            else {
                panic!("{:?}", events[at])
            };
            assert_eq!((*point, *value, *wavelength_um), (k, [1.5, 1.6][k], *value));
            assert_eq!((coarse.nx, coarse.ny), (13, 13));
            assert_eq!((coarse.x0, coarse.y0), (intensity.x0, intensity.y0));
            assert!(
                (coarse.x1 - intensity.x1).abs() < 1e-12
                    && (coarse.y1 - intensity.y1).abs() < 1e-12
            );
            assert_eq!(coarse.range().1, 1.0);
            assert!(
                coarse
                    .values
                    .iter()
                    .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
            );
            // the first point is the wavelength the job's own field is at: its blocks' means
            if k == 0 {
                let mean = |i: usize, j: usize| {
                    (0..2)
                        .flat_map(|b| (0..2).map(move |a| (a, b)))
                        .map(|(a, b)| f64::from(intensity.at(2 * i + a, 2 * j + b)))
                        .sum::<f64>()
                        / 4.0
                };
                let peak = (0..13)
                    .flat_map(|j| (0..13).map(move |i| (i, j)))
                    .map(|(i, j)| mean(i, j))
                    .fold(0.0, f64::max);
                for (i, j) in [(6, 6), (0, 6), (12, 7), (6, 1)] {
                    let expected = mean(i, j) / peak;
                    assert!(
                        (f64::from(coarse.at(i, j)) - expected).abs() < 1e-3,
                        "{i} {j}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_fdfd_jobs_field_starts_where_its_guide_comes_in() {
        // the strip crosses the whole window, its port 1 at x = −0.4 (column 25); the PML is the
        // first 20 columns. The field is launched at column 22, the picture's first, so from
        // the picture's left edge to the port the guide is as bright as beyond the port (a
        // straight, lossless guide), and as bright at the picture's right edge: the light
        // starts where the picture does and ends where it does
        let root = temp("fdfd-launch");
        let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
        let single = FDFD.replace(sweep, "");
        let field_of = |text: &str| {
            let job = Job::parse(text).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            execute(&job, &mut run, &Stop::new(None)).unwrap();
            replay(run.dir())
                .unwrap()
                .into_iter()
                .find_map(|e| match e {
                    Event::Field { intensity, .. } => Some(intensity),
                    _ => None,
                })
                .unwrap()
        };
        let field = field_of(&single);
        // along the guide's axis, y = 0
        assert_eq!((field.nx, field.ny), (26, 26));
        // along the guide's axis, y = 0: row 35 of the grid, 13 of the picture
        let on_axis = |i: usize| f64::from(field.at(i, 13));
        let beyond = on_axis(18);
        assert!(beyond > 0.5, "{beyond}");
        for i in [0, 1, 2, 25] {
            assert!(
                (on_axis(i) - beyond).abs() < 0.05 * beyond,
                "column {i}: {} against {beyond}",
                on_axis(i)
            );
        }
        // a strip that starts just before the port: at column 22 there is no guide, so the
        // field is launched at the port, and before it only what the device reflects shows
        let short = single.replace(
            "center_um = [0.0, 0.0]\nsize_um = [4.0, 0.5]",
            "center_um = [1.5, 0.0]\nsize_um = [3.9, 0.5]",
        );
        assert_ne!(short, single);
        let field = field_of(&short);
        assert_eq!(field.range().1, 1.0);
        assert!(f64::from(field.at(18, 13)) > 0.5);
        assert!(f64::from(field.at(1, 13)) < 0.05, "{}", field.at(1, 13));
    }

    #[test]
    fn an_fdfd_job_at_one_wavelength_records_no_sweep() {
        let root = temp("fdfd-single");
        let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
        let job = Job::parse(&FDFD.replace(sweep, "")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // started, the scene, S, the field, finished
        assert_eq!(events.len(), 5, "{events:?}");
        assert!(matches!(&events[3], Event::Field { .. }));
    }

    #[test]
    fn a_long_sweeps_fields_are_on_larger_blocks() {
        // the ring job's grid: 2 x 2 blocks for its 201 points, larger for ten times as many
        assert_eq!(fdfd::sweep_block(360, 256, 201), 2);
        let b = fdfd::sweep_block(360, 256, 2010);
        assert!(b > 2 && (360 / b) * (256 / b) * 2010 <= 5_000_000, "{b}");
        assert!((360 / (b - 1)) * (256 / (b - 1)) * 2010 > 5_000_000, "{b}");
        // never more than the grid
        assert_eq!(fdfd::sweep_block(3, 1, 10_000_000), 1);
    }

    #[test]
    fn an_fdfd_job_records_the_field_at_the_wavelength_asked() {
        let root = temp("fdfd-field");
        let text = FDFD.replace(
            "step_nm = 40.0",
            "step_nm = 40.0
field_um = 1.58",
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // the swept wavelength nearest 1.58 is 1.6, the second: its field follows its S
        let fields: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::Field { wavelength_um, .. } => Some(*wavelength_um),
                _ => None,
            })
            .collect();
        assert_eq!(fields, [1.6]);
        assert!(matches!(&events[6], Event::Field { .. }), "{:?}", events[6]);
    }

    #[test]
    fn an_fdfd_job_without_ports_or_with_a_port_in_the_pml_is_an_error() {
        let ports = "[[task.port]]\nx_um = -0.4\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
        let in_pml = "[[task.port]]\nx_um = -1.35\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
        for (to, error) in [("", "at least one"), (in_pml, "clear of the PMLs")] {
            let root = temp("fdfd-bad");
            let job = Job::parse(&FDFD.replace(ports, to)).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(error), "{e}");
        }
    }

    /// The job's events on a pool of `threads`, without the time it took.
    fn events_on(threads: usize, text: &str, tag: &str) -> Vec<Event> {
        let root = temp(&format!("{tag}-{threads}"));
        let job = Job::parse(text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| execute(&job, &mut run, &Stop::new(None)))
            .unwrap();
        replay(run.dir())
            .unwrap()
            .into_iter()
            .filter(|e| !matches!(e, Event::Finished { .. }))
            .collect()
    }

    #[test]
    fn a_sweep_records_the_same_events_on_any_number_of_threads() {
        // one thread solves the points one after another, as a loop over them does; four solve
        // them four at a time, and record them in the same order, to the last bit
        let modes = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 5\n"
        );
        let fdfd = FDFD.replace("points = 2", "points = 5");
        assert_ne!(fdfd, FDFD);
        for (text, tag) in [
            (modes.as_str(), "sweep-modes"),
            (fdfd.as_str(), "sweep-fdfd"),
        ] {
            let one = events_on(1, text, tag);
            let points = one
                .iter()
                .filter(|e| matches!(e, Event::SweepPoint { .. } | Event::SParameters { .. }))
                .count();
            assert_eq!(points, 5, "{tag}");
            for threads in [4, 20] {
                assert_eq!(events_on(threads, text, tag), one, "{tag} on {threads}");
            }
        }
    }

    #[test]
    fn fdfd_s_parameters_are_the_same_on_any_number_of_threads() {
        let job = Job::parse(&FDFD.replace("points = 2", "points = 5")).unwrap();
        let on = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| fdfd_s_parameters(&job, None))
                .unwrap()
        };
        let one = on(1);
        assert_eq!(one.s.len(), 5);
        assert_eq!(on(4), one);
        assert_eq!(on(20), one);
    }
}

//! The events of a run's record.

use serde::{Deserialize, Serialize};

use super::scene::{SceneLayer, SceneMedium, ScenePort, SceneShape};
use super::{
    ChipPart, Corner, DeviceFigure, FoundResonance, SpectrumSeries, StaticValue, SweepAxis,
    optimize,
};
use crate::raster::Raster;

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
    /// What kind of mode a modes job with a PML (`[task.pml]`) found, and its loss: recorded
    /// right after the mode's [`Event::ModeField`], or at a sweep's point right after its
    /// [`Event::SweepMode`]. A job without a PML records none: its window is closed, and every
    /// mode it keeps is guided.
    ModeLoss {
        /// The sweep point's index, from 0, or `None` for the job's own configuration.
        point: Option<usize>,
        /// The mode's label, as its [`Event::Mode`] or [`Event::SweepMode`] has it, e.g.
        /// `"mode 1 of 2"`.
        label: String,
        /// `"guided"`, its Re n_eff above the highest index of the media around the guides
        /// ([`crate::mode::vector::VectorMode::is_guided`]), or `"leaky"`, below it: its light
        /// leaks into those media, through the PML's edges.
        kind: String,
        /// The loss of its power along the guide, dB/cm, from its Im n_eff
        /// ([`crate::mode::dispersion::loss_db_per_cm`]) on the run's grid and PML, which the
        /// run's [`Event::Solver`] states. A guided mode's is the PML's small effect on it.
        loss_db_per_cm: f64,
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
    /// The named ports of a job's components (`[[task.component]]`), recorded after its
    /// [`Event::Scene`] when it has any: markers for the scene ([`preview_ports`](crate::job::preview_ports)).
    Ports {
        /// The ports, component by component.
        ports: Vec<ScenePort>,
    },
    /// A device's model, recorded by a job that computes its device (`[device]` with a
    /// `"spectrum"` task) once the model is built: what computes it, what it reproduces and on
    /// which grid, how long that took, and the device's figures from it. The S-parameters
    /// follow, one [`Event::SParameters`] per wavelength, each port's effective index its
    /// mode's where the model states one (0 where it doesn't).
    DeviceModel {
        /// The device's kind, e.g. `"MMI 1x2"`.
        device: String,
        /// The fidelity: `"analytic"`, `"2D"` or `"3D"`.
        fidelity: String,
        /// What computes it, in a sentence.
        method: String,
        /// Its write-up in docs/methods, e.g. `"devices"`.
        write_up: String,
        /// The papers it implements, each with its DOI.
        papers: Vec<String>,
        /// What the component reproduces, with its grid (its provenance's source).
        source: String,
        /// The grid its figures name, or `"exact: no grid"`.
        grid: String,
        /// The time building the model took, seconds: the solve's at a solver-backed fidelity.
        seconds: f64,
        /// The device's figures from the model, each with its fidelity and grid.
        figures: Vec<DeviceFigure>,
    },
    /// A fit of a compact model of a job's device about to run (`[device]` with a `"fit"` task,
    /// [`crate::device::FitPlan`]), recorded after the nominal design's scene and ports: the
    /// parameters it covers, how its samples are placed, and the model's options. An
    /// [`Event::DevicePoint`] per sample follows, in order (the `samples` it is fitted to, then the
    /// `held_out`), each `at` the covered parameters' values, then [`Event::CompactFit`].
    CompactPlan {
        /// The device's kind, e.g. `"all-pass ring"`.
        device: String,
        /// The fidelity every sample is computed at, the model's source: `"analytic"`, `"2D"`
        /// or `"3D"`.
        fidelity: String,
        /// What computes the samples, in a sentence.
        method: String,
        /// Its write-up in docs/methods, e.g. `"devices"`.
        write_up: String,
        /// The papers it implements, each with its DOI.
        papers: Vec<String>,
        /// The device's parameters' names: the order of each point's `values`.
        parameters: Vec<String>,
        /// Their units.
        units: Vec<String>,
        /// The parameters the model covers, in its order.
        axes: Vec<String>,
        /// Their units.
        axis_units: Vec<String>,
        /// Each one's range, [smallest, largest].
        ranges: Vec<[f64; 2]>,
        /// How the samples are placed, in words.
        sampling: String,
        /// How many samples the model is fitted to.
        samples: usize,
        /// How many follow them, held out of the fit.
        held_out: usize,
        /// The model's basis poles.
        poles: usize,
        /// How its coefficients depend on the parameters, in words.
        interpolation: String,
        /// The wavelengths each sample's S-parameters are at, µm.
        wavelengths_um: Vec<f64>,
        /// The ports' names, in the S-matrices' order.
        ports: Vec<String>,
    },
    /// The compact model a fit made of its samples ([`Event::CompactPlan`]): the file's content
    /// ([`crate::device::Fitted`]), its errors at its samples and held-out ones, its stability
    /// and passivity, and the model, every number to the bit.
    CompactFit {
        /// The model and what it was fitted from.
        fitted: Box<crate::device::Fitted>,
    },
    /// A study of a job's device about to run (`[device]` with a `"sweep"` or `"corners"` task,
    /// [`Study`](crate::job::Study)), recorded after the nominal design's scene and ports: what it steps through, so
    /// that a viewer can say how far along it is. An [`Event::DevicePoint`] per point follows, in
    /// order.
    DeviceStudy {
        /// `"sweep"` or `"corners"`.
        study: String,
        /// The device's kind, e.g. `"directional coupler"`.
        device: String,
        /// The fidelity every point is computed at: `"analytic"`, `"2D"` or `"3D"`.
        fidelity: String,
        /// What computes it, in a sentence.
        method: String,
        /// Its write-up in docs/methods, e.g. `"devices"`.
        write_up: String,
        /// The papers it implements, each with its DOI.
        papers: Vec<String>,
        /// The device's parameters' names: the order of each point's `values`.
        parameters: Vec<String>,
        /// Their units, e.g. `"µm"`, or `""`.
        units: Vec<String>,
        /// A sweep's axes, the second's values fastest; none for corners.
        axes: Vec<SweepAxis>,
        /// The axes' parameters' units, in the same order.
        axis_units: Vec<String>,
        /// The corners, after the nominal design; none for a sweep.
        corners: Vec<Corner>,
        /// How many points.
        points: usize,
        /// The wavelengths each point's S-parameters are at, µm.
        wavelengths_um: Vec<f64>,
        /// The ports' names, in the S-matrices' order.
        ports: Vec<String>,
    },
    /// A point of a device's study: its design computed at the study's fidelity, or why it
    /// wasn't. The same numbers, to the bit, as the run of that design's own `"spectrum"` job.
    DevicePoint {
        /// The point's index, from 0.
        point: usize,
        /// A sweep's: each axis's value there, in its unit; none for corners.
        at: Vec<f64>,
        /// A corners study's: the corner, `"nominal"` for the first point.
        corner: Option<String>,
        /// The device's parameters' values there, in the [`Event::DeviceStudy`]'s order.
        values: Vec<f64>,
        /// Why the model gave nothing there (a fit outside its range, a solve that failed); the
        /// fields after it are then empty.
        error: Option<String>,
        /// What the component reproduces, with its grid (its provenance's source).
        source: String,
        /// The grid its figures name, or `"exact: no grid"`.
        grid: String,
        /// The device's figures there, each with its fidelity and grid.
        figures: Vec<DeviceFigure>,
        /// Each port's mode's effective index where the model states one (0 where it doesn't).
        effective_indices: Vec<f64>,
        /// The power-normalized S-matrices, one per wavelength of the study: `s[w][q][p]` from
        /// port p into port q, real and imaginary parts.
        s: Vec<Vec<Vec<[f64; 2]>>>,
    },
    /// An optimization of a job's device about to run (`[device]` with an `"optimize"` task,
    /// [`optimize::Problem`]), recorded after the design's scene and ports: the problem, and
    /// each exact derivative checked against differences. An [`Event::Evaluation`] per
    /// evaluation follows, in the optimizer's order.
    Optimization {
        /// The device's kind, e.g. `"directional coupler"`.
        device: String,
        /// The fidelity every design is computed at: `"analytic"`, `"2D"` or `"3D"`.
        fidelity: String,
        /// What computes it, in a sentence.
        model: String,
        /// The method, as the job names it, e.g. `"lbfgsb"`.
        method: String,
        /// genoxide's method, e.g. `"L-BFGS-B"`.
        optimizer: String,
        /// The paper it implements, with its DOI.
        paper: String,
        /// The seed.
        seed: u64,
        /// The budget: the optimizer's evaluations.
        evaluations: usize,
        /// A population method's size.
        population: Option<usize>,
        /// The hard limit, minutes.
        timeout_minutes: f64,
        /// The variables, each with its bounds, start, and how its derivatives are taken.
        variables: Vec<optimize::OptimizeVariable>,
        /// The objectives, one or two.
        objectives: Vec<optimize::OptimizeQuantity>,
        /// The constraints.
        constraints: Vec<optimize::OptimizeQuantity>,
        /// The wavelengths each quantity is taken at, µm.
        wavelengths_um: Vec<f64>,
        /// The corners each is also taken at, beside the nominal design.
        corners: Vec<Corner>,
        /// The device's parameters' names.
        parameters: Vec<String>,
        /// Each exact derivative against fourth-order differences at the start.
        checks: Vec<optimize::GradientCheck>,
    },
    /// An evaluation of an optimization: the variables, each objective's cost and value, each
    /// constraint's g, and the best so far.
    Evaluation {
        /// The evaluation, from 1.
        index: usize,
        /// The variables, in their units.
        x: Vec<f64>,
        /// Each objective's cost (minimized); none where the model gave nothing.
        costs: Vec<f64>,
        /// Each objective's value.
        values: Vec<f64>,
        /// Each constraint's g (≤ 0 when met).
        constraints: Vec<f64>,
        /// Σ max(0, g); none where the model gave nothing.
        violation: Option<f64>,
        /// The first objective's gradient's norm in the variables scaled to their ranges, for a
        /// method that uses it.
        gradient_norm: Option<f64>,
        /// The best evaluation so far (one objective): feasible first, then the lowest cost.
        best: Option<usize>,
        /// The device's models built so far.
        solves: usize,
        /// Why the model gave nothing there.
        error: Option<String>,
    },
    /// The Pareto front of an optimization of two objectives after a generation: its
    /// non-dominated feasible evaluations, and the hypervolume they dominate up to a reference
    /// point fixed at the first front.
    Front {
        /// The evaluations so far.
        evaluations: usize,
        /// The front's points, in the evaluations' order.
        points: Vec<optimize::FrontPoint>,
        /// The hypervolume of their costs up to `reference`.
        hypervolume: f64,
        /// The reference point, each cost's.
        reference: Vec<f64>,
    },
    /// An optimization's end: its best evaluation and design (one objective), or its front (two),
    /// and why it stopped.
    Optimum {
        /// The best evaluation, from 1 (one objective).
        index: Option<usize>,
        /// Its variables.
        x: Vec<f64>,
        /// Its design's parameters' expressions: the job's, the variables written as numbers;
        /// what Apply takes into the design.
        parameters: Vec<(String, String)>,
        /// Whether it meets every constraint.
        feasible: bool,
        /// Why it stopped: the optimizer's reason (`"converged"`, `"evaluations"`, ...), or the
        /// run's (`"timeout"`, `"requested"`).
        stop: String,
        /// The evaluations.
        evaluations: usize,
        /// The device's models built.
        solves: usize,
        /// The front's evaluations (two objectives).
        front: Vec<usize>,
    },
    /// A chip about to be computed (a job's `[circuit]`, [`ChipDesign`](crate::job::ChipDesign)), recorded after
    /// [`Event::Started`]: each instance, what computes it with its fidelity and grid, and the
    /// external ports, the order of the S-matrices after it.
    Chip {
        /// The instances, in the chip's order.
        instances: Vec<ChipPart>,
        /// The external ports' names.
        ports: Vec<String>,
    },
    /// A study of a chip about to run (a `[circuit]` with a `"sweep"` or `"corners"` task),
    /// recorded after [`Event::Chip`]: what it steps through. An [`Event::ChipPoint`] per point
    /// follows, in order.
    ChipStudy {
        /// `"sweep"` or `"corners"`.
        study: String,
        /// A sweep's axes (`instance.parameter`), the second's values fastest; none for corners.
        axes: Vec<SweepAxis>,
        /// The axes' parameters' units, in the same order.
        axis_units: Vec<String>,
        /// The corners (their offsets by `instance.parameter`), after the nominal chip; none for
        /// a sweep.
        corners: Vec<Corner>,
        /// How many points.
        points: usize,
        /// The wavelengths each point's S-parameters are at, µm.
        wavelengths_um: Vec<f64>,
        /// The external ports' names, in the S-matrices' order.
        ports: Vec<String>,
    },
    /// A point of a chip's study: its chip's S-matrices over the band, or why it has none. The
    /// same numbers, to the bit, as the run of that point's own `"spectrum"` job.
    ChipPoint {
        /// The point's index, from 0.
        point: usize,
        /// A sweep's: each axis's value there, in its unit; none for corners.
        at: Vec<f64>,
        /// A corners study's: the corner, `"nominal"` for the first point.
        corner: Option<String>,
        /// Why the chip gave nothing there (an instance's model refused its values); `s` is then
        /// empty.
        error: Option<String>,
        /// The power-normalized S-matrices, one per wavelength: `s[w][q][p]` from port p into
        /// port q, real and imaginary parts.
        s: Vec<Vec<Vec<[f64; 2]>>>,
    },
    // New variants go here, at the end after the last one, so that the earlier variants keep
    // their discriminants (inserting one in between renumbers every variant after it).
    /// A field of a static job ([`crate::thermal`], [`crate::electro`]) on its cross-section,
    /// in its own unit (not scaled): each pixel the value of the cell its centre falls in.
    StaticField {
        /// What it is: `"temperature"` or `"potential"`; with an `electro_optic` table also
        /// `"electric field |E|"` and `"mode's |E|^2"` on the mode's window.
        label: String,
        /// Its unit: `"K"` or `"V"`; `"V/m"` for |E|, `"relative"` for the mode (1 at its peak).
        unit: String,
        /// The axes: across the guide and up, `["y", "z"]`.
        axes: [String; 2],
        /// The picture.
        raster: Raster,
    },
    /// A static job's grid and numbers, recorded after its fields.
    StaticResult {
        /// `"heat"`, `"electrostatic"` or `"current"`.
        physics: String,
        /// The grid's cells across and up.
        cells: [usize; 2],
        /// Its smallest and largest steps, µm.
        step_um: [f64; 2],
        /// The numbers, each with its unit: the probes' means, the largest value, what each
        /// held rectangle gives, what leaves through each side, the balance's imbalance, and a
        /// capacitance matrix or the contacts' Joule power.
        values: Vec<StaticValue>,
    },
    /// The run failed, recorded before its error is returned: what went wrong. A run's record
    /// ends with this or [`Event::Finished`]; one that ends with neither was cut short (the
    /// program was killed).
    Failed {
        /// The error, as photonoxide states it.
        error: String,
        /// How long the run had gone, seconds.
        seconds: f64,
    },
    /// A step of a static job's transient: each probe's mean at its time, recorded as the steps
    /// go (about 2000 of them in a run, the first at t = 0).
    StaticStep {
        /// The time, µs.
        time_us: f64,
        /// Each probe's mean, as `"mean over guide"` in K.
        means: Vec<StaticValue>,
    },
    /// A stage of an optimization's continuation begins: its parameter's value, and what the
    /// optimizer kept from the last stage. Its first evaluation is the last stage's point again,
    /// on the new value; the best so far is of this stage's evaluations.
    Stage {
        /// The stage, from 0.
        stage: usize,
        /// The stages.
        stages: usize,
        /// The parameter.
        parameter: String,
        /// Its value at this stage.
        value: f64,
        /// `"state"` or `"point"`: what the optimizer keeps from one stage to the next.
        keep: String,
        /// The evaluations before it.
        evaluations: usize,
    },
    /// A reported number's grid study ([`crate::convergence`]): a mode's Re n_eff on the run's
    /// grid and on coarser ones, its estimated error and whether it is within the tolerance.
    GridConvergence {
        /// The mode's label, as its [`Event::Mode`] has it, e.g. `"mode 1 of 2"`.
        label: String,
        /// Its wavelength, µm.
        wavelength_um: f64,
        /// The study and its verdict; its value is the run's grid's.
        check: crate::convergence::Convergence,
    },
}

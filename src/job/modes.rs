//! The modes task: its cross-section, its checks and its run.

use schemars::{JsonSchema, SchemaGenerator};
use serde::Deserialize;

use super::check::{
    check_cells, check_finite, check_layer, check_materials, check_step, check_sweep, check_window,
    task_error,
};
use super::chip::CHIP_EXPORTS;
use super::device::DEVICE_EXPORTS;
use super::event::Event;
use super::execute::solver_run;
use super::kind::{EM_FIELDS, Holder, TaskKind, find};
use super::scene::{scene, scene_shapes};
use super::structure::{
    CircleSpec, EtchSpec, RectSpec, RingSpec, StackField, draw, tensor_stack, window_z,
};
use super::{device, in_flight, params, schema, sweep_points};
use crate::geometry::Point;
use crate::raster::Raster;
use crate::run::{Job, Run, Stop};
use crate::stack::{Fill, LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// The most modes a modes job may ask for. The eigensolver keeps twice as many vectors of the
/// grid's size and 21 more (Krylov–Schur's space, which no longer grows with its restarts),
/// and its time grows with them: 500 modes ran six minutes past a one-minute limit before the
/// solve heeded it.
const MOST_MODES: usize = 50;

impl ModesTask {
    /// The axis the modes travel along: `propagation`, y when it isn't given (an older job).
    pub(super) fn along(&self) -> Result<Along> {
        match self.propagation.as_deref() {
            None | Some("y") => Ok(Along::Y),
            Some("x") => Ok(Along::X),
            Some(other) => Err(task_error(format!(
                "unknown propagation \"{other}\": x or y"
            ))),
        }
    }

    /// The window across the guide, µm: `y_um` for modes along x, `x_um` along y.
    pub(super) fn across(&self) -> Result<[f64; 2]> {
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
    pub(super) fn cut(&self) -> Result<f64> {
        Ok(match self.along()? {
            Along::X => self.cut_x_um,
            Along::Y => self.cut_y_um,
        }
        .unwrap_or(0.0))
    }

    /// What [`ModesTask::validate`] refuses of a `[task.pml]`: thicknesses that aren't numbers,
    /// are negative or are all 0, a layer thinner than [`PML_CELLS`] cells of the step, a strength
    /// that isn't positive, and layers across the guide that meet. Their heights against the
    /// window's and the guide's are [`modes_pml`]'s, which knows the stack.
    fn check_pml(&self, pml: &PmlSpec) -> Result<()> {
        let names = ["low across", "high across", "below", "above"];
        for (name, t) in names.iter().zip(pml.thickness_um) {
            if !(t.is_finite() && t >= 0.0) {
                return Err(task_error(format!(
                    "[task.pml] thickness_um {name} must be a number of at least 0, got {t}"
                )));
            }
            let step = self.step_nm / 1000.0;
            if t > 0.0 && t < PML_CELLS * step {
                return Err(task_error(format!(
                    "[task.pml] thickness_um {name} is {t} um, fewer than {PML_CELLS} cells of \
                     the step ({} nm): so thin a layer reflects; make it thicker or 0 for none",
                    self.step_nm
                )));
            }
        }
        if pml.thickness_um.iter().all(|&t| t == 0.0) {
            return Err(task_error(
                "[task.pml] has every thickness_um 0, which opens no edge: give the edges the \
                 light leaks through a thickness, or leave [task.pml] out",
            ));
        }
        let strength = pml.strength();
        if !(strength.is_finite() && strength > 0.0) {
            return Err(task_error(format!(
                "[task.pml] strength must be a positive number, got {strength}"
            )));
        }
        let across = self.across()?;
        let [low, high, _, _] = pml.thickness_um;
        if low + high >= across[1] - across[0] {
            return Err(task_error(format!(
                "[task.pml] thickness_um across the guide, {low} and {high} um, fill the window \
                 across, {} um: leave room for the guide",
                across[1] - across[0]
            )));
        }
        Ok(())
    }

    /// What [`check`] and the run both refuse before solving anything: the propagation, the
    /// windows, the step, the cut, the number of modes, the PML and the sweep.
    pub(super) fn validate(&self) -> Result<()> {
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
        if let Some(above) = self.modes_above {
            if let Some(count) = self.modes {
                return Err(task_error(format!(
                    "modes = {count} and modes_above = {above} are two ways to choose the modes: \
                     give a count or a threshold, not both"
                )));
            }
            if !(above.is_finite() && above > 0.0) {
                return Err(task_error(format!(
                    "modes_above must be a positive effective index, got {above}"
                )));
            }
        }
        if let Some(near) = self.modes_near {
            if let Some(above) = self.modes_above {
                return Err(task_error(format!(
                    "modes_near = {near} is for a count, and modes_above = {above} takes every \
                     mode above it: give one or the other"
                )));
            }
            if !(near.is_finite() && near > 0.0) {
                return Err(task_error(format!(
                    "modes_near must be a positive effective index, got {near}"
                )));
            }
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
        if let Some(pml) = &self.pml {
            self.check_pml(pml)?;
        }
        if let Some(export) = &self.export {
            export.validate()?;
            export.check_fields("a modes job", &crate::export::COMPONENTS)?;
        }
        if let Some(c) = &self.convergence {
            c.validate(self)?;
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

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct SweepSpec {
    pub(super) parameter: String,
    pub(super) from: f64,
    pub(super) to: f64,
    points: usize,
    /// For a width sweep: which rectangle, in the order given (0 by default).
    rect: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ModesTask {
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    pub(super) stack: StackField,
    pub(super) core_nm: Option<f64>,
    pub(super) bottom_oxide_um: Option<f64>,
    pub(super) wavelength_um: f64,
    pub(super) layer: String,
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
    pub(super) step_nm: f64,
    modes: Option<usize>,
    /// Every mode with n_eff above this, instead of a count.
    pub(super) modes_above: Option<f64>,
    /// The count nearest this effective index, instead of the highest index's.
    modes_near: Option<f64>,
    /// Perfectly matched layers inside the window's edges: the window is open, and its leaky
    /// modes are kept with their loss.
    pml: Option<PmlSpec>,
    #[serde(default)]
    pub(super) rect: Vec<RectSpec>,
    #[serde(default)]
    pub(super) circle: Vec<CircleSpec>,
    #[serde(default)]
    pub(super) ring: Vec<RingSpec>,
    /// Writes each mode's complex fields, and a summary table of every mode, into the run's
    /// directory (issue #431; not a sweep's points, which stay pictures only).
    export: Option<ExportSpec>,
    /// The modes' grid checked: each effective index solved again on coarser grids, and its
    /// error estimated against a tolerance ([`crate::convergence`]).
    convergence: Option<ConvergenceSpec>,
    pub(super) sweep: Option<SweepSpec>,
    /// The names of `[task.parameters]`, which a sweep may step through: set after reading.
    #[serde(skip)]
    pub(super) parameters: Vec<String>,
}

/// A modes job's `[task.pml]`: perfectly matched layers inside its window's edges
/// ([`crate::mode::vector::Pml`]), as the façade's cross-section has them.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PmlSpec {
    /// Each layer's thickness, µm, 0 for none: across the guide at its low and high end (the
    /// pictures' left and right), below and above.
    thickness_um: [f64; 4],
    /// The stretching's strength α, s = 1 + iα at the window's edge: 3 by default.
    strength: Option<f64>,
}

/// The PML's strength when `[task.pml]` gives none: Chew et al.'s stretching absorbs well at
/// 3–10 over a layer about a wavelength thick (docs/methods/pml.md).
const PML_STRENGTH: f64 = 3.0;

/// The fewest cells of the step a modes job's PML may be: a thinner one reflects.
const PML_CELLS: f64 = 4.0;

impl PmlSpec {
    fn strength(&self) -> f64 {
        self.strength.unwrap_or(PML_STRENGTH)
    }

    /// The library's PML in the frame the cross-section is solved in: across the guide is its
    /// x, from low to high (the frame keeps the job's order), and up its y.
    fn pml(&self) -> crate::mode::vector::Pml {
        let [west, east, south, north] = self.thickness_um;
        crate::mode::vector::Pml {
            west,
            east,
            south,
            north,
            strength: self.strength(),
        }
    }

    /// What the solver details say of it.
    fn describe(&self) -> String {
        let [low, high, below, above] = self.thickness_um;
        format!(
            "{low}, {high}, {below} and {above} um inside the window's edges (across the guide \
             low and high, below and above; 0 is none, the field zero beyond that edge), \
             strength {}: the coordinate stretched by s = 1 + i {} (u/d)^2 at depth u into a \
             layer d thick (Chew, Jin and Michielssen 1997)",
            self.strength(),
            self.strength()
        )
    }
}

/// A modes job's `[task.convergence]`: the grid study behind each mode's effective index. The
/// modes are solved again on grids each twice as coarse, and the task's grid's Re n_eff, the
/// number reported, is checked by [`crate::convergence::Study::check`].
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConvergenceSpec {
    /// The tolerance on each Re n_eff's estimated error.
    tolerance: f64,
    /// How many grids, the task's among them: 2 (a first-order estimate) or 3 (Richardson's,
    /// the default).
    grids: Option<usize>,
    /// Fail the run when a mode's grid hasn't converged (by default its record says so and the
    /// run goes on).
    require: Option<bool>,
}

impl ConvergenceSpec {
    fn grids(&self) -> usize {
        self.grids.unwrap_or(3)
    }

    /// The tolerance, the grids, and the coarsest grid's cells in the windows and the PML.
    pub(super) fn validate(&self, task: &ModesTask) -> Result<()> {
        if !(self.tolerance.is_finite() && self.tolerance > 0.0) {
            return Err(task_error(format!(
                "[task.convergence] tolerance must be above 0, got {}",
                self.tolerance
            )));
        }
        if !matches!(self.grids(), 2 | 3) {
            return Err(task_error(format!(
                "[task.convergence] grids is 2 or 3 (the task's and coarser ones), got {}",
                self.grids()
            )));
        }
        if task.sweep.is_some() {
            return Err(task_error(
                "[task.convergence] checks the modes at the task's wavelength, not a sweep's \
                 points: give one or the other",
            ));
        }
        let coarsest = task.step_nm * f64::from(1u32 << (self.grids() - 1));
        let name = |what: &str| format!("the coarsest grid's (step {coarsest} nm) {what}");
        check_cells(&name("window across the guide"), task.across()?, coarsest)?;
        if let Some(z) = task.z_um {
            check_cells(&name("z_um"), z, coarsest)?;
        }
        if let Some(pml) = &task.pml
            && let Some(&t) = pml
                .thickness_um
                .iter()
                .find(|&&t| t > 0.0 && t < PML_CELLS * coarsest / 1000.0)
        {
            return Err(task_error(format!(
                "[task.convergence]: a PML {t} um thick is fewer than {PML_CELLS} cells of the \
                 coarsest grid's {coarsest} nm: take fewer grids or a thicker PML"
            )));
        }
        Ok(())
    }
}

/// A job's `[task.export]`: what its run writes into its directory beside `events.jsonl`
/// ([`crate::export`]). A job without one writes every table its run has, as CSV, and no
/// fields.
#[derive(Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportSpec {
    /// `"csv"` (the tables), `"npz"` (the fields), `"both"` or `"none"`: `"both"` when
    /// `fields` is given, `"csv"` otherwise.
    format: Option<String>,
    /// Which tables, by name: `"modes"`, `"sweep"`, `"s_parameters"`, `"spectra"`,
    /// `"resonances"`, `"study"`, `"history"`, `"static"`, `"transient"`; every one the run has
    /// by default.
    tables: Option<Vec<String>>,
    /// Which fields: `"Ex"`, `"Ey"`, `"Ez"`, `"Hx"`, `"Hy"`, `"Hz"`, `"temperature"`,
    /// `"potential"`; every one the kind computes by default.
    fields: Option<Vec<String>>,
    /// `"f32"` or `"f64"` (the default): the npz's fields' precision (the grids and the small
    /// numbers stay `f64`).
    precision: Option<String>,
    /// The compression level: deflate's 0 (store) to 9 (smallest), 6 by default; Zstandard's
    /// 1 to 22, 3 by default. The csv is never compressed.
    level: Option<u8>,
    /// The npz's compression: `"deflate"` (the default, photonoxide's own) or `"zstd"`
    /// (Zstandard, from the user's own libzstd, loaded at run time: docs/libraries.md).
    compression: Option<String>,
    /// The fields' file: `"npz"` (the default, NumPy's, photonoxide's own) or `"hdf5"` (an
    /// `.h5` of the same arrays, from the user's own libhdf5, loaded at run time).
    container: Option<String>,
}

/// The fields a `[task.export]` may name.
const EXPORT_FIELDS: [&str; 8] = [
    "Ex",
    "Ey",
    "Ez",
    "Hx",
    "Hy",
    "Hz",
    "temperature",
    "potential",
];

impl ExportSpec {
    pub(super) fn validate(&self) -> Result<()> {
        if !matches!(self.format(), "npz" | "csv" | "both" | "none") {
            return Err(task_error(format!(
                "[task.export] format must be \"npz\", \"csv\", \"both\" or \"none\", got \"{}\"",
                self.format()
            )));
        }
        if let Some(t) = self
            .tables
            .iter()
            .flatten()
            .find(|t| crate::export::tables::Table::parse(t).is_none())
        {
            return Err(task_error(format!(
                "[task.export] has no table \"{t}\": the tables are {}",
                crate::export::tables::Table::ALL
                    .map(|t| t.name())
                    .join(", ")
            )));
        }
        if let Some(f) = self
            .fields
            .iter()
            .flatten()
            .find(|f| !EXPORT_FIELDS.contains(&f.as_str()))
        {
            return Err(task_error(format!(
                "[task.export] has no field \"{f}\": the fields are {}",
                EXPORT_FIELDS.join(", ")
            )));
        }
        if self.fields.is_some() && !self.npz() {
            return Err(task_error(format!(
                "[task.export] fields are written as npz, and format = \"{}\" writes none: \
                 make it \"npz\" or \"both\"",
                self.format()
            )));
        }
        if self.tables.is_some() && !self.csv() {
            return Err(task_error(format!(
                "[task.export] tables are written as csv, and format = \"{}\" writes none: \
                 make it \"csv\" or \"both\"",
                self.format()
            )));
        }
        if let Some(p) = &self.precision
            && !matches!(p.as_str(), "f32" | "f64")
        {
            return Err(task_error(format!(
                "[task.export] precision must be \"f32\" or \"f64\", got \"{p}\""
            )));
        }
        match self.compression.as_deref() {
            None | Some("deflate") => {
                if let Some(level) = self.level
                    && level > 9
                {
                    return Err(task_error(format!(
                        "[task.export] level must be at most 9, got {level}"
                    )));
                }
            }
            Some("zstd") => {
                if let Some(level) = self.level
                    && !(1..=22).contains(&level)
                {
                    return Err(task_error(format!(
                        "[task.export] zstd's level must be 1 to 22, got {level}"
                    )));
                }
                if self.npz() {
                    crate::export::codec::compressor("zstd")
                        .map_err(|e| task_error(format!("[task.export] compression: {e}")))?;
                }
            }
            Some(other) => {
                return Err(task_error(format!(
                    "[task.export] compression must be \"deflate\" or \"zstd\", got \"{other}\""
                )));
            }
        }
        match self.container.as_deref() {
            None | Some("npz") => {}
            Some("hdf5") => {
                if self.compression.as_deref() == Some("zstd") {
                    return Err(task_error(
                        "[task.export]: an HDF5 file takes libhdf5's own deflate, not zstd \
                         (its filter is a plugin of its own): leave compression out",
                    ));
                }
                if self.npz() {
                    crate::export::codec::container("hdf5")
                        .map_err(|e| task_error(format!("[task.export] container: {e}")))?;
                }
            }
            Some(other) => {
                return Err(task_error(format!(
                    "[task.export] container must be \"npz\" or \"hdf5\", got \"{other}\""
                )));
            }
        }
        Ok(())
    }

    /// The format, as given or by default.
    fn format(&self) -> &str {
        match (&self.format, &self.fields) {
            (Some(f), _) => f,
            (None, Some(_)) => "both",
            (None, None) => "csv",
        }
    }

    pub(super) fn npz(&self) -> bool {
        matches!(self.format(), "npz" | "both")
    }

    fn csv(&self) -> bool {
        matches!(self.format(), "csv" | "both")
    }

    /// The tables to write: `None` for none, `Some(None)` for every one the run has.
    pub(super) fn tables(&self) -> Option<Option<Vec<crate::export::tables::Table>>> {
        self.csv().then(|| {
            self.tables.as_ref().map(|t| {
                t.iter()
                    .filter_map(|t| crate::export::tables::Table::parse(t))
                    .collect()
            })
        })
    }

    /// Whether the field `name` is written: the npz is, and `fields` names it or is absent.
    pub(super) fn keeps(&self, name: &str) -> bool {
        self.npz()
            && self
                .fields
                .as_ref()
                .is_none_or(|f| f.iter().any(|f| f == name))
    }

    /// Refuses fields `has` doesn't hold, the fields a job of `what` computes.
    fn check_fields(&self, what: &str, has: &[&str]) -> Result<()> {
        if !self.npz() {
            return Ok(());
        }
        if has.is_empty() {
            return Err(task_error(format!(
                "[task.export]: {what} has no fields to write: format = \"csv\" for its tables"
            )));
        }
        if let Some(f) = self
            .fields
            .iter()
            .flatten()
            .find(|f| !has.contains(&f.as_str()))
        {
            return Err(task_error(format!(
                "[task.export]: {what} has no field \"{f}\": it has {}",
                has.join(", ")
            )));
        }
        Ok(())
    }

    fn precision(&self) -> crate::export::Precision {
        match self.precision.as_deref() {
            Some("f32") => crate::export::Precision::F32,
            _ => crate::export::Precision::F64,
        }
    }

    fn level(&self) -> u8 {
        self.level.unwrap_or(6)
    }

    /// An `.npz` at this export's precision, compression and level.
    pub(super) fn npz_file(&self) -> Result<crate::export::Npz> {
        if self.container.as_deref() == Some("hdf5") {
            let hdf5 = crate::export::codec::container("hdf5")
                .map_err(|e| task_error(format!("[task.export] container: {e}")))?;
            return Ok(crate::export::Npz::contained(
                self.precision(),
                hdf5,
                self.level(),
            ));
        }
        if self.compression.as_deref() == Some("zstd") {
            let zstd = crate::export::codec::compressor("zstd")
                .map_err(|e| task_error(format!("[task.export] compression: {e}")))?;
            let level = self.level.map_or(zstd.default_level(), i32::from);
            return Ok(crate::export::Npz::compressed(
                self.precision(),
                zstd,
                level,
            ));
        }
        Ok(crate::export::Npz::new(self.precision(), self.level()))
    }
}

/// `job`'s `[task.export]`, checked, or the default (every table, no fields) when it has none.
pub(super) fn export_of(job: &Job) -> Result<ExportSpec> {
    let Some(value) = job.task().get("export") else {
        return Ok(ExportSpec::default());
    };
    let spec: ExportSpec = value
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(format!("[task.export]: {}", e.message())))?;
    spec.validate()?;
    Ok(spec)
}

/// The `[task.export]` of the job in the run directory `dir` (its `job.toml`, `job.json` or
/// `job.yaml`), or the default when it has none or none can be read.
pub(super) fn export_in(dir: &std::path::Path) -> ExportSpec {
    ["toml", "json", "yaml"]
        .iter()
        .map(|ext| dir.join(format!("job.{ext}")))
        .find(|p| p.exists())
        .and_then(|p| Job::load(&p).ok())
        .and_then(|job| export_of(&job).ok())
        .unwrap_or_default()
}

/// What a job computes that `[task.export]` may write as fields, and what to call it in an
/// error: none for a structure, a device's study, fit or optimization, or a chip.
fn export_fields(job: &Job) -> Result<(&'static str, &'static [&'static str])> {
    let name = job.task().get("kind").and_then(|k| k.as_str());
    Ok(match Holder::of(job) {
        Holder::Chip => name
            .and_then(|n| find(Holder::Chip, n))
            .map_or(CHIP_EXPORTS, |k| k.exports(job)),
        Holder::Device => device::task_kind(job)?.map_or(DEVICE_EXPORTS, |k| k.exports(job)),
        Holder::Solver => name
            .and_then(|n| find(Holder::Solver, n))
            .map_or(("this job", &[]), |k| k.exports(job)),
    })
}

/// Checks `job`'s `[task.export]`: its own fields, and that the fields it asks for are ones the
/// job computes.
pub(super) fn check_export(job: &Job) -> Result<()> {
    let export = export_of(job)?;
    let (what, has) = export_fields(job)?;
    export.check_fields(what, has)
}

/// The axis a modes job's modes travel along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Along {
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
    pub(super) fn across(self) -> &'static str {
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
pub(super) struct Frame {
    pub(super) rect: Vec<RectSpec>,
    pub(super) circle: Vec<CircleSpec>,
    pub(super) ring: Vec<RingSpec>,
    pub(super) cut: f64,
}

impl Frame {
    pub(super) fn new(
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
                    etch: r.etch,
                })
                .collect(),
            circle: circles
                .iter()
                .map(|c| CircleSpec {
                    layer: c.layer.clone(),
                    center_um: turn(c.center_um),
                    radius_um: c.radius_um,
                    etch: c.etch,
                })
                .collect(),
            ring: rings
                .iter()
                .map(|r| RingSpec {
                    layer: r.layer.clone(),
                    center_um: turn(r.center_um),
                    radius_um: r.radius_um,
                    width_um: r.width_um,
                    etch: r.etch,
                })
                .collect(),
            cut: match along {
                Along::X => -cut,
                Along::Y => cut,
            },
        }
    }

    /// The frame's structure: the stack with its shapes drawn on it.
    pub(super) fn draw(&self, stack: LayerStack) -> Result<Structure> {
        draw(stack, &self.rect, &self.circle, &self.ring)
    }
}

/// The cross-section of `s` cut at y over `x` × `z` (µm) on a grid of about `step`, with every
/// shape's edge on the cut and every layer interface on a node, so a sweep over a width varies
/// it smoothly; each cell the permittivity at its centre at `wavelength`.
#[allow(clippy::too_many_arguments)]
pub(super) fn cross_section(
    s: &Structure,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
    cut_y: f64,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    wavelength: Wavelength,
    along: Along,
) -> Result<crate::mode::vector::CrossSection> {
    // every material the cut can meet must know this wavelength
    check_materials(s.stack(), &[wavelength.to_um()])?;
    // the edges the cut crosses, and the layers' interfaces; a sloped wall's foot too, and a
    // partial etch's floor
    let mut x_edges = Vec::new();
    let mut z_edges = vec![0.0];
    // how far an etched shape's walls reach out at its foot, µm, and its floor's height
    let reach = |layer: &str, etch: Option<EtchSpec>| -> (f64, Option<f64>) {
        let Some((l, _, top)) = s.stack().layer(layer) else {
            return (0.0, None);
        };
        let etch = EtchSpec::etch(etch);
        let depth = etch.depth.unwrap_or(l.thickness).to_um();
        (
            depth * etch.run(),
            etch.depth.map(|d| top.to_um() - d.to_um()),
        )
    };
    for r in rects {
        let (out, floor) = reach(&r.layer, r.etch);
        z_edges.extend(floor);
        if (cut_y - r.center_um[1]).abs() < r.size_um[1] / 2.0 + out {
            x_edges.extend([
                r.center_um[0] - r.size_um[0] / 2.0,
                r.center_um[0] + r.size_um[0] / 2.0,
            ]);
            if out > 0.0 {
                x_edges.extend([
                    r.center_um[0] - r.size_um[0] / 2.0 - out,
                    r.center_um[0] + r.size_um[0] / 2.0 + out,
                ]);
            }
        }
    }
    let mut circle_edges = |center: [f64; 2], radius: f64| {
        let d = cut_y - center[1];
        if radius > 0.0 && d.abs() < radius {
            let half = (radius * radius - d * d).sqrt();
            x_edges.extend([center[0] - half, center[0] + half]);
        }
    };
    for c in circles {
        let (out, floor) = reach(&c.layer, c.etch);
        z_edges.extend(floor);
        circle_edges(c.center_um, c.radius_um);
        if out > 0.0 {
            circle_edges(c.center_um, c.radius_um + out);
        }
    }
    for r in rings {
        let (out, floor) = reach(&r.layer, r.etch);
        z_edges.extend(floor);
        circle_edges(r.center_um, r.radius_um - r.width_um / 2.0);
        circle_edges(r.center_um, r.radius_um + r.width_um / 2.0);
        if out > 0.0 {
            circle_edges(r.center_um, r.radius_um - r.width_um / 2.0 - out);
            circle_edges(r.center_um, r.radius_um + r.width_um / 2.0 + out);
        }
    }
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
            // the material at the centre, or a sloped wall's share of the cell
            let fill = s.cut_cell(
                Point::um(xc, cut_y),
                zc,
                [1.0, 0.0],
                [xs[i + 1] - xs[i], zs[j + 1] - zs[j]],
            );
            let anisotropic = match fill {
                Fill::Uniform(m) => m.is_anisotropic(),
                Fill::Wall {
                    inside, outside, ..
                } => inside.is_anisotropic() || outside.is_anisotropic(),
            };
            cells.push(if anisotropic {
                cell_tensor(fill.tensor(wavelength)?, along)?
            } else {
                // an isotropic cell as it always was, to the bit
                crate::mode::vector::Permittivity::isotropic(
                    fill.permittivity(wavelength)
                        .unwrap_or(num_complex::Complex64::new(f64::NAN, 0.0)),
                )
            });
        }
    }
    crate::mode::vector::CrossSection::new(xs, zs, cells)
}

/// A tensor `t` in the chip's frame as the full-vector solver takes it, in the cross-section's
/// axes of a modes job along `along`: x across the guide, y up (the chip's z), z along it. Along
/// y the axes are the chip's x, z and y; along x the job is turned a quarter turn clockwise
/// ([`Frame`]), so across is the chip's y and along is its −x.
pub(super) fn cell_tensor(
    t: [[num_complex::Complex64; 3]; 3],
    along: Along,
) -> Result<crate::mode::vector::Permittivity> {
    // the chip's axes (and signs) that the cross-section's x, y and z are
    let axes: [(usize, f64); 3] = match along {
        Along::Y => [(0, 1.0), (2, 1.0), (1, 1.0)],
        Along::X => [(1, 1.0), (2, 1.0), (0, -1.0)],
    };
    let turned = std::array::from_fn(|a| {
        std::array::from_fn(|b| {
            let ((i, si), (j, sj)) = (axes[a], axes[b]);
            t[i][j] * (si * sj)
        })
    });
    crate::mode::vector::Permittivity::from_tensor(turned).map_err(|e| match e {
        Error::InvalidValue { reason, .. } => task_error(format!(
            "an oriented crystal along {}: {reason}; orient it so one of its principal axes \
             lies along the guide",
            along.name()
        )),
        other => other,
    })
}

/// A modes job's oriented crystals must have a principal axis along its guide, or the
/// full-vector solver can't take their tensors: what [`check`] and the run both refuse.
pub(super) fn check_tensors(stack: &LayerStack, along: Along, wavelength_um: f64) -> Result<()> {
    let w = Wavelength::um(wavelength_um)?;
    let mut media = vec![stack.substrate(), stack.cladding()];
    for l in stack.layers() {
        media.extend([&l.material, &l.background]);
    }
    for m in media.into_iter().filter(|m| m.is_anisotropic()) {
        cell_tensor(m.tensor(w)?, along)?;
    }
    Ok(())
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
pub(super) fn intensity(
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

/// What a modes job needs for there to be a guide in its cross-section: a z window that
/// reaches its layer, and a cut that meets a shape within the window across. Without either
/// the solver would find the modes of bare cladding in a box, and call them the job's.
pub(super) fn modes_guide(task: &ModesTask, s: &Structure, z: [f64; 2]) -> Result<()> {
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

/// What a modes job's `[task.pml]` needs of its window: layers below and above that leave room
/// between them, and that stay clear of its layer, and a shape meeting the cut between the
/// layers across the guide. A PML over the guide would absorb its modes. A guide may run into
/// a layer across it, as a rib's slab does: the slab's light leaks into the layer.
pub(super) fn modes_pml(task: &ModesTask, s: &Structure, z: [f64; 2]) -> Result<()> {
    let Some(pml) = &task.pml else {
        return Ok(());
    };
    let [low, high, below, above] = pml.thickness_um;
    if below + above >= z[1] - z[0] {
        return Err(task_error(format!(
            "[task.pml] thickness_um below and above, {below} and {above} um, fill the window's \
             height, {} um: leave room for the guide",
            z[1] - z[0]
        )));
    }
    let (_, bottom, top) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    let (bottom, top) = (bottom.to_um(), top.to_um());
    if below > 0.0 && z[0] + below > bottom {
        return Err(task_error(format!(
            "[task.pml] below reaches z = {} um, above the bottom of the layer {} ({bottom} um): \
             it would absorb the guide; make it thinner or the window deeper",
            z[0] + below,
            task.layer
        )));
    }
    if above > 0.0 && z[1] - above < top {
        return Err(task_error(format!(
            "[task.pml] above reaches down to z = {} um, below the top of the layer {} ({top} \
             um): it would absorb the guide; make it thinner or the window taller",
            z[1] - above,
            task.layer
        )));
    }
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    let open = [across[0] + low, across[1] - high];
    let step = task.step_nm / 4000.0;
    let count = ((open[1] - open[0]) / step).ceil() as usize;
    let meets = (0..=count).any(|k| {
        let a = (open[0] + k as f64 * step).min(open[1]);
        let p = match along {
            Along::X => Point::um(cut, a),
            Along::Y => Point::um(a, cut),
        };
        s.shapes(&task.layer).iter().any(|shape| shape.contains(p))
    });
    if !meets {
        return Err(task_error(format!(
            "[task.pml] across the guide leaves {} = {} to {} um open, and no shape of the layer \
             {} meets the cut there: the layers would absorb the guide",
            along.across(),
            open[0],
            open[1],
            task.layer
        )));
    }
    Ok(())
}

/// The share of a mode's |E|² (over the cells' areas) inside the PML of `cs`: near 1 for the
/// PML's own modes, whose fields grow into the layers, and small for a guided or leaky mode,
/// which the layers only absorb.
fn pml_share(fields: &crate::mode::fields::Fields, cs: &crate::mode::vector::CrossSection) -> f64 {
    let pml = cs.pml();
    let (x, y) = (cs.x(), cs.y());
    let open_x = [x[0] + pml.west, x[x.len() - 1] - pml.east];
    let open_y = [y[0] + pml.south, y[y.len() - 1] - pml.north];
    let (mut total, mut inside) = (0.0, 0.0);
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let e: f64 = fields.e(i, j).iter().map(|v| v.norm_sqr()).sum::<f64>()
                * (x[i + 1] - x[i])
                * (y[j + 1] - y[j]);
            total += e;
            if xc < open_x[0] || xc > open_x[1] || yc < open_y[0] || yc > open_y[1] {
                inside += e;
            }
        }
    }
    if total > 0.0 { inside / total } else { 0.0 }
}

/// The most of a mode's |E|² the PML may hold for the mode to be the guide's (guided or leaky)
/// and not the PML's own.
const PML_SHARE: f64 = 0.5;

/// The highest index the cross-section of a modes job holds at `wavelength_um`, from the
/// materials its window reaches: the substrate below the stack, each layer's surroundings, its
/// material where a shape meets the cut (sampled as [`modes_guide`] does), and the cladding
/// above. No mode's effective index is above it.
fn window_highest(task: &ModesTask, s: &Structure, z: [f64; 2], wavelength_um: f64) -> Result<f64> {
    let w = Wavelength::um(wavelength_um)?;
    // an oriented crystal's highest principal index
    let index = |m: &crate::material::Material| -> Result<f64> {
        Ok(principal_indices(m, w)?.into_iter().fold(0.0, f64::max))
    };
    let stack = s.stack();
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    let step = task.step_nm / 4000.0;
    let samples = ((across[1] - across[0]) / step).ceil() as usize;
    let mut highest = 0.0f64;
    if z[0] < 0.0 {
        highest = highest.max(index(stack.substrate())?);
    }
    let mut bottom = 0.0;
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        if z[1] > bottom && z[0] < top {
            highest = highest.max(index(&layer.background)?);
            let meets = (0..=samples).any(|k| {
                let a = (across[0] + k as f64 * step).min(across[1]);
                let p = match along {
                    Along::X => Point::um(cut, a),
                    Along::Y => Point::um(a, cut),
                };
                s.shapes(&layer.name).iter().any(|shape| shape.contains(p))
            });
            if meets {
                highest = highest.max(index(&layer.material)?);
            }
        }
        bottom = top;
    }
    if z[1] > bottom {
        highest = highest.max(index(stack.cladding())?);
    }
    Ok(highest)
}

/// The highest index of the media a modes job's guides sit in, within its window, at
/// `wavelength_um`: the backgrounds of its layers, and the substrate and cladding where the
/// window reaches them. A guided mode's n_eff is above it. The window has no absorbing layers,
/// so a mode not above it isn't a guide's: it is a mode of the closed box the window makes,
/// its n_eff below the cladding's, even below 1. The job knows its media, so it takes them
/// from its stack: a bare cross-section reads them at its open edges
/// ([`crate::mode::vector::CrossSection::surrounding_index`]), the same index unless a shape
/// reaches the window's side.
pub(super) fn window_edge_highest(
    _task: &ModesTask,
    s: &Structure,
    z: [f64; 2],
    wavelength_um: f64,
) -> Result<f64> {
    Ok(window_media(s, z, wavelength_um)?
        .into_iter()
        .fold(0.0f64, f64::max))
}

/// The lowest index of the media [`window_edge_highest`] reads. A mode of an open window
/// (a PML) below it is below every medium around the guides, and so radiates into all of them:
/// it is the window's discretized radiation, not a mode the guides hold. Above it, a mode is
/// evanescent in at least one of them, and a mode not above the highest leaks into the others.
fn window_edge_lowest(s: &Structure, z: [f64; 2], wavelength_um: f64) -> Result<f64> {
    let media = window_media(s, z, wavelength_um)?;
    Ok(media.iter().copied().reduce(f64::min).unwrap_or(0.0))
}

/// The indices of the media a modes job's window reaches around its guides at `wavelength_um`,
/// bottom to top: the substrate and the cladding where the window reaches them, and the
/// backgrounds of its layers.
fn window_media(s: &Structure, z: [f64; 2], wavelength_um: f64) -> Result<Vec<f64>> {
    let w = Wavelength::um(wavelength_um)?;
    // an oriented crystal's lowest and highest principal indices, an isotropic medium's one
    let index = |m: &crate::material::Material| -> Result<Vec<f64>> {
        let n = principal_indices(m, w)?;
        let low = n.into_iter().fold(f64::INFINITY, f64::min);
        let high = n.into_iter().fold(0.0, f64::max);
        Ok(if low == high {
            vec![high]
        } else {
            vec![low, high]
        })
    };
    let stack = s.stack();
    let mut media = Vec::new();
    if z[0] < 0.0 {
        media.extend(index(stack.substrate())?);
    }
    let mut bottom = 0.0;
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        if z[1] > bottom && z[0] < top {
            media.extend(index(&layer.background)?);
        }
        bottom = top;
    }
    if z[1] > bottom {
        media.extend(index(stack.cladding())?);
    }
    Ok(media)
}

/// A medium's principal refractive indices (real parts) at `w`: an isotropic one's three the
/// same, as its permittivity gives them.
fn principal_indices(m: &crate::material::Material, w: Wavelength) -> Result<[f64; 3]> {
    Ok(m.principal_permittivity(w)?.map(|e| e.re.max(0.0).sqrt()))
}

/// A modes job's `modes_above` must be below the highest index its cross-section holds at each
/// of `wavelengths_um`, or no mode lies above it. What [`check`] and the run both refuse.
pub(super) fn check_threshold(
    task: &ModesTask,
    s: &Structure,
    z: [f64; 2],
    wavelengths_um: &[f64],
) -> Result<()> {
    let Some(above) = task.modes_above else {
        return Ok(());
    };
    for &w in wavelengths_um {
        let highest = window_highest(task, s, z, w)?;
        if above >= highest {
            return Err(task_error(format!(
                "modes_above = {above} is not below the highest index the cross-section holds at \
                 {w} um, {highest:.4}: no mode lies above it (the cladding's index gives every \
                 guided mode)"
            )));
        }
    }
    Ok(())
}

/// A modes job's window height: `z_um`, else 1 µm below and above its layer.
pub(super) fn modes_z(task: &ModesTask, s: &Structure) -> Result<[f64; 2]> {
    window_z(task.z_um, &task.layer, s, task.step_nm)
}

/// A modes job's scene: a block behind the cut along the guide, as deep as the window across it
/// is wide.
pub(super) fn modes_scene(task: &ModesTask, s: &Structure) -> Result<Event> {
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

/// A modes job's task, its parameters and expressions resolved, with `set` (a parameter and its
/// value) if given.
pub(super) fn modes_task(job: &Job, set: Option<(&str, f64)>) -> Result<ModesTask> {
    let mut task: ModesTask = params::at(job.task(), set)?;
    task.parameters = params::names(job.task());
    Ok(task)
}

/// Writes a mode's complex fields as `<run's directory>/<label, spaces to dashes>.npz`, per its
/// job's `[task.export]` ([`crate::export::write_mode_npz`]); a no-op for a run kept only in
/// memory (a farm's worker, most tests) or an `export` whose format is `"csv"` only.
fn export_mode(
    export: &ExportSpec,
    job: &Job,
    run: &Run,
    label: &str,
    m: &crate::mode::vector::VectorMode,
    fields: &crate::mode::fields::Fields,
    wavelength: Wavelength,
) -> Result<()> {
    if !export.npz() || run.dir().as_os_str().is_empty() {
        return Ok(());
    }
    let path = run.dir().join(format!("{}.npz", label.replace(' ', "-")));
    crate::export::write_mode_into(
        export.npz_file()?,
        &path,
        m,
        fields,
        wavelength,
        &crate::export::Meta::of(job, run),
        &|name| export.keeps(name),
    )?;
    Ok(())
}

/// The `"modes"` kind: the guided modes of a waveguide's cross-section, by the full-vector
/// solver.
pub(super) struct ModesKind;

impl TaskKind for ModesKind {
    fn name(&self) -> &'static str {
        "modes"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        schema::task::<ModesTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        let task = modes_task(job, None)?;
        task.validate()?;
        let stack = tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
        let s = draw(stack, &task.rect, &task.circle, &task.ring)?;
        let z = modes_z(&task, &s)?;
        modes_guide(&task, &s, z)?;
        modes_pml(&task, &s, z)?;
        let mut wavelengths = Vec::new();
        if let Some(sw) = task.sweep.iter().find(|sw| sw.parameter == "wavelength") {
            wavelengths.extend([sw.from, sw.to]);
        }
        if task.modes_above.is_some() {
            // (each material's data at these wavelengths is checked first, as below)
            check_materials(s.stack(), &[task.wavelength_um])?;
            check_materials(s.stack(), &wavelengths)?;
        }
        let mut at = vec![task.wavelength_um];
        at.extend(&wavelengths);
        check_threshold(&task, &s, z, &at)?;
        for &w in &at {
            check_tensors(s.stack(), task.along()?, w)?;
        }
        // a parameter's sweep: its shapes at both ends must be shapes
        if let Some(sw) = task
            .sweep
            .iter()
            .find(|sw| task.parameters.contains(&sw.parameter))
        {
            for end in [sw.from, sw.to] {
                let at = modes_task(job, Some((&sw.parameter, end)))?;
                let stack = tensor_stack(&at.stack, at.core_nm, at.bottom_oxide_um)?;
                draw(stack, &at.rect, &at.circle, &at.ring)
                    .and_then(|swept| modes_pml(&task, &swept, z))
                    .map_err(|e| {
                        task_error(format!("at the sweep's {} = {end}: {e}", sw.parameter))
                    })?;
            }
        }
        check_layer(&s, &task.layer, &at)
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        super::check(job)?;
        let task = modes_task(job, None)?;
        let stack = tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
        modes_scene(&task, &draw(stack, &task.rect, &task.circle, &task.ring)?)
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        solver_run(job, run, stop, self.name(), modes)
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        ("a modes job", EM_FIELDS)
    }
}

pub(super) fn modes(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task = modes_task(job, None)?;
    task.validate()?;
    let stack = || tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um);
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
    let mut at = vec![task.wavelength_um];
    if let Some(sw) = task.sweep.iter().find(|sw| sw.parameter == "wavelength") {
        at.extend([sw.from, sw.to]);
    }
    check_threshold(&task, &s, z, &at)?;
    modes_pml(&task, &s, z)?;
    // the window is open where it has a PML: its leaky modes are kept
    let open = task.pml.is_some();
    // the modes of a cross-section: a count from the highest index (or `modes_near`), or every
    // one above a threshold; `None` when stopped. In a closed window, of a count only the
    // guided ones are kept: a mode not above the window's edges' index is the closed window's,
    // not the guide's. In an open one every mode is kept but the PML's own, most of their
    // field inside it
    let solve = |cs: &crate::mode::vector::CrossSection,
                 w: Wavelength|
     -> Result<Option<Vec<crate::mode::vector::VectorMode>>> {
        let stopped = || stop.reason().is_some();
        let found = match task.modes_above {
            Some(above) => crate::mode::vector::modes_above_until(
                cs,
                w,
                above,
                &crate::mode::region::Search::default(),
                stopped,
            )?
            .map(|found| found.modes),
            None => {
                let found =
                    crate::mode::vector::modes_until(cs, w, count, task.modes_near, stopped)?;
                if !open {
                    let edge = window_edge_highest(&task, &s, z, w.to_um())?;
                    return Ok(found
                        .map(|modes| modes.into_iter().filter(|m| m.is_guided(edge)).collect()));
                }
                found
            }
        };
        if !open {
            return Ok(found);
        }
        let Some(found) = found else {
            return Ok(None);
        };
        let lowest = window_edge_lowest(&s, z, w.to_um())?;
        let mut kept = Vec::with_capacity(found.len());
        for m in found {
            if m.effective_index().re > lowest && pml_share(&m.fields(cs)?, cs) <= PML_SHARE {
                kept.push(m);
            }
        }
        Ok(Some(kept))
    };
    // a mode's kind and loss in an open window, recorded after its pictures
    let loss = |point: Option<usize>,
                label: &str,
                m: &crate::mode::vector::VectorMode,
                w: Wavelength|
     -> Result<Option<Event>> {
        if !open {
            return Ok(None);
        }
        let edge = window_edge_highest(&task, &s, z, w.to_um())?;
        Ok(Some(Event::ModeLoss {
            point,
            label: label.to_owned(),
            kind: if m.is_guided(edge) { "guided" } else { "leaky" }.into(),
            loss_db_per_cm: crate::mode::dispersion::loss_db_per_cm(m.effective_index(), w),
        }))
    };
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
    // the cross-section with its PML, if the job has one
    let opened = |cs: crate::mode::vector::CrossSection| match &task.pml {
        Some(pml) => cs.with_pml(pml.pml()),
        None => Ok(cs),
    };
    let cs = opened(cross_section(
        &fs,
        &frame.rect,
        &frame.circle,
        &frame.ring,
        frame.cut,
        across,
        z,
        step,
        lam,
        along,
    )?)?;
    let edge = window_edge_highest(&task, &s, z, lam.to_um())?;
    // what an open window keeps of the modes it solves, and how it names them
    let kept = || -> Result<String> {
        Ok(format!(
            "guided above {edge:.4}, the highest index of the media around the guides, and \
             leaky from there down to {:.4}, the lowest, each with its loss; left out are the \
             PML's own modes (more than half of their |E|^2 inside the PML, where their fields \
             grow) and those below the lowest index, which radiate into every medium around \
             the guides: the window's radiation, not a mode the guides hold",
            window_edge_lowest(&s, z, lam.to_um())?
        ))
    };
    let asked = match (task.modes_above, open) {
        (Some(above), false) => format!("every one with n_eff above {above}"),
        (Some(above), true) => format!("every one with Re n_eff above {above}: {}", kept()?),
        (None, false) => format!(
            "{count}, the guided ones kept: n_eff above {edge:.4}, the highest index of \
             the media around the guides (the window has no absorbing layers, so a mode below \
             it is the window's own)"
        ),
        (None, true) => format!("{count}: {}", kept()?),
    };
    let mut details = vec![
        ["modes asked for".into(), asked],
        [
            "eigen-solver".into(),
            match (task.modes_above.is_some(), task.modes_near) {
                (true, _) => "contour integrals (FEAST) over n_eff from the threshold to the \
                              highest index, 16 points, each a multifrontal LU; an eigenpair \
                              is accepted at a residual of 1e-10"
                    .into(),
                (false, None) => "shift-and-invert Arnoldi about the highest index with \
                                  Krylov-Schur restarts, sparse LU of the shifted matrix; an \
                                  eigenpair is accepted at a residual of 1e-9"
                    .into(),
                (false, Some(near)) => format!(
                    "shift-and-invert Arnoldi about n_eff = {near} with Krylov-Schur \
                     restarts, sparse LU of the shifted matrix; an eigenpair is accepted at a \
                     residual of 1e-9"
                ),
            },
        ],
        [
            "nodes".into(),
            "on every shape's edge and layer interface, cells of about the step between \
             them"
                .into(),
        ],
    ];
    if let Some(pml) = &task.pml {
        details.push(["PML".into(), pml.describe()]);
        details.push([
            "loss".into(),
            "of the power along the guide, 20 log10(e) k0 Im n_eff in dB/cm, from n_eff on \
             this grid with this PML: a finer grid and a thicker PML (or another strength) \
             say how far it has converged"
                .into(),
        ]);
    }
    run.record(&Event::Solver {
        module: "mode::vector".into(),
        cells: [cs.x().len() - 1, cs.y().len() - 1],
        step_um: step,
        unknowns: cs.unknowns(),
        details,
    })?;
    // the solve itself heeds the stop and the time limit, at each of its steps
    let Some(found) = solve(&cs, lam)? else {
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
            label: label.clone(),
            wavelength_um: lam.to_um(),
            component,
            values,
        })?;
        if let Some(event) = loss(None, &label, m, lam)? {
            run.record(&event)?;
        }
        if let Some(export) = &task.export {
            export_mode(export, job, run, &label, m, &fields, lam)?;
        }
    }
    for (k, m) in sorted.iter().enumerate() {
        run.record(&Event::SolveError {
            point: None,
            value: lam.to_um(),
            measure: format!("eigen-residual, mode {} of {}", k + 1, sorted.len()),
            error: m.residual(&cs)?,
        })?;
    }
    // the grid study: each mode's Re n_eff on grids twice, then four times, as coarse, the
    // run's grid's checked by it; a mode a coarser grid doesn't find hasn't converged
    if let Some(spec) = &task.convergence {
        let reported: Vec<f64> = sorted.iter().map(|m| m.effective_index().re).collect();
        let mut coarse: Vec<(f64, Vec<f64>)> = Vec::new();
        for g in (1..spec.grids()).rev() {
            let h = step * f64::from(1u32 << g);
            let cs = opened(cross_section(
                &fs,
                &frame.rect,
                &frame.circle,
                &frame.ring,
                frame.cut,
                across,
                z,
                h,
                lam,
                along,
            )?)?;
            let Some(found) = solve(&cs, lam)? else {
                return Ok(());
            };
            let mut n: Vec<f64> = found.iter().map(|m| m.effective_index().re).collect();
            n.sort_by(|a, b| b.total_cmp(a));
            coarse.push((h, n));
        }
        let mut unconverged = Vec::new();
        for (k, &n) in reported.iter().enumerate() {
            let label = format!("mode {} of {}", k + 1, sorted.len());
            let quantity = format!("Re n_eff of {label}");
            let mut study = crate::convergence::Study::new(&quantity);
            let mut missing = None;
            for (h, values) in &coarse {
                match values.get(k) {
                    Some(&v) => study = study.on(*h, v)?,
                    None => missing = Some(*h),
                }
            }
            let mut check = study.on(step, n)?.check(spec.tolerance)?;
            if let Some(h) = missing {
                check.verdict = crate::convergence::Verdict::NotConverged;
                check.error = None;
                check.reason = format!(
                    "not converged: the {h} um grid finds {} modes, not this one",
                    coarse.iter().find(|c| c.0 == h).map_or(0, |c| c.1.len())
                );
            }
            if !check.converged() {
                unconverged.push(format!("{label}: {}", check.reason));
            }
            run.record(&Event::GridConvergence {
                label,
                wavelength_um: lam.to_um(),
                check,
            })?;
        }
        if spec.require == Some(true) && !unconverged.is_empty() {
            return Err(Error::invalid(
                "grid convergence",
                format!(
                    "[task.convergence] requires every mode's grid to converge: {}",
                    unconverged.join("; ")
                ),
            ));
        }
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
                        along,
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
                // a parameter may move the guide under a PML
                modes_pml(&task, &swept, z)?;
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
                        along,
                    )?,
                    lam,
                )
            }
        };
        let cs = opened(cs)?;
        let Some(mut found) = solve(&cs, w)? else {
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
            let label = format!("mode {} of {}", rank + 1, found.len());
            let kind = loss(Some(k), &label, m, w)?;
            events.push(Event::SweepMode {
                point: k,
                value,
                label,
                wavelength_um: w.to_um(),
                effective_index: [n.re, n.im],
                te_fraction: m.te_fraction(),
                intensity: coarse(intensity(&fields, &cs, across, z, 2.0 * step)),
                component,
                field: coarse(field),
                cut_y_um: cut,
            });
            events.extend(kind);
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

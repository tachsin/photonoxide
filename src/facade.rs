//! A small, stable façade over the library, for bindings to other languages: the `photonoxide`
//! package for Python (`python/` in the repository) wraps it one function for one, and MATLAB
//! reaches it through that package.
//!
//! The rest of the API can change between milestones until 1.0. The façade changes only on
//! purpose: a change elsewhere is absorbed here, in the same pull request, so that the bindings
//! don't follow every change of the library. It covers the calls a Python or MATLAB user meets
//! first ([the plan](https://github.com/tachsin/photonoxide/blob/main/docs/plans/bindings.md)):
//!
//! - materials of the catalogue and their indices: [`materials`], [`refractive_index`],
//!   [`group_index`];
//! - modes: the exact slab's, [`slab_modes`], and the full-vector solver's on a cross-section,
//!   [`vector_modes`], with their fields;
//! - jobs: [`check_job`], [`run_job`] (a run's record, as the studio's), and an `"fdfd"` job's
//!   S-parameters, [`fdfd_s_parameters`];
//! - circuits from a netlist given as data, [`circuit_spectrum`];
//! - Touchstone files: [`read_touchstone`], [`write_touchstone`].
//!
//! Its conventions:
//!
//! - **Plain types.** Owned values; no lifetimes, generics, closures or trait objects in its
//!   signatures; structures, jobs and netlists as data. Results are `#[non_exhaustive]` structs,
//!   so a field can be added without breaking a caller.
//! - **Units in names,** as job files have them: `wavelength_um`, `thickness_um`,
//!   `loss_db_per_cm`. A wavelength that isn't positive and finite is an
//!   [`Error::InvalidValue`], never a silent NaN.
//! - **Arrays in C order, with their coordinates.** A 2D array of `ny` rows of `nx` values is a
//!   flat `Vec` with the point (x_i, y_j) at `j * nx + i`, NumPy's `array[j, i]`, and comes with
//!   its `x_um` and `y_um`. S-matrices at `nλ` wavelengths are `(nλ, n, n)`: S_qp at wavelength k,
//!   from port p into port q, at `(k * n + q) * n + p`.
//! - **Long calls stop.** They take a [`Stop`] (a deadline and a request, which a binding sets on
//!   Ctrl+C) and return `None` when it stopped them.
//! - **The same bits.** Each function calls the library as a Rust program would, so its numbers
//!   are those calls' to the bit; the tests here check that. [`conformance`] lists calls with
//!   their results, for a binding to check that it adds nothing to the numbers either.

mod circuit;
mod conformance;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::Complex64 as c64;
use crate::compact::touchstone::{Convention, Precision, Touchstone};
use crate::material::Material;
use crate::material::catalogue::{self, Conditions, Entry, IndexModel};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::mode::vector::{self, Boundaries, Boundary, Pml};
use crate::run::{Format, Job, Run};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

pub use crate::run::Stop;
pub use circuit::circuit_spectrum;
pub use conformance::{Case, conformance};

/// A material of the catalogue ([`crate::material::catalogue`]), with its index models.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MaterialInfo {
    /// Its id, e.g. `"si"` or `"linbo3"`: what [`refractive_index`] takes.
    pub id: String,
    /// Its name, e.g. `"Silicon"`.
    pub name: String,
    /// Its formula, e.g. `"Si"`.
    pub formula: String,
    /// Its index models, the default first.
    pub models: Vec<IndexModelInfo>,
}

/// An index model of a material, from one paper.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct IndexModelInfo {
    /// Its id, e.g. `"li-1980"`.
    pub id: String,
    /// Its name, e.g. `"Li 1980"`.
    pub name: String,
    /// Whether it is the material's default model.
    pub default: bool,
    /// The indices it gives: `["isotropic"]`, or `["ordinary", "extraordinary"]`.
    pub axes: Vec<String>,
    /// The wavelengths it is valid for at its default conditions, µm.
    pub range_um: (f64, f64),
    /// Its default temperature, K, when it has one.
    pub temperature_k: Option<f64>,
    /// Its default composition (e.g. x in AlₓGa₁₋ₓAs), when it has one.
    pub composition: Option<f64>,
    /// The accuracy its paper states.
    pub accuracy: String,
    /// Where it comes from: each paper's citation, DOI and place, e.g.
    /// `"H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), doi:10.1063/1.555624, Table 1, 293 K"`.
    pub sources: Vec<String>,
}

/// Which index of a material [`refractive_index`] and [`group_index`] give: the model (the
/// default one when `None`), the axis (needed for a material with more than one), and the
/// conditions (the model's defaults when `None`).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct IndexOptions {
    /// The model's id, e.g. `"zelmon-1997-congruent"`.
    pub model: Option<String>,
    /// `"isotropic"`, `"ordinary"` or `"extraordinary"`.
    pub axis: Option<String>,
    /// The temperature, K, for a model that has one.
    pub temperature_k: Option<f64>,
    /// The composition, for a model that has one.
    pub composition: Option<f64>,
}

/// Every material of the catalogue, with its index models.
pub fn materials() -> Vec<MaterialInfo> {
    catalogue::catalogue()
        .iter()
        .map(|entry| MaterialInfo {
            id: entry.id.clone(),
            name: entry.name.clone(),
            formula: entry.formula.clone(),
            models: entry
                .index
                .iter()
                .map(|model| model_info(entry, model))
                .collect(),
        })
        .collect()
}

fn model_info(entry: &Entry, model: &IndexModel) -> IndexModelInfo {
    IndexModelInfo {
        id: model.id.clone(),
        name: model.name.clone(),
        default: entry
            .default_model()
            .is_some_and(|default| default.id == model.id),
        axes: model.axes.iter().map(|a| axis_name(*a)).collect(),
        range_um: model.wavelength,
        temperature_k: model.temperature.as_ref().map(|p| p.default),
        composition: model.composition.as_ref().map(|p| p.default),
        accuracy: model.accuracy.clone(),
        sources: model
            .sources
            .iter()
            .map(|source| match entry.reference(&source.reference) {
                Some(r) => format!("{}, doi:{}, {}", r.citation, r.doi, source.location),
                None => format!("{}, {}", source.reference, source.location),
            })
            .collect(),
    }
}

fn axis_name(axis: catalogue::Axis) -> String {
    format!("{axis:?}").to_lowercase()
}

/// The material `id` of the catalogue as `options` choose it.
fn material(id: &str, options: &IndexOptions) -> Result<Material> {
    let Some(entry) = catalogue::entry(id) else {
        let ids: Vec<String> = catalogue::catalogue().into_iter().map(|e| e.id).collect();
        return Err(Error::invalid(
            "material",
            format!("no {id:?} in the catalogue, which has {}", ids.join(", ")),
        ));
    };
    let model = match &options.model {
        Some(name) => entry.index.iter().find(|m| m.id == *name).ok_or_else(|| {
            let ids: Vec<&str> = entry.index.iter().map(|m| m.id.as_str()).collect();
            Error::invalid(
                "index model",
                format!("{id} has no model {name:?}, only {}", ids.join(", ")),
            )
        })?,
        None => entry
            .default_model()
            .ok_or_else(|| Error::invalid("index model", format!("{id} has no index model")))?,
    };
    let axes: Vec<String> = model.axes.iter().map(|a| axis_name(*a)).collect();
    let k = match (&options.axis, axes.len()) {
        (None, 1) => 0,
        (None, _) => {
            return Err(Error::invalid(
                "axis",
                format!(
                    "{} ({id}) has the axes {}: choose one",
                    model.name,
                    axes.join(" and ")
                ),
            ));
        }
        (Some(axis), _) => axes.iter().position(|a| a == axis).ok_or_else(|| {
            Error::invalid(
                "axis",
                format!(
                    "{} ({id}) has the axes {}, not {axis:?}",
                    model.name,
                    axes.join(" and ")
                ),
            )
        })?,
    };
    let mut materials = model.materials(Conditions {
        temperature: options.temperature_k,
        composition: options.composition,
    })?;
    Ok(materials.swap_remove(k))
}

/// The refractive index n + iκ (κ ≥ 0) of the catalogue's material `id` at each vacuum
/// wavelength, µm.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a material, model or axis the catalogue doesn't have (or no axis
/// for a material with several), conditions outside the model's range, or a wavelength that
/// isn't positive and finite; [`Error::OutsideValidity`] for a wavelength outside the model's
/// range.
pub fn refractive_index(
    id: &str,
    wavelength_um: &[f64],
    options: &IndexOptions,
) -> Result<Vec<c64>> {
    let m = material(id, options)?;
    wavelength_um
        .iter()
        .map(|&w| m.refractive_index(Wavelength::um(w)?))
        .collect()
}

/// The bulk group index n − λ dn/dλ of the catalogue's material `id` at each vacuum
/// wavelength, µm.
///
/// # Errors
///
/// As [`refractive_index`].
pub fn group_index(id: &str, wavelength_um: &[f64], options: &IndexOptions) -> Result<Vec<f64>> {
    let m = material(id, options)?;
    wavelength_um
        .iter()
        .map(|&w| m.group_index(Wavelength::um(w)?))
        .collect()
}

/// A guided mode of a three-layer slab.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SlabMode {
    /// `"te"` or `"tm"`.
    pub polarization: String,
    /// Its order: 0 for the fundamental mode of its polarization.
    pub order: usize,
    /// The effective index n_eff = β/k.
    pub effective_index: f64,
    /// Where its field is given across the slab, µm, 0 at the core's bottom.
    pub x_um: Vec<f64>,
    /// Its field there: E_y for TE, H_y for TM, unnormalized and 1 (TE) or h/q̄ (TM) at x = 0
    /// ([`crate::mode::slab::SlabMode::field`]).
    pub field: Vec<f64>,
}

fn parse_polarization(name: &str) -> Result<Polarization> {
    match name.to_ascii_lowercase().as_str() {
        "te" => Ok(Polarization::Te),
        "tm" => Ok(Polarization::Tm),
        _ => Err(Error::invalid(
            "polarization",
            format!("is \"te\" or \"tm\", not {name:?}"),
        )),
    }
}

fn polarization_name(p: Polarization) -> String {
    match p {
        Polarization::Te => "te",
        Polarization::Tm => "tm",
    }
    .into()
}

/// The guided modes of one polarization (`"te"` or `"tm"`) of a core of index `core` and
/// thickness `thickness_um` between `below` and `above`, at `wavelength_um`, exactly
/// ([`crate::mode::slab`]): fundamental first, each with its field at `x_um` (0 at the core's
/// bottom). Empty when the slab guides none.
///
/// # Errors
///
/// [`Error::InvalidValue`] for indices that aren't finite and at least 1, a core whose index
/// isn't the highest, a thickness that isn't positive, an unknown polarization, or a wavelength
/// that isn't positive and finite.
pub fn slab_modes(
    below: f64,
    core: f64,
    above: f64,
    thickness_um: f64,
    polarization: &str,
    wavelength_um: f64,
    x_um: &[f64],
) -> Result<Vec<SlabMode>> {
    let p = parse_polarization(polarization)?;
    let slab = Slab::new(below, core, above, Length::um(thickness_um))?;
    Ok(slab
        .modes(p, Wavelength::um(wavelength_um)?)
        .iter()
        .map(|mode| SlabMode {
            polarization: polarization_name(p),
            order: mode.order(),
            effective_index: mode.effective_index(),
            x_um: x_um.to_vec(),
            field: x_um.iter().map(|&x| mode.field(Length::um(x)).0).collect(),
        })
        .collect())
}

/// A cross-section for [`vector_modes`]: a rectilinear grid, the relative permittivity of each
/// of its cells (isotropic), and what lies at its edges.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CrossSection {
    /// The grid's nodes along x, µm, strictly increasing: `nx + 1` of them.
    pub x_um: Vec<f64>,
    /// Along y: `ny + 1`.
    pub y_um: Vec<f64>,
    /// Each cell's relative permittivity, `(ny, nx)` in C order: the cell between `x_um[i]`,
    /// `x_um[i + 1]`, `y_um[j]` and `y_um[j + 1]` at `j * nx + i`. Loss is a positive imaginary
    /// part (e^(−iωt)).
    pub permittivity: Vec<c64>,
    /// The west (smallest x), east, south (smallest y) and north edges: `"zero"` (the field
    /// vanishes beyond), `"electric"` (a perfect electric conductor: a plane of symmetry, for a
    /// TE-like mode the vertical one through the core) or `"magnetic"` (the horizontal one).
    pub boundaries: [String; 4],
    /// Perfectly matched layers inside the west, east, south and north edges: their thickness,
    /// µm, 0 for none.
    pub pml_um: [f64; 4],
    /// The layers' stretching strength α (s = 1 + iα at the window's edge); a few units absorb
    /// well over a layer about a wavelength thick.
    pub pml_strength: f64,
}

impl CrossSection {
    /// A cross-section of `permittivity` on the grid `x_um` × `y_um`, the field zero beyond its
    /// edges, without PMLs.
    pub fn new(x_um: Vec<f64>, y_um: Vec<f64>, permittivity: Vec<c64>) -> CrossSection {
        CrossSection {
            x_um,
            y_um,
            permittivity,
            boundaries: ["zero", "zero", "zero", "zero"].map(String::from),
            pml_um: [0.0; 4],
            pml_strength: 0.0,
        }
    }

    /// The library's cross-section.
    fn build(&self) -> Result<vector::CrossSection> {
        let (nx, ny) = (
            self.x_um.len().saturating_sub(1),
            self.y_um.len().saturating_sub(1),
        );
        if self.permittivity.len() != nx * ny {
            return Err(Error::invalid(
                "cross-section",
                format!(
                    "needs one permittivity per cell, ny × nx = {ny} × {nx}, got {}",
                    self.permittivity.len()
                ),
            ));
        }
        // the library's cells are i * ny + j
        let cells = (0..nx)
            .flat_map(|i| (0..ny).map(move |j| (i, j)))
            .map(|(i, j)| vector::Permittivity::isotropic(self.permittivity[j * nx + i]))
            .collect();
        let boundary = |name: &str| match name.to_ascii_lowercase().as_str() {
            "zero" => Ok(Boundary::Zero),
            "electric" => Ok(Boundary::ElectricWall),
            "magnetic" => Ok(Boundary::MagneticWall),
            _ => Err(Error::invalid(
                "boundary",
                format!("is \"zero\", \"electric\" or \"magnetic\", not {name:?}"),
            )),
        };
        let [west, east, south, north] = &self.boundaries;
        let [pw, pe, ps, pn] = self.pml_um;
        vector::CrossSection::new(self.x_um.clone(), self.y_um.clone(), cells)?
            .with_boundaries(Boundaries {
                west: boundary(west)?,
                east: boundary(east)?,
                south: boundary(south)?,
                north: boundary(north)?,
            })?
            .with_pml(Pml {
                west: pw,
                east: pe,
                south: ps,
                north: pn,
                strength: self.pml_strength,
            })
    }
}

/// A full-vector mode of a cross-section, with its six field components at the centres of the
/// grid's cells ([`crate::mode::fields`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct VectorMode {
    /// The effective index n_eff = β/k; its imaginary part, ≥ 0 in a passive structure, is the
    /// loss along z.
    pub effective_index: c64,
    /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode (E mostly
    /// along x), near 0 for a TM-like one.
    pub te_fraction: f64,
    /// The cells' centres along x, µm: `nx` of them.
    pub x_um: Vec<f64>,
    /// Along y: `ny`.
    pub y_um: Vec<f64>,
    /// E_x, E_y and E_z, each `(ny, nx)` in C order, in units of Z₀ times H's.
    pub e: [Vec<c64>; 3],
    /// H_x, H_y and H_z, the largest transverse component of H at the grid's nodes 1.
    pub h: [Vec<c64>; 3],
}

/// The `count` modes of `cross_section` at `wavelength_um` whose effective indices are nearest
/// `near_index` (the highest index in the cross-section when `None`: the fundamental modes),
/// nearest first, by the full-vector finite-difference solver ([`crate::mode::vector`]), with
/// their fields. `None` when `stop` stopped the solve.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a grid or permittivities the solver refuses (see
/// [`crate::mode::vector::CrossSection::new`]), an unknown boundary, PMLs that don't fit, a
/// count of 0, a `near_index` that isn't finite, a wavelength that isn't positive and finite,
/// or an eigenproblem that doesn't converge.
pub fn vector_modes(
    cross_section: &CrossSection,
    wavelength_um: f64,
    count: usize,
    near_index: Option<f64>,
    stop: &Stop,
) -> Result<Option<Vec<VectorMode>>> {
    let cs = cross_section.build()?;
    let wavelength = Wavelength::um(wavelength_um)?;
    let Some(modes) = vector::modes_until(&cs, wavelength, count, near_index, || {
        stop.reason().is_some()
    })?
    else {
        return Ok(None);
    };
    modes
        .iter()
        .map(|mode| {
            let fields = mode.fields(&cs)?;
            let (nx, ny) = (fields.x().len(), fields.y().len());
            // the library's (i, j), in C order (j, i)
            let component = |f: &dyn Fn(usize, usize) -> c64| -> Vec<c64> {
                (0..ny)
                    .flat_map(|j| (0..nx).map(move |i| (i, j)))
                    .map(|(i, j)| f(i, j))
                    .collect()
            };
            Ok(VectorMode {
                effective_index: mode.effective_index(),
                te_fraction: mode.te_fraction(),
                x_um: fields.x().to_vec(),
                y_um: fields.y().to_vec(),
                e: [0, 1, 2].map(|c| component(&|i, j| fields.e(i, j)[c])),
                h: [0, 1, 2].map(|c| component(&|i, j| fields.h(i, j)[c])),
            })
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

/// A job: a file (TOML, JSON or YAML, by its extension), or a job file's text in a format.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum JobSource {
    /// The job file at this path.
    File(PathBuf),
    /// A job file's text.
    Text {
        /// The text.
        text: String,
        /// Its format: `"toml"`, `"json"` or `"yaml"`.
        format: String,
    },
}

impl JobSource {
    fn load(&self) -> Result<Job> {
        match self {
            JobSource::File(path) => Job::load(path),
            JobSource::Text { text, format } => {
                let format = match format.to_ascii_lowercase().as_str() {
                    "toml" => Format::Toml,
                    "json" => Format::Json,
                    "yaml" | "yml" => Format::Yaml,
                    _ => {
                        return Err(Error::invalid(
                            "job format",
                            format!("is \"toml\", \"json\" or \"yaml\", not {format:?}"),
                        ));
                    }
                };
                Job::parse_as(text, format)
            }
        }
    }
}

/// Checks a job without running it, as the program does before a run ([`crate::job::check`]).
///
/// # Errors
///
/// The errors the run would return for the job's settings: [`Error::Parse`] for a file that
/// isn't a job, [`Error::InvalidValue`] for a value the run would refuse, [`Error::Io`] for a
/// file that can't be read.
pub fn check_job(job: &JobSource) -> Result<()> {
    crate::job::check(&job.load()?)
}

/// A run of a job: its folder and its record.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct JobRun {
    /// The run's folder, `<runs>/<YYYYMMDD>-<HHMMSS>-<name>`, which the program replays
    /// (`photonoxide view <folder>`).
    pub dir: PathBuf,
    /// Its events in order, each a JSON object as `events.jsonl` has it, with its `type`
    /// ([`crate::job::Event`]).
    pub events: Vec<String>,
    /// Why it ended early, if it did: `"timeout"` or `"requested"`.
    pub stopped: Option<String>,
}

/// Runs `job` as `photonoxide run --headless` does, into a new folder under `runs_dir`, and
/// returns its record. It stops at `stop`'s deadline or the job's own `timeout_minutes`,
/// whichever comes first, or when `stop` is requested: the record then ends there, its last
/// event saying why.
///
/// # Errors
///
/// [`Error::Io`] if the folder can't be written, and the errors of the job's settings and its
/// run ([`crate::job::execute`]); the folder keeps what was recorded before.
pub fn run_job(job: &JobSource, runs_dir: &Path, stop: &Stop) -> Result<JobRun> {
    let job = job.load()?;
    let stop = stop.within(job.timeout());
    let mut run = Run::create(runs_dir, &job)?;
    crate::job::execute(&job, &mut run, &stop)?;
    let dir = run.dir().to_path_buf();
    drop(run);
    let events: Vec<serde_json::Value> = crate::run::replay(&dir)?;
    let stopped = events
        .last()
        .filter(|e| e["type"] == "finished")
        .and_then(|e| e["stopped"].as_str())
        .map(str::to_owned);
    Ok(JobRun {
        dir,
        events: events.iter().map(serde_json::Value::to_string).collect(),
        stopped,
    })
}

/// An `"fdfd"` job's S-parameters ([`crate::job::fdfd_s_parameters`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SParameters {
    /// The ports' names, in the job's order, e.g. `"1 (left, x = -3.5 um)"`.
    pub ports: Vec<String>,
    /// The wavelengths, µm.
    pub wavelength_um: Vec<f64>,
    /// The power-normalized S-matrices, `(nλ, n, n)`, S_qp from port p into port q, the phases
    /// referred to the ports' columns.
    pub s: Vec<c64>,
    /// Each port mode's effective index at each wavelength, `(nλ, n)`.
    pub effective_index: Vec<f64>,
    /// The grid's cell, dx and dy, µm.
    pub cell_um: (f64, f64),
    /// The slab mode's polarization the plane's indices come from: `"te"` (H along z in the
    /// plane) or `"tm"` (E along z).
    pub polarization: String,
}

/// The S-parameters of an `"fdfd"` job's device by 2D FDFD, at `wavelength_um`, or at the job's
/// own wavelengths (its sweep, or its one wavelength) when `None`. A 2D result: an estimate,
/// not a device's performance in 3D.
///
/// # Errors
///
/// The errors [`check_job`] finds, a wavelength that isn't positive and finite, and those of
/// the solve (a port inside the PML, a material without data at a wavelength).
pub fn fdfd_s_parameters(job: &JobSource, wavelength_um: Option<&[f64]>) -> Result<SParameters> {
    let s = crate::job::fdfd_s_parameters(&job.load()?, wavelength_um)?;
    Ok(SParameters {
        s: s.s.iter().flatten().flatten().copied().collect(),
        effective_index: s.effective_indices.iter().flatten().copied().collect(),
        ports: s.ports,
        wavelength_um: s.wavelengths_um,
        cell_um: s.cell_um,
        polarization: polarization_name(s.polarization),
    })
}

/// S-matrices at several wavelengths, between named ports: a circuit's spectrum, or a
/// Touchstone file's.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Spectrum {
    /// The ports' names, in the matrices' order.
    pub ports: Vec<String>,
    /// The vacuum wavelengths, µm.
    pub wavelength_um: Vec<f64>,
    /// The S-matrices, `(nλ, n, n)`, S_qp from port p into port q, in photonoxide's e^(−iωt)
    /// convention.
    pub s: Vec<c64>,
}

impl Spectrum {
    /// The S-matrices `s`, `(nλ, n, n)` in C order, at `wavelength_um` between `ports`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `s` has `nλ × n × n` values.
    pub fn new(ports: Vec<String>, wavelength_um: Vec<f64>, s: Vec<c64>) -> Result<Spectrum> {
        let n = ports.len();
        if s.len() != wavelength_um.len() * n * n {
            return Err(Error::invalid(
                "spectrum",
                format!(
                    "needs (nλ, n, n) = ({}, {n}, {n}) values, got {}",
                    wavelength_um.len(),
                    s.len()
                ),
            ));
        }
        Ok(Spectrum {
            ports,
            wavelength_um,
            s,
        })
    }

    fn of(spectrum: &crate::circuit::Spectrum) -> Spectrum {
        Spectrum {
            ports: spectrum.ports().to_vec(),
            wavelength_um: spectrum.wavelengths().iter().map(|w| w.to_um()).collect(),
            s: spectrum
                .matrices()
                .iter()
                .flat_map(|m| m.rows().into_iter().flatten())
                .collect(),
        }
    }

    fn to_library(&self) -> Result<crate::circuit::Spectrum> {
        let n = self.ports.len();
        let wavelengths = self
            .wavelength_um
            .iter()
            .map(|&w| Wavelength::um(w))
            .collect::<Result<Vec<_>>>()?;
        let matrices = (0..self.wavelength_um.len())
            .map(|k| {
                crate::circuit::SMatrix::from_rows(
                    (0..n)
                        .map(|q| self.s[(k * n + q) * n..(k * n + q + 1) * n].to_vec())
                        .collect(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        crate::circuit::Spectrum::new(self.ports.clone(), wavelengths, matrices)
    }
}

fn parse_convention(name: &str) -> Result<Convention> {
    match name.to_ascii_lowercase().as_str() {
        "physics" => Ok(Convention::Physics),
        "engineering" => Ok(Convention::Engineering),
        _ => Err(Error::invalid(
            "time convention",
            format!(
                "is \"physics\" (e^(-iwt), photonoxide's) or \"engineering\" (e^(+jwt), most RF \
                 tools'), not {name:?}"
            ),
        )),
    }
}

/// The S-parameters of the Touchstone file at `path` (Version 1 or 2.0), its values in the time
/// convention `convention` (`"physics"`, e^(−iωt), or `"engineering"`, e^(+jωt): the format
/// doesn't say, so the reader must), as photonoxide's: in the file's order (increasing
/// frequency, so decreasing wavelength), λ = c/f, ports named `o1`, `o2`, …
///
/// # Errors
///
/// [`Error::Io`] if it can't be read, [`Error::Parse`] naming the line for text that doesn't
/// follow the specification, and [`Error::InvalidValue`] for an unknown convention, parameters
/// other than S, or a frequency of zero.
pub fn read_touchstone(path: &Path, convention: &str) -> Result<Spectrum> {
    let convention = parse_convention(convention)?;
    Ok(Spectrum::of(&Touchstone::read(path)?.spectrum(convention)?))
}

/// Writes `spectrum` to the Touchstone file at `path`, as Version 2.0 in real and imaginary
/// parts, frequencies c/λ in Hz (increasing, so a spectrum at increasing wavelengths is
/// reversed), its values in the time convention `convention`, each number the shortest decimal
/// that reads back to the same `f64`, or `significant_digits` (1 to 17) of it. The ports' names
/// aren't kept: Touchstone numbers its ports.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown convention, a spectrum without wavelengths or with a
/// wavelength twice, a value that isn't finite, or `significant_digits` outside 1 to 17;
/// [`Error::Io`] if the file can't be written.
pub fn write_touchstone(
    path: &Path,
    spectrum: &Spectrum,
    convention: &str,
    significant_digits: Option<usize>,
) -> Result<()> {
    let convention = parse_convention(convention)?;
    let precision = significant_digits.map_or(Precision::RoundTrip, Precision::Significant);
    Touchstone::from_spectrum(&spectrum.to_library()?, convention)?.write_file(path, precision)
}

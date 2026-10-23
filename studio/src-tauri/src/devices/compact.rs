//! A device's compact model on its page: the ladder a design's own device has (its compact
//! fidelity among them, once fitted), the job that fits one (photonoxide::job::fit_from_device),
//! what a model's file says of itself, and the file kept next to the design it was fitted for.

use std::path::{Path, PathBuf};

use photonoxide::compact::Interpolation;
use photonoxide::device::{FitAxis, FitPlan, Fitted, Sampling};
use photonoxide::job;
use photonoxide::run::Format;
use serde::{Deserialize, Serialize};

use super::{ComputeIn, MethodInfo, SpecIn, method_info, text};

/// The models a design's own device has, cheapest first: the library device's, and the compact
/// fidelity when the design names a model's file.
///
/// # Errors
///
/// What the library refuses of the design, or of its model's file.
pub fn fidelities_of(spec: &SpecIn) -> Result<Vec<MethodInfo>, String> {
    let device = spec.spec()?.device().map_err(text)?;
    Ok(device.fidelities().iter().map(method_info).collect())
}

/// The design's ladder, its compact model included.
#[tauri::command(async)]
pub fn device_fidelities(spec: SpecIn) -> Result<Vec<MethodInfo>, String> {
    fidelities_of(&spec)
}

/// A compact model's plan as the Model tab's form sends it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FitIn {
    /// Each covered parameter: its name, range and a grid's points along it.
    pub axes: Vec<FitAxisIn>,
    /// `"grid"` or `"latin hypercube"`.
    pub sampling: String,
    /// A Latin hypercube's samples and seed.
    pub count: usize,
    pub seed: u64,
    pub held_out: usize,
    pub poles: usize,
    /// A polynomial's degree; `None` for a piecewise-linear model.
    pub degree: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FitAxisIn {
    pub parameter: String,
    pub from: f64,
    pub to: f64,
    pub points: usize,
}

impl FitIn {
    /// The library's plan.
    ///
    /// # Errors
    ///
    /// A message for a sampling that isn't one.
    pub fn plan(&self) -> Result<FitPlan, String> {
        let sampling = match self.sampling.as_str() {
            "grid" => Sampling::Grid,
            "latin hypercube" => Sampling::LatinHypercube {
                points: self.count,
                seed: self.seed,
            },
            other => return Err(format!("no sampling \"{other}\": grid or latin hypercube")),
        };
        let axes = self
            .axes
            .iter()
            .map(|a| FitAxis::new(&a.parameter, a.from, a.to, a.points))
            .collect();
        Ok(FitPlan::new(axes)
            .sampled(sampling)
            .holding_out(self.held_out)
            .with_poles(self.poles)
            .interpolated(match self.degree {
                Some(degree) => Interpolation::Polynomial { degree },
                None => Interpolation::PiecewiseLinear,
            }))
    }
}

/// The text of the job that fits a compact model of `spec`'s device at its fidelity as `fit`
/// says, each sample computed as `compute` says, named `name`, with an opening comment saying
/// what it fits and how to run it.
///
/// # Errors
///
/// What the library refuses of the plan or the design.
pub fn fit_text_of(
    spec: &SpecIn,
    compute: &ComputeIn,
    fit: &FitIn,
    name: &str,
) -> Result<String, String> {
    let s = spec.spec()?;
    let mut c = compute.compute();
    c.fdtd = None;
    let plan = fit.plan()?;
    let j = job::fit_from_device(name, &s, &c, &plan).map_err(text)?;
    let body = j.to_text(Format::Yaml).map_err(text)?;
    Ok(format!(
        "# A compact model of the {} over {}, fitted to its {} model's runs.\n#   photonoxide run \
         jobs/{name}.yaml [--headless] [--workers 4]; the model is the run's compact.json\n{body}",
        spec.kind,
        fit.axes
            .iter()
            .map(|a| a.parameter.as_str())
            .collect::<Vec<_>>()
            .join(" and "),
        spec.fidelity
    ))
}

/// The job that fits a compact model of `spec`, as its text.
#[tauri::command(async)]
pub fn device_fit_job(
    spec: SpecIn,
    compute: ComputeIn,
    fit: FitIn,
    name: String,
) -> Result<String, String> {
    fit_text_of(&spec, &compute, &fit, &name)
}

/// What a compact model's file says of itself, its model left out.
#[derive(Debug, PartialEq, Serialize)]
pub struct CompactInfo {
    pub path: String,
    pub kind: String,
    pub source: String,
    pub grid: String,
    pub parameters: Vec<String>,
    pub units: Vec<String>,
    pub ranges: Vec<(f64, f64)>,
    pub fixed: Vec<(String, f64)>,
    pub band_um: (f64, f64),
    pub sampling: String,
    pub samples: usize,
    pub held_out: usize,
    pub fitted_error: f64,
    pub held_out_error: f64,
    pub stability: f64,
    pub passivity: f64,
    pub poles: usize,
    pub interpolation: String,
}

/// What `fitted` says of itself, read from `path`.
pub fn info_of(path: &Path, fitted: &Fitted) -> CompactInfo {
    CompactInfo {
        path: path.display().to_string(),
        kind: fitted.kind.clone(),
        source: fitted.source.clone(),
        grid: fitted.grid.clone(),
        parameters: fitted.parameters.clone(),
        units: fitted.units.clone(),
        ranges: fitted.ranges.clone(),
        fixed: fitted.fixed.clone(),
        band_um: fitted.band_um,
        sampling: fitted.sampling.clone(),
        samples: fitted.samples,
        held_out: fitted.held_out,
        fitted_error: fitted.fitted_error,
        held_out_error: fitted.held_out_error,
        stability: fitted.stability,
        passivity: fitted.passivity,
        poles: fitted.model.basis_poles(),
        interpolation: match fitted.model.interpolation() {
            Interpolation::Polynomial { degree } => format!("polynomial of degree {degree}"),
            Interpolation::PiecewiseLinear => "piecewise linear".into(),
        },
    }
}

/// What the compact model in the file at `path` says of itself.
#[tauri::command(async)]
pub fn compact_info(path: String) -> Result<CompactInfo, String> {
    let p = PathBuf::from(&path);
    let f = Fitted::read(&p).map_err(text)?;
    Ok(info_of(&p, &f))
}

/// The design `spec`, its compact model's file copied next to the design `jobs/<name>.yaml` in
/// `dir` (as `<name>.compact.json`) and named by that relative path; the design as it is when it
/// names none, or already names that file.
///
/// # Errors
///
/// A message if the file can't be copied.
pub fn kept_beside(spec: &SpecIn, dir: &Path, name: &str) -> Result<SpecIn, String> {
    let Some(from) = &spec.compact else {
        return Ok(spec.clone());
    };
    let file = format!("{name}.compact.json");
    let to = dir.join(&file);
    let from = if Path::new(from).is_absolute() {
        PathBuf::from(from)
    } else {
        dir.join(from)
    };
    let same = from
        .canonicalize()
        .ok()
        .zip(to.canonicalize().ok())
        .is_some_and(|(a, b)| a == b);
    if !same {
        std::fs::copy(&from, &to).map_err(|e| format!("{}: {e}", from.display()))?;
    }
    Ok(SpecIn {
        compact: Some(file),
        ..spec.clone()
    })
}

/// The design read from a file in `dir`, its compact model's file found from there: an absolute
/// path, which the page's commands take wherever they run.
pub fn found_from(mut spec: SpecIn, dir: &Path) -> SpecIn {
    if let Some(c) = &spec.compact
        && Path::new(c).is_relative()
    {
        spec.compact = Some(dir.join(c).display().to_string());
    }
    spec
}

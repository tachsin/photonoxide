//! A device's compact models fitted by a `"fit"` job: what the tests and the validation cases
//! measure of them.

use std::path::PathBuf;
use std::sync::OnceLock;

use super::{Compute, Event, execute, fit_from_device};
use crate::circuit::{Fidelity, SMatrix};
use crate::compact::Interpolation;
use crate::device::{self, FitAxis, FitPlan, Fitted, Sampling, Spec, Values};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};
use crate::{Error, Result};

/// A folder under the system's temporary one, removed when dropped.
pub(crate) struct TempDir(pub(crate) PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A new, empty folder for `tag`.
pub(crate) fn temp(tag: &str) -> TempDir {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("photonoxide-fit-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    TempDir(dir)
}

/// `kind` at `fidelity` on 220 nm SOI, with `parameters`.
pub(crate) fn spec(kind: &str, fidelity: Fidelity, parameters: &[(&str, &str)]) -> Spec {
    let mut s = Spec::new(kind);
    s.fidelity = fidelity;
    s.parameters = parameters
        .iter()
        .map(|&(n, e)| (n.to_owned(), e.to_owned()))
        .collect();
    s
}

/// The model a `"fit"` job of `spec` by `plan` over `band` (µm: from, to, points) makes, run in
/// memory, with the job.
///
/// # Errors
///
/// The job's.
pub(crate) fn fitted(
    spec: &Spec,
    plan: &FitPlan,
    band: (f64, f64, usize),
) -> Result<(Job, Fitted)> {
    let compute = Compute {
        band: Some(band),
        ..Compute::model()
    };
    let job = fit_from_device("fit", spec, &compute, plan)?;
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None))?;
    for line in run.lines() {
        if let Ok(Event::CompactFit { fitted }) = serde_json::from_str::<Event>(line) {
            return Ok((job, *fitted));
        }
    }
    Err(Error::invalid("fit", "the run recorded no model"))
}

/// The all-pass ring's closed form over its gap from 0.15 to 0.25 µm, at 1.54 to 1.56 µm (41
/// wavelengths), `points` samples on a grid, 6 held out, 12 poles, a polynomial of `degree`.
pub(crate) fn ring_over_gap(points: usize, degree: usize) -> Result<Fitted> {
    let plan = FitPlan::new(vec![FitAxis::new("gap", 0.15, 0.25, points)])
        .holding_out(6)
        .with_poles(12)
        .interpolated(Interpolation::Polynomial { degree });
    Ok(fitted(
        &spec("all-pass ring", Fidelity::Analytic, &[]),
        &plan,
        (1.54, 1.56, 41),
    )?
    .1)
}

/// The largest |ΔS| between `fitted` used as a design's compact fidelity and the design's own
/// closed form, at `gaps` and 201 wavelengths over the band: the model where it was neither
/// fitted nor held out, through the device that offers it.
pub(crate) fn ring_against_closed_form(fitted: &Fitted, gaps: &[f64]) -> Result<f64> {
    let dir = temp("ring-closed-form");
    let path = dir.0.join("ring.compact.json");
    fitted.write(&path)?;
    let mut compact = spec("all-pass ring", Fidelity::Compact, &[]);
    compact.compact = Some(path.display().to_string());
    let (device, _) = compact.build()?;
    let analytic = Spec::new("all-pass ring").device()?;
    let mut worst = 0.0f64;
    for &g in gaps {
        let values = Values::new(device.as_ref(), &[("gap", g)])?;
        let a = device.component(&values, Fidelity::Compact)?;
        let b = analytic.component(&values, Fidelity::Analytic)?;
        for k in 0..=200 {
            let w = Wavelength::um(1.54 + 0.02 * f64::from(k) / 200.0)?;
            worst = worst.max(largest(&a.s_matrix(w)?, &b.s_matrix(w)?));
        }
    }
    Ok(worst)
}

/// The largest |ΔS_qp| between two S-matrices.
pub(crate) fn largest(a: &SMatrix, b: &SMatrix) -> f64 {
    let n = a.size();
    let mut worst = 0.0f64;
    for q in 0..n {
        for p in 0..n {
            worst = worst.max((a[(q, p)] - b[(q, p)]).norm());
        }
    }
    worst
}

/// The exact dS/d(gap) of the compact fidelity through [`device::derivative`] against central
/// differences of 1e-6 µm, at a few gaps and wavelengths: the largest difference relative to the
/// largest derivative.
pub(crate) fn ring_derivative_error(fitted: &Fitted) -> Result<f64> {
    let dir = temp("ring-derivative");
    let path = dir.0.join("ring.compact.json");
    fitted.write(&path)?;
    let mut s = spec("all-pass ring", Fidelity::Compact, &[]);
    s.compact = Some(path.display().to_string());
    let (dev, _) = s.build()?;
    let (mut worst, mut scale) = (0.0f64, 0.0f64);
    let h = 1e-6;
    for g in [0.171, 0.2033, 0.237] {
        let at = |x: f64| {
            let mut t = s.clone();
            t.parameters = vec![("gap".into(), format!("{x:?}"))];
            t
        };
        let (_, design) = at(g).build()?;
        for k in [3, 47, 101, 150] {
            let w = Wavelength::um(1.54 + 0.02 * f64::from(k) / 200.0)?;
            let exact = device::derivative(dev.as_ref(), &design, Fidelity::Compact, w, "gap")?
                .ok_or_else(|| Error::invalid("fit", "no exact derivative"))?;
            let sm = |x: f64| -> Result<SMatrix> {
                let (d, v) = at(x).build()?;
                device::s_matrix(d.as_ref(), v.values(), Fidelity::Compact, w)
            };
            let (up, down) = (sm(g + h)?, sm(g - h)?);
            let n = exact.size();
            for q in 0..n {
                for p in 0..n {
                    let fd = (up[(q, p)] - down[(q, p)]) / (2.0 * h);
                    worst = worst.max((fd - exact[(q, p)]).norm());
                    scale = scale.max(exact[(q, p)].norm());
                }
            }
        }
    }
    Ok(worst / scale)
}

/// A device's 2D or 3D model fitted over one of its parameters: 4 samples on a grid, 2 held
/// out, 8 poles, a polynomial of degree 2.
pub(crate) struct DeviceFit {
    pub(crate) kind: &'static str,
    pub(crate) fidelity: Fidelity,
    pub(crate) parameter: &'static str,
    pub(crate) range: (f64, f64),
    /// The band, µm: from, to and how many wavelengths.
    pub(crate) band: (f64, f64, usize),
    /// The cells in the chip's plane, nm, when not the device's own.
    pub(crate) step_nm: Option<f64>,
}

/// The fit of each device of the library with a 2D or 3D fidelity, at its cheapest such
/// fidelity, over a parameter its model solves for, on the device's own grid (the Y-branch's
/// 2D FDFD on 40 nm cells): bands of 11 wavelengths, a ring's and an MZI's of 41 over 2 or 4 nm
/// to resolve their resonances and fringes. The Y-branch MZI's 3D FDFD is more than the direct
/// solver holds here, so it has none.
pub(crate) const DEVICE_FITS: [DeviceFit; 10] = [
    DeviceFit {
        kind: "waveguide",
        fidelity: Fidelity::ThreeD,
        parameter: "width",
        range: (0.49, 0.51),
        band: (1.545, 1.555, 11),
        step_nm: None,
    },
    DeviceFit {
        kind: "bend",
        fidelity: Fidelity::TwoD,
        parameter: "radius",
        range: (9.98, 10.02),
        band: (1.545, 1.555, 11),
        step_nm: None,
    },
    DeviceFit {
        kind: "directional coupler",
        fidelity: Fidelity::ThreeD,
        parameter: "gap",
        range: (0.19, 0.21),
        band: (1.545, 1.555, 11),
        step_nm: None,
    },
    DeviceFit {
        kind: "all-pass ring",
        fidelity: Fidelity::ThreeD,
        parameter: "gap",
        range: (0.19, 0.21),
        band: (1.548, 1.552, 41),
        step_nm: None,
    },
    DeviceFit {
        kind: "add-drop ring",
        fidelity: Fidelity::ThreeD,
        parameter: "gap",
        range: (0.19, 0.21),
        band: (1.548, 1.552, 41),
        step_nm: None,
    },
    DeviceFit {
        kind: "rib waveguide",
        fidelity: Fidelity::TwoD,
        parameter: "width",
        range: (0.49, 0.51),
        band: (1.545, 1.555, 11),
        step_nm: None,
    },
    DeviceFit {
        kind: "MMI 1x2",
        fidelity: Fidelity::TwoD,
        parameter: "width",
        range: (5.88, 6.12),
        band: (1.545, 1.555, 11),
        step_nm: None,
    },
    DeviceFit {
        kind: "MMI 2x2",
        fidelity: Fidelity::TwoD,
        parameter: "width",
        range: (5.88, 6.12),
        band: (1.545, 1.555, 11),
        step_nm: None,
    },
    DeviceFit {
        kind: "Y-branch",
        fidelity: Fidelity::TwoD,
        parameter: "length",
        range: (1.9, 2.1),
        band: (1.545, 1.555, 11),
        step_nm: Some(40.0),
    },
    DeviceFit {
        kind: "MZI",
        fidelity: Fidelity::ThreeD,
        parameter: "phase",
        range: (-10.0, 10.0),
        band: (1.549, 1.551, 41),
        step_nm: None,
    },
];

/// `fit`'s model.
///
/// # Errors
///
/// The job's.
pub(crate) fn device_fit(fit: &DeviceFit) -> Result<Fitted> {
    let plan = FitPlan::new(vec![FitAxis::new(
        fit.parameter,
        fit.range.0,
        fit.range.1,
        4,
    )])
    .sampled(Sampling::Grid)
    .holding_out(2)
    .with_poles(8)
    .interpolated(Interpolation::Polynomial { degree: 2 });
    let compute = Compute {
        band: Some(fit.band),
        step_nm: fit.step_nm,
        ..Compute::model()
    };
    let job = fit_from_device("fit", &spec(fit.kind, fit.fidelity, &[]), &compute, &plan)?;
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None))?;
    for line in run.lines() {
        if let Ok(Event::CompactFit { fitted }) = serde_json::from_str::<Event>(line) {
            return Ok(*fitted);
        }
    }
    Err(Error::invalid("fit", "the run recorded no model"))
}

// ---- the validation cases ----

/// The ring's model over its gap, 7 samples of degree 4, fitted once.
fn ring() -> &'static Option<Fitted> {
    static FIT: OnceLock<Option<Fitted>> = OnceLock::new();
    FIT.get_or_init(|| ring_over_gap(7, 4).ok())
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

fn failed() -> Outcome {
    zero(f64::NAN, 0.0)
}

fn ring_closed_form() -> Outcome {
    let Some(f) = ring() else { return failed() };
    ring_against_closed_form(f, &[0.1537, 0.18, 0.2111, 0.2463])
        .map_or_else(|_| failed(), |e| zero(e, RING_TOLERANCE))
}

fn ring_held_out() -> Outcome {
    ring()
        .as_ref()
        .map_or_else(failed, |f| zero(f.held_out_error, RING_TOLERANCE))
}

fn ring_convergence() -> Outcome {
    let errors: Option<Vec<f64>> = [(3, 1), (5, 2), (7, 4)]
        .iter()
        .map(|&(n, d)| ring_over_gap(n, d).ok().map(|f| f.held_out_error))
        .collect();
    let Some(e) = errors else { return failed() };
    // each step's error over the one before: the worst of the two, below 1 when it falls
    let worst = (e[1] / e[0]).max(e[2] / e[1]);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 0.1,
        error: worst,
    }
}

fn ring_derivatives() -> Outcome {
    let Some(f) = ring() else { return failed() };
    ring_derivative_error(f).map_or_else(|_| failed(), |e| zero(e, 1e-6))
}

fn ring_passive() -> Outcome {
    let Some(f) = ring() else { return failed() };
    Outcome {
        measured: f.passivity,
        expected: 1.0,
        tolerance: RING_TOLERANCE,
        error: (f.passivity - 1.0).max(0.0),
    }
}

/// The lines of a fit's record on 1 and on 20 threads that differ, the finish's time aside, and
/// whether its model reads back from its file to the same model.
fn threads() -> Outcome {
    let plan = FitPlan::new(vec![
        FitAxis::new("gap", 0.15, 0.25, 0),
        FitAxis::new("radius", 9.99, 10.01, 0),
    ])
    .sampled(Sampling::LatinHypercube {
        points: 12,
        seed: 3,
    })
    .holding_out(4)
    .with_poles(12);
    let s = spec("all-pass ring", Fidelity::Analytic, &[]);
    let lines = |threads: usize| -> Option<Vec<String>> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()?;
        pool.install(|| {
            let compute = Compute {
                band: Some((1.54, 1.56, 31)),
                ..Compute::model()
            };
            let job = fit_from_device("fit", &s, &compute, &plan).ok()?;
            let mut run = Run::in_memory();
            execute(&job, &mut run, &Stop::new(None)).ok()?;
            Some(
                run.lines()
                    .iter()
                    .filter(|l| !l.contains("\"finished\""))
                    .cloned()
                    .collect(),
            )
        })
    };
    let (Some(a), Some(b)) = (lines(1), lines(20)) else {
        return failed();
    };
    let mut differ = a.iter().zip(&b).filter(|(x, y)| x != y).count() + a.len().abs_diff(b.len());
    let reread = a
        .iter()
        .find_map(|l| match serde_json::from_str::<Event>(l) {
            Ok(Event::CompactFit { fitted }) => Some(*fitted),
            _ => None,
        });
    match reread {
        Some(f) => {
            let back = f.to_json().ok().and_then(|t| Fitted::from_json(&t).ok());
            differ += usize::from(back.as_ref() != Some(&f));
        }
        None => differ += 1,
    }
    zero(differ as f64, 0.0)
}

/// The tolerance of the ring's cases: a largest |ΔS| of 1e-6.
const RING_TOLERANCE: f64 = 1e-6;

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    let mut out = vec![
        Case {
            id: "device/compact-ring-closed-form",
            title: r"Devices' compact models: an all-pass ring's closed form fitted over its gap (0.15 to 0.25 µm, a grid of 7, 12 poles, a polynomial of degree 4, 41 wavelengths from 1.54 to 1.56 µm) as a job, then used as the design's compact fidelity: largest $\lvert\Delta S\rvert$ against the closed form at 4 gaps neither fitted nor held out, 201 wavelengths (shown)",
            tier: Tier::Analytic,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue), doi:10.1002/lpor.201100017, Eq. 1, through the ring device's analytic model; the model is P. Triverio et al.'s form, IEEE Trans. Adv. Packag. 32, 205 (2009), doi:10.1109/TADVP.2008.2007913, solved as C. K. Sanathanan and J. Koerner do, IEEE Trans. Autom. Control 8, 56 (1963), doi:10.1109/TAC.1963.1105517; 3.3e-7 here",
            run: ring_closed_form,
        },
        Case {
            id: "device/compact-ring-held-out",
            title: r"The same model's error at its 6 held-out samples (a Latin hypercube in the middle of its strata), the error its compact fidelity states (shown)",
            tier: Tier::Analytic,
            source: "the closed form at gaps the fit didn't see; 2.9e-7 here, against 3.3e-7 between them (above)",
            run: ring_held_out,
        },
        Case {
            id: "device/compact-ring-convergence",
            title: r"The ring's model over its gap from 3, 5 and 7 samples, degree 1, 2 and 4: each step's held-out error over the one before, the larger of the two (shown; below 1 when the error falls)",
            tier: Tier::Analytic,
            source: "the polynomial's error falls as its degree rises with the samples to determine it, as compact's convergence on the shifting ring does (compact/param-ring-shift); 9.3e-3, 3.7e-5 and 2.9e-7 here",
            run: ring_convergence,
        },
        Case {
            id: "device/compact-ring-derivatives",
            title: r"The ring's compact fidelity's exact $dS/d(\text{gap})$, through the device's derivative, against central differences of 1e-6 µm at 3 gaps and 4 wavelengths: largest difference relative to the largest derivative (shown)",
            tier: Tier::Analytic,
            source: "the model's own derivatives (compact/param-ring-derivatives) through the device, which maps its gap to the model's one to one; the circuit adjoint takes them in place of differences; 1.7e-9 here",
            run: ring_derivatives,
        },
        Case {
            id: "device/compact-ring-passive",
            title: "The ring's model's largest singular value over its band (201 wavelengths) and its range (21 gaps) (shown; Eq. 1's is at most 1)",
            tier: Tier::Analytic,
            source: "passivity: a ring without gain gives out at most the power it takes in; the fit's error allowed",
            run: ring_passive,
        },
        Case {
            id: "device/compact-threads",
            title: "A ring's model over its gap and radius (a Latin hypercube of 12, 4 held out) fitted as a job on 1 and on 20 threads: the lines of the records that differ, plus one if the model doesn't read back from its file to itself (count shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md): the samples are computed on their own and recorded in order, the fit is made from the record on one thread, and its file holds every number to the bit",
            run: threads,
        },
    ];
    out.extend(device_cases());
    out
}

/// Fit `k` of [`DEVICE_FITS`]: its held-out error, at most `tolerance`.
fn held_out(k: usize, tolerance: f64) -> Outcome {
    device_fit(&DEVICE_FITS[k]).map_or_else(|_| failed(), |f| zero(f.held_out_error, tolerance))
}

macro_rules! device_case {
    ($name:ident, $k:expr, $tolerance:expr) => {
        fn $name() -> Outcome {
            held_out($k, $tolerance)
        }
    };
}

device_case!(held_out_waveguide, 0, 1e-3);
device_case!(held_out_bend, 1, 1e-3);
device_case!(held_out_coupler, 2, 1e-3);
device_case!(held_out_all_pass, 3, 1e-3);
device_case!(held_out_add_drop, 4, 1e-3);
device_case!(held_out_rib, 5, 1e-3);
device_case!(held_out_mmi_1x2, 6, 5e-3);
device_case!(held_out_mmi_2x2, 7, 5e-3);
device_case!(held_out_y_branch, 8, 5e-2);
device_case!(held_out_mzi, 9, 1e-3);

/// What every device fit's case says, with what it measured here.
macro_rules! device_source {
    ($here:literal) => {
        concat!(
            "the device's own model at that fidelity, at 2 samples held out of the fit (a Latin hypercube in the middle of its strata): the error the compact fidelity states against its source; ",
            $here,
            " here. A 2D number is an estimate of the device, never its performance"
        )
    };
}

/// The held-out error of each device with a 2D or 3D fidelity.
fn device_cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/compact-waveguide-3d",
            title: r"The strip waveguide's 3D model (full-vector, Hadley, on its fitted grid of about 10 nm) fitted over its width, 0.49 to 0.51 µm (a grid of 4, 8 poles, degree 2, 11 wavelengths from 1.545 to 1.555 µm): largest $\lvert\Delta S\rvert$ at 2 held-out widths (shown)",
            tier: Tier::Analytic,
            source: device_source!("1.1e-4"),
            run: held_out_waveguide,
        },
        Case {
            id: "device/compact-bend-2d",
            title: r"The bend's 2D model (an exact bent slab by the effective index method, no grid) fitted over its radius, 9.98 to 10.02 µm, as above: largest $\lvert\Delta S\rvert$ at 2 held-out radii (shown)",
            tier: Tier::Analytic,
            source: device_source!("5.4e-5"),
            run: held_out_bend,
        },
        Case {
            id: "device/compact-coupler-3d",
            title: r"The directional coupler's 3D model (supermodes, full-vector, on its fitted grid of about 10 nm) fitted over its gap, 0.19 to 0.21 µm, as above: largest $\lvert\Delta S\rvert$ at 2 held-out gaps (shown)",
            tier: Tier::Analytic,
            source: device_source!("2.5e-4"),
            run: held_out_coupler,
        },
        Case {
            id: "device/compact-all-pass-3d",
            title: r"The all-pass ring's 3D model (its guide's and coupler's modes, full-vector, about 10 nm) fitted over its gap, 0.19 to 0.21 µm, at 41 wavelengths from 1.548 to 1.552 µm (a resonance resolved): largest $\lvert\Delta S\rvert$ at 2 held-out gaps (shown)",
            tier: Tier::Analytic,
            source: device_source!("2.0e-4"),
            run: held_out_all_pass,
        },
        Case {
            id: "device/compact-add-drop-3d",
            title: r"The add-drop ring's 3D model fitted over its gap, as the all-pass ring's: largest $\lvert\Delta S\rvert$ at 2 held-out gaps (shown)",
            tier: Tier::Analytic,
            source: device_source!("1.8e-4"),
            run: held_out_add_drop,
        },
        Case {
            id: "device/compact-rib-2d",
            title: r"The rib waveguide's 2D model (the effective index method, exact slabs, no grid) fitted over its width, 0.49 to 0.51 µm, as the strip's: largest $\lvert\Delta S\rvert$ at 2 held-out widths (shown)",
            tier: Tier::Analytic,
            source: device_source!("3.0e-5"),
            run: held_out_rib,
        },
        Case {
            id: "device/compact-mmi-1x2-2d",
            title: r"The 1 × 2 MMI's 2D model (guided modes by the effective index method, exact slab modes, no grid) fitted over its width, 5.88 to 6.12 µm, as the strip's: largest $\lvert\Delta S\rvert$ at 2 held-out widths (shown)",
            tier: Tier::Analytic,
            source: device_source!("1.5e-3"),
            run: held_out_mmi_1x2,
        },
        Case {
            id: "device/compact-mmi-2x2-2d",
            title: r"The 2 × 2 MMI's 2D model fitted over its width, as the 1 × 2's: largest $\lvert\Delta S\rvert$ at 2 held-out widths (shown)",
            tier: Tier::Analytic,
            source: device_source!("1.5e-3"),
            run: held_out_mmi_2x2,
        },
        Case {
            id: "device/compact-y-branch-2d",
            title: r"The Y-branch's 2D model (2D FDFD by the effective index method on 40 nm cells) fitted over its junction's length, 1.9 to 2.1 µm, as the strip's: largest $\lvert\Delta S\rvert$ at 2 held-out lengths (shown)",
            tier: Tier::Analytic,
            source: device_source!("1.5e-2"),
            run: held_out_y_branch,
        },
        Case {
            id: "device/compact-mzi-3d",
            title: r"The MZI's 3D model (its couplers' supermodes and its arms' mode, full-vector, about 10 nm) fitted over its phase, −10 to 10°, at 41 wavelengths from 1.549 to 1.551 µm: largest $\lvert\Delta S\rvert$ at 2 held-out phases (shown); the Y-branch MZI's 3D FDFD is more than the direct solver holds here",
            tier: Tier::Analytic,
            source: device_source!("1.7e-5"),
            run: held_out_mzi,
        },
    ]
}

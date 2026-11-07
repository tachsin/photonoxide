//! What the validation report checks an `"fdfd"` job's `[task.interpolate]` on (#600, #595): S
//! over the sweep's wavelengths interpolated from a few solves, every entry on one set of
//! wavelengths, against the same job solved at every point of its sweep.

use std::sync::OnceLock;

use super::{Event, execute};
use crate::run::{Job, Run, Stop};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid fdfd job";

/// The sweep's points: the dense sweep the interpolant is checked against.
pub(super) const POINTS: usize = 201;

/// The tolerance asked of the interpolant, relative to the largest |S_qp| (about absolute).
pub(super) const TOLERANCE: f64 = 1e-6;

/// A 500 nm silicon strip of the 220 nm SOI stack widening to 800 nm at x = 0, by 2D FDFD on
/// 40 nm cells (the effective index method), a port on either side of the step, swept from 1.5
/// to 1.6 µm at [`POINTS`] wavelengths; with `[task.interpolate]` at [`TOLERANCE`] when
/// `interpolate`.
pub(super) fn step_job(interpolate: bool) -> String {
    let mut text = format!(
        r#"
name = "strip-step-fdfd"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-2.0, 2.0]
y_um = [-1.6, 1.6]
step_nm = 40.0

[[task.rect]]
layer = "Si"
center_um = [-1.5, 0.0]
size_um = [3.0, 0.5]

[[task.rect]]
layer = "Si"
center_um = [1.5, 0.0]
size_um = [3.0, 0.8]

[[task.port]]
x_um = -0.6
side = "left"

[[task.port]]
x_um = 0.6
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = {POINTS}
"#
    );
    if interpolate {
        text.push_str(&format!(
            "\n[task.interpolate]\ntolerance = {TOLERANCE:e}\n"
        ));
    }
    text
}

/// What the study measured: the interpolant's estimate, the solves it took, its true error (the
/// largest |r_qp − S_qp| over the sweep's points and every entry against the job solved at each
/// point), and its largest |r_qp − r_pq|.
#[derive(Clone, Debug)]
pub(super) struct Study {
    pub estimate: f64,
    #[allow(dead_code)] // the study test prints it
    pub solves: usize,
    #[allow(dead_code)] // the study test prints it
    pub converged: bool,
    pub truth: f64,
    #[allow(dead_code)] // the study test prints it
    pub reciprocity: f64,
}

/// The step's study, made once.
pub(super) fn study() -> &'static Study {
    static STUDY: OnceLock<Study> = OnceLock::new();
    STUDY.get_or_init(|| {
        let job = Job::parse(&step_job(true)).expect(OK);
        super::check(&job).expect(OK);
        let mut run = Run::in_memory();
        execute(&job, &mut run, &Stop::new(None)).expect(OK);
        let events: Vec<Event> = run
            .lines()
            .iter()
            .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
            .collect();
        let interpolated: Vec<(f64, Vec<Vec<[f64; 2]>>)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SParameters {
                    wavelength_um, s, ..
                } => Some((*wavelength_um, s.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(interpolated.len(), POINTS, "an S at every point");
        let (estimate, measure) = events
            .iter()
            .find_map(|e| match e {
                Event::SolveError {
                    point: None,
                    measure,
                    error,
                    ..
                } if measure.starts_with("S over") => Some((*error, measure.clone())),
                _ => None,
            })
            .expect("the interpolant's estimate");
        let solves = measure
            .split("from ")
            .nth(1)
            .and_then(|s| s.split(' ').next())
            .and_then(|n| n.parse().ok())
            .expect("the solves");
        let points: Vec<f64> = interpolated.iter().map(|p| p.0).collect();
        let direct =
            super::fdfd::fdfd_s_parameters(&Job::parse(&step_job(false)).expect(OK), Some(&points))
                .expect("the direct sweep");
        let (mut truth, mut reciprocity) = (0.0f64, 0.0f64);
        for ((_, r), s) in interpolated.iter().zip(&direct.s) {
            let n = r.len();
            for q in 0..n {
                for p in 0..n {
                    let ours = num_complex::Complex64::new(r[q][p][0], r[q][p][1]);
                    let theirs = num_complex::Complex64::new(r[p][q][0], r[p][q][1]);
                    truth = truth.max((ours - s[q][p]).norm());
                    reciprocity = reciprocity.max((ours - theirs).norm());
                }
            }
        }
        Study {
            estimate,
            solves,
            converged: !measure.contains("not within"),
            truth,
            reciprocity,
        }
    })
}

/// The ratio of the true error to the estimate, rounded up to two digits: a ratio whose last
/// digits are the platform's.
fn ratio() -> f64 {
    let s = study();
    let v = s.truth / s.estimate;
    if v <= 0.0 || !v.is_finite() {
        return v;
    }
    let p = 10f64.powf(v.log10().floor() - 1.0);
    (v / p).ceil() * p
}

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![Case {
        id: "job/fdfd-interpolate-step",
        title: r"S over wavelength from a few solves through an `fdfd` job's `[task.interpolate]`: a 500 nm strip of the 220 nm SOI stack widening to 800 nm, 2D FDFD on 40 nm cells (the effective index method), a port either side of the step, its $2 \times 2$ S from 1.5 to 1.6 µm on one set of solves to $10^{-6}$: the ratio of its true error at the sweep's 201 wavelengths to its estimate (shown)",
        tier: Tier::Analytic,
        source: r"the same job solved at each of the 201 wavelengths; set-valued AAA (Lietaert et al. 2021, doi:10.1093/imanum/draa098, Section 2.2) with the sweep's estimate from two interleaved halves (`sweep/estimate-bounds-error`), which bounds the error where the fit to all solves is at least 1.5 times as accurate as either half's",
        run: || {
            let r = ratio();
            Outcome {
                measured: r,
                expected: 0.0,
                tolerance: 1.0,
                error: r,
            }
        },
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers the method page and the rule quote: run with `--ignored --nocapture` in
    /// release.
    #[test]
    #[ignore = "the step solved at 201 wavelengths: in release, prints"]
    fn the_interpolated_step_study() {
        let s = study();
        eprintln!(
            "fdfd step: {} solves (converged {}), estimate {:.3e}, true error {:.3e} (ratio {:.3e}), reciprocity {:.2e}",
            s.solves,
            s.converged,
            s.estimate,
            s.truth,
            s.truth / s.estimate,
            s.reciprocity
        );
    }

    #[test]
    fn an_interpolation_needs_a_sweep_and_a_tolerance_above_the_floor() {
        let without = step_job(false).replace(
            "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 201\n",
            "",
        ) + "\n[task.interpolate]\ntolerance = 1e-6\n";
        assert!(super::super::check(&Job::parse(&without).expect(OK)).is_err());
        let low = step_job(false) + "\n[task.interpolate]\ntolerance = 1e-15\n";
        assert!(super::super::check(&Job::parse(&low).expect(OK)).is_err());
        let few = step_job(false) + "\n[task.interpolate]\ntolerance = 1e-6\nmax_solves = 3\n";
        assert!(super::super::check(&Job::parse(&few).expect(OK)).is_err());
    }
}

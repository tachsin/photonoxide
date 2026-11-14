//! What the validation report checks a modes job's interpolated wavelength sweep on (#600,
//! #595): two slabs far apart, whose modes cross exactly, their effective indices over the
//! sweep interpolated from a few solves with the modes followed by their fields' overlap
//! (`follow = "overlap"`), against the same job solved and followed at every point.

use crate::portable;
use std::sync::OnceLock;

use super::{Event, execute};
use crate::run::{Job, Run, Stop};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid modes job";

/// The sweep's points: the dense sweep the interpolant is checked against.
pub(super) const POINTS: usize = 61;

/// The tolerance asked of the interpolant, relative to the largest |n_eff|.
pub(super) const TOLERANCE: f64 = 1e-6;

/// The sweep's ends, µm: the slabs' TE modes cross between them (at about 1.65 µm).
pub(super) const FROM: f64 = 1.5;
pub(super) const TO: f64 = 1.8;

/// Two slabs in air across the job's whole window, 4 µm apart: 120 nm of index 3.0 and 600 nm of
/// 2.0 above it, constant indices, their two highest modes on a 20 nm grid, swept from
/// [`FROM`] to [`TO`] at `points` wavelengths, the modes followed by overlap when `follow`,
/// with `[task.interpolate]` at [`TOLERANCE`] when `interpolate`.
pub(super) fn slabs_job(follow: bool, interpolate: bool, points: usize) -> String {
    let mut text = format!(
        r#"
name = "crossing-slabs"
timeout_minutes = 30

[task]
kind = "modes"
wavelength_um = 1.65
layer = "thin"
propagation = "x"
y_um = [-0.5, 0.5]
z_um = [-1.0, 5.72]
step_nm = 20.0
modes = 2

[task.stack]
substrate = 1.0
cladding = 1.0

[[task.stack.layer]]
name = "thin"
thickness_um = 0.12
material = 3.0
background = 1.0

[[task.stack.layer]]
name = "gap"
thickness_um = 4.0
material = 1.0
background = 1.0

[[task.stack.layer]]
name = "thick"
thickness_um = 0.6
material = 2.0
background = 1.0

[[task.rect]]
layer = "thin"
center_um = [0.0, 0.0]
size_um = [10.0, 2.0]

[[task.rect]]
layer = "thick"
center_um = [0.0, 0.0]
size_um = [10.0, 2.0]

[task.sweep]
parameter = "wavelength"
from = {FROM}
to = {TO}
points = {points}
"#
    );
    if follow {
        text.push_str("follow = \"overlap\"\n");
    }
    if interpolate {
        text.push_str(&format!(
            "\n[task.interpolate]\ntolerance = {TOLERANCE:e}\n"
        ));
    }
    text
}

/// The events of `text`'s run, in memory.
pub(super) fn events_of(text: &str) -> Vec<Event> {
    let job = Job::parse(text).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    run.lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .collect()
}

/// Each sweep point's (wavelength, Re n_eff of each mode in its number) of a run's events.
pub(super) fn points_of(events: &[Event]) -> Vec<(f64, Vec<f64>)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::SweepPoint {
                wavelength_um,
                effective_indices,
                ..
            } => Some((
                *wavelength_um,
                effective_indices.iter().map(|n| n[0]).collect(),
            )),
            _ => None,
        })
        .collect()
}

/// What a study measured: the interpolant's estimate and solves, and its true error (the
/// largest |n_eff| difference over the sweep's points and modes against the job solved at each
/// point, numbered the same way).
#[derive(Clone, Debug)]
pub(super) struct Study {
    pub estimate: f64,
    #[allow(dead_code)] // the study test prints it
    pub solves: usize,
    #[allow(dead_code)] // the study test prints it
    pub converged: bool,
    pub truth: f64,
}

/// The slabs' study, with the modes followed by overlap (`follow`) or numbered by index.
pub(super) fn study_of(follow: bool) -> Study {
    let events = events_of(&slabs_job(follow, true, POINTS));
    let interpolated = points_of(&events);
    assert_eq!(interpolated.len(), POINTS, "a point at every wavelength");
    let (estimate, measure) = events
        .iter()
        .find_map(|e| match e {
            Event::SolveError {
                point: None,
                measure,
                error,
                ..
            } if measure.contains("interpolated from") => Some((*error, measure.clone())),
            _ => None,
        })
        .expect("the interpolant's estimate");
    let solves = measure
        .split("interpolated from ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .and_then(|n| n.parse().ok())
        .expect("the solves");
    let direct = points_of(&events_of(&slabs_job(follow, false, POINTS)));
    assert_eq!(direct.len(), POINTS, "a direct solve at every wavelength");
    let mut truth = 0.0f64;
    for ((l, ours), (m, theirs)) in interpolated.iter().zip(&direct) {
        assert!((l - m).abs() < 1e-12, "the same wavelengths");
        for (a, b) in ours.iter().zip(theirs) {
            truth = truth.max((a - b).abs());
        }
    }
    Study {
        estimate,
        solves,
        converged: !measure.contains("not within"),
        truth,
    }
}

/// The followed study, made once.
pub(super) fn study() -> &'static Study {
    static STUDY: OnceLock<Study> = OnceLock::new();
    STUDY.get_or_init(|| study_of(true))
}

/// The ratio of the true error to the estimate, rounded up to two digits: a ratio whose last
/// digits are the platform's.
fn ratio() -> f64 {
    let s = study();
    let v = s.truth / s.estimate;
    if v <= 0.0 || !v.is_finite() {
        return v;
    }
    let p = portable::powf(10f64, portable::log10(v).floor() - 1.0);
    (v / p).ceil() * p
}

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![Case {
        id: "job/modes-interpolate-crossing",
        title: r"Effective indices over wavelength from a few solves through a `modes` job's `[task.interpolate]`, the modes followed by their fields' overlap (`follow = overlap`): two slabs in air 4 µm apart across the job's window, 120 nm of index 3.0 and 600 nm of 2.0, whose TE modes cross at about 1.65 µm, their two highest modes on a 20 nm grid from 1.5 to 1.8 µm to $10^{-6}$: the ratio of the interpolant's true error at the sweep's 61 wavelengths to its estimate (shown)",
        tier: Tier::Analytic,
        source: r"the same job solved and followed at each of the 61 wavelengths; the slabs' exact TE modes (mode::slab) cross at one wavelength in the range, and so far apart they don't couple, so each followed mode is one slab's and smooth; set-valued AAA (Lietaert et al. 2021, doi:10.1093/imanum/draa098, Section 2.2) with the sweep's estimate from two interleaved halves (`sweep/estimate-bounds-error`)",
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
    use crate::mode::Polarization;
    use crate::mode::slab::Slab;
    use crate::units::{Length, Wavelength};

    /// The two slabs' exact TE indices at `l` µm: (the thin one's, the thick one's).
    fn exact(l: f64) -> (f64, f64) {
        let w = Wavelength::um(l).expect("a wavelength");
        let thin = Slab::new(1.0, 3.0, 1.0, Length::um(0.12)).expect("a slab");
        let thick = Slab::new(1.0, 2.0, 1.0, Length::um(0.6)).expect("a slab");
        (
            thin.modes(Polarization::Te, w)[0].effective_index(),
            thick.modes(Polarization::Te, w)[0].effective_index(),
        )
    }

    /// The slabs' exact modes cross once in the sweep, and the job's followed modes cross with
    /// them: mode 1 is the thin slab's at both ends, above the other at the start and below it
    /// at the end; numbered by index, mode 1 stays the higher.
    #[test]
    fn the_followed_modes_cross_where_the_slabs_do() {
        let (a, b) = exact(FROM);
        let (c, d) = exact(TO);
        assert!(a > b && c < d, "the exact modes cross: {a} {b}, {c} {d}");
        let followed = points_of(&events_of(&slabs_job(true, false, 7)));
        let (first, last) = (&followed[0].1, &followed[6].1);
        assert!(
            first[0] > first[1] && last[0] < last[1],
            "{first:?} {last:?}"
        );
        let sorted = points_of(&events_of(&slabs_job(false, false, 7)));
        assert!(sorted.iter().all(|(_, n)| n[0] >= n[1]));
    }

    /// What a followed or interpolated sweep refuses before solving, and that neither is farmed.
    #[test]
    fn a_followed_sweep_is_checked_and_not_farmed() {
        let refused = |text: String, words: &str| {
            let job = Job::parse(&text).expect("a job's text");
            let e = super::super::check(&job).expect_err("refused").to_string();
            assert!(e.contains(words), "{e}");
        };
        let plain = slabs_job(false, false, 7);
        refused(
            plain.replace("parameter = \"wavelength\"", "parameter = \"width\"")
                + "follow = \"overlap\"\n",
            "over a wavelength sweep",
        );
        refused(plain.clone() + "follow = \"rank\"\n", "unknown follow");
        refused(
            plain.replace("parameter = \"wavelength\"", "parameter = \"width\"")
                + "\n[task.interpolate]\ntolerance = 1e-6\n",
            "not one of \"width\"",
        );
        refused(
            plain.replace("modes = 2", "modes_above = 1.2")
                + "\n[task.interpolate]\ntolerance = 1e-6\n",
            "not modes_above",
        );
        refused(
            plain.clone() + "\n[task.interpolate]\ntolerance = 1e-16\n",
            "at least 1e-13",
        );
        let no_sweep = plain
            .split("[task.sweep]")
            .next()
            .expect("the task")
            .to_owned();
        refused(
            no_sweep + "\n[task.interpolate]\ntolerance = 1e-6\n",
            "give a [task.sweep]",
        );
        let farms = |text: &str| super::super::farms(&Job::parse(text).expect("a job"));
        assert!(farms(&plain));
        assert!(!farms(&slabs_job(true, false, 7)));
        assert!(!farms(&slabs_job(false, true, 7)));
    }

    /// The numbers the method page and the rule quote: run with `--ignored --nocapture` in
    /// release.
    #[test]
    #[ignore = "the slabs solved at 61 wavelengths, twice: in release, prints"]
    fn the_interpolated_crossing_study() {
        for follow in [true, false] {
            let s = study_of(follow);
            println!(
                "follow {follow}: {} solves, converged {}, estimate {:e}, true error {:e}, \
                 ratio {}",
                s.solves,
                s.converged,
                s.estimate,
                s.truth,
                s.truth / s.estimate
            );
        }
    }
}

//! What every example shares: comparing what photonoxide computes with what a paper prints.

// each example compiles this module on its own and uses only part of it
#![allow(dead_code)]

use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use photonoxide::fdtd::Simulation;

/// The comparisons of one example. Each prints a row; [`Checks::finish`] fails the example
/// when any of them is outside its tolerance.
#[derive(Default)]
pub struct Checks {
    failed: usize,
    total: usize,
}

impl Checks {
    /// Compares `got` with the paper's `printed` value, allowing `tolerance` (absolute).
    pub fn compare(&mut self, what: &str, got: f64, printed: f64, tolerance: f64) {
        let ok = (got - printed).abs() <= tolerance;
        self.row(
            ok,
            format!("  {what:<34} {got:>12.6}   paper {printed:<10} ± {tolerance:e}"),
        );
    }

    /// Compares `got` with `value` from another `source` than a printed number (a method the
    /// paper describes, computed here), allowing `tolerance` (absolute).
    pub fn against(&mut self, what: &str, got: f64, source: &str, value: f64, tolerance: f64) {
        let ok = (got - value).abs() <= tolerance;
        self.row(
            ok,
            format!("  {what:<34} {got:>12.6}   {source} {value:<10.6} ± {tolerance:e}"),
        );
    }

    /// Checks `got` is at most the paper's bound `printed` (a value it states as "below").
    pub fn at_most(&mut self, what: &str, got: f64, printed: f64) {
        self.row(
            got <= printed,
            format!("  {what:<34} {got:>12.6}   paper below {printed}"),
        );
    }

    /// Compares a count, e.g. of guided modes, with the paper's.
    pub fn count(&mut self, what: &str, got: usize, printed: usize) {
        self.row(
            got == printed,
            format!("  {what:<34} {got:>12}   paper {printed}"),
        );
    }

    /// Checks the grid a reported number was computed on ([`photonoxide::convergence`]): its
    /// grid study's estimate of its error within the study's tolerance. Fails on an unconverged
    /// grid and on a number from one grid, which has no check.
    pub fn converged(&mut self, what: &str, c: &photonoxide::convergence::Convergence) {
        let estimate = c
            .error
            .map_or_else(|| "none".to_owned(), |e| format!("{e:.1e}"));
        let order = c
            .order
            .map_or_else(String::new, |p| format!(", order {p:.2}"));
        self.row(
            c.converged(),
            format!(
                "  {what:<34} {:>12.6}   on {} grids, error {estimate}{order} ≤ {:e}",
                c.value,
                c.values.len(),
                c.tolerance.unwrap_or(f64::NAN)
            ),
        );
    }

    fn row(&mut self, ok: bool, text: String) {
        self.total += 1;
        if !ok {
            self.failed += 1;
        }
        println!("{text:<80} {}", if ok { "ok" } else { "FAILED" });
    }

    /// Prints the summary; a failure if any comparison failed.
    pub fn finish(self) -> ExitCode {
        if self.failed == 0 {
            println!("all {} within tolerance of the paper", self.total);
            ExitCode::SUCCESS
        } else {
            println!("{} of {} outside tolerance", self.failed, self.total);
            ExitCode::FAILURE
        }
    }
}

/// How many steps a long run takes between looks at the clock.
pub const STRETCH: usize = 50;

/// A long run's progress: a line on stderr every few minutes (`PHOTONOXIDE_PROGRESS` seconds,
/// 300 by default; 0 for none) with the steps done of the total, the share, the time taken, the
/// time left and when it should end (UTC), and the rate in cell-updates/s. It goes to stderr, so
/// an example's output, which CI compares, doesn't change, and it changes nothing in the run.
pub struct Progress {
    label: String,
    cells: f64,
    first: usize,
    total: usize,
    exact: bool,
    started: Instant,
    last: Instant,
    every: Option<Duration>,
}

impl Progress {
    /// A run named `label` of `cells` cells, at step `first` now, to step `total`: all of them
    /// if `exact`, else at most (it may stop sooner, when its fields have decayed).
    pub fn new(label: &str, cells: usize, first: usize, total: usize, exact: bool) -> Progress {
        let seconds = std::env::var("PHOTONOXIDE_PROGRESS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(300);
        let every = (seconds > 0).then(|| Duration::from_secs(seconds));
        if every.is_some() {
            eprintln!(
                "{label}: {} steps to go{} on {:.1} million cells; its progress every {seconds} s",
                group(total.saturating_sub(first)),
                if exact { "" } else { " at most" },
                cells as f64 / 1e6
            );
        }
        let now = Instant::now();
        Progress {
            label: label.to_string(),
            cells: cells as f64,
            first,
            total,
            exact,
            started: now,
            last: now,
            every,
        }
    }

    /// The run is at step `steps`: a line if it's time for one.
    pub fn report(&mut self, steps: usize) {
        let Some(every) = self.every else { return };
        if self.last.elapsed() < every {
            return;
        }
        self.last = Instant::now();
        let done = steps.saturating_sub(self.first);
        let elapsed = self.started.elapsed().as_secs_f64();
        let eta = self.total.saturating_sub(steps) as f64 * elapsed / done.max(1) as f64;
        let end = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64())
            + eta;
        let most = if self.exact { "" } else { "at most " };
        eprintln!(
            "{}: step {} of {most}{} ({:.1} %), {} in, {most}{} to go (about {} UTC), {:.0} \
             million cell-updates/s",
            self.label,
            group(steps),
            group(self.total),
            100.0 * steps as f64 / self.total.max(1) as f64,
            duration(elapsed),
            duration(eta),
            utc(end),
            self.cells * done as f64 / elapsed.max(1e-9) / 1e6
        );
    }
}

/// Runs `sim` until its time reaches `time` (µm/c), as `Simulation::run_until` does (the same
/// bits), reporting its progress as `label`.
pub fn run_until(sim: &mut Simulation, time: f64, label: &str) {
    let total = steps_to(sim, time);
    let mut progress = Progress::new(label, sim.grid().cells(), sim.steps(), total, true);
    sim.run_until_reporting(time, STRETCH, &mut |s| progress.report(s.steps()));
}

/// Runs `sim` until its fields have decayed, as `Simulation::run_until_decayed` does (the same
/// blocks, checks and bits), reporting its progress as `label`.
pub fn run_until_decayed(
    sim: &mut Simulation,
    probes: &[usize],
    fraction: f64,
    interval: f64,
    limit: f64,
    label: &str,
) -> photonoxide::Result<bool> {
    let total = steps_to(sim, limit);
    let mut progress = Progress::new(label, sim.grid().cells(), sim.steps(), total, false);
    sim.run_until_decayed_reporting(probes, fraction, interval, limit, STRETCH, &mut |s| {
        progress.report(s.steps())
    })
}

/// The step at which `sim`'s time reaches `time`.
fn steps_to(sim: &Simulation, time: f64) -> usize {
    sim.steps().max((time / sim.dt()).ceil() as usize)
}

/// A count with spaces between the thousands: 150 650.
fn group(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Seconds as hours and minutes, or minutes and seconds.
fn duration(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    if s >= 3600 {
        format!("{} h {} min", s / 3600, (s % 3600) / 60)
    } else {
        format!("{} min {} s", s / 60, s % 60)
    }
}

/// Seconds since 1970 as a UTC date and time to the minute (H. Hinnant's days-to-civil).
fn utc(seconds: f64) -> String {
    let s = seconds.max(0.0) as i64;
    let (days, rest) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year}-{month:02}-{day:02} {:02}:{:02}",
        rest / 3600,
        (rest % 3600) / 60
    )
}

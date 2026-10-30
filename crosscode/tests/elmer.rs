//! Elmer against photonoxide's static solver: docs/validation-elmer.md, written here where
//! Elmer is installed (never in CI), in release, from the repository's root:
//!
//! ```sh
//! PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer
//! ```
//!
//! It skips, saying so, where Elmer isn't found. Its runs are kept in folders under
//! `crosscode/target/tmp/elmer/`, each with Elmer's output in `elmer.log`. Where
//! `PHOTONOXIDE_ELMER_ONLY` names some of the problems (`line`, `heater`, `heaters`,
//! `transient`, `joule`, `tfln`, comma-separated), only those run, and the report is printed,
//! not written.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use photonoxide_crosscode::cpw::{self, Cpw};
use photonoxide_crosscode::elmer::{self, Elmer};
use photonoxide_crosscode::heater::{self, Heater};
use photonoxide_crosscode::heaters::{self, Heaters};
use photonoxide_crosscode::joule::Joule;
use photonoxide_crosscode::report::{duration, machine, percent, signed_percent};
use photonoxide_crosscode::tfln::Tfln;
use photonoxide_crosscode::transient::{self, Transient};

/// The meshes' smallest cells, µm, each half the last, growing by 1.1 to the largest.
const LINE_SMALLEST: [f64; 5] = [2.0, 1.0, 0.5, 0.25, 0.125];
const LINE_LARGEST: f64 = 10.0;
const HEATER_SMALLEST: [f64; 5] = [0.2, 0.1, 0.05, 0.025, 0.0125];
const HEATER_LARGEST: f64 = 2.0;
const GROWTH: f64 = 1.1;
/// photonoxide's own converged grid studies: their smallest cells, growth and largest.
const LINE_CONVERGED: [f64; 3] = [0.01, 0.005, 0.0025];
const LINE_CONVERGED_GROWTH: f64 = 1.025;
const LINE_CONVERGED_LARGEST: f64 = 2.0;
const HEATER_CONVERGED: [f64; 3] = [0.004, 0.002, 0.001];
const HEATER_CONVERGED_GROWTH: f64 = 1.05;
const HEATER_CONVERGED_LARGEST: f64 = 1.0;

/// The four later problems' meshes: their smallest cells, µm, each half the last, growing by
/// [`GROWTH`] to [`SILICON_LARGEST`]; photonoxide's converged ones as the heater's.
const SILICON_SMALLEST: [f64; 5] = [0.08, 0.04, 0.02, 0.01, 0.005];
const SILICON_LARGEST: f64 = 2.0;

/// The transient's step, s, and the steps the step study takes on one mesh.
const STEP_S: f64 = 0.025e-6;
const STEP_STUDY_S: [f64; 4] = [0.1e-6, 0.05e-6, 0.025e-6, 0.0125e-6];
/// The mesh the step study takes, its smallest cell (µm).
const STEP_STUDY_MESH: f64 = 0.02;

/// Elmer extrapolated against photonoxide converged, relative.
const INDEX_TOLERANCE: f64 = 1e-3;
const IMPEDANCE_TOLERANCE: f64 = 5e-3;
const RISE_TOLERANCE: f64 = 5e-3;
const CURRENT_TOLERANCE: f64 = 1e-3;
const CAPACITANCE_TOLERANCE: f64 = 1e-3;
const FIELD_TOLERANCE: f64 = 5e-3;

const REPORT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/validation-elmer.md");

/// Richardson's extrapolation of the last three values on meshes each half as fine: the limit
/// and the observed order (the finest and NaN where the differences don't shrink).
fn extrapolated(v: &[f64]) -> (f64, f64) {
    let [a, b, c] = [v[v.len() - 3], v[v.len() - 2], v[v.len() - 1]];
    let ratio = (a - b) / (b - c);
    if ratio.is_nan() || ratio <= 1.0 {
        return (c, f64::NAN);
    }
    let p = ratio.log2();
    (c + (c - b) / (2f64.powf(p) - 1.0), p)
}

fn order(p: f64) -> String {
    if p.is_finite() {
        format!("order {p:.2}")
    } else {
        "no order: the finest shown".to_owned()
    }
}

/// One mesh: both codes' values and times.
struct Row<T> {
    smallest: f64,
    cells: usize,
    elmer: T,
    elmer_seconds: f64,
    ours: T,
    ours_seconds: f64,
}

/// Both codes' times over every mesh, s: Elmer's and photonoxide's.
fn totals<T>(rows: &[Row<T>]) -> (f64, f64) {
    rows.iter().fold((0.0, 0.0), |(e, o), r| {
        (e + r.elmer_seconds, o + r.ours_seconds)
    })
}

/// Each timed solve runs this many times, the fastest kept: one run's time on a busy computer
/// is noise at these few milliseconds.
const REPEATS: usize = 3;

/// `f`'s result and its fastest time of [`REPEATS`] runs, s.
fn timed<T>(f: impl Fn() -> T) -> (T, f64) {
    let mut best = f64::INFINITY;
    let mut value = None;
    for _ in 0..REPEATS {
        let t = Instant::now();
        value = Some(f());
        best = best.min(t.elapsed().as_secs_f64());
    }
    (value.expect("at least one run"), best)
}

/// Elmer's run of `case` in `folder`, [`REPEATS`] times: the fastest of its solvers' own
/// times, from its log.
fn elmer_fastest(program: &Elmer, case: &elmer::Case, folder: &Path) -> f64 {
    (0..REPEATS)
        .map(|_| {
            program.run(case, folder).expect("Elmer's run");
            elmer::solver_seconds(folder).expect("Elmer's log")
        })
        .fold(f64::INFINITY, f64::min)
}

/// Elmer's capacitance of the whole line (twice the half's), F/m.
fn capacitance(program: &Elmer, case: &elmer::Case, folder: &Path) -> f64 {
    program.run(case, folder).expect("Elmer's run");
    2.0 * elmer::capacitance(folder).expect("Elmer's capacitance")[0][0]
}

/// The study's steps, each reported with its share done, the time taken and the time left.
struct Progress {
    started: Instant,
    steps: usize,
    done: usize,
}

impl Progress {
    fn next(&mut self, what: &str) {
        let elapsed = self.started.elapsed().as_secs_f64();
        let k = self.done;
        let eta = if k > 0 {
            elapsed / k as f64 * (self.steps - k) as f64
        } else {
            0.0
        };
        eprintln!(
            "[elmer] step {}/{} ({:.0}%), {elapsed:.0} s elapsed, about {eta:.0} s left: {what}",
            k + 1,
            self.steps,
            100.0 * k as f64 / self.steps as f64
        );
        self.done += 1;
    }
}

/// A figure a pair compares: its name with its unit, the factor from SI to that unit, its
/// decimals there, and the tolerance of Elmer extrapolated against photonoxide converged.
struct Quantity {
    name: &'static str,
    scale: f64,
    decimals: usize,
    tolerance: f64,
}

/// One of the later pairs, studied: each mesh's values, both codes', and photonoxide's
/// converged ones.
struct Pair {
    /// The name in the summary tables.
    case: &'static str,
    /// Its section's heading.
    heading: &'static str,
    /// Its section's description.
    about: String,
    quantities: Vec<Quantity>,
    rows: Vec<Row<Vec<f64>>>,
    converged: Vec<(Vec<f64>, f64)>,
    /// More of its section, after its tables: the transient's step study.
    more: String,
}

/// A mesh: its smallest cell, growth and largest cell, µm.
type Grading = (f64, f64, f64);

/// Elmer's run of a mesh: its values, its fastest time (s) and the mesh's cells.
type ElmerRun = (Vec<f64>, f64, usize);

/// photonoxide's values and its fastest time, s.
type Timed = (Vec<f64>, f64);

/// A value extrapolated, and the observed order.
type Limit = (f64, f64);

/// Both codes on each of [`SILICON_SMALLEST`] (growth [`GROWTH`] to [`SILICON_LARGEST`]) and
/// photonoxide on [`HEATER_CONVERGED`]: `elmer` gives Elmer's values, its fastest time and its
/// cells, `ours` photonoxide's values and its fastest time.
fn study(
    progress: &mut Progress,
    label: &str,
    elmer: &dyn Fn(Grading, &str) -> ElmerRun,
    ours: &dyn Fn(Grading) -> Timed,
) -> (Vec<Row<Vec<f64>>>, Vec<Timed>) {
    let mut rows = Vec::new();
    for &smallest in &SILICON_SMALLEST {
        progress.next(&format!("{label}, {smallest} µm"));
        let mesh = (smallest, GROWTH, SILICON_LARGEST);
        let (e, elmer_seconds, cells) = elmer(mesh, &format!("{label}-{smallest}"));
        let (o, ours_seconds) = ours(mesh);
        eprintln!(
            "  Elmer {e:?} ({elmer_seconds:.2} s); photonoxide {o:?} ({ours_seconds:.2} s), {cells} cells"
        );
        rows.push(Row {
            smallest,
            cells,
            elmer: e,
            elmer_seconds,
            ours: o,
            ours_seconds,
        });
    }
    progress.next(&format!("{label}, photonoxide converged"));
    let converged = HEATER_CONVERGED
        .iter()
        .map(|&smallest| {
            let c = ours((smallest, HEATER_CONVERGED_GROWTH, HEATER_CONVERGED_LARGEST));
            eprintln!("  photonoxide, {smallest} µm: {:?} ({:.2} s)", c.0, c.1);
            c
        })
        .collect();
    (rows, converged)
}

/// A value in a quantity's unit and decimals.
fn shown(q: &Quantity, v: f64) -> String {
    format!("{:.*}", q.decimals, v * q.scale)
}

impl Pair {
    /// Elmer's and photonoxide's extrapolations, photonoxide's converged one, and the
    /// reference, of quantity `k`.
    fn summary(&self, k: usize) -> (Limit, Limit, Limit, f64) {
        let el = extrapolated(&self.rows.iter().map(|r| r.elmer[k]).collect::<Vec<_>>());
        let px = extrapolated(&self.rows.iter().map(|r| r.ours[k]).collect::<Vec<_>>());
        let cv = extrapolated(&self.converged.iter().map(|c| c.0[k]).collect::<Vec<_>>());
        let reference = self.converged[self.converged.len() - 1].0[k];
        (el, px, cv, reference)
    }

    /// Its rows of the results' table.
    fn results(&self) -> String {
        let last = &self.rows[self.rows.len() - 1];
        let mut out = String::new();
        for (k, q) in self.quantities.iter().enumerate() {
            let (el, _, _, reference) = self.summary(k);
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                self.case,
                q.name,
                shown(q, reference),
                shown(q, el.0),
                percent(el.0, reference),
                shown(q, last.elmer[k]),
                percent(last.elmer[k], reference),
                shown(q, last.ours[k]),
                percent(last.ours[k], reference)
            );
        }
        out
    }

    /// Its row of the times' table.
    fn times(&self) -> String {
        let last = &self.rows[self.rows.len() - 1];
        let (e, o) = totals(&self.rows);
        format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            self.case,
            duration(e),
            duration(o),
            duration(last.elmer_seconds),
            duration(last.ours_seconds),
            duration(self.converged[self.converged.len() - 1].1)
        )
    }

    /// Its section.
    fn section(&self) -> String {
        let mut out = format!("## {}\n\n{}\n\n", self.heading, self.about);
        let names: Vec<&str> = self.quantities.iter().map(|q| q.name).collect();
        let _ = writeln!(
            out,
            "| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: {} | Error | photonoxide, the same lines | Error |\n|---:|---:|---:|---|---|---|---|",
            names.join("; ")
        );
        for row in &self.rows {
            let values = |v: &[f64]| -> (String, String) {
                let mut shown_values = Vec::new();
                let mut errors = Vec::new();
                for (k, q) in self.quantities.iter().enumerate() {
                    let reference = self.converged[self.converged.len() - 1].0[k];
                    shown_values.push(shown(q, v[k]));
                    errors.push(percent(v[k], reference));
                }
                (shown_values.join(", "), errors.join(", "))
            };
            let (e, ee) = values(&row.elmer);
            let (o, oe) = values(&row.ours);
            let _ = writeln!(
                out,
                "| {} µm | {} | {}, {} | {e} | {ee} | {o} | {oe} |",
                row.smallest,
                row.cells,
                duration(row.elmer_seconds),
                duration(row.ours_seconds)
            );
        }
        let _ = writeln!(
            out,
            "\n| Quantity | photonoxide, converged (the reference) | Elmer, extrapolated | Error | photonoxide on the same lines, extrapolated | Error |\n|---|---:|---:|---:|---:|---:|"
        );
        for (k, q) in self.quantities.iter().enumerate() {
            let (el, px, cv, reference) = self.summary(k);
            let _ = writeln!(
                out,
                "| {} | {} (extrapolated {}, {}) | {} ({}) | {} | {} ({}) | {} |",
                q.name,
                shown(q, reference),
                shown(q, cv.0),
                order(cv.1),
                shown(q, el.0),
                order(el.1),
                percent(el.0, reference),
                shown(q, px.0),
                order(px.1),
                percent(px.0, reference)
            );
        }
        let _ = writeln!(
            out,
            "\nCells from the smallest at the silicon's and the electrodes' edges, growing by {GROWTH} to {SILICON_LARGEST} µm; photonoxide's converged values graded from {} µm by {HEATER_CONVERGED_GROWTH} to {HEATER_CONVERGED_LARGEST} µm. Elmer's extrapolation and photonoxide's on the same lines take the three finest meshes.\n",
            HEATER_CONVERGED[HEATER_CONVERGED.len() - 1]
        );
        out.push_str(&self.more);
        out
    }

    /// Each quantity's error, Elmer extrapolated against photonoxide converged, and tolerance.
    fn checks(&self) -> Vec<(String, f64, f64)> {
        self.quantities
            .iter()
            .enumerate()
            .map(|(k, q)| {
                let (el, _, _, reference) = self.summary(k);
                (
                    format!("{}: {}", self.case, q.name),
                    (el.0 - reference) / reference,
                    q.tolerance,
                )
            })
            .collect()
    }
}

/// Whether problem `name` runs: all of them, unless `PHOTONOXIDE_ELMER_ONLY` names some.
fn runs(name: &str) -> bool {
    match std::env::var("PHOTONOXIDE_ELMER_ONLY") {
        Ok(only) if !only.trim().is_empty() => only.split(',').any(|n| n.trim() == name),
        _ => true,
    }
}

/// A mesh's lines.
fn mesh_of((smallest, growth, largest): Grading) -> heaters::Mesh {
    heaters::Mesh {
        smallest,
        growth,
        largest,
    }
}

/// The last values of `variable` SaveScalars saved at nodes in `run`.
fn last_saved(run: &Path, variable: &str) -> std::collections::HashMap<usize, f64> {
    let mut saved = elmer::saved(run, variable).expect("Elmer's values");
    saved.pop().expect("a row of values").values
}

fn heaters_pair(program: &Elmer, folder: &Path, progress: &mut Progress) -> Pair {
    let case = Heaters::jacques();
    let axes = |g: Grading| case.axes(&mesh_of(g)).expect("a valid mesh");
    let elmer = |g: Grading, name: &str| {
        let (x, y) = axes(g);
        let (c, watches) = case.elmer(&x, &y).expect("the case");
        let run = folder.join(name);
        let seconds = elmer_fastest(program, &c, &run);
        let values = last_saved(&run, "temperature");
        let v = watches
            .iter()
            .map(|w| w.of(&values).expect("the guide's nodes"))
            .collect();
        (v, seconds, c.mesh.cells())
    };
    let ours = |g: Grading| {
        let (x, y) = axes(g);
        let (v, s) = timed(|| case.photonoxide(&x, &y).expect("the heat solve"));
        (v.to_vec(), s)
    };
    let (rows, converged) = study(progress, "heaters", &elmer, &ours);
    Pair {
        case: "the two heaters",
        heading: "Two heaters side by side: HeatSolver's rise at the neighbour's guide",
        about: format!(
            "Jacques et al.'s N++ silicon heater (Opt. Express 27, 10456 (2019), Section 2.2 and Table 3) beside a second one, the thermal crosstalk of their Fig. 9: each a rib 500 nm wide and 220 nm high on a 90 nm slab 2.1 µm wide, a strip of N++ silicon 1 µm wide beyond each side of the slab; their centres {} µm apart, so that 5 µm separate their facing N++ strips. 2 µm of buried oxide on silicon, the oxide's top 3.32 µm up with 10 W/(m²·K) to air at 0, a window 18 µm high whose bottom, in the silicon, is held at 0, its sides insulated 20 µm beyond each guide. The first heater dissipates the paper's P_π, 22.8 mW over 320 µm ({:.2} W/m), in its N++ strips; the second is off. Table 3's conductivities: oxide 1.38, substrate 148, rib 90, slab 55, N++ 25 W/(m·K). Elmer's HeatSolver with the power as a heat source and a heat transfer coefficient at the top, the guides' mean temperatures from its nodes (its bilinear elements integrated over each rib); photonoxide's heat solver, each rib's mean over its cells. The heated guide's own rise is shown beside its neighbour's.",
            case.pitch,
            heaters::WATTS_PER_METRE
        ),
        quantities: vec![
            Quantity {
                name: "neighbour's rise, K",
                scale: 1.0,
                decimals: 5,
                tolerance: RISE_TOLERANCE,
            },
            Quantity {
                name: "heated guide's rise, K",
                scale: 1.0,
                decimals: 4,
                tolerance: RISE_TOLERANCE,
            },
        ],
        rows: rows
            .into_iter()
            .map(|r| Row {
                elmer: vec![r.elmer[1], r.elmer[0]],
                ours: vec![r.ours[1], r.ours[0]],
                ..r
            })
            .collect(),
        converged: converged
            .into_iter()
            .map(|(v, s)| (vec![v[1], v[0]], s))
            .collect(),
        more: String::new(),
    }
}

/// A step, s, in µs as written: 0.0125, not 0.012499999999999999.
fn us(seconds: f64) -> f64 {
    (seconds * 1e10).round() / 1e4
}

/// The transient's figures by Elmer on `g` with steps of `step_s` in `run`: the rises at
/// [`transient::TIMES_US`] and the 1/e time, its fastest time over `repeats` runs, its cells.
fn transient_elmer(
    program: &Elmer,
    case: &Transient,
    g: Grading,
    step_s: f64,
    run: &Path,
    repeats: usize,
) -> (Vec<f64>, f64, usize) {
    let (x, y) = case.axes(&mesh_of(g)).expect("a valid mesh");
    let steps = (transient::END_US * 1e-6 / step_s).round() as usize;
    let (steady, stepped, rib) = case.elmer(&x, &y, step_s, steps).expect("the case");
    let at_rest = run.join("steady");
    program.run(&steady, &at_rest).expect("Elmer's run");
    let rise = rib
        .of(&last_saved(&at_rest, "temperature"))
        .expect("the rib's nodes");
    let seconds = (0..repeats)
        .map(|_| {
            program.run(&stepped, run).expect("Elmer's run");
            elmer::solver_seconds(run).expect("Elmer's log")
        })
        .fold(f64::INFINITY, f64::min);
    let mut history = vec![(0.0, 0.0)];
    for row in elmer::saved(run, "temperature").expect("Elmer's values") {
        history.push((row.time, rib.of(&row.values).expect("the rib's nodes")));
    }
    assert_eq!(history.len(), steps + 1, "Elmer saved every step");
    (
        transient::figures(&history, rise),
        seconds,
        stepped.mesh.cells(),
    )
}

/// The same by photonoxide: its figures, and its transient's fastest time over `repeats` runs
/// (its steady solve, for the 1/e time, not in it).
fn transient_ours(case: &Transient, g: Grading, step_s: f64, repeats: usize) -> (Vec<f64>, f64) {
    let (x, y) = case.axes(&mesh_of(g)).expect("a valid mesh");
    let steps = (transient::END_US * 1e-6 / step_s).round() as usize;
    let rise = case.steady(&x, &y).expect("the heat solve");
    let mut best = f64::INFINITY;
    let mut history = Vec::new();
    for _ in 0..repeats {
        let t = Instant::now();
        history = case
            .photonoxide(&x, &y, step_s, steps)
            .expect("the transient");
        best = best.min(t.elapsed().as_secs_f64());
    }
    (transient::figures(&history, rise), best)
}

fn transient_pair(program: &Elmer, folder: &Path, progress: &mut Progress) -> Pair {
    let case = Transient::jacques();
    let elmer = |g: Grading, name: &str| {
        transient_elmer(program, &case, g, STEP_S, &folder.join(name), REPEATS)
    };
    let ours = |g: Grading| transient_ours(&case, g, STEP_S, REPEATS);
    let (rows, converged) = study(progress, "transient", &elmer, &ours);
    progress.next("the transient's step study");
    let mesh = (STEP_STUDY_MESH, GROWTH, SILICON_LARGEST);
    let steps: Vec<(f64, Vec<f64>, Vec<f64>)> = STEP_STUDY_S
        .iter()
        .map(|&dt| {
            let run = folder.join(format!("transient-step-{}", us(dt)));
            let e = transient_elmer(program, &case, mesh, dt, &run, 1).0;
            let o = transient_ours(&case, mesh, dt, 1).0;
            eprintln!("  step {} µs: Elmer {e:?}; photonoxide {o:?}", us(dt));
            (dt, e, o)
        })
        .collect();
    let mut more = format!(
        "The time step, on the {STEP_STUDY_MESH} µm mesh: each code's figures with steps from {} to {} µs, and how far each moves from the step before, in percent. The meshes above take {} µs, {} steps.\n\n| Step | Elmer: rise at 1, 5, 20 µs (K); 1/e time (µs) | Change | photonoxide: the same | Change |\n|---:|---|---|---|---|\n",
        us(STEP_STUDY_S[0]),
        us(STEP_STUDY_S[STEP_STUDY_S.len() - 1]),
        us(STEP_S),
        (transient::END_US * 1e-6 / STEP_S).round()
    );
    let list = |v: &[f64]| -> String {
        v.iter()
            .map(|x| format!("{x:.4}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let change = |v: &[f64], before: Option<&Vec<f64>>| -> String {
        before.map_or("—".to_owned(), |b| {
            v.iter()
                .zip(b)
                .map(|(x, y)| percent(*x, *y))
                .collect::<Vec<_>>()
                .join(", ")
        })
    };
    for (k, (dt, e, o)) in steps.iter().enumerate() {
        let before = k.checked_sub(1).map(|j| &steps[j]);
        let _ = writeln!(
            more,
            "| {} µs | {} | {} | {} | {} |",
            us(*dt),
            list(e),
            change(e, before.map(|b| &b.1)),
            list(o),
            change(o, before.map(|b| &b.2))
        );
    }
    more.push('\n');
    let rise = |t: f64| Quantity {
        name: match t {
            1.0 => "rise at 1 µs, K",
            5.0 => "rise at 5 µs, K",
            _ => "rise at 20 µs, K",
        },
        scale: 1.0,
        decimals: 4,
        tolerance: RISE_TOLERANCE,
    };
    let mut quantities: Vec<Quantity> = transient::TIMES_US.iter().map(|&t| rise(t)).collect();
    quantities.push(Quantity {
        name: "1/e time, µs",
        scale: 1.0,
        decimals: 4,
        tolerance: RISE_TOLERANCE,
    });
    Pair {
        case: "the heater in time",
        heading: "A heater turned on: HeatSolve in time",
        about: format!(
            "Jacques et al.'s N++ silicon heater of the two-heater problem, alone in the paper's 40 × 18 µm window, its sides insulated; half of it beside the mirror plane through the rib. Cold (0) until t = 0, then dissipating the paper's P_π, {:.2} W/m. Table 3's heat capacities per volume: silicon 2330 × 711 and oxide 2203 × 709 J/(m³·K). Elmer's HeatSolve in time, BDF of order 2 (its first step by backward Euler); photonoxide's transient, BDF2 (its first step by backward Euler), both with the same {} µs steps to {} µs. The rib's mean rise at 1, 5 and 20 µs, and its 1/e time, when it first reaches 1 − 1/e of its steady rise (each code's own steady rise on the same mesh, read between steps linearly). The paper's simulated limiting 1/e time, of the phase, is about 5.8 µs (Fig. 1(c)). The times are each code's whole run of steps. Elmer's HeatSolve, as set up here with its defaults, assembles and factorizes its matrix at every step (keeping the first step's factors isn't right here, as BDF2's matrix changes after its first step); photonoxide factorizes its two matrices once and then only solves, which is most of the difference in time.",
            heaters::WATTS_PER_METRE,
            us(STEP_S),
            transient::END_US
        ),
        quantities,
        rows,
        converged,
        more,
    }
}

fn joule_pair(program: &Elmer, folder: &Path, progress: &mut Progress) -> Pair {
    let case = Joule::harris();
    let axes = |g: Grading| case.axes(&mesh_of(g)).expect("a valid mesh");
    let elmer = |g: Grading, name: &str| {
        let (x, y) = axes(g);
        let c = case.elmer(&x, &y).expect("the case");
        let run = folder.join(name);
        let seconds = elmer_fastest(program, &c, &run);
        let row = &elmer::scalars(&run).expect("Elmer's scalars")[0];
        (vec![row[1], row[0]], seconds, c.mesh.cells())
    };
    let ours = |g: Grading| {
        let (x, y) = axes(g);
        let (v, s) = timed(|| case.photonoxide(&x, &y).expect("the solves"));
        (v.to_vec(), s)
    };
    let (rows, converged) = study(progress, "joule", &elmer, &ours);
    Pair {
        case: "the doped heater",
        heading: "Joule heating: StatCurrentSolver and HeatSolve",
        about: "Harris et al.'s doped silicon heater (Opt. Express 22, 10487 (2014), Section 2 and Fig. 1), the section through one of its channels, where the doped slab runs to the contacts: a ridge 500 nm wide and 220 nm high on a 90 nm slab, p-doped 1 µm wide across the guide and p++ beyond to ±3 µm, contacts on the slab's last 100 nm each side at 1 V and 0 V. The conductivities from the paper's sheet resistances, σ = 1/(R_s t): 8.17 × 10⁴ S/m (p++, 136 Ω), 817 S/m (p, 13.6 kΩ) and 1175 S/m (the ridge, 3.87 kΩ). As the paper's model has it, 2 µm of buried oxide with its bottom held at 0 and 10 µm of oxide over the silicon with its top held at 0; the window ±20 µm wide, its sides insulated. The thermal conductivities, which the paper doesn't give, are Jacques et al.'s Table 3's: the ridge 90, the slab 55, the oxide 1.38 W/(m·K). Elmer's StatCurrentSolver in the silicon, its Joule heating the source of HeatSolve, the current the sum of the first contact's nodal reactions (its Potential Loads); photonoxide's current continuity, its Joule heat the source of its heat solver, the current its contact's. The current per metre of guide and the peak rise.".to_owned(),
        quantities: vec![
            Quantity {
                name: "current at 1 V, A/m",
                scale: 1.0,
                decimals: 5,
                tolerance: CURRENT_TOLERANCE,
            },
            Quantity {
                name: "peak rise, K",
                scale: 1.0,
                decimals: 4,
                tolerance: RISE_TOLERANCE,
            },
        ],
        rows,
        converged,
        more: String::new(),
    }
}

fn tfln_pair(program: &Elmer, folder: &Path, progress: &mut Progress) -> Pair {
    let case = Tfln::zhang();
    let axes = |g: Grading| case.axes(&mesh_of(g)).expect("a valid mesh");
    let elmer = |g: Grading, name: &str| {
        let (x, y) = axes(g);
        let (c, field) = case.elmer(&x, &y).expect("the case");
        let run = folder.join(name);
        let seconds = elmer_fastest(program, &c, &run);
        let capacitance = 2.0 * elmer::result(&run, "capacitance").expect("Elmer's capacitance");
        let e = field
            .of(&last_saved(&run, "potential"))
            .expect("the guide's nodes");
        (vec![capacitance, e], seconds, c.mesh.cells())
    };
    let ours = |g: Grading| {
        let (x, y) = axes(g);
        let (v, s) = timed(|| case.photonoxide(&x, &y).expect("the electrostatic solve"));
        (v.to_vec(), s)
    };
    let (rows, converged) = study(progress, "tfln", &elmer, &ours);
    Pair {
        case: "the modulator's electrodes",
        heading: "A thin-film lithium niobate modulator: StatElecSolver with a tensor permittivity",
        about: "Zhang et al.'s cross-section (Photon. Res. 10, 2380 (2022), Section 2): an x-cut film 600 nm thick, etched 300 nm to a rib 1.2 µm wide (its sidewalls upright here, so that the mesh lines follow them), on 2 µm of thermal oxide (ε 3.9) on 20 µm of silicon (ε 11.7); electrodes 0.83 µm thick on the slab, a signal 20 µm wide 5 µm from a ground 15 µm wide (not given); 0.6 µm of PECVD oxide (ε 5.5, the paper's) and 20 µm of air above. The film's clamped static permittivity, diagonal in the section's frame: 26.2 across (along the crystal's c axis) and 45.5 up. Half of the push–pull line beside the mirror plane through the signal, every side of the window without surface charge. Elmer's StatElecSolver with the film's permittivity a tensor and the signal at 1 V, its capacitance from the field's energy (2W/V², which its capacitance matrix matches to ten digits) and the potential at the guide's sides; photonoxide's electrostatics with the diagonal tensor, the signal's charge at 1 V and its field E_x (from the displacement through each face) averaged over the guide's cells. The whole line's capacitance (twice the half's), and the mean E_x over the guide, the rib and the slab under it, at 1 V on the signal: what the Pockels effect sees, so what sets V_π·L. It is negative, the field pointing from the signal to the ground.".to_owned(),
        quantities: vec![
            Quantity {
                name: "capacitance, pF/m",
                scale: 1e12,
                decimals: 4,
                tolerance: CAPACITANCE_TOLERANCE,
            },
            Quantity {
                name: "mean E_x at 1 V, V/µm",
                scale: 1e-6,
                decimals: 6,
                tolerance: FIELD_TOLERANCE,
            },
        ],
        rows,
        converged,
        more: String::new(),
    }
}

#[test]
#[ignore = "runs Elmer, where it is installed: minutes"]
fn elmer_agrees_with_the_static_solver() {
    let program = match Elmer::find() {
        Ok(p) => p,
        Err(e) => {
            println!("skipped: {e}");
            return;
        }
    };
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("elmer");
    let all = ["line", "heater", "heaters", "transient", "joule", "tfln"];
    let pairs_steps = SILICON_SMALLEST.len() + 1;
    let steps = all
        .iter()
        .filter(|n| runs(n))
        .map(|&n| match n {
            "line" => LINE_SMALLEST.len() + 1,
            "heater" => HEATER_SMALLEST.len() + 1,
            "transient" => pairs_steps + 1,
            _ => pairs_steps,
        })
        .sum();
    let mut progress = Progress {
        started: Instant::now(),
        steps,
        done: 0,
    };

    // the coplanar line: n and Z₀ from the capacitance with the substrate and without
    let cpw = Cpw::ghione_naldi();
    let mut lines: Vec<Row<(f64, f64)>> = Vec::new();
    let mut line_converged: Vec<((f64, f64), f64)> = Vec::new();
    if runs("line") {
        for &smallest in &LINE_SMALLEST {
            progress.next(&format!("the line, {smallest} µm"));
            let (y, z) = cpw
                .axes(&cpw::Mesh {
                    smallest,
                    growth: GROWTH,
                    largest: LINE_LARGEST,
                })
                .expect("a valid mesh");
            let with = cpw.elmer(&y, &z, cpw.permittivity).expect("the case");
            let without = cpw.elmer(&y, &z, 1.0).expect("the case");
            let cells = with.mesh.cells();
            let (dielectric, air) = (
                folder.join(format!("cpw-{smallest}")),
                folder.join(format!("cpw-{smallest}-air")),
            );
            let c = capacitance(&program, &with, &dielectric);
            let c0 = capacitance(&program, &without, &air);
            let elmer = ((c / c0).sqrt(), 1.0 / (cpw::C * (c * c0).sqrt()));
            // its solvers' own times, from its logs: the two solves, as photonoxide's has two
            let elmer_seconds = elmer_fastest(&program, &with, &dielectric)
                + elmer_fastest(&program, &without, &air);
            let (ours, ours_seconds) = timed(|| cpw.static_line(&y, &z).expect("the static solve"));
            eprintln!(
                "  Elmer n {:.6} Z {:.4} ({elmer_seconds:.2} s); photonoxide n {:.6} Z {:.4} ({ours_seconds:.2} s)",
                elmer.0, elmer.1, ours.0, ours.1
            );
            lines.push(Row {
                smallest,
                cells,
                elmer,
                elmer_seconds,
                ours,
                ours_seconds,
            });
        }
        progress.next("the line, photonoxide converged");
        line_converged = LINE_CONVERGED
            .iter()
            .map(|&smallest| {
                let (y, z) = cpw
                    .axes(&cpw::Mesh {
                        smallest,
                        growth: LINE_CONVERGED_GROWTH,
                        largest: LINE_CONVERGED_LARGEST,
                    })
                    .expect("a valid mesh");
                timed(|| cpw.static_line(&y, &z).expect("the static solve"))
            })
            .collect();
    }

    // the heater: the peak rise
    let heater = Heater::oxide_on_silicon();
    let mut heats: Vec<Row<f64>> = Vec::new();
    let mut heat_converged: Vec<(f64, f64)> = Vec::new();
    if runs("heater") {
        for &smallest in &HEATER_SMALLEST {
            progress.next(&format!("the heater, {smallest} µm"));
            let (x, y) = heater
                .axes(&heater::Mesh {
                    smallest,
                    growth: GROWTH,
                    largest: HEATER_LARGEST,
                })
                .expect("a valid mesh");
            let case = heater.elmer(&x, &y).expect("the case");
            let cells = case.mesh.cells();
            let run = folder.join(format!("heater-{smallest}"));
            program.run(&case, &run).expect("Elmer's run");
            let elmer = elmer::scalars(&run).expect("Elmer's scalars")[0][0];
            let elmer_seconds = elmer_fastest(&program, &case, &run);
            let (ours, ours_seconds) =
                timed(|| heater.photonoxide(&x, &y).expect("the heat solve"));
            eprintln!(
                "  Elmer {elmer:.6} K ({elmer_seconds:.2} s); photonoxide {ours:.6} K ({ours_seconds:.2} s)"
            );
            heats.push(Row {
                smallest,
                cells,
                elmer,
                elmer_seconds,
                ours,
                ours_seconds,
            });
        }
        progress.next("the heater, photonoxide converged");
        heat_converged = HEATER_CONVERGED
            .iter()
            .map(|&smallest| {
                let (x, y) = heater
                    .axes(&heater::Mesh {
                        smallest,
                        growth: HEATER_CONVERGED_GROWTH,
                        largest: HEATER_CONVERGED_LARGEST,
                    })
                    .expect("a valid mesh");
                timed(|| heater.photonoxide(&x, &y).expect("the heat solve"))
            })
            .collect();
    }

    // the later pairs
    let mut pairs: Vec<Pair> = Vec::new();
    if runs("heaters") {
        pairs.push(heaters_pair(&program, &folder, &mut progress));
    }
    if runs("transient") {
        pairs.push(transient_pair(&program, &folder, &mut progress));
    }
    if runs("joule") {
        pairs.push(joule_pair(&program, &folder, &mut progress));
    }
    if runs("tfln") {
        pairs.push(tfln_pair(&program, &folder, &mut progress));
    }
    eprintln!(
        "[elmer] done in {:.0} s",
        progress.started.elapsed().as_secs_f64()
    );

    let mut checks: Vec<(String, f64, f64)> = Vec::new();
    let mut r = String::new();
    let complete = all.iter().all(|n| runs(n));
    if runs("line") && runs("heater") {
        let el_n = extrapolated(&lines.iter().map(|r| r.elmer.0).collect::<Vec<_>>());
        let el_z = extrapolated(&lines.iter().map(|r| r.elmer.1).collect::<Vec<_>>());
        let px_n = extrapolated(&lines.iter().map(|r| r.ours.0).collect::<Vec<_>>());
        let px_z = extrapolated(&lines.iter().map(|r| r.ours.1).collect::<Vec<_>>());
        let cv_n = extrapolated(&line_converged.iter().map(|c| c.0.0).collect::<Vec<_>>());
        let cv_z = extrapolated(&line_converged.iter().map(|c| c.0.1).collect::<Vec<_>>());
        let ((ref_n, ref_z), ref_line_seconds) = line_converged[line_converged.len() - 1];
        let el_t = extrapolated(&heats.iter().map(|r| r.elmer).collect::<Vec<_>>());
        let px_t = extrapolated(&heats.iter().map(|r| r.ours).collect::<Vec<_>>());
        let cv_t = extrapolated(&heat_converged.iter().map(|c| c.0).collect::<Vec<_>>());
        let (ref_t, ref_heat_seconds) = heat_converged[heat_converged.len() - 1];
        checks.push((
            "the line: n".into(),
            (el_n.0 - ref_n) / ref_n,
            INDEX_TOLERANCE,
        ));
        checks.push((
            "the line: Z₀".into(),
            (el_z.0 - ref_z) / ref_z,
            IMPEDANCE_TOLERANCE,
        ));
        checks.push((
            "the heater: peak rise".into(),
            (el_t.0 - ref_t) / ref_t,
            RISE_TOLERANCE,
        ));

        let line_last = &lines[lines.len() - 1];
        let heat_last = &heats[heats.len() - 1];

        let _ = writeln!(r, "# Cross-code comparison: Elmer\n");
        let _ = writeln!(
            r,
            "Written by `just elmer` on a computer with Elmer installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. How Elmer is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).\n"
        );
        let _ = writeln!(
            r,
            "Elmer {} (ElmerSolver, found through {}), its direct solver (UMFPACK); photonoxide {}. Run on {}. Both codes solve the same 2D problems on the same mesh lines: Elmer's nodes are photonoxide's cell faces, Elmer's bilinear quadrilaterals against photonoxide's cell-centred finite volumes. Every error is against photonoxide's converged result, the reference, in percent. Both codes solve the same problems, so their times are compared: each code's assembly and solve, the fastest of three runs, Elmer's as its log reports them (the Assembly and Solve times of StatElecSolver, StatCurrentSolver and HeatSolve, each step's in a transient), photonoxide's measured around its solve; neither includes starting a program or reading and writing files.{}\n",
            program.version(),
            program.found(),
            photonoxide::VERSION,
            machine(),
            loaded()
        );

        let _ = writeln!(r, "## Results\n");
        let _ = writeln!(
            r,
            "| Case | Quantity | photonoxide, converged | Elmer, extrapolated | Error | Elmer, finest mesh | Error | photonoxide, finest mesh | Error |\n|---|---|---:|---:|---:|---:|---:|---:|---:|"
        );
        for (quantity, reference, elmer_x, elmer_last, ours_last) in [
            ("n", ref_n, el_n.0, line_last.elmer.0, line_last.ours.0),
            ("Z₀, Ω", ref_z, el_z.0, line_last.elmer.1, line_last.ours.1),
        ] {
            let _ = writeln!(
                r,
                "| the coplanar line | {quantity} | {reference:.6} | {elmer_x:.6} | {} | {elmer_last:.6} | {} | {ours_last:.6} | {} |",
                percent(elmer_x, reference),
                percent(elmer_last, reference),
                percent(ours_last, reference)
            );
        }
        let _ = writeln!(
            r,
            "| the heater | peak rise, K | {ref_t:.6} | {:.6} | {} | {:.6} | {} | {:.6} | {} |",
            el_t.0,
            percent(el_t.0, ref_t),
            heat_last.elmer,
            percent(heat_last.elmer, ref_t),
            heat_last.ours,
            percent(heat_last.ours, ref_t)
        );
        for p in &pairs {
            r.push_str(&p.results());
        }
        let _ = writeln!(
            r,
            "\n| Case | Elmer, all meshes | photonoxide, the same meshes | Elmer, finest mesh | photonoxide, finest mesh | photonoxide, converged |\n|---|---:|---:|---:|---:|---:|"
        );
        let _ = writeln!(
            r,
            "| the coplanar line (two solves a mesh, in each code) | {} | {} | {} | {} | {} |",
            duration(totals(&lines).0),
            duration(totals(&lines).1),
            duration(line_last.elmer_seconds),
            duration(line_last.ours_seconds),
            duration(ref_line_seconds)
        );
        let _ = writeln!(
            r,
            "| the heater | {} | {} | {} | {} | {} |",
            duration(totals(&heats).0),
            duration(totals(&heats).1),
            duration(heat_last.elmer_seconds),
            duration(heat_last.ours_seconds),
            duration(ref_heat_seconds)
        );
        for p in &pairs {
            r.push_str(&p.times());
        }
        r.push('\n');

        let _ = writeln!(
            r,
            "## A coplanar waveguide in a box: StatElecSolver's capacitance\n"
        );
        let _ = writeln!(
            r,
            "Ghione and Naldi's line, as in [the openEMS report](validation-openems.md): perfectly conducting strips of zero thickness, the signal 20 µm wide, gaps of 10 µm, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high; half of it beside the mirror plane through the signal. Elmer's StatElecSolver gives the signal's capacitance with the substrate and without it (its Capacitance Matrix), so n = √(C/C₀) and Z₀ = 1/(c√(CC₀)), as photonoxide's quasi-TEM line has them. Cells from the smallest at the strips' edges and in their plane, growing by {GROWTH} to {LINE_LARGEST} µm.\n"
        );
        let _ = writeln!(
            r,
            "| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: n, Z₀ (Ω) | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ (Ω) | Error: n, Z₀ |\n|---:|---:|---:|---|---|---|---|"
        );
        for row in &lines {
            let _ = writeln!(
                r,
                "| {} µm | {} | {}, {} | {:.6}, {:.4} | {}, {} | {:.6}, {:.4} | {}, {} |",
                row.smallest,
                row.cells,
                duration(row.elmer_seconds),
                duration(row.ours_seconds),
                row.elmer.0,
                row.elmer.1,
                percent(row.elmer.0, ref_n),
                percent(row.elmer.1, ref_z),
                row.ours.0,
                row.ours.1,
                percent(row.ours.0, ref_n),
                percent(row.ours.1, ref_z)
            );
        }
        let _ = writeln!(
            r,
            "\n| | n | Error | Z₀, Ω | Error | From |\n|---|---:|---:|---:|---:|---|\n| photonoxide, converged (the reference) | {ref_n:.6} | — | {ref_z:.4} | — | graded from {} µm by {LINE_CONVERGED_GROWTH} (extrapolated n {:.6}, Z₀ {:.4}, {}) |\n| Elmer, extrapolated | {:.6} | {} | {:.4} | {} | the three finest meshes; n {}, Z₀ {} |\n| photonoxide on the same lines, extrapolated | {:.6} | {} | {:.4} | {} | the same; n {}, Z₀ {} |\n",
            LINE_CONVERGED[LINE_CONVERGED.len() - 1],
            cv_n.0,
            cv_z.0,
            order(cv_z.1),
            el_n.0,
            percent(el_n.0, ref_n),
            el_z.0,
            percent(el_z.0, ref_z),
            order(el_n.1),
            order(el_z.1),
            px_n.0,
            percent(px_n.0, ref_n),
            px_z.0,
            percent(px_z.0, ref_z),
            order(px_n.1),
            order(px_z.1)
        );

        let _ = writeln!(r, "## A heater over silicon: HeatSolver's peak rise\n");
        let _ = writeln!(
            r,
            "A heater 2 µm wide and 0.2 µm thick, its centre 2 µm up, dissipating 1 W/m in 3 µm of oxide (k = 1.38 W/(m·K)) on 10 µm of silicon (k = 148 W/(m·K)) whose bottom is held at 0, the other sides insulated; half of it beside the mirror plane through the heater, in a window 30 µm wide. Elmer's HeatSolver with the heater's power as a heat source, its peak temperature by SaveScalars; photonoxide's heat solver's peak. Cells from the smallest at the heater's edges and the oxide's interface, growing by {GROWTH} to {HEATER_LARGEST} µm.\n"
        );
        let _ = writeln!(
            r,
            "| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: peak rise (K) | Error | photonoxide, the same lines (K) | Error |\n|---:|---:|---:|---:|---:|---:|---:|"
        );
        for row in &heats {
            let _ = writeln!(
                r,
                "| {} µm | {} | {}, {} | {:.6} | {} | {:.6} | {} |",
                row.smallest,
                row.cells,
                duration(row.elmer_seconds),
                duration(row.ours_seconds),
                row.elmer,
                percent(row.elmer, ref_t),
                row.ours,
                percent(row.ours, ref_t)
            );
        }
        let _ = writeln!(
            r,
            "\n| | Peak rise, K | Error | From |\n|---|---:|---:|---|\n| photonoxide, converged (the reference) | {ref_t:.6} | — | graded from {} µm by {HEATER_CONVERGED_GROWTH} (extrapolated {:.6}, {}) |\n| Elmer, extrapolated | {:.6} | {} | the three finest meshes, {} |\n| photonoxide on the same lines, extrapolated | {:.6} | {} | the same, {} |\n",
            HEATER_CONVERGED[HEATER_CONVERGED.len() - 1],
            cv_t.0,
            order(cv_t.1),
            el_t.0,
            percent(el_t.0, ref_t),
            order(el_t.1),
            px_t.0,
            percent(px_t.0, ref_t),
            order(px_t.1)
        );
    }
    for p in &pairs {
        if !(runs("line") && runs("heater")) {
            r.push_str(&p.results());
            r.push_str(&p.times());
        }
        r.push_str(&p.section());
        checks.extend(p.checks());
    }
    let summary: Vec<String> = checks
        .iter()
        .map(|(what, error, tolerance)| {
            format!(
                "{what} {} (tolerance {} %)",
                signed_percent(100.0 * error),
                100.0 * tolerance
            )
        })
        .collect();
    let _ = writeln!(
        r,
        "Elmer extrapolated against photonoxide converged: {}.",
        summary.join("; ")
    );
    print!("{r}");
    if complete {
        std::fs::write(REPORT, &r).expect("docs/validation-elmer.md");
    }

    let missed: Vec<&String> = checks
        .iter()
        // a NaN error misses too
        .filter(|(_, error, tolerance)| error.is_nan() || error.abs() > *tolerance)
        .map(|(what, ..)| what)
        .collect();
    assert!(missed.is_empty(), "outside their tolerances: {missed:?}");
}

/// The note on the times when `PHOTONOXIDE_LOADED` says the computer was busy with other work
/// as they were taken.
fn loaded() -> &'static str {
    match std::env::var("PHOTONOXIDE_LOADED") {
        Ok(v) if !v.trim().is_empty() && v.trim() != "0" => {
            " **These times were taken under load**, other builds running on the computer at the same time: the agreement stands, but the times are only rough until they are taken again with the computer idle."
        }
        _ => "",
    }
}

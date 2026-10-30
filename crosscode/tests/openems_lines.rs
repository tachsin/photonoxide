//! openEMS against photonoxide on more lines: docs/validation-openems-lines.md, written here
//! where openEMS is installed (never in CI), in release, from the repository's root:
//!
//! ```sh
//! PHOTONOXIDE_OPENEMS=C:/openEMS cargo test --manifest-path crosscode/Cargo.toml --release --test openems_lines -- --ignored --nocapture
//! ```
//!
//! It skips, saying so, where openEMS isn't found. The runs log their progress to stderr and
//! to `progress.log` in their folders under `crosscode/target/tmp/openems/`; a finished run of
//! the same model is taken again. Where `PHOTONOXIDE_OPENEMS_ONLY` names some of the lines
//! (`microstrip`, comma-separated), only those run, and the report is printed, not written.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use photonoxide_crosscode::cpw::{FREQUENCIES_GHZ, LENGTH, Measured, Mesh, measured};
use photonoxide_crosscode::microstrip::Microstrip;
use photonoxide_crosscode::openems::OpenEms;
use photonoxide_crosscode::report::{percent, signed_percent};

/// The microstrip's meshes' smallest cells, µm, each half the last: three meshes, the fewest
/// an extrapolation takes.
const STRIP_SMALLEST: [f64; 3] = [2.0, 1.0, 0.5];
/// Their growth and largest cell.
const GROWTH: f64 = 1.1;
const LARGEST: f64 = 10.0;
/// The end criterion: the energy down by 70 dB, as the coplanar line's.
const END: f64 = 1e-7;
/// photonoxide's converged grid study: its smallest cells, growth and largest.
const CONVERGED: [f64; 3] = [0.01, 0.005, 0.0025];
const CONVERGED_GROWTH: f64 = 1.025;
const CONVERGED_LARGEST: f64 = 2.0;

/// openEMS extrapolated against photonoxide converged, relative: n and Z₀. The basis: openEMS's
/// Z₀ converges at first order at a strip's edge, and the coplanar line's extrapolation from
/// the same three meshes (2, 1 and 0.5 µm) comes within 0.15 % of the reference; 0.5 % holds
/// that with room for a second line's own edge.
const INDEX_TOLERANCE: f64 = 5e-3;
const IMPEDANCE_TOLERANCE: f64 = 5e-3;

const REPORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/validation-openems-lines.md"
);

/// A quantity's limit at zero frequency, by least squares on v(f) = v₀ + a f² over the
/// measured frequencies: the quasi-TEM line's.
fn at_zero(values: &[(f64, f64)]) -> f64 {
    let n = values.len() as f64;
    let (sx, sy) = values
        .iter()
        .fold((0.0, 0.0), |(a, b), &(f, v)| (a + f * f, b + v));
    let (sxx, sxy) = values
        .iter()
        .fold((0.0, 0.0), |(a, b), &(f, v)| (a + f.powi(4), b + f * f * v));
    let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
    (sy - slope * sx) / n
}

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

/// openEMS's threads: `PHOTONOXIDE_OPENEMS_THREADS`, or 8.
fn threads() -> usize {
    std::env::var("PHOTONOXIDE_OPENEMS_THREADS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(8)
}

/// Whether line `name` runs: all of them, unless `PHOTONOXIDE_OPENEMS_ONLY` names some.
fn runs(name: &str) -> bool {
    match std::env::var("PHOTONOXIDE_OPENEMS_ONLY") {
        Ok(only) if !only.trim().is_empty() => only.split(',').any(|n| n.trim() == name),
        _ => true,
    }
}

fn sig(v: f64, digits: usize) -> String {
    format!("{v:.digits$}")
}

fn order(p: f64) -> String {
    if p.is_finite() {
        format!("order {p:.2}")
    } else {
        "no order: the finest shown".to_owned()
    }
}

/// One mesh's runs.
struct Row {
    smallest: f64,
    cells: usize,
    steps: u64,
    seconds: f64,
    lines: Vec<Measured>,
    index: f64,
    ohm: f64,
    ours: (f64, f64),
}

/// A check: what, openEMS's error against photonoxide converged, the tolerance.
type Check = (String, f64, f64);

fn microstrip(program: &OpenEms, folder: &Path, started: Instant) -> (String, Vec<Check>) {
    let line = Microstrip::alumina_like();
    let mut rows = Vec::new();
    let total = STRIP_SMALLEST.len();
    for (k, &smallest) in STRIP_SMALLEST.iter().enumerate() {
        let mesh = Mesh {
            smallest,
            growth: GROWTH,
            largest: LARGEST,
        };
        let (y, z) = line.axes(&mesh).expect("a valid mesh");
        let model = line.model(&y, &z, END);
        let elapsed = started.elapsed().as_secs_f64();
        eprintln!(
            "[openems-lines] step {}/{total} ({:.0}%), {elapsed:.0} s elapsed: the microstrip, {smallest} µm, {} cells",
            k + 1,
            100.0 * k as f64 / total as f64,
            model.cells()
        );
        let label = format!("microstrip {}/{total}, {smallest} µm", k + 1);
        let run = program
            .run(
                &model,
                &folder.join(format!("microstrip-{smallest}")),
                &label,
            )
            .expect("openEMS's run");
        assert!(
            run.decayed,
            "{label}: the energy didn't fall to the end criterion"
        );
        let lines = measured(&run, &FREQUENCIES_GHZ).expect("openEMS's probes");
        let fit = |f: fn(&Measured) -> f64| {
            at_zero(
                &lines
                    .iter()
                    .map(|m| (m.frequency_ghz, f(m)))
                    .collect::<Vec<_>>(),
            )
        };
        let (index, ohm) = (fit(Measured::index), fit(Measured::ohm));
        let ours = line.static_line(&y, &z).expect("the static solve");
        eprintln!(
            "  openEMS n {index:.6} Z {ohm:.4}; photonoxide on the same lines n {:.6} Z {:.4}",
            ours.0, ours.1
        );
        rows.push(Row {
            smallest,
            cells: model.cells(),
            steps: run.steps,
            seconds: run.seconds,
            lines,
            index,
            ohm,
            ours,
        });
    }
    let converged: Vec<(f64, f64)> = CONVERGED
        .iter()
        .map(|&smallest| {
            let (y, z) = line
                .axes(&Mesh {
                    smallest,
                    growth: CONVERGED_GROWTH,
                    largest: CONVERGED_LARGEST,
                })
                .expect("a valid mesh");
            line.static_line(&y, &z).expect("the static solve")
        })
        .collect();
    let column = |f: fn(&Row) -> f64| rows.iter().map(f).collect::<Vec<_>>();
    let (oe_n, oe_np) = extrapolated(&column(|r| r.index));
    let (oe_z, oe_zp) = extrapolated(&column(|r| r.ohm));
    let (px_n, px_np) = extrapolated(&column(|r| r.ours.0));
    let (px_z, px_zp) = extrapolated(&column(|r| r.ours.1));
    let (cv_n, _) = extrapolated(&converged.iter().map(|c| c.0).collect::<Vec<_>>());
    let (cv_z, cv_zp) = extrapolated(&converged.iter().map(|c| c.1).collect::<Vec<_>>());
    let (ref_n, ref_z) = converged[converged.len() - 1];
    let last = &rows[rows.len() - 1];

    let mut out = String::new();
    let _ = writeln!(
        out,
        "## A microstrip line in a box\n\n\
         A perfectly conducting strip of zero thickness, 20 µm wide, on 20 µm of a substrate of \
         ε = 10 (w/h = 1) over a ground plane, in a perfectly conducting box ±{r} µm across and \
         {r} µm high above the strip, its walls at the ground's potential. openEMS runs a \
         section {} mm long as the coplanar line's (25 µm cells along it, 8-cell PMLs at both \
         ends), driven by a soft source of E between the strip and the ground (a Gaussian pulse \
         of 5 to 55 GHz) until the energy is down by 70 dB; k and Z₀ at each frequency come from \
         the voltage from the strip down to the ground at three planes 1025 µm apart and the \
         current around the strip midway (a loop ±{} µm across and ±5 µm about the strip, clear \
         of the ground), its quasi-static n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². \
         photonoxide's quasi-TEM line is its static solver's on half the box: n = √(C/C₀), \
         Z₀ = 1/(c√(CC₀)). Across the line both take the same mesh lines, cells from the \
         smallest at the strip's edge and in its plane growing by {GROWTH} to {LARGEST} µm; \
         three meshes, the fewest an extrapolation takes, to keep openEMS's runs to about an \
         hour.\n\n\
         The closed form, Hammerstad and Jensen's (1980), will stand beside both codes: pending \
         the paper.\n",
        LENGTH / 1000.0,
        2.0 * line.half_width,
        r = line.reach
    );
    let _ = writeln!(
        out,
        "| | n | Error | Z₀, Ω | Error |\n|---|---:|---:|---:|---:|\n\
         | photonoxide, converged (the reference) | {} | — | {} | — |\n\
         | openEMS, extrapolated from its three meshes | {} | {} | {} | {} |\n\
         | photonoxide on the same lines, extrapolated | {} | {} | {} | {} |\n\
         | openEMS on its finest mesh ({} µm) | {} | {} | {} | {} |\n\
         | photonoxide on the same mesh | {} | {} | {} | {} |\n\
         | Hammerstad and Jensen (1980), closed form | pending the paper | | pending the paper | |\n",
        sig(ref_n, 6),
        sig(ref_z, 4),
        sig(oe_n, 6),
        percent(oe_n, ref_n),
        sig(oe_z, 4),
        percent(oe_z, ref_z),
        sig(px_n, 6),
        percent(px_n, ref_n),
        sig(px_z, 4),
        percent(px_z, ref_z),
        last.smallest,
        sig(last.index, 6),
        percent(last.index, ref_n),
        sig(last.ohm, 4),
        percent(last.ohm, ref_z),
        sig(last.ours.0, 6),
        percent(last.ours.0, ref_n),
        sig(last.ours.1, 4),
        percent(last.ours.1, ref_z)
    );
    let _ = writeln!(
        out,
        "| Smallest cell | openEMS's cells | Steps (its run) | n at {} GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |\n|---:|---:|---:|---|---|---|---|---|---|",
        FREQUENCIES_GHZ.map(|f| format!("{f:.0}")).join(", ")
    );
    for r in &rows {
        let _ = writeln!(
            out,
            "| {} µm | {} | {} ({:.0} min) | {} | {} | {}, {} | {}, {} | {}, {} | {}, {} |",
            r.smallest,
            r.cells,
            r.steps,
            r.seconds / 60.0,
            r.lines
                .iter()
                .map(|m| sig(m.index(), 5))
                .collect::<Vec<_>>()
                .join(", "),
            r.lines
                .iter()
                .map(|m| sig(m.ohm(), 3))
                .collect::<Vec<_>>()
                .join(", "),
            sig(r.index, 5),
            sig(r.ohm, 3),
            percent(r.index, ref_n),
            percent(r.ohm, ref_z),
            sig(r.ours.0, 5),
            sig(r.ours.1, 3),
            percent(r.ours.0, ref_n),
            percent(r.ours.1, ref_z)
        );
    }
    let _ = writeln!(
        out,
        "\n| | n | Z₀, Ω | From |\n|---|---:|---:|---|\n\
         | openEMS, extrapolated | {} | {} | its three meshes; n {}, Z₀ {} |\n\
         | photonoxide on the same lines, extrapolated | {} | {} | the same; n {}, Z₀ {} |\n\
         | photonoxide, converged | {} | {} | graded from {} µm by {CONVERGED_GROWTH} to {CONVERGED_LARGEST} µm (from {} µm: n {}, Z₀ {}; extrapolated n {}, Z₀ {}, {}) |\n",
        sig(oe_n, 5),
        sig(oe_z, 3),
        order(oe_np),
        order(oe_zp),
        sig(px_n, 5),
        sig(px_z, 3),
        order(px_np),
        order(px_zp),
        sig(ref_n, 5),
        sig(ref_z, 3),
        CONVERGED[CONVERGED.len() - 1],
        CONVERGED.map(|s| s.to_string()).join(", "),
        converged
            .iter()
            .map(|c| sig(c.0, 6))
            .collect::<Vec<_>>()
            .join(", "),
        converged
            .iter()
            .map(|c| sig(c.1, 4))
            .collect::<Vec<_>>()
            .join(", "),
        sig(cv_n, 6),
        sig(cv_z, 4),
        order(cv_zp)
    );
    let checks = vec![
        (
            "the microstrip: n".to_owned(),
            (oe_n - ref_n) / ref_n,
            INDEX_TOLERANCE,
        ),
        (
            "the microstrip: Z₀".to_owned(),
            (oe_z - ref_z) / ref_z,
            IMPEDANCE_TOLERANCE,
        ),
    ];
    (out, checks)
}

#[test]
#[ignore = "runs openEMS, where it is installed: hours"]
fn openems_agrees_on_more_lines() {
    let program = match OpenEms::find() {
        Ok(p) => p.with_threads(threads()),
        Err(e) => {
            println!("skipped: {e}");
            return;
        }
    };
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("openems");
    let started = Instant::now();
    let all = ["microstrip"];
    let mut sections = String::new();
    let mut checks: Vec<Check> = Vec::new();
    if runs("microstrip") {
        let (s, c) = microstrip(&program, &folder, started);
        sections.push_str(&s);
        checks.extend(c);
    }
    eprintln!(
        "[openems-lines] done in {:.0} s",
        started.elapsed().as_secs_f64()
    );

    let mut r = String::new();
    let _ = writeln!(r, "# Cross-code comparison: openEMS on more lines\n");
    let _ = writeln!(
        r,
        "Written by `cargo test --release --test openems_lines -- --ignored` (crosscode/README.md) on a computer with openEMS installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. The coplanar line, the first of these pairs, is in [its own report](validation-openems.md); how openEMS is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).\n"
    );
    let _ = writeln!(
        r,
        "openEMS {} (CSXCAD {}), found through {}, its multithreaded engine on {} threads (fewer than the coplanar line's 8, other work sharing the computer); photonoxide {}. Every error is against photonoxide's converged result, the reference, in percent. These pairs compare accuracy only: openEMS solves Maxwell's equations in time on a 3D section of each line, photonoxide the 2D cross-section's static fields once, so their times measure different problems; openEMS's run times are shown for planning only.\n",
        program.version(),
        program.csxcad(),
        program.found(),
        program
            .threads()
            .map_or("its own choice of".into(), |t| t.to_string()),
        photonoxide::VERSION
    );
    r.push_str(&sections);
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
        "openEMS extrapolated against photonoxide converged: {}.",
        summary.join("; ")
    );
    print!("{r}");
    if all.iter().all(|n| runs(n)) {
        std::fs::write(REPORT, &r).expect("docs/validation-openems-lines.md");
    }
    let missed: Vec<&String> = checks
        .iter()
        .filter(|(_, error, tolerance)| error.is_nan() || error.abs() > *tolerance)
        .map(|(what, ..)| what)
        .collect();
    assert!(missed.is_empty(), "outside their tolerances: {missed:?}");
}

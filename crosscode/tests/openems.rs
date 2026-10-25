//! openEMS against photonoxide's quasi-TEM line: docs/validation-openems.md, written here
//! where openEMS is installed (never in CI), in release, from the repository's root:
//!
//! ```sh
//! PHOTONOXIDE_OPENEMS=C:/openEMS just openems
//! ```
//!
//! It skips, saying so, where openEMS isn't found. The runs log their progress to stderr and
//! to `progress.log` in their folders under `crosscode/target/tmp/openems/`; a finished run of
//! the same model is taken again.

use std::fmt::Write as _;
use std::path::Path;

use photonoxide_crosscode::chart::{self, Point};
use photonoxide_crosscode::cpw::{Cpw, FREQUENCIES_GHZ, LENGTH, Measured, Mesh, measured};
use photonoxide_crosscode::openems::OpenEms;
use photonoxide_crosscode::report::percent;

/// The meshes' smallest cells, µm, each half the last.
const SMALLEST: [f64; 4] = [2.0, 1.0, 0.5, 0.25];
/// Their growth and largest cell.
const GROWTH: f64 = 1.1;
const LARGEST: f64 = 10.0;
/// The end criterion: the energy down by 70 dB (past it, a slow tail near -75 dB takes as long
/// again as the pulse).
const END: f64 = 1e-7;
/// photonoxide's converged grid study: its smallest cells, growth and largest.
const CONVERGED: [f64; 3] = [0.01, 0.005, 0.0025];
const CONVERGED_GROWTH: f64 = 1.025;
const CONVERGED_LARGEST: f64 = 2.0;

/// openEMS against photonoxide: the quasi-static index and impedance, relative.
const INDEX_TOLERANCE: f64 = 1e-3;
const IMPEDANCE_TOLERANCE: f64 = 5e-3;

const REPORT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/validation-openems.md");

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

/// openEMS's threads: `PHOTONOXIDE_OPENEMS_THREADS`, or 8 (the fastest of 4, 8 and 16 on a
/// 20-thread Core Ultra 7 265K, 258 million cells a second on the 0.5 µm mesh).
fn threads() -> usize {
    std::env::var("PHOTONOXIDE_OPENEMS_THREADS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(8)
}

struct Row {
    smallest: f64,
    cells: usize,
    steps: u64,
    timestep: f64,
    seconds: f64,
    lines: Vec<Measured>,
    index: f64,
    ohm: f64,
    ours: (f64, f64),
    ours_cells: usize,
}

#[test]
#[ignore = "runs openEMS, where it is installed: hours"]
fn openems_agrees_with_the_quasi_tem_line() {
    let program = match OpenEms::find() {
        Ok(p) => p.with_threads(threads()),
        Err(e) => {
            println!("skipped: {e}");
            return;
        }
    };
    let cpw = Cpw::ghione_naldi();
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("openems");
    let mut rows = Vec::new();
    for (k, &smallest) in SMALLEST.iter().enumerate() {
        let mesh = Mesh {
            smallest,
            growth: GROWTH,
            largest: LARGEST,
        };
        let (y, z) = cpw.axes(&mesh).expect("a valid mesh");
        let model = cpw.model(&y, &z, END);
        let label = format!("openEMS {}/{}, {smallest} µm", k + 1, SMALLEST.len());
        let run = program
            .run(&model, &folder.join(format!("cpw-{smallest}")), &label)
            .expect("openEMS's run");
        assert!(
            run.decayed,
            "{label}: the energy didn't fall to the end criterion"
        );
        let lines = measured(&run, &FREQUENCIES_GHZ).expect("openEMS's probes");
        let index = at_zero(
            &lines
                .iter()
                .map(|m| (m.frequency_ghz, m.index()))
                .collect::<Vec<_>>(),
        );
        let ohm = at_zero(
            &lines
                .iter()
                .map(|m| (m.frequency_ghz, m.ohm()))
                .collect::<Vec<_>>(),
        );
        let ours = cpw.static_line(&y, &z).expect("the static solve");
        eprintln!(
            "{label}: n {index:.6} Z {ohm:.4}; photonoxide on the same lines n {:.6} Z {:.4}",
            ours.0, ours.1
        );
        rows.push(Row {
            smallest,
            cells: model.cells(),
            steps: run.steps,
            timestep: run.timestep,
            seconds: run.seconds,
            lines,
            index,
            ohm,
            ours,
            ours_cells: y.cells() * z.cells(),
        });
    }
    // photonoxide converged: the quasi-TEM line on finer grids, its own grid study
    let converged: Vec<(f64, f64)> = CONVERGED
        .iter()
        .map(|&smallest| {
            let (y, z) = cpw
                .axes(&Mesh {
                    smallest,
                    growth: CONVERGED_GROWTH,
                    largest: CONVERGED_LARGEST,
                })
                .expect("a valid mesh");
            let line = cpw.quasi_tem(&y, &z).expect("the quasi-TEM line");
            (line.index(), line.impedance())
        })
        .collect();
    let column = |f: fn(&Row) -> f64| rows.iter().map(f).collect::<Vec<_>>();
    let (oe_n, oe_np) = extrapolated(&column(|r| r.index));
    let (oe_z, oe_zp) = extrapolated(&column(|r| r.ohm));
    let (px_n, px_np) = extrapolated(&column(|r| r.ours.0));
    let (px_z, px_zp) = extrapolated(&column(|r| r.ours.1));
    let (cv_n, _) = extrapolated(&converged.iter().map(|c| c.0).collect::<Vec<_>>());
    let (cv_z, cv_zp) = extrapolated(&converged.iter().map(|c| c.1).collect::<Vec<_>>());
    let finest = converged[converged.len() - 1];

    let report = write_report(
        &program,
        &rows,
        &converged,
        [oe_n, oe_np, oe_z, oe_zp],
        [px_n, px_np, px_z, px_zp],
        [cv_n, cv_z, cv_zp],
    );
    print!("{report}");
    std::fs::write(REPORT, &report).expect("docs/validation-openems.md");

    // the chart's points: openEMS's own run time and its errors (photonoxide's own points on
    // these meshes come from the Elmer study, which times its solves)
    let mut points = Vec::new();
    for r in &rows {
        for (quantity, value, reference) in [("n", r.index, finest.0), ("Z₀", r.ohm, finest.1)] {
            points.push(Point {
                case: "the coplanar line".into(),
                quantity: quantity.into(),
                code: "openEMS".into(),
                smallest_um: r.smallest,
                value,
                error_percent: 100.0 * (value - reference) / reference,
                seconds: r.seconds,
            });
        }
    }
    chart::write_points("openems", &points).expect("the chart's points");
    chart::redraw().expect("the chart");

    let index_error = (oe_n - finest.0).abs() / finest.0;
    let impedance_error = (oe_z - finest.1).abs() / finest.1;
    assert!(
        index_error <= INDEX_TOLERANCE,
        "openEMS's index {oe_n} against photonoxide's {}: {index_error:.2e}",
        finest.0
    );
    assert!(
        impedance_error <= IMPEDANCE_TOLERANCE,
        "openEMS's impedance {oe_z} against photonoxide's {}: {impedance_error:.2e}",
        finest.1
    );
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

fn write_report(
    program: &OpenEms,
    rows: &[Row],
    converged: &[(f64, f64)],
    openems: [f64; 4],
    same_lines: [f64; 4],
    ours: [f64; 3],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Validation report: openEMS\n");
    let _ = writeln!(
        out,
        "Written by `just openems` on a computer with openEMS installed; don't edit it by \
         hand. It is a local comparison, never run in CI, and [the main report](validation.md) \
         doesn't depend on it. How openEMS is found and run, and its licence: [cross-code \
         references](cross-code.md); how to run it: \
         [crosscode/README.md](../crosscode/README.md).\n"
    );
    let _ = writeln!(
        out,
        "openEMS {} (CSXCAD {}), found through {}, its multithreaded engine on {} threads; \
         photonoxide {}.\n",
        program.version(),
        program.csxcad(),
        program.found(),
        program
            .threads()
            .map_or("its own choice of".into(), |t| t.to_string()),
        photonoxide::VERSION
    );
    let _ = writeln!(
        out,
        "## A coplanar waveguide in a box: 3D FDTD against the quasi-TEM line\n\n\
         Ghione and Naldi's line (G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984), \
         doi:10.1049/el:19840120): perfectly conducting strips of zero thickness, the signal 20 \
         µm wide, gaps of 10 µm, on 20 µm of a substrate of ε = 10, air below it and above the \
         strips, in a perfectly conducting box ±200 µm across and high, the grounds out to its \
         sides. openEMS runs a section 4 mm long (25 µm cells along it, 8-cell PMLs at both \
         ends that the line runs into) driven across both gaps by a Gaussian pulse of 5 to 55 \
         GHz, until the energy is down by 70 dB, and gives k and Z₀ at each frequency from the \
         voltage across a gap at three planes 1025 µm apart and the current on the signal \
         midway (exact for a uniform line, whatever its ends reflect); its quasi-static n and \
         Z₀ are the limits at 0 GHz of v(f) = v₀ + a f² fitted to its five frequencies. \
         photonoxide's quasi-TEM line is the static solver's, on half the box beside its \
         mirror: n = √(C/C₀), Z₀ = 1/(c√(CC₀)).\n\n\
         Across the line both codes take the same mesh lines: cells from the smallest at the \
         strips' edges and in their plane, growing by {GROWTH} to {LARGEST} µm (openEMS's the \
         mirror image of photonoxide's half).\n"
    );
    let [oe_n, oe_np, oe_z, oe_zp] = openems;
    let [px_n, px_np, px_z, px_zp] = same_lines;
    let [cv_n, cv_z, cv_zp] = ours;
    let finest = converged[converged.len() - 1];
    let last = &rows[rows.len() - 1];
    let _ = writeln!(out, "### Summary\n");
    let _ = writeln!(
        out,
        "Every error is against photonoxide's converged quasi-TEM line, the reference, in \
         percent. These tables give no times: openEMS solves a different problem, a 3D section \
         of the line {} mm long in time (on the finest mesh {} cells for {} steps), where \
         photonoxide solves the 2D cross-section's static fields once ({} unknowns). Each \
         code's own time for its accuracy is on [the cross-code chart](cross-code.md), with \
         every case and code.\n",
        LENGTH / 1000.0,
        last.cells,
        last.steps,
        last.ours_cells
    );
    let _ = writeln!(
        out,
        "| | n | Error | Z₀, Ω | Error |\n|---|---:|---:|---:|---:|"
    );
    let _ = writeln!(
        out,
        "| photonoxide, converged (the reference) | {} | — | {} | — |",
        sig(finest.0, 6),
        sig(finest.1, 4)
    );
    let _ = writeln!(
        out,
        "| openEMS, extrapolated from its three finest meshes | {} | {} | {} | {} |",
        sig(oe_n, 6),
        percent(oe_n, finest.0),
        sig(oe_z, 4),
        percent(oe_z, finest.1)
    );
    let _ = writeln!(
        out,
        "| photonoxide on the same lines, extrapolated | {} | {} | {} | {} |",
        sig(px_n, 6),
        percent(px_n, finest.0),
        sig(px_z, 4),
        percent(px_z, finest.1)
    );
    let _ = writeln!(
        out,
        "| openEMS on its finest mesh ({} µm) | {} | {} | {} | {} |",
        last.smallest,
        sig(last.index, 6),
        percent(last.index, finest.0),
        sig(last.ohm, 4),
        percent(last.ohm, finest.1)
    );
    let _ = writeln!(
        out,
        "| photonoxide on the same mesh | {} | {} | {} | {} |\n",
        sig(last.ours.0, 6),
        percent(last.ours.0, finest.0),
        sig(last.ours.1, 4),
        percent(last.ours.1, finest.1)
    );
    let _ = writeln!(out, "### On each mesh\n");
    let _ = writeln!(
        out,
        "| Smallest cell | openEMS's cells | Steps (time step) | n at {} GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |",
        FREQUENCIES_GHZ.map(|f| format!("{f:.0}")).join(", ")
    );
    let _ = writeln!(out, "|---:|---:|---:|---|---|---|---|---|---|");
    for r in rows {
        let _ = writeln!(
            out,
            "| {} µm | {} | {} ({:.3e} s) | {} | {} | {}, {} | {}, {} | {}, {} | {}, {} |",
            r.smallest,
            r.cells,
            r.steps,
            r.timestep,
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
            percent(r.index, finest.0),
            percent(r.ohm, finest.1),
            sig(r.ours.0, 5),
            sig(r.ours.1, 3),
            percent(r.ours.0, finest.0),
            percent(r.ours.1, finest.1),
        );
    }
    let _ = writeln!(out, "\n### Converged\n");
    let _ = writeln!(out, "| | n | Z₀, Ω | From |");
    let _ = writeln!(out, "|---|---:|---:|---|");
    let _ = writeln!(
        out,
        "| openEMS, extrapolated | {} | {} | the three finest meshes; n {}, Z₀ {} |",
        sig(oe_n, 5),
        sig(oe_z, 3),
        order(oe_np),
        order(oe_zp)
    );
    let _ = writeln!(
        out,
        "| photonoxide on the same lines, extrapolated | {} | {} | the same; n {}, Z₀ {} |",
        sig(px_n, 5),
        sig(px_z, 3),
        order(px_np),
        order(px_zp)
    );
    let _ = writeln!(
        out,
        "| photonoxide, converged | {} | {} | `QuasiTem` from {} µm, growing by {CONVERGED_GROWTH} to {CONVERGED_LARGEST} µm (from {} µm: n {}, Z₀ {}; extrapolated n {}, Z₀ {}, {}) |",
        sig(finest.0, 5),
        sig(finest.1, 3),
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
    let _ = writeln!(
        out,
        "\nopenEMS against photonoxide converged: n {}, Z₀ {} (tolerances {} % and {} %).",
        percent(oe_n, finest.0),
        percent(oe_z, finest.1),
        100.0 * INDEX_TOLERANCE,
        100.0 * IMPEDANCE_TOLERANCE
    );
    out
}

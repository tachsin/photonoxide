//! Elmer against photonoxide's static solver: docs/validation-elmer.md, written here where
//! Elmer is installed (never in CI), in release, from the repository's root:
//!
//! ```sh
//! PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer
//! ```
//!
//! It skips, saying so, where Elmer isn't found. Its runs are kept in folders under
//! `crosscode/target/tmp/elmer/`, each with Elmer's output in `elmer.log`.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use photonoxide_crosscode::cpw::{self, Cpw};
use photonoxide_crosscode::elmer::{self, Elmer};
use photonoxide_crosscode::heater::{self, Heater};

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

/// Elmer extrapolated against photonoxide converged, relative.
const INDEX_TOLERANCE: f64 = 1e-3;
const IMPEDANCE_TOLERANCE: f64 = 5e-3;
const RISE_TOLERANCE: f64 = 5e-3;

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

struct LineRow {
    smallest: f64,
    cells: usize,
    seconds: f64,
    elmer: (f64, f64),
    ours: (f64, f64),
}

struct HeatRow {
    smallest: f64,
    cells: usize,
    seconds: f64,
    elmer: f64,
    ours: f64,
}

/// Elmer's capacitance of the whole line (twice the half's), F/m.
fn capacitance(program: &Elmer, case: &elmer::Case, folder: &Path) -> f64 {
    program.run(case, folder).expect("Elmer's run");
    2.0 * elmer::capacitance(folder).expect("Elmer's capacitance")[0][0]
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
    let started = Instant::now();
    let steps = LINE_SMALLEST.len() + HEATER_SMALLEST.len();
    let progress = |k: usize, what: &str| {
        let elapsed = started.elapsed().as_secs_f64();
        let eta = if k > 0 {
            elapsed / k as f64 * (steps - k) as f64
        } else {
            0.0
        };
        eprintln!(
            "[elmer] step {}/{steps} ({:.0}%), {elapsed:.0} s elapsed, about {eta:.0} s left: {what}",
            k + 1,
            100.0 * k as f64 / steps as f64
        );
    };

    // the coplanar line: n and Z₀ from the capacitance with the substrate and without
    let cpw = Cpw::ghione_naldi();
    let mut lines = Vec::new();
    for (k, &smallest) in LINE_SMALLEST.iter().enumerate() {
        progress(k, &format!("the line, {smallest} µm"));
        let t = Instant::now();
        let (y, z) = cpw
            .axes(&cpw::Mesh {
                smallest,
                growth: GROWTH,
                largest: LINE_LARGEST,
            })
            .expect("a valid mesh");
        let with = cpw.elmer(&y, &z, cpw.permittivity).expect("the case");
        let cells = with.mesh.cells();
        let c = capacitance(&program, &with, &folder.join(format!("cpw-{smallest}")));
        let without = cpw.elmer(&y, &z, 1.0).expect("the case");
        let c0 = capacitance(
            &program,
            &without,
            &folder.join(format!("cpw-{smallest}-air")),
        );
        let elmer = ((c / c0).sqrt(), 1.0 / (cpw::C * (c * c0).sqrt()));
        let ours = cpw.static_line(&y, &z).expect("the static solve");
        eprintln!(
            "  Elmer n {:.6} Z {:.4}; photonoxide on the same lines n {:.6} Z {:.4}",
            elmer.0, elmer.1, ours.0, ours.1
        );
        lines.push(LineRow {
            smallest,
            cells,
            seconds: t.elapsed().as_secs_f64(),
            elmer,
            ours,
        });
    }
    let line_converged: Vec<(f64, f64)> = LINE_CONVERGED
        .iter()
        .map(|&smallest| {
            let (y, z) = cpw
                .axes(&cpw::Mesh {
                    smallest,
                    growth: LINE_CONVERGED_GROWTH,
                    largest: LINE_CONVERGED_LARGEST,
                })
                .expect("a valid mesh");
            cpw.static_line(&y, &z).expect("the static solve")
        })
        .collect();

    // the heater: the peak rise
    let heater = Heater::oxide_on_silicon();
    let mut heats = Vec::new();
    for (k, &smallest) in HEATER_SMALLEST.iter().enumerate() {
        progress(
            LINE_SMALLEST.len() + k,
            &format!("the heater, {smallest} µm"),
        );
        let t = Instant::now();
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
        let ours = heater.photonoxide(&x, &y).expect("the heat solve");
        eprintln!("  Elmer {elmer:.6} K; photonoxide on the same lines {ours:.6} K");
        heats.push(HeatRow {
            smallest,
            cells,
            seconds: t.elapsed().as_secs_f64(),
            elmer,
            ours,
        });
    }
    let heat_converged: Vec<f64> = HEATER_CONVERGED
        .iter()
        .map(|&smallest| {
            let (x, y) = heater
                .axes(&heater::Mesh {
                    smallest,
                    growth: HEATER_CONVERGED_GROWTH,
                    largest: HEATER_CONVERGED_LARGEST,
                })
                .expect("a valid mesh");
            heater.photonoxide(&x, &y).expect("the heat solve")
        })
        .collect();
    eprintln!("[elmer] done in {:.0} s", started.elapsed().as_secs_f64());

    let col = |f: &dyn Fn(&LineRow) -> f64| lines.iter().map(f).collect::<Vec<_>>();
    let (el_n, el_np) = extrapolated(&col(&|r| r.elmer.0));
    let (el_z, el_zp) = extrapolated(&col(&|r| r.elmer.1));
    let (px_n, px_np) = extrapolated(&col(&|r| r.ours.0));
    let (px_z, px_zp) = extrapolated(&col(&|r| r.ours.1));
    let (cv_n, _) = extrapolated(&line_converged.iter().map(|c| c.0).collect::<Vec<_>>());
    let (cv_z, cv_zp) = extrapolated(&line_converged.iter().map(|c| c.1).collect::<Vec<_>>());
    let line_finest = line_converged[line_converged.len() - 1];
    let (el_t, el_tp) = extrapolated(&heats.iter().map(|r| r.elmer).collect::<Vec<_>>());
    let (px_t, px_tp) = extrapolated(&heats.iter().map(|r| r.ours).collect::<Vec<_>>());
    let (cv_t, cv_tp) = extrapolated(&heat_converged);
    let heat_finest = heat_converged[heat_converged.len() - 1];

    let index_error = (el_n - line_finest.0) / line_finest.0;
    let impedance_error = (el_z - line_finest.1) / line_finest.1;
    let rise_error = (el_t - heat_finest) / heat_finest;

    let mut r = String::new();
    let _ = writeln!(r, "# Validation report: Elmer\n");
    let _ = writeln!(
        r,
        "Written by `just elmer` where Elmer is installed (tachsin's machine); don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. How Elmer is found and run, and its licence: [cross-code references](cross-code.md).\n"
    );
    let _ = writeln!(
        r,
        "Elmer {} (ElmerSolver, found through {}), its direct solver (UMFPACK); photonoxide {}. Both codes take the same mesh lines: Elmer's nodes are photonoxide's cell faces, Elmer's bilinear quadrilaterals against photonoxide's cell-centred finite volumes.\n",
        program.version(),
        program.found(),
        photonoxide::VERSION
    );
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
        "| Smallest cell | Cells | Time | Elmer: n, Z₀ (Ω) | photonoxide, the same lines: n, Z₀ (Ω) |\n|---:|---:|---:|---|---|"
    );
    for row in &lines {
        let _ = writeln!(
            r,
            "| {} µm | {} | {:.1} s | {:.6}, {:.4} | {:.6}, {:.4} |",
            row.smallest, row.cells, row.seconds, row.elmer.0, row.elmer.1, row.ours.0, row.ours.1
        );
    }
    let _ = writeln!(
        r,
        "\n| | n | Z₀, Ω | From |\n|---|---:|---:|---|\n| Elmer, extrapolated | {el_n:.6} | {el_z:.4} | the three finest meshes; n {}, Z₀ {} |\n| photonoxide on the same lines, extrapolated | {px_n:.6} | {px_z:.4} | the same; n {}, Z₀ {} |\n| photonoxide, converged | {:.6} | {:.4} | graded from {} µm by {LINE_CONVERGED_GROWTH} (extrapolated n {cv_n:.6}, Z₀ {cv_z:.4}, {}) |\n",
        order(el_np),
        order(el_zp),
        order(px_np),
        order(px_zp),
        line_finest.0,
        line_finest.1,
        LINE_CONVERGED[LINE_CONVERGED.len() - 1],
        order(cv_zp)
    );
    let _ = writeln!(
        r,
        "Elmer against photonoxide converged: n {index_error:+.2e}, Z₀ {impedance_error:+.2e} (relative; tolerances {INDEX_TOLERANCE:e} and {IMPEDANCE_TOLERANCE:e}).\n"
    );
    let _ = writeln!(r, "## A heater over silicon: HeatSolver's peak rise\n");
    let _ = writeln!(
        r,
        "A heater 2 µm wide and 0.2 µm thick, its centre 2 µm up, dissipating 1 W/m in 3 µm of oxide (k = 1.38 W/(m·K)) on 10 µm of silicon (k = 148 W/(m·K)) whose bottom is held at 0, the other sides insulated; half of it beside the mirror plane through the heater, in a window 30 µm wide. Elmer's HeatSolver with the heater's power as a heat source, its peak temperature by SaveScalars; photonoxide's heat solver's peak. Cells from the smallest at the heater's edges and the oxide's interface, growing by {GROWTH} to {HEATER_LARGEST} µm.\n"
    );
    let _ = writeln!(
        r,
        "| Smallest cell | Cells | Time | Elmer: peak rise (K) | photonoxide, the same lines (K) |\n|---:|---:|---:|---:|---:|"
    );
    for row in &heats {
        let _ = writeln!(
            r,
            "| {} µm | {} | {:.1} s | {:.6} | {:.6} |",
            row.smallest, row.cells, row.seconds, row.elmer, row.ours
        );
    }
    let _ = writeln!(
        r,
        "\n| | Peak rise, K | From |\n|---|---:|---|\n| Elmer, extrapolated | {el_t:.6} | the three finest meshes, {} |\n| photonoxide on the same lines, extrapolated | {px_t:.6} | the same, {} |\n| photonoxide, converged | {heat_finest:.6} | graded from {} µm by {HEATER_CONVERGED_GROWTH} (extrapolated {cv_t:.6}, {}) |\n",
        order(el_tp),
        order(px_tp),
        HEATER_CONVERGED[HEATER_CONVERGED.len() - 1],
        order(cv_tp)
    );
    let _ = writeln!(
        r,
        "Elmer against photonoxide converged: {rise_error:+.2e} (relative; tolerance {RISE_TOLERANCE:e})."
    );
    print!("{r}");
    std::fs::write(REPORT, &r).expect("docs/validation-elmer.md");

    assert!(index_error.abs() <= INDEX_TOLERANCE, "n: {index_error:.2e}");
    assert!(
        impedance_error.abs() <= IMPEDANCE_TOLERANCE,
        "Z₀: {impedance_error:.2e}"
    );
    assert!(
        rise_error.abs() <= RISE_TOLERANCE,
        "the rise: {rise_error:.2e}"
    );
}

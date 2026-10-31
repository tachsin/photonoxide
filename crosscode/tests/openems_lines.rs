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
//! (`microstrip`, `zhang`, `lossy`, `rails`, comma-separated), only those run, and the report is printed, not written.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use photonoxide::electro::Axis;
use photonoxide_crosscode::cpw::{FREQUENCIES_GHZ, LENGTH, Measured, Mesh, measured};
use photonoxide_crosscode::lossy_line::{
    FREQUENCIES_GHZ as LOSSY_FREQUENCIES_GHZ, Figures, LossyLine, SILICA,
    measured as lossy_measured,
};
use photonoxide_crosscode::microstrip::Microstrip;
use photonoxide_crosscode::openems::Model;
use photonoxide_crosscode::openems::{OpenEms, Run};
use photonoxide_crosscode::report::{percent, signed_percent};
use photonoxide_crosscode::t_rails::{
    Extent, LEAD_CELLS, LEAD_STEP, LENGTH as RAILS_LENGTH, OXIDE, PERIODS as RAILS_PERIODS, QUARTZ,
    SPACING as RAILS_SPACING, TRails, measured as rails_measured,
};
use photonoxide_crosscode::zhang_line::{
    CLADDING, LN as ZHANG_LN, OXIDE as ZHANG_OXIDE, SILICON, ZhangLine,
};

/// The microstrip's meshes' smallest cells, µm, each half the last: three meshes, the fewest
/// an extrapolation takes.
const STRIP_SMALLEST: [f64; 3] = [2.0, 1.0, 0.5];
/// Their growth and largest cell.
const GROWTH: f64 = 1.1;
const LARGEST: f64 = 10.0;
/// Zhang et al.'s line's meshes, three as the microstrip's, and their largest cell.
const ZHANG_SMALLEST: [f64; 3] = [2.0, 1.0, 0.5];
const ZHANG_LARGEST: f64 = 25.0;
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

/// The lossy line's meshes: the smallest cell at the copper's faces two skin depths at 300 GHz
/// (0.12 µm), then one, a half and a quarter; growing by [`GROWTH`] to 5 µm. The extrapolation
/// takes the last three: on the coarsest the skin is two cells deep, and the loss's error isn't
/// yet in its asymptotic range.
const LOSSY_SMALLEST: [f64; 4] = [0.24, 0.12, 0.06, 0.03];
const LOSSY_LARGEST: f64 = 5.0;
/// The lossy line's end criterion: the energy down by 55 dB. Its last dB come slowly (from 50
/// to 60 dB takes the coarsest mesh as many steps as the first 50), and 60 dB would take the
/// finest more than six hours. The planes' voltages have rung down by more than 90 dB by then
/// (the study checks 80), and on the coarsest mesh the signals cut there give the loss within
/// 0.06 % of the 60 dB run's, n and Re Z₀ within 0.005 %.
const LOSSY_END: f64 = 3.162_277_660_168_379_5e-6;
/// photonoxide's full-wave converged study on the lossy line.
const LOSSY_CONVERGED: [f64; 3] = [0.03, 0.015, 0.0075];
const LOSSY_CONVERGED_GROWTH: f64 = 1.05;
const LOSSY_CONVERGED_LARGEST: f64 = 1.0;
/// openEMS extrapolated against photonoxide's full-wave mode converged, relative: the index
/// and Re Z₀ as the quasi-static lines' (0.5 %), the attenuation 2 %: the plan for #536's
/// lossy line, the loss converging more slowly than the index as the current crowds into the
/// copper's skin and corners.
const LOSS_TOLERANCE: f64 = 2e-2;

/// The T-rail line's meshes across and in height, as the microstrip's (2, 1 and 0.5 µm), the
/// cells growing by 1.2 to 25 µm; along the line, from twice the smallest at the rails' ends
/// and the stems' edges, growing by 1.3 to 5 µm. A fourth, 0.25 µm, is left: 11.8 million cells
/// for the half, its time step 0.26 fs, some 370 000 steps to settle, more than six hours here
/// with other work sharing the computer.
const RAILS_SMALLEST: [f64; 3] = [2.0, 1.0, 0.5];
const RAILS_GROWTH: f64 = 1.2;
const RAILS_ALONG_GROWTH: f64 = 1.3;
const RAILS_ALONG_LARGEST: f64 = 5.0;

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

/// How the run ended. Nothing to say where its energy fell by 70 dB, the coplanar line's end
/// criterion. A run that ended otherwise, its energy settled ([`OpenEms::with_settled_stop`]) or
/// a looser criterion (`loose`), must show that the first plane's voltage has rung down at least
/// as far as the 70 dB runs' do (78 dB on the microstrip's coarsest mesh; 80 dB asked, its last
/// tenth's largest against its peak): its level and that ring-down are said.
fn stopped(run: &Run, label: &str, loose: bool) -> String {
    if !run.decayed && run.settled.is_none() {
        panic!("{label}: the energy neither fell to the end criterion nor settled");
    }
    if run.decayed && !loose {
        return String::new();
    }
    let v = run.probe("v1").expect("openEMS's probe");
    let peak = v.value.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    let tail = v.value[v.value.len() * 9 / 10..]
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    let down = 20.0 * (tail / peak).log10();
    assert!(
        down < -80.0,
        "{label}: its voltage is only {down:.0} dB down at the end"
    );
    match run.settled {
        Some(db) => format!(
            "; its energy settled at {db:.1} dB, the voltage {:.0} dB down",
            -down
        ),
        None => format!("; the voltage {:.0} dB down", -down),
    }
}

/// One mesh's runs.
struct Row {
    smallest: f64,
    cells: usize,
    steps: u64,
    seconds: f64,
    /// How the run ended, where its energy settled rather than fell.
    stop: String,
    lines: Vec<Measured>,
    index: f64,
    ohm: f64,
    ours: (f64, f64),
}

/// A check: what, openEMS's error against photonoxide converged, the tolerance.
type Check = (String, f64, f64);

/// A line's n and Z₀ (Ω): photonoxide's converged, the reference, and openEMS's extrapolated.
#[derive(Clone, Copy)]
struct Limits {
    reference: (f64, f64),
    openems: (f64, f64),
}

/// The microstrip's closed form: its row of the results' table, and what it says beside the
/// codes. Hammerstad and Jensen's line is in the open, so photonoxide's converged line in a box
/// twenty times as wide and as high, nearly open, is what the paper's stated accuracy holds it
/// to: 0.1 % on n (their 0.2 % on ε_e, under its square root) and 0.11 % on Z₀ (their 0.01 % on
/// Z₀₁ with that); the box's own effect is the rest.
fn hammerstad_jensen(strip: &Microstrip, limits: Limits) -> (String, String) {
    let (n, z) = strip.hammerstad_jensen();
    let open = strip.widened(20.0);
    let (y, h) = open
        .axes(&Mesh {
            smallest: CONVERGED[CONVERGED.len() - 1],
            growth: CONVERGED_GROWTH,
            largest: 50.0,
        })
        .expect("a valid mesh");
    let (on, oz) = open.static_line(&y, &h).expect("the static solve");
    let (rn, rz) = limits.reference;
    let (en, ez) = limits.openems;
    let within = |a: f64, b: f64, tolerance: f64| {
        if ((a - b) / b).abs() <= tolerance {
            "within"
        } else {
            "outside"
        }
    };
    let verdict = match (within(on, n, 1e-3), within(oz, z, 1.1e-3)) {
        ("within", "within") => "both within the paper's stated accuracy".to_owned(),
        (a, b) => format!("{a} the paper's stated accuracy on n and {b} it on Z₀"),
    };
    let row = format!(
        "| Hammerstad and Jensen (1980), closed form, the line in the open | {} | {} | {} | {} |\n",
        sig(n, 6),
        percent(n, rn),
        sig(z, 4),
        percent(z, rz)
    );
    let text = format!(
        "Hammerstad and Jensen's closed form (their Eqs. 1–5, the strip of zero thickness, at zero frequency; IEEE MTT-S Digest 1980, p. 407) is for the line in the open, and their stated accuracy is 0.2 % on ε_e (0.1 % on n) and 0.01 % on Z₀₁ (0.11 % on Z₀ with ε_e's). In the box both codes meet each other, not the open line: the box's walls, ten substrate heights away, add a little capacitance. photonoxide's converged line in a box twenty times as wide and as high (±{} µm, graded as the reference out to 50 µm cells) gives n {} and Z₀ {} Ω, against the closed form's {} and {} Ω: {} on n and {} on Z₀, {}. In the box, photonoxide's converged line is {} on n and {} on Z₀ from the closed form, openEMS's extrapolated {} and {}: the box's effect, which the twenty-times box shows is nearly all of it.\n",
        open.reach,
        sig(on, 6),
        sig(oz, 4),
        sig(n, 6),
        sig(z, 4),
        percent(on, n),
        percent(oz, z),
        verdict,
        percent(rn, n),
        percent(rz, z),
        percent(en, n),
        percent(ez, z)
    );
    (row, text)
}

/// A line both codes solve: its name, its report's heading and description, its meshes, and
/// its mesh lines, openEMS model and static line on them.
struct Case<'a> {
    key: &'static str,
    name: &'static str,
    heading: &'static str,
    about: String,
    smallest: &'a [f64],
    largest: f64,
    axes: &'a dyn Fn(&Mesh) -> (Axis, Axis),
    model: &'a dyn Fn(&Axis, &Axis) -> Model,
    statics: &'a dyn Fn(&Axis, &Axis) -> (f64, f64),
    /// The closed form's row of the results' table, if it has one.
    closed_form: Option<&'a dyn Fn(Limits) -> (String, String)>,
}

fn study(program: &OpenEms, folder: &Path, started: Instant, case: &Case) -> (String, Vec<Check>) {
    let mut rows = Vec::new();
    let total = case.smallest.len();
    for (k, &smallest) in case.smallest.iter().enumerate() {
        let mesh = Mesh {
            smallest,
            growth: GROWTH,
            largest: case.largest,
        };
        let (y, z) = (case.axes)(&mesh);
        let model = (case.model)(&y, &z);
        let elapsed = started.elapsed().as_secs_f64();
        eprintln!(
            "[openems-lines] step {}/{total} ({:.0}%), {elapsed:.0} s elapsed: {}, {smallest} µm, {} cells",
            k + 1,
            100.0 * k as f64 / total as f64,
            case.name,
            model.cells()
        );
        let label = format!("{} {}/{total}, {smallest} µm", case.key, k + 1);
        let run = program
            .run(
                &model,
                &folder.join(format!("{}-{smallest}", case.key)),
                &label,
            )
            .expect("openEMS's run");
        let stop = stopped(&run, &label, false);
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
        let ours = (case.statics)(&y, &z);
        eprintln!(
            "  openEMS n {index:.6} Z {ohm:.4}; photonoxide on the same lines n {:.6} Z {:.4}",
            ours.0, ours.1
        );
        rows.push(Row {
            smallest,
            cells: model.cells(),
            steps: run.steps,
            seconds: run.seconds,
            stop,
            lines,
            index,
            ohm,
            ours,
        });
    }
    let converged: Vec<(f64, f64)> = CONVERGED
        .iter()
        .map(|&smallest| {
            let (y, z) = (case.axes)(&Mesh {
                smallest,
                growth: CONVERGED_GROWTH,
                largest: CONVERGED_LARGEST,
            });
            (case.statics)(&y, &z)
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
    let closed = case
        .closed_form
        .map_or((String::new(), String::new()), |f| {
            f(Limits {
                reference: (ref_n, ref_z),
                openems: (oe_n, oe_z),
            })
        });

    let mut out = String::new();
    let _ = writeln!(out, "## {}\n\n{}\n", case.heading, case.about);
    let _ = writeln!(
        out,
        "| | n | Error | Z₀, Ω | Error |\n|---|---:|---:|---:|---:|\n\
         | photonoxide, converged (the reference) | {} | — | {} | — |\n\
         | openEMS, extrapolated from its three meshes | {} | {} | {} | {} |\n\
         | photonoxide on the same lines, extrapolated | {} | {} | {} | {} |\n\
         | openEMS on its finest mesh ({} µm) | {} | {} | {} | {} |\n\
         | photonoxide on the same mesh | {} | {} | {} | {} |\n{}",
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
        percent(last.ours.1, ref_z),
        closed.0
    );
    out.push_str(&closed.1);
    let _ = writeln!(
        out,
        "| Smallest cell | openEMS's cells | Steps (its run) | n at {} GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |\n|---:|---:|---:|---|---|---|---|---|---|",
        FREQUENCIES_GHZ.map(|f| format!("{f:.0}")).join(", ")
    );
    for r in &rows {
        let _ = writeln!(
            out,
            "| {} µm | {} | {} ({:.0} min{}) | {} | {} | {}, {} | {}, {} | {}, {} | {}, {} |",
            r.smallest,
            r.cells,
            r.steps,
            r.seconds / 60.0,
            r.stop,
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
    let weak: Vec<String> = [
        ("openEMS's n", oe_np),
        ("openEMS's Z₀", oe_zp),
        ("photonoxide's n on the same lines", px_np),
        ("photonoxide's Z₀ on the same lines", px_zp),
    ]
    .iter()
    .filter(|(_, p)| *p < 0.5)
    .map(|(what, p)| format!("{what} (order {p:.2})"))
    .collect();
    if !weak.is_empty() {
        let _ = writeln!(
            out,
            "An extrapolation of order below 0.5 says little: its three meshes' differences barely shrink, and the finest mesh is the better guide. Here: {}.\n",
            weak.join(", ")
        );
    }
    let checks = vec![
        (
            format!("{}: n", case.name),
            (oe_n - ref_n) / ref_n,
            INDEX_TOLERANCE,
        ),
        (
            format!("{}: Z₀", case.name),
            (oe_z - ref_z) / ref_z,
            IMPEDANCE_TOLERANCE,
        ),
    ];
    (out, checks)
}

/// A mesh of the lossy line: its smallest cell, openEMS's cells, steps and minutes, its figures and photonoxide's on the same lines.
type LossyRow = (f64, usize, u64, f64, String, Vec<Figures>, Vec<Figures>);

/// The lossy line: openEMS with the copper a conductor against photonoxide's full-wave mode.
fn lossy(program: &OpenEms, folder: &Path, started: Instant) -> (String, Vec<Check>) {
    let line = LossyLine::copper_on_silica();
    let axes = |smallest: f64, growth: f64, largest: f64| {
        line.axes(&Mesh {
            smallest,
            growth,
            largest,
        })
        .expect("a valid mesh")
    };
    let total = LOSSY_SMALLEST.len();
    // each mesh: its cells, steps, minutes, openEMS's figures and photonoxide's on its lines
    let mut rows: Vec<LossyRow> = Vec::new();
    for (k, &smallest) in LOSSY_SMALLEST.iter().enumerate() {
        let (y, z) = axes(smallest, GROWTH, LOSSY_LARGEST);
        let model = line.model(&y, &z, LOSSY_END);
        eprintln!(
            "[openems-lines] step {}/{total} ({:.0}%), {:.0} s elapsed: the lossy line, {smallest} µm, {} cells",
            k + 1,
            100.0 * k as f64 / total as f64,
            started.elapsed().as_secs_f64(),
            model.cells()
        );
        let label = format!("lossy {}/{total}, {smallest} µm", k + 1);
        let run = program
            .run(&model, &folder.join(format!("lossy-{smallest}")), &label)
            .expect("openEMS's run");
        let stop = stopped(&run, &label, true);
        let theirs = lossy_measured(&run).expect("openEMS's probes");
        let ours = line.photonoxide(&y, &z).expect("the full-wave mode");
        eprintln!("  openEMS {theirs:?}\n  photonoxide on the same lines {ours:?}");
        rows.push((
            smallest,
            model.cells(),
            run.steps,
            run.seconds / 60.0,
            stop,
            theirs,
            ours,
        ));
    }
    eprintln!("[openems-lines] the lossy line, photonoxide converged");
    let converged: Vec<Vec<Figures>> = LOSSY_CONVERGED
        .iter()
        .map(|&smallest| {
            let (y, z) = axes(smallest, LOSSY_CONVERGED_GROWTH, LOSSY_CONVERGED_LARGEST);
            let f = line.photonoxide(&y, &z).expect("the full-wave mode");
            eprintln!("  photonoxide, {smallest} µm: {f:?}");
            f
        })
        .collect();
    type Pick = fn(&Figures) -> f64;
    let quantities: [(&str, Pick, usize, f64); 3] = [
        ("n", |f| f.index, 5, INDEX_TOLERANCE),
        ("Re Z₀, Ω", |f| f.ohm, 3, IMPEDANCE_TOLERANCE),
        ("α, dB/cm", Figures::db_per_cm, 3, LOSS_TOLERANCE),
    ];
    let mut out = format!(
        "## A lossy line to 300 GHz\n\n\
         A coplanar line of copper (5.8 × 10⁷ S/m) {} µm thick, about four skin depths at 300 \
         GHz (0.12 µm), the signal {} µm wide, gaps of {} µm, grounds {} µm wide, on {} µm of \
         fused silica (ε {SILICA}) whose underside is the floor of a perfectly conducting box, \
         the grounds running to its sides at ±{} µm (so that they are the box's potential, and \
         no static charge is left on them), its lid {} µm above the substrate: small enough that \
         the box's own modes start above 300 GHz. Where the quasi-TEM line no longer holds, photonoxide's full-wave \
         RF mode (the cross-section as a waveguide at the frequency, the copper a lossy medium \
         whose skin the grid resolves; half the line beside the mirror through the signal, its \
         even mode) against openEMS with the copper a conductor (its Kappa) whose skin the mesh \
         resolves. openEMS runs a section 1.2 mm long in 10 µm cells (25 to the guided \
         wavelength at 300 GHz), 8-cell PMLs at both ends, driven across both gaps by a Gaussian \
         pulse of 25 to 325 GHz until the energy is down by 55 dB (its slow last dB would take \
         hours more on the finest mesh; the planes' voltages have rung down by more than 80 dB \
         by then, which the study checks, and on the coarsest mesh the signals cut there give \
         the loss within 0.06 % of a run to 60 dB); the voltage across a gap at \
         three planes 150 µm apart (a quarter of the guided wavelength at 300 GHz, so that the \
         three-plane wavenumber's principal value is the right one) and the current around the \
         signal midway give k = β + iα and Z₀. Both take the same mesh lines across, the \
         smallest cells at the copper's faces two skin depths (at 300 GHz), one, a half and a \
         quarter, growing by {GROWTH} to {LOSSY_LARGEST} µm, and the extrapolation takes the \
         three finest (on the coarsest the skin is two cells deep, and the loss's error isn't \
         yet in its asymptotic range); photonoxide converged graded from {} µm by \
         {LOSSY_CONVERGED_GROWTH} to {LOSSY_CONVERGED_LARGEST} µm. The index βc/ω, Re Z₀ and the \
         attenuation α at {} GHz.\n\n",
        line.metal,
        2.0 * line.signal,
        line.gap,
        line.ground,
        line.substrate,
        line.reach(),
        line.lid,
        LOSSY_CONVERGED[LOSSY_CONVERGED.len() - 1],
        LOSSY_FREQUENCIES_GHZ.map(|f| format!("{f:.0}")).join(", ")
    );
    let _ = writeln!(
        out,
        "| Frequency | Quantity | photonoxide, converged (the reference) | openEMS, extrapolated | Error | photonoxide on the same lines, extrapolated | Error | openEMS, finest mesh | Error | openEMS against photonoxide on that mesh |\n|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    let mut checks = Vec::new();
    let mut same_worst: f64 = 0.0;
    for (j, f) in LOSSY_FREQUENCIES_GHZ.iter().enumerate() {
        for (name, pick, digits, tolerance) in quantities {
            let theirs: Vec<f64> = rows.iter().map(|r| pick(&r.5[j])).collect();
            let ours: Vec<f64> = rows.iter().map(|r| pick(&r.6[j])).collect();
            let (oe, oe_p) = extrapolated(&theirs);
            let (px, px_p) = extrapolated(&ours);
            let reference = pick(&converged[converged.len() - 1][j]);
            let last = theirs[theirs.len() - 1];
            let same = ours[ours.len() - 1];
            let _ = writeln!(
                out,
                "| {f:.0} GHz | {name} | {} | {} ({}) | {} | {} ({}) | {} | {} | {} | {} |",
                sig(reference, digits),
                sig(oe, digits),
                order(oe_p),
                percent(oe, reference),
                sig(px, digits),
                order(px_p),
                percent(px, reference),
                sig(last, digits),
                percent(last, reference),
                percent(last, same)
            );
            for (a, b) in theirs.iter().zip(&ours) {
                same_worst = same_worst.max(((a - b) / b).abs());
            }
            checks.push((
                format!("the lossy line at {f:.0} GHz: {name}"),
                (oe - reference) / reference,
                tolerance,
            ));
        }
    }
    // how far the loss still moves between the two finest meshes, at its worst
    let moving = (0..LOSSY_FREQUENCIES_GHZ.len())
        .map(|j| {
            let a = rows[rows.len() - 1].5[j].alpha;
            let b = rows[rows.len() - 2].5[j].alpha;
            ((a - b) / a).abs()
        })
        .fold(0.0f64, f64::max);
    let worst_loss = checks
        .iter()
        .filter(|(what, ..)| what.ends_with("dB/cm"))
        .fold(0.0f64, |m, (_, e, _)| m.max(e.abs()));
    let verdict = if worst_loss <= LOSS_TOLERANCE {
        format!(
            "extrapolated from the three finest meshes, openEMS's loss meets photonoxide's converged loss within {:.2} % at every frequency (tolerance {:.0} %)",
            100.0 * worst_loss,
            100.0 * LOSS_TOLERANCE
        )
    } else {
        format!(
            "extrapolated from the three finest meshes, openEMS's loss is still {:.2} % from photonoxide's converged loss at its worst (tolerance {:.0} %)",
            100.0 * worst_loss,
            100.0 * LOSS_TOLERANCE
        )
    };
    let _ = writeln!(
        out,
        "\nOn every mesh openEMS and photonoxide agree within {:.2} % on every figure (the last column, for the finest): mesh by mesh the two codes give the same line. The loss converges more slowly than n and Re Z₀, the current crowding into the copper's skin, a few cells deep: it moves by up to {:.1} % between the two finest meshes. With a quarter of the skin depth at 300 GHz as the finest cell, {verdict}.",
        100.0 * same_worst,
        100.0 * moving
    );
    let _ = writeln!(
        out,
        "\n| Smallest cell | openEMS's cells | Steps (its run) | openEMS: n, Re Z₀ (Ω), α (dB/cm) at {} GHz | photonoxide, the same lines |\n|---:|---:|---:|---|---|",
        LOSSY_FREQUENCIES_GHZ.map(|f| format!("{f:.0}")).join(", ")
    );
    let shown = |v: &[Figures]| {
        v.iter()
            .map(|f| {
                format!(
                    "{}, {}, {}",
                    sig(f.index, 5),
                    sig(f.ohm, 3),
                    sig(f.db_per_cm(), 3)
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    for (smallest, cells, steps, minutes, stop, theirs, ours) in &rows {
        let _ = writeln!(
            out,
            "| {smallest} µm | {cells} | {steps} ({minutes:.0} min{stop}) | {} | {} |",
            shown(theirs),
            shown(ours)
        );
    }
    let _ = writeln!(
        out,
        "\nphotonoxide converged, graded from {} µm: {}.\n",
        LOSSY_CONVERGED.map(|s| s.to_string()).join(", "),
        converged
            .iter()
            .map(|c| shown(c))
            .collect::<Vec<_>>()
            .join(" | ")
    );
    (out, checks)
}

/// openEMS's n and Z₀ (Ω) at 0 GHz from a T-rail run of `extent` on the mesh lines `across`:
/// the limits of v(f) = v₀ + a f² over [`FREQUENCIES_GHZ`], and each frequency's figures.
fn rails_figures(run: &Run, extent: Extent, across: &Axis) -> (f64, f64, Vec<Measured>) {
    let lines = rails_measured(run, &FREQUENCIES_GHZ, extent, across).expect("openEMS's probes");
    let fit = |f: fn(&Measured) -> f64| {
        at_zero(
            &lines
                .iter()
                .map(|m| (m.frequency_ghz, f(m)))
                .collect::<Vec<_>>(),
        )
    };
    (fit(Measured::index), fit(Measured::ohm), lines)
}

/// Kharel et al.'s T-rail electrodes: openEMS's 3D FDTD on the half line beside a magnetic wall
/// (checked against the whole line on the coarsest mesh) against photonoxide's periodic line.
fn rails(program: &OpenEms, folder: &Path, started: Instant) -> (String, Vec<Check>) {
    let line = TRails::kharel_d();
    let axes = |smallest: f64, growth: f64, largest: f64| {
        line.axes(
            &Mesh {
                smallest,
                growth,
                largest,
            },
            &Mesh {
                smallest: 2.0 * smallest,
                growth: RAILS_ALONG_GROWTH,
                largest: RAILS_ALONG_LARGEST,
            },
        )
        .expect("a valid mesh")
    };
    let total = RAILS_SMALLEST.len() + 1;
    let mut step = 0;
    let mut run_one = |smallest: f64, extent: Extent| {
        let (y, z, x) = axes(smallest, RAILS_GROWTH, ZHANG_LARGEST);
        let model = line.model(&y, &z, &x, END, extent);
        step += 1;
        let which = match extent {
            Extent::Whole => "whole",
            Extent::Half => "half",
        };
        eprintln!(
            "[openems-lines] step {step}/{total} ({:.0}%), {:.0} s elapsed: the T-rail line, its {which}, {smallest} µm, {} cells",
            100.0 * (step - 1) as f64 / total as f64,
            started.elapsed().as_secs_f64(),
            model.cells()
        );
        let label = format!("rails {step}/{total}, {which}, {smallest} µm");
        let run = program
            .run(
                &model,
                &folder.join(format!("rails-{which}-{smallest}")),
                &label,
            )
            .expect("openEMS's run");
        let stop = stopped(&run, &label, false);
        let (n, ohm, lines) = rails_figures(&run, extent, &y);
        eprintln!("  openEMS n {n:.6} Z {ohm:.4}");
        (model.cells(), run, stop, n, ohm, lines, (y, z))
    };
    // the magnetic wall against the whole line, on the coarsest mesh
    let whole = run_one(RAILS_SMALLEST[0], Extent::Whole);
    let mut rows = Vec::new();
    for &smallest in &RAILS_SMALLEST {
        let (cells, run, stop, n, ohm, lines, (y, z)) = run_one(smallest, Extent::Half);
        let ours = line.bloch(&y, &z).expect("the static solves");
        eprintln!(
            "  photonoxide on the same lines n {:.6} Z {:.4}",
            ours.0, ours.1
        );
        rows.push(Row {
            smallest,
            cells,
            steps: run.steps,
            seconds: run.seconds,
            stop,
            lines,
            index: n,
            ohm,
            ours,
        });
    }
    eprintln!("[openems-lines] the T-rail line, photonoxide converged");
    let converged: Vec<(f64, f64)> = CONVERGED
        .iter()
        .map(|&smallest| {
            let (y, z, _) = axes(smallest, CONVERGED_GROWTH, CONVERGED_LARGEST);
            let c = line.bloch(&y, &z).expect("the static solves");
            eprintln!("  photonoxide, {smallest} µm: n {:.6} Z {:.4}", c.0, c.1);
            c
        })
        .collect();
    let column = |f: fn(&Row) -> f64| rows.iter().map(f).collect::<Vec<_>>();
    let (oe_n, oe_np) = extrapolated(&column(|r| r.index));
    let (oe_z, oe_zp) = extrapolated(&column(|r| r.ohm));
    let (px_n, px_np) = extrapolated(&column(|r| r.ours.0));
    let (px_z, px_zp) = extrapolated(&column(|r| r.ours.1));
    let (cv_n, cv_np) = extrapolated(&converged.iter().map(|c| c.0).collect::<Vec<_>>());
    let (cv_z, cv_zp) = extrapolated(&converged.iter().map(|c| c.1).collect::<Vec<_>>());
    let (ref_n, ref_z) = converged[converged.len() - 1];
    let bound = {
        let (y, z, _) = axes(
            CONVERGED[CONVERGED.len() - 1],
            CONVERGED_GROWTH,
            CONVERGED_LARGEST,
        );
        line.loaded_bound(&y, &z).expect("the static solves")
    };
    let last = &rows[rows.len() - 1];
    let first = &rows[0];
    let same_worst = rows
        .iter()
        .flat_map(|r| [(r.index, r.ours.0), (r.ohm, r.ours.1)])
        .fold(0.0f64, |m, (a, b)| m.max(((a - b) / b).abs()));

    let mut out = String::new();
    let _ = writeln!(
        out,
        "## Kharel et al.'s T-rail electrodes\n\n\
         Kharel et al.'s segmented electrodes on thin-film lithium niobate on quartz (Optica 8, 357 (2021), Table 1's row d and Fig. 1; the erratum, Optica 8, 1218 (2021), corrects only Fig. 4(a)): a signal {} µm wide, the rails' gap g = {} µm, each rail s = {} µm wide across, h = {} µm from its channel electrode, r = {} µm long, c = {} µm between rails along the line, each joined to its channel by a stem t = {} µm wide in its middle; the channels {} µm apart, the grounds {} µm wide (not given). The electrodes {} µm thick (gold, perfect conductors here), on a {} µm x-cut film etched to a slab of {} µm and a rib {} µm wide in the rails' gap (upright sidewalls), on {} µm of oxide (ε {OXIDE}) over {} µm of quartz (ε {QUARTZ}), under {} µm of oxide over the slab and air, in a perfectly conducting box as wide as the line (±{} µm), its lid {} µm above the film; the film's permittivity {} across and {} up and along, as Zhang et al.'s line's.\n\n\
         photonoxide's line is its model of these electrodes (the validation cases line/kharel-2021-segmented-*): a period of half the channel line's c, the line loaded by the rails r long (Shin et al. 2005, Eq. 7: the capacitance of the section through the rails, the channel line's inductance) and the other half, each section from its static solver on half the box, the stems left out, and the Bloch wave of the period's ABCD matrix (`electro::line::Periodic`) at 1 GHz, where it is the static line's. openEMS solves the electrodes as drawn, rails, stems and all, in 3D: a section {} mm long ({} periods), its mesh the same in every period, and at both ends a lead of the channel line alone ({} µm in {LEAD_STEP} µm cells) ended by Mur's absorbing condition (the pulse leaves a static field on the line, under which openEMS's PMLs at the ends grew unstable, the energy rising without end some 70 to 90 ps in), driven across the channels' gap at the middle of a gap between rails by a Gaussian pulse of 10 to 110 GHz, until the energy has settled or fallen by 70 dB; the voltage across the gap at the rib's top at three planes {} µm apart at the middles of gaps between rails (six periods, so that the Bloch wave is the same at each and midway), the current around the signal and its rails midway (the mean of the dual mesh's planes either side), give k and the Bloch Z₀ at 10 to 50 GHz, and n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². It solves half the line, beside a magnetic wall through the signal's middle (the even mode's symmetry), the current loops starting at the first two dual lines beside the wall and the current between the wall and the first added in proportion; on the coarsest mesh the whole line checks it. Across and in height both codes take the same mesh lines, from the smallest at every edge and layer growing by {RAILS_GROWTH} to {ZHANG_LARGEST} µm; along the line openEMS's cells start at twice the smallest at the rails' ends and the stems' edges and grow by {RAILS_ALONG_GROWTH} to {RAILS_ALONG_LARGEST} µm.\n",
        2.0 * line.signal,
        line.g,
        line.s,
        line.h,
        line.r,
        line.c,
        line.t,
        line.channel_gap(),
        line.ground,
        line.metal,
        line.film,
        line.slab,
        line.rib,
        line.buried,
        line.quartz,
        line.clad,
        line.reach(),
        line.lid,
        ZHANG_LN[0],
        ZHANG_LN[1],
        RAILS_LENGTH / 1000.0,
        RAILS_PERIODS,
        LEAD_STEP * LEAD_CELLS as f64,
        RAILS_SPACING,
    );
    let _ = writeln!(
        out,
        "| | n | Error | Z₀, Ω | Error |\n|---|---:|---:|---:|---:|\n\
         | photonoxide, converged (the reference) | {} | — | {} | — |\n\
         | openEMS, extrapolated from its three meshes | {} | {} | {} | {} |\n\
         | photonoxide on the same lines, extrapolated | {} | {} | {} | {} |\n\
         | openEMS on its finest mesh ({} µm) | {} | {} | {} | {} |\n\
         | photonoxide on the same mesh | {} | {} | {} | {} |\n\
         | photonoxide, converged, every gap between rails loaded as the rails (the 2D sections' bound) | {} | {} | {} | {} |\n\
         | openEMS, the whole line, coarsest mesh ({} µm) | {} | {} | {} | {} |\n\
         | openEMS, its half beside the magnetic wall, the same mesh | {} | {} | {} | {} |\n",
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
        percent(last.ours.1, ref_z),
        sig(bound.0, 6),
        percent(bound.0, ref_n),
        sig(bound.1, 4),
        percent(bound.1, ref_z),
        first.smallest,
        sig(whole.3, 6),
        percent(whole.3, ref_n),
        sig(whole.4, 4),
        percent(whole.4, ref_z),
        sig(first.index, 6),
        percent(first.index, ref_n),
        sig(first.ohm, 4),
        percent(first.ohm, ref_z),
    );
    let _ = writeln!(
        out,
        "The half beside its magnetic wall is {} on n and {} on Z₀ from the whole line on the coarsest mesh, a difference of the order of its first cell that the finer meshes shrink. Mesh by mesh openEMS and photonoxide differ by up to {:.1} % ({} on n and {} on Z₀ on the finest mesh), openEMS's n falling and photonoxide's rising as the cells shrink. Their extrapolations don't meet: openEMS's n and Z₀ give the line a capacitance {} and an inductance {} against photonoxide's converged period (C = n/(cZ₀), L = nZ₀/c). openEMS solves the electrodes in 3D; photonoxide's 2D sections leave out the rails' ends fringing across the gaps between them (which the bound above, every gap loaded as the rails, holds at most: there n {} and Z₀ {}) and the current that charges the rails through the stems, as Shin et al.'s model does.\n",
        percent(first.index, whole.3),
        percent(first.ohm, whole.4),
        100.0 * same_worst,
        percent(last.index, last.ours.0),
        percent(last.ohm, last.ours.1),
        percent(oe_n / oe_z, ref_n / ref_z),
        percent(oe_n * oe_z, ref_n * ref_z),
        percent(bound.0, ref_n),
        percent(bound.1, ref_z),
    );
    let _ = writeln!(
        out,
        "| Smallest cell | openEMS's cells (the half) | Steps (its run) | n at {} GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |\n|---:|---:|---:|---|---|---|---|---|---|",
        FREQUENCIES_GHZ.map(|f| format!("{f:.0}")).join(", ")
    );
    for r in &rows {
        let _ = writeln!(
            out,
            "| {} µm | {} | {} ({:.0} min{}) | {} | {} | {}, {} | {}, {} | {}, {} | {}, {} |",
            r.smallest,
            r.cells,
            r.steps,
            r.seconds / 60.0,
            r.stop,
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
        "\nThe whole line on the coarsest mesh: {} cells, {} steps ({:.0} min{}).\n",
        whole.0,
        whole.1.steps,
        whole.1.seconds / 60.0,
        whole.2
    );
    let _ = writeln!(
        out,
        "| | n | Z₀, Ω | From |\n|---|---:|---:|---|\n\
         | openEMS, extrapolated | {} | {} | its three meshes; n {}, Z₀ {} |\n\
         | photonoxide on the same lines, extrapolated | {} | {} | the same; n {}, Z₀ {} |\n\
         | photonoxide, converged | {} | {} | graded from {} µm by {CONVERGED_GROWTH} to {CONVERGED_LARGEST} µm (from {} µm: n {}, Z₀ {}; extrapolated n {}, {}, Z₀ {}, {}) |\n",
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
        order(cv_np),
        sig(cv_z, 4),
        order(cv_zp)
    );
    let checks = vec![
        (
            "the T-rail line: n".to_owned(),
            (oe_n - ref_n) / ref_n,
            INDEX_TOLERANCE,
        ),
        (
            "the T-rail line: Z₀".to_owned(),
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
        Ok(p) => p.with_threads(threads()).with_settled_stop(),
        Err(e) => {
            println!("skipped: {e}");
            return;
        }
    };
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("openems");
    let started = Instant::now();
    let all = ["microstrip", "zhang", "lossy", "rails"];
    let mut sections = String::new();
    let mut checks: Vec<Check> = Vec::new();
    let strip = Microstrip::alumina_like();
    let zhang = ZhangLine::zhang();
    let cases = [
        Case {
            key: "microstrip",
            name: "the microstrip",
            heading: "A microstrip line in a box",
            about: format!(
                "A perfectly conducting strip of zero thickness, {} µm wide, on {} µm of a substrate of ε = {} (w/h = 1) over a ground plane, in a perfectly conducting box ±{r} µm across and {r} µm high above the strip, its walls at the ground's potential. openEMS runs a section {} mm long as the coplanar line's (25 µm cells along it, 8-cell PMLs at both ends), driven by a soft source of E between the strip and the ground (a Gaussian pulse of 5 to 55 GHz) until the energy is down by 70 dB; k and Z₀ at each frequency come from the voltage from the strip down to the ground at three planes 1025 µm apart and the current around the strip midway (a loop ±{} µm across and ±5 µm about the strip, clear of the ground), its quasi-static n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². photonoxide's quasi-TEM line is its static solver's on half the box: n = √(C/C₀), Z₀ = 1/(c√(CC₀)). Across the line both take the same mesh lines, cells from the smallest at the strip's edge and in its plane growing by {GROWTH} to {LARGEST} µm; three meshes, the fewest an extrapolation takes.",
                2.0 * strip.half_width,
                strip.height,
                strip.permittivity,
                LENGTH / 1000.0,
                2.0 * strip.half_width,
                r = strip.reach
            ),
            smallest: &STRIP_SMALLEST,
            largest: LARGEST,
            axes: &|m| strip.axes(m).expect("a valid mesh"),
            model: &|y, z| strip.model(y, z, END),
            statics: &|y, z| strip.static_line(y, z).expect("the static solve"),
            closed_form: Some(&|limits| hammerstad_jensen(&strip, limits)),
        },
        Case {
            key: "zhang",
            name: "Zhang et al.'s line",
            heading: "Zhang et al.'s thin-film lithium niobate line, its full stack",
            about: format!(
                "Zhang et al.'s line (Photon. Res. 10, 2380 (2022), Section 2) with everything around it: a ground–signal–ground line, the signal {} µm wide, gaps of {} µm, grounds {} µm wide (not given), the electrodes {} µm thick (800 nm of copper under 30 nm of gold, perfect conductors here), on an x-cut film {} µm thick etched {} µm to a rib {} µm wide in each gap (its sidewalls upright, so that the mesh lines follow them), on {} µm of thermal oxide (ε {ZHANG_OXIDE}) over {} µm of silicon (ε {SILICON}), under {} µm of PECVD oxide (ε {CLADDING}, the paper's) and air, in a perfectly conducting box as wide as the line (±{r} µm), the grounds running to its sides so that they are its potential and no static charge is left on them, its floor under the silicon and its lid {lid} µm above the film. The film's clamped static permittivity is anisotropic: {} across (the crystal's c axis), {} up and along the line, photonoxide's a diagonal tensor and openEMS's a diagonal material. openEMS runs a section {} mm long as the coplanar line's, driven across both gaps, the voltage across one gap at the ribs' top and the current around the signal (a loop through the gaps' middles, from 1 µm down in the buried oxide to 3 µm up in the air); its n and Z₀ at 0 GHz as above. photonoxide's quasi-TEM line is its static solver's on half the box. Both take the same mesh lines, cells from the smallest at the electrodes' and the ribs' edges and at every layer, growing by {GROWTH} to {} µm (the largest cells 25 µm, the line's own cell along it, a sixty-fourth of the shortest wavelength in the silicon, to keep openEMS's three runs to a few hours); the thin layers set openEMS's time step on every mesh.",
                2.0 * zhang.signal,
                zhang.gap,
                zhang.ground,
                zhang.metal,
                zhang.film,
                zhang.film - zhang.slab,
                zhang.rib,
                zhang.buried,
                zhang.silicon,
                zhang.clad,
                ZHANG_LN[0],
                ZHANG_LN[1],
                LENGTH / 1000.0,
                ZHANG_LARGEST,
                r = zhang.reach(),
                lid = zhang.lid
            ),
            smallest: &ZHANG_SMALLEST,
            largest: ZHANG_LARGEST,
            axes: &|m| zhang.axes(m).expect("a valid mesh"),
            model: &|y, z| zhang.model(y, z, END),
            statics: &|y, z| zhang.static_line(y, z).expect("the static solve"),
            closed_form: None,
        },
    ];
    for case in &cases {
        if runs(case.key) {
            let (s, c) = study(&program, &folder, started, case);
            sections.push_str(&s);
            checks.extend(c);
        }
    }
    if runs("lossy") {
        let (s, c) = lossy(&program, &folder, started);
        sections.push_str(&s);
        checks.extend(c);
    }
    if runs("rails") {
        let (s, c) = rails(&program, &folder, started);
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

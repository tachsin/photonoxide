use super::*;

/// A straight guide through a 1 × 1 µm region on 50 nm cells, one wavelength; `extra` adds to
/// the task.
fn job(extra: &str, design: &str) -> Job {
    Job::parse(&text(extra, design)).unwrap()
}

fn text(extra: &str, design: &str) -> String {
    format!(
        r#"
name = "straight"
timeout_minutes = 5

[task]
kind = "optimize"
x_um = [-1.5, 1.5]
y_um = [-1.5, 1.5]
step_nm = 50
pml_cells = 10
core_index = 3.4
cladding_index = 1.44
source = "in"
{extra}

[[task.guide]]
x_um = [-2.0, -0.5]
y_um = [-0.25, 0.25]

[[task.guide]]
x_um = [0.5, 2.0]
y_um = [-0.25, 0.25]

[[task.port]]
name = "in"
side = "west"
at_um = -0.8
window_um = [-0.6, 0.6]

[[task.port]]
name = "out"
side = "east"
at_um = 0.8
window_um = [-0.6, 0.6]

[[task.objective]]
port = "out"
from_um = 1.55

[task.design]
x_um = [-0.5, 0.5]
y_um = [-0.5, 0.5]
filter_nm = 100
beta = [4, 8]
evaluations = [3, 3]
{design}
"#
    )
}

fn refused(extra: &str, design: &str) -> String {
    match check(&job(extra, design)) {
        Err(e) => e.to_string(),
        Ok(()) => panic!("checked: {extra} {design}"),
    }
}

#[test]
fn a_design_region_job_is_checked_and_its_mistakes_named() {
    check(&job("", "")).unwrap();
    assert!(designs(&job("", "")));
    assert!(super::super::optimizes(&job("", "")));
    assert!(refused(r#"record_every = 0"#, "").contains("record_every"));
    assert!(refused("", "rules_from = 2").contains("rules_from"));
    assert!(refused("", "linewidth_nm = 90").contains("together"));
    assert!(refused("", "robust_eta = 0.6").contains("robust_eta"));
    assert!(refused("", "start = 0.9\nasymmetry = 0.2").contains("[0, 1]"));
    assert!(refused(r#"method = "lbfgsb""#, "").contains("mma"));
    // the region off the grid's lines, a port inside it, a source nowhere or on rows
    let edited = |from: &str, to: &str| {
        let t = text("", "");
        assert!(t.contains(from), "{from}");
        check(&Job::parse(&t.replacen(from, to, 1)).unwrap())
            .unwrap_err()
            .to_string()
    };
    assert!(
        edited(
            "x_um = [-0.5, 0.5]\ny_um = [-0.5, 0.5]\nfilter",
            "x_um = [-0.52, 0.5]\ny_um = [-0.5, 0.5]\nfilter"
        )
        .contains("line of the grid")
    );
    assert!(edited("at_um = 0.8", "at_um = 0.2").contains("crosses the design region"));
    assert!(edited("source = \"in\"", "source = \"nowhere\"").contains("no port is named"));
    assert!(
        edited(
            "side = \"west\"\nat_um = -0.8",
            "side = \"south\"\nat_um = -0.8"
        )
        .contains("west or east")
    );
    assert!(edited("timeout_minutes = 5\n", "").contains("timeout_minutes"));
}

/// Each term's gradient in the densities against fourth-order central differences, and the
/// epigraph's rows: the job's gradient is the library's, through the filter and the projection.
#[test]
fn the_jobs_gradient_is_central_differences() {
    let t = Topology::of(&job("", "asymmetry = 0.2")).unwrap();
    let stage = t.stage(1).unwrap();
    let n = t.len();
    let rho: Vec<f64> = t.start();
    let (values, gradients, _) = stage.terms(&rho).unwrap();
    assert_eq!(values.len(), 1);
    let h = 1e-3;
    for p in [0, n / 2 + 7, n - 3] {
        let at = |d: f64| {
            let mut r = rho.clone();
            r[p] += d;
            stage.terms(&r).unwrap().0[0]
        };
        let fd = (8.0 * (at(h) - at(-h)) - (at(2.0 * h) - at(-2.0 * h))) / (12.0 * h);
        let exact = gradients[0][p];
        assert!(
            (exact - fd).abs() <= 1e-6 * exact.abs().max(1e-3),
            "pixel {p}: {exact} against {fd}"
        );
    }
    // the epigraph: g = f − t, and t's own gradient
    let (mut y, start) = stage.start(&rho).unwrap();
    assert_eq!(start.worst, values[0]);
    y[n] = 0.5;
    let m = stage.constraints();
    let (mut grad, mut g, mut jac) = (vec![0.0; n + 1], vec![0.0; m], vec![0.0; m * (n + 1)]);
    let point = stage.evaluate(&y, &mut grad, &mut g, &mut jac).unwrap();
    assert_eq!(point.t, 0.5);
    assert_eq!(grad[n], 1.0);
    assert_eq!(g[0], values[0] - 0.5);
    assert_eq!(jac[n], -1.0);
    assert_eq!(&jac[..n], &gradients[0][..]);
}

/// The rules' rows against central differences: the length constraints (g/ε − 1) and the areas
/// (in pixels, less a t).
#[test]
fn the_rules_rows_are_central_differences() {
    let t = Topology::of(&job(
        "",
        "asymmetry = 0.3\nlinewidth_nm = 100\nspacing_nm = 100\nisland_um2 = 0.02\nhole_um2 = 0.02",
    ))
    .unwrap();
    let stage = t.stage(1).unwrap();
    assert_eq!(stage.rule_labels().len(), 4);
    assert_eq!(t.stage(0).unwrap().rule_labels().len(), 0);
    let n = t.len();
    let mut y = t.start();
    y.push(0.3);
    let (g, rows) = stage.rule_values(&y).unwrap();
    assert_eq!(g.len(), 4);
    // the length constraints are steep at this grid (a decay of 64 (R/Δ)²): a small step
    let h = 3e-5;
    for k in 0..2 {
        for p in [3, n / 2 + 4] {
            let at = |d: f64| {
                let mut z = y.clone();
                z[p] += d;
                stage.rule_values(&z).unwrap().0[k]
            };
            let fd = (8.0 * (at(h) - at(-h)) - (at(2.0 * h) - at(-2.0 * h))) / (12.0 * h);
            let exact = rows[k * (n + 1) + p];
            // the values rounded to 12 digits: an error of |g| 1e-12 / h in the differences
            let rounding = 4.0 * g[k].abs() * 1e-12 / h;
            assert!(
                (exact - fd).abs() <= 1e-5 * exact.abs() + rounding,
                "rule {k}, pixel {p}: {exact} against {fd}"
            );
        }
    }
    // the areas' t column
    assert_eq!(rows[2 * (n + 1) + n], -AREA_SLACK * n as f64);
}

/// A bend from the west to `side` ("north" or "south"), the guide leaving along y on the side's
/// half, its port's rows at `at_um`; 50 nm cells, a plane symmetric about y = 0.
fn corner(side: &str, at_um: f64) -> Topology {
    let sign = if side == "north" { 1.0 } else { -1.0 };
    let (a, b) = if sign > 0.0 { (0.5, 2.0) } else { (-2.0, -0.5) };
    let j = Job::parse(&format!(
        r#"
name = "corner"
timeout_minutes = 5

[task]
kind = "optimize"
x_um = [-1.5, 1.0]
y_um = [-1.5, 1.5]
step_nm = 50
pml_cells = 10
core_index = 3.4
cladding_index = 1.44
source = "in"

[[task.guide]]
x_um = [-2.0, -0.5]
y_um = [-0.25, 0.25]

[[task.guide]]
x_um = [-0.25, 0.25]
y_um = [{a}, {b}]

[[task.port]]
name = "in"
side = "west"
at_um = -0.8
window_um = [-0.6, 0.6]

[[task.port]]
name = "out"
side = "{side}"
at_um = {at_um}
window_um = [-0.6, 0.6]

[[task.objective]]
port = "out"
from_um = 1.55

[[task.objective]]
port = "out"
mode = 2
from_um = 1.55

[[task.objective]]
port = "in"
from_um = 1.55

[task.design]
x_um = [-0.5, 0.5]
y_um = [-0.5, 0.5]
filter_nm = 100
beta = [8]
evaluations = [3]
"#
    ))
    .unwrap();
    Topology::of(&j).unwrap()
}

/// A south port's two rows measure what a north port's measure in the plane mirrored in y: a
/// bend to the north and its mirror image to the south give the same power in each of the turned
/// guide's modes, to the solver's rounding; and with the reflection they carry no more than the
/// source's power. North rows 46 and 47 of 60 (from y = 0.825 µm) mirror south rows 13 and
/// 12: the south port's line is row 12, at y = −0.875 µm.
#[test]
fn a_row_port_measures_a_turned_guide() {
    let north = corner("north", 0.825);
    let south = corner("south", -0.875);
    let [nx, ny] = north.shape();
    // a bend of the guide's width with its outer corner cut at 45°, and its mirror image
    let design = |s: f64| -> Vec<bool> {
        (0..nx * ny)
            .map(|p| {
                let x = -0.5 + 0.05 * ((p % nx) as f64 + 0.5);
                let y = s * (-0.5 + 0.05 * ((p / nx) as f64 + 0.5));
                ((y.abs() < 0.25 && x < 0.25) || (x.abs() < 0.25 && y > -0.25)) && x < y
            })
            .collect()
    };
    let n = north.binary(&design(1.0)).unwrap().0;
    let s = south.binary(&design(-1.0)).unwrap().0;
    for (a, b) in n.iter().zip(&s) {
        assert!(
            (a - b).abs() <= 1e-9 * a.abs().max(1e-6),
            "{n:?} against {s:?}"
        );
    }
    assert!(n.iter().sum::<f64>() <= 1.0, "{n:?}");
    assert!(north.labels()[0].contains("T(out, mode 1)"));
}

/// The record: the problem, a stage, evaluations with the design at the stride, the end.
#[test]
fn a_run_records_its_problem_evaluations_and_design() {
    let j = job("record_every = 2", "");
    let t = Topology::of(&j).unwrap();
    let mut run = Run::in_memory();
    let mut recorder = TopologyRecorder::begin(&t, &j, &mut run).unwrap();
    let stage = t.stage(0).unwrap();
    recorder.stage(&stage, &mut run).unwrap();
    let (y, point) = stage.start(&t.start()).unwrap();
    recorder
        .record(&stage, &mut run, &y[..t.len()], &point)
        .unwrap();
    recorder
        .record(&stage, &mut run, &y[..t.len()], &point)
        .unwrap();
    recorder
        .record(&stage, &mut run, &y[..t.len()], &point)
        .unwrap();
    recorder.stage_end(&stage, &mut run).unwrap();
    recorder.finish(&stage, &mut run, "evaluations").unwrap();
    let events: Vec<Event> = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let count = |f: &dyn Fn(&Event) -> bool| events.iter().filter(|e| f(e)).count();
    assert_eq!(count(&|e| matches!(e, Event::Topology { .. })), 1);
    assert_eq!(count(&|e| matches!(e, Event::TopologyEvaluation { .. })), 3);
    // evaluations 1 and 3, the stage's end, the binary design
    assert_eq!(count(&|e| matches!(e, Event::Density { .. })), 4);
    assert_eq!(count(&|e| matches!(e, Event::Field { .. })), 4);
    let Some(Event::TopologyOptimum {
        powers, length_nm, ..
    }) = events.last()
    else {
        panic!("no optimum");
    };
    assert_eq!(powers.len(), 1);
    assert!(*length_nm > 0.0);
}

/// A strip of the guide's width through the region carries its mode on: T within 10⁻⁴ of 1, the
/// region of the cladding's a gap that loses some.
#[test]
fn a_strip_through_the_region_carries_the_guides_mode() {
    let t = Topology::of(&job("", "")).unwrap();
    let [nx, ny] = t.shape();
    let strip: Vec<bool> = (0..nx * ny)
        .map(|p| (-0.5 + 0.05 * ((p / nx) as f64 + 0.5)).abs() < 0.25)
        .collect();
    let through = t.binary(&strip).unwrap().0[0];
    let gap = t.binary(&vec![false; nx * ny]).unwrap().0[0];
    assert!((through - 1.0).abs() < 1e-4, "{through}");
    assert!(gap < 0.9, "{gap}");
}

/// A stage's best: a design that meets every rule beats one that doesn't, whatever their worst
/// terms; of two that do, the less worst; of two that don't, the less violation.
#[test]
fn a_stage_keeps_a_design_that_meets_its_rules_over_one_that_doesnt() {
    let met = Rank::of(0.3, &[-0.1, 0.0]);
    let broken = Rank::of(0.01, &[0.2, -1.0]);
    assert!(met.beats(&broken));
    assert!(!broken.beats(&met));
    assert!(Rank::of(0.2, &[-0.5]).beats(&met));
    assert!(Rank::of(0.9, &[0.1, 0.05]).beats(&broken));
    assert!(broken.beats(&Rank::NONE));
    // no rules: every design meets them
    assert!(Rank::of(0.1, &[]).beats(&Rank::of(0.2, &[])));
}

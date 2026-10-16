//! What the validation report checks a device's studies on ([`super::study`]): a sweep against
//! the device evaluated point by point, the same bits on any number of threads, corners against
//! their closed form, a published sweep reproduced through the coupler device, and that sweep's
//! convergence in its grid.

use super::{Compute, Corner, Event, Study, SweepAxis, execute, study_from_device};
use crate::circuit::Fidelity;
use crate::device::{Spec, Values};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid study";

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/study-sweep-analytic",
            title: r"Devices' studies: a directional coupler swept in its gap (0.15 to 0.3 µm, 4 points) and its length (5 to 20 µm, 3 points) at its analytic fidelity, as a job, against the device evaluated at each pair of values: its figures and its S-matrices from 1.53 to 1.57 µm (5 wavelengths), the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same computation: each point is the design with the swept values written as plain numbers in place of its parameters' expressions, its component built by Device::component; the points are solved side by side and recorded in their order",
            run: sweep_analytic,
        },
        Case {
            id: "device/study-threads",
            title: r"Devices' studies: an all-pass ring's sweep in its radius (5 to 20 µm, 31 points, analytic, 5 wavelengths) and a waveguide's corners (width and thickness), each run on 1 and on 20 threads: the lines of their records that differ (count shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md): each point is computed on its own and recorded in the points' order",
            run: threads,
        },
        Case {
            id: "device/study-corners-closed-form",
            title: r"Devices' studies: a waveguide's corners at its analytic fidelity (1 mm long, 2.7 dB/cm; the length ±50 µm and the loss ±1 dB/cm, all four combinations), $\lvert S_{21} \rvert^2$ from 1.53 to 1.57 µm against $10^{-\alpha L/10}$, and the extremes at the corners the closed form puts them (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the closed form of a straight guide's transmission, $e^{i\gamma L}$ with its loss $\alpha$ in dB per length: $\lvert S_{21} \rvert^2 = 10^{-\alpha L/10}$, smallest at the longest, lossiest corner and largest at the shortest, least lossy one",
            run: corners_closed_form,
        },
        Case {
            id: "device/study-coupler-chrostowski",
            title: r"Devices' studies: a directional coupler (two 500 x 220 nm strips on 220 nm SOI, Li's silicon and Malitson's silica) swept in its gap from 150 to 300 nm (4 points) at its 3D fidelity (the supermodes on cells fitted about 10 nm), as a job: the decades its cross-over length $L_x$ grows per nm of gap, the slope of a least-squares line through $\log_{10} L_x$ at 1.55 µm (shown)",
            tier: Tier::Published,
            source: r"L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Eq. 4.14, their fit to Lumerical MODE over the gap: $L_x = 10^{0.0037645\thinspace g + 0.799434}$ µm, g in nm; the tolerance is 5 % of the slope, the fit's own departure from their text's 37.5 µm at 200 nm (it gives 35.67 µm there); the materials differ (their Lorentz fit to Palik's silicon and oxide of 1.444)",
            run: coupler_chrostowski,
        },
        Case {
            id: "device/study-coupler-grid",
            title: r"Devices' studies: the coupler's gap sweep (study-coupler-chrostowski's, its ends 150 and 300 nm) at 3D on cells fitted about 10 and 5 nm: the decades of $L_x$ per nm of gap on 10 nm cells (shown) against 5 nm cells, within 0.1 %",
            tier: Tier::Analytic,
            source: "convergence in the grid: the supermodes' splitting converges about first order at the strips' corners (device/coupler-3d-order): L_x grows about 1 % from 10 to 5 nm cells, by nearly the same factor at every gap, which cancels in the slope of log L_x",
            run: coupler_grid,
        },
    ]
}

/// The events of `job`'s run, in memory.
fn events_of(job: &Job) -> Vec<Event> {
    let mut run = Run::in_memory();
    execute(job, &mut run, &Stop::new(None)).expect(OK);
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).expect("a recorded event"))
        .collect()
}

/// The points of a study's record.
fn points(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|e| matches!(e, Event::DevicePoint { .. }))
        .collect()
}

fn band(points: usize) -> Compute {
    Compute {
        band: Some((1.53, 1.57, points)),
        ..Compute::model()
    }
}

fn sweep_analytic() -> Outcome {
    let spec = Spec::new("directional coupler");
    let study = Study::Sweep(vec![
        SweepAxis::new("gap", 0.15, 0.3, 4),
        SweepAxis::new("length", 5.0, 20.0, 3),
    ]);
    let job = study_from_device("gap-length", &spec, &band(5), &study).expect(OK);
    let events = events_of(&job);
    let device = spec.device().expect(OK);
    let wavelengths: Vec<f64> = (0..5).map(|k| 1.53 + 0.04 * f64::from(k) / 4.0).collect();
    let mut differ = 0usize;
    let mut k = 0;
    for gap in SweepAxis::new("gap", 0.15, 0.3, 4).values() {
        for length in SweepAxis::new("length", 5.0, 20.0, 3).values() {
            let values =
                Values::new(device.as_ref(), &[("gap", gap), ("length", length)]).expect(OK);
            let built = device.component(&values, Fidelity::Analytic).expect(OK);
            let Some(Event::DevicePoint { figures, s, .. }) = points(&events).get(k).copied()
            else {
                return failed();
            };
            k += 1;
            differ += figures.len().abs_diff(built.figures.len());
            for (a, b) in figures.iter().zip(&built.figures) {
                differ += usize::from(a.value.to_bits() != b.value.to_bits());
            }
            for (w, m) in wavelengths.iter().zip(s) {
                let want = built.s_matrix(Wavelength::um(*w).expect(OK)).expect(OK);
                for (q, row) in m.iter().enumerate() {
                    for (p, v) in row.iter().enumerate() {
                        let x = want[(q, p)];
                        differ += usize::from(v[0].to_bits() != x.re.to_bits());
                        differ += usize::from(v[1].to_bits() != x.im.to_bits());
                    }
                }
            }
            if s.len() != wavelengths.len() {
                return failed();
            }
        }
    }
    if k != 12 || points(&events).len() != 12 {
        return failed();
    }
    count(differ)
}

fn threads() -> Outcome {
    let ring = Study::Sweep(vec![SweepAxis::new("radius", 5.0, 20.0, 31)]);
    let corners = Study::Corners(vec![
        Corner::new("wide", &[("width", 0.02)]),
        Corner::new("narrow", &[("width", -0.02)]),
        Corner::new("thick", &[("thickness", 0.01)]),
        Corner::new("thin", &[("thickness", -0.01)]),
    ]);
    let jobs = [
        study_from_device("ring", &Spec::new("all-pass ring"), &band(5), &ring).expect(OK),
        study_from_device("guide", &Spec::new("waveguide"), &band(5), &corners).expect(OK),
    ];
    let mut differ = 0;
    for job in &jobs {
        let on = |threads: usize| -> Vec<String> {
            let mut run = Run::in_memory();
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("a pool")
                .install(|| execute(job, &mut run, &Stop::new(None)))
                .expect(OK);
            run.lines()
                .iter()
                .filter(|l| !l.starts_with("{\"type\":\"finished\""))
                .cloned()
                .collect()
        };
        let (one, twenty) = (on(1), on(20));
        differ += one.len().abs_diff(twenty.len());
        differ += one.iter().zip(&twenty).filter(|(a, b)| a != b).count();
    }
    count(differ)
}

fn corners_closed_form() -> Outcome {
    let mut spec = Spec::new("waveguide");
    spec.parameters = vec![("length".into(), "1000".into())];
    let corners = Study::Corners(vec![
        Corner::new("long, lossy", &[("length", 50.0), ("loss", 1.0)]),
        Corner::new("long", &[("length", 50.0), ("loss", -1.0)]),
        Corner::new("lossy", &[("length", -50.0), ("loss", 1.0)]),
        Corner::new("short", &[("length", -50.0), ("loss", -1.0)]),
    ]);
    let job = study_from_device("corners", &spec, &band(5), &corners).expect(OK);
    let events = events_of(&job);
    let mut worst = 0f64;
    // each point's mean |S21|²
    let mut means = Vec::new();
    for p in points(&events) {
        let Event::DevicePoint { values, s, .. } = p else {
            return failed();
        };
        // the waveguide's parameters: width, thickness, slab, sidewall, length, loss, ...
        let device = spec.device().expect(OK);
        let at = |name: &str| {
            let k = device
                .parameters()
                .iter()
                .position(|q| q.name == name)
                .expect("a waveguide's parameter");
            values[k]
        };
        let (length, loss) = (at("length"), at("loss"));
        let want = 10f64.powf(-loss * length * 1e-4 / 10.0);
        let mut sum = 0.0;
        for m in s {
            let t = m[1][0][0].powi(2) + m[1][0][1].powi(2);
            worst = worst.max((t - want).abs() / want);
            sum += t;
        }
        means.push(sum);
    }
    if means.len() != 5 {
        return failed();
    }
    // the extremes: the longest, lossiest corner (1) the least, the shortest (4) the most
    let lowest = (0..5).min_by(|&a, &b| means[a].total_cmp(&means[b]));
    let highest = (0..5).max_by(|&a, &b| means[a].total_cmp(&means[b]));
    if (lowest, highest) != (Some(1), Some(4)) {
        return failed();
    }
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1e-12,
        error: worst,
    }
}

/// The slope of a least-squares line through (`x`, log₁₀ `y`).
fn log_slope(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let ly: Vec<f64> = y.iter().map(|v| v.log10()).collect();
    let (mx, my) = (x.iter().sum::<f64>() / n, ly.iter().sum::<f64>() / n);
    let sxy: f64 = x.iter().zip(&ly).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    sxy / sxx
}

/// The coupler's cross-over lengths over a sweep of its gap at 3D, at 1.55 µm, on `step_nm`
/// (the device's own cells when `None`): the gaps in nm and L_x in µm.
fn crossings(from: f64, to: f64, points_n: usize, step_nm: Option<f64>) -> (Vec<f64>, Vec<f64>) {
    let mut spec = Spec::new("directional coupler");
    spec.fidelity = Fidelity::ThreeD;
    let compute = Compute {
        band: Some((1.55, 1.55, 1)),
        step_nm,
        ..Compute::model()
    };
    let study = Study::Sweep(vec![SweepAxis::new("gap", from, to, points_n)]);
    let job = study_from_device("coupler-gap", &spec, &compute, &study).expect(OK);
    let events = events_of(&job);
    let mut gaps = Vec::new();
    let mut lx = Vec::new();
    for p in points(&events) {
        if let Event::DevicePoint { at, figures, .. } = p
            && let Some(f) = figures.iter().find(|f| f.symbol == "L_x")
        {
            gaps.push(at[0] * 1e3);
            lx.push(f.value);
        }
    }
    (gaps, lx)
}

fn coupler_chrostowski() -> Outcome {
    let (gaps, lx) = crossings(0.15, 0.3, 4, None);
    if lx.len() != 4 {
        return failed();
    }
    let slope = round(log_slope(&gaps, &lx), 9);
    let paper = 0.003_764_5;
    Outcome {
        measured: slope,
        expected: paper,
        tolerance: 0.05 * paper,
        error: (slope - paper).abs(),
    }
}

fn coupler_grid() -> Outcome {
    let slopes: Vec<f64> = [10.0, 5.0]
        .iter()
        .map(|&step| {
            let (gaps, lx) = crossings(0.15, 0.3, 2, Some(step));
            if lx.len() == 2 {
                log_slope(&gaps, &lx)
            } else {
                f64::NAN
            }
        })
        .collect();
    let (middle, fine) = (round(slopes[0], 9), round(slopes[1], 9));
    Outcome {
        measured: middle,
        expected: fine,
        tolerance: round(1e-3 * fine, 9),
        error: (middle - fine).abs(),
    }
}

/// `x` to `places` decimals: shown without the last bits of a platform's arithmetic.
fn round(x: f64, places: i32) -> f64 {
    let s = 10f64.powi(places);
    (x * s).round() / s
}

fn count(n: usize) -> Outcome {
    Outcome {
        measured: n as f64,
        expected: 0.0,
        tolerance: 0.0,
        error: n as f64,
    }
}

fn failed() -> Outcome {
    Outcome {
        measured: f64::NAN,
        expected: 0.0,
        tolerance: 0.0,
        error: f64::INFINITY,
    }
}

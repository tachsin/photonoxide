//! A maximally flat filter of two coupled rings co-designed from devices on a chip: each ring a
//! strip's two half-circle bends between directional couplers, the inner coupler's gap and each
//! ring's radius the variables of the chip's `optimize` job, fitted by genoxide's L-BFGS-B until
//! the drop port's response is the Butterworth shape 1/(1 + (δ/δ_c)⁴), and the ring-to-ring
//! coupling compared with Little's synthesis.
//!
//! B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, 998
//! (1997), [doi:10.1109/50.588673](https://doi.org/10.1109/50.588673), Table I: the maximally
//! flat filter of two rings has μ₁²/μ⁴ = 0.250, μ the bus's coupling (μ² = κ_b²/T for a round
//! trip T) and μ₁ the rings' (μ₁ = κ₁/T), so κ₁²/κ_b⁴ = 0.250 for rings of the same round trip.
//!
//! **The chip.** Lossless 500 × 220 nm silicon strips on 220 nm SOI, every device at its
//! analytic fidelity: the buses' couplers 200 nm apart for κ_b² = 1e-3, the rings' coupler 100 nm
//! long, each ring a bend of 10 µm radius and one of radius R (a design parameter offset by u,
//! R = R₀ + u × 1e-6 µm). The variables are the rings' coupler's gap and each ring's u: the gap
//! sets κ₁² through the coupler's coupled-mode model (its coupling's fit over the gap, exact: no
//! grid beyond the fit's), and the radii bring both rings onto resonance at 1.55 µm. The devices
//! give no exact derivative in a gap or a radius (each builds another component), so the job
//! takes central differences in them; its objective is the misfit to the Butterworth shape at 61
//! wavelengths over ±3κ_b² of round-trip phase about 1.55 µm, δ_c = κ_b²/√2 (the prototype's g₁/2
//! of the bus's coupling in coupled-mode theory, Little's Eq. 9 with N = 2).
//!
//! The circuit's exact couplers differ from coupled-mode theory at O(κ_b²): at κ_b² = 1e-3 the
//! circuit's maximally flat ratio is 0.2502 (`optimize_ring_flat` finds 0.25025 with δ_c free),
//! within the printed digits of 0.250. The circuit solve: exact, no grid.
//!
//! ```sh
//! cargo run --release --example optimize_chip_rings
//! ```

mod common;

use std::f64::consts::{PI, TAU};
use std::process::ExitCode;

use genoxide::prelude::*;
use photonoxide::job::{Problem, check};
use photonoxide::run::{Format, Job};
use photonoxide::units::Wavelength;

/// The centre wavelength, µm.
const LAMBDA: f64 = 1.55;
/// The buses' power coupling.
const BUS: f64 = 1e-3;
/// The rings' coupler's length, µm.
const INNER_LENGTH: f64 = 0.1;
/// The fixed bend's radius, µm.
const FIXED: f64 = 10.0;
/// The buses' couplers' gap, µm.
const BUS_GAP: f64 = 0.2;
/// The starts (the rings' gap µm, u₁, u₂), the evaluations of each, and the wavelengths.
const STARTS: [[f64; 3]; 3] = [[0.42, 20.0, -15.0], [0.46, -10.0, 25.0], [0.44, 5.0, 5.0]];
const EVALUATIONS: u64 = 3_000;
const POINTS: usize = 61;
/// The tolerances: μ₁²/μ⁴ (three printed digits), the radii (nm).
const RATIO_TOLERANCE: f64 = 5e-4;
const RADIUS_TOLERANCE: f64 = 1e-6;

/// A directional-coupler device's table: lossless, at `gap` and `length` µm.
fn coupler(gap: &str, length: f64) -> String {
    format!(
        "{{ kind = \"directional coupler\", fidelity = \"analytic\", parameters = {{ gap = \"{gap}\", length = \"{length:?}\", loss = \"0\" }} }}"
    )
}

/// A half-circle bend device: lossless, its radius the expression `radius`, with the named
/// parameter `u` at `offset` when it has one.
fn bend(radius: &str, offset: Option<f64>) -> String {
    let u = offset.map_or_else(String::new, |u| format!(", u = \"{u:?}\""));
    format!(
        "{{ kind = \"bend\", fidelity = \"analytic\", parameters = {{ angle = \"180\", loss = \"0\", radius = \"{radius}\"{u} }} }}"
    )
}

/// The chip's optimization job: two rings between buses, the target's width `width` µm in
/// wavelength, the radii about `r0`, the bus couplers `bus_length` µm long, the variables from
/// `start` (gap, u₁, u₂).
fn job(r0: f64, bus_length: f64, width: f64, start: [f64; 3]) -> photonoxide::Result<Job> {
    let r = format!("{r0:?} + u*1e-6");
    let lo = LAMBDA - 3.0 * width * 2f64.sqrt();
    let hi = LAMBDA + 3.0 * width * 2f64.sqrt();
    let text = format!(
        r#"name = "chip-little-rings"
timeout_minutes = 10

[circuit]
connections = [
    ["c0.o3", "a1.o1"], ["a1.o2", "c1.o2"], ["c1.o3", "b1.o1"], ["b1.o2", "c0.o2"],
    ["c1.o4", "a2.o1"], ["a2.o2", "c2.o2"], ["c2.o3", "b2.o1"], ["b2.o2", "c1.o1"],
]

[[circuit.instance]]
name = "c0"
device = {bus}

[[circuit.instance]]
name = "c1"
device = {inner}

[[circuit.instance]]
name = "c2"
device = {bus}

[[circuit.instance]]
name = "a1"
device = {fixed}

[[circuit.instance]]
name = "b1"
device = {b1}

[[circuit.instance]]
name = "a2"
device = {fixed}

[[circuit.instance]]
name = "b2"
device = {b2}

[[circuit.port]]
name = "in"
at = "c0.o1"

[[circuit.port]]
name = "through"
at = "c0.o4"

[[circuit.port]]
name = "add"
at = "c2.o1"

[[circuit.port]]
name = "drop"
at = "c2.o4"

[task]
kind = "optimize"
method = "lbfgsb"
seed = 1
evaluations = {EVALUATIONS}
from_um = {lo:?}
to_um = {hi:?}
points = {POINTS}
reduce = "average"

[[task.variable]]
parameter = "c1.gap"
from = 0.3
to = 0.5

[[task.variable]]
parameter = "b1.u"
from = -200.0
to = 200.0

[[task.variable]]
parameter = "b2.u"
from = -200.0
to = 200.0

[[task.objective]]
expression = "(T_drop_in - 1/(1 + ((lambda - {LAMBDA:?})/{width:?})^4))^2"
goal = "minimize"
"#,
        bus = coupler(&format!("{BUS_GAP:?}"), bus_length),
        inner = coupler(&format!("{:?}", start[0]), INNER_LENGTH),
        fixed = bend(&format!("{FIXED:?}"), None),
        b1 = bend(&r, Some(start[1])),
        b2 = bend(&r, Some(start[2])),
    );
    let job = Job::parse(&text)?;
    check(&job)?;
    Ok(job)
}

pub fn main() -> photonoxide::Result<ExitCode> {
    use photonoxide::device::{Device, StripBend, StripCoupler, Values};
    let mut checks = common::Checks::default();
    // the devices' own numbers at 1.55 µm: the coupling at 200 nm and the strip's indices
    let at = Wavelength::um(LAMBDA)?;
    let dc = StripCoupler::new();
    let c_bus = dc
        .component(
            &Values::new(&dc, &[("gap", BUS_GAP), ("loss", 0.0)])?,
            photonoxide::circuit::Fidelity::Analytic,
        )?
        .figure("C")
        .map_or(f64::NAN, |f| f.value);
    let bus_length = (BUS.sqrt()).asin() / c_bus;
    let db = StripBend::new();
    let half = db.component(
        &Values::new(&db, &[("angle", 180.0), ("radius", FIXED), ("loss", 0.0)])?,
        photonoxide::circuit::Fidelity::Analytic,
    )?;
    let n_g = half.figure("n_g").map_or(f64::NAN, |f| f.value);
    let n_eff = half.figure("n_eff").map_or(f64::NAN, |f| f.value);
    // the radius R₀ that puts a ring on resonance: its round-trip phase, the bends' β π (10 + R)
    // and its two couplers' common phases, a whole number of 2π
    let common = |gap: f64, length: f64| -> photonoxide::Result<f64> {
        let v = Values::new(&dc, &[("gap", gap), ("length", length), ("loss", 0.0)])?;
        let s = dc
            .component(&v, photonoxide::circuit::Fidelity::Analytic)?
            .s_matrix(at)?;
        Ok(s[(2, 1)].arg())
    };
    let beta = TAU * n_eff / LAMBDA;
    let rest =
        common(BUS_GAP, bus_length)? + common(STARTS[0][0], INNER_LENGTH)? + beta * PI * FIXED;
    let m = ((rest + beta * PI * FIXED) / TAU).round();
    let r0 = (TAU * m - rest) / (beta * PI);
    // the Butterworth half-width δ_c = κ_b²/√2 of round-trip phase, in wavelength: dφ/dλ =
    // 2π n_g L/λ² over the round trip's length L
    let length = PI * (FIXED + r0) + bus_length + INNER_LENGTH;
    let width = BUS / 2f64.sqrt() * LAMBDA * LAMBDA / (TAU * n_g * length);
    let mut setup = common::Setup::new();
    setup.part("the buses' couplers, c0 and c2");
    let bus = Values::new(
        &dc,
        &[("gap", BUS_GAP), ("length", bus_length), ("loss", 0.0)],
    )?;
    setup.device(&dc, &bus, at);
    setup.part("the rings' coupler, c1");
    let inner = Values::new(
        &dc,
        &[
            ("gap", STARTS[0][0]),
            ("length", INNER_LENGTH),
            ("loss", 0.0),
        ],
    )?;
    setup.parameters(&dc, &inner);
    setup.item(
        "its gap",
        "searched, 0.3 to 0.5 µm; the first start's above",
    );
    setup.part("the rings' bends: a1 and a2 fixed, b1 and b2 searched");
    setup.parameters(
        &db,
        &Values::new(&db, &[("angle", 180.0), ("radius", FIXED), ("loss", 0.0)])?,
    );
    setup.item(
        "b1, b2",
        &format!("as a1, of radius R₀ + u × 1e-6 µm, R₀ = {r0:.6} µm, u from −200 to 200"),
    );
    setup.part("the fit");
    setup.item(
        "wavelengths",
        &format!(
            "{POINTS} from {:.6} to {:.6} µm",
            LAMBDA - 3.0 * width * 2f64.sqrt(),
            LAMBDA + 3.0 * width * 2f64.sqrt()
        ),
    );
    setup.item(
        "grid",
        "the devices' analytic models; the circuit solve, exact",
    );
    setup.item(
        "solver",
        &format!(
            "L-BFGS-B by central differences, {EVALUATIONS} evaluations from each of {} starts \
             (gap, u₁, u₂): {}",
            STARTS.len(),
            STARTS
                .map(|s| format!("({})", s.map(common::number).join(", ")))
                .join(", ")
        ),
    );
    setup.item(
        "tolerance",
        &format!(
            "{RATIO_TOLERANCE:e} on μ₁²/μ⁴; {RADIUS_TOLERANCE:e} nm on the radii, alike and on \
             resonance"
        ),
    );
    setup.print();
    println!(
        "A maximally flat filter of two rings of devices on a chip (Little et al. 1997, Table I): lossless 500 x 220 nm strips, analytic, the circuit solve, exact: no grid"
    );
    println!(
        "  the buses' couplers 200 nm apart, {bus_length:.4} µm long for κ_b² = 1e-3; the rings' coupler {INNER_LENGTH} µm long; each ring a bend of {FIXED} µm and one of R = {r0:.6} µm + u × 1e-6 µm"
    );
    println!(
        "  variables: the rings' coupler's gap, u₁ and u₂, by central differences; fitted to 1/(1 + (δ/δ_c)⁴) at 61 wavelengths, δ_c = κ_b²/√2 = {:.4} pm, by L-BFGS-B",
        width * 1e6
    );
    // the built-in job (jobs/chip-little-rings.yaml) is this chip, from the first start
    let builtin = Job::parse_as(include_str!("../jobs/chip-little-rings.yaml"), Format::Yaml)?;
    let first = job(r0, bus_length, width, STARTS[0])?;
    checks.count(
        "jobs/chip-little-rings.yaml is this job",
        usize::from(builtin.circuit() == first.circuit() && builtin.task() == first.task()),
        1,
    );
    let mut best: Option<(f64, Vec<f64>)> = None;
    for start in STARTS {
        let problem = Problem::of(&job(r0, bus_length, width, start)?)?;
        let real = Real::new([0.3..=0.5, -200.0..=200.0, -200.0..=200.0]).expect("bounds");
        let f = |x: &Reals, g: &mut [f64]| {
            let e = problem.evaluate(x, true);
            match (e.costs.first(), e.gradients.first()) {
                (Some(&c), Some(d)) if c.is_finite() => {
                    g.copy_from_slice(d);
                    c
                }
                _ => f64::NAN,
            }
        };
        let lbfgsb = Lbfgsb::builder(real)
            .initial_genome(Reals::from(start.to_vec()))
            .gradient_tolerance(1e-16)
            .function_tolerance(0.0)
            .minimize()
            .build()
            .expect("a valid L-BFGS-B");
        let outcome = Engine::new(lbfgsb, Differentiable(&f))
            .stop_when(Stop::evaluations(EVALUATIONS))
            .run()
            .expect("a run");
        let x = outcome.best_genome().to_vec();
        let cost = problem.evaluate(&x, false).costs[0];
        if best.as_ref().is_none_or(|b| cost < b.0) {
            best = Some((cost, x));
        }
    }
    let (misfit, x) = best.expect("a start");
    // the couplings the chip's devices give at the optimum
    let cross = |gap: f64, length: f64| -> photonoxide::Result<f64> {
        let v = Values::new(&dc, &[("gap", gap), ("length", length), ("loss", 0.0)])?;
        Ok(dc
            .component(&v, photonoxide::circuit::Fidelity::Analytic)?
            .figure("cross")
            .map_or(f64::NAN, |f| f.value))
    };
    let kb = cross(BUS_GAP, bus_length)?;
    let ki = cross(x[0], INNER_LENGTH)?;
    println!(
        "  the best of 3 starts: misfit {}, the rings' gap {:.5} µm",
        if misfit < 1e-15 {
            "below 1e-15".to_owned()
        } else {
            format!("{misfit:.0e}")
        },
        x[0]
    );
    checks.compare("μ₁²/μ⁴ = κ₁²/κ_b⁴", ki / (kb * kb), 0.250, RATIO_TOLERANCE);
    checks.against(
        "the rings' radii alike, |R₁ − R₂| (nm)",
        (x[1] - x[2]).abs() * 1e-3,
        "symmetry",
        0.0,
        1e-6,
    );
    checks.against(
        "on resonance, |R₁ − R₀| (nm)",
        x[1].abs() * 1e-3,
        "the phase's closed form",
        0.0,
        1e-6,
    );
    Ok(checks.finish())
}

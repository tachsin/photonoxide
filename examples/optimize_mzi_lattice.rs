//! A maximally flat half-band lattice filter found by optimization: three couplers joined by
//! two delays, 2ΔL then ΔL, their couplings and phases fitted by genoxide's L-BFGS-B on the
//! circuit adjoint's gradient (from seeded random starts) until the cross port's response is the
//! maximally flat half-band response, and compared with Jinguji and Oguma's design.
//!
//! K. Jinguji, T. Oguma, J. Lightwave Technol. 18, 252 (2000),
//! [doi:10.1109/50.822800](https://doi.org/10.1109/50.822800), Table I: the maximally flat
//! half-band filter of this lattice, θ = 0.0833π, 0.3333π, 0.25π (power couplings sin²θ =
//! 0.067, 0.75, 0.5) and φ = π, 0; synthesized as in K. Jinguji, M. Kawachi, J. Lightwave
//! Technol. 13, 73 (1995), [doi:10.1109/50.350643](https://doi.org/10.1109/50.350643).
//!
//! **The target.** A lossless lattice of total delay 3ΔL is a power-complementary trigonometric
//! polynomial of degree 3 in ω, the phase across ΔL. The maximally flat half-band one of that
//! degree, flat to order 4 at ω = 0 and 0 to order 4 at ω = π, is
//! T(ω) = cos⁴(ω/2) (1 + 2 sin²(ω/2)), T(ω) + T(ω + π) = 1: the power response of the shortest
//! of I. Daubechies's filters after the Haar one (Commun. Pure Appl. Math. 41, 909 (1988),
//! doi:10.1002/cpa.3160410705). Table I's design gives it at the cross port to 1e-15.
//!
//! **The power, and the field.** Fitted to T alone at 64 phases over one period, the design is
//! found only roughly: T's double zero at ω = π makes the misfit about quartic near it, and
//! L-BFGS-B stops a few 1e-4 π away (printed). The power fixes |H| only; Table I's design
//! realizes the spectral factor H_D(ω) = ((1 + z)/2)² ((1 − √3)/2 + (1 + √3)/2 z), z = e^(iω),
//! and fitted to H_D's field (with the best global phase), the problem is well conditioned:
//! that is the fit checked. The circuit solve: exact, no grid.
//!
//! ```sh
//! cargo run --release --example optimize_mzi_lattice
//! ```

mod common;

use std::f64::consts::{PI, TAU};
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::math;
use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::circuit::components::{Coupler, PhaseShifter};
use photonoxide::circuit::{Circuit, Netlist, SMatrix};
use photonoxide::units::Wavelength;

const LAMBDA: f64 = 1.55;
/// The seeded random starts of each fit.
const STARTS: u64 = 16;
/// The tolerances: θ (units of π, Table I's 4 printed digits), φ (π), the power couplings.
const THETA_TOLERANCE: f64 = 5e-5;
const PHI_TOLERANCE: f64 = 1e-6;
const KAPPA_TOLERANCE: f64 = 5e-4;

/// Couplers c1, c2, c3; between c1 and c2 the upper arm u1 (2ΔL and φ₁) and the lower l1,
/// between c2 and c3 u2 (ΔL and φ₂) and l2; ports in1, in2, out1 (bar), out2 (cross).
fn lattice() -> photonoxide::Result<Circuit> {
    let coupler = Arc::new(Coupler::new());
    let shifter = Arc::new(PhaseShifter::new());
    let mut n = Netlist::new();
    for c in ["c1", "c2", "c3"] {
        n.add(c, coupler.clone())?;
    }
    for a in ["u1", "l1", "u2", "l2"] {
        n.add(a, shifter.clone())?;
    }
    for (a, b) in [
        ("c1.o4", "u1.o1"),
        ("u1.o2", "c2.o1"),
        ("c1.o3", "l1.o1"),
        ("l1.o2", "c2.o2"),
        ("c2.o4", "u2.o1"),
        ("u2.o2", "c3.o1"),
        ("c2.o3", "l2.o1"),
        ("l2.o2", "c3.o2"),
    ] {
        n.connect(a, b)?;
    }
    for (name, port) in [
        ("in1", "c1.o1"),
        ("in2", "c1.o2"),
        ("out1", "c3.o4"),
        ("out2", "c3.o3"),
    ] {
        n.expose(name, port)?;
    }
    n.compile()
}

/// The maximally flat half-band response.
fn target(w: f64) -> f64 {
    // genoxide's portable sine and cosine: the same bits, and the same optimizer paths, on every
    // system
    let (s, c) = math::sin_cos(w / 2.0);
    c.powi(4) * (1.0 + 2.0 * s * s)
}

/// The phases across ΔL.
fn phases() -> Vec<f64> {
    (0..64).map(|k| TAU * f64::from(k) / 64.0).collect()
}

/// x = (κ₁², κ₂², κ₃², φ₁, φ₂).
struct Lattice {
    circuit: Circuit,
    slots: [usize; 5],
    arms: [usize; 2],
}

impl Lattice {
    fn new() -> photonoxide::Result<Lattice> {
        let circuit = lattice()?;
        let at = |name: &str| circuit.parameter(name).expect("a parameter");
        let slots = [
            at("c1.coupling"),
            at("c2.coupling"),
            at("c3.coupling"),
            at("u1.phase"),
            at("u2.phase"),
        ];
        let arms = [at("u1.phase"), at("u2.phase")];
        Ok(Lattice {
            circuit,
            slots,
            arms,
        })
    }

    /// The circuit's values at x and the phase w across ΔL: the upper arms 2w + φ₁ and w + φ₂,
    /// wrapped into (−π, π].
    fn values(&self, x: &[f64], w: f64) -> Vec<f64> {
        let wrap = |p: f64| p - TAU * ((p + PI) / TAU).floor();
        let mut v = self.circuit.values().to_vec();
        for k in 0..3 {
            v[self.slots[k]] = x[k];
        }
        v[self.arms[0]] = wrap(2.0 * w + x[3]);
        v[self.arms[1]] = wrap(w + x[4]);
        v
    }

    /// The cross port's power at x and w.
    fn cross(&self, x: &[f64], w: f64) -> f64 {
        let lambda = Wavelength::um(LAMBDA).expect("a wavelength");
        self.circuit
            .s_matrix_with(lambda, &self.values(x, w))
            .map_or(f64::NAN, |s| s.power(3, 0))
    }

    /// Σ_w (P_cross − T)² and its gradient in x, by the circuit adjoint.
    fn misfit(&self, x: &[f64], gradient: &mut [f64]) -> f64 {
        let lambda = [Wavelength::um(LAMBDA).expect("a wavelength")];
        gradient.fill(0.0);
        let mut total = 0.0;
        for w in phases() {
            let t = target(w);
            let objective = |s: &[photonoxide::circuit::SMatrix]| {
                photonoxide::circuit::objective::power_error(s, 3, 0, &[t])
            };
            let Ok((f, g)) = self
                .circuit
                .gradient(&lambda, &self.values(x, w), objective)
            else {
                return f64::NAN;
            };
            total += f;
            for k in 0..5 {
                gradient[k] += g[self.slots[k]];
            }
        }
        total
    }

    /// Σ_w |H_cross − e^(iψ) H_D|², ψ the best global phase (Σ H_D* H's), and its gradient in
    /// x by the circuit adjoint (ψ's own derivative is zero at its best: the envelope theorem).
    fn misfit_field(&self, x: &[f64], gradient: &mut [f64]) -> f64 {
        let lambda = Wavelength::um(LAMBDA).expect("a wavelength");
        gradient.fill(0.0);
        let mut overlap = c64::new(0.0, 0.0);
        for w in phases() {
            let Ok(s) = self.circuit.s_matrix_with(lambda, &self.values(x, w)) else {
                return f64::NAN;
            };
            overlap += factor(w).conj() * s[(3, 0)];
        }
        let phase = if overlap.norm() > 0.0 {
            overlap / overlap.norm()
        } else {
            c64::new(1.0, 0.0)
        };
        let mut total = 0.0;
        for w in phases() {
            let want = phase * factor(w);
            let objective = |s: &[SMatrix]| {
                let miss = s[0][(3, 0)] - want;
                let mut g = SMatrix::zeros(4);
                g[(3, 0)] = miss.conj();
                (miss.norm_sqr(), vec![g])
            };
            let Ok((f, g)) = self
                .circuit
                .gradient(&[lambda], &self.values(x, w), objective)
            else {
                return f64::NAN;
            };
            total += f;
            for k in 0..5 {
                gradient[k] += g[self.slots[k]];
            }
        }
        total
    }
}

/// The spectral factor of T Table I's design realizes: H_D(ω) = ((1 + z)/2)² ((1 − √3)/2 +
/// (1 + √3)/2 z), z = e^(iω), |H_D|² = T.
fn factor(w: f64) -> c64 {
    let r3 = 3f64.sqrt();
    let (s, c) = math::sin_cos(w);
    let z = c64::new(c, s);
    let one = c64::new(1.0, 0.0);
    ((one + z) / 2.0).powi(2) * ((one - r3) / 2.0 + z * ((1.0 + r3) / 2.0))
}

/// Table I's design in x: power couplings sin²θ and the phases.
fn table() -> [f64; 5] {
    [
        (PI / 12.0).sin().powi(2),
        (PI / 3.0).sin().powi(2),
        (PI / 4.0).sin().powi(2),
        PI,
        0.0,
    ]
}

/// θ/π of a power coupling.
fn theta(kappa2: f64) -> f64 {
    kappa2.clamp(0.0, 1.0).sqrt().asin() / PI
}

/// A phase wrapped into (−π, π].
fn wrap(p: f64) -> f64 {
    let w = p - TAU * ((p + PI) / TAU).floor();
    if w <= -PI { w + TAU } else { w }
}

/// A misfit and its gradient.
type Misfit<'a> = &'a (dyn Fn(&[f64], &mut [f64]) -> f64 + Sync);

/// The largest relative difference between the adjoint's gradient of `misfit` at x and
/// fourth-order central differences.
fn gradient_check(misfit: Misfit<'_>, x: &[f64]) -> f64 {
    let mut g = vec![0.0; 5];
    misfit(x, &mut g);
    let scale = g.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let h = 1e-4;
    (0..5)
        .map(|k| {
            let at = |s: f64| {
                let mut y = x.to_vec();
                y[k] += s * h;
                let mut scratch = vec![0.0; 5];
                misfit(&y, &mut scratch)
            };
            let fd = (at(-2.0) - 8.0 * at(-1.0) + 8.0 * at(1.0) - at(2.0)) / (12.0 * h);
            (fd - g[k]).abs() / scale
        })
        .fold(0.0, f64::max)
}

/// The search space: three power couplings and two phases.
fn space() -> Real {
    Real::new([0.0..=1.0, 0.0..=1.0, 0.0..=1.0, -PI..=PI, -PI..=PI]).expect("bounds")
}

/// L-BFGS-B on `misfit` from `start`: the design, its misfit and the evaluations.
fn polish(misfit: Misfit<'_>, start: Reals) -> (Vec<f64>, f64, u64) {
    let lbfgsb = Lbfgsb::builder(space())
        .initial_genome(start)
        .gradient_tolerance(0.0)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let fitness = |x: &Reals, g: &mut [f64]| misfit(x, g);
    let outcome = Engine::new(lbfgsb, Differentiable(&fitness))
        .stop_when(Stop::target(1e-28).or(Stop::evaluations(3_000)))
        .run()
        .expect("a run");
    let x = outcome.best_genome().to_vec();
    let mut g = vec![0.0; 5];
    let f = misfit(&x, &mut g);
    (x, f, outcome.evaluations())
}

/// x as θ/π and φ/π.
fn canonical(x: &[f64]) -> Vec<f64> {
    vec![
        theta(x[0]),
        theta(x[1]),
        theta(x[2]),
        clean(wrap(x[3]) / PI),
        clean(wrap(x[4]) / PI),
    ]
}

/// A phase in units of π as printed: −1 is 1 (the same phase), and round-off about 0 or ±1
/// dropped, whose sign the last bit decides.
fn clean(p: f64) -> f64 {
    if (p.abs() - 1.0).abs() < 1e-9 {
        1.0
    } else if p.abs() < 1e-9 {
        0.0
    } else {
        p
    }
}

fn shown(d: &[f64]) -> String {
    format!(
        "θ = {:.5}π, {:.5}π, {:.5}π; φ = {:.5}π, {:.5}π",
        d[0], d[1], d[2], d[3], d[4]
    )
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let lattice = Lattice::new()?;
    let power: Misfit<'_> = &|x: &[f64], g: &mut [f64]| lattice.misfit(x, g);
    let field: Misfit<'_> = &|x: &[f64], g: &mut [f64]| lattice.misfit_field(x, g);
    let mut setup = common::Setup::new();
    setup.part("the circuit, at its parameters' defaults");
    setup.circuit(&lattice.circuit, lattice.circuit.values());
    setup.part("the fits");
    setup.item(
        "delays",
        "u1 2ΔL and u2 ΔL longer than l1 and l2: their phases 2ω + φ₁ and ω + φ₂",
    );
    setup.item(
        "phases",
        &format!(
            "{} values of ω across ΔL, 0 to 2π; the wavelength {LAMBDA} µm",
            phases().len()
        ),
    );
    setup.item("grid", "none: the circuit's S-matrix, exact");
    setup.item(
        "solver",
        &format!(
            "x = (κ₁², κ₂², κ₃², φ₁, φ₂) by L-BFGS-B on the circuit adjoint from {STARTS} seeded \
             random starts, by power and by field; CMA-ES as a global check"
        ),
    );
    setup.item(
        "tolerance",
        &format!(
            "{THETA_TOLERANCE:e} π in θ, {PHI_TOLERANCE:e} π in φ, {KAPPA_TOLERANCE:e} in the \
             power couplings"
        ),
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!(
        "A maximally flat half-band lattice: couplers θ₁, θ₂, θ₃, delays 2ΔL + φ₁ and ΔL + φ₂ (Jinguji & Oguma 2000, Table I), the circuit solve, exact: no grid"
    );
    println!(
        "  T(ω) = cos⁴(ω/2)(1 + 2 sin²(ω/2)) at 64 phases ω across ΔL; the cross port from in1"
    );
    let design = table();
    let worst = phases()
        .iter()
        .map(|&w| (lattice.cross(&design, w) - target(w)).abs())
        .fold(0.0, f64::max);
    checks.against(
        "Table I's design: |P_cross − T|, largest",
        worst,
        "T, maximally flat",
        0.0,
        1e-14,
    );
    let mut g = vec![0.0; 5];
    checks.against(
        "its field misfit to H_D",
        field(&design, &mut g),
        "H_D, T's factor",
        0.0,
        1e-28,
    );
    let start = [0.3, 0.3, 0.3, 1.0, -1.0];
    for (name, misfit) in [("power", power), ("field", field)] {
        let check = gradient_check(misfit, &start);
        println!(
            "  the circuit adjoint's gradient of the {name} misfit against fourth-order differences: {}",
            if check < 1e-10 {
                "within 1e-10 of the largest"
            } else {
                "past 1e-10 of the largest"
            }
        );
        checks.against(
            &format!("{name}: adjoint within 1e-8 of differences"),
            f64::from(u8::from(check < 1e-8)),
            "yes",
            1.0,
            0.0,
        );
    }

    // the power alone: its double zero at ω = π makes the misfit quartic near the design, so
    // that the design is found only to about the fourth root of the misfit
    let starts = |misfit: Misfit<'_>| -> Vec<(Vec<f64>, f64, u64)> {
        (1..=STARTS)
            .map(|seed| {
                polish(
                    misfit,
                    space().random_genome(&mut StreamRng::seed_from_u64(seed)),
                )
            })
            .collect()
    };
    let found = starts(power);
    let best = found
        .iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a start");
    println!(
        "  fitting the power alone, L-BFGS-B from 16 seeded random starts: best misfit {}, θ = {:.3}π, {:.3}π, {:.3}π: about 1e-3 π from Table I's",
        magnitude(best.1),
        canonical(&best.0)[0],
        canonical(&best.0)[1],
        canonical(&best.0)[2]
    );

    // the field, to T's spectral factor Table I realizes: well conditioned
    let found = starts(field);
    let exact: Vec<&(Vec<f64>, f64, u64)> = found.iter().filter(|f| f.1 < 1e-24).collect();
    println!(
        "  fitting the field to H_D, L-BFGS-B on the adjoint from 16 seeded random starts: {} reach a misfit below 1e-24, in {} evaluations at most",
        exact.len(),
        exact.iter().map(|f| f.2).max().unwrap_or(0)
    );
    let mut designs: Vec<Vec<f64>> = Vec::new();
    for (x, _, _) in &exact {
        let c = canonical(x);
        if !designs.iter().any(|d| {
            d.iter().zip(&c).enumerate().all(|(k, (a, b))| {
                // a phase's distance round the circle, in units of π
                let gap = (a - b).abs();
                (if k < 3 { gap } else { gap.min(2.0 - gap) }) < 1e-6
            })
        }) {
            designs.push(c);
        }
    }
    for d in &designs {
        println!("    {}", shown(d));
    }
    checks.against(
        "designs that fit H_D",
        designs.len() as f64,
        "one",
        1.0,
        0.0,
    );
    let printed = [0.0833, 0.3333, 0.25];
    if let Some(d) = designs.first() {
        for (k, p) in printed.iter().enumerate() {
            checks.compare(&format!("θ{} (π)", k + 1), d[k], *p, THETA_TOLERANCE);
        }
        checks.compare("|φ₁| (π)", d[3].abs(), 1.0, PHI_TOLERANCE);
        checks.compare("φ₂ (π)", d[4], 0.0, PHI_TOLERANCE);
        for (k, kappa) in [0.067, 0.75, 0.5].iter().enumerate() {
            checks.compare(
                &format!("power coupling {}", k + 1),
                (d[k] * PI).sin().powi(2),
                *kappa,
                KAPPA_TOLERANCE,
            );
        }
    }

    // CMA-ES on the field's values alone, then the same polish: a global check
    let value = |x: &Reals| {
        let mut g = vec![0.0; 5];
        field(x, &mut g)
    };
    let cmaes = Cmaes::builder(space())
        .minimize()
        .seed(1)
        .build()
        .expect("a valid CMA-ES");
    let outcome = Engine::new(cmaes, &value)
        .stop_when(Stop::evaluations(3_000))
        .run()
        .expect("a run");
    let (x, f, _) = polish(field, outcome.best_genome().clone());
    println!(
        "  CMA-ES (3000 evaluations, seed 1) then L-BFGS-B on the field: misfit {}, {}",
        magnitude(f),
        shown(&canonical(&x))
    );
    checks.against(
        "CMA-ES's design: largest |θ − Table I's| (π)",
        canonical(&x)[..3]
            .iter()
            .zip(&printed)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max),
        "Table I",
        0.0,
        5e-5,
    );
    Ok(checks.finish())
}

/// A misfit's order of magnitude, as printed: the digits of one at round-off move with the
/// order of a sum.
fn magnitude(f: f64) -> String {
    if f < 1e-24 {
        "below 1e-24".into()
    } else {
        format!("{f:.0e}")
    }
}

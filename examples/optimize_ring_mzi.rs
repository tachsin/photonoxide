//! Wang et al.'s double-ring-loaded Mach–Zehnder filter: the fourth-order elliptic low-passes
//! it is synthesized for, derived here; its Fig. 4 circuit solved; and the circuit optimized
//! towards them by Nelder–Mead, as in the paper.
//!
//! M. Wang, X. Chen, U. Khan, W. Bogaerts, "Programmable wavelength filter with double ring
//! loaded MZI", Sci. Rep. 12, 1482 (2022),
//! [doi:10.1038/s41598-021-04598-6](https://doi.org/10.1038/s41598-021-04598-6).
//!
//! **The elliptic low-pass, derived here.** The paper takes its targets from MATLAB's `ellip`
//! (its ref. 21): order 4, the passband edge at 0.5π rad/sample, 2 dB of ripple and 40 dB of
//! attenuation (p. 5), and then 1 dB and 60 dB (p. 6). No filter-synthesis text is in our papers
//! folder, so the design is derived here from the standard construction. An elliptic low-pass
//! of even order N has |H(Ω)|² = 1/(1 + ε² R_N(Ω)²), with ε² = 10^(A_p/10) − 1 and R_N the
//! elliptic rational function, equiripple between ±1 in the passband |Ω| ≤ 1 and at least
//! 1/k₁ beyond Ω_s = 1/k, where k₁ = ε/√(10^(A_s/10) − 1). The selectivity k follows from the
//! degree equation, k' = k₁'^N Π sn⁴(u_i K', k₁') with u_i = (2i − 1)/N, i = 1 … N/2. The zeros
//! are j/(k cd(u_i K, k)) and the poles j cd((u_i − j v₀)K, k), with v₀ = sn⁻¹(j/ε, k₁)/(jN)
//! in units of K₁. The Jacobi functions come from Landen's descending transformation, k_{n+1} =
//! (k_n/(1 + k_n'))², taken until k_n < 1e-16: cd(uK, k) is cos(uπ/2) carried back up the
//! sequence by w ← (1 + k_{n+1}) w/(1 + k_{n+1} w²), and sn alike from sin(uπ/2). This is the
//! construction of S. J. Orfanidis's lecture notes on elliptic filter design (Rutgers, 2006).
//! Along the imaginary axis the inverse stays imaginary, so v₀ needs only a real inverse
//! hyperbolic sine. The bilinear transform s = (1 − z⁻¹)/(1 + z⁻¹) with Ω scaled by
//! tan(ω_p/2) maps it to the digital filter, and the gain makes the DC value the ripple's floor,
//! 10^(−A_p/20), as for any even order. Its check is exact: at ω_p the response is −A_p and at
//! ω_s = 2 arctan(tan(ω_p/2)/k) it is −A_s, an identity of R_N that tests the degree equation,
//! the zeros and the poles together.
//!
//! **The paper's transfer functions.** Eq. 10, printed as the 40 dB filter, has about 1 dB of
//! ripple and only 20 dB of attenuation at ω = π, and its decomposition into Eq. 11's two
//! sections doesn't multiply back to it. Neither is the filter its text specifies. Both are
//! printed, not checked.
//!
//! **The circuit** (Fig. 1c and Fig. 4, pp. 3 and 5). Two stages between three tunable couplers
//! (k1, k5, k9). In each stage a phase on each arm, then a ring coupled to the upper arm, a
//! second ring coupled to the first and to the lower arm, then a phase on each arm again. Each
//! ring is one unit of delay, z⁻¹ = e^(−jω) a round trip, with its couplers half a ring apart
//! and its own phase zero, as the paper uses it ("no phase change in the ring is required",
//! p. 3). The couplers are photonoxide's (through √(1 − k), across j√k, k the power coupling).
//! The low-pass is the cross port. Our 2 × 2 model of it is checked against photonoxide's
//! circuit solve of the same circuit built as a netlist of couplers, phase shifters and half
//! rings, to round-off.
//!
//! **What is compared.** Fig. 4 gives the synthesized couplings and phases. The paper's Fig. 5
//! shows "around 34 dB extinction ratio" for them (p. 5). Ours, solved exactly, has 1.38 dB of
//! ripple and 19.3 dB of attenuation beyond the 40 dB design's stopband edge: printed, not
//! checked, and the gap is left open. From Fig. 4's values the paper's Nelder–Mead reached the
//! 40 dB filter (Fig. 6) and then a 60 dB filter with 1 dB of ripple (Fig. 7, p. 6). Here
//! genoxide's Nelder–Mead runs from Fig. 4's values on the paper's target function, Eq. 12,
//! T = w₁ x_lin + w₂ x_dB with its "typical" weights 1 and 100 (p. 7). x_lin is the mean
//! difference of the field magnitudes and x_dB the mean difference in dB, both over 501
//! frequencies from 0 to π, the dB floored at −100. It gets 20 000 evaluations for each target.
//! What it reaches is printed beside the paper's figures, not checked: the paper's runs are
//! read off figures, without their weights' second step or evaluation counts.
//!
//! Everything is exact: closed forms and a 4-unknown solve per ring pair, in plain arithmetic
//! and genoxide's portable sine, cosine, exponential and logarithm, the same bits everywhere.
//!
//! ```sh
//! cargo run --release --example optimize_ring_mzi
//! ```

mod common;

use std::f64::consts::{LN_10, PI, TAU};
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::math;
use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::circuit::components::{Coupler, PhaseShifter};
use photonoxide::circuit::{Component, Fixed, Netlist, SMatrix};
use photonoxide::units::Wavelength;

/// Frequencies of the optimizer's target, over [0, π].
const GRID: usize = 501;
/// Frequencies a response's ripple and attenuation are read on, over [0, π].
const FINE: usize = 20_000;
/// Nelder–Mead's evaluations for each target.
const EVALUATIONS: u64 = 20_000;
/// The tolerances: each design at its edges (relative), the 2 × 2 model against the circuit's.
const EDGE_TOLERANCE: f64 = 1e-8;
const CIRCUIT_TOLERANCE: f64 = 1e-12;
/// Fig. 4's couplings k1–k9 (p. 5).
const FIG4_K: [f64; 9] = [
    0.29, 0.097747, 1.0, 0.097747, 0.7617, 0.7715, 0.9142, 0.7715, 0.7617,
];
/// Fig. 4's phases φ1–φ8 (p. 5).
const FIG4_PHI: [f64; 8] = [0.0, PI, 0.0, 0.0, 0.0, PI, 0.0, 0.0];
/// Eq. 10 as printed (p. 5): numerator and denominator in z⁻¹.
const EQ10: ([f64; 5], [f64; 5]) = (
    [0.2318, 0.3378, 0.5297, 0.3378, 0.2318],
    [1.0, -0.3396, 1.2275, -0.3118, 0.2964],
);
/// Eq. 11's two sections as printed (p. 5).
const EQ11: [([f64; 3], [f64; 3]); 2] = [
    ([0.39, 0.0910, 0.39], [1.0, 0.0094, 0.9023]),
    ([0.3, 0.3673, 0.3], [1.0, -0.3490, 0.0785]),
];

/// e^(ix), by genoxide's portable sine and cosine.
fn cis(x: f64) -> c64 {
    let (s, c) = math::sin_cos(x);
    c64::new(c, s)
}

/// ln x, by genoxide's portable logarithm, kept finite at 0.
fn ln(x: f64) -> f64 {
    math::ln(x.max(f64::MIN_POSITIVE))
}

/// A power in dB.
fn db(power: f64) -> f64 {
    10.0 * ln(power) / LN_10
}

/// tan x.
fn tan(x: f64) -> f64 {
    let (s, c) = math::sin_cos(x);
    s / c
}

/// arctan y for y ≥ 0, by bisection on genoxide's portable tangent.
fn arctan(y: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, PI / 2.0);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if tan(mid) < y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// cos z for complex z.
fn cos(z: c64) -> c64 {
    let (s, c) = math::sin_cos(z.re);
    let (e, f) = (math::exp(z.im), math::exp(-z.im));
    c64::new(c * 0.5 * (e + f), -s * 0.5 * (e - f))
}

/// The descending Landen sequence of the modulus k, to below 1e-16.
fn landen(mut k: f64) -> Vec<f64> {
    let mut v = Vec::new();
    while k > 1e-16 {
        k = math::powi(k / (1.0 + (1.0 - k * k).sqrt()), 2);
        v.push(k);
    }
    v
}

/// Jacobi's cd(uK, k), u complex.
fn cd(u: c64, k: f64) -> c64 {
    let mut w = cos(u * (PI / 2.0));
    for &v in landen(k).iter().rev() {
        w = w * (1.0 + v) / (w * w * v + 1.0);
    }
    w
}

/// Jacobi's sn(uK, k), u real.
fn sn(u: f64, k: f64) -> f64 {
    let mut w = math::sin_cos(u * PI / 2.0).0;
    for &v in landen(k).iter().rev() {
        w = (1.0 + v) * w / (1.0 + v * w * w);
    }
    w
}

/// A digital filter b/a in z⁻¹, and its passband and stopband edges.
struct Filter {
    b: Vec<f64>,
    a: Vec<f64>,
    wp: f64,
    ws: f64,
}

impl Filter {
    /// The elliptic low-pass of even order `n`, ripple `rp` dB, attenuation `rs` dB, passband
    /// edge `wp` rad/sample (see the module's docs).
    fn elliptic(n: usize, rp: f64, rs: f64, wp: f64) -> Filter {
        let ep = (math::exp(rp / 10.0 * LN_10) - 1.0).sqrt();
        let es = (math::exp(rs / 10.0 * LN_10) - 1.0).sqrt();
        let k1 = ep / es;
        let k1p = (1.0 - k1 * k1).sqrt();
        let u: Vec<f64> = (1..=n / 2).map(|i| (2 * i - 1) as f64 / n as f64).collect();
        let kp = math::powi(k1p, n as i32)
            * u.iter()
                .map(|&ui| math::powi(sn(ui, k1p), 4))
                .product::<f64>();
        let k = (1.0 - kp * kp).sqrt();
        // sn⁻¹(j/ε, k₁) down the Landen sequence: w stays j y, and acos(j y) = π/2 − j asinh y
        let mut y = 1.0 / ep;
        let mut previous = k1;
        for v in landen(k1) {
            y = y / (1.0 + (1.0 + y * y * previous * previous).sqrt()) * 2.0 / (1.0 + v);
            previous = v;
        }
        let v0 = 2.0 / PI * ln(y + (y * y + 1.0).sqrt()) / n as f64;
        let scale = tan(wp / 2.0);
        let j = c64::new(0.0, 1.0);
        let (mut zeros, mut poles) = (Vec::new(), Vec::new());
        for &ui in &u {
            let zero = j / (cd(c64::new(ui, 0.0), k) * k) * scale;
            let pole = j * cd(c64::new(ui, -v0), k) * scale;
            for s in [zero, zero.conj()] {
                zeros.push((1.0 + s) / (1.0 - s));
            }
            for s in [pole, pole.conj()] {
                poles.push((1.0 + s) / (1.0 - s));
            }
        }
        let mut b = expand(&zeros);
        let a = expand(&poles);
        let gain = math::exp(-rp / 20.0 * LN_10) * a.iter().sum::<f64>() / b.iter().sum::<f64>();
        for x in &mut b {
            *x *= gain;
        }
        Filter {
            b,
            a,
            wp,
            ws: 2.0 * arctan(scale / k),
        }
    }

    /// |H(e^{jω})|².
    fn power(&self, w: f64) -> f64 {
        let poly = |c: &[f64]| -> c64 {
            c.iter().enumerate().fold(c64::new(0.0, 0.0), |s, (i, &x)| {
                s + cis(-(i as f64) * w) * x
            })
        };
        (poly(&self.b) / poly(&self.a)).norm_sqr()
    }
}

/// The real polynomial in z⁻¹ with roots `roots` (closed under conjugation), leading 1.
fn expand(roots: &[c64]) -> Vec<f64> {
    let mut c = vec![c64::new(1.0, 0.0)];
    for &r in roots {
        let mut next = vec![c64::new(0.0, 0.0); c.len() + 1];
        for (i, &x) in c.iter().enumerate() {
            next[i] += x;
            next[i + 1] -= x * r;
        }
        c = next;
    }
    c.iter().map(|x| x.re).collect()
}

/// The worst passband ripple (below 0 dB) up to `wp` and the least attenuation from `ws`, dB,
/// read on `FINE` frequencies.
fn spec(power: &dyn Fn(f64) -> f64, wp: f64, ws: f64) -> (f64, f64) {
    let (mut ripple, mut stop) = (0.0f64, f64::INFINITY);
    for i in 0..=FINE {
        let w = PI * i as f64 / FINE as f64;
        let d = db(power(w));
        if w <= wp {
            ripple = ripple.max(-d);
        }
        if w >= ws {
            stop = stop.min(-d);
        }
    }
    (ripple, stop)
}

type Two = [[c64; 2]; 2];

fn mul(a: &Two, b: &Two) -> Two {
    let mut c = [[c64::new(0.0, 0.0); 2]; 2];
    for (i, row) in c.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            *x = a[i][0] * b[0][j] + a[i][1] * b[1][j];
        }
    }
    c
}

/// A coupler of power coupling k on (upper, lower).
fn coupler(k: f64) -> Two {
    let t = c64::new((1.0 - k).sqrt(), 0.0);
    let x = c64::new(0.0, k.sqrt());
    [[t, x], [x, t]]
}

/// The ring pair between the arms at round-trip phase ω: ring 1 coupled to the upper arm by
/// `ka` and to ring 2 by `kb`, ring 2 to the lower arm by `kc`. With d = e^(jω/2) half a ring,
/// r₁ and r₂ ring 1's waves out of its two couplers and s₁, s₂ ring 2's,
/// r₁ = j√ka u + ta d r₂, r₂ = tb d r₁ + j√kb d s₂, s₁ = j√kb d r₁ + tb d s₂ and
/// s₂ = tc d s₁ + j√kc l: r₂ and s₁ put in, a 2 × 2 solve for r₁ and s₂.
fn rings(ka: f64, kb: f64, kc: f64, w: f64) -> Two {
    let d = cis(w / 2.0);
    let one = c64::new(1.0, 0.0);
    let (ta, xa) = ((1.0 - ka).sqrt(), c64::new(0.0, ka.sqrt()));
    let (tb, xb) = ((1.0 - kb).sqrt(), c64::new(0.0, kb.sqrt()));
    let (tc, xc) = ((1.0 - kc).sqrt(), c64::new(0.0, kc.sqrt()));
    let m00 = one - d * d * ta * tb;
    let m01 = -(d * d * xb * ta);
    let m10 = -(d * d * xb * tc);
    let m11 = one - d * d * tc * tb;
    let det = m00 * m11 - m01 * m10;
    let mut out = [[c64::new(0.0, 0.0); 2]; 2];
    for (input, (u, l)) in [(one, one * 0.0), (one * 0.0, one)].into_iter().enumerate() {
        let (f0, f1) = (xa * u, xc * l);
        let r1 = (f0 * m11 - m01 * f1) / det;
        let s2 = (m00 * f1 - m10 * f0) / det;
        let r2 = d * tb * r1 + d * xb * s2;
        let s1 = d * xb * r1 + d * tb * s2;
        out[0][input] = u * ta + xa * d * r2;
        out[1][input] = xc * d * s1 + l * tc;
    }
    out
}

/// The circuit's 2 × 2 from its inputs (upper, lower) to its outputs at ω, for the couplings
/// `x[..9]` (k1–k9) and the phases `x[9..]` (φ1–φ8).
fn circuit(x: &[f64], w: f64) -> Two {
    let zero = c64::new(0.0, 0.0);
    let arms = |up: f64, low: f64| [[cis(up), zero], [zero, cis(low)]];
    let (k, phi) = (&x[..9], &x[9..]);
    let mut t = coupler(k[0]);
    for (stage, p) in [(0, 0), (4, 4)] {
        t = mul(&arms(phi[p], phi[p + 2]), &t);
        t = mul(&rings(k[stage + 1], k[stage + 2], k[stage + 3], w), &t);
        t = mul(&arms(phi[p + 1], phi[p + 3]), &t);
        t = mul(&coupler(k[stage + 4]), &t);
    }
    t
}

/// The low-pass (cross) port's power.
fn low_pass(x: &[f64], w: f64) -> f64 {
    circuit(x, w)[1][0].norm_sqr()
}

/// The same circuit as a netlist of photonoxide's couplers, phase shifters and half rings (each
/// a fixed delay of e^(jω/2)), by the circuit solve at ω: its 2 × 2.
fn by_circuit(x: &[f64], w: f64) -> photonoxide::Result<Two> {
    let coupler = Arc::new(Coupler::new());
    let shifter = Arc::new(PhaseShifter::new());
    let d = cis(w / 2.0);
    let zero = c64::new(0.0, 0.0);
    let half: Arc<dyn Component> = Arc::new(Fixed::new(
        "half ring",
        &["a", "b"],
        SMatrix::from_fn(2, |q, p| if q == p { zero } else { d }),
    )?);
    let mut net = Netlist::new();
    let couplers = ["c1", "ca", "cb", "cc", "c5", "cd", "ce", "cf", "c9"];
    for (name, &k) in couplers.iter().zip(&x[..9]) {
        net.add(name, coupler.clone())?;
        net.set(name, "coupling", k)?;
    }
    for (i, &phi) in x[9..].iter().enumerate() {
        let name = format!("p{}", i + 1);
        net.add(&name, shifter.clone())?;
        net.set(&name, "phase", phi)?;
    }
    for i in 1..=8 {
        net.add(&format!("h{i}"), half.clone())?;
    }
    // a coupler's upper guide runs o2 to o3, its lower o1 to o4
    let mut links: Vec<(String, String)> = Vec::new();
    for (s, (left, a, b, c, right)) in [
        ("c1", "ca", "cb", "cc", "c5"),
        ("c5", "cd", "ce", "cf", "c9"),
    ]
    .into_iter()
    .enumerate()
    {
        let p = |i: usize| format!("p{}", 4 * s + i);
        let h = |i: usize| format!("h{}", 4 * s + i);
        for (from, to) in [
            (format!("{left}.o3"), format!("{}.o1", p(1))),
            (format!("{}.o2", p(1)), format!("{a}.o2")),
            (format!("{a}.o3"), format!("{}.o1", p(2))),
            (format!("{}.o2", p(2)), format!("{right}.o2")),
            (format!("{left}.o4"), format!("{}.o1", p(3))),
            (format!("{}.o2", p(3)), format!("{c}.o1")),
            (format!("{c}.o4"), format!("{}.o1", p(4))),
            (format!("{}.o2", p(4)), format!("{right}.o1")),
            // ring 1: the upper arm's coupler's lower guide, half a ring, the ring pair's
            // coupler's upper guide right to left, half a ring back
            (format!("{a}.o4"), format!("{}.a", h(1))),
            (format!("{}.b", h(1)), format!("{b}.o3")),
            (format!("{b}.o2"), format!("{}.a", h(2))),
            (format!("{}.b", h(2)), format!("{a}.o1")),
            // ring 2: the pair's lower guide right to left, half a ring, the lower arm's
            // coupler's upper guide, half a ring back
            (format!("{b}.o1"), format!("{}.a", h(3))),
            (format!("{}.b", h(3)), format!("{c}.o2")),
            (format!("{c}.o3"), format!("{}.a", h(4))),
            (format!("{}.b", h(4)), format!("{b}.o4")),
        ] {
            links.push((from, to));
        }
    }
    for (from, to) in &links {
        net.connect(from, to)?;
    }
    for (name, port) in [
        ("in1", "c1.o2"),
        ("in2", "c1.o1"),
        ("out1", "c9.o3"),
        ("out2", "c9.o4"),
    ] {
        net.expose(name, port)?;
    }
    let circuit = net.compile()?;
    let s = circuit.s_matrix(Wavelength::um(1.55)?)?;
    let at = |name: &str| {
        circuit
            .ports()
            .iter()
            .position(|p| p.name == name)
            .expect("a port")
    };
    let (ins, outs) = ([at("in1"), at("in2")], [at("out1"), at("out2")]);
    let mut t = [[zero; 2]; 2];
    for (q, row) in t.iter_mut().enumerate() {
        for (p, value) in row.iter_mut().enumerate() {
            *value = s[(outs[q], ins[p])];
        }
    }
    Ok(t)
}

/// Nelder–Mead from `start` on Eq. 12 with weights 1 and 100 against `target`.
fn optimize(target: &Filter, start: &[f64]) -> (Vec<f64>, f64) {
    let w = |i: usize| PI * i as f64 / (GRID - 1) as f64;
    let goal: Vec<f64> = (0..GRID).map(|i| target.power(w(i))).collect();
    let mut bounds = vec![0.0..=1.0; 9];
    bounds.extend(vec![-TAU..=TAU; 8]);
    let real = Real::new(bounds).expect("bounds");
    let eq12 = |x: &Reals| {
        let (mut lin, mut log) = (0.0, 0.0);
        for (i, &g) in goal.iter().enumerate() {
            let p = low_pass(x, w(i));
            lin += (p.sqrt() - g.sqrt()).abs();
            log += (db(p).max(-100.0) - db(g).max(-100.0)).abs();
        }
        (lin + 100.0 * log) / GRID as f64
    };
    let nm = NelderMead::builder(real)
        .initial_genome(Reals::from(start.to_vec()))
        .minimize()
        .seed(1)
        .build()
        .expect("a valid Nelder–Mead");
    let outcome = Engine::new(nm, eq12)
        .stop_when(Stop::evaluations(EVALUATIONS))
        .run()
        .expect("a run");
    let x = outcome.best_genome().to_vec();
    let value = eq12(outcome.best_genome());
    (x, value)
}

pub fn main() -> photonoxide::Result<ExitCode> {
    // (passband ripple A_p, stopband attenuation A_s), dB
    let specifications = [(2.0, 40.0), (1.0, 60.0)];
    let mut setup = common::Setup::new();
    setup.item(
        "circuit",
        "Fig. 4's: couplers k1 to k9 and two pairs of rings between MZI arms of phases φ1 to φ8, \
         lossless, solved as 2 × 2 transfer matrices and by photonoxide's circuit solve",
    );
    setup.item("k1 to k9", &FIG4_K.map(common::number).join(", "));
    setup.item(
        "φ1 to φ8",
        &FIG4_PHI
            .map(|p| format!("{}π", common::number(p / PI)))
            .join(", "),
    );
    setup.item(
        "designs",
        &format!(
            "elliptic low-passes of order 4, passband edge 0.5π, {}",
            specifications
                .map(|(rp, rs)| format!("{rp} dB ripple and {rs} dB attenuation"))
                .join("; ")
        ),
    );
    setup.item(
        "frequencies",
        &format!(
            "ω the rings' round-trip phase: {GRID} over 0 to π for the optimizer's target, \
             {FINE} for a response's ripple and attenuation"
        ),
    );
    setup.item("grid", "none: closed forms and the circuit solve, exact");
    setup.item(
        "solver",
        &format!(
            "Nelder–Mead on the paper's Eq. 12 (weights 1 and 100) from Fig. 4's values, \
             {EVALUATIONS} evaluations a target"
        ),
    );
    setup.item(
        "tolerance",
        &format!(
            "each design at its edges to {EDGE_TOLERANCE:e} of A_p and A_s; the 2 × 2 model \
             against the circuit to {CIRCUIT_TOLERANCE:e}"
        ),
    );
    setup.print();
    println!(
        "Wang et al. 2022's double-ring-loaded MZI filter: the fourth-order elliptic low-passes, derived here, against its circuit, exact"
    );
    let mut checks = common::Checks::default();

    // the elliptic designs: their edges exactly at −A_p and −A_s
    let designs = specifications.map(|(rp, rs)| {
        let f = Filter::elliptic(4, rp, rs, PI / 2.0);
        println!(
            "\nElliptic, order 4, {rp} dB ripple, {rs} dB attenuation, passband edge 0.5π: stopband edge {:.6}π",
            f.ws / PI
        );
        let row = |c: &[f64]| {
            c.iter()
                .map(|x| format!("{x:.4}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("  b: {}\n  a: {}", row(&f.b), row(&f.a));
        checks.against(
            &format!("response at ω_p, dB ({rs} dB)"),
            db(f.power(f.wp)),
            "−A_p",
            -rp,
            EDGE_TOLERANCE * rp,
        );
        checks.against(
            &format!("response at ω_s, dB ({rs} dB)"),
            db(f.power(f.ws)),
            "−A_s",
            -rs,
            EDGE_TOLERANCE * rs,
        );
        f
    });
    let [forty, sixty] = &designs;

    // the paper's printed transfer functions
    let eq10 = Filter {
        b: EQ10.0.to_vec(),
        a: EQ10.1.to_vec(),
        wp: forty.wp,
        ws: forty.ws,
    };
    let (ripple, stop) = spec(&|w| eq10.power(w), forty.wp, forty.ws);
    println!(
        "\nEq. 10 as printed: {ripple:.2} dB of ripple, {stop:.2} dB of attenuation from the 40 dB design's stopband edge; {:.2} dB at π",
        -db(eq10.power(PI))
    );
    let product = |i: usize| {
        let mut c = vec![0.0; 5];
        for (p, &x) in [EQ11[0].0, EQ11[0].1][i].iter().enumerate() {
            for (q, &y) in [EQ11[1].0, EQ11[1].1][i].iter().enumerate() {
                c[p + q] += x * y;
            }
        }
        c
    };
    let eq11 = Filter {
        b: product(0),
        a: product(1),
        wp: forty.wp,
        ws: forty.ws,
    };
    let row = |c: &[f64]| {
        c.iter()
            .map(|x| format!("{x:.4}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "  Eq. 11's sections multiplied: b {}, a {}; their peak {:.2} dB. Printed, not checked",
        row(&eq11.b),
        row(&eq11.a),
        db((0..=FINE)
            .map(|i| eq11.power(PI * i as f64 / FINE as f64))
            .fold(0.0, f64::max))
    );

    // the model against photonoxide's circuit solve
    let mut fig4 = FIG4_K.to_vec();
    fig4.extend(FIG4_PHI);
    let mut worst: f64 = 0.0;
    for w in [0.3, 1.1, 2.5] {
        let (ours, solved) = (circuit(&fig4, w), by_circuit(&fig4, w)?);
        for q in 0..2 {
            for p in 0..2 {
                worst = worst.max((ours[q][p] - solved[q][p]).norm_sqr().sqrt());
            }
        }
    }
    println!(
        "\nThe circuit (Fig. 4's values) against photonoxide's circuit solve of it as a netlist, 2 × 2 at 3 frequencies: up to {worst:.0e}"
    );
    checks.against("largest |ΔT|", worst, "round-off", 0.0, CIRCUIT_TOLERANCE);
    let (ripple, stop) = spec(&|w| low_pass(&fig4, w), forty.wp, forty.ws);
    println!(
        "  Fig. 4's circuit: {ripple:.2} dB of ripple, {stop:.2} dB of attenuation from the 40 dB design's stopband edge (paper, Fig. 5: \"around 34 dB extinction ratio\"). Printed, not checked"
    );

    // Nelder–Mead from Fig. 4's values
    println!(
        "\nNelder–Mead from Fig. 4's values on Eq. 12, weights 1 and 100, {EVALUATIONS} evaluations. Printed, not checked"
    );
    for (target, paper) in [
        (forty, "Fig. 6: the 40 dB filter"),
        (sixty, "Fig. 7: 60 dB, 1 dB of ripple"),
    ] {
        let (x, value) = optimize(target, &fig4);
        let (ripple, stop) = spec(&|w| low_pass(&x, w), target.wp, target.ws);
        let (want_ripple, want_stop) = spec(&|w| target.power(w), target.wp, target.ws);
        println!(
            "  towards {want_stop:.0} dB and {want_ripple:.0} dB of ripple: {ripple:.2} dB of ripple, {stop:.2} dB of attenuation, Eq. 12 at {value:.3} ({paper})"
        );
        println!("    k1–k9 {}; φ1–φ8 {}", row(&x[..9]), row(&x[9..]));
    }
    Ok(checks.finish())
}

//! Clements meshes of MZIs fitted to Haar-random unitaries by genoxide's L-BFGS-B and Adam,
//! from Pai et al.'s Haar initialization; and that initialization's statistics.
//!
//! S. Pai, B. Bartlett, O. Solgaard, D. A. B. Miller, Phys. Rev. Applied 11, 064044 (2019),
//! [doi:10.1103/PhysRevApplied.11.064044](https://doi.org/10.1103/PhysRevApplied.11.064044);
//! W. R. Clements et al., Optica 3, 1460 (2016),
//! [doi:10.1364/OPTICA.3.001460](https://doi.org/10.1364/OPTICA.3.001460).
//!
//! **The mesh.** Clements's rectangular arrangement of N(N − 1)/2 MZIs, N columns deep, then a
//! phase on every output: N² phases. Each MZI is a phase φ on its upper input, a 50:50 coupler,
//! a phase θ on its upper arm and a second 50:50 coupler; its cross power is
//! t = cos²(θ/2). Clements showed such a mesh implements any N × N unitary: a fit to any target
//! reaches round-off. The cost is Pai's Eq. 12, (1/2N) ‖Û − U‖², its gradient the matrix
//! model's adjoint (one pass forward, one back), checked against photonoxide's circuit solve
//! and circuit adjoint of the same mesh built as a netlist.
//!
//! **The Haar initialization.** An MZI's sensitivity index α = |I| + |O| − N − 1 counts the
//! inputs I that reach it and the outputs O it reaches (Pai's Section III); over a rectangular
//! mesh its mean is (N + 1)/3. A Haar-random unitary's MZIs have independent θ with
//! t^α uniform (Pai's Eq. 8), so θ = 2 arccos ξ^(1/2α) for ξ uniform, and every other phase
//! uniform, draws a Haar-random mesh (Eq. 9). Checked here by its statistics: a Haar-random
//! unitary's every entry has E|U_qp|⁴ = 2/(N(N + 1)), and the sampled means are checked within
//! 3 standard errors over the mesh and within 4 at each entry. With one more than Pai's α the
//! mean over the mesh hardly moves, but an entry's |z| reaches about 50; with every phase
//! uniform the mean is far off (printed): the entries' test is the one that tells them apart.
//!
//! **The fits.** From the Haar initialization, genoxide's L-BFGS-B fits Haar-random targets
//! (Mezzadri's construction) to round-off, each checked at 1e-20. The phases are free: kept within
//! a phase shifter's ±2π, L-BFGS-B stops against the bounds at misfits about 1e-3; with bounds of
//! ±1000 rad, never reached, it gets to round-off from every start. Adam at Pai's learning rate
//! 0.0025, for 20 000 iterations as in Pai's Figure 6, from the Haar initialization and from
//! uniform phases, is printed, not checked: Pai's figure (N = 128, a plot) has the Haar
//! initialization converging faster, as uniform phases leave so large a mesh nearly banded; at
//! N = 8 and 16 the two start about as well. Exact: no grid.
//!
//! ```sh
//! cargo run --release --example optimize_mesh
//! ```

mod common;

use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::math;
use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::circuit::components::{Coupler, PhaseShifter};
use photonoxide::circuit::{Circuit, Component, Netlist, SMatrix, objective};
use photonoxide::units::Wavelength;
use rayon::prelude::*;

/// The fits' targets for each N.
const TARGETS: u64 = 8;
/// Samples of the Haar initialization's statistics.
const SAMPLES: u64 = 10_000;
/// Adam's iterations, as Pai's Figure 6.
const ADAM: u64 = 20_000;
/// Adam's learning rate, Pai's.
const RATE: f64 = 0.0025;
/// The meshes' sizes.
const SIZES: [usize; 2] = [8, 16];
/// The tolerance on a fit's (1/2N)‖Û − U‖².
const FIT_TOLERANCE: f64 = 1e-20;

/// e^(ix), by genoxide's portable sine and cosine.
fn cis(x: f64) -> c64 {
    let (s, c) = math::sin_cos(x);
    c64::new(c, s)
}

/// An MZI's 2 × 2 matrix in (upper, lower) and its derivatives by θ and φ: couplers
/// (1/√2)[[1, i], [i, 1]], θ on the upper arm, φ on the upper input.
fn mzi(theta: f64, phi: f64) -> [[[c64; 2]; 2]; 3] {
    let (et, ep) = (cis(theta), cis(phi));
    let i = c64::new(0.0, 1.0);
    let zero = c64::new(0.0, 0.0);
    let t = [
        [ep * (et - 1.0) * 0.5, i * (et + 1.0) * 0.5],
        [i * ep * (et + 1.0) * 0.5, (-et + 1.0) * 0.5],
    ];
    let by_theta = [
        [i * ep * et * 0.5, -et * 0.5],
        [-ep * et * 0.5, -i * et * 0.5],
    ];
    let by_phi = [[i * t[0][0], zero], [i * t[1][0], zero]];
    [t, by_theta, by_phi]
}

/// An n × n complex matrix, row-major.
type Matrix = Vec<c64>;

fn identity(n: usize) -> Matrix {
    (0..n * n)
        .map(|k| c64::new(if k % (n + 1) == 0 { 1.0 } else { 0.0 }, 0.0))
        .collect()
}

/// Rows `top` and `top + 1` of the n × n matrix `u` multiplied by the 2 × 2 `t` from the left.
fn rotate(u: &mut [c64], n: usize, top: usize, t: &[[c64; 2]; 2]) {
    for c in 0..n {
        let (a, b) = (u[top * n + c], u[(top + 1) * n + c]);
        u[top * n + c] = t[0][0] * a + t[0][1] * b;
        u[(top + 1) * n + c] = t[1][0] * a + t[1][1] * b;
    }
}

/// A rectangular mesh of `n` modes: in column c the MZIs join modes (k, k + 1) for k = c mod 2,
/// c mod 2 + 2, …; then a phase on every output. Genes: θ and φ of every MZI in order, then the
/// output phases.
struct Mesh {
    n: usize,
    /// Each MZI's upper mode.
    tops: Vec<usize>,
}

impl Mesh {
    fn new(n: usize) -> Mesh {
        let tops = (0..n)
            .flat_map(|column| (column % 2..n - 1).step_by(2))
            .collect();
        Mesh { n, tops }
    }

    fn genes(&self) -> usize {
        self.n * self.n
    }

    /// Each MZI's sensitivity index α = |I| + |O| − N − 1: the inputs reaching it, the outputs
    /// it reaches.
    fn sensitivity(&self) -> Vec<usize> {
        let n = self.n;
        let count = |order: &mut dyn Iterator<Item = (usize, usize)>, out: &mut [usize]| {
            let mut reach: Vec<u128> = (0..n).map(|k| 1 << k).collect();
            for (m, top) in order {
                let joined = reach[top] | reach[top + 1];
                reach[top] = joined;
                reach[top + 1] = joined;
                out[m] = joined.count_ones() as usize;
            }
        };
        let mut from = vec![0; self.tops.len()];
        let mut to = vec![0; self.tops.len()];
        count(&mut self.tops.iter().copied().enumerate(), &mut from);
        count(&mut self.tops.iter().copied().enumerate().rev(), &mut to);
        from.iter().zip(&to).map(|(i, o)| i + o - n - 1).collect()
    }

    /// The output phases' e^(iδ).
    fn screen(&self, x: &[f64]) -> Vec<c64> {
        x[2 * self.tops.len()..].iter().map(|&d| cis(d)).collect()
    }

    /// Û at x.
    fn unitary(&self, x: &[f64]) -> Matrix {
        let n = self.n;
        let mut u = identity(n);
        for (m, &top) in self.tops.iter().enumerate() {
            rotate(&mut u, n, top, &mzi(x[2 * m], x[2 * m + 1])[0]);
        }
        for (q, e) in self.screen(x).iter().enumerate() {
            for z in &mut u[q * n..(q + 1) * n] {
                *z *= e;
            }
        }
        u
    }

    /// (1/2N) ‖Û − U‖² and its gradient. Û = D T_M ⋯ T_1, and for MZI m, Û = A T_m B: a phase p
    /// of it has ∂/∂p = (1/N) Re Σ conj(G) ∂T_m/∂p with G = A^H (Û − U) B^H on its two modes.
    fn cost(&self, target: &[c64], x: &[f64], gradient: &mut [f64]) -> f64 {
        let n = self.n;
        let scale = 1.0 / n as f64;
        let i = c64::new(0.0, 1.0);
        // forward, keeping each MZI's two rows of B, the product before it
        let mut u = identity(n);
        let mut before = Vec::with_capacity(self.tops.len());
        let mut parts = Vec::with_capacity(self.tops.len());
        for (m, &top) in self.tops.iter().enumerate() {
            before.push(u[top * n..(top + 2) * n].to_vec());
            let part = mzi(x[2 * m], x[2 * m + 1]);
            rotate(&mut u, n, top, &part[0]);
            parts.push(part);
        }
        let screen = self.screen(x);
        for (q, e) in screen.iter().enumerate() {
            for z in &mut u[q * n..(q + 1) * n] {
                *z *= e;
            }
        }
        let mut w: Matrix = u.iter().zip(target).map(|(a, b)| a - b).collect();
        let value = 0.5 * scale * w.iter().map(c64::norm_sqr).sum::<f64>();
        let outputs = 2 * self.tops.len();
        for q in 0..n {
            let row = q * n..(q + 1) * n;
            let s: c64 = w[row.clone()]
                .iter()
                .zip(&u[row])
                .map(|(e, z)| e.conj() * i * z)
                .sum();
            gradient[outputs + q] = scale * s.re;
        }
        // back: W = A^H (Û − U), from D^H (Û − U)
        for (q, e) in screen.iter().enumerate() {
            for z in &mut w[q * n..(q + 1) * n] {
                *z *= e.conj();
            }
        }
        for (m, &top) in self.tops.iter().enumerate().rev() {
            let b = &before[m];
            let mut g = [[c64::new(0.0, 0.0); 2]; 2];
            for (a, row) in g.iter_mut().enumerate() {
                for (j, entry) in row.iter_mut().enumerate() {
                    *entry = w[(top + a) * n..(top + a + 1) * n]
                        .iter()
                        .zip(&b[j * n..(j + 1) * n])
                        .map(|(wv, bv)| wv * bv.conj())
                        .sum();
                }
            }
            let [t, by_theta, by_phi] = &parts[m];
            let along = |d: &[[c64; 2]; 2]| -> f64 {
                let mut s = 0.0;
                for a in 0..2 {
                    for j in 0..2 {
                        s += (g[a][j].conj() * d[a][j]).re;
                    }
                }
                scale * s
            };
            gradient[2 * m] = along(by_theta);
            gradient[2 * m + 1] = along(by_phi);
            let adjoint = [
                [t[0][0].conj(), t[1][0].conj()],
                [t[0][1].conj(), t[1][1].conj()],
            ];
            rotate(&mut w, n, top, &adjoint);
        }
        value
    }
}

/// The same mesh as a netlist of photonoxide's ideal couplers and phase shifters, ports `in0`…
/// and `out0`…: the circuit solve and the circuit adjoint to check the matrix model against.
struct Netlisted {
    circuit: Circuit,
    base: Vec<f64>,
    slots: Vec<usize>,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
}

impl Netlisted {
    fn new(mesh: &Mesh) -> photonoxide::Result<Netlisted> {
        let n = mesh.n;
        let coupler = Arc::new(Coupler::new());
        let shifter = Arc::new(PhaseShifter::new());
        let mut net = Netlist::new();
        // what carries each mode's light so far: an output port, or none before its first MZI
        let mut head: Vec<Option<String>> = vec![None; n];
        let feed =
            |net: &mut Netlist, head: &mut Option<String>, mode: usize, port: &str| match head
                .take()
            {
                Some(out) => net.connect(&out, port),
                None => net.expose(&format!("in{mode}"), port),
            };
        for (m, &top) in mesh.tops.iter().enumerate() {
            let [phi, a, theta, b] = ["phi", "a", "theta", "b"].map(|s| format!("{s}{m}"));
            net.add(&phi, shifter.clone())?;
            net.add(&theta, shifter.clone())?;
            for c in [&a, &b] {
                net.add(c, coupler.clone())?;
                net.set(c, "coupling", 0.5)?;
            }
            // a coupler's upper guide runs o2 to o3, its lower o1 to o4
            feed(&mut net, &mut head[top], top, &format!("{phi}.o1"))?;
            feed(&mut net, &mut head[top + 1], top + 1, &format!("{a}.o1"))?;
            for (p, q) in [
                (format!("{phi}.o2"), format!("{a}.o2")),
                (format!("{a}.o3"), format!("{theta}.o1")),
                (format!("{theta}.o2"), format!("{b}.o2")),
                (format!("{a}.o4"), format!("{b}.o1")),
            ] {
                net.connect(&p, &q)?;
            }
            head[top] = Some(format!("{b}.o3"));
            head[top + 1] = Some(format!("{b}.o4"));
        }
        for (mode, h) in head.iter_mut().enumerate() {
            let d = format!("d{mode}");
            net.add(&d, shifter.clone())?;
            feed(&mut net, h, mode, &format!("{d}.o1"))?;
            net.expose(&format!("out{mode}"), &format!("{d}.o2"))?;
        }
        let circuit = net.compile()?;
        let at = |name: String| circuit.parameter(&name).expect("a parameter");
        let mut slots = Vec::new();
        for m in 0..mesh.tops.len() {
            slots.push(at(format!("theta{m}.phase")));
            slots.push(at(format!("phi{m}.phase")));
        }
        slots.extend((0..n).map(|q| at(format!("d{q}.phase"))));
        let port = |name: String| {
            circuit
                .ports()
                .iter()
                .position(|p| p.name == name)
                .expect("a port")
        };
        let inputs = (0..n).map(|k| port(format!("in{k}"))).collect();
        let outputs = (0..n).map(|k| port(format!("out{k}"))).collect();
        Ok(Netlisted {
            base: circuit.values().to_vec(),
            circuit,
            slots,
            inputs,
            outputs,
        })
    }

    /// The circuit solve's Û at x and, by the circuit adjoint, the cost's gradient against
    /// `target`; each phase wrapped into [−π, π), within the shifters' range.
    fn solve(&self, target: &[c64], x: &[f64]) -> photonoxide::Result<(Matrix, Vec<f64>)> {
        let n = self.inputs.len();
        let lambda = Wavelength::um(1.55)?;
        let mut values = self.base.clone();
        for (&s, &g) in self.slots.iter().zip(x) {
            values[s] = g - TAU * ((g + PI) / TAU).floor();
        }
        let s = self.circuit.s_matrix_with(lambda, &values)?;
        let u = (0..n * n)
            .map(|k| s[(self.outputs[k / n], self.inputs[k % n])])
            .collect();
        // the S-matrix carries Û both ways (out from in, and its transpose back), so its misfit
        // counts each entry twice
        let mut whole = SMatrix::zeros(2 * n);
        for (k, &t) in target.iter().enumerate() {
            let (q, p) = (self.outputs[k / n], self.inputs[k % n]);
            whole[(q, p)] = t;
            whole[(p, q)] = t;
        }
        let (_, g) = self.circuit.gradient(&[lambda], &values, |s| {
            objective::matrix_error(s, std::slice::from_ref(&whole))
        })?;
        let scale = 1.0 / (4.0 * n as f64);
        Ok((u, self.slots.iter().map(|&s| g[s] * scale).collect()))
    }
}

/// Uniform numbers in [0, 1] from a seeded generator.
struct Draw(StreamRng);

impl Draw {
    fn new(seed: u64) -> Draw {
        Draw(StreamRng::seed_from_u64(seed))
    }

    fn uniform(&mut self, k: usize) -> Vec<f64> {
        Real::new(vec![0.0..=1.0; k])
            .expect("a range")
            .random_genome(&mut self.0)
            .to_vec()
    }

    /// k standard complex normal numbers (Box–Muller on genoxide's portable ln and sin_cos).
    fn complex_normal(&mut self, k: usize) -> Vec<c64> {
        self.uniform(2 * k)
            .chunks(2)
            .map(|w| cis(TAU * w[1]) * (-ln(1.0 - w[0])).sqrt())
            .collect()
    }
}

/// ln x, by genoxide's portable logarithm, kept finite at 0.
fn ln(x: f64) -> f64 {
    math::ln(x.max(f64::MIN_POSITIVE))
}

/// A Haar-random n × n unitary, row-major: the Q of a complex Gaussian matrix's QR
/// decomposition with R's diagonal positive (F. Mezzadri, Notices AMS 54, 592 (2007),
/// arXiv:math-ph/0609050), by modified Gram–Schmidt on its columns.
fn haar(n: usize, draw: &mut Draw) -> Matrix {
    let mut columns: Vec<Vec<c64>> = (0..n).map(|_| draw.complex_normal(n)).collect();
    for p in 0..n {
        let norm = columns[p].iter().map(c64::norm_sqr).sum::<f64>().sqrt();
        for z in &mut columns[p] {
            *z /= norm;
        }
        let (done, rest) = columns.split_at_mut(p + 1);
        for later in rest {
            let dot: c64 = done[p]
                .iter()
                .zip(later.iter())
                .map(|(a, b)| a.conj() * b)
                .sum();
            for (z, a) in later.iter_mut().zip(&done[p]) {
                *z -= dot * a;
            }
        }
    }
    (0..n * n).map(|k| columns[k % n][k / n]).collect()
}

/// x in [0, π/2] with cos x = c, by bisection on genoxide's portable cosine.
fn arccos(c: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, FRAC_PI_2);
    for _ in 0..64 {
        let mid = 0.5 * (lo + hi);
        if math::sin_cos(mid).1 > c {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Pai's Haar initialization (Eq. 9) with sensitivity indices `alpha`: θ = 2 arccos ξ^(1/2α)
/// for ξ uniform; φ and the output phases uniform in [0, 2π].
fn haar_start(alpha: &[usize], genes: usize, draw: &mut Draw) -> Vec<f64> {
    let u = draw.uniform(genes);
    let mut x: Vec<f64> = u.iter().map(|v| TAU * v).collect();
    for (m, &a) in alpha.iter().enumerate() {
        x[2 * m] = 2.0 * arccos(math::exp(ln(1.0 - u[2 * m]) / (2.0 * a as f64)));
    }
    x
}

/// Every phase uniform in [0, 2π].
fn uniform_start(genes: usize, draw: &mut Draw) -> Vec<f64> {
    draw.uniform(genes).into_iter().map(|u| TAU * u).collect()
}

/// Mean and standard error.
fn mean_se(v: &[f64]) -> (f64, f64) {
    let count = v.len() as f64;
    let m = v.iter().sum::<f64>() / count;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (count - 1.0);
    (m, (var / count).sqrt())
}

/// What draws a mesh's phases.
type Start<'a> = &'a (dyn Fn(&mut Draw) -> Vec<f64> + Sync);

/// E|U_qp|⁴ of meshes drawn by `start`: the mean over the mesh with its standard error, and
/// the largest |z| of an entry's mean against `value`.
fn fourth_moments(mesh: &Mesh, start: Start, value: f64) -> (f64, f64, f64) {
    let all: Vec<Vec<f64>> = (0..SAMPLES)
        .into_par_iter()
        .map(|s| {
            let x = start(&mut Draw::new(10_000 + s));
            mesh.unitary(&x)
                .iter()
                .map(|z| z.norm_sqr() * z.norm_sqr())
                .collect()
        })
        .collect();
    let entries = mesh.genes() as f64;
    let over: Vec<f64> = all
        .iter()
        .map(|v| v.iter().sum::<f64>() / entries)
        .collect();
    let (m, se) = mean_se(&over);
    let worst = (0..mesh.genes())
        .map(|k| {
            let (mk, sk) = mean_se(&all.iter().map(|v| v[k]).collect::<Vec<_>>());
            ((mk - value) / sk).abs()
        })
        .fold(0.0, f64::max);
    (m, se, worst)
}

/// A fit of the mesh to `target` from `start`, by Adam or L-BFGS-B: its evaluations and cost.
fn fit(mesh: &Mesh, target: &[c64], start: Vec<f64>, adam: bool) -> (u64, f64) {
    let real = Real::new(vec![-1e3..=1e3; mesh.genes()]).expect("bounds");
    let fitness = |x: &Reals, gradient: &mut [f64]| mesh.cost(target, x, gradient);
    let outcome = if adam {
        let adam = FirstOrder::builder(real)
            .step(first_order::Step::adam(RATE))
            .initial_genome(Reals::from(start))
            .minimize()
            .build()
            .expect("a valid Adam");
        Engine::new(adam, Differentiable(&fitness))
            .stop_when(Stop::evaluations(ADAM))
            .run()
    } else {
        let lbfgsb = Lbfgsb::builder(real)
            .initial_genome(Reals::from(start))
            .gradient_tolerance(0.0)
            .function_tolerance(0.0)
            .minimize()
            .build()
            .expect("a valid L-BFGS-B");
        Engine::new(lbfgsb, Differentiable(&fitness))
            .stop_when(Stop::target(1e-24).or(Stop::evaluations(50_000)))
            .run()
    }
    .expect("a run");
    let cost = mesh.cost(target, outcome.best_genome(), &mut vec![0.0; mesh.genes()]);
    (outcome.evaluations(), cost)
}

/// The median of `v`.
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let k = v.len();
    if k % 2 == 1 {
        v[k / 2]
    } else {
        0.5 * (v[k / 2 - 1] + v[k / 2])
    }
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let mut setup = common::Setup::new();
    setup.item(
        "meshes",
        &format!(
            "Clements meshes of N = {} modes: N(N − 1)/2 MZIs, each two 50:50 couplers with \
             phases θ and φ, and N output phases; lossless",
            SIZES.map(|n| n.to_string()).join(" and ")
        ),
    );
    setup.item(
        "targets",
        &format!(
            "{TARGETS} Haar-random unitaries for each N; the initialization's statistics over \
             {SAMPLES} meshes"
        ),
    );
    setup.item(
        "grid",
        "none: the meshes' matrix model, checked against the circuit solve, exact",
    );
    setup.item(
        "solver",
        &format!(
            "(1/2N)‖Û − U‖² by L-BFGS-B, and by Adam at a learning rate of {RATE} for {ADAM} \
             iterations, from Pai's Haar initialization"
        ),
    );
    setup.item(
        "tolerance",
        &format!(
            "the mean sensitivity index exactly; E|U_qp|⁴ within 3 standard errors over the \
             mesh and 4 at each entry; every fit to {FIT_TOLERANCE:e}"
        ),
    );
    setup.print();
    println!(
        "Clements meshes (Clements et al. 2016) fitted to Haar-random unitaries from Pai et al. 2019's Haar initialization: the matrix model, exact: no grid"
    );
    let mut checks = common::Checks::default();
    for n in SIZES {
        let mesh = Mesh::new(n);
        let genes = mesh.genes();
        let alpha = mesh.sensitivity();
        println!("\nN = {n}: {} MZIs, {genes} phases", alpha.len());
        let mean = alpha.iter().sum::<usize>() as f64 / alpha.len() as f64;
        checks.against(
            "mean sensitivity index",
            mean,
            "Pai, (N + 1)/3",
            (n + 1) as f64 / 3.0,
            1e-12,
        );

        // the matrix model against the circuit solve and the circuit adjoint of the same mesh
        let netlisted = Netlisted::new(&mesh)?;
        let target = haar(n, &mut Draw::new(999));
        let x = uniform_start(genes, &mut Draw::new(998));
        let mut gradient = vec![0.0; genes];
        mesh.cost(&target, &x, &mut gradient);
        let (u, by_circuit) = netlisted.solve(&target, &x)?;
        let entries = mesh
            .unitary(&x)
            .iter()
            .zip(&u)
            .filter(|(a, b)| (*a - *b).norm() <= 1e-12)
            .count();
        let largest = by_circuit.iter().fold(0.0, |m: f64, g| m.max(g.abs()));
        let agreeing = gradient
            .iter()
            .zip(&by_circuit)
            .filter(|(a, b)| (*a - *b).abs() <= 1e-10 * largest)
            .count();
        checks.against(
            "Û's entries agreeing to 1e-12",
            entries as f64,
            "the circuit solve's",
            genes as f64,
            0.0,
        );
        checks.against(
            "gradient's agreeing to 1e-10",
            agreeing as f64,
            "the circuit adjoint's",
            genes as f64,
            0.0,
        );

        // the Haar initialization's statistics
        let value = 2.0 / (n * (n + 1)) as f64;
        let (m, se, worst) = fourth_moments(&mesh, &|d| haar_start(&alpha, genes, d), value);
        println!(
            "  Haar-initialized meshes, {SAMPLES} of them: E|U_qp|⁴ = {m:.6} ± {se:.6} over the mesh, a Haar-random unitary's 2/(N(N + 1)) = {value:.6}"
        );
        checks.against(
            "E|U_qp|⁴ over the mesh",
            m,
            "Haar, ±3 s.e.",
            value,
            // three standard errors, rounded up to print short
            (3e7 * se).ceil() / 1e7,
        );
        checks.against(
            "largest |z| of an entry's E|U_qp|⁴",
            worst,
            "Haar",
            0.0,
            4.0,
        );
        let plus: Vec<usize> = alpha.iter().map(|a| a + 1).collect();
        let (mp, sp, worst_plus) = fourth_moments(&mesh, &|d| haar_start(&plus, genes, d), value);
        let (mu, su, worst_uniform) = fourth_moments(&mesh, &|d| uniform_start(genes, d), value);
        println!(
            "  with α + 1 instead: {mp:.6} ± {sp:.6}, an entry's |z| up to {worst_plus:.0}; every phase uniform: {mu:.6} ± {su:.6}, up to {worst_uniform:.0e}"
        );

        // fits to Haar-random targets
        let targets: Vec<Matrix> = (0..TARGETS)
            .map(|k| haar(n, &mut Draw::new(1000 + k)))
            .collect();
        let fits = |haar_init: bool, adam: bool| -> Vec<(u64, f64)> {
            targets
                .par_iter()
                .enumerate()
                .map(|(k, target)| {
                    let mut draw = Draw::new(k as u64);
                    let start = if haar_init {
                        haar_start(&alpha, genes, &mut draw)
                    } else {
                        uniform_start(genes, &mut draw)
                    };
                    fit(&mesh, target, start, adam)
                })
                .collect()
        };
        let lbfgsb = fits(true, false);
        println!(
            "  L-BFGS-B from the Haar initialization, {TARGETS} targets: {} evaluations",
            lbfgsb
                .iter()
                .map(|(evaluations, _)| evaluations.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        checks.against(
            "the fits' largest (1/2N)‖Û − U‖²",
            lbfgsb.iter().map(|r| r.1).fold(0.0, f64::max),
            "round-off",
            0.0,
            FIT_TOLERANCE,
        );
        let from_uniform: Vec<String> = fits(false, false)
            .iter()
            .map(|(evaluations, cost)| {
                if *cost <= 1e-20 {
                    format!("{evaluations}")
                } else {
                    format!("{evaluations} (stopped at {cost:.0e})")
                }
            })
            .collect();
        println!(
            "  L-BFGS-B from uniform phases instead: {} evaluations",
            from_uniform.join(", ")
        );
        let costs = |v: Vec<(u64, f64)>| median(v.into_iter().map(|r| r.1).collect());
        println!(
            "  Adam (learning rate 0.0025, {ADAM} iterations), the median cost: from the Haar initialization {:.1e}, from uniform phases {:.1e}",
            costs(fits(true, true)),
            costs(fits(false, true))
        );
    }
    Ok(checks.finish())
}

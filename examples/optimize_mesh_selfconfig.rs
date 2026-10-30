//! A 36-unit hexagonal waveguide mesh set to its all-cross state by a genetic algorithm and a
//! particle swarm, without calibration, as in Pérez-López et al.'s self-configuration.
//!
//! D. Pérez-López, A. López, P. DasMahapatra, J. Capmany, "Multipurpose self-configuration of
//! programmable photonic circuits", Nat. Commun. 11, 6359 (2020),
//! [doi:10.1038/s41467-020-19608-w](https://doi.org/10.1038/s41467-020-19608-w), and its
//! Supplementary Information (Supplementary Notes 1, 4 and 6). Pages below are the article's
//! ("p.") and the supplement's ("Suppl. p.").
//!
//! **The mesh.** Fig. 3a (p. 5; the same drawing, Suppl. Fig. 17a, Suppl. p. 18): seven
//! hexagonal cells of tunable basic units (TBUs), 30 units, and six more at the edge (C1, C4, A3,
//! A7, B11, B12): 36 units, 72 phase shifters, 24 ports. Each unit is a Mach-Zehnder
//! interferometer: a 50:50 coupler, a phase shifter on each arm (φ_u, φ_l), a second 50:50
//! coupler, so its bar amplitude is (e^(iφ_u) − e^(iφ_l))/2 and it is in cross when φ_u = φ_l.
//! Where three units meet, each one's guide on a side joins the next unit's guide on that side;
//! where a unit is missing, those two guides are ports. Built that way from the drawing, the
//! all-cross state routes exactly the paper's twelve port pairs (Fig. 3b), which the example
//! checks.
//!
//! **What a mesh is unknown by** (Methods, p. 9; Suppl. Note 1, Suppl. p. 2): each unit's loss,
//! normal with mean 0.15 dB and standard deviation 0.05 dB, and each phase shifter's passive
//! phase, uniform over 2π (the Methods say "a normal distribution between 0 and 2π"; a phase
//! drawn over the circle is uniform). The couplers are 50:50: the supplement puts the couplers'
//! fabrication errors in that random passive state (Suppl. Note 2, Suppl. p. 2), and the Notes
//! give no other coupler model for this study. Every unit has the same length, one basic unit
//! delay (BUD), so the response is periodic in frequency; it is sampled at 8 points over that
//! period (Fig. 3c's axis, ±0.5 of it). The thermal crosstalk is Eq. 5 (p. 9): the phases the
//! shifters reach are a matrix with ones on its diagonal times the phases set. Fig. 3's runs
//! have none. Fig. 3's swarm with it, each coefficient uniform from 0 to 5 % between every pair
//! of shifters as for Fig. 4 (p. 6; Suppl. p. 35), on 20 meshes as Fig. 4's statistics, is
//! printed at the end, not checked: the paper reports no all-cross run with crosstalk.
//!
//! **The cost** (Eq. 2, p. 5; Suppl. Eq. 9, Suppl. p. 18): CF = −(1/N) Σ log |H_ch| over the
//! N = 12 channels (Suppl. p. 20: "the 12 optical channels"), |H_ch| the largest field
//! amplitude over the period. The logarithm is the natural one: Fig. 3d's final CF, about 0.3,
//! is −ln of Fig. 3c's −2.3 dB (a common logarithm would give 0.12). The output feature OF1 is
//! the power averaged over the channels and the period, in dB; a run succeeds when it ends
//! above −3 dB (p. 5: "an average error better than 3-dB"). An operation is one evaluation
//! (Suppl. Note 1, Suppl. p. 2).
//!
//! **The optimizers**, genoxide's, with the paper's settings, each on the same 100 meshes. The
//! particle swarm of Fig. 3 (p. 5; Suppl. Note 4, Suppl. p. 7): 108 particles (150 % of the 72
//! phases), cognitive 0.7, social 1.8, the inertia falling linearly from 0.9 to 0.35 over the
//! first 1000 operations and 0.35 after; 37 × 108 = 3996 operations, within p. 5's 4000. The
//! supplement's swarm (Suppl. Table 18, Suppl. p. 22): 108 particles, inertia 0.5 (no schedule
//! is given for it), cognitive 0.8, social 1.0, within 3000 operations: 27 × 108 = 2916. The
//! genetic algorithm (p. 5; Suppl. Note 4, Suppl. p. 6; Suppl. Table 15, Suppl. p. 20): 144
//! genomes (200 % of the phases), the best 18 (25 % of the phases) the parents, kept, and 126
//! children of them by one-point crossover; in each child 36 phases (50 %) move by a uniform
//! step within ±r, r = 0.5π e^(−0.5 t) (Table 15's initial range and decay); within 3000
//! operations, 144 + 22 × 126 = 2916. The text's "20 % mating" is Table 15's 25 % of the phases,
//! 12.5 % of the population. Phases are within [0, 2π]; the genetic algorithm's steps wrap
//! around the circle, as a phase does, and the swarm stops at the bounds. Gradient descent
//! with momentum (Suppl. Eqs. 5 and 7, Suppl. pp. 7–8 and 23), genoxide's: the gradient by
//! central differences of 0.3 mrad (Suppl. p. 7), 1 + 2 × 72 = 145 operations an iteration,
//! momentum 0.5, from uniform phases until CF is below 0.2 (Suppl. p. 23's "acceptable") or 60
//! iterations (its limit), on 20 meshes. The supplement raises the momentum by 0.05 an
//! iteration up to 0.5, which genoxide's step rule doesn't do: here it is 0.5 from the start.
//! The paper's learning rate is 50 (p. 5; Suppl. p. 23), but Suppl. Fig. 23's axis runs from
//! 10⁻¹ to 10⁰, and the step a rate makes depends on how the CF is scaled, which the paper
//! doesn't give: both 50 and 1 are run.
//!
//! **The checks.** The paper ran each method on 100 meshes and reports the share above −3 dB:
//! 76.66 % for Fig. 3's swarm (p. 5), 70 % for Table 18's (Suppl. p. 22), 95 % for the genetic
//! algorithm (p. 5 and Suppl. p. 20). A share of 100 runs is a binomial count, so ours and the
//! paper's differ by chance with a standard deviation of √(2 p(1 − p)/100) at the paper's p, and
//! a share is checked within two of them. (76.66 % isn't a whole number of 100 runs.) Fig. 3's
//! swarm is checked: 79 of 100. Table 18's swarm ends 2.0 standard deviations below the paper,
//! 57 against 70: printed, not checked. The genetic algorithm hangs on what t counts, which the
//! paper doesn't say: over generations, its range is 0.5π e^(−11) by the last and 4 of 100
//! meshes succeed; over thousands of operations, 99. Both are printed, neither checked. The
//! paper's descent converges in 5 to 6 iterations, 750 to 900 operations, at a rate of 50
//! (Suppl. p. 23); here a rate of 50 overshoots on every mesh and never gets there, while 1 does
//! on 19 of 20 in a median of 18 iterations: printed, not checked. Fig. 3's
//! curves (c, i) end near −2.3 dB; ours average −2.51 dB for Fig. 3's swarm, and −0.90 dB with
//! every unit exactly in cross, the meshes' loss alone. The drawing's all-cross routes are
//! checked against Fig. 3b's pairs, and our solve of the mesh against photonoxide's circuit
//! solve of the same mesh built as a netlist, to round-off.
//!
//! The mesh is solved exactly: one sparse LU of its 120 inner waves per frequency, in plain
//! arithmetic and genoxide's portable sine, cosine, exponential and logarithm, so a run gives
//! the same bits on every system and any number of threads.
//!
//! ```sh
//! cargo run --release --example optimize_mesh_selfconfig
//! ```

mod common;

use std::f64::consts::{LN_10, PI, TAU};
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::math;
use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::circuit::{Component, Fixed, Netlist, SMatrix};
use photonoxide::units::Wavelength;
use rayon::prelude::*;

/// Tunable basic units: A1–A12, B1–B12, C1–C12 (Fig. 3a).
const UNITS: usize = 36;
/// Phase shifters, two a unit: the genes.
const GENES: usize = 2 * UNITS;
/// The ports, numbered 1 to 24 as in Fig. 3a.
const PORTS: usize = 24;
/// Meshes per method (p. 5: "we repeated 100 times").
const RUNS: u64 = 100;
/// Meshes with crosstalk per method (Fig. 4's statistics are of 20, Suppl. p. 35).
const CROSSTALK_RUNS: u64 = 20;
/// Frequencies over one period of the response.
const SAMPLES: usize = 8;
/// The channels of the all-cross state (Fig. 3b, p. 5; Suppl. p. 18).
const CHANNELS: [(usize, usize); 12] = [
    (12, 23),
    (14, 21),
    (10, 1),
    (8, 3),
    (13, 6),
    (15, 4),
    (17, 2),
    (19, 24),
    (11, 16),
    (9, 18),
    (7, 20),
    (5, 22),
];
/// A unit's loss: mean and standard deviation, dB (Methods, p. 9).
const LOSS_DB: (f64, f64) = (0.15, 0.05);
/// The largest crosstalk coefficient of the printed study (p. 6; Suppl. p. 35).
const CROSSTALK: f64 = 0.05;

/// The genetic algorithm (p. 5; Suppl. Table 15, Suppl. p. 20).
const GA_POPULATION: usize = 144;
const GA_PARENTS: usize = 18;
const GA_MUTATED: usize = 36;
const GA_RANGE: f64 = 0.5 * PI;
const GA_DECAY: f64 = 0.5;
/// Generations after the first, within 3000 operations: 144 + 22 × 126 = 2916.
const GA_GENERATIONS: u64 = 22;
/// The particle swarm (p. 5; Suppl. Note 4, Suppl. p. 7).
const PSO_PARTICLES: usize = 108;
const PSO_COGNITIVE: f64 = 0.7;
const PSO_SOCIAL: f64 = 1.8;
const PSO_INERTIA: (f64, f64) = (0.9, 0.35);
/// The operations over which the inertia falls (Suppl. p. 7).
const PSO_SCHEDULE: f64 = 1000.0;
/// Gradient descent with momentum (Suppl. p. 23): the central differences' step, rad (Suppl.
/// p. 7), the momentum, the "acceptable" CF it runs to, and the most iterations.
const GD_STEP: f64 = 0.3e-3;
const GD_MOMENTUM: f64 = 0.5;
const GD_TARGET: f64 = 0.2;
const GD_ITERATIONS: u64 = 60;
/// Meshes per learning rate.
const GD_RUNS: u64 = 20;
/// The learning rates: the text's 50 (p. 5 and Suppl. p. 23), and 1, the top of Suppl. Fig. 23's
/// axis (10⁻¹ to 10⁰).
const GD_RATES: [f64; 2] = [50.0, 1.0];

/// e^(ix), by genoxide's portable sine and cosine.
fn cis(x: f64) -> c64 {
    let (s, c) = math::sin_cos(x);
    c64::new(c, s)
}

/// ln x, by genoxide's portable logarithm, kept finite at 0.
fn ln(x: f64) -> f64 {
    math::ln(x.max(f64::MIN_POSITIVE))
}

/// 10 log₁₀ x.
fn db(x: f64) -> f64 {
    10.0 * ln(x) / LN_10
}

/// A unit's end: (unit, end). An A unit's end 0 is its lower left, a B unit's its left, a C
/// unit's its upper left.
type End = (usize, usize);

fn a(k: usize, end: usize) -> End {
    (k - 1, end)
}

fn b(k: usize, end: usize) -> End {
    (11 + k, end)
}

fn c(k: usize, end: usize) -> End {
    (23 + k, end)
}

/// The unit's name in Fig. 3a.
fn unit_name(t: usize) -> String {
    format!("{}{}", ['A', 'B', 'C'][t / 12], t % 12 + 1)
}

/// The direction from an end along its unit, in sixths of a turn counterclockwise from east.
fn direction((t, end): End) -> usize {
    [[1, 4], [0, 3], [5, 2]][t / 12][end]
}

/// The meeting points of Fig. 3a's drawing: the ends that meet there (a free end alone).
fn meetings() -> Vec<Vec<End>> {
    let (lo, hi, l, r) = (0, 1, 0, 1);
    let (c_hi, c_lo) = (0, 1);
    vec![
        vec![b(1, l), a(1, hi), c(1, c_lo)],
        vec![b(1, r), c(5, c_hi), a(4, lo)],
        vec![a(1, lo), c(2, c_hi)],
        vec![b(2, l), a(2, hi), c(2, c_lo)],
        vec![b(2, r), c(6, c_hi), a(5, lo)],
        vec![a(2, lo), c(3, c_hi)],
        vec![b(3, l), a(3, hi), c(3, c_lo)],
        vec![b(3, r), c(7, c_hi), a(6, lo)],
        vec![b(4, l), a(4, hi), c(4, c_lo)],
        vec![b(4, r), c(8, c_hi)],
        vec![b(5, l), a(5, hi), c(5, c_lo)],
        vec![b(5, r), c(9, c_hi), a(8, lo)],
        vec![b(6, l), a(6, hi), c(6, c_lo)],
        vec![b(6, r), c(10, c_hi), a(9, lo)],
        vec![b(7, l), a(7, hi), c(7, c_lo)],
        vec![b(7, r), a(10, lo)],
        vec![b(8, l), a(8, hi), c(8, c_lo)],
        vec![b(8, r), c(11, c_hi)],
        vec![b(9, l), a(9, hi), c(9, c_lo)],
        vec![b(9, r), c(12, c_hi), a(11, lo)],
        vec![b(10, l), a(10, hi), c(10, c_lo)],
        vec![b(10, r), a(12, lo)],
        vec![b(11, l), a(11, hi), c(11, c_lo)],
        vec![b(12, l), a(12, hi), c(12, c_lo)],
        vec![c(1, c_hi)],
        vec![c(4, c_hi)],
        vec![a(3, lo)],
        vec![a(7, lo)],
        vec![b(11, r)],
        vec![b(12, r)],
    ]
}

/// The ports' numbers in Fig. 3a: (end, side, number), side 0 the guide clockwise of the unit
/// seen from its end, 1 the one counterclockwise.
fn port_numbers() -> Vec<(End, usize, usize)> {
    let (lo, r, c_hi) = (0, 1, 0);
    vec![
        (c(1, c_hi), 0, 12),
        (c(1, c_hi), 1, 13),
        (c(4, c_hi), 0, 14),
        (c(4, c_hi), 1, 15),
        (a(3, lo), 0, 6),
        (a(3, lo), 1, 7),
        (a(7, lo), 0, 4),
        (a(7, lo), 1, 5),
        (b(11, r), 0, 20),
        (b(11, r), 1, 21),
        (b(12, r), 0, 22),
        (b(12, r), 1, 23),
        (a(1, lo), 1, 11),
        (c(2, c_hi), 0, 10),
        (a(2, lo), 1, 9),
        (c(3, c_hi), 0, 8),
        (b(4, r), 0, 16),
        (c(8, c_hi), 1, 17),
        (b(8, r), 0, 18),
        (c(11, c_hi), 1, 19),
        (b(7, r), 1, 3),
        (a(10, lo), 0, 2),
        (b(10, r), 1, 1),
        (a(12, lo), 0, 24),
    ]
}

/// A unit's port: 4t + 2·end + side.
fn port((t, end): End, side: usize) -> usize {
    4 * t + 2 * end + side
}

/// The arm a port's guide is: 0 (u) for the guide left of the unit seen from its end 0, else 1
/// (l). It is counterclockwise at end 0 and clockwise at end 1.
fn arm(p: usize) -> usize {
    usize::from((p / 2) % 2 == p % 2)
}

/// The ports of the unit's other end than port p's.
fn across(p: usize) -> [usize; 2] {
    let base = 4 * (p / 4) + 2 * (1 - (p / 2) % 2);
    [base, base + 1]
}

/// What a unit's port joins.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Link {
    /// Another unit's port.
    Unit(usize),
    /// A port of the mesh, numbered as in Fig. 3a.
    Port(usize),
}

/// The mesh's connections, from Fig. 3a.
struct Mesh {
    links: Vec<Link>,
    /// Each port's unit port, by its number (index 0 unused).
    ports: [usize; PORTS + 1],
}

impl Mesh {
    fn new() -> Mesh {
        let mut links = vec![None; 4 * UNITS];
        for meeting in meetings() {
            let mut ends = meeting.clone();
            ends.sort_by_key(|&e| direction(e));
            for (k, &e) in ends.iter().enumerate() {
                let next = ends[(k + 1) % ends.len()];
                // neighbours a third of a turn apart: e's counterclockwise guide joins next's
                // clockwise one
                if ends.len() > 1 && (direction(next) + 6 - direction(e)) % 6 == 2 {
                    let (p, q) = (port(e, 1), port(next, 0));
                    links[p] = Some(Link::Unit(q));
                    links[q] = Some(Link::Unit(p));
                }
            }
        }
        let mut ports = [usize::MAX; PORTS + 1];
        for (e, side, n) in port_numbers() {
            let p = port(e, side);
            assert!(links[p].is_none(), "port {n} is a joined guide");
            links[p] = Some(Link::Port(n));
            ports[n] = p;
        }
        let links = links
            .into_iter()
            .map(|l| l.expect("every guide joined or a port"))
            .collect();
        Mesh { links, ports }
    }

    /// Where light from port `n` leaves with every unit in cross, and the units it crosses.
    fn cross_route(&self, n: usize) -> (usize, usize) {
        let mut p = self.ports[n];
        let mut units = 0;
        loop {
            units += 1;
            // cross: the same side at the other end
            let out = across(p)[p % 2];
            match self.links[out] {
                Link::Port(m) => return (m, units),
                Link::Unit(q) => p = q,
            }
        }
    }
}

/// One mesh's unknowns: each unit's loss (an amplitude) and passive phases, and the crosstalk.
struct Draw {
    /// e^(−loss/20 · ln 10) per unit.
    gain: Vec<f64>,
    /// Each phase shifter's passive phase.
    offset: Vec<f64>,
    /// The crosstalk matrix's off-diagonal coefficients, row-major (none: the identity).
    crosstalk: Option<Vec<f64>>,
}

/// Uniform numbers in [0, 1] from a seeded generator.
struct Uniform(StreamRng);

impl Uniform {
    fn new(seed: u64) -> Uniform {
        Uniform(StreamRng::seed_from_u64(seed))
    }

    fn take(&mut self, k: usize) -> Vec<f64> {
        Real::uniform(k, 0.0..=1.0)
            .expect("a range")
            .random_genome(&mut self.0)
            .to_vec()
    }
}

impl Draw {
    /// The mesh drawn from `seed`, with crosstalk up to `crosstalk` if any.
    fn new(seed: u64, crosstalk: Option<f64>) -> Draw {
        let mut u = Uniform::new(seed);
        // Box–Muller on genoxide's portable logarithm, sine and cosine
        let gain = u
            .take(2 * UNITS)
            .chunks(2)
            .map(|w| {
                let normal = (-2.0 * ln(1.0 - w[0])).sqrt() * math::sin_cos(TAU * w[1]).1;
                let loss = (LOSS_DB.0 + LOSS_DB.1 * normal).max(0.0);
                math::exp(-loss / 20.0 * LN_10)
            })
            .collect();
        let offset = u.take(GENES).iter().map(|v| TAU * v).collect();
        let crosstalk = crosstalk.map(|most| {
            let mut m: Vec<f64> = u.take(GENES * GENES).iter().map(|v| most * v).collect();
            for k in 0..GENES {
                m[k * GENES + k] = 0.0;
            }
            m
        });
        Draw {
            gain,
            offset,
            crosstalk,
        }
    }

    /// The phases the shifters reach when set to `v`: the crosstalk's sum, plus the passive
    /// phases.
    fn phases(&self, v: &[f64]) -> Vec<f64> {
        (0..GENES)
            .map(|k| {
                let mut phi = v[k];
                if let Some(m) = &self.crosstalk {
                    for (j, &x) in v.iter().enumerate() {
                        phi += m[k * GENES + j] * x;
                    }
                }
                phi + self.offset[k]
            })
            .collect()
    }

    /// The settings that put every unit in cross with no crosstalk: each phase cancelling its
    /// passive one.
    fn all_cross(&self) -> Vec<f64> {
        self.offset.iter().map(|o| TAU - o).collect()
    }
}

/// Each unit's 2 × 2 transfer between its arms, gain included, at the reached phases.
fn transfers(draw: &Draw, phases: &[f64]) -> Vec<[[c64; 2]; 2]> {
    let i = c64::new(0.0, 1.0);
    (0..UNITS)
        .map(|t| {
            let (u, l) = (cis(phases[2 * t]), cis(phases[2 * t + 1]));
            let g = 0.5 * draw.gain[t];
            [
                [(u - l) * g, i * (u + l) * g],
                [i * (u + l) * g, (l - u) * g],
            ]
        })
        .collect()
}

/// Where a wave into a unit port comes from: the unknown inner wave or a port's input.
#[derive(Clone, Copy, Debug)]
enum Source {
    Inner(usize),
    Input(usize),
}

/// A term of a wave: `source` through unit `unit` from arm `from` to arm `to`.
#[derive(Clone, Copy, Debug)]
struct Term {
    source: Source,
    unit: usize,
    to: usize,
    from: usize,
}

/// The mesh's linear system, a = M a + inputs over its inner waves (the waves into the units'
/// joined ports), factored by Gaussian elimination in a minimum-degree order found once.
struct Solver {
    /// Each inner wave's two terms, in elimination order.
    rows: Vec<[Term; 2]>,
    /// Each port's outgoing wave's two terms, by number.
    outputs: Vec<[Term; 2]>,
    n: usize,
    /// The slot of entry (r, c) of the filled pattern, or none.
    slot: Vec<Option<usize>>,
    slots: usize,
    /// Each pivot's later neighbours in the filled pattern, in order.
    after: Vec<Vec<usize>>,
    /// Each pivot's slot, its column's slots below it and its row's slots after it (both in
    /// the order of `after`), and the slots its elimination updates, row by row.
    diagonal: Vec<usize>,
    lower: Vec<Vec<usize>>,
    upper: Vec<Vec<usize>>,
    updates: Vec<Vec<usize>>,
    /// Each port's input's terms in the inner waves' equations: (row, term), by number.
    inputs: Vec<Vec<(usize, Term)>>,
}

impl Solver {
    fn new(mesh: &Mesh) -> Solver {
        let inner: Vec<usize> = (0..4 * UNITS)
            .filter(|&p| matches!(mesh.links[p], Link::Unit(_)))
            .collect();
        let mut index = vec![usize::MAX; 4 * UNITS];
        for (k, &p) in inner.iter().enumerate() {
            index[p] = k;
        }
        // a wave out of port q is the unit's transfer of the two waves into its other end
        let terms = |q: usize| -> [Term; 2] {
            across(q).map(|r| Term {
                source: match mesh.links[r] {
                    Link::Unit(_) => Source::Inner(index[r]),
                    Link::Port(m) => Source::Input(m),
                },
                unit: q / 4,
                to: arm(q),
                from: arm(r),
            })
        };
        let rows0: Vec<[Term; 2]> = inner
            .iter()
            .map(|&p| match mesh.links[p] {
                Link::Unit(q) => terms(q),
                Link::Port(_) => unreachable!("an inner wave"),
            })
            .collect();
        let n = inner.len();
        // the symmetric pattern's graph, and a minimum-degree order on it (lowest index on ties)
        let mut graph: Vec<std::collections::BTreeSet<usize>> = vec![Default::default(); n];
        for (r, row) in rows0.iter().enumerate() {
            for t in row {
                if let Source::Inner(c) = t.source
                    && c != r
                {
                    graph[r].insert(c);
                    graph[c].insert(r);
                }
            }
        }
        let mut order = Vec::with_capacity(n);
        let mut done = vec![false; n];
        let mut elimination = graph.clone();
        for _ in 0..n {
            let k = (0..n)
                .filter(|&k| !done[k])
                .min_by_key(|&k| elimination[k].len())
                .expect("a node left");
            done[k] = true;
            order.push(k);
            let neighbours: Vec<usize> = elimination[k].iter().copied().collect();
            for &x in &neighbours {
                elimination[x].remove(&k);
                for &y in &neighbours {
                    if x != y {
                        elimination[x].insert(y);
                    }
                }
            }
        }
        let mut position = vec![0; n];
        for (k, &x) in order.iter().enumerate() {
            position[x] = k;
        }
        // the filled pattern in the new order: each pivot's later neighbours at its elimination
        let mut filled: Vec<std::collections::BTreeSet<usize>> = graph
            .iter()
            .map(|s| s.iter().map(|&x| position[x]).collect())
            .collect();
        filled = order.iter().map(|&x| filled[x].clone()).collect();
        let mut after = vec![Vec::new(); n];
        for k in 0..n {
            let later: Vec<usize> = filled[k].iter().copied().filter(|&x| x > k).collect();
            for &x in &later {
                for &y in &later {
                    if x != y {
                        filled[x].insert(y);
                    }
                }
            }
            after[k] = later;
        }
        let mut slot = vec![None; n * n];
        let mut slots = 0;
        for (r, set) in filled.iter().enumerate() {
            for &c in std::iter::once(&r).chain(set.iter()) {
                if slot[r * n + c].is_none() {
                    slot[r * n + c] = Some(slots);
                    slots += 1;
                }
            }
        }
        let renumber = |t: Term| Term {
            source: match t.source {
                Source::Inner(c) => Source::Inner(position[c]),
                s => s,
            },
            ..t
        };
        let rows: Vec<[Term; 2]> = order.iter().map(|&x| rows0[x].map(renumber)).collect();
        let mut outputs = vec![
            [Term {
                source: Source::Input(0),
                unit: 0,
                to: 0,
                from: 0,
            }; 2];
            PORTS + 1
        ];
        for (m, out) in outputs.iter_mut().enumerate().skip(1) {
            *out = terms(mesh.ports[m]).map(renumber);
        }
        let mut inputs = vec![Vec::new(); PORTS + 1];
        for (r, row) in rows.iter().enumerate() {
            for &t in row {
                if let Source::Input(m) = t.source {
                    inputs[m].push((r, t));
                }
            }
        }
        let mut solver = Solver {
            rows,
            outputs,
            n,
            slot,
            slots,
            after,
            diagonal: Vec::new(),
            lower: Vec::new(),
            upper: Vec::new(),
            updates: Vec::new(),
            inputs,
        };
        for k in 0..n {
            let after = &solver.after[k];
            solver.diagonal.push(solver.at(k, k));
            solver
                .lower
                .push(after.iter().map(|&r| solver.at(r, k)).collect());
            solver
                .upper
                .push(after.iter().map(|&c| solver.at(k, c)).collect());
            solver.updates.push(
                after
                    .iter()
                    .flat_map(|&r| after.iter().map(move |&c| (r, c)))
                    .map(|(r, c)| solver.at(r, c))
                    .collect(),
            );
        }
        solver
    }

    fn at(&self, r: usize, c: usize) -> usize {
        self.slot[r * self.n + c].expect("an entry of the filled pattern")
    }

    /// The factored I − M at the units' transfers `h` times `z` (the delay's phase).
    fn factor(&self, h: &[[[c64; 2]; 2]], z: c64) -> Vec<c64> {
        let mut v = vec![c64::new(0.0, 0.0); self.slots];
        for (r, row) in self.rows.iter().enumerate() {
            v[self.diagonal[r]] += 1.0;
            for t in row {
                if let Source::Inner(c) = t.source {
                    v[self.at(r, c)] -= h[t.unit][t.to][t.from] * z;
                }
            }
        }
        for k in 0..self.n {
            let pivot = c64::new(1.0, 0.0) / v[self.diagonal[k]];
            let (upper, m) = (&self.upper[k], self.upper[k].len());
            for (i, &below) in self.lower[k].iter().enumerate() {
                let l = v[below] * pivot;
                v[below] = l;
                for (&target, &right) in self.updates[k][i * m..(i + 1) * m].iter().zip(upper) {
                    let u = v[right];
                    v[target] -= l * u;
                }
            }
        }
        v
    }

    /// The wave out of every port in `outputs` with unit power into port `input`.
    fn solve(
        &self,
        lu: &[c64],
        h: &[[[c64; 2]; 2]],
        z: c64,
        input: usize,
        outputs: &[usize],
    ) -> Vec<c64> {
        let zero = c64::new(0.0, 0.0);
        let through = |t: &Term| h[t.unit][t.to][t.from] * z;
        let mut x = vec![zero; self.n];
        for (r, t) in &self.inputs[input] {
            x[*r] += through(t);
        }
        for k in 0..self.n {
            let xk = x[k];
            for (&r, &below) in self.after[k].iter().zip(&self.lower[k]) {
                x[r] -= lu[below] * xk;
            }
        }
        for k in (0..self.n).rev() {
            let mut s = x[k];
            for (&c, &right) in self.after[k].iter().zip(&self.upper[k]) {
                s -= lu[right] * x[c];
            }
            x[k] = s / lu[self.diagonal[k]];
        }
        outputs
            .iter()
            .map(|&m| {
                self.outputs[m].iter().fold(zero, |s, t| match t.source {
                    Source::Inner(c) => s + through(t) * x[c],
                    Source::Input(i) if i == input => s + through(t),
                    Source::Input(_) => s,
                })
            })
            .collect()
    }
}

/// The delay's phase e^(−2πiν) at the period's samples ν = −1/2, −1/2 + 1/8, ….
fn delays() -> Vec<c64> {
    (0..SAMPLES)
        .map(|k| cis(-TAU * (k as f64 / SAMPLES as f64 - 0.5)))
        .collect()
}

/// What an operation measures: each channel's power at each sample.
fn powers(solver: &Solver, draw: &Draw, v: &[f64], z: &[c64]) -> Vec<[f64; SAMPLES]> {
    let h = transfers(draw, &draw.phases(v));
    let mut p = vec![[0.0; SAMPLES]; CHANNELS.len()];
    for (k, &zk) in z.iter().enumerate() {
        let lu = solver.factor(&h, zk);
        for (ch, &(from, to)) in CHANNELS.iter().enumerate() {
            p[ch][k] = solver.solve(&lu, &h, zk, from, &[to])[0].norm_sqr();
        }
    }
    p
}

/// CF = −(1/N) Σ ln max |H_ch| (Eq. 2).
fn cost(p: &[[f64; SAMPLES]]) -> f64 {
    let n = p.len() as f64;
    -p.iter()
        .map(|ch| 0.5 * ln(ch.iter().copied().fold(0.0, f64::max)))
        .sum::<f64>()
        / n
}

/// OF1: the power averaged over the channels and the period, dB.
fn feature(p: &[[f64; SAMPLES]]) -> f64 {
    let total: f64 = p.iter().flat_map(|ch| ch.iter()).sum();
    db(total / (p.len() * SAMPLES) as f64)
}

/// The paper's mutation (Suppl. p. 6): `count` genes, each moved by a uniform step within
/// ±`range`, around the circle.
#[derive(Clone, Debug)]
struct UniformStep {
    count: usize,
    range: f64,
}

impl Mutate<Real> for UniformStep {
    fn mutate(&self, _: &Real, genome: &mut Reals, rng: &mut StreamRng) {
        let n = genome.len();
        let u = Real::uniform(2 * self.count, 0.0..=1.0)
            .expect("a range")
            .random_genome(rng);
        let mut genes: Vec<usize> = (0..n).collect();
        for k in 0..self.count {
            let j = k + ((u[k] * (n - k) as f64) as usize).min(n - k - 1);
            genes.swap(k, j);
            let x = genome[genes[k]] + (2.0 * u[self.count + k] - 1.0) * self.range;
            genome[genes[k]] = (x - TAU * (x / TAU).floor()).clamp(0.0, TAU);
        }
    }
}

/// A run's end: its OF1 and its operations.
struct Run {
    feature: f64,
    operations: u64,
}

fn finish(solver: &Solver, draw: &Draw, z: &[c64], outcome: &Outcome<Reals>) -> Run {
    Run {
        feature: feature(&powers(solver, draw, outcome.best_genome(), z)),
        operations: outcome.evaluations(),
    }
}

/// Gradient descent with momentum (Suppl. Eqs. 5 and 7, Suppl. pp. 7–8 and 23): the gradient
/// by central differences of `GD_STEP`, 1 + 2 × 72 operations an iteration, from uniform
/// phases, until CF is below 0.2 or after 60 iterations (Suppl. p. 23). The run's end: OF1 at
/// its best point, and its iterations, or none if it didn't get there.
fn descent(solver: &Solver, draw: &Draw, z: &[c64], rate: f64, seed: u64) -> (f64, Option<u64>) {
    let start: Vec<f64> = Uniform::new(seed)
        .take(GENES)
        .iter()
        .map(|u| TAU * u)
        .collect();
    // the phases free, as a phase is: bounds never reached
    let real = Real::uniform(GENES, -1e3..=1e3).expect("bounds");
    let gd = FirstOrder::builder(real)
        .step(first_order::Step::momentum(rate, GD_MOMENTUM))
        .initial_genome(Reals::from(start))
        .gradient_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid descent");
    let fitness = |x: &Reals, gradient: &mut [f64]| {
        let mut probe = x.to_vec();
        for (k, g) in gradient.iter_mut().enumerate() {
            probe[k] = x[k] + GD_STEP;
            let up = cost(&powers(solver, draw, &probe, z));
            probe[k] = x[k] - GD_STEP;
            let down = cost(&powers(solver, draw, &probe, z));
            probe[k] = x[k];
            *g = (up - down) / (2.0 * GD_STEP);
        }
        cost(&powers(solver, draw, x, z))
    };
    let outcome = Engine::new(gd, Differentiable(&fitness))
        .stop_when(Stop::target(GD_TARGET).or(Stop::evaluations(GD_ITERATIONS)))
        .run()
        .expect("a run");
    let reached = outcome
        .best_fitness()
        .score()
        .is_some_and(|cf| cf < GD_TARGET);
    (
        feature(&powers(solver, draw, outcome.best_genome(), z)),
        reached.then(|| outcome.evaluations()),
    )
}

/// What the genetic algorithm's mutation range decays over: e^(−0.5 t), t this clock.
#[derive(Clone, Copy)]
enum Clock {
    /// t the generation.
    Generations,
    /// t the operations in thousands, the clock of the swarm's schedule (Suppl. p. 7).
    Thousands,
}

fn genetic(solver: &Solver, draw: &Draw, z: &[c64], clock: Clock, seed: u64) -> Run {
    let real = Real::uniform(GENES, 0.0..=TAU).expect("bounds");
    let ga = Ga::builder(real)
        .population_size(GA_POPULATION)
        .select(Truncation::new(GA_PARENTS as f64 / GA_POPULATION as f64).expect("a fraction"))
        .crossover(PointCrossover::one_point())
        .mutate(UniformStep {
            count: GA_MUTATED,
            range: GA_RANGE,
        })
        .crossover_rate(1.0)
        .mutation_rate(1.0)
        .scheme(Scheme::Generational {
            elitism: GA_PARENTS,
        })
        .minimize()
        .seed(seed)
        .build()
        .expect("a valid GA");
    let fitness = |x: &Reals| cost(&powers(solver, draw, x, z));
    let outcome = Engine::new(ga, fitness)
        .stop_when(Stop::generations(GA_GENERATIONS))
        .control(|ga, progress| {
            let t = match clock {
                Clock::Generations => progress.generation() as f64,
                Clock::Thousands => progress.evaluations() as f64 / 1000.0,
            };
            ga.mutate_mut().range = GA_RANGE * math::exp(-GA_DECAY * t);
            Ok(())
        })
        .run()
        .expect("a run");
    finish(solver, draw, z, &outcome)
}

/// A swarm's settings: its inertia from and to (over the first 1000 operations), its cognitive
/// and social coefficients, and its moves after the first swarm.
#[derive(Clone, Copy)]
struct Swarm {
    inertia: (f64, f64),
    cognitive: f64,
    social: f64,
    moves: u64,
}

/// p. 5 and Suppl. p. 7: 37 × 108 = 3996 operations, within 4000.
const PAPER_SWARM: Swarm = Swarm {
    inertia: PSO_INERTIA,
    cognitive: PSO_COGNITIVE,
    social: PSO_SOCIAL,
    moves: 36,
};

/// Suppl. Table 18, Suppl. p. 22: the inertia 0.5 throughout; 27 × 108 = 2916 operations,
/// within 3000.
const TABLE_18_SWARM: Swarm = Swarm {
    inertia: (0.5, 0.5),
    cognitive: 0.8,
    social: 1.0,
    moves: 26,
};

fn swarm(solver: &Solver, draw: &Draw, z: &[c64], settings: Swarm, seed: u64) -> Run {
    let real = Real::uniform(GENES, 0.0..=TAU).expect("bounds");
    let pso = Pso::builder(real)
        .population_size(PSO_PARTICLES)
        .inertia(settings.inertia.0)
        .acceleration(settings.cognitive, settings.social)
        .minimize()
        .seed(seed)
        .build()
        .expect("a valid PSO");
    let fitness = |x: &Reals| cost(&powers(solver, draw, x, z));
    let (from, to) = settings.inertia;
    let outcome = Engine::new(pso, fitness)
        .stop_when(Stop::generations(settings.moves))
        .control(|pso, progress| {
            let done = (progress.evaluations() as f64 / PSO_SCHEDULE).min(1.0);
            pso.set_inertia(from - (from - to) * done)
        })
        .run()
        .expect("a run");
    finish(solver, draw, z, &outcome)
}

/// The mesh as a netlist of its units, each a fixed 4-port at the reached phases and the delay
/// `z`, solved by photonoxide's circuit solve: the S-matrix between ports 1 to 24.
fn by_circuit(mesh: &Mesh, h: &[[[c64; 2]; 2]], z: c64) -> photonoxide::Result<SMatrix> {
    let names = ["o1", "o2", "o3", "o4"];
    let mut net = Netlist::new();
    for (t, ht) in h.iter().enumerate() {
        let s = SMatrix::from_fn(4, |q, p| {
            if q / 2 == p / 2 {
                c64::new(0.0, 0.0)
            } else {
                ht[arm(q)][arm(p)] * z
            }
        });
        let unit: Arc<dyn Component> = Arc::new(Fixed::new("tbu", &names, s)?);
        net.add(&unit_name(t), unit)?;
    }
    let name = |p: usize| format!("{}.{}", unit_name(p / 4), names[p % 4]);
    for (p, link) in mesh.links.iter().enumerate() {
        if let Link::Unit(q) = *link
            && p < q
        {
            net.connect(&name(p), &name(q))?;
        }
    }
    for n in 1..=PORTS {
        net.expose(&format!("p{n}"), &name(mesh.ports[n]))?;
    }
    let circuit = net.compile()?;
    let s = circuit.s_matrix(Wavelength::um(1.55)?)?;
    let at: Vec<usize> = (1..=PORTS)
        .map(|n| {
            circuit
                .ports()
                .iter()
                .position(|p| p.name == format!("p{n}"))
                .expect("a port")
        })
        .collect();
    Ok(SMatrix::from_fn(PORTS, |q, p| s[(at[q], at[p])]))
}

/// The count of runs above −3 dB, their mean OF1, and their median operations.
fn summary(runs: &[Run]) -> (usize, f64, u64) {
    let above = runs.iter().filter(|r| r.feature > -3.0).count();
    let mean = runs.iter().map(|r| r.feature).sum::<f64>() / runs.len() as f64;
    let mut ops: Vec<u64> = runs.iter().map(|r| r.operations).collect();
    ops.sort_unstable();
    (above, mean, ops[ops.len() / 2])
}

/// Prints a method's runs: how many end above −3 dB, their mean OF1, their operations.
fn report(runs: &[Run], paper: &str) -> f64 {
    let (above, mean, ops) = summary(runs);
    println!(
        "  {above} of {} meshes above −3 dB (paper {paper}); OF1 {mean:.2} dB on average; {ops} operations",
        runs.len()
    );
    above as f64 / runs.len() as f64
}

/// Checks a share of 100 runs against the paper's, both binomial counts of 100: within two
/// standard deviations of their difference at the paper's p, √(2 p(1 − p)/100).
fn check_share(checks: &mut common::Checks, what: &str, share: f64, paper: f64) {
    let tolerance = 2.0 * (2.0 * paper * (1.0 - paper) / RUNS as f64).sqrt();
    checks.against(
        what,
        share,
        "paper, 2σ of 2 counts",
        paper,
        (tolerance * 1e4).round() / 1e4,
    );
}

pub fn main() -> photonoxide::Result<ExitCode> {
    println!(
        "The 36-unit hexagonal mesh of Pérez-López et al. 2020 set to all-cross by a GA and a PSO, without calibration: exact solves, {SAMPLES} frequencies a period"
    );
    let mut checks = common::Checks::default();
    let mesh = Mesh::new();
    let solver = Solver::new(&mesh);
    let z = delays();

    // the drawing's all-cross routes against Fig. 3b's pairs
    println!("\nThe mesh: {UNITS} units, {GENES} phase shifters, {PORTS} ports");
    let mut routes = Vec::new();
    let mut matching = 0;
    for &(from, to) in &CHANNELS {
        let (end, units) = mesh.cross_route(from);
        if end == to && mesh.cross_route(to).0 == from {
            matching += 1;
        }
        routes.push(format!("{from}–{end} ({units})"));
    }
    println!("  all-cross routes (units crossed): {}", routes.join(", "));
    checks.count("Fig. 3b's channels routed", matching, CHANNELS.len());

    // the solve against photonoxide's circuit solve, all 24 × 24, at a random state with
    // crosstalk
    let draw = Draw::new(7, Some(CROSSTALK));
    let v: Vec<f64> = Uniform::new(8)
        .take(GENES)
        .iter()
        .map(|u| TAU * u)
        .collect();
    let h = transfers(&draw, &draw.phases(&v));
    let zk = z[3];
    let s = by_circuit(&mesh, &h, zk)?;
    let lu = solver.factor(&h, zk);
    let all: Vec<usize> = (1..=PORTS).collect();
    let mut worst: f64 = 0.0;
    for p in 1..=PORTS {
        let column = solver.solve(&lu, &h, zk, p, &all);
        for (q, value) in column.iter().enumerate() {
            worst = worst.max((value - s[(q, p - 1)]).norm_sqr().sqrt());
        }
    }
    println!(
        "  the solve against photonoxide's circuit solve of the mesh as a netlist, 24 × 24: |ΔS| up to {worst:.0e}"
    );
    checks.against("largest |ΔS|", worst, "round-off", 0.0, 1e-12);

    // the loss floor: every unit exactly in cross
    let draws: Vec<Draw> = (0..RUNS).map(|r| Draw::new(r, None)).collect();
    let floor = draws
        .iter()
        .map(|d| feature(&powers(&solver, d, &d.all_cross(), &z)))
        .sum::<f64>()
        / RUNS as f64;
    println!(
        "  every unit exactly in cross: OF1 {floor:.2} dB on average over the {RUNS} meshes, their loss alone"
    );

    println!(
        "\nParticle swarm, as in Fig. 3: {PSO_PARTICLES} particles, cognitive {PSO_COGNITIVE}, social {PSO_SOCIAL}, the inertia from {} to {} over the first {PSO_SCHEDULE} operations",
        PSO_INERTIA.0, PSO_INERTIA.1
    );
    let pso: Vec<Run> = draws
        .par_iter()
        .enumerate()
        .map(|(r, d)| swarm(&solver, d, &z, PAPER_SWARM, 2000 + r as u64))
        .collect();
    let share = report(&pso, "76.66 %");
    check_share(&mut checks, "PSO's share above −3 dB", share, 0.7666);

    println!(
        "\nParticle swarm, Suppl. Table 18: {PSO_PARTICLES} particles, inertia {}, cognitive {}, social {}",
        TABLE_18_SWARM.inertia.0, TABLE_18_SWARM.cognitive, TABLE_18_SWARM.social
    );
    let table_18: Vec<Run> = draws
        .par_iter()
        .enumerate()
        .map(|(r, d)| swarm(&solver, d, &z, TABLE_18_SWARM, 3000 + r as u64))
        .collect();
    let share = report(&table_18, "70 %");
    let sigma = (2.0 * 0.7 * 0.3 / RUNS as f64).sqrt();
    println!(
        "  {:.1} standard deviations of the difference from the paper's 70 %: printed, not checked",
        (share - 0.7).abs() / sigma
    );

    // the genetic algorithm hangs on what its mutation's decay runs over, which the paper
    // doesn't say: printed, not checked
    println!(
        "\nGenetic algorithm: {GA_POPULATION} genomes, the best {GA_PARENTS} kept as parents, one-point crossover, {GA_MUTATED} phases moved within ±0.5π·e^(−{GA_DECAY} t). Printed, not checked: the paper doesn't say what t counts"
    );
    for (clock, what) in [
        (Clock::Generations, "t the generation"),
        (Clock::Thousands, "t the operations in thousands"),
    ] {
        let ga: Vec<Run> = draws
            .par_iter()
            .enumerate()
            .map(|(r, d)| genetic(&solver, d, &z, clock, 1000 + r as u64))
            .collect();
        println!("  {what}:");
        report(&ga, "95 %");
    }

    // gradient descent: printed, not checked
    println!(
        "\nGradient descent with momentum {GD_MOMENTUM}, central differences of {GD_STEP} rad (145 operations an iteration), from uniform phases until CF < {GD_TARGET} or {GD_ITERATIONS} iterations, {GD_RUNS} meshes. Printed, not checked: the paper's 5 to 6 iterations at a learning rate of 50 hang on how its CF is scaled, which it doesn't say"
    );
    for rate in GD_RATES {
        let runs: Vec<(f64, Option<u64>)> = draws[..GD_RUNS as usize]
            .par_iter()
            .enumerate()
            .map(|(r, d)| descent(&solver, d, &z, rate, 5000 + r as u64))
            .collect();
        let mut reached: Vec<u64> = runs.iter().filter_map(|r| r.1).collect();
        reached.sort_unstable();
        let mean = runs.iter().map(|r| r.0).sum::<f64>() / runs.len() as f64;
        let median = reached
            .get(reached.len() / 2)
            .map_or_else(String::new, |i| {
                format!(" in a median of {i} iterations ({} operations)", i * 145)
            });
        println!(
            "  learning rate {rate}: CF < {GD_TARGET} on {} of {GD_RUNS} meshes{median}; OF1 {mean:.2} dB on average where they stopped",
            reached.len()
        );
    }

    // with crosstalk: printed, not checked
    let crossed: Vec<Run> = (0..CROSSTALK_RUNS)
        .into_par_iter()
        .map(|r| {
            let d = Draw::new(500 + r, Some(CROSSTALK));
            swarm(&solver, &d, &z, PAPER_SWARM, 4000 + r)
        })
        .collect();
    println!(
        "\nThe particle swarm of Fig. 3 with crosstalk from 0 to {:.0} % between every pair of shifters (Fig. 4's), {CROSSTALK_RUNS} meshes. Printed, not checked: the paper has no all-cross run with it",
        100.0 * CROSSTALK
    );
    report(&crossed, "none");
    Ok(checks.finish())
}

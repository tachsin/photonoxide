//! What the validation report checks the linear-optical statistics on: permanents of known
//! matrices, the formulas against each other and the definition, Hong–Ou–Mandel interference
//! against its closed forms, the probabilities' sum and the dilation, partly distinguishable
//! photons against their internal modes, the meshes, Clements et al.'s Fig. 3 and Peruzzo et
//! al.'s equations, and the sampler's and a coupler's convergence.

use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::components::{Coupler, Dispersion, Waveguide};
use crate::circuit::{Circuit, Component, Netlist};
use crate::device::{Mesh as Grid, Scheme, StripCoupler, Values};
use crate::mode::vector;
use crate::parallel::map_in_order;
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::validation::{Case, Outcome, Tier, note_numerical};

const OK: &str = "a valid quantum check";

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).expect(OK)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// A worst error shown as itself, against zero.
fn worst(error: f64, tolerance: f64) -> Outcome {
    outcome(error, 0.0, tolerance)
}

/// A complex matrix with entries spread over the unit square, from a seed.
fn random(n: usize, seed: u64) -> Matrix {
    let mut rng = super::rng::Rng::new(seed, 7);
    let v: Vec<c64> = (0..n * n)
        .map(|_| {
            let re = rng.uniform() - 0.5;
            c64::new(re, rng.uniform() - 0.5)
        })
        .collect();
    Matrix::from_fn(n, n, |i, j| v[i * n + j])
}

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "quantum/permanent-ones",
            title: r"Quantum: the permanent of the $n \times n$ all-ones matrix is $n!$, by Ryser's and Glynn's formulas, $n$ = 1 to 14 (largest relative error shown)",
            tier: Tier::Analytic,
            source: r"$\mathop{\mathrm{Per}}(J_n) = n!$: every permutation's product is 1",
            run: permanent_ones,
        },
        Case {
            id: "quantum/permanent-derangements",
            title: r"Quantum: the permanent of $J_n - I_n$ is the number of derangements $!n$, by both formulas, $n$ = 1 to 14 (largest relative error shown)",
            tier: Tier::Analytic,
            source: r"a permutation's product is 1 when it fixes no point; $!n = (n - 1)(!(n-1) + !(n-2))$",
            run: permanent_derangements,
        },
        Case {
            id: "quantum/permanent-formulas",
            title: r"Quantum: Ryser's and Glynn's formulas in Gray-code order against the defining sum over permutations, random complex matrices of $n$ = 1 to 8 (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "P. H. Lundow, K. Markström, J. Comput. Phys. 455, 110990 (2022), doi:10.1016/j.jcp.2022.110990, Eq. 3; D. G. Glynn, Eur. J. Combin. 31, 1887 (2010), doi:10.1016/j.ejc.2010.01.010, Theorem 2.1",
            run: permanent_formulas,
        },
        Case {
            id: "quantum/permanent-threads",
            title: r"Quantum: a $20 \times 20$ permanent by each formula on 1 and 7 threads (largest difference shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md): the Gray-code sum is cut into 256 parts fixed by n, added in order",
            run: permanent_threads,
        },
        Case {
            id: "quantum/hom-visibility",
            title: r"Quantum: two photons into a coupler's two inputs, through its S-matrix as a circuit: coincidences $(R - T)^2$ for indistinguishable photons, $R^2 + T^2$ for distinguishable, so $V = 2RT/(R^2 + T^2)$, at crossing powers 0.1 to 0.9 (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the permanent of the coupler's $2 \times 2$ block, $\lvert t^2 + (i\kappa)^2 \rvert^2$; C. K. Hong, Z. Y. Ou, L. Mandel, Phys. Rev. Lett. 59, 2044 (1987), doi:10.1103/PhysRevLett.59.2044",
            run: hom_visibility,
        },
        Case {
            id: "quantum/hom-balanced",
            title: r"Quantum: two indistinguishable photons into a 50:50 coupler never leave by both outputs (coincidence probability shown)",
            tier: Tier::Analytic,
            source: "C. K. Hong, Z. Y. Ou, L. Mandel, Phys. Rev. Lett. 59, 2044 (1987), doi:10.1103/PhysRevLett.59.2044",
            run: hom_balanced,
        },
        Case {
            id: "quantum/patterns-sum",
            title: r"Quantum: the probabilities of all $\binom{m+n-1}{n}$ patterns sum to 1: 4 photons in a Haar-random $6 \times 6$ unitary (126 patterns), indistinguishable and distinguishable, and 2 photons through a coupler with 1 dB of loss, its loss dilated (largest $\lvert \Sigma - 1 \rvert$ shown)",
            tier: Tier::Analytic,
            source: "a unitary, or an isometry with the loss modes, keeps the norm of the photons' state",
            run: patterns_sum,
        },
        Case {
            id: "quantum/distinguishable-classical",
            title: r"Quantum: distinguishable photons through a Haar-random $5 \times 5$ unitary: the mean number in each output mode is the classical $\sum_p \lvert U_{qp} \rvert^2 n_p$, and fully distinguishable overlaps (a Gram matrix $I$) give the permanents of $\lvert U \rvert^2$ (largest difference shown)",
            tier: Tier::Analytic,
            source: "distinguishable photons are classical particles: each takes its own path with probability $\\lvert U_{qp} \\rvert^2$",
            run: distinguishable_classical,
        },
        Case {
            id: "quantum/partly-distinguishable",
            title: r"Quantum: three partly distinguishable photons (complex overlaps) in a Haar-random $3 \times 3$ unitary, inputs (1, 1, 1) and (2, 0, 1): the sum over permutations of overlaps and permanents against each photon expanded in a two-dimensional internal space and summed out (largest difference over the 10 patterns shown)",
            tier: Tier::Analytic,
            source: "the photons' internal states as extra modes, $U \\otimes I_2$ on six modes: the same probability by a different route",
            run: partly_distinguishable,
        },
        Case {
            id: "quantum/full-bunching",
            title: r"Quantum: $n$ indistinguishable photons all leaving by one mode are $n!$ times as likely as distinguishable ones, $n$ = 2 to 6 in a Haar-random $8 \times 8$ unitary (largest relative difference from $n!$ shown)",
            tier: Tier::Analytic,
            source: r"$\lvert \mathop{\mathrm{Per}} \rvert^2 / n! = n! \prod_p \lvert U_{qp} \rvert^2$ against $\mathop{\mathrm{Per}}(\lvert U \rvert^2)/n! = \prod_p \lvert U_{qp} \rvert^2$ for $n$ equal rows",
            run: full_bunching,
        },
        Case {
            id: "quantum/dilation",
            title: r"Quantum: a lossy circuit's loss dilated to noise modes, a coupler with 1 dB of excess loss after a 2 dB/cm, 2 mm guide on one input: $S S^\dagger + L L^\dagger = I$ and $S^\dagger S + K^\dagger K = I$ (largest entry of the difference shown)",
            tier: Tier::Analytic,
            source: r"the noise and loss modes' matrices from the eigenvectors of $I - S S^\dagger$ and $I - S^\dagger S$, each scaled by its eigenvalue's square root",
            run: dilation,
        },
        Case {
            id: "quantum/hom-dip-gaussian",
            title: r"Quantum: the Hong–Ou–Mandel dip of two Gaussian photons (1 nm wide at 1.55 µm) through a coupler of crossing power 0.3 swept over $\pm 8\sigma$ (401 wavelengths), against $R^2 + T^2 - 2RT e^{-\sigma^2 \tau^2}$ at 21 delays (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the overlap of two equal transform-limited Gaussian photons delayed by $\tau$, $\lvert \int \lvert \varphi \rvert^2 e^{i \omega \tau} d\omega \rvert^2 = e^{-\sigma^2 \tau^2}$",
            run: hom_dip_gaussian,
        },
        Case {
            id: "quantum/hom-delay-line",
            title: r"Quantum: a 100 µm guide ($n_g$ 4.2, no dispersion) before one input of a 50:50 coupler, its $S(\omega)$ from the circuit's sweep: the coincidences vanish when the other photon is delayed by the guide's group delay $n_g L/c$ (coincidence probability shown)",
            tier: Tier::Analytic,
            source: r"a guide with constant $n_g$ multiplies the amplitude by $e^{i \omega n_g L / c}$ times a constant phase: a pure delay",
            run: hom_delay_line,
        },
        Case {
            id: "quantum/mesh-reconstruction",
            title: r"Quantum: Haar-random unitaries of 2 to 12 modes decomposed into Reck et al.'s and Clements et al.'s meshes of $N(N-1)/2$ cells, multiplied back (largest entry of the difference shown)",
            tier: Tier::Analytic,
            source: "M. Reck et al., Phys. Rev. Lett. 73, 58 (1994), doi:10.1103/PhysRevLett.73.58; W. R. Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Eqs. 1 to 5",
            run: mesh_reconstruction,
        },
        Case {
            id: "quantum/mesh-depth",
            title: r"Quantum: the meshes' optical depth, the most cells a path crosses, for 3 to 12 modes: $2N - 3$ for Reck's triangle and $N$ for Clements's rectangle (largest difference shown)",
            tier: Tier::Analytic,
            source: "W. R. Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Section 3",
            run: mesh_depth,
        },
        Case {
            id: "quantum/clements-fig3-size",
            title: r"Quantum: Reck's design programmed to Haar-random unitaries of 21 to 50 modes with 0.2 dB lost by every cell on both outputs: the mean fidelity of Clements et al.'s Eq. 6 over 500 unitaries each, against their Fig. 3 (left) read off the figure (largest difference over the 30 sizes shown)",
            tier: Tier::Published,
            source: "W. R. Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Fig. 3 (left), the triangles' centroids read off the raster figure (1 pixel = 0.00034); the reading and both means' sampling set the tolerance",
            run: clements_size,
        },
        Case {
            id: "quantum/clements-fig3-loss",
            title: r"Quantum: both designs programmed to Haar-random $20 \times 20$ unitaries with 0.15 to 1 dB lost by every cell: the mean fidelity over 500 unitaries against Clements et al.'s Fig. 3 (right), Reck's 18 triangles and their design's 20 circles (largest difference shown)",
            tier: Tier::Published,
            source: "W. R. Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Fig. 3 (right), the markers' centroids read off the raster figure (1 pixel = 0.00091); the reading and both means' sampling set the tolerance",
            run: clements_loss,
        },
        Case {
            id: "quantum/peruzzo-correlations",
            title: r"Quantum: two photons in a 21-waveguide array by coupled-mode theory with Peruzzo et al.'s $C$ = 5 per mm over 782 µm, inputs (0, 1) and (−1, 1): their Eq. 4, $\Gamma_{q,r} = \lvert U_{qq'} U_{rr'} + U_{qr'} U_{rq'} \rvert^2 / (1 + \delta_{qr})$, against the permanents, all 231 pairs (largest difference shown)",
            tier: Tier::Analytic,
            source: "A. Peruzzo et al., Science 329, 1500 (2010), doi:10.1126/science.1193515, Eqs. 1 to 4 and p. 1501's array: 700 µm of coupling region and an effective 82 µm from the spreading regions",
            run: peruzzo_correlations,
        },
        Case {
            id: "quantum/peruzzo-classical-bound",
            title: r"Quantum: Peruzzo et al.'s classical bound $V_{q,r} = \frac{2}{3}\sqrt{\Gamma_{qq}\Gamma_{rr}} - \Gamma_{qr} \lt 0$ (their Eq. 5) in the same array: the pairs that violate it with distinguishable photons, for both inputs (shown; indistinguishable photons violate it at 70 and 56 pairs)",
            tier: Tier::Published,
            source: "A. Peruzzo et al., Science 329, 1500 (2010), doi:10.1126/science.1193515, Eq. 5 and p. 1503: \"Inequality Eq. 5 is not violated when the photons of the input pairs are distinguishable\"",
            run: peruzzo_classical_bound,
        },
        Case {
            id: "quantum/sampler-convergence",
            title: r"Quantum: Clifford and Clifford's exact sampler, 3 photons in a Haar-random $5 \times 5$ unitary: the total variation distance to the exact distribution falls as $N^{-1/2}$ in the number of samples, from 1000 to 256 000 (mean of 8 seeds; the order shown, to two decimals)",
            tier: Tier::Analytic,
            source: "P. Clifford, R. Clifford, SODA 2018, doi:10.1137/1.9781611975031.10, algorithm B; an empirical distribution's error falls as the square root of the samples",
            run: sampler_convergence,
        },
        Case {
            id: "quantum/coupler-hom-order",
            title: r"Quantum: the HOM visibility of the 3D coupler (two 500 x 220 nm strips 200 nm apart, a quarter of its cross-over length) from the mode solver's supermodes on cells of 25 x 27.5, 12.5 x 13.75 and 6.25 x 6.875 nm: its order of convergence in the grid (shown, to two decimals)",
            tier: Tier::Analytic,
            source: "the visibility follows the supermodes' $\\Delta n$, which the standard full-vector scheme converges at about first order at the strips' convex corners (device/coupler-3d-order)",
            run: coupler_hom_order,
        },
    ]
}

fn permanent_ones() -> Outcome {
    let mut e: f64 = 0.0;
    for n in 1..=14usize {
        let ones = Matrix::from_fn(n, n, |_, _| c64::new(1.0, 0.0));
        let f: f64 = (1..=n).map(|k| k as f64).product();
        for formula in [Formula::Ryser, Formula::Glynn] {
            let p = permanent_by(&ones, formula).expect(OK);
            e = e.max((p - f).norm() / f);
        }
    }
    worst(e, 1e-12)
}

fn permanent_derangements() -> Outcome {
    let mut d = vec![1.0f64, 0.0];
    for k in 2..=14usize {
        d.push((k as f64 - 1.0) * (d[k - 1] + d[k - 2]));
    }
    let mut e: f64 = 0.0;
    for (n, &count) in d.iter().enumerate().skip(2) {
        let a = Matrix::from_fn(n, n, |i, j| c64::new(if i == j { 0.0 } else { 1.0 }, 0.0));
        for formula in [Formula::Ryser, Formula::Glynn] {
            let p = permanent_by(&a, formula).expect(OK);
            e = e.max((p - count).norm() / count);
        }
    }
    // n = 1: no derangement, a permanent of exactly 0
    let one = Matrix::from_fn(1, 1, |_, _| c64::new(0.0, 0.0));
    e = e.max(permanent(&one).expect(OK).norm());
    worst(e, 1e-12)
}

fn permanent_formulas() -> Outcome {
    let mut e: f64 = 0.0;
    for n in 1..=8usize {
        let a = random(n, n as u64);
        let exact = permanent_by(&a, Formula::Definition).expect(OK);
        for formula in [Formula::Ryser, Formula::Glynn] {
            let p = permanent_by(&a, formula).expect(OK);
            e = e.max((p - exact).norm() / exact.norm());
        }
    }
    worst(e, 1e-12)
}

fn permanent_threads() -> Outcome {
    let a = random(20, 20);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect(OK)
            .install(|| [Formula::Ryser, Formula::Glynn].map(|f| permanent_by(&a, f).expect(OK)))
    };
    let (one, seven) = (run(1), run(7));
    let e = (0..2)
        .map(|k| (one[k] - seven[k]).norm())
        .fold(0.0, f64::max);
    worst(e, 0.0)
}

/// The ideal coupler of crossing power `kappa2` as a transfer, ports o1 o2 in, o3 o4 out.
fn coupler(kappa2: f64) -> Transfer {
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new())).expect(OK);
    net.set("c", "coupling", kappa2).expect(OK);
    for p in ["o1", "o2", "o3", "o4"] {
        net.expose(p, &format!("c.{p}")).expect(OK);
    }
    Transfer::of(&net.compile().expect(OK), um(1.55)).expect(OK)
}

/// The probability of one photon at each of o3 and o4 for one at each of o1 and o2.
fn coincidences(t: &Transfer, photons: &Photons) -> f64 {
    let input = t.input(&[("o1", 1), ("o2", 1)]).expect(OK);
    let out = t.input(&[("o3", 1), ("o4", 1)]).expect(OK);
    probability(t.matrix(), &input, &out, photons).expect(OK)
}

fn hom_visibility() -> Outcome {
    let mut e: f64 = 0.0;
    for k in 1..=9 {
        let kappa2 = 0.1 * f64::from(k);
        let t = coupler(kappa2);
        let (r, tr) = (1.0 - kappa2, kappa2);
        let same = coincidences(&t, &Photons::Indistinguishable);
        let apart = coincidences(&t, &Photons::Distinguishable);
        e = e
            .max((same - (r - tr).powi(2)).abs())
            .max((apart - (r * r + tr * tr)).abs())
            .max(((apart - same) / apart - 2.0 * r * tr / (r * r + tr * tr)).abs());
    }
    worst(e, 1e-14)
}

fn hom_balanced() -> Outcome {
    worst(
        coincidences(&coupler(0.5), &Photons::Indistinguishable),
        1e-15,
    )
}

fn patterns_sum() -> Outcome {
    let u = haar_unitary(6, 21);
    let input = Fock::single_photons(6, &[0, 2, 3, 5]).expect(OK);
    let mut e: f64 = 0.0;
    for photons in [Photons::Indistinguishable, Photons::Distinguishable] {
        let d = Distribution::exact(&u, &input, &photons).expect(OK);
        e = e.max((d.total() - 1.0).abs());
    }
    let lossy = Coupler::new().with_excess_loss(1.0).expect(OK);
    let s = lossy.s_matrix(um(1.55), &[0.3]).expect(OK);
    let t = Transfer::new(&s, ["o1", "o2", "o3", "o4"].map(String::from).to_vec()).expect(OK);
    let input = t.input(&[("o1", 1), ("o2", 1)]).expect(OK);
    let d = t
        .distribution(&input, &Photons::Indistinguishable)
        .expect(OK);
    e = e.max((d.total() - 1.0).abs());
    worst(e, 1e-12)
}

fn distinguishable_classical() -> Outcome {
    let u = haar_unitary(5, 4);
    let occupied = [0, 1, 3];
    let input = Fock::single_photons(5, &occupied).expect(OK);
    let d = Distribution::exact(&u, &input, &Photons::Distinguishable).expect(OK);
    let mut e: f64 = 0.0;
    for q in 0..5 {
        let classical: f64 = occupied.iter().map(|&p| u[(q, p)].norm_sqr()).sum();
        e = e.max((d.mean(q) - classical).abs());
    }
    let eye = Photons::Overlaps(Matrix::identity(3));
    for out in patterns(5, 3) {
        let a = probability(&u, &input, &out, &Photons::Distinguishable).expect(OK);
        let b = probability(&u, &input, &out, &eye).expect(OK);
        e = e.max((a - b).abs());
    }
    worst(e, 1e-14)
}

/// The probability of a spatial pattern for photons with internal states `psi` (each of
/// length d) in modes `modes`: U ⊗ I_d on m·d modes, each output's internal modes summed out.
fn by_internal_modes(u: &Matrix, modes: &[usize], psi: &[Vec<c64>], out: &Fock) -> f64 {
    let (m, d, n) = (u.rows(), psi[0].len(), modes.len());
    let big = Matrix::from_fn(m * d, m * d, |i, j| {
        if i % d == j % d {
            u[(i / d, j / d)]
        } else {
            c64::new(0.0, 0.0)
        }
    });
    let amplitude = |k: &Fock| -> c64 {
        let rows = k.mode_list();
        let mut sum = c64::new(0.0, 0.0);
        for a in 0..d.pow(n as u32) {
            let labels: Vec<usize> = (0..n).map(|j| a / d.pow(j as u32) % d).collect();
            let weight: c64 = (0..n).map(|j| psi[j][labels[j]]).product();
            let cols: Vec<usize> = (0..n).map(|j| modes[j] * d + labels[j]).collect();
            sum += weight * permanent(&big.select(&rows, &cols)).expect(OK);
        }
        sum / k.factorials().sqrt()
    };
    let all = patterns(m * d, n);
    let spatial =
        |k: &Fock, r: usize| -> usize { (0..d).map(|a| k.occupations()[r * d + a]).sum() };
    let total: f64 = all.iter().map(|k| amplitude(k).norm_sqr()).sum();
    let mine: f64 = all
        .iter()
        .filter(|k| (0..m).all(|r| spatial(k, r) == out.occupations()[r]))
        .map(|k| amplitude(k).norm_sqr())
        .sum();
    mine / total
}

fn partly_distinguishable() -> Outcome {
    let u = haar_unitary(3, 11);
    let raw = [
        [c64::new(1.0, 0.0), c64::new(0.0, 0.0)],
        [c64::new(0.6, 0.2), c64::new(0.3, -0.5)],
        [c64::new(-0.2, 0.4), c64::new(0.7, 0.1)],
    ];
    let psi: Vec<Vec<c64>> = raw
        .iter()
        .map(|r| {
            let norm = r.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
            r.iter().map(|z| z / norm).collect()
        })
        .collect();
    let g = Matrix::from_fn(3, 3, |i, j| {
        (0..2).map(|a| psi[i][a].conj() * psi[j][a]).sum()
    });
    let mut e: f64 = 0.0;
    for input in [Fock::new(vec![1, 1, 1]), Fock::new(vec![2, 0, 1])] {
        let modes = input.mode_list();
        for out in patterns(3, 3) {
            let p = probability(&u, &input, &out, &Photons::Overlaps(g.clone())).expect(OK);
            e = e.max((p - by_internal_modes(&u, &modes, &psi, &out)).abs());
        }
    }
    worst(e, 1e-13)
}

fn full_bunching() -> Outcome {
    let u = haar_unitary(8, 5);
    let mut e: f64 = 0.0;
    for n in 2..=6usize {
        let input = Fock::single_photons(8, &(0..n).collect::<Vec<_>>()).expect(OK);
        let mut all = vec![0; 8];
        all[7] = n;
        let all = Fock::new(all);
        let q = probability(&u, &input, &all, &Photons::Indistinguishable).expect(OK);
        let c = probability(&u, &input, &all, &Photons::Distinguishable).expect(OK);
        let f: f64 = (1..=n).map(|k| k as f64).product();
        e = e.max((q / c / f - 1.0).abs());
    }
    worst(e, 1e-12)
}

/// A lossy two-input circuit: a 2 mm, 2 dB/cm guide before input b of a coupler with 1 dB of
/// excess loss.
fn lossy_circuit() -> Circuit {
    let mut net = Netlist::new();
    let guide = Dispersion::new(um(1.55), 2.4, 4.2).with_loss(2.0);
    net.add("g", Arc::new(Waveguide::new(guide))).expect(OK);
    net.set("g", "length", 2000.0).expect(OK);
    net.add(
        "c",
        Arc::new(Coupler::new().with_excess_loss(1.0).expect(OK)),
    )
    .expect(OK);
    net.set("c", "coupling", 0.4).expect(OK);
    net.connect("g.o2", "c.o2").expect(OK);
    net.expose("a", "c.o1").expect(OK);
    net.expose("b", "g.o1").expect(OK);
    net.expose("x", "c.o3").expect(OK);
    net.expose("y", "c.o4").expect(OK);
    net.compile().expect(OK)
}

fn dilation() -> Outcome {
    let t = Transfer::of(&lossy_circuit(), um(1.55)).expect(OK);
    worst(t.dilation_error(), 1e-13)
}

/// Wavelengths whose angular frequencies are evenly spaced over ω₀ ± `span` σ, `count` + 1 of
/// them.
fn sweep(photon: &Wavepacket, span: f64, count: u32) -> Vec<Wavelength> {
    let Wavepacket::Gaussian { center, width } = photon else {
        return Vec::new();
    };
    (0..=count)
        .map(|k| {
            let w = center + width * span * (2.0 * f64::from(k) / f64::from(count) - 1.0);
            um(TAU * SPEED_OF_LIGHT / w * 1e6)
        })
        .collect()
}

fn hom_dip_gaussian() -> Outcome {
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new())).expect(OK);
    net.set("c", "coupling", 0.3).expect(OK);
    for p in ["o1", "o2", "o3", "o4"] {
        net.expose(p, &format!("c.{p}")).expect(OK);
    }
    let circuit = net.compile().expect(OK);
    let photon = Wavepacket::gaussian(um(1.55), 1.0).expect(OK);
    let Wavepacket::Gaussian { width, .. } = photon else {
        return worst(f64::NAN, 1e-12);
    };
    let spectrum = circuit.spectrum(&sweep(&photon, 8.0, 400)).expect(OK);
    let delays: Vec<f64> = (0..=20).map(|k| f64::from(k - 10) * 0.25 / width).collect();
    let dip = hom_dip(
        &spectrum,
        ["o1", "o2"],
        ["o3", "o4"],
        [&photon, &photon],
        &delays,
    )
    .expect(OK);
    let (r, t) = (0.7, 0.3);
    let e = dip
        .delays
        .iter()
        .zip(&dip.coincidences)
        .map(|(tau, p)| (p - (r * r + t * t - 2.0 * r * t * (-(width * tau).powi(2)).exp())).abs())
        .fold(0.0, f64::max);
    worst(e, 1e-12)
}

fn hom_delay_line() -> Outcome {
    let guide = Dispersion::new(um(1.55), 2.4, 4.2);
    let mut net = Netlist::new();
    net.add("delay", Arc::new(Waveguide::new(guide))).expect(OK);
    net.set("delay", "length", 100.0).expect(OK);
    net.add("c", Arc::new(Coupler::new())).expect(OK);
    net.connect("delay.o2", "c.o2").expect(OK);
    net.expose("a", "c.o1").expect(OK);
    net.expose("b", "delay.o1").expect(OK);
    net.expose("x", "c.o3").expect(OK);
    net.expose("y", "c.o4").expect(OK);
    let circuit = net.compile().expect(OK);
    let photon = Wavepacket::gaussian(um(1.55), 2.0).expect(OK);
    let spectrum = circuit.spectrum(&sweep(&photon, 8.0, 600)).expect(OK);
    let group_delay = 4.2 * 100e-6 / SPEED_OF_LIGHT;
    let p = coincidence(
        &spectrum,
        ["a", "b"],
        ["x", "y"],
        [&photon, &photon],
        group_delay,
    )
    .expect(OK);
    worst(p.abs(), 1e-12)
}

fn mesh_reconstruction() -> Outcome {
    let mut e: f64 = 0.0;
    for n in 2..=12usize {
        let u = haar_unitary(n, 40 + n as u64);
        for mesh in [Mesh::reck(&u).expect(OK), Mesh::clements(&u).expect(OK)] {
            if mesh.cells().len() != n * (n - 1) / 2 {
                return worst(f64::INFINITY, 1e-12);
            }
            e = e.max(mesh.matrix().max_difference(&u));
        }
    }
    worst(e, 1e-12)
}

fn mesh_depth() -> Outcome {
    let mut e: f64 = 0.0;
    for n in 3..=12usize {
        let u = haar_unitary(n, 60 + n as u64);
        let reck = Mesh::reck(&u).expect(OK).depth() as f64;
        let clements = Mesh::clements(&u).expect(OK).depth() as f64;
        e = e
            .max((reck - (2 * n - 3) as f64).abs())
            .max((clements - n as f64).abs());
    }
    worst(e, 0.0)
}

/// The mean fidelity and its standard error over `count` Haar-random unitaries of `n` modes,
/// programmed into the mesh and losing `db` per cell.
fn mean_fidelity(n: usize, db: f64, count: u64, arrangement: Arrangement) -> (f64, f64) {
    let seeds: Vec<u64> = (0..count).map(|s| 1_000_000 * n as u64 + s).collect();
    let f = map_in_order(&seeds, |&s| {
        let u = haar_unitary(n, s);
        let mesh = match arrangement {
            Arrangement::Reck => Mesh::reck(&u),
            Arrangement::Clements => Mesh::clements(&u),
        }
        .expect(OK);
        fidelity(&u, &mesh.lossy(db))
    });
    let mean = f.iter().sum::<f64>() / f.len() as f64;
    let var = f.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (f.len() - 1) as f64;
    (mean, (var / f.len() as f64).sqrt())
}

/// Clements et al.'s Fig. 3 (left), Reck's triangles at 0.2 dB per cell, N = 21 to 50: each
/// triangle's centroid, read off the figure (their markers are centred on the centroid: the
/// box centres lie 0.0007 higher, off every computed mean).
const FIG3_LEFT: [f64; 30] = [
    0.96962, 0.96645, 0.96297, 0.95959, 0.95574, 0.95197, 0.94788, 0.94375, 0.93956, 0.93507,
    0.93033, 0.92584, 0.92098, 0.91618, 0.91128, 0.90616, 0.90099, 0.89517, 0.88997, 0.88470,
    0.87894, 0.87339, 0.86750, 0.86167, 0.85584, 0.84989, 0.84375, 0.83755, 0.83104, 0.82514,
];

/// Clements et al.'s Fig. 3 (right), N = 20: Reck's triangles at 0.15 to 1 dB per cell in steps
/// of 0.05 dB (the two below lie under the circles), their centroids.
const FIG3_RIGHT_RECK: [f64; 18] = [
    0.98501, 0.97329, 0.95788, 0.93952, 0.91867, 0.89550, 0.86905, 0.84260, 0.81324, 0.78315,
    0.75132, 0.71932, 0.68914, 0.65816, 0.62774, 0.59677, 0.56651, 0.53734,
];

/// Their design's circles at 0.05 to 1 dB per cell, their centroids.
const FIG3_RIGHT_CLEMENTS: [f64; 20] = [
    1.00071, 1.00028, 1.00005, 0.99969, 0.99908, 0.99852, 0.99795, 0.99713, 0.99611, 0.99498,
    0.99413, 0.99263, 0.99073, 0.98957, 0.98757, 0.98571, 0.98371, 0.98158, 0.97890, 0.97606,
];
fn clements_size() -> Outcome {
    let rows: Vec<(usize, f64)> = (21..=50).zip(FIG3_LEFT).collect();
    let results = rows
        .iter()
        .map(|&(n, read)| {
            let (mean, error) = mean_fidelity(n, 0.2, 500, Arrangement::Reck);
            ((mean - read).abs(), error)
        })
        .collect::<Vec<_>>();
    let e = results.iter().map(|r| r.0).fold(0.0, f64::max);
    note_numerical(results.iter().map(|r| r.1).fold(0.0, f64::max));
    worst(e, 1e-3)
}

fn clements_loss() -> Outcome {
    let mut e: f64 = 0.0;
    let mut own: f64 = 0.0;
    for (k, &read) in FIG3_RIGHT_RECK.iter().enumerate() {
        let db = 0.15 + 0.05 * k as f64;
        let (mean, error) = mean_fidelity(20, db, 500, Arrangement::Reck);
        e = e.max((mean - read).abs());
        own = own.max(error);
    }
    for (k, &read) in FIG3_RIGHT_CLEMENTS.iter().enumerate() {
        let db = 0.05 + 0.05 * k as f64;
        let (mean, error) = mean_fidelity(20, db, 500, Arrangement::Clements);
        e = e.max((mean - read).abs());
        own = own.max(error);
    }
    note_numerical(own);
    worst(e, 4e-3)
}

/// Peruzzo et al.'s array: 21 guides, C = 5 per mm, 700 + 82 µm.
fn peruzzo_array() -> Matrix {
    coupled_array(&[0.005; 20], 782.0).expect(OK)
}

/// Γ for photons in `a` and `b` (0-based guides; their guide 0 is 10) by permanents: Γ_qr
/// over ordered pairs, as their Eq. 4 defines it.
fn gamma(u: &Matrix, a: usize, b: usize, photons: &Photons) -> Vec<Vec<f64>> {
    let input = Fock::single_photons(21, &[a, b]).expect(OK);
    let mut g = vec![vec![0.0; 21]; 21];
    for q in 0..21 {
        for r in 0..21 {
            let mut m = vec![0; 21];
            m[q] += 1;
            m[r] += 1;
            g[q][r] = probability(u, &input, &Fock::new(m), photons).expect(OK);
        }
    }
    g
}

fn peruzzo_correlations() -> Outcome {
    let u = peruzzo_array();
    let mut e: f64 = 0.0;
    for (a, b) in [(10, 11), (9, 11)] {
        let g = gamma(&u, a, b, &Photons::Indistinguishable);
        for q in 0..21 {
            for r in 0..21 {
                let eq4 = (u[(q, a)] * u[(r, b)] + u[(q, b)] * u[(r, a)]).norm_sqr()
                    / if q == r { 2.0 } else { 1.0 };
                e = e.max((g[q][r] - eq4).abs());
            }
        }
    }
    worst(e, 1e-15)
}

/// The pairs q < r that violate Peruzzo et al.'s Eq. 5 for photons in `a` and `b`.
pub(crate) fn violations(a: usize, b: usize, photons: &Photons) -> usize {
    let g = gamma(&peruzzo_array(), a, b, photons);
    (0..21)
        .flat_map(|q| (q + 1..21).map(move |r| (q, r)))
        .filter(|&(q, r)| 2.0 / 3.0 * (g[q][q] * g[r][r]).sqrt() - g[q][r] > 0.0)
        .count()
}

fn peruzzo_classical_bound() -> Outcome {
    let n = violations(10, 11, &Photons::Distinguishable)
        + violations(9, 11, &Photons::Distinguishable);
    worst(n as f64, 0.0)
}

fn sampler_convergence() -> Outcome {
    let u = haar_unitary(5, 8);
    let input = Fock::single_photons(5, &[0, 1, 3]).expect(OK);
    let exact = Distribution::exact(&u, &input, &Photons::Indistinguishable).expect(OK);
    let counts = [1000usize, 4000, 16_000, 64_000, 256_000];
    let distance: Vec<f64> = counts
        .iter()
        .map(|&n| {
            (0..8u64)
                .map(|seed| {
                    let s = sample(&u, &input, n, 100 + seed).expect(OK);
                    Distribution::from_samples(&s).total_variation(&exact)
                })
                .sum::<f64>()
                / 8.0
        })
        .collect();
    // the least-squares slope of log distance against log N
    let x: Vec<f64> = counts.iter().map(|&n| (n as f64).ln()).collect();
    let y: Vec<f64> = distance.iter().map(|d| d.ln()).collect();
    let (mx, my) = (x.iter().sum::<f64>() / 5.0, y.iter().sum::<f64>() / 5.0);
    let slope = x
        .iter()
        .zip(&y)
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>()
        / x.iter().map(|a| (a - mx).powi(2)).sum::<f64>();
    outcome((-slope * 100.0).round() / 100.0, 0.5, 0.1)
}

fn coupler_hom_order() -> Outcome {
    let device = StripCoupler::new();
    let v = Values::defaults(&device);
    let l = um(1.55);
    // a quarter of a 5.5 × 1.32 µm window, as device/coupler-3d-order
    let dn = map_in_order(&[1, 2, 4], |&s| {
        let c =
            device
                .clone()
                .with_mesh(Grid::window(5.5, 1.32, 110 * s, 24 * s, Scheme::Standard));
        let (even, odd) = c.cross_sections(&v, l).expect(OK);
        let n = |cs| {
            vector::modes(cs, l, 1, None).expect(OK)[0]
                .effective_index()
                .re
        };
        n(&even) - n(&odd)
    });
    // a quarter of the finest grid's cross-over length λ/(2Δn)
    let length = 0.25 * 1.55 / (2.0 * dn[2]);
    let visibility: Vec<f64> = dn
        .iter()
        .map(|&d| {
            let kappa2 = (PI * d / 1.55 * length).sin().powi(2);
            let t = coupler(kappa2);
            let same = coincidences(&t, &Photons::Indistinguishable);
            let apart = coincidences(&t, &Photons::Distinguishable);
            1.0 - same / apart
        })
        .collect();
    let order = ((visibility[0] - visibility[1]) / (visibility[1] - visibility[2]))
        .abs()
        .log2();
    outcome((order * 100.0).round() / 100.0, 1.0, 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tallest Γ of Peruzzo et al.'s four simulated panels over the pairs they measured
    /// (q − r even: their fibre arrays have twice the output pitch), as docs/methods/quantum.md
    /// gives them against the paper's 0.0178, 0.0237, 0.0147 and 0.0266.
    #[test]
    fn peruzzo_tallest_on_the_measured_pairs() {
        let u = peruzzo_array();
        let panels = [
            (10, 11, Photons::Distinguishable, 0.0176),
            (10, 11, Photons::Indistinguishable, 0.0239),
            (9, 11, Photons::Distinguishable, 0.0162),
            (9, 11, Photons::Indistinguishable, 0.0264),
        ];
        for (a, b, photons, docs) in panels {
            let g = gamma(&u, a, b, &photons);
            let tallest = (0..21)
                .flat_map(|q| (0..21).map(move |r| (q, r)))
                .filter(|(q, r)| (q + r) % 2 == 0)
                .map(|(q, r)| g[q][r])
                .fold(0.0, f64::max);
            assert!((tallest - docs).abs() < 5e-5, "{tallest} against {docs}");
        }
    }
}

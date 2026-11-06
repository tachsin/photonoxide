use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::matrix::{hermitian_eigen, qr};
use super::permanent::minors;
use super::*;
use crate::circuit::components::{Coupler, Dispersion, Waveguide};
use crate::circuit::{Netlist, SMatrix};
use crate::units::{SPEED_OF_LIGHT, Wavelength};

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).unwrap()
}

/// A complex matrix with entries spread over the unit square, from a seed.
fn random(rows: usize, cols: usize, seed: u64) -> Matrix {
    let mut rng = super::rng::Rng::new(seed, 99);
    let v: Vec<c64> = (0..rows * cols)
        .map(|_| {
            let re = rng.uniform() - 0.5;
            c64::new(re, rng.uniform() - 0.5)
        })
        .collect();
    Matrix::from_fn(rows, cols, |i, j| v[i * cols + j])
}

/// The ideal coupler's S, ports o1 o2 (in) o3 o4 (out), crossing power `kappa2`.
fn coupler(kappa2: f64) -> Transfer {
    let c = Coupler::new();
    let s = crate::circuit::Component::s_matrix(&c, um(1.55), &[kappa2]).unwrap();
    Transfer::new(&s, ["o1", "o2", "o3", "o4"].map(String::from).to_vec()).unwrap()
}

#[test]
fn jacobi_gives_a_hermitian_matrix_its_eigenvectors() {
    let a = random(6, 6, 1);
    let h = Matrix::from_fn(6, 6, |i, j| a[(i, j)] + a[(j, i)].conj());
    let (values, v) = hermitian_eigen(&h);
    let back = Matrix::from_fn(6, 6, |i, j| {
        (0..6)
            .map(|k| v[(i, k)] * values[k] * v[(j, k)].conj())
            .sum()
    });
    assert!(
        back.max_difference(&h) < 1e-13,
        "{}",
        back.max_difference(&h)
    );
    assert!(v.isometry_error() < 1e-13);
}

#[test]
fn gram_schmidt_factors_a_square_matrix() {
    let a = random(5, 5, 2);
    let (q, r) = qr(&a);
    assert!(q.isometry_error() < 1e-14);
    assert!(q.product(&r).unwrap().max_difference(&a) < 1e-14);
    for i in 0..5 {
        for j in 0..i {
            assert_eq!(r[(i, j)], c64::new(0.0, 0.0));
        }
    }
}

#[test]
fn the_formulas_agree_with_the_definition() {
    for n in 1..=8 {
        let a = random(n, n, n as u64);
        let exact = permanent_by(&a, Formula::Definition).unwrap();
        for f in [Formula::Ryser, Formula::Glynn] {
            let p = permanent_by(&a, f).unwrap();
            assert!(
                (p - exact).norm() < 1e-14 * exact.norm().max(1.0),
                "{n} {f:?}"
            );
        }
    }
}

#[test]
fn known_permanents() {
    for n in 1..=12usize {
        let ones = Matrix::from_fn(n, n, |_, _| c64::new(1.0, 0.0));
        let factorial: f64 = (1..=n).map(|k| k as f64).product();
        let derangements =
            Matrix::from_fn(n, n, |i, j| c64::new(if i == j { 0.0 } else { 1.0 }, 0.0));
        // !n = n!·Σ (−1)^k/k!
        let mut d = [1.0f64, 0.0].to_vec();
        for k in 2..=n {
            d.push((k as f64 - 1.0) * (d[k - 1] + d[k - 2]));
        }
        for f in [Formula::Ryser, Formula::Glynn] {
            let p = permanent_by(&ones, f).unwrap();
            assert!((p.re / factorial - 1.0).abs() < 1e-13, "{n}");
            let q = permanent_by(&derangements, f).unwrap();
            assert!((q.re - d[n]).abs() <= 1e-13 * d[n].max(1.0), "{n} {f:?}");
        }
    }
}

#[test]
fn a_permanent_is_the_same_on_any_number_of_threads() {
    let a = random(16, 16, 5);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                (
                    permanent_by(&a, Formula::Ryser).unwrap(),
                    permanent_by(&a, Formula::Glynn).unwrap(),
                )
            })
    };
    let one = run(1);
    assert_eq!(one, run(3));
    assert_eq!(one, run(8));
    assert!((one.0 - one.1).norm() < 1e-11 * one.0.norm());
}

#[test]
fn the_minors_come_from_one_pass() {
    for k in 2..=7 {
        let b = random(k - 1, k, 10 + k as u64);
        let all = minors(&b);
        for (l, m) in all.iter().enumerate() {
            let cols: Vec<usize> = (0..k).filter(|&j| j != l).collect();
            let rows: Vec<usize> = (0..k - 1).collect();
            let direct = permanent_by(&b.select(&rows, &cols), Formula::Definition).unwrap();
            assert!((m - direct).norm() < 1e-14 * direct.norm().max(1.0));
        }
    }
}

#[test]
fn hong_ou_mandel_through_a_coupler() {
    for kappa2 in [0.1, 0.3, 0.5, 0.8] {
        let t = coupler(kappa2);
        let input = t.input(&[("o1", 1), ("o2", 1)]).unwrap();
        let out = t.input(&[("o3", 1), ("o4", 1)]).unwrap();
        let s = t.matrix();
        let p = probability(s, &input, &out, &Photons::Indistinguishable).unwrap();
        let q = probability(s, &input, &out, &Photons::Distinguishable).unwrap();
        let (r, tr) = (1.0 - kappa2, kappa2);
        assert!((p - (r - tr).powi(2)).abs() < 1e-15);
        assert!((q - (r * r + tr * tr)).abs() < 1e-15);
        assert!(((q - p) / q - 2.0 * r * tr / (r * r + tr * tr)).abs() < 1e-14);
    }
}

#[test]
fn overlaps_of_one_and_zero_give_the_two_limits() {
    let u = haar_unitary(4, 3);
    let input = Fock::new(vec![1, 1, 1, 0]);
    let ones = Photons::Overlaps(Matrix::from_fn(3, 3, |_, _| c64::new(1.0, 0.0)));
    let eye = Photons::Overlaps(Matrix::identity(3));
    for out in patterns(4, 3) {
        let a = probability(&u, &input, &out, &Photons::Indistinguishable).unwrap();
        let b = probability(&u, &input, &out, &ones).unwrap();
        let c = probability(&u, &input, &out, &Photons::Distinguishable).unwrap();
        let d = probability(&u, &input, &out, &eye).unwrap();
        assert!((a - b).abs() < 1e-15 && (c - d).abs() < 1e-15);
    }
}

/// The probability of a spatial pattern for photons with internal states `psi` (rows, each of
/// length d) in modes `modes`, by expanding every photon into the internal basis: U ⊗ I_d on
/// m·d modes, the internal modes of each output summed out.
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
            sum += weight * permanent(&big.select(&rows, &cols)).unwrap();
        }
        sum / k.factorials().sqrt()
    };
    let all = patterns(m * d, n);
    let total: f64 = all.iter().map(|k| amplitude(k).norm_sqr()).sum();
    let mine: f64 = all
        .iter()
        .filter(|k| {
            (0..m).all(|r| {
                (0..d).map(|a| k.occupations()[r * d + a]).sum::<usize>() == out.occupations()[r]
            })
        })
        .map(|k| amplitude(k).norm_sqr())
        .sum();
    mine / total
}

#[test]
fn partly_distinguishable_photons_match_their_internal_modes() {
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
    // with two photons in one mode, the state's norm matters too
    for input in [Fock::new(vec![1, 1, 1]), Fock::new(vec![2, 0, 1])] {
        let modes = input.mode_list();
        let mut total = 0.0;
        for out in patterns(3, 3) {
            let p = probability(&u, &input, &out, &Photons::Overlaps(g.clone())).unwrap();
            let q = by_internal_modes(&u, &modes, &psi, &out);
            assert!((p - q).abs() < 1e-14, "{input:?} {out:?}: {p} {q}");
            total += p;
        }
        assert!((total - 1.0).abs() < 1e-14);
    }
}

#[test]
fn a_lossy_circuit_keeps_its_probabilities() {
    let c = Coupler::new().with_excess_loss(1.0).unwrap();
    let s = crate::circuit::Component::s_matrix(&c, um(1.55), &[0.3]).unwrap();
    let t = Transfer::new(&s, ["o1", "o2", "o3", "o4"].map(String::from).to_vec()).unwrap();
    assert!(t.dilation_error() < 1e-14, "{}", t.dilation_error());
    assert_eq!(t.loss_modes(), 4);
    let input = t.input(&[("o1", 1), ("o2", 1)]).unwrap();
    let d = t.distribution(&input, &Photons::Indistinguishable).unwrap();
    assert!((d.total() - 1.0).abs() < 1e-14);
    // both photons arrive with probability η²
    let eta = 10f64.powf(-0.1);
    let both: f64 = d
        .patterns()
        .iter()
        .zip(d.probabilities())
        .filter(|(p, _)| p.photons() == 2)
        .map(|(_, q)| q)
        .sum();
    assert!((both - eta * eta).abs() < 1e-14);
    // a lossless coupler has no loss modes
    assert_eq!(coupler(0.5).loss_modes(), 0);
}

#[test]
fn gain_is_refused() {
    let s = SMatrix::from_rows(vec![vec![c64::new(1.1, 0.0)]]).unwrap();
    assert!(Transfer::new(&s, vec!["a".into()]).is_err());
}

#[test]
fn every_pattern_and_its_sum() {
    assert_eq!(patterns(6, 4).len(), 126);
    let u = haar_unitary(6, 21);
    let input = Fock::single_photons(6, &[0, 2, 3, 5]).unwrap();
    for photons in [Photons::Indistinguishable, Photons::Distinguishable] {
        let d = Distribution::exact(&u, &input, &photons).unwrap();
        assert!((d.total() - 1.0).abs() < 1e-13);
        // the mean photon number in each mode is the classical Σ_p |U_qp|² n_p
        for q in 0..6 {
            let classical: f64 = [0, 2, 3, 5].iter().map(|&p| u[(q, p)].norm_sqr()).sum();
            assert!((d.mean(q) - classical).abs() < 1e-13);
        }
    }
}

#[test]
fn full_bunching_is_n_factorial_times_the_classical() {
    let u = haar_unitary(5, 4);
    let input = Fock::single_photons(5, &[0, 1, 2, 3]).unwrap();
    let all = Fock::new(vec![0, 4, 0, 0, 0]);
    let q = probability(&u, &input, &all, &Photons::Indistinguishable).unwrap();
    let c = probability(&u, &input, &all, &Photons::Distinguishable).unwrap();
    assert!((q / c - 24.0).abs() < 1e-11);
}

#[test]
fn the_sampler_draws_from_the_exact_distribution() {
    let u = haar_unitary(5, 8);
    for input in [
        Fock::single_photons(5, &[0, 1, 3]).unwrap(),
        Fock::new(vec![2, 0, 1, 0, 0]),
    ] {
        let exact = Distribution::exact(&u, &input, &Photons::Indistinguishable).unwrap();
        let samples = sample(&u, &input, 40_000, 3).unwrap();
        let tv = Distribution::from_samples(&samples).total_variation(&exact);
        // 35 patterns: the expected distance is about 0.012 at 40 000 samples
        assert!(tv < 0.03, "{input:?}: {tv}");
    }
}

#[test]
fn samples_are_the_same_on_any_number_of_threads() {
    let u = haar_unitary(6, 2);
    let input = Fock::single_photons(6, &[1, 2, 4]).unwrap();
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| sample(&u, &input, 500, 9).unwrap())
    };
    assert_eq!(run(1), run(5));
}

#[test]
fn both_meshes_give_the_unitary_back() {
    for n in [2, 3, 4, 5, 8, 9] {
        let u = haar_unitary(n, 30 + n as u64);
        for mesh in [Mesh::reck(&u).unwrap(), Mesh::clements(&u).unwrap()] {
            assert_eq!(mesh.cells().len(), n * (n - 1) / 2);
            let e = mesh.matrix().max_difference(&u);
            assert!(e < 1e-13, "{n} {:?}: {e}", mesh.arrangement());
            assert!((fidelity(&u, &mesh.matrix()) - 1.0).abs() < 1e-13);
        }
        assert_eq!(
            Mesh::reck(&u).unwrap().depth(),
            (2 * n).saturating_sub(3).max(1)
        );
        if n >= 3 {
            assert_eq!(Mesh::clements(&u).unwrap().depth(), n);
        }
    }
}

#[test]
fn the_dip_of_two_gaussian_photons() {
    // a lossless coupler of crossing power 0.3 swept over ±6σ
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new())).unwrap();
    net.set("c", "coupling", 0.3).unwrap();
    for p in ["o1", "o2", "o3", "o4"] {
        net.expose(p, &format!("c.{p}")).unwrap();
    }
    let circuit = net.compile().unwrap();
    let photon = Wavepacket::gaussian(um(1.55), 1.0).unwrap();
    let Wavepacket::Gaussian { center, width } = photon else {
        unreachable!()
    };
    let wavelengths: Vec<Wavelength> = (0..=400)
        .map(|k| {
            let w = center + width * (-8.0 + 16.0 * f64::from(k) / 400.0);
            um(std::f64::consts::TAU * SPEED_OF_LIGHT / w * 1e6)
        })
        .collect();
    let spectrum = circuit.spectrum(&wavelengths).unwrap();
    let delays: Vec<f64> = (0..=20).map(|k| f64::from(k - 10) * 0.25 / width).collect();
    let dip = hom_dip(
        &spectrum,
        ["o1", "o2"],
        ["o3", "o4"],
        [&photon, &photon],
        &delays,
    )
    .unwrap();
    let (r, t) = (0.7, 0.3);
    for (tau, p) in dip.delays.iter().zip(&dip.coincidences) {
        let closed = r * r + t * t - 2.0 * r * t * (-(width * tau).powi(2)).exp();
        assert!((p - closed).abs() < 1e-12, "{tau}: {p} {closed}");
    }
    assert!((dip.visibility() - 2.0 * r * t / (r * r + t * t)).abs() < 1e-12);
}

#[test]
fn a_delay_line_moves_the_dip_by_its_group_delay() {
    let guide = Dispersion::new(um(1.55), 2.4, 4.2);
    let mut net = Netlist::new();
    net.add("delay", Arc::new(Waveguide::new(guide))).unwrap();
    net.set("delay", "length", 100.0).unwrap();
    net.add("c", Arc::new(Coupler::new())).unwrap();
    net.connect("delay.o2", "c.o2").unwrap();
    net.expose("a", "c.o1").unwrap();
    net.expose("b", "delay.o1").unwrap();
    net.expose("x", "c.o3").unwrap();
    net.expose("y", "c.o4").unwrap();
    let circuit = net.compile().unwrap();
    let photon = Wavepacket::gaussian(um(1.55), 2.0).unwrap();
    let Wavepacket::Gaussian { center, width } = photon else {
        unreachable!()
    };
    let wavelengths: Vec<Wavelength> = (0..=600)
        .map(|k| {
            let w = center + width * (-8.0 + 16.0 * f64::from(k) / 600.0);
            um(std::f64::consts::TAU * SPEED_OF_LIGHT / w * 1e6)
        })
        .collect();
    let spectrum = circuit.spectrum(&wavelengths).unwrap();
    let group_delay = 4.2 * 100e-6 / SPEED_OF_LIGHT;
    let p = coincidence(
        &spectrum,
        ["a", "b"],
        ["x", "y"],
        [&photon, &photon],
        group_delay,
    )
    .unwrap();
    assert!(p.abs() < 1e-12, "{p}");
}

#[test]
fn a_coupled_array_is_unitary_and_symmetric() {
    let u = coupled_array(&[0.005; 20], 782.0).unwrap();
    assert!(u.isometry_error() < 1e-13);
    for i in 0..21 {
        for j in 0..21 {
            assert!((u[(i, j)] - u[(j, i)]).norm() < 1e-13);
            assert!((u[(i, j)] - u[(20 - i, 20 - j)]).norm() < 1e-13);
        }
    }
}

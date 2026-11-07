//! Unit tests of the hafnians, the Gaussian states and the sources.

use num_complex::Complex64 as c64;

use super::hafnian::by_definition;
use super::*;

fn random_symmetric(n: usize, seed: u64) -> Matrix {
    let mut rng = super::rng::Rng::new(seed, 11);
    let mut a = Matrix::zeros(n, n);
    for i in 0..n {
        for j in 0..=i {
            let v = c64::new(rng.uniform() - 0.5, rng.uniform() - 0.5);
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

#[test]
fn hafnians_match_their_definition() {
    for n in [2, 4, 6, 8, 10] {
        let a = random_symmetric(n, n as u64);
        let h = hafnian(&a).unwrap();
        let d = by_definition(&a, false);
        assert!((h - d).norm() < 1e-12 * d.norm().max(1.0), "{n}: {h} {d}");
        let l = loop_hafnian(&a).unwrap();
        let dl = by_definition(&a, true);
        assert!(
            (l - dl).norm() < 1e-12 * dl.norm().max(1.0),
            "{n}: {l} {dl}"
        );
    }
    // odd sizes: no perfect matching, and a loop hafnian padded with a loop of 1
    let a = random_symmetric(5, 3);
    assert_eq!(hafnian(&a).unwrap(), c64::new(0.0, 0.0));
    let (l, d) = (loop_hafnian(&a).unwrap(), by_definition(&a, true));
    assert!((l - d).norm() < 1e-12 * d.norm(), "{l} {d}");
}

#[test]
fn the_all_ones_hafnian_counts_perfect_matchings() {
    let mut double_factorial = 1.0;
    for n in 1..=10usize {
        double_factorial *= (2 * n - 1) as f64;
        let ones = Matrix::from_fn(2 * n, 2 * n, |_, _| c64::new(1.0, 0.0));
        let h = hafnian(&ones).unwrap();
        assert!(
            (h.re - double_factorial).abs() < 1e-10 * double_factorial,
            "{n}: {h} against {double_factorial}"
        );
    }
}

#[test]
fn squeezed_vacuum_has_its_closed_form_statistics() {
    let r: f64 = 0.8;
    let g = Gaussian::vacuum(1).squeezed(0, r, 0.7).unwrap();
    let t = r.tanh();
    let mut total = 0.0;
    let mut mean = 0.0;
    for k in 0..12usize {
        let fact = |m: usize| (1..=m).map(|j| j as f64).product::<f64>();
        let closed =
            fact(2 * k) / (4f64.powi(k as i32) * fact(k).powi(2)) * t.powi(2 * k as i32) / r.cosh();
        let p = g.probability(&[2 * k]).unwrap();
        assert!((p - closed).abs() < 1e-13, "{k}: {p} {closed}");
        let odd = g.probability(&[2 * k + 1]).unwrap();
        assert!(odd.abs() < 1e-14, "{odd}");
        total += p;
        mean += 2.0 * k as f64 * p;
    }
    assert!((total - 1.0).abs() < 1e-3);
    assert!((g.mean_photons(0).unwrap() - r.sinh().powi(2)).abs() < 1e-14);
    assert!((mean - r.sinh().powi(2)).abs() < 2e-2);
    // clicks: dark with probability 1/cosh r
    let dark = g.click_probability(&[false]).unwrap();
    let click = g.click_probability(&[true]).unwrap();
    assert!((dark - 1.0 / r.cosh()).abs() < 1e-14, "{dark}");
    assert!((click - (1.0 - 1.0 / r.cosh())).abs() < 1e-14, "{click}");
}

#[test]
fn coherent_states_are_poissonian_and_displaced_squeezing_has_its_variance() {
    let alpha = c64::new(0.7, -0.4);
    let g = Gaussian::vacuum(1).displaced(0, alpha).unwrap();
    let mu = alpha.norm_sqr();
    let mut fact = 1.0;
    for n in 0..10usize {
        if n > 0 {
            fact *= n as f64;
        }
        let p = g.probability(&[n]).unwrap();
        let closed = (-mu).exp() * mu.powi(n as i32) / fact;
        assert!((p - closed).abs() < 1e-14, "{n}: {p} {closed}");
    }
    // a displaced squeezed state: the distribution's mean and variance against the covariance's
    let g = Gaussian::vacuum(1)
        .squeezed(0, 0.3, 1.1)
        .unwrap()
        .displaced(0, c64::new(0.4, 0.5))
        .unwrap();
    let (mut m1, mut m2) = (0.0, 0.0);
    for n in 0..30usize {
        let p = g.probability(&[n]).unwrap();
        m1 += n as f64 * p;
        m2 += (n * n) as f64 * p;
    }
    let mean = g.mean_photons(0).unwrap();
    let var = g.photon_covariance(0, 0).unwrap();
    assert!((m1 - mean).abs() < 1e-10, "{m1} {mean}");
    assert!((m2 - m1 * m1 - var).abs() < 1e-9, "{} {var}", m2 - m1 * m1);
}

#[test]
fn two_mode_squeezed_vacuum_is_thermal_in_each_half() {
    let r: f64 = 0.6;
    let g = Gaussian::vacuum(2).two_mode_squeezed(0, 1, r, 0.3).unwrap();
    let t2 = r.tanh().powi(2);
    for n in 0..6usize {
        for m in 0..6usize {
            let p = g.probability(&[n, m]).unwrap();
            let closed = if n == m {
                t2.powi(n as i32) / r.cosh().powi(2)
            } else {
                0.0
            };
            assert!((p - closed).abs() < 1e-14, "{n} {m}: {p} {closed}");
        }
    }
    let half = g.reduced(&[1]).unwrap();
    let nbar = r.sinh().powi(2);
    let thermal = Gaussian::vacuum(1).with_thermal(0, nbar).unwrap();
    for (a, b) in half.covariance().iter().zip(thermal.covariance()) {
        assert!((a - b).abs() < 1e-14);
    }
    // clicks against summed photon numbers: both dark, both clicking
    let both_dark = g.click_probability(&[false, false]).unwrap();
    assert!((both_dark - g.probability(&[0, 0]).unwrap()).abs() < 1e-14);
    let both = g.click_probability(&[true, true]).unwrap();
    let summed: f64 = (1..80).map(|n| t2.powi(n) / r.cosh().powi(2)).sum();
    assert!((both - summed).abs() < 1e-12, "{both} {summed}");
}

#[test]
fn loss_mixes_in_the_vacuum() {
    let r: f64 = 1.2;
    let eta = 0.7;
    let g = Gaussian::vacuum(1)
        .squeezed(0, r, 0.0)
        .unwrap()
        .lossy(0, eta)
        .unwrap();
    let v = g.quadrature_variance(0, 0.0).unwrap();
    assert!((v - (eta * (-2.0 * r).exp() + 1.0 - eta)).abs() < 1e-14);
}

#[test]
fn hafnians_are_the_same_on_any_number_of_threads() {
    let a = random_symmetric(28, 5);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| (hafnian(&a).unwrap(), loop_hafnian(&a).unwrap()))
    };
    assert_eq!(run(1), run(5));
}

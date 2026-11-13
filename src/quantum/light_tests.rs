//! Unit tests of the hafnians, the Gaussian states and the sources.

use num_complex::Complex64 as c64;

use super::hafnian::by_definition;
use super::*;
use crate::portable;

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
        assert!(
            portable::complex::abs(h - d) < 1e-12 * portable::complex::abs(d).max(1.0),
            "{n}: {h} {d}"
        );
        let l = loop_hafnian(&a).unwrap();
        let dl = by_definition(&a, true);
        assert!(
            portable::complex::abs(l - dl) < 1e-12 * portable::complex::abs(dl).max(1.0),
            "{n}: {l} {dl}"
        );
    }
    // odd sizes: no perfect matching, and a loop hafnian padded with a loop of 1
    let a = random_symmetric(5, 3);
    assert_eq!(hafnian(&a).unwrap(), c64::new(0.0, 0.0));
    let (l, d) = (loop_hafnian(&a).unwrap(), by_definition(&a, true));
    assert!(
        portable::complex::abs(l - d) < 1e-12 * portable::complex::abs(d),
        "{l} {d}"
    );
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
    let t = portable::tanh(r);
    let mut total = 0.0;
    let mut mean = 0.0;
    for k in 0..12usize {
        let fact = |m: usize| (1..=m).map(|j| j as f64).product::<f64>();
        let closed = fact(2 * k) / (portable::powi(4f64, k as i32) * portable::powi(fact(k), 2))
            * portable::powi(t, 2 * k as i32)
            / portable::cosh(r);
        let p = g.probability(&[2 * k]).unwrap();
        assert!((p - closed).abs() < 1e-13, "{k}: {p} {closed}");
        let odd = g.probability(&[2 * k + 1]).unwrap();
        assert!(odd.abs() < 1e-14, "{odd}");
        total += p;
        mean += 2.0 * k as f64 * p;
    }
    assert!((total - 1.0).abs() < 1e-3);
    assert!((g.mean_photons(0).unwrap() - portable::powi(portable::sinh(r), 2)).abs() < 1e-14);
    assert!((mean - portable::powi(portable::sinh(r), 2)).abs() < 2e-2);
    // clicks: dark with probability 1/cosh r
    let dark = g.click_probability(&[false]).unwrap();
    let click = g.click_probability(&[true]).unwrap();
    assert!((dark - 1.0 / portable::cosh(r)).abs() < 1e-14, "{dark}");
    assert!(
        (click - (1.0 - 1.0 / portable::cosh(r))).abs() < 1e-14,
        "{click}"
    );
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
        let closed = portable::exp(-mu) * portable::powi(mu, n as i32) / fact;
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
    let t2 = portable::powi(portable::tanh(r), 2);
    for n in 0..6usize {
        for m in 0..6usize {
            let p = g.probability(&[n, m]).unwrap();
            let closed = if n == m {
                portable::powi(t2, n as i32) / portable::powi(portable::cosh(r), 2)
            } else {
                0.0
            };
            assert!((p - closed).abs() < 1e-14, "{n} {m}: {p} {closed}");
        }
    }
    let half = g.reduced(&[1]).unwrap();
    let nbar = portable::powi(portable::sinh(r), 2);
    let thermal = Gaussian::vacuum(1).with_thermal(0, nbar).unwrap();
    for (a, b) in half.covariance().iter().zip(thermal.covariance()) {
        assert!((a - b).abs() < 1e-14);
    }
    // clicks against summed photon numbers: both dark, both clicking
    let both_dark = g.click_probability(&[false, false]).unwrap();
    assert!((both_dark - g.probability(&[0, 0]).unwrap()).abs() < 1e-14);
    let both = g.click_probability(&[true, true]).unwrap();
    let summed: f64 = (1..80)
        .map(|n| portable::powi(t2, n) / portable::powi(portable::cosh(r), 2))
        .sum();
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
    assert!((v - (eta * portable::exp(-2.0 * r) + 1.0 - eta)).abs() < 1e-14);
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

/// A Gaussian state's Wigner function: the vacuum's peak is 1/(2π) (ħ = 2), it integrates to 1,
/// a squeezed vacuum's falls as its variances say along x and p, and a coherent state's is the
/// vacuum's moved to its means.
#[test]
fn the_wigner_function_has_its_closed_forms() {
    use std::f64::consts::TAU;
    let vacuum = Gaussian::vacuum(1);
    assert!((vacuum.wigner(0, 0.0, 0.0).unwrap() - 1.0 / TAU).abs() < 1e-15);
    let r = 0.6;
    let s = Gaussian::vacuum(2).squeezed(1, r, 0.0).unwrap();
    // V = diag(e^{−2r}, e^{2r}), det 1: W(x, 0) = e^{−x² e^{2r}/2}/(2π), W(0, p) = e^{−p² e^{−2r}/2}/(2π)
    for v in [0.3, 1.0, 2.0] {
        let wx = s.wigner(1, v, 0.0).unwrap();
        let wp = s.wigner(1, 0.0, v).unwrap();
        assert!((wx - portable::exp(-0.5 * v * v * portable::exp(2.0 * r)) / TAU).abs() < 1e-15);
        assert!((wp - portable::exp(-0.5 * v * v * portable::exp(-2.0 * r)) / TAU).abs() < 1e-15);
    }
    // the other mode is the vacuum still
    assert!((s.wigner(0, 0.0, 0.0).unwrap() - 1.0 / TAU).abs() < 1e-15);
    // a lossy, displaced, squeezed state at an angle integrates to 1 on a grid
    let g = Gaussian::vacuum(1)
        .squeezed(0, 0.8, 1.1)
        .unwrap()
        .lossy(0, 0.7)
        .unwrap()
        .displaced(0, c64::new(0.5, -0.3))
        .unwrap();
    let (h, n) = (0.05, 640);
    let mut total = 0.0;
    for i in 0..n {
        for j in 0..n {
            let (x, p) = (-16.0 + h * i as f64, -16.0 + h * j as f64);
            total += g.wigner(0, x, p).unwrap();
        }
    }
    assert!((total * h * h - 1.0).abs() < 1e-9, "{}", total * h * h);
    // a coherent state's peak sits at its means, (2 Re α, 2 Im α)
    let c = Gaussian::vacuum(1)
        .displaced(0, c64::new(0.5, -0.3))
        .unwrap();
    assert!((c.wigner(0, 1.0, -0.6).unwrap() - 1.0 / TAU).abs() < 1e-15);
    assert!(c.wigner(1, 0.0, 0.0).is_err());
}

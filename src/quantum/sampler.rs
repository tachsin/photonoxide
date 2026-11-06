//! An exact boson sampler: P. Clifford and R. Clifford's algorithm B (Proc. 29th ACM-SIAM Symp.
//! Discrete Algorithms (2018), 146, doi:10.1137/1.9781611975031.10), O(n 2ⁿ + m n²) per sample
//! for n photons in m modes.
//!
//! The input photons' columns of T are put in a random order α; photon k's output mode is drawn
//! given the first k − 1, with weights |Per T[r₁..r_{k−1}, x; α₁..α_k]|² over the modes x, each
//! a Laplace expansion along the last row whose minors all come from one pass of Glynn's formula
//! (their Lemma 3.2). The draws, sorted, are a sample of the output pattern.

use num_complex::Complex64 as c64;

use super::fock::Fock;
use super::matrix::Matrix;
use super::permanent::minors;
use super::rng::Rng;
use crate::{Error, Result};

/// The most photons the sampler takes.
const LARGEST: usize = 30;

/// `count` samples of the output pattern of indistinguishable photons in `input` through T,
/// whose columns (the input modes) are orthonormal: a unitary, or a lossy circuit's isometry
/// ([`super::Transfer::isometry`]). Sample i uses its own stream of the generator, seeded by
/// `seed` and i, so the samples are the same on any number of threads and the first k of a
/// longer run are a shorter run's.
///
/// ```
/// use photonoxide::quantum::{Fock, haar_unitary, sample};
/// let u = haar_unitary(4, 7);
/// let input = Fock::single_photons(4, &[0, 1])?;
/// let samples = sample(&u, &input, 100, 42)?;
/// assert_eq!(samples, sample(&u, &input, 100, 42)?);
/// assert!(samples.iter().all(|s| s.photons() == 2));
/// # Ok::<(), photonoxide::Error>(())
/// ```
///
/// # Errors
///
/// [`Error::InvalidValue`] when the input doesn't fit T's columns, for more than 30 photons,
/// and for T's columns of the photons not orthonormal (to 1e-9).
pub fn sample(t: &Matrix, input: &Fock, count: usize, seed: u64) -> Result<Vec<Fock>> {
    if input.modes() != t.cols() {
        return Err(Error::invalid(
            "sampler",
            format!(
                "an input over {} modes doesn't fit {} columns",
                input.modes(),
                t.cols()
            ),
        ));
    }
    let n = input.photons();
    if n > LARGEST {
        return Err(Error::invalid(
            "sampler",
            format!("takes up to {LARGEST} photons, got {n}"),
        ));
    }
    let occupied: Vec<usize> = (0..t.cols())
        .filter(|&k| input.occupations()[k] > 0)
        .collect();
    let error = t
        .select(&(0..t.rows()).collect::<Vec<_>>(), &occupied)
        .isometry_error();
    if error > 1e-9 {
        return Err(Error::invalid(
            "sampler",
            format!(
                "the input modes' columns of T must be orthonormal (a unitary or an isometry), \
                 off by {error:.1e}"
            ),
        ));
    }
    let columns = input.mode_list();
    let indices: Vec<u64> = (0..count as u64).collect();
    Ok(crate::parallel::map_in_order(&indices, |&i| {
        one(t, &columns, &mut Rng::new(seed, i))
    }))
}

/// One sample.
fn one(t: &Matrix, columns: &[usize], rng: &mut Rng) -> Fock {
    let m = t.rows();
    let n = columns.len();
    // a random order of the photons, Fisher–Yates
    let mut alpha: Vec<usize> = columns.to_vec();
    for i in (1..n).rev() {
        let j = rng.below(i + 1);
        alpha.swap(i, j);
    }
    let a = |row: usize, l: usize| t[(row, alpha[l])];
    let mut r: Vec<usize> = Vec::with_capacity(n);
    let mut weights = vec![0.0; m];
    for k in 1..=n {
        if k == 1 {
            for (x, w) in weights.iter_mut().enumerate() {
                *w = a(x, 0).norm_sqr();
            }
        } else {
            let b = Matrix::from_fn(k - 1, k, |i, l| a(r[i], l));
            let per = minors(&b);
            for (x, w) in weights.iter_mut().enumerate() {
                let sum: c64 = (0..k).map(|l| a(x, l) * per[l]).sum();
                *w = sum.norm_sqr();
            }
        }
        r.push(rng.pick(&weights));
    }
    let mut occupations = vec![0; m];
    for x in r {
        occupations[x] += 1;
    }
    Fock::new(occupations)
}

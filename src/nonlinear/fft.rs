//! A radix-2 fast Fourier transform for the split-step integrators of [`super::pulse`] and
//! [`super::resonator`]: iterative Cooley–Tukey on power-of-two lengths, in place, on one
//! thread, its sums in a fixed order, so a result is the same bit for bit whatever the number of
//! threads. The twiddle factors are each computed from its own angle, not by a recurrence, so
//! their error stays at round-off.

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// A transform of one length: its twiddle factors and bit-reversal permutation, computed once.
#[derive(Clone, Debug)]
pub(crate) struct Fft {
    n: usize,
    /// e^{−2πik/n}, k = 0 … n/2 − 1
    twiddles: Vec<c64>,
    /// the bit-reversed index of each index
    reversed: Vec<usize>,
}

impl Fft {
    /// The transform of length `n`, a power of two of at least 2.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't a power of two of at least 2.
    pub(crate) fn new(n: usize) -> Result<Fft> {
        if n < 2 || !n.is_power_of_two() {
            return Err(Error::invalid(
                "fft",
                format!("needs a power of two of at least 2 points, got {n}"),
            ));
        }
        let bits = n.trailing_zeros();
        let twiddles = (0..n / 2)
            .map(|k| {
                let a = -std::f64::consts::TAU * k as f64 / n as f64;
                c64::new(a.cos(), a.sin())
            })
            .collect();
        let reversed = (0..n)
            .map(|i| i.reverse_bits() >> (usize::BITS - bits))
            .collect();
        Ok(Fft {
            n,
            twiddles,
            reversed,
        })
    }

    /// X_k = Σ_j x_j e^{−2πijk/n}, in place.
    pub(crate) fn forward(&self, x: &mut [c64]) {
        self.transform(x, false);
    }

    /// x_j = (1/n) Σ_k X_k e^{+2πijk/n}, in place: the inverse of [`Fft::forward`].
    pub(crate) fn inverse(&self, x: &mut [c64]) {
        self.transform(x, true);
        let s = 1.0 / self.n as f64;
        for v in x.iter_mut() {
            *v *= s;
        }
    }

    fn transform(&self, x: &mut [c64], conjugate: bool) {
        debug_assert_eq!(x.len(), self.n);
        for (i, &j) in self.reversed.iter().enumerate() {
            if i < j {
                x.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= self.n {
            let half = size / 2;
            let stride = self.n / size;
            for start in (0..self.n).step_by(size) {
                for k in 0..half {
                    let w = self.twiddles[k * stride];
                    let w = if conjugate { w.conj() } else { w };
                    let a = x[start + k];
                    let b = x[start + k + half] * w;
                    x[start + k] = a + b;
                    x[start + k + half] = a - b;
                }
            }
            size *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_direct_sum_and_inverts() {
        let n = 64;
        let fft = Fft::new(n).unwrap();
        let x: Vec<c64> = (0..n)
            .map(|j| c64::new((0.3 * j as f64).sin(), (0.11 * (j * j) as f64).cos()))
            .collect();
        let mut y = x.clone();
        fft.forward(&mut y);
        for (k, yk) in y.iter().enumerate() {
            let direct: c64 = x
                .iter()
                .enumerate()
                .map(|(j, &v)| {
                    let a = -std::f64::consts::TAU * (j * k) as f64 / n as f64;
                    v * c64::new(a.cos(), a.sin())
                })
                .sum();
            assert!((direct - yk).norm() < 1e-12, "{k}: {direct} {yk}");
        }
        fft.inverse(&mut y);
        for (a, b) in x.iter().zip(&y) {
            assert!((a - b).norm() < 1e-14);
        }
        assert!(Fft::new(48).is_err());
    }
}

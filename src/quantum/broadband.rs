//! Broadband pulsed squeezing at any gain: the Bogoliubov map of a non-degenerate parametric
//! source, whose squeezed modes change with the gain.
//!
//! # The map
//!
//! A pump that doesn't deplete couples the signal's frequencies a_j to the idler's b_k through
//! the joint spectral amplitude. Over a short length of the medium the coupling is a matrix G
//! (signal × idler), and the operators change as
//!
//! d a/dz = G b†,   d b†/dz = G† a,
//!
//! so the slice maps (a, b†) by the exponential of the Hermitian generator K = [[0, G], [G†, 0]]:
//! with the singular values G = U S V†, exp K = [[I + U (cosh S − I) U†, U sinh S V†], [V sinh S
//! U†, I + V (cosh S − I) V†]]. The medium is the ordered product of its slices' maps, the last
//! slice on the left. Each map keeps the commutators: with Σ = diag(I, −I), M Σ M† = Σ (the
//! complex form of the symplectic condition). The output's squeezing comes from the Bloch–Messiah
//! decomposition: the singular values of the block B (a_out = A a + B b†) are sinh r_k, their
//! left vectors the signal's squeezed modes, and the signal's mean photon number is Tr(B B†) =
//! Σ sinh² r_k.
//!
//! The slices' generators don't commute when the phase matching varies along the medium (a
//! dispersive guide or crystal): the ordered product then departs from the single exponential of
//! the whole joint spectrum, and its squeezed modes and squeezing change with the gain (the
//! "time-ordering" of the propagation). At low gain the product is the exponential of the sum,
//! whose singular values are g √λ_k, the Schmidt model of [`super::Source`]; with one frequency,
//! or with slices that commute, it is that exponential at every gain.
//!
//! The approach is W. Wasilewski, A. I. Lvovsky, K. Banaszek, C. Radzewicz, "Pulsed squeezed
//! light: Simultaneous squeezing of multiple modes", Phys. Rev. A 73, 063819 (2006),
//! doi:10.1103/PhysRevA.73.063819, and, for down-conversion at high gain, A. Christ, B. Brecht,
//! W. Mauerer, C. Silberhorn, "Theory of quantum frequency conversion and type-II parametric
//! down-conversion in the high-gain regime", New J. Phys. 15, 053038 (2013),
//! doi:10.1088/1367-2630/15/5/053038. Neither paper is in hand: they are cited by title, and the
//! equations above are the standard Heisenberg equations of a quadratic Hamiltonian, checked
//! here against their closed forms (docs/methods/quantum.md).
//!
//! Lossless and undepleted; the pump's self- and cross-phase modulation (four-wave mixing's)
//! are left out.

use faer::Mat;
use num_complex::Complex64 as c64;

use super::{Gaussian, Matrix};
use crate::dense::fixed;
use crate::nonlinear::pairs::JointSpectrum;
use crate::portable;
use crate::{Error, Result};

fn invalid(what: impl Into<String>) -> Error {
    Error::invalid("broadband squeezing", what)
}

/// The Bogoliubov map of a broadband source: a_out = A a + B b†, b_out† = C a + D b†, signal
/// frequencies first.
#[derive(Clone, Debug, PartialEq)]
pub struct Broadband {
    signal: usize,
    idler: usize,
    /// The map, (signal + idler)², row by row.
    map: Vec<c64>,
    /// r_k, largest first.
    squeezing: Vec<f64>,
    /// The signal's squeezed modes, one per r_k, each normalized: the left singular vectors of B.
    modes: Vec<Vec<c64>>,
}

fn to_faer(m: &Matrix) -> Mat<c64> {
    Mat::from_fn(m.rows(), m.cols(), |i, j| m[(i, j)])
}

/// The map of one slice of generator g (signal × idler), exp [[0, g], [g†, 0]].
fn slice_map(g: &Mat<c64>) -> Result<Mat<c64>> {
    let (ns, ni) = (g.nrows(), g.ncols());
    let svd = fixed::thin_svd(g.as_ref())
        .map_err(|e| invalid(format!("a slice's singular values: {e:?}")))?;
    let (u, v) = (svd.u.as_ref(), svd.v.as_ref());
    let s: Vec<f64> = svd.s.column_vector().iter().map(|x| x.re).collect();
    let k = s.len();
    let n = ns + ni;
    let mut m = Mat::<c64>::zeros(n, n);
    for i in 0..n {
        m[(i, i)] = c64::new(1.0, 0.0);
    }
    for q in 0..k {
        let (ch, sh) = (portable::cosh(s[q]) - 1.0, portable::sinh(s[q]));
        for i in 0..ns {
            for j in 0..ns {
                m[(i, j)] += u[(i, q)] * u[(j, q)].conj() * ch;
            }
            for j in 0..ni {
                m[(i, ns + j)] += u[(i, q)] * v[(j, q)].conj() * sh;
                m[(ns + j, i)] += v[(j, q)] * u[(i, q)].conj() * sh;
            }
        }
        for i in 0..ni {
            for j in 0..ni {
                m[(ns + i, ns + j)] += v[(i, q)] * v[(j, q)].conj() * ch;
            }
        }
    }
    Ok(m)
}

impl Broadband {
    /// The map of a medium cut into slices, each slice's generator G (signal × idler, the
    /// coupling over its length, in the units that make d a/dz = G b†), first slice first: the
    /// ordered product of their exponentials.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no slices, slices of different shapes or an empty one, a value
    /// that isn't finite, or a decomposition that fails.
    pub fn new(generators: &[Matrix]) -> Result<Broadband> {
        let first = generators
            .first()
            .ok_or_else(|| invalid("needs at least one slice"))?;
        let (ns, ni) = (first.rows(), first.cols());
        if ns == 0 || ni == 0 {
            return Err(invalid(
                "a slice needs at least one signal and one idler frequency",
            ));
        }
        if generators.iter().any(|g| g.rows() != ns || g.cols() != ni) {
            return Err(invalid(
                "every slice must have the same signal × idler shape",
            ));
        }
        let mut total = Mat::<c64>::identity(ns + ni, ns + ni);
        for g in generators {
            let g = to_faer(g);
            if (0..ns)
                .any(|i| (0..ni).any(|j| !(g[(i, j)].re.is_finite() && g[(i, j)].im.is_finite())))
            {
                return Err(invalid("a slice's coupling must be finite"));
            }
            total = slice_map(&g)? * &total;
        }
        Broadband::from_map(ns, ni, total)
    }

    /// The single exponential of a joint spectrum's generator, scaled so that its singular
    /// values are g √λ_k: the Schmidt model of [`super::Source`], the map without the
    /// propagation's ordering, exact at any gain where the slices commute.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a gain that isn't finite and > 0, a spectrum that is zero, or
    /// a decomposition that fails.
    pub fn exponential(spectrum: &JointSpectrum, gain: f64) -> Result<Broadband> {
        if !(gain.is_finite() && gain > 0.0) {
            return Err(invalid(format!(
                "the gain must be finite and positive, got {gain}"
            )));
        }
        let (ns, ni) = (spectrum.signal.len(), spectrum.idler.len());
        let norm: f64 = spectrum
            .amplitude
            .iter()
            .map(|a| a.norm_sqr())
            .sum::<f64>()
            .sqrt();
        if !(norm.is_finite() && norm > 0.0) {
            return Err(invalid("the joint spectrum is zero"));
        }
        let g = Matrix::from_fn(ns, ni, |i, j| {
            spectrum.amplitude[i * ni + j] * (gain / norm)
        });
        Broadband::new(&[g])
    }

    /// A down-conversion source cut into `slices` along its length: on the signal and idler
    /// grids (1/µm), the pump's amplitude α(ν_s + ν_i) and the phase mismatch x(ν_s, ν_i) = ΔkL/2
    /// (as a device's `phase_at_frequencies` gives it), slice n of `slices` couples by
    /// α e^{2ix t_n}/N at its middle t_n = (n + ½)/N − ½ of the length. Their sum is α sinc x,
    /// the joint spectrum's amplitude ([`crate::nonlinear::pairs::joint_spectrum`]'s), on the
    /// midpoint rule; `gain` scales it to the Schmidt model's g, its singular values g √λ_k.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than one point on a grid or one slice, a gain that isn't
    /// finite and > 0, a spectrum that is zero, and the errors of `mismatch`.
    pub fn sliced(
        signal: &[f64],
        idler: &[f64],
        pump: impl Fn(f64) -> c64,
        mismatch: impl Fn(f64, f64) -> Result<f64>,
        slices: usize,
        gain: f64,
    ) -> Result<Broadband> {
        if signal.is_empty() || idler.is_empty() || slices == 0 {
            return Err(invalid(
                "needs a frequency on each grid and at least one slice",
            ));
        }
        if !(gain.is_finite() && gain > 0.0) {
            return Err(invalid(format!(
                "the gain must be finite and positive, got {gain}"
            )));
        }
        let (ns, ni) = (signal.len(), idler.len());
        let mut alpha = Vec::with_capacity(ns * ni);
        let mut x = Vec::with_capacity(ns * ni);
        for &s in signal {
            for &i in idler {
                alpha.push(pump(s + i));
                x.push(mismatch(s, i)?);
            }
        }
        let n = slices as f64;
        let at = |t: f64| -> Vec<c64> {
            alpha
                .iter()
                .zip(&x)
                .map(|(a, x)| a * portable::complex::from_polar(1.0 / n, 2.0 * x * t))
                .collect()
        };
        let mids: Vec<f64> = (0..slices).map(|k| (k as f64 + 0.5) / n - 0.5).collect();
        let mut sum = vec![c64::new(0.0, 0.0); ns * ni];
        for &t in &mids {
            for (s, v) in sum.iter_mut().zip(at(t)) {
                *s += v;
            }
        }
        let norm = sum.iter().map(|a| a.norm_sqr()).sum::<f64>().sqrt();
        if !(norm.is_finite() && norm > 0.0) {
            return Err(invalid("the joint spectrum is zero"));
        }
        let scale = gain / norm;
        let generators: Vec<Matrix> = mids
            .iter()
            .map(|&t| {
                let v = at(t);
                Matrix::from_fn(ns, ni, |i, j| v[i * ni + j] * scale)
            })
            .collect();
        Broadband::new(&generators)
    }

    fn from_map(signal: usize, idler: usize, map: Mat<c64>) -> Result<Broadband> {
        let b = Mat::<c64>::from_fn(signal, idler, |i, j| map[(i, signal + j)]);
        let svd = fixed::thin_svd(b.as_ref())
            .map_err(|e| invalid(format!("the map's singular values: {e:?}")))?;
        let s: Vec<f64> = svd.s.column_vector().iter().map(|x| x.re).collect();
        let mut order: Vec<usize> = (0..s.len()).collect();
        order.sort_by(|&p, &q| s[q].total_cmp(&s[p]));
        let u = svd.u.as_ref();
        let n = signal + idler;
        Ok(Broadband {
            signal,
            idler,
            map: (0..n * n).map(|k| map[(k / n, k % n)]).collect(),
            squeezing: order.iter().map(|&k| portable::asinh(s[k])).collect(),
            modes: order
                .iter()
                .map(|&k| (0..signal).map(|i| u[(i, k)]).collect())
                .collect(),
        })
    }

    /// The squeezing r_k of each pair of squeezed modes, largest first: asinh of B's singular
    /// values.
    pub fn squeezing(&self) -> &[f64] {
        &self.squeezing
    }

    /// The signal's k-th squeezed mode over its frequencies, normalized (its phase is the
    /// decomposition's).
    pub fn signal_mode(&self, k: usize) -> Option<&[c64]> {
        self.modes.get(k).map(Vec::as_slice)
    }

    /// The map as a matrix: rows and columns the signal's frequencies, then the idler's.
    pub fn map(&self) -> Matrix {
        let n = self.signal + self.idler;
        Matrix::from_fn(n, n, |i, j| self.map[i * n + j])
    }

    /// The signal's mean photon number, Tr(B B†), from the map itself.
    pub fn mean_photons(&self) -> f64 {
        let n = self.signal + self.idler;
        (0..self.signal)
            .map(|i| {
                (0..self.idler)
                    .map(|j| self.map[i * n + self.signal + j].norm_sqr())
                    .sum::<f64>()
            })
            .sum()
    }

    /// The largest entry of M Σ M† − Σ, Σ = diag(I, −I), over the square of M's largest entry:
    /// zero for a map that keeps the commutators.
    pub fn symplectic_error(&self) -> f64 {
        let n = self.signal + self.idler;
        let sign = |k: usize| if k < self.signal { 1.0 } else { -1.0 };
        let largest = self
            .map
            .iter()
            .map(|v| portable::complex::abs(*v))
            .fold(0.0, f64::max);
        let mut worst: f64 = 0.0;
        for i in 0..n {
            for j in 0..n {
                let mut v: c64 = (0..n)
                    .map(|k| self.map[i * n + k] * self.map[j * n + k].conj() * sign(k))
                    .sum();
                if i == j {
                    v -= sign(i);
                }
                worst = worst.max(portable::complex::abs(v));
            }
        }
        worst / (largest * largest)
    }

    /// The effective number of squeezed modes, (Σ sinh² r_k)² / Σ sinh⁴ r_k: the Schmidt number
    /// 1/Σλ_k² at low gain.
    pub fn mode_number(&self) -> f64 {
        let n: Vec<f64> = self
            .squeezing
            .iter()
            .map(|r| portable::powi(portable::sinh(*r), 2))
            .collect();
        let total: f64 = n.iter().sum();
        total * total / n.iter().map(|x| x * x).sum::<f64>()
    }

    /// The source as a Gaussian state of its first `modes` pairs of squeezed modes, as
    /// [`super::Source::gaussian`]: mode k the signal's k-th, mode `modes + k` the idler's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no modes.
    pub fn gaussian(&self, modes: usize) -> Result<Gaussian> {
        let k = modes.min(self.squeezing.len());
        if k == 0 {
            return Err(invalid("a Gaussian state needs at least one pair of modes"));
        }
        let mut g = Gaussian::vacuum(2 * k);
        for (j, &r) in self.squeezing.iter().take(k).enumerate() {
            g = g.two_mode_squeezed(j, k + j, r, 0.0)?;
        }
        Ok(g)
    }
}

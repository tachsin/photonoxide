//! A circuit's transfer matrix at a wavelength, as photons see it, with its loss dilated to
//! noise modes.

use num_complex::Complex64 as c64;

use super::fock::{Distribution, Fock, Photons};
use super::matrix::{Matrix, hermitian_eigen};
use super::sampler::sample;
use crate::circuit::{Circuit, Component, SMatrix};
use crate::units::Wavelength;
use crate::{Error, Result};

/// An eigenvalue of I − S†S below this is no loss: round-off of a lossless channel.
const NO_LOSS: f64 = 1e-13;
/// An eigenvalue of I − S†S below minus this is gain: S isn't passive.
const GAIN: f64 = 1e-9;

/// A circuit's S-matrix as the transformation of its ports' modes, with its loss.
///
/// Every port is an input mode (light going in) and an output mode (light coming out), so an
/// n-port's S is n × n, reflections included. A lossy S keeps the commutators only with noise
/// modes: b̂ = S â + L ĉ, the vacuum ĉ entering through L L† = I − S S†. The photons that the
/// loss takes leave through loss modes, ê = K â + …, with S†S + K†K = I: the transfer from the
/// input modes to the outputs and the loss modes together, [S; K], is an isometry, and its
/// probabilities sum to 1 ([`Transfer::isometry`]). K and L have one row (column) per channel
/// that loses power, the eigenvectors of I − S†S (I − S S†) with a non-zero eigenvalue, scaled
/// by its square root.
#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    ports: Vec<String>,
    s: Matrix,
    /// K, r × n: the input modes into the loss modes.
    loss: Matrix,
    /// L, n × r′: the noise modes into the output modes.
    noise: Matrix,
}

impl Transfer {
    /// The circuit's transfer at `wavelength`: its S-matrix between its external ports, with
    /// its loss dilated.
    ///
    /// # Errors
    ///
    /// The circuit's own, and as [`Transfer::new`].
    pub fn of(circuit: &Circuit, wavelength: Wavelength) -> Result<Transfer> {
        let s = circuit.s_matrix(wavelength)?;
        let names = circuit.ports().iter().map(|p| p.name.clone()).collect();
        Transfer::new(&s, names)
    }

    /// The transfer of an S-matrix whose ports have the given names.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless there is one name per port, and for an S that isn't
    /// passive (an eigenvalue of S†S above 1 + 1e-9): gain can't be dilated with vacuum.
    pub fn new(s: &SMatrix, ports: Vec<String>) -> Result<Transfer> {
        let n = s.size();
        if ports.len() != n {
            return Err(Error::invalid(
                "transfer",
                format!("{} port names for {n} ports", ports.len()),
            ));
        }
        let s = Matrix::from(s);
        let loss = square_root_rows(&s.adjoint().product(&s)?)?;
        let noise = square_root_rows(&s.product(&s.adjoint())?)?.adjoint();
        Ok(Transfer {
            ports,
            s,
            loss,
            noise,
        })
    }

    /// The ports' names, in the order of S's rows and columns.
    pub fn ports(&self) -> &[String] {
        &self.ports
    }

    /// The index of the port named `name`.
    pub fn port(&self, name: &str) -> Option<usize> {
        self.ports.iter().position(|p| p == name)
    }

    /// S, n × n.
    pub fn matrix(&self) -> &Matrix {
        &self.s
    }

    /// The number of loss modes: the channels that lose power.
    pub fn loss_modes(&self) -> usize {
        self.loss.rows()
    }

    /// L, n × r′: how the noise modes' vacuum enters the outputs, L L† = I − S S†.
    pub fn noise(&self) -> &Matrix {
        &self.noise
    }

    /// [S; K], (n + r) × n: the ports' input modes into the ports' output modes and the loss
    /// modes. Its columns are orthonormal.
    pub fn isometry(&self) -> Matrix {
        let n = self.s.rows();
        Matrix::from_fn(n + self.loss.rows(), n, |i, j| {
            if i < n {
                self.s[(i, j)]
            } else {
                self.loss[(i - n, j)]
            }
        })
    }

    /// How far the dilation is from exact: the largest entry of |S S† + L L† − I| and of
    /// |S†S + K†K − I|. Round-off.
    pub fn dilation_error(&self) -> f64 {
        let n = self.s.rows();
        let gram = |a: &Matrix, i: usize, j: usize| -> c64 {
            (0..a.cols()).map(|k| a[(i, k)] * a[(j, k)].conj()).sum()
        };
        let out = Matrix::from_fn(n, n, |i, j| gram(&self.s, i, j) + gram(&self.noise, i, j));
        out.max_difference(&Matrix::identity(n))
            .max(self.isometry().isometry_error())
    }

    /// A Fock state on the ports: `photons[k]` is (port name, number of photons).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a name that isn't a port.
    pub fn input(&self, photons: &[(&str, usize)]) -> Result<Fock> {
        let mut m = vec![0; self.ports.len()];
        for &(name, count) in photons {
            let k = self.port(name).ok_or_else(|| {
                Error::invalid(
                    "photon input",
                    format!("no port {name}; the ports are {}", self.ports.join(", ")),
                )
            })?;
            m[k] += count;
        }
        Ok(Fock::new(m))
    }

    /// The distribution of the photons detected at the ports for photons in `input`: every
    /// pattern over the ports and loss modes, the loss modes then summed out, so a pattern with
    /// fewer photons is one where some were lost. Sums to 1.
    ///
    /// # Errors
    ///
    /// As [`super::probability`].
    pub fn distribution(&self, input: &Fock, photons: &Photons) -> Result<Distribution> {
        Ok(Distribution::exact(&self.isometry(), input, photons)?.detected(self.ports.len()))
    }

    /// `count` samples of the photons detected at the ports, from Clifford and Clifford's
    /// exact sampler through the isometry (indistinguishable photons), the loss modes dropped.
    ///
    /// # Errors
    ///
    /// As [`super::sample`].
    pub fn sample(&self, input: &Fock, count: usize, seed: u64) -> Result<Vec<Fock>> {
        let n = self.ports.len();
        Ok(sample(&self.isometry(), input, count, seed)?
            .into_iter()
            .map(|f| f.truncated(n))
            .collect())
    }
}

/// The rows of a matrix K with K†K = I − H for a Hermitian H with eigenvalues at most 1: one
/// row √λ v† per eigenpair of I − H with λ above round-off.
fn square_root_rows(h: &Matrix) -> Result<Matrix> {
    let n = h.rows();
    let minus = Matrix::from_fn(n, n, |i, j| {
        let one = if i == j { 1.0 } else { 0.0 };
        c64::new(one, 0.0) - h[(i, j)]
    });
    let (values, vectors) = hermitian_eigen(&minus);
    if let Some(&low) = values.iter().find(|&&v| v < -GAIN) {
        return Err(Error::invalid(
            "transfer",
            format!(
                "S isn't passive: a singular value of {:.12} is above 1, and gain can't be \
                 dilated with vacuum noise modes",
                (1.0 - low).sqrt()
            ),
        ));
    }
    let kept: Vec<usize> = (0..n).filter(|&k| values[k] > NO_LOSS).collect();
    Ok(Matrix::from_fn(kept.len(), n, |r, j| {
        let k = kept[r];
        vectors[(j, k)].conj() * values[k].sqrt()
    }))
}

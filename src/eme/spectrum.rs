//! A structure's S-matrix over wavelength from a few solves, interpolated with its error stated
//! ([`Spectrum`]).

use num_complex::Complex64 as c64;

use super::Scattering;
use crate::sweep::{Options, RationalSet, SweepSet};
use crate::units::Wavelength;
use crate::{Error, Result};

/// A structure's S-matrix between the first modes at its two ends as a function of wavelength:
/// every entry a rational function of the wavelength, all of them on one set of solves and
/// sharing one denominator, with each entry's error estimated ([`Spectrum::run`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Spectrum {
    /// The sweep over the wavelength (µm) of S's entries, row by row of the full matrix.
    pub sweep: SweepSet,
    /// The modes at port 1 and at port 2.
    pub ports: (usize, usize),
    /// The ports' modes' effective indices (port 1's, then port 2's), fitted by set-valued AAA
    /// on the same solves: for labelling the entries, without an estimate of their own.
    pub indices: RationalSet,
}

impl Spectrum {
    /// The S-matrix that `solve` gives at a wavelength, between the first `ports.0` modes at the
    /// first end and the first `ports.1` at the last, sampled from `from` to `to` until the
    /// estimated error of every entry is at most `options.tolerance` times the largest |S_qp|
    /// sampled ([`SweepSet::run`]: one set of wavelengths for all the entries, placed where the
    /// worst one needs a sample, and set-valued AAA with one denominator for all of them). S's
    /// entries are at most 1 in a passive structure, so the tolerance is about an absolute one on
    /// S. Every solve should be the same discretisation (modes, steps) at another wavelength, so
    /// that S is one smooth function of it: a solve whose number of modes or steps changes with
    /// the wavelength gives S jumps, which the estimate sees as error.
    ///
    /// The shared denominator keeps any linear relation that every solve's S keeps: S = Sᵀ
    /// (reciprocity) holds for the interpolant to round-off wherever it holds for the solves.
    /// Power is a quadratic relation: the interpolant keeps it to its stated error.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no modes at a port, a solve with fewer modes at a port than
    /// asked, and [`SweepSet::run`]'s and `solve`'s own errors.
    pub fn run(
        mut solve: impl FnMut(Wavelength) -> Result<Scattering>,
        from: Wavelength,
        to: Wavelength,
        ports: (usize, usize),
        options: &Options,
    ) -> Result<Spectrum> {
        let (n1, n2) = ports;
        if n1 == 0 || n2 == 0 {
            return Err(Error::invalid(
                "eme spectrum",
                format!("needs at least one mode at each port, got {n1} and {n2}"),
            ));
        }
        let mut indices: Vec<(f64, Vec<c64>)> = Vec::new();
        let sweep = SweepSet::run(
            |l| {
                let s = solve(Wavelength::um(l)?)?;
                if s.first.len() < n1 || s.last.len() < n2 {
                    return Err(Error::invalid(
                        "eme spectrum",
                        format!(
                            "the solve at {l} um has {} and {} modes at its ports, fewer than \
                             the {n1} and {n2} asked",
                            s.first.len(),
                            s.last.len()
                        ),
                    ));
                }
                let n: Vec<c64> = s.first[..n1].iter().chain(&s.last[..n2]).copied().collect();
                indices.push((l, n));
                Ok(entries(&s, ports))
            },
            from.to_um(),
            to.to_um(),
            options,
        )?;
        indices.sort_by(|a, b| a.0.total_cmp(&b.0));
        let x: Vec<f64> = indices.iter().map(|s| s.0).collect();
        let f: Vec<Vec<c64>> = indices.into_iter().map(|s| s.1).collect();
        let indices = RationalSet::aaa(&x, &f, crate::sweep::DEFAULT_TOLERANCE)?;
        Ok(Spectrum {
            sweep,
            ports,
            indices,
        })
    }

    /// The interpolated S-matrix at `wavelength`, (n₁ + n₂) × (n₁ + n₂): rows and columns port
    /// 1's modes, then port 2's, `s[q][p]` the amplitude leaving in mode q for mode p entering.
    pub fn at(&self, wavelength: Wavelength) -> Vec<Vec<c64>> {
        let n = self.ports.0 + self.ports.1;
        let v = self.sweep.eval(wavelength.to_um());
        v.chunks(n).map(<[c64]>::to_vec).collect()
    }

    /// The ports' modes' effective indices at `wavelength`, port 1's then port 2's, from their
    /// fit through the solves.
    pub fn effective_indices(&self, wavelength: Wavelength) -> Vec<c64> {
        self.indices.eval(wavelength.to_um())
    }

    /// Each entry's estimated error, in S's unit, as [`Spectrum::at`] lays them out.
    pub fn errors(&self) -> Vec<Vec<f64>> {
        let n = self.ports.0 + self.ports.1;
        self.sweep.errors.chunks(n).map(<[f64]>::to_vec).collect()
    }

    /// The worst entry's estimated error, in S's unit: the largest over the range.
    pub fn error(&self) -> f64 {
        self.sweep.error
    }

    /// The wavelengths solved at, µm, in order.
    pub fn wavelengths(&self) -> Vec<f64> {
        self.sweep.samples.iter().map(|s| s.0).collect()
    }

    /// The number of solves made.
    pub fn solves(&self) -> usize {
        self.sweep.calls()
    }

    /// Whether every entry's estimate reached the tolerance.
    pub fn converged(&self) -> bool {
        self.sweep.converged
    }
}

/// The entries of `s` between the first `n1` modes at port 1 and `n2` at port 2, row by row of
/// the full (n₁ + n₂)-square matrix.
pub(crate) fn entries(s: &Scattering, (n1, n2): (usize, usize)) -> Vec<c64> {
    let n = n1 + n2;
    let mut out = Vec::with_capacity(n * n);
    for q in 0..n {
        for p in 0..n {
            out.push(match (q < n1, p < n1) {
                (true, true) => s.s11[q][p],
                (true, false) => s.s12[q][p - n1],
                (false, true) => s.s21[q - n1][p],
                (false, false) => s.s22[q - n1][p - n1],
            });
        }
    }
    out
}

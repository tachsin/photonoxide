//! Periodic sections: a period's Bloch modes from its S-matrix, and a structure with a period
//! repeated N times by repeated squaring of the period's star product ([`Bloch`]).

use std::ops::Range;

use faer::Mat;
use num_complex::Complex64 as c64;

use super::scattering::{Blocks, rows, star};
use super::{Eme, Eme3d, Scattering, Section, Section3d};
use crate::units::Length;
use crate::{Error, Result};

/// A period's Bloch modes: the waves that each period multiplies by e^(iKΛ), K the Bloch
/// wavenumber and Λ the period ([`Eme::bloch`]).
///
/// With a₁ = (a₁⁺, a₁⁻) the amplitudes of the period's first section's modes at its start and
/// a₂ at the start of the next period (the same modes), its S-matrix gives
/// a₂ = T a₁, T = [[S₂₁ − S₂₂S₁₂⁻¹S₁₁, S₂₂S₁₂⁻¹], [−S₁₂⁻¹S₁₁, S₁₂⁻¹]], and a Bloch mode is an
/// eigenvector of T with the eigenvalue λ = e^(iKΛ) (P. Yeh, A. Yariv, C.-S. Hong, J. Opt. Soc.
/// Am. 67, 423 (1977), [doi:10.1364/JOSA.67.000423](https://doi.org/10.1364/JOSA.67.000423),
/// Eqs. (24)–(27), there for one mode). S₁₂ of an evanescent mode is exponentially small, so T
/// itself is ill-conditioned; the eigenvalues are those of the pencil
/// [[S₂₁, 0], [−S₁₁, 1]] x = λ [[1, −S₂₂], [0, S₁₂]] x, the same equations multiplied out,
/// which inverts nothing (QZ). In a reciprocal structure they come in pairs λ and 1/λ, the
/// same Bloch mode going either way, and each pair is one mode here: K from the one of the pair
/// with |λ| ≤ 1, the wave that decays (or keeps its power) forward.
///
/// The m-th Bloch mode is the one most made of the section's m-th mode (going either way), of
/// those not taken before it: the fundamental Bloch mode first, with the section's modes'
/// order. In a stop band it is the one the section's fundamental mode excites most.
///
/// A Bloch mode decaying by e^(−Im KΛ) per period has λ = e^(iKΛ) to the eigenproblem's
/// round-off, an absolute one: its K is good to about ε e^(Im KΛ) / Λ, so the modes that decay
/// by more than e^(−20) or so in one period are approximate. The few that matter (guided,
/// propagating or in a stop band) are at round-off.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Bloch {
    /// The period, Λ.
    pub period: Length,
    /// cos KΛ = (λ + 1/λ)/2 of each Bloch mode: real in a lossless period, |cos KΛ| ≤ 1 in a
    /// pass band and > 1 in a stop band.
    pub cos: Vec<c64>,
    /// K/k₀ of each Bloch mode in the reduced zone: Re between 0 and λ₀/(2Λ) (KΛ's phase, up to
    /// its sign, which is the direction), Im ≥ 0 its decay along z.
    pub effective_indices: Vec<c64>,
}

impl Bloch {
    /// The Bloch modes from a period's S-matrix between the same modes at both ends, its length
    /// `period` µm, at the vacuum wavenumber `k0` (1/µm), in the order of [`Bloch::cos`].
    pub(crate) fn of(s: &Scattering, period: f64, k0: f64) -> Result<Bloch> {
        let n = s.first.len();
        if n == 0 || s.last.len() != n {
            return Err(Error::invalid(
                "eme bloch",
                "a period's S-matrix is between the same modes at both ends",
            ));
        }
        let b = blocks(s);
        let zero = c64::new(0.0, 0.0);
        let one = c64::new(1.0, 0.0);
        let a = Mat::from_fn(2 * n, 2 * n, |i, j| match (i < n, j < n) {
            (true, true) => b.s21[(i, j)],
            (true, false) => zero,
            (false, true) => -b.s11[(i - n, j)],
            (false, false) => {
                if i == j {
                    one
                } else {
                    zero
                }
            }
        });
        let m = Mat::from_fn(2 * n, 2 * n, |i, j| match (i < n, j < n) {
            (true, true) => {
                if i == j {
                    one
                } else {
                    zero
                }
            }
            (true, false) => -b.s22[(i, j - n)],
            (false, true) => zero,
            (false, false) => b.s12[(i - n, j - n)],
        });
        let eig = a.generalized_eigen(&m).map_err(|e| {
            Error::invalid("eme bloch", format!("the period's eigenproblem: {e:?}"))
        })?;
        let (alpha, beta) = (eig.S_a(), eig.S_b());
        // ln λ = ln α − ln β: finite or ±∞ in its real part, never NaN unless both vanish
        let logs: Vec<c64> = (0..2 * n).map(|i| alpha[i].ln() - beta[i].ln()).collect();
        // each eigenvector's share in each of the section's modes, both ways
        let u = eig.U();
        let shares: Vec<Vec<f64>> = (0..2 * n)
            .map(|i| {
                let w: Vec<f64> = (0..n)
                    .map(|m| u[(m, i)].norm_sqr() + u[(n + m, i)].norm_sqr())
                    .collect();
                let total: f64 = w.iter().sum();
                w.into_iter().map(|v| v / total).collect()
            })
            .collect();
        Ok(Bloch::paired(&logs, &shares, period, k0))
    }

    /// The Bloch modes from the eigenvalues' logarithms, ln λ, paired λ with 1/λ, and ordered by
    /// the eigenvectors' `shares` in the section's modes.
    ///
    /// The pairs are taken closest first over all of them: a mode that decays by more than
    /// e^(−36) or so in one period has its λ or 1/λ lost to round-off (0 or ∞, or a wrong
    /// magnitude), and its partner then pairs with what is left, never with a guided mode's.
    fn paired(logs: &[c64], shares: &[Vec<f64>], period: f64, k0: f64) -> Bloch {
        // how far ln λ_p + ln λ_q is from 0 (mod 2πi): λ_p λ_q = 1 for a pair
        let apart = |p: usize, q: usize| {
            let s = logs[p] + logs[q];
            let phase = s.im - std::f64::consts::TAU * (s.im / std::f64::consts::TAU).round();
            let d = s.re.hypot(phase);
            if d.is_nan() { f64::INFINITY } else { d }
        };
        let count = logs.len();
        let mut candidates: Vec<(f64, usize, usize)> = (0..count)
            .flat_map(|p| (p + 1..count).map(move |q| (p, q)))
            .map(|(p, q)| (apart(p, q), p, q))
            .collect();
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut taken = vec![false; count];
        // each pair by the one with |λ| ≤ 1, the more strongly decaying forward
        let mut firsts: Vec<usize> = Vec::with_capacity(count / 2);
        for (_, p, q) in candidates {
            if taken[p] || taken[q] {
                continue;
            }
            taken[p] = true;
            taken[q] = true;
            let decaying = logs[p].re.total_cmp(&logs[q].re).then(p.cmp(&q)).is_le();
            firsts.push(if decaying { p } else { q });
        }
        // one left over (an odd count) is a mode of its own
        firsts.extend((0..count).filter(|&p| !taken[p]));
        // the most strongly decaying forward first
        firsts.sort_by(|&p, &q| logs[p].re.total_cmp(&logs[q].re).then(p.cmp(&q)));
        let mut modes: Vec<(c64, c64, usize)> = firsts
            .into_iter()
            .map(|p| {
                // K Λ = −i ln λ for the one of the pair with |λ| ≤ 1: Im ≥ 0
                let l = logs[p];
                let lambda = l.exp();
                let cos = 0.5 * (lambda + (-l).exp());
                let k = c64::new(l.im.abs(), (-l.re).max(0.0)) / (period * k0);
                (cos, k, p)
            })
            .collect();
        // the m-th: of those left, the one with the largest share in the section's m-th mode
        let mut ordered = Vec::with_capacity(modes.len());
        let sections = shares.first().map_or(0, Vec::len);
        while ordered.len() < sections && !modes.is_empty() {
            let m = ordered.len();
            let share = |i: usize| shares[modes[i].2][m];
            let best = (0..modes.len())
                .max_by(|&i, &j| share(i).total_cmp(&share(j)).then(j.cmp(&i)))
                .unwrap_or(0);
            ordered.push(modes.remove(best));
        }
        ordered.append(&mut modes);
        Bloch {
            period: Length::um(period),
            cos: ordered.iter().map(|m| m.0).collect(),
            effective_indices: ordered.iter().map(|m| m.1).collect(),
        }
    }

    /// The Bloch wavenumbers K, 1/µm: [`Bloch::effective_indices`] times k₀ at `wavelength`.
    pub fn wavenumbers(&self, wavelength: crate::units::Wavelength) -> Vec<c64> {
        let k0 = wavelength.wavenumber();
        self.effective_indices.iter().map(|n| n * k0).collect()
    }
}

/// S's blocks as matrices.
pub(crate) fn blocks(s: &Scattering) -> Blocks {
    let m = |v: &[Vec<c64>]| {
        let cols = v.first().map_or(0, Vec::len);
        Mat::from_fn(v.len(), cols, |i, j| v[i][j])
    };
    Blocks {
        s11: m(&s.s11),
        s12: m(&s.s12),
        s21: m(&s.s21),
        s22: m(&s.s22),
    }
}

/// `p` star-multiplied by itself `count` (≥ 1) times, by repeated squaring: about 2 log₂ count
/// star products instead of count − 1.
pub(crate) fn power(p: &Blocks, count: usize) -> Blocks {
    let mut out: Option<Blocks> = None;
    let mut base = p.clone();
    let mut n = count;
    loop {
        if n & 1 == 1 {
            out = Some(match out {
                None => base.clone(),
                Some(o) => star(&o, &base),
            });
        }
        n >>= 1;
        if n == 0 {
            break;
        }
        base = star(&base, &base);
    }
    out.unwrap_or(base)
}

/// The S-matrix of `sections` with `sections[period]` repeated `count` times, from three
/// solves: what comes before the period up to its first section's start, the period from its
/// first section's start to the next one's (repeated `count` − 1 times), and the last period
/// with what follows it, each by `solve`, which `reference` gives a zero-length copy of a
/// section for. A zero-length section between two others would add an interface pair that,
/// with finitely many modes, isn't the identity: the last period ends where the structure goes
/// on, not at a copy of its first section.
fn grating<S>(
    sections: &[S],
    period: Range<usize>,
    count: usize,
    reference: impl Fn(&S) -> S,
    solve: impl Fn(&[S]) -> Result<Scattering>,
) -> Result<Scattering>
where
    S: Clone,
{
    if period.is_empty() || period.end > sections.len() || count == 0 {
        return Err(Error::invalid(
            "eme grating",
            format!(
                "needs a period among the {} sections and a count of at least 1, got sections \
                 {}..{} and {count}",
                sections.len(),
                period.start,
                period.end
            ),
        ));
    }
    let start = reference(&sections[period.start]);
    // the last period and what follows it
    let tail = solve(&sections[period.start..])?;
    let mut total = blocks(&tail);
    let mut first = tail.first;
    if count > 1 {
        let mut one = sections[period.clone()].to_vec();
        one.push(start.clone());
        let p = solve(&one)?;
        total = star(&power(&blocks(&p), count - 1), &total);
        first = p.first;
    }
    if period.start > 0 {
        let mut before = sections[..period.start].to_vec();
        before.push(start);
        let b = solve(&before)?;
        total = star(&blocks(&b), &total);
        first = b.first;
    }
    Ok(Scattering {
        s11: rows(&total.s11),
        s12: rows(&total.s12),
        s21: rows(&total.s21),
        s22: rows(&total.s22),
        first,
        last: tail.last,
    })
}

/// The period's length, µm, which must be above 0.
fn period_length(lengths: impl Iterator<Item = f64>) -> Result<f64> {
    let l: f64 = lengths.sum();
    if !(l.is_finite() && l > 0.0) {
        return Err(Error::invalid(
            "eme bloch",
            format!("a period's length must be above 0, got {l} um"),
        ));
    }
    Ok(l)
}

impl Eme {
    /// The Bloch modes of a period made of `period`'s sections in order along z, between the
    /// first section's modes at its start and at the next period's: [`Bloch`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no sections or a period of no length, and
    /// [`Eme::s_matrix`]'s.
    pub fn bloch(&self, period: &[Section]) -> Result<Bloch> {
        let Some(first) = period.first() else {
            return Err(Error::invalid("eme bloch", "needs at least one section"));
        };
        let l = period_length(period.iter().map(|s| s.length.to_um()))?;
        let mut one = period.to_vec();
        one.push(Section::new(first.slice.clone(), Length::ZERO));
        Bloch::of(&self.s_matrix(&one)?, l, self.wavelength.wavenumber())
    }

    /// The S-matrix of `sections` (as [`Eme::s_matrix`]) with `sections[period]` repeated
    /// `count` times in place: the period's S-matrix multiplied by itself by repeated squaring
    /// (about 2 log₂ count products), the same as the sections in sequence to round-off.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a period that is empty or not among the sections, a count of
    /// 0, and [`Eme::s_matrix`]'s.
    pub fn grating(
        &self,
        sections: &[Section],
        period: Range<usize>,
        count: usize,
    ) -> Result<Scattering> {
        grating(
            sections,
            period,
            count,
            |s| Section::new(s.slice.clone(), Length::ZERO),
            |s| self.s_matrix(s),
        )
    }
}

impl Eme3d {
    /// The Bloch modes of a period of 3D sections, as [`Eme::bloch`].
    ///
    /// # Errors
    ///
    /// As [`Eme::bloch`], with [`Eme3d::s_matrix`]'s.
    pub fn bloch(&self, period: &[Section3d]) -> Result<Bloch> {
        let Some(first) = period.first() else {
            return Err(Error::invalid("eme bloch", "needs at least one section"));
        };
        let l = period_length(period.iter().map(|s| s.length.to_um()))?;
        let mut one = period.to_vec();
        one.push(Section3d::new(first.plane.clone(), Length::ZERO));
        Bloch::of(&self.s_matrix(&one)?, l, self.wavelength().wavenumber())
    }

    /// The S-matrix of 3D `sections` with `sections[period]` repeated `count` times, as
    /// [`Eme::grating`].
    ///
    /// # Errors
    ///
    /// As [`Eme::grating`], with [`Eme3d::s_matrix`]'s.
    pub fn grating(
        &self,
        sections: &[Section3d],
        period: Range<usize>,
        count: usize,
    ) -> Result<Scattering> {
        grating(
            sections,
            period,
            count,
            |s| Section3d::new(s.plane.clone(), Length::ZERO),
            |s| self.s_matrix(s),
        )
    }
}

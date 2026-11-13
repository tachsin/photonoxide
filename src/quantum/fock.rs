//! Fock states of photons in modes, the probability of each output pattern, and the
//! distribution over all of them.

use std::collections::BTreeMap;

use num_complex::Complex64 as c64;

use super::Matrix;
use super::permanent::permanent;
use crate::portable;
use crate::{Error, Result};

/// A Fock state: how many photons are in each mode.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fock(Vec<usize>);

impl Fock {
    /// The state with `occupations[k]` photons in mode k.
    pub fn new(occupations: Vec<usize>) -> Fock {
        Fock(occupations)
    }

    /// One photon in each of the modes `occupied` (each listed once) out of `modes`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode out of range or listed twice.
    pub fn single_photons(modes: usize, occupied: &[usize]) -> Result<Fock> {
        let mut m = vec![0; modes];
        for &k in occupied {
            if k >= modes || m[k] > 0 {
                return Err(Error::invalid(
                    "Fock state",
                    format!("mode {k} is out of range or listed twice ({modes} modes)"),
                ));
            }
            m[k] = 1;
        }
        Ok(Fock(m))
    }

    /// The occupations, mode by mode.
    pub fn occupations(&self) -> &[usize] {
        &self.0
    }

    /// The number of modes.
    pub fn modes(&self) -> usize {
        self.0.len()
    }

    /// The number of photons.
    pub fn photons(&self) -> usize {
        self.0.iter().sum()
    }

    /// Each photon's mode, in increasing order: mode k listed `occupations[k]` times.
    pub fn mode_list(&self) -> Vec<usize> {
        self.0
            .iter()
            .enumerate()
            .flat_map(|(k, &m)| std::iter::repeat_n(k, m))
            .collect()
    }

    /// ∏_k m_k!.
    pub fn factorials(&self) -> f64 {
        self.0.iter().map(|&m| factorial(m)).product()
    }

    /// The first `modes` modes' occupations: the detected part of a pattern whose last modes are
    /// loss.
    pub fn truncated(&self, modes: usize) -> Fock {
        Fock(self.0[..modes.min(self.0.len())].to_vec())
    }
}

fn factorial(m: usize) -> f64 {
    (1..=m).map(|k| k as f64).product()
}

/// Every pattern of `photons` photons in `modes` modes, the C(modes + photons − 1, photons) of
/// them, in reverse lexicographic order (all photons in mode 0 first).
pub fn patterns(modes: usize, photons: usize) -> Vec<Fock> {
    let mut out = Vec::new();
    let mut current = vec![0; modes];
    fill(&mut current, 0, photons, &mut out);
    out
}

fn fill(current: &mut Vec<usize>, mode: usize, left: usize, out: &mut Vec<Fock>) {
    if mode + 1 >= current.len() {
        if let Some(last) = current.len().checked_sub(1) {
            current[last] = left;
            out.push(Fock(current.clone()));
            current[last] = 0;
        } else if left == 0 {
            out.push(Fock(Vec::new()));
        }
        return;
    }
    for m in (0..=left).rev() {
        current[mode] = m;
        fill(current, mode + 1, left - m, out);
    }
    current[mode] = 0;
}

/// How alike the photons are, inside their spatial modes (their spectra, polarization, time of
/// arrival).
#[derive(Clone, Debug, PartialEq)]
pub enum Photons {
    /// All alike: probabilities are |Per|².
    Indistinguishable,
    /// Each told apart from the others: probabilities are permanents of |T|², those of classical
    /// particles.
    Distinguishable,
    /// The overlaps ⟨ψ_i|ψ_j⟩ of the photons' internal states, an n × n Gram matrix with ones on
    /// its diagonal, the photons in the order of the input's [`Fock::mode_list`].
    Overlaps(Matrix),
}

/// The probability that photons in `input` (over the transfer's columns) leave in `output`
/// (over its rows), through a transfer matrix T whose rows are output modes and columns input
/// modes: a circuit's S, or a lossy circuit's isometry with its loss modes as extra rows
/// ([`super::Transfer::isometry`]).
///
/// - indistinguishable: |Per(M)|² / (∏ m_in! ∏ m_out!), with M = T restricted to the output
///   photons' rows and the input photons' columns (Aaronson & Arkhipov 2013,
///   doi:10.4086/toc.2013.v009a004);
/// - distinguishable: Per(|M|²) / ∏ m_out!;
/// - partly distinguishable, with Gram matrix G: (1/∏ m_out!) Σ_σ (∏_j G_σ(j),j)
///   Per(M ∘ M̄_σ) over permutations σ of the photons, M̄_σ the conjugate of M with its columns
///   permuted by σ, divided by the input state's norm Σ_σ ∏_j G_σ(j),j over the σ that keep
///   every photon in its mode. This is the expansion of the photons' internal states, written
///   out in docs/methods/quantum.md; G of all ones and the identity give the two cases above.
///
/// # Errors
///
/// [`Error::InvalidValue`] when the patterns don't fit the matrix, hold different numbers of
/// photons, or a Gram matrix isn't n × n; and a permanent's own errors.
pub fn probability(t: &Matrix, input: &Fock, output: &Fock, photons: &Photons) -> Result<f64> {
    if input.modes() != t.cols() || output.modes() != t.rows() {
        return Err(Error::invalid(
            "photon pattern",
            format!(
                "an input over {} modes and an output over {} don't fit a {} × {} transfer \
                 matrix",
                input.modes(),
                output.modes(),
                t.rows(),
                t.cols()
            ),
        ));
    }
    let n = input.photons();
    if output.photons() != n {
        return Err(Error::invalid(
            "photon pattern",
            format!(
                "{n} photons in, {} out: a linear circuit keeps photons (lost ones leave by \
                 loss modes)",
                output.photons()
            ),
        ));
    }
    let m = t.select(&output.mode_list(), &input.mode_list());
    match photons {
        Photons::Indistinguishable => {
            Ok(permanent(&m)?.norm_sqr() / (input.factorials() * output.factorials()))
        }
        Photons::Distinguishable => Ok(permanent(&m.moduli_squared())?.re / output.factorials()),
        Photons::Overlaps(g) => partly(&m, g, input, output),
    }
}

/// The partly distinguishable probability, by the sum over permutations.
fn partly(m: &Matrix, g: &Matrix, input: &Fock, output: &Fock) -> Result<f64> {
    let n = m.cols();
    if g.rows() != n || g.cols() != n {
        return Err(Error::invalid(
            "photon overlaps",
            format!(
                "need a {n} × {n} Gram matrix, got {} × {}",
                g.rows(),
                g.cols()
            ),
        ));
    }
    if n > 8 {
        return Err(Error::invalid(
            "photon overlaps",
            format!("the sum over permutations takes up to 8 photons, got {n}"),
        ));
    }
    let modes = input.mode_list();
    let mut sum = c64::new(0.0, 0.0);
    let mut norm = c64::new(0.0, 0.0);
    for sigma in permutations(n) {
        let weight: c64 = (0..n).map(|j| g[(sigma[j], j)]).product();
        if portable::complex::abs(weight) == 0.0 {
            continue;
        }
        let twisted = Matrix::from_fn(n, n, |k, j| m[(k, j)] * m[(k, sigma[j])].conj());
        sum += weight * permanent(&twisted)?;
        if (0..n).all(|j| modes[sigma[j]] == modes[j]) {
            norm += weight;
        }
    }
    Ok(sum.re / (norm.re * output.factorials()))
}

/// Every permutation of 0..n, in Heap's order.
pub(super) fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut perm: Vec<usize> = (0..n).collect();
    let mut out = vec![perm.clone()];
    let mut c = vec![0usize; n];
    let mut i = 0;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                perm.swap(0, i);
            } else {
                perm.swap(c[i], i);
            }
            out.push(perm.clone());
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    out
}

/// A distribution over photon patterns: exact ([`Distribution::exact`]) or from samples
/// ([`Distribution::from_samples`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Distribution {
    patterns: Vec<Fock>,
    probabilities: Vec<f64>,
}

impl Distribution {
    /// A distribution of `patterns` with `probabilities`, one each, in that order.
    pub(super) fn from_parts(patterns: Vec<Fock>, probabilities: Vec<f64>) -> Distribution {
        Distribution {
            patterns,
            probabilities,
        }
    }

    /// Every output pattern's probability for photons in `input` through T (as
    /// [`probability`]), each pattern on rayon's threads, collected in order.
    ///
    /// # Errors
    ///
    /// As [`probability`].
    pub fn exact(t: &Matrix, input: &Fock, photons: &Photons) -> Result<Distribution> {
        let patterns = patterns(t.rows(), input.photons());
        let probabilities =
            crate::parallel::map_in_order(&patterns, |out| probability(t, input, out, photons))
                .into_iter()
                .collect::<Result<Vec<f64>>>()?;
        Ok(Distribution {
            patterns,
            probabilities,
        })
    }

    /// The empirical distribution of samples: each distinct pattern with its share, in sorted
    /// order.
    pub fn from_samples(samples: &[Fock]) -> Distribution {
        let mut sorted = samples.to_vec();
        sorted.sort();
        let mut patterns: Vec<Fock> = Vec::new();
        let mut counts: Vec<f64> = Vec::new();
        for s in sorted {
            if patterns.last() == Some(&s) {
                if let Some(c) = counts.last_mut() {
                    *c += 1.0;
                }
            } else {
                patterns.push(s);
                counts.push(1.0);
            }
        }
        let total = samples.len().max(1) as f64;
        Distribution {
            patterns,
            probabilities: counts.into_iter().map(|c| c / total).collect(),
        }
    }

    /// The patterns, in the order of [`Distribution::probabilities`].
    pub fn patterns(&self) -> &[Fock] {
        &self.patterns
    }

    /// Each pattern's probability.
    pub fn probabilities(&self) -> &[f64] {
        &self.probabilities
    }

    /// A pattern's probability, 0 for a pattern it doesn't hold.
    pub fn probability_of(&self, pattern: &Fock) -> f64 {
        self.patterns
            .iter()
            .zip(&self.probabilities)
            .filter(|(p, _)| *p == pattern)
            .map(|(_, &q)| q)
            .sum()
    }

    /// The sum of the probabilities: 1 for a unitary, or an isometry with its loss modes.
    pub fn total(&self) -> f64 {
        self.probabilities.iter().sum()
    }

    /// The distribution of the first `modes` modes alone, the others (a lossy circuit's loss
    /// modes) summed out: patterns with fewer photons where some were lost.
    pub fn detected(&self, modes: usize) -> Distribution {
        let mut pairs: Vec<(Fock, f64)> = self
            .patterns
            .iter()
            .zip(&self.probabilities)
            .map(|(p, &q)| (p.truncated(modes), q))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut patterns: Vec<Fock> = Vec::new();
        let mut probabilities: Vec<f64> = Vec::new();
        for (p, q) in pairs {
            if patterns.last() == Some(&p) {
                if let Some(last) = probabilities.last_mut() {
                    *last += q;
                }
            } else {
                patterns.push(p);
                probabilities.push(q);
            }
        }
        Distribution {
            patterns,
            probabilities,
        }
    }

    /// The probability of each photon number in `mode`: entry k is P(k photons there).
    pub fn marginal(&self, mode: usize) -> Vec<f64> {
        let most = self
            .patterns
            .iter()
            .map(|p| p.occupations().get(mode).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let mut out = vec![0.0; most + 1];
        for (p, &q) in self.patterns.iter().zip(&self.probabilities) {
            out[p.occupations().get(mode).copied().unwrap_or(0)] += q;
        }
        out
    }

    /// The mean number of photons in `mode`.
    pub fn mean(&self, mode: usize) -> f64 {
        self.marginal(mode)
            .iter()
            .enumerate()
            .map(|(k, &q)| k as f64 * q)
            .sum()
    }

    /// The probability that some mode holds two or more photons: bunching.
    pub fn bunching(&self) -> f64 {
        self.patterns
            .iter()
            .zip(&self.probabilities)
            .filter(|(p, _)| p.occupations().iter().any(|&m| m >= 2))
            .map(|(_, &q)| q)
            .sum()
    }

    /// The total variation distance to another distribution, ½ Σ |p − q| over the patterns of
    /// either.
    pub fn total_variation(&self, other: &Distribution) -> f64 {
        // each side's patterns by name, so that the distance takes n log n, not n²
        fn by_pattern(d: &Distribution) -> BTreeMap<&Fock, f64> {
            let mut m: BTreeMap<&Fock, f64> = BTreeMap::new();
            for (p, &q) in d.patterns.iter().zip(&d.probabilities) {
                *m.entry(p).or_default() += q;
            }
            m
        }
        let (mine, theirs) = (by_pattern(self), by_pattern(other));
        let mut sum = 0.0;
        for (p, &q) in &mine {
            sum += (q - theirs.get(p).copied().unwrap_or(0.0)).abs();
        }
        for (p, &q) in &theirs {
            if !mine.contains_key(p) {
                sum += q;
            }
        }
        0.5 * sum
    }

    /// The empirical distribution of counted samples: each pattern with its count's share of
    /// the counts' total, in sorted order, a pattern counted twice summed and one counted 0
    /// times left out.
    pub fn from_counts(counts: &[(Fock, usize)]) -> Distribution {
        let mut summed: BTreeMap<&Fock, usize> = BTreeMap::new();
        for (f, c) in counts {
            *summed.entry(f).or_default() += c;
        }
        let total = summed.values().sum::<usize>().max(1) as f64;
        let (patterns, probabilities) = summed
            .into_iter()
            .filter(|(_, c)| *c > 0)
            .map(|(f, c)| (f.clone(), c as f64 / total))
            .unzip();
        Distribution {
            patterns,
            probabilities,
        }
    }
}

//! The permanent of a square matrix, Per(A) = Σ_σ ∏_i a_{i,σ(i)}: by Ryser's formula or
//! Glynn's, each in Gray-code order (about n 2ⁿ operations), or by the definition (n · n!).
//!
//! - **Ryser** (read in P. H. Lundow, K. Markström, J. Comput. Phys. 455, 110990 (2022),
//!   doi:10.1016/j.jcp.2022.110990, Eq. 3; H. J. Ryser's book, doi:10.5948/UPO9781614440147,
//!   is out of print):
//!   Per(A) = (−1)ⁿ Σ_{S ⊆ [n]} (−1)^|S| ∏_i Σ_{j ∈ S} a_ij.
//! - **Glynn** (D. G. Glynn, Eur. J. Combin. 31, 1887 (2010), doi:10.1016/j.ejc.2010.01.010,
//!   Theorem 2.1): Per(A) = 2^(1−n) Σ_δ (∏_k δ_k) ∏_j Σ_i δ_i a_ij over δ ∈ {±1}ⁿ with δ₁ = 1.
//!
//! In Gray-code order consecutive terms differ by one column (Ryser) or one sign (Glynn), so
//! each term costs n operations. The sum is cut into parts fixed by n alone, 256 of them from
//! n = 12 (each part starts from its own sums, computed directly), each part summed in order
//! with Kahan's compensation (as Lundow and Markström recommend), and the parts added in order:
//! the threads only decide which part runs where, so the result is the same bit for bit on any
//! number of threads.

use num_complex::Complex64 as c64;

use super::Matrix;
use crate::{Error, Result};

/// Which formula computes a permanent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formula {
    /// Ryser's inclusion–exclusion, in Gray-code order.
    Ryser,
    /// Glynn's polarization formula, in Gray-code order: half Ryser's terms.
    Glynn,
    /// The defining sum over permutations: up to n = 10, for checks.
    Definition,
}

/// The largest n the Gray-code formulas take: 2³⁹ terms is hours on a workstation.
const LARGEST: usize = 40;
/// The largest n the definition takes: 10! = 3.6 million products.
const LARGEST_DEFINITION: usize = 10;
/// From this many terms' bits the sum is cut into [`PARTS`] parts, run on rayon's threads.
const THREADED_BITS: u32 = 12;
const PARTS: u64 = 256;

/// Per(A), by Glynn's formula.
///
/// ```
/// use photonoxide::Complex64 as c64;
/// use photonoxide::quantum::{Matrix, permanent};
/// // the permanent of the n × n all-ones matrix is n!
/// let ones = Matrix::from_fn(5, 5, |_, _| c64::new(1.0, 0.0));
/// assert!((permanent(&ones)? - c64::new(120.0, 0.0)).norm() < 1e-12);
/// # Ok::<(), photonoxide::Error>(())
/// ```
///
/// # Errors
///
/// As [`permanent_by`].
pub fn permanent(a: &Matrix) -> Result<c64> {
    permanent_by(a, Formula::Glynn)
}

/// Per(A) by the given formula. The 0 × 0 matrix's permanent is 1.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless A is square, and for n above 40 (10 for
/// [`Formula::Definition`]).
pub fn permanent_by(a: &Matrix, formula: Formula) -> Result<c64> {
    if !a.is_square() {
        return Err(Error::invalid(
            "permanent",
            format!("needs a square matrix, got {} × {}", a.rows(), a.cols()),
        ));
    }
    let n = a.rows();
    let largest = if formula == Formula::Definition {
        LARGEST_DEFINITION
    } else {
        LARGEST
    };
    if n > largest {
        return Err(Error::invalid(
            "permanent",
            format!("{formula:?} takes n up to {largest}, got {n}"),
        ));
    }
    match n {
        0 => return Ok(c64::new(1.0, 0.0)),
        1 => return Ok(a[(0, 0)]),
        _ => {}
    }
    Ok(match formula {
        Formula::Ryser => ryser(a),
        Formula::Glynn => glynn(a),
        Formula::Definition => definition(a),
    })
}

/// A complex sum with Kahan's compensation on each part.
#[derive(Clone, Copy, Default)]
pub(crate) struct Kahan {
    pub(crate) sum: c64,
    carry: c64,
}

impl Kahan {
    pub(crate) fn add(&mut self, x: c64) {
        let y = x - self.carry;
        let t = self.sum + y;
        self.carry = (t - self.sum) - y;
        self.sum = t;
    }
}

/// The Gray code of g.
fn gray(g: u64) -> u64 {
    g ^ (g >> 1)
}

/// Σ over g in 0..2^bits of `part(start, end)`, cut into fixed parts and added in order.
pub(crate) fn parts(bits: u32, part: impl Fn(u64, u64) -> c64 + Sync + Send) -> c64 {
    let total = 1u64 << bits;
    let count = if bits >= THREADED_BITS { PARTS } else { 1 };
    let size = total / count;
    let starts: Vec<u64> = (0..count).map(|k| k * size).collect();
    let sums = crate::parallel::map_in_order(&starts, |&s| part(s, s + size));
    let mut sum = Kahan::default();
    for s in sums {
        sum.add(s);
    }
    sum.sum
}

fn ryser(a: &Matrix) -> c64 {
    let n = a.rows();
    let bits = u32::try_from(n).unwrap_or(u32::MAX);
    let total = parts(bits, |start, end| {
        // the row sums of the columns in gray(start), computed directly
        let set = gray(start);
        let mut rows: Vec<c64> = (0..n)
            .map(|i| {
                (0..n)
                    .filter(|&j| set >> j & 1 == 1)
                    .map(|j| a[(i, j)])
                    .sum()
            })
            .collect();
        let mut size = set.count_ones();
        let mut sum = Kahan::default();
        let term = |rows: &[c64], size: u32| {
            let p: c64 = rows.iter().product();
            if size % 2 == 1 { -p } else { p }
        };
        sum.add(term(&rows, size));
        for g in start + 1..end {
            let j = g.trailing_zeros() as usize;
            let adding = gray(g) >> j & 1 == 1;
            for (i, r) in rows.iter_mut().enumerate() {
                if adding {
                    *r += a[(i, j)];
                } else {
                    *r -= a[(i, j)];
                }
            }
            if adding {
                size += 1;
            } else {
                size -= 1;
            }
            sum.add(term(&rows, size));
        }
        sum.sum
    });
    if n % 2 == 1 { -total } else { total }
}

fn glynn(a: &Matrix) -> c64 {
    let n = a.rows();
    let bits = u32::try_from(n - 1).unwrap_or(u32::MAX);
    let total = parts(bits, |start, end| {
        // row i + 1 has δ = −1 where bit i of the Gray code is set; δ₁ = +1 always
        let signs = gray(start);
        let delta = |i: usize| -> f64 {
            if i > 0 && signs >> (i - 1) & 1 == 1 {
                -1.0
            } else {
                1.0
            }
        };
        let mut cols: Vec<c64> = (0..n)
            .map(|j| (0..n).map(|i| a[(i, j)] * delta(i)).sum())
            .collect();
        let mut negative = signs.count_ones();
        let mut sum = Kahan::default();
        let term = |cols: &[c64], negative: u32| {
            let p: c64 = cols.iter().product();
            if negative % 2 == 1 { -p } else { p }
        };
        sum.add(term(&cols, negative));
        for g in start + 1..end {
            let bit = g.trailing_zeros() as usize;
            let row = bit + 1;
            let now_negative = gray(g) >> bit & 1 == 1;
            // δ flips from +1 to −1 (subtract twice the row) or back
            let factor = if now_negative { -2.0 } else { 2.0 };
            for (j, c) in cols.iter_mut().enumerate() {
                *c += a[(row, j)] * factor;
            }
            if now_negative {
                negative += 1;
            } else {
                negative -= 1;
            }
            sum.add(term(&cols, negative));
        }
        sum.sum
    });
    total / 2f64.powi(i32::try_from(n - 1).unwrap_or(i32::MAX))
}

/// The defining sum, over permutations in Heap's order.
fn definition(a: &Matrix) -> c64 {
    let n = a.rows();
    let mut perm: Vec<usize> = (0..n).collect();
    let mut c = vec![0usize; n];
    let product = |perm: &[usize]| -> c64 { (0..n).map(|i| a[(i, perm[i])]).product() };
    let mut sum = Kahan::default();
    sum.add(product(&perm));
    let mut i = 0;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                perm.swap(0, i);
            } else {
                perm.swap(c[i], i);
            }
            sum.add(product(&perm));
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    sum.sum
}

/// The permanents of every minor of a (k − 1) × k matrix B: Per(B with column ℓ removed) for
/// ℓ = 0..k, all from one pass of Glynn's formula over B's rows, the products of the other
/// columns' sums by prefix and suffix products (P. Clifford, R. Clifford, SODA 2018,
/// doi:10.1137/1.9781611975031.10, Lemma 3.2): O(k 2^k) for all k of them.
pub(crate) fn minors(b: &Matrix) -> Vec<c64> {
    let rows = b.rows();
    let k = b.cols();
    debug_assert_eq!(rows + 1, k);
    if rows == 0 {
        return vec![c64::new(1.0, 0.0); k];
    }
    let mut sums = vec![Kahan::default(); k];
    let signs_count = 1u64 << (rows - 1);
    let mut cols: Vec<c64> = (0..k).map(|j| (0..rows).map(|i| b[(i, j)]).sum()).collect();
    let mut negative = 0u32;
    let mut prefix = vec![c64::new(1.0, 0.0); k + 1];
    let mut suffix = vec![c64::new(1.0, 0.0); k + 1];
    for g in 0..signs_count {
        if g > 0 {
            let bit = g.trailing_zeros() as usize;
            let row = bit + 1;
            let now_negative = gray(g) >> bit & 1 == 1;
            let factor = if now_negative { -2.0 } else { 2.0 };
            for (j, c) in cols.iter_mut().enumerate() {
                *c += b[(row, j)] * factor;
            }
            if now_negative {
                negative += 1;
            } else {
                negative -= 1;
            }
        }
        for j in 0..k {
            prefix[j + 1] = prefix[j] * cols[j];
        }
        for j in (0..k).rev() {
            suffix[j] = suffix[j + 1] * cols[j];
        }
        let sign = if negative % 2 == 1 { -1.0 } else { 1.0 };
        for (l, s) in sums.iter_mut().enumerate() {
            s.add(prefix[l] * suffix[l + 1] * sign);
        }
    }
    let scale = 2f64.powi(i32::try_from(rows - 1).unwrap_or(i32::MAX));
    sums.into_iter().map(|s| s.sum / scale).collect()
}

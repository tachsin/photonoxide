//! Hafnians, loop hafnians and torontonians: what Gaussian states' photon counts are made of.
//!
//! - **The hafnian** of a symmetric 2m × 2m matrix is the sum over its perfect matchings,
//!   haf(A) = Σ_M ∏_{(i,j) ∈ M} a_ij; the **loop hafnian** lets a vertex pair with itself, a
//!   loop weighing a_ii. Both by A. Björklund, B. Gupt, N. Quesada, ACM J. Exp. Algorithmics 24,
//!   1.11 (2019), doi:10.1145/3325111, Eqs. (24)–(26): an inclusion–exclusion over the 2^m
//!   subsets Z of a fixed perfect matching (here the pairs (2i, 2i + 1)), each term the
//!   coefficient of λ^m in exp(Σ_k [tr(B^k)/(2k) + vᵀX B^(k−1) v/2] λ^k), B = A_Z X the
//!   rows and columns of Z's pairs with each pair's columns swapped, v the diagonal of A_Z (zero
//!   for the hafnian). The power traces come from det(I − λB), the characteristic polynomial of
//!   B's Hessenberg form ([`super::matrix::det_one_minus`]) instead of the paper's Schur
//!   decomposition: additions and multiplications only, the same bits on every system.
//! - **The torontonian** of a 2m × 2m matrix O, its modes i and i + m paired (N. Quesada,
//!   J. M. Arrazola, N. Killoran, Phys. Rev. A 98, 062322 (2018), doi:10.1103/PhysRevA.98.062322,
//!   Eq. (9)): Tor(O) = Σ_{Z ⊆ [m]} (−1)^(m−|Z|) / √det(I − O_Z).
//!
//! The 2^m terms are added as the permanents' are ([`super::permanent`]): in parts fixed by m,
//! each Kahan-summed, the parts in order, so the result is the same bit for bit on any number of
//! threads. A hafnian of a 2m × 2m matrix costs about 2^m m³.

use num_complex::Complex64 as c64;

use super::Matrix;
use super::matrix::{det_one_minus, determinant};
use super::permanent::parts;
use crate::{Error, Result};

/// The largest matrix a hafnian or torontonian takes: 2^30 terms is hours on a workstation.
const LARGEST: usize = 60;

fn check(a: &Matrix, what: &'static str, symmetric: bool) -> Result<usize> {
    if !a.is_square() {
        return Err(Error::invalid(
            what,
            format!("needs a square matrix, got {} × {}", a.rows(), a.cols()),
        ));
    }
    let n = a.rows();
    if n > LARGEST {
        return Err(Error::invalid(
            what,
            format!("takes matrices up to {LARGEST} × {LARGEST}, got {n} × {n}"),
        ));
    }
    if symmetric {
        let scale = (0..n)
            .flat_map(|i| (0..n).map(move |j| (i, j)))
            .map(|(i, j)| a[(i, j)].norm())
            .fold(0.0, f64::max);
        for i in 0..n {
            for j in 0..i {
                if (a[(i, j)] - a[(j, i)]).norm() > 1e-12 * scale {
                    return Err(Error::invalid(
                        what,
                        format!("needs a symmetric matrix: a[{i}, {j}] ≠ a[{j}, {i}]"),
                    ));
                }
            }
        }
    }
    Ok(n)
}

/// The hafnian of a symmetric matrix: the sum over its perfect matchings of the products of the
/// matched entries. 0 for an odd size, 1 for the 0 × 0 matrix.
///
/// ```
/// use photonoxide::Complex64 as c64;
/// use photonoxide::quantum::{Matrix, hafnian};
/// // the 6 × 6 all-ones matrix has 5!! = 15 perfect matchings
/// let ones = Matrix::from_fn(6, 6, |_, _| c64::new(1.0, 0.0));
/// assert!((hafnian(&ones)? - c64::new(15.0, 0.0)).norm() < 1e-12);
/// # Ok::<(), photonoxide::Error>(())
/// ```
///
/// # Errors
///
/// [`Error::InvalidValue`] unless A is square and symmetric (to 1e-12 of its largest entry), and
/// above 60 × 60.
pub fn hafnian(a: &Matrix) -> Result<c64> {
    let n = check(a, "hafnian", true)?;
    if n % 2 == 1 {
        return Ok(c64::new(0.0, 0.0));
    }
    Ok(by_traces(a, false))
}

/// The loop hafnian of a symmetric matrix: the sum over its perfect matchings with loops, a
/// vertex matched to itself weighing its diagonal entry. 1 for the 0 × 0 matrix; an odd size
/// is padded with a vertex whose only edge is a loop of weight 1.
///
/// # Errors
///
/// As [`hafnian`].
pub fn loop_hafnian(a: &Matrix) -> Result<c64> {
    let n = check(a, "loop hafnian", true)?;
    if n % 2 == 1 {
        let padded = Matrix::from_fn(n + 1, n + 1, |i, j| {
            if i < n && j < n {
                a[(i, j)]
            } else if i == n && j == n {
                c64::new(1.0, 0.0)
            } else {
                c64::new(0.0, 0.0)
            }
        });
        return Ok(by_traces(&padded, true));
    }
    Ok(by_traces(a, true))
}

/// The torontonian of a 2m × 2m matrix, mode i's rows and columns i and i + m:
/// Σ_{Z ⊆ [m]} (−1)^(m−|Z|) / √det(I − O_Z), the principal square root. 1 for the 0 × 0 matrix.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless O is square of even size, and above 60 × 60.
pub fn torontonian(o: &Matrix) -> Result<c64> {
    let n = check(o, "torontonian", false)?;
    if n % 2 == 1 {
        return Err(Error::invalid(
            "torontonian",
            format!("needs an even size, two rows per mode, got {n}"),
        ));
    }
    let m = n / 2;
    if m == 0 {
        return Ok(c64::new(1.0, 0.0));
    }
    let bits = u32::try_from(m).unwrap_or(u32::MAX);
    Ok(parts(bits, |start, end| {
        let mut sum = super::permanent::Kahan::default();
        for z in start..end {
            let modes: Vec<usize> = (0..m).filter(|&i| z >> i & 1 == 1).collect();
            let k = modes.len();
            let index = |r: usize| {
                if r < k { modes[r] } else { modes[r - k] + m }
            };
            let block = Matrix::from_fn(2 * k, 2 * k, |r, c| {
                let one = if r == c { 1.0 } else { 0.0 };
                c64::new(one, 0.0) - o[(index(r), index(c))]
            });
            let term = determinant(&block).sqrt().inv();
            sum.add(if (m - k) % 2 == 1 { -term } else { term });
        }
        sum.sum
    }))
}

/// Björklund et al.'s Eq. (24) (`loops` false) or (25).
fn by_traces(a: &Matrix, loops: bool) -> c64 {
    let n = a.rows();
    let m = n / 2;
    if m == 0 {
        return c64::new(1.0, 0.0);
    }
    let bits = u32::try_from(m).unwrap_or(u32::MAX);
    parts(bits, |start, end| {
        let mut sum = super::permanent::Kahan::default();
        for z in start..end {
            let pairs: Vec<usize> = (0..m).filter(|&i| z >> i & 1 == 1).collect();
            let k = 2 * pairs.len();
            let index = |r: usize| 2 * pairs[r / 2] + r % 2;
            // B = A_Z X: column c of B is column c ^ 1 of A_Z; the hafnian sees no loops, so
            // its diagonal is left out
            let b = Matrix::from_fn(k, k, |r, c| {
                if !loops && r == c ^ 1 {
                    c64::new(0.0, 0.0)
                } else {
                    a[(index(r), index(c ^ 1))]
                }
            });
            let term = coefficient(
                &b,
                m,
                loops.then(|| (0..k).map(|r| a[(index(r), index(r))])),
            );
            sum.add(if (m - pairs.len()) % 2 == 1 {
                -term
            } else {
                term
            });
        }
        sum.sum
    })
}

/// The coefficient of λ^m in exp(Σ_{k=1}^{m} t_k λ^k), t_k = tr(B^k)/(2k) + vᵀX B^(k−1) v / 2.
fn coefficient(b: &Matrix, m: usize, diagonal: Option<impl Iterator<Item = c64>>) -> c64 {
    let size = b.rows();
    let mut t = vec![c64::new(0.0, 0.0); m + 1];
    if size > 0 {
        // log det(I − λB) = −Σ tr(B^k) λ^k / k, from det(I − λB)'s coefficients h_j
        let h = det_one_minus(b);
        let coeff = |j: usize| h.get(j).copied().unwrap_or(c64::new(0.0, 0.0));
        let mut log = vec![c64::new(0.0, 0.0); m + 1];
        for j in 1..=m {
            let mut s = coeff(j);
            for i in 1..j {
                s -= log[i] * coeff(j - i) * (i as f64 / j as f64);
            }
            log[j] = s;
        }
        for k in 1..=m {
            // tr(B^k)/(2k) = −log_k/2
            t[k] = -0.5 * log[k];
        }
        if let Some(diagonal) = diagonal {
            let v: Vec<c64> = diagonal.collect();
            let swapped: Vec<c64> = (0..size).map(|r| v[r ^ 1]).collect();
            let mut w = v.clone();
            for tk in t.iter_mut().skip(1) {
                // vᵀ X B^(k−1) v = (Xv)ᵀ (B^(k−1) v)
                let dot: c64 = swapped.iter().zip(&w).map(|(x, y)| x * y).sum();
                *tk += 0.5 * dot;
                w = (0..size)
                    .map(|r| (0..size).map(|c| b[(r, c)] * w[c]).sum())
                    .collect();
            }
        }
    }
    // E = exp(Σ t_k λ^k): E_0 = 1, j E_j = Σ_{i=1}^{j} i t_i E_{j−i}
    let mut e = vec![c64::new(0.0, 0.0); m + 1];
    e[0] = c64::new(1.0, 0.0);
    for j in 1..=m {
        let mut s = c64::new(0.0, 0.0);
        for i in 1..=j {
            s += t[i] * e[j - i] * i as f64;
        }
        e[j] = s / j as f64;
    }
    e[m]
}

/// The hafnian (or, with `diagonal`, the loop hafnian with those loops) of the matrix in which
/// row and column i of the symmetric `a` appear `repeats[i]` times, by the moments of a sum
/// (R. Kan, J. Multivariate Anal. 99, 542 (2008), doi:10.1016/j.jmva.2007.01.013): with
/// N = Σ r_i and h_i = r_i/2 − v_i,
///
/// Σ_{v_i = 0}^{r_i} (−1)^(Σv) ∏ C(r_i, v_i) Σ_t (h·γ)^(N−2t) (hᵀ a h / 2)^t / ((N − 2t)! t!),
///
/// only t = N/2 without loops: the polarization of ∏ x_i^(r_i), whose Gaussian moments are the
/// matchings. ∏(r_i + 1) terms, few when the photons crowd into few modes, where Björklund et
/// al.'s sum over subsets would take 2^(N/2). The alternating sum loses digits as N grows: about
/// 1e-10 relative at 30 photons in one mode, and nothing useful beyond about 40.
pub(crate) fn repeated(a: &Matrix, diagonal: Option<&[c64]>, repeats: &[usize]) -> c64 {
    let d = repeats.len();
    let total: usize = repeats.iter().sum();
    if diagonal.is_none() && total % 2 == 1 {
        return c64::new(0.0, 0.0);
    }
    let factorial = |k: usize| (1..=k).map(|j| j as f64).product::<f64>();
    let binomial = |n: usize, k: usize| factorial(n) / (factorial(k) * factorial(n - k));
    let mut v = vec![0usize; d];
    let mut sum = super::permanent::Kahan::default();
    loop {
        let h: Vec<f64> = (0..d)
            .map(|i| 0.5 * repeats[i] as f64 - v[i] as f64)
            .collect();
        let q: c64 = (0..d)
            .flat_map(|i| (0..d).map(move |j| (i, j)))
            .map(|(i, j)| a[(i, j)] * h[i] * h[j])
            .sum::<c64>()
            * 0.5;
        let weight: f64 = (0..d).map(|i| binomial(repeats[i], v[i])).product();
        let sign = if v.iter().sum::<usize>() % 2 == 1 {
            -1.0
        } else {
            1.0
        };
        let term = match diagonal {
            None => q.powu((total / 2) as u32) / factorial(total / 2),
            Some(gamma) => {
                let g: c64 = gamma.iter().zip(&h).map(|(x, y)| x * y).sum();
                (0..=total / 2)
                    .map(|t| {
                        g.powu((total - 2 * t) as u32) * q.powu(t as u32)
                            / (factorial(total - 2 * t) * factorial(t))
                    })
                    .sum()
            }
        };
        sum.add(term * (sign * weight));
        // the next v, as a mixed-radix counter
        let mut k = 0;
        while k < d {
            v[k] += 1;
            if v[k] <= repeats[k] {
                break;
            }
            v[k] = 0;
            k += 1;
        }
        if k == d {
            break;
        }
    }
    sum.sum
}

/// The hafnian (or with `loops`, the loop hafnian) by its definition, a recursion over the
/// matchings of the first vertex: for checks, up to 16 vertices.
pub(crate) fn by_definition(a: &Matrix, loops: bool) -> c64 {
    fn go(a: &Matrix, left: &mut Vec<usize>, loops: bool) -> c64 {
        let Some(&first) = left.first() else {
            return c64::new(1.0, 0.0);
        };
        let mut sum = c64::new(0.0, 0.0);
        let rest: Vec<usize> = left[1..].to_vec();
        if loops {
            let mut others = rest.clone();
            sum += a[(first, first)] * go(a, &mut others, loops);
        }
        for (k, &partner) in rest.iter().enumerate() {
            let mut others: Vec<usize> = rest
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != k)
                .map(|(_, &v)| v)
                .collect();
            sum += a[(first, partner)] * go(a, &mut others, loops);
        }
        sum
    }
    let mut all: Vec<usize> = (0..a.rows()).collect();
    go(a, &mut all, loops)
}

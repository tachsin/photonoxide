//! Freund and Malhotra's Example 7.1, rebuilt: block QMR's iterations on a convection–diffusion
//! problem, for the validation report.

use num_complex::Complex64 as c64;

use super::super::{Operator, Sparse, Stopping};
use super::block_qmr_left;

/// R. W. Freund, M. Malhotra, Linear Algebra Appl. 254, 119 (1997),
/// doi:10.1016/S0024-3795(96)00529-0, Example 7.1 as their Eq. 7.4 reads in our copy:
/// −∇·(e^{xy} ∇u) + β(∂u/∂x + ∂u/∂y + ∂u/∂z) + γ u / (1 + x + y + z) = f on the unit cube,
/// u = 0 on its faces, β = 25, γ = 1, by centred differences on 15³ interior points (h = 1/16):
/// N = 3375 and 22 275 nonzeros, as they have. (Their cell Reynolds number below one, βh/2 =
/// 0.78, is what fixes the convection term along all three axes.) Scaled by h².
pub(crate) fn matrix() -> Sparse {
    const P: usize = 15;
    let (h, beta, gamma) = (1.0 / 16.0, 25.0, 1.0);
    let index = |i: usize, j: usize, k: usize| (i * P + j) * P + k;
    let a = |x: f64, y: f64| (x * y).exp();
    let mut entries = Vec::new();
    for i in 0..P {
        for j in 0..P {
            for k in 0..P {
                let (x, y, z) = ((i + 1) as f64 * h, (j + 1) as f64 * h, (k + 1) as f64 * h);
                let r = index(i, j, k);
                // the diffusion's coefficients at the half points, and the convection's βh/2
                let (xp, xm) = (a(x + h / 2.0, y), a(x - h / 2.0, y));
                let (yp, ym) = (a(x, y + h / 2.0), a(x, y - h / 2.0));
                let zz = a(x, y);
                let c = beta * h / 2.0;
                let diagonal = xp + xm + yp + ym + 2.0 * zz + gamma * h * h / (1.0 + x + y + z);
                entries.push((r, r, c64::new(diagonal, 0.0)));
                // each neighbour inside the cube, (i, j, k) to it, and its coefficient
                let up = |n: usize| (n + 1 < P).then_some(n + 1);
                let down = |n: usize| n.checked_sub(1);
                let neighbours = [
                    (up(i).map(|i| index(i, j, k)), -xp + c),
                    (down(i).map(|i| index(i, j, k)), -xm - c),
                    (up(j).map(|j| index(i, j, k)), -yp + c),
                    (down(j).map(|j| index(i, j, k)), -ym - c),
                    (up(k).map(|k| index(i, j, k)), -zz + c),
                    (down(k).map(|k| index(i, j, k)), -zz - c),
                ];
                for (at, v) in neighbours {
                    if let Some(at) = at {
                        entries.push((r, at, c64::new(v, 0.0)));
                    }
                }
            }
        }
    }
    Sparse::new(P * P * P, entries)
}

/// A preconditioned by two-sided SSOR with ω = 1, as their examples are (their Eq. 7.3):
/// A' = M₁⁻¹ A M₂⁻¹, M₁ = (D + L) D^{-1/2}, M₂ = D^{-1/2} (D + U), A = L + D + U.
pub(crate) struct Ssor<'a> {
    a: &'a Sparse,
    t: Sparse,
    root: Vec<f64>,
}

impl Ssor<'_> {
    pub(crate) fn new(a: &Sparse) -> Ssor<'_> {
        let n = a.n;
        let t = Sparse::new(
            n,
            (0..n).flat_map(|r| a.row(r).map(move |(c, v)| (c, r, v)).collect::<Vec<_>>()),
        );
        let root = (0..n)
            .map(|r| {
                a.row(r)
                    .find(|&(c, _)| c == r)
                    .map_or(1.0, |(_, v)| v.re.sqrt())
            })
            .collect();
        Ssor { a, t, root }
    }

    /// M₁⁻¹ b, b' of their Eq. 7.2.
    pub(crate) fn left(&self, b: &[c64]) -> Vec<c64> {
        let mut y = lower(self.a, b);
        y.iter_mut().zip(&self.root).for_each(|(v, d)| *v *= d);
        y
    }
}

/// (D + L)⁻¹ b over `m`'s rows.
fn lower(m: &Sparse, b: &[c64]) -> Vec<c64> {
    let mut y = vec![c64::new(0.0, 0.0); b.len()];
    for r in 0..b.len() {
        let (mut s, mut d) = (b[r], c64::new(1.0, 0.0));
        for (c, v) in m.row(r) {
            if c < r {
                s -= v * y[c];
            } else if c == r {
                d = v;
            }
        }
        y[r] = s / d;
    }
    y
}

/// (D + U)⁻¹ b over `m`'s rows.
fn upper(m: &Sparse, b: &[c64]) -> Vec<c64> {
    let mut y = vec![c64::new(0.0, 0.0); b.len()];
    for r in (0..b.len()).rev() {
        let (mut s, mut d) = (b[r], c64::new(1.0, 0.0));
        for (c, v) in m.row(r) {
            if c > r {
                s -= v * y[c];
            } else if c == r {
                d = v;
            }
        }
        y[r] = s / d;
    }
    y
}

impl Operator for Ssor<'_> {
    fn size(&self) -> usize {
        self.a.n
    }

    fn apply(&self, v: &[c64]) -> Vec<c64> {
        // M₂⁻¹ v = (D + U)⁻¹ D^{1/2} v, then A, then M₁⁻¹
        let scaled: Vec<c64> = v.iter().zip(&self.root).map(|(x, d)| x * d).collect();
        self.left(&self.a.apply(&upper(self.a, &scaled)))
    }

    fn apply_transpose(&self, v: &[c64]) -> Vec<c64> {
        // M₁⁻ᵀ v = (D + Lᵀ)⁻¹ D^{1/2} v, then Aᵀ, then M₂⁻ᵀ = D^{1/2} (D + Uᵀ)⁻¹
        let scaled: Vec<c64> = v.iter().zip(&self.root).map(|(x, d)| x * d).collect();
        let mut y = lower(&self.t, &self.a.apply_transpose(&upper(&self.t, &scaled)));
        y.iter_mut().zip(&self.root).for_each(|(x, d)| *x *= d);
        y
    }
}

/// Uniform random numbers in (−1, 1), from a fixed seed (xorshift64*).
pub(crate) fn uniform(seed: u64, n: usize) -> Vec<c64> {
    let mut state = seed.max(1);
    (0..n)
        .map(|_| {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let bits = state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
            c64::new(2.0 * (bits as f64 / (1u64 << 53) as f64) - 1.0, 0.0)
        })
        .collect()
}

/// Their Example 7.1's first runs: block QMR on the SSOR-preconditioned system, X₀ = 0, the
/// right-hand sides and the left block L random (uniform on (−1, 1)), stopped at a relative
/// residual of 1e-6 of the preconditioned system: the iterations for one right-hand side and
/// for five. Theirs: 19 and 85. Each the median over `draws` draws. Their stopping test isn't
/// legible in our scanned copy (nor is Eq. 7.4 whole); at 1e-6 this gives 14 and 63, at 1e-9 18
/// and 85: the counts depend on it, their ratio (4.5 to 4.7) hardly.
pub(crate) fn iterations(draws: u64) -> (usize, usize) {
    let a = matrix();
    let ssor = Ssor::new(&a);
    let n = a.n;
    let stopping = Stopping {
        tolerance: 1e-6,
        max_iterations: 2000,
    };
    let median = |m: usize| -> usize {
        let mut counts: Vec<usize> = (0..draws)
            .map(|draw| {
                let seed = 1 + 1000 * draw;
                let f: Vec<Vec<c64>> = (0..m as u64)
                    .map(|k| ssor.left(&uniform(seed + k, n)))
                    .collect();
                let l: Vec<Vec<c64>> = (0..m as u64).map(|k| uniform(seed + 500 + k, n)).collect();
                block_qmr_left(&ssor, &f, Some(&l), stopping, false, None)
                    .map_or(usize::MAX, |(_, how)| how.iterations)
            })
            .collect();
        counts.sort_unstable();
        counts[counts.len() / 2]
    };
    (median(1), median(5))
}

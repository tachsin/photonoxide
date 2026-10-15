//! Parks et al.'s Section 4.4 rebuilt: GCRO-DR(25, 10) on a convection–diffusion problem, run
//! twice, for the validation report.

use faer::Mat;
use num_complex::Complex64 as c64;

use super::super::{Sparse, Stopping, gmres_preconditioned, norm};
use super::{Identity, Space, gcro_dr, qr};

/// The interior points along each side: h = 1/41.
const P: usize = 40;

/// M. L. Parks et al., SIAM J. Sci. Comput. 28, 1651 (2006), doi:10.1137/040607277, Section 4.4:
/// u_xx + u_yy + c u_x = 0 on the unit square, u = 0 on y = 0 and x = 0, u = 1 on y = 1 and
/// x = 1, by central differences on 40 × 40 interior points (h = 1/41): their 1600 × 1600
/// matrix, scaled by h², and its right-hand side, the boundary's values moved over. Point (i, j),
/// at (i h, j h) for i, j = 1 … 40, is unknown (i − 1) + 40 (j − 1).
pub(crate) fn convection_diffusion(c: f64) -> (Sparse, Vec<c64>) {
    let h = 1.0 / (P + 1) as f64;
    let index = |i: usize, j: usize| i + P * j;
    let (east, west) = (1.0 + c * h / 2.0, 1.0 - c * h / 2.0);
    let mut entries = Vec::new();
    let mut b = vec![c64::new(0.0, 0.0); P * P];
    for j in 0..P {
        for i in 0..P {
            let r = index(i, j);
            entries.push((r, r, c64::new(-4.0, 0.0)));
            for (inside, at, weight, boundary) in [
                (i + 1 < P, (i + 1, j), east, 1.0),
                (i > 0, (i.wrapping_sub(1), j), west, 0.0),
                (j + 1 < P, (i, j + 1), 1.0, 1.0),
                (j > 0, (i, j.wrapping_sub(1)), 1.0, 0.0),
            ] {
                if inside {
                    entries.push((r, index(at.0, at.1), c64::new(weight, 0.0)));
                } else {
                    b[r] -= weight * boundary;
                }
            }
        }
    }
    (Sparse::new(P * P, entries), b)
}

/// Parks et al.'s Theorem 3.1 with an exact invariant subspace (δ = 0): GCRO-DR recycling it
/// converges as GMRES on the deflated problem. A = H D H, n = 300, with H = I − 2 w wᴴ a
/// Householder reflector (unitary, a dense A) and D diagonal: 8 eigenvalues near zero,
/// 0.002 j e^{i j/3} for j = 1 … 8, the rest on 1.5 + e^{2πi j/292}/1.2; U the 8 eigenvectors
/// H eⱼ of the small ones. GCRO-DR(299, 8) with U, from b, against full GMRES on D's other 292
/// eigenvalues from Hᴴ b's other components: the residual's history, step by step (as
/// fractions of ‖b‖). The largest difference between the two histories relative to the
/// reference's value at that step, and the iterations each took and full GMRES took on A itself,
/// to 1e-10.
pub(crate) fn deflated_against_gmres() -> (f64, usize, usize, usize) {
    const N: usize = 300;
    const K: usize = 8;
    let w: Vec<c64> = (0..N)
        .map(|i| c64::new(1.0 + (i % 7) as f64, (i % 5) as f64 - 2.0))
        .collect();
    let length = w.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    let w: Vec<c64> = w.iter().map(|z| z / length).collect();
    let h = |i: usize, j: usize| -> c64 {
        let identity = if i == j { 1.0 } else { 0.0 };
        c64::new(identity, 0.0) - 2.0 * w[i] * w[j].conj()
    };
    let d: Vec<c64> = (0..N)
        .map(|j| {
            if j < K {
                c64::from_polar(0.002 * (j + 1) as f64, (j + 1) as f64 / 3.0)
            } else {
                let angle = 2.0 * std::f64::consts::PI * (j - K) as f64 / (N - K) as f64;
                c64::new(1.5, 0.0) + c64::from_polar(1.0 / 1.2, angle)
            }
        })
        .collect();
    // A = H D H, H Hermitian
    let hm = Mat::<c64>::from_fn(N, N, h);
    let dm = Mat::<c64>::from_fn(N, N, |i, j| if i == j { d[i] } else { c64::new(0.0, 0.0) });
    let am = &hm * &dm * &hm;
    let a = Sparse::new(
        N,
        (0..N)
            .flat_map(|i| (0..N).map(move |j| (i, j)))
            .map(|(i, j)| (i, j, am[(i, j)])),
    );
    let b: Vec<c64> = (0..N)
        .map(|i| c64::new(((i * 3) % 11) as f64 - 5.0, ((i * 7) % 13) as f64 - 6.0))
        .collect();
    let stop = Stopping {
        tolerance: 1e-10,
        max_iterations: N,
    };
    let mut space = Space {
        u: (0..K).map(|j| (0..N).map(|i| h(i, j)).collect()).collect(),
        ..Space::default()
    };
    let Ok((_, recycled)) = gcro_dr(&a, &Identity, &b, stop, N - 1, K, &mut space, 0) else {
        return (f64::NAN, 0, 0, 0);
    };
    // the deflated problem: D's other eigenvalues, from Hᴴ b's other components
    let hb: Vec<c64> = (0..N)
        .map(|i| (0..N).map(|j| h(j, i).conj() * b[j]).sum())
        .collect();
    let deflated = Sparse::new(N - K, (0..N - K).map(|i| (i, i, d[i + K])));
    let rest = &hb[K..];
    let (Ok((_, reference)), Ok((_, whole))) = (
        gmres_preconditioned(
            &deflated,
            &Identity,
            rest,
            Stopping {
                tolerance: 1e-10 * norm(&b) / norm(rest),
                max_iterations: N,
            },
            N,
        ),
        gmres_preconditioned(&a, &Identity, &b, stop, N),
    ) else {
        return (f64::NAN, 0, 0, 0);
    };
    let scale = norm(rest) / norm(&b);
    let worst = recycled
        .history
        .iter()
        .zip(&reference.history)
        .map(|(p, q)| (p - q * scale).abs() / (q * scale))
        .fold(0.0, f64::max);
    (
        worst,
        recycled.iterations,
        reference.iterations,
        whole.iterations,
    )
}

/// The stopping test of their Section 4: a relative residual of 1e-10.
fn stopping() -> Stopping {
    Stopping {
        tolerance: 1e-10,
        max_iterations: 10_000,
    }
}

/// The problem with `c` solved twice by GCRO-DR(25, 10), the second time recycling the space
/// the first left (their "ideal" case), and once by full GMRES: the Arnoldi steps of each
/// (first, second, full GMRES) and the space the first left.
pub(crate) fn twice(c: f64) -> (usize, usize, usize, Space) {
    let (a, b) = convection_diffusion(c);
    let mut space = Space::default();
    let first = gcro_dr(&a, &Identity, &b, stopping(), 25, 10, &mut space, 0)
        .map_or(usize::MAX, |(_, how)| how.iterations);
    let left = space.clone();
    let second = gcro_dr(&a, &Identity, &b, stopping(), 25, 10, &mut space, 0)
        .map_or(usize::MAX, |(_, how)| how.iterations);
    let full = gmres_preconditioned(&a, &Identity, &b, stopping(), P * P)
        .map_or(usize::MAX, |(_, how)| how.iterations);
    (first, second, full, left)
}

/// Their Table 4.3 for c = 0: the cosines of the principal angles between the space GCRO-DR(25,
/// 10) recycles after its first run and the invariant subspace of the 10 eigenvalues of smallest
/// magnitude, largest first. For c = 0 the matrix is the 5-point Laplacian, whose eigenvectors
/// are sin(pπ i h) sin(qπ j h) with eigenvalues −4 sin²(pπh/2) − 4 sin²(qπh/2): the 10 smallest
/// are (p, q) = (1, 1), (1, 2), (2, 1), (2, 2), (1, 3), (3, 1), (2, 3), (3, 2), (1, 4), (4, 1),
/// and (3, 3) the 11th.
pub(crate) fn table_cosines() -> Vec<f64> {
    let (_, _, _, space) = twice(0.0);
    let h = 1.0 / (P + 1) as f64;
    let pi = std::f64::consts::PI;
    let modes = [
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (1, 3),
        (3, 1),
        (2, 3),
        (3, 2),
        (1, 4),
        (4, 1),
    ];
    let eigenvectors: Vec<Vec<c64>> = modes
        .iter()
        .map(|&(p, q)| {
            let mut v = vec![c64::new(0.0, 0.0); P * P];
            for j in 0..P {
                for i in 0..P {
                    let (x, y) = ((i + 1) as f64 * h, (j + 1) as f64 * h);
                    v[i + P * j] =
                        c64::new((p as f64 * pi * x).sin() * (q as f64 * pi * y).sin(), 0.0);
                }
            }
            let length = v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
            v.iter().map(|z| z / length).collect()
        })
        .collect();
    let (basis, _, _) = qr(space.u);
    let overlaps = Mat::<c64>::from_fn(basis.len(), eigenvectors.len(), |i, j| {
        basis[i]
            .iter()
            .zip(&eigenvectors[j])
            .map(|(p, q)| p.conj() * q)
            .sum()
    });
    let mut cosines = overlaps.singular_values().unwrap_or_default();
    cosines.sort_by(|p, q| q.total_cmp(p));
    cosines
}

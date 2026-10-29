//! Real least squares for small dense problems: min ‖A x − b‖ by a QR factorization and an SVD of
//! its triangle, the kernel of the vector fits ([`crate::compact`]) and of the media fits (the
//! non-negative poles of [`crate::fdtd`]).

use faer::Mat;

/// Singular values below this, relative to the largest, are dropped from a least squares: the
/// directions the data can't determine, as when more poles are asked for than the response has.
const RCOND: f64 = 1e-13;

/// Real least squares min ‖A x − b‖, columns scaled to unit norm first, and solved through the
/// singular values of R (A = QR), the smallest dropped below [`RCOND`]: the minimum-norm solution
/// when the problem is rank-deficient.
pub(crate) fn least_squares(a: Mat<f64>, b: &Mat<f64>) -> Vec<f64> {
    let (m, n) = (a.nrows(), a.ncols());
    let scale: Vec<f64> = (0..n)
        .map(|j| {
            let v = a.col(j).norm_l2();
            if v > 0.0 { 1.0 / v } else { 1.0 }
        })
        .collect();
    // [A D, b] = QR: R's first n columns and its last column, Qᵀb
    let augmented = Mat::from_fn(m, n + 1, |i, j| {
        if j < n {
            a[(i, j)] * scale[j]
        } else {
            b[(i, 0)]
        }
    });
    let qr = augmented.qr();
    let r = qr.thin_R();
    let rows = r.nrows().min(n);
    let rn = Mat::from_fn(rows, n, |i, j| r[(i, j)]);
    let qtb: Vec<f64> = (0..rows).map(|i| r[(i, n)]).collect();
    // faer's SVD, or where it doesn't converge (it can, rarely, on a nearly singular R whose
    // rounding differs between systems: #398) one-sided Jacobi, which always does
    let (u, s, v) = match rn.thin_svd() {
        Ok(svd) => (
            svd.U().to_owned(),
            svd.S()
                .column_vector()
                .iter()
                .copied()
                .collect::<Vec<f64>>(),
            svd.V().to_owned(),
        ),
        Err(_) => jacobi_svd(&rn),
    };
    let top = s.iter().copied().fold(0.0, f64::max);
    let mut x = vec![0.0; n];
    for (k, &sk) in s.iter().enumerate() {
        if sk <= RCOND * top || sk == 0.0 {
            continue;
        }
        let coef = (0..rows).map(|i| u[(i, k)] * qtb[i]).sum::<f64>() / sk;
        for (j, xj) in x.iter_mut().enumerate() {
            *xj += coef * v[(j, k)];
        }
    }
    x.iter().zip(&scale).map(|(x, w)| x * w).collect()
}

/// The thin SVD A = U Σ Vᵀ of a small dense matrix by one-sided Jacobi (M. R. Hestenes, J. Soc.
/// Ind. Appl. Math. 6, 51 (1958), doi:10.1137/0106005): pairs of A's columns rotated until every
/// pair is orthogonal to round-off, V accumulating the rotations; the columns' norms are Σ.
/// Slower than faer's, but it converges on any finite matrix: [`least_squares`]' fallback.
/// The singular values are in the columns' order, not sorted; a zero one has a zero column of U.
pub(crate) fn jacobi_svd(a: &Mat<f64>) -> (Mat<f64>, Vec<f64>, Mat<f64>) {
    let (m, n) = (a.nrows(), a.ncols());
    let mut u = a.to_owned();
    let mut v = Mat::<f64>::from_fn(n, n, |i, j| f64::from(u8::from(i == j)));
    for _ in 0..100 {
        let mut rotated = false;
        for p in 0..n {
            for q in p + 1..n {
                let (mut alpha, mut beta, mut gamma) = (0.0, 0.0, 0.0);
                for i in 0..m {
                    alpha += u[(i, p)] * u[(i, p)];
                    beta += u[(i, q)] * u[(i, q)];
                    gamma += u[(i, p)] * u[(i, q)];
                }
                if gamma == 0.0 || gamma.abs() <= f64::EPSILON * (alpha * beta).sqrt() {
                    continue;
                }
                rotated = true;
                let zeta = (beta - alpha) / (2.0 * gamma);
                let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = c * t;
                for i in 0..m {
                    let (x, y) = (u[(i, p)], u[(i, q)]);
                    u[(i, p)] = c * x - s * y;
                    u[(i, q)] = s * x + c * y;
                }
                for i in 0..n {
                    let (x, y) = (v[(i, p)], v[(i, q)]);
                    v[(i, p)] = c * x - s * y;
                    v[(i, q)] = s * x + c * y;
                }
            }
        }
        if !rotated {
            break;
        }
    }
    let s: Vec<f64> = (0..n)
        .map(|k| (0..m).map(|i| u[(i, k)] * u[(i, k)]).sum::<f64>().sqrt())
        .collect();
    for (k, &sk) in s.iter().enumerate() {
        for i in 0..m {
            u[(i, k)] = if sk > 0.0 { u[(i, k)] / sk } else { 0.0 };
        }
    }
    (u, s, v)
}

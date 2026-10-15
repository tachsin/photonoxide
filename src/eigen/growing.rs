//! The restarted Arnoldi process the mode solvers used before Krylov–Schur (photonoxide 0.5.1),
//! kept to compare the two: each restart from the wanted Ritz vectors combined into one, in a
//! Krylov space one first run larger than the last.

use faer::c64;

use super::{Pair, Shifted, dot, norm};
use crate::{Error, Result};

/// As [`super::nearest`], by the growing restarts; with the most basis vectors held at once.
pub(crate) fn nearest_growing(
    n: usize,
    entries: &[(usize, usize, c64)],
    shift: c64,
    count: usize,
    tolerance: f64,
) -> Result<(Vec<Pair>, usize)> {
    let count = count.min(n);
    let op = Shifted::new(n, entries, shift)?;
    // the Krylov space of the first run; each restart that follows takes one as large again
    let first = (2 * count + 20).max(40);
    let mut start = super::start(n);
    let mut best: Vec<Pair> = Vec::new();
    let mut held = 0;
    for restart in 0..20 {
        let m = n.min(first * (restart + 1));
        let s = norm(&start);
        let mut v: Vec<Vec<c64>> = vec![start.iter().map(|x| x / s).collect()];
        let mut h = faer::Mat::<c64>::zeros(m + 1, m);
        let mut steps = m;
        for j in 0..m {
            let mut w = op.solve(&v[j]);
            let before = norm(&w);
            for pass in 0..2 {
                for (i, vi) in v.iter().enumerate() {
                    let hij = dot(&w, vi);
                    h[(i, j)] += hij;
                    for (wk, vk) in w.iter_mut().zip(vi) {
                        *wk -= hij * vk;
                    }
                }
                if pass == 0 && norm(&w) > 0.7 * before {
                    break;
                }
            }
            let hn = norm(&w);
            h[(j + 1, j)] = c64::new(hn, 0.0);
            if hn <= 1e-14 * before.max(f64::MIN_POSITIVE) {
                steps = j + 1;
                break;
            }
            v.push(w.iter().map(|x| x / hn).collect());
        }
        held = held.max(v.len());
        let hm = faer::Mat::<c64>::from_fn(steps, steps, |i, j| h[(i, j)]);
        let eig = hm
            .eigen()
            .map_err(|e| Error::invalid("eigenproblem", format!("{e:?}")))?;
        let theta = eig.S();
        let y = eig.U();
        let mut order: Vec<usize> = (0..steps).collect();
        order.sort_by(|&a, &b| theta[b].norm().total_cmp(&theta[a].norm()));
        let mut pairs = Vec::with_capacity(count);
        let mut all = true;
        for &k in order.iter().take(count) {
            let value = shift + c64::new(1.0, 0.0) / theta[k];
            let mut x = vec![c64::new(0.0, 0.0); n];
            for (i, vi) in v.iter().take(steps).enumerate() {
                let c = y[(i, k)];
                for (xj, vj) in x.iter_mut().zip(vi) {
                    *xj += c * vj;
                }
            }
            let xn = norm(&x);
            let x: Vec<c64> = x.iter().map(|z| z / xn).collect();
            if op.residual(value, &x) > tolerance * value.norm().max(1.0) {
                all = false;
            }
            pairs.push(Pair { value, vector: x });
        }
        if all {
            return Ok((pairs, held));
        }
        start = vec![c64::new(0.0, 0.0); n];
        for p in &pairs {
            for (s, x) in start.iter_mut().zip(&p.vector) {
                *s += x;
            }
        }
        best = pairs;
    }
    Err(Error::invalid(
        "eigenproblem",
        format!(
            "{} eigenpairs near {shift} didn't converge in 20 restarts",
            best.len()
        ),
    ))
}

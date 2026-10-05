//! Large entries onto the diagonal: the row permutation that maximizes the product of the
//! diagonal's magnitudes, and the scaling that makes the permuted diagonal ±1 in magnitude and
//! every other entry at most 1 (I. S. Duff, J. Koster, "On algorithms for permuting large
//! entries to the diagonal of a sparse matrix", SIAM J. Matrix Anal. Appl. 22, 973 (2001),
//! doi:10.1137/S0895479899358443: their Section 4's weighted matching and Section 6's scaling,
//! after Olschowka and Neumaier; MC64's option 5).
//!
//! The product is maximized as a minimum-weight perfect matching on the bipartite graph of rows
//! and columns, each entry an edge of weight c_ij = log a_j − log |a_ij| (a_j the largest
//! magnitude in column j), found by shortest augmenting paths on reduced weights
//! c_ij − u_i − v_j ≥ 0 (their Figure 4.1, Dijkstra's search with a heap). The dual variables
//! u, v of the optimal matching give the scaling: p_i = exp(u_i), q_j = exp(v_j) / a_j, with
//! p_i |a_ij| q_j = exp(u_i + v_j − c_ij) ≤ 1, equal to 1 on the matching. Here each factor is
//! rounded to a power of two, so that scaling changes no digit of the matrix (Li & Demmel's
//! remark): the bounds then hold to within a factor of 2.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// The matching and the scaling of [`matching`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Matching {
    /// The row put on column j's diagonal: (Pr A)_jj = a_{rows[j], j}.
    pub(crate) rows: Vec<usize>,
    /// The rows' scale factors, powers of two, by original row.
    pub(crate) row_scale: Vec<f64>,
    /// The columns' scale factors, powers of two.
    pub(crate) column_scale: Vec<f64>,
}

/// A distance with a total order, for the heap.
#[derive(Clone, Copy, PartialEq)]
struct Distance(f64);

impl Eq for Distance {}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The maximum-product matching of the n × n matrix whose columns are `columns[j]`, lists of
/// (row, value), and its scaling.
///
/// # Errors
///
/// [`Error::InvalidValue`] if the matrix is structurally singular: no permutation puts a
/// nonzero on every diagonal entry.
pub(crate) fn matching(n: usize, columns: &[Vec<(usize, c64)>]) -> Result<Matching> {
    let singular = |what: String| Error::invalid("the matching", what);
    // the weights c_ij, by column, without the zeros
    let mut costs: Vec<Vec<(usize, f64)>> = Vec::with_capacity(n);
    let mut largest = vec![0.0f64; n];
    for (j, column) in columns.iter().enumerate() {
        let a = column.iter().map(|(_, v)| v.norm()).fold(0.0, f64::max);
        if a == 0.0 || !a.is_finite() {
            return Err(singular(format!("column {j} has no finite nonzero")));
        }
        largest[j] = a;
        let log_a = a.ln();
        costs.push(
            column
                .iter()
                .filter(|(_, v)| v.norm() > 0.0)
                .map(|&(i, v)| (i, log_a - v.norm().ln()))
                .collect(),
        );
    }
    // the dual variables' start (their Section 4, after Carpaneto and Toth): u_i the smallest
    // weight in row i, v_j the smallest reduced weight in column j
    let mut u = vec![f64::INFINITY; n];
    for column in &costs {
        for &(i, c) in column {
            u[i] = u[i].min(c);
        }
    }
    if let Some(i) = u.iter().position(|x| x.is_infinite()) {
        return Err(singular(format!("row {i} has no nonzero")));
    }
    let mut v: Vec<f64> = costs
        .iter()
        .map(|column| {
            column
                .iter()
                .map(|&(i, c)| c - u[i])
                .fold(f64::INFINITY, f64::min)
        })
        .collect();
    // an initial matching from the edges of zero reduced weight
    const NONE: usize = usize::MAX;
    let mut row_of = vec![NONE; n];
    let mut column_of = vec![NONE; n];
    for (j, column) in costs.iter().enumerate() {
        for &(i, c) in column {
            if column_of[i] == NONE && c - u[i] - v[j] <= 0.0 {
                row_of[j] = i;
                column_of[i] = j;
                break;
            }
        }
    }
    // the shortest augmenting path from each unmatched column (their Figure 4.1)
    let mut distance = vec![f64::INFINITY; n];
    let mut done = vec![false; n];
    let mut previous = vec![NONE; n];
    let mut touched: Vec<usize> = Vec::new();
    let mut heap: BinaryHeap<Reverse<(Distance, usize)>> = BinaryHeap::new();
    for root in 0..n {
        if row_of[root] != NONE {
            continue;
        }
        let mut shortest = 0.0;
        let mut augmenting = f64::INFINITY;
        let mut end = NONE;
        let mut settled: Vec<usize> = Vec::new();
        let mut j = root;
        loop {
            for &(i, c) in &costs[j] {
                if done[i] {
                    continue;
                }
                let reduced = (c - u[i] - v[j]).max(0.0);
                let d = shortest + reduced;
                if d >= augmenting {
                    continue;
                }
                if column_of[i] == NONE {
                    augmenting = d;
                    end = i;
                    if distance[i].is_infinite() {
                        touched.push(i);
                    }
                    distance[i] = d;
                    previous[i] = j;
                } else if d < distance[i] {
                    if distance[i].is_infinite() {
                        touched.push(i);
                    }
                    distance[i] = d;
                    previous[i] = j;
                    heap.push(Reverse((Distance(d), i)));
                }
            }
            // the closest row not yet settled
            let next = loop {
                match heap.pop() {
                    None => break None,
                    Some(Reverse((Distance(d), i))) => {
                        if !done[i] && d == distance[i] {
                            break Some((d, i));
                        }
                    }
                }
            };
            let Some((d, i)) = next else { break };
            if augmenting <= d {
                break;
            }
            shortest = d;
            done[i] = true;
            settled.push(i);
            j = column_of[i];
        }
        heap.clear();
        if end == NONE {
            return Err(singular(format!(
                "column {root} can't be matched: the matrix is structurally singular"
            )));
        }
        // the duals, so that the reduced weights stay nonnegative and are zero on the matching
        for &i in &settled {
            u[i] += distance[i] - augmenting;
        }
        // augment along the path: each row on it takes the column it was reached from
        let mut path = Vec::new();
        let mut i = end;
        loop {
            path.push(i);
            let j = previous[i];
            let next = row_of[j];
            row_of[j] = i;
            column_of[i] = j;
            if j == root {
                break;
            }
            i = next;
        }
        // v_j = c_ij − u_i on the matching, where u_i or the match changed
        for &i in settled.iter().chain(&path) {
            let j = column_of[i];
            v[j] = cost(&costs[j], i) - u[i];
        }
        for &i in &touched {
            distance[i] = f64::INFINITY;
            previous[i] = NONE;
        }
        for &i in &settled {
            done[i] = false;
        }
        touched.clear();
    }
    let power = |x: f64| 2f64.powi(x.log2().round() as i32);
    Ok(Matching {
        rows: row_of,
        row_scale: u.iter().map(|&x| power(x.exp())).collect(),
        column_scale: (0..n).map(|j| power(v[j].exp() / largest[j])).collect(),
    })
}

/// The weight of entry (i, ·) in a column's list.
fn cost(column: &[(usize, f64)], i: usize) -> f64 {
    column
        .iter()
        .find(|&&(r, _)| r == i)
        .map_or(f64::INFINITY, |&(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns_of(dense: &[&[f64]]) -> Vec<Vec<(usize, c64)>> {
        let n = dense.len();
        (0..n)
            .map(|j| {
                (0..n)
                    .filter(|&i| dense[i][j] != 0.0)
                    .map(|i| (i, c64::new(dense[i][j], 0.0)))
                    .collect()
            })
            .collect()
    }

    /// The largest product of |a_{σ(j), j}| over every permutation σ, by brute force.
    fn best_product(dense: &[&[f64]]) -> f64 {
        fn permute(k: usize, used: &mut Vec<bool>, dense: &[&[f64]], acc: f64, best: &mut f64) {
            let n = dense.len();
            if k == n {
                *best = best.max(acc);
                return;
            }
            for i in 0..n {
                if !used[i] && dense[i][k] != 0.0 {
                    used[i] = true;
                    permute(k + 1, used, dense, acc * dense[i][k].abs(), best);
                    used[i] = false;
                }
            }
        }
        let mut best = 0.0;
        permute(0, &mut vec![false; dense.len()], dense, 1.0, &mut best);
        best
    }

    #[test]
    fn the_matching_maximizes_the_diagonals_product() {
        // small matrices whose best permutation brute force finds
        let cases: [&[&[f64]]; 3] = [
            &[&[1.0, 5.0, 0.0], &[4.0, 0.5, 2.0], &[0.0, 3.0, 0.1]],
            &[
                &[0.0, 2.0, 0.0, 9.0],
                &[3.0, 0.0, 1.0, 0.0],
                &[0.0, 7.0, 0.0, 1.0],
                &[1.0, 0.0, 6.0, 0.0],
            ],
            &[
                &[1e-8, 1.0, 0.0, 0.0, 2.0],
                &[1.0, 1e-8, 3.0, 0.0, 0.0],
                &[0.0, 4.0, 1e-8, 1.0, 0.0],
                &[0.0, 0.0, 1.0, 1e-8, 5.0],
                &[6.0, 0.0, 0.0, 2.0, 1e-8],
            ],
        ];
        for dense in cases {
            let n = dense.len();
            let m = matching(n, &columns_of(dense)).unwrap();
            let mut seen = m.rows.clone();
            seen.sort_unstable();
            assert_eq!(seen, (0..n).collect::<Vec<_>>(), "a permutation");
            let product: f64 = (0..n).map(|j| dense[m.rows[j]][j].abs()).product();
            let best = best_product(dense);
            assert!((product - best).abs() <= 1e-12 * best, "{product} {best}");
            // scaled: the diagonal 1 and the rest at most 1, within the powers of two's rounding
            for (j, &row) in m.rows.iter().enumerate() {
                for (i, row_i) in dense.iter().enumerate() {
                    let s = (row_i[j] * m.row_scale[i] * m.column_scale[j]).abs();
                    assert!(s <= 2.0 + 1e-12, "({i}, {j}): {s}");
                    if i == row {
                        assert!((0.5..=2.0).contains(&s), "diagonal ({i}, {j}): {s}");
                    }
                }
                assert!(m.row_scale[j].log2().fract() == 0.0);
                assert!(m.column_scale[j].log2().fract() == 0.0);
            }
        }
    }

    #[test]
    fn a_dominant_diagonal_stays_and_a_singular_matrix_is_refused() {
        // diagonally dominant, a 1D Laplacian with a shift: the identity is the best matching
        let n = 50;
        let columns: Vec<Vec<(usize, c64)>> = (0..n)
            .map(|j| {
                let mut c = vec![(j, c64::new(4.0, 0.5))];
                if j > 0 {
                    c.push((j - 1, c64::new(-1.0, 0.0)));
                }
                if j + 1 < n {
                    c.push((j + 1, c64::new(-1.0, 0.1)));
                }
                c
            })
            .collect();
        let m = matching(n, &columns).unwrap();
        assert_eq!(m.rows, (0..n).collect::<Vec<_>>());
        // two rows with entries in one column only: no perfect matching
        let singular = vec![
            vec![(0, c64::new(1.0, 0.0)), (1, c64::new(1.0, 0.0))],
            vec![(2, c64::new(1.0, 0.0))],
            vec![(2, c64::new(2.0, 0.0))],
        ];
        assert!(matching(3, &singular).is_err());
    }
}

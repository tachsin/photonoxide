//! Sparse LU factorization in an order of photonoxide's choosing: nested dissection on the
//! grids its matrices come from (A. George, "Nested dissection of a regular finite element mesh",
//! SIAM J. Numer. Anal. 10, 345 (1973), doi:10.1137/0710032), passed to faer's supernodal LU as
//! its column permutation, which faer's own entry point fixes to COLAMD.
//!
//! George numbers each independent part of the grid before the set that separates it from its
//! neighbours, recursively (his Section 4): in 2D the factorization then takes O(n³) operations
//! and O(n² log n) storage on an n × n grid, against O(n⁴) and O(n³) numbered row by row, and the
//! same holds for an unsymmetric matrix with a symmetric structure (his closing remarks).
//!
//! Here the parts are cut geometrically, at the median of the longest axis of the unknowns'
//! grid positions, and the separator is taken from the matrix's own graph: the unknowns on one
//! side with a neighbour on the other. That separates for any stencil (the 2D solver's 5 points,
//! the mode solvers' wider ones, Yee's edges, a Bloch side's wrap-around), where George's sets
//! are drawn for one.

use faer::dyn_stack::{MemBuffer, MemStack, StackReq};
use faer::perm::PermRef;
use faer::sparse::linalg::SymbolicSupernodalParams;
use faer::sparse::linalg::lu::supernodal::{
    SupernodalLu, SymbolicSupernodalLu, factorize_supernodal_numeric_lu,
    factorize_supernodal_numeric_lu_scratch, factorize_supernodal_symbolic_lu,
    factorize_supernodal_symbolic_lu_scratch, solve_in_place_scratch,
};
use faer::sparse::linalg::qr::{col_etree, column_counts_ata, postorder};
use faer::sparse::{SparseColMat, SparseColMatRef};
use faer::{Conj, Mat, Par};
use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// Below this many unknowns a part is numbered as it is, not dissected further.
const LEAF: usize = 64;

/// The structure of A + Aᵀ without its diagonal, by rows: each unknown's neighbours.
pub(crate) fn adjacency(n: usize, entries: &[(usize, usize)]) -> (Vec<usize>, Vec<usize>) {
    let mut lists: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(r, c) in entries {
        if r != c {
            lists[r].push(c);
            lists[c].push(r);
        }
    }
    let mut starts = Vec::with_capacity(n + 1);
    let mut neighbours = Vec::new();
    starts.push(0);
    for mut list in lists {
        list.sort_unstable();
        list.dedup();
        neighbours.extend(list);
        starts.push(neighbours.len());
    }
    (starts, neighbours)
}

/// The order to eliminate the unknowns in: each part before the separator that cuts it from the
/// rest, recursively. `positions[u]` is unknown u's place on its grid (any units; only their
/// order along each axis matters); `(starts, neighbours)` the graph of [`adjacency`]. Fixed by
/// the grid and the structure, not by the values or the machine.
pub(crate) fn nested_dissection(
    starts: &[usize],
    neighbours: &[usize],
    positions: &[[f64; 3]],
) -> Vec<usize> {
    let n = positions.len();
    let mut order = Vec::with_capacity(n);
    // 0: elsewhere, 1: this cut's low side, 2: its high side, 3: the separator
    let mut side = vec![0u8; n];
    // a separator is numbered after both halves it cuts apart, so the work list holds it, to
    // emit once the halves pushed after it are done
    enum Work {
        Part(Vec<usize>),
        Emit(Vec<usize>),
    }
    let mut work = vec![Work::Part((0..n).collect())];
    while let Some(item) = work.pop() {
        let mut part = match item {
            Work::Emit(separator) => {
                order.extend(separator);
                continue;
            }
            Work::Part(part) => part,
        };
        if part.len() <= LEAF {
            part.sort_unstable();
            order.extend(part);
            continue;
        }
        // the longest axis of the part's box
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for &u in &part {
            for a in 0..3 {
                low[a] = low[a].min(positions[u][a]);
                high[a] = high[a].max(positions[u][a]);
            }
        }
        let axis = (0..3)
            .max_by(|&a, &b| (high[a] - low[a]).total_cmp(&(high[b] - low[b])))
            .unwrap_or(0);
        if high[axis] <= low[axis] {
            part.sort_unstable();
            order.extend(part);
            continue;
        }
        // cut at the median position along it; ties go to the high side, so neither is empty
        let middle = part.len() / 2;
        part.select_nth_unstable_by(middle, |&u, &v| {
            positions[u][axis]
                .total_cmp(&positions[v][axis])
                .then(u.cmp(&v))
        });
        let cut = positions[part[middle]][axis];
        let (mut lower, mut upper): (Vec<usize>, Vec<usize>) =
            part.iter().partition(|&&u| positions[u][axis] < cut);
        if lower.is_empty() {
            // the median is the lowest position: cut just above it instead
            (lower, upper) = part.iter().partition(|&&u| positions[u][axis] <= cut);
            if upper.is_empty() {
                part.sort_unstable();
                order.extend(part);
                continue;
            }
        }
        for &u in &lower {
            side[u] = 1;
        }
        for &u in &upper {
            side[u] = 2;
        }
        // the separator: the low side's unknowns within two steps of the high side. faer pivots
        // rows, so the factors' structure is bounded by AᵀA's, whose graph joins two columns
        // that share a row: two steps in A's. One step would separate A, not AᵀA (George's sets
        // are for a factorization without pivoting).
        let touches = |u: usize, side: &[u8], of: u8| {
            neighbours[starts[u]..starts[u + 1]]
                .iter()
                .any(|&v| side[v] == of)
        };
        let first: Vec<usize> = lower
            .iter()
            .copied()
            .filter(|&u| touches(u, &side, 2))
            .collect();
        for &u in &first {
            side[u] = 3;
        }
        let second: Vec<usize> = lower
            .iter()
            .copied()
            .filter(|&u| side[u] == 1 && touches(u, &side, 3))
            .collect();
        for &u in &second {
            side[u] = 3;
        }
        let (mut separator, mut rest): (Vec<usize>, Vec<usize>) =
            lower.iter().partition(|&&u| side[u] == 3);
        for &u in lower.iter().chain(&upper) {
            side[u] = 0;
        }
        separator.sort_unstable();
        rest.sort_unstable();
        upper.sort_unstable();
        work.push(Work::Emit(separator));
        work.push(Work::Part(upper));
        work.push(Work::Part(rest));
    }
    order
}

/// A sparse LU's analysis with its columns in a given order: what a matrix of the same structure
/// needs to be factorized again (a sweep's other wavelengths).
#[derive(Clone, Debug)]
pub(crate) struct OrderedSymbolic {
    columns: (Vec<usize>, Vec<usize>),
    symbolic: SymbolicSupernodalLu<usize>,
    entries: usize,
}

impl OrderedSymbolic {
    /// The analysis of `a`'s structure (its values aren't read), its columns eliminated in
    /// `order` (each column once): faer's own steps for its supernodal LU, with this order in
    /// place of COLAMD's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the order isn't a permutation of the columns, or if faer can't
    /// analyse the structure (out of memory).
    pub(crate) fn new(a: SparseColMatRef<'_, usize, c64>, order: &[usize]) -> Result<Self> {
        let (m, n) = (a.nrows(), a.ncols());
        let mut inverse = vec![usize::MAX; n];
        for (k, &c) in order.iter().enumerate() {
            if c >= n || inverse[c] != usize::MAX {
                return Err(failed("the order", format!("{c} twice or out of 0..{n}")));
            }
            inverse[c] = k;
        }
        if order.len() != n {
            return Err(failed(
                "the order",
                format!("{} columns of {n}", order.len()),
            ));
        }
        let forward = order.to_vec();
        let col_perm = PermRef::<'_, usize>::new_checked(&forward, &inverse, n);
        let at = transposed(a)?;
        let mut etree = vec![0usize; n];
        let mut min_col = vec![0usize; m];
        let mut col_counts = vec![0usize; n];
        let etree = {
            let mut post = vec![0usize; n];
            let etree = col_etree(
                a.symbolic(),
                Some(col_perm),
                &mut etree,
                MemStack::new(&mut MemBuffer::new(StackReq::new::<usize>(m + n))),
            );
            postorder(
                &mut post,
                etree,
                MemStack::new(&mut MemBuffer::new(StackReq::new::<usize>(3 * n))),
            );
            column_counts_ata(
                &mut col_counts,
                &mut min_col,
                at.symbolic(),
                Some(col_perm),
                etree,
                &post,
                MemStack::new(&mut MemBuffer::new(StackReq::new::<usize>(5 * n + m))),
            );
            etree
        };
        let symbolic = factorize_supernodal_symbolic_lu::<usize>(
            a.symbolic(),
            Some(col_perm),
            &min_col,
            etree,
            &col_counts,
            MemStack::new(&mut MemBuffer::new(
                factorize_supernodal_symbolic_lu_scratch::<usize>(m, n),
            )),
            SymbolicSupernodalParams::default(),
        )
        .map_err(|e| failed("the analysis", format!("{e:?}")))?;
        let entries = 2 * col_counts.iter().sum::<usize>() - n;
        Ok(OrderedSymbolic {
            columns: (forward, inverse),
            symbolic,
            entries,
        })
    }

    /// The entries of L and U the factorization makes room for: the structure of the Cholesky
    /// factor of AᵀA in this column order, once for L and once for U, the diagonal shared. Partial
    /// pivoting by rows stays inside it (George & Ng 1987), and faer's supernodal LU fills it.
    pub(crate) fn factor_entries(&self) -> usize {
        self.entries
    }

    fn columns(&self) -> PermRef<'_, usize> {
        PermRef::<'_, usize>::new_checked(&self.columns.0, &self.columns.1, self.columns.0.len())
    }
}

/// A sparse LU factorization with partial pivoting by rows, its columns in the order of its
/// [`OrderedSymbolic`].
#[derive(Clone, Debug)]
pub(crate) struct OrderedLu {
    symbolic: OrderedSymbolic,
    rows: (Vec<usize>, Vec<usize>),
    lu: SupernodalLu<usize, c64>,
}

impl OrderedLu {
    /// The factors of `a`, whose structure `symbolic` analysed.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if faer can't factorize the matrix (out of memory, a singular
    /// matrix).
    pub(crate) fn new(
        a: SparseColMatRef<'_, usize, c64>,
        symbolic: OrderedSymbolic,
        par: Par,
    ) -> Result<Self> {
        let n = a.ncols();
        let at = transposed(a)?;
        let mut rows = (vec![0usize; n], vec![0usize; n]);
        let mut lu = SupernodalLu::<usize, c64>::new();
        factorize_supernodal_numeric_lu(
            &mut rows.0,
            &mut rows.1,
            &mut lu,
            a,
            at.as_ref(),
            symbolic.columns(),
            &symbolic.symbolic,
            par,
            MemStack::new(&mut MemBuffer::new(
                factorize_supernodal_numeric_lu_scratch::<usize, c64>(
                    &symbolic.symbolic,
                    Default::default(),
                ),
            )),
            Default::default(),
        )
        .map_err(|e| failed("the factorization", format!("{e:?}")))?;
        Ok(OrderedLu { symbolic, rows, lu })
    }

    /// The analysis, for a matrix of the same structure.
    pub(crate) fn symbolic(&self) -> &OrderedSymbolic {
        &self.symbolic
    }

    /// x with A x = `b`.
    pub(crate) fn solve(&self, b: &[c64]) -> Vec<c64> {
        self.solved(b, false)
    }

    /// x with Aᵀ x = `b`.
    pub(crate) fn solve_transpose(&self, b: &[c64]) -> Vec<c64> {
        self.solved(b, true)
    }

    fn solved(&self, b: &[c64], transpose: bool) -> Vec<c64> {
        let n = b.len();
        let par = faer::get_global_parallelism();
        let mut x = Mat::<c64>::from_fn(n, 1, |r, _| b[r]);
        let rows = PermRef::<'_, usize>::new_checked(&self.rows.0, &self.rows.1, n);
        let mut scratch = MemBuffer::new(solve_in_place_scratch::<usize, c64>(n, 1, par));
        let stack = MemStack::new(&mut scratch);
        if transpose {
            self.lu.solve_transpose_in_place_with_conj(
                rows,
                self.symbolic.columns(),
                Conj::No,
                x.as_mut(),
                par,
                stack,
            );
        } else {
            self.lu.solve_in_place_with_conj(
                rows,
                self.symbolic.columns(),
                Conj::No,
                x.as_mut(),
                par,
                stack,
            );
        }
        (0..n).map(|r| x[(r, 0)]).collect()
    }
}

fn failed(what: &str, reason: String) -> Error {
    Error::invalid("sparse LU", format!("{what}: {reason}"))
}

/// Aᵀ, stored by columns.
fn transposed(a: SparseColMatRef<'_, usize, c64>) -> Result<SparseColMat<usize, c64>> {
    a.transpose()
        .to_col_major()
        .map_err(|e| failed("Aᵀ", format!("{e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 5-point stencil's graph on an nx × ny grid, and the cells' positions.
    fn grid(nx: usize, ny: usize) -> (Vec<usize>, Vec<usize>, Vec<[f64; 3]>) {
        let mut entries = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                if i + 1 < nx {
                    entries.push((k, k + 1));
                }
                if j + 1 < ny {
                    entries.push((k, k + nx));
                }
            }
        }
        let (starts, neighbours) = adjacency(nx * ny, &entries);
        let positions = (0..nx * ny)
            .map(|k| [(k % nx) as f64, (k / nx) as f64, 0.0])
            .collect();
        (starts, neighbours, positions)
    }

    #[test]
    fn the_order_is_a_permutation_with_each_separator_after_its_parts() {
        let (nx, ny) = (37, 23);
        let (starts, neighbours, positions) = grid(nx, ny);
        let order = nested_dissection(&starts, &neighbours, &positions);
        let mut seen = order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..nx * ny).collect::<Vec<_>>());
        // the last unknowns numbered are the first cut, across the longest axis (x): two whole
        // columns of the grid, side by side, since AᵀA reaches two steps
        let last: Vec<usize> = order[order.len() - 2 * ny..].to_vec();
        let columns: Vec<usize> = last
            .iter()
            .map(|k| k % nx)
            .collect::<std::collections::BTreeSet<usize>>()
            .into_iter()
            .collect();
        assert_eq!(columns.len(), 2, "{columns:?}");
        assert_eq!(columns[1], columns[0] + 1, "{columns:?}");
    }

    #[test]
    fn an_ordered_lu_solves_the_system() {
        // a shifted 5-point Laplacian, complex, unsymmetric in its values
        let (nx, ny) = (40, 30);
        let n = nx * ny;
        let mut triplets = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                triplets.push(faer::sparse::Triplet::new(k, k, c64::new(4.0, 0.3)));
                if i + 1 < nx {
                    triplets.push(faer::sparse::Triplet::new(k, k + 1, c64::new(-1.0, 0.1)));
                    triplets.push(faer::sparse::Triplet::new(k + 1, k, c64::new(-1.0, -0.2)));
                }
                if j + 1 < ny {
                    triplets.push(faer::sparse::Triplet::new(k, k + nx, c64::new(-1.0, 0.0)));
                    triplets.push(faer::sparse::Triplet::new(k + nx, k, c64::new(-1.0, 0.05)));
                }
            }
        }
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets).unwrap();
        let (starts, neighbours, positions) = grid(nx, ny);
        let order = nested_dissection(&starts, &neighbours, &positions);
        let symbolic = OrderedSymbolic::new(a.as_ref(), &order).unwrap();
        let lu = OrderedLu::new(a.as_ref(), symbolic, Par::Seq).unwrap();
        let b: Vec<c64> = (0..n).map(|i| c64::new((i % 7) as f64, 1.0)).collect();
        let column = |v: &[c64]| Mat::<c64>::from_fn(n, 1, |r, _| v[r]);
        let size = column(&b).norm_max();
        let x = lu.solve(&b);
        let residual = (&a * column(&x) - column(&b)).norm_max() / size;
        assert!(residual < 1e-12, "{residual}");
        let x = lu.solve_transpose(&b);
        let residual = (a.as_ref().transpose() * column(&x) - column(&b)).norm_max() / size;
        assert!(residual < 1e-12, "transposed: {residual}");
        // a matrix of the same structure, factorized with the analysis again
        let doubled: Vec<_> = triplets
            .iter()
            .map(|t| faer::sparse::Triplet::new(t.row, t.col, t.val * 2.0))
            .collect();
        let a2 = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &doubled).unwrap();
        let lu2 = OrderedLu::new(a2.as_ref(), lu.symbolic().clone(), Par::Seq).unwrap();
        // twice the matrix, half the solution
        let half: f64 = (lu.solve(&b).iter())
            .zip(lu2.solve(&b))
            .map(|(y, z)| (y / 2.0 - z).norm())
            .fold(0.0, f64::max);
        assert!(half < 1e-12, "{half}");
    }

    #[test]
    fn the_factors_entries_are_counted_exactly() {
        // tridiagonal: AᵀA is pentadiagonal and its Cholesky factor in the natural order has no
        // fill, 3 entries a column but 2 and 1 in the last two, so L and U together hold 5n − 6
        let n = 50;
        let mut triplets = Vec::new();
        for k in 0..n {
            triplets.push(faer::sparse::Triplet::new(k, k, c64::new(2.0, 0.1)));
            if k + 1 < n {
                triplets.push(faer::sparse::Triplet::new(k, k + 1, c64::new(-1.0, 0.0)));
                triplets.push(faer::sparse::Triplet::new(k + 1, k, c64::new(-1.0, 0.2)));
            }
        }
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets).unwrap();
        let natural: Vec<usize> = (0..n).collect();
        let counted = OrderedSymbolic::new(a.as_ref(), &natural).unwrap();
        assert_eq!(counted.factor_entries(), 5 * n - 6);
        // and the 2D Laplacian: nested dissection fills less than the natural (banded) order
        let (nx, ny) = (40, 30);
        let (starts, neighbours, positions) = grid(nx, ny);
        let mut triplets = Vec::new();
        for (k, w) in starts.windows(2).enumerate() {
            triplets.push(faer::sparse::Triplet::new(k, k, c64::new(4.0, 0.3)));
            for &l in &neighbours[w[0]..w[1]] {
                triplets.push(faer::sparse::Triplet::new(k, l, c64::new(-1.0, 0.1)));
            }
        }
        let n = nx * ny;
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets).unwrap();
        let natural: Vec<usize> = (0..n).collect();
        let banded = OrderedSymbolic::new(a.as_ref(), &natural).unwrap();
        let dissected = OrderedSymbolic::new(
            a.as_ref(),
            &nested_dissection(&starts, &neighbours, &positions),
        )
        .unwrap();
        assert!(
            dissected.factor_entries() < banded.factor_entries(),
            "{} {}",
            dissected.factor_entries(),
            banded.factor_entries()
        );
    }
}

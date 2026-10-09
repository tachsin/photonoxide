//! Dense kernels for the multifrontal fronts (#186): the products, triangular solves and LU
//! that photonoxide's own sparse solver spends its time in, as a trait another library can
//! stand behind.
//!
//! photonoxide's multifrontal solver ([`crate::sparse`]) eliminates each front with four dense
//! operations on complex matrices, which are BLAS's and LAPACK's:
//!
//! | Here | BLAS, LAPACK | Where |
//! |---|---|---|
//! | [`DenseKernels::multiply`] | `zgemm`, `zgemmt` | the Schur complements, the panels' updates |
//! | [`DenseKernels::solve_unit_lower`] | `ztrsm` (left, lower, unit) | U₁₂ = L₁₁⁻¹ F₁₂ |
//! | [`DenseKernels::solve_upper_from_right`] | `ztrsm` (right, upper) | L₂₁ = F₂₁ U₁₁⁻¹ |
//! | [`DenseKernels::solve_unit_lower_transposed_from_right`] | `ztrsm` (right, lower, transposed, unit) | L D = F L⁻ᵀ |
//! | [`DenseKernels::lu`] | `zgetrf` | a front's diagonal block |
//!
//! The ordering, the fronts, their assembly, the static pivoting and the scheduling of
//! independent subtrees on rayon's threads stay photonoxide's whatever the kernels: only the
//! arithmetic inside a front is another library's. Each call says whether it may thread
//! (`threaded`): the large fronts near the root do, the many small ones factorized side by
//! side must not.
//!
//! [`with_kernels`] makes a direct solver of photonoxide's own with a library's kernels,
//! named `photonoxide-` and the kernels' name, to be registered and chosen as any other
//! backend. The solver registered as `photonoxide` keeps faer's kernels, called as they
//! always were, and its bits; [`Faer`] is the same kernels behind the trait, the reference a
//! library's are checked against.
//!
//! A [`Block`] is a matrix by columns in a larger one's storage: its rows, its columns and
//! the distance between its columns (BLAS's leading dimension). photonoxide forbids `unsafe`,
//! so a block gives its first entry's address and no more; reading through it is the calling
//! crate's to answer for (`photonoxide-native`).

use std::sync::Arc;

use faer::linalg::matmul::matmul;
use faer::linalg::matmul::triangular::{BlockStructure, matmul as triangular};
use faer::linalg::triangular_solve::{
    solve_lower_triangular_in_place, solve_unit_lower_triangular_in_place,
};
use faer::{Accum, MatMut, MatRef, Par};
use num_complex::Complex64 as c64;

use super::{Capabilities, DirectSolver, Threads};

/// A matrix to read, by columns, inside a larger one's storage.
#[derive(Clone, Copy)]
pub struct Block<'a>(pub(crate) MatRef<'a, c64>);

/// A matrix to write, by columns, inside a larger one's storage.
pub struct BlockMut<'a>(pub(crate) MatMut<'a, c64>);

impl<'a> Block<'a> {
    /// The block these entries are, by columns, one column after another.
    ///
    /// # Panics
    ///
    /// Unless there are rows × columns of them.
    pub fn of(entries: &'a [c64], rows: usize, columns: usize) -> Block<'a> {
        Block(MatRef::from_column_major_slice(entries, rows, columns))
    }

    /// The block a faer matrix is.
    ///
    /// # Panics
    ///
    /// If its rows aren't one after another in memory (a transposed view).
    pub(crate) fn new(matrix: MatRef<'a, c64>) -> Block<'a> {
        assert!(matrix.nrows() == 0 || matrix.row_stride() == 1);
        Block(matrix)
    }

    /// Its rows.
    pub fn rows(&self) -> usize {
        self.0.nrows()
    }

    /// Its columns.
    pub fn columns(&self) -> usize {
        self.0.ncols()
    }

    /// The entries from one column's first to the next's: BLAS's leading dimension, at least
    /// the rows and at least 1.
    pub fn stride(&self) -> usize {
        self.0
            .col_stride()
            .unsigned_abs()
            .max(self.0.nrows())
            .max(1)
    }

    /// Its first entry's address. Entry (i, j) is `i + j * stride()` entries on; nothing is
    /// behind it when the block has no rows or no columns.
    pub fn pointer(&self) -> *const c64 {
        self.0.as_ptr()
    }
}

impl<'a> BlockMut<'a> {
    /// The block these entries are, by columns, one column after another.
    ///
    /// # Panics
    ///
    /// Unless there are rows × columns of them.
    pub fn of(entries: &'a mut [c64], rows: usize, columns: usize) -> BlockMut<'a> {
        BlockMut(MatMut::from_column_major_slice_mut(entries, rows, columns))
    }

    /// The block a faer matrix is.
    ///
    /// # Panics
    ///
    /// As [`Block::new`].
    pub(crate) fn new(matrix: MatMut<'a, c64>) -> BlockMut<'a> {
        assert!(matrix.nrows() == 0 || matrix.row_stride() == 1);
        BlockMut(matrix)
    }

    /// Its rows.
    pub fn rows(&self) -> usize {
        self.0.nrows()
    }

    /// Its columns.
    pub fn columns(&self) -> usize {
        self.0.ncols()
    }

    /// As [`Block::stride`].
    pub fn stride(&self) -> usize {
        self.0
            .col_stride()
            .unsigned_abs()
            .max(self.0.nrows())
            .max(1)
    }

    /// Its first entry's address, to write through. As [`Block::pointer`].
    pub fn pointer(&mut self) -> *mut c64 {
        self.0.as_ptr_mut()
    }
}

/// Which product [`DenseKernels::multiply`] is asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Product {
    /// C + α A Bᵀ (without conjugation) in place of C + α A B.
    pub transposed: bool,
    /// Only C's lower triangle (its diagonal with it) is wanted, C being square: what is
    /// written above it is not read.
    pub lower: bool,
}

/// What a library's dense kernels are.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Kernels {
    /// The name they are known by: lower-case letters, digits, `-` and `_`.
    pub name: String,
    /// The library's version.
    pub version: String,
    /// The library's licence, by name.
    pub licence: String,
    /// How a threaded call's threads are set.
    pub threads: Threads,
    /// Whether a call gives the same bits on every run and any number of threads.
    pub deterministic: bool,
}

impl Kernels {
    /// Kernels of this name, version and licence, on one thread, not deterministic.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        licence: impl Into<String>,
    ) -> Kernels {
        Kernels {
            name: name.into(),
            version: version.into(),
            licence: licence.into(),
            threads: Threads::One,
            deterministic: false,
        }
    }
}

/// The dense operations a front's elimination is made of, on complex matrices by columns.
///
/// No block of a call overlaps another. A block without rows or columns leaves nothing to do.
pub trait DenseKernels: Send + Sync {
    /// What they are.
    fn kernels(&self) -> Kernels;

    /// C ← C + α A B, or C + α A Bᵀ ([`Product::transposed`]): A is m × r, B is r × n (n × r
    /// transposed), C is m × n.
    fn multiply(
        &self,
        c: BlockMut<'_>,
        a: Block<'_>,
        b: Block<'_>,
        product: Product,
        alpha: c64,
        threaded: bool,
    );

    /// B ← L⁻¹ B: L is the lower triangle of `l` with ones on its diagonal, what `l` holds on
    /// and above its diagonal not read.
    fn solve_unit_lower(&self, l: Block<'_>, b: BlockMut<'_>, threaded: bool);

    /// B ← B U⁻¹: U is the upper triangle of `u`, its diagonal with it.
    fn solve_upper_from_right(&self, u: Block<'_>, b: BlockMut<'_>, threaded: bool);

    /// B ← B L⁻ᵀ: L is the lower triangle of `l` with ones on its diagonal.
    fn solve_unit_lower_transposed_from_right(&self, l: Block<'_>, b: BlockMut<'_>, threaded: bool);

    /// The LU of the square block with partial pivoting, in place: L below the diagonal, its
    /// ones not stored, U on and above it. Returns the rows' order: entry i is the row of the
    /// block that the factors' row i was.
    fn lu(&self, a: BlockMut<'_>, threaded: bool) -> Vec<usize>;
}

fn par(threaded: bool) -> Par {
    if threaded { Par::rayon(0) } else { Par::Seq }
}

/// faer's kernels behind the trait: the calls photonoxide's own solver makes, and the
/// reference a library's kernels are checked against.
#[derive(Clone, Copy, Debug, Default)]
pub struct Faer;

impl DenseKernels for Faer {
    fn kernels(&self) -> Kernels {
        Kernels {
            threads: Threads::Rayon,
            deterministic: true,
            ..Kernels::new("faer", "0.24", "MIT")
        }
    }

    fn multiply(
        &self,
        c: BlockMut<'_>,
        a: Block<'_>,
        b: Block<'_>,
        product: Product,
        alpha: c64,
        threaded: bool,
    ) {
        let b = if product.transposed {
            b.0.transpose()
        } else {
            b.0
        };
        if product.lower {
            triangular(
                c.0,
                BlockStructure::TriangularLower,
                Accum::Add,
                a.0,
                BlockStructure::Rectangular,
                b,
                BlockStructure::Rectangular,
                alpha,
                par(threaded),
            );
        } else {
            matmul(c.0, Accum::Add, a.0, b, alpha, par(threaded));
        }
    }

    fn solve_unit_lower(&self, l: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        solve_unit_lower_triangular_in_place(l.0, b.0, par(threaded));
    }

    fn solve_upper_from_right(&self, u: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        // X U = B is Uᵀ Xᵀ = Bᵀ, and Uᵀ is lower
        solve_lower_triangular_in_place(u.0.transpose(), b.0.transpose_mut(), par(threaded));
    }

    fn solve_unit_lower_transposed_from_right(
        &self,
        l: Block<'_>,
        b: BlockMut<'_>,
        threaded: bool,
    ) {
        // X Lᵀ = B is L Xᵀ = Bᵀ
        solve_unit_lower_triangular_in_place(l.0, b.0.transpose_mut(), par(threaded));
    }

    fn lu(&self, a: BlockMut<'_>, threaded: bool) -> Vec<usize> {
        crate::sparse::faer_lu(a.0, par(threaded))
    }
}

/// photonoxide's own direct solver with these kernels in its fronts: its ordering, its fill
/// and its static pivoting, the library's arithmetic. Its name is `photonoxide-` and the
/// kernels' name; it is deterministic if they are.
pub fn with_kernels(kernels: Arc<dyn DenseKernels>) -> Arc<dyn DirectSolver> {
    Arc::new(super::Photonoxide {
        kernels: Some(kernels),
    })
}

/// The capabilities of photonoxide's solver with `kernels`.
pub(crate) fn capabilities(own: Capabilities, kernels: &dyn DenseKernels) -> Capabilities {
    let k = kernels.kernels();
    Capabilities {
        name: format!("{}-{}", own.name, k.name),
        version: format!("{} with {} {}", own.version, k.name, k.version),
        licence: format!("{}; {}: {}", own.licence, k.name, k.licence),
        threads: match k.threads {
            Threads::One | Threads::Rayon => Threads::Rayon,
            Threads::Library(how) => {
                Threads::Library(format!("rayon's for the fronts; the kernels': {how}"))
            }
        },
        deterministic: own.deterministic && k.deterministic,
        ..own
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Choice, Form, Matrix, direct};

    /// The kernels written from the trait's words alone, one entry at a time: what a library's
    /// must agree with. What `lower` leaves unread is filled with NaN.
    struct Plain;

    impl DenseKernels for Plain {
        fn kernels(&self) -> Kernels {
            Kernels::new("plain", "0", "MIT OR Apache-2.0")
        }

        fn multiply(
            &self,
            mut c: BlockMut<'_>,
            a: Block<'_>,
            b: Block<'_>,
            product: Product,
            alpha: c64,
            _threaded: bool,
        ) {
            for i in 0..c.rows() {
                for j in 0..c.columns() {
                    if product.lower && j > i {
                        c.0[(i, j)] = c64::new(f64::NAN, f64::NAN);
                        continue;
                    }
                    let mut sum = c64::new(0.0, 0.0);
                    for r in 0..a.columns() {
                        let factor = if product.transposed {
                            b.0[(j, r)]
                        } else {
                            b.0[(r, j)]
                        };
                        sum += a.0[(i, r)] * factor;
                    }
                    c.0[(i, j)] += alpha * sum;
                }
            }
        }

        fn solve_unit_lower(&self, l: Block<'_>, mut b: BlockMut<'_>, _threaded: bool) {
            for j in 0..b.columns() {
                for i in 0..b.rows() {
                    let mut x = b.0[(i, j)];
                    for r in 0..i {
                        x -= l.0[(i, r)] * b.0[(r, j)];
                    }
                    b.0[(i, j)] = x;
                }
            }
        }

        fn solve_upper_from_right(&self, u: Block<'_>, mut b: BlockMut<'_>, _threaded: bool) {
            for i in 0..b.rows() {
                for j in 0..b.columns() {
                    let mut x = b.0[(i, j)];
                    for r in 0..j {
                        x -= b.0[(i, r)] * u.0[(r, j)];
                    }
                    b.0[(i, j)] = x / u.0[(j, j)];
                }
            }
        }

        fn solve_unit_lower_transposed_from_right(
            &self,
            l: Block<'_>,
            mut b: BlockMut<'_>,
            _threaded: bool,
        ) {
            // X Lᵀ = B: column j of X is B's minus the earlier columns' times L's row j
            for i in 0..b.rows() {
                for j in 0..b.columns() {
                    let mut x = b.0[(i, j)];
                    for r in 0..j {
                        x -= b.0[(i, r)] * l.0[(j, r)];
                    }
                    b.0[(i, j)] = x;
                }
            }
        }

        fn lu(&self, mut a: BlockMut<'_>, _threaded: bool) -> Vec<usize> {
            let n = a.rows();
            let mut rows: Vec<usize> = (0..n).collect();
            for j in 0..n {
                let best = (j..n)
                    .max_by(|&x, &y| a.0[(x, j)].norm().total_cmp(&a.0[(y, j)].norm()))
                    .unwrap();
                if best != j {
                    for c in 0..n {
                        let t = a.0[(j, c)];
                        a.0[(j, c)] = a.0[(best, c)];
                        a.0[(best, c)] = t;
                    }
                    rows.swap(j, best);
                }
                let pivot = a.0[(j, j)];
                for i in j + 1..n {
                    let l = a.0[(i, j)] / pivot;
                    a.0[(i, j)] = l;
                    for c in j + 1..n {
                        let u = a.0[(j, c)];
                        a.0[(i, c)] -= l * u;
                    }
                }
            }
            rows
        }
    }

    /// A Helmholtz operator with loss on a grid: complex symmetric, or general with an
    /// asymmetric coupling along x.
    fn helmholtz(cells: [usize; 3], form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
        let [mx, my, mz] = cells;
        let n = mx * my * mz;
        let strides = [1, mx, mx * my];
        let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
        for j in 0..n {
            let at = [j % mx, j / mx % my, j / (mx * my)];
            let mut column: Vec<(usize, c64)> = Vec::new();
            for axis in 0..3 {
                if at[axis] > 0 {
                    column.push((j - strides[axis], c64::new(1.0, 0.0)));
                }
                if at[axis] + 1 < cells[axis] {
                    let v = if form == Form::General && axis == 0 {
                        c64::new(1.0, 0.1)
                    } else {
                        c64::new(1.0, 0.0)
                    };
                    column.push((j + strides[axis], v));
                }
            }
            column.push((j, c64::new(-5.7, 0.02 + 0.001 * (j % 11) as f64)));
            column.sort_by_key(|&(r, _)| r);
            for (r, v) in column {
                rows.push(r);
                values.push(v);
            }
            starts.push(rows.len());
        }
        (starts, rows, values)
    }

    fn solutions(
        solver: &dyn DirectSolver,
        cells: [usize; 3],
        form: Form,
    ) -> (Vec<c64>, Vec<c64>, usize) {
        let (s, r, v) = helmholtz(cells, form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let b: Vec<c64> = (0..m.n())
            .map(|k| c64::new(1.0, k as f64 * 1e-3).powi((k % 3) as i32))
            .collect();
        let f = solver.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
        (
            f.solve(&b).unwrap(),
            f.solve_transpose(&b).unwrap(),
            f.report().perturbed_pivots,
        )
    }

    fn difference(a: &[c64], b: &[c64]) -> f64 {
        let scale = b.iter().map(|x| x.norm()).fold(0.0, f64::max);
        a.iter()
            .zip(b)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max)
            / scale
    }

    #[test]
    fn faers_kernels_behind_the_trait_give_the_solvers_own_bits() {
        let own = direct(&Choice::Photonoxide).unwrap();
        let through = with_kernels(Arc::new(Faer));
        // the second has fronts large enough to thread (a separator of 400 unknowns)
        for cells in [[40, 30, 1], [20, 20, 20]] {
            for form in [Form::General, Form::Symmetric] {
                let ours = solutions(own.as_ref(), cells, form);
                let theirs = solutions(through.as_ref(), cells, form);
                assert_eq!(ours.0, theirs.0, "{cells:?} {form:?}");
                assert_eq!(ours.1, theirs.1, "{cells:?} {form:?}, transposed");
                assert_eq!(ours.2, theirs.2);
            }
        }
    }

    #[test]
    fn kernels_written_from_the_traits_words_factorize_as_faers_do() {
        let own = direct(&Choice::Photonoxide).unwrap();
        let plain = with_kernels(Arc::new(Plain));
        for cells in [[40, 30, 1], [9, 8, 7]] {
            for form in [Form::General, Form::Symmetric] {
                let ours = solutions(own.as_ref(), cells, form);
                let theirs = solutions(plain.as_ref(), cells, form);
                let (plainly, transposed) = (
                    difference(&theirs.0, &ours.0),
                    difference(&theirs.1, &ours.1),
                );
                assert!(plainly < 1e-12, "{cells:?} {form:?}: {plainly:e}");
                assert!(transposed < 1e-12, "{cells:?} {form:?}: {transposed:e}");
                assert_eq!(ours.2, theirs.2);
            }
        }
    }

    #[test]
    fn a_solver_with_kernels_says_whose_they_are() {
        let c = with_kernels(Arc::new(Plain)).capabilities();
        assert_eq!(c.name, "photonoxide-plain");
        assert!(c.symmetric && c.transpose && !c.deterministic);
        assert!(c.licence.contains("plain"), "{}", c.licence);
        let c = with_kernels(Arc::new(Faer)).capabilities();
        assert_eq!(c.name, "photonoxide-faer");
        assert!(c.deterministic);
        assert_eq!(c.threads, Threads::Rayon);
        // and it registers as any other backend
        crate::backend::register(with_kernels(Arc::new(Plain))).unwrap();
        let chosen = direct(&Choice::Named("photonoxide-plain".into())).unwrap();
        assert_eq!(chosen.capabilities().name, "photonoxide-plain");
    }

    #[test]
    fn a_block_is_its_matrixs_columns() {
        let mut m = faer::Mat::<c64>::from_fn(5, 4, |i, j| c64::new(i as f64, j as f64));
        // (faer may leave room between a matrix's columns)
        let stride = m.as_ref().col_stride().unsigned_abs();
        assert!(stride >= 5);
        let block = Block::new(m.as_ref().submatrix(1, 1, 3, 2));
        assert_eq!(
            (block.rows(), block.columns(), block.stride()),
            (3, 2, stride)
        );
        assert_eq!(block.pointer(), m.as_ref().submatrix(1, 1, 3, 2).as_ptr());
        let mut block = BlockMut::new(m.as_mut().submatrix_mut(0, 2, 5, 2));
        assert_eq!(
            (block.rows(), block.columns(), block.stride()),
            (5, 2, stride)
        );
        assert!(!block.pointer().is_null());
        // entries by columns are a block as they are
        let mut entries: Vec<c64> = (0..6).map(|k| c64::new(k as f64, 0.0)).collect();
        let block = Block::of(&entries, 3, 2);
        assert_eq!((block.rows(), block.columns(), block.stride()), (3, 2, 3));
        assert_eq!(block.pointer(), entries.as_ptr());
        assert_eq!(BlockMut::of(&mut entries, 2, 3).stride(), 2);
        // an empty block still has a leading dimension BLAS takes
        let empty = faer::Mat::<c64>::zeros(0, 3);
        assert_eq!(Block::new(empty.as_ref()).stride(), 1);
    }

    #[test]
    #[should_panic]
    fn a_transposed_view_isnt_a_block() {
        let m = faer::Mat::<c64>::zeros(4, 3);
        let _ = Block::new(m.as_ref().transpose());
    }
}

//! The dense kernels' types and faer's implementation of them, below the sparse solver that
//! calls them and the backends that choose them. Public at [`crate::backend::dense`], where
//! the choice of kernels and the solver made from them are.

use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::matmul::matmul;
use faer::linalg::matmul::triangular::{BlockStructure, matmul as triangular};
use faer::linalg::triangular_solve::{
    solve_lower_triangular_in_place, solve_unit_lower_triangular_in_place,
};
use faer::traits::ComplexField;
use faer::{Accum, MatMut, MatRef, Par};
use num_complex::{Complex32 as c32, Complex64 as c64};

/// How a backend's threads are set.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Threads {
    /// One thread.
    One,
    /// rayon's: the pool the solve is called in, or `RAYON_NUM_THREADS`.
    Rayon,
    /// The library's own, set as these words say (an environment variable, a call).
    Library(String),
}

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
    /// How the library is called, as found when it was loaded: its routines' names, the size
    /// of its integers, how it returns a complex number. Empty if nothing needs saying.
    pub interface: String,
    /// What the library promises about repeating its bits, in words, and what was measured:
    /// on every run, and on another number of threads. Empty if nothing is known.
    pub reproducibility: String,
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
            interface: String::new(),
            reproducibility: String::new(),
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
            interface: "faer's own, in Rust".into(),
            reproducibility: "the same bits on every run and on any number of threads".into(),
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
        faer_multiply(c.0, a.0, b.0, product, alpha, threaded);
    }

    fn solve_unit_lower(&self, l: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        solve_unit_lower_triangular_in_place(l.0, b.0, par(threaded));
    }

    fn solve_upper_from_right(&self, u: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        faer_solve_upper_from_right(u.0, b.0, threaded);
    }

    fn solve_unit_lower_transposed_from_right(
        &self,
        l: Block<'_>,
        b: BlockMut<'_>,
        threaded: bool,
    ) {
        faer_solve_unit_lower_transposed_from_right(l.0, b.0, threaded);
    }

    fn lu(&self, a: BlockMut<'_>, threaded: bool) -> Vec<usize> {
        faer_lu(a.0, par(threaded))
    }
}

/// [`DenseKernels::multiply`] by faer, in any of its scalars.
fn faer_multiply<T: ComplexField>(
    c: MatMut<'_, T>,
    a: MatRef<'_, T>,
    b: MatRef<'_, T>,
    product: Product,
    alpha: T,
    threaded: bool,
) {
    let b = if product.transposed { b.transpose() } else { b };
    if product.lower {
        triangular(
            c,
            BlockStructure::TriangularLower,
            Accum::Add,
            a,
            BlockStructure::Rectangular,
            b,
            BlockStructure::Rectangular,
            alpha,
            par(threaded),
        );
    } else {
        matmul(c, Accum::Add, a, b, alpha, par(threaded));
    }
}

/// [`DenseKernels::solve_upper_from_right`] by faer, in any of its scalars.
fn faer_solve_upper_from_right<T: ComplexField>(
    u: MatRef<'_, T>,
    b: MatMut<'_, T>,
    threaded: bool,
) {
    // X U = B is Uᵀ Xᵀ = Bᵀ, and Uᵀ is lower
    solve_lower_triangular_in_place(u.transpose(), b.transpose_mut(), par(threaded));
}

/// [`DenseKernels::solve_unit_lower_transposed_from_right`] by faer, in any of its scalars.
fn faer_solve_unit_lower_transposed_from_right<T: ComplexField>(
    l: MatRef<'_, T>,
    b: MatMut<'_, T>,
    threaded: bool,
) {
    // X Lᵀ = B is L Xᵀ = Bᵀ
    solve_unit_lower_triangular_in_place(l, b.transpose_mut(), par(threaded));
}

/// The number a sparse factorization is computed and stored in: double precision (c64), as
/// photonoxide's own direct solver always has, or single (c32), whose factors take half the
/// memory and are refined in double ([`crate::sparse::refine`]). The fronts' dense operations
/// are faer's in either; a library's kernels ([`DenseKernels`]) take c64 only, and a c32
/// factorization is never given any.
pub(crate) trait Scalar:
    ComplexField
    + Copy
    + PartialEq
    + std::fmt::Debug
    + Send
    + Sync
    + 'static
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
    + std::ops::AddAssign
    + std::ops::SubAssign
    + std::ops::DivAssign
{
    /// Static pivoting's threshold relative to ‖A‖: a pivot below it is perturbed to it (Li and
    /// Demmel's √ε, double precision's in both).
    fn pivot_threshold() -> f64;

    /// `v`, rounded to this precision.
    fn from_c64(v: c64) -> Self;

    /// The real number `x`, rounded to this precision.
    fn real(x: f64) -> Self;

    /// |self|.
    fn modulus(self) -> f64;

    /// self × `x`.
    fn times(self, x: f64) -> Self;

    /// self / `x`.
    fn over(self, x: f64) -> Self;

    /// C ← C + α A B or C + α A Bᵀ, by `kernels` if given.
    fn multiply(
        c: MatMut<'_, Self>,
        a: MatRef<'_, Self>,
        b: MatRef<'_, Self>,
        product: Product,
        alpha: Self,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    );

    /// B ← L⁻¹ B, L unit lower, by `kernels` if given.
    fn solve_unit_lower(
        l: MatRef<'_, Self>,
        b: MatMut<'_, Self>,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    );

    /// B ← B U⁻¹, by `kernels` if given.
    fn solve_upper_from_right(
        u: MatRef<'_, Self>,
        b: MatMut<'_, Self>,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    );

    /// B ← B L⁻ᵀ, L unit lower, by `kernels` if given.
    fn solve_unit_lower_transposed_from_right(
        l: MatRef<'_, Self>,
        b: MatMut<'_, Self>,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    );

    /// The LU of the square block with partial pivoting, by `kernels` if given.
    fn lu(a: MatMut<'_, Self>, threaded: bool, kernels: Option<&dyn DenseKernels>) -> Vec<usize>;
}

impl Scalar for c64 {
    fn pivot_threshold() -> f64 {
        f64::EPSILON.sqrt()
    }

    fn from_c64(v: c64) -> c64 {
        v
    }

    fn real(x: f64) -> c64 {
        c64::new(x, 0.0)
    }

    fn modulus(self) -> f64 {
        self.norm()
    }

    fn times(self, x: f64) -> c64 {
        self * x
    }

    fn over(self, x: f64) -> c64 {
        self / x
    }

    fn multiply(
        c: MatMut<'_, c64>,
        a: MatRef<'_, c64>,
        b: MatRef<'_, c64>,
        product: Product,
        alpha: c64,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    ) {
        match kernels {
            Some(k) => k.multiply(
                BlockMut::new(c),
                Block::new(a),
                Block::new(b),
                product,
                alpha,
                threaded,
            ),
            None => faer_multiply(c, a, b, product, alpha, threaded),
        }
    }

    fn solve_unit_lower(
        l: MatRef<'_, c64>,
        b: MatMut<'_, c64>,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    ) {
        match kernels {
            Some(k) => k.solve_unit_lower(Block::new(l), BlockMut::new(b), threaded),
            None => solve_unit_lower_triangular_in_place(l, b, par(threaded)),
        }
    }

    fn solve_upper_from_right(
        u: MatRef<'_, c64>,
        b: MatMut<'_, c64>,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    ) {
        match kernels {
            Some(k) => k.solve_upper_from_right(Block::new(u), BlockMut::new(b), threaded),
            None => faer_solve_upper_from_right(u, b, threaded),
        }
    }

    fn solve_unit_lower_transposed_from_right(
        l: MatRef<'_, c64>,
        b: MatMut<'_, c64>,
        threaded: bool,
        kernels: Option<&dyn DenseKernels>,
    ) {
        match kernels {
            Some(k) => {
                k.solve_unit_lower_transposed_from_right(Block::new(l), BlockMut::new(b), threaded)
            }
            None => faer_solve_unit_lower_transposed_from_right(l, b, threaded),
        }
    }

    fn lu(a: MatMut<'_, c64>, threaded: bool, kernels: Option<&dyn DenseKernels>) -> Vec<usize> {
        match kernels {
            Some(k) => k.lu(BlockMut::new(a), threaded),
            None => faer_lu(a, par(threaded)),
        }
    }
}

impl Scalar for c32 {
    /// Double precision's, as photonoxide's own factors have it, not single's √ε (3.5 × 10⁻⁴):
    /// a perturbation enters the refinement's convergence as u_f ρₙ + ε (Amestoy et al.,
    /// 2023, Theorem 4.1), so at single's √ε it would dominate u_f = 6 × 10⁻⁸ a thousand times
    /// over, and at double's (1.5 × 10⁻⁸) it doesn't. Measured on the guide of
    /// docs/methods/mixed-precision.md at 24³: single's √ε perturbed 4 210 pivots, the L D Lᵀ
    /// gave way to the LU and GMRES took 45 to 50 iterations a step; double's perturbed none, two
    /// iterations a step.
    fn pivot_threshold() -> f64 {
        f64::EPSILON.sqrt()
    }

    fn from_c64(v: c64) -> c32 {
        c32::new(v.re as f32, v.im as f32)
    }

    fn real(x: f64) -> c32 {
        c32::new(x as f32, 0.0)
    }

    fn modulus(self) -> f64 {
        f64::from(self.norm())
    }

    fn times(self, x: f64) -> c32 {
        self * (x as f32)
    }

    fn over(self, x: f64) -> c32 {
        self / (x as f32)
    }

    fn multiply(
        c: MatMut<'_, c32>,
        a: MatRef<'_, c32>,
        b: MatRef<'_, c32>,
        product: Product,
        alpha: c32,
        threaded: bool,
        _: Option<&dyn DenseKernels>,
    ) {
        faer_multiply(c, a, b, product, alpha, threaded);
    }

    fn solve_unit_lower(
        l: MatRef<'_, c32>,
        b: MatMut<'_, c32>,
        threaded: bool,
        _: Option<&dyn DenseKernels>,
    ) {
        solve_unit_lower_triangular_in_place(l, b, par(threaded));
    }

    fn solve_upper_from_right(
        u: MatRef<'_, c32>,
        b: MatMut<'_, c32>,
        threaded: bool,
        _: Option<&dyn DenseKernels>,
    ) {
        faer_solve_upper_from_right(u, b, threaded);
    }

    fn solve_unit_lower_transposed_from_right(
        l: MatRef<'_, c32>,
        b: MatMut<'_, c32>,
        threaded: bool,
        _: Option<&dyn DenseKernels>,
    ) {
        faer_solve_unit_lower_transposed_from_right(l, b, threaded);
    }

    fn lu(a: MatMut<'_, c32>, threaded: bool, _: Option<&dyn DenseKernels>) -> Vec<usize> {
        faer_lu(a, par(threaded))
    }
}

/// faer's LU with partial pivoting of the square block in place: the rows' order, entry i the
/// row of the block that the factors' row i was.
pub(crate) fn faer_lu<T: ComplexField>(a: MatMut<'_, T>, par: Par) -> Vec<usize> {
    let k = a.nrows();
    let mut forward = vec![0usize; k];
    let mut inverse = vec![0usize; k];
    let mut stack = MemBuffer::new(
        faer::linalg::lu::partial_pivoting::factor::lu_in_place_scratch::<usize, T>(
            k,
            k,
            par,
            Default::default(),
        ),
    );
    faer::linalg::lu::partial_pivoting::factor::lu_in_place(
        a,
        &mut forward,
        &mut inverse,
        par,
        MemStack::new(&mut stack),
        Default::default(),
    );
    forward
}

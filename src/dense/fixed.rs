//! faer's QR, SVD and eigendecompositions, sequential: the same bits on any number of threads
//! (#822).
//!
//! faer's own methods (`Mat::qr`, `svd`, `thin_svd`, `singular_values`, `eigen`, `eigenvalues`,
//! `generalized_eigen`) run on its global parallelism, all of rayon's threads unless it is set, and its blocked
//! Householder code (faer 0.24's `linalg/householder.rs`) gives a block reflector's columns to
//! min(threads, 4) tasks, each a product of another shape. So a QR, an SVD that starts with one or
//! accumulates its vectors, and the eigenvectors (the Hessenberg reduction's Q) had other bits on
//! 2, 3 or 4 threads than on one: QR at 100 × 30 and from 128 × 128 on, the least squares of the
//! fits at 100 × 30 and 400 × 64 (the platform canaries, #832). rayon's threads are the machine's
//! cores unless `RAYON_NUM_THREADS` sets them, so the same fit could give other digits on a
//! 4-core and an 8-core machine. These functions compute the same decompositions as faer's methods,
//! step for step, on [`PAR`] instead. So are the eigenvalues alone (the canaries found their bits
//! the same on 1 to 5 threads up to 256 × 256, but they go through the same Hessenberg reduction)
//! and the generalized eigenproblems (QZ starts with a QR of B). faer's product and LUs give the
//! same bits on any number of threads and stay on rayon's.
//!
//! [`PAR`] is sequential because that was also the fastest at the sizes the library takes
//! (November 2026, faer 0.24.4, a Core Ultra 7 265K on 4 threads): a QR of 100 × 31 in 23 µs
//! against 444 µs on rayon's threads, the complex thin SVD of 300 × 30 in 0.35 ms against 1.4 ms,
//! an eigendecomposition of 100 × 100 in 4.7 ms against 7.9 ms; rayon's threads won only from
//! about 200 × 200 (a full SVD 8.8 ms against 7.2) and on tall matrices of thousands of rows (the
//! thin SVD of 4000 × 150, 52 ms against 38).
//!
//! The file uses faer only, never `crate::`, so that `tests/canary.rs` takes it by `#[path]` and
//! hashes the bits the library computes.

#![allow(
    dead_code,
    reason = "the canaries take the real eigendecomposition and the thin Q, the library doesn't"
)]

use faer::diag::Diag;
use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::evd::{self, ComputeEigenvectors, EvdError};
use faer::linalg::gevd::{self, GevdError};
use faer::linalg::householder::{
    apply_block_householder_sequence_on_the_left_in_place_scratch,
    apply_block_householder_sequence_on_the_left_in_place_with_conj,
};
use faer::linalg::qr::no_pivoting::factor::{
    qr_in_place, qr_in_place_scratch, recommended_block_size,
};
use faer::linalg::svd::{self, ComputeSvdVectors, SvdError};
use faer::sparse::linalg::LuError;
use faer::sparse::linalg::lu;
use faer::sparse::{FaerError, SparseColMatRef, SymbolicSparseColMatRef};
use faer::traits::ComplexField;
use faer::{Conj, Mat, MatMut, MatRef, Par};
use num_complex::Complex64 as c64;
use std::sync::Arc;

/// The parallelism of every decomposition here: sequential, whatever rayon's pool.
pub(crate) const PAR: Par = Par::Seq;

/// A Householder QR, A = Q R, as faer's `Qr`: Q as its Householder basis and coefficients.
pub(crate) struct Qr<T> {
    basis: Mat<T>,
    coefficients: Mat<T>,
    r: Mat<T>,
}

impl<T: ComplexField> Qr<T> {
    /// The QR of `a`, m × n.
    pub(crate) fn new(a: MatRef<'_, T>) -> Qr<T> {
        let mut qr = a.to_owned();
        let (m, n) = qr.shape();
        let size = m.min(n);
        let block = recommended_block_size::<T>(m, n);
        let mut coefficients = Mat::zeros(block, size);
        let mut scratch = MemBuffer::new(qr_in_place_scratch::<T>(
            m,
            n,
            block,
            PAR,
            Default::default(),
        ));
        qr_in_place(
            qr.as_mut(),
            coefficients.as_mut(),
            PAR,
            MemStack::new(&mut scratch),
            Default::default(),
        );
        // R is the upper triangle of the first min(m, n) rows; below it, the basis
        let mut r = Mat::zeros(size, n);
        for j in 0..n {
            for i in 0..=j.min(size - 1) {
                r[(i, j)] = qr[(i, j)].clone();
            }
        }
        let basis = Mat::from_fn(m, size, |i, j| {
            if i > j {
                qr[(i, j)].clone()
            } else if i == j {
                T::one_impl()
            } else {
                T::zero_impl()
            }
        });
        Qr {
            basis,
            coefficients,
            r,
        }
    }

    /// R's first min(m, n) rows, upper triangular: faer's `thin_R`.
    pub(crate) fn thin_r(&self) -> MatRef<'_, T> {
        self.r.as_ref()
    }

    /// Q's first min(m, n) columns: faer's `compute_thin_Q`.
    pub(crate) fn thin_q(&self) -> Mat<T> {
        let (m, size) = self.basis.shape();
        let mut q = Mat::identity(m, size);
        let mut scratch = MemBuffer::new(
            apply_block_householder_sequence_on_the_left_in_place_scratch::<T>(
                m,
                self.coefficients.nrows(),
                size,
            ),
        );
        apply_block_householder_sequence_on_the_left_in_place_with_conj(
            self.basis.as_ref(),
            self.coefficients.as_ref(),
            Conj::No,
            q.as_mut(),
            PAR,
            MemStack::new(&mut scratch),
        );
        q
    }
}

/// A singular value decomposition A = U Σ Vᴴ, as faer's `Svd`: Σ in nonincreasing order, real and
/// ≥ 0 (complex numbers for a complex A).
pub(crate) struct Svd<T> {
    pub(crate) u: Mat<T>,
    pub(crate) s: Diag<T>,
    pub(crate) v: Mat<T>,
}

fn svd_of<T: ComplexField>(a: MatRef<'_, T>, thin: bool) -> Result<Svd<T>, SvdError> {
    let (m, n) = a.shape();
    let size = m.min(n);
    let mut u = Mat::zeros(m, if thin { size } else { m });
    let mut v = Mat::zeros(n, if thin { size } else { n });
    let mut s = Diag::zeros(size);
    let vectors = if thin {
        ComputeSvdVectors::Thin
    } else {
        ComputeSvdVectors::Full
    };
    let mut scratch = MemBuffer::new(svd::svd_scratch::<T>(
        m,
        n,
        vectors,
        vectors,
        PAR,
        Default::default(),
    ));
    svd::svd(
        a,
        s.as_mut(),
        Some(u.as_mut()),
        Some(v.as_mut()),
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    Ok(Svd { u, s, v })
}

/// The full SVD of `a`: U m × m, V n × n. faer's `svd`.
pub(crate) fn svd<T: ComplexField>(a: MatRef<'_, T>) -> Result<Svd<T>, SvdError> {
    svd_of(a, false)
}

/// The thin SVD of `a`: U m × min(m, n), V n × min(m, n). faer's `thin_svd`.
pub(crate) fn thin_svd<T: ComplexField>(a: MatRef<'_, T>) -> Result<Svd<T>, SvdError> {
    svd_of(a, true)
}

/// The singular values of `a` alone, nonincreasing. faer's `singular_values`.
pub(crate) fn singular_values<T: ComplexField>(a: MatRef<'_, T>) -> Result<Vec<T::Real>, SvdError> {
    let (m, n) = a.shape();
    let mut s = Diag::<T>::zeros(m.min(n));
    let mut scratch = MemBuffer::new(svd::svd_scratch::<T>(
        m,
        n,
        ComputeSvdVectors::No,
        ComputeSvdVectors::No,
        PAR,
        Default::default(),
    ));
    svd::svd(
        a,
        s.as_mut(),
        None,
        None,
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    Ok(s.column_vector()
        .iter()
        .map(faer::traits::math_utils::real)
        .collect())
}

/// An eigendecomposition A U = U S, as faer's `Eigen`: S's eigenvalues, U's columns their right
/// eigenvectors.
pub(crate) struct Eigen {
    pub(crate) s: Diag<c64>,
    pub(crate) u: Mat<c64>,
}

/// The eigenvalues and right eigenvectors of the square complex `a`. faer's `eigen`.
pub(crate) fn eigen(a: MatRef<'_, c64>) -> Result<Eigen, EvdError> {
    let n = a.nrows();
    assert_eq!(a.ncols(), n, "a square matrix");
    let mut u = Mat::zeros(n, n);
    let mut s = Diag::zeros(n);
    let mut scratch = MemBuffer::new(evd::evd_scratch::<c64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        PAR,
        Default::default(),
    ));
    evd::evd_cplx::<f64>(
        a,
        s.as_mut(),
        None,
        Some(u.as_mut()),
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    Ok(Eigen { s, u })
}

/// The eigenvalues and right eigenvectors of the square real `a`, in complex numbers as faer's
/// `eigen` gives them: a complex pair's vectors are u ± i w from the real solver's two columns.
pub(crate) fn eigen_of_real(a: MatRef<'_, f64>) -> Result<Eigen, EvdError> {
    let n = a.nrows();
    assert_eq!(a.ncols(), n, "a square matrix");
    let mut real_vectors = Mat::<f64>::zeros(n, n);
    let mut re = Diag::<f64>::zeros(n);
    let mut im = Diag::<f64>::zeros(n);
    let mut scratch = MemBuffer::new(evd::evd_scratch::<f64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        PAR,
        Default::default(),
    ));
    evd::evd_real::<f64>(
        a,
        re.as_mut(),
        im.as_mut(),
        None,
        Some(real_vectors.as_mut()),
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    let (re, im) = (re.column_vector(), im.column_vector());
    let mut u = Mat::<c64>::zeros(n, n);
    let mut s = Diag::<c64>::zeros(n);
    let mut j = 0;
    while j < n {
        if im[j] == 0.0 {
            s[j] = c64::new(re[j], 0.0);
            for i in 0..n {
                u[(i, j)] = c64::new(real_vectors[(i, j)], 0.0);
            }
            j += 1;
        } else {
            s[j] = c64::new(re[j], im[j]);
            s[j + 1] = c64::new(re[j], -im[j]);
            for i in 0..n {
                let (x, y) = (real_vectors[(i, j)], real_vectors[(i, j + 1)]);
                u[(i, j)] = c64::new(x, y);
                u[(i, j + 1)] = c64::new(x, -y);
            }
            j += 2;
        }
    }
    Ok(Eigen { s, u })
}

/// A generalized eigendecomposition A U = B U diag(α / β), as faer's `GeneralizedEigen`: the
/// eigenvalues as pairs (α, β), an infinite one with β = 0, U's columns the right eigenvectors.
pub(crate) struct GeneralizedEigen {
    pub(crate) s_a: Diag<c64>,
    pub(crate) s_b: Diag<c64>,
    pub(crate) u: Mat<c64>,
}

/// The generalized eigenvalues and right eigenvectors of the square complex pencil (`a`, `b`),
/// by QZ. faer's `generalized_eigen`.
pub(crate) fn generalized_eigen(
    a: MatRef<'_, c64>,
    b: MatRef<'_, c64>,
) -> Result<GeneralizedEigen, GevdError> {
    let n = a.nrows();
    assert!(
        a.ncols() == n && b.nrows() == n && b.ncols() == n,
        "two square matrices of one size"
    );
    let (mut a, mut b) = (a.to_owned(), b.to_owned());
    let mut u = Mat::zeros(n, n);
    let mut s_a = Diag::zeros(n);
    let mut s_b = Diag::zeros(n);
    let mut scratch = MemBuffer::new(gevd::gevd_scratch::<c64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        PAR,
        Default::default(),
    ));
    gevd::gevd_cplx(
        a.as_mut(),
        b.as_mut(),
        s_a.as_mut(),
        s_b.as_mut(),
        None,
        Some(u.as_mut()),
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    Ok(GeneralizedEigen { s_a, s_b, u })
}

/// The parallelism of the sparse LU: [`SparseLu`].
pub(crate) const SPARSE_PAR: Par = Par::Seq;

/// faer's sparse LU with partial pivoting (its `sp_lu`), on [`SPARSE_PAR`] instead of faer's
/// global parallelism: faer's supernodal factorization takes another path on rayon's threads
/// than sequentially, and other bits on each number of them (its panels' LU).
pub(crate) struct SparseLu {
    symbolic: Arc<lu::SymbolicLu<usize>>,
    numeric: lu::NumericLu<usize, c64>,
}

impl SparseLu {
    /// The symbolic analysis of a pattern, to factorize several matrices of it with
    /// [`SparseLu::with_symbolic`]. faer's `SymbolicLu::try_new`.
    pub(crate) fn analyse(
        pattern: SymbolicSparseColMatRef<'_, usize>,
    ) -> Result<Arc<lu::SymbolicLu<usize>>, FaerError> {
        Ok(Arc::new(lu::factorize_symbolic_lu(
            pattern,
            Default::default(),
        )?))
    }

    /// The LU of `a`. faer's `sp_lu`.
    pub(crate) fn new(a: SparseColMatRef<'_, usize, c64>) -> Result<SparseLu, LuError> {
        SparseLu::with_symbolic(SparseLu::analyse(a.symbolic())?, a)
    }

    /// The LU of `a`, whose pattern `symbolic` analysed. faer's `Lu::try_new_with_symbolic`.
    pub(crate) fn with_symbolic(
        symbolic: Arc<lu::SymbolicLu<usize>>,
        a: SparseColMatRef<'_, usize, c64>,
    ) -> Result<SparseLu, LuError> {
        let mut numeric = lu::NumericLu::new();
        let mut scratch = MemBuffer::try_new(
            symbolic.factorize_numeric_lu_scratch::<c64>(SPARSE_PAR, Default::default()),
        )?;
        symbolic.factorize_numeric_lu(
            &mut numeric,
            a,
            SPARSE_PAR,
            MemStack::new(&mut scratch),
            Default::default(),
        )?;
        Ok(SparseLu { symbolic, numeric })
    }

    /// B ← A⁻¹ B.
    pub(crate) fn solve_in_place(&self, rhs: MatMut<'_, c64>) {
        let mut scratch = MemBuffer::new(
            self.symbolic
                .solve_in_place_scratch::<c64>(rhs.ncols(), SPARSE_PAR),
        );
        lu::LuRef::new_unchecked(&self.symbolic, &self.numeric).solve_in_place_with_conj(
            Conj::No,
            rhs,
            SPARSE_PAR,
            MemStack::new(&mut scratch),
        );
    }

    /// B ← A⁻ᵀ B.
    pub(crate) fn solve_transpose_in_place(&self, rhs: MatMut<'_, c64>) {
        let mut scratch = MemBuffer::new(
            self.symbolic
                .solve_transpose_in_place_scratch::<c64>(rhs.ncols(), SPARSE_PAR),
        );
        lu::LuRef::new_unchecked(&self.symbolic, &self.numeric).solve_transpose_in_place_with_conj(
            Conj::No,
            rhs,
            SPARSE_PAR,
            MemStack::new(&mut scratch),
        );
    }

    /// A⁻¹ B, B unchanged.
    pub(crate) fn solve(&self, rhs: MatRef<'_, c64>) -> Mat<c64> {
        let mut x = rhs.to_owned();
        self.solve_in_place(x.as_mut());
        x
    }

    /// A⁻ᵀ B, B unchanged.
    pub(crate) fn solve_transpose(&self, rhs: MatRef<'_, c64>) -> Mat<c64> {
        let mut x = rhs.to_owned();
        self.solve_transpose_in_place(x.as_mut());
        x
    }
}

/// The eigenvalues alone of the square complex `a`. faer's `eigenvalues`.
pub(crate) fn eigenvalues(a: MatRef<'_, c64>) -> Result<Vec<c64>, EvdError> {
    let n = a.nrows();
    assert_eq!(a.ncols(), n, "a square matrix");
    let mut s = Diag::<c64>::zeros(n);
    let mut scratch = MemBuffer::new(evd::evd_scratch::<c64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::No,
        PAR,
        Default::default(),
    ));
    evd::evd_cplx::<f64>(
        a,
        s.as_mut(),
        None,
        None,
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    Ok(s.column_vector().iter().copied().collect())
}

/// The eigenvalues alone of the square real `a`, complex pairs one after the other. faer's
/// `eigenvalues`.
pub(crate) fn eigenvalues_of_real(a: MatRef<'_, f64>) -> Result<Vec<c64>, EvdError> {
    let n = a.nrows();
    assert_eq!(a.ncols(), n, "a square matrix");
    let mut re = Diag::<f64>::zeros(n);
    let mut im = Diag::<f64>::zeros(n);
    let mut scratch = MemBuffer::new(evd::evd_scratch::<f64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::No,
        PAR,
        Default::default(),
    ));
    evd::evd_real::<f64>(
        a,
        re.as_mut(),
        im.as_mut(),
        None,
        None,
        PAR,
        MemStack::new(&mut scratch),
        Default::default(),
    )?;
    Ok(re
        .column_vector()
        .iter()
        .zip(im.column_vector().iter())
        .map(|(&x, &y)| c64::new(x, y))
        .collect())
}

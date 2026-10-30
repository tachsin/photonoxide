//! Dense kernels for the multifrontal fronts (#186): the products, triangular solves and LU
//! that photonoxide's own sparse solver spends its time in, as a trait another library can
//! stand behind.
//!
//! photonoxide's multifrontal solver eliminates each front with four dense
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

pub use crate::dense::{Block, BlockMut, DenseKernels, Faer, Kernels, Product};

use super::{Capabilities, Choice, DirectSolver, Threads};
use crate::Result;

/// The dense kernels a backend's choice stands for: faer's for `auto` and `photonoxide`, and
/// for a backend's name the kernels of photonoxide's solver it is (`photonoxide-mkl`'s are
/// oneMKL's).
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a backend that isn't registered or available, or that
/// has no dense kernels of its own (`pardiso`, `faer`).
pub fn kernels(choice: &Choice) -> Result<Arc<dyn DenseKernels>> {
    match choice {
        Choice::Auto | Choice::Photonoxide => Ok(Arc::new(Faer)),
        Choice::Named(name) => super::direct(choice)?.dense_kernels().ok_or_else(|| {
            crate::Error::invalid(
                "dense kernels",
                format!("the backend {name} has no dense kernels of its own"),
            )
        }),
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
    use num_complex::Complex64 as c64;

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
    fn a_choice_has_the_kernels_of_the_solver_it_names() {
        for own in [Choice::Auto, Choice::Photonoxide] {
            assert_eq!(kernels(&own).unwrap().kernels().name, "faer");
        }
        crate::backend::register(with_kernels(Arc::new(Plain))).unwrap();
        let named = kernels(&Choice::Named("photonoxide-plain".into())).unwrap();
        assert_eq!(named.kernels().name, "plain");
        // faer's sparse LU is a solver without dense kernels of its own to give
        let e = kernels(&Choice::Named("faer".into())).err().unwrap();
        assert!(e.to_string().contains("no dense kernels"), "{e}");
        assert!(kernels(&Choice::Named("no-such-backend".into())).is_err());
        // and the solvers say theirs
        let own = direct(&Choice::Photonoxide).unwrap();
        assert_eq!(own.dense_kernels().unwrap().kernels().name, "faer");
        assert!(Faer.kernels().deterministic && !Plain.kernels().deterministic);
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

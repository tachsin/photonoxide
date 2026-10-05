//! The bytes the iterative solvers' kernels move: what [`crate::bench`] divides by a phase's time
//! for the memory bandwidth a solve reached, against the machine's measured roof (S. Williams,
//! A. Waterman, D. Patterson, Commun. ACM 52(4), 65 (2009), doi:10.1145/1498765.1498785).
//!
//! The model counts what a kernel must read and write at least once: each stored nonzero's value
//! (16 bytes, complex) and column index (8), each row pointer (8), each vector it reads and each
//! it writes (16 bytes a value; one read and written, twice). A gathered vector, x in A x, is
//! counted once, as if it stayed in cache, and nothing is counted for the factorizations' setup.
//! The count is the kernels' traffic from memory or from cache: a problem whose vectors fit in
//! the last-level cache (the 40³ guide's, 3 MB each in a 30 MB L3) moves them faster than memory
//! does, and its bandwidth can exceed the triad's. Each kernel adds its bytes once a call, from
//! all threads into one counter: it counts every solve in the process, which a benchmark runs
//! one at a time.

use std::sync::atomic::{AtomicU64, Ordering};

static MOVED: AtomicU64 = AtomicU64::new(0);

/// Adds a kernel's bytes.
pub(crate) fn add(bytes: usize) {
    MOVED.fetch_add(bytes as u64, Ordering::Relaxed);
}

/// The bytes moved so far in this process.
pub(crate) fn moved() -> u64 {
    MOVED.load(Ordering::Relaxed)
}

/// A complex value.
const VALUE: usize = 16;
/// A stored index.
const INDEX: usize = 8;

/// A sparse product by rows, y = A x: A's values and columns, its row pointers, x read once and
/// y written.
pub(crate) fn product(rows: usize, columns: usize, nonzeros: usize) -> usize {
    nonzeros * (VALUE + INDEX) + (rows + 1) * INDEX + columns * VALUE + rows * VALUE
}

/// A triangular solve in place by rows, with or without a stored inverse diagonal: the factor's
/// off-diagonal values and columns, its row pointers, the diagonal, and x read and written.
pub(crate) fn triangular(rows: usize, nonzeros: usize, diagonal: bool) -> usize {
    nonzeros * (VALUE + INDEX)
        + (rows + 1) * INDEX
        + if diagonal { rows * VALUE } else { 0 }
        + 2 * rows * VALUE
}

/// A pass over vectors of `n` values, reading `read` of them and writing `written`.
pub(crate) fn vectors(n: usize, read: usize, written: usize) -> usize {
    n * (read + written) * VALUE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_products_bytes_by_hand() {
        // a 3 x 3 matrix with 7 nonzeros: 7 values (112 B) and 7 columns (56 B), 4 row pointers
        // (32 B), x read (48 B) and y written (48 B)
        assert_eq!(product(3, 3, 7), 112 + 56 + 32 + 48 + 48);
        // a unit lower triangle with 3 off-diagonal entries on 4 rows: 48 + 24 + 40 + 128
        assert_eq!(triangular(4, 3, false), 48 + 24 + 40 + 128);
        // with an inverse diagonal: 64 more
        assert_eq!(triangular(4, 3, true), 48 + 24 + 40 + 64 + 128);
        assert_eq!(vectors(10, 3, 1), 640);
    }
}

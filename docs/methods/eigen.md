---
title: "Shift-and-invert Arnoldi"
module: mode::vector
summary: "The eigenvalues of a large sparse matrix nearest a shift, for the mode solvers: Arnoldi on (A − σI)⁻¹, restarted, with every answer checked against A itself."
order: 6
papers:
  - cite: "Y. Saad, Numerical Methods for Large Eigenvalue Problems, 2nd ed., SIAM (2011)"
    doi: 10.1137/1.9781611970739
---

A mode solver wants a few eigenvalues of a matrix with millions of rows: the modes nearest an
expected effective index. photonoxide's eigensolver is Saad's, from his book (free from the
author's site), in pure Rust on faer's sparse LU.

## The method

1. **Shift and invert** (Section 8.1.3): the eigenvalues of $(A - \sigma I)^{-1}$ are
   $1/(\lambda - \sigma)$, so those of A nearest the shift σ become the largest, and converge
   first. σ is $k_0^2 n^2$ for the effective index n wanted: by default the highest index in the
   cross-section, which finds the fundamental modes.
2. **Arnoldi's method** with modified Gram–Schmidt (Algorithm 6.2), and a second
   orthogonalization where cancellation is severe (Section 6.2.2). Each step solves one system
   with the LU factors of $A - \sigma I$, computed once.
3. **Restarts** from the wanted Ritz vectors (Algorithm 6.3, several combined) until they
   converge.
4. A Ritz pair is accepted only when its **true residual** is small, computed with A itself:

$$
\frac{\lVert A x - \lambda x \rVert}{|\lambda|\, \lVert x \rVert} < \text{tol}.
$$

So an answer the iteration only believes converged is never returned.

## Validation

Its answers are the mode solver's: a uniform box's exact discrete eigenvalues to 1e-9, and every
[full-vector](vector.md) validation case.

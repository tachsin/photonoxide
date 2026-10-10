---
title: "Shift-and-invert Arnoldi, with Krylov–Schur restarts"
module: mode::vector
summary: "The eigenvalues of a large sparse matrix nearest a shift, for the mode solvers: Arnoldi on (A − σI)⁻¹, restarted by Stewart's Krylov–Schur method in a space of fixed size, with every answer checked against A itself."
order: 6
papers:
  - cite: "Y. Saad, Numerical Methods for Large Eigenvalue Problems, 2nd ed., SIAM (2011) (Arnoldi, shift and invert)"
    doi: 10.1137/1.9781611970739
  - cite: "G. W. Stewart, SIAM J. Matrix Anal. Appl. 23, 601 (2002), online 2001 (Krylov–Schur restarts)"
    doi: 10.1137/S0895479800371529
  - cite: "Z. Bai, J. W. Demmel, Linear Algebra Appl. 186, 73 (1993) (exchanging eigenvalues in a Schur form)"
    doi: 10.1016/0024-3795(93)90286-W
  - cite: "G. H. Golub, C. F. Van Loan, Matrix Computations, 4th ed., Johns Hopkins University Press (2013, issued 2012) (Hessenberg reduction and the shifted QR algorithm)"
    doi: 10.56021/9781421407944
validation:
  - mode/krylov-schur-box
  - mode/krylov-schur-dense
  - mode/krylov-schur-growing
  - mode/krylov-schur-chilwell
  - mode/krylov-schur-convergence
  - mode/krylov-schur-threads
---

A mode solver wants a few eigenvalues of a matrix with millions of rows: the modes nearest an
expected effective index. photonoxide's eigensolver is Saad's Arnoldi method, restarted by
Stewart's Krylov–Schur method, in pure Rust on faer's sparse LU.

## The method

1. **Shift and invert** (Saad, Section 8.1.3): the eigenvalues of $(A - \sigma I)^{-1}$ are
   $1/(\lambda - \sigma)$, so those of A nearest the shift σ become the largest, and converge
   first. σ is $k_0^2 n^2$ for the effective index n wanted: by default the highest index in the
   cross-section, which finds the fundamental modes.
2. **Arnoldi's method** with modified Gram–Schmidt (Saad, Algorithm 6.2), and a second
   orthogonalization where cancellation is severe (Section 6.2.2). Each step solves one system
   with the LU factors of $A - \sigma I$, computed once.
3. **Krylov–Schur restarts** (Stewart, Section 3). The process grows a Krylov decomposition
   $(A - \sigma I)^{-1} U_m = U_m S_m + u\thinspace b^H$ to m vectors (his (3.1)); its Rayleigh
   quotient $S_m$ is brought to Schur form $Q^H S_m Q = T$ (3.3), by Householder reduction to
   Hessenberg form and the shifted QR algorithm (Golub and Van Loan, Sections 7.4 and 7.5, in
   complex arithmetic); the wanted Ritz values, the largest, are moved to T's leading corner by
   exchanging neighbours, each exchange one rotation (Bai and Demmel); and the decomposition is
   truncated to the first k (3.4): $U_k = U_m Q_{:,1:k}$, $S_k = T_{1:k,1:k}$, $b_k = Q_{:,1:k}^H b$.
   That is again a Krylov decomposition (his Definition 2.1, Theorem 2.2), which grows again
   from k to m.
4. A Ritz pair $(\theta, U_m Q y)$ of $(A - \sigma I)^{-1}$ has residual $(b^H Q y)\thinspace u$, so
   as an eigenpair of A, $\lambda = \sigma + 1/\theta$, its residual is
   $-(b^H Q y/\theta)(A - \sigma I)u$, known without forming the vector. When every wanted
   pair's is small, each is accepted only when its **true residual** is small too, computed
   with A itself:

$$
\frac{\lVert A x - \lambda x \rVert}{\max(|\lambda|, 1)\thinspace\lVert x \rVert} \lt \text{tol}.
$$

So an answer the iteration only believes converged is never returned.

**The sizes.** For c modes the space holds $m = \max(2c + 20, 40)$ vectors and a restart keeps
$k = c + (m - c)/2$: the wanted and half the rest, so each growth takes $(m - c)/2$ steps. The
space never holds more than m + 1 vectors of the grid's size however many restarts it takes.
Until 0.5.1 a restart began again from the wanted Ritz vectors summed into one, in a space one
first run larger than the last, because a space of one size stalled on many modes (six, on a
strip's cross-section, whose cladding modes lie close together); Krylov–Schur keeps every wanted
direction found so far, and a double eigenvalue (a uniform box's $H_x$ and $H_y$ modes) comes out
twice.

**The same bits on any number of threads.** The Arnoldi steps, the Schur form and its
reordering, and the restart's product $U_m Q_{:,1:k}$ (faer's on one thread, 512 rows at a time)
run in a fixed order.

## Many modes, measured

The book's strip (500 × 220 nm silicon in oxide, 1.55 µm), the modes nearest its highest index,
on one machine (20 threads, though the solves are one thread's work); "vectors" are the most
vectors of the grid's size held at once, the process's peak memory from Windows' peak working
set:

| grid | unknowns | modes | Krylov–Schur | growing restarts (0.5.1) |
|---|---|---|---|---|
| 10 nm | 63 722 | 20 | 4.8 s, 61 vectors (62 MB) | 13.5 s, 181 vectors (185 MB) |
| 10 nm | 63 722 | 50 | 11.4 s, 121 vectors (123 MB) | 15.5 s, 241 vectors (246 MB) |
| 5 nm | 253 442 | 20 | 25.4 s, 61 vectors (247 MB); peak 1.37 GB | 77.3 s, 181 vectors (734 MB); peak 1.55 GB |
| 5 nm | 253 442 | 50 | 48.8 s, 121 vectors (491 MB); peak 1.37 GB | 142.9 s, 241 vectors (977 MB); peak 2.01 GB |

The peak of 1.37 GB is the sparse LU's, while it factorizes; the vectors come after. For 2 to 8
modes the two take the same time. The eigenvalues agree to 3.8e-12 relative (20 nm, 50 modes);
at 20 modes the 20th of each is one of a complex-conjugate pair of evanescent modes, equally near
the shift, and the two pick different ones.

## Validation

- **Analytic:** a uniform box's 50 modes nearest its index (ε = 2.25, 2 × 1.4 µm on 20 × 14
  cells), its exact discrete eigenvalues, 25 of them each twice, to 7.4e-14 relative, in a
  space of 120 with restarts. The 2D Laplacian's 20 eigenvalues nearest 0 on 40 × 30 nodes in a
  space of only 30 (11 growths, 31 vectors held), each to 1.4e-15.
- **Against a dense solve:** the leaky four-layer guide (2 µm of PML, 10 nm grid, 901
  unknowns), the 30 eigenvalues nearest $n_\text{eff} = 1.3 + 0.03i$: the leaky waves m = 4 … 7
  among them to 1.1e-11; the PML's own modes, ill-conditioned (the matrix far from normal), each
  to its residual times its condition, up to 1.4e-8.
- **Against the restarts it replaced:** the strip's 50 modes (20 nm), the same to 3.7e-12
  relative.
- **Published:** Chilwell and Hodgkinson's four-layer guide (1 nm grid), 20 modes nearest its
  highest index, TE and TM, the 4 bound modes of each within 5.5e-7 of their Table 3.
- **Convergence:** on the Laplacian, the wanted pairs' residual falls from 0.95 to 1.4e-14 over
  the growths, and each Ritz value's error with the square of its residual (fitted slope 1.9 for
  residuals from 1e-2 to 1e-8): a Ritz value of a Hermitian matrix errs by about its residual
  squared over its gap (Parlett, *The Symmetric Eigenvalue Problem*, SIAM (1998),
  doi:10.1137/1.9781611971163), which the restarts keep.
- **Threads:** the strip's 20 modes on 1 and 4 threads, every effective index and field entry
  the same bits.

Its answers are the mode solver's too: every [full-vector](vector.md) validation case.

## Not yet

Locking converged pairs (Stewart, Section 5), which would spare their part of the Schur form
each restart; and the real Schur form for a real matrix (his Section 7), which would halve the
arithmetic of a lossless guide.

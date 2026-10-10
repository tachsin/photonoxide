---
title: "Ports as a block: block QMR"
module: fdfd
summary: "All of a 3D S-matrix's ports solved at once by block QMR: one block Krylov space for every port's source, its Lanczos vectors built a vector at a time with deflation of dependent vectors and systems, in a general and a complex symmetric form, with the existing operators and preconditioners. Opt-in: faster with ILU(0), slower without a preconditioner."
order: 19.5
papers:
  - cite: "R. W. Freund, M. Malhotra, Linear Algebra Appl. 254, 119 (1997) (block QMR, its Lanczos-type process for many starting vectors, deflation)"
    doi: 10.1016/S0024-3795(96)00529-0
  - cite: "P. Jolivet, P.-H. Tournier, SC16, 190 (2016) (block methods for Maxwell problems with many sources; a block of vectors read against the matrix once)"
    doi: 10.1109/SC.2016.16
  - cite: "R. W. Freund, N. M. Nachtigal, Numer. Math. 60, 315 (1991) (QMR, one right-hand side)"
    doi: 10.1007/BF01385726
  - cite: "R. W. Freund, SIAM J. Sci. Stat. Comput. 13, 425 (1992) (QMR for complex symmetric matrices)"
    doi: 10.1137/0913023
validation:
  - fdfd3d/block-qmr-s-matrix
  - fdfd3d/block-qmr-ilu-s-matrix
  - fdfd3d/block-qmr-iterations
  - fdfd3d/block-qmr-dependent
  - fdfd3d/block-qmr-convergence
  - fdfd3d/block-qmr-freund-malhotra
  - fdfd3d/block-qmr-threads
---

A 3D S-matrix takes one solve per port: every port's mode source on the same matrix. QMR
([FDFD in 3D](fdfd-3d.md)) builds a Krylov space for each from scratch. Block QMR builds one
block Krylov space for all of them and draws every port's field from it:

```rust
use photonoxide::fdfd::{IterativeSolver3d, Stopping};

let solver = IterativeSolver3d::new(grid, wavelength, eps, boundaries, formulation)?
    .with_ilu()?
    .with_block(true);
let s = solver.s_matrix(&ports, Stopping::default())?;
// or, with how the block went
let (s, how) = solver.s_matrix_block(&ports, Stopping::default())?;
// any right-hand sides
let (fields, how) = solver.solve_systems(&sources, Stopping::default())?;
```

It is **opt-in**. With ILU(0) it is faster than one solve per port; without a preconditioner,
and with the multigrid, it is slower (the measurements below).

## The method

R. W. Freund and M. Malhotra, "A block QMR algorithm for non-Hermitian linear systems with
multiple right-hand sides", Linear Algebra Appl. 254, 119 (1997),
[doi:10.1016/S0024-3795(96)00529-0](https://doi.org/10.1016/S0024-3795(96)00529-0). The
implementation is `fdfd::krylov::block`.

**Lanczos vectors for many starting vectors** (their Algorithm 3.1, without look-ahead). Right
vectors $v_1, v_2, \ldots$ span the block Krylov space of $A$ and the right-hand sides $B$; left
ones $w_1, w_2, \ldots$ that of $A^\mathsf{T}$ and a left block $L$. They are biorthogonal,
$w_j^\mathsf{T} v_k = 0$ for $j \ne k$ and $\delta_k = w_k^\mathsf{T} v_k \ne 0$. They are built
**a vector at a time**: the next candidate is a column of $B$ while any is left, then
$A v_\mu$ for the oldest vector whose product hasn't been taken. Each candidate is
biorthogonalized against the earlier vectors its history index says it can meet (their Eqs.
3.3 to 3.6; $A v_\mu$ meets $w_i$ from the index whose product made $w_\mu$), then normalized.
$L = B$, as single QMR takes $w_1 = v_1$.

The coefficients are taken **in the order of modified Gram–Schmidt**, as their Remark 3.2
advises: each from the candidate as the earlier subtractions left it, one pass over the vectors
a term (the subtraction of one term and the next coefficient fused). Found together instead,
from the candidate as it came, the vectors lost biorthogonality to their neighbours
exponentially, from 1e-16 to 1e-5 in 30 steps on a nonsymmetric test matrix, and the block
stalled.

**Deflation** (their steps 1d and 2d). A candidate whose biorthogonalized part is at most
$10^{-6}$ of its norm before is dropped: their $d_\text{tol}$, as in their examples, taken
relative to the candidate since $A$'s scale is $k_0^2\varepsilon$. Much smaller misses
dependence that rounding blurs, and a vector kept from that noise stalls the iteration (their
Figs. 1 and 4). A product dropped with something left over joins the set 𝓘 against which the
other sequence's later candidates are biorthogonalized.

- A **column of $B$ dropped** is a right-hand side that is a combination of the earlier ones,
  $b = \sum_k \gamma_k b_k$ (the coefficients from the triangle of the kept ones'): its system
  leaves the block and is recovered as $x = \sum_k \gamma_k x_k$ (their Eq. 4.18).
- A **product dropped** leaves fewer quasi-residuals than systems. A combination $\gamma$ of the
  systems then has none: the null vector of the rotated right-hand side's last rows. The system
  that weighs most in it leaves the block, and is recovered at the end from the combination's
  iterate and the others' solutions (their Eqs. 4.16 to 4.20).
- The systems a dropped one is recovered from have their tolerances tightened by $|\gamma|$,
  so that the recovered residual $\sum \gamma_k r_k$ meets its own.
- A system whose **true residual** reaches its tolerance is frozen and leaves the block.

**Block quasi-minimal residuals** (their Algorithm 4.2 and Section 5). With $X = V Z$, the
residual is $V([\rho; 0] - T Z)$ up to the deflated vectors, $\rho$ the right-hand sides'
coefficients and $T$ the band of recurrence coefficients (their Eq. 3.14). $Z$ minimizes
$\lVert [\rho; 0] - T Z\rVert$, by Givens rotations as each column of $T$ comes: the earlier
columns' rotations, then new ones zeroing it below its diagonal (their Eqs. 5.1 to 5.6). The
rotated right-hand side's row $\mu$ is the step $y_\mu$, and $X$ moves by one rank-one update,
$x_j \mathrel{+}= p_\mu y_{\mu j}$, through directions $p_\mu = (v_\mu - \sum_i p_i
u_{i\mu})/u_{\mu\mu}$ over at most $2m$ earlier ones (their Eqs. 5.9 and 5.10). The rows below
$\mu$ are the quasi-residuals; when a system's reaches its tolerance its true residual is
computed, and it stops only on that.

**The complex symmetric form** (their Section 6 with $J = I$). For $A = A^\mathsf{T}$ and
$L = B$ the left vectors are the right ones: neither they nor $A^\mathsf{T}$'s products are
taken, one product an iteration. On a symmetric matrix the general form gives the same bits
(a test). The curl-curl operator is used through its complex symmetric similarity
$S A S^{-1}$ (as single QMR is): the block solves $B Y = S\,B$, and each system stops on the
residual of $A X = B$ itself, $\lVert S^{-1}(S b - B y)\rVert \le \text{tol}\,\lVert b\rVert$.

**Operators and preconditioners.** The assembled matrix and the matrix-free operator, and
ILU(0) and the multigrid cycle from the right ($A M^{-1}$, the general form). The products a
block needs next are all known once a vector is built (their Section 8): $A v_\mu, \ldots,
A v_n$. They are taken together, the matrix's rows read once for them all and ILU(0)'s
triangular solves made row by row for all of them, as P. Jolivet and P.-H. Tournier do for
Maxwell problems with many sources (SC16, 190 (2016),
[doi:10.1109/SC.2016.16](https://doi.org/10.1109/SC.2016.16), Section V-B). Each vector's sums
are taken in the order they are alone: the same bits. The matrix-free operator and the cycle
take a block's vectors one by one. With an iterative backend, which runs single QMR, the ports
go one at a time through it.

**Breakdowns** ($|\delta_n| < 10^{-14}$; look-ahead isn't implemented) and recovered systems
that fall short of their tolerance start the block again from the iterates reached, on their
true residuals, up to 10 times.

**The same bits on any number of threads.** Every vector operation is a pass over fixed chunks,
each chunk's sums in order and the chunks' sums in order; the small dense work (rotations,
null vectors) is sequential.

## Validation

- **The single solves' S-matrices**: a directional coupler's straight section (two silicon
  strips 350 × 220 nm, 150 nm apart, in oxide; 18 × 32 × 24 cells of 50 nm, PMLs of 4), its
  8 ports (each end's four supermodes) to a relative residual of 1e-10. Block against one QMR
  solve each: 1.7e-11 on the curl-curl operator, with its matrix and without; 6.6e-11 on Shin
  and Fan's operator with ILU(0); 1.1e-10 with the multigrid (against GMRES)
  (`fdfd3d/block-qmr-s-matrix`, `fdfd3d/block-qmr-ilu-s-matrix`).
- **Analytic**: a diagonal system with 6 distinct eigenvalues and 3 right-hand sides, solved to
  3e-16 when its block Krylov space is whole at 18 vectors, its last three products deflated
  (a test). On the coupler's own operator (41 472 unknowns), right-hand sides $b_1$, $b_2$,
  $b_1 + 2b_2$ and $A b_1$: the third leaves the block at once and is recovered as
  $x_1 + 2x_2$, the fourth makes the first product dependent and $A x = A b_1$ gives $b_1$, both
  to 6.2e-15, in the general and the symmetric form (`fdfd3d/block-qmr-dependent`).
- **Convergence**: a closed guide's S-matrix (a strip stepping from 300 to 400 nm wide in a box of
  oxide, 20 × 12 × 10 cells of 50 nm, 4 ports) against the direct solver's falls with the
  tolerance, 1.2e-5, 1.7e-7, 2.3e-9 and 1.9e-11 at 1e-4, 1e-6, 1e-8 and 1e-10: a slope of 0.97
  (`fdfd3d/block-qmr-convergence`). The iterations against the single solves' summed: 0.44 of
  them with ILU(0) on the coupler's 8 ports, 0.93 on the curl-curl operator without a
  preconditioner (`fdfd3d/block-qmr-iterations`).
- **Published**: Freund and Malhotra's Example 7.1, a convection–diffusion equation on the unit
  cube ($15^3$ interior points, two-sided SSOR, random right-hand sides and left block). Theirs:
  19 iterations for one right-hand side and 85 for five, a ratio of 4.47; here 14 and 63 at a
  relative residual of 1e-6 (medians over 5 draws), a ratio of 4.5 (`fdfd3d/block-qmr-freund-malhotra`).
  Their stopping test, and part of their Eq. 7.4, aren't legible in our scanned copy: at 1e-9
  the counts are 18 and 85. Their other examples are finite-element acoustics models whose
  matrices aren't published, and Jolivet and Tournier's are 89-million-unknown finite-element
  Maxwell problems: not reproduced.
- **Threads**: the coupler's block S-matrix on 1 and 4 threads, the same bits and the same
  convergence (`fdfd3d/block-qmr-threads`).

## Measured

The coupler above at 40 nm: 1.0 × 1.2 × 0.8 µm inside PMLs of 6 cells all round, 37 × 42 × 32
cells (149 184 unknowns), 4 ports (each end's two modes of highest β) or 8 (four each), to a
relative residual of 1e-6. An Intel Core Ultra 7 265K (20 cores, 64 GB), 8 threads, shared with
other work while it ran: each time is the faster of two runs, single and block alternately
(`cargo test --release fdfd::three::block_checks::tests::measure -- --ignored --nocapture`).

| Operator | Ports | One at a time | Block | Block's iterations / single's | Block's time / single's |
|---|---|---|---|---|---|
| curl-curl, stored | 4 | 4.41 s, 3 056 it. | 12.55 s, 3 201 it. | 1.05 | 2.8 |
| curl-curl, stored | 8 | 9.74 s, 6 234 it. | 33.51 s, 5 928 it. | 0.95 | 3.4 |
| curl-curl, no matrix | 8 | 5.97 s, 6 234 it. | 31.78 s, 5 972 it. | 0.96 | 5.3 |
| Shin and Fan, ILU(0) | 4 | 18.21 s, 1 535 it. | 12.53 s, 1 163 it. | 0.76 | **0.69** |
| Shin and Fan, ILU(0) | 8 | 36.21 s, 3 046 it. | 25.10 s, 1 856 it. | 0.61 | **0.69** |
| Shin and Fan, multigrid | 8 | 4.22 s, 198 it. (GMRES) | 9.36 s, 236 it. (2 cycles each) | 1.19 | 2.2 |

The S-matrices agree to the tolerance (2.5e-7 to 1.7e-6). With ILU(0), block QMR is 1.45
times faster, 4 ports or 8: fewer iterations, and each step's products and triangular solves
taken for the block at once (taken a vector at a time, the 8 ports' block took about as long
as their single solves: 90 s against 85 s, on a busier machine). Without a preconditioner the block is 2.8 to 5.3 times slower, and with the multigrid,
whose GMRES takes one cycle an iteration where QMR takes the cycle and its transpose, 2.2 times.
So it stays opt-in; the fastest of these remains the multigrid, one port at a time.

## Why the curl-curl operator gains so little

A sparse product carries a field one cell. A port's field must reach across the device, so
each source's Krylov space needs about as many products as the field has cells to cross,
whatever the other sources contribute: a block space of dimension $\mu$ holds polynomials of
degree only $\mu / m$ in $A$ applied to each source. Without a preconditioner the block takes
0.93 to 1.05 of the single solves' products, and each of its iterations does several times
their vector work: $2m$ to $4m$ passes of modified Gram–Schmidt, $2m$ earlier directions, $m$
iterates. A preconditioner that reaches further a step (ILU(0)'s triangular solves sweep the
whole grid) lets the sources share their spaces: 0.44 to 0.76 of the products, each step's
products and triangular solves taken for the whole block at once.

## Left for later

- Recycling Krylov subspaces across a wavelength sweep and between a forward solve and its
  adjoint (a separate item of 0.5.2).
- Their Remark 3.4: half the inner products, the rest known from biorthogonality.
- The matrix-free operator and the multigrid cycle on a block of vectors at once.
- Look-ahead.

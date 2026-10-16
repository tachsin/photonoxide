---
title: "The multigrid without its finest matrix"
module: fdfd
summary: "GMRES with the 3D multigrid on Shin and Fan's operator, the finest level's matrix and ILU(0) factors never assembled: GMRES's products and the finest level's residuals from the stencil, its smoothing by damped Jacobi with the stencil's diagonal (Erlangga, Oosterlee and Vuik's smoother), and the coarse operators either Galerkin's, made from the stencil's rows to the stored matrix's bits, or discretized again on each coarser grid. On the strip with ports at 20 nm (1.8 million unknowns) the peak memory falls from 9.5 GB to 6.0 or 3.3 GB, on the guide at 128³ (6.3 million) from 33.3 GB to 22.5 or 10.2 GB, and the whole solve is 1.1 to 1.5 times faster. Opt-in."
order: 19.7
papers:
  - cite: "Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci. Comput. 27, 1471 (2006) (the shifted operator, ω-Jacobi, Table 5's smoothing factors)"
    doi: 10.1137/040615195
  - cite: "B. Reps, W. Vanroose, H. bin Zubair, J. Comput. Phys. 229, 8384 (2010) (the multigrid with the matrix: ILU(0) smoothing, Galerkin coarse operators)"
    doi: 10.1016/j.jcp.2010.07.022
  - cite: "W. Shin, S. Fan, Opt. Express 21, 22578 (2013) (the operator, Eq. 7 with s = −1)"
    doi: 10.1364/OE.21.022578
  - cite: "R. Hiptmair, SIAM J. Numer. Anal. 36, 204 (1998) (the hybrid smoother, weighed)"
    doi: 10.1137/S0036142997326203
  - cite: "D. N. Arnold, R. S. Falk, R. Winther, Numer. Math. 85, 197 (2000) (patch smoothers, weighed)"
    doi: 10.1007/PL00005386
  - cite: "R. Hiptmair, J. Xu, SIAM J. Numer. Anal. 45, 2483 (2007) (auxiliary-space preconditioning, weighed)"
    doi: 10.1137/060660588
  - cite: "S. Cools, B. Reps, W. Vanroose, Numer. Linear Algebra Appl. 21, 513 (2013) (coarse grids discretized again for indefinite Helmholtz problems)"
    doi: 10.1002/nla.1895
validation:
  - fdfd3d/multigrid-stencil-direct
  - fdfd3d/multigrid-stencil-s-matrix
  - fdfd3d/multigrid-stencil-threads
  - fdfd3d/multigrid-stencil-refinement
  - fdfd3d/multigrid-stencil-plane-waves
  - fdfd3d/multigrid-stencil-smoothing-factors
---

GMRES with the multigrid ([FDFD in 3D](fdfd-3d.md#preconditioning-qmr)) is what solves the
large 3D problems: unpreconditioned QMR breaks down from about 2 million unknowns, with or
without its matrix. But the multigrid kept the matrix: GMRES multiplied by it, and the cycle's
finest level smoothed by its ILU(0) factors and made the first coarse operator as $P^{\mathsf T} A P$
from it. On the strip with ports at 20 nm (1.8 million unknowns) the hierarchy took 8.4 GB of
the 9.5 GB the solve peaked at:

| | Rows | Entries a row | Bytes |
|---|---|---|---|
| GMRES's matrix, Shin and Fan's operator | 1 806 624 | 14.5 | 1.29 GB |
| the finest level's, shifted | 1 806 624 | 14.5 | 1.29 GB |
| its ILU(0) factors | | | 1.29 GB |
| $P$ and $P^{\mathsf T}$ to the first coarse level | | 3.1 | 0.29 GB |
| the first coarse level, $P^{\mathsf T} A P$, and its ILU(0) | 480 816 | 64.6 | 3.00 GB |
| the coarser levels, all told | 218 970 | | 1.2 GB |

(`fdfd::three::multigrid::tests::sizes`; a matrix and its transpose, each entry a value and a
column, 48 bytes an entry.) The finest level's three are half of it, and the first coarse
level, four and a half times denser than the finest, another third.
`IterativeSolver3d::new_with(…, Formulation::ShinFan, Product::MatrixFree)` followed by
`with_multigrid` keeps none of the finest level's three, and with the coarse operators
discretized again the first coarse level has 15 entries a row in place of 65.

## Shin and Fan's operator from its stencil

The curl-curl product without the matrix ([FDFD in 3D](fdfd-3d.md#the-iterative-solver),
"Without the matrix") is two passes over the grid's rows, E to H and H to E. Shin and Fan's
operator adds $\nabla(\varepsilon^{-1}\nabla\cdot(\varepsilon\mathbf E))$ with $\varepsilon^{-1}$
at the nodes (their Eq. 7 with $s = -1$): a third pass takes the divergence of $\varepsilon\mathbf E$
at each node, over the step and the PML's stretch there, times $\varepsilon^{-1}$ there (zero at
a node on a wall behind a PML), and the second pass adds its gradient, over the step and the
stretch halfway, as `Lattice::assemble_with` has them. The multigrid's operator adds
Erlangga, Oosterlee and Vuik's shift $i\beta k_0^2\varepsilon$ to the diagonal.

**Its transpose without the matrix.** With $D$ the product of the stretches at a value of E and
$V$ at a node, the divergence is $-V^{-1}G^{\mathsf T}D$, $G$ the gradient: the stretches that
make $D(\nabla\times\nabla\times)$ symmetric do it for the divergence and the gradient too. So the
added term $K = G\varepsilon_n^{-1}\nabla\cdot\varepsilon$ has
$K^{\mathsf T} = (D\varepsilon)K(D\varepsilon)^{-1}$, and the whole operator's transpose is
$D\tilde A D^{-1}$, $\tilde A$ the same passes with $\varepsilon$ outside the gradient instead of
inside the divergence (a Bloch side's phases inverted, as for the curl-curl operator). No single
diagonal symmetrizes it ($D$ and $D\varepsilon$ differ), which is why the stored matrix wasn't
symmetrizable either. The transposed product is what the transposed cycle, QMR and block QMR
take.

Both the product and its transpose equal the assembled matrix's, with and without the shift, to
1e-13 and 1e-12 of the largest value, with plain and stretched PMLs, walls, PMLs of unequal
sides and Bloch sides on one, two and three axes (tests in `matrix_free.rs`), and are the same
bits on 1, 2, 4, 5 and 20 threads.

## The smoother: why a point smoother is enough here

The curl-curl operator's gradients are a near-null space no point smoother reduces: a
gradient's curl is zero, so the operator acts on it as $k_0^2\varepsilon$ alone, against
$1/h^2$ for everything else, and a Jacobi or Gauss–Seidel sweep leaves smooth and rough gradients
alike. The known remedies treat that space on its own:

- **Hiptmair's hybrid smoother** (SIAM J. Numer. Anal. 36, 204 (1998), Fig. 6.2): a
  Gauss–Seidel sweep on the edges, then one on the nodal potentials of the gradients (the edge
  operator's restriction to them, $G^{\mathsf T}AG$, a scalar Laplacian), carried back by $G$.
- **Arnold, Falk and Winther's patch smoothers** (Numer. Math. 85, 197 (2000)): overlapping
  Schwarz on the patches of edges around each vertex, which hold a gradient's support whole.
- **Auxiliary-space preconditioning** (Hiptmair and Xu, SIAM J. Numer. Anal. 45, 2483 (2007)):
  nodal multigrid on the gradients and on a vector-valued nodal space, around a point smoother.

All three are built for the definite form $(\nabla\times u, \nabla\times v) + (u, v)$; the
frequency-domain operator is indefinite. And the multigrid here doesn't cycle on the curl-curl
operator: it cycles on Shin and Fan's, whose added gradient of a divergence lifts the gradients
from $k_0^2\varepsilon$ to the vector Laplacian's eigenvalues (their Section 2; in a uniform
medium $\nabla\times\nabla\times - \nabla\nabla\cdot = -\Delta$, exactly on Yee's grid). That
is what let ILU(0) work at all. On it, the shifted operator is Erlangga, Oosterlee and Vuik's
setting, component by component in a uniform medium, and their smoother is a point smoother:

**Damped Jacobi,** $x \leftarrow x + \omega D^{-1}(b - Ax)$, $D$ the operator's diagonal, made
from the stencil's own rows (the same values the stored matrix has), $\omega = 0.8$, one sweep
before each coarse correction and one after. It is a diagonal: its transpose is itself, it
runs on every thread, and its sums are fixed by the grid.

**Its damping, against its symbol.** In a uniform periodic box each component's plane wave
$e^{i\theta\cdot n}$ is an eigenvector of the shifted operator, with eigenvalue
$\lambda = k_0^2(1+i\beta)\varepsilon - \sum_a 4\sin^2(\theta_a/2)/h_a^2$, and one sweep
multiplies it by $S = 1 - \omega\lambda/d$, $d = k_0^2(1+i\beta)\varepsilon - \sum_a 2/h_a^2$.
The smoother's sweep, from the stencil, gives that for every wave of every component of an
$8 \times 6 \times 10$ box of unequal steps to 7.3e-15 (`fdfd3d/multigrid-stencil-plane-waves`).
At the checkerboard $\lambda/d \to 2$: $\omega = 1$ leaves it undamped ($S = -1$), and GMRES
with $\omega = 1$ didn't converge in 2 000 iterations on the guide or Diel; $\omega = 0.9$ was
within a few iterations of 0.8, which keeps a margin.

**Erlangga, Oosterlee and Vuik's Table 5,** reproduced by this smoother: on a 2D slice (one cell
along z, periodic; walls on x and y; $k_0 = 40$ on the unit square), $E_z$ alone sees the 5-point
Laplacian with Dirichlet sides plus $k_0^2(1 + i\beta)$, the sine modes are the sweep's
eigenvectors, and their smoothing factor $\mu$ (two sweeps, the low frequencies projected out,
their Section 4.2) is the largest $\lvert S\rvert^2$ over the modes with ℓ or m at least N/2.
For their rows with $\beta_1 = 1$:

| $(\beta_1, \beta_2)$, ω | h = 1/64 | 1/32 | 1/16 | 1/8 |
|---|---|---|---|---|
| (1, 0), 0.7: theirs | 0.47 | 0.75 | 2.31 | 0.18 |
| ours | 0.4725 | 0.7597 | 2.312 | 0.1791 |
| (1, 1), 0.7: theirs | 0.47 | 0.56 | 0.35 | 0.13 |
| ours | 0.4681 | 0.5646 | 0.3449 | 0.1269 |
| (1, 0.5), 0.5: theirs | 0.60 | 0.77 | 0.81 | 0.32 |
| ours | 0.6023 | 0.7714 | 0.8071 | 0.3207 |

Each within one unit of their last digit (`fdfd3d/multigrid-stencil-smoothing-factors`); ours
are their symbol's to 1e-14, so their 0.75 and 0.35 read 0.7597 and 0.3449 by it. (Their
$\beta_1 = 0$ row has no $k_0^2$ of its own, which photonoxide's shift $k_0^2(1 + i\beta)\varepsilon$
can't express.)

## The coarse operators: two ways

**Galerkin's, from the stencil** (`Coarse::Galerkin`, the default): $P^{\mathsf T} A P$, the
first coarse level's rows summed coarse row by coarse row as before, each fine row of $A$ made
from the stencil when it is needed (`Lattice::row_into`, its entries summed by column in the
assembled matrix's order and the shift added as `shifted` adds it). The same bits: every coarse
operator, transfer and coarse smoother of the hierarchy is the stored matrix's (a test), and only
the finest level differs. This is Reps, Vanroose and bin Zubair's and Erlangga, Oosterlee and
Vuik's choice, and the one measured to work here: photonoxide's first multigrid, with coarse
grids discretized again and Jacobi smoothing, stalled with PMLs ([FDFD in 3D](fdfd-3d.md),
"What didn't pay").

**Discretized again** (`Coarse::Rediscretized`): Shin and Fan's operator on each coarser grid,
15 entries a row against $P^{\mathsf T} A P$'s 50 to 65. The grids coarsen as for Galerkin's
(a PML's cells merge later than the others), so a coarse grid's cells differ in length: each
coarse cell is as long, and each node's dual cell as wide, as the finest cells it takes in, in
the PMLs' complex stretched coordinate $\tilde x = \int s\,dx$, and the lattice carries those
lengths as its stretches. The permittivity of each coarse value of E is the harmonic mean along
its component of the fine values its cell takes, and across it the mean over its dual cell,
weighed by the coarse hat function. The residual is restricted by $R = W^{-1}P^{\mathsf T}D_f$:
$D_f$ the fine values' volumes (each edge's length times its dual face, in the stretched
coordinate), $W = P^{\mathsf T}D_f\mathbf 1$ each coarse value's share of them, so that $R$'s rows
sum to one. ($W$, not the coarse cells' own volumes: on a grid coarsened unevenly, a cell-centred
hat's integral isn't its cell's length, and with the coarse volumes $R\mathbf 1$ was off by 10 to
150% around the PMLs.) The transposed cycle restricts by $P^{\mathsf T}$ and prolongs by $R^{\mathsf T}$.

It takes more iterations, and fails with F-cycles: on the 40³ guide 25 V-cycle iterations
against Galerkin's 20, and 68 F-cycle iterations against 15. More accurate coarse solves making
the preconditioner worse is the mark of coarse operators that represent the fine problem
poorly in the indefinite range, which Reps et al. (their Section 5.1) and Cools, Reps and
Vanroose (Numer. Linear Algebra Appl. 21, 513 (2013)) trace to coarse grids that resonate; the
latter's level-dependent correction is the thing to try next. With V-cycles it converges, and its
hierarchy is a fifth of Galerkin's.

## Validation

- **Against the direct solver:** fdfd3d/gmres-multigrid-direct's strip ($24 \times 20 \times 16$
  cells of 40 nm, stretched PMLs of 6) by GMRES without the matrix to 1e-10, both coarse
  operators: 2.5e-10 in 23 iterations (Galerkin's) and 3.2e-10 in 30 (discretized again), where
  the stored matrix's multigrid gives 2.0e-10 in 24 (`fdfd3d/multigrid-stencil-direct`).
- **The S-matrices of a sweep against the stored matrix's multigrid:** the coupler of
  fdfd3d/recycle-s-matrix ($18 \times 32 \times 24$ cells of 50 nm, 1.53 to 1.56 µm), plain and
  recycled (GCRO-DR, 10 vectors, the last 4 solutions), all to 1e-11: within 5.7e-12 (Galerkin's) and 8.4e-12 (discretized again) of the
  stored multigrid's, in 417 and 559 iterations over the plain sweep against its 444
  (`fdfd3d/multigrid-stencil-s-matrix`).
- **The same bits on 1 and 4 threads:** the recycled sweep's S-matrices and iterations
  (`fdfd3d/multigrid-stencil-threads`); and the cycle and its transpose on 1, 2, 5 and 20
  (tests).
- **Convergence as the grid refines:** GMRES's iterations to 1e-8 on that strip's box on cells
  of 40, 30 and 20 nm (stretched PMLs as thick in µm), point current at its centre:

  | | 40 nm | 30 nm | 20 nm |
  |---|---|---|---|
  | the stored matrix's multigrid | 18 | 16 | 16 |
  | without the finest matrix, Galerkin's coarse operators | 18 | 17 | 19 |
  | without it, discretized again | 23 | 21 | 23 |

  Level as the cells halve (`fdfd3d/multigrid-stencil-refinement`).
- **The smoother** against its symbol and against Erlangga et al.'s Table 5 (above).
- **The transposed cycle is the cycle's transpose,** $u^{\mathsf T}(M^{-1}v) = v^{\mathsf T}(M^{-\mathsf T}u)$
  to 1e-10, V- and F-cycles, both coarse operators (a test).

No paper gives a multigrid without the matrix on this problem to reproduce: Erlangga et al.'s
smoothing factors are the published numbers this method's new part has.

## Measured

Core Ultra 7 265K, 20 threads, 64 GB (shared, so each problem was run twice, alternating, and
the faster kept), `photonoxide bench --tier full`, each problem in a process of its own, its
peak working set read when its timed phases end. GMRES to 1e-8 with the default V(0, 1) on the
coarse levels and, without the matrix, Jacobi V(1, 1) on the finest. The strip with ports (a
500 × 220 nm silicon strip in oxide, 0.8 × 1.4 × 1.0 µm inside stretched PMLs, its two ports'
S-matrix) against its exact S; the guide (a silicon guide through a cube of 10 nm cells,
stretched PMLs of 10, a dipole beside it) against the stored multigrid's field to 1e-12.

| Problem | Unknowns | Multigrid | Built in | Iterations | Solve | Total | Peak memory | Error |
|---|---|---|---|---|---|---|---|---|
| strip, 40 nm, 36 × 51 × 41, PMLs of 8 | 225 828 | the matrix's | 2.28 s | 56 | 1.81 s | 4.26 s | 1.22 GB | 8.8e-9 |
| | | Galerkin's, no finest matrix | 1.64 s | 62 | 1.92 s | 3.74 s | 0.77 GB | 2.5e-9 |
| | | discretized again | 1.30 s | 95 | 2.42 s | 3.92 s | 0.44 GB | 1.6e-9 |
| strip, 25 nm, 58 × 82 × 66, PMLs of 13 | 941 688 | the matrix's | 9.22 s | 55 | 5.98 s | 15.7 s | 4.92 GB | 1.3e-8 |
| | | Galerkin's, no finest matrix | 6.42 s | 61 | 5.98 s | 13.0 s | 3.13 GB | 9.9e-9 |
| | | discretized again | 5.20 s | 88 | 7.40 s | 13.2 s | 1.72 GB | 4.8e-9 |
| strip, 20 nm, 72 × 102 × 82, PMLs of 16 | 1 806 624 | the matrix's | 17.7 s | 56 | 11.7 s | 30.3 s | 9.51 GB | 4.1e-9 |
| | | Galerkin's, no finest matrix | 12.2 s | 62 | 11.7 s | 25.0 s | 6.00 GB | 3.5e-9 |
| | | discretized again | 9.84 s | 84 | 13.9 s | 24.7 s | 3.29 GB | 1.1e-8 |
| guide, 44³ | 255 552 | the matrix's | 2.68 s | 16 | 0.62 s | 3.30 s | 1.37 GB | 2.9e-6 |
| | | Galerkin's, no finest matrix | 1.96 s | 22 | 0.77 s | 2.73 s | 0.90 GB | 2.3e-6 |
| | | discretized again | 1.47 s | 27 | 0.79 s | 2.26 s | 0.49 GB | 3.4e-6 |
| guide, 68³ | 943 296 | the matrix's | 9.38 s | 22 | 2.51 s | 11.9 s | 4.99 GB | 3.5e-6 |
| | | Galerkin's, no finest matrix | 6.59 s | 23 | 2.32 s | 8.91 s | 3.22 GB | 1.7e-6 |
| | | discretized again | 5.21 s | 29 | 2.57 s | 7.78 s | 1.70 GB | 1.3e-6 |
| guide, 96³ | 2 654 208 | the matrix's | 25.8 s | 27 | 7.93 s | 33.8 s | 13.4 GB | 9.5e-6 |
| | | Galerkin's, no finest matrix | 17.4 s | 33 | 8.84 s | 26.3 s | 8.45 GB | 2.7e-6 |
| | | discretized again | 13.7 s | 47 | 10.9 s | 24.7 s | 4.45 GB | 6.5e-6 |
| guide, 128³ | 6 291 456 | the matrix's | 63.1 s | 40 | 27.8 s | 91.0 s | 33.3 GB | 2.0e-5 |
| | | Galerkin's, no finest matrix | 41.4 s | 50 | 31.1 s | 72.5 s | 22.5 GB | 7.6e-6 |
| | | discretized again | 32.9 s | 73 | 39.8 s | 72.7 s | 10.2 GB | 1.6e-5 |

(Built in: the operator and the hierarchy, the permittivity's average over each cell included;
the strip's total has its port modes too. Error: the strip's S against the exact one, the
guide's field against the stored multigrid's to 1e-12, relative.) The two passes agreed within
2% but for the 96³ and 128³ guides, whose first pass, run while I was compiling, was up to 30%
slower.

- **Memory:** the peak falls from 5.0 to 5.4 kB an unknown with the matrix to 3.2 to 3.6 kB
  without the finest one (Galerkin's coarse operators) and 1.6 to 1.9 kB discretized again: on
  the strip at 20 nm 9.5 GB to 6.0 and 3.3 GB, on the 128³ guide 33.3 GB to 22.5 and 10.2 GB.
  What is left with Galerkin's operators is mostly the first coarse level (about 1.7 kB a fine
  unknown with its ILU(0)) and GMRES's 40 vectors (0.64 kB).
- **Time:** faster at every size from 2 × 10⁵ unknowns, 1.1 to 1.5 times in total: the
  hierarchy builds in two thirds or half the time (no assembly, no ILU(0) of the finest level),
  and an iteration costs about what it did (the stencil's product and a Jacobi sweep against the
  stored product and ILU(0)'s triangular solves), so the solve itself is within 12% with Galerkin's
  operators and up to 1.4 times slower discretized again, which takes 1.3 to 1.8 times the
  iterations.
- **Where plain QMR broke down** ([#250](https://github.com/tachsin/photonoxide/issues/250): the
  strip at 20 nm, the guide at 96³ and 128³), all three multigrids solve, in 27 to 73
  iterations. At 128³ the field's error at a residual of 1e-8 is 2.0e-5 with the matrix and
  1.6e-5 discretized again, above the catalogue's 1e-5 check, which flags them: Shin and Fan's
  residual measures the field less well than the curl-curl one's ([FDFD in 3D](fdfd-3d.md)), and
  a tolerance a decade lower is what that size wants.

## Choosing it

`IterativeSolver3d::new_with(grid, wavelength, eps, boundaries, Formulation::ShinFan,
Product::MatrixFree)` builds Shin and Fan's operator without its matrix; `with_multigrid`
then builds the multigrid without its finest matrix, `Multigrid::coarse` saying which coarse
operators and `Multigrid::jacobi` the finest level's smoothing. Its results differ from the
stored matrix's multigrid's in their last bits (another smoother), so it is asked for, never
assumed: the default stays the matrix, and `Product::Auto` keeps the matrix on Shin and Fan's
operator. Plain QMR takes the operator too (its transpose is the product above); ILU(0) and an
iterative backend need the matrix and are refused. `photonoxide bench`'s catalogue has the
three side by side (`fdfd3d-iterative/strip-ports-multigrid-*` and `guide-multigrid-*`).

## Left

- **Coarse operators discretized again with F-cycles,** which need a level-dependent shift or
  correction (Cools, Reps and Vanroose) before they help.
- **The transfers without their matrices:** $P$ and $P^{\mathsf T}$ to the first coarse level are
  0.29 GB of the strip's peak; they are tensor products of 1D weights times the permittivity's
  ratios and could be applied from the 1D tables.
- **The first coarse level without its matrix** too, from a coarse stencil, smoothed by Jacobi:
  with Galerkin's operators it is the largest thing left (3 GB on the strip at 20 nm).
- **The Galerkin build from the stencil** makes each fine row about three times, once for each
  coarse row that reaches it; keeping a slab's rows while its coarse rows are summed would cut it.
- **A GPU's or another backend's cycle:** the backend's cycle takes each level's matrix, so the
  multigrid without the finest one runs on photonoxide's own GMRES.

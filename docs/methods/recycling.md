---
title: "Recycling: solves that learn from the ones before"
module: fdfd
summary: "A 3D wavelength sweep, an S-matrix's ports, a forward solve and its adjoint, an optimization's designs: each iterative solve can start from what the ones before learned. The earlier solutions' combination of least residual as the start (Fischer), on every path, QMR's included; and GCRO-DR's recycled Krylov space (Parks et al.) for GMRES with the multigrid. Opt-in, through a Recycler the caller carries."
order: 19.6
papers:
  - cite: "M. L. Parks, E. de Sturler, G. Mackey, D. D. Johnson, S. Maiti, SIAM J. Sci. Comput. 28, 1651 (2006) (GCRO-DR: recycling Krylov subspaces for sequences of systems)"
    doi: 10.1137/040607277
  - cite: "P. F. Fischer, Comput. Methods Appl. Mech. Engrg. 163, 193 (1998) (projection onto earlier solutions)"
    doi: 10.1016/S0045-7825(98)00012-7
  - cite: "D. Bertaccini, F. Durastante, Iterative Methods and Preconditioning for Large and Sparse Linear Systems with Applications, CRC (2018), Section 3.6 (preconditioning sequences of systems)"
    doi: 10.1201/9781315153575
  - cite: "Y. Saad, Iterative Methods for Sparse Linear Systems, 2nd ed., SIAM (2003) (GMRES, Givens rotations)"
    doi: 10.1137/1.9780898718003
validation:
  - fdfd3d/recycle-s-matrix
  - fdfd3d/recycle-deflation
  - fdfd3d/recycle-convergence
  - fdfd3d/recycle-adjoint
  - fdfd3d/recycle-threads
  - fdfd3d/recycle-parks-table
  - fdfd3d/recycle-parks-rerun
---

A 3D solve by QMR or GMRES ([FDFD in 3D](fdfd-3d.md)) builds its Krylov space from nothing and
throws it away. A sweep solves the same device at wavelength after wavelength, an S-matrix one
source per port, an adjoint gradient one more system with the forward's operator, and an
optimization the same pair at design after design. A `Recycler` carries what one solve learned
to the next:

```rust
use photonoxide::fdfd::{Formulation, IterativeSolver3d, Multigrid, Recycler, Stopping};

// a sweep: each wavelength started from the last ones' fields
let mut recycler = Recycler::new().with_solutions(8);
for &lam in &wavelengths {
    let solver = IterativeSolver3d::new(grid, lam, eps, boundaries, Formulation::ShinFan)?
        .with_multigrid(Multigrid::default())?;
    let ports = /* the solver's port modes */;
    let s = solver.s_matrix_recycled(&ports, Stopping::default(), &mut recycler)?;
}

// a forward solve and its adjoint, GCRO-DR's space carried from one to the other
let mut recycler = Recycler::new().with_krylov(10).with_solutions(2);
let (field, _) = solver.solve_system_recycled(&source, stopping, &mut recycler)?;
let (power, gradient, how) =
    solver.mode_power_gradient_recycled(&field, &mode, Direction::Forward, stopping, &mut recycler)?;
```

Every field is held to the same relative residual as a plain solve, so the S-matrices and
gradients agree with the plain ones to that tolerance (the validation below). It is
**opt-in**: the plain methods are unchanged, and the recycled ones are new.

## Two things to carry

**The earlier solutions.** P. F. Fischer, "Projection techniques for iterative solution of
Ax = b with successive right-hand sides", Comput. Methods Appl. Mech. Engrg. 163, 193 (1998),
[doi:10.1016/S0045-7825(98)00012-7](https://doi.org/10.1016/S0045-7825(98)00012-7), his
Method 1: of the last $s$ solutions $x_i$, the combination $x_0 = \sum_i \alpha_i x_i$ with the
least residual $\lVert b - A x_0\rVert$, found by projecting $b$ onto the orthonormalized
$A x_i$ (his Eq. 4); the solver then solves for what is left, $A\,\delta = b - A x_0$, to the
tolerance that makes $x_0 + \delta$ meet the original one. It is Parks et al.'s first step
(their Appendix, lines 6 and 7) with the earlier solutions as the space, and it serves any
solver: QMR plain or with ILU(0), GMRES with the multigrid, an iterative backend. Each $A x_i$
is taken once per operator (one product, no preconditioner) and kept; the residual the solve
continues from is computed afresh, $b - A x_0$, since nearly dependent solutions cancel. An
$A x_i$ dependent on the earlier ones to 1e-12 is dropped.

A wavelength's field lies near a polynomial in the wavelength through the last ones' fields, so
the best combination of $2p$ solutions of a two-port device starts each solve where a
polynomial of degree $p - 1$ would, and better: the projection finds the best combination, not
a fixed extrapolation. In an S-matrix the last solutions are each port's at the wavelengths
before, and the projection picks the port's own.

**A Krylov space: GCRO-DR.** M. L. Parks, E. de Sturler, G. Mackey, D. D. Johnson, S. Maiti,
"Recycling Krylov subspaces for sequences of linear systems", SIAM J. Sci. Comput. 28, 1651
(2006), [doi:10.1137/040607277](https://doi.org/10.1137/040607277), their Section 2.4 and
Appendix, for GMRES preconditioned by the multigrid cycle from the right, on
$\mathcal{A} = A M^{-1}$:

- **The recycled space** (their Eqs. 2.5 and 2.6): $U_k$, $C_k$ with $\mathcal{A} U_k = C_k$
  and $C_k^\mathsf{H} C_k = I$. For a new operator (the next wavelength's, with its own
  multigrid) $C_k$ is rebuilt from the last system's $U_k$: $[Q, R] = \operatorname{qr}(\mathcal{A}
  U_k)$, $C_k = Q$, $U_k \leftarrow U_k R^{-1}$ (their Eq. 2.7, Appendix lines 3 to 5), $k$
  cycles of the new multigrid. For the same operator (the adjoint, the next port) it is used as
  it is, at no cost: each solver has a number, renewed when its preconditioner changes, that
  says which operator $C_k$ belongs to.
- **A cycle** (lines 6, 7 and 22 to 29): $x \mathrel{+}= U_k C_k^\mathsf{H} r$,
  $r \mathrel{-}= C_k C_k^\mathsf{H} r$; then $m - k$ Arnoldi steps on
  $(I - C_k C_k^\mathsf{H})\mathcal{A}$ from $r$, by modified Gram–Schmidt against $C_k$ and then
  the Arnoldi vectors, which give $\mathcal{A}[U_k\;V] = [C_k\;V^+]\,G$,
  $G = \begin{bmatrix} D_k & B \\ 0 & \underline{H} \end{bmatrix}$ (Eqs. 2.9 to 2.11; $D_k$ scales
  $U_k$'s columns to unit length, $B = C_k^\mathsf{H}\mathcal{A}V$). With $r \perp C_k$ the least
  squares problem of Eqs. 2.12 to 2.15 solves its top block exactly, and its residual is GMRES's
  on $\underline{H}$: Givens rotations give it at every step (Saad 2003, Section 6.5.3), and a cycle
  stops as soon as the tolerance is met. The iterate's correction $U_k(d - B y) + V y$ takes one
  more cycle of the multigrid, and the true residual is computed after each cycle.
- **What is kept** (lines 30 to 34): the $k$ harmonic Ritz vectors of $\mathcal{A}$ over
  $\operatorname{range}([U_k\;V])$ of smallest harmonic Ritz value,
  $G^\mathsf{H} G z = \theta\, G^\mathsf{H} W^\mathsf{H} \hat V z$ with $W = [C_k\;V^+]$ and
  $\hat V = [U_k D_k\;V]$ (Eq. 2.16; the small pencil by faer's QZ), then $Y = \hat V P$,
  $[Q, R] = \operatorname{qr}(G P)$, $C_k = W Q$, $U_k = Y R^{-1}$. The first cycle of the
  first system, with no $U_k$, is GMRES($m$), and the same formula with $k = 0$ gives
  GMRES-DR's harmonic Ritz vectors (line 14). After the last cycle $U_k$ is kept for the next
  system (line 36).

$m$ is the multigrid's restart (40 by default), and $k$ must be smaller. GCRO-DR keeps
$m + k + 1$ vectors where GMRES keeps $m + 1$.

**The same bits on any number of threads.** Every vector operation is a pass over fixed
chunks, each chunk's sums in order and the chunks' sums in order, as QMR's are; the several
outputs of a combination are written in one pass over its inputs, chunk by chunk, each value's
terms in order. The small eigenproblem and the small QR factorizations are dense and
sequential.

**The adjoint is a solve with the forward's own operator.** For the power of a port mode,
$\lambda = V A^{-1}(V^{-1}\bar a\,\omega)$ with $V$ the PMLs' stretches
([adjoint gradients](fdfd-adjoint.md)): a solve with $A$ itself, which on Shin and Fan's
operator too is the forward's operator (their right-hand side is transformed with it, the
solution the same). `IterativeSolver3d::mode_power_gradient` solves it by the solver's own
iteration, and `mode_power_gradient_recycled` through a recycler, after the forward solve with
the same one.

## Validation

- **The plain solves' S-matrices**: a directional coupler's straight section (two silicon
  strips 350 × 220 nm, 150 nm apart, in oxide; 18 × 32 × 24 cells of 50 nm, PMLs of 4), one mode
  at each end, at 1.53, 1.54, 1.55 and 1.56 µm. Recycled against one plain solve per port:
  6.8e-12 by GMRES with the multigrid (GCRO-DR keeping 10 vectors and the last 4 solutions, to
  1e-11) and 2.1e-11 by symmetric QMR (the last 4 solutions, to 1e-10, the lowest it reaches on
  this problem) (`fdfd3d/recycle-s-matrix`).
- **Analytic**: Parks et al.'s Theorem 3.1 with an exact invariant subspace ($\delta = 0$). On
  $A = H D H$ ($n = 300$, $H$ a Householder reflector), GCRO-DR recycling the eigenvectors of
  $D$'s 8 eigenvalues near zero converges as GMRES on the deflated problem, step by step: the
  residual histories agree to 2.2e-11, 39 iterations each, against 114 for GMRES on $A$
  (`fdfd3d/recycle-deflation`). And the gradient: the iterative solvers' adjoint, recycled from
  the forward, against the direct solver's on a guide with a block beside it (14 × 12 × 24 cells
  of 50 nm): 2.3e-11 by QMR, 7.8e-11 by the multigrid and GCRO-DR (`fdfd3d/recycle-adjoint`).
- **Convergence**: the coupler by the multigrid at 1.500, 1.505, … 1.535 µm, recycling the last
  0, 2, 4, 6 and 8 solutions, to 1e-10. Iterations at each wavelength, both ports summed:

  | Solutions kept | 1.500 | 1.505 | 1.510 | 1.515 | 1.520 | … | 1.535 |
  |---|---|---|---|---|---|---|---|
  | none | 106 | 106 | 105 | 104 | 104 | | 103 |
  | 2 | 106 | 80 | 80 | 80 | 80 | | 78 |
  | 4 | 106 | 80 | 64 | 62 | 62 | | 62 |
  | 6 | 106 | 80 | 63 | 42 | 42 | | 42 |
  | 8 | 106 | 80 | 63 | 42 | 24 | | 24 |

  Each pair of solutions kept (two ports) raises the order the start is good to, once the sweep
  has that many: 0.23 of the plain sweep's at the last (`fdfd3d/recycle-convergence`).
- **Published**: Parks et al.'s Section 4.4, $u_{xx} + u_{yy} + c u_x = 0$ on the unit square
  ($h = 1/41$, 1600 unknowns, $u = 0$ on two sides and 1 on the others), GCRO-DR(25, 10) from
  zero to 1e-10, run twice. Their Table 4.3 for $c = 0$: the cosines of the principal angles
  between the recycled space and the invariant subspace of the 10 eigenvalues of smallest
  magnitude are 1, 1, 1, 1, 0.99999999999703, then 5.9e-9, 3.8e-11, 3e-14, 0 and 0; here 1 to
  7.2e-8, then 1.1e-8, 8.7e-10, 2.0e-11, 1.0e-12 and 1.1e-14 (`fdfd3d/recycle-parks-table`).
  Five, because the right-hand side has no part on the four antisymmetric eigenvectors among
  the ten, $\sin p\pi x \sin q\pi y - \sin q\pi x \sin p\pi y$, nor on $\sin 2\pi x \sin 2\pi y$ (its
  sums over the boundary's rows vanish for even $p$ and $q$), so no Krylov space holds them. Their
  Fig. 4.9: the second run converges faster than full GMRES, the first (GMRES-DR) a little
  slower; here 73, 126 and 133 products (`fdfd3d/recycle-parks-rerun`). For $c = 40$ they find the
  second run "approximately the same" as the first; here 115 against 129, and full GMRES 101. Their
  other problems (a fracture model's 151 stiffness matrices, an electronic-structure matrix, a QCD
  matrix) aren't published as matrices here.
- **Threads**: the coupler's recycled sweep by the multigrid and GCRO-DR, 1.54 to 1.56 µm, on
  1 and 4 threads: the same bits and the same iterations (`fdfd3d/recycle-threads`).

## Measured

The coupler of the validation, 1.0 µm long inside PMLs of 6 cells all round, one mode at each
end (two ports), its S-matrix at each wavelength to a relative residual of 1e-8. An Intel Core
Ultra 7 265K (20 cores, 64 GB), 20 threads, shared with other work while it ran: each time is
the faster of two runs, plain and recycled alternately; the solves' time, the builds (the
operator, the multigrid, the port modes) apart (`cargo test --release --lib
recycle_checks::tests::measure -- --ignored --nocapture`, its settings in its docs).

**A sweep of 16 wavelengths**, 1.500 to 1.575 µm every 5 nm:

| Grid | Solver | Recycled | Iterations | Solves | Builds | Iterations at the last wavelength |
|---|---|---|---|---|---|---|
| 50 nm, 32 × 36 × 28 cells, 96 768 unknowns | GMRES + multigrid | none | 1 419 | 26.2 s | 16 to 19 s | 81 |
| | | last 4 solutions | 792 | 13.4 s | | 42 |
| | | last 8 solutions | **435** | **8.1 s** | | 16 |
| | | last 8, and GCRO-DR(40, 10) | 439 | 14.2 s | | 14 |
| 50 nm, the same | symmetric QMR, plain PMLs | none | 26 864 | 16.4 s | 11 s | 1 730 |
| | | last 8 solutions | **10 648** | **8.1 s** | | 433 |
| 25 nm, 52 × 60 × 44 cells, 411 840 unknowns, 12 wavelengths | GMRES + multigrid | none | 858 | 41.4 s | 50 to 68 s | 70 |
| | | last 8 solutions | **287** | **25.5 s** | | 12 |

The S-matrices agree with the plain ones to the tolerance: 1.3e-8 to 4.9e-8 by the multigrid,
6.1e-9 by QMR. The earlier solutions take the multigrid's sweep from about 90 iterations a
wavelength to 16, and QMR's from about 1 700 to 450: 3.3 and 2.5 times fewer in all, the first
wavelengths included, and the solves 3.2 and 2 times faster. GCRO-DR's space adds nothing to
them and costs its 10 cycles a wavelength to carry over (150 in all). At 25 nm the iterations
fall 3 times and the solves 1.6 times, on a machine whose load varied more then (the same plain
sweep took 41 to 63 s), and a solve of 12 iterations keeps its fixed costs (the port's source,
the true residuals, the last cycle). With the solves recycled, the multigrid's builds are most of
a sweep's time.

**A forward solve and its adjoint**, the power of the right port's mode for the left port's
source, at 1.55 µm (50 nm, 96 768 unknowns), GMRES with the multigrid to 1e-8:

| Recycled | Forward | Adjoint | Time |
|---|---|---|---|
| none | 43 | 42 | 1.30 s |
| GCRO-DR(40, 10) | 43 | 44 | 1.49 s |
| GCRO-DR(40, 20) | 43 | 41 | 1.61 s |
| GCRO-DR(40, 30) | 43 | 41 | 2.04 s |

The gradients agree with the plain one's to 3.4e-7 to 5.8e-7 of its largest value (the
tolerance's). The forward's Krylov space hardly serves the adjoint (below), so GCRO-DR stays
opt-in and isn't recommended on this path.

**An optimization's designs**: the same pair at 8 designs in turn, the gap between the strips
over the middle third of the length raised by 0.02 in permittivity each time (a new operator and
multigrid each), one recycler through them all:

| Recycled | Forward, adjoint at each design | Time |
|---|---|---|
| none | 43, 42 at each | 10.9 s |
| last 2 solutions | 43, 42; then 26, 26 at each | 7.2 s |
| last 2, and GCRO-DR(40, 10) | 43, 44; then 25 to 27 | 9.2 s |

The last two solutions are the last design's forward field and adjoint, and the projection
starts each solve from its own.

## Why the Krylov space carries so little here

GCRO-DR's recycled space pays where convergence is held back by a few eigenvalues of
$\mathcal{A}$ near zero: deflate them, and the rest converges as the deflated problem does
(their Theorem 3.1, and the analytic case above, 39 iterations against 114). The multigrid's
shifted operator puts the preconditioned spectrum of a Helmholtz problem on a curve through the
origin, and the eigenvalues near it are the resonant ones, of which a 3D device has many. Ten or
thirty of them deflated leave the rest. Measured on the coupler at 1.55 µm (50 nm, 1 µm long,
96 768 unknowns), the second port's solve after the first port's, to 1e-8:

| Recycled | none | GCRO-DR(40, 10) | (40, 20) | (40, 30) | (80, 40) | the first solve's whole Krylov space (43 vectors) |
|---|---|---|---|---|---|---|
| Iterations | 42 | 45 | 42 | 41 | 39 | 39 |

and the first port's own source again, from its harmonic Ritz vectors: 44, 39, 38 and 20 (0 from the
whole space, which holds its solution). A port's Krylov space is the waves its source launches;
the other port's are other waves. The earlier solutions carry what the Krylov space doesn't: the
field itself, which moves little from one wavelength or design to the next.

## The preconditioner of a sequence

D. Bertaccini and F. Durastante, *Iterative Methods and Preconditioning for Large and Sparse
Linear Systems with Applications*, CRC (2018),
[doi:10.1201/9781315153575](https://doi.org/10.1201/9781315153575), Section 3.6, treat the other
half of a sequence: its preconditioner. Reusing one for every system slows convergence,
rebuilding one for each costs, and between the two they update an approximate inverse
factorization of a seed matrix by a sparsified correction for each $A^{(k)} = A^{(0)} +
\Delta_k$ (their Eqs. 3.38 to 3.43). Here each wavelength builds its own multigrid, and once the
solves are recycled the builds are most of a sweep's time (below). The multigrid isn't a factored
approximate inverse, so their updates don't apply as they stand; reusing a hierarchy across
nearby wavelengths, or updating its coarsest factorization, is left for later.

## Left for later

- Recycling inside QMR's own iteration: its short recurrences keep no space, and recycling for
  Lanczos methods (recycled BiCG and MINRES-type methods) is another literature.
- GCROT's optimal truncation, the paper's other method; and other recycled spaces than
  harmonic Ritz vectors, which their Section 4.4 suggests for nonsymmetric problems.
- Reusing or updating the multigrid from one wavelength to the next (Bertaccini and
  Durastante's Section 3.6), now that the builds are most of a recycled sweep's time.
- Recycling in block QMR (Jolivet and Tournier's block GCRO-DR), and with an iterative backend,
  where only the starting guess is recycled.
- A sweep and an optimization's loop that recycle by themselves: today the caller carries the
  recycler.
- No adjoint solve at all where an S-matrix is solved anyway: the mode power's λ is the field of
  the mode launched backwards from the monitor, times a constant ([adjoint
  gradients](fdfd-adjoint.md)), which is that port's own run.

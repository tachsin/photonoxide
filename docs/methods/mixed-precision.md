---
title: "Mixed precision: single-precision factors refined in double"
module: backend
summary: "photonoxide's own multifrontal factorization in single precision, half the factors' memory, each solve refined to double precision by GMRES preconditioned with the factors (GMRES-IR, Carson and Higham; Amestoy et al.). A system too ill-conditioned for single-precision factors is an error with the refinement's history, never a field. Opt-in, as the direct backend photonoxide-mixed; auto never chooses it."
order: 19.75
papers:
  - cite: "E. Carson, N. J. Higham, SIAM J. Sci. Comput. 40, A817 (2018) (iterative refinement in three precisions; GMRES-IR, its Algorithm 8.1)"
    doi: 10.1137/17M1140819
  - cite: "P. Amestoy, A. Buttari, N. J. Higham, J.-Y. L'Excellent, T. Mary, B. Vieublé, ACM Trans. Math. Softw. 49, 4 (2023) (mixed-precision refinement of sparse approximate factorizations, on MUMPS)"
    doi: 10.1145/3582493
  - cite: "N. J. Higham, T. Mary, Acta Numerica 31, 347 (2022) (mixed precision algorithms in numerical linear algebra: where low-precision factors stop being enough)"
    doi: 10.1017/S0962492922000022
  - cite: "Y. Saad, Iterative Methods for Sparse Linear Systems, 2nd ed., SIAM (2003) (Algorithm 9.4, GMRES with left preconditioning)"
    doi: 10.1137/1.9780898718003
---

A 3D direct solve is bounded by its factors' memory: photonoxide's own L D Lᵀ of a 40³ grid
(192 000 unknowns) takes 3.8 GB ([FDFD in 3D](fdfd-3d.md), "Cost"), and the factors grow as the
unknowns to the power 1.4. Single-precision factors take half of it. Refined in double, they
can still give the double-precision solution, or say that they can't:

```rust
use photonoxide::backend::{Choice, MIXED};
use photonoxide::fdfd::Solver3d;

let solver = Solver3d::new_on(grid, wavelength, eps, boundaries, &Choice::Named(MIXED.into()))?;
let field = solver.solve(&current)?; // photonoxide's own field, or an error saying why not
```

In a job file it is the `fdfd` job's direct solver:

```yaml
solver:
  direct: photonoxide-mixed
```

It is **opt-in**: `auto` never chooses it, whatever the benchmarks say (see [backends](backends.md),
"What auto chooses"), because on a system too ill-conditioned for single precision it fails,
by design, where photonoxide's own double-precision factors don't.

## The method

**The factors.** The same analysis as photonoxide's own (the matching and its scaling, nested
dissection, the fronts), the same elimination, the same static pivoting: B's entries are
rounded to single precision after the scaling, and the fronts are assembled, eliminated and
stored in it, by faer's dense kernels in `Complex<f32>`. A pivot below √ε ‖B‖₁ is perturbed to
it, ε double precision's as in photonoxide's own (1.5 × 10⁻⁸ ‖B‖₁), not single's (3.5 × 10⁻⁴):
a perturbation enters the refinement's convergence as u_f ρₙ + ε (Amestoy et al., Theorem 4.1),
so single's √ε would outweigh single precision itself (u_f = 6 × 10⁻⁸) by thousands, and
double's doesn't. Measured on the 24³ guide below, single's √ε perturbed 4 210 pivots, the
L D Lᵀ gave way to the LU (as many bytes in single as the L D Lᵀ in double), and GMRES took 45
to 50 iterations a step; double's perturbed none, and GMRES took two. As photonoxide's own, the
3D solver factorizes the complex symmetric similarity by L D Lᵀ when no pivot is perturbed, and
A by LU otherwise.

**The solves.** Every solve with the factors runs in double precision: each entry is widened as
it is read, so the factors stay single in memory and the substitutions add only double's
rounding. Amestoy et al. (2023, Section 5.1) describe the same cast "on the fly" and leave it
for later; their measurements cast the factors to double first.

**The refinement.** GMRES-IR (E. Carson, N. J. Higham, "Accelerating the solution of linear
systems by iterative refinement in three precisions", SIAM J. Sci. Comput. 40, A817 (2018),
[doi:10.1137/17M1140819](https://doi.org/10.1137/17M1140819), Algorithm 8.1; P. Amestoy et al.,
"Combining sparse approximate factorizations with mixed-precision iterative refinement", ACM
Trans. Math. Softw. 49, 4 (2023), [doi:10.1145/3582493](https://doi.org/10.1145/3582493),
Algorithm 2):

1. x₀ = M⁻¹ b, M the single-precision factors;
2. rᵢ = b − A xᵢ, in double;
3. dᵢ by GMRES on M⁻¹ A dᵢ = M⁻¹ rᵢ (Y. Saad, *Iterative Methods for Sparse Linear Systems*,
   2nd ed., SIAM (2003), [doi:10.1137/1.9780898718003](https://doi.org/10.1137/1.9780898718003),
   Algorithm 9.4), in double, until it has reduced its residual by 10⁻⁶ (Amestoy et al.'s τ_g,
   their Section 5.2) or for 60 iterations;
4. xᵢ₊₁ = xᵢ + dᵢ, and again from 2.

In Amestoy et al.'s five precisions, (u_f, u, u_r, u_g, u_p) = (single, double, double, double,
double): one of the variants they run on MUMPS. Refinement by the factors alone (LU-IR) is
guaranteed only for κ(A) u_f ≪ 1 (their Eq. 4.1); GMRES-IR's condition (their Eq. 4.2) is
weaker, so it solves systems the factors alone can't, at the price of more solves with them.
Their Theorem 4.1 adds what static pivoting and the factorization's growth cost: u_f ρₙ + ε
in place of u_f, ε the perturbation's size.

**When it stops.** Each iterate's normwise backward error,
η = ‖b − A x‖∞ / (‖A‖∞ ‖x‖∞ + ‖b‖∞) (Carson and Higham's Section 2), computed in double:

- **Converged** when η ≤ p u, u = 2⁻⁵³ and p the most nonzeros in a row of [A b]: the rounding
  of the residual itself, below which a computed η says nothing more (the p of the limiting
  accuracy in N. J. Higham, T. Mary, "Mixed precision algorithms in numerical linear algebra",
  Acta Numerica 31, 347 (2022), [doi:10.1017/S0962492922000022](https://doi.org/10.1017/S0962492922000022),
  Eq. 6.4).
- **Failed** when a step doesn't halve η, or after 20 steps. The solve is then an error that
  gives every η and GMRES's iterations in every step, the pivots the factors perturbed, and
  what to do instead (photonoxide's own factors): never an iterate that only looks like an
  answer. Higham and Mary (Section 7) set out the limits: refinement by low-precision factors
  is guaranteed only while κ(A) u_f ≪ 1, GMRES-IR (their Theorem 7.2) while
  κ(A)² u_f² (u_g + κ(A) u_p) ≪ 1, and past them nothing is promised.

The refinement measures η for the system the factors are of, the complex symmetric
similarity B = S A S⁻¹ when there is one. The 3D solver takes its field as it is, and doesn't
go on to the step of refinement and the QMR it gives photonoxide's own factors when their
relative residual is above 10⁻¹². The 2D solver keeps its one step, in A's terms, after either:
the validation below shows why.

## Where it converges and where it can't

A closed box of 12³ cells of 50 nm (walls all round), a silicon cube 240 nm across in oxide, a
dipole beside it, at longer and longer wavelengths. The curl-curl operator's gradients see only
k₀² ε, and as the wavelength grows that falls towards what single precision resolves in the
factors (u_f ‖B‖): GMRES needs more iterations a step, and at 1 mm (20 000 cells a wavelength)
single precision has lost them. The factors perturb pivots, GMRES runs to its 60 iterations
without halving η, and the solve is an error with this history. Photonoxide's own
double-precision factors reach η ≤ p u there (the test
`single_precision_factors_say_so_where_they_arent_enough` checks both, on 10³ cells). The field
is against photonoxide's own (relative, 2-norm): both are double-precision solutions of a system
whose condition grows as the wavelength squared, and they differ by what that condition lets
rounding do. `cargo test --release --lib fdfd::three::mixed_tests::histories -- --ignored
--nocapture`, 2026-11-12:

| Wavelength | Perturbed pivots | Backward errors η, step by step | GMRES iterations a step | Field against photonoxide's own |
|---|---|---|---|---|
| 1.55 µm | 0 | 2.2e-6, 9.9e-13, 3.4e-17, 4.1e-17 | 3, 4, 3 | 5.6e-15 |
| 15 µm | 0 | 1.8e-8, 6.5e-16, 5.5e-17, 4.1e-17 | 2, 2, 2 | 5.8e-13 |
| 100 µm | 0 | 1.2e-8, 7.7e-16, 1.6e-17, 3.5e-17 | 4, 4, 4 | 1.7e-11 |
| 300 µm | 0 | 1.7e-8, 5.6e-15, 2.7e-17, 3.3e-17 | 7, 7, 7 | 1.3e-10 |
| 1 mm | 21 | 5.2e-8, 5.9e-9, 6.4e-9 | 60, 60 | failed, said so |
| 3 mm | 139 | 2.1e-8, 4.3e-8 | 60 | failed, said so |

The target p u is 1.6e-15 here (p = 14). The last step of each converged row is the one that
stopped halving η: the limiting accuracy. At 10 mm photonoxide's own factors fail too.

## Validation

The 3D validation cases that factorize, run with `photonoxide-mixed` in `auto`'s place
(`backend::prefer_direct`; `cargo test --release --test mixed_precision -- --ignored`), each
against its own Rule and tolerance, unchanged; the value photonoxide's own reports beside it:

| Case | Factorizations | photonoxide | photonoxide-mixed | Difference | Tolerance | Within |
|---|---|---|---|---|---|---|
| `fdfd3d/film-reflection-te` | 2 | 0.058123 | 0.058123 | 5.285078e-13 | 6.000000e-5 | yes |
| `fdfd3d/film-reflection-tm` | 2 | 0.026843 | 0.026843 | 4.518608e-14 | 5.000000e-5 | yes |
| `fdfd3d/flux-conservation` | 12 | 1.726341e-14 | 1.883033e-13 | 1.710398e-13 | 1.000000e-10 | yes |
| `fdfd3d/pml-reflection` | 2 | 2.512754e-6 | 2.512754e-6 | 2.299888e-14 | 1.000000e-5 | yes |
| `fdfd3d/two-d-agreement` | 4 | 2.048213e-13 | 2.110736e-13 | 6.252293e-15 | 1.000000e-11 | yes |
| `fdfd3d/straight-strip` | 1 | 2.563149e-15 | 2.277987e-15 | 2.851617e-16 | 1.000000e-12 | yes |
| `fdfd3d/adjoint-gradient` | 33 | 3.406190e-10 | 2.572465e-10 | 8.337254e-11 | 1.000000e-7 | yes |
| `fdfd3d/reciprocity` | 1 | 1.162814e-15 | 8.181531e-16 | 3.446609e-16 | 1.000000e-12 | yes |
| `fdfd3d/closed-guide-energy` | 1 | 1.969013e-6 | 1.969013e-6 | 1.332268e-15 | 5.000000e-6 | yes |
| `fdfd3d/two-d-s-matrix` | 4 | 8.910855e-9 | 8.910858e-9 | 3.431558e-15 | 5.000000e-8 | yes |
| `fdfd3d/qmr-direct` | 2 | 4.754588e-11 | 4.754588e-11 | 3.644428e-18 | 1.000000e-9 | yes |
| `fdfd3d/qmr-symmetric-direct` | 1 | 6.854526e-12 | 6.854496e-12 | 3.039724e-17 | 1.000000e-9 | yes |
| `fdfd3d/ldlt-lu` | 2 | 1.206877e-15 | 1.597722e-15 | 3.908445e-16 | 1.000000e-10 | yes |
| `fdfd3d/pml-reflection-stretched` | 2 | 3.552796e-6 | 3.552796e-6 | 1.775309e-14 | 1.000000e-5 | yes |
| `fdfd3d/qmr-ilu-direct` | 1 | 2.286184e-10 | 2.286184e-10 | 3.460604e-18 | 1.000000e-8 | yes |
| `fdfd3d/gmres-multigrid-direct` | 2 | 2.025454e-10 | 2.025454e-10 | 6.607417e-18 | 1.000000e-8 | yes |
| `fdfd3d/block-qmr-ilu-s-matrix` | 1 | 1.145229e-10 | 1.145221e-10 | 7.482130e-16 | 1.000000e-9 | yes |
| `fdfd3d/block-qmr-convergence` | 1 | 0.100000 | 0.100000 | 0 | 0.200000 | yes |
| `fdfd3d/recycle-s-matrix` | 8 | 2.053668e-11 | 2.053668e-11 | 0 | 1.000000e-10 | yes |
| `fdfd3d/recycle-convergence` | 40 | 0.300000 | 0.300000 | 0 | 0.350000 | yes |
| `fdfd3d/recycle-adjoint` | 3 | 7.832697e-11 | 7.833638e-11 | 9.406253e-15 | 1.000000e-8 | yes |
| `fdfd3d/recycle-threads` | 6 | 0 | 0 | 0 | 0 | yes |
| `fdfd3d/multigrid-stencil-direct` | 4 | 3.230563e-10 | 3.230563e-10 | 1.571509e-17 | 1.000000e-8 | yes |
| `fdfd3d/multigrid-stencil-s-matrix` | 24 | 8.392363e-12 | 8.392626e-12 | 2.624310e-16 | 1.000000e-9 | yes |
| `fdfd3d/multigrid-stencil-threads` | 6 | 0 | 0 | 0 | 0 | yes |
| `fdfd3d/multigrid-stencil-refinement` | 9 | 1.100000 | 1.100000 | 0 | 1.500000 | yes |
| `fdfd3d/fill-seam` | 8 | 1.252389e-15 | 1.632632e-15 | 3.802433e-16 | 1.000000e-12 | yes |

All 27 pass. The 21 others (the port modes' eigensolves, QMR and block QMR without a
factorization, the matrix-free sweeps, the stencil's Fourier analyses, the fill fractions)
factorize nothing. The 2D cases that factorize (`fdfd/`, 28 of them) were run the same way and
pass too: the 2D solver keeps its one step of refinement after a `photonoxide-mixed` solve, which
the two adjoint-gradient cases need (without it their resolution, rounded up to a power of ten
from the solve's relative residual, came out a decade coarser: 1.6e-6 and 2.1e-6 against their
1e-6). Measured on the Core Ultra 7 265K, 6 threads, 2026-11-12.

## Cost

The guide of `photonoxide bench`'s `fdfd3d/guide-direct` (a 100 nm silicon guide in vacuum, PMLs
a quarter of the grid, a dipole beside it, 10 nm cells) on n³ grids, each mode in a process of
its own, on an Intel Core Ultra 7 265K with 64 GB under Windows, 6 threads, 2026-11-12. The
times start after the assembly (the averaged permittivity and the matrix, the same for both) and
include the analysis. Photonoxide's own: its factorization, then its solve with its one step of
refinement, whose field is the reference. Mixed: the analysis and the single-precision
factorization, then each iterate as it comes, with its field error against the reference
(relative, 2-norm); "to 1e-8" is the first iterate under 1e-8. The peak is the process's peak
working set, read from its handle after it exits. The factors are stored bytes: each front's
whole diagonal block and its panels, twice as many for photonoxide's own (the same fronts in
double). `MP_CELLS=n MP_MODE=double|mixed cargo test --release --lib
fdfd::three::mixed_tests::benchmark -- --ignored --nocapture`.

| Grid | Unknowns | Own: factorization | Own: total | Own: peak | Mixed: first solve (error) | Mixed: to 1e-8 | Mixed: converged (error) | Mixed: factors | Mixed: peak | Peak, mixed / own |
|---|---|---|---|---|---|---|---|---|---|---|
| 24³ | 41 472 | 0.54 s | 0.61 s | 0.44 GB | 0.38 s (9.9e-5) | 0.46 s | 0.61 s (1.1e-13) | 0.11 GB | 0.30 GB | 0.67 |
| 32³ | 98 304 | 1.81 s | 2.05 s | 1.38 GB | 1.26 s (3.7e-5) | 1.51 s | 2.01 s (1.0e-13) | 0.37 GB | 0.84 GB | 0.61 |
| 40³ | 192 000 | 5.85 s | 6.51 s | 3.08 GB | 3.61 s (3.0e-5) | 4.25 s | 5.50 s (4.4e-14) | 0.94 GB | 1.86 GB | 0.60 |
| 48³ | 331 776 | 13.42 s | 14.73 s | 6.69 GB | 7.99 s (1.5e-5) | 9.76 s | 11.53 s (2.8e-14) | 2.01 GB | 3.67 GB | 0.55 |
| 56³ | 526 848 | 30.95 s | 33.52 s | 12.00 GB | 17.84 s (2.4e-5) | 20.25 s | 25.06 s (2.8e-14) | 3.82 GB | 6.64 GB | 0.55 |

Every mixed run converged with no pivot perturbed and two or three GMRES iterations a step. The
peak falls towards the factors' half as they grow to dominate it (the matrix, its similarity and
the analysis are the rest); single precision's factorization, its first solve included, is also 1.4 to 1.7 times faster, so
the mixed solve reaches 1e-8 in 0.6 to 0.75 of photonoxide's own time and its converged field in
0.75 to 1.0.

## Limits

- **Ill-conditioned systems fail.** Photonoxide's own factors are the answer there; the error
  says so. A closed box at a wavelength thousands of cells long is one; a system near a
  resonance of a lossless cavity is where to expect another.
- **Each GMRES iteration is a solve with the factors.** On the guide it took two or three a
  step, on the closed box up to seven where it still converged; where single precision loses
  more, the refinement costs more, and
  the memory is what it buys.
- **Not chosen by `auto`:** a solve uses it only when its solver or job names it (the
  multigrid's coarsest level included, through `with_multigrid_on`).
- **The iterative solvers** (QMR, GMRES with the multigrid) are double precision throughout;
  single precision there, on the GPU, is the plan's next step ([the performance plan](../plans/performance.md),
  Phase D).

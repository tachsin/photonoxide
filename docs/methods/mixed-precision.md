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
it as in double, ε now single precision's (2⁻²³, so 3.5 × 10⁻⁴ ‖B‖₁): what makes the factors
safe to divide by, and what the refinement then has to undo. As photonoxide's own, the 3D
solver factorizes the complex symmetric similarity by L D Lᵀ when no pivot is perturbed, and A
by LU otherwise.

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

The 2D and 3D solvers don't refine a `photonoxide-mixed` solve again (their one step of
refinement after photonoxide's own factors would add nothing to η ≤ p u).

## Where it converges and where it can't

A closed box of 12³ cells of 50 nm (walls all round), a silicon cube 240 nm across in oxide, a
dipole beside it, at longer and longer wavelengths. The curl-curl operator's gradients see only
k₀² ε, and as the wavelength grows that falls below single precision's pivot threshold: the
factors perturb more and more pivots, the refinement needs more GMRES iterations, and from
60 µm (1 200 cells a wavelength) it fails and says so. Photonoxide's own double-precision
factors reach η ≤ p u on all of them. The field is against photonoxide's own (relative, 2-norm).
`cargo test --release --lib fdfd::three::mixed_tests::histories -- --ignored --nocapture`:

HISTORIES

The guide below (10 nm cells) is between the two: its first solve leaves η ≈ 10⁻⁴, and each
step takes some 25 GMRES iterations.

## Validation

The 3D validation cases that factorize, run with `photonoxide-mixed` in `auto`'s place
(`backend::prefer_direct`; `cargo test --release --test mixed_precision -- --ignored`), each
against its own Rule and tolerance, unchanged; the value photonoxide's own reports beside it:

VALIDATION

## Cost

BENCHMARK

## Limits

- **Ill-conditioned systems fail.** Photonoxide's own factors are the answer there; the error
  says so. A system near a resonance of a lossless cavity, or a closed box at a long
  wavelength, is where to expect it.
- **Time.** Each GMRES iteration is a solve with the factors, so where single precision loses
  much (the 10 nm guide's 10⁻⁴), the refinement costs more than it saves; the memory is what
  it buys.
- **Not chosen by `auto`,** and not used for the multigrid's coarsest level or any other solve
  that doesn't name it.
- **The iterative solvers** (QMR, GMRES with the multigrid) are double precision throughout;
  single precision there, on the GPU, is the plan's next step ([the performance plan](../plans/performance.md),
  Phase D).

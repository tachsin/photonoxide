# Solver backends

`photonoxide::backend` is the one place the solvers ask for a sparse direct solve, so that
another library can stand in for photonoxide's own. It is safe Rust and links nothing: a backend
for an external library is registered at run time by whoever has it. The plan for those
libraries is tracked in [#185](https://github.com/tachsin/photonoxide/issues/185); this page is
what exists today ([#173](https://github.com/tachsin/photonoxide/issues/173)).

## What a backend is

Three traits:

- **`DirectSolver`** says what it can do (`capabilities`) and analyses a matrix's structure
  (`analyse`): the ordering and the symbolic factorization.
- **`Analysis`** factorizes any matrix of that structure (`factorize`). A sweep analyses once
  and factorizes at every point, as `Solver2d::reuse` and `Solver3d::reuse` do.
- **`Factorization`** solves with the matrix (`solve`) and with its transpose
  (`solve_transpose`, which the adjoints and QMR's preconditioner need), and reports what it
  took (`report`): the factors' entries and the peak memory if the library says, the pivots it
  perturbed, and the seconds of each phase.

A matrix is given by columns (`Matrix`: column starts, row indices, values), in one of two
forms: `General`, for an LU, or `Symmetric`, complex symmetric without conjugation, for an
L D Lᵀ. It may carry each unknown's position on its grid, which photonoxide's nested dissection
orders by. Every error is photonoxide's `Error`; nothing panics on a bad matrix or a right-hand
side of the wrong size.

`analyse` returns `None` for a matrix that can't be factorized in the form asked for (one given
as symmetric that isn't, or whose pivots can't stay on its diagonal). The solvers then ask for
the general form, as they always have for a system with a Bloch-periodic side.

## Capabilities

Each backend declares:

- its name, its library's version and licence;
- whether it factorizes complex matrices (photonoxide's are: one that doesn't can't be
  registered);
- whether it takes the symmetric form, and whether it solves with the transpose;
- how its threads are set: one, rayon's, or the library's own;
- whether a solve gives the same bits on every run and any number of threads.

## The backends there are

| Name | What | Forms | Deterministic |
|---|---|---|---|
| `photonoxide` | the multifrontal LU and L D Lᵀ of `crate::sparse`, the default and the reference | general, symmetric | yes |
| `faer` | faer's sparse LU in its own ordering, the baseline of [docs/baselines.md](../baselines.md) | general | not declared |

Both are always registered. `backend::register` adds another under its name;
`backend::register_unavailable` records one that was looked for and can't be had, with the
reason. `backend::direct_solvers` lists them all.

## Choosing one

A `Choice` is `auto` (what this machine's measurements say: see "What auto chooses"),
`photonoxide`, or a backend's name:

- **In the solvers:** `Solver2d::new_on` and `from_cells_on`, `Solver3d::new_on`, and
  `IterativeSolver3d::with_multigrid_on` for the multigrid's coarsest level. `new`,
  `from_cells` and `with_multigrid` are `auto`, as before. A solver reused for another
  wavelength keeps its backend.
- **In a job file,** for an `fdfd` job:

  ```toml
  [solver]
  direct = "photonoxide"   # auto (the default), photonoxide, or a backend's name
  ```

  the same in JSON and YAML. `job::check` and the run refuse, in the same words, a name nothing
  is registered under, a backend that isn't available (with its reason), and the setting on a
  job without a direct solve.
- **In the record:** the run's `Solver` event names the backend that solved and its version,
  and if the job named none, that `auto` chose it and why.

A named backend that isn't available is an error. It is never replaced by another.

## What auto chooses

`photonoxide::backend::auto` chooses the direct solver of a 2D or 3D solve that names none,
from the benchmark runner's records of the machine it runs on (`photonoxide bench --tier`, the
Benchmarks page). The rule, `auto::choose`, is a function of those records alone:

- **The records that count:** the problem's family (`fdfd2d` or `fdfd3d`) and form (general,
  or complex symmetric), on the thread count measured nearest the solve's, that ran and passed
  their accuracy check.
- **The nearest size:** each backend's record nearest the problem's unknowns, and within a
  factor of 4 of them, against photonoxide's own on that same problem. A problem far from
  every measured size is photonoxide's own.
- **A margin:** a backend is taken only if it was at least 10% faster than photonoxide's own
  there.
- **Never** a backend that isn't registered and available (its library gone, a failed smoke
  test), nor one that missed an accuracy check in that family and form on this machine. A run
  that didn't finish says nothing either way.
- **Memory:** a backend whose peak, scaled from its record to the problem's size (as the
  factors of a nested dissection grow: n log₂ n in 2D, n^(4/3) in 3D), is beyond the free
  memory is passed over for the next that fits.
- **No records:** photonoxide's own. The library alone, the tests and the validation report
  have none, so there `auto` is what it always was, to the bit.

The decision comes with its reason, which the run's record keeps:

```
pardiso 2025.2 (auto: 2.1 times faster than photonoxide's own at 120000 unknowns on 20 threads
on this machine, measured 2026-10-12)
```

A sweep decides once: a solver reused for another wavelength keeps its backend. A job that
names its backend (`[solver] direct = "pardiso"`) isn't `auto`, and its record says so, which
is how a result is pinned to one.

The studio gives `auto` the records when a job starts: those of this machine only, and only
the catalogue's direct problems, each run's time without its assembly. It looks for the
external libraries then too, once, if the job names one or the records hold a run of one.

Not chosen by `auto` yet: the iterative backends, the multigrid's coarsest level (photonoxide's
own unless named) and the mode solvers.

## What is checked

- photonoxide's own backend through the traits gives the bits of the direct calls: on a
  matrix, in both forms, and for the 2D and 3D solvers' fields and the multigrid's cycle.
- A backend that counts its calls shows that the 2D solver, the 3D solver, the multigrid's
  coarsest level and a whole `fdfd` job factorize and solve through the registry: one analysis
  for a sweep, one factorization per point, the transposed solves of an LU, and the same
  S-matrices.
- faer's LU as the backend gives the 2D solver's field to 1e-11 of photonoxide's own.
- The validation report is unchanged.

## Dense kernels for the fronts

photonoxide's multifrontal solver spends its time in four dense operations on each front, which
are BLAS's and LAPACK's. `backend::dense::DenseKernels` is those four, so that a vendor's
library can do them:

| The trait's | BLAS, LAPACK | Where in a front |
|---|---|---|
| `multiply` | `zgemm`, `zgemmt` | the Schur complement, an L D Lᵀ panel's updates |
| `solve_unit_lower` | `ztrsm` (left, lower, unit) | U₁₂ = L₁₁⁻¹ F₁₂ |
| `solve_upper_from_right` | `ztrsm` (right, upper) | L₂₁ = F₂₁ U₁₁⁻¹ |
| `solve_unit_lower_transposed_from_right` | `ztrsm` (right, lower, transposed, unit) | L D = F L⁻ᵀ |
| `lu` | `zgetrf` | the diagonal block, pivoting among its own rows |

- **What stays photonoxide's:** the ordering, the fronts and their assembly, the static
  pivoting (a pivot below √ε ‖B‖₁ is found after the library's LU and the block redone with it
  replaced, as with faer's), and the scheduling of independent subtrees on rayon's threads.
  So the fill is the same whatever the kernels.
- **Threads.** Each call says whether it may thread: a front near the root may, the many small
  fronts factorized side by side may not. A library must be able to follow that per call.
- **A solver per library.** `dense::with_kernels` makes photonoxide's solver with a library's
  kernels, named `photonoxide-` and the kernels' name (`photonoxide-openblas`,
  `photonoxide-mkl`, `photonoxide-accelerate`), registered and chosen as any other backend:
  `direct = "photonoxide-openblas"` in a job. It declares itself deterministic only if the
  kernels do.
- **`photonoxide` itself is unchanged:** it calls faer's kernels as it always did, and gives
  the same bits. `dense::Faer` is those kernels behind the trait.
- **A block** is a matrix by columns inside a front's storage: its rows, its columns, the
  distance between its columns, and its first entry's address. photonoxide forbids `unsafe`,
  so reading through the address is `photonoxide-native`'s to do.

`photonoxide-native` loads the libraries (`native/src/blas.rs`): OpenBLAS, oneMKL's `mkl_rt`
and Apple Accelerate, by the Fortran interface with 32-bit integers.

- A build with 64-bit integers isn't used: OpenBLAS's has other names, oneMKL's interface layer
  is asked.
- `zgemmt` (the product into one triangle) isn't standard; where a library lacks it, `zgemm`
  computes both triangles.
- OpenBLAS's `zgemmt` is used from 0.3.27: with 0.3.26's (Ubuntu 24.04's package) the solver
  crashed.
- Accelerate is called by the names it has always had (`zgemm_`). Its newer interface's
  `zgemm$NEWLAPACK` crashed on macOS 26.6.2 for some products with an odd number of rows
  (1365 × 1024, 5461 × 4096), through Apple's own `cblas_zgemm$NEWLAPACK` too.
- The threads of a call are set for the calling thread: `mkl_set_num_threads_local`,
  `openblas_set_num_threads_local` (OpenBLAS 0.3.27 and later; an older one is held to one
  thread). Accelerate has no such setting and is left to itself.

Checked:

- faer's kernels behind the trait give the solver's own bits, on general and symmetric
  systems, with fronts large enough to thread.
- Kernels written from the trait's words alone, one entry at a time, and writing NaN where a
  triangle isn't wanted, factorize to 1e-12 of faer's: the trait says all a library needs.
- Each library's kernels against faer's on random complex matrices of 32 to 4096, and the
  solver with them against photonoxide's own, where the library is installed
  (`native/tests/blas.rs`).

## What isn't here yet

The mode solvers' shift-and-invert still calls faer's LU directly. AMD's AOCL and Arm's
Performance Libraries aren't loaded as dense kernels yet (#186). Iterative solvers (#189, #190)
and eigensolvers get their own traits with their first backend.

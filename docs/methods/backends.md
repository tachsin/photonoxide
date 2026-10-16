---
title: "Sparse direct solves and their backends"
module: backend
summary: "The one place the solvers ask for a sparse direct solve: photonoxide's own multifrontal LU and L D Lᵀ, the default and the reference, or another library registered at run time, each declaring what it can do and checked against photonoxide's own."
order: 19.7
---

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

## The validation cases on an external backend

The validation report runs on photonoxide's own solvers. A backend's issue checks it on the same
cases with

```sh
cargo run -p photonoxide-native --release --example validate -- <backend> [case prefix ...]
```

which finds and registers the external libraries, wraps the backend to count its
factorizations, and runs every case twice:

- **On the backend:** the cases ask for `auto`, and `auto` is given records saying the backend
  is twice as fast as photonoxide's own at every size, in both families and both forms, so every
  direct solve of a 2D or 3D FDFD solver goes to it. A case that made no factorization doesn't
  solve through a backend (the mode solvers, FDTD on its own, the circuits) and isn't listed.
- **On photonoxide's own,** with no records, as the report runs.

It prints a Markdown table, each case's value from both runs and their difference, and fails if
a case on the backend misses its tolerance.

### MUMPS (#176)

Run on 2026-10-11 on one machine (Intel Core Ultra 7 265K, 20 threads):

- MUMPS 5.8.2, conda-forge's `mumps-seq` (its flang build, with oneMKL's BLAS), on Windows 11;
- MUMPS 5.9.1, which no package manager shipped yet, built from mumps-solver.org's source
  (sequential, gfortran 13.3 at `-O2`, the reference BLAS and LAPACK 3.12), on Ubuntu 24.04 in
  WSL;
- each as `mumps` and as `mumps-blr` at its default tolerance, 1e-8.

49 of the 255 cases factorize through the backend, and every one is within its tolerance in all
four runs. The table gives each case's tolerance, photonoxide's own value (the Windows run's),
and for each run the difference between the backend's value and photonoxide's own in that run;
the factorizations are the 5.8.2 `mumps` run's.

- Many cases' values are themselves an error (a residual, a difference from a reference or
  between two solvers), so their difference is that error's round-off.
- The 2D adjoint gradients differ by up to 1.8e-7: their value is the gradient's difference from
  finite differences, whose own error a solve's round-off moves. Still within 1e-6.
- `fdfd3d/block-qmr-convergence`, by up to 1.8e-6 of its 0.0226: the slope of block QMR's
  error against the direct solver's S-matrix, whose error at a tolerance of 1e-10 is near the
  direct solve's own round-off. Its tolerance is 0.2.
- A difference of 0 is a value the solve doesn't move at the digits the case keeps.

| Case | Factorizations | Tolerance | photonoxide | mumps 5.8.2 | mumps 5.9.1 | mumps-blr 5.8.2 | mumps-blr 5.9.1 |
|---|---|---|---|---|---|---|---|
| `fdfd/slab-reflection-ez` | 2 | 5e-05 | 0.058135 | 1.5e-13 | 4.2e-15 | 9.4e-14 | 8.3e-14 |
| `fdfd/slab-reflection-hz` | 2 | 5e-05 | 0.026844 | 8.2e-14 | 1.8e-14 | 5.6e-14 | 3.3e-14 |
| `fdfd/flux-conservation` | 12 | 1e-10 | 4.09964e-14 | 1.4e-14 | 1.4e-14 | 5.0e-15 | 6.5e-15 |
| `fdfd/pml-reflection` | 2 | 1e-05 | 2.5128e-06 | 1.5e-14 | 3.3e-15 | 8.3e-15 | 7.3e-15 |
| `fdfd/port-mode-te` | 1 | 0.0002 | 2.84795 | 0 | 0 | 0 | 0 |
| `fdfd/port-mode-tm` | 1 | 0.0002 | 2.05347 | 0 | 0 | 0 | 0 |
| `fdfd/straight-guide` | 2 | 1e-12 | 2.07733e-14 | 2.3e-15 | 3.7e-16 | 4.7e-15 | 1.7e-15 |
| `fdfd/reciprocity` | 2 | 1e-12 | 1.57991e-14 | 4.2e-15 | 3.5e-15 | 4.2e-17 | 9.5e-16 |
| `fdfd/step-reflection-te` | 1 | 2e-05 | 0.001164 | 2.6e-16 | 5.7e-17 | 3.2e-16 | 1.3e-16 |
| `fdfd/adjoint-gradient-ez` | 13 | 1e-06 | 5.33735e-08 | 8.2e-08 | 1.1e-07 | 8.3e-08 | 1.4e-07 |
| `fdfd/adjoint-gradient-hz` | 13 | 1e-06 | 1.92988e-07 | 2.2e-08 | 1.8e-07 | 7.3e-08 | 1.5e-07 |
| `fdfd3d/film-reflection-te` | 2 | 6e-05 | 0.058123 | 9.5e-14 | 4.3e-14 | 6.8e-14 | 2.0e-14 |
| `fdfd3d/film-reflection-tm` | 2 | 5e-05 | 0.026843 | 1.3e-14 | 2.2e-14 | 1.7e-14 | 3.6e-14 |
| `fdfd3d/flux-conservation` | 12 | 1e-10 | 1.72634e-14 | 3.9e-15 | 2.2e-14 | 3.5e-15 | 2.2e-14 |
| `fdfd3d/pml-reflection` | 2 | 1e-05 | 2.51275e-06 | 9.8e-15 | 4.3e-15 | 7.7e-15 | 7.6e-15 |
| `fdfd3d/two-d-agreement` | 4 | 1e-11 | 2.04821e-13 | 1.3e-14 | 3.0e-15 | 6.2e-15 | 3.8e-15 |
| `fdfd3d/straight-strip` | 1 | 1e-12 | 2.56315e-15 | 2.4e-16 | 2.0e-16 | 2.4e-16 | 2.5e-16 |
| `fdfd3d/adjoint-gradient` | 33 | 1e-07 | 3.40619e-10 | 1.0e-10 | 8.1e-11 | 1.4e-10 | 4.1e-11 |
| `fdfd3d/reciprocity` | 1 | 1e-12 | 1.16281e-15 | 9.6e-16 | 4.9e-16 | 1.1e-15 | 5.7e-17 |
| `fdfd3d/closed-guide-energy` | 1 | 5e-06 | 1.96901e-06 | 4.4e-15 | 1.3e-15 | 2.7e-15 | 2.2e-16 |
| `fdfd3d/two-d-s-matrix` | 4 | 5e-08 | 8.91085e-09 | 3.0e-15 | 5.7e-16 | 1.9e-15 | 1.8e-16 |
| `fdfd3d/qmr-direct` | 2 | 1e-09 | 4.75459e-11 | 1.2e-18 | 2.8e-18 | 2.4e-18 | 1.2e-18 |
| `fdfd3d/qmr-symmetric-direct` | 1 | 1e-09 | 6.85453e-12 | 9.5e-18 | 2.5e-17 | 8.4e-18 | 1.9e-17 |
| `fdfd3d/ldlt-lu` | 2 | 1e-10 | 1.20688e-15 | 2.3e-16 | 1.5e-16 | 4.1e-16 | 4.5e-16 |
| `fdfd3d/pml-reflection-stretched` | 2 | 1e-05 | 3.5528e-06 | 3.0e-16 | 1.2e-14 | 1.6e-15 | 4.9e-15 |
| `fdfd3d/qmr-ilu-direct` | 1 | 1e-08 | 2.28618e-10 | 3.3e-18 | 9.7e-20 | 3.2e-18 | 4.2e-18 |
| `fdfd3d/gmres-multigrid-direct` | 1 | 1e-08 | 2.02545e-10 | 1.1e-17 | 7.9e-18 | 1.7e-18 | 5.0e-19 |
| `fdfd3d/block-qmr-convergence` | 1 | 0.2 | 0.022596 | 5.0e-07 | 1.8e-06 | 3.3e-07 | 4.3e-07 |
| `fdfd3d/recycle-adjoint` | 2 | 1e-08 | 7.8327e-11 | 1.8e-16 | 9.2e-17 | 9.2e-17 | 1.3e-17 |
| `fdtd/fdfd-lossy` | 2 | 1e-10 | 2.28652e-14 | 7.0e-16 | 2.6e-16 | 2.8e-16 | 1.4e-15 |
| `fdtd/fdfd-cpml` | 2 | 0.0005 | 0.000347324 | 8.1e-18 | 2.9e-17 | 2.2e-17 | 1.8e-17 |
| `fdtd/spectrum` | 2 | 1e-10 | 6.45912e-13 | 2.3e-18 | 3.7e-17 | 2.5e-17 | 2.5e-17 |
| `fdtd/mode-source-fdfd` | 1 | 1e-10 | 2.52373e-14 | 8.3e-17 | 1.6e-16 | 8.5e-17 | 1.1e-17 |
| `fdtd/ade-fdfd` | 2 | 1e-10 | 3.20471e-14 | 1.7e-15 | 1.2e-15 | 3.2e-15 | 2.7e-15 |
| `fdtd/bloch-fdfd` | 2 | 1e-10 | 2.85684e-14 | 3.9e-15 | 3.2e-15 | 2.2e-15 | 3.4e-15 |
| `fdtd/monitor-flux` | 1 | 1e-09 | 1.18113e-11 | 7.7e-15 | 1.0e-14 | 2.3e-15 | 1.2e-14 |
| `fdtd/monitor-modes` | 1 | 1e-09 | 1.48767e-11 | 1.2e-16 | 1.8e-16 | 3.9e-16 | 1.3e-16 |
| `fdtd/monitor-guides` | 2 | 0.0001 | 5.79373e-05 | 1.3e-15 | 5.9e-16 | 1.1e-15 | 2.7e-16 |
| `fdtd/fdfd-band-2d` | 10 | 0.0001 | 3.19325e-05 | 3.1e-16 | 1.4e-16 | 5.1e-17 | 1.5e-16 |
| `fdtd/fdfd-band-2d-fine` | 10 | 1e-06 | 1.69671e-07 | 5.7e-17 | 6.7e-16 | 2.6e-16 | 1.8e-16 |
| `fdtd/fdfd-band-3d-strip` | 3 | 0.0005 | 0.000218685 | 6.2e-17 | 3.8e-17 | 2.8e-17 | 5.5e-17 |
| `fdtd/fdfd-band-3d-bend` | 3 | 0.0005 | 0.000185982 | 2.2e-16 | 5.1e-16 | 1.1e-17 | 8.5e-16 |
| `fdtd/fdfd-smoothed` | 10 | 0.3 | 0.848365 | 3.0e-15 | 1.3e-15 | 1.0e-15 | 1.7e-15 |
| `fdtd/adjoint-fdfd` | 2 | 1e-10 | 7.15623e-14 | 1.4e-15 | 3.1e-16 | 1.4e-15 | 5.4e-16 |
| `fdtd/adjoint-fdfd-cpml` | 2 | 5e-05 | 2.28496e-05 | 3.1e-15 | 2.1e-16 | 1.8e-15 | 9.6e-16 |
| `fdtd/adjoint-backward-mode` | 2 | 1e-10 | 5.39297e-13 | 0 | 0 | 0 | 0 |
| `components/ring-fsr-fdfd` | 12 | 0.5 | 46.6964 | 6.9e-12 | 2.2e-13 | 0 | 2.9e-12 |
| `components/mmi-fdfd` | 1 | 0.03 | 0.397813 | 0 | 0 | 0 | 0 |
| `compact/fdfd-ring-held-out` | 51 | 0.0002 | 4.47012e-05 | 0 | 0 | 0 | 0 |

No example in `examples/` makes a direct solve through a backend: they are mode solvers,
circuits and FDTD, and the PDK's `IterativeSolver3d` is used for its ports' modes only.

**MUMPS 5.9.** Its `zmumps_c.h` changed the structure after `infog`: the null space's
`deficiency` (an int before its three pointers) went, and `nb_singular_values` came after them,
so the version and everything after it are 8 bytes earlier than in 5.7 and 5.8, and the structure
is 10 920 bytes (computed by C's rules and checked with gcc's `offsetof` against all three
layouts). The control parameters photonoxide sets mean what they did, by the user guides of
5.8.2 and 5.9.1, but for two changes it now handles: 5.9 made UCFS the default block low-rank
variant (`ICNTL(36)` = 1), so `mumps-blr` sets UFSC (0) to factorize the same way on every
release; and 5.9 reads control parameters from the file `MUMPS_CONFIG_FILE` names before every
call, over the ones it was given, so a 5.9 isn't used while it is set. The Libraries workflow
builds 5.9.1 from source and runs the MUMPS tests on it, as it installs the packaged releases.

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
  `photonoxide-mkl`, `photonoxide-aocl`, `photonoxide-armpl`, `photonoxide-accelerate`),
  registered and chosen as any other backend, and by the benchmark's dense kernels' problems:
  `direct = "photonoxide-openblas"` in a job. It declares itself deterministic only if the
  kernels do.
- **`photonoxide` itself is unchanged:** it calls faer's kernels as it always did, and gives
  the same bits. `dense::Faer` is those kernels behind the trait.
- **A block** is a matrix by columns inside a front's storage: its rows, its columns, the
  distance between its columns, and its first entry's address. photonoxide forbids `unsafe`,
  so reading through the address is `photonoxide-native`'s to do.

`photonoxide-native` loads the libraries (`native/src/blas.rs`), each provider found by its own
rules: OpenBLAS (its own builds, conda-forge's, the distributions', and pip's
`scipy-openblas32`), oneMKL's `mkl_rt`, AMD AOCL (AOCL-BLAS and AOCL-LAPACK, which are AMD's
BLIS and libFLAME, or the two in one `aocl` library), Arm Performance Libraries (on Arm
processors) and Apple Accelerate (on macOS).

- **The interface is measured, not assumed.** Every spelling photonoxide knows is tried in turn:
  Fortran's (`zgemm_`, `zgemm`, `ZGEMM`, with a prefix such as the wheels' `scipy_`, or a 64-bit
  suffix, `zgemm_64_`, `zgemm_64`, `zgemm64_`), then CBLAS and LAPACKE (`cblas_zgemm`,
  `LAPACKE_zgetrf_work`, with those suffixes). For the first a library has, `zgetrf` factorizes a
  1 × 1 matrix whose integers read 1 at either size, and the bytes it writes in its pivot and
  its `info` say whether it takes 32-bit integers or 64. A name is no proof: pip's
  `scipy-openblas64` exports `zgemm_` taking 64-bit integers, and is called so. How a Fortran
  function returns a complex number (by value, or through a first argument) is measured with
  `zdotc` the same way, with arguments either reading takes safely; on Windows on x86-64 the two
  are one convention. On Linux, Ubuntu's MKL 2020.4 returns through a first argument (Intel's
  convention, in its LP64 layer) and pip's OpenBLAS by value (gfortran's): both measured so. Every interface oneMKL 2026.1 has (Fortran's and C's, 32 and 64 bits) and
  each OpenBLAS build here agrees with faer's kernels to 1e-13.
- A library that has been called stays loaded for the rest of the process: unloading OpenBLAS
  while its threads waited crashed (an access violation on Windows).
- `zgemmt` (the product into one triangle) isn't standard; where a library lacks it, `zgemm`
  computes both triangles.
- OpenBLAS's `zgemmt` is used from 0.3.27: with 0.3.26's (Ubuntu 24.04's package) the solver
  crashed.
- Accelerate is called by the names it has always had (`zgemm_`). Its newer interface's
  `zgemm$NEWLAPACK` crashed on macOS 26.6.2 for some products with an odd number of rows
  (1365 × 1024, 5461 × 4096), through Apple's own `cblas_zgemm$NEWLAPACK` too.
- The threads of a call are set for the calling thread, from rayon's thread count:
  `mkl_set_num_threads_local`, and for AOCL and Arm PL, which thread by OpenMP,
  `omp_set_num_threads` on the calling thread, the control AMD's tuning guide gives AOCL-BLAS
  (without an OpenMP runtime AOCL-BLAS is held to one thread by `bli_thread_set_num_threads`).
  OpenBLAS has no count per thread: its `openblas_set_num_threads_local` sets the process's
  count, as `openblas_set_num_threads` does, and with rayon's threads setting 1 and 20 at once
  the solver ran 80 times slower and didn't repeat its bits. So OpenBLAS runs on one thread,
  and a threaded call sets the count around itself while no other call into OpenBLAS runs (a
  lock that the one-thread calls share). Its LU stays on one thread: its threaded `zgetrf`
  keeps arrays sized by the build's largest thread count on the stack at each level of its
  recursion, and overflowed a 2 MiB thread's stack with pip's and Ubuntu's builds on Linux.
  Accelerate has no such setting and is left to itself.
- **Reproducibility** is each library's own, recorded in its kernels' description: oneMKL's
  conditional numerical reproducibility (`mkl_cbwr_set`, which photonoxide sets to `AUTO`) gives
  the same bits on every run at one thread count, not on another; OpenBLAS promises nothing.
  No library's kernels are declared deterministic; faer's stay the deterministic default.
  What was measured is in [the baselines](../baselines.md#06-the-fronts-dense-kernels-from-onemkl-and-openblas-186).

Checked:

- faer's kernels behind the trait give the solver's own bits, on general and symmetric
  systems, with fronts large enough to thread.
- Kernels written from the trait's words alone, one entry at a time, and writing NaN where a
  triangle isn't wanted, factorize to 1e-12 of faer's: the trait says all a library needs.
- Each library's kernels against faer's on random complex matrices of 32 to 4096, each of its
  interfaces on its own, the solver with them against photonoxide's own on Helmholtz matrices
  and on the exported systems (the same fill and pivots, the solutions within 1e-10), and
  whether they repeat their bits, where the library is installed (`native/tests/blas.rs`).
- The interfaces' detection in CI, against the test library's own BLAS and LAPACK
  (`native/testlib`): Fortran's names taking 64-bit integers, a complex result through a first
  argument, and the C interfaces with 32-bit integers, each found as it is and agreeing with
  faer's kernels.

## Iterative backends

`backend::IterativeSolver` is the 3D iterative solver's Krylov method run elsewhere, a GPU's,
on the same recurrences as photonoxide's own (#190):

- **What a backend runs:** one run of QMR from x₀ = 0 (`qmr_run`): the complex symmetric QMR
  (R. W. Freund, SIAM J. Sci. Stat. Comput. 13, 425 (1992), doi:10.1137/0913023), one product
  an iteration, on a matrix given in the symmetric form, the general QMR (R. W. Freund, N. M.
  Nachtigal, Numer. Math. 60, 315 (1991), doi:10.1007/BF01385726) otherwise; the general QMR
  on A M⁻¹ with photonoxide's ILU(0) factors (`qmr_run_ilu`); and, if it says so
  (`runs_multigrid`), restarted GMRES with one cycle of photonoxide's multigrid as M⁻¹
  (`gmres_multigrid`; Y. Saad, *Iterative Methods for Sparse Linear Systems*, 2nd ed., SIAM
  (2003), doi:10.1137/1.9780898718003, Algorithm 9.5 restarted as 6.11), the cycle handed over
  as each level's operator, smoother factors and transfers, and the coarsest level's
  factorization (`MultigridCycle`).
- **What stays photonoxide's:** the restarts after a breakdown of the Lanczos process, the
  tightened tolerance of QMR on the symmetric similar matrix, x = M⁻¹ y after ILU(0), and the
  check of GMRES's solution on the residual computed on the CPU. A backend's solution is held to
  the same residual as photonoxide's own.
- **Choosing one:** `IterativeSolver3d::with_iterative_backend(&Choice)`, and in the
  catalogue each problem that takes one (`Entry::takes_an_iterative_backend`: the guides by
  QMR, ILU(0) or the multigrid, Diel's guide and the strip with ports). A solver without the
  matrix can't be given one. With a backend named, an S-matrix's ports and a block solve run
  one right-hand side at a time on it, and recycling's solves go to it as plain solves.
- **`auto`** is photonoxide's own, unless the process prefers a backend
  (`backend::prefer_iterative`, which nothing in photonoxide calls: the validation run on a
  backend does). Then the solves it runs as photonoxide would go to it (one right-hand side, by
  the matrix: plain QMR, QMR with ILU(0)'s exact solves, GMRES with the multigrid if it runs it
  and the hierarchy has every level's matrix and Galerkin's coarse operators), and the others
  stay photonoxide's: block QMR, GCRO-DR's recycling, and every solve without the matrix. The
  benchmark's measurements don't choose an iterative backend yet.

### cuSPARSE: photonoxide's QMR and GMRES on an NVIDIA GPU

`photonoxide-native` registers `cusparse` (`native/src/gpu_qmr.rs`, `gpu_multigrid.rs`) when it
finds the CUDA runtime and cuSPARSE and its QMR, run to 1e-12, matches photonoxide's direct
solve on the smoke test's systems in both forms.

- **On the GPU:** the matrix (and Aᵀ, by rows, for the general QMR), ILU(0)'s triangles and
  their transposes, or the multigrid's levels above the coarsest; the Krylov vectors, between
  iterations too. Only scalars come back each iteration: the dot products, the norms. The
  coarsest level's solve stays photonoxide's, on the CPU (at most 2 000 unknowns, 32 kB each
  way, 5 ms of a 1.6 s solve at 68³).
- **cuSPARSE's calls:** `cusparseSpMV` for the products and the multigrid's transfers,
  `cusparseSpSV` for the triangular solves (ILU(0)'s and the smoothers' block ILU(0), whose
  independent blocks its analysis finds), `cusparseAxpby` and `cusparseSpVV` for the vector
  work, each vector seen as a sparse vector with every entry. cuSPARSE's ILU(0) factorization
  is deprecated, and isn't used: the factors are photonoxide's.
- **Memory:** the GPU's free memory is checked against what a run keeps there before
  anything is copied, and a run that wouldn't fit is refused, saying how much it needs.
- **Determinism, declared:** yes, on one GPU. The sums are cuSPARSE's, in an order of its own,
  so the iterations are the CPU's to rounding's effect on the residual's history, not bit for
  bit; a repeated run gives the same bits, which the tests check on every guide. The products
  are `CUSPARSE_SPMV_CSR_ALG2`'s, chosen for it: on 50 repeated products, it gave the same bits
  on the curl-curl matrix of `strip-40` (192 000 unknowns) and on one with rows of 100 000
  entries, where cuSPARSE's default, `CSR_ALG1`, gave different bits; `CSR_ALG2` took 0.367 ms a
  product on the curl-curl matrix against `CSR_ALG1`'s 0.342 ms (RTX 4060, cuSPARSE 12.7.3).
- **Not on the GPU:** block QMR, GCRO-DR, and any solve without the matrix: the matrix-free
  QMR, and the multigrid without its finest matrix coming in 0.6 (#351), whose cycle the backend
  can't be given (#337).

### The validation cases on cusparse

```sh
cargo run -p photonoxide-native --release --example validate -- cusparse
```

prefers the backend for `auto` and counts its runs. Run on 2026-10-11 on an RTX 4060 (8 GB,
CUDA 13.1, cuSPARSE 12.7.3) beside a Core Ultra 7 265K (20 threads), Windows 11: 10 of the 257
cases solve through it, and every one is within its tolerance. The rest make no QMR or GMRES
solve it runs: the 2D and direct solves, the mode solvers, FDTD, and block QMR's and recycling's
own runs. Several of these cases' values are themselves the difference from a direct solve, so
their difference here is two iterative solves' rounding at a tolerance of 1e-9 to 1e-12.

| Case | Runs | photonoxide | cusparse | Difference | Tolerance |
|---|---|---|---|---|---|
| `fdfd3d/qmr-direct` | 2 | 4.75459e-11 | 5.37127e-11 | 6.2e-12 | 1e-09 |
| `fdfd3d/qmr-symmetric-direct` | 1 | 6.85453e-12 | 5.84424e-12 | 1.0e-12 | 1e-09 |
| `fdfd3d/qmr-ilu-direct` | 2 | 2.28618e-10 | 1.68686e-10 | 6.0e-11 | 1e-08 |
| `fdfd3d/gmres-multigrid-direct` | 1 | 2.02545e-10 | 2.02546e-10 | 8.5e-17 | 1e-08 |
| `fdfd3d/block-qmr-s-matrix` | 10 | 2.01599e-11 | 1.84858e-11 | 1.7e-12 | 1e-09 |
| `fdfd3d/block-qmr-ilu-s-matrix` | 16 | 1.14523e-10 | 1.14523e-10 | 2.9e-16 | 1e-09 |
| `fdfd3d/recycle-s-matrix` | 24 | 2.05367e-11 | 1.88577e-11 | 1.7e-12 | 1e-10 |
| `fdfd3d/recycle-convergence` | 80 | 0.233010 | 0.233010 | 0 | 0.35 |
| `fdfd3d/recycle-adjoint` | 2 | 7.8327e-11 | 7.8327e-11 | 0 | 1e-08 |
| `fdfd3d/qmr-matrix-free-sweep` | 8 | 2.44037e-11 | 2.42948e-11 | 1.1e-13 | 1e-09 |

The block, recycling and matrix-free cases are listed for the plain solves they also
make, which went to the GPU: their references, their one-port runs, and the solves for the
correction after recycling's initial guess. Their block QMR, GCRO-DR and matrix-free solves
stayed photonoxide's.

### Measured: the 3D iterative problems

```sh
cargo run -p photonoxide-native --release --example gpu_iterative -- --rounds 3 fdfd3d-iterative/
```

runs each catalogue problem that takes an iterative backend on `cusparse` and on photonoxide's
own, alternating, and keeps each side's faster run. Measured on 2026-10-11 and 12 on an RTX 4060
(8 GB, CUDA 13.1, cuSPARSE 12.7.3) against a Core Ultra 7 265K on 20 threads, Windows 11, a
shared machine. A solve's time is everything the solver does for it: on the GPU the matrix,
ILU(0)'s factors or the multigrid's hierarchy copied up and analysed, the iterations, and the
field copied back, so a step's time counts the transfers. The error is each problem's own check,
the same for both sides: the field against a reference solve to 1e-12 (QMR + ILU(0) for plain
QMR, GMRES + multigrid for the others), the strip with ports' S-matrix against its exact one.
Every repeated GPU run gave the same error to the bit.

| Problem | Grid | Unknowns | Iterations, CPU / GPU | CPU solve (a step) | GPU solve (a step) | Speed-up | Error, CPU / GPU |
|---|---|---|---|---|---|---|---|
| `guide-qmr-20` | 20³ cells of 10 nm, PMLs of 5 | 24 000 | 1423 / 1394 | 0.334 s (0.23 ms) | 0.333 s (0.24 ms) | 1.00 | 2.0e-7 / 1.9e-7 |
| `guide-qmr-30` | 30³ cells of 10 nm, PMLs of 7 | 81 000 | 1706 / 1701 | 0.818 s (0.48 ms) | 0.862 s (0.51 ms) | 0.95 | 2.6e-7 / 2.1e-7 |
| `guide-ilu-20` | 20³ cells of 10 nm, stretched PMLs of 5 | 24 000 | 66 / 66 | 0.128 s (1.94 ms) | 0.257 s (3.90 ms) | 0.50 | 3.4e-7 / 3.4e-7 |
| `guide-ilu-30` | 30³ cells of 10 nm, stretched PMLs of 7 | 81 000 | 130 / 130 | 0.887 s (6.82 ms) | 0.650 s (5.00 ms) | 1.36 | 5.1e-7 / 5.1e-7 |
| `guide-ilu-44` | 44³ cells of 10 nm, stretched PMLs of 10 | 255 552 | 251 / 250 | 5.418 s (21.59 ms) | 1.942 s (7.77 ms) | 2.79 | 9.1e-7 / 8.7e-7 |
| `guide-ilu-68` | 68³ cells of 10 nm, stretched PMLs of 14 | 943 296 | 560 / 559 | 43.33 s (77.37 ms) | 8.862 s (15.85 ms) | 4.89 | 2.0e-6 / 1.7e-6 |
| `guide-multigrid-20` | 20³ cells of 10 nm, stretched PMLs of 5 | 24 000 | 20 / 20 | 0.093 s (4.66 ms) | 0.142 s (7.09 ms) | 0.66 | 1.7e-6 / 1.7e-6 |
| `guide-multigrid-30` | 30³ cells of 10 nm, stretched PMLs of 7 | 81 000 | 19 / 19 | 0.272 s (14.32 ms) | 0.458 s (24.09 ms) | 0.59 | 4.2e-6 / 4.2e-6 |
| `guide-multigrid-44` | 44³ cells of 10 nm, stretched PMLs of 10 | 255 552 | 16 / 16 | 1.148 s (71.77 ms) | 0.853 s (53.29 ms) | 1.35 | 2.9e-6 / 2.9e-6 |
| `guide-multigrid-68` | 68³ cells of 10 nm, stretched PMLs of 14 | 943 296 | 22 / 22 | 2.423 s (110.2 ms) | 1.866 s (84.80 ms) | 1.30 | 3.5e-6 / 3.5e-6 |
| `diel-ilu-30` | 30 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 648 000 | 1307 / 1285 | 69.78 s (53.39 ms) | 12.73 s (9.91 ms) | 5.48 | 7.9e-7 / 4.9e-7 |
| `diel-ilu-80` | 80 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 1 728 000 | 1656 / 1620 | 241.2 s (145.6 ms) | 41.68 s (25.73 ms) | 5.79 | 7.8e-7 / 1.1e-6 |
| `diel-multigrid-30` | 30 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 648 000 | 44 / 44 | 3.282 s (74.59 ms) | 1.934 s (43.96 ms) | 1.70 | 2.9e-6 / 2.9e-6 |
| `diel-multigrid-80` | 80 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 1 728 000 | 51 / 51 | 14.96 s (293.4 ms) | 5.149 s (101.0 ms) | 2.91 | 2.1e-6 / 2.1e-6 |
| `strip-ports-ilu-40nm` | 36 × 51 × 41 cells of 40 nm, stretched PMLs of 8 | 225 828 | 875 / 851 | 33.45 s (38.23 ms) | 5.178 s (6.09 ms) | 6.46 | 1.7e-9 / 1.8e-9 |
| `strip-ports-ilu-25nm` | 58 × 82 × 66 cells of 25 nm, stretched PMLs of 13 | 941 688 | 1485 / 1499 | 152.0 s (102.4 ms) | 22.66 s (15.12 ms) | 6.71 | 2.9e-9 / 3.2e-9 |
| `strip-ports-multigrid-40nm` | 36 × 51 × 41 cells of 40 nm, stretched PMLs of 8 | 225 828 | 56 / 56 | 4.031 s (71.99 ms) | 1.650 s (29.46 ms) | 2.44 | 8.8e-9 / 8.8e-9 |
| `strip-ports-multigrid-25nm` | 58 × 82 × 66 cells of 25 nm, stretched PMLs of 13 | 941 688 | 55 / 55 | 10.76 s (195.5 ms) | 4.378 s (79.60 ms) | 2.46 | 1.3e-8 / 1.3e-8 |
| `strip-ports-qmr-40nm` | 36 × 51 × 41 cells of 40 nm, PMLs of 8 | 225 828 | 1529 / 1530 | 2.714 s (1.78 ms) | 1.323 s (0.86 ms) | 2.05 | 1.2e-10 / 1.1e-10 |
| `strip-ports-qmr-25nm` | 58 × 82 × 66 cells of 25 nm, PMLs of 13 | 941 688 | 2753 / 2774 | 23.94 s (8.70 ms) | 11.61 s (4.18 ms) | 2.06 | 7.4e-12 / 1.6e-11 |

The strip with ports' iterations are its two ports' runs together, its error its S-matrix's
against the exact one. What they say:

- **The same answers.** Every GPU run is within its problem's tolerance (1e-5), its error the
  CPU's or within a factor of 2.2 of it (`strip-ports-qmr-25nm`, at 1e-11). GMRES with the
  multigrid takes the CPU's iterations exactly, and its error agrees to two digits; QMR's
  iterations differ by up to 2.8% (`strip-ports-ilu-40nm`: 875 against 851), its Lanczos
  process summed in another order over a thousand steps.
- **Faster where the problem is large.** QMR with ILU(0) is 2.8 to 6.7 times faster from a
  quarter of a million unknowns up: its triangular solves are sequential on the CPU and
  cuSPARSE's analysis finds their parallelism. GMRES with the multigrid is 1.3 to 2.9 times
  faster there, more on the long Diel guide and the strip than on the cube, whose CPU cycle
  threads well already. Below 10⁵ unknowns the GPU is at par or slower: the copies and the
  analysis are a fixed cost, and a step is too small to use it.
- **Plain QMR** gains twice on the strip with ports, at 2 × 10⁵ and 10⁶ unknowns, and nothing
  at 24 000 and 81 000 on the guide: a step is a product and a few vector updates, each a call
  with its latency, and a small matrix stays in the CPU's cache. The guide's larger sizes
  weren't run, their reference fields taking over an hour.
- **What wasn't run:** the matrix-free problems, which a backend can't be given (#337). A run
  that wouldn't fit in the GPU's 8 GB is refused, saying how much it needs.

### AmgX

NVIDIA's AmgX was examined for #190 and isn't a backend: its complex modes have GMRES, block
Jacobi and one aggregation AMG only, and its complex GMRES's Givens rotation drops a phase, so
it diverges on photonoxide's systems. [The plan](../plans/backends.md#amgx-examined-and-not-a-backend-190-2026-10-11)
has what was read and measured.

## What isn't here yet

The mode solvers' shift-and-invert still calls faer's LU directly. AMD's AOCL and Arm's
Performance Libraries are loaded as dense kernels, but neither has yet run with photonoxide:
their discovery and the interface checks are tested (the CI's test library has their
variants), the libraries themselves wait for an AMD and an Arm machine. AMD's AOCL-Sparse as an
iterative backend is #189; the multigrid without its finest matrix on the GPU is #337.
Eigensolvers get their own trait with their first backend.

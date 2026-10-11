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

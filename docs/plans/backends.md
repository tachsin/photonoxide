# External backends and the in-app benchmark

photonoxide's own solvers are pure Rust and always present. This plan lets it also use
libraries that users install themselves: Intel's, AMD's, Apple's, Arm's and NVIDIA's math
libraries, MUMPS and SuperLU. It also lets it show each user, on their own machine, which
library is faster and leaner for which problem. I decided its terms on 2026-10-05; the
issues are tracked in #185.

## Decisions

1. **Loaded at run time, optional.** Nothing is linked at build time. The default build, CI and
   the released program stay pure Rust and work with no external library installed. A library
   is found when the program runs, checked, and used only if asked for, by name or through
   `auto`.
2. **One crate calls C:** `photonoxide-native`, the only crate allowed `unsafe`. The library
   `photonoxide` keeps `#![forbid(unsafe_code)]`.
3. **No GPL libraries** (no UMFPACK, no GPL parts of SuiteSparse). GPL programs may still be run
   as external programs for comparison, as Meep may.
4. **Nothing is redistributed.** Users install the libraries; photonoxide records each one's
   licence and shows it before installing.
5. **Installs are guided.** The studio shows the licence and this platform's steps, and runs a
   package manager's command (winget, conda, apt, brew, vcpkg) in a terminal only after the user
   confirms. Elevation, if needed, is the package manager's to ask for.
6. **PETSc and SLEPc are out** (2026-10-05): they have no native Windows build, and the direct
   methods they would bring are the libraries below.

## The architecture

```
photonoxide (safe Rust, forbid(unsafe_code))
├── backend: DirectSolver / Analysis / Factorization, Capabilities, the registry, Choice   (#193)
│   ├── "photonoxide": the multifrontal LU and L D Lᵀ (crate::sparse), always present, the default
│   └── "faer": faer's sparse LU, the baseline, always present
├── later traits beside DirectSolver, each with its first backend:
│   DenseKernels (#186), an iterative solver (#189, #190)
└── bench: the problem catalogue (#179) and the runner (#180)

photonoxide-native (unsafe allowed, libloading; optional)                                   (#174)
├── discovery: environment variables, install folders, conda environments, library paths
├── loading: symbols, version, integer size, a smoke test against photonoxide's answer
└── backends: PARDISO (#175), MUMPS (#176), SuperLU (#177), BLAS/LAPACK (#186),
    Accelerate (#187), cuDSS (#188), AOCL-Sparse (#189), cuSPARSE (#190)

the studio
├── Libraries: found or not, versions, licences, guided installs                            (#181)
├── Benchmarks: runs on this machine, charts, which library wins where                      (#182)
└── auto: the backend this machine's measurements favour                                    (#183)
```

- **The registry.** `photonoxide::backend` (#193) has the traits and the registry, with
  `photonoxide` and `faer` always registered. `photonoxide-native` registers what it finds
  (`backend::register`), and records what it looked for and didn't find, or found broken, with
  the reason (`backend::register_unavailable`). Asking for a library by name says why it can't
  be had: a named backend that isn't available is an error, never a silent fallback.
- **The choice.** `auto`, `photonoxide`, or a backend's name: in the solvers' settings, in job
  files (TOML, JSON, YAML) and in the run record. `auto` is the direct solver measured fastest
  on this machine for the problem's kind and size, photonoxide's own without measurements
  (#183; [the method's page](../methods/backends.md)).
- **New traits come with their first backend,** not ahead of it: dense kernels with #186, an
  iterative solver with #189 or #190, an eigensolver if one is planned.

## The libraries

| Library | What it brings | Licence | Windows | Linux | macOS |
|---|---|---|---|---|---|
| oneMKL 2026.1: PARDISO (#175), BLAS/LAPACK (#186) | sparse direct; dense kernels | Intel Simplified Software Licence | winget `Intel.oneMKL`, conda-forge `mkl` | conda-forge, apt, oneAPI | conda-forge, Intel Macs only |
| MUMPS 5.8.2; 5.9.1 from source (#176) | sparse direct, multifrontal, block low-rank | CeCILL-C | conda-forge `mumps-seq` | conda-forge, apt | conda-forge |
| SuperLU 7.0.1 (#177) | sparse direct, partial pivoting | BSD | vcpkg (built from source) | conda-forge, apt | conda-forge, Homebrew |
| AMD AOCL 5.2: BLIS, libFLAME (#186), AOCL-Sparse (#189) | dense kernels; iterative solvers | BSD-3, MIT (to confirm per component) | AMD's installer | AMD's packages, Spack | none |
| Apple Accelerate (#186, #187) | dense kernels; sparse direct (complex LU from macOS 15.5, complex symmetric L D Lᵀ from macOS 26) | part of macOS | none | none | built in |
| Arm Performance Libraries (#186) | dense kernels | free, no licence | Windows on Arm | Linux AArch64 | Apple Silicon |
| OpenBLAS (#186) | dense kernels, the fallback | BSD | conda-forge, vcpkg | everywhere | conda-forge, Homebrew |
| NVIDIA cuDSS (#188) | sparse direct on the GPU | NVIDIA EULA | conda-forge, pip, NVIDIA | the same | none |
| NVIDIA cuSPARSE (#190) | GPU kernels for photonoxide's Krylov solvers and its multigrid's cycle | NVIDIA EULA | CUDA toolkit | CUDA toolkit | none |

The platforms were checked on 2026-10-05 from the package indexes. #184 verifies each install
method, keeps them as data, and generates the user's guide from it.

Not used: cuSOLVER's sparse modules (`cusolverSp`, `cusolverRf`), deprecated in CUDA 13 in favour
of cuDSS; cuSPARSE's ILU(0) factorization (`csrilu02`), deprecated likewise; and NVIDIA's AmgX,
below.

### AmgX: examined, and not a backend (#190, 2026-10-11)

[AmgX](https://github.com/NVIDIA/AMGX) is NVIDIA's algebraic multigrid and Krylov library for the
GPU. Its licence is BSD-3-Clause (its repository's `LICENSES/BSD-3-Clause.txt`), so it could have
been one. It can't solve photonoxide's systems, which are complex, non-Hermitian and indefinite:

- **What its complex modes have.** In v2.5.0 (2025-12-21) and on its main branch (2026-07-01), a
  complex mode (`dZZI`, `dZCI`, `dCCI`) registers GMRES, block Jacobi, and the aggregation AMG
  with one selector (`SIZE_8`), one Galerkin product (`LOW_DEG`) and the V cycle, and nothing
  else (`registerClasses<T_Config, true>` in `src/core.cu`): no BiCGStab, no FGMRES, no
  ILU or Gauss–Seidel smoother, no classical AMG, and no dense LU for the coarsest level, whose
  complex build is commented out. Its README calls the complex support "currently limited".
- **Its GMRES is wrong for them.** Its complex Givens rotation (`GeneratePlaneRotation<…, true>`
  in `src/solvers/gmres_solver.cu`) takes c and s from |h_ii| and |h_i+1,i| alone, dropping
  h_ii's phase, so it doesn't zero the subdiagonal it then sets to zero, and the least-squares
  update is wrong whenever h_ii isn't a positive real number: nearly always, for a non-Hermitian
  matrix.
- **Measured.** Built from its main branch here as `amgxsh.dll` (CUDA 13.1, MSVC 14.29,
  sm_89) and loaded at run time by a throwaway program, outside the repository, that handed it
  the systems photonoxide gives an iterative backend, to a relative residual of 1e-8 in at most
  3000 iterations of GMRES(40), on an RTX 4060:
  - a complex control, (1 + i) times the 7-point Laplacian on 40³: solved, in 19 iterations with
    block Jacobi and 13 with its AMG, which divide the 1 + i out and leave h_ii real and
    positive; unpreconditioned, where h_ii keeps its phase of 45°, it stalled at 9e-5. The build
    and the complex mode work; the rotation is what fails.
  - the guide of `fdfd3d-iterative/guide-qmr-20` (24 000 unknowns, the curl-curl operator with
    PMLs, as QMR has it): the true residual grew to 1e33 unpreconditioned, 6e13 with block
    Jacobi and 4e14 with its AMG. Restarted GMRES never lets the residual grow; this one does.
  - the same guide on Shin and Fan's operator with stretched PMLs (`guide-multigrid-20`, what
    ILU(0) and the multigrid precondition): 4e21 unpreconditioned, 1.5e-5 with block Jacobi and
    1.2e-4 with its AMG after 3000 iterations, where photonoxide's GMRES with its multigrid
    takes 20 to 1e-8. Its AMG's coarsening stalled at about 2 350 rows of the 24 000.
  - with GMRES as the coarsest level's solver, nothing converged, the Laplacian control included.

So AmgX isn't loaded. Should a release fix its complex GMRES and add a complex smoother that
holds on indefinite systems, this is the place to look again; its GPU AMG would still be an
algebraic one on a curl-curl operator, which photonoxide's geometric multigrid exists for.

## Rules for backends

- **Checked before offered.** A backend is registered only after its smoke test: a small
  complex system solved and compared with photonoxide's answer. Each backend's issue also
  checks it on the exported systems (`photonoxide bench --export`) and on the validation cases
  it applies to (`cargo run -p photonoxide-native --release --example validate -- <backend>`,
  which puts the backend in `auto`'s place and sets each case beside photonoxide's own run). The
  validation report itself runs on photonoxide's own solvers; backend runs are reported apart,
  in [the backends' write-up](../methods/backends.md#the-validation-cases-on-an-external-backend).
- **Equal accuracy.** A backend's speed counts only at the accuracy photonoxide's own reaches;
  the benchmark checks every run's answer.
- **Determinism, declared.** photonoxide's own solvers give the same bits on any thread count.
  A backend declares in its `Capabilities` whether it repeats bit for bit, and under which
  setting (MKL's conditional numerical reproducibility, PARDISO's `iparm[33]`). The benchmark
  reports it, and `auto` (#183) can be told to prefer reproducible backends.
- **Contained.**
  - Each `unsafe` call is in one small function that states what the C side requires.
  - No panic crosses into C, and library errors become `Error`.
  - Versions whose structures photonoxide doesn't know (MUMPS's, SuperLU's options) are
    refused, not guessed.
  - A library that aborts can't be caught in-process, so the benchmark runs every backend in
    a child process.
- **CI has no external libraries.** `photonoxide-native` is tested in CI against a small
  test library, a `cdylib` crate in the workspace that exports a C interface. The real
  libraries' tests skip when the library isn't found, and say so. GPU backends are tested on
  tachsin's machine before releases, as FDTD's GPU kernel will be (#166).

## The benchmark

- **The catalogue (#179):** problem families at many sizes, from 10⁴ unknowns to the machine's
  memory: 2D and 3D FDFD, the iterative 3D problems, the mode solvers' eigenproblems,
  circuits and dense kernels. Each problem has its task, an estimate of its memory, and an
  accuracy check. It comes in tiers: `quick`, `standard` and `full`.
- **The runner (#180):** each problem on each available backend and thread count, in a child
  process. A record keeps time per phase, peak memory, factor entries, the accuracy check and
  determinism, in a results database tagged with the machine and the libraries' versions. It
  replaces the external drivers used for docs/baselines.md and #159.
- **In the studio (#182):** a user runs it on their own machine and sees time and memory
  against size per library, the speed-ups, and which library wins for which problem. `auto`
  (#183) then chooses from those measurements, and says why.

## Order

1. This plan, and AGENTS.md's rule (#172).
2. The traits and the registry (#193, done). `photonoxide-native`'s machinery (#174).
3. The libraries, in parallel (#175, #176, #177, #186, #187, #188, #189, #190), with the
   install guides (#184: [docs/libraries.md](../libraries.md), each method run on clean machines
   by the Libraries workflow, and each library's new releases read every week by the Releases
   workflow, which keeps them in one issue until they are checked:
   [docs/libraries.md, New releases](../libraries.md#new-releases)).
4. The catalogue (#179) and the runner (#180).
5. The studio's Libraries (#181) and Benchmarks (#182) pages, and `auto` (#183).

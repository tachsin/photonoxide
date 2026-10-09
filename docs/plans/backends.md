# External backends and the in-app benchmark

photonoxide's own solvers are pure Rust and always present. This plan lets it also use
libraries that users install themselves: Intel's, AMD's, Apple's, Arm's and NVIDIA's math
libraries, MUMPS and SuperLU. It also lets it show each user, on their own machine, which
library is faster and leaner for which problem. The owner decided its terms on 2026-10-05; the
issues are tracked in #185.

## The owner's decisions

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
    Accelerate (#187), cuDSS (#188), AOCL-Sparse (#189), cuSPARSE and AmgX (#190)

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
  files (TOML, JSON, YAML) and in the run record. Until #183, `auto` is photonoxide's own.
- **New traits come with their first backend,** not ahead of it: dense kernels with #186, an
  iterative solver with #189 or #190, an eigensolver if one is planned.

## The libraries

| Library | What it brings | Licence | Windows | Linux | macOS |
|---|---|---|---|---|---|
| oneMKL 2026.1: PARDISO (#175), BLAS/LAPACK (#186) | sparse direct; dense kernels | Intel Simplified Software Licence | winget `Intel.oneMKL`, conda-forge `mkl` | conda-forge, apt, oneAPI | conda-forge, Intel Macs only |
| MUMPS 5.8.2 (#176) | sparse direct, multifrontal, block low-rank | CeCILL-C | conda-forge `mumps-seq` | conda-forge, apt | conda-forge |
| SuperLU 7.0.1 (#177) | sparse direct, partial pivoting | BSD | vcpkg (built from source) | conda-forge, apt | conda-forge, Homebrew |
| AMD AOCL 5.2: BLIS, libFLAME (#186), AOCL-Sparse (#189) | dense kernels; iterative solvers | BSD-3, MIT (to confirm per component) | AMD's installer | AMD's packages, Spack | none |
| Apple Accelerate (#186, #187) | dense kernels; sparse direct (complex from macOS 15.5, to confirm) | part of macOS | none | none | built in |
| Arm Performance Libraries (#186) | dense kernels | free, no licence | Windows on Arm | Linux AArch64 | Apple Silicon |
| OpenBLAS (#186) | dense kernels, the fallback | BSD | conda-forge, vcpkg | everywhere | conda-forge, Homebrew |
| NVIDIA cuDSS (#188) | sparse direct on the GPU | NVIDIA EULA | conda-forge, pip, NVIDIA | the same | none |
| NVIDIA cuSPARSE, AmgX (#190) | GPU kernels for photonoxide's Krylov solvers; GPU multigrid | NVIDIA EULA; AmgX open source (to confirm) | CUDA toolkit | CUDA toolkit | none |

The platforms were checked on 2026-10-05 from the package indexes. #184 verifies each install
method, keeps them as data, and generates the user's guide from it.

Not used: cuSOLVER's sparse modules (`cusolverSp`, `cusolverRf`), deprecated in CUDA 13 in favour
of cuDSS; cuSPARSE's ILU(0) factorization (`csrilu02`), deprecated likewise.

## Rules for backends

- **Checked before offered.** A backend is registered only after its smoke test: a small
  complex system solved and compared with photonoxide's answer. Each backend's issue also
  checks it on the exported systems (`photonoxide bench --export`) and on the validation cases
  it applies to. The validation report itself runs on photonoxide's own solvers; backend runs
  are reported apart.
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
  the owner's machine before releases, as FDTD's GPU kernel will be (#166).

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
   by the Libraries workflow).
4. The catalogue (#179) and the runner (#180).
5. The studio's Libraries (#181) and Benchmarks (#182) pages, and `auto` (#183).

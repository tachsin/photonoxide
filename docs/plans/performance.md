# Performance plan: kernels, threads, GPUs and distributed memory

*A survey and a plan, 2026-10-04. Nothing here is on the roadmap yet: the owner reviews it first.*

The owner asked for GPU support, MPI-style parallel algorithms and MKL-class kernel
optimizations, for where each applies, and for what PETSc and SLEPc (and the libraries around
them) have added lately that photonoxide could implement from the papers. This page answers in
five parts:

1. [Where the time goes](#1-where-the-time-goes): each solver, now and planned, what dominates
   its cost and which kind of parallelism pays.
2. [What the HPC libraries offer](#2-what-the-hpc-libraries-offer): recent PETSc and SLEPc
   releases and their neighbours, each method traced to its paper and to a photonoxide problem.
3. [The Rust side](#3-the-rust-side-checked-2026-10-04): GPU, distributed and kernel crates as of
   today, and where they meet design principles 6 (pure Rust) and 9 (determinism).
4. [A phased plan](#4-the-plan-in-phases), cheapest wins first, mapped onto the milestones.
5. [Decisions for the owner](#5-decisions-for-the-owner).

**Rules this page follows.**

- PETSc, SLEPc, MUMPS, STRUMPACK, hypre, Ginkgo, HPDDM and FEAST were used for inspiration only:
  their release notes, manuals and papers were read, never their source code. Every method below
  is cited by its primary paper, and would be implemented from that paper.
- Every DOI was checked on Crossref on 2026-10-04. The papers are listed in the papers folder's
  README under "Performance: GPU, distributed, kernels (plan)"; 28 open copies are in the folder,
  33 are still needed (see [References](#references)).
- **Measured** facts say where they were measured. **Estimates** are marked *(estimate)* with
  their reasoning; none of them is a result until a benchmark of [Section 4](#4-the-plan-in-phases)
  measures it.

**The machine** behind this survey's measurements (the 3D cost table in docs/methods/fdfd-3d.md
names the same CPU): an Intel Core Ultra 7 265K (20 cores), 64 GB of
memory, and an NVIDIA GeForce RTX 4060 with 8 GB (driver 616.56), read with `nvidia-smi` on
2026-10-04. Two vendor figures are used in estimates and labelled so: the CPU's rated memory is
dual-channel DDR5-6400, about 100 GB/s in theory (the installed modules' speed wasn't checked),
and the RTX 4060 has 272 GB/s of memory bandwidth and runs double precision at 1/64 of its single
precision rate.

## Summary

The largest gains for the least work are on the CPU, in code photonoxide already has:

1. **The 3D iterative solver runs at about a fifth of the machine's memory bandwidth** *(estimate
   from the measured 45 ns per unknown per QMR iteration, below)*. A matrix-free Yee operator,
   threads that persist, no allocation inside the iteration, and reductions in a fixed order would
   cut an iteration's time several-fold before any new algorithm.
2. **QMR does twice the work it needs** on the curl-curl formulation, the default. The 3D system
   scaled by the PML stretches, V A, is complex symmetric (docs/methods/fdfd-3d.md). With the code's own start (w₁ = v₁), QMR's two
   Lanczos sequences then coincide, so Freund's QMR for complex symmetric matrices (Freund 1992)
   gives the same kind of iterates with one product per iteration instead of two, and no
   transposed matrix to store.
3. **Independent solves are the cheapest parallelism, and photonoxide mostly runs them one after
   another.** The FDFD job's wavelength sweep is a sequential loop, and the mode solvers get
   *slower* with more threads (measured below: 8.0 s on one thread against 13.2 s on twenty for
   Hadley's corner problems). Running sweep points side by side, each on one thread, is the
   first parallelism to add, and it is deterministic by construction.
4. **A nested-dissection ordering** (George 1973) for faer's sparse LU, which uses COLAMD now:
   less fill for every direct solve, 2D FDFD, the mode solvers, 3D, and later the multigrid's
   coarsest grid and the subdomain solves of domain decomposition.
5. **The 0.4.1 multigrid** (on the `fdfd3d-multigrid` branch, not touched here) is the right next
   preconditioner; its parallel smoothing can be made bit-for-bit deterministic with sweep-based
   methods (Chow & Patel 2015; Anzt et al. 2015) or multicolour orderings (Saad 2003).
6. **The GPU first pays in FDTD** (0.5, already planned on wgpu in single precision): a
   bandwidth-bound stencil (Micikevicius 2009). On this machine its memory is about 2.7 times the
   CPU's *(vendor figures)*, so a GPU FDTD should be compared with a CPU FDTD that is
   cache-blocked (Malas et al. 2015), not a naive one.
7. **Distributed memory is a domain-decomposition question, not an MPI question.** Optimized
   restricted additive Schwarz with PML transmission conditions and a coarse space (St-Cyr et al.
   2007; Dolean et al. 2009; Bonazzoli et al. 2019; Bootland et al. 2021) runs the same way on
   threads, processes or machines. It needs a communicator trait whose default is pure Rust (an
   in-process and a TCP backend), with MPI as an optional backend, and subdomains fixed by the
   problem so that the answer doesn't depend on the number of ranks.
8. **MKL, PARDISO, MUMPS and PETSc as external benchmarks only,** run the way Meep is: never a
   dependency. *(Superseded on 2026-10-05: external libraries become optional backends loaded at
   run time, never linked, in [the backends plan](backends.md); PETSc is out.)*

## 1. Where the time goes

### What was measured

From photonoxide's own docs (each number with its source) and from runs on 2026-10-04 at commit
e458386 (the examples, release build, all on the machine above):

| Solver | Problem | Measured | Source |
|---|---|---|---|
| 2D FDFD, sparse LU | 440 × 340 grid, 150 k unknowns | analysis 55 ms, numerical factorization ≈ 750 ms, each further source 80 ms | docs/methods/fdfd.md |
| 3D FDFD, sparse LU | 16³ to 40³ cells, 12 k to 192 k unknowns | 40³: factorization 66 s, 28 GB; time ∝ N^1.9, memory ∝ N^1.6 | docs/methods/fdfd-3d.md, "Cost" |
| 3D FDFD, QMR | 864 k unknowns (Diel) | 2 745 iterations, 106 s, 1.4 GB: about 45 ns per unknown per iteration on 20 threads | docs/methods/fdfd-3d.md |
| 3D FDFD, QMR | strip with ports, 1.8 M unknowns | 3 504 iterations, 263 s (42 ns per unknown per iteration) | docs/methods/fdfd-3d.md |
| QMR + ILU(0) | 192 k / 864 k unknowns | four triangular solves 15.6 / 63 ms against 2.5 / 9.7 ms for a threaded product; level scheduling slower on every thread count | docs/methods/fdfd-3d.md |
| QMR + multigrid | 40³ guide; Diel | 18 iterations, 2.2 s + 4.9 s to build; 52 iterations, 17.6 s + 37 s to build | `fdfd3d-multigrid` branch, HANDOFF.md (unmerged) |
| Full-vector modes | `strip_waveguide`: three grids up to 421 × 301 nodes | 6.6 s on 1 thread, 7.4 s on 20 (repeat: 8.7, 9.3 on 4, 12.7 on 20) | this survey |
| Hadley's equations | `hadley_corners`: four corner problems on a series of grids | 8.0 s on 1 thread, 13.2 s on 20 (repeat: 9.0, 12.8 on 4, 12.6 on 20) | this survey |
| Modes over wavelength | `group_index` | 11.3 s on 1 thread, 13.5 s on 20 | this survey |
| Modes with a PML | `leaky_wire_benchmark`: up to 504 × 899 nodes | 56.9 s on 1 thread, 39.3 s on 20 | this survey |
| Hadley modes, interferometers | `mzi_dwivedi` | 45.3 s on 1 thread, 62.2 s on 20 | this survey |
| Circuits | `circuit_fit` | 0.56 s on 1 thread, 0.60 s on 20 | this survey |

Threads were set with `RAYON_NUM_THREADS`, which sets faer's parallelism too. Three facts about
the code explain part of these numbers:

- `krylov::product` (src/fdfd/krylov.rs) spawns one OS thread per core on *every* product and
  allocates its output each time; `dot` and `norm` are sequential.
- The FDFD job's wavelength sweep (src/job/fdfd.rs, the loops at lines 410 and 551 on main) solves
  one wavelength after another; the circuit's sweep (src/circuit/mod.rs) already runs its
  wavelengths in parallel with rayon and collects them in order.
- faer's sparse LU is called with its default parallelism, all of rayon's threads, on matrices
  whose factorizations take tens of milliseconds: there, threads cost more than they bring.

### Solver by solver

| Solver | What dominates | What pays, in order | Why, and roughly how much |
|---|---|---|---|
| **2D FDFD, direct** | the numerical factorization (≈ 93% at 150 k) | task farming of wavelengths and parameters; nested dissection; then threads inside one factorization | Each wavelength is independent, needs a few hundred MB, and runs best on one or a few threads: a sweep of k points on 20 cores runs ≈ min(k, 20) times faster *(estimate)*. On a regular 2D grid nested dissection orders the factorization in O(N^{3/2}) operations with O(N log N) fill (George 1973); how much it gains over COLAMD here must be measured. |
| **3D FDFD, direct** | factorization: 66 s and 28 GB at 192 k unknowns | nested dissection; mixed precision; compression (BLR) much later | 192 k unknowns is a 1.6 µm cube: a device doesn't fit. The direct solver's role in 3D is as a building block: the multigrid's coarsest grid, the subdomain solver of domain decomposition, sweeping's slabs. Single-precision factors halve the memory and, refined in double (Carson & Higham 2018; Amestoy et al. 2023), can keep double-precision accuracy for moderately conditioned systems. |
| **3D FDFD, QMR** | two sparse products and a dozen vector passes per iteration, all bound by memory bandwidth | SIMD and cache-aware kernels; the symmetric QMR; then the GPU; distributed only when one machine's memory runs out | *(estimate)* A product reads 13 nonzeros of 24 B (16 B complex value, 8 B index) per row, about 350 B with the vectors; two products and the vector updates make ≈ 0.9 KB per unknown per iteration, or ≈ 20 GB/s at the measured 45 ns: about a fifth of the CPU's rated bandwidth. A matrix-free Yee operator reads the permittivity at each edge and the field (≈ 50–60 B per unknown), and the symmetric QMR needs one product instead of two. Together, 3 to 8 times less time per iteration is plausible; the benchmark decides. |
| **3D preconditioner** | ILU(0): sequential triangular solves (4 to 6 times a product); multigrid: the hierarchy's build (Galerkin products, ILUs), then smoothing | threads with deterministic algorithms; later GPU and distributed with the same algorithms | ILU(0)'s setup and application can both run as a fixed number of parallel sweeps (Chow & Patel 2015; Anzt et al. 2015): every value depends only on the previous sweep, so the result doesn't depend on the thread count. Multigrid partitions by planes like the operator; the Galerkin product is row-parallel. |
| **Mode solvers, shift-invert Arnoldi** | the sparse LU of A − σI (2N unknowns, Hx and Hy), then back-substitutions | task farming over sweep points and over modes; nested dissection; contour-integral solvers when many modes are wanted | Measured above: one thread beats twenty on these sizes, so a sweep of wavelengths or widths should run its points side by side, each single-threaded: ≈ 10 to 20 times on 20 cores for a long sweep *(estimate)*. Many modes in a region of n_eff, leaky ones included, suit contour integrals (Sakurai & Sugiura 2003; Polizzi 2009; Kestyn et al. 2016): one factorization per quadrature point, all independent. |
| **Hadley's nonlinear eigenproblem** | two or three Newton steps, each a sparse LU of M(ε̄ₖ) (docs/methods/hadley.md) | task farming over modes and sweep points | One mode at a time, from the standard scheme's mode. For many modes at once: Beyn's contour method (Beyn 2012) or NLEIGS (Güttel et al. 2014), both in SLEPc's NEP module (Campos & Roman 2021), both parallel over shifts or quadrature points. |
| **Sweeps over wavelength and parameters** | the solves themselves | task farming at the outermost level; then recycling and block methods | A sweep is the best case: independent, deterministic if collected in order. Where memory forbids one solve per core (3D), neighbouring wavelengths can share Krylov information (Parks et al. 2006), and the ports of one S-matrix can be solved as a block (Freund & Malhotra 1997; Jolivet & Tournier 2016), which reads the matrix once for several right-hand sides. |
| **Adjoints** | one more solve with Aᵀ | reuse what the forward solve built | In 2D the LU serves both (one back-substitution). In 3D, with V A symmetric, the adjoint system is the forward one with another right-hand side: a two-column block solve, or a recycled Krylov space. |
| **Circuits** | the components' S-matrices, not the global solve (as many unknowns as ports) | task farming only (done for wavelengths) | The solve is tiny; GPUs and distributed memory never pay here. A circuit of slow components (3D fidelity) is a farm of component solves. |
| **FDTD (0.5)** | the Yee update: a stencil bound by memory bandwidth | SIMD and cache (temporal) blocking; the GPU; multi-GPU and distributed with halo exchange | *(estimate)* In single precision a cell update moves ≥ 70 B (six components read and written, their coefficients), so streaming caps a CPU at ≈ 1.4 G cell-updates/s on this machine and the RTX 4060 at ≈ 3.8 G. Temporal blocking lifts the CPU's cap 3 to 4 times on a Maxwell stencil (Malas et al. 2016). GPU FDTD reaches 100λ × 100λ metalenses in minutes (Hughes et al. 2021). |
| **RCWA and EME (0.12)** | dense eigenproblems per layer and dense S-matrix products | dense kernels (faer) and task farming over wavelengths, angles and k-points | Dense, compute-bound, double precision: the CPU's SIMD units, not a consumer GPU's 1/64-rate double precision. |
| **Thermal and electro statics (0.6), drift-diffusion (0.8)** | Poisson-like solves on cross-sections | direct (faer) or algebraic multigrid; task farming over voltages and temperatures | Small and definite: classical AMG (Henson & Yang 2002) or geometric multigrid when a cross-section grows; never a GPU or distributed problem at these sizes. |
| **Inverse design with genoxide populations (0.7)** | forward and adjoint solves for every candidate | task farming of candidates across cores, then across machines | A population is independent work, collected by index: deterministic, and the cheapest distributed computing there is. Inside one candidate, parallelism pays only when the population is smaller than the number of cores. |

**What follows from the table.** Three kinds of work cover almost everything: task farming
(sweeps, populations, Monte Carlo, modes), bandwidth-bound grid kernels (QMR's products, multigrid
smoothing, FDTD) and sparse factorizations (2D, modes, coarse grids, subdomains). The first needs a
scheduler, the second needs kernels and then a GPU, and the third needs an ordering. Distributed
memory matters for one problem only, a 3D device larger than one machine's memory, and that is
the domain-decomposition work of Phase E.

## 2. What the HPC libraries offer

### What PETSc and SLEPc changed, 2024 to 2026

Read from PETSc's changes pages (petsc.org/release/changes, 3.21 to 3.26) and SLEPc's
CHANGELOG.md, both documentation, not code. Only the items that touch photonoxide's problems:

| Release | Change | The method behind it | Fit for photonoxide |
|---|---|---|---|
| PETSc 3.21 (2024-03) | `PCGAMG` low-memory graph filtering and heavy-edge-matching aggregates; l1-row Jacobi | smoothed-aggregation AMG | low: AMG for definite problems (the 0.6 statics at most) |
| PETSc 3.22 (2024-09) | ILU as hypre's preconditioner and as a BoomerAMG smoother | BoomerAMG (Henson & Yang 2002) with ILU smoothing | low to medium: the 0.6 and 0.8 static solves, if they ever outgrow a direct solver |
| PETSc 3.24 (2025-09) | MUMPS's block low-rank factorization exposed (`ICNTL(15)`, `MatMumpsSetBlk`); `PCMatApplyTranspose`; a multistage mesh partitioner | BLR multifrontal (Amestoy et al. 2015, 2019); partitioning (Karypis & Kumar 1998) | medium: BLR is the compressed direct solver for 3D FD Maxwell (Shantsev et al. 2017); a transposed preconditioner is what QMR needs |
| PETSc 3.25 (2026-03) | multi-precision MUMPS (`-pc_precision single`), MUMPS out of core | a low-precision factorization refined to working precision (Carson & Higham 2018; Amestoy et al. 2023) | high: 3D direct solves at half the memory |
| PETSc 3.26 (2026) | `KSPIDR` (IDR(s), the biorthogonal variant); `KSPEKSM` for multiple shifted systems, with `MatCreateNestFromMultipleShifts`; weighted restricted Schwarz (`PC_ASM_WEIGHTED`); an HPDDM coarse correction applied after the fine one; device sparse-dense products; native block solves (`KSPMatSolve`) with Richardson | IDR(s) (Sonneveld & van Gijzen 2008; van Gijzen & Sonneveld 2011); shifted Krylov (Frommer & Glässner 1998); restricted Schwarz (Cai & Sarkis 1999); HPDDM (Jolivet et al. 2021) | medium to high: IDR(s) needs no transpose; block solves fit ports; shifted solves fit contour integrals |
| SLEPc 3.22 (2024-09) | structured Krylov–Schur (Bethe–Salpeter); AMD GPUs through HIP | Krylov–Schur (Stewart 2002) | the restart, yes; the structure, no |
| SLEPc 3.23 (2025-03) | a threshold stopping test: every eigenvalue beyond a value, without knowing how many | a stopping rule, not a new method | high: "every guided mode", n_eff above the cladding's index, is exactly that question |
| SLEPc 3.24 (2025-09) | a Chebyshev polynomial filter in `STFILTER` | polynomial filtering of a Hermitian interval | low: photonoxide's mode operators aren't Hermitian (PMLs, the full-vector formulation) |
| SLEPc 3.26 (2026-09) | CISS solves all its quadrature points in one shifted solve (`EPSCISSSetStrategy` with `KSPEKSM`); Kokkos vectors | contour integrals (Sakurai & Sugiura 2003) with shifted Krylov (Frommer & Glässner 1998) | high for many modes at once (Phase C) |

PETSc's GPU support across these releases (Kokkos, CUDA, HIP and SYCL back ends; assembly on the
device) follows the design Mills et al. 2021 describe: the algorithms written once against vector
and matrix interfaces, the device code behind them. That separation is what photonoxide would
copy, with wgpu as its one back end.

### The methods, their papers and their fit

The candidates the owner named, and a few more, each checked against photonoxide's problems.
"Fit" is this survey's judgement, with its reason.

**Preconditioners for 3D FDFD at scale: domain decomposition and sweeping.**

| Method | Paper | What it does | Fit |
|---|---|---|---|
| The unifying review | Gander & Zhang 2019 | shows that sweeping, source transfer, polarized traces and optimized Schwarz are one family, optimized Schwarz methods with suitable transmission conditions | the map to read first, before choosing (Phase E) |
| Optimized Schwarz, Helmholtz | Gander et al. 2002 | non-overlapping Schwarz with optimized Robin and second-order transmission conditions | the transmission conditions; Phase E |
| Optimized Schwarz, Maxwell | Dolean et al. 2009 | the same for time-harmonic Maxwell: a hierarchy of optimized conditions | the vector version photonoxide needs |
| Restricted additive Schwarz, and its optimized form | Cai & Sarkis 1999; St-Cyr et al. 2007 | Schwarz at the algebraic level (RAS); ORAS puts optimized conditions into the subdomain matrices and cuts iterations significantly | **the recommended one-level method**: algebraic, so it works on photonoxide's own matrices, and a subdomain is a smaller FDFD problem with an absorbing boundary |
| Two-level Schwarz for Maxwell | Bonazzoli et al. 2019 | proves wavenumber-independent GMRES iterations for Maxwell with enough absorption, and extends the method by experiment towards less absorption and the propagative case | the closest analysis to 3D FDFD; photonoxide's silicon is lossless, so its regime is the paper's extension, not its theorem |
| Coarse spaces | Spillane et al. 2014 (GenEO); Bootland et al. 2021 | GenEO builds a coarse space from local generalized eigenproblems; Bootland et al. compare grid, DtN and GenEO coarse spaces on high-frequency Helmholtz and find each has its own strengths | needed for more than a few subdomains; which one fits Maxwell at optical frequencies is open, to measure |
| HPDDM in PETSc | Jolivet et al. 2021 | multilevel overlapping Schwarz (GenEO) with block and recycling Krylov methods | the reference design for Phase E |
| Sweeping, moving PMLs | Engquist & Ying 2011 (in the folder); Poulson et al. 2013 | factor the domain in slabs, each closed by PMLs, and sweep through them; in parallel, setup O(γ² N^{4/3}) and application O(γ N log N), γ the PML's points | photonoxide measured one 6-plane slab of Diel's cross-section at 19 s with faer's LU (docs/methods/fdfd-3d.md): too slow without a better ordering; sequential across the slabs |
| Sweeping, Maxwell | Tsuji et al. 2012 | the moving-PML sweep for time-harmonic Maxwell (finite elements) | the vector version |
| Layered decomposition with PML transmission | Stolk 2013 | many thin subdomains along one axis, PML transmission conditions, near-linear cost with multifrontal subdomain solves | natural for long devices (couplers, MMIs); scalar Helmholtz in the paper |
| L-sweeps | Taus et al. 2020 | sweeps in 90° cones over a checkerboard decomposition, scaling in parallel for one right-hand side | the parallel form of sweeping; scalar Helmholtz so far |
| The textbook | Dolean et al. 2015 | Schwarz methods, coarse spaces and their parallel implementation | for Phase E |

**Multigrid and algebraic multigrid.**

| Method | Paper | Fit |
|---|---|---|
| Complex-shifted multigrid with Galerkin coarse grids and ILU smoothing | Erlangga et al. 2006; Reps et al. 2010 (both in the folder) | in progress on the `fdfd3d-multigrid` branch: 18 and 52 QMR iterations on the two measured cases |
| Hybrid smoothing for the curl-curl operator | Hiptmair 1998 | what a multigrid on the curl-curl formulation, rather than Shin and Fan's, would need: the curl's null space smoothed on its own |
| Auxiliary-space AMG for H(curl), hypre's AMS | Hiptmair & Xu 2007; Kolev & Vassilevski 2009 | **outside its setting.** Both treat definite and semidefinite H(curl) problems (Kolev and Vassilevski: "second order definite and semi-definite Maxwell problems"); 3D FDFD at optical frequencies is indefinite. At most a building block inside a shifted preconditioner; not recommended as the preconditioner |
| Classical AMG | Henson & Yang 2002 | for the static problems of 0.6 and 0.8, if they outgrow a direct solver |
| Fine-grained parallel ILU and triangular solves | Chow & Patel 2015; Anzt et al. 2015 | high: ILU(0)'s factorization and its sequential triangular solves become a fixed number of parallel sweeps, deterministic for any thread count and the same on a GPU; this is photonoxide's measured bottleneck (four triangular solves at 4 to 6 times a product) |

**Sparse direct solvers.**

| Method | Paper | Fit |
|---|---|---|
| Nested dissection | George 1973; Karypis & Kumar 1998 | **high, and cheap**: photonoxide's grids are structured, so geometric nested dissection (halve the box across its longest axis, recursively) needs no graph partitioner; faer's low-level supernodal LU accepts a column permutation (read in faer 0.24.4's source) |
| Distributed multifrontal | Amestoy et al. 2001 (MUMPS); Schenk & Gärtner 2004 (PARDISO, which MKL ships) | external benchmarks only: the "MKL-class" baseline for photonoxide's direct solves |
| Block low-rank fronts | Amestoy et al. 2015, 2019; Shantsev et al. 2017 | Shantsev et al. applied BLR to 3D finite-difference frequency-domain Maxwell: at 20.6 M unknowns, 10% of the flops, 30% of the factors' size and 40% of the time, the complexity O(N²) down to about O(N^{1.4–1.6}). Their problem is low-frequency controlled-source EM, which is diffusive; whether ranks stay that low for optical waves in high-contrast silicon isn't established in these papers, and would be the first thing to measure (Phase F) |
| Hierarchically semiseparable fronts | Ghysels et al. 2016 (STRUMPACK) | up to 7 times faster than standard multifrontal on their test suite; the same caveat as BLR; Phase F |
| Mixed-precision refinement of approximate factorizations | Carson & Higham 2018; Higham & Mary 2022; Amestoy et al. 2023 | high: a single-precision (or BLR) factorization preconditioning a double-precision refinement or GMRES, at half the memory of the 3D direct solver (Phase D) |

**Krylov methods.**

| Method | Paper | Fit |
|---|---|---|
| QMR and COCG for complex symmetric matrices | Freund 1992; van der Vorst & Melissen 1990 | **high**: V A is complex symmetric, so one product per iteration and no Aᵀ (Phase A) |
| IDR(s) | Sonneveld & van Gijzen 2008; van Gijzen & Sonneveld 2011 | medium: short recurrences without the transpose, as good as or better than BiCGSTAB for s > 1 in their experiments; a second solver to compare with QMR on Shin and Fan's operator, which isn't known to be symmetric |
| Recycling | Parks et al. 2006 | medium to high for 3D sweeps and forward–adjoint pairs (Phase C) |
| Block QMR | Freund & Malhotra 1997; Jolivet & Tournier 2016 | high for S-matrices: every port at once, the matrix read once per block (Phase C) |
| Shifted systems | Frommer & Glässner 1998 | for contour-integral eigensolvers, whose quadrature points are shifts of one matrix (Phase C) |
| Pipelined Krylov | Ghysels & Vanroose 2014; Cools & Vanroose 2017 | low on one machine, where a reduction is cheap; useful once global reductions cross a network, with residual replacement to keep the attainable accuracy (Phase E) |

**Eigensolvers for the mode solvers.**

| Method | Paper | Fit |
|---|---|---|
| Krylov–Schur | Stewart 2002 | medium: a cleaner restart and deflation than the current restarts (Saad 2011, in the folder); a mode solve's cost is its factorization either way |
| Contour integrals, linear | Sakurai & Sugiura 2003; Polizzi 2009; Kestyn et al. 2016 | high when many modes are wanted (every guided mode of a wide guide, port modes, leaky modes in a region): one independent factorization per quadrature point, which is task farming |
| Contour integrals, nonlinear | Beyn 2012 | high for Hadley's nonlinear eigenproblem when more than one mode is wanted: no starting guesses, every eigenvalue inside the contour |
| Rational Krylov for nonlinear problems | Güttel et al. 2014 (NLEIGS); Güttel & Tisseur 2017 | medium: many modes of Hadley's problem; by its abstract NLEIGS converges more reliably than its predecessor near singularities |
| SLEPc's design | Hernandez et al. 2005; Campos & Roman 2021 | the architecture to borrow: spectral transformation, solver and stopping test as separate parts |

**GPU and CPU kernels.**

| Method | Paper | Fit |
|---|---|---|
| The roofline model | Williams et al. 2009 | every kernel benchmark reports its bandwidth against the machine's, so "fast" means "near the roof" |
| 3D finite differences on GPUs | Micikevicius 2009 | high: slice-by-slice traversal with shared memory, and multi-GPU halo exchange overlapped with computation: the FDTD backend's design |
| Temporal blocking on CPUs | Malas et al. 2015, 2016 | high: the CPU FDTD and the matrix-free FDFD operator; 3 to 4 times over spatial blocking on a Maxwell FDFD code (THIIM) on an 18-core CPU |
| Sparse products on GPUs | Bell & Garland 2009 | for a GPU QMR; on a structured grid the matrix-free stencil reads less than any stored format |
| GPU sparse libraries | Anzt et al. 2022 (Ginkgo); Mills et al. 2021 (PETSc) | design references |
| Multi-GPU FDTD | Nagaoka & Watanabe 2011 | 3.5 times on 4 GPUs: the scaling halo exchange gave there |
| GPU FDTD in photonics | Hughes et al. 2021 | a 100λ × 100λ metalens, focal length included, in under 5 minutes |

**Reproducible reductions.**

| Method | Paper | Fit |
|---|---|---|
| Reproducible summation | Demmel & Nguyen 2015; Ahrens et al. 2020 | sums bit-identical in any order, at about 7n to 9n flops instead of n: one option for principle 9 across ranks (Section 3) |

## 3. The Rust side, checked 2026-10-04

Versions and licences from crates.io's API; capabilities from each project's README or docs, and
for faer from its 0.24.4 source.

### GPU

| Crate | Version (date) | Licence | Pure Rust? | f64 | Complex | Platforms, status |
|---|---|---|---|---|---|---|
| wgpu | 30.0.1 (2026-08-22) | MIT OR Apache-2.0 | yes (it calls the system's Vulkan, DX12 or Metal drivers) | `SHADER_F64` on **Vulkan only**, native only (not DX12, Metal or WebGPU); its docs warn f64 is "frequently between 16 and 64 times slower" than f32 | none: two floats (`vec2<f32>`) by hand | Windows, Linux, macOS, web; stable and maintained. `SHADER_F16` everywhere; subgroups on Vulkan, DX12, Metal |
| CubeCL | 0.10.0 (2026-09-22) | MIT OR Apache-2.0 | the language is; its CUDA and HIP back ends compile through the vendors' toolchains, its CPU back end through MLIR | not stated in its README | not stated | targets WGSL (wgpu), CUDA, SPIR-V, HIP, Metal, CPU; alpha, "expect breaking changes between minor versions" |
| cudarc | 0.19.10 (2026-09-24) | MIT OR Apache-2.0 | no: bindings to NVIDIA's C libraries (driver, NVRTC, cuBLAS, cuSPARSE, cuSOLVER, NCCL, cuFFT…), loaded at run time by default, so no C at build time | yes | through cuBLAS and friends | NVIDIA only, CUDA 11.4 to 13.x |
| rust-gpu | spirv-builder 0.10.0 (2026-10-01) | MIT OR Apache-2.0 | yes: Rust compiled to SPIR-V | as SPIR-V allows, on Vulkan | none | "still heavily in development"; a pinned nightly toolchain |
| Rust-CUDA | cust 0.3.2 on crates.io (2022); the project is "being rebooted" on GitHub | MIT OR Apache-2.0 | no: NVIDIA's NVVM, the CUDA toolkit ≥ 12 and LLVM 7 (or an `llvm21` feature) | yes | none | "early development"; a pinned nightly |

**What this means for photonoxide.** wgpu, which principle 6 already names, is right for FDTD in
single precision, as 0.5 plans. Double precision on the GPU exists only through Vulkan: on this
machine's NVIDIA card it works (Vulkan runs on Windows), on a Mac it doesn't, and on a consumer
GPU it runs at a fraction of the single-precision rate. A double-precision GPU FDFD is therefore a
Vulkan-only feature, or single precision on the GPU refined in double on the CPU (Phase D). And
WGSL allows "reassociation and fusion" of floating-point operations, so results "may differ
between implementations" (the WGSL specification, §15.7): a GPU can't promise bit-for-bit equality
with the CPU, which matters for principle 9 below.

### Distributed memory

| Option | Version (date) | Licence | Pure Rust? | Notes |
|---|---|---|---|---|
| rsmpi (`mpi`) | 0.8.2 (2026-07-09) | MIT OR Apache-2.0 | no: links a C MPI (the MPI-3.1 C interface) and needs a C compiler and libclang to build | tested with Open MPI, MPICH and MS-MPI on Windows; no one-sided communication or parallel I/O |
| MPI 5.0's standard ABI | approved 2025-06; MPICH 5.0.0 (2026-02) implements it | — | still a C library | a future binding could load any conforming MPI at run time without compiling C, but it would still call C |
| Lamellar | 0.8.1 (2026-07-30) | BSD (by its README; crates.io lists it as non-standard) | its single-node back ends (local, shared memory) are; its network back ends need libfabric, UCX, ROFI or PMIx, all C | PNNL's asynchronous PGAS runtime: distributed arrays and active messages; Linux-centred; alpha |
| timely | 0.31.0 (2026-07-14) | MIT | yes, over TCP | a dataflow system, not the SPMD halo exchange a solver wants; proof that a pure-Rust TCP transport is practical |
| photonoxide's own communicator | — | photonoxide's | yes | a trait with send, receive and ordered all-reduce; an in-process back end (threads) and a TCP back end; an MPI back end optional (Section 5, decision 2) |

### Kernels: faer against MKL and PARDISO

| | faer 0.24.4 (MIT, pure Rust) | Intel MKL / PARDISO |
|---|---|---|
| Dense | SIMD through pulp (AVX2; AVX-512 behind faer's `nightly` feature), threads through rayon (`Par`) | hand-tuned for Intel CPUs |
| Sparse direct | supernodal and simplicial LU (COLAMD by default; the low-level API takes a column permutation), Cholesky LLᴴ, LDLᴴ and Bunch–Kaufman LBLᴴ (AMD or a custom ordering) | PARDISO (Schenk & Gärtner 2004), with nested-dissection orderings, a parallel factorization and, by MKL's documentation, complex symmetric matrix types |
| Missing in faer, as read in its source | nested dissection; low-rank compression; distributed memory; an unconjugated LDLᵀ for complex symmetric matrices (its symmetric factorizations are Hermitian, which V A is not); to confirm with faer's author | — |
| Iterative and eigen | CG, BiCGSTAB, LSMR; a restarted partial eigensolver (largest magnitude) | MKL's iterative solvers (reverse communication) |
| Licence | MIT: a dependency | proprietary: a benchmark only |

Measured (benchmark A0, PARDISO and MUMPS run as external programs on the exported systems): on one
thread faer's dense LU is within 10% of MKL's at large supernodes, but its sparse LU reserves the
structure of AᵀA, 3.3 to 4.9 times PARDISO's entries, and is 1.8 to 7 times slower; on 20 threads
its dense LU reaches 13 to 40% of MKL's. And PARDISO and MUMPS factorize the complex symmetric form
in half the entries. See [the comparison](../baselines.md).

### Where this meets principles 6 and 9

**Principle 6, pure Rust** ("no C, Fortran or Python dependencies; linear algebra is faer,
parallelism is rayon, and the GPU is wgpu"):

| Item | Conflict? | Options | Recommendation |
|---|---|---|---|
| wgpu | none | — | the only GPU dependency |
| CubeCL (wgpu back end only) | none in its wgpu back end; it is alpha | adopt for kernel authoring in Rust instead of WGSL, or write WGSL by hand | hand-written WGSL first (few kernels: a stencil, a few vector operations); revisit CubeCL when it leaves alpha |
| cudarc, Rust-CUDA, CubeCL's CUDA back end | yes: NVIDIA's C libraries or toolchain | (a) never; (b) an optional feature, off by default, outside CI's default build | (a) for photonoxide's own kernels: wgpu with Vulkan's f64 covers what photonoxide needs; NVIDIA's libraries as optional run-time backends ([the backends plan](backends.md)) |
| rsmpi | yes: a C MPI | (a) never; (b) a communicator trait, pure-Rust back ends by default, MPI an optional feature; (c) MPI as the main back end | (b), and the MPI back end only when a cluster user asks for it |
| MKL, PARDISO, MUMPS, PETSc, SLEPc | yes (C, Fortran) | (a) external benchmarks only, run as programs like Meep; (b) optional back ends | (b), decided 2026-10-05: loaded at run time, never linked, through `photonoxide-native` ([the backends plan](backends.md)); PETSc and SLEPc dropped. Formerly (a). Their licences (MKL's proprietary licence, MUMPS's CeCILL-C, PETSc's BSD-2) are checked before the benchmark harness runs them |

**Principle 9, determinism** ("reductions are ordered, seeds are explicit, and nothing depends on
thread scheduling"; and in the 1.0 criteria, "bit-for-bit the same result on any number of
threads"):

| Situation | What breaks it | Options |
|---|---|---|
| Threads | reductions whose order follows the thread count (rayon's `sum` over a parallel iterator splits adaptively) | **chunks fixed by the problem, not the thread count**: dot products over fixed blocks of, say, 4 096 entries, the blocks' partial sums added in index order. Products by rows, each row summed in order, as `krylov::product` already does. Cheap, and exact on any thread count |
| Iterative preconditioners in parallel | Gauss–Seidel or ILU sweeps whose result depends on the order rows are processed | Jacobi-type sweeps (each value from the previous sweep only, Chow & Patel 2015; Anzt et al. 2015), or a multicolour ordering fixed by the grid (Saad 2003): the same answer on any thread count |
| Distributed ranks | the partition, and so the subdomains and reductions, follow the number of ranks | **over-decomposition fixed by the problem**: the problem is cut into a fixed set of blocks (subdomains) from its size alone; ranks own whole blocks; every reduction runs block by block in block order. Or reproducible summation (Demmel & Nguyen 2015; Ahrens et al. 2020) at 7 to 9 times the flops of a plain sum. The first is free and fits Schwarz preconditioners, whose subdomains are part of the method anyway |
| GPU | WGSL permits reassociation and fusion (§15.7), so the GPU's arithmetic isn't the CPU's; floating-point atomics add order dependence | no float atomics in any kernel; fixed workgroup sizes and fixed-order tree reductions, so a run repeats bit-for-bit **on the same device and driver**; against the CPU, agreement **to a stated tolerance**, as 0.5 already says ("with the CPU results as its reference") |
| Pipelined Krylov | different rounding from the standard recurrence | deterministic, just different iterates: allowed, documented as a different method |

The GPU row needed the owner's wording (Section 5, decision 1): principle 9 as written couldn't
hold across a CPU and a GPU. *(Decided 2026-10-08: (b). ROADMAP's principle 9 says so, and FDTD's
GPU kernel, #166, holds to it: [the GPU's report](../validation-gpu.md).)*

## 4. The plan in phases

Ordered by what pays most for the least. Each phase states its goal, its methods and papers, how
it is validated and benchmarked (the roadmap's Benchmarks section: time to a converged answer at
equal accuracy, throughput, scaling with threads, peak memory, the GPU's speedup at equal results),
and what it needs from the owner.

| Phase | Goal | Payoff | Effort | Milestone |
|---|---|---|---|---|
| A | CPU foundations: kernels, task farming, nested dissection, deterministic threads | high | low to medium | 0.4.1's follow-up, proposed as 0.4.2 |
| B | FDTD on the CPU and the GPU | high | medium | 0.5 (its GPU item already planned) |
| C | sweeps, ports and many modes; farming across processes | medium to high | medium | 0.5.x to 0.6 |
| D | mixed precision, and 3D FDFD on the GPU | medium | medium to high | with 0.7 (3D inverse design) |
| E | distributed memory: domain decomposition and a communicator | high for the largest 3D devices only | high | a new milestone after 0.7 |
| F | compressed direct solvers (BLR, HSS) | unknown until measured | high (research) | optional, after a go/no-go measurement |

### Phase A: CPU foundations (with 0.4.1, then a proposed 0.4.2)

**Goal.** Use the machine photonoxide already runs on: the iterative solver near the memory
bandwidth's roof, sweeps run side by side, direct solvers with less fill, and every parallel
result bit-identical on any number of threads.

**Work and methods.**

- **A0, the benchmark harness first.** A `bench` command (the roadmap's studio already lists one)
  with fixed problems: the 2D 440 × 340 FDFD, the 3D 40³ silicon guide, Diel at 864 k unknowns,
  the strip with ports at 1.8 M, and the mode-solver examples above. Each records wall time,
  iterations, the field's error against a converged reference, peak memory, and the bandwidth it
  reached against a measured triad bandwidth of the machine (the roofline, Williams et al. 2009).
  The same matrices exported (Matrix Market) and solved by PARDISO (Schenk & Gärtner 2004) and
  MUMPS (Amestoy et al. 2001), run as external programs: the "MKL-class" baseline, never linked.
- **A1, task farming.** The FDFD job's wavelength loop and the mode solvers' sweeps run their
  points in parallel, each on one thread when there are at least as many points as cores (the
  measured mode solves are fastest single-threaded), collected in order, so events and results
  are the same as today's.
- **A2, QMR's kernels.** Threads from a persistent pool instead of new OS threads per product; no
  allocation inside the iteration (the roadmap's pitfall about allocation in solve loops);
  vector updates fused into single passes; dot products and norms over fixed-size chunks summed in
  index order; then a matrix-free Yee operator that reads the averaged permittivity and the
  stretches instead of a stored matrix and its transpose (the stencil structure Malas et al. 2016
  optimize).
- **A3, symmetric QMR.** Freund's QMR for complex symmetric matrices (Freund 1992) on V A x = V b,
  with COCG (van der Vorst & Melissen 1990) as a comparison: one product per iteration, no Aᵀ
  stored. It needs a symmetric preconditioner: an incomplete factorization of the symmetric V A
  (an unconjugated incomplete LDLᵀ, the symmetric form of Saad 2003's incomplete factorizations),
  or a multigrid cycle with restriction Pᵀ and symmetric smoothing. Shin and Fan's operator, on
  which ILU(0) and the 0.4.1 multigrid work, isn't known to become symmetric under a diagonal
  scaling (its added term carries ε on the right of the divergence); whether one exists is a
  question for A3, decided by the 0.4.1 branch's owner, not here.
- **A4, nested dissection.** A geometric nested-dissection ordering for structured grids (George
  1973), passed to faer's supernodal LU as its column permutation; a multilevel partitioner
  (Karypis & Kumar 1998) only if irregular sparsity ever appears.
- **A5, deterministic parallel preconditioning.** ILU(0)'s factorization by a fixed number of
  synchronous fixed-point sweeps (Chow & Patel 2015) and its triangular solves by a fixed number
  of Jacobi sweeps (Anzt et al. 2015), each sweep's values computed from the previous one only, so
  any thread count gives the same bits; and, once 0.4.1 merges, the multigrid's Galerkin products
  row by row and its smoothing by the same sweeps or a multicolour ordering of the grid (Saad
  2003). This is the 0.4.1 branch's own step 3; this plan only supplies the papers.

**Validation.** Every existing validation case passes unchanged. A test runs each parallel kernel
on 1, 4 and 20 threads and compares bits (CI's single-thread job stays). Symmetric QMR gives the
direct solver's field to 1e-10, like `fdfd3d/qmr-direct`. Nested dissection gives the same
solutions to round-off.

**Benchmarks.** Nanoseconds per unknown per iteration and GB/s against the measured triad; time
to a field error of 1e-8 on the 40³ guide and on Diel; sweep throughput (points per second) by
thread count; fill and factorization time with nested dissection against COLAMD in 2D (150 k) and
3D (32³, 40³); faer against PARDISO and MUMPS on the same matrices.

*(Estimates, to be replaced by the benchmarks.)* QMR's iteration 3 to 8 times faster (A2 + A3);
sweeps 10 to 20 times faster on 20 cores (A1); fill and factorization time down by a factor to be
measured (A4).

**From the owner.** Approval of the phase and of a 0.4.2 for it; a licence check before MKL and
MUMPS are run as external programs (decision 4).

### Phase B: FDTD on the CPU and the GPU (0.5)

**Goal.** 0.5's FDTD as fast per core as Meep (the 1.0 "Fast" criterion), and its GPU backend
(already planned: "wgpu compute, single precision, with the CPU results as its reference").

**Work and methods.**

- The CPU kernel first, in f32 and f64: the Yee update with SIMD over the fastest axis, CPML only
  in its slabs, no allocation in the loop; then spatial and temporal (wavefront diamond) blocking
  (Malas et al. 2015, 2016).
- The GPU kernel on wgpu in f32: the slice-by-slice traversal with workgroup memory (Micikevicius
  2009); DFT monitors accumulated per cell on the GPU, in time order, with no cross-thread
  reduction; fluxes reduced by fixed-order trees or on the CPU; no floating-point atomics.
- Multi-GPU halo exchange is Phase E's.

**Validation.** Every 0.5 validation case on both backends. GPU against CPU at the same precision,
within a tolerance set per quantity after measuring how rounding grows over a run; a repeated GPU
run on the same device is bit-identical.

**Benchmarks.** Cell-updates per second by thread count and on the GPU, against the roofline
estimate (≈ 1.4 G/s streaming on this CPU, ≈ 3.8 G/s on the RTX 4060 in f32, *estimates* from
vendor bandwidths and ≥ 70 B per update); Meep per core on its published cases (Oskooi et al.
2010); the GPU's speedup over the blocked CPU kernel, not a naive one; Hughes et al. 2021's
metalens figure as an outside reference point.

**From the owner.** The wording of principle 9 for GPUs (decision 1); which GPUs are supported
(f32 on any wgpu backend; f64, for checking, on Vulkan only); where GPU tests run, since GitHub's
hosted runners have no GPU (decision 7). *(Decided 2026-10-08: decisions 1 (b), 6 (b) and 7 (a).
The GPU kernel is in, #166: see [FDTD on the GPU](../methods/fdtd.md#the-gpu).)*

### Phase C: sweeps, ports and many modes (0.5.x to 0.6)

**Goal.** The work that multiplies a solve, wavelengths, ports, modes and candidates, at the cost
of fewer solves, and spread over processes and machines.

**Work and methods.**

- **Ports as a block.** Block QMR (Freund & Malhotra 1997; Jolivet & Tournier 2016) for all of a
  3D S-matrix's ports, the matrix read once per block of right-hand sides.
- **Recycling** across a 3D wavelength sweep and between a forward solve and its adjoint (Parks
  et al. 2006; preconditioning sequences of systems, Bertaccini & Durastante 2018, Section 3.6).
- **Many modes.** A contour-integral mode solver (Sakurai & Sugiura 2003; Polizzi 2009; for
  non-Hermitian operators with PMLs, Kestyn et al. 2016), its quadrature points farmed; "every
  guided mode" as a threshold on n_eff (the question SLEPc 3.23's threshold test answers); for
  Hadley's nonlinear problem, Beyn's contour method (Beyn 2012) and NLEIGS (Güttel et al. 2014;
  Güttel & Tisseur 2017). Krylov–Schur restarts (Stewart 2002) for the shift-invert solver.
- **Farming across processes.** A job runner that sends independent evaluations (a genoxide
  population, a Monte Carlo of 0.11, corners, a sweep) to worker processes on this machine or
  others over TCP, in pure Rust, and gathers results by index.

**Validation.** Block QMR and recycling give the same S-matrices as single solves to 1e-10. The
contour solver finds the same modes as shift-invert, and every eigenvalue of a small problem in
its region against a dense eigensolve. A farmed sweep is bit-identical for any number of workers.

**Benchmarks.** S-matrix time against ports; a 3D sweep's time against its points; modes per
second; a population's wall time against workers.

**From the owner.** Whether a second machine is available for farming tests. genoxide must be
able to hand a whole population to photonoxide's evaluator at once; if it can't, that is a
general need to describe to genoxide's owner, not a change made here.

### Phase D: mixed precision and 3D FDFD on the GPU (with 0.7)

**Goal.** 3D inverse design needs many 3D solves: halve the direct solver's memory, and move the
iterative solver to the GPU.

**Work and methods.**

- A single-precision sparse LU (faer is generic over the scalar) as the preconditioner of a
  double-precision refinement or GMRES (Carson & Higham 2018; Amestoy et al. 2023), with the
  limits Higham & Mary 2022 set out: when the condition number is too large for the low
  precision, the refinement fails and must say so.
- The matrix-free QMR of Phase A on wgpu: f32 inner solves refined in f64 on the CPU, or f64
  throughout on Vulkan; smoothing and ILU by sweeps (Chow & Patel 2015; Anzt et al. 2015); stored
  formats (Bell & Garland 2009) only where the operator isn't a stencil; Ginkgo's and PETSc's GPU
  designs as references (Anzt et al. 2022; Mills et al. 2021).

**Validation.** The same field error as the CPU in f64 on every 3D validation case; the
refinement's convergence tested, and its failure reported, on an ill-conditioned case.

**Benchmarks.** Time to a field error of 1e-8, CPU f64 against mixed and GPU; memory.
*(Estimate)* Shin and Fan's full Diel (15 M unknowns) stores A and Aᵀ in about 10 GB; matrix-free
in single precision its vectors and permittivity take under 2 GB, so it would fit the RTX 4060's
8 GB only matrix-free.

**From the owner.** Whether f32 on the GPU with f64 refinement is acceptable as the default GPU
path (decision 6).

### Phase E: distributed memory (a new milestone after 0.7)

**Goal.** 3D devices larger than one machine's memory, and fewer hours per 3D solve, with the
answer independent of the number of processes.

**Work and methods.**

- **A communicator trait** with an in-process back end and a TCP back end in pure Rust, and an
  optional MPI back end (rsmpi) off by default (decision 2).
- **Optimized restricted additive Schwarz** as QMR's preconditioner (St-Cyr et al. 2007; RAS:
  Cai & Sarkis 1999), with transmission conditions from optimized Schwarz for Maxwell (Dolean et
  al. 2009; Gander et al. 2002) or the PML itself, and a coarse space (GenEO, Spillane et al. 2014;
  the comparison of Bootland et al. 2021; Maxwell's two-level analysis, Bonazzoli et al. 2019),
  designed after HPDDM (Jolivet et al. 2021). Subdomain solves by faer's LU with Phase A's
  ordering, or by local multigrid. Read Gander & Zhang 2019 and Dolean et al. 2015 first.
- **Sweeping as the alternative** for long devices: moving PMLs (Engquist & Ying 2011; Poulson et
  al. 2013), Maxwell's version (Tsuji et al. 2012), layered decomposition (Stolk 2013), L-sweeps
  for parallelism (Taus et al. 2020). A small prototype of both decides between them on the strip
  with ports.
- **Determinism:** subdomains fixed by the problem, owned whole by ranks, reductions in subdomain
  order; reproducible summation (Demmel & Nguyen 2015; Ahrens et al. 2020) where a reduction can't
  be ordered. Pipelined Krylov (Ghysels & Vanroose 2014; Cools & Vanroose 2017) only if reductions'
  latency is measured to matter.
- **Distributed FDTD:** halo exchange overlapped with computation (Micikevicius 2009; Nagaoka &
  Watanabe 2011).

**Validation.** Bit-identical results on 1, 2 and 4 processes; the field against a one-machine
solve; iteration counts against the number of subdomains (flat with a working coarse space).

**Benchmarks.** Strong and weak scaling over processes on one machine, then over machines; time to
a field error of 1e-8 against Phase A's single-machine solver; memory per process.

**From the owner.** Machines to test on; the MPI decision; where the milestone goes (decision 5).

### Phase F: compressed direct solvers (research, optional)

**Goal.** Find out whether block low-rank or hierarchical compression makes 3D direct solves
practical at optical frequencies, before building either.

**Work.** A measurement first: the numerical ranks of the off-diagonal blocks of the fronts of a
3D FDFD factorization (silicon in oxide at 1550 nm, Phase A's ordering), against the frequency and
the grid. If ranks stay low, a BLR front (Amestoy et al. 2015, 2019) as in Shantsev et al. 2017,
or HSS (Ghysels et al. 2016); if not, the phase stops with its measurement published.

**From the owner.** Whether to spend research time here at all.

## 5. Decisions for the owner

1. **Principle 9 and GPUs.** WGSL allows fused and reassociated arithmetic, so a GPU result can't
   equal the CPU's bit for bit.
   - (a) Keep principle 9 strict, and treat GPU results as previews, never reported numbers.
   - (b) Reword: bit-for-bit on any number of CPU threads and processes; a GPU run repeats
     bit-for-bit on the same device and driver, and agrees with the CPU to a stated tolerance.
   - *Recommended: (b).* It is what 0.5 already implies. *(Decided 2026-10-08: (b).)*
2. **Principle 6 and MPI.**
   - (a) No MPI ever; pure-Rust transports only.
   - (b) A communicator trait with pure-Rust back ends by default and an optional `mpi` feature,
     off by default, documented as the one C dependency.
   - (c) MPI as the main transport.
   - *Recommended: (b),* with the MPI back end written only when someone needs a cluster.
3. **CUDA.** (a) Never: wgpu only, with Vulkan's f64. (b) An optional cudarc feature for cuSPARSE
   and cuSOLVER. *Recommended: (a).* *(Decided 2026-10-05: NVIDIA's libraries (cuDSS, cuSPARSE,
   AmgX) as optional backends loaded at run time, never linked, in [the backends plan](backends.md);
   photonoxide's own GPU kernels, FDTD's first, stay on wgpu.)*
4. **MKL, PARDISO, MUMPS, PETSc and SLEPc** as external benchmarks, run as programs on exported
   matrices, never linked, after their licences are checked. *Recommended: yes.* *(Decided
   2026-10-05, beyond this: optional backends loaded at run time through `photonoxide-native`,
   never linked, never GPL; PETSc and SLEPc dropped, having no native Windows build. See [the
   backends plan](backends.md).)*
5. **Where distributed memory goes.** (a) A new milestone after 0.7. (b) Inside 0.7. (c) After
   1.0. *Recommended: (a),* with Phase C's process farming earlier, since it is cheap and serves
   0.7's populations.
6. **Precision on the GPU.** (a) f32 only (0.5 as planned). (b) f32 with f64 refinement on the
   CPU for FDFD, f64 on Vulkan for checking. (c) f64 everywhere. *Recommended: (b).* *(Decided 2026-10-08: (b), f32 on the GPU with f64 for checking.)*
7. **Where GPU tests run.** GitHub's hosted runners have no GPU. (a) On the owner's machine before
   a release, recorded in the validation report. (b) A self-hosted runner with the RTX 4060.
   *Recommended: (a) first, (b) if GPU regressions slip through.* *(Decided 2026-10-08: (a), on the owner's machine before each release, recorded in the validation report.)*
8. **A 0.4.2 for Phase A.** (a) A patch release after 0.4.1. (b) Folded into 0.5. *Recommended:
   (a),* since Phase A speeds up what users run today.
9. **Symmetric QMR and the multigrid branch.** The symmetric solver needs a symmetric
   preconditioner; the 0.4.1 multigrid is built on Shin and Fan's operator with a transposed
   cycle. *Recommended:* leave 0.4.1 as it is; try symmetric QMR first on the curl-curl operator
   with an incomplete LDLᵀ, and decide afterwards whether the multigrid moves.

## References

Every DOI below was checked on Crossref on 2026-10-04. "Folder" marks a copy in the papers
folder; the others are listed there as needed. Gaps: 33 of the 61 new papers have no copy yet.
23 have no open copy that OpenAlex, Unpaywall or arXiv know of; 10 have one behind a bot check or
a publisher's block on scripts, and the folder's README gives each one's link.

**Already in the folder before this plan**

- Y. Saad, *Iterative Methods for Sparse Linear Systems*, 2nd ed., SIAM (2003). [10.1137/1.9780898718003](https://doi.org/10.1137/1.9780898718003)
- Y. Saad, *Numerical Methods for Large Eigenvalue Problems*, revised ed., SIAM (2011). [10.1137/1.9781611970739](https://doi.org/10.1137/1.9781611970739)
- D. Bertaccini, F. Durastante, *Iterative Methods and Preconditioning for Large and Sparse Linear Systems with Applications*, Chapman and Hall/CRC (2018). [10.1201/9781315153575](https://doi.org/10.1201/9781315153575)
- B. Engquist, L. Ying, Multiscale Model. Simul. 9, 686 (2011). [10.1137/100804644](https://doi.org/10.1137/100804644)
- Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci. Comput. 27, 1471 (2006). [10.1137/040615195](https://doi.org/10.1137/040615195)
- B. Reps, W. Vanroose, H. bin Zubair, J. Comput. Phys. 229, 8384 (2010). [10.1016/j.jcp.2010.07.022](https://doi.org/10.1016/j.jcp.2010.07.022)
- W. Shin, S. Fan, Opt. Express 21, 22578 (2013). [10.1364/OE.21.022578](https://doi.org/10.1364/OE.21.022578)
- A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010). [10.1016/j.cpc.2009.11.008](https://doi.org/10.1016/j.cpc.2009.11.008)

**New with this plan** (folder file names as in its README)

- `ahrens-2020`: W. Ahrens, J. Demmel, H. D. Nguyen, ACM Trans. Math. Softw. 46, 22 (2020), "Algorithms for Efficient Reproducible Floating Point Summation". [10.1145/3389360](https://doi.org/10.1145/3389360)
- `amestoy-2001`: P. R. Amestoy, I. S. Duff, J.-Y. L'Excellent, J. Koster, SIAM J. Matrix Anal. Appl. 23, 15 (2001), "A Fully Asynchronous Multifrontal Solver Using Distributed Dynamic Scheduling". [10.1137/S0895479899358194](https://doi.org/10.1137/S0895479899358194)
- `amestoy-2015`: P. Amestoy, C. Ashcraft, O. Boiteau, A. Buttari, J.-Y. L'Excellent, C. Weisbecker, SIAM J. Sci. Comput. 37, A1451 (2015), "Improving Multifrontal Methods by Means of Block Low-Rank Representations". [10.1137/120903476](https://doi.org/10.1137/120903476)
- `amestoy-2019`: P. R. Amestoy, A. Buttari, J.-Y. L'Excellent, T. Mary, ACM Trans. Math. Softw. 45, 1 (2019), "Performance and Scalability of the Block Low-Rank Multifrontal Factorization on Multicore Architectures". [10.1145/3242094](https://doi.org/10.1145/3242094)
- `amestoy-2023` (folder): P. Amestoy, A. Buttari, N. J. Higham, J.-Y. L’Excellent, T. Mary, B. Vieublé, ACM Trans. Math. Softw. 49, 4 (2023), "Combining Sparse Approximate Factorizations with Mixed-precision Iterative Refinement". [10.1145/3582493](https://doi.org/10.1145/3582493)
- `anzt-2015`: H. Anzt, E. Chow, J. Dongarra, Euro-Par 2015, Lect. Notes Comput. Sci. 9233, 650 (2015), "Iterative Sparse Triangular Solves for Preconditioning". [10.1007/978-3-662-48096-0_50](https://doi.org/10.1007/978-3-662-48096-0_50)
- `anzt-2022` (folder): H. Anzt, T. Cojean, G. Flegar, F. Göbel, T. Grützmacher, P. Nayak, T. Ribizel, Y. M. Tsai, E. S. Quintana-Ortí, ACM Trans. Math. Softw. 48, 2 (2022), "Ginkgo: A modern linear operator algebra framework for high performance computing". [10.1145/3480935](https://doi.org/10.1145/3480935)
- `bell-garland-2009` (folder): N. Bell, M. Garland, Proc. SC09, article 18 (2009), "Implementing sparse matrix-vector multiplication on throughput-oriented processors". [10.1145/1654059.1654078](https://doi.org/10.1145/1654059.1654078)
- `beyn-2012` (folder): W.-J. Beyn, Linear Algebra Appl. 436, 3839 (2012), "An integral method for solving nonlinear eigenvalue problems". [10.1016/j.laa.2011.03.030](https://doi.org/10.1016/j.laa.2011.03.030)
- `bonazzoli-2019` (folder): M. Bonazzoli, V. Dolean, I. G. Graham, E. A. Spence, P.-H. Tournier, Math. Comp. 88, 2559 (2019), "Domain decomposition preconditioning for the high-frequency time-harmonic Maxwell equations with absorption". [10.1090/mcom/3447](https://doi.org/10.1090/mcom/3447)
- `bootland-2021` (folder): N. Bootland, V. Dolean, P. Jolivet, P.-H. Tournier, Comput. Math. Appl. 98, 239 (2021), "A comparison of coarse spaces for Helmholtz problems in the high frequency regime". [10.1016/j.camwa.2021.07.011](https://doi.org/10.1016/j.camwa.2021.07.011)
- `cai-sarkis-1999`: X.-C. Cai, M. Sarkis, SIAM J. Sci. Comput. 21, 792 (1999), "A Restricted Additive Schwarz Preconditioner for General Sparse Linear Systems". [10.1137/S106482759732678X](https://doi.org/10.1137/S106482759732678X)
- `campos-roman-2021` (folder): C. Campos, J. E. Roman, ACM Trans. Math. Softw. 47, 23 (2021), "NEP: A module for the parallel solution of nonlinear eigenvalue problems in SLEPc". [10.1145/3447544](https://doi.org/10.1145/3447544)
- `carson-higham-2018`: E. Carson, N. J. Higham, SIAM J. Sci. Comput. 40, A817 (2018), "Accelerating the Solution of Linear Systems by Iterative Refinement in Three Precisions". [10.1137/17M1140819](https://doi.org/10.1137/17M1140819)
- `chow-patel-2015` (folder): E. Chow, A. Patel, SIAM J. Sci. Comput. 37, C169 (2015), "Fine-Grained Parallel Incomplete LU Factorization". [10.1137/140968896](https://doi.org/10.1137/140968896)
- `cools-vanroose-2017` (folder): S. Cools, W. Vanroose, Parallel Comput. 65, 1 (2017), "The communication-hiding pipelined BiCGstab method for the parallel solution of large unsymmetric linear systems". [10.1016/j.parco.2017.04.005](https://doi.org/10.1016/j.parco.2017.04.005)
- `demmel-nguyen-2015`: J. Demmel, H. D. Nguyen, IEEE Trans. Comput. 64, 2060 (2015), "Parallel Reproducible Summation". [10.1109/TC.2014.2345391](https://doi.org/10.1109/TC.2014.2345391)
- `dolean-2009` (folder): V. Dolean, M. J. Gander, L. Gerardo-Giorda, SIAM J. Sci. Comput. 31, 2193 (2009), "Optimized Schwarz Methods for Maxwell's Equations". [10.1137/080728536](https://doi.org/10.1137/080728536)
- `dolean-2015`: V. Dolean, P. Jolivet, F. Nataf, SIAM (2015), a book, "An Introduction to Domain Decomposition Methods: Algorithms, Theory, and Parallel Implementation". [10.1137/1.9781611974065](https://doi.org/10.1137/1.9781611974065)
- `freund-1992`: R. W. Freund, SIAM J. Sci. Stat. Comput. 13, 425 (1992), "Conjugate Gradient-Type Methods for Linear Systems with Complex Symmetric Coefficient Matrices". [10.1137/0913023](https://doi.org/10.1137/0913023)
- `freund-malhotra-1997`: R. W. Freund, M. Malhotra, Linear Algebra Appl. 254, 119 (1997), "A block QMR algorithm for non-Hermitian linear systems with multiple right-hand sides". [10.1016/S0024-3795(96)00529-0](https://doi.org/10.1016/S0024-3795(96)00529-0)
- `frommer-glassner-1998`: A. Frommer, U. Glässner, SIAM J. Sci. Comput. 19, 15 (1998), "Restarted GMRES for Shifted Linear Systems". [10.1137/S1064827596304563](https://doi.org/10.1137/S1064827596304563)
- `gander-2002` (folder): M. J. Gander, F. Magoulès, F. Nataf, SIAM J. Sci. Comput. 24, 38 (2002), "Optimized Schwarz Methods without Overlap for the Helmholtz Equation". [10.1137/S1064827501387012](https://doi.org/10.1137/S1064827501387012)
- `gander-zhang-2019` (folder): M. J. Gander, H. Zhang, SIAM Rev. 61, 3 (2019), "A Class of Iterative Solvers for the Helmholtz Equation: Factorizations, Sweeping Preconditioners, Source Transfer, Single Layer Potentials, Polarized Traces, and Optimized Schwarz Methods". [10.1137/16M109781X](https://doi.org/10.1137/16M109781X)
- `george-1973`: A. George, SIAM J. Numer. Anal. 10, 345 (1973), "Nested Dissection of a Regular Finite Element Mesh". [10.1137/0710032](https://doi.org/10.1137/0710032)
- `ghysels-2016` (folder): P. Ghysels, X. S. Li, F.-H. Rouet, S. Williams, A. Napov, SIAM J. Sci. Comput. 38, S358 (2016), "An Efficient Multicore Implementation of a Novel HSS-Structured Multifrontal Solver Using Randomized Sampling". [10.1137/15M1010117](https://doi.org/10.1137/15M1010117)
- `ghysels-vanroose-2014`: P. Ghysels, W. Vanroose, Parallel Comput. 40, 224 (2014), "Hiding global synchronization latency in the preconditioned Conjugate Gradient algorithm". [10.1016/j.parco.2013.06.001](https://doi.org/10.1016/j.parco.2013.06.001)
- `guttel-2014`: S. Güttel, R. Van Beeumen, K. Meerbergen, W. Michiels, SIAM J. Sci. Comput. 36, A2842 (2014), "NLEIGS: A Class of Fully Rational Krylov Methods for Nonlinear Eigenvalue Problems". [10.1137/130935045](https://doi.org/10.1137/130935045)
- `guttel-tisseur-2017`: S. Güttel, F. Tisseur, Acta Numerica 26, 1 (2017), "The nonlinear eigenvalue problem". [10.1017/S0962492917000034](https://doi.org/10.1017/S0962492917000034)
- `henson-yang-2002`: V. E. Henson, U. M. Yang, Appl. Numer. Math. 41, 155 (2002), "BoomerAMG: A parallel algebraic multigrid solver and preconditioner". [10.1016/S0168-9274(01)00115-5](https://doi.org/10.1016/S0168-9274(01)00115-5)
- `hernandez-2005`: V. Hernandez, J. E. Roman, V. Vidal, ACM Trans. Math. Softw. 31, 351 (2005), "SLEPc: A scalable and flexible toolkit for the solution of eigenvalue problems". [10.1145/1089014.1089019](https://doi.org/10.1145/1089014.1089019)
- `higham-mary-2022` (folder): N. J. Higham, T. Mary, Acta Numerica 31, 347 (2022), "Mixed precision algorithms in numerical linear algebra". [10.1017/S0962492922000022](https://doi.org/10.1017/S0962492922000022)
- `hiptmair-1998`: R. Hiptmair, SIAM J. Numer. Anal. 36, 204 (1998), "Multigrid Method for Maxwell's Equations". [10.1137/S0036142997326203](https://doi.org/10.1137/S0036142997326203)
- `hiptmair-xu-2007`: R. Hiptmair, J. Xu, SIAM J. Numer. Anal. 45, 2483 (2007), "Nodal Auxiliary Space Preconditioning in H(curl) and H(div) Spaces". [10.1137/060660588](https://doi.org/10.1137/060660588)
- `hughes-2021`: T. W. Hughes, M. Minkov, V. Liu, Z. Yu, S. Fan, Appl. Phys. Lett. 119, 150502 (2021), "A perspective on the pathway toward full wave simulation of large area metalenses". [10.1063/5.0071245](https://doi.org/10.1063/5.0071245)
- `jolivet-2021`: P. Jolivet, J. E. Roman, S. Zampini, Comput. Math. Appl. 84, 277 (2021), "KSPHPDDM and PCHPDDM: Extending PETSc with advanced Krylov methods and robust multilevel overlapping Schwarz preconditioners". [10.1016/j.camwa.2021.01.003](https://doi.org/10.1016/j.camwa.2021.01.003)
- `jolivet-tournier-2016`: P. Jolivet, P.-H. Tournier, SC16: Proc. Int. Conf. High Performance Computing, Networking, Storage and Analysis, 190 (2016), "Block Iterative Methods and Recycling for Improved Scalability of Linear Solvers". [10.1109/SC.2016.16](https://doi.org/10.1109/SC.2016.16)
- `karypis-kumar-1998`: G. Karypis, V. Kumar, SIAM J. Sci. Comput. 20, 359 (1998), "A Fast and High Quality Multilevel Scheme for Partitioning Irregular Graphs". [10.1137/S1064827595287997](https://doi.org/10.1137/S1064827595287997)
- `kestyn-2016` (folder): J. Kestyn, E. Polizzi, P. T. P. Tang, SIAM J. Sci. Comput. 38, S772 (2016), "Feast Eigensolver for Non-Hermitian Problems". [10.1137/15M1026572](https://doi.org/10.1137/15M1026572)
- `kolev-vassilevski-2009` (folder): T. V. Kolev, P. S. Vassilevski, J. Comput. Math. 27, 604 (2009), "Parallel auxiliary space AMG for H(curl) problems". [10.4208/jcm.2009.27.5.013](https://doi.org/10.4208/jcm.2009.27.5.013)
- `malas-2015` (folder): T. Malas, G. Hager, H. Ltaief, H. Stengel, G. Wellein, D. Keyes, SIAM J. Sci. Comput. 37, C439 (2015), "Multicore-Optimized Wavefront Diamond Blocking for Optimizing Stencil Updates". [10.1137/140991133](https://doi.org/10.1137/140991133)
- `malas-2016` (folder): T. M. Malas, J. Hornich, G. Hager, H. Ltaief, C. Pflaum, D. E. Keyes, Proc. IEEE IPDPS 2016, 142, "Optimization of an Electromagnetics Code with Multicore Wavefront Diamond Blocking and Multi-dimensional Intra-Tile Parallelization". [10.1109/IPDPS.2016.87](https://doi.org/10.1109/IPDPS.2016.87)
- `micikevicius-2009` (folder): P. Micikevicius, Proc. 2nd Workshop on General Purpose Processing on Graphics Processing Units (GPGPU-2), 79 (2009), "3D finite difference computation on GPUs using CUDA". [10.1145/1513895.1513905](https://doi.org/10.1145/1513895.1513905)
- `mills-2021` (folder): R. T. Mills, M. F. Adams, S. Balay, J. Brown, A. Dener, M. Knepley, S. E. Kruger, H. Morgan, T. Munson, K. Rupp, B. F. Smith, S. Zampini, H. Zhang, J. Zhang, Parallel Comput. 108, 102831 (2021), "Toward performance-portable PETSc for GPU-based exascale systems". [10.1016/j.parco.2021.102831](https://doi.org/10.1016/j.parco.2021.102831)
- `nagaoka-watanabe-2011`: T. Nagaoka, S. Watanabe, Proc. IEEE EMBC 2011, 401, "Multi-GPU accelerated three-dimensional FDTD method for electromagnetic simulation". [10.1109/IEMBS.2011.6090128](https://doi.org/10.1109/IEMBS.2011.6090128)
- `parks-2006` (folder): M. L. Parks, E. de Sturler, G. Mackey, D. D. Johnson, S. Maiti, SIAM J. Sci. Comput. 28, 1651 (2006), "Recycling Krylov Subspaces for Sequences of Linear Systems". [10.1137/040607277](https://doi.org/10.1137/040607277)
- `polizzi-2009` (folder): E. Polizzi, Phys. Rev. B 79, 115112 (2009), "Density-matrix-based algorithm for solving eigenvalue problems". [10.1103/PhysRevB.79.115112](https://doi.org/10.1103/PhysRevB.79.115112)
- `poulson-2013` (folder): J. Poulson, B. Engquist, S. Li, L. Ying, SIAM J. Sci. Comput. 35, C194 (2013), "A Parallel Sweeping Preconditioner for Heterogeneous 3D Helmholtz Equations". [10.1137/120871985](https://doi.org/10.1137/120871985)
- `sakurai-sugiura-2003`: T. Sakurai, H. Sugiura, J. Comput. Appl. Math. 159, 119 (2003), "A projection method for generalized eigenvalue problems using numerical integration". [10.1016/S0377-0427(03)00565-X](https://doi.org/10.1016/S0377-0427(03)00565-X)
- `schenk-gartner-2004`: O. Schenk, K. Gärtner, Future Gener. Comput. Syst. 20, 475 (2004), "Solving unsymmetric sparse systems of linear equations with PARDISO". [10.1016/j.future.2003.07.011](https://doi.org/10.1016/j.future.2003.07.011)
- `shantsev-2017`: D. V. Shantsev, P. Jaysaval, S. de la Kethulle de Ryhove, P. R. Amestoy, A. Buttari, J.-Y. L’Excellent, T. Mary, Geophys. J. Int. 209, 1558 (2017), "Large-scale 3-D EM modelling with a Block Low-Rank multifrontal direct solver". [10.1093/gji/ggx106](https://doi.org/10.1093/gji/ggx106)
- `sonneveld-vangijzen-2008`: P. Sonneveld, M. B. van Gijzen, SIAM J. Sci. Comput. 31, 1035 (2008), "IDR(s): A family of simple and fast algorithms for solving large nonsymmetric systems of linear equations". [10.1137/070685804](https://doi.org/10.1137/070685804)
- `spillane-2014` (folder): N. Spillane, V. Dolean, P. Hauret, F. Nataf, C. Pechstein, R. Scheichl, Numer. Math. 126, 741 (2014), "Abstract robust coarse spaces for systems of PDEs via generalized eigenproblems in the overlaps". [10.1007/s00211-013-0576-y](https://doi.org/10.1007/s00211-013-0576-y)
- `st-cyr-2007` (folder): A. St-Cyr, M. J. Gander, S. J. Thomas, SIAM J. Sci. Comput. 29, 2402 (2007), "Optimized Multiplicative, Additive, and Restricted Additive Schwarz Preconditioning". [10.1137/060652610](https://doi.org/10.1137/060652610)
- `stewart-2002`: G. W. Stewart, SIAM J. Matrix Anal. Appl. 23, 601 (2002), "A Krylov--Schur Algorithm for Large Eigenproblems". [10.1137/S0895479800371529](https://doi.org/10.1137/S0895479800371529)
- `stolk-2013` (folder): C. C. Stolk, J. Comput. Phys. 241, 240 (2013), "A rapidly converging domain decomposition method for the Helmholtz equation". [10.1016/j.jcp.2013.01.039](https://doi.org/10.1016/j.jcp.2013.01.039)
- `taus-2020` (folder): M. Taus, L. Zepeda-Núñez, R. J. Hewett, L. Demanet, J. Comput. Phys. 420, 109706 (2020), "L-Sweeps: A scalable, parallel preconditioner for the high-frequency Helmholtz equation". [10.1016/j.jcp.2020.109706](https://doi.org/10.1016/j.jcp.2020.109706)
- `tsuji-2012`: P. Tsuji, B. Engquist, L. Ying, J. Comput. Phys. 231, 3770 (2012), "A sweeping preconditioner for time-harmonic Maxwell’s equations with finite elements". [10.1016/j.jcp.2012.01.025](https://doi.org/10.1016/j.jcp.2012.01.025)
- `vandervorst-melissen-1990`: H. van der Vorst, J. Melissen, IEEE Trans. Magn. 26, 706 (1990), "A Petrov-Galerkin type method for solving Ax = b, where A is symmetric complex". [10.1109/20.106415](https://doi.org/10.1109/20.106415)
- `vangijzen-sonneveld-2011`: M. B. van Gijzen, P. Sonneveld, ACM Trans. Math. Softw. 38, 5 (2011), "Algorithm 913: An elegant IDR(s) variant that efficiently exploits biorthogonality properties". [10.1145/2049662.2049667](https://doi.org/10.1145/2049662.2049667)
- `williams-2009` (folder): S. Williams, A. Waterman, D. Patterson, Commun. ACM 52, 65 (2009), "Roofline: an insightful visual performance model for multicore architectures". [10.1145/1498765.1498785](https://doi.org/10.1145/1498765.1498785)

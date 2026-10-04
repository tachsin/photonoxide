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

**The machine** behind every measurement here: an Intel Core Ultra 7 265K (20 cores), 64 GB of
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
2. **QMR does twice the work it needs.** The 3D system scaled by the PML stretches, V A, is
   complex symmetric (docs/methods/fdfd-3d.md). With the code's own start (w₁ = v₁), QMR's two
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
   dependency.

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

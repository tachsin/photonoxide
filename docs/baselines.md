# Direct solvers: photonoxide against PARDISO and MUMPS

This page compares photonoxide's sparse direct solves with the two direct solvers most used in
practice, MKL's PARDISO (O. Schenk, K. Gärtner, Future Gener. Comput. Syst. 20, 475 (2004),
doi:10.1016/j.future.2003.07.011) and MUMPS (P. R. Amestoy, I. S. Duff, J.-Y. L'Excellent,
J. Koster, SIAM J. Matrix Anal. Appl. 23, 15 (2001), doi:10.1137/S0895479899358194; P. R.
Amestoy, A. Buttari, J.-Y. L'Excellent, T. Mary, ACM Trans. Math. Softw. 45, 1 (2019),
doi:10.1145/3242094). All three solve the same matrices, on the same machine.

It was first made for 0.4.2, when photonoxide's solves were faer's supernodal LU, and said where
the difference came from and what pure-Rust work would close it. 0.4.3 did that work: since then
the solves are photonoxide's own multifrontal factorization with static pivoting, as LU or as
L D Lᵀ of a complex symmetric similarity (see [FDFD in 3D](methods/fdfd-3d.md#cost)). The
first section below is 0.4.3's comparison; the rest is 0.4.2's, as it was measured.

PARDISO and MUMPS were run as external programs. Neither is linked, built or shipped with
photonoxide (the performance plan's decision 4).

## 0.4.3: the multifrontal factorization

The same four systems, the same machine and the same runs of PARDISO and MUMPS as the 0.4.2
results below; photonoxide 0.4.3's `bench::export::factorize`, each in a process of its own,
on 2026-10-05. Numeric factorization in seconds on 1 / 20 threads, the factors' entries, and peak
memory on 20 threads.

**The general LU** (PARDISO `mtype` 13, MUMPS `sym` 0):

| System | photonoxide 0.4.3 | PARDISO | MUMPS | photonoxide 0.4.2 (faer) |
|---|---|---|---|---|
| `slab-2d` | 0.33 / 0.105 s, 15.7 M, 0.54 GB | 0.31 / 0.077 s, 9.2 M, 0.39 GB | 0.36 / 0.29 s, 15.8 M, 0.35 GB | 0.64 / 0.85 s, 30.2 M, 0.63 GB |
| `strip-24` | 1.41 / 0.36 s, 22.0 M, 1.04 GB | 1.82 / 0.28 s, 23.7 M, 0.56 GB | 1.53 / 1.98 s, 25.5 M, 0.53 GB | 8.56 / 2.69 s, 107.4 M, 1.83 GB |
| `strip-32` | 7.64 / 1.36 s, 75.7 M, 2.32 GB | 11.1 / 1.67 s, 82.4 M, 1.65 GB | 7.89 / 2.64 s, 84.5 M, 1.76 GB | 55.4 / 10.9 s, 393.5 M, 6.34 GB |
| `strip-40` | 28.0 / 3.75 s, 194.6 M, 5.42 GB | 43.8 / 8.45 s, 219.2 M, 4.07 GB | 29.3 / 9.23 s, 214.3 M, 4.29 GB | 199.6 / 35.0 s, 1 064.8 M, 16.80 GB |

**As complex symmetric matrices** (photonoxide's L D Lᵀ, PARDISO `mtype` 6, MUMPS `sym` 2; L's
entries):

| System | photonoxide 0.4.3 | PARDISO | MUMPS |
|---|---|---|---|
| `slab-2d` | 0.19 / 0.059 s, 7.9 M, 0.45 GB | 0.20 / 0.047 s, 5.0 M, 0.29 GB | 0.31 / 0.34 s, 9.7 M, 0.26 GB |
| `strip-24` | 0.85 / 0.27 s, 11.0 M, 0.84 GB | 1.05 / 0.15 s, 12.3 M, 0.33 GB | 1.09 / 0.65 s, 13.1 M, 0.34 GB |
| `strip-32` | 4.56 / 1.01 s, 37.9 M, 2.03 GB | 5.98 / 0.82 s, 42.3 M, 0.97 GB | 5.38 / 2.39 s, 44.5 M, 1.05 GB |
| `strip-40` | 17.6 / 3.23 s, 97.4 M, 3.76 GB | 24.0 / 3.50 s, 112.0 M, 2.27 GB | 17.8 / 7.98 s, 111.8 M, 2.50 GB |

Every solve's residual was at most 5e-13, unrefined, and no pivot was perturbed.

- **The fill is PARDISO's and MUMPS's, or less, in 3D:** 0.89 to 0.93 times PARDISO's entries on
  the strips, from 4.5 to 4.9 times with faer's LU. The separators are what did it: the smallest
  vertex set across each cut (König's theorem) took the entries from 1.3 times PARDISO's to 0.9.
  In 2D, METIS's ordering still beats our geometric one (1.7 times PARDISO's entries, MUMPS's
  own).
- **One thread:** faster than PARDISO on every strip (1.2 to 1.6 times) and level with MUMPS or
  faster, as LU and as L D Lᵀ; level with PARDISO on the slab. From faer's LU: 2 to 7 times
  faster.
- **20 threads:** faster than PARDISO on the two larger strips as LU (1.2 and 2.3 times) and on
  `strip-40` as L D Lᵀ; slower on the slab, on `strip-24`, and on `strip-32` as L D Lᵀ (1.2 to
  1.8 times PARDISO's time), where the fronts are small and faer's threaded dense kernels are
  least efficient (below). From faer's LU: 7.5 to 9.3 times faster.
- **Memory:** the factors are fewer than PARDISO's, but the peak on 20 threads is 1.3 to 2.5
  times PARDISO's: each front is assembled as a full dense matrix, its factors and Schur
  complement copied out, and the fronts on separate threads are in memory at once. On one thread
  the peak on `strip-40` is 4.0 GB as LU (PARDISO 3.8 GB) and 2.8 GB as L D Lᵀ (2.1 GB).

What is left, in order: the peak memory (assembling a front in place of its children's updates,
the update stack of Duff and Reid), the 20-thread efficiency on small fronts (faer's threaded
dense LU, item 4 below), and METIS-quality separators in 2D.

## 0.4.2: faer's LU (as measured then)

### Summary

- **On one thread, the gap is the fill, not the arithmetic.** faer's LU makes room for 3.3 to 4.9
  times as many entries as PARDISO stores (1.9 to 5.0 times MUMPS's), and is 1.8 to 7 times
  slower. Its dense kernel runs at 67 to 92% of MKL's speed on one thread.
- **Why the fill is larger.** faer pivots by rows anywhere in a column, so it reserves the
  structure of AᵀA's Cholesky factor (George & Ng 1987). PARDISO and MUMPS factor on the structure
  of A + Aᵀ and keep their pivots inside it.
- **Symmetry halves it again.** All four matrices become complex symmetric under a diagonal
  similarity. As complex symmetric matrices, PARDISO and MUMPS store half the entries and factor
  them 1.1 to 1.9 times faster on one thread.
- **On 20 threads photonoxide is 4.1 to 11 times slower than PARDISO,** the most on the smaller
  systems. faer's dense LU reaches 13 to 40% of MKL's speed on 20 threads, least on small
  matrices, and on the 2D slab photonoxide's factorization is slower on 20 threads than on one.
- **Memory follows the fill.** On the 40³ strip photonoxide's peak is 16.8 GB, almost all of it
  factors. PARDISO's is 3.8 GB, and 2.1 GB as complex symmetric (one thread).

## The systems

`photonoxide bench --export <dir>` writes each direct solver's system (`photonoxide::bench::export`)
as Matrix Market files: A, b and the solution x, refined to round-off. For each, it also writes
the complex symmetric form in `<id>/symmetric/`: B = S A S⁻¹ with S² = D, the diagonal for which
D A is symmetric (see [FDFD in 3D](methods/fdfd-3d.md#the-iterative-solver)), with S b and S x.

| Id | Solver | Grid | Unknowns | Nonzeros |
|---|---|---|---:|---:|
| `slab-2d` | 2D FDFD | 440 × 340 cells of 10 nm, PMLs of 20, E along z | 149 600 | 746 440 |
| `strip-24` | 3D direct | 24 × 24 × 24 cells of 40 nm, PMLs of 6 | 41 472 | 479 208 |
| `strip-32` | 3D direct | 32 × 32 × 32 cells of 40 nm, PMLs of 6 | 98 304 | 1 170 408 |
| `strip-40` | 3D direct | 40 × 40 × 40 cells of 40 nm, PMLs of 6 | 192 000 | 2 327 016 |

The 3D strip is a silicon strip, 0.5 × 0.22 µm, in oxide, the one in
[FDFD in 3D](methods/fdfd-3d.md#cost).

## How it was run

**Machine.** Intel Core Ultra 7 265K, 20 logical processors, under WSL 2 (Ubuntu 24.04,
31 GB visible). The same machine as [the benchmarks](benchmarks.md). On 2026-10-05.

**Each solver in a process of its own,** each phase timed separately: analysis (ordering and
symbolic factorization), numeric factorization, and one solve. Only the factorization is compared
below. Peak memory is the process's peak resident set (VmHWM). Threads were set by
`MKL_NUM_THREADS`, `OMP_NUM_THREADS` and `RAYON_NUM_THREADS`.

- **PARDISO**, from MKL 2020.4 (Ubuntu's `libmkl-dev`):
  - complex unsymmetric, `mtype` 13, with METIS nested dissection (`iparm[1]` = 2);
  - complex symmetric, `mtype` 6 (Bunch–Kaufman pivoting), given the upper triangle;
  - no iterative refinement (`iparm[7]` = 0), so its raw factors are timed as photonoxide's are,
    and the defaults otherwise (scaling and matching on for `mtype` 13);
  - with MKL's GNU OpenMP layer (`MKL_THREADING_LAYER=GNU`): on Intel's OpenMP this MKL build
    failed PARDISO's analysis (error −3, internal −180) on two threads or more.
- **MUMPS** 5.6.2, the sequential library (Ubuntu's `libmumps-seq-dev`):
  - general, `sym` = 0; complex symmetric, `sym` = 2, given one triangle;
  - the ordering MUMPS chooses (`ICNTL(7)` = 7), which was SCOTCH (`INFOG(7)` = 3) on every system.
  - Its only threads are the BLAS's (MKL here). MUMPS's own parallelism over the tree needs its
    MPI build, so its 20-thread times understate it.
- **photonoxide** at this commit, faer 0.24.4: `bench::export::factorize`, faer's supernodal LU in
  the solvers' own order, COLAMD for `slab-2d` and nested dissection for the strips. Its "factor
  entries" are the entries of the structure faer's LU works in (`Factorized::factor_entries`), about
what it stores: 16.8 GB of peak memory for 1 065 M entries on `strip-40`.

The drivers that call PARDISO and MUMPS are short C programs. They live outside this repository,
which is Rust only. Each one:

1. reads `A.mtx`, `b.mtx` and `x.mtx`;
2. builds the solver's input (1-based CSR for PARDISO, coordinates for MUMPS);
3. times the three phases;
4. prints one line of JSON: the times, the solver's count of factor entries (PARDISO's
   `iparm[17]`, MUMPS's `INFOG(9)`), peak memory, the residual ‖b − A x‖/‖b‖, and the difference
   from photonoxide's solution.

**Every solve was accurate.** Every residual was at most 1.5e-12, and every solution was within
5.1e-13 of photonoxide's refined one, relative.

## 0.4.2 results: the general LU

Numeric factorization in seconds on 1 and on 20 threads, the factors' entries (L and U), and peak
memory on 20 threads:

| System | Solver | 1 thread | 20 threads | Factor entries | Peak memory |
|---|---|---:|---:|---:|---:|
| `slab-2d` | PARDISO | 0.31 s | 0.077 s | 9.2 M | 0.39 GB |
| | MUMPS | 0.36 s | 0.29 s | 15.8 M | 0.35 GB |
| | photonoxide | 0.64 s | 0.85 s | 30.2 M | 0.63 GB |
| `strip-24` | PARDISO | 1.82 s | 0.28 s | 23.7 M | 0.56 GB |
| | MUMPS | 1.53 s | 1.98 s | 25.5 M | 0.53 GB |
| | photonoxide | 8.56 s | 2.69 s | 107.4 M | 1.83 GB |
| `strip-32` | PARDISO | 11.1 s | 1.67 s | 82.4 M | 1.65 GB |
| | MUMPS | 7.89 s | 2.64 s | 84.5 M | 1.76 GB |
| | photonoxide | 55.4 s | 10.9 s | 393.5 M | 6.34 GB |
| `strip-40` | PARDISO | 43.8 s | 8.45 s | 219.2 M | 4.07 GB |
| | MUMPS | 29.3 s | 9.23 s | 214.3 M | 4.29 GB |
| | photonoxide | 199.6 s | 35.0 s | 1 064.8 M | 16.80 GB |

The analysis took 0.2 to 0.7 s for PARDISO and MUMPS, and 0.03 to 0.09 s for photonoxide. One
solve took 0.01 to 0.3 s for all three. PARDISO's factorization of `strip-32` is 477 Gflop by its
own count: 44 Gflop/s on one thread, 283 on 20.

## 0.4.2 results: as complex symmetric matrices

The same matrices made symmetric by S, factorized as such. The entries counted are L's (with D):

| System | Solver | 1 thread | 20 threads | Factor entries | Peak memory | Against the general LU (1 thread) |
|---|---|---:|---:|---:|---:|---:|
| `slab-2d` | PARDISO | 0.20 s | 0.047 s | 5.0 M | 0.29 GB | 1.6 × |
| | MUMPS | 0.31 s | 0.34 s | 9.7 M | 0.26 GB | 1.1 × |
| `strip-24` | PARDISO | 1.05 s | 0.15 s | 12.3 M | 0.33 GB | 1.7 × |
| | MUMPS | 1.09 s | 0.65 s | 13.1 M | 0.34 GB | 1.4 × |
| `strip-32` | PARDISO | 5.98 s | 0.82 s | 42.3 M | 0.97 GB | 1.9 × |
| | MUMPS | 5.38 s | 2.39 s | 44.5 M | 1.05 GB | 1.5 × |
| `strip-40` | PARDISO | 24.0 s | 3.50 s | 112.0 M | 2.27 GB | 1.8 × |
| | MUMPS | 17.8 s | 7.98 s | 111.8 M | 2.50 GB | 1.6 × |

## Results: the dense kernel (faer against MKL)

Each supernode is factorized by a dense LU, so the dense kernel sets the speed for a given fill.
The table compares faer's `partial_piv_lu` with MKL's `zgetrf` on a random complex n × n matrix,
the best of three runs, rated by 8n³/3 real flops in Gflop/s. PARDISO's largest supernode on
`strip-32` has 2 529 columns.

| n | MKL, 1 thread | faer, 1 thread | MKL, 20 threads | faer, 20 threads |
|---:|---:|---:|---:|---:|
| 500 | 62.8 | 42.4 (68%) | 210 | 27.5 (13%) |
| 1 000 | 64.7 | 49.1 (76%) | 350 | 68.6 (20%) |
| 2 500 | 67.1 | 61.0 (91%) | 696 | 161 (23%) |
| 4 000 | 70.3 | 64.5 (92%) | 716 | 287 (40%) |

## Where the difference came from (0.4.2)

**The fill.** faer's sparse LU pivots by rows, choosing any row of a column. Wherever the pivot
lands, L and U stay inside the structure of the Cholesky factor of AᵀA (A. George, E. Ng, SIAM J.
Sci. Stat. Comput. 8, 877 (1987), doi:10.1137/0908072). faer's storage is about that structure's size.

AᵀA's graph joins two columns that share a row. On a 3D curl-curl stencil it is far denser than
A's own graph, so its separators are wider and its fill larger. That is also why our nested
dissection needs separators two steps wide (see [FDFD in 3D](methods/fdfd-3d.md#cost)).

PARDISO and MUMPS work on the structure of A + Aᵀ, ordered on its graph, which is as sparse as A.
They keep pivoting inside that structure:

- **PARDISO** first permutes large entries onto the diagonal and scales the matrix (matching).
  It then pivots only within each supernode and perturbs a pivot that is too small. Iterative
  refinement repairs the perturbation.
- **MUMPS** pivots within each front, with a threshold. A pivot that fails the threshold is
  delayed to the parent front.

On these meshes this gives 3.3 to 4.9 times fewer entries, and the gap grows with the grid
(4.5 at 24³, 4.8 at 32³, 4.9 at 40³).

**The kernels.** On one thread faer's dense LU is within 10% of MKL's at the sizes of large
supernodes, and within a third at small ones. So on one thread photonoxide's 1.8 to 7 times is
almost all fill.

**The threads.** On 20 threads the picture changes:

- faer's dense LU reaches 13 to 40% of MKL's speed, and at n = 500 it is slower than on one
  thread.
- The penalty falls on small fronts. From 1 to 20 threads photonoxide's factorization speeds up
  5.1 times on `strip-32` and 5.7 on `strip-40`, against PARDISO's 6.6 and 5.2: on the largest
  strip, where large fronts take most of the time, photonoxide scales as well as PARDISO. On
  `strip-24` it speeds up 3.2 times, against PARDISO's 6.5.
- On the 2D slab, whose supernodes are small, photonoxide is slower on 20 threads than on one.

## What would close it, in pure Rust (0.4.2), and what 0.4.3 did

In order of gain. The first two are about fill and help on any number of threads; the last two
are about threads.

**What 0.4.3 did:** items 1, 2 and 3, in photonoxide's own `sparse` module ([the 0.4.3
comparison](#043-the-multifrontal-factorization) above). Item 2 is written here, not in faer: an
unconjugated L D Lᵀ is a front's dense kernel, built on faer's triangular products, and it takes
its pivots on the diagonal, as static pivoting does, falling back to the LU if a pivot would
need perturbing, so it needed no Bunch–Kaufman pivoting. Item 4 is still to do.

1. **An LU on the symmetric structure.** Order A + Aᵀ by nested dissection on its own graph: our
   `sparse::nested_dissection` with one-step separators, which lost to COLAMD only because faer
   factors AᵀA's structure. Then factor it supernodally or multifrontally, with pivoting kept
   inside the supernodes:
   - **static pivoting** in PARDISO's way: a matching and scaling first, small pivots perturbed,
     and refinement after;
   - or **threshold pivoting with delayed pivots**, in MUMPS's way.

   photonoxide already has the safety net static pivoting needs: a 3D solve refines its solution,
   and finishes with QMR preconditioned by the factors when refinement isn't enough (#82). faer's
   dense LU and triangular kernels do the arithmetic inside the supernodes.

   Expected: the fill of PARDISO's and MUMPS's general LU, 4 to 5 times fewer entries and less
   memory. On one thread it should be close to PARDISO's time, since the kernels are within 10%:
   about 4 to 5 times faster on the 3D strip.
2. **A complex symmetric LDLᵀ** (Bunch–Kaufman pivoting: J. R. Bunch, L. Kaufman, Math. Comp. 31,
   163 (1977), doi:10.1090/S0025-5718-1977-0428694-0) on B = S A S⁻¹. It is the same structure as
   item 1 with half the storage. For PARDISO it was 1.6 to 1.9 times faster than its own general
   LU.

   faer has the Hermitian version (LBLᴴ) but not the unconjugated one that B needs. Since
   photonoxide's linear algebra is faer, the unconjugated variant belongs upstream in faer, as a
   general method.
3. **Parallelism over the tree.** Nested dissection's tree is balanced and its subtrees are
   independent. Factor them on separate threads (rayon's `join` follows the recursion), and leave
   threaded dense kernels to the few large fronts near the root. This should help most in 2D and
   on small fronts, where photonoxide now loses time to threads.
4. **faer's threaded dense LU.** At 13 to 40% of MKL's speed on 20 threads, it limits the large
   fronts near the root. Measure it in faer's own benchmarks and report it upstream, rather than
   write a dense LU here.

Item 2 needs item 1's structure. Items 1 and 3 can be done in photonoxide now, item 2 waits on
faer, and item 4 belongs to faer. Each item is judged by this comparison again, after rewriting the
exported systems.

Block low-rank compression is the next step after these, as in MUMPS (P. R. Amestoy et al., SIAM
J. Sci. Comput. 37, A1451 (2015), doi:10.1137/120903476; for 3D electromagnetics, D. V. Shantsev
et al., Geophys. J. Int. 209, 1558 (2017), doi:10.1093/gji/ggx106). It is in the performance plan,
not here.

## Licences

- MKL is under the Intel Simplified Software License.
- MUMPS is under CeCILL-C. It asks that work using it cite it (above) and notify its authors on
  publication.

Neither licence restricts comparisons, and neither code was read for photonoxide.

# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. Bandwidth is the iterating phases' bytes over their time, the bytes counted by the kernels themselves (each nonzero's value and index, each row pointer, each vector read or written once: src/traffic.rs), from memory or from cache; beside it, its share of the triad on the same threads, the roof (Williams et al., Commun. ACM 52(4), 65 (2009)). A problem whose vectors fit in the last-level cache can count above it: the 40³ guide's do. FDTD counts the bytes of a step of the whole grid, so its diamonds, which step a tile several times in cache, count above it too: the 160³ guide's. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.3, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 33.0 GB/s on 1 thread, 54.8 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Bandwidth (of the triad) | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.79 s | — | — | — | 2.7e-13 (S21 = exp(iβL), S11 = 0) | 373 MB |
| `fdfd3d/guide-direct` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 18.87 s | — | — | — | 2.2e-14 (its relative residual) | 2.92 GB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 12.11 s | 2 077 | 27.2 | 21.6 GB/s (65%) | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 6.02 s | 195 | 121.8 | 15.8 GB/s (48%) | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 3.33 s | 16 | 268.2 | 13.7 GB/s (41%) | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.03 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 1 | 22.26 s | 46 | 278.4 | 15.1 GB/s (46%) | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.65 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 1 | 52.36 s | 56 | 275.2 | 14.3 GB/s (43%) | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.44 GB |
| `fdtd3d/box-f64` | 48 × 48 × 48 cells of 50 nm, CPMLs of 8, vacuum | 110 592 | 1 | 0.47 s | 400 | 10.5 | 35.0 GB/s (106%) | 0.0e0 (the plain loops: the same bits) | 38 MB |
| `fdtd3d/box-f32` | 48 × 48 × 48 cells of 50 nm, CPMLs of 8, vacuum | 110 592 | 1 | 0.37 s | 400 | 8.5 | 21.7 GB/s (66%) | 3.9e-7 (f64, relative to the largest E) | 31 MB |
| `fdtd3d/guide-f64` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 1 | 1.39 s | 60 | 5.6 | 43.4 GB/s (131%) | 0.0e0 (the plain loops: the same bits) | 876 MB |
| `fdtd3d/guide-f32` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 1 | 1.18 s | 60 | 4.8 | 25.4 GB/s (77%) | 3.4e-7 (f64, relative to the largest E) | 660 MB |
| `fdtd3d/guide-whole-f64` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 1 | 2.97 s | 60 | 12.1 | 20.3 GB/s (61%) | 0.0e0 (the plain loops: the same bits) | 875 MB |
| `fdtd3d/guide-whole-f32` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 1 | 1.63 s | 60 | 6.6 | 18.5 GB/s (56%) | 3.4e-7 (f64, relative to the largest E) | 659 MB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 1 | 4.04 s | — | — | — | — | 138 MB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 1 | 22.61 s | — | — | — | — | 833 MB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 6.10 s | — | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 7.27 s | — | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 10.57 s | — | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.45 s | — | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 0.63 s | — | — | — | 2.7e-13 (S21 = exp(iβL), S11 = 0) | 415 MB |
| `fdfd3d/guide-direct` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 4.62 s | — | — | — | 2.2e-14 (its relative residual) | 3.26 GB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 4.10 s | 2 077 | 7.1 | 82.6 GB/s (151%) | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 4.79 s | 195 | 89.0 | 21.6 GB/s (39%) | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 2.49 s | 16 | 161.1 | 22.8 GB/s (42%) | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.05 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 20 | 12.51 s | 46 | 112.8 | 37.3 GB/s (68%) | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.68 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 20 | 29.32 s | 56 | 114.3 | 34.4 GB/s (63%) | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.51 GB |
| `fdtd3d/box-f64` | 48 × 48 × 48 cells of 50 nm, CPMLs of 8, vacuum | 110 592 | 20 | 0.19 s | 400 | 4.4 | 83.8 GB/s (153%) | 0.0e0 (the plain loops: the same bits) | 39 MB |
| `fdtd3d/box-f32` | 48 × 48 × 48 cells of 50 nm, CPMLs of 8, vacuum | 110 592 | 20 | 0.16 s | 400 | 3.7 | 49.6 GB/s (91%) | 3.9e-7 (f64, relative to the largest E) | 32 MB |
| `fdtd3d/guide-f64` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 20 | 0.28 s | 60 | 1.1 | 216.8 GB/s (395%) | 0.0e0 (the plain loops: the same bits) | 877 MB |
| `fdtd3d/guide-f32` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 20 | 0.16 s | 60 | 0.7 | 186.3 GB/s (340%) | 3.4e-7 (f64, relative to the largest E) | 661 MB |
| `fdtd3d/guide-whole-f64` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 20 | 1.05 s | 60 | 4.3 | 57.1 GB/s (104%) | 0.0e0 (the plain loops: the same bits) | 876 MB |
| `fdtd3d/guide-whole-f32` | 160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide | 4 096 000 | 20 | 0.51 s | 60 | 2.1 | 59.5 GB/s (108%) | 3.4e-7 (f64, relative to the largest E) | 660 MB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 20 | 1.18 s | — | — | — | — | 1.10 GB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 20 | 8.95 s | — | — | — | — | 4.00 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 5.57 s | — | — | — | — | 1.46 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 2.42 s | — | — | — | — | 758 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 2.96 s | — | — | — | — | 1.01 GB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.43 s | — | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and factorization 0.63 s; port modes 0.00 s; S-matrix, 2 runs 0.15 s
- `fdfd3d/guide-direct` on 1 thread: assembly, analysis and L D Lᵀ 18.22 s; one solve, refined 0.65 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.27 s; QMR to 1e-6 10.84 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.46 s; QMR to 1e-8 4.56 s (195 iterations)
- `fdfd3d/guide-multigrid` on 1 thread: assembly and multigrid 2.51 s; GMRES to 1e-8 0.82 s (16 iterations)
- `fdfd3d/diel-multigrid` on 1 thread: assembly and multigrid 11.20 s; GMRES to 1e-8 11.06 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 1 thread: assembly and multigrid 23.88 s; port modes 0.64 s; S-matrix, 2 runs of GMRES to 1e-8 27.84 s (56 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and factorization 0.47 s; port modes 0.00 s; S-matrix, 2 runs 0.16 s
- `fdfd3d/guide-direct` on 20 threads: assembly, analysis and L D Lᵀ 3.98 s; one solve, refined 0.64 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.26 s; QMR to 1e-6 2.83 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.46 s; QMR to 1e-8 3.33 s (195 iterations)
- `fdfd3d/guide-multigrid` on 20 threads: assembly and multigrid 2.00 s; GMRES to 1e-8 0.49 s (16 iterations)
- `fdfd3d/diel-multigrid` on 20 threads: assembly and multigrid 8.03 s; GMRES to 1e-8 4.48 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 20 threads: assembly and multigrid 16.91 s; port modes 0.84 s; S-matrix, 2 runs of GMRES to 1e-8 11.57 s (56 iterations)

## Problems

- `fdfd2d/slab-lu`: 2D FDFD by the sparse direct solver: a straight 220 nm silicon slab in oxide, E along z, its two ports' S-matrix
- `fdfd3d/guide-direct`: 3D FDFD by the direct solver (L D Lᵀ of the curl-curl operator's symmetric similarity), plain PMLs: a 100 nm silicon guide through a 40³ grid, a dipole beside it
- `fdfd3d/guide-qmr`: 3D FDFD by QMR on the curl-curl operator, plain PMLs: a 100 nm silicon guide through a 40³ grid, a dipole beside it, to a residual of 1e-6
- `fdfd3d/guide-ilu`: 3D FDFD by QMR + ILU(0) on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `fdfd3d/guide-multigrid`: 3D FDFD by GMRES + multigrid on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `fdfd3d/diel-multigrid`: 3D FDFD by GMRES + multigrid, stretched PMLs: Shin and Fan's Diel, smaller (a 400 × 300 nm silicon guide in vacuum, a current across it), to 1e-8
- `fdfd3d/strip-ports-multigrid`: 3D FDFD by GMRES + multigrid, stretched PMLs: a straight 500 × 220 nm silicon strip in oxide, its two ports' S-matrix, to 1e-8
- `fdtd3d/box-f64`: FDTD's kernel in f64: 48³ cells of 50 nm of vacuum (in the last-level cache), CPMLs of 8, 400 steps from a bump of E, against the plain loops
- `fdtd3d/box-f32`: FDTD's kernel in f32: the same box, against f64
- `fdtd3d/guide-f64`: FDTD's kernel in f64: 160³ cells of 20 nm (well beyond the cache), a 500 × 220 nm silicon guide, CPMLs of 8, 60 steps from a bump of E, against the plain loops
- `fdtd3d/guide-f32`: FDTD's kernel in f32: the same guide, against f64
- `fdtd3d/guide-whole-f64`: FDTD's kernel in f64 on the same guide, the whole grid every step, without the tiles: what blocking gains
- `fdtd3d/guide-whole-f32`: FDTD's kernel in f32 on the same guide, the whole grid every step, against f64
- `job/strip-modes`: The built-in job jobs/strip-modes.toml, run headless: a strip's modes by the full-vector solver, and a sweep of 11 wavelengths
- `job/mmi-fdfd`: The built-in job jobs/mmi-fdfd.toml, run headless: a 1 × 2 MMI's S-parameters by 2D FDFD over a sweep of 11 wavelengths
- `example/strip_waveguide`: TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector, at 20, 10 and 5 nm grids (examples/strip_waveguide.rs)
- `example/hadley_corners`: Four waveguides with dielectric corners (boxes and impinged corners, ε = 2.25 and 8), full-vector with mirror walls, at 8 to 128 cells a side: the standard scheme (about first order at the corners) against Hadley's high-accuracy equations (about second order) (examples/hadley_corners.rs)
- `example/group_index`: The group index of 220 nm silicon strips, 400–600 nm wide, at 1.55 µm, with the book's dispersive silicon; the TE-like mode tracked over wavelength, on a quarter domain (examples/group_index.rs)
- `example/circuit_fit`: An add-drop ring's couplings, loss and radius fitted to its through and drop spectra, by L-BFGS-B with the adjoint gradient and by CMA-ES without it, counting evaluations (examples/circuit_fit.rs)

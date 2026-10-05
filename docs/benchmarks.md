# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. Bandwidth is the iterating phases' bytes over their time, the bytes counted by the kernels themselves (each nonzero's value and index, each row pointer, each vector read or written once: src/traffic.rs), from memory or from cache; beside it, its share of the triad on the same threads, the roof (Williams et al., Commun. ACM 52(4), 65 (2009)). A problem whose vectors fit in the last-level cache can count above it: the 40³ guide's do. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.2, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 32.8 GB/s on 1 thread, 55.3 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Bandwidth (of the triad) | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.81 s | — | — | — | 2.7e-13 (S21 = exp(iβL), S11 = 0) | 373 MB |
| `fdfd3d/guide-direct` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 19.68 s | — | — | — | 2.2e-14 (its relative residual) | 2.92 GB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 13.24 s | 2 077 | 29.9 | 19.7 GB/s (60%) | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 6.54 s | 195 | 134.4 | 14.3 GB/s (44%) | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 3.50 s | 16 | 284.6 | 12.9 GB/s (39%) | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.02 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 1 | 23.35 s | 46 | 293.4 | 14.4 GB/s (44%) | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.65 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 1 | 55.06 s | 56 | 289.6 | 13.6 GB/s (41%) | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.39 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 1 | 4.19 s | — | — | — | — | 137 MB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 1 | 22.56 s | — | — | — | — | 831 MB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 6.21 s | — | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 7.44 s | — | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 10.81 s | — | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.48 s | — | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 0.62 s | — | — | — | 2.7e-13 (S21 = exp(iβL), S11 = 0) | 423 MB |
| `fdfd3d/guide-direct` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 5.27 s | — | — | — | 2.2e-14 (its relative residual) | 3.46 GB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 3.99 s | 2 077 | 6.8 | 86.3 GB/s (156%) | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 4.67 s | 195 | 86.0 | 22.4 GB/s (40%) | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 2.48 s | 16 | 155.2 | 23.6 GB/s (43%) | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.06 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 20 | 12.39 s | 46 | 110.5 | 38.1 GB/s (69%) | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.67 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 20 | 29.41 s | 56 | 114.9 | 34.3 GB/s (62%) | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.46 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 20 | 1.15 s | — | — | — | — | 1.11 GB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 20 | 8.47 s | — | — | — | — | 4.01 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 5.60 s | — | — | — | — | 1.46 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 2.46 s | — | — | — | — | 764 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 3.03 s | — | — | — | — | 999 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.44 s | — | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and factorization 0.64 s; port modes 0.00 s; S-matrix, 2 runs 0.16 s
- `fdfd3d/guide-direct` on 1 thread: assembly, analysis and L D Lᵀ 19.08 s; one solve, refined 0.60 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.33 s; QMR to 1e-6 11.91 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.51 s; QMR to 1e-8 5.03 s (195 iterations)
- `fdfd3d/guide-multigrid` on 1 thread: assembly and multigrid 2.63 s; GMRES to 1e-8 0.87 s (16 iterations)
- `fdfd3d/diel-multigrid` on 1 thread: assembly and multigrid 11.69 s; GMRES to 1e-8 11.66 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 1 thread: assembly and multigrid 25.10 s; port modes 0.65 s; S-matrix, 2 runs of GMRES to 1e-8 29.30 s (56 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and factorization 0.47 s; port modes 0.00 s; S-matrix, 2 runs 0.16 s
- `fdfd3d/guide-direct` on 20 threads: assembly, analysis and L D Lᵀ 4.66 s; one solve, refined 0.61 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.28 s; QMR to 1e-6 2.71 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.45 s; QMR to 1e-8 3.22 s (195 iterations)
- `fdfd3d/guide-multigrid` on 20 threads: assembly and multigrid 2.00 s; GMRES to 1e-8 0.48 s (16 iterations)
- `fdfd3d/diel-multigrid` on 20 threads: assembly and multigrid 8.00 s; GMRES to 1e-8 4.39 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 20 threads: assembly and multigrid 16.95 s; port modes 0.83 s; S-matrix, 2 runs of GMRES to 1e-8 11.63 s (56 iterations)

## Problems

- `fdfd2d/slab-lu`: 2D FDFD by the sparse direct solver: a straight 220 nm silicon slab in oxide, E along z, its two ports' S-matrix
- `fdfd3d/guide-direct`: 3D FDFD by the direct solver (L D Lᵀ of the curl-curl operator's symmetric similarity), plain PMLs: a 100 nm silicon guide through a 40³ grid, a dipole beside it
- `fdfd3d/guide-qmr`: 3D FDFD by QMR on the curl-curl operator, plain PMLs: a 100 nm silicon guide through a 40³ grid, a dipole beside it, to a residual of 1e-6
- `fdfd3d/guide-ilu`: 3D FDFD by QMR + ILU(0) on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `fdfd3d/guide-multigrid`: 3D FDFD by GMRES + multigrid on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `fdfd3d/diel-multigrid`: 3D FDFD by GMRES + multigrid, stretched PMLs: Shin and Fan's Diel, smaller (a 400 × 300 nm silicon guide in vacuum, a current across it), to 1e-8
- `fdfd3d/strip-ports-multigrid`: 3D FDFD by GMRES + multigrid, stretched PMLs: a straight 500 × 220 nm silicon strip in oxide, its two ports' S-matrix, to 1e-8
- `job/strip-modes`: The built-in job jobs/strip-modes.toml, run headless: a strip's modes by the full-vector solver, and a sweep of 11 wavelengths
- `job/mmi-fdfd`: The built-in job jobs/mmi-fdfd.toml, run headless: a 1 × 2 MMI's S-parameters by 2D FDFD over a sweep of 11 wavelengths
- `example/strip_waveguide`: TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector, at 20, 10 and 5 nm grids (examples/strip_waveguide.rs)
- `example/hadley_corners`: Four waveguides with dielectric corners (boxes and impinged corners, ε = 2.25 and 8), full-vector with mirror walls, at 8 to 128 cells a side: the standard scheme (about first order at the corners) against Hadley's high-accuracy equations (about second order) (examples/hadley_corners.rs)
- `example/group_index`: The group index of 220 nm silicon strips, 400–600 nm wide, at 1.55 µm, with the book's dispersive silicon; the TE-like mode tracked over wavelength, on a quarter domain (examples/group_index.rs)
- `example/circuit_fit`: An add-drop ring's couplings, loss and radius fitted to its through and drop spectra, by L-BFGS-B with the adjoint gradient and by CMA-ES without it, counting evaluations (examples/circuit_fit.rs)

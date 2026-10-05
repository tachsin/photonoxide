# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. Bandwidth is the iterating phases' bytes over their time, the bytes counted by the kernels themselves (each nonzero's value and index, each row pointer, each vector read or written once: src/traffic.rs), from memory or from cache; beside it, its share of the triad on the same threads, the roof (Williams et al., Commun. ACM 52(4), 65 (2009)). A problem whose vectors fit in the last-level cache can count above it: the 40³ guide's do. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.1, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 32.9 GB/s on 1 thread, 56.3 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Bandwidth (of the triad) | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.92 s | — | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 690 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 13.39 s | 2 077 | 30.3 | 19.4 GB/s (59%) | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 6.27 s | 195 | 126.0 | 15.3 GB/s (46%) | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 3.52 s | 16 | 288.5 | 12.7 GB/s (39%) | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.03 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 1 | 23.72 s | 46 | 303.6 | 13.9 GB/s (42%) | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.65 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 1 | 55.66 s | 56 | 296.6 | 13.3 GB/s (40%) | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.39 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 1 | 4.19 s | — | — | — | — | 138 MB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 1 | 24.63 s | — | — | — | — | 1.42 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 6.09 s | — | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 7.51 s | — | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 10.77 s | — | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.47 s | — | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 1.02 s | — | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 692 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 4.07 s | 2 077 | 7.0 | 84.4 GB/s (150%) | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 4.82 s | 195 | 89.2 | 21.6 GB/s (38%) | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 2.52 s | 16 | 169.6 | 21.6 GB/s (38%) | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.07 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 20 | 12.63 s | 46 | 113.6 | 37.1 GB/s (66%) | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.67 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 20 | 29.45 s | 56 | 114.6 | 34.3 GB/s (61%) | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.51 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 20 | 1.27 s | — | — | — | — | 1.09 GB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 20 | 9.28 s | — | — | — | — | 6.02 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 5.70 s | — | — | — | — | 1.46 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 2.48 s | — | — | — | — | 695 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 3.01 s | — | — | — | — | 990 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.43 s | — | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and LU 0.78 s; port modes 0.00 s; S-matrix, 2 runs 0.14 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.29 s; QMR to 1e-6 12.10 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.55 s; QMR to 1e-8 4.72 s (195 iterations)
- `fdfd3d/guide-multigrid` on 1 thread: assembly and multigrid 2.63 s; GMRES to 1e-8 0.89 s (16 iterations)
- `fdfd3d/diel-multigrid` on 1 thread: assembly and multigrid 11.65 s; GMRES to 1e-8 12.07 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 1 thread: assembly and multigrid 24.99 s; port modes 0.66 s; S-matrix, 2 runs of GMRES to 1e-8 30.01 s (56 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and LU 0.86 s; port modes 0.00 s; S-matrix, 2 runs 0.16 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.30 s; QMR to 1e-6 2.77 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.48 s; QMR to 1e-8 3.34 s (195 iterations)
- `fdfd3d/guide-multigrid` on 20 threads: assembly and multigrid 2.00 s; GMRES to 1e-8 0.52 s (16 iterations)
- `fdfd3d/diel-multigrid` on 20 threads: assembly and multigrid 8.12 s; GMRES to 1e-8 4.51 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 20 threads: assembly and multigrid 16.93 s; port modes 0.93 s; S-matrix, 2 runs of GMRES to 1e-8 11.60 s (56 iterations)

## Problems

- `fdfd2d/slab-lu`: 2D FDFD by sparse LU: a straight 220 nm silicon slab in oxide, E along z, its two ports' S-matrix
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

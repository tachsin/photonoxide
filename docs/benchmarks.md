# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.1, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 32.2 GB/s on 1 thread, 55.5 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.90 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 690 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 19.78 s | 2 059 | 47.2 | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 309 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 5.91 s | 195 | 119.1 | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 3.35 s | 16 | 275.6 | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.03 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 1 | 22.11 s | 46 | 271.4 | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.67 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 1 | 51.60 s | 56 | 265.8 | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.43 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 1 | 4.07 s | — | — | — | 138 MB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 1 | 24.28 s | — | — | — | 1.42 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 6.22 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 7.31 s | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 10.60 s | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.42 s | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 1.05 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 692 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 6.50 s | 2 059 | 13.5 | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 309 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 4.87 s | 195 | 90.7 | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 2.45 s | 16 | 154.0 | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.08 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 20 | 12.46 s | 46 | 111.0 | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.68 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 20 | 29.39 s | 56 | 115.2 | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.47 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 20 | 1.32 s | — | — | — | 1.10 GB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 20 | 8.72 s | — | — | — | 6.85 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 6.94 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 9.14 s | — | — | — | 237 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 13.11 s | — | — | — | 260 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.41 s | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and LU 0.76 s; port modes 0.00 s; S-matrix, 2 runs 0.14 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.14 s; QMR to 1e-6 18.64 s (2 059 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.45 s; QMR to 1e-8 4.46 s (195 iterations)
- `fdfd3d/guide-multigrid` on 1 thread: assembly and multigrid 2.51 s; GMRES to 1e-8 0.85 s (16 iterations)
- `fdfd3d/diel-multigrid` on 1 thread: assembly and multigrid 11.32 s; GMRES to 1e-8 10.79 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 1 thread: assembly and multigrid 24.08 s; port modes 0.63 s; S-matrix, 2 runs of GMRES to 1e-8 26.89 s (56 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and LU 0.88 s; port modes 0.00 s; S-matrix, 2 runs 0.17 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.14 s; QMR to 1e-6 5.35 s (2 059 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.47 s; QMR to 1e-8 3.40 s (195 iterations)
- `fdfd3d/guide-multigrid` on 20 threads: assembly and multigrid 1.98 s; GMRES to 1e-8 0.47 s (16 iterations)
- `fdfd3d/diel-multigrid` on 20 threads: assembly and multigrid 8.05 s; GMRES to 1e-8 4.41 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 20 threads: assembly and multigrid 16.88 s; port modes 0.85 s; S-matrix, 2 runs of GMRES to 1e-8 11.65 s (56 iterations)

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

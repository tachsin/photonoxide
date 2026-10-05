# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.1, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 33.2 GB/s on 1 thread, 52.5 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.90 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 690 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 12.17 s | 2 077 | 27.3 | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 5.82 s | 195 | 116.6 | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 3.35 s | 16 | 271.4 | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.03 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 1 | 22.61 s | 46 | 282.1 | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.64 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 1 | 52.65 s | 56 | 273.4 | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.43 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 1 | 4.17 s | — | — | — | 138 MB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 1 | 24.42 s | — | — | — | 1.42 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 6.17 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 7.49 s | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 10.81 s | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.46 s | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 1.88 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 691 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 4.12 s | 2 077 | 7.1 | 1.5e-7 (QMR + ILU(0) to 1e-12, relative) | 320 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 4.76 s | 195 | 88.3 | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 2.49 s | 16 | 166.7 | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.08 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 20 | 12.64 s | 46 | 113.8 | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.67 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 20 | 29.60 s | 56 | 115.4 | 4.1e-9 (S21 = exp(iβL), S11 = 0) | 9.46 GB |
| `job/strip-modes` | 121 × 111 cells of 20 nm, 11 points | 27 328 | 20 | 1.16 s | — | — | — | 1.09 GB |
| `job/mmi-fdfd` | 725 × 300 cells of 20 nm, 11 points | 217 500 | 20 | 8.50 s | — | — | — | 6.90 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 5.60 s | — | — | — | 1.46 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 2.49 s | — | — | — | 732 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 2.98 s | — | — | — | 1.01 GB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.42 s | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and LU 0.75 s; port modes 0.00 s; S-matrix, 2 runs 0.14 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.27 s; QMR to 1e-6 10.90 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.45 s; QMR to 1e-8 4.36 s (195 iterations)
- `fdfd3d/guide-multigrid` on 1 thread: assembly and multigrid 2.52 s; GMRES to 1e-8 0.83 s (16 iterations)
- `fdfd3d/diel-multigrid` on 1 thread: assembly and multigrid 11.39 s; GMRES to 1e-8 11.21 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 1 thread: assembly and multigrid 24.34 s; port modes 0.65 s; S-matrix, 2 runs of GMRES to 1e-8 27.66 s (56 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and LU 1.72 s; port modes 0.00 s; S-matrix, 2 runs 0.16 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.30 s; QMR to 1e-6 2.82 s (2 077 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.46 s; QMR to 1e-8 3.31 s (195 iterations)
- `fdfd3d/guide-multigrid` on 20 threads: assembly and multigrid 1.98 s; GMRES to 1e-8 0.51 s (16 iterations)
- `fdfd3d/diel-multigrid` on 20 threads: assembly and multigrid 8.11 s; GMRES to 1e-8 4.52 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 20 threads: assembly and multigrid 17.13 s; port modes 0.80 s; S-matrix, 2 runs of GMRES to 1e-8 11.67 s (56 iterations)

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

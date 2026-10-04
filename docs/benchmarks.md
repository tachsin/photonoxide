# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.1, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 32.2 GB/s on 1 thread, 57.0 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.91 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 690 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 26.46 s | 1 976 | 66.7 | 3.8e-7 (QMR + ILU(0) to 1e-12, relative) | 309 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 6.59 s | 195 | 135.9 | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 3.44 s | 16 | 275.1 | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.03 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 1 | 23.06 s | 46 | 287.1 | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.64 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 1 | 53.67 s | 54 | 287.3 | 6.2e-4 (S21 = exp(iβL), S11 = 0) | 9.41 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 6.13 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 7.39 s | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 10.76 s | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.46 s | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 1.01 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 692 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 17.40 s | 1 976 | 42.8 | 3.8e-7 (QMR + ILU(0) to 1e-12, relative) | 309 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 5.67 s | 195 | 111.0 | 1.2e-6 (GMRES + multigrid to 1e-12, relative) | 460 MB |
| `fdfd3d/guide-multigrid` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 2.54 s | 16 | 164.6 | 1.6e-6 (GMRES + multigrid to 1e-12, relative) | 1.07 GB |
| `fdfd3d/diel-multigrid` | 40 × 90 × 80 cells of 10 nm, stretched PMLs of 10 | 864 000 | 20 | 12.66 s | 46 | 112.8 | 1.8e-6 (GMRES + multigrid to 1e-12, relative) | 4.67 GB |
| `fdfd3d/strip-ports-multigrid` | 72 × 102 × 82 cells of 20 nm, stretched PMLs of 16 | 1 806 624 | 20 | 29.01 s | 54 | 115.2 | 6.2e-4 (S21 = exp(iβL), S11 = 0) | 9.47 GB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 6.92 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 9.14 s | — | — | — | 238 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 13.04 s | — | — | — | 260 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.41 s | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and LU 0.76 s; port modes 0.00 s; S-matrix, 2 runs 0.14 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.17 s; QMR to 1e-6 25.29 s (1 976 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.50 s; QMR to 1e-8 5.09 s (195 iterations)
- `fdfd3d/guide-multigrid` on 1 thread: assembly and multigrid 2.60 s; GMRES to 1e-8 0.85 s (16 iterations)
- `fdfd3d/diel-multigrid` on 1 thread: assembly and multigrid 11.65 s; GMRES to 1e-8 11.41 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 1 thread: assembly and multigrid 25.00 s; port modes 0.64 s; S-matrix, 2 runs of GMRES to 1e-8 28.03 s (54 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and LU 0.84 s; port modes 0.00 s; S-matrix, 2 runs 0.17 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.18 s; QMR to 1e-6 16.22 s (1 976 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.52 s; QMR to 1e-8 4.16 s (195 iterations)
- `fdfd3d/guide-multigrid` on 20 threads: assembly and multigrid 2.04 s; GMRES to 1e-8 0.51 s (16 iterations)
- `fdfd3d/diel-multigrid` on 20 threads: assembly and multigrid 8.17 s; GMRES to 1e-8 4.48 s (46 iterations)
- `fdfd3d/strip-ports-multigrid` on 20 threads: assembly and multigrid 16.97 s; port modes 0.80 s; S-matrix, 2 runs of GMRES to 1e-8 11.23 s (54 iterations)

## Problems

- `fdfd2d/slab-lu`: 2D FDFD by sparse LU: a straight 220 nm silicon slab in oxide, E along z, its two ports' S-matrix
- `fdfd3d/guide-qmr`: 3D FDFD by QMR on the curl-curl operator, plain PMLs: a 100 nm silicon guide through a 40³ grid, a dipole beside it, to a residual of 1e-6
- `fdfd3d/guide-ilu`: 3D FDFD by QMR + ILU(0) on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `fdfd3d/guide-multigrid`: 3D FDFD by GMRES + multigrid on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `fdfd3d/diel-multigrid`: 3D FDFD by GMRES + multigrid, stretched PMLs: Shin and Fan's Diel, smaller (a 400 × 300 nm silicon guide in vacuum, a current across it), to 1e-8
- `fdfd3d/strip-ports-multigrid`: 3D FDFD by GMRES + multigrid, stretched PMLs: a straight 500 × 220 nm silicon strip in oxide, its two ports' S-matrix, to 1e-8
- `example/strip_waveguide`: TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector, at 20, 10 and 5 nm grids (examples/strip_waveguide.rs)
- `example/hadley_corners`: Four waveguides with dielectric corners (boxes and impinged corners, ε = 2.25 and 8), full-vector with mirror walls, at 8 to 128 cells a side: the standard scheme (about first order at the corners) against Hadley's high-accuracy equations (about second order) (examples/hadley_corners.rs)
- `example/group_index`: The group index of 220 nm silicon strips, 400–600 nm wide, at 1.55 µm, with the book's dispersive silicon; the TE-like mode tracked over wavelength, on a quarter domain (examples/group_index.rs)
- `example/circuit_fit`: An add-drop ring's couplings, loss and radius fitted to its through and drop spectra, by L-BFGS-B with the adjoint gradient and by CMA-ES without it, counting evaluations (examples/circuit_fit.rs)

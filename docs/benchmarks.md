# Benchmarks

What `photonoxide bench` measured on one machine: the fixed problems of `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend on the machine and on what else ran on it; the errors don't. Peak memory is the process's peak resident set (its peak working set on Windows); ns per unknown per iteration is the QMR phases' time over the unknowns and the iterations. See the performance plan, docs/plans/performance.md.

- photonoxide 0.4.1, Intel(R) Core(TM) Ultra 7 265K, 20 logical processors, Windows
- memory bandwidth, the STREAM triad over three arrays of 128 MiB: 33.0 GB/s on 1 thread, 55.1 GB/s on 20 threads

| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Error | Peak memory |
|---|---|---:|---:|---:|---:|---:|---|---:|
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 1 | 0.89 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 690 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 1 | 25.39 s | 1 976 | 63.9 | 3.8e-7 (QMR + ILU(0) to 1e-12, relative) | 309 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 1 | 6.15 s | 195 | 124.5 | 1.2e-6 (QMR + ILU(0) to 1e-12, relative) | 460 MB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 1 | 8.11 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 1 | 8.50 s | — | — | — | 235 MB |
| `example/group_index` | examples/group_index.rs | — | 1 | 13.36 s | — | — | — | 258 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 1 | 0.47 s | — | — | — | 11 MB |
| `fdfd2d/slab-lu` | 440 × 340 cells of 10 nm, PMLs of 20 | 149 600 | 20 | 1.04 s | — | — | 2.6e-13 (S21 = exp(iβL), S11 = 0) | 692 MB |
| `fdfd3d/guide-qmr` | 40 × 40 × 40 cells of 10 nm, PMLs of 10 | 192 000 | 20 | 24.68 s | 1 976 | 62.0 | 3.8e-7 (QMR + ILU(0) to 1e-12, relative) | 309 MB |
| `fdfd3d/guide-ilu` | 40 × 40 × 40 cells of 10 nm, stretched PMLs of 10 | 192 000 | 20 | 5.86 s | 195 | 117.7 | 1.2e-6 (QMR + ILU(0) to 1e-12, relative) | 460 MB |
| `example/strip_waveguide` | examples/strip_waveguide.rs | — | 20 | 11.81 s | — | — | — | 1.45 GB |
| `example/hadley_corners` | examples/hadley_corners.rs | — | 20 | 14.15 s | — | — | — | 237 MB |
| `example/group_index` | examples/group_index.rs | — | 20 | 25.03 s | — | — | — | 260 MB |
| `example/circuit_fit` | examples/circuit_fit.rs | — | 20 | 0.41 s | — | — | — | 11 MB |

## Phases

- `fdfd2d/slab-lu` on 1 thread: assembly and LU 0.74 s; port modes 0.00 s; S-matrix, 2 runs 0.14 s
- `fdfd3d/guide-qmr` on 1 thread: assembly 1.17 s; QMR to 1e-6 24.23 s (1 976 iterations)
- `fdfd3d/guide-ilu` on 1 thread: assembly and ILU(0) 1.48 s; QMR to 1e-8 4.66 s (195 iterations)
- `fdfd2d/slab-lu` on 20 threads: assembly and LU 0.87 s; port modes 0.00 s; S-matrix, 2 runs 0.16 s
- `fdfd3d/guide-qmr` on 20 threads: assembly 1.14 s; QMR to 1e-6 23.53 s (1 976 iterations)
- `fdfd3d/guide-ilu` on 20 threads: assembly and ILU(0) 1.45 s; QMR to 1e-8 4.41 s (195 iterations)

## Problems

- `fdfd2d/slab-lu`: 2D FDFD by sparse LU: a straight 220 nm silicon slab in oxide, E along z, its two ports' S-matrix
- `fdfd3d/guide-qmr`: 3D FDFD by QMR on the curl-curl operator, plain PMLs: a 100 nm silicon guide through a 40³ grid, a dipole beside it, to a residual of 1e-6
- `fdfd3d/guide-ilu`: 3D FDFD by QMR + ILU(0) on Shin and Fan's operator, stretched PMLs: the same guide, to a residual of 1e-8
- `example/strip_waveguide`: TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector, at 20, 10 and 5 nm grids (examples/strip_waveguide.rs)
- `example/hadley_corners`: Four waveguides with dielectric corners (boxes and impinged corners, ε = 2.25 and 8), full-vector with mirror walls, at 8 to 128 cells a side: the standard scheme (about first order at the corners) against Hadley's high-accuracy equations (about second order) (examples/hadley_corners.rs)
- `example/group_index`: The group index of 220 nm silicon strips, 400–600 nm wide, at 1.55 µm, with the book's dispersive silicon; the TE-like mode tracked over wavelength, on a quarter domain (examples/group_index.rs)
- `example/circuit_fit`: An add-drop ring's couplings, loss and radius fitted to its through and drop spectra, by L-BFGS-B with the adjoint gradient and by CMA-ES without it, counting evaluations (examples/circuit_fit.rs)

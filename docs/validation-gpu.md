# Validation report: the GPU

Written by `cargo test --release --features gpu --test gpu_report -- --ignored` on tachsin's machine before each release; don't edit it by hand. GitHub's runners have no GPU, so CI doesn't run this report: [the main report](validation.md) is the same with or without one. A GPU run repeats bit for bit on the same device and driver, and agrees with the CPU to the tolerances below (see [FDTD](methods/fdtd.md#the-gpu)).

Devices: NVIDIA GeForce RTX 4060 (Vulkan, NVIDIA 616.56) in Single; NVIDIA GeForce RTX 4060 (Vulkan, NVIDIA 616.56) in Double.

## The GPU's cases

| Case | Tier | What | Against | Measured | Expected | Tolerance | Result |
|---|---|---|---|---|---|---|---|
| `gpu/f64-3d` | cross-code | In f64, a 3D run (37 × 21 × 19 cells of 50 nm: CPMLs with κ and α, a wall, conductivity, a conductor, sources on E and H̃, a current, a dipole, probes, a transform box, a flux box; 800 steps, fused and in two passes, in one run and in runs of 130): fields, probes, transforms and fluxes against the CPU's, relative (largest shown) | photonoxide's CPU kernel in f64, the same problem | 1.21e-14 | 0 | 1e-13 | pass |
| `gpu/f64-2d` | cross-code | In f64, the same in 2D (70 × 45 cells of 50 nm, periodic along x, CPMLs along y, a flux plane) | photonoxide's CPU kernel in f64, the same problem | 2.88e-15 | 0 | 1e-13 | pass |
| `gpu/f32-3d` | cross-code | In f32, the 3D run against the CPU's in f64, relative (largest shown) | photonoxide's CPU kernel in f64, the same problem | 3.60e-6 | 0 | 2e-5 | pass |
| `gpu/f32-2d` | cross-code | In f32, the 2D run against the CPU's in f64, relative (largest shown) | photonoxide's CPU kernel in f64, the same problem | 4.42e-6 | 0 | 2e-5 | pass |
| `gpu/f32-rounding` | cross-code | In f32, a closed box (40 × 36 × 32 cells of 50 nm, a block of ε = 12, random fields) after $N$ = 25 600 steps: E against the CPU's f32 kernel, relative, a random walk of rounding (tolerance $2 \times 10^{-7} \sqrt{N}$) | photonoxide's CPU kernel in f32, the same problem | 1.59e-5 | 0 | 3.2e-5 | pass |
| `gpu/f32-drift` | cross-code | In f32, the closed box's drift from f64 after 25 600 steps (the coefficients rounded to f32), over the CPU's f32 kernel's drift | photonoxide's CPU kernels in f32 and f64, the same problem | 1.00190 | 1.00000 | 5e-2 | pass |
| `gpu/f64-rounding` | cross-code | In f64, the closed box after $N$ = 25 600 steps: E against the CPU's, relative (tolerance $10^{-15} \sqrt{N}$) | photonoxide's CPU kernel in f64, the same problem | 2.88e-14 | 0 | 1.6e-13 | pass |
| `gpu/repeat-f32` | analytic | In f32, two runs of the 3D and 2D problems, 300 steps, fused and in two passes: values that differ (fields, probes, transforms) | no atomics, fixed workgroups, every sum by one invocation in step order: the same bits on the same device and driver | 0 | 0 | 0e0 | pass |
| `gpu/repeat-f64` | analytic | In f64, the same: values that differ | as above | 0 | 0 | 0e0 | pass |
| `gpu/roden-gedney` | published | In f32, Roden and Gedney's plate in soil (the `cpml_roden_gedney` example: 126 × 51 × 26 cells of 1 mm, its reference 206 × 131 × 106, 2000 steps), the traditional PML's largest error, dB | J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000), doi:10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A, "on the order of −48 dB" | -48.6199 | -48.0000 | 5e0 | pass |
| `gpu/roden-gedney-cfs` | published | The same with the CFS-PML (α = 0.05 S/m), dB | Roden and Gedney, as above: "−67 dB" | -70.4736 | -67.0000 | 5e0 | pass |
| `gpu/roden-gedney-cpu` | cross-code | Both errors against the CPU's in f64 (−48.6201 and −70.4749 dB, the example's output), largest difference, dB | photonoxide's CPU kernel in f64, the `cpml_roden_gedney` example | 0.00130565 | 0 | 5e-2 | pass |

## Every FDTD case on the GPU

Each FDTD case of the main report run three times: on the CPU, then with every simulation it makes on the GPU in f64, then in f32. A simulation the GPU doesn't step yet (a Bloch phase, a dispersive medium, a smoothed tensor that couples E's components, a plane wave, the plain loops) falls back to the CPU; the last column but one is the share of the steps taken on the GPU. The tolerances are the main report's, set for f64: in f32 a case is shown within or beyond its tolerance, not held to it, and a case whose checks of its own f32 can't meet (a field decaying far below f32's rounding, say) stops short. 36 cases ran on the GPU, 34 on the CPU only or took no steps (analytic cases, and one whose runs are kept from before). The last column is the time on the CPU and on the GPU in f64, including the copies to and from the GPU at each run.

| Case | CPU | GPU, f64 | GPU, f32 | Tolerance | On the GPU | Time |
|---|---|---|---|---|---|---|
| `fdtd/dispersion` | 3.58e-15 pass | 3.51e-15 pass | 3.95e-8 beyond | 1e-12 | 100 % | 0.0 s / 0.1 s |
| `fdtd/energy` | 2.35e-15 pass | 2.22e-15 pass | 2.56e-8 beyond | 1e-12 | 100 % | 0.0 s / 1.4 s |
| `fdtd/fdfd-lossy` | 2.30e-14 pass | 2.29e-14 pass | 2.84e-7 beyond | 1e-10 | 100 % | 0.4 s / 2.4 s |
| `fdtd/fdfd-cpml` | 3.47e-4 pass | 3.47e-4 pass | 3.47e-4 within | 5e-4 | 100 % | 3.4 s / 11.4 s |
| `fdtd/cpml-thickness` | 6.13e-6 pass | 6.13e-6 pass | 6.28e-6 within | 1e-4 | 100 % | 0.2 s / 0.3 s |
| `fdtd/spectrum` | 6.46e-13 pass | 6.46e-13 pass | 9.43e-8 beyond | 1e-10 | 100 % | 0.3 s / 14.5 s |
| `fdtd/tfsf-leakage` | 1.21e-15 pass | 1.21e-15 pass | 1.21e-15 within | 1e-12 | CPU only | 2.3 s / 2.2 s |
| `fdtd/tfsf-slab` | 3.31e-4 pass | 3.31e-4 pass | 3.31e-4 within | 1e-3 | CPU only | 1.3 s / 1.2 s |
| `fdtd/mode-source-fdfd` | 2.50e-14 pass | 2.51e-14 pass | 1.50e-6 beyond | 1e-10 | 100 % | 1.3 s / 2.3 s |
| `fdtd/mode-source-backward` | 1.52e-7 pass | 1.52e-7 pass | 1.52e-7 within | 1e-5 | 100 % | 3.9 s / 12.2 s |
| `fdtd/mode-source-power` | 1.00005 pass | 1.00005 pass | 1.00005 within | 5e-4 | 100 % | 3.9 s / 17.2 s |
| `fdtd/beam-waist` | 1.49999 pass | 1.49999 pass | 1.49999 within | 1e-3 | 100 % | 0.6 s / 3.2 s |
| `fdtd/beam-divergence` | 2.69e-6 pass | 2.69e-6 pass | 2.71e-6 within | 1e-4 | 100 % | 0.6 s / 2.7 s |
| `fdtd/beam-tilt` | 1.54e-4 pass | 1.54e-4 pass | 1.54e-4 within | 1e-3 | 100 % | 0.6 s / 3.5 s |
| `fdtd/ade-fdfd` | 3.18e-14 pass | 3.18e-14 pass | 3.18e-14 within | 1e-10 | CPU only | 0.9 s / 1.0 s |
| `fdtd/drude-fresnel` | 2.96e-4 pass | 2.96e-4 pass | 2.95e-4 within | 1e-3 | 50 % | 1.8 s / 2.8 s |
| `fdtd/lorentz-group-delay` | 3.62e-4 pass | 3.62e-4 pass | 3.61e-4 within | 1e-3 | 50 % | 6.3 s / 7.1 s |
| `fdtd/bloch-fdfd` | 2.90e-14 pass | 2.90e-14 pass | 2.90e-14 within | 1e-10 | CPU only | 1.9 s / 1.9 s |
| `fdtd/bloch-multilayer` | 0.00127298 pass | 0.00127298 pass | 0.00127298 within | 3e-3 | CPU only | 10.0 s / 9.7 s |
| `fdtd/bloch-bands` | 3.07e-4 pass | 3.07e-4 pass | 3.07e-4 within | 1e-3 | CPU only | 1.4 s / 1.5 s |
| `fdtd/fit-lossless` | 5.96e-5 pass | 5.96e-5 pass | 5.96e-5 within | 1e-4 | no steps | 0.0 s / 0.0 s |
| `fdtd/fit-lossy` | 0.00805944 pass | 0.00805944 pass | 0.00805944 within | 1e-2 | no steps | 0.0 s / 0.0 s |
| `fdtd/fitted-slab` | 0.00199390 pass | 0.00199390 pass | 0.00199348 within | 5e-3 | 50 % | 0.7 s / 1.4 s |
| `fdtd/smoothing-slab` | 6.92e-4 pass | 6.92e-4 pass | 6.92e-4 within | 1.5e-3 | CPU only | 1.2 s / 1.2 s |
| `fdtd/smoothing-anisotropic-slab` | 3.01e-4 pass | 3.01e-4 pass | 3.01e-4 within | 1e-3 | CPU only | 1.7 s / 1.7 s |
| `fdtd/smoothing-oblique` | 2.15e-4 pass | 2.15e-4 pass | 2.15e-4 within | 4e-4 | CPU only | 8.6 s / 8.9 s |
| `fdtd/smoothing-oblique-nodes` | 0.00199259 pass | 0.00199259 pass | 0.00199259 within | 3e-3 | CPU only | 9.0 s / 9.1 s |
| `fdtd/smoothing-energy` | 1.97e-15 pass | 1.97e-15 pass | 1.97e-15 within | 1e-12 | CPU only | 5.5 s / 5.6 s |
| `fdtd/smoothing-contrast` | 1.00000 pass | 1.00000 pass | 1.00000 within | 1e0 | CPU only | 13.2 s / 13.2 s |
| `fdtd/smoothing-oskooi` | 0.0627803 pass | 0.0627803 pass | 0.0627803 within | 1e-1 | CPU only | 13.7 s / 14.3 s |
| `fdtd/smoothing-triplets-oblique` | 0.00105559 pass | 0.00105559 pass | 0.00105559 within | 1.6e-3 | CPU only | 6.3 s / 6.2 s |
| `fdtd/smoothing-triplets-order` | 1.15424 pass | 1.15424 pass | 1.15424 within | 3.5e-1 | CPU only | 5.8 s / 6.1 s |
| `fdtd/smoothing-diagonal-oblique` | 0.00442954 pass | 0.00442954 pass | 0.00953109 beyond | 6e-3 | 100 % | 3.2 s / 0.5 s |
| `fdtd/smoothing-diagonal-order` | 0.866212 pass | 0.866212 pass | 0.319799 beyond | 3.5e-1 | 100 % | 3.3 s / 0.5 s |
| `fdtd/smoothing-bauer-order` | 1.86519 pass | 1.86519 pass | 1.86519 within | 3e-1 | CPU only | 6.3 s / 6.4 s |
| `fdtd/smoothing-triplets-contrast` | 5.19e-15 pass | 5.19e-15 pass | 5.19e-15 within | 1e-12 | CPU only | 10.9 s / 11.7 s |
| `fdtd/smoothing-triplets-lattices` | 4.98e-15 pass | 4.98e-15 pass | 4.98e-15 within | 1e-12 | CPU only | 1.2 s / 1.2 s |
| `fdtd/smoothing-wc07-growth` | 2.85005 pass | 2.85005 pass | 2.85005 within | 6e-1 | CPU only | 1.5 s / 1.5 s |
| `fdtd/smoothing-crystal` | 0.00175448 pass | 0.00175448 pass | 0.00175448 within | 3e-3 | CPU only | 11.9 s / 13.1 s |
| `fdtd/smoothing-crystal-order` | 1.98728 pass | 1.98728 pass | 1.98728 within | 3e-1 | CPU only | 13.6 s / 13.7 s |
| `fdtd/monitor-transforms` | 0 pass | 8.90e-17 pass | 6.52e-7 beyond | 1e-15 | 100 % | 0.0 s / 4.0 s |
| `fdtd/monitor-flux` | 1.18e-11 pass | 1.18e-11 pass | 2.58e-6 beyond | 1e-9 | 100 % | 0.2 s / 0.4 s |
| `fdtd/monitor-flux-box` | 1.60e-15 pass | 1.90e-15 pass | 3.56e-7 beyond | 1e-12 | 100 % | 0.3 s / 0.4 s |
| `fdtd/monitor-modes` | 1.49e-11 pass | 1.49e-11 pass | 3.68e-6 beyond | 1e-9 | 100 % | 1.9 s / 0.9 s |
| `fdtd/monitor-guides` | 5.79e-5 pass | 5.79e-5 pass | stopped: the guide's field didn't decay | 1e-4 | 100 % | 1.4 s / 1.5 s |
| `fdtd/harmonic-inversion` | 1.50e-12 pass | 1.50e-12 pass | 1.50e-12 within | 1e-9 | no steps | 0.0 s / 0.0 s |
| `fdtd/cavity-resonances` | 2.95e-15 pass | 3.56e-15 pass | 4.39e-9 beyond | 1e-12 | 100 % | 0.1 s / 0.3 s |
| `fdtd/slab-resonance` | 2.00129 pass | 2.00129 pass | stopped: the slab's field didn't decay | 5e-2 | 100 % | 2.0 s / 1.9 s |
| `fdtd/mie-table` | 0.00295745 pass | 0.00295745 pass | 0.00295745 within | 5e-3 | no steps | 0.0 s / 0.0 s |
| `fdtd/mie-balance` | 1.39e-13 pass | 1.39e-13 pass | 1.39e-13 within | 1e-11 | no steps | 0.0 s / 0.0 s |
| `fdtd/mie-terms` | 6.97e-14 pass | 6.97e-14 pass | 6.97e-14 within | 1e-12 | no steps | 0.0 s / 0.0 s |
| `fdtd/mie-sphere` | 1.96748 pass | 1.96748 pass | 1.96748 within | 1.5e-1 | CPU only | 17.5 s / 17.5 s |
| `fdtd/mie-sphere-staircase` | 0.00597458 pass | 0.00597458 pass | 0.00597458 within | 1e-2 | CPU only | 11.0 s / 11.0 s |
| `fdtd/mie-drude` | 0.0158475 pass | 0.0158475 pass | 0.0158475 within | 3e-2 | CPU only | 12.4 s / 12.4 s |
| `fdtd/meep-pml-rates` | 0.324043 pass | 0.324044 pass | stopped: the field didn't decay | 4e-1 | 100 % | 19.8 s / 6.1 s |
| `fdtd/ring-wronskian` | 1.02e-10 pass | 1.02e-10 pass | 1.02e-10 within | 1e-9 | no steps | 0.0 s / 0.0 s |
| `fdtd/ring-resonances` | 7.67e-4 pass | 7.67e-4 pass | 7.67e-4 within | 1.5e-3 | no steps | 21.0 s / 0.0 s |
| `fdtd/ring-order` | 1.94866 pass | 1.94866 pass | 1.94866 within | 3e-1 | no steps | 3.2 s / 0.0 s |
| `fdtd/fdfd-band-2d` | 3.19e-5 pass | 3.19e-5 pass | stopped: the guide's field didn't decay | 1e-4 | 100 % | 2.9 s / 3.6 s |
| `fdtd/fdfd-band-2d-fine` | 1.70e-7 pass | 1.70e-7 pass | stopped: the guide's field didn't decay | 1e-6 | 100 % | 12.4 s / 7.4 s |
| `fdtd/fdfd-band-3d-strip` | 2.19e-4 pass | 2.19e-4 pass | stopped: the guide's field didn't decay | 5e-4 | 100 % | 19.6 s / 20.3 s |
| `fdtd/fdfd-band-3d-bend` | 1.86e-4 pass | 1.86e-4 pass | stopped: the guide's field didn't decay | 5e-4 | 100 % | 28.4 s / 27.9 s |
| `fdtd/fdfd-smoothed` | 0.848365 pass | 0.848365 pass | 0.848365 within | 3e-1 | CPU only | 13.6 s / 11.9 s |
| `fdtd/adjoint-modes` | 5.52e-10 pass | 3.32e-10 pass | 0.00659341 beyond | 1e-7 | 100 % | 38.6 s / 157.5 s |
| `fdtd/adjoint-flux` | 1.47e-9 pass | 1.83e-10 pass | 0.0594717 beyond | 1e-7 | 100 % | 38.9 s / 164.2 s |
| `fdtd/adjoint-fdfd` | 7.48e-14 pass | 7.08e-14 pass | 3.42e-6 beyond | 1e-10 | 100 % | 4.8 s / 6.3 s |
| `fdtd/adjoint-fdfd-cpml` | 2.28e-5 pass | 2.28e-5 pass | 2.42e-5 within | 5e-5 | 100 % | 10.7 s / 8.2 s |
| `fdtd/adjoint-slab` | 7.16e-4 pass | 7.16e-4 pass | 7.12e-4 within | 1e-3 | 100 % | 2.1 s / 4.2 s |
| `fdtd/adjoint-slab-order` | 2.01506 pass | 2.01506 pass | 2.02163 within | 1e-1 | 100 % | 1.9 s / 2.7 s |
| `fdtd/adjoint-backward-mode` | 5.48e-13 pass | 5.48e-13 pass | 2.28e-6 beyond | 1e-10 | 100 % | 6.2 s / 8.6 s |

## Speed

The kernel alone on random fields, million cell-updates a second, the best of three runs of a few tenths of a second each; the CPU's blocked kernel (Malas et al.'s diamonds, `Blocking::auto`) in the same precision on all its threads, unless the machine was busy when this report was written (`GPU_REPORT_CPU=0`): the GPU's rate hardly depends on the CPU's load, the CPU's does. [FDTD](methods/fdtd.md#the-gpu) has both measured on an idle machine.

| Grid | Precision | GPU, two passes | GPU, fused | CPU, diamonds on 20 threads | GPU / CPU |
|---|---|---:|---:|---:|---:|
| 128³ cells of 50 nm, vacuum, CPMLs of 8 | f32 | 2424 | 3325 | 841 | 3.95 |
| 128³ cells of 50 nm, vacuum, CPMLs of 8 | f64 | 1240 | 1325 | 545 | 2.43 |
| 256³ cells of 50 nm, vacuum, CPMLs of 8 | f32 | 2867 | 3717 | 1853 | 2.01 |
| 256³ cells of 50 nm, vacuum, CPMLs of 8 | f64 | 1422 | 1657 | 768 | 2.16 |

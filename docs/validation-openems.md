# Validation report: openEMS

Written by `just openems` where openEMS is installed (tachsin's machine); don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. How openEMS is found and run, and its licence: [cross-code references](cross-code.md).

openEMS v0.37.0 (CSXCAD v0.7.0), found through $PHOTONOXIDE_OPENEMS, its multithreaded engine on 8 threads; photonoxide 0.5.4.

## A coplanar waveguide in a box: 3D FDTD against the quasi-TEM line

Ghione and Naldi's line (G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984), doi:10.1049/el:19840120): perfectly conducting strips of zero thickness, the signal 20 µm wide, gaps of 10 µm, on 20 µm of a substrate of ε = 10, air below it and above the strips, in a perfectly conducting box ±200 µm across and high, the grounds out to its sides. openEMS runs a section 4 mm long (25 µm cells along it, 8-cell PMLs at both ends that the line runs into) driven across both gaps by a Gaussian pulse of 5 to 55 GHz, until the energy is down by 70 dB, and gives k and Z₀ at each frequency from the voltage across a gap at three planes 1025 µm apart and the current on the signal midway (exact for a uniform line, whatever its ends reflect); its quasi-static n and Z₀ are the limits at 0 GHz of v(f) = v₀ + a f² fitted to its five frequencies. photonoxide's quasi-TEM line is the static solver's, on half the box beside its mirror: n = √(C/C₀), Z₀ = 1/(c√(CC₀)).

Across the line both codes take the same mesh lines: cells from the smallest at the strips' edges and in their plane, growing by 1.1 to 10 µm (openEMS's the mirror image of photonoxide's half).

### On each mesh

| Smallest cell | openEMS's cells | Steps (time step) | Time | n at 10, 20, 30, 40, 50 GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | photonoxide, the same lines: n, Z₀ | Difference: n, Z₀ |
|---:|---:|---:|---:|---|---|---|---|---|
| 2 µm | 698560 | 38112 (3.816e-15 s) | 149 s (an earlier run) | 2.21693, 2.21698, 2.21727, 2.21768, 2.21821 | 51.056, 51.062, 51.068, 51.076, 51.085 | 2.21681, 51.057 | 2.21568, 57.167 | +5.10e-4, -1.07e-1 |
| 1 µm | 1175040 | 77895 (1.750e-15 s) | 461 s (an earlier run) | 2.21723, 2.21732, 2.21763, 2.21804, 2.21858 | 52.551, 52.557, 52.562, 52.571, 52.580 | 2.21713, 52.551 | 2.21625, 55.713 | +3.93e-4, -5.68e-2 |
| 0.5 µm | 1843840 | 154380 (8.833e-16 s) | 1388 s (an earlier run) | 2.21681, 2.21690, 2.21722, 2.21762, 2.21818 | 53.341, 53.346, 53.352, 53.360, 53.370 | 2.21671, 53.341 | 2.21719, 54.922 | -2.18e-4, -2.88e-2 |
| 0.25 µm | 2848000 | 339345 (4.018e-16 s) | 2408 s | 2.21718, 2.21725, 2.21757, 2.21798, 2.21853 | 53.711, 53.717, 53.723, 53.731, 53.741 | 2.21707, 53.711 | 2.21705, 54.555 | +7.42e-6, -1.55e-2 |

### Converged

| | n | Z₀, Ω | From |
|---|---:|---:|---|
| openEMS, extrapolated | 2.21707 | 54.039 | the three finest meshes; n no order: the finest shown, Z₀ order 1.09 |
| photonoxide on the same lines, extrapolated | 2.21705 | 54.236 | the same; n no order: the finest shown, Z₀ order 1.11 |
| photonoxide, converged | 2.21716 | 54.148 | `QuasiTem` from 0.0025 µm, growing by 1.025 to 2 µm (from 0.01, 0.005, 0.0025 µm: n 2.217157, 2.217162, 2.217156, Z₀ 54.1580, 54.1508, 54.1476; extrapolated n 2.217156, Z₀ 54.1450, order 1.16) |

openEMS against photonoxide converged: n -3.95e-5, Z₀ -2.01e-3 (relative; tolerances 1e-3 and 5e-3).

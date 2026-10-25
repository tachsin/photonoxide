# Validation report: Elmer

Written by `just elmer` where Elmer is installed (tachsin's machine); don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. How Elmer is found and run, and its licence: [cross-code references](cross-code.md).

Elmer 26.2 (ElmerSolver, found through $PHOTONOXIDE_ELMER), its direct solver (UMFPACK); photonoxide 0.5.4. Both codes take the same mesh lines: Elmer's nodes are photonoxide's cell faces, Elmer's bilinear quadrilaterals against photonoxide's cell-centred finite volumes.

## A coplanar waveguide in a box: StatElecSolver's capacitance

Ghione and Naldi's line, as in [the openEMS report](validation-openems.md): perfectly conducting strips of zero thickness, the signal 20 µm wide, gaps of 10 µm, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high; half of it beside the mirror plane through the signal. Elmer's StatElecSolver gives the signal's capacitance with the substrate and without it (its Capacitance Matrix), so n = √(C/C₀) and Z₀ = 1/(c√(CC₀)), as photonoxide's quasi-TEM line has them. Cells from the smallest at the strips' edges and in their plane, growing by 1.1 to 10 µm.

| Smallest cell | Cells | Time | Elmer: n, Z₀ (Ω) | photonoxide, the same lines: n, Z₀ (Ω) |
|---:|---:|---:|---|---|
| 2 µm | 2183 | 0.3 s | 2.217332, 52.4312 | 2.215678, 57.1665 |
| 1 µm | 3672 | 0.3 s | 2.217109, 53.2792 | 2.216253, 55.7129 |
| 0.5 µm | 5762 | 0.3 s | 2.216937, 53.7105 | 2.217192, 54.9224 |
| 0.25 µm | 8900 | 0.4 s | 2.217161, 53.9122 | 2.217052, 54.5551 |
| 0.125 µm | 12995 | 0.5 s | 2.217175, 54.0150 | 2.217159, 54.3714 |

| | n | Z₀, Ω | From |
|---|---:|---:|---|
| Elmer, extrapolated | 2.217176 | 54.1221 | the three finest meshes; n order 3.97, Z₀ order 0.97 |
| photonoxide on the same lines, extrapolated | 2.217159 | 54.1874 | the same; n no order: the finest shown, Z₀ order 1.00 |
| photonoxide, converged | 2.217156 | 54.1476 | graded from 0.0025 µm by 1.025 (extrapolated n 2.217156, Z₀ 54.1450, order 1.16) |

Elmer against photonoxide converged: n +8.97e-6, Z₀ -4.71e-4 (relative; tolerances 1e-3 and 5e-3).

## A heater over silicon: HeatSolver's peak rise

A heater 2 µm wide and 0.2 µm thick, its centre 2 µm up, dissipating 1 W/m in 3 µm of oxide (k = 1.38 W/(m·K)) on 10 µm of silicon (k = 148 W/(m·K)) whose bottom is held at 0, the other sides insulated; half of it beside the mirror plane through the heater, in a window 30 µm wide. Elmer's HeatSolver with the heater's power as a heat source, its peak temperature by SaveScalars; photonoxide's heat solver's peak. Cells from the smallest at the heater's edges and the oxide's interface, growing by 1.1 to 2 µm.

| Smallest cell | Cells | Time | Elmer: peak rise (K) | photonoxide, the same lines (K) |
|---:|---:|---:|---:|---:|
| 0.2 µm | 1155 | 0.1 s | 0.324539 | 0.334348 |
| 0.1 µm | 2250 | 0.1 s | 0.326228 | 0.329440 |
| 0.05 µm | 3976 | 0.1 s | 0.327070 | 0.328050 |
| 0.025 µm | 6831 | 0.2 s | 0.327346 | 0.327623 |
| 0.0125 µm | 10578 | 0.2 s | 0.327384 | 0.327508 |

| | Peak rise, K | From |
|---|---:|---|
| Elmer, extrapolated | 0.327390 | the three finest meshes, order 2.87 |
| photonoxide on the same lines, extrapolated | 0.327466 | the same, order 1.90 |
| photonoxide, converged | 0.327422 | graded from 0.001 µm by 1.05 (extrapolated 0.327422, order 1.90) |

Elmer against photonoxide converged: -9.94e-5 (relative; tolerance 5e-3).

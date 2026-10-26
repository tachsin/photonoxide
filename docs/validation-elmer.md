# Cross-code comparison: Elmer

Written by `just elmer` on a computer with Elmer installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. How Elmer is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).

Elmer 26.2 (ElmerSolver, found through $PHOTONOXIDE_ELMER), its direct solver (UMFPACK); photonoxide 0.5.4. Run on Intel Core Ultra 7 265K (20 threads), 64 GB, Windows 11. Both codes solve the same 2D problems on the same mesh lines: Elmer's nodes are photonoxide's cell faces, Elmer's bilinear quadrilaterals against photonoxide's cell-centred finite volumes. Every error is against photonoxide's converged result, the reference, in percent. Both codes solve the same problems, so their times are compared: each code's assembly and solve, the fastest of three runs, Elmer's as its log reports them (StatElecSolver's and HeatSolve's Assembly and Solve), photonoxide's measured around its solve; neither includes starting a program or reading and writing files.

## Results

| Case | Quantity | photonoxide, converged | Elmer, extrapolated | Error | Elmer, finest mesh | Error | photonoxide, finest mesh | Error |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| the coplanar line | n | 2.217156 | 2.217176 | +0.00090 % | 2.217175 | +0.00085 % | 2.217159 | +0.00011 % |
| the coplanar line | Z₀, Ω | 54.147614 | 54.122125 | −0.047 % | 54.015033 | −0.24 % | 54.371375 | +0.41 % |
| the heater | peak rise, K | 0.327422 | 0.327390 | −0.0099 % | 0.327384 | −0.012 % | 0.327508 | +0.026 % |

| Case | Elmer, all meshes | photonoxide, the same meshes | Elmer, finest mesh | photonoxide, finest mesh | photonoxide, converged |
|---|---:|---:|---:|---:|---:|
| the coplanar line (two solves a mesh, in each code) | 0.194 s | 0.145 s | 0.075 s | 0.053 s | 4.97 s |
| the heater | 0.120 s | 0.068 s | 0.070 s | 0.029 s | 0.369 s |

## A coplanar waveguide in a box: StatElecSolver's capacitance

Ghione and Naldi's line, as in [the openEMS report](validation-openems.md): perfectly conducting strips of zero thickness, the signal 20 µm wide, gaps of 10 µm, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high; half of it beside the mirror plane through the signal. Elmer's StatElecSolver gives the signal's capacitance with the substrate and without it (its Capacitance Matrix), so n = √(C/C₀) and Z₀ = 1/(c√(CC₀)), as photonoxide's quasi-TEM line has them. Cells from the smallest at the strips' edges and in their plane, growing by 1.1 to 10 µm.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: n, Z₀ (Ω) | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ (Ω) | Error: n, Z₀ |
|---:|---:|---:|---|---|---|---|
| 2 µm | 2183 | 0.012 s, 0.011 s | 2.217332, 52.4312 | +0.0079 %, −3.17 % | 2.215678, 57.1665 | −0.067 %, +5.58 % |
| 1 µm | 3672 | 0.023 s, 0.017 s | 2.217109, 53.2792 | −0.0021 %, −1.60 % | 2.216253, 55.7129 | −0.041 %, +2.89 % |
| 0.5 µm | 5762 | 0.033 s, 0.025 s | 2.216937, 53.7105 | −0.0099 %, −0.81 % | 2.217192, 54.9224 | +0.0016 %, +1.43 % |
| 0.25 µm | 8900 | 0.051 s, 0.038 s | 2.217161, 53.9122 | +0.00021 %, −0.43 % | 2.217052, 54.5551 | −0.0047 %, +0.75 % |
| 0.125 µm | 12995 | 0.075 s, 0.053 s | 2.217175, 54.0150 | +0.00085 %, −0.24 % | 2.217159, 54.3714 | +0.00011 %, +0.41 % |

| | n | Error | Z₀, Ω | Error | From |
|---|---:|---:|---:|---:|---|
| photonoxide, converged (the reference) | 2.217156 | — | 54.1476 | — | graded from 0.0025 µm by 1.025 (extrapolated n 2.217156, Z₀ 54.1450, order 1.16) |
| Elmer, extrapolated | 2.217176 | +0.00090 % | 54.1221 | −0.047 % | the three finest meshes; n order 3.97, Z₀ order 0.97 |
| photonoxide on the same lines, extrapolated | 2.217159 | +0.00011 % | 54.1874 | +0.073 % | the same; n no order: the finest shown, Z₀ order 1.00 |

## A heater over silicon: HeatSolver's peak rise

A heater 2 µm wide and 0.2 µm thick, its centre 2 µm up, dissipating 1 W/m in 3 µm of oxide (k = 1.38 W/(m·K)) on 10 µm of silicon (k = 148 W/(m·K)) whose bottom is held at 0, the other sides insulated; half of it beside the mirror plane through the heater, in a window 30 µm wide. Elmer's HeatSolver with the heater's power as a heat source, its peak temperature by SaveScalars; photonoxide's heat solver's peak. Cells from the smallest at the heater's edges and the oxide's interface, growing by 1.1 to 2 µm.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: peak rise (K) | Error | photonoxide, the same lines (K) | Error |
|---:|---:|---:|---:|---:|---:|---:|
| 0.2 µm | 1155 | 0.000 s, 0.002 s | 0.324539 | −0.88 % | 0.334348 | +2.12 % |
| 0.1 µm | 2250 | 0.010 s, 0.007 s | 0.326228 | −0.36 % | 0.329440 | +0.62 % |
| 0.05 µm | 3976 | 0.010 s, 0.010 s | 0.327070 | −0.11 % | 0.328050 | +0.19 % |
| 0.025 µm | 6831 | 0.030 s, 0.020 s | 0.327346 | −0.023 % | 0.327623 | +0.061 % |
| 0.0125 µm | 10578 | 0.070 s, 0.029 s | 0.327384 | −0.012 % | 0.327508 | +0.026 % |

| | Peak rise, K | Error | From |
|---|---:|---:|---|
| photonoxide, converged (the reference) | 0.327422 | — | graded from 0.001 µm by 1.05 (extrapolated 0.327422, order 1.90) |
| Elmer, extrapolated | 0.327390 | −0.0099 % | the three finest meshes, order 2.87 |
| photonoxide on the same lines, extrapolated | 0.327466 | +0.013 % | the same, order 1.90 |

Elmer extrapolated against photonoxide converged: the line's n +0.00090 %, Z₀ −0.047 %, the heater's rise −0.0099 % (tolerances 0.1 %, 0.5 % and 0.5 %).

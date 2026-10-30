# Cross-code comparison: Elmer

Written by `just elmer` on a computer with Elmer installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. How Elmer is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).

Elmer 26.2 (ElmerSolver, found through $PHOTONOXIDE_ELMER), its direct solver (UMFPACK); photonoxide 0.6.0. Run on Intel Core Ultra 7 265K (20 threads), 64 GB, Windows 11. Both codes solve the same 2D problems on the same mesh lines: Elmer's nodes are photonoxide's cell faces, Elmer's bilinear quadrilaterals against photonoxide's cell-centred finite volumes. Every error is against photonoxide's converged result, the reference, in percent. Both codes solve the same problems, so their times are compared: each code's assembly and solve, the fastest of three runs, Elmer's as its log reports them (the Assembly and Solve times of StatElecSolver, StatCurrentSolver and HeatSolve, each step's in a transient), photonoxide's measured around its solve; neither includes starting a program or reading and writing files. **These times were taken under load**, other builds running on the computer at the same time: the agreement stands, but the times are only rough until they are taken again with the computer idle.

## Results

| Case | Quantity | photonoxide, converged | Elmer, extrapolated | Error | Elmer, finest mesh | Error | photonoxide, finest mesh | Error |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| the coplanar line | n | 2.217156 | 2.217176 | +0.00090 % | 2.217175 | +0.00085 % | 2.217159 | +0.00011 % |
| the coplanar line | Z₀, Ω | 54.147614 | 54.122125 | −0.047 % | 54.015033 | −0.24 % | 54.371375 | +0.41 % |
| the heater | peak rise, K | 0.327422 | 0.327390 | −0.0099 % | 0.327384 | −0.012 % | 0.327508 | +0.026 % |
| the two heaters | neighbour's rise, K | 0.97981 | 0.97968 | −0.013 % | 0.97970 | −0.011 % | 0.97984 | +0.0033 % |
| the two heaters | heated guide's rise, K | 11.9062 | 11.9043 | −0.017 % | 11.9040 | −0.019 % | 11.9092 | +0.025 % |
| the heater in time | rise at 1 µs, K | 3.8352 | 3.8340 | −0.034 % | 3.8337 | −0.039 % | 3.8374 | +0.055 % |
| the heater in time | rise at 5 µs, K | 7.9369 | 7.9346 | −0.029 % | 7.9343 | −0.033 % | 7.9403 | +0.042 % |
| the heater in time | rise at 20 µs, K | 11.0297 | 11.0275 | −0.020 % | 11.0272 | −0.022 % | 11.0330 | +0.030 % |
| the heater in time | 1/e time, µs | 4.2594 | 4.2605 | +0.027 % | 4.2607 | +0.030 % | 4.2577 | −0.040 % |
| the doped heater | current at 1 V, A/m | 99.92240 | 99.92795 | +0.0056 % | 99.95854 | +0.036 % | 99.86603 | −0.056 % |
| the doped heater | peak rise, K | 18.5775 | 18.5755 | −0.011 % | 18.5805 | +0.016 % | 18.5721 | −0.029 % |
| the modulator's electrodes | capacitance, pF/m | 160.3602 | 160.4250 | +0.040 % | 160.4329 | +0.045 % | 160.2756 | −0.053 % |
| the modulator's electrodes | mean E_x at 1 V, V/µm | -0.124437 | -0.124456 | +0.016 % | -0.124474 | +0.030 % | -0.124386 | −0.041 % |

| Case | Elmer, all meshes | photonoxide, the same meshes | Elmer, finest mesh | photonoxide, finest mesh | photonoxide, converged |
|---|---:|---:|---:|---:|---:|
| the coplanar line (two solves a mesh, in each code) | 0.189 s | 0.262 s | 0.073 s | 0.112 s | 5.86 s |
| the heater | 0.110 s | 0.124 s | 0.060 s | 0.051 s | 0.472 s |
| the two heaters | 1.52 s | 0.788 s | 0.770 s | 0.332 s | 2.46 s |
| the heater in time | 6 min 30 s | 8.08 s | 2 min 57 s | 3.97 s | 43 s |
| the doped heater | 0.884 s | 0.564 s | 0.463 s | 0.258 s | 2.09 s |
| the modulator's electrodes | 0.740 s | 0.487 s | 0.369 s | 0.195 s | 1.37 s |

## A coplanar waveguide in a box: StatElecSolver's capacitance

Ghione and Naldi's line, as in [the openEMS report](validation-openems.md): perfectly conducting strips of zero thickness, the signal 20 µm wide, gaps of 10 µm, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high; half of it beside the mirror plane through the signal. Elmer's StatElecSolver gives the signal's capacitance with the substrate and without it (its Capacitance Matrix), so n = √(C/C₀) and Z₀ = 1/(c√(CC₀)), as photonoxide's quasi-TEM line has them. Cells from the smallest at the strips' edges and in their plane, growing by 1.1 to 10 µm.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: n, Z₀ (Ω) | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ (Ω) | Error: n, Z₀ |
|---:|---:|---:|---|---|---|---|
| 2 µm | 2183 | 0.013 s, 0.011 s | 2.217332, 52.4312 | +0.0079 %, −3.17 % | 2.215678, 57.1665 | −0.067 %, +5.58 % |
| 1 µm | 3672 | 0.021 s, 0.017 s | 2.217109, 53.2792 | −0.0021 %, −1.60 % | 2.216253, 55.7129 | −0.041 %, +2.89 % |
| 0.5 µm | 5762 | 0.032 s, 0.038 s | 2.216937, 53.7105 | −0.0099 %, −0.81 % | 2.217192, 54.9224 | +0.0016 %, +1.43 % |
| 0.25 µm | 8900 | 0.050 s, 0.083 s | 2.217161, 53.9122 | +0.00021 %, −0.43 % | 2.217052, 54.5551 | −0.0047 %, +0.75 % |
| 0.125 µm | 12995 | 0.073 s, 0.112 s | 2.217175, 54.0150 | +0.00085 %, −0.24 % | 2.217159, 54.3714 | +0.00011 %, +0.41 % |

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
| 0.1 µm | 2250 | 0.010 s, 0.021 s | 0.326228 | −0.36 % | 0.329440 | +0.62 % |
| 0.05 µm | 3976 | 0.010 s, 0.033 s | 0.327070 | −0.11 % | 0.328050 | +0.19 % |
| 0.025 µm | 6831 | 0.030 s, 0.018 s | 0.327346 | −0.023 % | 0.327623 | +0.061 % |
| 0.0125 µm | 10578 | 0.060 s, 0.051 s | 0.327384 | −0.012 % | 0.327508 | +0.026 % |

| | Peak rise, K | Error | From |
|---|---:|---:|---|
| photonoxide, converged (the reference) | 0.327422 | — | graded from 0.001 µm by 1.05 (extrapolated 0.327422, order 1.90) |
| Elmer, extrapolated | 0.327390 | −0.0099 % | the three finest meshes, order 2.87 |
| photonoxide on the same lines, extrapolated | 0.327466 | +0.013 % | the same, order 1.90 |

## Two heaters side by side: HeatSolver's rise at the neighbour's guide

Jacques et al.'s N++ silicon heater (Opt. Express 27, 10456 (2019), Section 2.2 and Table 3) beside a second one, the thermal crosstalk of their Fig. 9: each a rib 500 nm wide and 220 nm high on a 90 nm slab 2.1 µm wide, a strip of N++ silicon 1 µm wide beyond each side of the slab; their centres 9.1 µm apart, so that 5 µm separate their facing N++ strips. 2 µm of buried oxide on silicon, the oxide's top 3.32 µm up with 10 W/(m²·K) to air at 0, a window 18 µm high whose bottom, in the silicon, is held at 0, its sides insulated 20 µm beyond each guide. The first heater dissipates the paper's P_π, 22.8 mW over 320 µm (71.25 W/m), in its N++ strips; the second is off. Table 3's conductivities: oxide 1.38, substrate 148, rib 90, slab 55, N++ 25 W/(m·K). Elmer's HeatSolver with the power as a heat source and a heat transfer coefficient at the top, the guides' mean temperatures from its nodes (its bilinear elements integrated over each rib); photonoxide's heat solver, each rib's mean over its cells. The heated guide's own rise is shown beside its neighbour's.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: neighbour's rise, K; heated guide's rise, K | Error | photonoxide, the same lines | Error |
|---:|---:|---:|---|---|---|---|
| 0.08 µm | 10260 | 0.040 s, 0.057 s | 0.98011, 11.8953 | +0.031 %, −0.092 % | 0.97864, 11.9292 | −0.12 %, +0.19 % |
| 0.04 µm | 19182 | 0.090 s, 0.086 s | 0.97987, 11.8999 | +0.0061 %, −0.053 % | 0.97938, 11.9184 | −0.044 %, +0.10 % |
| 0.02 µm | 34408 | 0.190 s, 0.104 s | 0.97975, 11.9026 | −0.0058 %, −0.031 % | 0.97968, 11.9126 | −0.014 %, +0.053 % |
| 0.01 µm | 57420 | 0.430 s, 0.209 s | 0.97971, 11.9037 | −0.0098 %, −0.022 % | 0.97979, 11.9102 | −0.0016 %, +0.033 % |
| 0.005 µm | 92612 | 0.770 s, 0.332 s | 0.97970, 11.9040 | −0.011 %, −0.019 % | 0.97984, 11.9092 | +0.0033 %, +0.025 % |

| Quantity | photonoxide, converged (the reference) | Elmer, extrapolated | Error | photonoxide on the same lines, extrapolated | Error |
|---|---:|---:|---:|---:|---:|
| neighbour's rise, K | 0.97981 (extrapolated 0.97981, order 1.24) | 0.97968 (order 1.24) | −0.013 % | 0.97987 (order 1.28) | +0.0067 % |
| heated guide's rise, K | 11.9062 (extrapolated 11.9062, order 1.26) | 11.9043 (order 1.46) | −0.017 % | 11.9086 (order 1.32) | +0.020 % |

Cells from the smallest at the silicon's and the electrodes' edges, growing by 1.1 to 2 µm; photonoxide's converged values graded from 0.001 µm by 1.05 to 1 µm. Elmer's extrapolation and photonoxide's on the same lines take the three finest meshes.

## A heater turned on: HeatSolve in time

Jacques et al.'s N++ silicon heater of the two-heater problem, alone in the paper's 40 × 18 µm window, its sides insulated; half of it beside the mirror plane through the rib. Cold (0) until t = 0, then dissipating the paper's P_π, 71.25 W/m. Table 3's heat capacities per volume: silicon 2330 × 711 and oxide 2203 × 709 J/(m³·K). Elmer's HeatSolve in time, BDF of order 2 (its first step by backward Euler); photonoxide's transient, BDF2 (its first step by backward Euler), both with the same 0.025 µs steps to 20 µs. The rib's mean rise at 1, 5 and 20 µs, and its 1/e time, when it first reaches 1 − 1/e of its steady rise (each code's own steady rise on the same mesh, read between steps linearly). The paper's simulated limiting 1/e time, of the phase, is about 5.8 µs (Fig. 1(c)). The times are each code's whole run of steps. Elmer's HeatSolve, as set up here with its defaults, assembles and factorizes its matrix at every step (keeping the first step's factors isn't right here, as BDF2's matrix changes after its first step); photonoxide factorizes its two matrices once and then only solves, which is most of the difference in time.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: rise at 1 µs, K; rise at 5 µs, K; rise at 20 µs, K; 1/e time, µs | Error | photonoxide, the same lines | Error |
|---:|---:|---:|---|---|---|---|
| 0.08 µm | 3078 | 12 s, 0.284 s | 3.8271, 7.9255, 11.0183, 4.2657 | −0.21 %, −0.14 %, −0.10 %, +0.15 % | 3.8527, 7.9602, 11.0530, 4.2459 | +0.45 %, +0.29 %, +0.21 %, −0.32 % |
| 0.04 µm | 5520 | 33 s, 0.519 s | 3.8308, 7.9302, 11.0231, 4.2629 | −0.12 %, −0.085 %, −0.060 %, +0.084 % | 3.8441, 7.9493, 11.0421, 4.2525 | +0.23 %, +0.16 %, +0.11 %, −0.16 % |
| 0.02 µm | 9504 | 58 s, 1.09 s | 3.8327, 7.9329, 11.0258, 4.2615 | −0.067 %, −0.050 %, −0.035 %, +0.049 % | 3.8399, 7.9435, 11.0363, 4.2558 | +0.12 %, +0.083 %, +0.060 %, −0.084 % |
| 0.01 µm | 15510 | 1 min 51 s, 2.22 s | 3.8334, 7.9339, 11.0268, 4.2609 | −0.047 %, −0.038 %, −0.026 %, +0.036 % | 3.8381, 7.9412, 11.0340, 4.2571 | +0.075 %, +0.054 %, +0.039 %, −0.053 % |
| 0.005 µm | 24523 | 2 min 57 s, 3.97 s | 3.8337, 7.9343, 11.0272, 4.2607 | −0.039 %, −0.033 %, −0.022 %, +0.030 % | 3.8374, 7.9403, 11.0330, 4.2577 | +0.055 %, +0.042 %, +0.030 %, −0.040 % |

| Quantity | photonoxide, converged (the reference) | Elmer, extrapolated | Error | photonoxide on the same lines, extrapolated | Error |
|---|---:|---:|---:|---:|---:|
| rise at 1 µs, K | 3.8352 (extrapolated 3.8352, order 1.19) | 3.8340 (order 1.27) | −0.034 % | 3.8368 (order 1.21) | +0.040 % |
| rise at 5 µs, K | 7.9369 (extrapolated 7.9368, order 1.24) | 7.9346 (order 1.33) | −0.029 % | 7.9396 (order 1.23) | +0.033 % |
| rise at 20 µs, K | 11.0297 (extrapolated 11.0296, order 1.22) | 11.0275 (order 1.30) | −0.020 % | 11.0322 (order 1.20) | +0.023 % |
| 1/e time, µs | 4.2594 (extrapolated 4.2594, order 1.24) | 4.2605 (order 1.37) | +0.027 % | 4.2580 (order 1.30) | −0.031 % |

Cells from the smallest at the silicon's and the electrodes' edges, growing by 1.1 to 2 µm; photonoxide's converged values graded from 0.001 µm by 1.05 to 1 µm. Elmer's extrapolation and photonoxide's on the same lines take the three finest meshes.

The time step, on the 0.02 µm mesh: each code's figures with steps from 0.1 to 0.0125 µs, and how far each moves from the step before, in percent. The meshes above take 0.025 µs, 800 steps.

| Step | Elmer: rise at 1, 5, 20 µs (K); 1/e time (µs) | Change | photonoxide: the same | Change |
|---:|---|---|---|---|
| 0.1 µs | 3.8279, 7.9329, 11.0258, 4.2618 | — | 3.8351, 7.9435, 11.0363, 4.2561 | — |
| 0.05 µs | 3.8317, 7.9329, 11.0258, 4.2615 | +0.099 %, +0.00011 %, +0.000013 %, −0.0066 % | 3.8389, 7.9435, 11.0363, 4.2558 | +0.099 %, +0.00011 %, +0.000013 %, −0.0075 % |
| 0.025 µs | 3.8327, 7.9329, 11.0258, 4.2615 | +0.025 %, +0.000049 %, +0.0000034 %, −0.0011 % | 3.8399, 7.9435, 11.0363, 4.2558 | +0.025 %, +0.000049 %, +0.0000035 %, −0.00072 % |
| 0.0125 µs | 3.8329, 7.9329, 11.0258, 4.2614 | +0.0063 %, +0.000015 %, +0.00000088 %, −0.00051 % | 3.8401, 7.9435, 11.0363, 4.2558 | +0.0063 %, +0.000015 %, +0.00000088 %, −0.00030 % |

## Joule heating: StatCurrentSolver and HeatSolve

Harris et al.'s doped silicon heater (Opt. Express 22, 10487 (2014), Section 2 and Fig. 1), the section through one of its channels, where the doped slab runs to the contacts: a ridge 500 nm wide and 220 nm high on a 90 nm slab, p-doped 1 µm wide across the guide and p++ beyond to ±3 µm, contacts on the slab's last 100 nm each side at 1 V and 0 V. The conductivities from the paper's sheet resistances, σ = 1/(R_s t): 8.17 × 10⁴ S/m (p++, 136 Ω), 817 S/m (p, 13.6 kΩ) and 1175 S/m (the ridge, 3.87 kΩ). As the paper's model has it, 2 µm of buried oxide with its bottom held at 0 and 10 µm of oxide over the silicon with its top held at 0; the window ±20 µm wide, its sides insulated. The thermal conductivities, which the paper doesn't give, are Jacques et al.'s Table 3's: the ridge 90, the slab 55, the oxide 1.38 W/(m·K). Elmer's StatCurrentSolver in the silicon, its Joule heating the source of HeatSolve, the current the sum of the first contact's nodal reactions (its Potential Loads); photonoxide's current continuity, its Joule heat the source of its heat solver, the current its contact's. The current per metre of guide and the peak rise.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: current at 1 V, A/m; peak rise, K | Error | photonoxide, the same lines | Error |
|---:|---:|---:|---|---|---|---|
| 0.08 µm | 5704 | 0.031 s, 0.025 s | 100.94521, 18.7421 | +1.02 %, +0.89 % | 97.83822, 18.2346 | −2.09 %, −1.85 % |
| 0.04 µm | 10309 | 0.041 s, 0.042 s | 100.34152, 18.6426 | +0.42 %, +0.35 % | 99.10294, 18.4474 | −0.82 %, −0.70 % |
| 0.02 µm | 18091 | 0.115 s, 0.082 s | 100.09250, 18.6022 | +0.17 %, +0.13 % | 99.59342, 18.5281 | −0.33 %, −0.27 % |
| 0.01 µm | 30300 | 0.234 s, 0.157 s | 99.99890, 18.5871 | +0.077 %, +0.052 % | 99.77846, 18.5579 | −0.14 %, −0.11 % |
| 0.005 µm | 48896 | 0.463 s, 0.258 s | 99.95854, 18.5805 | +0.036 %, +0.016 % | 99.86603, 18.5721 | −0.056 %, −0.029 % |

| Quantity | photonoxide, converged (the reference) | Elmer, extrapolated | Error | photonoxide on the same lines, extrapolated | Error |
|---|---:|---:|---:|---:|---:|
| current at 1 V, A/m | 99.92240 (extrapolated 99.93194, order 1.04) | 99.92795 (order 1.21) | +0.0056 % | 99.94471 (order 1.08) | +0.022 % |
| peak rise, K | 18.5775 (extrapolated 18.5791, order 1.03) | 18.5755 (order 1.20) | −0.011 % | 18.5850 (order 1.07) | +0.041 % |

Cells from the smallest at the silicon's and the electrodes' edges, growing by 1.1 to 2 µm; photonoxide's converged values graded from 0.001 µm by 1.05 to 1 µm. Elmer's extrapolation and photonoxide's on the same lines take the three finest meshes.

## A thin-film lithium niobate modulator: StatElecSolver with a tensor permittivity

Zhang et al.'s cross-section (Photon. Res. 10, 2380 (2022), Section 2): an x-cut film 600 nm thick, etched 300 nm to a rib 1.2 µm wide (its sidewalls upright here, so that the mesh lines follow them), on 2 µm of thermal oxide (ε 3.9) on 20 µm of silicon (ε 11.7); electrodes 0.83 µm thick on the slab, a signal 20 µm wide 5 µm from a ground 15 µm wide (not given); 0.6 µm of PECVD oxide (ε 5.5, the paper's) and 20 µm of air above. The film's clamped static permittivity, diagonal in the section's frame: 26.2 across (along the crystal's c axis) and 45.5 up. Half of the push–pull line beside the mirror plane through the signal, every side of the window without surface charge. Elmer's StatElecSolver with the film's permittivity a tensor and the signal at 1 V, its capacitance from the field's energy (2W/V², which its capacitance matrix matches to ten digits) and the potential at the guide's sides; photonoxide's electrostatics with the diagonal tensor, the signal's charge at 1 V and its field E_x (from the displacement through each face) averaged over the guide's cells. The whole line's capacitance (twice the half's), and the mean E_x over the guide, the rib and the slab under it, at 1 V on the signal: what the Pockels effect sees, so what sets V_π·L. It is negative, the field pointing from the signal to the ground.

| Smallest cell | Cells | Time: Elmer, photonoxide | Elmer: capacitance, pF/m; mean E_x at 1 V, V/µm | Error | photonoxide, the same lines | Error |
|---:|---:|---:|---|---|---|---|
| 0.08 µm | 8798 | 0.024 s, 0.038 s | 160.6495, -0.125009 | +0.18 %, +0.46 % | 159.8367, -0.123285 | −0.33 %, −0.93 % |
| 0.04 µm | 15184 | 0.044 s, 0.053 s | 160.5345, -0.124709 | +0.11 %, +0.22 % | 160.0695, -0.123902 | −0.18 %, −0.43 % |
| 0.02 µm | 25344 | 0.094 s, 0.086 s | 160.4747, -0.124568 | +0.071 %, +0.11 % | 160.1912, -0.124193 | −0.11 %, −0.20 % |
| 0.01 µm | 39690 | 0.209 s, 0.115 s | 160.4448, -0.124501 | +0.053 %, +0.051 % | 160.2513, -0.124331 | −0.068 %, −0.085 % |
| 0.005 µm | 59700 | 0.369 s, 0.195 s | 160.4329, -0.124474 | +0.045 %, +0.030 % | 160.2756, -0.124386 | −0.053 %, −0.041 % |

| Quantity | photonoxide, converged (the reference) | Elmer, extrapolated | Error | photonoxide on the same lines, extrapolated | Error |
|---|---:|---:|---:|---:|---:|
| capacitance, pF/m | 160.3602 (extrapolated 160.3678, order 0.70) | 160.4250 (order 1.32) | +0.040 % | 160.2921 (order 1.31) | −0.043 % |
| mean E_x at 1 V, V/µm | -0.124437 (extrapolated -0.124451, order 0.74) | -0.124456 (order 1.33) | +0.016 % | -0.124423 (order 1.32) | −0.011 % |

Cells from the smallest at the silicon's and the electrodes' edges, growing by 1.1 to 2 µm; photonoxide's converged values graded from 0.001 µm by 1.05 to 1 µm. Elmer's extrapolation and photonoxide's on the same lines take the three finest meshes.

Elmer extrapolated against photonoxide converged: the line: n +0.00090 % (tolerance 0.1 %); the line: Z₀ −0.047 % (tolerance 0.5 %); the heater: peak rise −0.0099 % (tolerance 0.5 %); the two heaters: neighbour's rise, K −0.013 % (tolerance 0.5 %); the two heaters: heated guide's rise, K −0.017 % (tolerance 0.5 %); the heater in time: rise at 1 µs, K −0.034 % (tolerance 0.5 %); the heater in time: rise at 5 µs, K −0.029 % (tolerance 0.5 %); the heater in time: rise at 20 µs, K −0.020 % (tolerance 0.5 %); the heater in time: 1/e time, µs +0.027 % (tolerance 0.5 %); the doped heater: current at 1 V, A/m +0.0056 % (tolerance 0.1 %); the doped heater: peak rise, K −0.011 % (tolerance 0.5 %); the modulator's electrodes: capacitance, pF/m +0.040 % (tolerance 0.1 %); the modulator's electrodes: mean E_x at 1 V, V/µm +0.016 % (tolerance 0.5 %).

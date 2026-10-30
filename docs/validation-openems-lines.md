# Cross-code comparison: openEMS on more lines

Written by `cargo test --release --test openems_lines -- --ignored` (crosscode/README.md) on a computer with openEMS installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. The coplanar line, the first of these pairs, is in [its own report](validation-openems.md); how openEMS is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).

openEMS v0.37.0 (CSXCAD v0.7.0), found through $PHOTONOXIDE_OPENEMS, its multithreaded engine on 4 threads (fewer than the coplanar line's 8, other work sharing the computer); photonoxide 0.6.0. Every error is against photonoxide's converged result, the reference, in percent. These pairs compare accuracy only: openEMS solves Maxwell's equations in time on a 3D section of each line, photonoxide the 2D cross-section's static fields once, so their times measure different problems; openEMS's run times are shown for planning only.

## A microstrip line in a box

A perfectly conducting strip of zero thickness, 20 µm wide, on 20 µm of a substrate of ε = 10 (w/h = 1) over a ground plane, in a perfectly conducting box ±200 µm across and 200 µm high above the strip, its walls at the ground's potential. openEMS runs a section 4 mm long as the coplanar line's (25 µm cells along it, 8-cell PMLs at both ends), driven by a soft source of E between the strip and the ground (a Gaussian pulse of 5 to 55 GHz) until the energy is down by 70 dB; k and Z₀ at each frequency come from the voltage from the strip down to the ground at three planes 1025 µm apart and the current around the strip midway (a loop ±20 µm across and ±5 µm about the strip, clear of the ground), its quasi-static n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². photonoxide's quasi-TEM line is its static solver's on half the box: n = √(C/C₀), Z₀ = 1/(c√(CC₀)). Across the line both take the same mesh lines, cells from the smallest at the strip's edge and in its plane growing by 1.1 to 10 µm; three meshes, the fewest an extrapolation takes.

The closed form, Hammerstad and Jensen's (1980), will stand beside both codes: pending the paper.

| | n | Error | Z₀, Ω | Error |
|---|---:|---:|---:|---:|
| photonoxide, converged (the reference) | 2.580191 | — | 48.6603 | — |
| openEMS, extrapolated from its three meshes | 2.577318 | −0.11 % | 48.7212 | +0.13 % |
| photonoxide on the same lines, extrapolated | 2.579828 | −0.014 % | 48.5756 | −0.17 % |
| openEMS on its finest mesh (0.5 µm) | 2.581084 | +0.035 % | 48.2319 | −0.88 % |
| photonoxide on the same mesh | 2.579828 | −0.014 % | 49.0329 | +0.77 % |
| Hammerstad and Jensen (1980), closed form | pending the paper | | pending the paper | |

| Smallest cell | openEMS's cells | Steps (its run) | n at 10, 20, 30, 40, 50 GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |
|---:|---:|---:|---|---|---|---|---|---|
| 2 µm | 390720 | 40494 (2 min) | 2.58484, 2.58552, 2.58655, 2.58795, 2.58965 | 47.076, 47.091, 47.117, 47.153, 47.197 | 2.58470, 47.071 | +0.17 %, −3.27 % | 2.57750, 50.106 | −0.10 %, +2.97 % |
| 1 µm | 660480 | 75504 (4 min) | 2.58274, 2.58338, 2.58440, 2.58578, 2.58744 | 47.827, 47.843, 47.869, 47.905, 47.949 | 2.58259, 47.823 | +0.093 %, −1.72 % | 2.57863, 49.412 | −0.060 %, +1.54 % |
| 0.5 µm | 1017600 | 159024 (12 min) | 2.58124, 2.58186, 2.58287, 2.58424, 2.58590 | 48.236, 48.252, 48.278, 48.314, 48.358 | 2.58108, 48.232 | +0.035 %, −0.88 % | 2.57983, 49.033 | −0.014 %, +0.77 % |

| | n | Z₀, Ω | From |
|---|---:|---:|---|
| openEMS, extrapolated | 2.57732 | 48.721 | its three meshes; n order 0.49, Z₀ order 0.88 |
| photonoxide on the same lines, extrapolated | 2.57983 | 48.576 | the same; n no order: the finest shown, Z₀ order 0.87 |
| photonoxide, converged | 2.58019 | 48.660 | graded from 0.0025 µm by 1.025 to 2 µm (from 0.01, 0.005, 0.0025 µm: n 2.580185, 2.580193, 2.580191, Z₀ 48.6651, 48.6618, 48.6603; extrapolated n 2.580191, Z₀ 48.6591, order 1.15) |

An extrapolation of order below 0.5 says little: its three meshes' differences barely shrink, and the finest mesh is the better guide. Here: openEMS's n (order 0.49).

## Zhang et al.'s thin-film lithium niobate line, its full stack

Zhang et al.'s line (Photon. Res. 10, 2380 (2022), Section 2) with everything around it: a ground–signal–ground line, the signal 20 µm wide, gaps of 5 µm, grounds 100 µm wide (not given), the electrodes 0.83 µm thick (800 nm of copper under 30 nm of gold, perfect conductors here), on an x-cut film 0.6 µm thick etched 0.3 µm to a rib 1.2 µm wide in each gap (its sidewalls upright, so that the mesh lines follow them), on 2 µm of thermal oxide (ε 3.9) over 500 µm of silicon (ε 11.7), under 0.6 µm of PECVD oxide (ε 5.5, the paper's) and air, in a perfectly conducting box as wide as the line (±115 µm), the grounds running to its sides so that they are its potential and no static charge is left on them, its floor under the silicon and its lid 200 µm above the film. The film's clamped static permittivity is anisotropic: 26.2 across (the crystal's c axis), 45.5 up and along the line, photonoxide's a diagonal tensor and openEMS's a diagonal material. openEMS runs a section 4 mm long as the coplanar line's, driven across both gaps, the voltage across one gap at the ribs' top and the current around the signal (a loop through the gaps' middles, from 1 µm down in the buried oxide to 3 µm up in the air); its n and Z₀ at 0 GHz as above. photonoxide's quasi-TEM line is its static solver's on half the box. Both take the same mesh lines, cells from the smallest at the electrodes' and the ribs' edges and at every layer, growing by 1.1 to 25 µm (the largest cells 25 µm, the line's own cell along it, a sixty-fourth of the shortest wavelength in the silicon, to keep openEMS's three runs to a few hours); the thin layers set openEMS's time step on every mesh.

| | n | Error | Z₀, Ω | Error |
|---|---:|---:|---:|---:|
| photonoxide, converged (the reference) | 2.217902 | — | 40.3116 | — |
| openEMS, extrapolated from its three meshes | 2.224271 | +0.29 % | 40.1721 | −0.35 % |
| photonoxide on the same lines, extrapolated | 2.376082 | +7.13 % | 40.3299 | +0.045 % |
| openEMS on its finest mesh (0.5 µm) | 2.219010 | +0.050 % | 39.7716 | −1.34 % |
| photonoxide on the same mesh | 2.210763 | −0.32 % | 40.9523 | +1.59 % |

| Smallest cell | openEMS's cells | Steps (its run) | n at 10, 20, 30, 40, 50 GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |
|---:|---:|---:|---|---|---|---|---|---|
| 2 µm | 761600 | 143650 (11 min; its energy settled at -13.4 dB, the voltage 109 dB down) | 2.19886, 2.19899, 2.19923, 2.19956, 2.19999 | 37.765, 37.762, 37.759, 37.754, 37.748 | 2.19880, 37.765 | −0.86 %, −6.32 % | 2.20208, 43.527 | −0.71 %, +7.98 % |
| 1 µm | 1347840 | 185385 (17 min; its energy settled at -13.5 dB, the voltage 115 dB down) | 2.21277, 2.21287, 2.21311, 2.21345, 2.21390 | 39.189, 39.187, 39.184, 39.179, 39.172 | 2.21270, 39.190 | −0.23 %, −2.78 % | 2.20648, 41.741 | −0.52 %, +3.54 % |
| 0.5 µm | 2273280 | 298265 (39 min; its energy settled at -13.4 dB, the voltage 114 dB down) | 2.21909, 2.21918, 2.21943, 2.21977, 2.22023 | 39.770, 39.769, 39.765, 39.760, 39.753 | 2.21901, 39.772 | +0.050 %, −1.34 % | 2.21076, 40.952 | −0.32 %, +1.59 % |

| | n | Z₀, Ω | From |
|---|---:|---:|---|
| openEMS, extrapolated | 2.22427 | 40.172 | its three meshes; n order 1.14, Z₀ order 1.29 |
| photonoxide on the same lines, extrapolated | 2.37608 | 40.330 | the same; n order 0.04, Z₀ order 1.18 |
| photonoxide, converged | 2.21790 | 40.312 | graded from 0.0025 µm by 1.025 to 2 µm (from 0.01, 0.005, 0.0025 µm: n 2.217770, 2.217858, 2.217902, Z₀ 40.3165, 40.3132, 40.3116; extrapolated n 2.217948, Z₀ 40.3102, order 1.09) |

An extrapolation of order below 0.5 says little: its three meshes' differences barely shrink, and the finest mesh is the better guide. Here: photonoxide's n on the same lines (order 0.04).

openEMS extrapolated against photonoxide converged: the microstrip: n −0.11 % (tolerance 0.5 %); the microstrip: Z₀ +0.13 % (tolerance 0.5 %); Zhang et al.'s line: n +0.29 % (tolerance 0.5 %); Zhang et al.'s line: Z₀ −0.35 % (tolerance 0.5 %).

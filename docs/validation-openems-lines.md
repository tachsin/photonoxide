# Cross-code comparison: openEMS on more lines

Written by `cargo test --release --test openems_lines -- --ignored` (crosscode/README.md) on a computer with openEMS installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. The coplanar line, the first of these pairs, is in [its own report](validation-openems.md); how openEMS is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).

openEMS v0.37.0 (CSXCAD v0.7.0), found through $PHOTONOXIDE_OPENEMS, its multithreaded engine on 4 threads (fewer than the coplanar line's 8, other work sharing the computer); photonoxide 0.6.0. Every error is against photonoxide's converged result, the reference, in percent. These pairs compare accuracy only: openEMS solves Maxwell's equations in time on a 3D section of each line, photonoxide the 2D cross-section's static fields once, so their times measure different problems; openEMS's run times are shown for planning only.

## A microstrip line in a box

A perfectly conducting strip of zero thickness, 20 µm wide, on 20 µm of a substrate of ε = 10 (w/h = 1) over a ground plane, in a perfectly conducting box ±200 µm across and 200 µm high above the strip, its walls at the ground's potential. openEMS runs a section 4 mm long as the coplanar line's (25 µm cells along it, 8-cell PMLs at both ends), driven by a soft source of E between the strip and the ground (a Gaussian pulse of 5 to 55 GHz) until the energy is down by 70 dB; k and Z₀ at each frequency come from the voltage from the strip down to the ground at three planes 1025 µm apart and the current around the strip midway (a loop ±20 µm across and ±5 µm about the strip, clear of the ground), its quasi-static n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². photonoxide's quasi-TEM line is its static solver's on half the box: n = √(C/C₀), Z₀ = 1/(c√(CC₀)). Across the line both take the same mesh lines, cells from the smallest at the strip's edge and in its plane growing by 1.1 to 10 µm; three meshes, the fewest an extrapolation takes, to keep openEMS's runs to about an hour.

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

openEMS extrapolated against photonoxide converged: the microstrip: n −0.11 % (tolerance 0.5 %); the microstrip: Z₀ +0.13 % (tolerance 0.5 %).

# Cross-code comparison: openEMS on more lines

Written by `cargo test --release --test openems_lines -- --ignored` (crosscode/README.md) on a computer with openEMS installed; don't edit it by hand. It is a local comparison, never run in CI, and [the main report](validation.md) doesn't depend on it. The coplanar line, the first of these pairs, is in [its own report](validation-openems.md); how openEMS is found and run, and its licence: [cross-code references](cross-code.md); how to run it: [crosscode/README.md](../crosscode/README.md).

openEMS v0.37.0 (CSXCAD v0.7.0), found through $PHOTONOXIDE_OPENEMS, its multithreaded engine on 4 threads (fewer than the coplanar line's 8, other work sharing the computer); photonoxide 0.6.0. Every error is against photonoxide's converged result, the reference, in percent. These pairs compare accuracy only: openEMS solves Maxwell's equations in time on a 3D section of each line, photonoxide the 2D cross-section's static fields once, so their times measure different problems; openEMS's run times are shown for planning only.

## A microstrip line in a box

A perfectly conducting strip of zero thickness, 20 µm wide, on 20 µm of a substrate of ε = 10 (w/h = 1) over a ground plane, in a perfectly conducting box ±200 µm across and 200 µm high above the strip, its walls at the ground's potential. openEMS runs a section 4 mm long as the coplanar line's (25 µm cells along it, 8-cell PMLs at both ends), driven by a soft source of E between the strip and the ground (a Gaussian pulse of 5 to 55 GHz) until the energy is down by 70 dB; k and Z₀ at each frequency come from the voltage from the strip down to the ground at three planes 1025 µm apart and the current around the strip midway (a loop ±20 µm across and ±5 µm about the strip, clear of the ground), its quasi-static n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². photonoxide's quasi-TEM line is its static solver's on half the box: n = √(C/C₀), Z₀ = 1/(c√(CC₀)). Across the line both take the same mesh lines, cells from the smallest at the strip's edge and in its plane growing by 1.1 to 10 µm; three meshes, the fewest an extrapolation takes.

| | n | Error | Z₀, Ω | Error |
|---|---:|---:|---:|---:|
| photonoxide, converged (the reference) | 2.580191 | — | 48.6603 | — |
| openEMS, extrapolated from its three meshes | 2.577318 | −0.11 % | 48.7212 | +0.13 % |
| photonoxide on the same lines, extrapolated | 2.579828 | −0.014 % | 48.5756 | −0.17 % |
| openEMS on its finest mesh (0.5 µm) | 2.581084 | +0.035 % | 48.2319 | −0.88 % |
| photonoxide on the same mesh | 2.579828 | −0.014 % | 49.0329 | +0.77 % |
| Hammerstad and Jensen (1980), closed form, the line in the open | 2.589451 | +0.36 % | 48.8226 | +0.33 % |

Hammerstad and Jensen's closed form (their Eqs. 1–5, the strip of zero thickness, at zero frequency; IEEE MTT-S Digest 1980, p. 407) is for the line in the open, and their stated accuracy is 0.2 % on ε_e (0.1 % on n) and 0.01 % on Z₀₁ (0.11 % on Z₀ with ε_e's). In the box both codes meet each other, not the open line: the box's walls, ten substrate heights away, add a little capacitance. photonoxide's converged line in a box twenty times as wide and as high (±4000 µm, graded as the reference out to 50 µm cells) gives n 2.588506 and Z₀ 48.8432 Ω, against the closed form's 2.589451 and 48.8226 Ω: −0.036 % on n and +0.042 % on Z₀, both within the paper's stated accuracy. In the box, photonoxide's converged line is −0.36 % on n and −0.33 % on Z₀ from the closed form, openEMS's extrapolated −0.47 % and −0.21 %: the box's effect, which the twenty-times box shows is nearly all of it.
| Smallest cell | openEMS's cells | Steps (its run) | n at 10, 20, 30, 40, 50 GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |
|---:|---:|---:|---|---|---|---|---|---|
| 2 µm | 390720 | 40494 (1 min) | 2.58484, 2.58552, 2.58655, 2.58795, 2.58965 | 47.076, 47.091, 47.117, 47.153, 47.197 | 2.58470, 47.071 | +0.17 %, −3.27 % | 2.57750, 50.106 | −0.10 %, +2.97 % |
| 1 µm | 660480 | 75504 (3 min) | 2.58274, 2.58338, 2.58440, 2.58578, 2.58744 | 47.827, 47.843, 47.869, 47.905, 47.949 | 2.58259, 47.823 | +0.093 %, −1.72 % | 2.57863, 49.412 | −0.060 %, +1.54 % |
| 0.5 µm | 1017600 | 159024 (8 min) | 2.58124, 2.58186, 2.58287, 2.58424, 2.58590 | 48.236, 48.252, 48.278, 48.314, 48.358 | 2.58108, 48.232 | +0.035 %, −0.88 % | 2.57983, 49.033 | −0.014 %, +0.77 % |

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
| 2 µm | 761600 | 143650 (7 min; its energy settled at -13.4 dB, the voltage 109 dB down) | 2.19886, 2.19899, 2.19923, 2.19956, 2.19999 | 37.765, 37.762, 37.759, 37.754, 37.748 | 2.19880, 37.765 | −0.86 %, −6.32 % | 2.20208, 43.527 | −0.71 %, +7.98 % |
| 1 µm | 1347840 | 185385 (17 min; its energy settled at -13.5 dB, the voltage 115 dB down) | 2.21277, 2.21287, 2.21311, 2.21345, 2.21390 | 39.189, 39.187, 39.184, 39.179, 39.172 | 2.21270, 39.190 | −0.23 %, −2.78 % | 2.20648, 41.741 | −0.52 %, +3.54 % |
| 0.5 µm | 2273280 | 298265 (38 min; its energy settled at -13.4 dB, the voltage 114 dB down) | 2.21909, 2.21918, 2.21943, 2.21977, 2.22023 | 39.770, 39.769, 39.765, 39.760, 39.753 | 2.21901, 39.772 | +0.050 %, −1.34 % | 2.21076, 40.952 | −0.32 %, +1.59 % |

| | n | Z₀, Ω | From |
|---|---:|---:|---|
| openEMS, extrapolated | 2.22427 | 40.172 | its three meshes; n order 1.14, Z₀ order 1.29 |
| photonoxide on the same lines, extrapolated | 2.37608 | 40.330 | the same; n order 0.04, Z₀ order 1.18 |
| photonoxide, converged | 2.21790 | 40.312 | graded from 0.0025 µm by 1.025 to 2 µm (from 0.01, 0.005, 0.0025 µm: n 2.217770, 2.217858, 2.217902, Z₀ 40.3165, 40.3132, 40.3116; extrapolated n 2.217948, Z₀ 40.3102, order 1.09) |

An extrapolation of order below 0.5 says little: its three meshes' differences barely shrink, and the finest mesh is the better guide. Here: photonoxide's n on the same lines (order 0.04).

## A lossy line to 300 GHz

A coplanar line of copper (5.8 × 10⁷ S/m) 0.5 µm thick, about four skin depths at 300 GHz (0.12 µm), the signal 10 µm wide, gaps of 5 µm, grounds 20 µm wide, on 10 µm of fused silica (ε 3.8) whose underside is the floor of a perfectly conducting box, the grounds running to its sides at ±30 µm (so that they are the box's potential, and no static charge is left on them), its lid 50 µm above the substrate: small enough that the box's own modes start above 300 GHz. Where the quasi-TEM line no longer holds, photonoxide's full-wave RF mode (the cross-section as a waveguide at the frequency, the copper a lossy medium whose skin the grid resolves; half the line beside the mirror through the signal, its even mode) against openEMS with the copper a conductor (its Kappa) whose skin the mesh resolves. openEMS runs a section 1.2 mm long in 10 µm cells (25 to the guided wavelength at 300 GHz), 8-cell PMLs at both ends, driven across both gaps by a Gaussian pulse of 25 to 325 GHz until the energy is down by 55 dB (its slow last dB would take hours more on the finest mesh; the planes' voltages have rung down by more than 80 dB by then, which the study checks, and on the coarsest mesh the signals cut there give the loss within 0.06 % of a run to 60 dB); the voltage across a gap at three planes 150 µm apart (a quarter of the guided wavelength at 300 GHz, so that the three-plane wavenumber's principal value is the right one) and the current around the signal midway give k = β + iα and Z₀. Both take the same mesh lines across, the smallest cells at the copper's faces two skin depths (at 300 GHz), one, a half and a quarter, growing by 1.1 to 5 µm, and the extrapolation takes the three finest (on the coarsest the skin is two cells deep, and the loss's error isn't yet in its asymptotic range); photonoxide converged graded from 0.0075 µm by 1.05 to 1 µm. The index βc/ω, Re Z₀ and the attenuation α at 100, 200, 300 GHz.

| Frequency | Quantity | photonoxide, converged (the reference) | openEMS, extrapolated | Error | photonoxide on the same lines, extrapolated | Error | openEMS, finest mesh | Error | openEMS against photonoxide on that mesh |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 100 GHz | n | 1.59865 | 1.59905 (no order: the finest shown) | +0.025 % | 1.59893 (no order: the finest shown) | +0.018 % | 1.59905 | +0.025 % | +0.0077 % |
| 100 GHz | Re Z₀, Ω | 62.838 | 62.801 (order 1.43) | −0.060 % | 62.810 (order 1.44) | −0.045 % | 62.771 | −0.11 % | −0.015 % |
| 100 GHz | α, dB/cm | 6.204 | 6.207 (order 1.94) | +0.046 % | 6.211 (order 1.97) | +0.12 % | 6.230 | +0.43 % | −0.063 % |
| 200 GHz | n | 1.59247 | 1.59296 (no order: the finest shown) | +0.031 % | 1.59267 (no order: the finest shown) | +0.013 % | 1.59296 | +0.031 % | +0.019 % |
| 200 GHz | Re Z₀, Ω | 62.544 | 62.454 (order 1.56) | −0.14 % | 62.513 (order 1.57) | −0.050 % | 62.424 | −0.19 % | −0.095 % |
| 200 GHz | α, dB/cm | 8.424 | 8.447 (order 1.99) | +0.28 % | 8.441 (order 1.98) | +0.21 % | 8.503 | +0.94 % | +0.061 % |
| 300 GHz | n | 1.58990 | 1.59073 (order 6.69) | +0.052 % | 1.59005 (order 6.84) | +0.0091 % | 1.59073 | +0.052 % | +0.043 % |
| 300 GHz | Re Z₀, Ω | 62.359 | 62.189 (order 1.66) | −0.27 % | 62.326 (order 1.67) | −0.054 % | 62.159 | −0.32 % | −0.22 % |
| 300 GHz | α, dB/cm | 10.423 | 10.460 (order 1.90) | +0.35 % | 10.443 (order 1.89) | +0.19 % | 10.559 | +1.30 % | +0.15 % |

On every mesh openEMS and photonoxide agree within 0.22 % on every figure (the last column, for the finest): mesh by mesh the two codes give the same line. The loss converges more slowly than n and Re Z₀, the current crowding into the copper's skin, a few cells deep: it moves by up to 2.6 % between the two finest meshes. With a quarter of the skin depth at 300 GHz as the finest cell, extrapolated from the three finest meshes, openEMS's loss meets photonoxide's converged loss within 0.35 % at every frequency (tolerance 2 %).

| Smallest cell | openEMS's cells | Steps (its run) | openEMS: n, Re Z₀ (Ω), α (dB/cm) at 100, 200, 300 GHz | photonoxide, the same lines |
|---:|---:|---:|---|---|
| 0.24 µm | 816480 | 98820 (18 min; the voltage 92 dB down) | 1.59744, 62.213, 7.143; 1.58792, 61.739, 10.030; 1.58430, 61.423, 12.012 | 1.59732, 62.225, 7.148; 1.58762, 61.797, 10.025; 1.58363, 61.560, 12.003 |
| 0.12 µm | 1468800 | 162051 (34 min; the voltage 92 dB down) | 1.59940, 62.582, 6.554; 1.59218, 62.192, 9.332; 1.58907, 61.894, 11.830 | 1.59927, 62.591, 6.559; 1.59189, 62.251, 9.327; 1.58839, 62.031, 11.814 |
| 0.06 µm | 2476800 | 343938 (85 min; the voltage 91 dB down) | 1.59937, 62.720, 6.297; 1.59309, 62.365, 8.670; 1.59071, 62.095, 10.828 | 1.59925, 62.730, 6.300; 1.59279, 62.425, 8.666; 1.59003, 62.233, 10.814 |
| 0.03 µm | 3880800 | 638438 (211 min; the voltage 89 dB down) | 1.59905, 62.771, 6.230; 1.59296, 62.424, 8.503; 1.59073, 62.159, 10.559 | 1.59893, 62.781, 6.234; 1.59267, 62.483, 8.498; 1.59005, 62.296, 10.543 |

photonoxide converged, graded from 0.03, 0.015, 0.0075 µm: 1.59895, 62.813, 6.231; 1.59269, 62.515, 8.495; 1.59007, 62.329, 10.544 | 1.59876, 62.831, 6.210; 1.59256, 62.535, 8.440; 1.58999, 62.350, 10.451 | 1.59865, 62.838, 6.204; 1.59247, 62.544, 8.424; 1.58990, 62.359, 10.423.

## Kharel et al.'s T-rail electrodes

Kharel et al.'s segmented electrodes on thin-film lithium niobate on quartz (Optica 8, 357 (2021), Table 1's row d and Fig. 1; the erratum, Optica 8, 1218 (2021), corrects only Fig. 4(a)): a signal 100 µm wide, the rails' gap g = 5 µm, each rail s = 2 µm wide across, h = 5 µm from its channel electrode, r = 45 µm long, c = 5 µm between rails along the line, each joined to its channel by a stem t = 5 µm wide in its middle; the channels 19 µm apart, the grounds 100 µm wide (not given). The electrodes 0.8 µm thick (gold, perfect conductors here), on a 0.6 µm x-cut film etched to a slab of 0.25 µm and a rib 2 µm wide in the rails' gap (upright sidewalls), on 2 µm of oxide (ε 3.9) over 500 µm of quartz (ε 4.5), under 1 µm of oxide over the slab and air, in a perfectly conducting box as wide as the line (±169 µm), its lid 200 µm above the film; the film's permittivity 26.2 across and 45.5 up and along, as Zhang et al.'s line's.

photonoxide's line is its model of these electrodes (the validation cases line/kharel-2021-segmented-*): a period of half the channel line's c, the line loaded by the rails r long (Shin et al. 2005, Eq. 7: the capacitance of the section through the rails, the channel line's inductance) and the other half, each section from its static solver on half the box, the stems left out, and the Bloch wave of the period's ABCD matrix (`electro::line::Periodic`) at 1 GHz, where it is the static line's. openEMS solves the electrodes as drawn, rails, stems and all, in 3D: a section 1.3 mm long (26 periods), its mesh the same in every period, and at both ends a lead of the channel line alone (60 µm in 5 µm cells) ended by Mur's absorbing condition (the pulse leaves a static field on the line, under which openEMS's PMLs at the ends grew unstable, the energy rising without end some 70 to 90 ps in), driven across the channels' gap at the middle of a gap between rails by a Gaussian pulse of 10 to 110 GHz, until the energy has settled or fallen by 70 dB; the voltage across the gap at the rib's top at three planes 300 µm apart at the middles of gaps between rails (six periods, so that the Bloch wave is the same at each and midway), the current around the signal and its rails midway (the mean of the dual mesh's planes either side), give k and the Bloch Z₀ at 10 to 50 GHz, and n and Z₀ the limits at 0 GHz of v(f) = v₀ + a f². It solves half the line, beside a magnetic wall through the signal's middle (the even mode's symmetry), the current loops starting at the first two dual lines beside the wall and the current between the wall and the first added in proportion; on the coarsest mesh the whole line checks it. Across and in height both codes take the same mesh lines, from the smallest at every edge and layer growing by 1.2 to 25 µm; along the line openEMS's cells start at twice the smallest at the rails' ends and the stems' edges and grow by 1.3 to 5 µm.

| | n | Error | Z₀, Ω | Error |
|---|---:|---:|---:|---:|
| photonoxide, converged (the reference) | 2.122647 | — | 40.4707 | — |
| openEMS, extrapolated from its three meshes | 2.136312 | +0.64 % | 39.3642 | −2.73 % |
| photonoxide on the same lines, extrapolated | 2.137797 | +0.71 % | 40.4074 | −0.16 % |
| openEMS on its finest mesh (0.5 µm) | 2.143493 | +0.98 % | 39.0704 | −3.46 % |
| photonoxide on the same mesh | 2.103432 | −0.91 % | 41.0586 | +1.45 % |
| photonoxide, converged, every gap between rails loaded as the rails (the 2D sections' bound) | 2.163930 | +1.94 % | 39.6986 | −1.91 % |
| openEMS, the whole line, coarsest mesh (2 µm) | 2.168929 | +2.18 % | 37.9007 | −6.35 % |
| openEMS, its half beside the magnetic wall, the same mesh | 2.171467 | +2.30 % | 37.6880 | −6.88 % |

The half beside its magnetic wall is +0.12 % on n and −0.56 % on Z₀ from the whole line on the coarsest mesh, a difference of the order of its first cell that the finer meshes shrink. Mesh by mesh openEMS and photonoxide differ by up to 12.3 % (+1.90 % on n and −4.84 % on Z₀ on the finest mesh), openEMS's n falling and photonoxide's rising as the cells shrink. Their extrapolations don't meet: openEMS's n and Z₀ give the line a capacitance +3.47 % and an inductance −2.11 % against photonoxide's converged period (C = n/(cZ₀), L = nZ₀/c). openEMS solves the electrodes in 3D; photonoxide's 2D sections leave out the rails' ends fringing across the gaps between them (which the bound above, every gap loaded as the rails, holds at most: there n +1.94 % and Z₀ −1.91 %) and the current that charges the rails through the stems, as Shin et al.'s model does.

| Smallest cell | openEMS's cells (the half) | Steps (its run) | n at 10, 20, 30, 40, 50 GHz | Z₀ at the same, Ω | openEMS at 0 GHz: n, Z₀ | Error: n, Z₀ | photonoxide, the same lines: n, Z₀ | Error: n, Z₀ |
|---:|---:|---:|---|---|---|---|---|---|
| 2 µm | 807040 | 97335 (3 min; its energy settled at -12.5 dB, the voltage 97 dB down) | 2.17098, 2.17268, 2.17377, 2.17485, 2.17631 | 37.704, 37.688, 37.698, 37.718, 37.743 | 2.17147, 37.688 | +2.30 %, −6.88 % | 2.03520, 42.978 | −4.12 %, +6.20 % |
| 1 µm | 2217456 | 113305 (9 min; its energy settled at -12.5 dB, the voltage 88 dB down) | 2.15261, 2.15259, 2.15348, 2.15462, 2.15608 | 38.661, 38.678, 38.690, 38.709, 38.733 | 2.15220, 38.662 | +1.39 %, −4.47 % | 2.07842, 41.701 | −2.08 %, +3.04 % |
| 0.5 µm | 5063008 | 133960 (36 min; its energy settled at -12.5 dB, the voltage 93 dB down) | 2.14246, 2.14530, 2.14596, 2.14700, 2.14844 | 39.091, 39.064, 39.078, 39.099, 39.123 | 2.14349, 39.070 | +0.98 %, −3.46 % | 2.10343, 41.059 | −0.91 %, +1.45 % |

The whole line on the coarsest mesh: 1614080 cells, 83045 steps (5 min; its energy settled at -12.5 dB, the voltage 90 dB down).

| | n | Z₀, Ω | From |
|---|---:|---:|---|
| openEMS, extrapolated | 2.13631 | 39.364 | its three meshes; n order 1.15, Z₀ order 1.26 |
| photonoxide on the same lines, extrapolated | 2.13780 | 40.407 | the same; n order 0.79, Z₀ order 0.99 |
| photonoxide, converged | 2.12265 | 40.471 | graded from 0.0025 µm by 1.025 to 2 µm (from 0.01, 0.005, 0.0025 µm: n 2.122457, 2.122589, 2.122647, Z₀ 40.4749, 40.4720, 40.4707; extrapolated n 2.122692, order 1.19, Z₀ 40.4697, order 1.19) |

openEMS extrapolated against photonoxide converged: the microstrip: n −0.11 % (tolerance 0.5 %); the microstrip: Z₀ +0.13 % (tolerance 0.5 %); Zhang et al.'s line: n +0.29 % (tolerance 0.5 %); Zhang et al.'s line: Z₀ −0.35 % (tolerance 0.5 %); the lossy line at 100 GHz: n +0.025 % (tolerance 0.5 %); the lossy line at 100 GHz: Re Z₀, Ω −0.060 % (tolerance 0.5 %); the lossy line at 100 GHz: α, dB/cm +0.046 % (tolerance 2 %); the lossy line at 200 GHz: n +0.031 % (tolerance 0.5 %); the lossy line at 200 GHz: Re Z₀, Ω −0.14 % (tolerance 0.5 %); the lossy line at 200 GHz: α, dB/cm +0.28 % (tolerance 2 %); the lossy line at 300 GHz: n +0.052 % (tolerance 0.5 %); the lossy line at 300 GHz: Re Z₀, Ω −0.27 % (tolerance 0.5 %); the lossy line at 300 GHz: α, dB/cm +0.35 % (tolerance 2 %); the T-rail line: n +0.64 % (tolerance 0.5 %); the T-rail line: Z₀ −2.73 % (tolerance 0.5 %).

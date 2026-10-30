# Cross-code comparison

photonoxide's solvers, each checked against an established code that solves the same physics.
Each comparison is a pair. Where both codes solve the same equations on the same mesh, the pair
compares accuracy and speed; where they solve the physics differently, it compares accuracy
only. This crate runs the other codes as external programs; it is local only, and CI never builds
it.

## Pairs

| photonoxide's solver | Checked against | Why that code | Compares |
|---|---|---|---|
| The static solver: heat (steady and in time), current and electrostatics, 2D finite volumes | Elmer, 2D finite elements | the same equations on the same mesh lines | accuracy and speed |
| The quasi-TEM RF line, from the static solver | openEMS, 3D FDTD | Maxwell's equations in time, with no quasi-TEM approximation | accuracy only |

Every error is against photonoxide's converged result, in percent. Both codes take the same
mesh lines in each problem.

## photonoxide against Elmer

| Problem | Quantity | Reference | photonoxide | Elmer |
|---|---|---:|---:|---:|
| Coplanar line, 0.125 µm mesh | n | 2.217156 | 2.217159 (+0.00011 %) | 2.217175 (+0.00085 %) |
| | Z₀, Ω | 54.1476 | 54.3714 (+0.41 %) | 54.0150 (−0.24 %) |
| | time | | 0.112 s | 0.073 s |
| Heater over silicon, 0.0125 µm mesh | peak rise, K | 0.327422 | 0.327508 (+0.026 %) | 0.327384 (−0.012 %) |
| | time | | 0.051 s | 0.060 s |
| Two heaters side by side, 0.005 µm mesh | neighbour's rise, K | 0.97981 | 0.97984 (+0.0033 %) | 0.97970 (−0.011 %) |
| | time | | 0.332 s | 0.770 s |
| A heater turned on, 0.005 µm mesh | rise at 1 µs, K | 3.8352 | 3.8374 (+0.055 %) | 3.8337 (−0.039 %) |
| | rise at 5 µs, K | 7.9369 | 7.9403 (+0.042 %) | 7.9343 (−0.033 %) |
| | rise at 20 µs, K | 11.0297 | 11.0330 (+0.030 %) | 11.0272 (−0.022 %) |
| | 1/e time, µs | 4.2594 | 4.2577 (−0.040 %) | 4.2607 (+0.030 %) |
| | time, 800 steps | | 3.97 s | 2 min 57 s |
| Joule heating, 0.005 µm mesh | current at 1 V, A/m | 99.9224 | 99.8660 (−0.056 %) | 99.9585 (+0.036 %) |
| | peak rise, K | 18.5775 | 18.5721 (−0.029 %) | 18.5805 (+0.016 %) |
| | time | | 0.258 s | 0.463 s |
| A modulator's electrodes, 0.005 µm mesh | capacitance, pF/m | 160.360 | 160.276 (−0.053 %) | 160.433 (+0.045 %) |
| | mean E_x at 1 V, V/µm | −0.124437 | −0.124386 (−0.041 %) | −0.124474 (+0.030 %) |
| | time | | 0.195 s | 0.369 s |

- **Agreement:** refined to convergence, Elmer meets photonoxide within 0.00090 % on the line's n and 0.047 % on its Z₀, 0.0099 % on the heater's rise, 0.013 % on the neighbouring heater's rise, 0.034 % on the heater's rise in time and 0.027 % on its 1/e time, 0.0056 % on the doped heater's current and 0.011 % on its rise, and 0.040 % on the modulator's capacitance and 0.016 % on its field. Each problem's tolerance is 0.1 % for n, the current and the capacitance and 0.5 % for the others; all are met.
- **Times:** each code's own assembly and solve on the same mesh, the fastest of three runs. Elmer's are read from its log, which gives the heat solver's to 0.01 s; photonoxide's are measured around its solve. The line takes two solves in each code, with the substrate and without it; Joule heating, a current solve and a heat solve. In time, Elmer's HeatSolve, with its defaults, assembles and factorizes its matrix at every step, where photonoxide factorizes once and then only solves. These times were taken under load, with other builds running on the Intel Core Ultra 7 265K (20 threads), 64 GB, Windows 11: they are rough until they are taken again with the computer idle.
- Every mesh: [docs/validation-elmer.md](../docs/validation-elmer.md).

## photonoxide against openEMS

| Problem | Quantity | Reference | photonoxide | openEMS |
|---|---|---:|---:|---:|
| Coplanar line, 0.25 µm mesh | n | 2.217156 | 2.217052 (−0.0047 %) | 2.217069 (−0.0039 %) |
| | Z₀, Ω | 54.1476 | 54.5551 (+0.75 %) | 53.7113 (−0.81 %) |
| Microstrip, 0.5 µm mesh | n | 2.580191 | 2.579828 (−0.014 %) | 2.581084 (+0.035 %) |
| | Z₀, Ω | 48.6603 | 49.0329 (+0.77 %) | 48.2319 (−0.88 %) |
| | closed form in the open (Hammerstad and Jensen): n, Z₀ | 2.589451, 48.8226 | in a box 20 times as wide: 2.588506 (−0.036 %), 48.8432 (+0.042 %) | |
| Zhang et al.'s line, its full stack, 0.5 µm mesh | n | 2.217902 | 2.210763 (−0.32 %) | 2.219010 (+0.050 %) |
| | Z₀, Ω | 40.3116 | 40.9523 (+1.59 %) | 39.7716 (−1.34 %) |
| A lossy line at 300 GHz, 0.06 µm mesh | n | 1.58990 | 1.59003 (+0.0084 %) | 1.59070 (+0.050 %) |
| | Re Z₀, Ω | 62.359 | 62.233 (−0.20 %) | 62.094 (−0.42 %) |
| | α, dB/cm | 10.423 | 10.814 (+3.75 %) | 10.825 (+3.86 %) |

- **Agreement:** refined to convergence, openEMS meets photonoxide within 0.0039 % on n and 0.20 % on Z₀. This checks the quasi-TEM approximation as well as the solver: openEMS makes no such approximation. The microstrip, from three meshes (2, 1 and 0.5 µm) and openEMS on 4 threads, meets it within 0.11 % on n and 0.13 % on Z₀ (tolerances 0.5 %); its n's extrapolation, of order 0.49, overshoots, where its finest mesh is within 0.035 %. Against Hammerstad and Jensen's closed form for the open line, both codes in the box are 0.2 to 0.5 % off, the box's own effect: photonoxide with the box twenty times as wide comes within the paper's stated accuracy. Zhang et al.'s line with its whole stack (anisotropic film, thick electrodes, cladding, 500 µm of silicon), from three meshes the same way, meets it within 0.29 % on n and 0.35 % on Z₀. Its openEMS runs end when their energy has settled (at −13.4 dB) rather than fallen by 70 dB: a field stays in the box after the pulse, which none of the planes' voltages sees (they have rung down by more than 100 dB by then); we haven't found where it sits. The lossy copper line, against photonoxide's full-wave mode, from three meshes: within 0.11 % on n and 0.18 % on Re Z₀ at 100, 200 and 300 GHz (tolerances 0.5 %), but **not within 2 % on the loss** at 200 and 300 GHz (+3.9 % at 300 GHz; at 200 GHz the extrapolation breaks down): the loss still moves by up to 9 % between the two finest meshes, so they don't yet give its converged value. On each mesh openEMS and photonoxide agree within 0.22 % on every figure, the loss included; a finer openEMS mesh (0.03 µm, about five hours) is what's left.
- **No times:** openEMS solves a 3D section of the line, 4 mm long, in time (2.85 million cells for 339 345 steps on the 0.25 µm mesh), where photonoxide solves the 2D cross-section once. Their times measure different problems.
- Every mesh: [docs/validation-openems.md](../docs/validation-openems.md) and, for the lines after it, [docs/validation-openems-lines.md](../docs/validation-openems-lines.md).

## The problems

- **Ghione and Naldi's coplanar line:** strips of zero thickness, a 20 µm signal and 10 µm gaps, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high. Its index n and impedance Z₀.
  - photonoxide's quasi-TEM line: from the static solver's capacitances with the substrate and without it.
  - Elmer: the same capacitances, from StatElecSolver.
  - openEMS: a 4 mm section driven by a pulse, k and Z₀ from voltages and currents along it, at 0 GHz from a fit over 10 to 50 GHz.
- **A microstrip line:** a strip of zero thickness 20 µm wide on 20 µm of ε = 10 (w/h = 1) over a ground plane, in a perfectly conducting box ±200 µm across and 200 µm high. Its n and Z₀: photonoxide's quasi-TEM line from its static solver, and openEMS's 4 mm section driven between the strip and the ground, as the coplanar line's. Hammerstad and Jensen's closed form (1980, Eqs. 1–5) stands beside them: it is for the line in the open, and photonoxide in a box twenty times as wide meets it within 0.036 % on n and 0.042 % on Z₀, inside the paper's stated accuracy (0.1 % and 0.11 %).
- **Zhang et al.'s line, its full stack:** their thin-film lithium niobate line (2022) with everything around it: a 20 µm signal, 5 µm gaps, 0.83 µm thick electrodes (perfect conductors), the x-cut film's ribs (upright) and its anisotropic permittivity (26.2 across, 45.5 up and along), the PECVD cladding (ε 5.5), 2 µm of oxide over 500 µm of silicon, in a box as wide as the line. Its n and Z₀ from photonoxide's quasi-TEM line and openEMS's 4 mm section, as the coplanar line's.
- **A lossy line to 300 GHz:** a coplanar line of copper 0.5 µm thick (about four skin depths at 300 GHz) on fused silica, in a box as wide as the line, where the quasi-TEM line no longer holds: its index, Re Z₀ and loss at 100, 200 and 300 GHz from photonoxide's full-wave RF mode and from openEMS with the copper a conductor whose skin the mesh resolves.
- **A heater over silicon:** 2 × 0.2 µm, 2 µm up in oxide on 10 µm of silicon held at 0, dissipating 1 W/m. Its peak temperature rise, from photonoxide's heat solver and from Elmer's HeatSolver.
- **Two heaters side by side:** Jacques et al.'s N++ silicon heater (2019) beside a second one, 5 µm between their N++ strips, the first dissipating the paper's P_π (71.25 W/m) and the second off. The rise at the second one's guide, the thermal crosstalk, from photonoxide's heat solver and from Elmer's HeatSolver.
- **A heater turned on:** the same heater alone, cold until it is switched on. Its guide's rise at 1, 5 and 20 µs and its 1/e time, from photonoxide's transient (BDF2) and from Elmer's HeatSolver in time (BDF of order 2), with the same time steps.
- **Joule heating:** Harris et al.'s doped silicon heater (2014), a current across the guide through p and p++ silicon between contacts 6 µm apart at 1 V and 0 V. The current per metre and the peak rise of its Joule heat, from photonoxide's current continuity and heat solver and from Elmer's StatCurrentSolver and HeatSolver.
- **A modulator's electrodes:** Zhang et al.'s thin-film lithium niobate cross-section (2022), the film's permittivity a diagonal tensor (26.2 across, 45.5 up). The line's capacitance and the mean field across the guide at 1 V, which sets V_π·L, from photonoxide's electrostatics and from Elmer's StatElecSolver with a tensor permittivity.

More problems, each with the pair it suits: [#536](https://github.com/tachsin/photonoxide/issues/536).

## Running it

From the repository's root:

```sh
PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer   # writes docs/validation-elmer.md
PHOTONOXIDE_OPENEMS=C:/openEMS just openems                                    # writes docs/validation-openems.md
PHOTONOXIDE_OPENEMS=C:/openEMS cargo test --manifest-path crosscode/Cargo.toml --release --test openems_lines -- --ignored --nocapture   # writes docs/validation-openems-lines.md
just crosscode                                                               # this crate's fmt, clippy and unit tests
```

- **Finding the programs:** `PHOTONOXIDE_ELMER` and `PHOTONOXIDE_OPENEMS` name each program or its folder; otherwise it is looked for on `PATH`. `PHOTONOXIDE_ELMER_PATH` names folders of Elmer's run-time libraries when they aren't on `PATH`. Without a program, its study prints `skipped:` and passes.
- **Settings:** `PHOTONOXIDE_MACHINE` names the computer in the reports. `PHOTONOXIDE_OPENEMS_THREADS` sets openEMS's threads (8 by default). `PHOTONOXIDE_LOADED=1` says in Elmer's report that its times were taken while the computer was busy with other work. `PHOTONOXIDE_ELMER_ONLY` runs only some of Elmer's problems (`line`, `heater`, `heaters`, `transient`, `joule`, `tfln`, comma-separated) and prints their report without writing it; `PHOTONOXIDE_OPENEMS_ONLY` does the same for the later openEMS lines (`microstrip`, `zhang`, `lossy`).
- **Runs:** they are kept in `target/tmp/elmer/` and `target/tmp/openems/`, with each program's log beside them. Elmer's study takes about 35 minutes, most of it Elmer's steps in time. openEMS's takes about 1 h 13 min from scratch, and seconds when it can reuse its finished runs.
- **Timings:** take them with the computer otherwise idle.
- **Getting the programs and their licences:** [docs/cross-code.md](../docs/cross-code.md).

## Files

| File | What it does |
|---|---|
| `src/elmer.rs` | finds ElmerSolver, writes its mesh files and solver input (SIF) from its documented formats, runs it, reads its output (values at nodes too, for a guide's mean or a field across it) and its solvers' times |
| `src/openems.rs` | finds openEMS, writes its XML input from its documented format, runs it with its progress, reads its probes |
| `src/cpw.rs` | the coplanar line, on the same mesh lines for photonoxide, Elmer and openEMS |
| `src/microstrip.rs` | a microstrip line, on the same mesh lines for photonoxide and openEMS |
| `src/zhang_line.rs` | Zhang et al.'s thin-film line with its whole stack, for photonoxide and openEMS |
| `src/lossy_line.rs` | a copper line to 300 GHz, for photonoxide's full-wave RF mode and openEMS with the copper a conductor |
| `src/heater.rs` | the heater, for photonoxide and Elmer |
| `src/heaters.rs` | two heaters side by side, the crosstalk, for photonoxide and Elmer |
| `src/transient.rs` | a heater turned on, its rise in time, for photonoxide and Elmer |
| `src/joule.rs` | a doped heater's current and its Joule heat, for photonoxide and Elmer |
| `src/tfln.rs` | a thin-film lithium niobate modulator's electrodes, for photonoxide and Elmer |
| `src/line.rs` | a line's wavenumber and impedance from voltages and currents at planes along it |
| `src/report.rs` | errors in percent, times and the computer's name, as the reports print them |
| `tests/elmer.rs`, `tests/openems.rs`, `tests/openems_lines.rs` | the studies, which write the reports (the last, openEMS on the lines after the coplanar one, writes its own) |

## Rules

- **External programs only:** Elmer (GPL-2.0 or later) and openEMS (GPL-3.0) are run as programs. Nothing of theirs is linked, built, shipped or read; their input is written from their documentation, and only their output is read back.
- **A new problem:** it is a pair, photonoxide's solver against the code that solves the same physics, both on the same mesh lines. Its study reports each code's value and its error in percent against photonoxide's converged result. It gives times only when the two codes solve the same equations on the same mesh.

# Cross-code comparison

photonoxide's solvers, each checked against an established code that solves the same physics.
Each comparison is a pair. Where both codes solve the same equations on the same mesh, the pair
compares accuracy and speed; where they solve the physics differently, it compares accuracy
only. This crate runs the other codes as external programs; it is local only, and CI never builds
it.

## Pairs

| photonoxide's solver | Checked against | Why that code | Compares |
|---|---|---|---|
| The static solver: heat and electrostatics, 2D finite volumes | Elmer, 2D finite elements | the same equations on the same mesh lines | accuracy and speed |
| The quasi-TEM RF line, from the static solver | openEMS, 3D FDTD | Maxwell's equations in time, with no quasi-TEM approximation | accuracy only |

Every error is against photonoxide's converged result, in percent. Both codes take the same
mesh lines in each problem.

## photonoxide against Elmer

| Problem | Quantity | Reference | photonoxide | Elmer |
|---|---|---:|---:|---:|
| Coplanar line, 0.125 µm mesh | n | 2.217156 | 2.217159 (+0.00011 %) | 2.217175 (+0.00085 %) |
| | Z₀, Ω | 54.1476 | 54.3714 (+0.41 %) | 54.0150 (−0.24 %) |
| | time | | 0.053 s | 0.075 s |
| Heater over silicon, 0.0125 µm mesh | peak rise, K | 0.327422 | 0.327508 (+0.026 %) | 0.327384 (−0.012 %) |
| | time | | 0.029 s | 0.070 s |

- **Agreement:** refined to convergence, Elmer meets photonoxide within 0.00090 % on n, 0.047 % on Z₀ and 0.0099 % on the heater's rise.
- **Times:** each code's own assembly and solve on the same mesh, the fastest of three runs. Elmer's are read from its log, which gives the heat solver's to 0.01 s; photonoxide's are measured around its solve. The line takes two solves in each code, with the substrate and without it. They were taken on an Intel Core Ultra 7 265K (20 threads), 64 GB, Windows 11, with nothing else running.
- Every mesh: [docs/validation-elmer.md](../docs/validation-elmer.md).

## photonoxide against openEMS

| Problem | Quantity | Reference | photonoxide | openEMS |
|---|---|---:|---:|---:|
| Coplanar line, 0.25 µm mesh | n | 2.217156 | 2.217052 (−0.0047 %) | 2.217069 (−0.0039 %) |
| | Z₀, Ω | 54.1476 | 54.5551 (+0.75 %) | 53.7113 (−0.81 %) |

- **Agreement:** refined to convergence, openEMS meets photonoxide within 0.0039 % on n and 0.20 % on Z₀. This checks the quasi-TEM approximation as well as the solver: openEMS makes no such approximation.
- **No times:** openEMS solves a 3D section of the line, 4 mm long, in time (2.85 million cells for 339 345 steps on the 0.25 µm mesh), where photonoxide solves the 2D cross-section once. Their times measure different problems.
- Every mesh: [docs/validation-openems.md](../docs/validation-openems.md).

## The problems

- **Ghione and Naldi's coplanar line:** strips of zero thickness, a 20 µm signal and 10 µm gaps, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high. Its index n and impedance Z₀.
  - photonoxide's quasi-TEM line: from the static solver's capacitances with the substrate and without it.
  - Elmer: the same capacitances, from StatElecSolver.
  - openEMS: a 4 mm section driven by a pulse, k and Z₀ from voltages and currents along it, at 0 GHz from a fit over 10 to 50 GHz.
- **A heater over silicon:** 2 × 0.2 µm, 2 µm up in oxide on 10 µm of silicon held at 0, dissipating 1 W/m. Its peak temperature rise, from photonoxide's heat solver and from Elmer's HeatSolver.

More problems, each with the pair it suits: [#536](https://github.com/tachsin/photonoxide/issues/536).

## Running it

From the repository's root:

```sh
PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer   # writes docs/validation-elmer.md
PHOTONOXIDE_OPENEMS=C:/openEMS just openems                                    # writes docs/validation-openems.md
just crosscode                                                               # this crate's fmt, clippy and unit tests
```

- **Finding the programs:** `PHOTONOXIDE_ELMER` and `PHOTONOXIDE_OPENEMS` name each program or its folder; otherwise it is looked for on `PATH`. `PHOTONOXIDE_ELMER_PATH` names folders of Elmer's run-time libraries when they aren't on `PATH`. Without a program, its study prints `skipped:` and passes.
- **Settings:** `PHOTONOXIDE_MACHINE` names the computer in the reports. `PHOTONOXIDE_OPENEMS_THREADS` sets openEMS's threads (8 by default).
- **Runs:** they are kept in `target/tmp/elmer/` and `target/tmp/openems/`, with each program's log beside them. Elmer's study takes about a minute. openEMS's takes about 1 h 13 min from scratch, and seconds when it can reuse its finished runs.
- **Timings:** take them with the computer otherwise idle.
- **Getting the programs and their licences:** [docs/cross-code.md](../docs/cross-code.md).

## Files

| File | What it does |
|---|---|
| `src/elmer.rs` | finds ElmerSolver, writes its mesh files and solver input (SIF) from its documented formats, runs it, reads its output and its solvers' times |
| `src/openems.rs` | finds openEMS, writes its XML input from its documented format, runs it with its progress, reads its probes |
| `src/cpw.rs` | the coplanar line, on the same mesh lines for photonoxide, Elmer and openEMS |
| `src/heater.rs` | the heater, for photonoxide and Elmer |
| `src/line.rs` | a line's wavenumber and impedance from voltages and currents at planes along it |
| `src/report.rs` | errors in percent, times and the computer's name, as the reports print them |
| `tests/elmer.rs`, `tests/openems.rs` | the two studies, which write the reports |

## Rules

- **External programs only:** Elmer (GPL-2.0 or later) and openEMS (GPL-3.0) are run as programs. Nothing of theirs is linked, built, shipped or read; their input is written from their documentation, and only their output is read back.
- **A new problem:** it is a pair, photonoxide's solver against the code that solves the same physics, both on the same mesh lines. Its study reports each code's value and its error in percent against photonoxide's converged result. It gives times only when the two codes solve the same equations on the same mesh.

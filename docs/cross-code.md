# Cross-code comparison

photonoxide's solvers, each checked against an established code that solves the same physics.
Each comparison is a pair. Where both codes solve the same equations on the same mesh, the pair
compares accuracy and speed; where they solve the physics differently, it compares accuracy
only. The crate in [crosscode/](../crosscode/README.md) runs the other codes as external programs;
it is local only, CI never runs it, and photonoxide's own validation report
([docs/validation.md](validation.md)) doesn't depend on it.

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
- Every mesh: [docs/validation-elmer.md](validation-elmer.md).

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
- Every mesh: [docs/validation-openems.md](validation-openems.md) and, for the lines after it, [docs/validation-openems-lines.md](validation-openems-lines.md).

## The problems

- **Ghione and Naldi's coplanar line:** strips of zero thickness, a 20 µm signal and 10 µm gaps, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high. Its index n and impedance Z₀.
  - photonoxide's quasi-TEM line: from the static solver's capacitances with the substrate and without it.
  - Elmer: the same capacitances, from StatElecSolver.
  - openEMS: a 4 mm section driven by a pulse, k and Z₀ from voltages and currents along it, at 0 GHz from a fit over 10 to 50 GHz.
- **A microstrip line:** a strip of zero thickness 20 µm wide on 20 µm of ε = 10 (w/h = 1) over a ground plane, in a perfectly conducting box ±200 µm across and 200 µm high. Its n and Z₀: photonoxide's quasi-TEM line from its static solver, and openEMS's 4 mm section driven between the strip and the ground, as the coplanar line's. Hammerstad and Jensen's closed form (1980, Eqs. 1–5) stands beside them: it is for the line in the open, and photonoxide in a box twenty times as wide meets it within 0.036 % on n and 0.042 % on Z₀, inside the paper's stated accuracy (0.1 % and 0.11 %).
- **Zhang et al.'s line, its full stack:** their thin-film lithium niobate line (2022) with everything around it: a 20 µm signal, 5 µm gaps, 0.83 µm thick electrodes (perfect conductors), the x-cut film's ribs (upright) and its anisotropic permittivity (26.2 across, 45.5 up and along), the PECVD cladding (ε 5.5), 2 µm of oxide over 500 µm of silicon, in a box as wide as the line. Its n and Z₀ from photonoxide's quasi-TEM line and openEMS's 4 mm section, as the coplanar line's.
- **A lossy line to 300 GHz:** a coplanar line of copper 0.5 µm thick (about four skin depths at 300 GHz) on fused silica, in a box as wide as the line, where the quasi-TEM line no longer holds: its index, Re Z₀ and loss at 100, 200 and 300 GHz from photonoxide's full-wave RF mode and from openEMS with the copper a conductor whose skin the mesh resolves.
- **A heater over silicon:** 2 × 0.2 µm, 2 µm up in oxide on 10 µm of silicon held at 0, dissipating 1 W/m. Its peak temperature rise, from photonoxide's heat solver and from Elmer's HeatSolver.
- **Two heaters side by side:** Jacques et al.'s N++ silicon heater (Opt. Express 27, 10456 (2019)) beside a second one, 5 µm between their N++ strips, in the paper's stack (2 µm of buried oxide, the oxide's top 3.32 µm up with 10 W/(m²·K) to air, the window 18 µm high), the first dissipating the paper's P_π, 22.8 mW over 320 µm, and the second off. The mean rise of the second one's rib, the thermal crosstalk.
- **A heater turned on:** the same heater alone in the paper's 40 × 18 µm window, cold until it is switched on, with the paper's heat capacities. Its rib's mean rise at 1, 5 and 20 µs and its 1/e time, both codes stepping by BDF of order 2 with the same 0.025 µs steps; on one mesh, halving that step moves them by 0.0063 % or less.
- **Joule heating:** Harris et al.'s doped silicon heater (Opt. Express 22, 10487 (2014)), the section through a channel: a 500 nm ridge on a 90 nm slab, p-doped 1 µm wide and p++ beyond, contacts 6 µm apart at 1 V and 0 V, the conductivities from the paper's sheet resistances, the oxide held at 0 2 µm below and 10 µm above the silicon. Its current per metre and its peak rise.
- **A modulator's electrodes:** Zhang et al.'s thin-film lithium niobate cross-section (Photon. Res. 10, 2380 (2022)): a 600 nm x-cut film etched 300 nm to a 1.2 µm rib (its sidewalls upright here), electrodes 0.83 µm thick 5 µm apart, the paper's PECVD cladding (ε 5.5), the film's clamped permittivity a diagonal tensor (26.2 across, 45.5 up). The line's capacitance and the mean field across the guide at 1 V, which sets V_π·L.

More problems, each with the pair it suits: [#536](https://github.com/tachsin/photonoxide/issues/536).

## Licences

| Program | Version run | Licence | What photonoxide does with it |
|---|---|---|---|
| [openEMS](https://openems.de) (3D FDTD) | v0.37.0 (the Windows package `openEMS_x64_v0.37.0_msvc`) | GPL-3.0 (its `COPYING`: GNU General Public License, version 3) | runs `openEMS` on an XML input it writes, reads the probes' files |
| CSXCAD (openEMS's geometry library, inside openEMS) | v0.7.0 | LGPL-3.0 (its `COPYING`: GNU Lesser General Public License, version 3) | none: openEMS reads the geometry |
| [Elmer](https://www.elmerfem.org) (finite elements: heat, electrostatics) | 26.2 (ElmerSolver, built from its release's source) | GPL-2.0 or later (its `LICENSE.md`: "version 2 of the License, or (at your option) any later version"; the UMFPACK it ships, which these runs' direct solves use, LGPL) | runs `ElmerSolver` on a mesh and a solver input file (SIF) it writes, reads its output files |

As AGENTS.md has it for GPL codes, these are run as external programs only: nothing of theirs is
linked, built, shipped or read. The input files are written in Rust from the programs'
documentation (openEMS's concept pages on [properties](https://docs.openems.de/en/latest/concepts/properties.html),
[primitives](https://docs.openems.de/en/latest/concepts/primitives.html),
[boundary conditions](https://docs.openems.de/en/latest/concepts/bc.html),
[excitations](https://docs.openems.de/en/latest/concepts/excitations.html),
[signals](https://docs.openems.de/en/latest/concepts/signals.html) and
[probes](https://docs.openems.de/en/latest/concepts/probes.html)), and checked against what the
program reports of them (`--debug-CSX` writes the model as it read it,
`--showProbeDiscretization` where each probe snapped to the mesh). Its Python and MATLAB
interfaces aren't used.

openEMS is T. Liebig, A. Rennings, S. Held and D. Erni's, Int. J. Numer. Model. 26, 680 (published
online 10 December 2012; the 2013 volume), doi:10.1002/jnm.1875, the publication its
documentation asks to be cited.

## Getting the programs

### openEMS

- **Windows:** the ready-made package from the
  [openEMS-Project releases](https://github.com/thliebig/openEMS-Project/releases)
  (`openEMS_x64_v0.37.0_msvc.zip`), unzipped anywhere: `openEMS.exe` runs from its folder,
  nothing to install.
- **Linux:** a build from source (Debian 12 packages openEMS 0.0.35 of 2019 as `openems`, far
  behind the releases): `git clone --recursive https://github.com/thliebig/openEMS-Project.git`, then
  `./update_openEMS.sh ~/opt/openEMS` in it, the program in `~/opt/openEMS/bin`
  ([the build guide](https://docs.openems.de/en/latest/install/clone-build-install.html)).
- **macOS:** a build from source the same way; openEMS's documentation says its Homebrew formula
  isn't maintained.

photonoxide looks for the program in this order:

1. `PHOTONOXIDE_OPENEMS`, naming the program (`openEMS.exe`, `openEMS`, a build's `openEMS.sh`)
   or the folder holding it;
2. each folder on `PATH`.

### Elmer

- **Linux:** CSC's packages for Ubuntu, from their PPA
  ([`ppa:elmer-csc-ubuntu/elmer-csc-ppa`](https://launchpad.net/~elmer-csc-ubuntu/+archive/ubuntu/elmer-csc-ppa)),
  or a build from source with CMake ([its releases](https://github.com/ElmerCSC/elmerfem/releases)).
- **Windows:** a build from source. The one these reports ran was
  built from the 26.2 release's source with MSYS2's UCRT64 toolchain (gfortran, OpenBLAS, MUMPS
  and MS-MPI); its programs are in the build's `install\bin`, and its libraries in
  `C:\msys64\ucrt64\bin`.
- **macOS:** a build from source with CMake.

photonoxide looks for ElmerSolver in `PHOTONOXIDE_ELMER` (the program, `ElmerSolver.exe` or
`ElmerSolver`, or the folder holding it), then on `PATH`. A build whose run-time libraries
aren't on `PATH` names their folders in `PHOTONOXIDE_ELMER_PATH`, as `PATH` lists them: they are
put before `PATH` for Elmer's runs only (without them a Windows build stops at once with
0xC0000135, a library not found, and the harness says to set it). ElmerGrid isn't needed: the
harness writes Elmer's mesh files itself.

## Running it

```sh
PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer
PHOTONOXIDE_OPENEMS=C:/openEMS just openems
```

Each writes its report. Every option, where the runs are kept, and what each file of the harness
does: [crosscode/README.md](../crosscode/README.md).

# Cross-code comparison

photonoxide's solvers checked against established codes on the same problems and the same
meshes: how far apart the results are, and how long each code takes. The codes are openEMS
(3D FDTD) for the RF line and Elmer (finite elements) for heat and electrostatics, run as
external programs by the crate in [crosscode/](../crosscode/README.md). It is local only: CI never
runs it, and photonoxide's own validation report ([docs/validation.md](validation.md)) doesn't
depend on it.

## Results

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="cross-code-chart-dark.svg">
  <img src="cross-code-chart.svg" alt="For each problem, each code's time on the same mesh on a log scale, with its time and errors beside it: on the coplanar line photonoxide 0.057 s, Elmer 0.053 s, openEMS 40 min; on the heater photonoxide 0.036 s, Elmer 0.070 s." width="100%">
</picture>

Each code on the same mesh, its error against photonoxide's converged result (the reference):

| Problem | Quantity | Reference | photonoxide | openEMS | Elmer |
|---|---|---:|---:|---:|---:|
| Coplanar line, 0.25 µm mesh | n | 2.217156 | 2.217052 (−0.0047 %) | 2.217069 (−0.0039 %) | 2.217161 (+0.00021 %) |
| | Z₀, Ω | 54.1476 | 54.5551 (+0.75 %) | 53.7113 (−0.81 %) | 53.9122 (−0.43 %) |
| | time | | 0.057 s | 40 min 8 s | 0.053 s |
| Heater over silicon, 0.0125 µm mesh | peak rise, K | 0.327422 | 0.327508 (+0.026 %) | — | 0.327384 (−0.012 %) |
| | time | | 0.036 s | — | 0.070 s |

- **Agreement:** refined to convergence, openEMS and Elmer meet photonoxide within 0.20 % and 0.047 % on Z₀, 0.0039 % and 0.00090 % on n, and Elmer within 0.0099 % on the heater's rise.
- **Speed:** photonoxide and Elmer solve the same 2D problems, so their times compare directly. Each is the code's own assembly and solve, the fastest of three runs. openEMS solves the line in 3D and in time (2.85 million cells for 339 345 steps), so its time is its own cost for that accuracy, not a like-for-like race.
- Every mesh and both codes' numbers on it: [docs/validation-openems.md](validation-openems.md) and [docs/validation-elmer.md](validation-elmer.md). The times are on an Intel Core Ultra 7 265K (20 threads), 64 GB, Windows 11.

## Problems

Each problem is drawn on the same mesh lines in every code: cells from the smallest at the
conductors' or the heater's edges, growing by 1.1 to the largest. Each code's result is
extrapolated from its three finest meshes and compared with photonoxide's converged one.

- **Ghione and Naldi's coplanar line** (G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984),
  doi:10.1049/el:19840120): strips of zero thickness, a 20 µm signal, 10 µm gaps, on 20 µm of
  ε = 10, air below and above, in a perfectly conducting box ±200 µm across and high.
  - photonoxide: the quasi-TEM line from the static solver's capacitances with and without the
    substrate, n = √(C/C₀) and Z₀ = 1/(c√(CC₀)).
  - Elmer: the same from StatElecSolver's capacitance matrix, on the same half box.
  - openEMS: a 4 mm section driven by a 5 to 55 GHz pulse until its energy is down by 70 dB; k
    and Z₀ from the voltages at three planes and the current midway, and n and Z₀ at 0 GHz from
    a fit over 10 to 50 GHz.
- **A heater over silicon:** 2 µm wide and 0.2 µm thick, 2 µm up in oxide (k = 1.38 W/(m·K)) on
  10 µm of silicon (k = 148 W/(m·K)) held at 0 underneath, the other sides insulated, 1 W/m.
  - photonoxide: the heat solver's peak rise.
  - Elmer: HeatSolver's, with the heater's power as a heat source.

The harness writes each program's input from its documentation: openEMS's XML from its concept
pages, Elmer's mesh files (`mesh.header`, `.nodes`, `.elements`, `.boundary`) and solver input
from the ElmerSolver manual.

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
PHOTONOXIDE_OPENEMS=C:/openEMS just openems
PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer
```

Each writes its report and redraws the chart. Every option, where the runs are kept, and what
each file of the harness does: [crosscode/README.md](../crosscode/README.md).

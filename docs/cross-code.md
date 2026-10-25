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
| The static solver: heat and electrostatics, 2D finite volumes | Elmer, 2D finite elements | the same equations on the same mesh lines | accuracy and speed |
| The quasi-TEM RF line, from the static solver | openEMS, 3D FDTD | Maxwell's equations in time, with no quasi-TEM approximation | accuracy only |

Every error is against photonoxide's converged result, in percent. Both codes take the same
mesh lines in each problem.

## photonoxide against Elmer

| Problem | Quantity | Reference | photonoxide | Elmer |
|---|---|---:|---:|---:|
| Coplanar line, 0.125 µm mesh | n | 2.217156 | 2.217159 (+0.00011 %) | 2.217175 (+0.00085 %) |
| | Z₀, Ω | 54.1476 | 54.3714 (+0.41 %) | 54.0150 (−0.24 %) |
| | time | | 0.073 s | 0.080 s |
| Heater over silicon, 0.0125 µm mesh | peak rise, K | 0.327422 | 0.327508 (+0.026 %) | 0.327384 (−0.012 %) |
| | time | | 0.036 s | 0.070 s |

- **Agreement:** refined to convergence, Elmer meets photonoxide within 0.00090 % on n, 0.047 % on Z₀ and 0.0099 % on the heater's rise.
- **Times:** each code's own assembly and solve on the same mesh, the fastest of three runs. Elmer's are read from its log, which gives the heat solver's to 0.01 s; photonoxide's are measured around its solve. The line takes two solves in each code, with the substrate and without it. They were taken on an Intel Core Ultra 7 265K (20 threads), 64 GB, Windows 11.
- Every mesh: [docs/validation-elmer.md](validation-elmer.md).

## photonoxide against openEMS

| Problem | Quantity | Reference | photonoxide | openEMS |
|---|---|---:|---:|---:|
| Coplanar line, 0.25 µm mesh | n | 2.217156 | 2.217052 (−0.0047 %) | 2.217069 (−0.0039 %) |
| | Z₀, Ω | 54.1476 | 54.5551 (+0.75 %) | 53.7113 (−0.81 %) |

- **Agreement:** refined to convergence, openEMS meets photonoxide within 0.0039 % on n and 0.20 % on Z₀. This checks the quasi-TEM approximation as well as the solver: openEMS makes no such approximation.
- **No times:** openEMS solves a 3D section of the line, 4 mm long, in time (2.85 million cells for 339 345 steps on the 0.25 µm mesh), where photonoxide solves the 2D cross-section once. Their times measure different problems.
- Every mesh: [docs/validation-openems.md](validation-openems.md).

## The problems

- **Ghione and Naldi's coplanar line:** strips of zero thickness, a 20 µm signal and 10 µm gaps, on 20 µm of ε = 10, in a perfectly conducting box ±200 µm across and high. Its index n and impedance Z₀.
  - photonoxide's quasi-TEM line: from the static solver's capacitances with the substrate and without it.
  - Elmer: the same capacitances, from StatElecSolver.
  - openEMS: a 4 mm section driven by a pulse, k and Z₀ from voltages and currents along it, at 0 GHz from a fit over 10 to 50 GHz.
- **A heater over silicon:** 2 × 0.2 µm, 2 µm up in oxide on 10 µm of silicon held at 0, dissipating 1 W/m. Its peak temperature rise, from photonoxide's heat solver and from Elmer's HeatSolver.

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

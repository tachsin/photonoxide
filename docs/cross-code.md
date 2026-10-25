# Cross-code references: other codes on photonoxide's cases

Meep and MPB are the references for photonoxide's optics; for the static and RF solvers of 0.6
they are openEMS (RF) and Elmer (heat and electrostatics): open codes run as external programs
on shared cases, their numbers compared with photonoxide's at equal grids or converged (ROADMAP
0.6, decision 6; issue #422). photonoxide needs none of them: CI and the released program never
run them, and the tests that do skip, saying so, where they aren't installed.

## Results at a glance

Each error is against photonoxide's converged result, the reference, in percent. photonoxide
"on the finest mesh" and the other codes are on the same mesh lines; openEMS and Elmer are
extrapolated from their three finest meshes.

| Case | Quantity | photonoxide, converged (the reference) | photonoxide, finest mesh | openEMS | Elmer |
|---|---|---:|---:|---:|---:|
| Ghione and Naldi's coplanar line | n | 2.217156 | 2.217052 (−0.0047 %) | 2.217069 (−0.0039 %) | 2.217176 (+0.00090 %) |
| the same line | Z₀, Ω | 54.1476 | 54.5551 (+0.75 %) | 54.0387 (−0.20 %) | 54.1221 (−0.047 %) |
| a heater over silicon, 1 W/m | peak rise, K | 0.327422 | 0.327508 (+0.026 %) | — | 0.327390 (−0.0099 %) |

photonoxide's finest-mesh row is on openEMS's finest mesh (0.25 µm) for the line and Elmer's
(0.0125 µm) for the heater.

### Times

Times are given only where two codes solve the same problem, so they show speed as well as
agreement: Elmer and photonoxide solve the same 2D problems on the same mesh lines. Each is the
code's own assembly and solve (Elmer's as its log reports them, photonoxide's measured around
its solve), with no program start or file reading and writing; on an Intel Core Ultra 7 265K
(20 threads), 64 GB, Windows 11:

| Case | Elmer, all meshes | photonoxide, the same meshes | Elmer, finest mesh | photonoxide, finest mesh |
|---|---:|---:|---:|---:|
| the coplanar line (5 meshes, two solves each) | 0.20 s | 0.21 s | 0.08 s | 0.08 s |
| the heater (5 meshes) | 0.13 s | 0.08 s | 0.07 s | 0.03 s |

Elmer's HeatSolve reports its times to 0.01 s. openEMS's results carry no times: it solves a
different problem, a 3D section of the line 4 mm long in time (2.85 million cells for 339 345
steps on its finest mesh), so it is in the study to check the numbers.

The two reports have every mesh, with each code's value and error on it (and Elmer's and photonoxide's times):
[docs/validation-openems.md](validation-openems.md) and
[docs/validation-elmer.md](validation-elmer.md). The licences and how to get the programs
follow.

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

## Getting openEMS

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

## Getting Elmer

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

## Running the comparison

**This is a local comparison, for validation: CI never runs it.** The harness is the crate
`photonoxide-crosscode` (`crosscode/`): a crate of its own, outside photonoxide's workspace (its
own `Cargo.lock`, its build in `crosscode/target/`), never published, never built by CI or by the
released program. It holds openEMS's input, its runs with their progress, its probes' files, and
the shared cases on photonoxide's side. From the repository's root:

```sh
PHOTONOXIDE_OPENEMS=C:/openEMS just openems   # the comparison, in release
PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer   # Elmer's
just crosscode                                # the harness's rustfmt, clippy and unit tests
```

`just elmer` writes [docs/validation-elmer.md](validation-elmer.md) the same way, from runs kept
in `crosscode/target/tmp/elmer/` (each with Elmer's output in `elmer.log`); its runs take seconds.

`just openems` writes [docs/validation-openems.md](validation-openems.md): each run's mesh,
steps, time and numbers, and photonoxide's on the same lines and converged; it fails when the
two codes disagree by more than the tolerances there. Each run logs its progress as it goes
(the step against those it should take, the energy against the end criterion, the time elapsed
and left) to stderr and to `progress.log` in its folder under `crosscode/target/tmp/openems/`;
openEMS's own output is beside it in `openems.log`, and a finished run of the same model by the
same openEMS is taken again rather than run anew. `PHOTONOXIDE_OPENEMS_THREADS` sets openEMS's
threads (8 by default). Without openEMS the test prints `skipped:` and why, and passes.

The main validation report ([docs/validation.md](validation.md)) doesn't depend on any of it.

## The cases

Ghione and Naldi's coplanar waveguide (a = 10, b = 20 µm, perfectly conducting strips of zero
thickness on 20 µm of ε = 10, air below) in a perfectly conducting box ±200 µm across and high,
on the same mesh lines in both codes (cells from the smallest at the strips' edges, growing by
1.1 to 10 µm), openEMS v0.37.0's n and Z₀ at 0 GHz (each run's fit of v₀ + a f² from 10 to 50
GHz, the energy down by 70 dB):

| Smallest cell | openEMS: n, Z₀ (Ω) | photonoxide, the same lines: n, Z₀ (Ω) |
|---:|---|---|
| 2 µm | 2.216808, 51.0569 | 2.215678, 57.1665 |
| 1 µm | 2.217125, 52.5512 | 2.216253, 55.7129 |
| 0.5 µm | 2.216707, 53.3408 | 2.217192, 54.9224 |
| 0.25 µm | 2.21707, 53.711 | 2.21705, 54.555 |

photonoxide converged (`QuasiTem` graded from 2.5 nm by 1.025): n 2.21716, Z₀ 54.148 Ω (54.145
extrapolated, order 1.16). Both codes' Z₀ converge at about first order in the smallest cell, from
opposite sides; Richardson's extrapolation of the three finest meshes gives openEMS 54.039 Ω
(order 1.09) and photonoxide on the same lines 54.236 Ω (order 1.11). **openEMS against
photonoxide converged: n −0.0039 %, Z₀ −0.20 %**, within the study's tolerances of
0.1 % and 0.5 %; openEMS's index is within 0.021 % of the converged one on every mesh. The 0.25 µm
mesh has 2.85 million cells and 339 345 steps of 4.0e-16 s. Every mesh, its cells and
steps, and n and Z₀ at 10 to 50 GHz are in [docs/validation-openems.md](validation-openems.md),
which `just openems` writes.

## Elmer's cases

`crosscode::elmer` writes Elmer's input from its documented formats (the ElmerSolver manual's
mesh files, `mesh.header`, `mesh.nodes`, `mesh.elements` and `mesh.boundary`, and its solver
input file): a structured mesh of four-node quadrilaterals on the mesh lines photonoxide's static
solver takes, so Elmer's nodes are photonoxide's cell faces, each cell a body by where its centre
is, and boundary elements where the case puts conditions. Elmer's bilinear finite elements
against photonoxide's cell-centred finite volumes, on the same lines:

- **Ghione and Naldi's coplanar waveguide, by electrostatics** (the openEMS case's line, half of
  it beside its mirror plane): StatElecSolver's capacitance of the signal with the substrate and
  without it, so n = √(C/C₀) and Z₀ = 1/(c√(CC₀)). On five meshes from 2 to 0.125 µm, Elmer
  extrapolated gives n 2.217176 and Z₀ 54.1221 Ω, against photonoxide's converged 2.217156 and
  54.1476 Ω: **+0.00090 % and −0.047 %** (tolerances 0.1 % and 0.5 %).
- **A heater over silicon, by heat conduction:** a heater 2 µm wide and 0.2 µm thick, 2 µm up in
  oxide (k = 1.38 W/(m·K)) on 10 µm of silicon (k = 148 W/(m·K)) held at 0 underneath, 1 W/m:
  HeatSolver's peak rise. On five meshes from 0.2 to 0.0125 µm, Elmer extrapolated gives
  0.327390 K against photonoxide's converged 0.327422 K: **−0.0099 %** (tolerance 0.5 %).

Every mesh, its cells and time, and both codes' numbers on the same lines are in
[docs/validation-elmer.md](validation-elmer.md).

## Running it yourself

The harness, how to run each study and what each of its files does: [crosscode/README.md](../crosscode/README.md).

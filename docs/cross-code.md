# Cross-code references: other codes on photonoxide's cases

Meep and MPB are the references for photonoxide's optics; for the static and RF solvers of 0.6
they are openEMS (RF) and, once a build is at hand, Elmer (heat and electrostatics): open codes
run as external programs on shared cases, their numbers compared with photonoxide's at equal
grids or converged (ROADMAP 0.6, decision 6; issue #422). photonoxide needs none of them: CI
and the released program never run them, and the tests that do skip, saying so, where they
aren't installed.

## Licences

| Program | Version run | Licence | What photonoxide does with it |
|---|---|---|---|
| [openEMS](https://openems.de) (3D FDTD) | v0.37.0 (the Windows package `openEMS_x64_v0.37.0_msvc`) | GPL-3.0 (its `COPYING`: GNU General Public License, version 3) | runs `openEMS` on an XML input it writes, reads the probes' files |
| CSXCAD (openEMS's geometry library, inside openEMS) | v0.7.0 | LGPL-3.0 (its `COPYING`: GNU Lesser General Public License, version 3) | none: openEMS reads the geometry |
| [Elmer](https://www.elmerfem.org) (finite elements: heat, electrostatics) | not yet run | GPL-2.0 (its `LICENSE.md`; LGPL-2.1 beside it in `license_texts/`, for some of its libraries), to be read in full before it is run | waiting for a binary build (below) |

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

## Running the comparison

**This is a local comparison, for validation: CI never runs it.** The harness is the crate
`photonoxide-crosscode` (`crosscode/`): a crate of its own, outside photonoxide's workspace (its
own `Cargo.lock`, its build in `crosscode/target/`), never published, never built by CI or by the
released program. It holds openEMS's input, its runs with their progress, its probes' files, and
the shared cases on photonoxide's side. From the repository's root:

```sh
PHOTONOXIDE_OPENEMS=C:/openEMS just openems   # the comparison, in release
just crosscode                                # the harness's rustfmt, clippy and unit tests
```

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
photonoxide converged: n −3.95e-5, Z₀ −2.01e-3 (relative)**, within the study's tolerances of
1e-3 and 5e-3; openEMS's index is within 4.6e-4 of the converged one on every mesh. The 0.25 µm
run took 2408 s on 8 threads (339 345 steps of 4.0e-16 s, 2.85 million cells). Every mesh, its
steps and time, and n and Z₀ at 10 to 50 GHz are in [docs/validation-openems.md](validation-openems.md),
which `just openems` writes.

## Elmer: waiting for a binary build

Elmer's cases (P1's heat conduction from a line source and a heater in a slab, the coplanar
waveguide's capacitance by electrostatics, P4's heater) wait for a build of ElmerSolver to run;
building it from source isn't part of this work, and its source isn't read. With a binary, the
harness gains `crosscode::elmer` the same way: found through `PHOTONOXIDE_ELMER` or `PATH`, its
input (a mesh and a `.sif` file) written from Elmer's documented formats, its results read.

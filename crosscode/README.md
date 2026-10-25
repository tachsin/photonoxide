# photonoxide-crosscode

Other codes, run as external programs on photonoxide's cases: openEMS (3D FDTD) for the RF line,
and Elmer (finite elements) for heat and electrostatics. This is the cross-code half of
photonoxide's validation, as Meep and MPB are for the optics.

**Everything here is local:** CI never builds or runs it, and the library and the released
program don't need it. It is a crate of its own, outside photonoxide's workspace, with its own
`Cargo.lock`. The tests that need a program skip, saying so, where it isn't installed.

## The results

- [docs/cross-code.md](../docs/cross-code.md): every case at a glance (each code's error in
  percent against photonoxide's converged result, and each code's time), the licences, and how to
  get the programs.
- [docs/validation-openems.md](../docs/validation-openems.md): openEMS, mesh by mesh.
- [docs/validation-elmer.md](../docs/validation-elmer.md): Elmer, mesh by mesh.

## Running it

From the repository's root:

```sh
# openEMS: the program or its folder (or openEMS on PATH)
PHOTONOXIDE_OPENEMS=C:/openEMS just openems

# Elmer: ElmerSolver's folder (or ElmerSolver on PATH), and its libraries' folders if they
# aren't on PATH (a Windows build against MSYS2's)
PHOTONOXIDE_ELMER=C:/elmer/bin PHOTONOXIDE_ELMER_PATH=C:/msys64/ucrt64/bin just elmer

# the crate's own checks: rustfmt, clippy and its unit tests (no program needed)
just crosscode
```

| Study | Writes | Time here (a Core Ultra 7 265K) |
|---|---|---|
| `just openems` | docs/validation-openems.md | about 1 h 13 min for its four meshes; a finished run is taken again, so a rerun takes seconds |
| `just elmer` | docs/validation-elmer.md | about 15 s |

`PHOTONOXIDE_OPENEMS_THREADS` sets openEMS's threads (8 by default). The runs are kept in
`crosscode/target/tmp/openems/` and `crosscode/target/tmp/elmer/`, each with the program's own
output beside it (`openems.log`, `elmer.log`); openEMS's also log their progress (step k/N, %,
elapsed and ETA) to `progress.log`.

## What's in it

| File | What it does |
|---|---|
| `src/openems.rs` | finds openEMS, writes its XML input from its documented format, runs it with its progress, reads its probes |
| `src/elmer.rs` | finds ElmerSolver, writes Elmer's mesh files and solver input (SIF) from its documented formats, runs it, reads its output |
| `src/cpw.rs` | the shared RF case: Ghione and Naldi's coplanar waveguide in a box, on the same mesh lines for openEMS, Elmer and photonoxide |
| `src/heater.rs` | the shared heat case: a heater in oxide over silicon, for Elmer and photonoxide |
| `src/line.rs` | a line's wavenumber and impedance from voltages and currents at planes along it |
| `src/report.rs` | errors in percent and times, as the reports print them |
| `tests/openems.rs`, `tests/elmer.rs` | the two studies, which write the reports |

## The rules

openEMS (GPL-3.0) and Elmer (GPL-2.0 or later) are run as external programs only. Nothing of
theirs is linked, built, shipped or read: the input files are written in Rust from the programs'
documentation, and only their outputs are read back. A new case goes in `src/` with both codes'
setup on the same mesh lines, and its study reports each code's value, its error in percent and
its time.

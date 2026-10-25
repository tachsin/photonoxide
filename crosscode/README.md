# photonoxide-crosscode

Other codes, run as external programs on photonoxide's cases: openEMS (3D FDTD) for the RF line,
and Elmer (finite elements) for heat and electrostatics. This is the cross-code half of
photonoxide's validation, as Meep and MPB are for the optics.

**It is a local comparison:** CI never builds or runs this crate, and the library and the
released program don't need it. It is a crate of its own, outside photonoxide's workspace, with
its own `Cargo.lock`. The tests that need a program skip, saying so, where it isn't installed.

## The results

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
[docs/validation-openems.md](../docs/validation-openems.md) and
[docs/validation-elmer.md](../docs/validation-elmer.md). The licences and how to get the programs
are in [docs/cross-code.md](../docs/cross-code.md).

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

`PHOTONOXIDE_MACHINE` names the computer in the reports (`Intel Core Ultra 7 265K (20 threads),
64 GB, Windows 11`); without it they give its thread count and system. `PHOTONOXIDE_OPENEMS_THREADS`
sets openEMS's threads (8 by default). The runs are kept in `crosscode/target/tmp/openems/` and
`crosscode/target/tmp/elmer/`, each with the program's own output beside it (`openems.log`,
`elmer.log`); openEMS's also log their progress (step k/N, %, elapsed and ETA) to
`progress.log`, and a finished openEMS run of the same model is taken again, so a rerun takes
seconds.

## What's in it

| File | What it does |
|---|---|
| `src/openems.rs` | finds openEMS, writes its XML input from its documented format, runs it with its progress, reads its probes |
| `src/elmer.rs` | finds ElmerSolver, writes Elmer's mesh files and solver input (SIF) from its documented formats, runs it, reads its output |
| `src/cpw.rs` | the shared RF case: Ghione and Naldi's coplanar waveguide in a box, on the same mesh lines for openEMS, Elmer and photonoxide |
| `src/heater.rs` | the shared heat case: a heater in oxide over silicon, for Elmer and photonoxide |
| `src/line.rs` | a line's wavenumber and impedance from voltages and currents at planes along it |
| `src/report.rs` | errors in percent, times, and the computer's name, as the reports print them |
| `tests/openems.rs`, `tests/elmer.rs` | the two studies, which write the reports |

## The rules

openEMS (GPL-3.0) and Elmer (GPL-2.0 or later) are run as external programs only. Nothing of
theirs is linked, built, shipped or read: the input files are written in Rust from the programs'
documentation, and only their outputs are read back. A new case goes in `src/` with both codes'
setup on the same mesh lines, and its study reports each code's value, its error in percent and
its time where the codes solve the same problem (and only there).

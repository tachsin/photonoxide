<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/banner.svg" alt="photonoxide: light meets Rust" width="100%">
</p>

[![Crates.io](https://img.shields.io/crates/v/photonoxide.svg)](https://crates.io/crates/photonoxide)
[![PyPI](https://img.shields.io/pypi/v/photonoxide.svg)](https://pypi.org/project/photonoxide/)
[![Docs.rs](https://img.shields.io/docsrs/photonoxide)](https://docs.rs/photonoxide)
[![CI](https://github.com/tachsin/photonoxide/actions/workflows/ci.yml/badge.svg)](https://github.com/tachsin/photonoxide/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/photonoxide)](Cargo.toml)
[![License](https://img.shields.io/crates/l/photonoxide.svg)](#license)
[![Validation](https://img.shields.io/badge/validation-report-ce422b)](docs/validation.md)
[![Examples](https://img.shields.io/badge/examples-checked_against_papers-ce422b)](examples/README.md)

**Light meets Rust: an open reference for photonic design, and the software that runs it.**

<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/hero.gif" alt="The photonoxide studio: a silicon strip's first mode travelling along it in 3D as the camera orbits; a ring resonator's transmission spectrum building up point by point, then the ring lit at resonance in 3D; a Mach-Zehnder interferometer wired on the chip and its spectrum appearing" width="100%">
</p>
<p align="center"><sub>The studio: a strip's mode travelling in 3D; a ring's spectrum by 2D FDFD building up live (sped up) and the ring lit at resonance; an MZI wired on the chip and simulated.</sub></p>

photonoxide is an open reference for photonic design and the software that puts it to use. Its
methods pages derive each solver from the papers it implements; its Academy, written lesson by
lesson, teaches devices and numerical methods at three depths with charts the library computes;
its fabrication flows show step by step how a design would be made, every stated process
capability with its source. A Rust library, a Python package and a desktop studio run the same
methods, each checked against an analytic solution or a published result in a
[validation report](docs/validation.md) that CI keeps current.

It is released milestone by milestone ([changelog](CHANGELOG.md), [roadmap](ROADMAP.md)); until
1.0 the API can change between milestones. The methods, examples and report are also at
[tachsin.gr/projects/photonoxide](https://tachsin.gr/projects/photonoxide).

## What it holds

- **[Methods](docs/README.md#methods):** 52 write-ups in 9 topics, from waveguide modes to
  fabrication, each setting out its method, its limits and the papers it implements; their
  front matter cites 238 distinct sources by DOI.
- **[Academy](academy/README.md):** lessons on devices and numerical methods at three depths,
  each opening on a drawing of its device, with charts the library computes live; 8 written,
  and 12 more laid out section by section, read on GitHub or in the studio.
- **[Fabrication](docs/methods/fabrication.md):** the process flow that would make a design, each
  step with its reason, its alternatives and what its process can achieve, as its source states
  it; 54 processes on 5 platforms, every flow executed back to its design and shown step by step
  in the studio. New platforms and exports are in progress.
- **[The library, the Python package and the studio](#install):** the code that runs a design,
  with 55 examples that each reproduce a paper, 34 job files ready to run, and 806 validation
  cases that check it.

## What it computes

- **Materials:** [dispersion models](docs/methods/materials.md) with their source, validity range
  and temperature; a [catalogue](docs/methods/catalogue.md) of Si, SiO₂, SiN, GaAs, AlGaAs,
  InGaP, InP, AlN, AlGaN, LiNbO₃ and MgO:LiNbO₃, BaTiO₃ and LiTaO₃, and the phase-change
  Sb₂Se₃, Sb₂S₃ and Ge₂Sb₂Te₅ in both states, with χ⁽²⁾ and Pockels tensors, transparency
  windows, and the Kerr coefficients of Si, SiN, LiNbO₃ and AlN.
- **Geometry for any device:** [shapes](docs/methods/geometry.md) kept exact, from paths with
  Euler bends to regions defined by [expressions](docs/methods/expressions.md);
  [booleans and offsets](docs/methods/booleans.md), [arrays and lattices](docs/methods/arrays.md),
  [components drawn with named ports](docs/methods/generators.md),
  [stacks with partial etches and sidewalls](docs/methods/stacks.md), and
  [3D bodies and graded media](docs/methods/bodies.md); each solver reads it by points or by
  [each cell's fill](docs/methods/averaging.md).
- **Modes:** [slabs](docs/methods/slab.md) and [multilayers](docs/methods/multilayer.md),
  [full-vector cross-sections](docs/methods/vector.md) with [PMLs](docs/methods/pml.md),
  [Hadley's](docs/methods/hadley.md) scheme for dielectric corners, [bends](docs/methods/bends.md)
  and [dispersion](docs/methods/dispersion.md); every mode in a region of the complex
  effective-index plane by [contour integrals](docs/methods/contour.md).
- **FDFD:** [2D](docs/methods/fdfd.md) and [3D](docs/methods/fdfd-3d.md), with
  [mode ports and S-parameters](docs/methods/fdfd-ports.md) and
  [adjoint gradients](docs/methods/fdfd-adjoint.md); photonoxide's own direct and iterative
  solvers, on every core ([baselines](docs/baselines.md)); sweeps sped up by
  [recycling](docs/methods/recycling.md), and farmed to worker processes on one computer or
  several ([farming](docs/farming.md)).
- **FDTD:** [2D and 3D](docs/methods/fdtd.md) with CPMLs, subpixel smoothing, mode sources,
  monitors, dispersive media and [adjoint gradients](docs/methods/fdtd-adjoint.md); a CPU kernel
  blocked in space and time, and an optional GPU one (wgpu).
- **Circuits:** [components](docs/methods/components.md), [netlists](docs/methods/circuits.md),
  the [circuit adjoint](docs/methods/circuit-adjoint.md), [compact models](docs/methods/compact.md)
  and Touchstone files; optimization with [genoxide](https://github.com/tachsin/genoxide).
- **Devices:** [components that know their geometry](docs/methods/devices.md): the strip and rib
  waveguides, bend, directional coupler, rings, MMIs, Y-branch and MZIs, each with its models
  from a closed form to 3D and their errors; computed at a fidelity, swept over one or two
  parameters and taken through a process's corners, as jobs.
- **Nonlinear optics:** [phase matching](docs/methods/nonlinear.md), quasi-phase matching and
  the acceptance bandwidths; second-harmonic, sum- and difference-frequency generation and
  photon pairs, from a waveguide's own modes; pulses by the nonlinear Schrödinger equation,
  four-wave mixing, and Kerr combs by the Lugiato–Lefever equation.
- **Fabrication flows:** [how a design would be made](docs/methods/fabrication.md), each step
  with its reason, its alternatives and what its process can achieve, with the paper that says
  so, and how accurately each dimension comes out; shown step by step in the studio.
- **External solvers, optional:** oneMKL's PARDISO, MUMPS, SuperLU, Apple Accelerate, NVIDIA
  cuDSS and Krylov solvers on NVIDIA GPUs, and any BLAS and LAPACK as the dense kernels of
  photonoxide's own solver, loaded at run time when installed and checked against
  photonoxide's own, which `auto` leaves only for one measured faster on the computer that runs
  it, at the same accuracy ([backends](docs/methods/backends.md), [install guides](docs/libraries.md)).
- **Jobs and runs:** YAML, TOML or JSON job files, checked before they run; every run is recorded
  and replays exactly.

[Getting started](docs/getting-started.md) · [API](https://docs.rs/photonoxide) ·
[Examples](examples/README.md) · [What works in Rust and Python](docs/features.md) · [From any language](docs/languages.md)

## The studio

`photonoxide` opens a window on a folder of jobs, runs and chips: the examples, each run in one
click and checked against its paper; each component on its own page, its design in 3D, its models
side by side, its runs, sweeps and corners; the Custom device for any other structure; runs played live or
replayed (modes, fields, spectra, FDTD as the field propagates); the Academy's lessons with live
charts; the materials catalogue; chips; the validation report; and the external
solvers installed. The same program runs a job from the command line
(`photonoxide run job.yaml`, `--headless` for batches). [studio/README.md](studio/README.md) has
every page and command.

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/device.gif" alt="A directional coupler's page: its gap widened and its coupled length changed with the sliders, the 3D view following each step, then its cross-section, drawn from above with the cut and across the two strips" width="100%"><br><sub><b>Components:</b> a directional coupler's gap and length moved, its 3D view following; then its cross-section.</sub></td>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/materials.gif" alt="The Materials page: lithium niobate's ordinary and extraordinary indices read off the plot, then AlGaAs's index as its aluminium fraction slider moves" width="100%"><br><sub><b>Materials:</b> LiNbO₃'s n<sub>o</sub> and n<sub>e</sub> read off the plot; AlGaAs as its aluminium fraction moves.</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/validation.gif" alt="The Validation page: two cases looked up and opened, showing what each computes and what it is checked against, with the math rendered" width="100%"><br><sub><b>Validation:</b> cases looked up and opened, each against its exact solution or paper.</sub></td>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/academy.gif" alt="The Academy: the ring resonator's lesson opened on its diagram, the ring and its bus drawn from above with R, w, g and the coupler's r and k, then turned in 3D; then its live chart, the radius and window set on a resonance at 1.55 µm, then the input coupling raised to a wide, shallow dip and lowered to critical coupling, where the through power reaches zero" width="100%"><br><sub><b>Academy:</b> the ring resonator's lesson opens on its device, drawn and in 3D; then its transmission follows the coupling as the slider moves, critical coupling at the dip's zero.</sub></td>
  </tr>
</table>

## Install

**Rust:**

```sh
cargo add photonoxide
```

Pure Rust, with no C, Fortran or Python dependencies.

**Python** 3.10 or later, wheels for Linux, macOS and Windows, with [uv](https://docs.astral.sh/uv/):

```sh
uv add photonoxide          # in a uv project
uv pip install photonoxide  # or into the active environment
```

`pip install photonoxide` works too: the guide is [docs/python.md](docs/python.md), the
[API reference](https://tachsin.gr/projects/photonoxide/docs/python/reference) has every function. Any other language runs the
`photonoxide` program on a job file and reads its results: [docs/languages.md](docs/languages.md).

**The studio:** the `photonoxide` program for Linux (x86_64 and ARM64), Windows and macOS is on
each [release](https://github.com/tachsin/photonoxide/releases), and updates itself
([downloads](studio/README.md#downloads), [channels](studio/README.md#channels)).

## A first example

220 nm of silicon in oxide at 1550 nm, each material with its published dispersion, and the
slab's modes solved exactly. In Rust:

```rust
use photonoxide::material::{silica, silicon};
use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

fn main() -> photonoxide::Result<()> {
    let wavelength = Wavelength::um(1.55)?;
    let si = silicon().refractive_index(wavelength)?.re; // Li 1980
    let oxide = silica().refractive_index(wavelength)?.re; // Malitson 1965
    let slab = Slab::new(oxide, si, oxide, Length::nm(220.0))?;
    for (name, polarization) in [("TE", Polarization::Te), ("TM", Polarization::Tm)] {
        for mode in slab.modes(polarization, wavelength) {
            println!("{name}{}: n_eff = {:.4}", mode.order(), mode.effective_index());
        }
    }
    Ok(())
}
```

In Python:

```python
import photonoxide as phox

si = phox.refractive_index("si", wavelength_um=1.55).real    # Li 1980
ox = phox.refractive_index("sio2", wavelength_um=1.55).real  # Malitson 1965
for polarization in ("te", "tm"):
    for mode in phox.slab_modes(below=ox, core=si, above=ox, thickness_um=0.22,
                                polarization=polarization, wavelength_um=1.55):
        print(f"{polarization.upper()}{mode.order}: n_eff = {mode.effective_index:.4f}")
```

Both print `TE0: n_eff = 2.8475` and `TM0: n_eff = 2.0531`. Each of the
[examples](examples/README.md) reproduces one paper's numbers and fails when it disagrees:

```sh
cargo run --release --example strip_waveguide   # or: just example strip_waveguide
```

Every example prints its setup first: the layers from the top down, the shapes with their
positions and sizes, the materials with their models and sources, the grid, the solver and the
tolerance.

## A design as a job file

A job file holds a whole design, every dimension in it, and runs the same in the studio, on the
command line and from Python. [docs/design.md](docs/design.md) walks through Bienstman et al.'s
leaky silicon wire and the designs people start from: a strip's modes, a ring's spectrum and a
directional coupler.

## Building from source

You need:

- **Rust**, the latest stable through [rustup](https://rustup.rs) (`rustup update`), 1.95 at
  least. Each Rust release brings its own LLVM, so a newer toolchain is also a newer code
  generator.
- **[just](https://just.systems)** for the commands below. It is optional: each recipe in the
  [justfile](justfile) is one plain command.
- **For the studio:** Node.js and pnpm, and on Linux WebKitGTK and Tauri's other
  [prerequisites](https://v2.tauri.app/start/prerequisites/) ([studio/README.md](studio/README.md#building)).
- **For the Python package:** [uv](https://docs.astral.sh/uv/).

```sh
git clone https://github.com/tachsin/photonoxide
cd photonoxide
just build         # the library, in release mode
just test          # the unit tests
just studio        # the program as released, target/dist/photonoxide
just python-test   # the Python package in python/.venv, and its tests
just               # every recipe
```

The optimized build profiles (`release`, `dist` and `fastest`) are described in
[CONTRIBUTING.md](CONTRIBUTING.md#build-profiles).

## Validation

Nothing is merged without an analytic test, a reproduction of a published result and a
convergence test; adjoint gradients are checked against finite differences. The
[report](docs/validation.md) has 806 cases: 526 against closed forms, 271 against published
results and 9 against another code on a shared problem. Each states its source, tolerance and
grid. CI fails when a case fails or the committed report isn't the one the code writes. CI
checks every example's output too.

## Status

| Milestone | Scope | Status |
|---|---|---|
| 0.1 – 0.3 | Units and materials, mode solvers, FDFD in 2D and 3D, the studio | ✅ released |
| 0.4 – 0.4.3 | Components and circuits, compact models, a 3D preconditioner, multifrontal direct solves | ✅ released |
| 0.5 FDTD | 2D and 3D, CPML, smoothing, sources, monitors, adjoints, a GPU kernel; solver backends loaded at run time | ✅ released |
| 0.5.1 Channels and backends | Nightly built from main, MUMPS, SuperLU and Accelerate, the backend chosen from benchmarks, every mode in a region | ✅ released |
| 0.5.2 Many solves at once | Ports as a block, recycling across sweeps, more contour-integral mode solvers, farming across processes | ✅ released |
| 0.5.3 QMR without the matrix | QMR without its matrix where the matrix wouldn't fit, MUMPS 5.9 | ✅ released |
| 0.5.4 Geometry and devices | Geometry for any device; the device library on pages of its own, with its models, runs, sweeps, corners and optimization; compact models; the chip built from devices; devices in Python | ✅ released |
| 0.6.0 Thermal and electro-optic, and writing jobs | Heat, thermo-optic and Pockels devices, travelling-wave electrodes, phase-change materials, the job editor | ✅ released |
| 0.6.1 Nonlinear integrated optics | Phase matching and QPM, photon pairs, the NLSE, four-wave mixing and Kerr combs, fabrication flows; open gaps in [#595](https://github.com/tachsin/photonoxide/issues/595) | ✅ released |
| 0.6.2 Quantum light | Linear-optical statistics, photon-pair sources, Gaussian states, loss budgets of quantum circuits | planned |
| 0.6.3 Eigenmode expansion and the mode solver's tools | On main: 2D eigenmode expansion, convergence to a tolerance, adaptive sweeps, effective area and confinement, sidewall-roughness loss. Left: 3D eigenmode expansion, beam and fibre coupling | 🚧 in progress |
| 0.7 Inverse design | Topology and shape optimization with fabrication constraints | planned |
| 0.7.1 – 0.16 | Distributed memory, carrier modulators, layout, RCWA and beam propagation, a device library, photonic crystals, plasmonics, nonlinear FDTD | planned |
| 1.0 | A stable API | planned |

[ROADMAP.md](ROADMAP.md) has the details, with what each item measured.

## Principles

- **Validated:** every number from a checked method, with its grid.
- **Cited:** every methods page and every written lesson names the papers it rests on, and every
  stated process capability its source, by DOI where the source has one.
- **Pure Rust:** libraries you install yourself are optional backends, loaded at run time and
  never required ([the plan](docs/plans/backends.md)).
- **Reproducible:** the same bits on any number of threads; every run replays.
- **Visible:** the studio and the command line run the same job.

## Contributing

Contributions other than code are as welcome as code: a methods page improved, a section of an
Academy lesson written, a paper suggested, a correction, a validation case.
[CONTRIBUTING.md](CONTRIBUTING.md) says what a pull request needs. Questions and ideas go to
[Discussions](https://github.com/tachsin/photonoxide/discussions); reports and proposals go to
the [issue forms](https://github.com/tachsin/photonoxide/issues/new/choose). Everyone taking part
follows the [code of conduct](CODE_OF_CONDUCT.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in photonoxide by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

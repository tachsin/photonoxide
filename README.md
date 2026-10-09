<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/banner.svg" alt="photonoxide: validated photonics for Rust" width="100%">
</p>

[![Crates.io](https://img.shields.io/crates/v/photonoxide.svg)](https://crates.io/crates/photonoxide)
[![Docs.rs](https://img.shields.io/docsrs/photonoxide)](https://docs.rs/photonoxide)
[![CI](https://github.com/tachsin/photonoxide/actions/workflows/ci.yml/badge.svg)](https://github.com/tachsin/photonoxide/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/photonoxide)](Cargo.toml)
[![License](https://img.shields.io/crates/l/photonoxide.svg)](#license)
[![Validation](https://img.shields.io/badge/validation-report-ce422b)](docs/validation.md)
[![Examples](https://img.shields.io/badge/examples-checked_against_papers-ce422b)](examples/README.md)

**Photonics for Rust: validated, and visible while it runs.**

<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/hero.gif" alt="The photonoxide studio: a silicon strip's first mode travelling along it in 3D as the camera orbits; a ring resonator's transmission spectrum building up point by point, then the ring lit at resonance in 3D; a Mach-Zehnder interferometer wired on the chip and its spectrum appearing" width="100%">
</p>
<p align="center"><sub>The studio: a strip's mode travelling in 3D; a ring's spectrum by 2D FDFD building up live (sped up) and the ring lit at resonance; an MZI wired on the chip and simulated.</sub></p>

photonoxide is a photonics library for Rust and a desktop studio to use it with. It solves
waveguide modes, FDFD and FDTD in 2D and 3D, and circuits of components, with adjoint gradients
throughout. Every method is checked against an analytic solution or a published result, in a
[validation report](docs/validation.md) that CI keeps current.

It is released milestone by milestone ([changelog](CHANGELOG.md), [roadmap](ROADMAP.md)); until
1.0 the API can change between milestones. The methods, examples and report are also at
[tachsin.gr/projects/photonoxide](https://tachsin.gr/projects/photonoxide).

## What it does

- **Materials:** [dispersion models](docs/methods/materials.md) with their source, validity range
  and temperature; a [catalogue](docs/methods/catalogue.md) of Si, SiO₂, SiN, LiNbO₃, GaAs,
  AlGaAs, InGaP, InP and AlN, with χ⁽²⁾ and Pockels tensors.
- **Modes:** [slabs](docs/methods/slab.md) and [multilayers](docs/methods/multilayer.md),
  [full-vector cross-sections](docs/methods/vector.md) with [PMLs](docs/methods/pml.md),
  [Hadley's](docs/methods/hadley.md) scheme for dielectric corners, [bends](docs/methods/bends.md)
  and [dispersion](docs/methods/dispersion.md).
- **FDFD:** [2D](docs/methods/fdfd.md) and [3D](docs/methods/fdfd-3d.md), with
  [mode ports and S-parameters](docs/methods/fdfd-ports.md) and
  [adjoint gradients](docs/methods/fdfd-adjoint.md); photonoxide's own direct and iterative
  solvers, on every core ([baselines](docs/baselines.md)).
- **FDTD:** [2D and 3D](docs/methods/fdtd.md) with CPMLs, subpixel smoothing, mode sources,
  monitors, dispersive media and [adjoint gradients](docs/methods/fdtd-adjoint.md); a CPU kernel
  blocked in space and time, and an optional GPU one (wgpu).
- **Circuits:** [components](docs/methods/components.md), [netlists](docs/methods/circuits.md),
  the [circuit adjoint](docs/methods/circuit-adjoint.md), [compact models](docs/methods/compact.md)
  and Touchstone files; optimization with [genoxide](https://github.com/tachsin/genoxide).
- **External solvers, optional:** oneMKL's PARDISO, NVIDIA cuDSS and Krylov solvers on NVIDIA
  GPUs, loaded at run time when installed and checked against photonoxide's own
  ([backends](docs/methods/backends.md)).
- **Jobs and runs:** TOML, JSON or YAML job files, checked before they run; every run is recorded
  and replays exactly.

[Getting started](docs/getting-started.md) · [API](https://docs.rs/photonoxide) ·
[Examples](examples/README.md)

## The studio

`photonoxide` opens a window on a folder of jobs, runs and chips: the examples, each run in one
click and checked against its paper; a job builder with a live 3D preview; runs played live or
replayed (modes, fields, spectra, FDTD as the field propagates); the Academy's lessons with live
charts; the materials catalogue; components and chips; the validation report; and the external
solvers this machine has. The same program runs a job from the command line
(`photonoxide run job.toml`, `--headless` for batches). [studio/README.md](studio/README.md) has
every page and command.

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/builder.gif" alt="The job builder: the ring-fdfd job's ring radius and position changed in the form, the 3D preview following each step" width="100%"><br><sub><b>Job builder:</b> the ring's radius and place edited in the form; the 3D preview follows.</sub></td>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/materials.gif" alt="The Materials page: lithium niobate's ordinary and extraordinary indices read off the plot, then AlGaAs's index as its aluminium fraction slider moves" width="100%"><br><sub><b>Materials:</b> LiNbO₃'s n<sub>o</sub> and n<sub>e</sub> read off the plot; AlGaAs as its aluminium fraction moves.</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/validation.gif" alt="The Validation page: two cases looked up and opened, showing what each computes and what it is checked against, with the math rendered" width="100%"><br><sub><b>Validation:</b> cases looked up and opened, each against its exact solution or paper.</sub></td>
    <td width="50%" valign="top"><img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/themes.gif" alt="Settings: themes picked one after another, then the 3D viewer in the chosen theme and in photonoxide's own light and dark" width="100%"><br><sub><b>Themes:</b> photonoxide's light and dark, or one of some thirty others; the 3D view follows.</sub></td>
  </tr>
</table>

## Install

```sh
cargo add photonoxide
```

Pure Rust, with no C, Fortran or Python dependencies. The program is on each
[release](https://github.com/tachsin/photonoxide/releases) for Linux (x86_64 and ARM64), Windows
and macOS, and updates itself ([downloads](studio/README.md#downloads)).

## A first example

220 nm of silicon in oxide at 1550 nm, each material with its published dispersion, and the
slab's modes solved exactly:

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

It prints `TE0: n_eff = 2.8475` and `TM0: n_eff = 2.0531`. Each of the
[examples](examples/README.md) reproduces one paper's numbers and fails when it disagrees:

```sh
cargo run --release --example strip_waveguide
```

## Validation

Nothing is merged without an analytic test, a reproduction of a published result and a
convergence test; adjoint gradients are checked against finite differences. Each case in the
[report](docs/validation.md) states its source, tolerance and grid, and CI fails when a case fails
or the committed report isn't the one the code writes. CI checks every example's output too.

## Status

| Milestone | Scope | Status |
|---|---|---|
| 0.1 – 0.3 | Units and materials, mode solvers, FDFD in 2D and 3D, the studio | ✅ released |
| 0.4 – 0.4.3 | Components and circuits, compact models, a 3D preconditioner, multifrontal direct solves | ✅ released |
| 0.5 FDTD | 2D and 3D, CPML, smoothing, sources, monitors, adjoints, a GPU kernel; solver backends loaded at run time | ✅ released |
| 0.5.1 Many solves at once | Ports as a block, recycling across sweeps, contour-integral mode solvers, farming across processes | 🚧 next |
| 0.6 Thermal and electro-optic | Heat, thermo-optic phase shifters, Pockels modulators, travelling-wave electrodes | planned |
| 0.6.1 Nonlinear integrated optics | Transparency windows, phase matching, SHG, SFG, DFG, SPDC and FWM in waveguides, Kerr combs | planned |
| 0.6.2 Quantum light | Linear-optical statistics, photon-pair sources, Gaussian states, loss budgets of quantum circuits | planned |
| 0.7 Inverse design | Topology and shape optimization with fabrication constraints | planned |
| 0.7.1 – 0.16 | Distributed memory, carrier modulators, layout, semi-analytic methods, a device library, photonic crystals, plasmonics, nonlinear FDTD | planned |
| 1.0 | A stable API | planned |

[ROADMAP.md](ROADMAP.md) has the details, with what each item measured.

## Principles

- **Validated:** every number from a checked method, with its grid.
- **Pure Rust:** libraries you install yourself are optional backends, loaded at run time and
  never required ([the plan](docs/plans/backends.md)).
- **Reproducible:** the same bits on any number of threads; every run replays.
- **Visible:** the studio and the command line run the same job.

## Contributing

Ideas, use cases and validation cases are as welcome as code: open an
[issue](https://github.com/tachsin/photonoxide/issues). [CONTRIBUTING.md](CONTRIBUTING.md) says
what a pull request needs.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in photonoxide by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

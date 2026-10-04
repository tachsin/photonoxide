<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/banner.svg" alt="photonoxide: validated, fabrication-ready photonics for Rust" width="100%">
</p>

[![Crates.io](https://img.shields.io/crates/v/photonoxide.svg)](https://crates.io/crates/photonoxide)
[![Docs.rs](https://img.shields.io/docsrs/photonoxide)](https://docs.rs/photonoxide)
[![CI](https://github.com/tachsin/photonoxide/actions/workflows/ci.yml/badge.svg)](https://github.com/tachsin/photonoxide/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/photonoxide)](Cargo.toml)
[![License](https://img.shields.io/crates/l/photonoxide.svg)](#license)
[![Validation](https://img.shields.io/badge/validation-report-ce422b)](docs/validation.md)
[![Examples](https://img.shields.io/badge/examples-checked_against_papers-ce422b)](examples/README.md)

**Photonics for Rust: validated, fabrication-ready, and visible while it runs.**

<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/studio/hero.gif" alt="The photonoxide studio: a silicon strip's first mode travelling along it in 3D as the camera orbits; a ring resonator's transmission spectrum building up point by point, then the ring lit at resonance in 3D; a Mach-Zehnder interferometer wired on the chip and its spectrum appearing" width="100%">
</p>
<p align="center"><sub>The studio: a strip's mode travelling in 3D; a ring's spectrum by 2D FDFD building up live (sped up) and the ring lit at resonance; an MZI wired on the chip and simulated.</sub></p>

photonoxide is a photonics library for Rust and a program to use it with. The library computes
waveguide modes, fields and S-parameters by frequency-domain finite differences, and connects
components into circuits that it simulates, differentiates and optimizes. Every method is
checked against an analytic solution or a published result, and the checks are collected in a
[validation report](docs/validation.md) that CI keeps current. The program, `photonoxide`, is a
studio: a desktop window where you build jobs and chips, watch them run in 3D and 2D, and read
the report. The plan goes on to FDTD, thermal and electro-optic modulators, inverse design,
layout and tape-out; see the [roadmap](ROADMAP.md).

> **Alpha.** The latest release is **0.4.0**: materials, mode solvers and FDFD, components and
> circuits, compact models, and the studio. Next is 0.4.1, a stronger preconditioner for
> high-contrast 3D FDFD. The API will change between milestones.

The project's pages are at [tachsin.gr/projects/photonoxide](https://tachsin.gr/projects/photonoxide):
the methods, the examples with their output, the validation report and the roadmap.

## What you can do today

In the 0.4.0 release:

- **Materials with provenance:** Sellmeier, Cauchy, Drude and Lorentz models with their source,
  validity range and temperature, and refractiveindex.info files read with theirs
  ([Materials](docs/methods/materials.md)). A [catalogue](docs/methods/catalogue.md) of silica,
  silicon, silicon nitride, lithium niobate (congruent, MgO-doped and thin film), GaAs, AlGaAs,
  InGaP, InP and AlN: index models, crystal symmetry, χ⁽²⁾ and Pockels tensors, every number
  read from its paper.
- **Slab modes:** the [three-layer slab](docs/methods/slab.md) exactly, any
  [multilayer](docs/methods/multilayer.md) by transfer matrices (bound modes, leaky waves and
  plane-wave reflection), and [planar profiles](docs/methods/slab-fd.md) by finite differences.
- **Waveguide cross-sections:** [full-vector modes](docs/methods/vector.md) by finite
  differences, with a [PML](docs/methods/pml.md) for leaky modes and
  [mirror walls](docs/methods/walls.md); [Hadley's equations](docs/methods/hadley.md) for
  second-order accuracy at dielectric corners; [bends](docs/methods/bends.md);
  the [effective index method](docs/methods/eim.md) and
  [Marcatili's approximation](docs/methods/marcatili.md), each with its error stated; group
  index, [dispersion and loss](docs/methods/dispersion.md); [fields, power and
  coupling](docs/methods/fields.md).
- **FDFD:** [2D](docs/methods/fdfd.md) and [3D](docs/methods/fdfd-3d.md) on Yee's grid with
  stretched-coordinate PMLs and the scheme's exact power flux; in 2D, [mode ports and
  S-parameters](docs/methods/fdfd-ports.md) and [adjoint gradients](docs/methods/fdfd-adjoint.md);
  sparse direct solves (faer) and QMR, preconditioned by ILU(0) in 3D.
- **3D FDFD ports:** the grid's own full-vector port modes, one-way sources and a reciprocal
  S-matrix.
- **Circuits:** [components](docs/methods/components.md) with ports, parameters, a fidelity and
  their error against their source; [netlists](docs/methods/circuits.md) solved as one sparse
  system, checked against Filipsson's sub-network growth; nested circuits; the
  [circuit adjoint](docs/methods/circuit-adjoint.md), every parameter's gradient from one
  transposed solve; optimization by [genoxide](https://github.com/tachsin/genoxide)'s L-BFGS-B,
  Adam and CMA-ES (in the [examples](examples/README.md)).
- **The first components:** waveguide and bend from the mode solvers, phase shifter,
  directional coupler, 1 × 2 and 2 × 2 MMIs, Y-branch, all-pass and add-drop rings, MZI, and a
  2D FDFD job's sampled S-matrix.
- **Compact models and Touchstone:** [vector fitting](docs/methods/compact.md) with its fit
  error, stability and passivity; models over parameters, polynomial or piecewise linear with an
  exact test of uniform stability; Touchstone 1.1 and 2.0 files read and written, as the
  measured fidelity.
- **More materials:** AlN's index (Rigler 2015), AlGaN films (Rigler 2013), and InGaP beyond
  Tanaka's range (Ferrini 2002).
- **Validation against measurement:** Dwivedi et al. 2015's Mach-Zehnder interferometers,
  predicted from their wires' measured cross-sections.
- **Jobs and runs:** a job is a TOML file (`modes`, `fdfd` or `structure`); every run is
  recorded as events and replays exactly.

Next, in 0.4.1: a stronger preconditioner for high-contrast 3D FDFD (multigrid that copes with
PMLs, or a sweeping one), so a component's 3D fidelity takes minutes.

[Getting started](docs/getting-started.md) goes from `cargo add` to a strip waveguide's modes.
The API is on [docs.rs](https://docs.rs/photonoxide).

## The studio

`photonoxide` with no arguments opens the studio, a window on a workspace folder of jobs, runs
and chips:

- **Examples:** the simulations of [`jobs/`](jobs) and the published results of
  [`examples/`](examples/README.md) built in, each run in one click and checked against its paper
  as it prints.
- **Job builder:** a form for each kind of job, the device previewed in 3D as you type, the TOML
  beside it, and the library checking the job as it changes.
- **Runs, Viewer and Compare:** runs played live or replayed; in 3D, the layers as solids and
  the selected mode travelling along its guide as a volume; in 2D, fields, modes, S-parameters
  and spectra, with a slider through a sweep's points; several runs' sweeps and spectra on
  shared axes.
- **Materials:** the catalogue, each model plotted over its range with its equation, its
  coefficients and tensors, and its papers.
- **Validation:** the release's report, with its math rendered, and the same report run on your
  machine.
- **Settings:** the theme, photonoxide's own or one of some thirty others, the workspace folder, tips and updates.
- **Components:** the component library, each kind's S-parameters recomputed as its
  parameters move, and Touchstone import and export.
- **Chip:** components placed, wired port to port, checked and simulated.

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

The same program runs a job (`photonoxide run job.toml`, live in the window or `--headless`),
replays a run (`photonoxide view runs/<run>`), runs a built-in example and checks the report.
[studio/README.md](studio/README.md) describes every page and command, and how to build it.

## Install

The library:

```sh
cargo add photonoxide
```

It is pure Rust, with no C, Fortran or Python dependencies, so `cargo build` is all it needs.
For the work on the main branch:
`photonoxide = { git = "https://github.com/tachsin/photonoxide" }`.

The program: each [release](https://github.com/tachsin/photonoxide/releases) has it for
Linux x86_64 and ARM64 (`.AppImage`, `.deb`, `.rpm`; the AppImage runs on clusters too),
Windows x86_64 (a per-user installer that needs no administrator rights) and macOS (one
universal `.dmg` for Apple Silicon and Intel). An installed copy looks for a new release when it
opens and every hour, and updates itself with one click after checking the release's
signature; a build from source doesn't update itself. The
[downloads table](studio/README.md#downloads) has the details for each platform, clusters
included.

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

It prints `TE0: n_eff = 2.8475` and `TM0: n_eff = 2.0531`. The
[examples](examples/README.md) go further, each reproducing one published result and failing
when it disagrees with the paper:

```sh
cargo run --release --example strip_waveguide
```

## Validation

Nothing is merged without an analytic test, a reproduction of a published result and a
convergence test; adjoint gradients are checked against finite differences.
The [validation report](docs/validation.md) has 143 cases, all passing:

- **70 analytic:** closed forms and exact properties, such as the exact slab, Fresnel
  reflection, reciprocity, energy conservation, a ring's free spectral range, and the adjoints
  against finite differences.
- **72 published:** results reproduced from papers and books, such as Li's and Malitson's
  tables, Hadley's corner problems, the leaky photonic-wire benchmark, Bogaerts's rings and
  Gustavsen and Semlyen's vector-fitting test. Six of them are measurements: the effective and
  group indices of three silicon wires from Dwivedi et al.'s Mach-Zehnder interferometers
  (2015), predicted from the wires' measured cross-sections and within the paper's own
  fabrication estimate.
- **1 cross-code:** the 3D FDFD port mode against the mode solver. Comparisons with Meep, MPB,
  S4 and Ceviche come with the later milestones.

Each case states its source, tolerance and grid. The report is written by
`photonoxide validate`; CI fails when a case fails or when the committed report differs from the
one the code writes. The [19 examples](examples/README.md) each reproduce one paper's numbers,
and CI checks their output too.

## Status

| Milestone | Scope | Status |
|---|---|---|
| 0.1 Foundations | Units, materials with provenance, geometry, run records, the studio's first window, the validation harness | ✅ released |
| 0.2 Mode solvers | Slab, multilayer, full-vector 2D finite differences, EIM, bends, dispersion | ✅ released |
| 0.3 FDFD | 2D and 3D, mode ports, S-parameters, adjoints, an iterative 3D solver; Hadley's high-accuracy mode solver | ✅ released |
| 0.3.1 – 0.3.3 The studio and materials | The studio as a workspace (examples inside, the job builder, run comparison, updates by one click); rings and 3D previews; the travelling mode in 3D, sweeps in the viewer, the report's math, every theme; the materials catalogue | ✅ released |
| 0.4 Components and circuits | Components at several fidelities, netlists, the circuit adjoint, optimization through genoxide, compact models, Touchstone, 3D FDFD ports; the studio's component library and chip view | ✅ released |
| 0.4.1 A 3D preconditioner | A stronger preconditioner for high-contrast 3D FDFD (0.4.0 has ILU(0)), so a component's 3D fidelity takes minutes | 🚧 next |
| 0.5 FDTD | 2D and 3D Yee, CPML, subpixel smoothing, dispersive media, GPU | planned |
| 0.6 Thermal and electro-optic | Heat and electrostatics, thermo-optic phase shifters, Pockels modulators (thin-film lithium niobate first), travelling-wave electrodes | planned |
| 0.7 Inverse design | Adjoint topology and shape optimization, fabrication constraints, the 2D-to-3D pipeline, device and circuit co-design | planned |
| 0.8 Carrier modulators and signals | Drift-diffusion, plasma-dispersion modulators, time-domain circuits and eye diagrams, programmable meshes | planned |
| 0.9 Layout and PDK | GDSII and OASIS, parametric cells, routing, DRC, SiEPIC EBeam and Cornerstone | planned |
| 0.10 Tape-out | Submission packages, test structures, sign-off, openEBL and Cornerstone runs, measurements back | planned |
| 0.11 Fabrication realism | Process variation, lithography proxies, corners, yield, circuit variability | planned |
| 0.12 Semi-analytic | RCWA, eigenmode expansion, BPM | planned |
| 0.13 Device library | Validated devices, each a component at several fidelities | planned |
| 0.14 – 0.16 | Photonic crystals, metasurfaces, plasmonics, nonlinear and fiber optics, Kerr microcombs, quantum | planned |
| 1.0 | Stable API and the published validation report | planned |

The details, with what each item measured, are in [ROADMAP.md](ROADMAP.md); what each release
changed is in [CHANGELOG.md](CHANGELOG.md).

## Why photonoxide?

Open-source photonics has excellent individual tools, each in its own corner:

- Meep for FDTD, MPB for band structures and S4 for RCWA;
- Ceviche and SPINS for inverse design;
- KLayout for layout.

Rust has oxiphoton, which is broad but has no GUI, and its README shows no comparison with published results. photonoxide aims to be one coherent toolkit with three things none of them combine:

- **Validated:** every solver checked against analytic solutions, published devices and established codes, with the results in a public report. Every reported number carries its convergence.
- **Fabricable:** foundry design rules inside the optimization, and a tape-out package ready for a multi-project wafer run: GDSII or OASIS on the foundry's layers, DRC and connectivity checked, test structures included, and performance reported across process variation. The first target is SiEPIC openEBL, where photonoxide designs will be fabricated and measured.
- **Visible:** a studio that shows modes, fields, spectra and, later, layouts and optimizations as they run. The command line and the studio run the same job, and every run replays.

And underneath:

- **Pure Rust:** no C, Fortran or Python dependencies, from the linear algebra to the GDS writer. There are no Python bindings.
- **Fast:** parallel on the CPU, with a GPU backend planned for FDTD.
- **Reproducible:** the same input gives the same result on any number of threads.
- **Inverse design built in:** adjoint gradients for every solver, and the optimizers from [genoxide](https://github.com/tachsin/genoxide), our optimization library, which grows the general methods photonoxide needs.

## Contributing

photonoxide is pre-1.0, which is the best time to shape it. Ideas, use cases and validation
cases you'd like to see are as welcome as code: open an
[issue](https://github.com/tachsin/photonoxide/issues). [CONTRIBUTING.md](CONTRIBUTING.md) says
what a pull request needs.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in photonoxide by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

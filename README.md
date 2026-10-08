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

photonoxide is a photonics library for Rust and a program to use it with. The library computes
waveguide modes, fields and S-parameters by frequency-domain finite differences, and connects
components into circuits that it simulates, differentiates and optimizes. Every method is
checked against an analytic solution or a published result, and the checks are collected in a
[validation report](docs/validation.md) that CI keeps current. The program, `photonoxide`, is a
studio: a desktop window where you build jobs and chips, watch them run in 3D and 2D, and read
the report. The plan goes on to FDTD, thermal and electro-optic modulators, inverse design,
layout export; see the [roadmap](ROADMAP.md).

> **Released milestone by milestone** (the latest version is on the crates.io badge above, and
> what each release changed is in [CHANGELOG.md](CHANGELOG.md)): materials, mode solvers, FDFD in
> 2D and 3D with fast direct and iterative solvers, components and circuits, compact models, and
> the studio. Next is FDTD. Until 1.0 the API can change between milestones.

The project's pages are at [tachsin.gr/projects/photonoxide](https://tachsin.gr/projects/photonoxide):
the methods, the examples with their output, the validation report and the roadmap.

## What you can do today

In the released versions (the latest on the crates.io badge above):

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
  S-parameters](docs/methods/fdfd-ports.md) and [adjoint gradients](docs/methods/fdfd-adjoint.md).
- **Fast solves on the whole CPU:** photonoxide's own multifrontal LU and complex symmetric
  L D Lᵀ, ordered by nested dissection, at PARDISO's fill and 2 to 9 times faster than faer's LU
  ([baselines](docs/baselines.md)); QMR, plain or with ILU(0), and its complex symmetric form at
  twice the speed; GMRES preconditioned by multigrid for high-contrast 3D problems; sweeps solved
  side by side; the same bits on any number of threads.
- **Benchmarks:** `photonoxide bench` times fixed problems at a stated accuracy, with the memory
  bandwidth each solve reaches against the machine's ([benchmarks](docs/benchmarks.md)).
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
- **Jobs and runs:** a job is a TOML, JSON or YAML file (`modes`, `fdfd` or `structure`), with
  light along x in every kind; the check refuses what the run would refuse, before it starts;
  every run is recorded as events and replays exactly.

Next is 0.5: [FDTD](docs/methods/fdtd.md) in 2D and 3D (Yee's scheme, the convolutional PML,
subpixel smoothing, dispersive media, sources and monitors, and
[adjoint gradients](docs/methods/fdtd-adjoint.md) in 3D), and solver backends loaded at run
time when installed (oneMKL's PARDISO, NVIDIA cuDSS, and QMR on NVIDIA GPUs), each checked
against photonoxide's own solvers, as the [backends plan](docs/plans/backends.md) sets out.

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
| 0.4.1 Polish | Light along x in every job, µm/nm, live sweeps, every page at 960 × 600, a job check that matches the run, bends at any radius, crystal tags and five catalogue gaps filled | ✅ released |
| 0.4.2 A 3D preconditioner and the whole CPU | A stronger preconditioner for high-contrast 3D FDFD (multigrid), and the CPU's cores and bandwidth put to use | ✅ released |
| 0.4.3 Direct solves at PARDISO's fill | A multifrontal LU and L D Lᵀ, 2 to 9 times faster than faer's LU | ✅ released |
| 0.5 FDTD | 2D and 3D Yee, CPML, subpixel smoothing, dispersive media, sources and monitors, a GPU backend (wgpu); solver backends loaded at run time (PARDISO, cuDSS, GPU QMR) | 🚧 next |
| 0.5.1 Many solves at once | Block solves for ports, recycling across sweeps, contour-integral mode solvers, farming across processes | planned |
| 0.6 Thermal and electro-optic | Heat and electrostatics, thermo-optic phase shifters, Pockels modulators (thin-film lithium niobate first), travelling-wave electrodes | planned |
| 0.6.1 Nonlinear integrated optics | Transparency windows, phase matching (birefringent, QPM, modal), SHG, SFG, DFG, OPA, SPDC and FWM in waveguides, photon pairs, Kerr combs; on LiNbO₃, GaAs, AlGaAs, AlN, SiN and Si | planned |
| 0.7 Inverse design | Adjoint topology and shape optimization, fabrication constraints, the 2D-to-3D pipeline, device and circuit co-design | planned |
| 0.7.1 Distributed memory | Domain decomposition across processes and machines, results independent of their number | planned |
| 0.8 Carrier modulators and signals | Drift-diffusion, plasma-dispersion modulators, time-domain circuits and eye diagrams, programmable meshes | planned |
| 0.9 Layout and PDK | GDSII and OASIS, parametric cells, routing, DRC on rules the user loads | planned |
| 0.10 Fabrication data | Layout packages, test structures, design checks and a design record; measured spectra imported and compared | planned |
| 0.11 Fabrication realism | Process variation, lithography proxies, corners, yield, circuit variability | planned |
| 0.12 Semi-analytic | RCWA, eigenmode expansion, BPM | planned |
| 0.13 Device library | Validated devices, each a component at several fidelities | planned |
| 0.14 – 0.16 | Photonic crystals, metasurfaces, plasmonics, nonlinear FDTD and fiber optics, quantum | planned |
| 1.0 | Stable API and the published validation report | planned |

The details, with what each item measured, are in [ROADMAP.md](ROADMAP.md); what each release
changed is in [CHANGELOG.md](CHANGELOG.md).

## Why photonoxide?

Open-source photonics has excellent tools, each focused on its task: Meep for FDTD, MPB for band
structures, S4 for RCWA, Ceviche and SPINS for inverse design, KLayout for layout. photonoxide
brings these tasks together in one toolkit, built around three things:

- **Validated:** every solver checked against analytic solutions, published devices and established codes, with the results in a public report. Every reported number carries its convergence.
- **Fabrication-aware:** design rules inside the optimization, layouts exported as GDSII or OASIS with their design and connectivity checks and test structures, and performance reported across process variation.
- **Visible:** a studio that shows modes, fields, spectra and, later, layouts and optimizations as they run. The command line and the studio run the same job, and every run replays.

And underneath:

- **Pure Rust:** no C, Fortran or Python dependencies, from the linear algebra to the GDS writer. There are no Python bindings. Libraries you install yourself (Intel MKL, MUMPS, SuperLU, your CPU or GPU vendor's math libraries) can be used as optional backends where they help, loaded at run time and never required ([the plan](docs/plans/backends.md)).
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

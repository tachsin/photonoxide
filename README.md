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

Mode solvers, FDFD, FDTD, semi-analytic methods, inverse design, layout and PDKs, in one library, with a studio to watch every simulation and optimization live.

> **🚧 Alpha.** 0.2 Mode solvers is released: exact slabs and multilayers, full-vector 2D modes
> with PML, bends, the effective index method, dispersion, fields and coupling, and the studio's
> mode viewer, each checked against published results ([validation report](docs/validation.md),
> [examples](examples/README.md)). FDFD comes next; see the [roadmap](ROADMAP.md), and share your
> ideas in the issues.

```sh
cargo add photonoxide                               # the library
```

The `photonoxide` program runs jobs live in the studio window (3D and 2D views) or headless, and
replays runs; it is built from this repository, see [studio/README.md](studio/README.md).

## Why photonoxide?

Open-source photonics has excellent individual tools, each in its own corner:

- Meep for FDTD, MPB for band structures and S4 for RCWA;
- Ceviche and SPINS for inverse design;
- KLayout for layout.

Rust has oxiphoton, which is broad but has no GUI, and its README shows no comparison with published results. photonoxide aims to be one coherent toolkit with three things none of them combine:

- **Validated:** every solver checked against analytic solutions, published devices and established codes, with the results in a public report. Every reported number carries its convergence.
- **Fabricable:** foundry design rules inside the optimization, and a tape-out package ready for a multi-project wafer run: GDSII or OASIS on the foundry's layers, DRC and connectivity checked, test structures included, and performance reported across process variation. The first target is SiEPIC openEBL, where photonoxide designs will be fabricated and measured.
- **Visible:** a studio that shows fields propagating, modes, layouts and optimizations as they run. The CLI and the studio run the same job, and every run replays.

And underneath:

- **Pure Rust:** no C, Fortran or Python dependencies, from the linear algebra to the GDS writer. There are no Python bindings.
- **Fast:** parallel on the CPU, with a GPU backend for FDTD.
- **Reproducible:** the same input gives the same result on any number of threads.
- **Inverse design built in:** adjoint gradients for every solver, density and level-set methods, robust and length-scale constraints. The optimizers come from [genoxide](https://github.com/tachsin/genoxide), our optimization library, which grows the general methods photonoxide needs.

## Status

| Milestone | Scope | Status |
|---|---|---|
| 0.1 Foundations | Units, materials with provenance, geometry, run records, studio skeleton, validation harness | ✅ released |
| 0.2 Mode solvers | Slab, full-vector 2D finite differences, EIM, bends, dispersion | ✅ released |
| 0.3 FDFD | 2D and 3D, mode ports, S-parameters, adjoints | 🔜 next |
| 0.4 FDTD | 2D and 3D Yee, CPML, subpixel smoothing, dispersive media, GPU | planned |
| 0.5 Semi-analytic | TMM, RCWA, eigenmode expansion, BPM | planned |
| 0.6 Inverse design | Adjoint topology and shape optimization, fabrication constraints, the 2D-to-3D pipeline | planned |
| 0.7 Layout and PDK | GDSII and OASIS, parametric cells, routing, DRC, SiEPIC EBeam and Cornerstone | planned |
| 0.8 Tape-out | Submission packages, test structures, sign-off, openEBL and Cornerstone runs, measurements back | planned |
| 0.9 Fabrication realism | Process variation, lithography proxies, corners, yield | planned |
| 0.10 Circuits and devices | S-parameter circuits, compact models, a validated device library | planned |
| 0.11 – 0.13 | Photonic crystals, metasurfaces, plasmonics, nonlinear and fiber optics, multiphysics, quantum | planned |
| 1.0 | Stable API and the published validation report | planned |

The details are in [ROADMAP.md](ROADMAP.md).

## Validation

Every solver is validated on three tiers before it ships:

- **Analytic solutions:** Fresnel, slab modes, Mie scattering, Bragg stacks, PML theory.
- **Cross-code:** Meep, MPB, S4, Ceviche and oxiphoton on the same structures.
- **Published devices:** inverse-designed demultiplexers, beamsplitters and grating couplers, reproduced in 3D.

Each result states its tolerance, grid and source. The current report is [docs/validation.md](docs/validation.md), written by `photonoxide validate` and checked by CI.

The [examples](examples/README.md) each reproduce one published result, from silicon's refractive index (Li 1980) to the modes of a silicon strip waveguide (Chrostowski & Hochberg 2015), and fail when they disagree with the paper:

```sh
cargo run --release --example strip_waveguide
```

## Contributing

photonoxide is at the design stage, which is the best time to shape it. Open an issue for ideas, use cases or validation cases you'd like to see.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in photonoxide by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

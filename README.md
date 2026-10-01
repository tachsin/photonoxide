# photonoxide

**Photonics for Rust: validated, fabrication-ready, and visible while it runs.**

Mode solvers, FDFD, FDTD, semi-analytic methods, inverse design, layout and PDKs, in one library, with a studio to watch every simulation and optimization live.

> **🚧 Pre-alpha.** There is nothing to install yet. We're designing it in the open.
> See the [roadmap](ROADMAP.md) and share your ideas in the issues.

## Why photonoxide?

Open-source photonics has excellent individual tools, each in its own corner:

- Meep for FDTD, MPB for band structures and S4 for RCWA;
- Ceviche and SPINS for inverse design;
- gdsfactory and KLayout for layout.

Rust has oxiphoton, which is broad but has no GUI, and its README shows no comparison with published results. photonoxide aims to be one coherent toolkit with three things none of them combine:

- **Validated:** every solver checked against analytic solutions, published devices and established codes, with the results in a public report. Every reported number carries its convergence.
- **Fabricable:** foundry design rules inside the optimization, GDS output that passes an open PDK's DRC, and performance reported across process variation.
- **Visible:** a native studio that shows fields propagating, modes, layouts and optimizations as they run. The CLI and the studio run the same job, and every run replays.

And underneath:

- **Pure Rust:** no C, Fortran or Python dependencies, from the linear algebra to the GDS writer. There are no Python bindings.
- **Fast:** parallel on the CPU, with a GPU backend for FDTD.
- **Reproducible:** the same input gives the same result on any number of threads.
- **Inverse design built in:** adjoint gradients for every solver, density and level-set methods, robust and length-scale constraints. [genoxide](https://github.com/tachsin/genoxide) drives the global and discrete searches.

## Where it comes from

photonoxide grows out of a working, unpublished prototype: a wavelength demultiplexer designed by adjoint topology optimization in 2D and 3D, with a live egui window. The prototype reached 97–98% transmission in 2D under ±10 nm fabrication errors. In 3D the same design kept only 46–70%. That gap is why "validated" and "verified in 3D" are first-class here. The roadmap lists every pitfall it hit and the rule that rules it out.

## Status

| Milestone | Scope | Status |
|---|---|---|
| 0.1 Foundations | Units, materials with provenance, geometry, run records, studio skeleton, validation harness | 🔜 next |
| 0.2 Mode solvers | Slab, full-vector 2D finite differences, EIM, bends, dispersion | planned |
| 0.3 FDFD | 2D and 3D, mode ports, S-parameters, adjoints | planned |
| 0.4 FDTD | 2D and 3D Yee, CPML, subpixel smoothing, dispersive media, GPU | planned |
| 0.5 Semi-analytic | TMM, RCWA, eigenmode expansion, BPM | planned |
| 0.6 Inverse design | Adjoint topology and shape optimization, fabrication constraints, the 2D-to-3D pipeline | planned |
| 0.7 Layout and PDK | GDSII, parametric cells, routing, DRC, SiEPIC EBeam and Cornerstone | planned |
| 0.8 Fabrication realism | Process variation, lithography proxies, corners, yield | planned |
| 0.9 Circuits and devices | S-parameter circuits, compact models, a validated device library | planned |
| 0.10 – 0.12 | Photonic crystals, metasurfaces, plasmonics, nonlinear and fiber optics, multiphysics, quantum | planned |
| 1.0 | Stable API and the published validation report | planned |

The details are in [ROADMAP.md](ROADMAP.md).

## Validation

Every solver is validated on three tiers before it ships:

- **Analytic solutions:** Fresnel, slab modes, Mie scattering, Bragg stacks, PML theory.
- **Cross-code:** Meep, MPB, S4, Ceviche and oxiphoton on the same structures.
- **Published devices:** inverse-designed demultiplexers, beamsplitters and grating couplers, reproduced in 3D.

Each result states its tolerance, grid and source. The report will be published here.

## Contributing

photonoxide is at the design stage, which is the best time to shape it. Open an issue for ideas, use cases or validation cases you'd like to see.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in photonoxide by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

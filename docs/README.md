# photonoxide documentation

photonoxide is an open reference for photonic design and the software that puts it to use. The
documentation has four parts. The **methods** pages derive each solver from the papers it
implements. The **Academy** teaches devices and numerical methods at three depths, with charts
that the library computes. The **fabrication** pages show step by step how a design would be
made, every stated process capability with its source. The **software** pages describe the Rust
library, the Python package, the `photonoxide` program and the studio, which run the same
methods. A page cites its sources by DOI, and what it states is checked by validation cases in
[the validation report](validation.md): each case compares a result with an analytic solution, a
published result or another code, within a tolerance that has a stated basis.

## Start

- [Getting started](getting-started.md): adding photonoxide to a project, a first waveguide
  mode, a first material and a first circuit.
- [Designs as job files](design.md): the designs people start from, each as a job file to run
  and edit.
- [photonoxide in Python](python.md): the package and the part of the library it reaches.
- [photonoxide from any language](languages.md): the program, job files and run folders as the
  interface for every other language.
- [What photonoxide offers, and in which language](features.md): what works in Rust and in
  Python.

## Methods

One page per method, with its derivation, its sources and the validation cases that check it.
The pages are grouped as the project site groups them.

### Waveguide modes

*Exact and approximate*

- [slab](methods/slab.md): the TE and TM modes of a core between two claddings, from their
  characteristic equations, to the last bit.
- [multilayer](methods/multilayer.md): the bound modes and leaky waves of any planar stack, and
  its reflection and transmission of a plane wave, from 2 × 2 field-transfer matrices.
- [marcatili](methods/marcatili.md): a rectangular guide's modes from two slab-like equations,
  ignoring the corners.
- [eim](methods/eim.md): a ridge waveguide's mode from two slab problems, fast and approximate,
  with its error against the full-vector solver measured.

*On a grid*

- [vector](methods/vector.md): the modes of any waveguide cross-section, anisotropic and lossy
  media included, from the transverse magnetic field on a rectilinear grid.
- [hadley](methods/hadley.md): full-vector modes from finite-difference equations built on the
  exact local solutions, of high order at interfaces and corners.
- [slab-fd](methods/slab-fd.md): the modes of any planar profile, graded, lossy or leaky, by 1D
  finite differences.
- [bends](methods/bends.md): bent waveguides and their radiation loss, by an exact bent slab and
  by a conformal map in the full-vector solver.

*The window's edges*

- [pml](methods/pml.md): absorbing layers inside the window's edges, by complex coordinate
  stretching.
- [walls](methods/walls.md): electric and magnetic walls on the window's edges, to solve a
  symmetric waveguide on half or a quarter of its cross-section.

*What a mode gives*

- [fields](methods/fields.md): a full-vector mode's six field components, its power, effective
  area, nonlinear coefficient and confinement, and its effective index's change for a change of
  permittivity.
- [dispersion](methods/dispersion.md): group index, dispersion and loss from the effective index
  across wavelength, and following one mode through a sweep.

### Full-wave simulation

*Frequency domain (FDFD)*

- [fdfd](methods/fdfd.md): Maxwell's equations at one frequency on Yee's grid as one sparse
  linear system, with absorbing or periodic sides and the exact discrete power flux.
- [fdfd-ports](methods/fdfd-ports.md): waveguide modes into and out of a 2D FDFD problem, mode
  amplitudes and a reciprocal S-matrix.
- [fdfd-3d](methods/fdfd-3d.md): the same equations on Yee's 3D grid, with direct and iterative
  solvers, and ports with a reciprocal S-matrix.
- [averaging](methods/averaging.md): each field component's permittivity from the share of its
  cell that every material fills and the interface's normal.

*Time domain (FDTD)*

- [fdtd](methods/fdtd.md): Maxwell's equations stepped in time on Yee's grid, with the
  convolutional PML, dispersive media, sources, monitors, S-parameters over a band and a GPU
  path.

*Eigenmode expansion*

- [eme](methods/eme.md): a 2D structure cut along its length into uniform sections, joined by
  S-matrices of their modes.

### Numerical linear algebra

*Eigenvalues*

- [eigen](methods/eigen.md): the eigenvalues of a large sparse matrix nearest a shift, by
  Arnoldi on the shifted inverse with Krylov–Schur restarts.
- [contour](methods/contour.md): every mode whose effective index lies in a disc or an ellipse
  of the complex plane, by FEAST's subspace iteration on a contour integral.

*Linear systems*

- [block-qmr](methods/block-qmr.md): all of a 3D S-matrix's ports solved at once in one block
  Krylov space.
- [recycling](methods/recycling.md): each iterative solve started from what earlier solves
  learned, in a sweep, an S-matrix, an adjoint or an optimization.
- [multigrid-stencil](methods/multigrid-stencil.md): GMRES with a 3D multigrid whose finest
  level is never assembled as a matrix, to save memory.
- [backends](methods/backends.md): the one place where the solvers ask for a sparse direct
  solve, by photonoxide's own factorizations or by another library registered at run time.

### Gradients

- [fdfd-adjoint](methods/fdfd-adjoint.md): how a port's power changes with every cell's
  permittivity, from one more back-substitution.
- [fdtd-adjoint](methods/fdtd-adjoint.md): how an objective on an FDTD run's monitors changes
  with the density of every cell of a 3D design region, from a forward and an adjoint run.
- [circuit-adjoint](methods/circuit-adjoint.md): how a circuit's response changes with every
  parameter of every component, from one more back-substitution with the transposed system.
- [optimization](methods/optimization.md): a device's objectives and constraints, their
  derivatives, and the `optimize` job that moves its parameters with genoxide's optimizers.

### Heat and static fields

- [static](methods/static.md): heat conduction, electrostatics and current continuity on a
  waveguide's cross-section by finite volumes, and heat in time.
- [phase-shifter](methods/phase-shifter.md): a thermo-optic phase shifter from the static solver
  and the materials' thermo-optic coefficients: its power for a phase of π, its time constant
  and its crosstalk.
- [thermal-compact](methods/thermal-compact.md): a phase shifter's step responses fitted as a
  sum of first-order terms, as a compact model for a circuit.
- [electro-optic](methods/electro-optic.md): a crystal's electro-optic tensor turned into a
  mode's change of effective index and a modulator's Vπ·L.
- [travelling-wave](methods/travelling-wave.md): a modulator's electrode as a transmission line:
  its RF index, impedance, losses and the modulator's response.

### Nonlinear optics

- [nonlinear](methods/nonlinear.md): the wave-vector mismatch of three- and four-wave processes,
  phase matching, acceptance bandwidths, the coupled-amplitude equations, and third-order
  processes, in crystals and in waveguides.

### Circuits and compact models

- [circuits](methods/circuits.md): a chip as a netlist of components, each an S-matrix, solved
  as one sparse linear system.
- [components](methods/components.md): waveguides, bends, couplers, MMIs, Y-branches, rings and
  the Mach-Zehnder interferometer as S-matrices.
- [devices](methods/devices.md): a component that knows its geometry, with its models from a fit
  to the solvers and the fidelity and grid of every figure.
- [compact](methods/compact.md): a component's S-matrix spectrum as a rational function of
  frequency by vector fitting, and Touchstone files.
- [sweep](methods/sweep.md): a response over wavelength or frequency from as few solves as it
  needs, by adaptive sampling and AAA rational approximation.

### Fabrication

- [fabrication](methods/fabrication.md): how a design would be made, read from its layers and
  shapes: the steps in order from published processes, the rules that check them, and the
  accuracy of each fabricated dimension.

### Foundations

Not solvers: the conventions and the data the methods above are built on.

- [conventions](methods/conventions.md): units, and the one time convention that fixes every
  sign in the library.
- [geometry](methods/geometry.md): shapes kept exact and turned into polygons only on demand,
  paths, curves and GDSII files.
- [booleans](methods/booleans.md): union, intersection, difference and xor of any shapes,
  offsets and fillets.
- [arrays](methods/arrays.md): many shapes as one region, in lattices with defects, and named
  cells placed many times.
- [bodies](methods/bodies.md): bodies in three dimensions with exact volumes in a cell, and
  graded media.
- [generators](methods/generators.md): photonic components drawn from their parameters, as
  exact shapes on layers with named ports.
- [stacks](methods/stacks.md): custom layer stacks, partial etches and sloped sidewalls, read
  the same way by every solver.
- [expressions](methods/expressions.md): the small, safe expression language for a structure's
  numbers, with units and derivatives.
- [materials](methods/materials.md): refractive indices with their sources, and the models that
  hold them.
- [catalogue](methods/catalogue.md): each material's index models, crystal data and electro-optic
  tensors, every number from its paper.
- [export](methods/export.md): the tables and fields a run writes, so that a result can be used
  outside photonoxide.
- [convergence](methods/convergence.md): the gate that a number from a grid solve passes before
  it is reported, and the driver that refines a grid until a tolerance is met.

## Academy

[The Academy](../academy/README.md) lists the lessons: those written and those still to come,
in the order to read them in. A lesson explains a device or a topic at three depths, tells its
history paper by paper, and draws charts that photonoxide computes. The lessons are built into
the studio and read on GitHub as ordinary Markdown.

## Fabrication

- [Fabrication](methods/fabrication.md): the engine and its rules.
- [A fabrication engine: the plan](plans/fabrication.md): the plan and its phases.
- The studio shows a design's flow in the Fabrication tab of a device's page.

## Validation

- [Validation report](validation.md): every case, with the value photonoxide gives, the
  reference, the tolerance and its basis. It is written by `photonoxide validate`, and CI fails
  when a case fails or the file is out of date.
- [Validation report: the GPU](validation-gpu.md): the GPU's cases, written before each release
  on a computer with a GPU.
- [Cross-code comparison](cross-code.md): each solver against an established code that solves
  the same physics; the reports from other codes are [openEMS](validation-openems.md),
  [openEMS on more lines](validation-openems-lines.md) and [Elmer](validation-elmer.md), and the
  programs that run the other codes are in [crosscode/](../crosscode/README.md).
- [Direct solvers](baselines.md): photonoxide's sparse direct solves against PARDISO and MUMPS.
- [Benchmarks](benchmarks.md): what `photonoxide bench` measures.

## Running and reproducing

- [A run's folder](runs.md): what `photonoxide run` leaves behind, for a script in any language
  to read.
- [Farming](farming.md): a sweep's points and whole jobs sent to other processes.
- [Installing the external libraries](libraries.md): the optional libraries that the solvers can
  use in place of their own.
- [job.schema.json](job.schema.json) and [results.schema.json](results.schema.json): the schemas
  of a job file and of a run's results.

## Architecture and plans

- [Architecture](architecture.md): the library's modules and their layers.
- [Components and circuits: the design](design/components.md): the model behind
  `photonoxide::circuit`.
- [Roadmap](../ROADMAP.md): what each release holds and what comes next.
- Plans, each reviewed before work on it starts:
  - [External backends and the in-app benchmark](plans/backends.md)
  - [Python bindings, and every other language through the program](plans/bindings.md)
  - [A fabrication engine](plans/fabrication.md)
  - [Optimization in photonic design](plans/optimization.md)
  - [Performance: kernels, threads, GPUs and distributed memory](plans/performance.md)
  - [Quantum photonics](plans/quantum.md)

## Contributing

[CONTRIBUTING.md](../CONTRIBUTING.md) describes how to propose a change, how a validation case
and its tolerance are written, and what CI runs. The [code of conduct](../CODE_OF_CONDUCT.md)
and the [security policy](../SECURITY.md) apply to every contribution.

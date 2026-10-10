# What photonoxide offers, and in which language

photonoxide is a Rust library; Python reaches a part of it through the `photonoxide` package,
and MATLAB through that package ([docs/python.md](python.md)). This table says, for each solver
and method, where it can be used today.

- ✅ available, and tested in that language
- ◐ part of it, as the note says; "a job" means through a job file (TOML, JSON or YAML, or a
  dict in Python), which `run_job` runs as the `photonoxide` program does, returning the run's
  events
- — not yet
- † in MATLAB, through the Python package's `.m` wrappers: their conversions are tested in
  Python, MATLAB itself is untested until someone with a licence runs them

The Python and MATLAB columns follow the façade the package wraps (`photonoxide::facade`); a
test fails when one of its functions is missing here.

## Materials

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| The catalogue: indices with source, range and conditions, χ⁽²⁾ and Pockels tensors | ✅ | ◐ indices, models and sources: `materials`, `refractive_index`, `group_index`; not the tensors | ◐ † the same | [catalogue](methods/catalogue.md) |
| Dispersion models of your own (Sellmeier, Cauchy, Drude, Lorentz, tables) | ✅ | — | — | [materials](methods/materials.md) |
| refractiveindex.info files | ✅ | — | — | [materials](methods/materials.md) |

## Modes

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| The slab, exact, with its fields | ✅ | ✅ `slab_modes` | ✅ † | [slab](methods/slab.md) |
| Multilayers: modes and reflection | ✅ | — | — | [multilayer](methods/multilayer.md) |
| Planar profiles by finite differences | ✅ | — | — | [slab-fd](methods/slab-fd.md) |
| Full-vector modes of a cross-section, mirror walls and PMLs | ✅ | ✅ `vector_modes`, the permittivity as an array; ◐ shapes on a stack: a job | ✅ † | [vector](methods/vector.md), [PML](methods/pml.md), [walls](methods/walls.md) |
| A mode's six field components | ✅ | ✅ `vector_modes` | ✅ † | [fields](methods/fields.md) |
| Hadley's scheme for dielectric corners | ✅ | — | — | [hadley](methods/hadley.md) |
| Bends | ✅ | — | — | [bends](methods/bends.md) |
| The effective index method | ✅ | ◐ inside 2D FDFD and FDTD jobs | ◐ † the same | [eim](methods/eim.md) |
| Marcatili's method | ✅ | — | — | [marcatili](methods/marcatili.md) |
| Every mode in a region of n_eff, by contour integrals | ✅ | — | — | [contour](methods/contour.md) |
| Dispersion: group index and D | ✅ | ◐ effective indices over a wavelength sweep: a job | ◐ † the same | [dispersion](methods/dispersion.md) |

## FDFD

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| 2D | ✅ | ◐ a job (`fdfd_s_parameters`, `run_job`) | ◐ † the same | [fdfd](methods/fdfd.md) |
| 3D | ✅ | — | — | [fdfd-3d](methods/fdfd-3d.md) |
| Mode ports and S-parameters | ✅ | ◐ 2D, a job: `fdfd_s_parameters` | ◐ † the same | [ports](methods/fdfd-ports.md) |
| Adjoint gradients | ✅ | — | — | [fdfd-adjoint](methods/fdfd-adjoint.md) |
| Solvers: direct (multifrontal) | ✅ | ◐ a job's `[solver] direct` | ◐ † the same | [backends](methods/backends.md) |
| Solvers: QMR, block QMR, multigrid, recycling | ✅ | — | — | [block-qmr](methods/block-qmr.md), [fdfd-3d](methods/fdfd-3d.md), [recycling](methods/recycling.md) |

## FDTD

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| 2D and 3D, CPMLs | ✅ | ◐ a job (`run_job`): spectra and resonances in its events | ◐ † the same | [fdtd](methods/fdtd.md) |
| Sources and monitors | ✅ | ◐ a job's sources and monitors | ◐ † the same | [fdtd](methods/fdtd.md) |
| Subpixel smoothing, dispersive media | ✅ | — | — | [fdtd](methods/fdtd.md) |
| Adjoint gradients | ✅ | — | — | [fdtd-adjoint](methods/fdtd-adjoint.md) |
| The GPU (wgpu, the `gpu` feature) | ✅ | — | — | [fdtd](methods/fdtd.md) |

## Circuits, compact models and Touchstone

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| Components: waveguides, bends, phase shifters, couplers, Y-branches, rings, measured | ✅ | ◐ these, by kind in a netlist: `circuit_spectrum`; not MMIs or components from mode solves | ◐ † the same | [components](methods/components.md) |
| Netlists and the circuit solve | ✅ | ✅ `circuit_spectrum` | ✅ † | [circuits](methods/circuits.md) |
| The circuit adjoint | ✅ | — | — | [circuit-adjoint](methods/circuit-adjoint.md) |
| Optimization with genoxide | ✅ | — | — | [circuit-adjoint](methods/circuit-adjoint.md) |
| Compact models: vector fitting, models over parameters | ✅ | — | — | [compact](methods/compact.md) |
| Touchstone files | ✅ | ✅ `read_touchstone`, `write_touchstone` | ✅ † | [compact](methods/compact.md) |

## Jobs, runs and backends

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| Jobs: structure, modes, FDFD and FDTD, checked and run | ✅ | ✅ `check_job`, `run_job` | ◐ † `run_job` | [runs](getting-started.md#runs-and-the-studio) |
| Run records and replay | ✅ | ◐ the events of a run it makes | ◐ † the same | [runs](getting-started.md#runs-and-the-studio) |
| External solvers: PARDISO, MUMPS, SuperLU, Accelerate, cuDSS, Krylov on NVIDIA GPUs, `auto` | ✅ | — | — | [backends](methods/backends.md) |
| The studio, the `photonoxide` program | ✅ | ◐ it replays the runs `run_job` writes (`photonoxide view`) | ◐ † the same | [studio](../studio/README.md) |

# What photonoxide offers, and in which language

photonoxide is a Rust library; Python reaches a part of it through the `photonoxide` package,
and MATLAB through that package ([docs/python.md](python.md)). This table says, for each solver
and method, where it can be used today.

- ✅ available, and tested in that language
- ◐ part of it, as the note says; "a job" means through a job file (YAML, TOML or JSON, or a
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
| The catalogue: indices with source, range and conditions, χ⁽²⁾ and Pockels tensors | ✅ | ◐ indices, models and sources: `materials`, `refractive_index`, `group_index`; not the tensors | ◐ † the same: [`materials`](matlab.md#photonoxidematerials), [`refractive_index`](matlab.md#photonoxiderefractive_index), [`group_index`](matlab.md#photonoxidegroup_index) | [catalogue](methods/catalogue.md) |
| Dispersion models of your own (Sellmeier, Cauchy, Drude, Lorentz, tables) | ✅ | — | — | [materials](methods/materials.md) |
| refractiveindex.info files | ✅ | — | — | [materials](methods/materials.md) |

## Modes

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| The slab, exact, with its fields | ✅ | ✅ `slab_modes` | ✅ †: [`slab_modes`](matlab.md#photonoxideslab_modes) | [slab](methods/slab.md) |
| Multilayers: modes and reflection | ✅ | — | — | [multilayer](methods/multilayer.md) |
| Planar profiles by finite differences | ✅ | — | — | [slab-fd](methods/slab-fd.md) |
| Full-vector modes of a cross-section, mirror walls and PMLs | ✅ | ✅ `vector_modes`, the permittivity as an array: the guided modes, or with PMLs every mode solved, leaky ones with their loss; ◐ shapes on a stack: a job | ✅ †: [`vector_modes`](matlab.md#photonoxidevector_modes) | [vector](methods/vector.md), [PML](methods/pml.md), [walls](methods/walls.md) |
| Guided modes told from the closed window's own (n_eff above the media at its open edges) | ✅ | ✅ `vector_modes` returns only these, without PMLs | ✅ †: [`vector_modes`](matlab.md#photonoxidevector_modes) | [vector](methods/vector.md) |
| Leaky modes of shapes on a stack: a modes job with a PML, each mode named guided or leaky with its loss in dB/cm | ✅ | ◐ a job (`run_job`): its `mode_loss` events | ◐ † the same: [`run_job`](matlab.md#photonoxiderun_job) | [PML](methods/pml.md), [vector](methods/vector.md) |
| A mode's six field components | ✅ | ✅ `vector_modes` | ✅ †: [`vector_modes`](matlab.md#photonoxidevector_modes) | [fields](methods/fields.md) |
| Hadley's scheme for dielectric corners | ✅ | — | — | [hadley](methods/hadley.md) |
| Bends | ✅ | — | — | [bends](methods/bends.md) |
| The effective index method | ✅ | ◐ inside 2D FDFD and FDTD jobs | ◐ † the same | [eim](methods/eim.md) |
| Marcatili's method | ✅ | — | — | [marcatili](methods/marcatili.md) |
| Many modes nearest an index (20 to 50), shift-and-invert with Krylov–Schur restarts | ✅ | ◐ `vector_modes`, its count, the guided ones of it | ◐ † the same: [`vector_modes`](matlab.md#photonoxidevector_modes) | [eigen](methods/eigen.md) |
| Every mode in a region of n_eff, by contour integrals | ✅ | — | — | [contour](methods/contour.md) |
| Every guided mode above an n_eff threshold, no count | ✅ | ◐ a modes job's `modes_above` | ◐ † the same | [contour](methods/contour.md) |
| Dispersion: group index and D | ✅ | ◐ effective indices over a wavelength sweep: a job | ◐ † the same | [dispersion](methods/dispersion.md) |

## FDFD

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| 2D | ✅ | ◐ a job (`fdfd_s_parameters`, `run_job`) | ◐ † the same: [`fdfd_s_parameters`](matlab.md#photonoxidefdfd_s_parameters), [`run_job`](matlab.md#photonoxiderun_job) | [fdfd](methods/fdfd.md) |
| 3D | ✅ | — | — | [fdfd-3d](methods/fdfd-3d.md) |
| Mode ports and S-parameters | ✅ | ◐ 2D, a job: `fdfd_s_parameters` | ◐ † the same: [`fdfd_s_parameters`](matlab.md#photonoxidefdfd_s_parameters) | [ports](methods/fdfd-ports.md) |
| Adjoint gradients | ✅ | — | — | [fdfd-adjoint](methods/fdfd-adjoint.md) |
| Solvers: direct (multifrontal) | ✅ | ◐ a job's `[solver] direct` | ◐ † the same | [backends](methods/backends.md) |
| Solvers: QMR, block QMR, multigrid, recycling | ✅ | — | — | [block-qmr](methods/block-qmr.md), [fdfd-3d](methods/fdfd-3d.md), [multigrid-stencil](methods/multigrid-stencil.md), [recycling](methods/recycling.md) |

## FDTD

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| 2D and 3D, CPMLs | ✅ | ◐ a job (`run_job`): spectra and resonances in its events | ◐ † the same: [`run_job`](matlab.md#photonoxiderun_job) | [fdtd](methods/fdtd.md) |
| Sources and monitors | ✅ | ◐ a job's sources and monitors | ◐ † the same | [fdtd](methods/fdtd.md) |
| Subpixel smoothing, dispersive media | ✅ | — | — | [fdtd](methods/fdtd.md) |
| Adjoint gradients | ✅ | — | — | [fdtd-adjoint](methods/fdtd-adjoint.md) |
| The GPU (wgpu, the `gpu` feature) | ✅ | — | — | [fdtd](methods/fdtd.md) |

## Circuits, compact models and Touchstone

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| Components: waveguides, bends, phase shifters, couplers, Y-branches, rings, measured | ✅ | ◐ these, by kind in a netlist: `circuit_spectrum`; MMIs and the models from mode solves and FDFD as devices, `device_spectrum` | ◐ † the same: [`circuit_spectrum`](matlab.md#photonoxidecircuit_spectrum), [`device_spectrum`](matlab.md#photonoxidedevice_spectrum) | [components](methods/components.md) |
| Netlists and the circuit solve | ✅ | ✅ `circuit_spectrum`, its instances by kind, or devices with their parameters and fidelity | ✅ †: [`circuit_spectrum`](matlab.md#photonoxidecircuit_spectrum) | [circuits](methods/circuits.md) |
| The circuit adjoint | ✅ | — | — | [circuit-adjoint](methods/circuit-adjoint.md) |
| Optimization with genoxide | ✅ | — | — | [circuit-adjoint](methods/circuit-adjoint.md) |
| Compact models: vector fitting, models over parameters | ✅ | — | — | [compact](methods/compact.md) |
| Touchstone files | ✅ | ✅ `read_touchstone`, `write_touchstone` | ✅ †: [`read_touchstone`](matlab.md#photonoxideread_touchstone), [`write_touchstone`](matlab.md#photonoxidewrite_touchstone) | [compact](methods/compact.md) |

## Devices

The components that know their geometry, as a component's page in the studio has them: the
same designs give the same numbers in Rust, Python and MATLAB.

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| The library's devices: strip and rib waveguides, bend, directional coupler, rings, MMIs, Y-branch, MZIs, each with its parameters (unit, default, range), models and ports | ✅ | ✅ `devices` | ✅ †: [`devices`](matlab.md#photonoxidedevices) | [devices](methods/devices.md) |
| The catalogue's stacks (soi_220, soi, nitride), each with its layers of the catalogue's materials, which a design names | ✅ | ✅ `stacks`, and `stack=` by name in every device function | ✅ †: [`stacks`](matlab.md#photonoxidestacks) | [devices](methods/devices.md) |
| A design's S-parameters at a fidelity, from a closed form to 3D FDFD, with its model's grid and source | ✅ | ✅ `device_spectrum`: a stack of the catalogue's by name, and the compact fidelity from a fitted model's file (a `"fit"` job's, run by `run_job`); ◐ a stack of the catalogue's layers of your own: a job | ✅ †: [`device_spectrum`](matlab.md#photonoxidedevice_spectrum), [`run_job`](matlab.md#photonoxiderun_job) | [devices](methods/devices.md) |
| A design's solve stopped at a timeout or on Ctrl+C | ✅ `device_spectrum_until` | ✅ `device_spectrum(..., timeout_s=)` | ✅ †: [`device_spectrum`](matlab.md#photonoxidedevice_spectrum), `timeout_s` | [devices](methods/devices.md) |
| A design's figures (n_eff, n_g, the cross-over length, FSR, Q, a bend's loss), each with its fidelity and grid | ✅ | ✅ `device_derived` | ✅ †: [`device_derived`](matlab.md#photonoxidedevice_derived) | [devices](methods/devices.md) |
| A design's geometry: its stack, the polygons on each layer with their etch, its ports | ✅ | ✅ `device_geometry` | ✅ †: [`device_geometry`](matlab.md#photonoxidedevice_geometry) | [devices](methods/devices.md) |
| The job that computes a design, as the Simulate tab writes it | ✅ | ✅ `device_job`, run by `run_job` | ✅ †: [`device_job`](matlab.md#photonoxidedevice_job), [`run_job`](matlab.md#photonoxiderun_job) | [devices](methods/devices.md) |
| Sweeps, corners and optimization of a device | ✅ | ◐ a job (`run_job`) | ◐ † the same: [`run_job`](matlab.md#photonoxiderun_job) | [devices](methods/devices.md), [optimization](methods/optimization.md) |

## Jobs, runs and backends

| | Rust | Python | MATLAB | Method |
|---|---|---|---|---|
| Jobs: structure, modes, FDFD and FDTD, checked and run | ✅ | ✅ `check_job`, `run_job` | ◐ † [`run_job`](matlab.md#photonoxiderun_job) | [runs](getting-started.md#runs-and-the-studio) |
| Run records and replay | ✅ | ◐ the events of a run it makes | ◐ † the same | [runs](getting-started.md#runs-and-the-studio) |
| A run's tables (n_eff, S-parameters, spectra, studies, histories, static numbers) as data and as CSV; its fields as NumPy's `.npz` when its job asks, written again at a chosen precision and level with their sizes estimated before writing | ✅ | ✅ `run_tables`, `export_run`, `estimate_fields`, `export_fields`; the `.npz` files by `numpy.load` | ✅ † [`run_tables`](matlab.md#photonoxiderun_tables) (MATLAB tables), [`export_run`](matlab.md#photonoxideexport_run), [`estimate_fields`](matlab.md#photonoxideestimate_fields), [`export_fields`](matlab.md#photonoxideexport_fields) | [export](methods/export.md) |
| External solvers: PARDISO, MUMPS, SuperLU, Accelerate, cuDSS, Krylov on NVIDIA GPUs, `auto` | ✅ | — | — | [backends](methods/backends.md) |
| The studio, the `photonoxide` program | ✅ | ◐ it replays the runs `run_job` writes (`photonoxide view`) | ◐ † the same: [`run_job`](matlab.md#photonoxiderun_job) | [studio](../studio/README.md) |

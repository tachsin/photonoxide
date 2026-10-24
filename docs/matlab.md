# photonoxide in MATLAB

Written from the help text of the wrappers in [`python/photonoxide/matlab/+photonoxide`](../python/photonoxide/matlab/+photonoxide) by `src/facade/matlab_page.rs`; don't edit it by hand. CI fails when it isn't what the wrappers say, and `PHOTONOXIDE_WRITE_MATLAB_PAGE=1 cargo test --lib facade::matlab_page` rewrites it. In MATLAB, `help photonoxide.<name>` shows the same text.

MATLAB calls photonoxide through the Python package, from R2022b. Install the package with pip into a Python of 3.10 or later, then, once per session:

```matlab
pyenv(Version="/path/to/python");             % a Python with photonoxide installed, once
addpath(string(py.photonoxide.matlab.path()));
```

The wrappers take MATLAB's arrays, structs and name-value arguments and return MATLAB arrays and structs, keeping the Python package's shapes and indexing; photonoxide's errors are MATLAB errors with identifiers such as `photonoxide:InvalidValue`. How they convert, what is tested and what isn't (MATLAB itself isn't run in CI) is in [docs/python.md](python.md#matlab).

## The functions

| Function | What it is |
|---|---|
| [`photonoxide.circuit_spectrum`](#photonoxidecircuit_spectrum) | The S-matrices of a circuit given as data. |
| [`photonoxide.device_derived`](#photonoxidedevice_derived) | A device's figures at a fidelity. |
| [`photonoxide.device_geometry`](#photonoxidedevice_geometry) | A device's geometry: its stack, the polygons on each layer, its ports. |
| [`photonoxide.device_job`](#photonoxidedevice_job) | The job file that computes a device's design. |
| [`photonoxide.device_spectrum`](#photonoxidedevice_spectrum) | A device's S-parameters at a fidelity. |
| [`photonoxide.devices`](#photonoxidedevices) | The devices of photonoxide's library. |
| [`photonoxide.estimate_fields`](#photonoxideestimate_fields) | The field files of a photonoxide run, their sizes estimated before writing. |
| [`photonoxide.export_fields`](#photonoxideexport_fields) | Writes a photonoxide run's field files again, into another folder. |
| [`photonoxide.export_run`](#photonoxideexport_run) | Writes a photonoxide run's tables as CSV files. |
| [`photonoxide.fdfd_s_parameters`](#photonoxidefdfd_s_parameters) | The S-parameters of an "fdfd" job's device by 2D FDFD. |
| [`photonoxide.group_index`](#photonoxidegroup_index) | The bulk group index n - lambda dn/dlambda of a material of photonoxide's catalogue. |
| [`photonoxide.materials`](#photonoxidematerials) | The materials of photonoxide's catalogue. |
| [`photonoxide.read_touchstone`](#photonoxideread_touchstone) | The S-parameters of a Touchstone file. |
| [`photonoxide.refractive_index`](#photonoxiderefractive_index) | The refractive index n + i*kappa of a material of photonoxide's catalogue. |
| [`photonoxide.run_job`](#photonoxiderun_job) | Runs a photonoxide job, as photonoxide run --headless does. |
| [`photonoxide.run_tables`](#photonoxiderun_tables) | The tables of a photonoxide run, from its record. |
| [`photonoxide.slab_modes`](#photonoxideslab_modes) | The guided modes of a three-layer slab, solved exactly. |
| [`photonoxide.stacks`](#photonoxidestacks) | The stacks of photonoxide's catalogue, by the name the device functions take. |
| [`photonoxide.vector_modes`](#photonoxidevector_modes) | Full-vector modes of a waveguide's cross-section, with their fields. |
| [`photonoxide.version`](#photonoxideversion) | The photonoxide package's version, and photonoxide's: v.package and v.core. |
| [`photonoxide.write_touchstone`](#photonoxidewrite_touchstone) | Writes S-matrices to a Touchstone file. |

## photonoxide.circuit_spectrum

The S-matrices of a circuit given as data.

```matlab
s = photonoxide.circuit_spectrum(netlist, wavelength_um)
```

s = photonoxide.circuit_spectrum(netlist, wavelength_um) solves the circuit netlist
describes at the vacuum wavelengths wavelength_um (micrometres). netlist is a struct:

```matlab
netlist.instances.split = struct("kind", "coupler", "coupling", 0.5);
netlist.instances.upper = struct("kind", "waveguide", "n_eff", 2.44506, "n_g", 4.172901, ...
                                 "wavelength_um", 1.55, "length", 150);
...
netlist.connections = {{"split.o3", "upper.o1"}, {"split.o4", "lower.o1"}, ...};
netlist.ports = struct("in1", "split.o2", "in2", "split.o1", "out1", "combine.o3", ...);
```

Each instance names its kind, its settings and its parameters' values, as Python's
photonoxide.circuit_spectrum takes them (see its help, or docs/python.md); the ports are in
the order the struct has them. An instance can name a device of photonoxide's library in
place of a kind, its design as a job's [device] table holds it:

```matlab
netlist.instances.ring = struct("device", struct("kind", "all-pass ring", ...
                                "parameters", struct("radius", 5)));
```

s.ports are the external ports' names, s.wavelength_um the wavelengths, and s.s the
S-matrices, nl-by-n-by-n: s.s(k, q, p) from port p into port q at the k-th wavelength.

| Argument | Size | Class | Default |
|---|---|---|---|
| `netlist` | scalar | struct | required |
| `wavelength_um` | any | double | required |

## photonoxide.device_derived

A device's figures at a fidelity.

```matlab
f = photonoxide.device_derived(kind, options)
```

f = photonoxide.device_derived(kind) is a struct array of the figures of the device kind at
its defaults by its cheapest model: f(k).name, f(k).symbol (e.g. "n_eff"), f(k).value,
f(k).unit, f(k).fidelity and f(k).grid.

Options: values, fidelity, stack and compact, as photonoxide.device_spectrum takes them.

| Argument | Size | Class | Default |
|---|---|---|---|
| `kind` | scalar | string | required |
| `values` (name-value) | scalar | struct | `struct()` |
| `fidelity` (name-value) | scalar | string | `""` |
| `stack` (name-value) | scalar | string | `""` |
| `compact` (name-value) | scalar | string | `""` |

## photonoxide.device_geometry

A device's geometry: its stack, the polygons on each layer, its ports.

```matlab
g = photonoxide.device_geometry(kind, options)
```

g = photonoxide.device_geometry(kind) is the geometry of the device kind at its defaults:
g.layers, a struct array bottom to top (name, z_um, material, background, polygons: each
polygon's outer outline n-by-2 in micrometres, counterclockwise, its holes, etch_depth_um,
empty through the layer, and sidewall_deg), g.substrate, g.cladding and g.ports (name,
position_um, direction_deg, width_um, layer).

Options: values and stack, as photonoxide.device_spectrum takes them.

| Argument | Size | Class | Default |
|---|---|---|---|
| `kind` | scalar | string | required |
| `values` (name-value) | scalar | struct | `struct()` |
| `stack` (name-value) | scalar | string | `""` |

## photonoxide.device_job

The job file that computes a device's design.

```matlab
text = photonoxide.device_job(kind, options)
```

text = photonoxide.device_job(kind) is the job, as JSON text, that computes the device kind
at its defaults by its cheapest model over its own band, as the device's Simulate tab in
the photonoxide studio writes it. photonoxide.run_job(text) runs it.

Options: values, fidelity, stack and compact, as photonoxide.device_spectrum takes them; name (the
kind's by default); format ("json", "yaml" or "toml"), the text to save as a job file.

| Argument | Size | Class | Default |
|---|---|---|---|
| `kind` | scalar | string | required |
| `values` (name-value) | scalar | struct | `struct()` |
| `fidelity` (name-value) | scalar | string | `""` |
| `stack` (name-value) | scalar | string | `""` |
| `compact` (name-value) | scalar | string | `""` |
| `name` (name-value) | scalar | string | `""` |
| `format` (name-value) | scalar | string | `"json"` |

## photonoxide.device_spectrum

A device's S-parameters at a fidelity.

```matlab
s = photonoxide.device_spectrum(kind, wavelength_um, options)
```

s = photonoxide.device_spectrum(kind, wavelength_um) computes the device kind of
photonoxide's library (see photonoxide.devices) at its defaults, at the vacuum wavelengths
wavelength_um (micrometres), by its cheapest model.

Options: values, a struct of its parameters' values in their units, e.g.
struct("gap", 0.25, "length", 10) (those not given at their defaults); fidelity
("analytic", "2D", "3D", ... as the device has them); stack, one of photonoxide.stacks by
name ("soi_220", "soi" or "nitride"); compact, the file of a compact model fitted to its
runs (the compact.json a "fit" job writes), for fidelity "compact"; timeout_s (Inf: none;
past it, the error photonoxide:Timeout, the solve let go).

s.ports are the device's ports, s.wavelength_um the wavelengths, s.s the S-matrices,
nl-by-n-by-n: s.s(k, q, p) from port p into port q at the k-th wavelength; s.fidelity,
s.grid and s.source say where they come from, and s.validity_um the wavelengths the model
holds for (empty when not limited). These are the numbers the device's page in the
photonoxide studio shows for the same design.

| Argument | Size | Class | Default |
|---|---|---|---|
| `kind` | scalar | string | required |
| `wavelength_um` | any | double | required |
| `values` (name-value) | scalar | struct | `struct()` |
| `fidelity` (name-value) | scalar | string | `""` |
| `stack` (name-value) | scalar | string | `""` |
| `compact` (name-value) | scalar | string | `""` |
| `timeout_s` (name-value) | scalar | double | `Inf` |

## photonoxide.devices

The devices of photonoxide's library.

```matlab
d = photonoxide.devices()
```

d = photonoxide.devices() is a struct array, one element per device: d(k).kind (what the
device functions take, e.g. "directional coupler"), d(k).about, d(k).parameters (a struct
array: name, unit, default, min, max), d(k).fidelities (its models, cheapest first:
fidelity, name, write_up, papers, error, validity) and d(k).ports.

## photonoxide.estimate_fields

The field files of a photonoxide run, their sizes estimated before writing.

```matlab
f = photonoxide.estimate_fields(run_dir, options)
```

f = photonoxide.estimate_fields(run_dir) lists the field files of the run in run_dir (its
.npz files, and a container's .h5), a struct array with each one's file, its bytes as the
run wrote it, its estimate written again as asked, whether that estimate is exact, the
precision its fields would have and whether it is rewritable (an .h5, or an .npz
compressed with Zstandard, is copied as it is). Stored, the estimate is the size to the
byte; deflated, it is for members up to 1 MiB, and from slices of larger ones.

Options: precision, "f64" or "f32" (the fields downcast, the grid kept f64), and level,
deflate's 0 (stored) to 9 (smallest).

| Argument | Size | Class | Default |
|---|---|---|---|
| `run_dir` | scalar | string | required |
| `precision` (name-value) | scalar | string | `"f64"` |
| `level` (name-value) | scalar | double | `6` |

## photonoxide.export_fields

Writes a photonoxide run's field files again, into another folder.

```matlab
files = photonoxide.export_fields(run_dir, out_dir, options)
```

files = photonoxide.export_fields(run_dir, out_dir) writes every field file of the run in
run_dir into out_dir, another folder, each written again at the precision and deflate level
asked, its meta.json saying so, and returns the files written, a string array. An .h5, or
an .npz compressed with Zstandard, is copied as it is. photonoxide.estimate_fields gives
their sizes before writing.

Options: files, the names of those to write ("mode-1-of-1.npz", ...; all of them when
empty); precision, "f64" or "f32" (the fields downcast, the grid kept f64); and level,
deflate's 0 (stored) to 9 (smallest).

| Argument | Size | Class | Default |
|---|---|---|---|
| `run_dir` | scalar | string | required |
| `out_dir` | scalar | string | required |
| `files` (name-value) | 1-by-: | string | `string.empty` |
| `precision` (name-value) | scalar | string | `"f64"` |
| `level` (name-value) | scalar | double | `6` |

## photonoxide.export_run

Writes a photonoxide run's tables as CSV files.

```matlab
files = photonoxide.export_run(run_dir, options)
```

files = photonoxide.export_run(run_dir) writes every table the run in run_dir has (its
n_eff, S-parameters, spectra, study, history, static numbers) as CSV files into the run's
own folder, and returns the files written, a string array. A run writes them itself when
it ends; this writes them again, or into another folder.

Options: tables, the names of those to write ("modes", "s_parameters", ...; all of them
when empty), and out_dir, the folder to write into (the run's own when empty).

| Argument | Size | Class | Default |
|---|---|---|---|
| `run_dir` | scalar | string | required |
| `tables` (name-value) | 1-by-: | string | `string.empty` |
| `out_dir` (name-value) | scalar | string | `""` |

## photonoxide.fdfd_s_parameters

The S-parameters of an "fdfd" job's device by 2D FDFD.

```matlab
s = photonoxide.fdfd_s_parameters(job, wavelength_um)
```

s = photonoxide.fdfd_s_parameters(job) solves job (a job file's path, or a struct with its
fields) at its own wavelengths, and s = photonoxide.fdfd_s_parameters(job, wavelength_um)
at wavelength_um (micrometres). A 2D result, by the effective index method: an estimate, not
a device's performance in 3D.

s.ports are the ports' names; s.wavelength_um the wavelengths; s.s the power-normalized
S-matrices, nl-by-n-by-n, s.s(k, q, p) from port p into port q at the k-th wavelength;
s.effective_index each port mode's, nl-by-n; s.cell_um the grid's cell [dx dy]; and
s.polarization "te" or "tm".

| Argument | Size | Class | Default |
|---|---|---|---|
| `job` | any | any | required |
| `wavelength_um` | any | double | `[]` |

## photonoxide.group_index

The bulk group index n - lambda dn/dlambda of a material of photonoxide's catalogue.

```matlab
ng = photonoxide.group_index(material, wavelength_um, options)
```

ng = photonoxide.group_index(material, wavelength_um) at the vacuum wavelengths
wavelength_um, in micrometres; ng has wavelength_um's size. The options are
photonoxide.refractive_index's.

| Argument | Size | Class | Default |
|---|---|---|---|
| `material` | scalar | string | required |
| `wavelength_um` | any | double | required |
| `model` (name-value) | scalar | string | none |
| `axis` (name-value) | scalar | string | none |
| `temperature_k` (name-value) | scalar | double | none |
| `composition` (name-value) | scalar | double | none |

## photonoxide.materials

The materials of photonoxide's catalogue.

```matlab
m = photonoxide.materials()
```

m = photonoxide.materials() is a struct of string arrays, one element per material: m.id
(what photonoxide.refractive_index takes, e.g. "si"), m.name, m.formula, and of each one's
default index model m.model (its id), m.axes ("isotropic", or "ordinary,extraordinary"),
and m.range_um, the wavelengths it is valid for (2-by-n, micrometres).

## photonoxide.read_touchstone

The S-parameters of a Touchstone file.

```matlab
s = photonoxide.read_touchstone(file, convention)
```

s = photonoxide.read_touchstone(file, convention) reads file (Version 1 or 2.0), its values
in the time convention convention: "physics" (exp(-i w t), photonoxide's) or "engineering"
(exp(+j w t), most RF tools'). The format doesn't say which, so the reader must.

s.ports are "o1", "o2", ...; s.wavelength_um the wavelengths, c/f, in the file's order
(increasing frequency, so decreasing wavelength); s.s the S-matrices, nl-by-n-by-n, in
photonoxide's convention.

| Argument | Size | Class | Default |
|---|---|---|---|
| `file` | scalar | string | required |
| `convention` | scalar | string | required |

## photonoxide.refractive_index

The refractive index n + i*kappa of a material of photonoxide's catalogue.

```matlab
n = photonoxide.refractive_index(material, wavelength_um, options)
```

n = photonoxide.refractive_index(material, wavelength_um) is the index of the catalogue's
material (an id: "si", "sio2", "linbo3", ...; photonoxide.materials lists them) at the
vacuum wavelengths wavelength_um, in micrometres; n has wavelength_um's size.

n = photonoxide.refractive_index(..., model=id, axis=name, temperature_k=t, composition=x)
picks an index model other than the default, an axis ("ordinary" or "extraordinary") for a
material with two, and a model's conditions.

A wavelength outside the model's data is the error photonoxide:OutsideValidity: photonoxide
doesn't extrapolate.

| Argument | Size | Class | Default |
|---|---|---|---|
| `material` | scalar | string | required |
| `wavelength_um` | any | double | required |
| `model` (name-value) | scalar | string | none |
| `axis` (name-value) | scalar | string | none |
| `temperature_k` (name-value) | scalar | double | none |
| `composition` (name-value) | scalar | double | none |

## photonoxide.run_job

Runs a photonoxide job, as photonoxide run --headless does.

```matlab
run = photonoxide.run_job(job, options)
```

run = photonoxide.run_job(job) runs job, a job file's path (YAML, TOML or JSON) or a struct
with a job file's fields (name, timeout_minutes, task), into a new folder under "runs", and
returns run.dir (the folder, which the photonoxide program replays), run.events (a cell
array, each event a struct with its type, as jsondecode reads it) and run.stopped
("timeout" or "requested" when it ended early, else "").

Options: runs_dir ("runs") and timeout_s (3600). The run also stops at the job's own
timeout_minutes, whichever comes first.

| Argument | Size | Class | Default |
|---|---|---|---|
| `job` | any | any | required |
| `runs_dir` (name-value) | scalar | string | `"runs"` |
| `timeout_s` (name-value) | scalar | double | `3600` |

## photonoxide.run_tables

The tables of a photonoxide run, from its record.

```matlab
t = photonoxide.run_tables(run_dir)
```

t = photonoxide.run_tables(run_dir) reads the run in run_dir (run.dir of
photonoxide.run_job) and returns a struct of its tables: t.modes, t.sweep,
t.s_parameters, t.spectra, t.resonances, t.study, t.history, t.static, t.transient, those
the run has. Each is a MATLAB table whose variables are its CSV file's columns: numbers as
doubles (NaN where the table has none), text as strings. Every number is the one the run
recorded, to the bit.

| Argument | Size | Class | Default |
|---|---|---|---|
| `run_dir` | scalar | string | required |

## photonoxide.slab_modes

The guided modes of a three-layer slab, solved exactly.

```matlab
m = photonoxide.slab_modes(below, core, above, thickness_um, polarization, wavelength_um, x_um)
```

m = photonoxide.slab_modes(below, core, above, thickness_um, polarization, wavelength_um)
are the guided modes of one polarization ("te" or "tm") of a core of index core and
thickness thickness_um (micrometres) between indices below and above, at wavelength_um:
m.order and m.effective_index, a row each, fundamental first. Empty when it guides none.

m = photonoxide.slab_modes(..., x_um) adds m.field, their fields at x_um (0 at the core's
bottom; E_y for TE, H_y for TM, unnormalized), a column per mode.

| Argument | Size | Class | Default |
|---|---|---|---|
| `below` | scalar | double | required |
| `core` | scalar | double | required |
| `above` | scalar | double | required |
| `thickness_um` | scalar | double | required |
| `polarization` | scalar | string | required |
| `wavelength_um` | scalar | double | required |
| `x_um` | any | double | `[]` |

## photonoxide.stacks

The stacks of photonoxide's catalogue, by the name the device functions take.

```matlab
s = photonoxide.stacks()
```

s = photonoxide.stacks() is a struct array, one element per stack: s(k).name (what the
device functions take as stack, e.g. "soi_220"), s(k).about, s(k).substrate, s(k).layers (a
struct array, bottom to top: name, thickness_um, material, background), s(k).cladding and
s(k).core, the layer a device draws its guides on, as thick as the device's thickness. The
materials are the catalogue's ids, which photonoxide.refractive_index takes.

## photonoxide.vector_modes

Full-vector modes of a waveguide's cross-section, with their fields.

```matlab
modes = photonoxide.vector_modes(x_um, y_um, permittivity, wavelength_um, options)
```

modes = photonoxide.vector_modes(x_um, y_um, permittivity, wavelength_um) is the
fundamental mode of the cross-section on the grid with nodes x_um (nx + 1 of them,
increasing, micrometres) and y_um (ny + 1), each cell's relative permittivity in
permittivity, ny-by-nx: permittivity(j, i) fills the cell between x_um(i), x_um(i + 1),
y_um(j) and y_um(j + 1); real, or complex with a positive imaginary part for loss.

Each mode has effective_index (complex), te_fraction (near 1 for a TE-like mode), x_um and
y_um (the cells' centres) and its fields ex, ey, ez, hx, hy and hz, each ny-by-nx: ex(j, i)
is E_x at (x_um(i), y_um(j)).

Options: count (how many modes, 1), near_index (the modes nearest this effective index; the
highest index of the cross-section by default), boundaries (the west, east, south and north
edges: "zero", "electric" or "magnetic"), pml_um (PMLs inside the same edges, micrometres)
and pml_strength, and timeout_s (600; past it, the error photonoxide:Timeout).

Of the count modes solved, only the guided ones come back: effective index above the
highest index of the media at the window's open ("zero") edges. Below it the closed window
has modes of its own, not the guide's, and they are left out, so fewer than count modes
come back when fewer are guided, and none when none is. With PMLs (pml_um not all 0,
pml_strength above 0) the window is open, and every mode solved comes back, leaky ones
with their loss.

| Argument | Size | Class | Default |
|---|---|---|---|
| `x_um` | any | double | required |
| `y_um` | any | double | required |
| `permittivity` | any | double | required |
| `wavelength_um` | scalar | double | required |
| `count` (name-value) | scalar | double | `1` |
| `near_index` (name-value) | scalar | double | none |
| `boundaries` (name-value) | 1-by-4 | string | `["zero" "zero" "zero" "zero"]` |
| `pml_um` (name-value) | 1-by-4 | double | `[0 0 0 0]` |
| `pml_strength` (name-value) | scalar | double | `0` |
| `timeout_s` (name-value) | scalar | double | `600` |

## photonoxide.version

The photonoxide package's version, and photonoxide's: v.package and v.core.

```matlab
v = photonoxide.version()
```

v = photonoxide.version()

## photonoxide.write_touchstone

Writes S-matrices to a Touchstone file.

```matlab
photonoxide.write_touchstone(file, spectrum, convention, options)
```

photonoxide.write_touchstone(file, spectrum, convention) writes spectrum, a struct with
ports (names), wavelength_um and s (nl-by-n-by-n, photonoxide's convention), as
photonoxide.circuit_spectrum and photonoxide.read_touchstone return it, to file as
Version 2.0, its values in the time convention convention ("physics" or "engineering").
Each number is the shortest decimal that reads back to the same double, or, with
significant_digits=d, d significant digits (1 to 17).

| Argument | Size | Class | Default |
|---|---|---|---|
| `file` | scalar | string | required |
| `spectrum` | scalar | struct | required |
| `convention` | scalar | string | required |
| `significant_digits` (name-value) | scalar | double | none |

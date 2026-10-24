---
title: "Exporting a run's results"
module: export
summary: "Every run writes its tables (n_eff, S-parameters, spectra, studies, histories, static numbers) as CSV into its directory, and a job's [task.export] adds the complex fields as NumPy's .npz, so a result can be used outside photonoxide, not just drawn."
order: 26
---

Everything `photonoxide run` computes is recorded as `events.jsonl`, whose fields are pictures
for the studio: `Raster` values in `f32`, scaled to a peak of 1, as $\lvert E\rvert^2$ or one
component's real part — enough to draw, not enough to compute with (no phase, no H, no
normalization). `photonoxide::export` writes the numbers themselves, into the run's directory
beside `events.jsonl` and `results.json`.

## What a run writes

**By default, with no `[task.export]`:** every table the run has, as CSV, and no fields. The
tables are small; the fields can be large, and are written only when asked.

```yaml
task:
  export:
    format: both        # "csv" (the tables), "npz" (the fields), "both", or "none";
                        # "both" when fields is given, "csv" otherwise
    tables: [modes, s_parameters]   # which tables; every one the run has by default
    fields: [Ex, Ey, Hz]            # which fields; every one the kind computes by default
    precision: f32      # "f32" or "f64" (the default): the fields' width
    level: 6            # deflate's level, 0 (store) to 9 (smallest); 6 by default
```

A job's check refuses a table or a field it doesn't know, fields with `format = "csv"`, tables
with `format = "npz"`, and fields its kind doesn't compute (an fdfd job in TE solves $H_z$, not
$E_z$; a device's study, fit or optimization and a chip have tables only).

## The tables

One file a table, a header line then one line a row, in long form: one value a row with its
coordinates beside it, so any size of table reads into pandas, MATLAB's `readtable` or a
spreadsheet as it is. Each number is the shortest decimal that reads back to the same `f64`
(lossless); a text with a comma or a quote is quoted (RFC 4180); an empty field is a number
the record doesn't have.

| File | Columns | Written by |
|---|---|---|
| `modes.csv` | label, wavelength_um, n_eff_re, n_eff_im, loss_db_per_cm, te_fraction | modes |
| `sweep.csv` | point, parameter, value, wavelength_um, mode, n_eff_re, n_eff_im, te_fraction | a modes job's sweep |
| `s_parameters.csv` | wavelength_um, to, from, s_re, s_im | fdfd, a device's spectrum, a chip |
| `spectra.csv` | monitor, kind, quantity, series, wavelength_um, value | fdtd |
| `resonances.csv` | monitor, component, wavelength_um, frequency, q, decay, amplitude, error | fdtd |
| `study.csv` | point, corner, each parameter, error, wavelength_um, to, from, s_re, s_im | a device's or a chip's sweep or corners |
| `history.csv` | index, each variable, `cost:` and `value:` each objective, `g:` each constraint, violation, gradient_norm, best, solves, error | an optimization |
| `static.csv` | physics, name, value, unit | static |
| `transient.csv` | time_us, each probe's mean | a static job's transient |

The tables come from the run's own record when it ends (`export::tables::write_tables`, which
also reads a finished run's directory again), so every number is the one its event recorded, to
the bit.

## The fields

NumPy's own `.npz`, a ZIP of `.npy` arrays (NEP 1), deflated. The grid and the small numbers
are always `f64`; the fields are `f64` or, with `precision = "f32"`, half the size, each number
downcast. Each `.npz` holds `meta.json` too, a plain member `numpy.load`'s `NpzFile` never
mistakes for an array (it only reads members ending in `.npy`): photonoxide's version, the job's
name and its text as it ran, the run's id, the precision, and where the values sit, their units
and their normalization.

| File | Arrays | Grid | Normalization |
|---|---|---|---|
| `mode-<k>-of-<n>.npz` (modes) | `Ex` … `Hz` `(ny, nx)`; `effective_index`, `te_fraction`, `power`, `wavelength_um` | `x_um`, `y_um`: the cells' centres ([`Fields`](fields.md)) | the solver's own amplitude; `power` is $\tfrac12\mathrm{Re}\int \mathbf E\times\mathbf H^*\cdot\hat z\thinspace dA$, divide by its root for unit power |
| `field.npz` (fdfd) | `Hz` (TE) or `Ez` (TM) `(ny, nx)`, as $\tilde H_z = Z_0 H_z$; `wavelength_um` | `x_um`, `y_um`: the cells' centres, the whole window with its PMLs | the total field port 1's mode launches at unit amplitude, the S-parameters' own source |
| `field-<k>.npz` (fdtd, per field monitor) | each component of E and $\tilde H = Z_0 H$ the run computes, `(wavelengths, ny, nx)` | `x_um`, `y_um`: the cells' corners, `z_um` the plane; each component half a cell off as Yee places it ($E_x$ at $(i+\tfrac12, j, k)$, $H_x$ at $(i, j+\tfrac12, k+\tfrac12)$, and so on) | none: the transform $\sum_n F(t_n)\,e^{i\omega t_n}\Delta t$, not divided by the source's spectrum ([monitors](fdtd.md)) |
| `static.npz` (static) | `temperature` (K), `potential` (V), `(nz, ny)` | `y_um`, `z_um`: the cells' centres; `y_nodes_um`, `z_nodes_um`: their edges, the grid graded | none: the solver's values |

Every field is in C order, the last index fastest: row $j$ at `y_um[j]`, column $i$ at
`x_um[i]`.

From Python, with the `photonoxide` package ([python](../python.md)): a run's tables as
columns, and its fields as NumPy reads them.

```python
import numpy as np
import photonoxide as phox

job = {
    "name": "strip",
    "task": {
        "kind": "modes", "stack": "soi_220", "wavelength_um": 1.55, "layer": "Si",
        "propagation": "x", "y_um": [-1.0, 1.0], "step_nm": 50.0, "modes": 1,
        "rect": [{"layer": "Si", "center_um": [0.0, 0.0], "size_um": [10.0, 0.5]}],
        "export": {"fields": ["Ey", "Hz"], "precision": "f32"},
    },
}
run = phox.run_job(job, runs_dir="runs")
modes = phox.run_tables(run.dir)["modes"]          # also run.dir / "modes.csv"
f = np.load(run.dir / "mode-1-of-1.npz")
ey = f["Ey"] / np.sqrt(f["power"])                 # unit power
assert ey.shape == (len(f["y_um"]), len(f["x_um"]))
assert f["effective_index"].real == modes["n_eff_re"][0]
```

MATLAB has `photonoxide.run_tables` (each table a MATLAB `table`) and `photonoxide.export_run`
([matlab](../matlab.md)).

## Tests

photonoxide reads a `.npz` back only to write it again (below); the tests read every one, with
the `zip` crate (a dev-dependency only) as a real reader and a small `.npy` parser of their own,
to the bit:

- a mode's `.npz` against the solver's own `Fields` and `VectorMode`;
- an fdfd job's `field.npz`: its $\lvert H_z\rvert^2$ over the pictured cells, scaled as the
  run's [`Event::Field`] scales it, is that picture to the bit; its `s_parameters.csv` is every
  `SParameters` event's numbers;
- an fdtd job's `field-1.npz`: its $\lvert E\rvert^2$, the same against the run's picture;
- a static job's `static.npz`: its largest temperature is the run's own "largest", and
  `static.csv` its numbers;
- a device's study: `study.csv` is every point's S-parameters and parameters.

The ZIP is a minimal writer of the module's own (PKWARE's *APPNOTE*), deflating with `flate2`'s
pure-Rust backend: nothing written in C is linked.

## Zstandard

Deflate is photonoxide's own and the default, so every export works with nothing installed.
Zstandard (Collet and Kucherawy, RFC 8878, doi:10.17487/RFC8878) compresses better and
faster, but its main implementation is written in C, which photonoxide never links: with
`compression = "zstd"` the run uses **the user's own libzstd**, found and loaded at run time by
`photonoxide-native` ([installing it](../libraries.md#zstandard)).

```yaml
task:
  export:
    fields: [Ex, Ey, Hz]
    compression: zstd   # "deflate" (the default) or "zstd"
    level: 9            # zstd's 1 to 22, 3 by default
```

- Each member of the `.npz` is one Zstandard frame, stored as ZIP method 93. Python's
  `zipfile`, and so `numpy.load`, reads it from Python 3.14; checked with NumPy 2.5.4 there, its
  arrays the same bytes as the deflate file's. Older Pythons don't, so deflate stays the
  choice for files to share.
- `meta.json` names the compression and the library's version.
- The same frames from the same library on every run: its one-shot, single-threaded
  compression depends on nothing but the input and the level.
- The check refuses `"zstd"` where libzstd isn't found, saying why, rather than failing the
  run at its end. The `photonoxide` program looks for it as it starts a job; the Python
  package doesn't load external libraries, so its runs take deflate.
- `photonoxide-native`'s tests (`cargo test -p photonoxide-native --test zstd`) run a modes
  job both ways and read each zstd member back through libzstd: the bytes of the stored file,
  photonoxide's own path, every one. They skip, saying so, where libzstd isn't found.

## HDF5

NumPy's `.npz` is photonoxide's own and the default. For tools that read HDF5 (h5py, MATLAB's
`h5read`, ParaView, HDFView), `container = "hdf5"` writes each `.npz` as an `.h5` instead, with
**the user's own libhdf5** (release 1.10 or later), loaded at run time by `photonoxide-native`
([installing it](../libraries.md#hdf5)); libhdf5 is written in C, which photonoxide never
links.

```yaml
task:
  export:
    fields: [Ex, Ey, Hz]
    container: hdf5     # "npz" (the default) or "hdf5"
    level: 6            # libhdf5's deflate filter, 0 for none
```

- One dataset an array, by the `.npz`'s names, in the same C order and shape; `f64` or `f32` as
  IEEE little-endian; a complex array as the compound `{r, i}`, which h5py reads as complex.
- `meta.json`'s text is the root's string attribute `meta`.
- Deflated by libhdf5's own filter, each array one chunk. Zstandard isn't taken here (HDF5's
  is a plugin of its own).
- No modification times are kept, so the same arrays from the same library give the same
  file.
- `photonoxide-native`'s tests (`cargo test -p photonoxide-native --test hdf5`) run a modes
  job both ways and read every array of the `.h5` back through libhdf5: the shape and the bytes
  of the stored `.npz`'s, every one. They skip, saying so, where libhdf5 isn't found.

## A finished run

A run's tables can be written again, or ones its job didn't ask for, from its record:

```sh
photonoxide export runs/<run> --list                         # the tables it has, its files
photonoxide export runs/<run> --tables modes,s_parameters --to out/
```

The studio's run view has the same as its **Export…** dialog: the tables and the run's `.npz`
files by checkbox, with their sizes, written into the run's folder or saved into one of yours.
Its fields can't be made afterwards: a field is the solver's state, which a finished run no
longer has, so a job asks for them in `[task.export]`.

### Its fields at another precision and level

The fields a run wrote can be written again, into another folder, at another precision and
deflate level, each file's size estimated before writing:

```sh
photonoxide export runs/<run> --list --precision f32 --level 9   # each file's size, and written again so
photonoxide export runs/<run> --fields all --precision f32 --level 9 --to out/
```

```text
field-1.npz            20125593 bytes  written again: about 9694361 bytes at f32, level 9
```

The Export dialog has the precision and the level beside its fields, each file's size now and
written again (`19.2 MB → ≈ 9.2 MB`) as the choice changes, and the total; Python and MATLAB have
`estimate_fields` and `export_fields`.

- The arrays are read from the `.npz` the run wrote and written by the same writer, in the same
  order. The grid and the small numbers (no or one dimension) stay `f64`; the fields (two
  dimensions or more) are downcast to `f32` when asked, and `meta.json` says so and names the
  new compression. A file already `f32` stays `f32`: nothing brings its digits back.
- Written again at its own precision and level, a file is the same file, byte for byte.
- **The estimate.** Stored (level 0), the size is known before writing to the byte: each
  member's `.npy` and the ZIP's own records. Deflated, a member up to 1 MiB is deflated in
  memory as the writer would, so the size is exact too, and it says so; a larger member (a 3D
  run's monitor) is estimated from eight slices of 64 KiB spread over it, each deflated at the
  level asked, their ratio taken for the whole, and marked `≈` (`about` on the command line).
- The tests write each kind's file again (a mode, an fdfd field, an fdtd monitor, a static
  temperature) at both precisions and levels 0, 1, 6 and 9: each estimate is the written size
  to the byte where it says it is exact, and within 2 % where it is sliced.
- `jobs/mmi-fdtd.yaml`'s field monitor (20.1 MB in `f64`, on its 20 nm grid) written again:
  the estimate 0.9 % under at `f64` and level 6 (19 948 345 bytes for 20 125 593 written),
  1.0 % under at level 1, 0.1 % over at `f32` and level 6 or 9 (9 694 361 for 9 684 081);
  stored, 20 893 269 bytes, to the byte.
- An `.h5`, or an `.npz` compressed with Zstandard (which photonoxide reads only through
  libzstd), is copied as it is.

## The run's arrays

`events.jsonl`'s pictures are large: a mode's or a field's `Raster` is one `f32` a cell, and
an FDTD run records up to 500 frames. Written into each line, they made most of the record,
and every line had to be read and parsed to open a run. Each long value of a line now goes into
**`events.bin`** beside it, and the line keeps where it is ([`run::arrays`]):

- a flat array of 256 numbers or more, as `<f4` when every number is an `f32`'s shortest
  decimal, as `<f8` when it is an `f64`'s; a base64 string of 1024 characters or more (an
  FDTD frame's pixels), as its bytes, `|u1`;
- each is a NumPy `.npy` (NEP 1, one dimension), one after another, and its place in the line
  holds `{"$npy":<offset>}`, the byte where it starts;
- a value is moved only when writing it back gives the very text the line had, which is checked
  as it is moved. A replayed run (`photonoxide view`, the studio's replay, `results.json`, the
  tables) reads each line as the run recorded it, to the last byte, and so every float to the
  last bit;
- the array is written before the line that names it, so the studio following a run live
  finds it;
- a run recorded before, with its arrays in its lines, replays as it always did.

| Job | `events.jsonl` before | `events.jsonl` now | `events.bin` |
|---|---:|---:|---:|
| `jobs/strip-modes.yaml` (its modes, and a sweep of 11 wavelengths) | 1.59 MB | 16.7 kB | 0.86 MB |
| `jobs/ring-fdfd.yaml` (a sweep of 201 wavelengths, each one's field) | 16.8 MB | 178 kB | 13.8 MB |
| `jobs/ring-fdtd.yaml` (its frames) | 8.75 MB | 1.45 MB | 5.52 MB |

The studio's list of runs reads each record, now a hundredth of what it was for a sweep; a
replay reads the arrays back with its lines.

From Python, a picture of the run above, read from the record as NumPy reads it; the run's
`events` (as the façade gives them) have it in place again, to the bit:

```python
import json

lines = (run.dir / "events.jsonl").read_text(encoding="utf-8").splitlines()
k = next(k for k, line in enumerate(lines) if '{"$npy":' in line)
event = json.loads(lines[k])
key = next(name for name, v in event.items() if isinstance(v, dict) and isinstance(v.get("values"), dict))
with open(run.dir / "events.bin", "rb") as f:
    f.seek(event[key]["values"]["$npy"])
    values = np.load(f)                               # float32, a cell each, row by row
assert values.shape == (event[key]["nx"] * event[key]["ny"],)
assert np.array_equal(values, np.array(run.events[k][key]["values"], dtype=np.float32))
```

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

Nothing in photonoxide reads a `.npz` back: the tests do, with the `zip` crate (a
dev-dependency only) as a real reader and a small `.npy` parser of their own, to the bit:

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

## Left for later

Issue [#431](https://github.com/tachsin/photonoxide/issues/431) asks for more: MAT and VTK XML
writers, HDF5 and zstd through `photonoxide-native` (the user's own libhdf5 and libzstd loaded
at run time, never linked; deflate stays the pure-Rust default), and the heavy arrays moved out
of `events.jsonl`. This page grows with them.

---
title: "Exporting a run's results"
module: export
summary: "A job's [task.export] writes a solved mode's complex fields as an .npz or a summary table as csv into the run's directory, so a result can be used outside photonoxide, not just drawn."
order: 26
---

Everything `photonoxide run` computes is recorded as `events.jsonl`, whose fields are pictures
for the studio: `Raster` values in `f32`, scaled to a peak of 1, as $\lvert E\rvert^2$ or one
component's real part — enough to draw, not enough to compute with (no phase, no H, no
normalization). `photonoxide::export` writes the fields themselves.

## What a `"modes"` job's `[task.export]` writes

```toml
[task.export]
format = "both"       # "npz" (the fields), "csv" (the summary table), or "both"
precision = "f32"     # "f32" or "f64" (the default): the npz's fields' width
level = 6              # deflate's level, 0 (store) to 9 (smallest); 6 by default
```

Into the run's directory, beside `events.jsonl`:

- **`<label>.npz`** (e.g. `mode-1-of-2.npz`), one per mode: NumPy's own format, a ZIP of `.npy`
  arrays (NEP 1). `x_um`, `y_um` (the cells' centres); `wavelength_um`, `effective_index`
  (complex), `te_fraction`, `power` (half the real part of $\int \mathbf E\times\mathbf H^*\cdot\hat z\thinspace dA$,
  [`mode::fields::Fields::power`](fields.md)'s units); `Ex`, `Ey`, `Ez`, `Hx`, `Hy`, `Hz`, each
  `(ny, nx)` in C order, the solver's own units (E in units of $Z_0$ times H's), not scaled to
  unit power; and `meta.json`, a plain member naming photonoxide's version, the precision
  written, and the job and run — `numpy.load`'s `NpzFile` never mistakes it for an array, since
  it only reads members ending in `.npy`.
- **`modes.csv`**, one row a mode: `label`, `wavelength_um`, `n_eff_re`, `n_eff_im`,
  `loss_db_per_cm`, `te_fraction`, each number at full `f64` precision (17 significant digits:
  lossless).

The `.npz` is a minimal ZIP writer of this module's own (PKWARE's *APPNOTE*), deflating with
`flate2`'s pure-Rust backend; nothing here reads a `.npy` or `.npz` file back. Its own tests do,
with the `zip` crate (a dev-dependency only) as a real reader and a small parser of this module's
own, checking every array against the solver's own [`Fields`](fields.md) and `VectorMode`, to the
bit.

## Left for later PRs

Issue [#431](https://github.com/tachsin/photonoxide/issues/431) asks for more: FDFD and FDTD's
fields and S-parameters, the static solvers' temperature and potential, MAT (MATLAB) and VTK XML
writers, the studio's Export dialog, `photonoxide export`, and the façade's own `export` for
Python and MATLAB. This page will grow with them.

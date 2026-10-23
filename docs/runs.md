# A run's folder

What `photonoxide run <job> [--headless]` leaves behind, for a script in any language to read.

```sh
photonoxide run jobs/strip-modes.yaml --headless --timeout 30
```

prints the run's folder on its first line of output (`runs/20261014-120000-strip-modes/`; `--out
<dir>` puts it elsewhere) and exits when the run ends.

## Exit codes

| Code | The run |
|---:|---|
| 0 | finished |
| 1 | failed: the error is on stderr, in `results.json` and in the record's last event |
| 2 | wasn't started: the command line was wrong |
| 3 | stopped by its timeout: the job's `timeout_minutes` or `--timeout`, whichever is shorter |
| 4 | stopped on request: its window was closed before it finished |

A stopped run's folder holds what was done before it stopped.

## The files

| File | What it holds |
|---|---|
| `job.yaml`, `job.toml` or `job.json` | the job, as it was given |
| `meta.json` | what ran it: `photonoxide` (the version), `job` (its name), `started` (UTC, `2026-10-01T12:00:00Z`), `threads` (the machine's) |
| `events.jsonl` | the record, one JSON event a line, written as the run goes: what the studio's views show |
| `results.json` | the run's numbers, written when it ends however it ends: what a script reads |
| `*.f64`, `*.c128` | `results.json`'s large arrays, as raw numbers |

## `results.json`

```json
{
  "schema": 1,
  "photonoxide": "0.5.3",
  "job": "strip-fdfd",
  "kind": "fdfd",
  "outcome": "finished",
  "stopped": null,
  "error": null,
  "seconds": 0.151,
  "grids": [{ "module": "fdfd", "cells": [70, 70], "step_um": 0.04, "unknowns": 4900, "details": [["PML", "20 cells on each side, inside the window"]] }],
  "modes": [],
  "sweep": null,
  "s_parameters": {
    "ports": ["1 (left, x = -0.4 um)", "2 (right, x = 0.4 um)"],
    "wavelengths_um": [1.5, 1.6],
    "effective_indices": { "dtype": "float64", "shape": [2, 2], "unit": "1", "data": [2.56, 2.56, 2.46, 2.46] },
    "s": { "dtype": "complex128", "shape": [2, 2, 2], "unit": "1", "data": [-4.4e-16, 4.6e-16, -0.67, 0.75] }
  },
  "spectra": [],
  "resonances": []
}
```

(The two-port strip from the library's job tests, its numbers shortened, its `details` and the `s` array's data cut short.)

| Field | What it is |
|---|---|
| `schema` | 1. A change that removes or changes a field raises it; one that adds a field doesn't. Check it. |
| `photonoxide` | the version that ran the job |
| `job`, `kind` | the job's name, and its kind: `structure`, `modes`, `fdfd`, `fdtd` or `device` |
| `outcome` | `finished`; `stopped` (then `stopped` says `timeout` or `requested`); `failed` (then `error` has the error's text); `incomplete`, a record that just ends (the program was killed) |
| `seconds` | how long it ran; `null` if it was cut off |
| `grids` | each grid a solver ran on: `module`, `cells` along its two axes, `step_um`, `unknowns`, and what else it says (`details`, pairs of strings) |
| `modes` | each mode found: `label`, `wavelength_um`, `effective_index` ([real, imaginary]), `te_fraction` (near 1 for TE-like), `cut_um` (where along the guide its cross-section was cut) |
| `sweep` | a sweep's `parameter` (`wavelength`, `width` or a parameter's name) and its `points`, in order, each with its `value` (µm for the wavelength, a width or a length; radians for an angle), `wavelength_um`, `effective_indices` (highest first, [real, imaginary] each) and `te_fractions` |
| `s_parameters` | a 2D FDFD run's `ports`, its `wavelengths_um`, each port mode's `effective_indices` (shape [wavelengths, ports]) and the power-normalized S-matrix `s` (shape [wavelengths, ports, ports]: `s[k][q][p]` is from port p into port q at wavelength k) |
| `spectra` | each FDTD monitor's spectrum as it stood at the end: `monitor`, `kind`, `quantity`, `wavelengths_um`, the series' `labels`, their `values` (shape [series, wavelengths]) and `time_um` (how far the run had gone, µm of c t) |
| `resonances` | each FDTD monitor's resonances by harmonic inversion: `monitor`, `position_um`, `component`, and each resonance's `wavelength_um`, `frequency`, `q`, `decay`, `amplitude` and `error` |

Every number is the one photonoxide computed, written so that it reads back to the same bits:
the S-matrix is bit for bit what the library's `job::fdfd_s_parameters` gives for the same job.

### Arrays

An array is an object:

- `dtype`: `float64`, or `complex128`, each value its real part and then its imaginary part;
- `shape`: its dimensions, in C order (the last index changes fastest);
- `unit`: `1` for a plain number, or as its field says;
- `data`: its values, a flat list (twice the shape's product of them for `complex128`); or, for
  an array of more than 4 096 values, none, and
- `file`: the file beside `results.json` holding the same values as little-endian doubles.

Python, with NumPy:

```python
import json, pathlib
import numpy as np

run = pathlib.Path("runs/20261014-120000-strip-fdfd")
results = json.loads((run / "results.json").read_text())
assert results["schema"] == 1 and results["outcome"] == "finished"

def array(a):
    values = np.array(a["data"]) if "data" in a else np.fromfile(run / a["file"], "<f8")
    if a["dtype"] == "complex128":
        values = values.view(np.complex128)
    return values.reshape(a["shape"])

s = array(results["s_parameters"]["s"])   # s[k, q, p]
transmission = abs(s[:, 1, 0]) ** 2
```

MATLAB:

```matlab
run = "runs/20261014-120000-strip-fdfd";
results = jsondecode(fileread(fullfile(run, "results.json")));
a = results.s_parameters.s;
if isfield(a, "data")
    values = a.data(:);
else
    f = fopen(fullfile(run, a.file), "r", "ieee-le");
    values = fread(f, Inf, "double");
    fclose(f);
end
if a.dtype == "complex128"
    values = complex(values(1:2:end), values(2:2:end));
end
% C order: reverse the shape for MATLAB's column order, then the indices
s = permute(reshape(values, flip(a.shape')), numel(a.shape):-1:1);   % s(k, q, p)
```

## `events.jsonl`

One event a line, each a JSON object with its `type`; a reader skips the types it doesn't know,
since new ones are added. The record starts with `started` and ends with `finished` or `failed`
(`{"type": "failed", "error": "...", "seconds": 0.4}`); a record with neither was cut off, and its
last line may be cut off mid-write. The events' pictures are for the studio's views: `f32`,
normalized to a peak of 1. For the numbers, read `results.json`. The events are documented
field by field in the library's `job::Event`.

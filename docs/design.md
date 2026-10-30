# Designs as job files

The designs people start from, a strip waveguide's modes, a ring's spectrum and a directional
coupler, and then a stack of one's own: each as its job file, the file in `jobs/` exactly as
the repository runs and checks it, and beside it the same design through the Python package.
Each comes with its layers, its shapes, its materials and what is solved, and how.

Layers are listed from the top down: the bottom one, the substrate, is the first laid down in
fabrication. A job file's stack of its own lists them the other way, bottom to top, the order
they are laid down in (the last design shows one).

A job file runs from the command line, `photonoxide run jobs/<file>.yaml`, or in the studio,
which shows the run as it goes. In Python, `run_job` runs the same file and `run_tables` reads
its results ([docs/python.md](python.md) has the package). The numbers below are photonoxide's,
from these files and calls.

## The stack: 220 nm silicon-on-insulator

The first three designs are drawn on `soi_220`, the standard platform: 220 nm of silicon on
2 µm of buried oxide, under an oxide cladding (L. Chrostowski, M. Hochberg, *Silicon Photonics
Design*, Cambridge University Press (2015), Fig. 3.1).

| Layer | Thickness | Material | n at 1.55 µm | Model and source |
|---|---|---|---|---|
| cladding | above the stack | silica (`sio2`) | 1.444024 | Sellmeier, I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965), 0.21 to 3.71 µm |
| Si | 220 nm | silicon (`si`) where a shape is drawn, silica elsewhere | 3.4757 | a table, H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), 293 K, 1.2 to 14 µm, ±2 × 10⁻³ |
| BOX | 2 µm | silica | 1.444024 | as the cladding |
| substrate | below the stack | silicon | 3.4757 | as the Si layer |

The layer `Si` runs from z = 2 to 2.22 µm, the buried oxide from z = 0 to 2 µm. Python lists
the stack's layers as the library keeps them, bottom to top:

```python
import photonoxide as phox

soi = next(s for s in phox.stacks() if s.name == "soi_220")
print("cladding:", soi.cladding)
for layer in reversed(soi.layers):              # from the top down
    print(f"{layer.name}: {layer.thickness_um} um of {layer.material}")
print("substrate:", soi.substrate)
for material in ("sio2", "si"):
    print(material, phox.refractive_index(material, wavelength_um=1.55).real)
```

## A strip waveguide's modes

A 500 × 220 nm silicon strip along x, its two lowest modes at 1.55 µm and their effective
indices from 1.5 to 1.6 µm.

```yaml
# The modes of a 500 x 220 nm silicon strip along x, and their effective and group indices from
# 1.5 to 1.6 um, live in the studio.
#   photonoxide run jobs/strip-modes.yaml
name: strip-modes
timeout_minutes: 10

task:
  kind: modes
  stack: soi_220
  wavelength_um: 1.55
  layer: Si
  propagation: x           # the modes travel along x, cut at x = 0 (cut_x_um)
  y_um: [-1.2, 1.2]        # the window across the guide
  step_nm: 20.0
  modes: 2

  rect:
    - layer: Si
      center_um: [0.0, 0.0]
      size_um: [10.0, 0.5]

  sweep:
    parameter: wavelength
    from: 1.5
    to: 1.6
    points: 11
```

```python
import photonoxide as phox

run = phox.run_job("jobs/strip-modes.yaml")     # the file above, into runs/
modes = phox.run_tables(run.dir)["modes"]       # at 1.55 um; the sweep is run_tables(...)["sweep"]
print(modes["n_eff_re"], modes["te_fraction"])

# the same strip as the waveguide device, at its 3D model (its own mesh, about 10 nm)
figures = {f.symbol: f for f in phox.device_derived("waveguide", values={"width": 0.5},
                                                    fidelity="3D")}
print(figures["n_eff"].value, figures["n_g"].value, figures["n_eff"].grid)
```

| Shape | Layer | Centre | Size |
|---|---|---|---|
| the strip, a rectangle | Si | (0, 0) µm | 10 µm along x, 500 nm across |

- **Solved:** the full-vector finite-difference mode solver on the cross-section at x = 0, two
  modes; then again at 11 wavelengths from 1.5 to 1.6 µm.
- **Window and grid:** y from −1.2 to 1.2 µm; z 1 µm below and above the Si layer (the job's
  default when it gives no `z_um`); a uniform 20 nm grid.
- **Tolerance:** the job reports, it checks nothing. The example `strip_waveguide` checks the
  same strip, on the book's media, within 3 × 10⁻³ of the book's 2.443.

| Model | Grid | TE-like n_eff at 1.55 µm | TE fraction | TM-like n_eff | n_g |
|---|---|---|---|---|---|
| the modes job | 20 nm | 2.44980 | 0.974 | 1.78531 | |
| the waveguide device, 3D | 125 × 111 cells, 10 nm | 2.44511 | | | 4.1893 |

## A ring's spectrum

An all-pass ring: a 500 nm guide bent to a radius of 2 µm, 100 nm from its bus, its
transmission from 1.50 to 1.60 µm. A 2D estimate by the effective index method, not the
device's 3D performance.

```yaml
# An all-pass ring resonator on 220 nm SOI by 2D FDFD with the effective index method (an
# estimate, not the device's 3D performance): a 500 nm ring of 2 um radius, 0.1 um from its bus,
# swept from 1.50 to 1.60 um. The bus's transmission dips where the ring resonates, a free
# spectral range of about lambda^2 / (n_g L) apart. The spectrum builds up live, a point at a time;
# the field is the one at the first resonance, 1.5205 um, where the light circles the ring.
#   photonoxide run jobs/ring-fdfd.yaml
name: ring-fdfd
timeout_minutes: 20

task:
  kind: fdfd
  stack: soi_220
  wavelength_um: 1.55
  layer: Si
  polarization: te
  x_um: [-4.5, 4.5]
  y_um: [-2.9, 3.5]
  step_nm: 25.0
  field_um: 1.5205

  # the bus
  rect:
    - layer: Si
      center_um: [0.0, -2.0]
      size_um: [9.0, 0.5]

  # the ring: its outer edge 0.1 um above the bus
  ring:
    - layer: Si
      center_um: [0.0, 0.6]
      radius_um: 2.0
      width_um: 0.5

  port:
    - x_um: -3.5
      side: left
      y_um: [-2.9, -1.0]

    - x_um: 3.5
      side: right
      y_um: [-2.9, -1.0]

  sweep:
    parameter: wavelength
    from: 1.5
    to: 1.6
    points: 201
```

```python
import numpy as np
import photonoxide as phox

s = phox.fdfd_s_parameters("jobs/ring-fdfd.yaml")   # the file above: 201 wavelengths, 25 nm cells
through = np.abs(s.s[:, 1, 0]) ** 2                 # from port 1 into port 2
print(s.wavelength_um[np.argmin(through)], through.min(), s.cell_um)

# the same ring as the all-pass ring device, at its analytic model
ring = phox.device_spectrum("all-pass ring", values={"radius": 2.0, "gap": 0.1},
                            fidelity="analytic", wavelength_um=s.wavelength_um)
figures = {f.symbol: f.value for f in phox.device_derived(
    "all-pass ring", values={"radius": 2.0, "gap": 0.1}, fidelity="analytic")}
print(figures["FSR"], figures["resonance"], figures["Q"])
```

| Shape | Layer | Centre | Size |
|---|---|---|---|
| the bus, a rectangle | Si | (0, −2.0) µm | 9 µm along x, 500 nm across |
| the ring | Si | (0, 0.6) µm | radius 2 µm to the guide's centre, 500 nm wide; 100 nm from the bus |

- **Solved:** 2D FDFD, TE, on the plane the effective index method makes of the stack (its slab
  index from the Si layer's slab modes, at each wavelength); the bus's mode launched at the left
  port (x = −3.5 µm) and measured at the right (x = 3.5 µm); 201 wavelengths from 1.50 to
  1.60 µm.
- **Window and grid:** x from −4.5 to 4.5 µm, y from −2.9 to 3.5 µm, 25 nm cells.
- **Tolerance:** none: a 2D estimate. The examples `ring_q_factor` and `ring_liu_poon` check a
  ring's closed forms and its 3D FDTD against published numbers.

| Model | Grid | Resonances in the band (power through) | Free spectral range |
|---|---|---|---|
| the FDFD job, 2D | 25 nm cells | 1.5205 µm (0.101), 1.5675 µm (0.204) | 47.0 nm |
| the all-pass ring device, analytic | its fit to the full-vector solver, about 10 nm | 1.5419 µm, Q 13 600 | 45.8 nm |

The two models differ by what they solve: the 2D job's guide is the effective index method's
slab, the device's analytic model the 3D strip's index, fitted to the full-vector solver.

## A directional coupler

Two 500 × 220 nm strips side by side, `gap` apart: their two highest-index modes, the even and
odd supermodes, at 1.55 µm as the gap closes from 400 to 100 nm. The light crosses from one
strip to the other over L_x = λ / (2 (n_even − n_odd)).

```yaml
# A directional coupler's coupled section: two 500 nm silicon strips, 220 nm thick in oxide, a
# gap apart, and the pair's two highest-index modes, its even and odd supermodes, at 1.55 um, as
# the gap closes from 400 to 100 nm. Their effective indices part as the strips come together,
# and the light crosses from one strip to the other over L_c = wavelength / (2 (n_even - n_odd)).
# The grid is uniform, 10 nm, and the strips' edges lie on it at every gap of the sweep (steps of
# 20 nm). The Academy's lesson on the directional coupler shows the strips in 3D; its charts
# solve the same supermodes on a grid graded from 10 nm at the strips.
#   photonoxide run jobs/coupler-supermodes.yaml
name: coupler-supermodes
timeout_minutes: 10

task:
  kind: modes
  stack: soi_220
  wavelength_um: 1.55
  layer: Si
  propagation: x           # the modes travel along x, cut at x = 0
  y_um: [-2.0, 2.0]        # the window across the pair
  step_nm: 10.0
  modes: 2                 # the even and the odd supermode

  parameters:
    w: "500 nm"
    gap: "200 nm"

  rect:
    - layer: Si
      center: [0, "-(gap + w)/2"]
      size: ["10 um", w]

    - layer: Si
      center: [0, "(gap + w)/2"]
      size: ["10 um", w]

  sweep:
    parameter: gap
    from: "100 nm"
    to: "400 nm"
    points: 16
```

```python
import photonoxide as phox

run = phox.run_job("jobs/coupler-supermodes.yaml")   # the file above, into runs/
modes = phox.run_tables(run.dir)["modes"]            # at the 200 nm gap
n_even, n_odd = modes["n_eff_re"]
print(n_even, n_odd, 1.55 / (2 * (n_even - n_odd)))  # L_x, um

# the same coupler as the directional coupler device, at its 3D model
figures = {f.symbol: f for f in phox.device_derived("directional coupler", values={"gap": 0.2},
                                                    fidelity="3D")}
print(figures["L_x"].value, figures["L_x"].grid)
print(phox.device_job("directional coupler", values={"gap": 0.2}, fidelity="3D",
                      format="yaml"))                # the device's own job, as the studio writes it
```

| Shape | Layer | Centre | Size |
|---|---|---|---|
| the lower strip, a rectangle | Si | (0, −(gap + w)/2) = (0, −0.35) µm | 10 µm along x, w = 500 nm across |
| the upper strip, a rectangle | Si | (0, (gap + w)/2) = (0, 0.35) µm | 10 µm along x, 500 nm across |

- **Solved:** the full-vector mode solver on the cross-section at x = 0, two modes, at the gap
  of 200 nm and then at 16 gaps from 100 to 400 nm.
- **Window and grid:** y from −2.0 to 2.0 µm; z 1 µm below and above the Si layer; a uniform
  10 nm grid, the strips' edges on it at every gap of the sweep.
- **Tolerance:** the job reports. The example `directional_coupler` checks L_x at 200 nm within
  1 µm of the book's 37.5 µm.

| Model | Grid | n_even | n_odd | L_x at a 200 nm gap |
|---|---|---|---|---|
| the modes job | 10 nm | 2.46027 | 2.43924 | 36.85 µm |
| the directional coupler device, 3D | 160 × 111 cells, 10 nm | | | 36.85 µm |

## A stack of your own

The leaky silicon wire of Bienstman et al. (Opt. Quantum Electron. 38, 731 (2006)), on a stack
written out in the file, with the benchmark's fixed indices: a 500 × 220 nm wire on 1 µm of
oxide over a silicon substrate, under air. Its light leaks through the oxide into the substrate.

```yaml
# Bienstman et al.'s leaky silicon wire: 500 x 220 nm of silicon (3.5) on 1 um of oxide (1.45)
# over a silicon substrate, air above, at 1.55 um. Its light leaks through the oxide into the
# substrate, whose index is above the mode's: a PML in the substrate absorbs it, and the mode is
# leaky, its loss in dB/cm. Their consensus is 2.412372 + 2.9135e-8 i; on this 10 nm grid the job
# gives 2.4187 + 2.757e-8 i, and on 5 nm 2.4164 + 2.805e-8 i (the core's corners converge slowly).
#   photonoxide run jobs/leaky-wire.yaml
name: leaky-wire
timeout_minutes: 10

task:
  kind: modes
  wavelength_um: 1.55
  layer: Si
  propagation: x           # the modes travel along x, cut at x = 0 (cut_x_um)
  y_um: [-1.0, 1.0]        # the window across the guide
  z_um: [-1.6, 2.0]        # 1.6 um of the substrate, its last 1 um the PML
  step_nm: 10.0
  modes: 2
  modes_near: 2.4          # the count nearest this index: the substrate's own modes lie above it

  stack:
    substrate: 3.5         # fixed indices, the benchmark's
    cladding: 1.0

    layer:
      - name: BOX
        thickness_um: 1.0
        material: 1.45
        background: 1.45

      - name: Si
        thickness_um: 0.22
        material: 3.5
        background: 1.0

  rect:
    - layer: Si
      center_um: [0.0, 0.0]
      size_um: [10.0, 0.5]  # 500 nm wide, along x

  pml:
    thickness_um: [0.0, 0.0, 1.0, 0.0]  # across the guide low and high, below and above
    strength: 3.0
```

```python
import photonoxide as phox

run = phox.run_job("jobs/leaky-wire.yaml")      # the file above, into runs/
modes = phox.run_tables(run.dir)["modes"]
print(modes["n_eff_re"], modes["n_eff_im"], modes["loss_db_per_cm"])
```

The file's `layer` list runs bottom to top, `BOX` then `Si`, the order they are laid down in;
the table runs from the top down.

| Layer | Thickness | Material | n at 1.55 µm |
|---|---|---|---|
| cladding | above the stack | air | 1.0 |
| Si | 220 nm (z = 1.0 to 1.22 µm) | silicon in the wire, air beside it | 3.5; 1.0 |
| BOX | 1 µm (z = 0 to 1.0 µm) | oxide | 1.45 |
| substrate | below the stack | silicon | 3.5 |

| Shape | Layer | Centre | Size |
|---|---|---|---|
| the wire, a rectangle | Si | (0, 0) µm | 10 µm along x, 500 nm across |

- **Solved:** the full-vector mode solver on the cross-section at x = 0, the modes nearest
  n_eff = 2.4 (the substrate's own modes lie above it), a PML absorbing the leakage: two asked
  for, and the run keeps one, the wire's TE-like mode.
- **Window and grid:** y from −1.0 to 1.0 µm; z from −1.6 to 2.0 µm, the substrate's lowest
  1 µm (z = −1.6 to −0.6 µm) the PML, of strength 3; a uniform 10 nm grid.
- **Tolerance:** the job reports. The example `leaky_wire_benchmark` checks the same wire on
  graded grids, Richardson-extrapolated, within 2 × 10⁻⁴ in Re and 2 % in Im of the consensus.

| Model | Grid | n_eff | Loss |
|---|---|---|---|
| the modes job | 10 nm | 2.41872 + 2.757 × 10⁻⁸ i | 0.0097 dB/cm |
| the benchmark's consensus (Bienstman et al., Table 6) | | 2.412372 + 2.9135 × 10⁻⁸ i | |

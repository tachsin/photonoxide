---
title: "The strip waveguide and its cross-section"
summary: "A silicon strip confines light in both directions across it: its full-vector modes, the effective index method and Marcatili's approximation, corners and their convergence, the group index, and leakage into the substrate."
topic: Waveguides
level: intermediate
status: coming
prerequisites: [slab-waveguide]
examples: [strip_waveguide, effective_index_method, marcatili, hadley_corners, group_index, leaky_wire_benchmark]
jobs: [strip-modes.toml, strip-width-sweep.toml, strip-guided-modes.toml]
methods: [vector.md, eim.md, marcatili.md, hadley.md, dispersion.md, eigen.md, walls.md, pml.md]
---

## 1. Why does a strip need a full-vector solve?

::coming strip_waveguide
Confinement in two directions makes every mode hybrid, TE-like or TM-like, with field components
coupled at the sidewalls, which a scalar or slab solve leaves out.

## 2. What is the TE-like mode of a 500 × 220 nm strip?

::coming strip_waveguide
Its effective index at 1550 nm at 20, 10 and 5 nm grids, against Chrostowski and Hochberg's
Fig. 3.14, and its field.

## 3. How does the effective index method reduce a strip to two slabs?

::coming effective_index_method
Hocker and Burns's two slab solves, and the method's error in $n_\mathrm{eff}$ and $n_g$ against the
full-vector solve for strips 400 to 600 nm wide.

## 4. When is Marcatili's approximation good?

::coming marcatili
His fields that ignore the corners, his closed form, and where both depart from the full-vector
solve, from near cutoff to well guided, for $a = 2b$ and $n_1/n_4 = 1.05$.

## 5. Why do corners slow convergence, and what do Hadley's equations change?

::coming hadley_corners
The field's singularity at a dielectric corner, the standard scheme's first order there, and
Hadley's interface and corner equations at about second order, from 8 to 128 cells a side.

## 6. What is the group index, and why is it larger than the effective index?

::coming group_index
$n_g = n_\mathrm{eff} - \lambda\thinspace dn_\mathrm{eff}/d\lambda$, the parts the material and
the confinement contribute, for strips 400 to 600 nm wide against Chrostowski and Hochberg's
Fig. 3.22b.

## 7. How does a strip on a thin oxide leak into the substrate, and how precisely is that loss known?

::coming leaky_wire_benchmark
Tunnelling through 1 µm of oxide into the silicon below, a PML that absorbs it, and Richardson
extrapolation from 5, 2.5 and 1.25 nm grids against Bienstman et al.'s benchmark.

## 8. How does photonoxide solve a cross-section?

::coming strip_waveguide hadley_corners
Fallahkhair, Li and Murphy's eigenproblem for the transverse magnetic field on a rectilinear grid,
shift-and-invert with Krylov–Schur restarts,
mirror walls for a quarter domain, and the grid every quoted number comes with.

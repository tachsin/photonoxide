---
title: "Full-vector modes by finite differences"
module: mode::vector
summary: "The modes of any waveguide cross-section, anisotropic and lossy media included, from the transverse magnetic field on a rectilinear grid."
order: 5
papers:
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008)"
    doi: 10.1109/JLT.2008.923643
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), part II: dielectric corners (the corner test problems)"
    doi: 10.1109/JLT.2002.800371
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Fig. 3.14"
    doi: 10.1017/CBO9781316084168
  - cite: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), the leaky-wire benchmark"
    doi: 10.1007/s11082-006-9025-9
validation:
  - mode/vector-slab-limit-te
  - mode/vector-slab-limit-tm
  - mode/strip-book
  - mode/hadley-box-low
  - mode/hadley-box-high
  - mode/hadley-corner-low
  - mode/hadley-corner-high
  - mode/leaky-wire-bienstman
  - mode/leaky-wire-bienstman-loss
examples:
  - strip_waveguide
  - hadley_corners
  - group_index
  - leaky_wire_benchmark
---

The unknowns are the transverse magnetic field, $H_x$ and $H_y$, at the nodes of a rectilinear
grid whose spacing need not be uniform. The permittivity is uniform in each cell between nodes:
a tensor with $\varepsilon_{xx}, \varepsilon_{xy}, \varepsilon_{yx}, \varepsilon_{yy}$ and $\varepsilon_{zz}$, so anisotropic,
lossy and gyrotropic media are all allowed.

## The method

Fallahkhair, Li and Murphy discretize the coupled equations for $H_x$ and $H_y$ (their Eqs. 4a–4b)
on a nine-point stencil. At each node, the four cells around it may differ, and the
interface conditions between them are built into the coefficients (Eqs. 21–36). The result is
a sparse eigenproblem (Eq. 8) whose eigenvalues are $\beta^2$:

$$
\begin{pmatrix} A_{xx} & A_{xy} \cr A_{yx} & A_{yy} \end{pmatrix}
\begin{pmatrix} H_x \cr H_y \end{pmatrix} = \beta^2 \begin{pmatrix} H_x \cr H_y \end{pmatrix},
\qquad n_\text{eff} = \beta / k_0 .
$$

The modes nearest a guess come from [shift-and-invert Arnoldi](eigen.md).
`VectorMode::residual` measures a mode against the matrix assembled afresh,
$\lVert A h - \beta^2 h\rVert / (|\beta^2|\thinspace\lVert h\rVert)$; a `"modes"` job records it for every mode, and
the studio's Solver panel shows it.

- **Convention:** the paper uses $e^{+j\omega t}$. Its eigenvalue equations hold in photonoxide's
  $e^{-i\omega t}$ when written with that convention's permittivity: loss is $+i$, and a gyrotropic
  term the paper writes as $+j\Delta$ is $-i\Delta$ here.
- **A misprint:** Eq. (31) divides by $v_{12}$, which the paper never defines. It is $v_{21}$, as
  the matching term of Eq. (30) shows under the mirror $x \to -x$. A test checks that a
  structure and its mirror image have the same modes, to 1e-10.
- **Edges:** beyond each edge the field is zero, or a [mirror wall](walls.md) reflects it, or a
  [PML](pml.md) absorbs it.

## Accuracy

- **Interfaces:** second order, in both orientations. A slab turned on its side converges onto
  the [exact slab](slab.md) at orders 2.00 / 2.01 / 2.00; at a 2.5 nm grid the error is 4e-5
  (TE) and 5e-6 (TM).
- **Corners** converge more slowly, as with standard finite differences: their fields'
  derivatives are singular. On Hadley's four corner problems (series-expansion indices good to
  1e-8), about **first order at convex corners** (a high-index quadrant: boxes, strips), with the
  error changing sign on the way, and **1.8 falling towards 1.4 at concave ones**. Hadley's own
  equations, on the same grid and unknowns, reach about second order there and sixth at
  straight interfaces: see [high-accuracy modes](hadley.md), for uniform grids of lossless
  isotropic media.
- **The book's strip** (500 × 220 nm, 3.473 in 1.444, 1550 nm): TE-like 2.447067, 2.445713,
  2.444396, 2.443506 at 20, 10, 5, 2.5 nm, against the book's 2.443 (Lumerical, 20 nm mesh,
  good to about 1e-3).

## Grids and extrapolation

`graded_nodes` builds a grid fine in a box around the core and growing gently away from it,
the interfaces on nodes; keep the growth to a few hundredths per µm, since the scheme loses
accuracy where neighbouring spacings differ much. Refining at the corners alone doesn't pay:
the core's interior must stay resolved. Because the corners converge at a steady low order,
`richardson` extrapolates from three grids, each half the last, with the order fitted.

On Bienstman et al.'s leaky SOI wire (silicon against air, the hardest corners), the order is
0.67: +4.0e-3, +2.5e-3, +1.5e-3 at 5, 2.5 and 1.25 nm in Re, and 0.97, 0.98, 0.99 of the
loss. Extrapolated: 2.412289 + 2.9207e-8 i, against the benchmark's 2.412372 + 2.9135e-8 i
(CAMFR and the aperiodic Fourier modal method), 8e-5 and 0.25 % away. The loss also needs the
oxide and substrate resolved: grading to 40 nm there misstates it by 3 %, to 10 nm by 0.3 %.

## Validation

- A uniform box has its exact discrete eigenvalues, to 1e-9.
- The slab limit, at second order, against the exact slab (both orientations).
- Hadley's four corner problems within 1e-4 at an 80 × 80 grid, and 5e-5 at 160 × 160.
- Chrostowski and Hochberg's strip within 3e-3 at 5 nm.
- Bienstman et al.'s leaky wire, extrapolated: within 8e-5 in Re and 0.25 % in the loss.

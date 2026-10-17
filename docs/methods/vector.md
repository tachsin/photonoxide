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
  - cite: "J. Hu, C. R. Menyuk, Adv. Opt. Photon. 1, 58 (2009): guided, leaky and radiation modes"
    doi: 10.1364/AOP.1.000058
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

The modes nearest a guess come from [shift-and-invert Arnoldi](eigen.md), restarted by
Krylov–Schur, so 20 to 50 of them hold a space of fixed size; every mode in a region of the
complex $n_\text{eff}$ plane, with no count and no guess, from [contour integrals](contour.md),
and every guided mode above a threshold from `modes_above` (a modes job's `modes_above`).
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

## What a count returns: guided modes and the window's own

A guided mode's $n_\text{eff}$ lies above the index of every medium around the guide, so its
field decays away from it. Below that index the physical modes are leaky or radiating: their
fields grow or oscillate away from the guide, out to infinity, and they aren't normalizable
(Hu and Menyuk, [doi:10.1364/AOP.1.000058](https://doi.org/10.1364/AOP.1.000058), on the slab).
A window whose edges hold the field at zero can't represent them. It has modes of its own
there instead, standing waves between its edges, with $n_\text{eff}$ below the cladding's, even
below 1: modes of the box the window makes, not of the guide.

`modes` returns every mode its count asks for, so asked for more modes than the guide has it
returns these after the guided ones. `guided_modes` keeps only the guided ones: of the count
solved, those with $n_\text{eff}$ above `CrossSection::surrounding_index`, the highest index of
the media around the guides; fewer than the count when fewer are guided, none when none is, and
nothing more solved to make the count up. `VectorMode::is_guided` asks it of one mode. The media
around the guides are those at the window's open edges (a wall is a plane of symmetry through
the guides, not their surroundings): the substrate, the cladding and the layers' backgrounds.
Where a guide runs into an edge, as a rib's slab does, the fastest wave along that edge is the
slab's own mode, solved on the edge's cells by the same scheme, and a mode below it leaks into
the slab, so a window's side should stay clear of every guide but a slab. The façade's
`vector_modes` (Python's and MATLAB's) keeps the modes of a closed window by this rule. A
`"modes"` job keeps a count's modes by the same rule, reading the media from its stack: the
layers' backgrounds, and the substrate and cladding its window reaches, the same index on the
built-in jobs' cross-sections (a test checks it). `modes_above` with the surrounding index as
its threshold gives every guided mode, however many.

Leaky modes need an open window: a [PML](pml.md) inside the edges absorbs what reaches them,
and then `modes` finds a leaky mode with its loss, as the leaky-wire benchmark below does. The
PML's own modes come too, their loss large. Neither is a guided mode, so `guided_modes` leaves
both out; with PMLs, ask `modes` for them.

### With a PML, in a modes job

A `"modes"` job with a `[task.pml]` (a thickness for each edge, across the guide at its low and
high end, below and above, and a strength) solves its count, or every mode above `modes_above`,
in the open window, and keeps and names them:

- **guided:** $\operatorname{Re} n_\text{eff}$ above the highest index of the media around the
  guides, the rule above;
- **leaky:** below it but above the lowest of those media: evanescent in at least one of them,
  its light leaking into the others;
- **left out:** the PML's own modes, more than half of their $\lvert E \rvert^2$ inside the PML,
  where their fields grow (on Bienstman's wire 0.84 to 0.93, against below 0.012 for the leaky
  modes of a strip on a thin oxide); and modes below every medium around the guides, which
  radiate into all of them, the window's radiation rather than a mode the guides hold.

Each kept mode is recorded with its kind and its loss in dB/cm, $20 \log_{10}(e)\thinspace k_0
\operatorname{Im} n_\text{eff}$, which the run's solver details state with the grid and the
PML. Where the media around the guides hold the highest index (a substrate), the modes nearest
it are the PML's own, so a count is aimed at the guide with `modes_near`, as
`jobs/leaky-wire.toml` does. A PML belongs where light leaks: on a guided mode's evanescent tail
it changes $n_\text{eff}$ a little and gives it a small loss of either sign (a strip in oxide, its
PML 0.6 µm from the core: under 2e-6 in $n_\text{eff}$, $\lvert \operatorname{Im} n_\text{eff}
\rvert$ under 6e-6). A job without a `[task.pml]` runs as before, to the bit. The validation
report's `job/` cases check it: a film leaking into its substrate against the exact leaky mode,
second order in the grid and insensitive to the PML, and Bienstman's wire to its published index
and loss on a 5 nm grid.

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

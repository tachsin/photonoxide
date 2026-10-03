---
title: "Planar profiles by finite differences"
module: mode::slab_fd
summary: "The modes of any planar profile, graded, lossy or leaky, by 1D finite differences: fast, second order, with a PML."
order: 14
papers:
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008) (the 2D scheme this is the 1D limit of)"
    doi: 10.1109/JLT.2008.923643
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the modes reproduced)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - mode/slab-fd-chilwell
---

The [exact slab](slab.md) and the [transfer matrices](multilayer.md) need piecewise-constant
layers. `slab_fd::Profile` takes any permittivity per cell between nodes of any spacing: graded
index, loss, and leaky stacks with a [PML](pml.md). The field ψ (E along the layers for TE, H
for TM) lives on the nodes. At node i, with the cell to its left of width w and permittivity
$\varepsilon_w$, and to its right of width e and $\varepsilon_e$:

$$
\text{TE:}\quad \frac{2}{w+e}\left[\frac{\psi_{i+1} - \psi_i}{e} - \frac{\psi_i - \psi_{i-1}}{w}\right] + k^2 \bar\varepsilon\thinspace\psi_i = \beta^2 \psi_i,
\qquad \bar\varepsilon = \frac{w \varepsilon_w + e \varepsilon_e}{w + e},
$$

$$
\text{TM:}\quad \frac{2}{w+e}\left[\frac{\psi_{i+1} - \psi_i}{e\thinspace\varepsilon_e} - \frac{\psi_i - \psi_{i-1}}{w\thinspace\varepsilon_w}\right] + k^2 \psi_i = \beta^2 \left\langle \tfrac{1}{\varepsilon} \right\rangle \psi_i,
$$

which keeps ψ and ψ′/ε continuous at interfaces. These are the [full-vector](vector.md)
scheme's equations in the limit of a structure uniform in one direction, so the same accuracy
holds: second order with interfaces on nodes.

## Validation

- **The book's slab** (220 nm of 3.473 in 1.444, 1.55 µm): second order onto the exact slab,
  TE and TM.
- **Chilwell and Hodgkinson's four-layer guide**: all 8 bound modes within 2e-6 of their Table 3
  on a 1 nm grid (the substrate 6 µm deep, for TE₃'s slow tail), and their leaky waves m = 4–7
  within 2e-5 with a PML in the substrate.

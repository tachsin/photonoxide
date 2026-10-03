---
title: "Group index, dispersion and loss"
module: mode::dispersion
summary: "What follows from a mode's effective index across wavelength: group index, dispersion, loss, and following one mode through a sweep."
order: 9
papers:
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015), Section 3.2.9"
    doi: 10.1017/CBO9781316084168
validation:
  - mode/slab-group-index
  - mode/strip-group-index-book
examples:
  - group_index
  - effective_index_method
---

The effective index sets a mode's phase velocity. A pulse, and a ring's free spectral range, go
by the **group index**; pulse spreading by the **dispersion**.

## The quantities

Chrostowski and Hochberg's Eqs. 3.5 and 3.6:

$$
n_g = n_\text{eff} - \lambda \frac{d n_\text{eff}}{d\lambda}, \qquad
D = -\frac{\lambda}{c} \frac{d^2 n_\text{eff}}{d\lambda^2} \quad [\text{ps/(nm·km)}],
$$

from one mode's effective index at three or more wavelengths. The derivatives are the
three-point, second-order differences of the samples, which may be unevenly spaced: exact on a
parabola. Both include the materials' dispersion when the effective indices do, so each
wavelength's cross-section is built from the materials at that wavelength.

The **loss** follows from the imaginary index: the power decays as $e^{-2k_0 \operatorname{Im}(n_\text{eff}) z}$, in
dB/cm $10 \log_{10}(e) \cdot 2 k_0 \operatorname{Im}(n_\text{eff}) \cdot 10^4$ with $k_0$ in 1/µm.

## Following one mode

`track` solves at each wavelength for a few modes nearest the last effective index and keeps the
one whose fields overlap the last mode's most:

$$
\frac{\left|\int \mathbf H_a^* \cdot \mathbf H_b\thinspace dA\right|}{\lVert \mathbf H_a \rVert\thinspace\lVert \mathbf H_b \rVert},
$$

each node weighted by the area around it. It carries on through crossings with other modes.

## Validation

- **Analytic:** a TE slab's group index from differences equals Hellmann–Feynman's
  $\langle\varepsilon\rangle / n_\text{eff}$, from its exact field, to 1e-6 (no material dispersion).
- **Published:** the book's Fig. 3.22b, the group index of 220 nm silicon strips 400–600 nm
  wide, with its own materials (its Lorentz fit to silicon, and 1.444 oxide). Read off the plot
  at 1.55 µm: 4.37, 4.27, 4.18, 4.10, 4.04. photonoxide on a 6.25 × 5 nm grid: 4.355, 4.262,
  4.173, 4.098, 4.038.

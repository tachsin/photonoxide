---
title: "Fields, power and coupling"
module: mode::fields
summary: "A full-vector mode's six field components from its transverse H, the power it carries, and the share of it that launches another mode."
order: 12
papers:
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008) (the transverse H the fields start from)"
    doi: 10.1109/JLT.2008.923643
validation:
  - mode/fields-butt-coupling
---

The [full-vector solver](vector.md) finds the transverse magnetic field, $H_x$ and $H_y$, at the
grid's nodes. `VectorMode::fields` derives the rest from Maxwell's equations with the mode's
$e^{i(\beta z - \omega t)}$, at the centres of the cells, where each cell's permittivity is uniform:

- $H_z$ from $\nabla\cdot\mathbf H = 0$: $\partial_x H_x + \partial_y H_y + i\beta H_z = 0$;
- $E_z$ from Ampère's law: $\partial_x H_y - \partial_y H_x = -ik\,\varepsilon_{zz} E_z$;
- the transverse E from Faraday's law:

$$
E_x = \frac{k}{\beta} H_y + \frac{\partial_x E_z}{i\beta}, \qquad
E_y = -\frac{k}{\beta} H_x + \frac{\partial_y E_z}{i\beta}.
$$

E is in units of $Z_0 = (\mu_0/\varepsilon_0)^{1/2}$ times H's. Taking the transverse E from
Ampère's law instead would need H's second derivatives, which jump at interfaces: next to an
interface that is 20 % wrong on a 10 nm grid. Faraday's law gives the large transverse field
straight from H, and derivatives enter only through $E_z$, which is small and, being tangential
to every interface, continuous across them.

## Power and coupling

With $\langle a, b\rangle = \int (\mathbf E_a \times \mathbf H_b^*)\cdot\hat z\, dA$, the power is
$P = \tfrac12 \operatorname{Re}\langle a, a\rangle$, and the share of mode a's power that launches mode b, at a junction
between two waveguides or at the start of a bend, is

$$
\eta = \frac{\operatorname{Re}\left(\langle a, b\rangle \langle b, a\rangle\right)}{\operatorname{Re}\langle a, a\rangle \, \operatorname{Re}\langle b, b\rangle}.
$$

It is 1 for a mode with itself and 0 between modes that are orthogonal. Both modes must be on the
same grid.

## Validation

- **Impedance:** for a TE slab, $E_y = -(k/\beta) H_x$ in every cell, to 2e-3, including the cells
  beside the interfaces.
- **Butt coupling:** a 220 nm silicon slab's TE mode into a 300 nm slab's (3.473 in 1.444, 1.55
  µm). For TE slabs H ∝ E, so the exact value is $(\int E_1 E_2)^2 / (\int E_1^2 \int E_2^2) = 0.994662$
  from the [exact slab](slab.md) fields; from the full-vector fields, 0.994730 on a 10 nm grid.
- **Orthogonality:** the strip's TE-like and TM-like modes couple below 1e-6; a mode couples to
  itself exactly.

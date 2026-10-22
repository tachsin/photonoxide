---
title: "Fields, power and coupling"
module: mode::fields, mode::thermo
summary: "A full-vector mode's six field components from its transverse H, the power it carries, the share of it that launches another mode, and its effective index's change for a change of permittivity, such as a temperature field's, by first-order perturbation and by re-solve."
order: 12
papers:
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008) (the transverse H the fields start from)"
    doi: 10.1109/JLT.2008.923643
  - cite: "S. G. Johnson, M. Ibanescu, M. A. Skorobogatiy, O. Weisberg, J. D. Joannopoulos, Y. Fink, Phys. Rev. E 65, 066611 (2002) (first-order perturbation, away from moving boundaries)"
    doi: 10.1103/PhysRevE.65.066611
  - cite: "S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015) (silicon wires' measured dn_eff/dT, Fig. 8)"
    doi: 10.1109/JLT.2015.2476603
  - cite: "M. Jacques et al., Opt. Express 27, 10456 (2019) (the TiN heater whose temperature field enters a mode)"
    doi: 10.1364/OE.27.010456
validation:
  - mode/fields-butt-coupling
  - thermo/slab-first-order
  - thermo/slab-vector
  - thermo/slab-vector-order
  - thermo/wire-perturbation
  - thermo/dwivedi-470
  - thermo/dwivedi-602
  - thermo/dwivedi-805
  - thermo/heater-field
---

The [full-vector solver](vector.md) finds the transverse magnetic field, $H_x$ and $H_y$, at the
grid's nodes. `VectorMode::fields` derives the rest from Maxwell's equations with the mode's
$e^{i(\beta z - \omega t)}$, at the centres of the cells, where each cell's permittivity is uniform:

- $H_z$ from $\nabla\cdot\mathbf H = 0$: $\partial_x H_x + \partial_y H_y + i\beta H_z = 0$;
- $E_z$ from Ampère's law: $\partial_x H_y - \partial_y H_x = -ik\thinspace\varepsilon_{zz} E_z$;
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

In Python and MATLAB, each mode `vector_modes` returns ([the call](vector.md#what-a-count-returns-guided-modes-and-the-windows-own)) carries these six
components at the cells' centres, each `(ny, nx)`, with the centres' coordinates:

```python
m = modes[0]
m.ex, m.ey, m.ez, m.hx, m.hy, m.hz   # m.ex[j, i] is E_x at (m.x_um[i], m.y_um[j])
```

```matlab
m = modes(1);
m.ex                                  % m.ex(j, i) is E_x at (m.x_um(i), m.y_um(j))
```

## Power and coupling

With $\langle a, b\rangle = \int (\mathbf E_a \times \mathbf H_b^*)\cdot\hat z\thinspace dA$, the power is
$P = \tfrac12 \operatorname{Re}\langle a, a\rangle$, and the share of mode a's power that launches mode b, at a junction
between two waveguides or at the start of a bend, is

$$
\eta = \frac{\operatorname{Re}\left(\langle a, b\rangle \langle b, a\rangle\right)}{\operatorname{Re}\langle a, a\rangle \thinspace\operatorname{Re}\langle b, b\rangle}.
$$

It is 1 for a mode with itself and 0 between modes that are orthogonal. Both modes must be on the
same grid.

## A change of permittivity: perturbation and re-solve

A change $\Deltaarepsilon$ of the cells' relative permittivity, inside each cell so that no
boundary moves, shifts a mode's frequency at fixed $eta$ by
$\Delta\omega/\omega = -	frac12 \int \mathbf E^*\cdot\Deltaarepsilon\cdot\mathbf E	hinspace dA / \int arepsilon|\mathbf E|^2 dA$
to first order (Johnson et al. 2002, before their moving boundaries). At fixed $\omega$ that is
a shift of $eta$ of $\Delta\omega$ over the group velocity, the energy's,
$v_g = 	frac12\operatorname{Re}\int(\mathbf E	imes\mathbf H^*)\cdot\hat z	hinspace dA ig/ 	frac12arepsilon_0\intarepsilon|\mathbf E|^2 dA$;
with E in units of $Z_0$ times H's,

$$
\Delta n_\mathrm{eff} = rac{\int \mathbf E^*\cdot\Deltaarepsilon\cdot\mathbf E	hinspace dA}{2\operatorname{Re}\int (\mathbf E	imes\mathbf H^*)\cdot\hat z	hinspace dA}.
$$

`mode::thermo::index_change` takes it from the fields above, but for one thing: beside a change
of material, $\partial_x E_z$ and $\partial_y E_z$ come from the neighbour of the same material
(one-sided), so E's normal component keeps its jump at the interface ($arepsilon E_\perp$
continuous). A central difference across the interface spreads the jump over the two cells
beside it and leaves the overlap first order in the grid; one-sided, it is second order (2.19 and
2.34 on a TM slab, whose E crosses its interfaces). `index_change_resolved` solves the perturbed
cross-section (`CrossSection::perturbed`) instead, near the first-order estimate; the two differ
at second order in $\Deltaarepsilon$ and by the field's own error.

**Temperature.** `permittivity_change(cold, warm, step, rise)` takes the cross-section at a
temperature and the same one `step` K warmer, each with its materials at that temperature (the
catalogue's silicon and silica take one, [materials](materials.md#temperature)), and gives each
cell $\Deltaarepsilon = (arepsilon_\mathrm{warm} - arepsilon_\mathrm{cold})/\mathrm{step}
	imes \Delta T(x, y)$, the rise at its centre: a uniform one, or a heat solve's field
(`thermal::Heat::solve`, [static](static.md)), read at each cell with `Field::at`. To first order
in $\Delta T$, as the perturbation is.

```rust
let cs = at(293.0);                                   // the cross-section, materials at 293 K
let mode = vector::modes(&cs, wavelength, 1, None)?.remove(0);
let delta = permittivity_change(&at(292.5), &at(293.5), 1.0, |x, y| field.at(x, y))?;
let dn = index_change(&mode, &cs, &delta)?;            // first order
let check = index_change_resolved(&mode, &cs, &delta, wavelength)?;   // a re-solve
```

## Validation

- **Impedance:** for a TE slab, $E_y = -(k/\beta) H_x$ in every cell, to 2e-3, including the cells
  beside the interfaces.
- **Butt coupling:** a 220 nm silicon slab's TE mode into a 300 nm slab's (3.473 in 1.444, 1.55
  µm). For TE slabs H ∝ E, so the exact value is $(\int E_1 E_2)^2 / (\int E_1^2 \int E_2^2) = 0.994662$
  from the [exact slab](slab.md) fields; from the full-vector fields, 0.994730 on a 10 nm grid.
- **Orthogonality:** the strip's TE-like and TM-like modes couple below 1e-6; a mode couples to
  itself exactly.
- **A change of temperature, first order** (`thermo/`): a 220 nm silicon slab in silica (TE₀,
  1.55 µm) warmed from 293 K, its first-order change in closed form against the slab solved
  again: the difference falls as $\Delta T^2$ (order 2.03 from 10, 20 and 40 K). The full-vector
  perturbation on the same slab at 5 nm, TE₀ and TM₀, within 3.2e-4 of the exact
  $dn_\mathrm{eff}/dT$ (1.814e-4 and 1.425e-4 per K), converging at second order.
- **A wire:** 470 × 211 nm, $dn_\mathrm{eff}/dT$ by perturbation on a 5 nm grid is 0.36% above
  re-solves by [Hadley's equations](hadley.md) (1.9563e-4 per K at 10 nm, within 2e-6 relative of
  5 nm): the field's singularity at the core's corners converges slowly in a cell's overlap.
  Re-solves on the plain solver's grid converge to the same at about first order.
- **Measured** (Dwivedi et al. 2015, Fig. 8, at 1.55 µm): the 470, 602 and 805 nm wires, 211 nm
  thick, at 1.9563, 1.9251 and 1.8811 × 10⁻⁴ per K against 2.08, 2.005 and 1.98 measured, each
  4 to 6% below; within the plot's error bar and Li's ±0.15e-4 per K in silicon's $dn/dT$ carried
  through the wire. The order (narrower higher) is the measured one. The paper gives no
  temperature; at 293 K Li's silicon has $dn/dT$ = 1.82e-4 per K, where the paper takes 1.86e-4.
- **A heat solve's field:** Jacques et al.'s TiN heater ([static](static.md)) warming its 500 ×
  220 nm strip by 10 K on average, the rise at each cell of the mode's cross-section (5 nm):
  perturbation and re-solve agree to 0.38%.

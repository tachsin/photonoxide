---
title: "Fields, power and coupling"
module: mode::fields, mode::quantities, mode::thermo
summary: "A full-vector mode's six field components from its transverse H, the power it carries, the share of it that launches another mode, its effective area, nonlinear coefficient and confinement, the orthogonality of a set of modes, and its effective index's change for a change of permittivity, such as a temperature field's, by first-order perturbation and by re-solve."
order: 12
papers:
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008) (the transverse H the fields start from)"
    doi: 10.1109/JLT.2008.923643
  - cite: "S. Afshar V., T. M. Monro, Opt. Express 17, 2298 (2009) (the effective area from S_z, Eq. 41, and the standard one, Eq. 44)"
    doi: 10.1364/OE.17.002298
  - cite: "S. G. Johnson, M. Ibanescu, M. A. Skorobogatiy, O. Weisberg, J. D. Joannopoulos, Y. Fink, Phys. Rev. E 65, 066611 (2002) (first-order perturbation, away from moving boundaries)"
    doi: 10.1103/PhysRevE.65.066611
  - cite: "S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015) (silicon wires' measured dn_eff/dT, Fig. 8)"
    doi: 10.1109/JLT.2015.2476603
  - cite: "M. Jacques et al., Opt. Express 27, 10456 (2019) (the TiN heater whose temperature field enters a mode)"
    doi: 10.1364/OE.27.010456
validation:
  - mode/fields-butt-coupling
  - mode/effective-area-gaussian
  - mode/effective-area-slab-te
  - mode/effective-area-slab-te-order
  - mode/confinement-slab-te
  - mode/confinement-slab-tm
  - mode/confinement-slab-order
  - mode/orthogonality-slab
  - mode/orthogonality-slab-order
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

In Python, each mode `vector_modes` returns ([the call](vector.md#what-a-count-returns-guided-modes-and-the-windows-own)) carries these six
components at the cells' centres, each `(ny, nx)`, with the centres' coordinates:

```python
m = modes[0]
m.ex, m.ey, m.ez, m.hx, m.hy, m.hz   # m.ex[j, i] is E_x at (m.x_um[i], m.y_um[j])
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

## A mode's quantities

`mode::quantities` adds what a designer asks of a mode beside its index. With
$S_z = (\mathbf E \times \mathbf H^*)\cdot\hat z$:

| Quantity | Call | Definition |
|---|---|---|
| Effective area | `Fields::effective_area` | $A_\mathrm{eff} = \lvert\int S_z\thinspace dA\rvert^2 / \int \lvert S_z\rvert^2 dA$ (Afshar & Monro 2009, Eq. 41) |
| Standard effective area | `Fields::effective_area_transverse` | $A_\mathrm{eff} = (\int \lvert\mathbf E_t\rvert^2 dA)^2 / \int \lvert\mathbf E_t\rvert^4 dA$ (their Eq. 44) |
| Nonlinear coefficient | `Fields::nonlinear_coefficient` | $\gamma = 2\pi n_2 / (\lambda A_\mathrm{eff})$, $n_2$ given (their Eq. 42 with $\bar n_2 = n_2$) |
| Confinement | `VectorMode::confinement` | $\Gamma = \int_R \operatorname{Re} S_z\thinspace dA / \int \operatorname{Re} S_z\thinspace dA$, $R$ a material or a box |
| Orthogonality | `orthogonality` | $O_{ab} = \langle a, b\rangle / (\operatorname{Re}\langle a, a\rangle \operatorname{Re}\langle b, b\rangle)^{1/2}$, or the unconjugated product |
| Normalization | `Fields::normalized` | the fields scaled to $P = 1$ |

**The two areas.** Faraday's law gives, with our $e^{i(\beta z - \omega t)}$,
$S_z = (\beta^* \lvert\mathbf E_t\rvert^2 - i\thinspace\mathbf E_t\cdot\nabla_t E_z^*)/k$ (the paper's
Eq. 43 in its own convention). Where $\nabla_t E_z = 0$, as in a TE slab, Eqs. 41 and 44 are the
same area; elsewhere they differ by that term. On a grid Eq. 44 is the fragile one: $\lvert\mathbf
E_t\rvert^4$ takes the field's singularity at a dielectric corner twice. For a 500 × 220 nm silicon
strip in silica at 1.55 µm (TE-like, fields from `fields_one_sided`):

| Grid | Eq. 41, µm² | Eq. 44, µm² | γ for $n_2$ = 5e-18 m²/W, 1/(W m) | Γ in the silicon |
|---|---|---|---|---|
| 20 nm | 0.11363 | 0.15226 | 178.38 | 0.78547 |
| 10 nm | 0.11343 | 0.13112 | 178.69 | 0.78330 |
| 5 nm | 0.11329 | 0.09705 | 178.91 | 0.78260 |

Eq. 41 settles to four digits while Eq. 44 keeps falling, so `effective_area` and γ take Eq. 41.
The $n_2$ here is only an input to show the units: γ scales with it.

**Which fields.** The integrals take the fields at the cells' centres. From
`VectorMode::fields_one_sided`, E's normal component keeps its jump at each interface and a
region's share converges at second order; from `VectorMode::fields`, a TM slab's confinement
converges at first order only (3.9e-2, 1.8e-2, 9.0e-3 relative at 20, 10 and 5 nm, against 7.0e-3,
1.4e-3 and 3.2e-4 one-sided). `confinement` takes the one-sided fields.

**Walls.** A window cut by a wall on one edge is half the cross-section: the area, γ and the
confinement add the mirror image, so half a slab behind a wall gives the whole slab's numbers. A
window closed by walls at both ends of an axis is taken as it is: a slab uniform between two walls
has the effective width times the window's height as its area. A box straddling a wall counts
with its image. Next to a wall, $\partial_x E_z$ is taken one-sided, at first order, so a TM slab's
area from half the window is 8e-4 off the whole window's at 10 nm (a TE slab, with no $E_z$,
agrees to round-off).

**Orthogonality.** The conjugated product is the one `coupling` takes: power orthogonality,
between a lossless guide's modes. The unconjugated $\int (\mathbf E_a \times \mathbf H_b)\cdot\hat z\thinspace dA$
holds for lossy and leaky modes too, as [eigenmode expansion](eme.md) uses it; each mode is then
normalized by its own product, and the diagonal is 1 by construction. `off_diagonal` gives the
largest $\lvert O_{ab}\rvert$, `diagonal` the largest $\lvert O_{aa} - 1\rvert$ (with the conjugated
product, a mode's reactive power along z).

| Check (cases `mode/…`) | Grid | Ours | Reference |
|---|---|---|---|
| Gaussian $e^{-r^2/w^2}$, whole, half and quarter window, Eqs. 41 and 44 | 50 nm | within 3e-14 of $\pi w^2$ | $\pi w^2$ |
| 220 nm silicon slab TE₀ (3.473 in 1.444, 1.55 µm): effective width | 2.5 nm | 0.299874 µm | 0.299858 µm, closed form |
| the same, its confinement in the silicon | 2.5 nm | 0.809915 | 0.809930, `multilayer` |
| the same slab's TM₀, its confinement | 2.5 nm | 0.582843 | 0.582886, `multilayer` |
| 1 µm slab, four TE modes: largest $\lvert O_{ab}\rvert$ | 10, 5, 2.5 nm | 3.0e-4, 7.4e-5, 1.9e-5 | 0 |

The errors fall at second order: 2.00 for the TE width, the TE confinement and the orthogonality;
2.30, 2.17 and 2.10 at the TM confinement's successive halvings from 20 nm. The paper itself plots
γ against a rod's diameter (its Figs. 1 and 5) but prints no table of areas, so no case compares
with its numbers.

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

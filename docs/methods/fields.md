---
title: "Fields, power and coupling"
module: mode::fields, mode::quantities, mode::thermo
summary: "A full-vector mode's six field components from its transverse H, the power it carries, the share of it that launches another mode, its effective area, nonlinear coefficient and confinement, the orthogonality of a set of modes, and its effective index's change for a change of permittivity, such as a temperature field's, by first-order perturbation and by re-solve."
order: 12
papers:
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008) (the transverse H the fields start from)"
    doi: 10.1109/JLT.2008.923643
  - cite: "S. Afshar V., T. M. Monro, Opt. Express 17, 2298 (2009) (the effective area from S_z, Eq. 41, the standard one, Eq. 44, and the field-weighted n̄₂ of Eqs. 39 and 42)"
    doi: 10.1364/OE.17.002298
  - cite: "E. Snitzer, J. Opt. Soc. Am. 51, 491 (1961) (the round rod's exact HE₁₁ mode, against which their silicon rod's n̄₂ is checked)"
    doi: 10.1364/JOSA.51.000491
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
  - mode/kerr-index-slab-tm
  - mode/orthogonality-slab
  - mode/orthogonality-slab-order
  - mode/fields-folded-strip
  - mode/quantities-facade-strip
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
$P = \tfrac12 \mathop{\mathrm{Re}}\langle a, a\rangle$, and the share of mode a's power that launches mode b, at a junction
between two waveguides or at the start of a bend, is

$$
\eta = \frac{\mathop{\mathrm{Re}}\left(\langle a, b\rangle \langle b, a\rangle\right)}{\mathop{\mathrm{Re}}\langle a, a\rangle \thinspace\mathop{\mathrm{Re}}\langle b, b\rangle}.
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
| Field-weighted $\bar n_2$ | `VectorMode::field_weighted_n2`, `Fields::field_weighted_n2` | $\bar n_2 = \int n^2 n_2 [2\lvert\mathbf E\rvert^4 + \lvert\mathbf E\cdot\mathbf E\rvert^2] dA / 3\int \lvert S_z\rvert^2 dA$ (their Eqs. 39 and 42) |
| Full-vector γ | `VectorMode::vector_nonlinear_coefficient` | $\gamma^V = 2\pi \bar n_2 / (\lambda A_\mathrm{eff})$ (their Eq. 39) |
| Confinement | `VectorMode::confinement` | $\Gamma = \int_R \mathop{\mathrm{Re}} S_z\thinspace dA / \int \mathop{\mathrm{Re}} S_z\thinspace dA$, $R$ a material or a box |
| Orthogonality | `orthogonality` | $O_{ab} = \langle a, b\rangle / (\mathop{\mathrm{Re}}\langle a, a\rangle \mathop{\mathrm{Re}}\langle b, b\rangle)^{1/2}$, or the unconjugated product |
| Normalization | `Fields::normalized` | the fields scaled to $P = 1$ |
| Power shares | `Fields::power_shares` | each cell's $\mathop{\mathrm{Re}} S_z\thinspace dA / \int \mathop{\mathrm{Re}} S_z\thinspace dA$: a set of cells' sum is its confinement |

**In Python and in a job.** Each `VectorMode` that `vector_modes` returns carries its
`effective_area_um2` (Eq. 41) and `effective_area_transverse_um2` (Eq. 44), both the whole
cross-section's, its `power_shares` over the window's cells, `(ny, nx)`, and its row of the set's
`orthogonality` (the conjugated product with each mode the call returned, in their order), all from
the one-sided fields as the library takes them; `m.confinement(eps == 3.473**2)` sums the shares
over a set of cells, and `m.nonlinear_coefficient(n2)` gives γ for a uniform n₂. A modes job with
`[task.quantities]` (`nonlinear_index_m2_per_w` optional) records a `mode_quantities` event for each
mode after its field: the two areas, the confinement in the media whose index is above the media
around the guides, the row of products and, with an n₂, γ. `mode/quantities-facade-strip` holds the
façade's numbers to the library's own calls on a 480 × 200 nm strip's two modes: the same sums, so
round-off.

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

**The field-weighted $\bar n_2$.** Where $n_2$ varies across the section, or $E_z$ is large,
Afshar and Monro's full-vector γ (their Eq. 39) replaces $n_2$ with

$$
\bar n_2 = \frac{\int n^2 n_2 \left[2\lvert\mathbf E\rvert^4 + \lvert\mathbf E\cdot\mathbf E\rvert^2\right] dA}{3\int \lvert S_z\rvert^2 dA},
$$

with $\lvert\mathbf E\rvert^4 = (\mathbf E\cdot\mathbf E^*)^2$ and $\mathbf E\cdot\mathbf E$ the
unconjugated square. With H in $Z_0$'s units their $\varepsilon_0/\mu_0$ is 1. Their Eq. 42 prints
$\bar n_2$ with a leading $k$ as well; with it $\bar n_2$ wouldn't be an index in m²/W and
$2\pi\bar n_2/(\lambda A_\mathrm{eff})$ wouldn't be their Eq. 39, so we leave it out. Three closed
forms check it: a mode that is linearly polarized and transverse in a uniform medium gives
$\bar n_2 = n_2$, a circularly polarized one ($\mathbf E\cdot\mathbf E = 0$) $2n_2/3$, and a TE
slab with $n_2$ in its core gives $n_2\varepsilon_\text{core}/n_\text{eff}^2$ times the core's share
of $\int\lvert E_y\rvert^4$. A TM slab has $E_z$ a quarter period out of phase with $E_x$, and
the case `mode/kerr-index-slab-tm` compares its $\bar n_2$ with the exact slab's. For the 220 nm
silicon slab of 3.473 in 1.444 at 1.55 µm, with $n_2$ in the silicon only, the exact value is
0.851964 $n_2$:

| Grid | $\bar n_2 / n_2$ | Relative error |
|---|---|---|
| 20 nm | 0.859584 | 8.9e-3 |
| 10 nm | 0.853516 | 1.8e-3 |
| 5 nm | 0.852313 | 4.1e-4 |
| 2.5 nm | 0.852047 | 9.7e-5 |

The error falls at second order, as the TM confinement's does. A cell averaged across an
interface takes the area mean of $n^2 n_2$ over the cell: `Fields::field_weighted_n2` takes that
product cell by cell, and `VectorMode::field_weighted_n2` takes $n_2$ for each cell's permittivity.

**Where $\bar n_2$ doesn't converge.** $\lvert\mathbf E\rvert^4$ needs E right at every interface,
and where an interface is oblique to the grid it isn't. Each cell's $E_z$ takes the cell's own
averaged permittivity, so $E_z$ steps from cell to cell along a curved edge, and $E_t$'s
$\partial E_z/(i\beta)$ term spikes as $1/h$. Afshar and Monro's silicon rod (3.45 in silica,
1.45, at 800 nm, $D$ = 150 nm, the HE₁₁ mode on a quarter window) shows it. $\gamma^V/\gamma^F$, their
$\bar n_2$ over Foster et al.'s $S_z^2$-weighted $n_2$, is 1.31, 1.66, 11.9 and 82 at 20, 10, 5
and 2.5 nm. Eq. 44's area falls 30-fold over the same grids, while Eq. 41's moves by 12 %.
Neither the central differences of `fields` nor a staircased rod do better (1.45, 1.86, 11.6 and
2.33, 5.50, 24.6 at 20, 10 and 5 nm).

The rod's exact fields give what the paper does. A round rod is the step-index fibre (Snitzer
1961), whose HE₁₁ fields are Bessel functions in closed form ($E_z$ and $H_z$ as $J_1$ inside and
$K_1$ outside, the index from the fibre's eigenvalue equation, the transverse fields from $E_z$
and $H_z$). Through Eqs. 39, 41 and 48 they give:

| Their text (Section 3) | The exact fields |
|---|---|
| $\gamma^V$ and $\gamma^F$ differ by at most 40 %, at $D$ = 0.15 µm | 41.3 % at 0.15 µm (38.6 % at 0.14 µm, 37.4 % at 0.16 µm) |
| 14 % between the two curves' maxima | 15.2 %: $\gamma^V$ 1384 W⁻¹m⁻¹ at 0.19 µm, $\gamma^F$ 1201 at 0.20 µm (on a 0.01 µm step) |
| $\gamma^V$ twice $\gamma^S$ (Eq. 47, Agrawal's) at $D$ = 0.19 µm | 2.09 |

So our reading of their equations gives their numbers to the first digit, and the 1.4 that the
grid couldn't reach is the exact fields' 1.41. The second digit doesn't hold (41.3 against 40,
15.2 against 14). Their curves are fits through their computed points, and the paper names finite
elements and the multipole method as ways to find a mode's fields but not which one gave Fig. 1,
so that gap isn't explained yet.

On the grid there are two errors, told apart by giving the fields the exact mode's H at the grid's
nodes in place of the eigensolver's.

The first is how a cell gets E from H. Its $H_z$ comes from $\nabla\cdot\mathbf H$ over the cell
and its $E_z$ from $\nabla\times\mathbf H$; where the edge crosses the cell obliquely, those mix the
two sides' derivatives, which jump at the edge, and come out O(1) wrong at any grid. With the
exact H, the cells' $H_z$ is off by up to 0.22, 0.21 and 0.28 (where it is 0.24 to 0.34) at 10, 5
and 2.5 nm, while $\mathbf H_t$'s error falls (0.11, 0.042, 0.026). A difference of $E_z$ or $H_z$
across such cells then spikes as $1/h$. What converges takes E from D and smooths what is
continuous: $\mathbf D_t = (i/k)(\nabla\times\mathbf H)_t$ needs only H and $\partial H_z$, and a
cell of one material takes $\mathbf E_t = \mathbf D_t/\varepsilon$; in a cell the edge crosses,
$H_z$ and $E_z$ (both continuous across it) are the mean of the cells of one material within two
of it, and the cell is split into its two sides by the share of each and the edge's normal
$\hat n$, with $E_\parallel$ and $D_\perp$ the same on both sides (from the same neighbours),
$E_\perp = D_\perp/\varepsilon$ on each, and each side's $\lvert\mathbf E\rvert^4$ weighed by its own
$n^2 n_2$. $\gamma^V/\gamma^F$ for the same rod, against the exact 1.4127:

| Grid | Exact H: `fields_one_sided` | Exact H: smoothed | Eigensolver's mode: $n_\mathrm{eff}$ (exact 1.691643) | Its `fields_one_sided` | Its smoothed |
|---|---|---|---|---|---|
| 20 nm | 1.576 | 1.191 | 1.738927 | 1.314 | 1.133 |
| 10 nm | 1.783 | 1.345 | 1.728079 | 1.663 | 1.284 |
| 5 nm | 2.460 | 1.394 | 1.719673 | 11.92 | 1.331 |
| 2.5 nm | 17.71 | 1.408 | 1.714727 | 82.03 | 1.361 |
| 1.25 nm | 81.69 | 1.4119 | not run | not run | not run |

With the exact H, the smoothed fields converge to the exact ratio at second order (errors 16 %,
4.8 %, 1.3 %, 0.31 %, 0.059 %; orders 1.7, 1.9, 2.1 and 2.4), and Eq. 44's area with them (0.0681
to 0.0793 µm², against 0.0802). Leave out the smoothing of $H_z$ and $E_z$ and the split cells
still run away (1.51 and 2.11 at 2.5 and 1.25 nm with the exact H).

The second error is the mode's own. With the eigensolver's H, the smoothed ratio heads for the
exact one at order 0.6 to 1.1 (errors 20 %, 9.1 %, 5.8 %, 3.7 %), as slowly as the index does: 1.4 %
high at 2.5 nm, converging at order 0.3 to 0.4. Each cell takes the plain mean of its
permittivity, the average that did best for a thicker silicon rod's index at 1.55 µm
([stacks.md](stacks.md)), but this rod is near cut-off, where the index rises by 0.13 for each
10 nm of diameter, and the mean leaves it about a nanometre wider in radius at 5 nm.

So a converged $\bar n_2$ at an oblique interface needs both: the smoothing above in the library,
which needs each cell's share and the edge's normal (a cross-section keeps only the averaged
permittivity), and a mode that converges at an oblique interface. Until then `field_weighted_n2`
keeps its one-sided fields, right where the interfaces lie on the grid's lines
(`mode/kerr-index-slab-tm`). The study is
`mode::quantities_checks::tests::afshar_and_monros_rod_study` (`ROD_EXACT_H=1` for the exact H),
the exact fields `mode::quantities_checks::tests::afshar_and_monros_rod_exact`, and the cells'
$H_z$ `mode::quantities_checks::tests::rod_exact_h_diagnostic`.

**Which fields.** The integrals take the fields at the cells' centres. From
`VectorMode::fields_one_sided`, E's normal component keeps its jump at each interface and a
region's share converges at second order; from `VectorMode::fields`, a TM slab's confinement
converges at first order only (3.9e-2, 1.8e-2, 9.0e-3 relative at 20, 10 and 5 nm, against 7.0e-3,
1.4e-3 and 3.2e-4 one-sided). `confinement` takes the one-sided fields.

**Walls.** A window cut by a wall on one edge is half the cross-section: the area, γ and the
confinement add the mirror image, so half a slab behind a wall gives the whole slab's numbers. A
window closed by walls at both ends of an axis is taken as it is: a slab uniform between two walls
has the effective width times the window's height as its area. A box straddling a wall counts
with its image. Next to a wall, the derivative of $E_z$ across it takes the cell's mirror image as
its neighbour: $E_z$ is tangential to every wall, so beyond it $E_z$ is the image's value with its
parity, odd across an electric wall (zero on it) and even across a magnetic one. For a wall at
$x_0$ and the cell beside it, centred at $x_0 + h/2$,

$$
\partial_x E_z \approx \frac{E_z^{(1)} - s\thinspace E_z^{(0)}}{x^{(1)} - (2x_0 - x^{(0)})},
$$

with $s = -1$ for an electric wall and $s = 1$ for a magnetic one: the central difference the
whole window takes there. A folded window is then the whole window's discrete problem in its
fields too, not only in its index. For a 480 × 200 nm strip (case `mode/fields-folded-strip`),
the effective area, the confinement and the first-order index change from its halves and its
quarter agree with the whole window's to 2e-13 at 20, 10 and 5 nm. A one-sided difference beside
the wall would leave them off by up to 7.4e-3, 9.1e-4 and 1.1e-4 (the TM-like mode's confinement
behind an electric wall at y = 0, where its $E_z$ is odd): falling at third order, below the
grid's own second-order error, but not the whole window's numbers.

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

A change $\Delta\varepsilon$ of the cells' relative permittivity, inside each cell so that no
boundary moves, shifts a mode's frequency at fixed $\beta$ by
$\Delta\omega/\omega = -\tfrac12 \int \mathbf E^*\cdot\Delta\varepsilon\cdot\mathbf E\thinspace dA / \int \varepsilon|\mathbf E|^2 dA$
to first order (Johnson et al. 2002, before their moving boundaries). At fixed $\omega$ that is
a shift of $\beta$ of $\Delta\omega$ over the group velocity, the energy's,
$v_g = \tfrac12\mathop{\mathrm{Re}}\int(\mathbf E\times\mathbf H^*)\cdot\hat z\thinspace dA \big/ \tfrac12\varepsilon_0\int\varepsilon|\mathbf E|^2 dA$;
with E in units of $Z_0$ times H's,

$$
\Delta n_\mathrm{eff} = \frac{\int \mathbf E^*\cdot\Delta\varepsilon\cdot\mathbf E\thinspace dA}{2\mathop{\mathrm{Re}}\int (\mathbf E\times\mathbf H^*)\cdot\hat z\thinspace dA}.
$$

`mode::thermo::index_change` takes it from the fields above, but for one thing: beside a change
of material, $\partial_x E_z$ and $\partial_y E_z$ come from the neighbour of the same material
(one-sided), so E's normal component keeps its jump at the interface ($\varepsilon E_\perp$
continuous). A central difference across the interface spreads the jump over the two cells
beside it and leaves the overlap first order in the grid; one-sided, it is second order (2.07 and
2.23 on a TM slab, whose E crosses its interfaces). `index_change_resolved` solves the perturbed
cross-section (`CrossSection::perturbed`) instead, near the first-order estimate; the two differ
at second order in $\Delta\varepsilon$ and by the field's own error.

**Temperature.** `permittivity_change(cold, warm, step, rise)` takes the cross-section at a
temperature and the same one `step` K warmer, each with its materials at that temperature (the
catalogue's silicon and silica take one, [materials](materials.md#temperature)), and gives each
cell $\Delta\varepsilon = (\varepsilon_\mathrm{warm} - \varepsilon_\mathrm{cold})/\mathrm{step}
\times \Delta T(x, y)$, the rise at its centre: a uniform one, or a heat solve's field
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
  perturbation on the same slab at 5 nm, TE₀ and TM₀, within 2.9e-4 of the exact
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

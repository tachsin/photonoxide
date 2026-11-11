---
title: "Densities, filters and projections"
module: topology
summary: "A design region's pixels, each with a density between 0 and 1, filtered by a cone and projected towards 0 or 1 to the permittivity a solver sees; and the solver's adjoint gradient pulled back through the projection and the filter's transpose to the densities, checked against finite differences in 2D FDFD."
order: 23.7
papers:
  - cite: "F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (published online 24 December 2010; the June 2011 issue) (the filter, Eqs. 4–5; the chain rule, Eq. 7; the projection, Eq. 9)"
    doi: 10.1007/s00158-010-0602-y
  - cite: "R. E. Christiansen, O. Sigmund, J. Opt. Soc. Am. B 38, 496 (2021) (the projection, Eq. 18, and the permittivity's interpolation)"
    doi: 10.1364/JOSAB.406048
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004) (the adjoint gradient the chain starts from)"
    doi: 10.1364/OL.29.002288
validation:
  - topology/projection-derivative
  - topology/projection-identities
  - topology/filter-transpose
  - topology/filter-preserves-linear
  - topology/fdfd-gradient
---

Topology optimization gives every pixel of a design region its own variable, a density ρ
between 0 (one material) and 1 (the other), and lets an optimizer move all of them with the
adjoint gradient. Left alone, the densities form features a pixel wide and settle on grey values
between the two materials, neither of which can be made. Two operations before the solver keep
the design smooth and then push it towards 0 or 1 (Wang, Lazarov & Sigmund 2010, Section 3;
Christiansen & Sigmund 2021, Section 8.C). The plan they belong to is
[docs/plans/inverse-design.md](../plans/inverse-design.md), phase I1.

## The filter

The filtered density at pixel i is a weighted average over the pixels whose centres lie within a
radius R of its own (Wang et al., Eqs. 4 and 5):

$$
\tilde\rho_i = \frac{\sum_j w_{ij}\thinspace\rho_j}{\sum_j w_{ij}}, \qquad w_{ij} = R - \lvert x_j - x_i\rvert .
$$

The weights fall linearly to zero at R: a cone in 2D. `ConicFilter::new(shape, pixel_um,
radius_um, edge)` takes the region in pixels (1 along z for 2D) and how its edges are treated:

- `Edge::Renormalized`: only the region's own pixels are averaged, and each pixel's weights are
  normalized over those, as Eq. 4 with the neighbourhood cut off at the edge;
- `Edge::Padded(v)`: the region continues outside with the fixed density v, which the filter
  averages in and the optimization doesn't vary: 1 where a guide enters the region, 0 in the
  cladding. The filter is then affine, Fρ + c.

A radius below a pixel leaves the density as it is.

## The projection

The filtered density is projected with the threshold function of Wang et al.'s Eq. 9 (the tanh
form of Xu et al.'s; Christiansen & Sigmund's Eq. 18):

$$
\hat\rho = \frac{\tanh\beta\eta + \tanh\beta(\tilde\rho - \eta)}{\tanh\beta\eta + \tanh\beta(1 - \eta)} .
$$

η is the threshold and β the sharpness. At β = 1 it changes little; as β grows it tends to a
step at η. An optimization raises β in stages, the optimize job's
[continuation](optimization.md#continuation). `Projection::new(beta, eta)` takes a positive,
finite β and η in [0, 1]; `Projection::derivative` is

$$
\frac{d\hat\rho}{d\tilde\rho} = \frac{\beta\mathop{\mathrm{sech}}\nolimits^2\beta(\tilde\rho - \eta)}{\tanh\beta\eta + \tanh\beta(1 - \eta)},
$$

0 far from η where the cosh overflows. The tanh and cosh are the `libm` crate's, so a design's
bits don't depend on the system's maths library.

The solver sees $\varepsilon = \varepsilon_0 + \hat\rho\thinspace(\varepsilon_1 - \varepsilon_0)$, Christiansen &
Sigmund's interpolation: `Density::permittivity`.

## The gradient

A solver's adjoint gives ∂F/∂ε at each cell ([FDFD](fdfd-adjoint.md)) or ∂F/∂ρ̂ at each cell of
a design region ([FDTD](fdtd-adjoint.md), whose design interpolates the same two permittivities).
The chain rule (Wang et al.'s Eq. 7) takes it back to the densities:

$$
\frac{\partial F}{\partial \rho_j} = \sum_i \frac{\partial F}{\partial \hat\rho_i}\thinspace\frac{d\hat\rho_i}{d\tilde\rho_i}\thinspace\frac{\partial\tilde\rho_i}{\partial\rho_j}
= \Bigl[F^{\mathsf T}\bigl(\hat\rho'(\tilde\rho)\odot\partial F/\partial\hat\rho\bigr)\Bigr]_j ,
$$

the projection's derivative at each pixel, then the filter's transpose. Pixel j's weight in
pixel i's average is $w_{ij}/N_i$, with $N_i$ pixel i's sum of weights, so
$(F^{\mathsf T}g)_j = \sum_i w_{ij}\thinspace g_i/N_i$ over j's own neighbours: the neighbourhood is symmetric, so
each column is gathered, not scattered, and every pixel's sum is computed on its own, the same
bits on any number of threads. `Density::pullback(rho, d_physical)` takes ∂F/∂ρ̂;
`Density::gradient(rho, d_eps)` takes ∂F/∂ε and multiplies by ε₁ − ε₀ first. The gradient costs
two filters, nothing beside the solver's adjoint.

`Placement` puts a region's pixels on a solver's cells, one pixel a cell, ordered as the solvers
order theirs (i fastest, then j, then k): `embed` writes the permittivities into 2D FDFD's cells
for `Solver2d::from_cells` or `reuse_cells`, and `restrict` takes the region's part of
`Solver2d::mode_power_gradient`.

## Validation

| Case | What | Measured |
|---|---|---|
| `topology/projection-derivative` | the derivative against the complex step, Im ρ̂(x + ih)/h, at 201 points for β = 1, 8, 64, 256 and η = ¼, ½, ¾, relative to the largest | 4.1e-16 (held to 1e-12) |
| `topology/projection-identities` | ρ̂(0) = 0, ρ̂(1) = 1, and ρ̂(1 − x) = 1 − ρ̂(x) at η = ½ | 2.2e-16 (held to 1e-14) |
| `topology/filter-transpose` | ⟨Fx, y⟩ against ⟨x, Fᵀy⟩ on 160 × 160 and 24 × 20 × 16 pixels, renormalized and padded | 1.2e-14 relative (held to 1e-12) |
| `topology/filter-preserves-linear` | a uniform density everywhere, and a linear ramp a radius from the edges | 7.2e-16 (held to 1e-14) |
| `topology/fdfd-gradient` | 2D FDFD (E along z, 20 nm): a port mode's power against a 10 × 10 region's densities beside a 220 nm slab, filtered over 3 pixels and projected at β = 8, against fourth-order differences of the densities (δ = 2 × 10⁻³) at a corner, an edge and the middle | 4.9e-10 relative, below the differences' resolution of 2.8e-9 (held to 1e-8) |

**The complex step** takes no difference, so it checks the derivative to round-off at every
β: $f'(x) = \mathop{\mathrm{Im}} f(x + ih)/h$ with $h = 10^{-30}$ for any analytic f.

**The filter's identities.** The weights are normalized, so a uniform density is kept everywhere,
padded or not; they are symmetric about the pixel, so a linear function's first moment cancels
wherever the whole neighbourhood is inside. Both hold to round-off.

**The chain against differences.** Fourth-order central differences of the densities resolve
the gradient only so far: the powers' own precision, divided by δ through the stencil's weights,
gives a resolution that falls as 1/δ, and the stencil's truncation rises as δ⁴. A step study at
nine values of δ from 10⁻⁴ to 3.2 × 10⁻² (`the_chain_step_study`, an ignored test, in release)
on the case's three pixels, 2D FDFD at 20 nm:

| δ | largest error | largest resolution |
|---|---|---|
| 10⁻⁴ | 1.3e-8 | 5.6e-8 |
| 10⁻³ | 1.5e-9 | 5.6e-9 |
| 2 × 10⁻³ | 4.9e-10 | 2.8e-9 |
| 4 × 10⁻³ | 5.9e-10 | 1.4e-9 |
| 1.6 × 10⁻² | 1.1e-7 | 3.5e-10 |
| 3.2 × 10⁻² | 1.7e-6 | 1.8e-10 |

Up to 4 × 10⁻³ every error is below the resolution: the adjoint and the differences agree as far
as the differences can tell. Above it the truncation shows, 1.6 δ⁴ at the corner pixel (×15.9 for
a doubled δ, against the 16 of δ⁴); it meets the resolution, 5.6 × 10⁻¹²/δ there, near
δ = 5 × 10⁻³. The case takes δ = 2 × 10⁻³, where the truncation is 1 % of the resolution, and holds
the error to 10⁻⁸, a few times the resolution.

## Limits

- One pixel per solver cell. A coarser design grid is a linear map before the filter, and its
  gradient that map's transpose.
- The permittivities are real: the solvers' gradients are with respect to its real part.
- The conic filter only. The Helmholtz-equation filter of Lazarov & Sigmund (the one Christiansen
  & Sigmund's Eq. 17 solves) comes with the robust demultiplexer it is used for (the plan's I4).

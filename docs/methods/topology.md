---
title: "Densities, filters, projections, robust designs and length scales"
module: topology
summary: "A design region's pixels, each with a density between 0 and 1, filtered by a cone and projected towards 0 or 1 to the permittivity a solver sees; the solver's adjoint gradient pulled back through the projection and the filter's transpose to the densities, checked against finite differences in 2D FDFD; robust designs: the eroded, nominal and dilated realizations, the length scale and edge displacement their thresholds impose, and the epigraph form of their worst case; and the minimum length scale of any binary design, measured after the fact by morphological opening and closing, which reproduces the testbed's published lengths bit for bit."
order: 23.7
papers:
  - cite: "F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (published online 24 December 2010; the June 2011 issue) (the filter, Eqs. 4–5; the chain rule, Eq. 7; the projection, Eq. 9)"
    doi: 10.1007/s00158-010-0602-y
  - cite: "R. E. Christiansen, O. Sigmund, J. Opt. Soc. Am. B 38, 496 (2021) (the projection, Eq. 18, and the permittivity's interpolation)"
    doi: 10.1364/JOSAB.406048
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004) (the adjoint gradient the chain starts from)"
    doi: 10.1364/OL.29.002288
  - cite: "O. Sigmund, Acta Mech. Sin. 25, 227 (2009) (eroded, intermediate and dilated designs)"
    doi: 10.1007/s10409-009-0240-z
  - cite: "X. Qian, O. Sigmund, Comput. Methods Appl. Mech. Engrg. 253, 237 (2013) (the minimum length, Appendix, Eq. 29)"
    doi: 10.1016/j.cma.2012.08.020
  - cite: "M. Chen et al., J. Opt. Soc. Am. B 41, A161 (2024) (the a-posteriori minimum length scale, Section 2, Eqs. 1–3; the testbed's mode converter, Section 3.C)"
    doi: 10.1364/JOSAB.506412
validation:
  - topology/projection-derivative
  - topology/projection-identities
  - topology/filter-transpose
  - topology/filter-preserves-linear
  - topology/fdfd-gradient
  - topology/robust-edge-1d
  - topology/robust-edge-2d
  - topology/robust-length
  - topology/robust-epigraph
  - topology/ruler-shapes
  - topology/ruler-testbed
---

Topology optimization gives every pixel of a design region its own variable, a density ρ
between 0 (one material) and 1 (the other), and lets an optimizer move all of them with the
adjoint gradient. Left alone, the densities form features a pixel wide and settle on grey values
between the two materials, neither of which can be made. Two operations before the solver keep
the design smooth and then push it towards 0 or 1 (Wang, Lazarov & Sigmund 2010, Section 3;
Christiansen & Sigmund 2021, Section 8.C). The plan they belong to is
[docs/plans/inverse-design.md](../plans/inverse-design.md), phases I1 to I3.

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

## Robust designs

A design is made with its edges a little off: an etch that runs long narrows every feature, one
that runs short widens it. Wang et al. (Section 5.1, Eq. 14) and Sigmund (2009) optimize three
realizations of one density at once, its filtered density projected at three thresholds: 1 − η
(eroded, features shrink), ½ (the nominal design, the one drawn) and η (dilated, features grow),
η below ½. `Density::robust(eta)` gives the three as `Realizations { eroded, nominal, dilated }`,
each with the density's filter and its projection's β; each realization's gradient is its own
`Density::gradient`.

**How far an edge moves.** Across a straight edge of a solid region, the filtered density at a
distance x outside is the share of the filter's weight that reaches inside, and a threshold
moves the edge to where that share equals it. For the 1D hat it is ½ (1 − t)², t = x/R, so the
edge moves out by R(1 − √(2η)) for η ≤ ½ (`edge_shift_1d`). For the 2D cone, R − r over a disc,
the weight at distance s from the edge is the cone's marginal $m(s) = R\sqrt{R^2 - s^2} - s^2 \mathop{\mathrm{arcosh}}(R/s)$, and its integral gives

$$
\tilde\rho(t) = \frac12 - \frac{2t\sqrt{1 - t^2} + \arcsin t - t^3 \mathop{\mathrm{arcosh}}(1/t)}{\pi}, \qquad 0 \le t \le 1,
$$

½ at the edge, 0 at one radius, and $1 - \tilde\rho(-t)$ inside (worked out for this module:
`across_edge_2d`; `edge_shift_2d(eta)` solves it for the shift, to round-off). At η = ¼ the 1D
hat moves an edge 0.293 R and the cone 0.283 R.

**The minimum length.** When the eroded design just loses a feature, the nominal design keeps
it at a width b, the smallest the robust problem allows. With the 1D hat, Qian & Sigmund's
Eq. 29 (their Appendix) gives

$$
\frac{b}{R} = \begin{cases} 2 - 2\sqrt{\eta}, & \eta \le \tfrac14, \\ 2\sqrt{\tfrac12 - \eta}, & \tfrac14 \le \eta < \tfrac12, \end{cases}
$$

`minimum_length(eta)`: 1.1056 at η = 0.2. Wang et al.'s Fig. 12 plots the same curve with its axis
labelled b/2R and quotes 1.1055 at η = 0.2; Qian & Sigmund's Table 1 and Fig. 19 read that curve
as b/R, which is what the derivation gives (photonoxide's own working, and the bars below, agree
with Eq. 29 as b/R). Wang et al. take the same b as the diameter of the smallest disc in 2D.

**The worst case.** The robust problem minimizes the worst of the three objectives, a max that
isn't smooth where two of them cross. Its epigraph form is smooth: the variables (x, t), the
objective t, and one constraint f_k(x) − t ≤ 0 per objective, with the Jacobian rows
[∇f_k, −1]. `Epigraph::new(n, k)` and `Epigraph::evaluate` write the objective's gradient, the
constraints and their Jacobian in the arrays an optimizer with constraints takes (genoxide's
`Mma` and `Constrained`), and `Epigraph::start(values)` starts t at the worst value. The same
form takes the worst over wavelengths and corners.

## The minimum length scale of a design

A design is finally made from its binary pattern, and what a foundry checks is its smallest
feature and its narrowest gap. Chen et al. (2024, Section 2) measure both after the fact, whatever
algorithm made the design: the morphological opening of the pattern by a disc of diameter d
erases solid features narrower than d and keeps the rest, and the closing fills voids narrower
than d. Where the two differ inside the solid or the void (the pixels next to an interface left
out, so a staircased edge is no violation), the pattern violates d (their Eq. 3); its minimum
length scale is the smallest d with a violation, the smaller of the solid's and the void's, found
by bisection.

`ruler::minimum_length(&pattern, pixel)` measures a `ruler::Pattern` (a binary image, or
`Pattern::binarized` from densities at the midpoint of their range) in the pixel's unit, with the
discretisation of the paper's software, imageruler, in the version the testbed's published lengths
were measured with (April to November 2023): the disc drawn on round(d/Δ) pixels, a full square
below 3; the opening padded with solid and the closing with void; the interior a plus-shaped
kernel's erosion of the solid and dilation's complement in the void; the bisection between a pixel
and the image's shorter side, stopped at half a pixel. Each of the disc's rows is a run of pixels,
so an erosion or a dilation is a count per row from a prefix sum: a 350 × 300 image, measured from
a 300-pixel disc down, takes a quarter of a second. `ruler::violates(&pattern, pixel, d)` says
whether one d is violated.

The testbed's waveguide mode converter (Chen et al., Section 3.C) publishes its designs and each
one's minimum length scale, measured on the model's whole domain: the 1.6 µm design between its
two waveguides, at 10 nm. Eight of them, with lengths from 90.30 to 207.09 nm, are in the
repository as data (MIT licence; `src/cases/topology/testbed/`), and their measured lengths are the
printed ones to the last bit (`topology/ruler-testbed`).

## Validation

| Case | What | Measured |
|---|---|---|
| `topology/projection-derivative` | the derivative against the complex step, Im ρ̂(x + ih)/h, at 201 points for β = 1, 8, 64, 256 and η = ¼, ½, ¾, relative to the largest | 4.1e-16 (held to 1e-12) |
| `topology/projection-identities` | ρ̂(0) = 0, ρ̂(1) = 1, and ρ̂(1 − x) = 1 − ρ̂(x) at η = ½ | 2.2e-16 (held to 1e-14) |
| `topology/filter-transpose` | ⟨Fx, y⟩ against ⟨x, Fᵀy⟩ on 160 × 160 and 24 × 20 × 16 pixels, renormalized and padded | 1.2e-14 relative (held to 1e-12) |
| `topology/filter-preserves-linear` | a uniform density everywhere, and a linear ramp a radius from the edges | 7.2e-16 (held to 1e-14) |
| `topology/fdfd-gradient` | 2D FDFD (E along z, 20 nm): a port mode's power against a 10 × 10 region's densities beside a 220 nm slab, filtered over 3 pixels and projected at β = 8, against fourth-order differences of the densities (δ = 2 × 10⁻³) at a corner, an edge and the middle | 4.9e-10 relative, below the differences' resolution of 2.8e-9 (held to 1e-8) |
| `topology/robust-edge-1d` | the 1D hat's density across an edge on a pixel boundary against ½ (1 − t)² less the pixels' exact offset (Δ/R)²/8, at R/Δ = 5, 20 and 80 | within round-off (1.1e-16; held to 1e-14) |
| `topology/robust-edge-2d` | the cone's density across a straight edge against the closed form above, at R/Δ = 40 | 3.5e-5 (held to (Δ/R)²/8 = 7.8e-5) |
| `topology/robust-length` | Eq. 29's b/R against bars of whole pixels at R/Δ = 100, η from 0.1 to 0.45 | 9.3e-5 (held to 4 (Δ/R)² = 4e-4) |
| `topology/ruler-shapes` | a 20-pixel bar, a 30-pixel gap and a 15-pixel ring against their own widths | 0.63 pixel at most (held to a pixel; the bar 20.63, the gap 30.34, the ring 15.19) |
| `topology/ruler-testbed` | eight of the testbed's mode converters, in their domain at 10 nm, against the README's printed lengths, 90.2978515625 to 207.0947265625 nm | 0 (held to 5e-11 nm, half the last printed digit) |
| `topology/robust-epigraph` | the epigraph's Jacobian through the three realizations' FDFD gradients (thresholds 0.75, 0.5, 0.25) against differences of each realization's power, δ = 4 × 10⁻³ | 8.1e-9 relative, below the resolution of 6.7e-8 (held to 3e-7) |

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

**The pixels' own error.** On pixels the filter is a sum, not an integral, and the robust cases
measure how far that sum is from the closed forms as the radius grows in pixels
(`the_robust_study`, an ignored test, in release):

| R/Δ | 1D, across an edge | 2D, across an edge | Eq. 29's b/R |
|---|---|---|---|
| 5 | 5.0e-3 = (Δ/R)²/8 | 4.2e-3 = 0.105 (Δ/R)² | |
| 20 | 3.1e-4 = (Δ/R)²/8 | 1.7e-4 = 0.067 (Δ/R)² | |
| 25 | | | 4.6e-3 = 2.9 (Δ/R)² |
| 40 | 7.8e-5 = (Δ/R)²/8 | 3.5e-5 = 0.056 (Δ/R)² | |
| 50 | | | 7.0e-4 = 1.8 (Δ/R)² |
| 80 | 2.0e-5 = (Δ/R)²/8 | 9.6e-6 = 0.062 (Δ/R)² | |
| 100 | | | 9.3e-5 = 0.93 (Δ/R)² |
| 200 | | | 6.1e-5 = 2.4 (Δ/R)² |

In 1D the offset is exact: with R a whole number of pixels and the edge on a pixel boundary, the
sum over whole pixels is the integral less Δ²/8. In 2D the error is second order with no exact
form, below the 1D offset at every radius. Eq. 29 is checked on bars of whole pixels: the last bar
the eroded design removes and the first it keeps are interpolated in their peaks to the
threshold, since a bar rounded up to a whole pixel would move b by up to two pixels.

## Limits

- One pixel per solver cell. A coarser design grid is a linear map before the filter, and its
  gradient that map's transpose.
- The permittivities are real: the solvers' gradients are with respect to its real part.
- The length scale is measured in 2D, on a binary pattern: a grey design is binarized first, and
  a 3D design's layers would be measured one by one.
- The closed forms of the edge and of Eq. 29 are for a straight edge and a bar: corners and
  curved edges move by other amounts, which I3's length-scale metric measures on the design.
- The conic filter only. The Helmholtz-equation filter of Lazarov & Sigmund (the one Christiansen
  & Sigmund's Eq. 17 solves) comes with the robust demultiplexer it is used for (the plan's I4).

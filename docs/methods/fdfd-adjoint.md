---
title: "FDFD adjoint gradients"
module: fdfd
summary: "How a port's power changes with every cell's permittivity, from one more back-substitution: the adjoint variable method on the 2D FDFD system, checked against finite differences."
order: 17
papers:
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004)"
    doi: 10.1364/OL.29.002288
  - cite: "C. M. Lalau-Keraly, S. Bhargava, O. D. Miller, E. Yablonovitch, Opt. Express 21, 21693 (2013)"
    doi: 10.1364/OE.21.021693
  - cite: "T. W. Hughes, M. Minkov, I. A. D. Williamson, S. Fan, ACS Photonics 5, 4781 (2018)"
    doi: 10.1021/acsphotonics.8b01522
validation:
  - fdfd/adjoint-gradient-ez
  - fdfd/adjoint-gradient-hz
---

Inverse design changes a device's permittivity cell by cell to improve what it does. Each step
needs the objective's gradient with respect to every cell. Perturbing the cells one at a time
would take a solve per cell. The adjoint variable method takes one more back-substitution for
all of them.

## The method

The [FDFD](fdfd.md) system is $A(\varepsilon)\,u = b$, with the source b independent of ε. For an
objective F(u), Veronis, Dutton and Fan's Eqs. 2–4 give

$$
\frac{\partial F}{\partial \varepsilon_k} = -2\,\operatorname{Re}\!\left(\lambda^{\mathsf T}\,\frac{\partial A}{\partial \varepsilon_k}\,u\right),
\qquad A^{\mathsf T}\lambda = \frac{\partial F}{\partial u}.
$$

λ is the adjoint field: the response to a source placed where the objective is measured. The
factorization of A that gave u also solves with $A^{\mathsf T}$, so the whole gradient costs one more
back-substitution. Hughes et al. (Eqs. 9–13) write the same for FDFD; Lalau-Keraly et al. derive
it from the fields' reciprocity.

**The objective** is the power a [port](fdfd-ports.md) mode carries one way, $F = |a|^2$. Its
amplitude a is linear in the field, $a = c^{\mathsf T} u$: the projection on the port's two columns.
So $\partial F/\partial u = \bar a\, c$.

**∂A/∂ε** for a problem given cell by cell (`Solver2d::from_cells`, the form an optimization
varies):
- E along z: the cell's permittivity sits on the diagonal as $k_0^2\varepsilon_k$.
- H along z: it enters through the faces' couplings $1/(\varepsilon_f\,s\,s\,\Delta^2)$. Each face's
  $\varepsilon_f$ is the mean of its two cells, so a cell collects half of each of its faces'
  derivatives, or all of it for a face on a wall.

`Solver2d::mode_power_gradient(field, mode, direction)` returns |a|² and its gradient for every
cell. `Solver2d::reuse_cells` rebuilds the problem with new permittivities, reusing the analysis of
the matrix's sparsity, as each step of an optimization does.

## Validation

A straight silicon slab (220 nm, 3.476 in 1.444) with a 0.2 × 0.2 µm bump of permittivity 6 beside
it, on a 20 nm grid, between two ports. The objective is the power delivered into the right port's
mode from the left's. The adjoint gradient is compared with fourth-order central finite
differences ($\delta = 10^{-3}$) on a cell in the bump, one in the core and one in the oxide:

| | largest relative difference |
|---|---|
| E along z | 1.4e-7 |
| H along z | 8e-8 |

That is the finite differences' own limit. Their round-off grows as about $10^{-10}/\delta$, and with
$\delta = 10^{-5}$ and a second-order stencil the difference is 7e-6, all of it round-off. The
adjoint gradient is exact to that.

## Limits

- The cells varied must be away from the ports, whose sources and modes are taken as fixed.
- The gradient is with respect to the permittivity's real part. A lossy design would need the
  imaginary part's as well.
- One objective per adjoint solve. A sum of several (several ports, wavelengths) is a sum of
  their gradients.

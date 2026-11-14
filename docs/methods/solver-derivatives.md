---
title: "The other solvers' derivatives"
module: mode
summary: "Exact derivatives where the adjoints of FDFD, FDTD and circuits didn't reach: the mode solvers' effective index against every cell and node, RCWA's and eigenmode expansion's outputs by forward mode, FDFD's field by forward mode, and a Kerr medium in 2D FDFD with its nonlinear adjoint."
order: 23.8
papers:
  - cite: "T. W. Hughes, M. Minkov, I. A. D. Williamson, S. Fan, ACS Photonics 5, 4781 (2018)"
    doi: 10.1021/acsphotonics.8b01522
  - cite: "T. W. Hughes, I. A. D. Williamson, M. Minkov, S. Fan, ACS Photonics 6, 3010 (2019)"
    doi: 10.1021/acsphotonics.9b01238
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008)"
    doi: 10.1109/JLT.2008.923643
  - cite: "P. Bienstman, R. Baets, Opt. Quantum Electron. 33, 327 (2001)"
    doi: 10.1023/A:1010882531238
  - cite: "S. G. Johnson, M. Ibanescu, M. A. Skorobogatiy, O. Weisberg, J. D. Joannopoulos, Y. Fink, Phys. Rev. E 65, 066611 (2002)"
    doi: 10.1103/PhysRevE.65.066611
validation:
  - mode/slab-index-derivative
  - mode/profile-index-gradient
  - mode/vector-index-gradient
  - mode/rcwa-tangent-multilayer
  - mode/rcwa-tangent-gradient
  - mode/rcwa-tangent-born
  - eme/s-matrix-tangent
  - fdfd/forward-mode
  - fdfd/kerr-uniform
  - fdfd/kerr-gradient
---

An optimizer needs the derivative of what it measures. The [FDFD](fdfd-adjoint.md),
[FDTD](fdtd-adjoint.md) and [circuit](circuit-adjoint.md) adjoints give it for those solvers.
This page covers the others: the mode solvers, RCWA, eigenmode expansion, and FDFD with a Kerr
medium. It also covers forward mode, which is the right tool when a few parameters move many
outputs.

Every derivative here is exact for the discrete problem the solver solves. Each one is checked
against differences of the solver itself, and where a closed form exists, against that.

## Two directions

A solver maps parameters p to outputs F through a state u, with g(u, p) = 0. The Jacobian
dF/dp can be taken in two orders (Hughes et al. 2019, Eqs. 5–14):

- **The adjoint (reverse mode)** solves once per output, for an adjoint state, and then gives
  that output's derivative in every parameter. It wins when there are many parameters and few
  outputs: a figure of merit over a design region.
- **Forward mode** solves once per parameter, for du/dp, and then gives every output's
  derivative in that parameter. It wins when there are few parameters and many outputs: all
  the orders of a grating, every entry of an S-matrix, a whole field.

Both give the same numbers, the same Jacobian evaluated in the two orders.

## The mode solvers

A mode is an eigenpair of a matrix M whose eigenvalue is β², M x = λ x. For a simple
eigenvalue, the first-order perturbation is

$$
d\lambda = \frac{y^{\mathsf T}\thinspace dM\thinspace x}{y^{\mathsf T} x},
\qquad M^{\mathsf T} y = \lambda y,
$$

with y the left eigenvector and the unconjugated product. For a Hermitian M this is
Hellmann–Feynman's $x^{\mathsf H} dM x / x^{\mathsf H} x$. Then $n_\mathrm{eff} = \beta/k$ gives
$dn_\mathrm{eff} = d\lambda / 2k^2 n_\mathrm{eff}$. One left eigenvector serves every parameter.

**The 1D profile solver** (`Profile::index_gradient`). Its matrix is $M = W^{-1}H$ with H
symmetric and W diagonal: the weights its equations are divided through by, $(w + e)/2$ for TE
and $(w/\varepsilon_w + e/\varepsilon_e)/2$ for TM. So $y = Wx$ is the left eigenvector, at no
cost. Without loss or a PML, H is real symmetric and W positive, and the formula is
Hellmann–Feynman's for the pencil (H, W). The derivative is given for every cell's permittivity
and every interior node's position. A node's position is a shape derivative: moving the node on
an interface moves the interface.

**The full-vector solver** (`VectorMode::index_gradient`). Fallahkhair et al.'s matrix couples
H_x and H_y through unequal stencils, so it isn't symmetric, and a PML makes it complex. The
left eigenvector comes from inverse iteration with $A^{\mathsf T}$ shifted $10^{-7}|\lambda|$
off λ, started from the mode. The derivative is for an isotropic change δε·I of each cell, which
is what a density or a material changes.

**dM, exactly.** The matrix's entries are rational functions of the permittivities and the
spacings. They are evaluated a second time on dual numbers, $a + b\epsilon$ with $\epsilon^2 = 0$,
through the same code that assembles the matrix. The dual parts are the entries' exact
derivatives. A cell touches only the rows of its corner nodes, so the gradient over every cell
costs one pass over the grid.

`thermo::index_change` gives the same derivative from the continuous perturbation formula on
the mode's fields (Johnson et al. 2002). The two agree to the field's discretization error. The
adjoint agrees with the solver exactly.

**The slab, in closed form** (`SlabMode::derivatives`). A three-layer slab's mode solves its
characteristic equation $C(n_\mathrm{eff}; t, n_1, n_2, n_3) = ht - \arctan(\bar q/h) - \arctan(\bar p/h) - m\pi = 0$.
Along the mode C stays zero, so by the implicit function theorem
$dn_\mathrm{eff}/d\theta = -(\partial C/\partial\theta)/(\partial C/\partial n_\mathrm{eff})$ for
the thickness and each index. This is the closed form the profile solver's shape derivative is
checked against.

## RCWA

`Grating::diffract_tangents` differentiates the diffraction along a few parameters: a stripe's
permittivity or a layer's thickness. It is forward mode, because one pass gives every order's
amplitudes and efficiencies. Each step of the method is differentiated as it is computed:

- A stripe's permittivity moves the Toeplitz matrix E linearly, and $A = [\negthinspace[1/\varepsilon]\negthinspace]$
  by $-1/\varepsilon^2$ times the same share. Then $dE^{-1} = -E^{-1} dE\thinspace E^{-1}$, and
  the layer's matrix moves by $-dE$ (TE) or $A^{-1}(K_x dE^{-1} K_x - dA\thinspace M)$ (TM, Li's
  rules).
- The modes: with $MW = W\Lambda$ and $C = W^{-1} dM\thinspace W$, the eigenvalues move by
  $C_{ii}$ and the eigenvectors by $dW = WD$, $D_{ij} = C_{ij}/(\lambda_j - \lambda_i)$. The
  eigenvectors' scale is free and the outputs don't depend on it, so $D_{ii} = 0$. Then
  $dq = d\lambda/2q$.
- The interfaces, the layers' propagation $e^{-qk_0 d}$ and the star products follow by the
  product rule and $d(X^{-1}) = -X^{-1}dX\thinspace X^{-1}$.
- An efficiency $|r_m|^2\thinspace\mathrm{Re}(k_{z,m})/k_{z,0}$ moves by
  $2\thinspace\mathrm{Re}(\bar r_m\thinspace dr_m)$ times the same factor.

A degenerate pair of a layer's modes has no eigenvector derivative. At normal incidence a
grating's ±m orders can be degenerate, and `diffract_tangents` then says so; a small angle
avoids it. A layer whose stripes are all one index is solved as a uniform layer, so a stripe of
it can't be a parameter: give it as one stripe.

**The weak grating.** A grating of contrast Δ in its own medium scatters into its first order,
to first order in Δ (the first Born approximation), the amplitude
$t_1 = (ik_0^2\varepsilon_1/2k_{z1})\int_0^d e^{i(k_{z0} - k_{z1})z}dz$, with
$|\varepsilon_1| = \Delta\sin(\pi f)/\pi$. So $\eta_1 \propto \Delta^2$ and
$d\eta_1/d\Delta = 2\eta_1/\Delta$, up to a correction first order in Δ.

## Eigenmode expansion

`Eme::s_matrix_tangent` gives the derivative of every entry of a structure's S-matrix along one
parameter of one section: its length, a layer's index, or where a boundary between two layers
lies (the window's width kept). It is forward mode through the cascade.

- A length moves only its section's propagation, $\mathrm{diag}(e^{i\beta L})$, by
  $i\beta\thinspace e^{i\beta L}$.
- A slice's parameter moves the section's modes. Each mode's $\nu = n_\mathrm{eff}^2$ is a root of
  the Wronskian $W(\nu, p)$ of the sweeps from the two walls, so it moves by
  $d\nu = -(\partial W/\partial p)/(\partial W/\partial\nu)$. Both partials come from the same
  sweeps on dual numbers. The states at the layers' boundaries, the field inside each layer and
  the normalization $\langle E, H\rangle = 1$ are differentiated the same way.
- The overlaps $O = \langle E^\mathrm{I}, H^\mathrm{II}\rangle$ of the section's two interfaces
  move by the overlap of the fields' derivatives at fixed x. Where a boundary of the slice moves
  and the integrand jumps across it (TM's $E_x = n_\mathrm{eff} H_y/n^2$), the jump times the
  motion is added (Leibniz's rule); H is continuous in both polarizations.
- The interfaces' blocks $R = (1 + OO^{\mathsf T})^{-1}(1 - OO^{\mathsf T})$,
  $T = 2O^{\mathsf T}(1 + OO^{\mathsf T})^{-1}$ and their twins, and the star products, follow by
  the product rule.

A bent section's slice has no derivatives here, only its length.

## FDFD

**Forward mode** (`Solver2d::field_tangent`): $du = -A^{-1}(\partial A/\partial\varepsilon\cdot\delta\varepsilon)\thinspace u$
along a change δε of the cells, one more solve with the factors (Hughes et al. 2019, Eqs. 5–9,
in the frequency domain). Any linear measure of du, such as a mode's amplitude, is that measure's
derivative. With H along z, ∂A/∂ε acts through the faces' couplings, as the adjoint has it.

**A Kerr medium** (`Kerr2d`). An intensity-dependent permittivity
$\tilde\varepsilon = \varepsilon + \chi|u|^2$ cell by cell (Hughes et al. 2018, Eq. 14; χ is
3χ⁽³⁾ for a field in V/m, and here the change of ε per unit $|u|^2$ of the solver's field) turns
the system into $f(u, \bar u) = A(\varepsilon + \chi|u|^2)\thinspace u - b = 0$. With E along z,
ε enters A only on its diagonal.

- **Newton's method**, with u and ū independent (their Eqs. 4–5): $J = \partial f/\partial u = A(\varepsilon + 2\chi|u|^2)$
  and $K = \partial f/\partial\bar u = k_0^2\thinspace\mathrm{diag}(\chi u^2)$, so a step δ solves
  $J\delta + K\bar\delta = -f$. That system is linear over the reals but not over the complex
  numbers. It is solved by GMRES on the real vector space, preconditioned by J's own factors. A
  step that doesn't lower $\|f\|$ is halved. A Kerr device can have several solutions; Newton
  finds the one its start leads to, so a sweep of power should start each solve from the last.
- **The adjoint** of $F = |c^{\mathsf T}u|^2$: λ solves $J^{\mathsf T}\lambda + \bar K\bar\lambda = \bar a\thinspace c$
  (their Eq. 6), again linear over the reals and solved the same way, and
  $dF/d\varepsilon_k = -2\thinspace\mathrm{Re}(\lambda_k k_0^2 (1 + \chi'_k|u_k|^2)\thinspace u_k)$.
  χ′ = ∂χ/∂ε is for a nonlinearity that follows the material, as their Eq. 18's density does.

The sources and the ports must lie where χ = 0, and the cells varied away from them.

## Validation

Each case's tolerance follows from what limits it. Against differences, it is ten times what the
differences resolve (the gap between steps δ and 2δ), rounded up; against a closed form with a
grid or a small parameter, the measured coefficient of its order, rounded up.

| Case | What | Ours | Held to |
|---|---|---|---|
| `mode/slab-index-derivative` | the profile's dn/dt against the closed form, at 2.5 nm | 3.0e-4 (2.0 to 2.3 (h/t)², 20 to 2.5 nm) | 4e-4 (3 (h/t)², rounded up) |
| `mode/profile-index-gradient` | cells and nodes against differences | 7.3e-11 (they resolve 1.2e-10) | 2e-9 |
| `mode/vector-index-gradient` | cells against differences | 4.1e-11 (they resolve 6.8e-11) | 1e-9 |
| `mode/rcwa-tangent-multilayer` | uniform films against the transfer matrices' exact derivatives | round-off | 1e-12 |
| `mode/rcwa-tangent-gradient` | lamellar layers against differences | 1.9e-9 (they resolve 2.8e-8) | 5e-7 |
| `mode/rcwa-tangent-born` | a weak grating against the Born approximation, Δ = 2.5e-3 | 6.0e-4 (0.240 Δ) | 0.3 Δ, 7.5e-4 |
| `eme/s-matrix-tangent` | length, index and boundary against differences | 5.4e-12 (they resolve 5.0e-11) | 5e-10 |
| `fdfd/forward-mode` | forward mode against the adjoint | 7.6e-14 | 1e-12 |
| `fdfd/kerr-uniform` | a uniform Kerr medium against its scalar root | 8.9e-15 | 1e-11 |
| `fdfd/kerr-gradient` | the Kerr adjoint against differences through Newton | 4.3e-10 (they resolve 4.4e-10) | 5e-9 |

## Limits

- A degenerate eigenvalue has no derivative: the mode solvers' and RCWA's derivatives are for
  simple ones.
- Hughes et al. 2018's own switches can't be reproduced here: the paper gives their
  materials and powers but not the designs. The method is checked on its own problems.
- The Kerr problem is E along z only, and its χ is in the solver's field units.

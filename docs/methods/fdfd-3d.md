---
title: "FDFD in 3D"
module: fdfd
summary: "Maxwell's equations at one frequency on Yee's 3D grid: the electric field's curl-curl equation as one sparse system, with stretched-coordinate PMLs or Bloch-periodic sides on each axis, and the exact discrete power flux."
order: 19
papers:
  - cite: "A. Christ, H. L. Hartnagel, IEEE Trans. Microw. Theory Tech. 35, 688 (1987)"
    doi: 10.1109/TMTT.1987.1133733
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "W. Shin, S. Fan, J. Comput. Phys. 231, 3406 (2012)"
    doi: 10.1016/j.jcp.2012.01.013
  - cite: "W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994)"
    doi: 10.1002/mop.4650071304
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the exact reference)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - fdfd3d/film-reflection-te
  - fdfd3d/film-reflection-tm
  - fdfd3d/flux-conservation
  - fdfd3d/pml-reflection
  - fdfd3d/two-d-agreement
---

In 3D the fields no longer split into two polarizations: all six components are coupled. FDFD
in 3D solves for the three components of E on Yee's grid at once, as one sparse linear system,
the way Christ and Hartnagel set it up. It is the [2D solver](fdfd.md) without the invariant
axis. The 2D solver is what it reduces to when the structure is uniform along one axis, and the
3D solver reproduces it there to round-off.

## The method

Christ and Hartnagel (Section III) take Maxwell's equations in integral form (their Eqs. 6a–6b)
on Yee's cell (Yee's Fig. 1, their Fig. 2): E at the middle of the cells' edges, H at the
centres of their faces. Each H is then the circulation of E around its face, and each E the
circulation of H around its edge of the dual grid. Eliminating H leaves every E component tied
to itself and 12 neighbours (their Eq. 7). That is one sparse system for E alone (their Eq. 9a),
three unknowns per cell and 13 nonzeros per row. With $\tilde H = \eta_0 H$ and photonoxide's
$e^{-i\omega t}$ (their $e^{+j\omega t}$ makes their j our −i):

$$
\nabla \times \mathbf E = i k_0 \tilde{\mathbf H}, \qquad
\nabla \times \tilde{\mathbf H} = -i k_0 \varepsilon \mathbf E + \mathbf J
\quad\Longrightarrow\quad
-\nabla \times \nabla \times \mathbf E + k_0^2 \varepsilon \mathbf E = -i k_0 \mathbf J,
$$

with $\mathbf J = \eta_0 \mathbf J$ the electric current. A magnetic current M
($\nabla \times \mathbf E = i k_0 \tilde{\mathbf H} - \mathbf M$) adds $\nabla \times \mathbf M$
on the right. On the staggered grid every derivative is a centred difference, so the scheme is
second order.

Christ and Hartnagel's structures were shielded by a metal envelope, which their equations hold
as a boundary condition. Light needs open boundaries, so photonoxide adds the same PMLs as in 2D.

**Interfaces.** As in 2D, each component of E sees the permittivity averaged over its own cell
of the dual grid. The average is harmonic along the component and arithmetic across it, from
8 × 8 × 8 samples. That is exact for a layered medium, with the field along or across its
layers.

**Open boundaries** are stretched-coordinate PMLs (Chew and Weedon),
$\partial_w \to s_w^{-1}\partial_w$, graded as in 2D (Shin and Fan, Eqs. 2.5–2.9):

$$
s_w = 1 + i\,\frac{(m+1)(-\ln R)}{2 k_0 d}\left(\frac{l}{d}\right)^{m}.
$$

Each derivative uses the stretch where its result lives: at the faces for $\nabla\times\mathbf E$,
at the edges for $\nabla\times\tilde{\mathbf H}$. Behind each PML is a perfectly conducting wall:
the tangential E on the grid's outer faces is zero. Each axis can instead be **Bloch-periodic**:
one period on, the field is $e^{ikL}$ times itself. Shin and Fan chose this PML over the uniaxial
one because it keeps the system well conditioned (their Section 4), which the iterative solvers
that large 3D problems need depend on.

`Solver3d` factorizes the system once with faer's sparse LU. `Solver3d::solve` then gives the
field for any current by back-substitution, with one step of iterative refinement.
`Solver3d::reuse` keeps the analysis of the matrix's sparsity for a sweep.

## The power flux

`Field3d::flux` sums $\tfrac12 \operatorname{Re}(\mathbf E \times \tilde{\mathbf H}^*)\cdot\hat n$ over a
plane halfway between two planes of nodes. That is where the tangential H lives: the tangential
E is averaged across the plane, and each E pairs with the H that shares its place. This is the
scheme's own flux. Its difference between two planes is exactly the power the discrete
equations give to the layer of edges between them, so in a lossless region without sources,
between Bloch-periodic sides, it is the same through every plane to round-off.

## Validation

**A plane wave on a film.** 220 nm of silicon (3.476) on oxide (1.444), under air, at 1.55 µm.
The film is normal to z, and x and y are Bloch-periodic, one cell each. The wave comes from a
current sheet in the air, 30° from the normal in a plane 30° from x, so that kx, ky and every
component of E are in play. The sheet is the grid's own s or p wave: its discrete divergence in
the plane is zero for s, its curl for p. R is the flux of the scattered field (the field minus a
run without the film) over the incident flux. Both R and T are compared with the exact transfer
matrices of the [multilayer slab](multilayer.md). The grid is h in all three directions:

| | 20 nm | 10 nm | 5 nm | 2.5 nm |
|---|---|---|---|---|
| s (TE), 30°: error in R (R = 0.05817) | 2.8e-3 | 7.3e-4 | 1.9e-4 | 4.6e-5 |
| p (TM), 30°: error in R (R = 0.02687) | 1.9e-3 | 4.9e-4 | 1.2e-4 | 3.1e-5 |
| either, 0°: error in R (R = 0.03557) | 1.3e-3 | 3.6e-4 | 9.1e-5 | 2.3e-5 |

The error falls 4× per halving: second order. At 60° the PMLs' own reflection (below) is a
large part of the error, as in 2D, so 30° is the convergence test.

**Power.** Through every plane from the oxide through the silicon into the air, the flux is the
same to 2.2e-14 (10 nm grid, both polarizations, 0°, 30° and 60°). R + T is 1 to within what the
PMLs send back: 1.3e-6 at normal incidence, 1.8e-4 (s) and 2e-5 (p) at 60°, where a PML graded
for R = 1e-8 at normal incidence reflects about $R^{\cos 60°} = 10^{-4}$ in amplitude. A thicker,
stronger PML (40 cells, R = 1e-16) brings R + T to 1 within 2e-7 at every angle.

**The PML itself.** 20 cells graded to R = 1e-8 send back an amplitude of 2.5e-6 of a plane wave
17° off normal in oxide, in a plane 30° from x, for both polarizations. That is the 2D number
and the grading's prediction: the round trip keeps (1e-8)^1.38 of the power, an amplitude of
3e-6.

**Against the 2D solver.** A silicon rod (0.4 × 0.3 µm) in lossy oxide, uniform along z, in a
cell Bloch-periodic along x and y, on a 25 nm grid. The 2D solver's field and the 3D solver's
(one cell along z) differ by 9e-15 for E along z and 2e-13 for H along z, relative to the
largest field. The 3D grid is placed so that its E_z, or its H_z, sits at the 2D cells' centres.
The two discrete systems are then the same equations, one eliminating H and the other E.

## Cost

The sparse direct solver is exact, but in 3D its fill-in grows fast. Below is a silicon strip
(0.5 × 0.22 µm) in oxide on a 40 nm grid, PMLs of 6 cells all round, one current, on an Intel
Core Ultra 7 265K (20 threads, faer's default). The peak memory is the process's working set.

| Cells | Unknowns | Nonzeros | Analysis | Factorization | One source | Peak memory |
|---|---|---|---|---|---|---|
| 16³ | 12 288 | 134 k | 0.02 s | 0.42 s | 0.02 s | 0.36 GB |
| 24³ | 41 472 | 479 k | 0.09 s | 3.5 s | 0.11 s | 2.6 GB |
| 32³ | 98 304 | 1.17 M | 0.25 s | 19 s | 0.37 s | 10 GB |
| 40³ | 192 000 | 2.33 M | 0.55 s | 66 s | 0.89 s | 28 GB |

Time grows about as unknowns^1.9 and memory as unknowns^1.6. The 40³ grid is a cube 1.6 µm on a
side, so a device-sized problem is out of reach of a direct solver on one machine. Averaging the
permittivity (512 samples per component) costs 0.9 s at 40³, and assembly 0.07 s. faer orders
the matrix by COLAMD; a nested-dissection ordering would cut the fill.

## Limits

- No ports, mode sources or S-parameters in 3D yet, and no adjoint gradients: those are
  [2D](fdfd-ports.md) for now.
- The direct solver's memory caps a problem at about 200 k unknowns on a 64 GB machine. The
  iterative solver of Shin and Fan (2013) is the roadmap's answer.
- A uniform grid along each axis, and interfaces averaged by sampling: exact for interfaces
  along the axes, slower to converge for curved or slanted ones.

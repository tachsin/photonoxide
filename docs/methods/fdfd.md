---
title: "FDFD in 2D"
module: fdfd
summary: "Maxwell's equations at one frequency on Yee's grid, as one sparse linear system: either polarization, stretched-coordinate PMLs or Bloch-periodic sides, and the exact discrete power flux."
order: 15
papers:
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "W. Shin, S. Fan, J. Comput. Phys. 231, 3406 (2012)"
    doi: 10.1016/j.jcp.2012.01.013
  - cite: "W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994)"
    doi: 10.1002/mop.4650071304
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the exact reference)"
    doi: 10.1364/JOSAA.1.000742
  - cite: "A. George, SIAM J. Numer. Anal. 10, 345 (1973) (nested dissection)"
    doi: 10.1137/0710032
  - cite: "I. S. Duff, J. K. Reid, ACM Trans. Math. Softw. 9, 302 (1983) (the multifrontal method)"
    doi: 10.1145/356044.356047
  - cite: "X. S. Li, J. W. Demmel, ACM Trans. Math. Softw. 29, 110 (2003) (static pivoting)"
    doi: 10.1145/779359.779361
validation:
  - fdfd/slab-reflection-ez
  - fdfd/slab-reflection-hz
  - fdfd/flux-conservation
  - fdfd/pml-reflection
---

The finite-difference frequency-domain method solves Maxwell's equations at one frequency:
the structure, a source, and the field everywhere, from one sparse linear system. Where a mode
solver finds the light a waveguide guides, FDFD finds what a device does to light sent into it.

## The method

In 2D the structure is invariant along z and the fields vary in the x–y plane. They then split
into two polarizations (Yee's "Maxwell's equations in two dimensions"), each one equation for
the field along z. With $\tilde H = \eta_0 H$ and photonoxide's $e^{-i\omega t}$:

$$
\nabla^2 E_z + k_0^2 \varepsilon E_z = -i k_0 J, \qquad
\partial_x\negthinspace\left(\frac{1}{\varepsilon_y}\partial_x \tilde H_z\right) +
\partial_y\negthinspace\left(\frac{1}{\varepsilon_x}\partial_y \tilde H_z\right) + k_0^2 \tilde H_z = -i k_0 M,
$$

with J = η₀J_z the electric current and M the magnetic one. For layers normal to y, E along z
is TE and H along z is TM.

On Yee's staggered grid (his Fig. 1, in 2D) the field along z sits at the cells' centres and
the in-plane components on the faces between them. Every derivative is then a centred
difference: second order.

**Interfaces.** Each field component sees the permittivity averaged over its own cell of the
grid: harmonically along the component, arithmetically across it. That is what a layered medium
is for a field along or across its layers, so an interface anywhere in a cell keeps the scheme
second order. For E along z, the average is arithmetic over the whole cell.

**Open boundaries** are stretched-coordinate PMLs (Chew and Weedon), $\partial_w \to s_w^{-1}\partial_w$,
graded as Shin and Fan describe:

$$
s_w = 1 + i\thinspace\frac{(m+1)(-\ln R)}{2 k_0 d}\left(\frac{l}{d}\right)^{m},
$$

for a layer d thick, depth l into it, and a target reflection R at normal incidence (their
Eqs. 2.7–2.9, with σ/(ωε₀) in units of k₀; their $e^{+i\omega t}$ makes their s 1 − iσ/(ωε₀)). Shin and
Fan show that this PML, rather than the uniaxial one, keeps the linear system well conditioned:
it matters most for the iterative solvers 3D needs. An axis can instead be **Bloch-periodic**:
one period on, the field is $e^{ikL}$ times itself. That is how an infinite plane wave fits in a
grid a few cells wide.

The system is factorized once, by photonoxide's multifrontal factorization with static pivoting,
ordered by nested dissection on the grid (see [FDFD in 3D](fdfd-3d.md#cost)). The operator with
PMLs is similar to a complex symmetric matrix, $B = S A S^{-1}$, so $B$ is factorized as
$L D L^\mathsf{T}$; a Bloch side's phases break the similarity, and that system is factorized as
LU. `Solver2d::solve` then gives the field for any source by back-substitution, with one step of
iterative refinement.

**Cost.** The matrix's sparsity depends only on the grid and the boundaries, not on the
wavelength or the permittivity. `Solver2d::reuse` keeps its analysis (the matching and scaling,
the ordering and the symbolic factorization) for a sweep. Measured on a 440 × 340 grid (150 k
unknowns, Intel Core Ultra 7 265K), the analysis takes 0.18 s and $L D L^\mathsf{T}$ 0.27 s on one
thread, 0.11 s on 20 (faer's LU, before 0.4.3: 0.58 s and 0.85 s), and each further source about
80 ms (two solves with the factors, for the step of refinement).

## The power flux

`Field2d::flux_y` sums $\tfrac12 \operatorname{Re}(E \times \tilde H^*)_y$ across a row, with the in-plane
component and the field along z both taken on the face between two rows. This is the scheme's
own flux. It obeys a discrete Poynting theorem exactly, so in a lossless region without sources
it is the same through every row to round-off, interfaces included. Reflectance and
transmittance are ratios of these fluxes.

## Validation

**A plane wave on a slab.** 220 nm of silicon (3.476) on oxide (1.444), under air, at 1.55 µm,
with Bloch-periodic sides for the angle of incidence. The wave comes from a current sheet. R is
the flux of the scattered field (the field minus a run without the slab) over the incident
flux, and T is the flux in the oxide. Both are compared with the exact transfer matrices of the
[multilayer slab](multilayer.md):

| | 20 nm | 10 nm | 5 nm | 2.5 nm |
|---|---|---|---|---|
| E along z, 30°: error in R (R = 0.05817) | 2.1e-3 | 5.5e-4 | 1.4e-4 | 3.5e-5 |
| H along z, 30°: error in R (R = 0.02687) | 1.9e-3 | 4.9e-4 | 1.2e-4 | 3.1e-5 |

The error falls 4× per halving: second order, for both polarizations, and at 0° and 60° too.

**Power.** Through every row from the oxide through the silicon into the air, the flux is the
same to 1e-10, for both polarizations at 0°, 30° and 60°. R + T is 1 to within what the PMLs
send back: 1e-6 at normal incidence and 7e-5 at 60°, whatever the grid. A PML graded for R at
normal incidence reflects about $R^{\cos\theta}$. A thicker, stronger PML (40 cells, R = 1e-16)
brings R + T to 1 within 1e-6 at every angle.

**The PML itself.** 20 cells graded to R = 1e-8 send back an amplitude of 2.5e-6 of a plane wave
17° off normal in oxide, for both polarizations. That is the grading's prediction: the wave
crosses the layer 1.38 times as fast as in vacuum, so its round trip keeps (1e-8)^1.38 of the
power, an amplitude of 3e-6.

## Limits

- 2D here; 3D is [its own page](fdfd-3d.md), with the sparse direct solver for now.
- A uniform grid in each direction.
- The averaging is exact for interfaces along the grid's axes. A curved or slanted interface is
  sampled 8 × 8 times per cell, which converges more slowly. Farjadpour et al.'s subpixel
  smoothing, which follows the interface's normal, is the planned fix.
- Sources are currents on cells, or a waveguide's modes through its [ports](fdfd-ports.md).

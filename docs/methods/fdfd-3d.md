---
title: "FDFD in 3D"
module: fdfd
summary: "Maxwell's equations at one frequency on Yee's 3D grid: the electric field's curl-curl equation as one sparse system, with stretched-coordinate PMLs or Bloch-periodic sides on each axis, the exact discrete power flux, and a sparse direct or an iterative (QMR) solver."
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
  - cite: "W. Shin, S. Fan, Opt. Express 21, 22578 (2013)"
    doi: 10.1364/OE.21.022578
  - cite: "R. W. Freund, N. M. Nachtigal, Numer. Math. 60, 315 (1991)"
    doi: 10.1007/BF01385726
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the exact reference)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - fdfd3d/film-reflection-te
  - fdfd3d/film-reflection-tm
  - fdfd3d/flux-conservation
  - fdfd3d/pml-reflection
  - fdfd3d/two-d-agreement
  - fdfd3d/qmr-direct
  - fdfd3d/qmr-plateau
  - fdfd3d/qmr-iterations-curl-curl
  - fdfd3d/qmr-iterations-shin-fan
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
s_w = 1 + i\thinspace\frac{(m+1)(-\ln R)}{2 k_0 d}\left(\frac{l}{d}\right)^{m}.
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

## The iterative solver

`IterativeSolver3d` assembles the same system and never factorizes it, so its memory grows as
the unknowns. It solves by the quasi-minimal residual method (QMR) of Freund and Nachtigal,
which Shin and Fan use for their 3D problems:

- **Lanczos vectors.** Their Algorithm 3.1 builds on two sequences of unit vectors from the
  nonsymmetric Lanczos process, $v_n$ spanning $K_n(r_0, A)$ and $w_n$ spanning
  $K_n(w_1, A^{\mathsf T})$. These are biorthogonal under the unconjugated form $w^{\mathsf T}v$,
  with $w_1 = v_1 = b/\lVert b\rVert$ (their Algorithm 2.1, regular steps, Eq. 2.7).
- **The iterate.** It minimizes the norm of the residual's coordinates in that basis, with
  weights 1 (Eqs. 3.6–3.10), through Givens rotations of the tridiagonal (Eqs. 4.1–4.7) and short
  recurrences for the iterate (Eqs. 4.8–4.9).
- **Stopping.** The residual is updated at the cost of one vector sum per iteration (Eq. 4.12).
  Once it is below the tolerance, the true residual $\lVert b - Ax\rVert$ is computed and checked
  (Eq. 4.10). Freund and Nachtigal check the bound of Eq. 4.11 first. That bound is
  $\sqrt{n+1}$ times the quasi-residual, about 10× pessimistic after 100 iterations, and would
  add iterations.
- **Cost per iteration.** One product with A and one with $A^{\mathsf T}$. The rows are shared
  among threads, each summed in a fixed order, so the result is the same bit for bit on any
  number of threads.

Freund and Nachtigal's look-ahead steps (Algorithm 2.1's inner vectors, from their refs. 6–7)
step over a breakdown of the Lanczos process, $w_n^{\mathsf T}v_n = 0$. They are not
implemented: a breakdown or near-breakdown ($|w_n^{\mathsf T}v_n| \lt 10^{-14}$ for unit vectors)
is returned as an error. None occurred in the cases below.

**Shin and Fan's operator.** `Formulation::ShinFan` solves their Eq. 7 with s = −1 instead:

$$
-\nabla\times\nabla\times\mathbf E + k_0^2\varepsilon\mathbf E +
\nabla\negthinspace\left(\varepsilon^{-1}\nabla\cdot(\varepsilon\mathbf E)\right)
= -i k_0\mathbf J + \frac{1}{k_0^2}\thinspace\nabla\negthinspace\left(\varepsilon^{-1}\nabla\cdot(-i k_0\mathbf J)\right),
$$

our sign and units for their equation. The added terms vanish for the solution, by the
continuity equation $\nabla\cdot(\varepsilon\mathbf E) = \nabla\cdot\mathbf J/(i k_0)$. In a
uniform medium they turn $-\nabla\times\nabla\times$ into the vector Laplacian, which removes the
curl-curl operator's huge null space of near-zero eigenvalues (their Section 2).

On the grid, ∇· lives at the nodes and ∇ takes the nodes' values to the edges. So $\varepsilon^{-1}$
sits at the nodes, between the two, as in their Eq. 7: the mean of ε on the six edges a node's
divergence takes (the harmonic mean took up to 24% more iterations). Inside a PML
both use the stretch where their result lives, as the curls do. Then $\nabla\cdot\nabla\times = 0$
holds exactly on the grid, so the transformed system has exactly the curl-curl system's
solution: by the direct solver they agree to 7e-15 of the largest field (with PMLs, a Bloch
axis and silicon in oxide). Shin and Fan don't say how they discretize ∇∇· inside a PML; this
is our choice, the one that keeps the solution exact. Note that a relative residual of 1e-6 in
the transformed system is a weaker test than in the curl-curl one: its right-hand side carries
the large $\nabla\nabla\cdot\mathbf J$ of a point current. At 10 nm, 1e-6 in it is 1e-4 in the
curl-curl system.

**Against the direct solver.** For a silicon strip in oxide (16³ cells of 40 nm, PMLs all round),
QMR to a relative residual of 1e-10 gives the direct solver's field to 1.1e-11 on the curl-curl
operator and 1.3e-10 on Shin and Fan's (relative to the largest field).

**Shin and Fan's own test.** Their Fig. 1 is a vacuum square of 50 × 50 cells of 2 nm, periodic,
with an x-polarized dipole at the centre, at 1.55 µm. It is this solver with one cell along z.
The matrix is then real symmetric, and QMR is, in exact arithmetic, the GMRES their Fig. 3 uses
(their ref. 38):

| | s = 0 | s = −1 | s = +1 |
|---|---|---|---|
| ours: stagnates at | 0.709 (iterations 5–40) | no stagnation | no stagnation |
| theirs (text, Fig. 3) | 0.707 | | |
| ours: iterations to 1e-6 | 114 | 79 | 801 |
| theirs (read off Fig. 3, ±5) | 114 | 77 | not reached in 500; about 5e-5 at 400, ours 4.4e-5 |

**In 3D: faster by its own residual, not to the same field.** Shin and Fan's Fig. 9 shows
s = −1 converging 1.3–2.5× faster than s = 0 on three 3D problems, among them a silicon guide in
vacuum (their "Diel", 15 M unknowns). We measured QMR iterations to a relative residual of 1e-6
of each system: 40³ cells of 10 nm (0.4 µm across), 1.55 µm, an x-polarized current near the
centre, periodic sides or PMLs of 10 cells all round. The middle column is our first version,
with $\varepsilon^{-1}$ at the edge, outside the gradient: the same in a uniform medium, but
wrong at an interface.

| Structure | s = 0 | s = −1, ε⁻¹ at the edge (wrong) | s = −1 (Shin and Fan) |
|---|---|---|---|
| vacuum, periodic | 122 | 112 | 112 |
| vacuum, PMLs | 1231 | 559 | 559 |
| oxide cube 200 nm, periodic | 255 | 157 | 130 |
| silicon cube 200 nm, periodic | 416 | 367 | 186 |
| oxide guide 100 nm through the PMLs | 1457 | 802 | 724 |
| silicon cube 100 nm, PMLs | 1201 | 1920 | 903 |
| silicon guide 100 nm through the PMLs | 1976 | 3076 | 1629 |

With ε⁻¹ where Eq. 7 puts it, s = −1 needs fewer iterations in every case, 1.2–2.2× fewer, which
is Shin and Fan's result. But each count is to each system's own residual, and the two systems'
residuals don't measure the same thing (see above). Measured instead by the field's error against
a reference (s = −1 to 1e-11; in brackets, how far it is from s = 0 to 1e-8):

| Structure (reference agreement) | tolerance | s = 0: iterations, field error | s = −1: iterations, field error |
|---|---|---|---|
| vacuum, PMLs (3.4e-10) | 1e-5 | 967, 1.1e-7 | 361, 1.9e-4 |
| | 1e-6 | 1231, 8.9e-9 | 559, 2.4e-5 |
| | 1e-7 | 1397, 1.5e-9 | 814, 1.5e-6 |
| | 1e-8 | 1630, 3.4e-10 | 1031, 1.1e-7 |
| silicon cube 100 nm, PMLs (1.1e-9) | 1e-5 | 964, 1.4e-6 | 685, 2.2e-4 |
| | 1e-6 | 1201, 1.4e-7 | 903, 2.4e-5 |
| | 1e-7 | 1398, 1.3e-8 | 1222, 1.5e-6 |
| | 1e-8 | 1676, 1.1e-9 | 1411, 1.9e-7 |
| silicon guide 100 nm, PMLs (5.1e-9) | 1e-5 | 1744, 1.5e-6 | 1233, 2.2e-4 |
| | 1e-6 | 1976, 3.8e-7 | 1629, 1.2e-5 |
| | 1e-7 | 2303, 4.7e-8 | 2098, 1.7e-6 |
| | 1e-8 | 2739, 5.1e-9 | 2394, 2.3e-7 |

At the same field error, the curl-curl operator is as fast or faster: in vacuum, s = −1 reaches
1.1e-7 in 1031 iterations where s = 0 does in 967; on the silicon guide, 2.3e-7 takes s = −1
2394 and s = 0 about 2000. The transformation trades the curl-curl operator's null space for a
right-hand side dominated by $\nabla\nabla\cdot\mathbf J$, and the residual it then converges
mostly measures that part.

Their Diel at a smaller size (a 400 × 300 nm silicon guide in vacuum along x, 40 × 90 × 80 cells
of 10 nm with PMLs of 10, a current across the guide; 864 000 unknowns) takes 2 745 iterations
with s = 0 and 4 703 with s = −1 (14 524 with ε⁻¹ at the edge), each to 1e-6 of its own residual.
Measured in the curl-curl system, s = −1 ends at a residual of 9.1e-6, and the two solutions agree
to 7.2e-6. Here, unlike their Fig. 9, s = −1 is slower even by its own residual; their guide is
37 times larger, with a dipole source we don't reproduce exactly.

So `Formulation::CurlCurl` stays the default: by the field, the measure that matters, it is never
slower here. `Formulation::ShinFan` is Shin and Fan's operator as published, for comparison and
for the preconditioners of a later milestone, where the null space matters more.

**Cost.** `IterativeSolver3d` keeps the matrix and its transpose (13 or 15 nonzeros per row) and
a dozen vectors. The 864 000-unknown guide above peaked at 1.4 GB, the assembly's temporaries
included, where the direct solver needs 28 GB for 192 k unknowns. The time is the iteration
count times the cost of an iteration, about 45 ns per unknown on 20 threads here: 106 s for the
guide with s = 0 (2 745 iterations), against the direct solver's 66 s for a problem 4.5 times
smaller.

## Limits

- No ports, mode sources or S-parameters in 3D yet, and no adjoint gradients: those are
  [2D](fdfd-ports.md) for now.
- The direct solver's memory caps a problem at about 200 k unknowns on a 64 GB machine. QMR's
  doesn't, but it needs thousands of iterations, and nothing preconditions it yet.
- QMR without look-ahead: a breakdown of the Lanczos process is an error, not stepped over.
- A uniform grid along each axis, and interfaces averaged by sampling: exact for interfaces
  along the axes, slower to converge for curved or slanted ones.

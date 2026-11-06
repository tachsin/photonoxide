---
title: "FDFD and adjoint gradients"
summary: "Maxwell's equations at one frequency as one sparse linear system: the grid, its PMLs and sources, ports and S-parameters, direct and iterative solves and what they cost, and the adjoint method, which gives a figure of merit's gradient with respect to every cell for one more solve, derived, checked against finite differences, and used by optimizers on devices and circuits."
topic: Numerical methods
level: advanced
minutes: 60
prerequisites: [how-fdtd-works]
examples: [optimize_ybranch, circuit_splitter, circuit_ring_critical, circuit_fit]
jobs: [ring-fdfd.yaml, ring-fdtd.yaml, mmi-fdfd.yaml, mmi-fdtd.yaml]
methods: [fdfd.md, fdfd-3d.md, fdfd-ports.md, backends.md, block-qmr.md, recycling.md, multigrid-stencil.md, fdfd-adjoint.md, fdtd-adjoint.md, circuit-adjoint.md]
validation:
  - fdfd/slab-reflection-ez
  - fdfd/slab-reflection-hz
  - fdfd3d/two-d-agreement
  - fdfd/pml-reflection
  - fdfd/flux-conservation
  - fdfd/port-mode-te
  - fdfd/straight-guide
  - fdfd/reciprocity
  - fdfd/step-reflection-te
  - fdfd/fill-slab-te-mean
  - fdfd/fill-slab-te-point
  - fdfd/fill-slab-tm-diagonal
  - fdfd3d/ldlt-lu
  - fdfd3d/qmr-direct
  - fdfd3d/gmres-multigrid-direct
  - fdfd3d/block-qmr-s-matrix
  - fdfd3d/recycle-s-matrix
  - fdfd/adjoint-gradient-ez
  - fdfd/adjoint-gradient-hz
  - fdfd3d/adjoint-gradient
  - fdfd3d/recycle-adjoint
  - fdtd/adjoint-fdfd
  - fdtd/adjoint-backward-mode
  - fdfd/fill-derivative-ez
  - circuit/adjoint-mzi
  - circuit/adjoint-mesh
  - circuit/adjoint-differences
charts: [fdfd-convergence, fdfd-gradient, fdfd-difference-step]
papers:
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    title: "Numerical solution of initial boundary value problems involving Maxwell's equations in isotropic media"
    doi: 10.1109/TAP.1966.1138693
    year: 1966
    role: origin
    note: "The staggered grid: each component of E and H at its own place in the cell, so every derivative in the curl equations is a centred difference. FDFD solves on the same grid at one frequency what FDTD steps in time."
  - cite: "A. George, SIAM J. Numer. Anal. 10, 345 (1973)"
    title: "Nested Dissection of a Regular Finite Element Mesh"
    doi: 10.1137/0710032
    year: 1973
    role: milestone
    note: "Numbering an n × n grid's unknowns by recursive dissection lets a sparse factorization take O(n³) operations and O(n² log n) storage, against O(n⁴) and O(n³) for a banded numbering; no numbering does better than O(n³). The order photonoxide's direct solver uses."
  - cite: "A. Christ, H. L. Hartnagel, IEEE Trans. Microw. Theory Tech. 35, 688 (1987)"
    title: "Three-Dimensional Finite-Difference Method for the Analysis of Microwave-Device Embedding"
    doi: 10.1109/TMTT.1987.1133733
    year: 1987
    role: origin
    note: "Maxwell's equations at one frequency on Yee's 3D cell, H eliminated, as one sparse system for E alone, three unknowns per cell, solved iteratively, the scattering matrix from the mode amplitudes on two neighbouring planes."
  - cite: "R. W. Freund, N. M. Nachtigal, Numer. Math. 60, 315 (1991)"
    title: "QMR: a quasi-minimal residual method for non-Hermitian linear systems"
    doi: 10.1007/BF01385726
    year: 1991
    role: milestone
    note: "A Krylov method for non-Hermitian systems built on the nonsymmetric Lanczos process, its iterate minimizing a quasi-residual with short recurrences: the iterative solver photonoxide's 3D FDFD runs."
  - cite: "W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994)"
    title: "A 3D perfectly matched medium from modified Maxwell's equations with stretched coordinates"
    doi: 10.1002/mop.4650071304
    year: 1994
    role: origin
    note: "Maxwell's equations with each Cartesian coordinate stretched by a complex factor: an absorbing layer with zero reflection at all angles of incidence in the continuum, the PML photonoxide's FDFD uses."
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004)"
    title: "Method for sensitivity analysis of photonic crystal devices"
    doi: 10.1364/OL.29.002288
    year: 2004
    role: milestone
    note: "The adjoint variable method on the FDFD system: the sensitivity to any number of parameters from one LU decomposition and two back-substitutions, the second typically at least ten times cheaper than the decomposition (Eqs. 2–4), checked against finite differences."
  - cite: "F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidisc. Optim. 43, 767 (2011; online 2010)"
    title: "On projection methods, convergence and robust formulations in topology optimization"
    doi: 10.1007/s00158-010-0602-y
    year: 2010
    role: milestone
    note: "Densities filtered by a weighted average over a neighbourhood (Eq. 4), projected towards 0 or 1 by a tanh threshold of sharpness β (Eq. 9), the gradient through both by the chain rule (Eq. 7); and the robust formulation over eroded, intermediate and dilated designs."
  - cite: "W. Shin, S. Fan, J. Comput. Phys. 231, 3406 (2012)"
    title: "Choice of the perfectly matched layer boundary condition for frequency-domain Maxwell's equations solvers"
    doi: 10.1016/j.jcp.2012.01.013
    year: 2012
    role: milestone
    note: "The stretched-coordinate PML gives frequency-domain matrices far better conditioned than the uniaxial PML, and iterative solvers converge far faster on them; its polynomial grading for a target reflection at normal incidence (Eqs. 2.7–2.9)."
  - cite: "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012)"
    title: "Simple implementation of arbitrarily shaped total-field/scattered-field regions in finite-difference frequency-domain"
    doi: 10.2528/PIERB11092006
    year: 2012
    role: milestone
    note: "A one-way source for FDFD from a mask Q of the scattered-field cells and the system's own matrix, b = (QA − AQ)f (Eq. 55), f the source's field over the whole grid without the device."
  - cite: "Y. Zhang, S. Yang, A. E.-J. Lim, G.-Q. Lo, C. Galland, T. Baehr-Jones, M. Hochberg, Opt. Express 21, 1310 (2013)"
    title: "A compact and low loss Y-junction for submicron silicon waveguide"
    doi: 10.1364/OE.21.001310
    year: 2013
    role: milestone
    note: "A Y-junction's taper of 13 widths optimized by particle swarm in 2D FDTD, a swarm of 30 for 50 iterations; 0.13 dB of insertion loss in 3D FDTD, 0.28 dB measured on average across a wafer made by 248 nm lithography."
  - cite: "C. M. Lalau-Keraly, S. Bhargava, O. D. Miller, E. Yablonovitch, Opt. Express 21, 21693 (2013)"
    title: "Adjoint shape optimization applied to electromagnetic design"
    doi: 10.1364/OE.21.021693
    year: 2013
    role: milestone
    note: "The adjoint gradient from reciprocity: the field of a source at the monitor, times the forward field, at every point (Eq. 5); for a mode's power, the mode sent backwards from the monitor (Eq. 8). Zhang et al.'s Y-junction reshaped by level sets in 51 iterations, 102 simulations, to 0.07 dB."
  - cite: "A. Y. Piggott, J. Lu, K. G. Lagoudakis, J. Petykiewicz, T. M. Babinec, J. Vučković, Nat. Photonics 9, 374 (2015)"
    title: "Inverse design and demonstration of a compact and broadband on-chip wavelength demultiplexer"
    doi: 10.1038/nphoton.2015.69
    year: 2015
    role: milestone
    note: "A 2.8 × 2.8 µm² silicon splitter of 1,300 and 1,550 nm light designed by adjoint gradients on FDFD in three stages (continuous permittivity, level sets, ten wavelengths), made and measured: peak insertion losses of 1.8 and 2.4 dB, crosstalk below −11 dB."
  - cite: "S. Molesky, Z. Lin, A. Y. Piggott, W. Jin, J. Vučković, A. W. Rodriguez, Nat. Photonics 12, 659 (2018)"
    title: "Inverse design in nanophotonics"
    doi: 10.1038/s41566-018-0246-9
    year: 2018
    role: review
    note: "The review: density and level-set topology optimization, the adjoint method (Box 1), fabrication constraints, and what inverse design has made, from cavities and waveguides to demultiplexers."
  - cite: "T. W. Hughes, M. Minkov, I. A. D. Williamson, S. Fan, ACS Photonics 5, 4781 (2018)"
    title: "Adjoint Method and Inverse Design for Nonlinear Nanophotonic Devices"
    doi: 10.1021/acsphotonics.8b01522
    year: 2018
    role: current
    note: "The adjoint method for FDFD generalized to nonlinear problems in the frequency domain, solved once whatever the number of parameters; used to design compact Kerr switches that route low- and high-power light to different ports."
  - cite: "T. W. Hughes, M. Minkov, Y. Shi, S. Fan, Optica 5, 864 (2018)"
    title: "Training of photonic neural networks through in situ backpropagation and gradient measurement"
    doi: 10.1364/OPTICA.5.000864
    year: 2018
    role: current
    note: "The adjoint variable method applied to a mesh of interferometers is backpropagation, and its gradient can be measured on the chip itself from the forward and adjoint fields' intensities."
  - cite: "R. E. Christiansen, O. Sigmund, J. Opt. Soc. Am. B 38, 496 (2021)"
    title: "Inverse design in photonics by topology optimization: tutorial"
    doi: 10.1364/JOSAB.406048
    year: 2021
    role: review
    note: "Density topology optimization step by step: the filter, the tanh projection raised by continuation, eroded and dilated designs for robustness; and the caution that an optimizer can at most guarantee a local optimum, its design depending on where it starts."
  - cite: "A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, Opt. Express 29, 23916 (2021)"
    title: "Photonic topology optimization with semiconductor-foundry design-rule constraints"
    doi: 10.1364/OE.431188
    year: 2021
    role: current
    note: "Minimum linewidth, spacing, curvature, area and enclosed area imposed inside density topology optimization, so a design passes a foundry's rule checks: three broadband silicon devices under nine foundries' rule sets."
---

A frequency-domain problem as photonoxide's FDFD sets it up, in the symbols of this lesson: a grid
closed by PMLs, a guide through it, a source $b$ that launches the guide's mode and a monitor that
measures the mode's amplitude $a$; between them a design region of cells $\varepsilon_k$. The
forward field $x$ travels from the source, the adjoint field $\lambda$ back from the monitor, and
their product in each cell is the gradient. Beside it, Yee's grid for $E$ along $z$.

::diagram fdfd

The finite-difference frequency-domain method (FDFD) solves Maxwell's equations at one frequency
as one sparse linear system $A(\varepsilon)\,x = b$: the structure in $A$, the source in $b$, the
field everywhere in $x$. Its answers are steady states, so a device's S-parameters at a wavelength
come from one solve per port, and the same matrix, transposed, gives the derivative of any figure
of merit with respect to every cell's permittivity for one more solve: the adjoint method, the
engine of inverse design.

## 1. What does FDFD solve?

At one angular frequency, with fields varying as $e^{-i\omega t}$ (photonoxide's convention) and
$\tilde H = \eta_0 H$, Maxwell's curl equations are

$$
\nabla\times\mathbf E = ik_0\tilde{\mathbf H}, \qquad \nabla\times\tilde{\mathbf H} = -ik_0\varepsilon\mathbf E + \mathbf J,
$$

$k_0 = \omega/c$ and $\mathbf J$ the current times $\eta_0$. Taking the curl of the first and
substituting the second leaves one equation for $\mathbf E$:

$$
-\nabla\times\nabla\times\mathbf E + k_0^2\varepsilon\mathbf E = -ik_0\mathbf J .
$$

On [Yee (1966)](https://doi.org/10.1109/TAP.1966.1138693)'s staggered grid every derivative is a
centred difference, the unknowns are the values of $\mathbf E$, one equation for each: a sparse
linear system $A(\varepsilon)\,x = b$ with $b = -ik_0 J$. [Christ and Hartnagel (1987)](https://doi.org/10.1109/TMTT.1987.1133733)
set up the 3D system this way, $\mathbf H$ eliminated, three unknowns per cell, the matrix "sparse
but not symmetric", solved iteratively by biconjugate gradients (pp. 690–691). In photonoxide's 3D
FDFD each value of $\mathbf E$ is tied to itself and 12 neighbours: 13 nonzeros a row
([FDFD in 3D](../docs/methods/fdfd-3d.md)). Where [FDTD](how-fdtd-works.md) steps the fields in
time until a pulse has passed, FDFD solves for the steady state at one frequency at once.

### The 2D equation on Yee's grid {theory}

A structure uniform along $z$ splits the fields into two polarizations. With
$\mathbf E = E_z\hat z$, the first curl equation gives $\tilde H_x = \partial_y E_z/ik_0$ and
$\tilde H_y = -\partial_x E_z/ik_0$; the $z$ component of the second,
$\partial_x\tilde H_y - \partial_y\tilde H_x = -ik_0\varepsilon E_z + J$, becomes
$-\nabla^2E_z/ik_0 = -ik_0\varepsilon E_z + J$, and multiplied by $ik_0$:

$$
\nabla^2 E_z + k_0^2\varepsilon E_z = -ik_0 J .
$$

Yee's grid in 2D puts $E_z$ at the cells' centres and $\tilde H_x$, $\tilde H_y$ on their faces
(the diagram). Each second derivative is then a difference of two first differences, and on a
grid of steps $\Delta x$, $\Delta y$ the equation at cell $(i, j)$ is

$$
\frac{E_{i+1,j} - 2E_{i,j} + E_{i-1,j}}{\Delta x^2} + \frac{E_{i,j+1} - 2E_{i,j} + E_{i,j-1}}{\Delta y^2} + k_0^2\varepsilon_{i,j}E_{i,j} = -ik_0J_{i,j} ,
$$

five unknowns in each row of $A$. Taylor's series gives
$(E_{i+1} - 2E_i + E_{i-1})/\Delta^2 = \partial^2E + \tfrac{1}{12}\Delta^2\partial^4E + O(\Delta^4)$:
the scheme is second order wherever the field is smooth. With $\mathbf H = \tilde H_z\hat z$ the
same steps give $\partial_x(\varepsilon_y^{-1}\partial_x\tilde H_z) + \partial_y(\varepsilon_x^{-1}\partial_y\tilde H_z) + k_0^2\tilde H_z = -ik_0M$,
where $\varepsilon$ enters through the faces ([FDFD in 2D](../docs/methods/fdfd.md)). For layers
normal to $y$, $E$ along $z$ is TE and $H$ along $z$ is TM.

The size of the system is the grid's: the gradient chart below solves 2.8 × 3.8 µm in 20 nm cells,
140 × 190 = 26,600 unknowns and about 133,000 nonzeros. The reflectance of 220 nm of silicon on
oxide at 30°, against the exact transfer matrices, has errors of 2.1 × 10⁻³, 5.5 × 10⁻⁴,
1.4 × 10⁻⁴ and 3.5 × 10⁻⁵ on grids of 20, 10, 5 and 2.5 nm: four times smaller per halving,
second order, for both polarizations. The 3D solver on a structure uniform along $z$ gives the 2D
solver's field to round-off:

::validation fdfd/slab-reflection-ez fdfd/slab-reflection-hz fdfd3d/two-d-agreement

## 2. How is the grid closed, and where does the light come from?

A device radiates into open space, and a grid must end. Photonoxide ends it in **perfectly
matched layers**: [Chew and Weedon (1994)](https://doi.org/10.1002/mop.4650071304) stretch each
coordinate by a complex factor, $\partial_w \to s_w^{-1}\partial_w$, which in the continuum gives
an absorbing layer "with zero reflection at all angles of incidence" (p. 599). Inside a layer $d$
thick the stretch grows from 1 at its inner face as Shin and Fan grade it (their Eqs. 2.7–2.9,
p. 3408, in photonoxide's $e^{-i\omega t}$):

$$
s_w = 1 + i\thinspace\frac{(m+1)(-\ln R)}{2k_0d}\left(\frac{l}{d}\right)^m ,
$$

$l$ the depth into it, $m$ the grading's order (3 here) and $R$ the target reflection at normal
incidence. [Shin and Fan (2012)](https://doi.org/10.1016/j.jcp.2012.01.013) showed that this
stretched-coordinate PML makes the frequency-domain matrix far better conditioned than the
uniaxial PML, so iterative solvers converge "significantly faster" on it (p. 3406), which 3D
problems depend on (Section 5). Behind each PML the field is zero.

An axis can instead be **Bloch-periodic**: one period on, the field is $e^{ikL}$ times itself. A
plane wave at an angle then fits in a grid one period wide, which is how the slab's reflectance
in Section 1 is computed at 30°.

The light comes from **sources**: a current $J$ on cells, or a guide's mode launched one way by a
total-field/scattered-field boundary. [Rumpf (2012)](https://doi.org/10.2528/PIERB11092006)
writes it with the system's own matrix,

$$
b = (QA - AQ)\thinspace f ,
$$

$Q$ the mask of the scattered-field cells and $f$ the mode across the whole grid without the
device (Eq. 55, p. 234): only the rows at the boundary between the two regions are nonzero, and
the mode leaves it in one direction only ([FDFD ports](../docs/methods/fdfd-ports.md)).

### Why the stretch absorbs, and what the grid adds {theory}

A wave $e^{ik_wx_w}$ in stretched coordinates becomes $e^{ik_w\tilde x_w}$ with
$\tilde x_w = \int s_w\thinspace dx_w$: its phase is unchanged and its amplitude falls as
$e^{-k_w\int\operatorname{Im}s_w}$, whatever the angle, so the layer's face reflects nothing. With
the grading above, $\int_0^d\operatorname{Im}s_w\thinspace dl = -\ln R/2k_0$, and the round trip
through the layer and back returns $R^{k_w/k_0}$ of the amplitude: $R$ at normal incidence in
vacuum, and $R^{1.38}$, about 10⁻¹¹ for $R = 10^{-8}$, for a wave 17° off the normal in oxide,
$k_w/k_0 = 1.444\cos 17^\circ = 1.38$. On the grid the layer sends back more, and how much depends on
its cells: 20 cells of 20 nm graded to $R = 10^{-8}$ return 2.5 × 10⁻⁶ of that wave's amplitude, 10
cells of 10 nm 4.0 × 10⁻⁵ and 8 cells of 10 nm 9.8 × 10⁻⁵ ([FDFD in 3D](../docs/methods/fdfd-3d.md),
"Preconditioning QMR"). The continuum's round trip is the same for all three; the difference is
the grading's steps on the grid, which a thicker layer, graded more gently per cell, makes
smaller. The power through every row of a lossless region is the same to 10⁻¹⁰, because the
scheme's own flux obeys a discrete Poynting theorem exactly:

::validation fdfd/pml-reflection fdfd/flux-conservation

## 3. How are ports and S-parameters measured?

A device is described by what it does to the modes of its guides: its **S-matrix**, $S_{qp}$ the
amplitude that comes out in mode $q$ for a unit amplitude sent in in mode $p$, $|S_{qp}|^2$ the
share of power. A port in photonoxide's 2D FDFD is a column of the grid across a guide, with the
guide's modes ([FDFD ports](../docs/methods/fdfd-ports.md)):

- **The port's mode is the grid's own.** The 2D operator restricted to the column is a 1D
  eigenproblem, whose eigenvalue is the discrete wavenumber
  $\beta_d = (2/\Delta x)\sin(\beta\Delta x/2)$. A mode solved this way travels along a straight
  grid guide exactly: two ports on a straight slab give $S_{11} = 0$ and $S_{21} = e^{i\beta L}$
  to 10⁻¹³. A mode from a continuum solver would leave a small mismatch on every grid.
- **A launch** is the mode by total-field/scattered-field (Section 2), at unit amplitude on its
  column.
- **A measurement** projects the field on two neighbouring columns onto the mode, with the
  weights that make the column's operator symmetric, and separates the forward and backward
  waves (below).
- **The S-matrix** takes one solve per port. With a factorized $A$, each port after the first
  costs a back-substitution.

Against the exact slab, the port mode of 220 nm of silicon in oxide has an effective index
2.84794 on a 2.5 nm grid, exact 2.84778, its error falling 4 times per halving of the grid. A slab
stepping from 220 to 300 nm gives $S_{21} = S_{12}$ to 10⁻¹⁴, as a reciprocal device must.

::validation fdfd/port-mode-te fdfd/straight-guide fdfd/reciprocity fdfd/step-reflection-te

The step's reflection, 1.16150 × 10⁻³ on the 2.5 nm grid, differs from Fresnel's formula on the
two modes' effective indices, 1.16738 × 10⁻³, by 0.5 %: the formula treats each mode as a plane
wave of its effective index, an estimate, and the difference converges at second order to that
0.5 %, not to zero.

### Forward and backward from two columns {theory}

On column $c$ a mode's projection is $p_c = a + b$, the forward and backward amplitudes; one cell
on it is $p_{c+1} = ae^{i\beta\Delta x} + be^{-i\beta\Delta x}$. Eliminating $b$,

$$
a = \frac{p_{c+1} - p_c e^{-i\beta\Delta x}}{e^{i\beta\Delta x} - e^{-i\beta\Delta x}}, \qquad
b = \frac{p_c e^{i\beta\Delta x} - p_{c+1}}{e^{i\beta\Delta x} - e^{-i\beta\Delta x}} .
$$

Both are linear in the field: $a = c^\mathsf{T}x$ for a vector $c$ that is nonzero on the two
columns only, the form Section 8 differentiates. Photonoxide normalizes each mode by its
unconjugated Lorentz form, which keeps $S$ exactly symmetric for a reciprocal device, and measures
the incoming amplitude at every port too, so a trace of a mode the PMLs send back is counted as
incoming rather than as part of $S$ ([FDFD ports](../docs/methods/fdfd-ports.md)).

### Where ports came from {research}

Christ and Hartnagel computed a microwave embedding's scattering matrix this way in 3D: the
electric field from the sparse solve, then "the mode amplitudes" on two neighbouring planes one
grid step apart (p. 690). Photonoxide's 3D ports take the grid's own full-vector modes on a plane
the same way ([FDFD in 3D](../docs/methods/fdfd-3d.md)).

## 4. How accurate is FDFD, and how fast does it converge?

In a uniform region the scheme is second order (Section 1). At an interface that cuts a cell,
the cell needs one permittivity for two materials, and how it is chosen sets the order. The chart
solves the port mode of a silicon slab on grids whose lines miss its faces by 0.3 and 0.7 of a
cell, three ways:

::chart fdfd-convergence

With $E$ along $z$ (TE), 220 nm thick, on 11 grids from 50 to 1.7 nm, each order measured
between the grids of 3.4 and 1.7 nm:

- **Staircasing**, each cell its centre's material, moves each face by a share of a cell: order
  1.03, first order, an error of 3.5 × 10⁻³ in $n_\text{eff}$ on the finest grid.
- **The mean over the cell** is exact for a field along the interface: order 2.00, second order,
  an error of 3.6 × 10⁻⁵ on the finest grid.
- **The solver's default**, 8 × 8 samples a cell, places each face to a sixteenth of a cell: order
  1.09, first order with a small constant, 6.1 × 10⁻⁴ on the finest grid, below staircasing's on
  every grid.

The exact index, 2.84778, is the slab's transfer-matrix mode, with no grid.

### Across the interface {theory}

With $H$ along $z$ (TM) the slab's $E_y$ crosses the interface. Its normal $D_y = \varepsilon E_y$ is
continuous there and $E_y$ jumps, so the average that is exact across a layer is the harmonic mean,
$\langle\varepsilon^{-1}\rangle^{-1}$, and the plain mean is the wrong one: set the chart's field
to 2 and all three curves fall at first order, 1.01 for the samples and for staircasing and 1.06
for the mean. Photonoxide's averaging from the fill takes the harmonic mean across the interface
and the mean along it, and keeps the second order: 1.88 for this TM slab
([averaging from the fill](../docs/methods/averaging.md)). The cases:

::validation fdfd/fill-slab-te-mean fdfd/fill-slab-te-point fdfd/fill-slab-tm-diagonal

**Try it.** Set the slab to 300 nm. Which orders change?

:::answer
The mean's stays 2.00 and staircasing's moves from 1.03 to 1.06: one is second order and the other
first, whatever the thickness. The samples' order moves from 1.09 to 1.24. Their error has a first-
order part with a small constant and a second-order part, and on these grids neither dominates, so
the order fitted between two grids depends on how large each part is for the slab at hand; on
grids fine enough, the first-order part wins and the order tends to 1.
:::

## 5. How is the system solved, and what does it cost?

Two families of solver, with different costs:

- **Direct**: factorize $A = LDL^\mathsf{T}$ or $LU$ once; then each right-hand side costs two
  triangular solves. Exact to round-off, but the factors fill in: their size, not $A$'s, sets the
  memory.
- **Iterative**: never factorize; build the answer from products of $A$ with vectors until the
  residual $\lVert b - Ax\rVert/\lVert b\rVert$ is below a tolerance. Memory grows as the unknowns;
  the iterations depend on the matrix's conditioning and on the preconditioner.

How much a factorization fills depends on the order the unknowns are eliminated in.
[George (1973)](https://doi.org/10.1137/0710032) showed that numbering an $n \times n$ grid by
nested dissection, each part numbered before the line that separates it from its neighbours,
recursively, lets the factorization take $O(n^3)$ operations and $O(n^2\log n)$ storage, against
$O(n^4)$ and $O(n^3)$ for the usual row-by-row numbering, and that no numbering takes fewer than
$O(n^3)$ operations (p. 345). With $N = n^2$ unknowns: $O(N^{3/2})$ time and $O(N\log N)$ memory
in 2D. Photonoxide's direct solver is its own multifrontal factorization on that ordering
([FDFD in 3D](../docs/methods/fdfd-3d.md), "Cost").

Measured on one machine (an Intel Core Ultra 7 265K), as the method pages report them, not as a
promise for another:

| Problem | Unknowns | Solver | Time: the factorization, or the iterative solve | Memory |
|---|---|---|---|---|
| 2D, 440 × 340 cells | 150,000 | $LDL^\mathsf{T}$, 1 thread | analysis 0.18 s, factorization 0.27 s, each source about 80 ms | |
| 3D strip, 24³ cells of 40 nm | 41,472 | $LDL^\mathsf{T}$, 1 thread | 0.85 s | 11.0 M factor entries, 0.84 GB |
| 3D strip, 32³ | 98,304 | the same | 4.6 s | 37.9 M, 2.0 GB |
| 3D strip, 40³ | 192,000 | the same | 17.6 s (3.2 s on 20 threads) | 97.4 M, 3.8 GB |
| 3D guide, 40³ cells of 10 nm | 192,000 | QMR + ILU(0), 20 threads | 195 iterations, 5.6 s | 460 MB |
| | | GMRES + multigrid | 16 iterations, 0.5 s | 1.07 GB |
| 3D strip with ports, 72 × 102 × 82 cells of 20 nm, two runs | 1.8 million | QMR + ILU(0) | 1,965 iterations, 369 s | 4.27 GB |
| | | GMRES + multigrid | 54 iterations, 10.7 s | 9.48 GB |

In 3D the direct solver's time grows about as $N^2$ and its factors as $N^{1.4}$: the 40³ grid is a
cube 1.6 µm on a side, so a device-sized 3D problem is out of a direct solver's reach on one
machine, and the iterative solvers take over. The iterative fields agree with the direct solver's
to the tolerance they are solved to:

::validation fdfd3d/ldlt-lu fdfd3d/qmr-direct fdfd3d/gmres-multigrid-direct

### The iterative solvers {theory}

[Freund and Nachtigal (1991)](https://doi.org/10.1007/BF01385726)'s quasi-minimal residual method
(QMR) solves a non-Hermitian system with short recurrences from the nonsymmetric Lanczos process:
one product with $A$ and one with $A^\mathsf{T}$ an iteration. The curl-curl operator with PMLs is
similar to a complex symmetric matrix, $B = SAS^{-1}$ with $S$ diagonal, and QMR for complex
symmetric matrices needs only the product with $B$: on the 40³ guide, 2,077 iterations in 2.83 s
against general QMR's 2,059 in 5.69 s, to the same residual. A preconditioner changes the counts by
orders of magnitude: an incomplete factorization, ILU(0), keeps $A$'s sparsity; a multigrid cycle
solves the error on coarser grids, and needs the PMLs stretched in their real part too
([FDFD in 3D](../docs/methods/fdfd-3d.md)). The multigrid without its finest matrix brings the
1.8-million-unknown strip's memory from 9.5 GB to 6.0 or 3.3 GB
([the multigrid without its finest matrix](../docs/methods/multigrid-stencil.md)).

### The backends {theory}

Every direct solve in photonoxide goes through one interface ([backends](../docs/methods/backends.md)):
`photonoxide`, the library's own multifrontal $LU$ and $LDL^\mathsf{T}$, the default and the
reference; `faer`'s sparse $LU$, the baseline; and any other library registered at run time, each
declaring what it can do and checked against photonoxide's own. `auto` chooses from the benchmark
records of the machine it runs on: a backend at least 10 % faster than photonoxide's own at the
nearest measured size, within a factor of 4 of the problem's, whose memory fits. Without records,
as in the library's tests and its validation report, it is photonoxide's own. A job can name its
backend, and the run's record says which solved it and why.

## 6. How can many solves share the work?

One structure is solved many times: once per port for an S-matrix, again for an adjoint
(Section 8), at every wavelength of a sweep, and at every step of an optimization.

- **A direct solver** keeps what doesn't change. The matrix's sparsity depends only on the grid
  and the boundaries, so a sweep keeps the ordering and the symbolic factorization and redoes only
  the numbers (`Solver2d::reuse`); one factorization serves every port and the adjoint, each a
  back-substitution.
- **Block QMR** solves all ports at once in one block Krylov space
  ([block QMR](../docs/methods/block-qmr.md)). With ILU(0), on a coupler's 4 or 8 ports at 40 nm
  (149,184 unknowns), it takes 0.69 of the time of one solve per port; without a preconditioner it
  is 2.8 to 5.3 times slower, and with the multigrid 2.2 times. It is opt-in.
- **Recycling** starts each solve from the combination of the earlier solutions with the least
  residual ([recycling](../docs/methods/recycling.md)). A coupler's S-matrix at 16 wavelengths,
  1.500 to 1.575 µm on 50 nm cells (96,768 unknowns), by GMRES and the multigrid: 1,419 iterations
  and 26.2 s of solving from scratch, 435 iterations and 8.1 s starting from the last 8 solutions.
  At 8 designs of an optimization in turn, a forward and an adjoint solve each on the same grid,
  starting each from the last design's two solutions takes the solves from 10.9 s to 7.2 s.

The recycled S-matrices agree with the ones solved from scratch to the tolerance:

::validation fdfd3d/block-qmr-s-matrix fdfd3d/recycle-s-matrix

## 7. Why does a design need gradients?

A design with $n$ parameters, $F(p_1, \ldots, p_n)$, improves fastest along its gradient. By finite
differences the gradient costs one solve per parameter forward, $n + 1$ in all, or two per
parameter centred, $2n$; a design region of cells has thousands. An optimizer without gradients
samples instead, and needs many more evaluations as $n$ grows.

[Zhang et al. (2013)](https://doi.org/10.1364/OE.21.001310) shaped a silicon Y-junction's taper,
13 widths, by particle swarm optimization on 2D FDTD: a swarm of 30 for 50 iterations found a
design below 0.2 dB, 0.13 dB in 3D FDTD (p. 1313), and the junctions made by 248 nm lithography
lost 0.28 dB on average across the wafer (p. 1315). [optimize_ybranch](../examples/optimize_ybranch.rs)
reruns the swarm on photonoxide's 2D FDFD with genoxide's particle swarm, 30 particles and 1,500
evaluations, its 11 inner widths free:

::example optimize_ybranch

Its swarm's design loses 0.028 dB in 2D, a 2D estimate by the effective index method. In 3D FDFD,
extrapolated from three grids, the same design loses 0.202 ± 0.027 dB. **Zhang et al.'s 0.13 dB is
not reproduced**: ours is 0.07 dB higher, beyond the example's 0.05 dB. Their own Table 1 design,
on the same 3D grids, loses 0.168 ± 0.003 dB in photonoxide against their 0.13 dB, so 0.04 dB of
the gap lies between their 3D FDTD and our 3D FDFD of one design, not between the designs; the
example reports both as gaps.

[Lalau-Keraly et al. (2013)](https://doi.org/10.1364/OE.21.021693) took the same junction with the
adjoint gradient and level sets, in 2D and then 3D FDTD: 51 iterations of steepest descent, 102
simulations, reached an insertion loss of 0.07 dB, against the swarm's 0.13 dB "after 1500
simulations" (pp. 21698–21699). Each iteration was two simulations, whatever the number of points
the boundary moved at.

## 8. What is the adjoint method?

A figure of merit $F$ depends on the design $p$ through the field: $A(p)\,x = b$, $F = F(x)$. The
adjoint method computes $dF/dp$ for every parameter from the forward solve and **one more solve**,
with $A^\mathsf{T}$, whose source is $F$'s derivative with respect to the field
([Veronis et al., 2004](https://doi.org/10.1364/OL.29.002288), Eqs. 2–4, p. 2288). The field of that
second solve, $\lambda$, is the response to a source placed where $F$ is measured; by reciprocity
it is what that point would radiate back into the device, and $F$'s derivative with respect to
the permittivity of each cell is the product of the two fields there
([Lalau-Keraly et al., 2013](https://doi.org/10.1364/OE.21.021693), Eq. 5, p. 21696). For the power
of a port's mode, $\lambda$ in front of the monitor is the mode sent backwards from it (their
Eq. 8).

The chart solves the slab and bump of photonoxide's adjoint validation cases: the power the left
port's mode delivers into the right port's mode, the forward field, the right port's mode sent
backwards, and the gradient in a window of 3,000 cells:

::chart fdfd-gradient

With $E$ along $z$ and the bump at $\varepsilon_b = 6$, the right port's mode receives 0.969498 of
the power. The gradient is largest where both fields are strong: 4.618 × 10⁻⁴ per unit of a cell's
relative permittivity, in the core beside the bump at (0.99, −0.07) µm. Where it is positive, more
permittivity there raises the power into the right port's mode; where negative, it lowers it; along
the core its sign alternates with the relative phase of the two fields. At the cell (1.0, 0.2) µm
in the bump the adjoint gives −1.6785388 × 10⁻⁴; a fourth-order central difference there takes 4
more solves and agrees to a relative 5.3 × 10⁻⁹. For all 3,000 cells of the window, central
differences would take 6,000 solves. With $\varepsilon_b$ set to oxide's 2.085 there is no bump,
the guide delivers all but 8 × 10⁻¹¹ of the power, and the gradient falls to 3.3 × 10⁻⁸ at its
largest: the power can't exceed 1, so at 1 it is at a maximum, where every first derivative
vanishes.

### The derivation {theory}

$F$ is real and $x$ complex. To first order, $dF = 2\operatorname{Re}(g^\mathsf{T}dx)$ with
$g = \partial F/\partial x$ the Wirtinger derivative, $\bar x$ held fixed. Differentiate the system
with $b$ fixed: $A\thinspace dx + dA\thinspace x = 0$, so $dx = -A^{-1}dA\thinspace x$ and

$$
\frac{dF}{dp} = -2\operatorname{Re}\negthinspace\left(g^\mathsf{T}A^{-1}\frac{\partial A}{\partial p}x\right)
= -2\operatorname{Re}\negthinspace\left(\lambda^\mathsf{T}\frac{\partial A}{\partial p}x\right),
\qquad A^\mathsf{T}\lambda = g ,
$$

since $g^\mathsf{T}A^{-1} = (A^{-\mathsf{T}}g)^\mathsf{T}$. The parameter appears only in
$\partial A/\partial p$, which is sparse and cheap; $\lambda$ is the same for every parameter. So
the whole gradient costs the forward solve, one solve with $A^\mathsf{T}$, and a sparse product per
parameter. Veronis et al. note that with a direct solver "the only additional cost" is one
back-substitution, "typically at least an order of magnitude smaller" than the factorization
(p. 2288); on photonoxide's 150,000-unknown 2D grid, about 40 ms against 0.45 s (Section 5).

For the power of a port's mode, $F = |a|^2$ with $a = c^\mathsf{T}x$ (Section 3), so
$g = \bar a\thinspace c$: the adjoint source sits on the monitor's two columns. With $E$ along $z$,
$\varepsilon_k$ enters $A$ only as $k_0^2\varepsilon_k$ on cell $k$'s diagonal, and

$$
\frac{\partial F}{\partial\varepsilon_k} = -2k_0^2\operatorname{Re}(\lambda_kx_k) ,
$$

the product of the two fields in cell $k$. With $H$ along $z$, $\varepsilon$ enters through the faces'
$1/\varepsilon_f$, each face's $\varepsilon_f$ the mean of its two cells, so a cell collects half of
each of its faces' derivatives ([FDFD adjoint gradients](../docs/methods/fdfd-adjoint.md)). In 3D,
with $V$ the product of the PMLs' stretches at each value of $\mathbf E$, $VA$ is symmetric, so
$A^\mathsf{T} = VAV^{-1}$ and the adjoint is a solve with $A$ itself, which an iterative solver
can recycle from the forward solve (Section 6).

The derivation holds when $b$ doesn't depend on $p$: the cells varied must be clear of the ports,
whose sources and modes are held fixed, and the gradient is with respect to the permittivity's
real part. Hence the chart's window.

### Adjoints in research {research}

[Molesky et al. (2018)](https://doi.org/10.1038/s41566-018-0246-9) put the method's worth in one
line: adjoint methods "yield full derivative information from the solution of a problem that is,
in every respect, no harder to solve than that of the original field" (Box 1, p. 662).
[Hughes et al. (2018)](https://doi.org/10.1021/acsphotonics.8b01522) extended the FDFD adjoint to
nonlinear problems in the frequency domain, its adjoint "solved once regardless of the number of
parameters" (p. 4782), and used it to design compact Kerr switches that send low- and high-power
light to different ports (p. 4781).

## 9. How is an adjoint gradient checked?

Against finite differences, the definition of a derivative. A central difference of step $\delta$
errs two ways: by **truncation**, the Taylor terms it drops, $\delta^2$ for the second-order formula
$[F(p + \delta) - F(p - \delta)]/2\delta$ and $\delta^4$ for the fourth-order
$[-F(p + 2\delta) + 8F(p + \delta) - 8F(p - \delta) + F(p - 2\delta)]/12\delta$; and by **round-off**,
the powers' own precision divided by $\delta$. The chart computes both at one cell, at steps from
10⁻⁶ to 0.3:

::chart fdfd-difference-step

In the bump's cell, with $E$ along $z$, the second-order difference draws a V. From the largest
step down, its truncation falls a decade for every half decade of $\delta$, as $\delta^2$:
7.9 × 10⁻⁷ at $\delta$ = 0.32, 7.9 × 10⁻⁸ at 0.1, 8.2 × 10⁻⁹ at 0.032. It comes closest at
$\delta = 10^{-2}$, 1.8 × 10⁻⁹, and below that step the powers' round-off takes over and the
difference grows as $1/\delta$, to 3.8 × 10⁻⁶ at 10⁻⁶. The powers' precision is set by the forward
solve, solved to a relative residual of 3.9 × 10⁻¹⁵. The fourth-order difference's truncation is
too small to show in this cell at any step drawn: its difference keeps falling as the step grows,
to 3.2 × 10⁻¹² at $\delta$ = 0.32. Both stay under the dashed line, what the fourth-order formula
can resolve from powers of that precision: the adjoint gradient is exact to all the differences
can tell. Photonoxide's validation cases make that comparison on three cells, in the bump, the core
and the oxide, at $\delta = 4 \times 10^{-3}$: the largest relative difference is 3.2 × 10⁻⁸ with
$E$ along $z$ and 2.1 × 10⁻⁹ with $H$ along $z$, each within the 1.6 × 10⁻⁷ and 2.1 × 10⁻⁷ the
differences resolve. In 3D, eight values against fourth-order differences agree to 2.8 × 10⁻¹⁰ of
the largest gradient, and the iterative solvers' adjoints agree with the direct solver's to
2.3 × 10⁻¹¹ (QMR) and 7.8 × 10⁻¹¹ (GMRES with recycling):

::validation fdfd/adjoint-gradient-ez fdfd/adjoint-gradient-hz fdfd3d/adjoint-gradient fdfd3d/recycle-adjoint

### FDFD's adjoint against FDTD's {theory}

An FDTD run's transforms at a frequency obey a frequency-domain system exactly, at the leapfrog's
frequency $\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$, and its transpose is another run with
the sources rescaled in its CPMLs ([FDTD adjoint gradients](../docs/methods/fdtd-adjoint.md)). In a
closed lossy box, where no PML differs between the two, FDTD's adjoint gradient at 144 values of
$\mathbf E$ equals FDFD's to 7.5 × 10⁻¹⁴; and Lalau-Keraly et al.'s backward mode (their Eq. 8) is
the adjoint field there to 5.5 × 10⁻¹³:

::validation fdtd/adjoint-fdfd fdtd/adjoint-backward-mode

**Try it.** Set the field to 2, $H$ along $z$, and the cell to 3, in the oxide. Where does each
difference come closest to the adjoint?

:::answer
Now the fourth order draws a V too: 2.4 × 10⁻⁵ at $\delta$ = 0.32 and 2.4 × 10⁻⁷ at 0.1, two decades
for one half decade of step, its truncation falling as $\delta^4$, to its closest, 5.9 × 10⁻¹⁰, at
$\delta = 10^{-2}$. The second order, 2.5 × 10⁻³ at 0.32 and 2.5 × 10⁻⁵ at 0.032, falls as
$\delta^2$ and comes closest, 1.3 × 10⁻⁸, at a smaller step, $10^{-3.5}$, where its truncation meets
the round-off. A higher order truncates less at the same step, so it meets the round-off at a larger
step and a smaller error.
:::

## 10. What turns a grid of cells into a design?

The adjoint gives $\partial F/\partial\varepsilon_k$ for every cell. A design adds the map from its
own variables to the cells' permittivities, and the chain rule carries the gradient back through
it. What photonoxide has:

- **Cells as parameters, in 2D FDFD.** `Solver2d::from_cells` takes a permittivity per cell, and
  `Solver2d::reuse_cells` rebuilds the system with new ones, keeping the analysis of its sparsity:
  each step of an optimization pays one numerical factorization
  ([FDFD adjoint gradients](../docs/methods/fdfd-adjoint.md)).
- **Densities, in 3D FDTD.** A `Design` is a box of the grid's cells, each with a density
  $\rho_v$ between 0 and 1 and $\varepsilon(\rho) = \varepsilon_0 + \rho(\varepsilon_1 - \varepsilon_0)$;
  each value of $\mathbf E$ averages the cells it overlaps, and
  $\partial F/\partial\rho_v = (\varepsilon_1 - \varepsilon_0)\sum_r s_{rv}\thinspace\partial F/\partial\varepsilon_r$,
  $s_{rv}$ cell $v$'s share of value $r$'s cell ([FDTD adjoint gradients](../docs/methods/fdtd-adjoint.md)).
- **A shape's dimension.** Averaging from the fill makes a cell's permittivity a smooth function
  of where an edge lies, so the adjoint gives the derivative with respect to a disk's radius:
  within 2.2 × 10⁻⁶ of finite differences.

::validation fdfd/fill-derivative-ez

A free density in every cell makes grey, fine-grained designs that can't be made. Density
topology optimization adds two maps before the permittivity. [Wang et al. (2010)](https://doi.org/10.1007/s00158-010-0602-y)
**filter** each density by a weighted average over a neighbourhood of radius $R$ (Eq. 4), which
sets a length scale, then **project** the result towards 0 or 1,

$$
\bar{\tilde\rho} = \frac{\tanh\beta\eta + \tanh\beta(\tilde\rho - \eta)}{\tanh\beta\eta + \tanh\beta(1 - \eta)}
$$

(Eq. 9, p. 770), $\eta$ the threshold and $\beta$ the sharpness. $\beta$ is raised in stages, a
continuation from a nearly linear map to a nearly binary one, so the design can form before it is
forced to 0 or 1 ([Christiansen and Sigmund, 2021](https://doi.org/10.1364/JOSAB.406048)).
Thresholds above and below 0.5 give eroded and dilated designs, and Wang et al.'s robust
formulation optimizes the eroded, intermediate and dilated designs at once, so the result
tolerates over- and under-etching; Christiansen and Sigmund design a demultiplexer so for ±8 nm of
erosion and dilation (p. 505). [Hammond et al. (2021)](https://doi.org/10.1364/OE.431188)
add a foundry's rules inside the optimization: minimum linewidth, spacing, curvature, area and
enclosed area (Fig. 1, p. 23917), for three broadband devices under nine foundries' rule sets.

**Photonoxide has none of these yet.** Filters, projection, robust designs and fabrication
constraints are planned for 0.7, with [the lesson on inverse design](inverse-design.md); they are
not shipped. A filter or a projection is a map before the permittivity, and its gradient that
map's transpose, so the adjoints above are what they will be built on.

### A design taken through stages {research}

[Piggott et al. (2015)](https://doi.org/10.1038/nphoton.2015.69) designed a 2.8 × 2.8 µm² silicon
splitter of 1,300 and 1,550 nm light with adjoint gradients on FDFD in three stages: the
permittivity free between silicon's and air's, then a level set's binary boundary, then
broadband, at ten wavelengths (p. 375). Made by electron-beam lithography, the devices measured
peak insertion losses of 1.8 and 2.4 dB and crosstalk below −11 dB (p. 376).

## 11. What does the adjoint do for a circuit, and how many evaluations does an optimizer take?

The same method works on any linear system, and a circuit is one: photonoxide's circuit solve is
$MB = S_bE$ with $M = I - S_b\Gamma$, $S_b$ the block-diagonal matrix of every component's
S-matrix, $\Gamma$ the connections, $E$ the external ports ([circuit adjoint](../docs/methods/circuit-adjoint.md)).
A parameter $\theta$ of one component changes only its block of $S_b$, on both sides of the system,
and the gradient of a real $F$ of the circuit's S-matrix is

$$
\frac{\partial F}{\partial\theta} = 2\operatorname{Re}\operatorname{tr}\negthinspace\left(\Lambda^\mathsf{T}\thinspace\partial_\theta S_b\thinspace A\right),
\qquad M^\mathsf{T}\Lambda = EG, \qquad A = \Gamma B + E ,
$$

$G$ the Wirtinger derivative of $F$ with respect to $S$: Section 8's result for a system whose
right-hand side depends on $\theta$ too. One transposed back-substitution per input serves every
parameter of every component, for any netlist. Against fourth-order differences of whole circuits
it agrees to 1.3 × 10⁻¹⁰ on an MZI and 7.1 × 10⁻¹² on a 4 × 4 mesh of 24 parameters, each the
differences' own limit:

::validation circuit/adjoint-mzi circuit/adjoint-mesh circuit/adjoint-differences

[Hughes et al. (2018)](https://doi.org/10.1364/OPTICA.5.000864) applied the adjoint variable method
to a mesh of interferometers and found in it "the photonic analogue of the backpropagation
algorithm", its gradient obtained exactly "by performing intensity measurements within the device"
(p. 864).

genoxide's optimizers take these gradients. [circuit_splitter](../examples/circuit_splitter.rs)
sets an MZI's arm phase for 30 % of the power in its bar port, exact with no grid: L-BFGS-B takes 8
evaluations, each a circuit solve and an adjoint solve, and Adam, at a learning rate of 0.02, 169:

::example circuit_splitter

[circuit_ring_critical](../examples/circuit_ring_critical.rs) tunes an all-pass ring's coupling and
radius by L-BFGS-B until its through port is dark at 1.55 µm, in 40 evaluations.
[circuit_fit](../examples/circuit_fit.rs) fits four parameters of an add-drop ring to its through
and drop spectra at 81 wavelengths: L-BFGS-B with the adjoint gradient in 79 evaluations, CMA-ES
from the response alone in 1,504, 19 times as many:

::example circuit_fit

## 12. When FDFD, and when FDTD?

The two solve the same discrete equations: an FDTD run's transforms are FDFD's fields at the
leapfrog's frequency (Section 9). Which is cheaper depends on what is asked:

- **One frequency, or a few, many sources.** FDFD: one factorization gives every port and the
  adjoint by back-substitution. FDTD needs a run per source and one more for the adjoint.
- **A band.** FDTD: one pulse gives every frequency in its band from one run. FDFD solves each
  wavelength, sharing what it can (Section 6).
- **High Q.** A resonator's energy falls as $e^{-\omega t/Q}$, by a factor $e$ every $Q/2\pi$
  periods, and FDTD must run until it has decayed, or fit the decay by harmonic inversion
  ([photonic crystals](photonic-crystals.md)). FDFD solves the
  steady state at once, but needs wavelengths a fraction of a linewidth $\lambda/Q$ apart to find a
  resonance.
- **Dispersive materials.** FDFD's matrix holds $\varepsilon(\omega)$ at its own frequency, from any
  table. FDTD needs a model that can be stepped in time, such as a Lorentz oscillator fitted to the
  data.
- **Memory.** FDTD and iterative FDFD keep a few copies of the field, growing as the unknowns. A
  direct factorization grows faster, $N\log N$ in 2D and about $N^{1.4}$ in 3D (Section 5).

The built-in jobs run the same devices both ways: `ring-fdfd.yaml` and `ring-fdtd.yaml`,
`mmi-fdfd.yaml` and `mmi-fdtd.yaml`, each in 2D by the effective index method, an estimate and not
a device's 3D performance. The MMI's outputs by FDTD agree with FDFD's within 4.3 × 10⁻⁴ at
1.55 µm and 1.1 × 10⁻³ at 1.6 µm, the difference growing away from the carrier the mode source
launches ([the FDTD solver](../docs/methods/fdtd.md)). For a gradient, FDTD's adjoint keeps only the
transforms over the design and costs about two forward runs: on 160 × 64 × 64 cells of 50 nm, a
design of 10,240 cells, 17.3 s forward and 17.8 s adjoint
([FDTD adjoint gradients](../docs/methods/fdtd-adjoint.md)).

### Limits {theory}

- **Local optima.** A gradient leads to the nearest optimum. "At most, a mathematical optimization
  method is able to guarantee local optimality", and different starting designs give
  "qualitatively different designs that perform similarly" (Christiansen and Sigmund, p. 499).
- **What photonoxide's adjoints differentiate.** The permittivity's real part, with the ports'
  sources and modes fixed: the varied cells must be clear of the ports, and a lossy design needs
  the imaginary part's gradient too, which isn't offered.
- **Fabrication.** A design is only as good as what the foundry makes of it (Section 10); 0.7's
  constraints are not shipped.
- **2D is an estimate.** A 2D design by the effective index method has to be verified in 3D, as
  Section 7's Y-junction shows.

## 13. History and current research {research}

[Yee (1966)](https://doi.org/10.1109/TAP.1966.1138693) gave the staggered grid both methods are
written on. [George (1973)](https://doi.org/10.1137/0710032)'s nested dissection made the sparse
factorization of a 2D grid cost $O(N^{3/2})$, and [Christ and Hartnagel (1987)](https://doi.org/10.1109/TMTT.1987.1133733)
solved Maxwell's equations at one frequency in 3D as one sparse system, with mode amplitudes for
the scattering matrix. Iterative solvers for such systems came with
[Freund and Nachtigal (1991)](https://doi.org/10.1007/BF01385726)'s QMR; open boundaries with
[Chew and Weedon (1994)](https://doi.org/10.1002/mop.4650071304)'s stretched coordinates, which
[Shin and Fan (2012)](https://doi.org/10.1016/j.jcp.2012.01.013) showed to keep the matrix well
conditioned; one-way sources with [Rumpf (2012)](https://doi.org/10.2528/PIERB11092006).
[Veronis et al. (2004)](https://doi.org/10.1364/OL.29.002288) brought the adjoint variable method to
FDFD, [Lalau-Keraly et al. (2013)](https://doi.org/10.1364/OE.21.021693) explained it by
reciprocity and turned it on [Zhang et al. (2013)](https://doi.org/10.1364/OE.21.001310)'s
swarm-designed Y-junction, and [Piggott et al. (2015)](https://doi.org/10.1038/nphoton.2015.69)
made and measured a demultiplexer designed by it. Topology optimization's filters and projections
([Wang et al., 2010](https://doi.org/10.1007/s00158-010-0602-y)) carried over from mechanical design.

::timeline

Today's questions are in the papers above: adjoints for nonlinear devices
([Hughes et al., 2018](https://doi.org/10.1021/acsphotonics.8b01522)) and for circuits trained on
the chip ([Hughes et al., 2018](https://doi.org/10.1364/OPTICA.5.000864)), designs that pass a
foundry's rule checks ([Hammond et al., 2021](https://doi.org/10.1364/OE.431188)), and the gap
between a design's 2D estimate, its 3D simulation and the device made, which Section 7's Y-junction
shows at each step.

Further reading: [Molesky et al. (2018)](https://doi.org/10.1038/s41566-018-0246-9), the review;
[Christiansen and Sigmund (2021)](https://doi.org/10.1364/JOSAB.406048), the tutorial; the method
pages on [FDFD in 2D](../docs/methods/fdfd.md), [in 3D](../docs/methods/fdfd-3d.md), its
[ports](../docs/methods/fdfd-ports.md) and [adjoint gradients](../docs/methods/fdfd-adjoint.md),
and [optimizing a device](../docs/methods/optimization.md); and the lessons on
[how FDTD works](how-fdtd-works.md) and [inverse design](inverse-design.md).

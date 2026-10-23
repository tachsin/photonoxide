---
title: "Leaky and bent guides, and multilayers"
summary: "Modes that lose power as they travel: leaky waves of multilayer guides by transfer matrices and by a full-vector solver with a PML, every mode in a region of the complex plane at once, and the radiation loss of a bend against its radius, exactly and by Marcuse's formula."
topic: Waveguides
level: advanced
minutes: 40
prerequisites: [slab-waveguide, strip-waveguide]
examples: [multilayer_chilwell, leaky_waves, bend_loss]
jobs: [leaky-wire.yaml]
methods: [multilayer.md, contour.md, pml.md, bends.md, vector.md]
validation:
  - mode/multilayer-bound-chilwell
  - mode/multilayer-leaky-chilwell
  - mode/multilayer-power-chilwell
  - mode/pml-soi-leakage-te
  - mode/pml-soi-leakage-tm
  - mode/pml-leaky-chilwell
  - job/leaky-film-loss
  - job/leaky-film-grid-order
  - job/leaky-film-pml
  - mode/contour-chilwell-bound
  - mode/contour-chilwell-leaky
  - mode/every-guided-mode-chilwell
  - mode/bend-marcuse
  - mode/bend-slab-te
  - mode/bend-slab-te-loss
  - mode/bend-slab-tm
  - device/bend-2d
  - device/bend-3d-grid
  - device/bend-analytic
charts: [leaky-field, leaky-substrate, bend-loss]
papers:
  - cite: "T. Tamir, A. A. Oliner, Proc. IEE 110, 310 (1963)"
    title: "Guided complex waves. Part 1: Fields at an interface"
    doi: 10.1049/piee.1963.0044
    year: 1963
    role: origin
    note: "Waves guided along an open interface with complex propagation constants classified: the surface waves that are bound, and the leaky waves whose fields grow away from the interface and carry power off it, with the region of space where such a field describes the radiation."
  - cite: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2103 (1969)"
    title: "Bends in Optical Dielectric Guides"
    doi: 10.1002/j.1538-7305.1969.tb01167.x
    year: 1969
    role: origin
    note: "The bent rectangular dielectric guide, its field shifted towards the outside of the bend and its radiation loss in closed form, falling exponentially with the radius, in the issue of the Bell System Technical Journal that opened integrated optics."
  - cite: "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971)"
    title: "Bending Losses of the Asymmetric Slab Waveguide"
    doi: 10.1002/j.1538-7305.1971.tb02620.x
    year: 1971
    role: milestone
    note: "The bent slab's field outside the bend as an outgoing Hankel function, matched to the straight slab's field inside: a loss formula for large radii, Eqs. 32–33, which photonoxide's exact bent slab is checked against."
  - cite: "M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975)"
    title: "Analysis of curved optical waveguides by conformal transformation"
    doi: 10.1109/JQE.1975.1068563
    year: 1975
    role: milestone
    note: "A bend mapped to a straight guide whose index grows outwards as 1 + x/R: any straight-guide solver then solves a bend, the map photonoxide's full-vector solver bends its cross-sections with."
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984)"
    title: "Thin-films field-transfer matrix theory of planar multilayer waveguides and reflection from prism-loaded waveguides"
    doi: 10.1364/JOSAA.1.000742
    year: 1984
    role: milestone
    note: "One 2 × 2 matrix per film for the tangential fields, a dispersion function whose real roots are the bound modes and whose complex roots on the outgoing branch are the leaky waves, and each mode's power in every layer: Tables 2 and 3, reproduced here."
  - cite: "W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994)"
    title: "A 3D perfectly matched medium from modified Maxwell's equations with stretched coordinates"
    doi: 10.1002/mop.4650071304
    year: 1994
    role: milestone
    note: "Maxwell's equations with complex stretched coordinates: a layer that absorbs a wave at any angle and both polarizations without reflecting it."
  - cite: "W. C. Chew, J. M. Jin, E. Michielssen, Microw. Opt. Technol. Lett. 15, 363 (1997)"
    title: "Complex coordinate stretching as a generalized absorbing boundary condition"
    doi: 10.1002/(SICI)1098-2760(19970820)15:6<363::AID-MOP8>3.0.CO;2-C
    year: 1997
    role: milestone
    note: "The perfectly matched layer as a change of variables to a complex coordinate, which a mode solver takes as complex grid spacings: the PML of photonoxide's full-vector solver."
  - cite: "E. Anemogiannis, E. N. Glytsis, T. K. Gaylord, J. Lightwave Technol. 17, 929 (1999)"
    title: "Determination of guided and leaky modes in lossless and lossy planar multilayer optical waveguides: reflection pole method and wavevector density method"
    doi: 10.1109/50.762914
    year: 1999
    role: milestone
    note: "A multilayer's guided and leaky modes found as the poles of its reflection coefficient, or as the peaks of a density of wavevectors, without searching the complex plane for the roots of the dispersion function."
  - cite: "T. Sakurai, H. Sugiura, J. Comput. Appl. Math. 159, 119 (2003)"
    title: "A projection method for generalized eigenvalue problems using numerical integration"
    doi: 10.1016/S0377-0427(03)00565-X
    year: 2003
    role: milestone
    note: "The eigenvalues inside a closed curve of the complex plane from contour integrals of the resolvent, taken by the trapezoidal rule, one independent linear solve per point."
  - cite: "Y. A. Vlasov, S. J. McNab, Opt. Express 12, 1622 (2004)"
    title: "Losses in single-mode silicon-on-insulator strip waveguides and bends"
    doi: 10.1364/OPEX.12.001622
    year: 2004
    role: milestone
    note: "Single-mode SOI strips and their bends measured: 0.086 dB per 90° bend at 1 µm radius and 0.013 dB at 2 µm, losses small enough to route silicon circuits with micrometre bends."
  - cite: "P. Bienstman, S. Selleri, L. Rosa, H. P. Uranus, W. C. L. Hopman, R. Costa, A. Melloni, L. C. Andreani, J. P. Hugonin, P. Lalanne, D. Pinto, S. S. A. Obayya, M. Dems, K. Panajotov, Opt. Quantum Electron. 38, 731 (2006)"
    title: "Modelling leaky photonic wires: A mode solver comparison"
    doi: 10.1007/s11082-006-9025-9
    year: 2006
    role: milestone
    note: "Many mode solvers on one silicon wire leaking through its buried oxide: the consensus 2.412372 + 2.9135e-8 i, which photonoxide's solver and its modes job are checked against."
  - cite: "E. Polizzi, Phys. Rev. B 79, 115112 (2009)"
    title: "Density-matrix-based algorithm for solving eigenvalue problems"
    doi: 10.1103/PhysRevB.79.115112
    year: 2009
    role: milestone
    note: "FEAST: a contour integral filters a block of vectors onto the eigenvectors inside the curve, and a Rayleigh–Ritz step on that block, repeated, gives every eigenvalue inside."
  - cite: "J. Hu, C. R. Menyuk, Adv. Opt. Photon. 1, 58 (2009)"
    title: "Understanding leaky modes: slab waveguide revisited"
    doi: 10.1364/AOP.1.000058
    year: 2009
    role: review
    note: "Guided, leaky and radiation modes told apart on the slab: why a leaky mode's field grows away from the guide, where it describes the radiated field and where it doesn't, and how a mode solver's window and absorbing layers find it."
  - cite: "W. Bogaerts, S. K. Selvaraja, IEEE Photonics J. 3, 422 (2011)"
    title: "Compact Single-Mode Silicon Hybrid Rib/Strip Waveguide With Adiabatic Bends"
    doi: 10.1109/JPHOT.2011.2142931
    year: 2011
    role: current
    note: "Shallow-etched rib waveguides for the straight runs, converted to strips for compact bends whose curvature grows gradually: a low-loss single-mode wire that still turns in a few micrometres."
  - cite: "M. Cherchi, S. Ylinen, M. Harjanne, M. Kapulainen, T. Aalto, Opt. Express 21, 17814 (2013)"
    title: "Dramatic size reduction of waveguide bends on a micron-scale silicon photonic platform"
    doi: 10.1364/OE.21.017814
    year: 2013
    role: current
    note: "Euler bends, whose curvature grows linearly along the path, in micron-scale silicon rib waveguides: bends of a few micrometres' radius with low loss, where a circular bend's sudden curvature would couple light into higher modes."
  - cite: "J. Kestyn, E. Polizzi, P. T. P. Tang, SIAM J. Sci. Comput. 38, S772 (2016)"
    title: "Feast Eigensolver for Non-Hermitian Problems"
    doi: 10.1137/15M1026572
    year: 2016
    role: current
    note: "FEAST for non-Hermitian matrices, whose eigenvalues are complex: the residual test, the subspace's size and its rank, as photonoxide's search for every leaky and lossy mode in a region uses them."
---

The two ways a guide loses light without absorbing it, in the symbols the equations below use.
At the left, a planar guide on a substrate of higher index than its mode's: the mode sheds a
plane wave into the substrate at an angle $\theta$, and its field grows into it. At the right, a
guide bent at a radius $R$: its phase fronts turn together, and beyond the radius $\rho_r$ they
would outrun light in the cladding, so the field radiates from there.

::diagram leaky

A guided mode keeps its power because its field decays on both sides of the guide. A **leaky
wave** has a cladding or substrate whose index is above its effective index, and a **bent
guide** has, beyond some radius, a cladding that its phase fronts outrun: in both the field far
from the core is a wave travelling away, the mode loses power as it goes, and its effective index
is complex, $n_\text{eff} = n' + in''$, its power falling as $e^{-2kn''z}$. The
[slab waveguide](slab-waveguide.md) holds the bound modes of three layers; here are the leaky
waves of any planar stack, found exactly by transfer matrices and on a grid by the full-vector
solver with a perfectly matched layer, every mode in a region of the complex plane at once, and
the radiation of a bend against its radius. The [strip waveguide](strip-waveguide.md) lesson's
Section 8 is one case: a silicon wire leaking through its buried oxide into the substrate.

## 1. What is a leaky wave?

A plane wave in a medium of index $n$ has the wavenumber $kn$, so along $z$ its phase advances
at most at $kn$. A mode whose $n'$ lies below the substrate's index $n_s$ is slower along $z$ than
light in the substrate can be, so in
the substrate it matches a plane wave travelling at an angle $\theta$ from the faces' normal, with
$n_s \sin\theta = n'$, and that wave carries power away from the guide. The power it carries off
is drawn from the mode, which therefore decays along $z$: $n'' \gt 0$.

The chart draws a mode of [Chilwell and Hodgkinson's (1984)](https://doi.org/10.1364/JOSAA.1.000742)
four-layer guide: a cover of 1.0, films of 1.66, 1.53, 1.60 and 1.66, 500 nm each, and a substrate
of 1.50, at 632.8 nm. Its TE modes $m = 0$ to 3 are bound, their indices above 1.50; $m = 4$ to 8
are leaky waves. TE₄, $n_\text{eff} = 1.461857 + 0.007156i$ (exact), loses 617 dB/mm; in the
substrate its field leaves at 77.0° from the normal and grows by 0.308 per µm, so 3 µm into the
substrate $|E_y|$ is 1.82 times its largest value in the films.

::chart leaky-field

### Why the field grows away from the guide {theory}

In the substrate the field is $e^{ik\alpha_s x}$ with $\alpha_s^2 = n_s^2 - n_\text{eff}^2$, $x$
measured from the substrate's face. Writing $n_\text{eff} = n' + in''$,

$$
\alpha_s^2 = n_s^2 - n'^2 + n''^2 - 2in'n'' .
$$

The wave leaves the guide, so $\operatorname{Re}\alpha_s \gt 0$; then
$\operatorname{Im}\alpha_s = -n'n''/\operatorname{Re}\alpha_s \lt 0$, and the field's size,
$e^{-k\operatorname{Im}\alpha_s\thinspace x}$, grows with $x$. The rays say the same: the light at
depth $x$ left the guide a distance $x\tan\theta$ upstream, $\tan\theta = n'/\operatorname{Re}\alpha_s$,
where the mode was stronger by $e^{kn''x\tan\theta}$, and $kn''\tan\theta = -k\operatorname{Im}\alpha_s$.
For TE₄ that is $9.929 \times 0.007156 \times \tan 77.0° = 0.308$ per µm, the chart's figure.

Such a field can't be normalized, so a leaky wave isn't one of the guide's modes in the strict
sense: those are the bound modes and a continuum of radiation modes.
[Tamir and Oliner (1963)](https://doi.org/10.1049/piee.1963.0044) classified these complex waves
at an open interface, and [Hu and Menyuk (2009)](https://doi.org/10.1364/AOP.1.000058) show on the
slab where a leaky wave describes the field: near the guide, over the length on which it was
launched, where it stands with one complex index for a band of the radiation continuum. A
substrate leak, a bend and a rib's leakage into its slab are of that kind, and photonoxide's
solvers find leaky waves as roots and eigenvalues, as they find bound modes.

**Try it.** Set the order to 8, then to 3. How do the field in the substrate and the angle change?

:::answer
TE₈, $1.003037 + 0.070771i$, leaves at 41.9° and grows by 0.630 per µm, 6.61 times its largest
value in the films 3 µm in: the lower $n'$, the steeper the wave and the faster the leak (6100
dB/mm). TE₃, 1.503587, is bound, just above the substrate's 1.50: it decays into the substrate,
but by only 1.03 per µm, so 3 µm in it is still 0.043 of its peak; Chilwell and Hodgkinson's
Table 3 puts 32.1 % of its power in the substrate.
:::

## 2. How do transfer matrices find a multilayer's bound modes and leaky waves?

Within each film the tangential fields, $U$ ($E_y$ for TE, $H_y$ for TM) and its derivative across
the film, are carried from one face to the other by a 2 × 2 matrix, so the whole stack is the
product of its films' matrices. In the cover and the substrate the field is a single exponential.
Matching the two at the stack's faces leaves one equation in $n_\text{eff}$, its dispersion
function $\chi(n_\text{eff}) = 0$, whose real roots above the cover's and substrate's indices are
the bound modes, and whose complex roots, on the branch where the field in the substrate travels
away, are the leaky waves. Nothing is discretized: `mode::multilayer` finds the roots to rounding.

[multilayer_chilwell](../examples/multilayer_chilwell.rs) reproduces Chilwell and Hodgkinson's
Table 3, the 8 bound modes and the share of each one's power in every layer, and their Table 2,
the 5 TE leaky waves below the substrate's index:

::example multilayer_chilwell

::validation mode/multilayer-bound-chilwell mode/multilayer-leaky-chilwell mode/multilayer-power-chilwell

Nine of the ten printed leaky numbers are ours rounded to 5 decimals; $m = 5$'s real part is
printed 1.38250 against 1.3824892, one unit in the last place, most likely the table's rounding,
since the bound modes agree to 7 digits
([the multilayer slab](../docs/methods/multilayer.md)).

### The matrices and the dispersion function {theory}

In a medium of index $n$ let $\alpha = \sqrt{n^2 - n_\text{eff}^2}$ and $\gamma = \alpha$ for TE or
$\alpha/n^2$ for TM. Film $j$, of thickness $d_j$ and phase thickness $\Phi_j = k\alpha_j d_j$,
carries $(U, V)$ across itself by Chilwell and Hodgkinson's Eq. 10,

$$
M_j = \begin{pmatrix} \cos\Phi_j & -\frac{i}{\gamma_j}\sin\Phi_j \cr -i\gamma_j \sin\Phi_j & \cos\Phi_j \end{pmatrix},
\qquad M = \prod_{j=1}^{J} M_j ,
$$

and a mode is a root of their Eq. 26,

$$
\chi = \gamma_c m_{11} + \gamma_c \gamma_s m_{12} + m_{21} + \gamma_s m_{22} = 0 .
$$

$\alpha_c$ and $\alpha_s$ are square roots, so $\chi$ lives on four sheets, one for each choice of
sign. A bound mode takes the root that decays away from the stack in both outer media,
$\operatorname{Im}\alpha \gt 0$; a leaky wave takes, in a medium whose index is above $n'$, the
outgoing root, $\operatorname{Re}\alpha \gt 0$ (their Section 3.C), which is the growing field of
Section 1. `Multilayer::bound_modes` brackets and bisects the real roots;
`Multilayer::modes_in` finds the complex ones in a rectangle of the plane from the minima of
$|\chi|$ on a grid, each refined by secant iteration; the field is their Eq. 34 and each layer's
power their Eq. 42.

**Try it.** In the chart, step the order from 0 to 3. Where does each bound mode keep its light?

:::answer
TE₀ (1.622729) sits mostly in the fourth film, next to the substrate, 76.3 % of its power, and
TE₁ (1.605276) in the first, 87.1 %: the two 1.66 films are each a guide, and the 1.53 and 1.60
films between them a barrier. TE₂ (1.557136) peaks in the third film, and TE₃ (1.503587) spreads
over all of them and puts 32.1 % in the substrate (Table 3, reproduced to 0.05 %).
:::

## 3. How does a full-vector solver find the same leaky waves?

A mode solver on a grid needs a window, and a window's edge reflects: a closed window holds no
wave travelling out. A **perfectly matched layer** inside the window's edge absorbs the outgoing
wave without reflecting it, and then the solver's eigenvalues include the leaky waves, with their
loss. [leaky_waves](../examples/leaky_waves.rs) lays Chilwell and Hodgkinson's guide flat in the
full-vector solver, uniform across between electric walls, on a 2.5 nm grid, with 1 µm of
substrate and a 2 µm PML of strength 5 below it:

::example leaky_waves

$m = 4$ to 7 are within $4 \times 10^{-5}$ of Table 2 on that grid. $m = 8$, at 1.00304, sits just
above the cover's index: in the cover its field decays slowly, with its phase running towards the
guide, a wave a PML can't absorb, so the example closes the cover with a zero wall and $m = 8$
is within $2 \times 10^{-4}$:

::validation mode/pml-leaky-chilwell

The exact transfer matrices are the PML's reference for a silicon film too. The chart takes 220 nm
of silicon (3.473) under oxide (1.444) on a buried oxide $h$ thick over the silicon substrate
(3.473): the film's mode, its index about 2.845 for TE, is above the oxide's and below the
substrate's, so it leaks across the oxide. On 1 µm of oxide at 1.55 µm the TE mode loses
$3.24 \times 10^{-4}$ dB/cm and the TM mode 0.676 dB/cm (exact); on 0.5 µm, 6.70 and 246 dB/cm. The
loss falls exponentially with $h$: the field decays across the oxide at $\gamma = 9.94$ per µm for
TE, so each further micrometre of oxide divides the loss by $e^{2\gamma \cdot 1\thinspace\mu\text{m}} = 4.27 \times 10^{8}$,
and the grey estimate $e^{-2\gamma h}$, drawn through the exact loss at 1.5 µm, is within 2.7 % of
it at 0.1 µm. TM's mode, at 2.05, is nearer the oxide's index, its field decays more slowly, and
it leaks more.

::chart leaky-substrate

### The perfectly matched layer {theory}

[Chew and Weedon (1994)](https://doi.org/10.1002/mop.4650071304) stretched the coordinates of
Maxwell's equations, $\partial/\partial x \to (1/s_x)\thinspace\partial/\partial x$, and showed that a
layer with complex $s_x$ absorbs a wave at any angle and either polarization without reflecting it.
Chew, Jin and Michielssen (1997)
made it a change of variable: the coordinate itself is complex inside the layer (their Eqs. 34, 44
and 45),

$$
\tilde x = \int_0^x s_x(x')\thinspace dx', \qquad s_x = 1 + i\alpha \left(\frac{u}{d}\right)^2
\thickspace\Rightarrow\thickspace \tilde x = x \pm i\alpha\thinspace\frac{u^3}{3 d^2},
$$

$u$ the depth into a layer of thickness $d$. The outgoing wave $e^{ik\alpha_s\tilde x}$ gains the
factor $e^{-k\operatorname{Re}\alpha_s\thinspace\alpha u^3/3d^2}$, which overwhelms the leaky wave's
growth and brings it to nothing at the window's edge. In the full-vector solver this is complex
grid spacings in the layer, which the solver's equations take as they are
([the PML](../docs/methods/pml.md)). A wave whose phase runs towards the guide, as $m = 8$'s does in
the cover, is amplified by the same layer instead: no PML takes it.

On the silicon film of the chart, through 0.5 µm of oxide, the solver's loss is within 0.04 % of
the exact one on a 2.5 nm grid, converging at second order, and a modes job with a PML in the
substrate (`[task.pml]`) gives the same film's loss on its own grid:

::validation mode/pml-soi-leakage-te mode/pml-soi-leakage-tm job/leaky-film-loss job/leaky-film-grid-order job/leaky-film-pml

A modes job names each mode it keeps guided or leaky, with its loss in dB/cm, and leaves out the
PML's own modes. `jobs/leaky-wire.yaml` solves Bienstman et al.'s leaky wire this way, the
[strip waveguide](strip-waveguide.md)'s Section 8.

**Try it.** Move the oxide to 1.5 µm. What does the standard wafer's 2 µm give, by the estimate?

:::answer
At 1.5 µm TE loses $1.57 \times 10^{-8}$ dB/cm and TM $1.84 \times 10^{-3}$ dB/cm. Half a
micrometre more divides TE's loss by $e^{2\gamma \cdot 0.5\thinspace\mu\text{m}} = 2.1 \times 10^{4}$, to
about $8 \times 10^{-13}$ dB/cm: nothing beside a strip's measured 2 to 3 dB/cm, which is why a
silicon device is solved with oxide below it and no substrate. TM's 2 µm loss, by its slope on the
chart, is near $5 \times 10^{-6}$ dB/cm.
:::

## 4. How can every mode in a region of $n_\mathrm{eff}$ be found at once?

Shift-and-invert finds a given number of modes nearest a guess. A leaky wave's index isn't known
in advance, and a count is not either: the question is every mode in a region of the complex
plane. A **contour integral** of the solver's resolvent around a closed curve projects onto the
eigenvectors whose eigenvalues lie inside it, all of them and no others, so the modes inside come
out with no guess and no count, and the trace of the projector counts them. photonoxide's
`vector::modes_in` and `slab_fd::Profile::modes_in` take a disc or an ellipse of $n_\text{eff}$
and return every mode inside ([contour integrals](../docs/methods/contour.md)).

On Chilwell and Hodgkinson's guide, by 1D finite differences on a 1 nm grid, the search over
$n_\text{eff}$ from 1.5005 to 1.67 returns the 4 TE and 4 TM bound modes, and a disc about $m = 4$
and an ellipse about $m = 5$ to 7, with 2 µm of PML in the substrate, return those leaky waves:

::validation mode/contour-chilwell-bound mode/contour-chilwell-leaky mode/every-guided-mode-chilwell

### The projector and its quadrature {theory}

For the eigenproblem $Ax = \lambda Bx$ of the solver, $\lambda = k^2 n_\text{eff}^2$, and a closed
curve $\Gamma$,

$$
P = \frac{1}{2\pi i}\oint_\Gamma (zB - A)^{-1} B \thinspace dz
$$

projects onto the eigenvectors inside $\Gamma$
([Kestyn, Polizzi and Tang, 2016](https://doi.org/10.1137/15M1026572), Eq. 2.4). The integral is
taken by the trapezoidal rule at $N$ points of the curve
(Sakurai and Sugiura, 2003), one independent
solve per point, which makes $P$ a rational filter: about 1 inside, and damping an eigenvalue
outside by about its distance in units of the curve's size to the power $N$.
[Polizzi's (2009)](https://doi.org/10.1103/PhysRevB.79.115112) FEAST applies it to a block of
vectors, solves the small problem projected on the filtered block (Rayleigh–Ritz), and repeats
until every Ritz pair inside has a small residual. For the four-layer guide's TE bound modes one
pass of 8 vectors has errors of $1.3 \times 10^{-2}$, $1.9 \times 10^{-4}$ and
$1.8 \times 10^{-8}$ with 6, 12 and 24 points: exponential in $N$. The points are solved side by
side and summed in order, so the result is the same bits on any number of threads.

A PML has modes of its own: a line of discrete eigenvalues rising from the substrate's index into
$\operatorname{Im} n_\text{eff}$. A region that cuts that branch holds eigenvalues of enormous
condition, the filtered block loses rank and the residuals stall; the search stops and says so,
and a region below the branch finds the leaky waves, as the validation's do. In 1D the exact
stack needs no projector: `Multilayer::modes_in` searches the rectangle of the plane, the
example's from 1.0 to 1.499 and from 0 to 0.1, by the minima of $|\chi|$ on a grid of
120 × 120 points. [Anemogiannis, Glytsis and Gaylord (1999)](https://doi.org/10.1109/50.762914)
avoid the search altogether: a multilayer's modes are the poles of its reflection coefficient.

## 5. Why does a bend radiate?

A mode of a guide bent at radius $R$ about an axis goes round as $e^{i\nu\varphi}$, its angular
order $\nu = kn_\text{eff}R$ with $n_\text{eff}$ taken along the arc at $R$. At a radius $\rho$
the phase advances along the arc at $\nu/\rho$ per unit length, so there the mode behaves like a
wave of index $n_\text{eff}R/\rho$, which falls as $\rho$ grows. Beyond

$$
\rho_r = R\thinspace\frac{n_\text{eff}}{n_2},
$$

$x_r = R(n_\text{eff}/n_2 - 1)$ outside the guide's centre, that index is below the cladding's
$n_2$: the field there can't decay, it travels outwards, and the mode radiates. Between the guide
and $\rho_r$ it is evanescent, a barrier the field tunnels through, and the barrier widens in
proportion to $R$, so the loss falls exponentially as the radius grows.

The chart draws a slab of 1.6 in 1.5, 1 µm thick, at 1 µm, the slab of
[Marcuse (1971)](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x)'s formula, solved exactly as
a bent slab. At 40 µm, E normal to the bend plane, it loses 0.339 dB per 90° and radiates from
1.92 µm outside its centre; at 20 µm 4.32 dB, at 80 µm $6.9 \times 10^{-4}$ dB and at 120 µm
$9.3 \times 10^{-7}$ dB (exact, no grid). Its index along the arc, 1.57205 at 40 µm, is above the
straight slab's 1.569443: the field moves towards the outside of the bend, where the path round is
longer.

::chart bend-loss

### The tunnelling exponent {theory}

[Heiblum and Harris (1975)](https://doi.org/10.1109/JQE.1975.1068563) map the bend to a straight
guide: with $u = R\ln(1 + x/R)$, the bent guide's wave equation becomes a straight one in $u$
whose index profile is $n(x)(1 + x/R)$ (their Eqs. 9–10). The cladding's index then rises outwards
and reaches $n_\text{eff}$ at $x_r$, the picture above as a straight guide with a tilted cladding.
[Marcuse (1971)](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x) takes the field outside the
bend as the outgoing Hankel function $H^{(1)}_\nu(kn_2\rho)$ (his Eq. 10) and its ratio of the
field at the guide and beyond the turning point, which sets the loss, as $e^{-U}$ with (his
Eq. 33, for a symmetric slab)

$$
U = \left(\frac{\beta}{\gamma}\ln\frac{1 + \gamma/\beta}{1 - \gamma/\beta} - 2\right)\gamma R,
\qquad \beta = kn_\text{eff},\quad \gamma = \sqrt{\beta^2 - k^2 n_2^2} .
$$

For a weak guide, $\gamma \ll \beta$, the logarithm's series
$\ln\frac{1+t}{1-t} = 2(t + t^3/3 + t^5/5 + \ldots)$ with $t = \gamma/\beta$ gives

$$
U = \frac{2}{3}\frac{\gamma^3}{\beta^2}R + O\left(\frac{\gamma^5}{\beta^4}R\right):
$$

the exponent grows in proportion to $R$ and to the cube of the field's decay rate outside the
guide. A stronger guide, a larger $\gamma$, bends much more tightly at the same loss: the chart's
figure gives $U = 7.06$ at 40 µm for the slab of 1.6 in 1.5, 0.177 per µm, and 15.8 at 2 µm for
silicon's 2.845 in 1.444, 7.9 per µm. The chart draws each slab from $U = 3$ to 30.

**Try it.** Raise the core to 1.7 and find the radius at which the loss is the default's 0.339 dB
per 90°.

:::answer
Between 10 µm (0.687 dB) and 12 µm (0.266 dB), near 11.5 µm instead of 40 µm, where $U$ is about
7.3, near the default's 7.06: the loss is set by $U$. The straight slab's $\gamma$ grows from 2.90
to 4.51 per µm, and $\gamma^3$ 3.8 times.
:::

## 6. How accurate is Marcuse's bend-loss formula?

Marcuse's formula takes the field inside the bend to be the straight guide's, which holds when
the bend changes the field little, at large radii. Against the exact bent slab of the chart its
loss is 1.83 times too large at 20 µm, 1.36 at 40 µm, 1.14 at 80 µm and 1.08 at 120 µm, its excess
falling about as $1/R$; [bend_loss](../examples/bend_loss.rs) prints the same and checks 120 µm
within 10 %:

::example bend_loss

::validation mode/bend-marcuse

On a silicon slab, the book's strip seen from above as 2.845 in 1.444, 500 nm wide, at 1.55 µm,
the bends are micrometres. E normal to the bend plane it loses 0.0514 dB per 90° at 1 µm and
$5.6 \times 10^{-5}$ dB at 2 µm; E in the bend plane, the polarization of a strip's TE-like mode
bent in the chip's plane, 0.168 and $5.07 \times 10^{-4}$ dB (exact, no grid; a slab, not a strip).
Marcuse is 1.96 times too large at 1 µm and 1.36 at 2 µm.

::chart bend-loss{core=2.845, cladding=1.444, thickness=500nm, wavelength=1.55um, radius=2um}

### The exact bent slab, and the conformal map {theory}

`mode::bend` approximates nothing in Marcuse's Eq. 10. In each layer the field obeys
$\psi'' + \psi'/\rho + (k^2n^2 - \nu^2/\rho^2)\psi = 0$, with $\psi$ and $\psi'$ continuous at an
interface for E normal to the plane, $\psi$ and $\psi'/n^2$ for E in it. From inside, the solution
growing outwards is integrated by Runge–Kutta to the middle of the stack; from outside, the
outgoing wave's $\psi'/\psi$ comes from Debye's expansions of $H^{(1)}_\nu$ and is integrated in to
the same point; $\nu$ is the root of their difference, by secant iteration
([bends](../docs/methods/bends.md)). The loss resolves down to $n''/n' \approx 10^{-16}$, where the
outgoing part is lost to round-off: the chart leaves out what is below $10^{-15}$.

A cross-section of any shape bends in the full-vector solver by Heiblum and Harris's map, each
cell's permittivity scaled by $(1 + x/R)^2$, with a PML outside the bend. The map is exact for the
scalar wave equation, so for E normal to the bend plane; for E in it the exact equivalent medium is
anisotropic in both $\varepsilon$ and $\mu$, and scaling an isotropic $\varepsilon$ is an
approximation whose error falls with the radius. On the silicon slab on a 5 nm grid, at 1 µm,
the example finds $2.757741 + 9.297 \times 10^{-4}i$ against the exact $2.757800 + 9.295 \times 10^{-4}i$
for E normal, and $2.552891 + 3.293 \times 10^{-3}i$ against $2.551645 + 3.037 \times 10^{-3}i$ for E
in the plane, 8 % high in the loss; at 3 µm that error is 1 %:

::validation mode/bend-slab-te mode/bend-slab-te-loss mode/bend-slab-tm

**Try it.** In the silicon chart, move the radius to 1 µm and then 3 µm. How does the ratio of the
two polarizations' losses change?

:::answer
At 1 µm E in the plane loses 0.168 dB per 90° and E normal 0.0514, 3.3 times less; at 3 µm
$1.08 \times 10^{-6}$ against $3.6 \times 10^{-8}$ dB, 30 times. E in the plane is the slab's TM
mode, of lower index: its field decays more slowly outside the slab, a smaller $\gamma$, so by
Section 5 its exponent $U$ is smaller at every radius, and its loss is larger and falls more
slowly. Marcuse, for E normal, is 1.22 times the exact at 3 µm.
:::

## 7. What does photonoxide's bend device compute?

The **bend** device bends a strip on a stack, 500 × 220 nm on 220 nm SOI bent at 10 µm by default,
through an arc of a given radius and angle, and computes it at three fidelities:

- **Analytic:** the straight strip's mode along the arc, with no radiation and no change of the
  mode by the bend: within $3 \times 10^{-3}$ of the 3D index from 5 µm radius;
- **2D:** the exact bent slab of Section 6 for the strip seen from above, its core the effective
  index method's slab index: its radiation and the bend's shift of the mode, with no grid, but on
  a model whose own error, the method's few per cent in the index, is larger;
- **3D:** the straight strip by the full-vector solver on cells fitted about 10 nm apart, plus the
  bend's shift: the strip's cross-section mapped conformally with a PML beyond the turning
  point, less the same window straight on the same cells, so that the corners' error cancels.

[bend_loss](../examples/bend_loss.rs) prints the device bent at 2 µm at 1.55 µm (above): its 2D
model gives $n_\text{eff}$ 2.50566 along the arc and $4.85 \times 10^{-4}$ dB per 90° of radiation,
a 2D number and not the bend's; its 3D model 2.46021 and $2.63 \times 10^{-4}$ dB per 90°, on
125 × 111 cells of 10 nm for the straight strip and 250 × 66 cells of 6.97 nm for the shift.
[Vlasov and McNab (2004)](https://doi.org/10.1364/OPEX.12.001622) measured 0.013 dB per 90° on
their strips bent at 2 µm, fifty times that radiation. A measured bend also loses light where the
straight guide meets the arc, whose modes differ, and at its rough sidewalls; the device includes
neither, so its radiation is a lower bound on a fabricated bend's loss.

::validation device/bend-2d device/bend-3d-grid device/bend-analytic

::device bend

## 8. History and current research {research}

[Tamir and Oliner (1963)](https://doi.org/10.1049/piee.1963.0044) gave leaky waves their theory,
complex waves at an open interface whose fields grow away from it. Integrated optics met
radiating guides at once: in 1969
[Marcatili](https://doi.org/10.1002/j.1538-7305.1969.tb01167.x) gave the radiation loss of a bent
rectangular guide in closed form, and [Marcuse (1971)](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x)
matched the bent slab's field to the outgoing Hankel wave for his formula.
[Heiblum and Harris (1975)](https://doi.org/10.1109/JQE.1975.1068563) turned a bend into a straight
guide with a tilted index, so that straight-guide solvers solve bends.
[Chilwell and Hodgkinson (1984)](https://doi.org/10.1364/JOSAA.1.000742) put any planar stack's
bound modes and leaky waves into one 2 × 2 matrix per film, and
[Anemogiannis, Glytsis and Gaylord (1999)](https://doi.org/10.1109/50.762914) found them as poles
of the stack's reflection. The perfectly matched layer of
[Chew and Weedon (1994)](https://doi.org/10.1002/mop.4650071304), as
Chew, Jin and Michielssen (1997)
wrote it, gave grid solvers leaky modes of any cross-section, and
[Bienstman et al. (2006)](https://doi.org/10.1007/s11082-006-9025-9) set them a common leaky
benchmark. Sakurai and Sugiura (2003) and
[Polizzi (2009)](https://doi.org/10.1103/PhysRevB.79.115112) found eigenvalues inside a curve by
contour integrals; [Hu and Menyuk (2009)](https://doi.org/10.1364/AOP.1.000058) set out what a
leaky mode is and isn't.

::timeline

Silicon made bends small: [Vlasov and McNab (2004)](https://doi.org/10.1364/OPEX.12.001622)
measured 0.013 dB per 90° at 2 µm radius in single-mode strips. Today a bend is designed for more
than its radiation. [Bogaerts and Selvaraja (2011)](https://doi.org/10.1109/JPHOT.2011.2142931)
route light in low-loss shallow ribs and convert them to strips for bends whose curvature grows
gradually, and [Cherchi et al. (2013)](https://doi.org/10.1364/OE.21.017814) shrink bends in
micron-scale silicon ribs with Euler curves, whose curvature grows linearly along the path, so the
mode follows the bend instead of being thrown into higher modes at its start. On the numerical
side, [Kestyn, Polizzi and Tang (2016)](https://doi.org/10.1137/15M1026572) carried FEAST to the
non-Hermitian problems a PML makes, the search photonoxide uses for every leaky mode in a region.

Further reading: [Hu and Menyuk (2009)](https://doi.org/10.1364/AOP.1.000058) for what a leaky mode
is; [Chilwell and Hodgkinson (1984)](https://doi.org/10.1364/JOSAA.1.000742), Sections 2–3, for the
matrices and their branches; [Marcuse (1971)](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x)
for the bent slab; and [the bends](../docs/methods/bends.md) and
[contour integrals](../docs/methods/contour.md) write-ups for how photonoxide computes them.

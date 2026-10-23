---
title: "The strip waveguide and its cross-section"
summary: "A silicon strip confines light in both directions across it: why its modes are hybrid, how many it guides and how wide it can be with one, the effective index method and Marcatili's approximation against the full-vector solve, the corners that slow every grid's convergence, the group index, leakage into the substrate, and what the strip waveguide device computes at each fidelity."
topic: Waveguides
level: intermediate
minutes: 45
prerequisites: [slab-waveguide]
examples: [strip_waveguide, effective_index_method, marcatili, hadley_corners, group_index, mzi_dwivedi, leaky_wire_benchmark, rib_hadley]
jobs: [strip-modes.yaml, strip-width-sweep.yaml, strip-guided-modes.yaml]
methods: [vector.md, eim.md, marcatili.md, hadley.md, dispersion.md, devices.md, eigen.md, walls.md, pml.md]
validation:
  - mode/strip-book
  - device/strip-chrostowski
  - mode/eim-strip-book
  - mode/marcatili-closed-form
  - mode/marcatili-vector
  - mode/hadley-box-low
  - mode/hadley-box-high
  - mode/hadley-corners-box-low
  - mode/hadley-corners-box-high
  - mode/hadley-corners-order
  - device/waveguide-3d-order
  - mode/strip-group-index-book
  - device/wire-ng-dwivedi
  - mode/leaky-wire-bienstman
  - mode/leaky-wire-bienstman-loss
  - job/leaky-wire-bienstman
  - job/leaky-wire-bienstman-loss
  - device/rib-hadley
  - device/rib-eim
  - device/rib-wide-slab
  - mode/vector-slab-limit-te
  - mode/vector-slab-limit-tm
  - device/waveguide-fit-neff
  - device/waveguide-fit-ng
charts: [strip-fields, strip-neff, strip-models, strip-corners, strip-group-index]
papers:
  - cite: "S. E. Miller, Bell Syst. Tech. J. 48, 2059 (1969)"
    title: "Integrated Optics: An Introduction"
    doi: 10.1002/j.1538-7305.1969.tb01165.x
    year: 1969
    role: origin
    note: "Laser-beam circuits in miniature, guided by index changes in a substrate and patterned by photolithography: the proposal that opened the issue of the Bell System Technical Journal in which Marcatili and Goell solved the rectangular guide."
  - cite: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969)"
    title: "Dielectric Rectangular Waveguide and Directional Coupler for Integrated Optics"
    doi: 10.1002/j.1538-7305.1969.tb01166.x
    year: 1969
    role: origin
    note: "The rectangular dielectric guide solved by leaving out the field in its four corner regions: the field then separates, and each mode follows from two slab-like equations, with closed forms good to a few per cent away from cutoff; and the directional coupler of two such guides."
  - cite: "J. E. Goell, Bell Syst. Tech. J. 48, 2133 (1969)"
    title: "A Circular-Harmonic Computer Analysis of Rectangular Dielectric Waveguides"
    doi: 10.1002/j.1538-7305.1969.tb01168.x
    year: 1969
    role: origin
    note: "The rectangular guide's modes computed with the corners included: the field expanded in Bessel and modified Bessel functions inside and outside the core, matched at points on its boundary, and the propagation constants and mode patterns plotted; Marcatili's approximation was checked against these curves."
  - cite: "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977)"
    title: "Mode dispersion in diffused channel waveguides by the effective index method"
    doi: 10.1364/AO.16.000113
    year: 1977
    role: milestone
    note: "The effective index method, after Knox and Toulios: a slab across the guide's thin dimension, then a slab of that slab's index between the side claddings, keeping the TE or TM character of each step for large index steps."
  - cite: "R. A. Soref, J. Schmidtchen, K. Petermann, IEEE J. Quantum Electron. 27, 1971 (1991)"
    title: "Large single-mode rib waveguides in GeSi-Si and Si-on-SiO2"
    doi: 10.1109/3.83406
    year: 1991
    role: milestone
    note: "Rib waveguides several microns wide and high that guide one mode, because their higher modes leak into the slab beside the rib: silicon guides that a single-mode fibre couples to efficiently."
  - cite: "K. K. Lee, D. R. Lim, L. C. Kimerling, J. Shin, F. Cerrina, Opt. Lett. 26, 1888 (2001)"
    title: "Fabrication of ultralow-loss Si/SiO2 waveguides by roughness reduction"
    doi: 10.1364/OL.26.001888
    year: 2001
    role: milestone
    note: "Sidewall roughness as the loss of submicron silicon strips: oxidation smoothing took a 0.5 µm single-mode strip from 32 dB/cm to 0.8 dB/cm."
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1210 (2002)"
    title: "High-accuracy finite-difference equations for dielectric waveguide analysis I: uniform regions and dielectric interfaces"
    doi: 10.1109/JLT.2002.800361
    year: 2002
    role: milestone
    note: "Finite-difference equations built from the exact local solutions of the Helmholtz equation instead of Taylor series: sixth order in uniform regions and at straight interfaces."
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002)"
    title: "High-accuracy finite-difference equations for dielectric waveguide analysis II: dielectric corners"
    doi: 10.1109/JLT.2002.800371
    year: 2002
    role: milestone
    note: "The same for dielectric corners, where the field's derivatives are singular: Bessel functions of fractional order in the local solution, about second order where standard schemes manage about first, and four corner problems solved to 1e-8 by series expansions."
  - cite: "Y. A. Vlasov, S. J. McNab, Opt. Express 12, 1622 (2004)"
    title: "Losses in single-mode silicon-on-insulator strip waveguides and bends"
    doi: 10.1364/OPEX.12.001622
    year: 2004
    role: milestone
    note: "Single-mode SOI strips made on a 200 mm CMOS line: 3.6 ± 0.1 dB/cm for TE at 1.5 µm, and 0.086 dB per 90° bend at 1 µm radius, 0.013 dB at 2 µm."
  - cite: "P. Dumon, W. Bogaerts, V. Wiaux, J. Wouters, S. Beckx, J. Van Campenhout, D. Taillaert, B. Luyssaert, P. Bienstman, D. Van Thourhout, R. Baets, IEEE Photon. Technol. Lett. 16, 1328 (2004)"
    title: "Low-Loss SOI Photonic Wires and Ring Resonators Fabricated With Deep UV Lithography"
    doi: 10.1109/LPT.2004.826025
    year: 2004
    role: milestone
    note: "Single-mode photonic wires patterned by deep-UV lithography and dry etching, 2.4 dB/cm, and racetrack and ring add-drop filters with Q above 3000."
  - cite: "W. Bogaerts, R. Baets, P. Dumon, V. Wiaux, S. Beckx, D. Taillaert, B. Luyssaert, J. Van Campenhout, P. Bienstman, D. Van Thourhout, J. Lightwave Technol. 23, 401 (2005)"
    title: "Nanophotonic waveguides in silicon-on-insulator fabricated with CMOS technology"
    doi: 10.1109/JLT.2004.834471
    year: 2005
    role: milestone
    note: "Photonic wires and photonic-crystal waveguides made with the same CMOS tools and compared: 0.24 dB/mm for the wires against 7.5 dB/mm for the crystal guides."
  - cite: "A. C. Turner, C. Manolatou, B. S. Schmidt, M. Lipson, M. A. Foster, J. E. Sharping, A. L. Gaeta, Opt. Express 14, 4357 (2006)"
    title: "Tailored anomalous group-velocity dispersion in silicon channel waveguides"
    doi: 10.1364/OE.14.004357
    year: 2006
    role: milestone
    note: "Anomalous group-velocity dispersion measured in silicon strips across the telecommunication bands, tuned from −2000 to 1000 ps/(nm km) by the cross-section's size and shape."
  - cite: "P. Bienstman, S. Selleri, L. Rosa, H. P. Uranus, W. C. L. Hopman, R. Costa, A. Melloni, L. C. Andreani, J. P. Hugonin, P. Lalanne, D. Pinto, S. S. A. Obayya, M. Dems, K. Panajotov, Opt. Quantum Electron. 38, 731 (2006)"
    title: "Modelling leaky photonic wires: A mode solver comparison"
    doi: 10.1007/s11082-006-9025-9
    year: 2006
    role: milestone
    note: "Many mode solvers on one leaky SOI wire, 500 × 220 nm on 1 µm of oxide over silicon under air: the consensus 2.412372 + 2.9135e-8 i, which photonoxide's solver is checked against."
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008)"
    title: "Vector Finite Difference Modesolver for Anisotropic Dielectric Waveguides"
    doi: 10.1109/JLT.2008.923643
    year: 2008
    role: milestone
    note: "The coupled equations for the transverse magnetic field discretized on a rectilinear grid, the interface conditions built into the coefficients, as a sparse eigenproblem for β²: the scheme of photonoxide's full-vector solver."
  - cite: "J. Hu, C. R. Menyuk, Adv. Opt. Photon. 1, 58 (2009)"
    title: "Understanding leaky modes: slab waveguide revisited"
    doi: 10.1364/AOP.1.000058
    year: 2009
    role: review
    note: "Guided, leaky and radiation modes told apart on the slab: why a leaky mode's field grows away from the guide, and how a mode solver's window and absorbing layers find it."
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    title: "Silicon Photonics Design"
    doi: 10.1017/CBO9781316084168
    year: 2015
    role: review
    note: "Silicon photonics designed from the wafer up: the 500 × 220 nm strip's TE-like index (Fig. 3.14), the effective index method's error (Section 3.2.5) and the group index against width (Fig. 3.22b), all reproduced here."
  - cite: "S. Dwivedi, A. Ruocco, M. Vanslembrouck, T. Spuesens, P. Bienstman, P. Dumon, T. Van Vaerenbergh, W. Bogaerts, J. Lightwave Technol. 33, 4471 (2015)"
    title: "Experimental Extraction of Effective Refractive Index and Thermo-Optic Coefficients of Silicon-on-Insulator Waveguides Using Interferometers"
    doi: 10.1109/JLT.2015.2476603
    year: 2015
    role: current
    note: "The effective and group indices of fabricated silicon wires, 470, 602 and 805 nm wide, measured with interferometers and checked against their SEM cross-sections."
  - cite: "S. Y. Siew, B. Li, F. Gao, H. Y. Zheng, W. Zhang, P. Guo, S. W. Xie, A. Song, B. Dong, L. W. Luo, C. Li, X. Luo, G.-Q. Lo, J. Lightwave Technol. 39, 4374 (2021)"
    title: "Review of Silicon Photonics Technology and Platform Development"
    doi: 10.1109/JLT.2021.3066203
    year: 2021
    role: current
    note: "The foundries that make silicon photonics, from research pilot lines to commercial ones, and the process design kits whose libraries of tested devices, strips among them, designers build on."
---

The strip in section, in the symbols the equations below use: a silicon core of index $n_1$, $w$
wide and $t$ thick, in oxide of index $n_2$ on the buried oxide of an SOI wafer; the TE-like
mode's field around it; the four corners and the regions beside them that Marcatili's
approximation leaves out; and, at the right, the effective index method's two slabs. The 3D view
is the strip of `jobs/strip-modes.yaml`.

::diagram strip

A strip waveguide is a slab cut to a width: a ridge of silicon etched through the 220 nm film of
an SOI wafer, the [slab waveguide](slab-waveguide.md) whose light is now held sideways as well as
up and down. It is the wire every silicon circuit is routed with, and the guide that couplers,
rings and interferometers are built from; 500 × 220 nm at 1550 nm is the strip
[Chrostowski and Hochberg (2015)](https://doi.org/10.1017/CBO9781316084168) design with (their
Fig. 3.14). A strip has no exact solution in closed form. Its modes come from a numerical
solve of the full vector fields, or from approximations that reduce it to slabs, and the corners
of its cross-section set how fast a grid converges.

## 1. Why are a strip's modes neither TE nor TM?

A slab is uniform along $y$, so its fields split into two independent sets: TE, with $E_y$ alone
across the film, and TM, with $H_y$ alone. A strip has faces normal to $x$ and faces normal to
$y$. The electric field that lies along the top and bottom faces crosses the sidewalls, where its
normal component jumps by the ratio of the permittivities, $(n_1/n_2)^2 = 5.8$ for silicon in
oxide. The two transverse components are coupled at every face, and each mode has all six
field components. The modes are named by their main field: **TE-like**, $E_x$ mostly across the
width, and **TM-like**, $E_y$ mostly up the height, with an order counting the zeros of the main
field across the width: TE₀, TE₁, TM₀.

The chart shows the size of each electric field component of a strip's mode. For the
500 × 220 nm strip's TE₀, $E_x$ fills the core and reaches into the oxide beside the sidewalls.
$E_y$, which a slab's TE mode doesn't have, gathers at the four corners, where it reaches 0.82
of $E_x$'s peak on these 10 nm cells. $E_z$, the field along the guide, peaks at the sidewalls at
0.42 of it: in a strip of this contrast the field is far from transverse.

::chart strip-fields

### The vector wave equation {theory}

With fields varying as $e^{i(\beta z - \omega t)}$, Faraday's law $\nabla \times \mathbf E = i\omega\mu_0 \mathbf H$
and Ampère's law $\nabla \times \mathbf H = -i\omega\varepsilon_0 n^2 \mathbf E$ give, eliminating $\mathbf E$,

$$
\nabla \times \left( \frac{1}{n^2} \nabla \times \mathbf H \right) = k^2 \mathbf H, \qquad k = \frac{\omega}{c}.
$$

Where $n$ is uniform this is $\nabla^2 \mathbf H + k^2 n^2 \mathbf H = 0$, one scalar equation for
each Cartesian component, uncoupled. The coupling comes from the derivative of $1/n^2$, which is
non-zero only at the faces: there $\mathbf H$ is continuous, but $(1/n^2)\nabla \times \mathbf H$, the
electric field, keeps only its tangential part continuous. In a slab, $n$ depends on $x$ alone
and the derivative couples nothing that the slab's TE and TM sets don't already hold. In a strip
it is non-zero across $x$ at the sidewalls and across $y$ at the top and bottom, and it couples
$H_x$ to $H_y$. [Fallahkhair, Li and Murphy (2008)](https://doi.org/10.1109/JLT.2008.923643) write
the two coupled equations for $H_x$ and $H_y$ alone, using $\nabla \cdot \mathbf H = 0$ to remove
$H_z$, and discretize them with the interface conditions in the coefficients; photonoxide's
[full-vector solver](../docs/methods/vector.md) is that scheme. A mode's **TE fraction** is the
share of its transverse magnetic field in $H_y$, summed over the grid: 0.975 for the strip's TE₀
on 10 nm cells, 0.0066 for its TM₀.

**Try it.** Set the mode to 2. Which field is the main one now, and where does $|E_x|$ sit?

:::answer
The second mode of the 500 × 220 nm strip is TM₀, $n_\text{eff}$ 1.77914: its main field is
$E_y$, up the height, and it reaches far into the oxide above and below the strip. $|E_x|$
gathers at the corners again, at 0.83 of $|E_y|$'s peak, and $|E_z|$, 0.37 of it, sits at the
top and bottom faces, the faces $E_y$ crosses.
:::

## 2. What is the TE-like mode of a 500 × 220 nm strip?

Chrostowski and Hochberg solve the 500 × 220 nm strip of silicon (3.473) in oxide (1.444) at
1550 nm with Lumerical MODE on a 20 nm conformal mesh and find $n_\text{eff} = 2.443$ for its
TE-like mode (Fig. 3.14). [strip_waveguide](../examples/strip_waveguide.rs) solves the same
cross-section with photonoxide's full-vector solver on the quarter of their 2.1 × 1.5 µm window,
on halving grids, by both of its schemes:

::example strip_waveguide

The standard scheme gives 2.447067, 2.445713, 2.444396 and 2.443506 at 20, 10, 5 and 2.5 nm
([the full-vector solver](../docs/methods/vector.md)): the halvings move the index by
$1.35 \times 10^{-3}$, $1.32 \times 10^{-3}$ and $0.89 \times 10^{-3}$, where a second-order scheme's
steps would shrink fourfold each time. That slow approach is the corners' doing (Section 6), and
the [grid study](../docs/methods/convergence.md) doesn't pass it at $10^{-4}$. Hadley's equations
give 2.442212, 2.442178 and 2.442168 at 10, 5 and 2.5 nm, at order 1.84. Richardson's estimate
of the 2.5 nm value's error is $3.7 \times 10^{-6}$, and the limit is 2.442165, $8.4 \times 10^{-4}$
under the book's 2.443 and within its $10^{-3}$. The example quotes Hadley's index and gates it
by its grids (2.5 nm with `--full`).
The validation report holds the 5 nm value against the book's, and the same strip solved through
the strip waveguide device:

::validation mode/strip-book device/strip-chrostowski

The book's index, like the example's, is for fixed indices. The strip waveguide device solves its
cross-section with the catalogue's materials, Li's silicon (3.4757 at 1550 nm) and Malitson's
silica (1.4440), which raise the TE-like index by about 0.003; the charts here solve the
device's own cross-section.

## 3. How many modes does a strip guide, and how wide can it be with one?

A mode is guided while its effective index is above the oxide's: its field then decays away from
the strip on every side. The chart draws every guided mode of a 220 nm strip against its width,
solved on the device's cross-section. Each mode starts at the oxide's index at its cutoff width
and rises towards the index of the 220 nm slab's mode of its polarization, the strip's limit as
it widens without bound: 2.84749 for TE and 2.05312 for TM with the catalogue's materials
(exact, `mode::slab`). At 1550 nm on cells about 20 nm apart, TE₁ appears at 452 nm: below it
the strip has one TE-like mode. At 500 nm it guides TE₀ (2.4499), TM₀ (1.7849) and, just above cutoff,
TE₁ (1.5001).

::chart strip-neff

Three features of the chart follow from the strip's shape:

- **Below 220 nm wide the fundamental mode is TM-like.** At $w = t$ the square's two
  fundamental modes are one rotated onto the other and have the same index, 1.52872 (20 nm
  cells); narrower,
  the strip is taller than it is wide, and the mode whose main field lies along the longer side,
  TM₀, has the higher index.
- **Neither fundamental mode has a cutoff in width,** as a symmetric slab's fundamental modes
  have none: as the strip narrows, TE₀ and TM₀ spread into the oxide and approach its index.
- **The second mode changes.** Up to about 660 nm the second mode is TM₀; wider, TE₁ overtakes
  it, as `jobs/strip-width-sweep.yaml` shows when run. The two have different symmetries, so
  they cross without coupling in a strip with the same oxide above and below.

### How the solve is divided {theory}

The strip is symmetric about its vertical plane and its mid-plane, so each mode's field is even
or odd about each. An electric wall on a plane, where the tangential $\mathbf E$ vanishes, keeps
the modes whose $E_x$ is even about the vertical plane; a magnetic wall, where the tangential
$\mathbf H$ vanishes, those whose $E_x$ is odd. The device solves its TE-like mode on a quarter
of the cross-section, an electric wall on the vertical plane and a magnetic one on the mid-plane
([mirror walls](../docs/methods/walls.md)). The chart solves the four quarters, one for each pair
of walls: TE₀ and TE₂ (electric, magnetic), TE₁ and TE₃ (magnetic, magnetic), TM₀ and TM₂
(magnetic, electric), TM₁ and TM₃ (electric, electric). Each quarter has a quarter of the
unknowns, and a mode's symmetry names it.

A closed window's walls hold the field at zero. Below the oxide's index the window has modes of
its own, standing waves between its walls, which the solver keeps out by asking each mode
whether its index is above the surrounding medium's (`VectorMode::is_guided`, the guided-modes
filter of the [full-vector solver](../docs/methods/vector.md)). Near cutoff a guided mode's tail
reaches the walls, which lower its index: on 2 µm of oxide instead of 1 µm, TE₁ of the 500 nm
strip is 1.50382 instead of 1.50014 (20 nm cells), and its cutoff about 2 nm narrower.

**Try it.** Move the wavelength to 1.31 µm. Up to what width does the strip guide one TE-like
mode, and how many modes does the 500 nm strip guide?

:::answer
TE₁ appears at 335 nm, so the strip is single-mode in TE only below 335 nm; at 500 nm it guides
four modes, TE₀ (2.7125), TM₀ (2.1742), TE₁ (1.8357) and TM₁ (1.5966), on cells about 20 nm
apart. The shorter wavelength sees a larger strip: its width in wavelengths is what counts.
:::

## 4. How does the effective index method reduce a strip to two slabs?

[Hocker and Burns (1977)](https://doi.org/10.1364/AO.16.000113), after Knox and Toulios (1970),
replace the strip by two slabs, as the diagram's right side shows. A slab across the thickness,
oxide, silicon $t$ thick, oxide, gives an effective index $N$ for the strip's middle. A slab across
the width, of index $N$ and width $w$ between the oxide's, gives the mode's $n_\text{eff}$. For a
TE-like mode the first slab is solved as TE and the second as TM: the field that lies along the
first slab's layers lies across the second's.

For the 500 × 220 nm strip of 3.473 in 1.444 the method gives 2.488368 with exact slabs, against
the book's 2.489 from the same two steps (Section 3.2.5):

::validation mode/eim-strip-book

The full-vector index converges to 2.4422 (Section 2): the method is 1.9 % high. The chart below draws
the method's index against the width beside the full-vector one, from Hadley's equations on the
device's cross-section. The method is high at every width, and its error grows as the strip
narrows: +1.9 % at 500 nm, +10.6 % at 300 nm, +0.2 % at 1000 nm (220 nm thick, 1550 nm, the
catalogue's materials).

::chart strip-models

### Where the method fails {theory}

The method assumes the field separates, $H(x, y) = X(x)\thinspace Y(y)$: each column of the strip
has the vertical profile of the first slab, scaled by the lateral profile of the second. Beside the
strip there is no silicon, and the first step's vertical profile, a cosine in the silicon with
short tails, is wrong there: the true field beside the strip spreads up and down into the oxide
much further. The corners, where the two directions meet, are where the assumed product is
furthest from the field. The narrower the strip, the larger the share of its light beside the
core and near the corners, and the larger the error. In the group index, a derivative, the error
is larger still: [effective_index_method](../examples/effective_index_method.rs) measures both
against the full-vector solver on a 6.25 × 5 nm grid:

::example effective_index_method

The method is the device's 2D model of a rib, where the slab beside the rib holds the field as the
method assumes and its error is smaller (Section 9); for a strip, the full-vector solve is the
one to trust.

**Try it.** In the chart, find the width at which the effective index method's error is 5 %.

:::answer
Between 350 and 400 nm: +6.5 % at 350 nm (2.18773 against 2.05332) and +4.1 % at 400 nm
(2.31826 against 2.22705), 220 nm thick at 1550 nm. Below that, the method keeps a guided mode
where the strip's mode is near cutoff: at 250 nm it gives 1.828 where the strip's index is 1.601.
:::

## 5. When is Marcatili's approximation good?

[Marcatili (1969)](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x) divides the cross-section
into the core, the four regions beside its faces, and the four corner regions, and leaves the
corners out: the field there is small for a well guided mode, and without it the field
separates exactly in the five regions left. Each mode then follows from two slab-like equations,
one across the width and one across the height. On the strip in the chart above his index is
within 0.06 % of the full-vector one at 500 nm (2.4438 against 2.4452), closer than the effective
index method; near cutoff he is low, −7.3 % at 300 nm, where the field in the corners he leaves
out is no longer small. His closed form, which approximates the two equations further, is 2.6 %
low at 500 nm.

[marcatili](../examples/marcatili.rs) reproduces his own guide, $a = 2b$ with $n_1 = 1.5$ in a
cladding 1.05 times lower, in his normalized constant: 0 at cutoff, 1 when all the light is in the
core.

::example marcatili

At $B = 3$ and 4 his equations and the full-vector solver agree to $10^{-4}$, as he found against
[Goell's (1969)](https://doi.org/10.1002/j.1538-7305.1969.tb01168.x) computed modes; at $B = 1$,
near cutoff, they differ by $9 \times 10^{-3}$. His closed form is within 4.1 % of his equations
wherever the constant is at least 0.5, his "within a few percent" (p. 2083):

::validation mode/marcatili-vector mode/marcatili-closed-form

### His equations {theory}

In each of the five regions the field is a product of a function of $x$ and a function of $y$,
with transverse wavenumbers $k_x$ and $k_y$ in the core and

$$
\beta^2 = k^2 n_1^2 - k_x^2 - k_y^2 .
$$

Beside the core the field decays, across $x$ with $\xi^{-2} = k^2(n_1^2 - n_2^2) - k_x^2$ and across
$y$ with $\eta^{-2} = k^2(n_1^2 - n_2^2) - k_y^2$. Matching the fields at the four faces, for the
$E^x_{pq}$ modes, $E$ mostly along $x$, in a cladding of index $n_2$ all round (his Eqs. 20–21),

$$
k_x w = p\pi - 2\tan^{-1}\left(\frac{n_2^2}{n_1^2} k_x \xi\right), \qquad
k_y t = q\pi - 2\tan^{-1}\left(k_y \eta\right), \qquad p, q = 1, 2, \ldots
$$

The first is a slab's TM condition across the width, $E_x$ normal to the sidewalls; the second a
slab's TE condition across the thickness, $E_x$ along the top and bottom. They are the slab
lesson's transverse resonance written for a symmetric slab with $p$ and $q$ counted from 1, each
in the core's own index $n_1$, the two directions meeting only in $\beta^2$. The effective index
method uses the same two polarizations in turn, but puts the first slab's index $N$ into the
second. Marcatili's $E^x_{11}$ is the strip's TE₀. `mode::marcatili::Rectangle` solves both
equations exactly by bisection
([Marcatili's approximation](../docs/methods/marcatili.md)), and his closed forms, Eqs. 22–23,
which expand the arctangents for a well guided mode.

**Try it.** In the chart above, where is Marcatili's error largest, and on which side of the
full-vector index is he as the strip widens?

:::answer
Largest near cutoff, at the narrow end: −7.3 % at 300 nm, and no guided mode at all at 250 nm,
where the strip still has one (1.601). He is below the full-vector index up to about 550 nm and
slightly above it from there, by at most $8 \times 10^{-4}$ (0.03 %, at 700 nm), 220 nm thick at
1550 nm. Below, the field in the corners he leaves out is what holds the mode; above, his
separated field slightly overstates the confinement, as the effective index method's does more.
:::

## 6. Why do corners slow convergence, and what do Hadley's equations change?

A finite-difference scheme replaces derivatives by differences of neighbouring values, which is
accurate where the field is smooth. At a corner of a dielectric the field is not: near the
corner it varies as $r^\nu$ with a non-integer exponent, and its derivatives are infinite at the
corner itself. The standard scheme, second order at straight faces, converges at about first
order there. [Hadley (2002)](https://doi.org/10.1109/JLT.2002.800371) built each node's equation
from the exact local solutions of the Helmholtz equation instead, Bessel functions of fractional
order at a corner, and reached about second order with the same unknowns.

The chart solves the strip's TE₀ on cells from about 40 to 10 nm with both. Hadley's equations
approach a limit smoothly, at order 2.2 over the three finest grids; extrapolated, 2.44507 for
the 500 × 220 nm strip with the catalogue's materials. The standard scheme wanders: its error
changes from grid to grid by how the cells meet the corners, and on 10 nm cells it is still
$3.5 \times 10^{-3}$ above the limit, five times Hadley's error on 39 nm cells.

::chart strip-corners

### The field at a corner {theory}

Near a corner where four quadrants of permittivities $\varepsilon_1$ to $\varepsilon_4$ meet,
counterclockwise, the field's local solutions include $r^\nu$ terms whose exponents solve Hadley's
Eq. 36,

$$
\sin^2\frac{\pi\nu}{2} = -\frac{1}{4}\left(\varepsilon_{12} + \varepsilon_{34}\right)\left(\varepsilon_{23} + \varepsilon_{41}\right),
\qquad \varepsilon_{ij} = \frac{\varepsilon_i - \varepsilon_j}{\varepsilon_i + \varepsilon_j} .
$$

At the strip's corner one quadrant is silicon and three are oxide, so $\varepsilon_{34} = \varepsilon_{23} = 0$
and $\varepsilon_{41} = -\varepsilon_{12}$: $\sin(\pi\nu/2) = \varepsilon_{12}/2$. With
$\varepsilon_1 = 12.08$ and $\varepsilon_2 = 2.085$, $\varepsilon_{12} = 0.7056$ and the two roots in Hadley's
interval are $\nu = 1.77$ and $\mu = 2.23$. The magnetic field goes as $r^{1.77}$ near the corner:
continuous, with a continuous first derivative, but a second derivative that grows as
$r^{-0.23}$. A Taylor series at a node next to the corner misses that term, and its error does not
fall as $h^2$. Hadley's equations include the $r^\nu$ and $r^\mu$ terms and their logarithmic
companions in the local solution, so their stencils are exact for them
([high-accuracy modes](../docs/methods/hadley.md)).

Hadley's four corner problems, boxes and impinged corners of $\varepsilon = 2.25$ and 8 known to
$10^{-8}$ by series expansions, test both schemes:

::example hadley_corners

The validation report holds the standard scheme to $10^{-4}$ on an 80 × 80 grid and Hadley's
equations to $10^{-6}$ on 128 × 128, with their order, and the device's own 3D model, by Hadley's
equations, at order 1.92 on the 500 × 220 nm strip from 20 to 5 nm cells:

::validation mode/hadley-box-low mode/hadley-box-high mode/hadley-corners-box-low mode/hadley-corners-box-high mode/hadley-corners-order device/waveguide-3d-order

On the strip of Section 2, 3.473 in 1.444, Hadley's equations give 2.442378, 2.442219 and
2.442179 at 20, 10 and 5 nm, where the standard scheme gives 2.447067, 2.445713 and 2.444396: on a
20 nm grid they are already closer to the limit than the standard scheme on a 2.5 nm one, with a
sixty-fourth of the unknowns.

**Try it.** Narrow the strip to 300 nm. Do the two schemes come closer or move apart?

:::answer
They move apart. At 300 nm the standard scheme's 10 nm grid is 0.019 above the extrapolated
1.82632, five times its error at 500 nm, while Hadley's equations converge at order 2.7 and are
within $3.2 \times 10^{-3}$ even on 37 nm cells. The narrower strip holds more of its light near
its corners, where the standard scheme's error comes from.
:::

## 7. What is the group index, and why is it larger than the effective index?

The effective index sets the phase velocity, $c/n_\text{eff}$. A pulse, a ring's free spectral
range and an interferometer's fringes go by the group velocity, $c/n_g$, with Chrostowski and
Hochberg's Eq. 3.5,

$$
n_g = n_\text{eff} - \lambda \frac{dn_\text{eff}}{d\lambda} .
$$

For the 500 × 220 nm strip at 1550 nm $n_\text{eff} = 2.445$ and $n_g = 4.19$: the group
index is 1.7 higher than the effective index, and 0.6 higher than bulk silicon's own, 3.599. The
chart draws both against the width, from the strip waveguide device's 3D model, Hadley's
equations at five wavelengths 25 nm apart with the materials' dispersion. On the chart's 50 nm
steps the group index peaks at 350 nm, 4.42, and falls on both sides: wider, the mode is held in
the silicon and changes less with the wavelength; narrower, it spreads into the oxide, whose index
it approaches.

::chart strip-group-index

### The materials' part and the guide's {theory}

$n_\text{eff}$ depends on the wavelength directly, through the size of the strip in wavelengths,
and through the indices of the materials, $n_i(\lambda)$. By the chain rule,

$$
n_g = n_\text{eff} - \lambda \left.\frac{\partial n_\text{eff}}{\partial \lambda}\right|_{n_i}
- \lambda \sum_i \frac{\partial n_\text{eff}}{\partial n_i} \frac{dn_i}{d\lambda} .
$$

The second term is the guide's dispersion: at fixed indices a longer wavelength sees a smaller
strip, more of the mode is in the oxide, and $n_\text{eff}$ falls. The third is the materials':
silicon's index falls with the wavelength too. Both make $n_g$ larger than $n_\text{eff}$. The two
examples on the same 6.25 × 5 nm grid separate them for the 500 × 220 nm strip: with fixed indices,
3.473 and 1.444, [effective_index_method](../examples/effective_index_method.rs) finds
$n_g = 4.040$ for the full-vector mode, the guide's part alone; with the book's dispersive silicon,
[group_index](../examples/group_index.rs) finds 4.173, the oxide held at 1.444 in both. Of the
1.73 by which $n_g$ exceeds $n_\text{eff}$ (2.4451), the guide's dispersion gives 1.60 and
silicon's 0.13.

[group_index](../examples/group_index.rs) reproduces Chrostowski and Hochberg's Fig. 3.22b, the
group index of 220 nm strips 400 to 600 nm wide, with their silicon (Listing 3.1) and oxide:

::example group_index

::validation mode/strip-group-index-book

Dispersion, the change of $n_g$ with the wavelength, is
$D = -(\lambda/c)\thinspace d^2 n_\text{eff}/d\lambda^2$ (their Eq. 3.6), in ps/(nm km). The
device's 3D model gives $D = 469$ ps/(nm km) for the 500 × 220 nm strip at 1550 nm (cells about
20 nm apart): positive, anomalous, the regime [Turner et al. (2006)](https://doi.org/10.1364/OE.14.004357)
measured in silicon strips and tuned from −2000 to 1000 ps/(nm km) with the cross-section.

### Measured wires {theory}

[Dwivedi et al. (2015)](https://doi.org/10.1109/JLT.2015.2476603) measured the effective and group
indices of fabricated wires 470, 602 and 805 nm wide, 211 nm thick by their SEM cross-sections,
with interferometers. [mzi_dwivedi](../examples/mzi_dwivedi.rs) solves each measured
cross-section by Hadley's equations with Li's silicon and Malitson's silica, designs the
interferometers, and reads their computed spectra as the paper read its measured ones:

::example mzi_dwivedi

The 602 nm wire through the strip waveguide device: $n_g$ 4.0385 against the measured
4.0453 ± 0.0045.

::validation device/wire-ng-dwivedi

**Try it.** In the group index chart, what does the effective index method give for the 500 nm
strip, and why is it low?

:::answer
4.024, 3.9 % below the full-vector 4.189, the materials' dispersion included in both. The
method's index is too high (Section 4): the separated field it assumes is more confined than the
strip's, a more confined mode's index changes less with the wavelength, and the guide's part of
$n_g$, the term $-\lambda\thinspace\partial n_\text{eff}/\partial\lambda$, comes out too small.
:::

## 8. How does a strip on a thin oxide leak into the substrate?

Under the buried oxide is the silicon substrate, whose index, 3.48, is above any strip mode's.
The mode's field decays across the oxide, and what reaches the substrate is a wave travelling
away: the mode is leaky. Its effective index has an imaginary part, and its power falls as
$e^{-2k \operatorname{Im}(n_\text{eff}) z}$. A closed window can't hold such a wave; a perfectly
matched layer in the substrate absorbs it ([the PML](../docs/methods/pml.md)), and the solver
then finds the leaky mode with its loss
([Hu and Menyuk, 2009](https://doi.org/10.1364/AOP.1.000058), on what a leaky mode is).

[Bienstman et al. (2006)](https://doi.org/10.1007/s11082-006-9025-9) compared many mode solvers on
one such wire: 500 × 220 nm of silicon (3.5) on 1 µm of oxide (1.45) on silicon, under air, at
1.55 µm. Its consensus is $2.412372 + 2.9135 \times 10^{-8} i$, a loss of
$10\log_{10}(e) \cdot 2k\operatorname{Im}(n_\text{eff}) = 0.0103$ dB/cm.
[leaky_wire_benchmark](../examples/leaky_wire_benchmark.rs) solves it with a PML in the substrate
on core grids of 5, 2.5 and 1.25 nm, and extrapolates:

::example leaky_wire_benchmark

The raw indices converge at order 0.67, the corners' (silicon against air is the highest
contrast here, and the standard scheme the one that takes a PML). Richardson's extrapolation
assumes that order and cancels the leading error, landing $8 \times 10^{-5}$ from the consensus in
the real part and 0.25 % in the loss:

::validation mode/leaky-wire-bienstman mode/leaky-wire-bienstman-loss

A modes job solves the same wire: `jobs/leaky-wire.yaml` gives its stack fixed indices, puts 1 µm
of PML in the substrate with `[task.pml]`, and aims its count at the wire with `modes_near`, since
the modes nearest the substrate's index are the PML's own. Each mode it keeps is named guided or
leaky, with its loss in dB/cm. On its uniform 5 nm grid the job is off by what the solver's own
5 nm grid gives before extrapolation, $+4.0 \times 10^{-3}$ in the real part and 0.963 of the loss:

::validation job/leaky-wire-bienstman job/leaky-wire-bienstman-loss

The leakage falls about exponentially with the oxide's thickness $h$: the field decays across
the oxide as $e^{-\gamma y}$, $\gamma = k\sqrt{n_\text{eff}^2 - n_2^2} = 7.8$ /µm for this wire, so
each further micrometre of oxide divides the power reaching the substrate by about
$e^{2\gamma \cdot 1\thinspace\mu\text{m}} \approx 6 \times 10^{6}$. On the standard wafer's 2 µm the
estimate puts this wire's leakage near $2 \times 10^{-9}$ dB/cm, which is why the device and these
charts solve the strip with oxide below it and no substrate. The lesson on
[leaky and bent guides](leaky-and-bent-guides.md) takes leaky waves further.

## 9. What changes in a rib, and with sloped or rough sidewalls?

A **rib** is a strip etched part of the way through the film, a slab of silicon left beside it.
The slab beside the rib has its own mode, of index $n_\text{slab}$; a rib mode is guided while
its index is above it, and below, it leaks sideways into the slab.
[Soref, Schmidtchen and Petermann (1991)](https://doi.org/10.1109/3.83406) used that leakage to
make ribs several microns wide and high single-mode: their higher modes leak into the slab within
a few millimetres while the fundamental is held. The effective index method suits a rib better
than a strip, the slab beside it holding the field up and down as the method's first step
assumes; the rib waveguide
device takes it as its 2D model, within 0.07 of the 3D index over its stated range. Its 3D model
reproduces [Hadley's (2002)](https://doi.org/10.1109/JLT.2002.800371) Table I, a 3 µm rib on slabs
of 0 to 0.7 µm, to $1.4 \times 10^{-6}$:

::example rib_hadley

::validation device/rib-hadley device/rib-eim device/rib-wide-slab

::device rib-waveguide

**Sloped sidewalls** come from the etch: the strip waveguide and rib devices take the walls'
angle (`sidewall`, 90° vertical) and solve the cells a wall crosses with each material's share of
them, by the standard scheme. **Rough sidewalls** scatter light out of the guide, and set most of
a silicon strip's loss: the mode solvers don't give it. [Lee et al. (2001)](https://doi.org/10.1364/OL.26.001888)
smoothed the walls of a 0.5 µm strip and took its loss from 32 to 0.8 dB/cm.
[Vlasov and McNab (2004)](https://doi.org/10.1364/OPEX.12.001622) measured 3.6 dB/cm on strips
from a 200 mm CMOS line, and [Dumon et al. (2004)](https://doi.org/10.1109/LPT.2004.826025)
2.4 dB/cm on wires patterned by deep-UV lithography. The devices take the loss as a parameter,
in dB/cm, 2.7 by default.

## 10. How does photonoxide solve a strip, and what does the device compute?

`mode::vector::modes` takes a `CrossSection`, the permittivity of each cell between the nodes of a
rectilinear grid, and returns its modes nearest a guess: the eigenproblem for $\beta^2$ of
[Fallahkhair, Li and Murphy (2008)](https://doi.org/10.1109/JLT.2008.923643), solved by
shift-and-invert Arnoldi restarted by Krylov–Schur ([eigensolvers](../docs/methods/eigen.md)).
Each mode gives its effective index, its TE fraction, its six field components
(`VectorMode::fields`) and whether it is guided. Mirror walls halve or quarter the problem, and a
PML opens the window for leaky modes. `mode::hadley::modes` takes the same cross-section and
returns the same modes by Hadley's equations, starting from the standard scheme's and iterating
the eigenproblem, which is nonlinear in $\beta^2$ there, to convergence
([high-accuracy modes](../docs/methods/hadley.md)). The standard scheme is checked against the
exact slab, a cross-section uniform along one axis, in both polarizations, at second order:

::validation mode/vector-slab-limit-te mode/vector-slab-limit-tm

The **strip waveguide device** puts a strip on a stack, 220 nm SOI by default, with its width,
thickness, sidewall, slab, length, loss and band as parameters, and computes it at two
fidelities ([devices](../docs/methods/devices.md)):

- **Analytic:** $e^{i\gamma L}$ on a fit of $n_\text{eff}$ and $n_g$ at 1.55 µm, from tables of the 3D
  model's values over widths of 400 to 600 nm and thicknesses of 200 to 240 nm, at no cost; it
  refuses a design outside its tables, a rib or a sloped wall.
- **3D:** the full-vector solver's TE-like mode on the device's quarter, cells fitted to the
  strip about 10 nm apart with 1 µm of oxide beyond it, by Hadley's equations, at five
  wavelengths across the band: $n_\text{eff}$, $n_g$ and $D$, each figure with its grid.

The analytic fit is held to the 3D model at designs between its tables' nodes, within $10^{-4}$ in
$n_\text{eff}$ and 0.02 in $n_g$:

::validation device/waveguide-fit-neff device/waveguide-fit-ng

Every full-vector curve in this lesson's charts solves the device's own cross-section, on cells
about 20 nm apart for the sweeps and 10 nm for the fields, so the lesson and the device's page
agree; the group index chart's curve is the device's 3D model, and Section 4's chart draws the
analytic fit where it holds.

::device waveguide

## 11. History and current research {research}

The rectangular dielectric guide was solved twice in the same issue of the Bell System Technical
Journal in 1969, the one in which [Miller](https://doi.org/10.1002/j.1538-7305.1969.tb01165.x)
proposed integrated optics. [Marcatili](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x) left
out the corners and found closed forms; [Goell](https://doi.org/10.1002/j.1538-7305.1969.tb01168.x)
kept them, expanding the field in circular harmonics and computing the modes, the curves Marcatili
checked his against. [Hocker and Burns (1977)](https://doi.org/10.1364/AO.16.000113) gave the
effective index method its standard form, after Knox and Toulios. Finite-difference mode solvers
for the full vector fields followed, and
[Fallahkhair, Li and Murphy (2008)](https://doi.org/10.1109/JLT.2008.923643) wrote the scheme for
anisotropic media that photonoxide's solver implements;
[Hadley (2002)](https://doi.org/10.1109/JLT.2002.800361) had shown how to make such schemes
high-order at interfaces and at [corners](https://doi.org/10.1109/JLT.2002.800371).
[Bienstman et al. (2006)](https://doi.org/10.1007/s11082-006-9025-9) set the solvers a common
benchmark, a leaky silicon wire.

Silicon strips became practical as their loss fell.
[Soref, Schmidtchen and Petermann (1991)](https://doi.org/10.1109/3.83406) made silicon guides
single-mode with large ribs; submicron strips scattered light at their sidewalls, until
[Lee et al. (2001)](https://doi.org/10.1364/OL.26.001888) smoothed them to 0.8 dB/cm.
[Vlasov and McNab (2004)](https://doi.org/10.1364/OPEX.12.001622) and
[Dumon et al. (2004)](https://doi.org/10.1109/LPT.2004.826025) made low-loss single-mode wires
with CMOS tools, and [Bogaerts et al. (2005)](https://doi.org/10.1109/JLT.2004.834471) showed that
wires lose more than an order of magnitude less than photonic-crystal guides made the same way,
0.24 against 7.5 dB/mm.
[Turner et al. (2006)](https://doi.org/10.1364/OE.14.004357) shaped a strip's dispersion through
its cross-section.

::timeline

Today the strip is a foundry's standard device: process design kits carry it in their libraries
of tested elements, beside the couplers, rings and modulators built from it
([Siew et al., 2021](https://doi.org/10.1109/JLT.2021.3066203)). Its indices are measured on
fabricated wires, as [Dwivedi et al. (2015)](https://doi.org/10.1109/JLT.2015.2476603) did with
interferometers, and photonoxide's solve of their SEM cross-sections agrees with their group
indices within the tolerance their cross-sections' uncertainty sets (Section 7). The approximations stay in use where speed
matters more than the last per cent: the effective index method for ribs and for 2D models of
whole devices, Marcatili's equations for a quick estimate far from cutoff.

Further reading: Chrostowski and Hochberg's Sections 3.2.5–3.2.9 for the strip and its
dispersion; [Marcatili (1969)](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x) for the
separated fields; [Hadley (2002)](https://doi.org/10.1109/JLT.2002.800371) for what corners do to
a grid; and [Fallahkhair et al. (2008)](https://doi.org/10.1109/JLT.2008.923643) for the
full-vector scheme.

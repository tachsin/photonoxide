---
title: "Crossings, mode converters and the polarization splitter-rotator"
summary: "Devices that steer modes rather than split power: why a plain crossing loses a fifth of its light and how widened arms and a thin slab keep it, when a taper is adiabatic and which profile is shortest, how an asymmetric coupler turns TE₀ into TE₁, how a nitride cladding lets a taper turn TM₀ into TE₁, and how an inverse taper meets a fibre, each worked out with photonoxide's mode solver, eigenmode expansion and FDFD and checked against 3D FDTD and the published numbers."
topic: Multimode and polarization devices
level: advanced
minutes: 55
prerequisites: [mmi, directional-coupler]
examples: [crossing_liu_poon, mode_converter_liu_poon, splitter_rotator_liu_poon]
jobs: [eme-taper.yaml, eme-taper-3d.yaml]
methods: [eim.md, eme.md, fdfd.md, vector.md, fdtd.md]
validation:
  - eme/taper-fdfd
  - eme/3d-cut-taper-fdfd
charts: [crossing-field, crossing-width, mmi-field, taper-length, converter-match, converter-coupler, rotator-hybrid, spot-size]
papers:
  - cite: "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977)"
    title: "Mode dispersion in diffused channel waveguides by the effective index method"
    doi: 10.1364/AO.16.000113
    year: 1977
    role: milestone
    note: "The effective index method reviewed and applied to rectangular guides: the step that turns a crossing or a taper on a chip into the plane its first charts here are computed in."
  - cite: "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995)"
    title: "Optical multi-mode interference devices based on self-imaging: principles and applications"
    doi: 10.1109/50.372474
    year: 1995
    role: review
    note: "Self-imaging in a multimode guide: a centre-fed section images its input once at 3L_π/4 and twice midway (their Eqs. 34–37), the refocusing an MMI crossing places its intersection on."
  - cite: "P. Bienstman, R. Baets, Opt. Quantum Electron. 33, 327 (2001)"
    title: "Optical modelling of photonic crystals and VCSELs using eigenmode expansion and perfectly matched layers"
    doi: 10.1023/A:1010882531238
    year: 2001
    role: milestone
    note: "Eigenmode expansion with perfectly matched layers: a structure cut into uniform sections, the field in each its modes, the sections joined by their S-matrices, the method photonoxide's crossing and taper charts use."
  - cite: "D. Dai, J. E. Bowers, Opt. Express 19, 18614 (2011)"
    title: "Novel ultra-short and ultra-broadband polarization beam splitter based on a bent directional coupler"
    doi: 10.1364/OE.19.018614
    year: 2011
    role: milestone
    note: "A polarization splitter from a bent coupler of two silicon wires of different widths, TM phase matched and crossing, TE mismatched and staying: a coupling region 4.5 µm long and a 200 nm gap, about 200 nm of band at an extinction ratio above 10 dB."
  - cite: "D. Dai, J. Wang, Y. Shi, Opt. Lett. 38, 1422 (2013)"
    title: "Silicon mode (de)multiplexer enabling high capacity photonic networks-on-chip with a single-wavelength-carrier light"
    doi: 10.1364/OL.38.001422
    year: 2013
    role: milestone
    note: "Cascaded asymmetric couplers that turn a 0.4 µm guide's TM₀ into TM₁, TM₂ and TM₃ of a bus 1.02, 1.68 and 2.35 µm wide: under 0.5 dB of measured excess loss and crosstalk below −23 dB over 20 nm."
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    title: "Silicon Photonics Design"
    doi: 10.1017/CBO9781316084168
    year: 2015
    role: review
    note: "Section 3.2.8: a 220 nm strip's TE₁ and TM₀ cross near 680 nm of width. Section 5.3.1: a 180 nm nano-taper's mode (n_eff 1.46) against Gaussian beams, best at a numerical aperture of 0.4 for TE, and a taper at least 100 µm long."
  - cite: "B. Shen, P. Wang, R. Polson, R. Menon, Nat. Photonics 9, 378 (published online 18 May 2015)"
    title: "An integrated-nanophotonics polarization beamsplitter with 2.4 × 2.4 µm² footprint"
    doi: 10.1038/nphoton.2015.80
    year: 2015
    role: current
    note: "A polarization splitter of 20 × 20 silicon pixels 120 nm square, shaped by a nonlinear optimization: over 70 % average transmission and an extinction ratio above 10 dB across 32 nm, measured."
  - cite: "Z. Liu, J. K. S. Poon, Opt. Continuum 4, 2427 (2025)"
    title: "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD simulations of passive silicon photonic components"
    doi: 10.1364/OPTCON.572107
    year: 2025
    role: current
    note: "Two commercial 3D FDTD codes on gdsfactory's crossing, mode converter and polarization splitter-rotator among others: the through, TE₁ and TE₀ ports and their crosstalk against the resolution and over the band, the numbers this lesson's three examples are checked against."
---

The three devices of this lesson from above, in the symbols the equations below use: a crossing
whose arms widen from $w$ to $W$ over tapers $L_t$ long, over two ellipses of thinner slab; an
asymmetric coupler whose narrow guide's TE₀ crosses into the wide guide's TE₁; and a
splitter-rotator whose taper, under silicon nitride, turns TM₀ into TE₁ before a coupler passes it
on.

::diagram crossing

A splitter divides power among guides. The devices here move light between modes instead: a
crossing keeps a guide's fundamental mode intact through a hole in its walls, a mode converter hands
one guide's TE₀ to another guide's TE₁, a polarization splitter-rotator turns TM₀ into TE₀ on
another guide, and an inverse taper widens a silicon mode until it matches a fibre's. Each works
by keeping a mode's phase or shape under control along a structure that changes, and each is
judged by how much light reaches the one mode it should and how little reaches the others.

## 1. What do crossings and mode converters do, and how are they rated?

A device's ports carry modes, and its figures are shares of the input power that reach a given
mode at a given port. Liu and Poon rate gdsfactory's components so
([Liu and Poon, 2025](https://doi.org/10.1364/OPTCON.572107), Sections 3.2 to 3.5):

- the **transmission**: the share that reaches the mode the device is for, the through port's
  TE₀ of a crossing, the cross port's TE₁ of a mode converter, the upper port's TE₀ of a
  splitter-rotator;
- the **excess loss** of a crossing: the share that reaches the TE₀ of none of its three output
  ports, in dB;
- the **mode crosstalk**: the share in the mode the device should have left empty, the cross
  port's TE₀ of a mode converter or the upper port's TM₀ of a splitter-rotator, in dB.

A share $T$ in decibels is $10\log_{10}T$: 0.955 is −0.20 dB, 0.5 is −3.01 dB and 10⁻⁴ is −40 dB.
A crossing's crosstalk is the share in a side port's TE₀. [Dai, Wang and Shi (2013)](https://doi.org/10.1364/OL.38.001422)
define a multiplexer's crosstalk against the wanted port instead,
$\mathrm{CT} = 10\log_{10}(P_i/P_0)$ with $P_0$ the wanted port's measured power and $P_i$ another
port's, which differs from the share by the wanted port's own loss.

photonoxide computes these devices three ways: the guides' modes by its
[full-vector solver](../docs/methods/vector.md); a device on the plane of
[the effective index method](../docs/methods/eim.md) by
[eigenmode expansion](../docs/methods/eme.md) and [2D FDFD](../docs/methods/fdfd.md); and the whole
device in [3D FDTD](../docs/methods/fdtd.md), the three examples Liu and Poon's paper is the check
of.

## 2. Why does a plain crossing lose light?

Where two guides cross, each runs for a guide's width with no walls: the other guide's core
continues to either side. The mode, its walls gone, spreads as a free beam does, and on the far
side only the part that still matches the guide's mode is guided on. The rest radiates, much of it
into the other guide's arms.

The chart below is a crossing of two 500 nm guides on the plane of the effective index method
([Hocker and Burns, 1977](https://doi.org/10.1364/AO.16.000113)): the core the TE₀ index of
220 nm of silicon in silica, 2.8475 at 1.55 µm, the silica 1.4440. In this plane the TE-like
mode's E lies across the guide, so 2D FDFD solves for $H_z$. 0.7955 of the power leaves in the
through port's TE₀, −0.99 dB; 0.084 crosses into each side arm; 0.0047 comes back.

::chart crossing-field{kind=1}

### A beam across a gap {theory}

Model the mode as a Gaussian beam of waist $w_0$, $u(x, 0) = e^{-x^2/w_0^2}$, in a medium of index
$n$. In the paraxial approximation it travels as

$$
u(x, z) \propto \frac{1}{\sqrt{q(z)}}\, \exp\!\left(\frac{ikx^2}{2q(z)}\right), \qquad q(z) = z - iz_R, \qquad z_R = \frac{\pi n w_0^2}{\lambda},
$$

which at $z = 0$ is the waist ($ik/(2q) = -k/(2z_R) = -1/w_0^2$, $k = 2\pi n/\lambda$). The share
of its power still in the waist's mode a distance $d$ on is the overlap
$\lvert\int u(x,0)^* u(x,d)\,dx\rvert^2/\int\lvert u(x,0)\rvert^2dx\int\lvert u(x,d)\rvert^2dx$.
The integral is a Gaussian's, $\int e^{-ax^2}dx = \sqrt{\pi/a}$ with
$a = 1/w_0^2 - ik/(2q(d)) = (k/2)(q - iz_R)/(z_R q)$, and after the normalizations cancel,

$$
\eta(d) = \left\lvert\frac{2z_R}{2z_R + i d}\right\rvert = \left[1 + \left(\frac{d}{2z_R}\right)^2\right]^{-1/2} .
$$

This is the plane's one transverse dimension; a beam spreading in two loses the square of it. A
plain crossing leaves the beam unguided for $d = w$, the other guide's width. The best Gaussian for
the 500 nm guide's mode (overlap 0.9990) has $w_0 = 0.275$ µm, so $z_R = 0.436$ µm and
$\eta(0.5\,\mu\mathrm{m}) = 0.868$. The exact answer, 0.7955, is lower: the mode isn't a
Gaussian, and the gap's edges scatter it as well as let it spread. The estimate gives the scaling:
the loss falls as $z_R$ grows, with the square of the beam's waist.

## 3. How do widened arms and a slab keep the beam on course?

Widen the arms before the centre and the beam crosses with a larger waist: the Rayleigh length
grows as $w_0^2$ while the gap grows as $W$. gdsfactory's crossing widens its 500 nm arms to
1.2 µm over tapers 3.4 µm long either side of a 1.2 µm square, and lays under the centre two
ellipses of 150 nm slab, semi-axes 3 and 1.1 µm, one along each arm (Liu and Poon, Section 3.2).
In the plane the slab is a region of index 2.5391, the TE₀ index of 150 nm of silicon.

::chart crossing-field

With the arms alone (crossing 2), 0.8368 passes: the wider beam spreads less and 0.0136 reaches
each side arm's end, but 0.14 leaves the through arm's TE₀ for radiation and its other modes. With
the ellipses (crossing 3), 0.9480 passes: the slab around the centre keeps guiding the beam where
the arm's walls are open, and only 0.0064 reaches each side arm's end. 2D FDFD on 25 nm cells
and eigenmode expansion of the same crossing agree within three times the larger of their own
errors (the expansion 0.7999, 0.8379 and 0.9452 against FDFD's 0.7955, 0.8368 and
0.9480; FDFD's limit from 50 and 25 nm cells, by Richardson, 0.7973 and 0.9468 for the first and
the last).

### The expansion of a crossing {theory}

Eigenmode expansion cuts a structure along $z$ into sections uniform along $z$, writes the field in
each as that section's modes going both ways, and joins the sections by matching the tangential
fields at each interface, the sections' S-matrices combined by the star product
([Bienstman and Baets, 2001](https://doi.org/10.1023/A:1010882531238); [the method](../docs/methods/eme.md)).
A crossing fits it exactly: across the through arm, the side arm is core from one side of the
window to the other over the length where it crosses, so each section is a stack of core, slab and
silica, and the light the side arm takes runs off into the window's perfectly matched layers. The
chart's expansion keeps 40 modes in a half window of 2 µm inside a PML 4 µm thick, its steps at
most 0.1 µm long and broken at every kink of the outline; its own error, from 40 to 50 modes and
from steps of 0.1 to 0.05 µm, is under 2 × 10⁻³.

The chart below sweeps the centre's width $W$ with the tapers fixed. Without the ellipses the
through port rises from the plain cross's 0.7999 to 0.847 at 1.3 µm and falls beyond it; the
dashed Gaussian estimate,
$\eta(W)$ with $w_0$ fitted to the $W$-wide guide's mode, rises steadily (0.926 at 1.2 µm, where
$w_0 = 0.505$ µm and $z_R = 1.47$ µm) and misses that. With the ellipses the through port peaks at
0.986 near $W = 1.0$ µm and is 0.947 at gdsfactory's 1.2 µm, in this plane. At these tapers the
expansion is within 0.01 of 2D FDFD at every width tried (0.8209 against 0.8304 at 1.4 µm with the
ellipses, the largest); with tapers much shorter than 3 µm and wide centres its window of 2 µm no
longer holds the light, and the chart starts its tapers at 3 µm.

::chart crossing-width

**Try it.** Set $W$ to 1.6 µm, then lengthen the tapers to 6 µm. What happens to the through port
of the widened arms alone?

:::answer
It rises from 0.816 to 0.874: a longer taper widens the mode more gently (Section 5), so more of
the light reaches the centre in the wide guide's fundamental mode. With the ellipses it rises from
0.685 to 0.764, still far below its 0.986 near 1 µm in this plane.
:::

### Multimode crossings and the slab {research}

A wider centre is a short multimode section, and self-imaging can be put to use there: a multimode
section fed at its centre by a symmetric field images it once at $3L_\pi/4$ and twice halfway
([Soldano and Pennings, 1995](https://doi.org/10.1109/50.372474), Eqs. 34–37), so between its ends
the light spreads across the width and comes back to a single spot. A crossing of two such sections
puts the intersection where the light is back in one spot, its field small at the walls the other
section opens; the chart of [the MMI lesson](mmi.md) shows a centre-fed section's light refocusing:

::chart mmi-field{width=2.5um, offset=0, span=1}

[Chrostowski and Hochberg (2015)](https://doi.org/10.1017/CBO9781316084168) cite Bogaerts et al.'s
low-loss, low-crosstalk crossings for silicon wires (their ref. 49), a paper not yet read for this
lesson; photonoxide has no MMI crossing among its devices. gdsfactory's crossing guides the beam
by a partial etch instead, its 150 nm slab, the two etch depths drawn as two layers. Nor does the
library have periodic arrays of crossings.

## 4. What does gdsfactory's crossing pass in 3D?

[crossing_liu_poon](../examples/crossing_liu_poon.rs) runs gdsfactory's crossing in photonoxide's
3D FDTD: 220 nm of silicon for the arms, 150 nm for the ellipses, silica above and below, TE₀ into
the left arm. On CI's grid, 6 cells a wavelength in silicon (74 nm cells), the through port's TE₀
is 0.95227 at 1550 nm with an excess loss of −0.212 dB, and each side port's TE₀ is
1.4 × 10⁻⁶ and 1.2 × 10⁻⁶, −59 dB:

::example crossing_liu_poon

Liu and Poon's two codes settle at 0.957 (Lumerical) and between 0.9558 and 0.959 (Tidy3D) from
15 to 25 cells a wavelength, and print 95.4 % and 93.9 % at 6 cells (their Fig. 8(a)). photonoxide's
runs at 15 and 20 cells give 0.95666 and 0.95863, within both codes' spread, and with the coupling
tensor in place of its diagonal 0.95820 and 0.95944, which extrapolate, taking the error as $h^2$,
to about 0.961; the excess loss at 15 cells, −0.192 dB, is within the codes' −0.183 to −0.197 dB
(the example's notes). The plane of the effective index method gives 0.947 to 0.948 for the
same crossing, about 0.01 below 3D. That is the method's approximation, not a grid's: the plane's
two solvers agree to 0.002, and the method replaces each vertical cross-section, full film or slab,
by one index.

## 5. When is a taper adiabatic?

A taper widens a guide. If it widens slowly, the light stays in the local fundamental mode at each
point and arrives in the wide guide's fundamental mode; if quickly, part of it leaves for higher
modes and radiation. The abrupt junction is the limit: a 500 nm guide butted onto a 3 µm one in the
plane passes 0.4157 of its power into the wide guide's TE₀ by eigenmode expansion, near the overlap
of the two modes, 0.4316, which leaves out reflection and the light the other modes send back.

The chart below draws the TE₀ that reaches the wide guide against the taper's length for three
profiles from 500 nm to $w_2$ (the plane's eigenmode expansion, 40 modes, steps at most 0.25 µm
long). From 500 nm to 3 µm, 99 % of the power arrives from 17.3 µm for a linear taper, 11.0 µm for
a parabolic one and 34.8 µm for an exponential one.

::chart taper-length

### Coupled local modes {theory}

Take the scalar wave equation of the plane, $\partial_x^2\psi + \partial_z^2\psi + k_0^2 n^2(x, z)\psi = 0$,
and at each $z$ the local modes of the cross-section there,
$\partial_x^2\phi_m + k_0^2 n^2\phi_m = \beta_m^2\phi_m$, normalized to $\int\phi_m^2dx = 1$. Write
the field as $\psi = \sum_m a_m(z)\phi_m(x; z)$. Differentiating the local eigenvalue equation along
$z$, multiplying by $\phi_m$ and integrating, the operator's symmetry leaves, for $m \ne n$,

$$
C_{mn} \equiv \int \phi_m\,\partial_z\phi_n\,dx = \frac{k_0^2}{\beta_n^2 - \beta_m^2}\int \phi_m\phi_n\,\partial_z\!\left(n^2\right)dx .
$$

The amplitudes of the forward waves then obey, to first order in $C$,
$da_m/dz = i\beta_m a_m - \sum_n C_{mn}a_n$: the
local modes are coupled only through how fast the cross-section changes. Launched in mode 0, the
amplitude reaching mode $m$ is
$a_m(L) \approx -\int_0^L C_{m0}\exp\!\left(i\int_0^z(\beta_0 - \beta_m)\,dz'\right)dz$. Where $C_{m0}$
changes slowly over a beat length $2\pi/(\beta_0 - \beta_m)$, the integral oscillates and stays of
the order of $C_{m0}/(\beta_0 - \beta_m)$, so the taper is adiabatic where

$$
\frac{\lvert C_{m0}\rvert}{\lvert\beta_0 - \beta_m\rvert} \ll 1 .
$$

A symmetric taper couples TE₀ only to even modes, TE₂ first. For a step-index guide of width $w(z)$
whose two faces move at $w'/2$, $\partial_z(n^2) = (n_1^2 - n_2^2)\,(w'/2)\,[\delta(x - w/2) + \delta(x + w/2)]$,
and for two even modes

$$
C_{20} = \frac{k_0^2\,(n_1^2 - n_2^2)\,w'\,\phi_0(w/2)\,\phi_2(w/2)}{\beta_0^2 - \beta_2^2} .
$$

The chart's figures give the largest $\lvert C_{20}\rvert/(\beta_0 - \beta_2)$ along each taper at
the marked length, from the exact slab modes (the plane's TE, scalar; the chart's light is the
plane's TM, which weights the same integrals by $1/n^2$): at 10 µm, 0.17 for the linear taper, 0.10
for the parabolic and 0.37 for the exponential, the order in which they need length.

## 6. Which taper profile is shortest?

The parabolic one, for widening a guide well beyond single mode, and the criterion says why. For a
guide wide enough to hold its modes well, the lateral wavenumber of mode $m$ is near $(m + 1)\pi/w$,
so

$$
\beta_m \approx k_0 n_1 - \frac{(m + 1)^2\pi^2}{2k_0 n_1 w^2}, \qquad \beta_0 - \beta_2 \approx \frac{4\pi^2}{k_0 n_1 w^2} :
$$

the beat between TE₀ and TE₂ slows as the guide widens, while $C_{20}$, a derivative of the mode's
shape, scales as $w'/w$. Their ratio scales as $w'w$, so a taper keeps the same adiabaticity along
its length when $w'w$ is constant, $(w^2)' = \mathrm{const}$: the width squared growing linearly,
the parabolic taper. A linear taper widens as fast at its wide end, where the modes beat slowly, as
at its narrow end; an exponential one widens fastest there, and needs twice the linear taper's
length. Near the narrow end the approximation fails (TE₂ is cut off below 0.63 µm in this plane),
and the short tapers' curves are not monotonic.

**Try it.** Set $w_2$ to 1.2 µm, the width of gdsfactory's crossing arms. How much of the light does
its linear 3.4 µm taper lose?

:::answer
About 0.25 %: the linear taper passes 0.9975 at 3.27 µm and reaches 99 % from 1.63 µm. Widening by
0.7 µm is gentle; the crossing's loss is at its centre, not in its tapers.
:::

### What the expansion has been checked against {research}

The expansion's tapers are validation cases: a stepped taper's reflection against 2D FDFD's grid
limit, and a strip taper cut from its drawing in 3D against 3D FDFD on the same plane, and the
chart's own test compares the linear taper above at 5 µm with 2D FDFD from 50 and 25 nm cells:

::validation eme/taper-fdfd eme/3d-cut-taper-fdfd

The job `eme-taper.yaml` widens a 220 nm slab to 600 nm over 1 µm parabolically with steps placed
where the response changes, to 0.99808 between the fundamental modes, and `eme-taper-3d.yaml` a
3D strip from 400 to 700 nm with every mode of its plane, to 0.996238.

## 7. How does a coupler turn TE₀ into TE₁?

Two guides side by side exchange power completely only if their modes travel at the same speed
([the directional coupler](directional-coupler.md)). Two guides of different widths can still be
matched, for a chosen pair of modes: a narrow guide's TE₀ and a wide guide's TE₁ have the same
index at one width of the wide guide, and there only that pair exchanges power, every other mode
of the wide guide being mismatched. This is the asymmetric directional coupler. Dai, Wang and Shi
chose the widths of their multiplexer from the phase-matching condition
$n_\mathrm{eff,0}(w_a) = n_{\mathrm{eff},i}(w_i)$.

The chart below draws a 220 nm strip's first four modes against its width (the full-vector solver,
each family on the quarter that holds it, 10 nm cells) and the access guide's TE₀, dashed. For a
500 nm access guide at 1.55 µm, TE₀'s index is 2.4483, matched to TE₁ at 1.034 µm, TE₂ at
1.566 µm and TE₃ at 2.101 µm.

::chart converter-match

Dai, Wang and Shi's design is for TM, a 0.4 µm access guide at 1500 nm with silicon 3.455 and
silica 1.445 (their text): they give 1.02, 1.68 and 2.35 µm. photonoxide's solver with those
indices gives 1.047, 1.695 and 2.343 µm on 10 nm cells, 27 nm, 15 nm and −7 nm from theirs. The TM
modes converge slowly at the strip's corners (on 5 nm cells their indices move by up to 0.005), but
the mismatch at their 1.02 µm stays: 0.0115 on 5 nm cells against 0.0136 on 10 nm. Their widths
are not reproduced to their printed digit; their paper gives no grid or tolerance for them.

**Try it.** Switch to TM. Which width does a 500 nm guide's TM₀ match as TM₁?

:::answer
1.255 µm, against TE's 1.034 µm for TE₁: TM's indices change more slowly with the width, so the
same access guide needs a wider partner, and its TM₃ isn't matched below 2.5 µm.
:::

## 8. What does the mode converter deliver?

gdsfactory's mode converter puts a 500 nm guide beside a 1 µm one for 20 µm, 150 nm apart, the
narrow one brought in and out by bends, the wide one tapered to 1.2 µm over 25 µm either side (Liu
and Poon, Section 3.4). The chart below launches the narrow strip's TE₀ on the straight section's
two modes that it and the wide strip's TE₁ make, carries each at its own index and projects the
field on each strip's own modes. At 1 µm the wide strip's TE₁ has index 2.4206, below the narrow
strip's 2.4483: a mismatch $\delta = \pi(n_1 - n_2)/\lambda = 0.0563$ per µm against a coupling
$\kappa = 0.0551$ per µm, so at most $\kappa^2/(\kappa^2 + \delta^2) = 0.489$ can cross, by
coupled-mode theory, and the overlaps give 0.432. The transfer peaks at $\pi/2S = 19.94$ µm: the
20 µm straight section is the coupling length, and the 1 µm width, 34 nm short of phase matching,
caps it.

::chart converter-coupler

Coupled-mode theory takes the two strips' modes as orthogonal and $\kappa$ from the pair's
splitting alone; the overlaps don't, and the two differ by 0.06 at 1 µm. At the phase-matched
1.034 µm they agree within the share of the launch the pair's two modes leave out: 0.994 crosses,
at 32.5 µm.

[mode_converter_liu_poon](../examples/mode_converter_liu_poon.rs) runs the whole device, bends and
tapers included, in 3D FDTD. On CI's 5 cells a wavelength the cross port's TE₁ is 0.457 at 1550 nm
and its TE₀, the crosstalk, −39.2 dB:

::example mode_converter_liu_poon

Liu and Poon's codes disagree here by 16 % (their Table 14): Lumerical gives 0.443, 0.482 and 0.512
at 15, 20 and 25 cells a wavelength, Tidy3D 0.492, 0.479 and 0.430 (read off their Fig. 16(a); the
text prints Tidy3D's 49.2 % at 15), and their spectra swing by ±0.1 within a few nanometres
(Fig. 16(b)). photonoxide's run at 15 cells gives 0.459 and −37.9 dB, within
their spread, with a smooth spectrum over 1540 to 1560 nm. The straight section's 0.432 and
−40.6 dB (the wide strip's TE₀ at 20 µm, 8.7 × 10⁻⁵) leave out the bends and tapers, which couple
too.

## 9. How does a coupler split two polarizations?

A strip's TE and TM modes depend on its width differently, so a coupler can be phase matched for
one and not the other. In the chart of Section 7 a 500 nm strip's TE₀ index is 2.4483 and its
TM₀'s 1.7786; TM's index moves less with the width. [Dai and Bowers (2011)](https://doi.org/10.1364/OE.19.018614)
bent two silicon wires of different widths around the same centre, and chose the widths so that
TM's optical paths along the two match while TE's don't: TM crosses and TE goes through. Their
design's bent coupling region is 4.5 µm long with a 200 nm gap, and their simulations give about
200 nm of band at an extinction ratio above 10 dB, with more than ±60 nm of tolerance on the width
(their abstract).

[Shen et al. (2015)](https://doi.org/10.1038/nphoton.2015.80) let an optimization draw a splitter
instead: 20 × 20 pixels of silicon 120 nm square, each etched or not, in 2.4 × 2.4 µm². Their
simulation gives 89 % of TM and 81 % of TE at their ports, and the made device over 70 % on average
with an extinction ratio above 10 dB across 32 nm. photonoxide's adjoint gradients
([FDFD and adjoint gradients](fdfd-and-adjoints.md)) are the tools of such a design; its inverse
design comes in the roadmap's 0.7 ([inverse design](inverse-design.md)).

## 10. How does a taper turn TM₀ into TE₁?

A strip with the same cladding above and below is mirror-symmetric up and down, and its modes are
of two kinds under that mirror: a TE-like mode's main E lies along the mirror plane and its image
is itself, which a magnetic wall on the plane holds; a TM-like mode's main E crosses the plane, and
an electric wall holds it (the quarters the strip lesson's charts solve on). The guide's equations
don't mix the two kinds, so TE₁ and TM₀ can't exchange power however the strip changes along its
length: where their indices are equal, their curves simply cross. Under silica the chart below has
them cross at 0.659 µm of width.

Put silicon nitride above and silica below, as gdsfactory's splitter-rotator does (Liu and Poon,
Section 2 and 3.5, after Dai and Bowers's splitter-rotator, their ref. 16), and the mirror is gone.
TE₁ and TM₀ belong to the same kind under the one mirror left, the strip's vertical plane (a
magnetic wall there holds both), and they hybridize: near 0.762 µm the two branches turn away from
each other, the least gap in index 0.0596, and each branch's polarization changes from TM to TE
across it. A taper slow enough to stay on one branch
takes TM₀ in and delivers TE₁.

::chart rotator-hybrid

### Through the anticrossing {theory}

Near the anticrossing, write the field on the two modes as they would be without the coupling, TM₀
and TE₁, with amplitudes $a_1$ and $a_2$, their mismatch $\Delta\beta_d$ and a coupling $\kappa$:

$$
\frac{d}{dz}\begin{pmatrix} a_1 \\ a_2 \end{pmatrix} = i\begin{pmatrix} \Delta\beta_d/2 & \kappa \\ \kappa & -\Delta\beta_d/2 \end{pmatrix}\begin{pmatrix} a_1 \\ a_2 \end{pmatrix} .
$$

The matrix's eigenvalues, $\pm\tfrac12\sqrt{\Delta\beta_d^2 + 4\kappa^2}$, are the branches: their
splitting $S$ is least, $2\kappa$, where $\Delta\beta_d = 0$, so $\kappa = \pi\Delta n_\mathrm{min}/\lambda$.
Their eigenvectors turn from one mode to the other over the mixing angle $\theta$,
$\tan 2\theta = 2\kappa/\Delta\beta_d$. As the taper widens, $\Delta\beta_d$ sweeps through zero
at a rate $\alpha = d\Delta\beta_d/dz$, and $\theta$ turns fastest at the crossing, at $\alpha/4\kappa$.
The light follows its branch while that turning is slow beside the branches' beat there, $2\kappa$:

$$
\frac{\alpha}{8\kappa^2} \ll 1 .
$$

The chart's last figure integrates these equations through gdsfactory's slow taper, from 690 to
830 nm over 44 µm (Liu and Poon's Fig. 18), with $\kappa$ from the least gap and $\Delta\beta_d$ at
each width from the solver's branches, $\Delta\beta_d = \pm\sqrt{S^2 - 4\kappa^2}$, starting in the
upper branch's local mode: 0.9912 of the light stays on it and leaves as TE₁. A taper four times
longer keeps more.

**Try it.** Switch the cladding above to silica. Where do TE₁ and TM₀ cross, and what does
Chrostowski and Hochberg's width sweep give?

:::answer
At 0.659 µm, each keeping its polarization (TE₁'s TE fraction above 0.5, TM₀'s below 0.01). Their
sweep of a 220 nm strip in oxide on 20 nm steps puts the crossing at about 680 nm (Section 3.2.8):
21 nm away, more than half their step. photonoxide's strip device solver (20 nm cells) puts it
between 640 and 660 nm too, so the difference isn't the grid.
:::

## 11. What does the polarization splitter-rotator deliver?

gdsfactory's splitter-rotator widens its input to 0.69 µm over 4 µm, to 0.83 µm over 44 µm and to
0.9 µm over 1.867 µm, then runs 7 µm beside a 0.404 µm guide 0.15 µm away before both leave (Liu
and Poon's Fig. 18; the example draws the narrow guide 405 nm wide). TM₀ in becomes TE₁ in the
taper, and the coupler, phase matched between the 0.9 µm guide's TE₁ and the narrow guide's TE₀,
passes it up.
[splitter_rotator_liu_poon](../examples/splitter_rotator_liu_poon.rs) runs it in 3D FDTD under
nitride of index 2.0, Liu and Poon's approximation. On CI's 5 cells a wavelength the upper port's
TE₀ is 0.642 and its TM₀ −22.0 dB: at 6 cells the paper's two codes give 0.147 and 0.051, the device
not working at that grid in either.

::example splitter_rotator_liu_poon

Both of Liu and Poon's codes settle near 95 %: from 0.942 to 0.955 at 20 and 25 cells, both pulses
(their Fig. 21(a)). photonoxide's runs give 0.95919 at 15 cells and 0.96369 at 20, above that span,
so this check fails. The response slides along the band with the grid, as Lumerical's does with its
mesh: at 15 cells photonoxide's TE₀ peaks near 1550 to 1555 nm, close to Lumerical's on a uniform
22 nm mesh (about 0.956 near 1550 nm, read off their Fig. 22(a)); at 20 cells about 10 nm further
on, close to Lumerical's at 20 cells with a 50 nm pulse (96.8 % at 1565 nm, their text). A check at
1550 nm sits on the response's slope. Across the band photonoxide's TE₀ peaks at 0.960 and at least
0.972, the codes' from 93.9 % (Lumerical's 20 nm pulse, near 1545 nm) to 96.8 %; whether
photonoxide's settles needs a run at 25 cells (issue #256).

## 12. How does an inverse taper meet a fibre?

A silicon strip's mode is half a micrometre across, a fibre's ten. An inverse taper narrows the
strip towards the chip's edge until its mode barely holds to it and spreads into the cladding, to
match a lensed or high-numerical-aperture fibre's beam
([Chrostowski and Hochberg, 2015](https://doi.org/10.1017/CBO9781316084168), Section 5.3).

The chart below solves the tip's mode, 220 nm thick in silica with 5 µm of silica beyond it, and
sums its overlap with a Gaussian beam focused on the facet, of waist $w_0 = \lambda/(\pi\,\mathrm{NA})$,
against the numerical aperture. A 180 nm tip's TE₀ has $n_\mathrm{eff} = 1.463$, against
Chrostowski and Hochberg's 1.46 (their Fig. 5.22's caption); it couples best at an NA of 0.44,
−0.26 dB.

::chart spot-size

Chrostowski and Hochberg's overlaps (Fig. 5.24, read off the figure) are best at an NA of about
0.4 for TE, about −1.1 dB, and about 0.55 for TM, about −1.4 dB. photonoxide puts TE's best at 0.44
but its coupling 0.85 dB higher, and TM's at 0.68, −0.10 dB. Their beam comes from an ideal lens of
that NA in their mode solver, whose definition their listing doesn't give, and their oxide reaches
2 µm from the tip (their Listing 5.9); a smaller window moves photonoxide's coupling up, not down,
so the difference isn't the window. It is not reproduced.

The taper must be adiabatic too, and near the tip it is hardest: the mode's index approaches the
cladding's, so it beats slowly against the radiation it could shed. Chrostowski and Hochberg's 3D
FDTD needs at least 100 µm of taper to add no loss of its own, and reaches 1 dB of insertion loss at
best; their overlaps put 1 dB of extra loss at 0.6 µm of misalignment (Section 5.3.1).

**Try it.** Switch to TM. Why is TM's best beam smaller?

:::answer
The tip is taller (220 nm) than it is wide (180 nm), so TM₀, whose E crosses the film, is better
guided: $n_\mathrm{eff} = 1.490$ against TE's 1.463, a smaller mode, best matched at an NA of 0.68
(a waist of 0.73 µm against 1.12 µm).
:::

## 13. How are crossings and converters made?

A crossing, a mode converter and a splitter-rotator are strips etched through 220 nm of silicon,
as [the chip lesson's](how-a-chip-is-made.md) strip is: one full etch that stops on the buried
oxide.

::fabrication waveguide

gdsfactory's crossing adds its ellipses in a second, partial etch: 70 nm into the film, leaving
the 150 nm slab. A partial etch has no film below to stop on, so it is timed, and its depth is only
as good as the etch's uniformity, ±3.5 nm here (5 % of the depth, the engine's textbook figure). The
library's rib is that etch:

::fabrication rib-waveguide{etch_depth=0.07}

photonoxide's fabrication engine plays the flows of its component library's devices, and the
library has no crossing, mode converter or splitter-rotator: none of its devices has both a full and
a partial etch, and though its process library holds PECVD silicon nitride, no device lays it over
a strip, as the splitter-rotator's cladding.

## 14. History and current research {research}

The effective index method ([Hocker and Burns, 1977](https://doi.org/10.1364/AO.16.000113)) and
self-imaging in multimode guides ([Soldano and Pennings, 1995](https://doi.org/10.1109/50.372474))
gave the designer of a planar device its plane and its multimode sections, and eigenmode expansion
with perfectly matched layers ([Bienstman and Baets, 2001](https://doi.org/10.1023/A:1010882531238))
a way to compute a long structure that changes along its length. On silicon wires, asymmetric and
bent couplers sorted polarizations ([Dai and Bowers, 2011](https://doi.org/10.1364/OE.19.018614))
and modes ([Dai, Wang and Shi, 2013](https://doi.org/10.1364/OL.38.001422)), and a design book
modelled the edge couplers and mode solutions such devices are built from
([Chrostowski and Hochberg, 2015](https://doi.org/10.1017/CBO9781316084168)).

::timeline

Current work draws devices by optimization, as [Shen et al. (2015)](https://doi.org/10.1038/nphoton.2015.80)'s
pixelated splitter, and asks how far the 3D simulations that rate them can be trusted:
[Liu and Poon (2025)](https://doi.org/10.1364/OPTCON.572107) found two commercial codes within 0.3 %
on a crossing and 16 % apart on a mode converter, whose response swings within a few nanometres.

Further reading: [eigenmode expansion](../docs/methods/eme.md) for the crossing and taper charts and
their limits; [the full-vector solver](../docs/methods/vector.md) and its walls for the converter
charts; [3D FDTD](../docs/methods/fdtd.md) for the three examples and their grids; and
[the directional coupler](directional-coupler.md) and [multimode interference couplers](mmi.md),
the couplers these devices are built from.

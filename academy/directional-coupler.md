---
title: "The directional coupler and splitters"
summary: "Two waveguides side by side trade their light back and forth: the even and odd supermodes and the cross-over length they set, how the coupling falls with the gap and moves with the wavelength, what an unequal pair can't transfer, a whole coupler in 3D, and how a split is set, tuned and made with Y-branches."
topic: Couplers and splitters
level: intermediate
minutes: 40
prerequisites: [strip-waveguide]
examples: [directional_coupler, coupler_liu_poon, circuit_splitter]
jobs: [coupler-supermodes.yaml]
circuits: [splitter-1x4.toml]
methods: [components.md, vector.md, walls.md, fields.md, fdtd.md, circuits.md, circuit-adjoint.md]
validation:
  - mode/strip-book
  - mode/fields-butt-coupling
  - components/directional-coupler-power
  - components/unitarity
  - components/mzi-closed-form
charts: [coupler-power, coupler-supermodes, coupler-gap, coupler-spectrum, coupler-mismatch]
papers:
  - cite: "J. R. Pierce, J. Appl. Phys. 25, 179 (1954)"
    title: "Coupling of Modes of Propagation"
    doi: 10.1063/1.1721599
    year: 1954
    role: origin
    note: "Two modes of propagation coupled to each other, analysed from linearity and the conservation of energy alone: the coupled-mode formalism that Yariv's guided-wave theory starts from."
  - cite: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969)"
    title: "Dielectric Rectangular Waveguide and Directional Coupler for Integrated Optics"
    doi: 10.1002/j.1538-7305.1969.tb01166.x
    year: 1969
    role: origin
    note: "The rectangular dielectric guide for integrated optics, and the directional coupler made of two: a symmetric and an antisymmetric mode, a coupling that falls exponentially with the separation over the field's penetration, and 3 dB couplers a few hundred micrometres long."
  - cite: "S. Somekh, E. Garmire, A. Yariv, H. L. Garvin, R. G. Hunsperger, Appl. Phys. Lett. 22, 46 (1973)"
    title: "Channel optical waveguide directional couplers"
    doi: 10.1063/1.1654468
    year: 1973
    role: milestone
    note: "The first channel-waveguide directional couplers: guides made in GaAs by proton implantation, the light crossing completely to the next channel at 1.15 µm in about 2 mm."
  - cite: "A. Yariv, IEEE J. Quantum Electron. 9, 919 (1973)"
    title: "Coupled-mode theory for guided-wave optics"
    doi: 10.1109/JQE.1973.1077767
    year: 1973
    role: milestone
    note: "Coupled-mode theory written once for guided waves: codirectional coupling with its phase mismatch, contradirectional coupling, and their use in modulators, filters and nonlinear optics."
  - cite: "H. Kogelnik, R. V. Schmidt, IEEE J. Quantum Electron. 12, 396 (1976)"
    title: "Switched directional couplers with alternating Δβ"
    doi: 10.1109/JQE.1976.1069190
    year: 1976
    role: milestone
    note: "Couplers whose sections alternate in phase mismatch, so that complete transfer is set by an electrical adjustment: electro-optic switches and modulators."
  - cite: "K. Jinguji, N. Takato, A. Sugita, M. Kawachi, Electron. Lett. 26, 1326 (1990)"
    title: "Mach–Zehnder interferometer type optical waveguide coupler with wavelength-flattened coupling ratio"
    doi: 10.1049/el:19900855
    year: 1990
    role: milestone
    note: "Two couplers with a path difference between them, in silica waveguides on silicon: a coupling ratio of 20 ± 1.9 % from 1.25 to 1.65 µm."
  - cite: "A. Takagi, K. Jinguji, M. Kawachi, J. Lightwave Technol. 10, 735 (1992)"
    title: "Wavelength characteristics of (2*2) optical channel-type directional couplers with symmetric or nonsymmetric coupling structures"
    doi: 10.1109/50.143072
    year: 1992
    role: milestone
    note: "How a coupler's ratio moves with the wavelength, and how unequal or tapered widths in the coupling region flatten it, from 1.33 to 1.55 µm by beam propagation."
  - cite: "T. A. Ramadan, R. Scarmozzino, R. M. Osgood, J. Lightwave Technol. 16, 277 (1998)"
    title: "Adiabatic couplers: design rules and optimization"
    doi: 10.1109/50.661021
    year: 1998
    role: milestone
    note: "Design rules for adiabatic 3 dB and full couplers, checked by beam propagation; the 3 dB coupler's length scaling makes it the harder of the two."
  - cite: "D. Dai, J. E. Bowers, Opt. Express 19, 18614 (2011)"
    title: "Novel ultra-short and ultra-broadband polarization beam splitter based on a bent directional coupler"
    doi: 10.1364/OE.19.018614
    year: 2011
    role: milestone
    note: "A polarization splitter from a bent coupler of two unequal silicon wires: TM phase matched and crossing, TE mismatched and staying, over a coupling region 4.5 µm long."
  - cite: "Y. Zhang, S. Yang, A. E.-J. Lim, G.-Q. Lo, C. Galland, T. Baehr-Jones, M. Hochberg, Opt. Express 21, 1310 (2013)"
    title: "A compact and low loss Y-junction for submicron silicon waveguide"
    doi: 10.1364/OE.21.001310
    year: 2013
    role: milestone
    note: "A silicon Y-junction shaped by particle swarm optimization over FDTD simulations: 0.28 ± 0.02 dB of measured insertion loss across an 8-inch wafer, in under 1.2 × 2 µm."
  - cite: "D. Dai, J. Wang, Y. Shi, Opt. Lett. 38, 1422 (2013)"
    title: "Silicon mode (de)multiplexer enabling high capacity photonic networks-on-chip with a single-wavelength-carrier light"
    doi: 10.1364/OL.38.001422
    year: 2013
    role: milestone
    note: "Cascaded asymmetric directional couplers as a silicon mode (de)multiplexer: four TM modes, under 1 dB of excess loss over about 20 nm."
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    title: "Silicon Photonics Design"
    doi: 10.1017/CBO9781316084168
    year: 2015
    role: review
    note: "Silicon couplers by their supermodes: the cross-over length λ/(2Δn), its exponential fall with the gap, its wavelength dependence, the bends' share and the sensitivity to fabrication; the equations photonoxide's DirectionalCoupler implements."
  - cite: "Z. Lu, H. Yun, Y. Wang, Z. Chen, F. Zhang, N. A. F. Jaeger, L. Chrostowski, Opt. Express 23, 3795 (2015)"
    title: "Broadband silicon photonic directional coupler using asymmetric-waveguide based phase control"
    doi: 10.1364/OE.23.003795
    year: 2015
    role: current
    note: "Silicon couplers with asymmetric-waveguide phase-control sections: splits from 10/90 to 50/50, TE and TM, every one over more than 75 nm and some over 100 nm."
  - cite: "R. Halir, P. Cheben, J. M. Luque-González, J. D. Sarmiento-Merenguel, J. H. Schmid, G. Wangüemert-Pérez, D.-X. Xu, S. Wang, A. Ortega-Moñux, Í. Molina-Fernández, Laser Photonics Rev. 10, 1039 (published online 22 November 2016)"
    title: "Ultra-broadband nanophotonic beamsplitter using an anisotropic sub-wavelength metamaterial"
    doi: 10.1002/lpor.201600213
    year: 2016
    role: current
    note: "A multimode-interference beamsplitter in a sub-wavelength metamaterial whose anisotropy and dispersion widen its band to a 500 nm design bandwidth."
  - cite: "A. Y. Piggott, J. Petykiewicz, L. Su, J. Vučković, Sci. Rep. 7, 1786 (2017)"
    title: "Fabrication-constrained nanophotonic inverse design"
    doi: 10.1038/s41598-017-01939-2
    year: 2017
    role: current
    note: "Inverse design under fabrication constraints; among its devices a 50-50 directional coupler optimized at six wavelengths from 1470 to 1630 nm, which in simulation couples 90 % into its outputs at 1520 nm and resembles a grating-assisted coupler."
  - cite: "Z. Liu, J. K. S. Poon, Opt. Continuum 4, 2427 (2025)"
    title: "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD simulations of passive silicon photonic components"
    doi: 10.1364/OPTCON.572107
    year: 2025
    role: current
    note: "Two commercial 3D FDTD codes on gdsfactory's components, the directional coupler among them: its cross port against the resolution and over the band, the numbers coupler_liu_poon is checked against."
---

The coupler from above, in the symbols the equations below use: two strips of width $w$ a gap
$g$ apart over a length $L$, the input, through and cross ports, and beneath, the field across
the pair of its two supermodes. The 3D view is the coupled section of
`jobs/coupler-supermodes.yaml`, which solves those supermodes as the gap closes.

::diagram coupler

Light in one waveguide has an evanescent tail outside its core. Where a second waveguide runs
inside that tail, the light leaks across, and from there back again: power beats between the
two guides with a period set by how strongly their fields overlap. Stopping the coupler at the
right length gives any split, from all through to all across; at half the way it is a 3 dB
coupler, the splitter of interferometers and the coupler of rings
([Chrostowski and Hochberg, 2015](https://doi.org/10.1017/CBO9781316084168), Section 4.1).

## 1. How do two waveguides exchange power?

Light enters the lower strip. Along the coupled section its power drains into the upper strip,
all of it after a length $L_c$, then returns, and so on. The chart draws the share in each strip
along $z$ for two 500 × 220 nm silicon strips in silica; the gap and the wavelength set the
period, and the marker the length $L$ at which a coupler would end.

::chart coupler-power

### Coupled-mode theory {theory}

Let $a(z)$ and $b(z)$ be the amplitudes of each strip's own mode, of propagation constants
$\beta_a$ and $\beta_b$, scaled so that $|a|^2$ and $|b|^2$ are their powers. Alone, each
gains phase as $e^{i\beta z}$. Together, each strip's field drives the other's mode through the
overlap of its tail, at a rate $\kappa$:

$$
\frac{d}{dz}\begin{pmatrix} a \cr b \end{pmatrix}
= i \begin{pmatrix} \beta_a & \kappa \cr \kappa & \beta_b \end{pmatrix}
\begin{pmatrix} a \cr b \end{pmatrix},
$$

the codirectional coupled-mode equations of
[Yariv (1973)](https://doi.org/10.1109/JQE.1973.1077767), his Section II.A, written with
photonoxide's $e^{i(\beta z - \omega t)}$. A real $\kappa$ keeps $|a|^2 + |b|^2$ constant. For
equal strips, $\beta_a = \beta_b = \beta$, and light entering strip $a$:

$$
a(z) = e^{i\beta z}\cos\kappa z, \qquad b(z) = i\thinspace e^{i\beta z}\sin\kappa z, \qquad
|b|^2 = \sin^2\kappa z .
$$

All the power has crossed at $\kappa z = \pi/2$, and the crossing field leads by $i$, the
convention of photonoxide's couplers. The cross and through shares at the coupler's end,
$\sin^2\kappa L$ and $\cos^2\kappa L$, are Chrostowski and Hochberg's Eqs. 4.1 and 4.2;
photonoxide's `DirectionalCoupler` against them, and conserving power:

::validation components/directional-coupler-power components/unitarity

**Try it.** In the chart, set the length to the 3 dB length in the figures, then to three times
it. What is the split at each?

:::answer
Half and half at both: $\sin^2\kappa L$ is $\tfrac{1}{2}$ at $\kappa L = \pi/4$ and at
$3\pi/4$. The light has crossed over and partly come back in the second. Section 4 shows why the
shorter one is the better 3 dB coupler.
:::

## 2. What are supermodes, and what sets the cross-over length?

The pair of strips is one waveguide with its own modes. The two TE-like ones of highest index are
the **even supermode**, its field the same sign in both strips, and the **odd supermode**, its
field opposite in sign; each travels unchanged. Light in one strip is the two in step: their sum
fills one strip and cancels in the other. The even supermode has the higher index, so the two
slip out of step; after a slip of $\pi$ their sum fills the other strip. The chart
draws each one's magnetic field $H_y$ across the pair, which has the shape of its electric field
$E_x$ without $E_x$'s jumps at the strips' sides, and their sum (dashed).

::chart coupler-supermodes

### The beat {theory}

The pair's modes are the eigenvectors of the matrix of Section 1, of propagation constants

$$
\beta_\pm = \bar\beta \pm \sqrt{\kappa^2 + \delta^2}, \qquad
\bar\beta = \frac{\beta_a + \beta_b}{2}, \qquad \delta = \frac{\beta_a - \beta_b}{2} .
$$

For equal strips $\delta = 0$, $\beta_\pm = \beta \pm \kappa$, and the eigenvectors are
$(1, 1)/\sqrt 2$ and $(1, -1)/\sqrt 2$: the even and the odd supermode. So

$$
\kappa = \frac{\beta_\text{even} - \beta_\text{odd}}{2} = \frac{\pi\Delta n}{\lambda}, \qquad
L_c = \frac{\pi}{2\kappa} = \frac{\lambda}{2\Delta n}, \qquad
\Delta n = n_\text{even} - n_\text{odd},
$$

Chrostowski and Hochberg's Eqs. 4.3 and 4.5. The supermodes' indices need no overlap integral:
a mode solver gives them, with all the coupling between two close, high-contrast strips in them.
The book calls this more accurate than finding $\kappa$ by perturbation, especially for
high-contrast guides coupled strongly (Section 4.1). photonoxide's `DirectionalCoupler` is this
beat: through $e^{i\bar\beta L}\cos(\Delta\beta L/2)$, across
$i\thinspace e^{i\bar\beta L}\sin(\Delta\beta L/2)$
([the components](../docs/methods/components.md)).

### Solving for each supermode {theory}

The pair is mirror symmetric about the gap's middle and about the strips' mid-height, so the
solver needs a quarter of the cross-section, with a wall on each cut
([mirror walls](../docs/methods/walls.md)). A TE-like mode's large field is $E_x$, across the
pair. An electric wall at $x = 0$ forces the tangential $E_y$ and $E_z$ to zero there and leaves
$E_x$ even: it keeps the even supermode. A magnetic wall makes $E_x$ odd: the odd supermode. Each
is then the fundamental mode of its quarter.

The chart solves both on a grid of 10 nm over the strips and the gap, graded to 100 nm at the
window's metal walls (Chrostowski and Hochberg's 5.5 × 1.22 µm window), in Li's silicon and
Malitson's silica. At a 200 nm gap and 1.55 µm it finds $n_\text{even} = 2.45959$,
$n_\text{odd} = 2.43857$, $\Delta n = 0.02103$ and $L_c = 36.85$ µm. On a 5 nm grid graded the
same way $L_c$ is 37.32 µm: the strips' corners converge slowly, and the single strip shows the
same, 2.4444 on a 5 nm grid against the book's 2.443:

::validation mode/strip-book

[directional_coupler](../examples/directional_coupler.rs) takes the book's own Lorentz silicon
and 1.444 oxide: 36.740 µm on a uniform 10 nm grid and 37.152 µm on a 5 nm grid, against the
book's 37.5 µm (Lumerical MODE, 10 nm mesh). The book's fit, its Eq. 4.14, gives 35.7 µm at
200 nm. The example also builds the coupler from those supermodes and checks that it splits
evenly at $L_c/2$:

::example directional_coupler

**Try it.** Widen the gap to 500 nm. What happens to $\Delta n$, and to the dashed sum?

:::answer
$\Delta n$ falls about thirteenfold, to 0.0016, and $L_c$ grows to about 500 µm. Each supermode
now looks like one strip's mode in each strip, with the same or the opposite sign, and their sum
still sits in one strip: the supermodes stay distinct however weak the coupling, only their
indices draw together.
:::

## 3. How does the coupling fall with the gap?

Outside a strip its field decays exponentially into the oxide, so the overlap of one strip's
tail with the other strip falls exponentially with the gap, and $L_c$ grows exponentially. On the
chart's logarithmic scale an exponential is a straight line: $L_c$ goes from 14.3 µm at 100 nm
to 34.6 mm at 1 µm, an e-fold every 117 nm (fitted from 200 to 1000 nm, at 1.55 µm, on the grid
of Section 2).

::chart coupler-gap

### Why exponential {theory}

In the uniform oxide beside a strip, a field $\propto e^{i\beta z}$ obeys
$\nabla_t^2 E = (\beta^2 - k^2 n_\text{ox}^2) E$. A slab's field, uniform along $y$, falls as
$e^{-\gamma x}$ with

$$
\gamma = k\sqrt{n_\text{eff}^2 - n_\text{ox}^2} ,
$$

and $\kappa$, the overlap of that tail with the other guide, scales as $e^{-\gamma g}$:
[Marcatili (1969)](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x) found the coupling
falling exponentially with the separation over the field's penetration (his Section IV), and
Chrostowski and Hochberg fit $C = B e^{-Ag}$ (their Eq. 4.6). For one 500 nm strip
($n_\text{eff} = 2.448$ on the chart's grid) $1/\gamma = 125$ nm; the fitted e-fold length is
117 nm, the book's Eq. 4.14 gives 115 nm. Beside a strip 220 nm tall the field also varies
along $y$; a variation $e^{ik_y y}$ adds $k_y^2$ to $\gamma^2$, so the field in the gap falls
somewhat faster than a slab's. Below about 200 nm the curve bends away from the line, as the
book notes for small gaps (Section 4.1.1).

Fabrication moves the gap: a 10 nm wider gap lengthens $L_c$ by $e^{10/117}$, 9 %, and a narrower
strip widens the gap as it narrows, so couplers are among the most fabrication-sensitive parts
of a circuit (Chrostowski and Hochberg, Section 4.1.5).

**Try it.** Two strips run side by side for 1 mm. How far apart must they be for less than
$10^{-3}$ of the power (−30 dB) to cross?

:::answer
$\sin^2(\pi L/2L_c) \le 10^{-3}$ needs $\pi L/2L_c \le 0.0316$, so $L_c \ge 50$ mm. The chart
reaches 34.6 mm at 1 µm; at 117 nm an e-fold, 50 mm needs about
$1000 + 117 \ln(50/34.6) \approx 1040$ nm. Wider guides, or a wavelength of 1.31 µm, couple less.
:::

## 4. Why is a 3 dB coupler 3 dB only in a band?

At a longer wavelength the mode spreads further out of the strips, the tails overlap more, and
$L_c$ shortens. A coupler cut to cross half at 1.55 µm crosses more at longer wavelengths and less
at shorter ones. The chart draws the split of a coupler $L$ long, in units of its $L_c$ at
1.55 µm, from 1.50 to 1.60 µm, with the supermodes' and the materials' dispersion.

::chart coupler-spectrum

At a 200 nm gap $L_c$ is 45.81 µm at 1.50 µm, 36.85 at 1.55 and 29.91 at 1.60 (the chart's grid;
the example finds 40.059 and 33.746 µm at 1.53 and 1.57 on a uniform 10 nm grid). The 3 dB
coupler, 18.43 µm long, keeps its crossed share within 0.45 to 0.55 from 1.535 to 1.565 µm, a
band of 29.7 nm; a coupler 1.5 $L_c$ long, also 3 dB at 1.55 µm, over 9.5 nm.

### The slope at the 3 dB point {theory}

The crossed share is $P = \sin^2\varphi$, $\varphi = \kappa L = (\pi/2)(L/L_c)$. With $L$ fixed,

$$
\frac{dP}{d\lambda} = \sin 2\varphi\thinspace\frac{d\varphi}{d\lambda}
= -\sin 2\varphi\thinspace\varphi\thinspace\frac{d\ln L_c}{d\lambda} .
$$

At $\varphi = \pi/4$, $\sin 2\varphi = 1$; at $3\pi/4$, $-1$: the longer coupler's slope is three
times the shorter's, its band a third. From the chart's $L_c$,
$d\ln L_c/d\lambda \approx \ln(29.91/45.81)/0.1 = -4.26$ per µm, over the 0.1 µm drawn, so the 3 dB coupler's
$|dP/d\lambda|$ is $(\pi/4)(4.26) = 3.3$ per µm, and $\pm 0.05$ spans about 30 nm, as measured. A
coupler of fixed length is 3 dB at one wavelength; the shortest one, a single quarter beat, is
the broadest.

### Broadband couplers {research}

Three ideas widen the band. **Two couplers with a path difference between them**, a
Mach–Zehnder-type coupler, the phase it adds chosen against the couplers' change with
wavelength: [Jinguji et al. (1990)](https://doi.org/10.1049/el:19900855) held a 20 ± 1.9 % ratio from 1.25 to
1.65 µm in silica waveguides on silicon. **Unequal or tapered strips** in the coupling region, a mismatch
(Section 5) designed in: [Takagi, Jinguji and Kawachi (1992)](https://doi.org/10.1109/50.143072) predicted flat couplings
from 1.33 to 1.55 µm, and [Lu et al. (2015)](https://doi.org/10.1364/OE.23.003795) made silicon
couplers with asymmetric phase-control sections from 10/90 to 50/50, each over more than 75 nm.
**Adiabatic couplers**: unequal strips brought slowly to equal; light entering the wider strip
stays in the pair's higher-index mode as it evolves into the even supermode, which divides
evenly, at every wavelength slow enough for the change; the 3 dB version is the harder to make
short ([Ramadan, Scarmozzino and Osgood, 1998](https://doi.org/10.1109/50.661021)).

## 5. What happens when the two strips differ?

Two unequal strips have modes of different speeds. The light crossing into the second strip
falls out of step with the light already there before the transfer is complete, and only part
crosses, sooner. The chart launches light in a 500 nm strip's own mode beside a strip $w_2$ wide:
at 480 nm, 20 nm narrower, at most a third crosses; at 500 nm, all of it.

::chart coupler-mismatch

### Phase mismatch {theory}

Solving Section 1's equations with $\delta \ne 0$, light entering strip $a$:

$$
|b(z)|^2 = \frac{\kappa^2}{\kappa^2 + \delta^2}\thinspace\sin^2\left(\sqrt{\kappa^2 + \delta^2}\thinspace z\right),
$$

Yariv's Eq. 6 in this lesson's symbols (his $\Delta$ is $2\delta$). The transfer peaks at
$z = \pi/(2S)$, $S = \sqrt{\kappa^2 + \delta^2}$, and its peak $\kappa^2/(\kappa^2 + \delta^2)$
is below 1 once $\delta$ is comparable with $\kappa$. Section 2's eigenvalues give $S$ from the
pair's two modes, $S = \pi(n_a - n_b)/\lambda$, and $\delta = \pi(n_1 - n_2)/\lambda$ from each
strip alone, so $\kappa^2 = S^2 - \delta^2$. At a 200 nm gap and 1.55 µm the 500 nm strip has
$n_1 = 2.4479$ and a 480 nm one $n_2 = 2.4150$: $\delta = 0.0668$ per µm against
$\kappa = 0.0454$, a peak of 0.316. A width difference of 10 nm still caps the transfer at 0.65.

The chart's solid curves don't assume coupled-mode theory. The 500 nm strip's mode, $\psi_1$, is
written as a sum of the pair's two modes $\phi_m$ with amplitudes
$c_m = \langle\psi_1, \phi_m\rangle / \langle\phi_m, \phi_m\rangle$,
$\langle a, b\rangle = \int (\mathbf E_a \times \mathbf H_b^*)\cdot\hat z\thinspace dA$; each
travels at its own $\beta_m$; the share in strip $j$'s mode at $z$ is the overlap of that field
with $\psi_j$ ([the fields](../docs/methods/fields.md)). All five modes come from the full-vector
solver on one grid, the overlaps as validated here:

::validation mode/fields-butt-coupling

From 450 to 550 nm the solid curve's peak is within 0.025 of the dashed $\kappa^2/(\kappa^2 + \delta^2)$
(0.330 against 0.316 at 480 nm), and for equal strips 0.9994 of the power crosses, the rest the
small part of the strip's own mode that isn't in the pair's two modes.

### Mismatch put to use {research}

A mismatch that cancels for one mode and not another sorts them.
[Dai and Bowers (2011)](https://doi.org/10.1364/OE.19.018614) bent two unequal silicon wires
into a coupler where TM is phase matched and crosses while TE is not and stays: a polarization
splitter with a 4.5 µm coupling region. A narrow strip's fundamental mode can be matched to a
wide strip's second mode, and only that mode crosses: cascaded asymmetric couplers multiplex
four modes in [Dai, Wang and Shi (2013)](https://doi.org/10.1364/OL.38.001422), and gdsfactory's
mode converter turns a 500 nm strip's TE₀ into a 1 µm strip's TE₁, computed in 3D in
[the lesson on crossings and converters](crossings-and-converters.md).
[Kogelnik and Schmidt (1976)](https://doi.org/10.1109/JQE.1976.1069190) proposed setting the
mismatch electrically, in sections of alternating sign, to switch a coupler completely.

**Try it.** Set the second strip to 520 nm, then to 480 nm. Which way does the transfer length
change, and why does wider couple less?

:::answer
Both cut the peak to about 0.3 and shorten the distance to it ($\pi/2S$ grows with $\delta$).
The 520 nm pair also has the smaller $\kappa$, 0.039 against 0.045 per µm: wider strips hold
their light more tightly, so less of it reaches across the same gap.
:::

## 6. What does a whole coupler, S-bends included, do in 3D?

A real coupler brings its strips together and apart by bends, which couple too. Liu and Poon
compare two commercial 3D FDTD codes on gdsfactory's generic coupler: two 500 × 220 nm strips in
silica, 236 nm apart for 20 µm, each brought in and out by an S-bend 10 µm long and 1.632 µm
across ([Liu and Poon, 2025](https://doi.org/10.1364/OPTCON.572107), Section 3.1). At 1550 nm the
cross port's TE₀ settles from 15 cells a wavelength on at 0.411 to 0.446 in Lumerical and 0.448
to 0.492 in Tidy3D, rising with the wavelength in both.

[coupler_liu_poon](../examples/coupler_liu_poon.rs) runs the same device in photonoxide's 3D
FDTD: 0.42771 at 15 cells a wavelength (1437 × 260 × 99 cells of 29.7 nm) and 0.41352 at 20
(22.3 nm cells), within the two codes' range, with excess losses of −0.0014 and −0.0017 dB and
the band rising from 0.396 to 0.460 between 1540 and 1560 nm at 15 cells. The check CI runs is
at 6 cells a wavelength (74.3 nm cells), held to how far the paper's own codes stray there:

::example coupler_liu_poon

The coupled section alone, 20 µm at a 236 nm gap, crosses 0.336 (below, $L_c = 50.80$ µm on the
supermode grid of Section 2). The bends make up the rest: as extra straight length they are worth
2.6 µm at 20 cells and 3.1 µm at 15, 2.5 to 5.1 µm across the paper's values. Chrostowski and
Hochberg fold their rib coupler's bends in the same way, as 2.7 µm more length (Section 4.1.4).

::chart coupler-power{gap=236nm, length=20um}

## 7. Where are couplers used, and how is a split tuned?

- **Splitters**: a coupler $L$ long crosses $\sin^2\kappa L$; length and gap set any split.
- **Mach–Zehnder interferometers**: two couplers and two arms
  ([the Mach–Zehnder lesson](mach-zehnder.md)); a 55:45 splitter limits the extinction to
  26 dB, so the band of Section 4 is the band of deep fringes. photonoxide's interferometer of two
  `DirectionalCoupler`s against its transfer matrices:

::validation components/mzi-closed-form

- **Rings**: the coupler between bus and ring sets $\kappa^2$, and so the coupling regime
  ([the ring lesson](ring-resonator.md)); the gap's exponential sets it by tens of nanometres.
- **Polarization splitters and mode multiplexers**: mismatched couplers (Section 5).

A coupler's split is fixed when it is made. To tune it, two 3 dB couplers with a phase $\theta$
between them make a variable splitter whose bar port carries $\sin^2(\theta/2)$.
[circuit_splitter](../examples/circuit_splitter.rs) finds the $\theta$ that keeps 30 % in the bar
port, $2\arcsin\sqrt{0.3} = 1.159279$ rad, by genoxide's optimizers on the gradient of
[the circuit adjoint](../docs/methods/circuit-adjoint.md):

::example circuit_splitter

## 8. How do Y-branches split, and how do they compare with couplers?

A Y-branch divides one guide into two symmetric arms. By symmetry the stem's mode feeds both arms
equally at every wavelength, so it is a 1 × 2 splitter with no band to speak of, where a coupler
is a 2 × 2 one whose split moves with the wavelength. As a combiner a Y-branch keeps only the
arms' in-phase sum: light in one arm alone loses half its power into the second-order mode or
radiation (Chrostowski and Hochberg, Section 4.2). Its cost is excess loss at the junction:
[Zhang et al. (2013)](https://doi.org/10.1364/OE.21.001310) shaped a silicon Y-junction by
particle swarm optimization over FDTD and measured 0.28 ± 0.02 dB across an 8-inch wafer, in
under 1.2 × 2 µm.

photonoxide's `YBranch` is that ideal splitter with an excess loss as its parameter; three of them
make the 1 × 4 tree of `circuits/splitter-1x4.toml`, which the Chip page opens. A 1 × 2
multimode-interference splitter does the same by self-imaging
([the MMI lesson](mmi.md)).

## 9. How photonoxide computes it

- **The coupler**: `DirectionalCoupler` in `circuit::components`, the beat of Section 2 from two
  supermodes, with exact derivatives for gradients; `Coupler` is the ideal one with $\kappa^2$
  as its parameter ([the components](../docs/methods/components.md)). The supermodes come by
  hand (`Supermodes::from_coupling`) or from the full-vector solver
  (`DirectionalCoupler::from_modes`), tracked over a band to give their dispersion. Only the
  coupled section is modelled: no bends and no reflection.
- **The supermodes**: [the full-vector mode solver](../docs/methods/vector.md) on a quarter of
  the cross-section, with [mirror walls](../docs/methods/walls.md); `jobs/coupler-supermodes.yaml`
  solves them over gaps from 100 to 400 nm on a uniform 10 nm grid, and its strips are the 3D
  view of the diagram.
- **The charts**: each solves its supermodes as asked, on the graded grid of Section 2, and the
  unequal pair's five modes on one grid of the half cross-section.
- **The whole device**: [3D FDTD](../docs/methods/fdtd.md), as `coupler_liu_poon` runs it.
- **In circuits**: couplers in netlists, solved by [the circuit solve](../docs/methods/circuits.md).

## 10. History and current research {research}

The coupling of modes was worked out for microwave tubes and filters:
[Pierce (1954)](https://doi.org/10.1063/1.1721599) treated two coupled modes from linearity and
the conservation of energy alone, the formalism Yariv's paper starts from (his ref. 8). In 1969
Marcatili proposed the rectangular dielectric guide for integrated optics and the directional
coupler made of two, with its symmetric and antisymmetric modes and 3 dB couplers a few hundred
micrometres long. In 1973 [Somekh et al.](https://doi.org/10.1063/1.1654468) made the first
channel-waveguide couplers, in GaAs, the light crossing completely in about 2 mm at 1.15 µm, and
Yariv wrote coupled-mode theory once for guided-wave optics. Kogelnik and Schmidt proposed
switching couplers electrically with alternating mismatch in 1976. In silica waveguides on
silicon, Jinguji and colleagues made a Mach–Zehnder-type coupler flat over a wide band in 1990,
and Takagi, Jinguji and Kawachi predicted flat couplings from unequal and tapered strips in
1992. Adiabatic couplers got their design rules in 1998. In
silicon, strips a few hundred nanometres wide couple over tens of micrometres;
[Chrostowski and Hochberg (2015)](https://doi.org/10.1017/CBO9781316084168) set out how they are
designed from supermodes, the method photonoxide's coupler follows.

::timeline

Current work widens bands and shrinks footprints: asymmetric phase-control sections
([Lu et al., 2015](https://doi.org/10.1364/OE.23.003795)), sub-wavelength metamaterials, which
gave a multimode-interference beamsplitter a 500 nm design bandwidth
([Halir et al., 2016](https://doi.org/10.1002/lpor.201600213)), and inverse design, whose 50-50
coupler optimized at six wavelengths from 1470 to 1630 nm came out resembling a grating-assisted
coupler ([Piggott et al., 2017](https://doi.org/10.1038/s41598-017-01939-2)). Couplers serve
polarization and mode multiplexing ([Dai and Bowers, 2011](https://doi.org/10.1364/OE.19.018614);
[Dai, Wang and Shi, 2013](https://doi.org/10.1364/OL.38.001422)). Open problems are the ones
Sections 3 and 4 measure: a split that holds across a wafer's spread of gaps and widths, and
across a band. Simulating a whole coupler reliably is itself studied: two commercial 3D FDTD
codes agree on gdsfactory's coupler to within 4.6 % from 15 cells a wavelength on
([Liu and Poon, 2025](https://doi.org/10.1364/OPTCON.572107)).

Further reading: Chrostowski and Hochberg's Section 4.1 for silicon couplers' design, bends and
fabrication; [Yariv (1973)](https://doi.org/10.1109/JQE.1973.1077767) for coupled-mode theory;
[Marcatili (1969)](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x) for the coupler's first
analysis; and the lessons on [the Mach–Zehnder interferometer](mach-zehnder.md) and
[the ring resonator](ring-resonator.md), the two devices most couplers serve.

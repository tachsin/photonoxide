---
title: "Grating couplers"
summary: "A grating etched into a waveguide sends its light up and out at an angle, into a fibre above the chip: the phase matching that sets the angle, the overlap with the fibre's mode and the uniform grating's 80 %, apodization, the share sent down to the substrate, the bandwidth, the fabrication's errors, and optimized couplers against Passoni et al.'s published designs."
topic: Gratings and filters
level: intermediate
minutes: 45
prerequisites: [bragg-gratings, slab-waveguide]
examples: [optimize_grating_pareto]
methods: [fdfd.md, multilayer.md, slab.md]
validation:
  - mode/multilayer-fresnel
  - mode/multilayer-bragg
charts: [grating-angle, grating-overlap, grating-apodization, grating-directionality, grating-band]
papers:
  - cite: "T. Tamir, A. A. Oliner, Proc. IEE 110, 310 (1963)"
    title: "Guided complex waves. Part 1: Fields at an interface"
    doi: 10.1049/piee.1963.0044
    year: 1963
    role: origin
    note: "Waves along an interface whose wavenumbers are complex though nothing is lossy: leaky waves that travel along a guide while shedding power at an angle, the picture a grating coupler's beam is drawn from."
  - cite: "T. Tamir, S. T. Peng, Appl. Phys. 14, 235 (1977)"
    title: "Analysis and design of grating couplers"
    doi: 10.1007/BF00882729
    year: 1977
    role: origin
    note: "A grating coupler as a guided wave turned leaky: the angle of each diffraction order, the conditions for a single beam, the leakage rate α, and the 80 % that an exponential beam can couple at most to a Gaussian."
  - cite: "D. Taillaert, W. Bogaerts, P. Bienstman, T. F. Krauss, P. Van Daele, I. Moerman, S. Verstuyft, K. De Mesel, R. Baets, IEEE J. Quantum Electron. 38, 949 (2002)"
    title: "An out-of-plane grating coupler for efficient butt-coupling between compact planar waveguides and single-mode fibers"
    doi: 10.1109/JQE.2002.1017613
    year: 2002
    role: milestone
    note: "A short grating etched into a thin high-contrast guide couples it to a single-mode fibre held above: up to 74 % in 2D simulations for a 240 nm GaAs–AlOx guide, 19 % measured."
  - cite: "D. Taillaert, F. Van Laere, M. Ayre, W. Bogaerts, D. Van Thourhout, P. Bienstman, R. Baets, Jpn. J. Appl. Phys. 45, 6071 (2006)"
    title: "Grating Couplers for Coupling between Optical Fibers and Nanophotonic Waveguides"
    doi: 10.1143/JJAP.45.6071
    year: 2006
    role: milestone
    note: "Silicon-on-insulator grating couplers designed and measured: a 630 nm period etched 70 nm, 31 % and a 40 nm 1-dB band measured, and in simulation what a cladding, the buried oxide's thickness, a bottom mirror and apodization each add."
  - cite: "X. Chen, C. Li, C. K. Y. Fung, S. M. G. Lo, H. K. Tsang, IEEE Photon. Technol. Lett. 22, 1156 (2010)"
    title: "Apodized Waveguide Grating Couplers for Efficient Coupling to Optical Fibers"
    doi: 10.1109/LPT.2010.2051220
    year: 2010
    role: milestone
    note: "A grating whose fill factor varies along it to shape its beam to the fibre's Gaussian, in 340 nm silicon etched 200 nm deep for directionality: 1.2 dB of loss measured per fibre-to-chip interface."
  - cite: "D. Vermeulen, S. Selvaraja, P. Verheyen, G. Lepage, W. Bogaerts, P. Absil, D. Van Thourhout, G. Roelkens, Opt. Express 18, 18278 (2010)"
    title: "High-efficiency fiber-to-chip grating couplers realized using an advanced CMOS-compatible Silicon-On-Insulator platform"
    doi: 10.1364/OE.18.018278
    year: 2010
    role: milestone
    note: "A silicon overlay over the grating raises its directionality: −1.6 dB of coupling and a 3 dB band of 80 nm, made in a CMOS pilot line on 200 mm wafers."
  - cite: "A. Mekis, S. Gloeckner, G. Masini, A. Narasimha, T. Pinguet, S. Sahni, P. De Dobbelaere, IEEE J. Sel. Top. Quantum Electron. 17, 597 (2011)"
    title: "A Grating-Coupler-Enabled CMOS Photonics Platform"
    doi: 10.1109/JSTQE.2010.2086049
    year: 2011
    role: milestone
    note: "Grating couplers as the optical interface of a CMOS photonics platform: they make wafer-scale optical testing possible, from which the device library and a 4 × 10 Gb/s transceiver were built."
  - cite: "W. S. Zaoui, A. Kunze, W. Vogel, M. Berroth, J. Butschke, F. Letzkus, J. Burghartz, Opt. Express 22, 1277 (2014)"
    title: "Bridging the gap between optical fibers and silicon photonic integrated circuits"
    doi: 10.1364/OE.22.001277
    year: 2014
    role: milestone
    note: "Gratings over a backside metal mirror on standard SOI wafers: −1.08 dB measured with a uniform grating and −0.62 dB with a nonuniform one, each with a 1 dB band wider than 40 nm."
  - cite: "M. Passoni, D. Gerace, L. Carroll, L. C. Andreani, Appl. Phys. Lett. 110, 041107 (2017)"
    title: "Grating couplers in silicon-on-insulator: The role of photonic guided resonances on lineshape and bandwidth"
    doi: 10.1063/1.4974992
    year: 2017
    role: current
    note: "A small footprint's band is Gaussian, set by the beam's spread of wavevectors, a large one's Lorentzian, set by the grating's guided resonances; Pareto fronts of efficiency against 1-dB bandwidth for fibres of 4 and 6 µm mode field diameter, the designs optimize_grating_pareto re-solves."
  - cite: "R. Marchetti, C. Lacava, L. Carroll, K. Gradkowski, P. Minzioni, Photonics Res. 7, 201 (2019)"
    title: "Coupling strategies for silicon photonics integrated chips [Invited]"
    doi: 10.1364/PRJ.7.000201
    year: 2019
    role: review
    note: "A review of how light gets from fibres into silicon chips: grating couplers and edge couplers side by side, their efficiencies, bands and alignment tolerances."
---

The coupler in section along its guide, in the symbols the equations below use: a film $t$ thick
with grooves $e$ deep at a period $\Lambda$, the buried oxide $h$ thick over the substrate, the
fibre tilted at $\theta$, and the three ways the guided light leaves: up, down and through. Just
above the grating, the power a uniform grating sends up against the fibre's Gaussian.

::diagram grating

A grating coupler is a few tens of grooves etched part-way into a waveguide that has been widened
to a fibre's size. Each groove scatters a little of the guided light; in one direction the
scattered waves add in step and leave the chip as a beam, which a fibre held above the surface
catches. By reciprocity the same grating takes a fibre's light into the chip with the same
efficiency ([Tamir and Peng, 1977](https://doi.org/10.1007/BF00882729), p. 237).

## 1. What does a grating coupler do, and why does a chip need one?

A standard single-mode fibre's mode is 10.4 µm across at 1.55 µm, where its field falls to $1/e$
([Taillaert et al., 2006](https://doi.org/10.1143/JJAP.45.6071), Section 2.4;
[Chen et al., 2010](https://doi.org/10.1109/LPT.2010.2051220)). A silicon strip's mode fills about
500 × 220 nm. The fibre's mode has $\pi \times 5.2^2 = 85$ µm² of area, some 770 times the strip's
0.11 µm², so a fibre butted against a strip loses almost all its light.

Two answers exist ([Marchetti et al., 2019](https://doi.org/10.1364/PRJ.7.000201)). An **edge
coupler** brings the light out at the chip's polished facet through a taper that grows the mode;
the review cites one whose efficiency fell by 3 dB for 1 µm of misalignment of its lensed fibre. A **grating coupler**
brings it out of the surface: the strip is widened by a taper (Taillaert et al.'s: 400 µm long,
from a 450 nm wire to 12 µm, Section 3.2) and a grating across the wide guide sends the light up.
Its advantages follow from where it sits:

- **Anywhere on the wafer.** No facet is needed, so a wafer is measured before it is diced:
  [Mekis et al. (2011)](https://doi.org/10.1109/JSTQE.2010.2086049) built the device library of a
  CMOS photonics platform on wafer-scale optical testing through grating couplers.
- **Loose alignment.** A fibre 1 µm off in any direction adds less than 0.5 dB of loss
  (Taillaert et al., 2006, Fig. 9, measured and simulated).

Its costs are the subject of this lesson: a coupler of one period works over a band of tens of
nanometres (Section 7), for one polarization (a polarization-splitting grating couples both:
Mekis et al., Section I), and the light it sends down or past the fibre is lost
(Sections 4 to 6).

## 2. At what angle does the light leave?

The guided mode's field runs along the guide as $e^{i\beta z}$, $\beta = k n_\text{eff}$ and
$k = 2\pi/\lambda$. A grating of period $\Lambda$ multiplies it by a periodic function, which adds
space harmonics $\beta_m = \beta + 2\pi m/\Lambda$. A harmonic radiates into the cladding of index
$n_c$ where $\lvert\beta_m\rvert \lt k n_c$, at $\sin\theta_m = \beta_m/(k n_c)$ from the normal
(Tamir and Peng, Eqs. 1 to 3). The order $m = -1$ gives the coupler's equation:

$$
n_c \sin\theta = n_\text{eff} - \frac{\lambda}{\Lambda} .
$$

With $\theta \gt 0$ the beam leans along the guide's direction. At $\theta = 0$, where
$\lambda = n_\text{eff}\Lambda$, the second order is the Bragg condition of
[Bragg gratings](bragg-gratings.md) and the grating reflects the guided light back down the guide;
Tamir and Peng keep at least 3 to 4° away from it (p. 237), and Taillaert et al. tilt the fibre
10° for the same reason (Section 2.2).

The chart draws $\theta$ against the wavelength for a 220 nm silicon film on oxide with grooves
70 nm deep. It needs the grating's $n_\text{eff}$; here that is the mean of two exact slab
indices, the unetched film's and the grooves' thinner film's, weighted by the tooth's share $f$ of
the period, a first-order estimate:

::chart grating-angle{cladding=1}

Taillaert et al.'s grating, a 630 nm period, 70 nm deep, $f = 0.5$, in air, is designed for 10°
at 1550 nm. The estimate gives 11.81°: the slabs' indices are 2.8279 and 2.5021, their mean
2.6650, and the period it puts at 10° is 622.2 nm, 1.2 % shorter than theirs. The mean of the
slabs is a first step; the 2D solve of Section 9 needs no such estimate, as it solves the grooves
as drawn.

### The angle's slope {theory}

Differentiating the coupler's equation at a fixed period, with
$n_g = n_\text{eff} - \lambda\thinspace dn_\text{eff}/d\lambda$ the grating's group index:

$$
n_c\cos\theta\thinspace\frac{d\theta}{d\lambda} = \frac{dn_\text{eff}}{d\lambda} - \frac{1}{\Lambda}
= -\frac{n_g - n_c\sin\theta}{\lambda} .
$$

The beam swings towards the normal as the wavelength grows. Read the other way, a fibre held at a
fixed angle picks out one wavelength, which moves by
$d\lambda/d\theta = \lambda n_c\cos\theta/(n_g - n_c\sin\theta)$ per radian of tilt: 8.02 nm per
degree for the grating above ($n_g$ = 3.506 from the slabs, by a central difference over ±1 nm),
against Taillaert et al.'s "approximately 9 nm per degree" (Section 2.5).

**Try it.** In the chart, set the period to 622 nm. Then set the cladding to 1.444, an oxide
cladding over the grating. What happens to the angle at 1.55 µm, and why?

:::answer
At 622 nm the beam leaves at 10° in air. Under oxide the slabs' indices rise (2.8448 and 2.5367
at 630 nm) and $n_c$ in the equation is 1.444: at 630 nm the angle falls from 11.81° to 9.18°, and
10° needs 635.2 nm. Both sides of the equation change with the cladding, so a design is redone for
it; Taillaert et al. moved their fibre from 10° to 8° when they added an index-matched cladding
(Section 2.3).
:::

## 3. What is a fibre's mode, and how is coupling to it measured?

A single-mode fibre's mode is close to a Gaussian, $G \propto e^{-r^2/w_0^2}$, and its **mode field
diameter** (MFD) is $2w_0$, the diameter at which the field falls to $1/e$ of its peak
([Passoni et al., 2017](https://doi.org/10.1063/1.4974992), p. 2). Tilted at $\theta$, the beam
lights a strip of the surface $w = w_0/\cos\theta$ wide on each side of its axis, its footprint.

The **coupling efficiency** is the share of the fibre's power that ends up in the guide's mode.
In a 2D section along the guide it is the overlap of the field $E(z)$ the grating sends up with
the fibre's mode on a line above the grating,

$$
\eta = \frac{\left\lvert\int E(z)\thinspace G^*(z)\thinspace dz\right\rvert^2}{\int\lvert E\rvert^2 dz\int\lvert G\rvert^2 dz}\thinspace P_\text{up} ,
$$

with $P_\text{up}$ the share of the guided power the grating sends up, if the grating is long
enough to empty the guide. Two factors, then: the **directionality** $P_\text{up}$ (Section 6),
and the **overlap**, the shape and phase of the beam against the fibre's (Sections 4, 5 and 7).
Taillaert et al. name the same two limits (end of Section 2.2). A 3D coupler has a third: the overlap
across the guide, of the widened guide's mode with the fibre's.

Taillaert et al. measured theirs with a straight guide and a coupler at each end: the
transmission $T = \eta^2$, so $\eta = \sqrt{T}$ (their Eq. 3.1). Their 9.6 % ($-10.2$ dB) through
both couplers is 31 % ($-5.1$ dB) per coupler (Section 3.2).

## 4. Why does a uniform grating couple at most 80 %?

In a uniform grating every groove scatters the same share of what reaches it, so the guided power
falls as $P = P_0 e^{-2\alpha z}$ along the grating, and the beam's field as $e^{-\alpha z}$: a
leaky wave whose propagation constant is $\beta + i\alpha$ (Tamir and Peng, p. 236). Its beam
rises sharply at the grating's start and fades exponentially; the fibre's Gaussian is symmetric.
No choice of $\alpha$ makes them match:

::chart grating-overlap

At the best leakage the overlap is 80.10 % for any fibre and any angle: Tamir and Peng's "at most
80 %" for a Gaussian (p. 237). It is reached at $\alpha\thinspace\mathrm{MFD}\sec\theta = 1.37$
(their Eq. 6a, 1.36), with the fibre's axis 0.73 $w$ past the grating's start. For a standard
fibre at 10° that is $\alpha$ = 0.1295 per µm and the axis 3.86 µm from the start; the guided
power decays over $1/(2\alpha)$ = 3.86 µm. Taillaert et al.'s 70 nm grating has a "coupling
length" of 3.8 µm, which they chose by the same rule, $w_0 = 1.37 L_c\cos\theta$ (Section 2.2);
Chen et al. give $\alpha \approx 0.13$ per µm for the 80 % optimum with a 10.4 µm fibre.

### The overlap, computed {theory}

On the surface the fibre's field is $G(z) = e^{-(z - c)^2/w^2}$, its axis at $c$, and the
grating's, from its start at $z = 0$, is $E(z) = e^{-\alpha z}$, phase matched to it. The
normalizations are $\int_0^\infty e^{-2\alpha z}dz = 1/(2\alpha)$ and
$\int G^2 dz = w\sqrt{\pi/2}$, so

$$
\eta_\text{overlap} = \frac{2\alpha\left(\int_0^\infty e^{-\alpha z}e^{-(z - c)^2/w^2}dz\right)^2}{w\sqrt{\pi/2}} .
$$

It depends on $\alpha w$ and $c/w$ alone, which is why the optimum is one number for every fibre.
The chart maximizes it over $c$ at each $\alpha$ and over both, summing the integral on steps of
$w/400$; a unit test checks the sum against a sum a hundred times finer.

**Try it.** Set the mode field diameter to 4 µm, Passoni et al.'s smaller fibre. Where does the
optimum move, and what does a grating designed for the standard fibre ($\alpha$ = 0.13 per µm)
couple now?

:::answer
The optimum stays at 80.10 %, at $\alpha$ = 0.3367 per µm, 2.6 times stronger, with the axis
1.49 µm from the start: the grating must empty the guide in a beam 2.6 times narrower. At 0.13 per
µm the beam is too long for the fibre and the overlap falls to 64.18 %.
:::

## 5. How does apodization shape the beam to the fibre?

Make $\alpha$ vary along the grating so that the beam it sends up is the fibre's Gaussian: an
**apodized** grating. It scatters weakly at its start, where the Gaussian is faint and almost all
the power is still in the guide, and ever more strongly further on, where little is left to
scatter. Taillaert et al. vary the grooves' depth or the fill for it (Section 2.4);
[Chen et al. (2010)](https://doi.org/10.1109/LPT.2010.2051220) varied the fill factor along a
grating 200 nm deep in 340 nm of silicon and measured 1.2 dB of loss per interface.

::chart grating-apodization

The rule runs away at the end: as the guide empties, $\alpha$ must grow without bound. A real
grating has a strongest scattering, its deepest or widest grooves, and the chart holds $\alpha$ at
that cap from where it first reaches it, so the guide still empties. With a cap of 0.3 per µm
the overlap is 99.67 %; with no more than the uniform optimum's 0.13 it is still 92.67 %, against
the uniform grating's 80.10 %. On the fibre's axis, where half the power is left, $\alpha$ is
0.151 per µm for a standard fibre at 10°. The chart leaves out the other limit, the weakest
scattering a grating can give: the rule's first grooves scatter almost nothing, and Taillaert et
al.'s optimized grating's narrowest groove is already 30 nm wide (Section 2.4).

### The rule {theory}

Normalize the fibre's Gaussian on the surface, $\int G^2 dz = 1$, and let the guide carry
$P(0) = 1$ into the grating. The grating loses power as $dP/dz = -2\alpha(z)P$, and the power it
sends up from $dz$ is $2\alpha P\thinspace dz$. For that to be $G^2(z)\thinspace dz$, the power left
must be $P(z) = 1 - \int_0^z G^2(t)\thinspace dt$, and so

$$
2\alpha(z) = \frac{G^2(z)}{1 - \int_0^z G^2(t)\thinspace dt} ,
$$

Taillaert et al.'s Eq. 2.1. The field sent up is then $\sqrt{2\alpha P} = G$, and the overlap is 1
for a grating with neither a cap nor a start. Taillaert et al. note that the rule is exact only for
a long grating with small $\alpha$ and use it as an optimization's starting point. The chart marches
$P$ along $z$ on steps of $w/400$, $\alpha = \min(G^2/(2P), \alpha_\text{max})$ and held at
$\alpha_\text{max}$ once it gets there, from 2.5 $w$ before the fibre's axis, where the Gaussian's
power has fallen to $e^{-12.5}$.

### What apodization has reached {research}

In simulation, Taillaert et al.'s optimized nonuniform grating couples 61 % on SOI with 925 nm of
buried oxide, limited by the 35 % sent to the substrate, and 92 % with a two-pair Si/SiO₂ mirror
under it (Section 2.4, Fig. 5). Measured, Chen et al.'s fill-apodized grating lost 1.2 dB per
interface (76 %), and [Zaoui et al. (2014)](https://doi.org/10.1364/OE.22.001277) measured
$-1.08$ dB with a uniform grating and $-0.62$ dB (87 %) with a nonuniform one, both over an
aluminium mirror deposited from the wafer's back, their design's stack 250 nm of silicon on 3 µm
of oxide. photonoxide's `GratingCoupler`
generator draws an apodized layout: its fill is an expression of each tooth's start
(`docs/methods/generators.md`).

## 6. Why does a grating send light down as well as up, and how is more of it sent up?

A shallow grating scatters about as much down, into the buried oxide, as up. The downward half
reaches the silicon substrate, which reflects part of it back up; there it meets the upward half
in step or out of step, depending on the oxide's thickness $h$. The chart draws the share sent up
against $h$, from a model derived below:

::chart grating-directionality

The share swings with a period of half a wavelength across the oxide,
$\lambda/(2 n_\text{ox}\cos\theta_\text{ox})$ = 0.5450 µm at 10° under oxide: 68.41 % up on the
standard 2 µm, the best nearby at 1.95 µm. Times the uniform grating's 80.10 %, the model's
product estimates the uniform coupler's efficiency, and it can be set against Taillaert et al.'s
Table I, computed by full simulations of their 630 nm, 70 nm grating:

| Their stack | Their efficiency | The model's up share | × 80.10 % |
|---|---|---|---|
| air above, 1 µm of oxide | 37 % | 46.27 % | 37.1 % |
| an index-matched cladding (1.46), fibre at 8°, 1 µm | 44 % | 55.86 % | 44.7 % |
| the same, the oxide at its best, 1.45 µm | 53 % | 68.42 % | 54.8 % |
| the same over a two-pair Si/SiO₂ mirror | 79 % | 98.54 % | 78.9 % |

The model's best oxide is 0.855 or 1.40 µm, against their 0.9 or 1.45 µm (Section 2.3). The
model takes the 8° in the cladding and the mirror's layers as quarter waves; the paper gives
neither.

### The sheet between two mirrors {theory}

Put the grating's scattering in a sheet at the grooves' mid-depth, radiating a TE plane wave of
amplitude 1 up and 1 down at the beam's angle; $E$ is continuous across the sheet, so the two are
equal. Let $r_t$ be the reflection, seen from the sheet, of what lies above it (the grooves' upper
half of silicon and the cladding), and $r_b$ of what lies below (the rest of the film, the oxide,
any mirror and the substrate). The waves leaving the sheet upwards, $u$, and downwards, $d$, are
each the sheet's own wave plus the other's reflection:

$$
u = 1 + r_b d, \quad d = 1 + r_t u \quad\Rightarrow\quad
u = \frac{1 + r_b}{1 - r_t r_b}, \quad d = \frac{1 + r_t}{1 - r_t r_b} .
$$

Of $u$, the share $1 - \lvert r_t\rvert^2$ escapes upwards, and of $d$, $1 - \lvert r_b\rvert^2$
downwards, so the share sent up is

$$
D = \frac{\lvert 1 + r_b\rvert^2(1 - \lvert r_t\rvert^2)}{\lvert 1 + r_b\rvert^2(1 - \lvert r_t\rvert^2) + \lvert 1 + r_t\rvert^2(1 - \lvert r_b\rvert^2)} .
$$

Both reflections come from `Multilayer::reflection`, the transfer matrices of
[the multilayer](../docs/methods/multilayer.md), exact, and checked against Fresnel's equations at
one interface and the quarter-wave stack's closed form:

::validation mode/multilayer-fresnel mode/multilayer-bragg

The model leaves out that the grating itself reflects and that its scattering depends on the
stack; it says where the light goes, not how strongly the grooves scatter it.

### Sending more of it up {research}

The buried oxide is the wafer's: Taillaert et al.'s 1 µm was "not optimal for grating couplers,
but customized wafers are not available in small volumes" (Section 3.1). Other ways up change the
grating or the stack:

- **The grating's own layers.** Chen et al. chose a 200 nm etch in 340 nm of silicon so that
  light scattered down by the grooves' walls cancels its reflection from the film's top: a
  directionality of about 0.9 in simulation.
- **An overlay.** [Vermeulen et al. (2010)](https://doi.org/10.1364/OE.18.018278) defined a
  silicon overlay over the film before etching the grating: $-1.6$ dB with a 3 dB band of 80 nm, in a CMOS pilot
  line.
- **A mirror.** A two-pair Si/SiO₂ reflector raises Taillaert et al.'s simulated grating from 53 %
  to 79 %; with a gold mirror under a bonded film they measured 69 % (Section 4.4); Zaoui et al.'s
  backside metal mirror gave $-0.62$ dB.

**Try it.** Set the cladding to 1, the angle to 10° and the oxide to 1 µm, Taillaert et al.'s
measured wafers. Then add mirror pairs.

:::answer
46.27 % up, 37.1 % estimated against their 37 % simulated and 31 % measured. The best oxide nearby
is 0.853 µm. One mirror pair sends most of the downward light back, two nearly all: the dashed
curve, without the mirror, is the reference.
:::

## 7. Why is a grating coupler's band narrow, and what widens it?

A fibre's beam is a bundle of plane waves whose wavevectors along the surface spread by about
$2/w$ around $k n_c\sin\theta$. The grating sends up $k(n_\text{eff} - n_c\sin\theta)$ less a
mismatch $q$ that is zero at the design wavelength and grows away from it, so only near that
wavelength does the beam's bundle meet the grating's:

::chart grating-band

For a standard fibre at 10° over a 70 nm grating in air, the 1 dB band is 41.72 nm wide, against
the "approximately 40 nm" Taillaert et al. measured and simulated (Section 3.2). Under oxide it is
42.98 nm, the 3 dB band 74.48 nm. The chart's curve is only this broadening: the grating's own
resonance and the change of its directionality with the wavelength are left out.

### The band's width {theory}

The mismatch along the surface is $q(\lambda) = 2\pi(n_\text{eff} - n_c\sin\theta)/\lambda - 2\pi/\Lambda$,
zero at the design wavelength, and from Section 2 its slope is
$dq/d\lambda = -2\pi(n_g - n_c\sin\theta)/\lambda^2$. A beam matched in shape but off in phase by
$e^{iqz}$ overlaps the fibre's as

$$
\frac{\left\lvert\int e^{-2z^2/w^2}e^{iqz}dz\right\rvert^2}{\left(\int e^{-2z^2/w^2}dz\right)^2} = e^{-q^2w^2/4} ,
$$

the Gaussian's Fourier transform. It falls by 1 dB where $qw = 2\sqrt{0.1\ln 10} = 0.9597$, so
the full 1 dB width is

$$
\Delta\lambda_\text{1 dB} = \frac{2 \times 0.9597}{w}\thinspace\frac{\lambda^2}{2\pi(n_g - n_c\sin\theta)}
= \frac{0.306\thinspace\lambda^2\cos\theta}{(n_g - n_c\sin\theta)\thinspace w_0} .
$$

The band is inversely proportional to the fibre's mode: halve the mode field diameter and it
doubles. The chart's curve, from the slab indices at each wavelength, and this closed form agree
to within 0.3 %.

### Small beams and the trade-off {research}

[Passoni et al. (2017)](https://doi.org/10.1063/1.4974992) found the two regimes: a grating with
a small footprint has a Gaussian line, the broadening above; a large one a Lorentzian, the width
of the grating's quasi-guided resonance, which stays put however large the beam; between them,
Voigt lines. "The bandwidth increases on reducing the MFD", so they matched gratings to
fibres of 4 and 6 µm mode field diameter. The estimate above gives 128.1 and 79.17 nm for
uniform gratings in their stacks (150 and 120 nm etches, $f = 0.5$, under oxide at 10°), while
their fronts start at their most efficient designs with about 87 and 65 nm (read off their
Fig. 6). A smaller fibre widens the band, and an optimizer trades what remains of the efficiency
for more of it: Section 9.

**Try it.** Set the mode field diameter to 5.2 µm, half the standard fibre's. Then tilt the fibre
by a degree.

:::answer
The 1 dB band doubles, from 42.98 to 86.02 nm (the closed form 85.94 nm). A degree of tilt moves
the band's centre by the "band shift per degree" figure, 11.9 nm per degree of the angle in the
oxide (Section 2): a fibre's angle is a tuning knob, and an error in it shifts the band rather
than narrowing it.
:::

## 8. How do fabrication errors move a grating coupler?

Taillaert et al. made theirs in two lithography and etch steps on 200 mm wafers: the grooves
etched 70 nm deep first, then the guides etched through the 220 nm film, both by 248 nm
lithography at NA 0.63, which makes periods down to 400 nm (Section 3.1). Each dimension errs
differently:

- **The period** is the mask's, drawn exactly: it doesn't drift.
- **The etch depth** is timed, since no film below stops it. photonoxide's fabrication engine
  gives a timed etch to ±5 % of its depth (a textbook's figure:
  [How a chip is made](how-a-chip-is-made.md), Section 4): 70 ± 3.5 nm. From the angle chart in
  air, 66.5 nm leaves the beam at 12.43° and 73.5 nm at 11.17°, against 11.81°; at a fixed fibre
  angle the band moves by about 5 nm either way (0.63° at 8.02 nm per degree).
- **The fill** moves with the lithography's linewidth: a groove 16 nm wider in a 630 nm period
  moves $f$ by 0.025, and the beam by 0.48° (11.33° and 12.29°), about 3.8 nm of band.
- **The buried oxide** is the wafer's, and it sets the directionality (Section 6).

The library has no grating coupler's flow yet. Its rib's timed etch, set 70 nm deep, is the
grooves' step:

::fabrication rib-waveguide{etch_depth=0.07}

The fibre's placement errs too, and Taillaert et al. computed its tolerances for their apodized
coupler (Section 2.5): 1 µm along or across the grating costs less than 0.5 dB, a fibre facet up
to 15 µm above the grating costs nothing measurable and 55 µm costs 0.5 dB, and 2° of tilt across
the guide's plane costs 0.5 dB.

## 9. How far can optimization take a coupler, and what does photonoxide reproduce of Passoni et al.?

Passoni et al. optimized apodized gratings in 220 nm of silicon on a buried oxide over silicon,
under oxide, for fibres of 4 and 6 µm mode field diameter at 10°, varying "the width and position
of each grating element" by a multi-objective particle swarm for two objectives at once, the
efficiency and the 1-dB bandwidth. The result is a **Pareto front**: the most efficient design for
each bandwidth. A 100 nm band "can be achieved with a 47% CE, by simply using a 6 µm MFD, further
improved to 53% if a 4 µm MFD is used" (p. 4).

[optimize_grating_pareto](../examples/optimize_grating_pareto.rs) solves the same problem in 2D
FDFD on 20 × 10 nm cells, its efficiency by reciprocity: the guide's TE₀ is launched into the
grating and the field it sends up is projected on the fibre's beam, a sum of 161 plane waves. Its
search, genoxide's NSGA-II in place of the paper's multi-objective swarm, evaluated 1620
structures where the paper evaluated 2 × 10⁴. All its numbers are 2D:

- **Our fronts** run from 47.2 % at 81 nm to 19.9 % at 161 nm (4 µm) and from 52.9 % at 70 nm to
  25.4 % at 138 nm (6 µm). At 100 nm they couple 43.64 % and 39.77 %, 9.4 and 7.2 points below the
  paper's 53 % and 47 %: **not reproduced.** The two grids, 20 × 10 and 40 × 20 nm, agree to 0.07
  and 0.7 points, so the gap isn't the grid.
- **The paper's own designs**, from its supplement, re-solved by the same model: two 4 µm designs
  either side of 100 nm couple 53.45 % there, against the paper's 53 %. The model reproduces the
  paper's number from the paper's designs, so the gap above is our search's: fewer structures,
  11 grooves for the paper's 13 at 6 µm, and an etch of 150 nm for the 140 of the paper's 4 µm
  files.
- **The bandwidths.** Re-solved on 10 × 5 nm cells with the fibre at its height in the paper's
  files, the paper's designs couple within 0.10 to 0.52 points of its efficiencies at 4 µm, but
  every bandwidth comes out wider than the paper's, by 3.3 to 4.7 nm (3 to 5 %) at 4 µm and 1.6 to
  3.6 nm (2 to 4 %) at 6 µm, beyond the grid's error of at most 0.4 nm. The paper prints its
  indices as constants, 3.47 and 1.44; with silicon's and silica's own dispersion added about
  those values (the catalogue's), every bandwidth narrows by 2.5 to 2.9 % at 4 µm and 3.3 % at
  6 µm, and they end +0.2 to +1.9 % (4 µm) and −1.8 to +0.9 % (6 µm) from the paper's, about its
  files' 1.5 nm steps: **the bandwidths are those of dispersive silicon and silica.**

The run CI checks re-solves the recorded designs on both grids:

::example optimize_grating_pareto

## 10. How does photonoxide compute a grating coupler, and what does a 2D result mean?

- **The device**: a cross-section along the guide, solved by
  [the 2D FDFD solver](../docs/methods/fdfd.md) with $E$ along the grooves (TE), perfectly matched
  layers around it, each cell's permittivity its area's mean. The guide's TE₀ is launched and the
  field projected on the fibre's beam on a line 0.3 µm above the film: the flux form of the
  overlap, so $\eta$ counts power.
- **The charts**: exact slab modes ([the slab](../docs/methods/slab.md)) for the angle and the
  band, exact transfer matrices for the directionality, and the overlap integrals summed on fine
  steps. Each is a model of one factor, and each is set against a published number above.
- **The layout**: the `GratingCoupler` generator draws a taper and its teeth, partly etched or
  through a top layer, with a fill that may vary along the grating
  (`docs/methods/generators.md`). It isn't yet a device of the component
  library: no device page, no fabrication flow.

A 2D section takes the grooves as infinitely long across the guide and the fibre's beam as a
sheet. It leaves out the overlap across the widened guide, the taper that feeds it, and a focusing
grating's curved grooves. A 2D efficiency is therefore the along-the-guide factor of a coupler's,
not the coupler's; Passoni et al.'s optimizations are 2D too.

## 11. History and current research {research}

[Tamir and Oliner (1963)](https://doi.org/10.1049/piee.1963.0044) described waves whose
wavenumbers are complex though nothing absorbs, leaky waves that travel along an interface while
shedding power at an angle. [Tamir and Peng (1977)](https://doi.org/10.1007/BF00882729) analyzed
grating couplers as such waves: the angle of each order, how to keep a single beam, the leakage
rate, and the 80 % ceiling for a Gaussian beam. Thin, high-contrast guides made the grating short:
[Taillaert et al. (2002)](https://doi.org/10.1109/JQE.2002.1017613) coupled a 240 nm GaAs–AlOx
guide to a single-mode fibre held above it, 74 % in 2D simulations and 19 % measured, and by 2006
they had silicon-on-insulator couplers measured at 31 % over a 40 nm band, with the design rules
of Sections 2 to 6. Apodization ([Chen et al., 2010](https://doi.org/10.1109/LPT.2010.2051220)),
overlays ([Vermeulen et al., 2010](https://doi.org/10.1364/OE.18.018278)) and metal mirrors
([Zaoui et al., 2014](https://doi.org/10.1364/OE.22.001277)) took measured losses to 1.2, 1.6 and
0.62 dB, and [Mekis et al. (2011)](https://doi.org/10.1109/JSTQE.2010.2086049) built a CMOS
photonics platform's wafer-scale testing on grating couplers.

::timeline

The trade between efficiency and band is today's question:
[Passoni et al. (2017)](https://doi.org/10.1063/1.4974992) traced it as Pareto fronts and showed
that a smaller fibre mode widens the band, and
[Marchetti et al. (2019)](https://doi.org/10.1364/PRJ.7.000201) review where grating and edge
couplers stand. Open problems named in these papers are bands wide enough for many wavelength
channels at once, coupling both polarizations, and designs that keep their band under the etch
and linewidth errors of Section 8.

Further reading: [Taillaert et al. (2006)](https://doi.org/10.1143/JJAP.45.6071) for the design
of a silicon grating coupler step by step; [Tamir and Peng (1977)](https://doi.org/10.1007/BF00882729)
for the leaky-wave theory; [Marchetti et al. (2019)](https://doi.org/10.1364/PRJ.7.000201) for the
review; and the lessons on [Bragg gratings](bragg-gratings.md), the same periodic structure used
to reflect, and [the slab waveguide](slab-waveguide.md), whose modes set the angle.

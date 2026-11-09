---
title: "Multimode interference couplers"
summary: "A wide guide's modes beat so that its input reappears, whole or in copies, at set lengths: the self-imaging principle and the lengths it sets, 1 × 2 splitters and 2 × 2 couplers, what limits their loss, balance, band and tolerance, and what a 2D model of a silicon MMI gets right and wrong against 3D."
topic: Multimode and polarization devices
level: intermediate
minutes: 45
prerequisites: [directional-coupler]
examples: [mmi_soldano, mmi_liu_poon]
jobs: [mmi-fdfd.yaml, mmi-fdtd.yaml, mmi-1x2-self-imaging.yaml, mmi-2x2-self-imaging.yaml]
methods: [components.md, devices.md, slab.md, fdfd.md, fdtd.md]
validation:
  - components/mmi-beat-length-soldano
  - device/mmi-self-imaging
  - components/mmi-fdfd
  - device/mmi-guided-2d
  - device/mmi-guided-3d
  - device/mmi-best-length-3d
  - device/mmi-best-power-3d
charts: [mmi-field, mmi-modes, mmi-length, mmi-spectrum, mmi-tolerance]
papers:
  - cite: "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977)"
    title: "Mode dispersion in diffused channel waveguides by the effective index method"
    doi: 10.1364/AO.16.000113
    year: 1977
    role: milestone
    note: "The effective index method reviewed and applied to rectangular guides of small and large index difference, at least as accurate as the other approximations of its time: the step that turns an MMI on a chip into the plane its self-imaging is worked out in."
  - cite: "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995)"
    title: "Optical multi-mode interference devices based on self-imaging: principles and applications"
    doi: 10.1109/50.372474
    year: 1995
    role: review
    note: "Self-imaging by a guided-mode propagation analysis: the beat length and the effective width, general, paired and symmetric interference with their image lengths and output phases, imaging quality, reflections and tolerances, and the MMI devices of III-V and silica circuits made by 1994."
  - cite: "R. Halir, P. Cheben, J. M. Luque-González, J. D. Sarmiento-Merenguel, J. H. Schmid, G. Wangüemert-Pérez, D.-X. Xu, S. Wang, A. Ortega-Moñux, Í. Molina-Fernández, Laser Photonics Rev. 10, 1039 (published online 22 November 2016)"
    title: "Ultra-broadband nanophotonic beamsplitter using an anisotropic sub-wavelength metamaterial"
    doi: 10.1002/lpor.201600213
    year: 2016
    role: current
    note: "A 2 × 2 MMI on 220 nm silicon whose multimode section is a sub-wavelength grating, its anisotropy flattening the beat length against the wavelength: three times shorter than their conventional 3.25 × 38.5 µm MMI, measured over more than 300 nm and simulated in 3D over 500 nm, where the conventional one covers under 200 nm."
  - cite: "Z. Liu, J. K. S. Poon, Opt. Continuum 4, 2427 (2025)"
    title: "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD simulations of passive silicon photonic components"
    doi: 10.1364/OPTCON.572107
    year: 2025
    role: current
    note: "Two commercial 3D FDTD codes on gdsfactory's components, a 2 × 2 MMI with S-bends among them: its cross port and excess loss against the resolution and over the band, the numbers mmi_liu_poon is checked against."
---

The 1 × 2 MMI from above, in the symbols the equations below use: a section $W$ wide and $L$
long, the guide that feeds it and the two that take its light, the light inside it, and beneath,
the section's first four modes across its width. The 3D view is the splitter of
`jobs/mmi-fdfd.yaml`, 3 µm wide and 8.55 µm long.

::diagram mmi

A waveguide wide enough to guide many modes turns one spot of light into copies of itself. Each
mode keeps its shape and travels at its own speed; at certain lengths their phases line up again,
and their sum is the input, once or several times across the width. Cut the section at such a
length and put an output guide at each copy: a splitter, a coupler or a combiner set by geometry
alone ([Soldano and Pennings, 1995](https://doi.org/10.1109/50.372474), Sections II and III).

## 1. What does an MMI do, and where is it used?

An MMI is a section that guides several modes, typically three or more, between single-mode
access guides on its two faces; with $N$ inputs and $M$ outputs it is an $N \times M$ MMI
(Soldano and Pennings, Section III). In a circuit it is:

- a **1 × N splitter**, or used backwards an N × 1 combiner: a 1 × 2 fed in its middle sends the
  same share to each output, in phase;
- a **2 × 2 coupler**: the 3 dB coupler of interferometers and switches, its outputs a quarter
  turn apart, as a directional coupler's are;
- an **N × N coupler** whose outputs keep fixed phases: a 4 × 4 MMI's are in quadrature, the
  90° hybrid of coherent and phase-diversity receivers (their Sections IV and VII).

The chart draws the light inside a 3 µm silicon section, 220 nm thick in oxide, fed in its middle
by a 0.8 µm guide, over one beat length $L_\pi$ = 22.60 µm (Section 3), as photonoxide's MMI
model computes it in 2D. The input's single spot spreads across the section, forms two spots at
$3L_\pi/8$ = 8.48 µm and one again at $3L_\pi/4$ = 16.95 µm.

::chart mmi-field

Against a directional coupler, whose outputs go as $\cos^2$ and $\sin^2$ of its length, so that
its 3 dB point is where the split moves fastest with the length, the two outputs in opposite
directions, an MMI's image is a maximum of every output at once: a length error lowers all its
outputs together and keeps them balanced. Its access guides are microns apart, so nothing
couples before the section (Soldano and Pennings, Section VI.C). Against a Y-branch, a 1 × 2 MMI
also splits evenly by symmetry at every wavelength; used as a combiner, both lose the inputs'
antisymmetric part, which an MMI images back onto its input guides as a reflection (their
Section VI.D). photonoxide's 1 × 2 MMI device:

::device mmi-1x2

## 2. What is self-imaging?

The field an access guide launches at the section's face is a sum of the section's guided modes,
each with its own share. Each mode keeps its shape and gains phase at its own rate $\beta_\nu$.
Where every mode has slipped behind the fundamental by a whole number of cycles, the sum is the
input again: a **direct image**. Where the even modes have slipped by whole cycles and the odd
ones by half a cycle more, the odd modes' signs are flipped and the sum is the input mirrored
about the axis: a **mirrored image**. At simple fractions of those lengths the sum is several
copies. An input off the axis excites every mode, and its images form at the lengths of
**general interference**: here a 2.5 µm section fed 0.75 µm below its axis, its $L_\pi$ 15.74
µm, two copies at $3L_\pi/2$ = 23.6 µm and the mirrored image at $3L_\pi$ = 47.2 µm.

::chart mmi-field{width=2.5um, offset=-0.3, span=3.2}

### The images' lengths {theory}

With the section's guided modes $\psi_\nu(y)$ and the input's couplings $c_\nu$, the field is
(Soldano and Pennings's Eqs. 10 and 11, in photonoxide's $e^{i(\beta z - \omega t)}$)

$$
\Psi(y, z) = \sum_{\nu=0}^{m-1} c_\nu\psi_\nu(y)\thinspace e^{i\beta_\nu z}
= e^{i\beta_0 z}\sum_{\nu=0}^{m-1} c_\nu\psi_\nu(y)\thinspace e^{-i(\beta_0 - \beta_\nu)z} .
$$

Section 3 derives $\beta_0 - \beta_\nu \approx \nu(\nu + 2)\pi/3L_\pi$ (their Eq. 7). At
$z = p\thinspace(3L_\pi)$ the factor on mode $\nu$ is $e^{-i\nu(\nu + 2)p\pi}$. $\nu(\nu + 2)$ is
even for even $\nu$ and odd for odd $\nu$ (their Eq. 15), and the modes are even or odd about
the axis, $\psi_\nu(-y) = (-1)^\nu\psi_\nu(y)$ (Eq. 16). For even $p$ every factor is 1 and
$\Psi(y, z) \propto \Psi(y, 0)$; for odd $p$ the factor is $(-1)^\nu$ and
$\Psi(y, z) \propto \Psi(-y, 0)$: direct and mirrored single images at even and odd multiples
of $3L_\pi$ (Eq. 18).

Half-way, at $z = \tfrac{1}{2}(3L_\pi)$: for even $\nu = 2k$, $\nu(\nu + 2)/2 = 2k(k + 1)$ is
even and the factor is 1; for odd $\nu = 2k + 1$, $\nu(\nu + 2)/2 = 2k^2 + 4k + \tfrac{3}{2}$
and the factor is $e^{-3i\pi/2} = i$. With the input's even and odd parts,
$E(y) = [\Psi(y, 0) + \Psi(-y, 0)]/2$ and $O(y) = [\Psi(y, 0) - \Psi(-y, 0)]/2$,

$$
\Psi\left(y, \tfrac{3}{2}L_\pi\right) \propto E(y) + iO(y)
= \frac{1 + i}{2}\Psi(y, 0) + \frac{1 - i}{2}\Psi(-y, 0) :
$$

a direct and a mirrored image, each of amplitude $1/\sqrt 2$, a quarter turn apart in phase
(their Eq. 21), a 2 × 2 3 dB coupler. In general, at $z = (p/N)(3L_\pi)$ with $p$ and $N$
sharing no divisor, $N$ images form, each of amplitude $1/\sqrt N$, with phases set by $p$ and $N$
(their Eqs. 24 to 29, from a Fourier expansion with sine-like modes).

### Where the idea came from {research}

The self-imaging of periodic objects in coherent light goes back to Talbot in the 1830s
([Halir et al., 2016](https://doi.org/10.1002/lpor.201600213), their introduction). Soldano and
Pennings credit self-imaging in a uniform slab to a suggestion of Bryngdahl's in 1973 and its
fuller explanation to Ulrich in 1975, and the guided-mode analysis of this section to Ulrich
(their Sections II and III).

**Try it.** In the chart, move the input to the axis (offset 0). Where do the single and the
two-fold images now form?

:::answer
At $3L_\pi/4$ = 11.8 µm and $3L_\pi/8$ = 5.9 µm on this 2.5 µm section: four times sooner than
general interference's $3L_\pi$ and $3L_\pi/2$, since a centred input excites only the even modes
(Section 4).
:::

## 3. How long is an MMI?

Every image length is a multiple of the beat length $L_\pi = \pi/(\beta_0 - \beta_1)$, the
length over which the two lowest modes slip half a cycle apart (Soldano and Pennings's Eq. 6).
For a section of width $W$ much wider than the wavelength in it,
$L_\pi \approx 4n_rW_e^2/3\lambda_0$: twice the width, four times the length. On 220 nm of
silicon at 1.55 µm, $L_\pi$ is 22.60 µm for a 3 µm section, 89.49 µm for 6 µm and 355.5 µm for
12 µm (exact slab modes in the plane, no grid).

The chart draws how far each mode's phase falls behind the fundamental's over $3L_\pi$, in units
of $\pi$, against $\nu(\nu + 2)$, the law the image lengths assume. The 3 µm section guides 10
modes. The low ones lie close to the parabola, and the higher ones fall further behind it: at
$3L_\pi$ mode 4 lags $1.0\pi$ more than the law, and the last, near its cutoff, $17.9\pi$ more.

::chart mmi-modes

### The beat length and the effective width {theory}

A section of width $W_M$ and index $n_r$ in a cladding of $n_c$ guides modes that are standing
waves across it, with lateral wavenumbers $k_{y\nu}$ and

$$
k_{y\nu}^2 + \beta_\nu^2 = k_0^2 n_r^2, \qquad k_0 = \frac{2\pi}{\lambda_0}
$$

Each mode reaches into the cladding, so it sees a guide slightly wider than $W_M$:
$k_{y\nu} = (\nu + 1)\pi/W_{e\nu}$ (Soldano and Pennings's Eqs. 1 to 3). Taking every mode's
width to be the fundamental's (approximation 1),

$$
W_e = W_M + \frac{\lambda_0}{\pi}\left(\frac{n_c}{n_r}\right)^{2\sigma}\left(n_r^2 - n_c^2\right)^{-1/2}
$$

(Eq. 4), with $\sigma = 0$ for TE across the section and 1 for TM. A silicon guide's TE-like
light, its electric field in the chip's plane, is TM across the section: $\sigma = 1$. For
$k_{y\nu}^2 \ll k_0^2 n_r^2$ (approximation 2, the paraxial one), the binomial expansion gives

$$
\beta_\nu = \sqrt{k_0^2 n_r^2 - k_{y\nu}^2} \approx k_0 n_r - \frac{k_{y\nu}^2}{2k_0 n_r}
= k_0 n_r - \frac{(\nu + 1)^2\pi\lambda_0}{4n_rW_e^2}
$$

(Eq. 5). Then $\beta_0 - \beta_1 = 3\pi\lambda_0/4n_rW_e^2$, so

$$
L_\pi = \frac{\pi}{\beta_0 - \beta_1} \approx \frac{4n_rW_e^2}{3\lambda_0}, \qquad
\beta_0 - \beta_\nu \approx \frac{\left[(\nu + 1)^2 - 1\right]\pi\lambda_0}{4n_rW_e^2}
= \frac{\nu(\nu + 2)\pi}{3L_\pi}
$$

(Eqs. 6 and 7). Two more approximations stand behind every length in Section 2: the section is a
plane (approximation 3), $n_r$ the film's effective index by the effective index method of
[Hocker and Burns (1977)](https://doi.org/10.1364/AO.16.000113), and only the guided modes count
(approximation 4).

photonoxide's MMI drops the first two and keeps the last two: it takes the plane's exact slab
modes and their exact $\beta$ ([the components](../docs/methods/components.md),
[the slab](../docs/methods/slab.md)). On 220 nm of Li's silicon in Malitson's silica at 1.55 µm,
$n_r$ = 2.8475, and a 3 µm section has $W_e$ = 3.0517 µm: Eq. 6 gives 22.8115 µm against the
exact 22.6035, 0.9 % long; at 6 µm 0.2 % and at 12 µm 0.1 %.

::validation components/mmi-beat-length-soldano device/mmi-self-imaging

[mmi_soldano](../examples/mmi_soldano.rs) puts the 1 × 2's and the 2 × 2's first two-fold
images, $3L_\pi/8$ and $L_\pi/2$ from the exact $L_\pi$, beside the paper's closed forms for
sections 3, 6 and 12 µm wide, and prints what the guided modes deliver there:

::example mmi_soldano

[Halir et al. (2016)](https://doi.org/10.1002/lpor.201600213) design a conventional 2 × 2 MMI
3.25 µm wide on 220 nm of silicon. At 1.5 µm they take $n_r \approx 2.87$ and find
$L_\pi \approx 27$ µm from the paraxial formula and about 28 µm from a slab's modes (their
Section 2.1). The chart at their width and wavelength finds $n_r$ = 2.8754, 27.81 µm from Eq. 6
and 27.61 µm from the exact modes: their numbers to the figures they print.

::chart mmi-modes{width=3.25um, wavelength=1.5um}

**Try it.** Widen the section to 12 µm. How far is mode 4 from the parabola at the 1 × 2's image,
$3L_\pi/8$?

:::answer
$0.0079\pi$, against $0.125\pi$ at 3 µm and $0.283\pi$ at 2 µm (the chart's last figure): the
paraxial law holds better as $k_{y\nu}$ shrinks against $k_0 n_r$. The 12 µm section guides 39
modes, though, and its highest still leave the parabola.
:::

## 4. Why are 1 × 2 and 2 × 2 MMIs shorter, and what are their outputs' phases?

Exciting only some of the modes shortens the period. A centred, symmetric input has no overlap
with the odd modes, which are odd about the axis: **symmetric interference**, single images every
$3L_\pi/4$ and $N$ copies, $W/N$ apart, at $3L_\pi/4N$. A 1 × 2 splits at $3L_\pi/8$ into
outputs at $\pm W/4$, four times sooner than general interference. An input centred at
$\pm W/6$ sits where modes 2, 5, 8 … cross zero: **paired interference**, single images every
$L_\pi$ and two copies at $L_\pi/2$, three times sooner. On the 3 µm section the 0.8 µm guide at
$-W/6$ puts 0.00075 of its power into those modes:

::chart mmi-field{offset=-0.16667, span=1.1}

A 2 × 2 6 µm wide, fed at $-W/6$ by a 1.5 µm guide: at $L_\pi/2$ = 44.74 µm the light divides
between the cross and the bar output, 0.4583 and 0.4833, a quarter turn apart (88.8°). Near
$L_\pi$ = 89.49 µm almost all of it has crossed, the mirrored image of an input at $-W/6$ being at
$+W/6$: 0.87 at $L_\pi$ and 0.94 at $0.97L_\pi$, where the high modes' phase errors balance (2D,
exact slab modes, no grid).

::chart mmi-length{kind=2, width=6um, access=1.5um}

### The restricted mechanisms {theory}

**Paired.** $\nu(\nu + 2)$ is a multiple of 3 unless $\nu = 2, 5, 8, \ldots$: for $\nu \equiv 0$
(mod 3) the factor $\nu$ is, for $\nu \equiv 1$ the factor $\nu + 2$ is, and for $\nu \equiv 2$,
$\nu(\nu + 2) \equiv 2$ (Soldano and Pennings's Eq. 30). With $c_\nu = 0$ for those modes, the
phase factors of Section 2 repeat three times sooner: single images at $pL_\pi$ (Eq. 32), $N$
copies at $(p/N)L_\pi$ (Eq. 33). The sine-like modes,
$\sin[(\nu + 1)\pi(y + W_e/2)/W_e]$, cross zero for $\nu = 2$ at $y = \pm W_e/6$, where
$3\pi(\tfrac{1}{2} \pm \tfrac{1}{6})$ is $\pi$ or $2\pi$, and so do modes 5 and 8; each is odd
about those points, so an even input centred there doesn't excite them.

**Symmetric.** For even $\nu = 2k$, $\nu(\nu + 2) = 4k(k + 1)$ is a multiple of 4 (Eq. 34).
With the odd modes left out by a centred input, the factors repeat four times sooner: single
images at $p\thinspace(3L_\pi/4)$ (Eq. 36), $N$ copies at $(p/N)(3L_\pi/4)$, $W_e/N$ apart
(Eq. 37).

**The phases.** A 1 × 2's two images are mirror images of each other and leave in phase. In a
paired 2 × 2 each even mode leads its odd partner (0 and 1, 3 and 4, …) by $\pi/2$ at $L_\pi/2$
(Section V.A), so its two images are a quarter turn apart, as general interference's are by
Eq. 21. The phases decide what an MMI can be used for. A Mach–Zehnder interferometer's combiner
adds its arms' fields: a 2 × 2 whose outputs are in quadrature sends the light to one output or
the other as the arms' phase difference turns, as a directional coupler does
([the Mach–Zehnder lesson](mach-zehnder.md)), where a 1 × 2 used backwards keeps only the
in-phase sum. The splitting's balance sets the extinction: a 0.2 dB imbalance limits it to
−33 dB (Soldano and Pennings, Section VII.B). A 4 × 4's quadrature outputs are a 90° hybrid by
themselves.

### MMIs made by 1995 {research}

Soldano and Pennings collect the first MMI couplers (their Sections IV and V). A general
interference 2 × 2 in InGaAsP, 8 µm wide with four modes: excess losses of 0.4 to 0.7 dB,
imbalances well below 0.1 dB at its 3 dB length $\tfrac{1}{2}(3L_\pi)$ = 250 µm, and −28 dB
extinction in its cross state at $3L_\pi$ = 500 µm, TE and TM at 1.52 µm. Paired 2 × 2s in
silica rib guides: 150 µm long for 3 dB and 240 µm for the cross state, under 0.4 dB of
insertion loss and 0.2 dB of imbalance. A paired 2 × 2 isn't shorter than a general one: its
inputs at $\pm W/6$ need a wider section, and its length grows as $W^2$ (Section VI.A). 4 × 4
hybrids deeply etched in III-V, under a millimetre long: excess losses below 1 dB, imbalances of
0.3 to 0.9 dB, phase errors near 5°. The silicon 2 × 2 of Halir et al. is a general-interference
design: 38.5 µm long, which they set at $3L_\pi/2$, its 1.7 µm access guides 0.3 µm apart.

## 5. What sets an MMI's excess loss and imbalance?

Four things take light from the outputs (Soldano and Pennings, Section VI):

- **Resolution.** An image is a sum of $m$ modes and can be no narrower than about $W_e/m$, the
  lobe of the highest one (their Eq. 38; 0.89 to 1.50 $W_e/m$ by the line-spread function). An
  access guide narrower than that is imaged wider than itself, and the output takes only part.
- **Phase errors.** The modes leave the parabola (Section 3), so at the formula's length the
  higher ones are out of step and the image blurs; a slightly shorter length balances their
  errors (their Section VI.B).
- **The junctions.** The section's guided modes take a share $\sum c_\nu^2$ of the input; the
  rest radiates. On the 3 µm section with a 0.8 µm guide it is 0.99987 (the first chart's
  figures).
- **Reflections** from the far face between the outputs (their Section VI.D).

The chart draws the 1 × 2's outputs against the section's length, 3 µm wide with 0.8 µm guides,
in 2D. At $3L_\pi/8$ = 8.48 µm each output takes 0.4529, an excess loss of 0.429 dB; the most
reaches the outputs at 7.95 µm, 93.75 % of $3L_\pi/8$, losing 0.231 dB. The imbalance is zero at
every length, by symmetry.

::chart mmi-length

**Try it.** Narrow the access guides to 0.5 µm, then set the section to 4 µm and the guides to
1.2 µm. What happens to the loss at the best length?

:::answer
With 0.5 µm guides it rises to 0.727 dB at 7.57 µm (0.917 dB at $3L_\pi/8$): the narrow input
excites more of the high modes, whose phase errors blur the image. The 4 µm section with 1.2 µm
guides loses 0.056 dB at 14.47 µm, 96.5 % of its $3L_\pi/8$: a wide input in a section that
resolves it.
:::

2D FDFD of the same plane keeps what the guided modes leave out, the junctions' radiation and
the far face's reflection. On the splitter of `jobs/mmi-fdfd.yaml`, 3 × 8.55 µm with 0.5 µm
guides and no tapers, the guided modes put 0.3978 into each output and 2D FDFD on its 20 nm grid
0.3808; at its image, 8.48 µm, on a 5 nm grid the two agree within 1.3e-3
([the components](../docs/methods/components.md), [the devices](../docs/methods/devices.md)):

::validation components/mmi-fdfd device/mmi-guided-2d

Access guides widened by tapers where they meet the section are the usual cure: they need fewer
modes to be resolved and relax every tolerance (Section 6). photonoxide's MMI devices take a
`taper_length` and a `port_width`; their 2D model takes the tapers as adiabatic and lossless.

## 6. How wide is an MMI's band, and how much fabrication error does it take?

Whatever moves $L_\pi$ moves the image off the face where the section was cut. Since
$L_\pi \approx 4n_rW_e^2/3\lambda_0$, the wavelength, the width and the film's index each move it.
For the 1 × 2 above, 3 µm wide with 0.8 µm guides and cut at its best length, 7.95 µm, the excess
loss is 0.306, 0.231 and 0.317 dB at 1.50, 1.55 and 1.60 µm, and it and the imbalance stay under
1 dB from 1.43 µm to beyond 1.70 µm, the chart's end (2D, the materials' and the film's
dispersion included, exact slab modes, no grid).

::chart mmi-spectrum

The 2 × 2 6 µm wide with 1.5 µm guides, cut where its weaker output peaks, 43.54 µm, keeps both
under 1 dB from 1.485 to 1.630 µm, 145 nm, its imbalance −0.40 dB and its phase difference 89.1°
at 1.60 µm.

::chart mmi-spectrum{kind=2, width=6um, access=1.5um}

### The tolerances {theory}

Soldano and Pennings treat each image as a Gaussian beam focused at the section's end (their
Section VI.E). A length error then defocuses it, and the error that costs 0.5 dB is about the
beam's Rayleigh range: for a beam whose full $1/e$ amplitude width is $w_0$, a waist radius of
$w_0/2$, in an index $n_r$,

$$
\delta L \approx \frac{\pi n_r (w_0/2)^2}{\lambda_0} = \frac{\pi n_r w_0^2}{4\lambda_0}
$$

(their Eq. 39, the form that gives their example's numbers). Since $L \propto n_rW_e^2/\lambda_0$,
a relative change of $\delta L/L$ comes from

$$
\frac{\delta W}{W} = \frac{\delta L}{2L}, \qquad \frac{\delta\lambda_0}{\lambda_0} = \frac{\delta L}{L},
\qquad \frac{\delta n_r}{n_r} = \frac{\delta L}{L}
$$

(their Eq. 40). Their example, an InP 2 × 2 8 µm wide and 240 µm long with $n_r$ = 3.24 and
3 µm access guides at 1.52 µm: $\delta L \approx 15$ µm, $\delta W \approx 0.25$ µm,
$\delta\lambda \approx 95$ nm, $\delta n_r \approx 0.20$; the width is the dimension to control.
All three tolerances widen with wider access guides and a shorter section.

For the 3 µm silicon 1 × 2 at 7.95 µm, with $w_0$ taken as the 0.8 µm guide's width,
$\delta L = 0.923$ µm and $\delta W = \pm 174$ nm. The chart draws the outputs as every drawn
width comes out $\Delta w$ wider, the section's and the guides' alike, their centres in place:
the loss stays within 0.5 dB of its 0.231 dB from $\Delta w$ = −100 to +140 nm, a range of 240 nm
against the estimate's 349 nm, and is 0.258 and 0.227 dB at −20 and +20 nm. A film 10 nm
thinner or thicker moves it to 0.238 or 0.236 dB.

The 31 % between the two comes in three parts, each measured on the same device (widths and
lengths on 1 nm steps). First, the length: the section cut 0.740 µm short or 0.802 µm long
costs 0.5 dB, a mean of 0.771 µm against the estimate's 0.923, so the Rayleigh range overstates
this guide's tolerance by 20 %. Their $w_0$, "the full $1/e$ amplitude width of the input field",
doesn't close it: the 0.8 µm guide's field across the plane (TM, as the section's) falls to $1/e$
over 0.650 µm, which gives $\delta L$ = 0.610 µm, now too small: for a guide this narrow in
silicon the Gaussian-beam picture is only approximate. Through their Eq. 40 the measured length gives
$\Delta w$ = −140 to +151 nm, 291 nm. Second, a width error does more than move the image: with
only the section's width changed, the guides as drawn, the loss stays within 0.5 dB from −136 to
+126 nm, 262 nm. Third, guides that widen with the section narrow it again, to −104 to +141 nm,
245 nm on 1 nm steps (the chart's 5 nm steps give 240).

::chart mmi-tolerance

### Broadening the band {research}

[Halir et al. (2016)](https://doi.org/10.1002/lpor.201600213) computed their conventional silicon
2 × 2's beat length in 3D falling from 35 to 23 µm across their band, and found it keeping excess
loss and imbalance under 1 dB and the phase error under 5° over less than 200 nm. Their answer is
a section that is a sub-wavelength grating: its anisotropy gives
$L_\pi \approx (4W_e^2/3\lambda)(n_{zz}^2/n_{xx})$, 11.5 µm at 1.5 µm against the conventional
27, and changes far less with the wavelength; it measured within those limits over more than
300 nm, as far as their setup reached, and over 500 nm in 3D simulation.

## 7. What does a 2D model of a silicon MMI miss?

The plane of the effective index method (approximation 3) holds the section's modes nearly but
not exactly. photonoxide solved a 1 × 2 MMI 2 µm wide with 0.5 µm guides in full 3D by FDFD on
three grids of cubic cells, 44, 31.4 and 22 nm, extrapolated
([the devices](../docs/methods/devices.md#in-3d-the-mmi-and-the-y-branch-and-the-2d-models-errors)):

- **At $3L_\pi/8$ = 3.79 µm** from the plane's exact modes, 3D puts 0.410 ± 0.006 into each output,
  an excess loss of 0.86 ± 0.06 dB; the 2D guided modes put 0.4537, 0.42 dB.
- **At each one's best length**, 3D images at 3.34 ± 0.05 µm, 12 % short of $3L_\pi/8$, the 2D
  model at 3.495 µm: 3D then loses 0.221 ± 0.007 dB and the 2D model 0.176 dB.

Two causes. The plane's own modes leave the parabola: its best length is already 8 % short of
$3L_\pi/8$, where mode 4 is $0.283\pi$ off the law (Section 3's chart at 2 µm). And the stack's
even modes beat 3.2 % faster than the plane's, $\pi/(\beta_0 - \beta_2)$ = 3.602 µm against
3.716 µm, while the plane's fourth even mode isn't guided in 3D at all; the guided modes with the
3D modes' indices put the best length at 3.361 µm, within 3D's. What is left, 0.021 ± 0.007 dB,
is beyond the guided modes. A 2D number is an estimate of the device, never its performance:

::validation device/mmi-guided-3d device/mmi-best-length-3d device/mmi-best-power-3d

Coarse 3D grids get the image wrong before they get the guides right: on cells of 100, 80 and
67 nm the 3 µm section's output took 0.160, 0.181 and 0.224, still moving by 0.04 a step, against
0.406 in 2D. The image depends on the high modes' phases, which coarse cells miss.

## 8. What does photonoxide's MMI reproduce?

The 1 × 2 and 2 × 2 MMI devices (`device::StripMmi`) are 6 µm wide with 1.5 µm access guides on
220 nm of silicon by default, 33.56 µm ($3L_\pi/8$) and 44.74 µm ($L_\pi/2$) long. Their 2D model is
the guided modes of Sections 3 to 5 or 2D FDFD of the drawn geometry, both on the plane, and their
3D model is 3D FDFD of the stack ([the devices](../docs/methods/devices.md)).

::device mmi-2x2

[Liu and Poon (2025)](https://doi.org/10.1364/OPTCON.572107) compare two commercial 3D FDTD
codes on gdsfactory's generic 2 × 2 MMI with S-bends: a section 8 µm long, 1.6 µm wide at its
ends and 1.48 µm from 2 to 6 µm, four 500 nm guides tapered to 700 nm over 1 µm with their
centres 0.45 µm from the axis, and S-bends 11 µm long and 1.8 µm across to ports 4.5 µm apart, in
220 nm of silicon in silica, TE₀ into the lower left port (their Section 3.3). It is narrow,
guides few modes and has its inputs far from $\pm W/6$: a design tuned in 3D, which photonoxide's
guided-mode model doesn't take (its guides must be under a third of the width). At 1550 nm the
cross port's TE₀ takes 0.483, 0.486 and 0.489 in Lumerical and 0.479, 0.484 and 0.489 in Tidy3D
at 15, 20 and 25 cells a wavelength, with excess losses of 0.14, 0.13 and 0.12 dB and 0.155,
0.145 and 0.125 dB.

[mmi_liu_poon](../examples/mmi_liu_poon.rs) runs the same device in photonoxide's 3D FDTD
([FDTD](../docs/methods/fdtd.md)): 0.48148 at 15 cells a wavelength (1168 × 277 × 99 cells of
29.7 nm) and 0.48743 at 20 (22.3 nm cells), excess losses of 0.140 and 0.120 dB, and at 1540 and
1560 nm 0.48616 and 0.47574 at 15 cells, against the paper's 0.488 and 0.477 (Lumerical) and
0.4815 and 0.4724 (Tidy3D). The cross port is within the span of both codes' values from 15 to 25
cells, 0.479 to 0.489, the losses within theirs, 0.12 to 0.155 dB, the band's ends between the two
codes', and the band falls with the wavelength as theirs does. The check CI runs is at 6 cells a wavelength (74.3 nm cells), held
to how far the paper's own codes stray there, 0.376 and 0.358:

::example mmi_liu_poon

## 9. How is an MMI made?

As a strip is ([How a chip is made](how-a-chip-is-made.md)): the section and its guides are one
pattern in the silicon film of an SOI wafer, printed in resist and etched through to the buried
oxide in one step. The narrowest feature is the gap between the output guides at the far face:
1.5 µm on the default 1 × 2 (1.5 µm guides at $\pm$1.5 µm), 0.7 µm on the 3 µm one with 0.8 µm
guides. At 193 nm and NA 0.93, 0.7 µm asks a $k_1$ of 3.4 of the lens, where such lithography
prints as a matter of course down to 0.8 (the `fab-resolution` chart of that lesson). Soldano
and Pennings took 1 µm as an achievable open gap for InP splitters, and spread access guides
apart so that poor lithography doesn't blunt the corner between them (their Section V.B and
Fig. 4). The etch's bias widens or narrows the section and the guides together,
the width error of Section 6.

::fabrication mmi-1x2

## 10. History and current research {research}

Self-imaging of periodic objects was described by Talbot in the 1830s, in a uniform slab
suggested by Bryngdahl in 1973 and explained by Ulrich in 1975, as Soldano and Pennings recount.
[Hocker and Burns (1977)](https://doi.org/10.1364/AO.16.000113) set out the effective index
method that turns a channel guide into a slab of its effective index, the step by which Soldano
and Pennings's analysis, and photonoxide's, see an MMI as a plane. By
1994 MMIs were 1 × N splitters up to $N$ = 20, 2 × 2 couplers and 4 × 4 hybrids in III-V
semiconductors, silica and glass, built into coherent receivers, Mach–Zehnder switches and ring
lasers; [Soldano and Pennings (1995)](https://doi.org/10.1109/50.372474) wrote the analysis
this lesson follows.

::timeline

In silicon, MMIs are a few micrometres wide and tens of micrometres long. Current work widens
their band and shortens them by engineering the section itself: a sub-wavelength grating whose
anisotropy flattens the beat length took a 2 × 2 past 300 nm measured
([Halir et al., 2016](https://doi.org/10.1002/lpor.201600213)). Simulating one reliably is
itself measured: two commercial 3D FDTD codes agree on gdsfactory's 2 × 2 to within 2 % from 15
cells a wavelength on ([Liu and Poon, 2025](https://doi.org/10.1364/OPTCON.572107)).

Further reading: Soldano and Pennings's Sections III to VI for the analysis, its mechanisms and
its tolerances; [the components](../docs/methods/components.md) and
[the devices](../docs/methods/devices.md) for how photonoxide computes an MMI in 2D and 3D; and
[the directional coupler](directional-coupler.md) and [the Mach–Zehnder interferometer](mach-zehnder.md),
the coupler an MMI replaces and the interferometer it most often serves.

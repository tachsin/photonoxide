---
title: "Metasurfaces and metalenses"
summary: "A layer of subwavelength pillars that sets the phase of the light passing through it, point by point: the generalized law of refraction as a grating's order, how a pillar sets a phase by guiding or by resonance, the geometric phase of a rotated nanofin, the hyperbolic phase of a metalens and how sampling it costs efficiency, why a metalens is chromatic and how wide an achromatic one's band can be, and what photonoxide computes of them today."
topic: Nanophotonics
level: advanced
minutes: 50
prerequisites: [strip-waveguide, photonic-crystals]
methods: [fdtd.md, slab.md, multilayer.md]
validation:
  - fdtd/tfsf-slab
  - fdtd/bloch-multilayer
  - mode/slab-te-book
charts: [meta-orders, meta-pillar, meta-focus, meta-axis, meta-bandwidth]
papers:
  - cite: "A. Arbabi, Y. Horie, A. J. Ball, M. Bagheri, A. Faraon, Nat. Commun. 6, 7069 (2015)"
    title: "Subwavelength-thick lenses with high numerical apertures and large efficiency based on high-contrast transmitarrays"
    doi: 10.1038/ncomms8069
    year: 2015
    role: milestone
    note: "Amorphous-silicon posts 940 nm tall on fused silica, on an 800 nm hexagonal lattice, cover 0 to 2π of phase at 1550 nm with more than 92 % transmission; micro-lenses focus to spots of 0.57λ, and up to 82 % of the light reaches the focus."
  - cite: "M. Khorasaninejad, W. T. Chen, R. C. Devlin, J. Oh, A. Y. Zhu, F. Capasso, Science 352, 1190 (2016)"
    title: "Metalenses at visible wavelengths: Diffraction-limited focusing and subwavelength resolution imaging"
    doi: 10.1126/science.aaf6644
    year: 2016
    role: milestone
    note: "Titanium-dioxide nanofins 600 nm tall, each rotated by half the phase it must give, make metalenses of NA 0.8 at 405, 532 and 660 nm with focusing efficiencies of 86, 73 and 66 %; read in its arXiv version (1605.02248)."
  - cite: "P. Lalanne, P. Chavel, Laser Photonics Rev. 11, 1600295 (2017)"
    title: "Metalenses at visible wavelengths: past, present, perspectives"
    doi: 10.1002/lpor.201600295
    year: 2017
    role: review
    note: "The history from microwave delay lenses and blazed-binary TiO₂ gratings to today's metalenses, the two ways a pillar sets a phase, and the sampling and coupling that limit a high-NA lens's efficiency."
  - cite: "S. Shrestha, A. C. Overvig, M. Lu, A. Stein, N. Yu, Light Sci. Appl. 7, 85 (2018)"
    title: "Broadband achromatic dielectric metalenses"
    doi: 10.1038/s41377-018-0078-x
    year: 2018
    role: current
    note: "Libraries of amorphous-silicon meta-units with shaped cross-sections set both a phase and its dispersion; lenses keep a near-constant focal length from 1200 to 1650 nm with focusing efficiencies up to 50 %."
  - cite: "M. M. R. Elsawy, S. Lanteri, R. Duvigneau, G. Brière, M. S. Mohamed, P. Genevet, Sci. Rep. 9, 17918 (2019)"
    title: "Global optimization of metasurface designs using statistical learning methods"
    doi: 10.1038/s41598-019-53878-9
    year: 2019
    role: current
    note: "A GaN phase-gradient metasurface on sapphire, four nanoridges in a 1500 nm supercell, optimized in 3D at 600 nm: 88.1 % into the deflected order by an evolution strategy and 88.0 % by Bayesian optimization in about 150 simulations."
  - cite: "F. Presutti, F. Monticone, Optica 7, 624 (2020)"
    title: "Focusing on bandwidth: achromatic metalens limits"
    doi: 10.1364/OPTICA.389404
    year: 2020
    role: current
    note: "A lens that keeps its focus over a band must delay the light at its centre more than at its rim; delay-bandwidth products bound that band for resonant, waveguide and any other meta-atoms, and every published achromatic metalens obeys the bounds. Read in its arXiv version (2001.10899)."
  - cite: "T. W. Hughes, M. Minkov, V. Liu, Z. Yu, S. Fan, Appl. Phys. Lett. 119, 150502 (2021)"
    title: "A perspective on the pathway toward full wave simulation of large area metalenses"
    doi: 10.1063/5.0071245
    year: 2021
    role: current
    note: "A 3D FDTD of a whole metalens of 10,000 TiO₂ nanofins, 100 by 100 wavelengths across and 46 tall, run in under 5 minutes on GPUs, against the approximations that leave out the coupling between neighbouring cells."
  - cite: "M. Chen, R. E. Christiansen, J. A. Fan, G. Işiklar, J. Jiang, S. G. Johnson, W. Ma, O. D. Miller, A. Oskooi, M. F. Schubert, F. Wang, I. A. D. Williamson, W. Xue, Y. Zhou, J. Opt. Soc. Am. B 41, A161 (2024)"
    title: "Validation and characterization of algorithms and software for photonics inverse design"
    doi: 10.1364/JOSAB.506412
    year: 2024
    role: current
    note: "A suite of inverse-design test problems cross-checked between independent codes; its first is a 2D metalens that must focus 450, 550 and 650 nm light to one spot."
---

A metalens in section, in the symbols of this lesson: pillars of height $H$ and width $w$, one
every period $p$, on a substrate; the phase each sets, wrapped into $[0, 2\pi)$, so that their
widths repeat zone by zone; the focus a focal length $f$ above, the rim's ray at $\theta$ to the
axis, $\mathrm{NA} = \sin\theta$; and a nanofin from above, rotated by $\alpha$.

::diagram metalens

A metasurface is a layer of scatterers, each smaller than the wavelength and closer to its
neighbours than one wavelength, that sets the phase (and the amplitude and polarization) of the
light passing through it point by point. A lens built of one, a **metalens**, focuses in a layer
about a wavelength thick, where a glass lens of the same numerical aperture is tens to hundreds of
wavelengths thick.

## 1. What is a metasurface, and how does it differ from a grating or a lens?

A glass lens delays the light by its thickness: a thickness $t(r)$ of index $n$ adds the phase
$(n - 1)k_0 t(r)$, $k_0 = 2\pi/\lambda$. To focus, the centre must delay the light by the extra
path from the rim to the focus, $\sqrt{R^2 + f^2} - f$. For
[Khorasaninejad et al.'s](https://doi.org/10.1126/science.aaf6644) lenses, $f$ = 90 µm and
NA 0.8, that is $f(1/\sqrt{1 - \mathrm{NA}^2} - 1)$ = 60 µm, 113 wavelengths at 532 nm, and a
glass of $n = 1.46$ needs 130 µm more thickness at its centre than at its rim to supply it.

A phase is only defined modulo $2\pi$, so the delay can be **wrapped**: wherever it passes a whole
wavelength, start again from zero. A Fresnel lens and an échelette grating do this with a sawtooth
of thickness; [Lalanne and Chavel (2017)](https://doi.org/10.1002/lpor.201600295) explain why
neither works at high NA: each tooth's slope shadows its neighbour and wastes light into spurious
orders, worse the shorter the period (Section 2). A metasurface writes the same wrapped phase with
pillars of one height whose widths, or orientations, vary: every pillar a separate element, no
sloped faces.

Two lengths set it apart from an ordinary grating:

- **The pillars' lattice diffracts nothing.** A periodic array of period $p$ sends light into
  orders $n_t\sin\theta_m = n_i\sin\theta_i + m\lambda/p$ (Section 2), and only the zeroth leaves
  when $p \lt \lambda/(n_t + n_i\lvert\sin\theta_i\rvert)$. Below that period the lattice is
  invisible to the far field; only the pattern of phases across it acts.
- **The pattern is free.** Across many pillars the phases can follow any profile: a linear ramp
  deflects (Section 2), a hyperbola focuses (Section 5), a helix makes a vortex. A metasurface is
  a phase mask written in pillars, $t(x, y) = A(x, y)\,e^{i\varphi(x, y)}$, as thin as one pillar.

## 2. How does a phase gradient bend light?

Let a plane wave arrive at $\theta_i$ from a medium $n_i$ on a surface that multiplies it by
$e^{i\Phi(x)}$, and leave into $n_t$. The incident field along the surface varies as
$e^{ik_0 n_i\sin\theta_i\,x}$; just after the surface it varies as
$e^{i(k_0 n_i\sin\theta_i\,x + \Phi(x))}$. Where $\Phi$ changes slowly, its local wavevector along
the surface is $k_0 n_i\sin\theta_i + d\Phi/dx$, and the transmitted wave must have the same:

$$
n_t\sin\theta_t - n_i\sin\theta_i = \frac{\lambda}{2\pi}\frac{d\Phi}{dx} .
$$

With no gradient it is Snell's law. With one, light leaves at an angle Snell's law doesn't give:
the **generalized law of refraction**, the form in which Lalanne and Chavel date the field's
recent growth to Yu et al.'s nanoantenna arrays of 2011 (Section 11).

### It is a grating's first order {theory}

A gradient that keeps bending light the same way must wrap: $\Phi = 2\pi x/\Gamma$ modulo $2\pi$, a
**supercell** of length $\Gamma$ repeated. That is a grating of period $\Gamma$, whose orders leave
at

$$
n_t\sin\theta_m = n_i\sin\theta_i + m\frac{\lambda}{\Gamma} ,
$$

and the generalized law is its order $m = +1$. The ideal ramp $e^{i2\pi x/\Gamma}$ has one Fourier
component only, so it sends every bit of the light into that order: a perfectly blazed grating.
Pillars write the ramp in $N$ steps, one a period $p = \Gamma/N$, the $k$-th holding
$\varphi_k = 2\pi k/N$. Its Fourier coefficient of order $m$ is

$$
c_m = \frac1N\sum_{k=0}^{N-1} e^{i2\pi k/N}\,e^{-i2\pi mk/N}\;e^{-i\pi m/N}\,\mathrm{sinc}\frac mN ,
\qquad \mathrm{sinc}\,u = \frac{\sin\pi u}{\pi u} ,
$$

and the sum is 1 when $m - 1$ is a multiple of $N$ and 0 otherwise. So the staircase sends
$\mathrm{sinc}^2(m/N)$ of the power into the orders $m = 1, 1 \pm N, 1 \pm 2N, \ldots$ and none
into the rest:

::chart meta-orders

At the chart's start, Elsawy et al.'s supercell of four pillars in 1500 nm at 600 nm
([Elsawy et al., 2019](https://doi.org/10.1038/s41598-019-53878-9)), order +1 leaves normal
incidence at 23.58° into air, $\sin\theta = \lambda/\Gamma = 0.4$, and the four-step staircase sends
it 81.06 % of the power, the rest into orders −3 (9.01 %) and +5 (3.24 %), and less into the
farther ones. Exact, no grid.

### Its limits {theory}

- **The anomalous order can vanish.** When $\lvert n_i\sin\theta_i + \lambda/\Gamma\rvert \gt n_t$,
  order +1 is evanescent: no transmitted beam satisfies the generalized law. In the chart, tilt the
  incidence towards the gradient and watch order +1 end.
- **The staircase is a thin, scalar model.** It assumes each pillar transmits its phase and nothing
  else, whatever its neighbours. Real pillars are as tall as a wavelength and couple to one another,
  and a design can do better than the staircase's share: Elsawy et al. optimized four GaN
  nanoridges, 666 to 800 nm tall on sapphire, in a 3D solver, and sent 88.1 % of the light into the
  deflected order with an evolution strategy and 88.0 % with Bayesian optimization, against the
  four-step staircase's 81.06 %. photonoxide does not reproduce their 88 %: it needs 3D periodic
  cells, which [the roadmap](../ROADMAP.md) plans for 0.14 (Section 9).

**Try it.** At $N = 2$, how much of the light goes into order +1, and where does the rest go?

:::answer
$\mathrm{sinc}^2(1/2) = 4/\pi^2$ = 40.53 %, and as much into order −1, since $m = -1$ is $1 - N$:
two pillars a period make a symmetric grating that can't tell left from right. The rest, 18.94 %,
goes into the orders $\pm3, \pm5, \ldots$. A lens's rim, sampled by about two pillars a zone, works
this way (Section 6).
:::

## 3. How does a pillar set a phase?

A pillar of a high index standing in a low one is a short waveguide. Its fundamental mode crosses
the height $H$ with the effective index $n_\text{eff}$, the surrounding medium with its own index,
so the pillar delays the light by about

$$
\varphi \approx (n_\text{eff} - 1)\,k_0 H
$$

against air, and $n_\text{eff}$ rises from about 1 for a thin pillar, whose mode lives mostly
outside it, towards the material's index for a wide one (as in the
[strip waveguide](strip-waveguide.md)). A library of widths then covers $0$ to $2\pi$ if
$H(n_\text{eff,max} - n_\text{eff,min}) \ge \lambda$. Lalanne and Chavel call this the
propagation-delay, or waveguide, kind of metasurface; the other kind uses resonances (below).

[Arbabi et al. (2015)](https://doi.org/10.1038/ncomms8069) found such a library in amorphous
silicon on fused silica at 1550 nm: posts 940 nm tall, 200 to 550 nm across, on an 800 nm
hexagonal lattice, transmitting more than 92 % while their phases span 0 to $2\pi$ (p. 2, Fig. 1d).
They found the posts' phase depends mostly on the diameter and hardly on the lattice, the light
concentrated inside the posts, which "behave as weakly coupled low-quality factor resonators"
(p. 2).

The chart computes the same thing in 2D: silicon ridges on silica, lit from the silica, the
transmitted zeroth order's phase and power against the ridge's width, by FDTD of one period
between periodic sides, beside the waveguide picture's phase from each ridge's exact slab mode:

::chart meta-pillar

At the chart's start, ridges 1 µm tall, one every 600 nm, at 1.55 µm, the phase climbs by
1.38 × 2π from the narrowest ridge, 30 nm, to the widest, 540 nm: a library of widths covers
$2\pi$. The waveguide picture follows it to within 0.035 × 2π from 80 to 380 nm wide and departs by
up to 0.123 × 2π near 490 nm. The transmittance is not flat: it dips to 74 % at 100 nm, 52 % at
220 nm and 23.8 % at 490 nm, where the ridge's mode, reflected at the ridge's two ends, resonates;
above 466 nm a second even mode, TE₂, is guided too ($w = \lambda/\sqrt{n^2 - 1}$, its cutoff),
and one mode's picture can't hold. A library keeps the widths that transmit well. The numbers are
2D FDTD on 20 nm cells; on 10 nm cells the phase at 300 nm moves by 0.020 rad and the transmittance
by 1.4 %. A 2D ridge is infinitely long, so these are not Arbabi et al.'s posts' numbers, which need
a 3D cell (Section 9).

### How the chart computes a unit cell {theory}

One period of the ridges is the whole problem when every ridge is the same: the field one period
on is the field here, an `Edges::Bloch` side with $k = 0$ in
[photonoxide's FDTD](../docs/methods/fdtd.md). A sheet of current across the period in the silica
launches a pulse of plane wave upwards; a line 0.6 µm above the ridges records E's transform at
the wavelength. Averaged over one period, every diffraction order but the zeroth cancels, since
order $m$ varies as $e^{i2\pi mx/p}$; the period is below $\lambda/n_\text{silica}$, so no other
order travels anyway. Dividing by the same run with no ridge leaves the ridge's own transmission:
its phase against air's, and its power against the bare silica face's 96.70 %. Each cell's
permittivity is the mean over it, which is exact for E along the ridges, parallel to every
interface.

A ridge as wide as the period is a film, whose transmission the transfer matrices give exactly; a
unit test holds the chart's FDTD to them, and checks the phase moves by less than 5 % of $2\pi$
from 20 nm to 10 nm cells. The FDTD's plane waves and periodic sides are checked against Airy's
formula and the transfer matrices in the validation report, the slab modes against a textbook's:

::validation fdtd/tfsf-slab fdtd/bloch-multilayer mode/slab-te-book

### Resonant pillars {theory}

The waveguide picture leaves out the pillar's ends: the mode reflects at the top and the bottom,
and a pillar is a weak Fabry–Pérot resonator, which the ripples of the FDTD curve show. Short
pillars rely on resonance alone. A single resonance cannot cover $2\pi$: model a thin layer whose
pillars scatter as one electric dipole with resonance $\omega_0$ and width $\gamma$, radiating
equally forwards and backwards. Its reflection is $r = -i\gamma/(\omega - \omega_0 + i\gamma)$ and,
lossless, its transmission
$t = 1 + r = (\omega - \omega_0)/(\omega - \omega_0 + i\gamma)$: as $\omega$ crosses
$\omega_0$, $\arg t$ falls from $+\pi/2$ to $-\pi/2$, a range of $\pi$, and $\lvert t\rvert$
passes through zero on the way. Add a magnetic dipole at the same frequency and width, which
radiates backwards with the opposite sign: the reflections cancel and
$t = 1 + 2r = (\omega - \omega_0 - i\gamma)/(\omega - \omega_0 + i\gamma)$, of modulus 1, whose
phase sweeps the full $2\pi$. That is why resonant metasurfaces combine "two resonances, each
covering a standard phase range of π" (Lalanne and Chavel, Section 3), and why
Arbabi et al. found that posts 220 nm tall could not reach $2\pi$ by their diameter alone (p. 2).
Set the chart's height to 220 nm to see the same in 2D. Resonant scattering by particles is the
subject of [Surface plasmons and Mie scattering](plasmonics.md).

## 4. How does a rotated nanofin set a geometric phase? {theory}

A rectangular fin is birefringent: light polarized along its length and across it travel through
it with different effective indices, so a fin aligned with $x$ transmits with the Jones matrix
$T_0 = \operatorname{diag}(t_x, t_y)$. Rotate it by $\alpha$:

$$
T(\alpha) = R(\alpha)\,T_0\,R(-\alpha)
= \frac{t_x + t_y}{2}\,I + \frac{t_x - t_y}{2}\begin{pmatrix}\cos2\alpha & \sin2\alpha\\ \sin2\alpha & -\cos2\alpha\end{pmatrix} .
$$

Light of one circular polarization, $\mathbf e_+ = (1, i)/\sqrt2$, meets the second matrix as
$(\cos2\alpha + i\sin2\alpha,\ \sin2\alpha - i\cos2\alpha)/\sqrt2 = e^{i2\alpha}(1, -i)/\sqrt2$, so

$$
T(\alpha)\,\mathbf e_+ = \frac{t_x + t_y}{2}\,\mathbf e_+ + \frac{t_x - t_y}{2}\,e^{i2\alpha}\,\mathbf e_- .
$$

The part converted to the other circular polarization, $\mathbf e_- = (1, -i)/\sqrt2$, carries the
phase $2\alpha$: the **geometric**, or Pancharatnam–Berry, phase. It depends on the rotation alone,
not on the fin's size or the wavelength, so every fin of a lens can be the same, only turned. Its
share of the power is $\lvert t_x - t_y\rvert^2/4$; for a lossless fin, $t_y = t_x e^{i\delta}$ with
$\lvert t_x\rvert = 1$, it is $\sin^2(\delta/2)$, all of the light for a half-wave plate,
$\delta = \pi$. The rest, $(t_x + t_y)/2$, passes in its own polarization with no phase pattern.

Khorasaninejad et al. built their metalenses so: amorphous TiO₂ fins 600 nm tall, each rotated by
half the phase the lens needs at its place ($\varphi = 2\alpha$, their Eqs. 1 and 2), the fin
for 532 nm 250 by 95 nm on a 325 nm lattice, for 660 nm 410 by 85 nm on 430 nm, for 405 nm 150 by
40 nm on 200 nm. Their simulations of periodic fins converted up to 95 % of the light, and the
lenses focused 86, 73 and 66 % of it at 405, 532 and 660 nm; the phase works only on circularly
polarized light, and the focus is in the other circular polarization.

**Try it.** A fin's retardance is $\delta = \pi/2$, a quarter-wave plate. What share of circularly
polarized light does it put into the lens's focus?

:::answer
$\sin^2(\pi/4)$ = 50 %; the other half passes in its own polarization, unfocused, and spreads
behind the lens as background.
:::

## 5. What phase does a lens need, and how finely must pillars sample it?

A lens brings every ray from the plane wave to one point in step: the phase added at radius $r$
plus the path from there to the focus must be the same everywhere,
$\varphi(r) + k_0\sqrt{r^2 + f^2} = \varphi(0) + k_0 f$, so

$$
\varphi(r) = -\frac{2\pi}{\lambda}\left(\sqrt{r^2 + f^2} - f\right) ,
$$

the hyperbolic profile Khorasaninejad et al. (Eq. 1) and Presutti and Monticone (Eq. 1) write. Its
rim at $r = R$ sends its ray at $\theta$ to the axis, $\tan\theta = R/f$, and in air
$\mathrm{NA} = \sin\theta = R/\sqrt{R^2 + f^2}$. Its gradient,
$d\varphi/dr = -k_0\,r/\sqrt{r^2 + f^2} = -k_0\sin\theta(r)$, is Section 2's deflector at every
radius, bending its ray by $\theta(r)$. Wrapped, the profile is a grating whose local period, the
zone width, is

$$
\Lambda(r) = \frac{2\pi}{\lvert d\varphi/dr\rvert} = \frac{\lambda}{\sin\theta(r)} ,
$$

$\lambda/\mathrm{NA}$ at the rim, the narrowest. A lattice of period $p$ samples the rim's zone
with $\lambda/(\mathrm{NA}\,p)$ pillars, and at fewer than two the sampled ramp can no longer tell
order +1 from −1 (Section 2's $N = 2$): $p \le \lambda/(2\mathrm{NA})$. Lalanne and Chavel count
2.5 pillars in the rim zone of Khorasaninejad et al.'s 405 nm lens and 2.0 in the 532 nm one
(Section 4.3); the 660 nm lens's 430 nm lattice gives 1.9.

The chart focuses a cylindrical lens, a 2D one, whose phase is held over each pillar's cell and
rounded to $N$ levels, against the exact profile:

::chart meta-focus

At the chart's start, a lens of NA 0.6 and $f$ = 20 µm, 30 µm across, on 600 nm pillars with
8 levels, focuses to a peak 0.900 of the exact profile's, below the 8-level staircase's 0.950: its
rim zones hold 4.3 pillars each, and holding one phase across each 600 nm cell costs more (with
every phase allowed, set the levels to 0, the peak is 0.935). Its spot is 0.714λ wide at half
maximum, the exact profile's 0.704λ, against the paraxial slit's $0.443\lambda/\mathrm{NA}$ =
0.738λ. The chart is scalar and 2D, and leaves out each pillar's own transmission (Section 3).

### The focal spot {theory}

A 2D (cylindrical) lens of half-width $R$ focuses, paraxially, the Fourier transform of its
aperture, $\int_{-R}^{R} e^{-ik_0 xx'/f}dx' = 2R\,\mathrm{sinc}(2Rx/\lambda f)$, whose intensity
falls to half at $2Rx/\lambda f$ = 0.4429: a full width at half maximum of $0.443\lambda/\mathrm{NA}$
for small NA. A circular lens's focus is the Airy pattern $(2J_1(v)/v)^2$,
$v = k_0\mathrm{NA}\,\rho$, at half maximum at $v$ = 1.616: $0.514\lambda/\mathrm{NA}$. Arbabi et
al. measured 0.57λ from their highest-NA lens (p. 4); Khorasaninejad et al. measured 450 nm at
660 nm with NA 0.8, 0.68λ, where the Airy pattern's is 0.64λ.

## 6. How efficient can a metalens be?

A metalens loses light in four ways, each visible in what it computes or measures:

- **Rounding the phase.** A library of $N$ phases between 0 and $2\pi$ is Section 2's staircase in
  every zone, and sends $\mathrm{sinc}^2(1/N)$ of the light to the focus: 81.06 % for 4 levels,
  94.96 % for 8, 98.72 % for 16, the 81 and 99 % Lalanne and Chavel quote (Section 4.1).
- **Sampling the rim.** Near the rim a zone holds few pillars, and their steps are coarse whatever
  the library: Arbabi et al. simulated lenses of rising NA and found their transmission and focusing
  efficiency falling, "due to the undersampling of the phase profiles of high-NA lenses, which
  rapidly vary close to the lens circumference" (p. 3).
- **The pillars themselves.** Each pillar's transmittance (Section 3's chart), and its coupling to
  neighbours unlike itself: the phase a pillar gives in a periodic array of its twins is not quite
  the phase it gives among different ones. Lalanne and Chavel's remedy is high index contrast, so
  that each pillar's field dies before it reaches its neighbour (Section 4.1).
- **Fabrication.** Arbabi et al.'s measured focusing efficiencies were about 10 % below their
  simulations, 3 % lost to reflection at the substrate's back and 7 % to scattering by the posts'
  etched roughness (p. 4).

Efficiencies are defined differently from paper to paper. Arbabi et al. count the light through a
disc of three times the spot's FWHM in radius in the focal plane, against the light incident on the
lens (p. 3); Lalanne and Chavel note that Khorasaninejad et al.'s efficiencies are relative to light
that has already lost about 4 % to reflection (their Table 1). With that, Arbabi et al.'s lenses
reached 0.57λ spots and, at lower NA, more than 82 % efficiency (p. 4).

**Try it.** In the chart, set the levels to 4 and the period to 100 nm, then raise the period to
1200 nm. What happens to the peak, and why?

:::answer
On 100 nm pillars the peak is 0.820 of the exact profile's, close to the four-level staircase's
0.811. On 1200 nm pillars it falls to 0.710: the rim's zones hold 2.15 pillars each, and, as in
Section 2 at $N = 2$, so few steps send light into other orders besides the focus.
:::

## 7. Why does a metalens focus each colour elsewhere?

A pillar's phase is a delay: in the waveguide picture
$(n_\text{eff} - 1)k_0H$, proportional to $1/\lambda$ if the indices don't change. A lens designed
at $\lambda_0$ wraps its profile there, each pillar holding $\psi_0 = \varphi_0 \bmod 2\pi$, and at
another wavelength holds $\psi = \psi_0\lambda_0/\lambda$. Inside a zone the gradient of $\psi$
bends light by the same angle as before; but the jumps between zones are now
$2\pi\lambda_0/\lambda$, not $2\pi$, and the zones themselves, a grating of local period
$\Lambda(r) = \lambda_0/\sin\theta_0(r)$, send their order $m$ to
$\sin\theta = m\lambda/\Lambda = m(\lambda/\lambda_0)\sin\theta_0$. Order 1 focuses, paraxially, at

$$
f(\lambda) = f\,\frac{\lambda_0}{\lambda} ,
$$

and receives $\mathrm{sinc}^2(1 - \lambda_0/\lambda)$ of the light: the staircase's coefficient with
its steps no longer summing to $2\pi$. A glass lens's focal length moves only as its index does,
$1/f \propto n(\lambda) - 1$, a few parts in a thousand over such a band.

::chart meta-axis

At the chart's start, NA 0.3 and $f$ = 150 µm, light of 1.45 µm focuses at 158.8 µm, where
$f\lambda_0/\lambda$ = 160.3 µm, and the design wavelength itself at 149.2 µm: both peaks lie about
1 % short of the paraxial foci (the axis is sampled every 0.27 % of $f$), as a lens only
$R^2/\lambda f \approx 10$ Fresnel zones wide focuses its peak a little in front of $f$. The
second wavelength's peak is 0.993 of the design's, with $\mathrm{sinc}^2(1 - \lambda_0/\lambda)$
= 0.984 of the light in its focus.

Arbabi et al. measured the same shift: tuning the laser from 1550 to 1450 nm moved the focus of
their lens of $d$ = 175 µm and 400 µm diameter from about 175 to about 195 µm (p. 4).
$f\lambda_0/\lambda$ gives 187.1 µm; the chart set to NA 0.75, $f$ = 175 µm and 1.45 µm gives
191.8 µm, the zones' larger angles moving the focus further than the paraxial formula. **Their
195 µm is not reproduced**: ours is 1.6 % short, and their lens is circular, lit by a fibre's
diverging beam through 500 µm of silica and designed by their own rigorous method (p. 2), none of
which the chart models; the paper gives the focus only as "∼195 µm". At that NA the 1.45 µm peak
is only 0.614 of the design's: away from its wavelength a high-NA metalens is aberrated as well as
defocused. Khorasaninejad et al.'s 532 nm lens focused 660 and 405 nm light
to spots of 720 and 590 nm, broadened by the same chromatic aberration.

## 8. How wide can an achromatic metalens's band be? {research}

A lens whose focal length doesn't move must give every frequency the hyperbolic phase of its own,
$\varphi(r, \omega) = -(\omega/c)(\sqrt{r^2 + F^2} - F)$ plus any constant, so its derivative in
$\omega$, the group delay, must vary across the lens:
$\Delta T(r) = (\sqrt{F^2 + R^2} - \sqrt{F^2 + r^2})/c$ more at $r$ than at the rim
([Presutti and Monticone, 2020](https://doi.org/10.1364/OPTICA.389404), the equation before Eq. 4).
The centre needs the most,

$$
\Delta T_\text{max} = \frac Fc\left(\sqrt{1 + (R/F)^2} - 1\right) = \frac Fc\left(\frac{1}{\sqrt{1 - \mathrm{NA}^2}} - 1\right)
$$

in air (their Eqs. 4 and 5): the light through the centre must wait for the light from the rim. A
structure can delay a band $\Delta\omega$ by $\Delta T$ only if $\Delta T\,\Delta\omega \le \kappa$
(Eq. 3), so

$$
\Delta\omega \le \frac{\kappa c}{F}\,\Theta, \qquad \Theta = \frac{\sqrt{1 - \mathrm{NA}^2}}{1 - \sqrt{1 - \mathrm{NA}^2}}
$$

(Eq. 6), with $\kappa$ = 2 for a single resonance (Eq. 7), $2\pi(L/\lambda_c)\Delta n$ for a
waveguide of length $L$ and index contrast $\Delta n$ (Tucker et al.'s, Eq. 8), and
$(\pi/\sqrt3)(L/\lambda_c)\eta_\text{max}$ for any structure whose permittivity contrast is at most
$\eta_\text{max}$ (Miller's, Eq. 9). The chart draws the three bounds, Eqs. 10 to 12:

::chart meta-bandwidth

It opens on Shrestha et al.'s lens d₁ of Presutti and Monticone's Table 1
([Shrestha et al., 2018](https://doi.org/10.1038/s41377-018-0078-x)): NA 0.24, $F$ = 200 µm,
amorphous-silicon meta-units 800 nm tall ($\varepsilon$ = 13.5, $n$ = 3.674), achromatic from 1300
to 1650 nm. The bounds come out at 99.58, 460.2 and 621.0 rad/ps, Table 1's 99.6, 460 and 621. The
lens's own band, 307 rad/ps, is three times the single resonance's bound, so its meta-units can't
be single resonators, and inside Tucker's and Miller's, as every lens Presutti and Monticone
compared is. The bounds fall steeply with NA: the delay the centre needs grows as
$1/\sqrt{1 - \mathrm{NA}^2} - 1$ and diverges at NA = 1. The chart's numbers are exact closed
forms.

Shrestha et al. reached their bands with libraries of meta-units of shaped cross-sections, each
giving a phase and its slope against frequency, 800 and 1400 nm tall; their lenses kept a
near-constant focal length from 1200 to 1650 nm with focusing efficiencies up to 50 % (abstract).

**Try it.** Keep the lens's focal length, thickness and index and raise its NA to 0.85. By what
factor does Tucker's bound fall?

:::answer
From 460.2 to 15.43 rad/ps, 29.8 times, a band of 17.3 nm around 1.454 µm: the delay the centre
needs grows from 20.09 fs to 599.3 fs. Shrestha et al.'s high-NA lens M3, 100 µm across, has
a focal length of 30 µm against 200, which raises every bound by the ratio of the focal lengths,
and they corrected it over a narrower band, 1200 to 1400 nm.
:::

## 9. How is a metasurface computed, and what does photonoxide compute today?

Three levels of model, each with its cost:

- **The unit cell.** One pillar between periodic sides gives its transmission in an array of its
  twins, as Section 3's chart and Arbabi et al.'s and Khorasaninejad et al.'s libraries do. It is
  exact for a periodic array, and one run a pillar.
- **The locally periodic approximation.** Each pillar of a lens is given its unit cell's
  transmission, and the field after the lens is propagated to the focus, as Sections 5 to 7's charts
  do with ideal phases. It leaves out the coupling between unlike neighbours, worst where the
  phase changes fastest, at the rim. [Hughes et al. (2021)](https://doi.org/10.1063/5.0071245)
  name it, and recording only the near field after the lens, as the usual ways to keep a lens's
  simulation small, and note that they "neglect cell-to-cell coupling effects".
- **The whole lens.** Hughes et al. ran a 3D FDTD of a lens of 10,000 TiO₂ nanofins after
  Khorasaninejad et al.'s, 100 by 100 wavelengths across and 46 tall including its focal length,
  at λ/18 with subpixel smoothing, in under 5 minutes on GPUs.

What photonoxide computes today:

- **2D unit cells** by [FDTD](../docs/methods/fdtd.md) between periodic or Bloch sides, with
  plane waves and frequency-domain monitors: Section 3's chart, on 20 nm cells, checked against the
  transfer matrices for a film. The same calls take a 3D grid; no example or validation case runs a
  3D unit cell yet.
- **Waveguide pictures** from the exact [slab modes](../docs/methods/slab.md), as Section 3's
  chart draws them; the full-vector mode solver of [the strip waveguide](strip-waveguide.md) takes
  a post's cross-section too, though no chart or example uses it for one yet.
- **Scalar propagation** of a phase profile, Sections 5 to 7's charts, summed in the studio.

What it doesn't yet: RCWA, the method of choice for periodic layers, planned for 0.12; metasurfaces
with the locally periodic approximation, planned for 0.14 ([the roadmap](../ROADMAP.md)); 3D
periodic cells at scale, for which Elsawy et al.'s metasurface is the planned example
(`optimize_metasurface`, 0.14); and FDTD on the GPU with Bloch phases and plane waves, which its
GPU kernels don't have yet. Inverse design of metalenses is one of the test problems of
[Chen et al. (2024)](https://doi.org/10.1364/JOSAB.506412): a 2D lens of permittivity between 1 and
5.76 that must focus 450, 550 and 650 nm light to one spot, its worst wavelength's intensity
maximized.

## 10. How is a metasurface made?

A metasurface is one lithography and one etch, the steps that make a strip waveguide
([How a chip is made](how-a-chip-is-made.md)), with the pillars drawn in the mask where the strip's
outline is. The strip's flow in photonoxide's fabrication engine shows them:

::fabrication waveguide

The pillars are taller and narrower than a strip, and three processes in this lesson's papers make
them:

- **Etched posts.** Arbabi et al. deposited 940 nm of hydrogenated amorphous silicon on 500 µm of
  fused silica, wrote the posts by electron beam in ZEP520A resist, lifted off a 70 nm alumina hard
  mask, etched the silicon in a C₄F₈ and SF₆ plasma and removed the mask in ammonium hydroxide and
  hydrogen peroxide (p. 6).
- **Fins grown into resist.** Khorasaninejad et al. wrote the fins as holes in a resist as thick as
  the fins, 600 nm, filled them by atomic layer deposition of amorphous TiO₂ (at least half the fin's
  width, so the walls meet), etched away the film left on top and stripped the resist: walls of about
  90°, which etching a 40 nm fin 600 nm tall would not give.
- **Blazed binary TiO₂.** Lalanne et al.'s gratings of 1998 and 1999, as Lalanne and Chavel recount,
  were written by electron beam, lifted off and etched by chemically assisted reactive ion etching,
  their narrowest pillars 90 nm on a 272 nm period at 633 nm (Section 3).

A pillar's width error moves its phase by the slope of Section 3's chart; a height error moves every
pillar's phase together, which a lens forgives, and its efficiency through the staircase.

**Try it.** In the pillar chart's default, read the phase's slope near the middle of the widths.
What phase error does a 10 nm width error make there?

:::answer
Near 300 nm wide the phase climbs 0.056 × 2π over 35 nm, 1.6 × 2π per µm: a 10 nm error moves it
by 0.016 × 2π, 0.10 rad. Near 70 nm it climbs 7.3 × 2π per µm, 0.073 × 2π for 10 nm: the narrow
pillars are the sensitive ones (2D FDTD on 20 nm cells).
:::

## 11. History and current research {research}

Lalanne and Chavel trace the metalens to Kock's microwave delay lenses of 1948, made of polystyrene
foam doped with metal inserts; to proposals around 1991 to write graded effective indices in
subwavelength gratings; and to Lalanne et al.'s blazed-binary TiO₂ gratings and lens of 1998 and
1999, which beat the échelette's efficiency at large deflections and reached 80 % in a lens at
850 nm. They date the field's recent growth to Yu et al.'s 2011 arrays of metallic nanoantennas,
which revisited Snell's law with a phase gradient. Dielectric pillars then replaced the lossy
antennas for lenses in transmission:
[Arbabi et al. (2015)](https://doi.org/10.1038/ncomms8069) with silicon posts at 1550 nm and
[Khorasaninejad et al. (2016)](https://doi.org/10.1126/science.aaf6644) with TiO₂ fins in the
visible.

::timeline

Today's questions are in the papers above: bandwidth, where
[Shrestha et al. (2018)](https://doi.org/10.1038/s41377-018-0078-x) engineered each pillar's
dispersion and [Presutti and Monticone (2020)](https://doi.org/10.1364/OPTICA.389404) bounded what
any design can reach; design, where [Elsawy et al. (2019)](https://doi.org/10.1038/s41598-019-53878-9)
optimized a metasurface's whole supercell and [Chen et al. (2024)](https://doi.org/10.1364/JOSAB.506412)
set metalenses as a test for inverse design; and simulation, where
[Hughes et al. (2021)](https://doi.org/10.1063/5.0071245) solved a whole lens in full.

Further reading: [Lalanne and Chavel (2017)](https://doi.org/10.1002/lpor.201600295) for the
history and the efficiency of high-NA lenses;
[Presutti and Monticone (2020)](https://doi.org/10.1364/OPTICA.389404) for the bandwidth bounds;
and the lessons on [photonic crystals](photonic-crystals.md), the same periodic cells with their
bands, and [grating couplers](grating-couplers.md), another grating whose order sets an angle.

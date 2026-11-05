---
title: "Photonic crystals and band gaps"
summary: "A dielectric that repeats in one, two or three dimensions has bands of frequencies that cannot travel in it: Bloch modes and band structures, what opens a gap and for which polarization, how photonoxide computes bands, slabs and the light cone, waveguides with slow light, cavities with their Q and mode volume, and Akahane et al.'s L3 cavity in 3D FDTD."
topic: Nanophotonics
level: advanced
minutes: 50
prerequisites: [bragg-gratings]
examples: [subpixel_holes, l3_akahane]
methods: [fdtd.md, multilayer.md, arrays.md]
validation:
  - mode/multilayer-bragg
  - fdtd/bloch-bands
  - fdtd/harmonic-inversion
  - fdtd/smoothing-crystal
  - geometry/arrays/akahane-l3
charts: [crystal-stack, crystal-bands, crystal-group-index]
papers:
  - cite: "F. Bloch, Z. Phys. 52, 555 (1929)"
    title: "Über die Quantenmechanik der Elektronen in Kristallgittern"
    doi: 10.1007/BF01339455
    year: 1929
    role: origin
    note: "An electron's wavefunction in a periodic potential is a plane wave times a function with the lattice's periods (Eq. 6): the theorem every band structure, of electrons or of light, is computed from."
  - cite: "E. Yablonovitch, Phys. Rev. Lett. 58, 2059 (1987)"
    title: "Inhibited Spontaneous Emission in Solid-State Physics and Electronics"
    doi: 10.1103/PhysRevLett.58.2059
    year: 1987
    role: origin
    note: "A dielectric periodic in three dimensions, with a period of about half a wavelength, can have a gap in every direction; where it overlaps an electronic band edge, spontaneous emission is forbidden. An fcc lattice needs an index contrast of at least about 21 %."
  - cite: "S. John, Phys. Rev. Lett. 58, 2486 (1987)"
    title: "Strong localization of photons in certain disordered dielectric superlattices"
    doi: 10.1103/PhysRevLett.58.2486
    year: 1987
    role: origin
    note: "Near the band edge of a disordered dielectric superlattice light can be localized, as electrons are in Anderson localization, even for weak disorder; an fcc lattice needs a permittivity ratio above about 2.13."
  - cite: "V. A. Mandelshtam, H. S. Taylor, J. Chem. Phys. 107, 6756 (1997)"
    title: "Harmonic inversion of time signals and its applications"
    doi: 10.1063/1.475324
    year: 1997
    role: milestone
    note: "Filter diagonalization: a signal's frequencies and decay rates from a short record, more finely than its Fourier transform resolves; how photonoxide finds the bands and a cavity's Q in FDTD."
  - cite: "Y. Akahane, T. Asano, B.-S. Song, S. Noda, Nature 425, 944 (2003)"
    title: "High-Q photonic nanocavity in a two-dimensional photonic crystal"
    doi: 10.1038/nature02063
    year: 2003
    role: milestone
    note: "Three holes left out of a silicon photonic-crystal slab, the two end holes moved out by 0.15a: the field's envelope turns gentle, less of it falls in the light cone, and the measured Q rises to 45,000 with a mode volume of 7.0 × 10⁻¹⁴ cm³."
  - cite: "A. Farjadpour, D. Roundy, A. Rodriguez, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, G. W. Burr, Opt. Lett. 31, 2972 (2006)"
    title: "Improving accuracy by subpixel smoothing in the finite-difference time domain"
    doi: 10.1364/OL.31.002972
    year: 2006
    role: milestone
    note: "An anisotropic average of ε over each pixel at an interface makes FDTD's band frequencies converge as the square of the grid step, where other averages converge linearly; tested on a square lattice of elliptical holes, the case subpixel_holes re-runs."
  - cite: "J. D. Joannopoulos, S. G. Johnson, J. N. Winn, R. D. Meade, Photonic Crystals: Molding the Flow of Light, 2nd ed., Princeton University Press (2008)"
    title: "Photonic Crystals: Molding the Flow of Light (Second Edition)"
    doi: 10.2307/j.ctvcm4gz9
    year: 2008
    role: review
    note: "The textbook: the master equation, Bloch modes, 1D, 2D and 3D crystals and their gaps, slabs and the light cone, waveguides and cavities, and an atlas of gap maps; the source of this lesson's test numbers."
  - cite: "M. Minkov, V. Savona, Sci. Rep. 4, 5124 (2014)"
    title: "Automated optimization of photonic crystal slab cavities"
    doi: 10.1038/srep05124
    year: 2014
    role: current
    note: "A genetic algorithm moves the holes around L3, H0 and H1 cavities in a silicon slab, their Q computed by guided-mode expansion: an L3 of 5.1 × 10⁶ (4.2 × 10⁶ in 3D FDTD), an H0 of 8.9 × 10⁶."
  - cite: "M. Arcari, I. Söllner, A. Javadi, S. Lindskov Hansen, S. Mahmoodian, J. Liu, H. Thyrrestrup, E. H. Lee, J. D. Song, S. Stobbe, P. Lodahl, Phys. Rev. Lett. 113, 093603 (2014)"
    title: "Near-Unity Coupling Efficiency of a Quantum Emitter to a Photonic Crystal Waveguide"
    doi: 10.1103/PhysRevLett.113.093603
    year: 2014
    role: current
    note: "Quantum dots in a GaAs photonic-crystal waveguide, near its band edge where the group index exceeds 50, emit 98.43 % of their light into the guide's mode."
  - cite: "M. Minkov, I. A. D. Williamson, L. C. Andreani, D. Gerace, B. Lou, A. Y. Song, T. W. Hughes, S. Fan, ACS Photonics 7, 1729 (2020)"
    title: "Inverse Design of Photonic Crystals through Automatic Differentiation"
    doi: 10.1021/acsphotonics.0c00327
    year: 2020
    role: current
    note: "Plane-wave and guided-mode expansions written for automatic differentiation, so a gradient comes for every hole at once: a lithium-niobate cavity optimized from Q = 6,100 to 9.7 × 10⁶ in 131 evaluations."
---

A photonic crystal seen from above, in the symbols of this lesson: a triangular lattice of air
holes of radius $r$, a lattice constant $a$ apart, in a film of permittivity $\varepsilon$; a row
left out, the W1 waveguide; three holes left out, the L3 cavity, its end holes moved out by $s$;
and the lattice's Brillouin zone, along whose edge the bands are drawn.

::diagram crystal

A photonic crystal is a dielectric whose permittivity repeats with a period comparable to the
wavelength. Waves scattered by the repeats add up; in some bands of frequency they cancel every
wave that would travel, in every direction the crystal repeats in. Those bands are its **photonic
band gaps**. A defect in the pattern then holds light at a frequency in the gap: a line of them
guides it, a point traps it.

## 1. What is a photonic band gap, and why does a period open one?

In a uniform medium of index $n$ a plane wave of any frequency travels, with $\omega = ck/n$. In a
medium that repeats with period $a$, a wave of wavevector $k$ is scattered into waves of
$k + 2\pi m/a$, for every integer $m$. Where $k = \pi/a$, the forward wave and the wave scattered
back have the same frequency and couple, the Bragg condition of
[Bragg gratings](bragg-gratings.md): they form two standing waves, one with its energy in the
high-$\varepsilon$ layers and one in the low, at two frequencies. No travelling wave exists
between them: a gap.

Two facts make the gap general:

- **Any contrast opens one in 1D.** Joannopoulos et al. show that an arbitrarily weak periodicity
  gives a gap between the first two bands of a 1D crystal (p. 51), and credit Rayleigh's analysis
  of 1887 with first showing it (p. 46).
- **Scale invariance.** Maxwell's equations in a dielectric have no length of their own: a
  crystal made $s$ times larger has the same bands at frequencies $s$ times lower (p. 20). So
  frequencies are given as $\omega a/2\pi c = a/\lambda$, wavevectors in units of $2\pi/a$, and
  one band diagram serves every wavelength.

In 1987 two papers asked for a gap in all three dimensions at once.
[Yablonovitch (1987)](https://doi.org/10.1103/PhysRevLett.58.2059) proposed a 3D periodic
dielectric whose gap overlaps a semiconductor's electronic band edge, so that spontaneous emission
there is "rigorously forbidden" (p. 2059); for an fcc lattice the gaps at its X and L points must
overlap, which needs an index contrast $\Delta n \gtrsim 0.21 n$ (p. 2061).
[John (1987)](https://doi.org/10.1103/PhysRevLett.58.2486) showed that near such a band edge a
disordered superlattice localizes light, as disorder localizes electrons, for a permittivity ratio
above about 2.13 in an fcc lattice (p. 2488).

## 2. How does a 1D crystal's band structure follow from one period?

A wave in a periodic medium is a **Bloch mode**, $E(z) = e^{iKz}u(z)$ with $u(z + a) = u(z)$:
a plane wave times a function with the medium's period. [Bloch (1929)](https://doi.org/10.1007/BF01339455)
proved it for electrons in a crystal lattice (p. 559, Eq. 6); for light it follows from the same
symmetry of Maxwell's equations (Joannopoulos et al., p. 34, Eq. 9). $K$ is defined to within
$2\pi/a$, so all of it fits in $-\pi/a \lt K \le \pi/a$, the **Brillouin zone**, and the bands are
the frequencies $\omega_n(K)$ there.

The chart draws them for a stack of two layers, $\varepsilon_1$ for $d_1$ and $\varepsilon_2$ for
the rest of the period, at normal incidence:

::chart crystal-stack

At the book's 13:1 stack with $d_1 = 0.2a$ the first gap is 76.32 % of its centre frequency,
from $\omega a/2\pi c$ = 0.20305 to 0.45364; Joannopoulos et al. compute 76.3 % (p. 52). With
$d_1 = 0.5a$ it is 51.89 %, theirs 51.9 %, and a weak 13:12 stack of equal layers has 2.546 %,
theirs 2.55 % (p. 51). The second gap, at $K = 0$, is 5.850 % wide at $d_1 = 0.2a$, and closes
for the quarter-wave stack.

### The dispersion relation {theory}

Write the field in each layer as two counter-running waves, and carry their amplitudes across one
period by the layers' transfer matrices: one period multiplies them by a matrix $T$ with
$\det T = 1$ in a lossless stack. A Bloch mode is carried across a period into $e^{iKa}$ times
itself, so $e^{iKa}$ is an eigenvalue of $T$, and with $\det T = 1$ the two eigenvalues are
$e^{\pm iKa}$, whose sum is the trace:

$$
\cos Ka = \tfrac12 \operatorname{tr} T = \cos k_1 d_1 \cos k_2 d_2 - \tfrac12\left(\frac{n_1}{n_2} + \frac{n_2}{n_1}\right)\sin k_1 d_1 \sin k_2 d_2 ,
$$

$k_i = n_i\omega/c$. Where the right side lies between −1 and 1, $K$ is real and a wave travels;
where it lies outside, $K$ is complex and the wave decays along the stack: a gap. The chart
evaluates the half trace as $\operatorname{Re}(1/t)$, $t$ one period's transmission from
`Multilayer::reflection`, which equals it for a symmetric, lossless period; it is exact, with no
grid.

The gap is widest when both layers are a quarter of a wave thick at its centre,
$n_1 d_1 = n_2 d_2$, and then (Joannopoulos et al., p. 52, Eqs. 4 and 5)

$$
\frac{\omega_m a}{2\pi c} = \frac{n_1 + n_2}{4 n_1 n_2}, \qquad
\frac{\Delta\omega}{\omega_m} = \frac{4}{\pi}\arcsin\frac{\lvert n_1 - n_2\rvert}{n_1 + n_2} ,
$$

76.56 % for 13:1 (the book rounds it to 76.6 %), which the chart's stack gives to the bisection's
precision at
$d_1/a = 1/(1 + \sqrt{13}) = 0.2171$. A unit test checks the chart against all four of the book's
numbers. The same transfer matrices are checked against the quarter-wave stack's closed form, and
photonoxide's FDTD between Bloch-periodic sides finds a 1D crystal's bands to 3.1 × 10⁻⁴ at 80
cells a period:

::validation mode/multilayer-bragg fdtd/bloch-bands

**Try it.** In the chart, set $\varepsilon_2$ to 12, then the fill to 0.5. How wide is the gap,
and what does the perturbative estimate $\Delta\omega/\omega_m \approx (\Delta\varepsilon/\varepsilon)\sin(\pi d/a)/\pi$
(Joannopoulos et al., p. 51, Eq. 3) predict?

:::answer
2.546 %. With $\Delta\varepsilon/\varepsilon = 1/12$ and $\sin(\pi/2) = 1$ the estimate gives
2.65 %, the figure the book prints beside its own 2.55 %: the estimate is first order in the
contrast, and its error grows with it.
:::

## 3. How is a 2D crystal's band diagram read?

In two dimensions the wavevector $\mathbf k$ has two components, and the Brillouin zone is a
polygon: a square for a square lattice, a hexagon for a triangular one (the diagram). Its centre
is $\Gamma$, $\mathbf k = 0$; for the square lattice X is the middle of an edge,
$\mathbf k = (\pi/a)\hat x$, and M a corner, $(\pi/a)(\hat x + \hat y)$; for the triangular
lattice M is the middle of an edge and K a corner. The lattice's rotations and mirrors map the
zone onto a wedge, the irreducible zone, and the bands are drawn along its edge: their minima and
maxima, which set the gaps, "almost always occur at the zone edges, and often at a corner"
(Joannopoulos et al., pp. 68–69).

Light travelling in the plane of a 2D crystal splits into two polarizations that don't mix: **TM**,
$E$ along the rods or holes, and **TE**, $H$ along them, as Joannopoulos et al. name them. The
chart draws both, for a square lattice of alumina rods ($\varepsilon = 8.9$) in air, $r = 0.2a$,
the book's first 2D example:

::chart crystal-bands

The TM bands have a gap of 31.24 % between the first and the second band, from
$\omega a/2\pi c$ = 0.3221, the first band's top at M, to 0.4414, the second band's bottom at X;
Joannopoulos et al. give 31.4 % (p. 69), about 0.32 to 0.44 (p. 91). The TE bands have none, as in
the book. The chart's numbers are 2D FDTD's on 24 cells a lattice constant. On 32 cells the gap is
31.30 %, and the two extrapolated to a fine grid, the error taken as second order in the cell, give
31.38 %; a unit test checks that against the book's 31.4 %.

The first TM band is the book's dielectric band, the second its air band (p. 69, Fig. 3): the
first band's field is concentrated in the rods, while the second, which must be orthogonal to it,
has a node through each rod and more of its energy in the air. Field in a lower $\varepsilon$ costs
a higher frequency, and the difference between the two is the gap.

## 4. What opens a gap, and for which polarization?

Joannopoulos et al.'s rule of thumb: "TM band gaps are favored in a lattice of isolated high-ε
regions, and TE band gaps are favored in a connected lattice" (p. 75). TM's electric field lies
along the rods and can sit in isolated spots of dielectric; TE's lines of electric field lie in the
plane and must cross from spot to spot, so they need connected dielectric to follow.

A triangular lattice of air holes is both at once when its holes are large: the dielectric between
three holes is an isolated spot, joined to its neighbours by narrow veins. At $r = 0.48a$ in
$\varepsilon = 13$ its TE and TM gaps overlap in a **complete gap** of 18.6 % (p. 75):

::chart crystal-bands{lattice=2, kind=2, eps=13, radius=0.48}

On 24 cells the chart finds a complete gap of 18.58 %, from $\omega a/2\pi c$ = 0.4302 to 0.5183.
That agreement is partly the grid's: on finer grids the gap settles at 18.94, 18.89, 18.91 and
18.92 % on 32, 40, 48 and 64 cells. **The book's 18.6 % is not reproduced**: our converged gap is
0.3 points (1.7 %) wider, and the book prints neither its resolution nor its gap's edges to look
for the cause. The complete gap here is the TM gap, which lies inside the TE gap. The TE gap,
29.9 % on 24 cells, is not converged: the dielectric between neighbouring holes is $0.04a$ wide,
one cell, and the TE gap widens to 35.2 % on 64 cells as the grid resolves it.

### Gap maps {theory}

The gap moves with the radius and the contrast. Joannopoulos et al.'s Appendix C (pp. 242–250)
draws, for each lattice of rods or holes at $\varepsilon = 11.4$, the gaps' edges against $r/a$
for TE and TM, and the largest gap and its best radius against the contrast. The chart computes one
point of such a map at a time.

**Try it.** In the triangular lattice of holes, lower the radius from 0.48 to 0.40. What happens
to the TE and TM gaps?

:::answer
The TE gap widens to 50.5 %, from 0.2409 to 0.4035: with smaller holes the veins are wider, $0.2a$,
and the connected dielectric favours TE. The TM gap moves up and narrows to 6.19 %, from 0.6265 to
0.6665, as the dielectric spots between the holes become less isolated, and the complete gap
shrinks to 1.26 % (24 cells). Large holes make the spots isolated enough for TM while keeping them
joined for TE: the compromise of the rule of thumb.
:::

## 5. How does photonoxide compute a band, and how accurate is it?

The crystal's cell, one period ($a \times a$) or, for the triangular lattice, a rectangle of two
sites ($a \times \sqrt3 a$), is solved by photonoxide's FDTD with **Bloch-periodic sides**: the
field one period on is $e^{i\mathbf k\cdot\mathbf a}$ times itself. That makes the field complex,
and the solver steps its real and imaginary parts side by side, coupled only across the sides
([the FDTD solver](../docs/methods/fdtd.md)). A short pulse from point currents set off every
symmetry line excites every band at that $\mathbf k$; probes record the field for 160 $a/c$, and
**harmonic inversion** ([Mandelshtam and Taylor, 1997](https://doi.org/10.1063/1.475324)) finds the
frequencies in the record: a filter-diagonalization fit of decaying sinusoids, which resolves
frequencies more finely than a Fourier transform of the same record. One run gives both
polarizations, read from $E_z$ for TM and $H_z$ for TE. For the triangular lattice, the rectangle
of two sites also holds bands folded in from other wavevectors; adding each probe's signal to its
copy one lattice vector $\mathbf a_2$ away, times $e^{-i\mathbf k\cdot\mathbf a_2}$, cancels them.

Harmonic inversion is checked on a signal of known terms, and a cavity's resonances against the
exact ones:

::validation fdtd/harmonic-inversion

### Subpixel smoothing {theory}

A hole's edge cuts through the grid's cells. Giving each cell the $\varepsilon$ at its centre makes
the computed frequencies jump about as the grid is refined; giving it a mean of the
$\varepsilon$ inside it is better, but first order in the grid step.
[Farjadpour et al. (2006)](https://doi.org/10.1364/OL.31.002972) average differently across and
along the interface, $\tilde\varepsilon^{-1} = P\langle\varepsilon^{-1}\rangle + (1 - P)\langle\varepsilon\rangle^{-1}$,
$P$ the projection onto the interface's normal (their Eq. 1), and the error of a TE band of a
square lattice of elliptical holes in $\varepsilon = 12$ becomes "asymptotically quadratic", where
other averages converge linearly (p. 2973, Fig. 1).
[subpixel_holes](../examples/subpixel_holes.rs) re-runs that test at the X point, the two lowest
TE bands at 0.145257 and 0.164053 $c/a$, their reference the 64- and 96-cell runs extrapolated:

::example subpixel_holes

Its smoothing's error falls with a slope of −2.26 against the paper's −2.43, below the mean
$\varepsilon$'s at every resolution, as the paper found. One number is not reproduced: the mean
$\varepsilon$'s slope for the first band is −1.84 here against the paper's −1.33 (printed, not
checked). The case `fdtd/smoothing-crystal` checks the smoothing's error on a crystal's band
against another published reference:

::validation fdtd/smoothing-crystal

## 6. What changes in a slab, and why does the light cone matter?

A real crystal for a chip is a film a few hundred nanometres thick with holes through it, in air or
on oxide. The 2D crystal's rods are infinitely long; the slab's are not, and light can leave it
vertically. A mode of in-plane wavevector $\mathbf k_\parallel$ can couple to a plane wave in the
air above only if that wave has the same $\mathbf k_\parallel$ and a real vertical wavevector, which
needs $\omega \ge c\lvert\mathbf k_\parallel\rvert$: the **light cone** (Joannopoulos et al.,
p. 136). Below it, the slab's modes are guided by total internal reflection and lossless; inside it
they leak. A slab's band gaps are therefore incomplete: "they refer only to the guided modes"
(p. 136).

The book's slabs are a triangular lattice of holes, $r = 0.3a$, $0.6a$ thick, and a square lattice
of rods, $r = 0.2a$, $2a$ tall (p. 135). Their gaps depend on the thickness: too thin and the modes
spread into the air and feel little contrast; too thick and higher-order vertical modes fill the gap.
The holes' even (TE-like) gap approaches 28 % and the rods' odd (TM-like) gap 39 %, and each
closes above a critical thickness, about $1.2a$ for the holes and $2.25a$ for the rods (p. 139).
A slab's bands need a 3D solve of its cell; photonoxide's charts are 2D and draw none.

## 7. What does a line defect do, and why is light slow at a band edge?

Leave out one row of holes and the crystal around it acts as a mirror for frequencies in its gap:
light travels along the row and nowhere else. One missing row of a triangular lattice is the
**W1** waveguide (Joannopoulos et al., p. 143). Its guided band flattens towards the zone's edge,
where its group velocity $v_g = d\omega/dk$ "approaches zero" and its dispersion diverges: slow light
(p. 146).

The same happens at every band edge, and the 1D crystal shows it exactly. The group index
$n_g = c/v_g = c\,dK/d\omega$ of its first two bands:

::chart crystal-group-index

At long wavelengths the stack is a uniform medium and $n_g$ is $\sqrt{\langle\varepsilon\rangle}$,
1.8439 for the 13:1 stack at $d_1 = 0.2a$. Towards the gap $n_g$ grows without bound: 9.144 at 1 %
of the edge's frequency below it and 28.60 at 0.1 %.

### Why it diverges {theory}

Near the edge of the first band, $\cos Ka \approx -1 + \tfrac12(\pi - Ka)^2$, and the half trace
falls through −1 linearly in $\omega$: $\cos Ka + 1 \approx c'(\omega_e - \omega)$. So
$\pi - Ka \propto \sqrt{\omega_e - \omega}$ and

$$
n_g = c\frac{dK}{d\omega} \propto \frac{1}{\sqrt{\omega_e - \omega}} ,
$$

ten times closer to the edge, $\sqrt{10} = 3.16$ times slower; the chart's 28.60/9.144 is 3.13,
the curvature's next order making up the rest. A unit test checks both limits on the chart. The chart's $n_g$ is the derivative of the exact dispersion, taken by a central difference
of 10⁻⁶ of the frequency.

### Slow light at work {research}

A slower wave spends longer near each point it passes, and an emitter there couples to it more
strongly. [Arcari et al. (2014)](https://doi.org/10.1103/PhysRevLett.113.093603) put InAs quantum
dots into a GaAs photonic-crystal waveguide: "Purcell enhancement is proportional to the group index
$n_g = c/v_g$", and near the band edge they measured $n_g$ above 50, which leads to expected Purcell
factors close to 10 (p. 093603-2). A dot there sent 98.43 % ± 0.04 % of its light into the guide's
mode (abstract). Slowness has a price the same paper's band shows: it holds only within nanometres
of the band edge, in their guide within 20 nm of its cutoff at 925 nm for $\beta$ above 90 %
(p. 093603-3).

## 8. How does a point defect trap light, and what limits its Q?

Leave out one hole, or three in a row, and a mode with its frequency in the gap is held by the
crystal on every side in the plane, and by total internal reflection vertically. Its **quality
factor** $Q = \omega_0/\Gamma$ counts how many radians of oscillation its energy lasts, $\Gamma$ the
energy's decay rate (Joannopoulos et al., pp. 131–132); its **mode volume**
$V = \int\varepsilon\lvert E\rvert^2/\max(\varepsilon\lvert E\rvert^2)$ how tightly it is held
([Minkov and Savona, 2014](https://doi.org/10.1038/srep05124), Eq. 2), typically about
$(\lambda/2n)^3$ for these cavities (Joannopoulos et al., p. 150). An emitter in the cavity emits
faster in proportion to $Q/V$, the Purcell effect (p. 151).

In a slab the plane holds the light; the vertical leak sets $Q$. A cavity's field is a sum of plane
waves in the plane, and those with $\lvert\mathbf k_\parallel\rvert \lt \omega/c$, inside the light
cone, radiate. [Akahane et al. (2003)](https://doi.org/10.1038/nature02063) made the rule explicit:
a field whose envelope ends abruptly at the cavity's edges has a sinc-shaped spectrum with large
components inside the light cone, and a Gaussian envelope has very small ones; "light should be
confined gently in order to be confined strongly" (p. 944; Fig. 2, p. 945). In an L3 cavity in silicon,
$a = 0.42$ µm, $0.6a$ thick, $r = 0.29a$, moving the two end holes out by $s = 0.15a$ made the
envelope gentle: they measured $Q$ = 45,000 with $V$ = 7.0 × 10⁻¹⁴ cm³, a $Q/V$ ten times the
unshifted cavity's (p. 946, Fig. 4).

[l3_akahane](../examples/l3_akahane.rs) solves their cavity in photonoxide's 3D FDTD, the slab and
its holes as one body ([the arrays](../docs/methods/arrays.md)), and finds each mode's frequency and
$Q$ by harmonic inversion. On 8 cells a lattice constant, the unshifted cavity's $Q$ is 5,574 and the
shifted one's 56,156, a rise of 10.1 times against the measured tenfold; the wavelengths, 1.5941 and
1.6042 µm, lie 1.4 % and 2.0 % above 1.5725 µm, the middle of the measured 1.565 to 1.58 µm:

::example l3_akahane

The computed $Q$ is the ideal cavity's radiation limit; the measured one also loses light to the
fabricated holes' disorder and to absorption, $1/Q = 1/Q_\text{t} + 1/Q_\text{a} + 1/Q_\text{d}$
(Minkov and Savona, Eq. 1), so a computed $Q$ above a measured one is expected. Akahane et al. print
no computed $Q$ in their text to compare with ours. And 8 cells a lattice constant is coarse for a
$Q$: on 12 and 16 cells the rise is 9.7 and 7.5 times ([the roadmap](../ROADMAP.md), 0.14), so
the shifted cavity's $Q$ does not settle with the grid yet, and the example claims it only within
a factor of 2.

### Higher Q by design {research}

Moving more holes further raises $Q$. Minkov and Savona shifted the five nearest holes on either
end of an L3 along its axis by a genetic algorithm, with $Q$ computed by guided-mode expansion,
and reached $Q$ = 5.1 × 10⁶ (4.2 × 10⁶ in 3D FDTD) with $V$ = 0.95 $(\lambda/n)^3$, and an H0 cavity
of 8.89 × 10⁶ (p. 2 and p. 4). [Minkov et al. (2020)](https://doi.org/10.1021/acsphotonics.0c00327)
took the gradient of the same expansion by automatic differentiation and optimized 45 parameters
of a lithium-niobate cavity from $Q$ = 6,100 to 9.7 × 10⁶ in 131 evaluations of the model, and
2.4 × 10⁶ in an FDTD check of the result. Measured cavities stay below
their designs: Minkov and Savona's second H0 design, 1.88 × 10⁶ in the model, measured 485,000
(p. 4), the disorder term above.

**Try it.** l3_akahane records the field for 300 µm/c after the pulse. How many periods of the
shifted mode is that, and how much of its energy is lost over the record?

:::answer
At $\lambda$ = 1.6042 µm the mode completes $1/1.6042$ periods per µm/c, so 300 µm/c is about 187
periods, $2\pi \times 187 \approx 1175$ radians. Its energy falls as $e^{-\omega t/Q}$, by
$e^{-1175/56{,}156} = 0.979$: 2.1 %. A spectrum of that record resolves about one part in 187 of
the frequency, far coarser than the linewidth $1/Q$; harmonic inversion fits the decay rate itself,
which is how the example gets a $Q$ of 56,156 from it.
:::

## 9. How is a photonic-crystal slab made, and what do its errors do?

The holes go through the film in one lithography and one etch, the steps that make a strip
waveguide ([How a chip is made](how-a-chip-is-made.md)), with the holes drawn in the mask where the
strip's outline is. The strip's flow in photonoxide's fabrication engine shows them:

::fabrication waveguide

Akahane et al.'s slab has air above and below it (p. 945), as l3_akahane models it: a membrane,
which none of the library's flows makes. The holes are a few hundred nanometres across ($2r$ =
0.24 µm in their cavity), so a few nanometres of error in the lithography's linewidth is a few
percent of a radius, and each error moves something different:

- **The lattice constant** is drawn by the mask. By scale invariance it sets the gap's wavelength
  in proportion (Section 1).
- **The radius** moves with the linewidth, and the bands move with it: compare Section 4's chart
  at one radius and the next.
- **Disorder**, the random part of the holes' radii and positions, adds a loss of its own to a
  cavity, $1/Q_\text{d}$ in Section 8's sum, the reason measured $Q$s stay below designed ones.

## 10. How does photonoxide compute a photonic crystal, and what does 2D mean?

- **1D crystals**: [the multilayer](../docs/methods/multilayer.md)'s transfer matrices, exact, for
  the stack's bands and group index; FDTD between Bloch sides for the same bands
  (`fdtd/bloch-bands`).
- **2D crystals**: [2D FDTD](../docs/methods/fdtd.md) between Bloch-periodic sides on one cell,
  with subpixel smoothing, and harmonic inversion for the frequencies (Section 5). A 2D crystal is
  infinitely long along its rods or holes, and its bands are those of light in the plane only; a 2D
  gap is not a slab's, which has a light cone and leaks (Section 6).
- **Slabs and cavities**: the `PhotonicCrystal` generator draws square and hexagonal lattices with an
  L3, H1 or W1 defect ([the arrays](../docs/methods/arrays.md)), checked against Akahane et al.'s
  geometry; 3D FDTD solves them and harmonic inversion gives a cavity's frequency and Q
  (l3_akahane):

::validation geometry/arrays/akahane-l3

What it doesn't compute yet: band structures by plane-wave expansion, and with them a slab's
guided bands under its light cone and a W1 waveguide's dispersion; [the roadmap](../ROADMAP.md)
plans them in 0.14, with photonic-crystal waveguides and cavities, of which the lattices and the L3
cavity above are the start.

## 11. History and current research {research}

[Bloch (1929)](https://doi.org/10.1007/BF01339455) showed that a wave in a periodic potential is a
plane wave times a periodic function, the theorem every band structure rests on; Joannopoulos et
al. trace the optical 1D gap back to Rayleigh in 1887 (p. 46). A century later,
[Yablonovitch (1987)](https://doi.org/10.1103/PhysRevLett.58.2059) and
[John (1987)](https://doi.org/10.1103/PhysRevLett.58.2486) asked for a gap in three dimensions, to
forbid spontaneous emission and to localize light. The first complete 3D gap was measured at
microwave scale in Yablonovitch et al.'s drilled crystal of 1991, 19 % wide (Joannopoulos et al.,
p. 97). Patterned films brought crystals to chips: [Akahane et al. (2003)](https://doi.org/10.1038/nature02063)
reached $Q$ = 45,000 in a silicon slab cavity by confining its light gently. Numerical methods
grew with them: harmonic inversion ([Mandelshtam and Taylor, 1997](https://doi.org/10.1063/1.475324))
and subpixel smoothing ([Farjadpour et al., 2006](https://doi.org/10.1364/OL.31.002972)) made FDTD's
bands and $Q$s accurate, and [Joannopoulos et al. (2008)](https://doi.org/10.2307/j.ctvcm4gz9)
collected the theory.

::timeline

Today's questions are in the papers above: cavities whose $Q$ is set by optimizing many holes at
once ([Minkov and Savona, 2014](https://doi.org/10.1038/srep05124);
[Minkov et al., 2020](https://doi.org/10.1021/acsphotonics.0c00327)), slow-light waveguides that
couple single emitters to a chip ([Arcari et al., 2014](https://doi.org/10.1103/PhysRevLett.113.093603)),
and the gap between designed and measured $Q$s that fabrication disorder leaves.

Further reading: [Joannopoulos et al. (2008)](https://doi.org/10.2307/j.ctvcm4gz9), chapters 4
and 5 for 1D and 2D crystals, 8 for slabs and Appendix C for the gap maps;
[Akahane et al. (2003)](https://doi.org/10.1038/nature02063) for the light-cone argument; and the
lessons on [Bragg gratings](bragg-gratings.md), the same periodic structure as a mirror, and
[how a chip is made](how-a-chip-is-made.md).

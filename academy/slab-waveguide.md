---
title: "The slab waveguide"
summary: "A film of high index between two of lower index: why it guides light, the TE and TM modes it holds and where each is cut off, the b–V diagram, how much of the light the film keeps, and the modes of a 220 nm silicon slab, solved exactly."
topic: Waveguides
level: introductory
minutes: 40
prerequisites: [material-dispersion]
examples: [slab_yariv_yeh, slab_soi, multilayer_chilwell]
jobs: [slab-soi.yaml]
methods: [slab.md, multilayer.md, slab-fd.md, vector.md]
validation:
  - mode/slab-te-book
  - mode/slab-tm-book
  - mode/multilayer-bound-chilwell
  - mode/multilayer-power-chilwell
  - mode/slab-fd-chilwell
  - mode/vector-slab-limit-te
  - mode/vector-slab-limit-tm
  - mode/slab-group-index
charts: [slab-fields, slab-bv, slab-neff, slab-confinement]
papers:
  - cite: "F. Goos, H. Hänchen, Ann. Phys. 436, 333 (1947)"
    title: "Ein neuer und fundamentaler Versuch zur Totalreflexion"
    doi: 10.1002/andp.19474360704
    year: 1947
    role: origin
    note: "An experiment showing that totally reflected light travels in the rarer medium before it returns, detected in the denser medium without disturbing the reflection and related quantitatively to Maxwell's theory: the shift along the face that carries their names."
  - cite: "S. E. Miller, Bell Syst. Tech. J. 48, 2059 (1969)"
    title: "Integrated Optics: An Introduction"
    doi: 10.1002/j.1538-7305.1969.tb01165.x
    year: 1969
    role: origin
    note: "Laser-beam circuits in miniature: guides about 10 µm wide made by index changes of 10⁻² to 10⁻³ in a substrate such as glass, patterned by photolithography, with lasers, modulators and hybrids on the same substrate."
  - cite: "P. K. Tien, R. Ulrich, J. Opt. Soc. Am. 60, 1325 (1970)"
    title: "Theory of Prism–Film Coupler and Thin-Film Light Guides"
    doi: 10.1364/JOSA.60.001325
    year: 1970
    role: milestone
    note: "The prism–film coupler analysed by wave and ray optics together: the modes of a thin-film guide, how coupling changes them, and 81 % of a laser's power fed into a chosen mode. The coupler measures the mode spectrum, from which the film's index and thickness follow."
  - cite: "P. K. Tien, Appl. Opt. 10, 2395 (1971)"
    title: "Light Waves in Thin Films and Integrated Optics"
    doi: 10.1364/AO.10.002395
    year: 1971
    role: review
    note: "Light waves in thin films reviewed: their physics, the films' materials and losses, coupling light into and out of a film, and nonlinear interactions in guides."
  - cite: "H. Kogelnik, V. Ramaswamy, Appl. Opt. 13, 1857 (1974)"
    title: "Scaling Rules for Thin-Film Optical Waveguides"
    doi: 10.1364/AO.13.001857
    year: 1974
    role: milestone
    note: "An asymmetry measure beside the normalized frequency, so that the cutoff, effective index and effective thickness of every asymmetric thin-film guide can be read off universal charts: the b–V diagram."
  - cite: "H. Kogelnik, in T. Tamir (ed.), Integrated Optics, Topics in Applied Physics, Springer (1975), pp. 13–81"
    title: "Theory of dielectric waveguides"
    doi: 10.1007/BFb0103618
    year: 1975
    role: review
    note: "The theory of dielectric waveguides, slabs among them, as the second chapter of Tamir's Integrated Optics."
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984)"
    title: "Thin-films field-transfer matrix theory of planar multilayer waveguides and reflection from prism-loaded waveguides"
    doi: 10.1364/JOSAA.1.000742
    year: 1984
    role: milestone
    note: "A 2 × 2 transfer matrix per film: the bound modes and leaky waves of any planar stack as roots of one dispersion function, and the field and power in every layer; photonoxide's multilayer is this method."
  - cite: "D. Marcuse, Theory of Dielectric Optical Waveguides, 2nd ed., Academic Press (1991), Chapter 1"
    title: "The Asymmetric Slab Waveguide"
    doi: 10.1016/B978-0-12-470951-5.50007-X
    year: 1991
    role: review
    note: "A book on the theory of dielectric guides that opens on the asymmetric slab (Chapter 1, pp. 1–59) before weakly guiding fibres, coupled modes and the directional coupler."
  - cite: "R. Soref, IEEE J. Sel. Top. Quantum Electron. 12, 1678 (2006)"
    title: "The Past, Present, and Future of Silicon Photonics"
    doi: 10.1109/JSTQE.2006.883151
    year: 2006
    role: review
    note: "Silicon photonics on silicon-on-insulator in commercial CMOS foundries: modulators of 10 to 100 Gb/s, germanium photodetectors, fibre couplers and Raman lasers, and what was still missing, an electrically pumped silicon laser."
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008)"
    title: "Vector Finite Difference Modesolver for Anisotropic Dielectric Waveguides"
    doi: 10.1109/JLT.2008.923643
    year: 2008
    role: milestone
    note: "A full-vector finite-difference mode solver for anisotropic guides. photonoxide's full-vector solver follows it, and its planar-profile solver is the same scheme in one dimension."
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    title: "Silicon Photonics Design"
    doi: 10.1017/CBO9781316084168
    year: 2015
    role: review
    note: "Silicon photonics designed from the wafer up: the 220 nm slab's TE and TM modes found analytically and numerically, their decay into the cladding, and the thickness below which the slab is single-mode; the numbers slab_soi reproduces."
  - cite: "D. J. Blumenthal, R. Heideman, D. Geuzebroek, A. Leinse, C. Roeloffzen, Proc. IEEE 106, 2209 (2018)"
    title: "Silicon Nitride in Silicon Photonics"
    doi: 10.1109/JPROC.2018.2861576
    year: 2018
    role: current
    note: "Silicon nitride planar guides: low loss, transparent from 400 to 2350 nm, made at wafer scale, a platform beside silicon-on-insulator and III–V; their history, devices and foundries."
  - cite: "D. Zhu, L. Shao, M. Yu, R. Cheng, B. Desiatov, C. J. Xin, Y. Hu, J. Holzgrafe, S. Ghosh, A. Shams-Ansari, E. Puma, N. Sinclair, C. Reimer, M. Zhang, M. Lončar, Adv. Opt. Photon. 13, 242 (2021)"
    title: "Integrated photonics on thin-film lithium niobate"
    doi: 10.1364/AOP.411024
    year: 2021
    role: current
    note: "Lithium niobate films sliced from bulk crystals and bonded to oxide: an index contrast about ten times that of diffused guides, and losses from more than 6 dB/cm in 2007 to 0.027 dB/cm in 2017."
---

The slab in section, in the symbols the equations below use: a film of index $n_2$ and thickness
$d$ between a cover $n_1$ and a substrate $n_3$, a ray travelling along it at an angle $\theta$, a
steeper ray that escapes, and a guided mode's field across the three layers. The 3D view is the
silicon film of `jobs/slab-soi.yaml`, an SOI wafer before anything is etched.

::diagram slab

A slab waveguide is a film whose refractive index is higher than that of the media on either side.
Light in the film that meets its faces at a grazing enough angle is totally reflected at both and
travels along it; only a discrete set of field patterns, the modes, travels unchanged. Every planar
photonic platform starts as a slab: the 220 nm silicon film of an SOI wafer is one before it is
etched into strips, and designing its waveguides starts from the slab's modes
([Chrostowski and Hochberg, 2015](https://doi.org/10.1017/CBO9781316084168), Section 3.2.1).

## 1. Why does a slab guide light?

A ray in the film meets a face at an angle $\theta$ from the face's normal. Snell's law,
$n_2 \sin\theta = n_3 \sin\theta_3$, has no real angle $\theta_3$ when $n_2 \sin\theta \gt n_3$:
beyond the critical angle $\theta_c = \arcsin(n_3/n_2)$ all the light is reflected. A ray beyond
the critical angles of both faces zig-zags along the film and keeps its power. For silicon (3.473)
on oxide (1.444) $\theta_c = 24.6°$; at a face to air, 16.7°. A steeper ray, like the diagram's
grey one, loses part of its power to the cladding at every reflection.

Not every angle beyond $\theta_c$ is a mode. A ray that crosses the film and comes back must be in
phase with itself, or the waves of its many reflections cancel; only one angle $\theta_m$ for each
whole number $m$ is. Each is a mode, travelling along $z$ with the propagation constant
$\beta = k n_2 \sin\theta_m$, $k = 2\pi/\lambda$, and the effective index
$n_\text{eff} = \beta/k = n_2 \sin\theta_m$, which lies between $n_3$ (at $\theta_c$) and $n_2$
(at grazing incidence).

### The phase of total internal reflection {theory}

The ray is a plane wave in the film of wavenumber $k n_2$: along $z$ it varies as $e^{i\beta z}$,
$\beta = k n_2 \sin\theta$, and across the film as $e^{\pm ihx}$, $h = k n_2 \cos\theta$. In the
substrate a field with the same $e^{i\beta z}$ obeys
$\partial^2 E/\partial x^2 = (\beta^2 - k^2 n_3^2) E$. Beyond the critical angle the bracket is
positive, so the field there decays away from the face as $e^{px}$ (the substrate at $x \lt 0$),

$$
p = \sqrt{\beta^2 - k^2 n_3^2} .
$$

Put the face at $x = 0$ and let a TE wave, its $E_y$ along the face, arrive as $e^{-ihx}$ and
leave as $r e^{ihx}$, with $t e^{px}$ beyond. $E_y$ and $\partial E_y/\partial x$ are continuous
(they are the tangential $E$ and $H$): $1 + r = t$ and $-ih + ihr = pt$, so

$$
r = \frac{h - ip}{h + ip} = e^{-2i\phi_3}, \qquad \tan\phi_3 = \frac{p}{h} .
$$

$|r| = 1$: all the power comes back, delayed in phase by $2\phi_3$. The delay comes from the
field's excursion into the substrate: [Goos and Hänchen (1947)](https://doi.org/10.1002/andp.19474360704)
showed by experiment that totally reflected light travels in the rarer medium before it returns,
which shifts the reflected beam along the face. At the cover the same holds with
$q = \sqrt{\beta^2 - k^2 n_1^2}$ and $\tan\phi_1 = q/h$. A TM wave, its $H_y$ along the face, has
$H_y$ and $(1/n^2)\thinspace\partial H_y/\partial x$ continuous instead, which puts
$\bar p = (n_2/n_3)^2 p$ in place of $p$ and $\bar q = (n_2/n_1)^2 q$ in place of $q$.

### The transverse resonance {theory}

Across the film and back, the wave gains $2hd$ in phase by travelling and loses $2\phi_1 + 2\phi_3$
at the two reflections. In phase with itself,

$$
2hd - 2\phi_1 - 2\phi_3 = 2m\pi
\quad\Longrightarrow\quad
hd = m\pi + \arctan\frac{q}{h} + \arctan\frac{p}{h}, \qquad m = 0, 1, 2, \ldots
$$

This is the TE modes' characteristic equation, and with $\bar q$ and $\bar p$ the TM modes'. Yariv
and Yeh (*Photonics*, 6th ed., 2007, Section 3.2) write it as $\tan(hd)$ equal to a ratio, their
Eqs. 3.2-5 and 3.2-11; the tangent's addition formula turns theirs into this form, which photonoxide
solves. As $\beta$ rises from $k n_3$ to $k n_2$ the left side falls from $kd\sqrt{n_2^2 - n_3^2}$
to 0 and the right side rises, so each $m$ has one root or none: mode $m$, whose field has $m$
zeros in the film. [Tien and Ulrich (1970)](https://doi.org/10.1364/JOSA.60.001325) analysed
thin-film guides by this combination of ray and wave optics.

**Try it.** 220 nm of silicon in oxide has a TE mode of effective index 2.844816 and a TM mode of
2.051101 at 1550 nm (Section 5). At what angles do their rays travel?

:::answer
$\sin\theta = n_\text{eff}/n_2$: $\theta = 55.0°$ for TE₀ and 36.2° for TM₀, both beyond
$\theta_c = 24.6°$. TM₀'s ray is the steeper, nearer the critical angle, and its field reaches
further into the cladding (Sections 2 and 3).
:::

## 2. What does a mode look like?

A mode is a field across the slab that travels along $z$ unchanged, as $e^{i(\beta z - \omega t)}$.
In the film it is the standing wave of the zig-zag's upward and downward rays, a cosine across;
outside it decays exponentially, over $1/p$ into the substrate and $1/q$ into the cover. Mode $m$
has $m$ zeros in the film. The chart draws each guided mode's transverse electric field across a
silicon film on oxide: at 220 nm and 1550 nm the film holds one TE and one TM mode, and TE₀'s field
falls by a factor $e$ in the first 100.6 nm beyond each face.

::chart slab-fields

### The fields {theory}

For a field uniform along $y$ and varying as $e^{i(\beta z - \omega t)}$, Maxwell's equations split
into two independent sets: TE, with $E_y$, $H_x$ and $H_z$, and TM, with $H_y$, $E_x$ and $E_z$.
Inside each layer $E_y$ (TE) or $H_y$ (TM) obeys

$$
\frac{d^2 \psi}{dx^2} + \left(k^2 n^2 - \beta^2\right) \psi = 0 .
$$

In the film $k^2 n_2^2 - \beta^2 = h^2 \gt 0$ and the solutions oscillate; outside,
$k^2 n^2 - \beta^2$ is $-p^2$ or $-q^2$ and they are exponentials, of which a guided mode keeps the
decaying ones. With $x$ up from the substrate's face, the TE field is

$$
E_y(x) = A \begin{cases}
\cos\phi_3 \thinspace e^{px}, & x \lt 0, \cr
\cos(hx - \phi_3), & 0 \le x \le d, \cr
\cos(hd - \phi_3) \thinspace e^{-q(x - d)}, & x \gt d .
\end{cases}
$$

It is continuous by construction. Its slope is continuous at $x = 0$ if $h \tan\phi_3 = p$, and at
$x = d$ if $h \tan(hd - \phi_3) = q$; together they give
$hd = m\pi + \arctan(p/h) + \arctan(q/h)$, Section 1's transverse resonance found from the field.
This is Yariv and Yeh's Eq. 3.2-3, with their $x$ reversed.

The effective index lies between $n_3$ and $n_2$. Below $n_3$, $p$ is imaginary: the field in the
substrate is a wave travelling away, which carries power off, a radiation mode rather than a
guided one. Above $n_2$, $h$ is imaginary and the field is exponential in all three layers; no
such field decays on both sides with a continuous slope.

A tail's decay length,

$$
\frac{1}{p} = \frac{\lambda}{2\pi\sqrt{n_\text{eff}^2 - n_3^2}} ,
$$

is Chrostowski and Hochberg's Eq. 3.3: the higher the effective index, the shorter the tail. For
220 nm of silicon in oxide at 1550 nm it is 100.6 nm for TE₀ and 169.4 nm for TM₀ (exact).

**Try it.** Thicken the film to 1000 nm. How many modes does it hold, and how do their fields
differ?

:::answer
Five TE and five TM modes ($V = 12.80$, Section 4); the chart draws the first three of each. TE₀
has no zero in the film, TE₁ one and TE₂ two. Each higher order has a lower index, 3.40784,
3.20627 and so on for TE, and longer tails. TE₀'s tail shortens to 79.9 nm: the thicker film
holds it more tightly.
:::

## 3. How do TE and TM modes differ?

A TE mode's electric field lies along the faces, $E_y$. A TM mode's magnetic field does, $H_y$,
and its electric field stands mostly across them, $E_x$. Across a face the normal displacement
$\varepsilon E_x$ is continuous, so $E_x$ jumps: just outside it is $(n_2/n_3)^2$ times its value
just inside, 5.78 for silicon on oxide. The dashed curve in the chart above shows the jumps. More
of a TM mode's field lies outside the film, and its effective index is lower: 2.051101 against
2.844816 for 220 nm of silicon in oxide at 1550 nm.

The difference grows with the index contrast, because the two polarizations' characteristic
equations differ only by the factors $(n_2/n_1)^2$ and $(n_2/n_3)^2$ on the tails' terms. In Yariv
and Yeh's symmetric slab, 5 µm of 1.6 in 1.5 at 1.55 µm, the factor is 1.14 and TE₀ and TM₀
differ by $2.0 \times 10^{-4}$ (1.594589 and 1.594391, exact); in silicon in oxide it is 5.78 and
they differ by 0.794.

### Why TM's index is lower {theory}

For TM, $E_x = (\beta/\omega\varepsilon_0 n^2) H_y$ and $E_z \propto (1/n^2)\thinspace\partial H_y/\partial x$.
The tangential $H_y$ and $E_z$ are continuous across a face, so $E_x$ jumps by the ratio of the
permittivities, and the field's logarithmic slope just inside the film is $(n_2/n_3)^2$ times the
one just outside, $\bar p = (n_2/n_3)^2 p$ in place of $p$. Each reflection then delays a TM wave
by more than a TE wave of the same angle, $\arctan(\bar p/h) \gt \arctan(p/h)$, and the transverse
resonance

$$
hd = m\pi + \arctan\frac{\bar q}{h} + \arctan\frac{\bar p}{h}
$$

is met at a larger $h$, which is a smaller $\beta = \sqrt{k^2 n_2^2 - h^2}$. As $n_2/n_3$ and
$n_2/n_1$ approach 1, $\bar p \to p$ and $\bar q \to q$, and the TE and TM modes of each order
become one: in weakly guiding slabs the polarization hardly matters, in silicon it decides the
mode.

**Try it.** In the field chart, set the cover to 1 (air). What happens to the jump at the upper
face, and to each mode's index?

:::answer
The jump at the upper face grows to $(3.473/1)^2 = 12.06$. TM₀'s index falls to 1.888461 and
TE₀'s only to 2.827869 (exact): the TM mode, which reaches further out, feels the lower cover
more.
:::

## 4. How many modes does a slab hold?

A slab's modes depend on its thickness, wavelength and three indices, but only through three
numbers: the normalized frequency $V$, the thickness in units of the wavelength weighted by the
contrast; the normalized index $b$, where the effective index sits between the substrate's (0)
and the film's (1); and the asymmetry $a$, 0 for a symmetric slab. The chart draws every mode's
$b$ against $V$. Each mode starts at $b = 0$, its cutoff $V_m$, and rises towards 1. In silicon in
oxide TE and TM share their cutoffs, $V = m\pi$; lower the film's index towards the substrate's and
TM's dashed curves rise onto TE's, which stay where they are.

::chart slab-bv

### Normalized frequency and index {theory}

In [Kogelnik and Ramaswamy's (1974)](https://doi.org/10.1364/AO.13.001857) variables, for a
substrate at least as high in index as the cover,

$$
V = kd\sqrt{n_2^2 - n_3^2}, \qquad
b = \frac{n_\text{eff}^2 - n_3^2}{n_2^2 - n_3^2}, \qquad
a = \frac{n_3^2 - n_1^2}{n_2^2 - n_3^2} .
$$

Then $hd = V\sqrt{1 - b}$, $pd = V\sqrt b$ and $qd = V\sqrt{b + a}$, and the TE equation of
Section 1 becomes

$$
V\sqrt{1 - b} = m\pi + \arctan\sqrt{\frac{b}{1 - b}} + \arctan\sqrt{\frac{b + a}{1 - b}} .
$$

Only $V$, $b$, $a$ and $m$ appear: one set of curves serves every TE slab of one asymmetry, whatever
its indices, thickness and wavelength, which is what makes Kogelnik and Ramaswamy's charts
universal. TM's equation multiplies the first arctangent's argument by $(n_2/n_3)^2$ and the
second's by $(n_2/n_1)^2$, so its curves depend on the indices as well.

A mode is cut off where its effective index reaches the substrate's, $b = 0$:

$$
V_m = m\pi + \arctan\sqrt a \quad (\text{TE}), \qquad
V_m = m\pi + \arctan\left(\frac{n_2^2}{n_1^2}\sqrt a\right) \quad (\text{TM}).
$$

A slab of normalized frequency $V$ guides every mode with $V_m \lt V$, and only one TE mode while
$V \lt \pi + \arctan\sqrt a$. In a symmetric slab $a = 0$: TE and TM share the cutoffs
$V_m = m\pi$, the two fundamental modes have none, and the slab holds $\lceil V/\pi \rceil$ modes of
each polarization. Yariv and Yeh write $V$ with half the thickness, so their cutoffs are $m\pi/2$
(their p. 117).

[slab_yariv_yeh](../examples/slab_yariv_yeh.rs) checks both of their worked examples to all four
printed decimals. Their asymmetric slab (1.0 / 2.0 / 1.7, $d = \lambda$) has $V = 6.620$ and
$a = 1.703$, so TE cutoffs at 0.917, 4.059 and 7.200 and TM cutoffs at 1.382, 4.523 and 7.665:
two modes of each. Their symmetric slab (1.5 / 1.6 / 1.5, 5 µm at 1.55 µm) has $V = 11.28$: four of
each.

::example slab_yariv_yeh

The same in thickness: the next chart draws the effective indices of silicon in oxide against the
film's thickness, Chrostowski and Hochberg's Fig. 3.10 solved exactly. At 1550 nm TE₁ and TM₁
appear at $d = \lambda/(2\sqrt{n_2^2 - n_3^2}) = 245.4$ nm, where $V = \pi$. The book reads the
single-mode limit as 240 nm off its numerical solver's curves (Lumerical MODE, with its own
material models) and gives it as the reason SOI wafers are 220 nm thick for 1550 nm
(Section 3.2.4).

::chart slab-neff

**Try it.** Move the wavelength to 1.31 µm. Is 220 nm of silicon still single-mode?

:::answer
No. The single-mode limit scales with the wavelength, $245.4 \times 1.31/1.55 = 207.4$ nm, so at
1310 nm the 220 nm film guides TE₁ and TM₁ too ($V = 3.333$). Their indices, 1.47107 and 1.44495,
sit just above the oxide's: their tails are long and their light barely held.
:::

## 5. What are the modes of a 220 nm silicon slab at 1550 nm?

Chrostowski and Hochberg solve 220 nm of silicon (3.473) between two claddings of oxide (1.444) at
1550 nm, the SOI wafer's film, and find effective indices of 2.845 for TE and 2.051 for TM
(Section 3.2.2, to three decimals). The slab has $V = 2.817$, below $\pi$: one mode of each
polarization. TE₀ has $b = 0.602$ and TM₀ $b = 0.213$: on the scale of $n^2$ from the oxide to the
silicon, the TE mode sits past the middle and the TM mode in the lowest quarter.

[slab_soi](../examples/slab_soi.rs) solves the same slab:

::example slab_soi

and the validation report holds both indices against the book's:

::validation mode/slab-te-book mode/slab-tm-book

The book's indices, like the example's and the charts', are held fixed. Silicon's index at 1550 nm
is 3.4757 in Li's table and silica's 1.4440 in Malitson's formula, the catalogue's
([the lesson on material dispersion](material-dispersion.md)). With 3.476 for silicon the exact
TE₀ index is 2.84778, the reference of the finite-difference ports' validation cases: 0.003 above
the book's slab, from a change of 0.003 in the film's index.

## 6. When does an asymmetric slab stop guiding?

A symmetric slab guides its fundamental modes however thin it is: their fields spread further out
as the film thins, but never stop being guided. An asymmetric one does not. Silicon on oxide under
air guides TE₀ only above 24.9 nm and TM₀ only above 103.4 nm at 1550 nm; below, the mode spreads
into the substrate, the higher-index cladding, and leaks away. The chart draws the effective
indices of that slab: each curve starts at the oxide's index at its cutoff thickness.

::chart slab-neff{cover=1}

### The cutoff of the fundamental mode {theory}

At cutoff $n_\text{eff} = n_3$, so $p = 0$ and the substrate's phase $\arctan(p/h)$ vanishes. The
cover's does not: there $q/h = \sqrt{(n_3^2 - n_1^2)/(n_2^2 - n_3^2)} = \sqrt a$. The transverse
resonance of mode 0 then needs a film at least $hd = \arctan\sqrt a$ thick in phase,
$V \gt \arctan\sqrt a$, the cutoff condition Yariv and Yeh's Eq. 3.2-14 states. For silicon on oxide
under air, $a = 0.1088$:

$$
d_0^\text{TE} = \frac{\arctan\sqrt a}{k\sqrt{n_2^2 - n_3^2}} = 24.9 \text{ nm}, \qquad
d_0^\text{TM} = \frac{\arctan\left((n_2/n_1)^2\sqrt a\right)}{k\sqrt{n_2^2 - n_3^2}} = 103.4 \text{ nm}:
$$

TM's factor $(n_2/n_1)^2 = 12.06$ on the cover's term makes its cutoff four times as thick. In a symmetric slab
both phases vanish at cutoff, and $V \gt 0$ is enough. The same asymmetry moves every higher cutoff
up: under air, silicon at 1550 nm is single-mode in TE below 270.2 nm and in TM below 348.8 nm.
[slab_yariv_yeh](../examples/slab_yariv_yeh.rs)'s asymmetric slab, Section 4, counts its modes by
these cutoffs.

**Try it.** Thin-film lithium niobate is a film on oxide, left uncladded or covered; its
extraordinary index at 1.55 µm is 2.1376 (the catalogue's congruent lithium niobate). Set the film
index to 2.138 and the cover to 1. Where are TE₀'s cutoff and the single-mode limit?

:::answer
TE₀ is guided above 91.4 nm, TM₀ above 195.7 nm, and there is one TE mode only below 582.9 nm.
The lower contrast makes the slab more asymmetric ($a = 0.437$ against silicon's 0.109) and every
thickness larger: the cutoff almost four times silicon's, the single-mode limit more than twice.
:::

## 7. How much of the light is in the film?

The confinement factor $\Gamma$ is the share of a mode's power that travels inside the film. It
sets how much of the mode a change in the film acts on, while the tails set how far the mode
reaches: to a neighbouring guide ([the directional coupler](directional-coupler.md)) or through a
thin buried oxide into the silicon substrate
([leaky and bent guides](leaky-and-bent-guides.md)). For 220 nm of silicon in oxide at 1550 nm,
$\Gamma = 0.810$ for TE₀ and 0.583 for TM₀ (exact). Thinner films keep less: at 100 nm, 0.453 and
0.036, TM₀'s tails then 618 nm long.

::chart slab-confinement

A thinner film does not make a smaller mode. TE₀'s width from one tail's $1/e$ point to the other,
$d + 2/p$ in a symmetric slab, is smallest at $d = 136$ nm, 385 nm across, and grows on both sides:
thicker, the film itself is wider; thinner, the tails grow faster than the film shrinks. TM₀'s is
smallest at $d = 274$ nm, 522 nm across (both exact, silicon in oxide at 1550 nm).

### Power and the confinement factor {theory}

A mode's power flows along $z$ with the density $S_z = (\beta/2\omega\mu_0)|E_y|^2$ for TE and
$S_z = (\beta/2\omega\varepsilon_0 n^2)|H_y|^2$ for TM, so

$$
\Gamma = \frac{\int_0^d S_z \thinspace dx}{\int_{-\infty}^{\infty} S_z \thinspace dx} .
$$

photonoxide integrates the tails in closed form,
$\int_{-\infty}^0 |E_y(0)|^2 e^{2px}\thinspace dx = |E_y(0)|^2/2p$, and the film by Simpson's rule,
each layer's share as Chilwell and Hodgkinson's Eq. 42 defines it. A TM
mode weights its field by $1/n^2$, which counts the field outside the film more than the field
inside. At 220 nm TM₀'s longer tails hold 42 % of its power. At 500 nm the order turns: TM₀ keeps
0.985 and TE₀ 0.965, since TM's larger reflection phase puts the field nearer a zero at the faces,
and its tails start from a smaller value. The transfer matrices' power shares against Chilwell and
Hodgkinson's Table 3 (the chart's own unit test checks its shares against the slab's exact field,
integrated):

::validation mode/multilayer-power-chilwell

**Try it.** In the confinement chart, set the film index to 1.996 (silicon nitride) and the
thickness to 400 nm. How much of TE₀ is in the film, and how long are its tails?

:::answer
$\Gamma = 0.723$, with tails of 252 nm on each side: two and a half times silicon's at 220 nm, for a
film almost twice as thick. A nitride guide's mode reaches further into the oxide, so two nitride
guides couple across wider gaps than two silicon ones, the coupling falling with the gap over the
tail's decay length ([the directional coupler](directional-coupler.md), Section 3).
:::

## 8. What about more layers, or a graded film?

Real stacks have more than three layers: a buried oxide over a silicon substrate, a cladding, a
second film. Two methods solve any planar stack. **Transfer matrices** carry the field and its
derivative through each layer exactly, and find the modes as the roots of one function; they
handle any number of uniform films, and leaky waves too
([Chilwell and Hodgkinson, 1984](https://doi.org/10.1364/JOSAA.1.000742)). **Finite differences**
sample the field on a grid and find the modes as eigenvalues of a matrix; they take any profile of
the permittivity, graded or lossy, and an absorbing layer for leaky waves.

### Transfer matrices {theory}

In a film of index $n_j$ and thickness $d_j$, with $\alpha_j = \sqrt{n_j^2 - n_\text{eff}^2}$ and
$\gamma_j = \alpha_j$ for TE or $\alpha_j/n_j^2$ for TM, the field and its scaled derivative at one
face follow from those at the other by

$$
M_j = \begin{pmatrix} \cos\Phi_j & -\frac{i}{\gamma_j}\sin\Phi_j \cr -i\gamma_j\sin\Phi_j & \cos\Phi_j \end{pmatrix},
\qquad \Phi_j = k\alpha_j d_j ,
$$

Chilwell and Hodgkinson's Eq. 10. The stack's matrix $M = \prod M_j$ carries them from the cover to
the substrate, and a mode decays into both:

$$
\chi(n_\text{eff}) = \gamma_c m_{11} + \gamma_c\gamma_s m_{12} + m_{21} + \gamma_s m_{22} = 0 ,
$$

their Eq. 26. With one film it is the three-layer slab's equation; photonoxide's
[multilayer](../docs/methods/multilayer.md) reproduces the slab to $10^{-12}$ and their four-layer
guide's eight bound modes to the six printed decimals:

::validation mode/multilayer-bound-chilwell

### Finite differences {theory}

On nodes $x_i$ with cells of widths $w$ (left) and $e$ (right) and permittivities $\varepsilon_w$
and $\varepsilon_e$, the TE equation becomes

$$
\frac{2}{w + e}\left[\frac{\psi_{i+1} - \psi_i}{e} - \frac{\psi_i - \psi_{i-1}}{w}\right]
+ k^2 \bar\varepsilon\thinspace\psi_i = \beta^2 \psi_i, \qquad
\bar\varepsilon = \frac{w\varepsilon_w + e\varepsilon_e}{w + e} ,
$$

an eigenproblem for $\beta^2$, second order in the grid when the faces lie on nodes. It is the
one-dimensional limit of [Fallahkhair, Li and Murphy's (2008)](https://doi.org/10.1109/JLT.2008.923643)
full-vector scheme ([planar profiles](../docs/methods/slab-fd.md)). On a 1 nm grid it finds
Chilwell and Hodgkinson's eight modes within $2 \times 10^{-6}$:

::validation mode/slab-fd-chilwell

[multilayer_chilwell](../examples/multilayer_chilwell.rs) solves their guide, its bound modes and
its leaky waves, exactly; the lesson on [leaky and bent guides](leaky-and-bent-guides.md) takes
the leaky waves further.

## 9. How does photonoxide solve a slab?

`mode::slab::Slab` takes the three indices and the thickness, and `modes` returns each
polarization's guided modes at a wavelength, fundamental first, by bisecting the transverse
resonance of Section 1 to the last bit for each $m$ until a mode is cut off
([the exact slab](../docs/methods/slab.md)). Each mode gives its effective index, its $h$, $q$ and
$p$, and its field, Yariv and Yeh's Eqs. 3.2-3 and 3.2-10. Its tests check the roots against the
tangent form to $10^{-9}$, the asymmetric cutoff, and the fields' continuity at the faces. The
charts here call it directly; the confinement chart takes each mode's power from
`mode::multilayer`.

The slab is also the reference for the numerical solvers. The full-vector solver, given a
cross-section uniform along one axis, must find the slab's modes; it converges onto them at second
order and is within $10^{-4}$ at a 2.5 nm grid ([the full-vector solver](../docs/methods/vector.md)):

::validation mode/vector-slab-limit-te mode/vector-slab-limit-tm

The group index of 220 nm of silicon between oxide and air, $n_g = n_\text{eff} - \lambda\thinspace dn_\text{eff}/d\lambda$
from differences of the exact TE₀ index over ±2 nm, agrees to $10^{-6}$ with
$\langle\varepsilon\rangle/n_\text{eff}$, the permittivity averaged with the exact field's $E_y^2$
as its weight, which is the group index of a slab whose materials don't disperse:

::validation mode/slab-group-index

A strip waveguide is a slab cut to a width; the
[next lesson](strip-waveguide.md) solves it by slabs twice, the effective index method, and in
full.

## 10. History and current research {research}

That totally reflected light enters the rarer medium before it returns was shown by
[Goos and Hänchen (1947)](https://doi.org/10.1002/andp.19474360704), whose beam came back shifted
along the face. In 1969 [Miller](https://doi.org/10.1002/j.1538-7305.1969.tb01165.x) proposed
integrated optics: laser beams about 10 µm wide guided by index changes of $10^{-2}$ to $10^{-3}$
in a substrate such as glass, in circuits patterned by photolithography.
[Tien and Ulrich (1970)](https://doi.org/10.1364/JOSA.60.001325) coupled light into films through
a prism and read the film's index and thickness off its modes, analysed by rays and waves together;
[Tien (1971)](https://doi.org/10.1364/AO.10.002395) reviewed the field.
[Kogelnik and Ramaswamy (1974)](https://doi.org/10.1364/AO.13.001857) normalized the asymmetric
slab into universal charts, the b–V diagram of Section 4, and
[Kogelnik (1975)](https://doi.org/10.1007/BFb0103618) set out the theory of dielectric guides in
Tamir's *Integrated Optics*. Marcuse's *Theory of Dielectric Optical Waveguides* opens on the
asymmetric slab ([second edition, 1991](https://doi.org/10.1016/B978-0-12-470951-5.50007-X)), and
Yariv and Yeh's *Photonics* gives the equations photonoxide's slab solves.
[Chilwell and Hodgkinson (1984)](https://doi.org/10.1364/JOSAA.1.000742) extended the exact solution
to any planar stack with transfer matrices, and finite-difference solvers
([Fallahkhair et al., 2008](https://doi.org/10.1109/JLT.2008.923643)) to any profile.

::timeline

Today's planar platforms are slabs of high contrast, cut into guides. **Silicon on insulator**:
220 nm of silicon on oxide, made in CMOS foundries
([Soref, 2006](https://doi.org/10.1109/JSTQE.2006.883151);
[Chrostowski and Hochberg, 2015](https://doi.org/10.1017/CBO9781316084168)), its contrast the
highest, 3.47 against 1.44, and its single-mode film the thinnest, 245 nm at 1550 nm.
**Silicon nitride**: about 2.00 on oxide, low loss and transparent from 400 to 2350 nm
([Blumenthal et al., 2018](https://doi.org/10.1109/JPROC.2018.2861576)), single-mode in oxide
below 562 nm. **Thin-film lithium niobate**: a film sliced from a crystal and bonded to oxide,
whose index contrast is about ten times that of guides diffused into bulk crystals, and whose
losses fell from more than 6 dB/cm in 2007 to 0.027 dB/cm in 2017
([Zhu et al., 2021](https://doi.org/10.1364/AOP.411024)); in oxide it is single-mode below 492 nm.
These thicknesses are the slab's $V = \pi$ for each film in oxide, from the index chart (exact,
fixed indices: 3.473, 1.996 and 2.138). The diffused guides' low contrast is what Zhu et al. name
as their limit: modes of 10 to 100 µm², bends of millimetres, no compact rings; the film's
contrast removes it.

Further reading: Chrostowski and Hochberg's Section 3.2 for the silicon slab and how to simulate
it; [Kogelnik and Ramaswamy (1974)](https://doi.org/10.1364/AO.13.001857) for the normalized
charts; Yariv and Yeh's Section 3.2 for the derivations; and
[Chilwell and Hodgkinson (1984)](https://doi.org/10.1364/JOSAA.1.000742) for stacks of many layers.

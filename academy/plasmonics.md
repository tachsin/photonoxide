---
title: "Surface plasmons and Mie scattering"
summary: "Light bound to the electrons of a metal: the Drude permittivity and why it is negative, the surface plasmon of a metal–dielectric interface with its dispersion, reach and loss, Mie's exact scattering by a sphere, the localized resonance of a small metal sphere, where the free-electron model fails for gold, and how FDTD models a metal."
topic: Nanophotonics
level: advanced
minutes: 45
prerequisites: [material-dispersion, slab-waveguide]
examples: [lorentz_okoniewski]
methods: [materials.md, multilayer.md, fdtd.md]
validation:
  - fdtd/mie-table
  - fdtd/mie-balance
  - fdtd/mie-terms
  - fdtd/drude-fresnel
  - fdtd/mie-sphere
  - fdtd/mie-sphere-staircase
  - fdtd/mie-drude
charts: [plasmon-drude, plasmon-dispersion, plasmon-length, mie-efficiency, mie-plasmon, mie-gold]
papers:
  - cite: "P. Drude, Ann. Phys. 306, 566 (1900)"
    title: "Zur Elektronentheorie der Metalle"
    doi: 10.1002/andp.19003060312
    year: 1900
    role: origin
    note: "The electron theory of metals: free electrons that carry the current and heat, and, for gold's and copper's optical constants, bound electrons as well."
  - cite: "E. Hagen, H. Rubens, Ann. Phys. 313, 1 (1902)"
    title: "Das Reflexionsvermögen einiger Metalle für ultraviolette und ultrarote Strahlen"
    doi: 10.1002/andp.19023130502
    year: 1902
    role: milestone
    note: "The normal reflectivity of gold, silver, copper, platinum, nickel, steel and other metals from 251 nm to 1.5 µm (their Table 4); for gold, 0.27 to 0.33 from 357 to 450 nm and 0.889 at 650 nm, the reflectivity Mie took his gold's index from."
  - cite: "E. Hagen, H. Rubens, Ann. Phys. 313, 432 (1902)"
    title: "Die Absorption ultravioletter, sichtbarer und ultraroter Strahlen in dünnen Metallschichten"
    doi: 10.1002/andp.19023130613
    year: 1902
    role: milestone
    note: "The absorption of thin metal films, and from it the imaginary part of the index; for gold, films 53 to 101 nm thick from 0.33 to 2.5 µm, nκ = 3.58 at 650 nm, the other half of Mie's gold."
  - cite: "G. Mie, Ann. Phys. 330, 377 (1908)"
    title: "Beiträge zur Optik trüber Medien, speziell kolloidaler Metallösungen"
    doi: 10.1002/andp.19083300302
    year: 1908
    role: origin
    note: "The exact scattering of a plane wave by a sphere as a series of electric and magnetic partial waves, and with it the colours of colloidal gold: absorption peaking in the green near 525 nm for the finest particles, moving to the red and turning the solutions violet and blue as they grow past about 100 nm."
  - cite: "R. H. Ritchie, Phys. Rev. 106, 874 (1957)"
    title: "Plasma losses by fast electrons in thin films"
    doi: 10.1103/PhysRev.106.874
    year: 1957
    role: milestone
    note: "Fast electrons crossing a thin metal foil lose energy below the plasma energy as well as at it: the foil's surfaces bring a loss at ħω_p/√2, split in two as the foil thins, the surface plasmon's prediction."
  - cite: "M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave Lett. 7, 121 (1997)"
    title: "Simple treatment of multi-term dispersion in FDTD"
    doi: 10.1109/75.569723
    year: 1997
    role: milestone
    note: "Several Debye or Lorentz terms in FDTD by auxiliary differential equations, as cheap in memory and operations as the recursive convolutions and without their assumption of a linear medium: the scheme photonoxide's dispersive media use."
---

At the left, a surface plasmon: a wave of charge on a flat interface between a dielectric and a
metal, its field reaching further into the dielectric than into the metal. At the right, a metal
sphere much smaller than the wavelength, its electrons pushed to one side by the light's field.
The symbols are the ones the equations below use.

::diagram plasmon

Below a frequency set by its electron density, a metal's permittivity is negative. That lets a
metal hold light in ways a glass cannot: as a wave bound to its surface, shorter than light in the
dielectric beside it, and as a resonance of a particle far smaller than the wavelength. Both are
plasmons, the light and the electrons' motion as one wave. This lesson derives them from the
free-electron metal of [Drude (1900)](https://doi.org/10.1002/andp.19003060312), solves them with
photonoxide's planar-stack modes and its implementation of Mie's series
([Mie, 1908](https://doi.org/10.1002/andp.19083300302)), and sets them against Mie's gold.

## 1. What is a metal's permittivity at optical frequencies?

A metal's conduction electrons are free: no spring holds them, so the light's field drives them
without a resonance. Their permittivity is the Drude form,
$\varepsilon(\omega) = \varepsilon_\infty - \omega_p^2/(\omega^2 + i\gamma\omega)$, with the plasma
frequency $\omega_p$ set by the electron density and the collision rate $\gamma$ by their loss
(derived below). Below $\omega_p$ the real part is negative: the electrons move against the field
that drives them, the wave inside decays within a fraction of a wavelength, and the metal reflects.
Above $\omega_p$ it is positive, and the metal turns transparent.

The chart draws $\varepsilon$ for $\varepsilon_\infty = 1$ and $\gamma = 0.02\thinspace\omega_p$,
against the frequency in units of $\omega_p$ (exact, no grid). Re $\varepsilon$ crosses zero at
$0.9998\thinspace\omega_p$; at $0.5\thinspace\omega_p$, $\varepsilon = -2.994 + 0.160i$ and the
metal reflects 0.9549 of the light at normal incidence from vacuum, at $1.5\thinspace\omega_p$
only 0.0213. The dashed lines mark $\varepsilon = -1$, reached at $0.7068\thinspace\omega_p$, where
a flat surface in vacuum holds its surface plasmon (Section 2), and $\varepsilon = -2$, at
$0.5770\thinspace\omega_p$, where a small sphere in vacuum resonates (Section 5).

::chart plasmon-drude

### The Drude permittivity {theory}

An electron of charge $-e$ and mass $m$, driven by $E e^{-i\omega t}$ and slowed by collisions at
the rate $\gamma$, obeys

$$
m\ddot x = -eE e^{-i\omega t} - m\gamma\dot x .
$$

With $x = x_0 e^{-i\omega t}$, $-m\omega^2 x_0 = -eE + i m\gamma\omega x_0$, so
$x_0 = eE/[m(\omega^2 + i\gamma\omega)]$. $N$ electrons in a unit volume give the polarization
$P = -Nex_0 = -\varepsilon_0\omega_p^2 E/(\omega^2 + i\gamma\omega)$ with

$$
\omega_p^2 = \frac{Ne^2}{\varepsilon_0 m}, \qquad
\varepsilon(\omega) = \varepsilon_\infty - \frac{\omega_p^2}{\omega^2 + i\gamma\omega}
= \varepsilon_\infty - \frac{\omega_p^2}{\omega^2 + \gamma^2} + i\frac{\omega_p^2\gamma}{\omega(\omega^2 + \gamma^2)} ,
$$

$\varepsilon_\infty$ the permittivity every other charge leaves. It is the bound oscillator of
[the material lesson](material-dispersion.md) with its spring removed, $\omega_0 = 0$. Re
$\varepsilon = 0$ at $\omega^2 = \omega_p^2/\varepsilon_\infty - \gamma^2$, the chart's first figure.
Below it, $n = \sqrt\varepsilon$ is mostly imaginary, and Fresnel's
$r = (1 - n)/(1 + n)$ has $\lvert r\rvert$ near 1. photonoxide takes the same form as
`material::Model::Drude` ([the materials](../docs/methods/materials.md)), its plasma frequency
$f_p = \omega_p/2\pi$ and its collision rate $\gamma/2\pi$ in c/µm.

### Drude's electrons {research}

[Drude (1900)](https://doi.org/10.1002/andp.19003060312) built his electron theory of metals to
explain their electrical and thermal conduction together. He notes there that gold's and copper's
optical properties need bound electrons, held at positions of equilibrium, as well as free ones,
and that only the bound ones matter beyond the optics. Section 6 shows where gold needs them.

**Try it.** Raise $\varepsilon_\infty$ to 4. Where does Re $\varepsilon$ cross zero now?

:::answer
At $0.4996\thinspace\omega_p$, $\sqrt{1/4 - 0.02^2}$: the bound charges' $\varepsilon_\infty$ pulls
the crossing below $\omega_p$, as $1/\sqrt{\varepsilon_\infty}$.
:::

## 2. What is a surface plasmon?

On a flat interface between a dielectric of $\varepsilon_d > 0$ and a metal of
$\varepsilon_m < -\varepsilon_d$, one wave is bound: a TM wave, its magnetic field along the
interface and across the direction it travels, whose field falls away exponentially on both
sides. Its wavenumber along the interface is

$$
k = k_0\sqrt{\frac{\varepsilon_m\varepsilon_d}{\varepsilon_m + \varepsilon_d}} ,
\qquad k_0 = \frac{\omega}{c} ,
$$

derived below. With $\varepsilon_m$ negative and larger than $\varepsilon_d$ in size, the fraction
under the root exceeds $\varepsilon_d$: $k$ lies beyond the dielectric's light line
$k_0\sqrt{\varepsilon_d}$, so no light in the dielectric matches it, and it can't radiate into
the dielectric.

The chart solves it as photonoxide solves any planar guide, by the transfer matrix of
[the multilayer](../docs/methods/multilayer.md) (`Multilayer::mode_near`, TM), here a dielectric
over a Drude metal with $\varepsilon_\infty = 1$, and draws its frequency against Re $k$. Far
below $\omega_p$ it hugs the light line; at half the surface plasma frequency
$\omega_\text{sp} = \omega_p/\sqrt{1 + \varepsilon_d}$, 0.7071 $\omega_p$ in vacuum, $k$ is
1.0803 times the light's. Near $\omega_\text{sp}$ it flattens: Re $k$ reaches $3.409\thinspace\omega_p/c$
at $0.7042\thinspace\omega_p$ with $\gamma = 0.01\thinspace\omega_p$, and the loss bends it back. Light in
the metal exists only above $\omega_p$; between $\omega_\text{sp}$ and $\omega_p$ no wave travels
on the surface or in the metal. The stack's mode is the closed form above to $1.2 \times 10^{-14}$
at every frequency (exact, no grid).

::chart plasmon-dispersion

### The dispersion relation {theory}

Take the interface at $x = 0$, the dielectric above, the metal below, and a TM wave travelling
along $z$, $H_y = A_j e^{ikz - i\omega t}e^{-\kappa_j\lvert x\rvert}$ in medium $j$, with
$\kappa_j^2 = k^2 - \varepsilon_j k_0^2$ and Re $\kappa_j > 0$ so that it is bound. Maxwell's
$\nabla\times H = -i\omega\varepsilon_0\varepsilon E$ gives the tangential
$E_z = (i/\omega\varepsilon_0\varepsilon)\thinspace\partial H_y/\partial x$. $H_y$ is continuous,
so $A_d = A_m$, and so is $E_z$:

$$
-\frac{\kappa_d}{\varepsilon_d} = \frac{\kappa_m}{\varepsilon_m}
\quad\Longrightarrow\quad
\frac{\kappa_d}{\varepsilon_d} + \frac{\kappa_m}{\varepsilon_m} = 0 .
$$

Both $\kappa$ are positive, so $\varepsilon_m$ and $\varepsilon_d$ have opposite signs: only a
metal (or another medium of negative permittivity) holds the wave. Squaring,
$(k^2 - \varepsilon_d k_0^2)\varepsilon_m^2 = (k^2 - \varepsilon_m k_0^2)\varepsilon_d^2$, so
$k^2(\varepsilon_m^2 - \varepsilon_d^2) = k_0^2\varepsilon_m\varepsilon_d(\varepsilon_m - \varepsilon_d)$
and $k^2 = k_0^2\varepsilon_m\varepsilon_d/(\varepsilon_m + \varepsilon_d)$. For a real $k$
beyond the light line the denominator must be negative, $\varepsilon_m < -\varepsilon_d$; with the
Drude $\varepsilon_m = 1 - \omega_p^2/\omega^2$, $\varepsilon_m = -\varepsilon_d$ at
$\omega_\text{sp} = \omega_p/\sqrt{1 + \varepsilon_d}$, where $k$ grows without bound if there is
no loss.

**Why no TE wave.** For TE, $E_y$ and $\partial E_y/\partial x$ are continuous, which asks
$-\kappa_d = \kappa_m$: no two positive $\kappa$ satisfy it, whatever the permittivities. The
chart's tests check it on the stack: the TE iteration started on the plasmon's $k$ finds no mode
beyond the light line.

### Ritchie's thin films {research}

[Ritchie (1957)](https://doi.org/10.1103/PhysRev.106.874) computed the energy fast electrons lose
to a metal foil's conduction electrons. The foil's surfaces brought a loss below the plasma
energy, at $\hbar\omega_p/\sqrt 2$, $\omega_\text{sp}$ in vacuum, negligible in a thick foil and
growing as it thins, and resonances at $\omega_p\sqrt{1 \pm e^{-ka}}/\sqrt 2$ for a foil of
thickness $a$ and a wavenumber $k$ along it: the two faces' surface plasmons, coupled through the
foil. He set these beside the low-lying losses
experimenters had seen in thin films, and noted that, after Jensen, a small sphere's electrons
resonate at $\omega_p/\sqrt 3$ instead (Section 5).

**Try it.** Put the metal under glass, $n_d$ = 1.5. Where is $\omega_\text{sp}$, and how far does
the curve reach?

:::answer
$\omega_\text{sp} = \omega_p/\sqrt{3.25} = 0.5547\thinspace\omega_p$, and Re $k$ reaches
$4.164\thinspace\omega_p/c$ at $0.5519\thinspace\omega_p$: a denser dielectric lowers the plasmon's
frequency and shortens its wavelength.
:::

## 3. How far does a surface plasmon travel, and how tightly is it held?

The metal's loss makes $k$ complex. Its imaginary part sets the **propagation length**
$L = 1/(2\mathop{\mathrm{Im}} k)$, over which the power falls by $e$; the decay constants
$\kappa_d$ and $\kappa_m$ set how far the field reaches into the dielectric and into the metal,
$\delta_d = 1/\mathop{\mathrm{Re}}\kappa_d$ and $\delta_m = 1/\mathop{\mathrm{Re}}\kappa_m$. The chart
draws the three against frequency up to $\omega_\text{sp}$, in units of the plasma wavelength
$\lambda_p = 2\pi c/\omega_p$, for vacuum over a Drude metal with $\gamma = 0.01\thinspace\omega_p$
(exact, no grid):

- at $0.2\thinspace\omega_\text{sp}$ ($0.1414\thinspace\omega_p$), the plasmon travels
  $741\thinspace\lambda_p$ and reaches $7.79\thinspace\lambda_p$ into the vacuum, more than the
  vacuum wavelength of $7.07\thinspace\lambda_p$;
- at $0.5\thinspace\omega_\text{sp}$, $77.4\thinspace\lambda_p$ and $1.10\thinspace\lambda_p$;
- at $0.9\thinspace\omega_\text{sp}$, $2.52\thinspace\lambda_p$ and $0.172\thinspace\lambda_p$.

The depth in the metal stays near $0.12$ to $0.16\thinspace\lambda_p$ throughout: the skin depth.
Holding the field closer to the surface costs propagation: the ratio $L/\delta_d$ falls from 95 to
15 between the first and last of these frequencies, because the closer the plasmon is held, the
more of its energy is in the metal.

::chart plasmon-length

Mie's gold at 650 nm (his table on p. 417, from Hagen and Rubens's measurements) has
$\varepsilon_m = -12.36 + 2.90i$. In air the plasmon on it has $k = (1.0405 + 0.0101i)k_0$, travels
5.10 µm and reaches 357 nm into the air and 28 nm into the gold; under water ($n$ = 1.335),
1.94 µm, 194 nm and 27 nm (the closed form, exact).

### The lengths from the complex wavenumber {theory}

With $\varepsilon_m = \varepsilon' + i\varepsilon''$ and $\varepsilon'' \ll \lvert\varepsilon'\rvert$,
expand $k^2 = k_0^2\varepsilon_m\varepsilon_d/(\varepsilon_m + \varepsilon_d)$ to first order in
$\varepsilon''$: its derivative in $\varepsilon_m$ is $k_0^2\varepsilon_d^2/(\varepsilon_m + \varepsilon_d)^2$, so

$$
\mathop{\mathrm{Im}}(k^2) \approx k_0^2\frac{\varepsilon_d^2\varepsilon''}{(\varepsilon' + \varepsilon_d)^2},
\qquad
L = \frac{1}{2\mathop{\mathrm{Im}}k} = \frac{\mathop{\mathrm{Re}}k}{\mathop{\mathrm{Im}}(k^2)}
\approx \frac{\mathop{\mathrm{Re}}k\thinspace(\varepsilon' + \varepsilon_d)^2}{k_0^2\varepsilon_d^2\varepsilon''} .
$$

$L$ grows as $1/\varepsilon''$, and, for a Drude metal, as $1/\gamma$: with
$\gamma = 0.002\thinspace\omega_p$ the chart gives $387\thinspace\lambda_p$ at
$0.5\thinspace\omega_\text{sp}$, five times the $77.4\thinspace\lambda_p$ at $0.01\thinspace\omega_p$. It
falls as $\varepsilon' \to -\varepsilon_d$, near $\omega_\text{sp}$. The boundary condition of
Section 2 gives the depths' ratio exactly without loss,
$\delta_d/\delta_m = \kappa_m/\kappa_d = \lvert\varepsilon_m\rvert/\varepsilon_d$: a good metal, $\lvert\varepsilon_m\rvert \gg \varepsilon_d$,
holds most of the field outside itself. And
$\kappa_d^2 = k^2 - \varepsilon_d k_0^2 = k_0^2\varepsilon_d^2/(\lvert\varepsilon'\rvert - \varepsilon_d)$, so
$\delta_d \approx \lambda\sqrt{\lvert\varepsilon'\rvert}/2\pi\varepsilon_d$ for
$\lvert\varepsilon'\rvert \gg \varepsilon_d$: the plasmon reaches beyond a wavelength into the
dielectric where the metal is a good one.

The same holds for any guide whose light runs partly in a metal: confining it tighter puts more of
it in the metal, and shortens how far it goes. The plasmon on gold at 650 nm loses
$10\log_{10}e$ = 4.34 dB every 5.10 µm, 8500 dB/cm, against the 3 dB/cm the charts of
[the ring resonator](ring-resonator.md) take for a silicon wire.

**Try it.** At $0.5\thinspace\omega_\text{sp}$, put the metal under glass ($n_d$ = 1.5). What
happens to the propagation length and the depth?

:::answer
$L$ falls from $77.4$ to $38.3\thinspace\lambda_p$ and $\delta_d$ from $1.10$ to
$0.796\thinspace\lambda_p$: the denser dielectric lowers $\omega_\text{sp}$ to $0.5547\thinspace\omega_p$,
where at half of it, $0.2774\thinspace\omega_p$, the metal's $\varepsilon'$ is smaller against
$\varepsilon_d$ = 2.25.
:::

## 4. What does Mie's series give?

A plane wave on a sphere of any size and index has an exact solution, Mie's: the scattered field
is a series of outgoing partial waves, electric and magnetic, each weighted by a coefficient,
$a_\nu$ and $b_\nu$, that depends on the size parameter $\alpha = 2\pi n_0\rho/\lambda$ and the
sphere's index relative to the medium, $m$ ([the FDTD write-up](../docs/methods/fdtd.md#scattering-by-a-sphere)
has them in photonoxide's convention). The power the sphere takes from the wave and the power it
scatters, over its shadow $\pi\rho^2$, are its extinction and scattering efficiencies,

$$
Q_\text{ext} = \frac{2}{\alpha^2}\sum_\nu (2\nu + 1)\mathop{\mathrm{Re}}(a_\nu + b_\nu),
\qquad
Q_\text{sca} = \frac{2}{\alpha^2}\sum_\nu (2\nu + 1)\left(\lvert a_\nu\rvert^2 + \lvert b_\nu\rvert^2\right) ,
$$

and the absorption is their difference (Mie's §26, his Eq. 98, as the write-up converts them).

The chart draws them for a glass sphere, $m = 1.5$, against $\alpha$ (exact, no grid). Small
spheres scatter as Rayleigh's $(8/3)\alpha^4\lvert(m^2 - 1)/(m^2 + 2)\rvert^2$, the series' first
term (dashed): at $\alpha = 0.1$ the series is $7.0 \times 10^{-4}$ above it. $Q_\text{ext}$ peaks
at 4.364 at $\alpha = 4.312$: the sphere takes from the beam more than four times the light its
shadow blocks. Beyond, it oscillates towards 2 (2.353 at $\alpha = 30$): a large sphere takes the
light its shadow blocks and as much again by diffraction at its edge. Lossless, it scatters all it
takes, and the absorption is zero.

::chart mie-efficiency

The series is checked against Mie's own Table I and against itself:

::validation fdtd/mie-table fdtd/mie-balance fdtd/mie-terms

### How the series is summed {theory}

Mie writes each coefficient in Riccati–Bessel functions, $\psi_\nu(x) = xj_\nu(x)$ and the
outgoing $\xi_\nu(x) = xh^{(1)}_\nu(x)$, of $\alpha$ and of $m\alpha$, all from his recurrence
$(2\nu + 1)f_\nu/x = f_{\nu - 1} + f_{\nu + 1}$ (his Eq. 26). photonoxide's `fdtd::Mie` steps
$\xi_\nu$ up from $\xi_0$ and $\xi_1$, and $\psi_\nu$, which falls past $\nu \approx \alpha$ and
can't be stepped up, from its logarithmic derivative stepped down; the sphere's own
$\psi_\nu(m\alpha)$, exponentially large in a metal, never appears, only its logarithmic derivative.
The terms fall faster than exponentially once $\nu$ passes $\alpha$, and the sum stops where a term
adds less than $10^{-17}$ of it ([the FDTD write-up](../docs/methods/fdtd.md#scattering-by-a-sphere)).

### Mie's paper {research}

[Mie (1908)](https://doi.org/10.1002/andp.19083300302) wrote the series to explain the colours of
colloidal metal solutions, which Maxwell Garnett had explained for particles far smaller than the
wavelength (Mie's introduction), and computed by hand its first coefficients for gold spheres in water at seven
wavelengths (his Tables I to III). Of the 66 entries of his Table I, the series reproduces 61 to
within 0.016 of his three digits, the median 3.0e-3; the other five, near gold's resonance where his
expansion in $\alpha^2$ converges worst, differ from his by 0.04 to 0.40, and the series agrees
there with $a_1$'s closed form to $10^{-13}$.

**Try it.** Lower the index to 1.33, a water droplet in air. Where is the first maximum now?

:::answer
At $\alpha = 6.487$, $Q_\text{ext}$ = 3.982. Both maxima fall where $2\alpha(m - 1)$, the phase a ray
through the sphere's centre gains over one beside it, is near 4.3: 4.31 for glass, 4.28 for water.
:::

## 5. What is a localized plasmon resonance?

A metal sphere much smaller than the wavelength sees a field that is uniform across it. The field
pushes its electrons to one side; the charge left on its surface pulls them back; and at one
frequency the push and the pull resonate. The sphere's dipole is
$p = 4\pi\varepsilon_0\varepsilon_d\rho^3 E_0(\varepsilon - \varepsilon_d)/(\varepsilon + 2\varepsilon_d)$,
largest where $\varepsilon = -2\varepsilon_d$ (derived below); for a Drude metal in vacuum, at
$\omega_p/\sqrt 3$, the frequency Ritchie gives after Jensen for plasma in a small sphere.

The chart draws Mie's efficiencies for a Drude sphere of radius $0.02\thinspace\lambda_p$ in
vacuum, with $\gamma = 0.03\thinspace\omega_p$, and the quasi-static dipole's extinction dashed
(computed in the studio). The extinction peaks at $0.5760\thinspace\omega_p$, $Q_\text{ext}$ =
5.552: the sphere takes from the beam five and a half times its shadow. The dipole peaks at
$0.5775\thinspace\omega_p$, against $\omega_p/\sqrt 3 = 0.5774\thinspace\omega_p$ without loss.
Its full width at half height, $0.0300\thinspace\omega_p$, is $\gamma$, and 0.0049 of the
extinction is scattered: a small sphere absorbs (exact, no grid).

::chart mie-plasmon

### The quasi-static sphere {theory}

With the sphere small, $\nabla\times E \approx 0$ and $E = -\nabla\phi$ with $\nabla^2\phi = 0$
inside and out. A uniform field $E_0$ along $z$ gives $\phi = -E_0 r\cos\theta$ far away; the
solutions are $\phi_\text{in} = -Br\cos\theta$ and
$\phi_\text{out} = -E_0 r\cos\theta + C\cos\theta/r^2$. Continuity of $\phi$ and of
$\varepsilon\thinspace\partial\phi/\partial r$ at $r = \rho$ gives

$$
B = \frac{3\varepsilon_d}{\varepsilon + 2\varepsilon_d}E_0, \qquad
C = \frac{\varepsilon - \varepsilon_d}{\varepsilon + 2\varepsilon_d}\rho^3 E_0 ,
$$

the field of a dipole $p = 4\pi\varepsilon_0\varepsilon_d C$ outside. Its polarizability
$\alpha_p = 4\pi\rho^3 F$, $F = (\varepsilon - \varepsilon_d)/(\varepsilon + 2\varepsilon_d)$,
absorbs $C_\text{abs} = k\mathop{\mathrm{Im}}\alpha_p$ and scatters $C_\text{sca} = k^4\lvert\alpha_p\rvert^2/6\pi$,
$k = n_d k_0$; over $\pi\rho^2$, with $x = k\rho$,

$$
Q_\text{abs} = 4x\mathop{\mathrm{Im}}F, \qquad Q_\text{sca} = \frac{8}{3}x^4\lvert F\rvert^2 ,
$$

Mie's first electric term for small $\alpha$, his $(\mathfrak{a}_1)_0 = (m'^2 - 1)/(m'^2 + 2)$ (p. 418),
and Rayleigh's scattering. $F$ peaks where $\varepsilon + 2\varepsilon_d$ is smallest. For the
Drude metal with $\varepsilon_\infty = 1$,
$\varepsilon + 2\varepsilon_d = 0$ at $\omega^2 + i\gamma\omega = \omega_p^2/(1 + 2\varepsilon_d)$,
$\omega \approx \omega_F - i\gamma/2$ with $\omega_F = \omega_p/\sqrt{1 + 2\varepsilon_d}$: a
resonance of full width $\gamma$ at $\omega_F$, as the chart's smallest spheres show. Absorption
grows as $x$ and scattering as $x^4$, so small spheres absorb and larger ones scatter.

### Larger spheres {theory}

The quasi-static dipole doesn't depend on the size. Mie's series does: the field's phase changes
across a larger sphere, its dipole radiates and loses energy to it, and higher multipoles join. On
the same metal in vacuum the extinction's peak and full width are:

| $\rho/\lambda_p$ | peak, $\omega_p$ | full width, $\omega_p$ | scattered share |
|---|---|---|---|
| 0.005 | 0.5775 | 0.0300 | 7.7e-5 |
| 0.02 | 0.5760 | 0.0300 | 4.9e-3 |
| 0.05 | 0.5700 | 0.0314 | 0.069 |
| 0.1 | 0.5505 | 0.0423 | 0.352 |
| 0.2 | 0.4930 | 0.101 | 0.765 |

The resonance moves to lower frequencies and widens as the sphere grows, and between
$0.1$ and $0.2\thinspace\lambda_p$ the sphere comes to scatter more than it absorbs (Mie's series,
exact, no grid).

::chart mie-plasmon{radius=0.1}

**Try it.** Put the $0.02\thinspace\lambda_p$ sphere in water, $n_d$ = 1.33. Where is the peak?

:::answer
At $0.4680\thinspace\omega_p$, against $\omega_p/\sqrt{1 + 2 \times 1.769} = 0.4694\thinspace\omega_p$:
the denser medium lowers the resonance, as $1/\sqrt{1 + 2\varepsilon_d}$.
:::

## 6. Where does the free-electron model fail for a real metal?

Mie took gold's index from a smooth curve through Hagen and Rubens's measurements of its
reflection and absorption, at seven wavelengths (his table on p. 417). Squared, in photonoxide's convention, its
permittivity relative to vacuum is:

| λ, nm | 420 | 450 | 500 | 525 | 550 | 600 | 650 |
|---|---|---|---|---|---|---|---|
| gold, Mie | 0.00 + 5.78i | 0.03 + 5.95i | −2.87 + 4.44i | −4.35 + 3.52i | −5.68 + 2.79i | −8.62 + 2.25i | −12.36 + 2.90i |
| free electrons | −4.73 + 0.80i | −5.56 + 0.99i | −7.06 + 1.35i | −7.86 + 1.55i | −8.69 + 1.78i | −10.46 + 2.30i | −12.36 + 2.90i |

The second row is the Drude metal ($\varepsilon_\infty = 1$) through gold's value at 650 nm:
$\lambda_p$ = 174 nm and $\gamma = 0.0581\thinspace\omega_p$ (derived from his two numbers, exact).
Towards the blue the two part: at 420 nm gold's Re $\varepsilon$ has risen to zero and its Im
$\varepsilon$ is 5.78, seven times the free electrons'. Something other than the free electrons
absorbs there; [Drude (1900)](https://doi.org/10.1002/andp.19003060312) had already found that
gold's optical constants need bound electrons. Re $\varepsilon$ relative to water reaches $-2$
between 500 and 525 nm, Mie's $m'^2$ going from $-1.60$ to $-2.45$; for the fitted free-electron
metal it would be at 374 nm.

The chart computes, with photonoxide's series and Mie's gold (`Mie::gold_in_water`), what he
computed: the light 1 mm of a solution of 1 mm³ of gold per litre of water takes from a beam, in
thousandths, for spheres of one diameter (exact, no grid; the points are his seven wavelengths).
His numbers for the nearest diameter of his table on p. 438 are drawn dashed. Spheres 20 nm
across take most at 525 nm, 72.6 ‰ against his 76.6, and absorb 0.993 of it. The free-electron
metal's extinction keeps rising towards 420 nm, its resonance being at 374.

::chart mie-gold

Mie's table runs from his finest particles to 180 nm. At 1 nm the series gives his finest row to
1.2 %, its peak at 525 nm (69.1 ‰, his 69.0); over his 70 numbers the median difference is 1.4 %,
and every row peaks at his wavelength but the last, where his 550 and 600 nm differ by 2 %. At
100 nm the peak has moved to 600 nm, 86.3 ‰ against his 85.0, and 0.648 of it is scattered.
Mie's numbers for spheres of 20 to 60 nm at 650 nm are not reproduced: he has 8.0, 11.3 and 16.4 ‰,
the series 7.4, 9.4 and 14.7, 17 % below at 40 nm. Keeping only the first two electric and
magnetic terms, which hold his three, changes the series' numbers there by less than 0.1 ‰, so his
truncation isn't the cause. His curves are. He computed the absorption only at the sizes of his
Tables I to III, $\alpha^2$ = 0, 0.2, 0.4 and on, which at 650 nm are spheres 0, 69, 98 nm across
and larger, and read the table off curves drawn through those points against the diameter
(p. 437). Between 0 and 69 nm the series' extinction bends upward, as the sphere's resonance moves
towards the red, so a curve through the two points that starts flat (a straight line in
$\alpha^2$) stands above it: through the series' own value at $\alpha^2$ = 0.2 it gives 7.9, 11.0 and
16.1 ‰ at 20, 40 and 60 nm, and through his, $\mathfrak{a}_1$ = 1.920 − 0.515i where the series has
1.815 − 0.484i (one of the five entries of his Table I that the series doesn't reproduce), 8.0,
11.4 and 17.0. His 8.0, 11.3 and 16.4 lie on or between the two.

Nor is the gold the cause. [Hagen and Rubens (1902)](https://doi.org/10.1002/andp.19023130502)
measured gold's reflectivity $R$ (their Table 4, p. 17), and from thin films [its absorption
(1902)](https://doi.org/10.1002/andp.19023130613), $g = n\kappa$ (Table 4, p. 447); Mie's table on
p. 417 quotes both columns digit for digit. Their formula on p. 453 of the second paper gives the
index from the two, $n = a - \sqrt{a^2 - g^2 - 1}$ with $a = (1 + R)/(1 - R)$:

| λ, nm | 420 | 450 | 500 | 550 | 600 | 650 |
|---|---|---|---|---|---|---|
| $R$, measured | 0.293 | 0.331 | 0.470 | 0.740 | 0.844 | 0.889 |
| $g = n\kappa$, measured | 1.72 | 1.73 | 2.07 | 2.32 | 2.91 | 3.58 |
| $n$ from them | none | none | 1.22 | 0.50 | 0.41 | 0.41 |
| $n + i\kappa$, Mie | 1.70 + 1.70i | 1.73 + 1.72i | 1.10 + 2.02i | 0.57 + 2.45i | 0.38 + 2.96i | 0.41 + 3.54i |

At 420 and 450 nm the measured pair has no real $n$, the root being imaginary; Hagen and Rubens
thought their violet reflectivities too small (p. 453), and Mie raised them. With the measured $n$
and $\kappa$ from 500 to 650 nm in place of his, the series' median difference from his 70 numbers
grows from 1.4 % to 2.9 %, and his 650 nm spheres of 20, 40 and 60 nm come out at 7.1, 9.1 and
14.3 ‰, further from his than with his own gold. His table follows his smoothed gold, as it should,
and the series reproduces it but for those curves.

::chart mie-gold{diameter=100nm}

### Mie's colours {research}

Mie read the colours of gold solutions off these numbers (his pp. 438 and 439). The finest
particles absorb steeply in the green near 525 nm, and their solutions are ruby red. As the
particles grow, absorption rises across the spectrum and its maximum moves to the red; at about
100 nm the solution is violet, at 120 and 140 nm deep blue, at 160 indigo and at 180 greenish
blue: every colour seen in colloidal gold. In his summary (p. 442) he traced them to two
properties of gold particles, a sharp maximum of absorption in the green and a maximum of
reflection in the red-yellow: the small particles absorb strongly and scatter little and make the
solution ruby red, the large ones scatter strongly and make it blue.

**Try it.** In the chart, find the diameter where the extinction first peaks at 550 nm.

:::answer
60 nm: 96.1 ‰ at 550 nm against 89.8 at 525, Mie's 96.1 and 88.4. At 40 nm it still peaks at
525 nm.
:::

## 7. How does FDTD model a metal?

FDTD steps the fields in time, so a permittivity that depends on the frequency enters as a current
the field drives. For a Drude term, the electrons' equation of motion with $J = -Nev$ gives

$$
\frac{\partial J}{\partial t} + \gamma J = \varepsilon_0\omega_p^2 E ,
$$

and Ampère's law becomes
$\varepsilon_0\varepsilon_\infty\partial E/\partial t + J = \nabla\times H$. A Lorentz term adds the
spring: $\partial^2 J/\partial t^2 + \Gamma\partial J/\partial t + \omega_0^2 J = a\thinspace\partial E/\partial t$,
of which the Drude term is the time derivative with $\omega_0 = 0$. photonoxide steps each term's
current beside the fields, the synchronized scheme of
[Okoniewski, Mrozowski and Stuchly (1997)](https://doi.org/10.1109/75.569723), several terms in
one medium ([FDTD](../docs/methods/fdtd.md#dispersive-media)). Its two-term Lorentz half-space
reflects as the analytic one does, within 0.24 of the paper's own error curve on its grid of
37.5 µm cells:

::example lorentz_okoniewski

A Drude half-space ($f_p$ = 1 c/µm, $\gamma/2\pi$ = 0.05 c/µm) reflects as Fresnel's
$r = (1 - n)/(1 + n)$ to $3.0 \times 10^{-4}$ on 5 nm cells, below and above the plasma frequency,
at second order. A sphere's scattering against Mie's series converges at second order when the
sphere is smoothed, $7.2 \times 10^{-3}$ on 16 cells a radius for $\varepsilon = 4$; a damped Drude
sphere ($f_p$ = 0.5 c/µm, $\gamma/2\pi$ = 0.2 c/µm, radius 1 µm) is sampled, not smoothed, and
converges at first order, 1.6 % on 16 cells a radius:

::validation fdtd/drude-fresnel fdtd/mie-sphere fdtd/mie-sphere-staircase fdtd/mie-drude

A sharp plasmon is not yet converged. With $\gamma/2\pi$ = 0.02 c/µm the same sphere's dipole and
quadrupole plasmons fall at $\alpha \approx 1.5$ to 2, and on 16 cells a radius the staircased
sphere absorbs up to 2.7 times Mie's there, falling slowly with the grid
([FDTD](../docs/methods/fdtd.md#scattering-by-a-sphere)). A plasmon is a surface mode, and its field
is largest at the surface the staircase misplaces; a dispersive medium's permittivity is sampled
at each value of E, not averaged over the cell as a dielectric's is. Mie's series and the
transfer matrix of this lesson's charts have no grid.

## 8. History and current research {research}

[Drude (1900)](https://doi.org/10.1002/andp.19003060312) described a metal's conduction by free
electrons, and its optics by those and bound ones together.
[Mie (1908)](https://doi.org/10.1002/andp.19083300302) solved a plane wave on a sphere exactly and
computed from it, with gold's index from Hagen and Rubens's measurements, the absorption and
scattering of colloidal gold at seven wavelengths and sizes up to 180 nm: the localized plasmon of
Section 5. [Ritchie (1957)](https://doi.org/10.1103/PhysRev.106.874)
predicted losses of fast electrons in thin metal foils below the plasma energy, at
$\omega_p/\sqrt 2$, from the foil's surfaces: the surface plasmon of Section 2.
[Okoniewski, Mrozowski and Stuchly (1997)](https://doi.org/10.1109/75.569723) put several Debye or
Lorentz terms into FDTD by auxiliary differential equations, as cheap in memory and operations as
the recursive convolutions before them and without assuming a linear medium (their abstract): the
scheme photonoxide's dispersive media use.

::timeline

photonoxide computes the physics of this lesson exactly where it has no grid: the Drude metal
(`material::Model::Drude`), the surface plasmon as a mode of a planar stack, and Mie's series. It
has no metal in its material catalogue yet, beyond the gold of Mie's table, and no plasmonic device
in its component library. Its FDTD reproduces a Drude half-space and a damped Drude sphere, and
does not yet converge on a sphere's sharp plasmon (Section 7).

Further reading: [the material lesson](material-dispersion.md) for the bound and free oscillators
behind every permittivity; [the multilayer](../docs/methods/multilayer.md) for the transfer matrix
that solves the surface plasmon; [FDTD](../docs/methods/fdtd.md) for Mie's series in photonoxide's
convention, the dispersive media and their checks; and [the slab waveguide](slab-waveguide.md) for
the dielectric guide a surface plasmon is set against.

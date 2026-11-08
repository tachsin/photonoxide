---
title: "How FDTD works"
summary: "Maxwell's equations stepped in time on Yee's staggered grid: the leapfrog and its stability, numerical dispersion, absorbing boundaries and how thick they must be, plane waves by total-field/scattered-field, materials smoothed between cells, dispersive media, and what a run gives and when it is done."
topic: Numerical methods
level: intermediate
minutes: 50
prerequisites: [slab-waveguide]
examples: [cpml_roden_gedney, pml_oskooi, tfsf_square_cylinder, subpixel_holes, bump_oskooi, lorentz_okoniewski]
jobs: [mmi-fdtd.yaml, ring-fdtd.yaml]
methods: [fdtd.md, fdfd.md]
validation:
  - fdtd/dispersion
  - fdtd/energy
  - fdtd/cpml-thickness
  - fdtd/meep-pml-rates
  - fdtd/tfsf-leakage
  - fdtd/tfsf-slab
  - fdtd/smoothing-slab
  - fdtd/smoothing-oskooi
  - fdtd/drude-fresnel
  - fdtd/lorentz-group-delay
  - fdtd/fdfd-lossy
  - fdtd/monitor-flux
  - fdtd/harmonic-inversion
  - fdtd/ring-resonances
charts: [fdtd-dispersion, fdtd-stability, fdtd-pml]
papers:
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    title: "Numerical solution of initial boundary value problems involving Maxwell's equations in isotropic media"
    doi: 10.1109/TAP.1966.1138693
    year: 1966
    role: origin
    note: "Maxwell's equations as finite differences on a grid whose electric and magnetic field points are staggered, chosen so that a perfectly conducting surface's boundary condition applies, and a pulse scattered by a conducting cylinder."
  - cite: "A. Taflove, M. E. Brodwin, IEEE Trans. Microw. Theory Tech. 23, 623 (1975)"
    title: "Numerical solution of steady-state electromagnetic scattering problems using the time-dependent Maxwell's equations"
    doi: 10.1109/TMTT.1975.1128640
    year: 1975
    role: milestone
    note: "Yee's scheme run to the sinusoidal steady state for dielectric scatterers, with the stability condition on the time step derived in their appendix; Yee's own criterion, they found, was incorrect."
  - cite: "K. Umashankar, A. Taflove, IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982)"
    title: "A novel method to analyze electromagnetic scattering of complex objects"
    doi: 10.1109/TEMC.1982.304054
    year: 1982
    role: milestone
    note: "A plane wave on an object computed inside a box that holds the total field, with the scattered field outside it carried to the far field; the surface currents of a square conducting cylinder against the moment method."
  - cite: "J.-P. Berenger, J. Comput. Phys. 114, 185 (1994)"
    title: "A perfectly matched layer for the absorption of electromagnetic waves"
    doi: 10.1006/jcph.1994.1159
    year: 1994
    role: milestone
    note: "An absorbing layer designed to absorb waves without reflection at any angle and frequency, better in every comparison than the absorbing boundaries used before it."
  - cite: "V. A. Mandelshtam, H. S. Taylor, J. Chem. Phys. 107, 6756 (1997)"
    title: "Harmonic inversion of time signals and its applications"
    doi: 10.1063/1.475324
    year: 1997
    role: milestone
    note: "A short signal's frequencies and decay rates by filter diagonalization, a small generalized eigenproblem in a window of frequencies, resolving what a Fourier transform of the same length cannot."
  - cite: "M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave Lett. 7, 121 (1997)"
    title: "Simple treatment of multi-term dispersion in FDTD"
    doi: 10.1109/75.569723
    year: 1997
    role: milestone
    note: "Several Debye or Lorentz terms in FDTD by auxiliary differential equations, as cheap as the recursive convolutions and without assuming a linear medium."
  - cite: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000)"
    title: "Convolution PML (CPML): an efficient FDTD implementation of the CFS-PML for arbitrary media"
    doi: "10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A"
    year: 2000
    role: milestone
    note: "The complex-frequency-shifted PML applied by a recursive convolution, independent of the medium, absorbing evanescent waves, with a quarter of the memory of the traditional formulation; a plate in lossy soil as the test."
  - cite: "A. Farjadpour, D. Roundy, A. Rodriguez, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, G. W. Burr, Opt. Lett. 31, 2972 (2006)"
    title: "Improving accuracy by subpixel smoothing in the finite-difference time domain"
    doi: 10.1364/OL.31.002972
    year: 2006
    role: milestone
    note: "A subpixel smoothing of the permittivity from perturbation theory, the only one of those compared that converges quadratically at interfaces of any slope."
  - cite: "A. F. Oskooi, L. Zhang, Y. Avniel, S. G. Johnson, Opt. Express 16, 11376 (2008)"
    title: "The failure of perfectly matched layers, and towards their redemption by adiabatic absorbers"
    doi: 10.1364/OE.16.011376
    year: 2008
    role: milestone
    note: "A PML's reflection on a grid comes from how its absorption turns on: for σ rising as u^d it falls as 1/L^(2d+2) with the layer's thickness, by the adiabatic theorem; and in a photonic crystal a PML is no better than any gradual absorber."
  - cite: "A. F. Oskooi, D. Roundy, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, Comput. Phys. Commun. 181, 687 (2010)"
    title: "Meep: a flexible free-software package for electromagnetic simulations by the FDTD method"
    doi: 10.1016/j.cpc.2009.11.008
    year: 2010
    role: review
    note: "The design of a free FDTD code and the numbers behind it: subpixel smoothing's convergence on a bump, PML convergence against its grading, and resonances by harmonic inversion."
---

One cell of Yee's grid, where every field component lives, and the leapfrog that steps E and H
in turn. The symbols are those of the equations below.

::diagram yee

FDTD solves Maxwell's equations as they unfold in time. Space is cut into cells, time into steps,
and each field component is updated from its neighbours' values a step earlier: no matrix is
solved, and a pulse run once gives a structure's response over a whole band
([the FDTD write-up](../docs/methods/fdtd.md)). What the grid gets wrong is as definite as what it
gets right: light travels slightly slower on it, the time step has a ceiling, boundaries must
swallow outgoing waves, and an interface between cells must be averaged. Each section below takes
one of these, with what photonoxide's FDTD reproduces.

## 1. How does Yee's grid step Maxwell's equations in time?

With $c = 1$, lengths in µm and $\tilde H = \eta_0 H$, Maxwell's curl equations are
$\partial\tilde H/\partial t = -\nabla\times E$ and $\varepsilon\thinspace\partial E/\partial t = \nabla\times\tilde H$.
Yee placed E's components at the middles of a cell's edges and $\tilde H$'s at the centres of its
faces ([Yee, 1966](https://doi.org/10.1109/TAP.1966.1138693)): each E is circled by four
$\tilde H$ and each $\tilde H$ by four E, so every curl is a difference across half a cell,
centred on the component it updates. In time the same: E at whole steps, $\tilde H$ half a step
after, each advanced from the other's curl, the leapfrog of the diagram. Every derivative is a
centred difference, so the scheme is second order in space and in time.

Two exact properties follow and photonoxide checks both. A plane wave on a periodic grid
oscillates at exactly the frequency the discrete dispersion relation of Section 3 gives, to
round-off; and in a closed, lossless box the leapfrog conserves a discrete energy,
$\tfrac12\sum\varepsilon E^2 + \tfrac12\sum\tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$, to $2.4 \times 10^{-15}$
over 300 steps, because Yee's two curls are each other's transposes:

::validation fdtd/dispersion fdtd/energy

### The update, and why it is second order {theory}

On a line, with $E_z$ at the nodes $i\Delta$ and $\tilde H_y$ between them at $(i + \tfrac12)\Delta$,
the two equations become

$$
\tilde H_{y,i+1/2}^{n+1/2} = \tilde H_{y,i+1/2}^{n-1/2} + \frac{\Delta t}{\Delta}\left(E_{z,i+1}^n - E_{z,i}^n\right),
\qquad
E_{z,i}^{n+1} = E_{z,i}^n + \frac{\Delta t}{\varepsilon_i\Delta}\left(\tilde H_{y,i+1/2}^{n+1/2} - \tilde H_{y,i-1/2}^{n+1/2}\right) .
$$

Each difference is centred: by Taylor's series,
$[f(x + \Delta/2) - f(x - \Delta/2)]/\Delta = f'(x) + \Delta^2 f'''(x)/24 + O(\Delta^4)$, the odd
terms cancelling. A one-sided difference would leave an error of order $\Delta$; the staggering is
what makes every difference centred at no extra cost. A field value needs only its own last value
and its neighbours', so a step costs a fixed number of operations per cell.

### Yee's paper {research}

[Yee (1966)](https://doi.org/10.1109/TAP.1966.1138693) replaced Maxwell's equations by finite
differences and chose the field points so that the boundary condition on a perfectly conducting
surface applies on the grid. His example was a pulse scattered by a perfectly conducting
cylinder.

## 2. How large can the time step be?

The leapfrog is stable only while

$$
c\Delta t \le \frac{1}{\sqrt{\sum_i 1/\Delta_i^2}} ,
$$

the sum over the axes the field varies along: $\Delta$ on a line, $\Delta/\sqrt 2$ on a plane and
$\Delta/\sqrt 3$ in a volume of cubic cells (derived below). photonoxide takes the step as a
Courant number $C$ times this limit and refuses $C > 1$ ([FDTD](../docs/methods/fdtd.md#the-scheme)).
Beyond the limit, the grid's fastest wave, two cells a wavelength along every axis, grows at every
step. The chart draws that growth against $c\Delta t/\Delta$: at 0.6 a line and a plane are stable,
and a volume, past its limit of 0.5774, multiplies the wave by 1.748 a step, a millionfold in 25
steps (exact, no grid).

::chart fdtd-stability

### The limit from a plane wave {theory}

Put $E, \tilde H \propto e^{i(k\cdot x - \omega t)}$ into the update. A centred difference turns
$e^{ikx}$ into $2i\sin(k\Delta/2)/\Delta$ times itself, and a centred step in time turns
$e^{-i\omega t}$ into $-2i\sin(\omega\Delta t/2)/\Delta t$ times itself, so the curl equations
give, in a medium of index $n$,

$$
n^2\frac{\sin^2(\omega\Delta t/2)}{\Delta t^2} = \sum_i \frac{\sin^2(k_i\Delta_i/2)}{\Delta_i^2}
$$

([Taflove and Brodwin, 1975](https://doi.org/10.1109/TMTT.1975.1128640), their appendix). The
right side is largest, $\sum_i 1/\Delta_i^2$, for $k_i\Delta_i = \pi$ on every axis. A real
$\omega$ needs the left side's sine at most 1, so $\Delta t^2\sum_i 1/\Delta_i^2 \le n^2$: the
limit above, in vacuum. Past it, with $s = (\Delta t/n)\sqrt{\sum_i\sin^2(k_i\Delta_i/2)/\Delta_i^2} \gt 1$,
the root is $\omega\Delta t = \pi + 2i\operatorname{acosh}s$, and the wave grows by
$(s + \sqrt{s^2 - 1})^2$ every step. photonoxide gives the relation as `fdtd::grid_frequency`
and the step as `fdtd::time_step`; the chart's figures are these.

**Try it.** Set the step to 0.71. What happens on a plane and in a volume?

:::answer
A plane, just past its limit of 0.7071, grows 1.198 a step, a millionfold in 76 steps; a volume
3.785 a step, in 10. A line is stable up to 1: the magic step of Section 3.
:::

### Yee's criterion {research}

Yee gave a stability criterion in his paper; [Taflove and Brodwin
(1975)](https://doi.org/10.1109/TMTT.1975.1128640), running his scheme to the steady state for
dielectric scatterers, derived the condition above in their appendix and state that Yee's is
incorrect.

## 3. How fast do waves travel on the grid?

Slower than light, and how much slower depends on the direction. The relation of Section 2 gives
the grid's wavenumber $\tilde k$ for a frequency; it is larger than $n\omega/c$, so the phase
velocity falls short. The chart draws the shortfall $1 - v/c$ against the cells per wavelength on
a plane at $C = 0.5$ (exact, no grid): along an axis 1.48 % at 10 cells and 0.363 % at 20, along
the diagonal 0.627 % at 10. It falls as $1/N^2$: from 20 cells to 40 by $2^{2.01}$. Small as it
is, it accumulates: at 20 cells a wavelength, along an axis, a wave 100 wavelengths on lags
$2\pi \times 100 \times 0.00363 = 2.28$ rad, 131°, behind the true one.

::chart fdtd-dispersion

### The error to leading order {theory}

On a line with $\Delta t = C\Delta/c$, the relation is $\sin(\omega\Delta t/2)/C = \sin(\tilde k\Delta/2)$.
Expanding both sines to third order, $\omega\Delta t/2 - (\omega\Delta t/2)^3/6 = C\tilde k\Delta/2 - C(\tilde k\Delta/2)^3/6$,
and with $\omega\Delta t = Ck\Delta$, $k = \omega/c$,

$$
\frac{\tilde k}{k} - 1 \approx \frac{(k\Delta)^2}{24}\left(1 - C^2\right) ,
$$

second order in $k\Delta$, and zero at $C = 1$: on a line at the limit the grid is exact, the
"magic" step, and the chart shows it so. On a plane or in a volume no step makes every direction
exact; at $C = 1$ the diagonal is, and the axes are not. photonoxide's `fdtd::grid_wavenumber`
solves the relation for $\tilde k$ along any direction, the chart's curves; the chart's tests check
it against this leading term to 5 % from 10 to 40 cells, and `fdtd::grid_frequency` against a
run on a periodic grid to round-off.

**Try it.** Raise the Courant number to 0.99. What happens along the diagonal?

:::answer
At 10 cells it falls from 0.627 % to 0.0167 %: on a plane the diagonal is exact at $C = 1$. Along
an axis it is still 0.868 %; the grid stays anisotropic.
:::

## 4. How does a convolutional PML absorb outgoing waves?

A run must end at the edge of its grid while the waves that reach the edge go on as if the space
continued. A perfectly matched layer (PML) is a layer of cells that absorbs them: the coordinate
across it is stretched into the complex plane, $\partial/\partial x \to (1/s)\thinspace\partial/\partial x$,
so a wave entering it decays, and since the stretch changes no impedance, at its face nothing is
reflected, in the continuous equations, at any angle and frequency
([Berenger, 1994](https://doi.org/10.1006/jcph.1994.1159)). photonoxide's FDTD uses Roden and
Gedney's convolutional PML (CPML), whose stretch is complex-frequency-shifted
([FDTD](../docs/methods/fdtd.md#boundaries)):

$$
s = \kappa + \frac{\sigma}{\alpha - i\omega} .
$$

σ absorbs propagating waves; κ > 1 stretches the coordinate for real, and with α > 0 the layer
also absorbs evanescent waves, which the traditional PML (κ = 1, α = 0) reflects late in a run.
Roden and Gedney's test is a thin conducting plate
in lossy soil, its CPML 3 cells away: photonoxide finds the largest error −48.6 dB with α = 0
(their −48) and −70.5 dB with α = 0.05 S/m (their −67), on 1 mm cells over 2000 steps:

::example cpml_roden_gedney

### The recursive convolution {theory}

In the time domain $1/s$ is a filter, $1/\kappa$ plus a decaying exponential, since $1/s(\omega)$ is
a rational function. Each derivative across the layer becomes $\partial F/\kappa + \psi$, with an
auxiliary $\psi$ updated each step by $\psi \leftarrow b\psi + a\thinspace\partial F$,
$b = e^{-(\sigma/\kappa + \alpha)\Delta t}$ and $a = \sigma(b - 1)/(\sigma\kappa + \kappa^2\alpha)$:
the convolution done recursively, one multiply-add a step. The ψ live only in the layer, two per
field component per axis, and the update elsewhere is untouched, whatever the medium. With κ = 1
and α = 0 the stretch is FDFD's own PML, $s = 1 + i\sigma/\omega$; the recursive convolution is it
to first order in Δt, so FDTD's field approaches FDFD's as the step falls.

### Berenger's layer and its successor {research}

[Berenger (1994)](https://doi.org/10.1006/jcph.1994.1159) split one field component inside the
layer into two subcomponents, each with its own conductivity along one axis, so that the layer
absorbs without reflection; in his comparisons it worked better than every free-space boundary
used before. The complex
frequency shift and its recursive-convolution form came later; Roden and Gedney's version (2000)
is independent of the medium the layer ends, absorbs evanescent waves, and needed a quarter of
the memory of the traditional formulation in their test.

## 5. How thick must a PML be?

On a grid a PML reflects: the continuous layer's reflection is zero, but the grid samples its
absorption cell by cell, and an abrupt turn-on scatters. σ is therefore turned on gradually, as
$(x/d)^m$ across a layer $d$ thick, its maximum $(m + 1)(-\ln R)/2d$ set for a round-trip
reflection $R$ ([FDTD](../docs/methods/fdtd.md#boundaries)). The chart runs photonoxide's FDTD on
a line: a pulse at 1 c/µm on 20 cells a wavelength meets a CPML $L$ cells thick head on, its
reflection read against a run with a 160-cell layer, for $m$ = 1, 2 and 3 at $R = 10^{-15}$. From
16 to 32 cells the reflected power falls as $1/L^{4.13}$, $1/L^{6.01}$ and $1/L^{8.00}$, against
$1/L^{2m + 2}$, the rate [Oskooi et al. (2008)](https://doi.org/10.1364/OE.16.011376) derive. Half a
wavelength of cubic grading, 10 cells, reflects $2.7 \times 10^{-9}$ of the power; linear grading
$5.5 \times 10^{-4}$ (1D FDTD, 20 cells a wavelength, C = 0.5).

::chart fdtd-pml

### The adiabatic rate {theory}

Oskooi et al. treat the reflection from a gradual absorber by coupled-mode theory and the
adiabatic theorem (their Sec. 4): a profile $s(u) = u^d$, $u = x/L$, has its first discontinuity
in its $d$-th derivative, at $u = 0$, and the smoother the profile the faster the reflection falls
as it is stretched over a thicker layer, the reflected power as $1/L^{2d+2}$; the field a PML of $L$ disturbs converges one order faster in $L$,
as $1/L^{2d+4}$, the "field convergence" they and Meep's paper measure. The chart's curves bend
over at small $L$, where the grading is sampled by a few cells, and the round-trip $R$ sets a floor
far below them.

[Oskooi et al. (2010)](https://doi.org/10.1016/j.cpc.2009.11.008) measure this in 2D, a point
source in a cell with PMLs on every side: photonoxide reproduces their rates of field convergence,
6.03, 8.05 and 10.08 at $L$ = 8 against 6, 8 and 10, on 20 cells a wavelength:

::example pml_oskooi

::validation fdtd/cpml-thickness fdtd/meep-pml-rates

### When a PML fails {research}

[Oskooi et al. (2008)](https://doi.org/10.1364/OE.16.011376) show that a PML isn't reflectionless
even at infinite resolution where the medium varies along the direction it absorbs in, as a
photonic crystal does: the coordinate stretching relies on the medium continuing analytically into
the layer. There a PML does no better than any gradual absorber, and only its thickness helps, at
the same $1/L^{2d+2}$ rate; in a uniform medium a PML's advantage is a smaller constant factor, not
a faster rate.

## 6. How is a plane wave launched without touching the scatterer?

A plane wave must enter the grid from everywhere at once along its front, and the light a
scatterer sends back must leave through the PML without meeting the source. Total-field /
scattered-field (TF/SF) does both: inside a box around the scatterer the grid holds the total
field, outside it only the scattered field. Where an update reaches across the box's faces, the
known incident field is added or subtracted, so that each side sees its own field
([Umashankar and Taflove, 1982](https://doi.org/10.1109/TEMC.1982.304054)). With nothing in the
box, nothing leaves it: the scattered region stays empty to round-off when the incident field
solves the grid's own equations, which photonoxide ensures by computing it from a 1D run along the
wave's direction, with the grid's own dispersion, for any direction whose components are whole
numbers of cells. On a slab of $\varepsilon = 4$ the reflection from the scattered field matches
Airy's formula to $3.3 \times 10^{-4}$ on 5 nm cells, falling at second order:

::validation fdtd/tfsf-leakage fdtd/tfsf-slab

Umashankar and Taflove's own case is a square conducting cylinder lit broadside, its surface
current against the moment method. On 20 cells a side photonoxide gives 1.732 at the middle of
the lit face and 0.764 on a side face, against the 1.75 and 0.785 their figure shows; on 40 cells
1.735, 0.767 and 0.170 on the shadowed face, against 1.737, 0.769 and 0.171 from the moment method
of their Eq. 8a, which the example solves on 100 points a side. Their figure's curve on the
shadowed face, 0.207, lies 21 % above that moment method's value; photonoxide's FDTD agrees with
the moment method and does not reproduce the curve:

::example tfsf_square_cylinder

### The field equivalence behind it {theory}

Write the total field as incident plus scattered, $F = F_\text{inc} + F_\text{sca}$. Both satisfy
the update in free space, so the update can be applied to either, provided each difference uses
values of one kind. At a cell just inside the box whose update reads an E just outside it, the
stored E there is scattered, and the total the update needs is that plus $E_\text{inc}$: one term
added. The mirror correction applies just outside. Nothing else changes, so the box's faces
emit the incident wave inward and absorb it outward exactly, to the extent the stored incident
field obeys the same discrete equations; a continuous plane wave would leak at the grid's
dispersion error of Section 3.

## 7. Why smooth the permittivity between cells?

A grid that gives each cell the permittivity at its centre moves every interface to the nearest
cell boundary. That staircase is a first-order error, and an erratic one: as the grid refines, the
interfaces jump between cells and the error with them. Averaging the permittivity over each cell
helps only if the average is the right one: across an interface the normal field
$E_\perp$ jumps while $D_\perp = \varepsilon E_\perp$ is continuous, so the normal component sees
the harmonic mean, $\langle\varepsilon^{-1}\rangle^{-1}$, and the tangential components, continuous
themselves, the arithmetic mean $\langle\varepsilon\rangle$. Built as a tensor oriented with the
interface, this average restores second-order convergence at interfaces of any slope
([Farjadpour et al., 2006](https://doi.org/10.1364/OL.31.002972)). On a slab between grid points
photonoxide's smoothed reflection matches Airy's to $6.9 \times 10^{-4}$ on cells of 1/160 µm:

::validation fdtd/smoothing-slab fdtd/smoothing-oskooi

Farjadpour et al.'s lattice of tilted elliptical air holes in $\varepsilon = 12$: the lowest
bands' errors against pixels a period fall with slopes of −2.26 and −2.11 with their smoothing,
against their −2.43 and quadratic, from 16 to 64 pixels. With the plain mean ε, band 2's slope is
−1.40 against their −1.33; band 1's is −1.84 against their −1.33, which photonoxide prints and
does not reproduce. Over their own resolutions it stays steep: −1.81 from 20 to 64 pixels. A fit
of $A/N + B/N^2$ to ours gives −1.91 from 20 to 100. The plain mean's first-order error depends on
the mode and on $k$, which the paper doesn't give: at the M point, oblique to the grid, the two
lowest bands fall at −2.02 and −1.62. None of these is their −1.33, and their mean's error at 20
pixels, 2.83 × 10⁻³, is three times ours:

::example subpixel_holes

Oskooi et al. (2010)'s bump on a waveguide: least-squares orders of 2.37 smoothed and 1.07 not,
against their 2 and 1, from 10 to 30 pixels:

::example bump_oskooi

### The perturbation criterion {theory}

Farjadpour et al. choose the average from first-order perturbation theory: the frequency of a
mode shifts by an integral of $\Delta\varepsilon\thinspace\lvert E\rvert^2$, which for a
displaced interface must be written in $E_\parallel$ and $D_\perp$, the components continuous
across it. Requiring the smoothed medium to give the same shift as the true interface, to first
order, gives $\langle\varepsilon\rangle$ for $E_\parallel$ and $\langle\varepsilon^{-1}\rangle^{-1}$
for $D_\perp$, the tensor above. A scalar average, any one, gets one of the two components wrong
and converges at first order.

## 8. How does FDTD handle a dispersive material?

A permittivity that depends on the frequency can't be a number in a time-domain update. Each
Drude or Lorentz term becomes a polarization current the field drives, stepped beside the fields
by its own differential equation:
$\partial^2 J/\partial t^2 + \Gamma\thinspace\partial J/\partial t + \omega_0^2 J = a\thinspace\partial E/\partial t$,
the Drude term its time derivative with $\omega_0 = 0$, the synchronized scheme of
[Okoniewski, Mrozowski and Stuchly (1997)](https://doi.org/10.1109/75.569723)
([FDTD](../docs/methods/fdtd.md#dispersive-media); the
[plasmonics lesson](plasmonics.md) derives the Drude term). photonoxide refuses a medium that
isn't passive or whose update would let a plane wave grow; that works out as $C \le \sqrt{\varepsilon_\infty}$
and $\omega_0\Delta t \le 2$ for each Lorentz term, while Drude terms are stable at any
$\omega_p\Delta t$. Okoniewski et al.'s two-term Lorentz half-space reflects as the analytic one
does, within 0.24 of the paper's own error curve at 37.5 µm cells and $C = 1$:

::example lorentz_okoniewski

A Drude half-space reflects as Fresnel's formula to $3.0 \times 10^{-4}$ on 5 nm cells, and a
pulse crosses a Lorentz slab at its exact group delay:

::validation fdtd/drude-fresnel fdtd/lorentz-group-delay

## 9. What does an FDTD run give, and when is it done?

One pulse carries a band of frequencies, and a run gives the response at all of them. Monitors
accumulate discrete Fourier transforms of the fields as they pass, at chosen frequencies; once the
fields have died away, those transforms solve FDFD's equations at the leapfrog's frequency
$\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$ exactly, so fluxes and mode amplitudes computed
from them are FDFD's to round-off ([FDFD](../docs/methods/fdfd.md)):

::validation fdtd/fdfd-lossy fdtd/monitor-flux

A run stops when the fields at its probes have stayed below a fraction of their peak for a whole
block of steps, or at a time limit, and says which; a field that grows again instead stops it with
an error ([FDTD](../docs/methods/fdtd.md#probes-and-monitors)). The built-in
`jobs/mmi-fdtd.yaml` runs the 1 × 2 MMI of the [MMI lesson](mmi.md) on its 20 nm grid until its
fields decay to $10^{-8}$; each output's power is within $4.3 \times 10^{-4}$ of FDFD's at 1.55 µm.

A resonator rings long after the pulse. Its frequencies and decay rates come from a short record
of its field by harmonic inversion
([Mandelshtam and Taylor, 1997](https://doi.org/10.1063/1.475324)), which fits a sum of decaying
exponentials in a window of frequencies by a small generalized eigenproblem, and resolves two
peaks closer than a Fourier transform of the same record can. On a ring of $\varepsilon = 11.56$
(Oskooi et al. 2010's Fig. 11, 2D, 20 cells a µm), photonoxide's three resonances are within
0.077 % of the exact ones in frequency; `jobs/ring-fdtd.yaml` runs a ring beside its bus:

::validation fdtd/harmonic-inversion fdtd/ring-resonances

## 10. History and current research {research}

[Yee (1966)](https://doi.org/10.1109/TAP.1966.1138693) set out the staggered grid and the leapfrog.
[Taflove and Brodwin (1975)](https://doi.org/10.1109/TMTT.1975.1128640) gave its stability condition
and ran it to the steady state; [Umashankar and Taflove (1982)](https://doi.org/10.1109/TEMC.1982.304054)
launched plane waves by total-field/scattered-field and carried the scattered field to the far
zone. [Berenger (1994)](https://doi.org/10.1006/jcph.1994.1159) made the edge of the grid
reflectionless in principle, and Roden and Gedney (2000) gave the convolutional form photonoxide
uses. Dispersive media entered by auxiliary differential equations
([Okoniewski et al., 1997](https://doi.org/10.1109/75.569723)); resonances came from short records
by harmonic inversion, a method from chemical physics
([Mandelshtam and Taylor, 1997](https://doi.org/10.1063/1.475324)). For optics,
[Farjadpour et al. (2006)](https://doi.org/10.1364/OL.31.002972) made interfaces between cells
second order, and [Oskooi et al. (2008)](https://doi.org/10.1364/OE.16.011376) explained how
thick an absorber must be and where a PML fails; Meep's paper
([Oskooi et al., 2010](https://doi.org/10.1016/j.cpc.2009.11.008)) measured both in a free code.

::timeline

photonoxide's FDTD runs on the CPU and, behind the `gpu` feature, on the GPU, where it agrees with
the CPU to a tolerance stated per quantity (docs/validation-gpu.md). Its open problems are written
down beside the method: a sharp plasmon on a metal sphere does not yet converge, because a
dispersive medium is sampled at each value of E rather than smoothed
([FDTD](../docs/methods/fdtd.md#scattering-by-a-sphere), the [plasmonics lesson](plasmonics.md)).

Further reading: [the FDTD write-up](../docs/methods/fdtd.md) for every update, its derivation and
its checks; [FDFD](../docs/methods/fdfd.md) for the frequency-domain solver FDTD is checked
against; and [FDFD and adjoint gradients](fdfd-and-adjoints.md) for the other way to solve the
same equations.

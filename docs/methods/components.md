---
title: "First components"
module: circuit::components
summary: "Waveguides, bends, couplers, MMIs, Y-branches, rings and the Mach-Zehnder interferometer: closed forms with their exact parameter derivatives, models built from the mode solvers, and S-matrices sampled by 2D FDFD."
order: 22
papers:
  - cite: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue)"
    doi: 10.1002/lpor.201100017
  - cite: "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995)"
    doi: 10.1109/50.372474
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    doi: 10.1017/CBO9781316084168
  - cite: "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971)"
    doi: 10.1002/j.1538-7305.1971.tb02620.x
  - cite: "M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975)"
    doi: 10.1109/JQE.1975.1068563
  - cite: "S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015)"
    doi: 10.1109/JLT.2015.2476603
validation:
  - components/ring-all-pass-bogaerts
  - components/ring-add-drop-bogaerts
  - components/ring-extremes-bogaerts
  - components/ring-linewidth-bogaerts
  - components/ring-fsr-fdfd
  - components/mzi-closed-form
  - components/mzi-unitarity
  - components/unitarity
  - components/reciprocity
  - components/passivity
  - components/derivatives
  - components/directional-coupler-power
  - components/mmi-beat-length-soldano
  - components/mmi-fdfd
  - components/mzi-neff-dwivedi-470
  - components/mzi-ng-dwivedi-470
  - components/mzi-neff-dwivedi-602
  - components/mzi-ng-dwivedi-602
  - components/mzi-neff-dwivedi-805
  - components/mzi-ng-dwivedi-805
---

The components a chip is made of, each a `Component` (`photonoxide::circuit::components`): a
model shared by every instance of it in a netlist, its parameters' values passed in, its
S-matrix in the [circuits' conventions](circuits.md) ($e^{-i\omega t}$, power-normalized
amplitudes, $S_{qp}$ from port $p$ into port $q$, each port's phase at its reference plane).
Each states where its S-matrix comes from: its fidelity (analytic, compact, 2D, 3D, measured),
its source with the paper or the solver and grid, its error against that source where it was
measured, and the wavelengths it holds for. The [design note](../design/components.md) explains
the model.

| Component | Ports | Parameters | Fidelity |
|---|---|---|---|
| `Waveguide` | `o1`, `o2` | `length` (µm), `loss` (dB/cm) | analytic; 3D from the full-vector solver |
| `Bend` | `o1`, `o2` | `angle` (°) | analytic; 3D (conformal map) or 2D (exact bent slab) |
| `PhaseShifter` | `o1`, `o2` | `phase` (rad) | analytic |
| `Coupler` | `o1` to `o4` | `coupling` ($\kappa^2$) | analytic |
| `DirectionalCoupler` | `o1` to `o4` | `length` (µm) | analytic; 3D from the solver's supermodes |
| `Mmi` (1 × 2, 2 × 2) | `o1` to `o3` or `o4` | `length`, `width` (µm) | 2D (exact slab modes) |
| `YBranch` | `o1` (stem), `o2`, `o3` | `excess_loss` (dB) | analytic |
| `AllPassRing` | `in`, `through` | `length` (µm), `coupling` | analytic |
| `AddDropRing` | `in`, `through`, `add`, `drop` | `length`, `coupling`, `coupling_drop` | analytic |
| `mzi`, `mzi_y` | a netlist's | its instances' | its instances' |
| `Sampled` | the samples' | none | 2D (FDFD), or the samples' |

A four-port coupler's ports run `o1` lower left, `o2` upper left, `o3` upper right, `o4` lower
right: the lower guide is `o1` to `o4`, the upper `o2` to `o3`.

## Derivatives

The circuit adjoint ([circuit-adjoint.md](circuit-adjoint.md)) needs each component's
$\partial S / \partial \theta_k$. The closed forms give them exactly: each S-matrix is written
once over dual numbers $v + d\varepsilon$, $\varepsilon^2 = 0$, whose arithmetic carries the
chain rule, so evaluating it with parameter $k$ seeded ($d = 1$) gives $S$ and
$\partial S / \partial \theta_k$ together. The couplings' square roots have no derivative at
$\kappa^2 = 0$ or 1; there the components return none and the circuit takes differences. The MMI
and the sampled spectra return none either (the MMI's width moves every mode). The
`components/derivatives` case checks the dual numbers against central differences.

## Waveguide, bend and phase shifter

A mode's `Dispersion` is its effective index $n_0$, group index $n_g$ and dispersion parameter
$D$ at $\lambda_0$, and its loss. The effective index is second order in $\lambda$:

$$
n(\lambda) = n_0 + n'(\lambda - \lambda_0) + \tfrac{1}{2} n''(\lambda - \lambda_0)^2,
\qquad n' = \frac{n_0 - n_g}{\lambda_0}, \qquad n'' = -\frac{c D}{\lambda_0},
$$

from $n_g = n - \lambda\thinspace dn/d\lambda$ and $D = -(\lambda/c)\thinspace d^2 n/d\lambda^2$
(Chrostowski & Hochberg, Eqs. 3.5 and 3.6). A waveguide of length $L$ transmits
$S_{21} = S_{12} = e^{i\gamma L}$, $\gamma = 2\pi n(\lambda)/\lambda + i\alpha/2$, $\alpha$ the
power loss per unit length (the loss in dB/cm times $10^{-4}\ln 10/10$ per µm), and reflects
nothing. Lossless, it is unitary.

`Dispersion::from_modes` takes a mode of the [full-vector solver](vector.md) at
$\lambda_0 - 2h, \dots, \lambda_0 + 2h$, tracked across them ([dispersion.md](dispersion.md)):
$n_0$, $n_g$ and $D$ from the three in the middle by central differences, the loss from
$\operatorname{Im} n_0$, and the model's error from the outer two. A `Waveguide::from_modes`
built so is exact for a waveguide uniform along its length, to its grid's error, so its fidelity
is 3D; it holds over $\lambda_0 \pm 2h$.
`Dispersion::from_hadley` does the same with each mode refined by
[Hadley's high-accuracy equations](hadley.md), whose error at a waveguide's corners falls far
faster with the grid (a uniform grid of lossless isotropic media, so the model is lossless): a
470 × 211 nm wire's $n_\text{eff}$ and $n_g$ move by 4e-5 and 6e-5 from a 10 nm grid to 5 nm,
where the standard scheme's $n_g$ still moves by 9e-3.

A bend of radius $R$ is the same along its arc, $S_{21} = e^{i\gamma R\theta}$, its mode's
effective index the propagation constant along the arc at $R$ over $k_0$ and its loss the
radiation loss. `Bend::from_modes` takes it from the full-vector solver on a bent cross-section
(Heiblum & Harris's conformal map, [bends.md](bends.md)), exact for $E$ normal to the bend
plane, with an error of order $1/R^2$ for $E$ in it; `Bend::from_bent_slab` from the exact bent
slab (radial shooting to Marcuse's outgoing Hankel function, his Eq. 10), the guide seen from
above, a 2D model. The transitions to straight guides (their mode mismatch) are not included.

An ideal phase shifter transmits $e^{i\phi}$, the same at every wavelength: a heater or a
modulator before 0.6 models them.

## Couplers

The ideal `Coupler` sends $\kappa^2$ of the power across: through $\sqrt{1 - \kappa^2}$, across
$i\kappa$, the same both ways, times $10^{-A/20}$ for an excess loss $A$ in dB. The $i$ makes
it unitary, and it is the convention of Bogaerts et al.'s ring responses.

The `DirectionalCoupler` is coupled-mode theory from its two supermodes (Chrostowski & Hochberg,
Section 4.1). Light in one guide is the sum of the even and the odd supermode, which travel with
$\gamma_e$ and $\gamma_o$ (complex, with their losses), so after the coupled length $L$

$$
S_\text{through} = e^{i\bar\gamma L} \cos\frac{\Delta\gamma L}{2}, \qquad
S_\text{across} = i\thinspace e^{i\bar\gamma L} \sin\frac{\Delta\gamma L}{2},
$$

$\bar\gamma$ and $\Delta\gamma$ the supermodes' mean and difference: the power across is
$\sin^2(CL)$ with $C = \pi\Delta n/\lambda$ (their Eqs. 4.1 to 4.3), and the light crosses over
completely after $L_x = \lambda/(2\Delta n)$ (Eq. 4.5). The supermodes come as two dispersions:
by hand, from a coupling coefficient (`Supermodes::from_coupling`), or from the full-vector
solver with a mirror wall on the symmetry plane (`Supermodes::from_modes`: an electric wall
keeps the even TE-like supermode, a magnetic wall the odd). Only the coupled section is modelled:
the bends that bring the guides together couple too (their Section 4.1.4), and the coupler
reflects nothing.

The `directional_coupler` example reproduces their strip coupler: two 500 × 220 nm strips 200 nm
apart cross over in 37.15 µm on a 5 nm grid (36.74 on 10 nm) against their 37.5 µm (Eq. 4.14,
a fit to their 10 nm mesh's results).

## Multimode interference

The `Mmi` follows the guided-mode propagation analysis of Soldano & Pennings (their Section III).
The plane is seen from above: a ridge of effective index $n_r$ (the film's slab mode) between
cladding of $n_c$, as the [FDFD jobs](fdfd.md) see it. An access guide's mode launched into the
multimode section of width $W$ is decomposed into the section's guided modes $\psi_\nu$, each
travels with its own $\beta_\nu$, and what reaches the far face is projected on the output
guides' modes:

$$
S_{qp} = \sum_\nu t_{p\nu}\thinspace e^{i\beta_\nu L}\thinspace t_{\nu q},
$$

where $t_{a b}$ is the junction coefficient between two modes, power-normalized and without the
junction's reflection,
$t_{ab} = \tfrac{1}{2}\left[\int E_a \times H_b + \int E_b \times H_a\right]/\sqrt{P_a P_b}$
over the cross-section, each mode's fields from its own guide. For TE across the ridge ($E$
normal to the plane, the scalar $\psi$) this is
$\tfrac{1}{2}(\beta_a + \beta_b)\int\psi_a\psi_b / \sqrt{\beta_a\beta_b\int\psi_a^2\int\psi_b^2}$;
for TM ($H$ normal) each integral carries its guide's $1/n^2$. The modes are the exact modes of
symmetric slabs ([slab.md](slab.md)), with their exact $\beta$, not Soldano's paraxial ones
(their Eqs. 4, 5 and 7), and the integrals are Simpson's rule between every interface, outwards
in pieces doubling in length until every field has died.

The self-images form where the modes' phases realign, in units of the beat length of the two
lowest modes, $L_\pi = \pi/(\beta_0 - \beta_1)$ (their Eq. 6). A 1 × 2 splitter is fed in the
middle (symmetric interference) and splits at $3L_\pi/8$ (Eq. 37, $N = 2$), its outputs at
$\pm W/4$; a 2 × 2 3 dB coupler is fed at $\pm W/6$ (paired interference) and splits at
$L_\pi/2$ (Eq. 33), its outputs a quarter turn apart. Each `Mmi` starts at that length.

Its ports state the device's polarization, as a waveguide's do: on 220 nm SOI the TE-like
guide is TE in the film and TM across the ridge, so the lateral slab's TM modes make ports
stated TE, which connect to a TE-like waveguide.

What the model leaves out is the light the guided modes don't carry: the radiation modes the
junctions excite, which here is lost power, and the faces' reflections. Narrow access guides
excite the high modes too, whose $\beta$ are far from Soldano's parabola: a 3 µm 2 × 2 coupler
fed by 0.5 µm guides balances best at $0.44 L_\pi$, not $0.5 L_\pi$. Against 2D FDFD of the
same 1 × 2 splitter (`jobs/mmi-fdfd.toml`, 3 × 8.55 µm, 20 nm grid) each output's power is
0.398 against 0.381, the FDFD reflecting 0.2 % back; away from the image, at 7.5 µm, the FDFD
reflects 12 % and the model is no estimate.

## Y-branch

An ideal Y-branch splits its stem's power equally, $S_{21} = S_{31} = 10^{-A/20}/\sqrt{2}$, the
same back. It is passive but never lossless: light returning in the arms' odd combination has no
mode to go to in the stem and radiates, so $S$ has a zero singular value. A Mach-Zehnder
interferometer of two Y-branches loses its out-of-phase light that way.

## Rings

The rings are Bogaerts et al.'s closed forms. A round trip of length $L$ multiplies the ring's
field by $a e^{i\phi} = e^{i\gamma L}\thinspace 10^{-A_0/20}$, $A_0$ a fixed loss per round
trip in dB (the bends' and couplers', their Eq. 19), and a point coupler of self-coupling $r$
and cross-coupling $k$, $r^2 + k^2 = 1$, joins it to the bus in the convention of the `Coupler`.
The all-pass ring transmits

$$
S_\text{through} = e^{i(\pi + \phi)}\frac{a - r e^{-i\phi}}{1 - r a e^{i\phi}}
= \frac{r - a e^{i\phi}}{1 - r a e^{i\phi}}
$$

(their Eq. 1), the same backwards. The add-drop ring, $r_1, k_1$ on the input bus and
$r_2, k_2$ on the drop bus, with $A = a e^{i\phi}$ and $D = 1 - r_1 r_2 A$, has

$$
S_\text{in}^\text{through} = \frac{r_1 - r_2 A}{D}, \qquad
S_\text{in}^\text{drop} = S_\text{add}^\text{through} = \frac{-k_1 k_2\sqrt{A}}{D}, \qquad
S_\text{add}^\text{drop} = \frac{r_2 - r_1 A}{D},
$$

$\sqrt{A}$ the half round trip; their squares are Bogaerts's Eqs. 5 and 6, and the same
netlist of two couplers and two half rings solved as a circuit gives the same fields to round-off.
Each ring also gives its resonances ($n(\lambda)L = m\lambda$, Eq. 3), its free spectral range
$\lambda^2/(n_g L)$ (Eq. 9), its FWHM, finesse and loaded Q (Eqs. 7, 8 and 17 to 23), and its
powers on and off resonance (Eqs. 11 to 16). The FWHM formulas take the line to be Lorentzian
($\cos\phi \approx 1 - \phi^2/2$ across it), good to about $(1 - ra)^2$: measured on the
spectrum they agree to 6e-4 at a finesse of 110, but at a finesse of 5 (the `ring_q_factor`
example's 9 mm rings) the measured Q is 7 % above Eq. 20's. Bogaerts's Section 2.6, the
counter-directional coupling that splits resonances, is left out.

The `ring_q_factor` example reproduces their Fig. 5: at critical coupling, 2.7 dB/cm and the
bends' and couplers' 0.075 or 0.11 dB, the loaded Q peaks at 9.35 mm (all-pass, "approximately
10 mm") and 12.7 mm (add-drop, "almost 13 mm"), the peaks in the ratio 1.042 (theirs 1.42/1.36).

`jobs/ring-fdfd.toml`, an all-pass ring of 2 µm radius by 2D FDFD (25 nm grid), resonates at
1.52060 and 1.56730 µm, 46.70 nm apart; Eq. 9 with the group index of the exact bent slab of
the same plane (4.04 at the midpoint) says 46.95 nm. Both resonances sit 2.3 nm above the bent
slab's own, $\operatorname{Re}\nu = m$: the bus's coupling pulls them, and the 25 nm grid's
staircased ring is a little larger.

## Mach-Zehnder interferometer

`mzi` builds a Mach-Zehnder interferometer as a netlist (instances `splitter`, `combiner`,
`upper`, `lower`), solved by the [circuit solve](circuits.md) like any chip, and `mzi_y` the
same with Y-branches. Its closed form, without reflections, is the product of transfer matrices
$C_c \operatorname{diag}(t_\text{lower}, t_\text{upper})\thinspace C_s$, each coupler's
$C = \begin{pmatrix} t & x \\ x & t \end{pmatrix}$ in the basis (lower guide, upper guide),
$t$ its through and $x$ its across field; the netlist matches it to 2e-16, and without loss it is
unitary to 4e-16.

### Against measured interferometers

Dwivedi et al. (J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603; the
accepted manuscript is open at biblio.ugent.be) measured Mach-Zehnder interferometers on imec's
200 mm line: two 1 × 2 MMIs and silicon wires in oxide, designed 450, 600 and 800 nm wide and
215 nm thick, three interferometers per width. Two of low order ($m$ = 15 and 16, designed to
resonate at 1550 nm) give the effective index unambiguously, $n_\text{eff} L = m\lambda$
(their Eq. 3), and one of high order ($M$ = 110) the group index from its free spectral range,
$\lambda^2/(n_g L)$ (Eq. 8). Cross-section SEM puts the wires at 470 ± 4, 602 and 805 nm wide
and 211 ± 1 nm thick, and their Table I gives $n_\text{eff}$ and $n_g$ at 1550 nm with their
uncertainties.

The `mzi_dwivedi` example and the `components/mzi-*-dwivedi-*` cases predict them with nothing
taken from the measurement: each wire is the SEM's rectangle in photonoxide's silicon (Li 1980)
and silica (Malitson 1965, whose 1.4440 at 1550 nm is the paper's), its mode by
`Dispersion::from_hadley` on a quarter domain at about 10 nm; the interferometers' path
differences are designed as the paper's were, $m\lambda/n_\text{eff}$ of the drawn 215 nm wire
at 1550 nm; each interferometer is `mzi_y` of ideal splitters (a 1 × 2 MMI splits evenly) and
the wire's `Waveguide`; and its spectrum is read the way the paper reads the measured one:
$n_g$ from each pair of the $M$ = 110 interferometer's peaks,
$\lambda_1\lambda_2/((\lambda_2 - \lambda_1)\Delta L)$, fitted by a line in $\lambda$
(their Eq. 10), and $n_\text{eff}$ from the
$m$ = 15 interferometer's peak, carried to 1550 nm by $d(n/\lambda)/d\lambda = -n_g/\lambda^2$.

| Wire | $n_\text{eff}$ predicted | measured | $n_g$ predicted | measured |
|---|---|---|---|---|
| 470 × 211 nm | 2.3583 | 2.355 ± 0.002 | 4.2330 | 4.2739 ± 0.0042 |
| 602 × 211 nm | 2.5350 | 2.534 ± 0.0035 | 4.0385 | 4.0453 ± 0.0045 |
| 805 × 211 nm | 2.6581 | 2.67 ± 0.004 | 3.8883 | 3.8902 ± 0.005 |

The tolerance is the paper's own estimate of what fabrication moves: its Eq. 5,
$\Delta n = (dn/dw)\Delta w + (dn/dh)\Delta h$ with its Fig. 1's ±20 nm of width and ±5 nm of
thickness, the derivatives the solver's, plus Table I's uncertainty: 0.061, 0.042 and 0.031 in
$n_\text{eff}$, 0.049, 0.032 and 0.021 in $n_g$. Five of the six agree within 0.012. The 470 nm
wire's $n_g$ is 0.041 low, 84 % of its tolerance; within the SEM's own ±4 and ±1 nm it would be
allowed only 0.013. The narrowest wire feels its sidewalls most, and the model's rectangle leaves
out their slope (the SEM shows a trapezoid, its angle not printed) and roughness. The paper's
own simulations of the SEM geometry miss too (its Fig. 7, not tabulated), which it puts down to
"local environmental variations, and fabricated waveguide geometrical non-idealities". Across
the wafer the paper's standard deviations are 0.006 to 0.008 in $n_\text{eff}$ and 0.01 to 0.02
in $n_g$. The interferometers return the wire's own $n_\text{eff}$ and $n_g$ to about 1e-6: the
splitters, the path differences and the extraction add nothing, as they should.

This replaces the comparison first planned, Simphony's SiEPIC MZI against Lumerical INTERCONNECT
(Ploeg et al., Comput. Sci. Eng. 23, 65 (2021), doi:10.1109/MCSE.2020.3012099, Fig. 5), which
can't be reproduced from the paper: the figure gives the two curves without their numbers, it
needs the SiEPIC EBeam PDK's grating coupler and Y-branch data, and the MZI's arm lengths as
laid out (the listing's 50 and 150 µm don't fit the plotted fringes' 11 nm spacing).

## Sampled spectra

`Sampled` interpolates S-matrices sampled over wavelength: each $S_{qp}$ linearly in its
magnitude and its phase, the phase unwrapped between neighbouring samples, which must then be
less than half a turn apart (the sampling must resolve the device's delays and resonances).
`Sampled::from_fdfd` samples an `"fdfd"` job's device by 2D FDFD (`job::fdfd_s_parameters`),
its ports `o1`, `o2`, … in the job's order with the reference planes at the ports' columns: a 2D
estimate by the effective index method, not the device's 3D performance. For a smooth model,
fit the samples ([compact models](compact.md), `CompactModel::fit` of `Sampled::spectrum`).

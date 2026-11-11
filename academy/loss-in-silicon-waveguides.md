---
title: "Loss in silicon waveguides"
summary: "Where a silicon waveguide's light goes: scattering from rough sidewalls by the volume current, exactly on the slab and against Payne and Lacey's closed form, leakage through the buried oxide, radiation from bends, and how a loss is measured, by cutback and by a ring's Q."
topic: Waveguides
level: intermediate
minutes: 35
prerequisites: [slab-waveguide, strip-waveguide]
examples: [bend_loss, leaky_wire_benchmark, ring_q_factor]
jobs: [leaky-wire.yaml]
methods: [devices.md, multilayer.md, bends.md, components.md]
validation:
  - units/lossy-attenuation
  - roughness/payne-lacey-eq9
  - roughness/payne-lacey-slab
  - roughness/silicon-grid-order
  - mode/pml-soi-leakage-te
  - mode/pml-soi-leakage-tm
  - mode/leaky-wire-bienstman-loss
  - mode/bend-marcuse
  - mode/bend-slab-te-loss
  - circuit/ring-all-pass-bogaerts
charts: [loss-roughness, loss-correlation, leaky-substrate, bend-loss, loss-cutback, ring-identify]
papers:
  - cite: "D. Marcuse, Bell Syst. Tech. J. 48, 3187 (1969)"
    title: "Mode Conversion Caused by Surface Imperfections of a Dielectric Slab Waveguide"
    doi: 10.1002/j.1538-7305.1969.tb01742.x
    year: 1969
    role: origin
    note: "A slab whose walls stray from flat couples its guided mode to the radiation modes, by the spatial frequencies of the walls' deviation: the loss from rough walls as a coupled-mode problem, in the issue of the Bell System Technical Journal that opened integrated optics."
  - cite: "R. Soref, B. Bennett, IEEE J. Quantum Electron. 23, 123 (1987)"
    title: "Electrooptical effects in silicon"
    doi: 10.1109/JQE.1987.1073206
    year: 1987
    role: milestone
    note: "Silicon's index and absorption changed by free electrons and holes, from measured absorption spectra by the Kramers–Kronig relations, at 1.3 and 1.55 µm: the carriers that modulators use are also a loss."
  - cite: "J. P. R. Lacey, F. P. Payne, IEE Proc. J 137, 282 (1990)"
    title: "Radiation loss from planar waveguides with random wall imperfections"
    doi: 10.1049/ip-j.1990.0047
    year: 1990
    role: milestone
    note: "A slab's walls as a random process with an autocorrelation, each wall's deviation a sheet of current radiating into the cladding: the loss as the roughness's spectrum seen at the spatial frequencies that radiate."
  - cite: "F. P. Payne, J. P. R. Lacey, Opt. Quantum Electron. 26, 977 (1994)"
    title: "A theoretical analysis of scattering loss from planar optical waveguides"
    doi: 10.1007/BF00708339
    year: 1994
    role: milestone
    note: "The same slab's loss in closed form for an exponential autocorrelation, in normalized U, V and W, and its upper bound over every correlation length: the formula photonoxide's exact slab is checked against."
  - cite: "K. K. Lee, D. R. Lim, H.-C. Luan, A. Agarwal, J. Foresi, L. C. Kimerling, Appl. Phys. Lett. 77, 1617 (2000)"
    title: "Effect of size and roughness on light transmission in a Si/SiO2 waveguide: Experiments and model"
    doi: 10.1063/1.1308532
    year: 2000
    role: milestone
    note: "Silicon strips measured as they narrow, their loss rising as the width falls, and a model that traces it to the sidewalls' roughness: a map of loss against roughness for choosing a process."
  - cite: "K. K. Lee, D. R. Lim, L. C. Kimerling, J. Shin, F. Cerrina, Opt. Lett. 26, 1888 (2001)"
    title: "Fabrication of ultralow-loss Si/SiO2 waveguides by roughness reduction"
    doi: 10.1364/OL.26.001888
    year: 2001
    role: milestone
    note: "Sidewalls smoothed by oxidation and by anisotropic etching: a 0.5 µm wide single-mode strip's loss from 32 dB/cm, made the usual way, to 0.8 dB/cm."
  - cite: "Y. A. Vlasov, S. J. McNab, Opt. Express 12, 1622 (2004)"
    title: "Losses in single-mode silicon-on-insulator strip waveguides and bends"
    doi: 10.1364/OPEX.12.001622
    year: 2004
    role: milestone
    note: "Strips made on a 200 mm CMOS line: 3.6 ± 0.1 dB/cm for TE at 1.5 µm, and 0.086 dB per 90° bend at 1 µm radius, 0.013 dB at 2 µm."
  - cite: "T. Barwicz, H. A. Haus, J. Lightwave Technol. 23, 2719 (2005)"
    title: "Three-dimensional analysis of scattering losses due to sidewall roughness in microphotonic waveguides"
    doi: 10.1109/JLT.2005.850816
    year: 2005
    role: milestone
    note: "The volume current of a rectangular core's rough sidewalls radiating in 3D, for any contrast and polarization: 2D analyses overestimate the loss of small guides, and the loss depends strongly on polarization. photonoxide's sidewall_loss is their Section V."
  - cite: "W. R. McKinnon, D.-X. Xu, C. Storey, E. Post, A. Densmore, A. Delâge, P. Waldron, J. H. Schmid, S. Janz, Opt. Express 17, 18971 (2009)"
    title: "Extracting coupling and loss coefficients from a ring resonator"
    doi: 10.1364/OE.17.018971
    year: 2009
    role: milestone
    note: "A ring's loss and coupling from its resonances' widths, depths and spacings: the formulas can't tell which is which, and how the two change with wavelength or the device's dimensions is what disentangles them."
  - cite: "J. F. Bauters, M. J. R. Heck, D. John, D. Dai, M.-C. Tien, J. S. Barton, A. Leinse, R. G. Heideman, D. J. Blumenthal, J. E. Bowers, Opt. Express 19, 3163 (2011)"
    title: "Ultra-low-loss high-aspect-ratio Si3N4 waveguides"
    doi: 10.1364/OE.19.003163
    year: 2011
    role: current
    note: "Thin, wide silicon nitride cores that keep little field on their walls: 3 dB/m at a 2 mm bend radius, measured with rings and frequency-domain reflectometry, three orders of magnitude below a silicon strip."
  - cite: "Z. Ye, H. Jia, Z. Huang, C. Shen, J. Long, B. Shi, Y.-H. Luo, L. Gao, W. Sun, H. Guo, J. He, J. Liu, Photonics Res. 11, 558 (2023)"
    title: "Foundry manufacturing of tight-confinement, dispersion-engineered, ultralow-loss silicon nitride photonic integrated circuits"
    doi: 10.1364/PRJ.486379
    year: 2023
    role: current
    note: "Silicon nitride over 800 nm thick, tightly confining, made in a foundry process on 150 mm wafers with 2.6 dB/m loss and near 100% yield: low loss carried from the laboratory to manufacturing."
---

Where the light of a silicon waveguide goes, in the symbols the equations below use. The strip's
walls are rough and scatter light out; a bend radiates; the field reaches through the buried
oxide into the silicon below it; and carriers in the core absorb. Each takes a share per length,
and the shares add up to the loss $\alpha$.

::diagram loss

## 1. How is a waveguide's loss written?

A guide that loses light carries a power that falls exponentially along it,
$P(z) = P_0 e^{-\alpha z}$: each length takes the same fraction, whatever came before. The field's
amplitude falls at half the rate, $e^{-\alpha z/2}$. A mode solver gives the loss as the imaginary
part of the effective index: a field $e^{ik_0 n_\text{eff} z}$ carries a power
$e^{-2k_0 \mathop{\mathrm{Im}} n_\text{eff} z}$, so $\alpha = 2k_0 \mathop{\mathrm{Im}} n_\text{eff}$.

Papers and datasheets write it in decibels per centimetre, ten times the base-10 logarithm of
the power ratio over a centimetre:

$$
\alpha_\text{dB/cm} = 10\log_{10}(e)\, \alpha \cdot 10^4 = 4.343 \times 10^4\, \alpha,
$$

with $\alpha$ per µm. A loss of 3 dB/cm halves the power every centimetre; a silicon strip made on
a CMOS line loses about that much, 3.6 ± 0.1 dB/cm in
[Vlasov and McNab's measurement](https://doi.org/10.1364/OPEX.12.001622). Silicon nitride cores
built to keep their field off their walls reach
[3 dB/m](https://doi.org/10.1364/OE.19.003163), about a hundred times less.

::validation units/lossy-attenuation

### Why the losses add {theory}

Each mechanism takes power at its own rate where the guide carries it: roughness at the walls,
leakage through the oxide, absorption in the core, radiation in a bend. To first order they don't
interact, so the rates add,
$\alpha = \alpha_\text{rough} + \alpha_\text{leak} + \alpha_\text{abs} + \alpha_\text{bend}$,
and their decibels add too. Each section below takes one of them alone.

## 2. Why does a rough wall scatter light?

An etched wall isn't straight: it strays from its line by a few nanometres, $\sigma$ rms, in bumps
that stay alike over a correlation length $L_c$. A bump puts core where the cladding should be, or
cladding where the core should be, and that misplaced index is a small source of light, driven by
the guided field beside it. The sources along the wall add up with the phases of the guided wave,
$e^{i\beta z}$; a wave leaving at an angle $\theta$ from the axis in the cladding needs the phase
$n_2 k_0 \cos\theta\, z$, and only the part of the roughness whose spatial frequency fills the
difference, $\beta - n_2 k_0 \cos\theta$, sends light that way.

So the loss grows with three things: the roughness's strength, $\sigma^2$; the index step across
the wall, $(n_1^2 - n_2^2)^2$, which is why silicon in silica scatters far more than silica in
silica; and the field the mode keeps at the wall. The chart gives a slab's loss, both walls rough,
against its thickness.

::chart loss-roughness

**Try it.** Set the core to 2.0, silicon nitride's index. How much less does a 220 nm film lose,
and why?

:::answer
The index step's square, $(n_1^2 - n_2^2)^2$, falls from about 103 to 3.6 (from 3.5 and 1.45 to
2.0 and 1.45), so by about 29 times; and the nitride's weaker confinement changes the field at the
wall too. The figures give both losses at the marked thickness.
:::

### The volume current {theory}

A wall displaced by $f(z)$ from the smooth guide's is a sheet of polarization current beside it,

$$
J = -i\omega\varepsilon_0 (n_1^2 - n_2^2)\, E\, f(z),
$$

with $E$ the smooth guide's field at the wall. The roughness is random, with the exponential
autocorrelation $\langle f(z) f(z + u)\rangle = \sigma^2 e^{-|u|/L_c}$ and its spectrum

$$
\tilde R(\xi) = \frac{2\sigma^2 L_c}{1 + L_c^2 \xi^2}.
$$

Over a length the sheet radiates, at the angle $\theta$, what $\tilde R$ holds at
$\beta - n_2 k_0\cos\theta$, and the two walls, uncorrelated, add their powers. Divided by the
mode's power, that is the loss per length
([Lacey and Payne](https://doi.org/10.1049/ip-j.1990.0047)). For a symmetric slab of half-thickness
$d$, [Payne and Lacey](https://doi.org/10.1007/BF00708339) integrated it in closed form,

$$
\alpha = \frac{\sigma^2}{\sqrt{2}\, k_0 d^4 n_1}\, g(V)\, f_e(x, \gamma),
\qquad g = \frac{U^2 V^2}{1 + W},
$$

with the slab's normalized $U$, $V$ and $W$, and $f_e$ a function of $x = W L_c/d$ and of the
contrast through $\gamma$. photonoxide computes the same loss without the closed form, by summing
the radiated power on the exact slab mode, and the two agree once Payne and Lacey's $1/n_1$, the
weakly guiding slab's $k_0/\beta$, is read as such: the chart's dashed curve is their Eq. 10 as
printed, which differs from the exact loss by $\beta/(n_1 k_0)$. Their Eq. 9 for the field at the
wall carries a factor of two, as [Barwicz and Haus](https://doi.org/10.1109/JLT.2005.850816)
noted, which their Eq. 1 cancels:

::validation roughness/payne-lacey-eq9 roughness/payne-lacey-slab

## 3. Which strips and which roughness lose most?

A thinner core pushes its mode against the walls, so it loses more, and the loss of conventionally
etched strips climbs steeply as they narrow, as
[Lee et al. measured and modelled](https://doi.org/10.1063/1.1308532). But a core thin enough lets
the mode spill into the cladding, where the walls no longer hold much of it, and the loss falls
again: the first chart's curve peaks at a thickness, its figure "Lossiest thickness".

The correlation length decides which directions the roughness can send light. Very short bumps
scatter weakly at every angle; very long ones vary too slowly to reach the spatial frequency
$\beta - n_2 k_0$ that the shallowest radiation needs. Between them the loss peaks, near
$L_c \sim 1/(\beta - n_2 k_0)$.

::chart loss-correlation

**Try it.** At 220 nm, which correlation length is worst, and does it move for a 500 nm film?

:::answer
The figure "Lossiest $L_c$" gives it, of the order of $1/(\beta - n_2 k_0)$, the next figure. A
thicker film has a larger $\beta$, so a shorter worst correlation length.
:::

The roughness itself is the process's. Oxidizing the walls and etching along crystal planes
smoothed [Lee et al.'s](https://doi.org/10.1364/OL.26.001888) 0.5 µm strips from 32 dB/cm to
0.8 dB/cm; a 200 mm CMOS line gave Vlasov and McNab their 3.6 dB/cm.

### In three dimensions {research}

A slab overstates a strip's loss: its walls run the full height, and the field of a real strip
fades towards its top and bottom corners. Barwicz and Haus's 3D analysis gives each sidewall's
current the height of the core and lets it radiate beside the core's own slab, and finds that the
2D analyses substantially overestimate small guides' losses, and that the loss depends strongly on
polarization. photonoxide's strip device computes it from its mode by Hadley's equations (the
`roughness` and `correlation` parameters of the waveguide's 3D model): their 200 × 400 nm guide
with $L_c$ = 50 nm loses about 0.43 (dB/cm)/nm², so 1.7 dB/cm at $\sigma$ = 2 nm. Its field at the
corners is singular, and the grid study converges at the order that singularity predicts:

::validation roughness/silicon-grid-order

## 4. How thick must the buried oxide be?

A silicon-on-insulator strip sits on a few micrometres of oxide over a silicon substrate, whose
index, 3.48, exceeds every guided mode's effective index. Below the oxide the mode is no longer
bound: its field, decaying as $e^{-\gamma y}$ across the oxide, reaches the substrate and leaks
into it as a plane wave. The leakage falls as $e^{-2\gamma h}$ with the oxide's thickness $h$, so
each extra fraction of a micrometre divides it by a large factor.

::chart leaky-substrate

TM leaks first: its field reaches further into the oxide. Wafers for silicon photonics carry 2 or
3 µm of oxide, beyond the chart's range, where the leakage keeps falling as $e^{-2\gamma h}$. The
[leaky guides lesson](leaky-and-bent-guides.md) shows how photonoxide finds such a leaky mode, as
a complex root and with an absorbing layer:

::validation mode/pml-soi-leakage-te mode/pml-soi-leakage-tm mode/leaky-wire-bienstman-loss

::example leaky_wire_benchmark

## 5. How tight can a bend be?

A bend's mode must turn faster on its outside, and beyond a radius $\rho_r = R\, n_\text{eff}/n_2$
its phase fronts would have to outrun light in the cladding, so from there it radiates. The loss
falls exponentially with the radius, so a bend is either nearly lossless or useless: silicon's
high contrast lets it turn in micrometres, where a glass guide needs millimetres. Vlasov and McNab
measured 0.086 ± 0.005 dB per 90° bend at a radius of 1 µm and 0.013 ± 0.005 dB at 2 µm.

::chart bend-loss

::validation mode/bend-marcuse mode/bend-slab-te-loss

::example bend_loss

## 6. What does silicon itself absorb?

::coming
Free electrons and holes absorb in proportion to their densities, by Soref and Bennett's
measured spectra; two photons absorb together at high power and leave free carriers behind;
surface states absorb at the walls. Once 0.8 brings carriers into the library as a material model.

## 7. How is a loss measured?

A single transmission doesn't separate the guide's loss from the fibre's coupling to it. A
**cutback** does: guides of several lengths on one chip, each measured from fibre to fibre. Their
transmissions in dB fall on a straight line in the length, its slope the loss and its intercept the
two facets. Each facet is coupled anew for every guide, so each point strays a little from the
line, and the slope's uncertainty shrinks with more and longer guides.

::chart loss-cutback

**Try it.** With 0.3 dB of scatter, how long must the longest guide be to measure 0.5 dB/cm to
within 10%?

:::answer
The slope's standard error falls as one over the longest length: here about 0.36 dB/cm divided
by the longest guide in cm, so it drops below 0.05 dB/cm past about 7 cm. Move "Longest guide"
and watch the error figure.
:::

A **ring** measures a short guide's loss without a cutback. A ring's intrinsic quality factor is
set by its round trip's loss alone,

$$
Q_i = \frac{2\pi n_g}{\lambda \alpha},
$$

so a resonance's width gives $\alpha$; the cutback chart's last figure is the $Q_i$ its loss
would give. But the width mixes the loss with the coupling, and the depth alone can't tell which
is which: an under-coupled ring and its over-coupled twin have the same power spectrum.
[McKinnon et al.](https://doi.org/10.1364/OE.17.018971) disentangled them by how each changes
with wavelength or with the device; the phase does it at once, as fitting the complex field shows:

::chart ring-identify

::validation circuit/ring-all-pass-bogaerts

::example ring_q_factor

## 8. How photonoxide computes it

- **Roughness on a slab:** `mode::roughness::slab_loss` sums the radiated power of both walls'
  currents on the exact `mode::slab` mode by Gauss–Legendre quadrature, and
  `mode::roughness::payne_lacey` is Payne and Lacey's closed form, its check.
- **Roughness on a strip:** `mode::roughness::sidewall_loss`, Barwicz and Haus's 3D analysis, on
  a mode by Hadley's equations; the waveguide device's 3D model adds it to its loss
  ([the devices](../docs/methods/devices.md)).
- **Leakage:** `Multilayer::mode_near`, a complex root of the multilayer's dispersion function
  ([multilayers](../docs/methods/multilayer.md)), and the full-vector solver with a PML.
- **Bends:** the bent slab exactly, and Marcuse's formula ([bends](../docs/methods/bends.md)).
- **In a circuit:** every waveguide component takes its loss in dB/cm
  ([components](../docs/methods/components.md)).

## 9. History and today {research}

[Marcuse](https://doi.org/10.1002/j.1538-7305.1969.tb01742.x) treated a slab's imperfect walls as
coupling its mode to radiation in 1969.
[Lacey and Payne](https://doi.org/10.1049/ip-j.1990.0047) made the walls a random process and
[Payne and Lacey](https://doi.org/10.1007/BF00708339) gave its closed form, a loss growing as the
square of the index step across the wall, so that silicon in silica needs far smoother walls than
glass. Silicon
photonics then spent years smoothing them: [Lee et al.](https://doi.org/10.1364/OL.26.001888) by
oxidation, and [Vlasov and McNab](https://doi.org/10.1364/OPEX.12.001622) by moving to a CMOS
line. [Barwicz and Haus](https://doi.org/10.1109/JLT.2005.850816) took the analysis to three
dimensions.

Today the lowest losses come from guides that keep their light off their walls: thin, wide silicon
nitride cores at [3 dB/m](https://doi.org/10.1364/OE.19.003163), and tightly confining nitride made
in a [foundry at 2.6 dB/m](https://doi.org/10.1364/PRJ.486379). Silicon strips stay at a few
dB/cm; wider or shallow-etched guides keep less field on their walls, and long runs are often
made in them, narrowing only where the light must bend or couple.

::timeline

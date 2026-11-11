---
title: "Tolerances, corners and yield"
summary: "A design as the process makes it: how far a nanometre of width or thickness moves a ring's resonance and an MMI's or a coupler's split, corners that bound a design between the process's extremes, studies that run them, and the statistics, Monte Carlo and yield, that come after."
topic: Fabrication
level: intermediate
minutes: 25
prerequisites: [strip-waveguide, ring-resonator, how-a-chip-is-made]
methods: [devices.md, eim.md]
validation:
  - device/study-corners-closed-form
  - device/study-sweep-analytic
  - device/waveguide-fit-neff
charts: [tolerance-ring, mmi-tolerance, coupler-gap]
papers:
  - cite: "S. K. Selvaraja, P. Jaenen, W. Bogaerts, D. Van Thourhout, P. Dumon, R. Baets, J. Lightwave Technol. 27, 4076 (2009)"
    title: "Fabrication of Photonic Wire and Crystal Circuits in Silicon-on-Insulator Using 193-nm Optical Lithography"
    doi: 10.1109/JLT.2009.2022282
    year: 2009
    role: milestone
    note: "Silicon wires made with 193 nm lithography and dry etching in place of 248 nm: a larger process window, better linearity and less proximity effect, and 2.7 dB/cm for a 500 nm wire, the process precision that photonic circuits needed for mass production."
  - cite: "W. A. Zortman, D. C. Trotter, M. R. Watts, Opt. Express 18, 23598 (2010)"
    title: "Silicon photonics manufacturing"
    doi: 10.1364/OE.18.023598
    year: 2010
    role: milestone
    note: "Resonant silicon devices measured across wafers: how far their resonances scatter from the film's thickness and the strips' width, and what that scatter costs in tuning."
  - cite: "T.-W. Weng, Z. Zhang, Z. Su, Y. Marzouk, A. Melloni, L. Daniel, Opt. Express 23, 4242 (2015)"
    title: "Uncertainty quantification of silicon photonic devices with correlated and non-Gaussian random parameters"
    doi: 10.1364/OE.23.004242
    year: 2015
    role: milestone
    note: "A device's response as a polynomial in its random parameters, correlated and not Gaussian, fitted from a few deterministic solves: its statistics without thousands of Monte Carlo runs."
  - cite: "Z. Lu, J. Jhoja, J. Klein, X. Wang, A. Liu, J. Flueckiger, J. Pond, L. Chrostowski, Opt. Express 25, 9712 (2017)"
    title: "Performance prediction for silicon photonics integrated circuits with layout-dependent correlated manufacturing variability"
    doi: 10.1364/OE.25.009712
    year: 2017
    role: milestone
    note: "Variation that depends on where a component sits on the wafer and what surrounds it: nearby devices vary together, and a circuit's Monte Carlo must draw them so."
  - cite: "A. Waqas, D. Melati, P. Manfredi, A. Melloni, Opt. Express 26, 5894 (2018)"
    title: "Stochastic process design kits for photonic circuits based on polynomial chaos augmented macro-modelling"
    doi: 10.1364/OE.26.005894
    year: 2018
    role: milestone
    note: "Polynomial chaos inside a design kit's component models, so a circuit simulator propagates the process's spread through a whole circuit."
  - cite: "W. Bogaerts, Y. Xing, U. Khan, IEEE J. Sel. Top. Quantum Electron. 25(5), 1 (2019)"
    title: "Layout-Aware Variability Analysis, Yield Prediction, and Optimization in Photonic Integrated Circuits"
    doi: 10.1109/JSTQE.2019.2906271
    year: 2019
    role: review
    note: "Variability analysis that knows the layout, from a circuit's components and their places to its yield, and designs optimized for it."
  - cite: "Y. Xing, J. Dong, U. Khan, W. Bogaerts, ACS Photonics (online 2022)"
    title: "Capturing the Effects of Spatial Process Variations in Silicon Photonic Circuits"
    doi: 10.1021/acsphotonics.2c01194
    year: 2022
    role: current
    note: "Spatial process variation measured and modelled across wafers and carried into circuit simulations: the width and thickness maps that make nearby devices alike and distant ones differ."
---

A design and the process that makes it, in the symbols the equations below use. A strip drawn $w$
wide on a film $t$ thick comes out a few nanometres off, $\Delta w$ and $\Delta t$, differently on
each chip and across each wafer. A ring's resonance moves with them; corners bound a design
between the process's extremes; and statistics over many draws give its yield.

::diagram tolerance

## 1. How far does a nanometre move a ring?

A resonance sits where a whole number of wavelengths fits the ring, $m\lambda = n_\text{eff}L$.
Change the strip and its effective index changes; the ring keeps its order $m$, so the resonance
moves by

$$
\Delta\lambda = \lambda\,\frac{\Delta n_\text{eff}}{n_g},
$$

the group index $n_g$ appearing because the index changes with the wavelength too. A silicon strip
holds its mode tightly, so its effective index is sensitive to every nanometre: of the order of a
nanometre of resonance per nanometre of width, and more per nanometre of thickness, since a
220 nm film is the strip's smaller dimension. The chart solves the strip at every error:

::chart tolerance-ring

**Try it.** Widen the strip to 800 nm. Which error now matters most, and how do the corners
compare with a ring's free spectral range?

:::answer
The width's slope falls: the mode of a wide strip barely feels its walls. The thickness's
remains, so the corners' spread barely shrinks. A spread approaching the FSR, the last figure,
means a ring can come out anywhere in its period: only a heater can bring it back.
:::

### Sensitivities {theory}

For small errors the shift is linear in each, and the errors add,

$$
\Delta\lambda \approx \frac{\lambda}{n_g}\left(\frac{\partial n_\text{eff}}{\partial w}\,\Delta w
+ \frac{\partial n_\text{eff}}{\partial t}\,\Delta t\right),
$$

with $\partial n_\text{eff}/\partial w$ largest where the field presses on the side walls and
$\partial n_\text{eff}/\partial t$ where it presses on the top and bottom. The chart takes each
derivative by solving the strip a nanometre either side, and the corners solving it anew rather
than adding slopes. It solves by the effective index method, smooth in every dimension; the
full-vector solver on a grid gives the [strip's index](strip-waveguide.md) more exactly, and the
device's analytic fit to it is checked between its nodes:

::validation device/waveguide-fit-neff

## 2. How do other devices feel the same errors?

Every device whose function rests on an index or a beat length moves the same way. An MMI's length
is fixed by its section's beat length, which grows as the square of its width, so a width error
moves its self-images away from the outputs:

::chart mmi-tolerance

A directional coupler's cross-over length grows exponentially with its gap, so a gap error changes
its split by far more than the same error in a single strip changes its index:

::chart coupler-gap

**Try it.** Read off the coupler's cross-over length at 200 and 220 nm gaps. What split would a
coupler designed for 50% at 200 nm give at 220 nm?

:::answer
The cross-over length grows by a factor the chart's slope gives, so the coupler, its length now a
smaller share of $L_c$, crosses over less than 50%: the power crossed is $\sin^2(\pi L/2L_c)$, and
a longer $L_c$ makes the angle smaller.
:::

## 3. What are corners, and how are they run?

A process's errors aren't independent of each other on one chip: a lithography that prints every
line wider does so to all of them, and a film that comes out thin is thin everywhere nearby. So a
design is first checked at its **corners**: the nominal design and the combinations of each error
at its extremes, the width $\pm\Delta w$ and the thickness $\pm\Delta t$, and a temperature. If the
design works at every corner it works between them, as long as its response is monotonic in each
error. A study runs them: on a component's Study tab, or as a job of `kind = "corners"`, each
corner the design with offsets added to its parameters and its materials at a temperature, each
solved by the device's own model.

::validation device/study-corners-closed-form device/study-sweep-analytic

Corners are pessimistic: every error at its worst together is rare. And they say nothing of how
often a design fails, which is what a yield needs.

## 4. How many chips will work?

::coming
Monte Carlo over the process's distributions, each draw a design solved, and the yield, the share
that meets the specification; polynomial chaos (Weng et al.), which gets the same statistics from
a few deterministic solves, as 0.11 brings them.

## 5. Why do nearby devices vary together?

::coming
The width and thickness as maps across the wafer, measured and modelled (Lu et al.; Xing et al.):
two rings side by side differ by far less than two on different chips, which is why matched
designs and on-chip references work; location-correlated Monte Carlo comes with 0.11.

## 6. How is a design made robust?

The shift is a derivative, and a design can choose where it is small. A wider strip feels its
width less; a ring can be designed with the derivatives of its two arms' indices cancelling; a
coupler can be made of tapered or bent sections whose errors cancel; and a heater, its power
budgeted for the corners' spread, tunes what the process leaves. The figure "Corners' spread"
against the FSR says how much tuning a ring needs. Designs found by optimization can be made
robust by optimizing them at the corners together, nominal, eroded and dilated, as
[inverse design](inverse-design.md) will.

## 7. How photonoxide computes it

- **Sensitivities:** the strip's index by the effective index method (`mode::eim::Ridge`) or by
  the full-vector solver ([effective index method](../docs/methods/eim.md)); a device's response
  at any parameters by its own models.
- **Corners and sweeps:** `job::study_from_device` writes a study of a device's design, each point
  a design of its own, recorded with its figures and S-matrices
  ([devices, "Studied as a job"](../docs/methods/devices.md)).
- **Process:** the fabrication engine says how each dimension is made and how accurately
  ([how a chip is made](how-a-chip-is-made.md)).

## 8. History and today {research}

[Selvaraja et al.](https://doi.org/10.1109/JLT.2009.2022282) moved silicon photonics to 193 nm
lithography, for the process window and the linewidth control its devices needed.
[Zortman et al.](https://doi.org/10.1364/OE.18.023598) measured how far resonant devices scatter
across wafers. Statistics followed: [Weng et al.](https://doi.org/10.1364/OE.23.004242) took a
device's statistics from polynomial chaos, [Waqas et al.](https://doi.org/10.1364/OE.26.005894) put
it into design kits, and [Lu et al.](https://doi.org/10.1364/OE.25.009712) showed that variation
depends on where a device is and what is near it. Today layout-aware analysis
([Bogaerts et al.](https://doi.org/10.1109/JSTQE.2019.2906271)) and measured spatial variation
([Xing et al.](https://doi.org/10.1021/acsphotonics.2c01194)) carry the process into the circuit
simulation, and yield is designed for rather than measured afterwards.

::timeline

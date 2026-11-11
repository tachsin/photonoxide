---
title: "Edge couplers and spot-size converters"
summary: "Bringing a fibre's light in through the chip's edge: the overlap of two modes, Gaussian against Gaussian with offset and tilt; the inverse taper that grows a silicon mode to meet a fibre's, and how long it must be; and the converters, facets and packages built on them."
topic: Coupling
level: intermediate
minutes: 25
prerequisites: [strip-waveguide, crossings-and-converters]
methods: [eme.md, vector.md]
validation:
  - eme/beam-slab-coupling
  - eme/beam-fresnel
  - mode/effective-area-gaussian
  - eme/taper-steps-order
  - eme/taper-adaptive
  - eme/taper-fdfd
  - convergence/eme-taper-fdfd
charts: [edge-overlap, spot-size, taper-length]
papers:
  - cite: "D. Marcuse, Bell Syst. Tech. J. 56, 703 (1977)"
    title: "Loss Analysis of Single-Mode Fiber Splices"
    doi: 10.1002/j.1538-7305.1977.tb00534.x
    year: 1977
    role: origin
    note: "The loss of joining two single-mode fibres, their modes taken as Gaussians: closed forms for a mismatch of their sizes, an offset between their axes and a tilt between them, the formulas every fibre-to-chip coupling is first estimated with."
  - cite: "T. Shoji, T. Tsuchizawa, T. Watanabe, K. Yamada, H. Morita, Electron. Lett. 38, 1669 (2002)"
    title: "Low loss mode size converter from 0.3 µm square Si wire waveguides to singlemode fibres"
    doi: 10.1049/el:20021185
    year: 2002
    role: milestone
    note: "A silicon wire 0.3 µm square narrowed to a tip inside a larger, low-index waveguide that carries the light on to a single-mode fibre: the mode-size converter as two stages, the wire handing its light to a guide of the fibre's size."
  - cite: "V. R. Almeida, R. R. Panepucci, M. Lipson, Opt. Lett. 28, 1302 (2003)"
    title: "Nanotaper for compact mode conversion"
    doi: 10.1364/OL.28.001302
    year: 2003
    role: milestone
    note: "A silicon wire narrowed over 40 µm to a tip about 100 nm wide at the facet, its mode spread into the silica around it to meet a lensed fibre's: the short inverse taper, against the hundreds of micrometres proposed for laser diodes."
  - cite: "M. Pu, L. Liu, H. Ou, K. Yvind, J. M. Hvam, Opt. Commun. 283, 3678 (2010)"
    title: "Ultra-low-loss inverted taper coupler for silicon-on-insulator ridge waveguide"
    doi: 10.1016/j.optcom.2010.05.034
    year: 2010
    role: milestone
    note: "The inverse taper made for ridge waveguides, whose slab stays at the facet, coupling to a lensed fibre with very little loss."
  - cite: "J. Cardenas, C. B. Poitras, K. Luke, L.-W. Luo, P. A. Morton, M. Lipson, IEEE Photonics Technol. Lett. 26, 2380 (2014)"
    title: "High Coupling Efficiency Etched Facet Tapers in Silicon Waveguides"
    doi: 10.1109/LPT.2014.2357177
    year: 2014
    role: milestone
    note: "The chip's facet etched by lithography rather than diced and polished, the taper ending exactly where it was drawn: edge couplers defined with the rest of the chip."
  - cite: "R. Marchetti, C. Lacava, L. Carroll, K. Gradkowski, P. Minzioni, Photonics Res. 7, 201 (2019)"
    title: "Coupling strategies for silicon photonics integrated chips [Invited]"
    doi: 10.1364/PRJ.7.000201
    year: 2019
    role: review
    note: "In-plane and out-of-plane couplers compared: edge couplers give high efficiency, a broad band and little polarization dependence but need facets and careful assembly; gratings give less of each but test on the wafer and package easily."
  - cite: "X. Mu, S. Wu, L. Cheng, H. Y. Fu, Appl. Sci. 10, 1538 (2020)"
    title: "Edge Couplers in Silicon Photonic Integrated Circuits: A Review"
    doi: 10.3390/app10041538
    year: 2020
    role: review
    note: "Edge couplers by their structure, single tapers, multi-tip and trident tapers, two-stage converters through silicon nitride or polymer, suspended cantilevers, with their efficiencies, fabrication and packaging, beside the grating coupler."
---

A fibre meeting a silicon chip at its edge, in the symbols the equations below use. The strip, $w$
wide, narrows over a length $L$ to a tip $w_t$ wide at the facet, and its mode, too weakly held by
the tip, spreads into the cladding until it is as large as the fibre's. What crosses the gap is
the overlap of the two modes, $2w_c$ and $2w_f$ across, offset by $d$ and tilted by $\theta$.

::diagram edge

## 1. Edge or grating: how does light get onto a chip?

A single-mode fibre's mode is about 10 µm across; a silicon strip's, half a micrometre. Two
families of couplers bridge them ([Marchetti et al.](https://doi.org/10.1364/PRJ.7.000201)). A
**grating coupler** diffracts the light up out of the chip's surface, where a fibre meets it from
above, anywhere on the wafer: it can be tested before the wafer is cut, but its band is narrow and
it treats the two polarizations differently ([grating couplers](grating-couplers.md)). An **edge
coupler** takes the light in at the chip's side, in its own plane: its coupling is high, its band
broad and its polarization dependence small, but it needs a facet made at the chip's edge and a
fibre aligned to it, which happens only after the wafer is cut. This lesson is about the second.

## 2. How much light do two modes share?

Light crossing from one guide into another goes into the second guide's mode as far as its field
looks like that mode. The share of the power is the overlap of the two fields across the facet,

$$
\eta = \frac{\left\lvert \int E_1 E_2^* \,dA\right\rvert^2}{\int \lvert E_1\rvert^2 dA \int \lvert E_2\rvert^2 dA},
$$

1 for identical fields and less for any difference: in size, in position or in direction. Both a
fibre's mode and a spot-size converter's are close to Gaussians, and for two Gaussians the overlap
is a closed form ([Marcuse](https://doi.org/10.1002/j.1538-7305.1977.tb00534.x)). The chart sums it
on the fields themselves, against the offset, for the fibre chosen and three common ones:

::chart edge-overlap

**Try it.** A bare 500 nm strip's mode is roughly 0.5 µm across. Set the chip's mode to it: what
does a standard fibre take, and what does a lensed one?

:::answer
About −20 dB into the 10.4 µm fibre: the size mismatch alone wastes 99% of the light. The lensed
fibre's 2.5 µm spot takes about −8.3 dB, and loses 1 dB more at 0.43 µm of offset. A converter's
job is to grow the chip's mode until a fibre forgives both.
:::

### Two Gaussians {theory}

Take each mode as $e^{-r^2/w^2}$ across the facet, $w$ its field's $1/e$ radius, so that $2w$ is
its intensity's $1/e^2$ diameter, the mode-field diameter. Two of radii $w_1$ and $w_2$, their axes
$d$ apart, overlap as

$$
\eta = \left(\frac{2w_1w_2}{w_1^2 + w_2^2}\right)^2 \exp\left(-\frac{2d^2}{w_1^2 + w_2^2}\right).
$$

The first factor is the size mismatch; the second the offset, which costs 1 dB more at
$d = \sqrt{(w_1^2 + w_2^2)\ln 10/20}$, so a small mode against a small fibre leaves the least room.
A tilt $\theta$ adds a phase that grows across the facet, $e^{ikn\sin\theta\, x}$, and costs
another Gaussian factor in $\theta$, smaller for smaller modes, which spread over more angles. The
chart's sum and the closed form agree to round-off; photonoxide's beams launched on a slab's modes
are checked the same way:

::validation eme/beam-slab-coupling eme/beam-fresnel mode/effective-area-gaussian

## 3. How does an inverse taper grow the mode?

Narrow a silicon strip and its mode feels less silicon: its effective index falls towards the
cladding's, and it spreads further into the cladding to keep its power. Near the guide's cutoff it
spreads fast, so a tip a little over 100 nm wide holds a mode micrometres across. An **inverse
taper** narrows the strip towards the facet until its mode matches the fibre's
([Almeida et al.](https://doi.org/10.1364/OL.28.001302)). The tip's width sets the mode's size;
the [crossings and converters lesson](crossings-and-converters.md) solves the tip's mode and its
overlap with a focused beam:

::chart spot-size

**Try it.** Narrow the tip from 200 to 140 nm. Which way does the best numerical aperture move, and
why does a standard fibre still need a lens?

:::answer
The best NA falls as the tip narrows: the mode grows, so a wider, less tightly focused beam
matches it. Even at 140 nm the mode is about 5 µm across, short of a standard fibre's 10.4,
so a lensed or high-NA fibre, or a second converter stage, closes the rest.
:::

## 4. How long must the taper be?

A taper keeps its light in one mode only if it changes slowly enough: its local mode must turn
into the next section's without shedding power into the others or into radiation. The rule is
adiabatic: the taper's angle must stay well below the beat length between the mode and its
nearest neighbour, and near the tip that neighbour is the radiation itself, whose index the mode
is approaching, so the last stretch of a taper must be the slowest. Too short a taper loses light;
long enough, it delivers nearly all of it, and lengthening it more gains nothing. Eigenmode
expansion computes a taper's transmission from the modes of its cross-sections, cut into
sections:

::chart taper-length

The sections' steps are where the expansion's error comes from: second order for equal steps,
and placed where the response changes when they are adaptive. photonoxide's expansion is checked
for that order, for its adaptive steps and against FDFD on the same taper:

::validation eme/taper-steps-order eme/taper-adaptive eme/taper-fdfd convergence/eme-taper-fdfd

### What else meets the fibre {research}

The single inverse taper has relatives, each answering one of its limits
([Mu et al.](https://doi.org/10.3390/app10041538)). A tip inside a larger low-index guide hands its
light to a guide of the fibre's size, two stages instead of one
([Shoji et al.](https://doi.org/10.1049/el:20021185)); silicon nitride or polymer serve as that
guide. Ridge waveguides keep their slab to the facet ([Pu et al.](https://doi.org/10.1016/j.optcom.2010.05.034)).
Several tips side by side, or a trident, shape the mode in both directions; a tip suspended over
an etched cavity, a cantilever, removes the substrate below the grown mode. And a facet etched
with the rest of the chip ([Cardenas et al.](https://doi.org/10.1109/LPT.2014.2357177)) puts the
tip exactly where it was drawn, where dicing and polishing leave it wherever the cut falls. A facet
also reflects a few percent of the light back, which angled facets and index-matching gel keep
out of the circuit.

## 5. How much reaches a real fibre?

::coming
A fibre's mode launched on a 3D plane at the tip by eigenmode expansion, its coupling, offset and
tilt computed on the taper's real mode rather than a Gaussian, and angled facets, as #432 brings
them.

## 6. How photonoxide computes it

- **Overlaps:** `eme::Beam`, a Gaussian across a plane, tilted by an angle, launched on a
  section's modes; the chart above sums two of them ([eigenmode expansion](../docs/methods/eme.md)).
- **The tip's mode:** `mode::vector::modes` on the strip's cross-section with its fields, and their
  overlap with a beam ([the full-vector solver](../docs/methods/vector.md)).
- **Tapers:** `eme::Eme` cuts a taper into sections, adaptively where its response changes, and
  cascades their S-matrices.

## 7. History and today {research}

[Marcuse](https://doi.org/10.1002/j.1538-7305.1977.tb00534.x) gave the splice losses of Gaussian
modes in 1977, and the same formulas still give the first estimate of any coupler. Silicon wires
brought the problem to its extreme, a mode a twentieth of the fibre's:
[Shoji et al.](https://doi.org/10.1049/el:20021185) bridged it in two stages and
[Almeida et al.](https://doi.org/10.1364/OL.28.001302) with a single short inverse taper. Since,
the converters have multiplied ([Mu et al.](https://doi.org/10.3390/app10041538)), the facets have
been etched rather than cut ([Cardenas et al.](https://doi.org/10.1109/LPT.2014.2357177)), and the
choice between edge and grating has become one of packaging as much as of optics
([Marchetti et al.](https://doi.org/10.1364/PRJ.7.000201)).

::timeline

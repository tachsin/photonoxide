---
title: "How a chip is made"
summary: "From the bare wafer to the device, question by question: where the thin film comes from, how a pattern is written and how small it can be, why an etch stops where it does, how films are grown and deposited, lift-off, polishing, why the order of the steps matters, and what photonoxide's fabrication engine checks and what it does not claim; each step played on the wafer's cross-section."
topic: Fabrication
level: introductory
minutes: 45
prerequisites: [strip-waveguide]
jobs: [strip-flow.yaml, strip-flow-smart-cut.yaml]
methods: [fabrication.md]
validation:
  - fabrication/smart-cut-range-bruel
  - fabrication/lithography-choice
  - fabrication/min-features-above-limit
  - fabrication/etch-stop-or-timed
  - fabrication/deal-grove-tables
  - fabrication/deal-grove-campbell
  - fabrication/thermal-order
  - fabrication/reported-flows
  - fabrication/accuracy-combined
  - fabrication/round-trip-waveguide
charts: [fab-resolution, fab-oxidation]
papers:
  - cite: "B. E. Deal, A. S. Grove, J. Appl. Phys. 36, 3770 (1965)"
    title: "General Relationship for the Thermal Oxidation of Silicon"
    doi: 10.1063/1.1713945
    year: 1965
    role: origin
    note: "Silicon's oxidation as the oxidant's diffusion through the oxide in series with its reaction at the silicon: one law, linear while the oxide is thin and parabolic once it is thick, with rate constants measured from 700 to 1200 °C in dry and wet oxygen."
  - cite: "M. Bruel, Electron. Lett. 31, 1201 (1995)"
    title: "Silicon on insulator material technology"
    doi: 10.1049/el:19950805
    year: 1995
    role: milestone
    note: "Smart Cut: hydrogen implanted into one wafer, bonded to a second, and split along the implanted plane by heating, leaving a thin single-crystal film on oxide whose thickness the implant's energy sets."
  - cite: "S. Y. Chou, P. R. Krauss, P. J. Renstrom, Science 272, 85 (1996)"
    title: "Imprint Lithography with 25-Nanometer Resolution"
    doi: 10.1126/science.272.5258.85
    year: 1996
    role: milestone
    note: "A pattern pressed into a resist by a mould instead of written by light: 25 nm features with vertical walls, at the throughput of a press."
  - cite: "W. Bogaerts, R. Baets, P. Dumon, V. Wiaux, S. Beckx, D. Taillaert, B. Luyssaert, J. Van Campenhout, P. Bienstman, D. Van Thourhout, J. Lightwave Technol. 23, 401 (2005)"
    title: "Nanophotonic waveguides in silicon-on-insulator fabricated with CMOS technology"
    doi: 10.1109/JLT.2004.834471
    year: 2005
    role: milestone
    note: "Photonic wires and photonic crystals made on a CMOS line with 248 nm deep-UV lithography: the step that put silicon photonics on electronics' tools, wires losing 0.24 dB/mm."
  - cite: "C. Mack, Fundamental Principles of Optical Lithography, Wiley (2007)"
    title: "Fundamental Principles of Optical Lithography: The Science of Microfabrication"
    doi: 10.1002/9780470723876
    year: 2007
    role: review
    note: "Optical lithography from the image to the resist: the lens's numerical aperture, water immersion that takes it past 1, and the resolution and depth of focus that follow."
  - cite: "S. K. Selvaraja, P. Jaenen, W. Bogaerts, D. Van Thourhout, P. Dumon, R. Baets, J. Lightwave Technol. 27, 4076 (2009)"
    title: "Fabrication of Photonic Wire and Crystal Circuits in Silicon-on-Insulator Using 193-nm Optical Lithography"
    doi: 10.1109/JLT.2009.2022282
    year: 2009
    role: milestone
    note: "The same circuits with 193 nm lithography on 200 mm SOI wafers: the resist stack, the HBr/O₂ etch that stops on the buried oxide, the walls it leaves and the linewidth it holds; the flow the engine gives a silicon strip."
  - cite: "M. Krishnan, J. W. Nalaskowski, L. M. Cook, Chem. Rev. 110, 178 (published online 2009; the 2010 issue)"
    title: "Chemical Mechanical Planarization: Slurry Chemistry, Materials, and Mechanisms"
    doi: 10.1021/cr900170z
    year: 2009
    role: review
    note: "How a slurry's chemistry and a pad's pressure together polish a wafer flat, and how flat: the nonplanarity a polish leaves."
  - cite: "S. Franssila, Introduction to Microfabrication, 2nd edition, Wiley (2010)"
    title: "Introduction to Microfabrication"
    doi: 10.1002/9781119990413
    year: 2010
    role: review
    note: "A textbook of the unit processes: step coverage, resist selectivity, over-etch, RIE lag and the uniformity of a polish, the numbers the engine's rules read."
  - cite: "R. J. Bojko, J. Li, L. He, T. Baehr-Jones, M. Hochberg, Y. Aida, J. Vac. Sci. Technol. B 29, 06F309 (2011)"
    title: "Electron beam lithography writing strategies for low loss, high confinement silicon optical waveguides"
    doi: 10.1116/1.3653266
    year: 2011
    role: milestone
    note: "Silicon guides written by an electron beam in HSQ and etched in chlorine: how the beam's fields are stitched together, and the loss that follows."
  - cite: "G. Poberaj, H. Hu, W. Sohler, P. Günter, Laser Photonics Rev. 6, 488 (2012)"
    title: "Lithium niobate on insulator (LNOI) for micro-photonic devices"
    doi: 10.1002/lpor.201100035
    year: 2012
    role: review
    note: "Lithium niobate films on oxide by ion slicing and bonding, the anneals they survive, and the devices etched in them."
  - cite: "Y. Chen, Microelectron. Eng. 135, 57 (2015)"
    title: "Nanofabrication by electron beam lithography and its applications: A review"
    doi: 10.1016/j.mee.2015.02.042
    year: 2015
    role: review
    note: "Electron-beam lithography's resists, tones and resolutions, down to a few nanometres in HSQ."
  - cite: "M. Zhang, C. Wang, R. Cheng, A. Shams-Ansari, M. Lončar, Optica 4, 1536 (2017)"
    title: "Monolithic ultra-high-Q lithium niobate microring resonator"
    doi: 10.1364/OPTICA.4.001536
    year: 2017
    role: milestone
    note: "Lithium niobate rings written in HSQ and etched by argon ions with an intrinsic Q of ten million: the flow thin-film lithium niobate devices have followed since."
  - cite: "B. T. Kearney, B. Jugdersuren, J. C. Culbertson, P. A. DeSario, X. Liu, Thin Solid Films 661, 78 (2018)"
    title: "Substrate and annealing temperature dependent electrical resistivity of sputtered titanium nitride thin films"
    doi: 10.1016/j.tsf.2018.07.001
    year: 2018
    role: milestone
    note: "Titanium nitride sputtered from room temperature to 600 °C and annealed: the temperatures a heater's film is shown to survive."
  - cite: "M. J. Madou, Fundamentals of Microfabrication and Nanotechnology, 3rd edition, CRC Press (2018)"
    title: "Fundamentals of Microfabrication and Nanotechnology"
    doi: 10.1201/9781315274164
    year: 2018
    role: review
    note: "A handbook of microfabrication, its tables of deposition techniques with their temperatures among them."
  - cite: "Y. Zhang, L. Shao, J. Yang, Z. Chen, K. Zhang, K.-M. Shum, D. Zhu, C. H. Chan, M. Lončar, C. Wang, Photon. Res. 10, 2380 (2022)"
    title: "Systematic investigation of millimeter-wave optic modulation performance in thin-film lithium niobate"
    doi: 10.1364/PRJ.468518
    year: 2022
    role: current
    note: "Thin-film lithium niobate modulators to millimetre waves, with their flow: ribs etched in the film, electrodes evaporated and lifted off."
  - cite: "F. Kaufmann, G. Finco, A. Maeder, R. Grange, Nanophotonics 12, 1601 (2023)"
    title: "Redeposition-free inductively-coupled plasma etching of lithium niobate for integrated photonics"
    doi: 10.1515/nanoph-2022-0676
    year: 2023
    role: current
    note: "Argon etching of lithium niobate without redeposited material: its walls, its selectivity over the mask and how uniform its depth is."
---

Five cross-sections of one wafer as a silicon strip is made on it, in the symbols the sections
below use: the film's thickness $t$, the buried oxide's $h$, the line's width $w$, and the
exposure's wavelength $\lambda$ and numerical aperture NA.

::diagram process

A chip is made by adding films to a wafer and removing them where a pattern says, one layer at a
time. Four operations do nearly all of it: lithography writes a pattern into a light- or
electron-sensitive film, the resist; an etch removes a film where the resist leaves it bare; a
deposition or an oxidation adds a film; a polish makes the surface flat again. photonoxide's
[fabrication engine](../docs/methods/fabrication.md) reads a design's layers and shapes and writes
a flow that makes it from published processes, each step with its reason and every number with
its source. The blocks below play those flows on the wafer's cross-section, frame by frame.

## 1. What is a wafer, and where does the thin film come from?

A silicon photonic chip starts from a silicon-on-insulator (SOI) wafer: a film of single-crystal
silicon on a layer of oxide, the buried oxide, over a thick silicon handle. The film is the
guides' core and the oxide keeps their light from the handle. [Selvaraja et al.
(2009)](https://doi.org/10.1109/JLT.2009.2022282) made their wires on 200 mm wafers with 220 nm of
silicon on 2 µm of oxide (Section II), the stack of the library's strip. The engine's rule: a
single-crystal film is bought with its wafer, never deposited, so every flow on SOI or on lithium
niobate begins with the wafer as bought.

The film is made by **Smart Cut** ([Bruel, 1995](https://doi.org/10.1049/el:19950805), Fig. 1), in
four steps:

1. Hydrogen is implanted into a silicon wafer, the donor, at $2\times10^{16}$ to
   $1\times10^{17}$ cm⁻², preferably through a thermal oxide that becomes the buried oxide.
2. The donor is bonded at room temperature to a second wafer, the handle.
3. Heated to 400–600 °C, the donor splits along the implanted plane, leaving a thin film on the
   handle; the rest of the donor is reused. Then above 1000 °C the bond is strengthened.
4. The split surface is micro-rough, and is polished.

Bruel's films vary by at most 4 nm after splitting and close to 8 nm after polishing, for a
200 nm film on a 10 cm wafer (p. 1202): the engine carries the 8 nm as the strip's thickness
uncertainty. Lithium niobate on insulator is made the same way with helium, bonded onto oxide on
a lithium niobate handle, split at 228 °C and annealed at 450 °C ([Poberaj et al.,
2012](https://doi.org/10.1002/lpor.201100035)), which is why no later step of a lithium niobate
flow may pass 450 °C (Section 8).

`jobs/strip-flow-smart-cut.yaml` makes the strip's SOI wafer from bare silicon instead of buying
it, thirteen steps where the bought wafer is one. For 220 nm on 2 µm the engine gives:

| Step | What the engine works out | Its source |
|---|---|---|
| the donor oxidized | 8.275 h in wet oxygen at 1100 °C for 2 µm of oxide, consuming 0.88 µm of silicon | Deal and Grove's Table I; 44 % of the oxide's thickness is consumed silicon (Campbell, p. 74) |
| hydrogen implanted | 318.7 keV, which leaves the oxide with 110.7 keV and puts the peak 0.9337 µm below it | NIST's PSTAR ranges (ICRU Report 49) |
| bonded, split, annealed | at room temperature, at 400–600 °C, above 1000 °C | Bruel, Fig. 1 |
| polished | 0.3569 µm off the damaged surface | the engine's rule, stated as such in the flow |
| a sacrificial oxide grown and stripped | 0.8111 µm in 1.465 h, consuming the second 0.3569 µm; dissolved in 6:1 HF in 6.759 min | Deal and Grove's Table I; about 1200 Å/min, oxide over silicon better than 100:1 (Campbell, p. 285) |

The last step leaves the wafer a flow would have bought: 0.22 µm of silicon on 2 µm of oxide.

### Where the split falls {theory}

The film's thickness is the depth at which the hydrogen comes to rest, set by the implant's
energy alone: Bruel found about 8 nm of film per keV in silicon. The range in silicon is
proportional to the energy only where the range's slope equals its ratio to the energy, where
that ratio is least. In PSTAR's table that is the 100 keV row, 842.1 nm over 100 keV, 8.42 nm/keV,
5.3 % above Bruel's 8 and within half a unit of his last digit:

::validation fabrication/smart-cut-range-bruel

The 318.7 keV above is high because the protons cross the 2 µm of buried oxide first and lose
208 keV in it ([the method's write-up](../docs/methods/fabrication.md)).

## 2. How does a pattern get from a drawing onto the wafer?

A **resist** is spun onto the wafer, exposed where the pattern says, and developed. In a positive
resist the exposure makes the resist dissolve faster, so the developer washes the exposed part
away; in a negative resist the unexposed part dissolves (Campbell, *Fabrication Engineering at
the Micro- and Nanoscale*, 3rd ed., Chapter 8, p. 199). What stays is the mask of the next step:
over a film to be etched, the resist is kept on the shapes; for a lift-off it is opened where the
metal goes (Section 6).

The exposure is by light through a mask or by an electron beam:

- **Projection lithography** images a reduced copy of the mask onto the resist, one field after
  another. For the strip the engine picks 193 nm (ArF) projection: Selvaraja et al. coated 77 nm
  of anti-reflective layer and 330 nm of resist (Section III), and a 450 nm wire printed 423 nm
  wide until a 27 nm bias was added on the mask (p. 4079). The engine carries their 4.5 nm, ±1 %
  of the wire's width (Fig. 7), as the line's width uncertainty.
- **Electron-beam lithography** writes the pattern point by point with no mask: slow, so it suits
  prototypes, and fine, at most 5 nm in the negative resist HSQ ([Chen,
  2015](https://doi.org/10.1016/j.mee.2015.02.042), Table 1). The beam writes one field at a time,
  and the fields meet to about 5 nm ([Bojko et al., 2011](https://doi.org/10.1116/1.3653266),
  PDF p. 6).
- **Nanoimprint** presses a mould into the resist: 25 nm features ([Chou et al.,
  1996](https://doi.org/10.1126/science.272.5258.85)).

The machines, cut through in 2D and built in 3D: the 193 nm scanner the strip's flow chooses
first, the others to compare. Each moves as it works: the scanners sweep a slit over one field
after another, the mask and the wafer moving opposite ways; the electron beam fills a strip and a
bend line by line, blanked between lines; the contact aligner aligns, then opens its shutter.

::lithography duv-193 duv-248 i-line duv-193i euv uv-photolithography ebl nanoimprint

The silicon strip, 500 nm by 220 nm, at the library's defaults. Step 2 plays in three frames:
the resist spun on, the pattern exposed, the resist developed and kept only on the strip.

::fabrication waveguide

### What the SOI papers did {research}

[Bogaerts et al. (2005)](https://doi.org/10.1109/JLT.2004.834471) made silicon wires on a CMOS
line and chose 248 nm lithography, "the mainstream fabrication tool for high-end CMOS" then
(Section IV, p. 405); their wires lost 0.24 dB/mm. Selvaraja et al. moved the same circuits to 193 nm.
The engine's strip flow contains, in order, the steps Selvaraja et al. report:

::validation fabrication/reported-flows

## 3. How small can a feature be, and what limits it?

A projection lens images a line down to about

$$
W = k_1 \frac{\lambda}{\mathrm{NA}} ,
$$

the Rayleigh criterion (Campbell, Eq. 7.18, p. 183), with $\lambda$ the light's wavelength, NA
the lens's numerical aperture and $k_1$ a factor of the resist and the process, "typically" 0.8
down to 0.4, its theoretical limit 0.25 (the same page). The chart draws $W$ for each projection
lithography of the library over the numerical apertures its sources cite. Moving $k_1$ moves every
curve; the figures give the $k_1$ the feature asks of each at its highest NA.

::chart fab-resolution

Besides $k_1$, the criterion leaves two levers. A larger NA: water between the last lens and the
wafer lets it pass 1, to 1.35 for today's water-immersion lenses ([Mack,
2007](https://doi.org/10.1002/9780470723876), Chapter 1, p. 22). A shorter $\lambda$:
extreme-ultraviolet light of 13.4 nm, which every material absorbs, so it is reflected off mirrors
coated with many alternating thin layers onto a reflective mask and then the resist, all in
vacuum; prototype step-and-scan cameras had NA 0.1 and resolved better than 100 nm ([Madou,
2018](https://doi.org/10.1201/9781315274164), Vol. II, Chapter 2, p. 96).

::lithography euv duv-193i duv-193

The engine measures each layer's narrowest line and gap and picks the first lithography whose
cited minimum feature reaches them. A design no lithography of the library reaches is refused,
not given the closest:

::validation fabrication/lithography-choice

Electron beams and imprint are not imaging through a lens, so they have no $k_1$: their limits
are the cited minimum features above.

**Try it.** At $k_1 = 0.4$, which lithographies print the 200 nm gap of the library's MZI and
directional coupler as a matter of course?

:::answer
Set the feature to 200 nm: every curve but the i-line's (365 nm, NA up to 0.6) lies below
200 nm at its highest NA. At 193 nm and NA 0.93 the gap asks $k_1 = 0.96$.
:::

### The k₁ a design asks {theory}

Solving the criterion for $k_1$ at the class's highest cited NA, $k_1 = W\,\mathrm{NA}/\lambda$.
The strip's 500 nm at 193 nm and NA 0.93 asks $k_1 = 2.409$: far above 0.8, so the line prints
comfortably. At or above 0.4 the engine passes the layer; between 0.25 and 0.4 it warns that
resolution enhancement is needed; below 0.25 it refuses the lithography. Every cited minimum
feature of the library lies above its class's limit:

::validation fabrication/min-features-above-limit

A finer line also costs depth of focus, and the wafer's topography alone can reach 2 µm unless it
is planarized (Campbell, p. 184): one reason for Section 7's polish.

## 4. Why does an etch stop where it does?

A dry etch drives ions from a plasma straight down onto the wafer while a reactive gas attacks
the film, so it cuts down rather than sideways. It attacks some films far faster than others:
its **selectivity** is the ratio of their rates. An etch through a film onto one it hardly attacks
stops there, an **etch stop**: the HBr/O₂ etch Selvaraja et al. used goes through the silicon and
stops on the buried oxide (Section IV, p. 4081), so the strip's height is the film's, known to
Bruel's 8 nm.

Where no film lies at the floor, the etch is **timed**, and its depth is only as good as the
etch's uniformity. The rib below is etched 130 nm into the 220 nm film, leaving a 90 nm slab:
the engine gives the depth to ± 6.5 nm, a textbook's 5 % of the depth (Franssila, p. 138), as no
source in hand states this etch's own. The lithium niobate modulator's ribs, etched 300 nm into
600 nm by argon ions, are timed as well: ± 20 nm at 300 V and under 5 nm at 600 V ([Kaufmann et
al., 2023](https://doi.org/10.1515/nanoph-2022-0676), PDF p. 4).

::fabrication rib-waveguide

::validation fabrication/etch-stop-or-timed

Narrow openings etch slower than wide ones once their aspect ratio passes about one, so small
gaps are under-etched while open areas are over-etched (Campbell, p. 298): a timed depth comes out
shallower in a coupler's gap than beside it.

### Over-etch and the walls {theory}

An etch to a stop runs 10–20 % longer than the film takes, so the film clears everywhere
([Franssila, 2010](https://doi.org/10.1002/9781119990413), pp. 138–139). The film below loses that
over-etch divided by the selectivity $S$:

$$
\Delta h = \frac{f\,t}{S} = \frac{0.2 \times 220\ \text{nm}}{30} = 1.467\ \text{nm}
$$

for the strip, at Franssila's 30:1 of silicon over oxide under a resist mask (p. 135).

The etch leaves walls at an angle $\theta$ from the wafer's plane; 82° for the HBr/O₂ etch
(Selvaraja et al., p. 4081). A wall $d$ deep at $\theta$ reaches $d\cot\theta$ further out at its
foot than a vertical one, so a line drawn at $\theta_0$ and etched at $\theta$ has its floor wider by

$$
\Delta w = 2d\,(\cot\theta - \cot\theta_0) = 2 \times 220\ \text{nm} \times \cot 82^\circ = 61.84\ \text{nm}
$$

for the strip's vertical walls, 36.54 nm for the rib's 130 nm. The flow warns of it and names the
design's `sidewall` parameter, which set to 82° draws what the etch makes.

## 5. How are films grown and deposited?

**Thermal oxidation** grows oxide from the silicon itself: oxygen or water diffuses through the
oxide already there and reacts at the silicon beneath, consuming silicon 44 % as thick as the
oxide grown (Campbell, p. 74). It can only grow the first oxide on bare silicon, so in a flow it
comes before any other film. Wet oxygen grows faster than dry. The chart is Deal and Grove's law
with their printed constants:

::chart fab-oxidation

**Chemical vapour deposition** lays a film over whatever is there from gases that react on the
hot wafer. Low-pressure CVD (LPCVD) runs at 500–900 °C ([Madou,
2018](https://doi.org/10.1201/9781315274164), Vol. II, Table 7.19); its reaction happens at the
surface, so it coats tops, walls and floors alike, 100 % step coverage, conformal (Franssila,
p. 57). Plasma-enhanced CVD (PECVD) runs at 100–350 °C (Madou, Table 7.19) and covers a step
25–50 % (Franssila, p. 57). **Sputtering** knocks atoms off a target onto the wafer;
**evaporation** boils them off a source, and they fly in straight lines (Section 6).

**Try it.** At which temperature does wet oxygen grow 2 µm in about 8 hours?

:::answer
Near 1100 °C: the chart's wet figure at 8 h reads close to 2 µm there, and the Smart Cut flow
of Section 1 takes 8.275 h at 1100 °C for 2 µm.
:::

### Deal and Grove's law {theory}

[Deal and Grove (1965)](https://doi.org/10.1063/1.1713945) take three fluxes of oxidant in
series and equal in the steady state: from the gas to the oxide's surface, $F_1 = h(C^* - C_o)$;
through the oxide $x_0$ thick, $F_2 = D_\text{eff}(C_o - C_i)/x_0$; and the reaction at the silicon,
$F_3 = kC_i$. With $N_1$ molecules of oxidant in each unit volume of oxide, the oxide grows as
$dx_0/dt = F_3/N_1$. Integrating from an oxide $x_i$ at $t = 0$ (their Eqs. 11–12),

$$
x_0^2 + A x_0 = B(t + \tau), \qquad A = 2D_\text{eff}\left(\frac{1}{k} + \frac{1}{h}\right), \quad B = \frac{2D_\text{eff}C^*}{N_1}, \quad \tau = \frac{x_i^2 + A x_i}{B} .
$$

For $t + \tau \ll A^2/4B$ it is linear, $x_0 \approx (B/A)(t + \tau)$, the reaction limiting; for
$t \gg A^2/4B$ parabolic, $x_0^2 \approx Bt$, the diffusion limiting. $B/A$ is the linear rate
constant and $B$ the parabolic one (Campbell, p. 77); the chart draws both limits dashed and gives
$A^2/4B$ in its figures. Their printed constants hold together to the rounding of the tables, and
the law with their 920 °C wet row gives Campbell's Example 4.1, two hours of steam over 1000 Å of
oxide, 0.48 µm (p. 80), as 0.4770 µm:

::validation fabrication/deal-grove-tables fabrication/deal-grove-campbell

## 6. When is a metal lifted off instead of etched?

A **lift-off** is an etch's reverse. The resist is opened where the metal goes, the metal is
evaporated over everything, and dissolving the resist carries away the metal on it, leaving the
metal that landed in the openings. It needs the metal's step coverage to be poor, so the metal on
the resist and the metal in the openings don't join: evaporation's straight-line arrival gives
that, which "effectively prevents the use of sputtering" (Campbell, Section 11.9, p. 311). It
also needs a smooth or planarized surface, and the metal that floats off can land back on the
wafer (the same page). So the engine lifts off an evaporated metal and etches a sputtered one.

The thin-film lithium niobate modulator shows both kinds of patterning. Its ribs are written in
HSQ by the electron beam and etched by argon ions, timed to 300 nm. Its electrodes, copper under
gold, a 20 µm signal line between grounds 5 µm away, are opened in a resist by UV contact
lithography (about 0.5 µm with common sources: Campbell, Section 7.5, p. 180), evaporated 0.83 µm
thick and lifted off, as [Zhang et al. (2022)](https://doi.org/10.1364/PRJ.468518) made them
(Section 3.A).

::fabrication tfln-modulator

### The rings' and the modulator's reported flows {research}

[M. Zhang et al. (2017)](https://doi.org/10.1364/OPTICA.4.001536) wrote lithium niobate rings in
HSQ and etched them with argon ions to an intrinsic Q of ten million. The engine's modulator flow
contains, in order, both their steps and Y. Zhang et al.'s, and its warning names what the papers
leave open: the design's 70° walls lie outside argon milling's cited 60–63° (Kaufmann et al.,
PDF p. 3) and 63° (Poberaj et al.).

## 7. Why is a wafer polished flat?

A film deposited over an etched strip follows its step. A layer patterned on top of it would be
exposed on a slope, out of focus where the topography exceeds the lens's depth of focus (Section
3), and a lift-off needs a flat surface (Section 6). **Chemical-mechanical polishing** (CMP)
grinds the wafer flat under a pad with a slurry that also attacks it chemically ([Krishnan et
al., 2009](https://doi.org/10.1021/cr900170z)). It has no end point to sense, so a process needs a
high selectivity or very reproducible removal rates (Campbell, Section 11.2, p. 291).

The TiN heater's flow below shows it: the oxide under the heater is deposited 0.22 µm thicker
than its 2 µm, the height of the strip's step, and the polish (step 6) takes the excess off. The
polish leaves 20–30 nm of nonplanarity (Krishnan et al., p. 183) and removes its 0.22 µm to 10 %
(Franssila, Table 16.1), so the engine gives the oxide under the heater as 2000 nm ± 37.2 nm
(Section 9).

## 8. Why does the order of the steps matter?

Each film already on the wafer limits what may come after it: its **thermal budget**, the hottest
it is shown to survive. Silicon and its oxide take 1300 °C (the top of Deal and Grove's range),
titanium nitride 600 °C ([Kearney et al., 2018](https://doi.org/10.1016/j.tsf.2018.07.001)),
lithium niobate 450 °C (Poberaj et al.), aluminium on silicon its eutectic, 577 °C (Campbell,
Fig. 15.24, p. 454). The engine refuses a step that passes a cited limit of a film already there,
and among the steps left prefers one every film is shown to survive. Three orders follow:

- Oxidation and the wafer's anneals, above 1000 °C, come first, before any film that can't take
  them.
- Over the heater's TiN, LPCVD (to 900 °C) is refused and PECVD (to 350 °C) is chosen though it
  covers a step only 25–50 %. Under the TiN, before it exists, the conformal LPCVD fills the
  strip's surroundings.
- Over lithium niobate, the cladding is PECVD (Section 6's flow, step 8).

::fabrication thermo-optic-shifter

::validation fabrication/thermal-order

The heater's flow is the longest of the three, twelve steps: the strip's five, then the polish
(Section 7), the TiN sputtered and etched (not lifted off: Section 6), its cover, and its
contacts, listed but not drawn, since the design draws the heater alone.

## 9. What does the engine check, and what does it not claim?

**The round trip.** Every step has a geometry operation, run on the library's own structure; the
result after the last step is compared with the design, layer by layer and at 48 × 48 points
through every layer. Each block above ends with it; for the strip, at its defaults and at the
corners of its parameters, no point differs:

::validation fabrication/round-trip-waveguide

**How accurately each dimension is made.** For each dimension the engine lists what each step
contributes, with its source: the wafer's thickness spread, the lithography's linewidth, the
walls' angle carried down the etch, a timed etch's depth, the polish's flatness, the
over-etch's loss. Where no source states a contribution, it says "not specified", and the true
spread is then larger than the stated part. For the strip:

| Dimension | Design | Made to | Its contributions |
|---|---|---|---|
| the film's thickness | 220 nm | ± 8 nm | the wafer (Bruel) |
| the strip's width | 500 nm | ± 4.5 nm | the lithography (Selvaraja et al.); the etch's change of it not specified |
| its width at the floor | 500 nm | ± 62.0 nm | the 4.5 nm and the walls' 61.84 nm |
| the buried oxide beside it | 2000 nm | ± 1.467 nm | the over-etch (Section 4) |

**What it does not claim.** A flow is one sound way to make a design from published processes,
not a foundry's process, and the Fabrication page says so on every flow. The engine doesn't
simulate a process (no etch profile is computed, no aerial image formed), gives no recipes, doses
or yields, and doesn't predict how a fabricated device performs: a dimension's spread is read from
its sources, not from a model of the tool.

### Combining the contributions {theory}

Each contribution is read as a bound either way. Independent contributions $T_i$ add as the root
of the sum of their squares, as Campbell sums the components of an overlay (Eq. 7.27, p. 194):

$$
T = \Big(\sum_i T_i^2\Big)^{1/2} ,
$$

and the worst case, every contribution at its bound at once, is their sum. The strip's floor:
$\sqrt{4.5^2 + 61.84^2} = 62.0$ nm, at worst 66.34 nm. The heater's polished oxide:
$\sqrt{30^2 + 22^2} = 37.2$ nm, at worst 52 nm:

::validation fabrication/accuracy-combined

## History {research}

::timeline

Deal and Grove's law of 1965 described the oxidation every silicon process has used since, the
step that grows the buried oxide of Smart Cut ([Bruel, 1995](https://doi.org/10.1049/el:19950805)),
which made thin single-crystal films on oxide a product. Lithography came from the electronics
industry: Bogaerts et al. (2005) and Selvaraja et al. (2009) showed photonic wires made on CMOS
lines with 248 nm and 193 nm lithography, while electron beams (Bojko et al., 2011; Chen, 2015)
and imprint (Chou et al., 1996) write finer features for prototypes. Ion slicing brought the same
route to lithium niobate (Poberaj et al., 2012), whose etched ribs (M. Zhang et al., 2017; Y. Zhang
et al., 2022; Kaufmann et al., 2023) now carry modulators to millimetre waves.

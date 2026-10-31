---
title: "Fabrication: the process flow of a design"
module: fabrication
summary: "How a design would be made, read from its own layers and shapes: the steps in order from published processes, each with its reason in plain words and the alternatives with why they weren't chosen, every capability a range with the paper and the place in it, or 'not specified'. Rules read each layer with the films below and above it: a single-crystal film is a bought wafer, an etch stops on the film below where a source says it does and is timed otherwise, the lithography is the first whose cited minimum feature reaches the layer's narrowest line and gap (measured with the geometry kernel's offsets) and whose resist a source shows lasting the etch, an evaporated metal is lifted off and a sputtered one etched, a film over an etched step is polished flat before a patterned layer goes on it, and no step may pass a film's cited thermal limit. Each step's geometry operation runs on the library's own structure, and the flow gives back the design to the bit: the SOI strip, the TFLN modulator and the TiN heater at their defaults and corners, with zero differing points."
order: 27
papers:
  - cite: "M. Bruel, Electron. Lett. 31, 1201 (1995) (Smart Cut: Step 3, p. 1201; the film's spread, p. 1202)"
    doi: 10.1049/el:19950805
  - cite: "S. K. Selvaraja, P. Jaenen, W. Bogaerts, D. Van Thourhout, P. Dumon, R. Baets, J. Lightwave Technol. 27, 4076 (2009) (193 nm lithography and the HBr/O₂ etch, Sections II–IV)"
    doi: 10.1109/JLT.2009.2022282
  - cite: "R. J. Bojko, J. Li, L. He, T. Baehr-Jones, M. Hochberg, Y. Aida, J. Vac. Sci. Technol. B 29, 06F309 (2011) (electron-beam lithography in HSQ and the Cl₂ etch)"
    doi: 10.1116/1.3653266
  - cite: "Y. Chen, Microelectron. Eng. 135, 57 (2015) (Table 1: electron-beam resists)"
    doi: 10.1016/j.mee.2015.02.042
  - cite: "G. Poberaj, H. Hu, W. Sohler, P. Günter, Laser Photonics Rev. 6, 488 (2012) (lithium niobate on insulator, its bonding, splitting and argon milling)"
    doi: 10.1002/lpor.201100035
  - cite: "F. Kaufmann, G. Finco, A. Maeder, R. Grange, Nanophotonics 12, 1601 (2023) (argon etching of lithium niobate: walls, selectivity, uniformity, redeposition)"
    doi: 10.1515/nanoph-2022-0676
  - cite: "Y. Zhang, L. Shao, J. Yang, Z. Chen, K. Zhang, K.-M. Shum, D. Zhu, C. H. Chan, M. Lončar, C. Wang, Photon. Res. 10, 2380 (2022) (the modulator's flow, Section 3.A)"
    doi: 10.1364/PRJ.468518
  - cite: "M. Zhang, C. Wang, R. Cheng, A. Shams-Ansari, M. Lončar, Optica 4, 1536 (2017) (lithium niobate rings' flow)"
    doi: 10.1364/OPTICA.4.001536
  - cite: "B. E. Deal, A. S. Grove, J. Appl. Phys. 36, 3770 (1965) (thermal oxidation: Eqs. 12–15, Tables I and II)"
    doi: 10.1063/1.1713945
  - cite: "M. Krishnan, J. W. Nalaskowski, L. M. Cook, Chem. Rev. 110, 178 (2009) (chemical-mechanical planarization)"
    doi: 10.1021/cr900170z
  - cite: "M. J. Madou, Fundamentals of Microfabrication and Nanotechnology, 3rd ed., Vol. II, CRC Press (Table 7.19: deposition techniques)"
    doi: 10.1201/9781315274164
validation:
  - fabrication/round-trip-waveguide
  - fabrication/round-trip-tfln
  - fabrication/round-trip-heater
  - fabrication/capabilities-cited
  - fabrication/deal-grove-tables
  - fabrication/etch-stop-or-timed
  - fabrication/lithography-choice
  - fabrication/thermal-order
  - fabrication/reported-flows
---

# Fabrication: the process flow of a design

A design in photonoxide is a stack of layers and the shapes drawn on them, with each etch's
depth and sidewall angle. That is also what a cleanroom makes. `photonoxide::fabrication`
reads a design and says how it could be made: which steps, in which order, why each one, what
else could do it, and what each process can achieve, with the paper or book that says so. Then
it runs the steps on the library's own geometry and checks that they give the design back.

A flow is one sound way to make a design from published processes. It is a guide for learning
and for planning, alongside a foundry's own process and design rules, which a user's PDK brings
(roadmap 0.9).

## The words, for a first read

- **Wafer:** the disc everything is made on. Photonic chips start from a wafer bought with its
  first layers: a thin film of single-crystal silicon or lithium niobate on a layer of oxide,
  over a thicker silicon *handle*.
- **Film:** a layer of one material over the wafer. **Deposition** puts one down everywhere.
- **Resist:** a light- or electron-sensitive film. **Lithography** writes the pattern into it:
  light through a mask, or an electron beam point by point. The developer then washes away
  the written part (a *positive* resist) or the rest (a *negative* one).
- **Etch:** removing a film where the resist doesn't cover it. An etch that reaches a film it
  hardly attacks stops there: an **etch stop**. Without one, the etch is **timed**, and its
  depth is only as good as the etch's uniformity.
- **Sidewall angle:** the wall's angle from the wafer's plane that an etch leaves: 90° is
  vertical; less leans out going down, so a strip's cross-section is a trapezoid.
- **Selectivity:** how much faster an etch removes its film than something else (the resist,
  or the film below).
- **Lift-off:** the reverse of an etch: the resist is opened where a metal should go, the metal
  is evaporated over everything, and dissolving the resist takes away the metal on it.
- **Planarization (CMP):** polishing the wafer flat, so the next layer and its lithography sit on
  a flat surface.
- **Thermal budget:** the hottest a film already on the wafer may get. A step that goes hotter
  must come before that film, or not at all.

## The three first designs and their flows

Layers are listed from the top down, the substrate last, as a cross-section is drawn
(photonoxide's `LayerStack` lists them bottom to top).

### An SOI strip waveguide (`waveguide`, at its defaults)

| Layer | Film | Thickness | Drawn |
|---|---|---|---|
| cladding | SiO₂ | above everything | |
| Si | Si, SiO₂ where etched | 0.22 µm | a strip 0.5 µm wide, 10 µm long, vertical walls |
| BOX | SiO₂ | 2 µm | |
| substrate | Si | | |

| # | Step | Process | What it does, and why |
|---|---|---|---|
| 1 | wafer | Silicon on insulator, Smart Cut | Bought: a single-crystal film can't be deposited. 220 nm of Si on 2 µm of oxide is the wafer Selvaraja et al. used. |
| 2 | lithography | 193 nm projection | Resist kept on the strip. Its narrowest line, 0.5 µm, is above the 150 nm the source printed linearly; the SOI papers in hand used it. |
| 3 | pattern transfer | ICP etch in HBr/O₂ | Through the 0.22 µm of Si, stopping on the oxide below, which it etches far more slowly. |
| 4 | pattern transfer | Resist removal | |
| 5 | deposition | PECVD oxide | Fills the etched regions and is the cladding; at 100–350 °C, below what Si and SiO₂ are shown to survive (above 1000 °C). |

**Warning the flow gives:** the design's vertical walls are outside the cited 82° of both
silicon etches (Selvaraja et al.'s "8°" from the vertical, Bojko et al.'s 82°). The design's own
`sidewall` parameter can be set to 82° to draw what the etch makes.

### The thin-film lithium niobate modulator (`TFLN Mach-Zehnder modulator`, at its defaults)

| Layer | Film | Thickness | Drawn |
|---|---|---|---|
| cladding | SiO₂ | above everything | |
| metal | Cu/Au (drawn with the oxide's optics) | 0.83 µm | three electrodes: a 20 µm signal, 5 µm gaps, 100 µm grounds |
| LN | LiNbO₃, SiO₂ where etched | 0.6 µm | two rib arms 1.2 µm wide, etched 0.3 µm, walls at 70° |
| BOX | SiO₂ | 2 µm | |
| substrate | Si | | |

| # | Step | Process | What it does, and why |
|---|---|---|---|
| 1 | wafer | Lithium niobate on insulator, ion slicing and bonding | Bought, as Zhang et al. 2022's NanoLN wafer. |
| 2 | lithography | Electron beam in HSQ | Resist kept on the ribs. HSQ resolves 5 nm and below; its 500 nm, etched at a selectivity of 1, lasts the 0.3 µm etch. |
| 3 | pattern transfer | Argon-ion etching | Timed to 0.3 µm: nothing stops it in the film. Its depth varies by the cited 20 nm (300 V) or under 5 nm (600 V). |
| 4 | pattern transfer | Resist removal | |
| 5 | lithography | UV photolithography | Resist opened where the electrodes go. The narrowest feature, the 5 µm gap, is above contact printing's 0.5 µm. |
| 6 | deposition | Electron-beam evaporation | 0.83 µm of Cu/Au, in straight lines, as lift-off needs. |
| 7 | pattern transfer | Lift-off | Dissolving the resist leaves the electrodes. |
| 8 | deposition | PECVD oxide | Fills the etched regions and around the electrodes, and is the cladding. |

**Warnings the flow gives:** the design's 70° walls are outside argon milling's cited 60–63°
(Kaufmann et al.) and 63° (Poberaj et al.); the library's lithium niobate etches closest to 70°
are the fluorine etch after proton exchange (68°, Ulliac et al.) and the CHF₃/Ar etch (75°,
Krasnokutska et al.). The design draws its layers flat, so the electrodes' layer sits above the
film's top, where on the wafer the electrodes rest on the slab beside the ribs.

This flow contains, in order, the one Zhang et al. 2022 report for this modulator (Section 3.A)
and the one M. Zhang et al. 2017 report for their rings.

### The TiN thermo-optic phase shifter (`thermo-optic phase shifter`, at its defaults)

| Layer | Film | Thickness | Drawn |
|---|---|---|---|
| cladding | SiO₂ | above everything | |
| cover | SiO₂ | 1 µm | |
| TiN | TiN (drawn with the oxide's optics) | 0.1 µm | a heater 7.5 µm wide, 320 µm long |
| gap | SiO₂ | 2 µm | |
| Si | Si, SiO₂ where etched | 0.22 µm | a strip 0.5 µm wide |
| BOX | SiO₂ | 2 µm | |
| substrate | Si | | |

| # | Step | Process | What it does, and why |
|---|---|---|---|
| 1–4 | | as the strip waveguide's 1–4 | |
| 5 | deposition | PECVD oxide | Fills the etched regions and lays down the 2 µm under the heater, deposited 0.22 µm thicker for the polish. |
| 6 | planarization | CMP of oxide | The oxide follows the strip's step, and the heater is patterned next: polished flat, to within the cited 20–30 nm. |
| 7 | deposition | Reactive sputtering of TiN | 0.1 µm over the whole wafer, at 25–600 °C (Kearney et al.). |
| 8 | lithography | UV photolithography | Resist kept on the heater, 7.5 µm wide. |
| 9 | pattern transfer | Dry etch of TiN | Its chemistry, walls and selectivity are not specified by the sources in hand. |
| 10 | pattern transfer | Resist removal | |
| 11 | deposition | PECVD oxide | The 1 µm cover and the cladding; TiN is shown to survive 600 °C. |
| 12 | (not drawn) | Contact metal and vias | The heater's contacts, which the design doesn't draw; Jacques et al. show them without naming the metal. |

Sputtered TiN is etched, not lifted off: lift-off relies on evaporation's poor step coverage,
which "effectively prevents the use of sputtering" (Campbell, Section 11.9). Lift-off is listed
as the alternative, with Ríos et al.'s 10 nm sputtered film lifted off.

## The process library

Each process is data: its name, what it does in plain words, the films it acts on, and its
capabilities, each a range with its source and the place in it, or "not specified". A number
can't be written without its source (the type `Stated` has no other way). Only numbers read in
the papers and books are in it. The library's version, 1, is recorded with every flow.

| Process | Capability | Value | Source |
|---|---|---|---|
| SOI wafer (Smart Cut) | film thickness spread | at most 4 nm split, 8 nm polished (200 nm film, 10 cm wafer) | Bruel 1995, p. 1202 |
| | temperature | 400 to 600 °C (splitting), above 1000 °C (bonding) | Bruel 1995, p. 1201 |
| LNOI wafer | temperature | 220 to 228 °C (splitting), 450 °C (anneal) | Poberaj et al. 2012, PDF pp. 3–4 |
| | surface roughness | 0.5 nm rms after polishing | Poberaj et al. 2012, Fig. 5 |
| Thermal oxidation | temperature | 700 to 1300 °C | Deal & Grove 1965, abstract |
| PECVD oxide | temperature | 100 to 350 °C | Madou, Vol. II, Table 7.19 |
| UV photolithography | minimum feature | 0.5 µm (0.1 µm demonstrated) | Campbell 2008, Section 7.5 |
| 193 nm lithography | minimum feature | 150 to 500 nm, linear | Selvaraja et al. 2009, p. 4078 |
| Electron beam, HSQ | minimum feature | at most 5 nm | Chen 2015, Table 1 |
| | stitching | 5 nm mean | Bojko et al. 2011, PDF p. 6 |
| Electron beam, PMMA | minimum feature | at most 5 nm | Chen 2015, Table 1 |
| Nanoimprint | minimum feature | 25 nm | Chou et al. 1996, abstract |
| ICP etch of Si, HBr/O₂ | sidewall | 82° | Selvaraja et al. 2009, p. 4081 |
| ICP etch of Si, Cl₂ | sidewall | 82° | Bojko et al. 2011, PDF p. 3 |
| Bosch deep etch of Si | selectivity | 250:1 over SiO₂, 50:1 over resist | Donnelly & Kornblit 2013, PDF p. 34 |
| Argon etch of LiNbO₃ | sidewall | 60 to 63°; 63° | Kaufmann et al. 2023, PDF p. 3; Poberaj et al. 2012, PDF p. 6 |
| | selectivity over the mask | 1:1 | Kaufmann et al. 2023, PDF p. 3 |
| | depth variation | 20 nm (300 V), at most 5 nm (600 V) | Kaufmann et al. 2023, PDF p. 4 |
| Fluorine etches of LiNbO₃ | sidewall | 68° (SF₆ RIE), 75° (CF₄/He ICP) | Ulliac et al. 2011, PDF p. 3 |
| CHF₃/Ar etch of LiNbO₃ | sidewall | 75° | Krasnokutska et al. 2018, PDF p. 3 |
| TiN sputtering | temperature | 25 to 600 °C | Kearney et al. 2018, abstract |
| CMP of oxide | nonplanarity left | 20 to 30 nm | Krishnan et al. 2009, p. 183 |
| Dry etch of TiN, lift-off, contact metal | the numbers the rules would use | not specified | |

The thermal budgets the rules check: Si and SiO₂ survive above 1000 °C (Bruel), LiNbO₃ 450 °C
(Poberaj et al.), TiN 600 °C (Kearney et al.); aluminium on silicon is limited by the eutectic
at 577 °C (Campbell, Fig. 15.24) and an amorphous phase-change film by its crystallization
temperature, from the material catalogue (Sb₂Se₃: 200 °C, Delaney et al. 2020).

## The rules

The generator reads the design layer by layer, from the bottom, with the films below and above
each feature. Rules, not search: the same design always gives the same flow.

- **The wafer.** Everything up to the first single-crystal film (silicon or lithium niobate)
  is bought as a bonded wafer.
- **The etch.** The one the platform's papers used, else the library's first. A full etch stops
  on the film below when a source says it does; otherwise, and for every partial etch, it is
  timed, and its depth is uncertain by the etch's cited depth variation, or "not specified". The
  design's sidewall is compared with every cited range, and one outside them all is a warning
  that names the closest etches.
- **The lithography.** Each layer's narrowest line and gap are measured with the geometry
  kernel's offsets: a line w wide disappears when the shapes are shrunk by w/2, a gap g wide
  closes when they are grown by g/2, found by bisection and given to 0.01 nm. Among the
  lithographies whose first cited minimum feature is at most that, the user's preference comes
  first, then the one the platform's papers used, then the library's order; the first whose
  resist a source shows lasting the etch (its thickness times its selectivity, or a depth a
  source etched with it) is chosen. When none resolves the feature, the design is refused: the
  rules don't pick the closest. Electron-beam lithography is always listed for prototypes.
- **Metals.** A metal deposited in straight lines (evaporated) is lifted off; one that isn't
  (sputtered) is deposited over everything and etched.
- **Depositions.** The film's first process that breaks no cited thermal limit of a film already
  there. Thermal oxidation can't lay a film over others: it grows from silicon, by consuming it.
  A film over an etched step with a patterned layer to come above it is deposited thicker by the
  step and polished flat.
- **What isn't drawn.** A heater's contacts are listed as a step, not drawn.

Each step carries its reason, its alternatives with why each wasn't chosen, the capabilities it
relies on with their sources, and its uncertainties; the flow carries its warnings and what no
source in hand states.

## Executing a flow, and the round trip

Every step has a geometry operation, and the executor runs them on the library's own
`Structure`: the wafer's layers; a deposition's layers on top, filling every region etched open
so far (that sets each etched layer's background); an etch drawing the mask's shapes on its
layer with its depth and sidewall; a lift-off drawing the openings on the layer it makes. The
result is compared with the design: the structures by `PartialEq`, the films layer by layer, and
the material and film at the points of a 48 × 48 grid over the shapes through every layer.

Where the optical structure draws one material in place of another, `Device::physical` names
the film that is there: the modulator's electrodes are copper under gold (Zhang et al. 2022),
the heater titanium nitride (Jacques et al. 2019), both drawn with the oxide's optics because the
modes are solved without them. The flow deposits the real film; put back, the stand-ins give
`Device::geometry`'s structure exactly.

## Validation

| Case | What | Ours | Rule |
|---|---|---|---|
| `fabrication/round-trip-waveguide` | the strip's flow gives it back, at its defaults and the 8 corners that draw | 0 differing points | exact |
| `fabrication/round-trip-tfln` | the modulator's, at its defaults and the 48 corners that draw | 0 differing points | exact |
| `fabrication/round-trip-heater` | the heater's, at its defaults and 64 corners | 0 differing points | exact |
| `fabrication/capabilities-cited` | every number and statement names a listed reference with a DOI or ISBN, and a place | 0 faults | exact |
| `fabrication/deal-grove-tables` | the printed B/A is B over A in every row of Tables I and II, to the printing's rounding | largest misfit 0.17 of the allowance | reference, at most 1 |
| `fabrication/etch-stop-or-timed` | a strip on oxide stops; a rib and the lithium niobate rib are timed | 0 wrong | exact |
| `fabrication/lithography-choice` | 150 nm takes 193 nm lithography, 149 nm and 5 nm the electron beam, 4.9 nm is refused | 0 wrong | exact |
| `fabrication/thermal-order` | thermal oxidation refused over aluminium, PECVD not; PECVD refused over amorphous Sb₂Se₃; TiN survives PECVD | 0 wrong | exact |
| `fabrication/reported-flows` | the flows contain Selvaraja et al.'s, Zhang et al. 2017's and 2022's sequences | 0 missing | exact |

The round trip proves that a flow makes the design's layers and shapes, no more and no less. The
capabilities' sources and the rules' cases are what speak for the steps themselves.

## What isn't specified yet

These are listed in every flow that meets them, and wait for a source:

- titanium nitride's dry etch (its chemistry, walls and selectivity);
- the heater's contact metal and vias;
- the resist-to-metal thickness ratio lift-off needs, and its narrowest line;
- the HBr/O₂ etch's selectivity over the buried oxide as a number (the source states that it
  stops there);
- the evaporation's temperature, and so copper and gold's thermal budget.

## In the studio

A component's page has a **Fabrication** tab, opened by the **Fabricate** button beside Run. It
shows the flow for the design as it stands, read-only: the steps in order, and the wafer's
cross-section at the design view's cut, animated step by step (`fabrication::stages`, air where
nothing is yet): a deposition revealed from below as the film grows, an etch, a lift-off, a
polish or the resist's removal from above as material goes, a lithography faded in; play, pause,
step back and forward, a speed, and no motion under `prefers-reduced-motion`. Each medium is named
by the film that is there, so the heater reads TiN and the electrodes Cu/Au, drawn in a metal's
colour, though the optical model draws them as oxide; a table lists the layers from the top down,
the substrate last. The **Fabricate** button also stands beside each published result that
computes a library device (Validation's Published results) and beside each device a lesson
shows. For the step shown:
what the process does, why it was chosen, what else could do it, what it can achieve with each
number's source on hover, and what isn't certain, with the glossary's definitions of the words
it uses, each following a textbook's or paper's section (`fabrication::glossary`); then the
warnings, what no source states, the round trip and the references. A scene draws an etched layer in bands, its
sloped walls a staircase; the design view's cross-section and the flow's stages draw each of its
shapes exactly instead, a trapezoid from its top face down to its etch's floor
(`job::scene_etched`), over the bands' slab.

## As a job

```yaml
device:
  kind: TFLN Mach-Zehnder modulator

task:
  kind: fabrication
  prefer: [ebl-hsq]   # process ids to prefer where several qualify
  avoid: [bosch-si]   # process ids not to use
```

`job::check` refuses a process the library hasn't, one both preferred and avoided, and a
preference that makes no step of the design. The run records the design's scene and the flow
(`Event::Flow`), with the library's version and the round trip. A flow is set by the design and
the library alone, so the record replays bit for bit, the same on any number of threads.

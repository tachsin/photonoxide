---
title: "Fabrication: the process flow of a design"
module: fabrication
summary: "How a design would be made, read from its own layers and shapes: the steps in order from published processes, each with its reason in plain words and the alternatives with why they weren't chosen, every capability a range with the paper and the place in it, or 'not specified'. Rules read each layer with the films below and above it: a single-crystal film is a bought wafer, an etch stops on the film below where a source says it does and is timed otherwise, the lithography is the first whose cited minimum feature reaches the layer's narrowest line and gap (measured with the geometry kernel's offsets) and whose resist a source shows lasting the etch, an evaporated metal is lifted off and a sputtered one etched, a film over an etched step is polished flat before a patterned layer goes on it, and no step may pass a film's cited thermal limit. The rule engine reads each projection lithography's resolution by the Rayleigh criterion with its cited wavelength, numerical aperture and k₁, ends an etch on the film below by a cited etch stop or selectivity with the over-etch's loss, warns of RIE lag and of walls no cited etch makes, prefers a conformal film to fill etched openings and a cooler one over a film whose budget is not shown, checks each layer's registration against the tool's cited overlay, ranks each step's alternatives by what they gain and cost, and gives the accuracy of each fabricated dimension, its contributions combined as the root of the sum of their squares. Each step's geometry operation runs on the library's own structure, and the flow gives back the design to the bit: 15 of the library's 18 devices at their defaults and each parameter's ends, with zero differing points."
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
  - cite: "M. J. Madou, Fundamentals of Microfabrication and Nanotechnology, 3rd ed., Vol. II, CRC Press (Table 7.19: deposition techniques; Tables 3.13–3.14: plasma etchants and etch ratios; pp. 81, 96, 104: lithography)"
    doi: 10.1201/9781315274164
  - cite: "S. Franssila, Introduction to Microfabrication, 2nd ed., Wiley (2010) (Table 10.1: linewidth scaling; Sections 5.8, 11.7, 11.12 and 21.7: step coverage, resist selectivity, over-etch, RIE lag; Table 16.1: CMP)"
    doi: 10.1002/9781119990413
  - cite: "C. Mack, Fundamental Principles of Optical Lithography, Wiley (2007) (p. 22: numerical apertures; Eq. 2.83: the smallest half-pitch)"
    doi: 10.1002/9780470723876
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
  - fabrication/rayleigh-franssila
  - fabrication/deal-grove-campbell
  - fabrication/min-features-above-limit
  - fabrication/rule-choices
  - fabrication/accuracy-combined
  - fabrication/round-trip-library
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
| 2 | lithography | 193 nm projection | Resist kept on the strip. Its narrowest line, 0.5 µm, is above the 150 nm the source printed linearly; the SOI papers in hand used it. At NA 0.93 the line asks for k₁ = 2.4, far above the typical 0.4. |
| 3 | pattern transfer | ICP etch in HBr/O₂ | Through the 0.22 µm of Si, stopping on the oxide below, which it etches far more slowly: the over-etch takes at most 1.5 nm of it. |
| 4 | pattern transfer | Resist removal | |
| 5 | deposition | LPCVD oxide | Fills the etched regions and is the cladding. Conformal (100 % step coverage), where PECVD's is 25–50 %; at 500–900 °C, below what Si and SiO₂ are shown to survive (above 1000 °C). |

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
| 5 | deposition | LPCVD oxide | Fills the etched regions, conformally, and lays down the 2 µm under the heater, deposited 0.22 µm thicker for the polish. |
| 6 | planarization | CMP of oxide | The oxide follows the strip's step, and the heater is patterned next: polished flat, to within the cited 20–30 nm. |
| 7 | deposition | Reactive sputtering of TiN | 0.1 µm over the whole wafer, at 25–600 °C (Kearney et al.). |
| 8 | lithography | UV photolithography | Resist kept on the heater, 7.5 µm wide. |
| 9 | pattern transfer | Dry etch of TiN | Its chemistry, walls and selectivity are not specified by the sources in hand. |
| 10 | pattern transfer | Resist removal | |
| 11 | deposition | PECVD oxide | The 1 µm cover and the cladding. TiN is shown to survive 600 °C, PECVD's 350 °C; LPCVD's 900 °C it is not shown to survive, so PECVD is chosen though it isn't conformal. |
| 12 | (not drawn) | Contact metal and vias | The heater's contacts, which the design doesn't draw; Jacques et al. show them without naming the metal. |

Sputtered TiN is etched, not lifted off: lift-off relies on evaporation's poor step coverage,
which "effectively prevents the use of sputtering" (Campbell, Section 11.9). Lift-off is listed
as the alternative, with Ríos et al.'s 10 nm sputtered film lifted off.

## The process library

Each process is data: its name, what it does in plain words, the films it acts on, and its
capabilities, each a range with its source and the place in it, or "not specified". A number
can't be written without its source (the type `Stated` has no other way). Only numbers read in
the papers and books are in it. The library's version, 2, is recorded with every flow; it has 36
processes citing 24 sources.

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
| | uniformity across the wafer | 10 % of what it removes | Franssila 2010, Table 16.1 (p. 182) |
| i-line (365 nm) projection | minimum feature | 350 nm (production); 0.5 µm at NA 0.48, k₁ 0.6 | Madou, Vol. II, p. 81; Franssila 2010, Table 10.1 |
| | numerical aperture | 0.4 to 0.6 | Campbell 2008, p. 183 |
| 248 nm projection | minimum feature | 90 to 150 nm | Campbell 2008, Table 7.4 (p. 189) |
| | numerical aperture; overlay | 0.5 to 0.82; 8 to 25 nm | Campbell 2008, Table 7.4 |
| 193 nm projection | numerical aperture | 0.6 (the photonic wires); 0.5 to 0.92 (2006 tools); 0.93 | Selvaraja et al. 2009, Fig. 5; Campbell 2008, Table 7.4; Mack 2007, p. 22 |
| | overlay | 12 nm; 8 to 12 nm | Selvaraja et al. 2009, p. 4076; Campbell 2008, Table 7.4 |
| | linewidth control | 4.5 nm on a 450 nm wire | Selvaraja et al. 2009, Fig. 7 |
| 193 nm immersion | minimum feature; overlay | 45 nm; 7 nm | Campbell 2008, Table 7.4 |
| | numerical aperture | 0.75 to 1.3; 1.35 | Campbell 2008, Table 7.4; Mack 2007, p. 22 |
| EUV (13.4 nm) | minimum feature; numerical aperture | better than 100 nm; 0.1 (prototypes) | Madou, Vol. II, p. 96 |
| ICP etch of Si, HBr/O₂ | selectivity over SiO₂ | 30:1 under resist, up to 300:1 under an oxide hard mask (gate etching) | Franssila 2010, p. 135 |
| ICP etch of Si, Cl₂ | selectivity over SiO₂; over resist | 200:1 (low-power high-density plasma); 3 to 5:1 | Campbell 2008, p. 310; Franssila 2010, p. 134 |
| Bosch deep etch of Si | aspect ratio | 10:1 easily, 40:1 with effort | Franssila 2010, p. 208 |
| Oxide etch in fluorocarbons | selectivity over resist; over Si | 10:1 (5 to 10:1 typical); 30:1 | Madou, Vol. II, Table 3.14; Franssila 2010, p. 134 |
| Nitride etch in fluorine | selectivity over resist; over SiO₂ | 10:1; 50:1 | Madou, Vol. II, Table 3.14 |
| Aluminium and GaAs etches in chlorine | selectivity over resist | 3 to 5:1 | Franssila 2010, p. 134 |
| Argon ion milling (metals) | selectivity over resist | 1 to 5:1 (metals 20–100 nm/min, resist about 20) | Franssila 2010, p. 138 |
| Medium-temperature LPCVD (oxide, nitride) | temperature; step coverage | 500 to 900 °C; 100 % (conformal) | Madou, Vol. II, Table 7.19; Franssila 2010, p. 57 |
| | nitride's thickness | at most 200 nm (thicker cracks) | Campbell 2008, p. 373 |
| Low-temperature LPCVD oxide; PECVD nitride | temperature | 300 to 500 °C; 100 to 350 °C | Madou, Vol. II, Table 7.19 |
| PECVD oxide | step coverage | 25 to 50 % | Franssila 2010, p. 57 |
| Wet etch of AlN in KOH | its walls, its selectivity | not specified (the source states that KOH etches it) | Franssila 2010, p. 135 |
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

## The rule engine: what it reads and decides

Beyond choosing the steps, the rules read the physics each step rests on. Every number below is
the library's, with its source; where no source gives one, the engine says "not specified".

- **Resolution, by the Rayleigh criterion.** A projection lithography prints lines down to about
  W = k₁λ/NA (Campbell, Eq. 7.18; Franssila, Eq. 10.1): λ its light's wavelength, NA its lens's
  numerical aperture, and k₁ a factor of the resist and the process, "typically" 0.8 down to
  0.4, with a theoretical limit of 0.25 (Campbell, p. 183; Mack's smallest half-pitch with tilted
  illumination, Eq. 2.83). For each layer the engine works out the k₁ its narrowest line or gap
  asks of the lithography at its highest cited NA. At or above 0.4 the line prints as a matter
  of course; between 0.25 and 0.4 only with resolution enhancement, which the flow warns of; below
  0.25 the lithography is refused. A lithography the platform's papers used still comes first
  among those that resolve the layer.

  | Lithography | λ (nm) | NA | Prints as a matter of course (k₁ 0.4) | At the limit (k₁ 0.25) |
  |---|---|---|---|---|
  | i-line | 365 | 0.4 to 0.6 | 243 nm | 152 nm |
  | 248 nm | 248 | 0.5 to 0.82 | 121 nm | 75.6 nm |
  | 193 nm | 193 | 0.5 to 0.93 | 83.0 nm | 51.9 nm |
  | 193 nm immersion | 193 | 0.75 to 1.35 | 57.2 nm | 35.7 nm |
  | EUV | 13.4 | 0.1 | 53.6 nm | 33.5 nm |

  Each class's cited minimum feature lies above its limit (`fabrication/min-features-above-limit`):
  193 nm immersion's 45 nm, for example, asks for k₁ = 0.31.
- **Pattern transfer by material.** Silicon is etched in HBr/O₂ or Cl₂ (or the Bosch process for
  deep etches); lithium niobate by argon milling or fluorine etches after proton exchange; oxide in
  fluorocarbons (CF₄/H₂, C₂F₆, C₃F₈, CHF₃/O₂) and nitride in fluorine (Campbell, Table 11.3; Madou,
  Table 3.13); aluminium and gallium arsenide in chlorine, since their fluorides don't evaporate
  and their chlorides do (Franssila, pp. 133–134); aluminium nitride in KOH (Franssila, p. 135);
  metals by argon ion milling, or by lift-off. The design's wall angle picks the etch whose cited
  walls include it, before the platform's choice.
- **Etch stops and selectivity.** An etch through a film ends on the film below when a source
  says it stops there, or when a source gives its selectivity over it. The etch is run 10–20 %
  longer to clear the film everywhere (Franssila, pp. 138–139), and the film below loses that
  over-etch divided by the selectivity: 20 % of 220 nm of silicon at 30:1 is 1.5 nm of buried
  oxide. Without either, the etch is timed. The mask must outlast the etch: its thickness times
  its selectivity against the depth, or the depth a source etched with it; otherwise the flow
  says how thick a resist the etch needs, or that it needs a hard mask (chromium or nickel for
  chlorine, oxide for silicon gate etching, oxide or aluminium for deep etches; Franssila, p. 135).
- **Aspect ratio.** Openings of 1:1 etch straightforwardly; around 2:1 narrow ones start to etch
  slower than wide ones, RIE lag (Franssila, Section 21.7.1): a timed depth comes out shallower
  there, and a stop needs a longer over-etch. The Bosch process reaches 10:1 easily and 40:1 with
  effort (Franssila, p. 208).
- **Walls.** A design's walls outside every cited angle of its etch are a warning, with what the
  etch would make instead: two walls d deep at θ instead of the drawn θ₀ move the line's floor by
  2d(cot θ − cot θ₀). The strip's vertical walls against the HBr etch's 82° make its floor
  61.8 nm wider.
- **Thermal budget and ordering.** A deposition whose temperature passes a cited limit of a film
  already there is refused; among those left, one every film is shown to survive comes before
  one that isn't, and filling etched openings, a conformal one comes first. So the SOI strip is
  clad by LPCVD oxide (500–900 °C, conformal), the lithium niobate and the TiN heater's cover by
  PECVD (100–350 °C), and medium-temperature LPCVD (to 900 °C) is refused over aluminium on
  silicon (its eutectic, 577 °C), where PECVD nitride (to 350 °C) is not.
- **Planarization.** A film over an etched step, with a patterned layer to come above it, is
  deposited thicker and polished flat. A layer patterned over an unpolished step is flagged:
  lift-off needs a smooth or planarized surface (Campbell, Section 11.9).
- **Film thickness.** A film thicker than its process makes is flagged: LPCVD nitride over about
  200 nm cracks under its own stress (Campbell, p. 373).
- **Registration.** Each patterned layer's edges are measured against those of the patterned
  layer below, at the cut through the middle of the design, and compared with its lithography's
  cited overlay (193 nm: 12 nm; immersion: 7 nm; 248 nm: 8–25 nm; electron beam: 5 nm field
  stitching). Registration should also be no more than a quarter to a third of the resolution
  (Campbell, p. 194). The TiN heater's edges sit 3.5 µm from the guide's; the modulator's
  electrodes 1.9 µm from the ribs'; a contact aligner's overlay is not specified (manual aligners
  are "highly operator dependent", Campbell, p. 168).
- **Alternatives, ranked.** Each step lists the chosen process first, then the others that could
  do it, best first: what each would gain or cost against the choice (finer or coarser lines,
  tighter or looser overlay, walls closer to or further from the design's, cooler or hotter,
  conformal or not), and those that can't do it last, with why.

## How accurately each dimension is made

For each dimension the flow makes, the engine lists what its processes contribute to its
uncertainty, each with its source, or "not specified":

- a wafer film's thickness: the wafer's stated thickness spread;
- a line's width: the lithography's linewidth control and the etch's change of it;
- a line's width at the etch's floor: those, and the walls' cited angles carried down the etch;
- a timed etch's depth: the etch's stated depth variation, or a textbook's example of 5 % of the
  depth where the etch's own isn't stated (Franssila, p. 138);
- the walls' angle: the etch's cited angles against the design's;
- the film below a stopped etch: the over-etch's loss;
- a polished film's thickness: the polish's flatness (Krishnan et al.) and 10 % of what it
  removes (Franssila, Table 16.1);
- a layer's place against the layer below: its lithography's overlay.

Each stated variation is read as a bound either way. They are combined as the root of the sum of
their squares, as the total of independent components is (Campbell, Eq. 7.27, p. 194), and the
worst case, all at their bounds at once, is their sum (as Franssila, p. 135, adds up an etch's
loss of the film below). Where a contribution is not specified, the read-out says so: the true
spread is then larger than the stated part.

| Design | Dimension | Design value | Made to | At worst | Not specified |
|---|---|---|---|---|---|
| SOI strip | the Si film's thickness | 220 nm | ± 8 nm | ± 8 nm | |
| | the strip's width | 500 nm | ± 4.5 nm | ± 4.5 nm | the etch's linewidth change |
| | its width at the floor | 500 nm | ± 62.0 nm | ± 66.3 nm | the etch's linewidth change |
| | its walls | 90° | ± 8° | ± 8° | |
| | the buried oxide beside it | 2000 nm | ± 1.47 nm | ± 1.47 nm | |
| TFLN modulator | the LiNbO₃ film's thickness | 600 nm | | | the wafer's spread |
| | the rib's etch depth | 300 nm | ± 20 nm | ± 20 nm | |
| | its walls | 70° | ± 10° | ± 10° | |
| | the electrodes' clearance from the ribs | 1900 nm | | | the contact aligner's overlay |
| TiN heater | the polished oxide under the heater | 2000 nm | ± 37.2 nm | ± 52 nm | the deposition's spread |
| | the heater's etch depth | 100 nm | ± 5 nm | ± 5 nm | |
| | the heater's clearance from the guide | 3500 nm | | | the contact aligner's overlay |

The strip's floor is the case in point: its walls are drawn vertical, and the etch's cited 82°
puts two walls 220 nm deep at 8° from that, which moves the floor by 61.8 nm, far more than the
lithography's 4.5 nm.

## Thermal oxidation as a process model

`fabrication::oxidation` grows thermal oxide by Deal and Grove's law, x₀² + Ax₀ = B(t + τ), with
their printed constants (Tables I and II). Between the printed temperatures it carries the nearest
row's B and B/A by the Arrhenius law with their printed activation energies (B: 28.5 kcal/mol dry,
16.3 wet; B/A: 46.0 dry, 45.3 wet; Figs. 9 and 11), and τ from the 230 Å of oxide their dry data
extrapolate to (230 ± 30 Å). Its bound comes from the constants' stated precision: B to ±2 %, A to
±2 % at low temperatures and ±12 % at high (the larger taken everywhere). Campbell's Example 4.1,
two hours of steam at 920 °C over 1000 Å of oxide, gives 0.48 µm in the book and 0.4770 µm here
(`fabrication/deal-grove-campbell`). The example's τ is printed as 0.295 h where
(0.1² + 0.5 × 0.1)/0.203 = 0.2956 h: the book cuts the digit rather than rounding it, so the τ is
not a case of its own.

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
| `fabrication/rayleigh-franssila` | k₁λ/NA against Franssila's Table 10.1 at 436, 365 and 248 nm: 0.918, 0.456 and 0.248 µm against the printed 1, 0.5 and 0.25 µm | largest misfit 0.88 of the allowance | reference, at most 1 |
| `fabrication/deal-grove-campbell` | two hours of steam at 920 °C over 1000 Å of oxide (Campbell's Example 4.1) | 0.4770 µm against 0.48 µm | reference, ± 0.005 µm |
| `fabrication/min-features-above-limit` | every cited minimum feature of a projection lithography asks for k₁ ≥ 0.25 at its highest NA | 0 below | exact |
| `fabrication/rule-choices` | the Rayleigh classes, a stop by selectivity, LPCVD and PECVD by thermal budget and conformality, RIE lag, registration | 0 wrong | exact |
| `fabrication/accuracy-combined` | the heater's polished oxide: √(30² + 22²) nm | 37.2022 nm | round-off, 1e-12 relative |
| `fabrication/round-trip-library` | every library device the rules make, at its defaults and each parameter's ends (160 designs of 15 devices) | 0 differing points | exact |

One row of Franssila's Table 10.1 is left out of its case: at 248 nm, NA 0.65 and k₁ 0.5, k₁λ/NA
is 0.191 µm where the table prints 0.18 µm, 0.011 µm off, more than the printing's rounding
allows; the table says nothing that would explain it.

The rules refuse 40 of the 200 designs the library round trip tries, each with its reason: the
three phase-change shifters' 36 (no process in hand deposits a phase-change film; the platforms
of F6), and 4 with a guide of zero length, which draw no shape (a straight waveguide's, a rib's
and a coupler's length of 0, a bend's angle of 0).

The round trip proves that a flow makes the design's layers and shapes, no more and no less. The
capabilities' sources and the rules' cases are what speak for the steps themselves.

## What isn't specified yet

These are listed in every flow that meets them, and wait for a source:

- titanium nitride's dry etch (its chemistry, walls and selectivity);
- the heater's contact metal and vias;
- the resist-to-metal thickness ratio lift-off needs, and its narrowest line;
- the HBr/O₂ etch's selectivity over the buried oxide as a number (the source states that it
  stops there);
- the evaporation's temperature, and so copper and gold's thermal budget;
- a contact aligner's and an i-line stepper's overlay, and EUV's;
- the linewidth control of every lithography but 193 nm projection, and every etch's change of
  the linewidth;
- the thickness spread of lithium niobate on insulator, and of every deposited film;
- the walls of the oxide, nitride, aluminium, GaAs and AlN etches, and of argon milling of
  metals;
- the depth variation of every timed etch but argon milling of lithium niobate (the read-out
  uses a textbook's example, 5 %, in its place, and says so).

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
warnings, what no source states, the round trip and the references. For a projection
lithography the step also shows its optics (wavelength, numerical apertures, and the lines it
prints by the Rayleigh criterion); every step with alternatives shows them ranked, the chosen
first, each with what it would gain (+) or cost (−); and the rules' checks of the step. Under the
steps, a table gives how accurately each dimension is made (the design's value, ± the combined
contributions, and the worst case; hovering a row lists the contributions and their sources),
and a list gives what the rules flag. A scene draws an etched layer in bands, its
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
preference that makes no step of the design. The run records the design's scene, the flow
(`Event::Flow`), with the library's version and the round trip, and what the rules find in it
(`Event::FlowAssessment`: the checks, the ranked alternatives and the accuracy of each dimension). A flow is set by the design and
the library alone, so the record replays bit for bit, the same on any number of threads.

# A fabrication engine: the process flow of a design, suggested and animated

*A plan, 2026-10-30. I review it first; the work is tracked in #622.*

A design in photonoxide is a stack of layers and the shapes drawn on them, with each etch's depth
and sidewall angle. That is also what a cleanroom makes. This page plans an engine that reads a
design and answers how it would be made:

- which steps, in which order;
- why each step is the one chosen, and what else could do it;
- what each process can achieve, with the paper or book that says so.

It then shows the steps one by one as an animated cross-section, in words a newcomer can follow.

1. [Goal, non-goals and readers](#1-goal-non-goals-and-readers).
2. [The data model](#2-the-data-model): what the library has today, the process library, the
   flow generator and the geometry operations that execute a flow.
3. [Validation](#3-validation): why a flow can be trusted.
4. [The visualization](#4-the-visualization): the animation, the Fabrication tab, the Academy and
   the examples.
5. [The job format](#5-the-job-format): a `fabrication` task kind.
6. [Phases](#6-phases), each one pull request.
7. [Sources](#7-sources).
8. [Decisions](#8-decisions).
9. [Risks](#9-risks).
10. [Placement in the roadmap](#10-placement-in-the-roadmap).

**Rules this page follows.**

- Every paper and book is cited with its DOI or ISBN, checked on Crossref (Open Library for the two
  books Crossref doesn't hold) on 2026-10-30. The ones not yet in the papers folder are listed in
  its README, in the section "Wanted for the fabrication engine".
- No number appears without its source. This page quotes only numbers already read and recorded
  in the papers folder. Every capability the process library will hold is marked *to read* until
  its paper has been read, and the library stores no value for it until then.
- The engine itself follows the same rule: a capability without a source is shown as
  "not specified", never filled in from a guess.

## Summary

- **What it is.** A process library, a flow generator and an executor, in a new module
  `photonoxide::fabrication`.
  - The **process library** holds deposition, lithography, pattern transfer, planarization,
    anneal, doping, release and dicing. Each process has capabilities: minimum feature, overlay,
    selectivity, sidewall angle, uniformity, roughness, temperature and the materials it accepts.
    Each capability is a range with its source, or "not specified".
  - The **flow generator** reads a design layer by layer, with what lies below and above each
    feature. It measures feature sizes, gaps and aspect ratios, and checks materials and thermal
    budgets. It returns ordered steps, each with its reason, its alternatives and its
    uncertainties.
  - The **executor** runs a flow's geometry operations on the library's own `Structure`.
- **Why it can be trusted.** Three checks:
  - Executing the generated flow must give back the design's `Structure`, equal to the bit.
  - Every stated capability names its paper and the place in it.
  - The rules are tested on cases with textbook answers, and against the flows papers report
    for their own devices.
- **What it shows.** An animated cross-section per step: films grow, resist is exposed, etches cut
  down with the design's sidewall angle, lift-off takes the metal off with the resist. Each step
  has plain narration and a glossary. The animation lives in:
  - a **Fabrication tab**, opened by a **Fabricate** button on a component's page and in the run
    view;
  - the **Academy**, through a `::fabrication` directive and a lesson on how a chip is made;
  - the **examples listing**, as a button beside each example that builds a device.
- **How it runs.** A `fabrication` task kind beside a `[device]`, through the `TaskKind` registry
  (#596). Its flow is data and is checked by `job::check`. Its run replays bit for bit.
- **The first slice** is a read-only flow for three designs: an SOI strip waveguide, a thin-film
  lithium niobate modulator and a thermo-optic heater. More platforms follow: LPCVD silicon
  nitride, phase-change cells, aluminium nitride and gallium arsenide.
- **Eight phases**, F1 to F8 ([Section 6](#6-phases)). The first slice (F1 to F3) needs only what
  0.5.4 built, so it can land before 0.10, where the roadmap places the engine.

## 1. Goal, non-goals and readers

**The goal.** For any design photonoxide can draw, give a process flow that would make it:

- the steps in order;
- for each step, the reason in one or two plain sentences, the alternatives and why they weren't
  chosen;
- what the step can achieve, as a cited range;
- an animation of the wafer's cross-section through the steps.

The engine is *physically informed*. It reads the layer below and the layer above every feature,
its width, the gaps beside it and the etch's depth. From those it decides:

- whether an etch can stop on the layer underneath or must be timed;
- which lithography resolves the smallest gap;
- whether a metal goes on by lift-off or by etching;
- whether a step's temperature would damage a layer already there.

**Not goals.**

- **Not a foundry sign-off and not a PDK.** A flow is one sound way to make the design, with its
  reasons. It is not a foundry's process, it is not checked against a foundry's rules, and it
  doesn't certify that a design can be made anywhere. Foundry rules enter through 0.9's PDKs,
  which the user loads ([Section 8](#8-decisions), decision 2).
- **Not a process simulator.** No etch or deposition physics is solved and no lithography is
  imaged. Profiles are drawn from each process's cited sidewall angle and conformality. Etch
  and lithography proxies calibrated against measurements are 0.11's work (lithography and etch
  proxies), and a flow's uncertainties can feed 0.11's corners (F8).
- **No invented numbers.** A capability without a source says "not specified", and so does a
  step that depends on it. The engine still lists the step, says what is missing, and names the
  paper it is waiting for, as the material catalogue's `Coming` entries do.
- **No recipes.** Gas mixtures, powers, pressures and doses appear only as context quoted from a
  source, never as instructions.
- **No cost, yield or schedule estimates.**

**Who it is for.**

- **Students and designers new to fabrication.** They see how their own design would be made,
  step by step, in plain words, without a cleanroom course first.
- **Designers choosing a platform or a process.** They want to know which lithography can resolve
  their gap, whether a rib's partial etch needs a timed etch and how uncertain its depth is, and
  whether a heater's metal can be lifted off.
- **Teachers.** The Academy embeds the same animation beside a lesson's device.

## 2. The data model

### 2.1 What the library has today

The engine reads what every solver already reads.

- **`LayerStack`** (`src/stack.rs`) is a substrate, layers from bottom to top and a cladding
  above. Each `Layer` has a name, a thickness, its `material` where shapes are drawn and its
  `background` everywhere else. The background is where the layer was etched away and refilled,
  usually by the cladding. The presets are `LayerStack::soi`, `soi_220` and `nitride`.
- **`Structure`** is a stack, the shapes drawn on each layer, and bodies placed over them. Each
  shape is drawn with an **`Etch`** (`src/etch.rs`): `depth` is `None` through the layer (a
  strip) or a depth below its top (a rib on its slab), and `sidewall` is the wall's angle from
  the layer's plane. A sloped wall leans out going down, so a strip's cross-section is a
  trapezoid whose drawn outline is its top face. `Structure` derives `PartialEq`, so two
  structures can be compared exactly.
- **Devices** (`src/device.rs`, `src/device/`) give `stack(values)` and `geometry(values)`, a
  `Geometry` with the `Structure` and the ports. Every device draws through the generators
  (`crate::generator::Drawing`), so its shapes and etches are known exactly. Examples:
  - the TFLN modulator (`src/device/tfln.rs`) draws its rib with
    `Etch::partial(film − slab).with_sidewall_deg(sidewall)`;
  - the thermo-optic shifter (`src/device/thermo.rs`) adds the layers `gap`, `TiN` and `cover`
    above the core and draws the heater on `TiN`.
- **The geometry kernel** (`src/geometry/`, #227) has the operations a flow needs in the plane:
  booleans exact on shared edges, offsets and fillets, and the signed distance through which an
  etched wall is drawn. These measure widths and gaps and execute masks.
- **Provenance** (`src/material/`) is the model to copy for sources:
  - a material's `Provenance` gives its paper, DOI, data and temperature;
  - the catalogue's `Reference` (key, citation, title, checked DOI, open copy) and `Source`
    (a reference's key and the place in it, such as "Table 1") give every value its origin;
  - `Coming` lists a property that has no number yet, with the paper it will come from;
  - a film's `ResistivityValue` is a ranged value with how the film was made, its thickness and
    its source. A process capability has the same shape.
- **Thermal limits already in the catalogue.** The phase-change entries carry their switching
  temperatures from Delaney et al. 2020: T_c 200 °C for Sb₂Se₃, 270 °C for Sb₂S₃ and 180 °C for
  GST, as read in the papers folder. These are a thermal budget the flow generator can use from
  the start.
- **Jobs.** A job's `[task]` names a kind in the `TaskKind` registry (`src/job/kind.rs`, #596).
  `Event::Scene` (`src/job/event.rs`) carries the stack's layers and every shape's outline. The
  studio's `CutView.svelte` draws the cross-section from it with `cut()` in
  `studio/src/lib/deviceview.ts`.

**Gaps the engine has to close.** Reading the devices turned up four.

1. **Optical stand-ins.** The optical stack doesn't always hold the material that is physically
   there. The TFLN modulator's `metal` layer and the thermo-optic shifter's `TiN` layer are drawn
   with the oxide's optics, because each mode is solved without the metal. A flow needs the
   physical material. *Proposal:* a default method `Device::physical(values)`. It returns the
   geometry as it is, except where a device replaces a stand-in with the real material (the
   electrodes' metal, the heater's TiN) and adds what the optical model leaves out (a heater's
   contacts and pads). Each addition has its source, or "not specified".
2. **The scene has no etches.** `SceneShape` carries a layer and an outline only, so `CutView`
   draws every shape as a full, vertical-walled rectangle. A rib or a sloped TFLN wall is drawn
   wrong in the cross-section today. *Proposal (F2):* `SceneShape` gains an optional `etch`
   (`depth_um`, `sidewall_deg`). The field is optional with a default, so earlier run records
   still load. `CutView` then draws trapezoids and slabs. The Design tab gains from this as well.
3. **Doped regions aren't drawn.** The doped and in-guide heaters describe their doping by
   parameters, not by shapes. Implant steps therefore come later (F8), when the geometry draws
   doped regions (0.8's carrier modulators need them too).
4. **Flat layers.** `LayerStack` is planar by construction. A cladding deposited over a strip
   is not flat unless it is planarized or thick enough, so the exact model can't hold that
   topography. The 2D profile ([Section 2.5](#25-executing-a-flow)) shows it. A rule (F4)
   says when a flat layer drawn over a patterned one needs a planarization step.

### 2.2 The process library

A process is data, kept in Rust like the material catalogue (`src/fabrication/library/`, one file
per step type), with its references in one list whose DOIs are checked.

```rust
pub struct Process {
    pub id: &'static str,              // "icp-si", "ar-ion-milling", "lift-off"
    pub step: StepType,                // Deposition(Lpcvd), Lithography(Duv193Immersion), …
    pub name: &'static str,
    pub plain: &'static str,           // what it does, for someone new to it
    pub materials: Materials,          // what it deposits, patterns or etches, and on what
    pub capabilities: Vec<Capability>,
    pub references: &'static [&'static str], // Reference keys
}

pub struct Capability {
    pub quantity: Quantity,  // MinFeature, Overlay, Selectivity { over }, Sidewall, …
    pub value: Stated,       // a range with its source, or not specified
    pub conditions: String,  // the film, tool generation or thickness the source states
}

pub enum Stated {
    Range { low: f64, high: f64, unit: Unit, source: Source },
    NotSpecified { coming: Option<Coming> }, // the paper it will come from, if known
}
```

`Stated` makes a number without a source impossible to write: the range carries its `Source`,
and the only other value says it is not specified.

**Step types,** each with the capabilities that matter for it:

| Step type | Processes | Capabilities |
|---|---|---|
| Deposition | thermal oxidation, LPCVD, PECVD, sputtering, ALD, evaporation, epitaxy, wafer bonding and ion slicing | thickness range, uniformity, conformality, process temperature, stress, roughness, the materials it deposits and on what |
| Lithography | contact and i-line, DUV 248 nm and 193 nm (dry and immersion), EUV, electron beam, nanoimprint | resolution and minimum feature, overlay, field size, resist thickness, line-edge roughness |
| Pattern transfer | RIE and ICP dry etching by chemistry, ion-beam (Ar⁺) milling, wet etching, lift-off | selectivity over the mask and the layer below, sidewall angle, rate and its uniformity, sidewall roughness, aspect-ratio limits, redeposition |
| Planarization | chemical-mechanical polishing (CMP) | removal selectivity, dishing and erosion, remaining-thickness uncertainty |
| Anneal | furnace and rapid thermal anneal | temperature and time, what it changes (densification, crystallization, smoothing) |
| Doping | ion implantation, diffusion | species, dose and energy ranges, activation anneal |
| Release | undercut by wet or vapour etch | selectivity, undercut rate |
| Dicing and facets | sawing, cleaving, deep etching for facets, polishing | facet angle and roughness |

**Materials and thermal budgets.**

- Each catalogue material can carry a thermal budget: the highest temperature it survives once
  deposited, with its source, or "not specified".
- The phase-change materials' budgets come from the catalogue's switching temperatures above.
- Others come from the textbooks and papers in [Section 7](#7-sources). For example, metals
  before high-temperature steps is the textbook ordering rule (Plummer, Deal & Griffin; Campbell).

**Glossary.** Each term the narration uses (resist, mask, etch stop, selectivity, lift-off,
conformal, overlay) has a plain definition of one or two sentences. Each definition cites the
textbook it follows.

### 2.3 Reading a design: features and their neighbours

The generator works on the **physical structure** (`Device::physical`, or a job's own structure).
For each layer, from the bottom up, it measures:

- **Feature sizes:** the narrowest line and the narrowest gap, measured by erosion and dilation
  with the kernel's offsets. A line of width w disappears under an inward offset of w/2; a gap of
  width g closes under an outward offset of g/2. The measurement is exact for the kernel's
  shapes.
- **Aspect ratios:** the etch depth over the narrowest gap. The pattern-transfer rules compare it
  with each etch's cited limits, for example on aspect-ratio-dependent etching (Donnelly &
  Kornblit 2013, *to read*).
- **Below and above:**
  - the material under an etch's floor, which decides between an etch stop and a timed etch;
  - the material over a feature, which decides refill, cladding or planarization;
  - whether a later layer lies over topography.
- **Overlay between layers:** how far a feature on one layer sits from the feature it must align
  to, such as a heater over its guide. The rules compare that distance with the lithography's
  cited overlay.
- **Materials:** which processes can deposit, pattern and etch each material, and the thermal
  budgets of the materials already present.

### 2.4 The flow generator

Rules, not search: each rule reads the measurements above and either decides or offers choices.

- **Starting wafer.** A single-crystal film on an insulator comes as a bonded wafer, and is never
  deposited. Silicon on insulator is made by implantation and splitting (Bruel 1995). Lithium
  niobate on insulator is made by crystal ion slicing (Levy et al. 1998) and bonding (Poberaj et
  al. 2012). The layers bought with the wafer are named.
- **Subtractive or additive.** A crystalline film is etched. A thin metal is lifted off or etched,
  chosen by the resist-to-film thickness ratio and the feature size. A film that can't be etched
  selectively over what lies below may be patterned by a Damascene process (Pfeiffer et al.
  2016). The rule's thresholds come from the textbooks, *to read*.
- **Lithography.** Among the lithographies whose cited minimum feature is below the design's
  narrowest line and gap:
  - the generator suggests the one the platform's papers used;
  - it lists the others with their cited ranges;
  - electron-beam lithography is always offered for prototypes.

  When no lithography's cited range reaches the design's feature, the flow says so; it doesn't
  pick the closest one.
- **Mask.** Resist alone, or a hard mask when the etch depth over the resist's thickness needs
  more selectivity than the resist's cited value.
- **Etch depth.**
  - **Full etch with an etch stop** when the layer below has a cited selectivity: silicon on its
    buried oxide.
  - **Timed etch** otherwise, as in a rib's partial etch. Its depth uncertainty is the rate's
    cited uncertainty times the time when both are sourced, and "not specified" when either
    isn't.
- **Sidewall.** The design's sidewall angle is compared with each etch's cited range. A design
  outside every range is flagged, with the processes that come closest. Thin-film lithium niobate
  under argon milling is the case in point (Zhang et al. 2017; Kaufmann et al. 2023, *to read*).
- **Thermal-budget ordering.** Steps are ordered so that no step exceeds the budget of a layer
  already present. When the design's order forces a violation, the flow says so and offers a
  lower-temperature alternative: LPCVD nitride above a metal is refused, and PECVD is offered.
- **Planarization.** CMP when a later patterned layer or a flat cladding sits over topography.
  Otherwise the flow notes the topography the profile shows.
- **Ties** are broken by the library's order, so a design always gives the same flow.

**The output** is a `Flow`: ordered `Step`s, plus the warnings and the list of what was not
specified. Each step has:

- its process;
- its operation (below);
- its reason in plain words;
- its alternatives, each with why it wasn't chosen;
- the capabilities it relied on, with their sources;
- its uncertainties.

### 2.5 Executing a flow

Each step carries a geometry operation, so a flow can be run:

```rust
pub enum Operation {
    Wafer { stack: LayerStack },                        // the layers bought with the wafer
    Deposit { layer: String, material: Material, thickness: Length, profile: Profile },
    Coat { resist: String, thickness: Length },
    Expose { mask: Vec<Shape>, tone: Tone },            // the mask is the design's shapes
    Develop,
    Etch { layer: String, depth: Depth, sidewall: f64 }, // Depth::ToStop or Depth::Timed(d)
    LiftOff,
    Strip,
    Planarize { to: Length },
    Anneal { temperature: Stated, time: Stated },
}
```

The executor keeps two representations of the wafer:

- **The exact model is a `Structure`.**
  - `Deposit` adds a layer.
  - `Expose` and `Etch` draw the mask's shapes on the layer with the etch's depth and sidewall.
  - The next `Deposit` that fills the etched region sets the layer's `background`.
  - `LiftOff` draws the openings' shapes with the lifted-off film as material.
  - `Planarize` leaves the planar model unchanged, since it is flat by construction.

  The final state is compared with the design ([Section 3](#3-validation)).
- **The 2D profile is drawn for the animation.**
  - It holds polygons per material in a cut across the chip, at the same cut `CutView` uses.
  - Deposition is an offset of the surface: conformal, directional or planar, as the process's
    cited conformality says.
  - Etching subtracts the mask's opening swept down with the sidewall angle.
  - Lift-off removes the resist and everything on it.
  - Planarization clips at a height.

  All of these are the kernel's booleans and offsets.

## 3. Validation

The engine is decision support, and its trust rests on checks anyone can rerun. The cases follow
CONTRIBUTING.md, "Validation cases and their tolerances": each has a measure and a basis, and
none has a tolerance chosen by hand.

1. **The round trip.**
   - **What runs:** for every library device at its default design, at the corners of its
     parameter ranges, and for every Academy and example design, the generated flow is executed.
   - **What is compared:** the final `Structure` against the design's physical structure, by
     `PartialEq`, and `material_at` on a grid through every layer.
   - **The rule:** absolute, the count of differing points must be 0; basis round-off, an exact
     identity.
   - **The stand-ins:** with the stand-in materials put back, the result must equal
     `Device::geometry(values).structure` exactly, so the physical and optical structures can't
     drift apart.
   - **What it proves:** the flow makes the design's layers and shapes, no more and no less. It
     doesn't prove the physics of each step, which checks 2 and 3 cover.
2. **The profile against the structure.**
   - **What is compared:** after the last step, each material's area in the 2D profile against
     the same cut of the final `Structure`, through `cut()`'s geometry.
   - **The rule:** relative; basis discretisation, bounded by the integer grid the kernel's
     booleans snap to, times the outline's length. Where a planarization is missing, the profile
     differs on purpose. Those cases compare only below the topography and say so.
3. **Capabilities against their sources.**
   - **The automatic test:** every `Stated::Range` names a reference key that exists, with a
     checked DOI (or an ISBN for a book) and a location in it (a table, figure, equation or
     page). Every reference is listed in the papers folder's README or in this plan's other
     sources.
   - **The rule for numbers:** a value enters only after its paper has been read and the value
     recorded in the README's notes with its location, as the catalogue's values are.
   - **Review:** a reviewer checks the number against the note; the test checks that the note
     and the link exist.
4. **Rules on cases with known answers.**
   - **Thermal oxidation.** Deal & Grove's linear-parabolic law for the oxide's thickness against
     time, with their constants, against their own tabulated values. Basis: reference, to half a
     unit of the last printed digit.
   - **Measured features.** The narrowest line and gap of a rectangle, a ring and a coupler
     against their drawn values. Basis: round-off.
   - **Lithography choice.** Designs just inside and just outside each lithography's cited range
     must pick or refuse it. Exact: a choice is categorical.
   - **Thermal ordering.** The textbook ordering (metals after the high-temperature steps;
     Plummer, Deal & Griffin; Campbell), and a phase-change cell refused before a step above its
     cited switching temperature. Exact.
   - **Etch stop or timed.** A full silicon etch on oxide stops; a rib's partial etch is timed.
     Exact.
5. **Flows that papers report.** The generated sequence of step types must contain the sequence
   the paper describes for its own device, in order. Exact. The process sections are all *to
   read*:
   - Bogaerts et al. 2005 and Selvaraja et al. 2009 for silicon;
   - Zhang et al. 2017 and Krasnokutska et al. 2018 for lithium niobate;
   - Jacques et al. 2019 for the TiN heater;
   - Delaney et al. 2021 for Sb₂Se₃ cells.

   Where a paper leaves a step out, the case says so instead of guessing.
6. **Determinism.** The same design gives the same flow and the same run record, bit for bit, on
   1 and 20 threads, and on replay.

## 4. The visualization

### 4.1 Animated cross-sections

**One component, `ProcessAnimation.svelte`,** is used everywhere the animation appears.

- It reuses `CutView.svelte`'s drawing: the scale true to size or stretched in height with a
  label saying so, the axes and ticks, the layer names at the right, and `fillOf` for each
  material's colour.
- Each step's profile comes from the run's events.
- The component tweens from one profile to the next:
  - **deposition:** the new film grows from zero to its thickness, conformal or flat as the
    process is;
  - **coating:** the resist spreads;
  - **exposure:** light or the beam passes through the mask's openings, and the exposed resist
    changes shade;
  - **development:** the exposed (or unexposed) resist goes;
  - **etching:** the floor moves down, the walls at the design's sidewall angle, stopping on the
    layer below or at the timed depth with its uncertainty shown as a band;
  - **lift-off:** the resist rises and takes the metal on it, leaving the metal in the openings;
  - **planarization:** the surface is cut flat.
- **Controls:** play, pause, step forward and back, a speed control, and a list of steps beside
  the picture that also jumps to a step.
- **Reduced motion.** With `prefers-reduced-motion`, steps change without tweening.
- **2D first, in SVG.** A 3D animation on the 3D view (`Device3D.svelte`, `view3d.ts`) is F8 and
  optional ([Section 9](#9-risks)).

**Narration.**

- Each step shows its process's plain sentence, then the reason for this design in plain words.
  An example: "The silicon is etched through to the oxide below. The oxide etches far more slowly,
  so it stops the etch, and the strip's height is the film's."
- Then come the alternatives and the cited capabilities. Each number shows its source on hover;
  a capability without a source reads "not specified".
- Terms link to the glossary through `Tip.svelte`.
- The narration assumes no knowledge of fabrication: it says what a resist is the first time
  one appears.

### 4.2 The Fabrication tab and the Fabricate button

- **On a component's page** (`DevicePage.svelte`), a sixth tab, **Fabrication**, joins Design,
  Overview, Model, Simulate, Study and Optimize. A **Fabricate** button in the page's header,
  beside Run, opens it.
- **What the tab shows:**
  - the cut, which the Design view's draggable cut sets;
  - the animation;
  - the steps, each with its reason, its alternatives and its sources;
  - the warnings;
  - what was not specified.
- **Choices.** The user may prefer or avoid processes. The flow is generated again and says what
  changed. Until F4, the flow is read-only.
- **In the run view** (`Viewer.svelte`), a run whose job has a `[device]` shows **Fabricate**,
  which opens the Fabrication tab of that design on its page. A run of a structure job without a
  device fabricates the structure itself, from the same `Structure` the run's scene was drawn
  from (F4).
- **Export.** The flow as JSON and CSV, and a printable page with the steps, reasons and sources
  (F7).

### 4.3 The Academy and the examples

- **A directive.** `::fabrication <device>{key=value}` embeds the animation in a lesson, beside
  `::diagram`, `::chart` and `::example` (academy/README.md).
- **A lesson, "How a chip is made",** follows the Academy's style: a topic with numbered
  subsections, one question each. The questions:
  1. What is a wafer, and where does the thin film come from?
  2. How does a pattern get from a drawing onto the wafer?
  3. Why does an etch stop where it does?
  4. When is a metal lifted off instead of etched?
  5. Why does the order of the steps matter?
  6. How small can a feature be, and what limits it?

  Each subsection embeds the step it explains. The history section follows the lesson format:
  Deal & Grove's oxidation, Chou's imprint lithography, the move of photonics onto CMOS lines
  (Bogaerts et al. 2005), and ion slicing for lithium niobate.
- **Existing lessons.** Each lesson whose device is in the library (the strip waveguide, the
  MZI, the directional coupler) gains a short "How it is made" subsection with the directive.
- **The examples.** Validation's published results (`PublishedResults.svelte`) and a device
  page's list of examples show **Fabricate** beside every example that builds a library device.
  It opens the Fabrication tab of that example's design.

## 5. The job format

**A `fabrication` task kind** beside a `[device]` (`Holder::Device`), added as CONTRIBUTING.md's
"A new kind of job" sets out:

- one file, `src/job/fabrication.rs`, with the task's struct (`deny_unknown_fields`) and
  `FabricationKind`;
- one line in the `DEVICE` registry of `src/job/kind.rs`;
- its line in `kind_tests.rs`;
- the schema written again.

```toml
[device]
kind = "TFLN Mach-Zehnder modulator"
# … the design's values

[task]
kind = "fabrication"
cut = { normal = "x", at_um = 120.0 }  # where the profile is cut; the device's own cut if left out
prefer = ["electron-beam"]             # processes to prefer where several qualify
avoid = ["lift-off"]                   # processes not to use
```

**What each part of the kind does:**

- **`check`:** the design draws, every named process exists in the library, and a preferred
  process can make the design's features. Otherwise it refuses with the reason, such as "the
  design's narrowest gap is below every cited range of contact lithography", with the gap and
  the ranges and their sources.
- **`preview`:** the scene, as for any device job.
- **`execute`:**
  - an `Event::Flow` with the steps, their reasons, alternatives, capabilities and sources;
  - an `Event::Profile` per step;
  - the round trip's result, the count of differing points, which a run reports and which the
    validation case requires to be 0.
- **`exports`:** `flow`, the steps as a table.
- **`farms`:** false, since a flow has no points to farm.

A flow is data and fully determined by the design and the library's version, so a run record
replays bit for bit. The record names the library's version, so an older run states which
capabilities it used. A task alone (a structure job's shapes) follows in F4 as a kind of
`Holder::Solver`.

The new module `fabrication` sits in the geometry layer of `src/layering.rs`, beside `stack` and
`material`, since it reads only those. `Device::physical` is in the device layer and the task kind
in the top layer, as every other kind is.

## 6. Phases

Each phase is one pull request that can be merged on its own. Sizes: S is up to about a day's
review, M a few days, L a week or more.

| Phase | Size | What it delivers | Needs |
|---|---|---|---|
| **F1: The first flows, as data** | M | `photonoxide::fabrication`: the process library's types, `Stated`, the entries the three first designs need, the operations and the exact executor, `Device::physical` for the TFLN modulator and the metal-heater shifter, and the generator's rules for these designs. The `fabrication` task kind. Validation: the round trip for the three designs at their defaults and corners, the references test, determinism. | 0.5.4's devices |
| **F2: The Fabrication tab, read-only** | M | `SceneShape` with its etch, so `CutView` draws ribs and sloped walls everywhere. The Fabrication tab and the Fabricate button on a component's page and in the run view, with each step's still cross-section, reasons, alternatives, sources and what's not specified. | F1 |
| **F3: Animation and narration** | M | The 2D profile executor, `ProcessAnimation.svelte` with its controls and reduced motion, the plain narration, and the glossary with its sources. Validation: the profile against the structure. | F2 |
| **F4: The rules in full** | L | Feature and gap measurement, aspect ratios, lithography by cited resolution, hard or soft mask by selectivity, etch stop or timed with its uncertainty, thermal-budget ordering, overlay, planarization, and alternatives ranked. Preferences and avoidances in the tab. A structure job's flow. Validation: the textbook cases and the papers' reported flows. | F1; the textbooks read |
| **F5: The Academy and the examples** | S | The `::fabrication` directive, the lesson "How a chip is made", "How it is made" in the strip, MZI and coupler lessons, and Fabricate beside the examples. | F3 |
| **F6: More platforms** | S each | One pull request per platform, each with its processes, sources and a round-tripped design: LPCVD silicon nitride, with the Damascene alternative for thick films (Bauters et al. 2011; Luke et al. 2013; Pfeiffer et al. 2016); phase-change cells, sputtered with lift-off and a cap, their switching temperatures as budgets (Ríos et al. 2015; Delaney et al. 2021); sputtered aluminium nitride (Xiong et al. 2012); gallium arsenide by epitaxy and etching (Dietrich et al. 2016). | F4 |
| **F7: The façade and export** | S | `facade::device_fabrication` for Python and the program's other languages; the flow as JSON and CSV, and a printable report. | F4 |
| **F8: Later, each when asked for** | L | The 3D animation; implant and diffusion steps once doped regions are drawn (0.8); a user's PDK (0.9) as a source of foundry capabilities; a flow's stated uncertainties as 0.11's process corners. | 0.8, 0.9, 0.11 |

**The first slice, F1 to F3,** is the one asked for: a read-only flow, shown and animated, for
three designs.

- **The SOI strip waveguide** (the `waveguide` device on `soi_220`):
  - the SOI wafer as bought;
  - resist patterned by DUV lithography (Bogaerts et al. 2005; Selvaraja et al. 2009) or
    electron-beam lithography (Bojko et al. 2011);
  - silicon etched by ICP through to the buried oxide, which stops the etch;
  - the resist stripped and the oxide cladding deposited.
- **The TFLN modulator** (the device `TFLN Mach-Zehnder modulator`):
  - lithium niobate on insulator, made by ion slicing and bonding, as bought;
  - the rib patterned by electron-beam lithography;
  - the film etched part of the way by argon-ion milling, a timed etch, at the sidewall angle
    the design states, compared with the cited range;
  - the electrodes lifted off. The electrode metal and the lift-off are as Wang et al. 2018 and
    Zhu et al. 2021 state them, *to read*; "not specified" until then.
- **The thermo-optic heater** (the device `thermo-optic phase shifter`, its heater metal):
  - the strip as above, and the oxide to the heater's height;
  - TiN sputtered and patterned by lift-off or by etching, both offered;
  - the cover oxide;
  - the contact metal and vias, from `Device::physical`, sourced from Jacques et al. 2019 and
    Harris et al. 2014 where they state them, "not specified" where they don't.

## 7. Sources

Every DOI below was checked on Crossref on 2026-10-30, and every author, title, journal, volume,
first page and year is as Crossref gives it. The two books Crossref doesn't hold were checked on
Open Library. Years are the published-online year where Crossref gives one. "Folder" marks a
paper already in the papers folder. The other 27 are in its README, in "Wanted for the
fabrication engine". Open copies are noted only where one was seen.

**Textbooks**

- J. D. Plummer, M. D. Deal, P. B. Griffin, *Silicon VLSI Technology*, Prentice Hall (2000). ISBN 9780130850379 (Open Library; not on Crossref).
- S. A. Campbell, *Fabrication Engineering at the Micro- and Nanoscale*, Oxford University Press (2012). ISBN 9780199861224 (Open Library; not on Crossref).
- M. J. Madou, *Fundamentals of Microfabrication and Nanotechnology*, three-volume set, CRC Press (2018). [10.1201/9781315274164](https://doi.org/10.1201/9781315274164)
- S. Franssila, *Introduction to Microfabrication*, Wiley (2010). [10.1002/9781119990413](https://doi.org/10.1002/9781119990413)
- C. Mack, *Fundamental Principles of Optical Lithography*, Wiley (2007). [10.1002/9780470723876](https://doi.org/10.1002/9780470723876)

**Process steps**

- B. E. Deal, A. S. Grove, "General Relationship for the Thermal Oxidation of Silicon", J. Appl. Phys. 36, 3770 (1965). [10.1063/1.1713945](https://doi.org/10.1063/1.1713945)
- S. M. George, "Atomic Layer Deposition: An Overview", Chem. Rev. 110, 111 (2009). [10.1021/cr900056b](https://doi.org/10.1021/cr900056b)
- S. Y. Chou, P. R. Krauss, P. J. Renstrom, "Imprint Lithography with 25-Nanometer Resolution", Science 272, 85 (1996). [10.1126/science.272.5258.85](https://doi.org/10.1126/science.272.5258.85)
- Y. Chen, "Nanofabrication by electron beam lithography and its applications: A review", Microelectron. Eng. 135, 57 (2015). [10.1016/j.mee.2015.02.042](https://doi.org/10.1016/j.mee.2015.02.042)
- V. M. Donnelly, A. Kornblit, "Plasma etching: Yesterday, today, and tomorrow", J. Vac. Sci. Technol. A 31, 050825 (2013). [10.1116/1.4819316](https://doi.org/10.1116/1.4819316)
- M. Krishnan, J. W. Nalaskowski, L. M. Cook, "Chemical Mechanical Planarization: Slurry Chemistry, Materials, and Mechanisms", Chem. Rev. 110, 178 (2009). [10.1021/cr900170z](https://doi.org/10.1021/cr900170z)

**Silicon photonics**

- M. Bruel, "Silicon on insulator material technology", Electron. Lett. 31, 1201 (1995). [10.1049/el:19950805](https://doi.org/10.1049/el:19950805)
- W. Bogaerts et al., "Nanophotonic waveguides in silicon-on-insulator fabricated with CMOS technology", J. Lightwave Technol. 23, 401 (2005). [10.1109/JLT.2004.834471](https://doi.org/10.1109/JLT.2004.834471) (folder)
- S. K. Selvaraja, P. Jaenen, W. Bogaerts, D. Van Thourhout, P. Dumon, R. Baets, "Fabrication of Photonic Wire and Crystal Circuits in Silicon-on-Insulator Using 193-nm Optical Lithography", J. Lightwave Technol. 27, 4076 (2009). [10.1109/JLT.2009.2022282](https://doi.org/10.1109/JLT.2009.2022282)
- R. J. Bojko, J. Li, L. He, T. Baehr-Jones, M. Hochberg, Y. Aida, "Electron beam lithography writing strategies for low loss, high confinement silicon optical waveguides", J. Vac. Sci. Technol. B 29, 06F309 (2011). [10.1116/1.3653266](https://doi.org/10.1116/1.3653266)
- K. K. Lee, D. R. Lim, L. C. Kimerling, J. Shin, F. Cerrina, "Fabrication of ultralow-loss Si/SiO2 waveguides by roughness reduction", Opt. Lett. 26, 1888 (2001). [10.1364/OL.26.001888](https://doi.org/10.1364/OL.26.001888) (folder). Oxidation smoothing took a 0.5 µm strip's loss from 32 to 0.8 dB/cm, as read in the folder.
- F. P. Payne, J. P. R. Lacey, Opt. Quantum Electron. 26, 977 (1994), and T. Barwicz, H. A. Haus, J. Lightwave Technol. 23, 2719 (2005), [10.1109/JLT.2005.850816](https://doi.org/10.1109/JLT.2005.850816) (folder): sidewall roughness as loss, already in `mode::roughness`. A process's cited roughness becomes a design's `roughness` and `correlation`.
- S. Y. Siew et al., "Review of Silicon Photonics Technology and Platform Development", J. Lightwave Technol. 39, 4374 (2021). [10.1109/JLT.2021.3066203](https://doi.org/10.1109/JLT.2021.3066203) (folder): the platforms' process modules.

**Thin-film lithium niobate**

- M. Levy, R. M. Osgood, R. Liu, L. E. Cross, G. S. Cargill, A. Kumar, H. Bakhru, "Fabrication of single-crystal lithium niobate films by crystal ion slicing", Appl. Phys. Lett. 73, 2293 (1998). [10.1063/1.121801](https://doi.org/10.1063/1.121801)
- G. Poberaj, H. Hu, W. Sohler, P. Günter, "Lithium niobate on insulator (LNOI) for micro-photonic devices", Laser Photonics Rev. 6, 488 (2012). [10.1002/lpor.201100035](https://doi.org/10.1002/lpor.201100035)
- G. Ulliac, B. Guichardaz, J.-Y. Rauch, S. Queste, S. Benchabane, N. Courjal, "Ultra-smooth LiNbO3 micro and nano structures for photonic applications", Microelectron. Eng. 88, 2417 (2011). [10.1016/j.mee.2011.02.024](https://doi.org/10.1016/j.mee.2011.02.024) (an open copy on HAL, hal-00583832)
- M. Zhang, C. Wang, R. Cheng, A. Shams-Ansari, M. Lončar, "Monolithic ultra-high-Q lithium niobate microring resonator", Optica 4, 1536 (2017). [10.1364/OPTICA.4.001536](https://doi.org/10.1364/OPTICA.4.001536) (open access)
- I. Krasnokutska, J.-L. J. Tambasco, X. Li, A. Peruzzo, "Ultra-low loss photonic circuits in lithium niobate on insulator", Opt. Express 26, 897 (2018). [10.1364/OE.26.000897](https://doi.org/10.1364/OE.26.000897) (open access)
- C. Wang et al., "Integrated lithium niobate electro-optic modulators operating at CMOS-compatible voltages", Nature 562, 101 (2018). [10.1038/s41586-018-0551-y](https://doi.org/10.1038/s41586-018-0551-y) (folder)
- D. Zhu et al., "Integrated photonics on thin-film lithium niobate", Adv. Opt. Photon. 13, 242 (2021). [10.1364/AOP.411024](https://doi.org/10.1364/AOP.411024) (folder)
- F. Kaufmann, G. Finco, A. Maeder, R. Grange, "Redeposition-free inductively-coupled plasma etching of lithium niobate for integrated photonics", Nanophotonics 12, 1601 (2023). [10.1515/nanoph-2022-0676](https://doi.org/10.1515/nanoph-2022-0676) (open access, CC BY 4.0)

**Heaters, nitride, phase-change materials and other platforms**

- M. Jacques et al., Opt. Express 27, 10456 (2019), [10.1364/OE.27.010456](https://doi.org/10.1364/OE.27.010456), and N. C. Harris et al., Opt. Express 22, 10487 (2014), [10.1364/OE.22.010487](https://doi.org/10.1364/OE.22.010487) (folder): the TiN and doped heaters the thermo-optic shifter reproduces.
- J. F. Bauters et al., "Ultra-low-loss high-aspect-ratio Si3N4 waveguides", Opt. Express 19, 3163 (2011). [10.1364/OE.19.003163](https://doi.org/10.1364/OE.19.003163) (open access)
- K. Luke, A. Dutt, C. B. Poitras, M. Lipson, "Overcoming Si3N4 film stress limitations for high quality factor ring resonators", Opt. Express 21, 22829 (2013). [10.1364/OE.21.022829](https://doi.org/10.1364/OE.21.022829) (open access)
- M. H. P. Pfeiffer et al., "Photonic Damascene process for integrated high-Q microresonator based nonlinear photonics", Optica 3, 20 (2016). [10.1364/OPTICA.3.000020](https://doi.org/10.1364/OPTICA.3.000020) (open access)
- C. Ríos et al., "Integrated all-photonic non-volatile multi-level memory", Nat. Photonics 9, 725 (2015). [10.1038/nphoton.2015.182](https://doi.org/10.1038/nphoton.2015.182)
- M. Delaney et al., Adv. Funct. Mater. 30, 2002447 (2020), [10.1002/adfm.202002447](https://doi.org/10.1002/adfm.202002447) (folder), and M. Delaney, I. Zeimpekis, H. Du, X. Yan, M. Banakar, D. J. Thomson, D. W. Hewak, O. L. Muskens, "Nonvolatile programmable silicon photonics using an ultralow-loss Sb2Se3 phase change material", Sci. Adv. 7, eabg3500 (2021). [10.1126/sciadv.abg3500](https://doi.org/10.1126/sciadv.abg3500) (open access, CC BY 4.0)
- C. Xiong, W. H. P. Pernice, H. X. Tang, "Low-Loss, Silicon Integrated, Aluminum Nitride Photonic Circuits and Their Use for Electro-Optic Signal Processing", Nano Lett. 12, 3562 (2012). [10.1021/nl3011885](https://doi.org/10.1021/nl3011885)
- C. P. Dietrich, A. Fiore, M. G. Thompson, M. Kamp, S. Höfling, "GaAs integrated quantum photonics: Towards compact and multi-functional quantum photonic integrated circuits", Laser Photonics Rev. 10, 870 (2016). [10.1002/lpor.201500321](https://doi.org/10.1002/lpor.201500321)

**Roadmaps and open design rules (not papers; not in the papers folder).** Each site below was
reached on 2026-10-30.

- The International Roadmap for Devices and Systems (IEEE), 2024 Edition, its "Lithography and
  Patterning" chapter and lithography tables: <https://irds.ieee.org/editions/irds2024/>. The
  editions page lists a 2026 Edition as well: <https://irds.ieee.org/editions>.
- Open multi-project-wafer services whose design manuals are public or given to designers:
  - CORNERSTONE (University of Southampton): <https://www.cornerstone.sotonfab.co.uk/>;
  - AIM Photonics: <https://www.aimphotonics.com/>;
  - imec's silicon photonics, through EUROPRACTICE: <https://europractice-ic.com/>.

  Their numbers would enter only as decision 2 settles, each with its document's version, date
  and licence.

## 8. Decisions

These are for me to decide. Each has a recommendation; none is decided yet.

1. **Which platforms first.**
   - (a) The three designs of the first slice (SOI, TFLN, the TiN heater on SOI), then LPCVD
     nitride, phase-change cells, AlN and GaAs.
   - (b) Silicon only first, the others after F4.
   - (c) Another order.

   *Recommended: (a).* All three are library devices today, so the round trip covers them from
   F1.
2. **Foundry numbers.**
   - (a) The process library holds literature numbers only.
   - (b) It may also hold numbers from foundries' public design manuals, each with its version,
     date and licence, marked as a foundry's.
   - (c) Foundry numbers come only from a PDK the user loads (0.9), kept outside the repository.

   *Recommended: (a) with (c).* Foundry manuals change, are often under agreements, and describe
   one line's process, not the method. A user's PDK gives a foundry's numbers for that user's
   flow without the repository holding them.
3. **Where the release lands.**
   - (a) All of it in 0.10, where the roadmap places it.
   - (b) The first slice (F1 to F3, with F5) in the next minor release that has room, the rest in
     0.10.
   - (c) A line under "Throughout", landing phase by phase.

   *Recommended: (b).* The first slice needs only 0.5.4's devices and stands on its own; F4's
   rules and F8 meet 0.9's PDKs and 0.11's corners.
4. **Editing a flow.** Whether the tab lets a user reorder or replace steps by hand, with the
   round trip and the rules checking the result. *Recommended:* only preferences and avoidances
   until F4 is in place, then editing if asked for.
5. **The Academy's form.** A new lesson "How a chip is made" with the directive in existing
   lessons, or sections inside the existing lessons only. *Recommended:* the new lesson, plus a
   short "How it is made" subsection in the strip, MZI and coupler lessons.

## 9. Risks

- **Academic accuracy.** The engine teaches people who can't yet judge it, so a wrong number does
  more harm in it than in a solver.
  - *Mitigations:* `Stated` makes an unsourced number unwritable. Every value is read from its
    paper before it enters, and the test checks the link. "Not specified" is shown, never hidden.
  - *What remains:* a flow is one sound way, not the only one. The tab says so on every flow:
    "a suggested flow from published processes, not a foundry's process".
- **Scope creep.** Process simulation, lithography imaging, recipes, cost and yield are all
  natural next requests.
  - *Mitigation:* the non-goals ([Section 1](#1-goal-non-goals-and-readers)) hold. Imaging and
    etch proxies stay in 0.11, foundry rules in 0.9's PDKs, and anything else enters as its own
    plan.
- **Optical stand-ins drifting from the physical design.**
  - *Mitigation:* the round trip compares both: the physical structure with its stand-ins put
    back must equal the optical one exactly.
- **The cost of animation.** 3D animation of conformal films and sloped etches is costly to build
  and to keep.
  - *Mitigation:* 2D SVG first, reusing `CutView`'s drawing. 3D is F8 and only on request.
- **A library that grows stale.** New lithography generations and platforms appear every year.
  - *Mitigation:* each process carries its sources' years, and the IRDS edition it follows is
    named. A source's newer edition is a reading task, like any paper's.
- **Overconfidence in the round trip.** It proves that the flow makes the design's geometry, not
  that each step behaves as described.
  - *Mitigation:* the plan and the tab both say so. The capability and rule checks (Section 3,
    items 3 to 5) cover the steps.

## 10. Placement in the roadmap

The engine belongs with **0.10: Fabrication data**: that milestone is about taking a design to a
process of the user's choice, and the engine explains what that process does. Two of its parts
meet other milestones:

- F4's rules can read a user's PDK (0.9);
- F8 feeds 0.11's process corners.

The first slice (F1 to F3, and F5) needs only 0.5.4's devices and geometry, so it can come
earlier (decision 3). ROADMAP.md records the engine in 0.10 with its phases, and notes that the
first slice may land before it.

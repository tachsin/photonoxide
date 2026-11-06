# The Academy

The lessons the studio's Academy page teaches from. Each file here is one lesson, built into the
`photonoxide` program (`studio/src-tauri/src/academy.rs`), and read on GitHub as ordinary
Markdown. A lesson explains a device or a topic at three depths, tells its history paper by
paper, opens on a drawing of its device, and draws charts that photonoxide computes live, so
moving a slider shows the physics the library computes, the code the
[validation report](../docs/validation.md) checks.

| Lesson | Level | Diagram | Charts |
|---|---|---|---|
| [Silicon, silica and dispersion](material-dispersion.md) | introductory | `dispersion` | `material-oscillator`, `material-cauchy`, `material-index`, `material-gvd` |
| [The ring resonator](ring-resonator.md) | introductory | `ring` | `ring-spectrum`, `ring-extinction`, `ring-coupling`, `ring-q-length`, `ring-identify` |
| [Bragg gratings and mirrors](bragg-gratings.md) | intermediate | `bragg` | `bragg-reflectance`, `bragg-bandwidth` |
| [Grating couplers](grating-couplers.md) | intermediate | `grating` | `grating-angle`, `grating-overlap`, `grating-apodization`, `grating-directionality`, `grating-band` |
| [The Mach–Zehnder interferometer](mach-zehnder.md) | introductory | `mzi` | `mzi-spectrum`, `mzi-extinction` |
| [The directional coupler and splitters](directional-coupler.md) | intermediate | `coupler` | `coupler-power`, `coupler-supermodes`, `coupler-gap`, `coupler-spectrum`, `coupler-mismatch` |
| [The slab waveguide](slab-waveguide.md) | introductory | `slab` | `slab-fields`, `slab-bv`, `slab-neff`, `slab-confinement` |
| [The strip waveguide and its cross-section](strip-waveguide.md) | intermediate | `strip` | `strip-fields`, `strip-neff`, `strip-models`, `strip-corners`, `strip-group-index` |
| [Leaky and bent guides, and multilayers](leaky-and-bent-guides.md) | advanced | `leaky` | `leaky-field`, `leaky-substrate`, `bend-loss` |
| [How a chip is made](how-a-chip-is-made.md) | introductory | `process` | `fab-resolution`, `fab-oxidation` |
| [Multimode interference couplers](mmi.md) | intermediate | `mmi` | `mmi-field`, `mmi-modes`, `mmi-length`, `mmi-spectrum`, `mmi-tolerance` |
| [Photonic crystals and band gaps](photonic-crystals.md) | advanced | `crystal` | `crystal-stack`, `crystal-bands`, `crystal-group-index` |
| [Metasurfaces and metalenses](metasurfaces.md) | advanced | `metalens` | `meta-orders`, `meta-pillar`, `meta-focus`, `meta-axis`, `meta-bandwidth` |
| [FDFD and adjoint gradients](fdfd-and-adjoints.md) | advanced | `fdfd` | `fdfd-convergence`, `fdfd-gradient`, `fdfd-difference-step` |
| [Modulators and phase shifters](modulators.md) | intermediate | `modulator` | `heater-power`, `mzm-transfer`, `modulator-response`, `pcm-shift` |
| [Nonlinear integrated optics](nonlinear-optics.md) | advanced | `nonlinear` | `nl-miller`, `nl-coupled`, `nl-acceptance`, `nl-qpm`, `nl-ridge`, `nl-spm`, `nl-soliton`, `nl-fwm`, `nl-lle-states`, `nl-comb`, `nl-jsa` |

The whole curriculum is laid out: every lesson still to be written is here too, **coming soon**,
with its numbered subsections, what each will answer and the examples each will use, so writing
it is filling them in. The library lists them in this order, the order to read them in.

| Lesson coming soon | Topic | Level | Examples |
|---|---|---|---|
| [Crossings, mode converters and the polarization splitter-rotator](crossings-and-converters.md) | Multimode and polarization devices | advanced | `crossing_liu_poon`, `mode_converter_liu_poon`, `splitter_rotator_liu_poon` |
| [Surface plasmons and Mie scattering](plasmonics.md) | Nanophotonics | advanced | `lorentz_okoniewski` |
| [How FDTD works](how-fdtd-works.md) | Numerical methods | intermediate | `cpml_roden_gedney`, `pml_oskooi`, `tfsf_square_cylinder`, `subpixel_holes`, `bump_oskooi`, `lorentz_okoniewski` |
| [Inverse design](inverse-design.md), with 0.7 | Numerical methods | advanced | `circuit_splitter`, `circuit_ring_critical`, `circuit_fit` |
| [Quantum light on a chip](quantum-light.md), with 0.6.3 | Active and quantum photonics | advanced | |

## A lesson's file

Front matter in YAML between `---` lines, then the body in Markdown with TeX math.

```yaml
---
title: "The ring resonator"
summary: "One sentence for the library's card."
topic: Rings                  # what the library groups it under
level: introductory           # introductory, intermediate or advanced
minutes: 25                   # about how long it takes to read
prerequisites: []             # other lessons, by file name without .md
examples: [ring_q_factor]     # examples/<name>.rs it uses: "run the example"
jobs: [ring-fdfd.yaml]        # jobs/<file> it opens in the Custom device
circuits: [ring-all-pass.toml]  # circuits/<file> it opens on the chip
methods: [components.md]      # docs/methods/<file> it explains
validation: [components/ring-all-pass-bogaerts]  # validation cases it quotes
charts: [ring-spectrum]       # the charts it shows
papers:                       # its history, for the timeline
  - cite: "B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, 998 (1997)"
    title: "Microring resonator channel dropping filters"
    doi: 10.1109/50.588673    # checked on Crossref
    year: 1997
    role: milestone           # origin, milestone, review or current
    note: "What it did, in the lesson's own words."
---
```

Every field but the lists is required, and an unknown field is an error.

A lesson **coming soon** says so, has no `minutes`, `charts` or `papers` yet, and may name the
milestone of [the roadmap](../ROADMAP.md) whose solvers it waits for:

```yaml
---
title: "The slab waveguide"
summary: "One sentence for the library's card, and the lesson's header until it has an opening."
topic: Waveguides
level: introductory
status: coming                # published when left out
milestone: "0.6"              # only on a lesson coming soon, and only if it waits for one
prerequisites: [material-dispersion]
examples: [slab_yariv_yeh, slab_soi]  # each in one of its subsections' ::coming lines
methods: [slab.md]
---
```

## The body

**Sections** are `##` headings. A heading may end in its depth, `{intuition}`, `{theory}` or
`{research}`; without one it is intuition. The window shows intuition sections open and the
others closed, to open one at a time or all at once from the lesson's depth switch. Text before
the first heading is the lesson's opening. `#` headings aren't allowed: the title is the front
matter's.

**Parts.** A `###` heading that ends in a depth, `### The derivation {theory}`, starts a part of
the `##` section above it, at that depth, running to the next part or section: a section can
open on its intuition and keep its derivation and its literature a click away. A `###` heading
without a depth stays in the text as an ordinary subheading. The window lists every section and
part as the lesson's outline, beside the lesson on a wide window and under a Contents button
otherwise, to jump to.

A lesson either follows the usual sections:

- What it does
- The physics `{theory}`
- History `{research}`, with the timeline
- Today `{research}`: current papers and the state of the art
- How photonoxide computes it
- What our example reproduces
- Try it: exercises with the charts
- Further reading `{research}`

or, for a device worth analysing thoroughly, numbered sections each answering one question, each
with its theory and research parts and its exercises, and a closing history with the timeline:
[the ring resonator](ring-resonator.md) is written so. Every sentence states a fact, a step of a
derivation, a number with its source, or what to look at in a chart.

**Math** is TeX, as GitHub sets it: `$…$` inline (no space just inside the dollars, no letter
or digit just outside), and display math between lines holding only `$$`.

**Links** are ordinary Markdown links, so they work on GitHub too:

- a paper: `[Little et al. (1997)](https://doi.org/10.1109/50.588673)`, its DOI one of the
  front matter's papers;
- a method's write-up: `[the components](../docs/methods/components.md)`, opened beside the
  lesson in the window;
- an example: `[ring_q_factor](../examples/ring_q_factor.rs)`, opened among Validation's published results;
- another lesson: `[Bragg gratings](bragg-gratings.md)`.

**Blocks** are lines of their own, starting with `::`:

| Block | What it shows |
|---|---|
| `::diagram <id>` | the device drawn: a labelled schematic, and its structure in 3D where a job builds it |
| `::chart <id>` | a chart, every parameter at its default |
| `::chart <id>{key=value, key=value}` | a chart starting from these values |
| `::example <name>` | an example: what it printed when this release was checked, and a button to run it here |
| `::validation <id> <id> ...` | validation cases as the published report has them |
| `::device <id>` | a device the lesson teaches, by its id in the component library (`waveguide`, `rib-waveguide`), to open on its page |
| `::fabrication <id>` | how a device of the component library is made at its defaults: its flow from the bare wafer played frame by frame on its cross-section and in 3D, each step's words beneath, as the Fabrication page plays it |
| `::fabrication <id>{key=value, key=value}` | the same with these parameters, each in its own unit as a job's `[device.parameters]` writes it: `::fabrication waveguide{width=0.45, sidewall=82}` |
| `::timeline` | the front matter's papers by year, each linked by its DOI |

A chart's values carry their units or none: `radius=10um`, `radius=10000nm`, `loss=2dB/cm`,
`coupling=0.05`. A length may be written in nm, um (µm) or mm whatever unit the parameter keeps;
any other unit must be the parameter's own. Each value must be inside its parameter's range.

An **answer** to reveal on demand sits between `:::answer` and `:::` lines:

```markdown
**Try it.** Find the coupling that makes the through port dark.

:::answer
The dip reaches zero where κ₁² meets the chart's critical coupling.
:::
```

## Coming soon

A section still to be written is its heading, then a `::coming` line naming the examples it will
use (none, one or several), then one paragraph saying what it will answer, and nothing else:

```markdown
## 3. How many modes does a slab hold?

::coming slab_yariv_yeh
The normalized frequency $V$ and index $b$, the $b$–$V$ diagram, each mode's cutoff, and the mode
counts of an asymmetric and a symmetric slab.
```

The window shows it with a calm "coming soon" chip, its question, that paragraph and its examples,
whatever depth the lesson is read to. A written lesson may hold such sections; a lesson with
`status: coming` holds only them, numbered `1.`, `2.`, `3.` in order, with no opening: its summary
says what it is about. Each subsection answers one question, its heading; the paragraph names
what the answer will contain, its numbers with their sources, and no more. To write one, replace
its `::coming` line and paragraph with the text and blocks that answer it; when every section is
written, drop `status: coming` and give the lesson its reading time, diagram, papers and charts.

## The diagrams

A lesson opens on its device drawn, before any chart, with a sentence saying what to look for:
`::diagram <id>` near the top. A diagram is a clean schematic in the lesson's own symbols (the
same letters its equations use, set by KaTeX), with dimension arrows, the waves in and out, and a
caption saying what the reader is looking at. Its sizes may be illustrative; its labels may not.
Where a built-in job builds the same device, a 2D | 3D switch shows that job's structure as
photonoxide's geometry builds it, the Custom device's 3D view, turned by a drag; the schematic
shows first.

| Diagram | What it draws | 3D view |
|---|---|---|
| `dispersion` | a charge $-e$ of mass $m$ on a spring of resonance $\omega_0$ and damping $\gamma$, driven by the field $E_0 e^{-i\omega t}$, its displacement $x(t)$ and dipole $p = -ex$; beside it a sketch of a glass's $n(\omega)$ and $\kappa(\omega)$, an infrared resonance $\omega_\text{IR}$ of the lattice and an ultraviolet one $\omega_\text{UV}$ of the electrons, and the transparent window between them | none |
| `ring` | a ring and its bus from above: $R$, $w$, $g$, the coupler's $r_1$ and $k_1$, the round trip's $a e^{i\phi}$, the fields $a_1$, $b_1$, $a_2$, $b_2$, the input and through ports, and a faint second bus with its $r_2$, $k_2$, drop and add | `ring-fdfd.yaml` |
| `bragg` | a quarter-wave stack in section: $N$ pairs of $n_H$ and $n_L$, $d_H$, $d_L$ and $\Lambda$, the cover $n_0$ and substrate $n_s$, the incident, reflected ($r$) and transmitted ($t$) waves; and a waveguide grating from above, its period $\Lambda$ and length $L = N\Lambda$ | `bragg-grating.yaml` |
| `coupler` | a directional coupler from above: two strips of width $w$ a gap $g$ apart over a length $L$, brought in and out by S-bends, $z$ along the coupled section, $P_\text{cross}(z) = \sin^2\kappa z$, $\kappa = \pi\Delta n/\lambda$ and $L_c = \lambda/2\Delta n$, the input and second input, the through ($\cos^2\kappa L$) and cross ($\sin^2\kappa L$) ports; beneath, the field across the pair of the even ($n_\text{even}$) and odd ($n_\text{odd}$) supermodes | `coupler-supermodes.yaml` |
| `grating` | a grating coupler in section along its guide: a film $t$ thick with grooves $e$ deep at a period $\Lambda$, a buried oxide $h$ thick over the substrate, a cladding $n_c$; the guided mode ($n_\text{eff}$) in from the left, leaking up at $\theta$ towards a tilted fibre (MFD $2w_0$), down to the substrate and back ($r_b$), and through; $n_c \sin\theta = n_\text{eff} - \lambda/\Lambda$; above the grating, $e^{-2\alpha z}$ against the fibre's $\lvert G(z)\rvert^2$ | none |
| `crystal` | a photonic-crystal slab from above: a triangular lattice of air holes of radius $r$ at a lattice constant $a$ in a film of permittivity $\varepsilon$, a missing row (the W1 waveguide) with light guided along it, three missing holes (the L3 cavity) with its end holes moved out by $s$; beside it the hexagonal Brillouin zone with $\Gamma$, M, K and the irreducible wedge | none |
| `mzi` | a Mach–Zehnder interferometer from above: the splitter's $r_1$, $k_1$ and the combiner's $r_2$, $k_2$, arms of $L$ and $L + \Delta L$ (the longer drawn longer) with their fields $a_l e^{i\phi_l}$ and $a_u e^{i\phi_u}$, a phase shifter $\varphi$, the phase difference $\Delta\phi$, the input and the bar and cross outputs | `mzi-mmi.yaml` |
| `slab` | a slab in section: the film $n_2$ of thickness $d$ between the cover $n_1$ and the substrate $n_3$, a ray zig-zagging along $z$ at $\theta \gt \theta_c = \arcsin(n_3/n_2)$ and a steeper one lost, and a guided mode's $E_y(x)$ across the slab, $\cos(hx - \phi_3)$ in the film and the tails $e^{px}$ and $e^{-q(x - d)}$ | `slab-soi.yaml` |
| `strip` | a strip in section: the core $n_1$, $w$ wide and $t$ thick, in the cladding $n_2$ on a buried oxide $h$ thick over the substrate, $x$, $y$ and $z$; the TE-like mode's field with its main $E_x$, the four corners and Marcatili's four corner regions; and the effective index method's two slabs, across $t$ giving $N$ and across $w$ giving $n_\text{eff}$ | `strip-modes.yaml` |
| `process` | five cross-sections of one wafer as a silicon strip is made: the SOI wafer, a film $t$ thick on a buried oxide $h$ thick over the handle; resist coated and exposed through a mask by light of wavelength $\lambda$ through a lens of numerical aperture NA; the resist developed, a line $w$ wide; the silicon etched down to the oxide, where it stops; the resist stripped and the strip clad in oxide | none |
| `leaky` | a planar guide in section, the cover $n_c$, films $n_1 \ldots n_J$ and a substrate $n_s$ above $\operatorname{Re} n_\text{eff}$, the mode along $z$ shedding a plane wave into the substrate at $\theta$, weaker downstream, and its field $\lvert\psi(x)\rvert$ growing into the substrate as $e^{ik\alpha_s x}$; and a guide $n_1$ in $n_2$ bent at $R$ about $O$, its radial phase fronts, $e^{i\nu\varphi}$ with $\nu = kn_\text{eff}R$, and the radius $\rho_r = R\thinspace n_\text{eff}/n_2$ beyond which it radiates | none |
| `metalens` | a metalens in section: pillars of height $H$ and widths $w$, one every period $p$, on a substrate, lit from below by a plane wave, their widths repeating zone by zone with the wrapped phase $\varphi(r) = -k(\sqrt{r^2 + f^2} - f)$; the wave converging on the focus a focal length $f$ above, the rim's ray at $\theta$, $\mathrm{NA} = \sin\theta$, the half-width $R$; beside it a nanofin from above, rotated by $\alpha$, whose converted light carries the phase $2\alpha$, and $\varphi$ against $r$ | none |
| `fdfd` | a 2D FDFD problem: a grid closed by PMLs ($\partial_w \to s_w^{-1}\partial_w$), a guide through it, a source $b$, a monitor measuring $a = c^\mathsf{T}x$ and between them a design region of cells $\varepsilon_k$; the forward field $x$ of $A(\varepsilon)\,x = b$ travelling from the source and the adjoint field $\lambda$ of $A^\mathsf{T}\lambda = \partial F/\partial x$ travelling back from the monitor, and $dF/d\varepsilon_k = -2\operatorname{Re}(\lambda^\mathsf{T}\,\partial_kA\,x)$; beside it Yee's 2D grid for $E$ along $z$, $E_z$ at the centres and $H_x$, $H_y$ on the faces, and the five-point stencil | none |
| `mmi` | a 1 × 2 MMI from above: a section $W$ wide and $L = 3L_\pi/8$ long fed in its middle by a guide $w_a$ wide, the light inside it computed from Soldano and Pennings's sine-like modes and quadratic phases, the input's spot opening into two images at the far face, the outputs at $\pm W/4$, $z$ along it; beneath, the first four modes $\psi_0 \ldots \psi_3$ across $W$, $\beta_0 - \beta_\nu \approx \nu(\nu + 2)\pi/3L_\pi$ and $L_\pi \approx 4n_rW_e^2/3\lambda_0$ | `mmi-fdfd.yaml` |
| `nonlinear` | a periodically poled thin-film lithium niobate guide from above: domains of $+d$ and $-d$ a period $\Lambda$ apart over $L$, the fundamental $\omega$ in, $\omega$ and $2\omega$ out, $\Delta k = k_{2\omega} - 2k_\omega - 2\pi/\Lambda = 0$; beneath it the x-cut ridge in section, a film $t$ thick etched $e$ deep to $w$ wide on silica, the $c$ axis across, the fundamental's mode and the harmonic's smaller one, and $\kappa$ as their overlap through $d$; and a Kerr ring from above, $\gamma = 2\pi n_2/(\lambda A_\text{eff})$, a pump $\omega_p$ in its bus and the comb out, lines $D_1/2\pi$ apart, $\omega_s$ and $\omega_i$ either side, $2\omega_p = \omega_s + \omega_i$ | none |
| `modulator` | a push–pull Mach–Zehnder modulator from above: two arms in the gaps of a ground–signal–ground electrode $L$ long, the RF wave $n_m$ from $R_G$ into $R_L$ beside the light $n_o$, the field opposite in the two gaps; beneath it in section, x-cut thin-film lithium niobate, two ribs in gaps $g$ wide, the $c$ axis across and $\Delta n_e = -\tfrac12 n_e^3 r_{33} E$; and a thermo-optic shifter in section, a heater $h$ above a strip dissipating $P'$, its heat spreading to the substrate at $T_0$, $\Delta n = (dn/dT)\,\Delta T$ | none |

Each diagram is an entry in `studio/src-tauri/src/diagrams.rs` (its title, its captions, and its
job if it has one) and an SVG drawn by a component in `studio/src/components/diagrams/`, named
after its id (`ring` is `Ring.svelte`) and listed in that folder's `index.ts`. The SVG draws in
the theme's colours, so it follows the light and dark themes, and scales crisply with the page.
A diagram's job must be in the lesson's `jobs`.

## The charts

Each chart is a Rust function in `studio/src-tauri/src/charts.rs` that calls photonoxide's own
code; the window only draws what it returns, and asks again as a slider moves.

| Chart | Parameters (default) | Computed by |
|---|---|---|---|
| `ring-spectrum` | `radius` µm (10), `coupling` κ₁² (0.01), `drop` κ₂² (0: all-pass), `loss` dB/cm (3), `group_index` (4.2), `window` nm (20) | `AllPassRing`, `AddDropRing`: `s_matrix`, `resonance`, `fsr`, `fwhm`, `q_factor`, `finesse`, `extremes` |
| `ring-coupling` | `radius` µm (10), `loss` dB/cm (3), `drop` κ₂² (0) | `AllPassRing`, `AddDropRing`: `extremes`, `q_factor` |
| `ring-extinction` | `radius` µm (10), `loss` dB/cm (3), `drop` κ₂² (0), `coupling` κ₁² (0.01) | `AllPassRing`, `AddDropRing`: `extremes`, `q_factor`; the extinction against κ₁²/κc², and the coupling on the other side of critical with the same extinction |
| `ring-q-length` | `loss` dB/cm (2.7), `fixed` dB per round trip (0.075), `group_index` (4.30) | `AllPassRing`: `q_factor`, `finesse`, `s_matrix`, `resonance`, `fsr`, `extremes`; Bogaerts et al.'s Fig. 5, as `ring_q_factor` computes it |
| `ring-identify` | `radius` µm (10), `coupling` κ² (0.01), `loss` dB/cm (3) | `AllPassRing`: `s_matrix`, `resonance`, `fsr`, `finesse`, `q_factor`, `extremes`; `compact::fit::vector_fit` |
| `bragg-reflectance` | `high` (2.3), `low` (1.38), `pairs` (8), `period` nm (160), `cover` (1.0), `substrate` (1.52), `from` and `to` nm (350, 950) | `Multilayer::reflection`, and the endless stack's band as below |
| `bragg-bandwidth` | `low` (1.45), `up_to` (2.0) | `Multilayer::reflection`: one period's transmission $t$, and $\cos K\Lambda = \operatorname{Re}(1/t)$ |
| `mzi-spectrum` | `delta` ΔL µm (50), `split` κ₁² (0.5), `combine` κ₂² (0.5), `loss` dB/cm (3), `phase` φ/π (0), `group_index` (4.2), `window` nm (40) | a `Netlist` of `Coupler`, `Waveguide` and `PhaseShifter` solved by `Circuit::spectrum`, its peaks found on the spectrum; the extremes by `mzi_closed_form` |
| `mzi-extinction` | `combine` κ₂² (0.5), `loss` dB/cm (3), `delta` ΔL µm (50) | `mzi_closed_form`, with `Coupler`'s and `Waveguide`'s S-matrices |
| `coupler-supermodes` | `gap` nm (200), `wavelength` µm (1.55) | `mode::vector::modes` on a quarter of two 500 × 220 nm strips with mirror walls, `VectorMode::hy`: $H_y$ across the pair |
| `coupler-power` | `gap` nm (200), `wavelength` µm (1.55), `length` µm (20) | `DirectionalCoupler::s_matrix` from the supermodes of `mode::vector::modes` |
| `coupler-gap` | `wavelength` µm (1.55) | `mode::vector::modes` at 19 gaps from 100 to 1000 nm, $L_c = \lambda/2\Delta n$, on a logarithmic scale |
| `coupler-spectrum` | `gap` nm (200), `length` $L/L_c$ at 1.55 µm (0.5) | `DirectionalCoupler::from_modes` (the supermodes tracked from 1.50 to 1.60 µm) and `s_matrix` |
| `coupler-mismatch` | `width` $w_2$ nm (480), `gap` nm (200) | `mode::vector::modes` for an unequal pair and each strip alone on one grid, `Fields::cross` for the overlaps; coupled-mode theory's $\kappa^2/(\kappa^2 + \delta^2)\sin^2 Sz$ beside it |
| `material-oscillator` | `strength` Δε (1), `damping` γ/ω₀ (0.05), `background` ε∞ (1) | `material::Model::Lorentz`, `Material::refractive_index`: $n$ and $\kappa$ against $\omega/\omega_0$ |
| `material-cauchy` | `fit_to` µm (0.7) | `material::silica` (Malitson's `Model::Sellmeier`) and `Model::Cauchy` fitted to it by least squares from 0.4 µm |
| `material-index` | `material` (1 Malitson's silica, 2 Li's silicon, 3 Luke's nitride, 4 Leviton and Frey's silica at 295 K), `wavelength` µm (1.55) | `material::silica`, `silicon`, `silicon_nitride`, `material::catalogue`; `Material::refractive_index`, `Material::group_index`, `Material::provenance` |
| `material-gvd` | `material` (1 silica, 2 silicon), `wavelength` µm (1.55) | `mode::dispersion::dispersion` on the indices of `material::silica`, the catalogue's silicon `frey-2006` at 295 K and `material::silicon` (Li's table), 2 nm apart from 1.2 to 2.5 µm |
| `slab-fields` | `thickness` nm (220), `wavelength` µm (1.55), `cover` $n_1$ (1.444) | `Slab::modes`, `SlabMode::field`: each guided mode's $E_y$ (TE) or $E_x$ (TM) across silicon (3.473) on oxide (1.444) |
| `slab-bv` | `core` $n_2$ (3.473), `cover` $n_1$ (1.444) | `Slab::modes` at 300 values of $V$, as $b$; the cutoffs $m\pi + \arctan(r\sqrt a)$ |
| `slab-neff` | `core` $n_2$ (3.473), `cover` $n_1$ (1.444), `wavelength` µm (1.55) | `Slab::modes` at 300 thicknesses, each mode from its cutoff |
| `slab-confinement` | `core` $n_2$ (3.473), `cover` $n_1$ (1.444), `wavelength` µm (1.55), `thickness` nm (220) | `MultilayerMode::power_fractions` on the modes of `Slab::modes`, at 200 thicknesses |
| `strip-fields` | `width` nm (500), `thickness` nm (220), `wavelength` µm (1.55), `mode` (1) | `mode::vector::modes` on the four quarters of `StripWaveguide::cross_section` (10 nm cells), `VectorMode::fields`: $\lvert E_x \rvert$, $\lvert E_y \rvert$ and $\lvert E_z \rvert$ of the chosen guided mode as pictures |
| `strip-neff` | `thickness` nm (220), `wavelength` µm (1.55), `width` nm (500) | the same at 41 widths from 200 to 1200 nm (20 nm cells), `VectorMode::is_guided`; TE₁'s cutoff bisected to a nanometre |
| `strip-models` | `thickness` nm (220), `wavelength` µm (1.55), `width` nm (500) | `mode::hadley::modes` on `StripWaveguide::cross_section` (20 nm cells), `Ridge::mode`, `Rectangle::mode` and `closed_form`, and the device's analytic fit, at 16 widths from 250 to 1000 nm |
| `strip-corners` | `width` nm (500), `thickness` nm (220) | `mode::vector::modes` and `mode::hadley::modes` on `StripWaveguide::cross_section` at 11 grids from 40 to 10 nm, Richardson's extrapolation of Hadley's at its fitted order |
| `strip-group-index` | `thickness` nm (220), `wavelength` µm (1.55), `width` nm (500) | `StripWaveguide::dispersion` at its 3D fidelity (20 nm cells) at 13 widths from 300 to 900 nm, the effective index method's $n_g$, `Material::group_index` |
| `leaky-field` | `order` $m$ (4) | `Multilayer::bound_modes` and `modes_in` on Chilwell and Hodgkinson's four-layer guide, `MultilayerMode::field`: $\lvert E_y\rvert$ and $\operatorname{Re} E_y$ of a TE bound mode ($m \le 3$) or leaky wave ($m \ge 4$) |
| `leaky-substrate` | `oxide` $h$ µm (1), `wavelength` µm (1.55) | `Multilayer::mode_near` from `Slab::modes`: 220 nm of silicon under oxide on $h$ of oxide over silicon, at 57 thicknesses from 0.1 to 1.5 µm, TE and TM; the estimate $e^{-2\gamma h}$ |
| `bend-loss` | `core` $n_1$ (1.6), `cladding` $n_2$ (1.5), `thickness` µm (1), `wavelength` µm (1), `radius` µm (40) | `SlabBend::fundamental` and `mode_near` at 41 radii from Marcuse's $U = 3$ to 30, E normal to and in the bend plane; `marcuse_loss`, `bend_loss_db` |
| `fab-resolution` | `k1` $k_1$ (0.4), `feature` nm (500) | `fabrication::rayleigh::Optics` for each projection lithography of the process library, its cited wavelength and numerical apertures: $W = k_1\lambda/\mathrm{NA}$, and the $k_1$ the feature asks at the highest NA |
| `fab-oxidation` | `temperature` °C (1000), `hours` h (4) | `fabrication::oxidation::rates` and `grow`: Deal and Grove's law with their Tables I and II, dry and wet, and its linear and parabolic limits |
| `grating-angle` | `period` nm (630), `etch` nm (70), `fill` (0.5), `cladding` $n_c$ (1.444) | `Slab::modes`: TE₀ of 220 nm of silicon and of the grooves' thinner film on oxide, weighted by the fill; $n_c\sin\theta = n_\text{eff} - \lambda/\Lambda$ from 1.45 to 1.65 µm |
| `grating-overlap` | `mfd` µm (10.4), `angle` ° (10), `leakage` α per µm (0.13) | the overlap of $e^{-\alpha z}$ with the fibre's Gaussian on the surface, its axis placed where it couples most, summed on steps of $w/400$; Tamir and Peng's 80 % |
| `grating-apodization` | `mfd` µm (10.4), `angle` ° (10), `cap` α per µm (0.3) | Taillaert et al.'s $2\alpha = G^2/(1 - \int G^2)$, marched along $z$ with the power left and capped, and its overlap |
| `grating-directionality` | `box` µm (2), `cladding` (1.444), `angle` ° (10), `etch` nm (70), `mirror` pairs (0) | `Multilayer::reflection` above and below a sheet at the grooves' mid-depth: the share sent up against the buried oxide's thickness |
| `grating-band` | `mfd` µm (10.4), `angle` ° (10), `cladding` (1.444), `etch` nm (70), `fill` (0.5) | `Slab::modes` for the grating's index at each wavelength, the period phase matched at 1.55 µm; $e^{-q^2w^2/4}$ |
| `mmi-field` | `width` µm (3), `offset` $y_\text{in}/W$ (0), `span` in $L_\pi$ (1) | `circuit::components::Mmi::analysis_at`, `MmiModes::field`: the light a 0.8 µm guide's input carries along a 220 nm silicon section, from above, by the guided modes of the effective index method's plane; the shares of the odd modes and of modes 2, 5, 8 |
| `mmi-modes` | `width` µm (3), `wavelength` µm (1.55) | `Mmi::analysis`, `MmiModes::propagation_constants`: each mode's $(\beta_0 - \beta_\nu)3L_\pi/\pi$ against $\nu(\nu + 2)$; $L_\pi$ exact and by Soldano and Pennings's Eqs. 4 and 6 |
| `mmi-length` | `kind` (1: 1 × 2, 2: 2 × 2), `width` µm (3), `access` µm (0.8) | `Mmi::analysis`, `MmiModes::s_matrix` at 401 lengths to $1.25L_\pi$: each output, and the excess loss, imbalance and phase difference at the first two-fold image and where the weaker output peaks |
| `mmi-spectrum` | `kind` (1), `width` µm (3), `access` µm (0.8) | `Mmi::analysis` at 121 wavelengths from 1.40 to 1.70 µm (`material::silicon`, `material::silica`), cut at the best length at 1.55 µm; the band where excess loss and imbalance stay under 1 dB |
| `mmi-tolerance` | `kind` (1), `width` µm (3), `access` µm (0.8), `thickness` nm (220) | `Mmi::analysis_at` with the guides' centres in place and every drawn width $\Delta w$ wider, −250 to 250 nm, cut at the best length on 220 nm; Soldano and Pennings's Eqs. 39 and 40 beside it |
| `meta-orders` | `wavelength` µm (0.6), `period` nm (375), `pillars` $N$ (4), `angle` ° (0), `n_in` (1), `n_out` (1) | the grating equation $n_t\sin\theta_m = n_i\sin\theta_i + m\lambda/\Gamma$, $\Gamma = Np$, and the $N$-step staircase's Fourier series, $\mathrm{sinc}^2(m/N)$ where $m - 1$ is a multiple of $N$ |
| `meta-pillar` | `period` nm (600), `height` nm (1000), `wavelength` µm (1.55) | `fdtd::Simulation::new` on one period of silicon ridges on silica between periodic sides (`Edges::Bloch`, $k = 0$), 2D, 20 nm cells, a sheet of `Current` and `add_dft`, divided by the run with no ridge; `Slab::modes` for the waveguide picture $(n_\text{eff} - 1)k_0H$ |
| `meta-focus` | `na` (0.6), `focal` µm (20), `period` nm (600), `levels` $N$ (8; 0 for every phase) | a cylindrical lens's hyperbolic phase held over each pillar's cell and rounded to $N$ levels, propagated by the 2D Rayleigh–Sommerfeld integral in its far-zone form on steps of λ/20, summed in the studio |
| `meta-axis` | `na` (0.3), `focal` µm (150), `wavelength` µm (1.45) | the same lens designed at 1.55 µm, its wrapped phase scaled by λ₀/λ, the intensity along its axis; $f\lambda_0/\lambda$ |
| `meta-bandwidth` | `na` (0.24), `focal` µm (200), `thickness` µm (0.8), `index` (3.674), `centre` µm (1.454) | Presutti and Monticone's Eqs. 10–12: the bounds for resonant meta-atoms, Tucker's for waveguides and Miller's for any structure, against NA |
| `crystal-stack` | `high` $\varepsilon_1$ (13), `low` $\varepsilon_2$ (1), `fill` $d_1/a$ (0.2) | `Multilayer::reflection`: one period's $t$, $\cos Ka = \operatorname{Re}(1/t)$ at 2000 frequencies to $\omega a/2\pi c = 1$, the band edges bisected; the quarter-wave stack's gap |
| `crystal-group-index` | `high` $\varepsilon_1$ (13), `low` $\varepsilon_2$ (1), `fill` $d_1/a$ (0.2), `frequency` $\omega a/2\pi c$ (0.15) | `Multilayer::reflection` as above, $n_g = c\,dK/d\omega$ by a central difference of 10⁻⁶ of the frequency; $\sqrt{\langle\varepsilon\rangle}$ |
| `fdfd-convergence` | `polarization` (1 E along z, 2 H along z), `thickness` nm (220) | `fdfd::Solver2d::port_modes` of a silicon slab (3.476 in 1.444) at 1.55 µm on 11 grids of 4.4 to 128.4 cells across it, its faces 0.3 and 0.7 of a cell off the lines, each cell by 8 × 8 samples (`Solver2d::new`), its centre's material or its mean (`Solver2d::from_cells`); the error against `mode::slab::Slab::modes`, and each order between the grids of 64.4 and 128.4 cells |
| `fdfd-gradient` | `polarization` (1), `bump` $\varepsilon_b$ (6) | `Solver2d::from_cells` on the slab and bump of `fdfd/adjoint-gradient-ez`, 20 nm cells, `mode_source` and `solve_system`: the forward field, the right port's mode launched backwards, and `mode_power_gradient` in a window of 3,000 cells; a fourth-order central difference at one cell by `reuse_cells` as the check |
| `fdfd-difference-step` | `polarization` (1), `probe` (1 bump, 2 core, 3 oxide) | the same problem: `mode_power_gradient` at the cell against second- and fourth-order central differences of the power, 4 solves at each of 12 steps from 10⁻⁶ to 10⁻⁰·⁵; the round-off bound from `Solver2d::residual` |
| `crystal-bands` | `lattice` (1 square, 2 triangular), `kind` (1 rods, 2 holes), `eps` $\varepsilon$ (8.9), `radius` $r/a$ (0.2) | `fdtd::Simulation::smoothed` on one cell between Bloch-periodic sides, 2D, 24 cells a lattice constant, the subpixel average's diagonal; one complex run a k-point along the zone's edge, `fdtd::harmonic_inversion` on $E_z$ (TM) and $H_z$ (TE) probes; the triangular lattice on a rectangle of two sites, its folded bands cancelled by probes one lattice vector apart |
| `heater-power` | `kind` (1 TiN above a strip, 2 N++ in the slab), `strip` $w$ µm (0.5) | `device::ThermoShifter::metal` and `::doped` at the analytic fidelity (`Device::derived`), $P_\pi$, the 1/e time and $f_{3\,\mathrm{dB}}$ at 21 heights from 1 to 3 µm or 25 distances from 0.4 to 1.6 µm; $P_\pi\tau$ and $2\pi f_{3\,\mathrm{dB}}\tau$ |
| `mzm-transfer` | `length` µm (5800), `bias` $\varphi_0/\pi$ (0.5), `frequency` GHz (0) | `device::TflnModulator` at the 2D fidelity: `Device::component`'s $\lvert S_{21}\rvert^2$ at 81 voltages over $\pm 2V_\pi$, at the frequency and at DC; $V_\pi L$, $\lvert m(f)\rvert$ and the slope at the bias |
| `modulator-response` | `mismatch` $n_m - n_o$ (0.04), `loss` $a$ dB cm⁻¹ GHz⁻½ (0.42), `dielectric` $b$ dB cm⁻¹ GHz⁻¹ (0.025), `impedance` Ω (41.5), `length` µm (5800) | `electro::line::response` (Ghione's Eq. 6.31) between 50 Ω ends beside light of group index 2.26, from 1 to 300 GHz referred to 1 GHz, `falls_to` for the bandwidths; the limits $\lvert\sin U/U\rvert$ and $(1 - e^{-\alpha L})/(\alpha L)$ |
| `pcm-shift` | `thickness` nm (25), `width` µm (0.5) | `device::PcmShifter::sb2se3`, `::sb2s3` and `::gst` at the analytic fidelity: $\Delta n_\mathrm{eff}$ amorphous to crystalline at 17 film thicknesses from 10 to 50 nm, on a logarithmic scale; the crystalline loss, $L_\pi$ and the figure of merit at the thickness |
| `nl-miller` | `crystal` (1 congruent LiNbO₃, 2 MgO:LiNbO₃), `from` (1 0.852, 2 1.064, 3 1.313 µm) | `nonlinear::miller::miller_shg` on the catalogue's Shoji et al. tensors and the entries' default indices, against the fundamental from 0.82 to 2.2 µm; the measurements dashed |
| `nl-coupled` | `process` (1 SHG, 2 parametric gain), `efficiency` η %/(W·cm²) (4500), `power` mW (100), `length` µm (4000), `phase` ΔkL/2 (0) | `nonlinear::coupled::SecondHarmonic` and `ThreeWave` by RK4 on 2000 steps over the length; `undepleted_lossy_shg`, $\tanh^2$ and $1 + (\Gamma/g)^2\sinh^2 gz$ beside them |
| `nl-acceptance` | `fundamental` µm (1.064), `length` µm (1000), `variable` (1 wavelength, 2 temperature) | `nonlinear::phase::Uniaxial::catalogue` (Edwards and Lawrence's $n_e(T)$), `Device::quasi_phase_matched` with Jundt's expansion, `phase_tuned`, `response`, `acceptance` |
| `nl-qpm` | `order` $m$ (1), `duty` $D$ (0.5), `span` in $l_c$ (12) | the domains' $\int d(z)e^{-i\Delta kz}dz$ in closed form, summed in the studio on steps of $l_c/40$; `fourier_coefficient` for $G_m$ |
| `nl-ridge` | `width` µm (1.24), `etch` nm (50), `fundamental` µm (1.57) | `nonlinear::guide`: Zhao et al.'s x-cut MgO:LiNbO₃ ridge, `Ridge::cross_section` on 20 nm cells behind a mirror wall, `fundamental`, `ModeField`, `second_harmonic_coupling` with $d_{33}$ = 27 pm/V; $\lvert E_x\rvert$ of both TE₀ modes as pictures, the period, $A_\text{eff}$ and η |
| `nl-spm` | `phase` $\varphi_\mathrm{max}$ (4.5π) | `nonlinear::pulse::Guide::propagate` on a Gaussian without dispersion, 1024 points across 64 T₀, `Pulse::spectrum`, `rms_bandwidth`; `spm_rms_broadening` |
| `nl-soliton` | `order` $N$ (3), `distance` $z/z_0$ (0.5) | `Guide::propagate` on `Pulse::sech` (β₂ = −0.05 ps²/m, γ = 1 /(W·m), T₀ = 100 fs), 2000 split steps a soliton period, 1024 points across 50 T₀ |
| `nl-fwm` | `power` W (1), `gamma` /(W·m) (1), `length` m (2), `beta2` ps²/m (−0.05), `beta4` 10⁻⁴ ps⁴/m (0) | `nonlinear::fwm::signal_gain`, `conversion`, `peak_detuning`; `Dispersion::four_wave_mismatch` |
| `nl-lle-states` | `drive` $f^2$ (4), `detuning` $\zeta_0$ (3) | `nonlinear::resonator::homogeneous_powers` at 1201 detunings, `instability_rate` at each state |
| `nl-comb` | `detuning` $\zeta_0$ (10), `drive` $f$ (3), `d2` (0.004) | `Resonator::quadratic`, `soliton`, `soliton_field` and `settle` on 512 lines, steps of 0.002, `peak_width`, `max_detuning`; the lines by a discrete Fourier transform |
| `nl-jsa` | `pump` nm (0.6), `length` µm (10000), `idler` $g_i$ (2.25), `matching` (1 sinc, 2 Gaussian) | `nonlinear::pairs::joint_spectrum` and `schmidt` on 201 × 201 frequencies of a model guide (linear indices, $g_s$ 2.30, $g_p$ 2.40), `gaussian_purity`; the picture by `Device::phase_at_frequencies` on even wavelengths |
The ranges are in `charts.rs` (the coupler's in `charts/coupler.rs`), and the window's sliders keep
to them. A chart may draw its y axis on a logarithmic scale (`y_log`), and give curves that belong
together one colour (`colour`, as the slab's charts give a TE and a TM mode of one order). The
material charts (`charts/material.rs`) evaluate the catalogue's models, exactly, in milliseconds. The
slab's charts are exact and take tens of milliseconds, so they redraw as a slider moves. The coupler's charts solve
modes, a tenth of a second to a second each; their solves are cached, so moving a slider that
changes no geometry redraws at once. The strip's charts (`charts/strip.rs`) solve the strip
waveguide device's own cross-section, so a lesson and the device's page can't disagree: one to
five seconds for a sweep, cached likewise, their width marker moving along the sweep's own widths.
The leaky and bent guides' charts (`charts/leaky.rs`) are exact, transfer matrices and the
bent slab's radial shooting, and take milliseconds. The fabrication charts (`charts/fabrication.rs`)
are closed forms evaluated with the process library's cited numbers: exact, with no grid. The
grating coupler's charts (`charts/grating.rs`) take tens of milliseconds to a second: exact slab
modes and transfer matrices, and overlap integrals summed on fine steps; each is a model of one
factor of a coupler's efficiency, set in the lesson against a published number. The two overlap
charts, `grating-overlap` and `grating-apodization`, are integrals the studio sums itself, with no
library function behind them, and their captions say so. The MMI's charts (`charts/mmi.rs`) are its 2D guided-mode model, exact slab modes and no grid,
solved once for a width and wavelength so that a sweep of the length costs one solve.
The photonic crystal's charts (`charts/crystal.rs`) are of two kinds: the 1D crystal's, exact
transfer matrices in milliseconds, and the 2D crystal's, FDTD runs between Bloch-periodic sides,
one a k-point, about a second for a band diagram, cached by geometry; their gaps are checked against
Joannopoulos et al.'s in unit tests.
The metasurface's charts (`charts/meta.rs`) are of three kinds: exact closed forms (the grating's
orders and the bandwidth bounds) in milliseconds; scalar 2D propagation of a lens's phase, summed in
the studio with no library function behind it, in tens of milliseconds; and the pillar's 2D FDTD
unit cells, 31 runs between periodic sides in parallel, a few seconds, cached by geometry, checked
in unit tests against the transfer matrices for a film and against a finer grid.
The FDFD lesson's charts (`charts/fdfd.rs`) are 2D FDFD solves on the problems of the validation
cases: the port modes on 11 grids three ways, in parallel; the gradient's 26,600 unknowns
factorized once, with 4 more factorizations for its check; the step study's 48 factorizations in
parallel. Their three unit tests, which compute all of them, take 3.1 s together on 6 threads; each chart is cached by its
parameters, and checked against the exact slab's orders and against fourth-order differences to
the validation cases' 10⁻⁶.
The modulators' charts (`charts/modulator.rs`) call the library's active devices and the
electrode's response: the heaters' and the phase-change cells' closed forms, exact and in
milliseconds; the TFLN modulator's 2D cross-section, solved once, in seconds, and cached
by the device, so its voltage, bias and frequency redraw at once; and Ghione's response, exact.
Their unit tests check each curve against the device's own figures, the examples' printed numbers
and the response's closed-form limits.
The nonlinear lesson's charts (`charts/nonlinear.rs`) call the library's `nonlinear` module: closed
forms and phase matching (Miller's rule, the acceptance, four-wave mixing's gain, the
Lugiato–Lefever equation's uniform states), exact and in milliseconds; RK4 and split-step
integrations (the coupled-wave equations, the NLSE, a settled cavity soliton on 512 lines), tens
of milliseconds to a second, the soliton cached; a joint spectrum's singular values on 201 × 201
points; and the thin-film ridge's two full-vector modes on 20 nm cells, about a second, cached by
geometry. `nl-qpm` sums the harmonic over the domains in the studio, with no library function
behind it, and its caption says so. Their unit tests, 0.9 s together on 6 threads, check each
against the write-up's numbers (Fejer et al.'s Table I, Zhao et al.'s period and areas, the
job's η, the soliton's table) or a closed form.
A chart may draw pictures of a field over a cross-section in place of curves (`maps`), side by
side with one colour scale, as `strip-fields` does.

## Checks

`cargo test -p photonoxide-studio` fails when a lesson:

- doesn't parse, or, written, lacks an intuition, a theory or a research section, or a timeline;
- is coming soon and has an opening, a section written, sections not numbered 1, 2, 3 in order, a
  reading time or charts, an example no subsection's `::coming` line names, or a milestone the
  roadmap doesn't have;
- has a section coming soon with more than its `::coming` line and one paragraph, a paragraph
  longer than a line or two (320 characters), or an example its front matter doesn't list;
- shares its file's name with a method's write-up, so a link to it would be ambiguous;
- names an example, job, circuit, method write-up, validation case, diagram, chart or lesson that
  doesn't exist, or a block names one its front matter doesn't list;
- opens a device (`::device`) that isn't in the component library, or whose page names another
  lesson as its own;
- plays a flow (`::fabrication`) of a device that isn't in the component library, or one the
  fabrication engine can't make, or whose last step isn't the design to the bit;
- is written and has no diagram, or a chart before its first diagram, or a diagram whose 3D view is
  of a job its front matter doesn't list;
- has a chart block with an unknown parameter or a value outside its range;
- has a paper without a DOI, or its text links to a DOI its papers don't list, or a relative
  link leads nowhere;
- has unpaired math delimiters.

It fails too when a diagram has no drawing in `studio/src/components/diagrams/`, or names a job
that isn't built in. Every chart has a unit test against the library function it wraps. `pnpm build` in `studio/`
renders every formula of every lesson with KaTeX and fails on an error (`vite.config.ts`).

## Writing a lesson

- **Our own words and figures.** Papers are cited, never copied.
- **Every equation names its source** by DOI, checked on Crossref
  (`https://api.crossref.org/works/<doi>`), or is derived in the lesson.
- **Every number has its grid**, or says it is exact, and comes from an example's output or the
  validation report, shown by an `::example` or `::validation` block rather than retyped where
  that is possible. A 2D result is never a device's performance.
- New papers go into tachsin's papers list, ticked or needed, so their PDFs can be fetched.
- Add the file to `lessons!` in `studio/src-tauri/src/academy.rs` where it falls in the reading
  order, and a row to the tables above; a lesson coming soon moves to the first table once written.

---
title: "Component generators with ports"
module: generator
summary: "Photonic components drawn from their parameters, each a plain function giving exact shapes on layers and named ports (position, direction, width, layer): straights, tapers, arc and partial Euler bends, S-bends, double spirals, rings, racetracks and disks with their buses, straight and bent directional couplers, N × M MMIs, Y-branches, crossings, Bragg gratings (uniform, apodized, sampled), subwavelength gratings, grating couplers, photonic crystals with L3, H1 and W1 defects, and inverse-taper edge couplers. The devices draw their geometry with them, to the bit; jobs place them by name and find their ports by name for FDFD ports and FDTD sources and monitors."
order: 1.54
papers:
  - cite: "F. Vogelbacher, S. Nevlacsil, M. Sagmeister, J. Kraft, K. Unterrainer, R. Hainberger, Opt. Express 27, 31394 (2019) (the partial Euler bend: its lengths and ends, Eqs. (5)–(11))"
    doi: 10.1364/OE.27.031394
  - cite: "M. Cherchi, S. Ylinen, M. Harjanne, M. Kapulainen, T. Aalto, Opt. Express 21, 17814 (2013) (the Euler bend, p = 1)"
    doi: 10.1364/OE.21.017814
  - cite: "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995) (an MMI's beat length and self-imaging lengths, Eqs. 4, 6, 24, 33 and 37, as geometry inputs)"
    doi: 10.1109/50.372474
  - cite: "X. Wang, W. Shi, R. Vafaei, N. A. F. Jaeger, L. Chrostowski, IEEE Photon. Technol. Lett. 23, 290 (2011) (uniform and sampled Bragg gratings with sidewall corrugations, drawn)"
    doi: 10.1109/LPT.2010.2103305
  - cite: "Y. Akahane, T. Asano, B.-S. Song, S. Noda, Nature 425, 944 (2003) (the L3 cavity and its shifted end holes)"
    doi: 10.1038/nature02063
validation:
  - generator/ports
  - generator/lengths-and-areas
  - generator/gaps
  - generator/symmetry
  - generator/devices
  - generator/convergence
  - generator/job-expressions
  - generator/job-ports
  - generator/wang-bragg
---

A component is drawn the same way wherever it is used: on a device's page, in a job, in a
circuit's chip, and later in 0.9's layout. `photonoxide::generator` holds one generator per
component, a plain struct of its parameters whose `draw(layer)` gives a `Drawing`: its shapes,
each on a layer with its etch, and its ports. The shapes are the exact shapes of
`photonoxide::geometry` (docs/methods/geometry.md): sectors and rings, paths of arcs, clothoids
and Bézier curves, regions between two curves, polygons. Nothing is polygonized until a solver or
a layout asks, with its tolerance.

## Ports

A `Port` is where a guide crosses the component's edge:

| field | what |
|---|---|
| `name` | unique in the component, without dots: a job and a netlist name it `component.port` |
| `position` | the centre of the guide's end |
| `direction` | the way light leaves, degrees counterclockwise from +x, in [0, 360) |
| `width` | the guide's width there |
| `layer` | the layer the guide is drawn on |
| `modes` | how many modes it carries, where that is known (a generator draws without a stack, so it leaves this to a device or a mode solve) |

A port's position, direction and width come from the component's closed form, not from its
drawn outline: an arc of radius R through θ ends at (R sin θ, R(1 − cos θ)) facing θ, and the
port says exactly that. The validation checks that the drawn shapes agree: each guide's edges,
±w/2 across the port, lie on the shapes' boundary, a point 1 nm inside is inside and one 1 nm out
is outside.

A drawing moves as a whole (`Drawing::transformed`): shapes by the exact transforms of the
kernel, ports turned with them. Quarter turns and mirrors in x or y keep directions exact
(0, 90, 180, 270 stay whole numbers); any other angle turns them to round-off.
`Drawing::connected(port, to, mirrored)` turns and moves a drawing so that its port meets
another's: on it, facing it. `Port::meets` says whether two ports meet, and
`Drawing::circuit_ports` gives the ports as a circuit's (`crate::circuit::Port`), the same names in
the same order, so a netlist's `instance.port` and a drawing's port are one name.

## The first set

Each component is drawn in its own frame, its first port at or left of the origin; lengths are
µm, angles degrees.

| generator | drawn as | ports |
|---|---|---|
| `Straight` (w, L) | a rectangle from x = 0 to L | `o1` (0, 0) 180°, `o2` (L, 0) 0° |
| `Taper` (w₁, w₂, L, profile) | linear: a trapezoid; exponential w₁(w₂/w₁)^(x/L), parabolic √(w₁² + (w₂² − w₁²)x/L), or any w(x): the region between −w(x)/2 and w(x)/2 | `o1` (0, 0) 180° w₁, `o2` (L, 0) 0° w₂ |
| `Bend` (w, R, θ, p) | p = 0: an arc (a sector); 0 < p ≤ 1: a partial Euler bend of effective radius R (Vogelbacher et al. 2019), clothoid, arc, clothoid, its ends the arc's | `o1` (0, 0) 180°, `o2` (R sin\|θ\|, ±R(1 − cos θ)) θ |
| `SBend` (w, L, h, kind) | cosine y = h(1 − cos πx/L)/2; sine y = h(x/L − sin(2πx/L)/2π); Euler: two partial Euler bends through ±2 atan(h/L) of radius L/(2 sin θ); Bézier through (L/2, 0), (L/2, h) | `o1` (0, 0) 180°, `o2` (L, h) 0° |
| `Spiral` (w, d, R₀, n, 2c, lead) | a double spiral of semicircles, below | `o1`, `o2` outside, half a turn apart about the origin |
| `Ring` (w, R, gap, L_c, gap_d) | the bus along y = 0 from −reach to reach; a ring (an annulus about (0, w + gap + R)) or a racetrack (two half rings L_c apart and two straights); a drop bus gap_d above it | `in` (−reach, 0) 180°, `through` (reach, 0) 0°, `add` (reach, y_d) 0°, `drop` (−reach, y_d) 180°, y_d = 2(w + R) + gap + gap_d |
| `Disk` (w, R, gap, gap_d) | the bus, and a disk about (0, w/2 + gap + R) | as the ring's, y_d = w + 2R + gap + gap_d |
| `DirectionalCoupler` (w, gap, L, arms) | two guides at y = ±(gap + w)/2 from 0 to L; S-bend arms if asked, moving each guide out by h over L_a | `o1` lower left, `o2` upper left (180°), `o3` upper right, `o4` lower right (0°) |
| `BentCoupler` (w, gap, R, θ) | two concentric arcs about (0, −R), radii R and R + w + gap, spanning θ about +y | `o1`, `o2` at the left ends facing 180° + θ/2; `o3`, `o4` at the right facing −θ/2 |
| `Mmi` (W, L, N, M, w_a, w_t, L_t) | a W × L rectangle; N inputs and M outputs tapering from w_a to w_t over L_t, their centres given or at W(k + ½)/N − W/2 | `o1`…`oN` left, bottom to top; `o(N+1)`… right, top to bottom (Soldano & Pennings' numbering) |
| `YBranch` (w_in, w, L, gap, L_a, s, outline) | a stem from w_in to 2w + gap, linear or along w(x); two Bézier arms from it, s apart at their ends | `o1` (0, 0) 180°, `o2` (L + L_a, s/2), `o3` (L + L_a, −s/2) 0° |
| `Crossing` (w, a, w_c) | two bars through the origin, each tapering from w at its ends to w_c at the centre | `o1` left, `o2` top, `o3` right, `o4` bottom |
| `BraggGrating` (w, Δw, Λ, d, N) | a core w − 2Δw wide (recessed) or w − Δw (centred), and on each side a tooth Δw(x) high over the middle d Λ of each period; sampled: teeth in the first n₁ periods of every n₀ | `o1` (0, 0) 180°, `o2` (NΛ, 0) 0°, the core's width |
| `SwgWaveguide` (w₀, w₁, Λ, d, N, bridge) | segments d Λ long at the start of each period, their widths linear from w₀ to w₁, on a bridge if asked | `o1` (0, 0) 180°, `o2` at the last segment's end |
| `GratingCoupler` (w, W, L_t, Λ, F, N, depth) | a linear taper from w to W, then N teeth F(x) Λ long across W, partly etched (or fully, on a top layer over a slab layer) | `o1` (0, 0) 180° |
| `PhotonicCrystal` (lattice, a, r, sites, defect) | a slab less a square or hexagonal lattice of holes, with an L3 (three holes out, the end ones shifted), H1 (one out) or W1 (a row out) | W1: `o1`, `o2` at the slab's ends, √3a − 2r wide (2a − 2r square) |
| `EdgeCoupler` (w_tip, w, L_t, L_s) | an inverse taper from the tip at the facet to the guide, any taper profile, and a straight | `o1` the tip (0, 0) 180°, `o2` (L_t + L_s, 0) 0° |

The names are the circuit's where the circuit has the component: the rings' `in`, `through`,
`add`, `drop`, the coupler's and the MMI's `o1`… and the Y-branch's `o1`, `o2`, `o3`.

**The double spiral.** Its centre is an S through the origin heading +x. Arm B leaves (c, 0)
and turns left through semicircles of radii r₁ = R₀ and r_k = 2R₀ + (k − 3/2) d for k = 2 to n,
joined by straights 2c long, and leaves the last along a straight 2c + lead. Arm A is B turned
half a turn about the origin; the path runs in along A and out along B. From B's second
semicircle on, each semicircle is concentric with A's beside it and the guides are d apart,
centre to centre; the first turn is eccentric (from d at its foot to 2R₀ at its top), so the
generator asks 2R₀ ≥ d. Its length is 2c + 2(Σ π r_k + 2(n − 1)c + 2c + lead). The path itself
refuses an outline that crosses itself, so a spiral that drew is one that doesn't meet itself.

**MMI lengths.** An MMI's length is a geometric input here. `Mmi::beat_length` gives Soldano &
Pennings' estimate L_π ≈ 4 n_r W_e²/(3λ₀), with the effective width
W_e = W_M + (λ₀/π)(n_c/n_r)^{2σ}(n_r² − n_c²)^{−1/2} (their Eqs. 4 and 6), and
`SelfImaging::length` the first N-fold image: (1/N) 3L_π for general interference (Eq. 24),
(1/N) L_π for paired (Eq. 33), (1/N) 3L_π/4 for symmetric (Eq. 37). These are paraxial estimates
for choosing a length, not the device's performance, which `crate::circuit::components::Mmi`
computes from the exact slab modes and the solvers from the drawn geometry.

**Gratings.** A Bragg grating's teeth stand in the middle of their periods, so it ends on its
core at both ends and is its own mirror image about L/2. Its teeth are an `Array` of
rectangles over the core, none touching another, so every answer is exact and a point tests a
handful of the 4000 teeth of a 620 µm grating, through their index (docs/methods/arrays.md).
Apodized, each tooth's height is Δw(x) at its centre, an expression of `x`; a sampled grating
keeps its teeth in bursts. A grating coupler's fill F(x) is an expression of each tooth's start.
Drawn on one layer, its teeth are partly etched, and the stack leaves the layer's material below
their floor across the whole layer, as a rib's slab (docs/methods/stacks.md); with a slab layer,
the teeth are the top layer, fully etched, over a slab of the grating's extent.

**Photonic crystals** take their lattice and defects from docs/methods/arrays.md: a slab less a
lattice of holes, an exact boolean. Akahane et al.'s L3 is three holes out of a row with the end
ones moved out by 0.15a.

### Wang et al. 2010's gratings

`generator/wang-bragg` draws the gratings of Wang et al. (doi:10.1109/LPT.2010.2103305) from what
they state: strips 500 nm wide (Fig. 1), corrugations recessed into both sidewalls by the designed
Δw of 10 to 30 nm on each side (Figs. 3 and 4), period 310 nm, 50 % duty, 620 µm long (Section
II), and the sampled gratings of Section III, bursts of 20 periods (Z₁ = 6.2 µm) every Z₀ = 62 µm,
ten of them, at Δw = 20 and 40 nm (Fig. 6). It checks every tooth's place and size, the length,
the guide's 500 and 500 − 2Δw nm by inside tests, the area L(w − Δw), and the bursts' Z₁, Z₀,
count and Z₀/Z₁ = 10, the peaks their Eq. 4 counts in the 3 dB envelope. This is geometry only.

Their spectra are not reproduced. Their bandwidths and Bragg wavelengths are measured on
fabricated gratings whose corrugations are rounded and smaller than drawn (Fig. 1: 25 nm designed,
about 20 nm made; their "effective" square corrugation is about a quarter of the designed one,
Fig. 4), and the paper doesn't state the cladding. Coupled-mode theory on photonoxide's own
full-vector modes (oxide-clad 220 nm SOI, cells fitted about 10 nm, Hadley's equations; κ =
2Δn/λ for the square, half-duty corrugation, and their Eq. 2) gives a bandwidth of 7.3 nm for a
10 nm corrugation on each side, where their own simulation of a square corrugation (Fig. 4, read
from the plot) gives about 2.1 nm and the measurement 0.8 nm; and a Bragg wavelength of 1524 nm
where they measured about 1500 nm. The model and the paper's simulation differ by a factor of 3 in
κ that the paper doesn't give enough to resolve (its simulation's cladding and how it takes κ), so
no number of theirs stands in the report as reproduced.

## The devices draw with them

Each device in `photonoxide::device` draws its geometry with its generator: the strip waveguide
with `Straight`, the bend with `Bend::arc`, the directional coupler with `DirectionalCoupler`, the
rings with `Ring`. One generator per component, not two. They draw what the devices drew
before, to the bit: the validation case `generator/devices` writes the earlier geometry out and
compares every shape and port at twelve designs (lengths and angles of zero included).

## In jobs

`"structure"`, `"fdfd"` and `"fdtd"` jobs take `[[task.component]]` tables: a `type`, a `name`
for its ports, a `layer` (the task's by default), the generator's parameters, each an expression
of the task's parameters with its unit (or a number in a `_um` or `_deg` key), an `etch`, and a
place: `origin`, `rotate` and `mirror`, or `connect = ["o1", "ring.through"]` to put one of its
ports on an earlier component's.

```toml
[[task.component]]
type = "ring"
name = "ring"
width = "w"
radius = "5 um"
gap = "gap"
drop_gap = "gap"

[[task.component]]
type = "straight"
name = "lead"
width = "w"
length = "3 um"
connect = ["o1", "ring.through"]

[[task.port]]          # an fdfd job's port at the ring's input
port = "ring.in"
```

A port found by name stands for the fields it replaces: an FDFD port's column at the port's x, its
side the one light enters from (the port must face 0° or 180°), and its window across the guide,
unless given: the guide's width and 1 µm each side, no closer than half way to another port in
the same column, inside the job's window. An FDTD `"mode"` source or monitor takes `port` the same
way on a plane normal to x or y, a source launching into the component and a monitor named after
its port. `job::check` names a component and its parameter in what it refuses
(`[[task.component]] #1 (ring "ring"), gap: needs a length, got a number`), an unknown port by
the ports there are, and a port facing the wrong way for the plane it would need.
`job::preview_ports` gives the components' named ports, which a run records after its scene
(`Event::Ports`) and the studio marks on its previews.

## Validation

| case | what | measured | tolerance |
|---|---|---|---|
| `generator/ports` | 26 components' ports against their closed forms, and on their guides' ends | 1.5e-15 µm | 1e-12 |
| `generator/lengths-and-areas` | areas against closed forms: tapers' three profiles, arcs, partial Euler bends and an Euler S-bend against Vogelbacher et al.'s lengths, a cosine S-bend (Simpson), the spiral, rings, racetracks, disks, an MMI, a crossing, a bent coupler, Bragg gratings, a subwavelength grating, a grating coupler, an L3 slab, an inverse taper | 1.6e-15 relative | 1e-9 |
| `generator/gaps` | rings', racetracks', disks', couplers' and a Y-branch's gaps by signed distance | 1.2e-15 µm | 1e-12 |
| `generator/symmetry` | couplers, an MMI, a Y-branch, a racetrack, a Bragg grating and an L3 slab mirrored, a crossing and a square-lattice H1 slab turned a quarter, at 4000 points each | 0 points | 0 |
| `generator/devices` | the devices' geometry against what they drew before | 0 bits | 0 |
| `generator/convergence` | a ring, a half ring, an Euler bend, a cosine S-bend, an exponential taper and an L3 slab as polygons from 1 to 0.001 nm | order 2.00 | 2 ± 0.15 |
| `generator/job-expressions` | 24 components in a job, every parameter an expression, against the generators with numbers | 0 bits | 0 |
| `generator/job-ports` | a 2D FDFD job's S-parameters with its ports by name against the same job written out, 50 nm grid | 0 bits | 0 |
| `generator/wang-bragg` (published) | Wang et al. 2010's uniform and sampled gratings drawn: teeth, length, widths, area, bursts | 1.1e-13 µm | 1e-9 |

The gaps are exact because each gap is built into the definition (the ring's centre at
w + gap + R above the bus) and measured by the exact signed distance of a circle, a sector or a
rectangle. The convergence is the kernel's: chords of length h stray from an arc of curvature κ by
κh²/8, so the polygons' distance from the true boundary falls as the square of their vertices'
spacing.

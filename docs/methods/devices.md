---
title: "Devices"
module: device
summary: "A component that knows its geometry: a stack, the parameters of its shapes, its models from a fit to the solver up to 3D, and the component each gives, with every figure's fidelity and grid. The strip waveguide, bend, directional coupler and rings."
order: 22.5
papers:
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    doi: 10.1017/CBO9781316084168
  - cite: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue)"
    doi: 10.1002/lpor.201100017
  - cite: "B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, 998 (1997)"
    doi: 10.1109/50.588673
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008)"
    doi: 10.1109/JLT.2008.923643
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1210 (2002)"
    doi: 10.1109/JLT.2002.800361
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002)"
    doi: 10.1109/JLT.2002.800371
  - cite: "M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975)"
    doi: 10.1109/JQE.1975.1068563
  - cite: "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971)"
    doi: 10.1002/j.1538-7305.1971.tb02620.x
  - cite: "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977)"
    doi: 10.1364/AO.16.000113
  - cite: "S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015)"
    doi: 10.1109/JLT.2015.2476603
  - cite: "R. E. Wengert, Commun. ACM 7, 463 (1964)"
    doi: 10.1145/355586.364791
validation:
  - device/library-components
  - device/ring-closed-forms
  - device/coupler-closed-forms
  - device/derivatives
  - device/waveguide-fit-neff
  - device/waveguide-fit-ng
  - device/waveguide-3d-order
  - device/bend-analytic
  - device/bend-2d
  - device/bend-3d-grid
  - device/coupler-fit
  - device/coupler-3d-order
  - device/ring-fsr
  - device/threads
  - device/bend-ring-slab
  - device/strip-chrostowski
  - device/coupler-chrostowski
  - device/ring-q-bogaerts
  - device/wire-ng-dwivedi
---

A [component](components.md) maps its parameters to an S-matrix, and its physical inputs are
fixed when it is built: a directional coupler's supermodes, a bend's radius, a ring's guide. A
**device** (`photonoxide::device`) builds them from a geometry. It joins four things:

- a layer stack (`stack::LayerStack`), one of whose layers is the core;
- the parameters of the shapes drawn on it: a width, a thickness, a gap, a radius, each with its
  unit, default and range;
- the models that compute it, from a fit to the solver up to 3D, each with its write-up, papers,
  stated error against a higher model and the range it holds over;
- the component each model gives, built by the circuit's own builders.

The studio can then show, change, sweep and optimize the geometry, not fixed constants; the
first four devices are a silicon strip's: the waveguide, the bend, the directional coupler and
the ring, all-pass and add-drop.

## The abstraction

A `Device` has seven methods:

| Method | What it gives |
|---|---|
| `kind()`, `about()` | the component's kind (`"waveguide"`, …) and a sentence |
| `parameters()` | name, unit, default, range: a `circuit::Parameter` each |
| `stack(values)` | the stack, the core layer at the device's thickness |
| `geometry(values)` | a `stack::Structure` of shapes on the stack, and the ports: name, position, direction (degrees, the way light leaves), width and layer |
| `fidelities()` | a `Method` per model: its fidelity, what computes it, this write-up, its papers, its `Stated` error against a higher fidelity and its validity ranges |
| `component(values, fidelity)` | a `Built`: the boxed component, the values of its own parameters, their derivatives, and the figures |
| `derived(values, fidelity)` | the figures: n_eff, n_g, the cross-over length, the free spectral range, Q, a bend's loss, each a `Figure` with its fidelity and grid |

The **values** (`Values`) are the parameters' in their units, in order; `Values::new` takes
changes by name and checks each against its range. A **design** (`Design`) gives them as
[expressions](expressions.md) of named parameters, `w = "450 nm"`, `gap = "w/2.5"`: each
device parameter not named takes its default (as a plain number, so it reads back to the bit),
an expression's value is in its parameter's unit (a length in µm, and an angle in radians turned
into degrees for a parameter in degrees), and a length given to an angle, or the reverse, is
refused. `Design::with` sets a named parameter and evaluates those that use it again (a sweep's
point); `Design::sensitivities` gives the derivatives of every device value with respect to a
named parameter, by the expressions' forward mode.

A device's **component** is the one the circuit has always had: a `Waveguide`, a `Bend`, a
`DirectionalCoupler`, an `AllPassRing` or `AddDropRing`, with its own parameters (a length, an
angle, a round trip and a coupling). `Built::values` are those parameters' values at the
device's (a ring's round trip $2\pi R + 2L_c$), so the device's S-matrix is the component's at
them (`device::s_matrix`). `Built::derivatives` holds, for each device parameter, the
derivatives of those values with respect to it, exactly; `None` for a parameter the component is
built from (a width, a gap), whose derivative needs another component. `device::derivative`
chains the component's exact $\partial S/\partial\theta$ ([circuit-adjoint.md](circuit-adjoint.md)),
the device's and the expressions' into $dS/dp$ for any named parameter $p$, or says there is
none.

**Every figure names its model and its grid.** A closed form's is "exact: no grid"; a fit's names
the solver and the grids it was fitted to; a 2D figure says it is 2D and is never the device's
performance (the bend's 2D index is the effective index method's); a 3D figure gives the
cross-section's grid, "nx × ny cells, h nm" with h the smallest spacing.

## What the solver sees

A device's 3D models solve its cross-section, cut from its own structure: the stack with the
core at its thickness, and its guides drawn on the core layer, each cell taking the material at
its centre (`Structure::material_at`), each material's permittivity computed once per
wavelength. The cross-section's x runs across the chip, 0 on the device's vertical plane of
symmetry, and its y up the stack from the core layer's mid-plane. A `Mesh` says where the nodes
are:

- **fitted** (`Cells::Fitted { step, margin }`): along each axis the uniform spacing nearest
  `step` that puts a strip's faces on nodes, $d = (w/2)/\operatorname{round}(w/2\,/\,\text{step})$,
  and $\operatorname{round}((w/2 + \text{margin})/d)$ cells from the centre, as the `mzi_dwivedi`
  example fitted its grids; a coupler's gap, guides and margin each have their own spacing, so
  that every face is on a node; a bend's window is a whole number of widths inward (short of the
  bend's centre) and outward past the radiation's turning point, so that halving the step halves
  every cell and keeps the window;
- **window** (`Cells::Window`): a fixed window centred on the device, uniform cells.

On the mirrored domain (`Symmetry::Mirrored`, the default) the solve takes a quarter of a strip
or a coupler, an electric wall on the vertical plane (a magnetic one for a coupler's odd
supermode) and a magnetic wall on the core's mid-plane, which keep the TE-like modes; a bend's
half, the mid-plane wall only. The cut checks that the structure is mirrored across those
planes at every cell, and refuses a stack that isn't (silicon under air: the full domain then,
`Symmetry::Full`). The scheme is the [standard full-vector one](vector.md) or
[Hadley's](hadley.md), which needs a uniform grid along each axis and no PML: the fitted single
strip, by default.

The defaults are 220 nm SOI (`LayerStack::soi_220`: Li's silicon, Malitson's silica) and cells
fitted about 10 nm apart with 1 µm of oxide beyond the core, by Hadley's equations for the strip
(20 nm, standard, for the bend's window).

## The analytic fits

The analytic models need the strip's mode and the pair's coupling at any width and thickness,
without a solve. They come from tables of the 3D model's own values on 220 nm-class SOI at
1.55 µm, interpolated by Lagrange polynomials through every node, one variable at a time:

- n_eff and n_g of the TE-like mode on a 5 × 5 grid of widths (400 to 600 nm) and thicknesses
  (200 to 240 nm), by Hadley's equations on the fitted 10 nm mesh, n_g from five wavelengths
  25 nm apart;
- the coupling $C = \pi\Delta n/\lambda$ of two strips (Chrostowski & Hochberg, Eq. 4.3), its
  logarithm, on a 5 × 3 × 5 grid of widths, thicknesses (200, 220, 240 nm) and gaps (100 to
  500 nm), from the even and odd supermodes by the standard scheme with mirror walls; and its
  decay $\gamma = -d\ln C/d\,\text{gap}$, the interpolant's derivative (8.8/µm at 200 nm: C
  falls e-fold every 114 nm).

Each is shifted to agree, at 500 × 220 nm and a 200 nm gap, with the numbers the studio's
library has always used: n_eff 2.44506 and n_g 4.172901 (the `group_index` example, with
Chrostowski & Hochberg's silicon and 1.444 oxide on a 6.25 × 5 nm grid) and C = 0.04228/µm (the
`directional_coupler` example, a 5 nm grid). The shifts are −5e-5 in n_eff, −0.0164 in n_g (the
book's silicon is less dispersive than Li's) and −0.8 % in C. So at its defaults every analytic
model is the library's component to the bit, and its stated error against 3D is mostly the shift.
The fits refuse a width, thickness or gap outside their tables, a band centre outside 1.5 to
1.6 µm, and a stack whose core, claddings or background differ from silicon's and silica's
indices by more than 0.01: on another platform, solve the modes.

## Waveguide

`StripWaveguide`: ports `o1`, `o2`; parameters `width`, `thickness`, `length`, `loss` (dB/cm:
what the mode solvers don't give, roughness and absorption), and the band its 3D model samples,
`wavelength` (the centre) and `span`. Its component is a `Waveguide`,
$S_{21} = e^{i\gamma L}$ ([components.md](components.md)), its parameters the device's `length`
and `loss`.

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| analytic | $e^{i\gamma L}$ on the fit's n_eff and n_g at 1.55 µm, first order in λ; or on a dispersion stated for its geometry (`with_guide`) | n_g within 0.02 of 3D (n_eff within 1e-4) | widths 400–600 nm, thicknesses 200–240 nm, 1.5–1.6 µm |
| 3D | the full-vector solver's mode at five wavelengths span/4 apart (`Dispersion::from_hadley` or `Waveguide::from_modes`) | — | λ₀ ± span/2 |

The 3D model's n_eff converges at second order by Hadley's equations (1.92 from 12.5 × 10 to
3.125 × 2.5 nm); the standard scheme, which the strip's corners hold to about first order and
less on practical grids (2.447067, 2.445713 and 2.444396 at 20, 10 and 5 nm: `strip_waveguide`),
is there for windows Hadley's equations can't take.

## Bend

`StripBend`: ports `o1`, `o2`; parameters `width`, `thickness`, `radius` (to the centre line),
`angle` (degrees), `loss`, `wavelength`, `span`. Its component is a `Bend`,
$S_{21} = e^{i\gamma R\theta}$, the bent mode's index along the arc at R; the transitions to
straight guides are left out.

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| analytic | the straight strip's analytic mode along the arc: no radiation, no shift | n_eff within 0.003 of 3D | the fit's, radii from 5 µm |
| 2D | the exact bent slab of the strip seen from above by the effective index method: the vertical slab's TE index (2.84 for 220 nm) as the core of a slab `width` wide in the cladding, bent at R, solved by radial shooting to Marcuse's outgoing Hankel function (his Eq. 10), its lateral TM mode; labelled 2D | n_eff within 0.06 of 3D, the effective index method's own error ([eim.md](eim.md)) | radii from 1 µm |
| 3D | the straight strip's 3D model plus the bend's shift: the strip's cross-section mapped conformally (Heiblum & Harris), a PML beyond the turning point, less the same window straight, both by the standard scheme on the same cells | — | λ₀ ± span/2 |

The 3D model's shift is the device's own construction. The conformal map turns the bend into a
straight guide whose permittivity grows as $(1 + x/R)^2$ ([bends.md](bends.md)); its mode
radiates past the turning point $x = R(n_\text{eff}/n_\text{clad} - 1)$, where the window's PML
absorbs it. The standard scheme's error at the strip's corners (about 1e-3 in n_eff on
20 nm cells) is larger than the bend's own effect at 5 µm (2.4e-3), but it is nearly the same
bent and straight on the same cells, so the shift (bent less straight) keeps the bend's effect
and drops the corners'; the straight strip itself comes from Hadley's equations. The map
converges at second order where nothing has corners (`mode/bend-slab-te`); the shift moves by
under 3e-5 from 28 to 14 to 7 nm cells (`device/bend-3d-grid`). The radiation loss is the
bent less the straight window's, both from Im n_eff.

## Directional coupler

`StripCoupler`: ports `o1` (lower left), `o2` (upper left), `o3` (upper right), `o4` (lower
right); parameters `width`, `thickness`, `gap` (edge to edge), `length` (the coupled section's,
starting at the analytic 3 dB length), `loss`, `wavelength`, `span`. Its component is a
`DirectionalCoupler` by coupled-mode theory from the two supermodes (Chrostowski & Hochberg,
Section 4.1): through $e^{i\bar\gamma L}\cos(\Delta\gamma L/2)$, across
$i e^{i\bar\gamma L}\sin(\Delta\gamma L/2)$, the cross-over length $L_x = \lambda/2\Delta n$. The
bends that bring the guides together are left out.

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| analytic | supermodes split by $\Delta n = C\lambda/\pi$ about the strip's analytic mode, C the fit's | $L_x$ within 3 % of 3D | the fits', gaps 100–500 nm, 1.548–1.552 µm |
| 3D | the full-vector solver's even and odd supermodes, each on half the pair behind a mirror wall, at five wavelengths | — | λ₀ ± span/2 |

The analytic model's splitting is constant in λ, as the library's coupler has always had it, so
its C falls as 1/λ; the solver's grows, and $L_x$ falls by about 0.45 % a nanometre, so beyond
±2 nm use the 3D model. The 3D model converges at about first order in its grid, the standard
scheme's at the corners (0.84 for Δn from 25 × 27.5 to 6.25 × 6.875 nm).

## Ring

`StripRing`, all-pass (ports `in`, `through`) or add-drop (also `add` and `drop`): parameters
`width`, `thickness`, `radius` (to the guide's centre), `gap` (and `gap_drop`), `coupling_length`
(each straight section of a racetrack, zero for a circle), `loss`, `round_trip_loss` (dB per round
trip: the bends' and couplers', Bogaerts's Eq. 19), `wavelength`, `span`. Its component is an
`AllPassRing` or `AddDropRing` by Bogaerts's closed forms (his Eqs. 1, 5 and 6 as fields), on the
guide's dispersion and each coupler's power coupling:

$$
L = 2\pi R + 2L_c, \qquad
\kappa^2 = \sin^2\!\Big(C(g)\big(L_c + \sqrt{2\pi R/\gamma}\big)\Big).
$$

The coupler is coupled-mode theory's $\sin^2(CL)$ over the straight section and the curved one.
Along the curve the gap grows as $z^2/2R$ from its least, and the coupling falls with it as
$e^{-\gamma\Delta g}$, so the curve's length is the Gaussian integral
$\int e^{-\gamma z^2/2R}dz = \sqrt{2\pi R/\gamma}$: the parabolic treatment of the curved gap and
the exponential coupling of Little et al.'s Appendix. C and γ are the straight pair's at the gap.

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| analytic | the strip's analytic mode; C and γ from the fit | the free spectral range within 0.5 % of 3D | the fits' |
| 3D | the strip's 3D mode; C from the solver's supermodes at the gap, γ by a central difference over ±25 nm of gap, at the band's centre | — | λ₀ ± span/2 |

The ring's mode is the straight strip's: the bend's shift and radiation are left to
`round_trip_loss`, and the coupling is the band centre's at every wavelength. The figures are
Bogaerts's: the free spectral range $\lambda^2/(n_g L)$ (Eq. 9), the resonance nearest λ₀
(Eq. 3), the loaded Q (Eqs. 20 and 22), and the extinction or the drop on resonance (Eqs. 11–16).
At the defaults (10 µm, 200 nm) the analytic κ² is 0.0124, 2.2 % below 3D's.

## Validation

| Case | What it checks | Grid | Found |
|---|---|---|---|
| `device/library-components` | each analytic model at its defaults against the studio library's component | exact | 0 |
| `device/ring-closed-forms`, `device/coupler-closed-forms` | the figures against Bogaerts's and coupled-mode theory's closed forms | exact | 0 and 1.5e-15 |
| `device/derivatives` | exact $dS/dp$ (a length, a loss, an angle, a radius as an expression, a coupling length) against fourth-order differences | exact | 6.8e-10 relative |
| `device/waveguide-fit-neff`, `-ng` | the analytic strip against 3D at five designs between the nodes | fitted 10 nm, Hadley | 6.6e-5 and 0.0173 |
| `device/waveguide-3d-order` | the 3D strip's order | 12.5 × 10 to 3.125 × 2.5 nm | 1.92 |
| `device/bend-analytic`, `device/bend-2d` | the analytic and 2D bends against 3D | 10 nm straight, 20 nm shift | 2.4e-3 and 0.046 |
| `device/bend-3d-grid` | the 3D bend's shift from grid to grid | 28, 14, 7 nm | 2.8e-5 |
| `device/coupler-fit` | the analytic $L_x$ against 3D at four designs, 1.548 and 1.552 µm | fitted 10 nm | 2.5 % |
| `device/coupler-3d-order` | the 3D coupler's order | 25 × 27.5 to 6.25 × 6.875 nm | 0.84 |
| `device/ring-fsr` | the analytic free spectral range against 3D | fitted 10 nm | 0.39 % |
| `device/threads` | the 3D waveguide on 1 and 20 threads | fitted 10 nm | 0 |
| `device/bend-ring-slab` | the bend's 2D model against the bent slab of `components/ring-fsr-fdfd` | exact | 0 |
| `device/strip-chrostowski` | Chrostowski & Hochberg's Fig. 3.14 through the waveguide device | 5 nm, whole | 2.444396 (2.443 ± 3e-3) |
| `device/coupler-chrostowski` | their Eq. 4.14 at 200 nm through the coupler device | 5 nm, quarter | 37.152 µm (37.5 ± 1) |
| `device/ring-q-bogaerts` | Bogaerts's Fig. 5 through the ring device: the length where Q peaks | exact | 9.353 mm (10 ± 1) |
| `device/wire-ng-dwivedi` | Dwivedi et al.'s Table I, the 602 nm wire's n_g, through the waveguide device | fitted 10 nm, Hadley | 4.03847 (4.0453 ± 0.032) |

The published examples go through the devices too, to their bits: `strip_waveguide` solves the
waveguide device's cross-section, `directional_coupler` the coupler device's 3D model and
cross-sections, `mzi_dwivedi` the waveguide device's 3D model, `ring_q_factor` the ring devices
on its stated guide, and `bend_loss` prints the bend device's 2D and 3D models at 2 µm; their
outputs are what they were. The unit tests check that each device's cross-section is the one each
example built by hand, to the bit, and that the studio's library entries are the components it
built before the devices.

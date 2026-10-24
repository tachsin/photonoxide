---
title: "Devices"
module: device
summary: "A component that knows its geometry: a stack, the parameters of its shapes, its models from a fit to the solver up to 3D, and the component each gives, with every figure's fidelity and grid. The strip waveguide, bend, directional coupler and rings; the rib, the MMIs, the Y-branch and the MZIs, on custom stacks and partial etches; and the thermo-optic phase shifters, a metal heater and doped heaters, at analytic, 2D and measured fidelity, Jacques et al. 2019's through them."
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
  - cite: "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995)"
    doi: 10.1109/50.372474
  - cite: "Y. Zhang et al., Opt. Express 21, 1310 (2013)"
    doi: 10.1364/OE.21.001310
  - cite: "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012)"
    doi: 10.2528/PIERB11092006
  - cite: "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019) (the thermo-optic phase shifters: their region, materials and Eqs. 4-5, simulated and measured)"
    doi: 10.1364/OE.27.010456
  - cite: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969) (the analytic shifters' dn_eff/dT)"
    doi: 10.1002/j.1538-7305.1969.tb01166.x
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
  - device/mmi-library
  - device/mmi-self-imaging
  - device/mzi-closed-form
  - device/mzi-derivatives
  - device/rib-wide-slab
  - device/rib-eim
  - device/rib-hadley
  - device/fdfd-straight
  - device/mmi-2d-grid
  - device/mmi-guided-2d
  - device/y-branch-2d-grid
  - device/y-branch-ideal-2d
  - device/thermo-analytic-metal
  - device/thermo-analytic-doped
  - device/thermo-analytic-time-metal
  - device/thermo-analytic-time-doped
  - device/thermo-jacques-tin
  - device/thermo-jacques-npp
  - device/thermo-tin-measured
  - device/thermo-npp-measured
  - device/thermo-2d-grid
  - device/thermo-jobs
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

The studio can then show, change, sweep and optimize the geometry, not fixed constants. The
first devices are a silicon strip's: the waveguide, the bend, the directional coupler and the
ring, all-pass and add-drop. Then the rest of the studio's library: the rib waveguide, the 1 × 2
and 2 × 2 MMIs, the Y-branch, and the Mach–Zehnder interferometers of directional couplers and
of Y-branches; and the waveguide and the coupler take ribs and sloped walls too. The first
active devices follow: the thermo-optic phase shifters, a metal heater over a strip and doped
heaters beside a rib, their phase following the heater's power.

## The abstraction

A `Device` has seven methods:

| Method | What it gives |
|---|---|
| `kind()`, `about()` | the component's kind (`"waveguide"`, …) and a sentence |
| `parameters()` | name, unit, default, range: a `circuit::Parameter` each |
| `stack(values)` | the stack, the core layer at the device's thickness |
| `geometry(values)` | a `stack::Structure` of shapes on the stack, and the ports: name, position, direction (degrees, the way light leaves), width and layer; drawn by the device's [generator](generators.md) (`Straight`, `Bend`, `DirectionalCoupler`, `Ring`, `Mmi`, `YBranch`), the same bits as before |
| `fidelities()` | a `Method` per model: its fidelity, what computes it, this write-up, its papers, its `Stated` error against a higher fidelity and its validity ranges |
| `component(values, fidelity)` | a `Built`: the boxed component, the values of its own parameters, their derivatives, and the figures |
| `derived(values, fidelity)` | the figures: n_eff, n_g, the cross-over length, the free spectral range, Q, a bend's loss, each a `Figure` with its fidelity and grid |

From Python and MATLAB, the same methods are functions of the device's kind, its values given
as numbers in their units (those left out at their defaults): `devices` gives every device's
kind, parameters, models and ports; `device_spectrum` the component's S-matrices at a fidelity,
with the model's grid and source; `device_derived` the figures; and `device_geometry` the
stack, the polygons on each layer and the ports. A design's expressions go in a job or a
netlist ([below](#saved-as-a-job)).

```python
import numpy as np
import photonoxide as phox

coupler = next(d for d in phox.devices() if d.kind == "directional coupler")
print([p.name for p in coupler.parameters], [m.fidelity for m in coupler.fidelities])
design = dict(values={"gap": 0.25, "length": 12.5}, fidelity="analytic")
w = np.linspace(1.5, 1.6, 101)
dc = phox.device_spectrum("directional coupler", wavelength_um=w, **design)
cross = np.abs(dc.s[:, 2, 0]) ** 2   # from o1 into o3; dc.grid and dc.source name the model
for f in phox.device_derived("directional coupler", **design):
    print(f.symbol, f.value, f.unit, f.grid)
shapes = phox.device_geometry("directional coupler", values=design["values"])
```

```matlab
values = struct("gap", 0.25, "length", 12.5);
dc = photonoxide.device_spectrum("directional coupler", linspace(1.5, 1.6, 101), ...
                                 values=values, fidelity="analytic");
cross = abs(dc.s(:, 3, 1)).^2;       % from o1 into o3
f = photonoxide.device_derived("directional coupler", values=values, fidelity="analytic");
shapes = photonoxide.device_geometry("directional coupler", values=values);
```

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
`Symmetry::Full`). A rib, or a strip whose walls slope, isn't the same above and below its
mid-plane by its own shape: a mirrored mesh solves it on its half behind the vertical plane
alone (`Symmetry::Vertical`, which a stack that differs above and below can take too), and a
fitted mesh then puts the slab's top on a node as well, the spacing $(t/2)/m$ with m the whole
number nearest $(t/2)/\text{step}$ that does (from half to three times it). Cells a sloped wall
crosses hold each material's share of them (`Structure::cut_cell`), so a sloped strip or rib is
solved by the standard scheme, whose cells may hold any permittivity; Hadley's equations need
each cell one material. The scheme is the [standard full-vector one](vector.md) or
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
less on practical grids (2.445713, 2.444396 and 2.443506 at 10, 5 and 2.5 nm, where Hadley's
give 2.442212, 2.442178 and 2.442168: `strip_waveguide --full`),
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

## Ribs and sloped walls

**The waveguide and the coupler** take two more parameters, how the core layer is cut beside
their guides: `slab`, the layer's thickness left there (0, the default, etches through it: a
strip), and `sidewall`, the walls' angle from the layer's plane (90°, the default, vertical;
less leans them out going down, the shape's outline its top face). They draw their shapes with
`Etch::partial(thickness − slab).with_sidewall_deg(sidewall)` ([stacks.md](stacks.md)), and
their 3D models solve the rib's cross-section on its half. The analytic fits are a strip's with
vertical walls, and refuse a slab or a sloped wall (their validity ranges say so, and the
studio's page offers the 3D model).

**The rib waveguide** (`RibWaveguide`) is the rib as a device of its own: parameters `width`
(the top's), `thickness`, `etch_depth` (from the core's top; less than its thickness: etched
through it is the waveguide), `sidewall`, `length`, `loss`, `wavelength` and `span`; by default
500 nm wide on 220 nm SOI, etched 130 nm (a 90 nm slab), on cells fitted about 10 nm apart with
1.5 µm of cladding, the slab carrying the mode's tail farther than a strip's.

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| 2D | the effective index method ([eim.md](eim.md), Hocker & Burns): the vertical slabs through the rib and beside it, then the lateral slab between them, TE-like, at five wavelengths for n_g and D; a sloped wall at its mean width. Labelled 2D: an approximation of the 3D guide | n_eff within 0.07 of 3D (0.064 for a narrow, deep rib, 400 nm wide etched 200 nm; 3e-4 at 3 µm wide) | widths 0.4–3 µm, etched 50–200 nm on 220 nm SOI, vertical walls |
| 3D | the full-vector solver's mode at five wavelengths across the span, on the half: Hadley's equations for vertical walls, the standard scheme for sloped ones | — | λ₀ ± span/2 |

Its figures are n_eff, n_g (and D in 3D), and at 2D the slab's index beside it, $n_\text{slab}$:
the mode stays a rib's mode while its n_eff is above it. Through the device, Hadley's standard
rib (3 µm wide, a 1 µm film of 3.44 on 3.40 under air, 1.15 µm) reproduces his Table I for
slabs of 0.1 to 0.7 µm by his equations on a 50 nm grid (`device/rib-hadley`; the `rib_hadley`
example, slab 0 through the waveguide device), and a wide rib tends to its film's slab index,
the effective index method's limit (`device/rib-wide-slab`: 8 and 16 µm, extrapolated in
$1/W^2$).

## MMI

`StripMmi`, 1 × 2 (`StripMmi::one_by_two`, kind `"MMI 1x2"`: ports `o1` in the middle of the
left face, `o2` and `o3` at +W/4 and −W/4 on the right) or 2 × 2 (`two_by_two`, `"MMI 2x2"`:
`o1`, `o2` at −W/6 and +W/6 on the left, `o3`, `o4` at +W/6 and −W/6 on the right), the
circuit's `Mmi`'s names, drawn by the `Mmi` generator. Parameters `width` and `length` of the
multimode section, `access_width` (the access guides' width at the section), `taper_length` and
`port_width` (each access guide tapers over `taper_length` to `port_width` at its port; with no
taper the ports are on the section's faces, `access_width` wide), `thickness`, `slab` and
`sidewall` (as the waveguide's: a rib MMI), `wavelength` (where the default length is the first
two-fold image) and `span`. The defaults are the studio's MMIs': 6 µm wide with 1.5 µm access
guides on 220 nm SOI, 33.56 µm long (3L_π/8) and 44.74 µm (L_π/2).

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| 2D, guided modes (the default) | Soldano & Pennings's guided-mode propagation analysis (their Section III, Eqs. 8–12) on the plane seen from above by the effective index method, the section's and access guides' exact slab modes ([components.md](components.md)): the S-matrix between the access guides' fundamental modes at the section's faces; the tapers taken as adiabatic and lossless, their phase left out. On the default stack the film is Li's silicon in Malitson's silica, as the studio's MMIs have always been built; on another stack, or a rib, the slabs' indices at the design wavelength. Labelled 2D | each $\lvert S_{qp}\rvert^2$ within 5e-3 of 2D FDFD of the same plane | widths 3–6 µm, access guides 0.5–1.5 µm, vertical walls, 1.5–1.6 µm |
| 2D, FDFD (`with_model_2d(Model2d::Fdfd)`) | the drawn geometry by 2D FDFD on the effective index method's plane (below) | — (3D isn't resolved here) | — |
| 3D | the drawn geometry on its stack by 3D FDFD (below) | — | — |

The MMI has two 2D models, and a device setting chooses which its 2D fidelity is, as the strip's
mesh chooses its 3D scheme: the guided modes by default, cheap and the library's own; 2D FDFD
with `with_model_2d`. Its figures, from the S-matrix at the design wavelength: the powers into
the two outputs from `o1`, the excess loss $-10\log_{10}(P_a + P_b)$ and the imbalance
$10\log_{10}(P_a/P_b)$ in dB; and with the guided modes the beat length
$L_\pi = \pi/(\beta_0 - \beta_1)$ (Soldano & Pennings's Eq. 6, from the exact modes) and the
first two-fold image, $3L_\pi/8$ (symmetric interference, their Eq. 37) or $L_\pi/2$ (paired,
Eq. 33). Those lengths are within 1.2 % of the paper's closed forms with Eq. 4's effective width
from 3 to 12 µm wide (`device/mmi-self-imaging`; the `mmi_soldano` example).

The guided modes leave out what the junctions radiate and reflect; 2D FDFD keeps it. On a 3 µm
MMI with 0.5 µm access guides at its image (8.48 µm), 2D FDFD gives 0.39834, 0.40407 and
0.40622 into each output at 20, 10 and 5 nm (order 1.5), the guided modes 0.40486
(`device/mmi-2d-grid`, `device/mmi-guided-2d`); on the default 1 × 2, 0.47479 and 0.47933 at 20
and 10 nm against 0.48082, and on the 2 × 2, 0.45503 and 0.48009 at 10 nm against 0.45826 and
0.48326.

## Y-branch

`StripYBranch`, kind `"Y-branch"`: ports `o1` (the stem), `o2` (the upper arm) and `o3`, as the
circuit's ideal `YBranch`. Its stem is `length` long and its width the not-a-knot cubic spline
through `w1` … `w13` at 13 points evenly along it, written with the expression language's
`spline` ([expressions.md](expressions.md)) and drawn between $-w(x)/2$ and $w(x)/2$
(`geometry::Between`) by the `YBranch` generator; two sine S-bend arms `width` wide leave its
end `w13 − 2 width` apart and reach `separation` after `arm_length`. Parameters those, with
`thickness`, `slab`, `sidewall`, `excess_loss` (dB, the ideal model's), `wavelength` and `span`.
The defaults are Zhang et al.'s junction: 2 µm long, their Table 1's widths, from the 500 nm
strip to two 500 nm arms 200 nm apart; the arms, 5 µm long to ports 2 µm apart, are ours.

| Fidelity | Model | Stated error | Holds over |
|---|---|---|---|
| analytic | ideal: the stem's power split equally, with an excess loss | $\lvert S\rvert^2$ within 0.05 of 2D, on Zhang et al.'s junction | — |
| 2D | the drawn geometry by 2D FDFD on the effective index method's plane. Labelled 2D | — (3D isn't resolved here) | — |
| 3D | the drawn geometry on its stack by 3D FDFD | — | — |

Its 2D model converges at about second order: 0.46817, 0.46775 and 0.46763 into each arm at 20,
10 and 5 nm with sharp arms (3 µm long to ports 1.5 µm apart), an insertion loss of 0.29 dB in
2D, of which the arms' bends shed part: with the default arms, 0.47862 into each at 10 nm,
0.190 dB. Zhang et al. designed it in 2D FDTD to "sub-0.2 dB", and found 0.13 dB in 3D FDTD at
λ/34 and 0.28 ± 0.02 dB measured (the `y_junction_zhang` example: its drawn outline is their
Table 1's at the 13 points, and its 2D loss below their 2D design's).

## FDFD fidelities

The MMIs' and the Y-branch's FDFD models solve the device's own drawing (`device::FdfdGrid`):

- **The window** holds the drawing and `margin` of cladding beside it (1 µm), and each port's
  guide led straight out of it through the PML, cut as the core is; the port's plane is three
  cells out from the device's port, five more cells before the PML (12 cells). The cells are
  even in number each way and centred, so a device mirrored about its axis is mirrored on the
  grid; in 3D the core's faces are on nodes, `margin` of cladding below and above it.
- **The plane** in 2D is the effective index method's (the `"fdfd"` job's,
  [fdfd.md](fdfd.md)): each point's column's slab index squared, the slab's TE mode, H
  along z. 3D solves the stack itself, TE-like.
- **Each cell's permittivity** is as the grid's `averaging` says. In 2D the MMIs and the
  Y-branch take the shapes' fill ([averaging.md](averaging.md)), the diagonal of Farjadpour et
  al.'s tensor; under sloped walls, which the fill's two-column plane can't take, and in 3D,
  each cell is from 8 samples a side, harmonic along each component of E and arithmetic across
  it. A port's guide abuts its lead, a seam the fill sees no more (issue #376: it reflected
  |S₁₁|² 6e-5 on 100 nm cells): a straight strip through the solves gives |S₁₁|² 1.8e-30 and
  |S₂₁ − S₁₂| 6e-16 by the diagonal, 2D and 3D, as sampled (6e-31 and 1e-15). In 2D the fill
  converges faster. On a 3 µm 1 × 2 MMI at its image (`device/mmi-2d-grid`, 20, 10 and 5 nm) the
  power into an output changes by 0.0064 and then 0.0018, order 1.8, where sampled it changes by
  0.0057 and then 0.0021, order 1.4; on Zhang et al.'s Y-branch (`device/y-branch-2d-grid`, 40
  to 5 nm) by 1.3e-3, 3.0e-4 and 8.4e-5, where sampled by 1.4e-3, 4.2e-4 and 1.3e-4. (The
  whole tensor gives the MMI's numbers, its walls along the grid, and the Y-branch's 1.5e-3,
  2.4e-4 and 1.1e-4, at twice the factorization's cost.) 3D keeps samples: on the grids a 3D
  MMI fits, its image isn't resolved (below), so no convergence was measured there.
- **The ports** are each guide's fundamental mode on its plane, in a window across from the
  middles between it and its neighbours on the same side ([fdfd-ports.md](fdfd-ports.md),
  [fdfd-3d.md](fdfd-3d.md)); the S-matrix's phases are referred back from the planes to the
  device's ports by each mode's β. Through it a straight strip 2 µm long gives
  $S_{21} = e^{i\beta L}$ to 1.6e-7 in 2D and 2.6e-5 in 3D on 100 nm cells
  (`device/fdfd-straight`).
- **The wavelengths**: `samples` of them evenly across the span (five), the component a
  `Sampled` one, interpolated between them; one samples the centre only.

The 2D grid is 20 nm by default; 3D, 40 nm in the plane and 20 nm up the stack with 0.6 µm of
margin. **3D is limited by the direct solver's memory**, which grows faster than the unknowns:
a 2 µm MMI's 1.26 million (50 × 50 × 27.5 nm cells) took 32 GB, so a device's 3D solve refuses
more than 1.2 million unknowns and asks for a coarser grid. That doesn't resolve an MMI: its
self-image is sensitive to the higher modes' phases, which coarse cells get wrong. The 3 µm MMI
above gives 0.15981, 0.18082 and 0.22351 into each output on 100, 80 and 67 nm cells (55, 44 and
37 nm up), against 0.406 in 2D, still moving by 0.04 a step; the 2 µm one 0.299, 0.354 and 0.390
on 80, 67 and 50 nm cells. So the guided modes' and 2D FDFD's errors against 3D aren't stated
here; resolving them needs about 20 nm cells, an iterative solve's (`IterativeSolver3d`) or a
larger machine's. The surveys are `device::splitter_tests::survey` (ignored: run by hand).

## MZI

`StripMzi`, of directional couplers (`StripMzi::couplers`, kind `"MZI"`: ports `o1`, `o2` on the
left and `o3` (upper), `o4` on the right, as the circuit's `mzi` wires them) or of Y-branches
(`y_branches`, `"Y-branch MZI"`: `o1`, the splitter's stem, and `o2`, the combiner's): a
splitter, two strip arms `upper` and `lower` µm long, an ideal phase shifter of `phase` degrees
in the upper arm (a heater, as a circuit sees it), and the splitter again, mirrored, as the
combiner. Parameters `width`, `thickness` (the arms' and the splitters'), `upper`, `lower`,
`phase`, `loss` (the arms'), the splitter's own (a coupler's `gap` and `coupler_length`, at its
analytic 3 dB length; a Y-branch's `excess_loss`, its outline at Zhang et al.'s defaults),
`wavelength` and `span`. It is the netlist of its devices, solved as a circuit
([circuits.md](circuits.md)), each part at the MZI's fidelity: analytic (the coupler's or the
ideal Y-branch's and the strip's analytic models) or 3D (theirs). Its figures are the powers
through and across (or through, for the Y-branches).

Its drawing is its parts': the splitter at the origin (a coupler's guides parted by sine
S-bends 10 µm long, 2 µm each, drawn and not modelled), the shorter arm straight, the longer
one's extra length a detour of four quarter turns (radius up to 5 µm) away from the other, and
the combiner on their ends. The netlist's arms are as long as the drawn ones' centre lines; their
bends' loss and the mode's shift in them aren't modelled.

It is the closed form of the transfer matrices, $C_c \thinspace \mathrm{diag}(t_l, t_u e^{i\varphi}) \thinspace C_s$
with each coupler's through and across fields, to round-off (`device/mzi-closed-form`; for the
Y-branches, $s^2(t_u e^{i\varphi} + t_l)$), and its derivatives in the arms' lengths, the phase
and the arms' loss are the circuit adjoint's, exact (`device/mzi-derivatives`).

## Thermo-optic phase shifters

`ThermoShifter`, the first active devices (0.6's phase P9): a straight silicon guide `length`
µm long and a heater beside it, ports `o1` and `o2`. Two heaters, each a kind of its own, at
Jacques et al. 2019's fabricated designs by default:

- **a metal heater** (`ThermoShifter::metal`, kind `"thermo-optic phase shifter"`): a TiN strip
  `heater_width` wide and `heater_thickness` thick, `height` µm above a `width` × `thickness`
  strip, `cover` µm of oxide over it and air above (7.5 µm, 2 µm above a 500 × 220 nm strip;
  the paper doesn't give the TiN's thickness or the oxide over it: 100 nm and 1 µm);
- **doped heaters** (`ThermoShifter::doped`, `"doped thermo-optic phase shifter"`): two N++
  strips `heater_width` wide in a `slab` µm slab, `buffer` µm beyond a `width` × `thickness`
  rib's sides, the slab ending past them, the oxide's top `cladding` µm over the buried oxide's
  (1.0 µm strips 0.8 µm beyond a 500 × 220 nm rib on a 90 nm slab; the cladding not given: 3.32
  µm).

Both take `pitch` (a neighbour's centre from the guide's, for the crosstalk), `power` (mW),
`loss` (dB/cm, the propagation loss the heat model doesn't give), `wavelength` (1.5 to 1.6 µm)
and `span`. The buried oxide is the stack's, the layers under the core taken as oxide; the
heat and thermo-optic constants are Jacques et al.'s Table 3 at 300 K (the oxide 1.38
W/(m·K), the substrate 148, the strip 90, the slab 55, N++ 25, TiN 28; silicon's dn/dT
1.8e-4/K, the oxide's neglected, as theirs), so the core must be silicon: a nitride core is
refused.

Its **component** is the phase the heater adds and the guide's loss: ports `o1` and `o2`,
parameters `phase` (rad) and `loss` (dB), $S_{21} = S_{12} = 10^{-\mathrm{IL}/20}\, e^{i\varphi\lambda_0/\lambda}$
with $\varphi = \pi P/P_\pi$ (Jacques et al.'s Eqs. 4 and 5) and IL the loss over the length.
The unheated guide's own propagation is a waveguide's: a circuit puts one beside it. Its
derivatives in the power, the loss and the length are exact. The figures, each with its
fidelity and grid: `P_pi` (mW), `tau` (the phase's 1/e time turned on, µs), `f_3dB` (kHz), `X`
(the crosstalk on a neighbour at the pitch: the share of the guide's phase it takes), `s`
($\partial n_\mathrm{eff}/\partial P'$, m/W), `dneff_dT`, `n_eff`, `phi` and `IL`. $P_\pi$ is
a 2D number, a straight section's of any length (the length cancels: Jacques et al.'s Section
2.1): a device's bends, leads and ends aren't in it.

- **Analytic:** closed forms (`device::thermo_closed`). The cross-section thermally all oxide
  between the isothermal substrate (its 12.7 µm of silicon in the region as the 0.12 µm of oxide
  of the same resistance) and an insulated top (the air's 10 W/(m²·K) carries 4e-5 of what the
  oxide does), laterally unbounded; the heaters sheets at their height, each its share of the
  power spread across its width. In the slab's eigenfunctions $\sqrt{2/H}\sin(k_m y)$,
  $k_m = (m + \frac{1}{2})\pi/H$, each term obeys $-k(u'' - p_m^2 u) = q_m(x)$, whose Green's
  function across is $e^{-p_m|x|}/(2kp_m)$ and a strip's integral of it closed: steady
  ($p_m = k_m$), at a frequency ($p_m^2 = k_m^2 + i\omega/D$, the series' differences from the
  steady terms falling as $1/m^4$), and in time
  ($u_m(t) = u_m(\infty) - \frac{1}{\rho c}\int_t^\infty e^{-Dk_m^2\tau} W(x,\tau)\,d\tau$,
  $W$ the strips' heat kernel, a difference of erfs, by Simpson's rule in $\ln\tau$). The rise at
  the guide's centre times Marcatili's $dn_\mathrm{eff}/dT$ (his rectangle's $E^x_{11}$, solved
  exactly, its indices moved by their dn/dT: [marcatili.md](marcatili.md)) gives $P_\pi$; the
  1/e time and −3 dB frequency are the rise's. Microseconds of the analytic model's arithmetic
  (the elementary functions the `libm` crate's, the same bits on every system).
- **2D:** `thermal::PhaseShifter` on the cross-section ([phase-shifter.md](phase-shifter.md)) in
  Jacques et al.'s region, 18 µm high, its bottom in the substrate held, 10 W/(m²·K) to the air,
  40 µm wide with insulated sides (wider to hold the neighbour 10 µm inside them): the static
  solver's rise graded from 20 nm at the shapes' corners to 0.5 µm by 1.2, each optical cell's
  weight by first-order perturbation of the full-vector mode on 10 nm cells, the phase turned on
  by BDF2 steps of 0.2 µs (metal) or 0.1 µs (doped), its −3 dB frequency, and the neighbour's
  crosstalk (the guide's mode moved by the pitch: a probe whose own silicon isn't drawn). The
  cross-sections are the phase's own: at Jacques et al.'s designs the devices' slopes are
  `shifter/jacques-tin`'s and `-npp`'s to the bit (a unit test). Seconds: 6 to 8 here.
  `ThermoShifter::with_refinement(r)` divides the grids by r, as the validation does.
- **Measured:** a measured device's file (`device::Measurement`, TOML: its kind, source, the
  design's parameters the paper gives, and the figures measured, each with where in the paper),
  at its design's values only: the library holds Jacques et al.'s two
  (`src/device/measured/jacques-2019-*.toml`: the TiN's 29.2 mW, 7.2 µs and 33 kHz, the N++'s
  22.8 mW, 2.2 µs and 85 kHz, the MZIs' arms 200 µm apart), and
  `ThermoShifter::with_measurement` adds one's own.

Each lower model against the next one up (`device/thermo-*`): the analytic $P_\pi$ within 15%
of the 2D model's over its validity range (the metal heater 1 to 10 µm wide, 1 to 3 µm above
the strip, the strip 0.4 to 0.6 µm wide: at most 12.7% low, the guide's silicon and Marcatili's
mode the difference; the doped heater's buffer 0.4 to 1.6 µm and strips 0.5 to 2.0 µm wide: at
most 14.7% high, the slab's silicon, which carries the heat across to the rib, left out), its
time constant and bandwidth within 2.4% for the metal heater and 51% for the doped (the slab's
silicon again: the closed form is slower); the 2D model's $P_\pi$, gated by grids twice as fine,
14% above both measurements, as the paper's own simulations are 10% (TiN) and 12% (N++) above
them (Section 3.1). On the devices' own grids the 2D time constants are 4.1 (TiN) and 2.0 µs
(N++) above the measured ones (the paper's simulations 3.6 µs above, Section 3.2), and the
measured bandwidths, which peak (their Fig. 10(a), the leads' inductance), 2.2 and 3.6 times
the 2D's (`thermo_jacques` prints them).

```python
import photonoxide as phox

heater = phox.device_derived("thermo-optic phase shifter", values={"power": 12.5})
print({f.symbol: (round(f.value, 4), f.unit) for f in heater})   # P_pi, tau, f_3dB, X, ...
```

```matlab
f = photonoxide.device_derived("doped thermo-optic phase shifter", values=struct("power", 12.5));
```

## Saved as a job

A device's design is a job file with a `[device]` table in place of a task (`device::Spec`):
the device by its kind, its stack, its parameters' expressions, and the fidelity it is computed
at.

```yaml
name: narrow-coupler

device:
  kind: directional coupler    # waveguide, bend, directional coupler, all-pass ring, add-drop ring,
  # rib waveguide, MMI 1x2, MMI 2x2, Y-branch, MZI, Y-branch MZI
  fidelity: analytic           # analytic, 2D or 3D, as the device has them; its cheapest by default
  stack: soi_220               # soi_220 (the default), soi or nitride

  parameters:
    # expressions; the device's parameters left out take their defaults
    w: "450 nm"
    width: w
    gap: "w/2.5"
```

A stack is a preset, `"soi"` and `"nitride"` with 2 µm of oxide under the core or
`{ preset = "nitride", bottom_oxide_um = 3 }`, or layers of the
[materials catalogue](catalogue.md)'s, bottom to top, each material an id (its default model at
its default conditions) or `{ id = "si", model = "li-1980", temperature = 300 }`:

```yaml
device:
  stack:
    core: Si                   # the layer the guides are drawn on: its thickness is the device's
    substrate: si
    cladding: sio2

    layer:
      - name: BOX
        thickness_um: 2.0
        material: sio2
        background: sio2

      - name: Si
        thickness_um: 0.22
        material: si
        background: sio2
```

The presets are those materials' (Li's silicon, Malitson's silica, Luke's nitride), so a preset
written out as layers is the same stack to the bit. On the default stack a design's device is the
library's, to the bit. `job::check` builds the device and its design, and refuses an unknown kind,
fidelity, preset or material, an anisotropic material, a core that isn't a layer, and what
`Design::new` refuses; `job::preview` draws its structure over its shapes and ports with 1 µm
around them, from 1 µm below the core to 1 µm above; a run records that scene. The studio saves a
component page's design this way, in the workspace's `jobs/`, and opens it again on its page.

## Computed as a job

Beside the `[device]` table, a `[task]` says how the job computes the device, and
`job::from_device(name, &spec, &Compute)` writes it:

- `kind = "spectrum"`: the device's own model at the design's fidelity, the component
  `Device::component` gives. At a solver-backed fidelity that is the solve: the cross-section's
  modes (the waveguide's, coupler's, ring's and rib's 3D, the bend's 2D and 3D), 2D FDFD
  (labelled 2D) or 3D FDFD (the MMIs and the Y-branch). The run records the scene and the ports,
  then `DeviceModel`: the method, its write-up and papers, what the component reproduces on
  which grid, how long it took and the device's figures; then the S-parameters at each
  wavelength of the band. They are `Device::component`'s at that fidelity to the last bit, and
  replay so.
- `kind = "fdtd"`: the drawn structure stepped in time, as an `"fdtd"` job's shapes are, each
  port's guide led straight on through the window's edge and its PML; the sources and monitors
  find the ports by name (`port = "o1"`). `from_device` puts a mode source at the first port and
  a mode monitor at each, on 40 nm cells in 2D (the effective index method's plane, labelled
  2D) or 50 nm in 3D. No device's fidelity is FDTD; it is a check on the others.

```yaml
name: mmi-3d
timeout_minutes: 30.0

device:
  kind: MMI 1x2
  fidelity: "3D"

task:
  kind: spectrum
  from_um: 1.5       # the band: the device's own, its wavelength ± span/2, by default
  to_um: 1.6
  points: 101
  step_nm: 80.0      # each left out is the device's own: the cells in the chip's plane (an
  step_z_nm: 40.0    # FDFD's, or a cross-section's mesh step); 3D FDFD's cells' height; the
  margin_um: 0.6     # cladding beside the guides; the wavelengths an FDFD samples and its PML
  samples: 3         # cells; an MMI's 2D model by FDFD in place of its guided modes
  pml_cells: 12
# model_2d = "fdfd"
```

From Python and MATLAB, `device_job` writes the job `job::from_device` writes for a design, a
`"spectrum"` task at the device's own grids and band, as a component's Simulate tab does; and
`run_job` runs it, or any job of this page by its file's path:

```python
job = phox.device_job("directional coupler", values={"gap": 0.25}, fidelity="analytic",
                      name="narrow-coupler")   # a dict; format="yaml" gives the file's text
run = phox.run_job(job)                        # run.dir, which photonoxide view replays
```

```matlab
job = photonoxide.device_job("directional coupler", values=struct("gap", 0.25), ...
                             fidelity="analytic", name="narrow-coupler");   % JSON text
run = photonoxide.run_job(job);
```

`job::check` refuses a grid a device doesn't take (a 3D FDFD's cells' height or an FDFD's
samples for a mode solve, any grid for an MZI, whose parts solve on their own), a band that isn't
one, and a 3D FDFD larger than the direct solver holds here (1.2 million unknowns), saying how
large it is, before anything runs. A stop (the window's Stop, the job's timeout) ends the run at
once: a solve already started is let go and its result discarded. The unit tests run each
device's job at each fidelity, the mode solves on a 50 nm mesh and the FDFDs on 200 nm cells, and
compare their records with `Device::component` bit for bit; an ignored test does the same on each
device's own grids.

## Studied as a job

A study computes a design at many points, each a design of its own, by the device's own model at
the design's fidelity, on the task's grids. `job::study_from_device(name, &spec, &Compute,
&Study)` writes it:

- `kind = "sweep"`: one or two of the design's parameters stepped evenly from one value to
  another (`[[task.axis]]`), the second's values fastest, and at each point the band of
  wavelengths (the device's own at 21 points by default). A parameter is one of the device's or
  one the design's expressions name (`w` in `width = "w"`, `gap = "w/2.5"`); its values are in its
  unit, µm for a length and degrees for an angle.
- `kind = "corners"`: the nominal design, then each named corner (`[[task.corner]]`): offsets
  added to its parameters' nominal values (a width bias, a thickness) and its materials at a
  temperature (`temperature_k`), every material of the stack whose index model takes one.

```yaml
task:
  kind: sweep
  from_um: 1.53            # the band at each point
  to_um: 1.57
  points: 5
  # step_nm, step_z_nm, margin_um, samples, pml_cells, model_2d: the grids, as a spectrum's

  axis:
    - parameter: gap
      from: 0.15
      to: 0.3
      points: 4

    - parameter: length
      from: 5.0
      to: 20.0
      points: 3
```

```yaml
task:
  kind: corners

  corner:
    - name: "wide, thin"
      offsets: { width: 0.01, thickness: -0.005 }

    - name: cold
      temperature_k: 200.0
```

Each point is the design with its values written as plain numbers in place of the parameters'
expressions (a parameter the expressions name keeps its unit, so the ones that use it follow
it), and a corner's stack at its temperature, a preset as its layers: `Study::designs` gives
them, and the job `from_device` makes of each computes what the study records there, to the
bit. The run records the nominal design's scene and ports, then `DeviceStudy` (the study's axes
or corners, the method, the device's parameters and the band), then a `DevicePoint` per point
in order: the device's values there, its figures with their grid, its ports' effective indices
and its S-matrices over the band. A point the model refuses (an analytic fit outside its range,
a solve that fails) records why, and the study goes on. The points are solved side by side on
rayon's threads, as many at once as memory allows by their solves' size, or by a farm's workers
([farming](../farming.md)); the record is the same on any number of threads or workers. A stop
(the window's Stop, the job's timeout) ends the study at its next point, the points done kept; a
solve already started finishes first.

`job::check` builds every point's design, refuses a 3D FDFD at any point larger than the direct
solver holds here, at most 10 000 points (an axis of 2 to 1001), 256 corners, a parameter the
design doesn't have, a corner without offsets or temperature, and a temperature at the analytic
fidelity (the fits and closed forms don't solve the materials) or on a stack none of whose
materials' models takes one. Li's silicon (293 K) and Malitson's silica (20 °C), the presets'
materials, take none: a temperature corner of SOI needs a stack with Leviton and Frey's silica,
and silicon's thermo-optic coefficient comes with the thermal devices (0.6). A study runs by the
device's models only, not by FDTD, which computes one design at a time.

## Fitted as a compact model

A device solved at 2D or 3D takes seconds to minutes a design; a chip of it wants thousands of
designs a second. A compact model fitted to the device's own runs over some of its parameters
gives the design one more fidelity, `compact`, at its source's accuracy and a circuit's speed
(`device::compact`). The model is `compact::ParametricModel` ([compact models](compact.md)):
rational in frequency on basis poles found by vector fitting, its numerator and denominator
polynomial (or piecewise linear) in the parameters it covers.

`job::fit_from_device(name, &spec, &Compute, &FitPlan)` writes the job that fits it, `kind =
"fit"`, at the design's fidelity, its source:

```yaml
device:
  kind: all-pass ring
  fidelity: analytic      # the source: analytic, 2D or 3D, as the device has them

task:
  kind: fit
  from_um: 1.54           # the band, the device's own at 101 points by default
  to_um: 1.56
  points: 41
  sampling: grid          # or "latin hypercube", with count (and seed, 1 by default)
  held_out: 6             # samples kept out of the fit (8 by default)
  poles: 12               # the model's basis poles (12 by default)
  degree: 4               # a polynomial in the parameters (2 by default), or
  # interpolation = "piecewise linear" (on a grid)
  # step_nm, step_z_nm, margin_um, samples, pml_cells, model_2d: the grids, as a spectrum's

  axis:
    # one to four of the device's own parameters
    - parameter: gap
      from: 0.15
      to: 0.25
      points: 7           # a grid's
```

**Samples.** On a grid, every combination of each axis's points, evenly spaced. On a Latin
hypercube (M. D. McKay, R. J. Beckman, W. J. Conover, Technometrics 21, 239 (1979),
doi:10.1080/00401706.1979.10489755), `count` samples, each parameter taking `count` levels evenly
spaced from its `from` to its `to`, each level once, in an order drawn from the seed (SplitMix64,
the same on every system). The ends are levels, so the samples span the range the model covers.
The held-out samples are a Latin hypercube of their own (the seed plus one), each level in the
middle of its stratum, $(k + \tfrac12)/n$ of the range: inside it, and between the fit's samples.
Each sample is the design with the covered parameters written as plain numbers, computed as a
`"spectrum"` task computes it, side by side on rayon's threads or a farm's workers, as a study's
points are. The parameters it doesn't cover must keep their values at every sample: one that
follows a covered one through its expression (`coupling_length = "gap*10"`) is refused, since the
model would hold only along that line.

**The record.** The run records the nominal design's scene and ports, `CompactPlan` (what it
covers, the samples, the options, the band), a `DevicePoint` per sample (the fitted ones, then
the held-out ones), and `CompactFit`: the model and what it was fitted from, which the run also
writes to its folder as `compact.json`. The model is fitted from the samples as recorded, so a
farmed run's model is the bits of one run here; the fit itself runs on one thread, so the model
is the same on any number of threads, and reads back from its file to the bit.

**What it states.** `device::Fitted` holds, beside the model: the device's kind, the design it
was fitted from (its `[device]` table), the source fidelity and the grid its runs name, the
covered parameters and their ranges, the values of the others, the band, how it was sampled, its
largest $\lvert\Delta S_{qp}\rvert$ at its samples and at the held-out ones, the largest
$\operatorname{Re} a / \lvert a \rvert$ of its poles over its range (`ParametricModel::stability`)
and its largest singular value over its band and range (`passivity`), each sampled at 21 values
a parameter. The held-out error is the one quoted: the error where it wasn't fitted.

**The compact fidelity.** A design names the model's file next to its job file, `compact =
"ring.compact.json"` in its `[device]` table (a relative path is from the job file's folder,
`Job::dir`; the studio saves it beside the design). Its device then has the compact fidelity
after its analytic one: its method names the source fidelity and its grid, the poles, the
interpolation and the samples; its stated error is the held-out $\lvert\Delta S\rvert$ against the
source; it holds over the covered ranges, the band, and the other parameters at the values it was
fitted at. Its component is the model, its parameters the covered ones with exact derivatives in
them (`Component::derivatives`), so `device::derivative` and the circuit adjoint take them exactly.
A design whose other parameters differ from those values, a covered value outside its range, or a
stack it wasn't fitted on, is refused at the compact fidelity and says why. A run of a design with
a compact model keeps a copy of the file in the run's folder, so its job runs again from there;
its studies run here rather than on a farm's workers, who haven't the file.

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
| `device/mmi-library` | the MMIs' 2D models and the Y-branch's analytic one at their defaults against the studio library's components as built before | exact | 0 |
| `device/mmi-self-imaging` | the MMIs' first two-fold images against Soldano & Pennings's Eqs. 4, 6, 33 and 37, 3 to 12 µm wide | exact slab modes | 0.91 % |
| `device/mzi-closed-form` | both MZIs against their transfer matrices' closed forms | exact | 3.7e-16 |
| `device/mzi-derivatives` | the MZIs' exact $dS/dp$ (arms' lengths, phase, loss) against fourth-order differences | exact | 3.2e-10 relative |
| `device/rib-wide-slab` | a wide rib's 3D n_eff, extrapolated in $1/W^2$, against its film's slab index | 50 nm, Hadley | 3.417137 (3.417150 ± 2e-5) |
| `device/rib-eim` | the rib's effective index method against its 3D model over its stated range | fitted 20 nm, Hadley | 0.0638 |
| `device/rib-hadley` | Hadley's Table I through the rib device | 50 nm, Hadley | 1.4e-6 |
| `device/fdfd-straight` | a straight strip through the FDFD solves: $S_{21} = e^{i\beta L}$, no reflection, reciprocal | 100 nm (55 nm up) | 2.6e-5 |
| `device/mmi-2d-grid`, `device/mmi-guided-2d` | a 3 µm MMI's 2D FDFD from 10 to 5 nm; its guided modes against it | 20, 10, 5 nm | 2.1e-3 and 1.4e-3 |
| `device/y-branch-2d-grid`, `device/y-branch-ideal-2d` | Zhang et al.'s Y-branch's 2D FDFD from 20 to 10 nm; the ideal model against it | 40, 20, 10 nm | 4.2e-4 and 0.032 |
| `device/study-sweep-analytic` | a coupler's sweep in its gap and length as a job against the device at each pair of values | exact | 0 bits differ |
| `device/study-threads` | a ring's radius sweep and a waveguide's corners on 1 and 20 threads | exact | 0 lines differ |
| `device/study-corners-closed-form` | a waveguide's length and loss corners against $10^{-\alpha L/10}$, the extremes where it puts them | exact | ≤ 1e-12 |
| `device/study-coupler-chrostowski` | Chrostowski & Hochberg's Eq. 4.14 through a 3D gap sweep of the coupler device: decades of $L_x$ per nm of gap | fitted 10 nm | 3.8635e-3 (3.7645e-3 ± 5 %) |
| `device/study-coupler-grid` | that slope on 10 nm cells against 5 nm | fitted 10, 5 nm | 3.86521e-3 against 3.86633e-3 |
| `device/compact-ring-closed-form` | the all-pass ring's closed form fitted over its gap (0.15 to 0.25 µm, a grid of 7, 12 poles, degree 4, 1.54 to 1.56 µm) and used as its compact fidelity, against the closed form at 4 gaps neither fitted nor held out | exact | 3.3e-7 |
| `device/compact-ring-held-out` | the same model's error at its 6 held-out gaps, the error it states | exact | 2.9e-7 |
| `device/compact-ring-convergence` | its held-out error from 3, 5 and 7 samples at degree 1, 2 and 4 | exact | 9.3e-3, 3.7e-5, 2.9e-7 |
| `device/compact-ring-derivatives` | the compact fidelity's exact $dS/d(\text{gap})$ through the device against central differences | exact | 1.7e-9 relative |
| `device/compact-ring-passive` | its largest singular value over its band and range | exact | 0.999997 |
| `device/compact-threads` | a fit over the gap and radius (a Latin hypercube of 12) on 1 and 20 threads, and its file read back | exact | 0 lines differ |
| `device/compact-waveguide-3d` | the 3D strip fitted over its width, 0.49 to 0.51 µm (a grid of 4, 2 held out, 8 poles, degree 2, 11 wavelengths): held out | fitted 10 nm, Hadley | 1.1e-4 |
| `device/compact-bend-2d` | the 2D bend over its radius, 9.98 to 10.02 µm (2D) | exact bent slab | 5.4e-5 |
| `device/compact-coupler-3d` | the 3D coupler over its gap, 0.19 to 0.21 µm | fitted 10 nm | 2.5e-4 |
| `device/compact-all-pass-3d`, `-add-drop-3d` | the 3D rings over their gap, at 41 wavelengths over 4 nm (a resonance resolved) | fitted 10 nm | 2.0e-4 and 1.8e-4 |
| `device/compact-rib-2d` | the 2D rib over its width (2D) | exact slabs | 3.0e-5 |
| `device/compact-mmi-1x2-2d`, `-2x2-2d` | the MMIs' 2D guided modes over their width, 5.88 to 6.12 µm (2D) | exact slab modes | 1.5e-3 and 1.5e-3 |
| `device/compact-y-branch-2d` | the Y-branch's 2D FDFD over its junction's length, 1.9 to 2.1 µm (2D) | 40 nm cells | 1.5e-2 |
| `device/compact-mzi-3d` | the 3D MZI over its phase, −10 to 10°, at 41 wavelengths over 2 nm | fitted 10 nm | 1.7e-5 |
| `device/thermo-analytic-metal`, `-doped` | the shifters' analytic $P_\pi$ over the 2D model's at seven designs each over their validity (the ratio farthest from 1) | heat 20 nm to 0.5 µm, mode 10 nm | 0.873 and 1.147 |
| `device/thermo-analytic-time-metal`, `-doped` | their analytic 1/e time and −3 dB frequency over the 2D model's, three heater widths each | the same, BDF2 0.2 and 0.1 µs | 1.024 and 1.507 |
| `device/thermo-jacques-tin`, `-npp` | Jacques et al.'s simulated heaters through the devices, gated by grids twice as fine: $P_\pi$ (the TiN's four variants' midpoint) | the same and half | 25.35 and 26.09 mW (23.9 and 25.5 ± 10 %) |
| `device/thermo-tin-measured`, `-npp-measured` | the 2D $P_\pi$ at the fabricated designs, gated, over the measured fidelity's (29.2 and 22.8 mW) | the same and half | 1.142 and 1.144 |
| `device/thermo-2d-grid` | the 2D $P_\pi$ at the fabricated designs on the devices' grids over twice as fine | the same and half | 1.00042 |
| `device/thermo-jobs` | both shifters at each fidelity as jobs against `Device::component`, figures and S-parameters | the same | 0 bits differ |

The Y-branch MZI has none: its 3D model's FDFD of its Y-branches is more than the direct solver
holds here (7.3 million unknowns), as for the MMIs' and the Y-branch's own 3D models (#377). The
Y-branch's fit is the coarsest: its 2D FDFD's spectrum on 40 nm cells fits no closer than 1.5e-2
at its own samples with 8 poles, so its error is that and not the interpolation's. Each held-out
error is the one its compact fidelity states, against its source.

The published examples go through the devices too, to their bits: `strip_waveguide` solves the
waveguide device's cross-section, `directional_coupler` the coupler device's 3D model and
cross-sections, `mzi_dwivedi` the waveguide device's 3D model, `ring_q_factor` the ring devices
on its stated guide, and `bend_loss` prints the bend device's 2D and 3D models at 2 µm; their
outputs are what they were. The unit tests check that each device's cross-section is the one each
example built by hand, to the bit, and that the studio's library entries are the components it
built before the devices.

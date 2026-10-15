---
title: "Stacks, etches and sidewalls"
module: stack
summary: "Custom layer stacks, partial etches that leave a slab, and sloped sidewalls that make a strip's cross-section a trapezoid, read the same way by every solver: points for FDTD, FDFD and the pictures, the exact share of each material in a cross-section's cells for the mode solver, the material's height over each point for the effective index method, and etched bodies for FDTD's subpixel smoothing."
order: 1.55
papers:
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002) (the standard rib, Table I)"
    doi: 10.1109/JLT.2002.800371
  - cite: "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008) (the full-vector scheme)"
    doi: 10.1109/JLT.2008.923643
  - cite: "A. Farjadpour et al., Opt. Lett. 31, 2972 (2006) (subpixel smoothing)"
    doi: 10.1364/OL.31.002972
  - cite: "E. Snitzer, J. Opt. Soc. Am. 51, 491 (1961) (the round rod's exact modes)"
    doi: 10.1364/JOSA.51.000491
  - cite: "I. E. Sutherland, G. W. Hodgman, Commun. ACM 17, 32 (1974) (clipping a cell by a line)"
    doi: 10.1145/360767.360802
validation:
  - stack/vertical-sidewall-bits
  - stack/sloped-wall-fill
  - stack/wide-rib-slab
  - stack/rib-hadley-table
  - stack/rib-order
  - stack/oblique-rod
  - stack/sidewall-convergence
---

A chip is a stack of layers, and what is drawn on a layer is what its etch left standing.
`photonoxide::stack` describes both: any number of layers, each with its thickness, the
material of what is drawn and the material elsewhere (its background), between a substrate and
a cladding; and the shapes on each layer with the etch that cut them. The named stacks
(`soi_220`, `soi`, `nitride`) are presets of it, the same bits as always.

## Etches

A shape is drawn with an `Etch` (`Structure::draw_etched`):

- **its depth** from the layer's top: through the layer (a strip, the default), or less, which
  leaves the layer's material below the etch's floor across the whole layer (a rib on its
  slab);
- **its sidewall**, the walls' angle θ from the layer's plane: 90° (the default) for vertical
  walls, less for walls that lean out going down.

The drawn outline is the top face. At a depth δ below the layer's top, a shape reaches out to
every point within δ cot θ of its outline, through the shape's exact signed distance
(`geometry::Region::distance`): a rectangle's section is a trapezoid, exactly, and its convex
corners round as an etch rounds them. A point of the layer is its material when a shape holds
it so or it lies below a partial etch's floor; elsewhere it is the layer's background. Several
shapes' etches on one layer combine so. A slab of limited extent, or several etch levels with
their own slabs, are layers of their own.

`Structure::draw` is `draw_etched` with a full, vertical etch, and a layer whose shapes all are
so is "plain": every query of it takes the path it always took, to the bit.

## What each solver reads

- **Points** (`Structure::material_at`): FDTD in 3D, FDFD in 3D through its permittivity
  function, the pictures (`top_view`, `side_view`).
- **Boxes' fill** (`Solver3d::from_structure`, `top_view_averaged`, `side_view_averaged` with
  an averaging that reads the fill): a box cut into slices by the layers, each slice's shares
  its shapes' fill of the box's face, cut again at a partial etch's floor; a slice under sloped
  walls is sampled ([averaging from the fill](averaging.md)).
- **A cross-section's cells** (`Structure::cut_cell`): a cell that a sloped wall crosses, within
  one layer, gets the share of it the layer's material fills and the wall's normal. The wall is
  taken as the plane through the point of its outline nearest the cell's centre, d − δ cot θ = 0,
  and the cell is clipped by it (Sutherland and Hodgman's clipping, one line): exact for a
  straight edge cut square, a trapezoid's side. Every other cell is the material at its centre,
  as the solvers' cross-sections always were; vertical walls, layer interfaces and an etch's
  floor are on nodes. A modes job puts a sloped wall's top and foot on nodes too, and the
  devices' cross-sections (`device::section`) read their cells the same way.
- **The effective index method's columns** (`"fdfd"` and 2D `"fdtd"` jobs): at each point of the
  plane the column holds the layer's material up to the height `Structure::kept` gives there and
  its background above, a four-layer slab (`mode::multilayer`) between what lies just below and
  above. Its index is solved at the heights the etches stand at (the layer's top and bottom and
  each floor) and, under a sloped wall, at 64 heights across the layer, and taken between them as
  n² linear in the height: exact for vertical walls. A 2D job's etches are on its own layer.
- **FDTD's subpixel smoothing**: `fdtd::Body::etched(shape, [bottom, top], θ)` is the same
  mesa: its surface is the wall's plane through the outline's nearest point and the two faces,
  so a cell on a straight sloped wall takes the exact share and normal of Farjadpour et al.'s
  average (a unit test checks it against the half-space of the wall's plane, to 1e-14).
- **Bodies over the stack** (`Structure::place`, docs/methods/bodies.md): a body in space filled
  with a catalogue material or a graded medium, over the layers, their shapes and the bodies
  before it. `Structure::permittivity_at` answers at every point, a graded medium's value
  included, and is what a 3D `"fdtd"` job's permittivity reads; `material_at` gives a catalogue
  body's material.

## The cross-section's average

A cell a wall crosses takes the mean of its two permittivities by their shares. The full-vector
scheme of Fallahkhair et al. assumes every interface on its grid lines; a sloped wall is a run
of cells between them, and each corner of that run is a small dielectric corner where the scheme
is first order. There are about 1/h of them, so on an oblique interface its error doesn't vanish
with the grid, whatever the cells hold. Against exact answers on a 5 nm grid:

| average | silicon rod, r = 0.3 µm, in silica (HE₁₁ 3.001088) | 500 × 220 nm strip turned 30° (on the grid: 2.447600) |
|---|---|---|
| staircase (the centre's material) | 3.007144 (+6.1e-3) | 2.456909 (+9.3e-3) |
| mean, by share (taken) | 3.002681 (+1.6e-3) | 2.446795 (−8.1e-4) |
| ε tensor of Farjadpour et al. | 2.997636 (−3.4e-3) | 2.432261 (−1.5e-2) |

(silicon 3.476, silica 1.444, 1.55 µm, the rod's exact index from Snitzer's characteristic
equation.) The tensor, which is right on Yee's grid (FDTD's smoothing), does worse here. The
mean it is, with a sloped wall's n_eff good to about 2e-3 on such grids; vertical walls and
partial etches, on nodes, keep the scheme's own accuracy, and Hadley's equations
(`mode::hadley`) apply to them.

## Jobs

A task's stack is a preset's name, or a `[task.stack]` table: the layers bottom to top, each
thickness a number in µm (`thickness_um`) or an expression of the task's parameters
(`thickness = "t"`), each material a catalogue id or a table with its model, temperature and
composition, as a device's `[device.stack]` has them. A shape takes an `etch`:

```toml
[task.stack]
substrate = "si"
cladding = "sio2"

[[task.stack.layer]]
name = "BOX"
thickness = "2 um"
material = "sio2"
background = "sio2"

[[task.stack.layer]]
name = "Si"
thickness = "220 nm"
material = { id = "si", model = "li-1980" }
background = "sio2"

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size = ["10 um", "500 nm"]
etch = { depth = "150 nm", sidewall = "80 deg" }   # or depth_um, sidewall_deg
```

(`layer` was already a task's key, the layer the job looks at, so the stack's layers sit under
`[task.stack]`.) `job::check` refuses a depth that isn't within the layer, a sidewall outside
(0, 90°], an angle without its unit, and a 2D job's etch on another layer than its own.
`job::preview` draws an etched layer in bands: the top band keeps the layer's name, the ones
below are `Si 2`, `Si 3`, …, a sloped etch in four, each band's outlines at its middle, a band
under a partial etch's floor the layer's material all through.

## Validation

- **A vertical wall keeps the bits** (`stack/vertical-sidewall-bits`): a strip drawn with a 90°
  sidewall, and on a layer another shape's sloped etch makes an etched one, has the cells and
  n_eff of the strip drawn as before (20 nm grid): difference 0.
- **The shares are exact** (`stack/sloped-wall-fill`): a trapezoid's cells add up to
  (w + d cot θ) d at 60, 75 and 85°, its walls between a graded grid's nodes, and a rib's mesa
  and slab likewise, to 1e-12 relative.
- **A wide rib tends to its slab** (`stack/wide-rib-slab`): Hadley's rib with a 0.5 µm slab, 8
  and 16 µm wide on a 50 nm grid, extrapolated in 1/W², gives 3.417137 against the 1 µm film's
  exact TE index 3.417150 (−1.3e-5).
- **Hadley's standard rib** (`stack/rib-hadley-table`, published): 3 µm wide, a 1 µm film of
  3.44 on 3.40 under air at 1.15 µm, drawn as a partial etch leaving slabs of 0 to 0.7 µm, by his
  equations on a 50 nm grid with 3.5 µm of slab beside the rib and 4 µm of substrate under it:
  every value of his Table I within 1.4e-6 (his bounds 2–3e-6), the window's walls changing
  them by less than 2e-7.
- **It converges as Hadley says** (`stack/rib-order`): order 2.68 from 100 to 25 nm at a 0.5 µm
  slab, "from second to third".
- **An oblique interface** (`stack/oblique-rod`): the rod above on a 5 nm grid, 3.002681
  against 3.001088: the error the table above states, and the bound a sloped wall's n_eff
  carries.
- **A sidewall's effect converges** (`stack/sidewall-convergence`): n(80°) − n(90°) of a
  500 × 220 nm silicon strip in silica at 1.55 µm, 0.055476, 0.055785 and 0.055950 at 10, 5 and
  2.5 nm: the changes shrink by 0.53, first order.

All of these are 2D cross-sections or 2D estimates of a mode: never a device's performance.

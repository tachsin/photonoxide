---
title: "Bodies in space and graded media"
module: geometry
summary: "Bodies in three dimensions (half-spaces, boxes, spheres and ellipsoids at any angle, cylinders and cones along any axis, any planar shape extruded between two heights with sloped walls, and their booleans), each answering inside or not, its exact signed distance and normal, its bounds and volume, and the volume it fills of a cell: exact for planar faces by clipping and for quadrics by their elliptical slices, to a stated tolerance where two curved surfaces cross a cell. FDTD's subpixel smoothing reads these bodies, to the bit as it read its own. Graded media, n(x, y, z) or ε(x, y, z) as expressions with units, read by FDTD, 3D FDFD and the mode solvers at their points. Bodies and graded media in jobs; Akahane et al.'s L3 cavity as a body, in 3D FDTD."
order: 1.535
papers:
  - cite: "A. Farjadpour et al., Opt. Lett. 31, 2972 (2006) (subpixel smoothing: a cell's share of the tangent plane)"
    doi: 10.1364/OL.31.002972
  - cite: "I. E. Sutherland, G. W. Hodgman, Commun. ACM 17, 32 (1974) (clipping a polygon, and a polyhedron's faces, by a plane)"
    doi: 10.1145/360767.360802
  - cite: "L. F. Richardson, Phil. Trans. R. Soc. Lond. A 210, 307 (1911) (extrapolating a discretization's error away)"
    doi: 10.1098/rsta.1911.0009
  - cite: "V. A. Mandelshtam, H. S. Taylor, J. Chem. Phys. 107, 6756 (1997) (harmonic inversion, for the cavities' frequencies and Q)"
    doi: 10.1063/1.475324
  - cite: "Y. Akahane, T. Asano, B.-S. Song, S. Noda, Nature 425, 944 (2003) (the L3 cavity and its shifted end holes)"
    doi: 10.1038/nature02063
validation:
  - geometry/bodies/volumes
  - geometry/bodies/box
  - geometry/bodies/extrusions
  - geometry/bodies/sloped-walls
  - geometry/bodies/distances
  - geometry/bodies/booleans
  - geometry/bodies/oblique
  - geometry/bodies/tangent-order
  - geometry/bodies/fdtd-bits
  - geometry/bodies/akahane-l3
  - mode/graded-parabolic
  - mode/graded-parabolic-order
  - fdtd/graded-parabolic
  - fdtd/graded-parabolic-order
---

A photonic structure is mostly layers and what is etched into them (docs/methods/stacks.md),
but not all of it: a microsphere, a pillar standing on a chip, a tapered fibre's tip, a
photonic crystal's holes through a membrane, a diffused guide whose index falls off smoothly
with depth. `photonoxide::geometry::Body` is a region of space, as `Region` is one of the plane
(docs/methods/geometry.md), and `photonoxide::material::Graded` a medium that changes from point
to point. Coordinates are µm, as `[x, y, z]`, volumes µm³.

## Bodies

| body | made by | its surface |
|---|---|---|
| half-space | `Body::half_space(point, normal)` | a plane |
| ellipsoid at any angle; with one or two infinite semi-axes an elliptic cylinder or a slab | `Body::ellipsoid(center, semi_axes, axes)`, `Body::sphere`, `Body::ellipse` (a cylinder along z) | a quadric |
| box | `Body::cuboid(a, b)`, axis-aligned | six planes |
| cylinder, cone or frustum along any axis | `Body::cylinder(base, top, r)`, `Body::cone(base, top, [r0, r1])` | a quadric and two planes |
| a planar shape extruded without end | `Body::extruded(shape)` | the shape's boundary, vertical |
| a planar shape between two heights, its walls at a sidewall angle | `Body::etched(shape, [z0, z1], θ)`, `Body::prism` (θ = 90°) | the top and bottom faces, and walls leaning out going down |
| union, intersection, difference, xor | `Body::boolean(op, a, b)` | the operands' |

An extrusion takes any shape the plane has: primitives, polygons, paths, the regions defined by
functions, booleans, offsets and arrays (docs/methods/geometry.md, booleans.md, arrays.md). Its
sloped walls are phase 8's (`stack::Etch`, docs/methods/stacks.md): at a depth δ below the top
face, the shape grown by δ cot θ, with round joins, so a strip's cross-section is a trapezoid and
a disk's body a cone's frustum.

Every body answers:

- `locate(p)` and `contains(p)`: inside, on the surface or outside, from its definition: the sign
  of the quadric, the box's sides, the shape's own test in the plane.
- `distance(p)`: the signed distance to the surface, negative inside, and the outward normal at
  the nearest point. Exact for the half-space and the box; for the cylinder and the cone, the
  distance in the plane through their axis from the point to the profile they turn (base, side,
  top); for the ellipsoid, its nearest point by the Lagrange condition xᵢ = eᵢ² yᵢ/(t + eᵢ²) in its
  own frame, which leaves one equation Σ (eᵢyᵢ/(t + eᵢ²))² = 1 monotone in t where its root lies,
  solved by bisection to the last bit (a point on a principal plane takes the plane's ellipse,
  unless its nearest point lies off it). An extrusion's is the distance in the plane of
  $(s, z)$, $s$ the shape's own signed distance, to the profile its faces and walls make: exact
  where $s$ is and the nearest point is regular, and never more than the true distance, since
  $s$ is 1-Lipschitz (the wall's plane distance $(s - (z_1 - z)\cot\theta)/\sqrt{1 + \cot^2\theta}$
  bounds the true one from below). Booleans compose their operands' by min and max: exact
  outside a union and inside an intersection, a bound below the true distance elsewhere.
- `bounds()`: the smallest axis-aligned box, infinite along an axis it has no end along.
- `volume()`: closed forms ($\frac43\pi abc$, $\pi r^2 L$, $\pi L (r_0^2 + r_0 r_1 + r_1^2)/3$,
  a prism's $A h$, a disk's frustum), otherwise the fill of its bounds.
- `fill(cell)`: the volume inside an axis-aligned cell, what averaging over a grid's cells needs
  (below).
- `section(z, window)`: the planar shape it fills at a height, for drawing it.

## The volume a body fills of a cell

**Planar faces, exactly.** A half-space's share of a box is the volume its plane cuts off, in
closed form (the share Farjadpour et al.'s smoothing takes, the same function FDTD has always
used); a slab is two half-spaces whose union is everything, $V_1 + V_2 - V$; a box the product of
the overlaps along each axis; an extrusion with vertical walls the shape's own area in the cell's
footprint (exact for polygons, by clipping each boundary piece; to the shape's own tolerance for
curves) times the heights the two share. Several planes meeting in a cell, a sloped wall against
the top face, are clipped from the cell one after another, each face by Sutherland and
Hodgman's clipping with the cut closing it, the polyhedron's volume by the divergence theorem.

**Quadrics, by slices.** The planes across one of the grid's axes cut an ellipsoid (at any
angle), an elliptic cylinder (across the axis nearest its own) and a cylinder or cone along a
grid axis in ellipses: writing the body as $(q - c)^T M (q - c) \le \kappa$, with $A$ the part
of $M$ in the plane and $b$ its column along the slicing axis, the slice at $c_s + \delta$ is
$(w - w_0)^T A (w - w_0) \le \kappa - \sigma\delta^2$, $w_0 = c_w - \delta A^{-1}b$, $\sigma = m -
b^T A^{-1} b$ (a cone's $\kappa$ is zero, its $\sigma$ negative; a cylinder's $\sigma$ zero).
Each slice's area inside the cell is exact, its arc clipped to the cell in closed form. The area
is analytic in $\delta$ but where the slice vanishes, touches a side of the cell, or passes
through a corner: each the root of a quadratic in $\delta$, found in closed form. Between them the
area is integrated by 20-point Gauss–Legendre after the substitution
$v = a + (b - a)\sin^2(\pi\tau/2)$, which makes the square-root behaviour at those points
analytic; a piece is halved until its halves agree with it to $10^{-11}$ of its box. A sphere's,
an ellipsoid's, a cylinder's and a cone's fills over a grid sum to their volumes to about
$5 \times 10^{-15}$, a cell crossed by a sphere's surface in 40 to 60 µs.

**What two surfaces cross.** A boolean, a cylinder or cone along an oblique axis (its side, a
quadric whose slices across the grid axis nearest its own are ellipses, against the slab between
its caps), an extrusion with sloped walls: a cell that one surface alone crosses takes that
surface's own exact fill; a cell that two cross is halved along each axis, down to six levels,
where each surface is its tangent plane through its nearest point to the part's centre and the
planes are clipped exactly. A curved surface strays from its tangent plane by $\kappa\rho^2/2$
across a part of radius $\rho$, so where two surfaces meet in a curve the error falls as the cube
of the last level's size, which Richardson's extrapolation between the last level and its halves,
$(8V_\text{halves} - V)/7$, removes to the next order. A sloped wall is planar between the
outline's vertices: a part whose planes are the same at its centre and its eight corners is taken
from them, exactly; the rest (the round corners, a curved outline) halved to six levels and
extrapolated as $(4V_\text{halves} - V)/3$, its error falling as the square of the size over a
surface. Booleans of two spheres come within $1.3 \times 10^{-7}$ of their volumes, oblique cylinders
and cones within $1.5 \times 10^{-8}$, a rectangle's sloped walls within $3.7 \times 10^{-8}$.

## FDTD's smoothing reads these bodies

`fdtd::Body` is `geometry::Body`: FDTD's subpixel smoothing (docs/methods/fdtd.md) takes each
body's surface near a cell as the plane through its nearest point, with its normal, and the share
of the cell the plane cuts off. It reads `Body::surface`: the exact distance for every body but
the ellipsoid, whose estimate $f/\lvert\nabla f\rvert$ (second order in itself) the smoothing has
always taken. The half-space, the ellipsoid, the extruded and the etched shapes it had moved here
with the same arithmetic, and FDTD's smoothing gives the same bits as before. The check keeps
FDTD's bodies as they were, word for word, and compares, bit for bit on whatever system it runs,
what the smoothing takes from them: each body's surface (distance and normal) and its box's test
at 15 000 points through eight bodies of every kind, and the share of a cell a plane cuts off for
20 000 planes and cells (its edges and faces too, as the triplets take them). The smoothing's
averages, tensors and triplets are the same code on these, so their bits are the same too. The new bodies take part as
the old ones do: a box, a cylinder or a cone, a boolean (its composed surface), the L3 cavity
below as one body.

## Graded media

`material::Graded` is a refractive index $n(x, y, z)$ (`Graded::index`) or a relative
permittivity $\varepsilon(x, y, z)$ (`Graded::permittivity`): an expression (docs/methods/
expressions.md) of the coordinates, lengths with their units, and of named parameters, a plain
number: `1.5 + 0.1*exp(-(x/2um)^2 - (z/0.5um)^2)`, a diffused guide's $\operatorname{erf}$
profile, a GRIN lens's parabola. It has no dispersion, and being real it is never active;
`Graded::check` refuses it where $\varepsilon$ isn't positive and finite, and every solver that
samples it refuses what it finds there so.

Each solver samples it at its own points, as it samples any permittivity:

- FDTD's `Simulation::new` and 3D FDFD's `Solver3d` average a permittivity over each value's
  cell from $8^3$ points, harmonic along the component and arithmetic across it;
- FDTD's smoothing (`fdtd::Structure::with_graded`) takes it at the point each value's cell is
  about (the nodes, for `Coupling::Triplets`), and where the body's surface crosses a cell, its
  value at the cell's centre on the inside of the tangent plane;
- the mode solvers' planar profiles (`mode::slab_fd::Profile::uniform`) and cross-sections take
  it at their cells' centres.

For a medium smooth over a cell each is second order, the order of the schemes; a jump in the
index is a body's surface, which the smoothing takes as any.

**The parabolic medium.** In $\varepsilon(x) = n_0^2(1 - 2\Delta x^2/a^2)$, a wave
$\psi(x)e^{i\beta y}$ with $\psi'' + (k^2\varepsilon - \beta^2)\psi = 0$ is the harmonic
oscillator $-\psi'' + \Omega^2x^2\psi = (k^2n_0^2 - \beta^2)\psi$, $\Omega = kn_0\sqrt{2\Delta}/a$, so
its modes are exactly $k^2 n_0^2 - \beta_m^2 = (2m + 1)\Omega$. The planar mode solver, the index
an expression with its units ($n_0$ = 1.5, $a$ = 10 µm, $\Delta$ = 0.01) read at its cells'
centres across ±25 µm, finds the four lowest TE modes at 1.55 µm within $3.1 \times 10^{-8}$ of
$n_\text{eff}$ on 12.5 nm cells, converging at order 2.00. FDTD, the medium ($a$ = 2 µm, $\Delta$ =
0.05) a graded body of its smoothing structure, $E_z$ between Bloch sides at $\beta = 2\pi$ rad/µm,
finds the three lowest modes' frequencies, $n_0^2\omega^2 - (2m + 1)n_0\sqrt{2\Delta}\omega/a -
\beta^2 = 0$, by harmonic inversion within $1.0 \times 10^{-4}$ on 12.5 nm cells (the leapfrog's own
second-order error), converging at order 1.98.

## In jobs

`"structure"` jobs and 3D `"fdtd"` jobs take `[[task.body]]` tables, each placed over the
stack, its shapes and the bodies before it (`stack::Structure::place`):

```yaml
task:
  parameters:
    r: "300 nm"

  body:
    - type: sphere            # "ellipsoid" (semi_axes, angle about z), "box" (center, size),
      center: [0, 0, "2.5 um"]  # "cylinder" (base, top, radius), "cone" (base, top, radii)
      radius: r
      material: si            # a catalogue id or table, as a layer's

    - type: extrusion         # a shape between two heights, its walls at a sidewall angle
      z: ["2.22 um", "2.52 um"]
      sidewall: "80 deg"      # 90 deg when not given
      of: [{ type: circle, center: ["2 um", 0], radius: "1 um" }]
      index: "1.5 + 0.1*exp(-((x - 2um)/1um)^2 - (y/1um)^2)"  # a graded medium

    - type: difference        # also union, intersection, xor: of two bodies or more
      permittivity: 4
      of:
        - { type: box, center: [0, "3 um", "2.5 um"], size: ["2 um", "0.8 um", "0.4 um"] }
        - { type: cylinder, base: [0, "3 um", 2], top: [0, "3 um", 3], radius: "0.2 um" }
```

- A body's places and sizes are expressions with units of the task's parameters (or numbers in
  µm in their `_um` keys), its angles expressions with their unit (or `_deg` numbers).
- Its medium is `material` (a catalogue id or table), or `index` or `permittivity`: an expression
  of x, y and z, constant or graded. A body in another's `of` has none of its own.
- `job::check` names what it refuses: the table, its type and the field (`[[task.body]] #2
  (ellipsoid), angle: needs an angle, got a number: give its unit`), an unknown type, a field the
  type doesn't take, two media, an expression that reads a coordinate it can't or isn't a plain
  number, a body that isn't one (a cone with no radius); a 2D job's body (a body is three-
  dimensional); a graded medium that isn't positive and finite over the window's part of it.
- `job::preview` draws each body as bands of its sections over its height in the window: one
  band for a box, a prism or a cylinder along z, twelve for the rest; listed before the stack's
  layers, with nothing around them.
- A 3D `"fdtd"` job's permittivity is the body's where one holds the point (a graded medium's
  value there), the stack's elsewhere, averaged over each value's cell as any is.
- `stack::Structure::permittivity_at` is what every solver that samples a structure with bodies
  reads; `material_at` gives a catalogue body's material, and passes over a graded one, which has
  none.

## The L3 cavity of Akahane et al.

Their cavity (Nature 425, 944 (2003), Fig. 1 and p. 945): a silicon slab 0.6a thick (0.25 µm)
in air, a triangular lattice of air holes of radius 0.29a (0.12 µm), $a$ = 0.42 µm, three holes
missing in a line, and the holes at both its ends moved out by 0.15a (Fig. 1c), the shift that
makes the confinement gentle. As one body: a box of silicon less the lattice's disks extruded
through it. Its volume from its cells' fills over a grid of a/4 cells comes within
$7.8 \times 10^{-9}$ of $T(\lvert A\rvert - N\pi r^2)$, the cavity's sites and the band the shift
uncovers are silicon and every hole air through the slab, and FDTD's smoothing structure on the
body answers the same (`geometry/bodies/akahane-l3`).

In 3D FDTD (`examples/l3_akahane.rs`): the body, the catalogue's silicon at 1.57 µm, 21 × 15
sites, CPMLs of 12 cells and $0.7a$ of air above and below the slab inside them; FDTD's smoothing
with `Coupling::Diagonal`; a pulse of $E_y$ at the cavity's centre, $0.04/a$ wide about
$a/\lambda$ = 0.264; $E_y$ near the centre after it, by harmonic inversion: the fundamental mode's
frequency and $Q = \omega/2\gamma$.

| cells a lattice constant | record after the pulse | unshifted: $a/\lambda$, $\lambda$, Q | shifted 0.15a: $a/\lambda$, $\lambda$, Q | Q's rise |
|---|---|---|---|---|
| 8 (52.5 nm; 200 × 135 × 40) | 300 µm/c | 0.2635, 1.5941 µm, 5574 | 0.2618, 1.6042 µm, 56 156 | 10.1 |
| 12 (35 nm; 288 × 190 × 48) | 1000 µm/c | 0.2641, 1.5904 µm, 5413 | 0.2623, 1.6013 µm, 52 275 | 9.7 |
| 16 (26.25 nm; 376 × 246 × 56) | 1000 µm/c | 0.2639, 1.5912 µm, 5107 | 0.2621, 1.6025 µm, 38 182 | 7.5 |

The frequencies settle to 1e-3 of themselves, 0.7 % (unshifted) and 1.4 % (shifted) above the
top of the measured span; the shift lowers the frequency by 0.7 %, a little longer cavity. The
shifted cavity's Q, its field falling by 4 % over the record, doesn't yet settle with the grid,
and is reported, not claimed, beyond a factor of 2. The example checks each resonance against
the measured span widened by 2 % at 8 cells and 1.5 % at 12 and 16, and Q's rise against 10 and
the shifted Q against 45,000, each within a factor of 2; CI runs 8 cells (about a minute and a
half on 20 threads), `--full` the others (about 40 minutes).

They print no computed numbers: their cavities' resonances lie between 1.565 and 1.580 µm (Fig.
4a's axis), and moving the end holes by 0.15a raised the measured Q/V by "a factor of 10" (Fig.
4c) with V unchanged, so Q by about 10, to 45,000. The fabricated holes' radius and the slab's
thickness, which set the resonance, are given only to their nominal values; the measured Q is
limited by the fabrication's disorder, the computed one by the 21 × 15 sites and the grid.

## Checks

| case | what | measured | tolerance |
|---|---|---|---|
| `geometry/bodies/volumes` | a sphere, an ellipsoid at an angle, a cylinder along z and a cone's frustum along x, their fills over a 9³ grid against their volumes | 5.0e-15 relative | 1e-12 |
| `geometry/bodies/box` | a box's fills of a 7 × 6 × 5 grid of unequal cells against each cell's overlap | 0 | 1e-18 µm³ |
| `geometry/bodies/extrusions` | a pentagon between two heights against $Ah$, a disk with walls at 70° against its frustum | 8.9e-16 relative | 1e-12 |
| `geometry/bodies/sloped-walls` | a rectangle with walls at 70° against Steiner's formula integrated | 3.7e-8 relative | 1e-7 |
| `geometry/bodies/distances` | distances and normals at 400 points: a sphere, a box, a cylinder, a frustum, an ellipsoid's foot and normal | 1.6e-13 | 1e-12 |
| `geometry/bodies/booleans` | two spheres' union, intersection, difference and xor against the lens they share | 1.3e-7 relative | 1e-6 |
| `geometry/bodies/oblique` | a cylinder and a frustum along an oblique axis against their volumes | 1.5e-8 relative | 1e-7 |
| `geometry/bodies/tangent-order` | the tangent plane's share (FDTD's smoothing) summed over a sphere on 32 and 64 cells a µm: the order | 2.0 | 2 ± 0.25 |
| `geometry/bodies/fdtd-bits` | the shared bodies' surfaces and boxes, and the plane's share of a cell, against FDTD's own before they moved | 0 values differ | 0 |
| `geometry/bodies/akahane-l3` | Akahane et al.'s L3 cavity as a body (above) | 7.8e-9 relative | 1e-7 |
| `mode/graded-parabolic` | the planar mode solver in the parabolic medium, 12.5 nm cells | 3.1e-8 in $n_\text{eff}$ | 1e-7 |
| `mode/graded-parabolic-order` | the same on 50 and 25 nm cells: the order | 2.00 | 2 ± 0.25 |
| `fdtd/graded-parabolic` | FDTD in the parabolic medium as a graded body, 12.5 nm cells | 1.0e-4 relative | 2e-4 |
| `fdtd/graded-parabolic-order` | the same on 50 and 25 nm cells: the order | 1.98 | 2 ± 0.25 |

Besides the cases, the unit tests check the ellipsoid's distance against a dense sampling of its
surface; the half-space's and the slab's fills against their planes and a fine sampling; that
every body's bounds hold it; the tangent plane's order; FDTD's smoothing's bits; graded media's
units, parameters and refusals; jobs with every kind of body, what they refuse, a 2D job's body
refused, and a 3D FDTD job reading its bodies and a graded medium, and refusing one that isn't
positive.

---
title: "Booleans, offsets and corner rounding"
module: geometry
summary: "Union, intersection, difference and xor of any shapes, kept exact for the solvers as a composition of the operands' inside tests and signed distances, and as polygons for the layout on an integer grid, exact on shared edges, touching vertices and collinear overlaps; offsets with round, miter and square joins; convex and concave fillets with a least radius by opening and closing."
order: 1.52
papers:
  - cite: "A. Ricci, Comput. J. 16, 157 (1973) (regions composed by the min and max of their defining functions)"
    doi: 10.1093/comjnl/16.2.157
  - cite: "A. A. G. Requicha, ACM Comput. Surv. 12, 437 (1980) (regularized set operations)"
    doi: 10.1145/356827.356833
  - cite: "R. M. Haralick, S. R. Sternberg, X. Zhuang, IEEE Trans. Pattern Anal. Mach. Intell. 9, 532 (1987) (opening and closing)"
    doi: 10.1109/TPAMI.1987.4767941
  - cite: "Y. Zhang, S. Yang, A. E.-J. Lim, G.-Q. Lo, C. Galland, T. Baehr-Jones, M. Hochberg, Opt. Express 21, 1310 (2013) (the Y-junction, drawn as a union)"
    doi: 10.1364/OE.21.001310
validation:
  - geometry/booleans/areas
  - geometry/booleans/polygons
  - geometry/booleans/degenerate
  - geometry/booleans/distance
  - geometry/booleans/offsets
  - geometry/booleans/fillets
  - geometry/booleans/convergence
  - geometry/booleans/zhang-y-junction
---

A device is rarely one primitive. A ring's bus is cut where a via lands, a waveguide's cladding
keeps a clearance from it, a process biases every width by a few nanometres, and lithography
rounds the corners it can't print. `photonoxide::geometry` composes shapes for all of these, and
the result is a shape again: `Boolean`, `Offset` and `Rounded`
are regions with the same answers every region gives (docs/methods/geometry.md), and they take
any shapes as operands, booleans among them.

Each answers in two ways, as the kernel's principle asks: exactly where its definition allows,
for the solvers, and as polygons within a tolerance, for the layout and the studio.

## Booleans, exactly

**The inside test** of A op B is the operands' combined as a *regularized* set operation
(Requicha 1980): the closure of the interior of the result. Where a point is inside or outside
each operand, the operation decides. Where it is on one boundary and not the other, the result's
membership on either side of that boundary decides: if both sides agree, the point is inside or
outside, else on the result's boundary. Where it is on both boundaries, their outward normals
decide which sides are possible: back to back (two shapes sharing an edge), a nearby point is in
one or the other; facing the same way (the same edge twice), in both or neither; crossing, any
of the four. So two rectangles that share an edge are one region across it, the edge inside
their union, and two that only touch along it have no intersection there, not a line of material
a solver's grid could sample.

**The signed distance** is the operands' composed by min and max (Ricci 1973):

| | d |
|---|---|
| A ∪ B | min(d_A, d_B) |
| A ∩ B | max(d_A, d_B) |
| A \ B | max(d_A, −d_B) |
| A △ B | min(max(d_A, −d_B), max(d_B, −d_A)) |

with the normal of the operand that gives the value (negated with its distance). The composition
has the right sign everywhere and is zero exactly on the boundary. Off it, its magnitude is a
lower bound of the true distance: inside a union, a ball of radius |d_A| about the point lies in
A, so in the union, but A's nearest boundary point may lie inside B. The composed value is
exact exactly when the point it measures to, q = p − d n, is on the result's boundary; then the
true distance is at most |p − q| and at least |d|. That holds outside a union and inside an
intersection always, and near the boundary away from where the operands' boundaries cross.

photonoxide tests it at every query: a step of 10⁻³ of the polygons' tolerance below (see next)
along the normal either side of q must be inside and outside the result. Where the test fails —
deep inside a union, near a crossing, on an edge two operands share — the distance is to the
result's polygons within 10⁻⁷ of its size instead, through a bounding-volume hierarchy over their
edges, and is within that of the truth. Within a step of the boundary, where a normal is the
rounding's, the operands' places within the step decide as the inside test does.

**Areas, perimeters, fills and bounds** are from the result's polygons within
`FILL_TOLERANCE`, 10⁻⁷ of the larger side of the operands'
box, made once when the boolean is: the operands' polygons (each arc's vertices placed so that
it keeps its area) combined as the next section does. A fill is those polygons clipped to the
cell, ring by ring, rings whose boxes miss it skipped. The bounds are the polygons' box widened
by their deviation, within the operands' boxes. An empty result (the intersection of shapes that
don't overlap, a shape less one that covers it) is refused when the boolean is made.

## Booleans on polygons

A boolean's polygons at a tolerance are its operands' at half its budget, combined on the
tolerance's grid. The combining is [i_overlay](https://github.com/iShape-Rust/iOverlay) 9.1's
integer overlay, chosen by measuring it (see "The polygon-boolean decision" below): every vertex
goes onto the integer grid (for a tolerance with a grid, its grid, so a layout's database unit;
without one, a grid of about 2⁻⁶⁰ of the polygons' size), every crossing of two edges is found
and rounded to the grid, the edges are split there, and each edge is kept or dropped by the
winding numbers of the two operands on its two sides (non-zero filling). Shared edges, touching
vertices and collinear overlaps are then integer arithmetic, exact. The outer rings come out
counterclockwise and the holes clockwise, as the kernel's polygons are.

Two things follow. Rounding can leave a needle or crack of no width where two pieces meet; each
ring is cleared of vertices where it turns back on itself (within 10⁻⁹ rad) and of vertices
within 10⁻¹³ of the coordinates' size of the one before. And *slivers*, rings thinner than the
tolerance, go: a ring is thin when no disk of the tolerance's diameter fits in it. A ring with at
least the tolerance times half its perimeter in area has room for one if it is convex and is
kept without looking further; one with less is shrunk by half the tolerance (below), and is a
sliver if nothing is left. A thin polygon goes with its holes, a thin hole is filled.

The polygons' deviation, the largest distance between them and the true boundary, is measured:
the exact distance (above) at every vertex and at every edge's middle. An operand's deviation e
moves a crossing at an angle θ by up to e/sin θ, so where they cross at a shallow angle the
operands' tolerance is cut by four and the boolean redone, until the polygons are within the
tolerance (at most six times). Polygons asked for within 100 times the region's own fine
tolerance aren't measured; their deviation is their operands'.

## Offsets

An offset grows a shape by a distance d (shrinks it for a negative one). With **round joins** it
is exactly the set of points within d of the shape, the Minkowski sum with a disk, or for a
shrink the points at least |d| inside: d_A(p) ≤ d, an inside test as exact as the operand's
distance. Its distance d_A − d is exact outside a grown shape and inside a shrunk one, and
tested and backed by polygons elsewhere as a boolean's is; a grown one's box is the operand's
widened by d, exactly. **Miter joins** continue the two offset sides at a corner until they meet,
the corner kept sharp, and cut it square at `limit` times d from the original corner where they
would reach farther (perpendicular to the corner's bisector); **square joins** are miters with a
limit of 1. Corners exist only on polygons, so a miter or square offset answers everything from
its polygons within 10⁻⁷ of its size.

On polygons, an offset is the union of the polygons with pieces along their boundary (a shrink,
the polygons less the pieces): each edge's strip, the region between it and its offset, and at
each corner that turns away from the side the offset moves to (a convex corner when growing, a
concave one when shrinking) the join: the arc's sector (flattened as the kernel flattens arcs,
keeping its area), the miter's kite or a cut miter's pentagon. Every point of every piece is
within d of the boundary, so the union never takes too much, and every point within d of it is
in a strip or a join, so it never takes too little. The arc's ends are computed as the strips'
corners are, to the bit, so that they meet without a crack.

Along a curve's polygon, many short edges, the strips would overlap their many neighbours and
the overlay pay for every crossing. So the pieces are bands: a run of edges through corners that
turn away from the side (their joins included in the band) or towards it (the band's vertex
there where the two offset sides cross, while both keep their direction), at most a quarter turn
in all, and the band is the region between the run and its offset, the union of its strips and
joins and a simple polygon. The pieces are then merged in halves, and a shrink by half the
polygons' box's narrower side or more leaves nothing at once.

For a convex polygon grown by d, the area is A + P d + d² Σ J(φ) over its corners turning by φ:
J = φ/2 for a round join (Steiner's formula, A + P d + π d², for any convex region), tan(φ/2) for
a miter, less (sec(φ/2) − L)² cot(φ/2) where the miter is cut at L d (L = 1 for a square join).
These are the validation's closed forms.

## Corner rounding

A first lithography model: every convex corner rounded to at least a radius r₁ and every concave
corner to at least r₂. It is the *opening* by r₁, the shape shrunk by r₁ then grown by r₁ with
round joins, the union of every disk of radius r₁ inside it, followed by the *closing* by r₂,
grown by r₂ then shrunk (Haralick, Sternberg and Zhuang 1987). Wherever the shape is wider than
2r₁ the opening only rounds its convex corners: a corner turning by φ loses
r₁²(tan(φ/2) − φ/2), a rectangle w × h becomes the rounded rectangle of area wh − (4 − π)r₁².
Features narrower than 2r₁ go, as they would in print; the closing fills each concave corner with
r₂²(tan(φ/2) − φ/2) and closes gaps narrower than 2r₂. Each stage's arcs are within a quarter of
what the polygons' budget leaves after the shape's own polygons. The rounding of a curve's
polygon is not its own exact rounding where a feature is within the tolerance of twice a radius
wide (an erosion is not continuous there); elsewhere the polygons are within the tolerance. A
rounded shape answers everything from its polygons within 10⁻⁷ of its size, and a rounding that
leaves nothing is refused.

## In jobs

`[[task.shape]]` takes `type = "union"`, `"intersection"`, `"difference"`, `"xor"`, `"offset"`
and `"round"`, their operands in `of` as inline tables of any shape type (the shapes defined by
functions and masks, `"rect"`, `"circle"`, `"ring"`, `"polygon"`, `"path"`, and these again),
every value an expression:

```yaml
task:
  shape:
    - type: difference
      layer: Si
      of:
        - { type: rect, center: [0, 0], size: ["10 um", w] }
        - { type: circle, center: ["2 um", 0], radius: "150 nm" }
      etch: { depth: "70 nm", sidewall: "80 deg" }

    - type: offset
      layer: Si
      distance: "-10 nm"    # a process bias
      join: miter
      miter_limit: 2
      of: [{ type: polygon, points: [[0, 0], ["2 um", 0], ["1 um", "1 um"]] }]

    - type: round
      layer: Si
      convex: "100 nm"
      concave: "60 nm"
      of: [{ type: path, start: [0, "3 um"], width: w, segments: [{ straight: "2 um" }] }]
```

A union, intersection or xor takes all of `of` in turn, a difference the first less the others,
an offset or rounding the one shape. An etch, partial or sloped, works on the result: a sloped
wall reaches out to the result's distance at each depth (docs/methods/stacks.md). `job::check`
refuses an empty result, an unknown join, a miter's limit below 1, a shrink or rounding that
leaves nothing, and an operand with a layer or etch of its own, naming the table, the operand
(`of 2`) and the field. `job::preview` draws each polygon of the result as its own outline.

## The polygon-boolean decision

Issue #227 left open whether photonoxide writes its own Vatti-style sweep (B. R. Vatti, Commun.
ACM 35, 56 (1992), doi:10.1145/129902.129906) or uses a maintained crate under MIT or Apache-2.0.
I measured the two crates that qualify on the degenerate cases that break polygon clippers, before
choosing (14 pairs, the four operations each, in µm; 2026-10-15, release build):

| | i_overlay 9.1, i32 | i_overlay 9.1, i64 | i_overlay 9.1, 1 nm grid | geo 0.33 |
|---|---|---|---|---|
| shared edge, touching vertex, collinear overlap, vertex on an edge, the same square twice | exact areas and contours | exact | exact | exact |
| a hole, a hole opening on an edge, two frames with holes | exact | exact | exact | exact |
| gap / overlap of 0.1 pm | kept: 2 polygons, a 0.1 pm sliver | kept | snapped: 1 polygon, no sliver | kept, sliver |
| edges 10⁻⁹ rad apart | exact | 4 slivers of 10⁻⁸ µm² | no sliver | 2 slivers |
| square and diamond (octagon) | 6 × 10⁻⁹ | 9 × 10⁻¹⁶ | 7 × 10⁻⁴ (the 1 nm grid) | 6 × 10⁻⁹ |
| shared edge 1 cm from the origin | exact | exact | exact | exact |
| two 100 000-gons | 23 ms, 1.4 × 10⁻⁷ | 46 ms | 13 ms | 25 ms |
| the same bits twice | yes | yes | yes | yes |

Every engine got every topology right: no wrong contour, no wrongly oriented ring, no crossing
edges in any output. geo's booleans are i_overlay's (it pins i_overlay 4.5) behind its own types,
and bring geographic dependencies photonoxide has no use for; on these cases it measured as
i_overlay's i32 engine does. i_overlay itself is MIT OR Apache-2.0, its four dependencies (its
author's i_float, i_shape, i_tree, i_key_sort) MIT, without `rayon` unless its
`allow_multithreading` feature is on, which photonoxide leaves off, so the same bits on any thread
count. Its integer engines make the degenerate cases exact by construction, and its fixed-scale
overlay is a layout's grid: the 1 nm grid snapped the 0.1 pm gap and overlap together and left
no sliver, which is what "free of slivers below the tolerance" asks.

Writing our own would have meant reproducing exactly that: an integer snap-rounding overlay. A
Vatti sweep in floating point needs its own treatment of each degenerate case above, and these
cases are the bulk of such an implementation's work; the measurements gave no case where a crate
needed working around. So photonoxide uses i_overlay 9.1 (the i64 engine, on the tolerance's
grid, or its finest scale when there is none), its own code around it being the offsets, the
rounding, the sliver and spike removal, and the measured deviation. Vatti's paper is not needed.
The validation's `geometry/booleans/degenerate` case keeps these measurements as a test: the
grid's 12 degenerate pairs, every operation, every contour and hole counted.

## Checks

- **Areas** (`geometry/booleans/areas`): the four operations on two equal and two unequal
  crossing disks, two rectangles, a disk on a rectangle's edge tangent to two of its sides, a ring
  and a half-plane, a square and a diamond, against the lens
  r₁²α₁ + r₂²α₂ − ½√((−d + r₁ + r₂)(d + r₁ − r₂)(d − r₁ + r₂)(d + r₁ + r₂)), overlaps and the
  octagon 8√2 − 8, with inclusion–exclusion among the results: 1.2 × 10⁻¹⁰ relative, from the
  polygons within 10⁻⁷ of the size cut where the circles cross (the error there is of order the
  chord times the tolerance).
- **Polygons** (`geometry/booleans/polygons`): the same at 1 nm on a 1 nm grid, within 0.82 nm,
  their areas within the tolerance times the boundary's length (0.08 of it at most), oriented and
  without slivers.
- **Degenerate pairs** (`geometry/booleans/degenerate`): as in the table above, every result's
  polygons, holes and area exact.
- **Distance** (`geometry/booleans/distance`): the four operations on two crossing disks at 225
  points about each, against the exact distance to the arcs that bound the result: 1.8 × 10⁻⁴ nm
  (the polygons' 10⁻⁷ where the composition isn't exact), and 6 × 10⁻¹³ nm at points on the
  boundary.
- **Offsets** (`geometry/booleans/offsets`): a rectangle, a regular hexagon and a triangle with a
  25° corner grown by 0.15 µm with every join and miter limits of 2 and 1.5, a disk and an
  ellipse grown, a disk and a rectangle shrunk: 4.0 × 10⁻⁸ relative (the curves' polygons'), and
  their polygons at 1 nm within the tolerance.
- **Fillets** (`geometry/booleans/fillets`): the same polygons' convex corners rounded to 0.12 µm,
  an L's concave corner, its convex ones and both, a rectangle against the rounded rectangle:
  2.3 × 10⁻⁹ relative.
- **Convergence** (`geometry/booleans/convergence`): the union of two disks, an ellipse grown and
  a rectangle rounded, at tolerances from 1 to 0.001 nm: the distance from the exact boundary
  falls as the square of the chords' length, order 2.00 in the number of vertices.
- **A published structure** (`geometry/booleans/zhang-y-junction`): Zhang et al.'s Y-junction
  (Table 1's taper between two curves, its 0.5 µm input and two 0.5 µm arms 0.2 µm apart) drawn
  as the union of four shapes that share edges with the taper: one polygon without holes, its
  area the four's to 1.3 × 10⁻¹⁴ µm², the gap between the arms kept.
- The unit tests add the regularized inside test on shared and touching edges, distances and
  normals on and off the boundary, what is refused, transforms of booleans of offsets of
  roundings, booleans of every kind of shape (paths, regions between curves, polar, implicit,
  masks), a shrink deeper than a curve's radius with every join, and fills that sum to the area
  and are the same bits on one and four threads. Every number above is on its stated tolerance;
  none is a device's performance.

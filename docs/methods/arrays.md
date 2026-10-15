---
title: "Arrays, lattices and references"
module: geometry
summary: "Many shapes as one region: rectangular, hexagonal and polar lattices whose elements' sizes and places are expressions of their index and site, defects (sites removed, shifted or resized, as in L3, H1 and W1 photonic crystals), named cells placed many times with transforms, and booleans and etches of the result; found through a hierarchy of the elements' boxes so a lattice of 10⁴ holes costs a point a handful of tests, the same bits on any number of threads."
order: 1.53
papers:
  - cite: "G. R. Hjaltason, H. Samet, ACM Trans. Database Syst. 24, 265 (1999) (best-first nearest-neighbour search in a hierarchy of boxes)"
    doi: 10.1145/320248.320255
  - cite: "Y. Akahane, T. Asano, B.-S. Song, S. Noda, Nature 425, 944 (2003) (the L3 cavity and its shifted end holes, drawn)"
    doi: 10.1038/nature02063
validation:
  - geometry/arrays/lattices
  - geometry/arrays/fill
  - geometry/arrays/index-expressions
  - geometry/arrays/defects
  - geometry/arrays/references
  - geometry/arrays/slab-less-holes
  - geometry/arrays/convergence
  - geometry/arrays/akahane-l3
---

A photonic crystal is a few hundred holes, a metalens tens of thousands of pillars, a grating a
few hundred teeth, and a chip the same cell placed again and again. `photonoxide::geometry`
draws each as one shape: an `Array` is the union of its elements, each with an index, and a
`Reference` places a named `Cell` many times. Both are regions with every answer a region gives
(docs/methods/geometry.md), so they combine with any other shape (a slab less a hole lattice is a
`Boolean`, docs/methods/booleans.md), take an etch, and are read by every solver as any shape is.

## Lattices

A `Lattice` is its sites, each with its index $(i, j)$ and its place in the lattice's own frame,
site $(0, 0)$ at the origin:

| lattice | site $(i, j)$ | built by |
|---|---|---|
| rectangular | $(i a_x,\ j a_y)$ | `Lattice::rectangular([ax, ay], [i0, i1], [j0, j1])` |
| hexagonal | $((i + \frac12 (j \bmod 2))\thinspace a,\ j a \sqrt3/2)$ | `Lattice::hexagonal(a, [i0, i1], [j0, j1])` |
| polar | $r_j(\cos\theta_{ij}, \sin\theta_{ij})$, $r_j = r_0 + j\Delta r$, $\theta_{ij} = \theta_0 + 2\pi i/n_j$ | `Lattice::polar(r0, Δr, &[n0, n1, …], θ0)` |

The hexagonal (triangular) lattice is drawn as photonic-crystal slabs are: rows along x (Γ–K),
the odd rows half a pitch to the right, so a patch $i_0 \le i \le i_1$, $j_0 \le j \le j_1$ is a
rectangle of rows and every interior site has six nearest neighbours $a$ away and six more
$\sqrt3\thinspace a$ away. On a polar lattice, $j$ is the ring and $i$ the site around it; a ring of
radius zero holds one site, the centre. The sites come row by row (or ring by ring).

**Defects** change sites by their index: `remove` (a vacancy), `shift` by a vector in the
lattice's frame, and `resize` (the element scaled about its site by a factor). An L3 cavity is
three sites removed in a row and the holes at its ends shifted out; an H1 cavity one site
removed; a W1 guide a row removed. A defect at a site the lattice doesn't have, or one already
removed, is refused, naming it: `can't remove site (9, 0): i runs from -7 to 7 and j from -4 to 4`.

## Arrays

`Array::of(&lattice, &shape)` draws one shape, given about the origin, at every site: moved to
the site (a translation, exact to the bit for every kind) and scaled about it first where a
defect resizes it. `Array::generate(&lattice, make)` makes each site's element from the site:
its index and its place. An apodized grating's teeth widen with $i$; a metalens's pillars take
their radii from the phase the lens needs at their place,
$\varphi(x, y) = \frac{2\pi}{\lambda}\left(\sqrt{x^2 + y^2 + f^2} - f\right)$; a site can also be
left empty. `Array::new` takes any elements, any shapes, with indices of their own.

**Fast for many elements.** The elements' boxes go into the kernel's bounding-volume hierarchy
(`Index`, docs/methods/geometry.md), built in a fixed order, so:

- the inside test tests only the elements whose boxes hold the point (a leaf holds up to four);
- the area inside a cell sums the fills of the elements whose boxes meet it, in the elements'
  order, so the sum is the one taken over every element;
- the signed distance outside every element is the nearest element's, found by best-first search
  (Hjaltason and Samet 1999): the hierarchy's nodes visited nearest box first, those farther than
  the best element found so far left. An element's distance is at least its box's, so the answer
  is the least over every element, and of elements equally near the first, whatever order the
  search visits them in: the same bits on any number of threads.

A slab less a hexagonal lattice of $100 \times 100$ holes (radius 0.12 µm, $a$ = 0.42 µm): the
array is built in 3 ms, the boolean in 1.4 s (its polygons within $10^{-7}$ of its size, 44 µm:
4.4 pm), and then a point's inside test takes 108 ns and a cell's fill 4.1 µs, the fills of a
$300 \times 300$ grid summing to the area to the printed digit (one thread of an Intel Core Ultra 7
265K, release build; `cargo test --release --lib array_timing -- --ignored --nocapture`). The
boolean's polygons keep their rings in a hierarchy of their own, so a point or a cell meets only
the holes near it there too.

**What is exact.** When no two elements' boxes meet (a lattice of holes or pillars,
`Array::is_separate`), every answer is the elements' own: the inside test, the signed distance
(inside an element its own distance is the union's, the disc it measures lying in that element;
outside every element the nearest one's is), the area and perimeter (their sums), the fill and
the polygons. A hole lattice's area is $N\pi r^2$ to round-off.

When boxes meet (teeth on a spine, overlapping pillars), the array is its elements' union, as a
`Boolean` union is: the inside test combines the elements' (an edge two of them share is inside),
the distance is the elements' least where the point it measures to is on the union's boundary
(always outside), and from the union's polygons within $10^{-7}$ of its size elsewhere, as are its
area, perimeter and fill. The union of the elements' polygons is taken by the integer overlay
(docs/methods/booleans.md) and measured against the distance, as a boolean's is.

## References

A `Cell` is a named list of shapes in its own frame; a `Reference` places it at each of a list of
`Transform`s (translations, rotations, mirrors, scalings, composed). Its element $(k, m)$ is the
cell's shape $m$ under placement $k$, exactly `shape.transformed(&placement)`, so a reference
answers as the shapes transformed one by one, to the bit, and a transformed reference is its
placements followed by the transform. A cell's shapes can be arrays and references again: a
hierarchy, as 0.9's layout's cells will be, the two meeting there.

## In jobs

`[[task.shape]]` takes `type = "array"` and `type = "reference"` in `"structure"`, `"fdfd"` and
`"fdtd"` jobs:

```toml
[task.parameters]
a = "420 nm"

[[task.shape]]                # a slab less a hole lattice with an L3 cavity
type = "difference"
layer = "Si"
of = [
  { type = "rect", center = [0, 0], size = ["7 um", "4 um"] },
  { type = "array", lattice = "hexagonal", pitch = "a", i = [-7, 7], j = [-4, 4],
    element = { type = "circle", radius = "0.29*a" },
    remove = [[-1, 0], [0, 0], [1, 0]],
    shift = [{ at = [-2, 0], by = ["-0.15*a", 0] }, { at = [2, 0], by = ["0.15*a", 0] }] },
]

[[task.shape]]                # a metalens's pillars, rings of 1, 6, 12 and 18
type = "array"
layer = "Si"
lattice = "polar"
pitch = "0.5 um"              # the rings' spacing; inner, start_angle optional
rings = 4
count = "max(1, 6*j)"         # sites on ring j
origin = ["10 um", 0]         # where site (0, 0) goes; angle turns the lattice about it
element = { type = "circle", radius = "60 nm + 0.01*sqrt(x^2 + y^2)" }

[[task.cell]]
name = "pair"
shapes = [
  { type = "circle", center = [0, 0], radius = "100 nm" },
  { type = "rect", center = ["0.5 um", 0], size = ["300 nm", "200 nm"] },
]

[[task.shape]]
type = "reference"
layer = "Si"
cell = "pair"
place = [
  { translate = ["6 um", "6 um"] },
  { translate = ["8 um", "6 um"], rotate = "90 deg" },
  { translate = ["10 um", "6 um"], mirror = "y", scale = 2 },
]
```

- `lattice` is `"rectangular"` (`pitch` one value or `[x, y]`, `i`, `j`), `"hexagonal"`
  (`pitch`, `i`, `j`) or `"polar"` (`pitch`, `rings`, `count`, `inner`, `start_angle`); `remove`,
  `shift` and `resize` (`{ at = [i, j], by = factor }`) are its defects.
- `element` is any shape (`circle`, `rect`, `polygon`, `path`, `polar`, a boolean, an array, a
  reference…) drawn about its site: a `center` is relative to it. Its places and sizes are
  expressions of the task's parameters and of the site, `i`, `j` its index and `x`, `y` its place
  in the lattice's frame; definitions that read coordinates of their own (a polar radius's `t`)
  see `i` and `j` too.
- A placement is mirrored (`"x"`: in the x axis, y to −y; `"y"`: x to −x), scaled, rotated about
  the cell's origin and moved, in that order, as GDSII places a cell.
- `job::check` names what it refuses: the table, the element by its index
  (`[[task.shape]] #2 (array), element (0, 2) (circle): …`), the defect's site, the placement; an
  unknown cell, a cell that places itself, a `[[task.cell]]` placed nowhere, a count that isn't a
  whole number. `job::preview` draws each element as its own outline (circles as 64-gons, as
  everywhere in a scene), and an array whose elements meet as its union's polygons.
- An `etch` applies to the array or the boolean it is in, as to any shape.

## Checks

| case | what | measured | tolerance |
|---|---|---|---|
| `geometry/arrays/lattices` | rectangular, hexagonal and polar sites against their closed forms; each interior hexagonal site's six neighbours at $a$ and six at $\sqrt3\thinspace a$ | 4.0e-16 of the pitch | 1e-14 |
| `geometry/arrays/fill` | hole lattices' areas against $N\pi r^2$, a grid's fills summed, unit cells against $2\pi r^2/(\sqrt3\thinspace a^2)$ and $\pi r^2/(a_x a_y)$ | 5.3e-15 relative | 1e-12 |
| `geometry/arrays/index-expressions` | an apodized grating's teeth and a metalens's pillars, each an expression of $i$, $j$, $x$, $y$ evaluated at every element | 1.4e-15 µm | 1e-12 |
| `geometry/arrays/defects` | L3, H1, W1 and a resized hole: removed, moved by the shift, the rest the same to the bit; defects at missing sites refused, named | 0 wrong | 0 |
| `geometry/arrays/references` | a cell placed five ways and a lattice moved and turned, against the shapes transformed one by one, at 2000 points | 0 bits differ | 0 |
| `geometry/arrays/slab-less-holes` | a slab less 17 × 13 holes by inclusion–exclusion, $\lvert A\rvert - N\pi r^2$ | 9.8e-15 relative | 1e-9 |
| `geometry/arrays/convergence` | a slab less an L3 lattice as polygons at 1 to 0.001 nm: the distance's order in vertices | 2.00 | 2 ± 0.15 |
| `geometry/arrays/akahane-l3` | Akahane et al.'s L3 cavity drawn (below) | 2.2e-16 µm | 1e-12 |

**Akahane et al. 2003.** Their cavity (Fig. 1 and p. 945): a silicon slab 0.6a thick (0.25 µm)
with a triangular lattice of air holes, $a$ = 0.42 µm, of radius 0.29a (0.12 µm); three holes
missing in a line; the holes at both edges shifted out by 0.15a, the shift that made the
confinement gentle (their Figs. 1c and 3c). Drawn here as a silicon slab in air less a $15 \times
11$ hexagonal hole lattice with the L3 defect: its 162 holes' centres and radii against the
lattice's closed form, the end holes at $\pm 2.15a$, the cavity's length between them $4.3a$ =
1.806 µm, the cavity's three sites and the band the shift uncovers ($2a - r$ to $2.15a - r$ from
the centre) silicon through the slab, every hole air, and the unshifted cavity different only at
the two end holes. Their rounded values (0.25 and 0.12 µm) match $0.6a$ = 0.252 µm and
$0.29a$ = 0.1218 µm to the printed digit. This checks the geometry only. Their resonance and Q
come from 3D FDTD and measurement; the L3 cavity's resonance is #227's phase 9, once 3D bodies
are in, and no 2D number here stands for the device's performance.

Besides the cases, the unit tests check that a point's inside test on a $100 \times 100$ lattice
tests at most 8 elements and its distance at most 40, a 0.1 µm cell's fill at most 4, the slab
less that lattice's inside test and fill at most 4 and 8 of its rings; that fills and distances
are the same bits on 1, 2, 7 and 20 threads; a comb of teeth sharing edges with a spine and two
overlapping disks as unions (area, the shared edge inside, the distance to the concave corner,
the perimeter, one polygon); an array of polar shapes, an array inside a reference; jobs with
every kind; and what is refused.

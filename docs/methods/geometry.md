---
title: "Geometry"
module: geometry
summary: "Shapes kept exact and turned into polygons only on demand: every shape a region with an exact inside test, signed distance and outward normal, tight bounds, area, perimeter and area inside a cell; polygons within a stated tolerance that keep the area; similarity transforms that keep each shape's kind; and a bounding-volume hierarchy over a layer's shapes, the same bits on any number of threads."
order: 1.5
papers:
  - cite: "I. E. Sutherland, G. W. Hodgman, Commun. ACM 17, 32 (1974) (clipping to a cell one side at a time)"
    doi: 10.1145/360767.360802
  - cite: "C. Lanczos, J. SIAM Numer. Anal. Ser. B 1, 86 (1964) (the gamma function, for the superellipse's area)"
    doi: 10.1137/0701008
  - cite: "S. Adlaj, Notices Amer. Math. Soc. 59, 1094 (2012) (an ellipse's perimeter by the arithmetic-geometric mean)"
    doi: 10.1090/noti879
  - cite: "J. Goldsmith, J. Salmon, IEEE Comput. Graph. Appl. 7, 14 (1987) (bounding-volume hierarchies)"
    doi: 10.1109/MCG.1987.276983
validation:
  - geometry/areas
  - geometry/polygons
  - geometry/fill
  - geometry/transforms
---

Every solver, the studio's views and, later, the layout read one model of what is drawn on a
chip. `photonoxide::geometry` keeps each shape's exact definition: an arc stays an arc, an
ellipse at an angle stays an ellipse. A shape becomes polygons only when something asks, and
then within a tolerance it states.

## Regions

A `Region` answers, for a shape in the plane of the chip:

- `locate(p)`: inside, on the boundary or outside, exactly as far as the definition's
  arithmetic allows (a rectangle's edges by comparison, a circle by $\lvert p - c\rvert^2$
  against $r^2$, a polygon's edges by a zero cross product);
- `distance(p)`: the signed distance to the boundary, negative inside, and the outward unit
  normal at the nearest point;
- `bounds()`: the smallest axis-aligned box;
- `polygons(tolerance)`: outer boundaries counterclockwise, holes clockwise;
- `area()` and `perimeter()`;
- `fill(cell)`: the area of the region inside an axis-aligned cell, what averaging a
  permittivity over a grid's cells needs.

The shapes are the rectangle, circle, ring and polygon jobs already draw, and five more: an
ellipse at any angle, an annular sector (a pie slice with no inner radius), a regular polygon, a
superellipse $\lvert x/a\rvert^n + \lvert y/b\rvert^n \le 1$ ($1 \le n \le 100$) at any angle,
and a rectangle at any angle with rounded corners. The first four answer exactly as they did:
`Shape::contains`, `Shape::area` and `Shape::bounds` give the same bits, and a structure's
`material_at` the same materials.

**Boundaries as pieces.** Most shapes' boundaries are segments and arcs of conics, each arc
$c + R(\psi)\thinspace(r_x \cos t, r_y \sin t)$ over a range of $t$, every loop with the region
on its left. From the pieces, in closed form:

- the area, by Green's theorem $A = \oint x\thinspace dy$; along an arc,
  $x = c_x + p_1 \cos t + p_2 \sin t$ and $y = c_y + q_1 \cos t + q_2 \sin t$, so
  $\int x\thinspace dy = c_x \Delta y + p_1 q_2 \int\cos^2 - p_2 q_1 \int \sin^2 + (p_2 q_2 - p_1 q_1)\int \sin\cos$;
- the distance: the nearest point of each piece (a segment's foot, an arc's radial point or
  its ends, an ellipse's by the Lagrange condition reduced to one monotone equation in one
  variable, solved by bisection to the last bit); off the boundary the normal is along the line
  from the nearest point, on it the piece's own;
- the area in a cell: each loop clipped to the cell's four sides in turn, as Sutherland and
  Hodgman clip polygons. The parts beyond a side are cut out at the side's crossings (closed
  form for segments and arcs) and the loop closed along the side. The clipped loop's winding
  number is the original's inside the half-plane and zero outside, so $\oint x\thinspace dy$
  over it is the area inside, for convex and concave shapes and for holes alike. It is exact to
  round-off for every shape bounded by segments and arcs of circles and ellipses.

**Closed forms.** The ellipse's area $\pi a b$ and its perimeter by Gauss's
arithmetic-geometric mean, $2\pi (a^2 - \sum_n 2^{n-1} c_n^2)/M(a, b)$; the sector's
$\frac12 \beta (r_1^2 - r_0^2)$ and $\beta(r_0 + r_1) + 2(r_1 - r_0)$; the rounded rectangle's
$wh - (4 - \pi) r^2$ and $2(w + h) - 8r + 2\pi r$; the regular polygon's
$\frac n2 R^2 \sin(2\pi/n)$ and $2nR\sin(\pi/n)$; the superellipse's
$4ab\thinspace\Gamma(1 + 1/n)^2/\Gamma(1 + 2/n)$, with Lanczos's approximation of Γ (15 digits).
The superellipse's perimeter has no closed form: each quarter is integrated in two halves
about $\lvert x/a\rvert = \lvert y/b\rvert$, along the coordinate the curve is a smooth function
of there, by Gauss–Legendre quadrature graded toward the axis. Its bounds are exact: the
support function of the unit ball of the $n$-norm is the dual norm,
$h(u) = (\lvert a u_1\rvert^q + \lvert b u_2\rvert^q)^{1/q}$ with $1/n + 1/q = 1$.

The superellipse's distance is found by a search over its parameter (512 samples, then golden
sections to the last bit), and its fill from a polygon within $10^{-7}$ of its larger semi-axis
that keeps its area, so in error by at most that times the curve's length inside the cell.

## Polygons within a tolerance

`Tolerance { geometric, grid }` defaults to 1 nm and a grid of 1 nm, GDSII's usual database
unit. Every point of the polygons is within `geometric` of the true boundary and every point of
the boundary within `geometric` of the polygons: curves are flattened to within
$g - d/\sqrt2$ and the vertices snapped to the grid $d$, which moves each by at most $d/\sqrt2$.
Each `Polygons` records the tolerance asked for and the distance it reached.

An arc of a conic is the image of a unit circle's arc under an affine map, which keeps ratios
of areas. On the unit circle the inscribed polygon of $N$ chords of angle $\Delta$ strays by its
sagitta $1 - \cos(\Delta/2) \approx \Delta^2/8$ and loses area. photonoxide instead keeps the
arc's ends on it and puts the interior vertices at a radius $R$ just above 1, chosen so that
the polygon's fan from the centre has the sector's area $\Phi/2$:
$(N - 2) R^2 \sin\Delta + 2 R \sin\Delta = \Phi$, or $R^2 = \Phi/(N \sin\Delta)$ for a closed
conic. The polygon then keeps the arc's area exactly (to round-off), and strays by
$\max(R - 1, 1 - R\cos(\Delta/2)) \approx \Delta^2/12$ instead of $\Delta^2/8$. $N$ is the
fewest chords that keep that, times the map's largest stretch, within the tolerance; the
distance is second order in the chords, so a hundredth of the tolerance takes ten times the
vertices.

The superellipse's chords are halved until the curve strays from each by at most 3/4 of the
tolerance; each vertex then moves out along the normal by λ times the mean of its two chords'
signed sagittas, λ (about 2/3) the root of the quadratic that makes the polygon's area the
curve's closed form, and the distance is measured again (tightening the chords if it must).

## Transforms

A `Transform` is a similarity, $p \mapsto M p + t$ with $M$ a rotation or a mirror times a
positive scale: translations, rotations about a point, mirrors in a line, uniform scalings and
their compositions. A shape's image is a shape of its own kind: centres move, lengths scale,
angles turn (a mirror reverses a sector's sweep, so it starts at the image of its end). A
rectangle stays a rectangle under quarter turns and mirrors in x or y, which are exact (sine and
cosine of whole quarter turns are taken as 0 and ±1), and becomes a rounded rectangle of radius
zero at its new angle otherwise. Areas scale by the square of the factor and distances by the
factor, to round-off. The factor is kept as the product of the scalings given, so a rotation
keeps lengths to the bit.

## The index

A layer with more than 8 shapes is searched through a bounding-volume hierarchy (Goldsmith and
Salmon 1987) over the shapes' boxes, each widened by $10^{-9}$ of its size so that rounding never
puts a point a shape contains outside its box. It is built in a fixed order (split at the
median of the boxes' centres along the longer side, ties broken by the shapes' order) and
searched in a fixed order, so `Structure::material_at` answers exactly as testing every shape
does, the same bits on any number of threads. `Structure::index` gives the index to callers
that ask which shapes meet a cell (`Index::meeting`), as averaging over cells will.

A photonic crystal of 100 × 100 holes sampled at 2000 × 2000 points: 0.16 s with the index,
40 ns a point, against 28 s testing every shape (one thread of an Intel Core Ultra 7 265K,
release build; `cargo test --release --lib material_at_timing -- --ignored --nocapture`).

## Checks

- Areas and perimeters against Green's theorem and the boundaries' lengths, and the closed
  forms against each other (the superellipse of exponent 2 is the ellipse, of exponent 1 the
  rhombus): `geometry/areas`.
- At points off the boundary and its medial axis, the distance's gradient by central differences
  is the normal, and stepping back along it by the distance lands on the boundary; each
  primitive's distance at chosen points against its closed form.
- Polygons within their tolerance, both ways (their edges near the boundary, the boundary near
  them), oriented, on their grid, keeping the area exactly: `geometry/polygons`.
- Fills over a grid summing to the area, and each cell's against counting a fine lattice of
  points: `geometry/fill`.
- Transforms keeping areas and distances: `geometry/transforms`.
- The index answering as testing every shape does, at 20 000 points and on the boundaries, and
  fills and samples the same bits on 1, 2, 7 and 20 threads.

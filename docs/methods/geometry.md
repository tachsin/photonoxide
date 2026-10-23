---
title: "Geometry"
module: geometry
summary: "Shapes kept exact and turned into polygons only on demand: every shape a region with an exact inside test, signed distance and outward normal, tight bounds, area, perimeter and area inside a cell; polygons within a stated tolerance that keep the area; similarity transforms that keep each shape's kind; paths, a centre line of straights, arcs, clothoids placed by the Fresnel integrals, partial Euler bends, Bézier, sine and parametric curves with a width along it; regions between two curves, polar, inside a closed parametric curve and implicit, each an expression, and masks of fill values; and a bounding-volume hierarchy over a layer's shapes, the same bits on any number of threads."
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
  - cite: "M. Cherchi, S. Ylinen, M. Harjanne, M. Kapulainen, T. Aalto, Opt. Express 21, 17814 (2013) (the Euler bend)"
    doi: 10.1364/OE.21.017814
  - cite: "T. Fujisawa, S. Makino, T. Sato, K. Saitoh, Opt. Express 25, 9150 (2017) (a bend of clothoids and an arc)"
    doi: 10.1364/OE.25.009150
  - cite: "F. Vogelbacher, S. Nevlacsil, M. Sagmeister, J. Kraft, K. Unterrainer, R. Hainberger, Opt. Express 27, 31394 (2019) (the partial Euler bend)"
    doi: 10.1364/OE.27.031394
  - cite: "M. Abramowitz, I. A. Stegun, Handbook of Mathematical Functions (1964), chapter 7 (the Fresnel integrals)"
    doi: ""
  - cite: "W. E. Lorensen, H. E. Cline, ACM SIGGRAPH Comput. Graph. 21, 163 (1987) (marching cubes; in two dimensions, the implicit region's zero set)"
    doi: 10.1145/37402.37422
  - cite: "Y. Zhang, S. Yang, A. E.-J. Lim, G.-Q. Lo, C. Galland, T. Baehr-Jones, M. Hochberg, Opt. Express 21, 1310 (2013) (the Y-junction's outline, drawn between two curves)"
    doi: 10.1364/OE.21.001310
validation:
  - geometry/areas
  - geometry/polygons
  - geometry/fill
  - geometry/transforms
  - geometry/paths/fresnel
  - geometry/paths/euler-ends
  - geometry/paths/euler-curvature
  - geometry/paths/perimeter
  - geometry/paths/distance
  - geometry/paths/polygons
  - geometry/paths/fill
  - geometry/paths/convergence
  - geometry/paths/cherchi-l-bend
  - geometry/paths/cherchi-u-bend
  - geometry/paths/vogelbacher-singular
  - geometry/paths/vogelbacher-270
  - geometry/paths/vogelbacher-fujisawa
  - geometry/paths/fujisawa-ratios
  - geometry/curves/circle
  - geometry/curves/closed-forms
  - geometry/curves/taper-as-path
  - geometry/curves/distance
  - geometry/curves/polygons
  - geometry/curves/fill
  - geometry/curves/convergence
  - geometry/curves/implicit-estimate
  - geometry/curves/mask
  - geometry/curves/zhang-y-junction
---

Every solver, the studio's views and, later, the layout read one model of what is drawn on a
chip. `photonoxide::geometry` keeps each shape's exact definition: an arc stays an arc, an
ellipse at an angle stays an ellipse. A shape becomes polygons only when something asks, and
then within a tolerance it states. In space, a `Body` answers the same questions, with the volume
it fills of a cell (docs/methods/bodies.md).

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
  from the nearest point, on it the piece's own, and within the coordinates' rounding of it (16
  units in the last place) the piece's own too, where the line from the point is rounding and
  points anywhere (a grid's point on an edge, its coordinate summed from steps);
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

## Paths

Waveguides, bends, tapers and S-bends are paths: a centre line $c(s)$ of segments, with a width
$w(s)$ along it, and the region swept by the line's normals out to half the width on each side,
$\lbrace c(s) + v\thinspace n(s) : \lvert v\rvert \le w(s)/2 \rbrace$, its ends cut flat.
`Path::builder(start, heading, width)` adds segments one after another. Each starts where the
one before ends and along its direction, so the tangent is continuous at every joint, and the
curvature too where the two kinds allow (an Euler bend or a sine S-bend after a straight).

- **Straight** and **arc** (a radius and a signed angle, positive to the left).
- **Clothoid:** curvature linear in arc length, $\kappa(s) = \kappa_0 + (\kappa_1 - \kappa_0)\thinspace s/L$.
  When one end is straight its points are the Fresnel integrals,
  $\int_0^\tau e^{i a u^2/2} du = \sqrt{\pi/a}\thinspace(C + iS)(\tau\sqrt{a/\pi})$; one that
  ends straight is placed from its end, $c(s) = e^{i\theta_E}(F(L) - F(L - s))$. A clothoid with
  neither end straight is integrated by Gauss–Legendre on panels over which it turns by at most
  half a radian, to round-off.
- **Partial Euler bend** (Vogelbacher et al. 2019): a clothoid from the straight over the
  fraction $p$ of the turn's first half, an arc of the radius it reaches, $R_\text{min}$, and the
  clothoid back. It is built as their Eqs. (5)–(11) build it: the half bend unscaled (a clothoid
  of parameter $A = 1$ to $p\alpha/2$, then the arc to $\alpha/2$), then scaled by
  $\eta = R_\text{eff}/(y + x/\tan(\alpha/2))$ from that half's end $(x, y)$, so that its ends
  and their directions are those of the circular bend of the effective radius $R_\text{eff}$.
  $p = 1$ is Cherchi et al. 2013's Euler bend, $p = 0$ the circular bend. A bend whose
  denominator isn't positive doesn't exist and is refused (for $p = 1$, beyond about 263°).
  `partial_euler` gives the construction: $R_\text{min}$, the clothoids' length and parameter
  $A$ ($R L = A^2$, Fujisawa et al. 2017's Eq. (3)), the arc's angle and the whole length.
- **Bézier curve** (control points after the start, turned to leave along the path), **sine
  S-bend** $y = h(x/L - \sin(2\pi x/L)/2\pi)$ (straight and of zero curvature at both ends), and
  a **curve** $(x(t), y(t))$ of two [expressions](expressions.md) in $t$, placed so that it
  leaves along the path. Their derivatives are exact (de Casteljau on the hodographs, closed
  form, forward-mode derivatives of the expressions); a curve's second derivative, for its
  curvature, is a central difference of the first. Their lengths are Gauss–Legendre integrals of
  their speed between knots, to round-off.

The width is constant, tapers linearly along a segment (`taper`), or is an expression $w(s)$ of
the arc length (`Profile`). The ends are ports: `start()` and `end()` give the position, the
direction out of the path and the width. `length()`, `point(s)`, `direction(s)`,
`curvature(s)` and `width(s)` read the centre line. `shape()` gives the simplest exact shape of
the same region: a single arc of constant width is a `Sector`, a single straight a rectangle,
anything else the path itself. The strip bend device draws its arc this way, so its structure
is the sector it always drew, to the bit.

**Knots.** Each part of the centre line is cut at knots close enough that between two the line
turns by at most 0.1 rad and runs at most a quarter of the gap between its inner edge and its
centre of curvature, $(1/\kappa_\text{max} - w_\text{max}/2)/4$, and, under a width profile, half
the narrowest width. A path is refused where its width reaches twice its radius of curvature
(the inner edge would fold over itself), and where its outline crosses or touches itself
(checked on its polygon within a 32nd of the narrowest width).

**Inside.** $p$ is in the region when $p = c(s) + v\thinspace n(s)$ with $\lvert v\rvert \le w(s)/2$
for some $s$: its foot on the centre line, a root of $g = (p - c)\cdot t$. Since
$g' = -(1 - v\kappa)\thinspace ds/du$ is negative wherever $\lvert v\rvert < 1/\lvert\kappa\rvert$,
and the knots are closer than a quarter of the gap, an interval near the region holds at most
one foot, which its bracket shows; it is found by the Illinois method to the last bit, and only
the intervals whose boxes, widened by the half width, hold $p$ are tried (a bounding-volume
hierarchy). The test is exact up to that root, whose error moves $v$ only at second order.

**Distance.** The outline is the right edge forward, the end's cap, the left edge back and the
start's cap, each edge $E = c \pm \frac w2 n$, with
$E' = s'\thinspace[(1 \mp \frac w2 \kappa)\thinspace t \pm \frac12 w'\thinspace n]$. Along a
straight or an arc of constant width the edges are a segment and arcs of circles, kept exact; the
other edges are curve pieces, one between each two knots. The nearest point is each exact
piece's in closed form, or a curve piece's at its ends or where $(p - E)\cdot E'$ falls through
zero; the pieces are tried nearest box first, until a box is farther than the best so far.

**Area and perimeter.** The map $(s, v) \mapsto c(s) + v\thinspace n(s)$ has Jacobian
$1 - v\kappa$, whose integral over $\lvert v\rvert \le w/2$ is $w$: the area is $\int w\thinspace ds$,
exactly, whatever the centre line (Gauss–Legendre between knots under a profile). For a constant
width the edges are $L \mp \frac w2\Delta\theta$ long and the perimeter $2L + 2w$, which the
edges' lengths, integrated between knots, reproduce to $10^{-15}$.

**Polygons and fills.** Each curve piece's chords are halved until the curve strays from each by
at most 3/4 of the budget; each vertex inside a run of curve chords then moves out along the
normal by λ times the mean of its two chords' signed sagittas, λ (about 2/3) the root of the
quadratic that gives the polygon the area $\int w\thinspace ds$ exactly, and the distance is
measured again at nine points of each chord (tightening if it must). The arcs are flattened as
every shape's are. The distance is second order in the chords: on a 90° Euler bend, from 10 to
0.01 nm, the distance reached against the number of vertices has slope −2.03. A fill clips the
outline with its arcs exact and its curves within $10^{-7}$ of the widest width, keeping the
area, so the fills of a grid sum to it to $10^{-14}$.

**Jobs.** `[[task.path]]` draws a path in `"structure"`, `"fdfd"` and `"fdtd"` jobs, its values
expressions with units, checked by `job::check` and drawn by `job::preview`:

```yaml
task:
  path:
    - layer: Si
      start: ["-1 um", 0]  # or start_um; (0, 0) by default
      heading: "0 deg"     # or heading_rad; 0 by default
      width: w             # or width_um
      segments:
        - { straight: "1 um" }  # or straight_um
        - { euler: "90 deg", radius: "5 um", p: 0.2 }  # p = 1 by default
        - { arc: "45 deg", radius: "3 um" }
        - { straight: "2 um", width: "w + 100 nm" }  # tapers to its end's width
        - { sine: "4 um", offset: "-1 um" }
```

An angle needs its unit (`"90"` is refused: `"90 deg"` or a number in an `_rad` key).

**What it reproduces.** The geometry of the three papers' bends, from their own equations:

| | the papers | photonoxide |
|---|---|---|
| Euler L-bend (90°): $R_\text{eff}/R_\text{min}$, length$/R_\text{min}$ | 1.87, $\pi$ (Cherchi et al. 2013, Fig. 3(b)) | 1.870096, $\pi$ |
| Euler U-bend (180°): $R_\text{eff}/R_\text{min}$, length$/R_\text{min}$ | 1.38, $2\pi$ (Fig. 3(c)) | 1.376832, $2\pi$ |
| the turn beyond which no Euler bend of $p = 1$ exists | about 263.3° (Vogelbacher et al. 2019, §2.2) | 263.267° |
| the largest $p$ of a 270° bend | 0.585 (§2.2) | 0.58546 |
| the $p$ of a 90° bend 68 % clothoid | 0.52 (§3.2) | 0.5152 |
| clothoid fraction of 90° bends of $R_\text{eff}$ = 4 µm, A = 1.3, 2.4, 2.68 µm | 14, 58, 98 % (Fujisawa et al. 2017, §3) | 14.05, 58.23, 97.83 % |

Vogelbacher et al.'s text gives the pure Euler 90° bend's largest curvature as 82 % above the
circular bend's; their construction, built here, gives 87 %, Cherchi et al.'s 1.87. The papers'
bend losses are not reproduced here: they are 3D eigenmode-expansion, FDTD and FEM results on
grids the papers state, and a 2D estimate would not be their device's performance. That waits for
the solvers to read fill fractions (phase 10 of #227).

## Shapes defined by functions

Four kinds of shape are given by [expressions](expressions.md) of the coordinates and of named
parameters:

| kind | defined by | inside test |
|---|---|---|
| `Between` | $y_\text{low}(x) \le y \le y_\text{high}(x)$ for $x_0 \le x \le x_1$ | the two curves at the point's $x$ |
| `Polar` | $r \le \rho(\theta)$ about a centre, $\rho$ an expression of `t` $= \theta$ | $\rho$ at the point's angle |
| `ClosedCurve` | the inside of $(x(t), y(t))$, $0 \le t \le 1$ | crossings of a ray with the curve |
| `Implicit` | $f(x, y) \le 0$ in a window | the sign of $f$ |

Each keeps its definition in a frame of its own and a similarity transform to the plane, so a
transformed one is one of its own kind: its inside test maps the point back and asks the
definition, exactly as far as the arithmetic of the expressions goes.

**What is refused.** A region between two curves whose lower curve meets or crosses the upper
between its ends (they may meet at the ends: a pointed taper), checked at 2049 points and at
every knot of its boundary; a polar $\rho$ that isn't positive, or isn't periodic
($\rho(2\pi)$ more than $10^{-9}$ of the largest $\rho$ from $\rho(0)$); a closed curve that
doesn't close, stops (a zero derivative) or crosses or touches itself (checked on its polygon
within $10^{-5}$ of its size; it may run either way round); an implicit $f$ that isn't finite on
its window's sampling grid, isn't positive on the window's edge (the region must lie inside its
window), has no zero set the grid finds, or whose gradient vanishes on its zero set (a singular
point, as the lemniscate's). Any expression reading a coordinate it isn't given (a curve of `y`,
a radius of `x`) or of the wrong dimension is refused, with the expression named.

**The boundary.** Each kind traces its boundary as loops of curves, the region on their left:

- between two curves: the lower curve from $x_0$ to $x_1$, the side at $x_1$, the upper curve
  back and the side at $x_0$, corners at the four joints (a side of zero length dropped);
- polar: $\rho(\theta)(\cos\theta, \sin\theta)$ for $\theta$ from 0 to $2\pi$;
- a closed curve: the curve itself, walked backwards if it runs clockwise;
- implicit: $f$ is sampled on a grid of 256 cells along the window's longer side (or as many as
  asked) and its zero set traced by marching squares (Lorensen and Cline 1987, in two
  dimensions). Each cell's crossings are found on its edges by bracketed root finding to the
  last bit and joined with the region on their left; a saddle cell (its diagonal corners alone
  inside) is decided by $f$ at its centre. Where the gradient turns by more than 0.1 rad between
  two crossings, the chord is halved and its middle moved onto the zero set along the chord's
  normal. Between two vertices the boundary is the curve itself, a graph over their chord $e$:
  its point at $\tau$ the root of $f(a + \tau e + \sigma n) = 0$ along the chord's normal $n$,
  its derivative $e - (\nabla f\cdot e/\nabla f\cdot n)\thinspace n$, $\nabla f$ by the
  expressions' dual numbers. Holes and separate parts are loops of their own. A part (or hole)
  narrower than a sampling cell may be missed; a finer grid finds it.

Every curve is cut into intervals over which it turns by at most 0.1 rad and is monotone in $x$
and $y$ (cut where $x'$ or $y'$ changes sign, found to the last bit). So:

- an interval's box is its ends', and the bounds, the extremes of every interval's ends, are tight;
- a horizontal ray meets an interval at most once: a closed curve's inside test counts the
  crossings of the ray from the point towards $+x$ with the exact curve, each by root finding,
  through a bounding-volume hierarchy over the intervals;
- the nearest point of an interval is one of its ends or a root of $(p - c)\cdot c'$ where it
  falls through zero (or is zero at a sample), the intervals tried nearest box first;
- the area $\oint x\thinspace dy$ and the perimeter $\oint\lvert c'\rvert$ are 20-point
  Gauss–Legendre integrals over each interval, halved where one rule and the two on its halves
  disagree (a kink of a `piecewise`, `abs` or `min`): to round-off.

Polygons and fills are the paths' and the superellipse's: chords halved until the curve strays
from each by at most 3/4 of the budget, each vertex not at a corner moved out along the normal by
λ times the mean of its chords' signed sagittas, λ the root of the quadratic that keeps each
loop's area exactly; a fill clips the polygons within $10^{-7}$ of the larger side, so the fills of
a grid sum to the area to round-off.

**Distance of an implicit region.** $f/\lvert\nabla f\rvert$ (`Implicit::estimate`) is exact on
the boundary and in error by $O(\kappa d^2)$ a distance $d$ off it: for $f = r^2 - R^2$ it is
$d(2R + d)/(2(R + d))$, in error by $d^2/(2(R + d))$. `distance` corrects it to the exact distance:
the foot of the point is found by root finding on the curve itself, from the nearest pieces of the
traced boundary, so its error is the root finding's, $10^{-12}$ µm and below.

## Masks

A `Mask` is a grid of fill values $\rho$ from 0 to 1 (from `Mask::new`, `Mask::from_rows`, or
`Mask::parse` of a text file of rows, the top row first, as the picture reads; comma, space or tab
separated). It is two things, each answering what it is about:

- a density, constant over each pixel: `fill(cell)` is $\int\rho$ over the cell, exactly (each
  pixel's value times the area it shares with the cell; a rotated pixel clipped to it), and
  `area()` is $\int\rho$, so grey pixels count as the fractions they are and a grid's fills sum to
  the area;
- the design binarised at $\rho \ge \frac12$, the shape as it would be made: `contains`/`locate`
  (and so point sampling), `distance`, `polygons` and `perimeter` are the union of the pixels with
  $\rho \ge \frac12$, exactly. Its outline is the pixels' edges; pixels meeting only at a corner
  are separate polygons; holes are found. `binarised_area()` is its area.

For a mask of zeros and ones they are the same region. `bounds()` is the box of the pixels with
$\rho > 0$, which holds both. A mask with no pixel at ½ or more draws nothing and is refused.

## In jobs

`[[task.shape]]` draws any of them in `"structure"`, `"fdfd"` and `"fdtd"` jobs, with an `etch`
as every shape takes. Places and sizes are expressions with units (or `_um` numbers); the
definitions are expressions of the coordinates they are given and of `[task.parameters]`:

```yaml
task:
  shape:
    - type: between         # y_low(x) <= y <= y_high(x)
      layer: Si
      x: [0, "2 um"]
      lower: "-(w/2 + 0.1*x)"
      upper: "w/2 + 0.1*x"

    - type: polar           # r <= rho(t), t the polar angle
      layer: Si
      center: ["6 um", 0]
      radius: "r0*(1 + 0.43*cos(t))"

    - type: parametric      # the inside of (x(t), y(t)), t from 0 to 1
      layer: Si
      curve: ["1.3 um*cos(2*pi*t)", "0.6 um*sin(2*pi*t)"]

    - type: implicit        # f(x, y) <= 0 in the window x by y
      layer: Si
      inside: "(x/2um)^4 + (y/1um)^4 - 1"
      x: ["-3 um", "3 um"]
      y: ["-2 um", "2 um"]
      cells: 256            # optional

    - type: mask
      layer: Si
      corner: ["-1 um", "2 um"]  # the lower left corner
      pixel: ["100 nm", "100 nm"]
      rows: [[0, 1, 1], [1, 0.5, 0]]  # the top row first
```

`job::check` refuses what the library refuses, naming the table, its type and the field
(`[[task.shape]] #2 (polar): its radius ... must be positive`), and a field another type takes.

**What it reproduces.** Zhang et al.'s Y-junction (2013): a 2 µm taper from the 0.5 µm input
guide to two 0.5 µm arms 0.2 µm apart, its width defined by spline interpolation of 13 widths at
equal spacing (their Table 1: 0.5, 0.5, 0.6, 0.7, 0.9, 1.26, 1.4, 1.4, 1.4, 1.4, 1.31, 1.2,
1.2 µm). Drawn between $y = \mp w(x)/2$, $w$ a cubic spline through the widths written as one
`piecewise` expression (not-a-knot ends, MATLAB's `spline`; the paper doesn't name its ends, and
the widths at the 13 points don't depend on them), its width measured through its inside test at
the 13 points is the table's within 2.2e-15 µm, its ends are 0.5 and 1.2 µm, and its area is
$\int w\thinspace dx$ (Simpson's rule, exact on each cubic). This is the outline P6's Y-branch
device draws (`device::StripYBranch`, its outline now written with the expression language's
`spline`, the same curve to 1e-13); the paper's losses are 3D FDTD and measurement, not
reproduced here.

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
- Paths: the Fresnel integrals against A&S's table and quadrature (`geometry/paths/fresnel`);
  clothoids by the Fresnel integrals against quadrature of $e^{i\theta}$; Euler bends' ends on
  the circular bend's and their curvature linear in arc length
  (`geometry/paths/euler-ends`, `geometry/paths/euler-curvature`); an arc path answering as its
  sector does at 400 points; a parametric arc as the arc; perimeters $2L + 2w$
  (`geometry/paths/perimeter`); distances against brute force, their sign against the inside
  test, and stepping back along the normal landing on the outline (`geometry/paths/distance`);
  polygons within tolerance keeping the area, their convergence, and fills
  (`geometry/paths/polygons`, `geometry/paths/convergence`, `geometry/paths/fill`); transforms;
  the same bits on several threads; and the papers' bends (the table above).
- Shapes defined by functions: a circle written as a polar region, a closed curve and an
  implicit region against the primitive, their areas, perimeters, distances and normals at 300
  points to 1.3e-13 (`geometry/curves/circle`); areas against closed forms, a limaçon
  $\pi b^2 + \pi a^2/2$, a flower $\pi r_0^2(1 + \varepsilon^2/2)$, ellipses run both ways and
  implicit $\pi ab$, the implicit superellipse $4ab\thinspace\Gamma(5/4)^2/\Gamma(3/2)$, an
  annulus and two disks, a linear and a parabolic taper, to 3.6e-15
  (`geometry/curves/closed-forms`); a linear taper between two curves answering as the path taper
  does (`geometry/curves/taper-as-path`); distances against brute force within 4.7e-4 nm
  (`geometry/curves/distance`); polygons within 0.75 nm at the default 1 nm and keeping their
  area (`geometry/curves/polygons`); fills summing to the area to 7.6e-15, and the three circles'
  fills cell by cell within 2.8e-9 of the primitive's exact one (`geometry/curves/fill`); the
  polygons' distance second order in their vertices, slopes 1.94 and above
  (`geometry/curves/convergence`); the implicit estimate's error second order in $d$, 1.99
  (`geometry/curves/implicit-estimate`); masks' fills, areas and binarised polygons to round-off,
  drawn and rotated (`geometry/curves/mask`); transforms keeping each kind, areas and distances;
  the same bits on several threads; refusals; and Zhang et al.'s Y-junction
  (`geometry/curves/zhang-y-junction`).
- Booleans, offsets and corner rounding: their own write-up, docs/methods/booleans.md, with
  their checks (`geometry/booleans/*`).
- Arrays, lattices and references: their own write-up, docs/methods/arrays.md, with their
  checks (`geometry/arrays/*`) and Akahane et al.'s L3 cavity drawn.

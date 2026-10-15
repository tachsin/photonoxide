---
title: "Averaging from the fill"
module: averaging
summary: "Each field component's permittivity from the share of its cell every material fills and the interface's normal, instead of from points: Farjadpour et al.'s tensor in 2D FDFD with its off-diagonal coupling, its diagonal in 3D, the mean for pictures; second order where points are first, and geometric derivatives by the adjoint."
order: 15.5
papers:
  - cite: "A. Farjadpour, D. Roundy, A. Rodriguez, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, G. W. Burr, Opt. Lett. 31, 2972 (2006) (the tensor, Eq. 1; Fig. 1 reproduced in kind)"
    doi: 10.1364/OL.31.002972
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966) (where each component sits)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "G. R. Werner, C. A. Bauer, J. R. Cary, J. Comput. Phys. 255, 436 (2013) (symmetric placements first order at sharp interfaces)"
    doi: 10.1016/j.jcp.2013.08.009
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004) (the adjoint derivative)"
    doi: 10.1364/OL.29.002288
validation:
  - averaging/film-integrals
  - averaging/disk-area
  - averaging/aligned-bits
  - fdfd/fill-slab-te-mean
  - fdfd/fill-slab-te-point
  - fdfd/fill-slab-tm-diagonal
  - fdfd/fill-slab-tm-mean
  - fdfd3d/fill-film-te-diagonal
  - fdfd3d/fill-film-tm-diagonal
  - fdfd3d/fill-film-tm-point
  - fdfd/fill-crystal-te-tensor
  - fdfd/fill-crystal-te-others
  - fdfd/fill-crystal-tm-mean
  - fdfd/fill-derivative-ez
  - fdfd/fill-derivative-hz
---

A grid's cell that an interface crosses holds two materials. Read at a point, the cell is one of
them, so a shape moves in steps of the grid: staircasing. The solver's error then falls only as
the step does, first order, and erratically, and a derivative with respect to the geometry is
zero almost everywhere. `photonoxide::averaging` reads the cell instead from the share of it each
material fills, `geometry::Region::fill`, which is exact for polygons and to a stated tolerance
for curves ([the kernel](geometry.md)).

## The averages

`Averaging` says how a cell is read. It is an option: every solver's default is what it has
always done, to the bit.

| `Averaging` | the cell's permittivity | in jobs |
|---|---|---|
| `Samples` (the default) | 8 samples along each axis of the component's cell, harmonic along the component and arithmetic across it | `"samples"` |
| `Point` | the material at the component's own point | `"point"` |
| `Mean` | $\langle\varepsilon\rangle = \sum_m f_m \varepsilon_m$ for every component | `"mean"` |
| `Diagonal` | the diagonal of Farjadpour et al.'s tensor, below | `"diagonal"` |
| `Tensor` | the whole tensor, its off-diagonal coupling too (2D, $H$ along $z$) | `"tensor"` |

Farjadpour et al. ask what a smoothing does to the problem. Near a flat interface a change
$\Delta\varepsilon$ moves a frequency or a scattered power by
$\Delta\varepsilon\thinspace\lvert E_\parallel\rvert^2 - \Delta(\varepsilon^{-1})\thinspace\lvert D_\perp\rvert^2$,
with $E_\parallel$ and $D_\perp$ the fields that are continuous across it. For that to vanish to
first order, $\varepsilon_\parallel$ and $\varepsilon_\perp^{-1}$ must both average to the truth:
the mean for the components along the interface and the harmonic mean for the one across it,

$$
\tilde\varepsilon^{-1} = P\thinspace\langle\varepsilon^{-1}\rangle + (1 - P)\thinspace\langle\varepsilon\rangle^{-1},
\qquad P = n\thinspace n^T
$$

(their Eq. 1), $n$ the interface's unit normal. photonoxide takes $n$ from the signed distance
at the cell's centre (`Region::distance`): the normal at the nearest point of the boundary. The
mean alone is right only for $E$ along every interface; the diagonal of the tensor (their
"VP-EP") only for interfaces along the grid's axes; on a slanted interface only the whole tensor
keeps the scheme second order. Their Fig. 1 shows it, and so do the cases below.

A cell is answered from the shapes whose boxes meet it ([the spatial index](geometry.md)). One
shape crossing it gives its fill exactly. Where several do, the fill of their union (one
material) or each one's visible part, what the shapes drawn after it leave, is taken from a
boolean ([booleans](booleans.md): its polygons within `FILL_TOLERANCE` of its size), made once
per set of shapes. A share within $10^{-12}$ of 0 or 1 is taken as 0 or 1, so a cell inside a
curved shape is its material to the bit. The values are computed row by row side by side and
collected in order: the same bits on any number of threads.

## FDFD in 2D

`Solver2d::from_plane(grid, polarization, wavelength, &plane, boundaries, averaging)` takes a
`Plane`: a background permittivity and shapes drawn on it, each with its own, later ones on top.
Each component is averaged over the cell of the grid's size centred on it, on Yee's staggered
grid: $E_z$ at the cells' centres, $E_y$ on the $x$-faces, $E_x$ on the $y$-faces.

- **$E$ along $z$.** Every interface of a plane is parallel to $E_z$, so the mean is the right
  average, and the diagonal and the tensor give it.
- **$H$ along $z$.** $1/\varepsilon_y = (\tilde\varepsilon^{-1})_{yy}$ on the $x$-faces and
  $1/\varepsilon_x = (\tilde\varepsilon^{-1})_{xx}$ on the $y$-faces. With the tensor, the
  operator gains $-\partial_x(\eta_{xy}\partial_y H) - \partial_y(\eta_{xy}\partial_x H)$,
  $\eta = \tilde\varepsilon^{-1}$. Those terms need $D_y$ at each $E_x$ and $D_x$ at each $E_y$:
  as Farjadpour et al. take them, the mean of the four nearest, times each $E$'s own
  off-diagonal entry. Written with $G_x$, $G_y$ (the differences from the cells to the faces,
  Bloch phases included) and $S$ (the four-point mean from the $y$-faces to the $x$-faces),
  $$
  T = G_x^H \eta^{(y)} S\thinspace G_y + G_y^H \eta^{(x)} S^H G_x,
  $$
  $\eta^{(y)}$ the entry at the $E_y$ points and $\eta^{(x)}$ at the $E_x$ points. Where
  $\eta_{xy}$ varies, $T$ is not symmetric, so the problem is factorized by LU instead of
  through its complex symmetric similarity: about twice the work and memory, FDFD's only price.
  (In time the same placement grows from round-off, [FDTD](fdtd.md); and the symmetric
  placements that are stable there are first order in the end at a sharp interface, as Werner,
  Bauer and Cary find. The symmetric one here, $\tfrac12(T + T^g)$, each placement half with its
  transpose, does that on the crystal below: its band at 80, 160 and 320 cells per period moves
  by 4.8e-6 and then 3.2e-6, where this one's moves by 1.6e-5 and then 2.2e-6.) Inside a PML the
  terms are divided by the cell's stretches, as the rest of the row is; the PML's stretching of
  them is not modelled, which matters only where a slanted interface runs into a PML.

`Averaging::Samples` from a plane is `Solver2d::new` with the plane's permittivity at each point,
to the bit. `reuse_plane` reuses the analysis of the matrix's sparsity, except with the tensor,
whose sparsity follows the geometry. The ports' modes and the fluxes read the diagonal; they are
exact where their columns cross no slanted interface, as ports do.

**Derivatives with respect to the geometry.** With the fill, the matrix is a smooth function of
a shape's parameters (between the values at which a boundary passes a cell's corner, where its
second derivative jumps). `Solver2d::mode_power_derivative` gives a port mode's power and its
derivative with respect to a parameter $p$ by the adjoint method, $\mathrm{d}F/\mathrm{d}p =
-2\operatorname{Re}(\lambda^T (\mathrm{d}A/\mathrm{d}p)\thinspace u)$, $A^T\lambda = \overline{a}\thinspace c$
(Veronis et al., [adjoint gradients](fdfd-adjoint.md)), with $\mathrm{d}A/\mathrm{d}p$ the
difference of the matrices assembled from the geometry at $p \pm \delta$: two assemblies and one
back-substitution, no factorization. With samples or points the same difference is zero or a
jump.

## FDFD in 3D

`Solver3d::from_structure(grid, wavelength, &structure, boundaries, averaging)` reads a
[stack](stacks.md) and its shapes. Each component's box is cut by the stack's interfaces into
slices, each in one layer, the substrate or the cladding. In a plain layer a slice's shares are
its shapes' fill of the box's face, the same at every height, with their normal in the plane; a
layer with partial etches and vertical walls is cut again at its highest floor, below which it
is its material all through. Then:

- **the mean** is $\sum_s w_s \langle\varepsilon\rangle_s$, $w_s$ each slice's share of the
  height;
- **the diagonal** takes each slice's in-plane average (the walls are vertical, so $E_z$ sees
  their mean and $E_x$, $E_y$ the diagonal of the in-plane tensor), and across the slices,
  layers normal to $z$, the harmonic mean for $E_z$ and the mean for $E_x$ and $E_y$.

That is the tensor exactly where every interface is a layer's or a wall along a grid axis. A wall
slanted in the plane gets the tensor's diagonal only: the 3D operator has no off-diagonal
coupling, and `Averaging::Tensor` is refused there. A slice under sloped walls, whose shares
change with the height, is sampled (8 × 8 × 8), as `Samples` samples a whole cell.

## Rasters

`Structure::top_view_averaged` and `side_view_averaged` take an `Averaging`: `Point` is the
pictures as always, `Samples` the mean of 8 × 8 samples over each cell, `Mean` the fill's mean,
which shows what a solver averaging from the fill sees. A side view's cut is a line, so its
cells' fill is taken over a strip $10^{-6}$ of the step either side of it (`CUT_STRIP`). The
diagonal and the tensor are a field component's averages and are refused.

## Jobs

An `"fdfd"` job takes `averaging = "mean"` (or `"point"`, `"diagonal"`, `"tensor"`; `"samples"`
is the default and changes nothing). The plane is then two columns of the
[effective index method](eim.md): the layer's index outside every shape, and inside one. So the
other layers must have nothing drawn on them, and the walls must be vertical: a partial etch is
two columns (outside the shapes the layer stands at its highest floor), a sloped wall is a range
of them and is refused (`job::check` says so). The run records the averaging and the fill's
tolerance beside the grid.

## The mode solver's cross-section

The full-vector mode solver ([Fallahkhair et al.'s scheme](vector.md)) puts every vertical wall,
layer interface and etch floor on its nodes (the modes jobs and the devices' sections place them
there), so those cells hold one material and averaging changes nothing. A sloped wall's cells
already take the mean by share ([stacks](stacks.md)), which phase 8 measured to do best there.
What limits a slanted interface in that scheme is not the cells' values but its assumption that
interfaces lie on grid lines: a slanted wall is a run of cells, each corner of the run a small
dielectric corner where the scheme is first order, about $1/h$ of them, so the error doesn't
shrink with the grid whatever the cells hold. Fill fractions don't fix that, and the mode solver
is left as it is.

## Validation

Every number with its grid; the slabs' and crystals' are 2D or a film's, never a device's.

- **Exact integrals** (`averaging/film-integrals`): a film ($\varepsilon$ 12, 223.7 nm) in 2.085,
  its faces off a 50 nm grid's lines: a column's $\sum\langle\varepsilon\rangle h$ and
  $\sum \varepsilon_x h$ are $\int\varepsilon\thinspace dy$, and $\sum h/\varepsilon_y$ is
  $\int dy/\varepsilon$, to rounding.
- **A disk's area** (`averaging/disk-area`): $\sum(\langle\varepsilon\rangle - 1)A$ over 40 nm
  cells is $(12 - 1)\pi r^2$ for $r$ = 0.437 µm, to $10^{-12}$.
- **Aligned interfaces keep the bits** (`averaging/aligned-bits`): a block whose edges are the
  grid's lines gives every component the permittivity at its centre, to the bit.
- **A slab off the grid** (2D, `fdfd/fill-slab-*`): 220 nm of 3.476 in 1.444 at 1.55 µm, its
  faces 0.3 and 0.7 of a cell off the lines, $h = t/(N + 0.4)$ for $N$ = 8 to 256 cells across
  it (26 to 0.86 nm), 2 µm of cladding to walls, against the exact slab. The order from 128 to
  256 cells:

  | | point | samples | mean | diagonal |
  |---|---|---|---|---|
  | TE ($E$ along $z$) | 1.01 | 1.05 | 2.00 | 2.00 |
  | TM ($H$ along $z$) | 0.98 | 1.00 | 1.02 | 1.88 |

  The TE errors with the mean fall from 8.1e-3 to 9.0e-6; with points from 6.7e-2 to 1.7e-3.
  The TM errors with the diagonal change sign near 16 cells and fall from 1.9e-5 at 32 cells to
  6.4e-7 at 256; the mean, wrong for $E_y$ across the film, stays first order.
- **The same film in 3D** (`fdfd3d/fill-film-*`): a layer of the catalogue's silicon in silica,
  the full-vector port modes, the same cells: TE 2.00 with the diagonal (and the mean), TM 1.88
  with the diagonal, 1.02 with the mean, 0.98 with points.
- **Farjadpour et al.'s Fig. 1, in kind** (`fdfd/fill-crystal-*`, published): a square lattice
  of elliptical air holes in $\varepsilon$ = 12 (the paper gives only an inset; ours: semi-axes
  0.32$a$ and 0.2$a$ turned 30°), the lowest band at $k = (0.3, 0.2)\thinspace 2\pi/a$, not along
  the grid, as the smallest eigenvalue of the grid's operator (Bloch-periodic, shift-and-invert
  at 0). The orders fitted to the relative errors at 10 to 56 cells per period:

  | | point | samples | mean | diagonal | tensor |
  |---|---|---|---|---|---|
  | TE ($H$ along $z$) | 1.05 | 0.84 | 1.33 | 0.93 | 1.81 |
  | TM ($E$ along $z$) | 1.28 | 1.78 | 2.02 | 2.02 | 2.02 |

  As in the paper: in TE only the tensor converges at second order, with scatter, and its
  errors are the smallest (3.0e-3 at 10 cells to 1.5e-4 at 80, against 1.3e-2 to 1.0e-3 for
  the mean and 1.1e-2 to 1.7e-3 for the diagonal); the diagonal is no better than points. In TM
  every smoothing is the mean and is second order. The paper finds TM without smoothing
  quadratic too; ours is erratic (1.28). The references are the tensor's TE band (0.1269650
  $\omega a/2\pi c$) and the mean's TM band (0.1141450), each at 160 and 320 cells per period
  extrapolated at second order: 5.8e-6 and 1.1e-6 from their values at 320 cells, well below the
  errors fitted.
- **Derivatives** (`fdfd/fill-derivative-*`): a disk of $\varepsilon$ 6 and radius 0.13 µm
  beside a 220 nm silicon slab, 20 nm grid, the power into the far port's mode: the adjoint's
  derivative with respect to the radius against central differences of full solves over the
  same $\pm$0.1 nm, within 2.2e-6 ($E$ along $z$) and 8.5e-6 ($H$ along $z$, the mean; the
  tensor 1.1e-6). Over $\pm$1 and $\pm$2 nm, fourth-order differences of the mean and the
  diagonal differ from the adjoint by 4e-4 to 1.6e-2: the radius passes cells' corners there.

## What was left out

- **The bend losses of Cherchi et al. 2013, Fujisawa et al. 2017 and Vogelbacher et al. 2019**,
  which the paths' phase left for this one. Their losses come from 3D: eigenmode expansion,
  FDTD and FEM, and measurement. A bend in a 2D effective-index plane radiates into the slab's
  lateral continuum with an index contrast the method makes up, and has no substrate, no
  vertical mode mismatch at the joints and no polarization coupling; its loss would be the 2D
  model's, not the device's, and even the trend against a bend's parameter (Vogelbacher's $p$)
  would be the model's trend. The silicon-nitride bends are also tens of micrometres across, a
  2D problem of millions of cells at the grids the losses need. Rather than a 2D number beside a
  3D one, none: they wait for 3D FDFD or FDTD on these geometries.
- The off-diagonal coupling in 3D, and corner-aware smoothing (Farjadpour et al.'s Fig. 3: a
  sharp corner limits every scheme to about order 1.4).
- Sloped walls from the fill: in 3D they are sampled, and a 2D job refuses them.

## Use

```rust
use num_complex::Complex64 as c64;
use photonoxide::averaging::{Averaging, Plane};
use photonoxide::fdfd::{Boundaries, Grid, Polarization, Solver2d};
use photonoxide::geometry::{Point, Shape};
use photonoxide::units::{Length, Wavelength};

let mut plane = Plane::new(c64::new(1.444 * 1.444, 0.0));
let disk = Shape::circle(Point::um(0.0, 0.0), Length::um(0.4))?;
plane.draw(disk, c64::new(12.1, 0.0));
let grid = Grid { nx: 80, ny: 80, dx: 0.02, dy: 0.02, x0: -0.8, y0: -0.8 };
let solver = Solver2d::from_plane(
    grid,
    Polarization::Hz,
    Wavelength::um(1.55)?,
    &plane,
    Boundaries::pml(15),
    Averaging::Tensor,
)?;
# Ok::<(), photonoxide::Error>(())
```

---
title: "Eigenmode expansion"
module: eme
summary: "A 2D structure cut along its length into uniform sections, the field in each written as that section's modes, and the sections joined by S-matrices: slab steps and tapers, TE and TM, with a PML for the light that leaves the guide."
order: 16.5
papers:
  - cite: "P. Bienstman, R. Baets, Opt. Quantum Electron. 33, 327 (2001) (eigenmode expansion with PML: Eqs. 1–7)"
    doi: 10.1023/A:1010882531238
  - cite: "W. C. Chew, J. M. Jin, E. Michielssen, Microw. Opt. Technol. Lett. 15, 363 (1997) (a PML as a layer of complex thickness)"
    doi: 10.1002/(SICI)1098-2760(19970820)15:6<363::AID-MOP8>3.0.CO;2-C
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the layers' transfer matrices)"
    doi: 10.1364/JOSAA.1.000742
  - cite: "W. Dörfler, SIAM J. Numer. Anal. 33, 1106 (1996) (the bulk rule that marks the steps to split)"
    doi: 10.1137/0733054
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966) (the grid of a 3D section's plane: E's components at the middle of the cells' edges)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002) (the corner problems' modal indices, Figs. 4–7)"
    doi: 10.1109/JLT.2002.800371
  - cite: "P. Lietaert, K. Meerbergen, J. Pérez, B. Vandereycken, IMA J. Numer. Anal. 42, 1087 (2021) (set-valued AAA: rational functions sharing support points and weights, their Section 2.2)"
    doi: 10.1093/imanum/draa098
validation:
  - eme/step-one-mode
  - eme/step-fresnel-te
  - eme/step-fresnel-tm
  - eme/lossless-reciprocity
  - eme/lossless-power
  - eme/taper-steps-order
  - eme/taper-modes
  - eme/taper-fdfd
  - eme/taper-adaptive
  - eme/3d-orthonormal
  - eme/3d-ports
  - eme/3d-hadley
  - eme/3d-symmetry
  - eme/3d-fresnel
  - eme/3d-lossless-reciprocity
  - eme/3d-lossless-power
  - eme/3d-extruded-separable
  - eme/3d-extruded-order
  - eme/3d-extruded-slab
  - eme/3d-taper-fdfd
  - eme/3d-cut-extruded
  - eme/3d-cut-taper-exact
  - eme/3d-cut-taper-fdfd
  - eme/3d-cut-mmi-fdfd
  - eme/3d-cut-mmi-reflection
  - eme/3d-cut-mmi-reciprocity
  - eme/3d-cut-mmi-power
  - eme/wavelength-taper
  - eme/wavelength-mmi
  - eme/wavelength-reciprocity
  - eme/wavelength-power
---

Many devices are a waveguide whose cross-section changes along its length: a step, a taper, a
multimode section. Eigenmode expansion cuts such a structure into pieces that are each uniform
along the direction of travel. In each piece the field is a sum of that piece's modes, each
travelling as $e^{i\beta z}$; at each cut the modes of one piece are matched to those of the
next. What comes out is the structure's S-matrix between the modes at its two ends, with no
grid along its length, so a long uniform stretch costs no more than a short one.

This first version is 2D: the structure is uniform along y, light travels along z, and each
cross-section is a stack of layers across x between two walls. TE has E along y, TM has H along
y. Units and signs are the library's ([conventions](conventions.md)): µm, and $e^{-i\omega t}$,
so a mode goes as $e^{i(\beta z - \omega t)}$ and a lossy one has $\operatorname{Im} n_\text{eff} \gt 0$.

```rust
use photonoxide::eme::{Eme, Section, Slice};
use photonoxide::mode::Polarization;
use photonoxide::units::{Length, Wavelength};
use photonoxide::Complex64 as c64;

let slab = |t: f64| Slice::slab(c64::new(3.473, 0.0), Length::um(t), c64::new(1.444, 0.0), Length::um(3.0));
let eme = Eme::new(Polarization::Te, Wavelength::um(1.55)?, 60)?.with_pml(Length::um(4.0), 0.1)?;
let s = eme.s_matrix(&[
    Section::new(slab(0.22)?, Length::ZERO),        // port 1, at the step
    Section::new(slab(0.30)?, Length::ZERO),        // port 2
])?;
let reflected = s.s11[0][0].norm_sqr();             // fundamental mode back into itself
```

`Section::sampled` cuts a cross-section that changes along z (a taper) into equal sections, each
the cross-section at its middle. `Eme::adaptive` places a taper's steps where the response
changes instead, and states the error it leaves ([adaptive steps](#adaptive-steps)); an `eme`
job describes the sections in a file ([the job](#the-job)).

## The modes of a section

Across each layer the field U (E_y for TE, H_y for TM) and $P = (w/k)\thinspace dU/dx$, with
$w = 1$ for TE and $1/n^2$ for TM, are continuous, and a layer of thickness d carries them by
Chilwell and Hodgkinson's transfer matrix,

$$
\begin{pmatrix} U \cr P \end{pmatrix}_{x + d} =
\begin{pmatrix} \cos\Phi & \sin\Phi/\gamma \cr -\gamma\sin\Phi & \cos\Phi \end{pmatrix}
\begin{pmatrix} U \cr P \end{pmatrix}_x, \qquad \Phi = k\alpha d,\thickspace \gamma = w\alpha,\thickspace
\alpha^2 = n^2 - n_\text{eff}^2 .
$$

Every entry depends on $\alpha^2$ only, so a wall's condition (U = 0 at an electric wall for TE,
P = 0 at a magnetic one, and the other way round for TM) is an entire function of
$n_\text{eff}^2$. Between two walls the modes are discrete, guided and radiation modes alike.
Without a PML this is a Sturm–Liouville problem: the eigenvalues $n_\text{eff}^2$ are real and
simple and the m-th mode has m zeros, so the first N are found by bisection on a count of zeros
(Prüfer's angle) and none is missed.

**A PML** (Bienstman and Baets's Section 2.2, after Chew et al.) is a layer of complex thickness
$d(1 + i\sigma)$ inside a wall: the same formulas with a complex d. A wave entering it decays
without reflecting, so the light a step sends away from the guide doesn't come back from the
walls. Its modes are found by starting from a window filled with one medium, PML included,
whose modes are known in closed form, and following each with Newton's method as the layers'
permittivities move to their own. The guided modes hardly change; the radiation modes become
absorbing ones with $\operatorname{Im} n_\text{eff} \gt 0$, as in Bienstman and Baets's Fig. 2.

A half of a symmetric structure, with its plane of symmetry as the lower wall (magnetic for
TE's even modes, electric for TM's) and the PML at the upper wall only (`with_upper_pml`),
needs half the modes for the same result.

## Normalization and the interfaces

Each mode is normalized by the unconjugated product of their Eq. 4,
$\langle E_k, H_k \rangle = \int (\mathbf E_k \times \mathbf H_k)\cdot\hat z\thinspace dx = 1$, taken along
the complex coordinate in a PML; a section's modes are orthogonal in it even when they are
lossy. E is in units of $Z_0$ times H's: $H_x = -n_\text{eff} E_y$ for TE,
$E_x = (n_\text{eff}/n^2) H_y$ for TM.

At an interface between sections I and II the tangential E and H are continuous (their Eq. 2).
With $O_{ij} = \langle E^\text{I}_i, H^\text{II}_j \rangle$, testing E's continuity with section II's
modes and H's with section I's gives

$$
R = (1 + OO^T)^{-1}(1 - OO^T), \qquad T = 2\thinspace O^T (1 + OO^T)^{-1}
$$

for light from section I, and $R' = (1 + O^TO)^{-1}(O^TO - 1)$, $T' = 2\thinspace O(1 + O^TO)^{-1}$
from section II: an ideal transformer between the two sets of modes. Their Eq. 3 tests both
continuities with section I's modes; the two agree as the number of modes grows, and this form
keeps S exactly symmetric for any number of modes and, in a lossless closed window, conserves
power exactly. The overlaps are Gauss–Legendre sums on the pieces between the two sections'
layer boundaries, to round-off.

The interfaces and each section's propagation, $\operatorname{diag}(e^{i\beta_k L})$, are combined by
the scattering-matrix recursion of their Eqs. 6–7 (Redheffer's star product), which uses only
decaying exponentials and so stays stable with evanescent modes.

**S-parameters** are in the modes' amplitudes, as the [FDFD ports'](fdfd-ports.md): $\lvert S_{qp} \rvert^2$
is the share of power from mode p into mode q for a guided mode, and the reflections are the
tangential E's. `s21[q][p]` is mode q leaving port 2 for mode p entering port 1.

## Adaptive steps

A taper is a staircase of uniform steps, each the cross-section at its middle: the midpoint
rule, whose global error is second order in the step. A step $h$ long carries a local error
$c\thinspace h^3$, so splitting it into two halves leaves $2c(h/2)^3 = c\thinspace h^3/4$ of it: the change
$\Delta_i$ that splitting step $i$ makes to the structure's S is $-\tfrac34$ of that step's own
error. `Eme::adaptive` tries every step's split against the rest of the structure held fixed
(the cascades before and after each step are kept, so a try costs three overlaps and a few
star products), and estimates the staircase's error as $\tfrac43 \lvert \sum_i \Delta_i \rvert$
for each of the fundamental modes' entries $S_{11}$, $S_{21}$ and $S_{22}$: the steps' local
errors add to first order, and the sum is Richardson's estimate for splitting every step. Until
that estimate is within the tolerance, it splits the steps with the largest $\lvert \Delta_i
\rvert$ that together hold half of $\sum_i \lvert \Delta_i \rvert$ (Dörfler's bulk rule). A split step's
halves are at the quarter points its try already solved, so no mode is solved twice. A
structure's ports are its first and last pieces, uniform sections.

```rust
use photonoxide::eme::{Eme, Piece, Section, Slice, Taper};
use photonoxide::mode::Polarization;
use photonoxide::units::{Length, Wavelength};
use photonoxide::Complex64 as c64;

let slab = |t: f64| Slice::slab(c64::new(3.473, 0.0), Length::um(t), c64::new(1.444, 0.0), Length::um(3.0));
let eme = Eme::new(Polarization::Te, Wavelength::um(1.55)?, 30)?.with_pml(Length::um(4.0), 0.1)?;
let taper = Taper::new(Length::um(1.0), 4, move |z| slab(0.22 + 0.38 * z.to_um().powi(8)))?;
let out = eme.adaptive(&[
    Piece::Section(Section::new(slab(0.22)?, Length::ZERO)),
    Piece::Taper(taper),
    Piece::Section(Section::new(slab(0.60)?, Length::ZERO)),
], 1e-4)?;
let (s, error, edges) = (&out.scattering, out.error, &out.edges[0]);  // S, its estimate, the steps
```

Measured on a slab widening from 220 to 600 nm over 1 µm (TE, half the slab, 40 even modes, a
PML 4 µm thick), against Richardson's limit of 1280 and 2560 equal steps (its change from 2560
steps at most 2.1e-6), the largest error of the three entries:

| Profile | Tolerance | Adaptive steps | Stated error | True error | True error of as many equal steps |
|---|---|---|---|---|---|
| linear | 1e-4 | 44 | 4.98e-5 | 4.85e-5 | 2.20e-4 |
| $z^2$ | 1e-4 | 74 | 8.99e-5 | 9.05e-5 | 3.56e-5 |
| $z^8$ | 1e-3 | 20 | 5.91e-4 | 6.08e-4 | 3.33e-3 |
| $z^8$ | 1e-4 | 67 | 8.95e-5 | 9.77e-5 | 4.58e-4 |
| $z^8$ | 3e-5 | 144 | 2.50e-5 | 2.65e-5 | 1.52e-4 |

The stated error is within 10 % of the true one in each, and at most 2.3 % above the
tolerance's. Where the taper changes quickly in one part (the $z^8$ profile) the adaptive steps
reach the tolerance with half the steps or fewer that equal ones need. On the $z^2$ profile equal
steps happen to do better at the same count: their local errors partly cancel, which the
estimate doesn't count on. The steps are placed to meet the stated tolerance with an error that
is known, not to be the fewest possible.

## The job

A job of `kind = "eme"` describes the sections in its file (YAML, TOML or JSON, checked before it
runs): each `[[task.section]]` a straight, a core `core_um` thick in the `cladding`, or its
`layers` one by one, or a taper whose `core_um = [from, to]` changes linearly, parabolically
(its square linear) or exponentially along `length_um`, in `steps` steps. Media are a refractive
index or an entry of the [materials catalogue](catalogue.md). `[task.pml]` puts a PML inside
both walls or the upper one; `symmetric = true` solves the upper half of a structure symmetric
about its middle, its plane of symmetry the lower wall and a PML at the upper wall only (the
way every case above is run: a full window with a PML at both walls holds pairs of modes so
close that following them can merge two, which the solver refuses); `[task.adaptive]` places the tapers' steps where the response
changes, to its tolerance; `[task.convergence]` doubles the modes (and the equal steps, by
`convergence::ToTolerance::eme`) until the transmission or the reflection of the fundamental
modes changes by less than its relative tolerance; `[task.sweep]` repeats it over the
wavelength. The run records the S-parameters between the guided modes at the two ends, each
convergence study, and the adaptive steps' estimate. `photonoxide new eme` writes one to start
from; `jobs/eme-taper.yaml` is a parabolic taper with both.

```yaml
task:
  kind: eme
  wavelength_um: 1.55
  modes: 20
  symmetric: true          # the upper half, its even modes
  window_um: 1.5           # the half's width
  pml: { thickness_um: 4.0, strength: 0.1 }
  cladding: 1.444
  core: 3.473
  section:
    - { name: input, length_um: 0.0, core_um: 0.22 }
    - { name: taper, length_um: 1.0, core_um: [0.22, 0.60], profile: parabolic, steps: 8 }
    - { name: output, length_um: 0.0, core_um: 0.60 }
  adaptive: { tolerance: 1.0e-4 }
  convergence: { tolerance: 1.0e-3, quantity: transmission }
```

**In 3D,** `[task.plane]` makes each section a plane of Yee's grid ([below](#sections-in-3d)):
`cells = [across, up]` of `step_um = [dx, dy]`, centred on the strips' axis, each section a
strip `core_um` wide (a taper's `[from, to]`, in equal steps, each the strip at its middle) and
`height_um` high in the `cladding`. `symmetry = "half"` keeps x ≥ 0 behind an electric wall (the
TE-like modes, E_x even about it) and `"quarter"` keeps y ≥ 0 too behind a magnetic one;
`[task.pml]` goes inside every other wall. `modes` above the plane's unknowns keeps them all.
Beside a `[device]` the sections are cut from the device's geometry instead
([a device's sections](#a-devices-sections)): `[task.plane]` then takes `step_um` (across the
device's y and up its z), `margins_um = [beside, vertical]`, `along_um` (the longest step of a
sloped run) and `symmetry = "half"`. A run's S-parameters are `Eme3d::s_matrix`'s on the same
sections to the bit (the tests run both). `jobs/eme-taper-3d.yaml` is the strip taper of
`eme/3d-taper-fdfd`, every mode, and `jobs/eme-mmi-3d.yaml` a 1 × 2 MMI cut from the device.
Polarization, walls, `window_um`, `symmetric`, layers and `[task.adaptive]` are a 2D job's and
are refused in 3D.

```yaml
task:
  kind: eme
  wavelength_um: 1.55
  modes: 728                   # every mode of the plane
  cladding: 1.444
  core: 3.473
  plane: { step_um: [0.05, 0.05], cells: [24, 16], height_um: 0.2 }
  section:
    - { name: input, length_um: 0.0, core_um: 0.4 }
    - { name: step 1, length_um: 0.1, core_um: 0.5 }
    - { name: step 2, length_um: 0.1, core_um: 0.6 }
    - { name: output, length_um: 0.0, core_um: 0.7 }
```

## Sections in 3D

A real taper, MMI or Y-branch is a cross-section in x and y that changes along z. `Eme3d` takes
such sections: each a `Plane`, the permittivity on a uniform grid across the window, between
perfectly conducting walls, with a PML (`Pml3d`) inside them if asked.

```rust
use photonoxide::eme::{Eme3d, Plane, PlaneGrid, Section3d};
use photonoxide::units::{Length, Wavelength};
use photonoxide::Complex64 as c64;

let grid = PlaneGrid::new(24, 16, (0.05, 0.05), (-0.6, -0.4));
let strip = |w: f64| Plane::new(grid, move |x, y| {
    let core = x.abs() < 0.5 * w && y.abs() < 0.1;
    c64::new(if core { 3.473f64.powi(2) } else { 1.444f64.powi(2) }, 0.0)
});
let eme = Eme3d::new(Wavelength::um(1.55)?, 40)?;
let s = eme.s_matrix(&[
    Section3d::new(strip(0.4)?, Length::ZERO),
    Section3d::new(strip(0.5)?, Length::um(0.1)),
    Section3d::new(strip(0.6)?, Length::ZERO),
])?;
let transmitted = s.s21[0][0].norm_sqr();
```

**The modes of a section.** The unknowns are the transverse E on a plane of Yee's grid, $E_x$ at
$(i + \tfrac12, j)$ and $E_y$ at $(i, j + \tfrac12)$, the 3D FDFD's own places. A mode going as
$e^{i\beta z}$ has $E_z = i\nabla_t\cdot(\varepsilon E_t)/(\beta\varepsilon_z)$ from the continuity
equation, and what is left of the curl-curl equation is an eigenproblem for $E_t$,

$$
(C + D)\thinspace E_t = \beta^2 E_t, \qquad C = -\nabla_t\times\nabla_t\times{} + k_0^2\varepsilon, \qquad
D = \nabla_t\thinspace\varepsilon_z^{-1}\nabla_t\cdot\varepsilon,
$$

every difference Yee's, a stretched coordinate's inside the PML (Chew et al.). It is the
operator of the 3D FDFD's ports ([3D FDFD](fdfd-3d.md)) with $\beta$ in place of the grid's
$\beta_d$: the guide is discrete across and continuous along z. Faraday's law gives the transverse
H from $E_t$ alone, $(H_y, -H_x) = C E_t/(k_0\beta)$, at $E_x$'s and $E_y$'s own places. Each
permittivity is averaged over its cell of the dual grid, harmonically along the component and
arithmetically across it, from 8 × 8 points, as 3D FDFD averages it.

**Orthogonality, exactly.** With V the product of the stretches at each value of E, $VC$ is
symmetric, and since the curl of a gradient is zero on Yee's grid, $VCD$ is too. So for any two
modes $(\beta_i^2 - \beta_j^2)\thinspace E_i^T V C E_j = 0$: the unconjugated product
$\langle E_i, H_j\rangle = \sum V\thinspace E_i\cdot C E_j/(k_0\beta_j)\thinspace\Delta x\Delta y$, the discrete
$\int (E_i \times H_j)\cdot\hat z\thinspace dx\thinspace dy$ of Bienstman and Baets's Eq. 4, is zero between two
modes of a section to round-off, lossy or not, and each mode is normalized to 1 in it. The
interfaces and the cascade are then the 2D ones, unchanged. Modes of one $\beta$ (a symmetric
guide's pairs, a uniform medium's TE and TM of one order) are given the basis orthogonal in both
that product and $\sum V E_i\cdot E_j$, so the basis doesn't depend on the eigensolver; a mode's
sign is set by a weighted sum that no mirror of the window maps onto itself.

**Mirror walls.** Each of the window's four walls is electric (the tangential E zero on it)
or magnetic (`Walls`, `PlaneGrid::with_walls`). A magnetic wall is a plane of symmetry: the
tangential E on it is an unknown, and beyond it the field is its mirror image, the tangential
E even and the normal E and $H_z$ odd, so the differences across it double; each value on the
wall is weighted by half in the product, which keeps $VC$ and $VCA$ symmetric. A guide
symmetric about x = 0 and y = 0 is solved on a quarter of its window, for the modes of one
symmetry: the TE-like fundamental (E mostly along x) has an electric wall at x = 0 and a
magnetic one at y = 0. A quarter holds a quarter of the unknowns, and a dense solve costs a
64th. A PML on a magnetic wall is refused: a plane of symmetry isn't open.

**Which modes.** The N of largest $\operatorname{Re}\beta^2$. A plane of at most 1500 unknowns, or one
asked for a third of its modes or more, is solved for every mode, densely; a larger one for the
modes nearest $k_0^2\varepsilon_\text{max}$ by shift-and-invert Arnoldi, the full-vector mode
solver's. Both run on one thread, so a structure's S is the same bits on any number of threads.

**The overlaps** of two sections on the same grid are one matrix product, modes × unknowns ×
modes. A structure's distinct cross-sections are solved once each, however often they come back,
as many side by side as there are threads, and a section's modes are dropped as soon as no later
section needs them.

**Cost.** A mode holds $E_t$ and $H_t$, 32 bytes per unknown (two per cell): 60 modes of a plane
of 3920 unknowns (60 × 40 cells) are 7.5 MB, and the solve took 0.8 s. A dense solve holds three
$n \times n$ complex matrices, 108 MB at the limit of 1500 unknowns; every mode of a plane of 728
unknowns took about 4 s, and the four sections of `eme/3d-lossless-power` with all 728 modes 18 s
together (a laptop's processor, one thread per section).

**Power with fewer modes.** The interfaces keep S symmetric for any number of modes, as in 2D.
They conserve power exactly only with every mode of the plane: a 3D guide's evanescent modes
carry reactive power of either sign, and closed guides have complex modes in pairs
($n_\text{eff} = \pm 0.267 + 1.124i$ in the box of `eme/3d-lossless-power`), so the test of the
continuity with a truncated set doesn't keep the power balance that the 2D sets, all of one sign,
keep. Measured on that taper, $\lvert S_p^\dagger S_p - 1\rvert$:

| Modes kept (of 728) | 40 | 100 | 200 | 400 | 728 |
|---|---|---|---|---|---|
| $\lvert S_p^\dagger S_p - 1\rvert$ | 1.1e-4 | 1.4e-3 | 8.9e-6 | 1.5e-5 | 2.9e-13 |

That is the truncation's error, not round-off, and it isn't monotone in the number of modes. The
small reflection of the same taper converges the same way: $\lvert S_{11}\rvert^2$ = 2.60e-5,
2.54e-6 and 1.866e-5 with 40, 200 and all 728 modes.

### The PML in 3D

Measured on the strip taper of `eme/3d-taper-fdfd` opened (in oxide, a quarter of a window
0.8 × 0.6 µm inside a PML on its east and north walls, 50 nm grid; `pml_study` in the 3D tests
repeats it): the fundamental's $\lvert S_{11}\rvert^2$ and $\lvert S_{21}\rvert^2$ by the PML
and the modes kept, and the time for the four sections.

| PML | Modes | $\lvert S_{11}\rvert^2$ | $\lvert S_{21}\rvert^2$ | Time |
|---|---|---|---|---|
| 0.25 µm, uniform σ = 0.5 | 60 / 150 / all 697 | 1.16e-6 / 1.39e-6 / 2.03e-5 | 0.98539 / 0.98765 / 0.98893 | 0.5 / 0.6 / 9.5 s |
| 0.25 µm, uniform σ = 1 | 60 / 150 / all 697 | 1.64e-6 / 1.22e-6 / 2.02e-5 | 0.98610 / 0.98609 / 0.98880 | 0.6 / 0.8 / 9.4 s |
| 0.25 µm, σ = 3 as (u/d)² | 60 / 150 / all 697 | 1.91e-6 / 1.85e-6 / 1.94e-5 | 0.97664 / 0.98471 / 0.98874 | 0.7 / 0.7 / 9.7 s |
| 0.25 µm, σ = 6 as (u/d)² | 60 / 150 / all 697 | 1.04e-5 / 4.87e-6 / 1.93e-5 | 0.97609 / 0.98669 / 0.98896 | 0.6 / 0.8 / 9.6 s |
| 0.5 µm, uniform σ = 0.5 | 60 / 150 / all 1122 | 5.07e-6 / 3.28e-6 / 1.95e-5 | 0.98438 / 0.98745 / 0.98889 | 2.6 / 2.5 / 67 s |
| 0.5 µm, uniform σ = 1 | 60 / 150 / all 1122 | 7.94e-5 / 1.76e-6 / 2.01e-5 | 0.98628 / 0.98810 / 0.98891 | 2.7 / 2.8 / 66 s |

With every mode the six PMLs agree on $\lvert S_{21}\rvert^2$ to 2.2e-4 (0.98874 to 0.98896) and
on $\lvert S_{11}\rvert^2$ to 5 % (1.93e-5 to 2.03e-5): what is left is the PML's own reflection
of the light the steps scatter, which these thicknesses don't remove. With 60 or 150 modes the
transmission is 1e-3 to 1e-2 low and the reflection off by up to tenfold: an open guide's small
reflection needs nearly every mode of the plane, so a 3D section is solved densely where it can
be. Two settings fail outright: σ = 3 and 6 graded over 0.5 µm make the PML's own modes rise
above the guide's in $\operatorname{Re}\beta^2$, so the first mode, which S's first entry takes,
is not the guided one ($\lvert S_{21}\rvert^2$ = 0.0079 and 9e-8 there, with any number of
modes). S is still the structure's; its fundamental is then found by its index, not by its
place. Uniform layers of σ up to 1, as in 2D, keep the guided mode first in every case measured.

FDFD's PMLs along z in the cross-check, at $\Delta z$ = 25 nm with leads 1.5 µm long: its
$\lvert S_{21}\rvert^2$ moves by 1.9e-6, 3.8e-7 and 7.8e-8 as the PML goes from 0.5 to 1, 1.5
and 2 µm (6.5, 8.0, 10.0 and 11.3 s), five times less for each half micrometre. The leads
mattered more: 1 µm leads with a 0.5 µm PML left it 1.1e-5 lower, the guide's evanescent
modes from the steps still reaching the PML.

### A device's sections

`device::Cut` cuts a device's geometry ([devices](devices.md)) into sections. The device runs along
x, its ports facing ±x; each section's plane holds the device's y as the plane's x and its z as
the plane's y, so the expansion's z is the device's x. The plane spans the core layer's shapes
and `beside` µm of cladding, from `vertical` µm below the core layer to as far above it, its
faces on nodes when the layer is a whole number of cells thick; `mirrored()` keeps y ≥ 0 behind
an electric wall, for a device symmetric about its axis (the TE-like modes), and refuses one
that isn't. Each value of E sees the structure averaged over its cell from 8 × 8 points, as 3D
FDFD averages a cell of a structure uniform along x. The two ends are the ports' guides, led
straight out, as sections of no length.

```rust
use photonoxide::device::{Cut, Device, StripMmi, Values};
use photonoxide::eme::Eme3d;
use photonoxide::units::Wavelength;

let mmi = StripMmi::one_by_two();
let geometry = mmi.geometry(&Values::new(&mmi, &[("width", 1.6), ("length", 2.4),
    ("access_width", 0.5), ("port_width", 0.5)])?)?;
let lam = Wavelength::um(1.55)?;
let cut = Cut::new(0.05, 0.055, 0.1)?.with_margins(0.4, 0.33)?.mirrored().geometry(&geometry, lam)?;
let s = Eme3d::new(lam, cut.grid.unknowns())?.s_matrix(&cut.sections)?;   // every mode
let into_the_outputs = s.s21[0][0].norm_sqr();
```

**Where it cuts.** At every x where an outline turns (each vertex of every shape's polygons, on
every layer). Between two of them, a cross-section whose outlines run along x is one section; a
run of sloped outlines (a taper's sides, a curve's polygon) is cut into equal steps no longer
than `along`, each the cross-section at its middle. Or, `exactly()`, at every x where a sloped
outline crosses one of the points the cells are averaged from (8 a cell across, at odd
sixteenths of it): between two such x the averaged plane doesn't change, so each section holds
over its whole length and the cut has no step error. That is what a taper's steps converge to,
and they don't converge to it smoothly: the averaged plane is itself a staircase, which midpoint
steps sample. The linear taper of the cases below (400 to 800 nm over 1 µm) has 33 distinct
planes, its wall crossing a sampling point every 31.25 nm along x; its $\lvert S_{21}\rvert^2$
with every mode of the plane:

| Steps | 8 | 16 | 32 | 64, 128, 256, 512 | cut exactly (33 sections) |
|---|---|---|---|---|---|
| $\lvert S_{21}\rvert^2$ | 0.99296041 | 0.99239955 | 0.99260912 | 0.99218344 | 0.99218344 |

From 64 steps on, every step's edges hold the crossings (at odd sixty-fourths of a micrometre)
and S doesn't change in any digit; 32 steps put each crossing at a step's middle. Equal planes
side by side are one section and are solved once, so a fine cut costs its plane builds, not its
eigensolves. The exact cut is exact for polygons with vertical walls; a curve is cut at its
polygon's crossings (1 nm from the curve), and a sloped etch's walls lean out with z, which the
crossings don't follow.

**Against 3D FDFD of the same device.** The cross-check solves the device's own structure by
3D FDFD on the same plane: the same cells across and up, the same averaging and the same walls
(an electric one on the axis), the ports' guides led out through PMLs 1 µm thick, leads 1.5 µm
long, the ports' fundamental modes 0.1 µm outside the device's ports; along x at 0.1 µm over 3
to 7, and the limit at Δx → 0 fitted as `eme/3d-taper-fdfd`'s. The two share the cross-section's
discretization exactly, so they differ only along x. Silicon (3.473) in oxide (1.444), 220 nm
thick, at 1.55 µm, the half y ≥ 0, cells of 50 × 55 nm:

| Device | Plane | Ours ($\lvert S_{21}\rvert^2$; $\lvert S_{11}\rvert^2$) | 3D FDFD's limit | Gap | FDFD's own error |
|---|---|---|---|---|---|
| Linear taper, 400 to 800 nm over 1 µm, cut exactly | 16 × 16 cells, 480 unknowns, every mode | 0.9921834; 1.2416e-4 | 0.9921745 (fit residual 3.8e-5); 9.28e-5 | 8.9e-6; not compared | 1.3e-4; 3.2e-5 (with or without its coarsest step) |
| 1 × 2 MMI, 1.6 × 2.4 µm, 0.5 µm access guides | 24 × 16 cells, 728 unknowns, every mode | 0.9056087; 1.6111e-3 | 0.9056635 (8.9e-7); 1.61168e-3 (1.6e-7) | 5.5e-5; 5.4e-7 | 6.3e-5 (its PML from 1 to 2 µm); 4.5e-6 (its coarsest step) |

The MMI's half $\lvert S_{21}\rvert^2$ is the power into its two outputs together: the half's
port at the output is the two guides' even mode. With every mode its S is symmetric to 2.5e-13
and its propagating block unitary to 1.3e-12. FDFD's own values along x:

| Δx (nm) | 33.3 | 25 | 20 | 16.7 | 14.3 |
|---|---|---|---|---|---|
| Taper $\lvert S_{21}\rvert^2$ | 0.9923949 | 0.9924169 | 0.9923467 | 0.9923580 | 0.9922652 |
| Taper $\lvert S_{11}\rvert^2$ | 6.55e-5 | 1.032e-4 | 1.102e-4 | 9.41e-5 | 1.097e-4 |
| MMI $\lvert S_{21}\rvert^2$ | 0.8988133 | 0.9018723 | 0.9032539 | 0.9039972 | 0.9044433 |
| MMI $\lvert S_{11}\rvert^2$ | 2.362e-3 | 1.982e-3 | 1.834e-3 | 1.760e-3 | 1.718e-3 |

The MMI's FDFD values close on its limit at second order, its faces on nodes. The taper's don't:
FDFD averages each cell from 8 points along x too, so a sloped wall's crossings fall at
eighths of a cell and scatter its values by about 1e-4 about their trend, and its small
reflection by 30 %. So the taper's reflection is reported here, not checked: ours is 1.2416e-4,
FDFD's limit 9.28e-5 from these five steps and 1.245e-4 with 50 nm added. FDFD's PMLs and leads,
at Δx = 25 nm: the MMI's $\lvert S_{21}\rvert^2$ is 0.9034558, 0.9018723 and 0.9018091 with PMLs
and leads of 0.5 and 1, 1 and 1.5, and 2 and 1.5 µm; the light the MMI scatters needs the room.

**Size.** The studio's 6 µm MMI, its first image about 33 µm long, would take a half plane of
about 2200 unknowns on these cells and an FDFD solve of millions; the 1.6 µm one is what a test
holds. One FDFD solve at Δx = 50 nm took 1.9 s for the taper (92 000 unknowns) and 4.3 s for the
MMI (170 000), the five steps 40 s and 100 s together, the direct solver's memory peaking near
3 GB; every mode of the MMI's three planes took 16 s, of the taper's 33 about a minute (six
threads).

## S over wavelength

`Spectrum::run` gives a structure's S between the first modes at its two ends as a function of
wavelength from as few solves as a tolerance needs. Every entry of S is sampled at the same
wavelengths, one solve each, and the entries are fitted together by set-valued AAA (Lietaert,
Meerbergen, Pérez and Vandereycken 2021, their Section 2.2; [the sweeps' page](sweep.md#several-responses-on-one-set-of-samples)):
rational functions that share their support points and weights, so one denominator. The next
wavelength is where the worst entry's two interleaved halves disagree most, and the sweep stops
when every entry's estimate (twice the largest difference between the fit to all solves and
either half's, the estimate that `sweep/estimate-bounds-error` measured on 800 sweeps) is within
the tolerance, relative to the largest |S_qp| solved. S's entries are at most 1, so it is about
an absolute tolerance on S.

```rust
use photonoxide::eme::{Eme, Section, Slice, Spectrum, Wall};
use photonoxide::mode::Polarization;
use photonoxide::sweep::Options;
use photonoxide::units::{Length, Wavelength};
use photonoxide::Complex64;

// half of a step from a 220 to a 300 nm slab and back, 20 even modes, a PML
let half = |t: f64| {
    Slice::new(&[
        (Complex64::new(3.473, 0.0), Length::um(0.5 * t)),
        (Complex64::new(1.444, 0.0), Length::um(1.5 - 0.5 * t)),
    ])
};
let solve = |l: Wavelength| {
    Eme::new(Polarization::Te, l, 20)?
        .with_walls(Wall::Magnetic, Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)?
        .s_matrix(&[
            Section::new(half(0.22)?, Length::ZERO),
            Section::new(half(0.30)?, Length::um(0.5)),
            Section::new(half(0.22)?, Length::ZERO),
        ])
};
let s = Spectrum::run(solve, Wavelength::um(1.5)?, Wavelength::um(1.6)?, (1, 1), &Options::new(1e-8))?;
assert!(s.converged() && s.error() <= 1e-8);
let t = s.at(Wavelength::um(1.55)?)[1][0]; // S21 between the TE0 modes
# let _ = t;
# Ok::<(), photonoxide::Error>(())
```

- **What the shared denominator keeps.** A linear relation that every solve's S keeps, the
  interpolant keeps everywhere: S = Sᵀ holds for the solves to round-off (Lorentz reciprocity,
  `eme/lossless-reciprocity`), and so for the interpolant (`eme/wavelength-reciprocity`). Power
  is quadratic in S: the interpolant keeps a column's Σ_q |S_qp|² to Σ_q (2|S_qp| + e_qp) e_qp,
  e_qp each entry's estimate (`eme/wavelength-power`).
- **One discretisation.** The solves must be the same discretisation at every wavelength (the
  same modes and steps), so that S is one smooth function of it; a solve whose modes or steps
  change with the wavelength makes S jump, and the estimate counts the jumps as error. A job
  settles them once, at its `wavelength_um` (below).
- **A PML, not a closed window.** In a closed window the box modes cross cutoff (β = 0) at
  wavelengths inside the range, and S has a square-root branch point at each: a rational
  function approaches it slowly. Measured on the step above in a closed window with 10 modes,
  asked for 1e-8 from 1.5 to 1.6 µm, the estimate is 5.5e-3 after 20 solves, 1.3e-4 after 50 and
1.7e-6 after 100, and 4.5e-7 at the limit of 200, far from converged. With the PML no mode has a real cutoff, and the structures below
  converge.

On two structures, asked for 1e-6, against the same expansion solved at 201 wavelengths evenly
spaced over the range:

| Structure | Range | Entries | Solves | Estimate | True error on 201 wavelengths | Ratio | $\lvert r - r^T \rvert$ | Power ratio |
|---|---|---|---|---|---|---|---|---|
| the taper of `jobs/eme-taper.yaml` on 16 equal steps, 40 even modes | 1.45 to 1.65 µm | 9 (TE0 at 220 nm; TE0, TE2 at 600 nm) | 32 | 5.1e-7 | 3.9e-13 | 7.5e-7 | 1.8e-14 | under 1e-3 |
| a 400 nm slab into one 3 µm wide and 20 µm long and out, 40 even modes | 1.53 to 1.57 µm | 4 (TE0 at each end) | 117 | 3.1e-7 | 2.5e-12 | 7.8e-6 | 1.0e-14 | under 1e-3 |

The solves saved against the 201: 169 (84 %) on the taper, 84 (42 %) on the multimode section,
whose seven even modes beat along its 20 µm and turn $S_{21}$ by about 0.16 every 2 nm. Over
1.45 to 1.65 µm that section needs more than 200 solves: at the limit of 200 its estimate was 1.7
and its true error on 201 wavelengths 0.05, so the estimate still bounded it (a ratio of 0.029),
but did not converge. The estimate is cautious: the interpolants are 1e5 to 1e6 times more
accurate than it says (their true error 3.9e-13 and 2.5e-12 where it states 5.1e-7 and 3.1e-7),
since each half has only half the solves; a sweep stops later than it needs to, never earlier.
The solves' own reciprocity, 8.9e-15 and 1.7e-15, the interpolants keep to 1.8e-14 and 1.0e-14.
The estimate is a bound only where the fit has seen the response (the
sweeps' page, [What it can't see](sweep.md#the-error-estimate)): a resonance narrower than the
grid of 1001 wavelengths the estimate is taken on would be missed.

**In a job,** `[task.interpolate]` with a `tolerance` (and `max_solves`, 200 by default) makes the
`eme` job's `[task.sweep]` an interpolation: the modes (driven by `[task.convergence]` when the
job has one, by `convergence::ToTolerance` as at any wavelength) and the adaptive steps are
settled once at `wavelength_um`, every solve keeps them, and the run records S at each of the
sweep's points from the interpolant, between the modes guided at both ends of the range, then
the estimate with the wavelengths solved. 2D and 3D jobs alike. `jobs/eme-mmi-sweep.yaml` is the
multimode section above, 201 points over the C band from 117 solves.

```yaml
  sweep: { parameter: wavelength, from: 1.45, to: 1.65, points: 201 }
  interpolate: { tolerance: 1.0e-6 }
```

## Periodic sections

A grating is one period repeated. Two things follow from the period's S-matrix taken between its
first section's modes at its start and at the next period's start (`Eme::bloch` appends a
zero-length copy of the first section to the period for that).

**Its Bloch modes.** With $a_1 = (a_1^+, a_1^-)$ the amplitudes of those modes at the period's
start and $a_2$ at the next one's, the S-matrix gives $a_2 = T a_1$ with

$$T = \begin{pmatrix} S_{21} - S_{22} S_{12}^{-1} S_{11} & S_{22} S_{12}^{-1} \\ -S_{12}^{-1} S_{11} & S_{12}^{-1} \end{pmatrix},$$

and a Bloch mode is an eigenvector of $T$ with the eigenvalue $\lambda = e^{iK\Lambda}$, $K$ its
wavenumber and $\Lambda$ the period (Yeh, Yariv and Hong 1977, Eqs. (24)–(27), there for one
mode). $S_{12}$ of an evanescent mode is exponentially small, so $T$ itself is ill-conditioned
with many modes; the eigenvalues are taken instead from the pencil

$$\begin{pmatrix} S_{21} & 0 \\ -S_{11} & 1 \end{pmatrix} x = \lambda \begin{pmatrix} 1 & -S_{22} \\ 0 & S_{12} \end{pmatrix} x,$$

the same equations multiplied out, by QZ, which inverts nothing. In a reciprocal structure the
eigenvalues come in pairs $\lambda$ and $1/\lambda$, a Bloch mode going either way. Each pair is
one `Bloch` mode: $\cos K\Lambda = (\lambda + 1/\lambda)/2$, and $K/k_0$ from the one of the pair
with $\lvert\lambda\rvert \le 1$, in the reduced zone (its real part between 0 and $\lambda_0/2\Lambda$,
up to its sign, which is the direction; its imaginary part the decay along z). The $m$-th Bloch
mode is the one most made of the section's $m$-th mode, of those left: the fundamental first. A
Bloch mode that decays by $e^{-\operatorname{Im} K\Lambda}$ in one period has $\lambda$ to the
eigenproblem's round-off, an absolute one, so its $K$ is good to about $\varepsilon\,e^{\operatorname{Im} K\Lambda}/\Lambda$:
the guided, propagating and stop-band modes are at round-off, the strongly evanescent ones
approximate.

**N periods** (`Eme::grating`, `Eme3d::grating`): a structure's sections with some of them
repeated `count` times in place. The period's S-matrix is taken to the power `count` − 1 by
repeated squaring of the star product (about $2\log_2 N$ products instead of $N$), and starred
with what comes before the period and with the last period and what follows it. The last period
ends where the structure goes on, not at a zero-length copy of its first section: with finitely
many modes, a zero-length section between two others adds an interface pair that is not the
identity (5e-3 on the grating below with 20 modes).

```rust
use photonoxide::eme::{Eme, Section, Slice, Wall};
use photonoxide::mode::Polarization;
use photonoxide::units::{Length, Wavelength};
use photonoxide::Complex64;

// half of a silicon slab grating, 300 and 220 nm thick, 0.14 µm each, 20 even modes, a PML
let half = |t: f64| {
    Slice::new(&[
        (Complex64::new(3.473, 0.0), Length::um(0.5 * t)),
        (Complex64::new(1.444, 0.0), Length::um(1.5 - 0.5 * t)),
    ])
};
let eme = Eme::new(Polarization::Te, Wavelength::um(1.55)?, 20)?
    .with_walls(Wall::Magnetic, Wall::Electric)
    .with_upper_pml(Length::um(4.0), 0.1)?;
let sections = [
    Section::new(half(0.22)?, Length::ZERO),        // port 1
    Section::new(half(0.30)?, Length::um(0.14)),    // the period: sections 1 and 2
    Section::new(half(0.22)?, Length::um(0.14)),
    Section::new(half(0.22)?, Length::ZERO),        // port 2
];
let bloch = eme.bloch(&sections[1..3])?;            // the period's Bloch modes
let fundamental = bloch.effective_indices[0];       // K/k0 in the reduced zone
let s = eme.grating(&sections, 1..3, 100)?;         // 100 periods
# let _ = (fundamental, s);
# Ok::<(), photonoxide::Error>(())
```

Checked at round-off on a quarter-wave stack at 1.55 µm (silica, 1.444, and silicon nitride,
2.0), each layer uniform across a window between magnetic walls, so that TE's modes are
$\cos(m\pi x/W)$ in both layers with $n_\text{eff}^2 = n^2 - (m\lambda/2W)^2$ and each meets only
its own at the interfaces:

| Check | Ours | Closed form |
|---|---|---|
| $\cos K\Lambda$ of the one uniform mode at 61 wavelengths from 1 to 2.5 µm, the first stop band among them (`eme/bloch-quarter-wave`) | within 6.7e-16 | $\cos k_1 d_1 \cos k_2 d_2 - \tfrac12 (n_1/n_2 + n_2/n_1) \sin k_1 d_1 \sin k_2 d_2$ |
| the same with 8 modes in a window 2 µm wide, propagating and evanescent, at 1.3, 1.55 and 1.8 µm (`eme/bloch-quarter-wave-modes`) | within 8.1e-15 of each mode's closed form, relative to $\lvert\cos K\Lambda\rvert$ where it is above 1 (up to 26) | the same with $\beta_i = k_0 n_{\text{eff},i}$ |
| the first stop band's width, its edges by bisection on $\cos K\Lambda = -1$ (`eme/bloch-band-edges`) | 0.20645554758478 | $\Delta\omega/\omega_0 = (4/\pi)\arcsin((n_2 - n_1)/(n_2 + n_1))$ = 0.20645554758479, relative 2.0e-15 |
| the slab grating above, 100 periods by squaring (8 star products) against its 200 sections in sequence (`eme/grating-squaring`) | every entry of S within 2.5e-14 | the same products in another order |

**In a job,** `[task.period]` names the sections of one period, counted from 1 between the
ports, and how many periods: the run solves the structure by repeated squaring, and records the
period's Bloch modes at each wavelength it solves (at `wavelength_um` alone with
`[task.interpolate]`) as a sweep point, the four least decaying by the order above, each K/k₀ as
an effective index. 2D and 3D jobs (a task's strips) alike; a `[device]` is cut whole, with no
period of its own, and the adaptive steps place a whole structure's, so a job with a period
takes equal steps. `jobs/eme-grating.yaml` is the grating above, swept over its stop band.

```yaml
  section:
    - { name: input, length_um: 0.0, core_um: 0.22 }
    - { name: wide, length_um: 0.14, core_um: 0.30 }
    - { name: narrow, length_um: 0.14, core_um: 0.22 }
    - { name: output, length_um: 0.0, core_um: 0.22 }
  period: { sections: [2, 3], count: 100 }
```

## Validation

Measured with the cases' own settings, at 1.55 µm, silicon (3.473) in oxide (1.444):

| Case | What | Ours | Reference | Rule |
|---|---|---|---|---|
| `eme/step-one-mode` | a 220 to 300 nm step with one mode a side, R and \|T\|, TE and TM | within 4.5e-13 relative of the closed form (TE: R = −0.0314958, \|T\| = 0.999504) | mode-matching's closed form on the exact slab modes, every integral in closed form | round-off, 1e-10 |
| `eme/step-fresnel-te`, `-tm` | a step between two uniform media, 20 modes | within 1.4e-14 (TE) and 2.9e-15 (TM) of Fresnel's coefficient at each mode's angle, no coupling between modes | Fresnel's $r_s$, and $-r_p$ for the tangential E | round-off, 1e-12 |
| `eme/lossless-reciprocity` | a stepped taper in a closed window, 30 modes | $\lvert S - S^T \rvert$ at most 1.7e-15 | Lorentz reciprocity | round-off, 1e-12 |
| `eme/lossless-power` | the same | the propagating modes' block of S unitary within 2.6e-15 | power conservation | round-off, 1e-12 |
| `eme/taper-steps-order` | a slab widening linearly from 220 to 600 nm over 1 µm, 40, 80 and 160 sections | order 2.07 (2.18 from 10 to 40 sections) | second order, the midpoint rule's staircase | order, 2 within 0.1 |
| `eme/taper-modes` | the same with 40, 80 and 160 modes, $\lvert S_{21} \rvert^2$ = 0.996513, 0.996506, 0.996525 | estimated error 1.9e-5 at 160 modes | the convergence gate's estimate | discretisation, 1e-4 |
| `eme/taper-fdfd` | a 220 to 300 nm taper in four steps 0.1 µm long, TE, its reflection $\lvert S_{11} \rvert^2$ | 9.18190e-5 with 200 modes (own error 3.4e-5, the change from 160) | 2D FDFD on the same staircase: 9.19007, 9.18382 and 9.18225e-5 on 10, 5 and 2.5 nm grids, second order, limit 9.18173e-5 | discretisation, 2e-4 relative; 1.9e-5 |
| `eme/taper-adaptive` | a slab widening from 220 to 600 nm over 1 µm as $z^8$, its steps placed adaptively to 1e-4, the largest error of $S_{11}$, $S_{21}$, $S_{22}$ | 9.77e-5 with 67 steps (stated 8.95e-5; 67 equal steps: 4.58e-4) | Richardson's limit of 1280 and 2560 equal steps (own error 2.1e-6) | discretisation, 1e-4 absolute, the tolerance driven to |
| `eme/3d-orthonormal` | a 500 × 220 nm strip, a window 2.4 × 1.6 µm on a 40 nm grid, a PML 0.4 µm thick of strength 1, 60 modes: $\lvert\langle E_i, H_j\rangle - \delta_{ij}\rvert$ | 4.9e-12 | 0: the scheme's Lorentz reciprocity | round-off, 1e-10 |
| `eme/3d-ports` | a 400 × 200 nm strip in a box 1.2 × 0.8 µm on a 50 nm grid, its 6 propagating modes | $\beta^2$ within 1e-10 of the 3D FDFD ports' $\beta_d^2$ | the same transverse eigenproblem | round-off, 1e-10 |
| `eme/3d-hadley` | Hadley's four corner problems (ε = 2.25 or 8 in air, the 1 × 1 µm quarter domain between electric and magnetic walls, λ = 1.5 µm) on 40, 80 and 160 cells a side, Richardson's limit at the observed order (1.9, 2.0, 1.7, 1.3) | within 4.7e-7 relative of each (on 160 cells: 4.8e-6, −5.8e-6, 2.8e-6, 9.3e-6 off) | Hadley's 1.27627404, 2.65679692, 1.387926425, 2.761465320 | discretisation, 1.2e-5 relative, three times the finest grid's own error (3.8e-6) |
| `eme/3d-symmetry` | the strip taper of `eme/3d-taper-fdfd` on a quarter of its window, every mode, against the whole window with all 728 | $S_{21}$ within 2.0e-14, the fundamental's $\beta^2$ within 3.3e-14 | the mirror image | round-off, 1e-10 |
| `eme/3d-fresnel` | a step between uniform media in a box 1.3 × 1.1 µm, all 236 modes | within 7.7e-12 of Fresnel's coefficient at each mode's discrete angle, TE and TM, no coupling | Fresnel's $r_s$ and $-r_p$ | round-off, 1e-10 |
| `eme/3d-lossless-reciprocity`, `-power` | a strip 200 nm thick widening from 400 to 700 nm in two steps 0.1 µm long, a box 1.2 × 0.8 µm on a 50 nm grid, all 728 modes | $\lvert S - S^T\rvert$ 3.6e-15; $\lvert S_p^\dagger S_p - 1\rvert$ 2.9e-13 | Lorentz reciprocity, power conservation | round-off, 1e-10 |
| `eme/3d-extruded-separable` | the 2D lossless stepped slab as 3D sections uniform in y, 4 cells across against 1 (10 nm grid, 30 modes) | every entry of S within 4.9e-12 | the same discrete eigenproblem | round-off, 1e-10 |
| `eme/3d-extruded-order` | the same, 1 cell across, on 10, 5 and 2.5 nm grids | order 2.00 | Yee's second order | order, 2 within 0.1 |
| `eme/3d-extruded-slab` | the same: Richardson's limit of $S_{11}$ | $-0.00863322 - 0.00413359i$, within 4.8e-10 of the 2D expansion's (relative 5e-8) | the 2D expansion on the exact slab modes, 30 modes | discretisation, 3e-4 relative, three times the finest grid's own error (9.3e-5) |
| `eme/3d-taper-fdfd` | the lossless strip taper above, $\lvert S_{21}\rvert^2$, every one of its 728 modes | 0.996238 (and $\lvert S_{11}\rvert^2$ 1.866e-5) | 3D FDFD on the same plane at $\Delta z$ = 0.1 µm over 2 to 7, its limit by a fit of $v_0 + a\Delta z^2 + b\Delta z^4$: 0.996238, within 4e-7 (its $\lvert S_{11}\rvert^2$ 1.945e-5 at the finest step, approaching at second order) | discretisation, 4e-5 absolute, three times FDFD's PML error |
| `eme/3d-cut-extruded` | the stepped slab of `eme/3d-extruded-separable` drawn as strips on a layer one 10 nm cell thick, cut where its outlines turn | every entry of S the hand-built sections' to round-off (under 1e-10) | the same planes | round-off, 1e-10 |
| `eme/3d-cut-taper-exact` | a linear strip taper, 220 nm thick, 400 to 800 nm over 1 µm, the half y ≥ 0 on 50 × 55 nm cells, every one of 480 modes, cut exactly (33 sections) | every entry of S 64 midpoint steps' to round-off (under 1e-10) | the same 33 planes | round-off, 1e-10 |
| `eme/3d-cut-taper-fdfd` | the same, $\lvert S_{21}\rvert^2$ | 0.9921834 | 3D FDFD of the device on the same plane at Δx = 0.1 µm over 3 to 7: 0.9921745 (fit residual 3.8e-5) | discretisation, 4e-4 absolute, three times FDFD's own error (its limit moves 1.3e-4 with its coarsest step) |
| `eme/3d-cut-mmi-fdfd`, `-reflection` | a 1 × 2 strip MMI 1.6 × 2.4 µm, 0.5 µm access guides, the half y ≥ 0 on 50 × 55 nm cells, every one of 728 modes: $\lvert S_{21}\rvert^2$ (into both outputs) and $\lvert S_{11}\rvert^2$ | 0.9056087 and 1.6111e-3 | the same FDFD: 0.9056635 and 1.61168e-3 (residuals 8.9e-7, 1.6e-7) | discretisation, 2e-4 and 1.4e-5 absolute, three times FDFD's own error (its PML, 6.3e-5; its coarsest step, 4.5e-6) |
| `eme/3d-cut-mmi-reciprocity`, `-power` | the same MMI | $\lvert S - S^T\rvert$ 2.5e-13; $\lvert S_p^\dagger S_p - 1\rvert$ 1.3e-12 | Lorentz reciprocity, power conservation | round-off, 1e-10 |
| `eme/wavelength-taper`, `-mmi` | S over a range (1.45 to 1.65 µm, the C band) from a few solves, every entry on one set of wavelengths, asked for 1e-6 ([S over wavelength](#s-over-wavelength)) | ratio of the true error to the estimate 7.5e-7 (taper, 32 solves) and 7.8e-6 (multimode section, 117 solves) | the same expansion at each of 201 wavelengths | discretisation, a ratio of the true error to the estimate of at most 1, measured |
| `eme/wavelength-reciprocity` | the two interpolants on the 201 wavelengths | $\lvert r - r^T \rvert$ 1.8e-14 | the solves' own reciprocity, kept by a shared denominator | round-off, 1e-12 |
| `eme/wavelength-power` | the same: each column's power against the solves' | under 1e-3 of the bound $\sum_q (2\lvert S_{qp}\rvert + e_{qp}) e_{qp}$ | the bound the entries' estimates give | discretisation, a ratio of at most 1, measured |

The FDFD comparison is the most telling: the two methods share nothing but the structure, and
agree to 1.9e-5 of a reflection of 9.2e-5. Its transmission agrees too: 0.999829 against
FDFD's 0.999831. It needs a thick PML. With 1 µm of PML of strength 0.025 to 0.1 the
expansion's reflection converges, as the modes increase, to between 1.1e-3 and 6e-4 below FDFD's:
light leaving the guide at a grazing angle travels a long way along a thin, weak PML and comes
back. FDFD's own PML (0.2 or 0.4 µm) changes its result by under 1e-12.

In 3D the cross-check shares the cross-section's discretization with FDFD exactly, so it
measures the one thing that differs, z: the expansion with every mode is the exact solution of
the equations discrete across the guide and continuous along it, and FDFD's $\lvert S_{21}\rvert^2$
(0.9962164, 0.9962334, 0.9962365, 0.9962373, 0.9962376, 0.9962378 at $\Delta z$ = 50 to 14.3 nm)
closes on it at second order. FDFD needs room for that: with PMLs 0.5 µm thick and leads 1 µm
long its values sit 1.1e-5 lower, and with 10 cells of PML and short leads they don't converge
at all, as the guide's higher and evanescent modes come back from a PML too thin for them. Below
14 nm along z, on that 50 nm plane, its direct solve leaves a residual of 1e-9 that QMR can't
finish, so the steps stop at 0.1/7 µm.

## Limits

What it does not do yet (#432):

- **3D sections are planes given by a function** on a uniform grid. At convex dielectric
  corners the plane's modes converge more slowly than second order (1.3 on Hadley's fourth
  problem), as standard finite differences do there. A strong graded PML can put its own modes
  first, ahead of the guided one ([the PML in 3D](#the-pml-in-3d)), and an open guide's small
  reflection needs nearly every mode of the plane. With
  fewer than every mode of the plane, power is conserved only to the truncation's error
  ([above](#sections-in-3d)).
- **Sections are described or cut from a device:** a job gives straights and tapers of a core
  or of layers in 2D, or strips on a plane in 3D, or cuts a `[device]` into planes
  ([a device's sections](#a-devices-sections)); a job's own drawn shapes, a façade function of
  its own and a view of its own in the studio (which shows its scene and S-parameters as any
  job's, a device's job opening on the device's page) are to come. The Python package runs it as
  any job, through `run_job`. A device is cut along x only, its ports facing ±x; a sloped etch's
  walls, which lean with z, are cut in steps, not exactly; and the cut takes no bodies over the
  stack.
- **A device's size:** every mode of a plane is a dense solve, at most a few thousand unknowns
  in reasonable time, and the 3D FDFD that checks it millions of unknowns for a device tens of
  micrometres long. The cases check a 1.6 µm MMI and a 1 µm taper; the studio's 6 µm MMI is not
  a case.
- **Bends and periodic sections** are to come ([the bends' plan](../plans/eme-bends.md)).
- **S over wavelength** is interpolated in the wavelength only: a structure's own parameters
  (a width, a length) are swept by solving each value. Its estimate holds where the fit has seen
  the response, and needs one discretisation for every solve and a PML window
  ([above](#s-over-wavelength)).
- **No published taper or MMI yet:** no paper we have gives a taper's or an MMI's S by
  eigenmode expansion with the numbers needed to repeat it (Bienstman and Baets's grating sends
  its material data to another exercise; Soldano and Pennings's self-imaging gives closed forms,
  not a device's indices). The taper is checked against FDFD and against itself in steps and
  modes. Read again for 3D (2026-11-05): Soldano and Pennings's Table II
  ([doi:10.1109/50.372474](https://doi.org/10.1109/50.372474)) gives III-V MMIs by their
  ridges' effective index, not a 3D stack; Halir et al.'s conventional 2×2 MMI
  ([doi:10.1002/lpor.201600213](https://doi.org/10.1002/lpor.201600213), Table 1: 220 nm
  silicon in oxide, 3.25 × 38.5 µm) prints its geometry but its beat length only from
  approximations (27 and 28 µm at 1.5 µm) and its splitting only as a plot; Zhang et al.'s
  tapered junction ([doi:10.1364/OE.21.001310](https://doi.org/10.1364/OE.21.001310), Table 1)
  prints 0.13 dB by 3D FDTD but not its arms or indices. None gives a 3D device and a printed
  response to repeat, so a device's sections are checked against 3D FDFD of the same device on
  the same plane ([above](#a-devices-sections)), not against a paper.
- **A PML's modes are ill-conditioned** (the operator is far from normal), more so the
  stronger the PML and the higher the mode. A thick PML of small strength keeps them tractable:
  4 µm of strength 0.1 with up to about 200 modes in a half-window 1.5 µm wide. Where a mode
  can't be followed, `Eme::modes` returns an error that names it rather than a wrong mode.
  In that window (`eme/taper-fdfd`'s) the first mode lost is the uniform window's 230th, its
  $n_\text{eff}^2$ near $-1027 + 151i$. Such a mode grows by about $e^{15}$ from each wall to
  the PML's inner side, so the unconjugated $\int U^2\thinspace d\tilde x$ that normalizes it is
  about $2 \times 10^{-11}$ of $\int \lvert U\rvert^2\thinspace dx$ (estimated from that growth).
  The inverse ratio, about $5 \times 10^{10}$, is the eigenvalue's condition number: rounding
  moves $n_\text{eff}^2$ by about $10^{-16}$ times that times $\lvert n_\text{eff}^2\rvert$, a few
  $10^{-3}$. That is the scatter Newton's method returns (2 to 6 × 10⁻³, measured) on any form
  of the walls' condition. A global system with each layer's own
  exponentials was tried and resolves it no better. The following refuses a step whose
  correction is above $2 \times 10^{-6}$ of $\lvert n_\text{eff}^2\rvert$, and the growth, and
  with it the scatter, rises exponentially with the mode's order. So the stepped taper's
  $\lvert S_{11}\rvert^2$ can't be driven to $10^{-4}$ relative, which would take about 400
  modes (`convergence/eme-taper-fdfd` is driven to $2 \times 10^{-4}$). Higher precision for the
  high modes, or a PML whose high modes don't grow across the window, would settle it.
  Decided (2026-11-04), not done: compensated (double-double) arithmetic would have to carry
  the whole chain, the layers' transfer matrices, Newton's corrections and the normalizing
  integral, since a condition number of $5 \times 10^{10}$ leaves no digits to spare at any
  step; that is a rewrite of the mode finder with its own tests, not a change to it. The cheaper
  way, a PML that the high modes don't grow across (a graded strength, so that each mode is
  absorbed before it grows), changes the modes every case here is measured with. Both wait for
  the 3D expansion ([the plan](../plans/eme-3d.md)), whose PML will be chosen with the same
  measurement; until then, more than about 230 modes in that window is refused with an error
  that names the mode, never returned wrong.
- **The multimode interference coupler** against Soldano and Pennings's self-imaging lengths is
  not a case: their closed forms are the guided-mode analysis the MMI device's 2D model already
  is, not a 3D device's response; the 3D MMI is checked against 3D FDFD instead.

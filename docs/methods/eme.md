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
  - cite: "P. Yeh, A. Yariv, C.-S. Hong, J. Opt. Soc. Am. 67, 423 (1977) (a period's Bloch waves from its transfer matrix, Eqs. (24)–(27))"
    doi: 10.1364/JOSA.67.000423
  - cite: "M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975) (the conformal map that turns a bend into a straight guide of index n e^(u/R))"
    doi: 10.1109/JQE.1975.1068563
  - cite: "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971) (a bent slab's modes matched to the outgoing Hankel function, Eq. 10: the exact bend the bends are checked against)"
    doi: 10.1002/j.1538-7305.1971.tb02620.x
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
  - eme/bloch-quarter-wave
  - eme/bloch-quarter-wave-modes
  - eme/bloch-band-edges
  - eme/grating-squaring
  - eme/bloch-grating-lossless
  - eme/bloch-grating-pml-order
  - eme/bend-index
  - eme/bend-loss
  - eme/bend-order
  - eme/bend-halves
  - eme/bend-no-angle
  - eme/bend-joint
  - eme/bend-joint-leaky
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
so a mode goes as $e^{i(\beta z - \omega t)}$ and a lossy one has $\mathop{\mathrm{Im}} n_\text{eff} \gt 0$.

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
absorbing ones with $\mathop{\mathrm{Im}} n_\text{eff} \gt 0$, as in Bienstman and Baets's Fig. 2.

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

The interfaces and each section's propagation, $\mathop{\mathrm{diag}}(e^{i\beta_k L})$, are combined by
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
modes changes by less than its relative tolerance, and with bends then halves the bends' thin
layers (from `bend_step_nm`, 10 nm by default) at the modes it found until the same quantity
changes by less than the same tolerance, each study recorded; `[task.sweep]` repeats it over the
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
sloped run) or `exact = true` (a cut with no step error), and `symmetry = "half"`. A task's own
drawing is cut the same way ([a job's own drawing](#a-jobs-own-drawing)). A run's S-parameters
are `Eme3d::s_matrix`'s on the same sections to the bit (the tests run both).
`jobs/eme-taper-3d.yaml` is the strip taper of `eme/3d-taper-fdfd`, every mode,
`jobs/eme-mmi-3d.yaml` a 1 × 2 MMI cut from the device, and `jobs/eme-taper-drawn-3d.yaml` a
taper drawn in the job as a path and cut exactly.
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

The run records, after the solver's settings, the sections along the structure
(`eme_sections`: each one's name, kind, start, length, steps, a bend's radius and angle, a
period's repeats; a device's or a drawing's cut planes as `cut 1`, `cut 2`, ...).

### From Python

`photonoxide.eme(job)` (the façade's `facade::eme`) runs any `eme` job, 2D or 3D, without a
folder and returns what its run records: the solver's settings, the sections, S between the
guided modes at the two ends at each wavelength (`points`, each with its ports, their effective
indices and S as an `(n, n)` array), the convergence studies, the estimated errors and a
period's Bloch modes. Every number is the run's to the bit (the façade's conformance cases
check Python against Rust).

```python
import photonoxide as phox

e = phox.eme("jobs/eme-taper.yaml")
print([(s.name, s.kind, s.length_um) for s in e.sections])
wavelengths, t = e.transmission("2 TE0", "1 TE0")
print(f"|S21|^2 = {t[0]:.6f} at {wavelengths[0]} um")
for study in e.studies:
    print(study.label, study.values[-1], "converged" if study.converged else study.reason)
```

### In the studio

An `eme` run's page shows its sections along the structure as a strip to scale (each kind its
own colour, a period's sections marked with their repeats), its S-parameters over wavelength
as |S|² in dB between the guided modes, with an interpolated sweep's estimated error and the
wavelengths it solved, and each convergence study's values against its modes, steps or layers.

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

**Which modes.** The N of largest $\mathop{\mathrm{Re}}\beta^2$. A plane of at most 1500 unknowns, or one
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
above the guide's in $\mathop{\mathrm{Re}}\beta^2$, so the first mode, which S's first entry takes,
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

### A job's own drawing

An eme job needs no `[device]` to be cut: its task can draw its own structure, as a structure
or FDFD job does, and `[task.plane]` cuts it the same way (`Cut::structure`). The task gives a
`stack` (a named one, or a `[task.stack]` table of layers), the `layer` the plane spans (its
shapes across, its thickness up, with the plane's margins), the shapes drawn on the stack
(`[[task.rect]]`, `[[task.circle]]`, `[[task.ring]]`, `[[task.path]]` and `[[task.shape]]`:
polygons, curves, masks, booleans, offsets, and a GDSII file's cell and layer), and `x_um`,
where the cut starts and ends. A
drawing has no ports, so the ends are where the task says: the cross-sections just outside
them are the ports' guides, and the shapes on the core layer must run on past both ends (the
check refuses a drawing that stops short). `exact = true` cuts at every x where the plane
changes, as `Cut::exactly`, for a drawing or a `[device]`; without it a sloped run is cut in
steps of `along_um`.

```yaml
task:
  kind: eme
  wavelength_um: 1.55
  modes: 480
  stack: soi_220
  layer: Si
  x_um: [0.0, 1.0]
  path:
    - layer: Si
      start: ["-1 um", 0]
      width: 400 nm
      segments:
        - { straight: 1 um }
        - { straight: 1 um, width: 800 nm }   # a linear taper
        - { straight: 1 um }
  plane:
    step_um: [0.05, 0.055]
    margins_um: [0.4, 0.33]
    symmetry: half
    exact: true
```

Two cases check that a drawing is cut as the library cuts it. `eme/3d-drawing-extruded` writes
the stepped slab of `eme/3d-cut-extruded` in a job as five rectangles on a `[task.stack]` of
the same constant indices: the 9 entries of S the run records (the guided modes at the two
ends) are the hand-built sections' within 4.7e-16. `eme/3d-drawing-taper` runs the built-in job
`jobs/eme-taper-drawn-3d.yaml`, the taper of `eme/3d-cut-taper-exact` drawn as a path (the YAML
above on that case's constant indices), against the taper's device drawing cut exactly: its 9
recorded entries are the device's to the bit, $\lvert S_{21}\rvert^2$ = 0.9921834. The path's
polygon has the drawing's walls and its straights the led guides, so the cut meets the same
sampling points at the same x and solves the same 33 planes (every mode of the half plane, 92 s
on six threads).

A layout's sections come from its GDSII file the same way: a `[[task.shape]]` of `type = "gds"`
draws what a cell draws on one of the file's layers (every cell it places flattened into it) on
the stack's layer the shape names, the file found from the job file's folder, and the cut reads
it as any other shape ([GDSII files](geometry.md#gdsii-files)).

```yaml
task:
  kind: eme
  wavelength_um: 1.55
  modes: 480
  stack: soi_220
  layer: Si
  x_um: [0.0, 1.0]
  shape:
    - type: gds
      layer: Si
      file: taper.gds
      cell: taper          # the file's one top cell if left out
      gds_layer: [1, 0]
  plane:
    step_um: [0.05, 0.055]
    margins_um: [0.4, 0.33]
    symmetry: half
    exact: true
```

`eme/3d-drawing-gds` writes `eme/3d-drawing-extruded`'s five strips byte by byte as boxes on
layer 1/0 of one cell (a 1 nm database unit) and runs that job with them read from the file in
place of the rectangles: the 9 entries the run records are the rectangles' to the bit, since the
file's whole nanometres are the rectangles' corners, so the cut turns at the same x and the cells
see the same strips.

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

**Could the estimate be tighter?** (#595.) The two structures were swept over four ranges each
(the taper over 1.45 to 1.65, 1.45 to 1.55, 1.55 to 1.65 and 1.50 to 1.60 µm; the multimode
section over 1.53 to 1.57, 1.54 to 1.56, 1.52 to 1.58 and 1.50 to 1.60 µm) at every tolerance from
1e-3 to 1e-10, 64 sweeps, each against the same expansion solved at 201 wavelengths of its range
(`the_estimators_ratio_study`, in release, about five minutes). The estimate over the true error
was 25 to 3.7e8, and never below 1. It is about twice the halves' own true error: each half's fit
was 12 to 1.8e8 times less accurate than the fit to all the solves, which is what the estimate
measures. The true error itself stopped at 1e-13 to 2e-11, the solves' own last digits, from
tolerances of 1e-4 on: a sweep's fit reaches the solves' floor long before its halves do, so the
ratio is largest at loose tolerances (3.7e8 for the multimode section asked for 1e-3) and smallest
at tight ones (25 at 1e-10). Two tighter estimates were measured on the same sweeps: the estimate
to the power 1.5 (as if the full fit's error were the halves' to the power 1.5), and an
extrapolation from the halves' and the quarters' fits, 2d_h²/d_q (the error falling geometrically
with the solves). Both fell below the true error once it reached the solves' floor: the first to
1.3e-4 of it, the second to 1.6e-8. No safety factor makes them bounds, since the floor isn't
the estimate's to see, so the estimate stays as it is: cautious by a factor that grows with how
far the solves are below the tolerance, a sweep stopping later than it needs, never earlier.
Over 1.50 to 1.60 µm the multimode section shows the cost: at its limit of 200 solves its
estimate was 3.9e-3 and its true error 1.2e-11.

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
up to its sign, which is the direction; its imaginary part the decay along z). The pairs are
taken closest first over all of them, $\lvert \ln\lambda_p + \ln\lambda_q \rvert$ (mod $2\pi i$)
smallest first. The $m$-th Bloch
mode is the one most made of the section's $m$-th mode, of those left: the fundamental first. A
Bloch mode that decays by $e^{-\mathop{\mathrm{Im}} K\Lambda}$ in one period has $\lambda$ to the
eigenproblem's round-off, an absolute one, so its $K$ is good to about $\varepsilon\,e^{\mathop{\mathrm{Im}} K\Lambda}/\Lambda$:
the guided, propagating and stop-band modes are at round-off, the strongly evanescent ones
approximate. Beyond about $e^{-36}$ in a period ($\varepsilon$) the eigenproblem loses $\lambda$
or $1/\lambda$ altogether (0, ∞ or a wrong magnitude), and the partner left behind pairs with
what is left. Paired in order of decay instead, one such partner took the guided mode's
$1/\lambda$, and the slab grating below between walls, 1.5 µm wide with 75 or 80 modes (its
highest decaying by $e^{-44}$ and $e^{-46}$ in a period), put an evanescent mode first
($K/k_0 = 3.50i$ with 80); closest first, the guided mode comes first at every count
(`eme/bloch-grating-lossless`).

**A guided Bloch mode is lossless.** Below the light line no diffraction order radiates, so the
guided Bloch mode should have $\mathop{\mathrm{Im}} K = 0$. Between walls, with no PML, it does at
any number of modes: each interface conserves power for any truncation (`eme/lossless-power`),
so a propagating Bloch mode keeps $\lvert\lambda\rvert = 1$. The slab grating below, half of it
1.5 µm wide between walls, has $\lvert\mathop{\mathrm{Im}} K/k_0\rvert$ under 5e-15 with 10, 20, 40
and 80 modes (`eme/bloch-grating-lossless`), and its $\mathop{\mathrm{Re}} K/k_0$ converges fast:
2.60348157, 2.60348322 and 2.60348352 with 50, 60 and 70 modes. With a PML the same mode has an
$\mathop{\mathrm{Im}} K$ of the expansion's, not the grating's: the PML absorbs the part of the
expansion's field that finitely many modes leave across the window. The Bloch mode's own field
is evanescent there, which the PML's complex stretch leaves as it is. Three measurements show
it:

| What changes | $\mathop{\mathrm{Im}} K/k_0$ of the guided Bloch mode | What it shows |
|---|---|---|
| a PML 4 µm thick, its strength 0.2, 0.1, 0.05, 0.025, 0.0125, 0.00625, 20 modes | 1.34e-4, 7.19e-5, 3.66e-5, 1.84e-5, 9.20e-6, 4.60e-6 | first order in the strength, as absorption is, and 0 without a PML (`eme/bloch-grating-pml-order`) |
| the same, 40 modes | 4.25e-4, 2.04e-4, 1.01e-4, 5.02e-5, 2.51e-5, 1.25e-5 | the same |
| the PML 1, 1.5, 2.5 and 3.5 µm from the slab's centre (strength 0.1, 40 modes) | 1.64e-4, 2.04e-4, 2.08e-4, 1.64e-4 | not the mode's own tail, whose power there falls by about $10^{-19}$ over those distances ($e^{-2q\thinspace\Delta x}$, $q = k_0 ((K/k_0)^2 - 1.444^2)^{1/2} = 8.8$/µm) |
| the number of modes, the PML 4 µm thick of strength 0.1: 10, 20, 40, 80, 160 | 2.5e-6, 7.2e-5, 2.0e-4, 6.8e-5, 4.8e-6 | the truncation's error, which is not monotone in the modes |

So the PML's $\mathop{\mathrm{Im}} K$ of a guided Bloch mode is an error of the expansion, up to
about $10^{-4}$ $k_0$ on this grating with up to 160 modes, and its $\mathop{\mathrm{Re}} K$ needs
more modes in the PML's wider window (2.60347 with 160, against 2.60348 between walls with 50).
A grating whose Bloch mode is guided is best solved between walls; a PML is for what radiates (a
grating above the light line, or the light a finite grating's ends scatter).

**N periods** (`Eme::grating`, `Eme3d::grating`): a structure's sections with some of them
repeated `count` times in place. The period's S-matrix is taken to the power `count` − 1 by
repeated squaring of the star product (about $2\log_2 N$ products instead of $N$), and starred
with what comes before the period and with the last period and what follows it. The last period
ends where the structure goes on, not at a zero-length copy of its first section: with finitely
many modes, a zero-length section between two others adds an interface pair that is not the
identity (S off by 4.4e-3 on the grating below, 100 periods with 20 modes).

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
| the slab grating above, 100 periods by squaring (11 star products) against its 200 sections in sequence (`eme/grating-squaring`) | every entry of S within 2.5e-14 | the same products in another order |
| the slab grating's guided Bloch mode between walls, 1.5 µm wide, 10, 20, 40 and 80 modes (`eme/bloch-grating-lossless`) | $\lvert\mathop{\mathrm{Im}} K/k_0\rvert$ under 5e-15, the guided mode first at each count | 0, below the light line |
| the same with a PML 4 µm thick, its strength 0.0125 and 0.00625, 20 and 40 modes (`eme/bloch-grating-pml-order`) | $\mathop{\mathrm{Im}} K$ at order 1.00 in the strength | first order: the PML's absorption of the expansion's field |

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

## Bends

A bend is a section whose cross-section is uniform along an arc. `Slice::bent(radius)` bends a
slice in the plane of the expansion, its radius R to the window's middle: positive with the
bend's centre beyond the lower wall (the upper wall outside), negative beyond the upper one.
`Section::bend(slice, radius, angle)` is a bend of that angle, its length the arc's at R.

**The map.** With ρ the distance from the bend's centre and φ the angle along it, $u = R\ln(\rho/R)$
and $v = R\varphi$ are conformal, and turn the 2D wave equation of the bend into a straight
guide's (Heiblum and Harris 1975): for TE (E along y)

$$\partial_u^2 E + \partial_v^2 E + k^2 n^2 e^{2u/R} E = 0,$$

a straight guide of index $n(u)\thinspace e^{u/R}$. For TM (H along y) the profile enters through
$1/n^2$ and the map leaves it there, $\partial_u (n^{-2} \partial_u H) + \partial_v (n^{-2} \partial_v H) + k^2 e^{2u/R} H = 0$:
a layer's weight in $P = (w/k)\thinspace dU/du$ stays $1/n^2$ while its $\alpha^2 = n^2 e^{2u/R} - n_\text{eff}^2$.
A mode goes as $e^{i\beta v} = e^{i\beta R\varphi}$, so its $n_\text{eff} = \beta/k$ is the
propagation constant along the arc at R, and $\nu = n_\text{eff} k R$ the angular order that
`mode::bend` solves for exactly.

**Thin layers.** The mapped profile is graded, so the bent section's modes are those of a stack
of thin layers, each of the map's index at its middle: the midpoint rule's staircase, second
order in the layers' thickness. Each region (each of the slice's layers, each PML) is cut into a
power of two of them, at most `Eme::with_bend_step` thick in u (10 nm by default), so that
halving the step doubles every count: the staircases nest, and Richardson's limit of two of them
is good to the next order. With counts that only roughly doubled, the limit was off by 1e-8.

**The PML** keeps its physical thickness: in u its thickness is the physical one over $\rho/R$ at
the window's edge, and the profile goes on into it as its analytic continuation,
$n^2 e^{2\tilde u/R}$ along the complex coordinate $\tilde u$. The light the bend radiates keeps
meeting the profile it would have met. Two other PMLs were tried on TE at 2 µm, whose loss is
3.1e-7 dB per 90°: one of the edge's index, uniform, put the loss 10 % off (the reflection from the
change in the profile's slope), and one graded along the real coordinate 4 %; the continuation
is within 6e-7 of the exact loss. A thick graded PML in a tight bend reaches a high index at its
far end, where it holds modes of its own: TM at 2 µm lost two of them on the way to the bend
with a PML 4 µm thick (2 µm was followed); the closed window's path below follows them.

**Its modes** are followed in two stages: from the uniform window to the thin layers straight
(each of its physical index), as any section's; then the map's exponent $2\tilde u/R$ grows from 0
to its own. Where two of them meet on the way, the bend's modes are followed from the uniform
window directly instead; where they meet on that path too, from the closed window's modes (no
PML, no loss: real, and counted exactly) as the PML's strength grows from 0 to its own, a path on
which the guided modes hardly move. That third path is what follows `jobs/eme-bend.yaml`'s TM bend
at 2 µm with a PML of strength 0.5 or 1 (16 or 24 modes), where the first two lost two modes; at
24 modes the three strengths' $\lvert S_{21}\rvert^2$ agree within 1.5e-5 at 1.5, 1.55 and 1.6 µm
(a test, `the_bend_job_follows_its_modes_at_every_pml_strength`). Two things found here hold for every section: a mode that lives at the
PML's far end was stitched from the two walls' sweeps where one of them had followed a decaying
field, so that it was 0.4 off orthogonal to the others; the sweeps are now joined only where they
agree. And Newton's method now meets the sweeps where they meet best at the root, not at the guess.

**At a joint** both sections' tangential fields are functions of the physical x across the
joint's plane. A bent mode's are its mapped ones, but for the one that came from a derivative
along the arc (TE's $H_x$, TM's $E_x$), which is $R/\rho$ times the mapped one, and
$dx = (\rho/R)\thinspace du$. The overlaps between a straight and a bend are taken in x with those
factors, exactly (the plan's $u = x$ holds only to first order in x/R); between two sections of one
bend they are the mapped ones, so a bend's modes stay orthonormal and two bends of one radius in
sequence are one.

```rust
use photonoxide::eme::{Eme, Section, Slice};
use photonoxide::mode::Polarization;
use photonoxide::units::{Length, Wavelength};
use photonoxide::Complex64 as c64;

// a 220 nm silicon slab, 1.39 µm of oxide inside it and 2.89 µm outside, to a PML outside
let (si, sio2) = (c64::new(3.473, 0.0), c64::new(1.444, 0.0));
let slab = Slice::new(&[(sio2, Length::um(1.39)), (si, Length::um(0.22)), (sio2, Length::um(2.89))])?;
let eme = Eme::new(Polarization::Tm, Wavelength::um(1.55)?, 12)?.with_upper_pml(Length::um(2.0), 0.25)?;
// the core's centre 2 µm from the bend's: 2.75 µm to the window's middle
let quarter = std::f64::consts::FRAC_PI_2;
let s = eme.s_matrix(&[
    Section::new(slab.clone(), Length::ZERO),
    Section::bend(slab.clone(), Length::um(2.75), quarter)?,
    Section::new(slab, Length::ZERO),
])?;
let through = s.s21[0][0].norm_sqr();               // the bend's radiation and its two joints
# let _ = through;
# Ok::<(), photonoxide::Error>(())
```

Checked against `mode::bend`'s exact bent slab (radial shooting matched to the outgoing Hankel
function, Marcuse's Eq. 10 without his approximation inside): a 220 nm silicon slab in oxide at
1.55 µm, the window 2.5 µm outside it to a PML 2 µm thick of strength 0.5, and half the radius
(at most 2.5 µm) inside.

| Check | Ours | Exact |
|---|---|---|
| $n_\text{eff}$ at 2, 5, 10 and 20 µm, TE and TM, Richardson's limit of 10 and 5 nm layers (`eme/bend-index`) | within 3.4e-10 (our own error 2.2e-10: the limit's next halving and a PML twice as strong; the exact's own 1.9e-11) | `mode::bend`, its Runge–Kutta steps a quarter of its own |
| the loss where it is above the arithmetic's reach: TE at 2 µm, 3.1e-7 dB per 90°; TM at 2, 3 and 5 µm, 0.19, 0.017 and 8.0e-5 dB per 90° (`eme/bend-loss`) | within 2.7e-5 relative (TM at 5 µm, where a PML twice as strong moves it by as much) | the same |
| the layers' order at 2 µm, 10 nm and 2, 4 and 8 times as many (`eme/bend-order`) | 2.00 (TE 2.0001, 2.0000, 2.0000; TM 2.0007, 2.0009, 2.0032) | second order |
| a 180° turn as two 90° bends joined by their overlaps, against one 180° bend, 12 modes (`eme/bend-halves`) | every entry of S within 5.1e-15 | the same cascade |
| a bend of no angle between two straights at 2 and 3 µm, TE and TM, 24 modes (`eme/bend-no-angle`) | $\max(\lvert S_{21} - 1\rvert, \lvert S_{11}\rvert)$ 2.4e-5; at 2 µm TE with 6, 12, 16 and 24 modes 3.2e-3, 2.5e-3, 1.3e-4 and 3.5e-6 | the straight's S |
| the straight-to-bend joint's coupling η between the fundamental modes, TE at 2, 3 and 5 µm and TM at 10 µm (`eme/bend-joint`) | within 1.1e-9 ($1 - \eta$ is 2.8e-3 at 2 µm TE) | the exact bent field's overlap with the straight slab's mode, across the window and the PML |
| the same where the bend radiates, TM at 2, 3 and 5 µm, $\mathop{\mathrm{Im}} n_\text{eff}$ 1.3e-3, 8.5e-5 and 2.9e-7 (`eme/bend-joint-leaky`) | within 1.2e-9 (own error 6.9e-10, TM at 2 µm; $1 - \eta$ is 3.1e-2 there) | the same |

A bend of no angle leaves its two joints, whose overlaps are inverse only with every mode: what
remains is the truncation of two bases, as a zero-length section between two others in
[periodic sections](#periodic-sections). It is a property of finitely many modes, not an error
of the bend: the two bases differ (the bend's mapped profile is graded, the straight's is not),
so the overlap matrices of the two joints are inverse only in the limit, and no count short of
every mode makes the gap zero. It falls with the modes, though not monotonically, at 2 µm:

| Modes | 6 | 12 | 16 | 24 |
|---|---|---|---|---|
| TE, $\max(\lvert S_{21} - 1\rvert, \lvert S_{11}\rvert)$ | 3.2e-3 | 2.5e-3 | 1.3e-4 | 3.5e-6 |
| TM | 3.1e-2 | 2.5e-2 | 2.0e-4 | 2.4e-5 |

The gap is stated at 24 modes, where `eme/bend-no-angle` holds it to the mode-count gate of
`eme/taper-modes` (1e-4) at 2 and 3 µm, TE and TM; below 16 modes it is 1e-3 to 3e-2, the size of
a weak bend's own loss, so a bend's S is read with at least 24 modes, or with `[task.convergence]`,
which doubles them until S stops changing. Why the fall stalls from 6 to 12 modes and then drops
by an order of magnitude or more we have not found.

The exact bend had an error of its own at first: its inner solution started 6 decay lengths below
the slab with an approximate $\psi'/\psi$, which adds the solution that decays outwards; by the
slab that is down by $e^{-12}$, and it left $n_\text{eff}$ about 5e-9 off. It now starts 12 decay
lengths deep. The joint's check first integrated the exact field across the window only, which
left out what a radiating mode puts into the PML (TM at 5 µm, $\mathop{\mathrm{Im}} n_\text{eff}$
2.9e-7, was 7.6e-7 off). The exact bend is now closed as the expansion closes it: the inner
wall, and the PML as the outer cladding's analytic continuation along the same complex mapped
coordinate, $\tilde u = u_1 + (1 + i\sigma)(x - x_1)/(dx/du)_1$ from the window's edge, then its
wall; its field is integrated from that wall inwards, the direction in which the outgoing wave
grows, so that the integration is stable (integrated outwards, the growing solution that
round-off seeds swamped it, 2.5e-6 off at 2 µm TM with a PML of strength 1). Both walls matter
at 2 µm TM: the field at the inner wall is 7.5e-6 of its peak, and with the open bend's
solution there the coupling was 3.1e-8 off. Closed, every bend agrees within 1.2e-9 at either
PML strength, 0.5 or 1.

**In a job,** a `[[task.section]]` with `radius_um` and `angle_deg` instead of `length_um` is a
bend, its radius to the core's centre (to the window's middle for layers), positive turning
towards the lower wall. Its length is the arc at the window's middle. The ports are straights, a
bend is a core of one thickness or layers, and only 2D jobs that aren't symmetric take one; the
scene draws it turning. `jobs/eme-bend.yaml` is the TM bend above, swept from 1.5 to 1.6 µm:
$\lvert S_{21}\rvert^2$ is 0.8991 at 1.55 µm (0.46 dB, 0.19 dB of it the bend's radiation).

```yaml
  section:
    - { name: input, length_um: 0.0, core_um: 0.22, center_um: -0.75 }
    - { name: bend, radius_um: 2.0, angle_deg: 90.0, core_um: 0.22, center_um: -0.75 }
    - { name: output, length_um: 0.0, core_um: 0.22, center_um: -0.75 }
```

## A beam

A beam comes into a structure from outside it: from a fibre, a lens or another chip. Here it is
a Gaussian across the first section's plane, `Beam::gaussian(waist, centre)`, its phase fronts
tilted by `Beam::tilted(θ, n)`:

$$f(x) = e^{-(x - c)^2/w^2}\thinspace e^{i k n \sin\theta\thinspace(x - c)},$$

$E_y$ for TE and $H_y$ for TM. `Eme::launch` writes it as the first section's forward modes,
the amplitudes that make their transverse field on the plane the beam's: for TE
$a_m = \langle E_\text{in}, H_m\rangle = \int f\thinspace h_m\thinspace d\tilde x$, for TM
$a_m = \langle E_m, H_\text{in}\rangle = \int e_m f\thinspace d\tilde x$, exact by the modes'
unconjugated orthogonality ([normalization](#normalization-and-the-interfaces)), Gauss–Legendre
sums over the beam's nine waists either side of its centre. `Launched::arriving` keeps the
propagating modes alone (a beam that comes from afar carries no evanescent waves), and
`Scattering::forward` carries them through: $S_{11}a$ back, $S_{21}a$ on.

**The modes are a basis of plane waves.** In a uniform section between walls the modes are
standing plane waves, $k_x = m\pi/W$, so a beam's expansion samples its angular spectrum, a
Gaussian $A(k_x) = e^{-(k_x - k n \sin\theta)^2 w^2/4}$ centred on its tilt, and each mode
meets an interface between uniform media at its own angle with Fresnel's coefficient
(`eme/step-fresnel-te`). A tilted beam is therefore the plane-wave input the method lacked: the
power a step or a film returns is $\int \lvert A\rvert^2 \lvert r\rvert^2 k_z\thinspace dk_x /
\int \lvert A\rvert^2 k_z\thinspace dk_x$ over the propagating waves, which the discrete sum
reaches as fast as the beam's images in the walls fade ($e^{-89}$ for a 3 µm waist in a 40 µm
window). Measured (`eme/beam-fresnel`, `eme/beam-film`): a 3 µm beam in oxide meeting a uniform
2.0, or crossing a 0.3 µm film of it, at 0° to 30°, centred and 2 µm off, TE and TM, returns and
passes the spectral integral's shares to $2.2 \times 10^{-14}$, and $R + T = 1$ to the same.

**A beam's overlap with a guide's mode** is `Launched::couplings`:
$\lvert\int f U_m w\thinspace dx\rvert^2 / (\int \lvert f\rvert^2 w\thinspace dx \int U_m^2 w\thinspace dx)$,
$w = 1$ for TE and $1/n^2$ for TM, the share of the beam's power the mode takes where their
impedances match, which from the amplitudes is $\lvert a_m\rvert^2 / (\mathop{\mathrm{Re}} n_\text{eff}
\int \lvert f\rvert^2 w\thinspace dx)$. Against the exact slab's mode integrated across the whole
line (`eme/beam-slab-coupling`: a 220 nm silicon slab, waists 0.3 to 2 µm, centred and 0.5 µm
off, at 0° and 10°) it agrees to $3.2 \times 10^{-14}$ relative. With the facet's index step
the expansion gives more than the overlap: the reflection and the light that couples into the
guide's other modes and the radiation.

In a job, `[task.beam]` (`waist_um`, `center_um`, `angle_deg`, and `index`, the medium of the
tilt, the cladding's by default) launches the beam on the first section at each wavelength and
records the shares of its power reflected, passed and carried by each guided mode at the last
end (`eme_beam`). Its power is its propagating modes' $\sum \lvert a_m\rvert^2$, exact between
walls without a PML, so a beam's job has neither; it is a 2D job's. `jobs/eme-beam.yaml` meets
a 220 nm silicon slab's facet with a 1 µm beam in oxide: TE0 takes 0.3474 of its power and
0.0357 comes back.

```yaml
task:
  kind: eme
  wavelength_um: 1.55
  modes: 60                  # every propagating mode of the oxide (37) and more
  window_um: 20.0            # between walls, no PML
  cladding: 1.444
  core: 3.473
  section:
    - { name: oxide, length_um: 0.0, layers: [{ medium: 1.444, thickness_um: 20.0 }] }
    - { name: slab, length_um: 0.0, core_um: 0.22 }
  beam: { waist_um: 1.0, center_um: 0.0, angle_deg: 0.0 }
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
| `eme/3d-drawing-extruded` | the stepped slab of `eme/3d-cut-extruded` drawn in a job as five rectangles ([a job's own drawing](#a-jobs-own-drawing)) | every entry the run records within 4.7e-16 of the hand-built sections' | the same planes | round-off, 1e-10 |
| `eme/3d-drawing-taper` | `jobs/eme-taper-drawn-3d.yaml`: the taper of `eme/3d-cut-taper-exact` drawn as a path in the job, cut exactly | every entry the run records the device's exact cut's to the bit | the same 33 planes | round-off, 1e-10 |
| `eme/3d-drawing-gds` | `eme/3d-drawing-extruded`'s strips read from a GDSII file (boxes on layer 1/0, 1 nm database unit) by a `type = "gds"` shape | every entry the run records the rectangles' run's to the bit | the same planes | round-off, 1e-10 |
| `eme/wavelength-taper`, `-mmi` | S over a range (1.45 to 1.65 µm, the C band) from a few solves, every entry on one set of wavelengths, asked for 1e-6 ([S over wavelength](#s-over-wavelength)) | ratio of the true error to the estimate 7.5e-7 (taper, 32 solves) and 7.8e-6 (multimode section, 117 solves) | the same expansion at each of 201 wavelengths | discretisation, a ratio of the true error to the estimate of at most 1, measured |
| `eme/wavelength-reciprocity` | the two interpolants on the 201 wavelengths | $\lvert r - r^T \rvert$ 1.8e-14 | the solves' own reciprocity, kept by a shared denominator | round-off, 1e-12 |
| `eme/wavelength-power` | the same: each column's power against the solves' | under 1e-3 of the bound $\sum_q (2\lvert S_{qp}\rvert + e_{qp}) e_{qp}$ | the bound the entries' estimates give | discretisation, a ratio of at most 1, measured |
| `eme/bloch-quarter-wave`, `-modes` | a quarter-wave stack (1.444 and 2.0 at 1.55 µm), $\cos K\Lambda$ of its Bloch modes ([periodic sections](#periodic-sections)): one mode at 61 wavelengths from 1 to 2.5 µm; 8 modes at 1.3, 1.55 and 1.8 µm | within 6.7e-16 and 8.1e-15 | the two layers' closed form, each mode its own | round-off, 1e-12 |
| `eme/bloch-band-edges` | the same stack's first stop band, its edges by bisection on $\cos K\Lambda = -1$ | $\Delta\omega/\omega_0$ = 0.20645554758478, relative 2.0e-15 | $(4/\pi)\arcsin((n_2 - n_1)/(n_2 + n_1))$ | round-off, 1e-12 relative |
| `eme/grating-squaring` | a silicon slab grating, 100 periods (20 modes, a PML), by repeated squaring | every entry of S within 2.5e-14 of the 200 sections in sequence | the same star products in another order | round-off, 1e-12 |
| `eme/bloch-grating-lossless` | the same grating's guided Bloch mode between walls, 10 to 80 modes | $\lvert\mathop{\mathrm{Im}} K/k_0\rvert$ under 5e-15 | 0 below the light line: power conserved at each interface | round-off, 1e-12 |
| `eme/bloch-grating-pml-order` | the same with a PML of strength 0.0125 and 0.00625, 20 and 40 modes | order 1.00 of $\mathop{\mathrm{Im}} K$ in the strength | a weak PML's first-order absorption | order, 1 within 0.1 |
| `eme/bend-index` | a 220 nm silicon slab bent at 2, 5, 10 and 20 µm, TE and TM ([bends](#bends)), Richardson's limit of 10 and 5 nm layers | $n_\text{eff}$ within 3.4e-10 (own error 2.2e-10) | `mode::bend`'s exact bent slab | discretisation, 7e-10 absolute, three times our own error |
| `eme/bend-loss` | the same bends' loss where it is above the arithmetic's reach (TE at 2 µm, TM at 2, 3, 5 µm) | within 2.7e-5 relative | the same | discretisation, 9e-5 relative, three times our own error |
| `eme/bend-order` | the same at 2 µm, 10 nm layers and 2, 4 and 8 times as many | order 2.00 | the midpoint staircase's second order | order, 2 within 0.1 |
| `eme/bend-halves` | two 90° bends against one 180° bend, TE and TM, 12 modes | every entry of S within 5.1e-15 | the same cascade | round-off, 1e-12 |
| `eme/bend-no-angle` | a bend of no angle between two straights, 24 modes | 2.4e-5 | the straight's S | discretisation, 1e-4, the mode-count gate of `eme/taper-modes` |
| `eme/bend-joint` | the straight-to-bend joint's coupling, TE at 2, 3, 5 µm and TM at 10 µm | within 1.1e-9 (own error 1.3e-9) | the exact bent field's overlap with the straight's mode, window and PML | discretisation, 4e-9 absolute, three times our own error |
| `eme/bend-joint-leaky` | the same joint where the bend radiates, TM at 2, 3, 5 µm | within 1.2e-9 (own error 6.9e-10) | the same | discretisation, 3e-9 absolute, three times our own error |
| `eme/beam-fresnel` | a Gaussian (waist 3 µm) in oxide meeting a uniform 2.0, 110 modes in a 40 µm window, at 0°, 10°, 20°, 30°, centred and 2 µm off, TE and TM ([a beam](#a-beam)): its reflected share | within 2.2e-14 (and $R + T - 1$) | Fresnel's $r_s$, $r_p$ over its angular spectrum | round-off, 1e-12 |
| `eme/beam-film` | the same through a 0.3 µm film of 2.0 in oxide: its transmitted share | within 3.2e-15 | Airy's transmission over the angular spectrum | round-off, 1e-12 |
| `eme/beam-slab-coupling` | a Gaussian's overlap with a 220 nm silicon slab's fundamental mode, waists 0.3, 1, 2 µm, centred and 0.5 µm off, 0° and 10°, TE and TM | within 3.2e-14 relative | the exact slab's mode integrated across the line | round-off, 1e-12 relative |

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
- **Sections are described or cut from a geometry:** a job gives straights and tapers of a core
  or of layers in 2D, or strips on a plane in 3D, or cuts a `[device]` or its own drawing into
  planes ([a device's sections](#a-devices-sections), [a job's own drawing](#a-jobs-own-drawing)),
  and Python and the studio take what a run records ([from Python](#from-python),
  [in the studio](#in-the-studio)), and a drawing may be read from a GDSII file's cell by a
  `type = "gds"` shape (`eme/3d-drawing-gds`). A
  device or a drawing is cut along x
  only, its ports facing ±x; a sloped etch's walls, which lean with z, are cut in steps, not
  exactly; the cut takes no bodies over the stack, and a drawing takes no `[[task.component]]`
  (a component's ports face any way, the cut's two ends only ±x).
- **A device's size:** every mode of a plane is a dense solve, at most a few thousand unknowns
  in reasonable time, and the 3D FDFD that checks it millions of unknowns for a device tens of
  micrometres long. The cases check a 1.6 µm MMI and a 1 µm taper; the studio's 6 µm MMI is not
  a case.
- **Bends are 2D only,** in the plane of the expansion: the map is exact only there, and a 3D
  bend's modes need a bent-plane solver of their own ([the bends' plan](../plans/eme-bends.md)).
  A bend's modes are followed through two stages, then from the uniform window directly, then
  from the closed window as the PML grows; where two modes meet on all three paths the error
  names the mode, never a wrong one (`jobs/eme-bend.yaml`'s TM bend at 2 µm is followed with a
  PML of strength 0.25, 0.5 and 1).
  A bend's loss in the job is that of its window: a mode radiating strongly puts power into the
  PML that a narrower window would cut (the window must also stay on one side of the bend's
  centre). The bend's mapped profile needs its own convergence beside the modes': the default
  10 nm layers leave $n_\text{eff}$ off by 2.7e-6 (TE) and 2.1e-6 (TM) at 2 µm, the staircase's
  second-order error (`eme/bend-order`). The default is not a number chosen to be small enough
  for every job: it is where `[task.convergence]` starts, and the job's own tolerance decides
  how far the layers are halved (the rule every grid here follows, `convergence::ToTolerance`);
  a job without `[task.convergence]` takes `bend_step_nm` as given and its error with it.
- **A grating's Bloch modes are the expansion's,** with its modes' truncation error. With a
  PML, a guided Bloch mode below the light line has an $\mathop{\mathrm{Im}} K$ that is that error,
  not a loss of the grating ([a guided Bloch mode is lossless](#periodic-sections)): the slab
  grating's at 1.55 µm with a PML 4 µm thick of strength 0.1 is 2.5e-6, 7.2e-5, 2.0e-4, 6.8e-5
  and 4.8e-6 $k_0$ with 10, 20, 40, 80 and 160 modes, not monotone, and it grows with the PML's
  strength (4.3e-4 $k_0$ at 0.2 with 40 modes). It is the PML absorbing the part of the field
  that finitely many modes leave across the window: first order in the PML's strength
  (`eme/bloch-grating-pml-order`), the same with the PML 1 or 3.5 µm from the slab, and 0 to
  round-off between walls at any number of modes (`eme/bloch-grating-lossless`). A loss of that
  size is not resolved with a PML; between walls a guided Bloch mode is lossless to round-off.
  The job's stop band (an $\mathop{\mathrm{Im}} K/k_0$ above 0.01) is far above it.
- **S over wavelength** is interpolated in the wavelength only: a structure's own parameters
  (a width, a length) are swept by solving each value. Its estimate holds where the fit has seen
  the response, and needs one discretisation for every solve and a PML window
  ([above](#s-over-wavelength)).
- **A beam is 2D, a Gaussian, between walls** ([a beam](#a-beam)): its power is its
  propagating modes', exact without a PML, so a `[task.beam]` takes none, and the window must
  hold the beam's waists and keep every propagating mode. A fibre's mode is a Gaussian here, as
  its usual approximation is; its exact LP₀₁ and a beam on a 3D section's plane (the crossings
  lesson's inverse-taper tip against a lensed fibre, a full-vector mode's overlap) are not
  launched yet: a 3D beam's H on Yee's plane needs its own projection, since E alone doesn't
  give the vector modes' amplitudes. **Angled facets** are not sections: a facet tilted against
  the axis is an interface that leans across the window, which sections normal to z only
  staircase. A tilted beam on a normal interface is the same problem only for uniform media on
  both sides (a guide's facet is not), so an angled facet's reflection back into a guide waits for
  a staircase with an error bound of its own, or for a rotated frame. Measured on the simplest
  one (`study_an_angled_facet` in the beam's cases, ignored): `eme/beam-fresnel`'s 3 µm beam square
  on, TE, meeting 2.0 across a facet tilted by 10° in the same 40 µm window, the facet as 50 to
  800 sections, against Fresnel's $r_s$ over the beam's angular spectrum in its own frame, each
  plane wave meeting the facet at its own angle plus 10° (R = 0.0273777). With 110 modes, every
  propagating one and few more, R stalls 1.3e-3 to 3.5e-3 below. With 220 the gap halves with each
  doubling of the steps, 2.0e-3, 6.0e-4, 3.1e-4 and 1.7e-4 at 100 to 800: first order, as a
  staircase across the window is (the corners are a grating whose depth goes with the step), and
  the first-order limit of 400 and 800 steps is 2.4e-5 below. The modes' own error, measured at
  the same steps (R less the closed form):

  | Modes | 200 steps | 400 steps | 800 steps |
  |---|---|---|---|
  | 110 | −3.47e-3 | −2.34e-3 | |
  | 160 | −1.07e-3 | −7.40e-4 | |
  | 220 | −5.97e-4 | −3.13e-4 | −1.69e-4 |
  | 320 | −5.09e-4 | −1.63e-4 | −7.00e-5 |

  The two errors don't separate. From 220 to 320 modes R moves by 8.7e-5, 1.5e-4 and 9.9e-5 at
  200, 400 and 800 steps, as much as a halving of the steps moves it, so the modes still matter
  at every step tried: a finer staircase has smaller corners, whose scattering takes modes of
  higher order to hold. With 320 modes the halvings shrink R's gap by 3.1 and then 2.3 times,
  not the factor of 2 of a first-order error, and the first-order limit of 400 and 800 steps is
  2.3e-5 above the closed form, where 220 modes' is 2.4e-5 below. So the closed form is reached
  to about 2e-5, but no rule gives the error of one staircase from its own refinements: neither
  the steps' halvings nor the modes' added count is in its asymptotic range yet. That makes it a
  study, not a case. The largest run, 800 steps of 320 modes, takes 506 s on six threads
  (`PHOX_FACET_MODES` and `PHOX_FACET_STEPS` choose the counts the study runs).
- **FDTD on tapers is not a case.** The tapers are checked against FDFD on the same staircase
  (`eme/taper-fdfd`, `eme/3d-taper-fdfd`), which solves the same frequency-domain problem the
  expansion does; FDTD would add its time stepping's dispersion, which a stated error needs
  driven out by a grid study. The 2D taper's reflection, 9.2e-5, needs FDTD's to about 1e-6 for
  a Rule of the kind the FDFD case has: cells of 2.5 nm over a window several micrometres wide
  and long, millions of cells for tens of thousands of steps at each of three grids, beyond a
  validation case's time.
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

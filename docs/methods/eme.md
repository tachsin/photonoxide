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

The FDFD comparison is the most telling: the two methods share nothing but the structure, and
agree to 1.9e-5 of a reflection of 9.2e-5. Its transmission agrees too: 0.999829 against
FDFD's 0.999831. It needs a thick PML. With 1 µm of PML of strength 0.025 to 0.1 the
expansion's reflection converges, as the modes increase, to between 1.1e-3 and 6e-4 below FDFD's:
light leaving the guide at a grazing angle travels a long way along a thin, weak PML and comes
back. FDFD's own PML (0.2 or 0.4 µm) changes its result by under 1e-12.

## Limits

What it does not do yet (#432):

- **2D only:** slab stacks. Cross-sections from the [full-vector solver](vector.md) for 3D
  guides are next.
- **Sections are described, not drawn:** a job gives straights and tapers of a core or of
  layers; sections cut from a device's geometry or a job's drawn shapes, a façade function of
  its own and a view of its own in the studio (which shows its scene and S-parameters as any
  job's) are to come. The Python package runs it as any job, through `run_job`.
- **Bends, periodic sections** and S over wavelength interpolated with a stated error (a
  `[task.sweep]` solves each wavelength) are to come.
- **No published taper or MMI yet:** no paper we have gives a taper's or an MMI's S by
  eigenmode expansion with the numbers needed to repeat it (Bienstman and Baets's grating sends
  its material data to another exercise; Soldano and Pennings's self-imaging gives closed forms,
  not a device's indices). The taper is checked against FDFD and against itself in steps and
  modes.
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
  not a case yet.

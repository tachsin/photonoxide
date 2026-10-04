---
title: "FDFD ports and S-parameters"
module: fdfd
summary: "Waveguide modes into and out of a 2D FDFD problem: the grid's own port modes, one-way total-field/scattered-field sources, mode amplitudes, and a reciprocal S-matrix."
order: 16
papers:
  - cite: "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012)"
    doi: 10.2528/PIERB11092006
  - cite: "A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010) (the S-parameter conventions)"
    doi: 10.1016/j.cpc.2009.11.008
validation:
  - fdfd/port-mode-te
  - fdfd/port-mode-tm
  - fdfd/straight-guide
  - fdfd/reciprocity
  - fdfd/step-reflection-te
---

A device is described by what it does to the modes of its waveguides: how much of a mode going
in comes out in each mode, and with what phase. These are its S-parameters. A port is a column
of the [FDFD](fdfd.md) grid that crosses a waveguide running along x, with that guide's modes.

## Port modes

A port's modes are the scheme's own. The 2D operator, restricted to the port's column, gives a
1D eigenproblem along y, and the grid's difference along x turns β into
$\beta_d = (2/\Delta x)\sin(\beta\Delta x/2)$:

$$
L_y u + k_0^2 \varepsilon_z u = \beta_d^2 u \quad (E_z), \qquad
\varepsilon_y\thinspace(L_y + k_0^2)\thinspace u = \beta_d^2 u \quad (H_z),
$$

with $L_y$ the column's y-part of the operator, PML included. It is solved by shift-and-invert
Arnoldi near the column's highest index. A mode solved this way propagates along a straight grid
waveguide exactly. Launched, it neither reflects nor sheds radiation, so a port measures only
the device. A mode taken from a continuum solver would leave a small mismatch on every grid.

**Several guides side by side** each get their own port with
`Solver2d::port_modes_within(column, rows, count)`: the mode is solved on a window of the
column, with walls at its ends, and the projection sees only the window. The guide's field must
have decayed at the window's ends. Two silicon slabs 1.6 µm apart, each windowed, carry their
modes through with |S| = 1 and couple 2.5e-6 across the gap. The windows' walls pull β by 1.2e-5,
and S stays symmetric to 3e-12.

## Sources

A port launches its mode by total-field/scattered-field (Rumpf's Eq. 55). Q masks the cells of
the scattered field, f is the mode extended along x as $e^{\pm i\beta x}$ through the whole grid,
and A is the system's matrix:

$$
b = (QA - AQ)\thinspace f .
$$

Only the rows at the interface between the two regions are non-zero. The mode then travels one
way only, from the interface into the total field, at unit amplitude on the port's column. The
scattered field holds only what the device sends back.

## Amplitudes and the S-matrix

Two neighbouring columns c and c + 1 separate the forward and backward waves of a mode. The field
on each column is projected onto the mode with the operator's own orthogonality,
$\sum_j w_j \phi_m \phi_n = 0$ for m ≠ n. Here $w = s_y$ for E along z and $s_y/\varepsilon_y$ for H along z,
the weights that make the column's operator symmetric. With each mode normalized to
$\sum w\phi^2 = 1$ (real for a lossless guide), the two projections
$p_c = a + b$ and $p_{c+1} = a e^{i\beta\Delta x} + b e^{-i\beta\Delta x}$ give the forward and backward
amplitudes a and b.

`Solver2d::s_matrix` runs one solve per port. In every run it measures the incoming and the
outgoing amplitude at every port, and solves S A = B, with a column of A and of B per run. A
trace of a mode that the PMLs send back into a port is then measured as incoming, not mistaken
for part of S. The amplitudes are power-normalized: $|S_{qp}|^2$ is the share of power from mode p
into mode q. Each mode is normalized by its unconjugated Lorentz form, which for
$\sum w\phi^2 = 1$ is $\sin(\beta\Delta x)\thinspace\Delta y/(2k_0\Delta x)$. That is the mode's power when the mode is
real, and it keeps S exactly symmetric for a reciprocal device. Where a mode's tail reaches into
a PML, the physical power differs from it, by about the share of the mode in the PML.

A mode's backward twin is the one with the same tangential E, the usual convention of mode
expansions, which the [3D ports](fdfd-3d.md) follow too: E_z itself with E along z, and −H_z with
H along z, since H_z reverses with the tangential H. So $S_{11}$ and $S_{22}$ are the tangential
E's reflections with either polarization, and the 2D and 3D solvers give the same S-matrix for a
structure uniform along z, signs included. (Before 0.4.0, the twin with H along z had the same
H_z, and those reflections were minus these.)

## Validation

**Port modes against the exact slab:** 220 nm of silicon (3.476) in oxide (1.444) at 1.55 µm.

| | 10 nm | 5 nm | 2.5 nm |
|---|---|---|---|
| E along z (n_eff = 2.84778 exact) | 2.6e-3 | 6.5e-4 | 1.6e-4 |
| H along z (n_eff = 2.05332 exact) | 2.5e-3 | 6.1e-4 | 1.5e-4 |

The error falls exactly 4× per halving: second order, as the 2D scheme.

**A straight guide:** two ports 1.4 µm apart on a straight slab. S11 and S22 are 0, and S21 and
S12 are $e^{i\beta L}$, all to 1e-13 for both polarizations: the mode crosses the grid whole.

**Reciprocity:** a slab stepping from 220 to 300 nm thick. S21 and S12 agree to 1e-14 for both
polarizations.

**The step's reflection, E along z:** 1.16424e-3, against 1.16503e-3 from Fresnel's formula on
the two modes' effective indices (2.84742 and 3.04866). For a TE slab mode the modal impedance
is the effective index, and this estimate is good to 0.07 %. The step radiates 1.9e-4 and
transmits the rest, 0.99865. With H along z the TM mode is more weakly confined: the step
reflects 2.1e-3 and radiates 6 %. There, Fresnel on the effective indices isn't the right
estimate, since a TM mode's impedance isn't its index.

## In the studio

A job of kind `"fdfd"` runs a device on one layer, seen from above, with each point's
permittivity its slab's effective index squared. It records the S-parameters at each wavelength
of a sweep, and the field from the first port: in full at the wavelength the job asks for
(`field_um`), and at every point of a sweep averaged over 2 × 2 cells (or larger blocks, for a
sweep long enough that its pictures would pass five million pixels in all), each scaled to its
own peak. The run's pictures, the scene and the fields, cover the window without its PMLs and
the two cells beside them: the PMLs absorb the light and aren't part of the device, so a guide
drawn into them would look as if the light started late and ended early. The field is launched
from the window's end of port 1's guide, two cells inside the PML's edge, the pictures' first
column, when the guide there is the port's (the same mode index), so the picture shows the
wave along the whole guide; S stays referred to the ports' own columns. The 3D view draws the field on the layer's top face; the 2D view has the field, the
S-matrix and |S_q1|² against wavelength. Both follow a running sweep, showing each wavelength's
field as it is solved. The run also records, at each wavelength, the field's linear residual
$\lVert b - A u\rVert / \lVert b\rVert$ (`Solver2d::residual`) and how far the S-matrix is from
reciprocal, $\max |S_{qp} - S_{pq}|$; the viewer's Solver panel charts them. `jobs/mmi-fdfd.toml`
is a 1×2 splitter. These are 2D estimates by the effective index method, not a device's 3D
performance.

## Limits

- One mode per port is the common case. Several modes per port work the same way, with each a
  port of its own in `s_matrix`.
- A port's column and the next must be at least two cells clear of the PMLs along x, with the
  same guide on both.
- The projection separates the modes exactly, radiation included. Radiation itself isn't
  reported as a port: what a lossless device loses from S is what it radiates.

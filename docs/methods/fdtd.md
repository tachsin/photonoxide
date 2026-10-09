---
title: "FDTD"
module: fdtd
summary: "Maxwell's equations stepped in time on Yee's grid: E and H leapfrogging on FDFD's own grid, the convolutional PML, walls, periodic and Bloch-periodic sides (complex fields), conductors, lossy media, subpixel smoothing of isotropic and anisotropic bodies, Drude and Lorentz media by auxiliary differential equations and the catalogue's materials fitted by them, the same bits on any number of threads; dipoles and currents normalized exactly by their spectra, plane waves on total-field/scattered-field boxes at any grid angle, one-way waveguide modes and Gaussian beams; transform, flux and mode monitors, and resonances by harmonic inversion; Mie's series for a sphere, and spheres against it; a ring's exact resonances, and rings against them; S-parameters over a band from one pulse against FDFD at each frequency; Meep's published PML and smoothing convergence; and on the GPU through wgpu, repeating bit for bit, within a stated tolerance of the CPU."
order: 23
papers:
  - cite: "G. Mie, Ann. Phys. 330, 377 (1908) (scattering by a sphere)"
    doi: 10.1002/andp.19083300302
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "O. Aberth, Math. Comput. 27, 339 (1973) (the stability check's polynomial roots)"
    doi: 10.1090/S0025-5718-1973-0329236-7
  - cite: "A. Taflove, M. E. Brodwin, IEEE Trans. Microw. Theory Tech. 23, 623 (1975) (stability and numerical dispersion)"
    doi: 10.1109/TMTT.1975.1128640
  - cite: "K. Umashankar, A. Taflove, IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982) (total-field/scattered-field)"
    doi: 10.1109/TEMC.1982.304054
  - cite: "J.-P. Berenger, J. Comput. Phys. 114, 185 (1994) (the PML)"
    doi: 10.1006/jcph.1994.1159
  - cite: "C. L. Lawson, R. J. Hanson, Solving Least Squares Problems, SIAM (1995) (nonnegative least squares, for the catalogue's fits)"
    doi: 10.1137/1.9781611971217
  - cite: "V. A. Mandelshtam, H. S. Taylor, J. Chem. Phys. 107, 6756 (1997) (harmonic inversion by filter diagonalization)"
    doi: 10.1063/1.475324
  - cite: "M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave Lett. 7, 121 (1997) (dispersive media by auxiliary differential equations)"
    doi: 10.1109/75.569723
  - cite: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000) (the convolutional PML)"
    doi: 10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A
  - cite: "A. Farjadpour et al., Opt. Lett. 31, 2972 (2006) (subpixel smoothing)"
    doi: 10.1364/OL.31.002972
  - cite: "G. R. Werner, J. R. Cary, J. Comput. Phys. 226, 1085 (2007) (a symmetric ε⁻¹ for tensor media, `Coupling::Nodes`)"
    doi: 10.1016/j.jcp.2007.05.008
  - cite: "C. Kottke, A. Farjadpour, S. G. Johnson, Phys. Rev. E 77, 036611 (2008) (smoothing anisotropic media)"
    doi: 10.1103/PhysRevE.77.036611
  - cite: "A. F. Oskooi, C. Kottke, S. G. Johnson, Opt. Lett. 34, 2778 (2009) (anisotropic smoothing on Yee's grid)"
    doi: 10.1364/OL.34.002778
  - cite: "P. Micikevicius, Proc. GPGPU-2, 79 (2009) (3D finite differences on GPUs, slice by slice)"
    doi: 10.1145/1513895.1513905
  - cite: "A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010) (sources restricted to the grid; its Figs. 7 and 8 reproduced)"
    doi: 10.1016/j.cpc.2009.11.008
  - cite: "C. A. Bauer, G. R. Werner, J. R. Cary, J. Comput. Phys. 230, 2060 (2011) (the triplet tensor, exact at a plane)"
    doi: 10.1016/j.jcp.2010.12.005
  - cite: "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012) (total-field/scattered-field in the frequency domain)"
    doi: 10.2528/PIERB11092006
  - cite: "G. R. Werner, C. A. Bauer, J. R. Cary, J. Comput. Phys. 255, 436 (2013) (a stable ε⁻¹ from triplets, `Coupling::Triplets`)"
    doi: 10.1016/j.jcp.2013.08.009
  - cite: "T. Malas, G. Hager, H. Ltaief, H. Stengel, G. Wellein, D. Keyes, SIAM J. Sci. Comput. 37, C439 (2015) (wavefront diamond blocking)"
    doi: 10.1137/140991133
  - cite: "T. M. Malas, J. Hornich, G. Hager, H. Ltaief, C. Pflaum, D. E. Keyes, Proc. IEEE IPDPS 2016, 142 (diamond blocking of a Yee stencil)"
    doi: 10.1109/IPDPS.2016.87
  - cite: "Z. Liu, J. K. S. Poon, Opt. Continuum 4, 2427 (2025) (six PDK devices in Lumerical FDTD and Tidy3D)"
    doi: 10.1364/OPTCON.572107
validation:
  - fdtd/dispersion
  - fdtd/energy
  - fdtd/fdfd-lossy
  - fdtd/fdfd-cpml
  - fdtd/cpml-thickness
  - fdtd/spectrum
  - fdtd/tfsf-leakage
  - fdtd/tfsf-slab
  - fdtd/mode-source-fdfd
  - fdtd/mode-source-backward
  - fdtd/mode-source-power
  - fdtd/beam-waist
  - fdtd/beam-divergence
  - fdtd/beam-tilt
  - fdtd/ade-fdfd
  - fdtd/drude-fresnel
  - fdtd/lorentz-group-delay
  - fdtd/bloch-fdfd
  - fdtd/bloch-multilayer
  - fdtd/bloch-bands
  - fdtd/fit-lossless
  - fdtd/fit-lossy
  - fdtd/fitted-slab
  - fdtd/smoothing-slab
  - fdtd/smoothing-anisotropic-slab
  - fdtd/smoothing-oblique
  - fdtd/smoothing-oblique-nodes
  - fdtd/smoothing-energy
  - fdtd/smoothing-contrast
  - fdtd/smoothing-oskooi
  - fdtd/smoothing-triplets-oblique
  - fdtd/smoothing-triplets-order
  - fdtd/smoothing-diagonal-oblique
  - fdtd/smoothing-diagonal-order
  - fdtd/smoothing-bauer-order
  - fdtd/smoothing-triplets-contrast
  - fdtd/smoothing-triplets-lattices
  - fdtd/smoothing-wc07-growth
  - fdtd/smoothing-crystal
  - fdtd/smoothing-crystal-order
  - fdtd/monitor-transforms
  - fdtd/monitor-flux
  - fdtd/monitor-flux-box
  - fdtd/monitor-modes
  - fdtd/monitor-guides
  - fdtd/harmonic-inversion
  - fdtd/cavity-resonances
  - fdtd/slab-resonance
  - fdtd/mie-table
  - fdtd/mie-balance
  - fdtd/mie-terms
  - fdtd/mie-sphere
  - fdtd/mie-sphere-staircase
  - fdtd/mie-drude
  - fdtd/meep-pml-rates
  - fdtd/ring-wronskian
  - fdtd/ring-resonances
  - fdtd/ring-order
  - fdtd/fdfd-band-2d
  - fdtd/fdfd-band-2d-fine
  - fdtd/fdfd-band-3d-strip
  - fdtd/fdfd-band-3d-bend
  - fdtd/fdfd-smoothed
examples:
  - cpml_roden_gedney
  - tfsf_square_cylinder
  - lorentz_okoniewski
  - subpixel_holes
  - pml_oskooi
  - bump_oskooi
  - coupler_liu_poon
  - crossing_liu_poon
  - mmi_liu_poon
  - mode_converter_liu_poon
  - splitter_rotator_liu_poon
  - ring_liu_poon
---

The finite-difference time-domain method steps Maxwell's equations forward in time. One run
with a short pulse gives a structure's response over a whole band, and the field's evolution is
there to watch. photonoxide's FDTD shares FDFD's grid and conventions. A continuous source settles
to the field FDFD finds for the same current, and the leapfrog's own frequency, below, makes that
exact.

## The scheme

With c = 1, lengths in µm, times in µm/c and $\tilde H = \eta_0 H$ as in [FDFD](fdfd.md):

$$
\frac{\partial \tilde H}{\partial t} = -\nabla\times E - M, \qquad
\varepsilon\frac{\partial E}{\partial t} + \sigma E = \nabla\times\tilde H - J .
$$

Yee's scheme staggers the fields in space, on the same grid as [FDFD in 3D](fdfd-3d.md): E's
components at the middle of the cells' edges, H's at the centres of their faces. It also
staggers them in time: H half a step after E, each advanced from the other's curl (the
leapfrog). Every derivative is a centred difference, second order in space and in time. The
conductivity is averaged over the step, which keeps the update stable for any σ.

**Stability.** The step is $\Delta t = C/\sqrt{\sum_i 1/\Delta_i^2}$, with the Courant number
$0 \lt C \le 1$ (Taflove and Brodwin), the sum over the axes the field varies along. An axis one
cell long and periodic doesn't count, so a 2D problem steps at the 2D limit.

**Numerical dispersion.** A plane wave on the grid travels at Taflove and Brodwin's speed:

$$
\frac{\sin^2(\omega\Delta t/2)}{\Delta t^2} = \sum_i \frac{\sin^2(k_i\Delta_i/2)}{\Delta_i^2} ,
$$

slower than light, most along the axes. The scheme keeps this relation exactly, not
approximately: a plane wave started on a periodic grid oscillates at its frequency to round-off
(`fdtd/dispersion`, 3.6e-15).

**Energy.** In a closed lossless box the leapfrog conserves
$\tfrac12\sum\varepsilon E^2 + \tfrac12\sum\tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$ exactly, because
Yee's two curls are each other's transposes (`fdtd/energy`, 2.4e-15 over 300 steps).

**FDFD's field, exactly.** A current $\operatorname{Re}(\hat J e^{-i\omega t})$, sampled at the half steps where it enters
E's update, settles to a field that solves FDFD's equations at the leapfrog's frequency:

$$
\tilde\omega = \frac{2}{\Delta t}\sin\frac{\omega\Delta t}{2} ,
$$

with a conductivity σ entering as $\varepsilon + i\sigma\cos(\omega\Delta t/2)/\tilde\omega$. In a closed
lossy box FDTD's steady amplitude and `Solver3d`'s field at $\tilde\omega$ agree to 2.3e-14,
against 4.1e-4 at ω itself (`fdtd/fdfd-lossy`). FDTD at a given step is FDFD at a slightly lower
frequency, the difference falling as $\Delta t^2$.

## Boundaries

**The convolutional PML** (Roden and Gedney) absorbs outgoing waves in a layer of cells. It is
the stretched coordinate of Berenger's PML, complex-frequency-shifted:

$$
s = \kappa + \frac{\sigma}{\alpha - i\omega} ,
$$

applied in the time domain by a recursive convolution. Each derivative across the layer becomes
$\partial/\kappa + \psi$, with $\psi \leftarrow b\psi + a\,\partial F$,
$b = e^{-(\sigma/\kappa + \alpha)\Delta t}$ and $a = \sigma(b - 1)/(\sigma\kappa + \kappa^2\alpha)$.
σ and κ grow as $(\text{depth}/d)^m$; α is constant. The auxiliary ψ live only in the layer's
slabs, two per field component per axis.

- **Defaults:** κ = 1 and α = 0, with σ from a target reflection R = 10⁻⁸ and order 3:
  $\sigma_\text{max} = (m + 1)(-\ln R)/(2d)$. That is FDFD's own PML, $s = 1 + i\sigma/\omega$,
  graded the same way. With it, FDTD's field approaches FDFD's as the step falls, the recursive
  convolution being FDFD's stretch to first order in Δt (`fdtd/fdfd-cpml`: 6.6e-4 at 64 steps a
  period, 3.5e-4 at 128).
- **Reflection:** at 2 cells from the layer, a pulse's reflection is 2.6e-2 of the field for 4
  cells, 1.2e-4 for 8 and 6.1e-6 for 16 (`fdtd/cpml-thickness`).
- **Evanescent fields:** κ > 1 and α > 0 absorb them, which the traditional PML reflects in late
  time. Roden and Gedney measured this beside a conducting plate in soil, and the
  [`cpml_roden_gedney`](../../examples/cpml_roden_gedney.rs) example reproduces it:
  - −48.6 dB with the traditional PML (their −48);
  - −70.5 dB with α = 0.05 S/m (their −67).
  - Their pulse's width isn't given; it is set so the first number is theirs, and the second is
    then a prediction.
  - `Cpml::sigma_optimal` is their Eq. 15.

**Walls.** A PML of zero cells is a perfect electric conductor: the tangential E on the wall is
held at zero, as in FDFD. **Periodic** sides wrap. A 2D problem is a grid one cell thick along
z, periodic there (`Boundaries::cpml_2d`).

**Bloch-periodic sides.** `Edges::Bloch { k }` with k ≠ 0 makes the field one period on
$e^{ikL}$ times itself, L the grid's length along the axis, FDFD's convention. A periodic
structure, a grating or a metasurface's unit cell, then needs one cell, and a fixed k along the
layers of a stack is oblique incidence, each frequency at its own angle, $\sin\theta = k/\omega$.

- **Complex fields.** For kL not a multiple of π, $e^{ikL}$ isn't real, and neither is a field
  that obeys it, so the fields are complex (`Simulation::is_complex`). They are not two
  separate real runs: the real and imaginary parts are the "sine and cosine" runs of a Bloch
  problem, but each part's boundary reads the other's at every step,
  $\operatorname{Re} F(x + L) = \cos(kL)\operatorname{Re} F(x) - \sin(kL)\operatorname{Im} F(x)$.
  The parts are stored as two real runs stepped side by side: Yee's update is real everywhere
  but where a difference reaches across a Bloch side, so each part steps with the real update,
  the same kernel as a periodic run, and only the values next to a Bloch side are coupled after
  each half step, by the difference $(e^{\pm ikL} - 1)F$ that the phase makes. A real run at
  k = 0 is the same bits as before, at the same speed; a complex run costs two.
- **Sources** in a complex run: a current of complex amplitude a takes the complex values
  $a\thinspace w(t)$, w the waveform's analytic form, so a pulse puts in its positive frequencies
  only; a point `Source` is real. A dipole restricted across a Bloch side puts its weight there
  on the image at the other end, times $e^{\mp ikL}$. The run's DFT divided by
  $a\thinspace W(\omega)$ (`Waveform::analytic_spectrum`) is FDFD's Bloch field at $\tilde\omega$.
  Total-field/scattered-field sources (plane waves, modes and beams) need real fields for now;
  a sheet of `Current` with the Bloch phase launches a plane wave instead.
- **Stability:** along a Bloch axis one cell long, the field varies by $e^{ik\Delta}$ a cell,
  so its part of the curl's largest eigenvalue is $(2/\Delta)^2\sin^2(k\Delta/2)$, and Δt counts
  it so.
- **Against FDFD:** a continuous current and a dipole restricted across two Bloch sides
  ($k_x = 1.3$, $k_y = -2.1$ rad/µm) in a lossy box settle to `Solver3d`'s Bloch field at
  $\tilde\omega$ to 2.9e-14, in a plain medium and in a dispersive one (`fdtd/bloch-fdfd`). A
  Bloch phase of a whole turn, $kL = 2\pi$, is the periodic run to round-off (a unit test).
- **Oblique incidence:** a pulse at $k_x = 2\pi \times 0.2$ rad/µm on 4 pairs of n = 2 and 1.5,
  in a column one cell across, transmits as the transfer matrices say, at each frequency's own
  angle (16.6° at 0.7 c/µm to 8.8° at 1.3), to second order in the cells
  (`fdtd/bloch-multilayer`):

  | Cells | TE | TM |
  |---|---|---|
  | 20 nm | 2.0e-2 | 2.0e-2 |
  | 10 nm | 4.9e-3 | 5.0e-3 |
  | 5 nm | 1.2e-3 | 1.3e-3 |

  The pulse is 0.3 c/µm wide, so that it puts next to nothing below $k_x/2\pi = 0.2$ c/µm. There
  the stack guides light that vacuum can't take, which rings for ever and would leak into every
  frequency of a finite DFT.
- **Bands:** one period of a 1D photonic crystal (0.2 µm of n = 2, 0.3 µm of vacuum) between
  Bloch sides at $k = 0.3\pi/\Lambda$ gives its three bands below 2 c/µm. A broadband run's
  probe, its DFT windowed by Blackman and Harris's four terms, shows each band as a peak; then a
  run whose pulse is 0.03 c/µm wide at each peak excites that band alone (the next is far below
  round-off in its spectrum), and its frequency is the probe's one-mode recurrence
  $E^{n+1} = zE^n$ fitted by least squares once the pulse is over, $\omega = -\arg(z)/\Delta t$:
  a lossless band rings as exactly $z^n$. The bands converge to the analytic
  $\cos k\Lambda = \cos k_1d_1\cos k_2d_2 - \tfrac12(n_1/n_2 + n_2/n_1)\sin k_1d_1\sin k_2d_2$
  (half the trace of a period's transfer matrix) to second order: 5.0e-3, 1.2e-3 and 3.1e-4
  at 20, 40 and 80 cells a period, relative (`fdtd/bloch-bands`). Harmonic inversion, which
  finds many modes and their Q in one run, is #163's.

**Conductors and media.** `Simulation::plate` and `conductor` hold E at zero on a conducting
plate or edge. The permittivity is real, averaged over each component's cell as FDFD averages it
(or smoothed, below), and a conductivity σ (in 1/µm: S/m × 376.73 Ω × 10⁻⁶) makes a medium
lossy.

## Subpixel smoothing

`Simulation::new` averages a permittivity function over each value's cell by sampling it, which
puts an interface between grid points only to within the samples' spacing.
`Simulation::smoothed` instead takes a `Structure`: a background `Permittivity` and `Body`s over
it, each later one over those before (half-spaces; ellipsoids, and with infinite semi-axes
elliptic cylinders and slabs; photonoxide's planar shapes extruded along z), each of a real,
symmetric, positive-definite tensor (isotropic, diagonal, principal values along given axes, or
uniaxial as the catalogue's LiNbO₃ and AlN are). Each value of E sees the tensor averaged over a
cell about it, by a `Smoothing`: its `Average`, the cell's `diameter` in grid cells (Farjadpour
et al.'s s, 1 by default) and the `Coupling` that places the off-diagonal entries.

**The average** (`Average::Subpixel`). At an interface between isotropic media, Farjadpour et
al. (Eq. 1) average ε along the interface and ε⁻¹ across it,

$$
\tilde\varepsilon^{-1} = P\langle\varepsilon^{-1}\rangle + (1 - P)\langle\varepsilon\rangle^{-1},
\qquad P = nn^{\mathsf T},
$$

n the interface's normal. That is the one smoothing whose perturbation of the structure has no
first-order effect: what a small change of ε moves a mode's frequency or a scattered power by is
$\Delta\varepsilon\lvert E_\parallel\rvert^2 - \Delta(\varepsilon^{-1})\lvert D_\perp\rvert^2$
integrated across the interface, and these two averages make both integrals vanish. Kottke et al.
generalize it to anisotropic media (their Eqs. 4, 22, 23): in the interface's frame,
$\tilde\varepsilon = \tau^{-1}(\langle\tau(\varepsilon)\rangle)$, with
$\tau(\varepsilon)$ the matrix of $-1/\varepsilon_{11}$, $\varepsilon_{1j}/\varepsilon_{11}$,
$\varepsilon_{i1}/\varepsilon_{11}$ and $\varepsilon_{ij} - \varepsilon_{i1}\varepsilon_{1j}/\varepsilon_{11}$,
the map of the fields continuous across the interface, $(D_1, E_2, E_3)$; for isotropic media it
is Eq. 1 again (a unit test, to 1e-14). `Average::Mean` ($\langle\varepsilon\rangle$, Dey and
Mittra's), `Average::InverseMean` ($\langle\varepsilon^{-1}\rangle^{-1}$) and `Average::Sampled`
(the tensor at the value's own point: staircases) are there to compare with.

**The cell.** Where one body's surface crosses the cell, the surface is taken as the plane
through the nearest point, normal to it there: the body's signed distance and normal at the
cell's centre (exact for a half-space, a sphere and the planar shapes; for an ellipsoid the
distance is $f/\lvert\nabla f\rvert$ with $f = \lvert u\rvert - 1$, second order in itself). The
share of the cell inside the plane is the exact volume it cuts off: across the box, the area of
the cut along the axis of the plane's smallest slope is piecewise quadratic, so two-point
Gauss–Legendre on each piece is exact with no division by a small slope (to 1e-14 against
counting points). A curved surface's departure from the plane, and the distance's error, are
second order. The bodies are taken from the top down to one that covers the cell; where two
surfaces cross it, it is sampled 8 times along each axis instead, in the frame of the upper one.
A corner's cell has an error of the order of its area, as Farjadpour et al. find (between first
and second order where corners matter).

**On Yee's grid** (Oskooi et al., Fig. 1). Each value of E keeps its own diagonal entry of
$\tilde\varepsilon^{-1}$, from the cell centred on it. Where the tensor couples E's components
(an interface oblique to the grid, or an anisotropic medium whose axes aren't the grid's), the
simulation steps D as it would E in vacuum (its sources, plane waves and CPMLs too) and finds
$E = \tilde\varepsilon^{-1}D$ after each step; where nothing couples, the scalar update runs
unchanged. E_x needs D_y and D_z, which sit elsewhere, and there are three ways to place the
off-diagonal entries:

- `Coupling::Nodes` (the default before 0.5, still `Smoothing::with`'s): (ε̃⁻¹)_xy from the cells centred on the nodes, E_x taking at
  each of the two nodes beside it the mean of D_y on either side times (ε̃⁻¹)_xy there, and the
  mean of the two. This is G. R. Werner and J. R. Cary's scheme exactly (J. Comput. Phys. 226,
  1085 (2007), their (26c) with (27e), Eq. 39, which Oskooi et al. follow; checked against the
  paper): only their effective dielectric differs, MPB's average, the same as Kottke's for
  isotropic media, where photonoxide takes Kottke's for anisotropic media too, as Oskooi et al.
  and Werner et al. 2013 do. ε̃⁻¹ on the grid is symmetric, so the leapfrog conserves
  $\tfrac12\sum E\cdot D + \tfrac12\sum\tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$ exactly
  (`fdtd/smoothing-energy`, 2.0e-15 over 10⁵ steps), and the scheme is stable while that energy
  is positive: while ε̃⁻¹ on the grid is positive definite (below). But each row of
  ε̃⁻¹ then mixes the cells of three points, linearly, where the tangential part needs
  $\langle\varepsilon\rangle$: across an oblique interface that is a first-order error, as the
  mean's. Measured, with layers of ε = 12 and 1 at 26.6° to the grid against their transfer
  matrices (`fdtd/smoothing-oblique`): the error times n is −0.56, −0.39, −0.28, −0.26 at 16 to
  128 cells a µm, the mean's −0.74, −0.46, −0.32, −0.23.
- `Coupling::Points`: (ε̃⁻¹)_xy from E_x's own cell, times the mean of the four D_y around it,
  Farjadpour et al.'s placement. Each row is the cell's own tensor, and the same layers converge
  as h², the error times n² −2.1, −2.8, −3.3, −3.5. But ε̃⁻¹ isn't symmetric: the energy above
  changes by 6.3e-2 over 300 steps, and an error grows exponentially from round-off: in a period
  of the elliptical holes below at 24 cells by 10¹⁷ over 4 × 10⁵ steps (16 cells: 10³), and in
  the anisotropic lattice at 16 cells by 10¹⁶ within 2 × 10⁴. For short runs in isotropic media
  only.
- `Coupling::Triplets`: G. R. Werner, C. A. Bauer and J. R. Cary's scheme (J. Comput. Phys. 255,
  436 (2013), Secs. 4 and 7). The three values of E on the edges from a node, one each way along
  each axis, are a triplet, and each node has eight. Each triplet takes one symmetric
  positive-definite 3 × 3 tensor, its diagonal entries too, and ε̃⁻¹ on the grid is the mean of
  the eight block-diagonal matrices so made: positive definite at any contrast, with nothing to
  check. In a uniform medium it is Werner and Cary's interpolation, second order. At an interface,
  each tensor is C. A. Bauer, G. R. Werner and J. R. Cary's (J. Comput. Phys. 230, 2060 (2011),
  Eqs. 27 to 44): with F = (D_n, E_t, E_s), continuous across the interface's plane (found from
  the node, as the cell's), E = CF and D = PF in each medium, C = 1 + n(n − εn)ᵀ/(nᵀεn) and
  P = εC; the rows of Λ_C are C's averaged along each value's edge and those of Λ_P are P's over
  its dual face (the exact shares a plane cuts off, as the cells'), and
  κ = Λ_C Λ_P⁻¹ takes constant fields' D to their E exactly (a unit test, to 1e-13). κ isn't
  symmetric, and Werner et al. use ½(κ + κᵀ). Where that isn't positive, or two surfaces pass
  within a cell of the node, the node takes Kottke's average over its own cell, their step 6.
  It takes its own average, so `Average::Subpixel` and a diameter of 1 only.

**What the literature allows.** Werner, Bauer and Cary find every symmetric effective dielectric
they know, their own included, first order at sharp interfaces in the end. It is second order at
coarse grids and turns first order at a resolution that falls as the contrast rises (their Figs.
4, 7 and 8, Sec. 9). Only Bauer et al.'s tensor as it is, exact for constant fields at a plane,
is second order, and it isn't symmetric, so it isn't stable in time. They tried other weights and
found none both symmetric and second order. So no stable placement here is second order at
oblique interfaces. Measured at the layers above (relative error of the frequency, n cells a µm):

| n | `Nodes` | `Triplets` | Bauer's κ, not symmetric | `Points` |
|---|---|---|---|---|
| 16 | −3.5e-2 | −2.4e-2 | −2.2e-2 | −8.3e-3 |
| 32 | −1.2e-2 | −7.3e-3 | −5.6e-3 | −2.7e-3 |
| 64 | −4.3e-3 | −2.3e-3 | −1.5e-3 | −8.0e-4 |
| 128 | −2.0e-3 | −1.06e-3 | −4.2e-4 | −2.1e-4 |

The triplets' error times n is −0.38, −0.23, −0.15, −0.14: first order (1.15 from 64 to 128,
`fdtd/smoothing-triplets-order`), about half the nodes' (`fdtd/smoothing-triplets-oblique`), as
Werner et al. find (2 to 3 times better than the 2007 scheme). Bauer's own κ, run as a check
(not a `Coupling`), has error times n² −5.7, −5.7, −6.3, −6.9: second order (1.87,
`fdtd/smoothing-bauer-order`). Making it symmetric is what costs the order. At a plane along the
grid each triplet's κ is the τ average of the cell, and the slab's reflection is the nodes' to
1e-15, isotropic and anisotropic. In Bauer et al.'s own 3D crystal (their Sec. 4.3: an
orthorhombic lattice of turned anisotropic ellipsoids, principal values 8, 10 and 12 in vacuum),
the nine lowest bands at k = 0 against their Table 1, at 48 cells a lattice vector:

- the triplets: 1.1e-3 to 1.8e-3 (`fdtd/smoothing-crystal`);
- the nodes: 1.4e-3 to 2.3e-3;
- Bauer's κ: 0.8e-3 to 1.5e-3, converging at order 1.98 (`fdtd/smoothing-crystal-order`).

Their Eq. 57's rotations are read as passive: of the four readings (active or passive, for the
body and for the tensor), the only one under which all nine bands converge to the table. In
Oskooi et al.'s anisotropic lattice the triplets give 2.0e-4 at 32 cells, the nodes 2.3e-4. In
Farjadpour et al.'s holes (the lowest mode at the X point, against Bauer's κ by Richardson's
extrapolation from 64 and 96 cells), the triplets give 1.8e-4 at 32 cells and 6e-5 at 64. That
is half the mean ε's (3.8e-4, 1.2e-4); `Points` gives 1.4e-4 and 4e-5. Their figure's margin of
10 to 30 times over the mean isn't reproduced by any placement here.

**Positive only to a contrast.** Werner and Cary's ε̃⁻¹ is symmetric at any contrast but not
positive definite: each value's diagonal entry comes from its own cell and its off-diagonal
ones from the nodes' cells, and at a high contrast they no longer fit together. An isotropic
ellipsoid at an angle to the grid in vacuum (20 × 18 × 16 cells of 50 nm,
`fdtd/smoothing-contrast`): ε̃⁻¹ is positive definite at ε = 12 and 30 (its largest eigenvalue
1.06 and 1.09), but its least is −4.6e-3 at 50 and −1.7e-2 at 100. The energy, conserved, then
bounds nothing, and the fields grow: at ε = 50 by 10¹² in 10³ steps and 10¹⁴⁹ in 10⁴ at Courant
number 0.99, 10¹²⁰ in 10⁴ at 0.8. `Simulation::smoothed` refuses such a structure. The values
an off-diagonal entry reaches are their own block of ε̃⁻¹, a thin band about the interfaces, and the
check is its Cholesky factorization, which exists exactly when the block is positive definite
(milliseconds at this size, 0.5 s at 140³ cells with 1.5 × 10⁵ coupled values, less than
the smoothing itself; values held at zero are left out, a principal block). Here it
refuses from ε = 45, and 40 stays bounded over 10⁵ steps; the check is sufficient, not sharp: 45
let through stays bounded over 10⁵ steps too. A node-by-node bound (each node's 3 × 3 tensor
positive with the least diagonal of the values beside it) would be cheaper but fails already at
ε = 12. Werner, Bauer and Cary (2013, Sec. 5) found the 2007 scheme unstable at high contrast
for this reason: each node's tensor mixes the diagonal entries of the cells about the values with
the off-diagonal ones of the node's cell. Their case is reproduced (`fdtd/smoothing-wc07-growth`):

- **The structure:** a square lattice of isotropic discs of radius 0.37a in vacuum, TE, from
  random fields, `Coupling::Nodes` let through unchecked.
- **ε = 100:** the fields grow as e^{γt} with γ = 2.85 c/a at 32 cells a period (theirs ≈ 3) and
  6.11 at 64 (theirs ≈ 6).
- **ε = 60:** their 0.5 c/a at 64 cells isn't reproduced. Nothing grows here over 3000 a/c,
  though the check refuses that ε̃⁻¹; the disc's place on the grid isn't given.
- **`Coupling::Triplets`:** stays within 1.1 times the first ‖E‖ in all of them. In the
  ellipsoid above at ε = 100 it keeps the energy to 5.2e-15 over 10⁵ steps
  (`fdtd/smoothing-triplets-contrast`). Over 10⁵ steps in a period of Farjadpour et al.'s holes
  and of Oskooi et al.'s anisotropic lattice, where `Points` grows, it keeps the energy to
  4.3e-15 and 5.0e-15 (`fdtd/smoothing-triplets-lattices`).

Oskooi et al. report second-order convergence with Werner and Cary's placement in their
anisotropic lattice, at a contrast near 8, where Werner et al. 2013 also see second order up to
hundreds of cells a wavelength. At the contrast of 12 of the layers above it is first order.

`Coupling::Triplets` is the default (`Smoothing::default`) from 0.5; `Coupling::Triplets`
takes the same eight terms a value each step, from a table of the distinct nodes' entries. It is
stable at any contrast, with about half the error at oblique interfaces and the same results at
interfaces along the grid. Its smoothing costs more than the nodes' (8 tensors a node, each from
6 edges and 6 faces).

**The diagonal alone** (`Coupling::Diagonal`). Each value of E keeps its own diagonal entry of
$\tilde\varepsilon^{-1}$ over its cell, for isotropic media
$n_c^2\langle\varepsilon^{-1}\rangle + (1 - n_c^2)\langle\varepsilon\rangle^{-1}$, and the
off-diagonal ones are dropped: E = $(\tilde\varepsilon^{-1})_{cc}D_c$, a permittivity per
component, as conformal and subpixel meshes without a tensor take it. Nothing couples, so the
update is the scalar one, stable at any contrast and combinable with a conductivity, a dispersive
medium or a Bloch phase. Along the grid it is the tensor exactly (a unit test). At an oblique
interface the dropped coupling is first order: at the oblique layers the error times n is 0.14, 0.38, 0.52, 0.57 and 0.62 at 16 to 256 cells a µm, the frequencies too high, about four times the triplets'; 4.43e-3 at 128
(`fdtd/smoothing-diagonal-oblique`, order 0.87 from 64 to 128, `fdtd/smoothing-diagonal-order`). Its use
is speed: a tensor that couples anything in a 3D device (a bend's faces, a taper's) steps the
whole grid every step, where a scalar permittivity steps by the kernel's diamonds (below), about
four times faster beyond the caches. On a 500 × 220 nm silicon S-bend in silica, 6 million cells
of 30 nm, 20 threads of a Core Ultra 7 265K made 158 million cell-updates/s with the triplets
and 582 with the diagonal. The devices of Liu and Poon (below) take it.

**What doesn't combine.** A tensor that couples E's components refuses a conductivity, a
dispersive medium (whose ε∞ would have to enter the tensor) and a Bloch side with k ≠ 0 (D's
coupling across the side would need the phase). Where nothing couples, all three work as with
`Simulation::new`: a dispersive medium's values take its ε∞ at their points, unsmoothed.

**Measurements.**

- A slab of ε = 4, 0.3 µm thick, its faces between grid points, at normal incidence: its
  complex reflection against Airy's formula, second order wherever the faces fall, 5.8e-2,
  1.2e-2, 3.0e-3 and 6.9e-4 at 1/20 to 1/160 µm; sampled, 1.6e-1, 1.1e-2, 4.8e-2 and 1.8e-2
  (`fdtd/smoothing-slab`).
- An anisotropic slab (principal values 2, 3 and 4.5 along turned axes, every entry non-zero):
  $r_{xx}$ and $r_{yx}$ against the exact ones, from the transverse block of τ along whose
  principal axes the slab is Airy's (at normal incidence the 4 × 4 transfer matrices split in
  two; photonoxide has none for oblique incidence): 2.9e-2, 9.5e-3, 1.2e-3 and 3.0e-4
  (`fdtd/smoothing-anisotropic-slab`).
- Oblique layers, above: 2.1e-4 at 128 cells a µm with `Coupling::Points`
  (`fdtd/smoothing-oblique`), 2.0e-3 with `Coupling::Nodes` (`fdtd/smoothing-oblique-nodes`),
  1.06e-3 with `Coupling::Triplets` (`fdtd/smoothing-triplets-oblique`).
- Oskooi et al.'s 2D anisotropic lattice (ellipses of principal values 1.45, 2.81 and 4.98 in
  8.49, 8.78 and 11.52, 0.355 × 0.305 of the period at 30° as their inset, the axes ours), the
  lowest mode at k = (½, 0) 2π/a, 32 cells a period, `Coupling::Nodes` (`Coupling::Points` grows
  too fast here): the error 2.3e-4 against 4.5e-3 for the harmonic mean and 3.7e-3 without
  smoothing, their "order of magnitude" (`fdtd/smoothing-oskooi`); but 2.5e-4 for the mean,
  which their Fig. 2 has 6 times the new method's.
- Farjadpour et al.'s Fig. 1, the `subpixel_holes` example: elliptical air holes in ε = 12, TE,
  12 to 64 pixels a period. The lowest mode converges at −2.26 with s = 2 (their −2.43, read
  at 600 dpi) and −2.12 with s = 1, below the mean at every resolution; the next mode's mean
  at −1.40 (their −1.33). The lowest mode's mean converges at −1.84, not −1.33.

The smoothing is computed once, four cells' averages a Yee cell, in parallel; a coupling tensor
adds a pass over E each step, and D's storage.

The same smoothing would serve FDFD, which averages by sampling (8 samples an axis,
harmonically along each component): its grid-aligned interfaces would be exact wherever they
fall, and with the off-diagonal entries FDFD's matrix stays symmetric, as its solvers need,
only with Werner and Cary's placement. A separate change.

## Dispersive media

`Simulation::with_medium` puts a `Dispersive` medium where a function says, sampled at each value
of E (not averaged over the cell as the permittivity is, so a face is best put halfway between
two values):

$$
\varepsilon(\omega) = \varepsilon_\infty + \sum_p \frac{a_p}{\omega_{0p}^2 - \omega^2 - i\Gamma_p\omega} ,
$$

each term a Lorentz oscillator ($a_p = \Delta\varepsilon_p\thinspace\omega_{0p}^2$) or a Drude term
($\omega_{0p} = 0$, $a_p = \omega_p^2$), as the material models write them. Several of each go in
one medium, and several media in one run.

**Auxiliary differential equations** (Okoniewski, Mrozowski and Stuchly). Each term carries a
polarization current $J_p = \partial P_p/\partial t$, so that Ampère's law reads
$\varepsilon_\infty\partial E/\partial t + \sigma E + \sum_p J_p = \nabla\times\tilde H - J$, and each
current obeys their Eq. 5,

$$
\frac{\partial^2 J_p}{\partial t^2} + \Gamma_p\frac{\partial J_p}{\partial t} + \omega_{0p}^2 J_p = a_p\frac{\partial E}{\partial t} ,
$$

which for a Drude term is the time derivative of $\partial J_p/\partial t + \Gamma_p J_p = \omega_p^2 E$,
the same from rest. The update is their synchronized Lorentz scheme (LADES): the currents at the
steps, E's update at the half step with $J_p^{n+1/2} = (J_p^{n+1} + J_p^n)/2$, and the currents'
equation centred on step n with $(\partial E/\partial t)^n = (E^{n+1} - E^{n-1})/(2\Delta t)$:

$$
J_p^{n+1} = \alpha_p J_p^n + \xi_p J_p^{n-1} + \gamma_p\frac{E^{n+1} - E^{n-1}}{2\Delta t} ,
\quad \alpha_p = \frac{2 - \omega_{0p}^2\Delta t^2}{1 + \Gamma_p\Delta t/2},\
\xi_p = \frac{\Gamma_p\Delta t/2 - 1}{1 + \Gamma_p\Delta t/2},\
\gamma_p = \frac{a_p\Delta t^2}{1 + \Gamma_p\Delta t/2} .
$$

The coefficients are derived here from Eq. 5 by central differences: their Eq. 8 prints $\gamma_p$
without the $1/(1 + \delta_p\Delta t)$ that the centred damping term puts on every term. Put into
E's update, the new E appears only through $\gamma_p$, and the update solves to their pseudo-code's
LADES case: E ← $p_e E + p_m(\nabla\times\tilde H - J) + p_m[\chi E^{n-1} - \sum_p((\alpha_p + 1)J_p^n + \xi_p J_p^{n-1})/2]$,
with $\chi = \sum_p\gamma_p/(4\Delta t)$, $\nu = 1 + (\Delta t/\varepsilon_\infty)(\sigma/2 + \chi)$,
$p_e = (1 - \Delta t\sigma/(2\varepsilon_\infty))/\nu$ and $p_m = (\Delta t/\varepsilon_\infty)/\nu$. The
grid's own update gives the first two terms with $p_e$ and $p_m$ in place of its coefficients;
the rest is added after it, and the currents then step. A dispersive value of E keeps 2P + 1
numbers: $J_p^n$, $J_p^{n-1}$ and $E^{n-1}$.

**What a run solves.** For $e^{-i\omega t}$ at the steps the update is FDFD's equations at
$\tilde\omega$ with each term's $\chi_p$ replaced by

$$
\frac{i\cos(\omega\Delta t/2)}{\tilde\omega}\thinspace\frac{\gamma_p(z - 1/z)}{2\Delta t\thinspace(z - \alpha_p - \xi_p/z)} ,\quad z = e^{-i\omega\Delta t} ,
$$

`Dispersive::leapfrog_permittivity`, which is $\varepsilon(\omega)$ to second order in Δt (a unit
test). A continuous current in a closed box filled with a Drude and a Lorentz term and a
conductivity settles to `Solver3d`'s field with that permittivity to 3.3e-14; with the medium's
own $\varepsilon(\tilde\omega)$ the difference is 2.4e-3 at 64 steps a period (`fdtd/ade-fdfd`).

**Stability.** A medium is refused unless it is passive, $\varepsilon_\infty \gt 0$ and every
strength and damping nonnegative (a term with either negative is a gain), and unless the update
steps every plane wave the grid holds with $\lvert z\rvert \le 1$: the roots of its
characteristic polynomial,

$$
\varepsilon_\infty(z - 1)^2 + \frac{\sigma\Delta t}{2}(z^2 - 1) + \sum_p\frac{\gamma_p}{4}\frac{(z^2 - 1)^2}{z^2 - \alpha_p z - \xi_p} + (K\Delta t)^2 z = 0 ,
$$

for $(K\Delta t)^2$ from 0 to $4C^2$, by Aberth's iteration. That works out as $C \le \sqrt{\varepsilon_\infty}$
and $\omega_{0p}\Delta t \le 2$ for every Lorentz term (its own recursion's roots leave the unit
circle beyond); Drude terms are stable at any $\omega_p\Delta t$ (a unit test sweeps these).

**Validation.**

- **A Drude metal** ($\varepsilon_\infty = 1$, $f_p = 1$ c/µm, $\gamma/2\pi = 0.05$ c/µm) as a
  half-space, lit by a pulse at normal incidence: its reflection coefficient from 0.4 to 1.6
  c/µm, below and above the plasma frequency, against Fresnel's $(1 - n)/(1 + n)$ with the same
  $\varepsilon(\omega)$, its phase referred to the face with the grid's own wavenumber. Second
  order in the cells: $\lvert\Delta r\rvert$ = 6.9e-3 at 20 nm, 1.6e-3 at 10 and 3.0e-4 at 5
  (`fdtd/drude-fresnel`).
- **A Lorentz slab** 10 µm thick ($\varepsilon = 2.25 + \omega_0^2/(\omega_0^2 - \omega^2 - i\gamma\omega)$,
  $f_0 = 2$ c/µm): the group delay of a pulse's first pass, cut before its first echo arrives,
  as a group index, against the first pass's exact delay, which is the bulk
  $n_g = \operatorname{Re}\thinspace d(n\omega)/d\omega$ (1.977 to 2.402 over 0.8 to 1.2 c/µm) to
  3e-5. Second order to a floor near 1e-4: 3.0e-3 at 10 nm, 8.8e-4 at 5 and 3.6e-4 at 2.5
  (`fdtd/lorentz-group-delay`). The pulse is 0.3 c/µm wide: a wider one reaches the resonance,
  where light is slow enough to be cut off with the echo, and the cut's leakage shows.
- **Okoniewski, Mrozowski and Stuchly's own case,** a two-term Lorentz half-space on cells of
  37.5 µm, is the [`lorentz_okoniewski`](../../examples/lorentz_okoniewski.rs) example. Their
  Fig. 1, read at 600 dpi, shows the scheme's errors in $\lvert r\rvert$ about 0.5 × 10⁻³ with
  ripples of ±0.3 × 10⁻³ up to 45 GHz, rising to 1.84 × 10⁻³ at 59.5 GHz, and in its phase
  within ±1 × 10⁻³ rad up to 50 GHz, rising to 2.1 × 10⁻³ at 57 GHz. Their time step and run
  aren't given, so the curve is read as a bound. Here the errors are within it at every
  frequency from 2.5 to 60 GHz, at a Courant number of 1 (largest 2.3 × 10⁻⁴ and 3.5 × 10⁻⁴ rad,
  0.24 and 0.34 of the curve) and of 0.5 (0.33 and 0.52 of it), and fall by 4.00 at half the cell.
  The paper's errors are 5 to 8 times these at their largest; with its time step and run unknown
  that isn't explained here, nor are its curves' ripples, which its Debye curve doesn't have.

**Conductivity** goes in first (`with_conductivity`, then `with_medium`) and stays in the
medium. A medium put over another replaces it. A total-field/scattered-field surface must not
cross a dispersive medium; a box around one is fine.

**The catalogue's materials.** `Dispersive::from_model` takes a model whose terms are Lorentz or
Drude terms exactly: a Drude or Lorentz model, a real constant, and a Sellmeier formula, each
term $B\lambda^2/(\lambda^2 - C^2)$ being $\Delta\varepsilon = B$ undamped at $f_0 = 1/C$ (to
round-off, a unit test). A Sellmeier term far in the ultraviolet may need a smaller step than
the grid's ($\omega_0\Delta t \le 2$): fit it instead. `Dispersive::fit` fits any material, a
table or a formula, over a band:

- $\varepsilon_\infty \ge 1$ and Lorentz terms of nonnegative strength at fixed resonances, by
  nonnegative least squares (Lawson and Hanson, Ch. 23) over 64 frequencies across the band, so
  every fit is passive, and stable wherever its resonances are; its largest error over 256 is
  stated (`Fit::error`, `Fit::relative`).
- Where the material is transparent over the band, undamped terms at 1.25 to 4 times the
  band's highest frequency and 0.8 to 0.25 times its lowest: a Sellmeier formula. Silicon (Li
  1980's table) over 1.2 to 1.7 µm and silica (Malitson 1965) over 0.4 to 1.6 µm each take two
  terms, within 6.0e-5 of $\varepsilon$ (`fdtd/fit-lossless`).
- Where it absorbs, damped terms from half the band's lowest frequency to twice its highest at
  three dampings each: In₀.₄₉Ga₀.₅₁P above its gap (Ferrini 2002's n and k) over 0.4 to 0.6 µm
  takes eight, within 8.1e-3 (`fdtd/fit-lossy`).
- A band beyond the material's data is refused, as the material refuses those wavelengths.
- A silicon slab 0.4 µm thick, fitted over 1.2 to 1.7 µm, reflects as Airy's formula with the
  catalogue's own index at 1.3, 1.45 and 1.6 µm, to second order in the cells: 2.9e-2 at 20 nm,
  7.3e-3 at 10 and 2.0e-3 at 5 (`fdtd/fitted-slab`).

Debye terms aren't needed for the catalogue's materials, and aren't there.

## Sources

**Waveforms.** Every source takes its values in time from a waveform:

- a Gaussian pulse on a carrier, `Waveform::pulse(frequency, bandwidth)` with the power
  spectrum's full width at half maximum;
- a differentiated Gaussian, which has no DC, as Roden and Gedney's source;
- a continuous wave, switched on smoothly.

Each has an analytic form w(t), whose real part is the waveform: a current of complex amplitude
a takes the values Re(a w(t)).

**Normalization, exactly.** The leapfrog is linear and time-invariant, so each field's discrete
Fourier transform, taken at its own times ($E$ at $n\Delta t$, $\tilde H$ at $(n + \tfrac12)\Delta t$),
solves FDFD's equations at the leapfrog's frequency $\tilde\omega$ for the transforms of the
currents, taken at the times they enter (J at $(n + \tfrac12)\Delta t$, M at $n\Delta t$). That is
exact, not approximate, once the fields have died away. `Waveform::spectrum` is that transform
of a source's values, so the field per unit source spectrum is the field's transform divided by
it. A pulse from an electric point current and from a magnetic dipole in a lossy box gives
`Solver3d`'s field at $\tilde\omega$ for those spectra to 6.5e-13, at the carrier and off it
(`fdtd/spectrum`). For a complex amplitude a, the values Re(a w) transform as
$\tfrac12(a W(\omega) + \bar a\thinspace W(-\omega)^*)$. Near the carrier the second term is the
negative frequencies' image, $e^{-(\omega\tau)^2}$ of the first for a Gaussian of width τ.

**Dipoles and currents.** A `Current` sets complex amplitudes on many values of E (J) or H̃ (M)
with one waveform. A `Dipole` sits anywhere, not only on a value: Oskooi et al. restrict it to the
grid with the weights of trilinear interpolation, the transpose of interpolation, which keeps the
current's integral (their Section 3 and Fig. 4). Its amplitude is $\int J\thinspace dV$, over the
cross-section in a 2D problem.

**Total-field/scattered-field** (Umashankar and Taflove, Section II.A). A closed surface splits
the grid. Inside, the total field runs; outside, only the scattered field. Where an update's curl
reaches across the surface, the incident field at the value across is added (into the total
region) or subtracted (into the scattered one). Those corrections are currents on the surface:
J from the incident H̃, M from the incident E. They are exact when the incident field solves the
grid's own equations there, and then the scattered region sees nothing of it but round-off.
FDFD's ports use the same splitting in the frequency domain (Rumpf, Eq. 55).

**Plane waves.** `Simulation::add_plane_wave` puts a plane wave on a box. It travels along
$(p/\Delta x, q/\Delta y, s/\Delta z)$ with p, q and s integers. A field that depends only on the
position along that direction is a function of $\ell = \sum_i p_i\thinspace(2 r_i/\Delta_i)$, an
integer at every value of the grid, staggering included. Yee's update of such a field is an update
of a 1D field of ℓ: its differences along x reach $\ell \pm p$, along y $\ell \pm q$, along z
$\ell \pm s$. The incident field comes from that 1D update, run alongside the grid with the same
step:

- a sheet of current upstream starts the wave, at $E = -K/(2n)$ for a sheet K, so its E is the
  waveform times the polarization to second order in the cells (1.2e-2 off at 20 cells a
  wavelength, 7.7e-4 at 80);
- a matched absorber ends it at each side;
- `Simulation::incident` reads the field the box sees.

The box's corrections then hold at every frequency at once, at normal and oblique incidence
alike. With nothing inside, the scattered region holds 1.3e-15 of the incident field: in 2D along
(1, 0), (2, 1) and (1, −3), in 3D along (0, 0, 1) and (2, 1, 1) (`fdtd/tfsf-leakage`). A slab
inside reflects as Airy's formula says, to second order in the cells: 5.4e-3 at 20 nm, 1.3e-3
at 10 and 3.3e-4 at 5 (`fdtd/tfsf-slab`).

The [`tfsf_square_cylinder`](../../examples/tfsf_square_cylinder.rs) example reproduces
Umashankar and Taflove's square conducting cylinder (k₀A = 1, 20 cells a side). Its surface
current matches their Fig. 5(a) on the lit face (1.732, theirs 1.750) and the side faces (0.764,
theirs 0.785). At 40 cells a side it is within 0.4 % of their moment method, Eq. 8a, solved in
the example. On the shadowed face both give 0.17, and the figure's curve, 0.207, is 21 % above
the converged solution of its own equation.

**Modes.** `Simulation::add_mode_source` launches a mode of `PortMode3d`, the grid's own, one way
from its plane. The surface is the plane. The incident field is the mode carried along its axis
by $e^{\pm i\beta\Delta a}$ a step, and its H̃ is the grid's curl of its E. Going forward the total
field is from the plane on; going backward, up to the plane after it, as FDFD's `mode_source`.

- **Its frequency:** the mode must be FDFD's on the same grid and medium at $\tilde\omega$ for the
  waveform's carrier (`Simulation::fdfd_wavelength`). The source refuses one solved at ω itself.
- **Against FDFD:** in a lossy closed box the run settles to `Solver3d`'s field for its own mode
  source to 2.5e-14 (`fdtd/mode-source-fdfd`).
- **One way:** in an open guide, 1.5e-7 of the power goes backward (`fdtd/mode-source-backward`).
  The rest is the CPMLs, which are FDFD's PMLs only to first order in Δt. The forward flux is
  1.000048 of the mode's power (`fdtd/mode-source-power`).
- **A pulse** launches every frequency with the carrier's mode, and what goes back grows away
  from the carrier as the mode changes. For a pulse 0.1 c/µm wide on the same guide:

  | Frequency | Backward / forward power | Forward / mode's power |
  |---|---|---|
  | carrier − 0.05 c/µm (−7.8 %) | 4.1e-4 | 0.9968 |
  | carrier, 1/1.55 c/µm | 1.5e-7 | 1.000047 |
  | carrier + 0.05 c/µm (+7.8 %) | 3.8e-4 | 0.9989 |

**Gaussian beams.** `Simulation::add_beam` launches a beam one way from a plane, for grating
couplers. On the plane, E's tangential components are the paraxial beam's: waist, focus and
tilt, with its curvature and Gouy phase; in 2D, the 2D beam's $\sqrt{w_0/w}$. That field is
decomposed into the grid's own plane waves across the plane's window, a discrete Fourier series.
Each wave's normal wavenumber comes from the grid's dispersion at $\tilde\omega$, and its normal E
makes it divergence-free on the grid. On the plane the field is the beam's exactly. Off it, the
beam propagates as the grid propagates that field, not as the paraxial approximation does, and the
waves solve the grid's equations to 8e-15 in 3D (a unit test). For a 2D beam of
$w_0 = 1.5\lambda$ at 20 cells a wavelength:

- **Waist:** 1.49999 against 1.5 (`fdtd/beam-waist`), the focus 1.94 µm from the plane (2 asked).
- **Divergence:** the width grows exactly as $w^2 = w_0^2 + \theta^2 d^2$ for a sum of plane
  waves. θ is 0.218825, the grid's own prediction to 2.7e-6 (`fdtd/beam-divergence`). The
  paraxial $\lambda/(\pi w_0) = 0.2122$ is 3.1 % below: the beam isn't paraxial at
  $w_0 = 1.5\lambda$ (the continuum's exact 0.2160, 1.8 %), and the grid's dispersion adds 1.3 %.
- **Tilt:** tilted by 10°, the centre moves at 0.1825 µm per µm, to 1.5e-4 of the grid's
  prediction (`fdtd/beam-tilt`), above $\tan 10° = 0.1763$ by the same two effects.

## Probes and monitors

A probe records one value of E or H̃ every step. A monitor accumulates what a run gives back as
it goes, so a run needs no stored history:

- **Transforms** (`Simulation::add_dft`): every component of E and H̃ over a box of values,
  $\sum_n F(t_n) e^{i\omega t_n}\Delta t$ at chosen frequencies, each field at its own times
  (E at $n\Delta t$, H̃ at $(n - \tfrac12)\Delta t$). Each value's sum is its own, in step order,
  so the transforms are the same bits on any number of threads. Divided by the source's
  `Waveform::spectrum` once the fields have died away, they are FDFD's fields at the leapfrog's
  frequency $\tilde\omega = (2/\Delta t)\sin(\omega\Delta t/2)$. H̃'s transform is then
  $\nabla \times \hat E/(i\tilde\omega)$ exactly: summing $\tilde H^{n+1/2} - \tilde H^{n-1/2} =
  -\Delta t\,\nabla \times E^n$ against $e^{i\omega n\Delta t}$ gives $-2i\sin(\omega\Delta t/2)\hat H
  = -\Delta t\,\nabla \times \hat E$. Complex runs (a Bloch phase) transform the whole field.
- **Flux** (`add_flux`, `add_flux_box`): $\tfrac12\operatorname{Re}(E \times \tilde H^*)$ through
  a plane halfway between two planes of nodes, where H̃'s tangential components are, the
  tangential E the mean of its values on the nodes either side: FDFD's `Field3d::flux`, now
  within a window. It is exact for the scheme. Summation by parts of
  $\sum \tilde H^*\cdot(\nabla \times E) - \sum E\cdot(\nabla \times \tilde H)^*$ over a box, each
  value weighted by its share of it (1 inside, ½ on its boundary), leaves only pairs of values
  coupled across the boundary, and those are the faces' fluxes in this form, with the values on
  a face's edges counting half. In a lossless region without sources both sums are imaginary
  ($i\tilde\omega\sum\lvert\tilde H\rvert^2$ and $i\tilde\omega\sum\varepsilon\lvert E\rvert^2$),
  so the flux out of a closed box is zero to round-off: 1.5e-15 of a face's flux
  (`fdtd/monitor-flux-box`). The plane's and the box's flux equal FDFD's for the same current to
  1.2e-11 (`fdtd/monitor-flux`).
- **Mode amplitudes** (`add_mode_monitor`): a guide's modes from FDFD, each projected by FDFD's
  own Lorentz reciprocity form on its plane and the next, at the frequency whose $\tilde\omega$
  is the mode's $k_0$ (`Simulation::leapfrog_frequency`, $\omega = (2/\Delta t)\arcsin(k_0\Delta
  t/2)$). With a mode source they give S-parameters. In a closed lossy box they are FDFD's to
  1.5e-11 (`fdtd/monitor-modes`). Open, in 2D with CPMLs of 10 cells against FDFD's PMLs, a
  straight guide of ε = 12 transmits 0.9999991 against 1.0000002, and a sharp 90° bend 0.315408
  against 0.315415, the reflections agreeing to 1.4e-5 and 5.8e-5 (`fdtd/monitor-guides`): the
  CPML and FDFD's PML absorb differently in discrete time.
- **Stopping** (`run_until_decayed`): a run goes on in blocks until, over a whole block, |F|²
  at each of a set of probes has stayed below a fraction of its peak, or until a time limit,
  and says which. A fixed time is `run_until`.

## Resonances

A resonance's frequency and Q come from a probe's time series after the source has died, by
harmonic inversion (`harmonic_inversion`): V. A. Mandelshtam and H. S. Taylor's filter
diagonalization (J. Chem. Phys. 107, 6756 (1997), doi:10.1063/1.475324, Section II). The signal
$c_n = \sum_k d_k e^{-in\tau\omega_k}$ is the correlation function of an evolution operator
whose eigenvalues are $u_k = e^{-i\tau\omega_k}$. On a basis $\Psi(z_j) = \sum_{n=0}^{M}
(\hat U/z_j)^n\Phi_0$ for a few points of the window on the unit circle (their Eq. 19), the
operator's matrices $U^{(p)}(z, z')$ are sums of the signal alone (Eq. 25, checked against
the double sum it closes), and $U^{(1)}B = u\,U^{(0)}B$ (Eq. 23) gives the $u_k$ near the
window. photonoxide follows their summary:
- $J = \lceil N\tau(\omega_\text{max} - \omega_\text{min})/4\pi\rceil$ points, at least 8;
- $M = \lfloor(N - 3)/2\rfloor$;
- the eigenproblem on $U^{(0)}$'s singular vectors above 1e-11 of the largest;
- each $u_k$ checked by $\lVert(U^{(2)} - u_k^2U^{(0)})B_k\rVert$ relative to $\lVert u_k^2U^{(0)}B_k\rVert$ (step 5);
- the amplitudes from the whole signal (Eq. 27).

Every eigenvalue in the window is returned with that error, and the caller keeps what it trusts.

A signal of finitely many terms is recovered to round-off, and the resolution is not the
transform's 2π/T:
- **Synthetic:** two terms 0.01 c/µm apart, where 400 samples resolve 0.025, are found with
  frequencies, decay rates and amplitudes to 2.1e-12 (`fdtd/harmonic-inversion`).
- **A lossy cavity:** in a 2D cavity with walls, each mode $\sin(m\pi i/n_x)\sin(n\pi j/n_y)$
  evolves as $u^n$, with $u^2 - (1 + c_a - c_b\Delta t\lambda)u + c_a = 0$ from E's and H̃'s
  updates, where λ is the discrete curl-curl's eigenvalue. The four modes found agree to
  3.4e-15 (`fdtd/cavity-resonances`).
- **Convergence:** a slab of ε = 4, 0.5 µm thick, in vacuum, against the continuum's pole
  $\omega = (2\pi + i\ln r)/(nL)$, r = 1/3 (1 c/µm, Q = 2.86). Its second resonance's frequency
  converges at second order, 1.8e-3, 4.5e-4 and 1.1e-4 on cells of 20, 10 and 5 nm, and so does
  its decay rate, 1.4e-2, 3.6e-3 and 8.9e-4 (`fdtd/slab-resonance`).

**A ring's resonances, exactly.** Oskooi et al. 2010's Fig. 11 excites "a dielectric ring
resonator" of ε = 11.56 but gives neither its size nor a number, and their Fig. 9 gives only
Q ∼ 10⁶ for a missing rod in a square lattice of rods (r = 0.2a, ε = 12), with no crystal size.
A 2D ring has an exact solution instead, against which a run's modes and Q's are checked
(`fdtd::ring`, crate-private). With $E_z$ along the axis, $E_z = u(r)e^{im\phi}e^{-i\omega t}$ is
$J_m(kr)$ in the hole, $J_m(nkr)$ and $Y_m(nkr)$ in the ring and the outgoing $H^{(1)}_m(kr)$
outside; continuity of $E_z$ and $\partial E_z/\partial r$ at both faces leaves a 2 × 2
determinant, each row multiplied through by the outer function so that it has no poles, whose
zeros $\omega_r - i\gamma$ are the resonances, $Q = \omega_r/2\gamma$, found by the secant
method. $J_m$ and $Y_m$ of complex argument come from their power series (M. Abramowitz, I. A.
Stegun, *Handbook of Mathematical Functions*, NBS (1964), Eqs. 9.1.10 and 9.1.11), which lose
about $e^{\lvert z\rvert}/2\pi\lvert z\rvert$ to cancellation and are kept to |z| ≤ 15: their
Wronskian is $2/\pi z$ to 1.0e-10 (`fdtd/ring-wronskian`).

The ring between radii 1 and 2 µm, smoothed, kicked by a pulse at 0.15 c/µm and recorded for 300
µm/c, has three resonances from 0.1 to 0.2 c/µm, m = 3, 4 and 5 at 0.118192, 0.147431 and
0.175779 c/µm with Q = 77.26, 343.92 and 1634.2. On 20 cells a µm FDTD finds them within 2.8e-4,
5.2e-4 and 7.7e-4 in frequency and 0.13 %, 0.37 % and 0.73 % in Q (`fdtd/ring-resonances`), and
both errors fall at second order from 10 cells (`fdtd/ring-order`): $E_z$ lies along every face,
where the smoothed ε is the cell's mean, the right average for it.

## Scattering by a sphere

**Mie's series.** A plane wave on a sphere has an exact solution, G. Mie's (Ann. Phys. 330, 377
(1908), doi:10.1002/andp.19083300302), written here from his paper (`fdtd::Mie`, `fdtd::Sphere`).
A sphere of radius ρ in a lossless medium of index n₀ is described by his size parameter
$\alpha = 2\pi n_0\rho/\lambda$ and its relative index m. The scattered field is a series of
outgoing partial waves, the electric ones weighted by his $a_\nu$ and the magnetic by $p_\nu$
(his Eq. 55). The power they carry away and the power they take from the incident wave are his
§26's parts III and II, the scattering and extinction cross-sections.

Mie writes fields as $e^{+i\omega t}$, so his metals have $m' = n - i\kappa$. His functions are
his own: $I_\nu(x)$ (his Eq. 25) is the Riccati–Bessel $\psi_\nu(x) = x j_\nu(x)$, and his outgoing
$K_\nu(-x)$ (Eq. 19) is $(-i)^{\nu+1} x h^{(2)}_\nu(x)$. In photonoxide's $e^{-i\omega t}$, with
$\xi_\nu = x h^{(1)}_\nu = \psi_\nu - i\chi_\nu$ and $D_\nu = \psi_\nu'/\psi_\nu$, his Eq. 55,
divided through by $\psi_\nu(m\alpha)$, is

$$
a_\nu = \frac{(D_\nu(m\alpha)/m + \nu/\alpha)\psi_\nu(\alpha) - \psi_{\nu-1}(\alpha)}{(D_\nu(m\alpha)/m + \nu/\alpha)\xi_\nu(\alpha) - \xi_{\nu-1}(\alpha)},\qquad
b_\nu = \frac{(mD_\nu(m\alpha) + \nu/\alpha)\psi_\nu(\alpha) - \psi_{\nu-1}(\alpha)}{(mD_\nu(m\alpha) + \nu/\alpha)\xi_\nu(\alpha) - \xi_{\nu-1}(\alpha)},
$$

his own coefficients being $(2\nu+1)(-1)^\nu i\bar a_\nu$ and $-(2\nu+1)(-1)^\nu i\bar b_\nu$, the bar
the conjugate with m conjugated. His sums become

$$
Q_\text{sca} = \frac{2}{\alpha^2}\sum_\nu (2\nu+1)\left(\lvert a_\nu\rvert^2 + \lvert b_\nu\rvert^2\right),\qquad
Q_\text{ext} = \frac{2}{\alpha^2}\sum_\nu (2\nu+1)\operatorname{Re}(a_\nu + b_\nu),
$$

the cross-sections over πρ², and absorption is their difference. A perfect conductor is the limit
m → ∞ (his §17).

Every function follows from his recurrence (26), $(2\nu+1)f_\nu/x = f_{\nu-1} + f_{\nu+1}$:
- $\xi_\nu$, which grows past ν ≈ α, is stepped up from $\xi_0$ and $\xi_1$;
- $\psi_\nu$, which falls there and can't be stepped up, comes from $D_\nu$ stepped down,
  $D_{\nu-1} = \nu/z - 1/(D_\nu + \nu/z)$, from zero well above the last term, and then
  $\psi_\nu = \psi_{\nu-1}/(D_\nu + \nu/z)$;
- the sphere's own $\psi_\nu(m\alpha)$ never appears, only $D_\nu(m\alpha)$, so a metal's
  exponentially large values can't overflow.

The terms fall faster than exponentially once ν passes α. The series is summed until a term adds
less than 1e-17 of the sum.

**Checked against Mie.** His Table I gives $\mathfrak{a}_1 = a_1/2\alpha^3$ for a perfectly
conducting sphere and for gold spheres in water at seven wavelengths, from 420 to 650 nm, with
gold's $m'^2$ in his table on p. 417. Of its 66 entries, 61 agree to within 0.016, his three
digits by hand; the median difference is 3.0e-3 (`fdtd/mie-table`). The other five lie near gold's
resonance, where his series in α² converge worst. They differ from his by 0.04 to 0.40, and the
series agrees there with $a_1$ from the closed forms $\psi_1 = \sin z/z - \cos z$ and $\xi_1$ to
1e-13. The series is also checked against itself:
- a lossless sphere takes what it scatters, $Q_\text{ext} = Q_\text{sca}$, to 1.4e-13 for α from 0.1
  to 1000 (`fdtd/mie-balance`);
- the first $\alpha + 4\alpha^{1/3} + 10$ terms give $Q_\text{ext}$ to 7e-14 for α up to 200 (round-off), half
  as many off by more than 1e-2 (`fdtd/mie-terms`);
- Rayleigh's limits, (8/3)α⁴|(m² − 1)/(m² + 2)|² and (10/3)α⁴ for a perfect conductor, with the
  next term of order α²;
- $Q_\text{ext} \to 2$ for a large sphere, and a good conductor tends to a perfect one.

**FDTD against Mie.** A sphere of radius 1 µm in vacuum is lit along x by a TF/SF plane wave
polarized along z, a pulse over α = 1 to 3. A flux box in the scattered field, around the TF/SF
box, gives the scattering cross-section, and a box in the total field, around the sphere, the
absorption. The incident intensity comes from the same plane wave on a grid one cell across,
periodic, at the same step.
- **Smoothed,** ε = 4 (`Average::Subpixel`, the entries at the nodes): the mean relative error of
  $C_\text{sca}$ over the band is 5.1e-2, 2.8e-2, 1.2e-2, 7.2e-3 and 4.7e-3 on 6, 8, 12, 16 and
  20 cells a radius. That is second order (1.97 from 8 to 16 cells, `fdtd/mie-sphere`), as the
  scheme's dispersion, which dominates at the top of the band (α = 3 is 6 cells a wavelength
  in the sphere on the coarsest grid). The lossless sphere's absorption box reads below 1e-3 of
  the scattering.
- **Sampled,** ε at each value of E: 2.6e-2, 1.7e-2, 8.1e-3, 6.0e-3 and 4.8e-3
  (`fdtd/mie-sphere-staircase`). Irregular in the grid, and not larger than the smoothed sphere's
  here: the staircase's error partly cancels the dispersion's.
- **A damped Drude metal,** f_p = 0.5 c/µm, γ/2π = 0.2 c/µm (Re ε from −2.8 to 0.3, Im ε from 4.8
  to 0.5 over the band), sampled at each value of E, against Mie with the leapfrog's
  permittivity: the mean relative error of both cross-sections is 3.0e-2 and 1.6e-2 on 8 and 16
  cells a radius, first order, as a staircased surface (`fdtd/mie-drude`).

A metal sphere with a sharp plasmon in the band is not yet converged. With γ/2π = 0.02 c/µm the
dipole and quadrupole plasmons fall at α ≈ 1.5 to 2, and on 16 cells a radius the staircased
sphere absorbs up to 2.7 times Mie's there, falling slowly with the grid. A plasmon is a
surface mode, and a dispersive medium is sampled, not smoothed: its ε∞ would have to enter the
smoothed tensor.

## Agreement with FDFD over a band

One pulsed run gives a guide's S-parameters at every frequency of its band; FDFD gives them one
frequency at a time. On the same grid they solve the same equations at the leapfrog's frequency
$\tilde\omega$, so they are compared exactly there (`fdtd::agreement_checks`): the run's mode
source launches the carrier's mode at every frequency, and FDFD is given the run's own currents,
each J and M times its transform at the times it is applied
(`Waveform::analytic_spectrum`), at each frequency's $\tilde\omega$. The amplitudes of each
frequency's own modes, forward and backward on a reference plane after the source and forward
on the output's plane, give $S_{21} = a^+_\text{out}/a^+_\text{in}$ and $S_{11} =
a^-_\text{in}/a^+_\text{in}$ by FDTD's mode monitors and by FDFD's projection. What is left is the
CPML against FDFD's PML (κ = 1, α = 0: the same stretch, in discrete time), which differ in what
they reflect:

- **2D,** a guide of ε = 12 and 0.3 µm in air with E in the plane, straight and bent around a
  quarter circle of 1 µm, five frequencies across ±5 % of 1.55 µm, CPMLs of 0.5 µm: 3.2e-5 on
  50 nm cells (`fdtd/fdfd-band-2d`), 1.7e-7 on 25 (`fdtd/fdfd-band-2d-fine`), as the same
  thickness takes more cells and both reflect less.
- **3D,** a strip of ε = 12, 0.4 × 0.25 µm, in ε = 2.1, on 50 nm cells, three frequencies:
  straight, 2.2e-4 with CPMLs of 8 cells (`fdtd/fdfd-band-3d-strip`), 6.5e-5 with 10 and 7.9e-6
  with 12 (an ignored test); bent around a quarter circle of 0.6 µm, 1.9e-4
  (`fdtd/fdfd-band-3d-bend`), $\lvert S_{21}\rvert$ 0.78 to 0.85.
- **With subpixel smoothing,** the 2D bend smoothed by the triplets for FDTD and averaged as
  FDFD averages it: the two differ at the bend, and their difference falls as the grid shrinks,
  0.56, 0.31 and 0.16 on 50, 25 and 12.5 nm cells (orders 0.85 and 0.97,
  `fdtd/fdfd-smoothed`), mostly $S_{21}$'s phase over 3.6 µm of guide, $\lvert S_{21}\rvert$
  within 7e-4. Both converge to the continuum, FDFD's average first order at the curved faces,
  as is the triplets'.

## Meep's published cases

A. F. Oskooi et al.'s paper on Meep (Comput. Phys. Commun. 181, 687 (2010),
doi:10.1016/j.cpc.2009.11.008) is reproduced from the paper alone (Meep's code is GPL and isn't
read). Its figures give convergence rates rather than numbers to many digits; those with a
number are reproduced:

- **Fig. 8, a PML's reflection** (the `pml_oskooi` example, `fdtd/meep-pml-rates`): with σ
  graded as $(x/L)^d$ at a fixed round-trip reflection, the change of $E_z$ from a point source
  when the PML is a wavelength thicker falls as $1/L^{2d+4}$, at 20 pixels a wavelength. Here,
  in a cell of 4 wavelengths with R = 1e-15, the rates between consecutive differences, each at
  its midpoint, approach 6, 8 and 10 from above: 6.14, 8.22 and 10.32 at L = 4, 6.03, 8.05 and
  10.08 at L = 8.
- **Fig. 7, a bump on a guide** (the `bump_oskooi` example): the power a semicircular bump on a
  guide of ε = 12 scatters from a dipole converges "roughly second-order" with smoothing and
  "only first-order" without. With the bump, the frequency and the polarization read off the
  figure's inset (radius 0.25a, 4.5a from the dipole, f = 0.2 c/a, $E_z$), from 10 to 30 pixels
  a against the smoothed at 50: least-squares orders 2.37 and 1.07, smoothing better at every
  resolution, its error 7.5e-2 at 10 pixels (the paper's 0.08).
- **Figs. 3, 6, 9, 10, 11 and 12** give no number to check. Fig. 6's anisotropic ellipsoids and
  Fig. 11's ring are not sized in the paper; Fig. 9 gives Q ∼ 10⁶. The ring of Fig. 11 is
  checked against its exact resonances instead (above, "Resonances").

## Six devices against Lumerical FDTD and Tidy3D

Z. Liu and J. K. S. Poon (Opt. Continuum 4, 2427 (2025), doi:10.1364/OPTCON.572107) simulate
six devices of gdsfactory's generic PDK in 3D in both commercial codes, at 6 to 25 cells a
wavelength in silicon, and print or plot what each gives. The `*_liu_poon` examples run the same
devices here (`examples/pdk`):

- **The shapes** are gdsfactory's, drawn from its definitions (Bézier S-bends, Euler bends with
  p = 0.5 on the arc's footprint, tapers) and checked against the paper's own GDS files: every
  vertex within 1.1 nm. The guides run 10 µm past the ports, through the CPMLs, as the paper
  extends them.
- **The stack:** 220 nm of silicon (Li's, 3.4757 at 1550 nm), the crossing's slab 150 nm, in
  silica (Malitson's); the splitter-rotator under silicon nitride of n = 2.0. Not dispersive:
  each run takes its materials at one wavelength. The paper's Palik silicon is 3.4738 in
  Tidy3D's fit and about 3.4764 in Lumerical's data.
- **The cell** is the paper's in the plane (the ports and the device, 1 µm on every side); along
  z 1 µm of cladding either side, where the paper has 2, CPMLs of 12 cells outside.
- **The grid** is uniform: N cells a wavelength is h = 1.55 µm/(3.4757 N), 29.7 nm at 15 and
  22.3 nm at 20. The paper's grids are non-uniform, as fine in the silicon and coarser outside.
- **The smoothing** is `Coupling::Diagonal`: a permittivity per component of E, as the paper's
  conformal (Lumerical) and subpixel (Tidy3D) meshes take it, and the kernel's diamonds, about
  four times the speed of a coupling tensor.
- **Sources and monitors:** the input's mode at 1550 nm launched by a pulse 0.04 c/µm wide on its
  extension; each output's modes at 1540 to 1560 nm, solved by FDFD on the grid at each
  frequency, projected on planes 0.3 µm past the ports; transmissions over the incident mode's
  power at the same frequency. The run stops when the outputs' |E|² has stayed below 1e-6 of its
  peak for 20 µm/c.

At 1550 nm, against the span of both codes' values where the paper finds them settled (read off
its figures, the reading's uncertainty stated in each example):

| Device | At 1550 nm | Here, 15 cells (29.7 nm) | Here, 20 cells (22.3 nm) | Lumerical, 15 / 20 / 25 | Tidy3D, 15 / 20 / 25 |
|---|---|---|---|---|---|
| crossing | through TE₀ | 0.95666 | 0.95863 | 0.957 / 0.957 / 0.957 | 0.959 / 0.9558 / 0.9567 |
| crossing | excess loss | −0.192 dB | −0.183 dB | −0.1925 / −0.191 / −0.1915 | −0.183 / −0.1965 / −0.1925 |
| directional coupler | cross TE₀ | 0.42771 | 0.41352 | 0.411 / 0.446 / 0.430 | 0.448 / 0.492 / 0.456 |
| directional coupler | excess loss | −0.0014 dB | −0.0017 dB | 0.000 / 0.001 / 0.000 | −0.013 / −0.002 / −0.007 |
| 2 × 2 MMI | cross TE₀ | 0.48148 | 0.48743 | 0.483 / 0.486 / 0.489 | 0.479 / 0.484 / 0.489 |
| 2 × 2 MMI | excess loss | −0.140 dB | −0.120 dB | −0.14 / −0.13 / −0.12 | −0.155 / −0.145 / −0.125 |

Every one within the span of the codes' settled values and the reading. The spectra over 1540
to 1560 nm follow the paper's at 15 cells (the crossing's and the MMI's between the two codes' at both
ends). At 20 cells the crossing's through port is 0.002 higher across the
band, more than either code moves from 15 to 20 cells; the paper gives no band at 20, and the
difference is followed in issue #256.

The mode converter, the polarization splitter-rotator and the ring run in CI on coarse grids
only; their runs at the paper's grid (81 to 92 million cells, two to four hours each on 20
threads) are issue #256's. At 5 cells a wavelength the splitter-rotator turns 64 % of its TM₀
into the upper port's TE₀ (the codes, at 6: 15 % and 5 %), and the mode converter 46 % of its TE₀
into TE₁ (the codes at 6: 97 % and 36 %; settled, 36 % to 52 %). The ring's coupling across its
200 nm gap needs a fine grid: at 6 cells its Q is about 8000 against the codes' 1700 to 1800, so
CI checks its free spectral range instead, 7.54 nm from the strip's group index (4.1792, with
silicon's and silica's dispersion) and its 75.747 µm, against the text's "around" 7.4 and 7.6.

Run times on 20 threads of a Core Ultra 7 265K, against the paper's at the same resolution
(Tidy3D in the cloud; Lumerical on an AMD 3960X and on its cloud GPUs):

| Device | Cells, steps | Here (stepping, mode solves) | Tidy3D | Lumerical, local / cloud |
|---|---|---|---|---|
| crossing, 15 | 14.1 million, 14 124 | 8 min, 3 min | 29 s | 234 s / 15 s |
| crossing, 20 | 30.4 million, 18 840 | 22 min, 6 min (shared) | 104 s | 591 s / 23 s |
| coupler, 15 | 37.0 million, 16 478 | 63 min, 2 min (shared) | 63 s | 1293 s / 42 s |
| coupler, 20 | 80.0 million, 21 980 | 111 min, 3 min (shared) | 150 s | 2606 s / 77 s |
| MMI, 15 | 32.0 million, 17 655 | 26 min, 2 min | 45 s | 905 s / 34 s |
| MMI, 20 | 69.3 million, 23 550 | 96 min, 3 min (shared) | 63 s | 2182 s / 72 s |

On grids of 14 to 32 million cells the kernel made 358 to 436 million cell-updates/s with the
monitors; "shared" runs had other work on the machine and made 160 to 280. The paper's grids are
coarser in the cladding (Lumerical's crossing at 25 cells has 4.55e7 cells to our 14.1e6 at 15),
and its GPUs are many.

## Cost

Every update is a fixed stencil with no reduction, its rows shared among rayon's threads in chunks
of about 4096 values fixed by the grid (so a 2D grid, one plane thick, is shared as a 3D one is),
so the fields are the same bits on any number of threads.

**The kernel** (`src/fdtd/kernel.rs`) runs in f32 or f64.
- **Rows:** it goes through the grid row by row along x. Along a row, the neighbours along y and
  z are whole rows away, the same offset for every value, and those along x are the next value
  except at the row's ends. So each row is plain loops over slices of equal length, which the
  compiler vectorizes over x. The ends, where a wall, a periodic side or a Bloch side decides the
  neighbour, are done one value at a time.
- **One pass per field:** the three components of a field are updated in one pass, each row of
  the other field read once.
- **CPMLs:** each CPML slab updates ψ and the field it corrects row by row in one pass.
- **Tables:** the neighbours' offsets and the 1/(κΔ) factors are tabulated once, and nothing is
  allocated while stepping.
- **Monitors:** a transform monitor adds, each step, a complex multiply-add per value and
  frequency, in parallel over chunks of its rows.
- **The same bits:** every value is computed by the same operations in the same order as the
  plain loops it replaced, which are kept as its reference. IEEE 754 makes a vector lane's sum
  or product the scalar one, so in f64 the fields are the same bits as the plain loops. That is
  tested with CPMLs (κ and α), walls, periodic sides, 2D, rows of one value, a Bloch phase, a
  Drude medium and a smoothed tensor.
- **f32:** E stays within about 1e-8 of f64 per step, relative to the largest E. On random
  fields in a box with a block of ε = 12 the error is 1.8e-6, 4.5e-6 and 1.5e-5 after 100, 400
  and 1600 steps. f32 is also the same bits on any number of threads.

Against the plain loops, on a 4-core cloud container (STREAM triad 47 GB/s) with a 128³ guide
of 20 nm cells and CPMLs of 8, the kernel stepping the whole grid made 38.1 million
cell-updates/s on 1 thread and 124.0 on 4, 2.8 and 2.3 times the plain loops' 13.4 and 53.7.

**The memory's roof.** A step reads and writes at least 192 bytes a cell in f64 (96 in f32):
each field's three components read and the updated one written, E's two coefficients, and more
in the CPMLs. Stepping the whole grid every step, a grid beyond the caches is bound by memory,
not arithmetic: on 20 threads of a Core Ultra 7 265K (8 performance and 12 efficient cores,
30 MB of L3, STREAM triad 33.0 GB/s on 1 thread and 55.1 on 20) it runs at the triad's roof,
and AVX2 changes nothing measurable. In cache the same kernel is about eight times faster:
twenty 64 × 16 × 16 periodic grids stepped side by side made about 2000 million
cell-updates/s in f64 on 20 threads, 280 on one.

**Blocking** (`src/fdtd/kernel/blocked.rs`) steps a tile of the grid while it is in cache,
after T. M. Malas et al. (2015, 2016), who tiled a Yee stencil like this one.
- **The tiles:** whole rows along x (the fastest axis, left whole as Malas et al. leave it, so
  the rows stay long for the vector units), a run of rows along y, every plane along z.
- **One step at a time** (spatial blocking): each tile's H̃ then E, plane by plane, so E's rows
  are still in cache when H̃'s update has read them. Every row but those along the tiles' edges
  is done by its own tile; the edges' rows by a second pass.
- **Several steps at a time** (temporal blocking): Malas et al.'s diamonds in the plane of y and
  time (their Fig. 2), each stepped as a wavefront along z: plane k at step m right after plane
  k + 1 at step m − 1, each plane's H̃ then E. Over s steps a tile's rows shrink by one at each
  inner side each step (H̃ loses the last row, E the first), so it reads nothing another tile
  writes: a diamond's upper half. The rows it leaves around each boundary grow by one a side
  each step: the lower half of the next row of diamonds, centred on the boundary and stepped
  with the next s steps' upper half as one task. The diamonds of a row are apart and run side by
  side, one a thread. A tile's data is read from memory once every s steps and stepped s times
  in cache; split tiling (each half a phase of its own, the same tiles every block) reads it
  twice, and measured about 10 % slower.
- **The same bits:** in place, a value may be updated once its neighbours hold the level before,
  and they can't hold a later one, since that would have needed this value first. So any order
  that respects the stencil's dependencies leaves every value as the whole grid's step does, and
  the tiles compute each row by the same function as the whole grid (`curl_row`, `slab_row`):
  the same bits, in f64 and in f32, on any number of threads, however the grid is cut. That is
  tested on 1, 4 and 20 threads, split and diamonds, tiles of 1 to 1000 rows and 1 to 5 steps,
  with CPMLs (κ and α), conductors, walls, periodic x, 2D and rows of one value.
- **What else a step does:** sources and currents are added to each value right after its update,
  in the order a step adds them; a dispersive medium's currents are stepped after E, so with one
  it is a step at a time; probes and monitors (transforms, fluxes, modes, a design's) read each
  step's fields, which the diamonds copy out of each row right after its update and record after,
  step by step, in order. The adjoint runs through all of this. A simulation stepped by tiles is
  tested against the whole grid's bits with all of these, and so is the adjoint's gradient.
- **Where tiles can't go:** a side that wraps along y or z (periodic, Bloch) needs the far side's
  rows; a smoothed tensor's E = ε̃⁻¹D reads the rows around after D's update, and a plane wave's
  auxiliary grid steps between H̃'s update and E's. Those are stepped whole every step, as is the
  plain loops' reference.
- **When:** chosen by itself (`Blocking::auto`), from the grid alone and the threads, the same bits
  either way. `Simulation::run` and `run_until` (and so `run_until_converged`, `run_until_decayed`
  and the adjoint) take diamonds once E, H̃ and E's coefficients pass 40 MiB: the most steps, up
  to 5, whose tiles of 2s + 2 rows number at least min(threads, 16) and keep at most 4.5 MiB in
  flight (2s + 2 planes); `Simulation::step` takes a step by tiles past 64 MiB. Below, a grid the
  caches hold is stepped whole: there the tiles' fewer, unequal tasks cost more than they save.
  A job, which reads its probes every step, steps a step at a time.

**Measured** by `fdtd::kernel_rates` on the Core Ultra 7 265K: the kernel alone (no sources or
monitors) on cubes of 50 nm cells of vacuum inside CPMLs of 8, random fields, the best of three
runs; million cell-updates/s. The roof is the triad over the bytes a step reads and writes,
CPMLs included. The tiles are those `Blocking::auto` chooses: for a step at a time 16 rows (on
20 threads, 6 and 8 for 96³ and 128³), for the diamonds as shown.

| Grid | Precision | Threads | Whole | A step at a time by tiles | Diamonds | The roof |
|---|---|---:|---:|---:|---:|---:|
| 64³ | f64 | 1 | 115 | — | — | 102 |
| 64³ | f64 | 20 | 447 | — | — | 170 |
| 64³ | f32 | 1 | 183 | — | — | 204 |
| 64³ | f32 | 20 | 605 | — | — | 340 |
| 96³ | f64 | 1 | 86 | 117 | 155 (12 rows, 5 steps) | 118 |
| 96³ | f64 | 20 | 323 | 420 | 615 (6 rows, 2 steps) | 197 |
| 96³ | f32 | 1 | 191 | 174 | 203 (12 rows, 5 steps) | 236 |
| 96³ | f32 | 20 | 904 | 773 | 998 (6 rows, 2 steps) | 394 |
| 128³ | f64 | 1 | 82 | 118 | 158 (12 rows, 5 steps) | 128 |
| 128³ | f64 | 20 | 247 | 383 | 792 (8 rows, 3 steps) | 214 |
| 128³ | f32 | 1 | 163 | 188 | 225 (12 rows, 5 steps) | 256 |
| 128³ | f32 | 20 | 628 | 788 | 1344 (8 rows, 3 steps) | 427 |
| 192³ | f64 | 1 | 90 | 120 | 171 (12 rows, 5 steps) | 140 |
| 192³ | f64 | 20 | 245 | 391 | 1070 (12 rows, 5 steps) | 234 |
| 192³ | f32 | 1 | 175 | 194 | 250 (12 rows, 5 steps) | 280 |
| 192³ | f32 | 20 | 497 | 751 | 1836 (12 rows, 5 steps) | 467 |
| 256³ | f64 | 1 | 97 | 126 | 177 (12 rows, 5 steps) | 147 |
| 256³ | f64 | 20 | 252 | 390 | 1015 (12 rows, 5 steps) | 245 |
| 256³ | f32 | 1 | 170 | 199 | 252 (12 rows, 5 steps) | 293 |
| 256³ | f32 | 20 | 493 | 764 | 1986 (12 rows, 5 steps) | 490 |
| 320³ | f64 | 1 | 104 | 135 | 186 (12 rows, 5 steps) | 151 |
| 320³ | f64 | 20 | 261 | 402 | 1003 (12 rows, 5 steps) | 252 |
| 320³ | f32 | 1 | 182 | 199 | 269 (12 rows, 5 steps) | 302 |
| 320³ | f32 | 20 | 516 | 795 | 1854 (12 rows, 5 steps) | 505 |

So, beyond the caches:
- **Whole**, 20 threads reach the roof, to within a few per cent, and no further: 245 to 261
  million in f64, 493 to 516 in f32. One thread stays below its roof.
- **A step at a time by tiles** is 1.5 to 1.6 times the whole grid on 20 threads from 192³ on,
  1.1 to 1.4 on one: H̃'s update leaves E in cache for E's.
- **Diamonds** are 3.6 to 4.4 times the whole grid on 20 threads from 192³ on (1003 to 1070
  million in f64, 1836 to 1986 in f32), about 4 times the memory's roof; 1.4 to 1.9 times on
  one. Malas et al. measured 3 to 4 times over spatial blocking on their 18-core Haswell; here
  the diamonds are 2.3 to 2.7 times the step at a time by tiles. Temporal blocking pays on this
  machine, and is the default wherever a run can take it.
- At 96³ in f32 (42 MB) the step at a time by tiles was slower than the whole grid, so a step
  at a time now starts at 64 MiB; at 64³ the grid is in cache and stepped whole. Hence the
  thresholds above. The thresholds are this machine's; on one with less cache
  the tiles would pay from smaller grids.

The `photonoxide bench` problems time the kernel alone, its f64 fields checked against the
plain loops' bits (docs/benchmarks.md): `fdtd3d/box-*`, 48³ cells of 50 nm (in cache, so stepped
whole), and `fdtd3d/guide-*`, 160³ cells of 20 nm with a 500 × 220 nm silicon guide and CPMLs
of 8, 60 steps by diamonds, beside `fdtd3d/guide-whole-*`, the same guide whole every step. On
the guide, 20 threads made 878 million cell-updates/s in f64 by diamonds against 234 whole (3.8
times) and 1536 in f32 against 482 (3.2 times); one thread 177 against 83 and 208 against 151.

A simulation with a point source, a probe and a flux box (six faces transformed at three
frequencies) on a 256³ grid of 50 nm cells inside CPMLs of 8 (`fdtd::kernel_rates`): 241
million cell-updates/s whole, 335 a step at a time by tiles and 719 run by diamonds on 20
threads, 91, 114 and 162 on one; the transforms take their share. On the GPU, see
[below](#the-gpu).

Roden and Gedney's plate, 2.9 × 10⁶ cells for 2000 steps plus two smaller lattices, took about
45 s on 20 threads of a Core Ultra 7 265K with the plain loops.

## The GPU

With the `gpu` feature, a `Simulation` steps on a GPU through wgpu's compute shaders: the same
problem and the same API, chosen by a setting, the CPU the default.

```rust
let gpu = Gpu::new(Precision::Single)?; // or Precision::Double, on Vulkan
s.set_device(Device::Gpu(gpu))?;
s.run(2000); // the fields there and back, the probes and transforms with them
```

- **What it steps:** E and H̃, CPMLs (κ and α), walls and periodic sides, conductors and
  conductivity, point sources and currents (dipoles, mode sources, Gaussian beams, the adjoint's),
  probes, and every transform monitor (transforms, fluxes, modes, a design's): so the adjoint
  runs there too. Not yet: Bloch phases, dispersive media, smoothed tensors that couple E's
  components and plane waves on total-field/scattered-field boxes. `set_device` refuses a problem
  with them, saying which.
- **Precision:** f32 on any wgpu backend (Vulkan, DirectX 12, Metal); f64 on Vulkan where the
  adapter has `SHADER_F64`, for checking against the CPU (decision 6 of the [performance
  plan](../plans/performance.md)). `Gpu::new` takes a discrete GPU before an integrated one and
  never a software adapter.
- **Pure Rust, no `unsafe`:** wgpu is a Rust crate that loads the system's Vulkan, DirectX 12 or
  Metal driver at run time; photonoxide calls only its safe API, so the library keeps
  `#![forbid(unsafe_code)]`. The feature is off by default: wgpu and its shader compiler
  lengthen a clean build, and the default build needs no GPU. CI builds it, lints it and runs
  its tests, which skip there (its runners have no GPU, decision 7).

**The kernels** are WGSL written by hand (`src/fdtd/gpu/yee.wgsl`), compiled for f32 or f64.

- **Slice by slice** (P. Micikevicius 2009): a workgroup of 32 × 8 invocations owns a tile of
  32 × 8 values in each plane of a run of 4 (f32) or 8 (f64) planes along z, and marches through
  them. Each plane of the other field goes into workgroup memory with the row and column of
  neighbours the curl needs, and each invocation keeps its own column's value of the plane before
  or after in registers, so each value is read from memory about once.
- **Fused:** one pass a step takes H̃ over the tile and the row and column before it, then E over
  the tile from those: E, H̃ and E's coefficients are read once and E and H̃ written once,
  60 bytes a cell in f32 without conductivity (the two passes of the CPU's whole-grid step read
  and write 84 to 96). A neighbouring workgroup still reads the old values around its tile while
  this one writes, so the new ones go to a second copy of E, H̃ and ψ and the copies swap each
  step; the values of H̃ on the row and column before a tile, and on the plane before a run of
  planes, are computed again by each workgroup that needs them, by the same function as the one
  that owns them. Where the second copy doesn't fit, a step is two passes in place, H̃'s then
  E's.
- **CPMLs** in the same pass, only where a value is in a slab, in the order the CPU's slabs take
  them; **ca** isn't read where the medium has no conductivity (ca is then 1, or 0 with cb).
- **Sources and currents** are added one invocation per value, its sources and currents in the
  order the CPU adds them, their waveforms taken on the CPU in f64 at each step's times. With
  the fused step a magnetic current goes into H̃ just before its update rather than after: the
  same sum in another order.
- **Probes** are copied out after each step; **transforms** are summed per value, one
  invocation a value taking each frequency in turn, step after step, with e^(iωt)Δt computed on
  the CPU in f64. A run's sums start at zero on the GPU and are added to the simulation's in f64
  when it ends; the fields, probes and sums come back after each `run`, so run long stretches.

**Determinism** (principle 9, decided 2026-10-08): no atomics, fixed workgroup sizes, every value
written by one invocation and every sum taken by one in step order, so a GPU run repeats bit for
bit on the same device and driver: fields, probes and transforms, f32 and f64, fused and in two
passes (`a_run_on_the_gpu_repeats_bit_for_bit`). WGSL lets a compiler fuse and reorder
arithmetic, so the GPU isn't the CPU's bits; it agrees with the CPU to tolerances measured by how
rounding grows (`src/fdtd/gpu/tests.rs`):

- **f64:** a 3D problem with CPMLs (κ and α), a wall, conductivity, a conductor, sources on E and
  H̃, a current, a dipole, probes, a transform box and a flux box (37 × 21 × 19 cells, sizes that
  leave workgroups part-filled), and its 2D twin periodic along x, 800 steps fused or in two
  passes, in one run or in runs of 130: fields, probes, transforms and fluxes within 1e-13 of the
  CPU's (measured up to 1.2e-14). In a closed box of 40 × 36 × 32 cells over N steps, within
  10⁻¹⁵ √N (measured 2 × 10⁻¹⁶ √N, 2.9e-14 after 25 600).
- **f32 against the CPU's f32 kernel:** in the closed box, a random walk of rounding, within
  2 × 10⁻⁷ √N (measured 1.1e-6 after 100 steps to 1.6e-5 after 25 600).
- **f32 against f64:** both f32 kernels drift from f64 alike, linearly, from the coefficients
  rounded to f32 (5.6 × 10⁻⁸ N relative to the largest E in the closed box; within 10⁻⁷ N); the
  GPU's drift is the CPU's f32 drift to 0.2 %. With CPMLs and sources after 800 steps, within
  2e-5 of the CPU's f64 run (measured up to 4.4e-6).
- **A published result:** Roden and Gedney's plate in soil (the `cpml_roden_gedney` example) in
  f32 on the GPU: −48.620 and −70.474 dB against the CPU's −48.620 and −70.475 (paper: −48 and
  −67), in about 3 s.

These, every FDTD case of the validation report run again on the GPU, and its speed are
[the GPU's report](../validation-gpu.md), written on the owner's machine before each release.

**Speed** on the owner's RTX 4060 (Vulkan, driver 616.56), `fdtd::gpu::rates`: the kernel alone
on the cubes of the CPU's table above (50 nm cells of vacuum inside CPMLs of 8, random fields),
the best of three runs of a few tenths of a second, in million cell-updates/s; against the blocked
CPU kernel on 20 threads of the Core Ultra 7 265K, the diamonds' column of that table (measured
idle; the CPU kernel is unchanged since). The GPU was measured while another job kept 12 of the
CPU's 20 threads busy: its rate barely depends on the CPU's load (3723 then, 3727 at 5 % load at
256³ in f32), the CPU's does, so the CPU's numbers are the idle ones. The roof is the 4060's
272 GB/s over the bytes a fused step moves, 60 a cell in f32 and 120 in f64 (no conductivity, so
ca isn't read).

| Grid | Precision | GPU, two passes | GPU, fused | CPU, diamonds | GPU / CPU | The roof |
|---|---|---:|---:|---:|---:|---:|
| 128³ | f32 | 2433 | 3350 | 1344 | 2.49 | 4533 |
| 192³ | f32 | 2642 | 3567 | 1836 | 1.94 | 4533 |
| 256³ | f32 | 2876 | 3723 | 1986 | 1.87 | 4533 |
| 320³ | f32 | 2862 | 3663 | 1854 | 1.98 | 4533 |
| 128³ | f64 | 1242 | 1321 | 792 | 1.67 | 2267 |
| 192³ | f64 | 1388 | 1555 | 1070 | 1.45 | 2267 |
| 256³ | f64 | 1421 | 1642 | 1015 | 1.62 | 2267 |
| 320³ | f64 | 1429 | 1708 | 1003 | 1.70 | 2267 |

So:
- **In f32 the GPU is about twice the blocked CPU kernel** from 192³ on (1.9 to 2.0 times),
  2.5 times at 128³, at 74 to 82 % of its memory's roof: 3.7 G/s at 256³ against the
  performance plan's estimate of about 3.8. The two passes, at up to 89 % of their own roof (84
  bytes a cell), are 1.4 to 1.8 times the CPU. The diamonds run at about 4 times the CPU's own
  memory roof, so the 4060's 2.7 times the CPU's bandwidth comes out as about twice the speed.
- **f64** runs at 1.5 to 1.7 times the CPU, at 58 to 75 % of its roof: the 4060's f64 arithmetic
  is a sixty-fourth of its f32's, and the fused step's recomputed row and column of H̃ weigh more
  there. It varies more between sessions, by up to a fifth (the GPU report's run: 1147 and 1459
  fused at 128³ and 256³). It is for checking.
- A simulation with a source, a probe and a flux box (six faces at three frequencies, 256³) runs
  at 1873 million cell-updates/s in f32 and 1020 in f64 over runs of 200 steps, the copies to
  and from the GPU included; a `Simulation` steps the CPU in f64, 719 by diamonds (above, idle).

**Memory** on an 8 GB RTX 4060 (about 7 GB free beside the desktop), cubes inside CPMLs of 8,
found by allocating (`fdtd::gpu::rates::gpu_memory`): in f32, fused up to 416³ (72 million cells,
72 bytes a cell) and in two passes up to 480³ (111 million, 48 bytes a cell); in f64, fused up to
320³ (33 million) and in two passes up to 384³ (57 million). `Gpu::new` sets wgpu's memory
budget so that an allocation past the device's memory fails rather than spilling into the
system's; a simulation then takes two passes, or `set_device` says it doesn't fit.

## Jobs

An `"fdtd"` job (`photonoxide::job`, documented with the other kinds there) runs all this on a
layer stack and its shapes, in 2D or 3D, and the studio shows it live: the field on a plane as
it propagates, each monitor's spectrum as its transforms accumulate, and the resonances at the
end.

- **The structure:** in 2D, the layer's plane, each point's permittivity its slab mode's
  effective index squared, exactly as an `"fdfd"` job's (the effective index method: an
  estimate, not a device's 3D performance); in 3D, the stack itself. Either is averaged over each
  value's cell as `Simulation::new` averages it, and taken at the job's wavelength, the carrier
  of the one Gaussian pulse every source shares: the run is non-dispersive, so away from the
  carrier its spectra are not an `"fdfd"` job's, which takes the slab's index at each
  wavelength.
- **Sources** are this module's: guide modes (FDFD's port modes on a few planes cut out of the
  grid around the source, at the leapfrog's frequency for the carrier, of the job's
  polarization), dipoles, plane waves on total-field/scattered-field boxes and Gaussian beams.
- **Monitors:** a guide's mode each way through a plane, the flux through a plane and out of a
  box, transforms of the field on the view's plane, and harmonic inversion of a point's field
  after the pulse. The spectra are referred to the incident power at each wavelength: for a
  mode source, the power its guide carries forward in its own mode at that wavelength, measured
  on a plane three cells after it (exact, whatever the pulse launches off its carrier); for a
  plane wave, ½√ε|W(ω)|² over its box's face; for a beam, the paraxial beam's power. A mode
  source also records what comes back into its mode, the reflection.
- **The record:** a frame of the field on the view's plane at intervals that start at an
  eightieth of the pulse's length and the light's crossing of the window, and double after
  every hundred frames and whenever a frame takes more than a tenth of the steps' time since the
  last (at most 500 frames, each averaged to at most 240 pixels a side and quantized to bytes);
  the progress with each frame and at least every second (the field left at the monitors
  against the stopping rule, the cell-updates a second, the frames' share of the time); every
  monitor's spectrum every two seconds or so (less often when it takes long) and at the end.
- **The end:** when |field|² at the monitors' middles has stayed below a fraction of its peak
  for ten carrier periods after the pulse, at a time limit, or at the job's timeout; the last
  check's fraction is the run's recorded error, what the transforms leave out.

On the same grid with the permittivity of the carrier, a job's spectra are FDFD's
S-parameters at the leapfrog's frequencies. At the carrier they differ by the CPMLs' difference
from FDFD's PMLs; away from it the mode source, which launches the carrier's mode at every
frequency (see Sources), also puts in a little that isn't the guide's own mode there, and some
of that reaches the device's outputs, so the difference grows with the distance from the
carrier:
- a guide cut by a 0.3 µm gap, 2D TE on 40 nm cells with absorbers of 20 (the test
  `a_gaps_transmission_and_reflection_are_fdfds`): the transmitted power within 1.3e-4 of FDFD's
  at 1.55 µm and 2.4e-3 at 1.6 µm (3 % from the carrier), the reflected within 1.5e-5 and
  4.4e-4;
- the built-in `jobs/mmi-fdtd.toml`, the 1 × 2 MMI of `jobs/mmi-fdfd.toml` on its 20 nm grid
  with absorbers of 20 (the test `the_mmi_job_reproduces_the_fdfd_jobs_s_parameters`, run on
  demand): each output's power within 4.3e-4 of FDFD's at 1.55 µm and 1.1e-3 at 1.6 µm, the
  reflection within 2.5e-4, over 1.5 to 1.6 µm.

A ring of 1.5 µm beside its bus, 2D TE on 40 nm cells (`a_rings_resonances_are_where_its_bus_dips`):
harmonic inversion of the field in the ring finds a resonance within one of the spectrum's
2.5 nm steps of the bus's deepest dip, Q about 690. Harmonic inversion takes the field every
quarter period of the band's highest frequency, at most 4000 samples: a whole run's steps,
tens of thousands, make its powers of the eigenvalues run away.

These are 2D numbers by the effective index method, and non-dispersive: they check the job
against FDFD on the same discrete problem, not the device's 3D performance.

Before the rows were shared, a 2D grid, one plane thick, stepped on one thread: the MMI's
217 500 cells at 31 million cell-updates a second on 20 threads of a Core Ultra 7 265K, and
140 million after.

## Validation

| Case | What | Measured |
|---|---|---|
| `fdtd/dispersion` | a plane wave's frequency against Taflove and Brodwin's relation | 3.6e-15 |
| `fdtd/energy` | the leapfrog's invariant in a closed box, 300 steps | 2.4e-15 |
| `fdtd/fdfd-lossy` | a lossy box's steady state against FDFD at $\tilde\omega$ | 2.3e-14 |
| `fdtd/fdfd-cpml` | with CPMLs, against FDFD with its PML, at 128 steps a period | 3.5e-4 |
| `fdtd/cpml-thickness` | a 2D pulse's reflection from a CPML of 16 cells | 6.1e-6 |
| `fdtd/spectrum` | a pulse's DFT per unit source spectrum against FDFD at $\tilde\omega$ | 6.5e-13 |
| `fdtd/tfsf-leakage` | a plane wave's box with nothing in it, normal and oblique, 2D and 3D | 1.3e-15 |
| `fdtd/tfsf-slab` | a slab's reflection in a box against Airy's formula, cells of 5 nm | 3.3e-4 |
| `fdtd/mode-source-fdfd` | a mode source in a lossy box against FDFD's mode source | 2.5e-14 |
| `fdtd/mode-source-backward` | the power a mode source sends back in an open guide | 1.5e-7 |
| `fdtd/mode-source-power` | its forward flux against the mode's power | 1.000048 |
| `fdtd/beam-waist` | a 2D Gaussian beam's fitted waist, µm | 1.49999 (1.5) |
| `fdtd/beam-divergence` | its divergence against the grid's plane waves | 2.7e-6 |
| `fdtd/beam-tilt` | a tilted beam's direction against the grid's plane waves | 1.5e-4 |
| `fdtd/ade-fdfd` | a box of a Drude and a Lorentz term against FDFD with the leapfrog's permittivity | 3.3e-14 |
| `fdtd/drude-fresnel` | a Drude half-space's reflection against Fresnel's, 0.4 to 1.6 c/µm, cells of 5 nm | 3.0e-4 |
| `fdtd/lorentz-group-delay` | a Lorentz slab's group index from a pulse's delay, cells of 2.5 nm | 3.6e-4 |
| `fdtd/bloch-fdfd` | Bloch sides, plain and dispersive, against FDFD's Bloch field | 2.9e-14 |
| `fdtd/bloch-multilayer` | a stack at oblique incidence against the transfer matrices, cells of 5 nm | 1.3e-3 |
| `fdtd/bloch-bands` | a 1D photonic crystal's bands, 80 cells a period, relative | 3.1e-4 |
| `fdtd/fit-lossless` | silicon's and silica's fits, relative | 6.0e-5 |
| `fdtd/fit-lossy` | InGaP's fit above its gap, relative | 8.1e-3 |
| `fdtd/fitted-slab` | a fitted silicon slab against Airy's formula, cells of 5 nm | 2.0e-3 |
| `fdtd/smoothing-slab` | a slab's faces between grid points, smoothed, against Airy's formula, cells of 1/160 µm | 6.9e-4 |
| `fdtd/smoothing-anisotropic-slab` | an anisotropic slab's $r_{xx}$ and $r_{yx}$ against the exact ones, cells of 1/160 µm | 3.0e-4 |
| `fdtd/smoothing-oblique` | layers oblique to the grid, `Coupling::Points`, against their transfer matrices, 128 cells a µm, relative | 2.1e-4 |
| `fdtd/smoothing-oblique-nodes` | the same with `Coupling::Nodes`: first order | 2.0e-3 |
| `fdtd/smoothing-energy` | the leapfrog's invariant with a smoothed tensor, `Coupling::Nodes`, 300 steps | 1.5e-15 |
| `fdtd/smoothing-oskooi` | Oskooi et al.'s anisotropic lattice, 32 cells a period: the error relative to the harmonic mean's and no smoothing's | 0.063 |
| `fdtd/smoothing-triplets-oblique` | the oblique layers with `Coupling::Triplets`, 128 cells a µm, relative | 1.06e-3 |
| `fdtd/smoothing-triplets-order` | its order from 64 to 128 cells a µm: first, as Werner et al. 2013 find | 1.15 |
| `fdtd/smoothing-bauer-order` | Bauer et al.'s tensors not made symmetric (a check): their order | 1.87 |
| `fdtd/smoothing-triplets-contrast` | the leapfrog's invariant with `Coupling::Triplets` at ε = 100, 10⁵ steps, ‖E‖ bounded | 5.2e-15 |
| `fdtd/smoothing-triplets-lattices` | the same in a period of the holes and of the anisotropic lattice, 16 cells, 10⁵ steps | 5.0e-15 |
| `fdtd/smoothing-wc07-growth` | the 2007 scheme's growth at ε = 100 on Werner et al.'s discs, 32 cells, c/a | 2.85 (paper ≈ 3) |
| `fdtd/smoothing-crystal` | Bauer et al.'s anisotropic ellipsoid crystal with `Coupling::Triplets`, 48 cells: nine bands against their Table 1, relative | 1.8e-3 |
| `fdtd/smoothing-crystal-order` | the same with their tensors not made symmetric: the order from 32 to 48 cells | 1.98 |
| `fdtd/monitor-transforms` | a transform monitor against the sum by hand | 0 |
| `fdtd/monitor-flux` | the flux through a plane and out of a box against FDFD's, relative | 1.2e-11 |
| `fdtd/monitor-flux-box` | the flux out of a closed lossless box with no source, relative to a face's | 1.5e-15 |
| `fdtd/monitor-modes` | a guide's mode amplitudes in a lossy box against FDFD's, relative | 1.5e-11 |
| `fdtd/monitor-guides` | a 2D straight guide's and sharp bend's transmission and reflection against FDFD with PMLs | 5.8e-5 |
| `fdtd/harmonic-inversion` | two decaying terms closer than the Fourier resolution, recovered | 2.1e-12 |
| `fdtd/cavity-resonances` | a lossy cavity's resonances against the leapfrog's own eigenvalues | 3.4e-15 |
| `fdtd/slab-resonance` | a slab's resonance and Q against the continuum's: the order of convergence | 2.00 |
| `fdtd/mie-table` | Mie's series against his own Table I, 66 entries of $\mathfrak{a}_1$: the median difference (61 within 0.016) | 3.0e-3 |
| `fdtd/mie-balance` | a lossless sphere's $Q_\text{ext}$ against its $Q_\text{sca}$, α from 0.1 to 1000, relative | 1.4e-13 |
| `fdtd/mie-terms` | the series cut at $\alpha + 4\alpha^{1/3} + 10$ terms against the converged sum | 7.0e-14 |
| `fdtd/mie-sphere` | a smoothed sphere of ε = 4 against Mie, 8 and 16 cells a radius: the order of convergence | 1.97 |
| `fdtd/mie-sphere-staircase` | the same sphere sampled, 16 cells a radius: the mean relative error of $C_\text{sca}$ | 6.0e-3 |
| `fdtd/mie-drude` | a damped Drude metal sphere, 16 cells a radius: the mean relative error of $C_\text{sca}$ and $C_\text{abs}$ | 1.6e-2 |
| `fdtd/meep-pml-rates` | Oskooi et al.'s Fig. 8: the rate a PML's field convergence falls at, σ as $(x/L)^d$, at L = 4, against $2d + 4$ | 6.14, 8.22, 10.32 |
| `fdtd/ring-wronskian` | the ring's Bessel functions: their Wronskian against $2/\pi z$, relative | 1.0e-10 |
| `fdtd/ring-resonances` | Fig. 11's ring (ε = 11.56, radii 1 and 2 µm), 20 cells a µm: three resonances against the exact ones, relative frequency | 7.7e-4 (Q 7.3e-3) |
| `fdtd/ring-order` | the same from 10 to 20 cells: the order of the frequencies' and Q's errors | 1.95 to 2.05 |
| `fdtd/fdfd-band-2d` | a 2D straight guide and bend, one pulse against FDFD at five frequencies, 50 nm cells: $\lvert\Delta S\rvert$ | 3.2e-5 |
| `fdtd/fdfd-band-2d-fine` | the same on 25 nm cells | 1.7e-7 |
| `fdtd/fdfd-band-3d-strip` | a 3D strip, one pulse against `Solver3d` at three frequencies, CPMLs of 8 | 2.2e-4 |
| `fdtd/fdfd-band-3d-bend` | a 3D bend, the same | 1.9e-4 |
| `fdtd/fdfd-smoothed` | the 2D bend smoothed for FDTD against FDFD's average: the order their difference falls at, 50 to 25 nm | 0.85 |
| example `pml_oskooi` | Oskooi et al.'s Fig. 8: the rates at L = 8 wavelengths | 6.03, 8.05, 10.08 (paper: 6, 8, 10) |
| example `bump_oskooi` | Oskooi et al.'s Fig. 7: a bump's scattered power, 10 to 30 pixels a, least-squares orders | 2.37 smoothed, 1.07 not (paper: about 2 and 1) |
| example `cpml_roden_gedney` | Roden and Gedney's plate in soil, both PMLs | −48.6 and −70.5 dB (paper: −48, −67) |
| example `tfsf_square_cylinder` | Umashankar and Taflove's square cylinder's surface current | 1.732 and 0.764 (figure: 1.750, 0.785); within 0.4 % of their Eq. 8a |
| example `lorentz_okoniewski` | Okoniewski, Mrozowski and Stuchly's two-term Lorentz half-space, $\lvert r\rvert$ and phase errors, 37.5 µm cells | at most 0.24 and 0.34 of their Fig. 1's curve (C = 1), 0.33 and 0.52 (C = 0.5) |
| example `crossing_liu_poon` | Liu and Poon's crossing (gdsfactory's PDK) in 3D, through TE₀ at 1550 nm, 15 and 20 cells a wavelength (`--full`) | 0.95666, 0.95863 (Lumerical and Tidy3D settled: 0.9558 to 0.959) |
| example `coupler_liu_poon` | their directional coupler, cross TE₀ at 1550 nm, 15 and 20 cells | 0.42771, 0.41352 (0.411 to 0.492) |
| example `mmi_liu_poon` | their 2 × 2 MMI, cross TE₀ at 1550 nm, 15 and 20 cells | 0.48148, 0.48743 (0.479 to 0.489) |
| examples `mode_converter_liu_poon`, `splitter_rotator_liu_poon` | their mode converter and splitter-rotator at 5 cells (CI); the paper's grid in #256 | TE₁ 0.457, TE₀ 0.642: within the codes' own stray at 6 cells |
| example `ring_liu_poon` | their ring's free spectral range from the strip's group index and its length (CI); the 3D run in #256 | 7.54 nm (the text: around 7.4 and 7.6) |
| example `subpixel_holes` | Farjadpour et al.'s elliptical holes, TE, 12 to 64 pixels a period: slopes of the error | lowest mode −2.26 at 2Δx (paper −2.43), −2.12 at s = 1, below the mean at every resolution; next mode's mean −1.40 (paper −1.33); lowest mode's mean −1.84 |

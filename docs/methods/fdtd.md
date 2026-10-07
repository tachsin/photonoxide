| `fdtd/spectrum` | a pulse's DFT per unit source spectrum against FDFD at $\tilde\omega$ | 6.5e-13 |
| `fdtd/tfsf-leakage` | a plane wave's box with nothing in it, normal and oblique, 2D and 3D | 1.3e-15 |
| `fdtd/tfsf-slab` | a slab's reflection in a box against Airy's formula, cells of 5 nm | 3.3e-4 |
| `fdtd/mode-source-fdfd` | a mode source in a lossy box against FDFD's mode source | 2.5e-14 |
| `fdtd/mode-source-backward` | the power a mode source sends back in an open guide | 1.5e-7 |
| `fdtd/mode-source-power` | its forward flux against the mode's power | 1.000048 |
| `fdtd/beam-waist` | a 2D Gaussian beam's fitted waist, µm | 1.49999 (1.5) |
| `fdtd/beam-divergence` | its divergence against the grid's plane waves | 2.7e-6 |
| `fdtd/beam-tilt` | a tilted beam's direction against the grid's plane waves | 1.5e-4 |
| example `tfsf_square_cylinder` | Umashankar and Taflove's square cylinder's surface current | 1.732 and 0.764 (figure: 1.750, 0.785); within 0.4 % of their Eq. 8a |
---
title: "FDTD"
module: fdtd
summary: "Maxwell's equations stepped in time on Yee's grid: E and H leapfrogging on FDFD's own grid, the convolutional PML, walls and periodic sides, conductors, lossy media, the same bits on any number of threads; dipoles and currents normalized exactly by their spectra, plane waves on total-field/scattered-field boxes at any grid angle, one-way waveguide modes and Gaussian beams."
order: 23
papers:
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "A. Taflove, M. E. Brodwin, IEEE Trans. Microw. Theory Tech. 23, 623 (1975) (stability and numerical dispersion)"
    doi: 10.1109/TMTT.1975.1128640
  - cite: "K. Umashankar, A. Taflove, IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982) (total-field/scattered-field)"
    doi: 10.1109/TEMC.1982.304054
  - cite: "J.-P. Berenger, J. Comput. Phys. 114, 185 (1994) (the PML)"
    doi: 10.1006/jcph.1994.1159
  - cite: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000) (the convolutional PML)"
    doi: 10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A
  - cite: "A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010) (sources restricted to the grid)"
    doi: 10.1016/j.cpc.2009.11.008
  - cite: "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012) (total-field/scattered-field in the frequency domain)"
    doi: 10.2528/PIERB11092006
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
examples:
  - cpml_roden_gedney
  - tfsf_square_cylinder
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
held at zero, as in FDFD. **Periodic** sides wrap. A Bloch phase (k ≠ 0) needs complex fields,
and dispersive media need auxiliary equations; both are the next step (#164). A 2D problem is
a grid one cell thick along z, periodic there (`Boundaries::cpml_2d`).

**Conductors and media.** `Simulation::plate` and `conductor` hold E at zero on a conducting
plate or edge. The permittivity is real, averaged over each component's cell as FDFD averages it,
and a conductivity σ (in 1/µm: S/m × 376.73 Ω × 10⁻⁶) makes a medium lossy.

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

## Probes

A probe records one value of E or H̃ every step. DFT monitors, fluxes and harmonic inversion are
#163.

## Cost

Every update is a fixed stencil with no reduction, its z-planes shared among rayon's threads, so
the fields are the same bits on any number of threads. Each axis's neighbour offsets and 1/(κΔ)
factors are tabulated once, and the PML's convolutions run only in their slabs. The kernel is
plain Rust loops. SIMD, f32 and cache blocking are #165, and the GPU is #166. Roden and Gedney's
plate, 2.9 × 10⁶ cells for 2000 steps plus two smaller lattices, takes about 45 s on 20 threads
of a Core Ultra 7 265K.

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
| example `cpml_roden_gedney` | Roden and Gedney's plate in soil, both PMLs | −48.6 and −70.5 dB (paper: −48, −67) |
| example `tfsf_square_cylinder` | Umashankar and Taflove's square cylinder's surface current | 1.732 and 0.764 (figure: 1.750, 0.785); within 0.4 % of their Eq. 8a |

---
title: "FDTD"
module: fdtd
summary: "Maxwell's equations stepped in time on Yee's grid: E and H leapfrogging on FDFD's own grid, the convolutional PML, walls and periodic sides, conductors, lossy media, point sources and probes, the same bits on any number of threads, and a continuous source that settles to exactly FDFD's field."
order: 23
papers:
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "A. Taflove, M. E. Brodwin, IEEE Trans. Microw. Theory Tech. 23, 623 (1975) (stability and numerical dispersion)"
    doi: 10.1109/TMTT.1975.1128640
  - cite: "J.-P. Berenger, J. Comput. Phys. 114, 185 (1994) (the PML)"
    doi: 10.1006/jcph.1994.1159
  - cite: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000) (the convolutional PML)"
    doi: 10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A
validation:
  - fdtd/dispersion
  - fdtd/energy
  - fdtd/fdfd-lossy
  - fdtd/fdfd-cpml
  - fdtd/cpml-thickness
examples:
  - cpml_roden_gedney
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

## Sources and probes

A source is a current density on one value of E (an electric current J) or of H̃ (a magnetic
current M), with a waveform:

- a Gaussian pulse on a carrier;
- a differentiated Gaussian, which has no DC, as Roden and Gedney's source;
- a continuous wave, switched on smoothly.

A probe records one value of E or H̃ every step. Total-field/scattered-field and mode sources
are #162; DFT monitors, fluxes and harmonic inversion are #163.

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
| example `cpml_roden_gedney` | Roden and Gedney's plate in soil, both PMLs | −48.6 and −70.5 dB (paper: −48, −67) |

---
title: "FDTD adjoint gradients"
module: fdtd
summary: "How an objective on an FDTD run's monitors (a mode's power, a flux, a field, summed over frequencies) changes with the density of every cell of a 3D design region, from two runs, the forward and the adjoint: exact for the scheme, the mode's complex profile whole, through the design's average onto Yee's grid; no history kept."
order: 23.5
papers:
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004)"
    doi: 10.1364/OL.29.002288
  - cite: "C. M. Lalau-Keraly, S. Bhargava, O. D. Miller, E. Yablonovitch, Opt. Express 21, 21693 (2013)"
    doi: 10.1364/OE.21.021693
  - cite: "T. W. Hughes, M. Minkov, I. A. D. Williamson, S. Fan, ACS Photonics 5, 4781 (2018)"
    doi: 10.1021/acsphotonics.8b01522
  - cite: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000) (the CPML whose transform enters the adjoint)"
    doi: 10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A
validation:
  - fdtd/adjoint-modes
  - fdtd/adjoint-flux
  - fdtd/adjoint-fdfd
  - fdtd/adjoint-fdfd-cpml
  - fdtd/adjoint-slab
  - fdtd/adjoint-slab-order
  - fdtd/adjoint-backward-mode
---

Inverse design changes a device's permittivity cell by cell. Each step needs the objective's
gradient with respect to every cell, and a run per cell is out of the question. The adjoint
method gets all of them from two runs: the forward run, and one more, the adjoint, whose sources
are the objective's derivative at the monitors. This is the time-domain twin of
[FDFD's adjoint](fdfd-adjoint.md), and in a closed box it gives FDFD's gradient to round-off.

## The frequency-domain system a run solves

[FDTD](fdtd.md) runs in time, but its transforms obey a frequency-domain system exactly. Once a
run's fields have died away, the transforms at ω (E at $n\Delta t$, H̃ at $(n + \tfrac12)\Delta t$,
each current at the times it enters) satisfy

$$
i\tilde\omega\thinspace\hat H - C_E \hat E = \hat M, \qquad
-i\tilde\omega\thinspace\varepsilon \hat E - C_H \hat H = -\hat J, \qquad
\tilde\omega = \frac{2}{\Delta t}\sin\frac{\omega\Delta t}{2}.
$$

Call this $K(\varepsilon)\thinspace x = b$ with $x = (\hat E, \hat H)$. $C_E$ and $C_H$ are the grid's curls. In a CPML,
each difference along an axis w is divided by the stretch of the recursive convolution
$\psi \leftarrow b\psi + a\thinspace\partial F$, transformed:

$$
\frac{1}{\tilde s_w} = \frac1\kappa + \frac{a}{1 - b\thinspace e^{i\omega\Delta t}} .
$$

A conductivity, a dispersive medium and a smoothed tensor stay on K's diagonal blocks, each
symmetric. Only $-i\tilde\omega\varepsilon_r$ depends on the permittivity of value r.

## Its transpose is another run

Yee's curls are each other's transposes, $C_H = C_E^{\mathsf T}$, which is why the leapfrog
conserves energy. A stretched curl factorizes as $\Lambda\thinspace C\thinspace S$, with S the stretch along each
component's own axis and Λ the other two's. Let D be the product of the three stretches at each
value of E and H: at $E_c$, $\tilde s_c$ half a step along c times the stretches at the nodes
across; at $\tilde H_c$, the other way round. Then

$$
K^{\mathsf T} D = D K ,
$$

as FDFD's V A is symmetric with V the product of its PMLs' stretches ([FDFD in 3D](fdfd-3d.md)). Outside the CPMLs
D = 1. So the adjoint system $K^{\mathsf T}\mu = g$ is solved by an ordinary run whose sources are $D^{-1}g$,
with $\mu = D y$: no transposed scheme is needed.

## The gradient

For a real objective F of the transforms, $dF = 2\operatorname{Re}(g^{\mathsf T} dx)$ with $g = \partial F/\partial x$, the
Wirtinger derivative ($\bar x$ held fixed). Veronis, Dutton and Fan's Eqs. 2–4 then give

$$
\frac{\partial F}{\partial \varepsilon_r} = -2\operatorname{Re}\negthinspace\left(\mu^{\mathsf T}\frac{\partial K}{\partial\varepsilon_r}x\right)
= \sum_\omega 2\operatorname{Re}\negthinspace\left(i\tilde\omega\thinspace\hat E_{\text{adj},r}\thinspace\hat E_r\right),
$$

summed over the objective's frequencies. This is Lalau-Keraly et al.'s Eq. 5,
$\operatorname{Re}(E_\text{adj}\cdot E_\text{old})$, from the transforms of both runs over the design region. The
adjoint run's sources are $J = -g_E$ and $M = g_H$ at the monitors. Hughes et al. (Eqs. 9–13)
give the same structure for FDFD.

**The objective's derivative.**
- A mode's amplitude is linear in Ê: $a = \sum_r \omega_r \hat E_r$, the weights of FDFD's Lorentz form
  ([FDFD in 3D](fdfd-3d.md)). So $g = (\partial F/\partial a)\thinspace\omega$, with the mode's whole complex profile and the
  form's phase. Nothing of the mode's imaginary part is dropped.
- A flux $\tfrac12\operatorname{Re}(E\times\tilde H^*)$ sums $\kappa\operatorname{Re}(E\bar H)$ over pairs of values. It gives
  $g_E = \tfrac12\kappa\bar H$ and $g_H = \tfrac12\kappa\bar E$. The pairs are the monitor's own, so g reaches
  into the CPMLs wherever the plane does, and D takes care of that.
- A transform's value gives itself.

`Term::Mode`, `Term::Flux` and `Term::Transform` carry ∂F/∂(measurement). For F = |a|² it is ā.

**Exact transforms from a real run.** The adjoint's currents must have exactly the transforms g
at each of the objective's frequencies, and g is complex. Each value takes $\sum_m \beta_m w_m(t)$
with real amplitudes. For each frequency there are two Gaussian pulses on it, $e^{-u^2}\sin\omega(t - d)$,
the second a quarter period after the first. A small real system, one per field (J and M enter
at different times), gives the β from the pulses' transforms. Each pulse is odd about its peak,
so its mean is zero, to round-off even on the grid's samples. A current with a mean leaves a
charge behind, and its static field never dies away: complex amplitudes on one pulse,
$\operatorname{Re}(\alpha w)$, have a cosine part with a mean of $e^{-(\omega\tau/2)^2}$. In the strip below
that field held the adjoint's energy near $10^{-12}$ of its peak, where the pulses above let it
fall below $10^{-16}$.

**When to stop.** `run_until_converged` runs until, after every source has finished, no monitor's
transform moves by more than a tolerance (relative to its largest) over a block of time. The
transforms are then the frequency-domain fields to about that tolerance. A field left standing
moves them by at most its size over ω̃, so the rule doesn't wait for it. Waiting is no remedy:
with CPMLs (α = 0), the strip's fields grow again from round-off after about 400 µm/c, ten
times the run.

## The design

`Design` is a box of the grid's cells, each with a density ρ and the permittivity
$\varepsilon(\rho) = \varepsilon_0 + \rho(\varepsilon_1 - \varepsilon_0)$. `Simulation::with_design` gives each value of E the
average over its cell that `Simulation::new` takes of any permittivity (FDFD's: harmonic along
the component, arithmetic across it), of $\varepsilon(\rho)$ inside the design and the background outside.
Values whose cells don't reach the design keep their permittivity, smoothed by
`Simulation::smoothed` or not. The forward run is built from this average, and the gradient goes
through it:

- **Along its component**, a value's cell spans one design cell, so the harmonic mean is that
  cell's ε.
- **Across**, its cell covers parts of up to four design cells, and its permittivity is their
  shares' sum plus the background's average over the rest.
- **The chain rule:** each value's ε is linear in the densities, and
  $\partial F/\partial\rho_v = (\varepsilon_1 - \varepsilon_0)\sum_r s_{rv}\thinspace\partial F/\partial\varepsilon_r$, $s_{rv}$ cell v's share of value r's cell.

The design must be clear of the CPMLs, of conductivity and dispersive media, and of smoothed
tensors that couple E's components, and the problem must be reciprocal (no Bloch phase). Its
values must be away from the sources, which must not depend on ε. The modes are held fixed.

## Memory and time

**No history is kept, and nothing is recomputed: no checkpointing.** A time-domain adjoint
that runs the scheme backwards needs the forward fields at every step, stored or recomputed
from checkpoints. This one works with the transforms, so each run keeps only the transforms of
E over the design's values at the objective's frequencies: 16 bytes per value per frequency.
The gradient is then their product, which takes no time. The adjoint is a run of the forward's
size. `Simulation::gradient` reads both runs' transforms, so both are kept to the end here.

Measured with `cargo test --release --lib what_a_gradient_costs -- --ignored` on 20 threads of a
Core Ultra 7 265K (shared with other work, so the times are good to some 10 %), each run until
its transforms settle to 1e-11 over 1 µm/c. The strip (core ε = 6, 0.3 × 0.3 µm, in vacuum, at
1 c/µm) on cells of 50 nm with CPMLs of 8:

| | forward run | with the design's transforms | adjoint run | gradient (both) |
|---|---|---|---|---|
| 160 × 64 × 64 cells, a design of 40 × 16 × 16 | 17.3 s, 5390 steps, 76.4 MB | 16.0 s, 77.0 MB | 17.8 s, 6090 steps, 77.3 MB | 33.9 s, 154 MB |
| 36 × 20 × 20 cells, a design of 3 × 2 × 2 | 0.83 s, 1505 steps, 2.4 MB | 0.83 s, 2.4 MB | 1.02 s, 1890 steps, 2.4 MB | 1.85 s, 4.8 MB |

So a gradient costs about two forward runs in time and in memory, whatever the number of design
cells: 10 240 here. The design's transforms are 0.6 MB, one frequency's E over its 37 000 values;
the bytes are the fields, their coefficients, the CPMLs' auxiliary fields, the currents and the
transforms. The adjoint takes some 10 % more steps: its pulses come a quarter period apart, and
its fields start where the forward's ended, at the monitors.

## Validation

All in 3D but the slab, which is 1D; each number on its grid.

| Case | What | Measured |
|---|---|---|
| `fdtd/adjoint-modes` | a strip guide (36 × 20 × 20 cells of 50 nm, CPMLs of 6), $F = \lvert a_+(f_0)\rvert^2 + \lvert a_+(f_1)\rvert^2 - \frac12\lvert a_-(f_0)\rvert^2$ at 1 and 1.06 c/µm: the gradient at all 12 design cells against fourth-order differences ($\delta = 10^{-3}$), relative to the largest | 5.5e-10 |
| `fdtd/adjoint-flux` | the same strip, the flux through a plane whose window reaches into the CPMLs plus $\lvert E_y\rvert^2$ at a point | 1.5e-9 |
| `fdtd/adjoint-fdfd` | a guide in a closed lossy box (16 × 14 × 24 cells of 50 nm, 64 steps a period of 1.55 µm): FDTD's gradient at the design's 144 values of E against `Solver3d`'s adjoint on the same values at $\tilde\omega$ | 7.5e-14 |
| `fdtd/adjoint-fdfd-cpml` | the same box open along the guide, CPMLs against FDFD's PMLs, at 64, 128 and 256 steps a period | 1.3e-4, 5.1e-5, 2.3e-5 |
| `fdtd/adjoint-slab` | a slab of ε = 4, 0.3 µm, at 1 c/µm: the gradient summed over its cells against Airy's dT/dε, cells of 1/40, 1/80 and 1/160 µm | 1.2e-2, 2.9e-3, 7.2e-4 |
| `fdtd/adjoint-slab-order` | its order from 1/80 to 1/160 µm | 2.02 |
| `fdtd/adjoint-backward-mode` | Lalau-Keraly et al.'s Eq. 8 in the closed box: $\hat E_\text{adj}/(\bar a\thinspace\hat E_\text{back})$ against $\Delta V/4$ | 5.5e-13 |
| `fdfd3d/adjoint-gradient` | FDFD's adjoint in 3D, a guide with a block beside it (14 × 12 × 24 cells of 50 nm, PMLs of 4): eight values against fourth-order differences | 2.8e-10 |

**Against finite differences.** In the strip the transforms settle to 1e-11 a block, and the
differences' own round-off is about $10^{-10}/\delta$ of F: the adjoint gradient is exact to that.
The flux's sources include magnetic currents and values in the CPMLs, where D ≠ 1.

**The mode's imaginary part.** The strip's mode reaches into the CPMLs across, so its profile is
complex. With the adjoint's sources taken from the mode with the imaginary parts of its profile
and β dropped (and so its normal component, 90° out of phase), the gradient is off by 0.49 of
the largest (a unit test).

**Against FDFD.** In a closed lossy box the transforms are FDFD's fields at $\tilde\omega$ exactly, with
$\varepsilon + i\sigma\cos(\omega\Delta t/2)/\tilde\omega$, and the gradients agree to round-off: the adjoint here is
FDFD's adjoint, run in time. With CPMLs they differ as the CPML differs from FDFD's PML, to
first order in Δt (orders 1.3 and 1.1 from 64 to 256 steps a period, at a fixed $\tilde\omega$). Refining
the grid at a fixed Courant number refines the step with it.

**A closed form.** A uniform slab's gradient summed over its cells is dT/dε of the discrete
slab, so it converges to Airy's at the scheme's own second order.

**A published result.** Lalau-Keraly et al. (Eq. 8) find that the adjoint field of a mode's
transmission is the mode sent backwards from the monitor into the device, its amplitude the
forward overlap's conjugate (Eq. 9). With FDFD's Lorentz form as the projection this is exact
on the grid. Take a source b in front of the monitor, its field u, and v the field of the
backward mode f launched from the monitor's plane (total-field/scattered-field,
$b_\text{back} = (QA - AQ)f$). Reciprocity gives $v^{\mathsf T}b = (Vb_\text{back})^{\mathsf T}u$, the Lorentz form of u and f
across the source's cut: $4ik_0/\Delta V$ times the forward amplitude a(u). So in front of the
plane $\lambda = A^{-\mathsf T}\omega = (\Delta V/4ik_0)\thinspace v$, and in a run's units
$\hat E_\text{adj} = \bar a\thinspace(\Delta V/4)\thinspace\hat E_\text{back}$. Both hold to round-off: FDFD's to 1e-9 in a unit test,
FDTD's to 5.5e-13. With CPMLs across the guide it holds only as well as the CPML is FDFD's
PML, because the projection takes FDFD's stretch and the run the CPML's.

**Determinism.** The forward and adjoint runs and the gradient are the same bits on 1, 4 and
20 threads (a unit test).

## Limits

- The design is a box of the grid's own cells, its density one per cell. A coarser design, or a
  filter and projection (0.7), is a linear map before it, and its gradient that map's transpose.
- The gradient is with respect to the permittivity's real part, in a lossless, non-dispersive
  design region.
- Modes and sources are held fixed: a port's mode doesn't follow the design.
- The CPML is FDFD's PML only to first order in Δt. So with CPMLs the gradient is FDTD's own,
  exact for the run, and it approaches FDFD's as the step falls.

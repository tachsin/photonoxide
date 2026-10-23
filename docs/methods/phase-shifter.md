---
title: "The thermo-optic phase shifter: Pπ, the time constant and crosstalk"
module: "thermal"
summary: "A heater's power per unit length P′ heats the cross-section (the static solver, linear in P′); each medium's dn/dT turns the rise into Δε = 2n (dn/dT) ΔT; first-order perturbation sums it over the guide's mode, Δn_eff = Σ w_c ΔT(x_c, y_c), each optical cell's weight its share of a 1 K rise. So s = ∂n_eff/∂P′, Pπ = λ/(2s) for any length of a straight section (a 2D number), the crosstalk matrix s_ij/s_jj, and the phase in time as the weighted temperature in time: its 1/e and 10–90 % times and −3 dB frequency. Exact against a heated slab's closed form to 1.4e-5 and a line source's images to 2e-4 (second order, 2.10 and 1.92); Jacques et al. 2019's TiN and N++ heaters at 25.3 and 26.1 mW against 23.9 and 25.5 simulated, their 1/e times 9.5 and 4.3 µs from the switch against 9.0 and 4.6, gated by grids twice as fine."
order: 25.5
papers:
  - cite: "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019) (the TiN and N++ heaters, simulated and measured)"
    doi: 10.1364/OE.27.010456
  - cite: "S. G. Johnson, M. Ibanescu, M. A. Skorobogatiy, O. Weisberg, J. D. Joannopoulos, Y. Fink, Phys. Rev. E 65, 066611 (2002) (first-order perturbation of a mode)"
    doi: 10.1103/PhysRevE.65.066611
---

# The thermo-optic phase shifter

`photonoxide::thermal::PhaseShifter` computes what a heater does to the phase of the guides
near it, from their cross-section: the power a π shift takes, how fast the phase follows the
power, and how much of it the neighbours take. It is the chain the roadmap's 0.6 names, on
photonoxide's own solvers: the static solver's heat ([static](static.md)), the materials' dn/dT
([catalogue](catalogue.md)), and the full-vector mode with its perturbation
([fields](fields.md), `mode::thermo`).

## The chain

A **drive** is what heats: heaters drawn as shapes, each with its power per unit length P′
(W/m), or a current's Joule heat cell by cell (`Drive::joule`, the static solver's current
continuity), all turned on and off together. The heat problem is linear, so a drive's rise
above the background (the fixed and convective sides, a steady Joule heat) is ΔT = P′ ΔT₁,
ΔT₁ its rise per W/m.

A **guide** is a window of the section around a waveguide, on a uniform optical grid, each cell
the medium at its centre with its index n and dn/dT. Its fundamental mode (the highest
effective index) and first-order perturbation give each cell's weight

w_c = Re(E_c* · Δε_c · E_c) ΔA_c / (4 P), Δε_c = 2 n_c (dn/dT)_c per kelvin,

the share of the effective index's change a 1 K rise in that cell alone makes (Johnson et al.
2002; `mode::thermo::index_change_density`). The guide's response to a temperature field is
then Δn_eff = Σ_c w_c ΔT(x_c, y_c), the field interpolated at each optical cell's centre
(bilinear between the heat grid's centres, as `Field::at`); Σ_c w_c is the guide's dn_eff/dT
for a uniform rise. The weights are mapped once onto the heat grid's cells, so the same sum
reads a field in time at every step.

**Pπ.** With s_ij = ∂n_eff,i/∂P′_j, guide i's response to drive j (m/W), a π shift over a length
L takes P′ = λ/(2 s L), that is a power P = P′ L of

P_π = λ / (2 s),

whatever the length: L cancels (Jacques et al.'s Section 2.1: "L impacts neither Pπ nor τ").
**This is a 2D number,** a straight section's: the heat that leaves along the guide, the ends,
the leads and bends are not in it. It is a device's Pπ only where those are small, which a 3D
solve (or a device's measured fidelity) decides.

**Crosstalk.** X_ij = s_ij / s_jj, the share of the phase drive j gives its own guide j that
guide i takes. It is 2D as well: a neighbour at a pitch comparable to the heater's length sees
less than a cross-section says, the heat spreading along the guide too.

**In time.** The drive is switched at t = 0, on from the background or off from its steady
state, and the heat steps implicitly ([static](static.md#in-time): BDF2 or Crank–Nicolson,
each matrix factorized once); each step's phase is Σ w ΔT, so the time constant is the mode's,
not one point's. `PhaseShifter::times` gives the 1/e time (1 − 1/e of the way) and the 10–90 %
time, `PhaseShifter::cutoff` the −3 dB frequency of the phase's small-signal response, from
(K + iωM) T̂ = Q̂, and Pπ times the 1/e time is the figure of merit (Jacques et al.'s Eq. (3)).
On a linear problem a turn-off falls as a turn-on rises.

## Validation

| Case | What | Grid | Result |
|---|---|---|---|
| `shifter/slab-sheet` | analytic: a 220 nm silicon slab over an isothermal substrate under a heated sheet, thermally all oxide (T = q y/k exactly); s through the chain against Σ ∫ 2n (dn/dT) T E² / (2 n_eff ∫ E²) over the slab's closed-form TE₀ | heat from 10 nm; mode 5 nm | 1.4e-5 |
| `shifter/slab-sheet-order` | the same on modes of 20, 10 and 5 nm | | order 2.10 |
| `shifter/line-source` | analytic: a line source above an isothermal plane over a strip, thermally all oxide; the chain's s against the same weights over the images' P′/(2πk) ln(r′/r) | heat from 5 nm, growing by 1.05, a 10 × 5 mm window; mode 10 nm | 2e-4 |
| `shifter/line-source-order` | the same with the heat's grid refined everywhere (steps and growth) | | order 1.92 |
| `shifter/jacques-tin` | published: Jacques et al.'s TiN heater, Pπ (23.9 mW simulated) | heat from 10 nm (gated by 20 nm, 1 %); mode 5 nm (by 10) | 25.3 mW, the variants' midpoint |
| `shifter/jacques-npp` | published: their N++ silicon heater, Pπ (25.5 mW) | as above | 26.1 mW |
| `shifter/jacques-tin-time` | published: the TiN heater's 1/e time from the switch (9.0 µs) | as above; BDF2 steps of 0.2 µs | 9.47 µs |
| `shifter/jacques-npp-time` | published: the N++ heater's (4.6 µs) | steps of 0.1 µs | 4.25 µs |
| `shifter/jacques-time-step` | the N++ 1/e time on steps of 0.4, 0.2 and 0.1 µs, the convergence gate's error estimate | | 2.5e-5 of itself |

A grid study's refinement divides the grading's steps **and its growth's excess over 1** by the
same factor: refining the steps alone leaves the cells between the features, about (growth − 1)
times their distance from the nearest one, as they were, and the line source's error didn't
move (order 0.04) until the growth was refined too (order 1.92).

### Jacques et al. 2019's two heaters

M. Jacques et al., "Optimization of thermo-optic phase-shifter design and mitigation of thermal
crosstalk on the SOI platform", Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456,
simulate (Lumerical DEVICE, FEM) two cross-sections of a 320 µm phase shifter at 1550 nm.

- **What both share:** 2 µm of buried oxide over the substrate, an oxide cladding, a 40 × 18 µm
  region whose bottom (in the substrate) is held at the ambient, 10 W/(m²·K) of convection to the
  air above the oxide; Table 3's conductivities (the 220 nm guide 90, the 90 nm slab 55, N++
  silicon 25, the substrate 148, TiN 28, oxide 1.38 W/(m·K)), heat capacities (silicon 2330
  kg/m³ and 711 J/(kg·K), TiN 5240 and 598, oxide 2203 and 709) and indices (silicon 3.476, N++
  3.072, oxide 1.55); silicon's dn/dT, 1.8 × 10⁻⁴ K⁻¹, the cladding's neglected as the paper
  does. The mode's window: ±1.25 µm (TiN) or ±1.5 µm (N++) across, −0.75 to 0.97 µm up.
- **TiN:** a 500 × 220 nm strip, the heater 2.0 µm wide 2 µm above it. What the paper doesn't
  give is varied as in the static solver's case: the TiN's thickness, "withheld" in Table 3, 100
  or 200 nm; its 2 µm read from the strip's top and from the buried oxide's; the oxide above it
  1 µm. The four variants' midpoint is reported.
- **N++:** the 500 nm rib, 220 nm high, on the 90 nm slab, intrinsic 0.8 µm each side of the
  rib (Fig. 4's W_slab = 2.1 µm), the N++ strips 1.0 µm wide beyond. The current runs along the
  guide (Section 2.2), so each strip's heat is uniform in it, the two equal. The oxide's top,
  not given, is taken 3.32 µm over the buried oxide's, as the first TiN variant's; between 2.32
  and 8.32 µm Pπ moves from 25.0 to 27.9 mW.

**Pπ:** 25.3 mW for TiN (the variants 24.5 to 26.2), 6 % above the paper's 23.9, and 26.1 mW for
N++, 2.3 % above its 25.5; within the 10 % the unstated dimensions and two solvers' meshes allow.
The grids twice as fine everywhere (heat and mode) move Pπ by 4e-4 to 5e-4 of itself, and the
1/e times by 8e-4 to 2e-3, inside the 1 % gate each reported number passes.
The static solver's case (`static/jacques-tin-heater`, the strip's mean rise times ∂n_eff/∂n_Si)
gives 25.4 mW for TiN: the mode's weights and the strip's mean differ by 0.2 %, the field being
nearly uniform over the strip. The measured Pπ (29.2 to 31.5 mW for the 7.5 µm TiN heater the
paper made, 22.8 to 23.3 for N++, Section 3.1) is a device's, not this cross-section's.

**The time constants.** The paper's Fig. 1(c) plots the turn-on and turn-off from t = 0, the
heater switched at about 1.2 µs (where the N++ curves leave 0 and 1; ±0.2 µs reading the plot),
and its ~10.2 and ~5.8 µs are where the curves cross 1/e from the plot's origin (the turn-off
curves cross 0.37 at 10.2 and 5.8 µs on its axis). From the switch they are 9.0 and 4.6 µs.
The phase's 1/e times here are 9.47 µs (the variants' midpoint) and 4.25 µs: 5 % above and 8 %
below. The paper's measured times (rise 3.7 and fall 7.2 µs for TiN, 2.2 and 2.0 for N++,
Section 3.2) are 3.6 µs below its simulations, which it puts down to what the cross-section
doesn't hold. The N++ phase's 10–90 % time is 16.6 µs: heat's response is many poles, its slow
tail the substrate's, so a single pole's 0.35/f₃dB is no guide to it.

## As a job

A `kind = "static"` job ([static](static.md#as-a-job)) with `physics = "heat"` (or
`"current"`, its Joule heat the one drive) takes a `[task.shifter]`: `wavelength_um`, the optical
grid's `step_nm`, and optional `index` and `dn_dt` tables by medium. A medium they don't give
takes its catalogue entry's default model at `temperature_k` (300 K by default): its index and
its dn/dT, recorded with the model's id. `[[task.guide]]` rectangles (`y_um`, `z_um`, `name`)
are the guides' mode windows. Each `[[task.heater]]` is a drive, the k-th heater's own guide the
k-th guide. The run records each guide's n_eff and dn_eff/dT, every guide's ∂n_eff/∂P′ by every
drive, each drive's P_π on its own guide (W) and its crosstalk on the others; with a
`[task.transient]`, each drive's phase's 1/e and 10–90 % times, −3 dB frequency, and P_π times
its 1/e time. It replays to the bit, the same bits on any number of threads.

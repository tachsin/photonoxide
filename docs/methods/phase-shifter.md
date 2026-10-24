---
title: "The thermo-optic phase shifter: Pπ, the time constant and crosstalk"
module: "thermal"
summary: "A heater's power per unit length P′ heats the cross-section (the static solver, linear in P′); each medium's dn/dT turns the rise into Δε = 2n (dn/dT) ΔT; first-order perturbation sums it over the guide's mode, Δn_eff = Σ w_c ΔT(x_c, y_c), each optical cell's weight its share of a 1 K rise. So s = ∂n_eff/∂P′, Pπ = λ/(2s) for any length of a straight section (a 2D number), the crosstalk matrix s_ij/s_jj, and the phase in time as the weighted temperature in time: its 1/e and 10–90 % times and −3 dB frequency. Exact against a heated slab's closed form to 1.4e-5 and a line source's images to 2e-4 (second order, 2.10 and 1.92); Jacques et al. 2019's TiN and N++ heaters at 25.3 and 26.1 mW against 23.9 and 25.5 simulated, their 1/e times 9.5 and 4.3 µs from the switch against 9.0 and 4.6, gated by grids twice as fine. Crosstalk on a chip's substrate 30 to 34 % above the measured at 5 and 15 µm gaps (2D: the 320 µm length and the victim's far arm lower it), a deep trench's reduction 3.5 against 3.1; Harris et al. 2014's periodic doped heater as an averaged 2D section at 26.4 mW and 109 kHz against 24.77 mW and 130 kHz measured, inside its two 2D bounds, so no 3D static solver is needed for these heaters."
order: 25.5
papers:
  - cite: "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019) (the TiN and N++ heaters, simulated and measured)"
    doi: 10.1364/OE.27.010456
  - cite: "N. C. Harris, Y. Ma, J. Mower, T. Baehr-Jones, D. Englund, M. Hochberg, C. Galland, Opt. Express 22, 10487 (2014) (a doped heater in the guide, measured)"
    doi: 10.1364/OE.22.010487
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
| `shifter/line-source-crosstalk` | analytic: the line source's crosstalk on a strip 5 µm away against the images' | heat from 5 nm | ≤ 1e-3 |
| `shifter/jacques-crosstalk-5`, `-15` | published: the N++ heater's crosstalk on a victim strip at gaps of 5 and 15 µm on a 725 µm substrate (measured 0.141 and 0.0435) | heat from 10 nm (gated by 20, 1 %), ±2 mm | 0.183 and 0.0582 |
| `shifter/jacques-trench` | published: a deep trench's reduction of it at 5 µm (measured 3.13) | as above | 3.48 |
| `shifter/harris-p-pi` | published: Harris et al.'s heater, the averaged section's Pπ (24.77 ± 0.43 mW measured) | heat from 10 nm (gated by 20), mode 5 nm | 26.4 mW |
| `shifter/harris-bandwidth` | published: its −3 dB frequency (130.0 ± 5.6 kHz) | as above | 109 kHz |
| `shifter/harris-bounds` | the measured Pπ between the two 2D sections' (0 to 1) | heat from 20 nm | 0.46 |

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

### Crosstalk against Jacques et al.'s neighbours

Jacques et al. measure a passive victim MZI beside the N++ heater (Section 3.1, Fig. 9): its phase
at a heater power, for gaps d_inter of 5, 15, 45 and 135 µm, with oxide in the gap, the oxide
etched, or a deep trench. Over the heater's own phase (π at its measured 22.8 mW) the victim
takes 0.141 at 5 µm and 0.0435 at 15 µm (Fig. 9(b) at 50 mW, read to about ±0.03 rad), and a
deep trench cuts it by 0.97/0.31 = 3.13 at 5 µm (Fig. 9(d); the text's "a factor of ~3").

The neighbours are far enough for the substrate to matter, so the section is a chip's, not the
40 × 18 µm region of the paper's own simulations: the substrate 725 µm thick (an 8-inch wafer's;
not given) held at its bottom, ±2 mm wide, the N++ heater as before and a 500 × 220 nm victim
strip. The gap is read as from the heater's outer edge (the N++ strip's, 2.05 µm from the guide's
centre) to the victim's near edge, the paper's word for the inter-MZI distance (the intra-MZI one
is a pitch). The cross-section gives 0.183 at 5 µm and 0.0582 at 15 µm, 30 % and 34 % above the
measured; 0.0383 at 45 µm (+8 %) and 0.0246 at 135 µm (+70 %). Two things a cross-section
doesn't hold both lower the measured: the victim's phase is its two arms' difference, its far arm
warmed too; and the heater is 320 µm long, so heat spreading along the chip leaves less for a
neighbour as far as the length, which is why the 135 µm gap is furthest off. Read as centre to
centre instead, the 5 µm gap gives 0.317. The cases take the edge reading and 40 %.

**The deep trench** is air from the cladding's top through the silicon and the buried oxide into
the substrate ("partially": 2 µm taken), 0.5 µm clear of the heater and the victim: it cuts the
crosstalk at 5 µm by 3.48 (2 µm deep; 3.5 to 4.1 from 2 to 10 µm), against the measured 3.13.

### Harris et al. 2014's doped heater, and #411's decision 3

N. C. Harris et al., "Efficient, compact and low loss thermo-optic phase shifter in silicon",
Opt. Express 22, 10487 (2014), doi:10.1364/OE.22.010487, heat the guide itself: a ridge whose p
region (7 × 10¹⁷ cm⁻³, 1.0 µm wide across the guide) carries a current across it from p++ (1.7 ×
10²⁰) channels in the 90 nm slab, 800 nm wide along the guide every 2.44 µm, to contacts beside
it; 61.6 µm long, Pπ 24.77 ± 0.43 mW and a −3 dB frequency of 130.0 ± 5.6 kHz measured (their τ =
0.35/f₃dB = 2.69 µs). Their sheet resistances give each region's σ = 1/(R_s t): 8.17 × 10⁴ S/m
(p++), 817 (p, 90 nm) and 1175 (p, 220 nm); the current's Joule heat comes from the static
solver's current continuity, and it is the drive. Their COMSOL model held the ambient below the
buried oxide and 10 µm above the cladding (Section 3); the same here, the oxide 10 µm thick over
the silicon.

The channels make it periodic along the guide, so no single cross-section is it. Two bound it:
**between channels** the slab only beside the ridge (Fig. 1(b)'s intrinsic hexagons, taken 1.0 µm
each side), all the heat kept near the guide, Pπ 17.0 mW; **through a channel** the slab and its
contacts everywhere, the heat drained along it, 33.8 mW. The measured 24.77 mW falls 0.46 of the
way between them (`shifter/harris-bounds`). The **averaged section** takes the channels' share
of the length, f = 0.8/2.44, into the slab beyond the hexagons: its conductivity f k_Si + (1 − f)
k_ox, its σ f times the p++'s, its heat capacity likewise, the contacts (2.5 µm out) held at the
ambient as the made device's vias and aluminium hold them (their model left the metal out). It
gives Pπ 26.4 mW, 6 % above the measured, and 109 kHz, 16 % below. Assumed and not given: the
ridge 500 nm wide, the channels' length, the hexagons' width, the catalogue's films (Ju & Goodson
at 220 and 90 nm) and silica, Jacques et al.'s heat capacities. The contacts matter most: left
free, the averaged section gives 21.3 mW and 54 kHz. So does what is above: with 3 µm of oxide
insulated on top in place of 10 µm held, 25.2 mW and 90 kHz.

**Decision 3** (#411): Harris et al.'s periodic heater doesn't need a 3D static solver for its Pπ
and bandwidth here. Its two 2D sections bound the measurement, and the averaged one is within 6 %
and 16 %, inside what the paper leaves unstated. A 3D solve would settle the channels' and
contacts' share, not change the method; 0.6 keeps the static solver 2D. What stays 3D is a
device's ends and a neighbour as far as its length (the crosstalk above).

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

## As a device

The two heaters of Jacques et al. are devices too, `device::ThermoShifter` ([devices](devices.md#thermo-optic-phase-shifters)):
the cross-sections above with their geometry as parameters (the heater's width, its height
or buffer, the guide's width and thickness, a neighbour's pitch), a closed-form analytic model,
this chain as their 2D model (to the bit, at the papers' designs), and the measured devices,
each fidelity against the next one up; a component whose phase follows the heater's power, a
page in the studio, and the device calls from Python and MATLAB.

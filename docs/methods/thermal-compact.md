---
title: "Thermal compact models: heaters in a circuit, and their crosstalk corrected"
module: "thermal::RcModel, thermal::Network, thermal::Correction, circuit::Heated"
summary: "A phase shifter's step responses (P4's transients, every drive on every guide) fitted by a sum of first-order terms on poles every pair shares, Z_ij(s) = Σ R_k,ij/(1 + sτ_k), the poles by variable projection: stable by construction, and passive where the outputs are the heaters' own temperatures. Each pair is a Foster network, turned into its Cauer ladder by a continued fraction. A circuit's phase shifters take their phases from the model, exactly in time for powers held over each step and in frequency through the circuit adjoint. A known sum of exponentials comes back to 6e-11; Coenen et al. 2022's Foster networks give their Cauer ladders within 1.1 %, -28.6 dB at 200 kHz (their -30) and a 37.7 µs time constant (their 40); on Jacques et al.'s N++ heater the error falls from 0.15 to 1.9e-5 as poles are added to 8, and an MZI of two such arms follows the heat solver within 3e-4 in time and 4.5e-3 in frequency. Crosstalk correction is posed for genoxide (Gurses et al. 2022): the phase error Σ(1 − cos(φ − φ*)) of the heaters' powers with its exact gradient, an optimize job of its own; Gurses et al.'s lumped networks give their neighbour's 62.6 mW to 0.14 % and their bandwidth to 4 %, and genoxide's L-BFGS-B takes their array to their P = C⁻¹Φ."
order: 25.55
papers:
  - cite: "D. Coenen, H. Oprins, P. De Heyn, J. Van Campenhout, I. De Wolf, IEEE Trans. Compon. Packag. Manuf. Technol. 12, 1350 (2022) (dynamic compact RC models of a ring filter array, Foster and Cauer)"
    doi: 10.1109/TCPMT.2022.3195699
  - cite: "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019) (the N++ heater the models are fitted to)"
    doi: 10.1364/OE.27.010456
  - cite: "V. Gurses, R. Fatemi, A. Khachaturian, A. Hajimiri, IEEE J. Sel. Top. Quantum Electron. 28(6), 1 (2022) (a 288-element heater array, its lumped thermal model and crosstalk correction)"
    doi: 10.1109/JSTQE.2022.3189965
---

# Thermal compact models

A circuit of many heated phase shifters needs each one's phase in time from its own heater's
power and its neighbours', without solving the heat equation at every step.
`photonoxide::thermal::RcModel` is that model: fitted once to the step responses of a
[phase shifter's](phase-shifter.md) cross-section, then evaluated in microseconds. It follows
D. Coenen et al., "Analysis of thermal crosstalk in photonic integrated circuit using dynamic
compact models", IEEE Trans. Compon. Packag. Manuf. Technol. 12, 1350 (2022),
doi:10.1109/TCPMT.2022.3195699: a step response's least-squares fit by exponentials, its
Foster network and the Cauer ladder it converts to. `photonoxide::circuit::Heated` puts it in a
circuit.

## The model

Heat conduction is linear, so the outputs y (each guide's phase, or a heater's temperature) are
the inputs' powers P (W) through a matrix of responses. On a finite grid each response is a sum
of decaying modes; the model keeps a few:

Z_ij(s) = Σ_k R_k,ij / (1 + s τ_k),  y_i(t) = Σ_k Σ_j R_k,ij (1 − e^(−t/τ_k)) after P_j = 1 W is
switched on,

one set of poles τ_k for every pair (output i, input j), a residue matrix R_k per pole. Each
τ_k is positive, so the model is **stable by construction**: no fit can give a growing mode.
Each pair alone is a **Foster network**, resistances R_k each in parallel with C_k = τ_k/R_k,
in series (Coenen et al.'s Eq. 3), and `Foster::cauer` turns it into the **Cauer ladder**
of the same impedance (their Section II.C): C₁ from the input to the ambient, R₁ on to the next
node, and so on. 1/Z = D(s)/N(s) is expanded in a continued fraction from s → ∞, each step a
capacitance (the leading coefficients' ratio) and then a resistance; time is scaled by the
slowest pole so the polynomials stay well conditioned. The ladder's elements follow the heat's
path outward, which the Foster blocks, a fit's, don't.

**Passive.** Where the outputs are the heaters' own temperatures (the driving points), the
exact responses' residues are symmetric and positive semidefinite (each mode's contribution
is φ_k φ_kᵀ), and a fit with `RcFit::passive` keeps them so: each R_k is symmetrized and its
negative eigenvalues set to 0 (the nearest such matrix), inside the fit so the poles are those
that fit best once passive. Then Re Z(iω) = Σ R_k/(1 + ω²τ_k²) is positive semidefinite at
every frequency: the network gives out no heat it isn't given. A **guide's phase** beside a
heater is not a driving point: the heat takes time to reach it, and only residues of both signs
can give a response that starts slowly (a sum of decays of one sign has its full slope at
once). A phase model is therefore stable and not made passive; forcing R ≥ 0 on Jacques et
al.'s heater stopped the error at 4.5e-3 whatever the poles.

## The fit

`RcModel::fits` fits models of 1, 2, … poles in turn. For given poles the residues are linear
least squares, the same normal matrix for every pair, through the steady response exactly where
it is given (the steady solve's, `PhaseShifter::efficiency`, which a record of finite length
doesn't reach). The poles' logarithms move by Levenberg–Marquardt (variable projection) to the
least error over every pair and time, each pair's error relative to its input's largest
response, so a neighbour's small phase counts in units of the heater's own. Each model starts
from the one before with a pole added at each of twelve places across the record, the best kept.
A run's record of 0.1 µs steps limits how fast a pole it can see.

`PhaseShifter::step_responses` gives the data: each drive turned on from the background, every
guide's phase per W of it (rad/W, 2π s P/λ, any length of a straight section), at every step of
P2's transient, and the steady values. A static job's `[task.shifter]` with `poles` fits it in a
run ([static](static.md#as-a-job)): the record holds each fit's error from one pole up, and the
model's time constants and residues.

## In a circuit

`circuit::Heated` holds a circuit, the instances whose `phase` the heaters drive (ideal phase
shifters, `circuit::components::PhaseShifter`) and the model, the k-th instance its k-th output
over a bias. The light follows the heat at once (its picoseconds against the heat's
microseconds), so each instant is a circuit solve at that instant's phases:

- **Steady:** φ = φ₀ + Z(0) P (`Heated::phases`, `Heated::s_matrix`).
- **In time** (`Heated::transient`): powers held over each step, as a driver's DAC holds them,
  each pole's state x' = (P − x)/τ advanced exactly, x ← P + (x − P) e^(−h/τ); the phases are
  Σ_k R_k x_k, and the circuit is solved at each time.
- **In frequency** (`Heated::modulation`): about a steady state, a power's response to a heater
  modulated at f is Σ_k (∂|S_qp|²/∂φ_k) Z_kj(i2πf), ∂|S_qp|²/∂φ_k = 2 Re(S̄_qp ∂S_qp/∂φ_k) from
  the circuit adjoint ([circuit adjoint](circuit-adjoint.md)).

Its provenance is a compact model's (`Fidelity::Compact`), with the fit's error.

## Validation

| Case | What | Result |
|---|---|---|
| `thermal-rc/known-sum` | analytic: a 2 × 2 model of three poles (2, 20, 300 µs), positive definite residues, fitted back from its step responses | 6.4e-11 |
| `thermal-rc/coenen-cauer-ucut` | published: Coenen et al.'s Table I, the undercut ring's four-block Foster network into its Cauer ladder | within 1.05 % of the printed ladder |
| `thermal-rc/coenen-cauer` | published: the two-block network without the undercut | within 0.50 % |
| `thermal-rc/coenen-200khz` | published: the undercut network's response at 200 kHz (their −30 dB, Section III.A) | −28.6 dB |
| `thermal-rc/coenen-start-up` | published: its 1/e time (their device-level 40 µs, Section III.B) | 37.7 µs |
| `thermal-rc/jacques-poles` | convergence: Jacques et al.'s N++ heater's phase (heat from 20 nm, mode on 10 nm, BDF2 0.1 µs to 60 µs) fitted by 1 to 8 poles | 0.15, 0.020, 5.2e-3, 4.5e-3, 4.2e-4, 1.4e-4, 6.4e-5, 1.9e-5 |
| `thermal-rc/jacques-cutoff` | the 8-pole model's −3 dB frequency against the heat solver's (K + iωM) response on the same grid | 3.3e-4 |
| `circuit/heated-mzi-transient` | a balanced MZI whose arms are two N++ shifters 20 µm apart, the upper heater on at its Pπ: the cross power in time through a 6-pole model against the heat solver's phases | 3.0e-4 |
| `circuit/heated-mzi-modulation` | the MZI at quadrature, the cross power's response at 1 to 100 kHz against the heat solver's small-signal response of both arms | 4.5e-3 |

**Coenen et al.'s Table I** gives each Foster network's resistances in K/mW and capacitances in
their "µF" (the electrical units of their LTspice model: with K/mW they make µs). The ladders
come out within 1.05 % (undercut) and 0.50 % of the printed values; the last elements of the
four-block ladder are the most sensitive to the Foster values' three or four digits. The sum of
the resistances is the heater's efficiency, 4.798 K/mW with the undercut (their 4.8 °C/mW) and
1.523 without (their 1.52). The undercut network falls to −28.6 dB at 200 kHz, against their
"−30 dB" read from the circuit's response (Fig. 13, its neighbours' coupling in it), and turned
on reaches 1 − 1/e at 37.7 µs, against the 40 µs of their start-up (Fig. 14). Their main
channel's "30 kHz" −3 dB frequency (Section III.A) doesn't follow from Table I, whose network
falls by 3 dB at 3.7 kHz; it isn't a case. Their coupling (Section II.D) is a current source per
pair of channels with a first-order delay, sensing the heated channel's temperature; here the
coupling is the same model's off-diagonal pairs, on the shared poles, which a cross-section's
transient gives directly.

**Jacques et al.'s N++ heater** (as `shifter/jacques-npp`, the grids of its `r = 1`) is a
heater beside its guide, so its phase starts slowly: one pole leaves 15 % of the steady phase,
and each pair of poles added takes about a decade off the error until the record's own 0.1 µs
steps are the limit. Eight poles from 0.4 µs to 12 µs (and a slow one) follow the transient to
1.9e-5; the model's −3 dB frequency is the heat solver's to 3.3e-4, which is the difference
between BDF2's 0.1 µs steps and the exact frequency response.

**Two shifters 20 µm apart** (Jacques et al.'s N++ section twice, the substrate held 12.68 µm
below the buried oxide as in their region, 20 µm of it beyond each) take 0.59 % of each other's
phase, after a delay. Six poles fit both drives' four responses to 1.8e-4 (the error 0.15,
0.017, 5.8e-3, 3.1e-3, 4.0e-3, 1.8e-4 from one pole: Levenberg–Marquardt minimizes the squares,
so the largest error need not fall at every pole). In an MZI of ideal couplers whose arms they
are, the upper heater turned on at its own arm's Pπ, the cross power through the model and
through the heat solver's phases differ by 3.0e-4 at most over 200 µs; at quadrature its
response to the heater's modulation at 1, 10, 30 and 100 kHz differs from the heat solver's
small-signal response by 4.5e-3 of the steady response. These numbers are on one grid: they
measure the model against the solve it is fitted to, which is the model's error; the solve's
own against finer grids is P4's (`shifter/jacques-npp-time`).

## Lumped networks

`thermal::Network` is a network drawn by hand: nodes with heat capacities (J/K, 0 for a node of
no mass), conductances between them and to the ambient (W/K), C dT/dt = −G T + P. Its nodes of no
mass are eliminated (their temperatures follow the others' at once), and the rest give its exact
compact model by their modes: C^(−1/2) G C^(−1/2) = V Λ Vᵀ, each a pole τ_k = 1/λ_k with residues
u_k u_kᵀ/λ_k, u_k = C^(−1/2) v_k. Between the same nodes the residues are positive semidefinite:
a network of resistors and capacitors is passive, and so is its model. An output at a node of no
mass is refused, since it would answer at once and no pole does.

## Crosstalk correction, posed for genoxide

V. Gurses et al., "Large-scale crosstalk-corrected thermo-optic phase shifter arrays in silicon
photonics", IEEE J. Sel. Top. Quantum Electron. 28(6), 1 (2022), doi:10.1109/JSTQE.2022.3189965,
set each heater of an array so that its shifter takes the wanted phase whatever its neighbours
do: with the steady response C (rad/W, their Eq. 13), φ = φ₀ + C P, and P = C⁻¹(φ* − φ₀).
`thermal::Correction` poses it as an optimization, which an inverse can't be once the powers are
bounded: none below 0, none above what a heater stands, and a phase wanted only to a whole turn.
The objective is

F(P) = Σ_i (1 − cos(φ_i − φ*_i)),  ∂F/∂P_j = Σ_i sin(φ_i − φ*_i) C_ij,

zero exactly where every phase is its target to a whole turn, smooth, its gradient exact. The
response C is a compact model's steady one (`Correction::of_model`, a static job's thermal model),
a matrix given, or Gurses et al.'s row (`Correction::row`: C_ii = π/P_π, C_i,i±n = π/P_π^(n)).
`Correction::inverse` is their algorithm, `Correction::uncorrected` each heater set for its own
shifter alone, where an optimization starts.

**As a job.** An `"optimize"` task with a `[task.crosstalk]` and no `[device]` or `[circuit]`
(`phase_per_watt` or `count`, `p_pi_mw` and `neighbour_p_pi_mw`; `target_rad`, `bias_rad`,
`max_power_mw`) takes the heaters' powers `P1`, `P2`, … (mW) as its variables between 0 and the
maximum, and the phase error as its objective with its exact gradient, checked against
differences before it is used. genoxide's methods run it in the `photonoxide` program, as a
device's optimization; `jobs/crosstalk-correction.yaml` is Gurses et al.'s array.

**What genoxide needs.** Nothing new: L-BFGS-B takes the bounds and the gradient, and reaches
Gurses et al.'s inverse to 1e-6 of each power. A bounded nonlinear least-squares method
(Gauss–Newton or Levenberg–Marquardt with bounds, on the residuals sin and 1 − cos and their
Jacobian) would suit this problem better and would be a general one; it isn't needed here.

| Case | What | Result |
|---|---|---|
| `correction/gurses-cutoff` | published: Gurses et al.'s reduced shifter (Fig. 18) at their G = 62.8 µW/K and C_W = 440 pJ/K, as a network; its −3 dB frequency against the 56.8 kHz they extracted these from | 54.5 kHz |
| `correction/gurses-neighbour` | published: two shifters coupled as their Fig. 19 at G_SS = 4.57 µW/K; the neighbour's power for π (their 62.6 mW measured) | 62.69 mW |
| `correction/gurses-array` | published: their array as Eq. 13 takes it (9 in a row, first-order crosstalk at their tuned 13.2 µW/K), each at π, set by P = C⁻¹Φ | phases exact (≤ 1e-12 rad) |
| `correction/gradient` | analytic: the objective's gradient against central differences | 4.1e-10 |

**Gurses et al.'s networks.** Their Fig. 18 reduces a shifter to a heater node of no mass (R/2 to
the ambient, 2R to the waveguide) and the waveguide (R/2 to the ambient, with C_W), R = 1/G: the
waveguide rises by P R/12 (their Eq. 31) with one pole, τ = 5RC_W/12, so f₃dB = 12G/(10πC_W) =
54.5 kHz at their G and C_W. They extracted C_W from 56.8 kHz by their Eq. 35, C_W = 2.5 Gτ,
which follows from Eq. 33's divider without the heater node's own (the network gives 2.4 Gτ):
4 % between the two. Their Fig. 19 couples two shifters by R_X = 2R + 1/G_SS from each node of
one to each of the other's; solved as a network it is their Eq. 36 to 1e-12, and at their G_SS =
4.57 µW/K the neighbour's power for π is 62.69 mW, their 62.6 measured. (Eq. 38's approximation,
G_SS ≈ 4G²ΔT_π/P_π^(1), would give 3.5 µW/K; Eq. 36 is what they solved.)

**Their array.** For the 32 × 9 array they tune G_SS to 13.2 µW/K, which through Fig. 19's pair
puts the neighbour's π at 30.94 mW. With every shifter wanted at π, each heater at its own 10.6
mW leaves the inner shifters 2.15 rad over; their P = C⁻¹Φ (5.30 to 8.79 mW) makes every phase
exact, which is their Fig. 15's cancellation; genoxide's L-BFGS-B reaches it from the uncorrected
powers on the exact gradient. Their figure's MZI curves aren't reproduced: the array's drive (the
pulses' amplitudes and the reference shifter's neighbours) isn't given in numbers.

**First order isn't the array.** Eq. 13 keeps only the first neighbours, each from a pair alone.
The same nine shifters joined as one network (each pair as Fig. 19) respond otherwise: a
shifter's own response falls from π/P_π = 296 to 210 rad/W (its heat shared with both neighbours),
the second neighbours take 9.4 rad/W, and the first-order setting leaves every phase 0.41 to 0.67
rad short; corrected on the network's own response, the powers are 7.1 to 9.7 mW. A correction is
as good as its response matrix, which is why the response here comes from the array's own
compact model where there is one.

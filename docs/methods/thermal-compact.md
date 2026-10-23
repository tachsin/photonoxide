---
title: "Thermal compact models: heaters in a circuit, in time and frequency"
module: "thermal::RcModel, circuit::Heated"
summary: "A phase shifter's step responses (P4's transients, every drive on every guide) fitted by a sum of first-order terms on poles every pair shares, Z_ij(s) = Σ R_k,ij/(1 + sτ_k), the poles by variable projection: stable by construction, and passive where the outputs are the heaters' own temperatures. Each pair is a Foster network, turned into its Cauer ladder by a continued fraction. A circuit's phase shifters take their phases from the model, exactly in time for powers held over each step and in frequency through the circuit adjoint. A known sum of exponentials comes back to 6e-11; Coenen et al. 2022's Foster networks give their Cauer ladders within 1.1 %, -28.6 dB at 200 kHz (their -30) and a 37.7 µs time constant (their 40); on Jacques et al.'s N++ heater the error falls from 0.15 to 1.9e-5 as poles are added to 8, and an MZI of two such arms follows the heat solver within 3e-4 in time and 4.5e-3 in frequency."
order: 25.55
papers:
  - cite: "D. Coenen, H. Oprins, P. De Heyn, J. Van Campenhout, I. De Wolf, IEEE Trans. Compon. Packag. Manuf. Technol. 12, 1350 (2022) (dynamic compact RC models of a ring filter array, Foster and Cauer)"
    doi: 10.1109/TCPMT.2022.3195699
  - cite: "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019) (the N++ heater the models are fitted to)"
    doi: 10.1364/OE.27.010456
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

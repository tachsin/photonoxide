# Carrier modulators, signals and programmable circuits: a plan

*A plan, 2026-11-09, for the roadmap's 0.8, written for item C of #774. Nothing in it is built:
every module, type and function named below is a proposal, and I review the plan before work on
it starts.*

The roadmap's 0.8 lists drift-diffusion on the cross-section, plasma dispersion, the junction's
C(V) and series resistance, travelling-wave lines loaded by the junction, time-domain circuits,
signals, programmable meshes, the silicon depletion modulator (Mach–Zehnder and ring) and its
validation against Yu et al. 2012 and Patel et al. 2015. The audit of 2026-11-09 (#774) found
active devices and systems the weak side of the silicon photonics photonoxide covers: no carrier
modulator, no germanium, no circuit in time. This page says what 0.8 builds on, the phases in the
order I would build them (the cheapest valuable one first), what each is checked against, the
papers it needs, and the decisions to take. It has nine parts:

1. [What photonoxide has now](#1-what-photonoxide-has-now)
2. [From doping to a modulator's figures](#2-from-doping-to-a-modulators-figures): the chain of
   physics and where each link comes from.
3. [The phases](#3-the-phases): P1 to P6, and the germanium question (G).
4. [Validation targets by paper](#4-validation-targets-by-paper)
5. [Order, dependencies, cost and risk](#5-order-dependencies-cost-and-risk)
6. [API sketch](#6-api-sketch-a-proposal)
7. [Studio and Academy](#7-studio-and-academy)
8. [Out of scope](#8-out-of-scope)
9. [Decisions](#9-decisions)

**Rules this page follows.**

- Every DOI was checked on Crossref on 2026-11-09; a paper's year is Crossref's published-online
  year, or its issue's year where Crossref gives no online date.
- A number from a paper is quoted only from a paper read for this plan, with its page, figure or
  equation: Soref & Bennett 1987 (whole), Ding et al. 2014 (whole), Chrostowski & Hochberg 2015
  (chapter 6, Sections 6.1 to 6.2.4, and Listing 6.1), Bao et al. 2024 (Sections I to III) and
  Reed et al. 2010 (the plasma-dispersion expressions). Numbers of papers not yet in the folder are
  marked *(to read)*; where the literature survey of 2026-10-02 quoted them from the full text,
  they are given as the survey's and must be confirmed from the PDF before a case's tolerance is
  set.
- Arithmetic of mine on published expressions is marked *(ours)*.
- Every case takes its tolerance by the rule of #544 (a measure and a basis), not a number chosen
  on this page.
- Commercial tools aren't run (roadmap decision 6). Papers' numbers that came from one are quoted
  as published numbers. Open codes are run as external programs once their licences are checked.

## Summary

1. **The cheapest valuable step needs no new solver.** The plasma-dispersion expressions as a
   material model, and the abrupt p–n junction in the depletion approximation, give a depletion
   phase shifter's Δn_eff, loss, Vπ·L, C(V) and RC cutoff through the mode solver photonoxide
   already has. Chrostowski & Hochberg's chapter 6 publishes exactly that model with its numbers
   (a 500 × 220 nm rib on a 90 nm slab, 1.6 V·cm), so the first phase has a published target the
   day it starts.
2. **Drift-diffusion is the hard part and comes third,** after the modulator exists at analytic
   fidelity: in 1D first, then on the static solver's graded cross-section, steady state and small
   signal only, checked against the abrupt junction, the ideal diode and a uniform resistor.
3. **The devices reuse 0.6.** The depletion Mach–Zehnder modulator follows the thin-film lithium
   niobate modulator's pattern: three fidelities, its line from `electro::line` loaded by the
   junction's series R and C. Ding et al. 2014, in the folder, is a complete published target:
   doping, C_pn, R_pn, Vπ, loss and bandwidth at two biases.
4. **Signals only as far as they are validated:** pole–residue models stepped in time, rings by
   coupled-mode theory, PRBS and PAM sources, eye diagrams, extinction ratio and optical
   modulation amplitude. Link budgets are products of validated transmissions with closed-form
   penalties; no receiver sensitivity beyond Gaussian-noise definitions.
5. **Meshes are mostly there as examples.** The Clements fit and Pérez-López's self-configuring
   hexagonal mesh run today; the closed-form Reck and Clements decompositions are 0.6.3's Q1. 0.8
   promotes the meshes to the library and adds self-configuration and error correction.
6. **Germanium photodetectors** (Section 9, decision 8; decided 2026-11-10):
   germanium's optical data in the catalogue and a detector's absorption with 0.8, the detector as
   a device after silicon drift-diffusion is validated.

## 1. What photonoxide has now

- **The static solver** (`statics`, `thermal`, `electro`): −∇·(c∇u) = s on a cross-section by
  cell-centred finite volumes on graded tensor-product grids fitted to interfaces and edges,
  conductors held at a potential, current continuity with its Joule heat, capacitance matrices,
  and the small-signal response from K + iωM. Drift-diffusion's Poisson equation is this operator
  with a nonlinear source; its grid, features and grading are reused as they are.
- **Index changes into a mode:** `mode::thermo` (a temperature field) and `mode::pockels` (a
  static field through the rotated r tensor) move a mode by first-order perturbation and by
  re-solve, the two compared. A carrier map is a third kind of the same perturbation.
- **RF lines:** `electro::line`, the quasi-TEM line from the static solver's capacitances
  (`QuasiTem`), its losses, `QuasiTem::loaded` (Shin et al.'s loaded line), `Periodic` (a
  period's ABCD cascade and its Bloch wave), the full-wave lossy-metal mode (`electro::full_wave`)
  and the electro-optic response (Ghione's Eq. 6.31, `response`). The junction-loaded line is one
  more shunt admittance.
- **Active devices** (`device`): `TflnModulator` at analytic, 2D and measured fidelity with
  stated errors and measured files, `ThermoShifter`, `PcmShifter`. The depletion modulator is
  built the same way.
- **Circuits** (`circuit`, `compact`): netlists, the circuit adjoint, compact models by vector
  fitting (Gustavsen & Semlyen reproduced; stable poles by construction, passivity checked but not
  enforced), Touchstone files, `AllPassRing` and `AddDropRing` (Bogaerts et al. 2011's
  responses), and `circuit::Heated`, a circuit's phase shifters driven in time through
  `thermal::RcModel`. `thermal::Correction` poses heater crosstalk correction for genoxide.
- **Free carriers, once:** the nonlinear pulse model (`nonlinear::pulse`) has free-carrier
  absorption and dispersion made by two-photon absorption, with Lin, Painter & Agrawal 2007's
  coefficients. That is the only place photonoxide turns carriers into optics today.
- **Meshes, as examples:** `examples/optimize_mesh.rs` fits Clements meshes to Haar-random
  unitaries from Pai et al.'s initialization; `examples/optimize_mesh_selfconfig.rs` sets
  Pérez-López et al. 2020's 36-unit hexagonal mesh to its all-cross state by a genetic algorithm
  and a particle swarm; `examples/circuit_splitter.rs` reproduces Clements et al.'s variable beam
  splitter. The closed-form Reck and Clements decompositions moved from 0.8 to 0.6.3's Q1 (the
  quantum plan's decision 5) and are not built yet.
- **The Academy:** the modulators lesson's Section 9 describes the plasma-dispersion effect and
  the three carrier structures, and says photonoxide doesn't compute carrier modulators yet.

What is missing: semiconductor transport of any kind, doping profiles, a carrier material model in
the catalogue, germanium, a general circuit simulated in time, signal sources and eye diagrams.

## 2. From doping to a modulator's figures

### 2.1 Carriers in the cross-section

Drift-diffusion with Boltzmann statistics: Poisson's equation for the potential ψ,
−∇·(ε∇ψ) = q(p − n + N_D⁺ − N_A⁻), and the continuity of electrons and holes, ∇·J_n = qR and
∇·J_p = −qR, with J_n = qμ_n n E + qD_n∇n and its hole counterpart; Shockley–Read–Hall
recombination R (Shockley & Read 1952; Hall 1952); mobility by doping (Masetti et al. 1983) and
field (Caughey & Thomas 1967). The currents between two nodes by Scharfetter & Gummel's flux,
exact for a constant field between them; Gummel's decoupled iteration first, then Newton on the
coupled system (Gummel 1964; Selberherr 1984). Contacts are ohmic: charge neutrality and
equilibrium carrier densities at the applied potential.

The **abrupt junction in the depletion approximation** is the closed form drift-diffusion is
checked against and the analytic fidelity's model. As Chrostowski & Hochberg print it (Section
6.2.1, pp. 219–221): the depletion width W_d = √(2ε₀ε_s(N_A + N_D)(V_bi − V)/(qN_AN_D)) (Eq. 6.6),
the built-in potential V_bi = (k_BT/q) ln(N_AN_D/n_i²) (Eq. 6.7), the depletion edges (Eq. 6.8)
and the carriers on either side (Eqs. 6.9 to 6.11); the junction's capacitance per length and its
series resistance from the four doped regions' sheet resistances (Eq. 6.14, p. 223), and the RC
cutoff f_c = 1/(2πR_jC_j) (Eq. 6.15).

### 2.2 Carriers into index and absorption

**What Soref & Bennett 1987 print.** A Kramers–Kronig analysis of measured absorption spectra of
doped silicon, from 1.0 to 2.0 µm (abstract, p. 123). Δn against wavelength for free electrons
and holes (Figs. 8 and 9, p. 126) and against density at 1.3 and 1.55 µm (Figs. 10 and 11, p. 127),
fitted by lines of slope 0.805 (holes) and 1.05 (electrons) at 1.3 µm and 0.818 and 1.04 at
1.55 µm (p. 126); Δα measured against density (Figs. 12 to 15, pp. 127–128); depleting 10¹⁸
holes per cm³ raises the index by 1.5 × 10⁻³ at 1.3 µm and 2.1 × 10⁻³ at 1.55 µm (Section VI,
p. 128); at 10¹⁷ cm⁻³ holes move the index 3.3 times as much as electrons (p. 128). The Drude
forms, their Eqs. (4) and (5) with m*_ce = 0.26 m₀ and m*_ch = 0.39 m₀, give about 0.5 of the
measured Δα for holes and 0.25 for electrons (p. 127).

**What later papers fit to it.** The closed forms photonoxide's users expect are not printed in the
1987 paper. Reed et al. 2010 (p. 519) print, attributing them to Soref & Bennett, at 1.55 µm
Δn = −[8.8 × 10⁻²² ΔN_e + 8.5 × 10⁻¹⁸ (ΔN_h)^0.8] and
Δα = 8.5 × 10⁻¹⁸ ΔN_e + 6.0 × 10⁻¹⁸ ΔN_h (cm⁻³, cm⁻¹), and at 1.3 µm the same with 6.2 × 10⁻²²,
6.0 × 10⁻¹⁸, 6.0 × 10⁻¹⁸ and 4.0 × 10⁻¹⁸; Chrostowski & Hochberg print the same (Eqs. 6.1 and 6.2,
p. 217), a λ² extension of them (Eq. 6.5, p. 218), and Nedeljkovic et al. 2011's update at
1550 nm: Δn = −5.4 × 10⁻²² N^1.011 − 1.53 × 10⁻¹⁸ P^0.838 (Eq. 6.3, p. 217) and
Δα = 8.88 × 10⁻²¹ N^1.167 + 5.84 × 10⁻²⁰ P^1.109 (Eq. 6.4, p. 218), with 1310 nm coefficients
beside them. Nedeljkovic et al.'s own paper, with its range of 1 to 14 µm, is *(to read)*.

Checks of these on the 1987 paper *(ours)*: 8.5 × 10⁻¹⁸ × (10¹⁸)^0.8 = 2.14 × 10⁻³ against the
paper's 2.1 × 10⁻³; at 10¹⁷ cm⁻³ the expressions give holes 3.8 times electrons where the paper's
text says 3.3, about 15 % apart, so a case against the 1987 curves takes its tolerance from how
well the expressions fit them; Nedeljkovic's form gives 1.86 × 10⁻³
for 10¹⁸ holes, 13 % below the older one. That spread is the material's, and it ships with the
model, as the roadmap's decision 7 asks of variable material data.

### 2.3 Into the mode

The carrier map gives Δn(x, y) and Δα(x, y) over the silicon; Δε = 2n(Δn + iΔκ) with
Δκ = Δα λ/(4π); the mode's Δn_eff and loss follow by first-order perturbation, checked against a
re-solve, as for heat and the Pockels effect. Chrostowski & Hochberg's Eq. 6.12 (p. 221) is the
1D, effective-index version of the same overlap. Vπ·L = λ/(2 Δn_eff) for one arm at a bias, by the
roadmap's one definition (its decision 1); because Δn_eff isn't linear in V, a modulator reports
Vπ·L(V) and its small-signal value between two biases, as Ding et al. do (7.8 V between −1 and
0 V, p. 2244).

### 2.4 The line

Ding et al.'s distributed circuit (Fig. 1(c), p. 2241): the metal line's L_tl, R_tl and C_tl per
length, and across it the junction's C_pn in series with the silicon's R_pn. Its shunt admittance is
jωC_tl + jωC_pn/(1 + jωR_pnC_pn). The loss from R_pn grows as f², the metal's as √f (p. 2243).
`electro::line` already turns a line's Z and Y into γ, Z₀ and the electro-optic response; the
junction's branch is one more term in Y.

## 3. The phases

Each phase lists its work, its validation in the roadmap's three kinds (analytic, a published
result, convergence) and its cost. The numbers of cases are *estimates*.

### P1: Plasma dispersion and the abrupt junction

**Work.**

- A carrier material model in the catalogue: Δn and Δα of silicon from electron and hole
  densities at a wavelength, two models with their provenance and ranges: Soref & Bennett as
  Reed et al. and Chrostowski & Hochberg print it (1.31 and 1.55 µm, and the λ² form between),
  and Nedeljkovic et al. 2011 (1310 and 1550 nm as Chrostowski & Hochberg quote it; the paper's
  range once it is read). Each result can report both, and their difference is the stated spread.
- The abrupt junction in closed form (Section 2.1): a lateral p–n junction across a rib, its
  offset from the rib's centre, the doped regions' densities and sheet resistances, the bias;
  carriers across the rib, C_j(V), R_j, f_c.
- The carrier map into a mode (Section 2.3) through the existing full-vector and effective-index
  solvers.
- One carrier model for the whole library: the nonlinear pulse model's free carriers keep Lin et
  al.'s coefficients, which its published cases use, and say which model they are (decision 3).

**Validation.**

- *Analytic:* charge balance N_A x_p = N_D x_n to round-off; V_bi, W_d and C_j against their
  closed forms; a uniform Δn over the whole core moves n_eff by the core's share of the mode's
  energy; first-order perturbation against the re-solve.
- *Published:* the printed expressions against Soref & Bennett's own points (2.1 × 10⁻³ and
  1.5 × 10⁻³ at 10¹⁸ holes; the slopes of Figs. 10 and 11). Chrostowski & Hochberg's model (Section
  6.2, pp. 219–223; Listing 6.1, pp. 242–243): a 500 nm rib, 220 nm thick on a 90 nm slab, N_A =
  5 × 10¹⁷ and N_D = 3 × 10¹⁷ cm⁻³, sheet resistances 2.5 and 4.0 kΩ/sq (rib, n and p) and 6 and
  10 kΩ/sq (slab), a 50 nm offset; their Δn_eff and α_pn against reverse bias (Fig. 6.5, p. 222),
  Vπ·L = 1.6 V·cm (Fig. 6.6) and f_c = 35 and 51 GHz at 0 and 1 V (p. 223). Their mode is 1D by
  the effective index method: the case compares our effective-index path with theirs, and states
  the 2D mode's difference from it. The distances from the junction to the heavily doped regions
  and the other arguments of the run behind Figs. 6.5 to 6.7 are *(to read)* from Listings 6.2 and
  6.3.
- *Convergence:* the mode's Δn_eff in the mode solver's grid, the carrier map sampled on it.

**Cost:** small; no new solver. About 6 cases.

### P2: The depletion Mach–Zehnder modulator at analytic fidelity

**Work.**

- A device, a lateral p–n phase shifter in each arm of a balanced interferometer, driven in
  push–pull or differentially, its line a coplanar strip or ground–signal–ground line loaded by
  the junction (Section 2.4), at analytic fidelity (P1's junction and the line's closed forms, as
  the TFLN modulator's analytic fidelity) and measured fidelity (a measured device's file, as
  `device::active::Measurement` holds for the TFLN modulator). Its figures: Vπ·L(V) and the
  small-signal Vπ, the phase shifter's loss, C(V), R, f_c, the loaded line's index, Z₀ and loss,
  the electro-optic 3 dB and the RF 6.4 dB bandwidths, the insertion loss.
- `electro::line` gains the junction's shunt branch; segmented and slow-wave lines through
  `Periodic`'s cascade.

**Validation.**

- *Analytic:* with the line matched and the waves in step, the electro-optic response is
  (1 − e^(−αL))/(αL), which falls by 3 dB where the line's loss is 6.4 dB *(ours, from the closed
  form)*; Ding et al. use that rule on their measurements (p. 2244). The junction's branch at
  R_pn → 0 is a capacitance added to C_tl.
- *Published:* Ding et al. 2014 (Section 4 lists its numbers). The phase shifter and C_pn, R_pn
  are the cases; the slow-wave line is a stretch, since its segmented electrodes are 3D and Ding et
  al.'s line parameters came from a commercial 3D solver (their Fig. 2 values are quoted as
  published numbers). Patel et al. 2015 *(to read)*, which reuses Ding et al.'s doping.
- *Convergence:* the line's grid, as the TFLN modulator's.

**Cost:** small to medium; most of it is the TFLN modulator's structure again. About 5 cases.

### P3: Drift-diffusion, and the modulator at 2D fidelity

**Work.**

- A `carrier` module in the static layer (the roadmap's Architecture already names it), on the
  static solver's graded grid: Poisson and the two continuity equations, Scharfetter–Gummel
  fluxes, SRH recombination, Masetti and Caughey–Thomas mobility, ohmic contacts, scaled as
  Selberherr's book scales them; Gummel's iteration, then Newton, with continuation in bias.
- Doping as regions: uniform, analytic profiles (an implant's Gaussian with its range and
  straggle given), or a table. In 1D first (a junction along a line, all the checks below in
  milliseconds), then the cross-section.
- Small signal: C = dQ/dV and the admittance of the linearized system at a frequency, which gives
  R and C together.
- The modulator's 2D fidelity: the drift-diffusion carriers into the 2D mode, the line from the
  static solver.
- Deterministic as the rest of the library: the same bits on any thread count.

**Validation.**

- *Analytic:* far from the junction n = N_D and p = n_i²/N_D to round-off; the equilibrium
  junction against the depletion approximation, which differs by a known correction of order
  k_BT/q in the potential (the form in Sze & Ng 2006 is *(to read)*); a uniform doped bar's
  resistance; Scharfetter–Gummel exact for a constant field; a forward-biased long diode against
  Shockley's ideal-diode current with the same SRH lifetime; C from dQ/dV against the small-signal
  admittance at low frequency; second order in the grid.
- *Published:* Yu et al. 2012 *(to read)*: uniform doping and a simple geometry, the cleanest
  published drift-diffusion target the survey found: its abrupt-junction C(V) in closed form and
  the lateral junction's measured 0.44 pF/mm at 0 V (the survey's reading). Ding et al. 2014: C_pn
  ≈ 250 fF/mm at 0 V and R_pn = 0.7 Ω·cm for 5 × 10¹⁷ p and 3 × 10¹⁷ n with p+ and n+ at
  2 × 10¹⁸ cm⁻³ (p. 2242); their cross-section is drawn "not to scale" (Fig. 1(d)), so its doped
  regions' widths are read from the drawing's labels and stated as assumptions. Masetti et al.'s
  tables reproduce through the mobility model (in the folder, to read).
- *Cross-code:* DEVSIM on the same 1D and 2D junctions, run as an external program once its
  licence is checked (roadmap decision 6).
- *Convergence:* the grid near the junction, graded; the bias steps.

**Cost:** large, and the phase with the most risk (Section 5). About 10 cases.

### P4: The ring modulator

**Work.**

- A device: an all-pass or add-drop ring (`AllPassRing`, `AddDropRing`) whose doped arc's
  Δn_eff(V) and loss come from P1 or P3; the resonance's shift and Q against bias.
- Its response in time by coupled-mode theory, da/dt = (iω₀ − 1/τ)a − iμE_in with
  1/τ = 1/τ_l + 1/τ_e (Bao et al. 2024, Eqs. 1–3, p. 4926; Sacher & Poon 2008 *(to read)*),
  driven through the junction's electrical network (Bao et al.'s Fig. 2: C_j, R_s, C_si, R_si,
  C_pad); self-heating by a stated coefficient (their Eqs. 8–12) and the heater through the
  existing thermal model.

**Validation.**

- *Analytic:* coupled-mode theory's steady state against the ring's exact transfer function, its
  error measured against Q (the theory is an approximation near resonance); its step response's
  exponential; Sacher & Poon's small-signal responses for index, loss and coupling modulation
  *(to read)*.
- *Published:* Bao et al. 2024 (Section 4); its eye diagrams at 80 and 106 Gb/s come from a model
  fitted to measurements, so reproducing them from their parameters checks our coupled-mode model
  and time stepping, not the junction's physics. Chrostowski & Hochberg's ring-modulator listing
  (a 10 µm all-pass ring) as a second closed-form model *(to read)*.

**Cost:** medium. About 5 cases.

### P5: Circuits in time, and signals

**Work.**

- Each component's S(ω) as a pole–residue model (`compact::fit` exists), stepped in time exactly
  for a piecewise-linear input, in complex baseband about a carrier (Ye et al. 2018, in the
  folder); components joined node to node with their delays (Fiers et al. 2012 *(to read)*), in
  both directions (Ye et al. 2022 *(to read)*). A model that isn't passive is refused with the
  fit's advice (decision 6).
- Active blocks: the Mach–Zehnder modulator's electro-optic response (P2, P3) fitted the same way,
  the ring by coupled-mode theory (P4), heaters by `thermal::RcModel` as `circuit::Heated` drives
  them now.
- Sources: PRBS from linear-feedback shift registers, 2⁷ − 1 to 2³¹ − 1 long, each with its
  polynomial stated; NRZ and PAM-n with a stated rise time. A driver as a swing and a low-pass;
  a receiver as a responsivity, a low-pass and Gaussian noise.
- Outputs: time traces; the eye diagram (the trace folded on the symbol period), extinction ratio,
  optical modulation amplitude, eye height and width; the Q factor and BER = ½ erfc(Q/√2) as the
  Gaussian-noise definitions they are; a link budget as the laser's power times the path's
  transmissions from the S-matrices, with closed-form penalties (a finite extinction ratio's).
- Differentiable from the start (the roadmap's decision 5): gradients through the stepping,
  checked against finite differences.

**Validation.**

- *Analytic:* a single pole's impulse and step responses to round-off; the stepped response's
  spectrum against the model's S(ω), at the order the stepping claims; a passive model's energy
  never grows; a delay line's exact delay; a PRBS's period 2ⁿ − 1 and its run statistics; an
  RC-limited NRZ eye's height against its closed form; the ring's step response (P4).
- *Published:* Ye et al. 2022's three examples *(to read)*; Ye et al. 2018's fits (in the
  folder). Ding et al.'s measured eyes (Fig. 9, p. 2245: 12.4 dB extinction at 28 Gb/s, 10 dB at
  40 Gb/s) depend on drivers, an amplifier and a receiver the paper doesn't describe enough to
  model, so they illustrate a lesson and aren't a case.

**Cost:** large: a new simulator, if a small one. About 8 cases.

### P6: Programmable meshes

**Work.**

- `circuit::mesh`: Reck, Clements and hexagonal meshes as netlists (the hexagonal builder of
  `optimize_mesh_selfconfig` promoted); the decompositions from 0.6.3's Q1, or built here if Q1
  hasn't landed; mesh cells as devices (an MZI of two thermo-optic shifters and couplers, with
  their loss and splitting from the existing devices, heaters' crosstalk from `thermal`).
- Self-configuration: Miller 2013's progressive method, and Hamerly et al. 2022's for rectangular
  meshes; error correction by Bandyopadhyay et al. 2021; calibration and crosstalk correction posed
  for genoxide, as `thermal::Correction` and the examples pose them now.

**Validation.**

- *Analytic:* Reck and Clements decompositions of Haar-random unitaries reconstructed to round-off;
  N(N − 1)/2 cells in both, N columns for Clements; a lossless mesh's unitarity.
- *Published:* Clements et al. 2016's comparison of loss robustness against Reck's (in the folder,
  to read); Hamerly et al. 2022's and Bandyopadhyay et al. 2021's error scaling *(to read)*;
  Pérez-López et al. 2020's all-cross state (the example's checks, already run); Bandyopadhyay et
  al. 2024's chip as a mesh to reproduce *(to read)*.

**Cost:** small to medium; independent of P1 to P5. About 6 cases.

### G: Germanium photodetectors

The roadmap has no germanium: no material, no device, no milestone. #774 proposes germanium in the
catalogue, drift-diffusion in 0.8 and a device in 0.13. What a waveguide germanium detector needs,
in the order it could be built:

1. **Optical data.** Germanium's complex index at 1.31 and 1.55 µm, where it absorbs. The folder's
   Li 1980 covers germanium only from 1.9 to 16 µm (its abstract), so it doesn't reach the
   telecom bands; candidates: Nunley et al. 2016's ellipsometry from 0.5 to 6.6 eV, Dash & Newman
   1955's absorption edge, and Ishikawa et al. 2003's band-gap shrinkage of tensile-strained
   germanium grown on silicon, which moves the edge across 1.55 µm *(all to read)*. Thermal
   conductivity is in the folder (Glassbrenner & Slack 1964).
2. **Absorption and responsivity.** The mode solver with germanium's k gives the absorbed fraction
   along a butt- or evanescently coupled absorber; responsivity is ηqλ/(hc). No new solver: this
   is P1's level of cost.
3. **Bandwidth.** The carriers' transit across the intrinsic region and the RC of the contacts:
   P3's drift-diffusion with germanium's mobility and lifetime, and, for a germanium-on-silicon
   junction, band offsets at a heterojunction, which homojunction drift-diffusion doesn't have.
4. **Dark current.** Generation in the depletion region from an SRH lifetime is computable; the
   measured dark current is mostly defects and surfaces, which a lifetime can only be fitted to.
   A detector would state that rather than predict it.

Papers it would be checked against *(all to read)*: Wang et al. 2026 (a silicon nitride
waveguide-integrated germanium detector, above 110 GHz at 1 mA in the O band, its title says),
Lischke et al. 2015 (a waveguide-coupled germanium p–i–n photodiode) and Michel et al. 2010's
review. The options and what I decided are decision 8.

## 4. Validation targets by paper

"In the folder" means the PDF is in the papers folder. Numbers marked *(survey)* are the literature
survey's of 2026-10-02, read from the full text then, to be confirmed from the PDF.

| Paper | In the folder | Phase | What it gives |
|---|---|---|---|
| Soref & Bennett 1987 | yes, read | P1 | Δn at 10¹⁸ holes: 1.5 × 10⁻³ (1.3 µm), 2.1 × 10⁻³ (1.55 µm); slopes 0.805 and 1.05 (1.3 µm), 0.818 and 1.04 (1.55 µm); holes 3.3 × electrons at 10¹⁷; the Drude Δα 0.5 (holes) and 0.25 (electrons) of measured |
| Reed et al. 2010 | yes, read for this | P1 | the 1.55 and 1.3 µm expressions (p. 519); 5 × 10¹⁷ of each gives −1.66 × 10⁻³ at 1.55 µm |
| Chrostowski & Hochberg 2015, ch. 6 | yes, Sections 6.1–6.2.4 read | P1, P3, P4 | the expressions and Nedeljkovic's update (Eqs. 6.1–6.5); the abrupt junction (Eqs. 6.6–6.15); 1.6 V·cm at a 50 nm offset (Fig. 6.6); f_c 35 and 51 GHz at 0 and 1 V; a ring-modulator model *(to read)* |
| Ding et al. 2014 | yes, read | P2, P3 | 220 nm rib on a 90 nm slab; p 5 × 10¹⁷, n 3 × 10¹⁷, p+ and n+ 2 × 10¹⁸ cm⁻³; C_pn ≈ 250 fF/mm at 0 V, R_pn 0.7 Ω·cm; 3.5 mm device, 3 mm loaded, 11 sections and 10 optical delay loops; slow-wave line g 30, s 10, d 6, D 100, w 5, p 14 µm; loaded RF index 6.5 simulated, 6.1 measured near 25 GHz at 0 V; optical group index about 3.8; small-signal Vπ 7.8 V (−1 to 0 V), π at 5.5 V per arm; phase shifter's loss 3.6 dB over 3 mm; EO 3 dB 21.5 GHz (0 V) and 27 GHz (−1 V), RF 6.4 dB 21 and 26 GHz; simulated 24 GHz, the difference put down to 5–10 kΩ·µm² contacts (0.1–0.2 Ω·cm more R_pn) |
| Yu et al. 2012 | no | P1, P3 | lateral and interdigitated junctions, uniform doping; C(0 V) 0.44 pF/mm lateral, 1.59 pF/mm interdigitated; lateral at 2 × 10¹⁸ with a 40 nm gap 1.19 V·cm (−1 V) and 1.23 V·cm (−6 V); an analytic C(V) *(survey)* |
| Patel et al. 2015 | no | P2, P3 | 4 mm series push–pull, T-rail line, Ding et al.'s doping; Vπ·L 3.2 V·cm at 4 V; EO 3 dB 38 GHz (2 V), 41 GHz (4 V); group index 3.89; C_pn 230 pF/m at 0 V; microwave index 3.7 at 0 V and 30 GHz *(survey)* |
| Han et al. 2023 | no | P2 (stretch) | a corrugated slow-light silicon modulator with 110 GHz bandwidth (its title); its length and Vπ *(to read)* |
| Nedeljkovic et al. 2011 | no | P1 | the coefficients from 1 to 14 µm and their fit ranges *(to read)*; the 1550 and 1310 nm ones as Chrostowski & Hochberg quote them |
| Sze & Ng 2006 | no | P1, P3 | the abrupt junction's closed forms with their k_BT/q correction *(to read)* |
| Scharfetter & Gummel 1969; Masetti et al. 1983; Caughey & Thomas 1967; Selberherr 1984; Gummel 1964; Shockley & Read 1952; Hall 1952 | the first three | P3 | the algorithms and models; Masetti's tables *(to read)* |
| Bao et al. 2024 | yes, Sections I–III read | P4, P5 | rings of 8 µm (FSR 8 nm, Q ≈ 5000) and 11 µm (FSR 6.5 nm, Q ≈ 6000); τ_l(V) = −5.25 × 10⁻² V² + 0.56 V + 21.32 ps (Eq. 16) and Eq. 17; self-heating 0.519 per mW; heater 0.12 nm/mW; eyes at 80 and 106 Gb/s; Tables I and II *(to read: images in the folder's copy)* |
| Sacher & Poon 2008 | no | P4 | the ring's small-signal responses to index, loss and coupling modulation *(to read)* |
| Ye et al. 2018; Ye et al. 2022; Fiers et al. 2012 | 2018 yes | P5 | complex-baseband fitting; the three time-domain examples; node-based stepping *(2022 and Fiers to read)* |
| Reck et al. 1994; Clements et al. 2016; Miller 2013; Hamerly et al. 2022; Bandyopadhyay et al. 2021; Pérez et al. 2017; Bandyopadhyay et al. 2024 | Clements yes | P6 | decompositions; loss robustness; self-configuration; error scaling; hexagonal meshes; a mesh chip *(to read)* |
| Wang et al. 2026; Lischke et al. 2015; Michel et al. 2010; Nunley et al. 2016; Ishikawa et al. 2003; Dash & Newman 1955 | no | G | germanium detectors and data *(to read)* |

## 5. Order, dependencies, cost and risk

**Order.** P1, then P2 (the modulator exists at analytic fidelity, and the Academy's lesson has
its charts), then P3 in 1D and then 2D, the modulator's 2D fidelity with it, then P4 and P5. P6
depends only on 0.6.3's Q1 and can be done whenever an agent is free; G's first two items come with 0.8 and its detector device with 0.13, as decision 8 says.
Phases are commits on one branch and one pull request at the end, as every milestone's work is.

**Dependencies on earlier milestones.**

- 0.6's static solver, lines, devices and thermal models: done. 0.4's circuits and compact
  models: done.
- 0.6.3's Q1 for the Reck and Clements decompositions (P6): not done.
- #432's periodic sections (the eme-bends plan) or 3D FDFD for Han et al. 2023's corrugated guide.
- 0.7 for objectives built on these (an eye's opening, a modulator's Vπ·L against loss); 0.11 for
  doping and geometry corners and Monte Carlo; 0.9 for the meshes' layout.

**Cases** *(estimate)*: P1 about 6, P2 about 5, P3 about 10, P4 about 5, P5 about 8, P6 about 6:
about 40.

**Risks.**

- **Drift-diffusion's convergence.** Exponential carrier densities over twenty orders of
  magnitude, a stiff coupled system. Mitigated by scaling, Scharfetter–Gummel fluxes with a stable
  Bernoulli function, Gummel before Newton, continuation in bias, and 1D before 2D.
- **Doping that papers don't state.** Published modulators are implanted and their profiles
  simulated by process tools; Ding et al. draw theirs "not to scale", Patel et al. reuse it. Yu et
  al.'s uniform doping is the clean target; for the others the assumed profile is written into
  the case.
- **3D devices in a 2D cross-section.** Interdigitated junctions, slow-wave electrodes and
  corrugated slow-light guides vary along the guide. As with Harris et al.'s periodic heater in
  0.6, a 2D model either bounds them by two sections or leaves them out, and says which.
- **Numbers from commercial solvers in papers** (Ding et al.'s line parameters) are compared as
  published numbers; they aren't references we can rerun.
- **The plasma-dispersion models' spread** (13 % between the two at 10¹⁸ holes) is larger than
  most solver errors; a case's tolerance includes it by the rule, and a disagreement inside it
  isn't a result.
- **Fitted models in time.** A vector-fitted model that isn't passive can gain energy when stepped;
  it is refused, not stepped.

## 6. API sketch (a proposal)

Names only, to be settled when each phase starts.

- `material::catalogue`: a plasma-dispersion model, `PlasmaDispersion` with
  `SorefBennett` and `Nedeljkovic`, each `delta_n(electrons, holes, wavelength)` and `delta_alpha`,
  with their provenance and ranges.
- `carrier` (static layer): `Doping` (regions, profiles, tables), `AbruptJunction` (the closed
  form), `DriftDiffusion` (the problem on a `statics::Grid`), `Solution` (ψ, n, p, currents),
  `SmallSignal` (C, R, admittance at a frequency).
- `mode::carrier`: a carrier map into Δn_eff and loss, by perturbation and re-solve, beside
  `mode::thermo` and `mode::pockels`.
- `electro::line`: the junction's shunt branch on `QuasiTem`.
- `device`: `DepletionMzm`, `DepletionShifter`, `RingModulator`, each at its fidelities.
- `circuit::time`: `Baseband`, `Stepper`, `Prbs`, `Pam`, `Eye`, `LinkBudget`.
- `circuit::mesh`: `Reck`, `Clements`, `Hexagonal`, `self_configure`, `correct`.
- Jobs: a `"carrier"` job (a junction's carriers, C(V), the mode's Δn_eff) and a `"signal"` job
  (a circuit in time, its eye).

## 7. Studio and Academy

**Studio** (the roadmap's 0.8 lines): a junction view (doping, potential, carriers, the depletion
edges over the mode); C(V), Vπ·L(V) and loss swept over doping, offset and bias; time traces and
eye diagrams; mesh programming (a target unitary, its phases, a self-configuration's steps). Each
runs as a job from the program and the window, its record replayable.

**Academy.** The lessons this milestone enables, from #774's Phase B:

| Lesson (#774) | Enabled by | What is computed |
|---|---|---|
| Carrier-depletion modulators (B8) | P1 for its first sections; P2 to P4 for the rest | Δn and Δα against density by both models; the junction's width, C(V) and f_c; Δn_eff and Vπ·L against bias and offset; a modulator's bandwidth against its length and line; a ring modulator's resonance against bias |
| Programmable photonics (B6) | written now from the examples; P6 adds its decompositions, self-configuration and error-correction sections | a mesh's fidelity against splitter error and loss; Reck against Clements |
| From components to circuits (B2) | written now; P5 adds a section on circuits in time | a component's poles and its response in time |
| Optical links and co-packaged optics (B10) | an outline until P5; its receiver needs G | a link's budget and eye from a modulator, a channel and a receiver |
| Germanium photodetectors (B10) | an outline; decided 2026-11-10 (decision 8) | absorption, responsivity and bandwidth against length and doping |

When P1 lands, the modulators lesson's Section 9 loses its "photonoxide does not compute carrier
modulators yet" and links the new lesson.

## 8. Out of scope

- Carrier injection (p–i–n) phase shifters and attenuators as targets: drift-diffusion computes
  them in forward bias, but 0.8 validates depletion only.
- Large-signal transients of drift-diffusion (the roadmap's decision 4): signals use the
  behavioural models of P4 and P5.
- Electro-absorption (Franz–Keldysh, quantum-confined Stark) modulators, plasmonic and
  transparent-conductor modulators.
- Lasers and III–V integration (#774's other decision: decided 2026-11-10, a theory lesson only, no milestone).
- Drift-diffusion in 3D, Fermi–Dirac statistics and degenerate contacts, impact ionization,
  tunnelling, process simulation of implants.
- Circuit simulation of electronics: drivers and receivers are a few closed-form blocks
  (photonoxide isn't a SPICE), and no DSP, equalization or forward error correction.

## 9. Decisions

I decided on 2026-11-10 to take the recommended option of each of the nine; each says so below, and its text is kept. Lasers (heterogeneous III–V) get no milestone: a theory lesson only (#774).

1. **Where the carrier physics lives.** *Decided 2026-11-10: the recommended option.* (a) A `carrier` module in the static layer, the Δn map
   handed to the mode solver like a temperature field; (b) inside `electro`. *Recommended: (a)*,
   as the roadmap's Architecture lists it, in the one crate.
2. **The default plasma-dispersion model.** *Decided 2026-11-10: the recommended option.* (a) Nedeljkovic et al. 2011 once its paper is read,
   Soref & Bennett until then; (b) Soref & Bennett always. Either way both are available and their
   difference is stated. *Recommended: (a).*
3. **The nonlinear module's free carriers.** *Decided 2026-11-10: the recommended option.* (a) They keep Lin et al. 2007's coefficients, which
   their published cases use, and say so beside the new model; (b) they move to the new model.
   *Recommended: (a).*
4. **The order of P2 and P3.** *Decided 2026-11-10: the recommended option.* (a) The modulator at analytic fidelity before drift-diffusion; (b)
   drift-diffusion first. *Recommended: (a)*: it is cheaper, has published targets at once and
   gives the lesson its charts.
5. **The first drift-diffusion target.** *Decided 2026-11-10: the recommended option.* (a) Yu et al. 2012's uniform junction; (b) Ding et al.
   2014's, with its profile assumed from the drawing. *Recommended: (a) first, (b) as the second*,
   once Yu et al. is in the folder.
6. **Passivity of models stepped in time.** *Decided 2026-11-10: the recommended option.* (a) Refuse a model that isn't passive; (b) enforce
   passivity by perturbing the residues. *Recommended: (a) now, (b) only if real devices need it.*
7. **Han et al. 2023's slow-light modulator.** *Decided 2026-11-10: the recommended option.* (a) A stretch case after P2, needing a periodic
   guide's Bloch mode; (b) not in 0.8. *Recommended: (a), not a gate for the release.*
8. **Germanium photodetectors.** *Decided 2026-11-10: the recommended option.* (a) As #774 proposes: germanium's optical data in the catalogue
   and a detector's absorption and responsivity with 0.8 (items 1 and 2 of G, at P1's cost), the
   detector's bandwidth and dark current as a device in 0.13 once P3 is validated; (b) a milestone
   of its own for detectors, heterojunction drift-diffusion included; (c) an Academy lesson only,
   no code.
   *Recommended: (a).* It needs the six germanium papers in Section 4.
9. **Link budgets.** *Decided 2026-11-10: the recommended option.* (a) Products of validated transmissions with closed-form penalties and
   Gaussian-noise definitions only; (b) receiver sensitivity models. *Recommended: (a).*

## References

Every DOI below was checked on Crossref on 2026-11-09. "Folder" marks a PDF in the papers folder;
the others are listed in the folder's README as needed (journal versions), under the 0.8 section,
or already listed there by #774.

**In the folder**

- R. A. Soref, B. R. Bennett, IEEE J. Quantum Electron. 23, 123 (1987). [10.1109/JQE.1987.1073206](https://doi.org/10.1109/JQE.1987.1073206) (folder, `soref-bennett-1987.pdf`)
- G. T. Reed, G. Mashanovich, F. Y. Gardes, D. J. Thomson, Nat. Photonics 4, 518 (2010). [10.1038/nphoton.2010.179](https://doi.org/10.1038/nphoton.2010.179) (folder, `reed-2010.pdf`)
- L. Chrostowski, M. Hochberg, *Silicon Photonics Design: From Devices to Systems*, Cambridge University Press (2015). [10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168) (folder, `chrostowski-hochberg-2015.pdf`)
- R. Ding, Y. Liu, Y. Ma, Y. Yang, Q. Li, A. E.-J. Lim, G.-Q. Lo, K. Bergman, T. Baehr-Jones, M. Hochberg, J. Lightwave Technol. 32, 2240 (2014). [10.1109/JLT.2014.2323954](https://doi.org/10.1109/JLT.2014.2323954) (folder, `ding-2014.pdf`)
- S. Bao et al., J. Lightwave Technol. 42, 4924 (2024). [10.1109/JLT.2024.3382091](https://doi.org/10.1109/JLT.2024.3382091) (folder, `bao-2024.pdf`)
- D. L. Scharfetter, H. K. Gummel, IEEE Trans. Electron Devices 16, 64 (1969). [10.1109/T-ED.1969.16566](https://doi.org/10.1109/T-ED.1969.16566) (folder)
- G. Masetti, M. Severi, S. Solmi, IEEE Trans. Electron Devices 30, 764 (1983). [10.1109/T-ED.1983.21207](https://doi.org/10.1109/T-ED.1983.21207) (folder)
- D. M. Caughey, R. E. Thomas, Proc. IEEE 55, 2192 (1967). [10.1109/PROC.1967.6123](https://doi.org/10.1109/PROC.1967.6123) (folder)
- B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999). [10.1109/61.772353](https://doi.org/10.1109/61.772353) (folder)
- Y. Ye, D. Spina, Y. Xing, W. Bogaerts, T. Dhaene, Photonics Res. 6, 560 (2018). [10.1364/PRJ.6.000560](https://doi.org/10.1364/PRJ.6.000560) (folder)
- W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walmsley, Optica 3, 1460 (2016). [10.1364/OPTICA.3.001460](https://doi.org/10.1364/OPTICA.3.001460) (folder)
- S. Pai, B. Bartlett, O. Solgaard, D. A. B. Miller, Phys. Rev. Applied 11, 064044 (2019). [10.1103/PhysRevApplied.11.064044](https://doi.org/10.1103/PhysRevApplied.11.064044) (folder)
- D. Pérez-López, A. López, P. DasMahapatra, J. Capmany, Nat. Commun. 11, 6359 (2020). [10.1038/s41467-020-19608-w](https://doi.org/10.1038/s41467-020-19608-w) (folder)
- Q. Lin, O. J. Painter, G. P. Agrawal, Opt. Express 15, 16604 (2007). [10.1364/OE.15.016604](https://doi.org/10.1364/OE.15.016604)
- H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980). [10.1063/1.555624](https://doi.org/10.1063/1.555624) (folder)
- C. J. Glassbrenner, G. A. Slack, Phys. Rev. 134, A1058 (1964). [10.1103/PhysRev.134.A1058](https://doi.org/10.1103/PhysRev.134.A1058) (folder)

**Needed: carrier modulators and drift-diffusion**

- H. Yu et al., Opt. Express 20, 12926 (2012). [10.1364/OE.20.012926](https://doi.org/10.1364/OE.20.012926) (`yu-2012.pdf`)
- D. Patel et al., Opt. Express 23, 14263 (2015). [10.1364/OE.23.014263](https://doi.org/10.1364/OE.23.014263) (`patel-2015.pdf`)
- C. Han et al., Sci. Adv. 9, eadi5339 (2023). [10.1126/sciadv.adi5339](https://doi.org/10.1126/sciadv.adi5339) (`han-2023.pdf`, listed by #774)
- M. Nedeljkovic, R. Soref, G. Z. Mashanovich, IEEE Photonics J. 3, 1171 (2011). [10.1109/JPHOT.2011.2171930](https://doi.org/10.1109/JPHOT.2011.2171930) (`nedeljkovic-2011.pdf`)
- S. M. Sze, K. K. Ng, *Physics of Semiconductor Devices*, 3rd ed., Wiley (2006). [10.1002/0470068329](https://doi.org/10.1002/0470068329) (`sze-ng-2006.pdf`)
- S. Selberherr, *Analysis and Simulation of Semiconductor Devices*, Springer (1984). [10.1007/978-3-7091-8752-4](https://doi.org/10.1007/978-3-7091-8752-4) (`selberherr-1984.pdf`)
- H. K. Gummel, IEEE Trans. Electron Devices 11, 455 (1964). [10.1109/T-ED.1964.15364](https://doi.org/10.1109/T-ED.1964.15364) (`gummel-1964.pdf`)
- W. Shockley, W. T. Read, Phys. Rev. 87, 835 (1952). [10.1103/PhysRev.87.835](https://doi.org/10.1103/PhysRev.87.835) (`shockley-read-1952.pdf`)
- R. N. Hall, Phys. Rev. 87, 387 (1952). [10.1103/PhysRev.87.387](https://doi.org/10.1103/PhysRev.87.387) (`hall-1952.pdf`)
- J. Witzens, Proc. IEEE 106, 2158 (2018). [10.1109/JPROC.2018.2877636](https://doi.org/10.1109/JPROC.2018.2877636) (`witzens-2018.pdf`; the modulator-design tutorial, for the lesson)

**Needed: rings, circuits in time and meshes**

- W. D. Sacher, J. K. S. Poon, Opt. Express 16, 15741 (2008). [10.1364/OE.16.015741](https://doi.org/10.1364/OE.16.015741) (`sacher-poon-2008.pdf`)
- Y. Ye, T. Ullrick, W. Bogaerts, T. Dhaene, D. Spina, J. Lightwave Technol. 40, 7856 (2022). [10.1109/JLT.2022.3206818](https://doi.org/10.1109/JLT.2022.3206818) (`ye-2022.pdf`)
- M. Fiers et al., J. Opt. Soc. Am. B 29, 896 (2012). [10.1364/JOSAB.29.000896](https://doi.org/10.1364/JOSAB.29.000896) (`fiers-2012.pdf`)
- M. Reck, A. Zeilinger, H. J. Bernstein, P. Bertani, Phys. Rev. Lett. 73, 58 (1994). [10.1103/PhysRevLett.73.58](https://doi.org/10.1103/PhysRevLett.73.58) (`reck-1994.pdf`)
- D. A. B. Miller, Photonics Res. 1, 1 (2013). [10.1364/PRJ.1.000001](https://doi.org/10.1364/PRJ.1.000001) (`miller-2013.pdf`)
- R. Hamerly, S. Bandyopadhyay, D. Englund, Phys. Rev. Applied 18, 024019 (2022). [10.1103/PhysRevApplied.18.024019](https://doi.org/10.1103/PhysRevApplied.18.024019) (`hamerly-2022.pdf`)
- S. Bandyopadhyay, R. Hamerly, D. Englund, Optica 8, 1247 (2021). [10.1364/OPTICA.424052](https://doi.org/10.1364/OPTICA.424052) (`bandyopadhyay-2021.pdf`)
- D. Pérez et al., Nat. Commun. 8, 636 (2017). [10.1038/s41467-017-00714-1](https://doi.org/10.1038/s41467-017-00714-1) (`perez-2017.pdf`)
- S. Bandyopadhyay et al., Nat. Photonics 18, 1335 (2024). [10.1038/s41566-024-01567-z](https://doi.org/10.1038/s41566-024-01567-z) (`bandyopadhyay-2024.pdf`, listed by #774)

**Needed: germanium (decision 8)**

- X. Wang et al., ACS Photonics 13, 4449 (2026). [10.1021/acsphotonics.5c02939](https://doi.org/10.1021/acsphotonics.5c02939) (`wang-2026.pdf`, listed by #774)
- S. Lischke et al., Opt. Express 23, 27213 (2015). [10.1364/OE.23.027213](https://doi.org/10.1364/OE.23.027213) (`lischke-2015.pdf`)
- J. Michel, J. Liu, L. C. Kimerling, Nat. Photonics 4, 527 (2010). [10.1038/nphoton.2010.157](https://doi.org/10.1038/nphoton.2010.157) (`michel-2010.pdf`)
- T. N. Nunley et al., J. Vac. Sci. Technol. B 34, 061205 (2016). [10.1116/1.4963075](https://doi.org/10.1116/1.4963075) (`nunley-2016.pdf`)
- Y. Ishikawa et al., Appl. Phys. Lett. 82, 2044 (2003). [10.1063/1.1564868](https://doi.org/10.1063/1.1564868) (`ishikawa-2003.pdf`)
- W. C. Dash, R. Newman, Phys. Rev. 99, 1151 (1955). [10.1103/PhysRev.99.1151](https://doi.org/10.1103/PhysRev.99.1151) (`dash-newman-1955.pdf`)

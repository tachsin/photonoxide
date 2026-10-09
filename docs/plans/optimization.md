# Optimization in photonic design: a survey and a plan with genoxide

*A survey and a plan, 2026-10-08. The owner reviews it first; the work is tracked in #244.*

AGENTS.md settles who does what. Optimizers come from [genoxide](https://github.com/tachsin/genoxide).
photonoxide supplies objectives, gradients and parametrizations. A method genoxide lacks is added
to genoxide as a general method, never written here and never made photonics-specific. This page
asks what that means in practice:

1. [The methods](#1-the-methods-and-where-each-pays): which kinds of method the photonics
   literature uses, and where each one pays, with the papers that show it.
2. [Published results to reproduce](#2-published-results-to-reproduce): results from 2013 to
   2026 with numbers. For each, the device, the method, the objective, the published figure of
   merit and the model or grid it was computed on.
3. [genoxide's coverage](#3-genoxides-coverage): what genoxide has today, and the general methods
   these results would need from it.
4. [The two sides](#4-what-photonoxide-supplies-and-what-genoxide-supplies): what photonoxide
   supplies and what genoxide supplies, and how the studio shows a run.
5. [The examples and their phases](#5-the-examples-and-their-phases).
6. [Decisions for the owner](#6-decisions-for-the-owner).
7. [Open-source tools in the field](#7-open-source-tools-in-the-field), by what they do and their
   licence.

**Rules this page follows.**

- Every paper is cited with its DOI, checked on Crossref on 2026-10-08. The papers the plan and
  its examples use are listed in the papers folder's README, in its section
  "Optimization methods (survey)".
- A number is quoted only with its source: the figure, table or section, and the model or grid it
  came from. Numbers that a paper shows only in a plot are marked *(from a plot)*.
- Section 1 compares methods by what papers measured. Where no paper measured something, the
  page says so instead of guessing.
- Other tools are described by what they do and by their licence, and nothing else.
- Nothing here promises a device's measured performance. A reproduction checks photonoxide's
  numbers against a paper's simulated ones, on a stated grid. Measured numbers are quoted as
  context only.

## Summary

- **Many variables and a cheap gradient: adjoint gradients with MMA, L-BFGS-B or Adam.** This is
  topology optimization and large shape problems, from hundreds to millions of variables. The
  adjoint gives the whole gradient for one extra solve. Sigmund 2011 states the gap: gradient
  methods solve problems with "thousands and up to millions of design variables using a few
  hundred" function evaluations. Non-gradient methods need "orders of magnitude more function
  evaluations for extremely low resolution examples". Every result of Chen et al. 2024's
  cross-validated suite uses a gradient method, mostly CCSA/MMA. genoxide has MMA, GCMMA, L-BFGS-B,
  Adam and continuation, all since 0.12.
- **Few parameters, cheap evaluations, no gradient or a rugged landscape: evolutionary
  methods.** The published examples:
  - multilayer stacks by differential evolution (Barry 2020; Bennet 2024);
  - a Y-branch's 13 widths by particle swarm (Zhang 2013);
  - photonic-crystal cavities' hole shifts by a genetic algorithm (Minkov & Savona 2014);
  - pixel designs by direct binary search (Shen 2015) or binary particle swarm (Mak 2016);
  - mesh calibration with a black-box objective, by a genetic algorithm and a particle swarm
    (Pérez-López 2020).

  genoxide has GA, DE, CMA-ES, PSO and local search.
- **Few parameters and very costly evaluations: Bayesian optimization.** Schneider 2019 compared
  BO, PSO, DE, multi-start Nelder-Mead and multi-start L-BFGS-B on 3D finite-element problems with
  6 to 9 parameters, and BO needed the fewest solves. In their supplement, L-BFGS-B overtook it as
  the parameters grew to 21 and 51. Elsawy 2019 report BO reaching CMA-ES's metasurface
  efficiency, and call it about four times faster. genoxide has BO since 0.13.
- **Trade-offs: multi-objective methods.** Passoni 2017 trace grating couplers' Pareto fronts of
  efficiency against bandwidth. genoxide has NSGA-II, NSGA-III, SPEA2, MOEA/D and SMS-EMOA.
- **Robust and fabrication-aware design is a formulation, not an optimizer.** Eroded, nominal and
  dilated designs, corners, lithography models and length-scale constraints are objectives and
  constraints that photonoxide poses. MMA's epigraph form handles min-max over them (Hammond
  2021; Christiansen & Sigmund 2021; Probst 2024).
- **Machine-learning surrogates** are fast but approximate, and the reviews say so (Jiang, Chen &
  Fan 2020; Wiecha 2021). They are not planned as optimizers here.
- **genoxide's gaps** for these examples:
  - a systematic neighbourhood scan for binary and integer genomes: direct binary search, in
    general form;
  - optional extras: quasi-oppositional initialization for DE, binary PSO and multi-objective PSO.

  The rest of what the plan needs is already planned in genoxide: the augmented Lagrangian,
  Levenberg-Marquardt, multi-fidelity and surrogate-assisted methods, EHVI and ParEGO, and the
  mixed genome. See [Section 3](#3-genoxides-coverage).
- **Fifteen examples are proposed** ([Section 5](#5-the-examples-and-their-phases)).
  - Five run on what photonoxide has today: transfer matrices and the circuit solve with its
    adjoint.
  - Two more need only a mesh builder.
  - The device examples wait for the geometry builder (#227) and 0.7's inverse-design pieces.
  - The cavity examples wait for 0.14.

## 1. The methods, and where each pays

### 1.1 Gradient methods with adjoint gradients

**What they need:** a gradient. The adjoint variable method gives the gradient with respect to
every design variable for the cost of one extra solve with the transposed system (Veronis 2004;
Lalau-Keraly 2013; photonoxide's FDFD and circuit adjoints, checked against finite differences).

**Where they pay.** Topology optimization has one variable per design pixel, 10⁴ to 10⁶ of them,
so nothing else is practical:

- Sigmund 2011, abstract: gradient methods "may efficiently solve fine-resolution problems with
  thousands and up to millions of design variables using a few hundred (finite element) function
  evaluations". Non-gradient approaches "require orders of magnitude more function evaluations
  for extremely low resolution examples".
- Molesky 2018, the review of inverse design in nanophotonics: the space of designs is "on the
  order of the set of nodes", so convergence needs gradient iterations. The adjoint makes them
  affordable.

**Which optimizer:**

- **MMA and its globally convergent form, CCSA or GCMMA** (Svanberg 1987, 2002): the standard
  where there are constraints. Chen et al. 2024 used "the CCSA/MMA optimization algorithm" except
  where noted. Hammond 2021 used GCMMA with an epigraph variable for min-max over wavelengths.
- **Adam:** used where the parametrization is a generator with a straight-through estimator, as
  in Schubert 2022 (learning rate 0.01, β₁ 0.667, β₂ 0.9).
- **L-BFGS:** used for smooth parametric problems, as in Minkov 2020's 70 hole positions. That
  run reached Q = 9.7×10⁶ by guided-mode expansion and 2.4×10⁶ in an FDTD check, from 6 100.
- **Steepest descent on level sets:** used for shape optimization (Lalau-Keraly 2013; Piggott
  2017).

**Continuation.** The projection's sharpness β is raised in stages:

- Chen 2024's mode converter: six epochs, β from 8 to 256;
- Hammond 2021: 70 iterations at each of β = 8, 16, 32.

ROADMAP's pitfall table records that resetting Adam's state between stages destroyed designs.
genoxide's `Continuation` keeps the state, and that is tested.

**Where they don't pay:** discrete or non-differentiable choices, such as a number of rings, a
layer's material or a measured objective. Also landscapes with many poor local optima when the
variables are few. There, Sections 1.2 and 1.3 apply, or a global method followed by a gradient
polish (Section 1.6).

### 1.2 Genetic algorithms and other evolutionary methods

**What they need:** only values. They tolerate noise, discontinuities and discrete variables,
and they explore several basins at once. Their cost is evaluations: they need many, and the count
grows with the number of variables.

**Where the photonics literature used them, with numbers:**

| Work | Method | Variables | Evaluations | Model | Result |
|---|---|---|---|---|---|
| Zhang 2013, Y-branch | PSO, swarm of 30 | 13 taper widths | about 1 500 (50 iterations) | 2D FDTD in the loop, then 3D FDTD at λ₀/34 | simulated insertion loss 0.13 dB; measured 0.28 ± 0.02 dB |
| Lalau-Keraly 2013, the same Y-branch | adjoint level set, steepest descent | shape | 102 (51 iterations) | 2D, then 3D FDTD | −0.07 dB, against the PSO's −0.13 dB after 1 500 (their Figs. 3–4) |
| Shen 2015, polarization beamsplitter | direct binary search | 20 × 20 pixels of 120 nm, and the thickness | up to 402 passes over the pixels; about 140 h a design | FDTD, 30 nm grid | simulated TM 89%, TE 81% |
| Mak 2016, 2 × 2 splitter | binary PSO | 144 cells of 200 nm | 500 iterations, population 21 | 3D FDTD, 50 × 50 × 40 nm | FOM min(P₁, P₂) = 0.174 |
| Minkov & Savona 2014, L3 cavity | GA, population 80 | 3 hole shifts | a few tens of generations | guided-mode expansion; 3D FDTD check | Q = 2.1×10⁶ (GME), 1.6×10⁶ (FDTD), up from 3.3×10⁵ |
| Barry 2020; Bennet 2024, Bragg mirror | DE (current-to-best/1), CMA-ES | 10 to 40 thicknesses | budgets of 10³ to 3.2×10⁴ | transfer matrices, exact | rediscover the quarter-wave stack |
| Pérez-López 2020, hexagonal mesh | GA, PSO, gradient descent | 72 phases | under 3 000 operations (GA) | simulated mesh with loss and crosstalk; 30-unit chip | all-cross configuration in 95% of 100 GA runs, 76.7% of PSO runs |

**Where they win:**

- **Few variables:** up to a few tens. The comparison with gradients then costs little.
- **Rugged or multimodal landscapes.** Bennet 2024 counted many local minima in the Bragg problem
  by starting BFGS from random points. DE with a gradient polish did best there
  *(from a plot, Fig. 6)*.
- **Discrete and binary choices:** pixels, materials, the order of a filter.
- **Black-box objectives**, including measured ones: Pérez-López 2020 calibrated a chip with a
  particle swarm.

**Where they lose:** as the variables grow. Lalau-Keraly 2013's comparison on the Y-branch above
is about fifteen times fewer solves for the gradient method, and its authors note the comparison
is sensitive to the simulation settings. Molesky 2018 write that with a genetic algorithm "locally
optimal designs are missed", and more iterations are needed to converge as far as a gradient
method does.

**The methods themselves:**

- genetic algorithms, on bits, integers or reals;
- CMA-ES (Hansen & Ostermeier 2001), the reference evolution strategy for continuous parameters;
- DE (Storn & Price 1997);
- PSO (Kennedy & Eberhart 1995), its binary form (Kennedy & Eberhart 1997) and its
  multi-objective form (Coello 2004);
- direct binary search, a first-improvement local search over single-pixel flips (Shen 2015).

### 1.3 Bayesian optimization and surrogate models

**What it does:** a Gaussian process models the objective from every evaluation so far. An
acquisition function, such as expected improvement (Jones 1998), picks the next point. It suits
objectives that cost minutes to hours, with few parameters (Shahriari 2016).

**Measured in photonics:**

- **Schneider 2019:** five optimizers on three problems, solved by JCMsuite's finite elements and
  repeated 6 times each:
  - a single-photon source with 6 parameters;
  - a grating reconstruction with 9;
  - an antireflection metasurface with 6.

  BO averaged 55% after about 1 200 solves for the source; the others stayed below 52% after
  2 500. With derivatives, BO reached the reconstruction's target in a median of about 200 solves;
  the others didn't within 2 500. In their supplement's multilayer filter with 7, 21 and 51
  parameters, BO won at 7 and L-BFGS-B as the dimension grew. At 51 parameters, BO took about half
  a minute to choose each point.
- **Elsawy 2019:** a GaN metasurface with 12 parameters, in 3D discontinuous-Galerkin
  time-domain:
  - BO (EGO): 88.0% after 80 initial samples and about 150 iterations;
  - CMA-ES: 88.1% after about 550 solver calls.

  They call BO "about 4 times faster". Whether their "iterations" count solver calls is not
  certain.

**Multi-fidelity and surrogate-assisted evolution** fit photonics' 2D-then-3D pipeline: a cheap
2D or effective-index model guides an expensive 3D one. Zhang 2013 optimized in 2D and checked in
3D; ROADMAP's pitfall table records 2D designs at 97% measuring 46–70% in 3D. genoxide plans
multi-fidelity BO and lq-CMA-ES (batch F). No photonics paper read for this survey measures a
multi-fidelity optimizer against a single-fidelity one. That is open.

### 1.4 Multi-objective (Pareto) methods

**What they do:** return a set of non-dominated designs instead of one weighted compromise:
efficiency against bandwidth, loss against footprint, crosstalk against insertion loss.

**In photonics:**

- **Passoni 2017:** Pareto fronts of efficiency against 1-dB bandwidth for apodized grating
  couplers, by multi-objective PSO. 20 agents for 1 000 iterations, in 2D FDTD with a 10 nm
  vertical mesh. 100 nm bandwidth is reachable at 47% efficiency with a 6 µm mode-field diameter,
  or 53% with 4 µm (their Fig. 6).
- **With gradients**, the usual route is a scan of ε-constraints (one objective constrained,
  the other optimized by MMA) or of weights. None of the topology-optimization papers read here
  traces a full front. Probst 2024 combine objectives with spline scalings instead.

genoxide's NSGA-II, NSGA-III, SPEA2, MOEA/D and SMS-EMOA return fronts, with the hypervolume
indicator. EHVI and ParEGO (batch E) would serve expensive objectives.

### 1.5 Robust and fabrication-aware optimization

**The formulations:**

- **Eroded, nominal and dilated designs.** Sigmund 2009; Wang, Lazarov & Sigmund 2010.
  Christiansen & Sigmund 2021's 2D demultiplexer is robust to ±8 nm: transmittance 0.32, 0.32
  and 0.32 at 1300 nm and 0.31, 0.31 and 0.30 at 1550 nm (Table VII, FEM, 10 nm elements).
- **Corners by min-max in epigraph form:**
  - Hammond 2021: ±20 nm etch, error bars up to 24 times smaller;
  - Probst 2024: ±40 nm layer misalignment, a grating coupler varying 0.16 dB against 0.9 dB when
    not robust (2D, 50 px/µm);
  - Shang 2023: the minimum transmission over 10 wavelengths.
- **Foundry rules as constraints.** Hammond 2021 imposed linewidth, spacing and area
  constraints. Schubert 2022 used a generator that is feasible by construction.
- **Lithography models inside the loop.** Khan et al. 2024 (Optics Letters; arXiv): an
  SWG-to-strip converter's simulated loss with the DUV model was 0.836 dB as designed and 0.111 dB
  designed with the model (their Table 1).

**What each needs from the optimizer:** constraint values and their Jacobians (MMA), and min-max
through an epigraph variable. All of these are formulations that photonoxide supplies. genoxide
needs nothing photonics-specific for them.

### 1.6 Hybrids: global, then local

- **A global search with a gradient polish.** Bennet 2024's best method on multilayers was
  quasi-oppositional DE for half the budget, then BFGS from its best (QNDE in Nevergrad's terms).
  genoxide's `polish` example runs the same pattern: CMA-ES or SHADE, then L-BFGS-B from the best.
- **Continuous, then discrete.** Topology optimization's continuation of β (Section 1.1) is one
  case. Piggott 2015's three stages are another: continuous, then level set, then broadband.
- **A cheap model, then an expensive one:** 2D, then 3D (Zhang 2013; Lalau-Keraly 2013).

### 1.7 Machine-learning surrogates and inverse networks

Networks trained on simulations can predict a response quickly, or propose a design from a
target response. The reviews state the limits:

- Liu 2018: one response has many designs, which gives inverse networks "conflicting training
  instances". Their fix is a tandem network.
- Jiang, Chen & Fan 2020: networks "cannot guarantee accuracy and should not be used in lieu of an
  electromagnetic simulator when an exact physics calculation is required".
- Wiecha 2021: many models "generalize relatively poorly" outside their training range, and a
  data-driven inverse design "can never outperform an iterative method if it is based on the same
  simulation model".

This plan doesn't add learned surrogates. genoxide's Gaussian process is the surrogate it uses.
ROADMAP already treats learned fabrication predictors as comparisons only (Gostimirovic 2022).

### 1.8 Circuit-level optimization

- **Mesh programming.** Reck and Clements meshes have closed-form decompositions (Reck 1994;
  Clements 2016). Optimization takes over when the components are imperfect.
  - Pai 2019 trained 128-mode Clements meshes against random unitaries with Adam.
  - Their Haar-random initialization of the phases converges faster than a uniform one.
  - A redundant mesh with twice the layers reaches up to five orders of magnitude lower error
    *(from a plot)*.
- **Self-configuration and calibration** with black-box objectives (Pérez-López 2020; Miller
  2013; Bogaerts 2020's review). A measured chip has no adjoint, except by in-situ gradient
  measurement (Hughes 2018), so value-only methods are used.
- **Filter synthesis.** Coupled-ring and lattice filters have analytic syntheses:
  - Little 1997's maximally flat and Chebyshev couplings;
  - Jinguji & Kawachi 1995's lattice synthesis. Jinguji & Oguma 2000, Table I, give the maximally
    flat half-band lattice with two delays: power couplings 0.067, 0.75 and 0.5.
  - Horst 2013's flat-top cascaded MZIs, couplings 0.5, 0.29 and 0.08 for two stages
    *(from a plot, Fig. 2)*.

  An optimizer is checked against them, and needed where they stop: loss, dispersion, an
  arbitrary target.
- **Synthesis, then local optimization.** Wang 2022 synthesized a fourth-order elliptic filter
  from a double-ring-loaded MZI at 34 dB extinction. Nelder-Mead, Powell and basin hopping then
  took it to 60 dB with 1 dB ripple, in their circuit model, with couplings 0.146, 0.726, 0.925,
  0.726 and 0.146. Gao 2023 report analytic S-matrix gradients about three times faster than
  differential evolution and numerical gradients on programmable-circuit synthesis.
- **Device and circuit co-design.** Mason 2025 optimized an inverse-designed demultiplexer
  together with its Bragg filters. The measured crosstalk was −34.5 dB at 15 nm spacing in
  silicon nitride, and the simulation reached below −40 dB with 300 periods.

photonoxide's circuit adjoint (0.4) gives exact gradients for any netlist.
`examples/circuit_splitter.rs`, `circuit_ring_critical.rs` and `circuit_fit.rs` already run
genoxide's L-BFGS-B, Adam and CMA-ES on it.

### 1.9 Comparison by problem type

From the sources above. The last column says what measured the claim.

| Problem | Variables | Gradient | Cost per evaluation | Methods the literature used and measured | Source |
|---|---|---|---|---|---|
| Topology (density) | 10⁴–10⁶ | adjoint | one or two solves | MMA/CCSA, GCMMA; Adam with generators | Sigmund 2011; Chen 2024; Hammond 2021; Schubert 2022 |
| Shape, few parameters | 10–100 | adjoint, or none | one solve | PSO; adjoint steepest descent (15× fewer solves on a Y-branch) | Zhang 2013; Lalau-Keraly 2013 |
| Shape, very costly 3D | 5–20 | none | minutes to hours | BO fewest solves at 6–12 parameters; L-BFGS-B overtakes as parameters grow | Schneider 2019; Elsawy 2019 |
| Pixels, binary | 10²–10³ | none in practice | one 3D solve | direct binary search; binary PSO | Shen 2015; Mak 2016 |
| Multilayers | 10–40 | analytic | microseconds | DE with a BFGS polish best; CMA-ES and BFGS alone worse *(from a plot)* | Bennet 2024; Barry 2020 |
| Cavities, hole shifts | 3–70 | none (GA era) or autodiff | minutes (GME) | GA; L-BFGS with autodiff | Minkov & Savona 2014; Minkov 2020 |
| Trade-offs | any | either | any | multi-objective PSO; NSGA-II | Passoni 2017 |
| Robustness | as above | as above | × corners | epigraph min-max with MMA | Hammond 2021; Probst 2024 |
| Filters (rings, lattices) | 3–10 couplings and phases | circuit adjoint | microseconds | analytic synthesis, then local search (Nelder-Mead, Powell, basin hopping) or gradients | Little 1997; Jinguji & Oguma 2000; Wang 2022; Gao 2023 |
| Meshes, simulated | N² phases | circuit adjoint | microseconds | Adam with Haar initialization | Pai 2019 |
| Meshes, measured | N² phases | none | a measurement | GA, PSO, gradient descent | Pérez-López 2020 |

## 2. Published results to reproduce

Each is a candidate example. The tolerance and grid of each are in
[Section 5](#5-the-examples-and-their-phases).

| # | Paper | Device | Method | Objective | Published figure of merit | Model or grid | photonoxide needs |
|---|---|---|---|---|---|---|---|
| 1 | Bennet 2024 (code: Zenodo 10.5281/zenodo.10246032) | Bragg mirror, 20 layers (and 10), n 1.8/1.4 on n = 1.8, λ 600 nm, thicknesses in [0, 214.29 nm] | DE, QODE, QNDE, CMA, BFGS | 1 − R | quarter-wave stack 83/107 nm the best found (Fig. 7c); lowest costs about 0.015 and 0.165 *(from a plot, Fig. 6)* | transfer matrices, exact | `Multilayer::reflection` (has it); ∂R/∂d |
| 2 | Little 1997 | maximally flat filters of 2 to 6 coupled rings | analytic synthesis | match 1/(1 + (Δω/ω_c)^{2N}) | inter-ring couplings μᵢ²/μ⁴ of Table I: 0.250; 0.125; 0.100/0.040; 0.0955/0.0295; 0.0915/0.0245/0.0179 | coupled-mode theory | the circuit solve (has it) |
| 3 | Pai 2019; Clements 2016 | N-mode Clements mesh | Adam, Haar initialization (Eq. 9) | (1/2N)‖Û − U‖² | Haar initialization's mean sensitivity index (N + 1)/3; convergence *(from plots)* | matrix model | a mesh builder (0.8) |
| 4 | Bogaerts 2011 | add-drop ring, lossy | (a multi-objective test) | drop at resonance against bandwidth | the analytic trade-off from Eqs. 5–8 | closed form | the circuit solve (has it) |
| 5 | Pérez-López 2020 | 36-unit hexagonal mesh, loss N(0.15, 0.05) dB, crosstalk | GA, PSO, gradient descent | all-cross: −(1/N) Σ log \|H\| | under 3 dB average error in 95% (GA) and 76.7% (PSO) of 100 runs | simulated mesh | a hexagonal mesh (0.8) |
| 6 | Passoni 2017 | apodized grating couplers, 220 nm SOI, 10° fibre | multi-objective PSO | efficiency and 1-dB bandwidth | 100 nm at 47% (6 µm MFD), 53% (4 µm) (Fig. 6) | 2D FDTD, 10 nm vertical mesh | grating geometry (#227), 2D FDFD or FDTD |
| 7 | Zhang 2013; Lalau-Keraly 2013 | Y-branch, 13 widths over 2 µm | PSO; adjoint level set | transmission | 0.13 dB (PSO, 3D FDTD at λ₀/34); −0.07 dB (adjoint) | 2D in the loop, 3D check | paths and widths (#227), shape derivatives (0.7) |
| 8 | Chen 2024; Schubert 2022 | 2D mode converter, 1.6 × 1.6 µm, 6 wavelengths, 1265–1295 nm | MMA with an epigraph variable; Adam with a generator | min-max of 1 − \|S₂₁\|², \|S₁₁\|² | 50 nm constraint: worst R −33.33 dB, worst T −0.07 dB at a measured 55 nm (the testbed's run.py) | 2D, 10 nm grid | density, filter, projection, length-scale metric (0.7) |
| 9 | Christiansen & Sigmund 2021 | 2D demultiplexer, 1300/1550 nm, 2 × 2 µm | robust, eroded/nominal/dilated (±8 nm) | transmission to each port | 0.32/0.32/0.32 and 0.31/0.31/0.30 (Table VII) | 2D FEM, 10 nm elements | as 8, with the robust formulation |
| 10 | Hammond 2021 | 2D mirror, bend and T-splitter, 3 × 3 µm | GCMMA, epigraph, foundry constraints | 1 − \|α\|² over 10 frequencies | bends above 89% average transmission; strictest 98.2% | Meep at 30 px/µm, design 60 px/µm | as 8, with area constraints |
| 11 | Shen 2015 | polarization beamsplitter, 20 × 20 pixels of 120 nm | direct binary search | mean of the TE and TM transmissions | simulated TM 89%, TE 81% | FDTD, 30 nm | 3D FDTD (0.5); the scan in genoxide |
| 12 | Minkov & Savona 2014 | L3 cavity, 3 hole shifts, n 3.46, d 0.55a, r 0.25a | GA | Q | 2.1×10⁶ (GME), 1.6×10⁶ (3D FDTD) | GME; FDTD at 20/a | photonic-crystal Q (0.14) |
| 13 | Elsawy 2019 | GaN metasurface, 12 parameters | BO and CMA-ES | diffraction efficiency | 88.0% (BO, 80 initial samples and about 150 iterations); 88.1% (CMA-ES, about 550) | 3D DGTD | 3D periodic solves (0.14) |
| 14 | Schneider 2019, supplement | multilayer filter, 7, 21 and 51 parameters | BO, DE, PSO, Nelder-Mead, L-BFGS-B | spectral error | which method wins at each dimension *(from plots)* | transfer matrices | its definition, from the supplement |
| 15 | Jinguji & Oguma 2000; Jinguji & Kawachi 1995 | maximally flat half-band MZI lattice, delays 2ΔL and ΔL | analytic synthesis | match the half-band target | power couplings 0.067, 0.75, 0.5 (θ = 0.0833π, 0.3333π, 0.25π; Table I) | analytic | the circuit solve (has it) |
| 16 | Wang 2022 | double-ring-loaded MZI, fourth-order elliptic | synthesis, then Nelder-Mead, Powell, basin hopping | extinction with 1 dB ripple | 34 dB synthesized, 60 dB optimized; couplings 0.146, 0.726, 0.925, 0.726, 0.146 | circuit model | the circuit solve (has it) |

Two notes on these sources:

- **Little 1997's Table I.** Little's N = 4 row (0.100, 0.040) differs from the Butterworth
  prototype values. The relation μᵢ²/μ⁴ = g₁²/(4 gᵢ gᵢ₊₁) reproduces every other row exactly, and
  gives 0.1036 and 0.0429 for N = 4. This relation was derived for this survey and is not in the
  paper. The example checks N = 4 against the prototype values and prints both.
- **The Bragg problems differ by source.** Barry 2020 states its bounds as n in [1.4, 1.7] and
  thicknesses of 10–300 nm, without the substrate for its main case. Nevergrad's benchmark of the
  same name is another problem again: ε in [2, 3], λ 600 nm, substrate √3. Example 1 follows
  Bennet 2024's published code, which states everything. For it:
  - the quarter-wave stack's reflectance is R = 0.985521 with 20 layers and 0.835144 with 10, in
    closed form;
  - those match the lowest costs in Bennet's Fig. 6.

## 3. genoxide's coverage

Checked on genoxide's main branch on 2026-10-08. photonoxide's examples use 0.13.1, which has
everything in the first table.

| Method | genoxide | Photonic use |
|---|---|---|
| L-BFGS-B, gradients supplied or by differences | `Lbfgsb` (0.12) | circuit and shape parameters, fits |
| Gradient descent, momentum, Nesterov, Adam, AdamW, schedules | `FirstOrder` (0.12) | densities with many pixels; generators |
| MMA and GCMMA, with constraint values and Jacobians | `Mma`, `Constrained` (0.12) | topology optimization, length-scale constraints, epigraph min-max |
| Continuation, the optimizer's state kept | `Continuation` (0.12) | β schedules |
| Nelder-Mead with restarts | `NelderMead` (0.12) | a few parameters, no gradient |
| GA (generational, steady-state, (μ+λ), (μ,λ), memetic) on binary, integer, real and permutation genomes | `Ga`, `SteadyGa` | discrete choices, pixels |
| ES, CMA-ES (IPOP, BIPOP, separable) | `Es`, `Cmaes` | continuous parameters, up to hundreds |
| DE (rand/1, best/1, current-to-pbest/1; JADE, SHADE, L-SHADE) | `De` | multilayers. Current-to-pbest with p at the best individual is Barry's current-to-best/1 |
| PSO | `Pso` | shape parameters |
| Local search: hill climbing, annealing, tabu, iterated | `LocalSearch` | pixel flips at random neighbours |
| Islands, asynchronous engine, batch evaluation | `Islands`, `AsyncEngine`, `Batch` | many solves at once |
| BO: Gaussian process, log-EI, EI, PI, UCB; batch, asynchronous, constrained, integer | `Bo`, `model::gp` (0.13) | costly 3D evaluations |
| NSGA-II, NSGA-III, SPEA2, MOEA/D, SMS-EMOA, hypervolume | `multi` | Pareto fronts |

**Already planned in genoxide**, and used by this plan when they land:

| Method | genoxide batch | Used for |
|---|---|---|
| Augmented Lagrangian, SQP | C (0.14) | ROADMAP 0.7's constraints; many constraints |
| Levenberg-Marquardt, BFGS, trust region | D1 (0.15) | fitting measured spectra; mesh calibration |
| BOBYQA, COBYLA, MADS | D2 (0.15) | costly smooth black boxes |
| ParEGO, EHVI, TuRBO, mixed variables | E (0.16) | costly Pareto fronts; a filter's order with its couplings |
| Multi-fidelity BO, lq-CMA-ES, DIRECT | F (0.16) | 2D guiding 3D |
| MAP-Elites, CMA-ME | 0.17 | many different designs with one performance |

**Missing, and general enough to add to genoxide:**

1. **A systematic neighbourhood scan for binary and integer genomes** (needed by example 11).
   Visit every gene in a fresh random order each pass, try its move, and keep it if the score
   improves. Stop when a pass improves by less than a tolerance or a pass limit is reached. Shen
   2015's direct binary search is this method on pixels. `LocalSearch` today draws random
   neighbours instead of scanning them all. Opened in genoxide as [tachsin/genoxide#426](https://github.com/tachsin/genoxide/issues/426).
2. **Quasi-oppositional initialization for DE** (Rahnamayan 2008). Optional: it is the "Q" of
   Bennet 2024's best methods, and example 1 runs without it.
3. **Binary PSO** (Kennedy & Eberhart 1997) and **multi-objective PSO** (Coello 2004).
   Optional: they are Mak 2016's and Passoni 2017's methods. Examples 6 and 11 use genoxide's GA,
   local search and NSGA-II instead and compare the results, not the methods.

Items 2 and 3 are listed in the photonoxide issue as optional follow-ups, without genoxide
issues, until an example needs them.

**Nothing photonics-specific goes into genoxide.** Some pieces stay in photonoxide because they
are photonics:

- the epigraph variable for min-max over wavelengths and corners;
- eroded and dilated designs;
- length-scale constraints;
- Haar initialization of mesh phases.

These are formulations or starting points that photonoxide passes to genoxide's general methods.

## 4. What photonoxide supplies and what genoxide supplies

**photonoxide supplies:**

- **Objectives** with their gradients:
  - the circuit's (`circuit::objective`, done);
  - a multilayer's reflectance and transmittance, with derivatives in thickness and index (new);
  - a port mode's power in FDFD (the 2D adjoint, done) and FDTD (#167);
  - Q factors (0.14);
  - figures of merit in dB, worst cases and averages over wavelengths.
- **Parametrizations:**
  - component parameters with units and ranges (`Component::parameters`, done);
  - layer thicknesses;
  - spline widths along a path (#227);
  - pixels;
  - densities with filter and projection (0.7);
  - level sets (0.7);
  - Haar-random mesh phases.
- **Constraints:** values and Jacobians for genoxide's `Constrained`:
  - length scales (Zhou 2015; Hammond 2021);
  - foundry rules (Hammond 2021);
  - power budgets;
  - the epigraph variable's constraints for min-max.
- **Robust formulations:** eroded, nominal and dilated designs; corners; the lithography models
  of 0.11.
- **Multi-objective reporting:** a run's non-dominated set, each point with its objectives, its
  design and the grid it was computed on; the hypervolume over the run.
- **The run:**
  - an `optimize` job (method, seed, budget, a hard timeout);
  - every evaluation recorded as an event (the objective, the constraints, the gradient's norm,
    the best so far) and replayed bit for bit;
  - the studio's views.
- **Validation:**
  - every gradient against finite differences;
  - each example against its paper on a stated grid, with its convergence;
  - every result the same bits on 1 and 20 threads.

**genoxide supplies:**

- every method that searches;
- seeds;
- stop conditions;
- batch, parallel and asynchronous evaluation;
- the state kept across continuation stages;
- checkpoints.

**The studio shows a run live:**

- a convergence plot: the objective per evaluation and the best so far, on a log axis, with the
  constraints' violation;
- a parameter view: thicknesses as a stack, couplings on the chip view, a design's pixels;
- for multi-objective runs, a Pareto front that grows as the run goes, with its hypervolume;
- the Compare page overlays runs with different seeds or methods.

The window takes the job from the command line, starts by itself and exits when done. A hard
timeout is always set, as AGENTS.md requires.

**Determinism.** genoxide gives the same bits for a seed on any platform and thread count.
photonoxide's solvers give the same bits on any thread count. An optimization run is therefore
the same bits on any thread count, and every example checks that.

## 5. The examples and their phases

Each phase is one PR. Issue #244 has the details. Every example follows examples/README.md:

- one paper's numbers;
- `common::Checks`;
- the grid printed with every number;
- seeds explicit;
- the output compared in CI.

| Phase | Contents | Examples | Waits for |
|---|---|---|---|
| 1 | The `optimize` job, run records of every evaluation, the convergence plot; the three circuit examples moved onto it with their counts unchanged | (existing) | |
| 2 | A multilayer's ∂R/∂d and ∂R/∂n, checked against differences | `optimize_bragg` (1) | |
| 3 | Filter synthesis checked by optimization; synthesis then optimization | `optimize_ring_flat` (2), `optimize_mzi_lattice` (15), `optimize_ring_mzi` (16) | |
| 4 | Multi-objective runs: the Pareto record and the studio's front | `optimize_ring_pareto` (4) | |
| 5 | Mesh optimization with Haar initialization | `optimize_mesh` (3) | the mesh builder (0.8), or its own small builder |
| 6 | Value-only calibration of a simulated mesh | `optimize_mesh_selfconfig` (5) | a hexagonal mesh |
| 7 | A method study on multilayers: evaluations to target for BO, DE, CMA-ES and L-BFGS-B, by seed (a page in docs/methods, not an example) | (14, if its definition is recovered) | phase 2 |
| 8 | Shape optimization with PSO and the adjoint | `optimize_ybranch` (7) | #227, shape derivatives |
| 9 | Topology optimization: density, filter, projection, epigraph, length scale | `optimize_mode_converter` (8), `optimize_demux_robust` (9), `optimize_foundry_bend` (10) | 0.7 |
| 10 | Grating couplers' Pareto fronts | `optimize_grating_pareto` (6) | #227 and gratings |
| 11 | Direct binary search | `optimize_pbs` (11) | 3D FDTD, the genoxide scan |
| 12 | Cavities and metasurfaces | `optimize_l3` (12), `optimize_metasurface` (13) | 0.14 |

## 6. Decisions for the owner

*Decided by the owner on 2026-10-09: the recommendations, for all of them.*

1. **Where the adapter lives.** genoxide is a dev-dependency today, "so the library's API doesn't
   follow genoxide's versions" (Cargo.toml). An `optimize` job run by `photonoxide run` needs it
   at run time. The options:
   - **(a)** the library keeps no genoxide dependency and supplies objectives and gradients as
     plain functions; the studio crate holds the adapter and runs the job;
   - **(b)** an optional `genoxide` feature in the library, off by default;
   - **(c)** a regular dependency.

   Recommended: (a). The library's API stays independent, and the studio already builds the
   examples with genoxide.
2. **Plots as references.** Several sources give their numbers only in figures: Bennet 2024's
   costs and Pai 2019's convergence. Should an example check against a closed form where one
   exists, and quote the plot as context? Recommended: yes. Example 1 checks the quarter-wave
   stack's exact reflectance and thicknesses. Example 3 checks an exact decomposition to
   round-off, and Pai's mean sensitivity index (N + 1)/3.
3. **Statistics as references.** Pérez-López 2020 report success rates over 100 runs, with
   settings that differ between the text and the supplement, and other implementations of GA and
   PSO. Should example 5 check only what is deterministic? That would be the circuit adjoint's
   gradient descent reaching the target, with genoxide's success rates over 100 seeds reported
   beside theirs. Recommended: yes. Checking rates from other implementations would test their
   implementations, not ours.
4. **Little 1997's N = 4 row.** Check against the Butterworth prototype values (0.1036, 0.0429)
   and print the paper's (0.100, 0.040) beside them? Recommended: yes, with the relation derived
   in the example's doc comment.
5. **genoxide issues.** One issue opened now, for the neighbourhood scan that example 11 needs.
   The optional methods (quasi-oppositional initialization, binary PSO, multi-objective PSO) are
   listed in photonoxide's issue only. Open genoxide issues for them now, or when an example needs
   them? Recommended: when needed.
6. **The order.** Phases 1 to 4 need nothing new from photonoxide's solvers and can start now. 5
   and 6 overlap 0.8's meshes, and the rest wait for #227, 0.7 and 0.14. Start with 1 to 4, before
   0.7? Recommended: yes. They build the run records, the job and the studio views that 0.7's
   dashboard needs.
7. **ROADMAP 0.7's genoxide line.** It lists MMA and continuation as to be added to genoxide. Both
   have been in genoxide since 0.12, so this PR updates the line.

## 7. Open-source tools in the field

Described by what they do, with their licence as their repositories state it on 2026-10-08.

| Tool | What it does | Licence |
|---|---|---|
| Meep | FDTD with an adjoint module for topology optimization | GPL-2.0 (run only as an external program, AGENTS.md) |
| Ceviche | 2D FDFD with automatic differentiation | MIT |
| ceviche-challenges | Schubert 2022's benchmark problems | Apache-2.0 |
| photonics-opt-testbed | Chen 2024's problems, designs and length-scale metric | MIT |
| invrs-gym, invrs-opt | inverse-design challenges with baselines; optimizers | MIT |
| SPINS-B | inverse design (FDFD) | GPL-3.0 (external only) |
| legume | guided-mode expansion with automatic differentiation | MIT |
| angler | FDFD inverse design, linear and nonlinear | MIT |
| EMopt | shape and topology optimization | BSD-3-Clause |
| fdtdx | FDTD in JAX | MIT |
| Tidy3D (client) | a cloud FDTD's client, with an inverse-design plugin | LGPL-2.1 |
| PyMoosh | multilayers and their optimization (Bennet 2024's solver) | MIT |
| gdsfactory, SAX, Simphony | layout; differentiable and other circuit simulation | MIT; Apache-2.0; MIT |
| Photontorch | circuit simulation in PyTorch | AGPL-3.0 (external only) |
| NLopt | MMA, CCSA and many others | MIT, or LGPL with its Luksan routines |
| pycma, pymoo, DEAP, Nevergrad, BoTorch, Optuna | CMA-ES; multi-objective; evolutionary; gradient-free portfolios; Bayesian optimization; hyperparameter search | BSD-3; Apache-2.0; LGPL-3.0; MIT; MIT; MIT |

Their papers and problem definitions may be read, as the backends plan allows for libraries. No
code is ported from any of them, and the GPL ones are never read for porting (AGENTS.md).

## References

Every DOI below was checked on Crossref on 2026-10-08.

**Methods**
- K. Svanberg, Int. J. Numer. Methods Eng. 24, 359 (1987). [10.1002/nme.1620240207](https://doi.org/10.1002/nme.1620240207)
- K. Svanberg, SIAM J. Optim. 12, 555 (2002). [10.1137/S1052623499362822](https://doi.org/10.1137/S1052623499362822)
- N. Hansen, A. Ostermeier, Evol. Comput. 9, 159 (2001). [10.1162/106365601750190398](https://doi.org/10.1162/106365601750190398)
- R. Storn, K. Price, J. Glob. Optim. 11, 341 (1997). [10.1023/A:1008202821328](https://doi.org/10.1023/A:1008202821328)
- J. Kennedy, R. Eberhart, Proc. ICNN'95 4, 1942 (1995). [10.1109/ICNN.1995.488968](https://doi.org/10.1109/ICNN.1995.488968)
- J. Kennedy, R. C. Eberhart, IEEE Int. Conf. Syst. Man Cybern. (1997), binary PSO. [10.1109/ICSMC.1997.637339](https://doi.org/10.1109/ICSMC.1997.637339)
- C. A. C. Coello, G. T. Pulido, M. S. Lechuga, IEEE Trans. Evol. Comput. 8, 256 (2004). [10.1109/TEVC.2004.826067](https://doi.org/10.1109/TEVC.2004.826067)
- K. Deb et al., IEEE Trans. Evol. Comput. 6, 182 (2002), NSGA-II. [10.1109/4235.996017](https://doi.org/10.1109/4235.996017)
- S. Rahnamayan, H. R. Tizhoosh, M. M. A. Salama, IEEE Trans. Evol. Comput. 12, 64 (2008). [10.1109/TEVC.2007.894200](https://doi.org/10.1109/TEVC.2007.894200)
- D. R. Jones, M. Schonlau, W. J. Welch, J. Glob. Optim. 13, 455 (1998). [10.1023/A:1008306431147](https://doi.org/10.1023/A:1008306431147)
- B. Shahriari et al., Proc. IEEE 104, 148 (2016). [10.1109/JPROC.2015.2494218](https://doi.org/10.1109/JPROC.2015.2494218)

**Gradients, topology and fabrication**
- G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004). [10.1364/OL.29.002288](https://doi.org/10.1364/OL.29.002288)
- C. M. Lalau-Keraly et al., Opt. Express 21, 21693 (2013). [10.1364/OE.21.021693](https://doi.org/10.1364/OE.21.021693)
- O. Sigmund, Acta Mech. Sin. 25, 227 (2009). [10.1007/s10409-009-0240-z](https://doi.org/10.1007/s10409-009-0240-z)
- F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (published online 24 December 2010; the June 2011 issue). [10.1007/s00158-010-0602-y](https://doi.org/10.1007/s00158-010-0602-y)
- O. Sigmund, Struct. Multidiscip. Optim. 43, 589 (2011). [10.1007/s00158-011-0638-7](https://doi.org/10.1007/s00158-011-0638-7)
- A. Y. Piggott et al., Nat. Photonics 9, 374 (2015). [10.1038/nphoton.2015.69](https://doi.org/10.1038/nphoton.2015.69)
- A. Y. Piggott et al., Sci. Rep. 7, 1786 (2017). [10.1038/s41598-017-01939-2](https://doi.org/10.1038/s41598-017-01939-2)
- S. Molesky et al., Nat. Photonics 12, 659 (2018). [10.1038/s41566-018-0246-9](https://doi.org/10.1038/s41566-018-0246-9)
- M. Minkov et al., ACS Photonics 7, 1729 (2020). [10.1021/acsphotonics.0c00327](https://doi.org/10.1021/acsphotonics.0c00327)
- R. E. Christiansen, O. Sigmund, J. Opt. Soc. Am. B 38, 496 (2021). [10.1364/JOSAB.406048](https://doi.org/10.1364/JOSAB.406048)
- A. M. Hammond et al., Opt. Express 29, 23916 (2021). [10.1364/OE.431188](https://doi.org/10.1364/OE.431188)
- M. F. Schubert et al., ACS Photonics 9, 2327 (2022). [10.1021/acsphotonics.2c00313](https://doi.org/10.1021/acsphotonics.2c00313)
- C. Shang et al., ACS Photonics 10, 1019 (2023). [10.1021/acsphotonics.3c00040](https://doi.org/10.1021/acsphotonics.3c00040)
- M. Chen et al., J. Opt. Soc. Am. B 41, A161 (2024). [10.1364/JOSAB.506412](https://doi.org/10.1364/JOSAB.506412)
- M. J. Probst et al., Opt. Express 32, 31448 (2024). [10.1364/OE.527442](https://doi.org/10.1364/OE.527442)
- S. R. Khan et al., Opt. Lett. 50, 117 (published online 18 December 2024; the 1 January 2025 issue), arXiv:2410.07353. Crossref lists the first author as "Shaheer Raza", and arXiv as "Shaheer Khan". [10.1364/OL.543961](https://doi.org/10.1364/OL.543961)
- S. Mason et al., arXiv:2509.07233 (2025), no journal DOI yet.

**Evolutionary, Bayesian and multi-objective photonics**
- Y. Zhang et al., Opt. Express 21, 1310 (2013). [10.1364/OE.21.001310](https://doi.org/10.1364/OE.21.001310)
- M. Minkov, V. Savona, Sci. Rep. 4, 5124 (2014). [10.1038/srep05124](https://doi.org/10.1038/srep05124)
- B. Shen et al., Nat. Photonics 9, 378 (2015). [10.1038/nphoton.2015.80](https://doi.org/10.1038/nphoton.2015.80)
- J. C. C. Mak et al., Opt. Lett. 41, 3868 (2016). [10.1364/OL.41.003868](https://doi.org/10.1364/OL.41.003868)
- M. Passoni et al., Appl. Phys. Lett. 110, 041107 (2017). [10.1063/1.4974992](https://doi.org/10.1063/1.4974992)
- P.-I. Schneider et al., ACS Photonics 6, 2726 (2019). [10.1021/acsphotonics.9b00706](https://doi.org/10.1021/acsphotonics.9b00706)
- M. M. R. Elsawy et al., Sci. Rep. 9, 17918 (2019). [10.1038/s41598-019-53878-9](https://doi.org/10.1038/s41598-019-53878-9)
- M. A. Barry et al., Sci. Rep. 10, 12024 (2020). [10.1038/s41598-020-68719-3](https://doi.org/10.1038/s41598-020-68719-3)
- P. Bennet et al., J. Opt. Soc. Am. B 41, A126 (2024). [10.1364/JOSAB.506389](https://doi.org/10.1364/JOSAB.506389)

**Circuits**
- M. Reck et al., Phys. Rev. Lett. 73, 58 (1994). [10.1103/PhysRevLett.73.58](https://doi.org/10.1103/PhysRevLett.73.58)
- K. Jinguji, M. Kawachi, J. Lightwave Technol. 13, 73 (1995). [10.1109/50.350643](https://doi.org/10.1109/50.350643)
- B. E. Little et al., J. Lightwave Technol. 15, 998 (1997). [10.1109/50.588673](https://doi.org/10.1109/50.588673)
- K. Jinguji, T. Oguma, J. Lightwave Technol. 18, 252 (2000). [10.1109/50.822800](https://doi.org/10.1109/50.822800)
- F. Horst et al., Opt. Express 21, 11652 (2013). [10.1364/OE.21.011652](https://doi.org/10.1364/OE.21.011652)
- M. Wang, X. Chen, U. Khan, W. Bogaerts, Sci. Rep. 12, 1482 (2022). [10.1038/s41598-021-04598-6](https://doi.org/10.1038/s41598-021-04598-6)
- Z. Gao et al., Photonics Res. 11, 643 (2023). [10.1364/PRJ.474606](https://doi.org/10.1364/PRJ.474606)
- W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue). [10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017)
- D. A. B. Miller, Photon. Res. 1, 1 (2013). [10.1364/PRJ.1.000001](https://doi.org/10.1364/PRJ.1.000001)
- W. R. Clements et al., Optica 3, 1460 (2016). [10.1364/OPTICA.3.001460](https://doi.org/10.1364/OPTICA.3.001460)
- T. W. Hughes et al., Optica 5, 864 (2018). [10.1364/OPTICA.5.000864](https://doi.org/10.1364/OPTICA.5.000864)
- S. Pai et al., Phys. Rev. Appl. 11, 064044 (2019). [10.1103/PhysRevApplied.11.064044](https://doi.org/10.1103/PhysRevApplied.11.064044)
- D. Pérez-López et al., Nat. Commun. 11, 6359 (2020). [10.1038/s41467-020-19608-w](https://doi.org/10.1038/s41467-020-19608-w)
- W. Bogaerts et al., Nature 586, 207 (2020). [10.1038/s41586-020-2764-0](https://doi.org/10.1038/s41586-020-2764-0)

**Machine learning**
- D. Liu et al., ACS Photonics 5, 1365 (2018). [10.1021/acsphotonics.7b01377](https://doi.org/10.1021/acsphotonics.7b01377)
- J. Jiang, M. Chen, J. A. Fan, Nat. Rev. Mater. 6, 679 (published online 17 December 2020; volume 6, 2021). [10.1038/s41578-020-00260-1](https://doi.org/10.1038/s41578-020-00260-1)
- P. R. Wiecha et al., Photonics Res. 9, B182 (2021). [10.1364/PRJ.415960](https://doi.org/10.1364/PRJ.415960)

# Optimizing a device

A device's page offers its optimization: the design's parameters moved between bounds by one of
genoxide's methods until an objective is as small, as large or as near its target as it gets,
within constraints. This page describes what photonoxide supplies for that: the objectives and
constraints each device declares, their derivatives and how they are checked, the `optimize`
job, and its record. The optimizers themselves are genoxide's (AGENTS.md: optimizers come from
genoxide); the plan behind this is [docs/plans/optimization.md](../plans/optimization.md) and
issue #244.

## Objectives and constraints

`Device::objectives()` and `Device::constraints()` (`photonoxide::device::objective`) list what
an optimization of a device offers. Each is an expression ([expressions.md](expressions.md)) of
the device's **outputs** at a design:

- `T_q_p`: |S_qp|², the power from port p into port q (`T_o3_o1`), at each wavelength;
- each figure of its model by its symbol (`n_g`, `FSR`, `Q`, `kappa2`, `extinction`);
- each of its parameters by name (`gap`, `loss`, `radius`); a figure of the same name hides one;
- `lambda`, the wavelength in µm.

Every output is a plain number in its unit (a length in µm, the FSR in nm). An objective has a
goal: to minimize it, maximize it, or bring it to a target. A constraint keeps its expression at
most or at least a bound. A job may name a declared one by its symbol, or give an expression of
its own.

| Device | Objectives | Constraints |
|---|---|---|
| waveguide, rib waveguide | `loss` (−10 log₁₀ T₂₁, dB), `n_g` and `n_eff` to a target | smallest `width`; `loss` at most |
| bend | `loss` through it, `loss_90` per quarter turn | smallest `width`; `radius` at most (footprint); `loss` at most |
| directional coupler | `split` = T₃₁/(T₃₁ + T₄₁) to a target, its `flatness` (the split's spread over the band), `excess_loss` | smallest `width` and `gap`; `length` at most; `excess_loss` at most |
| MMI 1x2, Y-branch, MMI 2x2 | `imbalance` (\|10 log₁₀ of the outputs' ratio\|, dB), `excess_loss` | `excess_loss` at most |
| all-pass ring | `extinction`, `critical` coupling (κ² − (1 − a²), to 0), loaded `Q` and `FSR` to targets | smallest `width` and `gap`; `FSR`, `Q`, `extinction` at least |
| add-drop ring | `drop` on resonance, `Q`, `FSR` | smallest `width`, `gap`, `gap_drop`; `FSR`, `Q` at least |
| MZI | `cross`, `bar`, the through port's `extinction` over the band | smallest `width` and `gap` |
| Y-branch MZI | `T`, its `extinction` over the band | smallest `width` |

The ring's a² is its round trip's power transmission, 10^(−(αL + extra)/10), from its guide's
`loss` α (dB/cm) over its round trip `L` and its `round_trip_loss`: κ² = 1 − a² is critical
coupling (W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011;
the January 2012 issue), doi:10.1002/lpor.201100017), where the dip goes to zero.

A waveguide's single-mode margin is not declared yet: the analytic model knows only the
fundamental mode, and an estimate of the second mode's cutoff (the effective index method's) isn't
validated here against the full-vector solver. It needs that solver's second mode (on the half
behind the odd mirror wall), with its own validation.

## Over a band and corners

Each quantity is taken at every wavelength of the task's band (the design's own wavelength alone
by default) and at the nominal design and each corner (offsets of its parameters, as a corners
study's). Its samples become one number:

- **worst** (the default): an objective's largest cost; a constraint's value nearest breaking
  its bound;
- **average**: the mean;
- **spread**: the largest value less the smallest, a flatness (the coupler's `flatness` always
  takes it).

An objective's cost, minimized, is its value, its negative, or (value − target)². A constraint's
g = (value − bound) or (bound − value) is met when g ≤ 0; the violation is Σ max(0, g).

## Derivatives

`Problem::evaluate` gives each cost's gradient and each constraint's, variable by variable:

- **Exact** where every quantity has one: an expression of the powers and the parameters, in a
  parameter whose component gives its derivatives (a coupler's or waveguide's length, a bend's
  angle, a ring's radius and coupling length, an MZI's arms and phase through the circuit
  adjoint, [circuit-adjoint.md](circuit-adjoint.md)). dS/dθ comes from the component's own
  derivatives through the device's values (`Built::derivatives`) and the design's expressions
  (`Design::sensitivities`); dT/dθ = 2 Re(conj(S) dS/dθ), and the expression's by forward mode
  (`Expr::eval_dual`). The reduction's derivative is its weights: the worst sample's derivative,
  the mean's, or the largest less the smallest.
- **Central differences** elsewhere: a figure (n_g, Q, FSR), or a parameter the component is
  built from (a width, a gap: another component), with a step ε^(1/3) max(|x|, 1) (as the circuit
  adjoint's), at most an eighth of the bounds' span, second-order one-sided at a bound. The
  variables' differences are computed side by side on rayon's threads and gathered in order.

**Every exact derivative is checked before the optimizer uses it.** At the start design,
`Problem::check_gradients` compares each with fourth-order central differences,
(f(x−2h) − 8f(x−h) + 8f(x+h) − f(x+2h))/12h, h = 5×10⁻⁶ max(|x|, 1) (small enough to resolve an
MZI arm's phase, whose power's period in its length is 0.4 µm), sample by sample before the
reduction (whose worst case has kinks where the worst sample changes). A variable whose
derivatives all agree to 10⁻⁶ (relative, with a floor of 10⁻¹⁰ of the quantity) uses them; one
that doesn't is differenced instead. A quantity that disagrees is checked again at steps 10 and
100 times smaller: a chip's ring resonance can be sharp on the first step's scale (a bus coupler's
length moves the ring's phase), and the differences' own error falls as the step's fourth power,
so an exact derivative is refused only where no step confirms it. The run records every check,
at the step that decided it.

## The optimize job

A `[device]` job's `[task]` of kind `"optimize"` (`photonoxide::job`'s `optimize.rs`):

```toml
name = "coupler-3db"
timeout_minutes = 2       # required: an optimization always has a hard limit

[device]
kind = "directional coupler"
fidelity = "analytic"

[task]
kind = "optimize"
method = "lbfgsb"         # genoxide's: lbfgsb, adam, mma, gcmma, nelder-mead, cmaes, de,
seed = 1                  #   pso, nsga2, sms-emoa
evaluations = 200         # the budget: the optimizer's evaluations
from_um = 1.53            # the band: the design's own wavelength alone when left out
to_um = 1.57
points = 5
reduce = "worst"          # or "average"

[[task.variable]]
parameter = "length"
from = 2.0
to = 30.0

[[task.objective]]
name = "split"            # declared, or: expression = "T_o3_o1", goal = "maximize"
target = 0.5

[[task.constraint]]       # (not with lbfgsb or adam, which ignore constraints)
name = "excess_loss"      # declared, or: expression = "gap", at_least = 0.12
at_most = 0.2
```

| Method | genoxide's | Uses | Constraints | Objectives | Paper |
|---|---|---|---|---|---|
| `lbfgsb` | L-BFGS-B | the gradient | no | 1 | Byrd, Lu, Nocedal, Zhu 1995, doi:10.1137/0916069 |
| `adam` | Adam | the gradient | no | 1 | Kingma & Ba 2015, arXiv:1412.6980 |
| `mma` | MMA | the gradient and the constraints' Jacobian | by their values | 1 | Svanberg 1987, doi:10.1002/nme.1620240207 |
| `gcmma` | GCMMA, the globally convergent form | as MMA's | by their values | 1 | Svanberg 2002, doi:10.1137/S1052623499362822 |
| `nelder-mead` | Nelder-Mead | values | Deb's rules | 1 | Nelder & Mead 1965, doi:10.1093/comjnl/7.4.308 |
| `cmaes` | CMA-ES | values | Deb's rules | 1 | Hansen & Ostermeier 2001, doi:10.1162/106365601750190398 |
| `de` | differential evolution | values | Deb's rules | 1 | Storn & Price 1997, doi:10.1023/A:1008202821328 |
| `pso` | particle swarm | values | Deb's rules | 1 | Kennedy & Eberhart 1995, doi:10.1109/ICNN.1995.488968 |
| `nsga2` | NSGA-II | values | constrained dominance | 2 | Deb et al. 2002, doi:10.1109/4235.996017 |
| `sms-emoa` | SMS-EMOA | values | constrained dominance | 2 | Beume, Naujoks, Emmerich 2007, doi:10.1016/j.ejor.2006.08.008 |

`job::check` refuses an unknown method, a missing seed, budget or `timeout_minutes`, a variable
twice, bounds that aren't (or outside its parameter's range, or that the design's value isn't
between: a run starts from the design), an objective or constraint the device hasn't, an
expression that doesn't parse or isn't a number, two objectives for a method of one, and
constraints for L-BFGS-B or Adam. `job::optimize_from_device` writes such a job from a design.

MMA takes one evaluation an iteration and has no proof of convergence from any start: on the
coupler's split, a squared distance from 1/2, it ends cycling between two lengths 0.07 and
0.18 µm either side of the 3 dB length (the program's tests). GCMMA's conservative steps reach it
to 10⁻⁶ µm; it is the one to take where MMA doesn't settle.

**Who runs it.** The library has no genoxide dependency (#244's decision 1): `job::execute`
refuses an optimize job and says so, and the `photonoxide` program runs it. `photonoxide run
<job>` opens the studio's window, runs it live and closes when it is done (AGENTS.md:
optimizations run in the studio); `--headless` runs it without a window, for batches. The
program works in the variables scaled to [0, 1] by their bounds (each gradient scaled by its
span), evaluates a generation's points side by side on rayon's threads and records them in the
optimizer's order, and stops at the budget, the optimizer's own end, the hard limit or Stop.

## Continuation

A `[task.continuation]` runs the optimization in stages, a parameter set to each stage's value
in turn: a projection's sharpness β, a smoothing's, a penalty's weight (Wang, Lazarov & Sigmund
2010, doi:10.1007/s00158-010-0602-y).

```toml
[task.continuation]
parameter = "beta"        # one the design names, or a new name the expressions use
values = [1, 4, 16, 64, 256]
generations = 100         # each stage's budget; without it a stage ends when the method converges
keep = "state"            # the default; "point" starts the optimizer's state again at each stage

[[task.objective]]
expression = "tanh(beta*(T_o3_o1/(T_o3_o1 + T_o4_o1) - 0.5))/beta"
goal = "target"
target = 0.0
```

The program runs the method inside genoxide's `Continuation`, which keeps its state from one
stage to the next: Adam's moments and step count, L-BFGS-B's curvature pairs (built with
`keep_pairs`), MMA's and GCMMA's asymptotes, a Nelder-Mead simplex, a CMA-ES distribution. The
other methods (de, pso, nsga2, sms-emoa) have no such state in genoxide, and `job::check` refuses
a continuation for them. Each stage begins with the last stage's point evaluated again on the new
value. A device parameter (or one the design names) is written into the design at each stage; a
new name is a plain number the objectives' and constraints' expressions use. The task's
`evaluations` bound the whole run.

**Why the state is kept.** Resetting Adam's state between projection stages destroyed designs
at high β (ROADMAP's pitfalls). After a reset, Adam's first step is its whole learning rate
whatever the gradient (its bias correction makes m̂/√v̂ = ±1), and that step throws a converged
design out of a sharpened optimum onto the projection's plateau, where the gradient vanishes.
The program's test `a_continuation_keeps_adams_state_across_its_stages` runs the coupler above
both ways (Adam, learning rate 0.05, lengths 2 to 30 µm, the analytic model at 1.55 µm: exact, no
grid). Kept, each stage starts at the last stage's point, the step after it is under a
twentieth of a reset's, and the run ends within 10⁻⁴ µm of L_x/2 at β = 256. Reset, each stage's
first step is the learning rate's 1.4 µm to 10⁻³, and at β = 64 it throws the design onto the
plateau, where it stays more than 1 µm off.

## The record

`job::Recorder` writes the run's events:

- `Started`, the design's `Scene` and `Ports`;
- `Optimization`: the device, fidelity and model, the method with its paper, seed, budget and
  population, the variables (bounds, start, how each is differentiated), the objectives and
  constraints, the band, the corners and the derivatives' checks;
- an `Evaluation` per evaluation, in the optimizer's order: the variables, each objective's cost
  and value, each constraint's g, the violation, the first gradient's norm in the variables
  scaled to their ranges, the best so far (feasible first, then the lowest cost) and the models
  built;
- with a continuation, a `Stage` as each stage begins: its parameter's value, what the optimizer
  keeps and the evaluations before it; the best so far is then of that stage's evaluations;
- for two objectives, a `Front` after each generation: the non-dominated feasible evaluations
  and their hypervolume up to a reference point fixed at the first front (the largest costs then,
  a tenth of their range beyond);
- `Optimum`: the best evaluation and its design's parameters (what Apply takes into the design),
  or the front, and why it stopped; then `Finished`.

A seed gives the same record, to the bit, on any number of threads: genoxide's methods do
(their own guarantee), the devices' models do, and each generation's points are recorded in
the optimizer's order whatever the threads computed them in.

## A chip of devices

A job's `[circuit]` table is a chip (`photonoxide::job::ChipDesign`): instances, each a device's
design at its fidelity (a `[device]` table) or one of the circuit's own components (an ideal
coupler, a phase shifter, a terminator), the connections between their ports, and the external
ports. Each device is built at its fidelity and placed with the values its parameters give its
component, so a chip of devices at their defaults is the studio's chip of the library's kinds,
to the bit. A `"spectrum"` task records each instance's model with its fidelity and grid
(`Chip`), then the chip's S-parameters between its ports; a `"sweep"` or `"corners"` task steps
any instances' parameters, named `instance.parameter` (`ring.gap`, `split.coupling`), each point
the bits of its own chip's spectrum; an `"optimize"` task optimizes them.

**A chip's objectives** are expressions of its outputs: `T_q_p`, the power from its port p into
its port q, and `lambda`, reduced over the band and corners as a device's are (worst, average,
spread), so a loss in dB is `-10*log10(T_out_in)`. An instance's own objectives and constraints
(`instance = "ring"`, a declared `name` or an `expression` of its device's outputs: a smallest
gap, a footprint) are taken on that device alone.

**Derivatives.** At each wavelength the circuit adjoint gives ∂S/∂θ for every parameter θ of
every instance, one transposed solve with a right-hand side per external port
(`Circuit::jacobian`, the method of [circuit-adjoint.md](circuit-adjoint.md)); a component's
value is its own θ, and a device's parameter x moves its component's θ by
∂θ/∂x = Σ_k (∂θ/∂p_k)(∂p_k/∂x), the device's exact derivatives (`Built::derivatives`) through its
design's expressions (`Design::sensitivities`). The expression's derivative follows by forward
mode, ∂T_qp/∂x = 2 Re(conj(S_qp) ∂S_qp/∂x). A parameter its device doesn't differentiate (a gap,
a bend's radius: each builds another component) is differenced, and every exact derivative is
checked against fourth-order differences before use, as a device's. `circuit::objective` has the
same objectives for `Circuit::gradient`: `power_db`, and `worst` and `average` over wavelengths
of any one-matrix objective; the validation case `chip/adjoint` checks the job's gradient of a
worst loss in dB against them.

**A co-design.** `examples/optimize_chip_rings.rs` runs the chip job `jobs/chip-little-rings.toml`:
two rings, each two lossless half-circle bends between directional couplers (analytic), the rings'
coupler's gap and each ring's radius fitted by L-BFGS-B until the drop port is the Butterworth
shape over ±3κ_b² of round-trip phase. The rings' coupling comes out κ₁²/κ_b⁴ = 0.250023 at
κ_b² = 1e-3, Little et al. 1997's Table I printing 0.250 (doi:10.1109/50.588673); the circuit's
exact couplers differ from coupled-mode theory by O(κ_b²) there (`optimize_ring_flat` finds
0.25025 with the width free). Exact: no grid beyond the coupler's fit of its coupling.

**The circuit examples on the chip.** `circuit_splitter`, `circuit_ring_critical` and
`circuit_fit` drive genoxide on `Circuit::gradient` themselves and keep their evaluation counts;
three built-in chip jobs reach the same closed forms through the chip's `optimize` task, by the
program's L-BFGS-B at genoxide's default tolerances (the studio's tests check each, its record
the same bits on 1 and 4 threads and read back from its run). All exact: no grid beyond the
strip's analytic model.

| Job | What | Closed form | Found |
|---|---|---|---|
| `jobs/chip-splitter.toml` | an MZI's phase to 30% in the bar port (Clements et al. 2016, Eq. 1) | θ = 2 arcsin √0.3 = 1.15927948 rad | 1.15927946 rad, 8 evaluations |
| `jobs/chip-ring-critical.toml` | a ring's κ² and radius, its through port dark at 1.55 µm, 3 dB/cm (Bogaerts et al. 2011, Eqs. 2, 3, 12) | R = mλ/(2π n_eff) = 9.988436283 µm (m = 99), κ² = 1 − a² = 0.004325868 | to 2e-12, 45 evaluations |
| `jobs/chip-fit.toml` | an add-drop ring's κ₁², κ₂², loss and radius fitted to its through and drop spectra (Eqs. 5 and 6) at 81 wavelengths | 0.08, 0.05, 20 dB/cm, 10 µm | to 1.2e-9, 7e-10, 2.6e-7 dB/cm and 5e-14 µm, 32 evaluations |

The ring's n_eff is the 500 × 220 nm strip's analytic model's (2.44506), not the example's stated
2.4, and the fit's spectra are written out in its objective on that model's dispersion. The fit's
misfit is scaled by 1e6, as L-BFGS-B's relative-decrease test is taken against 1 for a cost
below 1, and the ring's radius is bounded within 0.02 µm of its resonance.

## A method study

Which of genoxide's methods to reach for, on a problem with cheap evaluations and an exact
gradient? A study on Bennet et al.'s Bragg mirror (J. Opt. Soc. Am. B 41, A126 (2024),
doi:10.1364/JOSAB.506389, with the problem as their published code has it, Zenodo
doi:10.5281/zenodo.10246032), in the spirit of their Fig. 6 and of Schneider et al.'s benchmark
(ACS Photonics 6, 2726 (2019), doi:10.1021/acsphotonics.9b00706). Schneider's problems are 3D
finite-element models, and the high-dimensional one of their Supporting Information isn't in
hand, so the problem is Bennet's: 10 and 20 layers of n = 1.8 and 1.4 alternating from the
top, on 1.8, under air, at 600 nm and normal incidence, each thickness from 0 to 214.29 nm, the
cost 1 − R by transfer matrices (exact: no grid) and its exact gradient
(`Multilayer::reflection_derivatives`). The quarter-wave stack is the optimum, 1 − R = 0.164856
and 0.014479 in closed form, and no run of the study found a lower cost. A run reaches the
target when its best comes within 10⁻⁶ of it. 31 seeds per method, as Bennet's runs; budgets as
the `optimize_bragg` example's, 5000 and 20 000 evaluations, and 300 for Bayesian optimization,
whose model costs grow with the evaluations.

**10 layers** (1 − R = 0.164856):

| Method | Budget | Runs reaching the target | Evaluations to it: median (quartiles) | Best 1 − R above the optimum: best, median, worst run |
|---|---|---|---|---|
| Bayesian optimization (genoxide's defaults) | 300 | 0 of 31 | – | 1.2e-6, 1.7e-2, 9.4e-2 |
| DE (current-to-best/1, F 0.8, CR 0.5, 30 individuals: Bennet's) | 5000 | 0 of 31 | – | 1.8e-5, 1.1e-4, 1.0e-3 |
| CMA-ES (genoxide's defaults, from the middle of the box) | 5000 | 24 of 31 | 1175 (1113–1245) | below 1e-9, below 1e-9, 1.7e-2 |
| L-BFGS-B on the exact gradient, restarted from random points | 5000 | 31 of 31 | 104 (52–234) | below 1e-9, below 1e-9, below 1e-9 |

**20 layers** (1 − R = 0.014479):

| Method | Budget | Runs reaching the target | Evaluations to it: median (quartiles) | Best 1 − R above the optimum: best, median, worst run |
|---|---|---|---|---|
| Bayesian optimization | 300 | 0 of 31 | – | 1.0e-2, 3.5e-2, 9.3e-2 |
| DE | 20 000 | 0 of 31 | – | 1.5e-6, 1.5e-5, 1.6e-3 |
| CMA-ES | 20 000 | 18 of 31 | 2350 (2219–2472) | below 1e-9, below 1e-9, 9.3e-3 |
| L-BFGS-B, restarted | 20 000 | 31 of 31 | 896 (332–2209) | below 1e-9, below 1e-9, 1.3e-9 |

What it says, for this problem:

- **With an exact gradient, a local method restarted is hard to beat.** Every L-BFGS-B run found
  the quarter-wave stack, in a median of 104 evaluations for 10 layers and 896 for 20, each
  evaluation the stack's reflectance and its gradient at once. The landscape has local minima,
  so some runs needed several starts, but each local run is cheap.
- **CMA-ES is fast when it finds the basin, and inconsistent.** Most of its runs reached the
  optimum in about 1200 and 2400 evaluations; a quarter to two fifths settled in other minima,
  1e-2 above. Bennet et al. describe the same poorer consistency of CMA-ES (their Fig. 6, from a
  plot); unlike theirs, its median here is better than DE's.
- **DE with Bennet's settings comes close and stays close.** No run reached 10⁻⁶ within the
  budget, but its worst runs are better than CMA-ES's worst: its median ends 1e-4 and 1.5e-5
  above the optimum. Bennet et al. follow DE with a local method (their QNDE) for that reason, as
  `optimize_bragg` does with L-BFGS-B.
- **Bayesian optimization is for expensive evaluations, not these.** With 300 evaluations it
  came within 1.2e-6 once for 10 layers; in 20 dimensions its model can't resolve the optimum in
  that budget. Its own cost made the study's runtime (about half an hour on 20 threads, most of
  it the 62 Bayesian runs); the transfer matrices take microseconds.

Run with `cargo test --release --test method_study -- --ignored --nocapture`, which prints the
tables. Every run is seeded and runs on its own thread, so the tables are the same on any
number of threads; a small study in the same file checks that on 1 and 4 threads in CI.

## Validation

| Case | What | Grid | Result |
|---|---|---|---|
| `optimize/derivatives` | every exact derivative of every device's declared objectives and constraints at its cheapest model, against fourth-order differences | its model's | ≤ 1e-6 relative |
| `optimize/coupler-3db` | the coupler's split at L_x/2 against 1/2, coupled-mode theory's closed form | exact: no grid | ≤ 1e-14 |
| `optimize/threads` | an evaluation with three variables' gradient on 1 and 20 threads | exact: no grid | 0 bits differ |
| `optimize/hypervolume` | a front's hypervolume against one computed by hand | — | 6 |
| `chip/mzi-closed-form` | an MZI of directional-coupler and waveguide devices against the product of its devices' transfer matrices | exact: no grid | ≤ 1e-13 |
| `chip/derivatives` | a chip's exact derivatives through its devices against fourth-order differences | exact: no grid | ≤ 1e-6 relative |
| `chip/adjoint` | a chip job's gradient of a worst loss in dB against `Circuit::gradient` of `objective::worst` of `objective::power_db` | exact: no grid | ≤ 1e-9 relative |
| `chip/study-points` | a chip's sweep against each point's own spectrum | exact: no grid | 0 bits differ |
| `chip/threads` | a chip's evaluation with three variables' gradient on 1 and 20 threads | exact: no grid | 0 bits differ |

The library's tests also run genoxide's L-BFGS-B (a dev-dependency) on the coupler: it reaches
the 3 dB length L_x/2 to 10⁻⁶ µm. The `photonoxide` program's tests run each method through the
job, a seeded run's record the same on 1 and 4 threads, the hard limit, and the convergence of
value-only methods with their budget.

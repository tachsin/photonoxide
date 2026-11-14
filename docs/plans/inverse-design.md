# Inverse design (0.7): a plan

*A plan, 2026-11-09, for ROADMAP's 0.7, tracked in #761. It follows the optimization survey
([optimization.md](optimization.md), #244), whose phases 1 to 8 and 10 are built, and
[quantum.md](quantum.md)'s Q4, which belongs here but comes after the classical machinery. The
decisions in [section 4](#4-decisions) were taken on 2026-11-10: I decided the recommended option
of each of the eight, and the phases follow them. The Academy lesson [inverse-design.md](../../academy/inverse-design.md) is written last,
once the milestone is built. Where the work stands and how to pick it up:
[section 6](#6-where-the-work-stands).*

The rule of [AGENTS.md](../../AGENTS.md) holds throughout: optimizers come from
[genoxide](https://github.com/tachsin/genoxide). photonoxide supplies the objectives, their
gradients, the parametrizations and the constraints; a method genoxide lacks is proposed there as
a general method. The library keeps no genoxide dependency (#244's decision 1): the examples and
the studio run genoxide's methods on what the library computes.

## 1. What the repository already has

**Adjoint gradients, each checked against fourth-order differences:**

| Solver | What | Validation |
|---|---|---|
| Circuits | `Circuit::gradient` and `Circuit::jacobian`, exact for any netlist ([circuit-adjoint.md](../methods/circuit-adjoint.md)) | the circuit adjoint's cases |
| 2D FDFD | `Solver2d::mode_power_gradient`: a port mode's power against every cell's permittivity (its real part), E or H along z, for a problem given cell by cell ([fdfd-adjoint.md](../methods/fdfd-adjoint.md)) | `fdfd/adjoint-gradient-ez`, `-hz` |
| 2D FDFD, shapes | `Solver2d::mode_power_derivative`: dF/dp for one geometric parameter, through the fill fractions' smooth averaging (dA/dp by two assemblies, no factorization; #371) | unit tests |
| 3D FDFD | `Solver3d::mode_power_gradient` (direct) and `IterativeSolver3d::mode_power_gradient(_recycled)` | `fdfd3d/adjoint-gradient`, `fdfd3d/recycle-adjoint` |
| 3D FDTD | `fdtd::Design` (a box of cells, one density each, ε linear in it) and `Simulation::gradient`, from two runs' transforms, modes, fluxes and transforms over several frequencies ([fdtd-adjoint.md](../methods/fdtd-adjoint.md)) | seven `fdtd/adjoint-*` cases |
| Transfer matrices | `Multilayer::reflection_derivatives`: ∂R/∂d and ∂R/∂n, exact | the multilayer's cases |
| Devices | `Problem::evaluate`: exact derivatives where the components give them, central differences elsewhere, every exact one checked by `Problem::check_gradients` before use ([optimization.md](../methods/optimization.md)) | `job/optimize-*` |

**No adjoint yet:** the mode solvers (no dn_eff/dp), eigenmode expansion, RCWA, the nonlinear
solvers, the thermal and electrostatic solvers, and the quantum statistics. (I9 added the mode
solvers', RCWA's and eigenmode expansion's derivatives and a Kerr FDFD with its adjoint: section 3.)

**Parametrizations:** a device's parameters with units and bounds (`Component::parameters`),
layer thicknesses, a spline's widths along a path (the Y-branch's taper), Haar-random mesh phases,
and FDTD's density per cell (linear, with no filter and no projection). `geometry::implicit` traces
the zero set of any function, but nothing optimizes one. **Missing:** density filters,
projections, robust (eroded and dilated) designs, length-scale constraints and metrics, level
sets, subpixel-smoothed projection, lithography models inside a loop.

**The optimize job and the studio** (#244, #338's P8): a `[device]` job's `[task]` of kind
`"optimize"` with genoxide's L-BFGS-B, Adam, MMA, GCMMA, Nelder-Mead, CMA-ES, DE, PSO, NSGA-II and
SMS-EMOA; bands and corners reduced to the worst or the mean; continuation that keeps the
optimizer's state; every evaluation recorded and replayed bit for bit; the Optimize tab with the
convergence plot, the parameters' traces and the Pareto front; a chip of devices optimized
together through the circuit adjoint (#346).

**The examples** (all with genoxide 0.13.1, the newest on crates.io on 2026-11-09):
`optimize_bragg`, `optimize_ring_flat`, `optimize_mzi_lattice`, `optimize_ring_mzi`,
`optimize_ring_pareto`, `optimize_mesh`, `optimize_mesh_selfconfig`, `optimize_chip_rings`,
`optimize_ybranch` (PSO in 2D, checked in 3D FDFD: 0.202 ± 0.027 dB against Zhang et al.'s
0.13 dB, a gap in #595; the adjoint half waits for shape derivatives) and
`optimize_grating_pareto` (39.77 % and 43.64 % against Passoni et al.'s 47 % and 53 %, a gap in
#595). #244's phase 9, the topology examples, waits for this plan.

## 2. What 0.7 promises

ROADMAP's 0.7 section: quantum objectives with gradients; adjoint gradients for every solver
(Lalau-Keraly 2013; nonlinear: Hughes 2018) and forward mode where it pays (Hughes 2019); density
topology optimization with filtering, projection and continuation, and robust designs (Wang
2010; Jensen 2010; Christiansen 2021); minimum length scales (Zhou 2015; Hammond 2021), strict
foundry rules (Schubert 2022) and level sets (Vercruysse 2019); subpixel-smoothed projection
(Hammond 2025) and hyperparameter-free length-scale constraints (Arrieta 2026); shape and
parametric optimization with lithography models (Khan 2024); the a-posteriori length-scale metric
for every design (Chen 2024); device and circuit co-design (Mason 2025); genoxide's optimizers;
explore in 2D, verify in 3D; the performance plan's Phase D; the studio's live dashboard; and
validation in 3D against Chen 2024's suite, Khan 2024, Piggott 2015, Shen 2015, Frellsen 2016,
Su 2018 and Piggott 2020.

## 3. The phases

In this order: each builds what the next needs. Every phase has a methods page, validation cases
with a Rule each (a measure and a basis, never a hand-picked number), and its papers read before
it starts; a phase that can't meet its check says so, with its numbers, and leaves the gap in
#595. The work is one branch and one pull request at the end (`feat/inverse-design-0.7`).

### I1: Densities, filters and projections

A solver-independent module (decision 1) for a design region of n_x × n_y (× n_z) pixels:

- the **density** ρ ∈ [0, 1] per pixel, mapped to a permittivity ε(ρ̂) = ε_min + ρ̂(ε_max − ε_min);
- the **conic filter** of radius R, ρ̃ = Σ w ρ / Σ w with w = max(0, R − |x − x'|) (Bourdin 2001,
  as Christiansen & Sigmund 2021 and Wang et al. 2010 restate it), with the region's edges
  either cut off (the weights renormalized) or padded with a fixed solid or void;
- the **projection** ρ̂ = (tanh βη + tanh β(ρ̃ − η)) / (tanh βη + tanh β(1 − η)) (Wang et al. 2010),
  with its derivatives in ρ̃;
- the **chain**: the gradient a solver gives with respect to ε pulled back through the projection
  and the filter's transpose, for 2D FDFD (`Solver2d::from_cells`) and FDTD's `Design`.

*Validation* (five cases, drafted on the branch, section 6):

- `topology/projection-derivative`: the derivative against the complex step, Im ρ̂(x + ih)/h,
  which takes no difference, at 201 points for β = 1, 8, 64, 256 and η = ¼, ½, ¾ (round-off);
- `topology/projection-identities`: ρ̂(0) = 0, ρ̂(1) = 1, and ρ̂(1 − x) = 1 − ρ̂(x) at η = ½
  (round-off);
- `topology/filter-transpose`: ⟨Fx, y⟩ = ⟨x, Fᵀy⟩ on 160 × 160 and 24 × 20 × 16 pixels, cut off
  and padded at the edges (round-off);
- `topology/filter-preserves-linear`: a uniform density kept everywhere, a linear ramp kept a
  radius from the edges (the weights are normalized and symmetric; round-off);
- `topology/fdfd-gradient`: the whole chain's gradient (a 10 × 10 region's densities beside a
  220 nm slab to a port mode's power, 2D FDFD, E along z, 20 nm) against fourth-order differences
  of the densities, within what the differences resolve, ruled as `fdfd/adjoint-gradient-ez` is.
  Its δ comes from a step study (an ignored test, `the_chain_step_study`) not yet run.

*Papers:* Wang, Lazarov & Sigmund 2010 (read for this phase: Eqs. 4, 5, 7 and 9, Section 5.1's
thresholds, Section 6's length scale) and Christiansen & Sigmund 2021 (read: Eqs. 17 and 18). The
Helmholtz filter of Lazarov & Sigmund 2010, Christiansen & Sigmund's Eq. 17, waits for I4's
demultiplexer, which uses it; on a regular grid with Neumann edges a cosine transform
diagonalizes it, so it needs no sparse solver.

### I2: Robust designs and min-max

- **Eroded, nominal and dilated designs** from one density by three thresholds, 1 − η, ½ and η
  (Wang et al. 2010, Section 5.1 and Eq. 14; Sigmund 2009), and the relation between a threshold
  and the edge's displacement. In 1D (the hat kernel R − |x|) it is exact: a solid bar that the
  eroded threshold η_e just removes has width h = 2R(1 − √(1 − η_e)), and the nominal design keeps
  a width b with b/R = 2 − 2√(1 − η_e) for η_e ≥ ¾ and 2√(η_e − ½) below: Qian & Sigmund 2013's
  Eq. 29 (Appendix). *Settled 2026-11-11:* Wang et al.'s Fig. 12 and their 1.1055 at η = 0.2 are
  b/R, though the axis reads b/2R; Qian & Sigmund's Table 1 and Fig. 19 read Wang et al.'s curve
  as b/R, and the bars of `topology/robust-length` agree with Eq. 29 as b/R. (Wang et al.'s
  worked numbers, b = 0.092L and R = 8.4L/200, give b/2R = 1.095, not their 1.1055.) On a 2D
  pixel grid the cone's marginal across a straight edge is m(x) = aR − x² ln((R + a)/|x|),
  a = √(R² − x²), not the hat; its integral is the closed form of `topology::across_edge_2d`,
  and the discrete filter's values are checked against it at second order in the pixel.
- **The epigraph form** of min-max over wavelengths, corners and the three designs: the variables
  (x, t), the objective t, the constraints f_k(x) − t ≤ 0 with their Jacobians, in the form
  genoxide's `Mma` and `Constrained` take.

*Validation:* the edge displacement against the closed form on a straight edge, the pixels' own
error stated (discretisation, with its order); the epigraph's Jacobian against differences.
*Papers:* Sigmund 2009, Wang 2010, Christiansen & Sigmund 2021, Qian & Sigmund 2013 (in hand;
read for this phase: Qian & Sigmund's Appendix and Table 1, Wang et al.'s Section 6 and Fig. 12).
*Done 2026-11-11:* `topology::robust` and four cases (the methods page, topology.md).

### I3: Length scales: the metric and the constraints

- **Chen et al. 2024's a-posteriori metric:** the smallest of the solid's and the void's minimum
  length scales, each the largest disk diameter whose morphological opening leaves the pattern
  unchanged (beyond the boundary's pixels), found by bisection, as the testbed's `imageruler`
  does. Reported for every design the examples produce.
- **Constraints** on the filtered and projected design: Zhou et al. 2015's geometric
  constraints with Hammond et al. 2021's foundry forms (linewidth, spacing), their values and
  Jacobians for MMA; Arrieta et al. 2026's hyperparameters from the length scale alone.

*Validation:* the metric on shapes whose length scale is known in closed form (a bar of width w,
a ring, a gap), within a pixel; the metric on the testbed's published mode-converter designs
against the length scales its `run.py` prints (decision 2); the constraints' gradients against
differences; a constraint satisfied exactly by a pattern built to its length scale. *Papers:*
Chen 2024 and Hammond 2021 (in hand); Zhou 2015 (in hand); Arrieta 2026 (to get, section 5).
*Done 2026-11-11, the metric:* `topology::ruler`, written from Chen et al.'s Section 2 with the
discretisation of imageruler's 2023 version (MIT), which the testbed's README used on 2023-05-11
(`minimum_length` on ceviche-challenges' whole domain, 350 × 300 at 10 nm, not the 160 × 160
design alone; the testbed's `run.py` moved to imageruler's integer rewrite in April 2024, after
its README's numbers). Eight of the circle-brush designs are in `src/cases/topology/testbed/`
(MIT, commit a06518872de6) and reproduce the printed lengths bit for bit (`topology/ruler-testbed`);
shapes of known width within a pixel (`topology/ruler-shapes`). *Done 2026-11-11, the
constraints:* `topology::LengthConstraints` (Zhou et al.'s Eqs. 8–11, c = (R/Δ)⁴) with Hammond et
al.'s thresholds (Eqs. 14 and 15, checked against Eq. 29's inverse to round-off) and the gradients
against differences (3.9e-11). Zhou's c = r⁴ is in pixels; Hammond et al. write r as the grid's
resolution, a different scale, so c is a setting (`with_decay`). *Done 2026-11-11, the
hyperparameters:* Arrieta et al. 2026's (the arXiv version, 2507.16108v2): `with_arrieta_decay`
(c = 64 R² in µm, 64 (R/Δ)² per pixel) and `geometric_threshold` (their Eq. 18, integrated by parts
here), the pixels' sum at the critical strip against Eq. 18 to 0.21 % at R/Δ = 200, second order
(`topology/constraint-arrieta`). I3 is done.

### I4: 2D topology optimization, the published devices (#244's phase 9)

- `optimize_mode_converter` (Chen 2024; Schubert 2022): the testbed's 2D mode converter, 1.6 ×
  1.6 µm, ε 12.25/2.25, E along z, the fundamental mode in and the second out, six wavelengths
  over 1265–1295 nm (from the testbed's 1260–1300 nm band). First the published designs
  re-evaluated on our 2D FDFD at 10 nm and on finer grids: worst reflection and transmission
  against the run.py numbers (for the 50 nm design −33.33 dB and −0.07 dB at a measured 55.26 nm).
  Then our own design by MMA with the epigraph and continuation of β from 8 to 256, with a 50 nm
  length-scale constraint, its metric reported.
- `optimize_demux_robust` (Christiansen & Sigmund 2021, Table VII): the robust 2D demultiplexer,
  eroded, nominal and dilated at ±8 nm.
- `optimize_foundry_bend` (Hammond 2021): the bend with linewidth and spacing constraints, the
  epigraph over ten frequencies.

*Validation:* the published designs' numbers within their printed digits with our grid's error
beside them (the rule's third-of-the-tolerance test decides whether each is a case or a reported
gap); our designs against the papers' stated targets. FEM and Meep are other discretizations, so
each comparison carries our own grid study.

*In progress 2026-11-11, the published designs re-evaluated* (`the_testbed_spectrum_study` and
`the_testbed_grid_study`, ignored tests in `topology::tests`; `checks::testbed_spectrum_at`): the
eight circle-brush designs of `src/cases/topology/testbed/` in the model's domain on our 2D FDFD
(E along z, 20 cells of PML, ports as 1.9 µm windows at columns 25 and 325, the fundamental mode
in and the second out), worst over the six wavelengths, against the testbed README's numbers
(Ceviche, also at 10 nm):

| Design | R, ours at 10 nm (dB) | R, printed | T, ours (dB) | T, printed |
|---|---|---|---|---|
| circle_10 w11 | −22.61 | −22.50 | −0.187 | −0.19 |
| circle_10 w27 | −26.90 | −27.17 | −0.186 | −0.19 |
| circle_12 w12 | −28.32 | −28.49 | −0.347 | −0.35 |
| circle_14 w13 | −34.24 | −33.62 | −0.188 | −0.19 |
| circle_16 w14 | −21.07 | −21.23 | −0.812 | −0.81 |
| circle_18 w15 | −23.46 | −23.66 | −0.928 | −0.93 |
| circle_20 w16 | −18.47 | −18.43 | −1.232 | −1.23 |
| circle_20 w40 | −18.18 | −18.16 | −1.344 | −1.34 |

Every transmission rounds to the printed digits; the reflections differ by 0.02 to 0.62 dB. Our
own grid moves them as much: from 10 to 5 nm (each pixel 2 × 2 cells) the first three designs'
R changes by +0.04, +0.17 and +0.54 dB and T by −0.004, −0.005 and +0.009 dB. Against the
printed digits (half a unit, 0.005 dB for T) the rule's third-of-the-tolerance test fails on our
10 nm grid's own change (up to 0.009 dB), though both codes ran the same 10 nm pixels; the deep
reflections are grid-limited in both. *Decided 2026-11-10 (decision 7): no validation case; the
gap is kept open in #595.* The numbers are reported here and in `optimize_mode_converter`'s
output, and nothing is loosened.

*Done 2026-11-11, `optimize_mode_converter`:* the published designs as above, their
transmissions checked to the printed digits and their reflections printed beside the grid study;
then our design: the conic filter of 50 nm, β 8 to 256 in six stages of 30 MMA evaluations on the
epigraph (twelve constraints), then Zhou et al.'s linewidth and spacing with Arrieta et al.'s
hyperparameters for 60 more, values and gradients rounded to 12 digits so that `--full` repeats
the recorded design. A uniform start is stuck: a mirror-symmetric device can't convert the even
mode into the odd one, and the gradient there is zero, so the start breaks the symmetry. Found:
−32.89 dB and −0.154 dB at a measured 43.6 nm (on the model's whole domain, as the testbed
measures), both constraints met (g/ε 0.83 and 0.99): within Chen et al.'s 20 % of the imposed
length and within the published designs' range at 55 to 76 nm, checked; 13 % short of 50 nm,
said. The plain run measures the recorded design in 17 s; `--full` optimizes it again (6.5 min).
*Done 2026-11-11, `optimize_demux_robust`:* the Helmholtz filter by the cosine transform
(`topology::HelmholtzFilter`, exact on mirrored edges) and Christiansen & Sigmund's double
filtering (`DoubleFiltered`), three cases (`topology/helmholtz-*`, `double-filter-gradient`); the
demultiplexer of their Fig. 7 and Tables 4–6 on our 2D FDFD (PMLs, the input's mode launched and
each output's mode measured, a real permittivity: their absorbing boundaries, Gaussian beam and
damping term differ, said), MMA on the worst of the six cases, β (5, 2.5) to (160, 80) in six
stages of 80. All six reach 0.980–0.986 of the power entering, against Table 7's 0.69–0.86,
checked as targets met; crosstalk at most 0.14 %, the outputs' flux consistent, 5 nm within
0.002. The d_WG of Fig. 7 are taken to the waveguides' centres (the figure doesn't settle it).
*Done 2026-11-12, `optimize_foundry_bend`:* Hammond et al.'s minimum area and enclosed area
(`topology::AreaConstraints`, `topology/area-shapes`) and, since the bend's output runs along y,
FDFD's adjoint for any linear functional and a mode measured on rows
(`functional_power_gradient`, `row_mode_functional`, `fdfd/adjoint-gradient-row`). The bend under
Fig. 8's rulebook (a 0.5 µm guide and E along z, which the paper doesn't state): transmission
into the fundamental mode 0.998 to 0.999 over 1.50–1.60 µm, a measured 110 nm, no small island or
hole; above every rulebook's 89 % of Fig. 9. Each stage keeps its own best design, ranked: one
that meets every rule first, then the least worst objective, then the least violation (MMA's
iterates sit just outside the epigraph, and genoxide's best feasible genome stayed the start);
the area constraints are counted in pixels for MMA's scaling; the filter is 135 nm, 1.5 times the
rules (at 90 nm the unconstrained design's 58 nm features never met the rules in 300 evaluations).
*Corrected 2026-11-12, with I5:* the first record measured the output on a window shifted by
1 µm across the guide (the turned grid's rows placed by y's origin, not x's), half the guide and
a wall at its centre, which read about the power in all three of the guide's modes (0.96) where
the fundamental's was 0.35; the topology job's row ports found it (a bend and its mirror image
agree to 10⁻⁹, and the modes' powers sum to what the bend passes). *I4 is done.*

### I5: The topology job and the studio's design view

- A density design region in the `optimize` job: the region, its materials, the filter, the
  projection and its continuation, the robust thresholds, the length-scale constraint, the
  objective's ports and band; checked by `job::check`, in the schema, with Python bindings per the
  repository's conventions.
- Records: every evaluation's figure of merit, constraints, β, and the design at a stride.
- **The studio:** on the device page's Optimize tab (decision 8), the design region's density as
  it changes, the field at the last evaluation, the figure of merit and the constraints over the
  run, the length-scale metric of the best design.

*Validation:* a run replays bit for bit and gives the same bits on 1 and 20 threads; the job's
gradient against the library's; the studio's tests.

*Done 2026-11-12:* an `"optimize"` task with a `[task.design]` (`job::Topology`,
docs/methods/topology.md, "The job"): the window and cell, the core's and cladding's indices,
guides as rectangles, west and east ports on columns and south and north ones on rows, the source,
objectives (a mode's power at a port, the source's own for a reflection) over their wavelengths,
the conic filter, β per stage with each stage's evaluations, the robust threshold, the linewidth,
spacing and areas from a chosen stage. It shares the solver-alone `"optimize"` kind with the
crosstalk correction, told apart by its table, in the schema as one of the two. The library reads,
evaluates and records it (`Topology`, `TopologyStage::evaluate`, `TopologyRecorder`); the program
runs genoxide's MMA on it a stage at a time. Records: the problem, every evaluation's t, worst
term, terms, rules and the stage's best, the physical density and |E_z|² at a stride and at each
stage's end, and the binary design at the end with its spectrum, its measured minimum length and
its small islands and holes. The run's window shows them live (`TopologyResults`): the design
(any recorded one by a slider), the field, the worst term and the rules over the run, the spectra.
Tests: each term's gradient and the rules' rows against fourth-order differences, a bend to the
north and its mirror image to the south the same to 10⁻⁹, a strip through the region T within 10⁻⁴
of 1, the record's events, a stage's best ranked, and in the program a run the same bits on 1 and
4 threads that improves on its start. `jobs/topology-bend.yaml` runs in seconds. The view is in the
run's window, where `photonoxide run` shows a job, rather than on a device page's Optimize tab: a
design region isn't one of a device's parameters; when a device gains one (I11), its Optimize tab
takes the same view. Python checks the job (`check_job`) and, as for every `"optimize"` job, the
program runs it. *I5 is done.*

### I6: Subpixel-smoothed projection

Hammond et al. 2025's projection, which keeps the design differentiable at β = ∞ and passes from
a density to a level set: the filtered field's value and gradient at a pixel give the fill of the
pixel by the interface it implies.

*Validation:* a straight and a curved interface whose fill fraction per pixel is exact, at the
order the paper states; the gradient against differences at large and infinite β; their
convergence comparison against the standard projection on one of their problems (reproduced on
the mode converter). *Papers:* Hammond 2025 (Opt. Express 33, 33620; in hand, read 2026-11-12).

*Done 2026-11-12:* `topology::SmoothedProjection` and `Density::with_smoothed_projection` (their
Eqs. 7, 10–12 with R̂ = 0.55 Δx, β up to ∞, the gradient through each pixel's value and its
differences; docs/methods/topology.md, "The subpixel-smoothed projection"). Cases:
`topology/ssp-plane` (F₂ against the parabolic kernel's share of a half-space by quadrature, and
the projection of a linear field against F₂ of the exact distance, 1.0e-11), `topology/ssp-disc`
(each pixel near a circle against the exact smoothed disc: first order in R̂/a, 0.078 R̂/a from 20
to 2.5 nm, the planar fill's own error on a curve) and `topology/ssp-gradient` (against
differences at β = 64 and ∞, 1.6e-10). The comparison is on their own crossing rather than the
mode converter, since it is the problem their Figs. 2, 5 and 6 report:
`optimize_crossing_ssp` reaches their 99 % by topology optimization to β = ∞ (0.9993) and by shape
optimization at β = ∞ from a plain cross (0.9952); the standard projection's gradient falls to
8.6e-29 at β = 1024 while the smoothed one's settles at its β = ∞ value (Fig. 5b); from the plain
cross the smoothed projection reaches 99 % in 23 to 28 evaluations at every β, the standard one at
β = 16 only within 60 (Fig. 2). The robust realizations and the length constraints keep the
threshold projection; the topology job too, since a run's record (JSON) has no number for β = ∞.
*I6 is done.*

### I7: Level sets and shape derivatives

- A **level set** on the design grid, the interface traced (`geometry::implicit`) and averaged
  into the cells by the fill fractions, so the matrix is smooth in the level-set values; the
  gradient with respect to many shape parameters from one adjoint solve (generalizing
  `mode_power_derivative`'s one parameter: each parameter costs an assembly, no factorization).
- **Vercruysse et al. 2019's analytical constraints** on curvature and gap.
- **The Y-branch's adjoint half** in `optimize_ybranch`: Lalau-Keraly et al. 2013's −0.07 dB in
  102 solves, the taper's widths by L-BFGS-B on the adjoint shape derivatives.

*Validation:* the shape gradient against differences; a circle's level-set shape derivative
against the analytic dF/dR of a field identity; the Y-branch against Lalau-Keraly's 2D and 3D
numbers. *Papers:* Lalau-Keraly 2013 (read), Piggott 2017 (in hand); Vercruysse 2019 (Sci. Rep. 9, 8999; in hand, read 2026-11-12).

*Done 2026-11-12:* `topology::LevelSet` (φ at the cells' corners; each cell's fill from four
triangles about its centre, exact for a linear φ and continuously differentiable as a corner
changes sign; docs/methods/topology.md, "Level sets"). The shape derivative is taken on the cells
rather than by an assembly per parameter as planned: the solver's ∂F/∂ε at every cell from one
adjoint solve, through each fill's derivatives in its corners, so a node costs nothing beyond its
four cells (`mode_power_derivative` stays for a geometry drawn as shapes). `LevelSetPenalty`,
Vercruysse et al.'s Eqs. 1–3, with the arctan's argument in units of π/d (theirs takes a length).
Cases: `topology/level-set-plane` (the fill exact for a linear φ, 1e-16), `-circle` (dA/dR of a
circle drawn as a cone against 2πR, within 0.3 (h/R)²; the identity the plan asked for, every
node moving with R), `-gradient` (2D FDFD against differences, within their resolution),
`topology/penalty-gap` (the switch at a grating's period 2d, second order), `-curvature` (at a
disc's radius d/2) and `-gradient`. The Y-branch's adjoint half in `optimize_ybranch`: L-BFGS-B
on the taper's widths from Table 1's, the derivatives from the swarm's 2D model's adjoint and
the cells' exact areas (within 1.8e-4 of differences), 0.039 dB in the device's 2D model after
their 102 solves against the swarm's 0.028 dB after 1500 evaluations. Its 3D loss, to go beside
their −0.07 dB (a level set's shape, its last ten iterations in 3D), is `--full`'s, run
2026-11-12 (`RECORDED_3D_ADJOINT`): 0.474312, 0.477102 and 0.479050 into each arm on the 44,
31.4 and 22 nm grids, extrapolated 0.169 ± 0.024 dB, against the swarm's 0.202 ± 0.027 dB and
Table 1's 0.168 dB; 0.10 dB above theirs, reported and not checked. *I7 is done.*

### I8: Lithography inside the loop

Khan et al. 2024's fabrication-aware shape optimization: a lithography proxy (a Gaussian blur and
a threshold, with the DUV and EBL parameters they state) applied to the design inside the loop,
its gradient through the blur's transpose. The Y-branch and the SWG-to-strip converter.

*Validation:* the proxy's blur on a straight edge in closed form; their Table 1's simulated
insertion losses (0.836 dB as designed, 0.111 dB designed with the model) within its printed
digits with our grid study. Measured numbers are context only. *Papers:* Khan 2024 (in hand; the
manuscript, not the typeset version).

*Read 2026-11-13:* Khan et al. state no parameter of either model: the DUV one is UBC's in Mentor
Graphics Calibre (their ref. 11, Lin et al. 2020), the EBL one PreFab's neural network (ref. 13,
Gostimirovic et al. 2022), and neither is open; the devices' dimensions aren't published ("Data
... not publicly available"). Table 1's numbers are 3D FDTD of their own devices at 1320 nm under
those models (and 0.836 dB is the standard design *printed*, 0.034 dB as designed). So Table 1
can't be reproduced here, at any digits: what it would need is listed in section 5 and the gap in
#595.

*Done 2026-11-13:* `topology::GaussianBlur` and `topology::Lithography` (docs/methods/topology.md,
"Lithography inside the loop"): a Gaussian blur, each pixel's weight the Gaussian's integral over
it so that a pixelated mask's blur is its exact convolution, renormalized or padded at the
region's edges; the threshold by the subpixel-smoothed projection at η = t, sharp by default;
the gradient through the threshold's and the blur's transposes (their Eq. 2); the closed forms of
a straight edge's shift, √2σ erfc⁻¹(2t), and an isolated line's printed width. Cases:
`topology/litho-blur` (a half-plane, a corner and a line against the exact convolution, 7.8e-16),
`-transpose` (1.8e-15), `-print-edge` (the printed edge against the closed form at t = 0.3, 0.5
and 0.7, 0.125 pixel, the smoothed threshold's own row error, 0.1217 pixel, plus the blur's
curvature) and `-gradient` (2D FDFD through the proxy against fourth-order differences, 9.2e-10,
below their resolution). The proxy's σ in the examples, 58.16 nm, is the widest that prints a
150 nm line within 1 % at t = ½ (Selvaraja et al. 2009, Section II-B and Fig. 6: 193 nm
lithography on target from 150 nm, and photonic circuits ask 1 %): a bound, not a calibration.
`optimize_ybranch` designs its taper on the printed model too (L-BFGS-B, 51 evaluations from
Table 1's): the standard design loses 0.0387 dB drawn and 0.0543 dB printed, the one designed
printed 0.0357 dB printed (0.0670 drawn). `optimize_swg_converter`, Khan et al.'s second device on
Halir et al. 2016's SWG (Table 1: 190 nm, 50 %, 0.5 µm, 30 periods) at 1.55 µm, two back to back,
the 30 gratings' widths and lengths and the bridge's edge (their Fig. 2): per converter the
standard design 0.0001 dB drawn and 0.0403 dB printed, the one designed printed 0.0017 dB printed
(10 nm beside: 0.0015, 0.0437, 0.0029). In both, the design made with the model prints better
than the standard one, Khan et al.'s test of a simulation, checked; Table 1 is printed beside as
context. Both examples' derivatives are checked against fourth-order differences of the power
along each variable's change of the cells' shares, within what they resolve. *Left:* Table 1's
comparison (above); the job and the studio take no lithography yet (the plan asks for none).

### I9: Adjoints for the other solvers

- **The mode solvers:** dn_eff/dp by the eigenproblem's own adjoint (Hellmann–Feynman for the
  Hermitian case), for dispersion objectives (and Q4's).
- **Eigenmode expansion:** the cascade's S-matrix derivative with respect to a section's
  parameter.
- **RCWA:** a diffraction efficiency against each layer's permittivity.
- **Nonlinear 2D FDFD** (Hughes et al. 2018): the Kerr problem's adjoint through its Newton or
  fixed-point solve.
- **Forward mode** (Hughes et al. 2019) where parameters are fewer than outputs (decision 6).

*Validation:* each against fourth-order differences within what they resolve, and a closed form
each: a slab's dn_eff/dw from the dispersion relation's implicit derivative; a uniform grating's
first-order efficiency derivative; Hughes 2018's own device numbers where the paper states them.
*Papers:* Hughes 2018 and Hughes 2019 (in hand, read 2026-11-14).

*Done 2026-11-14:* docs/methods/solver-derivatives.md. **The mode solvers:**
`SlabMode::derivatives` (the characteristic equation's implicit derivative in the thickness and
the three indices, closed form); `Profile::index_gradient` (every cell's permittivity and every
interior node's position, the left eigenvector y = Wx free since M = W⁻¹H) and
`VectorMode::index_gradient` (every cell, the left eigenvector by inverse iteration with Aᵀ), each
dM from the assembling expressions on dual numbers (`mode::dual`), so exact. **RCWA:**
`Grating::diffract_tangents`, forward mode through the Toeplitz matrices, the eigenpairs, the
interfaces and the star products, in a stripe's permittivity or a layer's thickness. **Eigenmode
expansion:** `Eme::s_matrix_tangent`, forward mode in a section's length, a layer's index or a
boundary's position, through the modes (dν from the Wronskian, the fields and their
normalization on duals), the overlaps (with Leibniz's rule where TM's E jumps), the interfaces
and the cascade. **FDFD:** `Solver2d::field_tangent` (forward mode, Hughes 2019) and `Kerr2d`
(Hughes 2018: Newton's method with u and ū independent, its R-linear steps and adjoint by GMRES
preconditioned with J's factors, χ′ for a nonlinearity that follows the density). Cases, each with
its Rule: `mode/slab-index-derivative` (3.0e-4 at 2.5 nm, second order, 2.0 to 2.3 (h/t)²),
`mode/profile-index-gradient` (7.3e-11), `mode/vector-index-gradient` (4.1e-11),
`mode/rcwa-tangent-multilayer` (against the transfer matrices' exact derivatives, round-off),
`mode/rcwa-tangent-gradient` (1.9e-9), `mode/rcwa-tangent-born` (a weak grating's first order
against the Born approximation, 0.240 Δ: the uniform grating's closed form read as the
weak-contrast limit), `eme/s-matrix-tangent` (5.4e-12), `fdfd/forward-mode` (against the adjoint,
7.6e-14), `fdfd/kerr-uniform` (a uniform Kerr medium against its scalar cubic, 8.9e-15) and
`fdfd/kerr-gradient` (through Newton's method, 4.3e-10). *Left:* Hughes et al.'s switches
(98.2 % and 3.1 %; 81.8 %/5.9 % and 6.1 %/80.8 %) can't be reproduced: the paper gives the
materials and powers but not the designs, and their optimization's settings are in the
Supporting Information, not in the folder (listed in the papers' README); kept in #595.

### I10: Quantum objectives (quantum.md's Q4)

Purity, heralding efficiency, HOM visibility, a unitary's fidelity and an output pattern's
probability, with gradients: the circuit adjoint for ∂S/∂θ, the minors' permanents for the
statistics (∂ Perm/∂U_ij is the permanent of the minor), I9's mode-solver derivatives for
dispersion.

*Validation:* each gradient against differences within what they resolve; optimizers recovering
closed-form optima: Vernon et al.'s dual-coupling condition, the 50:50 coupler of a HOM
experiment, a Clements mesh programmed to a Haar-random unitary.

*Done 2026-11-14:* `quantum::objective` (docs/methods/quantum.md, "Objectives and their
gradients"): `probability_gradient` (the minors' permanents, indistinguishable and
distinguishable photons), `fidelity_gradient` (Clements et al.'s Eq. 6), `visibility_gradient`
(the quotient of the two coincidences) and `purity_gradient` (tr A²/(tr A)² on the joint
amplitude, no Schmidt decomposition), each a Wirtinger sensitivity that the circuit adjoint takes;
and `ring_spectrum_derivative`, a ring's joint amplitude differentiated in its enhancements (a
coupling, Vernon et al.'s second channel at the pump) or the pump's bandwidth. Cases, each with
its Rule: `quantum/probability-gradient` (2.0e-10), `quantum/fidelity-gradient` (1.3e-10),
`quantum/visibility-coupler` (the closed form, 8.9e-16) and `quantum/purity-gradient` (2.7e-9).
`optimize_quantum`: the HOM coupler's R = ½ by L-BFGS-B through the circuit adjoint (10
evaluations from 0.15), a 4-mode Clements mesh to a Haar-random unitary (1 − F at round-off in
146), and a ring with equal linewidths (the purity rises to the broad-pump limit, K = 1.0904
against Vernon et al.'s Fig. 2(a)'s 1.09, checked). *Left:* Vernon et al.'s dual-coupling
condition is not reproduced: for K = 1.0009 ours needs a pump linewidth about 14 times the
signal's (12.8 on ±40 linewidths, rising with the window), their Fig. 2(b) 6.6 (from their full
device's simulation, whose model the paper doesn't give); at 6.6 ours gives K = 1.0034. Kept in
#595. The dispersion objectives through the mode solvers' derivatives (I9) are named on the
methods page; no example uses them yet.

### I11: Device and circuit co-design

The chip optimization of #346 through the solvers' own shape derivatives (I7, I9) rather than
differenced gaps and radii: an inverse-designed device inside a circuit, its compact model's
derivatives carried through the circuit adjoint (Mason 2025, arXiv 2509.07233, as the target).

*Validation:* the composed gradient against differences; a closed-form chip (a ring with a
designed coupler at critical coupling).

### I12: Explore in 2D, verify in 3D

The pipeline's last step and ROADMAP's 3D validation set, as examples with a plain run that CI
compares and a `--full` run that solves in 3D (decision 4): Chen 2024's 3D mode converter, Piggott
2015's demultiplexer, Shen 2015's polarization beamsplitter (with tachsin/genoxide#426's scan),
Frellsen 2016's mode multiplexer, Su 2018's grating coupler and Piggott 2020's foundry designs.
The performance plan's Phase D (mixed precision and the GPU's QMR) is decision 3.

## 4. Decisions

*I decided on 2026-11-10 to take the recommended option of each of the eight; each says so below, and its text is kept.*

1. **Where densities live.** *Decided 2026-11-10: the recommended option.* (a) A new module, `photonoxide::topology`: densities, filters,
   projections, robust designs, length scales and level sets, solver-independent, each solver's
   gradient pulled back through it; (b) inside each solver, beside its adjoint. *Recommended: (a);*
   FDFD and FDTD share it, and the Rust module boundary keeps it in the one crate.
2. **The testbed's designs as data.** *Decided 2026-11-10: the recommended option.* Chen et al.'s designs and scripts are released under the
   MIT licence (NanoComp/photonics-opt-testbed). (a) Commit only the designs the cases use
   (160 × 160 values, 128 kB each), with the licence and the commit they came from; (b) fetch them
   at test time. *Recommended: (a):* the cases must run offline, and the licence allows it.
3. **The performance plan's Phase D** *Decided 2026-11-10: the recommended option.* (a single-precision LU refined in double, the GPU's QMR).
   (a) Its own issue under the performance plan, outside this umbrella; (b) a phase here.
   *Recommended: (a):* it is a solver change that every 3D user needs, independent of design.
4. **3D verifications.** *Decided 2026-11-10: the recommended option.* The 3D devices take hours each. (a) Examples with `--full`, each run
   once, its numbers recorded in its doc comment and the plain run compared by CI; (b) cases in
   the validation report. *Recommended: (a),* as `optimize_ybranch` does.
5. **The augmented Lagrangian.** *Decided 2026-11-10: the recommended option.* It is planned in genoxide (batch C) but not released. (a) Use
   MMA and GCMMA's constraints, which genoxide has; (b) wait. *Recommended: (a).*
6. **Forward mode.** *Decided 2026-11-10: the recommended option.* (a) Only where parameters are fewer than outputs, and as a cross-check of
   the adjoints; (b) a general forward mode through every solver. *Recommended: (a).*
7. **Published designs on another discretization.** *Decided 2026-11-10: the recommended option.* Christiansen & Sigmund's FEM and the
   testbed's Meep numbers are not our grid's. *Recommended:* each compared at the paper's
   printed digits with our grid study beside it; where our own error is above a third of the
   tolerance, the number is reported and the gap kept open (#595), never the tolerance loosened.
8. **The dashboard's place.** *Decided 2026-11-10: the recommended option.* (a) The device page's Optimize tab gains the design view, as #338
   moved #244's views there; (b) a separate dashboard. *Recommended: (a).*

## 5. Papers

In the folder: Chen 2024, Wang, Lazarov & Sigmund 2010, Jensen & Sigmund 2010, Christiansen &
Sigmund 2021, Sigmund 2009 and 2011, Hammond 2021, Schubert 2022, Probst 2024, Shang 2023,
Piggott 2015 and 2017, Khan 2024, Molesky 2018, Lalau-Keraly 2013, Hughes 2018, Minkov 2020,
Shen 2015, Su 2020, Svanberg 1987 and 2002, Liu 2026. Each is read before the phase that uses it.

Missing, each DOI checked on Crossref on 2026-11-09 (the file name by the year online):
To get, the journal versions, each DOI checked on Crossref on 2026-11-09, the file name by the
year online (most important first):

- M. Zhou, B. S. Lazarov, F. Wang, O. Sigmund, "Minimum length scale in topology optimization by
  geometric constraints", Comput. Methods Appl. Mech. Eng. 293, 266 (2015),
  doi:10.1016/j.cma.2015.05.003 (I3): `zhou-2015.pdf`.
- A. M. Hammond, A. Oskooi, I. M. Hammond, M. Chen, S. E. Ralph, S. G. Johnson, "Unifying and
  accelerating level-set and density-based topology optimization by subpixel-smoothed
  projection", Opt. Express 33, 33620 (2025), doi:10.1364/OE.563512 (I6): `hammond-2025.pdf`.
- R. Arrieta, G. Romano, S. G. Johnson, "Hyperparameter-free minimum-lengthscale constraints for
  topology optimization", Struct. Multidiscip. Optim. 69, 210 (2026),
  doi:10.1007/s00158-026-04388-6 (I3): `arrieta-2026.pdf`.
- X. Qian, O. Sigmund, "Topological design of electromechanical actuators with robustness toward
  over- and under-etching", Comput. Methods Appl. Mech. Eng. 253, 237 (2013),
  doi:10.1016/j.cma.2012.08.020 (I2, the threshold's edge shift): `qian-sigmund-2013.pdf`.
- B. S. Lazarov, O. Sigmund, "Filters in topology optimization based on Helmholtz-type
  differential equations", Int. J. Numer. Methods Eng. 86, 765 (published online 2010),
  doi:10.1002/nme.3072 (I4, the Helmholtz filter): `lazarov-sigmund-2010.pdf`.
- D. Vercruysse, N. V. Sapra, L. Su, R. Trivedi, J. Vučković, "Analytical level set fabrication
  constraints for inverse design", Sci. Rep. 9, 8999 (2019), doi:10.1038/s41598-019-45026-0
  (I7): `vercruysse-2019.pdf`.
- T. W. Hughes, I. A. D. Williamson, M. Minkov, S. Fan, "Forward-mode differentiation of
  Maxwell's equations", ACS Photonics 6, 3010 (2019), doi:10.1021/acsphotonics.9b01238 (I9):
  `hughes-2019.pdf`.
- L. F. Frellsen, Y. Ding, O. Sigmund, L. H. Frandsen, "Topology optimized mode multiplexing in
  silicon-on-insulator photonic wire waveguides", Opt. Express 24, 16866 (2016),
  doi:10.1364/OE.24.016866 (I12): `frellsen-2016.pdf`.
- L. Su, R. Trivedi, N. V. Sapra, A. Y. Piggott, D. Vercruysse, J. Vučković, "Fully-automated
  optimization of grating couplers", Opt. Express 26, 4023 (2018), doi:10.1364/OE.26.004023
  (I12): `su-2018.pdf`.
- A. Y. Piggott, E. Y. Ma, L. Su, G. H. Ahn, N. V. Sapra, D. Vercruysse, A. M. Netherton,
  A. S. P. Khope, J. E. Bowers, J. Vučković, "Inverse-designed photonics for semiconductor
  foundries", ACS Photonics 7, 569 (2020), doi:10.1021/acsphotonics.9b01540 (I12):
  `piggott-2020.pdf`.
- B. Bourdin, "Filters in topology optimization", Int. J. Numer. Methods Eng. 50, 2143 (2001),
  doi:10.1002/nme.116 (I1, the filter's origin; Wang et al. restate it): `bourdin-2001.pdf`,
  optional.
- J. K. Guest, J. H. Prévost, T. Belytschko, "Achieving minimum length scale in topology
  optimization using nodal design variables and projection functions", Int. J. Numer. Methods
  Eng. 61, 238 (2004), doi:10.1002/nme.1064 (I1, the projection's origin): `guest-2004.pdf`,
  optional.

For I8's Table 1, each DOI checked on Crossref on 2026-11-13 (the journal versions):

- S. Lin, M. Hammood, H. Yun, E. Luan, N. A. F. Jaeger, L. Chrostowski, "Computational
  lithography for silicon photonics design", IEEE J. Sel. Top. Quantum Electron. 26(2), 1 (the
  March 2020 issue; Crossref gives no online date, so the year is the issue's),
  doi:10.1109/JSTQE.2019.2958931: UBC's DUV model, Khan et al.'s ref. 11: `lin-2020.pdf`.
- D. Gostimirovic, D.-X. Xu, O. Liboiron-Ladouceur, Y. Grinberg, "Deep learning-based prediction
  of fabrication-process-induced structural variations in nanophotonic devices", ACS Photonics 9,
  2623 (published online 20 July 2022), doi:10.1021/acsphotonics.1c01973: PreFab, their ref. 13:
  `gostimirovic-2022.pdf`.
- M. Zhou, B. S. Lazarov, O. Sigmund, "Topology optimization for optical projection lithography
  with manufacturing uncertainties", Appl. Opt. 53, 2720 (published online 18 April 2014),
  doi:10.1364/AO.53.002720: a physical projection-lithography model (an aerial image and a
  resist's threshold) inside topology optimization, the open alternative to the proxy:
  `zhou-2014.pdf`.
- V. Donzella, A. Sherwali, J. Flueckiger, S. Talebi Fard, S. Grist, L. Chrostowski,
  "Sub-wavelength grating components for integrated optics applications on SOI chips", Opt.
  Express 22, 21037 (published online 22 August 2014), doi:10.1364/OE.22.021037: UBC's SWG-to-strip
  converter, Khan et al.'s ref. 21, with dimensions: `donzella-2014.pdf`.

Even with these, Khan et al.'s own dimensions and their models' settings are unpublished; the
comparison wants them from the authors or a decision to compare on Donzella et al.'s converter.

Mason et al.'s co-optimized multiplexers (I11's target) have no journal version on Crossref yet;
the preprint is arXiv 2509.07233. Khan 2024 in the folder is the manuscript; the typeset version
(Opt. Lett. 50, 117) would replace it.

## 6. Where the work stands

*2026-11-14: I1 to I7 are merged (pull request #796), I8 (#806) and I9 (#835) too; I10 is on
`feat/inverse-design-i10`. Each later phase is its own branch and pull request from main.*

**Done** (each phase's *Done* paragraph above has its numbers):
- I1 to I3: densities, filters, projections, robust designs, length scales;
- I4: the mode converter, the robust demultiplexer and the foundry bend;
- I5: the topology job and its view in the run's window;
- I6: the subpixel-smoothed projection, with the crossing;
- I7: level sets, Vercruysse et al.'s penalty and the Y-branch's adjoint half;
- I8 (2026-11-13, its own pull request): the lithography proxy inside the loop, the Y-branch and
  the SWG-to-strip converter designed printed; Table 1's comparison left open (I8 above);
- I9 (2026-11-14, its own pull request): the mode solvers', RCWA's and eigenmode expansion's
  derivatives, FDFD's forward mode and a Kerr FDFD with its adjoint; Hughes et al.'s switches left
  open (I9 above);
- I10 (2026-11-14, its own pull request): quantum objectives with their gradients and
  `optimize_quantum`; Vernon et al.'s dual-coupling condition left open (I10 above).

The validation report has 31 topology cases (27 from I1 to I7, 4 from I8),
`fdfd/adjoint-gradient-row`, I9's ten (six `mode/`, one `eme/`, three `fdfd/`) and I10's four `quantum/`.

**Settled with #796:**

1. **The validation report:** rewritten with every case (`validate --write`, about an hour).
2. **The Y-branch's adjoint design in 3D: done 2026-11-12.** `--full` solved the swarm's design
   and then the adjoint search's on the 44, 31.4 and 22 nm grids (36 minutes on 6 threads, a
   peak of 33 GB); the swarm's three powers repeat the recorded ones, and the adjoint design's
   are in `RECORDED_3D_ADJOINT` (0.169 ± 0.024 dB, I7 above) and the plain run's output.
3. **The testbed's eight published designs: decided, the gap kept open in #595.** Their
   transmissions at the testbed's 10 nm against the README's printed digits are in the ignored
   studies `the_testbed_spectrum_study` and `the_testbed_grid_study`. Our grid's own change
   (0.009 dB) is above a third of the printed digits' 0.005 dB, so by decision 7 they are not a
   validation case: the numbers are reported (I4 above) and the gap is listed in #595.

**Left of the plan** (each its own pull request, part of #761; the phases above say what each
holds and how it is validated):
- I11: device and circuit co-design;
- I12: explore in 2D, verify in 3D;
- the Academy lesson `academy/inverse-design.md`, written last.

Section 5 lists the papers. Most are in tachsin's papers folder (`photonoxide-papers/`, its README
says which are read and for what).

**Pitfalls found on the way:**
- **Turned grids:** a turned grid's window is placed by its own origin. The bend's first output
  window used y's origin for x and read the wrong quantity (I4's *Corrected* note).
- **Rounding tiny values:** `round12` must keep values below about 1e-297, whose scale overflows.
- **Stage selection:** a stage's best must rank a design that meets the rules first; its start
  is not exempt.
- **Writing case texts:** a substitution that interpolates `$` (Perl's) strips a case's TeX.
  The math test only counts dollars, so it won't notice.

**The first commands on the next machine:**

```bash
git fetch origin
git switch -c feat/inverse-design-i9 origin/main   # a phase's branch, from main
cargo test --release --lib topology
cargo run -p photonoxide-studio --release -- validate --write docs/validation.md   # once, at the end
```

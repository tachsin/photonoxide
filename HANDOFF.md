# Handoff: 0.4.1, a multigrid preconditioner for 3D FDFD

Branch `fdfd3d-multigrid`. Paused on 2026-10-03; delete this file before merging.

**Goal** (ROADMAP.md, 0.4.1): QMR converging in tens to low hundreds of iterations on high-contrast
silicon problems with PMLs, so a component's 3D fidelity takes minutes; measured at equal field
accuracy against a converged reference, with wall time, against plain QMR and 0.4.0's ILU(0).

**Done** (see commit 37ae9cd's message for the details):
- `src/fdfd/three/multigrid.rs`: multigrid on Shin and Fan's operator after Reps, Vanroose & bin
  Zubair 2010 (Sec. 6.1, Fig. 14) and Erlangga, Oosterlee & Vuik 2006 (complex shift, Eq. 8; F-cycles):
  Galerkin coarse operators, ILU(0) smoothing, a direct coarsest solve; walls extend the normal E
  evenly; semicoarsening that respects the PMLs' stretch (`Line::coarser`, `ANISOTROPY`);
  interpolation of εE along a component's axis in silicon.
- `IterativeSolver3d::with_multigrid`, a `Preconditioning` enum; `krylov.rs` gained `Sparse::row`,
  and (in the last, paused commit) a start on `Sparse::from_rows` and a parallel product.
- `src/fdfd/three/multigrid_tests.rs`: experiments
  (`MG_CASE=guide|diel|vacuum cargo test --release fdfd::three::multigrid::tests::experiment -- --ignored --nocapture`).

**Measured** (stretched PMLs, shift 0.5, V(0,1)):

| Case | plain QMR | ILU(0) (0.4.0) | multigrid |
|---|---|---|---|
| 40³ silicon guide | 2739 its, 21.5 s | 254 its, 6.7 s | 18 its, 2.2 s (+4.9 s build) |
| Diel (864k) | 3662 its, 131 s | 1869 its, 158 s | 52 its, 17.6 s (+37 s build) |

The cycle alone stalls on Diel (0.97/cycle), yet QMR converges fast.

**Next:**
1. Finish what the last commit started (`Sparse::from_rows`, the parallel product); make sure it
   builds and the tests pass.
2. Tests for CI (fast): Galerkin consistency (coarse = PᵀAP), the transposed cycle is the transpose
   (QMR needs it), the convergence factor on small vacuum/silicon boxes.
3. Cut the hierarchy's build time (the Galerkin product, the ILUs) and the cycle time; parallelize
   smoothing and transfers with rayon, results bit-identical for any thread count (CI runs a
   single-thread job).
4. The strip with ports (1.8M unknowns); equal-field-accuracy benchmarks against plain QMR and
   ILU(0) (as docs/methods/fdfd-3d.md does), wall time with the build included.
5. Docs (docs/methods/fdfd-3d.md: what pays and what doesn't), a validation case (GitHub-safe LaTeX
   convention), ROADMAP 0.4.1 ticked only if the goal is met.

**Rules:** pure Rust (faer, rayon); additive API (`cargo semver-checks check-release -p photonoxide
--baseline-rev v0.4.0`); conventional commits, no AI attribution; no tags; papers in
`G:\My Drive\photonoxide-papers` (reps-2010.pdf, erlangga-2006.pdf). PR, CI green, squash-merge.

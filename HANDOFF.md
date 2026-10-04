# Handoff: 0.4.1, a multigrid preconditioner for 3D FDFD

Branch `fdfd3d-multigrid`. Paused a second time on 2026-10-04 (the machine was too slow for the
last benchmark); delete this file before merging. The first pause's account is in commit
37ae9cd's message.

**Goal** (ROADMAP.md, 0.4.1): the iterative 3D solve converging in tens to low hundreds of
iterations on high-contrast silicon problems with PMLs, so a component's 3D fidelity takes
minutes; measured at equal field accuracy against a converged reference, with wall time, against
plain QMR and 0.4.0's ILU(0).

## Where it stands

Merged with main (982a54c and after), builds without warnings, `cargo test --release --lib fdfd::`
passes (53 tests), `cargo fmt --check` passes. Not run: clippy on current stable, the full
suite, the validation report.

**The method** (`src/fdfd/three/multigrid.rs`): cycles on Shin and Fan's operator after Reps,
Vanroose and bin Zubair 2010 (Section 6.1, Fig. 14: ILU(0) smoothing, Galerkin coarse operators,
V(0, 1)) with Erlangga, Oosterlee and Vuik 2006's complex shift (Eq. 8). Semicoarsening that
respects the PMLs' stretch, interpolation of εE along a component's axis, the smoother in
slabs of 8 planes on rayon's threads.

**Added in this session:**
- `krylov::gmres_preconditioned`: restarted GMRES with right preconditioning (Saad 2003,
  Algorithms 9.5 and 6.11, modified Gram–Schmidt 6.2, complex Givens rotations of Section 6.5.9;
  the numbers checked against `saad-2003.pdf`, whose text now extracts). Its sums are in fixed
  chunks, so it is the same bit for bit on any number of threads. `IterativeSolver3d` solves by
  it when the multigrid preconditions (one cycle an iteration; QMR takes the cycle and its
  transpose). `Multigrid::restart` (40) bounds its memory: 16 bytes per unknown and step.
- A faster build: the shifted operator row by row (no sorting again), the smoother's blocks by
  binary search (no hash map each). 40³ guide: 3.3 s to 1.8 s.
- `IterativeSolver3d::multigrid_grids`: each level's cells.
- Tests that CI can run (1.5 s in a debug build), in `multigrid_tests.rs`: each coarse operator
  is PᵀAP; the transposed cycle is the cycle's transpose (V, F, W); V(1, 1) reduces its own
  operator's residual by 0.08 a cycle in vacuum (periodic, walls) and 0.2 to 0.5 with silicon in
  stretched PMLs; the cycle is the same bit for bit on 1, 2 and 5 threads; GMRES with the cycle
  takes fewer iterations than QMR with ILU(0), to the same field. In `krylov.rs`: GMRES solves to
  its tolerance restarted or not and agrees with QMR; its errors; its threads.

## Measured (this machine, 12 threads, same run; stretched PMLs of 10 cells, shift 0.5, V(0, 1), coarsest 2000, restart 40)

`MG_ILU=1 MG_CASE=guide|diel cargo test --release --lib fdfd::three::multigrid::tests::experiment -- --ignored --nocapture`

40³ silicon guide, 10 nm grid, 192 000 unknowns; hierarchy built in 1.9 s (7 levels), ILU(0) in 0.8 s:

| Solver | tolerance | iterations | time | field error |
|---|---|---|---|---|
| QMR + ILU(0) | 1e-8 | 195 | 11.2 s | 1.2e-6 |
| QMR + ILU(0) | 1e-10 | 254 | 14.9 s | 9.2e-9 |
| QMR + multigrid | 1e-10 | 26 | 4.2 s | 3.9e-9 |
| GMRES(40) + multigrid | 1e-8 | 16 | 1.3 s | 1.6e-6 |
| GMRES(40) + multigrid | 1e-10 | 22 | 1.8 s | 1.4e-8 |

Diel (Shin and Fan's, smaller: 40 × 90 × 80 cells of 10 nm, 864 000 unknowns); hierarchy built
in 6.8 s (7 levels), ILU(0) in 4.1 s:

| Solver | tolerance | iterations | time | field error |
|---|---|---|---|---|
| QMR + ILU(0) | 1e-8 | 1 386 | 365 s | 7.0e-7 |
| QMR + ILU(0) | 1e-10 | 1 869 | 493 s | 6.5e-9 |
| QMR + multigrid | 1e-10 | 66 | 33.7 s | 3.3e-9 |
| GMRES(40) + multigrid | 1e-8 | 46 | 13.5 s | 1.8e-6 |
| GMRES(40) + multigrid | 1e-10 | 61 | 17.7 s | 1.8e-8 |

The field error is against QMR + multigrid converged to 1e-12. This machine ran ILU(0) about
twice as slowly as docs/methods/fdfd-3d.md records (6.7 s and 158 s), so compare within a run.
The cycle alone, as a solver of its own operator, converges on the guide (0.41 a cycle) and
diverges on Diel (1.5 a cycle); as a preconditioner it works on both.

Where the time goes (`MG_TIMES=1`): the first coarse level has more entries than the fine one
(Diel: 14.7 M against 12.4 M, on a quarter of the rows), because PMLs of 10 cells in 40 coarsen
slowly (40, 28, 21, 17, 14, 11, 8) and the Galerkin stencil is wide. The smoothers' ILU(0) is
most of the build. One GMRES iteration is one cycle, about 0.07 s on the guide and 0.28 s on Diel.

## Not finished

1. **The strip with ports** (`a_strips_s_matrix_by_qmr`, 1.8 M unknowns; multigrid rows added to
   it, `STRIP_ONLY=multigrid` runs only them). Two attempts were stopped without a result: the
   hierarchy built (5 levels: 72 × 102 × 82, 48 × 63 × 53, 33 × 41 × 36, 24 × 28 × 25,
   17 × 19 × 18), then no solve finished in over 10 minutes. The rows use `Multigrid::default()`,
   **whose shift is 0 and whose coarsest grid is 20 000 unknowns, neither of which was measured**
   (every number above is shift 0.5, coarsest 2000). Run it with shift 0.5 first; print the
   solver's history to see whether it converges slowly or the coarsest factorization or the
   port modes are what takes the time.
2. **Settle the defaults** from measurements: the shift (0, 0.5, 1), the coarsest size, the
   cycle (V(0, 1) against F and V(1, 1)), the restart. Then make `Multigrid::default()` what the
   benchmarks use.
3. **A validation case** against the direct solver, as `fdfd3d/qmr-ilu-direct` is
   (`checks3d::ilu_against_direct`: the same strip, 24 × 20 × 16 cells): GMRES + multigrid to
   1e-10, the largest field difference. `docs/validation.md` regenerated.
4. **Docs** (`docs/methods/fdfd-3d.md`, "Preconditioning QMR"): the multigrid's section with the
   tables above measured again on the machine that writes them; the paragraph under "What didn't
   pay" about the first multigrid attempt rewritten (it now pays); the Limits' last sentence;
   Saad's GMRES and Reps et al. in the front matter's papers. ROADMAP 0.4.1 ticked only if the
   strip confirms it.
5. `cargo semver-checks check-release -p photonoxide --baseline-rev v0.4.0` (additive API:
   `Multigrid`, `CycleShape`, `with_multigrid`, `multigrid_levels`, `multigrid_grids`), clippy on
   current stable, the full suite, then delete this file, PR, CI green, squash-merge.

## Ideas not tried

- The papers folder has Bertaccini and Durastante 2018 (`chapman-2018.pdf`: 4.2 multigrid
  preconditioning, 4.3 complex symmetric systems, 3.6 sequences of systems, which a wavelength
  sweep is). Not read.
- A sweep could build the hierarchy once and reuse it across nearby wavelengths as the
  preconditioner.
- Fewer, cheaper levels: a larger coarsest grid, or a narrower interpolation, to cut the first
  coarse level's entries.

**Rules:** pure Rust (faer, rayon); additive API; results the same bit for bit on any number of
threads (CI has a single-thread job); numbers only with their grid; conventional commits, no AI
attribution; no tags. Papers: `G:\.shortcut-targets-by-id\1TbaFJQcmsCC5xgU2wzgBlSZkgm8ST-N5\photonoxide-papers`
(the handoff before named `G:\My Drive\photonoxide-papers`, the same folder): `reps-2010.pdf`,
`erlangga-2006.pdf`, `saad-2003.pdf`, `chapman-2018.pdf`.

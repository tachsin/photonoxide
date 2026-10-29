---
title: "Grid convergence"
module: convergence
summary: "The gate a number from a grid solve passes before it is reported: its grid, a grid study's estimate of its error (Richardson's where three grids allow), and a verdict, converged, not converged or unchecked."
order: 26.5
papers:
  - cite: "L. F. Richardson, Philos. Trans. R. Soc. A 210, 307 (1911) (the deferred approach to the limit)"
    doi: 10.1098/rsta.1911.0009
  - cite: "P. J. Roache, Annu. Rev. Fluid Mech. 29, 123 (1997) (grid convergence studies and their error estimates)"
    doi: 10.1146/annurev.fluid.29.1.123
---

ROADMAP's pitfalls include "a result quoted on one grid changed by 0.07 on the next", with
the rule "grid convergence gates every reported number". `photonoxide::convergence` is that
gate. A number from a grid solve is reported with its grid and a check of that grid. A number
that fails the check, or that was never checked, is flagged, and where the caller asks, refused.

## The estimate

A `convergence::Study` holds one quantity on grids, each finer than the last. The reported
number is the value on the finest grid. `Study::check(tolerance)` estimates that number's error:

| Grids | The estimate |
|---|---|
| one | none: `unchecked` |
| two, spacings h and h/r | the first-order estimate \|Δ\|/(r − 1), Δ the last change: the change itself when the spacing halves |
| three or more, the last three in one ratio r | from the changes' ratio q = Δ₁/Δ₂ (below) |

- **q above 1:** the grids converge at the observed order p = ln q / ln r. The estimate is
  Richardson's, \|v − v∞\| = \|Δ₂\|/(q − 1), with v∞ from `convergence::richardson`. That
  limit doesn't depend on r.
- **q below 0:** the values oscillate, and the estimate is the larger of the last two changes.
- **q between 0 and 1:** the changes don't shrink, the grids don't show convergence, and the
  verdict is `not_converged` whatever the tolerance.

If the last three grids aren't in one ratio, only the last two are used. The two-grid estimate
assumes first order. That is the lowest order of photonoxide's discretizations, each of which
has its convergence-order test: staircased corners converge at about first order, and smoothed
or Hadley's schemes at second.

The verdict is `converged` when the estimate is at most the tolerance. `Convergence::gated()`
returns the number only then. Otherwise it returns an error that names the number, its grid
and the reason.

## Where it gates

- **A modes job's indices.** Add `[task.convergence]`:

  ```toml
  [task.convergence]
  tolerance = 1e-4   # on each mode's Re n_eff
  grids = 3          # the task's grid and coarser ones, each twice the last: 2 or 3 (default 3)
  require = true     # fail the run when one hasn't converged (by default it is flagged)
  ```

  The modes are solved again on grids twice and four times as coarse. Each Re n_eff on the
  task's grid, the number the run reports, is checked by its study and recorded as a
  `GridConvergence` event. A mode that a coarser grid doesn't find hasn't converged. The coarse
  grids are the cheap side of the study: a quarter and a sixteenth of the solve. In the
  asymptotic range they estimate the error of the task's grid as well as a finer grid would.
  `job::check` refuses a coarsest grid with too few cells across the window or the PML, and a
  sweep, whose points it doesn't check.
- **results.json.** Its `convergence` lists every mode's index with its check, in the modes'
  order. A mode solved on one grid is `unchecked`, so a number without a check is flagged, not
  passed silently (docs/runs.md).
- **The examples.** `common::Checks::converged` adds a row for a number's grid study, and the
  example fails on an unconverged or unchecked grid. `hadley_corners` gates its 7.8 nm numbers
  this way, both schemes against their papers' tolerances (5e-5 and 1e-6), on its five grids.
  Richardson gives orders from 1.5 to 2.6, and the standard scheme's oscillating box (Fig. 4)
  gives the larger of its last two changes, 3.5e-5.
  `strip_waveguide` gates its TE-like index at 1e-4. With the standard scheme, its 20, 10 and
  5 nm indices change by 1.35e-3 and then 1.32e-3, so q = 1.03, the observed order is 0.04 and
  Richardson's estimate is 0.05: the strip's convex corners. So the example quotes Hadley's
  equations, on the quarter of its window at 10 and 5 nm (error 3.4e-5 on two grids), and with
  `--full` at 2.5 nm too (2.442168, order 1.84, error 3.7e-6). The standard scheme's indices are
  printed beside them, with their study's verdict, not converged.

## Scope

The gate covers the numbers listed above. It does not cover the rest yet: a sweep's points, FDFD's
S-parameters, FDTD's spectra, a device's figures, and the other examples' numbers. Each of those
quotes its grid but is not gated. ROADMAP's 0.6 "Convergence to
a tolerance" (the mode solver, FDFD and eigenmode expansion refining their grid until the
tolerance) is the next step.

## Validation

`convergence::tests`:

- a number that changes by 0.07 on the next grid is caught on two grids and on three;
- changes that don't shrink aren't converging, and one grid is no check;
- a second-order quantity on three halving grids gives Richardson's order 2, its limit and its
  error to 1e-12.

`job::tests::a_modes_jobs_index_on_an_unconverged_grid_is_caught` runs a 500 nm strip at 25 nm
with 50 and 100 nm beside it. At 1e-6 the index is flagged in results.json, and with `require`
the run fails. At 0.05 it is converged. Without `[task.convergence]` every index is `unchecked`.

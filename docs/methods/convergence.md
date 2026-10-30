---
title: "Grid convergence"
module: convergence
summary: "The gate a number from a grid solve passes before it is reported: its grid, a grid study's estimate of its error (Richardson's where three grids allow), and a verdict, converged, not converged or unchecked; and a driver that refines the mode solver, FDFD or eigenmode expansion until a stated tolerance, recording each grid."
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

## Convergence to a tolerance

The gate checks the grids it is given. `convergence::ToTolerance` chooses the grids itself: it
refines a solve until the quantity changes by less than a stated tolerance, and records every grid
on the way. It is a library function. The solve is a closure, `|h| -> Result<f64>` for a grid of
spacing h, or `|n| -> Result<f64>` for a count, so the same driver serves the mode solvers, FDFD
and eigenmode expansion.

```rust
use photonoxide::convergence::{ToTolerance, Tolerance};

let r = ToTolerance::new("TE n_eff", Tolerance::Absolute(1e-5))?
    .with_order(2.0)?   // the scheme's order, where it is known
    .with_budget(8)?    // at most 8 grids
    .grids(0.02, |h| solve_on(h))?;  // 20 nm, 10, 5, ... until the tolerance
println!("{} ± {:?} on {:?}", r.value, r.error, r.grids);
```

| Step | What the driver does |
|---|---|
| refine | halves the spacing (`grids`), or doubles the count (`counts`) |
| stop | once there are two grids, when the last change \|Δ\| and the estimated error are both within the tolerance |
| report, order p known | Richardson's limit of the last two grids, v + Δ/(2^p − 1); its estimated error is how far that limit moved from the previous pair's (on two grids, \|Δ\|/(2^p − 1)) |
| report, order unknown | the finest grid's value, with the gate's estimate (`Study::check`) |
| record | every grid's spacing and value (`Refined::grids`, `Refined::values`), the order the last three grids show, and the reason it stopped |
| refuse | when the budget runs out first: an error that names the quantity, its last two values and grids, the tolerance, and lists every grid |

A tolerance is absolute (`Tolerance::Absolute`, in the quantity's unit) or relative to the
quantity's value (`Tolerance::Relative`).

### Where the order is known

Give the order only where the scheme converges at it on the problem at hand. Richardson's limit
with the wrong order is wrong. These are second order:

- `mode::slab_fd`, with the layers' faces on nodes;
- 2D FDFD's S-parameters, with the interfaces on nodes (its ports' 1D operator is the same
  scheme);
- `mode::vector` on a slab, uniform along one axis, and Hadley's equations (`mode::hadley`) on
  most of his problems;
- eigenmode expansion's steps along a smooth taper: each step is the cross-section at its middle
  (the midpoint rule).

These are not, and are driven with the order unknown:

- `mode::vector` (the standard scheme) on a strip: the convex corners slow it to first order or
  below (observed 0.04 on `strip_waveguide`'s 20, 10 and 5 nm grids, above);
- staircased interfaces that fall between nodes, which converge erratically;
- eigenmode expansion's number of modes: a truncated basis, not a grid (the stepped taper below
  shows 4.8).

### A slab's index

The TE index of 220 nm of silicon (3.473) in oxide (1.444) at 1.55 µm, by `mode::slab_fd`,
driven to 1e-5 from 20 nm at second order. The exact root, by transfer matrices
(`mode::multilayer`), is 2.8448155441.

| Grid (nm) | n_eff | Change | Error against the exact root |
|---|---|---|---|
| 20 | 2.8423273 | | −2.49e-3 |
| 10 | 2.8441916 | 1.86e-3 | −6.24e-4 |
| 5 | 2.8446594 | 4.68e-4 | −1.56e-4 |
| 2.5 | 2.8447765 | 1.17e-4 | −3.90e-5 |
| 1.25 | 2.8448058 | 2.93e-5 | −9.76e-6 |
| 0.625 | 2.8448131 | 7.32e-6 | −2.44e-6 |
| Richardson's limit | 2.8448155441 | | 4.4e-11 |

It stops at 0.625 nm, the first change below 1e-5. The last three grids show order 2.00. The
estimated error of the limit is 5.8e-10, 13 times the true error. Asked for 1e-6, it would
need 0.156 nm, where the eigensolver doesn't converge. Asked for 1e-12 within three grids, it
refuses:

```text
the slab's TE n_eff did not reach the tolerance 1e-12 within 3 grids: its last two values
2.844191585596657 (grid 0.01) and 2.8446594351114105 (grid 0.005) differ by 4.678e-4; ...
```

### Eigenmode expansion

An expansion has two parameters: the number of modes each section keeps, and the number of
steps a varying section is cut into. `ToTolerance::eme(modes, steps, solve)` drives both:

1. **modes:** doubled from `modes` at `steps` steps, the order unknown;
2. **steps:** doubled from `steps` at the modes the first study ended with, at second order;
3. **together:** the modes doubled again at the steps the second study ended with, from half the
   first study's last number, to what the steps' estimated error leaves of the tolerance.

The value is the third study's, plus the steps' Richardson correction. Its error is the two
estimates' sum. A value already solved is not solved again.

The stepped taper of `eme/taper-fdfd` (a silicon slab from 220 to 300 nm through 240, 260 and
280 nm, each 0.1 µm long, TE) driven to 2e-4 relative, from 25 modes and 3 steps, and FDFD on
the same staircase driven to 2e-4 from 10 nm at second order:

| Study | Grids | \|S₁₁\|² (×1e-5) | Last change | Estimate |
|---|---|---|---|---|
| modes, at 3 steps | 25, 50, 100, 200 modes | 9.00125, 9.16320, 9.18128, 9.18194 | 6.6e-9 (7.1e-5 relative) | 2.5e-10 (observed order 4.8) |
| steps, at 200 modes | 3, 6 steps | 9.18194, 9.18194 | 0: the steps are uniform | 0 |
| together, at 6 steps | 100, 200 modes | 9.18128, 9.18194 | 6.6e-9 | 6.6e-9 |
| FDFD | 10, 5, 2.5 nm | 9.19007, 9.18382, 9.18225 | 1.57e-8 (1.7e-4 relative) | 3.5e-11 (observed order 2.00) |

The expansion gives 9.18194e-5 with an estimated error of 7.1e-5 relative. FDFD gives
Richardson's limit 9.18173e-5, with an estimated error of 3.9e-7 relative. They differ by
2.2e-5 relative, inside the expansion's estimate. With 250 modes or more, the expansion's mode
finder fails on this structure, so 1e-4 relative, which would need 400, can't be reached yet.

A smooth taper shows the steps study at work: a slab widening linearly from 220 to 600 nm over
1 µm, |S₂₁|² driven to 2e-5 relative from 40 modes and 10 steps. The modes stop at 80. The steps
go from 10 to 80, where |S₂₁|² changes by 9.6e-6, and show order 1.98. The modes, checked again
at 80 steps, change by 6.0e-6. The value is 0.996493, with an estimated error of 6.2e-6.

### Not in a job yet

`ToTolerance` is a library function. A modes job's `[task.convergence]` still checks the grids
the job gives; it doesn't drive them to a tolerance. Doing so would add fields to the job's
schema.

## Scope

The gate covers the numbers listed above. It does not cover the rest yet: a sweep's points, FDFD's
S-parameters in a job, FDTD's spectra, a device's figures, and the other examples' numbers. Each
of those quotes its grid but is not gated. `ToTolerance` can drive any of them from a script.

## Validation

`convergence::tests`:

- a number that changes by 0.07 on the next grid is caught on two grids and on three;
- changes that don't shrink aren't converging, and one grid is no check;
- a second-order quantity on three halving grids gives Richardson's order 2, its limit and its
  error to 1e-12;
- the driver on 1.5 + 3h² + 40h⁴ at second order stops on the fifth grid, its estimate 15 times
  the true error (the h⁴ term); a quantity short of its tolerance is refused, naming what it
  should; an expansion's 1/N³ and 1/S² terms are driven separately and together, each value
  solved once.

`convergence::checks::tests` drive the slab, FDFD (to 1e-3 from 10 nm: two grids, Richardson's
limit 9.18173e-5 within its estimate) and the smooth taper (its steps at order 1.98).

The validation report's cases, each with its rule (`src/validation/convergence.rs`):

| Case | What | Ours | Against | Tolerance |
|---|---|---|---|---|
| `convergence/slab-tolerance` | the slab's index driven to 1e-5 | 2.8448155441 | the exact root, 2.8448155441 (error 4.4e-11) | 1e-5, the one asked for |
| `convergence/slab-estimate` | the true error over the estimate | 0.077 | at most 1: the estimate bounds the error | 1 |
| `convergence/slab-order` | the order the recorded grids show | 2.00 | the scheme's 2 | 0.1 |
| `convergence/eme-taper-fdfd` | the stepped taper's \|S₁₁\|², the expansion driven in modes and steps | 9.1819e-5 | FDFD driven the same way, 9.18173e-5 (1.9e-5 relative, the expansion's value to five digits) | 6e-4 relative: three times the 2e-4 both are driven to, so that our own estimate (7.1e-5) is at most a third |
| `convergence/refusal` | the slab asked for 1e-12 within three grids: the parts its error misses | 0 | the quantity, the last two values and the tolerance, all named | half a part |

`job::tests::a_modes_jobs_index_on_an_unconverged_grid_is_caught` runs a 500 nm strip at 25 nm
with 50 and 100 nm beside it. At 1e-6 the index is flagged in results.json, and with `require`
the run fails. At 0.05 it is converged. Without `[task.convergence]` every index is `unchecked`.

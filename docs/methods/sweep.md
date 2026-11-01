---
title: "Adaptive sweeps"
module: sweep
summary: "A response over wavelength or frequency from as few solves as it needs: samples placed where the response changes, fitted by the AAA rational approximation, with an error estimate between the samples that was checked against the true error of 200 test functions; and a mode followed through crossings by its fields' overlaps."
order: 20.5
papers:
  - cite: "Y. Nakatsukasa, O. Sète, L. N. Trefethen, SIAM J. Sci. Comput. 40, A1494 (2018) (the AAA algorithm: Eqs. 2.5, 3.4–3.11, Fig. 4.1)"
    doi: 10.1137/16M1106122
validation:
  - sweep/aaa-rational
  - sweep/estimate-bounds-error
  - sweep/slab-dispersion
  - sweep/mode-crossing
---

A spectrum computed on a uniform grid of wavelengths spends most of its solves where nothing
happens, and still misses a narrow resonance between two of them. `photonoxide::sweep` asks
the solver only where the response needs it. It fits the samples with a rational function and
adds a sample where its own error estimate is largest, until that estimate is below the
tolerance asked for. What it returns is the fit, which can be evaluated anywhere in the range,
the samples it took and the estimated error.

```rust
use photonoxide::sweep::{Options, Sweep};
use photonoxide::Complex64;

// any response: here a resonance; in practice a solver's n_eff or S-parameter at λ (µm)
let response = |l: f64| Ok(Complex64::new(1.0 / (1.0 + ((l - 1.55) / 0.002).powi(2)), 0.0));
let sweep = Sweep::run(response, 1.5, 1.6, &Options::new(1e-8)).expect("a sweep");
assert!(sweep.converged);
println!("{} solves, estimated error {:.1e}", sweep.calls(), sweep.relative_error());
let value = sweep.eval(1.5512);
```

## The fit: AAA

AAA (Nakatsukasa, Sète and Trefethen 2018) writes a rational function in barycentric form,
their Eq. 2.5:

$$r(x) = \sum_{j=1}^{m} \frac{w_j f_j}{x - z_j} \bigg/ \sum_{j=1}^{m} \frac{w_j}{x - z_j},$$

a function of type $(m - 1, m - 1)$ that equals $f_j$ at each support point $z_j$. At each step
the next support point is the sample where the last step's error is largest, and the weights
$w$ ($\lVert w \rVert = 1$) minimize the linearized error over the other samples: $w$ is the
smallest right singular vector of the Loewner matrix $(F_i - f_j)/(Z_i - z_j)$ (their Eqs.
3.4–3.9). It stops when the error on the samples is at most $10^{-13}$ of the largest sample,
the paper's default. The poles are the finite eigenvalues of the arrowhead pencil of their Eq.
3.11. `sweep::Rational` is this algorithm on real points with complex values.

## The error estimate

The paper measures the error only on the samples, which a fit through them makes small however
wrong it is between them. A sweep needs the error between samples, so it makes its own estimate:

- The samples, in order along the range, are split into two interleaved halves, every other
  one, and each half is fitted alone.
- The estimate is twice the largest difference between the fit to all samples, $r$, and either
  half's, $r_h$. The difference is taken on a grid across the range (1001 points by default) and
  at the real part of every pole of the three fits inside the range, where a fit's spike would
  be. Its floor is $10^{-13}$ of the largest sample, AAA's own tolerance, for samples smooth to
  their last digits; samples that aren't raise it far more than their roughness (below).
- The next sample goes where the difference is largest. A pole's real part can be sampled, and
  that removes a spurious pole: a fit has no pole at a support point.

**Why it bounds the error.** With $e = f - r$ and $e_h = f - r_h$, the difference
$d_h = r - r_h$ is $e_h - e$. Wherever $r$ is at least 1.5 times as accurate as one of the
halves, $\lvert e \rvert \le \tfrac{2}{3} \lvert e_h \rvert \le \tfrac{2}{3}(\lvert e \rvert
+ \lvert d_h \rvert)$, so $\lvert e \rvert \le 2 \lvert d_h \rvert$, the estimate. A rational
fit's error falls quickly with the number of samples, geometrically for a function analytic
around the range. So $r$, with twice the samples, is usually far more accurate than either
half, not just 1.5 times. That is an argument, not a proof, so we measured it.

**How it was checked.** 200 test functions with known values on $[-1, 1]$, 20 of each of ten
kinds: Lorentzian resonances, pairs of overlapping ones, a ring's all-pass response, a
Fabry–Perot transmission, a delay line's phase $e^{ikx}$, poles near the range, a square root
with its branch point near the range's end, steps ($\tanh$), $\exp(\sin)$ and a smooth
effective index. Their widths, positions and strengths are spread over decades. Each was swept
at four tolerances, $10^{-4}$, $10^{-6}$, $10^{-8}$ and $10^{-10}$. The true error was taken on
20001 points and at the real parts of the fit's poles (`sweep/estimate-bounds-error`):

| | Our result |
|---|---|
| sweeps | 800 |
| largest ratio, true error / estimate | 0.51 |
| median ratio | 6.3e-5 |
| mean number of samples | 32 |

A ratio near one half is what the factor of two leaves where the fit to all samples is no better
than a half's, as at a spurious pole's spike, which the estimate measures at its peak. In the
median the estimate is cautious by four orders of magnitude.

The obvious alternative estimate is the difference between two successive fits, before and after
the newest sample. We tried it first, and it did not bound the error. On the same kinds of
function it often fell below the true error, at points that neither fit had sampled.

**What it can't see** is a feature that no fit has seen, one narrower than the grid's spacing,
lying between its points. We swept Lorentzian resonances on a slope over $[-1, 1]$ at $10^{-6}$,
20 positions for each half-width, with the grid's spacing 0.002:

| Half-width | In grid spacings | Mean samples | Largest ratio, true error / estimate |
|---|---|---|---|
| 0.03 | 15 | 16 | 0.04 |
| 0.01 | 5 | 16 | 0.39 |
| 0.003 | 1.5 | 16 | 0.50 |
| 0.002 | 1 | 16 | 0.42 |
| 0.001 | 0.5 | 16 | 0.76 |
| 0.0003 | 0.15 | 16 | 1.11 |

Every resonance was found, even the narrowest, far narrower than the five initial samples'
spacing of 0.5. The estimate held down to half a grid spacing, and at 0.15 of one it fell short
by 11 %.

A grid that resolves the response is the condition of the estimate. 1001 points is the default,
and `Options::with_points` changes it.

**The floor on samples that aren't smooth to their last digits.** The fits pass through the
samples to AAA's $10^{-13}$, roughness and all, and the two halves' fits part where the
roughness shows. The estimate then stops far above the roughness and grows, and more samples
don't bring it down: the sweep ends at `Options::max_samples` with `converged` false, its
`history` showing the turn. We swept $2 + 0.3 \sin 1.5x + 0.1x^2$ over $[-1, 1]$ with a relative
noise of its own on each sample, at six tolerances, 60 samples at most:

| Noise | Least estimate | Its ratio to the noise | Tolerances reached |
|---|---|---|---|
| none | 3e-11 | | all, $10^{-10}$ in 28 samples |
| 1e-12 | 7e-6 | 7e6 | $10^{-2}$ to $10^{-4}$ |
| 1e-10 | 1e-4 | 1e6 | $10^{-2}$, $10^{-3}$ |
| 1e-8 | 9e-3 | 9e5 | $10^{-2}$ |
| 1e-6 | 6e-5 | 60 | $10^{-2}$ to $10^{-4}$ |
| 1e-4 | 3e-3 | 30 | $10^{-2}$ |

The floor is 30 to 7e6 times the noise, and not in its order. A solver's samples show it too. A
slab's effective index (0.4 µm of 2.2 in 1.45, from the full-vector solver) over 1.50 to 1.60 µm
is smooth to 5.5e-16 of itself on 10 nm cells and to 9e-15 (3.2e-14 at most) on 2.5 nm cells,
by the level its Chebyshev coefficients settle to. The sweep reaches 2.5e-11 on the first in 12
samples; on the second it stops at 6.2e-9 and grows past it when asked for $10^{-10}$. On another
slab's fine grids it stopped near 1e-6, which is why the nonlinear guides' effective index is
interpolated at Chebyshev points instead ([nonlinear.md](nonlinear.md)). A sweep's tolerance must
sit above its floor, which the samples' roughness doesn't predict.

## Against a uniform grid

The TE0 mode of a slab, 220 nm of 3.473 in 1.444, from 1.2 to 2.0 µm, against its exact
characteristic equation (`mode::multilayer`). A uniform grid needs the number of samples shown
to reach the tolerance asked, when interpolated by Lagrange's cubic through the four nearest
samples or by AAA. The true error is the largest over 4001 wavelengths, relative to the largest
$n_\text{eff}$:

| Tolerance asked | Sweep: samples | Its estimate | Its true error | Uniform, cubic | Uniform, AAA | Uniform AAA at the sweep's true error |
|---|---|---|---|---|---|---|
| 1e-4 | 12 | 1.2e-6 | 6.9e-13 | 4 | 6 | 10 |
| 1e-6 | 14 | 8.8e-7 | 3.4e-13 | 9 | 6 | 12 |
| 1e-8 | 16 | 1.2e-9 | 3.5e-14 | 25 | 8 | 15 |
| 3.3e-11 | 20 | 6.4e-13 | 2.8e-15 | 100 | 10 | 15 |

For a smooth response like this one, AAA on a uniform grid needs fewer samples than the sweep
for the tolerance asked, and about as many for the error the sweep reaches. That is the cost of
the estimate: the two halves must agree, so the fit to all samples ends far more accurate than
asked. On a uniform grid there is no estimate, and nothing tells when the grid is fine enough.
With features in the response, the gap to a piecewise cubic grows by orders of magnitude:

| Response on $[-1, 1]$ | Tolerance | Sweep: samples | Its estimate | Its true error | Uniform, cubic | Uniform, AAA |
|---|---|---|---|---|---|---|
| a Lorentzian of half-width 0.01 at 0.3, on a slope | 1e-4 | 16 | 1.0e-13 | 3.5e-15 | 1656 | 8 |
| the same | 1e-8 | 16 | 1.0e-13 | 3.5e-15 | 15704 | 8 |
| a ring's all-pass response, $r = 0.95$, $a = 0.97$, phase $0.7 + 12x$ | 1e-4 | 36 | 1.2e-5 | 1.0e-13 | 2660 | 20 |
| the same | 1e-8 | 44 | 1.0e-8 | 6.9e-14 | over 20000 | 26 |

The Lorentzian on a slope is a rational function of type (3, 2), which AAA holds exactly from
eight samples anywhere. The ring's response is not rational in $x$, and its resonances are
narrow. A piecewise cubic needs thousands of samples for both. AAA on a uniform grid needs
half to 60 % of the sweep's samples, but only the sweep knows when it has enough.

## Following one mode

Sorted by effective index, the modes of a waveguide swap places where two of them cross, and a
sweep of "the second mode" jumps from one to the other. A sweep has to fit one mode across the
range. `sweep::Branch` follows it by overlap. At each new wavelength it takes, among the
solver's few modes nearest the mode found at the nearest wavelength so far, the one whose
transverse magnetic field overlaps that mode's most (`VectorMode::overlap`). That is the rule
of `mode::dispersion::track`, but with the reference the nearest wavelength sampled rather than
the previous one in a list, since a sweep samples in any order. The sweep's first samples are in
order from the start of the range, so the branch steps along the range before the samples go
where the estimate asks.

```rust,ignore
use photonoxide::sweep::{Branch, Options, Sweep};
use photonoxide::units::Wavelength;

let um = |l: f64| Wavelength::um(l).unwrap();
// the cross-section at each wavelength, on the same grid; the mode nearest 2.08 at 1.3 µm
let mut branch = Branch::new(cross_section, um(1.3), Some(2.08), 3)?;
let sweep = Sweep::run(|l| branch.effective_index(um(l)), 1.3, 1.6, &Options::new(1e-10))?;
```

`sweep/mode-crossing` checks this on a 600 × 220 nm silicon strip in oxide, on a 20 nm grid.
The strip is symmetric across its width and its height, so its TE1-like mode (odd across the
width) and its TM0-like mode (even) can't couple, and they cross near 1.42 µm. The branch
starts on TE1 at 1.3 µm and is swept to 1.6 µm.

| | Our result |
|---|---|
| samples (solves) | 29 |
| the sweep's estimate, relative | 2.5e-11 |
| largest error against the solver's TE1-like mode at 12 wavelengths between samples, relative | 2.4e-13 |
| largest departure of the second mode by index from the branch, in $n_\text{eff}$ | 0.069 |

The TE1-like and TM0-like modes are told apart by their polarization for the check, not by
overlap. Sorted by index, the second mode would jump from TE1 to TM0 at the crossing, a kink
that no smooth fit follows.

### Anticrossings

Two modes of the same symmetry don't cross. They couple and anticross: their indices come
close and part again, and through the anticrossing each mode's field turns into the other's.
Overlap has no single right answer there, and `Branch` may follow either mode without saying
so. A step much shorter than the anticrossing is wide finds the same mode's field turned only
a little, so the branch stays on the mode by index while its field changes character. A step
that jumps the whole anticrossing finds the other mode's field still like the one it left, so
the branch changes mode and keeps the field's character. A step that lands inside it finds
both modes about equally alike and takes either. So the mode followed depends on where the
samples fall.

The same strip shows it once its cladding differs above and below. It stays on oxide (1.444),
and a cladding of 1.40 is put above and beside it. The two sides now differ a little, so its
TE1-like and TM0-like modes couple weakly and anticross over a narrow range. Their indices
come closest, 0.0094 apart, at 1.4325 µm (scanned from 1.40 to 1.47 µm in 2.5 nm steps). At
1.7 µm the second mode by index is TM-like (n_eff 1.55273, TE fraction 0.01) and the third is
TE-like (1.45801, TE fraction 0.76). The branch starts on the TE1-like mode at 1.3 µm (TE
fraction 0.92) with 3 candidates, and is stepped to 1.7 µm, or swept there at 1e-8. The margin
of a step is its overlap with the mode it was followed from, less the largest overlap of the
other candidates (`BranchStep::margin`).

| How the branch was sampled | Mode at 1.7 µm | Least margin (overlap against the next) | at (µm) |
|---|---|---|---|
| steps of 2.5 nm | second by index, TM-like | 0.926 (0.998 against 0.072) | 1.4350 |
| steps of 50 nm | third by index, TE-like | 0.108 (0.761 against 0.653) | 1.4500 |
| steps of 100 nm | third by index, TE-like | 0.492 (0.902 against 0.411) | 1.5000 |
| steps of 150 nm | second by index, TM-like | 0.109 (0.755 against 0.646) | 1.4333 |
| adaptive sweep, 1e-8 | both: 39 samples past 1.482 µm TE-like, 26 TM-like | 0.469 (0.903 against 0.435) | 1.4304 |

The 2.5 nm steps stay on the mode by index; the 50 and 100 nm steps cross the gap onto the
other. The 150 nm steps go back to the mode by index, which is not a trend in the step size.
Their first step lands at 1.4333 µm, 0.8 nm from the narrowest gap, where the two fields are
about equal mixtures; the choice there was 0.755 against 0.646, and it fell on the mode by
index. The 100 nm steps cross with a margin of 0.492: their step from 1.40 to 1.50 µm passes
over the whole anticrossing and lands outside it, where one mode is still clearly more alike.
Its overlap, 0.902, is still well below the 0.998 of every 2.5 nm step.

The adaptive sweep followed both modes: past 1.482 µm, 39 of its samples are on the TE-like
mode and 26 on the TM-like one, so it was fitting a curve that jumps between two modes 0.03
(at 1.5 µm) to 0.095 (at 1.7 µm) apart in n_eff. After 86 modes found (against 36 solves for
the whole sweep with air above, below), the eigensolver did not converge for 3 modes at
1.4905 µm and the sweep stopped with that error. That failure comes from the solver and is not
explained yet.

With air above the strip instead, the two sides differ a lot and the anticrossing is broad:
the TE fraction of the mode followed falls from 0.86 at 1.3 µm to 0.17 at 1.7 µm. Every step
size from 2.5 to 150 nm, and the sweep at 1e-8 (converged in 36 solves), end on the second
mode by index (n_eff 1.42861), with least margins of 0.49 to 0.93.

The rule, then: at each step the branch takes the candidate most like the mode at the nearest
wavelength sampled. Through an anticrossing wider than its steps it follows the mode by index,
its field changing character. Through one narrower than its steps it keeps the field's
character and changes mode. Inside one, it can take either. To see whether this happened, look
at `Branch::least_margin()` (or every step in `Branch::steps()`) after a sweep. Steps of
2.5 nm kept every margin at 0.926 or above, even through the narrow anticrossing. A margin well
below that, or an overlap well below 1, marks a step that changed the field a lot or chose
between two modes. Sample more finely around that wavelength and check that the mode found
there stays the same. A sweep that follows both modes shows it too: it places many of its
samples around one wavelength and needs far more solves.

The numbers in this section are printed by
`cargo test --release --lib sweep::tests::a_branch_at_an_anticrossing_study -- --ignored --nocapture`;
`sweep::tests::a_branch_shows_an_anticrossing_in_its_margins` checks the 50 nm step's margin
on every run.

## Scope

- The variable is any real one: a wavelength in µm, a frequency. Values may be complex (an
  S-parameter), and the tolerance is relative to the largest value sampled.
- The solver's own noise sets a floor, far above the scatter of the solver's values (30 to 7e6
  times it, measured above). A sweep asked for less stops at `Options::max_samples`, with
  `converged` false.
- Mode following uses the full-vector solver's modes on a fixed grid. At an anticrossing of two
  modes of the same symmetry, the branch may follow either mode, depending on where the samples
  fall (Anticrossings, above). It says so only through its margins: it doesn't refine by itself
  or let a user choose which mode to follow.
- Not yet in photonoxide: sweeps inside the jobs, an S-matrix's entries sharing one set of samples, and
  FDFD's and eigenmode expansion's S-parameters over wavelength. These can already be swept with
  `Sweep::run` on a closure.

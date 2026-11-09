---
title: "Adaptive sweeps"
module: sweep
summary: "A response over wavelength or frequency from as few solves as it needs: samples placed where the response changes, fitted by the AAA rational approximation, with an error estimate between the samples that was checked against the true error of 200 test functions; and a mode followed through crossings by its fields' overlaps."
order: 20.5
papers:
  - cite: "Y. Nakatsukasa, O. Sète, L. N. Trefethen, SIAM J. Sci. Comput. 40, A1494 (2018) (the AAA algorithm: Eqs. 2.5, 3.4–3.11, Fig. 4.1)"
    doi: 10.1137/16M1106122
  - cite: "P. Lietaert, K. Meerbergen, J. Pérez, B. Vandereycken, IMA J. Numer. Anal. 42, 1087 (2021) (set-valued AAA: Section 2.2)"
    doi: 10.1093/imanum/draa098
validation:
  - sweep/aaa-rational
  - sweep/estimate-bounds-error
  - sweep/slab-dispersion
  - sweep/mode-crossing
  - job/fdfd-interpolate-step
  - job/modes-interpolate-crossing
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
sit above its floor, which the samples' roughness doesn't predict. (These runs are without the
stopping rule below, `Options::with_stall(0)`.)

**The stopping rule.** It is off by default, and `Options::with_stall(n)` turns it on (the runs
below take 10). A sweep whose estimate has gone n samples, and at least half as many again as it
had then, without falling below half its least, has stalled. From then on each step also
measures the samples' roughness by cross-validation. For AAA tolerances from $10^{-13}$ to
$10^{-1}$, two a decade, each interleaved half is fitted at the tolerance and evaluated at the
other half's samples. The root mean square of those misses, over $\sqrt 2$ (each miss holds two
samples' noise), is the roughness at that tolerance. Too large a tolerance misses the response,
and too small a one follows the noise between the samples. The least is the step's roughness,
and the step's gain is how many times as much the interpolating fits miss. A step takes the fits
at that tolerance, with their estimate at least the roughness, when all of these hold:

- the gain is at least 2;
- the roughness is at most a tenth of those fits' estimate and of the least estimate so far;
- that least is at most 10 %;
- their estimate is the smaller.

At its next stall the sweep looks at its steps since the estimate last made progress, at least
n of them. They show the sweep at its noise, asked for a tolerance the noise doesn't allow, when
four things hold:

- at least a fifth of them have a gain of 2 or more;
- the least estimate is at least 2.6 times their median roughness (`Sweep::noise`);
- that least is at most 10 %;
- the tolerance asked for is below 6.99 times that median roughness, the least estimate the
  noise has allowed (below).

The sweep then stops (`Sweep::stalled`) with the fit of its least estimate. Otherwise it samples
on, counting its steps afresh: it may be a response not yet resolved, which misses its own
held-out samples by about as much as its estimate, or a tolerance the noise may still allow,
which the sweep meets or runs to its limit trying, with `Sweep::noise` saying where the noise
is.

One step can't make this decision. On one sweep below (η = 1e-10), the gain went from 1.0 to 96
between consecutive samples and the roughness from 0.44 to 1.8 η. The rule as I first wrote it
(#714) decided at the one step where the stall came, so its outcome turned on the samples' last
bits. I moved every sample of that sweep by k units in its last place, as another system's `sin`
or rounding could (k = 0 to 29). It then stopped anywhere from 33 to 200 samples, and 5 of the
29 that finished ran to the limit; CI on macOS found one such case. Over 30 seeds at each of 1e-12,
1e-10, 1e-8, 1e-6 and 1e-4, about one sweep in five ran to its limit (32 of 147). Deciding
from the window, all 30 moved sweeps stop, in 46 to 147 samples.

Each threshold is the geometric mean of the two sides it separates. I measured both sides at
every stall of those noisy sweeps and of the 2000 noiseless sweeps below:

- **The share of steps with a gain of 2.** It was at most 0.09 at a noiseless sweep's stall, and
  its median was 0.45 at a noisy sweep's stall at its noise (a least estimate of 100 η at most).
  The rule takes 0.2.
- **The least estimate over the median roughness.** It was at most 1.79 at a noiseless stall
  with a least of 10 % or less, a response not yet resolved, and at least 3.87 at a noisy stall
  at its noise. The rule takes 2.6. One noiseless exp(sin) response had 7.61, and its share,
  0.07, refuses it.

The last threshold isn't a midpoint but a bound: the least estimate the noise has allowed. I
ran the 300 sweeps below the noise in the table that follows on Windows and on Linux (WSL), whose
`sin` and rounding differ in the last bits. None of the 600 got its estimate under 6.99 times its
median roughness (7.35 on Windows, 6.99 on Linux). A tolerance below that was never met, so a
stall there is the noise's; above it the sweep samples on. Without this condition the window's
rule stopped sweeps whose tolerance the noise allows. Asked for $10^{-6}$
at η = 1e-8, 34 of 60 stopped with estimates of 1.8e-6 to 8.6e-3 while their fits' true errors
were under 4e-7; macOS's CI found one of them.

The same function, asked for $10^{-13}$ with 200 samples at most, over 30 seeds at each of nine
levels and the 30 moved sweeps (relative to the largest value; a uniform noise of $\pm\eta$ has
a root mean square of about $0.5\eta$ of it; medians, with the 95 % point after the slash):

| Noise η | Stopped | Samples | Roughness | Estimate | True error of the fit |
|---|---|---|---|---|---|
| 1e-12 | 30 of 30 | 73 / 132 | 1.07 / 77 η | 68 η | 8.8 / 41 η |
| 1e-11 | 30 of 30 | 61 / 123 | 0.79 / 112 η | 144 η | 4.4 / 68 η |
| 1e-10 | 30 of 30 | 63 / 124 | 0.94 / 96 η | 52 η | 8.8 / 70 η |
| 1e-9 | 29 of 30 | 51 / 96 | 0.73 / 213 η | 67 η | 7.7 / 19 η |
| 1e-8 | 30 of 30 | 45 / 82 | 2.05 / 17 η | 807 η | 12.7 / 40 η |
| 1e-7 | 29 of 30 | 46 / 106 | 1.00 / 44 η | 2200 η | 14.3 / 57 η |
| 1e-6 | 28 of 30 | 49 / 200 | 0.65 / 2.7 η | 75 η | 10.6 / 1100 η |
| 1e-5 | 25 of 30 | 42 / 200 | 0.75 / 1.4 η | 490 η | 11.5 / 510 η |
| 1e-4 | 23 of 30 | 46 / 200 | 0.63 / 1.0 η | 120 η | 8.3 / 570 η |
| 1e-10, one seed moved by 0 to 29 ulps | 30 of 30 | 58 / 94 | 0.81 / 6.7 η | 33 η | 8.2 / 40 η |

In all 300, and in the 180 below asked for more, the estimate bounded the fit's true error, at most 0.72 of it, on both systems. Where the rule
falls short:

- **The estimate is pessimistic.** Its median over the 300 is 106 η, and one sweep in twenty
  has 1e8 η or more. Fits through noisy samples carry spurious poles, and the estimate takes the
  fits' difference at them. The fits themselves do better: their true error has a median of
  9.5 η.
- **At 1e-6 to 1e-4, 14 of 90 sweeps run to their limit.** On those, the spurious poles keep
  every fit's estimate above 10 % (1e7 to 1e10 of the largest value on one of them), so the
  least never reaches 10 % and the rule can't stop. Their fits are poor too, the true error up
  to 0.38 of the largest value. They need AAA's cleanup of spurious poles, which isn't done.
- **Some stop before the roughness is down to the noise.** At its 95 % point the roughness is
  41 η, at most 623 η, where the estimate's fits haven't resolved the noise yet.

Asked for a tolerance above the noise, 30 seeds at each of three levels (the same study):

| Noise η | Asked for | Converged | Ran to the limit | Stopped |
|---|---|---|---|---|
| 1e-10 | 10 η | 12 | 9 | 9 |
| 1e-10 | 100 η | 25 | 2 | 3 |
| 1e-8 | 10 η | 5 | 7 | 18 |
| 1e-8 | 100 η | 22 | 6 | 2 |
| 1e-6 | 10 η | 9 | 18 | 3 |
| 1e-6 | 100 η | 20 | 10 | 0 |

The fits above the noise meet a tolerance the interpolating fits never reach: 67 of 90 at
100 η (67 on Linux too). A sweep stops here only where its measured roughness puts the
tolerance under the floor, which a roughness measured too early can (its 95 % point is 24 η). Then it reports that
roughness, and the tolerance is what has to change. At 10 η, just above the floor, most sweeps
either stop so or run to their limit, the estimate seldom coming down that far. The rule
is off by default because it watches sweeps without noise too. A response not yet resolved can
stall, and a fit through it can then look better above a tolerance. With the rule on, 2000
sweeps of the test functions (50 of each kind, at the four tolerances of
`sweep/estimate-bounds-error`) ran as follows:

- 737 were watched and 117 sampled differently;
- none stopped;
- the true error stayed at most 0.43 of the estimate
  (`sweep::tests::the_stopping_rule_on_noiseless_sweeps_study`).

Before the condition of a least estimate of 10 % at most, one Fabry–Perot response stopped
after 25 samples with its estimate at 40 % and its true error 64 times that.
`sweep/estimate-bounds-error` runs without the rule, as sweeps do unless asked. On the slab's
2.5 nm samples asked for $10^{-10}$, the roughness is 1.0e-13, far below the estimate's 6.2e-9:
that floor is not the samples' noise. The tolerance, $10^{-10}$, is far above what that roughness
allows, so the sweep runs on to its 60 samples and ends with its least estimate, 6.2e-9 (7.8
without the rule). The numbers are printed by
`cargo test --release --lib sweep::tests::the_stopping_rule_on_noisy_samples_study -- --ignored --nocapture`.
`sweep::tests::a_sweep_of_noisy_samples_stops_at_their_noise` checks 18 sweeps on every run,
three realisations at each level: one seed, the same moved by 4 ulps, and a second seed.

- **Asked for $10^{-13}$ at 1e-12, 1e-10 and 1e-8.** Eight of the nine must stop (the ensemble
  stopped 120 of 120 there), and their median roughness must lie within the ensemble's 5 % to
  95 % points, 0.54 to 41 η.
- **Asked for 100 η at 1e-10, 1e-8 and 1e-6.** At least three of the nine must converge (the
  ensemble converged 67 of 90 on Windows and on Linux; fewer has a chance under 0.3 %).
- **All 18.** Each must stop only with its tolerance below 6.99 times its roughness, and each
  estimate must bound its true error.

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
| adaptive sweep, 1e-8 | both: 98 samples past 1.482 µm TE-like, 67 TM-like | 0.469 (0.903 against 0.435) | 1.4304 |

The 2.5 nm steps stay on the mode by index; the 50 and 100 nm steps cross the gap onto the
other. The 150 nm steps go back to the mode by index, which is not a trend in the step size.
Their first step lands at 1.4333 µm, 0.8 nm from the narrowest gap, where the two fields are
about equal mixtures; the choice there was 0.755 against 0.646, and it fell on the mode by
index. The 100 nm steps cross with a margin of 0.492: their step from 1.40 to 1.50 µm passes
over the whole anticrossing and lands outside it, where one mode is still clearly more alike.
Its overlap, 0.902, is still well below the 0.998 of every 2.5 nm step.

The adaptive sweep followed both modes: past 1.482 µm, 98 of its samples are on the TE-like
mode and 67 on the TM-like one, so it was fitting a curve that jumps between two modes 0.03
(at 1.5 µm) to 0.095 (at 1.7 µm) apart in n_eff. It doesn't converge: it ends at its 200
solves (against 36 for the whole sweep with air above, below), its estimate 66 relative, and
says so. It used to stop after 86 solves, at 1.4905 µm, because the eigensolver hadn't
converged for 3 modes there. Its shift, the effective index found at the nearest wavelength,
lay $2.5 \times 10^{-5}$ from an eigenvalue, which the solver now moves off
([eigen.md](eigen.md), A shift almost on an eigenvalue).

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

**Refining by itself.** `Branch::with_refinement(margin, halvings)` samples more finely on its
own. A step whose margin is below `margin` is taken again in two halves: from the mode it was
followed from to the midpoint, then on from there. Each half is refined the same way, at most
`halvings` times over. Short enough steps stay on the mode by index, so a refined branch follows
the mode *along* an anticrossing, its field changing character, wherever the samples fall.
The modes found on the way are in `Branch::found` and their steps in `Branch::steps`. On the
1.40 strip, the 50 nm steps from 1.35 to 1.45 µm with `with_refinement(0.5, 5)` become 4 steps
where they were 2 (a close choice's own solve is then not kept). Their least margin rises from 0.108 to 0.631 (0.950 against 0.319, at 1.425 µm), and the
mode at 1.45 µm keeps the rank by index it started with, as the 2.5 nm steps do. Unrefined,
it changes rank (`sweep::tests::a_refined_branch_follows_the_mode_by_index_through_an_anticrossing`).
Refinement is off by default, so a branch chooses as before unless asked.

## Several responses on one set of samples

An S-matrix has several entries, each a response over the same wavelengths, and each solve gives
all of them. `SweepSet::run` samples them together: one set of samples, each a solve, placed
where the worst response needs it.

- **The fit** is set-valued AAA (P. Lietaert, K. Meerbergen, J. Pérez, B. Vandereycken, IMA J.
  Numer. Anal. 42, 1087 (2021), [doi:10.1093/imanum/draa098](https://doi.org/10.1093/imanum/draa098),
  their Section 2.2): the K responses share their support points and weights, so one
  denominator, and each keeps its own values at the support points (`RationalSet`). The next
  support point is where the largest of the K residuals is largest, and the weights minimize the
  stacked Loewner matrices' ‖Aw‖, the sum of every response's squared linearized residuals.
  Lietaert et al. scale each response by its own largest value first; here they keep their sizes,
  so that the fit and its stopping test are at one scale for all of them, the tolerance's: a
  reflection of 1e-6 scaled up to 1 would weigh as much as the transmission in choosing the
  denominator, and its last digits' roughness with it. With one response it is AAA itself, to
  the bit (`sweep::tests::a_set_of_one_function_is_aaa`).
- **The estimate** is each response's as a single sweep's (twice the largest difference between
  the fit to all samples and either interleaved half's, on the grid and at the real parts of the
  three fits' poles), and the set's error the largest of them; the next sample is where the
  largest difference over the responses is largest. The argument that it bounds the error holds
  for each response as for one.
- **What sharing keeps.** The fits are linear in the values at the support points, so a linear
  relation that every sample keeps, such as S_qp = S_pq, the fits keep at every point to
  round-off (`sweep::tests::equal_values_make_equal_functions`, and on eigenmode expansion's S,
  `eme/wavelength-reciprocity`).
- **Checked:** three functions with the same four poles (type (3, 4) each) are held exactly, to
  1e-10 of the largest on 2001 points with at most 7 support points, the poles found to 1e-8;
  two resonances and a phase over 1.5 to 1.6 sampled together to 1e-9, each response's true error
  on 4001 points within its own estimate (`sweep::tests`). On eigenmode expansion's S, two
  structures against a solve at every nanometre: [the method's page](eme.md#s-over-wavelength).

## Scope

- The variable is any real one: a wavelength in µm, a frequency. Values may be complex (an
  S-parameter), and the tolerance is relative to the largest value sampled.
- Noisy samples set a floor. With the stopping rule the sweep measures the noise, fits above it
  and stops at it: 254 of 270 sweeps at noise levels from 1e-12 to 1e-4 did, whatever the
  seed or the samples' last bits, with estimates of a median 106 times the noise that bounded
  every fit's true error (measured above). It stops only where the tolerance is below 6.99
  times the roughness, what the noise allows; above that it samples on. A floor that isn't the
  samples' noise, as on the slab's 2.5 nm samples, isn't noticed: the sweep runs to
  `Options::max_samples` with `converged` false. The sweep doesn't clean up AAA's spurious poles as Nakatsukasa et al. do;
  it samples at their real parts instead. On noisy samples that leaves the estimate
  pessimistic, and at noise of 1e-6 and above it keeps 14 sweeps of 90 from stopping.
- Mode following uses the full-vector solver's modes on a fixed grid. At an anticrossing of two
  modes of the same symmetry, the branch may follow either mode, depending on where the samples
  fall (Anticrossings, above). Its margins show it, and `Branch::with_refinement` makes it
  follow the mode by index. Following the mode by its character instead (the other mode, across
  the gap) has no option yet: what "character" is to mean (the TE fraction, the overlap with the
  first mode) is still to be decided.
- A set of responses ([above](#several-responses-on-one-set-of-samples)) sweeps without the
  stopping rule on noisy samples.
- S over wavelength from a sweep: eigenmode expansion's (`eme::Spectrum` and the `eme` job's
  `[task.interpolate]`) and 2D FDFD's, through the `fdfd` job's `[task.interpolate]` (below).
  A `modes` job's effective indices over wavelength, through its `[task.interpolate]` (below).
  Other solvers' responses are swept with `SweepSet::run` on a closure; the other jobs' sweeps,
  and a convergence drive (`convergence::ToTolerance`) over wavelength, are not in yet.

## In an FDFD job

An `fdfd` job's `[task.interpolate]` (a `tolerance`, at least 1e-13, and `max_solves`, 200 by
default) makes its `[task.sweep]` over wavelength an interpolation: every entry of the ports' S
on one set of wavelengths by `SweepSet::run`, the grid and the ports the same at every
wavelength (the plane's slab indices taken at each solve's own), and the run records S at each
of the sweep's points from the interpolant, the ports' effective indices from a set-valued fit
through the same solves, and one solve error: the worst entry's estimate, with the wavelengths
solved. The field is recorded at `field_um` (the first point by default) from one solve, as a run
without a sweep records it; the sweep's per-point fields are left out, since it doesn't solve at
its points. `job/fdfd-interpolate-step` checks it on a 500 nm strip of the 220 nm SOI stack
widening to 800 nm, 2D on 40 nm cells, its 2 × 2 S from 1.5 to 1.6 µm asked for 1e-6: 28 solves
in place of 201, the estimate 7.9e-7 and the true error against the job solved at all 201 points
1.9e-8 (a ratio of 0.025); the interpolant's reciprocity 6.7e-14.

## In a modes job

A `modes` job's `[task.sweep]` numbers its modes by effective index at each point, highest
first. Where two modes cross, "mode 1" then jumps from one to the other. With
`follow = "overlap"` in the sweep (a wavelength sweep only, where the grid is the same at every
point) the sweep keeps each mode's number through crossings instead. Mode k at the first point
is the k-th by index. At each next point, the pairs of a mode before and a mode found are taken
by their fields' overlap (`VectorMode::overlap`), largest first, each mode once. The points are
then solved one after another, so a followed sweep isn't farmed. Leave `follow` out, or set it to
`"index"`, and the sweep numbers its modes by index as before. At an anticrossing the overlap
behaves as `Branch`'s does ([Anticrossings](#anticrossings)): which mode it keeps depends on
the step between points.

A `[task.interpolate]` (a `tolerance`, at least 1e-13 relative to the largest |n_eff|, and
`max_solves`, 200 by default) turns a wavelength sweep into an interpolation. Every mode's
effective index and TE fraction go on one set of wavelengths through `SweepSet::run`, on the same
grid at every wavelength. With `follow = "overlap"`, each solve's modes are followed from the
nearest solve's. The run records each of the sweep's points from the interpolant (in an open
window, each mode's kind and loss from its interpolated index too), and one solve error: the
worst value's estimate, with the wavelengths solved. The task's own wavelength is solved and
recorded with its fields as always. The sweep's points carry no pictures, since it doesn't
solve at them. It needs a count of modes (`modes`, not `modes_above`), and the same number of
modes at every wavelength.

`job/modes-interpolate-crossing` checks it on two slabs in air, 4 µm apart, across the job's
1 µm window. One is 120 nm of index 3.0, the other 600 nm of 2.0 above it, on a 20 nm grid. The
slabs' exact TE modes (`mode::slab`) cross once between 1.5 and 1.8 µm, at about 1.65 µm. So far
apart they don't couple, so each slab's mode is smooth through the crossing. The job's two
highest modes are asked for to 1e-6 over the sweep's 61 wavelengths. Both runs are checked
against the same job solved at all 61 points, numbered the same way:

| How the modes are numbered | Solves | Within its tolerance | Estimate | True error | Ratio |
|---|---|---|---|---|---|
| followed by overlap | 19 | yes | 1.3e-6 | 1.4e-12 | 1.0e-6 |
| by index at each solve | 200 (the most) | no | 6.4 | 3.4e-9 | 5.2e-10 |

Followed, each mode is one slab's, and 19 solves fit both to well within the tolerance. By
index, each mode has a kink where the slabs cross, and the fit can't reach the tolerance there.
The sweep spends all 200 solves on it and says it isn't within its tolerance. Its fit, from 200
solves, is still close (3.4e-9), but its estimate can't tell. The numbers are printed by
`cargo test --release --lib job::modes_checks::tests::the_interpolated_crossing_study -- --ignored --nocapture`,
and `job::modes_checks::tests::the_followed_modes_cross_where_the_slabs_do` checks on every run
that the followed modes cross where the slabs' exact ones do.

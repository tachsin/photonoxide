---
title: "Compact models and Touchstone files"
module: compact
summary: "A component's S-matrix spectrum as a rational function of frequency by vector fitting, with its fit error, stability and passivity; over its parameters as a rational model whose numerator and denominator depend on them, polynomially or piecewise linearly, with a uniform stability test; and Touchstone files, read and written, as the measured fidelity."
order: 20
papers:
  - cite: "B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999)"
    doi: 10.1109/61.772353
  - cite: "D. Deschrijver, M. Mrozowski, T. Dhaene, D. De Zutter, IEEE Microw. Wireless Compon. Lett. 18, 383 (2008) (fast vector fitting)"
    doi: 10.1109/LMWC.2008.922585
  - cite: "P. Triverio, S. Grivet-Talocia, M. S. Nakhla, IEEE Trans. Adv. Packag. 32, 205 (2009) (the parameterized form)"
    doi: 10.1109/TADVP.2008.2007913
  - cite: "C. K. Sanathanan, J. Koerner, IEEE Trans. Autom. Control 8, 56 (1963) (the reweighting)"
    doi: 10.1109/TAC.1963.1105517
  - cite: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012) (the analytic ring)"
    doi: 10.1002/lpor.201100017
  - cite: "Touchstone File Format Specification, Rev. 1.1, EIA/IBIS Open Forum (2002), ibis.org"
  - cite: "Touchstone File Format Specification, Version 2.0, IBIS Open Forum (2009), ibis.org"
validation:
  - compact/vf-paper-rms
  - compact/vf-paper-poles
  - compact/vf-paper-residues
  - compact/vf-paper-real-start
  - compact/vf-fast-equivalence
  - compact/vf-ring-allpass
  - compact/vf-ring-poles
  - compact/fdfd-ring-held-out
  - compact/fdfd-ring-passive
  - compact/param-ring-coupling
  - compact/param-ring-shift
  - compact/param-ring-stable
  - compact/param-ring-piecewise
  - compact/param-ring-piecewise-stable
  - compact/param-ring-piecewise-crossings
---

A component's S-matrix over a band is a spectrum: one matrix per wavelength, from a solver or a
measurement. A compact model replaces it with a few numbers that can be evaluated at any
wavelength in the band, and costs nothing to evaluate inside a circuit. The models are
**rational in frequency**, fitted by vector fitting, and **polynomial (or piecewise linear) in
the component's parameters**. Each comes with its error against the spectrum it was fitted to. Measured and
simulated spectra travel between tools as Touchstone files, which are read and written here.
A file read in is the **measured** fidelity of a component.

Everything is in `compact`: `CompactModel`, `ParametricModel` and `Measured` are circuit
components (`circuit::Component`), and `compact::fit` is the vector fitting itself.

## Vector fitting

Gustavsen and Semlyen (1999) fit a frequency response f(s) as poles and residues (their Eq. 2):

$$
f(s) \approx \sum_{n=1}^{N} \frac{c_n}{s - a_n} + d + s h .
$$

The poles make this nonlinear. Vector fitting solves it as two linear least squares, both with
known poles.

**Pole relocation** (their Stage 1). Start from poles $\bar a_n$. Fit an unknown scaling
function $\sigma(s) = \sum_n \tilde c_n/(s - \bar a_n) + 1$ together with $\sigma f$, both on
the starting poles, from the linear equation (their Eq. 4)

$$
\sum_n \frac{c_n}{s - \bar a_n} + d + s h - \Big(\sum_n \frac{\tilde c_n}{s - \bar a_n}\Big) f(s) = f(s)
$$

at every sample. Written as ratios of polynomials, the starting poles cancel, and
$f = (\sigma f)/\sigma$ has the zeros of $\sigma$ as its poles (their Eq. 8). Those zeros are the
eigenvalues of $A - b\tilde c^T$, with A the starting poles on its diagonal and b ones (their
Appendix B). Poles that come out in the right half plane have their real parts' signs inverted,
as the paper does.

**Residue identification** (their Stage 2). With the new poles fixed, the residues, d and h of
f come from a second linear least squares.

**Iteration.** The new poles are the next starting poles. Real starting poles on a response with
resonances fail at first, then converge. The paper's Table 4 has an RMS error of 7.1, then
1.0e-11 and 4.2e-13. Here it is 20, then 6e-12 and 2e-12 (`compact/vf-paper-real-start`). A fit
runs up to 20 relocations, stops when no pole moves by more than 1e-10 of its magnitude, and
keeps the relocation with the smallest error.

**Several responses** share σ, and so share their poles: the vector formulation of the
discussion's reply (their Eqs. 15–21). The S-matrix model fits all n² elements this way. The
Stage-1 least squares then holds every element's own unknowns and the shared $\tilde c$, a
sparse block system (Deschrijver et al. 2008, Eq. 8). Each element's own unknowns are
eliminated exactly by a QR factorization of its block $[A\ B\ b] = QR$ (their Eq. 10, with the
right side taken into the factorization). What is left for $\tilde c$ is the lower-right block
of R, and the blocks of every element are stacked and solved together (their Eq. 11). This is
their fast vector fitting; it gives the whole block system's $\tilde c$ to 1.1e-11
(`compact/vf-fast-equivalence`). They mention two refinements, not used here: Gustavsen's
relaxed σ, for noisy data, and solving the normal equations, faster but less accurate. Columns
are scaled to unit norm, and the least
squares are solved through the singular values of R, dropping those below $10^{-13}$ of the
largest. A fit asked for more poles than its response has then has a minimum-norm solution
instead of a singular one.

**Starting poles** follow the paper's Section 3.2 (Eqs. 9–10): complex, their imaginary parts β
spread linearly over the samples, their real parts −β/100. On a band far from zero frequency,
an optical one, β/100 is far wider than the poles' spacing. There the real parts are capped at
half the spacing, which keeps each starting pole within its share of the band.

**The paper's own test** (Section 4.2) is an 18th-order response with 2 real poles and 8
conjugate pairs, d = 0.2, h = 2e-5, at 100 frequencies to 100 kHz. Fitted from the 20 starting
poles of their Table 2 in one relocation, the paper's RMS error is 3.8e-12; here it is 1.4e-11.
The poles are recovered to 1.8e-11 and the residues to 4.3e-11 of their magnitudes. The two
surplus poles' residues are 3e-6 rad/s, against the paper's 4.3e-7 Hz.

## Photonics

**The Laplace variable.** photonoxide's fields go as $e^{-i\omega t}$, so a field $e^{st}$ has
$s = -i\omega$. A wavelength λ is $s = -i\,2\pi/\lambda$, in rad/µm. A stable pole has
$\operatorname{Re} a \lt 0$, as in the paper's convention. A resonance at ω with quality factor
Q has $a \approx -\omega/2Q - i\omega$. A delay τ (µm of light travel, $n_g L$ for a guide) is
$e^{-s\tau} = e^{i\omega\tau}$, the phase of light that travels. `Delay::Fixed(τ)` takes it out
before the fit. `Delay::Estimate` takes each element's delay from the slope of its unwrapped
phase.

**Real and complex models.** A real system has $f(\bar s) = \overline{f(s)}$: its poles and
residues are real or come in conjugate pairs, and d and h are real. That is the paper's model,
`Symmetry::Real`, with their real formulation of the least squares (A.5–A.8) and of the zeros
(B.2). It is what a time-domain model needs. An optical spectrum, though, is sampled in a narrow
band near 200 THz and never at negative frequencies. There `Symmetry::Complex`, the default, lets
every pole, residue and d be any complex number: the same equations, A.1–A.4 and B.1, without the
pairing. Each resonance then needs one pole instead of a pair. Both fit the analytic ring below
to about 1e-11 RMS (12 complex poles, or 24 real ones in pairs).

**An all-pass ring** (Bogaerts et al. 2012, Eq. 1) is the analytic test:

$$
\frac{E_\text{pass}}{E_\text{in}} = \frac{r - a e^{i\phi}}{1 - r a e^{i\phi}} , \qquad \phi = \beta L .
$$

With $n_\text{eff}$ first order in λ (their Eq. 10, $n_g$ constant), φ is exactly linear in
$k = 2\pi/\lambda$: $\phi = L(n_g k - (n_g - n_0)k_0)$. The poles are then exactly where
$r a e^{i\phi} = 1$:

$$
k_m = \frac{2\pi m + C + i \ln(ra)}{L n_g}, \qquad C = L(n_g - n_0)k_0 ,
$$

each with the residue $-(1/r - r)/(L n_g)$ in s. The test ring has r = 0.95, a = 0.98, a 10 µm
radius and $n_g = 4.2$: resonances 9.1 nm apart and 0.2 nm wide. Over 1.53–1.57 µm, its four
resonances sampled at 401 wavelengths, 12 complex poles fit it to 8e-12 at 1001 wavelengths.
The four poles in the band are recovered to 1e-11 of their real parts, which are the
resonances' half-widths. The other 8 poles stand for the resonances outside the band.

**A solver's spectrum.** The ring-fdfd job (`jobs/ring-fdfd.toml`: a 2 µm ring on 220 nm SOI
by 2D FDFD with the effective index method) is run at a coarse 40 nm grid, at 51 wavelengths
from 1.50 to 1.60 µm, about 25 s. Its 2-port S, fitted with 12 complex poles on the 26
even-numbered wavelengths, predicts the 25 between them to 4.5e-5. It fits its own wavelengths
to 8.5e-7, and all 51 to 6.6e-7. On 25, 40 and 50 nm grids alike, the solver's spectrum fits no
closer than about 2e-7, which is the solver's own noise. The fitted model is passive, its
largest singular value 0.908. These are 2D numbers, an estimate of the device, not its
performance.

**Fit error.** `FitError` holds the RMS and the largest $\lvert\Delta S_{qp}\rvert$, each also
relative: the RMS over the data's RMS, and the largest over the data's largest. A relative error
taken point by point would be infinite at a critical-coupling notch. A `CompactModel`'s
provenance reports the largest absolute error.

**Passivity.** A device without gain gives out at most the power it takes in, so no singular
value of S exceeds 1. `CompactModel::passivity(points)` checks this at points evenly spaced in
frequency over the band, and lists every violation with its wavelength. It only checks; it doesn't
enforce. A fit that fails needs more poles or samples. Enforcing passivity by perturbing the
residues is not implemented.

**Bands.** A rational fit says nothing outside the band it was fitted on, so evaluating a model
there is an error. The same rule holds for a material outside its data.

## Over a component's parameters

A `ParametricModel` has the form of Triverio, Grivet-Talocia and Nakhla (2009, Eqs. 6, 7 and 11):
a numerator and a denominator on one set of fixed basis poles, their coefficients depending on
the parameters p through weights $w_l(p)$:

$$
H_e(s, p) = \frac{N_e(s, p)}{D(s, p)}, \quad
N_e = \sum_l w_l(p) \Big(\sum_n \frac{c_{enl}}{s - a_n} + d_{el}\Big), \quad
D = 1 + \sum_l w_l(p) \sum_n \frac{\tilde c_{nl}}{s - a_n} .
$$

The basis poles cancel between N and D (their Section IV-B). The model's poles at p are the
zeros of D, again the eigenvalues of $A - b\tilde c(p)^T$, and their residues are $N(z)/D'(z)$.
D's constant term is fixed at 1. Their Theorem 1 leaves it free ($r_0$, "fixed at will"), and
fixing it keeps D from vanishing at high frequency, so no pole goes to infinity. The weights
come in two kinds, `Interpolation::PiecewiseLinear` and `Interpolation::Polynomial`.

**Piecewise linear, Triverio et al.'s.** The samples lie on a grid. Each is fitted on its own by
vector fitting, with stable poles, and its pole–residue model is rewritten exactly on the basis
poles by their Theorem 1 (Eqs. 24–26): D's coefficients come from $D(p_l) = 0$ at every local
pole, which is a Cauchy system, and N's from $R_n = r_n H(a_n)$ and $R_0 = Q_0$. Between samples
the coefficients are linear, or multilinear for several parameters (their Eqs. 8–9). The basis
poles are spread linearly over the band, which their Table I shows conditions the rewriting
best. The model is stable at every sample by construction (their Section III-D).

**Polynomial.** The weights are the monomials of total degree at most D, each parameter scaled
to [−1, 1] over the samples' range. The basis poles are every sample's common poles by vector
fitting. Written at every sample and frequency, N − fD = 0 is linear in c, d and $\tilde c$:
one least squares, eliminated element by element as in Stage 1. That is Sanathanan and
Koerner's weighted error (their Eq. 3), which weights each equation by $\lvert D \rvert$, small
at resonances. So it is solved again with each equation divided by the last solve's
$\lvert D \rvert$, as their Eqs. 5–7 do. They ran 10 iterations. Here there are up to 10,
stopping when three in a row don't improve the largest error at the samples, and the best is
kept. On the shifting ring of the table below, the 10 iterations reach 1.9e-7 between samples
where 3 reached 5.8e-7. Triverio et al. fit each sample on its own instead; a joint fit iterated this way is what they
cite as a parametric Sanathanan–Koerner iteration (their references 20 and 21), whose least
squares, they note, needed far more memory on their examples.

**Which one.** A resonance moves with a device's parameters, and its pole moves with it. All of
the following were measured on the analytic ring of the previous section. With $n_\text{eff}$
swept from 2.39 to 2.41, the resonances shift by 0.8 of a free spectral range, about 35
linewidths; with the self-coupling r from 0.90 to 0.97, none moves:

| Method | Between samples, n_eff (13 samples) | Between samples, r (5 samples) |
|---|---|---|
| Common poles for every sample, residues interpolated | the common 16 poles alone fit the samples only to 1.3 | |
| Each sample fitted alone, its poles and residues interpolated | 0.2 to 0.8 (measured while choosing the method) | |
| Triverio et al.: each sample alone, rewritten on the basis, piecewise linear | 7, with poles crossing the axis; no better with 25 or 49 samples, or warm-started fits | 8.6e-4 |
| Polynomial, all samples at once, reweighted | 1.9e-7 (16 basis poles, degree 6) | 2.3e-10 (12 basis poles, degree 2) |

A straight line between two denominators whose zeros lie more than a linewidth apart doesn't
move the zero; it makes new ones, some in the right half plane. That is why the piecewise-linear
model fails on the shift however densely it is sampled. On the coupling it is a straight line's
error, 8.6e-4. Eq. 1 is a ratio of functions linear in r, so a polynomial of degree 1 represents
it exactly. The polynomial model is the one to use for resonant devices. The piecewise-linear
model suits responses that change by less than a linewidth between samples, and only for it can
stability be decided.

**Errors.** `ParametricModel::error` is the model's error at its samples. `error_against` takes
spectra at values that weren't fitted, and that is the error to quote. Neither model says
anything outside the sampled range, so values there are an error.

**Stability.** Triverio et al. test uniform stability, over the whole parameter range, by
writing the piecewise-linear model as a descriptor system whose matrices lie in a polytope, and
asking whether linear matrix inequalities at its corners are feasible (their Theorem 2, solved
with SeDuMi). That needs a semidefinite solver, and the pure-Rust ones need BLAS and LAPACK for
semidefinite cones. `uniform_stability()` decides the same question exactly, for one parameter,
by another route:

- Along a segment between samples k and k + 1, D's coefficients are $\tilde c_k + t\delta$, so the
  poles are the eigenvalues of $H - t\thinspace b\delta^T$, with $H = A - b\tilde c_k^T$: a
  rank-one change.
- A pole sits on the axis at $s = j\omega$ when $1 + t\thinspace g(j\omega) = 0$, with
  $g(s) = \delta^T(sI - H)^{-1}b$, that is, where $g(j\omega)$ is real and at most −1.
- g is real on the axis where $F(s) = g(s) - \overline{g(-\bar s)}$ vanishes. F is realized on
  $\operatorname{blkdiag}(H, -\bar H)$, and its zeros are the finite generalized eigenvalues of
  its system pencil: every candidate ω at once, without sampling.
- Each candidate's t is pinned by bisection on the sign of the nearest pole's real part.

The samples' own poles are checked as well. On the coupling model it finds no crossing, and its
poles sampled at 1001 values agree (`compact/param-ring-piecewise-stable`). On the shift model
it finds 50. Each one's nearest pole changes sign across it, and stepping through 2001 values
finds no change in the count of unstable poles that it didn't report
(`compact/param-ring-piecewise-crossings`). Poles within $10^{-7}$ of the axis, relative to
their size, are at the round-off of these non-normal eigenproblems, and can't be put on either
side.

The polynomial model's poles aren't constrained. Some fall in the right half plane outside the
band, where they stand for the resonances beyond it. With one parameter or several,
`stability(points)` samples them, and the rank-one argument doesn't apply to a polynomial. This
doesn't affect the frequency response, which is what was fitted and what a circuit evaluates.
On the coupling model the poles are stable at every sampled value, and on the shift model a few
aren't. A time-domain model needs stable poles: `stable_rational(values, points)` gives one at
a parameter point, flipping D's unstable zeros as Gustavsen and Semlyen do and identifying the
residues again (Stage 2) against the model's own response. On the shifting ring this costs at
most 1.2e-7 against Eq. 1.

## Touchstone files

The format is specified by the IBIS Open Forum: Rev. 1.1 (2002) for the original syntax and
Version 2.0 (2009), which adds keywords in square brackets. Both are free from ibis.org.
`Touchstone::parse` and `Touchstone::read` take either, and `Touchstone::write` writes either.

**Version 1** (Rev. 1.1, Section 3) is an option line and data:

```text
! a comment, anywhere after !
# GHz S RI R 50
1.0  0.39 -0.12  -0.0003 -0.0021  -0.0003 -0.0021  0.39 -0.12
```

- The option line, `# <unit> <parameter> <format> R <n>`, comes first. Its fields may be in any
  order, any case, or left out: the defaults are GHz, S, MA and 50 Ω. The units are Hz, kHz,
  MHz and GHz. The parameters are S, Y, Z, H and G (H and G for 2 ports only). The formats are
  RI (real, imaginary), MA (magnitude, angle in degrees) and DB (20 log₁₀ of the magnitude, angle).
- Each frequency's data starts with the frequency, and frequencies increase.
- For 1 and 2 ports, a frequency is one line. The 2-port order is N11 N21 N12 N22.
- From 3 ports, each row of the matrix starts a new line, with at most four pairs to a line. A
  longer row wraps to the next lines, four pairs to each line, until it ends.
- A 2-port file may end with noise parameters, five values a line. They begin where the frequency
  stops increasing, and are skipped.

The number of ports is in the extension, `.sNp`. Without one, it is read from the layout: a
frequency's data is a line with an odd number of values and the even lines after it, 2n² + 1
values in all.

**Version 2.0** starts with `[Version] 2.0`, then the option line, then `[Number of Ports]`.
Before `[Network Data]` come, in any order:

- `[Two-Port Data Order]`: `21_12` or `12_21`, required for 2 ports;
- `[Number of Frequencies]`: required;
- `[Reference]`: a resistance per port, over as many lines as needed;
- `[Matrix Format]`: `Full`, or `Lower` or `Upper` for a symmetric matrix's triangle with its
  diagonal, row by row;
- `[Number of Noise Frequencies]`, and `[Begin Information]` to `[End Information]`, which is
  skipped.

The data may run over any number of lines. A frequency starts each matrix, first on its line:
2n² + 1 values for a full matrix, n² + n + 1 for a triangle. `[Noise Data]` is skipped and
`[End]` ends the file. Mixed-mode data (`[Mixed-Mode Order]`) isn't supported, and reading it is
an error.

Every error names its line, e.g. `device.s4p, line 9: from 3 ports each row of the matrix
(4 pairs) starts on a new line, four pairs to a line`.

**Writing** follows the same rules: Version 1 keeps four pairs to a line and starts each row on a
new line. Numbers are written as the shortest decimal that reads back to the same `f64`, so RI
data round-trips exactly. MA and DB data round-trips to a few units of round-off, through
magnitude and angle. With `Precision::Significant(d)` the data is rounded to d digits, and the
frequencies are still written exactly. DB can't hold an exact zero, and writing one is an error.

### Photonics in a microwave format

**Power-normalized S.** An optical S-matrix is power-normalized: $\lvert S_{qp} \rvert^2$ is the
share of the power going in at port p that comes out at port q. A microwave S-matrix with a real
reference resistance means the same thing, so the numbers carry over unchanged. The 50 Ω written
with optical data is nominal.

**Frequency and wavelength.** Files list frequencies in hertz, increasing. A wavelength is
$\lambda = c/f$, with the exact c. `Touchstone::from_spectrum` writes a spectrum sampled in
wavelength, reversed when the wavelengths increase. `Touchstone::spectrum` reads it back, the
wavelengths to within 2 units of round-off, the ports named `o1`, `o2`, ….

**The time convention.** The specification doesn't state one. Microwave tools use the
engineering convention, fields as $e^{+j\omega t}$. photonoxide uses $e^{-i\omega t}$
([conventions](conventions.md)). The two S-matrices of one device are complex conjugates of each
other: a delay $\tau$ is $e^{-j\omega\tau}$ in one and $e^{+i\omega\tau}$ in the other.
`from_spectrum` and `spectrum` take the file's convention as an argument, `Convention::Physics`
or `Convention::Engineering`, and conjugate as needed. Nothing guesses it.

### The measured fidelity

`Measured` is a spectrum as a component of measured fidelity. `Measured::read` takes a
Touchstone file and its convention. Between samples, S is interpolated linearly in frequency,
element by element, and outside the band it is an error. Its provenance names the file and has
no error, because a file doesn't state its uncertainty. Linear interpolation needs samples dense
against the spectrum's features. A resonant device's measurement is better fitted, with
`CompactModel::from_touchstone`.

---
title: "Compact models and Touchstone files"
module: compact
summary: "A component's S-matrix spectrum as a rational function of frequency by vector fitting, with its fit error, stability and passivity; over its parameters as a rational model whose numerator and denominator are polynomials; and Touchstone files, read and written, as the measured fidelity."
order: 20
papers:
  - cite: "B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999)"
    doi: 10.1109/61.772353
  - cite: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012) (the analytic ring)"
    doi: 10.1002/lpor.201100017
  - cite: "C. K. Sanathanan, J. Koerner, IEEE Trans. Autom. Control 8, 56 (1963) (not yet read)"
    doi: 10.1109/TAC.1963.1105517
  - cite: "P. Triverio, S. Grivet-Talocia, M. S. Nakhla, IEEE Trans. Adv. Packag. 32, 205 (2009) (not yet read)"
    doi: 10.1109/TADVP.2008.2007913
  - cite: "Touchstone File Format Specification, Rev. 1.1, EIA/IBIS Open Forum (2002), ibis.org"
  - cite: "Touchstone File Format Specification, Version 2.0, IBIS Open Forum (2009), ibis.org"
validation:
  - compact/vf-paper-rms
  - compact/vf-paper-poles
  - compact/vf-paper-residues
  - compact/vf-paper-real-start
  - compact/vf-ring-allpass
  - compact/vf-ring-poles
  - compact/fdfd-ring-held-out
  - compact/fdfd-ring-passive
  - compact/param-ring-coupling
  - compact/param-ring-shift
  - compact/param-ring-stable
---

A component's S-matrix over a band is a spectrum: one matrix per wavelength, from a solver or a
measurement. A compact model replaces it with a few numbers that can be evaluated at any
wavelength in the band, and costs nothing to evaluate inside a circuit. The models are
**rational in frequency**, fitted by vector fitting, and **polynomial in the component's
parameters**. Each comes with its error against the spectrum it was fitted to. Measured and
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
Stage-1 least squares then holds every element's own unknowns and the shared $\tilde c$. Each
element's own unknowns are eliminated exactly by a QR factorization of its block
$[A\ B\ b] = QR$. What is left for $\tilde c$ is the lower-right block of R, and the blocks of
every element are stacked and solved together. Columns are scaled to unit norm, and the least
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

A `ParametricModel` is a rational model whose numerator and denominator have coefficients that
are polynomials of total degree D in the parameters p. Each parameter is scaled to [−1, 1] over
the samples' range, and $\xi_l$ are the monomials. The model is the paper's Stage-1 pair itself:

$$
H_e(s, p) = \frac{N_e(s, p)}{D(s, p)}, \quad
N_e = \sum_{n,l} \frac{c_{enl}\thinspace\xi_l(p)}{s - a_n} + \sum_l d_{el}\thinspace\xi_l(p), \quad
D = 1 + \sum_{n,l} \frac{\tilde c_{nl}\thinspace\xi_l(p)}{s - a_n} .
$$

The basis poles $a_n$ are fixed: every sample's spectrum is fitted together by vector fitting,
with common poles. They cancel between N and D. The model's poles at p are the zeros of D,
again the eigenvalues of $A - b\tilde c(p)^T$, and they move continuously with p. Written at
every sample and frequency, N − fD = 0 is linear in c, d and $\tilde c$: one least squares,
eliminated element by element as in Stage 1. It weights each equation by $\lvert D \rvert$,
which is small at resonances. So it is solved again three times, each equation divided by the
last solve's $\lvert D \rvert$, and the best solve is kept. This is the iteration of Sanathanan
and Koerner (1963). The parameterized form is Triverio, Grivet-Talocia and Nakhla's (2009).
Neither paper has been read yet; this implementation follows from Gustavsen and Semlyen's Eq. 4
with parameter-dependent coefficients, and will be checked against them.

**Why not interpolate poles and residues.** A resonance moves with a device's parameters, and its
pole moves with it. Two simpler methods were measured on the ring, with $n_\text{eff}$ swept
from 2.39 to 2.41. Over that range the resonances shift by 0.8 of a free spectral range, about
35 linewidths:

| Method | Error at parameter values between samples |
|---|---|
| Common poles for every sample, residues interpolated | the common poles alone fit the samples only to 1.3 (16 poles) |
| Each sample fitted alone, its poles and residues interpolated | 0.2 to 0.8 (measured while choosing the method): the poles in the band follow their resonances exactly, but those placed outside the band, standing for the resonances beyond it, change from sample to sample |
| Numerator and denominator polynomial in p | 5.8e-7 (13 samples, 16 basis poles, degree 6) |

On the coupling r the denominator form is exact at low degree. Eq. 1 is a ratio of functions
linear in r, so degree 1 represents it exactly, and degree 2 from 5 samples is within 3.4e-10
between them.

**Errors.** `ParametricModel::error` is the model's error at its samples. `error_against` takes
spectra at values that weren't fitted, and that is the error to quote. A polynomial says nothing
outside the sampled range, so values there are an error.

**Stability.** The denominator's zeros aren't constrained. Some fall in the right half plane
outside the band, where they stand for the resonances beyond it: `stability(points)` reports the
largest $\operatorname{Re} a/\lvert a \rvert$ over a grid of the parameters. This doesn't affect
the frequency response, which is what was fitted and is what a circuit evaluates. A time-domain
model needs stable poles. `stable_rational(values, points)` gives one at a parameter point: D's
zeros, the unstable ones flipped as the paper does, and the residues identified again (Stage 2)
against the model's own response. On the shifting ring this costs at most 4.2e-7 against Eq. 1.

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

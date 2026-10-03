---
title: "Compact models and Touchstone files"
module: compact
summary: "S-parameter spectra in and out of Touchstone files (Version 1 and 2.0), with the conversions between frequency and wavelength and between the engineering and physics time conventions."
order: 20
papers:
  - cite: "Touchstone File Format Specification, Rev. 1.1, EIA/IBIS Open Forum (2002), ibis.org"
  - cite: "Touchstone File Format Specification, Version 2.0, IBIS Open Forum (2009), ibis.org"
validation: []
---

A component's S-matrix over a band is a spectrum: one matrix per wavelength. Measured and
simulated spectra travel between tools as Touchstone files. This page covers reading and writing
them in `compact::touchstone`.

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

## Photonics in a microwave format

**Power-normalized S.** An optical S-matrix is power-normalized: $\lvert S_{qp} \rvert^2$ is the
share of the power going in at port p that comes out at port q. A microwave S-matrix with a real
reference resistance means the same thing, so the numbers carry over unchanged. The 50 Ω written
with optical data is nominal.

**Frequency and wavelength.** Files list frequencies in hertz, increasing. A wavelength is
$\lambda = c/f$, with the exact c. `Touchstone::from_wavelengths` writes a spectrum sampled in
wavelength, reversed when the wavelengths increase. `Touchstone::wavelengths` reads the
wavelengths back to within 2 units of round-off.

**The time convention.** The specification doesn't state one. Microwave tools use the
engineering convention, fields as $e^{+j\omega t}$. photonoxide uses $e^{-i\omega t}$
([conventions](conventions.md)). The two S-matrices of one device are complex conjugates of each
other: a delay $\tau$ is $e^{-j\omega\tau}$ in one and $e^{+i\omega\tau}$ in the other.
`from_wavelengths` and `s_matrices` take the file's convention as an argument,
`Convention::Physics` or `Convention::Engineering`, and conjugate as needed. Nothing guesses it.

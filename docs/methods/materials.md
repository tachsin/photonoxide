---
title: Materials
module: material
summary: Refractive indices with their sources: silicon (Li 1980), silica (Malitson 1965), silicon nitride (Luke 2015), and Sellmeier, Lorentz, Drude, Cauchy and tabulated models.
order: 2
papers:
  - cite: "H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980)"
    doi: 10.1063/1.555624
  - cite: "I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965)"
    doi: 10.1364/JOSA.55.001205
  - cite: "K. Luke et al., Opt. Lett. 40, 4823 (2015)"
    doi: 10.1364/OL.40.004823
  - cite: "M. N. Polyanskiy, Sci. Data 11, 94 (2024), the refractiveindex.info database"
    doi: 10.1038/s41597-023-02898-2
validation:
  - material/spline-line
  - material/silicon-li-table
  - material/silica-malitson-formula
  - material/silica-malitson-measured
examples:
  - silicon_index
  - silica_index
  - group_index
---

A `Material` is a dispersion model with the range of wavelengths it is valid for and its
**provenance**: the publication, its DOI, where the numbers were taken from and under which
license, and the temperature. Asking for a wavelength outside the range is an error, not an
extrapolation.

## Models

| Model | Permittivity or index |
|---|---|
| Constant | $n$ |
| Sellmeier | $n^2 = A + \sum_j B_j \lambda^2 / (\lambda^2 - C_j^2)$, λ in µm |
| Cauchy | $n = A_0 + A_1/\lambda^2 + A_2/\lambda^4 + \dots$ |
| Drude | $\varepsilon = \varepsilon_\infty - \omega_p^2 / (\omega^2 + i\gamma\omega)$ |
| Lorentz | $\varepsilon = \varepsilon_\infty + \sum_j \Delta\varepsilon_j\, \omega_j^2 / (\omega_j^2 - \omega^2 - i\gamma_j\omega)$ |
| Tabulated | measured n and k, natural cubic splines between them |

The signs of the Drude and Lorentz terms follow the [time convention](conventions.md):
loss is a positive imaginary permittivity.

## Built-in materials

- **Silicon**, 293 K, 1.2–14 µm: Li's recommended values (Table 1), through a natural cubic
  spline. All 35 values were checked against the paper. Li's uncertainty is ±2 × 10⁻⁴.
- **Silica**, 20 °C, 0.21–3.71 µm: Malitson's three-term Sellmeier formula (Eq. 1), which
  interpolates his 60 measured wavelengths to five decimals.
- **Silicon nitride** (LPCVD), 0.31–5.5 µm: Luke et al.'s two-term Sellmeier formula.

Any material of the refractiveindex.info database (CC0) can be loaded with its provenance.

## Validation

- Silicon passes through Li's table at all 35 wavelengths (to 1e-12: the spline is exact at its
  knots).
- Silica equals Malitson's computed index at his 60 wavelengths to 1e-6, and his measured mean
  of three specimens to 1e-4.
- A natural spline through points on a line is that line.

The examples `silicon_index` and `silica_index` print a few of the papers' values beside the
library's.

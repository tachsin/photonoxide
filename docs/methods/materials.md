---
title: "Materials"
module: material
summary: "Refractive indices with their sources: silicon (Li 1980), silica (Malitson 1965), silicon nitride (Luke 2015), and Sellmeier, Lorentz, Drude, Cauchy and tabulated models."
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
| Lorentz | $\varepsilon = \varepsilon_\infty + \sum_j \Delta\varepsilon_j\thinspace\omega_j^2 / (\omega_j^2 - \omega^2 - i\gamma_j\omega)$ |
| Tabulated | measured n and k, natural cubic splines between them |

The signs of the Drude and Lorentz terms follow the [time convention](conventions.md):
loss is a positive imaginary permittivity.

## Built-in materials

- **Silicon**, 293 K, 1.2–14 µm: Li's recommended values (Table 1), through a natural cubic
  spline. All 35 values were checked against the paper. Li's uncertainty is ±2 × 10⁻³; he
  prints a fourth decimal for a smooth table.
- **Silica**, 20 °C, 0.21–3.71 µm: Malitson's three-term Sellmeier formula (Eq. 1), which
  interpolates his 60 measured wavelengths to five decimals.
- **Silicon nitride** (LPCVD), 0.31–5.5 µm: Luke et al.'s two-term Sellmeier formula.

Any material of the refractiveindex.info database (CC0) can be loaded with its provenance.

## A smooth silicon

Li's table is rounded to four decimals, so the spline through it has a second derivative that is
straight between knots and jumps at them: silicon's group-velocity dispersion
$D = -(\lambda/c)\,d^2n/d\lambda^2$ zig-zags. For $D$, or for a temperature other than 293 K,
the catalogue's silicon has a second model, `frey-2006`: Frey, Leviton and Madison's
temperature-dependent Sellmeier formula (Proc. SPIE 6273, 62732J, 2006,
doi:[10.1117/12.672850](https://doi.org/10.1117/12.672850), Table 5), fitted to their
minimum-deviation measurements of two prisms from 1.1 to 5.6 µm and 20 to 300 K, with an
uncertainty of about 1e-4. It is the absolute (vacuum) index; at 1.55 µm and 293 K it is
3.4784, 2.7e-3 above Li's 3.4757 (in air), and its $D$ is about −860 ps/(nm·km), smooth.
Li's table stays the default. In Python, `refractive_index("si", wavelength_um=1.55,
model="frey-2006", temperature_k=295)`; [the catalogue](catalogue.md) has its validation.

## Temperature

The catalogue's default silicon, silica and silicon nitride take a temperature and move by their
thermo-optic coefficients $dn/dT$; at their own temperatures (293 K, 20 °C, 295 K) they are the
materials above, bit for bit. Silicon moves by Li's own $n(\lambda, T)$, his Eq. (22), 250–750 K,
below 250 K by Frey et al.'s formula, to 40 K, and below 40 K by Komma et al.'s measured $dn/dT$
at 1550 nm, to 5 K; silica by Leviton & Frey's change of index, 30–300 K, held at their $dn/dT$
at 300 K up to 400 K; the nitride by Arbabi & Goddard's 2.45e-5 per K from 270 to 400 K and
below by Elshaari et al.'s cryogenic measurement, to 19.4 K. At 1.55 µm and 293 K silicon's
$dn/dT$ is 1.82e-4 per K and silica's 8.4e-6. `IndexModel::thermo_optic` gives $dn/dT$; [the catalogue](catalogue.md) says
how the models join and has their validation against the papers' tables. A device's temperature
corner sets every material that takes a temperature, so an SOI design's corners move its silicon
and its silica. A temperature field, uniform or a heat solve's, moves a mode by first-order
perturbation or a re-solve ([fields](fields.md#a-change-of-permittivity-perturbation-and-re-solve)):
a silicon wire's $dn_\mathrm{eff}/dT$ against Dwivedi et al.'s measurements is there.

## Thermal conductivity

Beside each index the catalogue keeps the material's thermal conductivity, with the same
provenance (source, validity range, temperature): bulk silicon (Ho, Powell & Liley 1972, 148
W/(m·K) at 300 K; Kremer et al. 2004; Glassbrenner & Slack 1964, 50 K to the melting point), a
silicon film by its thickness (Ju & Goodson 1999, 90.7
W/(m·K) at 220 nm), fused silica (Powell, Ho & Liley 1966, 1.38) and the heater metals W, Ti and
Pt. A heat job's media take them unless the job gives a value
([the catalogue](catalogue.md#thermal-conductivities)).

## Phase-change materials

Sb₂Se₃, Sb₂S₃ and GST are in the catalogue in both states, amorphous and crystalline, each its own
entry: our fits to Delaney et al. 2020's ellipsometry over 1.0 to 1.69 µm, with the temperatures
that switch them and their thermal properties where a source gives them
([the catalogue](catalogue.md#phase-change-materials)).

## Validation

- Silicon passes through Li's table at all 35 wavelengths (to 1e-12: the spline is exact at its
  knots).
- Silica equals Malitson's computed index at his 60 wavelengths to 1e-6, and his measured mean
  of three specimens to 1e-4.
- A natural spline through points on a line is that line.

The examples `silicon_index` and `silica_index` print a few of the papers' values beside the
library's.

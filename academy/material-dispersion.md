---
title: "Silicon, silica and dispersion"
summary: "Why a material's refractive index changes with the wavelength, the driven oscillator behind the Lorentz, Sellmeier, Drude and Cauchy formulas, where silica's and silicon's indices come from with their uncertainties, ranges and temperatures, the group index and group-velocity dispersion they give, and what photonoxide's catalogue records for each material."
topic: Materials
level: introductory
minutes: 30
prerequisites: []
examples: [silica_index, silicon_index]
methods: [materials.md, catalogue.md, dispersion.md, conventions.md]
validation:
  - material/silica-malitson-formula
  - material/silica-malitson-measured
  - material/silicon-li-table
  - material/spline-line
  - material/silica-leviton-table
charts: [material-oscillator, material-cauchy, material-index, material-gvd]
papers:
  - cite: "W. Sellmeier, Ann. Phys. 219, 272 (1871)"
    title: "Zur Erklärung der abnormen Farbenfolge im Spectrum einiger Substanzen"
    doi: 10.1002/andp.18712190612
    year: 1871
    role: origin
    note: "The anomalous order of colours Christiansen and Kundt saw in a fuchsine solution, explained by the material's particles set vibrating with the light: particles that absorb one period raise the index for longer periods and lower it for shorter ones, the more so the nearer the period."
  - cite: "P. Drude, Ann. Phys. 306, 566 (1900)"
    title: "Zur Elektronentheorie der Metalle"
    doi: 10.1002/andp.19003060312
    year: 1900
    role: origin
    note: "The electron theory of metals: conduction electrons as free charges, the oscillator without a spring, whose permittivity is the Drude model's."
  - cite: "I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965)"
    title: "Interspecimen Comparison of the Refractive Index of Fused Silica"
    doi: 10.1364/JOSA.55.001205
    year: 1965
    role: milestone
    note: "Fused silica from three makers measured by minimum deviation at 60 wavelengths from 0.21 to 3.71 µm at 20 °C, and the three-term Sellmeier formula that interpolates them to five decimals: the silica of photonoxide's catalogue."
  - cite: "H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980)"
    title: "Refractive index of silicon and germanium and its wavelength and temperature derivatives"
    doi: 10.1063/1.555624
    year: 1980
    role: milestone
    note: "Every published index of silicon and germanium compiled and judged, and recommended values from a dispersion equation fitted to three selected sets: silicon from 1.2 to 14 µm and 100 to 750 K, to ±2 × 10⁻³."
  - cite: "B. Tatian, Appl. Opt. 23, 4477 (1984)"
    title: "Fitting refractive-index data with the Sellmeier dispersion formula"
    doi: 10.1364/AO.23.004477
    year: 1984
    role: milestone
    note: "Glasses and crystals fitted with the Sellmeier formula: three terms, two in the ultraviolet and one in the infrared, generally necessary and sufficient, and the formula good beyond the band where a power series fitted to the same data is not."
  - cite: "D. B. Leviton, B. J. Frey, Proc. SPIE 6273, 62732K (2006)"
    title: "Temperature-dependent absolute refractive index measurements of synthetic fused silica"
    doi: 10.1117/12.672853
    year: 2006
    role: milestone
    note: "The absolute index of Corning 7980 fused silica from 0.4 to 2.6 µm and 30 to 310 K, to about 1 × 10⁻⁵, with a temperature-dependent Sellmeier fit; within 1 × 10⁻⁵ of Malitson's formula corrected to vacuum below 2 µm."
  - cite: "B. J. Frey, D. B. Leviton, T. J. Madison, Proc. SPIE 6273, 62732J (2006)"
    title: "Temperature-dependent refractive index of silicon and germanium"
    doi: 10.1117/12.672850
    year: 2006
    role: current
    note: "The absolute index of silicon from 1.1 to 5.6 µm and 20 to 300 K, to about 1 × 10⁻⁴, and the advice to measure a prism from the same batch when an absolute index better than ±5 × 10⁻³ is needed."
  - cite: "K. Luke, Y. Okawachi, M. R. E. Lamont, A. L. Gaeta, M. Lipson, Opt. Lett. 40, 4823 (2015)"
    title: "Broadband mid-infrared frequency comb generation in a Si3N4 microresonator"
    doi: 10.1364/OL.40.004823
    year: 2015
    role: current
    note: "A Sellmeier formula for LPCVD silicon nitride from ellipsometry between 193 nm and 33 µm, made because one fitted to near-infrared data leaves out the absorption near 10 µm that shapes the index in the mid-infrared."
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015)"
    title: "Silicon Photonics Design"
    doi: 10.1017/CBO9781316084168
    year: 2015
    role: review
    note: "The group index and the dispersion parameter D of a guide's mode, Eqs. 3.5 and 3.6, which this lesson applies to the bulk materials."
  - cite: "M. N. Polyanskiy, Sci. Data 11, 94 (2024)"
    title: "Refractiveindex.info database of optical constants"
    doi: 10.1038/s41597-023-02898-2
    year: 2024
    role: current
    note: "An open database of optical constants, each entry with its source, in the public domain (CC0): where the catalogue's silicon, silica and nitride were taken from before each was checked against its paper."
---

A charge bound in a material and driven by the light's field, and the index that such charges
give a glass across frequency, in the symbols the equations below use. The subject is a material,
not a device, so there is no 3D view.

::diagram dispersion

The refractive index $n$ sets the speed $c/n$ of a wave's phase in a material, and every
waveguide's modes are built from the indices of its materials. No material's index is a
constant: silicon's falls from 3.5167 at 1.2 µm to 3.4757 at 1.55 µm
([Li, 1980](https://doi.org/10.1063/1.555624)), fused silica's from 1.460078 at 546 nm to 1.444024
at 1.55 µm ([Malitson, 1965](https://doi.org/10.1364/JOSA.55.001205)). A design takes them from
measurements, through a formula or a table, each valid over a stated range of wavelengths, at a
stated temperature, to a stated uncertainty.

## 1. Why does a material's index depend on wavelength?

The light's field pushes on the material's charges: electrons bound to their atoms, ions held in
the lattice. Each behaves as a mass on a spring, with a frequency $\omega_0$ at which it would
swing by itself. Driven well below $\omega_0$, a charge follows the field in step, and its dipole
adds to the field's polarization: the wave is slowed, $n \gt 1$. The nearer the drive comes to
$\omega_0$, the larger the swing, so $n$ rises with the frequency and falls with the wavelength.
Near $\omega_0$ the charges take energy from the wave and the material absorbs. Above it they swing
against the field and $n$ drops, below 1 just past the resonance.

[Sellmeier (1871)](https://doi.org/10.1002/andp.18712190612) gave this explanation for the
"abnormal" order of colours seen in a solution of fuchsine: particles that absorb light of one
period raise the index for light of longer periods and lower it for shorter ones, the more
strongly the nearer the period. A transparent material sits between resonances: those of the
electrons in the ultraviolet, those of the lattice's vibrations in the infrared
([Li, 1980](https://doi.org/10.1063/1.555624), Section 2). A centimetre of fused silica stops
transmitting near 0.16 µm in the ultraviolet and 4.5 µm in the infrared (Malitson, Section II).

The chart draws $n$ and the extinction $\kappa$ of a medium of identical oscillators of strength 1
and damping $0.05\thinspace\omega_0$. At low frequency $n = \sqrt 2 = 1.4142$; at
$0.5\thinspace\omega_0$ it has risen to 1.5271 with $\kappa$ still small; it peaks at 3.730 just
below $\omega_0$; $\kappa$ peaks at 3.470 at $1.015\thinspace\omega_0$; $n$ falls to 0.1007 at
$1.31\thinspace\omega_0$ and climbs back towards 1 (exact).

::chart material-oscillator

### The driven, damped charge {theory}

A charge $-e$ of mass $m$ on a spring of stiffness $m\omega_0^2$, slowed by a friction $m\gamma$
per unit velocity, driven by the field $E_0 e^{-i\omega t}$ along $x$:

$$
m\ddot x = -m\omega_0^2 x - m\gamma\dot x - eE_0 e^{-i\omega t} .
$$

In the steady state $x = x_0 e^{-i\omega t}$, and the equation gives
$x_0 = -(e/m)E_0/(\omega_0^2 - \omega^2 - i\gamma\omega)$. $N$ such charges per unit volume make
the polarization $P = -Nex = \varepsilon_0\chi E$, so

$$
\varepsilon(\omega) = \varepsilon_\infty + \frac{\Delta\varepsilon\thinspace\omega_0^2}{\omega_0^2 - \omega^2 - i\gamma\omega},
\qquad \Delta\varepsilon = \frac{Ne^2}{\varepsilon_0 m\omega_0^2},
\qquad n + i\kappa = \sqrt{\varepsilon} ,
$$

where $\varepsilon_\infty$ collects the resonances far above $\omega$, 1 in vacuum. With the
$e^{-i\omega t}$ convention a lossy material has $\operatorname{Im}\varepsilon \gt 0$
([conventions](../docs/methods/conventions.md)). The real part,
$\Delta\varepsilon\thinspace\omega_0^2(\omega_0^2 - \omega^2)/[(\omega_0^2 - \omega^2)^2 + \gamma^2\omega^2]$,
is positive below the resonance and negative above it; the imaginary part is a line of width
$\gamma$ at $\omega_0$. For a weak oscillator, $n \approx 1 + \chi/2$, and the real part's extremes
lie near $\omega_0(1 \mp \gamma/2)$: $n$ falls with rising frequency over a band about $\gamma$ wide,
the anomalous dispersion Sellmeier explained. For a strong one with little damping,
$\operatorname{Re}\varepsilon \lt 0$ from $\omega_0$ to
$\omega_0\sqrt{(\varepsilon_\infty + \Delta\varepsilon)/\varepsilon_\infty}$, where
$\varepsilon = 0$ when $\gamma = 0$: across that band $n$ is small, $\kappa$ large, and the
material reflects. Several resonances add, $\varepsilon = \varepsilon_\infty + \sum_j$, which is
photonoxide's `Model::Lorentz` ([materials](../docs/methods/materials.md)).

**Try it.** Lower the strength to 0.1. Where does $n$ fall, and how far?

:::answer
From $0.983\thinspace\omega_0$ to $1.048\thinspace\omega_0$, a band 0.065 $\omega_0$ wide, near
$\gamma = 0.05$; $n$ falls from 1.474 to 0.582. At low frequency $n$ is
$\sqrt{1.1} = 1.0488$. With strength 1 the band runs from $0.983\thinspace\omega_0$ to
$1.31\thinspace\omega_0$, towards $\sqrt 2\thinspace\omega_0$, where $\varepsilon$ would vanish
without damping.
:::

## 2. What do the Sellmeier, Lorentz, Drude and Cauchy models describe?

Each is the oscillator of Section 1 in a limit, or a table of measurements.

- **Lorentz**: the damped oscillators themselves, valid across an absorption line.
- **Sellmeier**: the oscillators without damping, written in the wavelength,
  $n^2 = A + \sum_j B_j\lambda^2/(\lambda^2 - C_j^2)$. Each term is an oscillator of strength
  $B_j$ resonant at the wavelength $C_j$. It holds only where the material is transparent, away
  from every $C_j$.
- **Drude**: the oscillator without a spring, $\omega_0 = 0$: free charges, the conduction
  electrons of a metal ([Drude, 1900](https://doi.org/10.1002/andp.19003060312)),
  $\varepsilon = \varepsilon_\infty - \omega_p^2/(\omega^2 + i\gamma\omega)$. The lesson on
  [surface plasmons](plasmonics.md) takes it further.
- **Cauchy**: a power series, $n = A_0 + A_1/\lambda^2 + A_2/\lambda^4$, the Sellmeier terms
  expanded for wavelengths far above every resonance and far below none.
- **A table**: measured values, interpolated, as photonoxide takes silicon (Section 4).

### From the oscillator to Sellmeier and Cauchy {theory}

With $\omega = 2\pi c/\lambda$ and the resonance at $C = 2\pi c/\omega_0$, an undamped term is

$$
\frac{\omega_0^2}{\omega_0^2 - \omega^2} = \frac{1/C^2}{1/C^2 - 1/\lambda^2} = \frac{\lambda^2}{\lambda^2 - C^2} ,
$$

so $\Delta\varepsilon\thinspace\omega_0^2/(\omega_0^2 - \omega^2)$ is the Sellmeier term with
$B = \Delta\varepsilon$. photonoxide's tests check that three undamped Lorentz oscillators at
Malitson's three wavelengths give his formula to $10^{-12}$. For $\lambda \gg C$,

$$
\frac{B\lambda^2}{\lambda^2 - C^2} = B\left(1 + \frac{C^2}{\lambda^2} + \frac{C^4}{\lambda^4} + \cdots\right),
$$

a series in $1/\lambda^2$: Cauchy's form, for $n^2$ and, to the same order, for $n$. A resonance
far on the long-wavelength side, $\lambda \ll C$, gives instead
$B\lambda^2/(\lambda^2 - C^2) \approx -(B/C^2)\lambda^2$, a term that lowers $n$ as $\lambda$
grows, which no series in $1/\lambda^2$ has.

### How far a fit can be trusted

A formula fitted to measurements holds where it was fitted. The chart fits Cauchy's three terms to
Malitson's silica from 0.4 to 0.7 µm: inside that band they agree within $6.0 \times 10^{-5}$,
and the fit stays within $10^{-4}$ only up to 0.719 µm. At 1.55 µm it is $5.4 \times 10^{-3}$ high,
at 3.0 µm 0.029: silica's infrared resonance, at 9.9 µm, bends its index down, and the series has
no term that can. Fitting further does not help: fitted up to 1.0 µm, the three terms miss
inside their own band by $3.7 \times 10^{-4}$ (exact).

::chart material-cauchy

[Tatian (1984)](https://doi.org/10.1364/AO.23.004477) fitted glasses and crystals with the
Sellmeier formula and found three terms, two on the ultraviolet side and one on the infrared,
generally necessary and sufficient. Schott's catalogue formula, a power series, and his Sellmeier
fit were equally good from 0.36 to 1.014 µm; from 1.014 to 2.33 µm the Sellmeier fit held and
Schott's formula showed large errors.
A fit can also lose its physics: in his two-term fits to silicon and germanium the infrared term's strength came out
negative, unphysical but smooth, because their index changes little at long wavelengths and is
hard to measure there. [Luke et al. (2015)](https://doi.org/10.1364/OL.40.004823) give the
converse warning: a Sellmeier formula holds in the transparent region only if every absorption
that shapes the index is in it, and silicon nitride's near 10 µm is missing from a fit to
near-infrared data.

**Try it.** Fit the Cauchy formula up to 2.0 µm. Is it better at 1.55 µm, and what has it lost?

:::answer
At 1.55 µm it is now $2.9 \times 10^{-4}$ low instead of $5.4 \times 10^{-3}$ high, but it misses
by up to $4.2 \times 10^{-3}$ inside its own band, and by 0.022 at 3.0 µm: three terms in
$1/\lambda^2$ can't follow both ends of silica's transparent window.
:::

## 3. Where does fused silica's index come from?

[Malitson (1965)](https://doi.org/10.1364/JOSA.55.001205) measured prisms of three makers' fused
silica (Corning 7940, Dynasil and General Electric 151) by minimum deviation at 20 °C, at 60
wavelengths from 0.2139 to 3.7067 µm, emission lines and absorption bands (his Table I), and
fitted the mean of the three:

$$
n^2 - 1 = \frac{0.6961663\thinspace\lambda^2}{\lambda^2 - 0.0684043^2}
+ \frac{0.4079426\thinspace\lambda^2}{\lambda^2 - 0.1162414^2}
+ \frac{0.8974794\thinspace\lambda^2}{\lambda^2 - 9.896161^2} ,
$$

$\lambda$ in µm (his Eq. 1). Its average absolute residual is $10.5 \times 10^{-6}$, so it
interpolates to five decimals. Two resonances lie in the ultraviolet, at 0.068 and 0.116 µm, and
one in the infrared, at 9.9 µm, all outside the glass's transmission. The infrared term lowers the
index at 1.55 µm by 0.0078 and at 3.0 µm by 0.032: without it the formula would give 1.451818 at
1.55 µm instead of 1.444024 (exact). His Table I has no index from 2.4 to 3.2 µm, where a strong
absorption by OH groups near 2.7 µm stopped the measurement. Twelve specimens from the three makers
differed from one another by 4 to $27 \times 10^{-6}$ (his Section III), and the index rose by about
$10^{-5}$ per kelvin in the visible (his Fig. 3).

[silica_index](../examples/silica_index.rs) checks the catalogue's silica against his Table I:

::example silica_index

and the validation report against all 60 of his computed indices, to $10^{-6}$, and his measured
means, to $10^{-4}$:

::validation material/silica-malitson-formula material/silica-malitson-measured

The chart draws $n$ and the group index $n_g$ (Section 5) over the 0.21 to 3.71 µm of the
formula; at 1.55 µm $n = 1.444024$ and $n_g = 1.4626$.

::chart material-index{material=1}

**Try it.** What are silica's index and group index at the helium–neon laser's 632.8 nm?

:::answer
$n = 1.45702$ and $n_g = 1.4754$; $dn/d\lambda = -0.0290$ per µm, against $-0.0120$ per µm at
1.55 µm. Silica disperses more strongly towards its ultraviolet resonances.
:::

## 4. Where does silicon's index come from?

[Li (1980)](https://doi.org/10.1063/1.555624) compiled 55 sets of measurements of silicon's
index and chose three on which to base his recommendation, Primak's, Salzberg and Villa's and
Icenogle et al.'s, which agree in the third decimal. He fitted them with a dispersion equation in
the wavelength and the temperature, its parameters for silicon from 1.2 to 14 µm and 100 to 750 K,
and tabulated what it gives: Table 1. He states the recommended values' uncertainty as
$\pm 2 \times 10^{-3}$, and prints a fourth decimal "merely for the purpose of tabular smoothness"
(his Section 3.1 and Table 1's note). Silicon absorbs above its band gap, 1.12 eV at 300 K or
1.1071 µm (his Section 3.1), and the table starts at 1.2 µm.

photonoxide takes the 293 K column, 35 values from 1.2 to 14 µm, and draws a natural cubic spline
through them: a cubic between each pair of values, with continuous slope and curvature, and no
curvature at the ends. It passes through every value exactly, and through points on a line it is
that line:

::validation material/silicon-li-table material/spline-line

[silicon_index](../examples/silicon_index.rs) prints five of them:

::example silicon_index

At 1.55 µm $n = 3.4757$ and $n_g = 3.5993$: the group index is 0.124 above the index, where
silica's is 0.019 above.

::chart material-index{material=2}

[Frey, Leviton and Madison (2006)](https://doi.org/10.1117/12.672850) measured silicon's absolute
index from 1.1 to 5.6 µm and 20 to 300 K to about $1 \times 10^{-4}$: 3.48314 at 1.5 µm and 295 K,
against Li's 3.4799 at 293 K. Their $dn/dT$, $1.87 \times 10^{-4}$ per kelvin there (their Table 4),
accounts for $3.7 \times 10^{-4}$ of the $3.2 \times 10^{-3}$, and the rest lies just beyond Li's
$\pm 2 \times 10^{-3}$. They found the published indices of silicon to vary widely, and advise
measuring a prism from the same batch as the optics whenever an absolute index better than
$\pm 5 \times 10^{-3}$ is needed. Silicon's $dn/dT$ is about 19 times silica's.

**Try it.** Move the wavelength to 1.1 µm. What does the chart give, and why?

:::answer
No index: 1.1 µm is outside Li's range, 1.2 to 14 µm, and the catalogue refuses it. Light of
1.1 µm has more energy than silicon's band gap, at 1.107 µm, and the crystal absorbs it; at 1.2 µm, the table's
first value, $n = 3.5167$ and $n_g = 3.7252$.
:::

## 5. What do the group index and group-velocity dispersion of a material measure?

The index sets the speed of a wave's phase. A pulse's envelope travels at the group velocity,
$c/n_g$, and a ring's free spectral range and an interferometer's fringes are set by it too.
How fast $n_g$ changes with the wavelength spreads a pulse: the dispersion parameter $D$, the
change of the group delay per unit length per unit of wavelength, in ps/(nm km). Positive $D$ is
anomalous dispersion, the longer wavelengths arriving first; negative is normal.

At 1.55 µm, silica has $D = 21.91$ ps/(nm km), and its $D$ crosses zero at 1.2728 µm (exact, from
Malitson's formula):

::chart material-gvd{material=1}

Silicon's $D$ is negative, several hundred ps/(nm km), but how negative, Li's table can't tell:
the curve zig-zags, $-717$, $-945$ and $-447$ ps/(nm km) at 1.50, 1.55 and 1.60 µm.

::chart material-gvd{material=2}

### Group index and dispersion from $n(\lambda)$ {theory}

A wave of frequency $\omega$ has $k = n\omega/c$, and a pulse's envelope travels at
$v_g = d\omega/dk$. With $\omega = 2\pi c/\lambda$, $\omega\thinspace dn/d\omega = -\lambda\thinspace dn/d\lambda$, so

$$
\frac{c}{v_g} = c\frac{dk}{d\omega} = n + \omega\frac{dn}{d\omega} = n - \lambda\frac{dn}{d\lambda} = n_g ,
$$

Chrostowski and Hochberg's Eq. 3.5 for a mode's effective index
([2015](https://doi.org/10.1017/CBO9781316084168)), here for the bulk. The group delay per unit
length is $n_g/c$, and $dn_g/d\lambda = dn/d\lambda - dn/d\lambda - \lambda\thinspace d^2n/d\lambda^2$, so

$$
D = \frac{1}{c}\frac{dn_g}{d\lambda} = -\frac{\lambda}{c}\frac{d^2 n}{d\lambda^2} ,
$$

their Eq. 3.6. $n_g$ needs the first derivative of the index, $D$ the second. photonoxide's
`Material::group_index` takes the slope of each model, and the chart takes $D$ by
`mode::dispersion::dispersion`, three-point differences of the index 2 nm apart
([dispersion](../docs/methods/dispersion.md)).

### Why silicon's curve zig-zags {theory}

The spline's curvature is a straight line between two of Li's values, so $D$ from it is a broken
line with corners at the values, 0.05 µm apart near 1.55 µm. The values are rounded to
$10^{-4}$, an error of up to $5 \times 10^{-5}$ in each, and a second difference over
$h = 0.05$ µm magnifies it to up to $4 \times 5 \times 10^{-5}/h^2 = 0.08$ per µm²: at 1.55 µm,
up to 410 ps/(nm km) in $D$, as large as the scatter the chart shows. The group index, a first
derivative, is held far better, to about $2 \times 5 \times 10^{-5}/h = 2 \times 10^{-3}$ in
$dn/d\lambda$ and $3 \times 10^{-3}$ in $n_g$. From Frey et al.'s measured slopes, $-0.11583$ per µm
at 1.4 µm and $-0.09229$ at 1.5 µm at 295 K (their Table 3), the difference gives
$d^2n/d\lambda^2 \approx 0.235$ per µm² and $D \approx -1140$ ps/(nm km) near 1.45 µm, where the
spline gives $-996$. A table fixes $n$ and $n_g$; for $D$ a smooth formula fitted to the
measurements is needed, which the catalogue doesn't yet have for silicon.

**Try it.** Where is silica's $D$ largest in the chart's window, and what does that mean for a
pulse?

:::answer
At the long end: 47.38 ps/(nm km) at 2.0 µm, and still rising towards 2.45 µm. Positive $D$ is
anomalous: the longer wavelengths of a pulse travel faster. In a waveguide the guide's own
dispersion adds to the material's: the 500 × 220 nm silicon strip has
$D = +469$ ps/(nm km), anomalous, although bulk silicon's is normal
([the strip waveguide](strip-waveguide.md), Section 7).
:::

## 6. What does photonoxide's catalogue record for each material?

Each `Material` is a model with the range of wavelengths it is valid for and its provenance: the
paper and its DOI, where the numbers were taken from and under which licence, the temperature,
and notes such as the sample measured. Silicon, silica and silicon nitride were taken from the
refractiveindex.info database, in the public domain (CC0)
([Polyanskiy, 2024](https://doi.org/10.1038/s41597-023-02898-2)), and each was checked against its
paper ([materials](../docs/methods/materials.md)). The catalogue adds, for each material, every
index model it has, its coefficients as the paper prints them, the accuracy the paper states, and
the temperature or composition a model takes ([the materials catalogue](../docs/methods/catalogue.md)).

A wavelength outside a model's range is an error, `Error::OutsideValidity`, not an extrapolation.
Section 2 showed a fit leaving its data; Malitson's formula has no measurement under it from 2.4
to 3.2 µm; Li's table stops at silicon's absorption edge. Within the range a model is held to its
paper; beyond it the library has nothing it can vouch for.

Two silica models show what the provenance is for. Malitson's index is relative to air.
[Leviton and Frey (2006)](https://doi.org/10.1117/12.672853) measured Corning 7980 in vacuum, the
absolute index, from 0.4 to 2.6 µm and 30 to 310 K, and found Malitson's formula, corrected to
vacuum with an index of air of 1.00027, within $1 \times 10^{-5}$ of theirs below 2 µm. The chart's
fourth material is their model at 295 K: $4.07 \times 10^{-4}$ above Malitson's at 1.55 µm, of which
the air accounts for $3.90 \times 10^{-4}$ and the 1.85 K between 20 °C and 295 K, at his
$10^{-5}$ per kelvin, for the rest. Their temperature-dependent Sellmeier fit reproduces their
tabulated indices:

::validation material/silica-leviton-table

::chart material-index{material=4}

The third material is Luke et al.'s silicon nitride, for a 340 nm LPCVD film annealed at 1200 °C;
the paper states no temperature, and the record says so. Its second term has $B = 40314$ and
$C = 1239.842$ µm, a resonance at no physical wavelength: inside its range of 0.31 to 5.504 µm it
acts as $-(B/C^2)\lambda^2 = -0.0262\thinspace\lambda^2$ (µm), the long-wavelength term of
Section 2.

**Try it.** Choose the nitride. What are its index and group index at 1.55 µm, and how do they
compare with silicon's and silica's?

:::answer
$n = 1.99628$ and $n_g = 2.0396$: between silica's 1.444024 and 1.4626 and silicon's 3.4757 and
3.5993. The contrast to the oxide, 0.55, is about a quarter of silicon's 2.03, the reason a nitride
guide's mode reaches further into its cladding ([the slab waveguide](slab-waveguide.md),
Section 7).
:::

## 7. History and current research {research}

[Sellmeier (1871)](https://doi.org/10.1002/andp.18712190612) tied dispersion to absorption: the
particles of a material, set vibrating by the light, change its index most near the periods they
absorb, which explained the anomalous dispersion seen in fuchsine. Modelled as bound electrons,
his oscillators are the Lorentz model of Section 1, and with the spring removed
[Drude (1900)](https://doi.org/10.1002/andp.19003060312) described the free electrons of metals.
[Malitson (1965)](https://doi.org/10.1364/JOSA.55.001205) measured fused silica from three makers
and fitted the three-term formula the catalogue uses. [Li (1980)](https://doi.org/10.1063/1.555624)
compiled every measurement of silicon and germanium and recommended tables with stated
uncertainties. [Tatian (1984)](https://doi.org/10.1364/AO.23.004477) showed how to fit Sellmeier
formulas reliably and how far they hold.

::timeline

Today's measurements reach cryogenic temperatures and the absolute index:
[Leviton and Frey (2006)](https://doi.org/10.1117/12.672853) for fused silica to about $10^{-5}$,
[Frey et al. (2006)](https://doi.org/10.1117/12.672850) for silicon to about $10^{-4}$, for optics
cooled in space telescopes. New platforms need their materials measured anew and over wider bands:
[Luke et al. (2015)](https://doi.org/10.1364/OL.40.004823) measured silicon nitride from the
ultraviolet to 33 µm to design mid-infrared combs. Open databases such as refractiveindex.info
([Polyanskiy, 2024](https://doi.org/10.1038/s41597-023-02898-2)) collect such data with their
sources; a design still has to check each against its paper, its range and its temperature, as
the catalogue does. The open problems are the ones Sections 4 and 5 met: a material's index
varies from batch to batch by more than the measurements' uncertainty, and the second derivative
that sets dispersion needs data denser and smoother than most tables.

Further reading: [Malitson (1965)](https://doi.org/10.1364/JOSA.55.001205), five pages, for how a
material's index is measured and fitted; [Li (1980)](https://doi.org/10.1063/1.555624),
Sections 2 and 3, for the theory and the judging of data; and
[Tatian (1984)](https://doi.org/10.1364/AO.23.004477) for fitting.

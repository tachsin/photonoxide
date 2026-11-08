---
title: "Lamellar gratings, rigorous coupled-wave analysis"
module: mode::rcwa
summary: "A plane wave's diffraction by a stack of one-dimensional lamellar gratings and uniform films, TE and TM, lossy or not: the Fourier modal method with Li's factorization rules, joined by scattering matrices."
order: 4.5
papers:
  - cite: "M. G. Moharam, T. K. Gaylord, J. Opt. Soc. Am. 71, 811 (1981) (rigorous coupled-wave analysis of planar gratings; not read here, cited for the method's name and origin)"
    doi: 10.1364/JOSA.71.000811
  - cite: "M. G. Moharam, E. B. Grann, D. A. Pommet, T. K. Gaylord, J. Opt. Soc. Am. A 12, 1068 (1995) (a stable formulation for binary gratings; not read here, its TM form as Li's Eq. 5 quotes it)"
    doi: 10.1364/JOSAA.12.001068
  - cite: "M. G. Moharam, D. A. Pommet, E. B. Grann, T. K. Gaylord, J. Opt. Soc. Am. A 12, 1077 (1995) (the enhanced transmittance matrix; not read here)"
    doi: 10.1364/JOSAA.12.001077
  - cite: "P. Lalanne, G. M. Morris, J. Opt. Soc. Am. A 13, 779 (1996) (the reformulation that speeds up TM; not read here, its matrices as Li's Eqs. 3–4 give them)"
    doi: 10.1364/JOSAA.13.000779
  - cite: "G. Granet, B. Guizal, J. Opt. Soc. Am. A 13, 1019 (1996) (the same reformulation, for metallic gratings; not read here)"
    doi: 10.1364/JOSAA.13.001019
  - cite: "L. Li, J. Opt. Soc. Am. A 13, 1024 (1996) (recursive scattering-matrix algorithms for layered gratings; not read here, cited for the S-matrix algorithm)"
    doi: 10.1364/JOSAA.13.001024
  - cite: "L. Li, J. Opt. Soc. Am. A 13, 1870 (1996) (the factorization rules: Theorems 1–3, Eqs. 3–5, 22)"
    doi: 10.1364/JOSAA.13.001870
  - cite: "P. Lalanne, S. Astilean, P. Chavel, E. Cambril, H. Launois, Opt. Lett. 23, 1081 (1998) (an échelette grating's 66.5 % in its first order, p. 1083)"
    doi: 10.1364/OL.23.001081
  - cite: "P. Lalanne, S. Astilean, P. Chavel, E. Cambril, H. Launois, J. Opt. Soc. Am. A 16, 1143 (1999) (blazed gratings by RCWA as staircases of 15 lamellar layers, Section 1)"
    doi: 10.1364/JOSAA.16.001143
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the transfer matrices uniform layers are checked against)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - mode/rcwa-uniform-multilayer
  - mode/rcwa-energy-lossless
  - mode/rcwa-effective-medium-order
  - mode/rcwa-tm-convergence
---

A grating here is a stack of layers between a semi-infinite cover and substrate. Each layer is
uniform, or lamellar: stripes of complex index $n + i\kappa$ across one period $\Lambda$ along x,
the same along y and through the layer's thickness along z. A plane wave of vacuum wavelength λ
comes from the cover in the xz plane at angle θ, TE (E along y, along the stripes) or TM (H along
y). The time dependence is $e^{-i\omega t}$, so $\kappa \ge 0$ is loss, as everywhere in
photonoxide. A profile that isn't lamellar, a sawtooth say, is a staircase of lamellar layers.

## The modes of a layer

In a layer the fields are Floquet series truncated to $\lvert m \rvert \le M$ (2M + 1 harmonics):

$$
\psi(x, z) = \sum_{m=-M}^{M} S_m(z)\thinspace e^{i k_m x}, \qquad
k_m = k_0 n_c \sin\theta + \frac{2\pi m}{\Lambda},
$$

with ψ the field along y (E for TE, H for TM). In units of $k_0 = 2\pi/\lambda$, $K_x =
\operatorname{diag}(k_m/k_0)$ and $z' = k_0 z$. Write $E = [\![\varepsilon]\!]$ and $A =
[\![1/\varepsilon]\!]$ for the Toeplitz matrices whose (m, n) entries are the Fourier
coefficients $\varepsilon_{m-n}$ and $(1/\varepsilon)_{m-n}$; for stripes of permittivity
$\varepsilon_j$ between $x_j$ and $x_{j+1}$,

$$
\varepsilon_h = \frac{1}{\Lambda}\int_0^\Lambda \varepsilon(x)\thinspace e^{-i 2\pi h x/\Lambda} dx
= \sum_j \varepsilon_j \thinspace \frac{e^{-i 2\pi h x_j/\Lambda} - e^{-i 2\pi h x_{j+1}/\Lambda}}{i 2\pi h} .
$$

**TE.** $E_y$ is tangential to every wall, so the product $\varepsilon E_y$ has no jump that
$E_y$ doesn't share, and Laurent's rule (the product's coefficients as the convolution of the
factors') holds: Li's Theorem 1 (Li 1996, p. 1872). The wave equation becomes

$$
\frac{d^2 S}{dz'^2} = (K_x^2 - E)\thinspace S .
$$

**TM.** From Maxwell's equations, $E_x = (1/i\omega\varepsilon_0\varepsilon)\thinspace \partial_z H_y$
and $E_z = -(1/i\omega\varepsilon_0\varepsilon)\thinspace \partial_x H_y$. Across a wall, $E_z$ is
tangential and continuous, but ε and $\partial_x H_y$ both jump: their jumps are complementary,
and Li's Theorem 3 says such a product converges only by the **inverse rule**, its coefficients
from $[\![\varepsilon]\!]^{-1}$ (his Eq. 22). $D_x = \varepsilon E_x$ is continuous, so
$E_x = (1/\varepsilon) D_x$ is a product with one continuous factor, Laurent's rule's: $A$. So

$$
\frac{d^2 H}{dz'^2} = A^{-1}\left(K_x E^{-1} K_x - I\right) H ,
$$

Li's Eq. 4, the form Lalanne and Morris and Granet and Guizal found in 1996 and Li explained
(his Section 3). The older form, $E(K_x E^{-1} K_x - I)$ (Li's Eq. 5, Moharam et al.'s of 1995),
uses Laurent's rule where the inverse rule belongs; Li's Theorem 2 shows its error at each wall
doesn't vanish as M grows (p. 1872). It is kept in the code to show the difference.

The eigenvectors $W$ and eigenvalues $q^2$ of the layer's matrix are its modes, $\psi = W(e^{-Qz'}
c^+ + e^{Q(z'-d')} c^-)$, $Q = \operatorname{diag}(q)$, $q = -ik_z$. Each $q$ takes the root that
decays downward, $\operatorname{Re} q \gt 0$, or for a propagating mode (Re q zero to rounding)
the one that travels downward. The tangential fields' coefficients are

$$
\begin{pmatrix} \psi \cr \phi \end{pmatrix} = \begin{pmatrix} W & W \cr -V & V \end{pmatrix}
\begin{pmatrix} c^+ \cr X c^- \end{pmatrix}, \qquad
V = WQ \text{ (TE)}, \quad V = AWQ \text{ (TM)}, \quad X = e^{-Qd'} ,
$$

where φ is $\partial\psi/\partial z'$ (TE, the tangential H) or $A\thinspace\partial H/\partial z'$
(TM, the tangential E). A uniform layer, the cover and the substrate have the plane waves for
modes: $W = I$, $q_m^2 = k_m^2/k_0^2 - \varepsilon$, $V = Q$ or $Q/\varepsilon$. A uniform layer
is therefore exact at any M.

## The stack: scattering matrices

Matching ψ and φ at the interface between layers 1 (above) and 2 gives, with $a = W_1^{-1}W_2$ and
$b = V_1^{-1}V_2$, the upper layer's amplitudes from the lower's through $T_{11} = T_{22} = (a +
b)/2$ and $T_{12} = T_{21} = (a - b)/2$. Solved for the waves that leave the interface, its
scattering matrix is

$$
S_{21} = T_{11}^{-1}, \quad S_{22} = -T_{11}^{-1}T_{12}, \quad S_{11} = T_{12}T_{11}^{-1}, \quad
S_{12} = T_{11} - T_{12}T_{11}^{-1}T_{12} ,
$$

($S_{11}$ reflects from above, $S_{21}$ transmits down). A layer's own matrix carries each mode
across it, $X$ both ways. The stack's is the Redheffer star product of the interfaces and layers
in order; with $M = (I - A_{22}B_{11})^{-1}$, A above B,

$$
S_{11} = A_{11} + A_{12}B_{11}MA_{21}, \quad S_{21} = B_{21}MA_{21}, \quad
S_{12} = A_{12}(B_{12} + B_{11}MA_{22}B_{12}), \quad S_{22} = B_{22} + B_{21}MA_{22}B_{12} .
$$

Only the decaying $e^{-qd'}$ appear, never their inverses, so a deep layer or an absorbing one
doesn't overflow: the stability Moharam et al.'s enhanced transmittance matrix (1995) and Li's
recursive S-matrix algorithm (1996, JOSA A 13, 1024) were made for. This page cites those two for
the idea only; the algebra above is derived here.

## Efficiencies

The incident wave is the zeroth harmonic, amplitude 1. The cover's reflected amplitudes are
$r = S_{11}\delta_0$ and the substrate's transmitted ones $t = S_{21}\delta_0$, of the tangential
E (TE) or H (TM). With $k_{z,m} = i q_m$, the flux of each order gives its share of the incident
power:

$$
R_m = \lvert r_m \rvert^2 \frac{\operatorname{Re} k^c_{z,m}}{k^c_{z,0}}, \quad
T_m = \lvert t_m \rvert^2 \frac{\operatorname{Re} k^s_{z,m}}{k^c_{z,0}} \text{ (TE)}; \qquad
\text{TM: } k_z/\varepsilon \text{ in place of } k_z .
$$

What is neither reflected nor transmitted is absorbed, $A = 1 - \sum R_m - \sum T_m$. An order
exactly at grazing (a Wood anomaly) makes its plane wave's V singular; the solver says so.

## Validation

- **Uniform layers** (`mode/rcwa-uniform-multilayer`): three films, one lossy, TE and TM at two
  angles; $r$ and $t$ agree with [the transfer matrices](multilayer.md) to round-off.
- **Energy** (`mode/rcwa-energy-lossless`): lossless silicon gratings, one layer and an
  off-centre staircase, TE and TM at two angles, 41 harmonics: $1 - R - T$ is zero to round-off.
- **The static limit** (`mode/rcwa-effective-medium-order`): a silicon-in-air grating 300 nm thick
  at 1.55 µm tends, as Λ → 0, to a uniform film of $\varepsilon_\parallel = f\varepsilon_1 + (1 -
  f)\varepsilon_2$ for TE and $1/\varepsilon_\perp = f/\varepsilon_1 + (1 - f)/\varepsilon_2$ for
  TM (form birefringence). TE's zeroth-order $t$ departs from the film's as $\Lambda^2$: halving
  the period from 10 to 5 nm divides the difference by 3.98 (order 1.996). That departure grows with the
  thickness, in proportion from 0.3 µm on (a 0.15, 0.3, 0.6, 1.2 µm layer in air at Λ = 10 nm:
  the phase of $t/t_\text{film}$ off by 0.85, 3.6, 7.3, 14.5 × 10⁻⁴), a correction to the medium's index. TM's
  departs as Λ (order 1.03), and it hardly depends on the thickness (2.4–3.7 × 10⁻³ over the same
  layers): the normal E at the layer's two faces drives evanescent orders that reach about Λ/2π
  into it and out of it, a skin whose share of the layer is first order in Λ. The static limit of
  a layer thinner than its period is therefore not the film of $\varepsilon_\perp$ for TM.
- **TM's convergence** (`mode/rcwa-tm-convergence`): a deep silicon grating (500 nm period, fill
  0.5, 600 nm thick, at 1 µm). With Laurent's rule where the inverse rule belongs (Li's Eq. 5) the
  zeroth-order transmittance converges at first order in 1/M: from 201 to 401 to 801 harmonics it
  moves by 9.29 × 10⁻⁴ and then 4.66 × 10⁻⁴ (order 0.995), and at 81 harmonics it is still
  4.6 × 10⁻³ low. Extrapolated at that order, its limit, 0.9180784, meets Li's rules at 801
  harmonics to 7 × 10⁻⁸; Li's rules are within 1.8 × 10⁻⁵ at 51 harmonics. TE on the same grating
  converges at third order (its steps fall eightfold per doubling). Li's rules leave a slow tail of
  their own, about 10⁻⁶ per doubling beyond 100 harmonics: the field's singularity at the stripes'
  corners, which no factorization rule removes.

**Lalanne et al.'s échelette** (not a case). The 1998 letter (p. 1083) gives 66.5 % as the best
first-order efficiency of an ideal échelette (sawtooth) grating in glass (1.52) with a 3λ period,
for unpolarized light at normal incidence from air, its depth "slightly larger than λ/(n − 1)",
computed with electromagnetic theory; their 1999 paper computes blazed gratings as staircases of
15 lamellar layers (Section 1). Ours, at the letter's 1.9 µm and 633 nm (3.0016λ), with the mean of
TE and TM and 61 harmonics, the depth optimized: 66.76 % with 15 layers, 66.48 % with 30, 66.44 %
with 60 and 66.43 % with 120 (depth 1.120 λ/(n − 1)), so the sawtooth itself is 66.43 %, 0.07
points below theirs. The number is sensitive to the period: the third orders graze the air at
exactly 3λ (a Wood anomaly), and there the efficiency changes as the square root of the detuning.
At 2.9999λ and 3.0001λ the 60-layer staircase gives 66.18 % and 66.26 %, the 15-layer one 66.52 %
and 66.59 %. So the staircase's resolution and the period's last digit each move the efficiency
by a few tenths of a point, and the letter states neither; 66.5 % lies inside that range, but no
single reading of it reproduces the printed digits, so it is not a case.

The figures above that aren't cases come from ignored tests,
`mode::rcwa_checks::tests::print_tm_convergence` and `print_tails`.

## Limits

- One-dimensional gratings in the classical mount (the plane of incidence across the stripes);
  no conical incidence and no crossed (two-dimensional) gratings.
- The absorption is the stack's total; a layer's own share is not separated.
- TM converges more slowly than TE at stripes' corners (see above); check a result at two values
  of M.

---
title: "High-accuracy modes: Hadley's interface and corner equations"
module: mode::hadley
summary: "Full-vector modes from finite-difference equations built on the exact local solutions: sixth order at interfaces, about second order at dielectric corners, where the standard scheme manages about first."
order: 18
papers:
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1210 (2002), part I: uniform regions and dielectric interfaces"
    doi: 10.1109/JLT.2002.800361
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), part II: dielectric corners"
    doi: 10.1109/JLT.2002.800371
  - cite: "S. Güttel, F. Tisseur, Acta Numerica 26, 1 (2017) (nonlinear inverse iteration)"
    doi: 10.1017/S0962492917000034
validation:
  - mode/hadley-uniform-box
  - mode/hadley-uniform-order
  - mode/hadley-interface
  - mode/hadley-interface-order
  - mode/hadley-interface-turned
  - mode/hadley-corners-box-low
  - mode/hadley-corners-box-high
  - mode/hadley-corners-impinged-low
  - mode/hadley-corners-impinged-high
  - mode/hadley-corners-order
examples:
  - hadley_corners
---

The [standard full-vector scheme](vector.md) converges at second order where interfaces are
straight, but only at about first order at a dielectric corner, where the field's derivatives
are singular. Hadley's equations keep the same unknowns and the same nine-point stencil, and
build each node's equation from the exact local solutions of the Helmholtz equation instead
of Taylor series. `mode::hadley::modes` takes the same `CrossSection` as
`mode::vector::modes` and returns the same `VectorMode`s.

## The method

In a uniform region every Cartesian component of H solves
$\nabla^2 H + k^2(\varepsilon - \bar\varepsilon) H = 0$, with $\bar\varepsilon = n_\text{eff}^2$
(Hadley I, Eq. 1), so near a node

$$
H(r, \theta) = \sum_n J_n(\xi r)\thinspace(c_n \cos n\theta + d_n \sin n\theta),
\qquad \xi^2 = k^2(\varepsilon - \bar\varepsilon)
$$

(Eqs. 2–3). Each node's equation is the combination of its nine stencil values that cancels
as many of the unknown $c_n$, $d_n$ as possible. The four cells around a node decide which
equation it takes:

- **Uniform** (all four alike): Eq. (7), with A and B from Eqs. (8)–(9) cancelling $c_2$ and
  $c_4$. Truncation error sixth order on a square grid, fourth otherwise.
- **Interface** (two pairs alike): separate expansions on each side, joined by the interface
  conditions. The component normal to the interface keeps H and its normal derivative
  continuous: Eq. (20), with $A_1, \ldots, A_4$ solving Eqs. (21)–(24), fifth order. The tangential
  one has a jump in its normal derivative set by the other component (Eq. 25,
  $\tfrac{1}{\varepsilon_1}\partial_y H^x\vert_+ - \tfrac{1}{\varepsilon_2}\partial_y H^x\vert_- = (\tfrac{1}{\varepsilon_1} - \tfrac{1}{\varepsilon_2})\thinspace\partial_x H^y$):
  Eq. (43), with the A's from Eqs. (31), (32), (23), (24) and terms on the normal component
  through $f_1$, $f_2$ and $g_1, \ldots, g_6$. Hadley writes them for a horizontal interface; a
  vertical one is the same with x and y, and $H^x$ and $H^y$, exchanged (Eq. 25 is symmetric
  under that exchange).
- **Corner** (anything else; part II, Fig. 3, quadrants $\varepsilon_1$ to $\varepsilon_4$
  counterclockwise from the north-east): no power series satisfies all four interfaces. Part II
  adds Bessel functions of fractional order ν and μ, the two roots of
  $\sin^2(\pi\nu/2) = -\tfrac14(\varepsilon_{12} + \varepsilon_{34})(\varepsilon_{23} + \varepsilon_{41})$,
  $\varepsilon_{ij} = (\varepsilon_i - \varepsilon_j)/(\varepsilon_i + \varepsilon_j)$ (Eq. 36), in [5/3, 7/3].
  It also adds their derivatives with respect to the order, which carry $\ln r$ (Eqs. 29–33).
  The equations are Eq. (50) for $H^y$ and Eq. (52) for $H^x$, built from the operators of
  Eqs. (40)–(44), with $A_1, \ldots, A_3$ and $B_1, \ldots, B_3$ from the Appendix, Eqs. (A1)–(A14). They
  are first order at the corner itself.
- **A misprint:** Eqs. (50) and (52) print the term
  $(\theta\sin\theta + \cos 2\theta \ln\sin\theta)$. It is there to cancel $A_2$ (or $B_2$)
  times the same term of Eq. (47), which prints it as
  $(\theta\sin 2\theta + \cos 2\theta\ln\sin\theta)$. Eq. (47)'s form is right. The term comes
  from the second-order logarithmic function $r^2(\ln r\cos 2\theta - \theta\sin 2\theta)$
  (Eq. 24 with ν = 2). Exchanging x and y ($\theta \to \pi/2 - \theta$) turns it into the
  companion term $((\pi/2 - \theta)\sin 2\theta - \cos 2\theta\ln\cos\theta)$, which
  Eqs. (48), (50) and (52) all print. Taking the printed form instead changes the corner
  problems' errors by up to a third, not their order. Separately, Eq. (48)'s fractional terms
  lack the factors $\sin(\pi\nu/2)$ and $\sin(\pi\mu/2)$ that Eq. (46) and the Appendix's B and D
  (A4, A6) carry. The solver uses the Appendix, so this doesn't affect it.

The coefficients depend on $\bar\varepsilon$ through ξ, so the eigenproblem
$M(\bar\varepsilon)\thinspace h = 0$ is nonlinear. It is solved by nonlinear inverse iteration (Güttel
and Tisseur, Algorithm 4.7: Newton's method on $M(\bar\varepsilon)h = 0$ with a normalization,
Eqs. 4.15–4.16). Each step solves $M(\bar\varepsilon_k)\thinspace w = M'(\bar\varepsilon_k)\thinspace v_k$ and sets
$\bar\varepsilon_{k+1} = \bar\varepsilon_k - u^H v_k / u^H w$, with $M'$ by central differences.
It starts from the standard scheme's mode on the same grid and converges quadratically:
two or three steps to 1e-13 on the corner problems.

The Bessel functions take arguments $|\xi| r$ of a few units at most; their power series is
accurate there and has no cancellation when ξ is imaginary (ε < ε̄). Grids whose arguments
exceed 12 are refused.

## Scope

Hadley's derivation covers a **uniform grid** along each axis (Δx and Δy may differ),
**real isotropic** media, and no PML; `modes` checks all three. Mirror walls and zero
boundaries work as in the standard solver. The corner equation is undefined where
$\varepsilon_1\varepsilon_3 = \varepsilon_2\varepsilon_4$: there ν = μ = 2, and the Appendix's
denominators vanish. Hadley shows that no true corner satisfies this together with
$\varepsilon_2 + \varepsilon_4 = \varepsilon_1 + \varepsilon_3$ (after Eq. 39). Corners of
three media can still meet it, and the solver then returns an error.

## Accuracy

- **Uniform:** Hadley I's box (his Fig. 5: n = 3.44, 2 × 2 µm, 1.15 µm) converges at sixth
  order onto its exact index: relative errors 4.9e-9, 7.1e-11, 1.1e-12, 1.7e-14 at 4, 8, 16, 32
  cells per side. His slope is 6.03. The standard scheme's error at 8 × 8 is 4.8e-5.
- **Interfaces:** Hadley I's two-dielectric box (his Fig. 6) is separable, so its index is
  exact. It converges at sixth order, his "apparent sixth order": relative errors 7.6e-5,
  1.6e-6, 2.8e-8, 4.8e-10 at 250, 125, 62.5 and 31.25 nm for ε = 1 over 11.8336, and 6.0e-6,
  9.0e-8, 1.4e-9, 2.2e-11, 3.5e-13 from 500 to 31.25 nm for 10.5 over 10.89. The same box on
  its side gives the same index to the last digit. Applied to exact local solutions, the
  stencils' residuals fall as h⁷ (normal component) and h⁵ (tangential), as the derivation says.
- **Corners:** Hadley II's four problems (his Figs. 4–7, indices from series expansions good to
  1e-8), relative errors:

| Grid | Box, ε 2.25 | | Box, ε 8 | | Impinged, ε 2.25 | | Impinged, ε 8 | |
|---|---|---|---|---|---|---|---|---|
| | standard | Hadley | standard | Hadley | standard | Hadley | standard | Hadley |
| 8 × 8 (125 nm) | −1.4e-3 | 1.5e-4 | −1.1e-3 | 1.2e-5 | −6.5e-4 | 2.8e-5 | −6.7e-4 | 6.1e-5 |
| 16 × 16 (62.5 nm) | −2.2e-4 | 3.2e-5 | −3.2e-4 | 5.1e-6 | −2.0e-4 | −3.7e-6 | −2.0e-4 | 4.9e-6 |
| 32 × 32 (31.25 nm) | 1.2e-5 | 7.1e-6 | −7.1e-5 | 1.3e-6 | −6.8e-5 | −2.1e-6 | −5.9e-5 | 4.7e-7 |
| 64 × 64 (15.6 nm) | 4.0e-5 | 1.7e-6 | −8.6e-6 | 3.2e-7 | −2.5e-5 | −6.7e-7 | −1.8e-5 | 1.2e-7 |
| 128 × 128 (7.8 nm) | 3.0e-5 | 4.1e-7 | 3.4e-6 | 8.5e-8 | −9.9e-6 | −1.8e-7 | −5.6e-6 | 6.8e-8 |

  Hadley's equations converge at about **second order**: 2.2, 2.2, 2.1, 2.0 on the
  low-contrast box; 1.3, 2.0, 2.0, 1.9 on the high-contrast one; 0.8, 1.7, 1.9 on the
  low-contrast impinged corner, after a sign change; 3.6, 3.4, 1.9, then 0.9 on the
  high-contrast one, whose error levels off near 6e-8 (5.7e-8 at 160 × 160). Hadley's own Fig. 11 levels off the same way,
  and he reports second rather than third order "for most cases". The standard scheme's error
  changes sign on the boxes and converges at 1.3–1.8 on the impinged corners. At 128 × 128,
  Hadley's equations are 40 to 80 times more accurate.

  The errors follow Hadley's "Full Model" curves (Figs. 8–11, read off the log plots): about
  5e-6 at 16 points per axis in Figs. 9 and 11, about 1e-7 at 64 in Fig. 11. His text gives
  better than 2e-4 at 8 × 8 (ours: 1.5e-4 at most) and below 1e-7 on his finest grid in three
  of four cases. At 160 × 160 ours are 2.6e-7, 5.6e-8, 1.2e-7 and 5.7e-8: two of four.

- **The book's strip** (500 × 220 nm, 3.473 in 1.444, 1550 nm, as on the
  [standard solver's page](vector.md)): TE-like 2.442378, 2.442219, 2.442179 at 20, 10 and
  5 nm, converging at second order. The standard scheme gives 2.447067, 2.445713 and 2.444396
  there, and 2.443506 at 2.5 nm.

## Validation

- Hadley I's uniform box against its exact index, and the sixth order.
- Hadley I's two-dielectric box against its exact (separable) index, the sixth order, and
  the box turned on its side.
- Hadley II's four corner problems within 1e-6 at 128 × 128, and the second order on the first.
- Unit tests: each stencil applied to exact local solutions (Bessel series joined across the
  interface by its conditions) leaves a residual of the derivation's order. Flipping the sign
  of Eq. (43)'s coupling, or dropping it, makes the test fail.

---
title: "Every mode in a region, by contour integrals"
module: mode::region
summary: "Every mode whose effective index lies in a disc or an ellipse of the complex plane, with no count and no guess: FEAST's subspace iteration on a contour integral, its quadrature points solved independently and side by side."
order: 6.5
papers:
  - cite: "T. Sakurai, H. Sugiura, J. Comput. Appl. Math. 159, 119 (2003) (the projection by numerical integration, the trapezoidal rule on a circle)"
    doi: 10.1016/S0377-0427(03)00565-X
  - cite: "E. Polizzi, Phys. Rev. B 79, 115112 (2009) (FEAST: subspace iteration with Rayleigh–Ritz on the filtered block)"
    doi: 10.1103/PhysRevB.79.115112
  - cite: "J. Kestyn, E. Polizzi, P. T. P. Tang, SIAM J. Sci. Comput. 38, S772 (2016) (non-Hermitian FEAST: leaky and lossy modes)"
    doi: 10.1137/15M1026572
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the modes reproduced)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - mode/contour-uniform-box
  - mode/contour-dense
  - mode/contour-strip-shift-invert
  - mode/contour-leaky-shift-invert
  - mode/contour-chilwell-bound
  - mode/contour-chilwell-leaky
  - mode/contour-exponential
  - mode/contour-threads
  - mode/contour-sakurai-sugiura
---

[Shift-and-invert Arnoldi](eigen.md) finds a given number of modes nearest a guess. Often the
question is different: every guided mode of a wide guide, every mode above an index, every
leaky wave in some part of the complex plane. `mode::vector::modes_in` and
`slab_fd::Profile::modes_in` answer it. They take a `Region` of effective indices, a disc or an
ellipse, and return every mode inside with no count and no guess:

```rust
use photonoxide::mode::region::{Region, Search};
use photonoxide::mode::vector;

let found = vector::modes_in(&cross_section, wavelength, &Region::between(1.5, 3.4)?, &Search::default())?;
for (mode, residual) in found.modes.iter().zip(&found.residuals) {
    println!("{} ({residual:.1e})", mode.effective_index());
}
```

## The method

**The projector.** For a closed curve Γ around some eigenvalues of the pencil $A x = \lambda B x$,

$$
P = \frac{1}{2\pi i}\oint_\Gamma (zB - A)^{-1} B \thinspace dz
$$

projects onto their eigenvectors (Kestyn et al., Eq. 2.4). Applied to a block Y of random
vectors, at least as many as the eigenvalues inside, P Y spans them.

**The quadrature.** The integral is taken by the N-point trapezoidal rule in the curve's
parameter (Sakurai and Sugiura, Section 3), so P becomes a rational filter, one shifted solve per
point:

$$
P Y \approx \sum_{j=1}^{N} w_j (z_j B - A)^{-1} B Y,
\qquad
\rho(\lambda) = \sum_{j=1}^{N} \frac{w_j}{z_j - \lambda} \approx
\begin{cases} 1 & \text{inside,} \cr 0 & \text{outside.} \end{cases}
$$

For an integrand analytic near the curve the rule converges exponentially in N: an eigenvalue
outside is damped by about (its distance in units of the curve's size)$^{-N}$. The points sit at
$\theta_j = 2\pi(j + \tfrac12)/N$, off the real axis for a curve symmetric about it.

**The region.** The mode solvers' eigenvalue is $\beta^2 = k_0^2 n_\text{eff}^2$. A region is drawn
in $n_\text{eff}$, the ellipse $n(\theta) = c + a\cos\theta + i b\sin\theta$, and carried to the
$\beta^2$ plane by the map itself: $z(\theta) = k_0^2 n(\theta)^2$,
$z'(\theta) = 2 k_0^2 n(\theta) n'(\theta)$, weights $w_j = z'(\theta_j)/(iN)$. The map is one to
one on a region that doesn't hold $n_\text{eff} = 0$, so a region must not, and it must lie in
$\operatorname{Re} n_\text{eff} \geq 0$, the forward modes' half. `Region::between(low, high)`
is the guided modes' ellipse on the real axis, half as tall as it is wide.

**The iteration** is FEAST's (Polizzi, Fig. 2): filter the block, orthonormalize it, solve the
small projected problem $U^H A U w = \theta\thinspace U^H B U w$ (Rayleigh–Ritz), and filter the
subspace again until every Ritz pair inside the region has a small true residual, computed with
A and B themselves (Kestyn et al., Eq. 2.11):

$$
\frac{\lVert A x - \theta B x \rVert}{\alpha \lVert B x \rVert} \lt \text{tol}, \qquad \alpha = \max_j |z_j|,
$$

1e-10 by default. Each pass multiplies the error by about $|\rho(\lambda_{m_0+1})/\rho(\lambda_i)|$,
$\lambda_{m_0+1}$ the eigenvalue outside that the subspace of $m_0$ vectors misses
(Kestyn et al., Section 2.3), so two to four passes are typical. The subspace is resized to its
numerical rank (Section 2.5), here by modified Gram–Schmidt twice, which keeps a real subspace
real; Kestyn et al. do it by the spectral decomposition of $Q^H B Q$. The Rayleigh–Ritz step is
one-sided, on the right subspace only, which they note is possible; it gives no left
eigenvectors.

**The count.** The trace of P is the number of eigenvalues inside, and the first filtered block
of ±1 vectors estimates it for free: $\frac1m \sum_c y_c^H (P y)_c$ has mean tr P. `Found::estimate`
reports it. On a real, near-normal problem it is close (3.2 for the four-layer guide's 4 TE
modes); on a leaky one it can be far off, since a PML makes the matrix far from normal and the
estimate's spread grows with $\lVert P \rVert_F$. The count that matters is what the search
returns: every Ritz value inside, each converged.

**The subspace.** It must hold at least as many vectors as there are eigenvalues inside
(Kestyn et al., Section 2.3), and more makes it converge faster. Without a size
(`Search::subspace` `None`, the default), the search starts with 8, takes $2e + 8$ from a
plausible estimate e, and doubles when the Ritz values inside fill the subspace or when six passes
haven't converged. With a size, a subspace too small (an estimate above it, or Ritz values
inside filling it) is an error that says so.

**When it can't.** A filtered block whose rank stays below its size spans more than 14 decades:
eigenvalues near the boundary, or of enormous condition, swamp the rest, and the residuals stall.
It happens when a region cuts a PML's own branch of modes, the line of discrete eigenvalues that
rises from the cladding's or substrate's index into $\operatorname{Im} n_\text{eff}$. The search
stops and says so; a region below the branch, or more points, finds the leaky waves (as the
validation's regions do).

## The points, in parallel

Each point factorizes its own $z_j B - A$ with photonoxide's
[multifrontal LU](fdfd-3d.md) (one analysis of the structure shared by all, nested dissection
on the grid), refines its solves against the matrix, and keeps its factors between passes while
they fit in 2 GiB in all. The points run side by side on rayon's threads, a batch at a time, and
their contributions are summed in the points' order, so the result is the same bits on any number
of threads. A real pencil (no loss, no PML) on a region centred on the real axis needs only the
upper half's points: the lower half's solves are their conjugates (Kestyn et al., Table 1), so
16 points cost 8 factorizations.

Sakurai and Sugiura's own method, the eigenvalues of a Hankel pencil of the moments
$\mu_k = \frac1N\sum_j (\omega_j - \gamma)^{k+1} u^H(\omega_j B - A)^{-1} v$, is there too, for
their published example (the moments taken in units of the radius, the same pencil better
scaled).

## Validation

- **Analytic:** a uniform box (ε = 2.25, 2 × 1.4 µm on 20 × 14 cells, 1 µm): its six highest
  exact discrete eigenvalues, each twice ($H_x$ and $H_y$), all 12 found, to 3.8e-15 relative.
  A diagonal pencil's eigenvalues to rounding, and every eigenvalue in a disc of a 160-unknown
  non-Hermitian matrix against a dense solve to 1e-10 (tests).
- **Against a dense solve:** every eigenvalue in an ellipse of a leaky planar guide (the
  four-layer guide over 2 µm of PML, 10 nm grid, 901 unknowns): the same 3 as faer's dense QR
  algorithm, to 6.1e-10 (the leaky waves are sensitive, the PML making the matrix far from
  normal).
- **Against shift-and-invert Arnoldi:** the book's strip (500 × 220 nm, 1.55 µm, 20 nm grid),
  both guided modes found in $n_\text{eff}$ from 1.5 to 3.4, to 4.7e-12; the four-layer guide lying
  flat in the full-vector solver with its PML (2.5 nm grid), the leaky waves m = 4 … 7, to 3.1e-12.
- **Published:** Chilwell and Hodgkinson's four-layer guide at 632.8 nm (1D, 1 nm grid): every
  bound mode in $n_\text{eff}$ from 1.5005 to 1.67, no count given, 4 TE and 4 TM, within 5.5e-7 of
  their Table 3; their leaky waves m = 4 … 7 of Table 2, found as every mode in a disc about
  m = 4 and an ellipse about m = 5 … 7 below the PML's branch, within 1.2e-5 (5 printed
  decimals; the grid adds about 1e-5 at m = 7). Sakurai and Sugiura's Example 5 by their Hankel
  method: the error falls by 1.59e6 from 64 to 128 points (median of 33 random u, v; theirs
  2.1e-6 to 1.3e-12, a ratio of 1.6e6, and their bound $1.25^{64} = 1.59 \times 10^6$), our
  median error at 64 points 8.4e-7 against their one draw's 2.1e-6.
- **Convergence:** the four-layer guide's TE bound modes after one filter pass of 8 vectors fall
  exponentially with the points: 3.1e-3, 9.2e-6, 3.3e-11 at N = 8, 16, 32, the rate per point
  from 16 to 32 1.08 times that from 8 to 16 (1 for an exponential, ½ for a power of N).
- **Threads:** the strip's modes on 1 and 4 threads, every effective index and field entry the
  same bits.

## Not yet

Left eigenvectors and the two-sided Rayleigh–Ritz of Kestyn et al.'s dual subspaces; their
flags for spurious Ritz values (Section 2.7); shifted Krylov solves at the points for 3D port
modes; Hadley's nonlinear eigenproblem by Beyn's contour method (W.-J. Beyn, Linear Algebra
Appl. 436, 3839, online 2011, doi:10.1016/j.laa.2011.03.030); and "every guided mode above an
index" as a job in the studio.

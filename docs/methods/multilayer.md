---
title: Multilayer slab, transfer matrices
module: mode::multilayer
summary: The bound modes and leaky waves of any planar stack, exactly, from 2 × 2 field-transfer matrices.
order: 4
papers:
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - mode/multilayer-bound-chilwell
  - mode/multilayer-leaky-chilwell
  - mode/multilayer-power-chilwell
examples:
  - multilayer_chilwell
---

Films 1 … J lie between a semi-infinite cover and substrate. In each medium of index n,
$\alpha = \sqrt{n^2 - \beta^2}$ (β the effective index), and $\gamma = \alpha$ for TE or $\alpha/n^2$ for TM.

## The method

Each film's field-transfer matrix (Chilwell and Hodgkinson's Eq. 10), with phase thickness
$\Phi_j = k \alpha_j d_j$:

$$
M_j = \begin{pmatrix} \cos\Phi_j & -\frac{i}{\gamma_j}\sin\Phi_j \\ -i\gamma_j \sin\Phi_j & \cos\Phi_j \end{pmatrix},
\qquad M = \prod_{j=1}^{J} M_j .
$$

A mode is a root of the modal-dispersion function (Eq. 26):

$$
\chi(\beta) = \gamma_c m_{11} + \gamma_c \gamma_s m_{12} + m_{21} + \gamma_s m_{22} = 0 .
$$

In the cover and substrate, α takes the root that decays away from the stack
($\operatorname{Im}\alpha > 0$), except for a **leaky wave**: where $\operatorname{Re}\beta$ is below that medium's
index, the outgoing root ($\operatorname{Re}\alpha > 0$), which grows away from the stack (Section 3.C).
The paper's convention, $e^{i(k\beta y - \omega t)}$ with $\operatorname{Im}\beta > 0$, is photonoxide's.

- **Bound modes** of a lossless stack: real roots between the largest bounding index and the
  largest film index, bracketed and bisected.
- **Leaky waves** and lossy stacks: complex roots, by secant iteration from a guess, or all of
  them in a region of the complex plane, from the minima of $|\chi|$ on a grid.
- **Fields** (Eq. 34) and each layer's **share of the power** (Eq. 42).

## Validation

The paper's four-layer guide: cover 1.0; films 1.66, 1.53, 1.60, 1.66 (500 nm each); substrate
1.50; 632.8 nm.

- **Table 3**, the 8 bound modes: all within 5e-7 (printed to 6 decimals).
- **Table 3**, all 48 shares of power per layer: within 0.05 % (printed to 0.1 %).
- **Table 2**, the 5 TE leaky waves: nine of the ten printed numbers are ours rounded. m = 5's
  real part is printed 1.38250 against our 1.3824892, one unit in the last place. With the
  bound modes agreeing to 7 digits, that is most likely the 1984 table's rounding.
- One film reproduces the [three-layer slab](slab.md) to 1e-12.

The multilayer slab is also the exact reference for the [PML](pml.md): an SOI slab's leakage
through its buried oxide into the substrate.

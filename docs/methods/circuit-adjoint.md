---
title: "Circuit adjoint"
module: circuit
summary: "How a circuit's response changes with every parameter of every component, from one more back-substitution with the transposed system: the adjoint variable method on the circuit solve, checked against finite differences of whole circuits."
order: 21
papers:
  - cite: "G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004)"
    doi: 10.1364/OL.29.002288
  - cite: "T. W. Hughes, M. Minkov, Y. Shi, S. Fan, Optica 5, 864 (2018)"
    doi: 10.1364/OPTICA.5.000864
  - cite: "W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walmsley, Optica 3, 1460 (2016)"
    doi: 10.1364/OPTICA.3.001460
  - cite: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue)"
    doi: 10.1002/lpor.201100017
validation:
  - circuit/adjoint-mzi
  - circuit/adjoint-ring
  - circuit/adjoint-mesh
  - circuit/adjoint-nested
  - circuit/adjoint-differences
  - circuit/component-differences
examples:
  - circuit_splitter
  - circuit_ring_critical
  - circuit_fit
---

Optimizing a circuit moves its components' parameters (couplings, phases, lengths, a ring's
radius) to improve what it does. Each step needs the response's gradient with respect to all of
them. Perturbing one parameter at a time would take a circuit solve per parameter. The adjoint
takes one more back-substitution with the system the [circuit solve](circuits.md) already
factorized, as the [FDFD adjoint](fdfd-adjoint.md) does for a device's permittivity.

## The method

The circuit solve is $M B = S_b E$ with $M = I - S_b \Gamma$: $S_b$ the block-diagonal matrix of
every component's S-matrix, $\Gamma$ the connections, $E$ the external ports, and $B$ the
outgoing waves, a column per external input. The circuit's S-matrix is $S = E^\mathsf{T} B$. A
parameter $\theta$ of one instance changes only that instance's block of $S_b$, and it enters on
both sides: $\partial M/\partial \theta = -\partial_\theta S_b\thinspace\Gamma$ on the left and
$\partial_\theta S_b\thinspace E$ on the right. Differentiating,

$$
M\thinspace\partial_\theta B = \partial_\theta S_b\thinspace (E + \Gamma B),
\qquad
\frac{\partial S}{\partial \theta} = E^\mathsf{T} M^{-1}\thinspace \partial_\theta S_b\thinspace A,
\qquad A = \Gamma B + E,
$$

where $A$ holds the waves going into every port.

**The objective** F is real, and S complex. Its change is
$dF = 2 \operatorname{Re} \sum_{qp} G_{qp}\thinspace dS_{qp}$, with $G_{qp} = \partial F/\partial S_{qp}$
the Wirtinger derivative, $\frac{1}{2}(\partial F/\partial \operatorname{Re} S_{qp} - i\thinspace\partial F/\partial \operatorname{Im} S_{qp})$.
For the power $F = \lvert S_{qp} \rvert^2$, $G_{qp} = \overline{S_{qp}}$. Then

$$
\frac{\partial F}{\partial \theta} = 2 \operatorname{Re} \operatorname{tr}\negthinspace\left(\Lambda^\mathsf{T}\thinspace \partial_\theta S_b\thinspace A\right),
\qquad
M^\mathsf{T} \Lambda = E\thinspace G .
$$

This is Veronis, Dutton and Fan's Eqs. 2–4 for a system $M b = r$ whose right-hand side depends
on $\theta$ too: their $-2 \operatorname{Re}(\lambda^\mathsf{T}\thinspace \partial_\theta A\thinspace u)$
becomes $2 \operatorname{Re}(\lambda^\mathsf{T} (\partial_\theta r - \partial_\theta M\thinspace b))$, and the two terms
combine into $\partial_\theta S_b\thinspace a$. Hughes, Minkov, Shi and Fan apply the same adjoint to a
mesh of interferometers, where it is backpropagation through the mesh.

Three points of the formulation:

- **Transposed, not conjugated.** S is a holomorphic function of $S_b$; the conjugate enters only
  through G. So the adjoint system is $M^\mathsf{T}$, solved with the LU factors of M that the
  circuit solve made: `solve_transpose`, and one step of iterative refinement, as in FDFD.
  (The conjugated form, $M^\dagger \tilde\Lambda = E \bar G$ with
  $2 \operatorname{Re} \operatorname{tr}(\tilde\Lambda^\dagger \ldots)$, is the same number and
  needs a conjugated solve.)
- **Real parameters.** Every component parameter is real, so the gradient is
  $2 \operatorname{Re}(\ldots)$, with no separate derivatives for the real and imaginary parts.
- **One column per input.** $\Lambda$ has a column per external input the objective depends on, a
  column of G with an entry other than 0: one, for the power from one port. A sum over
  wavelengths is the sum of each wavelength's terms.

**The cost per parameter** is a product the size of its component: $\partial_\theta S_b$ is one
block, so $\operatorname{tr}(\Lambda^\mathsf{T} \partial_\theta S_b A)$ touches only that block's rows of
$\Lambda$ and A. The whole gradient costs the circuit solve, one transposed back-substitution
per input, and a component's derivative per parameter.

**The Jacobian.** With $M^\mathsf{T} Z = E$ (a column per external port), $\partial S/\partial
\theta = Z^\mathsf{T} \partial_\theta S_b\thinspace A$ for every parameter at once: `Circuit::jacobian`. It is
what a circuit returns as its own `Component::derivatives`, so a circuit nested in a larger one
passes its derivatives up exactly.

**∂S/∂θ of a component** is its `Component::derivatives`, when it gives them. One that doesn't is
differentiated by fourth-order central differences of its `s_matrix`,

$$
\partial_\theta S \approx \frac{-S(\theta + 2h) + 8 S(\theta + h) - 8 S(\theta - h) + S(\theta - 2h)}{12 h},
\qquad h = \epsilon^{1/3} \max(\lvert\theta\rvert, 1) \approx 6.1 \times 10^{-6} \max(\lvert\theta\rvert, 1),
$$

h at most an eighth of the parameter's range and rounded so that $\theta + h$ is exact; within
$2h$ of a bound, by the fourth-order one-sided formula
$(-25 S_0 + 48 S_1 - 36 S_2 + 16 S_3 - 3 S_4)/12h$ inwards. Four evaluations per parameter, and
about $10^{-11}$ relative error for a phase that turns by $2\pi n L/\lambda$ (a waveguide's length):
the truncation, $(\beta h)^4/30$, is negligible at this step, and the round-off is about
$\epsilon/h$.

## Using it

```rust
use photonoxide::circuit::objective;

// F = Σ_λ (|S_out1,in1|² − 0.5)²: a 50:50 splitter over the band
let (f, gradient) = circuit.gradient(&wavelengths, &values, |s| {
    objective::power_error(s, out1, in1, &vec![0.5; s.len()])
})?;
```

`objective` has the power (`power`), its error against targets (`power_error`) and a matrix's
distance from a target, amplitudes and phases (`matrix_error`); any closure that returns F and G
per wavelength is an objective. `Circuit::parameter("instance.parameter")` finds a parameter's
position in the values, and `Component::parameters` gives every range, the bounds for an
optimizer. The optimizers are genoxide's: the examples run L-BFGS-B and CMA-ES on these
gradients.

## Validation

Each case compares the adjoint gradient with fourth-order central finite differences of the whole
circuit's F, parameter by parameter, and shows the largest difference relative to the gradient's
largest component. The components (a lossy waveguide, a lossless coupler, a phase shifter) give
closed-form derivatives, except in the fifth case.

| Case | Parameters | Step $\delta$ | Difference |
|---|---|---|---|
| MZI, bar power summed over 5 wavelengths | 2 couplings, 2 lengths | $10^{-3}$, $10^{-4}$ µm | 1.3e-10 |
| Add-drop ring on a resonance's flank, $(\lvert S_{41} \rvert^2 - 0.5)^2$ | 2 couplings, 2 lengths | $10^{-4}$, $10^{-5}$ µm | 3.4e-11 |
| 4 × 4 Clements mesh, $\sum_{qp} \lvert S_{qp} - T_{qp} \rvert^2$, complex T | 12 couplings, 12 phases | $10^{-3}$ | 7.1e-12 |
| Circuits as instances (a ring, an MZI), a loop, 3 wavelengths | 9, through the inner Jacobians | $10^{-4}$, $10^{-5}$ µm | 4.8e-10 |
| The circuit solve's 11-instance netlist, no component derivatives | 11, by component differences | $10^{-4}$, $10^{-5}$ µm | 4.5e-10 |

That is the finite differences' own limit, which the step sets. Their truncation falls as
$\delta^4$, their round-off rises as $1/\delta$. Ten times the step gives
$3 \times 10^{-7}$ (MZI), $4 \times 10^{-7}$ (ring), $3 \times 10^{-8}$ (mesh) and
$5.6 \times 10^{-9}$ (nested): the truncation, ten thousand times larger; a tenth of the step gives
$2 \times 10^{-10}$, $3 \times 10^{-9}$, $6 \times 10^{-11}$ and $1.3 \times 10^{-9}$: the round-off, ten times
larger. The adjoint gradient sits below both, so it is exact to what the differences can tell.
The ring's resonance curves F sharply, so its lengths need the smaller step.

A sixth case checks the component differences on their own against the closed forms: a
waveguide's length at 100 µm and at its lower bound 0 (one-sided), a coupler's $\kappa^2$, a phase
shifter at both its bounds: $1.6 \times 10^{-11}$.

The tests also check the Jacobian against differences of S itself, that a reciprocal circuit's
$\partial S/\partial\theta$ is symmetric, that a sum over wavelengths is the sum of the gradients, and an
MZI's $\partial \lvert S_{31} \rvert^2/\partial\phi = \sin(\phi)/2$ in closed form, to $10^{-15}$.

## Limits

- Parameters are real and continuous. Anything that changes a component's ports is fixed when
  it is built.
- A component without derivatives costs four evaluations of its S-matrix per parameter and is
  accurate to about $10^{-11}$, not round-off; a component built on a solver (2D or 3D FDFD) should
  give its own, from the solver's adjoint (0.6's co-design).
- The objective is a function of the circuit's S-matrices. A function of the internal waves (the
  power circulating in a ring) needs its own right-hand side, not yet offered.
- Near a lossless resonance the system is nearly singular and the gradient grows as the
  response does; the circuit solve refuses an exactly singular one.

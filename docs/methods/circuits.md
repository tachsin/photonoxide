---
title: "Circuits"
module: circuit
summary: "A chip as a netlist of components, each an S-matrix, solved as one sparse linear system for the S-matrix between its external ports; Filipsson's sub-network growth is its reference."
order: 20
papers:
  - cite: "G. Filipsson, 11th European Microwave Conference, 700 (1981)"
    doi: 10.1109/EUMA.1981.332972
  - cite: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012)"
    doi: 10.1002/lpor.201100017
validation:
  - circuit/series-waveguides
  - circuit/mzi-closed-form
  - circuit/ring-all-pass-bogaerts
  - circuit/ring-add-drop-bogaerts
  - circuit/ring-fsr-bogaerts
  - circuit/sub-network-growth
  - circuit/reciprocity
  - circuit/unitarity
---

A circuit is a **netlist**: instances of components, connections between their ports, and
external ports left open. Each component is an S-matrix at a wavelength, from a closed form, a
compact model, a 2D or 3D solve, or a measurement. The circuit's own S-matrix, between its
external ports, follows from the components' alone: a circuit adds no physics, it only
connects. The model, the types and how the rest of photonoxide plugs in are in the
[design note](../design/components.md).

## Conventions

They are the [FDFD ports'](fdfd-ports.md). Fields go as $e^{-i\omega t}$, so a waveguide of
effective index $n$ and length $L$ transmits $e^{+i 2\pi n L/\lambda}$. Amplitudes are
power-normalized: $b = S a$ with $\lvert a_p \rvert^2$ the power going in at port $p$ and
$\lvert b_q \rvert^2$ the power coming out at port $q$, and $S_{qp}$ is from $p$ into $q$. Each
port's phase is referred to its reference plane, where the component ends, and a connection
joins two reference planes with nothing between them.

## The global solve

Number all ports of all instances together, an instance's ports one after the other. Then
$S_b$, block diagonal with every component's S-matrix, relates all outgoing waves to all incoming
ones, $b = S_b a$. A connection between ports $i$ and $j$ says that what leaves one enters the
other, $a_i = b_j$ and $a_j = b_i$; an external port $k$'s incoming wave is the circuit's input
$x_k$. So $a = \Gamma b + E x$, with $\Gamma$ the connections' symmetric permutation and $E$ the
selection of the external ports, and

$$
(I - S_b \Gamma)\thinspace b = S_b E\thinspace x,
\qquad
S = E^\mathsf{T} (I - S_b \Gamma)^{-1} S_b E .
$$

This is one sparse system, as many unknowns as ports, with a right-hand side per external port.
Column $j$ of $S_b \Gamma$ is column $\Gamma(j)$ of $S_b$, which is nonzero only on the rows of
that port's own component, so the system has $\sum_c n_c^2$ entries for components of $n_c$
ports, plus the diagonal. It is factorized by faer's sparse LU with one step of iterative
refinement, as the [FDFD solver](fdfd.md) is.

Every entry of every component's block is kept in the pattern, zero or not, so the sparsity
depends only on the netlist. `Netlist::compile` analyses it once (ordering and symbolic
factorization), and each wavelength of a sweep pays only for the numerical factorization.

A circuit is a component too: its ports are the netlist's external ports, and its parameters
its instances', named `instance.parameter`. Circuits nest.

## Sub-network growth, the reference

Filipsson (1981) joins the ports one connection at a time. From the block-diagonal S of all
ports (his Eq. 7), connecting ports $k$ and $l$ replaces every other entry by his Eq. 6,

$$
S_{ij} + \frac{S_{il} S_{kj} (1 - S_{lk}) + S_{il} S_{kk} S_{lj} + S_{ik} S_{lj} (1 - S_{kl}) + S_{ik} S_{ll} S_{kj}}{(1 - S_{kl})(1 - S_{lk}) - S_{kk} S_{ll}},
$$

and drops ports $k$ and $l$. His connection conditions, $V_k^- = V_l^+$ and $V_l^- = V_k^+$ (his
Eqs. 2-3, $V^-$ outgoing), are the ones above. It is dense, $O(N^2)$ per connection for $N$
ports, and `Circuit::s_matrix_by_growth` implements it as printed, as the reference the sparse
solve is checked against. On a netlist with reflections everywhere, loops, nested circuits and
ports joined to their own component, which exercises all four terms, the two agree to
$1.6 \times 10^{-16}$: Eq. 6 has no misprint.

## Checks

`SMatrix` checks what a circuit must satisfy, for any S:

- **reciprocity:** $S = S^\mathsf{T}$ for reciprocal components, as the FDFD solver's are by
  construction; `reciprocity_error` is the largest $\lvert S_{qp} - S_{pq} \rvert$;
- **passivity:** every singular value of $S$ at most 1, `largest_singular_value` and
  `is_passive`;
- **unitarity** for lossless components, $S^\dagger S = I$; `unitarity_error` is the largest
  entry of $\lvert S^\dagger S - I \rvert$.

A netlist of reciprocal, passive or lossless components is reciprocal, passive or lossless
itself, so these check the solve as well.

## Rings and interferometers

Bogaerts et al. (2012) give the ring's responses in closed form. An all-pass ring is a coupler
with self-coupling $r$ and cross-coupling $\kappa$, $r^2 + \kappa^2 = 1$, one of whose outputs
returns to its input through the ring, with single-pass amplitude $a$ and phase $\phi$. Their
Eq. 1,

$$
\frac{E_\text{pass}}{E_\text{input}} = e^{i(\pi + \phi)}\thinspace \frac{a - r e^{-i\phi}}{1 - r a e^{i\phi}} ,
$$

is $(r - a e^{i\phi}) / (1 - r a e^{i\phi})$, what a coupler transmitting $r$ straight through and $i\kappa$
across, and a ring transmitting $a e^{+i\phi}$ give: the $e^{-i\omega t}$ convention. The circuit of a
coupler and a waveguide reproduces it, and its square, Eq. 2, to $4 \times 10^{-14}$; an add-drop
ring of two couplers and two half rings reproduces Eqs. 5 and 6 to $2 \times 10^{-14}$.

Their Eq. 9, the free spectral range $\lambda^2 / (n_g L)$, is first order in the dispersion. For
a group index constant in wavelength it is exact at the geometric mean of two neighbouring
resonances; at their midpoint its relative error is $(\Delta\lambda / 2\lambda)^2$. The circuit's
resonances, found as minima of the through power, are $9.081493$ nm apart against Eq. 9's
$9.081571$ nm at the midpoint, the difference that error predicts ($7.8 \times 10^{-5}$ nm).

A Mach-Zehnder interferometer of two couplers and two arms is the product of their transfer
matrices, $C \operatorname{diag}(t_1, t_2)\thinspace C$, which the circuit reproduces to
$2 \times 10^{-16}$.

## Limits

- A connection has no length, loss or reflection: a waveguide between two components is a
  component, and a mismatch between two joined waveguides belongs to one of them.
- A port is one mode. A multimode waveguide is a port per mode; ports whose stated modes differ
  in polarization or order can't be joined, but nothing converts between modes unless a
  component does.
- The solve is linear and at one wavelength: no nonlinearity, no time domain (0.8 brings
  circuits in time).
- A lossless loop exactly on a resonance it can't couple out of makes the system singular, and
  the solve says so.
- A circuit states no error of its own: its components' errors don't simply add through it, and
  a resonance magnifies them.
- The sub-network growth is a reference: dense, and slow on a large netlist.

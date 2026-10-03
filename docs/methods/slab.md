---
title: "Three-layer slab, exact"
module: mode::slab
summary: "The TE and TM modes of a core between two claddings, from their characteristic equations, to the last bit."
order: 3
papers:
  - cite: "A. Yariv, P. Yeh, Photonics: Optical Electronics in Modern Communications, 6th ed., Oxford University Press (2007), Section 3.2"
    doi: ""
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015), Listings 3.2–3.3"
    doi: 10.1017/CBO9781316084168
validation:
  - mode/slab-te-book
  - mode/slab-tm-book
examples:
  - slab_soi
  - slab_yariv_yeh
---

A core of index $n_2$ and thickness $t$ lies between a lower cladding $n_1$ and an upper cladding
$n_3$. A guided mode has $\max(n_1, n_3)\thinspace k \lt \beta \lt n_2 k$, with $k = 2\pi/\lambda$, and transverse
wavenumbers

$$
h = \sqrt{n_2^2 k^2 - \beta^2}, \qquad q = \sqrt{\beta^2 - n_1^2 k^2}, \qquad p = \sqrt{\beta^2 - n_3^2 k^2}.
$$

## The characteristic equations

Yariv and Yeh's Eqs. (3.2-5) and (3.2-11):

$$
\text{TE:}\quad \tan(ht) = \frac{p + q}{h\thinspace(1 - pq/h^2)}, \qquad
\text{TM:}\quad \tan(ht) = \frac{h\thinspace(\bar p + \bar q)}{h^2 - \bar p \bar q},
$$

with $\bar p = (n_2/n_3)^2 p$ and $\bar q = (n_2/n_1)^2 q$. By the tangent's addition formula each is
the same as

$$
h t = m\pi + \arctan(q/h) + \arctan(p/h), \qquad m = 0, 1, \dots
$$

(with $\bar q, \bar p$ for TM). The left side falls and the right side rises with β, so mode m
has exactly one root. photonoxide brackets it and bisects to the last bit. The fields are the
book's (3.2-3) and (3.2-10): a cosine and sine in the core, exponentials outside.

Yariv and Yeh put the core at $-t \lt x \lt 0$; photonoxide's x is their −x, with the same $n_1$,
$n_2$, $n_3$, $p$ and $q$.

## Validation

- **Chrostowski and Hochberg**, Section 3.2.2: 220 nm of silicon (3.473) in oxide (1.444) at
  1550 nm, TE 2.845 and TM 2.051 (3 decimals). photonoxide: 2.844816 and 2.051101.
- **Yariv and Yeh**, Sections 3.1–3.2, every effective index to the 4 printed decimals:
  an asymmetric slab (1.0 / 2.0 / 1.7, t = λ) with TE 1.9594 and 1.8375, and a symmetric one
  (1.5 / 1.6 / 1.5, 5 µm at 1.55 µm) with TE 1.5946, 1.5785, 1.5521 and 1.5175; and the number of
  guided modes of each.
- The roots satisfy the tangent form to 1e-9; an asymmetric slab has its analytic cutoff; the
  fields meet the boundary conditions.

For more than one film, see the [multilayer slab](multilayer.md).

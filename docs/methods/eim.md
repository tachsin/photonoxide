---
title: "Effective index method"
module: mode::eim
summary: "A ridge waveguide's mode from two slab problems, fast and approximate, with its error against the full-vector solver measured."
order: 10
papers:
  - cite: "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977)"
    doi: 10.1364/AO.16.000113
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015), Section 3.2.5"
    doi: 10.1017/CBO9781316084168
validation:
  - mode/eim-strip-book
examples:
  - effective_index_method
---

The effective index method replaces a 2D cross-section by two 1D [slabs](slab.md). It is fast
enough for design sweeps and 2.5D simulation, and its error is known.

## The method

Hocker and Burns, following Knox and Toulios:

1. the vertical slab through the ridge gives an effective index $n_c$; beside it, a rib's
   thinner slab, or a strip's cladding, gives $n_s$;
2. the lateral slab, of core index $n_c$ between sides of $n_s$ and the ridge's width, gives the
   mode's effective index.

For large index steps the polarization must be kept: a **TE-like** mode solves the vertical slabs
as TE and the lateral one as TM, since the field that lies along the vertical slab's layers lies
across the lateral slab's; a TM-like mode the reverse.

## Its error

The method assumes the field separates, $E(x, y) = E(x)\,E(y)$, which corners break. Against
photonoxide's [full-vector solver](vector.md) (6.25 × 5 nm grid), 220 nm silicon strips in oxide
at 1550 nm, TE-like:

| Width | $n_\text{eff}$ | $n_g$ |
|---|---|---|
| 400 nm | +3.9 % | −6.7 % |
| 450 nm | +2.6 % | −5.0 % |
| 500 nm | +1.8 % | −3.7 % |
| 550 nm | +1.3 % | −2.9 % |
| 600 nm | +1.0 % | −2.2 % |

The narrower the strip, the more of its light is near the corners. Chrostowski and Hochberg
state 1.2 % and 2.7 % for 500 nm, against a 2D solver whose 2.443 their own EIM value is 1.9 %
above.

## Validation

The book's 500 × 220 nm strip gives 2.489 (Lumerical's 1D solver on a 10 nm mesh, fed the slab
index rounded to 2.845). On that input the exact lateral slab gives 2.488558; from the unrounded
2.844816, the method gives 2.488368, within 1e-3.

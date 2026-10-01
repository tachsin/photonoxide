---
title: Perfectly matched layers
module: mode::vector
summary: Absorbing layers inside the window's edges, by complex coordinate stretching: leaky modes and their loss, and no reflections from the window.
order: 8
papers:
  - cite: "W. C. Chew, J. M. Jin, E. Michielssen, Microw. Opt. Technol. Lett. 15, 363 (1997)"
    doi: 10.1002/(SICI)1098-2760(19970820)15:6<363::AID-MOP8>3.0.CO;2-C
  - cite: "W. C. Chew, W. H. Weedon, Microw. Opt. Technol. Lett. 7, 599 (1994)"
    doi: 10.1002/mop.4650071304
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984) (the leaky waves reproduced)"
    doi: 10.1364/JOSAA.1.000742
validation:
  - mode/pml-soi-leakage-te
  - mode/pml-soi-leakage-tm
  - mode/pml-leaky-chilwell
examples:
  - leaky_waves
---

A waveguide whose light leaks (into a substrate, around a bend) has a complex effective index:
its imaginary part is the loss. Finding it needs a window edge that absorbs the outgoing wave
instead of reflecting it.

## The method

Chew and Weedon showed that Maxwell's equations with stretched coordinates,
$\partial/\partial x \to (1/s_x)\, \partial/\partial x$, absorb without reflecting when s is complex. Chew,
Jin and Michielssen made it a change of variables: the coordinate itself becomes complex,

$$
\tilde x = \int_0^x s_x(x')\, dx', \qquad s_x = 1 + i\alpha \left(\frac{u}{d}\right)^2
\;\Rightarrow\;
\tilde x = x \pm i\alpha\, \frac{u^3}{3 d^2}
$$

(their Eqs. 34, 44 and 45), with u the depth into a layer of thickness d, plus towards larger x
and minus towards smaller. A wave $e^{ik\tilde x}$ travelling into the layer then decays, at any angle
and for both polarizations, without reflecting, and also where a dielectric interface runs into
the layer.

In the [full-vector solver](vector.md) this is just complex grid spacings, which Fallahkhair et
al.'s equations take as they are (their ref. 21). Both Chew papers use $e^{-i\omega t}$, like
photonoxide: no signs to translate. `Pml` sets a thickness per edge and the strength α; a PML on
an edge with a [mirror wall](walls.md) is an error, since a wall is a plane of symmetry, not an
open boundary.

## Validation

- **Exact substrate leakage:** 220 nm SOI on 0.5 µm of buried oxide leaks into the silicon
  substrate. The [multilayer slab](multilayer.md) gives its exact complex index. The loss,
  $\operatorname{Im} n_\text{eff}$, is within 0.68 %, 0.17 % and 0.04 % at 10, 5 and 2.5 nm (TE): second order,
  and the same for TM. On 0.8 µm of oxide the TE loss is $\operatorname{Im} n = 4.9 \times 10^{-8}$, still
  within 0.3 %.
- **Insensitive to the layer:** strength 3–10 and 0.5–2 µm change the result by under 1e-4
  relative; strength 1 is weaker (0.9 %).
- **A guided mode is left alone:** its index changes by under 1e-9.
- **Published leaky waves:** Chilwell and Hodgkinson's Table 2, from the full-vector solver with
  a PML in the substrate, m = 4–7 within 4e-5 at 2.5 nm. m = 8 sits just above the cover's index:
  in the cover its field decays slowly with its phase running towards the guide, which a PML
  can't absorb, so the cover is closed by a zero wall and m = 8 is within 2e-4.

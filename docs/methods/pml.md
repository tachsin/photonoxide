---
title: "Perfectly matched layers"
module: mode::vector
summary: "Absorbing layers inside the window's edges, by complex coordinate stretching: leaky modes and their loss, and no reflections from the window."
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
  - job/leaky-film-loss
  - job/leaky-film-grid-order
  - job/leaky-film-pml
  - job/leaky-wire-bienstman
  - job/leaky-wire-bienstman-loss
examples:
  - leaky_waves
---

A waveguide whose light leaks (into a substrate, around a bend) has a complex effective index:
its imaginary part is the loss. Finding it needs a window edge that absorbs the outgoing wave
instead of reflecting it.

## The method

Chew and Weedon showed that Maxwell's equations with stretched coordinates,
$\partial/\partial x \to (1/s_x)\thinspace\partial/\partial x$, absorb without reflecting when s is complex. Chew,
Jin and Michielssen made it a change of variables: the coordinate itself becomes complex,

$$
\tilde x = \int_0^x s_x(x')\thinspace dx', \qquad s_x = 1 + i\alpha \left(\frac{u}{d}\right)^2
\thickspace\Rightarrow\thickspace\tilde x = x \pm i\alpha\thinspace\frac{u^3}{3 d^2}
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

## In a modes job

A `"modes"` job takes a PML as `[task.pml]`: `thickness_um`, four thicknesses in µm (across the
guide at its low and high end, below and above; 0 for none, otherwise at least 4 cells of the
step), and `strength`, α, 3 by default. Its window is then open there, and its leaky modes are
kept, each named guided or leaky and recorded with its loss in dB/cm; the PML's own modes are
left out ([what a count returns](vector.md)). `job::check` refuses layers that are negative,
all 0, too thin, that fill the window, or that reach the guide's layer or every shape at the cut.

- **Exact substrate leakage, through a job:** 220 nm of silicon (3.473) on 0.5 µm of oxide
  (1.444) over silicon, across a job's whole window, with 1 µm of PML in the substrate. The
  film is uniform across, so the job's mode is the film's times the window's lowest standing
  wave, which is taken out exactly. The loss is within 2.7 %, 0.68 %, 0.17 % and 0.042 % of the
  exact one at 20, 10, 5 and 2.5 nm: second order. A PML of 0.5 µm changes it by 1.8e-4, of
  1.5 µm by 2.9e-6, strength 10 by 2.8e-5.
- **Bienstman et al.'s leaky wire, through a job** (`jobs/leaky-wire.yaml`, fixed indices in a
  `[task.stack]`, a window of 2 × 3.6 µm): on uniform grids of 20, 10 and 5 nm,
  $\operatorname{Re} n_\text{eff}$ is 8.8e-3, 6.4e-3 and 4.0e-3 above their 2.412372 and the loss
  0.94, 0.95 and 0.963 of their 2.9135e-8: the core's corners converge slowly, as on the
  library's graded grids (+4.0e-3 and 0.97 at 5 nm, before extrapolation).
- **The same guided modes with and without it:** a 500 × 220 nm strip in oxide, its window
  4.4 × 2.6 µm, a PML 0.6 µm thick on every edge: both modes the same within 2e-6.

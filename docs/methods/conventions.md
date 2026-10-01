---
title: "Units and conventions"
module: units
summary: "Micrometres, frequencies in c/µm, and one time convention, e^(−iωt), that fixes every sign in the library."
order: 1
papers:
  - cite: "A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010)"
    doi: 10.1016/j.cpc.2009.11.008
validation:
  - units/amplitude-convention
  - units/lossy-attenuation
---

Every solver in photonoxide shares one set of units and one time convention. Papers don't: the
mode solver's paper writes $e^{+j\omega t}$, the PML's papers $e^{-i\omega t}$. Each method's page says
how its paper's signs were brought into line.

## Units

Lengths are in **micrometres**. A frequency is in units of **c/µm**, the reciprocal of the vacuum
wavelength in micrometres: 1.55 µm is the frequency 1/1.55. With c = 1 a time step and a grid
step share a unit, the usual choice of electromagnetic solvers (Oskooi et al. 2010).
`Frequency::thz` and `to_thz` convert to SI with the exact speed of light.

`Length`, `Wavelength` (in vacuum) and `Frequency` are different types, so one can't be passed
where another is meant.

## The time convention

A time-harmonic field is the real part of a complex amplitude times $e^{-i\omega t}$. That fixes
every sign:

- a wave travelling towards +x is $e^{i(kx - \omega t)}$: at a fixed time, its phase grows with x;
- a waveguide mode travels along z as $e^{i(\beta z - \omega t)}$, so a lossy or leaky mode has
  $\operatorname{Im}\beta > 0$, and so does its effective index $n_\text{eff} = \beta / k_0$;
- a passive, lossy medium has a **positive** imaginary permittivity, and refractive index
  $n = n' + i\kappa$ with $\kappa \ge 0$;
- a real signal's amplitude at ω is recovered with the kernel $e^{+i\omega t}$:

$$
A = \frac{2}{T} \int_0^T s(t)\, e^{i\omega t}\, dt .
$$

## Validation

Two analytic cases pin the convention: the amplitude of a real signal is recovered with the
kernel above, and a wave in a medium with $\operatorname{Im}\varepsilon > 0$ decays over one wavelength by
exactly $e^{-2\pi\kappa}$.

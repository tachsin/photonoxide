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
  $\operatorname{Im}\beta \gt 0$, and so does its effective index $n_\text{eff} = \beta / k_0$;
- a passive, lossy medium has a **positive** imaginary permittivity, and refractive index
  $n = n' + i\kappa$ with $\kappa \ge 0$;
- a real signal's amplitude at ω is recovered with the kernel $e^{+i\omega t}$:

$$
A = \frac{2}{T} \int_0^T s(t)\thinspace e^{i\omega t}\thinspace dt .
$$

## The same numbers on every system

photonoxide's own solvers give the same bits on any number of threads: each sums in an order
fixed by the problem, not by the threads, and CI runs the tests on one thread as well as many.
Across systems the same bits are wanted but not promised, and CI checks what shows: the examples'
outputs, written on Linux, are compared on Windows and macOS (Apple silicon) too, the 3D devices
and the validation report by hand (the Systems workflow).

What can differ between systems, and what photonoxide does about each:

- **The maths library.** `f64::exp`, `sin`, `cos`, `powf`, `atan2` and `hypot` call the
  system's: glibc on Linux, the Microsoft C runtime on Windows, Apple's libm on macOS. Each is
  within an ulp or so, but they round differently. Where a printed result hangs on the last bit,
  photonoxide takes these from the pure-Rust [`libm`](https://docs.rs/libm) crate instead, the
  same bits everywhere: the circuit's closed-form S-matrices, which optimizations run on, and
  FDTD's source waveforms. Elsewhere the system's functions stay, and the last bits of a mode's
  index or a field may differ between systems with nothing printed to show it.
- **Fused multiply-adds.** Rust never fuses a multiply and an add on its own, and photonoxide
  builds with no `target-cpu` or feature flags; faer's dense kernels pick their SIMD
  instructions at run time.
- **Randomness.** The contour solver's probe vectors come from splitmix64 on fixed seeds, and
  genoxide's generators and its random choices use its portable maths.

Measured in October 2026 (issue #280), every example and the validation report on the three
systems before the change: the report and 29 of the 31 examples printed the same everywhere, the
3D devices included. Two didn't, both from the maths library:

- `circuit_fit`: L-BFGS-B took 78 evaluations on Linux, 80 on Windows, 81 on macOS. The target
  spectra differed in the last bits at 2 of their 81 wavelengths (`cos` near the resonance, up to
  5 ulps of the drop port after the cancellation there) and the circuit's gradient in its first
  evaluation (the guide's $e^{i\beta L}$), and the optimizer's stopping test, F below 1e-18,
  turned that into a different count. Now 79 on all three.
- `subpixel_holes`: two slopes moved in their sixth decimal on Windows. The pulse's
  $e^{-u^2}$, sin and cos and the source's cos(πx) differed in the last bit, and the matrix pencil
  that reads the modes' frequencies off the probes, keeping singular values down to 1e-10 of the
  largest, turned that into 1e-11 of a frequency. Unchanged on Linux.

## Validation

Two analytic cases pin the convention: the amplitude of a real signal is recovered with the
kernel above, and a wave in a medium with $\operatorname{Im}\varepsilon \gt 0$ decays over one wavelength by
exactly $e^{-2\pi\kappa}$.

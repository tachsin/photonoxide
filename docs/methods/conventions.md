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
  $\mathop{\mathrm{Im}}\beta \gt 0$, and so does its effective index $n_\text{eff} = \beta / k_0$;
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
  within an ulp or so, but they round differently. num-complex's `Complex::norm` is the system's
  `hypot`, and its `exp`, `sqrt`, `ln` and the rest are built on the system's functions too: the
  Givens rotations of QMR and GMRES take `norm`, so their iterates differ in the last bits between
  systems (block QMR took 2412, 2222 and 2467 iterations on Linux, Windows and macOS). The fix,
  #822, is `photonoxide::portable`: the same functions, real and complex, built from IEEE 754's
  exact operations only (fdlibm's exp and ln, the pure-Rust [`libm`](https://docs.rs/libm) crate,
  square and multiply, Kahan's complex square root), the same bits everywhere, each within the
  ulps its doc states. The rule (CONTRIBUTING.md, "The same numbers on every system") is to take
  every function but `+ − × ÷` and `sqrt` from there, and clippy forbids the system's. Today the
  solvers take it (the mode solvers, FDFD, EME, the eigensolvers, the sparse direct solver, the
  sweeps, and the Krylov solvers with their Givens rotations), and so do the physics areas:
  geometry and the cells' averaging, FDTD (its PML, media, sources and monitors), the materials,
  the stacks, the static solvers (heat and electrostatics), the circuits, the devices and their
  compact models, topology, fabrication, nonlinear and quantum optics, and the rest of the
  library: the jobs and their runs, the validation cases, expressions, units, the exports and the
  benchmarks; the examples and the tests too, but for the two tests that take the system's
  functions as an independent reference. The studio, native and crosscode follow in #822's
  phases, and until then the last bits of a number computed there may differ between systems with
  nothing printed to show it.
- **Fused multiply-adds.** Rust never fuses a multiply and an add on its own, and photonoxide
  builds with no `target-cpu` or feature flags; `mul_add` (fused in hardware or in software,
  whichever the build has) is forbidden. faer's dense kernels pick their SIMD instructions at run
  time, so results through its products, factorizations and eigensolvers can differ between CPUs;
  #822 leaves those for a later decision.
- **powi.** `f64::powi` is square and multiply (compiler-builtins' `__powidf2`) on Linux, but
  the C runtime's `pow` on Windows, where LLVM lowers it so: both within a few ulps, but a third
  of the powers differ in their last bit between the two (measured by the platform canaries,
  November 2026). `portable::powi` is the first, written out: the same products in the same order
  everywhere, exact for n in −1..=2, and 0⁰ = 1.
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

Since November 2026 the whole suite runs on Linux per pull request, and Windows and macOS run the
platform canaries weekly (`just canaries`, [CONTRIBUTING.md](../../CONTRIBUTING.md#platform-canaries)):
the system's maths library against correctly rounded values, faer's dense kernels, the sparse LU,
QMR and GMRES, the FFT, a slab's modes, a small FDFD solve, threads, the bits of a 2D FDTD run and
a job file read with CRLF line endings, each against an exact value or against itself computed
another way.

## Validation

Two analytic cases pin the convention: the amplitude of a real signal is recovered with the
kernel above, and a wave in a medium with $\mathop{\mathrm{Im}}\varepsilon \gt 0$ decays over one wavelength by
exactly $e^{-2\pi\kappa}$.

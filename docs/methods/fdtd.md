---
title: "FDTD"
module: fdtd
summary: "Maxwell's equations stepped in time on Yee's grid: E and H leapfrogging on FDFD's own grid, the convolutional PML, walls, periodic and Bloch-periodic sides (complex fields), conductors, lossy media, Drude and Lorentz media by auxiliary differential equations and the catalogue's materials fitted by them, the same bits on any number of threads; dipoles and currents normalized exactly by their spectra, plane waves on total-field/scattered-field boxes at any grid angle, one-way waveguide modes and Gaussian beams."
order: 23
papers:
  - cite: "K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966)"
    doi: 10.1109/TAP.1966.1138693
  - cite: "O. Aberth, Math. Comput. 27, 339 (1973) (the stability check's polynomial roots)"
    doi: 10.1090/S0025-5718-1973-0329236-7
  - cite: "A. Taflove, M. E. Brodwin, IEEE Trans. Microw. Theory Tech. 23, 623 (1975) (stability and numerical dispersion)"
    doi: 10.1109/TMTT.1975.1128640
  - cite: "K. Umashankar, A. Taflove, IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982) (total-field/scattered-field)"
    doi: 10.1109/TEMC.1982.304054
  - cite: "J.-P. Berenger, J. Comput. Phys. 114, 185 (1994) (the PML)"
    doi: 10.1006/jcph.1994.1159
  - cite: "C. L. Lawson, R. J. Hanson, Solving Least Squares Problems, SIAM (1995) (nonnegative least squares, for the catalogue's fits)"
    doi: 10.1137/1.9781611971217
  - cite: "M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave Lett. 7, 121 (1997) (dispersive media by auxiliary differential equations)"
    doi: 10.1109/75.569723
  - cite: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000) (the convolutional PML)"
    doi: 10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A
  - cite: "A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010) (sources restricted to the grid)"
    doi: 10.1016/j.cpc.2009.11.008
  - cite: "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012) (total-field/scattered-field in the frequency domain)"
    doi: 10.2528/PIERB11092006
validation:
  - fdtd/dispersion
  - fdtd/energy
  - fdtd/fdfd-lossy
  - fdtd/fdfd-cpml
  - fdtd/cpml-thickness
  - fdtd/spectrum
  - fdtd/tfsf-leakage
  - fdtd/tfsf-slab
  - fdtd/mode-source-fdfd
  - fdtd/mode-source-backward
  - fdtd/mode-source-power
  - fdtd/beam-waist
  - fdtd/beam-divergence
  - fdtd/beam-tilt
  - fdtd/ade-fdfd
  - fdtd/drude-fresnel
  - fdtd/lorentz-group-delay
  - fdtd/bloch-fdfd
  - fdtd/bloch-multilayer
  - fdtd/bloch-bands
  - fdtd/fit-lossless
  - fdtd/fit-lossy
  - fdtd/fitted-slab
examples:
  - cpml_roden_gedney
  - tfsf_square_cylinder
  - lorentz_okoniewski
---

The finite-difference time-domain method steps Maxwell's equations forward in time. One run
with a short pulse gives a structure's response over a whole band, and the field's evolution is
there to watch. photonoxide's FDTD shares FDFD's grid and conventions. A continuous source settles
to the field FDFD finds for the same current, and the leapfrog's own frequency, below, makes that
exact.

## The scheme

With c = 1, lengths in µm, times in µm/c and $\tilde H = \eta_0 H$ as in [FDFD](fdfd.md):

$$
\frac{\partial \tilde H}{\partial t} = -\nabla\times E - M, \qquad
\varepsilon\frac{\partial E}{\partial t} + \sigma E = \nabla\times\tilde H - J .
$$

Yee's scheme staggers the fields in space, on the same grid as [FDFD in 3D](fdfd-3d.md): E's
components at the middle of the cells' edges, H's at the centres of their faces. It also
staggers them in time: H half a step after E, each advanced from the other's curl (the
leapfrog). Every derivative is a centred difference, second order in space and in time. The
conductivity is averaged over the step, which keeps the update stable for any σ.

**Stability.** The step is $\Delta t = C/\sqrt{\sum_i 1/\Delta_i^2}$, with the Courant number
$0 \lt C \le 1$ (Taflove and Brodwin), the sum over the axes the field varies along. An axis one
cell long and periodic doesn't count, so a 2D problem steps at the 2D limit.

**Numerical dispersion.** A plane wave on the grid travels at Taflove and Brodwin's speed:

$$
\frac{\sin^2(\omega\Delta t/2)}{\Delta t^2} = \sum_i \frac{\sin^2(k_i\Delta_i/2)}{\Delta_i^2} ,
$$

slower than light, most along the axes. The scheme keeps this relation exactly, not
approximately: a plane wave started on a periodic grid oscillates at its frequency to round-off
(`fdtd/dispersion`, 3.6e-15).

**Energy.** In a closed lossless box the leapfrog conserves
$\tfrac12\sum\varepsilon E^2 + \tfrac12\sum\tilde H^{n-1/2}\cdot\tilde H^{n+1/2}$ exactly, because
Yee's two curls are each other's transposes (`fdtd/energy`, 2.4e-15 over 300 steps).

**FDFD's field, exactly.** A current $\operatorname{Re}(\hat J e^{-i\omega t})$, sampled at the half steps where it enters
E's update, settles to a field that solves FDFD's equations at the leapfrog's frequency:

$$
\tilde\omega = \frac{2}{\Delta t}\sin\frac{\omega\Delta t}{2} ,
$$

with a conductivity σ entering as $\varepsilon + i\sigma\cos(\omega\Delta t/2)/\tilde\omega$. In a closed
lossy box FDTD's steady amplitude and `Solver3d`'s field at $\tilde\omega$ agree to 2.3e-14,
against 4.1e-4 at ω itself (`fdtd/fdfd-lossy`). FDTD at a given step is FDFD at a slightly lower
frequency, the difference falling as $\Delta t^2$.

## Boundaries

**The convolutional PML** (Roden and Gedney) absorbs outgoing waves in a layer of cells. It is
the stretched coordinate of Berenger's PML, complex-frequency-shifted:

$$
s = \kappa + \frac{\sigma}{\alpha - i\omega} ,
$$

applied in the time domain by a recursive convolution. Each derivative across the layer becomes
$\partial/\kappa + \psi$, with $\psi \leftarrow b\psi + a\,\partial F$,
$b = e^{-(\sigma/\kappa + \alpha)\Delta t}$ and $a = \sigma(b - 1)/(\sigma\kappa + \kappa^2\alpha)$.
σ and κ grow as $(\text{depth}/d)^m$; α is constant. The auxiliary ψ live only in the layer's
slabs, two per field component per axis.

- **Defaults:** κ = 1 and α = 0, with σ from a target reflection R = 10⁻⁸ and order 3:
  $\sigma_\text{max} = (m + 1)(-\ln R)/(2d)$. That is FDFD's own PML, $s = 1 + i\sigma/\omega$,
  graded the same way. With it, FDTD's field approaches FDFD's as the step falls, the recursive
  convolution being FDFD's stretch to first order in Δt (`fdtd/fdfd-cpml`: 6.6e-4 at 64 steps a
  period, 3.5e-4 at 128).
- **Reflection:** at 2 cells from the layer, a pulse's reflection is 2.6e-2 of the field for 4
  cells, 1.2e-4 for 8 and 6.1e-6 for 16 (`fdtd/cpml-thickness`).
- **Evanescent fields:** κ > 1 and α > 0 absorb them, which the traditional PML reflects in late
  time. Roden and Gedney measured this beside a conducting plate in soil, and the
  [`cpml_roden_gedney`](../../examples/cpml_roden_gedney.rs) example reproduces it:
  - −48.6 dB with the traditional PML (their −48);
  - −70.5 dB with α = 0.05 S/m (their −67).
  - Their pulse's width isn't given; it is set so the first number is theirs, and the second is
    then a prediction.
  - `Cpml::sigma_optimal` is their Eq. 15.

**Walls.** A PML of zero cells is a perfect electric conductor: the tangential E on the wall is
held at zero, as in FDFD. **Periodic** sides wrap. A 2D problem is a grid one cell thick along
z, periodic there (`Boundaries::cpml_2d`).

**Bloch-periodic sides.** `Edges::Bloch { k }` with k ≠ 0 makes the field one period on
$e^{ikL}$ times itself, L the grid's length along the axis, FDFD's convention. A periodic
structure, a grating or a metasurface's unit cell, then needs one cell, and a fixed k along the
layers of a stack is oblique incidence, each frequency at its own angle, $\sin\theta = k/\omega$.

- **Complex fields.** For kL not a multiple of π, $e^{ikL}$ isn't real, and neither is a field
  that obeys it, so the fields are complex (`Simulation::is_complex`). They are not two
  separate real runs: the real and imaginary parts are the "sine and cosine" runs of a Bloch
  problem, but each part's boundary reads the other's at every step,
  $\operatorname{Re} F(x + L) = \cos(kL)\operatorname{Re} F(x) - \sin(kL)\operatorname{Im} F(x)$.
  The parts are stored as two real runs stepped side by side: Yee's update is real everywhere
  but where a difference reaches across a Bloch side, so each part steps with the real update,
  the same kernel as a periodic run, and only the values next to a Bloch side are coupled after
  each half step, by the difference $(e^{\pm ikL} - 1)F$ that the phase makes. A real run at
  k = 0 is the same bits as before, at the same speed; a complex run costs two.
- **Sources** in a complex run: a current of complex amplitude a takes the complex values
  $a\thinspace w(t)$, w the waveform's analytic form, so a pulse puts in its positive frequencies
  only; a point `Source` is real. A dipole restricted across a Bloch side puts its weight there
  on the image at the other end, times $e^{\mp ikL}$. The run's DFT divided by
  $a\thinspace W(\omega)$ (`Waveform::analytic_spectrum`) is FDFD's Bloch field at $\tilde\omega$.
  Total-field/scattered-field sources (plane waves, modes and beams) need real fields for now;
  a sheet of `Current` with the Bloch phase launches a plane wave instead.
- **Stability:** along a Bloch axis one cell long, the field varies by $e^{ik\Delta}$ a cell,
  so its part of the curl's largest eigenvalue is $(2/\Delta)^2\sin^2(k\Delta/2)$, and Δt counts
  it so.
- **Against FDFD:** a continuous current and a dipole restricted across two Bloch sides
  ($k_x = 1.3$, $k_y = -2.1$ rad/µm) in a lossy box settle to `Solver3d`'s Bloch field at
  $\tilde\omega$ to 2.9e-14, in a plain medium and in a dispersive one (`fdtd/bloch-fdfd`). A
  Bloch phase of a whole turn, $kL = 2\pi$, is the periodic run to round-off (a unit test).
- **Oblique incidence:** a pulse at $k_x = 2\pi \times 0.2$ rad/µm on 4 pairs of n = 2 and 1.5,
  in a column one cell across, transmits as the transfer matrices say, at each frequency's own
  angle (16.6° at 0.7 c/µm to 8.8° at 1.3), to second order in the cells
  (`fdtd/bloch-multilayer`):

  | Cells | TE | TM |
  |---|---|---|
  | 20 nm | 2.0e-2 | 2.0e-2 |
  | 10 nm | 4.9e-3 | 5.0e-3 |
  | 5 nm | 1.2e-3 | 1.3e-3 |

  The pulse is 0.3 c/µm wide, so that it puts next to nothing below $k_x/2\pi = 0.2$ c/µm. There
  the stack guides light that vacuum can't take, which rings for ever and would leak into every
  frequency of a finite DFT.
- **Bands:** one period of a 1D photonic crystal (0.2 µm of n = 2, 0.3 µm of vacuum) between
  Bloch sides at $k = 0.3\pi/\Lambda$ gives its three bands below 2 c/µm. A broadband run's
  probe, its DFT windowed by Blackman and Harris's four terms, shows each band as a peak; then a
  run whose pulse is 0.03 c/µm wide at each peak excites that band alone (the next is far below
  round-off in its spectrum), and its frequency is the probe's one-mode recurrence
  $E^{n+1} = zE^n$ fitted by least squares once the pulse is over, $\omega = -\arg(z)/\Delta t$:
  a lossless band rings as exactly $z^n$. The bands converge to the analytic
  $\cos k\Lambda = \cos k_1d_1\cos k_2d_2 - \tfrac12(n_1/n_2 + n_2/n_1)\sin k_1d_1\sin k_2d_2$
  (half the trace of a period's transfer matrix) to second order: 5.0e-3, 1.2e-3 and 3.1e-4
  at 20, 40 and 80 cells a period, relative (`fdtd/bloch-bands`). Harmonic inversion, which
  finds many modes and their Q in one run, is #163's.

**Conductors and media.** `Simulation::plate` and `conductor` hold E at zero on a conducting
plate or edge. The permittivity is real, averaged over each component's cell as FDFD averages it,
and a conductivity σ (in 1/µm: S/m × 376.73 Ω × 10⁻⁶) makes a medium lossy.

## Dispersive media

`Simulation::with_medium` puts a `Dispersive` medium where a function says, sampled at each value
of E (not averaged over the cell as the permittivity is, so a face is best put halfway between
two values):

$$
\varepsilon(\omega) = \varepsilon_\infty + \sum_p \frac{a_p}{\omega_{0p}^2 - \omega^2 - i\Gamma_p\omega} ,
$$

each term a Lorentz oscillator ($a_p = \Delta\varepsilon_p\thinspace\omega_{0p}^2$) or a Drude term
($\omega_{0p} = 0$, $a_p = \omega_p^2$), as the material models write them. Several of each go in
one medium, and several media in one run.

**Auxiliary differential equations** (Okoniewski, Mrozowski and Stuchly). Each term carries a
polarization current $J_p = \partial P_p/\partial t$, so that Ampère's law reads
$\varepsilon_\infty\partial E/\partial t + \sigma E + \sum_p J_p = \nabla\times\tilde H - J$, and each
current obeys their Eq. 5,

$$
\frac{\partial^2 J_p}{\partial t^2} + \Gamma_p\frac{\partial J_p}{\partial t} + \omega_{0p}^2 J_p = a_p\frac{\partial E}{\partial t} ,
$$

which for a Drude term is the time derivative of $\partial J_p/\partial t + \Gamma_p J_p = \omega_p^2 E$,
the same from rest. The update is their synchronized Lorentz scheme (LADES): the currents at the
steps, E's update at the half step with $J_p^{n+1/2} = (J_p^{n+1} + J_p^n)/2$, and the currents'
equation centred on step n with $(\partial E/\partial t)^n = (E^{n+1} - E^{n-1})/(2\Delta t)$:

$$
J_p^{n+1} = \alpha_p J_p^n + \xi_p J_p^{n-1} + \gamma_p\frac{E^{n+1} - E^{n-1}}{2\Delta t} ,
\quad \alpha_p = \frac{2 - \omega_{0p}^2\Delta t^2}{1 + \Gamma_p\Delta t/2},\
\xi_p = \frac{\Gamma_p\Delta t/2 - 1}{1 + \Gamma_p\Delta t/2},\
\gamma_p = \frac{a_p\Delta t^2}{1 + \Gamma_p\Delta t/2} .
$$

The coefficients are derived here from Eq. 5 by central differences: their Eq. 8 prints $\gamma_p$
without the $1/(1 + \delta_p\Delta t)$ that the centred damping term puts on every term. Put into
E's update, the new E appears only through $\gamma_p$, and the update solves to their pseudo-code's
LADES case: E ← $p_e E + p_m(\nabla\times\tilde H - J) + p_m[\chi E^{n-1} - \sum_p((\alpha_p + 1)J_p^n + \xi_p J_p^{n-1})/2]$,
with $\chi = \sum_p\gamma_p/(4\Delta t)$, $\nu = 1 + (\Delta t/\varepsilon_\infty)(\sigma/2 + \chi)$,
$p_e = (1 - \Delta t\sigma/(2\varepsilon_\infty))/\nu$ and $p_m = (\Delta t/\varepsilon_\infty)/\nu$. The
grid's own update gives the first two terms with $p_e$ and $p_m$ in place of its coefficients;
the rest is added after it, and the currents then step. A dispersive value of E keeps 2P + 1
numbers: $J_p^n$, $J_p^{n-1}$ and $E^{n-1}$.

**What a run solves.** For $e^{-i\omega t}$ at the steps the update is FDFD's equations at
$\tilde\omega$ with each term's $\chi_p$ replaced by

$$
\frac{i\cos(\omega\Delta t/2)}{\tilde\omega}\thinspace\frac{\gamma_p(z - 1/z)}{2\Delta t\thinspace(z - \alpha_p - \xi_p/z)} ,\quad z = e^{-i\omega\Delta t} ,
$$

`Dispersive::leapfrog_permittivity`, which is $\varepsilon(\omega)$ to second order in Δt (a unit
test). A continuous current in a closed box filled with a Drude and a Lorentz term and a
conductivity settles to `Solver3d`'s field with that permittivity to 3.3e-14; with the medium's
own $\varepsilon(\tilde\omega)$ the difference is 2.4e-3 at 64 steps a period (`fdtd/ade-fdfd`).

**Stability.** A medium is refused unless it is passive, $\varepsilon_\infty \gt 0$ and every
strength and damping nonnegative (a term with either negative is a gain), and unless the update
steps every plane wave the grid holds with $\lvert z\rvert \le 1$: the roots of its
characteristic polynomial,

$$
\varepsilon_\infty(z - 1)^2 + \frac{\sigma\Delta t}{2}(z^2 - 1) + \sum_p\frac{\gamma_p}{4}\frac{(z^2 - 1)^2}{z^2 - \alpha_p z - \xi_p} + (K\Delta t)^2 z = 0 ,
$$

for $(K\Delta t)^2$ from 0 to $4C^2$, by Aberth's iteration. That works out as $C \le \sqrt{\varepsilon_\infty}$
and $\omega_{0p}\Delta t \le 2$ for every Lorentz term (its own recursion's roots leave the unit
circle beyond); Drude terms are stable at any $\omega_p\Delta t$ (a unit test sweeps these).

**Validation.**

- **A Drude metal** ($\varepsilon_\infty = 1$, $f_p = 1$ c/µm, $\gamma/2\pi = 0.05$ c/µm) as a
  half-space, lit by a pulse at normal incidence: its reflection coefficient from 0.4 to 1.6
  c/µm, below and above the plasma frequency, against Fresnel's $(1 - n)/(1 + n)$ with the same
  $\varepsilon(\omega)$, its phase referred to the face with the grid's own wavenumber. Second
  order in the cells: $\lvert\Delta r\rvert$ = 6.9e-3 at 20 nm, 1.6e-3 at 10 and 3.0e-4 at 5
  (`fdtd/drude-fresnel`).
- **A Lorentz slab** 10 µm thick ($\varepsilon = 2.25 + \omega_0^2/(\omega_0^2 - \omega^2 - i\gamma\omega)$,
  $f_0 = 2$ c/µm): the group delay of a pulse's first pass, cut before its first echo arrives,
  as a group index, against the first pass's exact delay, which is the bulk
  $n_g = \operatorname{Re}\thinspace d(n\omega)/d\omega$ (1.977 to 2.402 over 0.8 to 1.2 c/µm) to
  3e-5. Second order to a floor near 1e-4: 3.0e-3 at 10 nm, 8.8e-4 at 5 and 3.6e-4 at 2.5
  (`fdtd/lorentz-group-delay`). The pulse is 0.3 c/µm wide: a wider one reaches the resonance,
  where light is slow enough to be cut off with the echo, and the cut's leakage shows.
- **Okoniewski, Mrozowski and Stuchly's own case,** a two-term Lorentz half-space on cells of
  37.5 µm, is the [`lorentz_okoniewski`](../../examples/lorentz_okoniewski.rs) example. Their
  Fig. 1, read at 600 dpi, shows the scheme's errors in $\lvert r\rvert$ about 0.5 × 10⁻³ with
  ripples of ±0.3 × 10⁻³ up to 45 GHz, rising to 1.84 × 10⁻³ at 59.5 GHz, and in its phase
  within ±1 × 10⁻³ rad up to 50 GHz, rising to 2.1 × 10⁻³ at 57 GHz. Their time step and run
  aren't given, so the curve is read as a bound. Here the errors are within it at every
  frequency from 2.5 to 60 GHz, at a Courant number of 1 (largest 2.3 × 10⁻⁴ and 3.5 × 10⁻⁴ rad,
  0.24 and 0.34 of the curve) and of 0.5 (0.33 and 0.52 of it), and fall by 4.00 at half the cell.
  The paper's errors are 5 to 8 times these at their largest; with its time step and run unknown
  that isn't explained here, nor are its curves' ripples, which its Debye curve doesn't have.

**Conductivity** goes in first (`with_conductivity`, then `with_medium`) and stays in the
medium. A medium put over another replaces it. A total-field/scattered-field surface must not
cross a dispersive medium; a box around one is fine.

**The catalogue's materials.** `Dispersive::from_model` takes a model whose terms are Lorentz or
Drude terms exactly: a Drude or Lorentz model, a real constant, and a Sellmeier formula, each
term $B\lambda^2/(\lambda^2 - C^2)$ being $\Delta\varepsilon = B$ undamped at $f_0 = 1/C$ (to
round-off, a unit test). A Sellmeier term far in the ultraviolet may need a smaller step than
the grid's ($\omega_0\Delta t \le 2$): fit it instead. `Dispersive::fit` fits any material, a
table or a formula, over a band:

- $\varepsilon_\infty \ge 1$ and Lorentz terms of nonnegative strength at fixed resonances, by
  nonnegative least squares (Lawson and Hanson, Ch. 23) over 64 frequencies across the band, so
  every fit is passive, and stable wherever its resonances are; its largest error over 256 is
  stated (`Fit::error`, `Fit::relative`).
- Where the material is transparent over the band, undamped terms at 1.25 to 4 times the
  band's highest frequency and 0.8 to 0.25 times its lowest: a Sellmeier formula. Silicon (Li
  1980's table) over 1.2 to 1.7 µm and silica (Malitson 1965) over 0.4 to 1.6 µm each take two
  terms, within 6.0e-5 of $\varepsilon$ (`fdtd/fit-lossless`).
- Where it absorbs, damped terms from half the band's lowest frequency to twice its highest at
  three dampings each: In₀.₄₉Ga₀.₅₁P above its gap (Ferrini 2002's n and k) over 0.4 to 0.6 µm
  takes eight, within 8.1e-3 (`fdtd/fit-lossy`).
- A band beyond the material's data is refused, as the material refuses those wavelengths.
- A silicon slab 0.4 µm thick, fitted over 1.2 to 1.7 µm, reflects as Airy's formula with the
  catalogue's own index at 1.3, 1.45 and 1.6 µm, to second order in the cells: 2.9e-2 at 20 nm,
  7.3e-3 at 10 and 2.0e-3 at 5 (`fdtd/fitted-slab`).

Debye terms aren't needed for the catalogue's materials, and aren't there.

## Sources

**Waveforms.** Every source takes its values in time from a waveform:

- a Gaussian pulse on a carrier, `Waveform::pulse(frequency, bandwidth)` with the power
  spectrum's full width at half maximum;
- a differentiated Gaussian, which has no DC, as Roden and Gedney's source;
- a continuous wave, switched on smoothly.

Each has an analytic form w(t), whose real part is the waveform: a current of complex amplitude
a takes the values Re(a w(t)).

**Normalization, exactly.** The leapfrog is linear and time-invariant, so each field's discrete
Fourier transform, taken at its own times ($E$ at $n\Delta t$, $\tilde H$ at $(n + \tfrac12)\Delta t$),
solves FDFD's equations at the leapfrog's frequency $\tilde\omega$ for the transforms of the
currents, taken at the times they enter (J at $(n + \tfrac12)\Delta t$, M at $n\Delta t$). That is
exact, not approximate, once the fields have died away. `Waveform::spectrum` is that transform
of a source's values, so the field per unit source spectrum is the field's transform divided by
it. A pulse from an electric point current and from a magnetic dipole in a lossy box gives
`Solver3d`'s field at $\tilde\omega$ for those spectra to 6.5e-13, at the carrier and off it
(`fdtd/spectrum`). For a complex amplitude a, the values Re(a w) transform as
$\tfrac12(a W(\omega) + \bar a\thinspace W(-\omega)^*)$. Near the carrier the second term is the
negative frequencies' image, $e^{-(\omega\tau)^2}$ of the first for a Gaussian of width τ.

**Dipoles and currents.** A `Current` sets complex amplitudes on many values of E (J) or H̃ (M)
with one waveform. A `Dipole` sits anywhere, not only on a value: Oskooi et al. restrict it to the
grid with the weights of trilinear interpolation, the transpose of interpolation, which keeps the
current's integral (their Section 3 and Fig. 4). Its amplitude is $\int J\thinspace dV$, over the
cross-section in a 2D problem.

**Total-field/scattered-field** (Umashankar and Taflove, Section II.A). A closed surface splits
the grid. Inside, the total field runs; outside, only the scattered field. Where an update's curl
reaches across the surface, the incident field at the value across is added (into the total
region) or subtracted (into the scattered one). Those corrections are currents on the surface:
J from the incident H̃, M from the incident E. They are exact when the incident field solves the
grid's own equations there, and then the scattered region sees nothing of it but round-off.
FDFD's ports use the same splitting in the frequency domain (Rumpf, Eq. 55).

**Plane waves.** `Simulation::add_plane_wave` puts a plane wave on a box. It travels along
$(p/\Delta x, q/\Delta y, s/\Delta z)$ with p, q and s integers. A field that depends only on the
position along that direction is a function of $\ell = \sum_i p_i\thinspace(2 r_i/\Delta_i)$, an
integer at every value of the grid, staggering included. Yee's update of such a field is an update
of a 1D field of ℓ: its differences along x reach $\ell \pm p$, along y $\ell \pm q$, along z
$\ell \pm s$. The incident field comes from that 1D update, run alongside the grid with the same
step:

- a sheet of current upstream starts the wave, at $E = -K/(2n)$ for a sheet K, so its E is the
  waveform times the polarization to second order in the cells (1.2e-2 off at 20 cells a
  wavelength, 7.7e-4 at 80);
- a matched absorber ends it at each side;
- `Simulation::incident` reads the field the box sees.

The box's corrections then hold at every frequency at once, at normal and oblique incidence
alike. With nothing inside, the scattered region holds 1.3e-15 of the incident field: in 2D along
(1, 0), (2, 1) and (1, −3), in 3D along (0, 0, 1) and (2, 1, 1) (`fdtd/tfsf-leakage`). A slab
inside reflects as Airy's formula says, to second order in the cells: 5.4e-3 at 20 nm, 1.3e-3
at 10 and 3.3e-4 at 5 (`fdtd/tfsf-slab`).

The [`tfsf_square_cylinder`](../../examples/tfsf_square_cylinder.rs) example reproduces
Umashankar and Taflove's square conducting cylinder (k₀A = 1, 20 cells a side). Its surface
current matches their Fig. 5(a) on the lit face (1.732, theirs 1.750) and the side faces (0.764,
theirs 0.785). At 40 cells a side it is within 0.4 % of their moment method, Eq. 8a, solved in
the example. On the shadowed face both give 0.17, and the figure's curve, 0.207, is 21 % above
the converged solution of its own equation.

**Modes.** `Simulation::add_mode_source` launches a mode of `PortMode3d`, the grid's own, one way
from its plane. The surface is the plane. The incident field is the mode carried along its axis
by $e^{\pm i\beta\Delta a}$ a step, and its H̃ is the grid's curl of its E. Going forward the total
field is from the plane on; going backward, up to the plane after it, as FDFD's `mode_source`.

- **Its frequency:** the mode must be FDFD's on the same grid and medium at $\tilde\omega$ for the
  waveform's carrier (`Simulation::fdfd_wavelength`). The source refuses one solved at ω itself.
- **Against FDFD:** in a lossy closed box the run settles to `Solver3d`'s field for its own mode
  source to 2.5e-14 (`fdtd/mode-source-fdfd`).
- **One way:** in an open guide, 1.5e-7 of the power goes backward (`fdtd/mode-source-backward`).
  The rest is the CPMLs, which are FDFD's PMLs only to first order in Δt. The forward flux is
  1.000048 of the mode's power (`fdtd/mode-source-power`).
- **A pulse** launches every frequency with the carrier's mode, and what goes back grows away
  from the carrier as the mode changes. For a pulse 0.1 c/µm wide on the same guide:

  | Frequency | Backward / forward power | Forward / mode's power |
  |---|---|---|
  | carrier − 0.05 c/µm (−7.8 %) | 4.1e-4 | 0.9968 |
  | carrier, 1/1.55 c/µm | 1.5e-7 | 1.000047 |
  | carrier + 0.05 c/µm (+7.8 %) | 3.8e-4 | 0.9989 |

**Gaussian beams.** `Simulation::add_beam` launches a beam one way from a plane, for grating
couplers. On the plane, E's tangential components are the paraxial beam's: waist, focus and
tilt, with its curvature and Gouy phase; in 2D, the 2D beam's $\sqrt{w_0/w}$. That field is
decomposed into the grid's own plane waves across the plane's window, a discrete Fourier series.
Each wave's normal wavenumber comes from the grid's dispersion at $\tilde\omega$, and its normal E
makes it divergence-free on the grid. On the plane the field is the beam's exactly. Off it, the
beam propagates as the grid propagates that field, not as the paraxial approximation does, and the
waves solve the grid's equations to 8e-15 in 3D (a unit test). For a 2D beam of
$w_0 = 1.5\lambda$ at 20 cells a wavelength:

- **Waist:** 1.49999 against 1.5 (`fdtd/beam-waist`), the focus 1.94 µm from the plane (2 asked).
- **Divergence:** the width grows exactly as $w^2 = w_0^2 + \theta^2 d^2$ for a sum of plane
  waves. θ is 0.218825, the grid's own prediction to 2.7e-6 (`fdtd/beam-divergence`). The
  paraxial $\lambda/(\pi w_0) = 0.2122$ is 3.1 % below: the beam isn't paraxial at
  $w_0 = 1.5\lambda$ (the continuum's exact 0.2160, 1.8 %), and the grid's dispersion adds 1.3 %.
- **Tilt:** tilted by 10°, the centre moves at 0.1825 µm per µm, to 1.5e-4 of the grid's
  prediction (`fdtd/beam-tilt`), above $\tan 10° = 0.1763$ by the same two effects.

## Probes

A probe records one value of E or H̃ every step. DFT monitors, fluxes and harmonic inversion are
#163.

## Cost

Every update is a fixed stencil with no reduction, its z-planes shared among rayon's threads, so
the fields are the same bits on any number of threads. Each axis's neighbour offsets and 1/(κΔ)
factors are tabulated once, and the PML's convolutions run only in their slabs. The kernel is
plain Rust loops. SIMD, f32 and cache blocking are #165, and the GPU is #166. Roden and Gedney's
plate, 2.9 × 10⁶ cells for 2000 steps plus two smaller lattices, takes about 45 s on 20 threads
of a Core Ultra 7 265K.

## Validation

| Case | What | Measured |
|---|---|---|
| `fdtd/dispersion` | a plane wave's frequency against Taflove and Brodwin's relation | 3.6e-15 |
| `fdtd/energy` | the leapfrog's invariant in a closed box, 300 steps | 2.4e-15 |
| `fdtd/fdfd-lossy` | a lossy box's steady state against FDFD at $\tilde\omega$ | 2.3e-14 |
| `fdtd/fdfd-cpml` | with CPMLs, against FDFD with its PML, at 128 steps a period | 3.5e-4 |
| `fdtd/cpml-thickness` | a 2D pulse's reflection from a CPML of 16 cells | 6.1e-6 |
| `fdtd/spectrum` | a pulse's DFT per unit source spectrum against FDFD at $\tilde\omega$ | 6.5e-13 |
| `fdtd/tfsf-leakage` | a plane wave's box with nothing in it, normal and oblique, 2D and 3D | 1.3e-15 |
| `fdtd/tfsf-slab` | a slab's reflection in a box against Airy's formula, cells of 5 nm | 3.3e-4 |
| `fdtd/mode-source-fdfd` | a mode source in a lossy box against FDFD's mode source | 2.5e-14 |
| `fdtd/mode-source-backward` | the power a mode source sends back in an open guide | 1.5e-7 |
| `fdtd/mode-source-power` | its forward flux against the mode's power | 1.000048 |
| `fdtd/beam-waist` | a 2D Gaussian beam's fitted waist, µm | 1.49999 (1.5) |
| `fdtd/beam-divergence` | its divergence against the grid's plane waves | 2.7e-6 |
| `fdtd/beam-tilt` | a tilted beam's direction against the grid's plane waves | 1.5e-4 |
| `fdtd/ade-fdfd` | a box of a Drude and a Lorentz term against FDFD with the leapfrog's permittivity | 3.3e-14 |
| `fdtd/drude-fresnel` | a Drude half-space's reflection against Fresnel's, 0.4 to 1.6 c/µm, cells of 5 nm | 3.0e-4 |
| `fdtd/lorentz-group-delay` | a Lorentz slab's group index from a pulse's delay, cells of 2.5 nm | 3.6e-4 |
| `fdtd/bloch-fdfd` | Bloch sides, plain and dispersive, against FDFD's Bloch field | 2.9e-14 |
| `fdtd/bloch-multilayer` | a stack at oblique incidence against the transfer matrices, cells of 5 nm | 1.3e-3 |
| `fdtd/bloch-bands` | a 1D photonic crystal's bands, 80 cells a period, relative | 3.1e-4 |
| `fdtd/fit-lossless` | silicon's and silica's fits, relative | 6.0e-5 |
| `fdtd/fit-lossy` | InGaP's fit above its gap, relative | 8.1e-3 |
| `fdtd/fitted-slab` | a fitted silicon slab against Airy's formula, cells of 5 nm | 2.0e-3 |
| example `cpml_roden_gedney` | Roden and Gedney's plate in soil, both PMLs | −48.6 and −70.5 dB (paper: −48, −67) |
| example `tfsf_square_cylinder` | Umashankar and Taflove's square cylinder's surface current | 1.732 and 0.764 (figure: 1.750, 0.785); within 0.4 % of their Eq. 8a |
| example `lorentz_okoniewski` | Okoniewski, Mrozowski and Stuchly's two-term Lorentz half-space, $\lvert r\rvert$ and phase errors, 37.5 µm cells | at most 0.24 and 0.34 of their Fig. 1's curve (C = 1), 0.33 and 0.52 (C = 0.5) |

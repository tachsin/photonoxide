---
title: "Phase matching"
module: nonlinear::phase
summary: "The wave-vector mismatch of any three- or four-wave process from any index (a catalogue model with its temperature, a material, or a mode's effective indices given as data); quasi-phase matching, its period, order and duty cycle; birefringent phase matching of types 0, I and II, noncritical or angle tuned; the undepleted response sinc²(ΔkL/2); the acceptance bandwidths in wavelength, temperature and angle, checked against Fejer et al.'s Table I; and the coupled-amplitude equations of SHG, sum- and difference-frequency generation, depleted and lossy, against Armstrong et al.'s closed forms."
order: 30
papers:
  - cite: "M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, IEEE J. Quantum Electron. 28, 2631 (1992) (Eqs. (2), (19)–(25), (30), (41)–(44), (A1.1)–(A1.9); Table I)"
    doi: 10.1109/3.161322
  - cite: "R. W. Boyd, Nonlinear Optics, 4th ed., Academic Press (2020) (Eqs. (2.2.11), (2.3.8); Table 2.3.2)"
    doi: 10.1016/C2015-0-05510-1
  - cite: "G. J. Edwards, M. Lawrence, Opt. Quantum Electron. 16, 373 (1984) (Eq. 1, Table I)"
    doi: 10.1007/BF00620081
  - cite: "D. H. Jundt, M. M. Fejer, R. L. Byer, IEEE J. Quantum Electron. 26, 135 (1990) (Table III)"
    doi: 10.1109/3.44926
  - cite: "D. H. Jundt, Opt. Lett. 22, 1553 (1997) (Eq. (3), the thermal expansion)"
    doi: 10.1364/OL.22.001553
  - cite: "J. A. Armstrong, N. Bloembergen, J. Ducuing, P. S. Pershan, Phys. Rev. 127, 1918 (1962) (Eq. (6.6), Sec. VI)"
    doi: 10.1103/PhysRev.127.1918
  - cite: "W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997) (Eqs. (7)–(12), the joint spectral amplitude)"
    doi: 10.1103/PhysRevA.56.1627
  - cite: "C. Wang et al., Optica 5, 1438 (2018) (the duty cycle, p. 1439)"
    doi: 10.1364/OPTICA.5.001438
validation:
  - nonlinear/sinc2-half-maximum
  - nonlinear/sinc2-length-squared
  - nonlinear/fejer-table-1-coherence
  - nonlinear/fejer-table-1-wavelength
  - nonlinear/fejer-table-1-temperature
  - nonlinear/fejer-table-1-tilt-about-z
  - nonlinear/fejer-table-1-tilt-toward-z
  - nonlinear/wang-2018-duty-cycle
  - material/linbo3-zelmon-opo
  - material/linbo3-jundt-opo
  - nonlinear/coupled-undepleted
  - nonlinear/coupled-lossy
  - nonlinear/coupled-shg-tanh
  - nonlinear/coupled-sfg-sech
  - nonlinear/coupled-opa-cosh
  - nonlinear/coupled-manley-rowe
  - nonlinear/coupled-rk4-order
  - nonlinear/pairs-gaussian-purity
---

A second- or third-order process converts light efficiently only when its waves stay in step:
the polarization they drive and the wave it radiates must keep the same phase along the guide.
`photonoxide::nonlinear::phase` computes how far out of step they are, the wave-vector mismatch
Δk, and what fixes it: a periodically poled grating, the birefringence of a crystal, or a
waveguide's dispersion. It gives the response of a device of length L to a detuning, and the
detunings it accepts, in wavelength, temperature and angle.

`photonoxide::nonlinear::coupled` follows the waves along the guide when the process does deplete:
the coupled-amplitude equations of second-harmonic, sum- and difference-frequency generation, with
loss. `photonoxide::nonlinear::pairs` gives photon pairs' joint spectra and their purity. The
efficiencies from a mode solver's overlaps come next, on the same conventions.

## Conventions

Fixed once, for everything in `nonlinear`:

- **Units.** Vacuum wavelengths in µm; wave numbers k = 2πn/λ and mismatches Δk in rad/µm;
  lengths and periods in µm; temperatures in kelvin; angles in radians, inside the crystal.
- **A process** is an energy balance: ω₃ = ω₁ + ω₂ for three waves, ω_a + ω_b = ω_c + ω_d for four.
  The side that holds the highest frequency is the *high side*.
- **The sign of Δk.** Δk = Σ k(high side) − Σ k(other side) − K_m, with K_m the grating's wave
  vector. For three waves this is Δk = k₃ − k₁ − k₂ − K_m, Fejer et al.'s Eq. (20); for SHG,
  k₂ − 2k₁ − K_m. It is positive in a normally dispersive medium without a grating. Boyd's
  textbook writes the opposite sign, Δk = k₁ + k₂ − k₃ (Eq. (2.2.11)); only the sign differs.
  For four waves with a degenerate pump the high side is the signal and idler, so
  Δk = k_s + k_i − 2k_p, the form common in fibre and silicon photonics.
- **The grating** is a periodic sign of the nonlinear coefficient, period Λ, duty cycle D (the
  fraction of a period at +d). Its m-th harmonic has K_m = 2πm/Λ and amplitude
  G_m = (2/(mπ)) sin(mπD) (Eq. (22)). A grating designed for a process points K_m along that
  process's own mismatch, so Λ = 2πm/|Δk₀| works for either sign.
- **Types** follow Boyd's Table 2.3.2: type I, the two lower-frequency waves share a
  polarization; type II, theirs are orthogonal; type 0, all three share one, which only
  quasi-phase matching reaches (LiNbO₃'s d₃₃, for example).
- **Uniaxial crystals.** An extraordinary wave at the angle θ to the optic axis has
  1/n²(θ) = cos²θ/n_o² + sin²θ/n̄_e² (Boyd, Eq. (2.3.8)). θ = 90° is noncritical.
- **The response** of an undepleted process is sinc²(ΔkL/2), sinc x = sin x / x (Eq. (19)). It
  is ½ at ΔkL/2 = ±1.391557…, Fejer's 0.4429π.
- **Acceptance bandwidths** are full widths at half maximum (FWHM) of the response against one
  variable, the others held.
- **Tilts** follow Fejer's Fig. 2(b): the input waves travel at an angle ν to the reference axis
  (the grating's wave vector and the entrance face's normal); the wave that energy conservation
  fixes (the harmonic, the sum, the idler) takes the transverse wave vector the others give it
  (Eq. (37)), and Δk is measured along the axis (Eq. (38)). In an anisotropic crystal each wave's
  index follows its own direction (Eq. (A1.1)).
- **Temperature** moves the indices through their models; with a thermal expansion ε(T) the
  length and the period both grow by (1 + ε), so the phase is Δk·L(T)/2 with
  K_m(T) = K_m/(1 + ε) (Eqs. (42)–(43)).

## In code

```rust
use photonoxide::material::catalogue::Axis;
use photonoxide::nonlinear::phase::{Device, Expansion, Polarization, State, Type, Uniaxial, Variable};
use photonoxide::units::Wavelength;

// congruent LiNbO₃ with Edwards & Lawrence's n_o(T) and n_e(T)
let ln = Uniaxial::catalogue("linbo3", Some("edwards-lawrence-1984"), Some("edwards-lawrence-1984"))?;
let state = State::celsius(25.0);
// d₃₃ SHG of 1.064 µm: every wave extraordinary (type 0)
let shg = ln.second_harmonic(Wavelength::um(1.064)?, Type::Zero(Polarization::Extraordinary), &state)?;
println!("l_c = {:.2} µm", shg.coherence_length(&state)?); // 3.40
// 1 mm, first-order QPM at a 50 % duty cycle, the grating cut for this process
let device = Device::quasi_phase_matched(shg, 1000.0, state, 1, 0.5)?
    .with_expansion(Expansion::LINBO3_JUNDT);
println!("period {:.3} µm", device.grating().unwrap().period()); // 6.802
let dt = device.acceptance(Variable::Temperature)?;
println!("δT = {:.1} K", dt.linear); // 25.8
```

An index can be a catalogue model's axis (`Index::catalogue`, rebuilt at each temperature), any
`Material`, or a mode's effective indices given as data (`Index::effective`, or
`Index::effective_over_temperature` for n_eff at several temperatures). The processes are
`Process::second_harmonic`, `sum_frequency`, `difference_frequency` (also parametric
amplification and down-conversion) and `four_wave`, each with a tuning: which waves move when
the wavelength changes (for SHG the fundamental, the harmonic following; for difference
frequency the signal, the pump held). `Process::with_tuning` sets another.

## The mismatch and quasi-phase matching

Δk₀, the material (or modal) mismatch, is the sum of the waves' wave numbers with their sides'
signs. Its coherence length l_c = π/|Δk₀| is the distance over which the driven and free waves
slip by π (Eq. (2)). A grating of period Λ = 2πm/|Δk₀| = 2m l_c cancels it at order m.

The grating's duty cycle sets how much of the nonlinear coefficient is left:
d_Q = (2/(mπ)) sin(mπD) d (Eqs. (22)–(23)). At 50 % and first order, 2/π. Wang et al. poled thin-film
LiNbO₃ at a 4.1 µm period with a duty cycle of 39 % ± 3 % and write that it "reduces the actual
d_eff by 7 %". Eq. (22) gives 5.91 %; ±3 % on the duty cycle moves this between 3.14 % (42 %) and
9.52 % (36 %), so the paper's 7 % is within its own uncertainty:

| Duty cycle | d_Q / d_Q(50 %) | Cut |
|---|---|---|
| 50 % | 1 | 0 |
| 42 % | 0.9686 | 3.14 % |
| 39 % (Wang et al.) | 0.9409 | 5.91 % (theirs: 7 %) |
| 36 % | 0.9048 | 9.52 % |

`nearest_order` gives the order whose period is nearest a fabricated one, and the mismatch it
leaves.

## Birefringent phase matching

`Uniaxial` holds a crystal's n_o and n̄_e, and builds the processes of each type with the
polarizations of Boyd's Table 2.3.2 for the crystal's sign. `phase_matching_angle` finds the
angle θ to the optic axis where Δk₀ = 0 (scanned down from 90° and refined by bisection);
`phase_matching_temperature` finds the temperature at a fixed angle, noncritical at 90°. A unit
test checks type-I angle tuning against the closed form
sin²θ = (n_o1⁻² − n_o2⁻²)/(n̄_e2⁻² − n_o2⁻²) to 1e-12 rad.

## The response and the acceptance bandwidths

`Device` is a process over a length, with its grating (or none), thermal expansion and
operating point. `Device::response` is sinc²(ΔkL/2). At exact phase matching the undepleted power
L² sinc²(ΔkL/2) grows as L²: the case `nonlinear/sinc2-length-squared` finds the ratio of 2 mm to
1 mm to be 4 to round-off.

`Device::acceptance` takes the FWHM against the wavelength of the quoted wave, the temperature,
or a tilt about the optic axis or toward it, three ways:

- **linear**, Fejer's Eq. (25): δζ = 4·1.39156/(L |∂Δk/∂ζ|), from the first derivative. For SHG in
  wavelength it is Eq. (30), in temperature Eq. (44) with the expansion.
- **noncritical**, where the first derivative vanishes: 2·(4·1.39156/(L |∂²Δk/∂ζ²|))^½. For a tilt
  of an isotropic crystal this is Eq. (41); `first_order_tilt` is Fejer's Eq. (A1.9), the same
  curvature to first order in the birefringence.
- **numerical**, the width between the two points where the response is ½, found by stepping out
  and bisecting. Fejer's Table I prints these in parentheses where the linear estimate fails. It
  is NaN when a half-maximum point lies outside an index model's range (a temperature model's,
  say) and infinite when the response never falls to ½ (a noncritical birefringent tilt).

The derivatives are central differences (steps of 1e-4 λ, 0.1 K and 1 mrad); the validation
cases state how much halving the steps moves each value.

## Fejer et al.'s Table I

Fejer et al.'s Table I lists the coherence lengths and the acceptance bandwidths of SHG in 1 mm of
congruent LiNbO₃ at 25 °C, for three fundamentals and four interactions. It was computed with
Edwards and Lawrence's Sellmeier equations (their Ref. [24]), except δT at 0.86 µm, which used
Jundt et al.'s for lithium-rich material (Ref. [25], footnote f). Both are in the catalogue, the
first as the `linbo3` entry's `edwards-lawrence-1984` model, the second for these three values
only. The grating is first order at a 50 % duty cycle, the thermal expansion Jundt's
(1.54e-5 /K and 5.3e-9 /K² along the grating, Eq. (3)): with no expansion δT at 1.064 µm (d₃₃) is
27.6 K, not 25.8.

The table below has every printed value beside ours. Columns c and d are the tilt about the
crystal's z axis (n_e invariant, Eq. (41)) and about x or y (n_e varies with the tilt, Eq. (A1.9));
values in parentheses are numerical widths. ✓ marks a value within half a unit of its last printed
digit, which is how the five cases `nonlinear/fejer-table-1-*` check it: 41 of the 49 values of
the quasi-phase-matched rows.

| λ (µm) | Coefficient | l_c (µm) | δλ (nm) | δT (K) | δν, column c (mrad) | δν, column d (mrad) |
|---|---|---|---|---|---|---|
| 0.86 | d₃₁(Q) | 4.26 / 4.256 ✓ | 0.89 / 0.892 ✓ | 6.0 / 5.96 ✓ | 176 / 175.6 ✓ | 79 / 79.4 ✓ |
| | d₃₃(Q) | 1.67 / 1.668 ✓ | 0.72 / 0.715 ✓ | 13.7 / 13.72 ✓ | 112 / 111.9 ✓ | 103 / 102.8 ✓ |
| | d₂₂(Q) | 1.42 / 1.423 ✓ | 0.60 / 0.602 ✓ | 17.4 / 17.35 | 104 / 103.7 ✓ | 104 / 103.7 ✓ |
| 1.064 | d₃₁(Q) | 129 / 129.6 | 3.11 / 3.111 ✓ | 9.4 / 9.45 ✓ | 959 / 958.7 ✓, (916) / 916.1 ✓ | 103 / 102.5 |
| | d₃₃(Q) | 3.40 / 3.401 ✓ | 2.04 / 2.036 ✓ | 25.8 / 25.77 ✓ | 158 / 158.0 ✓ | 146 / 145.7 ✓ |
| | d₂₂(Q) | 2.92 / 2.923 ✓ | 1.74 / 1.739 ✓ | 64 / 64.2 ✓ | 147 / 146.8 ✓ | 147 / 146.8 ✓ |
| 2.12 | d₃₁(Q) | 14.4 / 14.42 ✓ | 40 / 40.8 | 27.5 / 27.53 ✓ | 317 / 317.0 ✓ | 139 / 176.8, (183) / 182.9 ✓ |
| | d₃₃(Q) | 15.6 / 15.79 | 62 / 61.6 ✓ | 193 / 192.7 ✓, (203) / — | 337 / 337.2 ✓ | 306 / 306.2 ✓ |
| | d₂₂(Q) | 13.4 / 13.44 ✓ | 55 / 54.7 ✓ | 379 / 379.1 ✓, (455) / — | 311 / 311.4 ✓ | 311 / 311.4 ✓ |

Printed / ours. Not reproduced, and not among the cases:

- **Six values differ by more than half their last digit**, by 0.3 % to 4 %:
  - l_c = 129 µm (d₃₁, 1.064 µm): ours is 129.6. This row is close to birefringent phase
    matching (n₂ − n₁ is 0.002), so l_c moves fast with the indices; but Fejer's own column c
    value, 959 mrad, which scales as √l_c, is consistent with 129.6 (129 would give 957).
  - l_c = 15.6 µm (d₃₃, 2.12 µm): ours is 15.79. Here too the printed column c value, 337 mrad, is
    consistent with ours (15.6 would give 335).
  - δλ = 40 nm (d₃₁, 2.12 µm): ours is 40.8.
  - δT = 17.4 K (d₂₂, 0.86 µm, lithium-rich): ours is 17.35, 0.05 K off; the numerical width is 17.35
    too.
  - δν = 103 mrad (d₃₁, 1.064 µm, column d): Eq. (A1.9) gives 102.5.
  - δν = 139 mrad (d₃₁, 2.12 µm, column d): Eq. (A1.9) gives 176.8. Here n₂ < n₁, so Δk′ is
    negative; Eq. (A1.9) with |Δk′| in place of Δk′ gives 139, and the true width is the
    numerical one, 183, which Fejer prints beside it and we reproduce.
- **The numerical (203) and (455) K** of d₃₃ and d₂₂ at 2.12 µm: their half-maximum points lie
  below 0 °C, outside the 0–500 °C Edwards and Lawrence fitted. Their linear values, 193 and 379,
  are reproduced.
- **The birefringent d₃₁(B) rows** are phase matched at −19.4 °C (1.064 µm) and 580.3 °C (2.12 µm),
  outside the same range; the catalogue doesn't extrapolate a model.

Eq. (A1.9) is first order in the birefringence. The full curvature of Δk in the tilt, which
`Acceptance::noncritical` takes, gives column d's widths 1 % to 3 % larger: 81.4, 103.9, 105.4,
147.1 and 309.2 mrad where (A1.9) gives 79.4, 102.8, 102.5, 145.7 and 306.2. The numerical widths
are within 0.5 % of the full curvature's.

## The coupled-amplitude equations

`nonlinear::coupled` integrates the amplitudes A_j (√W, |A_j|² the power) of a second-order
process along the guide, by the classical fourth-order Runge–Kutta method:

- **SHG** (`SecondHarmonic`): dA₁/dz = −iκ A₁* A₂ e^{iΔkz} − α₁A₁/2,
  dA₂/dz = −iκ A₁² e^{−iΔkz} − α₂A₂/2.
- **Three waves**, ω₃ = ω₁ + ω₂ (`ThreeWave`): dA₁/dz = −iκ(ν₁/ν₃) A₂*A₃ e^{iΔkz},
  dA₂/dz = −iκ(ν₂/ν₃) A₁*A₃ e^{iΔkz}, dA₃/dz = −iκ A₁A₂ e^{−iΔkz}, each with its loss; this is
  sum- and difference-frequency generation and parametric amplification, by the inputs given.

Δk is the module's (k₃ − k₁ − k₂ − K_m). Wang et al. write Δk = 2k₁ − k₂ for SHG, the opposite
sign; the response is the same. κ is set by the undepleted normalized efficiency, η = κ²:
P_2ω = η P_ω² L² sinc²(ΔkL/2), and P₃ = η P₁P₂L² sinc²(ΔkL/2) for a sum. `coupling_from_efficiency`
takes η in %/(W·cm²), `loss_from_db_per_cm` a loss in dB/cm.

The cases, each on 1000 steps over 4 mm with η = 4500 %/(W·cm²), Wang et al.'s calculated
efficiency (their 4 mm device's ideal transfer function, p. 1439):

| Case | Ours | Closed form | Error on 1000 steps | On 500 (the tolerance's basis) |
|---|---|---|---|---|
| Undepleted SHG, ΔkL/2 = 1 | 5.09813e-40 W | η P₀² L² sinc²(1), the same | 1.2e-14 | 1.8e-13 |
| The same with 3 and 6 dB/cm | 2.93367e-40 W | the closed form above, the same | 1.2e-14 | 1.7e-13 |
| Depleted SHG, κ√P₀L = 2 (Armstrong) | 0.929349 | tanh² 2 | 5.7e-14 | 9.3e-13 |
| Sum of equal photon fluxes, L = 2l (Armstrong) | 0.929349 | tanh² 2 | 5.7e-14 | 9.3e-13 |
| Parametric gain, ΓL = 2 | 14.1541 | cosh² 2 | 5.1e-13 | 8.2e-12 |
| Manley–Rowe fluxes, mismatched and depleted | drift 2.3e-15 | 0 | | |

The errors fall as the fourth power of the step: 4.06, 4.03 and 4.04 for the halvings from 125
steps in the depleted SHG case. Without loss, P₁ + P₂ (SHG) and the Manley–Rowe photon fluxes
P₁/ν₁ + P₃/ν₃ and P₂/ν₂ + P₃/ν₃ (Armstrong et al., Eq. (6.6)) are conserved by the equations,
and by the integration to round-off on 1000 steps.
## Photon pairs

`nonlinear::pairs` builds the joint spectral amplitude of spontaneous parametric
down-conversion, f(ν_s, ν_i) = α(ν_s + ν_i) Φ(ν_s, ν_i) (Grice and Walmsley, Eqs. (7)–(9)). The
pump's amplitude α is a Gaussian whose intensity has the FWHM Δν. Φ is the device's phase
matching, sinc(ΔkL/2), or its Gaussian stand-in exp(−γ(ΔkL/2)²), γ = ln 2/(2·1.39156²) = 0.17897,
whose square has the sinc²'s FWHM. Frequencies are ν = 1/λ in 1/µm. Any difference-frequency
`Device` of `phase` gives the mismatch, with its indices, grating and temperature.

The Schmidt decomposition is the singular value decomposition of f sampled on a grid. Its
weights λ_k give the purity P = Σλ_k² and the Schmidt number K = 1/P. With a Gaussian pump and
Gaussian phase matching, and Δk linear in the frequencies, f is a two-dimensional Gaussian,
exp(−½zᵀMz) with M = [[a, c], [c, b]], and its purity is P = √(1 − c²/(ab)): Tr ρ² in closed form.
The case `nonlinear/pairs-gaussian-purity` is such a source: 0.775 µm down-converted to 1.55 µm
over 1 cm, group indices 2.30, 2.25 and 2.40, a pump 0.6 nm wide. The sampled purity is 0.190323,
as the closed form gives:

| Points a side (±0.025 /µm) | Purity | Relative error |
|---|---|---|
| 101 | 0.190573 | 1.3e-3 |
| 201 | 0.190323 | 3.4e-13 |
| 301 | 0.190323 | 4.4e-13 |
| 401 | 0.190323 | 2.3e-12 |
| closed form | 0.190323 | |

From 201 points the grid's error is below the decomposition's round-off.

Grice and Walmsley's Eq. (8) writes the phase matching as sin(ΔkL)/(ΔkL), and its Eq. (11) and
cw bandwidth Δν_cw = 2.78/|k′_o − k′_e|L follow it. The integral of their Eq. (6) over the
crystal, −L/2 to L/2, gives sin(ΔkL/2)/(ΔkL/2), as Fejer et al.'s Eq. (19) does, and so a
bandwidth twice as wide, 5.57/|k′_o − k′_e|L. We use the integral's form. Zhao et al.'s thin-film
source (2020) is next: their paper gives the guide (300 nm of x-cut 5 % MgO:LN, 50 nm etch, 1.24
µm wide, 5 mm poled at 2.8 µm), so its modes can be solved; that is the modal phase matching still
to come.
## Optical parametric oscillators

The catalogue's PPLN OPO cases find their idler through this module: a difference-frequency
process, the pump held and the idler tuned, its grating first order. With Zelmon et al.'s n_e the
idler of a 1.064 µm pump at a 30 µm period is 3.354 µm (their Fig. 3: 3.35); with Jundt's n_e(T)
at 250 °C and a 25.5 µm period, expanded by his Eq. (3), it is 4.755 µm (his Fig. 1: 4.755). Both
are the numbers these cases gave before they were moved onto phase matching.

## Limits

- Uniaxial crystals; a biaxial one (KTP, LBO) only along its principal axes, with an `Index` per
  polarization: no biaxial index surface yet.
- The tilt geometry has the grating along the reference axis (Fejer's κ = 0) and the inputs
  collinear.
- Plane waves: focused beams (Boyd and Kleinman) are not in yet, nor the waveguide overlaps that
  give κ from the mode solver's fields. Wang et al.'s 4500 %/(W·cm²) needs their ridge's etch depth
  and sidewall, which the paper doesn't print; it is used here as a value of η only.

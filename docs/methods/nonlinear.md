---
title: "Phase matching"
module: nonlinear::phase
summary: "The wave-vector mismatch of any three- or four-wave process from any index (a catalogue model with its temperature, a material, or a mode's effective indices given as data); quasi-phase matching, its period, order and duty cycle; birefringent phase matching of types 0, I and II, noncritical or angle tuned; the undepleted response sinc²(ΔkL/2); the acceptance bandwidths in wavelength, temperature and angle, checked against Fejer et al.'s Table I; and the coupled-amplitude equations of SHG, sum- and difference-frequency generation, depleted and lossy, against Armstrong et al.'s closed forms; and all of it in waveguides, from the mode solver's effective indices and full-vector fields with the d tensor turned into the guide's frame, against a plane wave, a slab's exact fields and Zhao et al.'s poling period."
order: 30
papers:
  - cite: "M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, IEEE J. Quantum Electron. 28, 2631 (1992) (Eqs. (2), (19)–(25), (30), (41)–(44), (A1.1)–(A1.9); Table I)"
    doi: 10.1109/3.161322
  - cite: "R. W. Boyd, Nonlinear Optics, 4th ed., Academic Press (2020) (Eqs. (1.4.25), (1.4.28), (2.2.11), (2.3.8); Sec. 1.4.2; Table 2.3.2)"
    doi: 10.1016/C2015-0-05510-1
  - cite: "R. C. Miller, Appl. Phys. Lett. 5, 17 (1964) (Eq. (4), Miller's rule)"
    doi: 10.1063/1.1754022
  - cite: "I. Shoji, T. Kondo, A. Kitamoto, M. Shirane, R. Ito, J. Opt. Soc. Am. B 14, 2268 (1997) (Tables 10 and 12)"
    doi: 10.1364/JOSAB.14.002268
  - cite: "S. Afshar V., T. M. Monro, Opt. Express 17, 2298 (2009) (Eq. 42, γ)"
    doi: 10.1364/OE.17.002298
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
  - cite: "C. Wang et al., Optica 5, 1438 (2018) (the duty cycle, p. 1439; Eq. (1), the normalized efficiency)"
    doi: 10.1364/OPTICA.5.001438
  - cite: "J. Zhao, C. Ma, M. Rüsing, S. Mookherjea, Phys. Rev. Lett. 124, 163603 (2020) (p. 1 and Fig. 1(c), the guide and its period; p. 3, the duty cycle, the poling depth and the brightness)"
    doi: 10.1103/PhysRevLett.124.163603
  - cite: "L. Chang et al., Laser Photonics Rev. 12, 1800149 (2018) (Sec. 2, the strip and its phase matching; Sec. 4, the tuning)"
    doi: 10.1002/lpor.201800149
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
  - nonlinear/miller-linbo3-d33
  - nonlinear/miller-linbo3-d31
  - nonlinear/miller-linbo3-mgo-d33
  - nonlinear/miller-linbo3-mgo-d31
  - nonlinear/kerr-gamma-gaussian
  - nonlinear/guide-bulk-limit
  - nonlinear/guide-zhao-period
  - nonlinear/guide-zhao-wavelength
  - nonlinear/guide-zhao-area-signal
  - nonlinear/guide-zhao-area-pump
  - nonlinear/guide-plane-wave
  - nonlinear/guide-slab-overlap
  - nonlinear/guide-slab-order
  - nonlinear/guide-slab-coupled
  - nonlinear/guide-group-index
---

A second- or third-order process converts light efficiently only when its waves stay in step:
the polarization they drive and the wave it radiates must keep the same phase along the guide.
`photonoxide::nonlinear::phase` computes how far out of step they are, the wave-vector mismatch
Δk, and what fixes it: a periodically poled grating, the birefringence of a crystal, or a
waveguide's dispersion. It gives the response of a device of length L to a detuning, and the
detunings it accepts, in wavelength, temperature and angle.

`photonoxide::nonlinear::coupled` follows the waves along the guide when the process does deplete:
the coupled-amplitude equations of second-harmonic, sum- and difference-frequency generation, with
loss. `photonoxide::nonlinear::pairs` gives photon pairs' joint spectra and their purity.
`photonoxide::nonlinear::guide` puts all of them in a waveguide: the mismatch and the poling
period from the mode solver's effective indices, and the coupling κ from its full-vector fields
and the crystal's d tensor turned into the guide's frame ([In waveguides](#in-waveguides)).

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
bandwidth twice as wide, 5.57/|k′_o − k′_e|L. We use the integral's form. A waveguide's joint
spectrum takes its modes' indices ([photon pairs from a guide](#photon-pairs-from-a-guide)).

## Optical parametric oscillators

The catalogue's PPLN OPO cases find their idler through this module: a difference-frequency
process, the pump held and the idler tuned, its grating first order. With Zelmon et al.'s n_e the
idler of a 1.064 µm pump at a 30 µm period is 3.354 µm (their Fig. 3: 3.35); with Jundt's n_e(T)
at 250 °C and a 25.5 µm period, expanded by his Eq. (3), it is 4.755 µm (his Fig. 1: 4.755). Both
are the numbers these cases gave before they were moved onto phase matching.

## Miller's rule

The catalogue's d coefficients are measured at a few fundamentals. `nonlinear::miller` carries one
to another wavelength by Miller's rule. R. C. Miller (1964, Eq. (4)) wrote
d_ijk = χ_ii(ω₃) χ_jj(ω₁) χ_kk(ω₂) δ_ijk, with χ = n² − 1 the linear susceptibility of the index
each wave's polarization sees. He found δ nearly the same in every crystal he tabulated: "always
within a factor of two of the average value", and for one coefficient among crystals of one
symmetry class "all equal to within ±50 %" (p. 17). Boyd (Sec. 1.4.2, Eqs. (1.4.25) and (1.4.28),
pp. 26–27) derives the rule from the anharmonic oscillator, whose χ⁽²⁾ is exactly such a product
with a constant δ.

With δ held, d at new frequencies is d at the measured ones times the ratio of the products
(`miller_scale`); `miller_shg` does it for second-harmonic generation, from a d at one fundamental
to another, with the harmonic's index and the two fundamental fields' indices as materials. For
LiNbO₃'s d₃₃ all three are n_e; for d₃₁, n_e at the harmonic and n_o at the fundamental. The rule
is a nonresonant model: every wave below the band edge.

The catalogue has Shoji et al.'s d₃₃ and d₃₁ of congruent and 5 % MgO-doped LiNbO₃ at three
fundamentals (Table 10). Carried from 1.064 µm with the entries' default indices (Zelmon et al.),
they come out like this:

| Crystal | d | 1.064 µm, measured | 1.313 µm: Miller, measured | Deviation | 0.852 µm: Miller, measured | Deviation |
|---|---|---|---|---|---|---|
| LiNbO₃ | d₃₃ | 25.2 pm/V | 23.58, 19.5 | +20.9 % | 28.12, 25.7 | +9.4 % |
| LiNbO₃ | d₃₁ | 4.6 | 4.29, 3.2 | +34.2 % | 5.15, 4.8 | +7.3 % |
| MgO:LiNbO₃ | d₃₃ | 25.0 | 23.43, 20.3 | +15.4 % | 27.84, 28.4 | −2.0 % |
| MgO:LiNbO₃ | d₃₁ | 4.4 | 4.11, 3.4 | +20.9 % | 4.92, 4.9 | +0.4 % |

Toward the blue (0.852 µm) the rule follows the measurements to 0.4 to 9 %; toward the infrared
(1.313 µm) the measured d falls faster than the susceptibilities, and the rule lands 15 to 34 %
above. Shoji et al.'s own Miller's Δ₃₃ (Table 12: 3.92, 4.73 and 4.34 × 10⁻¹³ m/V at 1.313,
1.064 and 0.852 µm, which `material/linbo3-shoji-miller` reproduces) shows the same: δ is not
constant across this crystal's spectrum to better than about 20 %. The cases
(`nonlinear/miller-*`) take Miller's stated ±50 % as the model's accuracy and show the deviation;
nothing is fitted. GaAs's d₁₄ is measured at 1.533 and 1.064 µm, but the second harmonic of
1.064 µm, 532 nm, lies above GaAs's band edge and outside its index models, so the rule isn't
applied there.

## The Kerr coefficient in a guide

The catalogue has n₂ and two-photon absorption with their papers (`catalogue::kerr`, the
[catalogue's write-up](catalogue.md#kerr-coefficients-and-two-photon-absorption)). A guide's
nonlinear coefficient is γ = 2π n₂ / (λ A_eff) (Afshar & Monro, Eq. 42 with n₂ uniform), from
the mode's effective area: `Fields::kerr_coefficient` takes a catalogue value,
`Fields::nonlinear_coefficient` a number in m²/W, and `mode::quantities::nonlinear_coefficient`
the closed form with any area. For a Gaussian mode of area π µm² and silicon nitride's n₂
(2.4 × 10⁻¹⁵ cm²/W at 1548 nm), γ is 0.310 /(W·m), the closed form to round-off
(`nonlinear/kerr-gamma-gaussian`). A value holds at its own wavelength: silicon's n₂ changes
fourfold from 850 to 2200 nm, and nothing here moves it.

## In waveguides

`nonlinear::guide` takes a process into a waveguide: its waves are the guide's modes, from the
full-vector mode solver, and its coupling is their overlap through the crystal's d tensor.

### Conventions

Those above, and:

- **The guide's frame** is the cross-section's: x across, y up, z along. A crystal is put on the
  chip by its cut and the guide runs along the chip's x or y, as for the Pockels effect
  ([electro-optic](electro-optic.md)): the permittivity and the d tensor are turned by both
  (`SecondOrder::rotated`, `SecondOrder::in_section`, d'_abc = Σ M_ai M_bj M_ck d_ijk). An x-cut
  film along the crystal's y has c across, so its TE modes meet d₃₃; a z-cut film has c up, and
  its TM modes do.
- **A mode's index over wavelength** (`ModeIndex`) is the fundamental mode of one family (TE-like
  or TM-like, by the share of H in H_y) solved at Chebyshev points across a range, 5, 9, 17, … of
  them, until the interpolants on the last two sets differ by less than a tolerance in n_eff. Each
  wavelength's cross-section is built from the materials there (`Ridge`), so the materials'
  dispersion is in it. The group index n_g = n_eff − λ dn_eff/dλ and the group-velocity
  dispersion β₂ = (λ³/2πc²) d²n_eff/dλ² come from the interpolant's own derivatives, and as an
  `Index` it is what every process above takes: Δk, the period, the bandwidths and the joint
  spectrum then come from the modes.
- **Fields per √W.** Each mode's fields are scaled to carry 1 W, ½ Re ∫ (E × H*)·ẑ dA = 1 W, so a
  wave of amplitude A (√W, as `nonlinear::coupled` has it) has the field A ê(x, y) e^{i(βz − ωt)},
  ê in V/m per √W.
- **The polarization** of these fields (the real field their real part), with d = χ⁽²⁾/2, is
  P(2ω) = ε₀ Σ d_ijk E_j E_k for SHG and P(ω₃) = 2ε₀ Σ d_ijk E₁ⱼ E₂ₖ for a sum (Boyd's Eqs.
  (2.7.8)–(2.7.9), whose amplitudes are half these).
- **The coupling.** A mode of 1 W driven by a polarization grows as
  dA/dz = (iω/4) ∫ ê*·P dA e^{−iβz}: the first-order perturbation the thermo-optic and Pockels
  changes use, with P = ε₀ Δε E. With the overlap O = ∫ Σ ê₃ᵢ* d_ijk ê₁ⱼ ê₂ₖ dA over the crystal,
  the sum's coupling is κ = (ω₃ε₀/2)|O| (`sum_frequency_coupling`) and SHG's κ = (ωε₀/2)|O|, ω the
  fundamental's (`second_harmonic_coupling`), half the sum's of two equal waves. These are the κ
  of `ThreeWave` and `SecondHarmonic`, and η = κ² is the normalized efficiency, in %/(W·cm²) by
  `percent_per_watt_cm2`. A grating multiplies κ by its Fourier coefficient G_m.
- **The plane-wave limit.** Fields uniform over an area A give η = 2ω²d²/(ε₀c³n_ω²n_2ω A): Boyd's
  Eq. (2.7.11) with his I = 2nε₀c|A|² (Eq. (2.2.16)) over the area, and C. Wang et al.'s Eq. (1)
  (Optica 5, 1438 (2018)) for uniform modes, A_2ω/A_ω² = 1/A: the convention of their %/(W·cm²).
- **Photon pairs.** Down-conversion ω_p → ω_s + ω_i is difference-frequency generation of the same
  κ seeded by the vacuum, one photon per mode, an idler of hν_i per unit bandwidth. From the
  three-wave equations, undepleted, the signal's rate per unit of its frequency is
  dR/dν_s = κ²(ν_sν_i/ν_p²) P_p L² sinc²(ΔkL/2) pairs per second per hertz
  (`pair_rate_density`); at degeneracy κ is twice SHG's, so this is η P_p L².

The overlap takes the fields at the cells' centres with E's normal component kept discontinuous at
each interface (the one-sided fields). A window halved by a mirror wall adds each mode's image with
its components' parities (an electric wall keeps E's normal component even, a magnetic one its
tangential ones), so a half cross-section, which halves the solve, gives the whole one's κ.

### In code

```rust
use photonoxide::material::catalogue::{Conditions, entry};
use photonoxide::material::{CrystalAxis, Orientation, vacuum};
use photonoxide::mode::pockels::Along;
use photonoxide::nonlinear::guide::{Family, Grid, ModeIndex, Ridge};
use photonoxide::nonlinear::phase::{self, Device, Grating, Process, State};
use photonoxide::units::Wavelength;

// Zhao et al.'s guide: 300 nm of x-cut MgO:LN etched 50 nm, 1.24 µm wide, 75° sidewalls
let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z))?;
let silica = entry("sio2").unwrap().default_model().unwrap()
    .materials(Conditions::default())?.remove(0);
let ridge = Ridge {
    film: 0.3, etch: 0.05, width: 1.24, angle: 75.0,
    core: entry("linbo3-mgo").unwrap().oriented(None, Conditions::default(), x_cut)?,
    substrate: silica, cladding: vacuum(), along: Along::Y,
};
let grid = Grid::new(0.01).halved(); // 10 nm cells, x ≥ 0 behind a mirror wall
let fundamental = ModeIndex::of_ridge(&ridge, &grid, Family::Te, 4, (1.50, 1.65), 1e-10)?;
let harmonic = ModeIndex::of_ridge(&ridge, &grid, Family::Te, 4, (0.75, 0.825), 1e-10)?;
let (f, h) = (fundamental.index()?, harmonic.index()?);
let shg = Process::second_harmonic(Wavelength::um(1.57)?, f.clone(), f, h);
println!("period {:.3} µm", phase::period(shg.mismatch(&State::default())?, 1)?); // 2.815
let device = Device::new(shg, 5000.0, State::default())?
    .with_grating(Grating::new(2.8, 1, 0.5)?);
println!("matched at {:.4} µm", device.phase_matching_wavelength(1.51, 1.64)?); // 1.5583
```

`ModeField::new` scales a solved mode's fields to 1 W; `second_harmonic_coupling` and
`sum_frequency_coupling` give κ from three of them, the d tensor in the guide's frame and each
cell's share of the crystal (`Ridge::film_share`, which can keep only a poled depth). κ goes
straight into `SecondHarmonic` or `ThreeWave` with the modes' Δk and losses.

### The mismatch from the modes

Where the mode is a plane wave, its Δk must be the crystal's. The case `nonlinear/guide-bulk-limit`
fills a 0.2 µm square window with x-cut lithium niobate between walls that hold a uniform field
along c (electric walls normal to it, magnetic ones along it): a uniform field is then the scheme's
exact mode, and n_eff the crystal's n_e. SHG of 1.064 µm from these modes, interpolated over
1.04–1.09 and 0.52–0.545 µm, has the bulk Δk of `Uniaxial` to 3.9e-15 relative, against a tolerance
of 3e-9 (the interpolants' 1e-10 in n_eff through Δk's difference of indices).

Why Chebyshev points and not the adaptive sweep of `photonoxide::sweep`: its AAA fit was tried
first. On a slab's 2.5 nm cells the rational fits put spurious poles in the samples' last digits,
and the estimate stalled at 1e-6 after 60 samples. The polynomials settle to 1e-10 in n_eff, their
derivatives with them: 17 Chebyshev points across 0.15 µm for each of Zhao et al.'s modes below.

### Zhao et al.'s guide

Zhao et al. (Phys. Rev. Lett. 124, 163603 (2020), p. 1 and Fig. 1(c)) give their guide whole: 300
nm of x-cut 5 mol% MgO-doped lithium niobate on 1.8 µm of silica over silicon, etched 50 nm to a
ridge 1.24 µm wide with 75° sidewalls, air above, the guide along the crystal's y (p. 3), 5 mm
poled. They poled it at "the calculated QPM period, Λ = 2.8 µm, to match the refractive indices of
the lowest-order TE-polarized waveguide modes at 1570 and 785 nm" (p. 1). They don't name an index
model; ours is the catalogue's default, Zelmon et al.'s (21 °C). The silicon is left out: the
window stops 1.5 µm under the film.

| Cells | n_eff TE₀, 1.57 µm | n_eff TE₀, 0.785 µm | Period (µm) | A_eff, 1.57 µm (µm²) | A_eff, 0.785 µm |
|---|---|---|---|---|---|
| 20 nm | 1.693544 | 1.972185 | 2.8172 | 1.0788 | 0.3970 |
| 10 nm | 1.694002 | 1.972865 | 2.8150 | 1.0772 | 0.3975 |
| 5 nm | 1.694073 | 1.973020 | 2.8142 | 1.0768 | 0.3975 |
| Zhao et al. | | | 2.8 | 1.1 | 0.4 |

The grid study puts the 5 nm period's error at 5.2e-4 µm, and the paper's 2.8 holds 2.75 to 2.85
(`nonlinear/guide-zhao-period`). Poled at 2.8 µm, the guide phase-matches SHG of 1.5583 µm on 10 nm
cells (1.5600 on 20 nm; `nonlinear/guide-zhao-wavelength`), 11.7 nm short of their 1570: across
the period's half digit, 2.75 to 2.85 µm, the matched wavelength moves by ±38.9 nm. The guide's
group indices are 2.1306 at 1.57 µm and 2.3308 at 0.785 µm, and its β₂ at 1.57 µm is 1220 fs²/mm
(10 nm cells).

Their modes' effective areas, 1.1 µm² at 1570 nm and 0.4 µm² at 785 nm (p. 1), are the guide's too
(`nonlinear/guide-zhao-area-signal`, `-pump`). The paper doesn't define its area; the table's is
Afshar and Monro's Eq. 41, from S_z, which converges on these grids. Their Eq. 44, from |E_t|⁴,
takes the ridge's corners twice and drifts with the grid (1.110, 1.107 and 1.103 µm² at 1.57 µm;
0.400, 0.401 and 0.401 at 0.785 µm), and rounds to the paper's numbers just the same.

### Chang et al.'s GaAs strip: not reproduced

Chang et al. (Laser Photonics Rev. 12, 1800149 (2018), Sec. 2) phase-match SHG of 2 µm in a GaAs
strip in silica by its modes alone: TE₀₀ at 2 µm and TM₀₀ at 1 µm have one index when 150 nm of
GaAs is 1.5 µm wide. Their supporting information, with the geometry in full, isn't at hand; the
main text gives the thickness and the width, the silica on every side (p. 2), a 5 nm SiN bonding
layer whose index it doesn't give, and nothing on the sidewalls. Taken vertical, without the SiN,
with Skauli et al.'s indices (the catalogue's default), on 10 nm cells:

| Width | n_eff TE₀₀, 2 µm | n_eff TM₀₀, 1 µm | Difference |
|---|---|---|---|
| 1.5 µm | 2.11630 | 2.16303 | −0.0467 |
| 2.0 µm | 2.15526 | 2.17221 | −0.0170 |
| 2.6 µm | 2.17629 | 2.17725 | −0.0010 |

The indices meet at a width of 2.655 µm (2.694 on 20 nm cells), not 1.5; Gehrsitz et al.'s indices
(the catalogue's other GaAs model) give 2.681. The width moves the match slowly, so the gap reads
better in wavelength: 1.5 µm wide, our strip phase-matches near 2.066 µm, 66 nm (3.3 %) above
their 2 µm. Its sensitivities agree with theirs: the matched wavelength moves by 10.4 nm per nm of
thickness and −0.127 nm per nm of width, where the paper has 11 nm and 1 nm per 10 nm (Sec. 4).
At 10.4 nm per nm, 66 nm is what 6.3 nm less GaAs would take away. It isn't a case: an open
gap, with what would settle it (the supporting information's cross-section and the SiN's index) in
the list of gaps against papers.

### The overlap and the efficiency

Two closed forms check κ. In the plane-wave window above, z-cut so the uniform field lies along c
(d₃₃ = 25.2 pm/V, Shoji et al.'s at 1.064 µm), η from the overlap is the plane-wave
2ω²d²/(ε₀c³n_ω²n_2ω A) to 6.7e-16 (`nonlinear/guide-plane-wave`): the fields' scale to 1 W, ε₀, the
factor ½ and the units hold to round-off. A 400 nm slab of lithium niobate (n_e, c along its
plane) in silica, uniform between walls, has exact TE fields, cos inside and exponentials outside
(`mode::slab`): η from them, integrated by a midpoint rule whose cells end on the interfaces,
against η from the full-vector fields (`nonlinear/guide-slab-overlap`, `nonlinear/guide-slab-order`).
η of a slab falls as its window's height H, so the table has η·H, the efficiency of 1 µm of it:

| Cells | η·H, %/(W·cm²)·µm | Relative error |
|---|---|---|
| 10 nm | 41655.66 | 2.8e-5 |
| 5 nm | 41654.79 | 7.1e-6 |
| 2.5 nm | 41654.57 | 1.8e-6 |
| exact fields | 41654.49 | |

The errors fall at order 1.96, and the grid study's estimate of the 2.5 nm error, 1.8e-6, is the
error itself.

### Zhao et al.'s efficiency and brightness: not reproduced

The same guide's κ from its TE₀ modes, with Zhao et al.'s own d₃₃ = 27 pm/V (p. 1) alone, gives:

| Cells | η, 50 % duty, poled through | η, 55.2 % duty, poled 245 nm | B, poled through | B, 55.2 %, 245 nm |
|---|---|---|---|---|
| 20 nm | 4158.9 %/(W·cm²) | 2974.7 | 1.040 × 10⁷ | 7.437 × 10⁶ |
| 10 nm | 4162.5 | 2981.6 | 1.041 × 10⁷ | 7.454 × 10⁶ |
| 5 nm | 4163.5 | 2984.5 | 1.041 × 10⁷ | 7.461 × 10⁶ |
| Zhao et al. | | | 4.6 × 10⁵ | |

B is the pairs' brightness in pairs/(s·GHz·mW) over 5 mm, η P_p L² per hertz (above). The second
and fourth columns take the paper's measured duty cycle, 55.2 % ± 3.0 %, and its poling depth,
"closer to 245 nm" (p. 3), read as the film poled from its top down to 245 nm: our reading, which
the paper doesn't spell out. Their brightness, 4.6 × 10⁵ pairs/(s·GHz·mW) (45 MHz/mW of on-chip
pairs over a 100 GHz filter, p. 3), is 22.6 times below ours for an ideal grating and 16.2 times
below with their duty cycle and depth. The paper doesn't print a calculated efficiency, so the gap
can't be put on the overlap or on the measurement; it isn't a case, and it is listed with the
other gaps against papers. Nor does the paper give the "normalized mode-overlap integral" of 78 %
(p. 1) a definition, so it isn't compared.

### SHG along a guide

κ and the modes' Δk go straight into the coupled equations. `nonlinear/guide-slab-coupled` takes
the slab's κ (5 nm cells) and its mismatch from its modes, against a grating 0.1 % longer than the
one that cancels it, with 3 and 6 dB/cm of loss over 4 mm: the harmonic's power by RK4 is the
undepleted lossy closed form to 4.5e-12 on 1000 steps, the error falling as the step's fourth power
(1.2e-9, 7.2e-11, 4.5e-12 and 2.8e-13 on 250 to 2000 steps). The power in is 1 aW so that the
undepleted closed form holds: the slab's window is 20 nm tall, and its η is large.

### Photon pairs from a guide

A guide's joint spectrum is `nonlinear::pairs`'s, its `Device` built from the modes' indices: the
pump's and the signal's and idler's `ModeIndex::index`, and the grating. The phase matching then
follows each wave's own group index and dispersion, which come from the interpolant's derivatives.
`nonlinear/guide-group-index` checks them where an exact value exists: a 400 nm slab of index 2.2
in 1.45, without material dispersion, whose TE₀ group index is ⟨ε⟩/n_eff from its exact field
(Hellmann–Feynman on the TE equation):

| Cells | n_g at 1.55 µm | Relative error |
|---|---|---|
| 10 nm | 2.223324 | 6.5e-5 |
| 5 nm | 2.223433 | 1.6e-5 |
| 2.5 nm | 2.223460 | 4.0e-6 |
| exact | 2.223469 | |

Zhao et al.'s guide, pumped at 0.785 µm and poled for degeneracy at 1.57 µm, gives pairs whose
signal spans 55.9 nm at half maximum over its 5 mm (1.5425 to 1.5985 µm, 10 nm cells): a broad
spectrum, since at degeneracy Δk changes only at second order (its β₂, above). The paper prints no
bandwidth, purity or Schmidt number to compare with, and its main text doesn't give the pump's
spectrum. A pump much narrower than this, a continuous laser say, ties ν_s + ν_i to ν_p: the pairs
are then entangled in frequency, and their purity, as `JointSpectrum::schmidt` gives it, falls
toward 0 as the pump narrows.

## Limits

- Uniaxial crystals; a biaxial one (KTP, LBO) only along its principal axes, with an `Index` per
  polarization: no biaxial index surface yet.
- The tilt geometry has the grating along the reference axis (Fejer's κ = 0) and the inputs
  collinear.
- Focused bulk beams (Boyd and Kleinman) are not in yet.
- In a guide, the crystal must have a principal axis along it (the full-vector solver's
  condition), and a mode is followed by its family and index, not through a crossing with another
  mode of its family. Wang et al.'s 4500 %/(W·cm²) (Optica 5, 1438) needs their ridge's etch depth
  and sidewall, which the paper doesn't print: it is not reproduced, and stands here as a value of
  η only.

---
title: "Phase matching"
module: nonlinear::phase
summary: "The wave-vector mismatch of any three- or four-wave process from any index (a catalogue model with its temperature, a material, or a mode's effective indices given as data); quasi-phase matching, its period, order and duty cycle; birefringent phase matching of types 0, I and II, noncritical or angle tuned; the undepleted response sinc²(ΔkL/2); the acceptance bandwidths in wavelength, temperature and angle, checked against Fejer et al.'s Table I; and the coupled-amplitude equations of SHG, sum- and difference-frequency generation, depleted and lossy, against Armstrong et al.'s closed forms; and all of it in waveguides, from the mode solver's effective indices and full-vector fields with the d tensor turned into the guide's frame, against a plane wave, a slab's exact fields and Zhao et al.'s poling period. Third-order processes from the same modes: pulses by the nonlinear Schrödinger equation (β₂, β₃, β₄ from the modes, γ from the effective area, solitons and self-phase modulation against Agrawal's closed forms), four-wave mixing's parametric gain and photon pairs, and Kerr microresonators by the Lugiato–Lefever equation, against Lugiato and Lefever's steady states and instability and Coen et al.'s soliton."
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
  - cite: "G. P. Agrawal, Nonlinear Fiber Optics, 6th ed., Academic Press (2019) (Eqs. (2.3.23)–(2.3.46), (2.4.1)–(2.4.13), (3.1.5), (4.1.4)–(4.1.16), (5.2.3)–(5.2.24), (10.1.8), (10.2.1)–(10.2.26), (10.3.6), (10.4.5)–(10.4.12))"
    doi: 10.1016/C2018-0-01168-8
  - cite: "L. A. Lugiato, R. Lefever, Phys. Rev. Lett. 58, 2209 (1987) (Eqs. (1), (2), (4)–(6))"
    doi: 10.1103/PhysRevLett.58.2209
  - cite: "S. Coen, H. G. Randle, T. Sylvestre, M. Erkintalo, Opt. Lett. 38, 37 (2012) (Eq. (3); Fig. 2)"
    doi: 10.1364/OL.38.000037
  - cite: "T. Herr et al., Nat. Photonics 8, 145 (2013) (Eqs. (1)–(2); Methods)"
    doi: 10.1038/nphoton.2013.343
  - cite: "T. J. Kippenberg, A. L. Gaeta, M. Lipson, M. L. Gorodetsky, Science 361, eaan8083 (2018) (Eqs. (1)–(3))"
    doi: 10.1126/science.aan8083
  - cite: "M. H. P. Pfeiffer et al., Optica 3, 20 (2016) (pp. 23–24, the soliton's width)"
    doi: 10.1364/OPTICA.3.000020
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
  - nonlinear/nlse-dispersion-silica-beta2
  - nonlinear/nlse-dispersion-silica-beta4
  - nonlinear/nlse-fundamental-soliton
  - nonlinear/nlse-second-order-soliton
  - nonlinear/nlse-third-order-period
  - nonlinear/nlse-split-step-order
  - nonlinear/nlse-spm-phase
  - nonlinear/nlse-spm-broadening
  - nonlinear/nlse-photon-number
  - nonlinear/fwm-gain-matched
  - nonlinear/fwm-gain-mismatched
  - nonlinear/fwm-invariants
  - nonlinear/fwm-nlse-gain
  - nonlinear/fwm-sfwm-purity
  - nonlinear/lle-homogeneous-lower
  - nonlinear/lle-homogeneous-upper
  - nonlinear/lle-instability-rate
  - nonlinear/lle-split-step-order
  - nonlinear/lle-coen-2012-duration
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

The third-order processes take the same modes. `photonoxide::nonlinear::pulse` follows pulses
along a Kerr guide by the nonlinear Schrödinger equation, its β₂, β₃, β₄ from the modes' index
over wavelength and its γ from their effective area ([Pulses along a Kerr guide](#pulses-along-a-kerr-guide)).
`photonoxide::nonlinear::fwm` gives four-wave mixing's gain and conversion
([Four-wave mixing](#four-wave-mixing)), and `photonoxide::nonlinear::resonator` Kerr
microresonators by the Lugiato–Lefever equation ([Kerr microresonators](#kerr-microresonators)).

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

The example `qpm_fejer` prints the whole table beside the paper's, checking the 41 values and listing the other eight.

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

### Filters and heralding

A pair source's photons go through filters before they are counted. `JointSpectrum::filtered`
multiplies the amplitude by the filters' amplitude transmissions, t_s(ν_s)t_i(ν_i), on the
spectrum's own grids; any component's response gives them, `circuit::transmission(component,
from, to, values, wavelengths)` reading S_to,from from its S-matrix (a ring's drop port, say, with
its insertion loss), at λ = 1/ν. The filtered amplitude's `schmidt` is the filtered pairs' purity.
`JointSpectrum::heralding` gives each photon's heralding efficiency, the share of its partner's
counts it comes with:

η_s = Σ|f|²|t_s|²|t_i|² / Σ|f|²|t_i|², η_i = Σ|f|²|t_s|²|t_i|² / Σ|f|²|t_s|²,

its spectral part: a detector's efficiency and the paths' losses multiply it. A filter narrower
than a photon's marginal spectrum raises the purity and costs its partner's heralding.

For a Gaussian |f|² = e^{−zᵀMz} through Gaussian filters |t|² = e^{−ν²/w²} each sum is a Gaussian
integral, π/√det, so η_s = (det(M + D_i)/det(M + D_s + D_i))^½ with D_s = diag(1/w_s², 0) and
D_i = diag(0, 1/w_i²) (`gaussian_heralding`), and the filtered purity is the Gaussian's,
(1 − c²/(a′b′))^½ with the filters' 1/w² added to M's diagonal. The cases take
M = [[1, −0.9], [−0.9, 1]] × 10⁶ µm², pairs 10⁻³ /µm wide and anticorrelated, through filters
0.8 and 1.2 × 10⁻³ /µm wide, on 201 points a side 10⁻² /µm either way: the signal's η_s is
0.5004078, and the filters raise the purity from 0.436 to 0.9019150 (`nonlinear/pairs-heralding-gaussian`,
`nonlinear/pairs-filtered-purity`). The window has to hold the pairs' tails along their
correlation: 6 × 10⁻³ /µm either way cut them at 7.7 × 10⁻¹⁰ of η_s.

## Optical parametric oscillators

The catalogue's PPLN OPO cases find their idler through this module: a difference-frequency
process, the pump held and the idler tuned, its grating first order. With Zelmon et al.'s n_e the
idler of a 1.064 µm pump at a 30 µm period is 3.354 µm (their Fig. 3: 3.35); with Jundt's n_e(T)
at 250 °C and a 25.5 µm period, expanded by his Eq. (3), it is 4.755 µm (his Fig. 1: 4.755). Both
are the numbers these cases gave before they were moved onto phase matching.

`nonlinear::opo` puts the guide in a cavity. `Oscillator` holds a `ThreeWave` (1 the signal, 2 the
idler, 3 the pump), its length, and the cavity's `Feedback` for the signal and, if doubly
resonant, the idler: the share R of the power at the guide's end returned to its start, with a
round-trip phase φ from resonance, A_j(0) ← √R_j e^{iφ_j}A_j(L), and the share T ≤ 1 − R that
leaves as output. The pump goes through once.

- **Threshold** (`Oscillator::threshold`): the pump power where the undepleted round trip first
  returns (A₁, A₂*) as large as they started, its 2 × 2 matrix's spectral radius 1. The matrix is
  the guide's transfer, taken from the coupled equations with seeds 10⁻⁹ of the pump's amplitude,
  times the feedback; bisection finds the power to 10⁻¹³. Phase matched, lossless and on
  resonance this is Boyd's Eq. (2.9.8) (R. W. Boyd, *Nonlinear Optics*, 3rd ed., Academic Press
  (2008)), cosh gL = 1 + l₁l₂/(2 − l₁ − l₂), l_j = 1 − √R_j, with g² = κ²ν₁ν₂P/ν₃²; singly
  resonant (l₂ = 1) it is cosh gL = 1/√R₁. A 1.5 µm signal and 3.0 µm idler from 1.0 µm,
  κ = 10⁻⁴ /(√W·µm), 1 cm: doubly resonant (√R₁ = 0.95, √R₂ = 0.9) at 24.31338 mW, within the
  bisection's 10⁻¹³ of Eq. (2.9.8) on every step count; singly (√R₁ = 0.9) at 982.0113 mW, the
  error RK4's (1.3e-10, 7.9e-12, 4.9e-13 and 6.5e-14 on 50 to 400 steps;
  `nonlinear/opo-threshold-doubly`, `-singly`). Boyd's low-loss limits, g²L² = l₁l₂ and 2l₁
  (Eqs. (2.9.9)–(2.9.10)), follow from it.
- **Steady state** (`Oscillator::steady_state`): the round trips run with the depleted coupled
  equations from a weak signal until the signal's power at the guide's end changes by less than a
  tolerance. In it each signal photon gained per pass leaves through the coupler, and by the
  Manley–Rowe relations each is a pump photon lost: the singly resonant oscillator above
  (R₁ = 0.9) at twice threshold balances them to 8.1e-10, 5.0e-11 and 2.3e-12 on 100 to 400 steps a
  pass (`nonlinear/opo-steady-photons`, on 200).
- **The uniform-signal limit**: where the signal loses little per round trip it is nearly the same
  all along the guide. Holding A₁ fixed, the pump goes as cos gz and the idler as sin gz, so the
  pump's depletion is sin²gL and the signal's gain per pass is ∝ sin²gL/g², which at threshold is
  its L². Above threshold, at N = P/P_th, x = gL solves x²/sin²x = N and the depletion is
  D = sin²x (`uniform_depletion`): 0.9682 at N = 2, all of the pump at N = π²/4, and less beyond
  as the pump comes back. The steady state approaches it at first order in the loss: at N = 2 the
  gap is 1.56e-3, 7.75e-4, 3.86e-4 and 1.93e-4 for 4, 2, 1 and 0.5 % lost per round trip
  (`nonlinear/opo-uniform-limit-order`).

A ring resonant at its pump too (a triply resonant oscillator, or a Kerr comb, whose threshold the
mean-field equation gives: [Pu et al.'s ring](#pu-et-als-algaas-ring-the-comb-threshold)) is
`Feedback` on the pump, not in yet.

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

The example `pairs_zhao` computes the period, the matched wavelength and the two areas on 20 and 10 nm cells, and the pairs' signal spectrum pumped at 785 nm.

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

## Pulses along a Kerr guide

`nonlinear::pulse` follows a pulse along a guide whose index depends on its intensity, by the
nonlinear Schrödinger equation (NLSE) of G. P. Agrawal's *Nonlinear Fiber Optics* (6th ed., 2019).
The guide is three numbers and a list: its dispersion β₂, β₃, β₄, … at the carrier, from the
mode solver's effective index over wavelength; its nonlinear coefficient γ = 2πn₂/(λA_eff), from
the mode's effective area and the catalogue's n₂ ([above](#the-kerr-coefficient-in-a-guide)); and
its loss.

### Conventions

- **Units.** Distance z in m; time T in ps, in the frame moving at the group velocity
  (T = t − β₁z, Agrawal's Eq. (2.3.44)); angular frequencies in rad/ps; β_k = d^kβ/dω^k in
  ps^k/m (1 ps²/m is 1000 ps²/km, or 1000 fs²/mm); γ in 1/(W·m); the power loss α in 1/m; the
  amplitude A in √W, |A|² the power.
- **The carrier** is e^{−iω₀t} (Eq. (2.3.2)): a spectral component e^{−iΩT} sits at ω₀ + Ω, so
  Ω > 0 is bluer. The spectrum is Ã(Ω) = ∫A(T) e^{iΩT} dT (Eq. (2.4.12)).
- **The equation** is Agrawal's Eq. (2.3.36) with γ₁ = γ/ω₀ and α₁ = 0 (p. 44):
  ∂A/∂z = −(α/2)A + i Σ_{k≥2} (i^kβ_k/k!) ∂^kA/∂T^k + iγ(1 + (i/ω₀)∂/∂T)[A ∫R(T′)|A(T − T′)|² dT′],
  R(T) = (1 − f_R)δ(T) + f_R h_R(T). Self-steepening (the 1/ω₀ term) and the delayed Raman
  response h_R are optional and off by default; without them it is the NLSE, Eq. (2.3.46), with
  β₃, β₄, … kept. h_R is Eq. (2.3.39), silica's τ₁ = 12.2 fs, τ₂ = 32 fs and f_R = 0.18 given
  (`Raman::SILICA`, p. 42).
- **Split-step.** The symmetrized split-step Fourier method, Eq. (2.4.8), with the half steps of
  dispersion between two steps merged (Eq. (2.4.10)). Dispersion and loss are exact in the
  spectrum, exp(h(iΣβ_kΩ^k/k! − α/2)). The Kerr step alone is exact too, A e^{iγ|A|²h}
  (Eq. (4.1.3)); with Raman or self-steepening it is the classical fourth-order Runge–Kutta
  method. The scheme is second order in h (p. 47).
- **The grid** is a power of two of times, T_j = (j − n/2)δT, periodic as the FFT makes it
  (p. 49). The transforms are a radix-2 FFT of this crate's own, on one thread, so a run is the same
  bit for bit whatever the number of threads.

### In code

```rust
use photonoxide::nonlinear::pulse::{Dispersion, Guide, Pulse, soliton_power, soliton_period};

// a guide of β₂ = −0.05 ps²/m and γ = 1 /(W·m) at 1.55 µm, no loss
let guide = Guide::new(Dispersion::new(1.55, &[-0.05])?, 1.0, 0.0);
// its fundamental soliton of T₀ = 100 fs: P₀ = |β₂|/(γT₀²) = 5 W
let p0 = soliton_power(1.0, 0.1, -0.05);
let start = Pulse::sech(p0, 0.1, 1024, 5.0)?;
let end = guide.propagate(&start, 5.0 * soliton_period(0.1, -0.05), 4000)?;
println!("{:.4} W", end.peak_power()); // 5.0000 W
```

From a waveguide's modes, `Dispersion::of_mode` takes a `ModeIndex` ([above](#conventions-1)) and
`Fields::kerr_coefficient` the mode's γ; `Guide::with_raman` and `Guide::with_self_steepening` add
the higher-order terms.

### The dispersion from the modes

β(ω) = ωn_eff(ω)/c, so β_k = (ω₀n^(k) + k n^(k−1))/c with n^(k) the k-th derivative of n_eff in ω.
`Dispersion::of_index` samples n at Chebyshev points in ω across a range of wavelengths, 9, 17,
33 and 65 of them, and differentiates the interpolants. More points shrink the interpolation's
error but magnify the samples' round-off in a high derivative, roughly as the points' number to
the 2k-th power, so each β_k is taken from the pair of successive interpolants that agree best on
it, and their difference is its stated error. `Dispersion::of_mode` does this on a mode's
interpolant and on its interpolant on half the points (`ModeIndex::coarse`), and states the larger
change: what the mode solver's samples leave in each β_k.

A Sellmeier medium's β_k are known exactly: n² = 1 + ΣB_i/(1 − C_i²ω²/(2πc)²) has a Taylor
series in ω, and so has β. Silica (Malitson's formula, the catalogue's) at 1.55 µm, fitted across
1.0–2.2 µm:

| | Fit | Taylor series | Error | Stated error |
|---|---|---|---|---|
| β₂ (ps²/m) | −2.794736629 × 10⁻² | −2.794736614 × 10⁻² | 5.6e-9 | 4.9e-9 |
| β₃ (ps³/m) | 1.513408910 × 10⁻⁴ | 1.513408715 × 10⁻⁴ | 1.3e-7 | 1.2e-7 |
| β₄ (ps⁴/m) | −4.944259814 × 10⁻⁷ | −4.944270069 × 10⁻⁷ | 2.1e-6 | 1.9e-6 |

Relative errors. A narrower range holds β₄ worse: 1.45–1.65 µm gives it to 1e-7 but states 8.8e-6,
and 1.3–1.8 µm to 7.3e-6 (`nonlinear/nlse-dispersion-silica-beta2`, `-beta4`). β₂ = −27.9 ps²/km
is bulk silica's familiar value at 1.55 µm.

A silicon nitride strip in silica from its modes: 1.75 µm wide and 0.85 µm high, the
cross-section of Pfeiffer et al.'s Fig. 5 (Optica 3, 20 (2016), p. 23), straight, the catalogue's
default indices (Luke et al.'s nitride, Malitson's silica), its fundamental modes interpolated on
33 Chebyshev points across 1.2–2.0 µm, at 1.55 µm, with the catalogue's n₂ of the nitride
(2.4 × 10⁻¹⁵ cm²/W) for γ:

| Cells | Mode | n_g | β₂ (ps²/m) | β₃ (ps³/m) | β₄ (ps⁴/m) | A_eff (µm²) | γ (1/(W·m)) |
|---|---|---|---|---|---|---|---|
| 20 nm | TE₀ | 2.11596 | −0.154295 | 2.4492 × 10⁻⁴ | 7.015 × 10⁻⁷ | 1.1357 | 0.8566 |
| 10 nm | TE₀ | 2.11604 | −0.154668 | 2.4674 × 10⁻⁴ | 6.859 × 10⁻⁷ | 1.1357 | 0.8566 |
| 20 nm | TM₀ | 2.14284 | −0.176852 | −2.0316 × 10⁻⁴ | 4.115 × 10⁻⁶ | 1.1380 | 0.8549 |
| 10 nm | TM₀ | 2.14290 | −0.177328 | −2.0024 × 10⁻⁴ | 4.088 × 10⁻⁶ | 1.1384 | 0.8546 |

The fits' stated errors are 4 × 10⁻¹⁰ ps²/m in β₂ and below 4 × 10⁻¹¹ ps³/m and 3 × 10⁻¹² ps⁴/m: the
grid's error, the change from 20 to 10 nm cells, is far larger (0.24 % in β₂, 2.2 % in β₄ of TE₀).
β₂ of TE₀ on 20 nm cells is −0.154271 ps²/m by a central difference of β(ω) over three separate
solves 20 rad/ps apart, and −0.154295 from the interpolant's own λ-derivative (`ModeIndex::gvd`).
The strip's fundamental soliton of T₀ = 100 fs needs 18.1 W (TE₀, 10 nm cells).

In a ring of Pfeiffer et al.'s FSR, 95.65 GHz, these give D₂/2π = (c/n_g)D₁²|β₂|/2π = 1.260 MHz
(TE₀) and 1.426 MHz (TM₀) on 10 nm cells. Pfeiffer et al. measured 0.49 MHz for the TM₀ family
of their 238.2 µm ring and simulated 0.26 MHz ("we attribute deviations to inaccuracy in the values
of material dispersion", p. 23). They don't give the indices they simulated with, and ours is a straight
guide, not their bend, so this is not a case; the factor of three between their measurement and
our straight guide is listed with the gaps against papers.

### Solitons

With β₂ < 0 a pulse √P₀ sech(T/T₀) is a soliton of order N = (γP₀T₀²/|β₂|)^½ (Eq. (5.2.3)). The
fundamental one, N = 1, keeps its shape and gains the phase z/2L_D, L_D = T₀²/|β₂| (Eq. (5.2.16));
the second-order one has a closed form (Eq. (5.2.23)); every higher order returns to its input's
power profile after the soliton period z₀ = (π/2)L_D (Eq. (5.2.24), p. 144). The cases take
β₂ = −0.05 ps²/m, γ = 1 /(W·m), T₀ = 0.1 ps (P₀ = 5 W, z₀ = 0.314 m), 1024 points across 40 or
50 T₀:

| Split steps | N = 1, after 5z₀ | N = 2, at z₀/2 | N = 3, after z₀ |
|---|---|---|---|
| 500 | 1.8e-4 | 2.6e-4 | 3.4e-3 |
| 1000 | 4.4e-5 | 6.6e-5 | 8.1e-4 |
| 2000 | 1.1e-5 | 1.6e-5 | 2.0e-4 |
| 4000 | 2.8e-6 | 4.1e-6 | 5.0e-5 |
| 8000 | 7.0e-7 | 1.0e-6 | 1.3e-5 |

The largest difference from the closed form over the field's peak (N = 1, 2), and of |A|² from
the input's over 9P₀ (N = 3). Each halving divides the error by 4.00: the scheme's second order
(`nonlinear/nlse-split-step-order`). On 2048 points the third-order soliton's errors are the same
to three digits, so the time grid adds nothing at these steps.

### Self-phase modulation

Without dispersion the pulse keeps its shape and its peak gains the phase φ_max = γP₀L_eff,
L_eff = (1 − e^{−αL})/α (Eqs. (4.1.4)–(4.1.6)). A Gaussian's RMS spectral width grows by
(1 + 4φ_max²/(3√3))^½ (Eq. (4.1.16)), which holds exactly for the RMS width.

| Case | Ours | Closed form | Error |
|---|---|---|---|
| φ_max, 2 cm at 3 dB/cm, γP₀ = 230 /m, 400 steps | 2.493234 rad | 2.493236 rad | 5.0e-7 |
| RMS broadening at φ_max = 4.5π, 1024 points | 12.443951 | 12.443951 | 2.2e-16 |

The phase's error is the lumped step's midpoint rule for the loss, second order (7.9e-6, 2.0e-6,
5.0e-7, 1.2e-7 on 100 to 800 steps). Without loss and dispersion the scheme is exact, and the
broadening's error is the time grid's: 1.8e-8 on 512 points, round-off on 1024 and 2048.

### Raman and self-steepening

With both, the equation conserves the number of photons, ∝ ∫|Ã|²/(ω₀ + Ω) dΩ, but not the energy,
part of which the Raman response takes (Agrawal, p. 43). A third-order soliton of 50 fs with
β₃ = 0.001 ps³/m over 0.1 m loses 3.40 % of its energy; its photon number drifts by 2.0e-8,
1.4e-9, 8.7e-11 and 5.8e-12 on 500 to 4000 steps (`nonlinear/nlse-photon-number`, on 2000).
Without self-steepening the Raman term only moves the phase, so the energy is kept, and a soliton
still moves to lower frequencies (the soliton self-frequency shift, Sec. 5.5.4).

Agrawal's Eq. (2.3.45) gives the Raman response's first moment as T_R = f_Rτ₁τ₂²/(τ₁² + τ₂²),
1.92 fs for silica; the integral ∫T h_R(T) dT of his Eq. (2.3.39) is 2τ₁²τ₂/(τ₁² + τ₂²), which
gives 1.46 fs. Both are near the "about 2 fs" the text gives. Only h_R's transform enters the
propagation, so the difference touches nothing here; `Raman::first_moment` is the integral.

### Two-photon and free-carrier absorption

In silicon below its half band gap, and in other semiconductors there, two photons can be absorbed
together, and the carriers they free absorb more and lower the index. `Guide::with_two_photon`
gives an `AbsorbingGuide`, the NLSE with Lin, Painter and Agrawal's terms (Q. Lin, O. J. Painter,
G. P. Agrawal, Opt. Express 15, 16604 (2007), Eqs. (37), (40) and (47)–(49)):

∂A/∂z = … + iγ|A|²A − (β_T/2ā)|A|²A + (n₀/n)(ik₀σ_n − σ_a/2)N A,
∂N/∂T = β_T|A|⁴/(2ħω₀ā²) − N/τ₀.

`TwoPhoton` holds β_T (m/W, the catalogue's two-photon coefficient), the mode's effective area ā,
and the `FreeCarriers` if any: their absorption σ_a and index change σ_n per carrier, their
effective lifetime τ₀ (recombination, diffusion and drift together, Eq. (36)), and n₀/n, the
material's index over the mode's, by which the mode feels them more (Eq. (33)).
`FreeCarriers::silicon` takes Lin et al.'s Eq. (20) at 1.55 µm, σ_a = 1.45 × 10⁻²¹ m² and
σ_n = −5.3 × 10⁻²⁷ m³, each scaled by (λ/1.55 µm)², the holes' share of σ_n five times the
electrons' as they take it for 10¹⁶ to 10¹⁷ cm⁻³. The carriers are integrated along the pulse
from none at the window's start, the generation linear between samples and the decay exact (the
trapezoidal rule without recombination); a train's carriers left from earlier pulses are not
held. Two-photon absorption alone keeps the nonlinear step exact, |A|² → P/(1 + 2bPh) and the
phase (γ/2b) ln(1 + 2bPh), b = β_T/2ā; with the carriers (or Raman or self-steepening) it is RK4,
the carriers taken again at each stage. The terms sit outside the self-steepening operator, as
Lin et al.'s Eq. (47) has them.

| Case | Ours | Closed form | Error | Order |
|---|---|---|---|---|
| 20 W Gaussian (T₀ = 1 ps) over 2 cm at 2 dB/cm, β_T = 0.5 cm/GW, ā = 0.3 µm²: the peak's power (Eq. (60)) | 1.48641 W | P₀e^{−αL}/(1 + β_TP₀L_eff/ā) | 1.8e-7 on 400 steps | 2.00 |
| Its Kerr phase, γ = 20 /(W·m) (Eq. (61)) | 2.01401 rad | (γā/β_T) ln(1 + β_TP₀L_eff/ā) | 1.1e-7 | 2.00 |
| The carriers behind a 10 W Gaussian, no recombination (Eq. (51)) | 2.71652 × 10²² m⁻³ | √π β_TP₀²T₀/(2√2ħω₀ā²) | round-off | |
| 0.5 W continuous over 5 cm, τ₀ = 100 ps: the power | 0.292306 W | below | 1.7e-8 on 100 steps | 4 |
| Its phase, the Kerr's and the carriers' | −1.46557 rad | below | 2.2e-7 | 4 |

The power's and the phase's errors are the loss's midpoint rule in the split-step (2.9e-6, 7.2e-7,
1.8e-7 and 4.5e-8 on 100 to 800 steps for the power), the nonlinear step being exact; the
continuous wave's are RK4's (4.4e-6, 2.8e-7, 1.7e-8 and 1.1e-9 on 25 to 200).

With the carriers steady, N = cP², c = β_Tτ₀/(2ħω₀ā²), a continuous wave loses power as
dP/dz = −aP² − κP³, a = β_T/ā, κ = (n₀/n)σ_a c, whose solution is
z(P) = 1/(aP) − 1/(aP₀) + (κ/a²) ln[P(a + κP₀)/(P₀(a + κP))]; its phase, γ∫P dz + (n₀/n)k₀σ_n c∫P² dz,
is (γ/a) ln[P₀(a + κP)/(P(a + κP₀))] + ((n₀/n)k₀σ_n c/κ) ln[(a + κP₀)/(a + κP)]. The window of the
continuous cases is 4096 ps on 1024 points, 41 lifetimes, so the carriers at its end are steady to
e⁻⁴⁰; without dispersion each sample evolves on its own, and the error is RK4's alone
(`nonlinear/nlse-tpa-power`, `-phase`, `-order`, `nonlinear/nlse-carriers-gaussian`,
`nonlinear/nlse-fca-cw-power`, `-phase`). Over 5 cm at 0.5 W two-photon absorption alone would
lose 1.51 dB (P₀/(1 + aP₀L) = 0.3529 W); the carriers add 0.82 dB, and turn the phase negative:
their index change outweighs the Kerr's.

### Cross-phase modulation between two pulses

Two pulses at different wavelengths share the guide's nonlinearity. `xpm::TwoPulses` holds each
one's `Guide` (its dispersion about its own carrier, its γ and loss), the walk-off d = 1/v_g2 −
1/v_g1 (ps/m) and the cross-phase factor c (2 for co-polarized pulses), and integrates Agrawal's
Eqs. (7.4.1)–(7.4.2) with each pulse's loss and higher orders: ∂A₁/∂z = D₁A₁ + iγ₁(|A₁|² +
c|A₂|²)A₁, and A₂'s with d∂A₂/∂T on its left, in the first pulse's frame. The overlaps of
Eq. (7.1.10) are taken as one area, as Eqs. (7.1.11)–(7.1.13) take them, and the four-wave terms
between the pulses aren't carried. The split-step is the NLSE's: the second pulse's linear step
carries e^{idΩh}, which moves it to later T by dh, and the Kerr step is exact.

Without dispersion the pulses keep their shapes and their phases are Eqs. (7.4.5)–(7.4.7); for
two Gaussians of one width, φ₁ = γ₁L[P₁e^{−τ²} + (cP₂/2)(√π/δ)(erf(τ − τ_d) − erf(τ − τ_d − δ))],
δ = dL/T₀ (Eq. (7.4.9), `xpm::gaussian_phase`). On Agrawal's Fig. 7.2 (γ₁P₁L = 40,
P₂/P₁ = 0.5, γ₂/γ₁ = 1.2, δ = 5) the largest |A₁(L)/A₁(0) − e^{iφ₁}| over |T| ≤ 3T₀ is 2.9e-5,
7.1e-6, 1.8e-6 and 1.3e-6 on 500 to 4000 steps (1024 points across 64 ps): second order to 2000
steps, then near a floor of about 1e-6 we haven't traced (`nonlinear/xpm-walk-off-phase`, on
2000, and `nonlinear/xpm-split-step-order`).

## Four-wave mixing

`nonlinear::fwm` gives four-wave mixing with one pump, 2ω_p = ω_s + ω_i, signal and idler at
ω_p ± Ω (Agrawal, Chapter 10).

### Conventions

- **The linear mismatch** is Δk = β_s + β_i − 2β_p (Eq. (10.1.8)), the sign `phase` uses for four
  waves. From a guide's dispersion, `Dispersion::four_wave_mismatch` gives
  Δk = 2Σ_m β_{2m}Ω^{2m}/(2m)! (Eq. (10.4.12)), β₂Ω² + β₄Ω⁴/12 to fourth order (Eq. (10.3.6)).
  Foster et al. (Nature 441, 960 (2006)) write Δk_L = 2k_p − k_s − k_i, the opposite sign; their
  gain, (γPΔk_L − (Δk_L/2)²)^½, is the one below.
- **The effective mismatch** κ = Δk + 2γP (Eq. (10.2.21)) holds the pump's self- and
  cross-phase modulation; the parametric gain is g = ((γP)² − (κ/2)²)^½ (Eq. (10.2.19), r = 1),
  largest, γP, at κ = 0.
- **Gain and conversion** with no idler in: P_s(L)/P_s(0) = 1 + (γP sinh(gL)/g)²
  (Eq. (10.4.7)) and P_i(L)/P_s(0) = (γP sinh(gL)/g)² (Eq. (10.4.6)). Where |κ/2| > γP, g is
  imaginary and sinh(gL)/g is sin(|g|L)/|g|: the signal oscillates, and far from phase matching
  the gain is 1 + (γPL)² sinc²(κL/2) (Eq. (10.4.8)).
- **Along the guide** (`FourWave`), the coupled equations of the three waves, Eqs. (10.2.1)–(10.2.4)
  with one pump, all overlaps 1/A_eff and one γ: depleted, with every self- and cross-phase term.
  They conserve the total power and P_s − P_i (p. 422), which the classical fourth-order
  Runge–Kutta method keeps to its own error.

### The closed forms

A 1 W pump, γ = 1 /(W·m) and 2 m, the signal from 10⁻¹² W:

| Steps | Gain at κ = 0 (14.154116) | Gain at Δk = 1 /m (1.4951794) | Invariants, a 0.1 W signal |
|---|---|---|---|
| 100 | 4.7e-7 | 9.1e-9 | 1.0e-7 |
| 200 | 3.0e-8 | 5.3e-10 | 6.4e-9 |
| 400 | 1.9e-9 | 3.3e-11 | 4.0e-10 |
| 800 | 1.4e-10 | 3.4e-12 | 2.5e-11 |

Relative errors, falling as the step's fourth power; the cases take 800 steps
(`nonlinear/fwm-gain-matched`, `-mismatched`, `-invariants`). The undepleted closed form's own
error, the signal's 10⁻¹² of the pump's power, is far below these.

### From the NLSE

One envelope holds the pump and its sidebands (Eq. (10.2.26)), so the NLSE gives the same gain. A
1 W CW pump and a signal at the gain's peak, Ω = (2γP/|β₂|)^½ = 6.32 rad/ps (p. 423), with
β₂ = −0.05 ps²/m over 2 m, against Eq. (10.4.7) with Δk = β₂Ω² from the dispersion: 14.154107
against 14.154116, 6.4e-7, on 2000 split steps (1.0e-5, 2.6e-6, 6.4e-7 and 1.6e-7 on 500 to 4000;
`nonlinear/fwm-nlse-gain`).

### Photon pairs by spontaneous four-wave mixing

`pairs::joint_spectrum` takes a four-wave process with one pump (`Process::four_wave` with the
pump twice). Each pump photon is taken at (ν_s + ν_i)/2 in the phase matching, and the pump's
spectrum is that of two photons, the pump's amplitude convolved with itself (`Pump::two_photon`): a
Gaussian whose intensity is √2 times as wide. With indices linear in the wavelength the mismatch
k_s + k_i − 2k_p((ν_s + ν_i)/2) is linear in the frequencies, and the joint amplitude is a
two-dimensional Gaussian whose purity is known in closed form ([photon pairs](#photon-pairs)). A
1.55 µm pump into 1.50 µm and its idler over 1 cm, group indices 2.05, 1.96 and 2.00, a pump 0.24
nm wide:

| Points a side (±0.025 /µm) | Purity | Relative error |
|---|---|---|
| 101 | 0.9464046366 | 2.7e-8 |
| 201 | 0.9464046109 | 1.0e-13 |
| 301 | 0.9464046109 | 5.0e-14 |
| closed form | 0.9464046109 | |

### Foster et al.'s silicon guide: not reproduced

Foster et al. measured parametric gain in silicon guides 300 × 550 nm (Nature 441, 960 (2006)). A
reproduction would need what the letter doesn't fully give: the cladding above the guide, the n₂
it assumes (it cites (4–9) × 10⁻¹⁴ cm²/W), and the two-photon and free-carrier absorption its
measured gain includes: the NLSE has those now ([two-photon and free-carrier
absorption](#two-photon-and-free-carrier-absorption)), but not the letter's β_T, carrier lifetime
or pulses' rate. It is not a case.

## Resonant enhancement in rings

`nonlinear::cavity` gives SHG and four-wave mixing in a ring resonant at their waves, from the
0.4 ring components: `AllPassRing::ring_wave(wavelength, values)` gives a wave's `RingWave`, its
round trip a e^{iφ} (the guide's loss and the fixed loss per round trip) and the field just past
the coupler for a unit field in the bus, F = ik/(1 − r a e^{iφ}): Helt et al.'s field-enhancement
factor (L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010), after Eq. (5)),
with the loss, in the coupler's convention (through r, across ik). Each wave has its own ring:
a harmonic's coupling isn't the fundamental's. Every loss of the round trip is spread along it,
α = −2 ln a/L.

One pass generates the new wave as the undepleted coupled equations do, each driving field
decaying from its value past the coupler; the generated field G at the pass's end builds up over
the round trips and leaves through the coupler as F e^{iφ}G, so P_out = |F|²|G|²:

- **SHG** (`cavity::second_harmonic`): P_2ω = |F_2ω|²κ²|F_ω|⁴P_ω²|e^{−α₂L/2}(e^{xL} − 1)/x|²,
  x = −α₁ + α₂/2 − iΔk, the lossy undepleted closed form with |F_ω|²P_ω inside.
- **Four-wave mixing** (`cavity::four_wave`): P_i/P_s = |F_i|²|F_s|²|F_p|⁴γ²P_p²|I|²,
  I = e^{−α_iL/2}(e^{xL} − 1)/x, x = −(2α_p + α_s − α_i)/2 − iΔk; lossless and on resonance it is
  Helt et al.'s Eq. (5), (γP_pL)²|F_s|²|F_i|²|F_p|⁴.

Waves all on resonance have ΔkL a whole number of turns, and only ΔkL = 0 converts: the azimuthal
orders must add up. The closed forms hold while the generated waves don't deplete the pump and the
pump's own Kerr phase doesn't detune the ring.

Two cases run the ring itself, round trip by round trip, with the depleted coupled equations
(`SecondHarmonic`, and `FourWave::propagate_with_loss`, which carries each wave's loss) and the
coupler, until the output changes by less than 10⁻¹⁴, and compare it with the closed forms
(1 nW in, so the depletion and the pump's Kerr phase are far below the errors):

| Case | Error on 5, 10, 20 and 40 RK4 steps a pass |
|---|---|
| SHG: the fundamental's ring on resonance near 1.55 µm (100 µm, coupling 0.05, 3 dB/cm), the harmonic's (n_eff 2.25, n_g 2.4, coupling 0.08) off its resonance as its indices put it, κ = 10⁻⁴ /(√W·µm), Δk = 0.002 rad/µm | 1.8e-9, 1.0e-10, 1.0e-12, 7.7e-12 |
| FWM: one ring at the pump's resonance and the two either side, γ = 100 /(W·m), Δk = 0 | 4.4e-13, 4.0e-13, 2.9e-13, 1.3e-13 |

RK4's error falls to a floor near 10⁻¹¹ for SHG and 10⁻¹³ for FWM, where the iteration stops
(`nonlinear/ring-shg-enhancement`, `nonlinear/ring-fwm-enhancement`, on 20 steps).

## Kerr microresonators

`nonlinear::resonator` integrates the Lugiato–Lefever equation (LLE), the mean-field equation of a
driven, damped Kerr cavity (L. A. Lugiato and R. Lefever, Phys. Rev. Lett. 58, 2209 (1987)), as it
describes a microresonator's comb (T. J. Kippenberg et al., Science 361, eaan8083 (2018); S. Coen
et al., Opt. Lett. 38, 37 (2012)).

### Conventions

- **The equation**, dimensionless: ∂ψ/∂τ = −(1 + iζ₀)ψ + i|ψ|²ψ + i ½∂²ψ/∂θ² + f. This is
  Kippenberg et al.'s Eq. (3) and Lugiato and Lefever's Eq. (1) with E = ψ, E_I = f, θ_LL = ζ₀,
  η = +1 and a = ½. τ = κt/2 is time in units of twice the photon lifetime; ζ₀ = 2Δ/κ the pump's
  detuning from the cold resonance in half linewidths, positive with the pump redder; f the drive;
  |ψ|² the power whose Kerr shift is half a linewidth.
- **Modes.** ψ is sampled at n points around the ring, each spectral component a comb line μ
  (μ > 0 bluer). Dispersion is each line's own detuning: dψ_μ/dτ = −(1 + i(ζ₀ + δ_μ))ψ_μ +
  i[|ψ|²ψ]_μ + fδ_{μ0}, with δ_μ = 2D_int(μ)/κ, D_int(μ) = D₂μ²/2 + D₃μ³/6 + … the resonances'
  departure from an equidistant comb (Kippenberg et al.'s Eq. (1)). With D₂ alone δ_μ = d₂μ²,
  d₂ = D₂/κ.
- **A ring from its waveguide** (`Ring`) is Coen et al.'s Eq. (3): round trip L and t_R, loss
  α = α_i + θ/2 per round trip, coupling θ, detuning δ₀, the waveguide's γ and β_k, pump P_in. Then
  ζ₀ = δ₀/α, f² = θγLP_in/α³, the power scale α/(γL), and δ_μ = −(L/α)Σβ_kΩ_μ^k/k!, Ω_μ = 2πμ/t_R:
  the guide's `Dispersion` from its modes goes straight in.
- **Split-step.** Strang splitting, each part exact: the linear one (loss, detuning, dispersion and
  the drive) in each mode, ψ_μ ↦ e^{L_μh}ψ_μ plus f(e^{L₀h} − 1)/L₀ in mode 0, and the Kerr one,
  ψ ↦ ψe^{i|ψ|²h}. Second order, its steady states too. The largest δ_μh must stay below π; above it
  the scheme's own instability grows the outermost lines.

### Steady states and bistability

A uniform ψ is stationary where f² = X(1 + (X − ζ₀)²), X = |ψ|² (Lugiato and Lefever's Eq. (2)),
ψ = f/(1 + i(ζ₀ − X)). Above ζ₀ = √3 the response is S shaped: between the turning points
X = (2ζ₀ ∓ (ζ₀² − 3)^½)/3 three states share one drive, and the middle one is unstable. At ζ₀ = 3
and f² = 4 the roots are 2 − √2, 2 and 2 + √2; the split-step settles on the lowest from ψ = 0 and
on the highest from 1.2 times its field:

| Steps over τ = 40 | Lower, 0.5857864 | Upper, 3.4142136 |
|---|---|---|
| 1600 | 6.1e-5 | 3.6e-4 |
| 3200 | 1.5e-5 | 8.9e-5 |
| 6400 | 3.8e-6 | 2.2e-5 |
| 12 800 | 9.5e-7 | 5.6e-6 |

Relative errors of the settled power: the steady state of the splitting is second order in the
step (`nonlinear/lle-homogeneous-lower`, `-upper`, on 12 800).

### Modulation instability

A small perturbation of a uniform state in the lines ±μ grows as e^{λτ}, λ the larger root of
λ² + 2λ + ζ = 0 with ζ = 1 + (X − ζ₀)(3X − ζ₀) + δ_μ(δ_μ − 2(2X − ζ₀)) (Lugiato and Lefever's
Eq. (4), their a(n) our δ_μ). It is unstable for X > 1 between δ_μ = 2X − ζ₀ ∓ (X² − 1)^½ (their
Eqs. (5)–(6)): where a comb starts. At ζ₀ = 1, X = 1.5, d₂ = 0.05 and μ = 5, λ = 0.2990381;
measured between τ = 10 and 20 the split-step gives it to 8.5e-5, 2.1e-5, 5.3e-6 and 1.4e-6 on 800
to 6400 steps each, second order (`nonlinear/lle-instability-rate`, `nonlinear/lle-split-step-order`).

### Dissipative Kerr solitons

Herr et al. (Nat. Photonics 8, 145 (2013), Eq. (1)) write the soliton as a sech of width
(2(ω₀ − ω_p)/D₂)^{−½} in the angle around the ring on a CW background. Its amplitude follows from
the same equation without loss and drive: i∂ψ/∂τ + ½ψ_θθ + |ψ|²ψ = ζ₀ψ has the stationary
soliton (2ζ₀)^½ sech((2ζ₀)^½θ), Agrawal's Eq. (5.2.15) with η² = 2ζ₀. `Resonator::soliton` puts it
on the lower uniform state, B sech(φ/W) e^{iφ₀}, B = (2ζ₀)^½, W = (d₂/ζ₀)^½. Its phase is ours: the
drive must make up the soliton's loss of energy, 2∫|ψ|² = 2Re∫fψ* over the pulse to first order,
which gives cos φ₀ = (8ζ₀)^½/(πf). That has a root only up to ζ₀ = π²f²/8 (`max_detuning`), the
approximation's limit of the soliton's existence.

The approximation against the soliton the split-step settles on, f = 3, d₂ = 0.004, 512 lines,
followed up in ζ₀ from 8 (`peak_width` takes the FWHM of |ψ|² from the field's band-limited
interpolation):

| ζ₀ | Peak \|ψ\|², settled | Peak, the approximation | FWHM (rad), settled | FWHM, the approximation |
|---|---|---|---|---|
| 8 | 17.41 | 14.88 | 0.03693 | 0.03942 |
| 9 | 19.09 | 17.17 | 0.03548 | 0.03716 |
| 10 | 20.74 | 19.50 | 0.03423 | 0.03525 |
| 11 | 22.17 | 22.06 | 0.03336 | 0.03361 |
| 11.25 | decays to the CW state | none | | |

π²f²/8 = 11.10: the settled soliton lasts to ζ₀ = 11 and is gone at 11.25. Inside, the
approximation's peak lies 0.5 to 14.5 % below the settled one and its width 0.8 to 6.7 % above: at
f = 3 the drive and the loss aren't small beside the soliton. With Coen et al.'s stronger drive
below (f = 8.31) the width agrees to 0.7 %. None of this is a case: no paper in hand gives the
approximation an accuracy to check it to.

### Coen et al.'s resonator

Coen et al. (Opt. Lett. 38, 37 (2012), Fig. 2 and p. 38) give a resonator whole: a critically
coupled MgF₂ disc, FSR 18.2 GHz, γ = 0.032 /(W·km), β₂ = −13 ps²/km, α = θ = 1.75 × 10⁻⁵,
P_in = 55.6 mW, L = 11.9 mm, δ₀ = 0.0012, and describe its comb's pulse as "a ∼400 fs duration
pulse with a peak power of 100 mW atop a weak cw background". In our units ζ₀ = 68.57, f = 8.315,
d₂ = 0.0578, and its loaded Q, ωt_R/2α, is 1.91 × 10⁹ against the caption's 1.90 × 10⁹. The
soliton settled from the approximate one on 512 lines:

| Step | FWHM (fs) | Peak power inside (W) | Out through the coupler, θ × peak (W) |
|---|---|---|---|
| 8 × 10⁻⁴ | 444.337 | 6365.8 | 0.1114 |
| 4 × 10⁻⁴ | 444.356 | 6361.0 | 0.1113 |
| 2 × 10⁻⁴ | 444.361 | 6359.8 | 0.1113 |
| the approximation | 447.54 | 6247.1 | |
| Coen et al. | ∼400 | 100 mW | |

On 1024 lines (at 10⁻⁴, to keep δ_μh below π) the FWHM moves by less than 0.01 fs. Our 444.4 fs is
within half a unit of the paper's one significant digit, ±50 fs (`nonlinear/lle-coen-2012-duration`);
read to two digits, 400 would not hold. The paper's 100 mW is the pulse's peak "intracavity" power,
but no soliton of this guide is that weak: a 400 fs soliton of β₂ = −13 ps²/km and γ = 0.032 /(W·km)
needs |β₂|/(γT₀²) ≈ 7.9 kW. Our peak inside is 6.36 kW, and what leaves through the coupler, θ
times it, is 0.111 W. The paper doesn't say which it plotted, so the power is not a case; it is
listed with the gaps against papers.

### Pu et al.'s AlGaAs ring: the comb threshold

Pu et al. (M. Pu, L. Ottaviano, E. Semenova, K. Yvind, Optica 3, 823 (2016)) measured the threshold
of parametric oscillation, where a comb begins, in AlGaAs-on-insulator rings: 3.1 mW in the bus
for a ring 25 µm across with an FSR of 995 GHz and a loaded Q of about 10⁵ (Fig. 4), its guide
320 × 630 nm with γ ∼ 660 /(W·m) (p. 823), the comb and the Q measured near 1590 nm. Their own
estimate is a band, from critical coupling (Q_C = 2Q_L) to under-coupling (Q_C = 10Q_L), and the
measurements lie in it (Fig. 4(d)).

The mean-field equation gives the threshold directly: the homogeneous state turns unstable at
|ψ|² = 1 ([modulation instability](#modulation-instability)), which the drive reaches first at
ζ₀ = 1 with f² = 1, so P_th = α³/(θγL) in the bus, α = ω₀t_R/(2Q_L) the field's loss per round
trip and θ = ω₀t_R/Q_C the coupler's power transmission. The example `comb_pu` takes the paper's
numbers into `Ring` and gives 0.684 mW critically coupled and 3.419 mW under-coupled; the measured
3.1 mW lies in the band, as in the paper's figure. The library's instability rate confirms the
onset: at ζ₀ = 1 the homogeneous state is stable at f = 0.999 and unstable at 1.001.

Pu et al.'s formula (their Ref. [29]), 1.54(π/2)(Q_C/2Q_L)n²LA_eff/(n₂λQ_L²), is 1.54n²/n_g² =
1.1395 times ours: a factor 1.54 the mean-field equation's minimum doesn't have, and the
material's index n = 3.3 where the round trip takes the group index, n_g = c/(FSR·L) = 3.8363. Its
band, 0.779 to 3.896 mW, also holds 3.1 mW. Where the 1.54 comes from is in their Ref. [29], not in
hand, so the difference between the two bands is listed with the gaps against papers. γ from our
own modes needs the n₂ of Al₀.₁₇Ga₀.₈₃As, which the catalogue doesn't have yet (the paper prints
2.6 × 10⁻¹⁷ m²/W, Table 1).

### Papers whose resonators aren't given whole: not reproduced

- **Herr et al. 2013** (Nat. Photonics 8, 145, Methods): an MgF₂ resonator, FSR 35.2 GHz,
  κ/2π ≈ 450 kHz, Q ≈ 4 × 10⁸, β₂ ≈ −9.39 ps²/km, D₂/2π ≈ 16 kHz, γ ≈ 4.1 × 10⁻⁴ /(W·m),
  finesse "≈ 78 × 10⁴", and a single soliton of 200 fs. The coupled pump is given as a typical
  5–30 mW, and the soliton's detuning not at all, so its width can't be computed. Their Eq. (2),
  Δt_min ≈ 2(−β₂/(γFP_in))^½, is the approximation's shortest width: at critical coupling ours,
  `Ring::minimum_soliton_width`, is 2 acosh(√2)(2/π)^½·(|β₂|/(γFP_in))^½ = 1.989(…)^½. Their
  FSR over κ/2π is 7.8 × 10⁴, not the printed 78 × 10⁴: with 7.8 × 10⁴ and 30 mW the formula gives
  198 fs, with the printed finesse and 5 mW, 153 fs.
- **Pfeiffer et al. 2016** (Optica 3, 20, p. 24): a soliton width of 51.3 fs from P = 3.28 W,
  F = 638, γ = 1.41 /(W·m) and β₂ = 1.95 fs²/m by the same formula. With β₂ as printed the
  formula gives 0.0514 fs; 51.3 fs needs 1.94 × 10⁻²⁴ s²/m (1.94 fs²/µm), and this resonator's D₂
  isn't printed, so the number can't be checked.
- **Luke et al. 2013** (Opt. Express 21, 22829) is about thick nitride films and their Q, with no
  comb or soliton to compare.
- **Coen et al.'s octave comb** (their Fig. 3) gives its dispersion as a plotted curve
  (Fig. 3(b)), not as numbers.

## As a job

A task of kind `"nonlinear"` sets all of this up from a job file
([`jobs/ppln-shg.yaml`](../../jobs/ppln-shg.yaml)): the process (`"shg"`, `"sfg"`, `"dfg"`,
`"opa"`, `"spdc"` or `"fwm"`), the waveguide its waves travel in, the grating that poles it, and the
pump and the signal. The waveguide is one of three:

- `crystal`: plane waves in a catalogue crystal, each wave's index one axis of an index model
  (`Index::model`), rebuilt at the task's temperature;
- `n_eff`: a guide's effective indices given as data, one table for every wave or one per wave
  (`Index::effective`);
- `ridge`: a film of a `[task.stack]`-like table etched to a ridge, each wave's index its
  fundamental mode of a family over the wavelengths the run needs (`ModeIndex::of_ridge`: the
  operating point, the map's ends and the joint spectrum's corners, each wave's range at least
  1 % wide), and a second-order process's η from the modes' overlap through the film's catalogue d
  tensor turned onto the chip ([the overlap](#the-overlap-and-the-efficiency)). The tensor is the
  catalogue's as measured (its label says at which wavelength), not carried by Miller's rule.

The waves are taken in each process's order: SHG's fundamental (the pump) and harmonic; SFG's
first input, second input and sum; DFG's, OPA's and SPDC's pump, signal and idler (the signal
twice the pump's wavelength by default for SPDC); FWM's pump, signal and idler. A grating without
a period is designed to cancel Δk₀ at the operating point, its period at the expansion's
reference temperature (`Grating::for_mismatch` of Δk₀(1 + ε(T))); with a period, its wave vector
points along Δk₀.

The run records four events after its start, and writes their contents beside its record as
`nonlinear.json`, `phase_map.csv`, `efficiency.csv` and `pair_spectrum.csv`:

| Event | What it holds |
|---|---|
| `nonlinear_setup` | the waves with their wavelengths, indices (a ridge's group indices too) and input powers; Δk₀ and the coherence length; the grating; Δk and sinc²(ΔkL/2) at the operating point; η and where it comes from; the acceptance bandwidths in the tuned wave's wavelength and in temperature (Fejer et al.'s estimate and the width between the half maxima); the phase-matched wavelength inside the map; notes, among them each wave outside the material's [transparency window](catalogue.md#transparency-windows) |
| `phase_map` | Δk and sinc²(ΔkL/2) against the tuned wave's wavelength (SHG's fundamental, every other process's signal, the pump held: `Device::phase_tuned`), row by row against the temperature (a crystal) or the ridge's width, each width's modes solved |
| `nonlinear_efficiency` | against the length: the generated power by the coupled-amplitude equations (RK4, 2000 steps over the length, the losses in) beside the undepleted closed form; OPA's signal gain; SPDC's spectral brightness at the signal's centre, η P_p L² per hertz at degeneracy (`pair_rate_density`); FWM's signal gain and idler conversion, in closed form with the pump undepleted |
| `pair_spectrum` | SPDC's and FWM's joint spectral amplitude, its purity and Schmidt number (`pairs::joint_spectrum`) |

The job's numbers are the library's own calls: for Fejer et al.'s first row (SHG of 1.064 µm over
1 mm of congruent LiNbO₃, d₃₃, poled for it at 25 °C) the run's coherence length, period and
bandwidths are those of `Device::quasi_phase_matched` to the bit, which a unit test checks:
3.40 µm, 6.802 µm, 2.04 nm and 25.8 K, Table I's numbers to half their last digit. From Python:

```python
import photonoxide as phox

p = phox.nonlinear("jobs/ppln-shg.yaml")
print(f"l_c = {p.setup['coherence_length_um']:.2f} µm")   # 3.40
print(f"Λ = {p.setup['grating']['period_um']:.3f} µm")     # 6.802
print(f"δT = {p.setup['temperature_acceptance_k']['estimate']:.1f} K")  # 25.8
print(p.response.shape)                                     # (21, 201): temperatures by wavelengths
```

The studio's Materials page has the same job as a form under **Nonlinear processes**, in its
list: the process, the waveguide, the poling, the pump and the map; the setup, the phase-matching
map, the efficiency along the length and the joint spectrum drawn from the job's events; and every
material's transparency window as a band over wavelength, the process's wavelengths marked on
top. A run of the job opens on the same charts in the run view.

What a job can't set yet: a ridge's materials at the task's temperature (they are taken at their
stack's conditions, so its map can't be against temperature), a type-II SHG (one index for both
fundamental photons), a depleted FWM with its losses, and a cavity around the guide.

## Limits

- Uniaxial crystals; a biaxial one (KTP, LBO) only along its principal axes, with an `Index` per
  polarization: no biaxial index surface yet.
- The tilt geometry has the grating along the reference axis (Fejer's κ = 0) and the inputs
  collinear.
- Focused bulk beams (Boyd and Kleinman) are not in yet.
- Cross-phase modulation is between two pulses, Kerr only: no Raman, self-steepening or two-photon
  absorption between them. Free carriers start at none at each window's start: a pulse train's
  left-over carriers aren't held.
- Resonant enhancement is for all-pass rings, each wave's own, and undepleted; an add-drop ring's
  second coupler, a ring resonant at its pump for an oscillator, and a ring's photon pairs (0.6.2)
  are not in yet.
- In a guide, the crystal must have a principal axis along it (the full-vector solver's
  condition), and a mode is followed by its family and index, not through a crossing with another
  mode of its family. Wang et al.'s 4500 %/(W·cm²) (Optica 5, 1438) needs their ridge's etch depth
  and sidewall, which the paper doesn't print: it is not reproduced, and stands here as a value of
  η only.

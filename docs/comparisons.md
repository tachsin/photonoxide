# Comparisons with published results

photonoxide checks its results in two ways, and they are not the same kind of check.

**Verification** asks whether the code solves its equations correctly. It needs nothing from
outside: an analytic solution, a convergence study, an adjoint gradient against finite
differences, an identity the result must obey (energy, reciprocity, a mirror image), or another
solver on the same problem ([cross-code.md](cross-code.md)). It always ends: the check passes, or
there is a bug to fix. Verification is what a solver or a device needs before it is done.

**A comparison with a paper** asks whether our number is the paper's. It can only end when the
paper states the whole problem: every dimension, material, polarization and grid that the number
depends on. Many do not, many numbers are read from a figure, and some are measurements. Such a
comparison is still worth printing, but it is not a test of the code, and a difference in it is
not a defect.

## How a comparison ends

Every comparison ends in one of three ways, each final:

| Outcome | When | What the repository holds |
|---|---|---|
| **Reproduced** | the paper states the whole problem, and our number agrees within a tolerance that has a basis ([CONTRIBUTING.md](../CONTRIBUTING.md#validation-cases-and-their-tolerances)) | a validation case or an example's check |
| **Consistent** | the paper leaves something out, so we chose it; our number is near theirs or on the same side of what they claim | the example prints both numbers and names each choice we made; no check on the comparison; a row below when the difference is worth knowing |
| **Not comparable** | the paper lacks what a comparison needs, or its number can't be read to the accuracy a comparison needs | a row below, saying what is missing and what would settle it |

- **A choice of ours is never checked against the paper it was made for.** Where we picked a
  width, a cladding or a filter radius the paper doesn't give, the example says so and prints its
  number beside the paper's without a pass or a fail.
- **A day for a paper.** A comparison that isn't reproduced after about a day's work gets its
  outcome and its row, and the work moves on. It is reopened when a source arrives, not before.
- **Rows, not issues.** An issue is work someone can do now: a wrong result of ours, a missing
  feature, a decision. "The paper doesn't say" is a fact about the paper; it goes in this page.
  A row here that turns out to be a bug of ours becomes an issue then.
- **Nothing is loosened.** A tolerance is never widened to turn a difference into an agreement,
  and no value is chosen to make a case pass. A row closes only when a source settles it.

## The reference problems

A small set of problems that are stated completely and have an exact or tabulated answer does
more for trust than many partial comparisons. This is a first list, drawn from what the
repository already reproduces; each has a closed form, a printed table, a benchmark several codes
solved, or published design data.

| Area | Problem | Source of the answer |
|---|---|---|
| Materials | `silicon_index`, `silica_index` | Li 1980's Table 1; Malitson 1965's Table I |
| Slab modes | `slab_soi`, `slab_yariv_yeh` | the slab's dispersion relation, as Chrostowski & Hochberg (Section 3.2.2) and Yariv & Yeh (Sections 3.1 and 3.2) print its solutions |
| Multilayers | `multilayer_chilwell`, `leaky_waves` | Chilwell & Hodgkinson 1984's Tables 2 and 3 |
| Mode solver | `rib_hadley`, `hadley_corners`, `leaky_wire_benchmark` | Hadley 2002's Table I and series expansions; Bienstman et al. 2006's Table 6, a benchmark of several codes |
| Bends | `bend_loss` | Marcuse 1971's closed form |
| FDTD | `cpml_roden_gedney`, `tfsf_square_cylinder`, `lorentz_okoniewski`, `pml_oskooi`, `bump_oskooi` | the methods' own papers, each a stated numerical experiment |
| Circuits | `circuit_splitter`, `circuit_ring_critical`, `optimize_ring_flat`, `optimize_mzi_lattice` | closed forms and printed tables (Clements 2016; Bogaerts 2011; Little 1997; Jinguji & Oguma 2000) |
| 3D devices | the six `*_liu_poon` examples | Liu & Poon 2025: an open PDK's devices solved by two commercial codes |
| Inverse design | `optimize_mode_converter`'s testbed designs | Chen et al. 2024: the designs are published as data |
| Nonlinear | `qpm_fejer` | Fejer et al. 1992's Table I |
| Static and RF | the cases of [cross-code.md](cross-code.md) | Elmer and openEMS on the same problems |

## What a source would settle

Each row is a place where our number and a paper's differ and the paper doesn't say enough to
tell why, or where no source gives a number we need. None is hidden: no tolerance was loosened
and no value was chosen to make a case pass. A row says what would settle it. The rows came from
issue #595, where they were open lines.

### Materials and data

- **Sb₂Se₃'s amorphous density** (the crystal's is used, #585).
- **Undoped congruent lithium tantalate's index, what is left** (#629): n_e with temperature terms is in the catalogue from Barboza & Cudney 2009 (0.3–5 µm, 23–230 °C) and Bruner 2002 (0.39–1.85 µm, 25–200 °C); n_o and a direct infrared dn/dT measurement are still missing. In their overlap the two models differ by 0.6e-3 to 1.2e-2 in n_e and their dn_e/dT differ 7 to 100 times (8.3e-5 against 1.1e-6 per K at 1.55 µm and 25 °C): a disagreement between the sources, unexplained. **What would settle it:** a direct measurement of n_e and n_o against temperature at 1.3 to 1.6 µm.
- **TiN's thermal conductivity for a sputtered heater film** (#583 takes Goel's 26.96 W/(m·K) for an 86 nm ALD film, with Moraes and Jagannadham as the spread of 0.5 to 4.7).
- **Barium titanate's n_e(λ), and a film on silicon** (#586): Suceava's n_e is the ordinary index plus an extended-Cauchy difference whose form the paper leaves to the analysis software's manual, so it isn't reproduced, and their film is strained on GdScO₃, not on silicon. Needs that form, or a paper giving n_e and a film on silicon's dispersion.
- **Amorphous Sb₂S₃'s heat capacity** (#585): the amorphous state's heat capacity is Kaito's density times the crystal's c_p; no amorphous c_p is in the catalogue.
- **Goel's plasma-enhanced ALD film's zT** (#583): their Eq. 3 with 512 µW m⁻¹ K⁻² and 26.96 W/(m·K) gives 0.0057 at 300 K against the printed 0.0056, which holds only below 297.5 K. The paper doesn't state its room temperature, so it is in the docs, not a case.
- The old 1.5600 µm in the Zhao write-up is explained as the 10 nm value plus the grid step added the wrong way round; that is inferred, not seen.
- **Voltage or current drives for the doped and in-guide heaters** (#655, narrowed by #764; the TiN film choice and the heater that follows the temperature coefficient of resistance are done): the device has no N++ sheet resistance. Harris's series sheet resistances give about 170 Ω against 769 Ω measured, a factor of 4.5, because the current's path is 3D.

### Devices and modulators

- **Delaney's Δn_eff (0.0717/0.0356)**: Fig. 7 read by pixel fixes the geometry (500 nm rib etched 120 nm, 25 nm film on the top and slab, TE), and the overlaps agree within 0.007 there. On it ours is 0.0613/0.0371 (−14.5 %, +4.2 %), so the remaining question is which film indices their mode solver used (their ratio 2.01 against our 1.65 with Table 1's indices); no source gives them (#776). The paper's Sec. 2.5 says 22 nm films, its figure and Δn_eff text use 25 nm.
- **Barium titanate** (#586): with the paper's k our reflectances are 0.1294 and 0.1345 where the paper computes 0.1237 and 0.1292; k is left out because the paper's two models give 0.061 and 0.120 at 1550 nm.
- **Two sources disagree on amorphous Sb₂S₃'s density** (#585): Kaito 1984 gives 3420 ± 60 kg/m³, 0.74 of the crystal's; Kepič 2024's film thickness change implies 0.88.
- **Small differences, found while checking papers:** Lee 2001's loss tangent rises 12 to 13 % across 90 to 147 GHz, our reading of its Fig. 2 rises 9 to 10 % (#578); Tsai 1993's Eq. 3 puts the 500 °C unbiased TiN film 4.8 µΩ·cm below its Table II (#581); Kharel's measured loss of 0.69 is 15 % below our 0.796, so it is reported and not a case (#579); Payne & Lacey's Eq. 9 carries an extra factor of 2 according to Barwicz & Haus (p. 2729).
- **The bandwidth against the crystal cut**: needs a z-cut modulator model, and no z-cut modulator paper is in the folder.
- **Gürses' Fig. 15** (#416): the drive is not given in numbers, so the figure cannot be reproduced as a case.
- **The phase-change shifter's measured fidelity** (#577, #543): Delaney et al.'s cells as `Measurement` files wait for numbers worth keeping as measured (their Δn_eff is simulated, their loss extrinsic).
- **The crystallized fraction from a heat pulse** (#577, #543): the fraction is an input. Setting it from the switching job's peak temperature against T_c and T_m needs crystallization kinetics, which the device doesn't have.
- **The conformal film's corners, and the TM-like mode against a paper** (#610): the film's corners are square, (1 − π/4)t² per corner more than a truly conformal film's; no paper we have gives a TM-like number with a complete geometry.
- **Valdez 2023's hybrid mode** (#576): their n_eff 1.90, the power in the film and the strip (0.69 and 0.15) and n_g of about 2.24 (ours 1.908, 0.64, 0.20 and 2.233) aren't cases, because the mode hangs on the nitride's index, which the paper's main text doesn't give (we take the catalogue's LPCVD, 2.03). **What would settle it:** the nitride's index.
- **`bench` as a Cargo feature** (#509): bench stays in the library (the owner's one-crate decision); if its compile cost matters, gate it behind a Cargo feature. To be decided separately; needs a measurement of what it costs a user who does not use it. **Decided 2026-11-13:** measure first, then decide.
- No literature source for the modal Fresnel formula's accuracy was looked for (`step-reflection-te`'s basis is the expansion measurement).

### Nonlinear optics

- **Fejer 1992's Table I, the values that do not match within half the last printed digit** (ours from Edwards & Lawrence 1984; 41 of 49 QPM values do): l_c for d31 at 1.064 µm, 129 printed against 129.6 ours (Fejer's own 959 mrad fits 129.6); l_c for d33 at 2.12 µm, 15.6 against 15.79 (Fejer's 337 mrad fits ours); δλ for d31 at 2.12 µm, 40 against 40.8; δT for d22 at 0.86 µm (lithium-rich), 17.4 against 17.35; δν in column d for d31 at 1.064 µm, 103 against 102.5 (A1.9); δν in column d for d31 at 2.12 µm, 139 against 176.8 (A1.9 with |Δk′| gives 139, and (183) is reproduced). **What would settle it:** Fejer's own coefficients or an erratum for those rows.
- **Wang 2018's 4500 %/W/cm² from overlaps** (part 2b): blocked because the paper does not print the etch depth (the supplement, a group thesis or a later paper that states it would settle it). The nonlinear optics lesson (#732) shows Wang's 2600 and 4500 %/(W·cm²) as not matched.
- **Grice & Walmsley 1997's Eq. (8)** writes the phase-matching function as sin(ΔkL)/ΔkL, but integrating their Eq. (6) gives sin(ΔkL/2)/(ΔkL/2), so their continuous-wave bandwidth (2.78) is half the integral's (5.57) (#621, part 3a). A finding about the paper, to check against the published erratum if there is one.
- **Bruner 2002's congruent n_e** sits 0.8 to 3.5e-3 below Bond's table at 25 °C (rms 2.1e-3), against their stated standard deviation of 0.001327.
- **Vurgaftman 2001's AlGaAs Γ–X crossover:** the tables put it at x = 0.435 at 300 K (0.417 at 0 K); the text says 0.39 (0.38).
- **Transparency windows without a source:** every multiphonon edge (Si, SiN, AlN, GaAs, and the rest) and every edge of SiO₂, SiN, MgO:LiNbO₃ and BaTiO₃ read "not specified" in the catalogue.
- **Miller's rule at 1.313 µm** lands 15 to 34 % above the measured Shoji d values (0.4 to 9.4 % at 0.852 µm); the basis is Miller's own ±50 %. GaAs is not covered: the second harmonic of 1.064 µm lies above its band gap.
- **Si's n₂ and TPA at 1550 nm are read from Bristow's figures** (β 0.79 ± 0.39 cm/GW, n₂ (4.8 ± 2.0)e-14 cm²/W), the paper prints no number there; **SiN's n₂** is a PECVD film's and has no uncertainty; **AlN's n₂** is derived from a comb threshold, not measured.
- **Chang 2018's modal phase matching is not reproduced:** TE00 at 2 µm and TM00 at 1 µm meet at a width of 2.655 µm, not their 1.5 µm; at 1.5 µm the match is near 2.066 µm, 66 nm above theirs; the other GaAs index model barely changes it. Only the main text was used: the sidewalls are taken vertical and the 5 nm SiN layer, whose index is not given, is left out. The sensitivities agree (10.4 against 11 nm per nm of thickness; −0.127 against 0.1 nm per nm of width). The paper has no supporting information (the published PDF does not mention one, nor does the arXiv copy 1805.09379v2, nor the figshare records), so the geometry behind TE00 at 2 µm and TM00 is not published anywhere we could find. **What would settle it:** the geometry from the authors (Chang et al. 2018, Laser Photonics Rev.); ask them. The nonlinear optics lesson (#732) shows Chang's GaAs width as not matched.
- **Zhao 2020's brightness:** ours is 1.04e7 pairs/(s·GHz·mW) with their d33 = 27 pm/V and an ideal grating, and 7.46e6 with their 55.2 % duty cycle and 245 nm poling depth; they print 4.6e5, that is 22.6 and 16.2 times lower, and give no calculated efficiency to split the gap. Unexplained; the loss of the real device and the collection efficiency are candidates. The nonlinear optics lesson (#732) shows it too: Zhao's brightness is 16 to 23 times below ours.
- **Zhao 2020's purity and Schmidt number:** the paper prints neither, so nothing to compare.
- **Coen 2012's Fig. 2 soliton: 444.4 fs against "∼400 fs".** It passes as a case only because "∼400" is read as one significant digit (the basis of half the last printed digit, ±50 fs, a 12 % window); to look at again if a tighter reference appears. The nonlinear optics lesson (#732) reads Coen's peak power and his 400 fs to two digits, not one; both need the same tighter reference.
- **Agrawal's Eq. 2.3.45** prints T_R = f_R τ1 τ2²/(τ1² + τ2²) = 1.92 fs, but integrating his Eq. 2.3.39 gives f_R · 2 τ1² τ2/(τ1² + τ2²) = 1.46 fs. Nothing in the code depends on it.
- **Coen's "intracavity" 100 mW peak:** ours inside the ring is 6.36 kW; θ times that, what leaves through the coupler, is 0.111 W.
- **Herr 2013:** the finesse is printed as ≈ 78 × 10⁴, but FSR/κ gives 7.8 × 10⁴; the coupled power for the 200 fs soliton is not given (only "5–30 mW"). The nonlinear optics lesson (#732) does not reproduce Herr's soliton.
- **Pfeiffer 2016's soliton width:** their β2 = 1.95 fs²/m gives 0.0514 fs by their formula, not 51.3 fs; 51.3 fs needs 1.94e-24 s²/m.
- **Pfeiffer 2016's D2**: their text's 0.49 MHz disagrees with their own Fig. 5(c): the drawn polynomial, read from vector paths, is 1.90 MHz. Ours (1.43 MHz, straight TM0, catalogue indices) is 25 % below the figure; their bend and film indices aren't given (#776).
- **Not fully specified, so no case:** Foster 2006 (cladding, n2, TPA and free-carrier absorption), Luke 2013 (has no comb data), Coen's Fig. 3 (dispersion only as a curve).
- Pu 2016's own threshold formula (1.54 n²/n_g²) is 1.14 times ours: their band is 0.779 to 3.896 mW, ours 0.684 to 3.419 mW, both holding the measured 3.1 mW. Where the 1.54 comes from is in their Ref. [29], not in hand. The nonlinear optics lesson (#732) prints the 1.14.
- Pu 2016's γ from our own modes needs Al0.17Ga0.83As's n₂ in the catalogue; the paper prints 2.6 × 10⁻¹⁷ m²/W.
- MgO:LiNbO3's d₂₂ is missing from Shoji 1997 at every wavelength (Table 10 is blank; Kleinman's symmetry can't supply d_yyy), so a MgO:LN ridge's η needs `d_pm_per_v` and the job stops with a note asking for it; a source that measures d₂₂ in the doped crystal is still missing. (#652)

### Quantum

- **Peruzzo et al. 2010's Fig. 4B, and the last 1–2 % of their panels** (#738, #770): their simulated panels show only the 121 measured pairs (q − r even; the fibre arrays have twice the output pitch, p. 1501). On those pairs, at C = 5 per mm over 782 µm (Cz = 3.91), Figs. 3B, 3D and 4D agree bar by bar within 0.00025, 0.00042 and 0.00065 (75, 61 and 90 bars read from the vector paths; tallest 0.0176, 0.0239 and 0.0264 against 0.0178, 0.0237 and 0.0266). Fig. 4B is up to 0.0041 off and fits best at Cz = 4.035 (0.00077), an array 3 % longer than 4D's, which has the same input. A scale factor, normalizing over the measured pairs and a second-neighbour coupling (±0.06 C) leave the 1–2 % as it is. The arXiv version has the same panels and no supplement. **What would settle it:** the SOM's fig. S1 and methods, or the authors' simulation of Fig. 4B.
- **Figures show mostly on the Papers page** (#736): Home's top ten are usually today's journal papers, which keep initials because no source gave pictures within the publishers' terms. **What would settle it:** a source of figures for new journal papers within the publishers' terms.
- **Paesani et al. 2020 (dual-pump TM0/TM1 source), what is left** (#742): the supplement was read. Our TM1 − TM0 group-index difference, 0.1070, 0.1095 and 0.1111 on 20, 10 and 5 nm cells, agrees with their Supp. Fig. 4's 0.112 (n_g 4.007 for TM0, 4.119 for TM1). Their 1.46 ps delay does not match their own Supp. Eq. 2 (2.06 ps); with the corrected delay our purity at our own Eq. 2 delay extrapolates to about 0.99934 against their 99.95 % (a gap of 0.00016, three times half a unit of their last digit). Pairs land at 1584.2 / 1517.3 nm against their 1588 / 1516 nm (their Supp. Fig. 4 puts phase matching at about 1514.5 and 1587.5 nm). With their measured FROG pump (peak 1549.4 nm, FWHM 4.6 nm, non-Gaussian) through `quantum::shaped_dual_pump_spectrum` the purity is 0.9911 without and 0.9817 with the pump phase, against their 0.993 and 0.984. The phase's cost agrees (0.0094 against 0.009); both purities are about 0.002 low, and the raster edges of the pixel-read pump matter (zeroing below 3 % of the peak gives 0.9845). **What would settle it:** the pump's spectrum as data rather than read from pixels. Tracked here since #737 closed (2026-11-09). Their Supp. Fig. 2a inset reads 0.9934 and 0.9838 at 11 mm (one pixel is 0.00025), so the gap is not in reading their figure. Reading the pump per unit frequency (λ²), without its 0.005 line floor, or interpolating its amplitude moves ours by at most 0.0007. The authors' figshare data (10.6084/m9.figshare.11882760) has the main figures' data but not the FROG pump. **What would settle it:** the FROG pump as numbers, from the authors.
- **Vaidya et al. 2020's quadrature squeezing** (#742): about 1.0 dB measured and about 4 dB inferred need the collection efficiency η and the gain g. The supplement was read: it prints neither, only V± = 1 + 4gη(2g ± √(1+4g²)). The case stays blocked on the authors' data. **What would settle it:** the authors' η and g. Tracked here since #737 closed (2026-11-09).
- **Bogaerts 2007's crossing cannot be reproduced** (#741): the letter is inconsistent with itself ("-0.16 dB, which amounts to 97.5 %": 97.5 % is -0.11 dB, and -0.16 dB is 96.4 %), and its abstract's "-1.7 dB" is a misprint. Neither it nor its double-etch reference (Bogaerts 2005) gives the shallow-etch depth or the plane's indices; a chart needs that depth and a parabolic outline. **What would settle it:** the shallow-etch depth and the indices from the authors or another paper.
- **Almeida 2003's tip: 20 nm wider at both polarizations** (#741, #770): TE₀ is best at 140 nm (0.946, 0.24 dB) against their 120 nm (94 %), and TM₀ at 75 nm (0.941) against their 50 nm (0.4 dB). The window isn't the cause (12 and 20 µm agree; 5 µm contradicts their narrow side), nor the fibre's mode, nor their measured 270 nm film (5–10 nm). On the wide side their 1.9 dB at 150 nm is ours at 175 nm. **What would settle it:** their simulation's tip cross-section (sidewalls, grid).
- **Silverstone et al. 2013, non-degenerate fringes (Fig. 2c)** (#743): ours is 99.944 % for both; A_s × B_i agrees with their 98.8 ± 1.2 %; A_i × B_s does not: theirs 99.1 ± 0.5 %, ours 0.84 % higher, outside their uncertainty. The supplement was read: it has no MMI design (its Section S4 assumes ξ = η = ½) and no pump centres, and its Fig. 5 is schematic (pump about 1547.9 nm). Its Table 1 gives A_s × B_i 95.6 ± 1.2 % raw and 98.8 ± 1.2 % net, B_s × A_i 96.1 ± 0.5 % raw and 99.1 ± 0.5 % net; its Table 2 swaps the 9.6 and 3.2 nm HOM visibilities relative to the main text. The spurious-pair model (S5-S6) does not lower the split-type fringes, so the gap stays unexplained. Our case uses a guided-mode MMI designed to split evenly at 1549.6 nm, whose fringe at the model's own image length would be 95.8 %, and the 22.4 nm pump spacing (centres assumed 1549.6 ∓ 11.2 nm). **What would settle it:** the authors' MMI design. Tracked here since #737 closed (2026-11-09).
- **Reddy et al. 2020's stack: the planar model's resonance sits 145 nm red of theirs** (#743): the supplement (Section 2) was read: SiO2 1.453, αSi 2.735, MoSi 4.1 nm with (5.817, 6.033), the 2 nm Si cap n = 2.274; the nominal AR coating is αSi 50 / SiO2 248.7 / αSi 68 nm with reflection minima at 1530-1540 nm, the top αSi tuned in situ to 1550 nm (final thickness not given); single-pass absorption is 0.057-0.1, and 45-80 passes give 99 %; no simulated absorption is given. Our planar films with the meander as an effective medium absorb 0.715 along the wires, 0.065 across and 0.763 as a whole film at 1550 nm; the reflection minimum is at 1678 nm with absorption 0.998 (1679.75 nm and 0.999 with Fig. S3's dispersion), about 145 nm red of theirs. Each reading of the stack checked alone (coating reversed 1706 nm, no cap 1672, light from glass 1712, across a 100 nm air gap 1664, the index across the wires 1709, a whole MoSi film 1687, n and κ swapped 1700) puts the minimum nowhere near 1530-1540 nm. Without the coating the minimum is at 1550 nm, so the shift comes from the coating; reaching 1535 nm needs the top αSi at 22-24 nm instead of 68, or the bottom αSi near 20 instead of 50, or the SiO2 near 175 instead of 248.7. A 1D RCWA is in (`mode::rcwa`, #757) and the patterned stack was run: absorption 0.716 along the wires (planar 0.715) and 0.480 across (planar 0.065), 0.598 unpolarized; reflection minima TE 1677.88 nm (1679.16 nm with Fig. S3's dispersion; planar 1679.75), TM 1729.91 nm, unpolarized 1707.12 nm. The gap fill (SiO2, vacuum, 2.274) moves them 4 to 11 nm. Per extra nm of thickness: top αSi +2.19 nm (TE), coating SiO2 +1.71, first αSi +4.50. A top αSi of 23.25 nm puts the TE minimum at 1550 nm but the reflection there is 0.155 (18.14 nm gives 1535 nm and 0.197): no reasonable top αSi reproduces theirs. **What would settle it:** the as-built layer thicknesses, or the reflection spectrum from their in-situ tuning.
- **Quantum Q3's distributions over process variation** (#743): HOM visibility, fidelity and the loss budget as distributions over process variation wait for 0.11: they need its Monte Carlo and yield. Tracked here since #737 closed (2026-11-09).

### Fabrication

- **A TFLN sidewall angle in Zhang 2017 (Optica)**: not given; Krasnokutska 2018 (about 75°) and Kaufmann 2023 (60–63°) give angles for their own etches.
- **A GaAs etch process** (Dietrich 2016 gives none) and **a III–V etch chemistry** (Donnelly & Kornblit 2013 gives none), for the GaAs platform.
- **Proximity effects of electron-beam lithography** (Chen 2015 has no numbers on them).
- **Luke 2013 does not name its nitride deposition method.**
- **No source in hand covers:** TiN dry etch, the heater's contact metal, lift-off's resist-to-metal ratio, the HBr selectivity as a number, or copper's and gold's thermal budget. The flows say "not specified" there. (Campbell 7.5 is cited by section, not page.)
- **Franssila 2010's Table 10.1, row 4** (248 nm, NA 0.65, k1 0.5): k1·λ/NA gives 0.191 µm where the table prints 0.18 µm. Unexplained, so it is left out of the case.
- **Campbell's Example 4.1** prints τ = 0.295 h where the formula gives 0.2956 h (the book cuts the digit rather than rounding), so τ is not a case; the thickness case passes (0.4770 against 0.48 µm).
- **Not specified by any source in hand:** the overlay of contact, i-line and EUV lithography; the linewidth control of every lithography except 193 nm and every etch's change of linewidth; the thickness spread of LNOI and of all deposited films; the walls of the newer etches; the depth variation of timed etches other than argon milling of LiNbO3 (Franssila's 5 % example stands in, labelled as such).
- GaAs: no source states the waveguide etch (depth, chemistry, walls); the flow says "not specified" and a timed chlorine etch stands in.
- No source gives the sputtering temperature of the phase-change film or the oxide cap.
- The nitride and AlN etches' selectivity over oxide and their wall angles are not stated by any source in hand.
- ma-N's resolution: only the features the papers made.
- Solving an AlN guide on silicon: the catalogue's AlN model (Rigler 2015) covers only 0.4 to 0.9 µm and its silicon (Li 1980) only 1.2 to 14 µm, so no wavelength has both; it needs an AlN index model in the near infrared or a silicon one in the visible (#794).
- Delaney 2021 doesn't give its deep-UV scanner's wavelength.
- The photonic Damascene process's reflow step is in no source in hand.
- A source for each resist's thickness: where none is stated, it is drawn 0.3 µm and the page says so.
- Drawing defaults no source states, each said on the page: fully conformal coverage (100 %) where no sidewall fraction is stated, the imprint residue at 0.1 of the resist thickness, the planarizing spin-on model.

### Optimization and inverse design

- **The genetic algorithm of Pérez-López 2020**: the paper reports a 95 % success rate; ours is 4 % or 99 % depending on whether the mutation decay counts generations or thousands of operations, which the paper does not say. **What would settle it:** the authors' settings or code.
- **Suppl. Table 18's PSO success rate**: the paper gives 70 %, ours 57 % (2.0σ off of two binomial counts of 100), unexplained.
- **Fig. 3's 76.66 %** is not a whole number of 100 runs, so the number of runs behind it is not 100.
- **The mesh's gradient descent** (Suppl. p. 23, #617): the paper's learning rate of 50 never converges with our cost function; a rate of 1 converges on 19 of 20 meshes in a median of 18 iterations, where the paper reports 5 to 6. The paper's momentum ramp is not something genoxide can do. **What would settle it:** the paper's cost function and normalisation of the gradient.
- **Wang 2022's ring-loaded MZI filter** (#617): the paper's Eq. 10 is not the 40 dB filter its text specifies, and its Eq. 11 does not multiply back to Eq. 10; Fig. 4's circuit gives 19.3 dB where the paper says "around 34 dB"; Nelder–Mead reaches 39.6 dB with 6.9 dB of ripple towards the 40 dB filter and 59.4 dB with 1.64 dB towards the 60 dB / 1 dB one, neither matching the paper's figures. **What would settle it:** the authors' coefficients (an erratum or the design code).
- **The testbed's eight published designs at equal grids** (#761, I4): at the testbed's 10 nm our 2D FDFD rounds every transmission to the README's printed digits (−0.187 to −1.344 dB against −0.19 to −1.34) and the reflections differ by 0.02 to 0.62 dB, but our own grid moves T by up to 0.009 dB from 10 to 5 nm (−0.004, −0.005, +0.009 dB for the first three designs; R by +0.04, +0.17, +0.54 dB), above a third of the printed digits' 0.005 dB, so by decision 7 they are not a validation case and nothing is loosened. **What would settle it:** the testbed's transmissions to more digits or at a finer grid (`the_testbed_spectrum_study`, `the_testbed_grid_study`).
- **Passoni 2017's bandwidths on its own designs: explained** (#776). Silicon's and silica's dispersion about the printed constant indices narrows every bandwidth 2.5–3.3 %, and the gap goes from +1.6…+4.7 % to −1.8…+1.9 %. Not a case because designs 56, 52, 49 and 37 stay just beyond the paper's 1.32 % spread, and the paper doesn't name its material models.
- Passoni 2017's supplement disagrees with itself: the 4 µm etch is 140 nm in the data file and Supp. Fig. 4a but 150 nm in Fig. 6's table; FIO is 2.48 µm in Supp. Tab. II's chirp and 2.60 µm in the file; the 8 µm mean period is 646 nm in the file and 644 nm in Supp. Tab. I.
- **Vernon et al. 2017's dual-coupling condition is not reproduced** (#761, I10): for K = 1.0009 (their Fig. 2(b)) our point-coupled model of Helt et al.'s ring needs the pump's linewidth 12.8 times the signal's on ±40 linewidths, rising with the window (9.6, 11.7, 12.8 on ±10, 20, 40; about 14 extrapolated), where they have 6.6; at 6.6 ours gives K = 1.0034. The equal-linewidth case agrees (K = 1.0904 against their Fig. 2(a)'s 1.09). Their Fig. 2(b) is from their full device's numerical simulation (Fig. 3's markers), whose model and parameters the paper doesn't give (`optimize_quantum`). **What would settle it:** the full device's model (the coupling scheme of Gentry et al., their ref. 13, with its numbers) or their point-coupling curve's numbers.

### Modes, eigenmode expansion and the lessons

- **Jacques's thermal crosstalk**: the length-averaged solve at 320 µm gives 0.166/0.0403 (+18 %/−7 % against measured, ratio 4.12 against 3.24), and no heater length fits both gaps. What's left needs the victim MZI's layout (its arms' lengths beside the heater, where its far arm runs), which the paper doesn't print (#776).
- **The photonic crystal's complete gap, 18.9 % against the book's 18.6 %** (#724): the triangular lattice of air holes (r = 0.48a, ε = 13) settles at 18.94, 18.89, 18.91 and 18.92 % on 32, 40, 48 and 64 cells, 0.3 points (1.7 %) above the book's p. 75; the 24-cell value, 18.58 %, agrees only through the grid. The book prints neither its resolution nor the gap's edges. **What would settle it:** the book's edges, or the same gap from an independent plane-wave code.
- **Arbabi's metalens focus, 191.8 µm against about 195 µm** (#727): ours is 1.6 % short; the zones' larger angles move the focus further than the paraxial formula, their lens is circular and lit by a fibre's beam, and the paper gives the focus only as "∼195 µm". **What would settle it:** the lens's diameter and illumination as built, or a printed focal-plane scan.
- **Dai, Wang and Shi 2013's TM bus widths** (#733): with their indices (3.455 and 1.445 at 1.5 µm, a 0.4 µm access guide) ours are 1.047, 1.695 and 2.343 µm against their 1.02, 1.68 and 2.35 µm on 10 nm cells. On 5 nm cells the index mismatch at 1.02 µm stays (0.0115 against 0.0136 on 10 nm), so the cell size is not the whole cause. **What would settle it:** their mismatch's definition and the design's cell size, from the paper's text or its authors.
- **Chrostowski and Hochberg's Fig. 5.24, the 180 nm tip against a Gaussian beam** (#733): TE best NA 0.44 and −0.26 dB against about 0.4 and about −1.1 dB; TM 0.68 and −0.10 dB against about 0.55 and about −1.4 dB. Their beam convention is not given, and a smaller window raises our coupling. **What would settle it:** their beam's definition (waist or NA, and the window), or the chapter's data.
- **Umashankar and Taflove's figure on the shadowed face: 0.207 in the figure, 0.170 ours** (#734): ours matches their Eq. 8a moment method (0.171), so the figure's curve and their own equation disagree. **What would settle it:** their figure's data, or a note from the authors on the curve.
- Two papers are still wanted for the quantum lesson (#751): Broome 2013's published version (10.1126/science.1231440; only the Science Express version is in the papers folder) and Reck 1994 (10.1103/PhysRevLett.73.58; open access, not yet in the folder). Politi 2008, Spring 2013, Tillmann 2013 (advance online version), Crespi 2013 (advance online version), Paesani 2019 (first release) and Zhong 2019 are read and ticked. The lesson labels Broome's and Reck's items as known from a citation until those PDFs are in.
- **Six papers the RCWA page cites are not in the folder** (#757): Moharam & Gaylord 1981; Moharam et al. 1995 (JOSA A 12, 1068 and 1077); Lalanne & Morris 1996; Granet & Guizal 1996; Li 1996 (JOSA A 13, 1024). They are cited from Crossref records only and are in the papers README as needed.
- **Wasilewski et al. 2006 (10.1103/PhysRevA.73.063819) and Christ et al. 2013 (10.1088/1367-2630/15/5/053038)** (#759): cited by title only for `quantum::Broadband`; they are in the papers README as needed.

### External libraries

- conda-forge's oneMKL 2026.2.0 build was marked broken there (`mkl=2026.2` does not exist; unpinned it installs 2026.1.0): re-check it when conda-forge publishes a 2026.2 build.
- MUMPS 5.9 has no Windows or macOS package (conda-forge's mumps-seq is still 5.8.2); it is checked on Linux, built from source, only.

### Comparisons that had issues of their own

Each of these was an open issue; its history is there.

- **Zhang 2022's and He 2019's lithium niobate modulators** (#867): 92 and 33 GHz against their 170 and 100, and 2.87 V·cm against 2.2 to 2.5. *Not comparable* without the authors' data. **What would settle it:** Zhang's measured S-parameters and electro-optic response from 10 MHz to 6 GHz, how the 1 GHz Vπ was measured and the substrate's resistivity; He's supplement and its r₃₃.
- **Hughes 2018's Kerr switches** (#866): 98.2 % and 3.1 % on `Kerr2d`. *Not comparable* without the Supporting Information. **What would settle it:** their design region, grid, filter and projection.
- **Published inverse designs read from their figures** (#865): Shen 2015's 41 and 65 % against 81 and 89 %, Piggott 2020's 0.913 against 0.880, Su 2018's 0.036 dB, Chen 2024's 75 against 80 %. *Consistent* where ours is another optimum of the same problem; *not comparable* where the design is only a figure. **What would settle it:** the designs as data.
- **Khan 2024's lithography in the loop** (#864): their Table 1 can't be compared. *Not comparable.* **What would settle it:** Donzella et al. 2014's converter (doi:10.1364/OE.22.021037), whose dimensions and numbers are printed.
- **Arcari 2014's β = 98.43 % in a 3D W1 waveguide** (#863): not attempted; about 4 h a run by the first estimate. **What would settle it:** the supplement (arXiv:1402.2081v3) and a timed run at a/16 with two mirror planes.
- **Pernice 2012's NbN guide** (#859): 1.9 dB/µm against their 1.0. *Not comparable:* the paper leaves part of the geometry out. **What would settle it:** the film's thickness and width that 1.0 dB/µm needs with Anant et al. 2008's NbN, against what the paper gives.
- **`subpixel_holes`, the first band's slope** (#816): −1.81 against Farjadpour et al.'s −1.33; the smoothing's own slope, −2.26 against −2.43, is as they found. *Not comparable:* the paper doesn't state its k or its band. **What would settle it:** a reading of k (Γ, X, M, K) and the band that gives −1.33.
- **Zhang's Y-branch in 3D** (#813): 0.168 dB for Table 1's design and 0.202 dB for the swarm's against the paper's 0.13 dB. *Consistent:* the paper doesn't give its arms. **What would settle it:** a 3D FDTD of Table 1's design, to split the method's error from the arms' difference.
- **Passoni 2017's grating couplers** (#817): the paper's own designs give 53.45 % (4 µm) and 49.26 % (6 µm) against its 53 % and 47 %, so the model is *reproduced*; our own search's fronts were 43.64 % and 39.77 %. **What would settle it:** a search in the paper's design space that reaches its fronts.

---
title: "Nonlinear integrated optics"
summary: "Light that changes the light: the polarization's second- and third-order terms, second-harmonic generation by the coupled-wave equations, phase matching and its bandwidths, quasi-phase matching by periodic poling, the efficiency of a thin-film lithium niobate guide from its modes' overlap, parametric gain and oscillators, the Kerr effect and the nonlinear Schrödinger equation, four-wave mixing, Kerr combs by the Lugiato–Lefever equation, and photon pairs and their purity, against the published devices photonoxide reproduces and the ones it doesn't."
topic: Active and quantum photonics
level: advanced
minutes: 65
prerequisites: [strip-waveguide, ring-resonator]
examples: [qpm_fejer, ppln_wang, pairs_zhao, comb_pu, soliton_coen]
methods: [nonlinear.md, catalogue.md, fields.md]
validation:
  - nonlinear/sinc2-half-maximum
  - nonlinear/sinc2-length-squared
  - nonlinear/fejer-table-1-coherence
  - nonlinear/fejer-table-1-wavelength
  - nonlinear/fejer-table-1-temperature
  - nonlinear/edwards-lawrence-table-2
  - nonlinear/wang-2018-duty-cycle
  - nonlinear/coupled-undepleted
  - nonlinear/coupled-shg-tanh
  - nonlinear/coupled-sfg-sech
  - nonlinear/coupled-opa-cosh
  - nonlinear/coupled-manley-rowe
  - nonlinear/coupled-rk4-order
  - nonlinear/miller-linbo3-d33
  - nonlinear/miller-linbo3-d31
  - material/linbo3-shoji-miller
  - material/linbo3-zelmon-opo
  - material/linbo3-jundt-opo
  - nonlinear/opo-threshold-doubly
  - nonlinear/opo-threshold-singly
  - nonlinear/opo-steady-photons
  - nonlinear/guide-plane-wave
  - nonlinear/guide-slab-overlap
  - nonlinear/guide-zhao-period
  - nonlinear/guide-zhao-wavelength
  - nonlinear/guide-zhao-area-signal
  - nonlinear/guide-zhao-area-pump
  - job/nonlinear-ridge-zhao-period
  - job/nonlinear-ridge-zhao-efficiency
  - nonlinear/kerr-gamma-gaussian
  - material/kerr-bristow-readings
  - nonlinear/nlse-spm-phase
  - nonlinear/nlse-spm-broadening
  - nonlinear/nlse-tpa-power
  - nonlinear/nlse-fca-cw-power
  - nonlinear/nlse-fca-cw-phase
  - nonlinear/nlse-dispersion-silica-beta2
  - nonlinear/nlse-fundamental-soliton
  - nonlinear/nlse-third-order-period
  - nonlinear/nlse-split-step-order
  - nonlinear/fwm-gain-matched
  - nonlinear/fwm-gain-mismatched
  - nonlinear/fwm-nlse-gain
  - nonlinear/ring-shg-enhancement
  - nonlinear/ring-fwm-enhancement
  - job/nonlinear-ring-shg
  - job/nonlinear-ring-fwm
  - nonlinear/lle-homogeneous-lower
  - nonlinear/lle-homogeneous-upper
  - nonlinear/lle-instability-rate
  - nonlinear/lle-split-step-order
  - nonlinear/lle-coen-2012-duration
  - nonlinear/pairs-gaussian-purity
  - nonlinear/fwm-sfwm-purity
  - nonlinear/pairs-heralding-gaussian
  - nonlinear/pairs-filtered-purity
  - fabrication/round-trip-tfln
charts: [nl-miller, nl-coupled, nl-acceptance, nl-qpm, nl-ridge, nl-spm, nl-soliton, nl-fwm, nl-lle-states, nl-comb, nl-jsa]
papers:
  - cite: "P. A. Franken, A. E. Hill, C. W. Peters, G. Weinreich, Phys. Rev. Lett. 7, 118 (1961)"
    title: "Generation of Optical Harmonics"
    doi: 10.1103/PhysRevLett.7.118
    year: 1961
    role: origin
    note: "About 3 J of ruby light at 6943 Å in a millisecond pulse, focused into crystalline quartz, gave about 10¹¹ photons of its second harmonic at 3472 Å per pulse; with the quartz removed or replaced by glass, none: the first optical harmonic."
  - cite: "J. A. Armstrong, N. Bloembergen, J. Ducuing, P. S. Pershan, Phys. Rev. 127, 1918 (1962)"
    title: "Interactions between Light Waves in a Nonlinear Dielectric"
    doi: 10.1103/PhysRev.127.1918
    year: 1962
    role: origin
    note: "The coupled-wave theory of three- and four-wave mixing: the exact depleted second harmonic, tanh² (Sec. V, Eq. 5.12), the Manley–Rowe relations (Eq. 6.6), and the first proposal of quasi-phase matching, the crystal inverted after every coherence length so that the nonlinear susceptibility changes sign (Sec. VIII.A, Fig. 10)."
  - cite: "R. C. Miller, Appl. Phys. Lett. 5, 17 (1964)"
    title: "Optical second harmonic generation in piezoelectric crystals"
    doi: 10.1063/1.1754022
    year: 1964
    role: milestone
    note: "Eleven crystals from KDP to GaAs: their d coefficients spread over a factor of about 600, but d divided by the product of the three waves' linear susceptibilities, δ, stays within a factor of two of its average (Eq. 4): Miller's rule."
  - cite: "G. J. Edwards, M. Lawrence, Opt. Quantum Electron. 16, 373 (1984)"
    title: "A temperature-dependent dispersion equation for congruently grown lithium niobate"
    doi: 10.1007/BF00620081
    year: 1984
    role: milestone
    note: "Congruent lithium niobate's n_o and n_e as functions of wavelength and temperature (Eq. 1, Table I), checked against noncritical difference-frequency mixing of the 488 nm argon line and a dye laser in a 50 mm crystal (Table II), the measured temperatures about 3 °C from the equation's."
  - cite: "L. A. Lugiato, R. Lefever, Phys. Rev. Lett. 58, 2209 (1987)"
    title: "Spatial Dissipative Structures in Passive Optical Systems"
    doi: 10.1103/PhysRevLett.58.2209
    year: 1987
    role: origin
    note: "A driven, damped cavity filled with a Kerr medium in the mean-field limit (Eq. 1), its uniform states S shaped above a detuning of √3 (Eq. 2), and the instability that turns them into a stationary pattern (Eqs. 4–6): transverse, by diffraction; the same equation with dispersion in place of diffraction describes a microresonator's comb."
  - cite: "M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, IEEE J. Quantum Electron. 28, 2631 (1992)"
    title: "Quasi-phase-matched second harmonic generation: tuning and tolerances"
    doi: 10.1109/3.161322
    year: 1992
    role: milestone
    note: "Quasi-phase matching worked out as a design tool: the grating's Fourier coefficient (Eq. 22), the acceptance bandwidths in wavelength, temperature and angle (Eqs. 25, 30, 41, 44), the effect of period and duty-cycle errors, and Table I's coherence lengths and widths for lithium niobate."
  - cite: "R. DeSalvo, A. A. Said, D. J. Hagan, E. W. Van Stryland, M. Sheik-Bahae, IEEE J. Quantum Electron. 32, 1324 (1996)"
    title: "Infrared to ultraviolet measurements of two-photon absorption and n₂ in wide bandgap solids"
    doi: 10.1109/3.511545
    year: 1996
    role: milestone
    note: "Z-scan with 30 ps pulses at 1064 nm and its harmonics on wide-gap crystals: lithium niobate's n₂ = (48 ± 7) × 10⁻¹⁴ esu at 1064 nm, extraordinary, and its two-photon absorption 0.38 cm/GW at 532 nm (Table I)."
  - cite: "W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997)"
    title: "Spectral information and distinguishability in type-II down-conversion with a broadband pump"
    doi: 10.1103/PhysRevA.56.1627
    year: 1997
    role: milestone
    note: "The two-photon state of down-conversion as a joint spectral amplitude, the pump's spectrum times the phase matching (Eqs. 7–9): a broadband pump leaves the photons less anticorrelated and spectrally distinguishable, which lowers their interference, and filters restore it."
  - cite: "I. Shoji, T. Kondo, A. Kitamoto, M. Shirane, R. Ito, J. Opt. Soc. Am. B 14, 2268 (1997)"
    title: "Absolute scale of second-order nonlinear-optical coefficients"
    doi: 10.1364/JOSAB.14.002268
    year: 1997
    role: review
    note: "Second-order coefficients remeasured with the multiple reflections in plate samples accounted for, which had made many accepted values too large: lithium niobate's d₃₃ = 25.2 pm/V at 1.064 µm (Table 10), GaAs's 170 pm/V at 1.064 µm and 119 at 1.533 µm (Table 11)."
  - cite: "D. E. Zelmon, D. L. Small, D. Jundt, J. Opt. Soc. Am. B 14, 3319 (1997)"
    title: "Infrared corrected Sellmeier coefficients for congruently grown lithium niobate and 5 mol% magnesium oxide–doped lithium niobate"
    doi: 10.1364/JOSAB.14.003319
    year: 1997
    role: milestone
    note: "The ordinary and extraordinary indices of congruent and 5 % MgO-doped lithium niobate from 0.4 to 5 µm at 21 °C, the catalogue's default model for both crystals."
  - cite: "D. H. Jundt, Opt. Lett. 22, 1553 (1997)"
    title: "Temperature-dependent Sellmeier equation for the index of refraction, n_e, in congruent lithium niobate"
    doi: 10.1364/OL.22.001553
    year: 1997
    role: milestone
    note: "Congruent lithium niobate's n_e over wavelength and temperature from quasi-phase-matched devices, and the crystal's thermal expansion along the grating, 1.54 × 10⁻⁵ /K and 5.3 × 10⁻⁹ /K² (Eq. 3)."
  - cite: "M. Levy, R. M. Osgood, R. Liu, L. E. Cross, G. S. Cargill, A. Kumar, H. Bakhru, Appl. Phys. Lett. 73, 2293 (1998)"
    title: "Fabrication of single-crystal lithium niobate films by crystal ion slicing"
    doi: 10.1063/1.121801
    year: 1998
    role: milestone
    note: "3.8 MeV helium ions implanted into lithium niobate left a damaged layer about 10 µm down; etched away in dilute HF, it released films about 9 µm thick, bonded to silicon or GaAs: the start of thin-film lithium niobate."
  - cite: "I. Vurgaftman, J. R. Meyer, L. R. Ram-Mohan, J. Appl. Phys. 89, 5815 (2001)"
    title: "Band parameters for III–V compound semiconductors and their alloys"
    doi: 10.1063/1.1368156
    year: 2001
    role: review
    note: "Recommended band parameters of the III–V semiconductors and their alloys, among them GaAs's Γ gap of 1.519 eV at 0 K and its temperature dependence (Table I) and wurtzite AlN's 6.23 eV (Table X): the catalogue's band edges."
  - cite: "M. A. Foster, A. C. Turner, J. E. Sharping, B. S. Schmidt, M. Lipson, A. L. Gaeta, Nature 441, 960 (2006)"
    title: "Broad-band optical parametric gain on a silicon photonic chip"
    doi: 10.1038/nature04932
    year: 2006
    role: milestone
    note: "Silicon guides 300 × 550 nm with anomalous dispersion engineered by their cross-section gave four-wave-mixing gain on chip: net on/off gain over 28 nm, conversion from 1,511 to 1,591 nm, up to 2.9 dB of on/off gain at 4.2 W of peak pump in 17 mm."
  - cite: "A. D. Bristow, N. Rotenberg, H. M. van Driel, Appl. Phys. Lett. 90, 191104 (2007)"
    title: "Two-photon absorption and Kerr coefficients of silicon for 850–2200 nm"
    doi: 10.1063/1.2737359
    year: 2007
    role: milestone
    note: "Z-scan with 200 fs pulses on bulk silicon from 850 to 2200 nm: two-photon absorption of 2.1 ± 0.4 cm/GW and n₂ = (4.7 ± 2.0) × 10⁻¹⁴ cm²/W at 1220 nm, n₂ peaking at 1.2 × 10⁻¹³ cm²/W near 1800 nm, the absorption falling to zero beyond 2200 nm."
  - cite: "Q. Lin, O. J. Painter, G. P. Agrawal, Opt. Express 15, 16604 (2007)"
    title: "Nonlinear optical phenomena in silicon waveguides: modeling and applications"
    doi: 10.1364/OE.15.016604
    year: 2007
    role: review
    note: "One model of silicon waveguides' nonlinear optics: the Kerr effect, two-photon absorption, the free carriers it makes and their absorption and index change (Eqs. 20, 37, 47–49), and Raman amplification and lasing (Sec. 4)."
  - cite: "K. Ikeda, R. E. Saperstein, N. Alic, Y. Fainman, Opt. Express 16, 12987 (2008)"
    title: "Thermal and Kerr nonlinear properties of plasma-deposited silicon nitride/silicon dioxide waveguides"
    doi: 10.1364/OE.16.012987
    year: 2008
    role: milestone
    note: "Plasma-deposited silicon nitride's n₂ from the shift of a ring's resonance under a pump modulated at 1 GHz, too fast for heat: 2.4 × 10⁻¹⁵ cm²/W, ten times silica's."
  - cite: "S. Afshar V., T. M. Monro, Opt. Express 17, 2298 (2009)"
    title: "A full vectorial model for pulse propagation in emerging waveguides with subwavelength structures part I: Kerr nonlinearity"
    doi: 10.1364/OE.17.002298
    year: 2009
    role: milestone
    note: "The nonlinear coefficient of a high-contrast guide from its full vector fields (Eqs. 39–44): the effective area from the Poynting vector, and a silicon rod in silica whose γ is twice the usual estimate at a diameter of 0.19 µm at 800 nm."
  - cite: "L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010)"
    title: "Spontaneous four-wave mixing in microring resonators"
    doi: 10.1364/OL.35.003006
    year: 2010
    role: milestone
    note: "Photon pairs from a ring by spontaneous four-wave mixing: the field enhancement of each resonance, the generated power as the cube of the pump's enhancement times that of the signal and idler (Eq. 6), and Schmidt numbers from 10.00 for a 5 ns pump to 1.09 for 5 ps."
  - cite: "S. Coen, H. G. Randle, T. Sylvestre, M. Erkintalo, Opt. Lett. 38, 37 (2012)"
    title: "Modeling of octave-spanning Kerr frequency combs using a generalized mean-field Lugiato–Lefever model"
    doi: 10.1364/OL.38.000037
    year: 2012
    role: milestone
    note: "The Lugiato–Lefever equation with the waveguide's whole dispersion (Eq. 3) as a model of microresonator combs, an MgF₂ resonator's single soliton, about 400 fs (Fig. 2), and an octave-spanning comb (Fig. 3)."
  - cite: "T. Herr, V. Brasch, J. D. Jost, C. Y. Wang, N. M. Kondratiev, M. L. Gorodetsky, T. J. Kippenberg, Nat. Photonics 8, 145 (2013)"
    title: "Temporal solitons in optical microresonators"
    doi: 10.1038/nphoton.2013.343
    year: 2013
    role: milestone
    note: "Dissipative Kerr solitons in an MgF₂ microresonator, reached by tuning the pump across the resonance and seen as steps in the transmission: a single soliton of 200 fs measured, its comb's spectrum a sech² (Eqs. 1–2)."
  - cite: "J. W. Silverstone, D. Bonneau, K. Ohira, N. Suzuki, H. Yoshida, N. Iizuka, M. Ezaki, C. M. Natarajan, M. G. Tanner, R. H. Hadfield, V. Zwiller, G. D. Marshall, J. G. Rarity, J. L. O'Brien, M. G. Thompson, Nat. Photonics 8, 104 (2013)"
    title: "On-chip quantum interference between silicon photon-pair sources"
    doi: 10.1038/nphoton.2013.339
    year: 2013
    role: milestone
    note: "Two spontaneous four-wave-mixing sources, 5.2 mm silicon spirals, on one chip in an interferometer: their pairs interfere with a visibility of up to 100.0 ± 0.4 % on chip."
  - cite: "H. Jung, C. Xiong, K. Y. Fong, X. Zhang, H. X. Tang, Opt. Lett. 38, 2810 (2013)"
    title: "Optical frequency comb generation from aluminum nitride microring resonator"
    doi: 10.1364/OL.38.002810
    year: 2013
    role: current
    note: "A Kerr comb from an aluminium nitride ring, about 70 lines over 200 nm, and the material's n₂, (2.3 ± 1.5) × 10⁻¹⁵ cm²/W, derived from the comb's threshold of about 210 mW."
  - cite: "M. Leidinger, S. Fieberg, N. Waasem, F. Kühnemann, K. Buse, I. Breunig, Opt. Express 23, 21690 (2015)"
    title: "Comparative study on three highly sensitive absorption measurement techniques characterizing lithium niobate over its entire transparent spectral range"
    doi: 10.1364/OE.23.021690
    year: 2015
    role: current
    note: "Lithium niobate's absorption across its window, measured three ways: the band edge around 320 nm and multiphonon absorption from around 3000 nm."
  - cite: "M. H. P. Pfeiffer, A. Kordts, V. Brasch, M. Zervas, M. Geiselmann, J. D. Jost, T. J. Kippenberg, Optica 3, 20 (2016)"
    title: "Photonic Damascene process for integrated high-Q microresonator based nonlinear photonics"
    doi: 10.1364/OPTICA.3.000020
    year: 2016
    role: current
    note: "Silicon nitride rings made by filling trenches and polishing, Q to 3.7 × 10⁶, and a single-soliton comb in a 238.2 µm ring with a 1.75 × 0.85 µm guide, its measured D₂/2π 0.49 MHz in the text (1.90 MHz by the polynomial drawn in its Fig. 5(c)) against 0.26 simulated."
  - cite: "M. Pu, L. Ottaviano, E. Semenova, K. Yvind, Optica 3, 823 (2016)"
    title: "Efficient frequency comb generation in AlGaAs-on-insulator"
    doi: 10.1364/OPTICA.3.000823
    year: 2016
    role: current
    note: "AlGaAs-on-insulator rings with γ ≈ 660 /(W·m): parametric oscillation from 3 mW in the bus in a ring of 995 GHz free spectral range and a loaded Q of about 10⁵, and a comb of 98 GHz spacing over about 350 nm from 72 mW."
  - cite: "T. J. Kippenberg, A. L. Gaeta, M. Lipson, M. L. Gorodetsky, Science 361, eaan8083 (2018)"
    title: "Dissipative Kerr solitons in optical microresonators"
    doi: 10.1126/science.aan8083
    year: 2018
    role: review
    note: "Soliton microcombs reviewed: the resonances' dispersion D_int (Eq. 1), the Lugiato–Lefever equation dimensional and dimensionless (Eqs. 2–3), how solitons are reached, and their uses in coherent communications, atomic clocks, distance measurement, dual-comb spectroscopy, frequency synthesis and the calibration of astronomical spectrometers."
  - cite: "C. Wang, C. Langrock, A. Marandi, M. Jankowski, M. Zhang, B. Desiatov, M. M. Fejer, M. Lončar, Optica 5, 1438 (2018)"
    title: "Ultrahigh-efficiency wavelength conversion in nanophotonic periodically poled lithium niobate waveguides"
    doi: 10.1364/OPTICA.5.001438
    year: 2018
    role: milestone
    note: "Ridges in 600 nm x-cut thin-film lithium niobate poled at 4.1 µm over 4 mm: SHG of 1550 nm at a measured 2600 %/(W·cm²), against 4500 calculated for an ideal grating, the duty cycle of 39 ± 3 % cutting d_eff by 7 %."
  - cite: "L. Chang, A. Boes, X. Guo, D. T. Spencer, M. J. Kennedy, J. D. Peters, N. Volet, J. Chiles, A. Kowligy, N. Nader, D. D. Hickstein, E. J. Stanton, S. A. Diddams, S. B. Papp, J. E. Bowers, Laser Photonics Rev. 12, 1800149 (2018)"
    title: "Heterogeneously Integrated GaAs Waveguides on Insulator for Efficient Frequency Conversion"
    doi: 10.1002/lpor.201800149
    year: 2018
    role: current
    note: "GaAs strips 150 nm thick in silica, phase matched by their modes alone, TE at 2 µm to TM at 1 µm: SHG at 13,000 %/(W·cm²) measured, 32,000 calculated."
  - cite: "G. P. Agrawal, Nonlinear Fiber Optics, 6th ed., Academic Press (2019)"
    title: "Nonlinear Fiber Optics"
    doi: 10.1016/C2018-0-01168-8
    year: 2019
    role: review
    note: "The nonlinear Schrödinger equation and its split-step Fourier solution (Chapter 2), self-phase modulation (Chapter 4), solitons (Chapter 5) and four-wave mixing's parametric gain (Chapter 10): the equations photonoxide's pulses and four-wave mixing integrate."
  - cite: "R. W. Boyd, Nonlinear Optics, 4th ed., Academic Press (2020)"
    title: "Nonlinear Optics"
    doi: 10.1016/C2015-0-05510-1
    year: 2020
    role: review
    note: "The textbook's susceptibilities (Eq. 1.1.2), the d tensor (Eq. 1.5.22), Miller's rule from the anharmonic oscillator (Sec. 1.4.2), the coupled-wave equations and phase matching (Chapter 2) and the intensity-dependent index (Eq. 4.1.19)."
  - cite: "J. Zhao, C. Ma, M. Rüsing, S. Mookherjea, Phys. Rev. Lett. 124, 163603 (2020)"
    title: "High Quality Entangled Photon Pair Generation in Periodically Poled Thin-Film Lithium Niobate Waveguides"
    doi: 10.1103/PhysRevLett.124.163603
    year: 2020
    role: current
    note: "Photon pairs at 1570 nm from a 5 mm ridge of 300 nm x-cut MgO:LiNbO₃ poled at 2.8 µm: 4.6 × 10⁵ pairs/(s·GHz·mW), a coincidence-to-accidental ratio above 67,000 and an entanglement visibility above 99 %."
---

The lesson's devices in the symbols its equations use. Above, a periodically poled thin-film
lithium niobate guide from above: domains of $+d$ and $-d$ a period $\Lambda$ apart, the
fundamental $\omega$ in and its second harmonic $2\omega$ out. Below at the left, the same guide in
section, an x-cut ridge whose two modes overlap; at the right, a Kerr ring whose pump becomes a
comb.

::diagram nonlinear

A material's polarization follows the field linearly only while the field is weak beside the
fields that bind its electrons. Its next terms make light at new frequencies (sums, differences,
harmonics) and make the index depend on the light itself. On a chip these terms are strong because
a guide holds the light in a square micrometre or less over centimetres, and useful only when the
waves stay in step along that length: phase matching is the subject's centre. photonoxide's
`nonlinear` module computes phase matching and its bandwidths, the coupled-wave equations, the
efficiency of a guide from its modes, pulses by the nonlinear Schrödinger equation, four-wave
mixing, Kerr microresonators and photon pairs, each against closed forms and published devices
([the nonlinear write-up](../docs/methods/nonlinear.md)).

## 1. What makes an optical process nonlinear?

A field $E$ drives a polarization that photonoxide, like Boyd, expands in powers of the field
([Boyd, 2020](https://doi.org/10.1016/C2015-0-05510-1), Eq. 1.1.2):

$$
P = \varepsilon_0\left(\chi^{(1)}E + \chi^{(2)}E^2 + \chi^{(3)}E^3 + \cdots\right).
$$

The linear term is the index, $n^2 = 1 + \chi^{(1)}$. The others matter when $E$ is not small
beside the field that holds an electron to its atom, $E_\text{at} = e/(4\pi\varepsilon_0a_0^2) = 5.14 \times 10^{11}$ V/m (Boyd, p. 3): $\chi^{(2)}$ is then of order $\chi^{(1)}/E_\text{at}$, and
Boyd's estimate is $1.94 \times 10^{-12}$ m/V (Eq. 1.1.3), a few pm/V as measured crystals have it.

A field $E_0\cos\omega t$ squared is $\tfrac12E_0^2(1 + \cos 2\omega t)$: the second-order term
radiates at $2\omega$, second-harmonic generation (SHG), and at zero frequency. Two fields at
$\omega_1$ and $\omega_2$ give $\omega_1 + \omega_2$ (sum-frequency generation) and $\omega_1 - \omega_2$ (difference-frequency generation); a strong pump at $\omega_3$ amplifies a weak signal
$\omega_1$ and makes its idler $\omega_3 - \omega_1$ (parametric amplification), and from vacuum
it splits into photon pairs (spontaneous parametric down-conversion). The third-order term gives
$3\omega$, four-wave mixing $\omega_a + \omega_b = \omega_c + \omega_d$, and, at the field's own
frequency, a change of index with intensity, $n = n_0 + n_2I$ (the Kerr effect).

**Which crystals have $\chi^{(2)}$.** In a crystal with a centre of inversion, reversing the field
reverses the polarization: $P(-E) = -P(E)$. The even terms change no sign under $E \to -E$, so
they must vanish: $\chi^{(2)} = 0$ (Boyd, Sec. 1.5.10). Silicon, silica and amorphous silicon
nitride have no bulk $\chi^{(2)}$; lithium niobate, GaAs, AlN and the III–V alloys, which lack the
centre, do. Every material has $\chi^{(3)}$.

### The d tensor {theory}

Each term is a tensor: $P_i = \varepsilon_0\sum_{jk}\chi^{(2)}_{ijk}E_jE_k$. Crystals are
tabulated by $d_{ijk} = \tfrac12\chi^{(2)}_{ijk}$ (Boyd, Eq. 1.5.22), written $d_{il}$ with the
pair $jk$ contracted to one index $l$ (Voigt's notation: $xx \to 1$, $yy \to 2$, $zz \to 3$,
$yz \to 4$, $xz \to 5$, $xy \to 6$). Lithium niobate's largest is $d_{33}$, which couples fields
all along its $c$ axis. photonoxide writes each wave's field as the real part of a complex
amplitude, $\operatorname{Re}(E e^{-i\omega t})$; in that convention the polarization's amplitude
at $2\omega$ is $\varepsilon_0\sum d_{ijk}E_jE_k$, and at a sum frequency
$2\varepsilon_0\sum d_{ijk}E_{1j}E_{2k}$, since $\tfrac14(E_1e^{-i\omega_1t} + E_2e^{-i\omega_2t} + \text{c.c.})^2$ holds $\tfrac14E_1^2$ at $2\omega_1$ and $\tfrac12E_1E_2$ at $\omega_1 + \omega_2$. A crystal is put on the chip by turning its tensor into the guide's frame, $d'_{abc} = \sum M_{ai}M_{bj}M_{ck}d_{ijk}$ ([the write-up](../docs/methods/nonlinear.md#in-waveguides)).

## 2. Which materials carry a process, and how large are their χ⁽²⁾ and χ⁽³⁾?

A material carries a process where every wave lies in its transparency window: below its band edge
in wavelength light is absorbed, and below twice the band edge two photons can be absorbed
together. Two-photon absorption limits silicon below 2.2 µm and GaAs below 1.74 µm, exactly where
telecommunications work. photonoxide's catalogue holds each window with its source, and the
values below with theirs ([the catalogue](../docs/methods/catalogue.md#kerr-coefficients-and-two-photon-absorption)):

| Material | Band edge | Two-photon absorption below | $\lvert d\rvert$ (pm/V) | $n_2$ (m²/W) |
|---|---|---|---|---|
| LiNbO₃ | about 0.32 µm; multiphonon from about 3.0 µm | about 0.64 µm | $d_{33}$ 25.2, $d_{31}$ 4.6 at 1.064 µm | 9.15 × 10⁻²⁰ at 1064 nm, e |
| GaAs | 0.8716 µm (1.4225 eV, 300 K) | 1.743 µm | $d_{14}$ 170 at 1.064 µm, 119 at 1.533 µm | not in the catalogue |
| AlN | 0.2020 µm (6.138 eV) | 0.404 µm | not used here | 2.3 × 10⁻¹⁹ near 1550 nm |
| Si | 1.107 µm (indirect, 1.12 eV) | 2.2 µm | 0 (centrosymmetric) | 4.8 × 10⁻¹⁸ at 1550 nm, read from a figure |
| SiN (PECVD) | not specified | not specified | 0 (amorphous) | 2.4 × 10⁻¹⁹ at 1548 nm |

The sources: lithium niobate's window ([Leidinger et al., 2015](https://doi.org/10.1364/OE.23.021690),
p. 21692), its $d$ ([Shoji et al., 1997](https://doi.org/10.1364/JOSAB.14.002268), Table 10) and
$n_2$, $(48 \pm 7) \times 10^{-14}$ esu converted with its $n_0 = 2.2$
([DeSalvo et al., 1996](https://doi.org/10.1109/3.511545), Tables I–II); GaAs's $d$ (Shoji et
al., Table 11, which label it $d_{36}$) and its and AlN's gaps at 300 K from
[Vurgaftman et al. (2001)](https://doi.org/10.1063/1.1368156)'s Tables I and X; silicon's window
and $n_2$ ([Bristow et al., 2007](https://doi.org/10.1063/1.2737359): the text prints
$(4.7 \pm 2.0) \times 10^{-18}$ m²/W at 1220 nm, the 1550 nm value is read from its Fig. 3); silicon
nitride's ([Ikeda et al., 2008](https://doi.org/10.1364/OE.16.012987), ten times silica's); AlN's,
derived from a comb's threshold rather than measured directly
([Jung et al., 2013](https://doi.org/10.1364/OL.38.002810)). The readings from Bristow et al.'s
figures are checked where the text prints the same quantity:

::validation material/kerr-bristow-readings

Silicon's $n_2$ is 20 times silicon nitride's, but its two-photon absorption at 1550 nm,
$0.79 \pm 0.39$ cm/GW read from Bristow et al.'s Fig. 2, takes part of the light it would convert
(Section 8). Lithium niobate's window reaches into the visible; GaAs's $d$ is about seven times
larger, but it absorbs below 0.87 µm and two photons below 1.74 µm.

### Miller's rule: how d moves with the wavelength {theory}

$d$ is measured at a few fundamentals. [Miller (1964)](https://doi.org/10.1063/1.1754022) found
that across eleven crystals whose $d$ spread over a factor of about 600, $d$ divided by the product
of the three waves' linear susceptibilities stayed within a factor of two of its average (his Eq.
4):

$$
d_{ijk} = \chi_{ii}(\omega_3)\,\chi_{jj}(\omega_1)\,\chi_{kk}(\omega_2)\,\delta_{ijk}, \qquad
\chi = n^2 - 1 .
$$

Boyd derives it from an anharmonic oscillator, whose $\chi^{(2)}$ is exactly such a product with a
constant $\delta$ (Sec. 1.4.2, Eq. 1.4.28). Holding $\delta$ carries $d$ from one fundamental to
another. The chart carries lithium niobate's $d_{33}$ and $d_{31}$ from Shoji et al.'s 1.064 µm
measurement with Zelmon et al.'s indices ([Zelmon et al., 1997](https://doi.org/10.1364/JOSAB.14.003319))
and draws the measurements dashed (exact: no grid). At 0.852 µm the rule gives 28.12 pm/V for
$d_{33}$ against the measured 25.7, 9.4 % high; at 1.313 µm 23.58 against 19.5, 20.9 % high, and
$d_{31}$ 34.2 % high.

::chart nl-miller

**Try it.** Carry the doped crystal's coefficients from 1.064 µm (crystal 2). Where does the rule
hold best?

:::answer
Toward the blue: MgO:LiNbO₃'s $d_{31}$ at 0.852 µm comes out 4.918 pm/V against the measured 4.90,
0.4 % high, and its $d_{33}$ 2.0 % low; toward 1.313 µm the rule is 15.4 % and 20.9 % high. The
measured $d$ falls faster into the infrared than the susceptibilities do.
:::

Shoji et al.'s own $\delta_{33}$ varies by about 20 % between their three fundamentals (their
Table 12), which `material/linbo3-shoji-miller` reproduces; the cases take Miller's own ±50 % as the
rule's accuracy and show the deviation, nothing fitted:

::validation nonlinear/miller-linbo3-d33 nonlinear/miller-linbo3-d31 material/linbo3-shoji-miller

A nonlinear job carries the catalogue's measurement nearest its wavelengths, never the first one
listed, for this reason ([as a job](../docs/methods/nonlinear.md#as-a-job)).

## 3. How do the coupled-wave equations of second-harmonic generation follow from Maxwell's?

The harmonic's field grows from the polarization the fundamental drives, and takes its power from
the fundamental. Two equations hold that exchange; the derivation below follows
[Armstrong et al. (1962)](https://doi.org/10.1103/PhysRev.127.1918) and Boyd's Chapter 2, in
photonoxide's convention.

### The derivation {theory}

Take plane waves along $z$, each $\operatorname{Re}[A_j(z)e^{i(k_jz - \omega_jt)}]$ with $k_j = n_j\omega_j/c$. The wave equation with the nonlinear polarization as a source,
$\nabla^2E - (n^2/c^2)\,\partial_t^2E = \mu_0\,\partial_t^2P_\text{NL}$, for the amplitude at
$\omega_j$ reads

$$
\frac{d^2}{dz^2}\left(A_je^{ik_jz}\right) + k_j^2A_je^{ik_jz} = -\mu_0\omega_j^2P_j .
$$

Expanding the first term leaves $(A_j'' + 2ik_jA_j')e^{ik_jz}$. When the amplitude changes little
over a wavelength, $\lvert A_j''\rvert \ll \lvert k_jA_j'\rvert$ (the slowly varying envelope
approximation), so

$$
\frac{dA_j}{dz} = \frac{i\mu_0\omega_j^2}{2k_j}\,P_je^{-ik_jz}
= \frac{i\omega_j}{2n_j\varepsilon_0c}\,P_je^{-ik_jz} .
$$

For SHG of a fundamental $A_1$ into a harmonic $A_2$ with one effective coefficient $d$, Section
1's polarizations are $P_2 = \varepsilon_0dA_1^2e^{2ik_1z}$ and $P_1 = 2\varepsilon_0dA_2A_1^* e^{i(k_2 - k_1)z}$. With $\Delta k = k_2 - 2k_1$:

$$
\frac{dA_2}{dz} = \frac{i\omega d}{n_2c}\,A_1^2e^{-i\Delta kz}, \qquad
\frac{dA_1}{dz} = \frac{i\omega d}{n_1c}\,A_1^*A_2e^{i\Delta kz} .
$$

Over an area $\mathcal A$ a wave carries the power $\tfrac12n_j\varepsilon_0c\lvert A_j\rvert^2 \mathcal A$; rescaling each amplitude to $a_j$ with $\lvert a_j\rvert^2$ its power gives one
coupling for both equations,

$$
\frac{da_2}{dz} = i\kappa\,a_1^2e^{-i\Delta kz}, \qquad \frac{da_1}{dz} = i\kappa\,a_1^*a_2e^{i\Delta kz},
\qquad \kappa^2 = \eta = \frac{2\omega^2d^2}{\varepsilon_0c^3n_1^2n_2\mathcal A} .
$$

$\eta$ is the normalized efficiency. photonoxide's `nonlinear::coupled` writes $-i\kappa$ in both,
the same equations with the harmonic's phase taken $\pi$ apart. They conserve $\lvert a_1\rvert^2 + \lvert a_2\rvert^2$: multiply each by its conjugate amplitude and add.

**Undepleted.** With $a_1 = \sqrt P$ held, $a_2(L) = i\kappa P\int_0^Le^{-i\Delta kz}dz$, and

$$
P_{2\omega} = \eta P^2L^2\,\mathrm{sinc}^2(\Delta kL/2), \qquad \mathrm{sinc}\,x = \sin x/x .
$$

**Depleted, matched.** Put $a_1 = \sqrt{P}\,u$ and $a_2 = i\sqrt{P}\,v$ with $u$, $v$ real: the
equations become $v' = \kappa\sqrt P\,u^2$ and $u' = -\kappa\sqrt P\,uv$, and with $u^2 + v^2 = 1$,
$v' = \kappa\sqrt P(1 - v^2)$, so $v = \tanh(\kappa\sqrt Pz)$ and

$$
P_{2\omega}(z) = P\tanh^2(\kappa\sqrt{P}z),
$$

Armstrong et al.'s exact solution (their Sec. V, Eq. 5.12): every fundamental photon pair becomes a
harmonic photon as $z \to \infty$.

### What the chart integrates {theory}

The chart integrates the equations by the classical fourth-order Runge–Kutta method, as the library
does, on 2000 steps over the length. At its defaults, 100 mW into a guide of 4500 %/(W·cm²) and
4 mm, $\kappa\sqrt PL = 0.8485$: the harmonic takes 0.4765 of the power, Armstrong's $\tanh^2$ to
the digits shown, while the undepleted formula would give 0.7200.

::chart nl-coupled

**Try it.** Raise the power to 1 W, then set the mismatch $\Delta kL/2$ to 2 at 100 mW.

:::answer
At 1 W, $\kappa\sqrt PL = 2.683$ and 0.9815 of the power is converted, where the undepleted formula
gives 7.2, an impossibility: the fundamental is spent. At $\Delta kL/2 = 2$ and 100 mW the harmonic
takes 0.0751 against the undepleted 0.1488: it grows and returns to the fundamental as the waves
slip out of step.
:::

The cases check the integrator against the closed forms: undepleted, depleted ($\tanh^2 2 = 0.929349$), the equal-flux sum of Armstrong et al.'s Sec. VI.A, parametric gain ($\cosh^2 2$), the
Manley–Rowe photon fluxes conserved to round-off, and RK4's fourth order (4.06, 4.03, 4.04 for
successive halvings from 125 steps):

::validation nonlinear/coupled-undepleted nonlinear/coupled-shg-tanh nonlinear/coupled-sfg-sech nonlinear/coupled-opa-cosh nonlinear/coupled-manley-rowe nonlinear/coupled-rk4-order

## 4. What is phase matching, and how wide is its band?

The polarization at $2\omega$ travels at the fundamental's phase velocity, the free harmonic at its
own. Where $n_{2\omega} \ne n_\omega$ the two slip apart, and after a coherence length the harmonic
made earlier is out of phase with what is being made:

$$
\Delta k = k_{2\omega} - 2k_\omega = \frac{4\pi}{\lambda}\left(n_{2\omega} - n_\omega\right),
\qquad l_c = \frac{\pi}{\lvert\Delta k\rvert} = \frac{\lambda}{4\lvert n_{2\omega} - n_\omega\rvert},
$$

λ the fundamental's vacuum wavelength. In normally dispersive lithium niobate, $d_{33}$ SHG of
1.064 µm has $l_c = 3.401$ µm (Edwards and Lawrence's indices, exact: no grid): beyond it the
harmonic shrinks again (Section 5's chart). Phase matching makes $\Delta k = 0$ by birefringence
(the two waves on different axes of the crystal), by a guide's dispersion (two modes of the same
index), or by a grating that adds its own wave vector (Section 5).

### The acceptance bandwidths {theory}

Near a matched point the response is $\mathrm{sinc}^2(\Delta kL/2)$, which falls to ½ at
$\Delta kL/2 = \pm 1.3916 = \pm 0.4429\pi$ ([Fejer et al., 1992](https://doi.org/10.1109/3.161322),
p. 2635; the case `nonlinear/sinc2-half-maximum`). Any variable $\zeta$ that moves $\Delta k$
linearly then has the full width (their Eq. 25)

$$
\delta\zeta = \frac{5.57}{L}\left\lvert\frac{\partial\Delta k}{\partial\zeta}\right\rvert^{-1} .
$$

For the fundamental's wavelength, with $k_\omega = 2\pi n_\omega/\lambda$ and $k_{2\omega} = 4\pi n_{2\omega}/\lambda$ and the group index $n_g = n - \lambda\,dn/d\lambda$,
$dk_\omega/d\lambda = -2\pi n_{g,\omega}/\lambda^2$ and $dk_{2\omega}/d\lambda = -4\pi n_{g,2\omega}/\lambda^2$, so

$$
\frac{\partial\Delta k}{\partial\lambda} = -\frac{4\pi}{\lambda^2}\left(n_{g,2\omega} - n_{g,\omega}\right),
\qquad \delta\lambda = \frac{0.4429\,\lambda^2}{L\,\lvert n_{g,2\omega} - n_{g,\omega}\rvert} :
$$

Fejer et al.'s Eq. 30, which they write with the phase indices' derivatives. The width falls as
$1/L$ while the efficiency grows as $L^2$, and it is set by the group-velocity mismatch, not by the
indices. The temperature's width adds the crystal's thermal expansion, which stretches the grating
(their Eq. 44). The chart draws the response of first-order quasi-phase-matched $d_{33}$ SHG in
bulk congruent lithium niobate, 1 mm long at 25 °C, with Edwards and Lawrence's $n_e(T)$
([Edwards and Lawrence, 1984](https://doi.org/10.1007/BF00620081)) and
[Jundt (1997)](https://doi.org/10.1364/OL.22.001553)'s expansion (exact: no grid): at 1.064 µm,
$\delta\lambda = 2.036$ nm and $\delta T = 25.77$ K, Fejer et al.'s 2.04 nm and 25.8 K.

::chart nl-acceptance

**Try it.** Make the crystal 10 mm long; then move the fundamental to 1.55 µm at 1 mm.

:::answer
At 10 mm both widths fall tenfold, to 0.2036 nm and 2.577 K. At 1.55 µm the coherence length is
9.512 µm (a period of 19.02 µm) and $\delta\lambda$ 11.76 nm: the group indices of 1.55 and
0.775 µm lie closer than those of 1.064 and 0.532 µm. The temperature width there, 76.1 K by Eq.
44, has no numerical counterpart: its lower half maximum lies below 0 °C, outside the index model.
:::

### Fejer et al.'s Table I {research}

Fejer et al.'s Table I prints the coherence lengths and widths of SHG at three fundamentals in
1 mm of lithium niobate. [qpm_fejer](../examples/qpm_fejer.rs) computes every quasi-phase-matched
entry from the indices the paper names and checks each to half a unit of its last printed digit: 41
of the 49 values agree.

::example qpm_fejer

Eight don't, and the example lists them: six differ by 0.3 to 4 % (among them $l_c$ = 129 µm, ours
129.6, where the paper's own angular width, 959 mrad, agrees with ours), and two numerical
temperature widths at 2.12 µm, (203) and (455) K, have their half maxima below 0 °C, outside the
range Edwards and Lawrence fitted. The birefringent rows phase-match at −19.4 and 580.3 °C, outside
it too ([the write-up](../docs/methods/nonlinear.md#fejer-et-als-table-i) has each value).

::validation nonlinear/fejer-table-1-coherence nonlinear/fejer-table-1-wavelength nonlinear/fejer-table-1-temperature

Edwards and Lawrence checked their equation by difference-frequency mixing in a 50 mm crystal
matched by temperature alone (their Table II). photonoxide reproduces the calculated column to
0.07 °C only with the printed wavelengths read as wavelengths in air: read as vacuum wavelengths
every row comes out 0.22 to 0.47 °C low. The measured temperatures sit up to 4.5 °C from the
equation's, the "systematic difference of about 3 °C" the paper notes.

::validation nonlinear/edwards-lawrence-table-2

### Birefringent phase matching {theory}

An extraordinary wave at an angle $\theta$ to the optic axis sees $1/n_e(\theta)^2 = \cos^2\theta/ n_o^2 + \sin^2\theta/\bar n_e^2$ (Boyd, Eq. 2.3.8). Type I puts both fundamental photons on one
polarization and the harmonic on the other, so $\Delta k = 0$ where $n_e(\theta)$ at $2\omega$
meets $n_o$ at $\omega$; `phase_matching_angle` finds that angle and `phase_matching_temperature`
the temperature at a fixed angle, noncritical at 90°. Birefringence uses the off-diagonal $d_{31}$,
5.5 times smaller than $d_{33}$ in lithium niobate, and walk-off moves the beams apart; quasi-phase
matching uses $d_{33}$ with all waves on one axis (type 0).

## 5. How does periodic poling phase-match a crystal?

Reversing the crystal's orientation reverses the sign of $d$. Done every coherence length, just
where the harmonic would start to fall, it keeps the harmonic growing:
[Armstrong et al. (1962)](https://doi.org/10.1103/PhysRev.127.1918) proposed it with crystal plates,
each the inversion of the last (their Sec. VIII.A, Fig. 10). In lithium niobate voltage pulses on
patterned electrodes write the domains (Section 13).
The chart sums the harmonic's field domain by domain with no grating, poled and phase matched:
without a grating the power peaks at $(2/\pi)^2 = 0.4053$ of the matched power at one coherence
length and returns to zero at two; poled at 50 %, it grows on average as $(2/\pi)^2$ of the
matched.

::chart nl-qpm

### The grating's Fourier coefficient {theory}

Write the poled coefficient as $d\,s(z)$, $s = +1$ over a share $D$ of each period $\Lambda$ and
$-1$ over the rest. Its Fourier series is $s(z) = \sum_m G_me^{iK_mz}$, $K_m = 2\pi m/\Lambda$, with

$$
G_m = \frac1\Lambda\left(\int_0^{D\Lambda} - \int_{D\Lambda}^{\Lambda}\right)e^{-iK_mz}\,dz
= \frac{2}{m\pi}\sin(m\pi D)\,e^{-im\pi D},
$$

since $K_m\Lambda = 2\pi m$. In the undepleted integral $\int_0^L s(z)e^{-i\Delta kz}dz$ every term
oscillates and stays bounded except the one with $K_m = \Delta k$, which grows as $G_mL$: the
grating phase-matches at order $m$ when

$$
\Lambda = \frac{2\pi m}{\lvert\Delta k\rvert} = 2m\,l_c, \qquad d_Q = \lvert G_m\rvert\,d
= \frac{2}{m\pi}\left\lvert\sin(m\pi D)\right\rvert d ,
$$

Fejer et al.'s Eq. 22. At first order and 50 %, $d_Q = (2/\pi)d$ and the efficiency is 0.405 of a
matched crystal's; even orders vanish at 50 %; third order at 50 % keeps $(2/3\pi)^2 = 0.0450$. For
1.064 µm in bulk lithium niobate the first-order period is $2l_c = 6.802$ µm.

**Try it.** In the chart, set the order to 2, then to 3; then a duty cycle of 0.39 at first order.

:::answer
Order 2 at 50 % doesn't grow: $G_2 = 0$. Order 3 grows as 0.04503 of the matched power, a period of
6 coherence lengths. At 39 % first order the share is 0.3588 against 0.4053, $G_1$ 5.91 % below
its 50 % value.
:::

### Against Wang et al.'s duty cycle {research}

[Wang et al. (2018)](https://doi.org/10.1364/OPTICA.5.001438) poled 600 nm x-cut thin-film lithium
niobate at 4.1 µm over 4 mm and measured a duty cycle of 39 ± 3 %, which they write "reduces the
actual $d_\text{eff}$ by 7 %". Eq. 22 gives 5.91 %; across the stated ±3 % it runs from 3.14 % to
9.52 %, so their 7 % lies inside their own uncertainty.

::validation nonlinear/wang-2018-duty-cycle

[ppln_wang](../examples/ppln_wang.rs) checks that and the bulk group-velocity mismatch between
1550 and 775 nm, 301.9 fs/mm against their "~300". Their 4500 %/(W·cm²) calculated for the ridge
and the 4.1 µm period need the ridge's etch depth and sidewall, which the paper doesn't print: not
reproduced.

::example ppln_wang

## 6. How efficient is second-harmonic generation in a waveguide?

In a guide the waves are modes, and Section 3's area $\mathcal A$ becomes the modes' overlap. A
thin-film guide holds both waves in about a square micrometre over millimetres (Zhao et al.'s
below: 1.1 and 0.4 µm² over 5 mm), and Section 3's $\eta \propto 1/\mathcal A$ is the gain.
photonoxide's `nonlinear::guide` takes each wave's mode from the full-vector solver, scaled to
carry 1 W, and the crystal's $d$ turned into the guide's frame
([in waveguides](../docs/methods/nonlinear.md#in-waveguides)).

### The overlap {theory}

A mode $\hat e(x, y)$ of 1 W driven by a polarization grows as $dA/dz = (i\omega/4)\int\hat e^* \cdot P\,dA\,e^{-i\beta z}$, the first-order perturbation photonoxide uses for thermo-optic and
Pockels changes. With the SHG polarization of Section 1,

$$
\kappa = \frac{\omega\varepsilon_0}{2}\left\lvert\int\sum_{ijk}\hat e_{2\omega,i}^*\,d_{ijk}\,
\hat e_{\omega,j}\,\hat e_{\omega,k}\,dA\right\rvert, \qquad \eta = \kappa^2 G_m^2 ,
$$

ω the fundamental's, the integral over the poled crystal, and $\eta$ in %/(W·cm²), 1 %/(W·cm²)
being $10^{-10}$ /(W·µm²). Two closed forms check it. Uniform fields over an area give Section 3's
plane-wave $\eta$ to $6.7 \times 10^{-16}$; a lithium niobate slab's exact fields give its $\eta$,
which the full-vector fields approach at second order, 2.8 × 10⁻⁵ on 10 nm cells and 1.8 × 10⁻⁶ on
2.5 nm:

::validation nonlinear/guide-plane-wave nonlinear/guide-slab-overlap

### Zhao et al.'s ridge {research}

[Zhao et al. (2020)](https://doi.org/10.1103/PhysRevLett.124.163603) give a guide whole: 300 nm of
x-cut 5 % MgO:LiNbO₃ on 1.8 µm of silica, etched 50 nm to a ridge 1.24 µm wide with 75° sidewalls,
air above, along the crystal's y so that $c$ lies across and the TE modes meet $d_{33}$, poled over
5 mm at "the calculated QPM period, Λ = 2.8 µm" for TE₀ at 1570 and 785 nm. The chart solves both
modes on 20 nm cells: the period 2.8172 µm, the effective areas 1.0788 and 0.3969 µm² (Afshar and
Monro's Eq. 41), and with the paper's $d_{33} = 27$ pm/V, poled through at 50 %,
$\eta = 4160$ %/(W·cm²). The harmonic's mode is the smaller: their overlap, not either area alone,
sets $\eta$.

::chart nl-ridge

**Try it.** Widen the ridge to 2 µm, narrow it to 0.8 µm, then etch it 150 nm deep at 1.24 µm.

:::answer
At 2 µm η falls to 3774 %/(W·cm²) and the period rises to 2.8968 µm; at 0.8 µm the fundamental
spreads out of the ridge (1.178 µm²) and η falls to 3713. Etched 150 nm, both modes shrink, to
0.6827 and 0.3083 µm², and η rises to 6238.5 with a period of 2.5975 µm (all on 20 nm cells).
:::

On finer cells the period converges to 2.8150 and 2.8142 µm (10 and 5 nm), within the paper's
printed digit, and the guide phase-matches 1.5583 µm at 2.8 µm on 10 nm cells, 11.7 nm short of
their 1570 nm, which the half digit of their period, ±0.05 µm, moves by ±38.9 nm. The areas round
to their 1.1 and 0.4 µm²:

::validation nonlinear/guide-zhao-period nonlinear/guide-zhao-wavelength nonlinear/guide-zhao-area-signal nonlinear/guide-zhao-area-pump

[pairs_zhao](../examples/pairs_zhao.rs) computes the same on 20 and 10 nm cells, and the pairs'
spectrum of Section 12:

::example pairs_zhao

**Not reproduced: Zhao et al.'s brightness.** η is 4160.1, 4163.7 and 4164.7 %/(W·cm²) on 20, 10
and 5 nm cells. Their pairs' brightness, 4.6 × 10⁵ pairs/(s·GHz·mW), is 22.6 times below what this
$\eta$ gives for an ideal grating and 16.2 times below with their measured duty cycle (55.2 %) and
their poling depth read as 245 nm from the top. The paper prints no calculated efficiency, so the
gap can't be put on the overlap or on the measurement; it stays open with the other gaps against
papers. Wang et al.'s measured 2600 %/(W·cm²) and calculated 4500 need their ridge's etch depth,
and [Chang et al. (2018)](https://doi.org/10.1002/lpor.201800149)'s GaAs strip, 150 nm thick and
1.5 µm wide, phase-matched by its TE mode at 2 µm and TM mode at 1 µm (13,000 %/(W·cm²) measured),
needs its supporting information: with vertical walls our strip's two indices meet at 2.655 µm wide,
not 1.5, the match 66 nm (3.3 %) above their 2 µm (10 nm cells). Neither is reproduced.

### As a job {research}

A job of kind `"nonlinear"` sets up the same from a file ([jobs/ppln-shg.yaml](../jobs/ppln-shg.yaml)):
the process, a crystal, effective indices or a ridge, the grating and the pump; its run records the
mismatch, the bandwidths, the efficiency along the length, a ridge's temperature acceptance and a
photon pair's joint spectrum, and
the studio's Materials page edits it. Zhao et al.'s guide through a job designs 2.8172 µm on its
default 20 nm cells and gives the library's η to round-off:

::validation job/nonlinear-ridge-zhao-period job/nonlinear-ridge-zhao-efficiency

## 7. What do difference-frequency generation and parametric gain add?

A pump at $\omega_p$ and a signal at $\omega_s$ make an idler at $\omega_i = \omega_p - \omega_s$,
and the idler and pump in turn amplify the signal: each signal photon gained comes with an idler
photon and costs a pump photon (the Manley–Rowe relations, Armstrong et al., Eq. 6.6). With the pump
undepleted the three-wave equations of [the write-up](../docs/methods/nonlinear.md#the-coupled-amplitude-equations)
give, for a signal and no idler in,

$$
G = \frac{P_s(L)}{P_s(0)} = 1 + \frac{\Gamma^2}{g^2}\sinh^2 gL, \qquad g^2 = \Gamma^2 - (\Delta k/2)^2,
\qquad \Gamma = \frac{\kappa\sqrt{\nu_s\nu_iP_p}}{\nu_p},
$$

$\nu = 1/\lambda$, and at $\Delta k = 0$, $G = \cosh^2\Gamma L$. The chart's second process
integrates the coupled equations for 1.50 µm amplified by a 0.775 µm pump in the same 4 mm guide:
100 mW of pump gives a gain of 1.191, 1 W 4.170 (6.20 dB), each the closed form to the digits shown.

::chart nl-coupled{process=2, power=1000}

### Oscillators {theory}

Feeding the signal back turns the amplifier into an optical parametric oscillator. Singly resonant,
the signal returns from each round trip with its amplitude times $\sqrt R$, and the undepleted guide
multiplies it by $\cosh gL$; it oscillates when the product reaches 1,

$$
\cosh g_\text{th}L = \frac{1}{\sqrt{R}} .
$$

`nonlinear::opo` finds the threshold from the round trip's transfer matrix and the steady state by
running round trips with the depleted equations, where each signal photon that leaves is a pump
photon lost. The library's PPLN oscillator cases find their idlers through the same phase matching:
a 1.064 µm pump at a 30 µm period gives 3.354 µm with Zelmon et al.'s indices, which their Fig. 3
plots between 3.3 and 3.4 µm, and at 250 °C and 25.5 µm with Jundt's 4.755 µm, read from his Fig. 1:

::validation nonlinear/opo-threshold-doubly nonlinear/opo-threshold-singly nonlinear/opo-steady-photons material/linbo3-zelmon-opo material/linbo3-jundt-opo

## 8. What does the Kerr effect do to light in a waveguide?

The third-order term at the field's own frequency changes the index by $n_2I$, with $n_2 = 3\chi^{(3)}/(4n_0^2\varepsilon_0c)$ for a lossless medium (Boyd, Eq. 4.1.19). In a guide the
intensity is the power over an effective area, and a mode of power $P$ gains the phase $\gamma P$
per metre, with the nonlinear coefficient
([Afshar and Monro, 2009](https://doi.org/10.1364/OE.17.002298), Eq. 42 with $n_2$ uniform)

$$
\gamma = \frac{2\pi n_2}{\lambda A_\text{eff}}, \qquad
A_\text{eff} = \frac{\left\lvert\int(\mathbf e\times\mathbf h^*)\cdot\hat z\,dA\right\rvert^2}
{\int\left\lvert(\mathbf e\times\mathbf h^*)\cdot\hat z\right\rvert^2dA}
\quad\text{(their Eq. 41)} .
$$

A Gaussian mode of area π µm² with silicon nitride's $n_2$ has $\gamma = 0.310$ /(W·m), the closed
form to round-off. A silicon nitride strip 1.75 × 0.85 µm in silica, Pfeiffer et al.'s
cross-section, has $A_\text{eff} = 1.1357$ µm² and $\gamma = 0.8566$ /(W·m) at 1.55 µm (TE₀, 20 and
10 nm cells alike). Where the index contrast is high, the full-vector form weights $n_2$ by the
fields, $E_z$ included: Afshar and Monro's silicon rod in silica, 0.19 µm across at 800 nm, has
twice the usual $\gamma$ (their Fig. 1); `VectorMode::vector_nonlinear_coefficient` computes it.

::validation nonlinear/kerr-gamma-gaussian

### Self-phase modulation {theory}

A pulse's peak gains the most phase, its wings the least: without dispersion it keeps its shape and
gains $\varphi(T) = \gamma\lvert A(T)\rvert^2L_\text{eff}$, $L_\text{eff} = (1 - e^{-\alpha L})/\alpha$.
The phase's slope is a frequency shift, red on the leading edge and blue on the trailing, so the
spectrum broadens; two instants with the same shift interfere, so it splits into peaks. A
Gaussian's RMS spectral width grows by $(1 + 4\varphi_\text{max}^2/(3\sqrt3))^{1/2}$
([Agrawal, 2019](https://doi.org/10.1016/C2018-0-01168-8), Eq. 4.1.16). At $\varphi_\text{max} = 4.5\pi$ the chart's split-step gives 12.44395, the closed form to the digits shown (1024 points
across 64 T₀; the Kerr step is exact without dispersion).

::chart nl-spm

::validation nonlinear/nlse-spm-phase nonlinear/nlse-spm-broadening

### Two-photon and free-carrier absorption in silicon {theory}

In silicon below 2.2 µm, two photons are absorbed together at a rate $\beta_T\lvert A\rvert^2/ A_\text{eff}$, and the carriers they free absorb more and lower the index. photonoxide's
`Guide::with_two_photon` adds both to the NLSE as
[Lin et al. (2007)](https://doi.org/10.1364/OE.15.016604) model them (their Eqs. 37 and 47–49),
with their carrier absorption $\sigma_a = 1.45 \times 10^{-21}$ m² at 1.55 µm (Eq. 20). A 0.5 W
continuous wave over 5 cm with a 100 ps carrier lifetime loses 1.51 dB to two-photon absorption
alone; the carriers add 0.82 dB and turn its nonlinear phase negative, their index change
outweighing the Kerr's. These are closed forms the split-step meets to $10^{-7}$:

::validation nonlinear/nlse-tpa-power nonlinear/nlse-fca-cw-power nonlinear/nlse-fca-cw-phase

## 9. How does a pulse become a soliton, and how is the NLSE integrated?

Dispersion spreads a pulse; with anomalous dispersion it sends the blue ahead of the red, the
opposite of what self-phase modulation does to the pulse's edges. When the two balance, a pulse
travels unchanged: a soliton. The nonlinear Schrödinger equation (NLSE) holds both, for the
envelope $A(z, T)$ in the frame moving at the group velocity (Agrawal, Eq. 2.3.46):

$$
\frac{\partial A}{\partial z} = -\frac{\alpha}{2}A - \frac{i\beta_2}{2}\frac{\partial^2A}{\partial T^2}
+ i\gamma\lvert A\rvert^2A ,
$$

with $\beta_2 = d^2\beta/d\omega^2$; photonoxide keeps $\beta_3, \beta_4, \ldots$ too, and optional
Raman and self-steepening terms ([pulses](../docs/methods/nonlinear.md#pulses-along-a-kerr-guide)).
The dispersion length $L_D = T_0^2/\lvert\beta_2\rvert$ (Agrawal, Eq. 3.1.5) and the nonlinear
length $1/(\gamma P_0)$ set the soliton order $N^2 = \gamma P_0T_0^2/\lvert\beta_2\rvert$ (Eq. 5.2.3).
At $N = 1$, $\sqrt{P_0}\,\mathrm{sech}(T/T_0)$ keeps its shape; higher orders narrow, split and
return to their input every soliton period $z_0 = (\pi/2)L_D$ (Eq. 5.2.24). The chart's third-order
soliton at $z_0/2$ peaks at 2.299 times its input; back at $z_0$ its power differs from the input's
by $2.0 \times 10^{-4}$ of its peak (2000 split steps a period, 1024 points across 50 T₀).

::chart nl-soliton

**Try it.** Set the order to 1.5 and the distance to 1.

:::answer
The pulse doesn't return: its power differs from the input's by 1.55 of its peak. Only whole orders
are periodic.
:::

### The split-step method and its order {theory}

The symmetrized split-step Fourier method (Agrawal, Eq. 2.4.8) advances the dispersion half a step
in the spectrum, exactly, the Kerr phase a whole step in time, exactly ($A \mapsto Ae^{i\gamma \lvert A\rvert^2h}$), and the dispersion another half; its error per step is of third order in $h$,
so second order over a length. photonoxide measures that order: on the fundamental, second- and
third-order solitons each halving of the step divides the error by 4.00, from $1.8 \times 10^{-4}$
on 500 steps to $7.0 \times 10^{-7}$ on 8000 for the fundamental after $5z_0$.

::validation nonlinear/nlse-fundamental-soliton nonlinear/nlse-third-order-period nonlinear/nlse-split-step-order

### The dispersion from the modes {research}

$\beta_k$ come from the mode solver's effective index sampled at Chebyshev points across a band
and differentiated; silica's come out as its Sellmeier formula's Taylor series to $5.6 \times 10^{-9}$ in $\beta_2$, −27.9 ps²/km at 1.55 µm:

::validation nonlinear/nlse-dispersion-silica-beta2

The silicon nitride strip of Section 8 has $\beta_2 = -0.1547$ ps²/m for TE₀ (10 nm cells, 0.24 %
from 20 nm), and its fundamental soliton of 100 fs needs 18.1 W. In a ring of
[Pfeiffer et al. (2016)](https://doi.org/10.1364/OPTICA.3.000020)'s free spectral range, 95.65 GHz,
this straight guide gives $D_2/2\pi = 1.426$ MHz for TM₀. Their text gives 0.49 MHz measured
and 0.26 simulated for their 238.2 µm ring, but the polynomial they drew through their measured
resonances (their Fig. 5(c), read from the PDF's vector paths) is 1.90 MHz: the text and the
figure disagree by 3.9 times. Against the figure ours is 25 % low; their bend and the indices
they simulated with aren't given.

## 10. What does four-wave mixing need for gain?

Two pump photons become a signal and an idler, $2\omega_p = \omega_s + \omega_i$, with the signal
and idler at $\omega_p \pm \Omega$. The process needs momentum to balance, and a strong pump shifts
every wave's index: Agrawal's mismatch with the pump's self- and cross-phase modulation is
$\kappa = \Delta k + 2\gamma P$ (Eq. 10.2.21), $\Delta k = \beta_s + \beta_i - 2\beta_p$, and the
parametric gain (Eqs. 10.2.19 and 10.4.7) is

$$
g^2 = (\gamma P)^2 - (\kappa/2)^2, \qquad G = 1 + \left(\frac{\gamma P}{g}\sinh gL\right)^2 .
$$

A guide's dispersion gives $\Delta k = \beta_2\Omega^2 + \beta_4\Omega^4/12$ (Eq. 10.3.6), even in
$\Omega$. The gain peaks at $\kappa = 0$, $G = \cosh^2\gamma PL$, which needs $\beta_2\Omega^2 = -2\gamma P$: anomalous dispersion, at $\Omega = (2\gamma P/\lvert\beta_2\rvert)^{1/2}$. At the
chart's defaults (1 W, $\gamma = 1$ /(W·m), 2 m, $\beta_2 = -0.05$ ps²/m) the peak lies at
6.325 rad/ps, 1.007 THz or 8.07 nm from a 1.55 µm pump, and the gain there is 14.1541 (11.51 dB),
exact: no grid.

::chart nl-fwm

**Try it.** Make the dispersion normal, $\beta_2 = +0.05$; then set $\beta_2 = 0$ and
$\beta_4 = -1$.

:::answer
With normal dispersion $\kappa$ never vanishes away from the pump and the gain is a narrow peak at
Ω = 0 only. With $\beta_2 = 0$ a negative $\beta_4$ alone matches the Kerr shift where
$\beta_4\Omega^4/12 = -2\gamma P$, at $\Omega = (24\gamma P/\lvert\beta_4\rvert)^{1/4} = 22.1$ rad/ps
here, far from the pump: two narrow gain bands.
:::

The closed forms against the library's coupled equations of the three waves (RK4, fourth order) and
against the NLSE, whose one envelope holds the pump and its sidebands: 14.154107 against 14.154116 on
2000 split steps.

::validation nonlinear/fwm-gain-matched nonlinear/fwm-gain-mismatched nonlinear/fwm-nlse-gain

### Silicon and rings {research}

[Foster et al. (2006)](https://doi.org/10.1038/nature04932) engineered anomalous dispersion into
300 × 550 nm silicon guides and measured net on/off gain over 28 nm, up to 2.9 dB at 4.2 W of peak
pump in 17 mm, with two-photon and free-carrier absorption in the measurement. The letter doesn't
give the cladding, the $n_2$ it assumed or the carriers' lifetime, so it is not reproduced. A ring
resonant at all three waves multiplies each field by its enhancement $F = ik/(1 - rae^{i\phi})$,
and the idler by $\lvert F_p\rvert^4\lvert F_s\rvert^2\lvert F_i\rvert^2$
([Helt et al., 2010](https://doi.org/10.1364/OL.35.003006), Eq. 5); the library's ring run round
trip by round trip meets that closed form:

::validation nonlinear/ring-shg-enhancement nonlinear/ring-fwm-enhancement

A nonlinear job's `[task.ring]` closes its guide into the same all-pass ring, every wave on its
resonance, for SHG of type I and for four-wave mixing, and records each wave's build-up beside one
pass of the straight guide ([as a job](../docs/methods/nonlinear.md#as-a-job)):

::validation job/nonlinear-ring-shg job/nonlinear-ring-fwm

## 11. How does a ring make a Kerr comb?

A ring pumped on a resonance holds the pump, and four-wave mixing sends power into the resonances
either side, which mix again: a comb of lines one free spectral range apart. In the mean-field limit
the field around the ring obeys the Lugiato–Lefever equation, in units of half linewidths
([Kippenberg et al., 2018](https://doi.org/10.1126/science.aan8083), Eq. 3):

$$
\frac{\partial\psi}{\partial\tau} = -(1 + i\zeta_0)\psi + i\lvert\psi\rvert^2\psi
+ \frac{i}{2}\frac{\partial^2\psi}{\partial\theta^2} + f ,
$$

τ time in twice the photon lifetime, θ the angle around the ring, $\zeta_0$ the pump's detuning,
$f$ the drive, and the dispersion term a dispersion $D_2$ in place of the diffraction
[Lugiato and Lefever (1987)](https://doi.org/10.1103/PhysRevLett.58.2209) wrote for a transverse
pattern. Their uniform states satisfy $f^2 = X(1 + (X - \zeta_0)^2)$, $X = \lvert\psi\rvert^2$
(their Eq. 2): one state below $\zeta_0 = \sqrt3$, three above it on an S-shaped stretch. A pair of
lines ±μ grows from a uniform state at the rate of their Eq. 4, and with anomalous dispersion some
pair grows wherever $X > 1$: the largest rate is $X - 1$. The chart draws the states: at
$\zeta_0 = 3$ and $f^2 = 4$ they are $2 - \sqrt2$, 2 and $2 + \sqrt2$, the lower stable and the
others unstable to comb lines (exact: no grid).

::chart nl-lle-states

### The threshold of a comb {theory}

$X = 1$ is reached first at $\zeta_0 = 1$, where $f^2 = 1$. With the ring's field loss per round
trip $\alpha = \omega_0t_R/2Q_L$, its coupler's power transmission $\theta = \omega_0t_R/Q_C$ and
its guide's $\gamma$ over the round trip $L$ (Coen et al.'s normalization, below), the bus power at
threshold is

$$
P_\text{th} = \frac{\alpha^3}{\theta\gamma L} .
$$

[Pu et al. (2016)](https://doi.org/10.1364/OPTICA.3.000823) measured 3.1 mW in an AlGaAs ring of
995 GHz free spectral range, a loaded Q of about 10⁵ and $\gamma \approx 660$ /(W·m), and bracketed
it between critical coupling and $Q_C = 10Q_L$. The formula gives 0.684 to 3.419 mW, and the
measurement lies in the band; the library's instability rate confirms the onset, stable at
$f = 0.999$ and unstable at 1.001:

::example comb_pu

Pu et al.'s own formula is 1.1395 times ours: the material's index in place of the group index, and
a factor 1.54 whose source, their Ref. 29, isn't in hand. The difference between the two bands is
not explained yet.

### Dissipative Kerr solitons {theory}

[Herr et al. (2013)](https://doi.org/10.1038/nphoton.2013.343) tuned a pump from the blue side of
an MgF₂ resonance to the red: past noisy comb states, the transmission fell in discrete steps, each
a state of pulses on a weak background, dissipative Kerr solitons; a single soliton's spectrum had
a sech² envelope 1.6 THz wide at 3 dB. Without loss and drive the equation's soliton is
$\sqrt{2\zeta_0}\,\mathrm{sech}(\sqrt{2\zeta_0}\,\theta)$ in the angle, $W = (d_2/\zeta_0)^{1/2}$
wide with $d_2 = D_2/\kappa$, and the drive must make up its loss, $2\int\lvert\psi\rvert^2 = 2\operatorname{Re} \int f\psi^*$, which gives $\cos\varphi_0 = (8\zeta_0)^{1/2}/(\pi f)$ for its phase against the
drive: a root only up to $\zeta_0 = \pi^2f^2/8$. The chart settles the soliton from that
approximation by split-step on 512 lines: at $f = 3$, $d_2 = 0.004$ and $\zeta_0 = 10$ its peak is
20.74 and its width 0.03423 rad, against the approximation's 19.50 and 0.03525.

::chart nl-comb

**Try it.** Move the detuning to 8, to 11, and to 12.

:::answer
At 8 the settled peak is 17.41 against the approximation's 14.88; at 11 they nearly meet, 22.17 and
22.06. At 12 there is no soliton: π²f²/8 = 11.10. Followed up from 8, the settled soliton survives
at 11 and decays to the uniform state at 11.25.
:::

The steady states and the instability against Lugiato and Lefever's closed forms, the splitting's
second order, and [Coen et al. (2012)](https://doi.org/10.1364/OL.38.000037)'s MgF₂ resonator,
given whole (free spectral range 18.2 GHz, $\gamma = 0.032$ /(W·km), $\beta_2 = -13$ ps²/km,
critically coupled at 55.6 mW): its soliton settles at 444.4 fs, within half a unit of the paper's
one digit, "a ∼400 fs duration pulse".

::validation nonlinear/lle-homogeneous-lower nonlinear/lle-homogeneous-upper nonlinear/lle-instability-rate nonlinear/lle-split-step-order nonlinear/lle-coen-2012-duration

::example soliton_coen

**Not reproduced.** Read to two digits, Coen et al.'s 400 fs is not 444. Their peak power, "100 mW
atop a weak cw background", is not a soliton of this guide: a 400 fs soliton of $\beta_2 = -13$
ps²/km and $\gamma = 0.032$ /(W·km) needs about 7.9 kW. Ours peaks at 6.36 kW inside the ring and
0.111 W leaves through the coupler; the paper doesn't say which it plotted. Herr et al.'s 200 fs
soliton can't be computed: their coupled pump is given as a typical 5 to 30 mW and the detuning not
at all. Both stay open with the other gaps against papers.

## 12. How are photon pairs made, and how pure are they?

The same processes run from vacuum. A pump photon splits into a signal and an idler by spontaneous
parametric down-conversion in a χ⁽²⁾ guide, two pump photons become a pair by spontaneous four-wave
mixing in a χ⁽³⁾ one. Down-conversion is difference-frequency generation seeded by one photon per
mode: from the three-wave equations the signal's rate per unit of its frequency is
$dR/d\nu_s = \kappa^2(\nu_s\nu_i/\nu_p^2)P_pL^2\,\mathrm{sinc}^2(\Delta kL/2)$, at degeneracy
$\eta P_pL^2$ pairs per second per hertz. Pumped at 785 nm, Zhao et al.'s guide gives pairs whose
signal spans 55.9 nm at half maximum over 5 mm (10 nm cells): wide, because at degeneracy $\Delta k$
moves only at second order.

### The joint spectrum and its purity {theory}

A pair's two-photon state is $\iint f(\nu_s, \nu_i)\,a_s^\dagger a_i^\dagger\,d\nu_sd\nu_i \lvert0\rangle$ with the joint spectral amplitude
([Grice and Walmsley, 1997](https://doi.org/10.1103/PhysRevA.56.1627), Eqs. 7–9)

$$
f(\nu_s, \nu_i) = \alpha(\nu_s + \nu_i)\,\Phi(\nu_s, \nu_i) ,
$$

the pump's spectrum, which ties the sum of the frequencies, times the phase matching
$\Phi = \mathrm{sinc}(\Delta kL/2)$, which ties a combination set by the group indices. (Grice and
Walmsley print $\sin(\Delta kL)/(\Delta kL)$; the integral over the crystal from $-L/2$ to $L/2$
gives the half-argument form, twice as wide, which photonoxide uses.) The singular value
decomposition $f = \sum_k\sqrt{\lambda_k}\,u_k(\nu_s)v_k(\nu_i)$ is its Schmidt decomposition; a
heralded photon is pure, as interference between sources needs, when one term holds it all: the
purity is $P = \sum_k\lambda_k^2$ and the Schmidt number $K = 1/P$.

With a Gaussian pump and the Gaussian stand-in for the sinc, $\exp(-0.17897(\Delta kL/2)^2)$, and
group indices $g_p$, $g_s$, $g_i$, write the detunings $\delta_s$, $\delta_i$ in $\nu$: then
$\ln f = -p(\delta_s + \delta_i)^2 - g(A\delta_s + B\delta_i)^2$ with $A = g_p - g_s$,
$B = g_p - g_i$, $p = 2\ln2/\Delta\nu^2$ and $g = 0.17897(\pi L)^2$. It is the Gaussian
$\exp(-\tfrac12z^\mathsf TMz)$ with $M = 2[[p + gA^2,\ p + gAB],[p + gAB,\ p + gB^2]]$, whose
purity is $P = (1 - c^2/(ab))^{1/2}$ for $M$'s entries $a$, $b$ and $c$. It is 1 when $c = 0$:

$$
(g_p - g_s)(g_p - g_i) = -\frac{p}{g} < 0 ,
$$

the pump's group velocity between the signal's and the idler's, and the pump's bandwidth matched to
the length. The chart draws $\lvert f\rvert^2$ for 0.775 µm down-converted to 1.55 µm over 10 mm,
$g_s = 2.30$ and $g_p = 2.40$, and its purity by the library's decomposition on 201 × 201
frequencies: with $g_i = 2.25$ the ellipse lies along the pump's anti-diagonal, $P = 0.1904$,
$K = 5.25$, the closed form to the digits shown.

::chart nl-jsa

**Try it.** Raise the idler's group index to 2.5; then switch to the sinc's own phase matching;
then widen the pump to 2 nm.

:::answer
At $g_i = 2.5$, $A = 0.10$ and $B = -0.10$ and the ellipse turns round: $P = 0.9928$. With the sinc
the purity falls to 0.8234: the sinc's side lobes are correlated. At 2 nm the pump no longer
matches the length and the purity falls to 0.4969.
:::

The purity on a grid against the closed form, the same for spontaneous four-wave mixing (0.9464
for a 1.55 µm pump into 1.50 µm and its idler over 1 cm), and filters' heralding efficiency and the
purity they buy:

::validation nonlinear/pairs-gaussian-purity nonlinear/fwm-sfwm-purity nonlinear/pairs-heralding-gaussian nonlinear/pairs-filtered-purity

### On chip {research}

[Silverstone et al. (2013)](https://doi.org/10.1038/nphoton.2013.339) put two spontaneous
four-wave-mixing sources, 5.2 mm silicon spirals, on one chip and interfered their pairs with a
visibility of up to 100.0 ± 0.4 %. Zhao et al. measured a coincidence-to-accidental ratio above
67,000 and an entanglement visibility above 99 % from their ridge; they print no bandwidth, purity
or Schmidt number to compare with, and their brightness is Section 6's open gap. A ring's pairs,
whose resonances set the joint spectrum (Helt et al. compute Schmidt numbers from 10.00 for a 5 ns
pump to 1.09 for 5 ps), come with the lesson on
[quantum light on a chip](quantum-light.md).

## 13. How is a nonlinear guide made?

A thin film of lithium niobate starts as a bulk crystal: [Levy et al. (1998)](https://doi.org/10.1063/1.121801)
implanted 3.8 MeV helium ions, which damaged a layer about 10 µm down, etched that layer away in
dilute HF to lift off films about 9 µm thick, and bonded them to silicon or GaAs with epoxy. Wang
et al. poled a 600 nm film on insulator before etching it: chromium and gold finger electrodes by
photolithography and lift-off, several 580 V pulses 5 ms long with the sample in oil (about
7.6 kV/mm), the electrodes removed, then the ridges written inside the poled regions by aligned
electron-beam lithography and an argon-ion etch. Pfeiffer et al.'s silicon nitride rings are made
the other way round from an etched film: trenches etched into thermal silica, nitride deposited by
LPCVD in one run, the excess polished away, and an anneal that densifies the nitride and lowers
its residual hydrogen.

photonoxide's fabrication engine has no poling step yet, so no flow for a poled guide. Its
thin-film lithium niobate modulator makes the same film and ridge before its electrodes, and its
flow runs back to the design to the bit
([how a chip is made](how-a-chip-is-made.md)):

::fabrication tfln-modulator

::validation fabrication/round-trip-tfln

## 14. History and current research {research}

[Franken et al. (1961)](https://doi.org/10.1103/PhysRevLett.7.118) saw the first optical harmonic a
year after the laser: about 10¹¹ ultraviolet photons per 3 J ruby pulse in quartz.
[Armstrong et al. (1962)](https://doi.org/10.1103/PhysRev.127.1918) wrote the coupled-wave theory
the next year, and with it quasi-phase matching by inverted plates; it waited thirty years for
crystals poled finely enough, and [Fejer et al. (1992)](https://doi.org/10.1109/3.161322) set out its
design. [Miller (1964)](https://doi.org/10.1063/1.1754022)'s rule and
[Shoji et al. (1997)](https://doi.org/10.1364/JOSAB.14.002268)'s absolute scale gave the
coefficients; [Edwards and Lawrence (1984)](https://doi.org/10.1007/BF00620081),
[Zelmon et al. (1997)](https://doi.org/10.1364/JOSAB.14.003319) and
[Jundt (1997)](https://doi.org/10.1364/OL.22.001553) the indices every lithium niobate design
still takes.

On chips, [Levy et al. (1998)](https://doi.org/10.1063/1.121801)'s ion slicing began thin-film
lithium niobate, whose poled ridges reached thousands of %/(W·cm²)
([Wang et al., 2018](https://doi.org/10.1364/OPTICA.5.001438)), and GaAs on insulator more
([Chang et al., 2018](https://doi.org/10.1002/lpor.201800149)). Silicon guides gave four-wave-mixing
gain ([Foster et al., 2006](https://doi.org/10.1038/nature04932)) within the limits
[Bristow et al. (2007)](https://doi.org/10.1063/1.2737359) and
[Lin et al. (2007)](https://doi.org/10.1364/OE.15.016604) set out, and
[Afshar and Monro (2009)](https://doi.org/10.1364/OE.17.002298) a nonlinear coefficient for
high-contrast guides. [Lugiato and Lefever (1987)](https://doi.org/10.1103/PhysRevLett.58.2209)'s
equation became the model of microresonator combs
([Coen et al., 2012](https://doi.org/10.1364/OL.38.000037)), dissipative solitons were seen in them
([Herr et al., 2013](https://doi.org/10.1038/nphoton.2013.343)) and reviewed
([Kippenberg et al., 2018](https://doi.org/10.1126/science.aan8083)), in silicon nitride
([Ikeda et al., 2008](https://doi.org/10.1364/OE.16.012987); [Pfeiffer et al., 2016](https://doi.org/10.1364/OPTICA.3.000020)),
aluminium nitride ([Jung et al., 2013](https://doi.org/10.1364/OL.38.002810)) and AlGaAs
([Pu et al., 2016](https://doi.org/10.1364/OPTICA.3.000823)). Photon pairs followed the same
devices: [Grice and Walmsley (1997)](https://doi.org/10.1103/PhysRevA.56.1627)'s joint spectrum,
[Helt et al. (2010)](https://doi.org/10.1364/OL.35.003006)'s rings,
[Silverstone et al. (2013)](https://doi.org/10.1038/nphoton.2013.339)'s interfering silicon sources
and [Zhao et al. (2020)](https://doi.org/10.1103/PhysRevLett.124.163603)'s thin-film ridge. The
textbooks this lesson's equations follow are [Boyd (2020)](https://doi.org/10.1016/C2015-0-05510-1)
and [Agrawal (2019)](https://doi.org/10.1016/C2018-0-01168-8); the materials' data are
[DeSalvo et al. (1996)](https://doi.org/10.1109/3.511545),
[Vurgaftman et al. (2001)](https://doi.org/10.1063/1.1368156) and
[Leidinger et al. (2015)](https://doi.org/10.1364/OE.23.021690)'s.

::timeline

What photonoxide doesn't do yet: a biaxial crystal's index surface, focused bulk beams, a ring
resonant at its pump as a parametric oscillator, and poling in the fabrication engine
([limits](../docs/methods/nonlinear.md#limits)). A ring's photon pairs are in the quantum module
([pairs from rings](../docs/methods/quantum.md#pairs-from-rings)).

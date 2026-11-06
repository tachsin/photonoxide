---
title: "Modulators and phase shifters"
summary: "Changing a waveguide's index with heat, with an electric field or with a phase change: thermo-optic heaters and their power and time constant, the Pockels effect and Vπ·L in thin-film lithium niobate, push–pull Mach–Zehnder modulators, travelling-wave electrodes and their bandwidth, silicon's carrier modulators, and non-volatile phase-change cells, against the published devices photonoxide reproduces and the ones it doesn't."
topic: Active and quantum photonics
level: intermediate
minutes: 55
prerequisites: [mach-zehnder, ring-resonator]
examples: [thermo_jacques, thermo_harris, tfln_zhang, pcm_delaney]
jobs: [tfln-line.yaml, pcm-switching.yaml]
methods: [phase-shifter.md, electro-optic.md, travelling-wave.md, devices.md, static.md]
validation:
  - thermo/dwivedi-602
  - shifter/jacques-tin
  - shifter/jacques-npp
  - shifter/harris-p-pi
  - shifter/jacques-crosstalk-5
  - shifter/jacques-crosstalk-15
  - shifter/jacques-trench
  - shifter/jacques-tin-time
  - shifter/jacques-npp-time
  - shifter/harris-bandwidth
  - pockels/bulk-x-cut
  - pockels/overlap-gamma
  - pockels/wang-2018
  - pockels/zhang-2022
  - pockels/valdez-2023
  - line/response-walk-off
  - line/response-loss-3db
  - line/zhang-2022-index
  - line/zhang-2022-impedance
  - line/zhang-2022-loss
  - line/zhang-2022-rf-vpi-short
  - line/zhang-2022-rf-vpi-long
  - material/pcm-delaney-contrast
  - material/pcm-delaney-fom
  - fabrication/round-trip-tfln
  - fabrication/round-trip-heater
  - fabrication/pcm-reported-flows
charts: [heater-power, mzm-transfer, modulator-response, pcm-shift]
papers:
  - cite: "R. C. Alferness, IEEE Trans. Microw. Theory Tech. 30, 1121 (1982)"
    title: "Waveguide electrooptic modulators"
    doi: 10.1109/TMTT.1982.1131213
    year: 1982
    role: review
    note: "Titanium-diffused lithium niobate and GaAs modulators reviewed: a lumped electrode's RC limit and transit-time cutoff, about 2.2 GHz·cm in lithium niobate, and the travelling wave's sin x/x response to the walk-off between the light and the RF wave (his Eqs. 20 and 21), 9 GHz·cm in lithium niobate and more than 60 in GaAs."
  - cite: "R. A. Soref, B. R. Bennett, IEEE J. Quantum Electron. 23, 123 (1987)"
    title: "Electrooptical effects in silicon"
    doi: 10.1109/JQE.1987.1073206
    year: 1987
    role: origin
    note: "Silicon's index changes by a numerical Kramers–Kronig analysis of measured absorption spectra: the field's electrorefraction is small, 1.3 × 10⁻⁵ at 10⁵ V/cm, and free carriers' larger, +2.1 × 10⁻³ at 1.55 µm for 10¹⁸ holes per cm³ depleted, the basis of every silicon carrier modulator since."
  - cite: "M. Jazbinšek, M. Zgonik, Appl. Phys. B 74, 407 (2002)"
    title: "Material tensor parameters of LiNbO3 relevant for electro- and elasto-optics"
    doi: 10.1007/s003400200818
    year: 2002
    role: review
    note: "Lithium niobate's electro-optic, dielectric, elastic and photoelastic tensors fitted to the measurements in one consistent set, clamped and unclamped: r₃₃ = 31.2 and 31.45 pm/V at 633 nm, the bulk tensor photonoxide's catalogue holds."
  - cite: "G. Ghione, Semiconductor Devices for High-Speed Optoelectronics, Cambridge University Press (2009), Chapter 6"
    title: "Semiconductor Devices for High-Speed Optoelectronics"
    doi: 10.1017/CBO9780511635595
    year: 2009
    role: review
    note: "The travelling-wave modulator's response with a source and a load, Eq. (6.31), and its limits, the walk-off's |sin U/U| and the loss's (1 − e^(−αL))/αL, the equations photonoxide's electrode response computes."
  - cite: "G. T. Reed, G. Mashanovich, F. Y. Gardes, D. J. Thomson, Nat. Photonics 4, 518 (2010)"
    title: "Silicon optical modulators"
    doi: 10.1038/nphoton.2010.179
    year: 2010
    role: review
    note: "Silicon's modulators by carrier injection, accumulation and depletion, with the plasma-dispersion expressions at 1.55 µm and the figures they are compared by (bandwidth, extinction, insertion loss, energy per bit); depletion devices reached 40 Gbit/s."
  - cite: "N. C. Harris, Y. Ma, J. Mower, T. Baehr-Jones, D. Englund, M. Hochberg, C. Galland, Opt. Express 22, 10487 (2014)"
    title: "Efficient, compact and low loss thermo-optic phase shifter in silicon"
    doi: 10.1364/OE.22.010487
    year: 2014
    role: milestone
    note: "A doped heater in the guide itself, current crossing a 61.6 µm rib through periodic p++ channels: 24.77 mW for π, a −3 dB bandwidth of 130 kHz and 0.23 dB of insertion loss, measured."
  - cite: "C. Ríos, M. Stegmaier, P. Hosseini, D. Wang, T. Scherer, C. D. Wright, H. Bhaskaran, W. H. P. Pernice, Nat. Photonics 9, 725 (2015)"
    title: "Integrated all-photonic non-volatile multi-level memory"
    doi: 10.1038/nphoton.2015.182
    year: 2015
    role: milestone
    note: "10 nm GST cells under 10 nm of ITO on silicon nitride guides, switched by optical pulses: up to eight levels in one cell, switching energies down to 13.4 pJ, the states kept without power."
  - cite: "C. Wang, M. Zhang, B. Stern, M. Lipson, M. Lončar, Opt. Express 26, 1547 (2018)"
    title: "Nanophotonic lithium niobate electro-optic modulators"
    doi: 10.1364/OE.26.001547
    year: 2018
    role: current
    note: "Etched thin-film lithium niobate ribs between electrodes 3.5 µm apart: a 2 mm Mach–Zehnder modulator with a DC V_π of 9 V, 1.8 V·cm, the cross-section photonoxide's Pockels chain is checked on."
  - cite: "C. Wang, M. Zhang, X. Chen, M. Bertrand, A. Shams-Ansari, S. Chandrasekhar, P. Winzer, M. Lončar, Nature 562, 101 (2018)"
    title: "Integrated lithium niobate electro-optic modulators operating at CMOS-compatible voltages"
    doi: 10.1038/s41586-018-0551-y
    year: 2018
    role: milestone
    note: "Travelling-wave thin-film lithium niobate Mach–Zehnder modulators 5 to 20 mm long: 1.4 V at 20 mm with more than 45 GHz, 2.3 and 2.2 V·cm with 80 and 100 GHz at 10 and 5 mm, 30 dB of extinction, under 0.5 dB on chip, and data at up to 210 Gbit/s."
  - cite: "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019)"
    title: "Optimization of thermo-optic phase-shifter design and mitigation of thermal crosstalk on the SOI platform"
    doi: 10.1364/OE.27.010456
    year: 2019
    role: current
    note: "TiN and N++ silicon heaters on one SOI die, simulated and measured: P_π, the 1/e time and their product as the figure of merit, and the crosstalk on a neighbour through the oxide, its etches and deep trenches."
  - cite: "M. He, M. Xu, Y. Ren, J. Jian, Z. Ruan, Y. Xu, S. Gao, S. Sun, X. Wen, L. Zhou, L. Liu, C. Guo, H. Chen, S. Yu, L. Liu, X. Cai, Nat. Photonics 13, 359 (2019)"
    title: "High-performance hybrid silicon and lithium niobate Mach–Zehnder modulators for 100 Gbit s−1 and beyond"
    doi: 10.1038/s41566-019-0378-6
    year: 2019
    role: current
    note: "A thin lithium niobate film bonded onto a silicon circuit, its arms etched into it: 2.2 V·cm push–pull, at least 70 GHz, 2.5 dB of insertion loss, 100 Gbit/s on–off keying."
  - cite: "M. Delaney, I. Zeimpekis, D. Lawson, D. W. Hewak, O. L. Muskens, Adv. Funct. Mater. 30, 2002447 (2020)"
    title: "A new family of ultralow loss reversible phase-change materials for photonic integrated circuits: Sb2S3 and Sb2Se3"
    doi: 10.1002/adfm.202002447
    year: 2020
    role: milestone
    note: "Sb₂S₃ and Sb₂Se₃, transparent at 1550 nm in both states with contrasts of 0.60 and 0.77, on silicon waveguides, and a figure of merit of phase per loss, 29 rad/dB for Sb₂Se₃, against GST's larger contrast and loss."
  - cite: "Y. Zhang, L. Shao, J. Yang, Z. Chen, K. Zhang, K.-M. Shum, D. Zhu, C. H. Chan, M. Lončar, C. Wang, Photon. Res. 10, 2380 (2022)"
    title: "Systematic investigation of millimeter-wave optic modulation performance in thin-film lithium niobate"
    doi: 10.1364/PRJ.468518
    year: 2022
    role: current
    note: "Thin-film lithium niobate modulators measured to 325 GHz with their line modelled and measured: 2.3 V·cm at DC, 3 dB bandwidths of 170 and 100 GHz at 5.8 and 10.8 mm, an RF V_π of 7.3 V at 250 GHz."
  - cite: "F. Valdez, V. Mere, S. Mookherjea, Optica 10, 578 (2023)"
    title: "100 GHz bandwidth, 1 volt integrated electro-optic Mach–Zehnder modulator at near-IR wavelengths"
    doi: 10.1364/OPTICA.484549
    year: 2023
    role: current
    note: "An unetched 200 nm lithium niobate film bonded over a silicon nitride strip, at 784 nm: 0.8 V·cm push–pull and 100 GHz."
  - cite: "J. Holzgrafe, E. Puma, R. Cheng, H. Warner, A. Shams-Ansari, R. Shankar, M. Lončar, Opt. Express 32, 3619 (2024)"
    title: "Relaxation of the electro-optic response in thin-film lithium niobate modulators"
    doi: 10.1364/OE.507536
    year: 2024
    role: current
    note: "Thin-film lithium niobate's response measured from 1 mHz to 1 MHz: it relaxes on millisecond scales in cladded devices and is enhanced between 1 and 20 kHz, which can confound a V_π measured at low frequency."
  - cite: "D. Chelladurai, M. Kohli, J. Winiger, D. Moor, A. Messner, Y. Fedoryshyn, M. Eleraky, Y. Liu, H. Wang, J. Leuthold, Nat. Mater. 24, 868 (2025)"
    title: "Barium titanate and lithium niobate permittivity and Pockels coefficients from megahertz to sub-terahertz frequencies"
    doi: 10.1038/s41563-025-02158-1
    year: 2025
    role: current
    note: "Thin-film lithium niobate's r₃₃ measured from 100 MHz to 330 GHz at 1550 nm: 26.9 pm/V and flat, the coefficient photonoxide's modulator cases take."
---

The two ways the library changes a guide's phase, in the symbols the equations below use. Above,
a push–pull Mach–Zehnder modulator from above: its arms in the gaps of a travelling-wave electrode
$L$ long, the RF wave of index $n_m$ beside light of group index $n_o$, the field opposite in the
two gaps. Below, the same in section in thin-film lithium niobate, and a heater $h$ above a
silicon strip.

::diagram modulator

A phase shifter changes the effective index of a length of guide, and with it the phase the light
leaves with. Heat does it through the thermo-optic coefficient $dn/dT$, an electric field in a
crystal without inversion symmetry through the Pockels tensor, free carriers in silicon through
plasma dispersion, and a film that changes phase through the contrast between its states. Put in
one arm of a Mach–Zehnder interferometer or in a ring, a phase becomes an intensity
([Reed et al., 2010](https://doi.org/10.1038/nphoton.2010.179)).

## 1. What do phase shifters and modulators do, and how are they compared?

A section of length $L$ whose effective index moves by $\Delta n_\mathrm{eff}$ adds the phase

$$
\Delta\varphi = \frac{2\pi}{\lambda}\,\Delta n_\mathrm{eff}\,L ,
$$

so a phase of $\pi$ takes $L_\pi = \lambda/(2\lvert\Delta n_\mathrm{eff}\rvert)$. What makes
$\Delta n_\mathrm{eff}$ names each device's figure:

- a **heater**: the power for $\pi$, $P_\pi$ (mW), and the 1/e time $\tau$ of its phase;
- a **Pockels modulator**: the voltage for $\pi$ times the length, $V_\pi L$ (V·cm), since
  $\Delta n_\mathrm{eff}$ is proportional to the voltage and $V_\pi$ falls as $1/L$;
- a **phase-change cell**: $\Delta n_\mathrm{eff}$ between its two states and the loss of the
  lossier one, as $L_\pi$ and phase per loss (rad/dB).

A modulator is compared by four more ([Reed et al., 2010](https://doi.org/10.1038/nphoton.2010.179),
"Performance metrics"): its **bandwidth**, the frequency at which the modulation has fallen to
half (the optical definition) or by 3 dB electrically; its **extinction ratio**,
$10\log_{10}(I_\mathrm{max}/I_\mathrm{min})$; its **insertion loss**, the light it costs in its
on state; and its **energy per bit**. For a lumped electrode of capacitance $C$ driven between 0
and $V$, charging it draws $CV^2$ from the supply, of which $\tfrac12CV^2$ is stored and
$\tfrac12CV^2$ lost in the resistance, and the stored half is lost on discharge; random data rise
from 0 to 1 on a quarter of the bits, so the energy is $CV^2/4$ per bit. Reed et al. give
10 fJ/bit as the modulator's share of an on-chip link's budget.

photonoxide computes the figures of three kinds of device, each with its fidelities, its grids and
the published devices it is checked against: thermo-optic phase shifters
([the phase shifter](../docs/methods/phase-shifter.md)), a thin-film lithium niobate Mach–Zehnder
modulator ([the Pockels effect](../docs/methods/electro-optic.md),
[travelling-wave electrodes](../docs/methods/travelling-wave.md)) and phase-change cells
([the devices](../docs/methods/devices.md)). Each is a device of the component library:

::device thermo-optic-shifter

## 2. How does heat shift a waveguide's phase, and what power does π take?

Silicon's index rises with temperature by $dn/dT = 1.8 \times 10^{-4}$ per kelvin at 300 K, the
value Jacques et al. take, about 18 times silica's 1.0 × 10⁻⁵ (their Section 2). A guide's
effective index follows its materials' through its mode: photonoxide's 602 × 211 nm wire in silica gives
$dn_\mathrm{eff}/dT = 1.925 \times 10^{-4}$ per kelvin against the $2.005 \times 10^{-4}$ Dwivedi
et al. measured (10 nm grid):

::validation thermo/dwivedi-602

### Pπ is independent of the length {theory}

A heater dissipating $P'$ per unit length along a straight section raises the temperature of the
cross-section by $\Delta T(x, y) = P'\,\Delta T_1(x, y)$, $\Delta T_1$ the rise per W/m: steady
heat conduction is linear. First-order perturbation gives the mode's index change as a weighted
sum of the rise over the cross-section, $\Delta n_\mathrm{eff} = \sum_c w_c\,\Delta T(x_c, y_c)$,
each cell's weight its share of the index change a 1 K rise there makes
([the phase shifter](../docs/methods/phase-shifter.md)). So $\Delta n_\mathrm{eff} = sP'$ with
$s = \partial n_\mathrm{eff}/\partial P'$ (m/W), and with $P = P'L$ the phase is

$$
\Delta\varphi = \frac{2\pi}{\lambda}\,s\,\frac{P}{L}\,L = \frac{2\pi}{\lambda}\,sP,
\qquad P_\pi = \frac{\lambda}{2s} .
$$

The length cancels: a longer heater spreads the same power more thinly over more guide
([Jacques et al., 2019](https://doi.org/10.1364/OE.27.010456), Section 2.1). $P_\pi$ is set by
the cross-section alone, how much of the heat stays near the mode before it reaches the substrate.
It is a 2D number: the heat leaving along the guide, through the leads and at the ends isn't in it.

The chart draws a TiN heater 7.5 µm wide, Jacques et al.'s fabricated width, above a 500 × 220 nm
strip on 2 µm of buried oxide, by the device's closed forms (the oxide a slab between an
isothermal substrate and an insulated top, exact: no grid). $P_\pi$ rises from 25.75 mW at a
height of 1 µm to 29.85 mW at 2 µm and 34.30 mW at 3 µm: the higher the heater, the more of its
heat spreads sideways before it reaches the guide.

::chart heater-power

**Try it.** Switch to the doped heaters, two N++ strips in the slab 0.8 µm beside a rib. What do
$P_\pi$ and $P_\pi\tau$ become?

:::answer
27.61 mW and 167 nJ, against the TiN heater's 29.85 mW and 333 nJ: the silicon slab carries the
heat across to the rib, and there is less oxide to heat. $P_\pi$ climbs from 22.38 mW at 0.4 µm to
39.31 mW at 1.6 µm, 17 mW over 1.2 µm against the TiN heater's 8.6 mW over 2 µm of height.
:::

### What the heaters' cross-sections reproduce {research}

Jacques et al. simulated two heaters 320 µm long on a 40 × 18 µm region: a TiN
heater 2 µm wide 2 µm above the strip at 23.9 mW, and N++ strips beside a rib at 25.5 mW.
photonoxide's chain, the static solver's heat on grids from 10 nm and the full-vector mode's
weights on 5 nm cells, gives 25.3 mW (the midpoint of the four variants of what the paper doesn't
give, the TiN's thickness and the oxide above it) and 26.1 mW. Harris et al.'s heater in the
guide, periodic along it, is averaged into one section: 26.4 mW against their measured 24.77 mW
([Harris et al., 2014](https://doi.org/10.1364/OE.22.010487)).

::validation shifter/jacques-tin shifter/jacques-npp shifter/harris-p-pi

[thermo_jacques](../examples/thermo_jacques.rs) runs the two heater devices at Jacques et al.'s
fabricated designs (the TiN heater 7.5 µm wide) by each fidelity, against the paper's simulations
and measurements:

::example thermo_jacques

The 2D model's 33.36 mW for the TiN heater is 14 % above the 29.2 mW measured, as the paper's own
simulation of that heater, 32.2 mW, is 10 % above it: both are a cross-section's, not the made
device's.

### How much of the heat reaches a neighbour {research}

A neighbouring guide takes a share $X_{ij} = s_{ij}/s_{jj}$ of the phase heater $j$ gives its own
guide. Jacques et al. measured a passive Mach–Zehnder beside their N++ heater: 0.141 of its phase
at a gap of 5 µm and 0.0435 at 15 µm. photonoxide's cross-section of the chip, the substrate
725 µm thick and ±2 mm wide, gives 0.183 and 0.0582, 30 % and 34 % above: the cross-section has no
length, and the measured victim's far arm is warmed too, and both lower the measured share. A deep
trench between them, through the buried oxide into the substrate, divides the crosstalk by 3.48
against the measured 3.13.

::validation shifter/jacques-crosstalk-5 shifter/jacques-crosstalk-15 shifter/jacques-trench

## 3. How fast does a heater's phase follow its power?

Only as fast as heat flows from the heater to the guide and on to the substrate. Section 2's chart
draws the 1/e time of the phase turned on beside $P_\pi$: for the TiN heater 11.2 µs at a height
of 2 µm, from 7.21 µs at 1 µm to 15.9 µs at 3 µm, the heat having further to go and more oxide to
warm (exact: no grid).

### A first-order thermal response {theory}

Lump the cross-section into one thermal resistance per unit length $R'$ (K·m/W) between the guide
and the substrate and one heat capacity per unit length $C'$ (J/(K·m)). The rise $\Delta T$ obeys

$$
C'\,\frac{d\Delta T}{dt} = P' - \frac{\Delta T}{R'} ,
$$

and for a power switched on at $t = 0$, $\Delta T(t) = P'R'\left(1 - e^{-t/\tau}\right)$ with
$\tau = R'C'$. Driven at a frequency, $\Delta\hat T/\hat P' = R'/(1 + i\omega\tau)$, which falls by
3 dB at $f_{3\,\mathrm{dB}} = 1/(2\pi\tau)$. With $s = (dn_\mathrm{eff}/dT)R'$ in this model,
$P_\pi = \lambda/(2\,(dn_\mathrm{eff}/dT)\,R')$, and

$$
P_\pi\,\tau = \frac{\lambda\,C'}{2\,dn_\mathrm{eff}/dT} :
$$

the thermal resistance cancels. A heater placed to need less power keeps its heat in longer; only
less heat capacity to warm, or a larger $dn_\mathrm{eff}/dT$, lowers the product. Jacques et al.
write the same model with a conductance $G$ over an area $A$ and a heat capacity $H$:
$P_\pi = \Delta T_\pi GA$ and $\tau = H/GA$, so $P_\pi\tau = \Delta T_\pi H$, their figure of merit,
to be minimized (their Eqs. 1 to 3).

A real cross-section is not one pole: its response has many, the slowest the substrate's. The chart's last figure, $2\pi f_{3\,\mathrm{dB}}\tau$, would be 1 for a single
pole. The closed forms give 1.07 for the TiN heater (11.2 µs, 15.22 kHz) and 0.761 for the doped
heaters (6.06 µs, 19.99 kHz): exact, no grid. A single pole rises from 10 to 90 % in
$\tau\ln 9 = 2.2\tau$; the N++ heater's phase, by the static solver in time, takes 16.6 µs, 3.9
times its 1/e time of 4.25 µs: the substrate's slow tail
([the phase shifter](../docs/methods/phase-shifter.md)).

### Against the papers {research}

From the switch, Jacques et al.'s simulated phases cross 1/e at 9.0 µs (TiN, the 2 µm heater) and
4.6 µs (N++); photonoxide's static solver, stepping the heat implicitly, gives 9.47 and 4.25 µs
(BDF2 steps of 0.2 and 0.1 µs). Their measured times are 3.6 µs below their own simulations, which
they put down to what the cross-section doesn't hold. Harris et al.'s measured −3 dB frequency,
130 kHz, is above the averaged section's 109 kHz by 19 % (ours 16 % below theirs).

::validation shifter/jacques-tin-time shifter/jacques-npp-time shifter/harris-bandwidth

Harris et al.'s heater runs its current across the rib itself, through p silicon, so the heat is
made in the guide: 0.93 µs and 109.3 kHz on the 2D model's own grid, against the TiN heater's
11.34 µs above its strip. [thermo_harris](../examples/thermo_harris.rs) prints each fidelity:

::example thermo_harris

::device in-guide-thermo-optic-shifter

## 4. How does an electric field change a crystal's index?

In a crystal without a centre of symmetry, a static field $E$ changes the optical
impermeability $\eta = \varepsilon^{-1}$ in proportion to it, the linear electro-optic or Pockels
effect: $\Delta\eta_{ij} = \sum_k r_{ijk}E_k$, $r$ the electro-optic tensor
([the Pockels effect](../docs/methods/electro-optic.md)). Lithium niobate's largest element is
$r_{33}$, which couples a field along the crystal's $c$ axis to light polarized along it, of
index $n_e$: 31.2 pm/V clamped in the bulk at 633 nm
([Jazbinšek and Zgonik, 2002](https://doi.org/10.1007/s003400200818), Tables 4 and 5), and
26.9 pm/V in thin films at 1550 nm, flat from 100 MHz to 330 GHz
([Chelladurai et al., 2025](https://doi.org/10.1038/s41563-025-02158-1)).

### The index change, Δn = −½n³rE {theory}

On a principal axis the impermeability is $1/n^2$. A field $E$ along $c$ changes it by
$\Delta(1/n_e^2) = r_{33}E$. Since $d(1/n^2) = -2\,dn/n^3$, to first order

$$
\Delta n_e = -\tfrac12\,n_e^3\,r_{33}\,E .
$$

At 1.55 µm $n_e$ = 2.1376 (Zelmon et al.'s fit, as the catalogue gives it), so 1 V/µm and the thin
film's $r_{33}$ move $n_e$ by $1.31 \times 10^{-4}$ (exact: no grid). photonoxide turns the whole
tensor into the chip's axes by the cut, $r'_{abc} = \sum M_{ai}M_{bj}M_{ck}r_{ijk}$, and takes
$\Delta\varepsilon = -\varepsilon\,\Delta\eta\,\varepsilon$, which reduces to this on a principal
axis: within $3.6 \times 10^{-6}$ for x-cut, second order in the field.

::validation pockels/bulk-x-cut

**The cut.** In **x-cut** lithium niobate the $c$ axis lies in the film's plane; a guide running
across it carries its TE mode, the field in the plane, along $c$, and electrodes beside the guide
put their field along $c$ too: $r_{33}$ for both. In z-cut the $c$ axis is normal to the film, and
$r_{33}$ needs the TM mode and a vertical field. The thin-film devices of this lesson are x-cut.

## 5. What does Vπ·L measure, and why do papers quote it differently?

The field the electrodes put across the guide isn't uniform, and the mode isn't all in the crystal.
The guided mode's index moves by the overlap of the two, first-order perturbation:

$$
\Delta n_\mathrm{eff} = \frac{\int E^*\cdot\Delta\varepsilon\cdot E\,dA}{2\,\mathrm{Re}\int (E\times H^*)\cdot\hat z\,dA},
$$

with $\Delta\varepsilon$ from the static solver's field in each cell
([the Pockels effect](../docs/methods/electro-optic.md)). It is proportional to the voltage, so a
section of length $L$ at a voltage $V$ adds $(2\pi/\lambda)(dn_\mathrm{eff}/dV)VL$. On one arm a
phase of $\pi$ takes

$$
V_\pi L = \frac{\lambda}{2\,\lvert dn_\mathrm{eff}/dV\rvert} ,
$$

and in push and pull, a ground–signal–ground line with an arm in each gap and the fields in them
opposite, the arms move apart by twice that:

$$
V_\pi L = \frac{\lambda}{4\,\lvert dn_\mathrm{eff}/dV\rvert} .
$$

Both are a straight section's at DC: 2D numbers, per unit length.

### Why the published numbers differ {theory}

Five things change a quoted $V_\pi L$ for the same device:

- **Push–pull or one arm**: a factor of 2.
- **The normalization.** The thin-film papers write
   $V_\pi L = \lambda g/(2n_e^3r_{33}\Gamma)$ in push–pull, $g$ the gap and $\Gamma$ the overlap of
   the field with $\lvert E\rvert^2$ normalized by $\int\lvert E\rvert^2$, not by the power. With
   the power, $\mathrm{Re}(E\times H^*)\cdot\hat z \approx n_\mathrm{eff}\lvert E\rvert^2$, the
   perturbation gives $V_\pi L \approx \lambda g\,n_\mathrm{eff}/(2n_e^4r_{33}\Gamma)$, about 10 %
   below the papers' form on a thin-film rib ($n_\mathrm{eff}/n_e \approx 1.93/2.14$). On Wang et
   al.'s cross-section $\Gamma$ = 0.51, and the scaled form is within 0.8 % of the perturbation
   (10 nm grid).
- **The coefficient.** The bulk's 633 nm coefficients overstate the film's at 1550 nm: with them
   Wang et al.'s and Zhang et al.'s cross-sections give 1.56 and 2.02 V·cm instead of 1.84 and 2.37.
- **The frequency.** Zhang et al. quote 2.3 V·cm at DC and 2.67 at 1 GHz, the line's impedance
   between them. Below about 1 MHz a thin film's response relaxes, and between 1 and 20 kHz it can
   be enhanced, which can confound a $V_\pi$ measured slowly
   ([Holzgrafe et al., 2024](https://doi.org/10.1364/OE.507536)).
- **The length.** An electrode's ends and bends, which a cross-section doesn't hold.

::validation pockels/overlap-gamma

### What the Pockels cross-sections reproduce {research}

photonoxide's chain, the static solver's field and the full-vector mode's perturbation, each grid
refined to 10 nm and gated at 0.5 %, gives 1.839 V·cm on
[Wang et al. (2018)](https://doi.org/10.1364/OE.26.001547)'s 2 mm modulator against their 1.8,
2.372 V·cm on [Zhang et al. (2022)](https://doi.org/10.1364/PRJ.468518)'s against 2.3 (both
measured at DC), and 0.844 V·cm on [Valdez et al. (2023)](https://doi.org/10.1364/OPTICA.484549)'s
hybrid of an unetched film over silicon nitride at 784 nm against 0.8 (grids to 5 nm).

::validation pockels/wang-2018 pockels/zhang-2022 pockels/valdez-2023

[He et al. (2019)](https://doi.org/10.1038/s41566-019-0378-6)'s hybrid silicon and lithium niobate
modulator is not reproduced: its cross-section gives 2.85 V·cm push–pull against the 2.2 and
2.5 V·cm they measured at 100 kHz on 3 and 5 mm devices. No structural, permittivity or clamping
change tried closes it; only an $r_{33}$ of 30.7 to 34.9 pm/V does, above the film's measured
26.9, and the two lengths' products also differ from each other by 15 %. Below 1 MHz a thin
film's response changes in bands that depend on its layers (Holzgrafe et al.), and He et al.'s
film, on a polymer bond and uncladded, may have one near their 100 kHz; until their $V_\pi$ is
known against frequency it stays open
([the Pockels effect](../docs/methods/electro-optic.md), "He et al. 2019").

## 6. How does a push–pull Mach–Zehnder turn phase into intensity?

Between two ideal 3 dB splitters, the arms' phase difference $\Delta\varphi$ sends a share
$\cos^2(\Delta\varphi/2)$ of the light to one output
([the Mach–Zehnder interferometer](mach-zehnder.md)). Driven in push and pull at a voltage $V$
with a bias $\varphi_0$,

$$
\Delta\varphi = \pi\,\frac{V\,\lvert m(f)\rvert}{V_\pi} + \varphi_0, \qquad
T = \cos^2\frac{\Delta\varphi}{2},
$$

$m(f)$ the electrode's response at the drive's frequency, 1 at DC (Section 7). The transmission's
slope, $dT/dV = -(\pi\lvert m\rvert/2V_\pi)\sin\varphi_0$, is steepest at quadrature,
$\varphi_0 = \pm\pi/2$, where a small signal is passed on linearly; a heater on one arm can set
$\varphi_0$. The light the combiner can't take is radiated, so the extinction ratio is set by
how evenly the splitters divide: Wang et al.'s 20 mm modulator reached 30 dB
([Wang et al., 2018](https://doi.org/10.1038/s41586-018-0551-y), Methods).

The chart is photonoxide's TFLN modulator at Zhang et al.'s cross-section, its component's
transmission against the voltage by the 2D model: $V_\pi L$ = 2.372 V·cm, so $V_\pi$ = 4.090 V for
5.8 mm, and at quadrature $dT/dV$ = −0.384 per volt.

::chart mzm-transfer

**Try it.** Set the frequency to 50 GHz. How far does the swing narrow, and what does
$V_\pi$ become? Then set the length to 10.8 mm at DC.

:::answer
At 50 GHz the 5.8 mm electrode's $\lvert m\rvert$ is 0.7536, so $V_\pi$ = 5.426 V and the slope
−0.289 per volt: the dashed DC curve swings faster. At 10.8 mm and DC $V_\pi$ = 2.196 V and the
slope −0.715 per volt, the same $V_\pi L$ over a longer electrode.
:::

A ring turns a small index change into a large change of transmission near its resonance, at the
price of a narrow optical band and a resonance that moves with temperature
([the ring resonator](ring-resonator.md); Reed et al., 2010).

::device tfln-modulator

## 7. What limits a travelling-wave modulator's speed?

A lumped electrode is a capacitor charged through the source's resistance, its bandwidth
$1/(\pi RC)$ falling as its length grows; once it is long enough for the light's transit to
matter, the voltage changes while the light crosses it, a cutoff of about 2.2 GHz·cm in bulk
lithium niobate ([Alferness, 1982](https://doi.org/10.1109/TMTT.1982.1131213)). A
**travelling-wave** electrode is a transmission line along the arms, fed at one end and ended in a
load, the RF wave travelling with the light. Three things then limit it ([Ghione, 2009](https://doi.org/10.1017/CBO9780511635595),
Section 6.5): the **walk-off** between the RF wave's index $n_m$ and the light's group index $n_o$,
the line's **loss**, and its **impedance** against the source's and the load's.

### The walk-off and the loss {theory}

On a matched lossless line the voltage the light sees at $z$, entering at $t_0$, is
$V\cos(\omega t_0 + \omega(n_o - n_m)z/c)$: it rides the RF wave, slipping behind or ahead. The
phase it gathers is the integral over the length, whose amplitude, against the matched case's, is

$$
\left\lvert \frac{1}{L}\int_0^L e^{i\omega(n_o - n_m)z/c}\,dz \right\rvert
= \left\lvert\frac{\sin U}{U}\right\rvert, \qquad U = \frac{\omega(n_m - n_o)L}{2c}
$$

(Alferness's Eq. 20; Ghione's Section 6.5.2). It falls to $1/\sqrt2$, 3 dB electrically, at
$U$ = 1.39, so $f_{3\,\mathrm{dB}}L = 1.39\,c/(\pi\lvert n_m - n_o\rvert)$, and to half, the optical
definition, at $U$ = 1.895. Alferness's 9 GHz·cm for bulk lithium niobate's half-modulation
point is this with $\lvert n_m - n_o\rvert$ = 2.0: an RF wave far slower than the light.
On a matched line at the light's speed whose loss $\alpha$ grows as $\sqrt f$, the response is
$(1 - e^{-\alpha L})/(\alpha L)$, falling by 3 dB where the line loses 6.39 dB over its length
(Ghione). photonoxide's response, Ghione's Eq. (6.31) with any source, load, index and loss, meets
both limits:

::validation line/response-walk-off line/response-loss-3db

In a thin film the light is held in the submicron rib while the RF field lies mostly in the
substrate, so the stack's thicknesses set $n_m$ apart from the light: Wang et al. matched it to the
light's group index ([Wang et al., 2018](https://doi.org/10.1038/s41586-018-0551-y)). The chart is Ghione's Eq. (6.31)
for Zhang et al.'s fit to their measured line: $0.42\sqrt f + 0.025f$ dB/cm (f in GHz), 41.5 Ω
between 50 Ω ends, its index 0.04 above the light's 2.26, 5.8 mm long. It falls by 3 dB at
127.7 GHz from 1 GHz; the walk-off alone would allow 571.5 GHz and the loss alone 214.8 GHz (exact:
no grid). The line's early fall and its ripples are the 41.5 Ω line's reflections between its
50 Ω ends.

::chart modulator-response

**Try it.** Match the line to the ends: set the impedance to 50 Ω. Then set the length to 10.8 mm
with the impedance back at 41.5 Ω.

:::answer
186.1 GHz matched, against 127.7: the reflections at the 41.5 Ω line's two ends cost the rest. At
10.8 mm the 3 dB bandwidth falls to 61.6 GHz, the loss's limit to 94.0 GHz: loss, not walk-off,
limits a thin-film electrode this long.
:::

A walk-off of 2.0, the bulk crystal's by Alferness's number, on a matched lossless line 10 mm
long: from 1 GHz, the response falls by 3 dB at 6.69 GHz and by 6 dB at 9.07 GHz.

::chart modulator-response{mismatch=2, loss=0, dielectric=0, impedance=50, length=10mm}

## 8. What does photonoxide's line reproduce of a measured modulator?

photonoxide models the electrode as a quasi-TEM line from the static solver's cross-section: its
capacitances with and without the dielectrics give $n_m = \sqrt{C/C_0}$ and
$Z_0 = 1/(c\sqrt{CC_0})$, each medium's share of the electric energy weighs its loss tangent, and
the metal's loss comes from the current on the strips (Holloway and Kuester's method)
([travelling-wave electrodes](../docs/methods/travelling-wave.md)). On Zhang et al.'s line, graded
from 2 nm, it gives an index of 2.259 at 100 GHz against their simulated 2.27, 41.1 Ω at 250 GHz
against their extracted 41.5 Ω, and 15.2 dB/cm against the 16 dB/cm their Fig. 3(b) extracts there.
Their RF $V_\pi$ at 250 GHz, from their 2.67 V·cm at 1 GHz and the line's response, is 8.16 and
6.15 V at 5.8 and 10.8 mm against their measured 8.3 and 7.3 V, the second 16 % low inside the
measured points' ±15 % spread and the case's 20 %.

::validation line/zhang-2022-index line/zhang-2022-impedance line/zhang-2022-loss line/zhang-2022-rf-vpi-short line/zhang-2022-rf-vpi-long

Their 3 dB bandwidths are not reproduced. The TFLN modulator device's 2D model, at their design,
falls by 3 dB at 91.7 and 32.6 GHz for 5.8 and 10.8 mm against their measured 170 and 100 GHz;
the full-wave RF mode of the same cross-section lowers them to 84 and 31 GHz. Their own fitted line
in Section 7's chart falls at 127.7 and 61.6 GHz, also below. Ours falls fastest below 30 GHz,
where the conductor loss on 0.83 µm of copper, thinner than two skin depths below 25 GHz, is
3.0 dB/cm at 10 GHz against their fit's 1.3; their 1 GHz reference, which already holds
2.67 V·cm against 2.3 at DC, may hold part of the fall. It stays open
([travelling-wave electrodes](../docs/methods/travelling-wave.md), "Their bandwidths aren't
reproduced"). [tfln_zhang](../examples/tfln_zhang.rs) prints the device at both lengths by each
fidelity:

::example tfln_zhang

`jobs/tfln-line.yaml` solves the same line as a job, quasi-TEM and full-wave, with its grid study.

## 9. How do free carriers modulate silicon?

Silicon is centrosymmetric, so it has no Pockels effect, and its Kerr and Franz–Keldysh effects
are weak at 1.3 and 1.55 µm (Reed et al., 2010); heat is too slow for data. Free carriers change
both its index and its absorption, the **plasma dispersion effect**.
[Soref and Bennett (1987)](https://doi.org/10.1109/JQE.1987.1073206) found the index changes from
measured absorption spectra by a Kramers–Kronig analysis: depleting $10^{18}$ holes per cm³ raises
the index by $2.1 \times 10^{-3}$ at 1.55 µm, and the change grows as $\Delta N^{0.818}$ for holes
and $\Delta N^{1.04}$ for electrons; injecting carriers lowers it. At 1.55 µm Reed et al. give
their result as

$$
\Delta n = -\left[8.8 \times 10^{-22}\,\Delta N_e + 8.5 \times 10^{-18}\,(\Delta N_h)^{0.8}\right],
\qquad
\Delta\alpha = 8.5 \times 10^{-18}\,\Delta N_e + 6.0 \times 10^{-18}\,\Delta N_h
$$

(per cm³ and per cm), so that $5 \times 10^{17}$ cm⁻³ of each moves the index by
$-1.66 \times 10^{-3}$, with absorption that a phase modulator can't avoid. Three structures change
the carriers in a guide (Reed et al., Fig. 1): **injection** through a forward-biased p–i–n diode,
efficient but limited by the minority carriers' lifetime; **accumulation** at a thin oxide
inside the guide, a capacitor; and **depletion** of a reverse-biased p–n junction across the guide,
fast but less efficient. Depletion modulators passed 40 Gbit/s at 1 dB of extinction by 2010.

photonoxide does not compute carrier modulators yet: it has no drift-diffusion solver for the
junction's carriers and no plasma-dispersion material model. Both, with the junction's
capacitance and travelling-wave lines loaded by it, are [the roadmap](../ROADMAP.md)'s 0.8.

## 10. How do phase-change materials hold a phase without power?

A chalcogenide such as Ge₂Sb₂Te₅ (GST), Sb₂S₃ or Sb₂Se₃ has two stable solid states, amorphous and
crystalline, with different indices. Heating it above its crystallization temperature for long
enough crystallizes it; melting and quenching it fast amorphizes it. Either state then stays
without power, for years at operating temperatures
([Ríos et al., 2015](https://doi.org/10.1038/nphoton.2015.182)): a phase shifter that holds its
setting. Ríos et al. switched 10 nm GST cells on silicon nitride guides between up to eight levels
with optical pulses, from 13.4 pJ. `jobs/pcm-switching.yaml` heats an Sb₂Se₃ cell on a silicon
strip with a 100 ns pulse and reports how hot the film gets and whether it melts.

GST's contrast at 1550 nm is $\Delta n$ = 3.56, but it absorbs strongly when crystalline; Sb₂S₃'s
is 0.60 and Sb₂Se₃'s 0.77, and both are transparent in both states
([Delaney et al., 2020](https://doi.org/10.1002/adfm.202002447)). Delaney et al. compare cells by
the phase per loss, $2\pi\,\Delta n_\mathrm{eff}/(\lambda\alpha)$ in rad/dB (their Eq. 1), $\alpha$
the loss of the lossier state. photonoxide's fits to their ellipsometry give 0.764 and 0.590 for the
two:

::validation material/pcm-delaney-contrast

### The cells' index change {theory}

The device puts a film on a 500 × 220 nm silicon strip; its analytic model is the effective index
method with exact complex slabs, its 2D model the full-vector mode solver on 5 nm cells. At 25 nm
the analytic $\Delta n_\mathrm{eff}$ is 0.0533 (Sb₂Se₃), 0.0296 (Sb₂S₃) and 0.802 (GST), so $L_\pi$
is 14.53, 26.21 and 0.966 µm; crystalline GST loses 16.5 dB/µm, a figure of merit of 0.198 rad/dB.
The 2D model gives 0.0568, 0.0318 and 0.817. The index change grows with the film, more than ten
times faster for GST, whose crystalline film pulls the mode into itself.

::chart pcm-shift

**Try it.** Thin the films to 10 nm, Ríos et al.'s GST thickness. What happens to GST's figure of
merit and to Sb₂Se₃'s $L_\pi$?

:::answer
GST's figure of merit rises a little, to 0.223 rad/dB, its loss falling to 3.73 dB/µm with
$\Delta n_\mathrm{eff}$ 0.205; Sb₂Se₃'s $L_\pi$ grows to 38.56 µm, its $\Delta n_\mathrm{eff}$
0.0201. A thin film costs length, not phase per loss.
:::

### Against Delaney et al. {research}

Our fits absorb almost nothing in Sb₂Se₃ and Sb₂S₃, so the cells' figures of merit (342 rad/dB for
Sb₂Se₃) are the materials' and not comparable with Delaney et al.'s 29.0 and 5.15 rad/dB, which
rest on the 0.01 and 0.028 dB/µm they measured in their cells. Their Eq. 1 with their own
$\Delta n_\mathrm{eff}$ and losses gives 30.3 and 5.01 rad/dB.

::validation material/pcm-delaney-fom

Their $\Delta n_\mathrm{eff}$ of 0.0717 and 0.0356 for a 25 nm film on a rib is not reproduced: the
paper gives neither the rib's etch, the film's extent over the slab nor the polarization, and on a
strip we get 0.0568 and 0.0318 (2D), 21 % and 11 % below theirs, with no single geometry giving
both. [pcm_delaney](../examples/pcm_delaney.rs) prints each cell by each fidelity:

::example pcm_delaney

::device sb2se3-phase-shifter

## 11. How do the technologies compare?

Published devices, each with the figure its paper measured, and what photonoxide gives for it:

| Technology | Device (paper) | Measured | Speed | Loss | photonoxide |
|---|---|---|---|---|---|
| heater, TiN | 7.5 µm wide (Jacques et al., 2019) | $P_\pi$ 29.2 mW | 7.2 µs, 33 kHz | — | 33.36 mW, 11.34 µs (2D) |
| heater in the guide | doped rib, 61.6 µm (Harris et al., 2014) | $P_\pi$ 24.77 mW | 130 kHz | 0.23 dB | 26.37 mW, 109 kHz (2D) |
| Pockels, thin film | 5 mm (Wang et al., Nature 2018) | 2.2 V·cm | 100 GHz | under 0.5 dB | no case: gap not given |
| Pockels, thin film | 5.8 mm (Zhang et al., 2022) | 2.3 V·cm, DC | 170 GHz | — | 2.372 V·cm, 91.7 GHz (2D) |
| Pockels on silicon | 3 mm (He et al., 2019) | 2.2 V·cm, 100 kHz | ≥ 70 GHz | 2.5 dB | 2.85 V·cm: not reproduced |
| carriers, depletion | p–n MZI (Reed et al., 2010) | 6.5 V drive | 40 Gbit/s | — | none yet (0.8) |
| phase change, Sb₂Se₃ | 25 nm film (Delaney et al., 2020) | $\Delta n_\mathrm{eff}$ 0.0717 | — | 0.01 dB/µm | 0.0568 on a strip (2D) |
| phase change, GST | 10 nm cell (Ríos et al., 2015) | eight levels | switched from 13.4 pJ | — | 25 nm: 0.817, 16.5 dB/µm (2D) |

A heater's phase follows its power in microseconds and lasts only while it is powered, for tens
of milliwatts whatever its length. A thin-film lithium niobate modulator takes millimetres and a
few volts and reaches the highest bandwidths in the table. A silicon carrier modulator absorbs as
it shifts. A phase-change cell takes microns, holds its state without power and is switched by
pulses of heat or light.

## 12. How are a heater and a thin-film lithium niobate modulator made?

As the chip lesson's strip is ([How a chip is made](how-a-chip-is-made.md)), with layers added.
The TiN heater's flow etches the strip in an SOI wafer's silicon down to the buried oxide, covers
it with deposited oxide polished flat to the heater's height, deposits and patterns the titanium
nitride, and covers it again; the heater's place over its guide must hold within the
lithography's overlay. The contacts, vias and pads that carry its current are listed as a step
and not drawn, since the design draws the heater alone. The flow played below gives back the device's structure
and films to the bit:

::fabrication thermo-optic-shifter

The modulator's flow starts from a lithium niobate on insulator wafer, a thin crystal film bonded
on oxide over silicon, etches the ribs part-way into the film (a timed etch: there is no stop layer
inside the crystal), clads them in deposited oxide and lays the electrodes on the slab: Zhang et
al.'s 800 nm of copper under 30 nm of gold (their Section 3.A). The flow contains the steps they
report, in order:

::fabrication tfln-modulator

::validation fabrication/round-trip-heater fabrication/round-trip-tfln fabrication/pcm-reported-flows

## 13. History and current research {research}

[Alferness (1982)](https://doi.org/10.1109/TMTT.1982.1131213) reviewed the first waveguide
modulators, titanium diffused into lithium niobate, whose weak index contrast kept the electrodes
far from the light; the travelling-wave electrode's walk-off set their bandwidth-length product.
[Soref and Bennett (1987)](https://doi.org/10.1109/JQE.1987.1073206) gave silicon's carrier index
changes, and the injection, accumulation and depletion modulators built on them are reviewed by
[Reed et al. (2010)](https://doi.org/10.1038/nphoton.2010.179). [Harris et al. (2014)](https://doi.org/10.1364/OE.22.010487) put the heater in the
guide, and [Jacques et al. (2019)](https://doi.org/10.1364/OE.27.010456) compared metal and doped
heaters and their crosstalk on one die. Etched thin films of lithium niobate on insulator brought
the electrodes to microns from the guides:
[Wang et al. (2018)](https://doi.org/10.1038/s41586-018-0551-y) reached CMOS voltages with
bandwidths to 100 GHz. Phase-change cells added a phase that holds without power, GST's in
[Ríos et al. (2015)](https://doi.org/10.1038/nphoton.2015.182) and the transparent Sb₂S₃ and Sb₂Se₃
in [Delaney et al. (2020)](https://doi.org/10.1002/adfm.202002447).

::timeline

Current work measures the thin film's coefficient and response:
[Chelladurai et al. (2025)](https://doi.org/10.1038/s41563-025-02158-1) found $r_{33}$ flat from
100 MHz to 330 GHz, [Holzgrafe et al. (2024)](https://doi.org/10.1364/OE.507536) the relaxation
below 1 MHz, and [Zhang et al. (2022)](https://doi.org/10.1364/PRJ.468518) modulators to 325 GHz.
Hybrids bond the film onto silicon or silicon nitride circuits:
[He et al. (2019)](https://doi.org/10.1038/s41566-019-0378-6) on silicon, and
[Valdez et al. (2023)](https://doi.org/10.1364/OPTICA.484549) over nitride at 784 nm, at 0.8 V·cm.

Further reading: [the phase shifter](../docs/methods/phase-shifter.md) and
[the static solver](../docs/methods/static.md) for the heat;
[the Pockels effect](../docs/methods/electro-optic.md) and
[travelling-wave electrodes](../docs/methods/travelling-wave.md) for the modulator;
[the devices](../docs/methods/devices.md) for the three device families and their fidelities; and
[the Mach–Zehnder interferometer](mach-zehnder.md) and [the ring resonator](ring-resonator.md), the
circuits a phase shifter turns into a switch or a modulator.

---
title: "The Mach–Zehnder interferometer"
summary: "Split the light, send it down two paths, bring it back together: the phase between the arms decides which output it leaves by. Fringes a free spectral range apart, set by the group index; their depth, set by the couplers' balance; and the switches, filters and modulators built on it."
topic: Interferometers
level: introductory
minutes: 30
prerequisites: []
examples: [mzi_dwivedi, circuit_splitter]
jobs: [mzi-mmi.toml]
circuits: [mzi.toml]
methods: [components.md, circuits.md, circuit-adjoint.md]
validation:
  - components/mzi-closed-form
  - components/mzi-unitarity
  - circuit/mzi-closed-form
  - circuit/adjoint-mzi
  - circuit/adjoint-mesh
  - components/mzi-neff-dwivedi-470
  - components/mzi-ng-dwivedi-470
  - components/mzi-neff-dwivedi-602
  - components/mzi-ng-dwivedi-602
  - components/mzi-neff-dwivedi-805
  - components/mzi-ng-dwivedi-805
charts: [mzi-spectrum, mzi-extinction]
papers:
  - cite: "R. C. Alferness, IEEE Trans. Microw. Theory Tech. 30, 1121 (1982)"
    title: "Waveguide Electrooptic Modulators"
    doi: 10.1109/TMTT.1982.1131213
    year: 1982
    role: review
    note: "Integrated-optic modulators in titanium-diffused lithium niobate, the interferometers among them: light split in two, the arms' phases pushed apart by an applied field, and recombined."
  - cite: "R. A. Soref, B. R. Bennett, IEEE J. Quantum Electron. 23, 123 (1987)"
    title: "Electrooptical effects in silicon"
    doi: 10.1109/JQE.1987.1073206
    year: 1987
    role: milestone
    note: "How much silicon's index changes with an applied field and with free carriers: the carriers' effect is the larger, the one silicon modulators came to use."
  - cite: "N. Takato, T. Kominato, A. Sugita, K. Jinguji, H. Toba, M. Kawachi, IEEE J. Sel. Areas Commun. 8, 1120 (1990)"
    title: "Silica-based integrated optic Mach-Zehnder multi/demultiplexer family with channel spacing of 0.01-250 nm"
    doi: 10.1109/49.57816
    year: 1990
    role: milestone
    note: "Mach–Zehnder interferometers in silica waveguides on silicon as wavelength multiplexers, their channel spacing set by the arms' path difference across four orders of magnitude."
  - cite: "M. Kawachi, Opt. Quantum Electron. 22, 391 (1990)"
    title: "Silica waveguides on silicon and their application to integrated-optic components"
    doi: 10.1007/BF02113964
    year: 1990
    role: review
    note: "Silica waveguides on silicon, the planar lightwave circuits, and the integrated-optic components made from them."
  - cite: "M. Reck, A. Zeilinger, H. J. Bernstein, P. Bertani, Phys. Rev. Lett. 73, 58 (1994)"
    title: "Experimental realization of any discrete unitary operator"
    doi: 10.1103/PhysRevLett.73.58
    year: 1994
    role: milestone
    note: "Any unitary transformation of N modes built from a triangle of beam splitters and phase shifters: the idea behind meshes of Mach–Zehnder interferometers."
  - cite: "K. Jinguji, M. Kawachi, J. Lightwave Technol. 13, 73 (1995)"
    title: "Synthesis of coherent two-port lattice-form optical delay-line circuit"
    doi: 10.1109/50.350643
    year: 1995
    role: milestone
    note: "Mach–Zehnder interferometers cascaded into lattice filters, with thermo-optic heaters as phase shifters: any two-port filter response a finite impulse response can give, synthesized stage by stage."
  - cite: "M. Born, E. Wolf, Principles of Optics, 7th ed., Cambridge University Press (1999)"
    title: "Principles of Optics"
    doi: 10.1017/CBO9781139644181
    year: 1999
    role: review
    note: "The classical theory of interference and of the two-beam interferometers, the Mach–Zehnder among them."
  - cite: "E. L. Wooten, K. M. Kissa, A. Yi-Yan, E. J. Murphy, D. A. Lafaw, P. F. Hallemeier, D. Maack, D. V. Attanasio, D. J. Fritz, G. J. McBrien, D. E. Bossi, IEEE J. Sel. Top. Quantum Electron. 6, 69 (2000)"
    title: "A review of lithium niobate modulators for fiber-optic communications systems"
    doi: 10.1109/2944.826874
    year: 2000
    role: review
    note: "Lithium niobate Mach–Zehnder modulators as the fibre links of 2000 used them, at 2.5, 10 and 40 Gb/s: their electrodes, chirp, bias and reliability."
  - cite: "A. Liu, R. Jones, L. Liao, D. Samara-Rubio, D. Rubin, O. Cohen, R. Nicolaescu, M. Paniccia, Nature 427, 615 (2004)"
    title: "A high-speed silicon optical modulator based on a metal–oxide–semiconductor capacitor"
    doi: 10.1038/nature02310
    year: 2004
    role: milestone
    note: "A silicon Mach–Zehnder modulator past a gigahertz, its arms' phases moved by the charge on metal–oxide–semiconductor capacitors."
  - cite: "G. T. Reed, G. Mashanovich, F. Y. Gardes, D. J. Thomson, Nat. Photonics 4, 518 (2010)"
    title: "Silicon optical modulators"
    doi: 10.1038/nphoton.2010.179
    year: 2010
    role: review
    note: "Silicon's modulators surveyed: how free carriers move its index, in interferometers and rings, and how fast the devices had become."
  - cite: "W. Bogaerts, P. De Heyn, T. Van Vaerenbergh, K. De Vos, S. Kumar Selvaraja, T. Claes, P. Dumon, P. Bienstman, D. Van Thourhout, R. Baets, Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue)"
    title: "Silicon microring resonators"
    doi: 10.1002/lpor.201100017
    year: 2011
    role: review
    note: "The ideal coupler's convention, through r and across ik with r² + k² = 1, that photonoxide's couplers follow, set out for rings."
  - cite: "F. Horst, W. M. J. Green, S. Assefa, S. M. Shank, Y. A. Vlasov, B. J. Offrein, Opt. Express 21, 11652 (2013)"
    title: "Cascaded Mach-Zehnder wavelength filters in silicon photonics for low loss and flat pass-band WDM (de-)multiplexing"
    doi: 10.1364/OE.21.011652
    year: 2013
    role: milestone
    note: "One-to-eight wavelength demultiplexers from a binary tree of Mach–Zehnder lattice filters in a 90 nm CMOS silicon photonics process: flat pass bands and under 1.6 dB of loss."
  - cite: "S. Dwivedi, A. Ruocco, M. Vanslembrouck, T. Spuesens, P. Bienstman, P. Dumon, T. Van Vaerenbergh, W. Bogaerts, J. Lightwave Technol. 33, 4471 (2015)"
    title: "Experimental Extraction of Effective Refractive Index and Thermo-Optic Coefficients of Silicon-on-Insulator Waveguides Using Interferometers"
    doi: 10.1109/JLT.2015.2476603
    year: 2015
    role: milestone
    note: "Three Mach–Zehnder interferometers per silicon wire: two of low order fix the effective index without ambiguity, one of high order gives the group index from its free spectral range."
  - cite: "W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walmsley, Optica 3, 1460 (2016)"
    title: "Optimal design for universal multiport interferometers"
    doi: 10.1364/OPTICA.3.001460
    year: 2016
    role: milestone
    note: "A rectangular mesh of Mach–Zehnder interferometers that makes any unitary with half the depth of Reck's triangle, and so tolerates loss better."
  - cite: "C. Wang, M. Zhang, X. Chen, M. Bertrand, A. Shams-Ansari, S. Chandrasekhar, P. Winzer, M. Lončar, Nature 562, 101 (2018)"
    title: "Integrated lithium niobate electro-optic modulators operating at CMOS-compatible voltages"
    doi: 10.1038/s41586-018-0551-y
    year: 2018
    role: current
    note: "Mach–Zehnder modulators in thin-film lithium niobate: a few volts or less to switch, tens of gigahertz to 100 GHz of bandwidth, and under half a decibel of loss on the chip."
  - cite: "W. Bogaerts, D. Pérez, J. Capmany, D. A. B. Miller, J. Poon, D. Englund, F. Morichetti, A. Melloni, Nature 586, 207 (2020)"
    title: "Programmable photonic circuits"
    doi: 10.1038/s41586-020-2764-0
    year: 2020
    role: review
    note: "Meshes of tunable 2 × 2 gates, most often Mach–Zehnder interferometers with two phase shifters, programmed by software for routing, filtering and matrix operations."
  - cite: "D. Pérez-López, A. López, P. DasMahapatra, J. Capmany, Nat. Commun. 11, 6359 (2020)"
    title: "Multipurpose self-configuration of programmable photonic circuits"
    doi: 10.1038/s41467-020-19608-w
    year: 2020
    role: current
    note: "A programmable hexagonal mesh that characterizes itself, routes its light and configures itself by computational optimization."
---

A splitter, two arms and a combiner, seen from above, in the symbols of the equations below. The
3D view is `jobs/mzi-mmi.toml`: two 1 × 2 MMIs joined by arms whose centre lines differ by 6 µm.

::diagram mzi

A Mach–Zehnder interferometer splits light between two arms and recombines it. The phase
difference $\Delta\phi$ between the arms decides which output the light leaves by. A path
difference $\Delta L$ makes $\Delta\phi$ change with wavelength, which makes a filter; a phase
shifter changes it at a fixed wavelength, which makes a switch or a modulator.

## 1. The ideal interferometer: phase and transmission

Each coupler keeps a share $r$ of the field in its guide and crosses a share $k$ over, with
$r^2 + k^2 = 1$ and a phase of $i$ on the crossing field, the convention of
[Bogaerts et al. (2011)](https://doi.org/10.1002/lpor.201100017) that photonoxide's couplers
follow. On the basis (lower guide, upper guide) a coupler is
$C = \begin{pmatrix} r & ik \cr ik & r \end{pmatrix}$. An arm multiplies its field by
$a e^{i\phi}$: $a$ its amplitude after loss, $\phi = 2\pi n_\text{eff} L/\lambda$ its phase. The
interferometer is the product

$$
M = C_2 \begin{pmatrix} a_l e^{i\phi_l} & 0 \cr 0 & a_u e^{i\phi_u} \end{pmatrix} C_1 .
$$

Light entering the lower guide leaves the bar (lower) output with $M_{11}$ and the cross (upper)
output with $M_{21}$:

$$
M_{11} = r_1 r_2\thinspace a_l e^{i\phi_l} - k_1 k_2\thinspace a_u e^{i\phi_u}, \qquad
M_{21} = i\left(r_1 k_2\thinspace a_l e^{i\phi_l} + k_1 r_2\thinspace a_u e^{i\phi_u}\right).
$$

Each output adds one path through each arm, so only $\Delta\phi = \phi_u - \phi_l$ matters:

$$
T_\text{bar} = (r_1 r_2 a_l)^2 + (k_1 k_2 a_u)^2 - 2 r_1 r_2 k_1 k_2\thinspace a_l a_u \cos\Delta\phi ,
$$

$$
T_\text{cross} = (r_1 k_2 a_l)^2 + (k_1 r_2 a_u)^2 + 2 r_1 r_2 k_1 k_2\thinspace a_l a_u \cos\Delta\phi .
$$

Even couplers, $r^2 = k^2 = \tfrac{1}{2}$, and lossless arms give

$$
T_\text{cross} = \cos^2\frac{\Delta\phi}{2}, \qquad T_\text{bar} = \sin^2\frac{\Delta\phi}{2} .
$$

At $\Delta\phi = 0$ all the light crosses over; at $\Delta\phi = \pi$ all of it stays in the bar
output. A 1 × 2 combiner (a Y-branch or an MMI) has one output carrying
$(a_l e^{i\phi_l} + a_u e^{i\phi_u})/2$, which is $\cos^2(\Delta\phi/2)$ when lossless; the
difference field excites a mode the output guide can't carry and radiates.

In the chart, cross (solid) and bar alternate across the spectrum; the dashed line is their sum,
below 1 by the loss both arms share.

::chart mzi-spectrum

## 2. Group index and the free spectral range {theory}

With the upper arm longer by $\Delta L$ and a phase shifter adding $\varphi$ to it,

$$
\Delta\phi(\lambda) = \frac{2\pi n_\text{eff}(\lambda)\thinspace\Delta L}{\lambda} + \varphi .
$$

The cross output peaks at $\Delta\phi = 2\pi m$; with $\varphi = 0$ that is
$n_\text{eff}\Delta L = m\lambda$, Eq. 3 of
[Dwivedi et al. (2015)](https://doi.org/10.1109/JLT.2015.2476603), $m$ the order. Differentiating,

$$
\frac{d\Delta\phi}{d\lambda} = -\frac{2\pi\Delta L}{\lambda^2}\left(n_\text{eff} - \lambda\frac{dn_\text{eff}}{d\lambda}\right) = -\frac{2\pi n_g \Delta L}{\lambda^2},
\qquad n_g = n_\text{eff} - \lambda\frac{dn_\text{eff}}{d\lambda} .
$$

One turn of $\Delta\phi$ separates neighbouring peaks, so

$$
\text{FSR} = \frac{\lambda^2}{n_g \Delta L} ,
$$

Dwivedi et al.'s Eq. 8. The group index sets the spacing because $n_\text{eff}$ changes with
$\lambda$ as well as $1/\lambda$. The chart's wire has $n_\text{eff} = 2.4$ and $n_g = 4.2$ at
1.55 µm, so $\lambda^2/(n_\text{eff}\Delta L)$ overestimates the spacing by a factor of 1.75.

Between neighbouring peaks $\lambda_1 < \lambda_2$ the order drops by one,
$n_\text{eff}(\lambda_1)\Delta L/\lambda_1 - n_\text{eff}(\lambda_2)\Delta L/\lambda_2 = 1$. If $n_g$
is constant between them, $n_\text{eff}/\lambda = n_g/\lambda + \text{const}$, and exactly

$$
n_g = \frac{\lambda_1 \lambda_2}{(\lambda_2 - \lambda_1)\Delta L} .
$$

In the first chart: "Free spectral range, between peaks" is measured on the computed spectrum and
differs from $\lambda^2/(n_g\Delta L)$ only because $\lambda_1\lambda_2 \ne \lambda^2$; "$n_g$ read
from the peaks" returns the $n_g$ slider's value; the phase shifter slides the fringes without
changing their spacing.

## 3. Unbalanced couplers and extinction {theory}

Each output swings between its two path amplitudes $A$ and $B$ adding, $(A + B)^2$, and
cancelling, $(A - B)^2$. For the bar output $A = r_1 r_2 a_l$ and $B = k_1 k_2 a_u$:

$$
\text{ER}_\text{bar} = \left(\frac{r_1 r_2 a_l + k_1 k_2 a_u}{r_1 r_2 a_l - k_1 k_2 a_u}\right)^2 ,
$$

and for the cross output $A = r_1 k_2 a_l$, $B = k_1 r_2 a_u$. An output goes dark only when
$A = B$. With equal arms and an even combiner, a splitter $\kappa_1^2 = \tfrac{1}{2} + \varepsilon$
gives $r_1 \approx (1 - \varepsilon)/\sqrt 2$ and $k_1 \approx (1 + \varepsilon)/\sqrt 2$, so

$$
\text{ER} \approx \frac{1}{\varepsilon^2} :
$$

a 55:45 splitter limits the extinction to 26 dB, and 30 dB needs $|\varepsilon| \lesssim 0.03$.
Two equal couplers of any $\kappa^2$ keep the cross output's paths equal ($r k a$ and $k r a$), so
it still goes dark, with a peak of $4\kappa^2(1 - \kappa^2)$; the bar output's minimum is
$(1 - 2\kappa^2)^2$.

In the chart, the cross (solid) and bar (dashed) curves coincide for an even combiner and peak
where the dark condition holds; move the combiner away from 0.5 and the two peaks separate.

::chart mzi-extinction

## 4. Loss in the arms {theory}

An arm of length $L$ losing $\alpha$ dB/cm has $a = 10^{-\alpha L/20}$, so the longer arm loses
$\alpha\Delta L$ dB more. With even couplers and $\rho = a_u/a_l$,

$$
\text{ER} = \left(\frac{1 + \rho}{1 - \rho}\right)^2 .
$$

The chart's default, 3 dB/cm over $\Delta L$ = 50 µm, is 0.015 dB, $\rho$ = 0.9983, an extinction
of about 61 dB. At 100 dB/cm over 2 mm the difference is 20 dB, $\rho$ = 0.1, and the extinction
1.7 dB. A splitter that sends more light into the lossier arm restores the balance: with an even
combiner, $r_1 a_l = k_1 a_u$ gives

$$
\kappa_1^2 = \frac{a_l^2}{a_l^2 + a_u^2} ,
$$

0.50086 for the default and 0.990 for the 20 dB case (the second chart's "Bar dark at" figure).
The loss both arms share, here 100 µm of wire, lowers both outputs equally and leaves the
extinction alone.

## 5. The interferometer as a modulator and a switch {theory}

A heater over a length $L_h$ adds $\varphi = 2\pi (dn_\text{eff}/dT)\Delta T\thinspace L_h/\lambda$.
Silicon's thermo-optic coefficient is 1.86 × 10⁻⁴ per kelvin at 1550 nm, which moves a silicon
filter by about 80 pm per kelvin ([Dwivedi et al., 2015](https://doi.org/10.1109/JLT.2015.2476603));
taking it for the mode's, a π shift over 100 µm needs $\Delta T = \lambda/(2 L_h\thinspace dn/dT)$,
about 42 K.

An electro-optic (Pockels) crystal gives $\varphi = \pi V/V_\pi$, and the cross output follows

$$
T_\text{cross} = \cos^2\left(\frac{\pi V}{2V_\pi} + \frac{\Delta\phi_0}{2}\right),
$$

$\Delta\phi_0$ the phase difference at no voltage. Biased at quadrature, $\Delta\phi_0 = \pi/2$,
the slope is steepest and the response to a small voltage nearly linear.
[Wang et al. (2018)](https://doi.org/10.1038/s41586-018-0551-y) measured thin-film lithium niobate
modulators: with 20 mm electrodes $V_\pi$ = 1.4 V, a 3 dB bandwidth above 45 GHz, an extinction
of about 30 dB and under 0.5 dB of loss on the chip; with 5 mm, 100 GHz at 4.4 V. Silicon has no
Pockels effect; free carriers change its index
([Soref and Bennett, 1987](https://doi.org/10.1109/JQE.1987.1073206)) and also absorb, so its
modulators unbalance the arms' amplitudes as they modulate
([Reed et al., 2010](https://doi.org/10.1038/nphoton.2010.179)).

With even couplers and a phase between them, an interferometer is a tunable splitter from all bar
to all cross, the usual gate of programmable meshes, each gate losing about 0.05 to 0.2 dB
([Bogaerts et al., 2020](https://doi.org/10.1038/s41586-020-2764-0)).
[circuit_splitter](../examples/circuit_splitter.rs) tunes such a gate by the circuit adjoint's
gradient until 30 % of the light stays in the bar port, at $2\arcsin\sqrt{0.3}$ from
$T_\text{bar} = \sin^2(\Delta\phi/2)$:

::example circuit_splitter

photonoxide's `PhaseShifter` is ideal: $e^{i\varphi}$ at every wavelength, lossless. The heater
and the Pockels modulator come in photonoxide 0.6, the silicon carrier modulator in 0.8.

## 6. Measuring the group index (Dwivedi et al.)

A silicon wire's index depends steeply on its cross-section: an interferometer's fringe moves by
about 1 nm per nanometre of width and 1.4 nm per nanometre of thickness
([Dwivedi et al., 2015](https://doi.org/10.1109/JLT.2015.2476603)). A fringe's spacing gives
$n_g$; its position gives $n_\text{eff}$ only once the order $m$ is known.

Dwivedi and colleagues used three interferometers per wire, between 1 × 2 MMIs. Two of low
order, $m$ = 15 and 16, have fringes far enough apart that fabrication can't shift them by a
whole order (their Fig. 1 puts the limit at orders 17, 27 and 38 for wires 450, 600 and 800 nm
wide), so one peak gives $n_\text{eff}$ by Eq. 3. One of order 110 has many fringes across the
band, and their spacing gives $n_g$ by Eq. 8. Measured at several temperatures, the same
interferometers give the thermo-optic coefficient, $dn/dT = (n_g/\lambda)\thinspace d\lambda/dT$
(their Eq. 13).

For the wire drawn 450 nm wide, `mzi_dwivedi` designs the order-110 interferometer with
$\Delta L$ = 73.06 µm; the paper measured that wire's $n_g$ as 4.2739 (Table I). With those
values the chart's fringes are 7.7 nm apart (its $n_\text{eff}$ stays at 2.4, so the order isn't
theirs):

::chart mzi-spectrum{delta=73.06um, group_index=4.27, window=30nm}

[mzi_dwivedi](../examples/mzi_dwivedi.rs) predicts their Table I from nothing measured but the
SEM cross-sections: each wire a rectangle in photonoxide's silicon and silica, its mode by
Hadley's equations on grids of about 10 nm, the interferometers built from ideal splitters and
their spectra read by Eqs. 3, 8 and 10. Five of the six numbers agree within 0.012; the 470 nm
wire's $n_g$ is 0.041 low, inside the tolerance of the paper's own fabrication estimate (its
Eq. 5); its sloped, rough sidewalls are straight in the model.

::validation components/mzi-neff-dwivedi-470 components/mzi-ng-dwivedi-470 components/mzi-neff-dwivedi-602 components/mzi-ng-dwivedi-602 components/mzi-neff-dwivedi-805 components/mzi-ng-dwivedi-805

::example mzi_dwivedi

## 7. History and current research {research}

Ludwig Zehnder described an interference refractometer in 1891, and Ludwig Mach, Ernst Mach's
son, another in 1892, in the Zeitschrift für Instrumentenkunde (11, 275 and 12, 89; neither has
a DOI): two mirrors and two half-silvered plates separate the beams widely enough to put a
sample, a gas say, in one of them and measure its index against the other.
[Born and Wolf](https://doi.org/10.1017/CBO9781139644181) describe it among the classical
two-beam interferometers.

Waveguides diffused into lithium niobate carried the arms of the first integrated versions, the
phase pushed by electrodes through the Pockels effect:
[Alferness's 1982 review](https://doi.org/10.1109/TMTT.1982.1131213) includes Martin's Y-branch
interferometer of 1975 and the balanced bridge of Ramaswamy, Divino and Standley of 1978. By 2000
lithium niobate Mach–Zehnder modulators met the needs of 2.5, 10 and 40 Gb/s fibre links
([Wooten et al., 2000](https://doi.org/10.1109/2944.826874)). In silica waveguides on silicon
([Kawachi, 1990](https://doi.org/10.1007/BF02113964)),
[Takato et al. (1990)](https://doi.org/10.1109/49.57816) made interferometric multiplexers with
channel spacings from 0.01 to 250 nm, and
[Jinguji and Kawachi (1995)](https://doi.org/10.1109/50.350643) synthesized lattice filters from
cascaded interferometers, their phase shifters thermo-optic heaters.
[Liu et al. (2004)](https://doi.org/10.1038/nature02310) modulated a silicon interferometer past a
gigahertz with metal–oxide–semiconductor capacitors in its arms.
[Reck et al. (1994)](https://doi.org/10.1103/PhysRevLett.73.58) showed that a triangle of beam
splitters and phase shifters makes any unitary, and
[Clements et al. (2016)](https://doi.org/10.1364/OPTICA.3.001460) a rectangle with half the depth.

::timeline

Current work has three lines. Modulators: thin-film lithium niobate combines low voltage, high
bandwidth and low loss ([Wang et al., 2018](https://doi.org/10.1038/s41586-018-0551-y)). Filters:
cascaded interferometers give flat pass bands, as in the eight-channel silicon demultiplexers of
[Horst et al. (2013)](https://doi.org/10.1364/OE.21.011652), under 1.6 dB of loss on about
500 × 400 µm. Programmable meshes: software-set gates for routing, filtering and matrix
operations ([Bogaerts et al., 2020](https://doi.org/10.1038/s41586-020-2764-0)), configured by
optimization on the chip ([Pérez-López et al., 2020](https://doi.org/10.1038/s41467-020-19608-w)).
Open problems: couplers that stay even across a band and a wafer, heaters that don't warm their
neighbours, and meshes that calibrate themselves as they grow.

## 8. How photonoxide computes it

The interferometer is a circuit of closed-form components with exact derivatives
([the components](../docs/methods/components.md)):

- `Coupler`: through $\sqrt{1 - \kappa^2}$, across $i\kappa$; `DirectionalCoupler` from a
  coupled pair's supermodes; `YBranch` and `Mmi` for 1 × 2 splitters.
- `Waveguide`: $e^{i\gamma L}$ from the mode's $n_\text{eff}$, $n_g$ and dispersion at a
  wavelength, and its loss, set by hand or found by the full-vector mode solver.
- `PhaseShifter`: $e^{i\varphi}$.
- `mzi` and `mzi_y`: the interferometer as a netlist, solved by
  [the circuit solve](../docs/methods/circuits.md); `mzi_closed_form`, the product of transfer
  matrices of section 1, which checks it.

The first chart solves that netlist, with a phase shifter on the upper arm, at every wavelength;
the second evaluates the closed form. The Chip page opens the same interferometer from
`circuits/mzi.toml`, and [the circuit adjoint](../docs/methods/circuit-adjoint.md) gives the
gradient of any output with respect to every coupler, length and phase. `jobs/mzi-mmi.toml` is a
structure job: it draws the interferometer and solves nothing. The netlist matches the closed
form, conserves power without loss, and its gradients match finite differences, for one
interferometer and for a 4 × 4 mesh of six:

::validation components/mzi-closed-form components/mzi-unitarity circuit/mzi-closed-form circuit/adjoint-mzi circuit/adjoint-mesh

## 9. Try it

**Spacing.** In the first chart, double the path difference, then raise the group index. What
narrows the fringes, and what only slides them?

:::answer
Both narrow them: the free spectral range is $\lambda^2/(n_g\Delta L)$, so doubling $\Delta L$
halves it. The
"$\lambda^2/(n_\text{eff}\Delta L)$" figure never matches the measured spacing. The phase shifter
adds the same phase at every wavelength, so it only slides the fringes.
:::

**Even enough.** Set the splitter to 0.55, then find how close to 0.5 it must be for 30 dB of bar
extinction, Wang et al.'s figure.

:::answer
0.55 gives about 26 dB. $\text{ER} \approx 1/\varepsilon^2 = 1000$ needs
$|\varepsilon| \lesssim 0.03$: between about 0.47 and 0.53.
:::

**Equal uneven couplers.** Set both couplers to 0.3. Which output still goes dark?

:::answer
The cross output, whose paths $r_1 k_2 a_l$ and $k_1 r_2 a_u$ are equal for equal couplers; its peak
is $4 \times 0.3 \times 0.7 = 0.84$. The bar output bottoms out at $(1 - 0.6)^2 = 0.16$, an
extinction of 8 dB.
:::

**A lossy arm.** Set 100 dB/cm and a 2 mm path difference, then find the splitter that restores
the fringes in the second chart.

:::answer
The upper arm's field is a tenth of the lower's and the extinction falls to 1.7 dB. Both outputs
go dark again at $\kappa_1^2 = a_l^2/(a_l^2 + a_u^2) \approx 0.99$.
:::

**Working point.** With even couplers, where is the cross output most sensitive to the phase?

:::answer
At quadrature, $\pm\pi/2$ from a peak, where $\cos^2(\Delta\phi/2)$ is at half height and steepest.
A shift of π swaps the outputs.
:::

## 10. Further reading {research}

- [Born and Wolf](https://doi.org/10.1017/CBO9781139644181): two-beam interference and the
  classical interferometers.
- [Dwivedi et al. (2015)](https://doi.org/10.1109/JLT.2015.2476603): $n_\text{eff}$, $n_g$ and
  $dn/dT$ measured with interferometers.
- [Wooten et al. (2000)](https://doi.org/10.1109/2944.826874),
  [Reed et al. (2010)](https://doi.org/10.1038/nphoton.2010.179) and
  [Wang et al. (2018)](https://doi.org/10.1038/s41586-018-0551-y): lithium niobate, silicon and
  thin-film lithium niobate modulators.
- [Jinguji and Kawachi (1995)](https://doi.org/10.1109/50.350643): filters synthesized from
  cascaded interferometers.
- [Bogaerts et al. (2020)](https://doi.org/10.1038/s41586-020-2764-0) and
  [Clements et al. (2016)](https://doi.org/10.1364/OPTICA.3.001460): programmable meshes.
- [The components](../docs/methods/components.md), and the lesson on the
  [ring resonator](ring-resonator.md), where the second coupler is replaced by a loop back to the
  first.

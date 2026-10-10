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

Here is the device, seen from above, labelled with the symbols this lesson's equations use: a
splitter, two arms of different length, one with a phase shifter on it, and a combiner, with the
input and the two outputs. The 3D view is an interferometer as a job builds it, from two 1 × 2
splitters.

::diagram mzi

A Mach–Zehnder interferometer divides light between two paths and brings it back together.
Where the two halves meet again in step, they add; where they meet out of step, they cancel. So
everything depends on one number, the difference in phase the arms give the light, and anything
that changes that difference, a longer arm, a warmer arm, a voltage across one arm, moves the
light from one output to the other. With arms of different length the difference changes with
wavelength, and the interferometer becomes a filter whose transmission rises and falls in
fringes. With a phase shifter on an arm it becomes a switch or a modulator. Meshes of them make
programmable circuits.

## What it does

Light enters one guide and meets the **splitter**, a coupler that sends part of it into the
other guide. The two **arms** carry the halves separately, and the **combiner**, a second
coupler, mixes them again. Each output receives two contributions, one through each arm. The
**cross** output, the guide the light didn't enter by, gets them in step when the arms are equal:
with two even couplers and equal arms, all the light crosses over. Lengthen one arm by half a
wavelength in the guide and the contributions swap roles: all the light leaves by the **bar**
output, the guide it came in by.

Make one arm longer by $\Delta L$ and the phase difference grows with frequency, so the outputs
take turns in fringes across the spectrum, a **free spectral range** apart. Move the path
difference and the fringes crowd together or spread out; turn the phase shifter and they slide
sideways without changing their spacing; make the couplers uneven, or the arms lossy, and the dark
fringes stop being dark.

::chart mzi-spectrum

Three numbers describe it: where the fringes are (the phase difference at each wavelength), how
far apart they are (the free spectral range, set by $\Delta L$ and the **group index**), and how
deep they go (the **extinction**, set by how evenly the two paths share the light).

## The physics {theory}

### Two couplers and two arms

Take ideal couplers that keep a share $r$ of the field in its guide and cross a share $k$ over,
with $r^2 + k^2 = 1$ and a phase of $i$ on the crossing field, as for the ring
([Bogaerts et al., 2011](https://doi.org/10.1002/lpor.201100017), whose convention photonoxide's
couplers follow). On the basis (lower guide, upper guide) a coupler is the matrix
$C = \begin{pmatrix} r & ik \cr ik & r \end{pmatrix}$. The arms multiply their fields by
$a_l e^{i\phi_l}$ and $a_u e^{i\phi_u}$, where $a$ is each arm's amplitude after its loss and
$\phi = 2\pi n_\text{eff} L/\lambda$ its phase. The interferometer is the product

$$
M = C_2 \begin{pmatrix} a_l e^{i\phi_l} & 0 \cr 0 & a_u e^{i\phi_u} \end{pmatrix} C_1 .
$$

Light entering the lower guide leaves the bar (lower) output with the field $M_{11}$ and the cross
(upper) one with $M_{21}$:

$$
M_{11} = r_1 r_2\thinspace a_l e^{i\phi_l} - k_1 k_2\thinspace a_u e^{i\phi_u}, \qquad
M_{21} = i\left(r_1 k_2\thinspace a_l e^{i\phi_l} + k_1 r_2\thinspace a_u e^{i\phi_u}\right).
$$

Each output adds two paths, one through each arm, and only their phase difference
$\Delta\phi = \phi_u - \phi_l$ matters. Squaring,

$$
T_\text{bar} = (r_1 r_2 a_l)^2 + (k_1 k_2 a_u)^2 - 2 r_1 r_2 k_1 k_2\thinspace a_l a_u \cos\Delta\phi ,
$$

$$
T_\text{cross} = (r_1 k_2 a_l)^2 + (k_1 r_2 a_u)^2 + 2 r_1 r_2 k_1 k_2\thinspace a_l a_u \cos\Delta\phi .
$$

For even couplers, $r^2 = k^2 = \tfrac{1}{2}$, and lossless arms these are the interferometer's
classic transmissions,

$$
T_\text{cross} = \cos^2\frac{\Delta\phi}{2}, \qquad T_\text{bar} = \sin^2\frac{\Delta\phi}{2} ,
$$

which add to 1: a lossless interferometer only moves light between its outputs. A Y-branch or a
1 × 2 splitter in place of the second coupler has one output, which takes the sum of the arms'
fields, $(a_l e^{i\phi_l} + a_u e^{i\phi_u})/2$ for even splitting, and so, lossless, the cross
port's $\cos^2(\Delta\phi/2)$; the
difference goes into a mode the single guide can't carry, and is radiated away.

### The phase difference, and the free spectral range

With the upper arm longer by $\Delta L$ and a phase shifter adding $\varphi$ to it,

$$
\Delta\phi(\lambda) = \frac{2\pi n_\text{eff}(\lambda)\thinspace\Delta L}{\lambda} + \varphi .
$$

The cross output peaks where $\Delta\phi$ is a whole number of turns, $2\pi m$; without the phase
shifter that is $n_\text{eff}\Delta L = m\lambda$, Eq. 3 of
[Dwivedi et al. (2015)](https://doi.org/10.1109/JLT.2015.2476603), $m$ the interferometer's
**order**. Neighbouring peaks are one turn of $\Delta\phi$ apart. Differentiating,

$$
\frac{d\Delta\phi}{d\lambda} = -\frac{2\pi\Delta L}{\lambda^2}\left(n_\text{eff} - \lambda\frac{dn_\text{eff}}{d\lambda}\right) = -\frac{2\pi n_g \Delta L}{\lambda^2},
\qquad n_g = n_\text{eff} - \lambda\frac{dn_\text{eff}}{d\lambda} ,
$$

so the fringes are a free spectral range apart,

$$
\text{FSR} = \frac{\lambda^2}{n_g \Delta L} ,
$$

Dwivedi et al.'s Eq. 8. The spacing is set by the **group index** $n_g$, not the effective index:
the phase difference changes with wavelength both because $1/\lambda$ does and because
$n_\text{eff}$ does. In a silicon wire the two differ a lot, $n_\text{eff}$ near 2.4 and $n_g$
near 4.2 at 1.55 µm, so using the effective index would put the fringes 75 % too far apart. The
first chart shows both figures beside the spacing it measures between its peaks.

Between two neighbouring peaks $\lambda_1 < \lambda_2$ the order drops by one,
$n_\text{eff}(\lambda_1)\Delta L/\lambda_1 - n_\text{eff}(\lambda_2)\Delta L/\lambda_2 = 1$. For a
mode whose $n_g$ doesn't change between them this is exactly

$$
n_g = \frac{\lambda_1 \lambda_2}{(\lambda_2 - \lambda_1)\Delta L} ,
$$

which is how a measured spectrum gives the group index; the chart's "$n_g$ read from the peaks"
does this to its own spectrum, and finds the $n_g$ it was given.

### Extinction

Each output's power swings between the two paths adding, $(A + B)^2$, and cancelling,
$(A - B)^2$, where $A$ and $B$ are the two paths' amplitudes: $A = r_1 r_2 a_l$ and
$B = k_1 k_2 a_u$ for the bar output. The **extinction ratio** is their ratio,

$$
\text{ER}_\text{bar} = \left(\frac{r_1 r_2 a_l + k_1 k_2 a_u}{r_1 r_2 a_l - k_1 k_2 a_u}\right)^2 ,
$$

infinite only when the two paths carry exactly equal amplitudes, $r_1 r_2 a_l = k_1 k_2 a_u$, and
the cross output's likewise with $A = r_1 k_2 a_l$ and $B = k_1 r_2 a_u$. Two things unbalance
them: couplers that don't split evenly, and an arm that loses more than the other. For a splitter
that is off by $\varepsilon$, $\kappa_1^2 = \tfrac{1}{2} + \varepsilon$, an even combiner and equal
loss, $\text{ER} \approx 1/\varepsilon^2$: a splitting of 55:45 caps the extinction near 26 dB. Two
equal but uneven couplers leave the cross output perfectly dark at its minima (its two paths are
$r k a$ and $k r a$, always equal) while the bar output never goes dark. And an arm that loses more
can be balanced by a splitter that sends it more light.

::chart mzi-extinction

## History {research}

Ludwig Zehnder described a new interference refractometer in 1891, and Ludwig Mach, Ernst Mach's
son, another in 1892, both in the Zeitschrift für Instrumentenkunde (11, 275 and 12, 89; neither
paper has a DOI): two mirrors and two half-silvered plates that separate the beams widely, so that
a sample, a gas say, can be put in one of them and its index measured against the other.
[Born and Wolf](https://doi.org/10.1017/CBO9781139644181) describe it among the classical two-beam
interferometers.

Integrated optics made it a component. Waveguides diffused into lithium niobate could carry the
arms, and electrodes beside them push the phase through the Pockels effect:
[Alferness's 1982 review](https://doi.org/10.1109/TMTT.1982.1131213) sets out these
interferometric modulators, Martin's Y-branch interferometer of 1975 and the balanced bridge of
Ramaswamy, Divino and Standley of 1978 among them, and by 2000 lithium niobate Mach–Zehnder
modulators met the needs of the 2.5, 10 and 40 Gb/s fibre links of the time
([Wooten et al., 2000](https://doi.org/10.1109/2944.826874)). Silica waveguides on silicon,
the planar lightwave circuits ([Kawachi, 1990](https://doi.org/10.1007/BF02113964)), turned the
interferometer into a wavelength filter: [Takato et al. (1990)](https://doi.org/10.1109/49.57816)
made multiplexers whose channel spacing went from 0.01 to 250 nm with the path difference, and
[Jinguji and Kawachi (1995)](https://doi.org/10.1109/50.350643) cascaded interferometers with
phase shifters, on such chips thermo-optic heaters, into lattice filters synthesized like digital
ones.

Silicon came next. Its crystal has no Pockels effect, but free carriers change its index
([Soref and Bennett, 1987](https://doi.org/10.1109/JQE.1987.1073206)), and in 2004 Liu and
colleagues at Intel put metal–oxide–semiconductor capacitors in the arms of a silicon
Mach–Zehnder and modulated it past a gigahertz
([Liu et al., 2004](https://doi.org/10.1038/nature02310)); by 2010 silicon modulators were
reviewed as a field of their own ([Reed et al., 2010](https://doi.org/10.1038/nphoton.2010.179)).
Meanwhile [Reck et al. (1994)](https://doi.org/10.1103/PhysRevLett.73.58) had shown that a
triangle of beam splitters and phase shifters makes any unitary transformation, and
[Clements et al. (2016)](https://doi.org/10.1364/OPTICA.3.001460) a rectangle of them with half the
depth: meshes whose unit is a Mach–Zehnder interferometer with a phase shifter.

::timeline

## Why the group index matters {research}

A silicon wire's index depends steeply on its width and thickness: an interferometer's fringe
moves by about a nanometre for each nanometre of width, and by about 1.4 nm for each nanometre of
thickness ([Dwivedi et al., 2015](https://doi.org/10.1109/JLT.2015.2476603)). So the index a
design assumes is rarely the index a wafer delivers, and measuring it is worth care. A spectrum's
spacing gives the group index, but the effective index sits in the fringes' absolute position,
$n_\text{eff}\Delta L = m\lambda$, and that needs the order $m$, which a high-order interferometer
can't tell apart from its neighbours.

Dwivedi and colleagues used three interferometers per wire. Two of low order, $m$ = 15 and 16,
have fringes so far apart that the fabrication can't shift the index by enough to mistake the
order (their Fig. 1 puts the limit at orders of 17, 27 and 38 for wires 450, 600 and 800 nm
wide): one fringe gives $n_\text{eff}$ outright. The third, of order 110, has many fringes across
the band, and their spacing gives $n_g$ and how it changes with wavelength. The same
interferometers measured at several temperatures give the thermo-optic coefficient, from
$dn/dT = (n_g/\lambda)\thinspace d\lambda/dT$ (their Eq. 13): silicon's 1.86 × 10⁻⁴ per kelvin moves a
silicon filter by about 80 pm per kelvin.

Try their high-order interferometer. For the wire drawn 450 nm wide, the `mzi_dwivedi` example
designs it with a path difference of 73.06 µm, and the paper measured that wire's group index as
4.2739 (its Table I). The chart's effective index stays at its 2.4, so the order isn't theirs,
but the spacing is:

::chart mzi-spectrum{delta=73.06um, group_index=4.27, window=30nm}

## Modulators and switches {research}

Hold $\Delta L$ at zero, or at a fixed value, and change $\varphi$: the light moves between the
outputs. Heating one arm is the simplest way, through the thermo-optic effect: a phase
$\varphi = 2\pi (dn_\text{eff}/dT)\Delta T\thinspace L_h/\lambda$ for a heater of length $L_h$, so
a π shift over 100 µm of silicon wire needs it about 40 K warmer, taking silicon's own coefficient
for the mode's. Heaters are slow, microseconds, but simple and nearly lossless, and they set the
working points of most programmable circuits.

For speed, the index must follow a voltage. A field through an electro-optic crystal changes its
index in proportion (the Pockels effect), so $\varphi = \pi V/V_\pi$, and the cross output follows
$\cos^2(\pi V / 2V_\pi + \Delta\phi_0/2)$, $\Delta\phi_0$ the phase difference at no voltage:
biased half way, at quadrature, it turns a small voltage into a nearly proportional change of
power. The half-wave voltage $V_\pi$ is the figure of merit,
traded against the length of the electrodes and so against bandwidth.
[Wang et al. (2018)](https://doi.org/10.1038/s41586-018-0551-y) made such modulators in thin-film
lithium niobate: with 20 mm electrodes, a $V_\pi$ of 1.4 V, a 3 dB bandwidth above 45 GHz, an
extinction of about 30 dB and under 0.5 dB of loss on the chip; with 5 mm, 100 GHz at 4.4 V.
Silicon has no Pockels effect and modulates with carriers instead, which also absorb: its
modulators are lossier and longer for the same phase, and unbalance the arms' amplitudes as they
modulate ([Reed et al., 2010](https://doi.org/10.1038/nphoton.2010.179)).

photonoxide's phase shifter is so far ideal, $e^{i\varphi}$ at every wavelength and without loss.
The heater, with its power, time constant and crosstalk, and the Pockels modulator, with its
electrodes, come in photonoxide 0.6; the silicon carrier modulator in 0.8.

## Today {research}

The interferometer is everywhere in integrated photonics, and three directions stand out. One is
**modulators**: thin-film lithium niobate's low voltage, high bandwidth and low loss together
([Wang et al., 2018](https://doi.org/10.1038/s41586-018-0551-y)), against the longer and lossier
silicon carrier modulators that a CMOS process makes cheaply. Another is **filters**: cascades of interferometers as lattice filters, which can be
designed with flat pass bands, such as the eight-channel silicon demultiplexers of
[Horst et al. (2013)](https://doi.org/10.1364/OE.21.011652), under 1.6 dB of loss on about
500 × 400 µm, where each interferometer's fringes must sit to a fraction of a nanometre despite
the wafer's spread. The third is **programmable meshes**: many interferometers, each with two phase
shifters, joined so that software sets what the circuit does
([Bogaerts et al., 2020](https://doi.org/10.1038/s41586-020-2764-0)). An interferometer with even
couplers can give any split from all bar to all cross, which is why it is the mesh's usual gate,
and each gate loses something like 0.05 to 0.2 dB. Configuring hundreds of them is a problem of
its own, which [Pérez-López et al. (2020)](https://doi.org/10.1038/s41467-020-19608-w) solve by
optimization on the chip itself. Open problems include couplers that stay even across a wide band
and a whole wafer, heaters that don't warm their neighbours, and meshes that calibrate and correct
their own errors as they scale.

## How photonoxide computes it

photonoxide has the interferometer as a circuit of components, each a closed form with its exact
derivatives ([the components](../docs/methods/components.md)):

- **`Coupler`**: an ideal 2 × 2 coupler, through $\sqrt{1 - \kappa^2}$ and across $i\kappa$, and
  `DirectionalCoupler` from the two supermodes of a coupled pair of guides, from the mode solver;
  `YBranch` and `Mmi` for even 1 × 2 splitters.
- **`Waveguide`**: $e^{i\gamma L}$, its mode's effective and group index and dispersion at a
  wavelength, and its loss, set by hand or found by the full-vector mode solver.
- **`PhaseShifter`**: $e^{i\varphi}$.
- **`mzi`** and **`mzi_y`**: the interferometer as a netlist of the above, solved by
  [the circuit solve](../docs/methods/circuits.md), and `mzi_closed_form`, the product of transfer
  matrices above, which checks it.

The first chart builds that netlist with a phase shifter on the upper arm and solves it at every
wavelength; the second evaluates the closed form. The Chip page opens the same interferometer
from `circuits/mzi.toml`, and the circuit adjoint gives the gradient of any output with respect
to every coupler, length and phase ([the circuit adjoint](../docs/methods/circuit-adjoint.md)).
The 3D view above is `jobs/mzi-mmi.toml`, a structure job: it draws an interferometer, solving
nothing.

## What our example reproduces

The netlist is checked against the closed form and, without loss, for conserving power; its
gradients against finite differences, for one interferometer and for a 4 × 4 mesh of six:

::validation components/mzi-closed-form components/mzi-unitarity circuit/mzi-closed-form circuit/adjoint-mzi circuit/adjoint-mesh

[mzi_dwivedi](../examples/mzi_dwivedi.rs) predicts Dwivedi et al.'s measured indices with nothing
taken from the measurement: each wire is the cross-section their SEM shows, a rectangle in
photonoxide's silicon and silica, its mode by Hadley's equations on grids of about 10 nm; the
interferometers are designed as theirs were and built from ideal splitters, and their spectra are
read as the paper reads the measured ones. Five of the six numbers agree within 0.012; the
narrowest wire's group index is 0.041 low, inside the tolerance the paper's own fabrication
estimate gives, and its sidewalls, sloped and rough where the model's are straight, are the likely
reason:

::validation components/mzi-neff-dwivedi-470 components/mzi-ng-dwivedi-470 components/mzi-neff-dwivedi-602 components/mzi-ng-dwivedi-602 components/mzi-neff-dwivedi-805 components/mzi-ng-dwivedi-805

::example mzi_dwivedi

[circuit_splitter](../examples/circuit_splitter.rs) uses the interferometer as a tunable
splitter, a mesh's gate: two even couplers and a phase between them, tuned by the circuit
adjoint's gradient until 30 % of the light stays in the bar port, at the phase
$2\arcsin\sqrt{0.3}$ that $T_\text{bar} = \sin^2(\Delta\phi/2)$ gives:

::example circuit_splitter

## Try it

**The group index sets the spacing.** In the first chart, double the path difference, then
change the group index. What moves the fringes closer together, and what only slides them?

:::answer
Doubling $\Delta L$ halves the free spectral range, and so does raising $n_g$:
$\text{FSR} = \lambda^2/(n_g\Delta L)$. The "$\lambda^2/(n_\text{eff}\Delta L)$" figure, which uses
the effective index, never matches the measured spacing. The phase shifter only slides the
fringes: it adds the same phase at every wavelength.
:::

**How even is even enough?** Set the splitter to 0.55, then find how close to 0.5 it must be for
the bar output to reach 30 dB of extinction, the figure Wang et al. measured.

:::answer
At 0.55 the bar's extinction is about 26 dB. With $\kappa_1^2 = \tfrac{1}{2} + \varepsilon$ and an
even combiner, $\text{ER} \approx 1/\varepsilon^2$, so 30 dB, a ratio of 1000, needs
$|\varepsilon| \lesssim 0.03$: a splitter between about 47:53 and 53:47. The second chart shows the
same thing as a sharp peak around 0.5.
:::

**Uneven but equal couplers.** Set both couplers to 0.3. Which output still goes dark?

:::answer
The cross output: its two paths, $r_1 k_2 a_l$ and $k_1 r_2 a_u$, are equal whenever the couplers
are, so it still reaches zero, though its brightest is only $4\kappa^2(1 - \kappa^2) = 0.84$. The
bar output's paths are $r^2 a_l$ and $k^2 a_u$, unequal unless $\kappa^2 = \tfrac{1}{2}$: it never
goes below $(1 - 2\kappa^2)^2 = 0.16$, an extinction of about 8 dB.
:::

**A lossy arm.** Set the arm loss to 100 dB/cm and the path difference to 2 mm: the longer arm
now loses 20 dB more. What is left of the fringes, and what splitter brings them back?

:::answer
The upper arm's field is a tenth of the lower's, so the extinction falls to
$((1 + 0.1)/(1 - 0.1))^2$, under 2 dB. In the second chart, with the same loss and path difference,
both outputs go dark again at $\kappa_1^2 = a_l^2/(a_l^2 + a_u^2) \approx 0.99$: send almost all
the light into the lossy arm, so that what survives it matches what the short arm carries.
:::

**A modulator's working point.** With even couplers, turn the phase shifter. Where is the cross
output most sensitive to a small change of phase?

:::answer
At $\varphi$ = ±π/2 from a peak, quadrature, where $\cos^2(\Delta\phi/2)$ is at half its height and
steepest. A modulator is biased there; at a peak or a null the power hardly changes for a small
phase. A shift of π swaps the outputs: that is the half-wave voltage's π.
:::

## Further reading {research}

- [Born and Wolf](https://doi.org/10.1017/CBO9781139644181) for two-beam interference and the
  classical interferometers.
- [Dwivedi et al. (2015)](https://doi.org/10.1109/JLT.2015.2476603) for measuring a waveguide
  with interferometers: $n_\text{eff}$, $n_g$ and their temperature dependence.
- [Wooten et al. (2000)](https://doi.org/10.1109/2944.826874) and
  [Reed et al. (2010)](https://doi.org/10.1038/nphoton.2010.179) for lithium niobate and silicon
  modulators, and [Wang et al. (2018)](https://doi.org/10.1038/s41586-018-0551-y) for thin-film
  lithium niobate.
- [Jinguji and Kawachi (1995)](https://doi.org/10.1109/50.350643) for filters synthesized from
  cascaded interferometers.
- [Bogaerts et al. (2020)](https://doi.org/10.1038/s41586-020-2764-0) for programmable meshes,
  and [Clements et al. (2016)](https://doi.org/10.1364/OPTICA.3.001460) for their arrangement.
- In photonoxide: [the components](../docs/methods/components.md), and the lesson on the
  [ring resonator](ring-resonator.md), the other interferometer, which brings the light back to
  itself instead of to a second coupler.

---
title: "The ring resonator"
summary: "A loop of waveguide beside a bus, analysed question by question: resonance, free spectral range, the coupling regimes, linewidth and Q, the add-drop ring, and what a measured spectrum can and can't tell about a ring."
topic: Rings
level: introductory
minutes: 50
prerequisites: []
examples: [circuit_ring_critical, ring_q_factor, circuit_fit]
jobs: [ring-fdfd.toml]
circuits: [ring-all-pass.toml, ring-add-drop.toml]
methods: [components.md, circuits.md, circuit-adjoint.md, compact.md]
validation:
  - components/ring-all-pass-bogaerts
  - components/ring-add-drop-bogaerts
  - components/ring-extremes-bogaerts
  - components/ring-linewidth-bogaerts
  - circuit/ring-fsr-bogaerts
  - components/ring-fsr-fdfd
  - compact/vf-ring-allpass
  - compact/vf-ring-poles
charts: [ring-spectrum, ring-extinction, ring-coupling, ring-q-length, ring-identify]
papers:
  - cite: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2103 (1969)"
    title: "Bends in Optical Dielectric Guides"
    doi: 10.1002/j.1538-7305.1969.tb01167.x
    year: 1969
    role: origin
    note: "How much light curved dielectric guides radiate, and a high-Q closed loop between two straight guides, a ring-type channel-dropping filter, proposed for integrated optics."
  - cite: "B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, 998 (1997)"
    title: "Microring resonator channel dropping filters"
    doi: 10.1109/50.588673
    year: 1997
    role: milestone
    note: "Microrings side-coupled to waveguides as compact channel-dropping filters, analysed by coupling of modes in time, and higher-order filters from several coupled rings."
  - cite: "B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999)"
    title: "Rational approximation of frequency domain responses by vector fitting"
    doi: 10.1109/61.772353
    year: 1999
    role: milestone
    note: "Vector fitting: a measured or simulated response as poles and residues, found by two linear least squares repeated. photonoxide's compact models use it, and it reads a ring's coupling and loss off its complex field."
  - cite: "C. Manolatou, M. J. Khan, S. Fan, P. R. Villeneuve, H. A. Haus, J. D. Joannopoulos, IEEE J. Quantum Electron. 35, 1322 (1999)"
    title: "Coupling of modes analysis of resonant channel add-drop filters"
    doi: 10.1109/3.784592
    year: 1999
    role: milestone
    note: "The resonator as a single mode decaying into its ports and its losses: the picture behind intrinsic, coupling and loaded Q."
  - cite: "A. Yariv, Electron. Lett. 36, 321 (2000)"
    title: "Universal relations for coupling of optical power between microresonators and dielectric waveguides"
    doi: 10.1049/el:20000340
    year: 2000
    role: milestone
    note: "The relations between a ring's coupling and its loss, and the condition, coupling equal to loss, under which the bus transmits nothing."
  - cite: "M. Cai, O. Painter, K. J. Vahala, Phys. Rev. Lett. 85, 74 (2000)"
    title: "Observation of Critical Coupling in a Fiber Taper to a Silica-Microsphere Whispering-Gallery Mode System"
    doi: 10.1103/PhysRevLett.85.74
    year: 2000
    role: milestone
    note: "Critical coupling observed: a fused-silica microsphere of Q about ten million coupled to a fibre taper, its transmission on resonance 26 dB down at the critical point."
  - cite: "K. J. Vahala, Nature 424, 839 (2003)"
    title: "Optical microcavities"
    doi: 10.1038/nature01939
    year: 2003
    role: review
    note: "A survey of the resonators that confine light in small volumes for a long time, rings and disks among them, and what their Q makes possible."
  - cite: "J. E. Heebner, V. Wong, A. Schweinsberg, R. W. Boyd, D. J. Jackson, IEEE J. Quantum Electron. 40, 726 (2004)"
    title: "Optical transmission characteristics of fiber ring resonators"
    doi: 10.1109/JQE.2004.828232
    year: 2004
    role: milestone
    note: "A fibre ring's transmission and the phase it adds, measured under-coupled, critically coupled and over-coupled: the phase behaviour that tells the regimes apart."
  - cite: "Q. Xu, B. Schmidt, S. Pradhan, M. Lipson, Nature 435, 325 (2005)"
    title: "Micrometre-scale silicon electro-optic modulator"
    doi: 10.1038/nature03569
    year: 2005
    role: milestone
    note: "A silicon ring whose resonance is moved by injected carriers: a modulator a few micrometres across, where a straight one needs millimetres."
  - cite: "W. R. McKinnon, D.-X. Xu, C. Storey, E. Post, A. Densmore, A. Delâge, P. Waldron, J. H. Schmid, S. Janz, Opt. Express 17, 18971 (2009)"
    title: "Extracting coupling and loss coefficients from a ring resonator"
    doi: 10.1364/OE.17.018971
    year: 2009
    role: milestone
    note: "Coupling and loss from one ring's resonance widths, depths and spacings; the formulas can't say which is which, and how each varies with wavelength or device parameters tells them apart."
  - cite: "W. Bogaerts, P. De Heyn, T. Van Vaerenbergh, K. De Vos, S. Kumar Selvaraja, T. Claes, P. Dumon, P. Bienstman, D. Van Thourhout, R. Baets, Laser Photonics Rev. 6, 47 (published online 13 September 2011; the January 2012 issue)"
    title: "Silicon microring resonators"
    doi: 10.1002/lpor.201100017
    year: 2011
    role: review
    note: "The closed forms of all-pass and add-drop rings, their linewidth, finesse and Q, how coupling and loss are measured, and silicon rings' design and uses: the formulas photonoxide's rings implement."
  - cite: "X. Ji, F. A. S. Barbosa, S. P. Roberts, A. Dutt, J. Cardenas, Y. Okawachi, A. Bryant, A. L. Gaeta, M. Lipson, Optica 4, 619 (2017)"
    title: "Ultra-low-loss on-chip resonators with sub-milliwatt parametric oscillation threshold"
    doi: 10.1364/OPTICA.4.000619
    year: 2017
    role: current
    note: "Silicon nitride rings with so little loss that a fraction of a milliwatt starts parametric oscillation."
  - cite: "T. J. Kippenberg, A. L. Gaeta, M. Lipson, M. L. Gorodetsky, Science 361, eaan8083 (2018)"
    title: "Dissipative Kerr solitons in optical microresonators"
    doi: 10.1126/science.aan8083
    year: 2018
    role: review
    note: "Light circling a nonlinear ring as stable pulses, which make frequency combs on a chip."
  - cite: "B. Shen, L. Chang, J. Liu, H. Wang, Q.-F. Yang, C. Xiang, R. N. Wang, J. He, T. Liu, W. Xie, J. Guo, D. Kinghorn, L. Wu, Q.-X. Ji, T. J. Kippenberg, K. Vahala, J. E. Bowers, Nature 582, 365 (2020)"
    title: "Integrated turnkey soliton microcombs"
    doi: 10.1038/s41586-020-2358-x
    year: 2020
    role: current
    note: "A laser coupled straight into a ring, without isolator or control: switched on, they start a soliton comb by themselves."
  - cite: "M. W. Puckett, K. Liu, N. Chauhan, Q. Zhao, N. Jin, H. Cheng, J. Wu, R. O. Behunin, P. T. Rakich, K. D. Nelson, D. J. Blumenthal, Nat. Commun. 12, 934 (2021)"
    title: "422 Million intrinsic quality factor planar integrated all-waveguide resonator with sub-MHz linewidth"
    doi: 10.1038/s41467-021-21205-4
    year: 2021
    role: current
    note: "A planar waveguide resonator whose intrinsic Q passes four hundred million."
---

The device from above, labelled with the symbols the equations below use: the bus, the ring of
radius $R$, the coupler with its $r$ and $k$, the round trip's $a e^{i\phi}$, and the ports. The
3D view is the ring of `jobs/ring-fdfd.toml` as photonoxide's geometry builds it.

::diagram ring

A ring resonator is a waveguide closed on itself beside a straight waveguide, the bus. Light
crosses into the ring where the two run close, goes round, and meets the bus again. Where a whole
number of wavelengths fits around the loop, every pass adds in step, the ring stores light, and
the bus's transmission dips. Rings serve as filters, modulators and sensors in silicon photonics
([Bogaerts et al., 2011](https://doi.org/10.1002/lpor.201100017)) and, in low-loss materials, as
frequency-comb sources ([Kippenberg et al., 2018](https://doi.org/10.1126/science.aan8083)).

## 1. The all-pass ring: transmission, phase and resonance

After one round trip the light in the ring has picked up a phase $\phi = 2\pi n_\text{eff} L/\lambda$,
for a round trip of length $L = 2\pi R$ and a mode of effective index $n_\text{eff}$, and its
amplitude has fallen by a factor $a$. Where $\phi$ is a whole number of turns the ring
**resonates**: the through port dips. The resonances are evenly spaced in frequency, and each has
a width and a depth. In the chart, the radius moves the resonances and their spacing; the
coupling and the loss set their width and depth.

::chart ring-spectrum

### The round trip {theory}

A point coupler keeps a share $r$ of the field going straight and crosses a share $k$ over,
$r^2 + k^2 = 1$ (no loss in the coupler), with a phase of $i$ on the crossing field: on the bus
$b_1 = r a_1 + i k a_2$, into the ring $b_2 = i k a_1 + r a_2$, where $a_1$ is the bus's incoming
field and $a_2$ the ring's field arriving back at the coupler. One trip round multiplies the field
by $A = a e^{i\phi}$, so $a_2 = A b_2$. Eliminating the ring's fields,

$$
b_2 = \frac{i k a_1}{1 - r A}, \qquad
t = \frac{b_1}{a_1} = r - \frac{k^2 A}{1 - r A} = \frac{r - a e^{i\phi}}{1 - r a e^{i\phi}} ,
$$

using $r^2 + k^2 = 1$: Eq. 1 of [Bogaerts et al. (2011)](https://doi.org/10.1002/lpor.201100017).
The through power is its square, their Eq. 2,

$$
T(\phi) = \frac{a^2 - 2 r a \cos\phi + r^2}{1 - 2 r a \cos\phi + (r a)^2} ,
$$

and the power circulating in the ring, $|b_2/a_1|^2 = k^2 / |1 - r a e^{i\phi}|^2$, peaks at
$\cos\phi = 1$: on resonance $n_\text{eff} L = m \lambda$ for a whole number $m$, the ring's
order (their Eq. 3). Two facts to keep for later:

- $T(\phi)$ is symmetric in $r$ and $a$: exchanging the coupler's loss and the round trip's
  leaves the through power unchanged at every wavelength.
- $t$ is not. Its phase, $\arg t$, follows Bogaerts et al.'s Eq. 4, and which of $r$ and $a$ is
  larger decides how it behaves across a resonance (Section 3).

The closed form agrees with the same ring built as a netlist, a coupler fed back through a
waveguide, to round-off:

::validation components/ring-all-pass-bogaerts

## 2. Free spectral range and group index

Neighbouring resonances are one **free spectral range** (FSR) apart. A ring twice as long has
resonances twice as close. The spacing is set by the group index $n_g$, not the effective index:
moving to the next resonance means changing the wavelength, and $n_\text{eff}$ changes with it.

### The derivation {theory}

The phase changes with wavelength as

$$
\frac{d\phi}{d\lambda} = \frac{2\pi L}{\lambda^2}\left(\lambda \frac{dn_\text{eff}}{d\lambda} - n_\text{eff}\right) = -\frac{2\pi n_g L}{\lambda^2},
\qquad n_g = n_\text{eff} - \lambda \frac{dn_\text{eff}}{d\lambda} ,
$$

so a whole turn, $\Delta\phi = 2\pi$, is

$$
\text{FSR} = \frac{\lambda^2}{n_g L} ,
$$

Bogaerts et al.'s Eqs. 9 and 10. It is first order: the error is about $(\text{FSR}/2\lambda)^2$
relative when the FSR is measured between two resonances and $\lambda$ taken at their midpoint.
Measuring the FSR of rings of several lengths is how $n_g$ is measured: Bogaerts et al. fit
Eq. 9 to rings of one cross-section and find $n_g = 4.30$ (their Fig. 13), the value Section 4
uses. The chart's "Order $m$" is $n_\text{eff} L/\lambda$ rounded.

The FSR measured between two minima of the netlist's spectrum, and between two resonances of a
2D finite-difference solve of a 2 µm ring (the bent slab's group index; a 2D estimate on a 25 nm
grid, not a device's performance):

::validation circuit/ring-fsr-bogaerts components/ring-fsr-fdfd

**Try it.** In the first chart, double the radius at a fixed coupling. What happens to the FSR?

:::answer
It halves: $\lambda^2/(n_g L)$ with $L$ doubled. The order $m$ doubles too, give or take one,
since $m$ is $n_\text{eff} L/\lambda$ rounded.
:::

## 3. Coupling regimes: under, critical and over

On resonance the through port carries two waves: the light that passed the coupler without
crossing, and the light leaking back out of the ring. They are out of phase. When they have the
same amplitude they cancel and the bus goes dark: **critical coupling**. The ring then couples
out per round trip exactly what it loses per round trip. With less coupling the ring is
**under-coupled**, with more **over-coupled**, and the dip is shallower either way.

The chart puts the input coupling on the scale of that loss: $\kappa_1^2/\kappa_c^2 = 1$ is
critical. Each depth below 1 has a twin above 1; the figures name the twin of the ring set by the
slider and give both rings' Q.

::chart ring-extinction

### On resonance {theory}

Off resonance ($\cos\phi = -1$) and on it ($\cos\phi = 1$) the through power is (Bogaerts et
al.'s Eqs. 11 and 12)

$$
T_\text{off} = \frac{(r + a)^2}{(1 + r a)^2}, \qquad
T_\text{on} = \frac{(r - a)^2}{(1 - r a)^2} ,
$$

and $T_\text{on}$ vanishes at $r = a$, [Yariv's (2000)](https://doi.org/10.1049/el:20000340)
condition and Bogaerts et al.'s Eq. 12. With a power loss $\alpha$ per unit length,
$a = e^{-\alpha L/2}$, or $a = 10^{-\alpha_\text{dB} L/20}$ for a loss in dB; critical coupling
needs

$$
\kappa_c^2 = 1 - a^2 ,
$$

the share of the power one round trip loses. For small losses write $1 - r \approx \kappa^2/2$ and
$1 - a \approx \kappa_c^2/2$, and $x = \kappa^2/\kappa_c^2$:

$$
\sqrt{T_\text{on}} \approx \frac{|1 - x|}{1 + x}, \qquad
\frac{T_\text{off}}{T_\text{on}} \approx \left(\frac{1 + x}{1 - x}\right)^2 .
$$

This is the chart's dashed curve. It is unchanged by $x \to 1/x$: a ring coupled at half the
critical coupling and one coupled at twice it have the same extinction. The depth alone cannot
say which side of critical a ring is on.

The phase can. Write $t$ on resonance as $(r - a)/(1 - ra)$: real, positive under-coupled
($r > a$), negative over-coupled. Off resonance $t \approx 1$. Across a resonance an
under-coupled ring's phase swings out and back; an over-coupled ring's winds a whole turn; at
critical coupling it steps by $\pi$ as $|t|$ passes through zero (Bogaerts et al.'s Fig. 3, from
[Heebner et al., 2004](https://doi.org/10.1109/JQE.2004.828232)). Section 6 draws both.

The closed forms for the powers on and off resonance, all-pass and add-drop, against the spectra
themselves:

::validation components/ring-extremes-bogaerts

### Critical coupling by optimization {theory}

[circuit_ring_critical](../examples/circuit_ring_critical.rs) starts a 9.97 µm ring at
$\kappa^2 = 0.05$ and lets genoxide's L-BFGS-B, on the gradient of the circuit's adjoint
([the circuit adjoint](../docs/methods/circuit-adjoint.md)), drive the through power at 1.55 µm to
zero by moving the coupling and the radius. It ends on the closed forms: the radius on the
resonance of order $m$, $R = m\lambda/(2\pi n_\text{eff})$, and $\kappa^2 = 1 - a^2$, so $r/a = 1$.

::example circuit_ring_critical

### In the literature {research}

[Yariv (2000)](https://doi.org/10.1049/el:20000340) wrote the condition in its general form, for
any resonator coupled to a guide: transmission vanishes when the coupling equals the internal
loss. [Cai, Painter and Vahala (2000)](https://doi.org/10.1103/PhysRevLett.85.74) observed it the
same year in a fused-silica microsphere of Q about $10^7$ coupled to a fibre taper, with an
extinction on resonance of up to 26 dB at the critical point. In a fabricated ring the coupling is
set by the gap; Bogaerts et al. measure silicon rings with gaps of 150, 250 and 400 nm to separate
coupling from loss (their Section 3.3.3).

**Try it.** In the extinction chart, set the coupling to half the critical value (read
$\kappa_c^2$ from the figures). Where is the other coupling with the same extinction, and which
of the two has the higher Q?

:::answer
At about twice $\kappa_c^2$: the small-loss symmetry $x \to 1/x$. Not exactly twice, since the
losses aren't infinitely small; the figure gives the exact value. The under-coupled ring has the
higher Q: less light leaves it through the coupler, so its line is narrower.
:::

## 4. Linewidth and the Q factor: loaded, intrinsic and coupling

Each resonance is a dip of some width. The narrower the dip, the longer light stays in the ring:
the **quality factor** $Q$ is the resonance wavelength over its full width at half maximum
(FWHM), and the stored energy decays as $e^{-\omega t/Q}$. A ring of $Q = 10^5$ at 1.55 µm holds
its light for $Q/\omega$ = 82 ps (exact arithmetic). Light leaves by two ways, absorbed or
scattered in the ring, or coupled out to the bus; each way has its own Q, and the measured,
**loaded** Q combines them.

### Linewidth, finesse and Q {theory}

Near a resonance write $\phi = 2\pi m + \delta$ with $\delta$ small, $\cos\phi \approx 1 - \delta^2/2$:

$$
1 - T \approx \frac{(1 - r^2)(1 - a^2)}{(1 - r a)^2 + r a\thinspace\delta^2} ,
$$

a Lorentzian, at half its depth where $\delta = \pm(1 - ra)/\sqrt{ra}$. With
$d\phi/d\lambda = -2\pi n_g L/\lambda^2$ from Section 2,

$$
\text{FWHM} = \frac{(1 - r a)\lambda^2}{\pi n_g L \sqrt{r a}}, \qquad
F = \frac{\text{FSR}}{\text{FWHM}} = \frac{\pi\sqrt{r a}}{1 - r a}, \qquad
Q = \frac{\lambda}{\text{FWHM}} = \frac{\pi n_g L \sqrt{r a}}{\lambda (1 - r a)} ,
$$

Bogaerts et al.'s Eqs. 7, 21 and 20. The **finesse** $F$ counts linewidths between neighbouring
resonances. The widths measured on the spectra against Eq. 7 (and Eq. 8 for the add-drop ring):

::validation components/ring-linewidth-bogaerts

### Intrinsic and coupling Q {theory}

Set $r = 1$ (nothing coupled out) and Eq. 20 gives the ring's own, **intrinsic** Q, $Q_i$; set
$a = 1$ (no loss) and it gives the coupling's, $Q_c$. For a good ring $ra$ is close to 1 and
$1 - ra \approx (1 - r) + (1 - a)$: the rates of loss add, and so do the inverse Q's,

$$
\frac{1}{Q} \approx \frac{1}{Q_i} + \frac{1}{Q_c} ,
$$

the coupled-mode picture of [Manolatou et al. (1999)](https://doi.org/10.1109/3.784592): one
resonant mode decaying into each port and into its loss at its own rate. At critical coupling
$r = a$, so $Q_c = Q_i$ and the loaded Q is half the intrinsic one. In the chart, the dashed curve
crosses 0.5 where the through power reaches zero.

::chart ring-coupling

### Loss and coupling from a measured Q {theory}

A measurement gives the loaded Q and the depth $T_\text{on}$ (as a share of $T_\text{off}$).
In the small-loss form, $\sqrt{T_\text{on}} = |1/Q_i - 1/Q_c|\thinspace Q$ (Section 3 with
$1 - r$ and $1 - a$ as rates). Under-coupled, $1/Q_c < 1/Q_i$, and with $1/Q_c = 1/Q - 1/Q_i$ this
is $\sqrt{T_\text{on}} = 2Q/Q_i - 1$; over-coupled, $\sqrt{T_\text{on}} = 1 - 2Q/Q_i$. So

$$
Q_i = \frac{2Q}{1 + \sqrt{T_\text{on}}} \ \text{(under-coupled)}, \qquad
Q_i = \frac{2Q}{1 - \sqrt{T_\text{on}}} \ \text{(over-coupled)} .
$$

Two answers from one measurement; Section 6 shows why no fit of the through power alone can choose
between them.

### Q against length: Bogaerts et al.'s Fig. 5 {theory}

Eq. 20 grows with $L$, but a longer ring loses more per round trip, so $a$ falls. Bogaerts et al.
hold rings at critical coupling with 2.7 dB/cm in the waveguide, plus 0.04 dB in the bends and
0.035 dB in the coupler per round trip (their Eq. 19), and find Eq. 20's loaded Q peaking near
a 10 mm round trip, "about 1.42·10⁵". [ring_q_factor](../examples/ring_q_factor.rs) reproduces the
peak's length and the ratio of the all-pass and add-drop peaks; the Q itself needs the group index
their figure was drawn with, which the paper doesn't give, and with the 4.30 they measured it comes
out 6 % lower.

::example ring_q_factor

The example's last line measures the width on the spectrum and finds it narrower than Eq. 7: at
the peak the finesse is near 5, and Eq. 7 assumes a Lorentzian line, $\cos\phi \approx 1 - \phi^2/2$
across it. The chart below draws three loaded Q's of the same critically coupled ring. They agree
for short rings, where the finesse is high, and part where it falls:

- **Eq. 20**, the Lorentzian's $\lambda/\text{FWHM}$, peaks.
- **$\lambda/\text{FWHM}$ measured on the spectrum** keeps rising: as the finesse falls the dips
  widen into a cosine, whose half-depth width tends to FSR/2, and $\lambda/(\text{FSR}/2) = 2 n_g L/\lambda$ grows with $L$.
- **The energy's decay.** Each round trip, $n_g L/c$ long, multiplies the stored energy by
  $(ra)^2$, so $e^{-\omega t/Q}$ gives $Q = \pi n_g L/(\lambda\lvert\ln ra\rvert)$. This equals
  Eq. 20 to first order in $1 - ra$, rises with $L$ and levels off at $\pi n_g/(\lambda\alpha)$,
  half the waveguide's own $2\pi n_g/(\lambda\alpha)$ (critical coupling halves it).

So the peak of Fig. 5 is a property of Eq. 20 at low finesse, not of how long the ring stores
light. The chart's defaults are the example's ring, and its figures at the peak are the
example's numbers.

::chart ring-q-length

### In the literature {research}

Bogaerts et al. distinguish loaded from unloaded (intrinsic) Q and use "Q" for the loaded one, as
here. Record intrinsic Q's come from loss, not length: silicon nitride rings with losses low enough
for sub-milliwatt parametric oscillation ([Ji et al., 2017](https://doi.org/10.1364/OPTICA.4.000619)),
and a planar waveguide resonator with intrinsic Q above four hundred million
([Puckett et al., 2021](https://doi.org/10.1038/s41467-021-21205-4)). A high Q stores light long
enough for weak nonlinearities to act, the basis of Kerr combs
([Kippenberg et al., 2018](https://doi.org/10.1126/science.aan8083)). [Vahala (2003)](https://doi.org/10.1038/nature01939)
surveys microcavities by Q and mode volume.

**Try it.** In the Q chart, lower the loss to 1 dB/cm. Where does Eq. 20's peak go?

:::answer
To 2.7 times the length, 2.7 dB/cm over 1 dB/cm: with the fixed 0.075 dB small, Eq. 20 peaks
where $ra$ reaches a fixed value (the finesse there stays near 5), so where the waveguide's loss
over a round trip reaches a fixed amount, at a length going as $1/\alpha$. The decay Q's level,
$\pi n_g/(\lambda\alpha)$, rises by the same factor.
:::

**Try it.** In the coupling chart, read the loaded Q over the intrinsic Q where the through power
reaches zero.

:::answer
One half: at critical coupling the coupler's rate equals the ring's own, $1/Q = 2/Q_i$.
:::

## 5. The add-drop ring

A second bus on the far side takes the stored light out at its **drop** port: on resonance the
ring hands one wavelength from one bus to the other and lets the rest pass. The drop coupler is
one more way out of the ring, so it lowers the Q and moves critical coupling. Set a drop coupling
in the first chart to see the drop port.

### Two couplers {theory}

With the drop coupler's $r_2$ and $k_2$, the same elimination gives the through and drop powers
on resonance (Bogaerts et al., Eqs. 13 to 16),

$$
T_\text{through} = \frac{(r_2 a - r_1)^2}{(1 - r_1 r_2 a)^2}, \qquad
T_\text{drop} = \frac{(1 - r_1^2)(1 - r_2^2)\thinspace a}{(1 - r_1 r_2 a)^2} ,
$$

and the linewidth and Q of Section 4 with $ra$ replaced by $r_1 r_2 a$ (their Eqs. 8, 22 and 23).
The through port goes dark at $r_1 = r_2 a$: the drop coupler counts as loss, and the extinction
chart's $\kappa_c^2 = 1 - (1 - \kappa_2^2)a^2$ includes it. A lossless ring with equal couplers
($r_1 = r_2$, $a = 1$) sends everything on resonance to the drop port.

The through power is symmetric in $r_1$ and $r_2 a$, as the all-pass ring's is in $r$ and $a$; the
drop power is not. Measuring both ports is one way to tell coupling from loss (Section 6).

The add-drop closed form against Bogaerts et al.'s Eqs. 5 and 6 and against its netlist of two
couplers and two half rings:

::validation components/ring-add-drop-bogaerts

**Try it.** In the first chart, give the ring a drop coupling equal to its input coupling, then set
the loss to zero. Where does the light go on resonance?

:::answer
All of it to the drop port: with $r_1 = r_2$ and $a = 1$, $T_\text{drop} = 1$ and the through port
is dark. With loss, the through port goes dark only at $r_1 = r_2 a$, and the drop port then
carries less than all of it.
:::

## 6. Fitting a measured spectrum, and what it can't tell apart

A measured spectrum is fitted with the ring's model to find its coupling, loss and length. Whether
the fit's answer is the ring depends on what the spectrum contains. The through power of an
all-pass ring is the same for $(r, a)$ and $(a, r)$ (Section 1), so a ring and its **twin**, coupler
and round trip exchanged, give the same power at every wavelength: one over-coupled ring with low
loss, one under-coupled ring with high loss. Their fields differ in phase. The chart draws both.

::chart ring-identify

### What a power spectrum gives {theory}

From an all-pass ring's through power, three numbers: the FSR (so $n_g L$, Section 2), the finesse
and the depth.

- The finesse $F = \pi\sqrt{p}/(1 - p)$ gives $p = ra$: with $f = F/\pi$,
  $\sqrt{p} = (\sqrt{1 + 4f^2} - 1)/(2f)$.
- The depth gives $|r - a| = \sqrt{T_\text{on}}\thinspace(1 - p)$.
- $r$ and $a$ are then the two roots of $z^2 - (r + a) z + p = 0$, with $r + a = \sqrt{(r - a)^2 + 4p}$.

The two roots are the two readings in the chart: which is $r$ and which $a$ the power can't say
([Bogaerts et al., 2011](https://doi.org/10.1002/lpor.201100017), Section 3.3.3: "In an APF, a and
r are even completely interchangeable"). The same holds for any fit of the through power, however
it is done: both rings make the same data.

### What the field gives: vector fitting {theory}

The complex field $t$ does say. Vector fitting
([Gustavsen and Semlyen, 1999](https://doi.org/10.1109/61.772353); photonoxide's
[compact models](../docs/methods/compact.md)) writes a sampled response as poles and residues,
$t(s) \approx \sum_n c_n/(s - p_n) + d$, with $s = -i\omega$. The all-pass ring's poles are where
$r a e^{i\phi} = 1$; with $\phi$ linear in $k = 2\pi/\lambda$ (first-order dispersion, Bogaerts
et al.'s Eq. 10) they are exactly

$$
p_m = \frac{\ln(ra)}{n_g L} - i\thinspace\frac{2\pi m + C}{n_g L}, \qquad
c = -\frac{1/r - r}{n_g L} ,
$$

$C$ a constant of the dispersion (the compact models' write-up derives it). The pole's real part
gives $ra$, as the finesse did. The residue gives $r$ alone, and so $a = ra/r$: exchanging $r$ and
$a$ keeps the poles and changes the residue. The chart fits 201 samples of $t$ across the
resonance with 4 poles and reads $\kappa^2 = 1 - r^2$ and the loss in dB/cm back for the ring and
for its twin. The pole also gives the decay Q of Section 4, $|\operatorname{Im} p|/(2|\operatorname{Re} p|) = \pi n_g L/(\lambda\lvert\ln ra\rvert)$.

The library's fit of an all-pass ring ($r = 0.95$, $a = 0.98$) over four resonances, and its poles
against these exact ones:

::validation compact/vf-ring-allpass compact/vf-ring-poles

### Fitting several parameters: circuit_fit {theory}

[circuit_fit](../examples/circuit_fit.rs) fits an add-drop ring's through **and** drop powers, 81
wavelengths across one resonance, for four unknowns: $\kappa_1^2$, $\kappa_2^2$, the loss and the
radius. The drop power breaks the symmetry of the through power (Section 5), so the four are
determined. Two of genoxide's optimizers minimize the squared misfit from the same start: L-BFGS-B
on the gradient from one adjoint solve per wavelength
([the circuit adjoint](../docs/methods/circuit-adjoint.md)), and CMA-ES on the misfit alone. Both
return the parameters the spectra were made with; the count of evaluations is the price of no
gradient. The "measured" spectra are made with genoxide's portable exp and cos, as the circuit's
are, so the count is the same on every system.

::example circuit_fit

The radius's range is $\pm 0.01$ µm. The misfit has a minimum at every resonance order $m$: a fit
started more than about half a free spectral range away in resonance wavelength,
$\Delta\lambda = \lambda\thinspace\Delta R/R$, can settle on a neighbouring order. A single
resonance fixes $n_\text{eff} L = m\lambda$, not $m$; the FSR fixes $n_g L$.

### In the literature {research}

Bogaerts et al. (Section 3.3.3) list the ways out of the all-pass ambiguity. McKinnon et al.
([2009](https://doi.org/10.1364/OE.17.018971)) extract the two coefficients from the widths,
depths and spacings of one ring's resonances, note that the formulas don't say which is coupling
and which loss, and tell them apart by how each varies with wavelength or with the device's
parameters. The phase can be measured, the ring in one arm of a nearly balanced Mach–Zehnder
interferometer (Bogaerts et al., after
[Heebner et al., 2004](https://doi.org/10.1109/JQE.2004.828232), who measured a fibre ring's
transmission and phase under-, critically and over-coupled). An add-drop ring and an
all-pass ring with the same gap, or series of rings differing in one length or gap, give more
equations than unknowns; rings on different parts of a chip, though, differ by fabrication. Light
scattered backwards in the ring couples the two directions of circulation and splits a
high-Q resonance in two; the closed forms here leave that out, and Bogaerts et al. measure it
(their Fig. 12).

**Try it.** In the twin chart, lower the coupling below the critical coupling for 3 dB/cm (about
0.004 at 10 µm). Which phase now winds a whole turn?

:::answer
The twin's. The ring is now under-coupled and its twin, coupler and round trip exchanged,
over-coupled; the power curves still coincide, and the fit of the field still returns each ring.
:::

## 7. How photonoxide computes it

- **Closed forms.** `AllPassRing` and `AddDropRing` in `circuit::components` are Bogaerts et al.'s
  equations, with their derivatives for gradients. The waveguide is a `Dispersion`: an effective
  and group index at a wavelength, first order in wavelength (second with a dispersion
  parameter), a loss in dB/cm, and optionally a fixed loss per round trip. They give the
  resonance nearest a wavelength, the FSR, the linewidth, finesse, Q and the powers on and off
  resonance. Every chart here calls them ([the components](../docs/methods/components.md)).
- **A netlist.** The same ring as a coupler and a waveguide, the coupler's output fed back to its
  input, solved as a circuit ([circuits](../docs/methods/circuits.md)); the Chip page opens it from
  `circuits/ring-all-pass.toml`. Its adjoint gives the gradients the examples optimize with.
- **A field solve.** `jobs/ring-fdfd.toml` solves a 2 µm ring and its bus by 2D finite
  differences in the frequency domain, from the effective index method: an estimate of the
  spectrum's shape, not the device's 3D performance.
- **A compact model.** Any of these spectra, or a measured one in a Touchstone file, fitted by
  vector fitting ([compact models](../docs/methods/compact.md)).

## 8. History and today {research}

Marcatili, working out in 1969 how much light a curved dielectric guide radiates, drew a closed
loop between two straight guides as a channel-dropping filter. For the small index contrasts he
had in mind, a loop had to be tenths of a millimetre across not to radiate. High-contrast
waveguides made rings micrometres across: in 1997 Little, Chu, Haus and colleagues analysed
microrings side-coupled to waveguides as channel-dropping filters, each ring one mode coupled in
time, and Manolatou and others in Haus's group wrote the resonator as a mode leaking into its
ports. Yariv's letter of 2000 gave critical coupling its general form, and Cai, Painter and
Vahala observed it the same year. In 2005 Xu, Schmidt, Pradhan and Lipson moved a silicon ring's
resonance with injected carriers to make a modulator micrometres across. Bogaerts and colleagues'
2011 review gathered the closed forms and the practice of silicon rings that photonoxide's ring
components implement.

::timeline

Today's rings divide into silicon rings as filters, modulators and sensors, where holding a
resonance against temperature and fabrication spread and the splitting by backscattering are open
problems ([Bogaerts et al., 2011](https://doi.org/10.1002/lpor.201100017)), and low-loss rings for
nonlinear optics: parametric oscillation below a milliwatt
([Ji et al., 2017](https://doi.org/10.1364/OPTICA.4.000619)), soliton combs
([Kippenberg et al., 2018](https://doi.org/10.1126/science.aan8083)) started by a laser alone
([Shen et al., 2020](https://doi.org/10.1038/s41586-020-2358-x)), and intrinsic Q above $4 \times 10^8$
([Puckett et al., 2021](https://doi.org/10.1038/s41467-021-21205-4)).

Further reading: Bogaerts et al.'s review for every formula here and for thermal tuning,
backscattering and silicon ring design; [Little et al. (1997)](https://doi.org/10.1109/50.588673)
and [Manolatou et al. (1999)](https://doi.org/10.1109/3.784592) for coupled modes in time, which
carry over to disks and photonic-crystal cavities; and the lesson on
[Bragg gratings](bragg-gratings.md) for the other way a waveguide picks out a wavelength.

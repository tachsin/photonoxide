---
title: "The ring resonator"
summary: "A loop of waveguide beside a bus: why it picks out wavelengths, what sets its linewidth and Q, and why critical coupling makes the bus go dark."
topic: Rings
level: introductory
minutes: 30
prerequisites: []
examples: [circuit_ring_critical, ring_q_factor, circuit_fit]
jobs: [ring-fdfd.toml]
circuits: [ring-all-pass.toml, ring-add-drop.toml]
methods: [components.md, circuits.md, circuit-adjoint.md]
validation:
  - components/ring-all-pass-bogaerts
  - components/ring-add-drop-bogaerts
  - components/ring-extremes-bogaerts
  - components/ring-linewidth-bogaerts
  - circuit/ring-fsr-bogaerts
  - components/ring-fsr-fdfd
charts: [ring-spectrum, ring-coupling]
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
  - cite: "K. J. Vahala, Nature 424, 839 (2003)"
    title: "Optical microcavities"
    doi: 10.1038/nature01939
    year: 2003
    role: review
    note: "A survey of the resonators that confine light in small volumes for a long time, rings and disks among them, and what their Q makes possible."
  - cite: "Q. Xu, B. Schmidt, S. Pradhan, M. Lipson, Nature 435, 325 (2005)"
    title: "Micrometre-scale silicon electro-optic modulator"
    doi: 10.1038/nature03569
    year: 2005
    role: milestone
    note: "A silicon ring whose resonance is moved by injected carriers: a modulator a few micrometres across, where a straight one needs millimetres."
  - cite: "W. Bogaerts, P. De Heyn, T. Van Vaerenbergh, K. De Vos, S. Kumar Selvaraja, T. Claes, P. Dumon, P. Bienstman, D. Van Thourhout, R. Baets, Laser Photonics Rev. 6, 47 (2012)"
    title: "Silicon microring resonators"
    doi: 10.1002/lpor.201100017
    year: 2012
    role: review
    note: "The closed forms of all-pass and add-drop rings, their linewidth, finesse and Q, and silicon rings' design and uses: the formulas photonoxide's rings implement."
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

A ring resonator is a waveguide closed on itself, set beside a straight waveguide, the bus.
Light in the bus leaks a little into the ring at the place where they run close, goes round,
and meets the bus again. At most wavelengths the light coming round arrives out of step with
itself and the ring stays nearly empty. At the wavelengths where a whole number of wavelengths
fits around the loop, each pass adds in step, the ring fills up, and the bus loses that colour.
That is all a ring does, and it is enough to make filters, modulators, sensors, lasers and
frequency combs.

## What it does

Picture the light that has crossed into the ring. After one round trip it has picked up a phase
$\phi = 2\pi n_\text{eff} L/\lambda$, for a round trip of length $L$ and a mode of effective index
$n_\text{eff}$, and lost a little of its amplitude. When $\phi$ is a whole number of turns, the
light from every earlier pass adds up: the ring **resonates**. The resonances are evenly spaced
in frequency, a **free spectral range** apart, and each is a narrow dip in the bus's
transmission (the **through** port). A second bus on the far side (an **add-drop** ring) takes
the stored light out at its **drop** port, so the ring hands one wavelength from one waveguide to
the other and lets the rest pass.

Three numbers describe a ring: how far apart its resonances are (the free spectral range, set by
its length), how narrow each one is (its linewidth, set by how fast light leaves the ring), and
how deep the dip is (set by how the coupling compares with the loss). Move the sliders: the
radius moves the resonances and their spacing, the coupling and the loss change their width and
depth.

::chart ring-spectrum

The depth is the surprising part. A lossy ring coupled weakly makes a shallow dip; coupled
strongly, it makes a shallow, wide one; in between, at one coupling, the dip reaches the floor and
the bus transmits nothing at all on resonance. This is **critical coupling**: the light the ring
couples back out exactly cancels the light that passes it by.

## The physics {theory}

### One round trip

Take a point coupler between bus and ring that keeps a share $r$ of the field going straight and
crosses a share $k$ over, with $r^2 + k^2 = 1$ (no loss in the coupler), and a phase of $i$ on
the crossing field: on the bus $b_1 = r a_1 + i k a_2$ and into the ring $b_2 = i k a_1 + r a_2$,
where $a_1$ is the bus's incoming field and $a_2$ the ring's field arriving back at the coupler.
One trip round the ring multiplies the field by $A = a e^{i\phi}$, where $a$ is the single-pass
amplitude (1 for a lossless ring), so $a_2 = A b_2$. Eliminating the ring's fields,

$$
b_2 = \frac{i k a_1}{1 - r A}, \qquad
\frac{b_1}{a_1} = r - \frac{k^2 A}{1 - r A} = \frac{r - a e^{i\phi}}{1 - r a e^{i\phi}} ,
$$

using $r^2 + k^2 = 1$. This is the all-pass ring's through field, Eq. 1 of
[Bogaerts et al. (2012)](https://doi.org/10.1002/lpor.201100017). Its square is the through
power,

$$
T(\phi) = \frac{a^2 - 2 r a \cos\phi + r^2}{1 - 2 r a \cos\phi + (r a)^2} ,
$$

and the field inside, $|b_2/a_1|^2 = k^2 / |1 - r a e^{i\phi}|^2$, is largest where $\cos\phi = 1$:
on resonance, $n_\text{eff} L = m \lambda$ for a whole number $m$, the ring's order.

### Critical coupling

On resonance the through power is

$$
T_\text{min} = \frac{(r - a)^2}{(1 - r a)^2} ,
$$

which vanishes when $r = a$: the share of the field the coupler keeps in the ring equals the
share the round trip keeps. In words, the ring couples out per round trip what it loses per round
trip. With a power loss $\alpha$ per unit length, $a = e^{-\alpha L/2}$, or $a = 10^{-\alpha_\text{dB} L/20}$
for a loss in dB, and critical coupling needs $k^2 = 1 - a^2$, a small number for a good ring.
The condition is [Yariv's (2000)](https://doi.org/10.1049/el:20000340), and Bogaerts et al.
write it as their Eq. 12. When $k^2$ is smaller the ring is **under-coupled**, when larger
**over-coupled**; the two can give the same depth, which is why a dip alone doesn't tell you
which side of critical a ring is on.

### Linewidth, finesse and Q

Near a resonance write $\phi = 2\pi m + \delta$ with $\delta$ small, so
$\cos\phi \approx 1 - \delta^2/2$, and

$$
1 - T \approx \frac{(1 - r^2)(1 - a^2)}{(1 - r a)^2 + r a\thinspace\delta^2} ,
$$

a Lorentzian in $\delta$. It falls to half its depth at $\delta = \pm(1 - ra)/\sqrt{ra}$. The
phase changes with wavelength as $d\phi/d\lambda = -2\pi n_g L/\lambda^2$, with the group index
$n_g = n_\text{eff} - \lambda\thinspace dn_\text{eff}/d\lambda$, so a whole turn of phase, the
free spectral range, and the full width at half maximum are

$$
\text{FSR} = \frac{\lambda^2}{n_g L}, \qquad
\text{FWHM} = \frac{(1 - r a)\lambda^2}{\pi n_g L \sqrt{r a}} ,
$$

Bogaerts et al.'s Eqs. 9 and 7. Their ratio is the **finesse**, how many linewidths fit between
neighbouring resonances, and the wavelength over the linewidth is the **loaded Q**:

$$
F = \frac{\text{FSR}}{\text{FWHM}} = \frac{\pi\sqrt{r a}}{1 - r a}, \qquad
Q = \frac{\lambda}{\text{FWHM}} = \frac{\pi n_g L \sqrt{r a}}{\lambda (1 - r a)} .
$$

Q also counts how long light stays: the stored energy decays as $e^{-\omega t/Q}$, so a ring of
Q a hundred thousand at 1.55 µm holds its light for about 80 picoseconds. Since $ra$ is
close to 1 for a good ring, $1 - ra \approx (1 - r) + (1 - a)$: the losses through the coupler
and in the ring add, and so do their inverse Q's,

$$
\frac{1}{Q} = \frac{1}{Q_i} + \frac{1}{Q_c} ,
$$

$Q_i$ the **intrinsic** Q of the ring alone ($r = 1$) and $Q_c$ the coupling's. At critical
coupling the two are equal and the loaded Q is half the intrinsic one. This is the coupled-mode
picture of [Manolatou et al. (1999)](https://doi.org/10.1109/3.784592): one resonant mode,
decaying into each port and into its loss at its own rate.

::chart ring-coupling

### Two buses

An add-drop ring has a second coupler, $r_2$ and $k_2$, on the far side. The same elimination
gives the through and drop powers on resonance (Bogaerts et al., Eqs. 13 to 16),

$$
T_\text{through} = \frac{(r_2 a - r_1)^2}{(1 - r_1 r_2 a)^2}, \qquad
T_\text{drop} = \frac{(1 - r_1^2)(1 - r_2^2)\thinspace a}{(1 - r_1 r_2 a)^2} ,
$$

and the linewidth and Q are those above with $ra$ replaced by $r_1 r_2 a$. The through port goes
dark when $r_1 = r_2 a$: the drop coupler now counts as a loss. A lossless ring with equal
couplers ($r_1 = r_2$, $a = 1$) sends everything on resonance to the drop port.

## History {research}

Bent guides came first. Marcatili, working out in 1969 how much light a curved dielectric guide
radiates, drew a closed loop between two straight guides as a channel-dropping filter: the ring
as a component of integrated optics. His estimates put high-Q loops at radii of tenths of a
millimetre, for the small index differences of the guides he had in mind; a weakly guiding loop
radiates unless it is large. High-contrast waveguides changed that. In 1997 Little, Chu, Haus
and colleagues analysed microrings side-coupled to waveguides as compact channel-dropping
filters, treating each ring as one mode by coupling of modes in time, which made filters of
several coupled rings easy to write down; Manolatou and others in Haus's group developed the same
picture of a resonator as a mode leaking into its ports. Yariv's short letter of 2000 put critical coupling in its universal
form. Silicon photonics then made rings everyday parts: in 2005 Xu, Schmidt, Pradhan and Lipson
moved a silicon ring's resonance with injected carriers to make a modulator micrometres across,
and Bogaerts and colleagues' 2012 review gathered the closed forms and the practice of silicon
rings that photonoxide's ring components implement.

::timeline

## Today {research}

Two directions dominate. One is **loss**: silicon nitride rings with losses low enough that a
fraction of a milliwatt starts parametric oscillation ([Ji et al., 2017](https://doi.org/10.1364/OPTICA.4.000619)),
and planar resonators with intrinsic Q above four hundred million
([Puckett et al., 2021](https://doi.org/10.1038/s41467-021-21205-4)). Low loss stores light long
enough for weak nonlinearities to act, which is the other direction: **combs**. Light circling a
Kerr-nonlinear ring can settle into solitons, pulses that keep their shape while the ring's loss
and gain balance ([Kippenberg et al., 2018](https://doi.org/10.1126/science.aan8083)), and a
laser coupled straight into a ring can start such a comb by itself when switched on
([Shen et al., 2020](https://doi.org/10.1038/s41586-020-2358-x)). Meanwhile rings remain the
workhorse filters, modulators and sensors of silicon photonics
([Bogaerts et al., 2012](https://doi.org/10.1002/lpor.201100017)), where open problems include
holding a resonance in place against temperature and fabrication spread, and the splitting of
resonances by light scattered backwards, which the closed forms here leave out.

## How photonoxide computes it

photonoxide has a ring three ways, which check each other.

- **Closed forms.** `AllPassRing` and `AddDropRing` in `circuit::components` are the equations
  above, Bogaerts et al.'s, with their derivatives for gradients. The waveguide is a
  `Dispersion`: an effective index and group index at a wavelength, first order in wavelength
  (second with a dispersion parameter), and a loss in dB/cm. They also give the resonance nearest
  a wavelength, the free spectral range, the linewidth, finesse, Q and the powers on and off
  resonance. Every chart on this page calls them. See [the components](../docs/methods/components.md).
- **A netlist.** The same ring built from a coupler and a length of waveguide, the coupler's
  output fed back to its input, and solved as a circuit ([circuits](../docs/methods/circuits.md)).
  The Chip page opens it from `circuits/ring-all-pass.toml`; the solve agrees with the closed
  form to round-off, and its adjoint gives the gradient that tunes it
  ([the circuit adjoint](../docs/methods/circuit-adjoint.md)).
- **A field solve.** `jobs/ring-fdfd.toml` solves a 2 µm ring and its bus by 2D finite
  differences in the frequency domain, from the effective index method: an estimate of the
  spectrum's shape, not the device's 3D performance.

## What our example reproduces

The closed forms are checked against Bogaerts et al.'s equations and against the netlist, and
the measured linewidth against the Lorentzian formula; the free spectral range of the 2D FDFD
ring against the formula with the bent slab's group index (on its 25 nm grid):

::validation components/ring-all-pass-bogaerts components/ring-add-drop-bogaerts components/ring-extremes-bogaerts components/ring-linewidth-bogaerts circuit/ring-fsr-bogaerts components/ring-fsr-fdfd

[circuit_ring_critical](../examples/circuit_ring_critical.rs) tunes a ring's radius and coupling
with L-BFGS-B on the circuit adjoint's gradient until its through port is dark, and checks the
result against the closed forms of resonance and critical coupling:

::example circuit_ring_critical

[ring_q_factor](../examples/ring_q_factor.rs) reproduces Bogaerts et al.'s Fig. 5: a longer ring
stores light longer but loses more per round trip, so at critical coupling the loaded Q peaks at
one length. Its last line measures the linewidth on the spectrum itself and finds it narrower
than the Lorentzian formula: at a finesse near 5 the line is no longer Lorentzian.

::example ring_q_factor

[circuit_fit](../examples/circuit_fit.rs) runs the problem backwards: given an add-drop ring's
spectra, it finds the couplings, loss and radius that made them.

::example circuit_fit

## Try it

**Critical coupling.** In the first chart, with the defaults (a 10 µm ring losing 3 dB/cm), lower
the input coupling until the dip at the marked resonance reaches the floor.

:::answer
The dip touches zero where $\kappa_1^2$ equals the chart's "critical coupling" figure, $1 - a^2$,
a few parts in a thousand here: a 63 µm round trip at 3 dB/cm keeps $a$ just below 1. The
`circuit_ring_critical` example finds the same condition by optimization, starting from 0.05.
:::

**Same depth, different ring.** Set the coupling to twice the critical value, note the depth,
then find the coupling below critical that gives the same depth. What differs?

:::answer
The linewidth. The over-coupled ring loses light through its coupler faster, so its line is
wider and its loaded Q lower; the under-coupled one's is narrower. Measuring the depth alone
can't tell them apart; the width, or the phase of the through field, can.
:::

**Half the intrinsic Q.** In the second chart, read the loaded Q over the intrinsic Q where the
through power reaches zero.

:::answer
One half: at critical coupling the coupler's loss rate equals the ring's own, so
$1/Q = 1/Q_i + 1/Q_c = 2/Q_i$. The dashed curve crosses 0.5 at the marker.
:::

**A drop port.** Give the ring a drop coupler equal to its input coupler, then set the loss to
zero. Where does the light go on resonance?

:::answer
All of it to the drop port: with $r_1 = r_2$ and $a = 1$, $T_\text{drop} = 1$ and the through
port is dark. With loss, the through port goes dark only when $r_1 = r_2 a$, and the drop port
then carries less than all of it.
:::

**Bigger rings.** Double the radius at a fixed coupling. What happens to the free spectral range
and to the Q?

:::answer
The FSR halves ($\lambda^2/(n_g L)$). The Q roughly doubles while the coupler's loss dominates,
since the light then spends twice as long per escape; once the ring's own loss dominates, the
intrinsic Q stays put (loss per unit length, and light travelling at the same speed), so growing
the ring stops helping. `ring_q_factor` finds where that trade peaks.
:::

## Further reading {research}

- Bogaerts et al.'s review, [Silicon microring resonators](https://doi.org/10.1002/lpor.201100017),
  derives every formula on this page and much more: thermal tuning, splitting by backscattering,
  and the design of silicon rings.
- [Little et al. (1997)](https://doi.org/10.1109/50.588673) and
  [Manolatou et al. (1999)](https://doi.org/10.1109/3.784592) for the resonator as coupled modes
  in time, which carries over to photonic-crystal cavities and disks.
- [Vahala (2003)](https://doi.org/10.1038/nature01939) for rings among the other microcavities,
  and [Kippenberg et al. (2018)](https://doi.org/10.1126/science.aan8083) for what nonlinear rings
  do with the light they store.
- In photonoxide: [the components](../docs/methods/components.md) for the closed forms, and the
  lesson on [Bragg gratings](bragg-gratings.md) for the other way to make a waveguide pick out a
  wavelength.

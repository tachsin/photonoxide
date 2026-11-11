---
title: "From components to circuits"
summary: "A component as its S-matrix, ports and waves; a netlist of components solved as one circuit; compact models, a component's response as a sum of poles fitted by vector fitting; Touchstone files; and circuits on the Chip page."
topic: Circuits
level: intermediate
minutes: 30
prerequisites: [directional-coupler, mach-zehnder, ring-resonator]
examples: [compact_mzi_ye, circuit_fit]
circuits: [mzi.toml, ring-add-drop.toml, wdm-demux-4.toml]
methods: [circuits.md, components.md, compact.md, circuit-adjoint.md]
validation:
  - circuit/reciprocity
  - circuit/unitarity
  - circuit/series-waveguides
  - circuit/mzi-closed-form
  - circuit/sub-network-growth
  - compact/vf-paper-rms
  - compact/vf-ring-allpass
  - compact/vf-ring-poles
charts: [mzi-spectrum, circuit-fit, ring-identify]
papers:
  - cite: "K. Kurokawa, IEEE Trans. Microw. Theory Tech. 13, 194 (1965)"
    title: "Power Waves and the Scattering Matrix"
    doi: 10.1109/TMTT.1965.1125964
    year: 1965
    role: origin
    note: "Waves going into and out of a network's ports, normalized so that their squared magnitudes are powers, and the scattering matrix between them: a lossless network's is unitary, a reciprocal one's symmetric."
  - cite: "B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999)"
    title: "Rational approximation of frequency domain responses by vector fitting"
    doi: 10.1109/61.772353
    year: 1999
    role: milestone
    note: "A response sampled in frequency fitted as a sum of poles, in two linear least-squares stages: starting poles relocated by a scaling function's zeros, then the residues; complex starting poles for responses with many resonances."
  - cite: "M. Fiers, T. Van Vaerenbergh, K. Caluwaerts, D. Vande Ginste, B. Schrauwen, J. Dambre, P. Bienstman, J. Opt. Soc. Am. B 29, 896 (2012)"
    title: "Time-domain and frequency-domain modeling of nonlinear optical components at the circuit-level using a node-based approach"
    doi: 10.1364/JOSAB.29.000896
    year: 2012
    role: milestone
    note: "A photonic circuit simulated node by node, in frequency and in time, with nonlinear components among the linear ones: circuit-level models of photonic chips the way electronics simulates its circuits."
  - cite: "Y. Ye, D. Spina, Y. Xing, W. Bogaerts, T. Dhaene, Photonics Res. 6, 560 (2018)"
    title: "Numerical modeling of a linear photonic system for accurate and efficient time-domain simulations"
    doi: 10.1364/PRJ.6.000560
    year: 2018
    role: milestone
    note: "A photonic circuit's S-parameters fitted by vector fitting for time-domain simulation: an MZI over the C and L bands takes 67 poles for −60 dB, the same MZI about a carrier only 8, reproduced here by compact_mzi_ye."
  - cite: "W. Bogaerts, L. Chrostowski, Laser Photonics Rev. 12, 1700237 (2018)"
    title: "Silicon Photonics Circuit Design: Methods, Tools and Challenges"
    doi: 10.1002/lpor.201700237
    year: 2018
    role: review
    note: "The photonic design flow moving from components to circuits, schematic to layout and verification, beside electronic design automation, and what it still lacks: variability, co-simulation with electronics, and compact models."
---

How a chip's devices become one circuit, in the symbols the equations below use. A component is
known by what it does to the waves at its ports; a netlist says which ports meet; and a compact
model stands in for a slow solver with a formula a circuit can evaluate at every wavelength.

::diagram circuit

## 1. What does a component's S-matrix hold?

A component is a box with ports: the end of each guide, each in its mode. A wave goes in at
port $p$ with a complex amplitude $a_p$ and comes out at port $q$ as $b_q$, normalized so that
$\lvert a_p\rvert^2$ and $\lvert b_q\rvert^2$ are powers. A linear component relates them all at
once,

$$
\mathbf{b} = S\,\mathbf{a},
$$

so $S_{qp}$ is the field that goes from $p$ to $q$ and $\lvert S_{qp}\rvert^2$ the share of the
power. A waveguide of length $L$ is a two-port with $S_{21} = S_{12} = e^{i 2\pi n_\text{eff}L/\lambda}$
and nothing reflected; a directional coupler is a four-port whose through and cross fields are
$r$ and $i\kappa$ with $r^2 + \kappa^2 = 1$. Everything else about the component, its geometry and
its fields, is gone: only $S$, at each wavelength, enters the circuit.

### Reciprocity and loss {theory}

Two properties hold for almost every passive photonic component. **Reciprocity**: light from $p$
to $q$ meets the same medium as light from $q$ to $p$, and for media whose permittivity is a
symmetric tensor (no magnet, no moving parts) Lorentz's reciprocity theorem makes
$S = S^\mathsf{T}$. **Energy**: what goes in comes out or is lost, so
$\lVert S\mathbf{a}\rVert \le \lVert\mathbf{a}\rVert$ for every input, the largest singular value of $S$ is at most 1, and a
lossless component's $S$ is unitary, $S^\dagger S = I$
([Kurokawa](https://doi.org/10.1109/TMTT.1965.1125964)). A circuit of reciprocal or lossless
parts inherits the property, which photonoxide checks on netlists of many parts:

::validation circuit/reciprocity circuit/unitarity

## 2. How does a netlist become one circuit?

A netlist names the instances, each a component with its parameters, says which of their ports
meet, and which ports are the circuit's own. Where two ports meet, what leaves one enters the
other. Two waveguides in series are one longer waveguide, their phases multiplied; an MZI is two
couplers and two arms, and its spectrum, solved from the netlist, is the closed form of the
[Mach–Zehnder lesson](mach-zehnder.md):

::chart mzi-spectrum

::validation circuit/series-waveguides circuit/mzi-closed-form

### The connection equations {theory}

Number every port of every instance. The block-diagonal $S_b$ of all the components relates all
the outgoing waves to all the incoming ones, $\mathbf{b} = S_b\mathbf{a}$. A connection between
ports $i$ and $j$ says $a_i = b_j$ and $a_j = b_i$, and an external port's incoming wave is the
circuit's input $x$, so $\mathbf{a} = \Gamma\mathbf{b} + E\mathbf{x}$ with $\Gamma$ the connections'
permutation and $E$ selecting the external ports. Then

$$
(I - S_b\Gamma)\,\mathbf{b} = S_b E\,\mathbf{x},
\qquad
S = E^\mathsf{T}(I - S_b\Gamma)^{-1} S_b E,
$$

one sparse linear system, as many unknowns as ports, whatever loops the circuit holds: a ring's
light circulating, or reflections bouncing between parts. photonoxide factorizes it once per
wavelength and checks it against sub-network growth, which joins the instances one connection at
a time:

::validation circuit/sub-network-growth

## 3. Why a compact model?

A component's S-matrix from a full-wave solver takes seconds to hours a wavelength, and a circuit
needs it at thousands of wavelengths, inside an optimizer's loop, or in time. A **compact model**
gives the same $S$ from a formula: a closed form where one exists, as for the rings and couplers
above, or a rational function fitted to samples. The poles of a rational function are what a
resonance is: a ring's response near a resonance is a pole, $c/(s - a)$, its real part the
resonance's half-width and its imaginary part its frequency. So a sum of poles,

$$
f(s) \approx \sum_n \frac{c_n}{s - a_n} + d,
$$

fits a resonant response with a pole for each resonance. The chart fits an add-drop ring's through
and drop fields together, sharing their poles:

::chart circuit-fit

**Try it.** Lower the poles one at a time from 6. Where does the model break, and why there?

:::answer
At fewer poles than the figure "Resonances in the band": a resonance without its pole can't be
drawn by the others, and the error figure jumps from round-off to tens of percent. Widen the band
or grow the radius and the count needed grows with it.
:::

### Vector fitting {theory}

The poles are unknown, which makes the fit nonlinear; [Gustavsen and
Semlyen](https://doi.org/10.1109/61.772353) made it two linear steps. Start from guessed poles
$\bar a_n$ spread over the band. **Relocate** them: fit a scaling function
$\sigma(s) = \sum_n \tilde c_n/(s - \bar a_n) + 1$ together with $(\sigma f)(s)$ on the same poles,
by linear least squares on $(\sigma f)(s) - \sigma(s) f(s) \approx 0$; the zeros of $\sigma$ are
better poles. Repeat until they stop moving. **Identify** the residues $c_n$ and $d$ by a last
linear least squares with the poles fixed. Responses that share poles, a component's every
$S_{qp}$, share $\sigma$. A pole that comes out with a positive real part, unstable, has its sign
flipped. photonoxide's fit reproduces the paper's own test, and finds a ring's poles where
$r a e^{i\phi} = 1$:

::validation compact/vf-paper-rms compact/vf-ring-allpass compact/vf-ring-poles

The same fit tells a ring from its twin, which its power spectrum can't, because the field's
phase is fitted too:

::chart ring-identify

### How many poles a band needs {research}

[Ye et al.](https://doi.org/10.1364/PRJ.6.000560) fitted an MZI's S-parameters for time-domain
simulation: over 187.5 to 200 THz it took 67 poles to keep the error below −60 dB at frequencies
the fit hadn't seen, and about a carrier, ±150 GHz, only 8. An MZI's arms differ by 50 µm, so its
fringes repeat every 11 nm and each needs its poles; a narrow band holds few of them.
photonoxide's netlist of the same MZI needs 66 and 8:

::example compact_mzi_ye

## 4. What is a Touchstone file?

The text file that S-parameters travel in between tools. An option line gives the units and the
format, `# Hz S RI R 50`; then each frequency is a row: the frequency, then every $S_{qp}$ as a
pair of numbers, real and imaginary (RI), magnitude and angle (MA) or decibels and angle (DB).
The number of ports is in the extension, `.s2p`, `.s4p`. The format comes from microwave
engineering, specified by the IBIS Open Forum; an optical S-matrix is power-normalized as a
microwave one with a real reference resistance is, so its numbers carry over unchanged, and the
50 Ω written with them is nominal. The frequencies increase, so a spectrum sampled in wavelength
is written reversed. photonoxide reads and writes both versions of the format
([compact models](../docs/methods/compact.md)), and a measured component on the Chip page is a
Touchstone file.

## 5. How does the Chip page solve a circuit?

A circuit file names its instances, their parameters and their connections, as a netlist does;
the Chip page draws it, solves it at every wavelength of a sweep, and shows each port's
spectrum. The lesson's circuits open there: the MZI, the add-drop ring, and a four-channel
demultiplexer of rings, each a few lines of TOML. A circuit's parameters can be fitted to spectra
too: an add-drop ring's couplings, loss and radius, found from its through and drop spectra by an
optimizer with the circuit's adjoint gradient and by one without it.

::example circuit_fit

## 6. How does a circuit run in time?

::coming
A compact model whose poles are a real system's, stable and passive, stepped in time by
recursive convolution, the circuit solved node by node at each step (Fiers et al.), with signals
in and eye diagrams out, as 0.8 brings them.

## 7. How photonoxide computes it

- **Components:** `circuit::components`, each an `S(λ)` in closed form or from a device's model
  ([the components](../docs/methods/components.md)).
- **Circuits:** `circuit::Netlist` and `Circuit::spectrum`, the one sparse solve above, and its
  adjoint for gradients ([circuits](../docs/methods/circuits.md),
  [the circuit adjoint](../docs/methods/circuit-adjoint.md)).
- **Compact models:** `compact::fit::vector_fit`, and `CompactModel::fit` for a component's whole
  S-matrix with its error, stability and passivity checked; Touchstone files read and written
  ([compact models](../docs/methods/compact.md)).

## 8. History and today {research}

[Kurokawa](https://doi.org/10.1109/TMTT.1965.1125964) gave microwave engineering its power waves
and scattering matrix in 1965, and photonic circuits took both over unchanged.
[Gustavsen and Semlyen's](https://doi.org/10.1109/61.772353) vector fitting, made for power
lines, became the way to turn sampled responses into models.
[Fiers et al.](https://doi.org/10.1364/JOSAB.29.000896) simulated photonic circuits node by node,
in frequency and in time, and [Ye et al.](https://doi.org/10.1364/PRJ.6.000560) fitted their
S-parameters for time-domain runs. [Bogaerts and Chrostowski](https://doi.org/10.1002/lpor.201700237)
reviewed the design flow this builds: from components to circuits, schematic to layout, and what
it still lacks, variability analysis, co-simulation with electronics and compact models.

::timeline

---
title: "Bragg gratings and mirrors"
summary: "A stack of layers, or a waveguide whose index repeats, reflects one band of wavelengths almost perfectly: the Bragg condition, transfer matrices, the stop band, and coupled modes."
topic: Gratings and filters
level: intermediate
minutes: 35
prerequisites: []
examples: [multilayer_chilwell]
methods: [multilayer.md]
validation:
  - mode/multilayer-bragg
  - mode/multilayer-fresnel
  - mode/multilayer-bound-chilwell
charts: [bragg-reflectance, bragg-bandwidth]
papers:
  - cite: "Lord Rayleigh, Phil. Mag. 24, 145 (1887)"
    title: "On the maintenance of vibrations by forces of double frequency, and on the propagation of waves through a medium endowed with a periodic structure"
    doi: 10.1080/14786448708628074
    year: 1887
    role: origin
    note: "Waves in a medium whose properties repeat: some frequencies cannot travel through it, the first account of a stop band."
  - cite: "W. H. Bragg, W. L. Bragg, Proc. R. Soc. Lond. A 88, 428 (1913)"
    title: "The reflection of X-rays by crystals"
    doi: 10.1098/rspa.1913.0040
    year: 1913
    role: origin
    note: "X-rays reflected by a crystal's planes of atoms, strongly only where the reflections from successive planes add in step: the Bragg condition."
  - cite: "F. Bloch, Z. Phys. 52, 555 (1929)"
    title: "Über die Quantenmechanik der Elektronen in Kristallgittern"
    doi: 10.1007/BF01339455
    year: 1929
    role: origin
    note: "Waves in a periodic potential are plane waves times a periodic function, with bands and gaps: the same mathematics as light in a periodic stack."
  - cite: "H. Kogelnik, C. V. Shank, J. Appl. Phys. 43, 2327 (1972)"
    title: "Coupled-Wave Theory of Distributed Feedback Lasers"
    doi: 10.1063/1.1661499
    year: 1972
    role: milestone
    note: "Two counter-running waves coupled by backward Bragg scattering from a periodic index or gain: the resonances and thresholds of the distributed-feedback laser, and its stop band, inside which an index grating has no mode."
  - cite: "A. Yariv, IEEE J. Quantum Electron. 9, 919 (1973)"
    title: "Coupled-mode theory for guided-wave optics"
    doi: 10.1109/JQE.1973.1077767
    year: 1973
    role: milestone
    note: "Coupled-mode theory for waveguides in general: gratings, directional couplers and mode conversion in one framework."
  - cite: "P. Yeh, A. Yariv, C.-S. Hong, J. Opt. Soc. Am. 67, 423 (1977)"
    title: "Electromagnetic propagation in periodic stratified media. I. General theory"
    doi: 10.1364/JOSA.67.000423
    year: 1977
    role: milestone
    note: "Light in a periodic stack as Bloch waves from the period's transfer matrix: the band structure and the forbidden bands of a multilayer."
  - cite: "K. O. Hill, Y. Fujii, D. C. Johnson, B. S. Kawasaki, Appl. Phys. Lett. 32, 647 (1978)"
    title: "Photosensitivity in optical fiber waveguides: Application to reflection filter fabrication"
    doi: 10.1063/1.89881
    year: 1978
    role: milestone
    note: "Light written into a germanium-doped fibre leaves a permanent index grating: the first fibre Bragg reflector."
  - cite: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984)"
    title: "Thin-films field-transfer matrix theory of planar multilayer waveguides and reflection from prism-loaded waveguides"
    doi: 10.1364/JOSAA.1.000742
    year: 1984
    role: milestone
    note: "Field-transfer matrices that give both a multilayer's guided modes and its reflection: the method photonoxide's multilayer implements."
  - cite: "E. Yablonovitch, Phys. Rev. Lett. 58, 2059 (1987)"
    title: "Inhibited Spontaneous Emission in Solid-State Physics and Electronics"
    doi: 10.1103/PhysRevLett.58.2059
    year: 1987
    role: milestone
    note: "Starting from distributed-feedback lasers and quarter-wave coatings: a dielectric periodic in all three dimensions can have an electromagnetic band gap, and where it overlaps an electronic band edge spontaneous emission is forbidden."
  - cite: "S. John, Phys. Rev. Lett. 58, 2486 (1987)"
    title: "Strong localization of photons in certain disordered dielectric superlattices"
    doi: 10.1103/PhysRevLett.58.2486
    year: 1987
    role: milestone
    note: "Light localized by disorder in a dielectric superlattice, in a pseudogap left by its Bragg resonances: the band gap reached the same year from another direction."
  - cite: "T. Erdogan, J. Lightwave Technol. 15, 1277 (1997)"
    title: "Fiber grating spectra"
    doi: 10.1109/50.618322
    year: 1997
    role: review
    note: "Coupled-mode theory of fibre gratings worked through: uniform, apodized, chirped and tilted gratings, with the closed forms of the uniform one."
  - cite: "H. A. Macleod, Thin-Film Optical Filters, 4th ed., CRC Press (2010)"
    title: "Thin-Film Optical Filters"
    doi: 10.1201/9781420073034
    year: 2010
    role: review
    note: "The thin-film designer's reference: quarter-wave stacks, their admittances and band widths, and the filters built from them."
  - cite: "X. Wang, W. Shi, R. Vafaei, N. A. F. Jaeger, L. Chrostowski, IEEE Photon. Technol. Lett. 23, 290 (2011)"
    title: "Uniform and Sampled Bragg Gratings in SOI Strip Waveguides With Sidewall Corrugations"
    doi: 10.1109/LPT.2010.2103305
    year: 2011
    role: milestone
    note: "Gratings in 500 × 220 nm silicon strips with both sidewalls corrugated, made with one mask by 193 nm lithography: the corrugation's width sets the coupling and the band, and the lithography rounds it to less than drawn."
  - cite: "X. Wang, W. Shi, H. Yun, S. Grist, N. A. F. Jaeger, L. Chrostowski, Opt. Express 20, 15547 (2012)"
    title: "Narrow-band waveguide Bragg gratings on SOI wafers with CMOS-compatible fabrication process"
    doi: 10.1364/OE.20.015547
    year: 2012
    role: milestone
    note: "Bragg gratings in silicon waveguides made in a CMOS-compatible process, with narrow bands from weak corrugations."
  - cite: "D. Oser, D. Pérez-Galacho, X. Le Roux, S. Tanzilli, L. Vivien, L. Labonté, É. Cassan, C. Alonso-Ramos, Opt. Lett. 45, 5784 (2020)"
    title: "Silicon subwavelength modal Bragg grating filters with narrow bandwidth and high optical rejection"
    doi: 10.1364/OL.394455
    year: 2020
    role: current
    note: "A grating that couples the forward mode to a backward mode of another order, in a subwavelength-structured guide: narrow bands with deep rejection."
  - cite: "R. Cheng, L. Chrostowski, J. Lightwave Technol. 39, 712 (2021)"
    title: "Spectral Design of Silicon Integrated Bragg Gratings: A Tutorial"
    doi: 10.1109/JLT.2020.3035372
    year: 2021
    role: review
    note: "How silicon gratings are designed today: coupling from the corrugation, apodization, phase shifts and the fabrication's limits."
---

A Bragg mirror is a stack of thin layers whose index alternates, high, low, high, low. Each
boundary reflects a few percent of the light. At one wavelength, set by the layers' thickness,
all those small reflections come back in step and add up, and a stack of a few dozen layers
reflects almost everything. The same idea, an index that repeats along a waveguide, makes the
grating filters of fibres and of silicon chips, the mirrors of distributed-feedback and
surface-emitting lasers, and, extended to two and three dimensions, photonic crystals.

## What it does

The chart starts from a classic coating: eight pairs of layers of index 2.3 and 1.38 (roughly a
high-index oxide and magnesium fluoride) on glass, each a quarter wave thick at 550 nm. Around
550 nm the stack reflects nearly everything: that is its **stop band**. Outside it, the reflection
ripples between small values. Add pairs and the band's top rises towards 1 and its edges sharpen;
raise the contrast and the band widens; change the period and the whole band moves.

::chart bragg-reflectance

Three numbers describe a Bragg mirror: where the band is (the **Bragg wavelength**, set by the
period), how wide it is (set by the index contrast), and how much it reflects (set by the contrast
and the number of periods together).

## The physics {theory}

### One interface, and many

At normal incidence a boundary from index $n_1$ to $n_2$ reflects the field
$r = (n_1 - n_2)/(n_1 + n_2)$, Fresnel's coefficient: 0.25 for 2.3 to 1.38, so 6 % of the power.
The sign alternates from boundary to boundary, high to low then low to high. A layer of
thickness $d$ and index $n$ delays the light by a phase $k n d$ each way, $k = 2\pi/\lambda$, so the
reflections from the two boundaries of one layer return with a phase difference of $2 k n d$ plus
the sign's $\pi$. They add in step when $2 k n d = \pi$, a layer a quarter wave thick, $n d = \lambda/4$.
A whole period of thickness $\Lambda = d_H + d_L$ then has the optical thickness of half a wavelength,

$$
n_H d_H + n_L d_L = \frac{\lambda_B}{2}, \qquad
\lambda_B = 4 n_H d_H = 4 n_L d_L = \frac{4 \Lambda\thinspace n_H n_L}{n_H + n_L}
$$

for quarter-wave layers: the **Bragg condition**, the optical cousin of the Braggs' condition for
X-rays reflected by a crystal's planes ([Bragg and Bragg, 1913](https://doi.org/10.1098/rspa.1913.0040)).

### Transfer matrices

Adding reflections one by one soon gets out of hand, because the light bounces back and forth
between all the boundaries. Transfer matrices do the bookkeeping exactly. In each layer the
tangential fields at its two faces are related by a $2 \times 2$ matrix
([Chilwell and Hodgkinson, 1984](https://doi.org/10.1364/JOSAA.1.000742), Eq. 10),

$$
M_j = \begin{pmatrix} \cos\Phi_j & -\frac{i}{\gamma_j}\sin\Phi_j \cr -i\gamma_j\sin\Phi_j & \cos\Phi_j \end{pmatrix},
\qquad \Phi_j = k n_j d_j ,
$$

with $\gamma_j = n_j$ at normal incidence, and the stack's matrix $M$ is the product of its
layers'. Light from a cover of index $n_0$ onto a substrate of index $n_s$ is then reflected with

$$
r = \frac{n_0 m_{11} + n_0 n_s m_{12} - m_{21} - n_s m_{22}}{n_0 m_{11} + n_0 n_s m_{12} + m_{21} + n_s m_{22}} ,
$$

their Eq. 13 at normal incidence, and the reflectance is $R = |r|^2$. Nothing is approximated:
any number of layers, any indices, lossy or not.

### The quarter-wave stack, exactly

At $\lambda_B$ every layer is a quarter wave, $\Phi_j = \pi/2$, and each matrix loses its
diagonal: $M_j = \begin{pmatrix} 0 & -i/n_j \cr -i n_j & 0 \end{pmatrix}$. A high-low pair
multiplies to a diagonal matrix,

$$
M_H M_L = \begin{pmatrix} -n_L/n_H & 0 \cr 0 & -n_H/n_L \end{pmatrix},
$$

and $N$ pairs to its $N$-th power. Put into $r$, with $\rho = n_H/n_L$,

$$
r = \frac{1 - q}{1 + q}, \quad q = \frac{n_s}{n_0}\rho^{2N}, \qquad
R(\lambda_B) = \left(\frac{1 - q}{1 + q}\right)^2 .
$$

Each pair multiplies $q$ by $\rho^2$, so the light that gets through, $1 - R \approx 4/q$, falls
geometrically with the number of pairs. When cover and substrate match, $R = \tanh^2(N \ln\rho)$.

### Bloch waves and the stop band

An endless stack is periodic, and its waves are Bloch waves: a wave that crosses one period comes
out multiplied by $e^{iK\Lambda}$, the Bloch phase, as electrons do in a crystal
([Bloch, 1929](https://doi.org/10.1007/BF01339455)). So $e^{iK\Lambda}$ is an eigenvalue of the
period's matrix $M_H M_L$, whose determinant is 1, and the eigenvalues of such a matrix satisfy
$\cos K\Lambda = \tfrac{1}{2}\operatorname{tr}(M_H M_L)$
([Yeh, Yariv and Hong, 1977](https://doi.org/10.1364/JOSA.67.000423)). Multiplying out the two
layer matrices,

$$
\cos K\Lambda = \cos\Phi_H \cos\Phi_L - \frac{1}{2}\left(\frac{n_H}{n_L} + \frac{n_L}{n_H}\right)\sin\Phi_H \sin\Phi_L .
$$

Where the right side lies between −1 and 1, $K$ is real and light travels through. Where it falls
below −1, $K$ has an imaginary part and the wave dies away into the stack: the **stop band**. For
quarter-wave layers $\Phi_H = \Phi_L = \tfrac{\pi}{2}\omega/\omega_0$, with $\omega_0$ the Bragg
frequency. Setting the right side to −1 gives
$\sin\Phi = 2\sqrt\rho/(1 + \rho)$, and since $(2\sqrt\rho)^2 + (\rho - 1)^2 = (\rho + 1)^2$, the
band's edges are where $\cos\Phi = \pm(\rho - 1)/(\rho + 1)$. Writing $\Phi = \tfrac{\pi}{2}(1 + x)$,
$\cos\Phi = -\sin(\pi x/2)$, and the band spans

$$
\frac{\Delta\omega}{\omega_0} = \frac{4}{\pi}\arcsin\frac{n_H - n_L}{n_H + n_L} .
$$

The band's width is set by the contrast alone; more pairs only make the stack reflect more of
what lies inside it. This is the dashed curve of the second chart.

### Weak gratings: coupled modes

A waveguide grating has a small contrast: its effective index wanders by a few percent or less.
Then a simpler picture works, two waves, forward and backward, each slowly changing as the grating
feeds one into the other ([Kogelnik and Shank, 1972](https://doi.org/10.1063/1.1661499);
[Yariv, 1973](https://doi.org/10.1109/JQE.1973.1077767)). For an index
$n(z) = \bar n + \delta n_1 \cos(2\pi z/\Lambda)$ the waves exchange power at the rate
$\kappa = \pi\delta n_1/\lambda$ per unit length. A grating of length $L$ reflects at its centre

$$
R = \tanh^2(\kappa L),
$$

and the infinite grating's stop band is where the detuning
$\delta = 2\pi\bar n/\lambda - \pi/\Lambda$ satisfies $|\delta| < \kappa$, a width
$\Delta\lambda = \kappa\lambda^2/(\pi n_g)$ in wavelength (the uniform grating of
[Erdogan, 1997](https://doi.org/10.1109/50.618322)). A square grating stepping by
$\Delta n = n_H - n_L$ with equal layers has the first Fourier component $\delta n_1 = 2\Delta n/\pi$,
so $\kappa = 2\Delta n/\lambda$, and with $L = N\Lambda = N\lambda/(2\bar n)$,

$$
\kappa L = \frac{N\Delta n}{\bar n}, \qquad \frac{\Delta\omega}{\omega_0} \approx \frac{2\Delta n}{\pi\bar n} .
$$

These are the exact results' first terms: $\ln\rho \approx \Delta n/\bar n$, and
$\arcsin x \approx x$. Coupled modes are what designers use for fibre and waveguide gratings, where
they are accurate; the transfer matrices are exact at any contrast.

::chart bragg-bandwidth

The solid curve finds the band's edges numerically, from one period's transmission $t$ by the
transfer matrices: for a symmetric, lossless period the half trace of its matrix is
$\operatorname{Re}(1/t)$, so $\cos K\Lambda = \operatorname{Re}(1/t)$. It lies on the closed form at
every contrast, and the coupled-mode line leaves them only at contrasts no waveguide grating has.

What the band doesn't tell you is how much a grating of finite length reflects. When $\kappa L$ is
small, a short, weak grating reflects weakly, and over a range set by its length,
$\Delta\lambda \sim \lambda^2/(n_g L)$, rather than by its contrast. The first chart shows it with
few pairs and a small contrast.

## History {research}

Rayleigh worked out in 1887 that a wave in a medium whose properties repeat cannot travel at
certain frequencies, the first stop band. In 1913 the Braggs explained why crystals reflect X-rays
only at certain angles, the reflections of successive planes adding in step, and in 1929 Bloch
gave the mathematics of waves in any periodic structure, written for electrons. Thin-film
interference coatings turned the quarter-wave stack into the workhorse mirror of optics
([Macleod](https://doi.org/10.1201/9781420073034) gathers that craft). Gratings in waveguides came
with integrated optics and lasers: Kogelnik and Shank's coupled-wave theory of 1972 explained the
distributed-feedback laser, Yariv generalized coupled modes to guided-wave optics in 1973, and
Yeh, Yariv and Hong described periodic stacks by Bloch waves in 1977. In 1978 Hill and colleagues
found that light could write a permanent grating into a fibre's core, the start of the fibre Bragg
grating. In 1984 Chilwell and Hodgkinson put a multilayer's guided modes and its reflection into
one transfer-matrix theory, the one photonoxide implements. In 1987 Yablonovitch, starting from the
quarter-wave coating and the distributed-feedback laser, asked what a dielectric periodic in all
three dimensions would do, and John, starting from light localized by disorder, reached a gap from
another direction: the photonic band gap, of which a Bragg mirror is the one-dimensional case. Erdogan's 1997 paper became the reference for
grating spectra, and silicon photonics brought gratings onto chips: by 2011 silicon strips with
corrugated sidewalls made with a single lithography mask, and narrow-band gratings made in a
CMOS-compatible process by 2012.

::timeline

## Today {research}

In silicon, a Bragg grating is usually a waveguide whose width is corrugated, and its coupling
coefficient follows from the corrugation's depth and the mode's overlap with it: a wider
corrugation couples more, and widens the band. What is drawn is not what is made. In
[Wang et al. (2011)](https://doi.org/10.1109/LPT.2010.2103305)'s strips, corrugated on both sides
by 193 nm lithography, the corners came out rounded, and the band of a grating drawn with a
20 nm square corrugation matched the one simulated for a 5 nm square corrugation. The design
questions are spectral: how narrow a band, how deep a rejection, how low the sidelobes. Narrow
bands need weak coupling over a long length ([Wang et al., 2012](https://doi.org/10.1364/OE.20.015547)
made such gratings in a CMOS-compatible process), and then the fabrication noise in the
corrugation matters; apodization, varying the coupling
along the grating, suppresses sidelobes; a phase shift in the middle turns a grating into a
resonator; and gratings that couple the forward mode into a backward mode of another order
separate the reflection from the input, as in subwavelength-structured modal gratings
([Oser et al., 2020](https://doi.org/10.1364/OL.394455)). Cheng and Chrostowski's tutorial
([2021](https://doi.org/10.1109/JLT.2020.3035372)) surveys this design space. Open problems
include gratings that keep their band under fabrication variation, and the inverse design of
gratings for arbitrary spectra.

## How photonoxide computes it

`mode::multilayer::Multilayer` is Chilwell and Hodgkinson's transfer-matrix theory: a cover, any
number of films with complex indices, and a substrate. `Multilayer::reflection` gives $r$, $t$, $R$
and $T$ for a plane wave of either polarization at any angle, from the matrices above (their
Eqs. 13 to 16), and the same matrices give the stack's guided modes and leaky waves. It is exact:
no grid. Both charts call it, at normal incidence, where the two polarizations agree. See
[the multilayer](../docs/methods/multilayer.md).

A waveguide grating can be pictured as a stack of the effective indices of its wide and narrow
sections; that one-dimensional picture is what the first chart draws when you give it such indices.
It leaves out the mode's change of shape between sections and the light scattered out of the guide,
so it is an estimate of where the band is and how wide, not of a device's performance.

## What our example reproduces

The transfer matrices are checked against the quarter-wave stack's closed form derived above, to
round-off; against Fresnel's equations at one interface; and, for the guided modes, against
Chilwell and Hodgkinson's own tables:

::validation mode/multilayer-bragg mode/multilayer-fresnel mode/multilayer-bound-chilwell

[multilayer_chilwell](../examples/multilayer_chilwell.rs) uses the same matrices the other way
round, to find the bound modes and the leaky waves of a four-layer guide, and checks every number
of the paper's Tables 2 and 3:

::example multilayer_chilwell

## Try it

**More pairs.** In the first chart, set the pairs to 2, then 4, 8 and 16. What happens to the
reflectance at the Bragg wavelength, and to the band's width?

:::answer
The peak climbs towards 1, by the factor $\rho^2$ in $q$ for each pair, and the "R at λ_B" figures,
transfer matrices and closed form, agree at every step. The band's width hardly changes once a
few pairs are there: it is set by the contrast. Its edges sharpen, and the ripples outside crowd
closer.
:::

**Less contrast.** Set the high index to 1.6 and the low to 1.5. How many pairs bring the peak
back above 99 %?

:::answer
With cover and substrate matched, $R = \tanh^2(N\ln\rho)$, $\ln(1.6/1.5) \approx 0.065$, and
$\tanh^2 x > 0.99$ needs $x > 3$: about 46 pairs. The air cover and glass substrate here reflect
a little of their own, and 44 pairs do it. The band is now much narrower: the
"stop band" figure follows $\arcsin((n_H - n_L)/(n_H + n_L))$.
:::

**Move the band.** Change the period. Where does the band go?

:::answer
It moves in proportion: $\lambda_B = 4\Lambda n_H n_L/(n_H + n_L)$. A coating designer sets the
period for the colour to reflect, and the contrast for how wide a range of colours.
:::

**A waveguide grating.** Picture a silicon wire whose effective index alternates between 2.45 and
2.35, with a 320 nm period, in a guide of index 2.4:

::chart bragg-reflectance{high=2.45, low=2.35, cover=2.4, substrate=2.4, period=320nm, pairs=60, from=1450nm, to=1650nm}

Where is the band, how wide is it, and how many periods make it reflect 90 %?

:::answer
Near 1535 nm, a few tens of nanometres wide (the "stop band" figure). With
$\kappa L = N\Delta n/\bar n$, 90 % needs $\tanh(\kappa L) \approx 0.95$, $\kappa L \approx 1.8$:
about 45 periods, some 15 µm of grating. Real gratings use weaker corrugations than this for
narrower bands, and are longer.
:::

**When length wins.** Back in the first chart, set the indices to 1.6 and 1.5, the cover and the
substrate to 1.55, and the pairs to 5. Compare the reflection with the band edges the figures give,
then raise the pairs to 40.

:::answer
With 5 pairs, $\kappa L = N\Delta n/\bar n \approx 0.3$: the peak reflects about a tenth, and the
lobe spills well past the band edges, its width set by the grating's length. With 40 pairs,
$\kappa L \approx 2.6$: the peak nears 98 %, and the lobe's flanks settle onto the edges, the
width now set by the contrast.
:::

## Further reading {research}

- [Erdogan (1997)](https://doi.org/10.1109/50.618322) works through coupled-mode theory for
  uniform, apodized and chirped gratings, and is where most grating designers start.
- [Yeh, Yariv and Hong (1977)](https://doi.org/10.1364/JOSA.67.000423) for periodic stacks as
  Bloch waves, at any angle and polarization.
- [Macleod's book](https://doi.org/10.1201/9781420073034) for thin-film filters built from
  quarter-wave stacks.
- [Cheng and Chrostowski (2021)](https://doi.org/10.1109/JLT.2020.3035372) for gratings in silicon
  waveguides.
- [Yablonovitch (1987)](https://doi.org/10.1103/PhysRevLett.58.2059) and
  [John (1987)](https://doi.org/10.1103/PhysRevLett.58.2486) for the step from one dimension to three.
- In photonoxide: [the multilayer](../docs/methods/multilayer.md), and the lesson on the
  [ring resonator](ring-resonator.md), the other way to make a waveguide pick out a wavelength.

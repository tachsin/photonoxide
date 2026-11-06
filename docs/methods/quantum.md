---
title: "Linear-optical statistics"
module: quantum
summary: "Photons on a circuit's ports and what comes out: the S-matrix as the modes' transformation with its loss dilated, permanents by Ryser's and Glynn's formulas, partly distinguishable photons, an exact sampler, Hong–Ou–Mandel dips through S(ω), and the Reck and Clements meshes."
order: 31
papers:
  - cite: "S. Aaronson, A. Arkhipov, Theory Comput. 9, 143 (2013)"
    doi: 10.4086/toc.2013.v009a004
  - cite: "H. J. Ryser, Combinatorial Mathematics, Carus Mathematical Monographs 14, MAA (1963)"
    doi: 10.5948/UPO9781614440147
  - cite: "P. H. Lundow, K. Markström, J. Comput. Phys. 455, 110990 (2022)"
    doi: 10.1016/j.jcp.2022.110990
  - cite: "D. G. Glynn, Eur. J. Combin. 31, 1887 (2010)"
    doi: 10.1016/j.ejc.2010.01.010
  - cite: "P. Clifford, R. Clifford, Proc. 29th ACM-SIAM Symp. Discrete Algorithms (SODA 2018), 146"
    doi: 10.1137/1.9781611975031.10
  - cite: "C. K. Hong, Z. Y. Ou, L. Mandel, Phys. Rev. Lett. 59, 2044 (1987)"
    doi: 10.1103/PhysRevLett.59.2044
  - cite: "M. Reck, A. Zeilinger, H. J. Bernstein, P. Bertani, Phys. Rev. Lett. 73, 58 (1994)"
    doi: 10.1103/PhysRevLett.73.58
  - cite: "W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walmsley, Optica 3, 1460 (2016)"
    doi: 10.1364/OPTICA.3.001460
  - cite: "A. Peruzzo et al., Science 329, 1500 (2010)"
    doi: 10.1126/science.1193515
  - cite: "F. Mezzadri, Notices Amer. Math. Soc. 54, 592 (2007)"
    doi: ""
    ref: mezzadri-2007
  - cite: "D. Blackman, S. Vigna, ACM Trans. Math. Softw. 47, 36 (2021)"
    doi: 10.1145/3460772
validation:
  - quantum/permanent-ones
  - quantum/permanent-derangements
  - quantum/permanent-formulas
  - quantum/permanent-threads
  - quantum/hom-visibility
  - quantum/hom-balanced
  - quantum/patterns-sum
  - quantum/distinguishable-classical
  - quantum/partly-distinguishable
  - quantum/full-bunching
  - quantum/dilation
  - quantum/hom-dip-gaussian
  - quantum/hom-delay-line
  - quantum/mesh-reconstruction
  - quantum/mesh-depth
  - quantum/clements-fig3-size
  - quantum/clements-fig3-loss
  - quantum/peruzzo-correlations
  - quantum/peruzzo-classical-bound
  - quantum/sampler-convergence
  - quantum/coupler-hom-order
---

A linear circuit does to photons what it does to light: it mixes the ports' modes by its
S-matrix. Everything about photons through a circuit, from two photons on a coupler to a boson
sampler, follows from that matrix, its loss and its change with frequency. The module
`photonoxide::quantum` takes a [circuit](circuits.md)'s S-matrix, or any matrix, and computes
the photons' statistics exactly, up to about 30 single photons. It is the first part of the
0.6.3 milestone's plan, [docs/plans/quantum.md](../plans/quantum.md) (Q1).

## The S-matrix as a mode transformation

photonoxide's S-matrices are power-normalized in the $e^{-i\omega t}$ convention
([conventions](conventions.md)), so the same matrix transforms the ports' annihilation operators:
the modes leaving a lossless circuit are $\hat b = S \hat a$. Every port is an input mode (light
going in) and an output mode (light coming out), so an $n$-port's $S$ is $n \times n$, and a
reflection is as much a path for a photon as a transmission.

A lossy $S$ isn't unitary, and $\hat b = S \hat a$ alone would break the commutators
$[\hat b_q, \hat b_r^\dagger] = \delta_{qr}$. The loss lets the vacuum in through noise modes
$\hat c$:

$$
\hat b = S \hat a + L \hat c, \qquad L L^\dagger = I - S S^\dagger .
$$

The photons the loss takes leave through loss modes $\hat e = K \hat a + \dots$, with
$K^\dagger K = I - S^\dagger S$. The transfer from the inputs to the outputs and the loss modes
together, $T$, the $n$ rows of $S$ above the rows of $K$, has orthonormal columns: it is
an isometry, and the probabilities of every pattern over the ports and the loss modes sum to 1. `Transfer` builds $K$ and $L$ from the eigenvectors of
$I - S^\dagger S$ and $I - S S^\dagger$ (Jacobi's method), one row or column per channel that
loses power, each scaled by the square root of its eigenvalue; a lossless channel (eigenvalue
below $10^{-13}$) gets none. A matrix with gain, an eigenvalue of $S^\dagger S$ above
$1 + 10^{-9}$, is refused: vacuum noise can't dilate it. `Transfer::distribution` then sums the
loss modes out, so a pattern with fewer photons at the ports is one where some were lost.

## Output probabilities and the permanent

$n$ photons with occupations $s_t$ of the input modes leave with occupations $m_r$ of the output
modes with probability

$$
P = \frac{\lvert \operatorname{Per}(M) \rvert^2}{\prod_t s_t! \prod_r m_r!},
$$

where $M$ is $T$'s $n \times n$ submatrix of the output photons' rows and the input photons'
columns, each mode repeated as often as it is occupied (Aaronson & Arkhipov 2013,
doi:10.4086/toc.2013.v009a004). The permanent is the determinant without its signs,
$\operatorname{Per}(A) = \sum_\sigma \prod_i a_{i,\sigma(i)}$, and has no known polynomial
algorithm. Two formulas bring its cost from $n \cdot n!$ to about $n 2^n$:

- **Ryser's**, read in Lundow & Markström 2022 (doi:10.1016/j.jcp.2022.110990, Eq. 3), since
  Ryser's book (doi:10.5948/UPO9781614440147) is out of print:
  $\operatorname{Per}(A) = (-1)^n \sum_{S \subseteq [n]} (-1)^{\lvert S \rvert} \prod_i \sum_{j \in S} a_{ij}$;
- **Glynn's** (doi:10.1016/j.ejc.2010.01.010, Theorem 2.1):
  $\operatorname{Per}(A) = 2^{1-n} \sum_\delta \left(\prod_k \delta_k\right) \prod_j \sum_i \delta_i a_{ij}$
  over $\delta \in \lbrace \pm 1 \rbrace^n$ with $\delta_1 = 1$, half Ryser's terms.

Both run in Gray-code order, so consecutive terms differ by one column (Ryser) or one sign
(Glynn) and each costs $n$ operations. The sum is cut into parts fixed by $n$ alone (256 of them
from $n = 12$), each part starting from its own sums computed directly and summed in order with
Kahan's compensation, as Lundow and Markström recommend for precision; the parts run on rayon's
threads and are added in order. The threads only decide which part runs where, so a permanent
is the same bit for bit on any number of them (`quantum/permanent-threads`). `permanent` uses
Glynn's; `permanent_by` takes either, or the definition up to $n = 10$ for checks. A $20 \times 20$
permanent takes 20 ms on one thread by Glynn's formula (38 ms by Ryser's, with twice the terms);
each added photon doubles it, so $n = 30$ is about half a minute on one thread, and the limit,
$n = 40$, hours on many.

## Distinguishable and partly distinguishable photons

Photons that differ in something the detectors don't see (their spectra, arrival times or
polarizations) interfere only as far as those internal states overlap. Photon $j$ in input mode
$c_j$ with internal state $\lvert\psi_j\rangle$ gives, expanding each photon's creation operator
over an internal basis and summing the internal labels out at the detectors,

$$
P = \frac{1}{\prod_r m_r!\thinspace \mathcal N} \sum_{\sigma \in S_n} \Big( \prod_j G_{\sigma(j), j} \Big)
\operatorname{Per}\big( M \circ \overline{M_\sigma} \big),
\qquad G_{ij} = \langle \psi_i \vert \psi_j \rangle ,
$$

where $M \circ \overline{M_\sigma}$ is the entrywise product of $M$ with the conjugate of $M$
whose columns $\sigma$ permutes, and
$\mathcal N = \sum_\sigma \prod_j G_{\sigma(j), j}$ over the $\sigma$ that keep every photon in its
input mode is the input state's norm (1 for photons in different modes). The overlaps' Gram
matrix $G$ of all ones gives back $\lvert \operatorname{Per} M \rvert^2 / (\prod s_t! \prod m_r!)$,
by the identity $\sum_\sigma \operatorname{Per}(A \circ B_\sigma) = \operatorname{Per}(A) \operatorname{Per}(B)$;
$G = I$ leaves only $\sigma = \text{id}$ and the probabilities of classical particles,
$\operatorname{Per}(\lvert M \rvert^2) / \prod m_r!$. `Photons` names the three cases, and the
sum over permutations takes up to eight photons. `quantum/partly-distinguishable` checks it
against the route it summarizes: each photon expanded into a two-dimensional internal space,
$U \otimes I_2$ on six modes, its internal modes summed out at the end.

## Two-photon interference with spectra

Two photons on a beam splitter of reflectance $R$ and transmittance $T$ leave by both outputs
with probability $(R - T)^2$ when they are indistinguishable and $R^2 + T^2$ when they aren't, a
visibility $V = 2RT/(R^2 + T^2)$ (Hong, Ou & Mandel 1987, doi:10.1103/PhysRevLett.59.2044). With
spectra and a circuit whose $S$ depends on frequency, photon 1 (spectral amplitude $\varphi_1$)
entering port $i_1$ delayed by $\tau$ and photon 2 ($\varphi_2$) entering $i_2$ reach ports $o_1$
at $\omega$ and $o_2$ at $\omega'$ with amplitude $f(\omega, \omega') = A(\omega) B(\omega') + D(\omega) C(\omega')$:

$$
A = \varphi_1 e^{i\omega\tau} S_{o_1 i_1}, \quad B = \varphi_2 S_{o_2 i_2}, \quad
C = \varphi_1 e^{i\omega\tau} S_{o_2 i_1}, \quad D = \varphi_2 S_{o_1 i_2},
$$

and the coincidence probability $\iint \lvert f \rvert^2 \thinspace d\omega \thinspace d\omega'$ separates into single
integrals,

$$
P(\tau) = \int \lvert A \rvert^2 \int \lvert B \rvert^2 + \int \lvert C \rvert^2 \int \lvert D \rvert^2
+ 2 \operatorname{Re}\Big( \int A \bar D \thinspace d\omega \int B \bar C \thinspace d\omega \Big),
$$

the last term the interference, which a long delay averages away. `hom_dip` evaluates each
integral by the trapezoidal rule over the frequencies of the circuit's own sweep
(`Circuit::spectrum`), unevenly spaced as they may be, with the photons' spectra normalized on
the same grid. For two equal transform-limited Gaussian photons of spectral width $\sigma$ through
a frequency-independent coupler it gives $R^2 + T^2 - 2RT e^{-\sigma^2\tau^2}$ to round-off
(`quantum/hom-dip-gaussian`); a guide of group index $n_g$ and length $L$ before one input moves
the dip to $\tau = n_g L / c$, where the coincidences vanish (`quantum/hom-delay-line`).

## The exact sampler

Clifford and Clifford's algorithm B (doi:10.1137/1.9781611975031.10) draws exact samples of the
output pattern in $O(n 2^n + m n^2)$ for $n$ photons in $m$ modes. It puts the input photons'
columns of $T$ in a random order $\alpha$ and draws the photons' output modes one at a time, the
$k$-th with weights $\lvert \operatorname{Per} T[r_1, \dots, r_{k-1}, x; \alpha_1, \dots, \alpha_k] \rvert^2$
over the modes $x$. Each weight is a Laplace expansion along the last row, and the minors of all
of them come from one pass of Glynn's formula with prefix and suffix products (their Lemma 3.2).
`sample` takes an explicit seed; sample $i$ draws from its own stream of a xoshiro256**
generator (Blackman & Vigna 2021, doi:10.1145/3460772) seeded by the seed and $i$, so the samples
are the same on any number of threads. Its columns must be orthonormal (a unitary, or a lossy
circuit's isometry: `Transfer::sample` drops the loss modes from each sample). Several photons in
one input mode are allowed: the unit tests compare the sampler with the exact distribution for
(2, 0, 1, 0, 0) as well as for single photons.

## Haar-random unitaries and the Reck and Clements meshes

`haar_unitary` draws a Haar-random unitary as the $Q$ of a matrix of independent complex
Gaussians, its QR factorization by Gram–Schmidt with $R$'s diagonal positive, which makes the
distribution the Haar measure (F. Mezzadri, Notices Amer. Math. Soc. 54, 592 (2007),
ref:mezzadri-2007).

`Mesh` decomposes a unitary into $N(N-1)/2$ cells, each Clements et al.'s
$T_{m,m+1}(\theta, \phi)$: a phase $\phi$ on mode $m$, then a beam splitter of reflectivity
$\cos\theta$ (their Eq. 1), and the output phases $D$, so that $U = D \prod T$ (their Eq. 2).
Reck et al.'s triangle (doi:10.1103/PhysRevLett.73.58) nulls $U$ row by row from the bottom with
$T^{-1}$ from the right; Clements et al.'s rectangle (doi:10.1364/OPTICA.3.001460) nulls
successive diagonals, alternating $T^{-1}$ from the right and $T$ from the left (their Fig. 2),
and moves the left ones through $D$ by $T^{-1}(\theta, \phi) D = D' T(\theta, \phi')$ with
$\phi' = \arg(-d_m/d_{m+1})$, $d'_m = -e^{-i\phi} d_{m+1}$ and $d'_{m+1} = d_{m+1}$ (their Eq. 5).
The optical depth, the most cells a path crosses, is $2N - 3$ for the triangle and $N$ for the
rectangle. `Mesh::lossy` loses the same power on both outputs of every cell, Clements et al.'s
model of unbalanced loss, and `fidelity` is their Eq. 6,
$\lvert \operatorname{tr}(U^\dagger U_\text{exp}) \rvert^2 / (N \operatorname{tr}(U_\text{exp}^\dagger U_\text{exp}))$.
Programming a mesh as a circuit of couplers and phase shifters, with self-configuration and
error correction, is 0.8's.

## Coupled waveguide arrays

`coupled_array` is the transfer of $N$ evanescently coupled guides by coupled-mode theory,
$U = e^{iHz}$ with $H$ tridiagonal and the couplings off its diagonal, the continuous-time
quantum walk of Peruzzo et al. 2010 (doi:10.1126/science.1193515, Eqs. 1–3; their Hamiltonian has
the opposite sign, which conjugates $U$ and leaves every probability).

## What it is checked against

The analytic cases hold to round-off: the permanent of the all-ones matrix is $n!$ and that of
$J_n - I_n$ the number of derangements up to $n = 14$; both formulas agree with the definition to
$1.5 \times 10^{-14}$ relative for $n \le 8$; the HOM coincidences and visibility of an ideal
coupler from 0.1 to 0.9; the 126 patterns of 4 photons in 6 modes sum to 1, and so do a lossy
coupler's patterns with its loss modes; $n$ photons all in one mode are $n!$ times as likely as
distinguishable ones; the dilation of a lossy circuit (a 2 dB/cm guide and a coupler with 1 dB
of excess loss) is exact to $10^{-17}$; the meshes give their unitaries back to $4 \times 10^{-16}$.

**Clements et al.'s Fig. 3.** Their figure gives the mean fidelity of both designs over 500
Haar-random unitaries with a fixed loss per beam splitter, against the number of modes at
0.2 dB (left) and against the loss for 20 modes (right). The figure is a raster, read here
marker by marker at each marker's centroid (one pixel is 0.00034 in fidelity on the left, 0.00091
on the right; the triangles' box centres lie 0.0007 higher than their centroids on the left and
0.002 on the right, and the means computed fall on the centroids). Our means over 500 other unitaries:

| Case | Paper (read) | photonoxide | Difference |
|---|---|---|---|
| Reck, 21 modes, 0.2 dB | 0.96962 | 0.96959 | 0.00003 |
| Reck, 30 modes, 0.2 dB | 0.93507 | 0.93512 | 0.00005 |
| Reck, 40 modes, 0.2 dB | 0.88470 | 0.88465 | 0.00005 |
| Reck, 50 modes, 0.2 dB | 0.82514 | 0.82546 | 0.00032 |
| Reck, 20 modes, 0.5 dB | 0.84260 | 0.84190 | 0.00070 |
| Reck, 20 modes, 1 dB | 0.53734 | 0.53624 | 0.00110 |
| Clements, 20 modes, 0.5 dB | 0.99498 | 0.99456 | 0.00042 |
| Clements, 20 modes, 1 dB | 0.97606 | 0.97599 | 0.00007 |

The largest difference over the left panel's 30 sizes is 0.00046, against a tolerance of 0.001
(one pixel and three standard errors of two means: ours is 0.00015 at 50 modes); over the right
panel's 38 markers it is 0.0022 (Reck's design at 0.85 dB), against 0.004 (one pixel and three
standard errors, ours 0.00073 at 1 dB). The two means come from different unitaries, so they
differ by their sampling; the reading's pixel is the rest.

**Peruzzo et al.'s quantum walk.** Their array is 21 guides with $C$ = 5 per mm over the 700 µm
coupling region plus an effective 82 µm from the spreading regions (p. 1501), so
$Cz = 3.91$. With those, their Eq. 4 for the two-photon correlation $\Gamma_{q,r}$ is the
permanent formula written out for two photons (`quantum/peruzzo-correlations`, to $10^{-17}$),
and their classical bound, Eq. 5, holds at every pair for distinguishable photons, as they report,
while indistinguishable photons violate it at 70 pairs for inputs (0, 1) and 56 for (−1, 1)
(`quantum/peruzzo-classical-bound`). The features they describe appear: for inputs (0, 1), the
photons leave on opposite sides of the array (one at guide 0 or below, the other at 1 or above)
with probability 0.50 when distinguishable and 0.27 when not, the off-diagonal lobes their
Fig. 3 shows vanishing; for (−1, 1), the coincidence of guides 0 and 7 falls from 0.0068 for
distinguishable photons to 0.00084.

**What doesn't agree yet.** Their simulated correlation matrices (Figs. 3B, 3D, 4B and 4D) are
MATLAB 3D bar charts drawn as vector paths, and each bar's height is the length of its side
faces' vertical edges, scaled by the panel's z axis, whose own line reads 0.02001, 0.04005,
0.02002 and 0.04000 for its 0.02 and 0.04 ticks. Their tallest bars and ours, with $Cz = 3.91$:

| Panel | Input, photons | Paper (tallest bar) | photonoxide (largest $\Gamma_{q,r}$) |
|---|---|---|---|
| 3B | (0, 1), distinguishable | 0.0178 | 0.0236 |
| 3D | (0, 1), indistinguishable | 0.0237 | 0.0361 |
| 4B | (−1, 1), distinguishable | 0.0147 | 0.0162 |
| 4D | (−1, 1), indistinguishable | 0.0266 | 0.0264 |

Only Fig. 4D agrees (1%). No single $Cz$ from 2 to 6 matches all four panels, either as
$\Gamma_{q,r}$ or with the $21 \times 21$ matrix normalized to 1, so their simulations' length or
couplings differ from the uniform array the text gives, perhaps by the spreading regions'
coupling, which the paper states only as an effective length (its fig. S1, in the supplement, is
not read here). The gap stays open, and the panels' heights are not a case until it closes.

**Convergence.** The sampler's total variation distance to the exact distribution falls as
$N^{-1/2}$: 3 photons in 5 modes, 1000 to 256 000 samples, eight seeds each, give an order of
0.50 (`quantum/sampler-convergence`). The HOM visibility of the 3D directional coupler (two
500 × 220 nm strips 200 nm apart, a quarter of its cross-over length) from the mode solver's
supermodes on three grids converges at order 0.87, the standard scheme's first order at the
strips' corners (`quantum/coupler-hom-order`, as `device/coupler-3d-order`). The plan's
convergence study names the coupler from FDFD; the mode solver's supermodes stand in until Q3
brings a coupler's S-matrix from FDFD into the statistics.

## Limits

- Permanents up to $n = 40$ (hours), the definition up to 10, partly distinguishable photons up
  to 8 (a sum over $n!$ permutations), the sampler up to 30 photons.
- `Distribution::exact` lists all $\binom{m+n-1}{n}$ patterns: 4 photons in 21 modes is 10 626,
  10 in 30 modes 6.4 × 10⁸, beyond memory. Single probabilities (`probability`) and samples have
  no such limit.
- Gaussian states, squeezing, hafnians and the photon pairs' sources are Q2; frequency-dependent
  statistics beyond two photons, loss budgets, emitters and detectors are Q3.

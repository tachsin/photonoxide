---
title: "Linear-optical statistics"
module: quantum
summary: "Photons on a circuit's ports and what comes out: the S-matrix as the modes' transformation with its loss dilated, permanents by Ryser's and Glynn's formulas, partly distinguishable photons, an exact sampler, Hong–Ou–Mandel dips through S(ω), the Reck and Clements meshes; Gaussian states with hafnians, loop hafnians and torontonians, photon-pair sources from joint spectra and rings, and squeezers from parametric amplifiers."
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
  - cite: "A. Björklund, B. Gupt, N. Quesada, ACM J. Exp. Algorithmics 24, 1.11 (2019)"
    doi: 10.1145/3325111
  - cite: "C. S. Hamilton et al., Phys. Rev. Lett. 119, 170501 (2017)"
    doi: 10.1103/PhysRevLett.119.170501
  - cite: "N. Quesada, J. M. Arrazola, N. Killoran, Phys. Rev. A 98, 062322 (2018)"
    doi: 10.1103/PhysRevA.98.062322
  - cite: "R. Kan, J. Multivariate Anal. 99, 542 (2008)"
    doi: 10.1016/j.jmva.2007.01.013
  - cite: "C. Weedbrook et al., Rev. Mod. Phys. 84, 621 (2012)"
    doi: 10.1103/RevModPhys.84.621
  - cite: "W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997)"
    doi: 10.1103/PhysRevA.56.1627
  - cite: "L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010)"
    doi: 10.1364/OL.35.003006
  - cite: "Z. Vernon et al., Opt. Lett. 42, 3638 (2017)"
    doi: 10.1364/OL.42.003638
  - cite: "S. Paesani et al., Nat. Commun. 11, 2505 (2020)"
    doi: 10.1038/s41467-020-16187-8
  - cite: "V. D. Vaidya et al., Sci. Adv. 6, eaba9186 (2020)"
    doi: 10.1126/sciadv.aba9186
  - cite: "R. Nehra et al., Science 377, 1333 (2022)"
    doi: 10.1126/science.abo6213
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
  - quantum/hafnian-ones
  - quantum/hafnian-definition
  - quantum/hafnian-threads
  - quantum/squeezed-vacuum
  - quantum/two-mode-thermal
  - quantum/squeezed-loss
  - quantum/displaced-states
  - quantum/source-pairs
  - quantum/source-hom-gaussian
  - quantum/heralding-coupler
  - quantum/ring-helt-short
  - quantum/ring-helt-long
  - quantum/ring-vernon-equal
  - quantum/squeezer-degenerate
  - quantum/squeezer-four-wave
  - quantum/squeezer-rk4-order
  - quantum/vaidya-number-difference
  - quantum/vaidya-outcoupling
  - quantum/nehra-inferred
  - quantum/nehra-all-optical
  - quantum/nehra-homodyne-bound
---

A linear circuit does to photons what it does to light: it mixes the ports' modes by its
S-matrix. Everything about photons through a circuit, from two photons on a coupler to a boson
sampler, follows from that matrix, its loss and its change with frequency. The module
`photonoxide::quantum` takes a [circuit](circuits.md)'s S-matrix, or any matrix, and computes
the photons' statistics exactly, up to about 30 single photons. Squeezed light through the same
matrices is a Gaussian state, whose photon counts are hafnians; photon pairs come from 0.6.1's
joint spectra, from rings, or from two pumps in two modes of a guide; squeezing from 0.6.1's
parametric amplifiers. These are the first two parts of the 0.6.3 milestone's plan,
[docs/plans/quantum.md](../plans/quantum.md) (Q1 and Q2).

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

## Gaussian states

Squeezed, displaced and thermal light are Gaussian states: a state of $n$ modes is its
quadratures' means $d$ and covariance $V$, with $\hat x = \hat a + \hat a^\dagger$ and
$\hat p = -i(\hat a - \hat a^\dagger)$, so the vacuum's variances are 1 (Weedbrook et al. 2012,
doi:10.1103/RevModPhys.84.621, with their $\hbar = 2$). `Gaussian` holds them, the $x$'s first.

- **Operations.** A Gaussian unitary $\hat b = U \hat a + W \hat a^\dagger$ acts as the real
  symplectic $S$ with the blocks $\operatorname{Re}(U + W)$ and $-\operatorname{Im}(U - W)$ in its first row of blocks and $\operatorname{Im}(U + W)$ and $\operatorname{Re}(U - W)$ in its second,
  $V \to S V S^T$, $d \to S d$. The squeezer $\exp(\frac{1}{2}(\xi^* \hat a^2 - \xi \hat a^{\dagger 2}))$,
  $\xi = r e^{i\varphi}$, takes $\hat a \to \hat a \cosh r - \hat a^\dagger e^{i\varphi} \sinh r$, and the
  two-mode squeezer $\hat a \to \hat a \cosh r - \hat b^\dagger e^{i\varphi} \sinh r$.
- **Circuits.** A circuit's transfer $T$ with its loss ([the S-matrix as a mode transformation](#the-s-matrix-as-a-mode-transformation))
  gives $V \to R V R^T + I - R R^T$, $R$ the real form of $T$: the vacuum enters through the
  loss. One loss $\eta$ turns a variance $v$ into $\eta v + 1 - \eta$. `Gaussian::through` takes a
  `Transfer`, so any circuit, with the S-matrices its components compute, acts on squeezed light.
- **What a detector sees.** A homodyne detector at phase $\theta$ measures the variance of
  $x\cos\theta + p\sin\theta$; the squeezing in dB is $10 \log_{10}$ of the smallest one. The mean
  photon number is $\frac{1}{4}(V_{xx} + V_{pp} + d_x^2 + d_p^2) - \frac{1}{2}$, and the photon numbers'
  covariances follow from Wick's theorem: $\frac{1}{8}\operatorname{Tr}(V_{ab} V_{ab}^T) + \frac{1}{4} d_a^T V_{ab} d_b$
  between two modes, and $\frac{1}{8}(\operatorname{Tr} V_{aa}^2 - 2) + \frac{1}{4} d_a^T V_{aa} d_a$ for one.
- **Photon counts** (Hamilton et al. 2017, doi:10.1103/PhysRevLett.119.170501). In the basis
  $(\hat a, \hat a^\dagger)$ the covariance is $\sigma = \frac{1}{4} W V W^\dagger$, $W$ the blocks $I$, $iI$ over $I$, $-iI$;
  with $\sigma_Q = \sigma + \frac{1}{2} I$ and $A = X(I - \sigma_Q^{-1})$, $X$ the swap of the two
  halves, a pattern $\bar n$ of photons has
  $P(\bar n) = \operatorname{Haf}(A_{\bar n}) / (\sqrt{\det \sigma_Q} \prod_i n_i!)$, $A_{\bar n}$ the rows and
  columns of mode $i$ and of its conjugate each repeated $n_i$ times. With means
  $\beta = (\alpha, \alpha^*)$, the loop hafnian with the diagonal $\gamma = \beta^\dagger \sigma_Q^{-1}$
  and the factor $\exp(-\frac{1}{2}\beta^\dagger \sigma_Q^{-1} \beta)$ (Björklund et al. 2019,
  doi:10.1145/3325111).
- **Clicks** of threshold detectors (Quesada et al. 2018, doi:10.1103/PhysRevA.98.062322): the
  modes $S$ click and the rest stay dark with probability
  $\operatorname{Tor}(O_S) / \sqrt{\det \sigma_Q}$, $O = I - \sigma_Q^{-1}$, for a state without means.

## Hafnians, loop hafnians and torontonians

The hafnian of a symmetric $2m \times 2m$ matrix is the sum over its perfect matchings of the
products of the matched entries; the loop hafnian lets a vertex match itself, by its diagonal
entry. Björklund et al.'s formula (Eqs. 24–26) sums over the $2^m$ subsets $Z$ of a fixed perfect
matching: each term is the coefficient of $\lambda^m$ in
$\exp\left(\sum_k \left[\frac{\operatorname{tr}(B^k)}{2k} + \frac{1}{2} v^T X B^{k-1} v\right] \lambda^k\right)$,
$B = A_Z X$ the rows and columns of $Z$'s pairs with each pair's columns swapped and $v$ the diagonal
of $A_Z$ (zero for the hafnian). The paper takes the power traces from a Schur decomposition; here
they come from $\det(I - \lambda B)$, the characteristic polynomial of $B$'s Hessenberg form by the
recursion over its leading blocks, then $\log$ and $\exp$ of power series: additions and
multiplications only. The torontonian is
$\sum_{Z} (-1)^{m - \lvert Z \rvert} / \sqrt{\det(I - O_Z)}$ (Quesada et al., Eq. 9). The $2^m$ terms are
added as the permanents' are: in 256 parts fixed by $m$, each Kahan-summed, the parts in order, so
the result is the same bit for bit on any number of threads.

A pattern of many photons in few modes repeats the same rows many times, and the sum over subsets
costs $2^N$ for $N$ photons whatever the repetition. There the probability takes Kan's moments of a
sum instead (R. Kan, J. Multivariate Anal. 99, 542 (2008), doi:10.1016/j.jmva.2007.01.013): with
$r_i$ the repetitions and $h_i = r_i/2 - v_i$,
$\operatorname{lhaf} = \sum_{v} (-1)^{\sum v} \prod_i \binom{r_i}{v_i} \sum_t \frac{(h \cdot \gamma)^{N - 2t} (h^T A h / 2)^t}{(N - 2t)!\thinspace t!}$
over $\prod (r_i + 1)$ terms, the polarization of $\prod x_i^{r_i}$ whose Gaussian moments are the
matchings. `Gaussian::probability` takes whichever costs less. The alternating sum loses digits as
the photons in one mode grow: about $10^{-10}$ relative at 30 in one mode, and patterns are
limited to 30 photons.

## Pair sources

A spontaneous pair source whose pump doesn't deplete leaves a two-mode squeezed vacuum in each
pair of Schmidt modes of its joint spectral amplitude (Grice & Walmsley 1997,
doi:10.1103/PhysRevA.56.1627; 0.6.1's `JointSpectrum::schmidt` gives the weights $\lambda_k$ and the
purity $P = \sum \lambda_k^2$). `Source` gives the pair $k$ the squeezing $r_k = g\sqrt{\lambda_k}$,
the gain $g$ set by the mean number of pairs per pulse, $\sum_k \sinh^2 r_k = \mu$. Then:

- **Pairs.** Mode $k$ holds $n$ pairs with probability $(1 - t_k) t_k^n$, $t_k = \tanh^2 r_k$; the
  number of pairs is their sum, and `Source::multi_pair` the share of pulses with a pair that hold
  more than one.
- **$g^{(2)}$.** Either arm alone is thermal in each mode: $g^{(2)} = 1 + \sum \bar n_k^2 / (\sum \bar n_k)^2$,
  2 for one mode and $1 + P$ at low gain.
- **Two sources.** Heralded photons from sources $A$ and $B$ interfere on a balanced coupler with
  visibility $\operatorname{Tr}(\rho_A \rho_B) = \lVert F_A^\dagger F_B \rVert^2 / (\lVert F_A \rVert^2 \lVert F_B \rVert^2)$,
  $F$ the amplitudes as signal-by-idler matrices on a common signal grid (`hom_visibility`); a
  source with itself gives its purity.
- **Heralding through a circuit.** `heralding_through` takes each photon's path through any
  component (filters, couplers, a whole circuit) from its S-matrix at the grid's frequencies, and
  0.6.1's heralding efficiencies follow: the filters' spectral cut and the paths' loss together.
  `Source::gaussian` gives the source as a `Gaussian` state of its first Schmidt modes, for
  everything above.

The squeezing $g\sqrt{\lambda_k}$ keeps the low-gain Schmidt modes: at high gain the modes
themselves change with the gain, which this doesn't follow.

## Pairs from rings

A ring point-coupled to its bus enhances a wave's field by $F(\nu) = ik/(1 - r a e^{i\phi(\nu)})$,
the ring components' `field_enhancement`. Two pump photons, each enhanced, make a signal and an
idler, each enhanced on the way out, so (Helt et al. 2010, doi:10.1364/OL.35.003006; Vernon et al.
2017, doi:10.1364/OL.42.003638, Eqs. 3–4)

$f(\nu_s, \nu_i) = F_P(\nu_s + \nu_i) F_s(\nu_s) F_i(\nu_i)$, $F_P(\nu) = \int \alpha(\nu_p) \alpha(\nu - \nu_p) F_p(\nu_p) F_p(\nu - \nu_p)\thinspace d\nu_p$,

$\alpha$ the pump pulse's spectral amplitude. `ring_spectrum` samples it, the pump's integral by the
trapezoidal rule, and returns a `JointSpectrum`. The phase mismatch along the ring is left out: on
resonance the azimuthal orders conserve momentum, and across a resonance it changes by far less
than a radian in a ring of a few hundred µm. Vernon et al.'s second channel, coupled to the pump's
resonance alone, is a pump wave whose round trip loses more, which widens its resonance.

## Two pumps in two modes

Paesani et al. 2020 (doi:10.1038/s41467-020-16187-8) pump a multimode silicon guide in its TM₀ and
TM₁ modes at once, the TM₀ pump delayed, and take the signal in TM₁ and the idler in TM₀.
`dual_pump_spectrum` computes

$f(\nu_s, \nu_i) = \int \alpha(\nu_a) e^{i\omega_a\tau} \alpha(\nu_s + \nu_i - \nu_a) \int_0^L e^{i\Delta k z} dz\thinspace d\nu_a$,
$\Delta k = \beta_a(\nu_a) + \beta_b(\nu_s + \nu_i - \nu_a) - \beta_s(\nu_s) - \beta_i(\nu_i)$,

each $\beta$ the propagation constant of its wave's mode (from the mode solver's effective index
over wavelength, `ModeIndex`). The delayed, faster pump catches the slower one inside the guide and
passes it; their overlap rises and falls smoothly, which removes the sinc's side lobes and with them
the spectral correlations. The middle of the overlap is at $z = c\tau/(n_{g,b} - n_{g,a})$.

## Squeezing from parametric amplifiers

An undepleted parametric amplifier is linear in its signal: the classical amplitudes leave as
$A' = \mu A + \nu A^*$ (one mode) or $A'_i = \nu A_s^*$ beside $A'_s = \mu A_s$ (two), and the same
Bogoliubov map on the annihilation operators, in photon units, is the quantum amplifier, a squeezer
of $r = \operatorname{asinh}\lvert\nu\rvert$. `degenerate`, `non_degenerate` and `four_wave` read $\mu$
and $\nu$ off 0.6.1's own coupled equations (second-harmonic generation run with the harmonic as
the pump, difference-frequency generation, and four-wave mixing with the pump's self- and
cross-phase modulation), seeded so weakly that the seed leaves at $10^{-6}$ of the pump's
amplitude: its own depletion of the pump, of order $10^{-12}$ of its power, is below the
integration's error. Without loss only:
a lossy amplifier adds noise a Bogoliubov map can't carry, so the loss is applied to the squeezed
state after it. These are single-frequency (continuous-wave) amplifiers, exact at any gain while the
pump doesn't deplete; the multimode squeezing of a broadband pulse at high gain is left for later.

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

### Gaussian states, sources and squeezers (Q2)

The identities hold to round-off: the all-ones hafnian is $(2n - 1)!!$ up to $2n = 24$ (to
$1.3 \times 10^{-9}$ relative, the inclusion–exclusion's cancellation); hafnians and loop hafnians
agree with the sums over matchings to $1.7 \times 10^{-13}$ relative up to 12 rows; a hafnian, a
loop hafnian and a torontonian of 14 modes are the same bits on 1 and 7 threads. Squeezed vacuum
has its closed-form $P(2k)$ up to 30 photons and no odd ones, and is dark on a threshold detector
with probability $1/\cosh r$; each half of a two-mode squeezed vacuum is thermal; a 2 mm guide's
4.5 dB/cm through the circuit gives $\eta e^{\mp 2r} + 1 - \eta$; a coherent state is Poissonian
by the loop hafnian, and a displaced squeezed state's distribution has the mean and variance of its
covariance. A source of four equal Schmidt modes has negative-binomial pairs and $g^{(2)} = 1.25$;
the HOM visibility of two Gaussian joint spectra matches its closed form to $7 \times 10^{-15}$;
the heralding through a lossy coupler adds up to its transmission. The squeezers read off 0.6.1's
amplifiers give $r = \kappa\sqrt{P}L$ and four-wave mixing's closed-form gain, their RK4 converging
at order 3.92.

**Published.**

| Case | Paper | photonoxide |
|---|---|---|
| Helt et al. 2010, ring of 47.5 µm, 1.48 GHz lines, 5 ps pump | K = 1.09 | 1.0909 |
| Helt et al. 2010, the same, 5 ns pump | K = 10.00 | 9.986 (0.14 %; their linewidth is given to two figures, 0.68 %) |
| Vernon et al. 2017, Fig. 2(a), equal linewidths, broad pump | K = 1.09 | 1.0909 |
| Vaidya et al. 2020, number-difference squeezing on chip, escape efficiency 0.8 | ~7 dB | 6.99 dB |
| Vaidya et al. 2020, the bound through 1.5 and 0.5 dB of coupling loss | about 5, nearly 10 dB | 5.35, 9.64 dB |
| Nehra et al. 2022, 4.2 and 9.7 dB measured, both unknowns of the loss model solved | 10.48 ± 0.87 dB inferred, L ≈ 0.3 | 11.29 dB, L = 0.33 |
| Nehra et al. 2022, 11 dB read through 33 dB of measurement gain (Eq. 1) | ~11 dB | 11.05 dB |
| Nehra et al. 2022, homodyne behind an efficiency of 0.20 | < 1 dB | 0.97 dB |

Vaidya et al.'s Eq. 3, the number-difference variance $(1 - \eta) n_\text{tot}$, holds at every gain
for a lossy two-mode squeezed vacuum (to round-off, from the Gaussian state's photon-number
covariances), so their measured 0.704 is an efficiency of 0.296. Nehra et al. solve for the loss
with their own fit; from the two measured levels alone the squeezing before the loss is 11.29 dB,
0.81 dB above their 10.48, inside their ±0.87.

**What doesn't agree yet.**

- **Vernon et al.'s dual-coupled ring.** For a pump resonance 6.6 times wider than the signal's and
  idler's, their Fig. 2(b) gives $K = 1.0009$ (purity 0.999); their own point-coupling model, Eqs. 3
  and 4, with the pump far broader than its resonance, gives $K = 1.0033$ here (purity 0.9967), and
  a narrower pump only raises it. Their text's lower bound for equal linewidths, $K = 1.08$, is below
  the model's broad-pump 1.0909, which pumps of finite duration only raise (their Fig. 2(a) prints 1.09), and
  their Fig. 3's line reads about 0.84 at $Q_{SI}/Q_P = 0.5$ and 0.93 at 1 against our 0.800 and
  0.917. Their purities sit above ours by about the same margin at every ratio; the full device
  simulation behind their markers isn't given in the letter. Not a case until it closes.
- **Paesani et al.'s inter-modal source.** Their guide (2 µm × 220 nm silicon in silica, 11 mm,
  TM₀ and TM₁, a 4.5 nm pump at 1550 nm, the TM₀ pump delayed by 1.46 ps) from the mode solver on
  20 and 10 nm cells puts the pairs at 1584.2 and 1517.3 nm (theirs about 1588 and 1516 nm) and
  gives a purity of 0.9980 with the delay and 0.716 without (their simulation: 0.994 and 0.840;
  their measurement: 0.9904 ± 0.0006 and 0.931 ± 0.002). Our TM₁ − TM₀ group-index difference at
  1550 nm is 0.109, where their delay, chosen to centre the pumps' overlap in the guide, implies
  0.080 ($2c\tau/L$); with our dispersion the best delay is near 1.96 ps (purity 0.9990). The pump's
  shape (theirs filtered by a 5 nm square filter) and the dispersion behind their simulation are in
  their Supplementary Note 1, not read here. The purity converges in both grids (81 against 161
  frequencies: $10^{-5}$; 20 against 10 nm cells: $10^{-5}$ with the delay, 0.001 without), so the
  gap is the model's, not the numerics'. Not a case until it closes.
- **Vaidya et al.'s quadrature squeezing.** Their 1.0 dB measured and about 4 dB inferred on chip
  need their collection efficiency and gain parameter, which are in their Supplementary Materials.
  With the escape efficiency of 0.75 alone, the most a ring can give on chip is 6.0 dB, above their
  4 dB; the inference itself waits for those numbers.

## Limits

- Permanents up to $n = 40$ (hours), the definition up to 10, partly distinguishable photons up
  to 8 (a sum over $n!$ permutations), the sampler up to 30 photons.
- `Distribution::exact` lists all $\binom{m+n-1}{n}$ patterns: 4 photons in 21 modes is 10 626,
  10 in 30 modes 6.4 × 10⁸, beyond memory. Single probabilities (`probability`) and samples have
  no such limit.
- Hafnians and torontonians up to 60 rows (30 photons, or 30 clicking modes); patterns up to 30
  photons, the moments-of-a-sum route losing digits as they crowd into one mode (about
  $10^{-10}$ relative at 30). Clicks of states with means (a loop torontonian) aren't computed.
- A source's squeezing per Schmidt mode, $g\sqrt{\lambda_k}$, is the low-gain picture; the
  squeezers are continuous-wave and lossless (the loss goes after them). Broadband squeezing at
  high gain, whose temporal modes change with the gain, is left for later; so are the Schmidt
  modes' shapes, which the studio's source designer will draw.
- Frequency-dependent statistics beyond two photons, loss budgets, emitters and detectors are
  Q3.

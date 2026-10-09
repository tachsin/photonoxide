# Quantum photonics: a survey and a plan

*A survey and a plan, 2026-10-08. Nothing here is on the roadmap yet: the owner reviews it first.*

The roadmap has photon pairs in 0.6.1 (SPDC and SFWM, the joint spectral amplitude, purity,
brightness and heralding) and one line in 0.16, "Quantum photonics: linear optical circuits and
their statistics". This page asks what photonic quantum computing and quantum photonics need from
a validated photonics library, and proposes how photonoxide, its studio and its Academy could
serve them. It has five parts:

1. [The landscape in 2026](#1-the-landscape-in-2026): the approaches, their status and their
   papers.
2. [Open-source software](#2-open-source-software-in-the-field): what each tool does, its licence
   and its focus.
3. [Where photonoxide fits](#3-where-photonoxide-fits): what a from-scratch stack of mode
   solvers, FDFD, FDTD, circuits with adjoints and nonlinear optics can add, and what it should
   leave to dedicated quantum simulators.
4. [A proposed plan](#4-a-proposed-plan): milestones with their validation, studio pages and
   Academy lessons.
5. [Decisions for the owner](#5-decisions-for-the-owner).

**Rules this page follows.**

- Every DOI was checked on Crossref on 2026-10-08. The papers are listed in the papers folder's
  README under "Quantum photonics (survey)": 43 new entries, three papers already in the folder
  and one in its open-access survey list (see [References](#references)).
- Other tools are described by what they do. GPL tools may be named; none is read.
- Numbers from papers are quoted as the papers state them, with what they include (a fidelity
  "not accounting for loss" is quoted with that caveat). *(estimate)* marks an estimate of ours,
  with its reasoning.
- The plan proposes software and its validation only. It promises no hardware, measurements or
  collaborations.

## Summary

1. **Loss decides feasibility.** Every route to a photonic quantum computer turns on how many
   photons survive from source to detector. Thresholds are stated per photon or per operation:
   source times detector efficiency above 2/3 for one cluster-state scheme (Varnava et al. 2008),
   about 10% loss per fusion for a fusion-based one (Bartolucci et al. 2023), squeezing near 10 dB
   for GKP qubits in a surface code (Fukui et al. 2018). A chip's loss budget is a product of its
   components' transmissions, which is what photonoxide's solvers compute.
2. **The quantum side of a linear circuit is its S-matrix.** photonoxide's circuits already use
   power-normalized S-matrices in the e^(−iωt) convention, which is the transformation of the
   modes' annihilation operators. Multi-photon statistics (permanents for single photons,
   hafnians for squeezed light) need nothing from a circuit but that matrix, its frequency
   dependence and its loss.
3. **What photonoxide can add that a quantum simulator can't:** the matrix from the device's
   geometry, at every wavelength, with its loss and its error against the solver's grid; photon
   pair sources from the mode solver's dispersion (0.6.1); emitter and detector coupling from FDTD
   and the mode solver; gradients of quantum figures of merit through the circuit adjoint.
4. **What it should leave to others:** fault-tolerance thresholds and decoders, large Fock-space
   simulations, tensor-network samplers, compilers for quantum processors, emitter dynamics.
   photonoxide exports transfer and covariance matrices for those tools.
5. **The proposal:** a milestone right after 0.6.1, "0.6.2: Quantum light and linear-optical
   statistics", in three parts (statistics, sources and Gaussian states, real components in
   quantum circuits); quantum figures of merit as objectives in 0.7; seven Academy lessons, the
   first two of which (Hong–Ou–Mandel interference, programmable interferometers) need only
   today's circuits plus the statistics.

## 1. The landscape in 2026

### Discrete variables: from KLM to fusion

- **Linear optics with measurement is universal.** Knill, Laflamme and Milburn (2001) showed that
  single photons, linear optics, photon detection and feed-forward suffice for scalable quantum
  computing. The gates are probabilistic: the basic nonlinear sign gate succeeds with probability
  1/4, and teleportation makes them near-deterministic at a large cost in photons.
- **Measurement-based computing moved the entangling work into a resource state.** The one-way
  computer of Raussendorf and Briegel (2001) prepares a cluster state and computes by measuring
  single qubits. Browne and Rudolph (2005) built cluster states from small pieces by "fusion",
  a partial Bell measurement that succeeds with probability 1/2 with linear optics alone; Ewert
  and van Loock (2014) reached 3/4 with unentangled single-photon ancillae.
- **Loss became the number to beat.** Varnava, Browne and Rudolph (2008) showed a cluster-state
  scheme works when the product of source and detector efficiencies exceeds 2/3. Fusion-based
  quantum computation (Bartolucci et al. 2023) builds a fault-tolerant computer from many copies
  of a small resource state and fusions between them, and its ballistic scheme tolerates a 10.4%
  probability of losing a photon in each fusion.
- **Where hardware stands.** PsiQuantum's platform paper (Alexander et al. 2025) reports
  dual-rail qubits with 99.98% state preparation and measurement fidelity, 99.50%
  Hong–Ou–Mandel visibility between independent sources, 99.22% two-qubit fusion fidelity and a
  99.72% chip-to-chip interconnect, all "not accounting for loss", from monolithic silicon
  photonics, with silicon nitride waveguides, photon-number-resolving detectors and barium
  titanate phase shifters previewed. Quandela's platform (Maring et al. 2024) feeds a
  reconfigurable chip from a quantum-dot source and reports six-photon boson sampling and heralded
  three-photon entanglement.

### Continuous variables: squeezing, cluster states and GKP qubits

- **Gaussian states are the cheap part.** Squeezed light, beam splitters and homodyne detection
  form a well-understood toolbox (Weedbrook et al. 2012). Menicucci et al. (2006) showed that
  continuous-variable cluster states with measurement are universal; time-domain multiplexing
  built large two-dimensional ones from a few squeezers and delay lines (Asavanant et al. 2019).
- **Fault tolerance needs a non-Gaussian code.** Gottesman, Kitaev and Preskill (2001) encoded a
  qubit in an oscillator, the GKP qubit. With analog error correction in a surface code, the
  squeezing it needs drops below 10 dB (Fukui et al. 2018).
- **GKP states of light now exist.** Konno et al. (2024) made them in propagating light at a
  telecom wavelength. Larsen et al. (2025) made them on a multilayer silicon nitride chip with
  photon-number-resolving detectors, heralded from Gaussian boson sampling: at least four
  resolvable peaks in both quadratures and a 3 × 3 grid of negative Wigner regions, a proof of
  concept below fault-tolerant quality. Aghaee Rad et al. (2025) networked photonic chips in
  separate racks into one machine that synthesizes a cluster state across them, at room
  temperature.

### Boson sampling and Gaussian boson sampling

- **The idea.** Aaronson and Arkhipov (2013) showed that sampling the output of single photons in
  a random interferometer is hard classically under plausible conjectures: each outcome's
  probability is the squared permanent of a submatrix of the unitary. Gaussian boson sampling
  replaces single photons with squeezed states, and probabilities become hafnians (Hamilton et al.
  2017), or torontonians for threshold detectors (Quesada et al. 2018).
- **The experiments.** Zhong et al. (2020) sent 50 squeezed states into a 100-mode interferometer
  and observed up to 76 photon clicks. Madsen et al. (2022) ran 216 squeezed modes in a
  time-multiplexed, programmable machine with photon-number-resolving detectors, up to 219
  photons and a mean of 125. Jiuzhang 4.0 injected 1,024 squeezed states into 8,176 modes (Liu et
  al. 2026), with up to 3,050 photon detection events in its preprint.
- **The debate.** Loss and noise help classical algorithms. Exact boson sampling costs
  O(n 2ⁿ + poly(m, n)) per sample (Clifford & Clifford 2018); a hafnian of an n × n matrix costs
  O(n³ 2^(n/2)) (Björklund et al. 2019). Qi et al. (2020) gave an inequality between squeezing,
  transmission and detector quality beyond which noisy GBS is efficiently simulable, and showed
  that loss growing exponentially with depth removes the advantage asymptotically. Oh et al.
  (2024) simulated the largest experiments of the time with a tensor-network algorithm that
  exploits loss, and argued it matches the ideal distribution better than they do; the
  Jiuzhang 4.0 authors report outperforming that method. The question is open, and it is a
  question about loss.

### The hardware

| Part | What matters | State, with sources |
|---|---|---|
| Platforms | loss per component, phase stability, fabrication spread | Silicon (Silverstone et al. 2014; Paesani et al. 2020; Alexander et al. 2025), silicon nitride (Larsen et al. 2025; Vaidya et al. 2020), thin-film lithium niobate (Nehra et al. 2022; Zhao et al. 2020, folder), glass and InP among others (Wang et al. 2019's review) |
| Pair sources (SPDC, SFWM) | purity, indistinguishability, heralding efficiency, multi-pair noise | Silicon intermodal SFWM: purity 0.9904 ± 0.0006, mutual indistinguishability 0.987 ± 0.002, > 90% intrinsic heralding efficiency (Paesani et al. 2020); rings whose pump and pair resonances are coupled separately can reach a Schmidt number of 1 (Vernon et al. 2017) |
| Emitters (quantum dots) | efficiency into one mode, indistinguishability | 57% of photons delivered at the end of a fibre, at GHz rates (Tomm et al. 2021); 98.43 ± 0.04% of emission coupled into a photonic-crystal waveguide (Arcari et al. 2014) |
| Squeezers | dB on chip, bandwidth, single temporal mode | 1.0 dB measured, about 4 dB inferred on chip, from silicon nitride rings (Vaidya et al. 2020); 4.9 dB measured, about 11 dB inferred, over more than 25 THz, in thin-film lithium niobate (Nehra et al. 2022) |
| Interferometers | universality, depth, loss balance | Reck et al. (1994) and Clements et al. (2016, folder): N(N−1)/2 beam splitters, depth 2N−3 against N; Carolan et al. (2015): six modes, 15 MZIs and 30 thermo-optic phase shifters |
| Detectors (SNSPD, TES) | efficiency, photon-number resolution, jitter | 98% system detection efficiency at 1550 nm (Reddy et al. 2020); 91% on-chip efficiency for a nanowire on a waveguide (Pernice et al. 2012) |
| Feed-forward and switching | speed, loss per switch | fast low-loss phase shifters (electro-optic: lithium niobate, barium titanate) for multiplexing and adaptive measurement (Alexander et al. 2025; Wang et al. 2019) |

**The loss budget.** A photon's chance of arriving is the product of every transmission on its
path: source extraction, couplers, waveguide length, crossings, switches, fibre interfaces and the
detector. n photons arrive together with that product to the n-th power, and in a mesh of depth d
each path crosses d cells. A budget is therefore a product of component S-matrix elements, which
is why the field's thresholds and photonoxide's solvers meet.

### Communication and sensing on chip, briefly

Quantum key distribution and quantum sensing on chip use the same components: sources, phase
shifters, interferometers, squeezers and detectors (Wang et al. 2019). Nothing in this plan is
specific to computing; a source's purity, a squeezer's dB and a detector's efficiency serve all
three.

## 2. Open-source software in the field

Checked on GitHub and PyPI on 2026-10-08. Each tool is described by what it does.

| Tool | What it does | Licence | Language |
|---|---|---|---|
| Perceval (Quandela; Heurtel et al. 2023) | discrete-variable linear-optical circuits with single photons: simulation back ends, noise models, access to Quandela's processors | MIT | Python |
| Strawberry Fields (Xanadu; Killoran et al. 2019) | continuous-variable circuits with Gaussian, Fock and bosonic back ends; its repository is archived | Apache-2.0 | Python |
| The Walrus (Xanadu; Gupt et al. 2019) | hafnians, loop hafnians, torontonians and permanents, Gaussian boson sampling; archived | Apache-2.0 | Python |
| MrMustard (Xanadu) | differentiable Gaussian and Fock-space simulation; archived | Apache-2.0 | Python |
| FlamingPy (Xanadu) | error-correction simulations for fault-tolerant photonic architectures; archived | Apache-2.0 | Python |
| Piquasso | Gaussian, Fock and boson-sampling simulators | Apache-2.0 | Python and C++ |
| Lightworks (Aegiq) | a software kit for photonic quantum computing: circuits and their emulation | Apache-2.0 | Python |
| Graphix | measurement-based computing: patterns, compilation and simulation | Apache-2.0 | Python |
| BosonSampling.jl | boson-sampling simulation and analysis | MIT | Julia |
| QuTiP; QuantumOptics.jl | open quantum systems, used for emitters and cavities | BSD-3-Clause; MIT | Python; Julia |
| bosonic-qiskit | hybrid qubit and boson circuits on Qiskit | BSD-2-Clause | Python |
| ZPGenerator (Quandela) | pulsed sources from time-dependent emitters, and the photons they emit | MIT | Python |
| SPDCalc | SPDC in nonlinear crystals: phase matching, joint spectra, Schmidt number, HOM | MIT | Rust, with Python bindings |
| pyJSA | the joint spectral amplitude of SPDC in PPLN ridge waveguides, purity and heralding | GPL-3.0 | Python |
| gdsfactory; SAX; Simphony | layout; S-parameter circuit simulation (classical), used for quantum chips too | MIT; Apache-2.0; MIT | Python |

The Xanadu repositories above were archived on GitHub by 2026-10-08; their code remains
available. The table's papers are listed in [References](#references) only where a tool has one
the plan uses; the tools' own papers are not added to the papers folder.

What the table shows: the quantum simulators start from a unitary or a covariance matrix that the
user supplies. The photonic design tools stop at classical S-parameters. Pair-source tools model
bulk crystals or one waveguide type. The link between a device's geometry and its quantum figures
of merit is left to each user.

## 3. Where photonoxide fits

### What photonoxide has now

- Mode solvers with dispersion and group index (0.2), 2D and 3D FDFD with ports and S-matrices
  (0.3, 0.4), FDTD with dipoles, mode sources, flux and mode monitors (0.5, in progress).
- Circuits of components with power-normalized S-matrices, reciprocity, passivity and unitarity
  checks, the circuit adjoint, compact models and Touchstone files (0.4).
  `examples/circuit_splitter.rs` already reproduces Clements et al.'s variable beam splitter, and
  the adjoint is checked on a 4 × 4 Clements mesh of 24 parameters.
- Planned: phase matching, coupled-mode equations, JSA and purity (0.6.1); inverse design (0.7);
  Reck and Clements meshes, self-configuration and error correction (0.8); process variation
  (0.11).

### The bridge: a circuit's S-matrix is its mode transformation

photonoxide's S-matrices use the e^(−iωt) convention and power normalization, so the outgoing
annihilation operators are $\hat b = S \hat a$ for a lossless circuit. A lossy circuit needs noise
modes to keep the commutators: $\hat b = S \hat a + L \hat c$ with $L L^\dagger = I - S S^\dagger$,
the vacuum entering through the loss. Every quantity below follows from $S(\omega)$:

- **Single photons:** n photons in input modes $T$ leave in modes $R$ with probability
  $\lvert \operatorname{Per}(S_{R,T}) \rvert^2 / \prod_r m_r!$, for occupation numbers $m_r$
  (Aaronson & Arkhipov 2013). Partly distinguishable photons and frequency-dependent $S$ enter
  through the photons' spectra.
- **Gaussian states:** a covariance matrix $\sigma$ goes to $S \sigma S^\dagger$ plus the
  vacuum's share through $L$; photon-number patterns follow from hafnians (Hamilton et al. 2017)
  and threshold clicks from torontonians (Quesada et al. 2018).
- **Two-photon interference:** for a beam splitter of reflectance $R$ and transmittance $T$, the
  coincidence probability is $(R - T)^2$ for identical photons and $R^2 + T^2$ for
  distinguishable ones, a visibility $V = 2RT/(R^2 + T^2)$ (derived from the permanent). Two
  heralded photons from identical sources of purity $P$ interfere with visibility $P$.

### What photonoxide can provide that the field lacks

1. **Pair sources from geometry.** 0.6.1 gives the JSA from the mode solver's dispersion and the
   pump. Extended to heralding efficiency (with the filters' and couplers' S-matrices),
   multi-pair noise, rings (Helt et al. 2010; Vernon et al. 2017) and intermodal SFWM (Paesani
   et al. 2020), purity and brightness become functions of width, thickness, length and poling,
   with their convergence on the solver's grid.
2. **Squeezers as Gaussian devices.** The parametric amplifiers of 0.6.1 give squeezing; with the
   device's loss they give the output covariance, the dB that leaves the chip, and the
   multimode structure of a broadband squeezer.
3. **Quantum figures of merit of real components.** A directional coupler's $R(\lambda)$ from the
   mode solver, an MMI's imbalance from FDFD, a mesh's loss and crosstalk from its components:
   each becomes a HOM visibility, a unitary's fidelity or a sampling distribution, with the
   solver's error carried through. Over the 0.11 process variation, each becomes a distribution.
4. **Loss budgets with their sources.** Every transmission in the product is a solver's or a
   measured Touchstone file's, with its grid; the budget is checked against thresholds like
   Varnava et al.'s or Bartolucci et al.'s, quoted with the assumptions they carry.
5. **Emitters and detectors as electromagnetic problems.** FDTD with a dipole and a mode monitor
   gives an emitter's β factor and Purcell factor in a waveguide or cavity (Arcari et al. 2014's
   photonic-crystal waveguide is a 3D case). The mode solver with a lossy film gives a
   waveguide-integrated nanowire's absorption length (Pernice et al. 2012); the transfer-matrix
   method gives a normal-incidence detector stack's absorption (Reddy et al. 2020).
6. **Gradients.** The circuit adjoint gives $\partial S/\partial\theta$; a permanent's derivative
   with respect to an entry is the permanent of its minor; a JSA's purity depends on the
   dispersion the mode solver differentiates. Quantum figures of merit become objectives for
   genoxide with gradients, checked against finite differences like every other.
7. **One place to see it.** The studio shows the device, its S-matrix, its photons' statistics
   and its budget together, live, with the record to replay.

### What to leave to dedicated tools

- **Fault tolerance:** thresholds, decoders and architecture simulations (FlamingPy's or
  Bartolucci et al.'s kind of work). photonoxide quotes published thresholds and doesn't compute
  new ones.
- **Large Fock spaces and non-Gaussian dynamics:** GKP state preparation by heralding, cat
  states, many-mode Fock simulations. photonoxide keeps to Gaussian states and exact multi-photon
  probabilities of moderate size.
- **Classical spoofing at scale:** tensor-network GBS samplers (Oh et al. 2024) and large
  distributed hafnian runs.
- **Processor software:** compilers, transpilers, cloud access, noise models of specific
  machines (Perceval, Lightworks, Graphix).
- **Emitter dynamics:** time-dependent quantum-dot physics (ZPGenerator, QuTiP). photonoxide
  supplies the photonic environment, the β and Purcell factors.

For these, photonoxide exports what it computes: transfer matrices with their loss (Touchstone
already exists), covariance matrices and JSAs, in plain documented formats.

### Size and cost

*(estimates)* A permanent by Ryser's formula (Ryser 1963) or Glynn's (Glynn 2010) in Gray-code
order costs about n 2ⁿ operations: 2 × 10⁷ for n = 20 (milliseconds), 3 × 10¹⁰ for n = 30
(seconds to tens of seconds on one core), 4 × 10¹³ for n = 40 (hours on 20 cores). The hafnian
behind an N-photon GBS pattern costs O(N³ 2^(N/2)) (Björklund et al. 2019), about 4 × 10¹² for
N = 50. So exact statistics of up to about 30 single photons, or GBS patterns of up to about 50
photons, are within a workstation; beyond that, sampling and dedicated codes take over.
Determinism (principle 9) holds if the Gray-code sum is split into fixed chunks by the problem,
not the thread count, and the chunks are added in order; samplers take explicit seeds.

## 4. A proposed plan

### Where it fits

The roadmap's 0.6.1 already computes JSAs and purity. The multi-photon statistics need only the
0.4 circuits. Three placements:

- **(a) A new 0.6.2 right after 0.6.1:** statistics, sources and Gaussian states, real
  components in quantum circuits (Q1 to Q3 below), with quantum objectives in 0.7 (Q4).
- **(b) Q1 earlier,** as a small item beside 0.5 or 0.6, since it depends only on 0.4; Q2 and Q3
  after 0.6.1.
- **(c) As now, at 0.16.**

*Recommended: (a).* The sources are where photonoxide's solvers add most, and they arrive with
0.6.1; Q1 is small enough to land in the same milestone without delaying anything. 0.16's line
then becomes Q5 or goes.

Each item below is validated as the roadmap requires: an analytic test, a published result
reproduced, a convergence test. Numbers marked "to read" come from papers whose tables or figures
must be read before a tolerance is set.

### Q1: Linear-optical statistics

**Work.**

- A `quantum` module: Fock inputs on a circuit's ports; the transfer matrix of any `Circuit` at a
  wavelength, with its loss dilated to noise modes; permanents by Ryser's and Glynn's formulas,
  threaded and deterministic; output probabilities, marginals and bunching; partly
  distinguishable photons through their overlaps; an exact sampler (Clifford & Clifford 2018)
  with an explicit seed.
- Two-photon interference with spectra: the HOM dip against delay for given photon spectra and a
  frequency-dependent $S(\omega)$, from the circuit's own sweep.
- Haar-random unitaries and the Reck and Clements decompositions (Reck et al. 1994; Clements et
  al. 2016), moved forward from 0.8 if the owner agrees (decision 5).

**Validation.**

- *Analytic:* $V = 2RT/(R^2+T^2)$, and no coincidences at $R = 1/2$; $\operatorname{Per}(J_n) =
  n!$ for the all-ones matrix and $\operatorname{Per}(J_n - I_n)$ the number of derangements;
  Ryser, Glynn and the defining sum agree to round-off for n ≤ 8; the probabilities of all
  $\binom{m+n-1}{n}$ patterns sum to 1 for a unitary; distinguishable photons give the classical
  probabilities, permanents of $\lvert S \rvert^2$; a Gaussian-spectrum HOM dip against its closed
  form.
- *Published:* Peruzzo et al. (2010)'s two-photon correlations in an array of coupled waveguides,
  predicted from coupled-mode theory with the paper's array parameters, and with couplings from
  the mode solver (to read); Clements et al. (2016)'s comparison of loss robustness against Reck's
  design (the paper is in the folder; to read).
- *Convergence:* the sampler's distance to the exact distribution falls as N^(−1/2) in the number
  of samples; a coupler's HOM visibility from FDFD converges at the solver's order with the grid.

**Studio:** the quantum view of the Chip page (below).

### Q2: Sources and Gaussian states

**Work.**

- From 0.6.1's JSA: the Schmidt decomposition and purity (Grice & Walmsley 1997, folder),
  heralding efficiency through the filters' and couplers' S-matrices, multi-pair probability,
  HOM visibility between two sources.
- Rings: pair generation in rings (Helt et al. 2010) from the 0.4 ring components; Vernon et
  al.'s dual-coupled ring.
- A `Gaussian` state: covariance and means, squeezers, any circuit's $S$ with its loss, homodyne
  variances, photon-number probabilities by hafnians and loop hafnians (Björklund et al. 2019),
  threshold clicks by torontonians (Quesada et al. 2018).
- Squeezing from 0.6.1's parametric amplifiers, low gain first; high gain only when its
  validation is in place.

**Validation.**

- *Analytic:* single-mode squeezed vacuum has only even photon numbers, with
  $P(2k) = \frac{(2k)!}{4^k (k!)^2} \frac{\tanh^{2k} r}{\cosh r}$ and mean $\sinh^2 r$; each half of
  a two-mode squeezed vacuum is thermal; $\operatorname{Haf}(J_{2n}) = (2n-1)!!$; loss $\eta$
  turns a quadrature variance $e^{-2r}$ into $\eta e^{-2r} + 1 - \eta$; the purity of a Gaussian
  JSA against its closed form.
- *Published:* Paesani et al. (2020)'s source: the purity predicted from the multimode
  waveguide's TE0 and TE1 dispersion, against their 0.9904 ± 0.0006 measured (their geometry and
  simulated purity to read); Vernon et al. (2017)'s Schmidt number against the ratio of the
  rings' couplings; Zhao et al. (2020, folder)'s thin-film lithium niobate pairs, shared with
  0.6.1; the on-chip squeezing of Vaidya et al. (2020) (about 4 dB inferred) and Nehra et al.
  (2022) (about 11 dB inferred) from their devices' parameters and stated losses (to read).
- *Convergence:* purity with the JSA's frequency grid and with the mode solver's grid (through
  Δk); squeezing with the coupled-mode equations' step.

**Studio:** the source designer and the Gaussian view (below).

### Q3: Real components in quantum circuits

**Work.**

- Multi-photon statistics with frequency-dependent S-matrices from any fidelity: closed forms,
  compact models, 2D and 3D FDFD, FDTD, Touchstone files.
- Loss budgets: per path, per photon and per n-photon event, against thresholds quoted with
  their assumptions; Qi et al. (2020)'s simulability inequality evaluated for a designed GBS
  chip.
- With 0.11: HOM visibility, a unitary's fidelity and the budget as distributions over process
  variation.
- Emitters and detectors: β and Purcell factors from FDTD with a dipole and mode monitors; a
  waveguide nanowire's absorption from the mode solver; a detector stack's absorption from
  `Multilayer`. These need the films' optical constants (NbN, WSi or MoSi, GaAs at low
  temperature) with provenance, which the catalogue doesn't have.

**Validation.**

- *Analytic:* frequency-independent S gives Q1's results exactly; the dilation satisfies
  $S S^\dagger + L L^\dagger = I$ to round-off; a budget equals the product of the S-matrix
  elements on its path; a dipole in a homogeneous medium has a Purcell factor of 1.
- *Published:* Silverstone et al. (2014)'s two silicon SFWM sources in an interferometer, the
  on-chip fringe against their 100.0 ± 0.4% visibility; Arcari et al. (2014)'s β = 98.43% in a
  photonic-crystal waveguide (3D FDTD; geometry to read); Pernice et al. (2012)'s nanowire on a
  waveguide, the absorption against their 91% on-chip efficiency and their own simulated
  absorption (to read).
- *Convergence:* every quantum figure of merit inherits its solver's grid study and is reported
  with it.

**Studio:** the loss budget (below).

### Q4: Quantum objectives in inverse design (with 0.7)

- Objectives with gradients: a source's purity and heralding efficiency, HOM visibility, a
  unitary's fidelity, an output pattern's probability.
- Gradients: the circuit adjoint for $\partial S/\partial\theta$, the minors' permanents for the
  statistics, the mode solver's derivatives for dispersion; genoxide supplies the optimizers.
- *Validation:* every gradient against finite differences to 1%; optimizers recover closed-form
  optima (Vernon et al.'s dual-coupling condition, the 50:50 coupler of a HOM experiment, a
  Clements mesh programmed to a Haar-random unitary).

### Q5: Resource states (0.16, or Academy only)

Small linear-optical circuits with heralding: the nonlinear sign gate (success 1/4, Knill et al.
2001), type-II fusion (1/2, Browne & Rudolph 2005), boosted fusion (3/4, Ewert & van Loock 2014).
Q1's permanents compute these success probabilities exactly; their closed forms are the tests.
Beyond that, thresholds and architectures stay with dedicated tools. *Recommended:* as Academy
material, not a milestone.

### Studio pages

| Page | What it shows | Computed by |
|---|---|---|
| **Quantum view of a chip** (on the Chip page) | photons or squeezed states on chosen input ports; the transfer matrix as magnitude and phase; output statistics against distinguishable photons; HOM dips; the loss budget per path; sampling live against the exact distribution, with a hard timeout | `quantum` with the circuit's `s_matrix` sweep |
| **Source designer** (with the nonlinear jobs of 0.6.1) | the JSA and JSI as heatmaps, Schmidt coefficients, purity, brightness, heralding efficiency, against pump bandwidth, length, width and poling | 0.6.1's JSA, Q2's Schmidt decomposition |
| **Gaussian view** | covariance matrix, squeezing in dB after loss, photon-number distributions, a mode's Wigner function | Q2's `Gaussian` |
| **Loss budget** | a waterfall of every component's transmission along a path, the n-photon success probability, and published thresholds marked with their assumptions | Q3 |

All four follow principle 5: a job runs the same from the CLI and the window, and its record
replays.

### Academy lessons

Each lesson has the three depths and the timeline of the Academy's format. The charts are Rust
functions in `charts.rs` calling the library, as now.

| Lesson | Level | Charts | Computed by | Needs |
|---|---|---|---|---|
| **Hong–Ou–Mandel interference** | introductory | `hom-dip`: delay, bandwidth, reflectance, photons' overlap; `hom-coupler`: a directional coupler's gap and length, its $R(\lambda)$ and the visibility over wavelength | Q1's two-photon interference; the existing `Coupler` component | Q1 |
| **Programmable interferometers** (Reck and Clements) | intermediate | `mesh-fidelity`: modes, phase error, loss per cell, Reck against Clements | Q1's decompositions, `Circuit`, `SMatrix::unitarity_error` | Q1 (or 0.8) |
| **Photon pairs and purity** | intermediate | `jsa`: pump bandwidth, length, width, poling; `schmidt`: the Schmidt coefficients and purity; `ring-pairs`: a dual-coupled ring's pump and pair couplings | 0.6.1's JSA; Q2 | 0.6.1, Q2 |
| **Boson sampling and permanents** | intermediate | `boson-distribution`: photons, modes, seed, distinguishability, the pattern probabilities; `permanent-cost`: measured time of Ryser and Glynn against n, live | Q1 | Q1 |
| **Squeezed light and Gaussian boson sampling** | advanced | `squeezing`: r or dB and loss, photon numbers and quadrature variance; `gbs-simulability`: squeezing, transmission and detector efficiency against Qi et al.'s inequality | Q2; Q3 | Q2, Q3 |
| **From KLM to fusion** | advanced | `fusion-success`: the success probability of the sign gate, fusion and boosted fusion, from their circuits; `loss-threshold`: a budget against published thresholds | Q1 (Q5's circuits); Q3 | Q1, Q3 |
| **GKP qubits and continuous-variable cluster states** | advanced, theory | `gkp-wigner`: a finite-energy GKP state's Wigner function, in closed form (Gottesman et al. 2001); squeezing against Fukui et al.'s threshold | closed forms only | none beyond the lesson |

The first two can be written as soon as Q1 exists; the HOM lesson's `hom-coupler` chart uses only
today's components plus the visibility formula.

## 5. Decisions for the owner

*Decided by the owner on 2026-10-09: the recommendations, for all of them. The milestone is 0.6.2 in ROADMAP.md, the Reck and Clements decompositions move there from 0.8, and 0.16's quantum line is gone.*

1. **Where the work goes.** (a) A new 0.6.2 right after 0.6.1; (b) Q1 earlier, beside 0.5 or 0.6,
   and Q2 and Q3 after 0.6.1; (c) at 0.16 as now. *Recommended: (a).*
2. **The scope boundary.** photonoxide computes exact statistics of moderate size (about 30
   single photons, about 50 in Gaussian patterns, *estimate*), Gaussian states, and the
   electromagnetics of sources, emitters and detectors; it exports matrices for everything
   beyond. No decoders, thresholds or Fock-space architectures. *Recommended: yes.*
3. **Cross-checks against Python tools.** AGENTS.md rules out Python anywhere. (a) No: check only
   against closed forms and published numbers. (b) The owner may run The Walrus, Perceval or
   Piquasso outside the repository and record the comparison, as Meep is run, with nothing Python
   in the repository. *Recommended: (a), with (b) allowed for one-off comparisons.* SPDCalc is
   Rust and MIT-licensed: whether it may be run as an external program for comparison on bulk
   SPDC is a separate yes or no.
4. **Published experimental data.** The GBS experiments published their samples and settings.
   Use them as reproduction targets only where their licence is clear, fetched by the test
   rather than stored in the repository? *Recommended: yes, with the licence recorded.*
5. **Meshes earlier.** Move the Reck and Clements decompositions (not self-configuration or error
   correction) from 0.8 into Q1. *Recommended: yes;* they are small and Q1's tests need them.
6. **Material data for detectors and emitters.** NbN, WSi or MoSi films and low-temperature GaAs
   enter the catalogue with provenance when Q3's reproductions need them. *Recommended: yes, only
   then.*
7. **The Academy's order.** Hong–Ou–Mandel and boson sampling with Q1; photon pairs with Q2;
   squeezing with Q3; KLM and fusion, and GKP, as theory lessons whenever written.
   *Recommended: as listed.*
8. **0.16's line.** Replace "Quantum photonics: linear optical circuits and their statistics" by
   Q5 as Academy material, or remove it. *Recommended: remove it once 0.6.2 is on the roadmap.*

## References

Every DOI below was checked on Crossref on 2026-10-08. "Folder" marks a copy in the papers
folder; "survey" marks an open-access paper listed in the folder's survey-2026-10.md. The other
43 are listed in the folder's README as needed. OpenAlex knows an open copy of 37 of them: 14 open
at the publisher, 23 as preprints or accepted versions; Hong et al. 1987, Knill et al. 2001,
Raussendorf & Briegel 2001, Ryser 1963, Konno et al. 2024 and Helt et al. 2010 have none it knows
of.

**Already in the folder or its survey list**

- M. Reck, A. Zeilinger, H. J. Bernstein, P. Bertani, Phys. Rev. Lett. 73, 58 (1994). [10.1103/PhysRevLett.73.58](https://doi.org/10.1103/PhysRevLett.73.58) (survey)
- W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walmsley, Optica 3, 1460 (2016). [10.1364/OPTICA.3.001460](https://doi.org/10.1364/OPTICA.3.001460) (folder)
- W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997). [10.1103/PhysRevA.56.1627](https://doi.org/10.1103/PhysRevA.56.1627) (folder)
- J. Zhao, C. Ma, M. Rüsing, S. Mookherjea, Phys. Rev. Lett. 124, 163603 (2020). [10.1103/PhysRevLett.124.163603](https://doi.org/10.1103/PhysRevLett.124.163603) (folder)

**Discrete variables and fusion**

- C. K. Hong, Z. Y. Ou, L. Mandel, Phys. Rev. Lett. 59, 2044 (1987). [10.1103/PhysRevLett.59.2044](https://doi.org/10.1103/PhysRevLett.59.2044)
- E. Knill, R. Laflamme, G. J. Milburn, Nature 409, 46 (2001). [10.1038/35051009](https://doi.org/10.1038/35051009)
- R. Raussendorf, H. J. Briegel, Phys. Rev. Lett. 86, 5188 (2001). [10.1103/PhysRevLett.86.5188](https://doi.org/10.1103/PhysRevLett.86.5188)
- D. E. Browne, T. Rudolph, Phys. Rev. Lett. 95, 010501 (2005). [10.1103/PhysRevLett.95.010501](https://doi.org/10.1103/PhysRevLett.95.010501)
- M. Varnava, D. E. Browne, T. Rudolph, Phys. Rev. Lett. 100, 060502 (2008). [10.1103/PhysRevLett.100.060502](https://doi.org/10.1103/PhysRevLett.100.060502)
- F. Ewert, P. van Loock, Phys. Rev. Lett. 113, 140403 (2014). [10.1103/PhysRevLett.113.140403](https://doi.org/10.1103/PhysRevLett.113.140403)
- S. Bartolucci et al., Nat. Commun. 14, 912 (2023). [10.1038/s41467-023-36493-1](https://doi.org/10.1038/s41467-023-36493-1)

**Boson sampling, its algorithms and its experiments**

- S. Aaronson, A. Arkhipov, Theory Comput. 9, 143 (2013). [10.4086/toc.2013.v009a004](https://doi.org/10.4086/toc.2013.v009a004)
- H. J. Ryser, *Combinatorial Mathematics*, Carus Mathematical Monographs 14, MAA (1963). [10.5948/UPO9781614440147](https://doi.org/10.5948/UPO9781614440147)
- D. G. Glynn, Eur. J. Combin. 31, 1887 (2010). [10.1016/j.ejc.2010.01.010](https://doi.org/10.1016/j.ejc.2010.01.010)
- P. Clifford, R. Clifford, Proc. 29th ACM-SIAM Symp. Discrete Algorithms (SODA 2018), 146. [10.1137/1.9781611975031.10](https://doi.org/10.1137/1.9781611975031.10)
- C. S. Hamilton et al., Phys. Rev. Lett. 119, 170501 (2017). [10.1103/PhysRevLett.119.170501](https://doi.org/10.1103/PhysRevLett.119.170501)
- N. Quesada, J. M. Arrazola, N. Killoran, Phys. Rev. A 98, 062322 (2018). [10.1103/PhysRevA.98.062322](https://doi.org/10.1103/PhysRevA.98.062322)
- A. Björklund, B. Gupt, N. Quesada, ACM J. Exp. Algorithmics 24, 1 (2019). [10.1145/3325111](https://doi.org/10.1145/3325111)
- H. Qi et al., Phys. Rev. Lett. 124, 100502 (2020). [10.1103/PhysRevLett.124.100502](https://doi.org/10.1103/PhysRevLett.124.100502)
- C. Oh et al., Nat. Phys. 20, 1461 (2024). [10.1038/s41567-024-02535-8](https://doi.org/10.1038/s41567-024-02535-8)
- H.-S. Zhong et al., Science 370, 1460 (2020). [10.1126/science.abe8770](https://doi.org/10.1126/science.abe8770)
- L. S. Madsen et al., Nature 606, 75 (2022). [10.1038/s41586-022-04725-x](https://doi.org/10.1038/s41586-022-04725-x)
- H.-L. Liu et al., Nature 653, 687 (2026). [10.1038/s41586-026-10523-6](https://doi.org/10.1038/s41586-026-10523-6) (Jiuzhang 4.0; preprint arXiv:2508.09092)
- A. Peruzzo et al., Science 329, 1500 (2010). [10.1126/science.1193515](https://doi.org/10.1126/science.1193515)

**Continuous variables and GKP qubits**

- D. Gottesman, A. Kitaev, J. Preskill, Phys. Rev. A 64, 012310 (2001). [10.1103/PhysRevA.64.012310](https://doi.org/10.1103/PhysRevA.64.012310)
- N. C. Menicucci et al., Phys. Rev. Lett. 97, 110501 (2006). [10.1103/PhysRevLett.97.110501](https://doi.org/10.1103/PhysRevLett.97.110501)
- C. Weedbrook et al., Rev. Mod. Phys. 84, 621 (2012). [10.1103/RevModPhys.84.621](https://doi.org/10.1103/RevModPhys.84.621)
- W. Asavanant et al., Science 366, 373 (2019). [10.1126/science.aay2645](https://doi.org/10.1126/science.aay2645)
- K. Fukui et al., Phys. Rev. X 8, 021054 (2018). [10.1103/PhysRevX.8.021054](https://doi.org/10.1103/PhysRevX.8.021054)
- S. Konno et al., Science 383, 289 (2024). [10.1126/science.adk7560](https://doi.org/10.1126/science.adk7560)
- M. V. Larsen et al., Nature 642, 587 (2025). [10.1038/s41586-025-09044-5](https://doi.org/10.1038/s41586-025-09044-5)
- H. Aghaee Rad et al., Nature 638, 912 (2025). [10.1038/s41586-024-08406-9](https://doi.org/10.1038/s41586-024-08406-9)

**Hardware: chips, sources, emitters, detectors**

- J. Wang, F. Sciarrino, A. Laing, M. G. Thompson, Nat. Photonics 14, 273 (published online 21 October 2019; the May 2020 issue). [10.1038/s41566-019-0532-1](https://doi.org/10.1038/s41566-019-0532-1)
- K. Alexander et al. (PsiQuantum), Nature 641, 876 (2025). [10.1038/s41586-025-08820-7](https://doi.org/10.1038/s41586-025-08820-7)
- N. Maring et al., Nat. Photonics 18, 603 (2024). [10.1038/s41566-024-01403-4](https://doi.org/10.1038/s41566-024-01403-4)
- J. Carolan et al., Science 349, 711 (2015). [10.1126/science.aab3642](https://doi.org/10.1126/science.aab3642)
- J. M. Arrazola et al., Nature 591, 54 (2021). [10.1038/s41586-021-03202-1](https://doi.org/10.1038/s41586-021-03202-1)
- J. W. Silverstone et al., Nat. Photonics 8, 104 (2014). [10.1038/nphoton.2013.339](https://doi.org/10.1038/nphoton.2013.339)
- S. Paesani et al., Nat. Commun. 11, 2505 (2020). [10.1038/s41467-020-16187-8](https://doi.org/10.1038/s41467-020-16187-8)
- L. G. Helt et al., Opt. Lett. 35, 3006 (2010). [10.1364/OL.35.003006](https://doi.org/10.1364/OL.35.003006)
- Z. Vernon et al., Opt. Lett. 42, 3638 (2017). [10.1364/OL.42.003638](https://doi.org/10.1364/OL.42.003638)
- V. D. Vaidya et al., Sci. Adv. 6, eaba9186 (2020). [10.1126/sciadv.aba9186](https://doi.org/10.1126/sciadv.aba9186)
- R. Nehra et al., Science 377, 1333 (2022). [10.1126/science.abo6213](https://doi.org/10.1126/science.abo6213)
- N. Tomm et al., Nat. Nanotechnol. 16, 399 (2021). [10.1038/s41565-020-00831-x](https://doi.org/10.1038/s41565-020-00831-x)
- M. Arcari et al., Phys. Rev. Lett. 113, 093603 (2014). [10.1103/PhysRevLett.113.093603](https://doi.org/10.1103/PhysRevLett.113.093603)
- D. V. Reddy et al., Optica 7, 1649 (2020). [10.1364/OPTICA.400751](https://doi.org/10.1364/OPTICA.400751)
- W. H. P. Pernice et al., Nat. Commun. 3, 1325 (2012). [10.1038/ncomms2307](https://doi.org/10.1038/ncomms2307)

**Software papers named in Section 2** (not added to the folder)

- N. Heurtel et al., Quantum 7, 931 (2023). [10.22331/q-2023-02-21-931](https://doi.org/10.22331/q-2023-02-21-931)
- N. Killoran et al., Quantum 3, 129 (2019). [10.22331/q-2019-03-11-129](https://doi.org/10.22331/q-2019-03-11-129)
- B. Gupt, J. Izaac, N. Quesada, J. Open Source Softw. 4, 1705 (2019). [10.21105/joss.01705](https://doi.org/10.21105/joss.01705)

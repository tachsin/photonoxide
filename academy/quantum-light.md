---
title: "Quantum light on a chip"
summary: "Single photons and squeezed light in linear-optical circuits: two photons on a coupler, meshes that make any unitary, permanents and boson sampling, what loss does, heralded photons from pair sources, squeezed light and Gaussian boson sampling, and the routes to computing with photons, each worked out with photonoxide's quantum module and checked against the closed forms and the published numbers, with those it doesn't reproduce stated."
topic: Active and quantum photonics
level: advanced
minutes: 75
prerequisites: [mach-zehnder, nonlinear-optics]
circuits: [mzi.toml]
methods: [quantum.md]
validation:
  - quantum/hom-visibility
  - quantum/hom-dip-gaussian
  - quantum/hom-delay-line
  - quantum/coupler-hom-order
  - quantum/mesh-reconstruction
  - quantum/mesh-depth
  - quantum/clements-fig3-size
  - quantum/clements-fig3-loss
  - quantum/patterns-sum
  - quantum/full-bunching
  - quantum/partly-distinguishable
  - quantum/peruzzo-correlations
  - quantum/peruzzo-classical-bound
  - quantum/sampler-convergence
  - quantum/dilation
  - quantum/budget-path
  - quantum/spectral-hom
  - quantum/touchstone-statistics
  - quantum/varnava-threshold
  - quantum/bartolucci-fusion
  - quantum/source-pairs
  - quantum/source-hom-gaussian
  - quantum/heralding-coupler
  - quantum/ring-helt-short
  - quantum/ring-helt-long
  - quantum/ring-vernon-equal
  - quantum/silverstone-fringe
  - quantum/squeezed-vacuum
  - quantum/two-mode-thermal
  - quantum/squeezed-loss
  - quantum/vaidya-number-difference
  - quantum/vaidya-outcoupling
  - quantum/nehra-inferred
  - quantum/nehra-all-optical
  - quantum/nehra-homodyne-bound
  - quantum/qi-simulability
charts: [hom-visibility, hom-dip, mesh-fidelity, boson-distribution, permanent-cost, loss-varnava, nl-jsa, pair-statistics, ring-pairs, squeezing-loss, squeezed-photons, gbs-simulability, fusion-loss]
papers:
  - cite: "C. K. Hong, Z. Y. Ou, L. Mandel, Phys. Rev. Lett. 59, 2044 (1987)"
    title: "Measurement of subpicosecond time intervals between two photons by interference"
    doi: 10.1103/PhysRevLett.59.2044
    year: 1987
    role: origin
    note: "Signal and idler photons from down-conversion in KDP meet on a beam splitter of R/T = 0.95; moving it by a few micrometres makes their coincidences fall to a few per cent: a dip about 16 µm wide at half height, a wave packet of about 100 fs (their Eq. 11 and Fig. 2)."
  - cite: "M. Reck, A. Zeilinger, H. J. Bernstein, P. Bertani, Phys. Rev. Lett. 73, 58 (1994)"
    title: "Experimental realization of any discrete unitary operator"
    doi: 10.1103/PhysRevLett.73.58
    year: 1994
    role: milestone
    note: "Any unitary of N modes from a triangle of beam splitters and phase shifters. Known here from Clements et al.'s account and the decomposition photonoxide implements; its PDF is not in the papers folder."
  - cite: "W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997)"
    title: "Spectral information and distinguishability in type-II down-conversion with a broadband pump"
    doi: 10.1103/PhysRevA.56.1627
    year: 1997
    role: milestone
    note: "Down-conversion pumped by a broadband source: the crystal decides how the pump's energies are shared between the photons, which are then no longer exactly anticorrelated; the joint spectrum this lesson's pairs are made of."
  - cite: "E. Knill, R. Laflamme, G. J. Milburn, Nature 409, 46 (2001)"
    title: "A scheme for efficient quantum computation with linear optics"
    doi: 10.1038/35051009
    year: 2001
    role: milestone
    note: "Computing with beam splitters, phase shifters, single photons and detectors: a nonlinear sign shift that succeeds with 1/4 from a 3 × 3 unitary (their Fig. 1), a conditional sign flip from two of them (1/16), and teleportation that raises the success towards 1."
  - cite: "R. Raussendorf, H. J. Briegel, Phys. Rev. Lett. 86, 5188 (2001)"
    title: "A One-Way Quantum Computer"
    doi: 10.1103/PhysRevLett.86.5188
    year: 2001
    role: milestone
    note: "Computing by one-qubit measurements alone on an entangled cluster state, the measurements the program: the model fusion-based photonic architectures build on."
  - cite: "D. Gottesman, A. Kitaev, J. Preskill, Phys. Rev. A 64, 012310 (2001)"
    title: "Encoding a qubit in an oscillator"
    doi: 10.1103/PhysRevA.64.012310
    year: 2001
    role: origin
    note: "Codes that protect a qubit in an oscillator against small shifts of its position and momentum; in optics the encoded states need a nonlinear element to make, after which linear optics, squeezing and homodyne detection compute on them."
  - cite: "D. E. Browne, T. Rudolph, Phys. Rev. Lett. 95, 010501 (2005)"
    title: "Resource-Efficient Linear Optical Quantum Computation"
    doi: 10.1103/PhysRevLett.95.010501
    year: 2005
    role: milestone
    note: "Fusion gates that join small photonic cluster states: the type-II fusion succeeds with probability 1/2 and its failures are benign, so clusters grow far more cheaply than with KLM's gates."
  - cite: "N. C. Menicucci, P. van Loock, M. Gu, C. Weedbrook, T. C. Ralph, M. A. Nielsen, Phys. Rev. Lett. 97, 110501 (2006)"
    title: "Universal Quantum Computation with Continuous-Variable Cluster States"
    doi: 10.1103/PhysRevLett.97.110501
    year: 2006
    role: milestone
    note: "Cluster states of squeezed light: Gaussian operations by homodyne detection alone, universality by one non-Gaussian measurement."
  - cite: "M. Varnava, D. E. Browne, T. Rudolph, Phys. Rev. Lett. 100, 060502 (2008)"
    title: "How Good Must Single Photon Sources and Detectors Be for Efficient Linear Optical Quantum Computation?"
    doi: 10.1103/PhysRevLett.100.060502
    year: 2008
    role: milestone
    note: "Loss-tolerant tree clusters grown by type-II fusion: efficient computation when the product of the sources' and detectors' efficiencies exceeds 2/3."
  - cite: "A. Politi, M. J. Cryan, J. G. Rarity, S. Yu, J. L. O'Brien, Science 320, 646 (2008)"
    title: "Silica-on-Silicon Waveguide Quantum Circuits"
    doi: 10.1126/science.1155441
    year: 2008
    role: milestone
    note: "Silica-on-silicon waveguides at 804 nm: two-photon interference on a directional coupler with a visibility of 94.8 ± 0.5 % and a CNOT gate with an average logical-basis fidelity of 94.3 ± 0.2 % (abstract, p. 646). Wang et al.'s review calls it a landmark of integrated quantum photonics (p. 1)."
  - cite: "A. Peruzzo et al., Science 329, 1500 (2010)"
    title: "Quantum Walks of Correlated Photons"
    doi: 10.1126/science.1193515
    year: 2010
    role: milestone
    note: "Two photons in an array of 21 coupled SiOxNy guides: their correlations violate a classical bound by up to 76 standard deviations, and depend on where the photons enter."
  - cite: "L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010)"
    title: "Spontaneous four-wave mixing in microring resonators"
    doi: 10.1364/OL.35.003006
    year: 2010
    role: milestone
    note: "Photon pairs from a ring: their spectrum the product of the rings' field enhancements, and a Schmidt number from 1.09 for short pump pulses to 10.00 for long ones."
  - cite: "S. Aaronson, A. Arkhipov, Theory Comput. 9, 143 (2013)"
    title: "The Computational Complexity of Linear Optics"
    doi: 10.4086/toc.2013.v009a004
    year: 2013
    role: origin
    note: "Boson sampling: identical photons through a random linear-optical network, counted at its outputs, sample a distribution of permanents that no efficient classical algorithm could sample unless the polynomial hierarchy collapses. An extended abstract appeared at STOC 2011."
  - cite: "D. G. Glynn, Eur. J. Combin. 31, 1887 (2010)"
    title: "The permanent of a square matrix"
    doi: 10.1016/j.ejc.2010.01.010
    year: 2010
    role: milestone
    note: "A formula for the permanent as a sum over sign vectors, half the terms of Ryser's: the one photonoxide computes permanents by."
  - cite: "C. Weedbrook, S. Pirandola, R. García-Patrón, N. J. Cerf, T. C. Ralph, J. H. Shapiro, S. Lloyd, Rev. Mod. Phys. 84, 621 (2012)"
    title: "Gaussian quantum information"
    doi: 10.1103/RevModPhys.84.621
    year: 2012
    role: review
    note: "Gaussian states, their covariance matrices and the channels that act on them, loss among them: the conventions of photonoxide's Gaussian states."
  - cite: "M. A. Broome et al., Science 339, 794 (2013)"
    title: "Photonic Boson Sampling in a Tunable Circuit"
    doi: 10.1126/science.1231440
    year: 2013
    role: milestone
    note: "One of the first boson-sampling experiments: two and three photons in six modes, the two polarizations in each of the three spatial modes of a fused-fibre 3 × 3 beam splitter with polarization controllers before and after it, so not a chip, though its abstract speaks of a 6-mode integrated optical circuit. Published online in December 2012; read from that first version, so no page numbers are given."
  - cite: "J. B. Spring et al., Science 339, 798 (2013)"
    title: "Boson Sampling on a Photonic Chip"
    doi: 10.1126/science.1231692
    year: 2013
    role: milestone
    note: "One of the first boson-sampling experiments, on a chip: three photons and, on a second chip of the same geometry, four (|202000⟩, a double pair from one source), in a silica-on-silicon circuit written by UV laser with M = 6 modes and 10 beam splitters (abstract, p. 798; p. 799). Their L1 distances to the permanents' prediction are d(3) = 0.094 ± 0.014 and d(4) = 0.097 ± 0.004 (p. 800). Published online in December 2012."
  - cite: "M. Tillmann et al., Nat. Photonics 7, 540 (2013)"
    title: "Experimental boson sampling"
    doi: 10.1038/nphoton.2013.102
    year: 2013
    role: milestone
    note: "One of the first boson-sampling experiments: three photons in networks of 5 × 5 modes, 8 directional couplers and 11 phase shifters, written by femtosecond laser in fused silica (PDF p. 2); the measured three-photon distributions agree with the predicted ones to fidelities of 0.98, 0.97, 0.98 and 0.96, each ± 0.01 (Fig. 4, PDF p. 3). Read from the advance online version of 12 May 2013, whose pages are numbered 1–5."
  - cite: "A. Crespi et al., Nat. Photonics 7, 545 (2013)"
    title: "Integrated multimode interferometers with arbitrary designs for photonic boson sampling"
    doi: 10.1038/nphoton.2013.112
    year: 2013
    role: milestone
    note: "One of the first boson-sampling experiments: three photons in five modes, a random 5 × 5 unitary built as Reck et al.'s triangle of 10 directional couplers, written by femtosecond laser in borosilicate glass (PDF pp. 1–2, 5); the three-photon distribution lies at a variation distance of 0.105 ± 0.024 from the prediction of the reconstructed unitary with the photons' partial distinguishability (PDF p. 3). Read from the advance online version of 26 May 2013, whose pages are numbered 1–5."
  - cite: "J. W. Silverstone et al., Nat. Photonics 8, 104 (published online 15 December 2013)"
    title: "On-chip quantum interference between silicon photon-pair sources"
    doi: 10.1038/nphoton.2013.339
    year: 2013
    role: milestone
    note: "Two silicon spiral sources between two MMIs: their pairs interfere on the chip with up to 100.0 ± 0.4 % visibility."
  - cite: "F. Ewert, P. van Loock, Phys. Rev. Lett. 113, 140403 (2014)"
    title: "3/4-Efficient Bell Measurement with Passive Linear Optics and Unentangled Ancillae"
    doi: 10.1103/PhysRevLett.113.140403
    year: 2014
    role: milestone
    note: "Static linear optics tells apart at most half of the Bell states; four ancilla single photons on 50:50 beam splitters raise it to 75 %, the boosted fusion of later architectures."
  - cite: "J. Carolan et al., Science 349, 711 (2015)"
    title: "Universal linear optics"
    doi: 10.1126/science.aab3642
    year: 2015
    role: milestone
    note: "A six-mode chip of 15 Mach–Zehnder interferometers and 30 thermo-optic phase shifters, reprogrammable to any linear-optical protocol of its size."
  - cite: "W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walmsley, Optica 3, 1460 (2016)"
    title: "Optimal design for universal multiport interferometers"
    doi: 10.1364/OPTICA.3.001460
    year: 2016
    role: milestone
    note: "A rectangle of beam splitters that makes any unitary with half the optical depth of Reck et al.'s triangle and far more robust to loss (their Fig. 3)."
  - cite: "P. Clifford, R. Clifford, Proc. 29th ACM-SIAM Symp. Discrete Algorithms (SODA 2018), 146"
    title: "The Classical Complexity of Boson Sampling"
    doi: 10.1137/1.9781611975031.10
    year: 2018
    role: milestone
    note: "An exact sampler of boson sampling in O(n2ⁿ) a sample beside a polynomial in the modes, the photons' outputs drawn one at a time: the algorithm the studio's Sampling part runs."
  - cite: "C. S. Hamilton, R. Kruse, L. Sansoni, S. Barkhofen, C. Silberhorn, I. Jex, Phys. Rev. Lett. 119, 170501 (2017)"
    title: "Gaussian Boson Sampling"
    doi: 10.1103/PhysRevLett.119.170501
    year: 2017
    role: origin
    note: "The probability of a photon pattern from a Gaussian state is a hafnian: boson sampling with squeezed light."
  - cite: "Z. Vernon et al., Opt. Lett. 42, 3638 (2017)"
    title: "Truly unentangled photon pairs without spectral filtering"
    doi: 10.1364/OL.42.003638
    year: 2017
    role: milestone
    note: "A ring whose pump resonance a second channel widens: pairs with a Schmidt number of 1.0009 (purity 0.999) without filters, by their model."
  - cite: "K. Fukui, A. Tomita, A. Okamoto, K. Fujii, Phys. Rev. X 8, 021054 (2018)"
    title: "High-Threshold Fault-Tolerant Quantum Computation with Analog Quantum Error Correction"
    doi: 10.1103/PhysRevX.8.021054
    year: 2018
    role: milestone
    note: "Fault-tolerant computing with GKP qubits needs 14.8 dB of squeezing in the schemes before it; their analog error correction lowers it below 10 dB, numerically."
  - cite: "W. Asavanant et al., Science 366, 373 (2019)"
    title: "Generation of time-domain-multiplexed two-dimensional cluster state"
    doi: 10.1126/science.aay2645
    year: 2019
    role: milestone
    note: "A 5 × 1248-site two-dimensional cluster state of squeezed light multiplexed in time, every nullifier below −4.5 dB."
  - cite: "S. Paesani, Y. Ding, R. Santagati et al., Nat. Phys. 15, 925 (2019)"
    title: "Generation and sampling of quantum states of light in a silicon chip"
    doi: 10.1038/s41567-019-0567-8
    year: 2019
    role: milestone
    note: "Four spiral photon-pair sources, a random walk over 12 coupled guides and 16 nanowire detectors of about 78 % efficiency (PDF p. 1): scattershot and Gaussian boson sampling on one silicon chip, up to eight photons; one of the two experiments Qi et al. test. Read from the first release, whose pages carry no numbers."
  - cite: "H.-S. Zhong, L.-C. Peng, Y. Li et al., Sci. Bull. 64, 511 (2019)"
    title: "Experimental Gaussian Boson sampling"
    doi: 10.1016/j.scib.2019.04.007
    year: 2019
    role: milestone
    note: "Three two-mode squeezed vacua from down-conversion through a random 12 × 12 transformation of six spatial modes and two polarizations, read by 12 nanowire detectors of 75 % average efficiency at 1550 nm (p. 512): Gaussian boson sampling of three to five photons (p. 513); the other experiment Qi et al. test."
  - cite: "J. Wang, F. Sciarrino, A. Laing, M. G. Thompson, Nat. Photonics 14, 273 (published online 21 October 2019)"
    title: "Integrated photonic quantum technologies"
    doi: 10.1038/s41566-019-0532-1
    year: 2019
    role: review
    note: "Sources, circuits and detectors on chips and what they had done by 2019, the first chips and boson-sampling experiments among them."
  - cite: "H. Qi, D. J. Brod, N. Quesada, R. García-Patrón, Phys. Rev. Lett. 124, 100502 (2020)"
    title: "Regimes of Classical Simulability for Noisy Gaussian Boson Sampling"
    doi: 10.1103/PhysRevLett.124.100502
    year: 2020
    role: milestone
    note: "A test on squeezing, transmission and detectors below which a Gaussian boson sampler can be sampled classically, applied to two experiments of 2019."
  - cite: "S. Paesani, M. Borghi, S. Signorini, A. Maïnos, L. Pavesi, A. Laing, Nat. Commun. 11, 2505 (2020)"
    title: "Near-ideal spontaneous photon sources in silicon quantum photonics"
    doi: 10.1038/s41467-020-16187-8
    year: 2020
    role: current
    note: "Inter-modal four-wave mixing in a multimode silicon guide, one pump delayed: a measured purity of 0.9904 ± 0.0006 without filters."
  - cite: "V. D. Vaidya et al., Sci. Adv. 6, eaba9186 (2020)"
    title: "Broadband quadrature-squeezed vacuum and nonclassical photon number correlations from a nanophotonic device"
    doi: 10.1126/sciadv.aba9186
    year: 2020
    role: current
    note: "Squeezed light from silicon nitride rings: about 1 dB measured and about 4 dB inferred on the chip, and photon-number correlations below the shot noise."
  - cite: "H.-S. Zhong et al., Science 370, 1460 (2020)"
    title: "Quantum computational advantage using photons"
    doi: 10.1126/science.abe8770
    year: 2020
    role: current
    note: "Gaussian boson sampling with 50 squeezed states in a 100-mode interferometer, up to 76 photon clicks."
  - cite: "J. M. Arrazola et al., Nature 591, 54 (2021)"
    title: "Quantum circuits with many photons on a programmable nanophotonic chip"
    doi: 10.1038/s41586-021-03202-1
    year: 2021
    role: current
    note: "Squeezers, a programmable interferometer and eight photon-number-resolving detectors around one silicon nitride chip."
  - cite: "L. S. Madsen et al., Nature 606, 75 (2022)"
    title: "Quantum computational advantage with a programmable photonic processor"
    doi: 10.1038/s41586-022-04725-x
    year: 2022
    role: current
    note: "Gaussian boson sampling on 216 squeezed modes multiplexed in time, events with up to 219 photons and a mean of 125."
  - cite: "P. H. Lundow, K. Markström, J. Comput. Phys. 455, 110990 (2022)"
    title: "Efficient computation of permanents, with applications to Boson sampling and random matrices"
    doi: 10.1016/j.jcp.2022.110990
    year: 2022
    role: review
    note: "Ryser's and Glynn's formulas in Gray-code order and their accuracy in floating point: where photonoxide reads Ryser's formula, his 1963 book being out of print."
  - cite: "R. Nehra et al., Science 377, 1333 (2022)"
    title: "Few-cycle vacuum squeezing in nanophotonics"
    doi: 10.1126/science.abo6213
    year: 2022
    role: current
    note: "Squeezing in thin-film lithium niobate measured by a second, on-chip amplifier: up to 4.9 dB over more than 25 THz, about 11 dB inferred."
  - cite: "S. Bartolucci et al., Nat. Commun. 14, 912 (2023)"
    title: "Fusion-based quantum computation"
    doi: 10.1038/s41467-023-36493-1
    year: 2023
    role: current
    note: "Computing by fusions on small resource states: a network that tolerates 10.4 % of loss per fusion, 2.7 % per photon."
  - cite: "S. Konno et al., Science 383, 289 (2024)"
    title: "Logical states for fault-tolerant quantum computation with propagating light"
    doi: 10.1126/science.adk7560
    year: 2024
    role: current
    note: "GKP states of travelling light, their effective squeezing about 2.5 dB."
  - cite: "C. Oh, M. Liu, Y. Alexeev, B. Fefferman, L. Jiang, Nat. Phys. 20, 1461 (2024)"
    title: "Classical algorithm for simulating experimental Gaussian boson sampling"
    doi: 10.1038/s41567-024-02535-8
    year: 2024
    role: current
    note: "A tensor-network algorithm whose cost falls as the loss grows, which samples the largest Gaussian boson sampling experiments of its time."
  - cite: "K. Alexander et al. (PsiQuantum), Nature 641, 876 (2025)"
    title: "A manufacturable platform for photonic quantum computing"
    doi: 10.1038/s41586-025-08820-7
    year: 2025
    role: current
    note: "Silicon photonics in a foundry: HOM interference of independent sources at 99.50 ± 0.25 % and two-qubit fusion at 99.22 ± 0.12 % fidelity, conditional on detection and not accounting for loss."
  - cite: "M. V. Larsen et al., Nature 642, 587 (2025)"
    title: "Integrated photonic source of Gottesman–Kitaev–Preskill qubits"
    doi: 10.1038/s41586-025-09044-5
    year: 2025
    role: current
    note: "GKP states from a multilayer silicon nitride chip and photon-number-resolving detectors: four resolvable peaks in each quadrature and a 3 × 3 grid of negative regions in the Wigner function."
  - cite: "H. Aghaee Rad et al., Nature 638, 912 (2025)"
    title: "Scaling and networking a modular photonic quantum computer"
    doi: 10.1038/s41586-024-08406-9
    year: 2025
    role: current
    note: "A modular machine of silicon nitride chips (84 squeezers in 42 Gaussian boson sampling cells) and thin-film lithium niobate multiplexers, networked by fibre, following the GKP route."
  - cite: "H.-L. Liu et al., Nature 653, 687 (2026)"
    title: "Gaussian boson sampling with 1,024 squeezed states in 8,176 modes"
    doi: 10.1038/s41586-026-10523-6
    year: 2026
    role: current
    note: "Jiuzhang 4.0: 1,024 squeezed states in a circuit of 8,176 modes encoded in space and time, 92 % source efficiency and 51 % over the whole system."
---

A chip that works on quantum light, in the symbols of this lesson: a pump making photon pairs in
rings, a mesh of Mach–Zehnder cells that mixes four modes, detectors at its outputs; and beneath
it, the simplest quantum circuit there is, two photons meeting on a coupler.

::diagram quantum

A linear circuit does to photons what it does to light: its S-matrix mixes the modes. What changes
is what the detectors see. Two photons that meet on a balanced coupler always leave together;
photons through a large interferometer arrive in patterns whose probabilities are permanents,
which no known algorithm computes in polynomial time; squeezed light has less noise in one
quadrature than the vacuum, and loss gives that noise back. Every number below comes from
photonoxide's [quantum module](../docs/methods/quantum.md), which takes a circuit's S-matrix and
computes the photons' statistics exactly.

## 1. What happens when two photons meet at a coupler?

A lossless coupler keeps a share $R$ of the light in its guide and sends $T = 1 - R$ across. Send
one photon into each input. Either both stay, or both cross, or one stays and one crosses. If the
photons are alike in everything the detectors don't see (their spectra, polarizations and arrival
times), "both stay" and "both cross" lead to the same final state, one photon in each output, and
their amplitudes add before they are squared. The coupler's phases make them subtract: the
coincidences between the outputs are $(R - T)^2$, zero on a balanced coupler. Photons that can be
told apart don't interfere, and their coincidences are $R^2 + T^2$, a half on a balanced coupler.

::chart hom-visibility

The **visibility** compares the two, $V = 1 - P_\text{alike}/P_\text{apart}$. For photons alike it
is $2RT/(R^2 + T^2)$: 1 on a balanced coupler, 0.7241 at $T = 0.3$. Photons whose internal states
overlap by $\lvert\langle\psi_1\vert\psi_2\rangle\rvert^2$ give $R^2 + T^2 - 2RT\lvert\langle\psi_1\vert\psi_2\rangle\rvert^2$,
so on a balanced coupler the visibility *is* the overlap: the chart's 0.8 for photons that overlap
by 0.8. A visibility is how experiments measure how alike two photons are.

### Why the amplitudes subtract {theory}

The coupler acts on the modes' annihilation operators as it does on fields: the operators leaving
are $\hat b = S\hat a$, $S$ its 2 × 2 S-matrix
([the S-matrix as a mode transformation](../docs/methods/quantum.md)). The creation operators
transform the other way round, $\hat a_j^\dagger = \sum_k S_{kj}\hat b_k^\dagger$, since a unitary's
inverse is its adjoint. Take the symmetric coupler of [the directional coupler](directional-coupler.md),
$S = \begin{pmatrix} r & it \\ it & r \end{pmatrix}$ with $r = \sqrt R$ and $t = \sqrt T$. One photon
in each input is $\hat a_1^\dagger\hat a_2^\dagger\lvert 0\rangle$, and

$$
\hat a_1^\dagger\hat a_2^\dagger = (r\hat b_1^\dagger + it\hat b_2^\dagger)(it\hat b_1^\dagger + r\hat b_2^\dagger)
= irt\,\hat b_1^{\dagger 2} + (r^2 - t^2)\,\hat b_1^\dagger\hat b_2^\dagger + irt\,\hat b_2^{\dagger 2} .
$$

The middle term is one photon in each output, with amplitude $R - T$; the other two put both
photons in one output, each $\hat b^{\dagger 2}\lvert 0\rangle = \sqrt 2\lvert 2\rangle$, with
probability $2RT$ each. The three add to $(R - T)^2 + 4RT = 1$. The amplitude $r^2 - t^2$ is
$S_{11}S_{22} + S_{12}S_{21}$: the determinant's two products, added instead of subtracted. That is
the **permanent** of $S$, and section 3 shows it for any number of photons.

For photons told apart each crosses or stays on its own, and the coincidences are the chance that
both stay plus the chance that both cross, $R^2 + T^2$. Hong, Ou and Mandel wrote the same,
$N_c = C(T^2 + R^2)[1 - 2RT/(R^2 + T^2)\,e^{-(\Delta\omega\,\delta\tau)^2}]$ for their Gaussian
spectra ([Hong et al., 1987](https://doi.org/10.1103/PhysRevLett.59.2044), Eq. 11).

### The dip against the delay {theory}

Delay one photon by $\tau$ and the two arrive told apart by their arrival times, so the dip comes
back up. With spectral amplitudes $\varphi_1(\omega)$ and $\varphi_2(\omega)$, photon 1 delayed,
through a coupler that doesn't change with frequency, the coincidence probability is

$$
P(\tau) = R^2 + T^2 - 2RT\,\Big\lvert \int \varphi_1(\omega)\,\overline{\varphi_2(\omega)}\,e^{i\omega\tau}\,d\omega \Big\rvert^2 ,
$$

the overlap of the two photons with one shifted in time. For two transform-limited Gaussians of
the same width, $\lvert\varphi\rvert^2 \propto e^{-(\omega - \omega_0)^2/2\sigma^2}$, whose centres
lie $\Delta$ apart, the integral is a Gaussian's Fourier transform, and

$$
P(\tau) = R^2 + T^2 - 2RT\,e^{-\Delta^2/4\sigma^2}\,e^{-\sigma^2\tau^2} .
$$

The dip is $2\sqrt{\ln 2}/\sigma$ wide at half depth: 5.00 ps for photons 1 nm wide at 1.55 µm.
Moving one photon's centre lowers the dip by its overlap, $e^{-\Delta^2/4\sigma^2}$, without
widening it. The chart computes it as the library does for any circuit: `hom_dip` takes the
circuit's S-matrices over a sweep of wavelengths and integrates by the trapezoidal rule, so a
coupler whose split changes with wavelength, or a delay line before one input, is included as it
is ([two-photon interference with spectra](../docs/methods/quantum.md)).

::chart hom-dip

**Try it.** Set the coupling to 0.3 and the offset to 0.5 nm. What visibility do you expect, and
does the dip get wider?

:::answer
$2RT/(R^2 + T^2) = 0.42/0.58 = 0.7241$ times the spectral overlap $e^{-\Delta^2/4\sigma^2}$. For
1 nm photons the offset of 0.5 nm is half the FWHM, so
$\Delta/\sigma = 0.5 \cdot 2\sqrt{2\ln 2} = 1.177$ and the overlap is $e^{-\ln 2/2} = 0.707$: a
visibility of 0.512. The width stays
$2\sqrt{\ln 2}/\sigma$: the overlap scales the dip but doesn't change its shape.
:::

### What has been measured {research}

Hong, Ou and Mandel sent signal and idler photons from down-conversion in an 8 cm KDP crystal onto
a beam splitter of $R/T = 0.95$, so $2RT/(R^2 + T^2) = 0.999$; the chart gives 0.99869. Moving the
beam splitter made the coincidences fall to a few per cent of their level in the wings, a dip about
16 µm wide at half height, a wave packet of about 100 fs, fitted by their Eq. 11 with
$\Delta\omega = 3 \times 10^{13}$ rad/s ([Hong et al., 1987](https://doi.org/10.1103/PhysRevLett.59.2044),
Fig. 2). On chips the same interference measures sources: two independent silicon sources reach
99.50 ± 0.25 % ([Alexander et al., 2025](https://doi.org/10.1038/s41586-025-08820-7), conditional on
detection).

The validation report holds the coupler's coincidences and visibility to the closed forms, the dip
through a frequency-independent coupler to $R^2 + T^2 - 2RTe^{-\sigma^2\tau^2}$, a delay line's dip
to the guide's group delay, and the visibility of a 3D directional coupler from the mode solver as
its grid is refined:

::validation quantum/hom-visibility quantum/hom-dip-gaussian quantum/hom-delay-line quantum/coupler-hom-order

A visibility from a solver carries the solver's grid. A 2 × 2 MMI, 3 µm wide on 220 nm silicon, cut
where its split is not balanced so that the figure moves, gives two 3 nm photons a visibility of
0.8909 by the guided-mode model, 0.7871, 0.8371 and 0.8528 by 2D FDFD on 40, 20 and 10 nm cells,
and the same to $8 \times 10^{-16}$ through a Touchstone file of the 10 nm solve
([many photons with spectra through S(ω)](../docs/methods/quantum.md)). The FDFD values converge at
about 1.7th order; the guided-mode model, which leaves out radiation and reflection at the MMI's
faces, sits about 0.03 above their limit.

::validation quantum/spectral-hom quantum/touchstone-statistics

## 2. How does a programmable interferometer make any unitary?

A [Mach–Zehnder interferometer](mach-zehnder.md) with a phase shifter in one arm and another before
it is a beam splitter of any reflectivity with any phase: a cell that acts on two neighbouring
modes. [Reck et al. (1994)](https://doi.org/10.1103/PhysRevLett.73.58) showed that a triangle of
$N(N-1)/2$ such cells makes any unitary of $N$ modes;
[Clements et al. (2016)](https://doi.org/10.1364/OPTICA.3.001460) found a rectangle of as many cells
in which every path crosses $N$ of them instead of up to $2N - 3$. Each cell's two phases are
computed from the unitary, column by column, so a chip of cells and heaters becomes any
interferometer of its size: [Carolan et al. (2015)](https://doi.org/10.1126/science.aab3642) made
one of six modes from 15 interferometers and 30 thermo-optic phase shifters.

Loss is where the two designs differ. In Reck's triangle the paths cross different numbers of
cells, so a uniform loss per cell becomes an unbalanced loss between paths, which no reprogramming
removes. In the rectangle every path crosses the same number of cells, so the loss is a common
factor and the unitary keeps its shape. The chart is the fidelity
$\lvert\mathop{\mathrm{tr}}(U^\dagger U_\text{exp})\rvert^2/(N\mathop{\mathrm{tr}}(U_\text{exp}^\dagger U_\text{exp}))$
(Clements et al., Eq. 6), averaged over Haar-random unitaries, as each cell loses the same power on
both outputs.

::chart mesh-fidelity

### Nulling a unitary {theory}

Clements et al. write a cell between modes $m$ and $n = m + 1$ as the identity but for the block

$$
T_{m,n}(\theta, \phi) = \begin{pmatrix} e^{i\phi}\cos\theta & -\sin\theta \\ e^{i\phi}\sin\theta & \cos\theta \end{pmatrix}
$$

in rows and columns $m$ and $n$: a phase $\phi$ on mode $m$, then a beam splitter of reflectivity
$\cos\theta$ (their Eq. 1). Multiplying $U$ from the right by $T_{m,n}^{-1}$ mixes two of its
columns, and $\theta$ and $\phi$ can be chosen to make any one entry of those columns zero; from
the left, $T_{m,n}$ mixes two rows likewise. Reck's triangle nulls the entries of the last row one
by one from the right, then the row above, until a diagonal $D$ of phases is left,
$U = D\prod T$. Clements's rectangle nulls successive diagonals, alternating from the right and
from the left (their Fig. 2), and moves the left ones through $D$ by
$T^{-1}(\theta, \phi)D = D'T(\theta, \phi')$ (their Eq. 5). Either way there are $N(N-1)/2$ cells,
one for each entry below the diagonal, and photonoxide's `Mesh` rebuilds the unitary from them to
$4 \times 10^{-16}$.

::validation quantum/mesh-reconstruction quantum/mesh-depth

### Clements et al.'s Fig. 3 {research}

Their figure gives the mean fidelity over 500 Haar-random unitaries against the modes at 0.2 dB a
cell and against the loss for 20 modes. Read from the figure marker by marker, and computed by
photonoxide over 500 other unitaries:

| Case | Their figure, read | photonoxide |
|---|---|---|
| Reck, 20 modes, 0.5 dB | 0.84260 | 0.84190 |
| Reck, 20 modes, 1 dB | 0.53734 | 0.53624 |
| Clements, 20 modes, 0.5 dB | 0.99498 | 0.99456 |
| Clements, 20 modes, 1 dB | 0.97606 | 0.97599 |

The differences are within one pixel of the figure and three standard errors of the two means
(ours is 0.00073 at 1 dB), over all 68 markers of both panels.

::validation quantum/clements-fig3-size quantum/clements-fig3-loss

**Try it.** At 20 modes and 1 dB a cell, what share of the light does a path through Clements's
rectangle keep, and why is its fidelity still near 1?

:::answer
Every path crosses 20 cells, so it keeps $10^{-20/10} = 0.01$ of the light; but every path keeps the
same 0.01, so the matrix is 0.1 times a unitary, and Eq. 6 divides that factor out. The fidelity
(0.976) falls only through the cells' internal imbalance. In the triangle the longest paths cross
37 cells and others far fewer, so the factor differs from path to path and the fidelity falls to
0.536.
:::

## 3. Why are photon statistics hard to compute?

Send $n$ photons into $n$ of the $m$ inputs of a unitary $U$. The amplitude of a pattern at the
outputs sums over every way the photons can reach it, and the sum is a permanent: for single photons
in the inputs $c_1 \ldots c_n$ and the outputs $r_1 \ldots r_n$,

$$
P = \frac{\lvert\mathop{\mathrm{Per}}(M)\rvert^2}{\prod_r m_r!}, \qquad M_{jk} = U_{r_j c_k},
$$

$m_r$ the photons in output $r$ ([Aaronson and Arkhipov, 2013](https://doi.org/10.4086/toc.2013.v009a004)).
The permanent is the determinant without its signs, $\mathop{\mathrm{Per}}(M) = \sum_\sigma\prod_j M_{j\sigma(j)}$
over the $n!$ permutations. Photons told apart give $\mathop{\mathrm{Per}}(\lvert M\rvert^2)/\prod m_r!$, the
probabilities of classical particles, each photon going its own way.

::chart boson-distribution

The two distributions differ most where photons share a mode. Three photons in a Haar-random
unitary of six modes (seed 1) leave with two or more in one output with probability 0.566 when
alike and 0.328 when told apart, and all three in one output is exactly $3! = 6$ times as likely
alike: for that pattern every term of the permanent is the same product, so
$\lvert\mathop{\mathrm{Per}}\rvert^2 = (n!)^2\prod\lvert U\rvert^2$ against $n!\prod\lvert U\rvert^2$.
This **bunching** is the many-photon version of the coupler's dip.

### Why it is hard {theory}

The determinant is easy because Gaussian elimination keeps it: adding a multiple of one row to
another leaves it unchanged. The permanent has no such invariance. Ryser's formula,
$\mathop{\mathrm{Per}}(A) = (-1)^n\sum_{S\subseteq[n]}(-1)^{\lvert S\rvert}\prod_i\sum_{j\in S}a_{ij}$,
sums over the $2^n$ subsets of columns, and Glynn's over half as many sign vectors; in Gray-code
order consecutive terms differ by one column or one sign, so each costs $n$ operations and the
whole about $n2^n$ ([Lundow and Markström, 2022](https://doi.org/10.1016/j.jcp.2022.110990);
[Glynn, 2010](https://doi.org/10.1016/j.ejc.2010.01.010)). No polynomial algorithm is known, and
Aaronson and Arkhipov showed that sampling the photons' distribution exactly on a classical computer
in polynomial time would collapse the polynomial hierarchy: **boson sampling**, a task photons do by
being sent through. photonoxide samples it exactly by Clifford and Clifford's algorithm, in
$O(n2^n)$ a sample ([Clifford and Clifford, 2018](https://doi.org/10.1137/1.9781611975031.10)),
which is how the studio's Sampling part works.

The chart below times both formulas, and the definition up to $n = 9$, as it is drawn. Each photon
more doubles the time once the threads are busy: the methods page gives 20 ms for $n = 20$ by
Glynn's formula on one thread and about half a minute for $n = 30$.

::chart permanent-cost

::validation quantum/patterns-sum quantum/full-bunching quantum/partly-distinguishable quantum/sampler-convergence

### Experiments {research}

Two photons in a chip of 21 coupled guides ([Peruzzo et al., 2010](https://doi.org/10.1126/science.1193515))
were the first many-mode test on a chip: their correlations violated a classical bound by up to 76
standard deviations. photonoxide's coupled-mode array with their coupling, $C = 5$ per mm over an
effective 782 µm, gives their Eq. 4 for the correlations as the permanent formula to $10^{-17}$,
and the bound violated at 70 pairs of guides for inputs (0, 1) and 56 for (−1, 1) by photons alike
and nowhere by photons told apart, as they report. Their simulated correlation matrices show only
the 121 pairs of guides they measured, those whose numbers differ by an even number, since their
fibre arrays have twice the pitch of the output guides. On those pairs their Figs. 3B, 3D and 4D
agree with ours bar by bar to 2.4% of the tallest bar (0.0178, 0.0237 and 0.0266 against our
0.0176, 0.0239 and 0.0264). Their Fig. 4B, the same input as 4D with photons told apart, is
**not reproduced**: its tallest bar is 0.0147 against our 0.0162, and its bars fit a 3% longer
array than the other three panels'.

::validation quantum/peruzzo-correlations quantum/peruzzo-classical-bound

The first boson-sampling experiments followed in 2013 with three photons, and in one of them four.
[Broome et al.](https://doi.org/10.1126/science.1231440) used six modes of a fused-fibre 3 × 3 beam
splitter, the two polarizations of each of its three spatial modes. [Spring et al.](https://doi.org/10.1126/science.1231692)
used a silica-on-silicon chip of six modes and ten beam splitters, written by UV laser, and sent four
photons through a second chip of the same geometry. [Tillmann et al.](https://doi.org/10.1038/nphoton.2013.102)
and [Crespi et al.](https://doi.org/10.1038/nphoton.2013.112) used networks of five modes written
by femtosecond laser in glass. Broome's and Spring's reports were published online in December
2012. Single photons from probabilistic sources made larger experiments slow; squeezed light, in
section 6, took sampling to tens and hundreds of photons.

**Try it.** Set the overlap to 0 and to 1. Which curve does the partly distinguishable one follow,
and how does the bunching change between them?

:::answer
At 0 it lies on the photons told apart, at 1 on the photons alike: the library's general formula
weighs each permutation's permanent by the product of the overlaps it pairs, and only the identity
survives at 0. Bunching rises from 0.328 to 0.566 as the overlap goes from 0 to 1.
:::

## 4. What does loss do to a quantum circuit?

A lossy circuit's $S$ is not unitary, and $\hat b = S\hat a$ alone would break the operators'
commutation. The light it loses lets the vacuum in through noise modes $\hat c$,

$$
\hat b = S\hat a + L\hat c, \qquad LL^\dagger = I - SS^\dagger ,
$$

and the photons it loses leave through loss modes, so that the transfer from the inputs to the
outputs and the loss modes together is an isometry and every pattern's probability, lost photons
included, sums to 1 ([the S-matrix as a mode transformation](../docs/methods/quantum.md)). A photon
that crosses stages keeping $\eta_1, \eta_2, \ldots$ of the light arrives with probability
$\eta = \prod\eta_k$, and an event that needs $n$ photons with $\eta^n$: a hundred photons at
$\eta = 0.9$ arrive together about once in 37 650 tries. photonoxide builds such a **budget** along a path
through a circuit, each stage $\lvert S\rvert^2$ of an instance at its own fidelity with its
provenance, and the studio draws it in the Chip page's Loss budget.

::validation quantum/dilation quantum/budget-path

What loss a computer tolerates depends on how it computes, and two thresholds are quoted with what
they assume. [Varnava et al. (2008)](https://doi.org/10.1103/PhysRevLett.100.060502) grow
loss-tolerant tree clusters from three-photon GHZ states by type-II fusion. Sources of efficiency
$\eta_S$ (at most one photon) and detectors of $\eta_D$ make GHZ states that miss a photon with
probability $\epsilon = 1 - \eta_S/(2 - \eta_D\eta_S)$, and their trees need a photon to be there and
be detected more often than not, $(1 - \epsilon)\eta_D > 1/2$, which is $\eta_S\eta_D > 2/3$.

::chart loss-varnava

[Bartolucci et al. (2023)](https://doi.org/10.1038/s41467-023-36493-1) compute by fusions on small
resource states and tolerate 10.4 % of loss per fusion, which they give as 2.7 % per photon; section
7 shows how the two numbers are one.

::validation quantum/varnava-threshold quantum/bartolucci-fusion

**Try it.** With detectors of 0.9, what source efficiency does Varnava et al.'s threshold need, and
what does a chip between them that loses 1 dB do to it?

:::answer
$\eta_S > 2/(3 \cdot 0.9) = 0.741$. Their model has no loss between source and detector: a lossy
chip counts against the product, so 1 dB ($\eta = 0.794$) needs $\eta_S\eta_D > 2/3/0.794 = 0.84$
from the rest, $\eta_S > 0.93$ with the same detectors.
:::

## 5. How good is a heralded single-photon source?

A pair source pumped weakly makes a signal and an idler photon together, from down-conversion or
four-wave mixing ([nonlinear integrated optics](nonlinear-optics.md)). Detecting one **heralds** the
other. The heralded photon is useful only if it is pure, the same every time: its spectrum must not
depend on what the herald's was. The pair's joint spectral amplitude $f(\omega_s, \omega_i)$ tells:
its Schmidt decomposition $f = \sum_k\sqrt{\lambda_k}\,u_k(\omega_s)v_k(\omega_i)$ gives the heralded
photon's purity $P = \sum_k\lambda_k^2$ and the Schmidt number $K = 1/P$, the number of modes in
effect. A joint spectrum that factorizes has $K = 1$; energy conservation pulls it along the
anti-diagonal, $\omega_s + \omega_i$ constant, and only a broad pump and matched group velocities
round it ([Grice and Walmsley, 1997](https://doi.org/10.1103/PhysRevA.56.1627)). The nonlinear
lesson's chart computes a guide's:

::chart nl-jsa{matching=2}

### Pairs per pulse and multi-pair noise {theory}

Each pair of Schmidt modes is a two-mode squeezed vacuum, $k$ pairs with probability
$(1 - t)t^k$, $t = \tanh^2 r$. photonoxide's `Source` gives mode $k$ the squeezing
$r_k = g\sqrt{\lambda_k}$, the gain $g$ set by the mean number of pairs per pulse,
$\sum_k\sinh^2 r_k = \mu$, and convolves the modes' distributions. Either arm alone is thermal in each
mode, so its $g^{(2)} = 1 + \sum\bar n_k^2/(\sum\bar n_k)^2$: 2 for one mode, $1 + P$ at low gain.
Its unheralded $g^{(2)}$ measures the purity without the spectrum. The price of brightness is
multi-pair noise: a pulse with a pair holds two or more with probability $\mu/(1 + \mu)$ for one
Schmidt mode and about $\mu/2$ for very many.

::chart pair-statistics

At a purity of 0.9 and 0.1 pairs a pulse, 8.7 % of the pulses with a pair hold more than one, and
the arm's $g^{(2)}$ is 1.903. A Gaussian joint spectrum's weights are $\lambda_k = (1 - q)q^k$ with
$q = (1 - P)/(1 + P)$; the chart uses them, and the source designer holds them to the closed form
to $10^{-9}$.

::validation quantum/source-pairs quantum/source-hom-gaussian quantum/heralding-coupler

### Rings {theory}

A ring on a bus enhances each wave's field by $F(\nu) = ik/(1 - rae^{i\phi(\nu)})$ on its resonances.
Two pump photons, each enhanced, make a signal and an idler, each enhanced on the way out, so
$f(\nu_s, \nu_i) = F_P(\nu_s + \nu_i)F_s(\nu_s)F_i(\nu_i)$, with $F_P$ the pump pulse's spectrum and
enhancement convolved with themselves ([Helt et al., 2010](https://doi.org/10.1364/OL.35.003006);
[Vernon et al., 2017](https://doi.org/10.1364/OL.42.003638), Eqs. 3–4). A long pump pins
$\nu_s + \nu_i$ and correlates the photons; a short one leaves the signal's and idler's resonances
to shape them, nearly but not quite a product, because $F_P$ is itself a resonance. Widen the pump's
resonance alone, by a second channel coupled to it, and $F_P$ flattens over the others' width.

::chart ring-pairs

### Sources against their papers {research}

Helt et al.'s ring (47.5 µm, resonances 1.48 GHz wide) has $K = 1.09$ for 5 ps pulses and 10.00 for
5 ns ones; photonoxide gives 1.0909 and 9.986, the second 0.14 % below theirs, inside their
linewidth's two printed figures. Vernon et al.'s Fig. 2(a), equal linewidths and a broad pump,
gives 1.09; ours is 1.0909.

::validation quantum/ring-helt-short quantum/ring-helt-long quantum/ring-vernon-equal

**Explained, not a case: Vernon et al.'s dual-coupled ring.** For a pump resonance 6.6 times
wider than the signal's and idler's their Fig. 2(b) gives $K = 1.0009$ (purity 0.999); their own
model, Eqs. 3 and 4, with a pump far broader than its resonance, gives 1.0033 here (purity
0.9967), the chart's value, converged on its grid. Their Fig. 3's line, the same model against
$Q_{SI}/Q_P$, reads 0.8396 at a ratio of 0.5, 0.9273 at 1 and 0.9967 at 6.5 (from the figure's
vector paths), ours 0.800, 0.917 and 0.9965. The difference is how far the integrals reach,
which the letter doesn't state: cut the signal's and idler's frequencies at three pump
linewidths either side, and the model gives the whole line within 7.3e-4, and their text's 1.08
for equal linewidths (1.0781). The remaining correlation lies in the resonances' tails, so the
cut matters most where the pump's resonance is narrow. Fig. 2(b)'s 1.0009 isn't on the line: it
is the full device simulation, which Fig. 3 marks at a ratio of 9.9, not 6.6. The window is
inferred from the figure, so this stays out of the validation cases.

**Not reproduced: Paesani et al.'s inter-modal source.** They pump an 11 mm silicon guide,
2 × 0.22 µm, in its TM₀ and TM₁ modes at once, the TM₀ pump delayed, and measure a purity of
0.9904 ± 0.0006 with the delay and 0.931 ± 0.002 without
([Paesani et al., 2020](https://doi.org/10.1038/s41467-020-16187-8)). Their delay, 1.46 ps, does
not follow from their own Supplementary Eq. 2 with their Supplementary Fig. 4's group indices
(4.007 and 4.119, a difference of 0.112), which give 2.06 ps; photonoxide's mode solver gives a
difference of 0.1070, 0.1095 and 0.1111 on 20, 10 and 5 nm cells, converging towards theirs. For
their note's Gaussian pump of 4.58 nm, which they say gives 99.95 % at 11 mm, ours at the delay of
their Eq. 2 extrapolates to 0.99934, below their printed digit. With their measured pump, read off
their Supplementary Fig. 2b, ours extrapolates to about 0.991 without its spectral phase and 0.982
with it, against their simulated 0.993 and 0.984: the phase costs the same 0.009, but both sit
about 0.002 below theirs.

**Silverstone et al.'s two sources.** Two 5.2 mm silicon spirals between two MMIs, pumped through
the first, interfere their pairs at the second
([Silverstone et al., 2013](https://doi.org/10.1038/nphoton.2013.339)). Pairs born inside a circuit
add their amplitudes, and two sources a phase $\phi$ apart make a fringe of visibility
$4\lvert x\rvert\lvert y\rvert/(\lvert x\rvert + \lvert y\rvert)^2$. With a balanced MMI of the
guided-mode model (their MMIs' design is in neither the letter nor its supplement) the degenerate
fringe is 99.99996 %, inside their 100.0 ± 0.4 %. **Not reproduced:** their non-degenerate fringes,
99.1 ± 0.5 % and 98.8 ± 1.2 %, against our 99.944 % for both: the second agrees, the first is 0.84 %
above theirs, outside its uncertainty, and nothing in the letter or supplement accounts for net
fringes below 100 %.

::validation quantum/silverstone-fringe

**Try it.** In the ring chart, at equal linewidths, why does $K$ stop falling below a few
picoseconds?

:::answer
Once the pump is much broader than the pump's resonance, $F_P$ is the resonance convolved with
itself, whatever the pulse: a Lorentzian of twice the width, which still correlates
$\nu_s + \nu_i$ over the signal's and idler's own width. $K$ settles at 1.0909. Only widening the
pump's resonance, not shortening the pulse, takes it towards 1.
:::

## 6. What is squeezed light, and what is Gaussian boson sampling?

The vacuum has noise in both quadratures, $\hat x = \hat a + \hat a^\dagger$ and
$\hat p = -i(\hat a - \hat a^\dagger)$, variance 1 each in photonoxide's units. A degenerate parametric
amplifier amplifies one quadrature and de-amplifies the other: its squeezed vacuum has variances
$e^{-2r}$ and $e^{2r}$, squeezed by $-10\log_{10}e^{-2r}$ dB. Squeezed, displaced and thermal light
are **Gaussian states**, described by their quadratures' means and covariance matrix $V$
([Weedbrook et al., 2012](https://doi.org/10.1103/RevModPhys.84.621)); a lossless circuit acts on
$V$ by its real symplectic form, and a loss that keeps $\eta$ of the light lets the vacuum in:

$$
V \to \eta V + (1 - \eta)I .
$$

The squeezed variance becomes $\eta e^{-2r} + 1 - \eta$, which can't fall below $1 - \eta$ however
strong the squeezing: loss caps squeezing at $-10\log_{10}(1 - \eta)$.

::chart squeezing-loss

10 dB of squeezing through $\eta = 0.75$ leaves 4.88 dB, and no squeezing leaves more than 6.02 dB.
A detector behind an efficiency of 0.20 sees at most 0.97 dB of any squeezing.

### What has been measured {research}

[Vaidya et al. (2020)](https://doi.org/10.1126/sciadv.aba9186) squeezed light in silicon nitride
rings. Their number-difference variance is $(1 - \eta)n_\text{tot}$ (their Eq. 3), which the
Gaussian state's photon-number covariances give for a lossy two-mode squeezed vacuum at any gain;
on the chip, at the ring's escape efficiency of 0.8, it is 6.99 dB below the shot noise against
their "~7 dB", and through 1.5 and 0.5 dB of coupling loss the most any squeezing keeps is 5.35 and
9.64 dB against their "about 5" and "nearly 10".
[Nehra et al. (2022)](https://doi.org/10.1126/science.abo6213) measured squeezing in thin-film lithium
niobate with a second, on-chip amplifier of more than 33 dB gain. From the squeezing and
anti-squeezing of their Fig. 2D, 4.2 and 9.7 dB as the methods page reads them, the loss model
gives 11.29 dB before a loss of 0.33, against their inferred 10.48 ± 0.87 dB, inside its
uncertainty; 11 dB read through the amplifier by their Eq. 1 is 11.05 dB here; and a homodyne
behind 0.20 sees at most 0.97 dB, below their 1 dB.

::validation quantum/squeezed-loss quantum/vaidya-number-difference quantum/vaidya-outcoupling quantum/nehra-inferred quantum/nehra-all-optical quantum/nehra-homodyne-bound

**Not reproduced: Vaidya et al.'s quadrature squeezing**, 1.0 dB measured and about 4 dB inferred
on the chip. Their model at the locked detuning, $V_\pm = 1 + 4g\eta(2g \pm\sqrt{1 + 4g^2})$, needs
their net collection efficiency $\eta$ and gain $g$, which neither the article nor its supplement
prints. With the escape efficiency of 0.75 alone, the most a ring could give on the chip is 6.0 dB.

### Photon numbers and hafnians {theory}

A squeezer makes photons in pairs, so a single-mode squeezed vacuum holds only even numbers:
$P(2k) = \tanh^{2k}r\,(2k)!/(2^kk!)^2/\cosh r$. For any Gaussian state the probability of a pattern
of photons is a **hafnian**, the sum over a symmetric matrix's perfect matchings of the products of
the matched entries, of a matrix built from $V$ with each mode's rows repeated as often as it holds
photons ([Hamilton et al., 2017](https://doi.org/10.1103/PhysRevLett.119.170501)). The hafnian is to
pairs what the permanent is to single photons: a permanent of $M$ is the hafnian of
$\begin{pmatrix} 0 & M \\ M^T & 0\end{pmatrix}$. Loss breaks pairs, and odd numbers appear.

::chart squeezed-photons

At 6 dB the mean is $\sinh^2 r = 0.558$ photons and $P(0) = 1/\cosh r = 0.801$; the hafnians give
the closed form to $10^{-12}$, odd numbers included.

::validation quantum/squeezed-vacuum quantum/two-mode-thermal

### Gaussian boson sampling {research}

Squeezed vacua through a large interferometer, counted at its outputs, sample hafnians: **Gaussian
boson sampling**. [Zhong et al. (2020)](https://doi.org/10.1126/science.abe8770) sent 50 squeezed
states into 100 modes and counted up to 76 clicks;
[Madsen et al. (2022)](https://doi.org/10.1038/s41586-022-04725-x) multiplexed 216 squeezed modes in
time and counted events of up to 219 photons, a mean of 125;
[Liu et al. (2026)](https://doi.org/10.1038/s41586-026-10523-6) used 1,024 squeezed states in 8,176
modes. On a chip, [Arrazola et al. (2021)](https://doi.org/10.1038/s41586-021-03202-1) put squeezers
and a programmable interferometer on one silicon nitride chip, read by eight photon-number-resolving
detectors.

Loss is what classical algorithms use. [Qi et al. (2020)](https://doi.org/10.1103/PhysRevLett.124.100502)
showed that a sampler of $K$ squeezed modes with transmission $\eta$ and threshold detectors can be
sampled classically to a total variation distance $\epsilon$ above
$\epsilon^* = 2\sqrt{-K\ln F_\text{max}}$, $F_\text{max}$ the largest fidelity between the lossy state and
a classical one its detectors can't tell apart (their Eqs. 1 and 12);
[Oh et al. (2024)](https://doi.org/10.1038/s41567-024-02535-8) built a tensor-network algorithm whose
cost falls as the loss grows.

::chart gbs-simulability

Qi et al. evaluate two experiments of 2019. [Paesani et al.'s chip](https://doi.org/10.1038/s41567-019-0567-8)
sent $K = 4$ squeezed modes into a walk of 12 guides read by detectors of about 78 % efficiency
(their PDF p. 1). Qi et al. take $r = 0.1$ and $\eta = 0.088$ for it, which the article doesn't
print; they would come from its Supplementary Information, which we haven't read. With these, Qi
et al. find it simulable "with error 2%", where photonoxide gives $\epsilon^* = 0.0224$.
[Zhong et al.'s sampler](https://doi.org/10.1016/j.scib.2019.04.007) had $K = 6$ squeezed modes of
mean $r = 0.38$ and detectors of 75 % average efficiency (pp. 512–513). It gives no transmission:
$\eta = 0.89$ is Qi et al.'s own estimate (their note 51), and they take $\eta_D = 0.82$. Qi et al.
find "no solution for any ε ∈ [0, 1]"; with the paper's $\eta_D = 0.75$, photonoxide gives
$\epsilon^* = 1.10$ (1.1022, against 1.1023 at 0.82), so the conclusion holds.

::validation quantum/qi-simulability

**Try it.** Keep $K = 6$ and $r = 0.38$. Below what transmission does the test simulate the sampler
to 10 %? And to 10 % with $K = 50$?

:::answer
Below about 0.106: there $\epsilon^* = 0.1$. $\epsilon^*$ grows as $\sqrt K$ at a given
transmission, so with 50 modes the crossing moves down to about 0.038: more modes need more loss to
be simulable by this test.
:::

## 7. How could photons compute?

Photons don't interact, so a two-qubit gate of linear optics works by measurement: extra photons
are sent in, some outputs are measured, and the gate has happened when the counts come out right.
[Knill, Laflamme and Milburn (2001)](https://doi.org/10.1038/35051009) showed this is enough. Their
nonlinear sign shift takes $\alpha_0\lvert 0\rangle + \alpha_1\lvert 1\rangle + \alpha_2\lvert 2\rangle$ to
$\alpha_0\lvert 0\rangle + \alpha_1\lvert 1\rangle - \alpha_2\lvert 2\rangle$ with one ancilla photon in a
second mode, the vacuum in a third, and the 3 × 3 unitary of their Fig. 1,

$$
u = \begin{pmatrix} 1 - \sqrt 2 & 2^{-1/4} & (3/\sqrt 2 - 2)^{1/2} \\ 2^{-1/4} & 1/2 & 1/2 - 1/\sqrt 2 \\ (3/\sqrt 2 - 2)^{1/2} & 1/2 - 1/\sqrt 2 & \sqrt 2 - 1/2 \end{pmatrix},
$$

kept when the second mode holds one photon and the third none. The amplitude for $k$ photons in the
first mode is $\langle k, 1, 0\rvert U\lvert k, 1, 0\rangle = \mathop{\mathrm{Per}}(u_{[k\cdot 1, 2]})/k!$, the
permanent of $u$'s rows and columns of mode 1 repeated $k$ times and of mode 2: $u_{22} = 1/2$ for
$k = 0$, $u_{11}u_{22} + u_{12}u_{21} = 1/2$ for $k = 1$, and $u_{11}^2u_{22} + 2u_{11}u_{12}u_{21} = -1/2$ for
$k = 2$. The sign flips, and the gate succeeds with $(1/2)^2 = 1/4$. Two of them make a conditional
sign flip that succeeds with 1/16; teleportation with prepared entangled photons raises it towards
1 at a cost in photons.

Measurement-based computing moves the difficulty into a state made beforehand:
one-qubit measurements on a **cluster state** compute
([Raussendorf and Briegel, 2001](https://doi.org/10.1103/PhysRevLett.86.5188)). Clusters of photons
grow by **fusion**: a Bell measurement on one photon of each of two clusters joins them.
[Browne and Rudolph (2005)](https://doi.org/10.1103/PhysRevLett.95.010501)'s type-II fusion succeeds
with 1/2, the most static linear optics can tell apart of the four Bell states; four ancilla
photons raise it to 3/4 ([Ewert and van Loock, 2014](https://doi.org/10.1103/PhysRevLett.113.140403)).
In [Bartolucci et al. (2023)](https://doi.org/10.1038/s41467-023-36493-1)'s model a fusion boosted to
fail with $p_\text{fail}$ measures $1/p_\text{fail}$ photons and is erased when any is lost, so a
loss $\ell$ per photon erases it with $1 - (1 - \ell)^{1/p_\text{fail}}$: their 2.7 % per photon is
10.37 % per fusion at $p_\text{fail} = 1/4$, their 10.4 %.

::chart fusion-loss

The chart's figures are computed from the circuits: the sign gate's amplitudes from permanents of
Knill et al.'s $u$, 1/2, 1/2 and −1/2 to $10^{-14}$; and the share of Bell states a balanced coupler
and polarization-resolving detectors tell apart, from the permanents of the 4-mode coupler of two
polarizations, 1/2: $\lvert\psi^-\rangle$ leaves one photon in each output, $\lvert\psi^+\rangle$ two
photons of opposite polarization in one, and $\lvert\phi^\pm\rangle$ give the same patterns as each
other.

### Continuous variables and GKP qubits {theory}

Squeezed light offers a second route. Cluster states of squeezed modes, entangled by beam splitters,
do any Gaussian operation by homodyne detection alone, and need one non-Gaussian measurement for
universality ([Menicucci et al., 2006](https://doi.org/10.1103/PhysRevLett.97.110501)).
[Gottesman, Kitaev and Preskill (2001)](https://doi.org/10.1103/PhysRevA.64.012310) encoded a qubit
in an oscillator as a comb of position states, which shifts smaller than half the comb's spacing
can't corrupt; the encoded states need a nonlinear element to make, but once made, linear optics,
squeezing and homodyne detection compute on them. Real GKP states are combs of finite width,
squeezed peaks under a broad envelope, and a fault-tolerant computer needs the peaks narrow: 14.8 dB
of squeezing in the schemes before [Fukui et al. (2018)](https://doi.org/10.1103/PhysRevX.8.021054),
below 10 dB with their analog error correction. photonoxide computes the Gaussian states these
schemes start from, the squeezing after loss above, but not GKP states, which are not Gaussian.

### Where it stands {research}

[Asavanant et al. (2019)](https://doi.org/10.1126/science.aay2645) made a 5 × 1248-site cluster state of
squeezed light multiplexed in time, every nullifier below −4.5 dB.
[Konno et al. (2024)](https://doi.org/10.1126/science.adk7560) made GKP states of travelling light with
about 2.5 dB of effective squeezing, and [Larsen et al. (2025)](https://doi.org/10.1038/s41586-025-09044-5)
made them on a silicon nitride chip, four resolvable peaks in each quadrature.
[Aghaee Rad et al. (2025)](https://doi.org/10.1038/s41586-024-08406-9) networked silicon nitride and
thin-film lithium niobate chips into a modular machine of 84 squeezers on that route. On the
single-photon route, [Alexander et al. (2025)](https://doi.org/10.1038/s41586-025-08820-7) report
two-qubit fusion at 99.22 ± 0.12 % fidelity in silicon photonics from a foundry, conditional on
detection and not accounting for loss, which sections 4 and 6 show is what decides.

**Try it.** In the fusion chart, how much loss per photon can a fusion that fails with 1/2 take
before it is erased 10.4 % of the time, against one boosted to 1/8?

:::answer
$1 - 0.896^{p_\text{fail}}$: 5.3 % for $p_\text{fail} = 1/2$ (two photons measured) and 1.4 % for 1/8
(eight). Boosting lowers the chance of failure but measures more photons, each of which can be lost.
:::

## 8. What does the studio compute, and what doesn't it?

The Chip page's **Quantum** tab computes every part of this lesson on a chip of your own, from the
S-matrices its components compute at their fidelities; open [the Mach–Zehnder chip](../circuits/mzi.toml)
to try it.

- **Photons:** photons on each port (up to 8), every output pattern for photons alike ($\lvert\mathop{\mathrm{Per}}\rvert^2$)
  and told apart, the bunching, each port's photon numbers, and $\lvert S\rvert$ between the ports;
  the chip's loss dilated to loss modes.
- **HOM dip:** two Gaussian photons, one delayed, through the chip's own $S(\omega)$ over its sweep,
  with the visibility and the level told apart: section 1's chart on any circuit.
- **Loss budget:** a photon's route through the chip as a waterfall of each instance's
  $\lvert S\rvert^2$, its source's and detector's efficiencies, the $n$-photon event and Varnava et
  al.'s and Bartolucci et al.'s thresholds with their assumptions.
- **Squeezed light:** squeezed and displaced vacua on chosen ports through the chip with its loss:
  each port's squeezing, the covariance, one port's photon numbers by hafnians and its Wigner function.
- **Sampling:** the exact sampler, batch after batch with a hard time limit, its total variation
  distance against the number of samples beside what a correct sampler reaches.
- **Run as a job:** all of these written as a `"quantum"` task beside the chip's circuit, run by the
  studio or by `photonoxide run`, its record holding the same numbers.

The Materials page's **Photon-pair sources** designs a source: a guide, a catalogue crystal, a ring
(with a second channel at the pump) or a guide pumped in two modes; its joint spectrum, Schmidt
weights, purity, brightness where the library defines one, heralding through each arm's filter and
coupler, and pairs per pulse with $g^{(2)}$ and the multi-pair share. For a guide or a crystal,
its **Squeezing at high gain** cuts the source into slices along its length and shows how far the
squeezing and the squeezed modes leave the Schmidt model's as the gain grows.

What the library does not do: it has no single-photon emitters beyond their Purcell and β factors
from FDTD, and no quantum-dot sources; detectors are their absorption, from transfer matrices and the
mode solver, not their timing or dark counts; an S-matrix from FDTD isn't a device's fidelity yet,
one launch a port; squeezing is lossless in the amplifier, and broadband squeezing at high gain,
whose modes change with the gain, is a dense map of a few hundred frequencies, lossless and with
an undepleted pump; and it computes no GKP or other non-Gaussian
states, decoders or thresholds of its own. Two comparisons on the detector side don't agree:
Pernice et al.'s (2012) NbN wires on a silicon strip absorb
1.01 ± 0.02 dB/µm by their simulation and 0.98 ± 0.10 measured, where the mode solver with the
catalogue's NbN gives between about 1.86 and 1.96 dB/µm, nearly twice; and
Arcari et al.'s (2014) β of 98.43 % waits for their
waveguide's geometry, which is in their supplement, not in hand. The
[methods page](../docs/methods/quantum.md) gives both in full.

## 9. History and current research {research}

[Hong, Ou and Mandel (1987)](https://doi.org/10.1103/PhysRevLett.59.2044) measured two-photon
interference on a beam splitter; [Reck et al. (1994)](https://doi.org/10.1103/PhysRevLett.73.58)
showed that beam splitters and phase shifters make any unitary, and
[Grice and Walmsley (1997)](https://doi.org/10.1103/PhysRevA.56.1627) how a broadband pump shapes a
pair's joint spectrum. In 2001 [Knill, Laflamme and Milburn](https://doi.org/10.1038/35051009)
showed that linear optics, single photons and detectors suffice to compute,
[Raussendorf and Briegel](https://doi.org/10.1103/PhysRevLett.86.5188) that measurements on a
cluster state do, and [Gottesman, Kitaev and Preskill](https://doi.org/10.1103/PhysRevA.64.012310)
how to encode a qubit in an oscillator. Fusion
([Browne and Rudolph, 2005](https://doi.org/10.1103/PhysRevLett.95.010501)), continuous-variable
clusters ([Menicucci et al., 2006](https://doi.org/10.1103/PhysRevLett.97.110501)) and loss
thresholds ([Varnava et al., 2008](https://doi.org/10.1103/PhysRevLett.100.060502)) followed.

On chips, two-photon interference and a CNOT gate in silica waveguides
([Politi et al., 2008](https://doi.org/10.1126/science.1155441))
were an early landmark of integrated quantum photonics; [Peruzzo et al. (2010)](https://doi.org/10.1126/science.1193515)
walked two photons through 21 guides, [Helt et al. (2010)](https://doi.org/10.1364/OL.35.003006)
gave rings' pairs their theory, and [Silverstone et al. (2013)](https://doi.org/10.1038/nphoton.2013.339)
interfered two silicon sources on a chip. [Aaronson and Arkhipov](https://doi.org/10.4086/toc.2013.v009a004)
posed boson sampling (STOC 2011, the journal 2013), and four groups sampled three photons in 2013,
one of them four as well.
[Carolan et al. (2015)](https://doi.org/10.1126/science.aab3642) made a universal six-mode chip,
[Clements et al. (2016)](https://doi.org/10.1364/OPTICA.3.001460) a better mesh, and
[Hamilton et al. (2017)](https://doi.org/10.1103/PhysRevLett.119.170501) Gaussian boson sampling.
Since 2019, [Wang et al.'s review](https://doi.org/10.1038/s41566-019-0532-1) surveys the field;
Gaussian boson samplers reached tens, then hundreds of photons
([Zhong et al., 2020](https://doi.org/10.1126/science.abe8770);
[Madsen et al., 2022](https://doi.org/10.1038/s41586-022-04725-x);
[Liu et al., 2026](https://doi.org/10.1038/s41586-026-10523-6)) while classical algorithms used
their loss ([Qi et al., 2020](https://doi.org/10.1103/PhysRevLett.124.100502);
[Oh et al., 2024](https://doi.org/10.1038/s41567-024-02535-8)); sources became pure without filters
([Vernon et al., 2017](https://doi.org/10.1364/OL.42.003638);
[Paesani et al., 2020](https://doi.org/10.1038/s41467-020-16187-8)) and squeezing reached the chip
([Vaidya et al., 2020](https://doi.org/10.1126/sciadv.aba9186);
[Arrazola et al., 2021](https://doi.org/10.1038/s41586-021-03202-1);
[Nehra et al., 2022](https://doi.org/10.1126/science.abo6213)); and the two routes to a computer,
fusion ([Bartolucci et al., 2023](https://doi.org/10.1038/s41467-023-36493-1);
[Alexander et al., 2025](https://doi.org/10.1038/s41586-025-08820-7)) and GKP states
([Fukui et al., 2018](https://doi.org/10.1103/PhysRevX.8.021054);
[Asavanant et al., 2019](https://doi.org/10.1126/science.aay2645);
[Konno et al., 2024](https://doi.org/10.1126/science.adk7560);
[Larsen et al., 2025](https://doi.org/10.1038/s41586-025-09044-5);
[Aghaee Rad et al., 2025](https://doi.org/10.1038/s41586-024-08406-9)), were built in hardware.

::timeline

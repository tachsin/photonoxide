# References

<!-- Written by `photonoxide references --write` (or `just references`) from docs/references.yaml and the citations in the repository: edit those, not this page. -->

Every paper, book and data set photonoxide cites, with every place in the repository that cites it, by file and line. 555 sources, 551 with a DOI and 4 without, are cited at 2819 places in 372 files. The sources are listed by their first author; [references-by-file.md](references-by-file.md) lists each methods page, lesson, example and source file with the sources it cites, and the [validation report](validation.md) gives each case's source on its row. How to cite a source is in [CONTRIBUTING.md](../CONTRIBUTING.md#citing-a-source); the list of sources is [references.yaml](references.yaml).

[A](#a) · [B](#b) · [C](#c) · [D](#d) · [E](#e) · [F](#f) · [G](#g) · [H](#h) · [I](#i) · [J](#j) · [K](#k) · [L](#l) · [M](#m) · [N](#n) · [O](#o) · [P](#p) · [Q](#q) · [R](#r) · [S](#s) · [T](#t) · [U](#u) · [V](#v) · [W](#w) · [X](#x) · [Y](#y) · [Z](#z)

## Sources

### A

#### Aaronson and Arkhipov 2013

S. Aaronson, A. Arkhipov, "The Computational Complexity of Linear Optics", Theory of Comput. 9, 143 (2013). [doi:10.4086/toc.2013.v009a004](https://doi.org/10.4086/toc.2013.v009a004)

- Other documents: [docs/plans/quantum.md:495](plans/quantum.md?plain=1#L495)

#### Abate and Valkó 2004

J. Abate, P. P. Valkó, "Multi‐precision Laplace transform inversion", Numerical Meth Engineering 60, 979 (2004). [doi:10.1002/nme.995](https://doi.org/10.1002/nme.995)

- Methods pages: [devices.md:42](methods/devices.md?plain=1#L42), [971](methods/devices.md?plain=1#L971)
- Validation cases: [`device/thermo-analytic-time-in-guide`](validation.md?plain=1#L653)
- Code: [src/cases/device/thermo_checks.rs:489](../src/cases/device/thermo_checks.rs#L489), [src/device/thermo.rs:785](../src/device/thermo.rs#L785), [src/device/thermo_closed.rs:522](../src/device/thermo_closed.rs#L522)

#### Abedin and Ito 1996

K. S. Abedin, H. Ito, "Temperature-dependent dispersion relation of ferroelectric lithium tantalate", Journal of Applied Physics 80, 6561 (1996). [doi:10.1063/1.363679](https://doi.org/10.1063/1.363679)

- Methods pages: [catalogue.md:828](methods/catalogue.md?plain=1#L828)
- Validation cases: [`material/litao3-abedin-bond`](validation.md?plain=1#L71), [`material/litao3-abedin-refit`](validation.md?plain=1#L72), [`material/litao3-abedin-points`](validation.md?plain=1#L73), [`material/litao3-abedin-curves`](validation.md?plain=1#L74)
- Code: [src/material/catalogue/ferroelectric.rs:12](../src/material/catalogue/ferroelectric.rs#L12), [212](../src/material/catalogue/ferroelectric.rs#L212), [src/material/catalogue/references.rs:564](../src/material/catalogue/references.rs#L564), [src/validation.rs:522](../src/validation.rs#L522), [529](../src/validation.rs#L529), [536](../src/validation.rs#L536), [543](../src/validation.rs#L543)

#### Abel et al. 2018

S. Abel, F. Eltes, J. E. Ortmann, A. Messner, P. Castera, T. Wagner, D. Urbonas, A. Rosa, A. M. Gutierrez, D. Tulli, et al., "Large Pockels effect in micro- and nanostructured barium titanate integrated on silicon", Nature Mater 18, 42 (2018). [doi:10.1038/s41563-018-0208-0](https://doi.org/10.1038/s41563-018-0208-0)

- Methods pages: [electro-optic.md:303](methods/electro-optic.md?plain=1#L303)
- Validation cases: [`material/batio3-abel-films`](validation.md?plain=1#L76), [`pockels/bto-angle`](validation.md?plain=1#L555), [`pockels/bto-abel-response`](validation.md?plain=1#L559), [`pockels/bto-abel`](validation.md?plain=1#L560)
- Code: [src/cases/mode/pockels/checks.rs:1047](../src/cases/mode/pockels/checks.rs#L1047), [src/material/catalogue/ferroelectric.rs:18](../src/material/catalogue/ferroelectric.rs#L18), [282](../src/material/catalogue/ferroelectric.rs#L282), [src/material/catalogue/references.rs:466](../src/material/catalogue/references.rs#L466), [src/mode/pockels/bto.rs:2](../src/mode/pockels/bto.rs#L2), [225](../src/mode/pockels/bto.rs#L225), [232](../src/mode/pockels/bto.rs#L232), [src/validation.rs:557](../src/validation.rs#L557)
- Other documents: [ROADMAP.md:788](../ROADMAP.md?plain=1#L788)

#### Aberth 1973

O. Aberth, "Iteration methods for finding all zeros of a polynomial simultaneously", Math. Comp. 27, 339 (1973). [doi:10.1090/S0025-5718-1973-0329236-7](https://doi.org/10.1090/S0025-5718-1973-0329236-7)

- Methods pages: [fdtd.md:12](methods/fdtd.md?plain=1#L12)
- Code: [src/fdtd/media.rs:454](../src/fdtd/media.rs#L454)

#### Abramowitz and Stegun 1964

M. Abramowitz, I. A. Stegun, "Handbook of Mathematical Functions with Formulas, Graphs, and Mathematical Tables", National Bureau of Standards, Applied Mathematics Series 55 (1964)., open access.

- Methods pages: [geometry.md:23](methods/geometry.md?plain=1#L23)
- Other documents: [CONTRIBUTING.md:78](../CONTRIBUTING.md?plain=1#L78)

#### Adachi 1985

S. Adachi, "GaAs, AlAs, and Al x Ga1− x As: Material parameters for use in research and device applications", Journal of Applied Physics 58, R1 (1985). [doi:10.1063/1.336070](https://doi.org/10.1063/1.336070)

- Code: [src/material/catalogue/references.rs:291](../src/material/catalogue/references.rs#L291)

#### Adlaj 2012

S. Adlaj, "An Eloquent Formula for the Perimeter of an Ellipse", Notices Amer. Math. Soc. 59, 1094 (2012). [doi:10.1090/noti879](https://doi.org/10.1090/noti879)

- Methods pages: [geometry.md:12](methods/geometry.md?plain=1#L12)
- Validation cases: [`geometry/areas`](validation.md?plain=1#L393)
- Code: [src/geometry/checks.rs:220](../src/geometry/checks.rs#L220)

#### Afromowitz 1974

M. A. Afromowitz, "Refractive index of Ga1−xAlxAs", Solid State Communications 15, 59 (1974). [doi:10.1016/0038-1098(74)90014-3](https://doi.org/10.1016/0038-1098(74)90014-3)

- Methods pages: [catalogue.md:68](methods/catalogue.md?plain=1#L68)
- Validation cases: [`material/algaas-afromowitz-eta`](validation.md?plain=1#L82)
- Code: [src/material/catalogue/models.rs:840](../src/material/catalogue/models.rs#L840), [src/material/catalogue/references.rs:298](../src/material/catalogue/references.rs#L298), [src/material/mod.rs:141](../src/material/mod.rs#L141), [src/validation.rs:599](../src/validation.rs#L599)

#### Aghaee Rad et al. 2025

H. Aghaee Rad, T. Ainsworth, R. N. Alexander, B. Altieri, M. F. Askarani, R. Baby, L. Banchi, B. Q. Baragiola, J. E. Bourassa, R. S. Chadwick, et al., "Scaling and networking a modular photonic quantum computer", Nature 638, 912 (2025). [doi:10.1038/s41586-024-08406-9](https://doi.org/10.1038/s41586-024-08406-9), open access.

- Other documents: [docs/plans/quantum.md:519](plans/quantum.md?plain=1#L519)

#### Agrawal 2019

G. P. Agrawal, "Nonlinear Fiber Optics", Elsevier (2019). [doi:10.1016/C2018-0-01168-8](https://doi.org/10.1016/C2018-0-01168-8)

- Methods pages: [nonlinear.md:34](methods/nonlinear.md?plain=1#L34)
- Validation cases: [`nonlinear/nlse-fundamental-soliton`](validation.md?plain=1#L782), [`nonlinear/nlse-second-order-soliton`](validation.md?plain=1#L783), [`nonlinear/nlse-third-order-period`](validation.md?plain=1#L784), [`nonlinear/nlse-split-step-order`](validation.md?plain=1#L785), [`nonlinear/nlse-spm-phase`](validation.md?plain=1#L786), [`nonlinear/nlse-spm-broadening`](validation.md?plain=1#L787), [`nonlinear/nlse-photon-number`](validation.md?plain=1#L788), [`nonlinear/fwm-gain-matched`](validation.md?plain=1#L789), [`nonlinear/nlse-tpa-order`](validation.md?plain=1#L801), [`nonlinear/xpm-walk-off-phase`](validation.md?plain=1#L805)
- Code: [src/nonlinear/absorption_checks.rs:275](../src/nonlinear/absorption_checks.rs#L275), [303](../src/nonlinear/absorption_checks.rs#L303), [src/nonlinear/fwm.rs:5](../src/nonlinear/fwm.rs#L5), [src/nonlinear/kerr_checks.rs:662](../src/nonlinear/kerr_checks.rs#L662), [669](../src/nonlinear/kerr_checks.rs#L669), [676](../src/nonlinear/kerr_checks.rs#L676), [683](../src/nonlinear/kerr_checks.rs#L683), [690](../src/nonlinear/kerr_checks.rs#L690), [697](../src/nonlinear/kerr_checks.rs#L697), [704](../src/nonlinear/kerr_checks.rs#L704), [716](../src/nonlinear/kerr_checks.rs#L716), [src/nonlinear/pulse.rs:4](../src/nonlinear/pulse.rs#L4), [src/nonlinear/xpm.rs:6](../src/nonlinear/xpm.rs#L6)
- Other documents: [ROADMAP.md:863](../ROADMAP.md?plain=1#L863)

#### Ahler et al. 2026a

L. Ahler, E. Zanchetta Ulsig, L. Wang, L. Thiel, "Data used in the paper "Low-loss InGaP-on-insulator waveguides for high-efficiency entangled pair generation and nonlinear photonics"", Zenodo (2026). [doi:10.5281/zenodo.17748661](https://doi.org/10.5281/zenodo.17748661)

- Code: [src/material/catalogue/coming.rs:105](../src/material/catalogue/coming.rs#L105)

#### Ahler et al. 2026b

L. C. Ahler, L. Wang, L. Thiel, P. H. Godoy, A. Qarar, J. Castro, X. Li, Y. Pang, M. Meunier, J. E. Bowers, et al., "Low-loss InGaP-on-insulator waveguides for high-efficiency entangled pair generation and nonlinear photonics", Optica 13, 1447 (2026). [doi:10.1364/OPTICA.589921](https://doi.org/10.1364/OPTICA.589921)

- Methods pages: [catalogue.md:82](methods/catalogue.md?plain=1#L82)
- Code: [src/material/catalogue/references.rs:340](../src/material/catalogue/references.rs#L340), [341](../src/material/catalogue/references.rs#L341)

#### Ahrens et al. 2020

W. Ahrens, J. Demmel, H. D. Nguyen, "Algorithms for Efficient Reproducible Floating Point Summation", ACM Trans. Math. Softw. 46, 1 (2020). [doi:10.1145/3389360](https://doi.org/10.1145/3389360)

- Other documents: [docs/plans/performance.md:589](plans/performance.md?plain=1#L589)

#### Aitchison et al. 1997

J. Aitchison, D. Hutchings, J. Kang, G. Stegeman, A. Villeneuve, "The nonlinear optical properties of AlGaAs at the half band gap", IEEE J. Quantum Electron. 33, 341 (1997). [doi:10.1109/3.556002](https://doi.org/10.1109/3.556002)

- Methods pages: [catalogue.md:42](methods/catalogue.md?plain=1#L42)
- Code: [src/material/catalogue/references.rs:627](../src/material/catalogue/references.rs#L627), [src/material/catalogue/transparency.rs:21](../src/material/catalogue/transparency.rs#L21)
- Other documents: [ROADMAP.md:848](../ROADMAP.md?plain=1#L848)

#### Akahane et al. 2003

Y. Akahane, T. Asano, B.-S. Song, S. Noda, "High-Q photonic nanocavity in a two-dimensional photonic crystal", Nature 425, 944 (2003). [doi:10.1038/nature02063](https://doi.org/10.1038/nature02063)

- Methods pages: [arrays.md:10](methods/arrays.md?plain=1#L10), [bodies.md:16](methods/bodies.md?plain=1#L16), [generators.md:16](methods/generators.md?plain=1#L16)
- Validation cases: [`geometry/arrays/akahane-l3`](validation.md?plain=1#L436), [`geometry/bodies/akahane-l3`](validation.md?plain=1#L447)
- Examples: [README.md:69](../examples/README.md?plain=1#L69), [l3_akahane.rs:5](../examples/l3_akahane.rs#L5)
- Code: [src/cases/geometry/body_checks.rs:514](../src/cases/geometry/body_checks.rs#L514), [src/geometry/array_checks.rs:658](../src/geometry/array_checks.rs#L658)

#### Akiyama et al. 2017

K. Akiyama, S. Nakano, I. Shoji, "Accurate measurements of electro-optic coefficients of MgO-doped and undoped congruent and stoichiometric LiNbO\<inf\>3\</inf\>", 2017 Conference on Lasers and Electro-Optics Pacific Rim (CLEO-PR) (2017). [doi:10.1109/CLEOPR.2017.8118808](https://doi.org/10.1109/CLEOPR.2017.8118808)

- Code: [src/material/catalogue/references.rs:158](../src/material/catalogue/references.rs#L158)

#### Alferness 1982

R. Alferness, "Waveguide Electrooptic Modulators", IEEE Trans. Microwave Theory Techn. 30, 1121 (1982). [doi:10.1109/TMTT.1982.1131213](https://doi.org/10.1109/TMTT.1982.1131213)

- Academy lessons: [mach-zehnder.md:28](../academy/mach-zehnder.md?plain=1#L28), [357](../academy/mach-zehnder.md?plain=1#L357)

#### Amestoy et al. 2001

P. R. Amestoy, I. S. Duff, J.-Y. L'Excellent, J. Koster, "A Fully Asynchronous Multifrontal Solver Using Distributed Dynamic Scheduling", SIAM J. Matrix Anal. Appl. 23, 15 (2001). [doi:10.1137/S0895479899358194](https://doi.org/10.1137/S0895479899358194)

- Code: [native/src/mumps.rs:3](../native/src/mumps.rs#L3), [698](../native/src/mumps.rs#L698), [707](../native/src/mumps.rs#L707), [native/tests/mumps.rs:38](../native/tests/mumps.rs#L38), [src/bench/export.rs:5](../src/bench/export.rs#L5)
- Other documents: [docs/baselines.md:6](baselines.md?plain=1#L6), [docs/plans/performance.md:590](plans/performance.md?plain=1#L590)

#### Amestoy et al. 2015

P. Amestoy, C. Ashcraft, O. Boiteau, A. Buttari, J.-Y. L'Excellent, C. Weisbecker, "Improving Multifrontal Methods by Means of Block Low-Rank Representations", SIAM J. Sci. Comput. 37, A1451 (2015). [doi:10.1137/120903476](https://doi.org/10.1137/120903476)

- Code: [native/src/mumps.rs:51](../native/src/mumps.rs#L51), [707](../native/src/mumps.rs#L707), [native/tests/mumps.rs:295](../native/tests/mumps.rs#L295)
- Other documents: [docs/baselines.md:405](baselines.md?plain=1#L405), [docs/plans/performance.md:591](plans/performance.md?plain=1#L591)

#### Amestoy et al. 2019

P. R. Amestoy, A. Buttari, J.-Y. L'Excellent, T. Mary, "Performance and Scalability of the Block Low-Rank Multifrontal Factorization on Multicore Architectures", ACM Trans. Math. Softw. 45, 1 (2019). [doi:10.1145/3242094](https://doi.org/10.1145/3242094)

- Code: [native/src/mumps.rs:6](../native/src/mumps.rs#L6), [698](../native/src/mumps.rs#L698), [708](../native/src/mumps.rs#L708)
- Other documents: [docs/baselines.md:8](baselines.md?plain=1#L8), [docs/plans/performance.md:592](plans/performance.md?plain=1#L592)

#### Amestoy et al. 2023

P. Amestoy, A. Buttari, N. J. Higham, J.-Y. L’Excellent, T. Mary, B. Vieublé, "Combining Sparse Approximate Factorizations with Mixed-precision Iterative Refinement", ACM Trans. Math. Softw. 49, 1 (2023). [doi:10.1145/3582493](https://doi.org/10.1145/3582493)

- Other documents: [docs/plans/performance.md:593](plans/performance.md?plain=1#L593)

#### Anemogiannis et al. 1999

E. Anemogiannis, E. Glytsis, T. Gaylord, "Determination of guided and leaky modes in lossless and lossy planar multilayer optical waveguides: reflection pole method and wavevector density method", J. Lightwave Technol. 17, 929 (1999). [doi:10.1109/50.762914](https://doi.org/10.1109/50.762914)

- Academy lessons: [leaky-and-bent-guides.md:83](../academy/leaky-and-bent-guides.md?plain=1#L83), [399](../academy/leaky-and-bent-guides.md?plain=1#L399), [561](../academy/leaky-and-bent-guides.md?plain=1#L561)

#### Anzt et al. 2015

H. Anzt, E. Chow, J. Dongarra, "Iterative Sparse Triangular Solves for Preconditioning", Lecture Notes in Computer Science, Springer Berlin Heidelberg (2015). [doi:10.1007/978-3-662-48096-0\_50](https://doi.org/10.1007/978-3-662-48096-0_50)

- Methods pages: [fdfd-3d.md:44](methods/fdfd-3d.md?plain=1#L44), [864](methods/fdfd-3d.md?plain=1#L864)
- Code: [src/backend/krylov.rs:1297](../src/backend/krylov.rs#L1297)
- Other documents: [docs/plans/performance.md:594](plans/performance.md?plain=1#L594)

#### Anzt et al. 2022

H. Anzt, T. Cojean, G. Flegar, F. Göbel, T. Grützmacher, P. Nayak, T. Ribizel, Y. M. Tsai, E. S. Quintana-Ortí, "Ginkgo : A Modern Linear Operator Algebra Framework for High Performance Computing", ACM Trans. Math. Softw. 48, 1 (2022). [doi:10.1145/3480935](https://doi.org/10.1145/3480935)

- Other documents: [docs/plans/performance.md:595](plans/performance.md?plain=1#L595)

#### Arbabi and Goddard 2013

A. Arbabi, L. L. Goddard, "Measurements of the refractive indices and thermo-optic coefficients of Si\_3N\_4 and SiO\_x using microring resonances", Opt. Lett. 38, 3878 (2013). [doi:10.1364/OL.38.003878](https://doi.org/10.1364/OL.38.003878)

- Methods pages: [catalogue.md:16](methods/catalogue.md?plain=1#L16)
- Validation cases: [`material/silicon-nitride-room`](validation.md?plain=1#L31), [`material/silicon-nitride-arbabi-table-2`](validation.md?plain=1#L32)
- Code: [src/material/catalogue/references.rs:130](../src/material/catalogue/references.rs#L130), [src/material/catalogue/thermo.rs:16](../src/material/catalogue/thermo.rs#L16), [481](../src/material/catalogue/thermo.rs#L481), [485](../src/material/catalogue/thermo.rs#L485), [488](../src/material/catalogue/thermo.rs#L488), [src/validation.rs:242](../src/validation.rs#L242), [249](../src/validation.rs#L249)

#### Arcari et al. 2014

M. Arcari, I. Söllner, A. Javadi, S. Lindskov Hansen, S. Mahmoodian, J. Liu, H. Thyrrestrup, E. H. Lee, J. D. Song, S. Stobbe, et al., "Near-Unity Coupling Efficiency of a Quantum Emitter to a Photonic Crystal Waveguide", Phys. Rev. Lett. 113, 093603 (2014). [doi:10.1103/PhysRevLett.113.093603](https://doi.org/10.1103/PhysRevLett.113.093603)

- Other documents: [docs/plans/quantum.md:535](plans/quantum.md?plain=1#L535)

#### Armstrong et al. 1962

J. A. Armstrong, N. Bloembergen, J. Ducuing, P. S. Pershan, "Interactions between Light Waves in a Nonlinear Dielectric", Phys. Rev. 127, 1918 (1962). [doi:10.1103/PhysRev.127.1918](https://doi.org/10.1103/PhysRev.127.1918)

- Methods pages: [nonlinear.md:24](methods/nonlinear.md?plain=1#L24)
- Validation cases: [`nonlinear/coupled-shg-tanh`](validation.md?plain=1#L722), [`nonlinear/opo-steady-photons`](validation.md?plain=1#L813)
- Code: [src/nonlinear/cavity_checks.rs:401](../src/nonlinear/cavity_checks.rs#L401), [src/nonlinear/checks.rs:746](../src/nonlinear/checks.rs#L746), [src/nonlinear/coupled.rs:31](../src/nonlinear/coupled.rs#L31)
- Other documents: [ROADMAP.md:842](../ROADMAP.md?plain=1#L842)

#### Arnold et al. 2000

D. N. Arnold, R. S. Falk, R. Winther, "Multigrid in H (div) and H (curl)", Numer. Math. 85, 197 (2000). [doi:10.1007/PL00005386](https://doi.org/10.1007/PL00005386)

- Methods pages: [multigrid-stencil.md:16](methods/multigrid-stencil.md?plain=1#L16)

#### Aroyo 2016

M. I. Aroyo, "International Tables for Crystallography: Space-group symmetry", International Union of Crystallography (2016). [doi:10.1107/97809553602060000114](https://doi.org/10.1107/97809553602060000114)

- Methods pages: [catalogue.md:122](methods/catalogue.md?plain=1#L122)
- Code: [src/material/catalogue/tags.rs:40](../src/material/catalogue/tags.rs#L40)

#### Arrazola et al. 2021

J. M. Arrazola, V. Bergholm, K. Brádler, T. R. Bromley, M. J. Collins, I. Dhand, A. Fumagalli, T. Gerrits, A. Goussev, L. G. Helt, et al., "Quantum circuits with many photons on a programmable nanophotonic chip", Nature 591, 54 (2021). [doi:10.1038/s41586-021-03202-1](https://doi.org/10.1038/s41586-021-03202-1)

- Other documents: [docs/plans/quantum.md:527](plans/quantum.md?plain=1#L527)

#### Arrieta et al. 2026

R. Arrieta, G. Romano, S. G. Johnson, "Hyperparameter-free minimum-lengthscale constraints for topology optimization", Struct Multidisc Optim 69, 210 (2026). [doi:10.1007/s00158-026-04388-6](https://doi.org/10.1007/s00158-026-04388-6)

- Other documents: [ROADMAP.md:830](../ROADMAP.md?plain=1#L830)

#### Aryana et al. 2021

K. Aryana, J. T. Gaskins, J. Nag, D. A. Stewart, Z. Bai, S. Mukhopadhyay, J. C. Read, D. H. Olson, E. R. Hoglund, J. M. Howe, et al., "Interface controlled thermal resistances of ultra-thin chalcogenide-based phase change memory devices", Nat Commun 12, 774 (2021). [doi:10.1038/s41467-020-20661-8](https://doi.org/10.1038/s41467-020-20661-8), open access.

- Methods pages: [catalogue.md:422](methods/catalogue.md?plain=1#L422)

#### Aryana et al. 2023

K. Aryana, H. J. Kim, M. R. Islam, N. Hong, C.-C. Popescu, S. Makarem, T. Gu, J. Hu, P. E. Hopkins, "Optical and thermal properties of Ge2Sb2Te5, Sb2Se3, and Sb2S3 for reconfigurable photonic devices \[Invited\]", Opt. Mater. Express 13, 3277 (2023). [doi:10.1364/OME.503178](https://doi.org/10.1364/OME.503178)

- Methods pages: [catalogue.md:406](methods/catalogue.md?plain=1#L406)
- Validation cases: [`material/pcm-sb2s3-gurevich-density`](validation.md?plain=1#L60)
- Code: [src/material/catalogue/references.rs:53](../src/material/catalogue/references.rs#L53), [54](../src/material/catalogue/references.rs#L54), [src/validation.rs:445](../src/validation.rs#L445)

#### Asavanant et al. 2019

W. Asavanant, Y. Shiozawa, S. Yokoyama, B. Charoensombutamon, H. Emura, R. N. Alexander, S. Takeda, J.-I. Yoshikawa, N. C. Menicucci, H. Yonezawa, et al., "Generation of time-domain-multiplexed two-dimensional cluster state", Science 366, 373 (2019). [doi:10.1126/science.aay2645](https://doi.org/10.1126/science.aay2645)

- Other documents: [docs/plans/quantum.md:515](plans/quantum.md?plain=1#L515)

### B

#### Bahadori et al. 2018

M. Bahadori, K. Bergman, M. Nikdast, S. Rumley, L. Y. Dai, N. Janosik, T. Van Vaerenbergh, A. Gazman, Q. Cheng, R. Polster, "Design Space Exploration of Microring Resonators in Silicon Photonic Interconnects: Impact of the Ring Curvature", J. Lightwave Technol. 36, 2767 (2018). [doi:10.1109/JLT.2018.2821359](https://doi.org/10.1109/JLT.2018.2821359)

- Methods pages: [fdtd.md:1169](methods/fdtd.md?plain=1#L1169)

#### Bai and Demmel 1993

Z. Bai, J. W. Demmel, "On swapping diagonal blocks in real Schur form", Linear Algebra and its Applications 186, 75 (1993). [doi:10.1016/0024-3795(93)90286-W](https://doi.org/10.1016/0024-3795(93)90286-W)

- Methods pages: [eigen.md:12](methods/eigen.md?plain=1#L12)
- Code: [src/eigen/schur.rs:12](../src/eigen/schur.rs#L12)

#### Bandyopadhyay et al. 2021

S. Bandyopadhyay, R. Hamerly, D. Englund, "Hardware error correction for programmable photonics", Optica 8, 1247 (2021). [doi:10.1364/OPTICA.424052](https://doi.org/10.1364/OPTICA.424052)

- Other documents: [ROADMAP.md:823](../ROADMAP.md?plain=1#L823)

#### Barboza and Cudney 2009

N. A. Barboza, R. S. Cudney, "Improved Sellmeier equation for congruently grown lithium tantalate", Appl. Phys. B 95, 453 (2009). [doi:10.1007/s00340-009-3434-y](https://doi.org/10.1007/s00340-009-3434-y)

- Methods pages: [catalogue.md:26](methods/catalogue.md?plain=1#L26)
- Validation cases: [`material/litao3-barboza-thg-period`](validation.md?plain=1#L728), [`material/litao3-barboza-tables`](validation.md?plain=1#L729)
- Code: [src/material/catalogue/litao3_ne.rs:4](../src/material/catalogue/litao3_ne.rs#L4), [94](../src/material/catalogue/litao3_ne.rs#L94), [src/material/catalogue/references.rs:571](../src/material/catalogue/references.rs#L571), [src/nonlinear/data_checks.rs:284](../src/nonlinear/data_checks.rs#L284), [291](../src/nonlinear/data_checks.rs#L291)

#### Barry et al. 2020

M. A. Barry, V. Berthier, B. D. Wilts, M.-C. Cambourieux, P. Bennet, R. Pollès, O. Teytaud, E. Centeno, N. Biais, A. Moreau, "Evolutionary algorithms converge towards evolved biological photonic structures", Sci Rep 10, 12024 (2020). [doi:10.1038/s41598-020-68719-3](https://doi.org/10.1038/s41598-020-68719-3), open access.

- Examples: [optimize_bragg.rs:14](../examples/optimize_bragg.rs#L14)
- Other documents: [docs/plans/optimization.md:614](plans/optimization.md?plain=1#L614)

#### Bartolucci et al. 2023

S. Bartolucci, P. Birchall, H. Bombín, H. Cable, C. Dawson, M. Gimeno-Segovia, E. Johnston, K. Kieling, N. Nickerson, M. Pant, et al., "Fusion-based quantum computation", Nat Commun 14, 912 (2023). [doi:10.1038/s41467-023-36493-1](https://doi.org/10.1038/s41467-023-36493-1), open access.

- Other documents: [docs/plans/quantum.md:491](plans/quantum.md?plain=1#L491)

#### Barwicz and Haus 2005

T. Barwicz, H. Haus, "Three-dimensional analysis of scattering losses due to sidewall roughness in microphotonic waveguides", J. Lightwave Technol. 23, 2719 (2005). [doi:10.1109/JLT.2005.850816](https://doi.org/10.1109/JLT.2005.850816)

- Methods pages: [devices.md:64](methods/devices.md?plain=1#L64)
- Validation cases: [`roughness/payne-lacey-eq9`](validation.md?plain=1#L704)
- Code: [src/mode/roughness.rs:9](../src/mode/roughness.rs#L9), [src/mode/roughness_checks.rs:148](../src/mode/roughness_checks.rs#L148)
- Other documents: [ROADMAP.md:738](../ROADMAP.md?plain=1#L738), [docs/plans/fabrication.md:639](plans/fabrication.md?plain=1#L639)

#### Bauer et al. 2011

C. A. Bauer, G. R. Werner, J. R. Cary, "A second-order 3D electromagnetics algorithm for curved interfaces between anisotropic dielectrics on a Yee mesh", Journal of Computational Physics 230, 2060 (2011). [doi:10.1016/j.jcp.2010.12.005](https://doi.org/10.1016/j.jcp.2010.12.005)

- Methods pages: [fdtd.md:42](methods/fdtd.md?plain=1#L42)
- Validation cases: [`fdtd/smoothing-triplets-oblique`](validation.md?plain=1#L234), [`fdtd/smoothing-bauer-order`](validation.md?plain=1#L238), [`fdtd/smoothing-crystal`](validation.md?plain=1#L242)
- Code: [src/fdtd/smoothing.rs:27](../src/fdtd/smoothing.rs#L27), [263](../src/fdtd/smoothing.rs#L263), [src/validation.rs:1663](../src/validation.rs#L1663), [1691](../src/validation.rs#L1691), [1719](../src/validation.rs#L1719)

#### Bauters et al. 2011

J. F. Bauters, M. J. R. Heck, D. John, D. Dai, M.-C. Tien, J. S. Barton, A. Leinse, R. G. Heideman, D. J. Blumenthal, J. E. Bowers, "Ultra-low-loss high-aspect-ratio Si\_3N\_4 waveguides", Opt. Express 19, 3163 (2011). [doi:10.1364/OE.19.003163](https://doi.org/10.1364/OE.19.003163)

- Methods pages: [fabrication.md:36](methods/fabrication.md?plain=1#L36)
- Code: [src/fabrication/references.rs:240](../src/fabrication/references.rs#L240), [242](../src/fabrication/references.rs#L242)
- Other documents: [docs/plans/fabrication.md:656](plans/fabrication.md?plain=1#L656)

#### Bell and Garland 2009

N. Bell, M. Garland, "Implementing sparse matrix-vector multiplication on throughput-oriented processors", Proceedings of the Conference on High Performance Computing Networking, Storage and Analysis (2009). [doi:10.1145/1654059.1654078](https://doi.org/10.1145/1654059.1654078)

- Other documents: [docs/plans/performance.md:596](plans/performance.md?plain=1#L596)

#### Bennet et al. 2024

P. Bennet, D. Langevin, C. Essoual, A. Khaireh-Walieh, O. Teytaud, P. Wiecha, A. Moreau, "Illustrated tutorial on global optimization in nanophotonics", J. Opt. Soc. Am. B 41, A126 (2024). [doi:10.1364/JOSAB.506389](https://doi.org/10.1364/JOSAB.506389)

- Methods pages: [optimization.md:286](methods/optimization.md?plain=1#L286)
- Validation cases: [`mode/multilayer-quarter-wave-stationary`](validation.md?plain=1#L116)
- Examples: [README.md:50](../examples/README.md?plain=1#L50), [optimize_bragg.rs:7](../examples/optimize_bragg.rs#L7)
- Code: [src/validation.rs:837](../src/validation.rs#L837), [tests/method_study.rs:5](../tests/method_study.rs#L5)
- Other documents: [docs/plans/optimization.md:615](plans/optimization.md?plain=1#L615)

#### Berenger 1994

J.-P. Berenger, "A perfectly matched layer for the absorption of electromagnetic waves", Journal of Computational Physics 114, 185 (1994). [doi:10.1006/jcph.1994.1159](https://doi.org/10.1006/jcph.1994.1159)

- Methods pages: [fdtd.md:18](methods/fdtd.md?plain=1#L18)
- Code: [src/fdtd.rs:23](../src/fdtd.rs#L23)
- Other documents: [ROADMAP.md:716](../ROADMAP.md?plain=1#L716)

#### Berseth et al. 1992

C.-A. Berseth, C. Wuethrich, F. K. Reinhart, "The electro-optic coefficients of GaAs: Measurements at 1.32 and 1.52 μm and study of their dispersion between 0.9 and 10 μm", Journal of Applied Physics 71, 2821 (1992). [doi:10.1063/1.351011](https://doi.org/10.1063/1.351011)

- Methods pages: [catalogue.md:74](methods/catalogue.md?plain=1#L74)
- Code: [src/material/catalogue/references.rs:277](../src/material/catalogue/references.rs#L277)

#### Bertaccini and Durastante 2018

D. Bertaccini, F. Durastante, "Iterative Methods and Preconditioning for Large and Sparse Linear Systems with Applications", Chapman and Hall/CRC (2018). [doi:10.1201/9781315153575](https://doi.org/10.1201/9781315153575)

- Methods pages: [recycling.md:12](methods/recycling.md?plain=1#L12), [255](methods/recycling.md?plain=1#L255)
- Other documents: [docs/plans/performance.md:580](plans/performance.md?plain=1#L580)

#### Beume et al. 2007

N. Beume, B. Naujoks, M. Emmerich, "SMS-EMOA: Multiobjective selection based on dominated hypervolume", European Journal of Operational Research 181, 1653 (2007). [doi:10.1016/j.ejor.2006.08.008](https://doi.org/10.1016/j.ejor.2006.08.008)

- Methods pages: [optimization.md:140](methods/optimization.md?plain=1#L140)
- Code: [src/job/optimize.rs:266](../src/job/optimize.rs#L266)

#### Beyn 2012

W.-J. Beyn, "An integral method for solving nonlinear eigenvalue problems", Linear Algebra and its Applications 436, 3839 (2012). [doi:10.1016/j.laa.2011.03.030](https://doi.org/10.1016/j.laa.2011.03.030)

- Methods pages: [contour.md:192](methods/contour.md?plain=1#L192)
- Other documents: [docs/plans/performance.md:597](plans/performance.md?plain=1#L597)

#### Bienstman and Baets 2001

P. Bienstman, R. Baets, "Optical modelling of photonic crystals and VCSELs using eigenmode expansion and perfectly matched layers", Optical and Quantum Electronics 33, 327 (2001). [doi:10.1023/A:1010882531238](https://doi.org/10.1023/A:1010882531238)

- Methods pages: [eme.md:8](methods/eme.md?plain=1#L8)
- Code: [src/eme.rs:6](../src/eme.rs#L6)
- Other documents: [ROADMAP.md:734](../ROADMAP.md?plain=1#L734)

#### Bienstman et al. 2007

P. Bienstman, S. Selleri, L. Rosa, H. P. Uranus, W. C. L. Hopman, R. Costa, A. Melloni, L. C. Andreani, J. P. Hugonin, P. Lalanne, et al., "Modelling leaky photonic wires: A mode solver comparison", Opt Quant Electron 38, 731 (2007). [doi:10.1007/s11082-006-9025-9](https://doi.org/10.1007/s11082-006-9025-9)

- Methods pages: [vector.md:14](methods/vector.md?plain=1#L14)
- Academy lessons: [leaky-and-bent-guides.md:101](../academy/leaky-and-bent-guides.md?plain=1#L101), [566](../academy/leaky-and-bent-guides.md?plain=1#L566), [strip-waveguide.md:112](../academy/strip-waveguide.md?plain=1#L112), [562](../academy/strip-waveguide.md?plain=1#L562), [677](../academy/strip-waveguide.md?plain=1#L677)
- Validation cases: [`mode/leaky-wire-bienstman`](validation.md?plain=1#L125), [`mode/leaky-wire-bienstman-loss`](validation.md?plain=1#L126), [`job/leaky-wire-bienstman`](validation.md?plain=1#L481), [`job/leaky-wire-bienstman-loss`](validation.md?plain=1#L482)
- Examples: [README.md:41](../examples/README.md?plain=1#L41), [leaky_wire_benchmark.rs:5](../examples/leaky_wire_benchmark.rs#L5)
- Code: [src/job/leaky_checks.rs:57](../src/job/leaky_checks.rs#L57), [64](../src/job/leaky_checks.rs#L64), [src/validation.rs:900](../src/validation.rs#L900), [907](../src/validation.rs#L907)

#### Björklund et al. 2019

A. Björklund, B. Gupt, N. Quesada, "A Faster Hafnian Formula for Complex Matrices and Its Benchmarking on a Supercomputer", ACM J. Exp. Algorithmics 24, 1 (2019). [doi:10.1145/3325111](https://doi.org/10.1145/3325111)

- Other documents: [docs/plans/quantum.md:502](plans/quantum.md?plain=1#L502)

#### Bloch 1929

F. Bloch, "�ber die Quantenmechanik der Elektronen in Kristallgittern", Z. Physik 52, 555 (1929). [doi:10.1007/BF01339455](https://doi.org/10.1007/BF01339455)

- Academy lessons: [bragg-gratings.md:31](../academy/bragg-gratings.md?plain=1#L31), [209](../academy/bragg-gratings.md?plain=1#L209)

#### Blumenthal et al. 2018

D. J. Blumenthal, R. Heideman, D. Geuzebroek, A. Leinse, C. Roeloffzen, "Silicon Nitride in Silicon Photonics", Proc. IEEE 106, 2209 (2018). [doi:10.1109/JPROC.2018.2861576](https://doi.org/10.1109/JPROC.2018.2861576)

- Academy lessons: [slab-waveguide.md:90](../academy/slab-waveguide.md?plain=1#L90), [586](../academy/slab-waveguide.md?plain=1#L586)

#### Bogaerts and Chrostowski 2018

W. Bogaerts, L. Chrostowski, "Silicon Photonics Circuit Design: Methods, Tools and Challenges", Laser & Photonics Reviews 12, 1700237 (2018). [doi:10.1002/lpor.201700237](https://doi.org/10.1002/lpor.201700237)

- Other documents: [ROADMAP.md:750](../ROADMAP.md?plain=1#L750)

#### Bogaerts and Selvaraja 2011

W. Bogaerts, S. K. Selvaraja, "Compact Single-Mode Silicon Hybrid Rib/Strip Waveguide With Adiabatic Bends", IEEE Photonics J. 3, 422 (2011). [doi:10.1109/JPHOT.2011.2142931](https://doi.org/10.1109/JPHOT.2011.2142931)

- Academy lessons: [leaky-and-bent-guides.md:119](../academy/leaky-and-bent-guides.md?plain=1#L119), [576](../academy/leaky-and-bent-guides.md?plain=1#L576)

#### Bogaerts et al. 2005

W. Bogaerts, R. Baets, P. Dumon, V. Wiaux, S. Beckx, D. Taillaert, B. Luyssaert, J. Van Campenhout, P. Bienstman, D. Van Thourhout, "Nanophotonic waveguides in silicon-on-insulator fabricated with CMOS technology", J. Lightwave Technol. 23, 401 (2005). [doi:10.1109/JLT.2004.834471](https://doi.org/10.1109/JLT.2004.834471)

- Academy lessons: [strip-waveguide.md:100](../academy/strip-waveguide.md?plain=1#L100), [686](../academy/strip-waveguide.md?plain=1#L686)
- Other documents: [docs/plans/fabrication.md:635](plans/fabrication.md?plain=1#L635)

#### Bogaerts et al. 2011

W. Bogaerts, P. De Heyn, T. Van Vaerenbergh, K. De Vos, S. Kumar Selvaraja, T. Claes, P. Dumon, P. Bienstman, D. Van Thourhout, R. Baets, "Silicon microring resonators", Laser & Photonics Reviews 6, 47 (2011). [doi:10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017)

- Methods pages: [circuit-adjoint.md:14](methods/circuit-adjoint.md?plain=1#L14), [circuits.md:10](methods/circuits.md?plain=1#L10), [compact.md:16](methods/compact.md?plain=1#L16), [components.md:8](methods/components.md?plain=1#L8), [devices.md:10](methods/devices.md?plain=1#L10), [fdtd.md:1168](methods/fdtd.md?plain=1#L1168), [optimization.md:41](methods/optimization.md?plain=1#L41)
- Academy lessons: [mach-zehnder.md:88](../academy/mach-zehnder.md?plain=1#L88), [144](../academy/mach-zehnder.md?plain=1#L144), [ring-resonator.md:85](../academy/ring-resonator.md?plain=1#L85), [125](../academy/ring-resonator.md?plain=1#L125), [152](../academy/ring-resonator.md?plain=1#L152), [480](../academy/ring-resonator.md?plain=1#L480), [587](../academy/ring-resonator.md?plain=1#L587)
- Validation cases: [`circuit/ring-all-pass-bogaerts`](validation.md?plain=1#L277), [`circuit/ring-add-drop-bogaerts`](validation.md?plain=1#L278), [`circuit/ring-fsr-bogaerts`](validation.md?plain=1#L279), [`components/ring-all-pass-bogaerts`](validation.md?plain=1#L289), [`components/ring-add-drop-bogaerts`](validation.md?plain=1#L290), [`components/ring-extremes-bogaerts`](validation.md?plain=1#L291), [`components/ring-linewidth-bogaerts`](validation.md?plain=1#L292), [`components/ring-fsr-fdfd`](validation.md?plain=1#L293), [`device/ring-closed-forms`](validation.md?plain=1#L310), [`device/ring-fsr`](validation.md?plain=1#L321), [`device/ring-q-bogaerts`](validation.md?plain=1#L326), [`device/compact-ring-closed-form`](validation.md?plain=1#L351), [`compact/vf-ring-allpass`](validation.md?plain=1#L381)
- Examples: [README.md:48](../examples/README.md?plain=1#L48), [54](../examples/README.md?plain=1#L54), [61](../examples/README.md?plain=1#L61), [circuit_fit.rs:7](../examples/circuit_fit.rs#L7), [circuit_ring_critical.rs:5](../examples/circuit_ring_critical.rs#L5), [optimize_ring_pareto.rs:6](../examples/optimize_ring_pareto.rs#L6), [ring_liu_poon.rs:40](../examples/ring_liu_poon.rs#L40), [ring_q_factor.rs:5](../examples/ring_q_factor.rs#L5), [python/examples/README.md:29](../python/examples/README.md?plain=1#L29), [python/examples/_ring.py:3](../python/examples/_ring.py#L3), [python/examples/ring_fsr.py:5](../python/examples/ring_fsr.py#L5), [python/examples/touchstone_ring.py:5](../python/examples/touchstone_ring.py#L5)
- Code: [python/tests/test_api.py:47](../python/tests/test_api.py#L47), [src/cases/compact/checks.rs:128](../src/cases/compact/checks.rs#L128), [740](../src/cases/compact/checks.rs#L740), [src/circuit/components/coupler.rs:37](../src/circuit/components/coupler.rs#L37), [111](../src/circuit/components/coupler.rs#L111), [src/circuit/components/ring.rs:3](../src/circuit/components/ring.rs#L3), [388](../src/circuit/components/ring.rs#L388), [611](../src/circuit/components/ring.rs#L611), [src/device/checks.rs:68](../src/device/checks.rs#L68), [145](../src/device/checks.rs#L145), [180](../src/device/checks.rs#L180), [src/device/ring.rs:286](../src/device/ring.rs#L286), [src/job/fit_checks.rs:429](../src/job/fit_checks.rs#L429), [src/validation.rs:1964](../src/validation.rs#L1964), [1971](../src/validation.rs#L1971), [1978](../src/validation.rs#L1978), [2048](../src/validation.rs#L2048), [2055](../src/validation.rs#L2055), [2062](../src/validation.rs#L2062), [2069](../src/validation.rs#L2069), [2076](../src/validation.rs#L2076), [tests/jobs-toml/chip-fit.toml:2](../tests/jobs-toml/chip-fit.toml#L2), [tests/jobs-toml/chip-ring-critical.toml:1](../tests/jobs-toml/chip-ring-critical.toml#L1)
- Jobs and circuits: [jobs/chip-fit.yaml:2](../jobs/chip-fit.yaml#L2), [jobs/chip-ring-critical.yaml:1](../jobs/chip-ring-critical.yaml#L1)
- Other documents: [ROADMAP.md:749](../ROADMAP.md?plain=1#L749), [docs/plans/optimization.md:625](plans/optimization.md?plain=1#L625)

#### Bogaerts et al. 2020

W. Bogaerts, D. Pérez, J. Capmany, D. A. B. Miller, J. Poon, D. Englund, F. Morichetti, A. Melloni, "Programmable photonic circuits", Nature 586, 207 (2020). [doi:10.1038/s41586-020-2764-0](https://doi.org/10.1038/s41586-020-2764-0)

- Academy lessons: [mach-zehnder.md:118](../academy/mach-zehnder.md?plain=1#L118), [303](../academy/mach-zehnder.md?plain=1#L303), [379](../academy/mach-zehnder.md?plain=1#L379), [463](../academy/mach-zehnder.md?plain=1#L463)
- Other documents: [docs/plans/optimization.md:631](plans/optimization.md?plain=1#L631)

#### Bojko et al. 2011

R. J. Bojko, J. Li, L. He, T. Baehr-Jones, M. Hochberg, Y. Aida, "Electron beam lithography writing strategies for low loss, high confinement silicon optical waveguides", Journal of Vacuum Science & Technology B, Nanotechnology and Microelectronics: Materials, Processing, Measurement, and Phenomena 29, 06F309 (2011). [doi:10.1116/1.3653266](https://doi.org/10.1116/1.3653266)

- Methods pages: [fabrication.md:14](methods/fabrication.md?plain=1#L14)
- Code: [src/fabrication/references.rs:56](../src/fabrication/references.rs#L56)
- Other documents: [docs/plans/fabrication.md:637](plans/fabrication.md?plain=1#L637)

#### Bonazzoli et al. 2019

M. Bonazzoli, V. Dolean, I. G. Graham, E. A. Spence, P.-H. Tournier, "Domain decomposition preconditioning for the high-frequency time-harmonic Maxwell equations with absorption", Math. Comp. 88, 2559 (2019). [doi:10.1090/mcom/3447](https://doi.org/10.1090/mcom/3447)

- Other documents: [docs/plans/performance.md:598](plans/performance.md?plain=1#L598)

#### Bond 1965

W. L. Bond, "Measurement of the Refractive Indices of Several Crystals", Journal of Applied Physics 36, 1674 (1965). [doi:10.1063/1.1703106](https://doi.org/10.1063/1.1703106)

- Validation cases: [`material/litao3-bond-fit`](validation.md?plain=1#L69), [`material/litao3-abedin-bond`](validation.md?plain=1#L71)
- Code: [src/material/catalogue/ferroelectric.rs:5](../src/material/catalogue/ferroelectric.rs#L5), [113](../src/material/catalogue/ferroelectric.rs#L113), [src/material/catalogue/references.rs:508](../src/material/catalogue/references.rs#L508), [src/validation.rs:508](../src/validation.rs#L508), [522](../src/validation.rs#L522)

#### Bootland et al. 2021

N. Bootland, V. Dolean, P. Jolivet, P.-H. Tournier, "A comparison of coarse spaces for Helmholtz problems in the high frequency regime", Computers & Mathematics with Applications 98, 239 (2021). [doi:10.1016/j.camwa.2021.07.011](https://doi.org/10.1016/j.camwa.2021.07.011)

- Other documents: [docs/plans/performance.md:599](plans/performance.md?plain=1#L599)

#### Born et al. 1999

M. Born, E. Wolf, A. B. Bhatia, P. C. Clemmow, D. Gabor, A. R. Stokes, A. M. Taylor, P. A. Wayman, W. L. Wilcock, "Principles of Optics: Electromagnetic Theory of Propagation, Interference and Diffraction of Light", Cambridge University Press (1999). [doi:10.1017/CBO9781139644181](https://doi.org/10.1017/CBO9781139644181)

- Academy lessons: [mach-zehnder.md:64](../academy/mach-zehnder.md?plain=1#L64), [352](../academy/mach-zehnder.md?plain=1#L352), [453](../academy/mach-zehnder.md?plain=1#L453)

#### Boyd 2020

R. W. Boyd, "Nonlinear Optics", Elsevier (2020). [doi:10.1016/C2015-0-05510-1](https://doi.org/10.1016/C2015-0-05510-1)

- Methods pages: [nonlinear.md:10](methods/nonlinear.md?plain=1#L10)
- Code: [src/material/catalogue/tags.rs:43](../src/material/catalogue/tags.rs#L43), [src/nonlinear/phase.rs:51](../src/nonlinear/phase.rs#L51)
- Other documents: [ROADMAP.md:862](../ROADMAP.md?plain=1#L862)

#### Boyd and Kleinman 1968

G. D. Boyd, D. A. Kleinman, "Parametric Interaction of Focused Gaussian Light Beams", Journal of Applied Physics 39, 3597 (1968). [doi:10.1063/1.1656831](https://doi.org/10.1063/1.1656831)

- Other documents: [ROADMAP.md:844](../ROADMAP.md?plain=1#L844)

#### Bragg and Bragg 1913

W. H. Bragg, W. L. Bragg, "The reflection of X-rays by crystals", Proceedings of the Royal Society of London. Series A, Containing Papers of a Mathematical and Physical Character 88, 428 (1913). [doi:10.1098/rspa.1913.0040](https://doi.org/10.1098/rspa.1913.0040)

- Academy lessons: [bragg-gratings.md:25](../academy/bragg-gratings.md?plain=1#L25), [161](../academy/bragg-gratings.md?plain=1#L161)

#### Bristow et al. 2007

A. D. Bristow, N. Rotenberg, H. M. van Driel, "Two-photon absorption and Kerr coefficients of silicon for 850–2200nm", Applied Physics Letters 90, 191104 (2007). [doi:10.1063/1.2737359](https://doi.org/10.1063/1.2737359)

- Methods pages: [catalogue.md:30](methods/catalogue.md?plain=1#L30)
- Validation cases: [`material/kerr-bristow-readings`](validation.md?plain=1#L733)
- Code: [src/material/catalogue/kerr.rs:7](../src/material/catalogue/kerr.rs#L7), [src/material/catalogue/references.rs:585](../src/material/catalogue/references.rs#L585), [src/material/catalogue/transparency.rs:12](../src/material/catalogue/transparency.rs#L12), [src/nonlinear/data_checks.rs:319](../src/nonlinear/data_checks.rs#L319)

#### Browne and Rudolph 2005

D. E. Browne, T. Rudolph, "Resource-Efficient Linear Optical Quantum Computation", Phys. Rev. Lett. 95, 010501 (2005). [doi:10.1103/PhysRevLett.95.010501](https://doi.org/10.1103/PhysRevLett.95.010501)

- Other documents: [docs/plans/quantum.md:488](plans/quantum.md?plain=1#L488)

#### Bruel 1995

M. Bruel, "Silicon on insulator material technology", Electron. Lett. 31, 1201 (1995). [doi:10.1049/el:19950805](https://doi.org/10.1049/el:19950805)

- Methods pages: [fabrication.md:8](methods/fabrication.md?plain=1#L8)
- Validation cases: [`fabrication/smart-cut-range-bruel`](validation.md?plain=1#L765)
- Code: [src/fabrication/references.rs:40](../src/fabrication/references.rs#L40), [src/fabrication/smart_cut.rs:3](../src/fabrication/smart_cut.rs#L3), [src/fabrication/smart_cut_checks.rs:124](../src/fabrication/smart_cut_checks.rs#L124)
- Other documents: [docs/plans/fabrication.md:634](plans/fabrication.md?plain=1#L634)

#### Bruner et al. 2002

A. Bruner, D. Eger, M. B. Oron, P. Blau, M. Katz, S. Ruschin, "\<title\>Refractive index dispersion measurements of congruent and stoichiometric LiTaO\<formula\>\<inf\>\<roman\>3\</roman\>\</inf\>\</formula\>\</title\>", SPIE Proceedings (2002). [doi:10.1117/12.460817](https://doi.org/10.1117/12.460817)

- Methods pages: [catalogue.md:28](methods/catalogue.md?plain=1#L28)
- Validation cases: [`material/litao3-bruner-slt-633`](validation.md?plain=1#L730)
- Code: [src/material/catalogue/litao3_ne.rs:9](../src/material/catalogue/litao3_ne.rs#L9), [265](../src/material/catalogue/litao3_ne.rs#L265), [src/material/catalogue/references.rs:578](../src/material/catalogue/references.rs#L578), [src/nonlinear/data_checks.rs:298](../src/nonlinear/data_checks.rs#L298)

#### Bruner et al. 2003

A. Bruner, D. Eger, M. B. Oron, P. Blau, M. Katz, S. Ruschin, "Temperature-dependent Sellmeier equation for the refractive index of stoichiometric lithium tantalate", Opt. Lett. 28, 194 (2003). [doi:10.1364/OL.28.000194](https://doi.org/10.1364/OL.28.000194)

- Methods pages: [catalogue.md:24](methods/catalogue.md?plain=1#L24)
- Code: [src/material/catalogue/coming.rs:141](../src/material/catalogue/coming.rs#L141)

#### Brunner et al. 1997

D. Brunner, H. Angerer, E. Bustarret, F. Freudenberg, R. Höpler, R. Dimitrov, O. Ambacher, M. Stutzmann, "Optical constants of epitaxial AlGaN films and their temperature dependence", Journal of Applied Physics 82, 5090 (1997). [doi:10.1063/1.366309](https://doi.org/10.1063/1.366309)

- Code: [src/material/catalogue/coming.rs:121](../src/material/catalogue/coming.rs#L121)

#### Bunch and Kaufman 1977

J. R. Bunch, L. Kaufman, "Some stable methods for calculating inertia and solving symmetric linear systems", Math. Comp. 31, 163 (1977). [doi:10.1090/S0025-5718-1977-0428694-0](https://doi.org/10.1090/S0025-5718-1977-0428694-0)

- Other documents: [docs/baselines.md:385](baselines.md?plain=1#L385)

#### Byrd et al. 1995

R. H. Byrd, P. Lu, J. Nocedal, C. Zhu, "A Limited Memory Algorithm for Bound Constrained Optimization", SIAM J. Sci. Comput. 16, 1190 (1995). [doi:10.1137/0916069](https://doi.org/10.1137/0916069)

- Methods pages: [optimization.md:131](methods/optimization.md?plain=1#L131)
- Code: [src/job/optimize.rs:150](../src/job/optimize.rs#L150)

### C

#### Cai and Sarkis 1999

X.-C. Cai, M. Sarkis, "A Restricted Additive Schwarz Preconditioner for General Sparse Linear Systems", SIAM J. Sci. Comput. 21, 792 (1999). [doi:10.1137/S106482759732678X](https://doi.org/10.1137/S106482759732678X)

- Other documents: [docs/plans/performance.md:600](plans/performance.md?plain=1#L600)

#### Cai et al. 2000

M. Cai, O. Painter, K. J. Vahala, "Observation of Critical Coupling in a Fiber Taper to a Silica-Microsphere Whispering-Gallery Mode System", Phys. Rev. Lett. 85, 74 (2000). [doi:10.1103/PhysRevLett.85.74](https://doi.org/10.1103/PhysRevLett.85.74)

- Academy lessons: [ring-resonator.md:55](../academy/ring-resonator.md?plain=1#L55), [283](../academy/ring-resonator.md?plain=1#L283)

#### Calma Company 1987

Calma Company, "GDSII Stream Format Manual, release 6.0" (1987)., open access.

- Methods pages: [geometry.md:30](methods/geometry.md?plain=1#L30)

#### Campos and Roman 2021

C. Campos, J. E. Roman, "NEP: A Module for the Parallel Solution of Nonlinear Eigenvalue Problems in SLEPc", ACM Trans. Math. Softw. 47, 1 (2021). [doi:10.1145/3447544](https://doi.org/10.1145/3447544)

- Other documents: [docs/plans/performance.md:601](plans/performance.md?plain=1#L601)

#### Carolan et al. 2015

J. Carolan, C. Harrold, C. Sparrow, E. Martín-López, N. J. Russell, J. W. Silverstone, P. J. Shadbolt, N. Matsuda, M. Oguma, M. Itoh, et al., "Universal linear optics", Science 349, 711 (2015). [doi:10.1126/science.aab3642](https://doi.org/10.1126/science.aab3642)

- Other documents: [docs/plans/quantum.md:526](plans/quantum.md?plain=1#L526)

#### Carson and Higham 2018

E. Carson, N. J. Higham, "Accelerating the Solution of Linear Systems by Iterative Refinement in Three Precisions", SIAM J. Sci. Comput. 40, A817 (2018). [doi:10.1137/17M1140819](https://doi.org/10.1137/17M1140819)

- Other documents: [docs/plans/performance.md:602](plans/performance.md?plain=1#L602)

#### Casson et al. 2004

J. L. Casson, K. T. Gahagan, D. A. Scrymgeour, R. K. Jain, J. M. Robinson, V. Gopalan, R. K. Sander, "Electro-optic coefficients of lithium tantalate at near-infrared wavelengths", J. Opt. Soc. Am. B 21, 1948 (2004). [doi:10.1364/JOSAB.21.001948](https://doi.org/10.1364/JOSAB.21.001948)

- Methods pages: [electro-optic.md:322](methods/electro-optic.md?plain=1#L322)
- Validation cases: [`material/litao3-casson`](validation.md?plain=1#L70), [`pockels/litao3-x-cut`](validation.md?plain=1#L558)
- Code: [src/cases/mode/pockels/checks.rs:1068](../src/cases/mode/pockels/checks.rs#L1068), [src/material/catalogue/ferroelectric.rs:10](../src/material/catalogue/ferroelectric.rs#L10), [src/material/catalogue/references.rs:494](../src/material/catalogue/references.rs#L494), [src/validation.rs:515](../src/validation.rs#L515)

#### Caughey and Thomas 1967

D. Caughey, R. Thomas, "Carrier mobilities in silicon empirically related to doping and field", Proc. IEEE 55, 2192 (1967). [doi:10.1109/PROC.1967.6123](https://doi.org/10.1109/PROC.1967.6123)

- Other documents: [ROADMAP.md:810](../ROADMAP.md?plain=1#L810)

#### Černošková et al. 2014

E. Černošková, R. Todorov, Z. Černošek, J. Holubová, L. Beneš, "Thermal properties and the structure of amorphous Sb2Se3 thin film", J Therm Anal Calorim 118, 105 (2014). [doi:10.1007/s10973-014-4000-3](https://doi.org/10.1007/s10973-014-4000-3)

- Code: [src/material/catalogue/references.rs:39](../src/material/catalogue/references.rs#L39)

#### Chang et al. 2018

L. Chang, A. Boes, X. Guo, D. T. Spencer, M. J. Kennedy, J. D. Peters, N. Volet, J. Chiles, A. Kowligy, N. Nader, et al., "Heterogeneously Integrated GaAs Waveguides on Insulator for Efficient Frequency Conversion", Laser &amp; Photonics Reviews 12, 1800149 (2018). [doi:10.1002/lpor.201800149](https://doi.org/10.1002/lpor.201800149)

- Methods pages: [nonlinear.md:32](methods/nonlinear.md?plain=1#L32)
- Other documents: [ROADMAP.md:860](../ROADMAP.md?plain=1#L860)

#### Chelladurai et al. 2025

D. Chelladurai, M. Kohli, J. Winiger, D. Moor, A. Messner, Y. Fedoryshyn, M. Eleraky, Y. Liu, H. Wang, J. Leuthold, "Barium titanate and lithium niobate permittivity and Pockels coefficients from megahertz to sub-terahertz frequencies", Nat. Mater. 24, 868 (2025). [doi:10.1038/s41563-025-02158-1](https://doi.org/10.1038/s41563-025-02158-1), open access.

- Methods pages: [catalogue.md:54](methods/catalogue.md?plain=1#L54)
- Validation cases: [`material/batio3-abel-films`](validation.md?plain=1#L76), [`pockels/bto-angle`](validation.md?plain=1#L555)
- Code: [src/cases/mode/pockels/checks.rs:1047](../src/cases/mode/pockels/checks.rs#L1047), [src/device/tfln.rs:1211](../src/device/tfln.rs#L1211), [src/material/catalogue/references.rs:200](../src/material/catalogue/references.rs#L200), [src/validation.rs:557](../src/validation.rs#L557)

#### Chembo and Menyuk 2013

Y. K. Chembo, C. R. Menyuk, "Spatiotemporal Lugiato-Lefever formalism for Kerr-comb generation in whispering-gallery-mode resonators", Phys. Rev. A 87, 053852 (2013). [doi:10.1103/PhysRevA.87.053852](https://doi.org/10.1103/PhysRevA.87.053852)

- Other documents: [ROADMAP.md:855](../ROADMAP.md?plain=1#L855)

#### Chen 2015

Y. Chen, "Nanofabrication by electron beam lithography and its applications: A review", Microelectronic Engineering 135, 57 (2015). [doi:10.1016/j.mee.2015.02.042](https://doi.org/10.1016/j.mee.2015.02.042)

- Methods pages: [fabrication.md:16](methods/fabrication.md?plain=1#L16)
- Validation cases: [`fabrication/lithography-choice`](validation.md?plain=1#L746)
- Code: [src/fabrication/checks.rs:447](../src/fabrication/checks.rs#L447), [src/fabrication/references.rs:64](../src/fabrication/references.rs#L64)
- Other documents: [docs/plans/fabrication.md:627](plans/fabrication.md?plain=1#L627)

#### Chen et al. 2024

M. Chen, R. E. Christiansen, J. A. Fan, G. Işiklar, J. Jiang, S. G. Johnson, W. Ma, O. D. Miller, A. Oskooi, M. F. Schubert, et al., "Validation and characterization of algorithms and software for photonics inverse design", J. Opt. Soc. Am. B 41, A161 (2024). [doi:10.1364/JOSAB.506412](https://doi.org/10.1364/JOSAB.506412)

- Other documents: [ROADMAP.md:828](../ROADMAP.md?plain=1#L828), [docs/plans/optimization.md:601](plans/optimization.md?plain=1#L601)

#### Cheng and Chrostowski 2021

R. Cheng, L. Chrostowski, "Spectral Design of Silicon Integrated Bragg Gratings: A Tutorial", J. Lightwave Technol. 39, 712 (2021). [doi:10.1109/JLT.2020.3035372](https://doi.org/10.1109/JLT.2020.3035372)

- Academy lessons: [bragg-gratings.md:109](../academy/bragg-gratings.md?plain=1#L109), [313](../academy/bragg-gratings.md?plain=1#L313), [407](../academy/bragg-gratings.md?plain=1#L407)

#### Cherchi et al. 2013

M. Cherchi, S. Ylinen, M. Harjanne, M. Kapulainen, T. Aalto, "Dramatic size reduction of waveguide bends on a micron-scale silicon photonic platform", Opt. Express 21, 17814 (2013). [doi:10.1364/OE.21.017814](https://doi.org/10.1364/OE.21.017814)

- Methods pages: [generators.md:10](methods/generators.md?plain=1#L10), [geometry.md:16](methods/geometry.md?plain=1#L16)
- Academy lessons: [leaky-and-bent-guides.md:125](../academy/leaky-and-bent-guides.md?plain=1#L125), [578](../academy/leaky-and-bent-guides.md?plain=1#L578)
- Validation cases: [`geometry/paths/cherchi-l-bend`](validation.md?plain=1#L405)
- Code: [src/geometry/path_checks.rs:425](../src/geometry/path_checks.rs#L425)

#### Chew and Weedon 1994

W. C. Chew, W. H. Weedon, "A 3D perfectly matched medium from modified maxwell's equations with stretched coordinates", Micro & Optical Tech Letters 7, 599 (1994). [doi:10.1002/mop.4650071304](https://doi.org/10.1002/mop.4650071304)

- Methods pages: [fdfd-3d.md:14](methods/fdfd-3d.md?plain=1#L14), [fdfd.md:12](methods/fdfd.md?plain=1#L12), [pml.md:10](methods/pml.md?plain=1#L10), [travelling-wave.md:28](methods/travelling-wave.md?plain=1#L28)
- Academy lessons: [leaky-and-bent-guides.md:71](../academy/leaky-and-bent-guides.md?plain=1#L71), [316](../academy/leaky-and-bent-guides.md?plain=1#L316), [563](../academy/leaky-and-bent-guides.md?plain=1#L563)
- Validation cases: [`fdfd3d/pml-reflection-stretched`](validation.md?plain=1#L178)
- Code: [src/electro/full_wave.rs:43](../src/electro/full_wave.rs#L43), [src/fdfd/mod.rs:26](../src/fdfd/mod.rs#L26), [src/mode/vector.rs:172](../src/mode/vector.rs#L172), [src/validation.rs:1271](../src/validation.rs#L1271)

#### Chew et al. 1997

W. C. Chew, J. M. Jin, E. Michielssen, "Complex coordinate stretching as a generalized absorbing boundary condition", Microw. Opt. Technol. Lett. 15, 363 (1997). [doi:10.1002/(SICI)1098-2760(19970820)15:6\<363::AID-MOP8\>3.0.CO;2-C](https://doi.org/10.1002/(SICI)1098-2760(19970820)15:6%3C363::AID-MOP8%3E3.0.CO;2-C)

- Methods pages: [eme.md:10](methods/eme.md?plain=1#L10), [pml.md:8](methods/pml.md?plain=1#L8)
- Academy lessons: [leaky-and-bent-guides.md:77](../academy/leaky-and-bent-guides.md?plain=1#L77)
- Code: [src/mode/vector.rs:170](../src/mode/vector.rs#L170)

#### Chilwell and Hodgkinson 1984

J. Chilwell, I. Hodgkinson, "Thin-films field-transfer matrix theory of planar multilayer waveguides and reflection from prism-loaded waveguides", J. Opt. Soc. Am. A 1, 742 (1984). [doi:10.1364/JOSAA.1.000742](https://doi.org/10.1364/JOSAA.1.000742)

- Methods pages: [contour.md:14](methods/contour.md?plain=1#L14), [devices.md:48](methods/devices.md?plain=1#L48), [eme.md:12](methods/eme.md?plain=1#L12), [fdfd-3d.md:24](methods/fdfd-3d.md?plain=1#L24), [fdfd.md:14](methods/fdfd.md?plain=1#L14), [multilayer.md:8](methods/multilayer.md?plain=1#L8), [pml.md:12](methods/pml.md?plain=1#L12), [slab-fd.md:10](methods/slab-fd.md?plain=1#L10)
- Academy lessons: [bragg-gratings.md:61](../academy/bragg-gratings.md?plain=1#L61), [168](../academy/bragg-gratings.md?plain=1#L168), [leaky-and-bent-guides.md:65](../academy/leaky-and-bent-guides.md?plain=1#L65), [165](../academy/leaky-and-bent-guides.md?plain=1#L165), [559](../academy/leaky-and-bent-guides.md?plain=1#L559), [585](../academy/leaky-and-bent-guides.md?plain=1#L585), [slab-waveguide.md:60](../academy/slab-waveguide.md?plain=1#L60), [480](../academy/slab-waveguide.md?plain=1#L480), [574](../academy/slab-waveguide.md?plain=1#L574), [599](../academy/slab-waveguide.md?plain=1#L599)
- Validation cases: [`mode/multilayer-bound-chilwell`](validation.md?plain=1#L110), [`mode/multilayer-leaky-chilwell`](validation.md?plain=1#L111), [`mode/multilayer-power-chilwell`](validation.md?plain=1#L112), [`mode/multilayer-fresnel`](validation.md?plain=1#L113), [`mode/pml-leaky-chilwell`](validation.md?plain=1#L119), [`mode/slab-fd-chilwell`](validation.md?plain=1#L130), [`fdfd/slab-reflection-ez`](validation.md?plain=1#L131), [`fdfd3d/film-reflection-te`](validation.md?plain=1#L159), [`fdtd/bloch-multilayer`](validation.md?plain=1#L222), [`mode/contour-chilwell-bound`](validation.md?plain=1#L465), [`mode/contour-chilwell-leaky`](validation.md?plain=1#L466), [`mode/krylov-schur-chilwell`](validation.md?plain=1#L473), [`mode/every-guided-mode-chilwell`](validation.md?plain=1#L477), [`job/leaky-film-loss`](validation.md?plain=1#L478), [`device/pcm-analytic-index`](validation.md?plain=1#L661), [`sweep/slab-dispersion`](validation.md?plain=1#L709)
- Examples: [README.md:43](../examples/README.md?plain=1#L43), [leaky_waves.rs:4](../examples/leaky_waves.rs#L4), [multilayer_chilwell.rs:5](../examples/multilayer_chilwell.rs#L5)
- Code: [src/cases/device/pcm_checks.rs:426](../src/cases/device/pcm_checks.rs#L426), [src/cases/eigen/checks.rs:346](../src/cases/eigen/checks.rs#L346), [377](../src/cases/eigen/checks.rs#L377), [src/device/pcm.rs:983](../src/device/pcm.rs#L983), [src/job/leaky_checks.rs:36](../src/job/leaky_checks.rs#L36), [src/mode/multilayer.rs:5](../src/mode/multilayer.rs#L5), [src/mode/region/checks.rs:373](../src/mode/region/checks.rs#L373), [380](../src/mode/region/checks.rs#L380), [src/sweep/checks.rs:443](../src/sweep/checks.rs#L443), [src/validation.rs:795](../src/validation.rs#L795), [802](../src/validation.rs#L802), [809](../src/validation.rs#L809), [816](../src/validation.rs#L816), [858](../src/validation.rs#L858), [935](../src/validation.rs#L935), [942](../src/validation.rs#L942), [1138](../src/validation.rs#L1138), [1579](../src/validation.rs#L1579), [studio/src-tauri/src/charts/leaky.rs:4](../studio/src-tauri/src/charts/leaky.rs#L4)

#### Chou et al. 1996

S. Y. Chou, P. R. Krauss, P. J. Renstrom, "Imprint Lithography with 25-Nanometer Resolution", Science 272, 85 (1996). [doi:10.1126/science.272.5258.85](https://doi.org/10.1126/science.272.5258.85)

- Code: [src/fabrication/references.rs:72](../src/fabrication/references.rs#L72)
- Other documents: [docs/plans/fabrication.md:626](plans/fabrication.md?plain=1#L626)

#### Chow and Patel 2015

E. Chow, A. Patel, "Fine-Grained Parallel Incomplete LU Factorization", SIAM J. Sci. Comput. 37, C169 (2015). [doi:10.1137/140968896](https://doi.org/10.1137/140968896)

- Methods pages: [fdfd-3d.md:42](methods/fdfd-3d.md?plain=1#L42), [858](methods/fdfd-3d.md?plain=1#L858)
- Other documents: [docs/plans/performance.md:603](plans/performance.md?plain=1#L603)

#### Choy and Byer 1976

M. M. Choy, R. L. Byer, "Accurate second-order susceptibility measurements of visible and infrared nonlinear crystals", Phys. Rev. B 14, 1693 (1976). [doi:10.1103/PhysRevB.14.1693](https://doi.org/10.1103/PhysRevB.14.1693)

- Code: [src/material/catalogue/references.rs:221](../src/material/catalogue/references.rs#L221)

#### Christ and Hartnagel 1987

A. Christ, H. Hartnagel, "Three-Dimensional Finite-Difference Method for the Analysis of Microwave-Device Embedding", IEEE Trans. Microwave Theory Techn. 35, 688 (1987). [doi:10.1109/TMTT.1987.1133733](https://doi.org/10.1109/TMTT.1987.1133733)

- Methods pages: [fdfd-3d.md:8](methods/fdfd-3d.md?plain=1#L8)
- Code: [src/fdfd/mod.rs:18](../src/fdfd/mod.rs#L18), [src/fdfd/three.rs:4](../src/fdfd/three.rs#L4)

#### Christiansen and Sigmund 2021

R. E. Christiansen, O. Sigmund, "Inverse design in photonics by topology optimization: tutorial", J. Opt. Soc. Am. B 38, 496 (2021). [doi:10.1364/JOSAB.406048](https://doi.org/10.1364/JOSAB.406048)

- Other documents: [ROADMAP.md:775](../ROADMAP.md?plain=1#L775), [docs/plans/optimization.md:597](plans/optimization.md?plain=1#L597)

#### Chrostowski and Hochberg 2015

L. Chrostowski, M. Hochberg, "Silicon Photonics Design: From Devices to Systems", Cambridge University Press (2015). [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168)

- Methods pages: [components.md:12](methods/components.md?plain=1#L12), [devices.md:8](methods/devices.md?plain=1#L8), [dispersion.md:8](methods/dispersion.md?plain=1#L8), [eim.md:10](methods/eim.md?plain=1#L10), [slab.md:11](methods/slab.md?plain=1#L11), [vector.md:12](methods/vector.md?plain=1#L12)
- Academy lessons: [directional-coupler.md:88](../academy/directional-coupler.md?plain=1#L88), [130](../academy/directional-coupler.md?plain=1#L130), [495](../academy/directional-coupler.md?plain=1#L495), [material-dispersion.md:68](../academy/material-dispersion.md?plain=1#L68), [349](../academy/material-dispersion.md?plain=1#L349), [slab-waveguide.md:84](../academy/slab-waveguide.md?plain=1#L84), [114](../academy/slab-waveguide.md?plain=1#L114), [583](../academy/slab-waveguide.md?plain=1#L583), [strip-waveguide.md:130](../academy/strip-waveguide.md?plain=1#L130), [160](../academy/strip-waveguide.md?plain=1#L160)
- Validation cases: [`mode/slab-te-book`](validation.md?plain=1#L99), [`mode/slab-tm-book`](validation.md?plain=1#L100), [`mode/strip-book`](validation.md?plain=1#L103), [`mode/strip-group-index-book`](validation.md?plain=1#L109), [`mode/eim-strip-book`](validation.md?plain=1#L120), [`components/directional-coupler-power`](validation.md?plain=1#L300), [`device/coupler-closed-forms`](validation.md?plain=1#L311), [`device/strip-chrostowski`](validation.md?plain=1#L324), [`device/coupler-chrostowski`](validation.md?plain=1#L325), [`device/study-coupler-chrostowski`](validation.md?plain=1#L349)
- Examples: [README.md:36](../examples/README.md?plain=1#L36), [40](../examples/README.md?plain=1#L40), [60](../examples/README.md?plain=1#L60), [directional_coupler.rs:4](../examples/directional_coupler.rs#L4), [42](../examples/directional_coupler.rs#L42), [effective_index_method.rs:4](../examples/effective_index_method.rs#L4), [group_index.rs:4](../examples/group_index.rs#L4), [48](../examples/group_index.rs#L48), [slab_soi.rs:4](../examples/slab_soi.rs#L4), [strip_waveguide.rs:5](../examples/strip_waveguide.rs#L5), [62](../examples/strip_waveguide.rs#L62), [python/examples/README.md:27](../python/examples/README.md?plain=1#L27), [python/examples/output/strip_waveguide.txt:15](../python/examples/output/strip_waveguide.txt#L15), [16](../python/examples/output/strip_waveguide.txt#L16), [python/examples/slab_soi.py:5](../python/examples/slab_soi.py#L5), [python/examples/strip_waveguide.py:5](../python/examples/strip_waveguide.py#L5), [59](../python/examples/strip_waveguide.py#L59), [64](../python/examples/strip_waveguide.py#L64)
- Code: [python/tests/test_api.py:29](../python/tests/test_api.py#L29), [src/circuit/components/coupler.rs:200](../src/circuit/components/coupler.rs#L200), [253](../src/circuit/components/coupler.rs#L253), [src/circuit/components/waveguide.rs:20](../src/circuit/components/waveguide.rs#L20), [327](../src/circuit/components/waveguide.rs#L327), [src/device/bend.rs:250](../src/device/bend.rs#L250), [src/device/checks.rs:75](../src/device/checks.rs#L75), [166](../src/device/checks.rs#L166), [173](../src/device/checks.rs#L173), [721](../src/device/checks.rs#L721), [src/device/fit.rs:12](../src/device/fit.rs#L12), [src/device/rib.rs:297](../src/device/rib.rs#L297), [src/device/tests.rs:353](../src/device/tests.rs#L353), [490](../src/device/tests.rs#L490), [src/device/waveguide.rs:22](../src/device/waveguide.rs#L22), [src/facade/device.rs:286](../src/facade/device.rs#L286), [src/job/study_checks.rs:43](../src/job/study_checks.rs#L43), [src/mode/dispersion.rs:5](../src/mode/dispersion.rs#L5), [src/mode/eim.rs:4](../src/mode/eim.rs#L4), [src/mode/slab.rs:13](../src/mode/slab.rs#L13), [src/stack.rs:134](../src/stack.rs#L134), [src/validation.rs:718](../src/validation.rs#L718), [725](../src/validation.rs#L725), [746](../src/validation.rs#L746), [788](../src/validation.rs#L788), [865](../src/validation.rs#L865), [2125](../src/validation.rs#L2125), [4722](../src/validation.rs#L4722), [studio/src-tauri/src/charts/coupler.rs:7](../studio/src-tauri/src/charts/coupler.rs#L7), [studio/src-tauri/src/charts/leaky.rs:36](../studio/src-tauri/src/charts/leaky.rs#L36), [studio/src-tauri/src/charts/slab.rs:8](../studio/src-tauri/src/charts/slab.rs#L8), [studio/src-tauri/src/circuits/library.rs:136](../studio/src-tauri/src/circuits/library.rs#L136), [studio/src-tauri/src/examples.rs:602](../studio/src-tauri/src/examples.rs#L602)
- Other documents: [CONTRIBUTING.md:26](../CONTRIBUTING.md?plain=1#L26), [72](../CONTRIBUTING.md?plain=1#L72), [74](../CONTRIBUTING.md?plain=1#L74), [ROADMAP.md:748](../ROADMAP.md?plain=1#L748)

#### Clements et al. 2016

W. R. Clements, P. C. Humphreys, B. J. Metcalf, W. S. Kolthammer, I. A. Walsmley, "Optimal design for universal multiport interferometers", Optica 3, 1460 (2016). [doi:10.1364/OPTICA.3.001460](https://doi.org/10.1364/OPTICA.3.001460), open access.

- Methods pages: [circuit-adjoint.md:12](methods/circuit-adjoint.md?plain=1#L12)
- Academy lessons: [mach-zehnder.md:106](../academy/mach-zehnder.md?plain=1#L106), [370](../academy/mach-zehnder.md?plain=1#L370), [464](../academy/mach-zehnder.md?plain=1#L464)
- Validation cases: [`circuit/adjoint-mesh`](validation.md?plain=1#L285)
- Examples: [README.md:47](../examples/README.md?plain=1#L47), [56](../examples/README.md?plain=1#L56), [circuit_splitter.rs:6](../examples/circuit_splitter.rs#L6), [optimize_mesh.rs:7](../examples/optimize_mesh.rs#L7)
- Code: [src/circuit/adjoint/checks.rs:245](../src/circuit/adjoint/checks.rs#L245), [src/validation.rs:2020](../src/validation.rs#L2020), [tests/jobs-toml/chip-splitter.toml:1](../tests/jobs-toml/chip-splitter.toml#L1)
- Jobs and circuits: [jobs/chip-splitter.yaml:1](../jobs/chip-splitter.yaml#L1)
- Other documents: [ROADMAP.md:821](../ROADMAP.md?plain=1#L821), [docs/plans/optimization.md:627](plans/optimization.md?plain=1#L627), [docs/plans/quantum.md:479](plans/quantum.md?plain=1#L479)

#### Clifford and Clifford 2018

P. Clifford, R. Clifford, "The Classical Complexity of Boson Sampling", Proceedings of the Twenty-Ninth Annual ACM-SIAM Symposium on Discrete Algorithms, Society for Industrial and Applied Mathematics (2018). [doi:10.1137/1.9781611975031.10](https://doi.org/10.1137/1.9781611975031.10)

- Other documents: [docs/plans/quantum.md:499](plans/quantum.md?plain=1#L499)

#### Coello et al. 2004

C. Coello, G. Pulido, M. Lechuga, "Handling multiple objectives with particle swarm optimization", IEEE Trans. Evol. Computat. 8, 256 (2004). [doi:10.1109/TEVC.2004.826067](https://doi.org/10.1109/TEVC.2004.826067)

- Other documents: [docs/plans/optimization.md:581](plans/optimization.md?plain=1#L581)

#### Coen et al. 2012

S. Coen, H. G. Randle, T. Sylvestre, M. Erkintalo, "Modeling of octave-spanning Kerr frequency combs using a generalized mean-field Lugiato–Lefever model", Opt. Lett. 38, 37 (2012). [doi:10.1364/OL.38.000037](https://doi.org/10.1364/OL.38.000037)

- Methods pages: [nonlinear.md:38](methods/nonlinear.md?plain=1#L38)
- Validation cases: [`nonlinear/lle-coen-2012-duration`](validation.md?plain=1#L798)
- Examples: [README.md:87](../examples/README.md?plain=1#L87), [soliton_coen.rs:5](../examples/soliton_coen.rs#L5)
- Code: [src/nonlinear/kerr_checks.rs:784](../src/nonlinear/kerr_checks.rs#L784), [src/nonlinear/resonator.rs:7](../src/nonlinear/resonator.rs#L7)
- Other documents: [ROADMAP.md:836](../ROADMAP.md?plain=1#L836)

#### Coenen et al. 2022

D. Coenen, H. Oprins, P. De Heyn, J. Van Campenhout, I. De Wolf, "Analysis of Thermal Crosstalk in Photonic Integrated Circuit Using Dynamic Compact Models", IEEE Trans. Compon., Packag. Manufact. Technol. 12, 1350 (2022). [doi:10.1109/TCPMT.2022.3195699](https://doi.org/10.1109/TCPMT.2022.3195699)

- Methods pages: [thermal-compact.md:8](methods/thermal-compact.md?plain=1#L8), [23](methods/thermal-compact.md?plain=1#L23)
- Validation cases: [`thermal-rc/coenen-cauer-ucut`](validation.md?plain=1#L632), [`thermal-rc/coenen-cauer`](validation.md?plain=1#L633), [`thermal-rc/coenen-200khz`](validation.md?plain=1#L634), [`thermal-rc/coenen-start-up`](validation.md?plain=1#L635)
- Code: [src/circuit/heated.rs:124](../src/circuit/heated.rs#L124), [src/thermal/rc.rs:4](../src/thermal/rc.rs#L4), [src/thermal/rc_checks.rs:233](../src/thermal/rc_checks.rs#L233), [243](../src/thermal/rc_checks.rs#L243), [253](../src/thermal/rc_checks.rs#L253), [264](../src/thermal/rc_checks.rs#L264)
- Other documents: [ROADMAP.md:804](../ROADMAP.md?plain=1#L804)

#### Collet 2021

Y. Collet, "Zstandard Compression and the 'application/zstd' Media Type", RFC Editor (2021). [doi:10.17487/RFC8878](https://doi.org/10.17487/RFC8878)

- Methods pages: [export.md:122](methods/export.md?plain=1#L122)
- Code: [native/src/zstd.rs:1](../native/src/zstd.rs#L1)

#### Cools and Vanroose 2017

S. Cools, W. Vanroose, "The communication-hiding pipelined BiCGstab method for the parallel solution of large unsymmetric linear systems", Parallel Computing 65, 1 (2017). [doi:10.1016/j.parco.2017.04.005](https://doi.org/10.1016/j.parco.2017.04.005)

- Other documents: [docs/plans/performance.md:604](plans/performance.md?plain=1#L604)

#### Cools et al. 2013

S. Cools, B. Reps, W. Vanroose, "A new level-dependent coarse grid correction scheme for indefinite Helmholtz problems: LEVEL-DEPENDENT MULTIGRID SCHEME FOR HELMHOLTZ PROBLEMS", Numer. Linear Algebra Appl. 21, 513 (2013). [doi:10.1002/nla.1895](https://doi.org/10.1002/nla.1895)

- Methods pages: [multigrid-stencil.md:20](methods/multigrid-stencil.md?plain=1#L20)

#### Crank and Nicolson 1947

J. Crank, P. Nicolson, "A practical method for numerical evaluation of solutions of partial differential equations of the heat-conduction type", Math. Proc. Camb. Phil. Soc. 43, 50 (1947). [doi:10.1017/S0305004100023197](https://doi.org/10.1017/S0305004100023197)

- Methods pages: [static.md:22](methods/static.md?plain=1#L22)

### D

#### Dai and Bowers 2011

D. Dai, J. E. Bowers, "Novel ultra-short and ultra-broadband polarization beam splitter based on a bent directional coupler", Opt. Express 19, 18614 (2011). [doi:10.1364/OE.19.018614](https://doi.org/10.1364/OE.19.018614)

- Academy lessons: [directional-coupler.md:70](../academy/directional-coupler.md?plain=1#L70), [382](../academy/directional-coupler.md?plain=1#L382), [506](../academy/directional-coupler.md?plain=1#L506)

#### Dai et al. 2013

D. Dai, J. Wang, Y. Shi, "Silicon mode (de)multiplexer enabling high capacity photonic networks-on-chip with a single-wavelength-carrier light", Opt. Lett. 38, 1422 (2013). [doi:10.1364/OL.38.001422](https://doi.org/10.1364/OL.38.001422)

- Academy lessons: [directional-coupler.md:82](../academy/directional-coupler.md?plain=1#L82), [386](../academy/directional-coupler.md?plain=1#L386), [507](../academy/directional-coupler.md?plain=1#L507)

#### Daubechies 1988

I. Daubechies, "Orthonormal bases of compactly supported wavelets", Comm Pure Appl Math 41, 909 (1988). [doi:10.1002/cpa.3160410705](https://doi.org/10.1002/cpa.3160410705)

- Examples: [optimize_mzi_lattice.rs:17](../examples/optimize_mzi_lattice.rs#L17)

#### de Boor 1978

C. de Boor, "A Practical Guide to Splines", Springer New York (1978). [doi:10.1007/978-1-4612-6333-3](https://doi.org/10.1007/978-1-4612-6333-3)

- Methods pages: [expressions.md:13](methods/expressions.md?plain=1#L13), [46](methods/expressions.md?plain=1#L46)
- Code: [src/expr/parse.rs:908](../src/expr/parse.rs#L908)

#### Deal and Grove 1965

B. E. Deal, A. S. Grove, "General Relationship for the Thermal Oxidation of Silicon", Journal of Applied Physics 36, 3770 (1965). [doi:10.1063/1.1713945](https://doi.org/10.1063/1.1713945)

- Methods pages: [fabrication.md:26](methods/fabrication.md?plain=1#L26)
- Validation cases: [`fabrication/deal-grove-tables`](validation.md?plain=1#L744), [`fabrication/deal-grove-campbell`](validation.md?plain=1#L750)
- Code: [src/fabrication/checks.rs:433](../src/fabrication/checks.rs#L433), [src/fabrication/oxidation.rs:3](../src/fabrication/oxidation.rs#L3), [src/fabrication/references.rs:96](../src/fabrication/references.rs#L96), [src/fabrication/rule_checks.rs:330](../src/fabrication/rule_checks.rs#L330)
- Other documents: [docs/plans/fabrication.md:624](plans/fabrication.md?plain=1#L624)

#### Deb et al. 2002

K. Deb, A. Pratap, S. Agarwal, T. Meyarivan, "A fast and elitist multiobjective genetic algorithm: NSGA-II", IEEE Trans. Evol. Computat. 6, 182 (2002). [doi:10.1109/4235.996017](https://doi.org/10.1109/4235.996017)

- Methods pages: [optimization.md:139](methods/optimization.md?plain=1#L139)
- Code: [src/job/optimize.rs:253](../src/job/optimize.rs#L253)
- Other documents: [docs/plans/optimization.md:582](plans/optimization.md?plain=1#L582)

#### Delaney et al. 2020a

M. Delaney, I. Zeimpekis, D. Lawson, D. W. Hewak, O. L. Muskens, "A New Family of Ultralow Loss Reversible Phase‐Change Materials for Photonic Integrated Circuits: Sb 2 S 3 and Sb 2 Se 3", Adv Funct Materials 30, 2002447 (2020). [doi:10.1002/adfm.202002447](https://doi.org/10.1002/adfm.202002447), open access.

- Methods pages: [catalogue.md:346](methods/catalogue.md?plain=1#L346), [devices.md:44](methods/devices.md?plain=1#L44), [fabrication.md:44](methods/fabrication.md?plain=1#L44)
- Validation cases: [`material/pcm-delaney-fits`](validation.md?plain=1#L45), [`material/pcm-delaney-fits-gst`](validation.md?plain=1#L46), [`material/pcm-delaney-contrast`](validation.md?plain=1#L47), [`material/pcm-delaney-table-1`](validation.md?plain=1#L48), [`material/pcm-delaney-losses`](validation.md?plain=1#L49), [`material/pcm-delaney-fom`](validation.md?plain=1#L50), [`material/pcm-delaney-lossless`](validation.md?plain=1#L51), [`material/pcm-sb2se3-crystallization`](validation.md?plain=1#L52), [`material/pcm-sb2s3-crystallization`](validation.md?plain=1#L53), [`device/pcm-fom`](validation.md?plain=1#L666), [`device/pcm-conformal-area`](validation.md?plain=1#L669), [`fabrication/thermal-order`](validation.md?plain=1#L747), [`fabrication/pcm-rules`](validation.md?plain=1#L759)
- Examples: [README.md:82](../examples/README.md?plain=1#L82), [pcm_delaney.rs:6](../examples/pcm_delaney.rs#L6)
- Code: [src/cases/device/pcm_checks.rs:461](../src/cases/device/pcm_checks.rs#L461), [482](../src/cases/device/pcm_checks.rs#L482), [src/device/pcm.rs:963](../src/device/pcm.rs#L963), [src/fabrication/checks.rs:454](../src/fabrication/checks.rs#L454), [src/fabrication/platform_checks.rs:742](../src/fabrication/platform_checks.rs#L742), [src/fabrication/references.rs:192](../src/fabrication/references.rs#L192), [src/material/catalogue/phase_change.rs:6](../src/material/catalogue/phase_change.rs#L6), [686](../src/material/catalogue/phase_change.rs#L686), [src/material/catalogue/references.rs:11](../src/material/catalogue/references.rs#L11), [src/validation.rs:340](../src/validation.rs#L340), [347](../src/validation.rs#L347), [354](../src/validation.rs#L354), [361](../src/validation.rs#L361), [368](../src/validation.rs#L368), [375](../src/validation.rs#L375), [382](../src/validation.rs#L382), [389](../src/validation.rs#L389), [396](../src/validation.rs#L396)
- Other documents: [ROADMAP.md:806](../ROADMAP.md?plain=1#L806), [docs/plans/fabrication.md:660](plans/fabrication.md?plain=1#L660)

#### Delaney et al. 2020b

M. Delaney, I. Zeimpekis, D. Lawson, D. Hewak, O. Muskens, "Dataset for A new family of ultra-low loss reversible phase change materials for photonic integrated circuits: Sb2S3 and Sb2Se3", University of Southampton (2020). [doi:10.5258/SOTON/D1247](https://doi.org/10.5258/SOTON/D1247)

- Methods pages: [catalogue.md:347](methods/catalogue.md?plain=1#L347)
- Validation cases: [`material/pcm-delaney-fits`](validation.md?plain=1#L45), [`material/pcm-delaney-fits-gst`](validation.md?plain=1#L46), [`material/pcm-delaney-losses`](validation.md?plain=1#L49), [`material/pcm-sb2se3-crystallization`](validation.md?plain=1#L52), [`material/pcm-sb2s3-crystallization`](validation.md?plain=1#L53)
- Code: [src/material/catalogue/phase_change.rs:7](../src/material/catalogue/phase_change.rs#L7), [687](../src/material/catalogue/phase_change.rs#L687), [src/material/catalogue/references.rs:18](../src/material/catalogue/references.rs#L18), [src/validation.rs:340](../src/validation.rs#L340), [347](../src/validation.rs#L347), [368](../src/validation.rs#L368), [389](../src/validation.rs#L389), [396](../src/validation.rs#L396)

#### Delaney et al. 2021

M. Delaney, I. Zeimpekis, H. Du, X. Yan, M. Banakar, D. J. Thomson, D. W. Hewak, O. L. Muskens, "Nonvolatile programmable silicon photonics using an ultralow-loss Sb 2 Se 3 phase change material", Sci. Adv. 7, eabg3500 (2021). [doi:10.1126/sciadv.abg3500](https://doi.org/10.1126/sciadv.abg3500), open access.

- Methods pages: [fabrication.md:46](methods/fabrication.md?plain=1#L46)
- Validation cases: [`fabrication/pcm-reported-flows`](validation.md?plain=1#L760)
- Code: [src/fabrication/platform_checks.rs:749](../src/fabrication/platform_checks.rs#L749), [src/fabrication/references.rs:232](../src/fabrication/references.rs#L232), [234](../src/fabrication/references.rs#L234)
- Other documents: [docs/plans/fabrication.md:660](plans/fabrication.md?plain=1#L660)

#### Demmel and Nguyen 2015

J. Demmel, H. D. Nguyen, "Parallel Reproducible Summation", IEEE Trans. Comput. 64, 2060 (2015). [doi:10.1109/TC.2014.2345391](https://doi.org/10.1109/TC.2014.2345391)

- Other documents: [docs/plans/performance.md:605](plans/performance.md?plain=1#L605)

#### Demmel et al. 1999

J. W. Demmel, S. C. Eisenstat, J. R. Gilbert, X. S. Li, J. W. H. Liu, "A Supernodal Approach to Sparse Partial Pivoting", SIAM J. Matrix Anal. Appl. 20, 720 (1999). [doi:10.1137/S0895479895291765](https://doi.org/10.1137/S0895479895291765)

- Code: [native/src/superlu.rs:3](../native/src/superlu.rs#L3)

#### Deng et al. 2026

C. Deng, Y. He, W. Yang, H. Yu, Z. Hong, H. Liu, H. Han, W. Li, Y. Ma, Z. Zhang, et al., "Self-buffered epitaxy of barium titanate on oxide insulators enables high-performance electro-optic modulators", Light Sci Appl 15, 21 (2026). [doi:10.1038/s41377-025-02081-9](https://doi.org/10.1038/s41377-025-02081-9), open access.

- Validation cases: [`material/batio3-abel-films`](validation.md?plain=1#L76), [`pockels/bto-deng`](validation.md?plain=1#L556)
- Code: [src/cases/mode/pockels/checks.rs:1054](../src/cases/mode/pockels/checks.rs#L1054), [src/material/catalogue/references.rs:501](../src/material/catalogue/references.rs#L501), [502](../src/material/catalogue/references.rs#L502), [src/validation.rs:557](../src/validation.rs#L557)
- Other documents: [ROADMAP.md:790](../ROADMAP.md?plain=1#L790)

#### DeSalvo et al. 1996

R. DeSalvo, A. Said, D. Hagan, E. Van Stryland, M. Sheik-Bahae, "Infrared to ultraviolet measurements of two-photon absorption and n/sub 2/ in wide bandgap solids", IEEE J. Quantum Electron. 32, 1324 (1996). [doi:10.1109/3.511545](https://doi.org/10.1109/3.511545)

- Methods pages: [catalogue.md:34](methods/catalogue.md?plain=1#L34)
- Validation cases: [`material/kerr-desalvo-esu`](validation.md?plain=1#L731)
- Code: [src/material/catalogue/kerr.rs:14](../src/material/catalogue/kerr.rs#L14), [src/material/catalogue/references.rs:599](../src/material/catalogue/references.rs#L599), [src/nonlinear/data_checks.rs:305](../src/nonlinear/data_checks.rs#L305)

#### Deschrijver et al. 2008

D. Deschrijver, M. Mrozowski, T. Dhaene, D. De Zutter, "Macromodeling of Multiport Systems Using a Fast Implementation of the Vector Fitting Method", IEEE Microw. Wireless Compon. Lett. 18, 383 (2008). [doi:10.1109/LMWC.2008.922585](https://doi.org/10.1109/LMWC.2008.922585)

- Methods pages: [compact.md:10](methods/compact.md?plain=1#L10)
- Validation cases: [`compact/vf-fast-equivalence`](validation.md?plain=1#L380)
- Code: [src/cases/compact/checks.rs:733](../src/cases/compact/checks.rs#L733), [src/compact/fit.rs:39](../src/compact/fit.rs#L39)

#### Dietrich et al. 2016

C. P. Dietrich, A. Fiore, M. G. Thompson, M. Kamp, S. Höfling, "GaAs integrated quantum photonics: Towards compact and multi‐functional quantum photonic integrated circuits", Laser & Photonics Reviews 10, 870 (2016). [doi:10.1002/lpor.201500321](https://doi.org/10.1002/lpor.201500321)

- Methods pages: [fabrication.md:52](methods/fabrication.md?plain=1#L52)
- Validation cases: [`fabrication/gaas-rules`](validation.md?plain=1#L764)
- Code: [src/fabrication/platform_checks.rs:777](../src/fabrication/platform_checks.rs#L777), [src/fabrication/references.rs:216](../src/fabrication/references.rs#L216)
- Other documents: [docs/plans/fabrication.md:662](plans/fabrication.md?plain=1#L662)

#### Dolean et al. 2009

V. Dolean, M. Gander, L. Gerardo-Giorda, "Optimized Schwarz Methods for Maxwell's Equations", SIAM J. Sci. Comput. 31, 2193 (2009). [doi:10.1137/080728536](https://doi.org/10.1137/080728536)

- Other documents: [docs/plans/performance.md:606](plans/performance.md?plain=1#L606)

#### Dolean et al. 2015

V. Dolean, P. Jolivet, F. Nataf, "An Introduction to Domain Decomposition Methods: Algorithms, Theory, and Parallel Implementation", Society for Industrial and Applied Mathematics (2015). [doi:10.1137/1.9781611974065](https://doi.org/10.1137/1.9781611974065)

- Other documents: [docs/plans/performance.md:607](plans/performance.md?plain=1#L607)

#### Dolev et al. 2009

I. Dolev, A. Ganany-Padowicz, O. Gayer, A. Arie, J. Mangin, G. Gadret, "Linear and nonlinear optical properties of MgO:LiTaO3", Appl. Phys. B 96, 423 (2009). [doi:10.1007/s00340-009-3502-3](https://doi.org/10.1007/s00340-009-3502-3)

- Methods pages: [catalogue.md:20](methods/catalogue.md?plain=1#L20)
- Code: [src/material/catalogue/litao3_dndt.rs:3](../src/material/catalogue/litao3_dndt.rs#L3), [src/material/catalogue/references.rs:550](../src/material/catalogue/references.rs#L550)

#### Donnelly and Kornblit 2013

V. M. Donnelly, A. Kornblit, "Plasma etching: Yesterday, today, and tomorrow", Journal of Vacuum Science & Technology A: Vacuum, Surfaces, and Films 31, 050825 (2013). [doi:10.1116/1.4819316](https://doi.org/10.1116/1.4819316)

- Code: [src/fabrication/references.rs:80](../src/fabrication/references.rs#L80)
- Other documents: [docs/plans/fabrication.md:628](plans/fabrication.md?plain=1#L628)

#### Drude 1900

P. Drude, "Zur Elektronentheorie der Metalle", Annalen der Physik 306, 566 (1900). [doi:10.1002/andp.19003060312](https://doi.org/10.1002/andp.19003060312)

- Academy lessons: [material-dispersion.md:26](../academy/material-dispersion.md?plain=1#L26), [173](../academy/material-dispersion.md?plain=1#L173), [437](../academy/material-dispersion.md?plain=1#L437)

#### Duff and Koster 2001

I. S. Duff, J. Koster, "On Algorithms For Permuting Large Entries to the Diagonal of a Sparse Matrix", SIAM J. Matrix Anal. Appl. 22, 973 (2001). [doi:10.1137/S0895479899358443](https://doi.org/10.1137/S0895479899358443)

- Methods pages: [fdfd-3d.md:36](methods/fdfd-3d.md?plain=1#L36), [356](methods/fdfd-3d.md?plain=1#L356)
- Validation cases: [`fdfd3d/ldlt-lu`](validation.md?plain=1#L174)
- Code: [src/sparse/matching.rs:5](../src/sparse/matching.rs#L5), [src/validation.rs:1243](../src/validation.rs#L1243)

#### Duff and Reid 1983

I. S. Duff, J. K. Reid, "The Multifrontal Solution of Indefinite Sparse Symmetric Linear", ACM Trans. Math. Softw. 9, 302 (1983). [doi:10.1145/356044.356047](https://doi.org/10.1145/356044.356047)

- Methods pages: [fdfd-3d.md:32](methods/fdfd-3d.md?plain=1#L32), [347](methods/fdfd-3d.md?plain=1#L347), [fdfd.md:18](methods/fdfd.md?plain=1#L18)
- Validation cases: [`fdfd3d/ldlt-lu`](validation.md?plain=1#L174)
- Code: [src/sparse/multifrontal.rs:12](../src/sparse/multifrontal.rs#L12), [src/validation.rs:1243](../src/validation.rs#L1243)

#### Dumon et al. 2004

P. Dumon, W. Bogaerts, V. Wiaux, J. Wouters, S. Beckx, J. Van Campenhout, D. Taillaert, B. Luyssaert, P. Bienstman, D. Van Thourhout, et al., "Low-Loss SOI Photonic Wires and Ring Resonators Fabricated With Deep UV Lithography", IEEE Photon. Technol. Lett. 16, 1328 (2004). [doi:10.1109/LPT.2004.826025](https://doi.org/10.1109/LPT.2004.826025)

- Academy lessons: [strip-waveguide.md:94](../academy/strip-waveguide.md?plain=1#L94), [620](../academy/strip-waveguide.md?plain=1#L620), [685](../academy/strip-waveguide.md?plain=1#L685)

#### Dwivedi et al. 2015

S. Dwivedi, A. Ruocco, M. Vanslembrouck, T. Spuesens, P. Bienstman, P. Dumon, T. Van Vaerenbergh, W. Bogaerts, "Experimental Extraction of Effective Refractive Index and Thermo-Optic Coefficients of Silicon-on-Insulator Waveguides Using Interferometers", J. Lightwave Technol. 33, 4471 (2015). [doi:10.1109/JLT.2015.2476603](https://doi.org/10.1109/JLT.2015.2476603)

- Methods pages: [components.md:18](methods/components.md?plain=1#L18), [249](methods/components.md?plain=1#L249), [devices.md:26](methods/devices.md?plain=1#L26), [fields.md:14](methods/fields.md?plain=1#L14)
- Academy lessons: [mach-zehnder.md:100](../academy/mach-zehnder.md?plain=1#L100), [198](../academy/mach-zehnder.md?plain=1#L198), [281](../academy/mach-zehnder.md?plain=1#L281), [317](../academy/mach-zehnder.md?plain=1#L317), [455](../academy/mach-zehnder.md?plain=1#L455), [strip-waveguide.md:136](../academy/strip-waveguide.md?plain=1#L136), [529](../academy/strip-waveguide.md?plain=1#L529), [697](../academy/strip-waveguide.md?plain=1#L697)
- Validation cases: [`components/mzi-neff-dwivedi-470`](validation.md?plain=1#L303), [`components/mzi-ng-dwivedi-470`](validation.md?plain=1#L304), [`components/mzi-neff-dwivedi-602`](validation.md?plain=1#L305), [`components/mzi-ng-dwivedi-602`](validation.md?plain=1#L306), [`components/mzi-neff-dwivedi-805`](validation.md?plain=1#L307), [`components/mzi-ng-dwivedi-805`](validation.md?plain=1#L308), [`device/wire-ng-dwivedi`](validation.md?plain=1#L327), [`thermo/dwivedi-470`](validation.md?plain=1#L538), [`thermo/dwivedi-602`](validation.md?plain=1#L539), [`thermo/dwivedi-805`](validation.md?plain=1#L540)
- Examples: [README.md:62](../examples/README.md?plain=1#L62), [mzi_dwivedi.rs:8](../examples/mzi_dwivedi.rs#L8)
- Code: [src/cases/mode/thermo_checks.rs:366](../src/cases/mode/thermo_checks.rs#L366), [373](../src/cases/mode/thermo_checks.rs#L373), [380](../src/cases/mode/thermo_checks.rs#L380), [src/circuit/components/measured.rs:6](../src/circuit/components/measured.rs#L6), [src/device/checks.rs:187](../src/device/checks.rs#L187), [src/validation.rs:2146](../src/validation.rs#L2146), [2153](../src/validation.rs#L2153), [2160](../src/validation.rs#L2160), [2167](../src/validation.rs#L2167), [2174](../src/validation.rs#L2174), [2181](../src/validation.rs#L2181)
- Jobs and circuits: [jobs/wire-470nm-corners.yaml:2](../jobs/wire-470nm-corners.yaml#L2), [jobs/wire-602nm-corners.yaml:2](../jobs/wire-602nm-corners.yaml#L2), [jobs/wire-805nm-corners.yaml:2](../jobs/wire-805nm-corners.yaml#L2)
- Other documents: [ROADMAP.md:755](../ROADMAP.md?plain=1#L755)

### E

#### Edwards and Lawrence 1984

G. J. Edwards, M. Lawrence, "A temperature-dependent dispersion equation for congruently grown lithium niobate", Opt Quant Electron 16, 373 (1984). [doi:10.1007/BF00620081](https://doi.org/10.1007/BF00620081)

- Methods pages: [catalogue.md:48](methods/catalogue.md?plain=1#L48), [nonlinear.md:18](methods/nonlinear.md?plain=1#L18)
- Validation cases: [`nonlinear/fejer-table-1-coherence`](validation.md?plain=1#L713), [`nonlinear/fejer-table-1-wavelength`](validation.md?plain=1#L714), [`nonlinear/fejer-table-1-temperature`](validation.md?plain=1#L715), [`nonlinear/fejer-table-1-tilt-about-z`](validation.md?plain=1#L716), [`nonlinear/fejer-table-1-tilt-toward-z`](validation.md?plain=1#L717), [`nonlinear/edwards-lawrence-table-2`](validation.md?plain=1#L718)
- Examples: [python/examples/output/ppln_shg.txt:12](../python/examples/output/ppln_shg.txt#L12)
- Code: [src/material/catalogue/models.rs:685](../src/material/catalogue/models.rs#L685), [src/material/catalogue/references.rs:228](../src/material/catalogue/references.rs#L228), [src/nonlinear/checks.rs:660](../src/nonlinear/checks.rs#L660), [718](../src/nonlinear/checks.rs#L718)

#### Efthimiopoulos et al. 2013

I. Efthimiopoulos, J. Zhang, M. Kucway, C. Park, R. C. Ewing, Y. Wang, "Sb2Se3 under pressure", Sci Rep 3, 2665 (2013). [doi:10.1038/srep02665](https://doi.org/10.1038/srep02665), open access.

- Code: [src/material/catalogue/references.rs:32](../src/material/catalogue/references.rs#L32)

#### Elsawy et al. 2019

M. M. R. Elsawy, S. Lanteri, R. Duvigneau, G. Brière, M. S. Mohamed, P. Genevet, "Global optimization of metasurface designs using statistical learning methods", Sci Rep 9, 17918 (2019). [doi:10.1038/s41598-019-53878-9](https://doi.org/10.1038/s41598-019-53878-9), open access.

- Other documents: [docs/plans/optimization.md:613](plans/optimization.md?plain=1#L613)

#### Elshaari et al. 2016

A. W. Elshaari, I. E. Zadeh, K. D. Jons, V. Zwiller, "Thermo-Optic Characterization of Silicon Nitride Resonators for Cryogenic Photonic Circuits", IEEE Photonics J. 8, 1 (2016). [doi:10.1109/JPHOT.2016.2561622](https://doi.org/10.1109/JPHOT.2016.2561622)

- Methods pages: [catalogue.md:18](methods/catalogue.md?plain=1#L18)
- Validation cases: [`material/silicon-nitride-elshaari-reading`](validation.md?plain=1#L29), [`material/silicon-nitride-elshaari-between`](validation.md?plain=1#L30), [`material/silicon-nitride-room`](validation.md?plain=1#L31)
- Code: [src/material/catalogue/references.rs:137](../src/material/catalogue/references.rs#L137), [138](../src/material/catalogue/references.rs#L138), [src/material/catalogue/thermo.rs:19](../src/material/catalogue/thermo.rs#L19), [483](../src/material/catalogue/thermo.rs#L483), [src/validation.rs:228](../src/validation.rs#L228), [235](../src/validation.rs#L235), [242](../src/validation.rs#L242)

#### Eltes et al. 2019

F. Eltes, C. Mai, D. Caimi, M. Kroh, Y. Popoff, G. Winzer, D. Petousi, S. Lischke, J. E. Ortmann, L. Czornomaz, et al., "A BaTiO 3 -Based Electro-Optic Pockels Modulator Monolithically Integrated on an Advanced Silicon Photonics Platform", J. Lightwave Technol. 37, 1456 (2019). [doi:10.1109/JLT.2019.2893500](https://doi.org/10.1109/JLT.2019.2893500)

- Validation cases: [`material/batio3-abel-films`](validation.md?plain=1#L76), [`pockels/bto-eltes`](validation.md?plain=1#L557)
- Code: [src/cases/mode/pockels/checks.rs:1061](../src/cases/mode/pockels/checks.rs#L1061), [src/material/catalogue/references.rs:487](../src/material/catalogue/references.rs#L487), [src/validation.rs:557](../src/validation.rs#L557)
- Other documents: [ROADMAP.md:789](../ROADMAP.md?plain=1#L789)

#### Engquist and Ying 2011

B. Engquist, L. Ying, "Sweeping Preconditioner for the Helmholtz Equation: Moving Perfectly Matched Layers", Multiscale Model. Simul. 9, 686 (2011). [doi:10.1137/100804644](https://doi.org/10.1137/100804644)

- Methods pages: [fdfd-3d.md:887](methods/fdfd-3d.md?plain=1#L887)
- Other documents: [docs/plans/performance.md:581](plans/performance.md?plain=1#L581)

#### Erdogan 1997

T. Erdogan, "Fiber grating spectra", J. Lightwave Technol. 15, 1277 (1997). [doi:10.1109/50.618322](https://doi.org/10.1109/50.618322)

- Academy lessons: [bragg-gratings.md:79](../academy/bragg-gratings.md?plain=1#L79), [250](../academy/bragg-gratings.md?plain=1#L250), [401](../academy/bragg-gratings.md?plain=1#L401)

#### Erlangga et al. 2006

Y. A. Erlangga, C. W. Oosterlee, C. Vuik, "A Novel Multigrid Based Preconditioner For Heterogeneous Helmholtz Problems", SIAM J. Sci. Comput. 27, 1471 (2006). [doi:10.1137/040615195](https://doi.org/10.1137/040615195)

- Methods pages: [fdfd-3d.md:50](methods/fdfd-3d.md?plain=1#L50), [819](methods/fdfd-3d.md?plain=1#L819), [multigrid-stencil.md:8](methods/multigrid-stencil.md?plain=1#L8)
- Validation cases: [`fdfd3d/gmres-multigrid-direct`](validation.md?plain=1#L180), [`fdfd3d/multigrid-stencil-direct`](validation.md?plain=1#L195), [`fdfd3d/multigrid-stencil-refinement`](validation.md?plain=1#L198), [`fdfd3d/multigrid-stencil-plane-waves`](validation.md?plain=1#L199), [`fdfd3d/multigrid-stencil-smoothing-factors`](validation.md?plain=1#L200)
- Code: [src/fdfd/three/iterative.rs:380](../src/fdfd/three/iterative.rs#L380), [src/fdfd/three/multigrid.rs:8](../src/fdfd/three/multigrid.rs#L8), [src/fdfd/three/stencil_checks.rs:13](../src/fdfd/three/stencil_checks.rs#L13), [src/validation.rs:1285](../src/validation.rs#L1285), [1390](../src/validation.rs#L1390), [1411](../src/validation.rs#L1411), [1418](../src/validation.rs#L1418), [1425](../src/validation.rs#L1425)
- Other documents: [docs/plans/performance.md:582](plans/performance.md?plain=1#L582)

#### Ewert and van Loock 2014

F. Ewert, P. van Loock, "3/4-Efficient Bell Measurement with Passive Linear Optics and Unentangled Ancillae", Phys. Rev. Lett. 113, 140403 (2014). [doi:10.1103/PhysRevLett.113.140403](https://doi.org/10.1103/PhysRevLett.113.140403)

- Other documents: [docs/plans/quantum.md:490](plans/quantum.md?plain=1#L490)

### F

#### Fallahkhair et al. 2008

A. B. Fallahkhair, K. S. Li, T. E. Murphy, "Vector Finite Difference Modesolver for Anisotropic Dielectric Waveguides", J. Lightwave Technol. 26, 1423 (2008). [doi:10.1109/JLT.2008.923643](https://doi.org/10.1109/JLT.2008.923643)

- Methods pages: [devices.md:14](methods/devices.md?plain=1#L14), [fields.md:8](methods/fields.md?plain=1#L8), [slab-fd.md:8](methods/slab-fd.md?plain=1#L8), [stacks.md:10](methods/stacks.md?plain=1#L10), [vector.md:8](methods/vector.md?plain=1#L8)
- Academy lessons: [slab-waveguide.md:78](../academy/slab-waveguide.md?plain=1#L78), [520](../academy/slab-waveguide.md?plain=1#L520), [576](../academy/slab-waveguide.md?plain=1#L576), [strip-waveguide.md:118](../academy/strip-waveguide.md?plain=1#L118), [199](../academy/strip-waveguide.md?plain=1#L199), [628](../academy/strip-waveguide.md?plain=1#L628), [673](../academy/strip-waveguide.md?plain=1#L673), [706](../academy/strip-waveguide.md?plain=1#L706)
- Validation cases: [`device/coupler-3d-order`](validation.md?plain=1#L320), [`job/leaky-film-grid-order`](validation.md?plain=1#L479)
- Code: [src/circuit/components/waveguide.rs:367](../src/circuit/components/waveguide.rs#L367), [src/device/checks.rs:138](../src/device/checks.rs#L138), [src/device/fit.rs:11](../src/device/fit.rs#L11), [src/device/pcm.rs:1004](../src/device/pcm.rs#L1004), [src/device/tfln.rs:1260](../src/device/tfln.rs#L1260), [src/device/thermo.rs:772](../src/device/thermo.rs#L772), [src/device/waveguide.rs:33](../src/device/waveguide.rs#L33), [src/job/leaky_checks.rs:43](../src/job/leaky_checks.rs#L43), [src/mode/slab_fd.rs:16](../src/mode/slab_fd.rs#L16), [src/mode/vector.rs:5](../src/mode/vector.rs#L5)
- Other documents: [ROADMAP.md:726](../ROADMAP.md?plain=1#L726)

#### Farjadpour et al. 2006

A. Farjadpour, D. Roundy, A. Rodriguez, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, G. W. Burr, "Improving accuracy by subpixel smoothing in the finite-difference time domain", Opt. Lett. 31, 2972 (2006). [doi:10.1364/OL.31.002972](https://doi.org/10.1364/OL.31.002972)

- Methods pages: [averaging.md:8](methods/averaging.md?plain=1#L8), [bodies.md:8](methods/bodies.md?plain=1#L8), [fdtd.md:30](methods/fdtd.md?plain=1#L30), [stacks.md:12](methods/stacks.md?plain=1#L12), [static.md:14](methods/static.md?plain=1#L14)
- Validation cases: [`fdtd/smoothing-slab`](validation.md?plain=1#L227), [`fdtd/ring-order`](validation.md?plain=1#L262), [`device/mmi-2d-grid`](validation.md?plain=1#L337), [`averaging/film-integrals`](validation.md?plain=1#L490), [`fdfd/fill-slab-te-mean`](validation.md?plain=1#L493), [`fdfd/fill-crystal-te-tensor`](validation.md?plain=1#L500)
- Examples: [README.md:66](../examples/README.md?plain=1#L66), [subpixel_holes.rs:6](../examples/subpixel_holes.rs#L6)
- Code: [src/averaging.rs:15](../src/averaging.rs#L15), [src/averaging/checks.rs:150](../src/averaging/checks.rs#L150), [src/cases/fdfd/fill_checks.rs:3](../src/cases/fdfd/fill_checks.rs#L3), [535](../src/cases/fdfd/fill_checks.rs#L535), [626](../src/cases/fdfd/fill_checks.rs#L626), [src/device/splitter_checks.rs:117](../src/device/splitter_checks.rs#L117), [src/fdfd/fill.rs:4](../src/fdfd/fill.rs#L4), [src/fdfd/three/fill.rs:6](../src/fdfd/three/fill.rs#L6), [src/fdtd/smoothing.rs:5](../src/fdtd/smoothing.rs#L5), [src/stack.rs:414](../src/stack.rs#L414), [src/statics.rs:25](../src/statics.rs#L25), [src/validation.rs:1614](../src/validation.rs#L1614), [1859](../src/validation.rs#L1859)
- Other documents: [ROADMAP.md:718](../ROADMAP.md?plain=1#L718)

#### Fejer et al. 1992

M. Fejer, G. Magel, D. Jundt, R. Byer, "Quasi-phase-matched second harmonic generation: tuning and tolerances", IEEE J. Quantum Electron. 28, 2631 (1992). [doi:10.1109/3.161322](https://doi.org/10.1109/3.161322)

- Methods pages: [nonlinear.md:8](methods/nonlinear.md?plain=1#L8)
- Validation cases: [`nonlinear/sinc2-half-maximum`](validation.md?plain=1#L711), [`nonlinear/fejer-table-1-coherence`](validation.md?plain=1#L713), [`nonlinear/fejer-table-1-wavelength`](validation.md?plain=1#L714), [`nonlinear/fejer-table-1-temperature`](validation.md?plain=1#L715), [`nonlinear/fejer-table-1-tilt-about-z`](validation.md?plain=1#L716), [`nonlinear/fejer-table-1-tilt-toward-z`](validation.md?plain=1#L717)
- Examples: [README.md:84](../examples/README.md?plain=1#L84), [qpm_fejer.rs:6](../examples/qpm_fejer.rs#L6), [python/examples/README.md:30](../python/examples/README.md?plain=1#L30), [python/examples/ppln_shg.py:6](../python/examples/ppln_shg.py#L6)
- Code: [src/nonlinear/checks.rs:660](../src/nonlinear/checks.rs#L660), [669](../src/nonlinear/checks.rs#L669), [src/nonlinear/phase.rs:50](../src/nonlinear/phase.rs#L50)
- Other documents: [ROADMAP.md:845](../ROADMAP.md?plain=1#L845)

#### Ferrini et al. 2002

R. Ferrini, G. Guizzetti, M. Patrini, A. Parisini, L. Tarricone, B. Valenti, "Optical functions of InGaP/GaAs epitaxial layers from 0.01 to 5.5 eV", The European Physical Journal B - Condensed Matter 27, 449 (2002). [doi:10.1140/epjb/e2002-00177-x](https://doi.org/10.1140/epjb/e2002-00177-x)

- Methods pages: [catalogue.md:96](methods/catalogue.md?plain=1#L96)
- Validation cases: [`material/ingap-ferrini-table-consistency`](validation.md?plain=1#L92), [`material/ingap-ferrini-table-knots`](validation.md?plain=1#L93)
- Code: [src/material/catalogue/models.rs:1102](../src/material/catalogue/models.rs#L1102), [src/material/catalogue/references.rs:403](../src/material/catalogue/references.rs#L403), [src/validation.rs:669](../src/validation.rs#L669), [676](../src/validation.rs#L676)

#### Fiers et al. 2012

M. Fiers, T. Van Vaerenbergh, K. Caluwaerts, D. Vande Ginste, B. Schrauwen, J. Dambre, P. Bienstman, "Time-domain and frequency-domain modeling of nonlinear optical components at the circuit-level using a node-based approach", J. Opt. Soc. Am. B 29, 896 (2012). [doi:10.1364/JOSAB.29.000896](https://doi.org/10.1364/JOSAB.29.000896)

- Other documents: [ROADMAP.md:818](../ROADMAP.md?plain=1#L818)

#### Filipsson 1981

G. Filipsson, "A New General Computer Algorithm for S-Matrix Calculation of Interconnected Multiports", 11th European Microwave Conference, 1981 (1981). [doi:10.1109/EUMA.1981.332972](https://doi.org/10.1109/EUMA.1981.332972)

- Methods pages: [circuits.md:8](methods/circuits.md?plain=1#L8)
- Validation cases: [`circuit/sub-network-growth`](validation.md?plain=1#L280)
- Code: [src/circuit/solve.rs:19](../src/circuit/solve.rs#L19), [src/validation.rs:1985](../src/validation.rs#L1985)
- Other documents: [ROADMAP.md:819](../ROADMAP.md?plain=1#L819), [docs/design/components.md:105](design/components.md?plain=1#L105)

#### Fischer 1998

P. F. Fischer, "Projection techniques for iterative solution of with successive right-hand sides", Computer Methods in Applied Mechanics and Engineering 163, 193 (1998). [doi:10.1016/S0045-7825(98)00012-7](https://doi.org/10.1016/S0045-7825(98)00012-7)

- Methods pages: [recycling.md:10](methods/recycling.md?plain=1#L10), [58](methods/recycling.md?plain=1#L58)
- Validation cases: [`fdfd3d/recycle-s-matrix`](validation.md?plain=1#L188), [`fdfd3d/recycle-convergence`](validation.md?plain=1#L190)
- Code: [src/backend/krylov/recycle.rs:30](../src/backend/krylov/recycle.rs#L30), [181](../src/backend/krylov/recycle.rs#L181), [src/fdfd/three/recycling.rs:21](../src/fdfd/three/recycling.rs#L21), [src/validation.rs:1341](../src/validation.rs#L1341), [1355](../src/validation.rs#L1355)
- Other documents: [docs/plans/performance.md:608](plans/performance.md?plain=1#L608)

#### Foster et al. 2006

M. A. Foster, A. C. Turner, J. E. Sharping, B. S. Schmidt, M. Lipson, A. L. Gaeta, "Broad-band optical parametric gain on a silicon photonic chip", Nature 441, 960 (2006). [doi:10.1038/nature04932](https://doi.org/10.1038/nature04932)

- Other documents: [ROADMAP.md:852](../ROADMAP.md?plain=1#L852)

#### Frankel et al. 1991

M. Frankel, S. Gupta, J. Valdmanis, G. Mourou, "Terahertz attenuation and dispersion characteristics of coplanar transmission lines", IEEE Trans. Microwave Theory Techn. 39, 910 (1991). [doi:10.1109/22.81658](https://doi.org/10.1109/22.81658)

- Methods pages: [travelling-wave.md:30](methods/travelling-wave.md?plain=1#L30)
- Validation cases: [`line/frankel-1991-radiation-100ghz`](validation.md?plain=1#L611)
- Code: [src/electro/full_wave.rs:46](../src/electro/full_wave.rs#L46), [src/electro/full_wave_coplanar.rs:644](../src/electro/full_wave_coplanar.rs#L644)

#### Franken et al. 1961

P. A. Franken, A. E. Hill, C. W. Peters, G. Weinreich, "Generation of Optical Harmonics", Phys. Rev. Lett. 7, 118 (1961). [doi:10.1103/PhysRevLett.7.118](https://doi.org/10.1103/PhysRevLett.7.118)

- Other documents: [ROADMAP.md:841](../ROADMAP.md?plain=1#L841)

#### Franssila 2010

S. Franssila, "Introduction to Microfabrication", Wiley (2010). [doi:10.1002/9781119990413](https://doi.org/10.1002/9781119990413)

- Methods pages: [fabrication.md:32](methods/fabrication.md?plain=1#L32)
- Validation cases: [`fabrication/rayleigh-franssila`](validation.md?plain=1#L749), [`fabrication/rule-choices`](validation.md?plain=1#L752)
- Code: [src/fabrication/references.rs:280](../src/fabrication/references.rs#L280), [src/fabrication/rule_checks.rs:323](../src/fabrication/rule_checks.rs#L323), [344](../src/fabrication/rule_checks.rs#L344)
- Other documents: [docs/plans/fabrication.md:619](plans/fabrication.md?plain=1#L619)

#### Frantz et al. 2023

J. A. Frantz, J. D. Myers, A. Clabeau, R. Y. Bekele, N. Hong, M. A. Vincenti, M. Gandolfi, J. S. Sanghera, "Optical constants of germanium antimony telluride (GST) in amorphous, crystalline, and intermediate states", Opt. Mater. Express 13, 3631 (2023). [doi:10.1364/OME.506019](https://doi.org/10.1364/OME.506019)

- Methods pages: [catalogue.md:361](methods/catalogue.md?plain=1#L361), [devices.md:46](methods/devices.md?plain=1#L46)
- Validation cases: [`device/pcm-frantz-mix`](validation.md?plain=1#L665)
- Code: [src/cases/device/pcm_checks.rs:454](../src/cases/device/pcm_checks.rs#L454), [src/device/pcm.rs:261](../src/device/pcm.rs#L261), [1007](../src/device/pcm.rs#L1007), [src/material/catalogue/phase_change.rs:13](../src/material/catalogue/phase_change.rs#L13), [245](../src/material/catalogue/phase_change.rs#L245), [762](../src/material/catalogue/phase_change.rs#L762), [src/material/catalogue/references.rs:81](../src/material/catalogue/references.rs#L81), [82](../src/material/catalogue/references.rs#L82)

#### Frellsen et al. 2016

L. F. Frellsen, Y. Ding, O. Sigmund, L. H. Frandsen, "Topology optimized mode multiplexing in silicon-on-insulator photonic wire waveguides", Opt. Express 24, 16866 (2016). [doi:10.1364/OE.24.016866](https://doi.org/10.1364/OE.24.016866)

- Other documents: [ROADMAP.md:765](../ROADMAP.md?plain=1#L765)

#### Freund 1992

R. W. Freund, "Conjugate Gradient-Type Methods for Linear Systems with Complex Symmetric Coefficient Matrices", SIAM J. Sci. and Stat. Comput. 13, 425 (1992). [doi:10.1137/0913023](https://doi.org/10.1137/0913023)

- Methods pages: [backends.md:344](methods/backends.md?plain=1#L344), [block-qmr.md:14](methods/block-qmr.md?plain=1#L14), [fdfd-3d.md:20](methods/fdfd-3d.md?plain=1#L20), [417](methods/fdfd-3d.md?plain=1#L417)
- Validation cases: [`fdfd3d/qmr-symmetric-direct`](validation.md?plain=1#L173), [`fdfd3d/block-qmr-s-matrix`](validation.md?plain=1#L181)
- Code: [src/backend/iterative.rs:13](../src/backend/iterative.rs#L13), [src/backend/krylov.rs:880](../src/backend/krylov.rs#L880), [src/validation.rs:1236](../src/validation.rs#L1236), [1292](../src/validation.rs#L1292)
- Other documents: [docs/plans/performance.md:609](plans/performance.md?plain=1#L609)

#### Freund and Malhotra 1997

R. W. Freund, M. Malhotra, "A block QMR algorithm for non-Hermitian linear systems with multiple right-hand sides", Linear Algebra and its Applications 254, 119 (1997). [doi:10.1016/S0024-3795(96)00529-0](https://doi.org/10.1016/S0024-3795(96)00529-0)

- Methods pages: [block-qmr.md:8](methods/block-qmr.md?plain=1#L8), [49](methods/block-qmr.md?plain=1#L49)
- Validation cases: [`fdfd3d/block-qmr-s-matrix`](validation.md?plain=1#L181), [`fdfd3d/block-qmr-ilu-s-matrix`](validation.md?plain=1#L182), [`fdfd3d/block-qmr-iterations`](validation.md?plain=1#L183), [`fdfd3d/block-qmr-dependent`](validation.md?plain=1#L184), [`fdfd3d/block-qmr-freund-malhotra`](validation.md?plain=1#L186)
- Code: [src/backend/krylov/block.rs:6](../src/backend/krylov/block.rs#L6), [src/backend/krylov/example.rs:10](../src/backend/krylov/example.rs#L10), [src/fdfd/three/iterative.rs:679](../src/fdfd/three/iterative.rs#L679), [src/validation.rs:1292](../src/validation.rs#L1292), [1299](../src/validation.rs#L1299), [1306](../src/validation.rs#L1306), [1313](../src/validation.rs#L1313), [1327](../src/validation.rs#L1327)
- Other documents: [docs/plans/performance.md:610](plans/performance.md?plain=1#L610)

#### Freund and Nachtigal 1991

R. W. Freund, N. M. Nachtigal, "QMR: a quasi-minimal residual method for non-Hermitian linear systems", Numer. Math. 60, 315 (1991). [doi:10.1007/BF01385726](https://doi.org/10.1007/BF01385726)

- Methods pages: [backends.md:346](methods/backends.md?plain=1#L346), [block-qmr.md:12](methods/block-qmr.md?plain=1#L12), [fdfd-3d.md:18](methods/fdfd-3d.md?plain=1#L18)
- Validation cases: [`fdfd3d/qmr-direct`](validation.md?plain=1#L172), [`fdfd3d/block-qmr-ilu-s-matrix`](validation.md?plain=1#L182)
- Code: [src/backend/iterative.rs:12](../src/backend/iterative.rs#L12), [src/backend/krylov.rs:4](../src/backend/krylov.rs#L4), [src/fdfd/three/iterative.rs:106](../src/fdfd/three/iterative.rs#L106), [src/validation.rs:1229](../src/validation.rs#L1229), [1299](../src/validation.rs#L1299)

#### Frey et al. 2006

B. J. Frey, D. B. Leviton, T. J. Madison, "Temperature-dependent refractive index of silicon and germanium", SPIE Proceedings (2006). [doi:10.1117/12.672850](https://doi.org/10.1117/12.672850)

- Methods pages: [catalogue.md:12](methods/catalogue.md?plain=1#L12), [materials.md:63](methods/materials.md?plain=1#L63)
- Academy lessons: [material-dispersion.md:56](../academy/material-dispersion.md?plain=1#L56), [302](../academy/material-dispersion.md?plain=1#L302), [332](../academy/material-dispersion.md?plain=1#L332), [448](../academy/material-dispersion.md?plain=1#L448)
- Validation cases: [`material/silicon-frey-table`](validation.md?plain=1#L13), [`material/silicon-frey-li`](validation.md?plain=1#L14), [`material/silicon-frey-dndt`](validation.md?plain=1#L23), [`material/silicon-li-frey-dndt`](validation.md?plain=1#L24), [`material/silicon-komma-reading`](validation.md?plain=1#L25), [`material/silicon-komma-frey-dndt`](validation.md?plain=1#L26)
- Code: [src/material/catalogue/models.rs:277](../src/material/catalogue/models.rs#L277), [src/material/catalogue/references.rs:95](../src/material/catalogue/references.rs#L95), [src/material/catalogue/thermo.rs:7](../src/material/catalogue/thermo.rs#L7), [247](../src/material/catalogue/thermo.rs#L247), [261](../src/material/catalogue/thermo.rs#L261), [src/validation.rs:116](../src/validation.rs#L116), [123](../src/validation.rs#L123), [186](../src/validation.rs#L186), [193](../src/validation.rs#L193), [200](../src/validation.rs#L200), [207](../src/validation.rs#L207), [studio/src-tauri/src/charts/material.rs:629](../studio/src-tauri/src/charts/material.rs#L629)
- Other documents: [ROADMAP.md:802](../ROADMAP.md?plain=1#L802)

#### Friedrich et al. 2000

I. Friedrich, V. Weidenhof, W. Njoroge, P. Franz, M. Wuttig, "Structural transformations of Ge2Sb2Te5 films studied by electrical resistance measurements", Journal of Applied Physics 87, 4130 (2000). [doi:10.1063/1.373041](https://doi.org/10.1063/1.373041)

- Validation cases: [`fabrication/pcm-rules`](validation.md?plain=1#L759)
- Code: [src/fabrication/platform_checks.rs:742](../src/fabrication/platform_checks.rs#L742), [src/fabrication/references.rs:224](../src/fabrication/references.rs#L224), [src/material/catalogue/references.rs:74](../src/material/catalogue/references.rs#L74)

#### Frommer and Glässner 1998

A. Frommer, U. Glässner, "Restarted GMRES for Shifted Linear Systems", SIAM J. Sci. Comput. 19, 15 (1998). [doi:10.1137/S1064827596304563](https://doi.org/10.1137/S1064827596304563)

- Other documents: [docs/plans/performance.md:611](plans/performance.md?plain=1#L611)

#### Fujisawa et al. 2017

T. Fujisawa, S. Makino, T. Sato, K. Saitoh, "Low-loss, compact, and fabrication-tolerant Si-wire 90° waveguide bend using clothoid and normal curves for large scale photonic integrated circuits", Opt. Express 25, 9150 (2017). [doi:10.1364/OE.25.009150](https://doi.org/10.1364/OE.25.009150)

- Methods pages: [geometry.md:18](methods/geometry.md?plain=1#L18)
- Validation cases: [`geometry/paths/euler-curvature`](validation.md?plain=1#L399), [`geometry/paths/fujisawa-ratios`](validation.md?plain=1#L410)
- Code: [src/geometry/path_checks.rs:383](../src/geometry/path_checks.rs#L383), [470](../src/geometry/path_checks.rs#L470)

#### Fukui et al. 2018

K. Fukui, A. Tomita, A. Okamoto, K. Fujii, "High-Threshold Fault-Tolerant Quantum Computation with Analog Quantum Error Correction", Phys. Rev. X 8, 021054 (2018). [doi:10.1103/PhysRevX.8.021054](https://doi.org/10.1103/PhysRevX.8.021054), open access.

- Other documents: [docs/plans/quantum.md:516](plans/quantum.md?plain=1#L516)

### G

#### Gander and Zhang 2019

M. J. Gander, H. Zhang, "A Class of Iterative Solvers for the Helmholtz Equation: Factorizations, Sweeping Preconditioners, Source Transfer, Single Layer Potentials, Polarized Traces, and Optimized Schwarz Methods", SIAM Rev. 61, 3 (2019). [doi:10.1137/16M109781X](https://doi.org/10.1137/16M109781X)

- Other documents: [docs/plans/performance.md:613](plans/performance.md?plain=1#L613)

#### Gander et al. 2002

M. J. Gander, F. Magoulès, F. Nataf, "Optimized Schwarz Methods without Overlap for the Helmholtz Equation", SIAM J. Sci. Comput. 24, 38 (2002). [doi:10.1137/S1064827501387012](https://doi.org/10.1137/S1064827501387012)

- Other documents: [docs/plans/performance.md:612](plans/performance.md?plain=1#L612)

#### Gao et al. 2023

Z. Gao, X. Chen, Z. Zhang, U. Chakraborty, W. Bogaerts, D. S. Boning, "Automatic synthesis of light-processing functions for programmable photonics: theory and realization", Photon. Res. 11, 643 (2023). [doi:10.1364/PRJ.474606](https://doi.org/10.1364/PRJ.474606)

- Other documents: [docs/plans/optimization.md:624](plans/optimization.md?plain=1#L624)

#### Gayer et al. 2008

O. Gayer, Z. Sacks, E. Galun, A. Arie, "Temperature and wavelength dependent refractive index equations for MgO-doped congruent and stoichiometric LiNbO3", Appl. Phys. B 91, 343 (2008). [doi:10.1007/s00340-008-2998-2](https://doi.org/10.1007/s00340-008-2998-2)

- Methods pages: [catalogue.md:50](methods/catalogue.md?plain=1#L50)
- Validation cases: [`material/linbo3-mgo-gayer-zelmon`](validation.md?plain=1#L68)
- Code: [src/material/catalogue/models.rs:734](../src/material/catalogue/models.rs#L734), [src/material/catalogue/references.rs:242](../src/material/catalogue/references.rs#L242), [src/validation.rs:501](../src/validation.rs#L501)
- Other documents: [ROADMAP.md:854](../ROADMAP.md?plain=1#L854)

#### Gayer et al. 2010

O. Gayer, Z. Sacks, E. Galun, A. Arie, "Erratum to: Temperature and wavelength dependent refractive index equations for MgO-doped congruent and stoichiometric LiNbO3", Appl. Phys. B 101, 481 (2010). [doi:10.1007/s00340-010-4203-7](https://doi.org/10.1007/s00340-010-4203-7)

- Code: [src/material/catalogue/references.rs:240](../src/material/catalogue/references.rs#L240)

#### Gehrsitz et al. 2000

S. Gehrsitz, F. K. Reinhart, C. Gourgon, N. Herres, A. Vonlanthen, H. Sigg, "The refractive index of AlxGa1−xAs below the band gap: Accurate determination and empirical modeling", Journal of Applied Physics 87, 7825 (2000). [doi:10.1063/1.373462](https://doi.org/10.1063/1.373462)

- Methods pages: [catalogue.md:64](methods/catalogue.md?plain=1#L64)
- Validation cases: [`material/gaas-gehrsitz-gap`](validation.md?plain=1#L78), [`material/gaas-gehrsitz-n-inf`](validation.md?plain=1#L79), [`material/algaas-gehrsitz-samples`](validation.md?plain=1#L80)
- Code: [src/material/catalogue/models.rs:476](../src/material/catalogue/models.rs#L476), [src/material/catalogue/references.rs:249](../src/material/catalogue/references.rs#L249), [src/validation.rs:571](../src/validation.rs#L571), [578](../src/validation.rs#L578), [585](../src/validation.rs#L585)
- Other documents: [ROADMAP.md:849](../ROADMAP.md?plain=1#L849)

#### George 1973

A. George, "Nested Dissection of a Regular Finite Element Mesh", SIAM J. Numer. Anal. 10, 345 (1973). [doi:10.1137/0710032](https://doi.org/10.1137/0710032)

- Methods pages: [fdfd-3d.md:30](methods/fdfd-3d.md?plain=1#L30), [340](methods/fdfd-3d.md?plain=1#L340), [fdfd.md:16](methods/fdfd.md?plain=1#L16), [static.md:20](methods/static.md?plain=1#L20)
- Code: [src/sparse.rs:4](../src/sparse.rs#L4)
- Other documents: [docs/plans/performance.md:614](plans/performance.md?plain=1#L614)

#### George 2009

S. M. George, "Atomic Layer Deposition: An Overview", Chem. Rev. 110, 111 (2009). [doi:10.1021/cr900056b](https://doi.org/10.1021/cr900056b)

- Other documents: [docs/plans/fabrication.md:625](plans/fabrication.md?plain=1#L625)

#### George and Ng 1987

A. George, E. Ng, "Symbolic Factorization for Sparse Gaussian Elimination with Partial Pivoting", SIAM J. Sci. and Stat. Comput. 8, 877 (1987). [doi:10.1137/0908072](https://doi.org/10.1137/0908072)

- Methods pages: [fdfd-3d.md:40](methods/fdfd-3d.md?plain=1#L40), [381](methods/fdfd-3d.md?plain=1#L381)
- Other documents: [docs/baselines.md:326](baselines.md?plain=1#L326)

#### Ghione 2009

G. Ghione, "Semiconductor Devices for High-Speed Optoelectronics", Cambridge University Press (2009). [doi:10.1017/CBO9780511635595](https://doi.org/10.1017/CBO9780511635595)

- Methods pages: [devices.md:60](methods/devices.md?plain=1#L60), [travelling-wave.md:14](methods/travelling-wave.md?plain=1#L14)
- Validation cases: [`line/response-walk-off`](validation.md?plain=1#L569)
- Code: [src/cases/electro/line_checks.rs:979](../src/cases/electro/line_checks.rs#L979), [src/device/tfln.rs:1206](../src/device/tfln.rs#L1206), [src/electro/line.rs:53](../src/electro/line.rs#L53)
- Other documents: [ROADMAP.md:793](../ROADMAP.md?plain=1#L793)

#### Ghione and Naldi 1984

G. Ghione, C. Naldi, "Analytical formulas for coplanar lines in hybrid and monolithic MICs", Electron. Lett. (UK) 20, 179 (1984). [doi:10.1049/el:19840120](https://doi.org/10.1049/el:19840120)

- Methods pages: [devices.md:56](methods/devices.md?plain=1#L56), [static.md:18](methods/static.md?plain=1#L18), [travelling-wave.md:12](methods/travelling-wave.md?plain=1#L12)
- Validation cases: [`static/coplanar-wen`](validation.md?plain=1#L522), [`line/ghione-naldi-thick`](validation.md?plain=1#L563), [`device/tfln-analytic-line`](validation.md?plain=1#L686)
- Code: [crosscode/src/cpw.rs:6](../crosscode/src/cpw.rs#L6), [crosscode/tests/openems.rs:239](../crosscode/tests/openems.rs#L239), [src/cases/device/tfln_checks.rs:306](../src/cases/device/tfln_checks.rs#L306), [src/cases/electro/line_checks.rs:937](../src/cases/electro/line_checks.rs#L937), [src/cases/statics/checks.rs:938](../src/cases/statics/checks.rs#L938), [src/device/tfln.rs:1201](../src/device/tfln.rs#L1201)
- Other documents: [ROADMAP.md:794](../ROADMAP.md?plain=1#L794), [docs/validation-openems.md:9](validation-openems.md?plain=1#L9)

#### Ghysels and Vanroose 2014

P. Ghysels, W. Vanroose, "Hiding global synchronization latency in the preconditioned Conjugate Gradient algorithm", Parallel Computing 40, 224 (2014). [doi:10.1016/j.parco.2013.06.001](https://doi.org/10.1016/j.parco.2013.06.001)

- Other documents: [docs/plans/performance.md:616](plans/performance.md?plain=1#L616)

#### Ghysels et al. 2016

P. Ghysels, X. S. Li, F.-H. Rouet, S. Williams, A. Napov, "An Efficient Multicore Implementation of a Novel HSS-Structured Multifrontal Solver Using Randomized Sampling", SIAM J. Sci. Comput. 38, S358 (2016). [doi:10.1137/15M1010117](https://doi.org/10.1137/15M1010117)

- Other documents: [docs/plans/performance.md:615](plans/performance.md?plain=1#L615)

#### Glassbrenner and Slack 1964

C. J. Glassbrenner, G. A. Slack, "Thermal Conductivity of Silicon and Germanium from 3°K to the Melting Point", Phys. Rev. 134, A1058 (1964). [doi:10.1103/PhysRev.134.A1058](https://doi.org/10.1103/PhysRev.134.A1058)

- Methods pages: [catalogue.md:106](methods/catalogue.md?plain=1#L106)
- Validation cases: [`material/silicon-glassbrenner-table`](validation.md?plain=1#L35), [`material/silicon-glassbrenner-ho`](validation.md?plain=1#L36)
- Code: [src/material/catalogue/conductivity.rs:8](../src/material/catalogue/conductivity.rs#L8), [src/material/catalogue/references.rs:417](../src/material/catalogue/references.rs#L417), [src/validation.rs:270](../src/validation.rs#L270), [277](../src/validation.rs#L277)

#### Glick et al. 1988

M. Glick, F. K. Reinhart, D. Martin, "Linear electro-optic effect: Comparison of GaAs/AlGaAs multi-quantum-well heterostructures with an AlGaAs solid solution at 1.1523 μm", Journal of Applied Physics 63, 5877 (1988). [doi:10.1063/1.340279](https://doi.org/10.1063/1.340279)

- Code: [src/material/catalogue/references.rs:186](../src/material/catalogue/references.rs#L186), [src/material/catalogue/tests.rs:576](../src/material/catalogue/tests.rs#L576)

#### Glynn 2010

D. G. Glynn, "The permanent of a square matrix", European Journal of Combinatorics 31, 1887 (2010). [doi:10.1016/j.ejc.2010.01.010](https://doi.org/10.1016/j.ejc.2010.01.010)

- Other documents: [docs/plans/quantum.md:498](plans/quantum.md?plain=1#L498)

#### Goel et al. 2026

P. Goel, C. Kauppinen, R. Raju, I. Tittonen, "Thermoelectric and electronic transport properties of thermal and plasma-enhanced ALD grown titanium nitride thin films", Nanoscale Adv. 8, 612 (2026). [doi:10.1039/d5na00914f](https://doi.org/10.1039/d5na00914f), open access.

- Methods pages: [catalogue.md:108](methods/catalogue.md?plain=1#L108)
- Validation cases: [`material/tin-goel-zt`](validation.md?plain=1#L37)
- Code: [src/material/catalogue/conductivity.rs:17](../src/material/catalogue/conductivity.rs#L17), [src/material/catalogue/references.rs:445](../src/material/catalogue/references.rs#L445), [446](../src/material/catalogue/references.rs#L446), [src/validation.rs:284](../src/validation.rs#L284)

#### Goell 1969

J. E. Goell, "A Circular-Harmonic Computer Analysis of Rectangular Dielectric Waveguides", Bell System Technical Journal 48, 2133 (1969). [doi:10.1002/j.1538-7305.1969.tb01168.x](https://doi.org/10.1002/j.1538-7305.1969.tb01168.x)

- Academy lessons: [strip-waveguide.md:52](../academy/strip-waveguide.md?plain=1#L52), [365](../academy/strip-waveguide.md?plain=1#L365), [668](../academy/strip-waveguide.md?plain=1#L668)

#### Goldsmith and Salmon 1987

J. Goldsmith, J. Salmon, "Automatic Creation of Object Hierarchies for Ray Tracing", IEEE Comput. Grap. Appl. 7, 14 (1987). [doi:10.1109/MCG.1987.276983](https://doi.org/10.1109/MCG.1987.276983)

- Methods pages: [geometry.md:14](methods/geometry.md?plain=1#L14)

#### Golub 2012

G. Golub, "Matrix Computations", Johns Hopkins University Press (2012). [doi:10.56021/9781421407944](https://doi.org/10.56021/9781421407944)

- Methods pages: [eigen.md:14](methods/eigen.md?plain=1#L14)
- Code: [src/eigen/schur.rs:8](../src/eigen/schur.rs#L8)

#### Goos and Hänchen 1947

F. Goos, H. Hänchen, "Ein neuer und fundamentaler Versuch zur Totalreflexion", Annalen der Physik 436, 333 (1947). [doi:10.1002/andp.19474360704](https://doi.org/10.1002/andp.19474360704)

- Academy lessons: [slab-waveguide.md:24](../academy/slab-waveguide.md?plain=1#L24), [153](../academy/slab-waveguide.md?plain=1#L153), [561](../academy/slab-waveguide.md?plain=1#L561)

#### Gostimirovic et al. 2022

D. Gostimirovic, D.-X. Xu, O. Liboiron-Ladouceur, Y. Grinberg, "Deep Learning-Based Prediction of Fabrication-Process-Induced Structural Variations in Nanophotonic Devices", ACS Photonics 9, 2623 (2022). [doi:10.1021/acsphotonics.1c01973](https://doi.org/10.1021/acsphotonics.1c01973), open access.

- Other documents: [ROADMAP.md:778](../ROADMAP.md?plain=1#L778)

#### Gottesman et al. 2001

D. Gottesman, A. Kitaev, J. Preskill, "Encoding a qubit in an oscillator", Phys. Rev. A 64, 012310 (2001). [doi:10.1103/PhysRevA.64.012310](https://doi.org/10.1103/PhysRevA.64.012310)

- Other documents: [docs/plans/quantum.md:512](plans/quantum.md?plain=1#L512)

#### Gräupner et al. 1992

P. Gräupner, J. C. Pommier, A. Cachard, J. L. Coutaz, "Electro-optical effect in aluminum nitride waveguides", Journal of Applied Physics 71, 4136 (1992). [doi:10.1063/1.350844](https://doi.org/10.1063/1.350844)

- Methods pages: [catalogue.md:90](methods/catalogue.md?plain=1#L90)
- Code: [src/material/catalogue/references.rs:382](../src/material/catalogue/references.rs#L382)

#### Grice and Walmsley 1997

W. P. Grice, I. A. Walmsley, "Spectral information and distinguishability in type-II down-conversion with a broadband pump", Phys. Rev. A 56, 1627 (1997). [doi:10.1103/PhysRevA.56.1627](https://doi.org/10.1103/PhysRevA.56.1627)

- Methods pages: [nonlinear.md:26](methods/nonlinear.md?plain=1#L26)
- Validation cases: [`nonlinear/pairs-gaussian-purity`](validation.md?plain=1#L727)
- Code: [src/nonlinear/checks.rs:781](../src/nonlinear/checks.rs#L781), [src/nonlinear/pairs.rs:25](../src/nonlinear/pairs.rs#L25)
- Other documents: [ROADMAP.md:847](../ROADMAP.md?plain=1#L847), [docs/plans/quantum.md:480](plans/quantum.md?plain=1#L480)

#### Gupt et al. 2019

B. Gupt, J. Izaac, N. Quesada, "The Walrus: a library for the calculation of hafnians, Hermite polynomials and Gaussian boson sampling", JOSS 4, 1705 (2019). [doi:10.21105/joss.01705](https://doi.org/10.21105/joss.01705), open access.

- Other documents: [docs/plans/quantum.md:543](plans/quantum.md?plain=1#L543)

#### Gurses et al. 2022

B. V. Gurses, R. Fatemi, A. Khachaturian, A. Hajimiri, "Large-Scale Crosstalk-Corrected Thermo-Optic Phase Shifter Arrays in Silicon Photonics", IEEE J. Select. Topics Quantum Electron. 28, 1 (2022). [doi:10.1109/JSTQE.2022.3189965](https://doi.org/10.1109/JSTQE.2022.3189965)

- Methods pages: [thermal-compact.md:12](methods/thermal-compact.md?plain=1#L12), [151](methods/thermal-compact.md?plain=1#L151)
- Validation cases: [`correction/gurses-cutoff`](validation.md?plain=1#L640), [`correction/gurses-neighbour`](validation.md?plain=1#L641), [`correction/gurses-array`](validation.md?plain=1#L642)
- Code: [src/job/optimize/crosstalk.rs:379](../src/job/optimize/crosstalk.rs#L379), [src/thermal/correction.rs:4](../src/thermal/correction.rs#L4), [src/thermal/correction_checks.rs:9](../src/thermal/correction_checks.rs#L9), [104](../src/thermal/correction_checks.rs#L104), [118](../src/thermal/correction_checks.rs#L118), [131](../src/thermal/correction_checks.rs#L131)
- Other documents: [ROADMAP.md:805](../ROADMAP.md?plain=1#L805)

#### Gustavsen and Semlyen 1999

B. Gustavsen, A. Semlyen, "Rational approximation of frequency domain responses by vector fitting", IEEE Trans. Power Delivery 14, 1052 (1999). [doi:10.1109/61.772353](https://doi.org/10.1109/61.772353)

- Methods pages: [compact.md:8](methods/compact.md?plain=1#L8)
- Academy lessons: [ring-resonator.md:37](../academy/ring-resonator.md?plain=1#L37), [487](../academy/ring-resonator.md?plain=1#L487)
- Validation cases: [`compact/vf-paper-rms`](validation.md?plain=1#L376)
- Code: [src/cases/compact/checks.rs:705](../src/cases/compact/checks.rs#L705), [src/compact/fit.rs:6](../src/compact/fit.rs#L6), [src/compact/mod.rs:4](../src/compact/mod.rs#L4), [src/compact/model.rs:269](../src/compact/model.rs#L269), [src/compact/model/tests.rs:76](../src/compact/model/tests.rs#L76), [src/compact/param.rs:14](../src/compact/param.rs#L14), [1369](../src/compact/param.rs#L1369), [src/device/compact.rs:703](../src/device/compact.rs#L703)
- Other documents: [ROADMAP.md:815](../ROADMAP.md?plain=1#L815)

#### Güttel and Tisseur 2017

S. Güttel, F. Tisseur, "The nonlinear eigenvalue problem", Acta Numerica 26, 1 (2017). [doi:10.1017/S0962492917000034](https://doi.org/10.1017/S0962492917000034)

- Methods pages: [hadley.md:12](methods/hadley.md?plain=1#L12)
- Code: [src/mode/hadley.rs:32](../src/mode/hadley.rs#L32)
- Other documents: [docs/plans/performance.md:618](plans/performance.md?plain=1#L618)

#### Güttel et al. 2014

S. Güttel, R. Van Beeumen, K. Meerbergen, W. Michiels, "NLEIGS: A Class of Fully Rational Krylov Methods for Nonlinear Eigenvalue Problems", SIAM J. Sci. Comput. 36, A2842 (2014). [doi:10.1137/130935045](https://doi.org/10.1137/130935045)

- Other documents: [docs/plans/performance.md:617](plans/performance.md?plain=1#L617)

### H

#### Hadley 1992

G. R. Hadley, "Wide-angle beam propagation using Padé approximant operators", Opt. Lett. 17, 1426 (1992). [doi:10.1364/OL.17.001426](https://doi.org/10.1364/OL.17.001426)

- Other documents: [ROADMAP.md:739](../ROADMAP.md?plain=1#L739)

#### Hadley 2002a

G. Hadley, "High-accuracy finite-difference equations for dielectric waveguide analysis I: uniform regions and dielectric interfaces", J. Lightwave Technol. 20, 1210 (2002). [doi:10.1109/JLT.2002.800361](https://doi.org/10.1109/JLT.2002.800361)

- Methods pages: [devices.md:16](methods/devices.md?plain=1#L16), [hadley.md:8](methods/hadley.md?plain=1#L8)
- Academy lessons: [strip-waveguide.md:76](../academy/strip-waveguide.md?plain=1#L76), [675](../academy/strip-waveguide.md?plain=1#L675)
- Validation cases: [`mode/hadley-uniform-box`](validation.md?plain=1#L149)
- Code: [src/device/fit.rs:9](../src/device/fit.rs#L9), [src/device/waveguide.rs:24](../src/device/waveguide.rs#L24), [35](../src/device/waveguide.rs#L35), [474](../src/device/waveguide.rs#L474), [src/mode/hadley.rs:5](../src/mode/hadley.rs#L5), [src/validation.rs:1068](../src/validation.rs#L1068)
- Other documents: [ROADMAP.md:727](../ROADMAP.md?plain=1#L727)

#### Hadley 2002b

G. Hadley, "High-accuracy finite-difference equations for dielectric waveguide analysis II: dielectric corners", J. Lightwave Technol. 20, 1219 (2002). [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371)

- Methods pages: [devices.md:18](methods/devices.md?plain=1#L18), [fdfd-3d.md:28](methods/fdfd-3d.md?plain=1#L28), [hadley.md:10](methods/hadley.md?plain=1#L10), [stacks.md:8](methods/stacks.md?plain=1#L8), [vector.md:10](methods/vector.md?plain=1#L10), [walls.md:8](methods/walls.md?plain=1#L8)
- Academy lessons: [strip-waveguide.md:82](../academy/strip-waveguide.md?plain=1#L82), [416](../academy/strip-waveguide.md?plain=1#L416), [605](../academy/strip-waveguide.md?plain=1#L605), [676](../academy/strip-waveguide.md?plain=1#L676), [705](../academy/strip-waveguide.md?plain=1#L705)
- Validation cases: [`mode/hadley-box-low`](validation.md?plain=1#L104), [`mode/hadley-box-high`](validation.md?plain=1#L105), [`mode/hadley-corner-low`](validation.md?plain=1#L106), [`mode/hadley-corner-high`](validation.md?plain=1#L107), [`mode/hadley-corners-box-low`](validation.md?plain=1#L154), [`fdfd3d/port-mode-strip`](validation.md?plain=1#L166), [`device/waveguide-fit-neff`](validation.md?plain=1#L313), [`device/waveguide-3d-order`](validation.md?plain=1#L315), [`device/rib-eim`](validation.md?plain=1#L333), [`device/rib-hadley`](validation.md?plain=1#L334), [`stack/rib-hadley-table`](validation.md?plain=1#L486)
- Examples: [README.md:45](../examples/README.md?plain=1#L45), [76](../examples/README.md?plain=1#L76), [hadley_corners.rs:6](../examples/hadley_corners.rs#L6), [rib_hadley.rs:5](../examples/rib_hadley.rs#L5), [49](../examples/rib_hadley.rs#L49), [strip_waveguide.rs:16](../examples/strip_waveguide.rs#L16)
- Code: [src/bench/catalogue.rs:776](../src/bench/catalogue.rs#L776), [src/cases/stack/checks.rs:449](../src/cases/stack/checks.rs#L449), [src/device/checks.rs:89](../src/device/checks.rs#L89), [103](../src/device/checks.rs#L103), [src/device/fit.rs:9](../src/device/fit.rs#L9), [src/device/splitter_checks.rs:89](../src/device/splitter_checks.rs#L89), [96](../src/device/splitter_checks.rs#L96), [385](../src/device/splitter_checks.rs#L385), [src/device/waveguide.rs:25](../src/device/waveguide.rs#L25), [36](../src/device/waveguide.rs#L36), [474](../src/device/waveguide.rs#L474), [src/mode/hadley.rs:7](../src/mode/hadley.rs#L7), [src/mode/vector.rs:16](../src/mode/vector.rs#L16), [1464](../src/mode/vector.rs#L1464), [src/validation.rs:753](../src/validation.rs#L753), [760](../src/validation.rs#L760), [767](../src/validation.rs#L767), [774](../src/validation.rs#L774), [1103](../src/validation.rs#L1103), [1187](../src/validation.rs#L1187)
- Other documents: [ROADMAP.md:728](../ROADMAP.md?plain=1#L728)

#### Hairer and Wanner 1996

E. Hairer, G. Wanner, "Solving Ordinary Differential Equations II", Springer Berlin Heidelberg (1996). [doi:10.1007/978-3-642-05221-7](https://doi.org/10.1007/978-3-642-05221-7)

- Methods pages: [static.md:26](methods/static.md?plain=1#L26)

#### Halir et al. 2016

R. Halir, P. Cheben, J. M. Luque‐González, J. D. Sarmiento‐Merenguel, J. H. Schmid, G. Wangüemert‐Pérez, D. Xu, S. Wang, A. Ortega‐Moñux, Í. Molina‐Fernández, "Ultra‐broadband nanophotonic beamsplitter using an anisotropic sub‐wavelength metamaterial", Laser & Photonics Reviews 10, 1039 (2016). [doi:10.1002/lpor.201600213](https://doi.org/10.1002/lpor.201600213), open access.

- Academy lessons: [directional-coupler.md:100](../academy/directional-coupler.md?plain=1#L100), [503](../academy/directional-coupler.md?plain=1#L503)

#### Hamerly et al. 2022

R. Hamerly, S. Bandyopadhyay, D. Englund, "Accurate Self-Configuration of Rectangular Multiport Interferometers", Phys. Rev. Applied 18, 024019 (2022). [doi:10.1103/PhysRevApplied.18.024019](https://doi.org/10.1103/PhysRevApplied.18.024019)

- Other documents: [ROADMAP.md:824](../ROADMAP.md?plain=1#L824)

#### Hamilton et al. 2017

C. S. Hamilton, R. Kruse, L. Sansoni, S. Barkhofen, C. Silberhorn, I. Jex, "Gaussian Boson Sampling", Phys. Rev. Lett. 119, 170501 (2017). [doi:10.1103/PhysRevLett.119.170501](https://doi.org/10.1103/PhysRevLett.119.170501)

- Other documents: [docs/plans/quantum.md:500](plans/quantum.md?plain=1#L500)

#### Hammerstad and Jensen 1980

E. Hammerstad, O. Jensen, "Accurate Models for Microstrip Computer-Aided Design", 1980 IEEE MTT-S International Microwave symposium Digest (1980). [doi:10.1109/MWSYM.1980.1124303](https://doi.org/10.1109/MWSYM.1980.1124303)

- Code: [crosscode/src/microstrip.rs:100](../crosscode/src/microstrip.rs#L100)

#### Hammond et al. 2021

A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, "Photonic topology optimization with semiconductor-foundry design-rule constraints", Opt. Express 29, 23916 (2021). [doi:10.1364/OE.431188](https://doi.org/10.1364/OE.431188)

- Other documents: [ROADMAP.md:776](../ROADMAP.md?plain=1#L776), [docs/plans/optimization.md:598](plans/optimization.md?plain=1#L598)

#### Hansen and Ostermeier 2001

N. Hansen, A. Ostermeier, "Completely Derandomized Self-Adaptation in Evolution Strategies", Evolutionary Computation 9, 159 (2001). [doi:10.1162/106365601750190398](https://doi.org/10.1162/106365601750190398)

- Methods pages: [optimization.md:136](methods/optimization.md?plain=1#L136)
- Code: [src/job/optimize.rs:214](../src/job/optimize.rs#L214)
- Other documents: [docs/plans/optimization.md:577](plans/optimization.md?plain=1#L577)

#### Haralick et al. 1987

R. M. Haralick, S. R. Sternberg, X. Zhuang, "Image Analysis Using Mathematical Morphology", IEEE Trans. Pattern Anal. Mach. Intell. PAMI-9, 532 (1987). [doi:10.1109/TPAMI.1987.4767941](https://doi.org/10.1109/TPAMI.1987.4767941)

- Methods pages: [booleans.md:12](methods/booleans.md?plain=1#L12)
- Validation cases: [`geometry/booleans/fillets`](validation.md?plain=1#L426)
- Code: [src/geometry/boolean_checks.rs:798](../src/geometry/boolean_checks.rs#L798)

#### Harris et al. 2014

N. C. Harris, Y. Ma, J. Mower, T. Baehr-Jones, D. Englund, M. Hochberg, C. Galland, "Efficient, compact and low loss thermo-optic phase shifter in silicon", Opt. Express 22, 10487 (2014). [doi:10.1364/OE.22.010487](https://doi.org/10.1364/OE.22.010487)

- Methods pages: [devices.md:40](methods/devices.md?plain=1#L40), [phase-shifter.md:10](methods/phase-shifter.md?plain=1#L10), [161](methods/phase-shifter.md?plain=1#L161)
- Validation cases: [`shifter/harris-p-pi`](validation.md?plain=1#L628), [`shifter/harris-bandwidth`](validation.md?plain=1#L629), [`shifter/harris-bounds`](validation.md?plain=1#L630), [`device/thermo-harris`](validation.md?plain=1#L654), [`device/thermo-harris-bandwidth`](validation.md?plain=1#L655), [`device/thermo-in-guide-measured`](validation.md?plain=1#L656)
- Examples: [README.md:81](../examples/README.md?plain=1#L81), [thermo_harris.rs:5](../examples/thermo_harris.rs#L5)
- Code: [crosscode/src/joule.rs:2](../crosscode/src/joule.rs#L2), [src/cases/device/thermo_checks.rs:496](../src/cases/device/thermo_checks.rs#L496), [503](../src/cases/device/thermo_checks.rs#L503), [510](../src/cases/device/thermo_checks.rs#L510), [src/device/measured/harris-2014.toml:5](../src/device/measured/harris-2014.toml#L5), [src/device/thermo.rs:708](../src/device/thermo.rs#L708), [src/fabrication/references.rs:168](../src/fabrication/references.rs#L168), [170](../src/fabrication/references.rs#L170), [src/thermal/crosstalk_checks.rs:135](../src/thermal/crosstalk_checks.rs#L135), [346](../src/thermal/crosstalk_checks.rs#L346), [411](../src/thermal/crosstalk_checks.rs#L411), [431](../src/thermal/crosstalk_checks.rs#L431), [445](../src/thermal/crosstalk_checks.rs#L445)
- Other documents: [ROADMAP.md:800](../ROADMAP.md?plain=1#L800), [docs/plans/fabrication.md:655](plans/fabrication.md?plain=1#L655)

#### He et al. 2019

M. He, M. Xu, Y. Ren, J. Jian, Z. Ruan, Y. Xu, S. Gao, S. Sun, X. Wen, L. Zhou, et al., "High-performance hybrid silicon and lithium niobate Mach–Zehnder modulators for 100 Gbit s−1 and beyond", Nat. Photonics 13, 359 (2019). [doi:10.1038/s41566-019-0378-6](https://doi.org/10.1038/s41566-019-0378-6)

- Methods pages: [electro-optic.md:16](methods/electro-optic.md?plain=1#L16), [218](methods/electro-optic.md?plain=1#L218)
- Other documents: [ROADMAP.md:782](../ROADMAP.md?plain=1#L782)

#### Heebner et al. 2004

J. Heebner, Vincent Wong, A. Schweinsberg, R. Boyd, D. Jackson, "Optical transmission characteristics of fiber ring resonators", IEEE J. Quantum Electron. 40, 726 (2004). [doi:10.1109/JQE.2004.828232](https://doi.org/10.1109/JQE.2004.828232)

- Academy lessons: [ring-resonator.md:67](../academy/ring-resonator.md?plain=1#L67), [262](../academy/ring-resonator.md?plain=1#L262), [536](../academy/ring-resonator.md?plain=1#L536)

#### Heiblum and Harris 1975

M. Heiblum, J. Harris, "Analysis of curved optical waveguides by conformal transformation", IEEE J. Quantum Electron. 11, 75 (1975). [doi:10.1109/JQE.1975.1068563](https://doi.org/10.1109/JQE.1975.1068563)

- Methods pages: [bends.md:8](methods/bends.md?plain=1#L8), [components.md:16](methods/components.md?plain=1#L16), [devices.md:20](methods/devices.md?plain=1#L20)
- Academy lessons: [leaky-and-bent-guides.md:59](../academy/leaky-and-bent-guides.md?plain=1#L59), [430](../academy/leaky-and-bent-guides.md?plain=1#L430), [557](../academy/leaky-and-bent-guides.md?plain=1#L557)
- Validation cases: [`mode/bend-slab-te`](validation.md?plain=1#L121), [`device/bend-analytic`](validation.md?plain=1#L316), [`device/bend-3d-grid`](validation.md?plain=1#L318)
- Examples: [bend_loss.rs:14](../examples/bend_loss.rs#L14)
- Code: [src/circuit/components/waveguide.rs:505](../src/circuit/components/waveguide.rs#L505), [src/device/bend.rs:271](../src/device/bend.rs#L271), [476](../src/device/bend.rs#L476), [src/device/checks.rs:110](../src/device/checks.rs#L110), [124](../src/device/checks.rs#L124), [src/mode/vector.rs:444](../src/mode/vector.rs#L444), [src/validation.rs:872](../src/validation.rs#L872)

#### Heinrich 1993

W. Heinrich, "Quasi-TEM description of MMIC coplanar lines including conductor-loss effects", IEEE Trans. Microwave Theory Techn. 41, 45 (1993). [doi:10.1109/22.210228](https://doi.org/10.1109/22.210228)

- Methods pages: [travelling-wave.md:26](methods/travelling-wave.md?plain=1#L26)
- Validation cases: [`line/full-wave-quasi-tem-index`](validation.md?plain=1#L596), [`line/heinrich-1993-external`](validation.md?plain=1#L603)
- Code: [src/electro/full_wave.rs:34](../src/electro/full_wave.rs#L34), [src/electro/full_wave_checks.rs:281](../src/electro/full_wave_checks.rs#L281), [src/electro/full_wave_coplanar.rs:577](../src/electro/full_wave_coplanar.rs#L577)

#### Helt et al. 2010

L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, "Spontaneous four-wave mixing in microring resonators", Opt. Lett. 35, 3006 (2010). [doi:10.1364/OL.35.003006](https://doi.org/10.1364/OL.35.003006)

- Validation cases: [`nonlinear/ring-shg-enhancement`](validation.md?plain=1#L807), [`nonlinear/ring-fwm-enhancement`](validation.md?plain=1#L808)
- Code: [src/circuit/components/ring.rs:309](../src/circuit/components/ring.rs#L309), [src/nonlinear/cavity.rs:32](../src/nonlinear/cavity.rs#L32), [src/nonlinear/cavity_checks.rs:359](../src/nonlinear/cavity_checks.rs#L359), [366](../src/nonlinear/cavity_checks.rs#L366)
- Other documents: [docs/plans/quantum.md:530](plans/quantum.md?plain=1#L530)

#### Henson and Yang 2002

V. E. Henson, U. M. Yang, "BoomerAMG: A parallel algebraic multigrid solver and preconditioner", Applied Numerical Mathematics 41, 155 (2002). [doi:10.1016/S0168-9274(01)00115-5](https://doi.org/10.1016/S0168-9274(01)00115-5)

- Other documents: [docs/plans/performance.md:619](plans/performance.md?plain=1#L619)

#### Hernandez et al. 2005

V. Hernandez, J. E. Roman, V. Vidal, "SLEPc: A scalable and flexible toolkit for the solution of eigenvalue problems", ACM Trans. Math. Softw. 31, 351 (2005). [doi:10.1145/1089014.1089019](https://doi.org/10.1145/1089014.1089019)

- Other documents: [docs/plans/performance.md:620](plans/performance.md?plain=1#L620)

#### Herr et al. 2013

T. Herr, V. Brasch, J. D. Jost, C. Y. Wang, N. M. Kondratiev, M. L. Gorodetsky, T. J. Kippenberg, "Temporal solitons in optical microresonators", Nature Photon 8, 145 (2013). [doi:10.1038/nphoton.2013.343](https://doi.org/10.1038/nphoton.2013.343)

- Methods pages: [nonlinear.md:40](methods/nonlinear.md?plain=1#L40)
- Code: [src/nonlinear/resonator.rs:9](../src/nonlinear/resonator.rs#L9)
- Other documents: [ROADMAP.md:856](../ROADMAP.md?plain=1#L856)

#### Hestenes 1958

M. R. Hestenes, "Inversion of Matrices by Biorthogonalization and Related Results", Journal of the Society for Industrial and Applied Mathematics 6, 51 (1958). [doi:10.1137/0106005](https://doi.org/10.1137/0106005)

- Code: [src/lstsq.rs:64](../src/lstsq.rs#L64)

#### Heurtel et al. 2023

N. Heurtel, A. Fyrillas, G. D. Gliniasty, R. Le Bihan, S. Malherbe, M. Pailhas, E. Bertasi, B. Bourdoncle, P.-E. Emeriau, R. Mezher, et al., "Perceval: A Software Platform for Discrete Variable Photonic Quantum Computing", Quantum 7, 931 (2023). [doi:10.22331/q-2023-02-21-931](https://doi.org/10.22331/q-2023-02-21-931), open access.

- Other documents: [docs/plans/quantum.md:541](plans/quantum.md?plain=1#L541)

#### Higham 2002

N. J. Higham, "Accuracy and Stability of Numerical Algorithms: Second Edition", Society for Industrial and Applied Mathematics (2002). [doi:10.1137/1.9780898718027](https://doi.org/10.1137/1.9780898718027)

- Code: [src/validation/gpu.rs:33](../src/validation/gpu.rs#L33)

#### Higham and Mary 2022

N. J. Higham, T. Mary, "Mixed precision algorithms in numerical linear algebra", Acta Numerica 31, 347 (2022). [doi:10.1017/S0962492922000022](https://doi.org/10.1017/S0962492922000022), open access.

- Other documents: [docs/plans/performance.md:621](plans/performance.md?plain=1#L621)

#### Hill et al. 1978

K. O. Hill, Y. Fujii, D. C. Johnson, B. S. Kawasaki, "Photosensitivity in optical fiber waveguides: Application to reflection filter fabrication", Applied Physics Letters 32, 647 (1978). [doi:10.1063/1.89881](https://doi.org/10.1063/1.89881)

- Academy lessons: [bragg-gratings.md:55](../academy/bragg-gratings.md?plain=1#L55)

#### Hiptmair 1998

R. Hiptmair, "Multigrid Method for Maxwell's Equations", SIAM J. Numer. Anal. 36, 204 (1998). [doi:10.1137/S0036142997326203](https://doi.org/10.1137/S0036142997326203)

- Methods pages: [multigrid-stencil.md:14](methods/multigrid-stencil.md?plain=1#L14)
- Other documents: [docs/plans/performance.md:622](plans/performance.md?plain=1#L622)

#### Hiptmair and Xu 2007

R. Hiptmair, J. Xu, "Nodal Auxiliary Space Preconditioning in H(curl) and H(div) Spaces", SIAM J. Numer. Anal. 45, 2483 (2007). [doi:10.1137/060660588](https://doi.org/10.1137/060660588)

- Methods pages: [multigrid-stencil.md:18](methods/multigrid-stencil.md?plain=1#L18)
- Other documents: [docs/plans/performance.md:623](plans/performance.md?plain=1#L623)

#### Hjaltason and Samet 1999

G. R. Hjaltason, H. Samet, "Distance browsing in spatial databases", ACM Trans. Database Syst. 24, 265 (1999). [doi:10.1145/320248.320255](https://doi.org/10.1145/320248.320255)

- Methods pages: [arrays.md:8](methods/arrays.md?plain=1#L8)

#### Ho et al. 1972

C. Y. Ho, R. W. Powell, P. E. Liley, "Thermal Conductivity of the Elements", Journal of Physical and Chemical Reference Data 1, 279 (1972). [doi:10.1063/1.3253100](https://doi.org/10.1063/1.3253100)

- Methods pages: [catalogue.md:98](methods/catalogue.md?plain=1#L98)
- Validation cases: [`material/conductivity-tables`](validation.md?plain=1#L33), [`material/silicon-glassbrenner-ho`](validation.md?plain=1#L36)
- Code: [src/material/catalogue/conductivity.rs:5](../src/material/catalogue/conductivity.rs#L5), [src/material/catalogue/references.rs:410](../src/material/catalogue/references.rs#L410), [src/validation.rs:256](../src/validation.rs#L256), [277](../src/validation.rs#L277)

#### Hocker and Burns 1977

G. B. Hocker, W. K. Burns, "Mode dispersion in diffused channel waveguides by the effective index method", Appl. Opt. 16, 113 (1977). [doi:10.1364/AO.16.000113](https://doi.org/10.1364/AO.16.000113)

- Methods pages: [devices.md:24](methods/devices.md?plain=1#L24), [eim.md:8](methods/eim.md?plain=1#L8)
- Academy lessons: [strip-waveguide.md:58](../academy/strip-waveguide.md?plain=1#L58), [300](../academy/strip-waveguide.md?plain=1#L300), [670](../academy/strip-waveguide.md?plain=1#L670)
- Validation cases: [`mode/eim-strip-book`](validation.md?plain=1#L120), [`device/rib-eim`](validation.md?plain=1#L333), [`device/mmi-guided-3d`](validation.md?plain=1#L339), [`device/mmi-fdfd-3d`](validation.md?plain=1#L340), [`device/mmi-best-length-3d`](validation.md?plain=1#L341), [`device/y-branch-2d-3d`](validation.md?plain=1#L344), [`device/pcm-analytic-index`](validation.md?plain=1#L661)
- Examples: [README.md:39](../examples/README.md?plain=1#L39)
- Code: [src/cases/device/pcm_checks.rs:426](../src/cases/device/pcm_checks.rs#L426), [src/circuit/components/mmi.rs:45](../src/circuit/components/mmi.rs#L45), [src/device/mmi.rs:320](../src/device/mmi.rs#L320), [338](../src/device/mmi.rs#L338), [src/device/pcm.rs:980](../src/device/pcm.rs#L980), [src/device/rib.rs:226](../src/device/rib.rs#L226), [295](../src/device/rib.rs#L295), [src/device/splitter_checks.rs:89](../src/device/splitter_checks.rs#L89), [131](../src/device/splitter_checks.rs#L131), [138](../src/device/splitter_checks.rs#L138), [145](../src/device/splitter_checks.rs#L145), [166](../src/device/splitter_checks.rs#L166), [src/mode/eim.rs:6](../src/mode/eim.rs#L6), [src/validation.rs:865](../src/validation.rs#L865)

#### Holloway and Kuester 1995

C. Holloway, E. Kuester, "A quasi-closed form expression for the conductor loss of CPW lines, with an investigation of edge shape effects", IEEE Trans. Microwave Theory Techn. 43, 2695 (1995). [doi:10.1109/22.477846](https://doi.org/10.1109/22.477846)

- Methods pages: [devices.md:58](methods/devices.md?plain=1#L58), [travelling-wave.md:8](methods/travelling-wave.md?plain=1#L8)
- Validation cases: [`line/holloway-kuester-current`](validation.md?plain=1#L561), [`line/wheeler-holloway-kuester`](validation.md?plain=1#L567), [`line/full-wave-quasi-tem-loss`](validation.md?plain=1#L598)
- Code: [src/cases/electro/line_checks.rs:920](../src/cases/electro/line_checks.rs#L920), [965](../src/cases/electro/line_checks.rs#L965), [src/device/tfln.rs:1204](../src/device/tfln.rs#L1204), [src/electro/full_wave_checks.rs:295](../src/electro/full_wave_checks.rs#L295), [src/electro/line.rs:17](../src/electro/line.rs#L17)
- Other documents: [ROADMAP.md:795](../ROADMAP.md?plain=1#L795)

#### Holzgrafe et al. 2024

J. Holzgrafe, E. Puma, R. Cheng, H. Warner, A. Shams-Ansari, R. Shankar, M. Lončar, "Relaxation of the electro-optic response in thin-film lithium niobate modulators", Opt. Express 32, 3619 (2024). [doi:10.1364/OE.507536](https://doi.org/10.1364/OE.507536)

- Methods pages: [electro-optic.md:20](methods/electro-optic.md?plain=1#L20)
- Validation cases: [`pockels/wang-2018`](validation.md?plain=1#L552)
- Code: [src/cases/mode/pockels/checks.rs:1026](../src/cases/mode/pockels/checks.rs#L1026)
- Other documents: [ROADMAP.md:792](../ROADMAP.md?plain=1#L792)

#### Hong et al. 1987

C. K. Hong, Z. Y. Ou, L. Mandel, "Measurement of subpicosecond time intervals between two photons by interference", Phys. Rev. Lett. 59, 2044 (1987). [doi:10.1103/PhysRevLett.59.2044](https://doi.org/10.1103/PhysRevLett.59.2044)

- Other documents: [docs/plans/quantum.md:485](plans/quantum.md?plain=1#L485)

#### Horst et al. 2013

F. Horst, W. M. Green, S. Assefa, S. M. Shank, Y. A. Vlasov, B. J. Offrein, "Cascaded Mach-Zehnder wavelength filters in silicon photonics for low loss and flat pass-band WDM (de-)multiplexing", Opt. Express 21, 11652 (2013). [doi:10.1364/OE.21.011652](https://doi.org/10.1364/OE.21.011652)

- Academy lessons: [mach-zehnder.md:94](../academy/mach-zehnder.md?plain=1#L94), [377](../academy/mach-zehnder.md?plain=1#L377)
- Other documents: [docs/plans/optimization.md:622](plans/optimization.md?plain=1#L622)

#### Hu and Menyuk 2009

J. Hu, C. R. Menyuk, "Understanding leaky modes: slab waveguide revisited", Adv. Opt. Photon. 1, 58 (2009). [doi:10.1364/AOP.1.000058](https://doi.org/10.1364/AOP.1.000058)

- Methods pages: [vector.md:16](methods/vector.md?plain=1#L16), [74](methods/vector.md?plain=1#L74)
- Academy lessons: [leaky-and-bent-guides.md:113](../academy/leaky-and-bent-guides.md?plain=1#L113), [209](../academy/leaky-and-bent-guides.md?plain=1#L209), [569](../academy/leaky-and-bent-guides.md?plain=1#L569), [584](../academy/leaky-and-bent-guides.md?plain=1#L584), [strip-waveguide.md:124](../academy/strip-waveguide.md?plain=1#L124), [560](../academy/strip-waveguide.md?plain=1#L560)

#### Hughes et al. 2018a

T. W. Hughes, M. Minkov, I. A. D. Williamson, S. Fan, "Adjoint Method and Inverse Design for Nonlinear Nanophotonic Devices", ACS Photonics 5, 4781 (2018). [doi:10.1021/acsphotonics.8b01522](https://doi.org/10.1021/acsphotonics.8b01522)

- Methods pages: [fdfd-adjoint.md:12](methods/fdfd-adjoint.md?plain=1#L12), [fdtd-adjoint.md:12](methods/fdtd-adjoint.md?plain=1#L12)
- Code: [src/fdfd/adjoint.rs:9](../src/fdfd/adjoint.rs#L9), [src/fdtd/adjoint.rs:30](../src/fdtd/adjoint.rs#L30)
- Other documents: [ROADMAP.md:768](../ROADMAP.md?plain=1#L768)

#### Hughes et al. 2018b

T. W. Hughes, M. Minkov, Y. Shi, S. Fan, "Training of photonic neural networks through in situ backpropagation and gradient measurement", Optica 5, 864 (2018). [doi:10.1364/OPTICA.5.000864](https://doi.org/10.1364/OPTICA.5.000864)

- Methods pages: [circuit-adjoint.md:10](methods/circuit-adjoint.md?plain=1#L10)
- Other documents: [docs/plans/optimization.md:628](plans/optimization.md?plain=1#L628)

#### Hughes et al. 2019

T. W. Hughes, I. A. D. Williamson, M. Minkov, S. Fan, "Forward-Mode Differentiation of Maxwell’s Equations", ACS Photonics 6, 3010 (2019). [doi:10.1021/acsphotonics.9b01238](https://doi.org/10.1021/acsphotonics.9b01238)

- Other documents: [ROADMAP.md:769](../ROADMAP.md?plain=1#L769)

#### Hughes et al. 2021

T. W. Hughes, M. Minkov, V. Liu, Z. Yu, S. Fan, "A perspective on the pathway toward full wave simulation of large area metalenses", Applied Physics Letters 119, 150502 (2021). [doi:10.1063/5.0071245](https://doi.org/10.1063/5.0071245), open access.

- Other documents: [docs/plans/performance.md:624](plans/performance.md?plain=1#L624)

### I

#### Ikeda et al. 2008

K. Ikeda, R. E. Saperstein, N. Alic, Y. Fainman, "Thermal and Kerr nonlinear properties of plasma-deposited silicon nitride/ silicon dioxide waveguides", Opt. Express 16, 12987 (2008). [doi:10.1364/OE.16.012987](https://doi.org/10.1364/OE.16.012987)

- Methods pages: [catalogue.md:32](methods/catalogue.md?plain=1#L32)
- Code: [src/material/catalogue/kerr.rs:11](../src/material/catalogue/kerr.rs#L11), [src/material/catalogue/references.rs:592](../src/material/catalogue/references.rs#L592), [593](../src/material/catalogue/references.rs#L593)

### J

#### Jacques et al. 2019

M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, "Optimization of thermo-optic phase-shifter design and mitigation of thermal crosstalk on the SOI platform", Opt. Express 27, 10456 (2019). [doi:10.1364/OE.27.010456](https://doi.org/10.1364/OE.27.010456)

- Methods pages: [devices.md:36](methods/devices.md?plain=1#L36), [fields.md:16](methods/fields.md?plain=1#L16), [phase-shifter.md:8](methods/phase-shifter.md?plain=1#L8), [95](methods/phase-shifter.md?plain=1#L95), [static.md:28](methods/static.md?plain=1#L28), [285](methods/static.md?plain=1#L285), [thermal-compact.md:10](methods/thermal-compact.md?plain=1#L10)
- Validation cases: [`material/silicon-film-220`](validation.md?plain=1#L39), [`static/jacques-tin-heater`](validation.md?plain=1#L526), [`static/jacques-tin-heater-time`](validation.md?plain=1#L527), [`static/jacques-tin-heater-catalogue`](validation.md?plain=1#L528), [`thermo/heater-field`](validation.md?plain=1#L541), [`shifter/jacques-tin`](validation.md?plain=1#L619), [`shifter/jacques-npp`](validation.md?plain=1#L620), [`shifter/jacques-tin-time`](validation.md?plain=1#L621), [`shifter/jacques-npp-time`](validation.md?plain=1#L622), [`shifter/jacques-crosstalk-5`](validation.md?plain=1#L625), [`shifter/jacques-crosstalk-15`](validation.md?plain=1#L626), [`shifter/jacques-trench`](validation.md?plain=1#L627), [`device/thermo-jacques-tin`](validation.md?plain=1#L648), [`device/thermo-jacques-npp`](validation.md?plain=1#L649), [`device/thermo-tin-measured`](validation.md?plain=1#L650), [`device/thermo-npp-measured`](validation.md?plain=1#L651)
- Examples: [README.md:80](../examples/README.md?plain=1#L80), [thermo_jacques.rs:5](../examples/thermo_jacques.rs#L5)
- Code: [crosscode/src/heaters.rs:2](../crosscode/src/heaters.rs#L2), [crosscode/src/joule.rs:15](../crosscode/src/joule.rs#L15), [crosscode/src/transient.rs:2](../crosscode/src/transient.rs#L2), [src/cases/device/thermo_checks.rs:441](../src/cases/device/thermo_checks.rs#L441), [458](../src/cases/device/thermo_checks.rs#L458), [468](../src/cases/device/thermo_checks.rs#L468), [475](../src/cases/device/thermo_checks.rs#L475), [src/cases/mode/thermo_checks.rs:387](../src/cases/mode/thermo_checks.rs#L387), [src/cases/statics/checks.rs:144](../src/cases/statics/checks.rs#L144), [509](../src/cases/statics/checks.rs#L509), [998](../src/cases/statics/checks.rs#L998), [1019](../src/cases/statics/checks.rs#L1019), [1043](../src/cases/statics/checks.rs#L1043), [src/cases/thermal/shifter_checks.rs:189](../src/cases/thermal/shifter_checks.rs#L189), [451](../src/cases/thermal/shifter_checks.rs#L451), [464](../src/cases/thermal/shifter_checks.rs#L464), [474](../src/cases/thermal/shifter_checks.rs#L474), [493](../src/cases/thermal/shifter_checks.rs#L493), [src/device/active.rs:198](../src/device/active.rs#L198), [src/device/measured/jacques-2019-npp.toml:5](../src/device/measured/jacques-2019-npp.toml#L5), [src/device/measured/jacques-2019-tin.toml:4](../src/device/measured/jacques-2019-tin.toml#L4), [src/device/thermo.rs:701](../src/device/thermo.rs#L701), [src/fabrication/references.rs:160](../src/fabrication/references.rs#L160), [162](../src/fabrication/references.rs#L162), [src/thermal/crosstalk_checks.rs:320](../src/thermal/crosstalk_checks.rs#L320), [364](../src/thermal/crosstalk_checks.rs#L364), [379](../src/thermal/crosstalk_checks.rs#L379), [394](../src/thermal/crosstalk_checks.rs#L394), [src/validation.rs:298](../src/validation.rs#L298)
- Jobs and circuits: [jobs/pcm-switching.yaml:39](../jobs/pcm-switching.yaml#L39)
- Other documents: [ROADMAP.md:799](../ROADMAP.md?plain=1#L799), [docs/plans/fabrication.md:655](plans/fabrication.md?plain=1#L655)

#### JaeHyuk Shin et al. 2005

JaeHyuk Shin, C. Ozturk, S. Sakamoto, Y. Chiu, N. Dagli, "Novel T-rail electrodes for substrate removed low-voltage high-speed GaAs/AlGaAs electrooptic modulators", IEEE Trans. Microwave Theory Techn. 53, 636 (2005). [doi:10.1109/TMTT.2004.840735](https://doi.org/10.1109/TMTT.2004.840735)

- Methods pages: [travelling-wave.md:22](methods/travelling-wave.md?plain=1#L22)
- Validation cases: [`line/periodic-shin-limit`](validation.md?plain=1#L583)
- Code: [src/cases/electro/line_checks.rs:1077](../src/cases/electro/line_checks.rs#L1077), [src/electro/line.rs:46](../src/electro/line.rs#L46), [501](../src/electro/line.rs#L501)
- Other documents: [ROADMAP.md:797](../ROADMAP.md?plain=1#L797)

#### Jazbinšek and Zgonik 2002

M. Jazbinšek, M. Zgonik, "Material tensor parameters of LiNbO 3 relevant for electro- and elasto-optics", Applied Physics B: Lasers and Optics 74, 407 (2002). [doi:10.1007/s003400200818](https://doi.org/10.1007/s003400200818)

- Methods pages: [catalogue.md:52](methods/catalogue.md?plain=1#L52), [electro-optic.md:8](methods/electro-optic.md?plain=1#L8)
- Validation cases: [`material/linbo3-zelmon-633`](validation.md?plain=1#L64), [`pockels/plates-x-cut`](validation.md?plain=1#L545)
- Code: [src/cases/mode/pockels/checks.rs:977](../src/cases/mode/pockels/checks.rs#L977), [src/device/tfln.rs:704](../src/device/tfln.rs#L704), [src/material/catalogue/references.rs:151](../src/material/catalogue/references.rs#L151), [src/validation.rs:473](../src/validation.rs#L473)
- Other documents: [ROADMAP.md:786](../ROADMAP.md?plain=1#L786)

#### Jensen and Sigmund 2010

J. Jensen, O. Sigmund, "Topology optimization for nano‐photonics", Laser &amp; Photonics Reviews 5, 308 (2010). [doi:10.1002/lpor.201000014](https://doi.org/10.1002/lpor.201000014)

- Other documents: [ROADMAP.md:760](../ROADMAP.md?plain=1#L760)

#### Ji et al. 2017

X. Ji, F. A. S. Barbosa, S. P. Roberts, A. Dutt, J. Cardenas, Y. Okawachi, A. Bryant, A. L. Gaeta, M. Lipson, "Ultra-low-loss on-chip resonators with sub-milliwatt parametric oscillation threshold", Optica 4, 619 (2017). [doi:10.1364/OPTICA.4.000619](https://doi.org/10.1364/OPTICA.4.000619)

- Academy lessons: [ring-resonator.md:91](../academy/ring-resonator.md?plain=1#L91), [397](../academy/ring-resonator.md?plain=1#L397), [589](../academy/ring-resonator.md?plain=1#L589)

#### Jiang et al. 2020

J. Jiang, M. Chen, J. A. Fan, "Deep neural networks for the evaluation and design of photonic devices", Nat Rev Mater 6, 679 (2020). [doi:10.1038/s41578-020-00260-1](https://doi.org/10.1038/s41578-020-00260-1)

- Other documents: [docs/plans/optimization.md:635](plans/optimization.md?plain=1#L635)

#### Jinguji and Kawachi 1995

K. Jinguji, M. Kawachi, "Synthesis of coherent two-port lattice-form optical delay-line circuit", J. Lightwave Technol. 13, 73 (1995). [doi:10.1109/50.350643](https://doi.org/10.1109/50.350643)

- Academy lessons: [mach-zehnder.md:58](../academy/mach-zehnder.md?plain=1#L58), [364](../academy/mach-zehnder.md?plain=1#L364), [461](../academy/mach-zehnder.md?plain=1#L461)
- Examples: [optimize_mzi_lattice.rs:10](../examples/optimize_mzi_lattice.rs#L10)
- Other documents: [docs/plans/optimization.md:619](plans/optimization.md?plain=1#L619)

#### Jinguji and Oguma 2000

K. Jinguji, M. Oguma, "Optical half-band filters", J. Lightwave Technol. 18, 252 (2000). [doi:10.1109/50.822800](https://doi.org/10.1109/50.822800)

- Examples: [README.md:52](../examples/README.md?plain=1#L52), [optimize_mzi_lattice.rs:7](../examples/optimize_mzi_lattice.rs#L7)
- Other documents: [docs/plans/optimization.md:621](plans/optimization.md?plain=1#L621)

#### Jinguji et al. 1990

K. Jinguji, N. Takato, A. Sugita, M. Kawachi, "Mach–Zehnder interferometer type optical waveguide coupler with wavelength-flattened coupling ratio", Electron. Lett. (UK) 26, 1326 (1990). [doi:10.1049/el:19900855](https://doi.org/10.1049/el:19900855)

- Academy lessons: [directional-coupler.md:52](../academy/directional-coupler.md?plain=1#L52), [330](../academy/directional-coupler.md?plain=1#L330)

#### John 1987

S. John, "Strong localization of photons in certain disordered dielectric superlattices", Phys. Rev. Lett. 58, 2486 (1987). [doi:10.1103/PhysRevLett.58.2486](https://doi.org/10.1103/PhysRevLett.58.2486)

- Academy lessons: [bragg-gratings.md:73](../academy/bragg-gratings.md?plain=1#L73), [410](../academy/bragg-gratings.md?plain=1#L410)

#### Johnson and Joannopoulos 2001

S. Johnson, J. Joannopoulos, "Block-iterative frequency-domain methods for Maxwell's equations in a planewave basis", Opt. Express 8, 173 (2001). [doi:10.1364/OE.8.000173](https://doi.org/10.1364/OE.8.000173)

- Other documents: [ROADMAP.md:730](../ROADMAP.md?plain=1#L730)

#### Johnson et al. 2002

S. G. Johnson, M. Ibanescu, M. A. Skorobogatiy, O. Weisberg, J. D. Joannopoulos, Y. Fink, "Perturbation theory for Maxwell’s equations with shifting material boundaries", Phys. Rev. E 65, 066611 (2002). [doi:10.1103/PhysRevE.65.066611](https://doi.org/10.1103/PhysRevE.65.066611)

- Methods pages: [electro-optic.md:10](methods/electro-optic.md?plain=1#L10), [fields.md:12](methods/fields.md?plain=1#L12), [phase-shifter.md:12](methods/phase-shifter.md?plain=1#L12)
- Validation cases: [`thermo/slab-first-order`](validation.md?plain=1#L534), [`shifter/slab-sheet`](validation.md?plain=1#L615)
- Code: [src/cases/mode/thermo_checks.rs:338](../src/cases/mode/thermo_checks.rs#L338), [src/cases/thermal/shifter_checks.rs:409](../src/cases/thermal/shifter_checks.rs#L409), [src/device/thermo.rs:769](../src/device/thermo.rs#L769), [src/mode/thermo.rs:8](../src/mode/thermo.rs#L8)
- Other documents: [ROADMAP.md:798](../ROADMAP.md?plain=1#L798)

#### Jolivet and Tournier 2016

P. Jolivet, P.-H. Tournier, "Block Iterative Methods and Recycling for Improved Scalability of Linear Solvers", SC16: International Conference for High Performance Computing, Networking, Storage and Analysis (2016). [doi:10.1109/SC.2016.16](https://doi.org/10.1109/SC.2016.16)

- Methods pages: [block-qmr.md:10](methods/block-qmr.md?plain=1#L10), [111](methods/block-qmr.md?plain=1#L111)
- Code: [src/backend/krylov.rs:318](../src/backend/krylov.rs#L318)
- Other documents: [docs/plans/performance.md:626](plans/performance.md?plain=1#L626)

#### Jolivet et al. 2021

P. Jolivet, J. E. Roman, S. Zampini, "KSPHPDDM and PCHPDDM: Extending PETSc with advanced Krylov methods and robust multilevel overlapping Schwarz preconditioners", Computers & Mathematics with Applications 84, 277 (2021). [doi:10.1016/j.camwa.2021.01.003](https://doi.org/10.1016/j.camwa.2021.01.003)

- Other documents: [docs/plans/performance.md:625](plans/performance.md?plain=1#L625)

#### Jones et al. 1998

D. R. Jones, M. Schonlau, W. J. Welch, "Efficient Global Optimization of Expensive Black-Box Functions", Journal of Global Optimization 13, 455 (1998). [doi:10.1023/A:1008306431147](https://doi.org/10.1023/A:1008306431147)

- Other documents: [docs/plans/optimization.md:584](plans/optimization.md?plain=1#L584)

#### Ju and Goodson 1999

Y. S. Ju, K. E. Goodson, "Phonon scattering in silicon films with thickness of order 100 nm", Applied Physics Letters 74, 3005 (1999). [doi:10.1063/1.123994](https://doi.org/10.1063/1.123994)

- Methods pages: [catalogue.md:110](methods/catalogue.md?plain=1#L110)
- Validation cases: [`material/silicon-film-ju-goodson`](validation.md?plain=1#L38)
- Code: [src/material/catalogue/conductivity.rs:10](../src/material/catalogue/conductivity.rs#L10), [src/material/catalogue/references.rs:452](../src/material/catalogue/references.rs#L452), [src/validation.rs:291](../src/validation.rs#L291)
- Other documents: [ROADMAP.md:803](../ROADMAP.md?plain=1#L803)

#### Jundt 1997

D. H. Jundt, "Temperature-dependent Sellmeier equation for the index of refraction, n\_e, in congruent lithium niobate", Opt. Lett. 22, 1553 (1997). [doi:10.1364/OL.22.001553](https://doi.org/10.1364/OL.22.001553)

- Methods pages: [catalogue.md:46](methods/catalogue.md?plain=1#L46), [nonlinear.md:22](methods/nonlinear.md?plain=1#L22)
- Validation cases: [`material/linbo3-jundt-opo`](validation.md?plain=1#L66)
- Code: [src/material/catalogue/models.rs:602](../src/material/catalogue/models.rs#L602), [src/material/catalogue/references.rs:235](../src/material/catalogue/references.rs#L235), [src/nonlinear/phase.rs:796](../src/nonlinear/phase.rs#L796), [src/validation.rs:487](../src/validation.rs#L487)

#### Jundt et al. 1990

D. Jundt, M. Fejer, R. Byer, "Optical properties of lithium-rich lithium niobate fabricated by vapor transport equilibration", IEEE J. Quantum Electron. 26, 135 (1990). [doi:10.1109/3.44926](https://doi.org/10.1109/3.44926)

- Methods pages: [nonlinear.md:20](methods/nonlinear.md?plain=1#L20)
- Validation cases: [`nonlinear/fejer-table-1-coherence`](validation.md?plain=1#L713), [`nonlinear/fejer-table-1-wavelength`](validation.md?plain=1#L714), [`nonlinear/fejer-table-1-temperature`](validation.md?plain=1#L715), [`nonlinear/fejer-table-1-tilt-about-z`](validation.md?plain=1#L716), [`nonlinear/fejer-table-1-tilt-toward-z`](validation.md?plain=1#L717)
- Code: [src/material/catalogue/models.rs:710](../src/material/catalogue/models.rs#L710), [src/nonlinear/checks.rs:660](../src/nonlinear/checks.rs#L660)

#### Jung et al. 2013

H. Jung, C. Xiong, K. Y. Fong, X. Zhang, H. X. Tang, "Optical frequency comb generation from aluminum nitride microring resonator", Opt. Lett. 38, 2810 (2013). [doi:10.1364/OL.38.002810](https://doi.org/10.1364/OL.38.002810)

- Methods pages: [catalogue.md:36](methods/catalogue.md?plain=1#L36), [fabrication.md:50](methods/fabrication.md?plain=1#L50)
- Validation cases: [`fabrication/aln-reported-flows`](validation.md?plain=1#L762)
- Code: [src/fabrication/platform_checks.rs:763](../src/fabrication/platform_checks.rs#L763), [src/fabrication/references.rs:208](../src/fabrication/references.rs#L208), [src/material/catalogue/kerr.rs:17](../src/material/catalogue/kerr.rs#L17), [src/material/catalogue/references.rs:606](../src/material/catalogue/references.rs#L606)

### K

#### Kaito et al. 1984

C. Kaito, N. Nakamura, T. Yoshida, M. Shiojiri, "Structure and the process of crystallization in thin amorphous Sb2S3 films", Journal of Crystal Growth 66, 156 (1984). [doi:10.1016/0022-0248(84)90086-1](https://doi.org/10.1016/0022-0248(84)90086-1)

- Methods pages: [catalogue.md:444](methods/catalogue.md?plain=1#L444)
- Validation cases: [`material/pcm-sb2s3-kaito-deficit`](validation.md?plain=1#L61)
- Code: [src/material/catalogue/references.rs:60](../src/material/catalogue/references.rs#L60), [src/validation.rs:452](../src/validation.rs#L452)

#### Kalb et al. 2003

J. Kalb, F. Spaepen, M. Wuttig, "Calorimetric measurements of phase transformations in thin films of amorphous Te alloys used for optical data storage", Journal of Applied Physics 93, 2389 (2003). [doi:10.1063/1.1540227](https://doi.org/10.1063/1.1540227)

- Methods pages: [catalogue.md:413](methods/catalogue.md?plain=1#L413)
- Validation cases: [`material/pcm-gst-kalb-fusion`](validation.md?plain=1#L54)
- Code: [src/material/catalogue/phase_change_thermal.rs:2](../src/material/catalogue/phase_change_thermal.rs#L2), [src/material/catalogue/references.rs:515](../src/material/catalogue/references.rs#L515), [src/validation.rs:403](../src/validation.rs#L403)

#### Kane Yee 1966

Kane Yee, "Numerical solution of initial boundary value problems involving maxwell's equations in isotropic media", IEEE Trans. Antennas Propagat. 14, 302 (1966). [doi:10.1109/TAP.1966.1138693](https://doi.org/10.1109/TAP.1966.1138693)

- Methods pages: [averaging.md:10](methods/averaging.md?plain=1#L10), [fdfd-3d.md:10](methods/fdfd-3d.md?plain=1#L10), [fdfd.md:8](methods/fdfd.md?plain=1#L8), [fdtd.md:10](methods/fdtd.md?plain=1#L10), [travelling-wave.md:24](methods/travelling-wave.md?plain=1#L24)
- Validation cases: [`fdfd3d/two-d-agreement`](validation.md?plain=1#L163), [`fdtd/dispersion`](validation.md?plain=1#L203), [`fdtd/energy`](validation.md?plain=1#L204), [`line/full-wave-plates-order`](validation.md?plain=1#L594)
- Code: [src/electro/full_wave.rs:7](../src/electro/full_wave.rs#L7), [src/electro/full_wave_checks.rs:264](../src/electro/full_wave_checks.rs#L264), [src/fdfd/mod.rs:6](../src/fdfd/mod.rs#L6), [src/fdfd/three.rs:6](../src/fdfd/three.rs#L6), [src/fdtd.rs:4](../src/fdtd.rs#L4), [src/validation.rs:1166](../src/validation.rs#L1166), [1446](../src/validation.rs#L1446), [1453](../src/validation.rs#L1453)
- Other documents: [ROADMAP.md:715](../ROADMAP.md?plain=1#L715)

#### Karypis and Kumar 1998

G. Karypis, V. Kumar, "A Fast and High Quality Multilevel Scheme for Partitioning Irregular Graphs", SIAM J. Sci. Comput. 20, 359 (1998). [doi:10.1137/S1064827595287997](https://doi.org/10.1137/S1064827595287997)

- Other documents: [docs/plans/performance.md:627](plans/performance.md?plain=1#L627)

#### Kassem et al. 2023

M. Kassem, C. J. Benmore, A. Tverjanovich, T. Usuki, M. Khomenko, D. Fontanari, A. Sokolov, K. Ohara, M. Bokova, S. Kohara, et al., "Glassy and liquid Sb 2 S 3 : insight into the structure and dynamics of a promising functional material", J. Mater. Chem. C 11, 4654 (2023). [doi:10.1039/d3tc00081h](https://doi.org/10.1039/d3tc00081h), open access.

- Methods pages: [catalogue.md:441](methods/catalogue.md?plain=1#L441)
- Code: [src/material/catalogue/references.rs:543](../src/material/catalogue/references.rs#L543), [544](../src/material/catalogue/references.rs#L544)

#### Kato and Takaoka 2002

K. Kato, E. Takaoka, "Sellmeier and thermo-optic dispersion formulas for KTP", Appl. Opt. 41, 5040 (2002). [doi:10.1364/AO.41.005040](https://doi.org/10.1364/AO.41.005040)

- Methods pages: [catalogue.md:1073](methods/catalogue.md?plain=1#L1073)

#### Kato et al. 1994

H. Kato, S. Adachi, H. Nakanishi, K. Ohtsuka, "Optical Properties of (AlxGa1-x)0.5In0.5P Quaternary Alloys", Jpn. J. Appl. Phys. 33, 186 (1994). [doi:10.1143/JJAP.33.186](https://doi.org/10.1143/JJAP.33.186)

- Code: [src/material/catalogue/coming.rs:94](../src/material/catalogue/coming.rs#L94)

#### Kaufmann et al. 2023

F. Kaufmann, G. Finco, A. Maeder, R. Grange, "Redeposition‐free inductively‐coupled plasma etching of lithium niobate for integrated photonics", Nanophotonics 12, 1601 (2023). [doi:10.1515/nanoph-2022-0676](https://doi.org/10.1515/nanoph-2022-0676), open access.

- Methods pages: [fabrication.md:20](methods/fabrication.md?plain=1#L20)
- Code: [src/fabrication/references.rs:144](../src/fabrication/references.rs#L144), [146](../src/fabrication/references.rs#L146)
- Other documents: [docs/plans/fabrication.md:651](plans/fabrication.md?plain=1#L651)

#### Kawachi 1990

M. Kawachi, "Silica waveguides on silicon and their application to integrated-optic components", Opt Quant Electron 22, 391 (1990). [doi:10.1007/BF02113964](https://doi.org/10.1007/BF02113964)

- Academy lessons: [mach-zehnder.md:46](../academy/mach-zehnder.md?plain=1#L46), [361](../academy/mach-zehnder.md?plain=1#L361)

#### Kearney et al. 2018

B. Kearney, B. Jugdersuren, J. Culbertson, P. Desario, X. Liu, "Substrate and annealing temperature dependent electrical resistivity of sputtered titanium nitride thin films", Thin Solid Films 661, 78 (2018). [doi:10.1016/j.tsf.2018.07.001](https://doi.org/10.1016/j.tsf.2018.07.001)

- Methods pages: [catalogue.md:102](methods/catalogue.md?plain=1#L102)
- Validation cases: [`fabrication/thermal-order`](validation.md?plain=1#L747)
- Code: [src/fabrication/checks.rs:454](../src/fabrication/checks.rs#L454), [src/fabrication/references.rs:176](../src/fabrication/references.rs#L176), [src/material/catalogue/references.rs:431](../src/material/catalogue/references.rs#L431), [src/material/catalogue/resistivity.rs:9](../src/material/catalogue/resistivity.rs#L9)

#### Kennedy and Eberhart 1995

J. Kennedy, R. Eberhart, "Particle swarm optimization", Proceedings of ICNN'95 - International Conference on Neural Networks (1995). [doi:10.1109/ICNN.1995.488968](https://doi.org/10.1109/ICNN.1995.488968)

- Methods pages: [optimization.md:138](methods/optimization.md?plain=1#L138)
- Code: [src/job/optimize.rs:239](../src/job/optimize.rs#L239)
- Other documents: [docs/plans/optimization.md:579](plans/optimization.md?plain=1#L579)

#### Kennedy and Eberhart 1997

J. Kennedy, R. Eberhart, "A discrete binary version of the particle swarm algorithm", 1997 IEEE International Conference on Systems, Man, and Cybernetics. Computational Cybernetics and Simulation (1997). [doi:10.1109/ICSMC.1997.637339](https://doi.org/10.1109/ICSMC.1997.637339)

- Other documents: [docs/plans/optimization.md:580](plans/optimization.md?plain=1#L580)

#### Kepič et al. 2024

P. Kepič, P. Liška, B. Idesová, O. Caha, F. Ligmajer, T. Šikola, "Pulsed laser deposition of Sb2S3 films for phase-change tunable nanophotonics", New J. Phys. 26, 013005 (2024). [doi:10.1088/1367-2630/ad1696](https://doi.org/10.1088/1367-2630/ad1696), open access.

- Methods pages: [catalogue.md:451](methods/catalogue.md?plain=1#L451)
- Validation cases: [`material/pcm-sb2s3-kaito-deficit`](validation.md?plain=1#L61)
- Code: [src/material/catalogue/references.rs:67](../src/material/catalogue/references.rs#L67), [68](../src/material/catalogue/references.rs#L68), [src/validation.rs:452](../src/validation.rs#L452)

#### Kestyn et al. 2016

J. Kestyn, E. Polizzi, P. T. Peter Tang, "Feast Eigensolver for Non-Hermitian Problems", SIAM J. Sci. Comput. 38, S772 (2016). [doi:10.1137/15M1026572](https://doi.org/10.1137/15M1026572)

- Methods pages: [contour.md:12](methods/contour.md?plain=1#L12)
- Academy lessons: [leaky-and-bent-guides.md:131](../academy/leaky-and-bent-guides.md?plain=1#L131), [381](../academy/leaky-and-bent-guides.md?plain=1#L381), [581](../academy/leaky-and-bent-guides.md?plain=1#L581)
- Validation cases: [`mode/contour-leaky-shift-invert`](validation.md?plain=1#L464)
- Code: [src/eigen/contour.rs:6](../src/eigen/contour.rs#L6), [src/mode/region.rs:12](../src/mode/region.rs#L12), [src/mode/region/checks.rs:366](../src/mode/region/checks.rs#L366)
- Other documents: [docs/plans/performance.md:628](plans/performance.md?plain=1#L628)

#### Kharel et al. 2021a

P. Kharel, C. Reimer, K. Luke, L. He, M. Zhang, "Breaking voltage-bandwidth limits in integrated lithium niobate modulators using micro-structured electrodes: erratum", Optica 8, 1218 (2021). [doi:10.1364/OPTICA.440484](https://doi.org/10.1364/OPTICA.440484)

- Methods pages: [travelling-wave.md:20](methods/travelling-wave.md?plain=1#L20)
- Validation cases: [`line/kharel-2021-impedance`](validation.md?plain=1#L572), [`line/kharel-2021-quartz-index`](validation.md?plain=1#L586)
- Code: [crosscode/src/t_rails.rs:4](../crosscode/src/t_rails.rs#L4), [src/cases/electro/line_checks.rs:1000](../src/cases/electro/line_checks.rs#L1000), [1098](../src/cases/electro/line_checks.rs#L1098)

#### Kharel et al. 2021b

P. Kharel, C. Reimer, K. Luke, L. He, M. Zhang, "Breaking voltage–bandwidth limits in integrated lithium niobate modulators using micro-structured electrodes", Optica 8, 357 (2021). [doi:10.1364/OPTICA.416155](https://doi.org/10.1364/OPTICA.416155)

- Methods pages: [devices.md:54](methods/devices.md?plain=1#L54), [electro-optic.md:184](methods/electro-optic.md?plain=1#L184), [travelling-wave.md:18](methods/travelling-wave.md?plain=1#L18)
- Validation cases: [`line/kharel-2021-impedance`](validation.md?plain=1#L572), [`line/kharel-2021-quartz-index`](validation.md?plain=1#L586), [`device/tfln-kharel-loss`](validation.md?plain=1#L692)
- Code: [crosscode/src/t_rails.rs:3](../crosscode/src/t_rails.rs#L3), [src/cases/device/tfln_checks.rs:354](../src/cases/device/tfln_checks.rs#L354), [src/cases/electro/line_checks.rs:1000](../src/cases/electro/line_checks.rs#L1000), [1098](../src/cases/electro/line_checks.rs#L1098), [src/device/measured/kharel-2021.toml:6](../src/device/measured/kharel-2021.toml#L6)
- Jobs and circuits: [jobs/tfln-segmented.yaml:2](../jobs/tfln-segmented.yaml#L2)
- Other documents: [ROADMAP.md:783](../ROADMAP.md?plain=1#L783)

#### Killoran et al. 2019

N. Killoran, J. Izaac, N. Quesada, V. Bergholm, M. Amy, C. Weedbrook, "Strawberry Fields: A Software Platform for Photonic Quantum Computing", Quantum 3, 129 (2019). [doi:10.22331/q-2019-03-11-129](https://doi.org/10.22331/q-2019-03-11-129), open access.

- Other documents: [docs/plans/quantum.md:542](plans/quantum.md?plain=1#L542)

#### Kippenberg et al. 2018

T. J. Kippenberg, A. L. Gaeta, M. Lipson, M. L. Gorodetsky, "Dissipative Kerr solitons in optical microresonators", Science 361, eaan8083 (2018). [doi:10.1126/science.aan8083](https://doi.org/10.1126/science.aan8083)

- Methods pages: [nonlinear.md:42](methods/nonlinear.md?plain=1#L42)
- Academy lessons: [ring-resonator.md:97](../academy/ring-resonator.md?plain=1#L97), [126](../academy/ring-resonator.md?plain=1#L126), [401](../academy/ring-resonator.md?plain=1#L401), [590](../academy/ring-resonator.md?plain=1#L590)
- Code: [src/nonlinear/resonator.rs:12](../src/nonlinear/resonator.rs#L12)
- Other documents: [ROADMAP.md:835](../ROADMAP.md?plain=1#L835)

#### Knill et al. 2001

E. Knill, R. Laflamme, G. J. Milburn, "A scheme for efficient quantum computation with linear optics", Nature 409, 46 (2001). [doi:10.1038/35051009](https://doi.org/10.1038/35051009)

- Other documents: [docs/plans/quantum.md:486](plans/quantum.md?plain=1#L486)

#### Kogelnik 1975

H. Kogelnik, "2. Theory of dielectric waveguides", Topics in Applied Physics, Springer Berlin Heidelberg (1975). [doi:10.1007/BFb0103618](https://doi.org/10.1007/BFb0103618)

- Academy lessons: [slab-waveguide.md:54](../academy/slab-waveguide.md?plain=1#L54), [570](../academy/slab-waveguide.md?plain=1#L570)

#### Kogelnik and Ramaswamy 1974

H. Kogelnik, V. Ramaswamy, "Scaling Rules for Thin-Film Optical Waveguides", Appl. Opt. 13, 1857 (1974). [doi:10.1364/AO.13.001857](https://doi.org/10.1364/AO.13.001857)

- Academy lessons: [slab-waveguide.md:48](../academy/slab-waveguide.md?plain=1#L48), [306](../academy/slab-waveguide.md?plain=1#L306), [568](../academy/slab-waveguide.md?plain=1#L568), [597](../academy/slab-waveguide.md?plain=1#L597)
- Code: [studio/src-tauri/src/charts/slab.rs:12](../studio/src-tauri/src/charts/slab.rs#L12)

#### Kogelnik and Schmidt 1976

H. Kogelnik, R. Schmidt, "Switched directional couplers with alternating ΔΒ", IEEE J. Quantum Electron. 12, 396 (1976). [doi:10.1109/JQE.1976.1069190](https://doi.org/10.1109/JQE.1976.1069190)

- Academy lessons: [directional-coupler.md:46](../academy/directional-coupler.md?plain=1#L46), [389](../academy/directional-coupler.md?plain=1#L389)

#### Kogelnik and Shank 1972

H. Kogelnik, C. V. Shank, "Coupled-Wave Theory of Distributed Feedback Lasers", Journal of Applied Physics 43, 2327 (1972). [doi:10.1063/1.1661499](https://doi.org/10.1063/1.1661499)

- Academy lessons: [bragg-gratings.md:37](../academy/bragg-gratings.md?plain=1#L37), [238](../academy/bragg-gratings.md?plain=1#L238)

#### Komma et al. 2012

J. Komma, C. Schwarz, G. Hofmann, D. Heinert, R. Nawrodt, "Thermo-optic coefficient of silicon at 1550 nm and cryogenic temperatures", Appl. Phys. Lett. 101, 041905 (2012). [doi:10.1063/1.4738989](https://doi.org/10.1063/1.4738989), open access.

- Methods pages: [catalogue.md:14](methods/catalogue.md?plain=1#L14)
- Validation cases: [`material/silicon-komma-reading`](validation.md?plain=1#L25), [`material/silicon-komma-frey-dndt`](validation.md?plain=1#L26), [`material/silicon-komma-room`](validation.md?plain=1#L27), [`material/silicon-dndt-positive`](validation.md?plain=1#L28)
- Code: [src/material/catalogue/references.rs:102](../src/material/catalogue/references.rs#L102), [103](../src/material/catalogue/references.rs#L103), [src/material/catalogue/thermo.rs:10](../src/material/catalogue/thermo.rs#L10), [262](../src/material/catalogue/thermo.rs#L262), [src/validation.rs:200](../src/validation.rs#L200), [207](../src/validation.rs#L207), [214](../src/validation.rs#L214), [221](../src/validation.rs#L221)
- Other documents: [ROADMAP.md:801](../ROADMAP.md?plain=1#L801)

#### Konno et al. 2024

S. Konno, W. Asavanant, F. Hanamura, H. Nagayoshi, K. Fukui, A. Sakaguchi, R. Ide, F. China, M. Yabuno, S. Miki, et al., "Logical states for fault-tolerant quantum computation with propagating light", Science 383, 289 (2024). [doi:10.1126/science.adk7560](https://doi.org/10.1126/science.adk7560)

- Other documents: [docs/plans/quantum.md:517](plans/quantum.md?plain=1#L517)

#### Kottke et al. 2008

C. Kottke, A. Farjadpour, S. G. Johnson, "Perturbation theory for anisotropic dielectric interfaces, and application to subpixel smoothing of discretized numerical methods", Phys. Rev. E 77, 036611 (2008). [doi:10.1103/PhysRevE.77.036611](https://doi.org/10.1103/PhysRevE.77.036611)

- Methods pages: [fdtd.md:34](methods/fdtd.md?plain=1#L34)
- Validation cases: [`fdtd/smoothing-anisotropic-slab`](validation.md?plain=1#L228)
- Code: [src/fdtd/smoothing.rs:10](../src/fdtd/smoothing.rs#L10), [src/validation.rs:1621](../src/validation.rs#L1621)
- Other documents: [ROADMAP.md:719](../ROADMAP.md?plain=1#L719)

#### Krasnokutska et al. 2018

I. Krasnokutska, J.-L. J. Tambasco, X. Li, A. Peruzzo, "Ultra-low loss photonic circuits in lithium niobate on insulator", Opt. Express 26, 897 (2018). [doi:10.1364/OE.26.000897](https://doi.org/10.1364/OE.26.000897)

- Code: [src/fabrication/references.rs:136](../src/fabrication/references.rs#L136), [138](../src/fabrication/references.rs#L138)
- Other documents: [docs/plans/fabrication.md:648](plans/fabrication.md?plain=1#L648)

#### Kremer et al. 2004

R. Kremer, K. Graf, M. Cardona, G. Devyatykh, A. Gusev, A. Gibin, A. Inyushkin, A. Taldenkov, H.-J. Pohl, "Thermal conductivity of isotopically enriched 28Si: revisited", Solid State Communications 131, 499 (2004). [doi:10.1016/j.ssc.2004.06.022](https://doi.org/10.1016/j.ssc.2004.06.022)

- Methods pages: [catalogue.md:104](methods/catalogue.md?plain=1#L104)
- Validation cases: [`material/silicon-kremer-ho`](validation.md?plain=1#L34)
- Code: [src/material/catalogue/conductivity.rs:6](../src/material/catalogue/conductivity.rs#L6), [src/material/catalogue/references.rs:438](../src/material/catalogue/references.rs#L438), [src/validation.rs:263](../src/validation.rs#L263)

#### Krishnan et al. 2009

M. Krishnan, J. W. Nalaskowski, L. M. Cook, "Chemical Mechanical Planarization: Slurry Chemistry, Materials, and Mechanisms", Chem. Rev. 110, 178 (2009). [doi:10.1021/cr900170z](https://doi.org/10.1021/cr900170z)

- Methods pages: [fabrication.md:28](methods/fabrication.md?plain=1#L28)
- Validation cases: [`fabrication/accuracy-combined`](validation.md?plain=1#L753)
- Code: [src/fabrication/references.rs:88](../src/fabrication/references.rs#L88), [src/fabrication/rule_checks.rs:351](../src/fabrication/rule_checks.rs#L351)
- Other documents: [docs/plans/fabrication.md:629](plans/fabrication.md?plain=1#L629)

#### Krupka et al. 2006

J. Krupka, J. Breeze, A. Centeno, N. Alford, T. Claussen, L. Jensen, "Measurements of Permittivity, Dielectric Loss Tangent, and Resistivity of Float-Zone Silicon at Microwave Frequencies", IEEE Trans. Microwave Theory Techn. 54, 3995 (2006). [doi:10.1109/TMTT.2006.883655](https://doi.org/10.1109/TMTT.2006.883655)

- Methods pages: [catalogue.md:116](methods/catalogue.md?plain=1#L116)
- Validation cases: [`material/rf-krupka-table-1`](validation.md?plain=1#L43)
- Code: [src/material/catalogue/references.rs:641](../src/material/catalogue/references.rs#L641), [src/material/catalogue/rf.rs:9](../src/material/catalogue/rf.rs#L9), [src/validation.rs:326](../src/validation.rs#L326)

### L

#### Lalau-Keraly et al. 2013

C. M. Lalau-Keraly, S. Bhargava, O. D. Miller, E. Yablonovitch, "Adjoint shape optimization applied to electromagnetic design", Opt. Express 21, 21693 (2013). [doi:10.1364/OE.21.021693](https://doi.org/10.1364/OE.21.021693)

- Methods pages: [fdfd-adjoint.md:10](methods/fdfd-adjoint.md?plain=1#L10), [fdtd-adjoint.md:10](methods/fdtd-adjoint.md?plain=1#L10)
- Validation cases: [`fdtd/adjoint-modes`](validation.md?plain=1#L268), [`fdtd/adjoint-backward-mode`](validation.md?plain=1#L274)
- Examples: [README.md:58](../examples/README.md?plain=1#L58), [optimize_ybranch.rs:9](../examples/optimize_ybranch.rs#L9)
- Code: [src/fdtd.rs:56](../src/fdtd.rs#L56), [src/fdtd/adjoint.rs:27](../src/fdtd/adjoint.rs#L27), [src/fdtd/adjoint_checks.rs:543](../src/fdtd/adjoint_checks.rs#L543), [src/validation.rs:1901](../src/validation.rs#L1901), [1943](../src/validation.rs#L1943)
- Other documents: [ROADMAP.md:759](../ROADMAP.md?plain=1#L759), [docs/plans/optimization.md:589](plans/optimization.md?plain=1#L589)

#### Lamb 1996

J. W. Lamb, "Miscellaneous data on materials for millimetre and submillimetre optics", Int J Infrared Milli Waves 17, 1997 (1996). [doi:10.1007/BF02069487](https://doi.org/10.1007/BF02069487)

- Methods pages: [catalogue.md:118](methods/catalogue.md?plain=1#L118)
- Validation cases: [`material/rf-lamb-table-1`](validation.md?plain=1#L44), [`line/job-catalogue-tangents`](validation.md?plain=1#L592)
- Code: [src/job/line_checks.rs:87](../src/job/line_checks.rs#L87), [src/material/catalogue/references.rs:648](../src/material/catalogue/references.rs#L648), [src/material/catalogue/rf.rs:12](../src/material/catalogue/rf.rs#L12), [src/validation.rs:333](../src/validation.rs#L333)

#### Lanczos 1964

C. Lanczos, "A Precision Approximation of the Gamma Function", Journal of the Society for Industrial and Applied Mathematics Series B Numerical Analysis 1, 86 (1964). [doi:10.1137/0701008](https://doi.org/10.1137/0701008)

- Methods pages: [geometry.md:10](methods/geometry.md?plain=1#L10)
- Validation cases: [`geometry/areas`](validation.md?plain=1#L393), [`geometry/curves/closed-forms`](validation.md?plain=1#L412)
- Code: [src/geometry/checks.rs:220](../src/geometry/checks.rs#L220), [src/geometry/curve_checks.rs:679](../src/geometry/curve_checks.rs#L679)

#### Larsen et al. 2025

M. V. Larsen, J. E. Bourassa, S. Kocsis, J. F. Tasker, R. S. Chadwick, C. González-Arciniegas, J. Hastrup, C. E. Lopetegui-González, F. M. Miatto, A. Motamedi, et al., "Integrated photonic source of Gottesman–Kitaev–Preskill qubits", Nature 642, 587 (2025). [doi:10.1038/s41586-025-09044-5](https://doi.org/10.1038/s41586-025-09044-5), open access.

- Other documents: [docs/plans/quantum.md:518](plans/quantum.md?plain=1#L518)

#### Lawson and Hanson 1995

C. L. Lawson, R. J. Hanson, "Solving Least Squares Problems", Society for Industrial and Applied Mathematics (1995). [doi:10.1137/1.9781611971217](https://doi.org/10.1137/1.9781611971217)

- Methods pages: [fdtd.md:20](methods/fdtd.md?plain=1#L20)
- Validation cases: [`fdtd/fit-lossless`](validation.md?plain=1#L224)
- Code: [src/fdtd/media.rs:42](../src/fdtd/media.rs#L42), [src/validation.rs:1593](../src/validation.rs#L1593)

#### Lawson et al. 2022

D. Lawson, D. W. Hewak, O. L. Muskens, I. Zeimpekis, "Time-resolved reversible optical switching of the ultralow-loss phase change material Sb2Se3", J. Opt. 24, 064013 (2022). [doi:10.1088/2040-8986/ac5ece](https://doi.org/10.1088/2040-8986/ac5ece), open access.

- Validation cases: [`material/pcm-sb2se3-crystallization`](validation.md?plain=1#L52)
- Code: [src/material/catalogue/references.rs:25](../src/material/catalogue/references.rs#L25), [src/validation.rs:389](../src/validation.rs#L389)

#### Lee 2001

M. Lee, "Dielectric constant and loss tangent in LiNbO3 crystals from 90 to 147 GHz", Applied Physics Letters 79, 1342 (2001). [doi:10.1063/1.1399305](https://doi.org/10.1063/1.1399305)

- Methods pages: [catalogue.md:114](methods/catalogue.md?plain=1#L114)
- Code: [src/material/catalogue/references.rs:634](../src/material/catalogue/references.rs#L634), [src/material/catalogue/rf.rs:6](../src/material/catalogue/rf.rs#L6)

#### Lee and Fan 1974

C. C. Lee, H. Y. Fan, "Second-harmonic generation in InSb, InP, and AlSb", Phys. Rev. B 10, 703 (1974). [doi:10.1103/PhysRevB.10.703](https://doi.org/10.1103/PhysRevB.10.703)

- Code: [src/material/catalogue/references.rs:361](../src/material/catalogue/references.rs#L361)

#### Lee et al. 2001

K. K. Lee, D. R. Lim, L. C. Kimerling, J. Shin, F. Cerrina, "Fabrication of ultralow-loss Si/SiO\_2 waveguides by roughness reduction", Opt. Lett. 26, 1888 (2001). [doi:10.1364/OL.26.001888](https://doi.org/10.1364/OL.26.001888)

- Academy lessons: [strip-waveguide.md:70](../academy/strip-waveguide.md?plain=1#L70), [617](../academy/strip-waveguide.md?plain=1#L617), [683](../academy/strip-waveguide.md?plain=1#L683)
- Other documents: [docs/plans/fabrication.md:638](plans/fabrication.md?plain=1#L638)

#### Leidinger et al. 2015

M. Leidinger, S. Fieberg, N. Waasem, F. Kühnemann, K. Buse, I. Breunig, "Comparative study on three highly sensitive absorption measurement techniques characterizing lithium niobate over its entire transparent spectral range", Opt. Express 23, 21690 (2015). [doi:10.1364/OE.23.021690](https://doi.org/10.1364/OE.23.021690)

- Methods pages: [catalogue.md:40](methods/catalogue.md?plain=1#L40)
- Code: [src/material/catalogue/references.rs:620](../src/material/catalogue/references.rs#L620), [621](../src/material/catalogue/references.rs#L621), [src/material/catalogue/transparency.rs:15](../src/material/catalogue/transparency.rs#L15)
- Other documents: [ROADMAP.md:857](../ROADMAP.md?plain=1#L857)

#### Leviton and Frey 2006

D. B. Leviton, B. J. Frey, "Temperature-dependent absolute refractive index measurements of synthetic fused silica", SPIE Proceedings (2006). [doi:10.1117/12.672853](https://doi.org/10.1117/12.672853)

- Methods pages: [catalogue.md:10](methods/catalogue.md?plain=1#L10)
- Academy lessons: [material-dispersion.md:50](../academy/material-dispersion.md?plain=1#L50), [403](../academy/material-dispersion.md?plain=1#L403), [447](../academy/material-dispersion.md?plain=1#L447)
- Validation cases: [`material/silicon-frey-li`](validation.md?plain=1#L14), [`material/silica-leviton-table`](validation.md?plain=1#L18), [`material/silica-leviton-dndt`](validation.md?plain=1#L19)
- Code: [src/material/catalogue/models.rs:149](../src/material/catalogue/models.rs#L149), [src/material/catalogue/references.rs:116](../src/material/catalogue/references.rs#L116), [src/material/catalogue/thermo.rs:13](../src/material/catalogue/thermo.rs#L13), [367](../src/material/catalogue/thermo.rs#L367), [src/validation.rs:123](../src/validation.rs#L123), [151](../src/validation.rs#L151), [158](../src/validation.rs#L158)

#### Levy et al. 1998

M. Levy, R. M. Osgood, R. Liu, L. E. Cross, G. S. Cargill, A. Kumar, H. Bakhru, "Fabrication of single-crystal lithium niobate films by crystal ion slicing", Applied Physics Letters 73, 2293 (1998). [doi:10.1063/1.121801](https://doi.org/10.1063/1.121801)

- Code: [src/fabrication/references.rs:104](../src/fabrication/references.rs#L104)
- Other documents: [docs/plans/fabrication.md:644](plans/fabrication.md?plain=1#L644)

#### Li 1980

H. H. Li, "Refractive index of silicon and germanium and its wavelength and temperature derivatives", Journal of Physical and Chemical Reference Data 9, 561 (1980). [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624)

- Methods pages: [catalogue.md:8](methods/catalogue.md?plain=1#L8), [materials.md:8](methods/materials.md?plain=1#L8)
- Academy lessons: [material-dispersion.md:38](../academy/material-dispersion.md?plain=1#L38), [89](../academy/material-dispersion.md?plain=1#L89), [109](../academy/material-dispersion.md?plain=1#L109), [277](../academy/material-dispersion.md?plain=1#L277), [439](../academy/material-dispersion.md?plain=1#L439), [459](../academy/material-dispersion.md?plain=1#L459)
- Validation cases: [`material/silicon-li-table`](validation.md?plain=1#L12), [`material/silicon-frey-li`](validation.md?plain=1#L14), [`material/silicon-li-table-1`](validation.md?plain=1#L20), [`material/silicon-li-table-2`](validation.md?plain=1#L21), [`material/silicon-li-frey-dndt`](validation.md?plain=1#L24), [`material/silicon-komma-room`](validation.md?plain=1#L27)
- Examples: [README.md:34](../examples/README.md?plain=1#L34), [silicon_index.rs:5](../examples/silicon_index.rs#L5), [python/examples/README.md:26](../python/examples/README.md?plain=1#L26), [python/examples/output/silicon_index.txt:7](../python/examples/output/silicon_index.txt#L7), [python/examples/silicon_index.py:5](../python/examples/silicon_index.py#L5)
- Code: [python/tests/test_api.py:102](../python/tests/test_api.py#L102), [361](../python/tests/test_api.py#L361), [src/facade.rs:123](../src/facade.rs#L123), [src/facade/tests.rs:122](../src/facade/tests.rs#L122), [src/material/catalogue/references.rs:88](../src/material/catalogue/references.rs#L88), [src/material/catalogue/thermo.rs:6](../src/material/catalogue/thermo.rs#L6), [src/material/mod.rs:727](../src/material/mod.rs#L727), [785](../src/material/mod.rs#L785), [src/material/refractiveindex.rs:362](../src/material/refractiveindex.rs#L362), [598](../src/material/refractiveindex.rs#L598), [599](../src/material/refractiveindex.rs#L599), [src/validation.rs:109](../src/validation.rs#L109), [123](../src/validation.rs#L123), [165](../src/validation.rs#L165), [172](../src/validation.rs#L172), [193](../src/validation.rs#L193), [214](../src/validation.rs#L214), [studio/src-tauri/src/charts/material.rs:629](../studio/src-tauri/src/charts/material.rs#L629)
- Other documents: [ROADMAP.md:743](../ROADMAP.md?plain=1#L743)

#### Li 1996

L. Li, "Use of Fourier series in the analysis of discontinuous periodic structures", J. Opt. Soc. Am. A 13, 1870 (1996). [doi:10.1364/JOSAA.13.001870](https://doi.org/10.1364/JOSAA.13.001870)

- Other documents: [ROADMAP.md:732](../ROADMAP.md?plain=1#L732)

#### Li and Demmel 2003

X. S. Li, J. W. Demmel, "SuperLU\_DIST: A scalable distributed-memory sparse direct solver for unsymmetric linear systems", ACM Trans. Math. Softw. 29, 110 (2003). [doi:10.1145/779359.779361](https://doi.org/10.1145/779359.779361)

- Methods pages: [fdfd-3d.md:38](methods/fdfd-3d.md?plain=1#L38), [354](methods/fdfd-3d.md?plain=1#L354), [fdfd.md:20](methods/fdfd.md?plain=1#L20)
- Validation cases: [`fdfd3d/ldlt-lu`](validation.md?plain=1#L174)
- Code: [src/sparse/multifrontal.rs:6](../src/sparse/multifrontal.rs#L6), [src/validation.rs:1243](../src/validation.rs#L1243)

#### Liebig et al. 2012

T. Liebig, A. Rennings, S. Held, D. Erni, "openEMS – a free and open source equivalent‐circuit (EC) FDTD simulation platform supporting cylindrical coordinates suitable for the analysis of traveling wave MRI applications", Int J Numerical Modelling 26, 680 (2012). [doi:10.1002/jnm.1875](https://doi.org/10.1002/jnm.1875)

- Code: [crosscode/src/openems.rs:6](../crosscode/src/openems.rs#L6)
- Other documents: [docs/cross-code.md:107](cross-code.md?plain=1#L107)

#### Lin et al. 2007

Q. Lin, O. J. Painter, G. P. Agrawal, "Nonlinear optical phenomena in silicon waveguides: modeling and applications", Opt. Express 15, 16604 (2007). [doi:10.1364/OE.15.016604](https://doi.org/10.1364/OE.15.016604)

- Validation cases: [`nonlinear/nlse-tpa-power`](validation.md?plain=1#L799), [`nonlinear/nlse-tpa-phase`](validation.md?plain=1#L800), [`nonlinear/nlse-carriers-gaussian`](validation.md?plain=1#L802), [`nonlinear/nlse-fca-cw-power`](validation.md?plain=1#L803)
- Code: [src/nonlinear/absorption_checks.rs:261](../src/nonlinear/absorption_checks.rs#L261), [268](../src/nonlinear/absorption_checks.rs#L268), [282](../src/nonlinear/absorption_checks.rs#L282), [289](../src/nonlinear/absorption_checks.rs#L289)
- Other documents: [ROADMAP.md:853](../ROADMAP.md?plain=1#L853)

#### Little et al. 1997

B. Little, S. Chu, H. Haus, J. Foresi, J.-P. Laine, "Microring resonator channel dropping filters", J. Lightwave Technol. 15, 998 (1997). [doi:10.1109/50.588673](https://doi.org/10.1109/50.588673)

- Methods pages: [devices.md:12](methods/devices.md?plain=1#L12), [optimization.md:260](methods/optimization.md?plain=1#L260)
- Academy lessons: [README.md:61](../academy/README.md?plain=1#L61), [123](../academy/README.md?plain=1#L123), [ring-resonator.md:31](../academy/ring-resonator.md?plain=1#L31), [595](../academy/ring-resonator.md?plain=1#L595)
- Examples: [README.md:51](../examples/README.md?plain=1#L51), [55](../examples/README.md?plain=1#L55), [optimize_chip_rings.rs:8](../examples/optimize_chip_rings.rs#L8), [optimize_ring_flat.rs:8](../examples/optimize_ring_flat.rs#L8)
- Code: [src/device/ring.rs:289](../src/device/ring.rs#L289)
- Other documents: [docs/plans/optimization.md:620](plans/optimization.md?plain=1#L620)

#### Liu 1990

J. W. Liu, "The Role of Elimination Trees in Sparse Factorization", SIAM J. Matrix Anal. Appl. 11, 134 (1990). [doi:10.1137/0611010](https://doi.org/10.1137/0611010)

- Methods pages: [fdfd-3d.md:34](methods/fdfd-3d.md?plain=1#L34), [351](methods/fdfd-3d.md?plain=1#L351)
- Code: [src/sparse/multifrontal.rs:19](../src/sparse/multifrontal.rs#L19)

#### Liu and Fan 2012

V. Liu, S. Fan, "S4 : A free electromagnetic solver for layered periodic structures", Computer Physics Communications 183, 2233 (2012). [doi:10.1016/j.cpc.2012.04.026](https://doi.org/10.1016/j.cpc.2012.04.026)

- Other documents: [ROADMAP.md:733](../ROADMAP.md?plain=1#L733)

#### Liu and Poon 2025

Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025). [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107)

- Methods pages: [fdtd.md:52](methods/fdtd.md?plain=1#L52), [1067](methods/fdtd.md?plain=1#L1067)
- Academy lessons: [directional-coupler.md:112](../academy/directional-coupler.md?plain=1#L112), [406](../academy/directional-coupler.md?plain=1#L406), [511](../academy/directional-coupler.md?plain=1#L511)
- Examples: [README.md:70](../examples/README.md?plain=1#L70), [coupler_liu_poon.rs:5](../examples/coupler_liu_poon.rs#L5), [crossing_liu_poon.rs:5](../examples/crossing_liu_poon.rs#L5), [mmi_liu_poon.rs:5](../examples/mmi_liu_poon.rs#L5), [mode_converter_liu_poon.rs:5](../examples/mode_converter_liu_poon.rs#L5), [pdk/mod.rs:7](../examples/pdk/mod.rs#L7), [ring_liu_poon.rs:5](../examples/ring_liu_poon.rs#L5), [splitter_rotator_liu_poon.rs:5](../examples/splitter_rotator_liu_poon.rs#L5)

#### Liu et al. 2004

A. Liu, R. Jones, L. Liao, D. Samara-Rubio, D. Rubin, O. Cohen, R. Nicolaescu, M. Paniccia, "A high-speed silicon optical modulator based on a metal–oxide–semiconductor capacitor", Nature 427, 615 (2004). [doi:10.1038/nature02310](https://doi.org/10.1038/nature02310)

- Academy lessons: [mach-zehnder.md:76](../academy/mach-zehnder.md?plain=1#L76), [366](../academy/mach-zehnder.md?plain=1#L366)

#### Liu et al. 2018

D. Liu, Y. Tan, E. Khoram, Z. Yu, "Training Deep Neural Networks for the Inverse Design of Nanophotonic Structures", ACS Photonics 5, 1365 (2018). [doi:10.1021/acsphotonics.7b01377](https://doi.org/10.1021/acsphotonics.7b01377)

- Other documents: [docs/plans/optimization.md:634](plans/optimization.md?plain=1#L634)

#### Liu et al. 2026

H.-L. Liu, H. Su, Y.-H. Deng, S.-Q. Gong, Y.-C. Gu, H.-Y. Tang, M.-H. Jia, Q. Wei, Y.-K. Song, D.-Z. Wang, et al., "Gaussian boson sampling with 1,024 squeezed states in 8,176 modes", Nature 653, 687 (2026). [doi:10.1038/s41586-026-10523-6](https://doi.org/10.1038/s41586-026-10523-6)

- Other documents: [docs/plans/quantum.md:507](plans/quantum.md?plain=1#L507)

#### Loh et al. 2009

P.-R. Loh, A. F. Oskooi, M. Ibanescu, M. Skorobogatiy, S. G. Johnson, "Fundamental relation between phase and group velocity, and application to the failure of perfectly matched layers in backward-wave structures", Phys. Rev. E 79, 065601 (2009). [doi:10.1103/PhysRevE.79.065601](https://doi.org/10.1103/PhysRevE.79.065601)

- Methods pages: [fdtd.md:28](methods/fdtd.md?plain=1#L28)
- Validation cases: [`fdtd/cpml-late-growth`](validation.md?plain=1#L208)
- Code: [src/validation.rs:1481](../src/validation.rs#L1481)

#### Lorensen and Cline 1987

W. E. Lorensen, H. E. Cline, "Marching cubes: A high resolution 3D surface construction algorithm", SIGGRAPH Comput. Graph. 21, 163 (1987). [doi:10.1145/37402.37422](https://doi.org/10.1145/37402.37422)

- Methods pages: [geometry.md:25](methods/geometry.md?plain=1#L25)
- Code: [src/geometry/implicit.rs:5](../src/geometry/implicit.rs#L5)

#### Lu and Vučković 2013

J. Lu, J. Vučković, "Nanophotonic computational design", Opt. Express 21, 13351 (2013). [doi:10.1364/OE.21.013351](https://doi.org/10.1364/OE.21.013351)

- Other documents: [ROADMAP.md:758](../ROADMAP.md?plain=1#L758)

#### Lu et al. 2015

Z. Lu, H. Yun, Y. Wang, Z. Chen, F. Zhang, N. A. F. Jaeger, L. Chrostowski, "Broadband silicon photonic directional coupler using asymmetric-waveguide based phase control", Opt. Express 23, 3795 (2015). [doi:10.1364/OE.23.003795](https://doi.org/10.1364/OE.23.003795)

- Academy lessons: [directional-coupler.md:94](../academy/directional-coupler.md?plain=1#L94), [333](../academy/directional-coupler.md?plain=1#L333), [501](../academy/directional-coupler.md?plain=1#L501)

#### Lu et al. 2017

Z. Lu, J. Jhoja, J. Klein, X. Wang, A. Liu, J. Flueckiger, J. Pond, L. Chrostowski, "Performance prediction for silicon photonics integrated circuits with layout-dependent correlated manufacturing variability", Opt. Express 25, 9712 (2017). [doi:10.1364/OE.25.009712](https://doi.org/10.1364/OE.25.009712)

- Other documents: [ROADMAP.md:825](../ROADMAP.md?plain=1#L825)

#### Lugiato and Lefever 1987

L. A. Lugiato, R. Lefever, "Spatial Dissipative Structures in Passive Optical Systems", Phys. Rev. Lett. 58, 2209 (1987). [doi:10.1103/PhysRevLett.58.2209](https://doi.org/10.1103/PhysRevLett.58.2209)

- Methods pages: [nonlinear.md:36](methods/nonlinear.md?plain=1#L36)
- Validation cases: [`nonlinear/lle-homogeneous-lower`](validation.md?plain=1#L794), [`nonlinear/lle-homogeneous-upper`](validation.md?plain=1#L795), [`nonlinear/lle-instability-rate`](validation.md?plain=1#L796)
- Code: [src/nonlinear/kerr_checks.rs:756](../src/nonlinear/kerr_checks.rs#L756), [763](../src/nonlinear/kerr_checks.rs#L763), [770](../src/nonlinear/kerr_checks.rs#L770), [src/nonlinear/resonator.rs:4](../src/nonlinear/resonator.rs#L4)
- Other documents: [ROADMAP.md:834](../ROADMAP.md?plain=1#L834)

#### Luke et al. 2013

K. Luke, A. Dutt, C. B. Poitras, M. Lipson, "Overcoming Si\_3N\_4 film stress limitations for high quality factor ring resonators", Opt. Express 21, 22829 (2013). [doi:10.1364/OE.21.022829](https://doi.org/10.1364/OE.21.022829)

- Methods pages: [fabrication.md:38](methods/fabrication.md?plain=1#L38)
- Validation cases: [`fabrication/nitride-rules`](validation.md?plain=1#L756), [`fabrication/nitride-reported-flows`](validation.md?plain=1#L757)
- Code: [src/fabrication/platform_checks.rs:721](../src/fabrication/platform_checks.rs#L721), [728](../src/fabrication/platform_checks.rs#L728), [src/fabrication/references.rs:248](../src/fabrication/references.rs#L248), [250](../src/fabrication/references.rs#L250)
- Other documents: [docs/plans/fabrication.md:657](plans/fabrication.md?plain=1#L657)

#### Luke et al. 2015

K. Luke, Y. Okawachi, M. R. E. Lamont, A. L. Gaeta, M. Lipson, "Broadband mid-infrared frequency comb generation in a Si\_3N\_4 microresonator", Opt. Lett. 40, 4823 (2015). [doi:10.1364/OL.40.004823](https://doi.org/10.1364/OL.40.004823)

- Methods pages: [materials.md:12](methods/materials.md?plain=1#L12)
- Academy lessons: [material-dispersion.md:62](../academy/material-dispersion.md?plain=1#L62), [219](../academy/material-dispersion.md?plain=1#L219), [450](../academy/material-dispersion.md?plain=1#L450)
- Code: [src/material/catalogue/references.rs:123](../src/material/catalogue/references.rs#L123), [src/material/mod.rs:837](../src/material/mod.rs#L837), [861](../src/material/mod.rs#L861), [src/material/refractiveindex.rs:350](../src/material/refractiveindex.rs#L350)
- Other documents: [ROADMAP.md:745](../ROADMAP.md?plain=1#L745)

#### Lundow and Markström 2022

P. Lundow, K. Markström, "Efficient computation of permanents, with applications to Boson sampling and random matrices", Journal of Computational Physics 455, 110990 (2022). [doi:10.1016/j.jcp.2022.110990](https://doi.org/10.1016/j.jcp.2022.110990), open access.

- Other documents: [docs/plans/quantum.md:497](plans/quantum.md?plain=1#L497)

#### Lyeo et al. 2006

H.-K. Lyeo, D. G. Cahill, B.-S. Lee, J. R. Abelson, M.-H. Kwon, K.-B. Kim, S. G. Bishop, B.-K. Cheong, "Thermal conductivity of phase-change material Ge2Sb2Te5", Applied Physics Letters 89, 151904 (2006). [doi:10.1063/1.2359354](https://doi.org/10.1063/1.2359354)

- Code: [src/material/catalogue/references.rs:46](../src/material/catalogue/references.rs#L46)

### M

#### Mack 2007

C. Mack, "Fundamental Principles of Optical Lithography: The Science of Microfabrication", Wiley (2007). [doi:10.1002/9780470723876](https://doi.org/10.1002/9780470723876)

- Methods pages: [fabrication.md:34](methods/fabrication.md?plain=1#L34)
- Validation cases: [`fabrication/min-features-above-limit`](validation.md?plain=1#L751)
- Code: [src/fabrication/references.rs:288](../src/fabrication/references.rs#L288), [src/fabrication/rule_checks.rs:337](../src/fabrication/rule_checks.rs#L337)
- Other documents: [docs/plans/fabrication.md:620](plans/fabrication.md?plain=1#L620)

#### Macleod 2017

H. A. Macleod, "Thin-Film Optical Filters, Fifth Edition", CRC Press (2017). [doi:10.1201/b21960](https://doi.org/10.1201/b21960)

- Academy lessons: [bragg-gratings.md:85](../academy/bragg-gratings.md?plain=1#L85), [281](../academy/bragg-gratings.md?plain=1#L281), [405](../academy/bragg-gratings.md?plain=1#L405)

#### Madou 2018

M. J. Madou, "Fundamentals of Microfabrication and Nanotechnology, Three-Volume Set", CRC Press (2018). [doi:10.1201/9781315274164](https://doi.org/10.1201/9781315274164)

- Methods pages: [fabrication.md:30](methods/fabrication.md?plain=1#L30)
- Validation cases: [`fabrication/rule-choices`](validation.md?plain=1#L752)
- Code: [src/fabrication/references.rs:264](../src/fabrication/references.rs#L264), [src/fabrication/rule_checks.rs:344](../src/fabrication/rule_checks.rs#L344)
- Other documents: [docs/plans/fabrication.md:618](plans/fabrication.md?plain=1#L618)

#### Madsen et al. 2022

L. S. Madsen, F. Laudenbach, M. F. Askarani, F. Rortais, T. Vincent, J. F. F. Bulmer, F. M. Miatto, L. Neuhaus, L. G. Helt, M. J. Collins, et al., "Quantum computational advantage with a programmable photonic processor", Nature 606, 75 (2022). [doi:10.1038/s41586-022-04725-x](https://doi.org/10.1038/s41586-022-04725-x), open access.

- Other documents: [docs/plans/quantum.md:506](plans/quantum.md?plain=1#L506)

#### Majkić et al. 2017

A. Majkić, A. Franke, R. Kirste, R. Schlesser, R. Collazo, Z. Sitar, M. Zgonik, "Optical nonlinear and electro‐optical coefficients in bulk aluminium nitride single crystals", Physica Status Solidi (b) 254, 1700077 (2017). [doi:10.1002/pssb.201700077](https://doi.org/10.1002/pssb.201700077)

- Methods pages: [catalogue.md:88](methods/catalogue.md?plain=1#L88)
- Validation cases: [`material/aln-majkic-d33`](validation.md?plain=1#L89)
- Code: [src/material/catalogue/references.rs:375](../src/material/catalogue/references.rs#L375), [src/validation.rs:648](../src/validation.rs#L648)

#### Mak et al. 2016

J. C. C. Mak, C. Sideris, J. Jeong, A. Hajimiri, J. K. S. Poon, "Binary particle swarm optimized 2 × 2 power splitters in a standard foundry silicon photonic platform", Opt. Lett. 41, 3868 (2016). [doi:10.1364/OL.41.003868](https://doi.org/10.1364/OL.41.003868)

- Other documents: [docs/plans/optimization.md:610](plans/optimization.md?plain=1#L610)

#### Malas et al. 2015

T. Malas, G. Hager, H. Ltaief, H. Stengel, G. Wellein, D. Keyes, "Multicore-Optimized Wavefront Diamond Blocking for Optimizing Stencil Updates", SIAM J. Sci. Comput. 37, C439 (2015). [doi:10.1137/140991133](https://doi.org/10.1137/140991133)

- Methods pages: [fdtd.md:48](methods/fdtd.md?plain=1#L48)
- Code: [src/fdtd/kernel/blocked.rs:2](../src/fdtd/kernel/blocked.rs#L2)
- Other documents: [docs/plans/performance.md:630](plans/performance.md?plain=1#L630)

#### Malas et al. 2016

T. M. Malas, J. Hornich, G. Hager, H. Ltaief, C. Pflaum, D. E. Keyes, "Optimization of an Electromagnetics Code with Multicore Wavefront Diamond Blocking and Multi-dimensional Intra-Tile Parallelization", 2016 IEEE International Parallel and Distributed Processing Symposium (IPDPS) (2016). [doi:10.1109/IPDPS.2016.87](https://doi.org/10.1109/IPDPS.2016.87)

- Methods pages: [fdtd.md:50](methods/fdtd.md?plain=1#L50)
- Code: [src/fdtd/kernel/blocked.rs:3](../src/fdtd/kernel/blocked.rs#L3)
- Other documents: [docs/plans/performance.md:631](plans/performance.md?plain=1#L631)

#### Malitson 1965

I. H. Malitson, "Interspecimen Comparison of the Refractive Index of Fused Silica\*,†", J. Opt. Soc. Am. 55, 1205 (1965). [doi:10.1364/JOSA.55.001205](https://doi.org/10.1364/JOSA.55.001205)

- Methods pages: [materials.md:10](methods/materials.md?plain=1#L10)
- Academy lessons: [material-dispersion.md:32](../academy/material-dispersion.md?plain=1#L32), [90](../academy/material-dispersion.md?plain=1#L90), [234](../academy/material-dispersion.md?plain=1#L234), [438](../academy/material-dispersion.md?plain=1#L438), [458](../academy/material-dispersion.md?plain=1#L458)
- Validation cases: [`material/silica-malitson-formula`](validation.md?plain=1#L16), [`material/silica-malitson-measured`](validation.md?plain=1#L17)
- Examples: [README.md:35](../examples/README.md?plain=1#L35), [silica_index.rs:4](../examples/silica_index.rs#L4)
- Code: [src/material/catalogue/references.rs:109](../src/material/catalogue/references.rs#L109), [src/material/mod.rs:46](../src/material/mod.rs#L46), [796](../src/material/mod.rs#L796), [825](../src/material/mod.rs#L825), [src/material/refractiveindex.rs:332](../src/material/refractiveindex.rs#L332), [445](../src/material/refractiveindex.rs#L445), [src/validation.rs:137](../src/validation.rs#L137), [144](../src/validation.rs#L144), [studio/src-tauri/src/charts/material.rs:629](../studio/src-tauri/src/charts/material.rs#L629)
- Other documents: [ROADMAP.md:744](../ROADMAP.md?plain=1#L744)

#### Mandelshtam and Taylor 1997

V. A. Mandelshtam, H. S. Taylor, "Harmonic inversion of time signals and its applications", The Journal of Chemical Physics 107, 6756 (1997). [doi:10.1063/1.475324](https://doi.org/10.1063/1.475324)

- Methods pages: [bodies.md:14](methods/bodies.md?plain=1#L14), [fdtd.md:22](methods/fdtd.md?plain=1#L22), [881](methods/fdtd.md?plain=1#L881)
- Validation cases: [`fdtd/harmonic-inversion`](validation.md?plain=1#L250)
- Code: [src/fdtd.rs:51](../src/fdtd.rs#L51), [src/fdtd/harmonic.rs:3](../src/fdtd/harmonic.rs#L3), [src/validation.rs:1775](../src/validation.rs#L1775)
- Other documents: [ROADMAP.md:722](../ROADMAP.md?plain=1#L722)

#### Manolatou et al. 1999

C. Manolatou, M. Khan, S. Fan, P. Villeneuve, H. Haus, J. Joannopoulos, "Coupling of modes analysis of resonant channel add-drop filters", IEEE J. Quantum Electron. 35, 1322 (1999). [doi:10.1109/3.784592](https://doi.org/10.1109/3.784592)

- Academy lessons: [ring-resonator.md:43](../academy/ring-resonator.md?plain=1#L43), [340](../academy/ring-resonator.md?plain=1#L340), [596](../academy/ring-resonator.md?plain=1#L596)

#### Manteuffel and White 1986

T. A. Manteuffel, A. B. White, "The numerical solution of second-order boundary value problems on nonuniform meshes", Math. Comp. 47, 511 (1986). [doi:10.1090/S0025-5718-1986-0856700-3](https://doi.org/10.1090/S0025-5718-1986-0856700-3)

- Methods pages: [static.md:12](methods/static.md?plain=1#L12)
- Code: [src/statics.rs:28](../src/statics.rs#L28)

#### Marcatili 1969a

E. A. J. Marcatili, "Bends in Optical Dielectric Guides", Bell System Technical Journal 48, 2103 (1969). [doi:10.1002/j.1538-7305.1969.tb01167.x](https://doi.org/10.1002/j.1538-7305.1969.tb01167.x)

- Academy lessons: [leaky-and-bent-guides.md:47](../academy/leaky-and-bent-guides.md?plain=1#L47), [554](../academy/leaky-and-bent-guides.md?plain=1#L554), [ring-resonator.md:25](../academy/ring-resonator.md?plain=1#L25)

#### Marcatili 1969b

E. A. J. Marcatili, "Dielectric Rectangular Waveguide and Directional Coupler for Integrated Optics", Bell System Technical Journal 48, 2071 (1969). [doi:10.1002/j.1538-7305.1969.tb01166.x](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x)

- Methods pages: [devices.md:38](methods/devices.md?plain=1#L38), [marcatili.md:8](methods/marcatili.md?plain=1#L8)
- Academy lessons: [directional-coupler.md:28](../academy/directional-coupler.md?plain=1#L28), [274](../academy/directional-coupler.md?plain=1#L274), [515](../academy/directional-coupler.md?plain=1#L515), [strip-waveguide.md:46](../academy/strip-waveguide.md?plain=1#L46), [348](../academy/strip-waveguide.md?plain=1#L348), [667](../academy/strip-waveguide.md?plain=1#L667), [704](../academy/strip-waveguide.md?plain=1#L704)
- Validation cases: [`mode/marcatili-closed-form`](validation.md?plain=1#L128), [`mode/marcatili-vector`](validation.md?plain=1#L129), [`device/thermo-analytic-metal`](validation.md?plain=1#L644), [`device/thermo-analytic-in-guide`](validation.md?plain=1#L652)
- Examples: [README.md:42](../examples/README.md?plain=1#L42), [marcatili.rs:5](../examples/marcatili.rs#L5)
- Code: [src/cases/device/thermo_checks.rs:413](../src/cases/device/thermo_checks.rs#L413), [482](../src/cases/device/thermo_checks.rs#L482), [src/device/thermo.rs:716](../src/device/thermo.rs#L716), [src/mode/marcatili.rs:5](../src/mode/marcatili.rs#L5), [src/validation.rs:921](../src/validation.rs#L921), [928](../src/validation.rs#L928)
- Other documents: [ROADMAP.md:725](../ROADMAP.md?plain=1#L725)

#### Marcuse 1971

D. Marcuse, "Bending Losses of the Asymmetric Slab Waveguide", Bell System Technical Journal 50, 2551 (1971). [doi:10.1002/j.1538-7305.1971.tb02620.x](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x)

- Methods pages: [bends.md:10](methods/bends.md?plain=1#L10), [components.md:14](methods/components.md?plain=1#L14), [devices.md:22](methods/devices.md?plain=1#L22)
- Academy lessons: [leaky-and-bent-guides.md:53](../academy/leaky-and-bent-guides.md?plain=1#L53), [419](../academy/leaky-and-bent-guides.md?plain=1#L419), [434](../academy/leaky-and-bent-guides.md?plain=1#L434), [555](../academy/leaky-and-bent-guides.md?plain=1#L555), [586](../academy/leaky-and-bent-guides.md?plain=1#L586)
- Validation cases: [`mode/bend-slab-te`](validation.md?plain=1#L121), [`mode/bend-marcuse`](validation.md?plain=1#L124), [`components/ring-fsr-fdfd`](validation.md?plain=1#L293), [`device/bend-ring-slab`](validation.md?plain=1#L323)
- Examples: [README.md:38](../examples/README.md?plain=1#L38), [bend_loss.rs:5](../examples/bend_loss.rs#L5)
- Code: [src/circuit/components/waveguide.rs:539](../src/circuit/components/waveguide.rs#L539), [src/device/bend.rs:247](../src/device/bend.rs#L247), [425](../src/device/bend.rs#L425), [src/device/checks.rs:159](../src/device/checks.rs#L159), [src/mode/bend.rs:8](../src/mode/bend.rs#L8), [src/validation.rs:872](../src/validation.rs#L872), [893](../src/validation.rs#L893), [2076](../src/validation.rs#L2076), [studio/src-tauri/src/charts/leaky.rs:13](../studio/src-tauri/src/charts/leaky.rs#L13)

#### Marcuse 1991

D. Marcuse, "THE ASYMMETRIC SLAB WAVEGUIDE", Theory of Dielectric Optical Waveguides, Elsevier (1991). [doi:10.1016/B978-0-12-470951-5.50007-X](https://doi.org/10.1016/B978-0-12-470951-5.50007-X)

- Academy lessons: [slab-waveguide.md:66](../academy/slab-waveguide.md?plain=1#L66), [572](../academy/slab-waveguide.md?plain=1#L572)

#### Maring et al. 2024

N. Maring, A. Fyrillas, M. Pont, E. Ivanov, P. Stepanov, N. Margaria, W. Hease, A. Pishchagin, A. Lemaître, I. Sagnes, et al., "A versatile single-photon-based quantum computing platform", Nat. Photon. 18, 603 (2024). [doi:10.1038/s41566-024-01403-4](https://doi.org/10.1038/s41566-024-01403-4), open access.

- Other documents: [docs/plans/quantum.md:525](plans/quantum.md?plain=1#L525)

#### Masetti et al. 1983

G. Masetti, M. Severi, S. Solmi, "Modeling of carrier mobility against carrier concentration in arsenic-, phosphorus-, and boron-doped silicon", IEEE Trans. Electron Devices 30, 764 (1983). [doi:10.1109/T-ED.1983.21207](https://doi.org/10.1109/T-ED.1983.21207)

- Other documents: [ROADMAP.md:809](../ROADMAP.md?plain=1#L809)

#### McKay et al. 1979

M. D. McKay, R. J. Beckman, W. J. Conover, "Comparison of Three Methods for Selecting Values of Input Variables in the Analysis of Output from a Computer Code", Technometrics 21, 239 (1979). [doi:10.1080/00401706.1979.10489755](https://doi.org/10.1080/00401706.1979.10489755)

- Methods pages: [compact.md:332](methods/compact.md?plain=1#L332), [devices.md:1520](methods/devices.md?plain=1#L1520)
- Code: [src/device/compact.rs:54](../src/device/compact.rs#L54)

#### McKinnon et al. 2009

W. R. McKinnon, D. X. Xu, C. Storey, E. Post, A. Densmore, A. Delâge, P. Waldron, J. H. Schmid, S. Janz, "Extracting coupling and loss coefficients from a ring resonator", Opt. Express 17, 18971 (2009). [doi:10.1364/OE.17.018971](https://doi.org/10.1364/OE.17.018971)

- Academy lessons: [ring-resonator.md:79](../academy/ring-resonator.md?plain=1#L79), [531](../academy/ring-resonator.md?plain=1#L531)

#### Meixner 1972

J. Meixner, "The behavior of electromagnetic fields at edges", IEEE Trans. Antennas Propagat. 20, 442 (1972). [doi:10.1109/TAP.1972.1140243](https://doi.org/10.1109/TAP.1972.1140243)

- Methods pages: [devices.md:66](methods/devices.md?plain=1#L66)
- Validation cases: [`roughness/silicon-grid-order`](validation.md?plain=1#L706)
- Code: [src/mode/roughness.rs:430](../src/mode/roughness.rs#L430), [src/mode/roughness_checks.rs:170](../src/mode/roughness_checks.rs#L170)

#### Menicucci et al. 2006

N. C. Menicucci, P. van Loock, M. Gu, C. Weedbrook, T. C. Ralph, M. A. Nielsen, "Universal Quantum Computation with Continuous-Variable Cluster States", Phys. Rev. Lett. 97, 110501 (2006). [doi:10.1103/PhysRevLett.97.110501](https://doi.org/10.1103/PhysRevLett.97.110501)

- Other documents: [docs/plans/quantum.md:513](plans/quantum.md?plain=1#L513)

#### Micikevicius 2009

P. Micikevicius, "3D finite difference computation on GPUs using CUDA", Proceedings of 2nd Workshop on General Purpose Processing on Graphics Processing Units (2009). [doi:10.1145/1513895.1513905](https://doi.org/10.1145/1513895.1513905)

- Methods pages: [fdtd.md:38](methods/fdtd.md?plain=1#L38)
- Code: [src/fdtd/gpu.rs:9](../src/fdtd/gpu.rs#L9), [src/fdtd/gpu/yee.wgsl:11](../src/fdtd/gpu/yee.wgsl#L11)
- Other documents: [docs/plans/performance.md:632](plans/performance.md?plain=1#L632)

#### Mie 1908

G. Mie, "Beiträge zur Optik trüber Medien, speziell kolloidaler Metallösungen", Annalen der Physik 330, 377 (1908). [doi:10.1002/andp.19083300302](https://doi.org/10.1002/andp.19083300302)

- Methods pages: [fdtd.md:8](methods/fdtd.md?plain=1#L8), [933](methods/fdtd.md?plain=1#L933)
- Validation cases: [`fdtd/mie-table`](validation.md?plain=1#L253), [`fdtd/mie-sphere`](validation.md?plain=1#L256)
- Code: [src/fdtd.rs:58](../src/fdtd.rs#L58), [src/fdtd/mie.rs:2](../src/fdtd/mie.rs#L2), [src/validation.rs:1796](../src/validation.rs#L1796), [1817](../src/validation.rs#L1817)
- Other documents: [ROADMAP.md:740](../ROADMAP.md?plain=1#L740)

#### Miller 1964

R. C. Miller, "OPTICAL SECOND HARMONIC GENERATION IN PIEZOELECTRIC CRYSTALS", Applied Physics Letters 5, 17 (1964). [doi:10.1063/1.1754022](https://doi.org/10.1063/1.1754022)

- Methods pages: [nonlinear.md:12](methods/nonlinear.md?plain=1#L12)
- Validation cases: [`nonlinear/miller-linbo3-d33`](validation.md?plain=1#L735), [`nonlinear/miller-linbo3-d31`](validation.md?plain=1#L736), [`nonlinear/miller-linbo3-mgo-d33`](validation.md?plain=1#L737), [`nonlinear/miller-linbo3-mgo-d31`](validation.md?plain=1#L738)
- Code: [src/nonlinear/data_checks.rs:333](../src/nonlinear/data_checks.rs#L333), [340](../src/nonlinear/data_checks.rs#L340), [347](../src/nonlinear/data_checks.rs#L347), [354](../src/nonlinear/data_checks.rs#L354), [src/nonlinear/miller.rs:4](../src/nonlinear/miller.rs#L4)
- Other documents: [ROADMAP.md:843](../ROADMAP.md?plain=1#L843)

#### Miller 1969

S. E. Miller, "Integrated Optics: An Introduction", Bell System Technical Journal 48, 2059 (1969). [doi:10.1002/j.1538-7305.1969.tb01165.x](https://doi.org/10.1002/j.1538-7305.1969.tb01165.x)

- Academy lessons: [slab-waveguide.md:30](../academy/slab-waveguide.md?plain=1#L30), [562](../academy/slab-waveguide.md?plain=1#L562), [strip-waveguide.md:40](../academy/strip-waveguide.md?plain=1#L40), [666](../academy/strip-waveguide.md?plain=1#L666)

#### Miller 2013

D. A. B. Miller, "Self-configuring universal linear optical component \[Invited\]", Photon. Res. 1, 1 (2013). [doi:10.1364/PRJ.1.000001](https://doi.org/10.1364/PRJ.1.000001)

- Other documents: [ROADMAP.md:822](../ROADMAP.md?plain=1#L822), [docs/plans/optimization.md:626](plans/optimization.md?plain=1#L626)

#### Miller et al. 1971

R. C. Miller, W. A. Nordland, P. M. Bridenbaugh, "Dependence of Second-Harmonic-Generation Coefficients of LiNbO3 on Melt Composition", Journal of Applied Physics 42, 4145 (1971). [doi:10.1063/1.1659746](https://doi.org/10.1063/1.1659746)

- Code: [src/material/catalogue/references.rs:179](../src/material/catalogue/references.rs#L179)

#### Mills et al. 2021

R. T. Mills, M. F. Adams, S. Balay, J. Brown, A. Dener, M. Knepley, S. E. Kruger, H. Morgan, T. Munson, K. Rupp, et al., "Toward performance-portable PETSc for GPU-based exascale systems", Parallel Computing 108, 102831 (2021). [doi:10.1016/j.parco.2021.102831](https://doi.org/10.1016/j.parco.2021.102831)

- Other documents: [docs/plans/performance.md:633](plans/performance.md?plain=1#L633)

#### Minkov and Savona 2014

M. Minkov, V. Savona, "Automated optimization of photonic crystal slab cavities", Sci Rep 4, 5124 (2014). [doi:10.1038/srep05124](https://doi.org/10.1038/srep05124), open access.

- Other documents: [docs/plans/optimization.md:608](plans/optimization.md?plain=1#L608)

#### Minkov et al. 2020

M. Minkov, I. A. D. Williamson, L. C. Andreani, D. Gerace, B. Lou, A. Y. Song, T. W. Hughes, S. Fan, "Inverse Design of Photonic Crystals through Automatic Differentiation", ACS Photonics 7, 1729 (2020). [doi:10.1021/acsphotonics.0c00327](https://doi.org/10.1021/acsphotonics.0c00327)

- Other documents: [docs/plans/optimization.md:596](plans/optimization.md?plain=1#L596)

#### Moharam et al. 1995

M. G. Moharam, T. K. Gaylord, E. B. Grann, D. A. Pommet, "Formulation for stable and efficient implementation of the rigorous coupled-wave analysis of binary gratings", J. Opt. Soc. Am. A 12, 1068 (1995). [doi:10.1364/JOSAA.12.001068](https://doi.org/10.1364/JOSAA.12.001068)

- Other documents: [ROADMAP.md:731](../ROADMAP.md?plain=1#L731)

#### Moille et al. 2019

G. Moille, Q. Li, L. Xiyuan, K. Srinivasan, "pyLLE: A Fast and User Friendly Lugiato-Lefever Equation Solver", J. RES. NATL. INST. STAN. 124, 124012 (2019). [doi:10.6028/jres.124.012](https://doi.org/10.6028/jres.124.012)

- Other documents: [ROADMAP.md:837](../ROADMAP.md?plain=1#L837)

#### Molesky et al. 2018

S. Molesky, Z. Lin, A. Y. Piggott, W. Jin, J. Vucković, A. W. Rodriguez, "Inverse design in nanophotonics", Nature Photon 12, 659 (2018). [doi:10.1038/s41566-018-0246-9](https://doi.org/10.1038/s41566-018-0246-9)

- Other documents: [ROADMAP.md:767](../ROADMAP.md?plain=1#L767), [docs/plans/optimization.md:595](plans/optimization.md?plain=1#L595)

#### Moutzouris et al. 2011

K. Moutzouris, G. Hloupis, I. Stavrakas, D. Triantis, M.-H. Chou, "Temperature-dependent visible to near-infrared optical properties of 8 mol% Mg-doped lithium tantalate", Opt. Mater. Express 1, 458 (2011). [doi:10.1364/OME.1.000458](https://doi.org/10.1364/OME.1.000458)

- Methods pages: [catalogue.md:22](methods/catalogue.md?plain=1#L22)
- Validation cases: [`material/litao3-moutzouris-fit`](validation.md?plain=1#L75)
- Code: [src/material/catalogue/litao3_dndt.rs:5](../src/material/catalogue/litao3_dndt.rs#L5), [src/material/catalogue/references.rs:557](../src/material/catalogue/references.rs#L557), [558](../src/material/catalogue/references.rs#L558), [src/validation.rs:550](../src/validation.rs#L550)

### N

#### Nagaoka and Watanabe 2011

T. Nagaoka, S. Watanabe, "Multi-GPU accelerated three-dimensional FDTD method for electromagnetic simulation", 2011 Annual International Conference of the IEEE Engineering in Medicine and Biology Society (2011). [doi:10.1109/IEMBS.2011.6090128](https://doi.org/10.1109/IEMBS.2011.6090128)

- Other documents: [docs/plans/performance.md:634](plans/performance.md?plain=1#L634)

#### Nakano et al. 2017

S. Nakano, K. Akiyama, I. Shoji, "Accurate measurement of electro-optic coefficients of undoped and MgO-doped stoichiometric LiNbO\_3", Opt. Mater. Express 7, 939 (2017). [doi:10.1364/OME.7.000939](https://doi.org/10.1364/OME.7.000939)

- Code: [src/material/catalogue/references.rs:165](../src/material/catalogue/references.rs#L165), [166](../src/material/catalogue/references.rs#L166)

#### Nakatsukasa et al. 2018

Y. Nakatsukasa, O. Sète, L. N. Trefethen, "The AAA Algorithm for Rational Approximation", SIAM J. Sci. Comput. 40, A1494 (2018). [doi:10.1137/16M1106122](https://doi.org/10.1137/16M1106122)

- Methods pages: [sweep.md:8](methods/sweep.md?plain=1#L8)
- Validation cases: [`sweep/aaa-rational`](validation.md?plain=1#L707)
- Code: [src/sweep.rs:24](../src/sweep.rs#L24), [src/sweep/aaa.rs:6](../src/sweep/aaa.rs#L6), [src/sweep/checks.rs:422](../src/sweep/checks.rs#L422)
- Other documents: [ROADMAP.md:735](../ROADMAP.md?plain=1#L735)

#### Nedeljkovic et al. 2011

M. Nedeljkovic, R. Soref, G. Z. Mashanovich, "Free-Carrier Electrorefraction and Electroabsorption Modulation Predictions for Silicon Over the 1&#x2013;14- \<formula formulatype="inline"\>\<tex Notation="TeX"\>\$\\mu\\hbox{m}\$\</tex\>\</formula\> Infrared Wavelength Range", IEEE Photonics J. 3, 1171 (2011). [doi:10.1109/JPHOT.2011.2171930](https://doi.org/10.1109/JPHOT.2011.2171930)

- Other documents: [ROADMAP.md:811](../ROADMAP.md?plain=1#L811)

#### Nehra et al. 2022

R. Nehra, R. Sekine, L. Ledezma, Q. Guo, R. M. Gray, A. Roy, A. Marandi, "Few-cycle vacuum squeezing in nanophotonics", Science 377, 1333 (2022). [doi:10.1126/science.abo6213](https://doi.org/10.1126/science.abo6213)

- Other documents: [docs/plans/quantum.md:533](plans/quantum.md?plain=1#L533)

#### Nelder and Mead 1965

J. A. Nelder, R. Mead, "A Simplex Method for Function Minimization", The Computer Journal 7, 308 (1965). [doi:10.1093/comjnl/7.4.308](https://doi.org/10.1093/comjnl/7.4.308)

- Methods pages: [optimization.md:135](methods/optimization.md?plain=1#L135)
- Code: [src/job/optimize.rs:201](../src/job/optimize.rs#L201)

#### Njoroge et al. 2002

W. K. Njoroge, H.-W. Wöltgens, M. Wuttig, "Density changes upon crystallization of Ge2Sb2.04Te4.74 films", Journal of Vacuum Science & Technology A: Vacuum, Surfaces, and Films 20, 230 (2002). [doi:10.1116/1.1430249](https://doi.org/10.1116/1.1430249)

- Methods pages: [catalogue.md:425](methods/catalogue.md?plain=1#L425)
- Validation cases: [`material/pcm-gst-njoroge-change`](validation.md?plain=1#L56), [`material/pcm-gst-njoroge-xrd`](validation.md?plain=1#L57)
- Code: [src/material/catalogue/phase_change_thermal.rs:4](../src/material/catalogue/phase_change_thermal.rs#L4), [src/material/catalogue/references.rs:522](../src/material/catalogue/references.rs#L522), [src/validation.rs:417](../src/validation.rs#L417), [424](../src/validation.rs#L424)

#### Novotny and Hecht 2012

L. Novotny, B. Hecht, "Principles of Nano-Optics", Cambridge University Press (2012). [doi:10.1017/CBO9780511794193](https://doi.org/10.1017/CBO9780511794193)

- Methods pages: [devices.md:68](methods/devices.md?plain=1#L68)
- Code: [src/mode/roughness.rs:50](../src/mode/roughness.rs#L50)

### O

#### Oh et al. 2024

C. Oh, M. Liu, Y. Alexeev, B. Fefferman, L. Jiang, "Classical algorithm for simulating experimental Gaussian boson sampling", Nat. Phys. 20, 1461 (2024). [doi:10.1038/s41567-024-02535-8](https://doi.org/10.1038/s41567-024-02535-8)

- Other documents: [docs/plans/quantum.md:504](plans/quantum.md?plain=1#L504)

#### Ohashi et al. 1993

M. Ohashi, T. Kondo, R. Ito, S. Fukatsu, Y. Shiraki, K. Kumata, S. S. Kano, "Determination of quadratic nonlinear optical coefficient of AlxGa1−xAs system by the method of reflected second harmonics", Journal of Applied Physics 74, 596 (1993). [doi:10.1063/1.355272](https://doi.org/10.1063/1.355272)

- Code: [src/material/catalogue/references.rs:305](../src/material/catalogue/references.rs#L305)

#### Okoniewaki et al. 1997

M. Okoniewaki, M. Mrozowski, M. Stuchly, "Simple treatment of multi-term dispersion in FDTD", IEEE Microw. Guid. Wave Lett. 7, 121 (1997). [doi:10.1109/75.569723](https://doi.org/10.1109/75.569723)

- Methods pages: [fdtd.md:24](methods/fdtd.md?plain=1#L24)
- Validation cases: [`fdtd/ade-fdfd`](validation.md?plain=1#L218)
- Examples: [README.md:65](../examples/README.md?plain=1#L65), [lorentz_okoniewski.rs:6](../examples/lorentz_okoniewski.rs#L6)
- Code: [src/fdtd.rs:37](../src/fdtd.rs#L37), [src/fdtd/media.rs:8](../src/fdtd/media.rs#L8), [src/validation.rs:1551](../src/validation.rs#L1551)

#### Olver et al. 2010

F. W. J. Olver, et al., "NIST Digital Library of Mathematical Functions", National Institute of Standards and Technology (2010). [Open copy](https://dlmf.nist.gov/).

- Methods pages: [bends.md:13](methods/bends.md?plain=1#L13), [expressions.md:11](methods/expressions.md?plain=1#L11)

#### Oser et al. 2020

D. Oser, D. Pérez-Galacho, X. Le Roux, S. Tanzilli, L. Vivien, L. Labonté, É. Cassan, C. Alonso-Ramos, "Silicon subwavelength modal Bragg grating filters with narrow bandwidth and high optical rejection", Opt. Lett. 45, 5784 (2020). [doi:10.1364/OL.394455](https://doi.org/10.1364/OL.394455)

- Academy lessons: [bragg-gratings.md:103](../academy/bragg-gratings.md?plain=1#L103), [312](../academy/bragg-gratings.md?plain=1#L312)

#### Oskooi et al. 2009

A. F. Oskooi, C. Kottke, S. G. Johnson, "Accurate finite-difference time-domain simulation of anisotropic media by subpixel smoothing", Opt. Lett. 34, 2778 (2009). [doi:10.1364/OL.34.002778](https://doi.org/10.1364/OL.34.002778)

- Methods pages: [fdtd.md:36](methods/fdtd.md?plain=1#L36)
- Validation cases: [`fdtd/smoothing-oblique-nodes`](validation.md?plain=1#L230), [`fdtd/smoothing-oskooi`](validation.md?plain=1#L233)
- Code: [src/fdtd/smoothing.rs:15](../src/fdtd/smoothing.rs#L15), [src/validation.rs:1635](../src/validation.rs#L1635), [1656](../src/validation.rs#L1656)
- Other documents: [ROADMAP.md:720](../ROADMAP.md?plain=1#L720)

#### Oskooi et al. 2010

A. F. Oskooi, D. Roundy, M. Ibanescu, P. Bermel, J. Joannopoulos, S. G. Johnson, "Meep: A flexible free-software package for electromagnetic simulations by the FDTD method", Computer Physics Communications 181, 687 (2010). [doi:10.1016/j.cpc.2009.11.008](https://doi.org/10.1016/j.cpc.2009.11.008)

- Methods pages: [conventions.md:8](methods/conventions.md?plain=1#L8), [fdfd-ports.md:10](methods/fdfd-ports.md?plain=1#L10), [fdtd.md:40](methods/fdtd.md?plain=1#L40), [1045](methods/fdtd.md?plain=1#L1045)
- Validation cases: [`fdtd/spectrum`](validation.md?plain=1#L209), [`fdtd/meep-pml-rates`](validation.md?plain=1#L259)
- Examples: [README.md:67](../examples/README.md?plain=1#L67), [bump_oskooi.rs:6](../examples/bump_oskooi.rs#L6), [pml_oskooi.rs:6](../examples/pml_oskooi.rs#L6)
- Code: [src/fdtd.rs:43](../src/fdtd.rs#L43), [src/fdtd/meep_checks.rs:2](../src/fdtd/meep_checks.rs#L2), [src/fdtd/sources.rs:9](../src/fdtd/sources.rs#L9), [src/units.rs:8](../src/units.rs#L8), [src/validation.rs:1488](../src/validation.rs#L1488), [1838](../src/validation.rs#L1838)
- Other documents: [ROADMAP.md:721](../ROADMAP.md?plain=1#L721), [docs/plans/performance.md:585](plans/performance.md?plain=1#L585)

### P

#### Paesani et al. 2020

S. Paesani, M. Borghi, S. Signorini, A. Maïnos, L. Pavesi, A. Laing, "Near-ideal spontaneous photon sources in silicon quantum photonics", Nat Commun 11, 2505 (2020). [doi:10.1038/s41467-020-16187-8](https://doi.org/10.1038/s41467-020-16187-8), open access.

- Other documents: [docs/plans/quantum.md:529](plans/quantum.md?plain=1#L529)

#### Pai et al. 2019

S. Pai, B. Bartlett, O. Solgaard, D. A. B. Miller, "Matrix Optimization on Universal Unitary Photonic Devices", Phys. Rev. Applied 11, 064044 (2019). [doi:10.1103/PhysRevApplied.11.064044](https://doi.org/10.1103/PhysRevApplied.11.064044)

- Examples: [README.md:56](../examples/README.md?plain=1#L56), [optimize_mesh.rs:5](../examples/optimize_mesh.rs#L5)
- Other documents: [docs/plans/optimization.md:629](plans/optimization.md?plain=1#L629)

#### Papatryfonos et al. 2021

K. Papatryfonos, T. Angelova, A. Brimont, B. Reid, S. Guldin, P. R. Smith, M. Tang, K. Li, A. J. Seeds, H. Liu, et al., "Refractive indices of MBE-grown AlxGa(1−x)As ternary alloys in the transparent wavelength region", AIP Advances 11, 025327 (2021). [doi:10.1063/5.0039631](https://doi.org/10.1063/5.0039631), open access.

- Methods pages: [catalogue.md:66](methods/catalogue.md?plain=1#L66)
- Validation cases: [`material/algaas-papatryfonos`](validation.md?plain=1#L81)
- Code: [src/material/catalogue/references.rs:256](../src/material/catalogue/references.rs#L256), [257](../src/material/catalogue/references.rs#L257), [src/validation.rs:592](../src/validation.rs#L592)

#### Parks et al. 2006

M. L. Parks, E. de Sturler, G. Mackey, D. D. Johnson, S. Maiti, "Recycling Krylov Subspaces for Sequences of Linear Systems", SIAM J. Sci. Comput. 28, 1651 (2006). [doi:10.1137/040607277](https://doi.org/10.1137/040607277)

- Methods pages: [recycling.md:8](methods/recycling.md?plain=1#L8), [77](methods/recycling.md?plain=1#L77)
- Validation cases: [`fdfd3d/recycle-s-matrix`](validation.md?plain=1#L188), [`fdfd3d/recycle-deflation`](validation.md?plain=1#L189), [`fdfd3d/recycle-adjoint`](validation.md?plain=1#L191), [`fdfd3d/recycle-parks-table`](validation.md?plain=1#L201), [`fdfd3d/recycle-parks-rerun`](validation.md?plain=1#L202)
- Code: [src/backend/krylov/recycle.rs:6](../src/backend/krylov/recycle.rs#L6), [src/backend/krylov/recycle_example.rs:13](../src/backend/krylov/recycle_example.rs#L13), [src/fdfd/three/recycling.rs:29](../src/fdfd/three/recycling.rs#L29), [src/validation.rs:1341](../src/validation.rs#L1341), [1348](../src/validation.rs#L1348), [1362](../src/validation.rs#L1362), [1432](../src/validation.rs#L1432), [1439](../src/validation.rs#L1439)
- Other documents: [docs/plans/performance.md:635](plans/performance.md?plain=1#L635)

#### Parlett 1998

B. N. Parlett, "The Symmetric Eigenvalue Problem", Society for Industrial and Applied Mathematics (1998). [doi:10.1137/1.9781611971163](https://doi.org/10.1137/1.9781611971163)

- Methods pages: [eigen.md:108](methods/eigen.md?plain=1#L108)
- Validation cases: [`mode/krylov-schur-convergence`](validation.md?plain=1#L474)
- Code: [src/cases/eigen/checks.rs:353](../src/cases/eigen/checks.rs#L353)

#### Passoni et al. 2017

M. Passoni, D. Gerace, L. Carroll, L. C. Andreani, "Grating couplers in silicon-on-insulator: The role of photonic guided resonances on lineshape and bandwidth", Applied Physics Letters 110, 041107 (2017). [doi:10.1063/1.4974992](https://doi.org/10.1063/1.4974992)

- Examples: [README.md:59](../examples/README.md?plain=1#L59), [optimize_grating_pareto.rs:7](../examples/optimize_grating_pareto.rs#L7)
- Other documents: [docs/plans/optimization.md:611](plans/optimization.md?plain=1#L611)

#### Pastrňák and Roskovcová 1966

J. Pastrňák, L. Roskovcová, "Refraction Index Measurements on AlN Single Crystals", Physica Status Solidi (b) 14 (1966). [doi:10.1002/pssb.19660140127](https://doi.org/10.1002/pssb.19660140127)

- Code: [src/material/catalogue/references.rs:368](../src/material/catalogue/references.rs#L368)

#### Patankar 2018

S. V. Patankar, "Numerical Heat Transfer and Fluid Flow", CRC Press (2018). [doi:10.1201/9781482234213](https://doi.org/10.1201/9781482234213)

- Methods pages: [static.md:8](methods/static.md?plain=1#L8)
- Validation cases: [`static/two-layer-temperature`](validation.md?plain=1#L507)
- Code: [src/cases/statics/checks.rs:710](../src/cases/statics/checks.rs#L710), [src/statics.rs:20](../src/statics.rs#L20)

#### Patel et al. 2015

D. Patel, S. Ghosh, M. Chagnon, A. Samani, V. Veerasubramanian, M. Osman, D. V. Plant, "Design, analysis, and transmission system performance of a 41 GHz silicon photonic modulator", Opt. Express 23, 14263 (2015). [doi:10.1364/OE.23.014263](https://doi.org/10.1364/OE.23.014263)

- Other documents: [ROADMAP.md:813](../ROADMAP.md?plain=1#L813)

#### Pauline Bennet 2023

Pauline Bennet, "Ellawin/tuto\_global\_optimization\_photonics: v2", Zenodo (2023). [doi:10.5281/zenodo.10246032](https://doi.org/10.5281/zenodo.10246032)

- Methods pages: [optimization.md:287](methods/optimization.md?plain=1#L287)
- Examples: [optimize_bragg.rs:9](../examples/optimize_bragg.rs#L9)
- Code: [tests/method_study.rs:7](../tests/method_study.rs#L7)
- Other documents: [docs/plans/optimization.md:330](plans/optimization.md?plain=1#L330)

#### Payne and Lacey 1994

F. P. Payne, J. P. R. Lacey, "A theoretical analysis of scattering loss from planar optical waveguides", Opt Quant Electron 26, 977 (1994). [doi:10.1007/BF00708339](https://doi.org/10.1007/BF00708339)

- Methods pages: [devices.md:62](methods/devices.md?plain=1#L62)
- Validation cases: [`roughness/payne-lacey-eq9`](validation.md?plain=1#L704), [`roughness/payne-lacey-slab`](validation.md?plain=1#L705)
- Code: [src/mode/roughness.rs:11](../src/mode/roughness.rs#L11), [src/mode/roughness_checks.rs:148](../src/mode/roughness_checks.rs#L148), [163](../src/mode/roughness_checks.rs#L163)
- Other documents: [ROADMAP.md:737](../ROADMAP.md?plain=1#L737)

#### Pérez-López et al. 2020

D. Pérez-López, A. López, P. DasMahapatra, J. Capmany, "Multipurpose self-configuration of programmable photonic circuits", Nat Commun 11, 6359 (2020). [doi:10.1038/s41467-020-19608-w](https://doi.org/10.1038/s41467-020-19608-w), open access.

- Academy lessons: [mach-zehnder.md:124](../academy/mach-zehnder.md?plain=1#L124), [380](../academy/mach-zehnder.md?plain=1#L380)
- Examples: [README.md:57](../examples/README.md?plain=1#L57), [optimize_mesh_selfconfig.rs:6](../examples/optimize_mesh_selfconfig.rs#L6)
- Other documents: [docs/plans/optimization.md:630](plans/optimization.md?plain=1#L630)

#### Pernice et al. 2012

W. Pernice, C. Schuck, O. Minaeva, M. Li, G. Goltsman, A. Sergienko, H. Tang, "High-speed and high-efficiency travelling wave single-photon detectors embedded in nanophotonic circuits", Nat Commun 3, 1325 (2012). [doi:10.1038/ncomms2307](https://doi.org/10.1038/ncomms2307)

- Other documents: [docs/plans/quantum.md:537](plans/quantum.md?plain=1#L537)

#### Peruzzo et al. 2010

A. Peruzzo, M. Lobino, J. C. F. Matthews, N. Matsuda, A. Politi, K. Poulios, X.-Q. Zhou, Y. Lahini, N. Ismail, K. Wörhoff, et al., "Quantum Walks of Correlated Photons", Science 329, 1500 (2010). [doi:10.1126/science.1193515](https://doi.org/10.1126/science.1193515)

- Other documents: [docs/plans/quantum.md:508](plans/quantum.md?plain=1#L508)

#### Pettit and Turner 1965

G. D. Pettit, W. J. Turner, "Refractive Index of InP", Journal of Applied Physics 36, 2081 (1965). [doi:10.1063/1.1714410](https://doi.org/10.1063/1.1714410)

- Methods pages: [catalogue.md:84](methods/catalogue.md?plain=1#L84)
- Code: [src/material/catalogue/models.rs:938](../src/material/catalogue/models.rs#L938), [src/material/catalogue/references.rs:347](../src/material/catalogue/references.rs#L347)

#### Pfeiffer et al. 2016

M. H. P. Pfeiffer, A. Kordts, V. Brasch, M. Zervas, M. Geiselmann, J. D. Jost, T. J. Kippenberg, "Photonic Damascene process for integrated high-Q microresonator based nonlinear photonics", Optica 3, 20 (2016). [doi:10.1364/OPTICA.3.000020](https://doi.org/10.1364/OPTICA.3.000020)

- Methods pages: [fabrication.md:40](methods/fabrication.md?plain=1#L40), [nonlinear.md:44](methods/nonlinear.md?plain=1#L44)
- Validation cases: [`fabrication/nitride-rules`](validation.md?plain=1#L756), [`fabrication/nitride-reported-flows`](validation.md?plain=1#L757)
- Code: [src/fabrication/platform_checks.rs:721](../src/fabrication/platform_checks.rs#L721), [728](../src/fabrication/platform_checks.rs#L728), [src/fabrication/references.rs:256](../src/fabrication/references.rs#L256), [258](../src/fabrication/references.rs#L258)
- Other documents: [docs/plans/fabrication.md:658](plans/fabrication.md?plain=1#L658)

#### Pierce 1954

J. R. Pierce, "Coupling of Modes of Propagation", Journal of Applied Physics 25, 179 (1954). [doi:10.1063/1.1721599](https://doi.org/10.1063/1.1721599)

- Academy lessons: [directional-coupler.md:22](../academy/directional-coupler.md?plain=1#L22), [483](../academy/directional-coupler.md?plain=1#L483)

#### Piggott et al. 2015

A. Y. Piggott, J. Lu, K. G. Lagoudakis, J. Petykiewicz, T. M. Babinec, J. Vučković, "Inverse design and demonstration of a compact and broadband on-chip wavelength demultiplexer", Nature Photon 9, 374 (2015). [doi:10.1038/nphoton.2015.69](https://doi.org/10.1038/nphoton.2015.69)

- Other documents: [ROADMAP.md:763](../ROADMAP.md?plain=1#L763), [docs/plans/optimization.md:593](plans/optimization.md?plain=1#L593)

#### Piggott et al. 2017

A. Y. Piggott, J. Petykiewicz, L. Su, J. Vučković, "Fabrication-constrained nanophotonic inverse design", Sci Rep 7, 1786 (2017). [doi:10.1038/s41598-017-01939-2](https://doi.org/10.1038/s41598-017-01939-2), open access.

- Academy lessons: [directional-coupler.md:106](../academy/directional-coupler.md?plain=1#L106), [505](../academy/directional-coupler.md?plain=1#L505)
- Other documents: [docs/plans/optimization.md:594](plans/optimization.md?plain=1#L594)

#### Piggott et al. 2020

A. Y. Piggott, E. Y. Ma, L. Su, G. H. Ahn, N. V. Sapra, D. Vercruysse, A. M. Netherton, A. S. P. Khope, J. E. Bowers, J. Vučković, "Inverse-Designed Photonics for Semiconductor Foundries", ACS Photonics 7, 569 (2020). [doi:10.1021/acsphotonics.9b01540](https://doi.org/10.1021/acsphotonics.9b01540)

- Other documents: [ROADMAP.md:772](../ROADMAP.md?plain=1#L772)

#### Ploeg et al. 2021

S. Ploeg, H. Gunther, R. M. Camacho, "Simphony: An Open-Source Photonic Integrated Circuit Simulation Framework", Comput. Sci. Eng. 23, 65 (2021). [doi:10.1109/MCSE.2020.3012099](https://doi.org/10.1109/MCSE.2020.3012099)

- Methods pages: [components.md:291](methods/components.md?plain=1#L291)
- Other documents: [ROADMAP.md:754](../ROADMAP.md?plain=1#L754)

#### Poberaj et al. 2012

G. Poberaj, H. Hu, W. Sohler, P. Günter, "Lithium niobate on insulator (LNOI) for micro‐photonic devices", Laser & Photonics Reviews 6, 488 (2012). [doi:10.1002/lpor.201100035](https://doi.org/10.1002/lpor.201100035)

- Methods pages: [fabrication.md:18](methods/fabrication.md?plain=1#L18)
- Code: [src/fabrication/references.rs:112](../src/fabrication/references.rs#L112)
- Other documents: [docs/plans/fabrication.md:645](plans/fabrication.md?plain=1#L645)

#### Polizzi 2009

E. Polizzi, "Density-matrix-based algorithm for solving eigenvalue problems", Phys. Rev. B 79, 115112 (2009). [doi:10.1103/PhysRevB.79.115112](https://doi.org/10.1103/PhysRevB.79.115112)

- Methods pages: [contour.md:10](methods/contour.md?plain=1#L10)
- Academy lessons: [leaky-and-bent-guides.md:107](../academy/leaky-and-bent-guides.md?plain=1#L107), [386](../academy/leaky-and-bent-guides.md?plain=1#L386), [568](../academy/leaky-and-bent-guides.md?plain=1#L568)
- Validation cases: [`mode/contour-uniform-box`](validation.md?plain=1#L461)
- Code: [src/eigen/contour.rs:17](../src/eigen/contour.rs#L17), [src/mode/region.rs:10](../src/mode/region.rs#L10), [src/mode/region/checks.rs:341](../src/mode/region/checks.rs#L341)
- Other documents: [docs/plans/performance.md:636](plans/performance.md?plain=1#L636)

#### Polyanskiy 2024

M. N. Polyanskiy, "Refractiveindex.info database of optical constants", Sci Data 11, 94 (2024). [doi:10.1038/s41597-023-02898-2](https://doi.org/10.1038/s41597-023-02898-2), open access.

- Methods pages: [materials.md:14](methods/materials.md?plain=1#L14)
- Academy lessons: [material-dispersion.md:74](../academy/material-dispersion.md?plain=1#L74), [392](../academy/material-dispersion.md?plain=1#L392), [452](../academy/material-dispersion.md?plain=1#L452)
- Code: [src/material/mod.rs:14](../src/material/mod.rs#L14), [705](../src/material/mod.rs#L705), [src/material/refractiveindex.rs:4](../src/material/refractiveindex.rs#L4)
- Other documents: [ROADMAP.md:746](../ROADMAP.md?plain=1#L746)

#### Poulson et al. 2013

J. Poulson, B. Engquist, S. Li, L. Ying, "A Parallel Sweeping Preconditioner for Heterogeneous 3D Helmholtz Equations", SIAM J. Sci. Comput. 35, C194 (2013). [doi:10.1137/120871985](https://doi.org/10.1137/120871985)

- Other documents: [docs/plans/performance.md:637](plans/performance.md?plain=1#L637)

#### Powell et al. 1966

R. W. Powell, C. Y. Ho, P. E. Liley, "Thermal conductivity of selected materials", National Bureau of Standards (1966). [doi:10.6028/NBS.NSRDS.8](https://doi.org/10.6028/NBS.NSRDS.8)

- Methods pages: [catalogue.md:112](methods/catalogue.md?plain=1#L112)
- Validation cases: [`material/conductivity-tables`](validation.md?plain=1#L33)
- Code: [src/material/catalogue/conductivity.rs:13](../src/material/catalogue/conductivity.rs#L13), [src/material/catalogue/references.rs:459](../src/material/catalogue/references.rs#L459), [src/validation.rs:256](../src/validation.rs#L256)

#### Probst et al. 2024

M. J. Probst, A. Khurana, J. B. Slaby, A. M. Hammond, S. E. Ralph, "Fabrication tolerant multi-layer integrated photonic topology optimization", Opt. Express 32, 31448 (2024). [doi:10.1364/OE.527442](https://doi.org/10.1364/OE.527442)

- Other documents: [docs/plans/optimization.md:602](plans/optimization.md?plain=1#L602)

#### PsiQuantum team et al. 2025

PsiQuantum team, K. Alexander, A. Benyamini, D. Black, D. Bonneau, S. Burgos, B. Burridge, H. Cable, G. Campbell, G. Catalano, et al., "A manufacturable platform for photonic quantum computing", Nature 641, 876 (2025). [doi:10.1038/s41586-025-08820-7](https://doi.org/10.1038/s41586-025-08820-7), open access.

- Other documents: [docs/plans/quantum.md:524](plans/quantum.md?plain=1#L524)

#### Pu et al. 2016

M. Pu, L. Ottaviano, E. Semenova, K. Yvind, "Efficient frequency comb generation in AlGaAs-on-insulator", Optica 3, 823 (2016). [doi:10.1364/OPTICA.3.000823](https://doi.org/10.1364/OPTICA.3.000823)

- Examples: [README.md:88](../examples/README.md?plain=1#L88), [comb_pu.rs:5](../examples/comb_pu.rs#L5)
- Other documents: [ROADMAP.md:858](../ROADMAP.md?plain=1#L858)

#### Puckett et al. 2021

M. W. Puckett, K. Liu, N. Chauhan, Q. Zhao, N. Jin, H. Cheng, J. Wu, R. O. Behunin, P. T. Rakich, K. D. Nelson, et al., "422 Million intrinsic quality factor planar integrated all-waveguide resonator with sub-MHz linewidth", Nat Commun 12, 934 (2021). [doi:10.1038/s41467-021-21205-4](https://doi.org/10.1038/s41467-021-21205-4), open access.

- Academy lessons: [ring-resonator.md:109](../academy/ring-resonator.md?plain=1#L109), [399](../academy/ring-resonator.md?plain=1#L399), [592](../academy/ring-resonator.md?plain=1#L592)

### Q

#### Qi et al. 2020

H. Qi, D. J. Brod, N. Quesada, R. García-Patrón, "Regimes of Classical Simulability for Noisy Gaussian Boson Sampling", Phys. Rev. Lett. 124, 100502 (2020). [doi:10.1103/PhysRevLett.124.100502](https://doi.org/10.1103/PhysRevLett.124.100502)

- Other documents: [docs/plans/quantum.md:503](plans/quantum.md?plain=1#L503)

#### Quesada et al. 2018

N. Quesada, J. M. Arrazola, N. Killoran, "Gaussian boson sampling using threshold detectors", Phys. Rev. A 98, 062322 (2018). [doi:10.1103/PhysRevA.98.062322](https://doi.org/10.1103/PhysRevA.98.062322)

- Other documents: [docs/plans/quantum.md:501](plans/quantum.md?plain=1#L501)

### R

#### Rahnamayan et al. 2008

S. Rahnamayan, H. Tizhoosh, M. Salama, "Opposition-Based Differential Evolution", IEEE Trans. Evol. Computat. 12, 64 (2008). [doi:10.1109/TEVC.2007.894200](https://doi.org/10.1109/TEVC.2007.894200)

- Other documents: [docs/plans/optimization.md:583](plans/optimization.md?plain=1#L583)

#### Ramadan et al. 1998

T. Ramadan, R. Scarmozzino, R. Osgood, "Adiabatic couplers: design rules and optimization", J. Lightwave Technol. 16, 277 (1998). [doi:10.1109/50.661021](https://doi.org/10.1109/50.661021)

- Academy lessons: [directional-coupler.md:64](../academy/directional-coupler.md?plain=1#L64), [338](../academy/directional-coupler.md?plain=1#L338)

#### Rannacher 1984

R. Rannacher, "Finite element solution of diffusion problems with irregular data", Numer. Math. 43, 309 (1984). [doi:10.1007/BF01390130](https://doi.org/10.1007/BF01390130)

- Methods pages: [static.md:24](methods/static.md?plain=1#L24)
- Validation cases: [`static/transient-slab-order-time-cn`](validation.md?plain=1#L512)
- Code: [src/cases/statics/checks.rs:787](../src/cases/statics/checks.rs#L787), [src/thermal.rs:104](../src/thermal.rs#L104)

#### Raussendorf and Briegel 2001

R. Raussendorf, H. J. Briegel, "A One-Way Quantum Computer", Phys. Rev. Lett. 86, 5188 (2001). [doi:10.1103/PhysRevLett.86.5188](https://doi.org/10.1103/PhysRevLett.86.5188)

- Other documents: [docs/plans/quantum.md:487](plans/quantum.md?plain=1#L487)

#### Rayleigh 1887

L. Rayleigh, "XVII. On the maintenance of vibrations by forces of double frequency, and on the propagation of waves through a medium endowed with a periodic structure", The London, Edinburgh, and Dublin Philosophical Magazine and Journal of Science 24, 145 (1887). [doi:10.1080/14786448708628074](https://doi.org/10.1080/14786448708628074)

- Academy lessons: [bragg-gratings.md:19](../academy/bragg-gratings.md?plain=1#L19)

#### Raza et al. 2024

S. Raza, M. Hammood, N. A. F. Jaeger, L. Chrostowski, "Fabrication-aware inverse design with shape optimization for photonic integrated circuits", Opt. Lett. 50, 117 (2024). [doi:10.1364/OL.543961](https://doi.org/10.1364/OL.543961)

- Other documents: [ROADMAP.md:831](../ROADMAP.md?plain=1#L831), [docs/plans/optimization.md:603](plans/optimization.md?plain=1#L603)

#### Reck et al. 1994

M. Reck, A. Zeilinger, H. J. Bernstein, P. Bertani, "Experimental realization of any discrete unitary operator", Phys. Rev. Lett. 73, 58 (1994). [doi:10.1103/PhysRevLett.73.58](https://doi.org/10.1103/PhysRevLett.73.58)

- Academy lessons: [mach-zehnder.md:52](../academy/mach-zehnder.md?plain=1#L52), [368](../academy/mach-zehnder.md?plain=1#L368)
- Other documents: [ROADMAP.md:820](../ROADMAP.md?plain=1#L820), [docs/plans/optimization.md:618](plans/optimization.md?plain=1#L618), [docs/plans/quantum.md:478](plans/quantum.md?plain=1#L478)

#### Reddy et al. 2020

D. V. Reddy, R. R. Nerem, S. W. Nam, R. P. Mirin, V. B. Verma, "Superconducting nanowire single-photon detectors with 98% system detection efficiency at 1550 nm", Optica 7, 1649 (2020). [doi:10.1364/OPTICA.400751](https://doi.org/10.1364/OPTICA.400751)

- Other documents: [docs/plans/quantum.md:536](plans/quantum.md?plain=1#L536)

#### Reed et al. 2010

G. T. Reed, G. Mashanovich, F. Y. Gardes, D. J. Thomson, "Silicon optical modulators", Nature Photon 4, 518 (2010). [doi:10.1038/nphoton.2010.179](https://doi.org/10.1038/nphoton.2010.179)

- Academy lessons: [mach-zehnder.md:82](../academy/mach-zehnder.md?plain=1#L82), [299](../academy/mach-zehnder.md?plain=1#L299), [458](../academy/mach-zehnder.md?plain=1#L458)

#### Reps et al. 2010

B. Reps, W. Vanroose, H. B. Zubair, "On the indefinite Helmholtz equation: Complex stretched absorbing boundary layers, iterative analysis, and preconditioning", Journal of Computational Physics 229, 8384 (2010). [doi:10.1016/j.jcp.2010.07.022](https://doi.org/10.1016/j.jcp.2010.07.022)

- Methods pages: [fdfd-3d.md:48](methods/fdfd-3d.md?plain=1#L48), [809](methods/fdfd-3d.md?plain=1#L809), [multigrid-stencil.md:10](methods/multigrid-stencil.md?plain=1#L10)
- Validation cases: [`fdfd3d/gmres-multigrid-direct`](validation.md?plain=1#L180), [`device/fdfd-3d-iterative`](validation.md?plain=1#L336)
- Code: [src/device/splitter_checks.rs:110](../src/device/splitter_checks.rs#L110), [src/fdfd/three/iterative.rs:378](../src/fdfd/three/iterative.rs#L378), [src/fdfd/three/multigrid.rs:5](../src/fdfd/three/multigrid.rs#L5), [src/validation.rs:1285](../src/validation.rs#L1285)
- Other documents: [docs/plans/performance.md:583](plans/performance.md?plain=1#L583)

#### Requicha 1980

A. G. Requicha, "Representations for Rigid Solids: Theory, Methods, and Systems", ACM Comput. Surv. 12, 437 (1980). [doi:10.1145/356827.356833](https://doi.org/10.1145/356827.356833)

- Methods pages: [booleans.md:10](methods/booleans.md?plain=1#L10)

#### Ricci 1973

A. Ricci, "A constructive geometry for computer graphics", The Computer Journal 16, 157 (1973). [doi:10.1093/comjnl/16.2.157](https://doi.org/10.1093/comjnl/16.2.157)

- Methods pages: [booleans.md:8](methods/booleans.md?plain=1#L8)
- Validation cases: [`geometry/booleans/distance`](validation.md?plain=1#L424)
- Code: [src/geometry/boolean_checks.rs:770](../src/geometry/boolean_checks.rs#L770)

#### Richardson 1911

L. F. Richardson, "IX. The approximate arithmetical solution by finite differences of physical problems involving differential equations, with an application to the stresses in a masonry dam", Philosophical Transactions of the Royal Society of London. Series A, Containing Papers of a Mathematical or Physical Character 210, 307 (1911). [doi:10.1098/rsta.1911.0009](https://doi.org/10.1098/rsta.1911.0009)

- Methods pages: [bodies.md:12](methods/bodies.md?plain=1#L12), [convergence.md:8](methods/convergence.md?plain=1#L8)
- Code: [src/convergence.rs:38](../src/convergence.rs#L38)

#### Rigal and Gaches 1967

J. L. Rigal, J. Gaches, "On the Compatibility of a Given Solution With the Data of a Linear System", J. ACM 14, 543 (1967). [doi:10.1145/321406.321416](https://doi.org/10.1145/321406.321416)

- Code: [native/src/accelerate.rs:32](../native/src/accelerate.rs#L32)

#### Rigler et al. 2013

M. Rigler, M. Zgonik, M. P. Hoffmann, R. Kirste, M. Bobea, R. Collazo, Z. Sitar, S. Mita, M. Gerhold, "Refractive index of III-metal-polar and N-polar AlGaN waveguides grown by metal organic chemical vapor deposition", Applied Physics Letters 102, 221106 (2013). [doi:10.1063/1.4800554](https://doi.org/10.1063/1.4800554)

- Methods pages: [catalogue.md:94](methods/catalogue.md?plain=1#L94)
- Code: [src/material/catalogue/models.rs:1019](../src/material/catalogue/models.rs#L1019), [src/material/catalogue/references.rs:396](../src/material/catalogue/references.rs#L396)

#### Rigler et al. 2015

M. Rigler, J. Buh, M. P. Hoffmann, R. Kirste, M. Bobea, S. Mita, M. D. Gerhold, R. Collazo, Z. Sitar, M. Zgonik, "Optical characterization of Al- and N-polar AlN waveguides for integrated optics", Appl. Phys. Express 8, 042603 (2015). [doi:10.7567/APEX.8.042603](https://doi.org/10.7567/APEX.8.042603)

- Methods pages: [catalogue.md:92](methods/catalogue.md?plain=1#L92)
- Validation cases: [`material/aln-rigler-se`](validation.md?plain=1#L90), [`material/aln-rigler-maie`](validation.md?plain=1#L91)
- Code: [src/material/catalogue/models.rs:975](../src/material/catalogue/models.rs#L975), [src/material/catalogue/references.rs:389](../src/material/catalogue/references.rs#L389), [src/validation.rs:655](../src/validation.rs#L655), [662](../src/validation.rs#L662)

#### Ríos et al. 2015

C. Ríos, M. Stegmaier, P. Hosseini, D. Wang, T. Scherer, C. D. Wright, H. Bhaskaran, W. H. P. Pernice, "Integrated all-photonic non-volatile multi-level memory", Nature Photon 9, 725 (2015). [doi:10.1038/nphoton.2015.182](https://doi.org/10.1038/nphoton.2015.182)

- Methods pages: [fabrication.md:42](methods/fabrication.md?plain=1#L42)
- Validation cases: [`fabrication/nitride-reported-flows`](validation.md?plain=1#L757), [`fabrication/pcm-rules`](validation.md?plain=1#L759)
- Code: [src/fabrication/platform_checks.rs:728](../src/fabrication/platform_checks.rs#L728), [742](../src/fabrication/platform_checks.rs#L742), [src/fabrication/references.rs:184](../src/fabrication/references.rs#L184)
- Other documents: [docs/plans/fabrication.md:659](plans/fabrication.md?plain=1#L659)

#### Roache 1997

P. J. Roache, "QUANTIFICATION OF UNCERTAINTY IN COMPUTATIONAL FLUID DYNAMICS", Annu. Rev. Fluid Mech. 29, 123 (1997). [doi:10.1146/annurev.fluid.29.1.123](https://doi.org/10.1146/annurev.fluid.29.1.123)

- Methods pages: [convergence.md:10](methods/convergence.md?plain=1#L10)
- Code: [src/convergence.rs:40](../src/convergence.rs#L40)

#### Roberts 1992

D. Roberts, "Simplified characterization of uniaxial and biaxial nonlinear optical crystals: a plea for standardization of nomenclature and conventions", IEEE J. Quantum Electron. 28, 2057 (1992). [doi:10.1109/3.159516](https://doi.org/10.1109/3.159516)

- Methods pages: [catalogue.md:62](methods/catalogue.md?plain=1#L62)
- Code: [src/material/catalogue/mod.rs:157](../src/material/catalogue/mod.rs#L157), [src/material/catalogue/references.rs:214](../src/material/catalogue/references.rs#L214), [src/material/catalogue/tags.rs:38](../src/material/catalogue/tags.rs#L38), [src/material/crystal.rs:7](../src/material/crystal.rs#L7)

#### Roden and Gedney 2000

J. A. Roden, S. D. Gedney, "Convolution PML (CPML): An efficient FDTD implementation of the CFS-PML for arbitrary media", Microw. Opt. Technol. Lett. 27, 334 (2000). [doi:10.1002/1098-2760(20001205)27:5\<334::AID-MOP14\>3.0.CO;2-A](https://doi.org/10.1002/1098-2760(20001205)27:5%3C334::AID-MOP14%3E3.0.CO;2-A)

- Methods pages: [fdtd-adjoint.md:14](methods/fdtd-adjoint.md?plain=1#L14), [fdtd.md:26](methods/fdtd.md?plain=1#L26)
- Examples: [README.md:63](../examples/README.md?plain=1#L63), [cpml_roden_gedney.rs:5](../examples/cpml_roden_gedney.rs#L5)
- Code: [src/fdtd.rs:21](../src/fdtd.rs#L21), [src/validation/gpu.rs:248](../src/validation/gpu.rs#L248)
- Other documents: [ROADMAP.md:717](../ROADMAP.md?plain=1#L717), [docs/validation-gpu.md:22](validation-gpu.md?plain=1#L22)

#### Rumpf 2012

R. C. Rumpf, "SIMPLE IMPLEMENTATION OF ARBITRARILY SHAPED TOTAL-FIELD/SCATTERED-FIELD REGIONS IN FINITE-DIFFERENCE FREQUENCY-DOMAIN", PIER B 36, 221 (2012). [doi:10.2528/PIERB11092006](https://doi.org/10.2528/PIERB11092006)

- Methods pages: [devices.md:34](methods/devices.md?plain=1#L34), [fdfd-3d.md:26](methods/fdfd-3d.md?plain=1#L26), [fdfd-ports.md:8](methods/fdfd-ports.md?plain=1#L8), [fdtd.md:44](methods/fdtd.md?plain=1#L44)
- Validation cases: [`fdfd/straight-guide`](validation.md?plain=1#L144), [`fdfd3d/straight-strip`](validation.md?plain=1#L167), [`fdtd/mode-source-fdfd`](validation.md?plain=1#L212), [`device/fdfd-straight`](validation.md?plain=1#L335)
- Code: [src/device/mmi.rs:316](../src/device/mmi.rs#L316), [src/device/splitter_checks.rs:103](../src/device/splitter_checks.rs#L103), [src/fdfd/ports.rs:12](../src/fdfd/ports.rs#L12), [src/fdfd/three/ports.rs:18](../src/fdfd/three/ports.rs#L18), [src/validation.rs:1033](../src/validation.rs#L1033), [1194](../src/validation.rs#L1194), [1509](../src/validation.rs#L1509)

#### Ryser 1963

H. Ryser, "Combinatorial Mathematics", American Mathematical Society (1963). [doi:10.5948/UPO9781614440147](https://doi.org/10.5948/UPO9781614440147)

- Other documents: [docs/plans/quantum.md:496](plans/quantum.md?plain=1#L496)

### S

#### S. Afshar V. and Monro 2009

S. Afshar V., T. M. Monro, "A full vectorial model for pulse propagation in emerging waveguides with subwavelength structures part I: Kerr nonlinearity", Opt. Express 17, 2298 (2009). [doi:10.1364/OE.17.002298](https://doi.org/10.1364/OE.17.002298)

- Methods pages: [fields.md:10](methods/fields.md?plain=1#L10), [nonlinear.md:16](methods/nonlinear.md?plain=1#L16)
- Validation cases: [`nonlinear/kerr-gamma-gaussian`](validation.md?plain=1#L734)
- Code: [src/mode/quantities.rs:10](../src/mode/quantities.rs#L10), [src/nonlinear/data_checks.rs:326](../src/nonlinear/data_checks.rs#L326)
- Other documents: [ROADMAP.md:736](../ROADMAP.md?plain=1#L736)

#### Saad 2003

Y. Saad, "Iterative Methods for Sparse Linear Systems", Society for Industrial and Applied Mathematics (2003). [doi:10.1137/1.9780898718003](https://doi.org/10.1137/1.9780898718003)

- Methods pages: [backends.md:350](methods/backends.md?plain=1#L350), [fdfd-3d.md:46](methods/fdfd-3d.md?plain=1#L46), [recycling.md:14](methods/recycling.md?plain=1#L14)
- Validation cases: [`fdfd3d/qmr-ilu-direct`](validation.md?plain=1#L179), [`fdfd3d/gmres-multigrid-direct`](validation.md?plain=1#L180), [`fdfd3d/block-qmr-ilu-s-matrix`](validation.md?plain=1#L182)
- Code: [src/backend/iterative.rs:89](../src/backend/iterative.rs#L89), [src/backend/krylov.rs:487](../src/backend/krylov.rs#L487), [1384](../src/backend/krylov.rs#L1384), [src/fdfd/three/iterative.rs:334](../src/fdfd/three/iterative.rs#L334), [375](../src/fdfd/three/iterative.rs#L375), [src/validation.rs:1278](../src/validation.rs#L1278), [1285](../src/validation.rs#L1285), [1299](../src/validation.rs#L1299)
- Other documents: [docs/plans/performance.md:578](plans/performance.md?plain=1#L578)

#### Saad 2011

Y. Saad, "Numerical Methods for Large Eigenvalue Problems: Revised Edition", Society for Industrial and Applied Mathematics (2011). [doi:10.1137/1.9781611970739](https://doi.org/10.1137/1.9781611970739)

- Methods pages: [eigen.md:8](methods/eigen.md?plain=1#L8)
- Validation cases: [`mode/contour-strip-shift-invert`](validation.md?plain=1#L463), [`mode/contour-leaky-shift-invert`](validation.md?plain=1#L464), [`mode/krylov-schur-growing`](validation.md?plain=1#L472)
- Code: [src/cases/eigen/checks.rs:339](../src/cases/eigen/checks.rs#L339), [src/eigen.rs:5](../src/eigen.rs#L5), [src/mode/region/checks.rs:359](../src/mode/region/checks.rs#L359), [366](../src/mode/region/checks.rs#L366)
- Other documents: [ROADMAP.md:729](../ROADMAP.md?plain=1#L729), [docs/plans/performance.md:579](plans/performance.md?plain=1#L579)

#### Sacher and Poon 2008

W. D. Sacher, J. K. S. Poon, "Dynamics of microring resonator modulators", Opt. Express 16, 15741 (2008). [doi:10.1364/OE.16.015741](https://doi.org/10.1364/OE.16.015741)

- Other documents: [ROADMAP.md:814](../ROADMAP.md?plain=1#L814)

#### Sakurai and Sugiura 2003

T. Sakurai, H. Sugiura, "A projection method for generalized eigenvalue problems using numerical integration", Journal of Computational and Applied Mathematics 159, 119 (2003). [doi:10.1016/S0377-0427(03)00565-X](https://doi.org/10.1016/S0377-0427(03)00565-X)

- Methods pages: [contour.md:8](methods/contour.md?plain=1#L8)
- Academy lessons: [leaky-and-bent-guides.md:89](../academy/leaky-and-bent-guides.md?plain=1#L89)
- Validation cases: [`mode/contour-exponential`](validation.md?plain=1#L467), [`mode/contour-sakurai-sugiura`](validation.md?plain=1#L469)
- Code: [src/eigen/contour.rs:11](../src/eigen/contour.rs#L11), [src/mode/region.rs:8](../src/mode/region.rs#L8), [src/mode/region/checks.rs:387](../src/mode/region/checks.rs#L387), [401](../src/mode/region/checks.rs#L401)
- Other documents: [docs/plans/performance.md:638](plans/performance.md?plain=1#L638)

#### Sanathanan and Koerner 1963

C. Sanathanan, J. Koerner, "Transfer function synthesis as a ratio of two complex polynomials", IEEE Trans. Automat. Contr. 8, 56 (1963). [doi:10.1109/TAC.1963.1105517](https://doi.org/10.1109/TAC.1963.1105517)

- Methods pages: [compact.md:14](methods/compact.md?plain=1#L14)
- Validation cases: [`device/compact-ring-closed-form`](validation.md?plain=1#L351), [`compact/param-ring-coupling`](validation.md?plain=1#L385)
- Code: [src/cases/compact/checks.rs:768](../src/cases/compact/checks.rs#L768), [src/compact/param.rs:31](../src/compact/param.rs#L31), [1361](../src/compact/param.rs#L1361), [src/device/compact.rs:709](../src/device/compact.rs#L709), [src/job/fit_checks.rs:429](../src/job/fit_checks.rs#L429)

#### Sanford et al. 2003

N. A. Sanford, L. H. Robins, A. V. Davydov, A. Shapiro, D. V. Tsvetkov, A. V. Dmitriev, S. Keller, U. K. Mishra, S. P. DenBaars, "Refractive index study of AlxGa1−xN films grown on sapphire substrates", Journal of Applied Physics 94, 2980 (2003). [doi:10.1063/1.1598276](https://doi.org/10.1063/1.1598276)

- Code: [src/material/catalogue/coming.rs:123](../src/material/catalogue/coming.rs#L123)

#### Sanford et al. 2005

N. A. Sanford, A. V. Davydov, D. V. Tsvetkov, A. V. Dmitriev, S. Keller, U. K. Mishra, S. P. DenBaars, S. S. Park, J. Y. Han, R. J. Molnar, "Measurement of second order susceptibilities of GaN and AlGaN", Journal of Applied Physics 97, 053512 (2005). [doi:10.1063/1.1852695](https://doi.org/10.1063/1.1852695)

- Code: [src/material/catalogue/references.rs:193](../src/material/catalogue/references.rs#L193)

#### Scharfetter and Gummel 1969

D. Scharfetter, H. Gummel, "Large-signal analysis of a silicon Read diode oscillator", IEEE Trans. Electron Devices 16, 64 (1969). [doi:10.1109/T-ED.1969.16566](https://doi.org/10.1109/T-ED.1969.16566)

- Other documents: [ROADMAP.md:807](../ROADMAP.md?plain=1#L807)

#### Schenk and Gärtner 2004

O. Schenk, K. Gärtner, "Solving unsymmetric sparse systems of linear equations with PARDISO", Future Generation Computer Systems 20, 475 (2004). [doi:10.1016/j.future.2003.07.011](https://doi.org/10.1016/j.future.2003.07.011)

- Code: [native/src/pardiso.rs:3](../native/src/pardiso.rs#L3), [src/bench/export.rs:4](../src/bench/export.rs#L4)
- Other documents: [docs/baselines.md:5](baselines.md?plain=1#L5), [docs/plans/performance.md:639](plans/performance.md?plain=1#L639)

#### Schneider et al. 2019

P.-I. Schneider, X. Garcia Santiago, V. Soltwisch, M. Hammerschmidt, S. Burger, C. Rockstuhl, "Benchmarking Five Global Optimization Approaches for Nano-optical Shape Optimization and Parameter Reconstruction", ACS Photonics 6, 2726 (2019). [doi:10.1021/acsphotonics.9b00706](https://doi.org/10.1021/acsphotonics.9b00706)

- Methods pages: [optimization.md:288](methods/optimization.md?plain=1#L288)
- Other documents: [docs/plans/optimization.md:612](plans/optimization.md?plain=1#L612)

#### Schubert et al. 1995

M. Schubert, V. Gottschalch, C. M. Herzinger, H. Yao, P. G. Snyder, J. A. Woollam, "Optical constants of GaxIn1−xP lattice matched to GaAs", Journal of Applied Physics 77, 3416 (1995). [doi:10.1063/1.358632](https://doi.org/10.1063/1.358632)

- Code: [src/material/catalogue/references.rs:326](../src/material/catalogue/references.rs#L326)

#### Schubert et al. 2022

M. F. Schubert, A. K. C. Cheung, I. A. D. Williamson, A. Spyra, D. H. Alexander, "Inverse Design of Photonic Devices with Strict Foundry Fabrication Constraints", ACS Photonics 9, 2327 (2022). [doi:10.1021/acsphotonics.2c00313](https://doi.org/10.1021/acsphotonics.2c00313), open access.

- Other documents: [ROADMAP.md:777](../ROADMAP.md?plain=1#L777), [docs/plans/optimization.md:599](plans/optimization.md?plain=1#L599)

#### Selberherr 1984

S. Selberherr, "Analysis and Simulation of Semiconductor Devices", Springer Vienna (1984). [doi:10.1007/978-3-7091-8752-4](https://doi.org/10.1007/978-3-7091-8752-4)

- Other documents: [ROADMAP.md:808](../ROADMAP.md?plain=1#L808)

#### Sellmeier 1871

Sellmeier, "Zur Erklärung der abnormen Farbenfolge im Spectrum einiger Substanzen", Annalen der Physik 219, 272 (1871). [doi:10.1002/andp.18712190612](https://doi.org/10.1002/andp.18712190612)

- Academy lessons: [material-dispersion.md:20](../academy/material-dispersion.md?plain=1#L20), [104](../academy/material-dispersion.md?plain=1#L104), [433](../academy/material-dispersion.md?plain=1#L433)

#### Seltzer 1993

S. Seltzer, "Stopping-Powers and Range Tables for Electrons, Protons, and Helium Ions, NIST Standard Reference Database 124", National Institute of Standards and Technology (1993). [doi:10.18434/T4NC7P](https://doi.org/10.18434/T4NC7P)

- Methods pages: [fabrication.md:10](methods/fabrication.md?plain=1#L10)
- Validation cases: [`fabrication/smart-cut-range-bruel`](validation.md?plain=1#L765)
- Code: [src/fabrication/references.rs:296](../src/fabrication/references.rs#L296), [src/fabrication/smart_cut.rs:40](../src/fabrication/smart_cut.rs#L40), [src/fabrication/smart_cut_checks.rs:124](../src/fabrication/smart_cut_checks.rs#L124)
- Other documents: [docs/plans/fabrication.md:630](plans/fabrication.md?plain=1#L630)

#### Selvaraja et al. 2009

S. Selvaraja, P. Jaenen, W. Bogaerts, D. Van Thourhout, P. Dumon, R. Baets, "Fabrication of Photonic Wire and Crystal Circuits in Silicon-on-Insulator Using 193-nm Optical Lithography", J. Lightwave Technol. 27, 4076 (2009). [doi:10.1109/JLT.2009.2022282](https://doi.org/10.1109/JLT.2009.2022282)

- Methods pages: [fabrication.md:12](methods/fabrication.md?plain=1#L12)
- Validation cases: [`fabrication/etch-stop-or-timed`](validation.md?plain=1#L745), [`fabrication/lithography-choice`](validation.md?plain=1#L746), [`fabrication/reported-flows`](validation.md?plain=1#L748), [`fabrication/rule-choices`](validation.md?plain=1#L752)
- Code: [src/fabrication/checks.rs:440](../src/fabrication/checks.rs#L440), [447](../src/fabrication/checks.rs#L447), [461](../src/fabrication/checks.rs#L461), [src/fabrication/references.rs:48](../src/fabrication/references.rs#L48), [src/fabrication/rule_checks.rs:344](../src/fabrication/rule_checks.rs#L344)
- Other documents: [docs/plans/fabrication.md:636](plans/fabrication.md?plain=1#L636)

#### Shahriari et al. 2016

B. Shahriari, K. Swersky, Z. Wang, R. P. Adams, N. de Freitas, "Taking the Human Out of the Loop: A Review of Bayesian Optimization", Proc. IEEE 104, 148 (2016). [doi:10.1109/JPROC.2015.2494218](https://doi.org/10.1109/JPROC.2015.2494218)

- Other documents: [docs/plans/optimization.md:585](plans/optimization.md?plain=1#L585)

#### Shang et al. 2023

C. Shang, J. Yang, A. M. Hammond, Z. Chen, M. Chen, Z. Lin, S. G. Johnson, C. Wang, "Inverse-Designed Lithium Niobate Nanophotonics", ACS Photonics, acsphotonics.3c00040 (2023). [doi:10.1021/acsphotonics.3c00040](https://doi.org/10.1021/acsphotonics.3c00040)

- Other documents: [docs/plans/optimization.md:600](plans/optimization.md?plain=1#L600)

#### Shantsev et al. 2017

D. V. Shantsev, P. Jaysaval, S. de la Kethulle de Ryhove, P. R. Amestoy, A. Buttari, J.-Y. L’Excellent, T. Mary, "Large-scale 3-D EM modelling with a Block Low-Rank multifrontal direct solver", Geophysical Journal International 209, 1558 (2017). [doi:10.1093/gji/ggx106](https://doi.org/10.1093/gji/ggx106)

- Other documents: [docs/baselines.md:406](baselines.md?plain=1#L406), [docs/plans/performance.md:640](plans/performance.md?plain=1#L640)

#### Shen et al. 2015

B. Shen, P. Wang, R. Polson, R. Menon, "An integrated-nanophotonics polarization beamsplitter with 2.4 × 2.4 μm2 footprint", Nature Photon 9, 378 (2015). [doi:10.1038/nphoton.2015.80](https://doi.org/10.1038/nphoton.2015.80)

- Other documents: [ROADMAP.md:764](../ROADMAP.md?plain=1#L764), [docs/plans/optimization.md:609](plans/optimization.md?plain=1#L609)

#### Shen et al. 2020

B. Shen, L. Chang, J. Liu, H. Wang, Q.-F. Yang, C. Xiang, R. N. Wang, J. He, T. Liu, W. Xie, et al., "Integrated turnkey soliton microcombs", Nature 582, 365 (2020). [doi:10.1038/s41586-020-2358-x](https://doi.org/10.1038/s41586-020-2358-x)

- Academy lessons: [ring-resonator.md:103](../academy/ring-resonator.md?plain=1#L103), [591](../academy/ring-resonator.md?plain=1#L591)

#### Shin and Fan 2012

W. Shin, S. Fan, "Choice of the perfectly matched layer boundary condition for frequency-domain Maxwell’s equations solvers", Journal of Computational Physics 231, 3406 (2012). [doi:10.1016/j.jcp.2012.01.013](https://doi.org/10.1016/j.jcp.2012.01.013)

- Methods pages: [fdfd-3d.md:12](methods/fdfd-3d.md?plain=1#L12), [fdfd.md:10](methods/fdfd.md?plain=1#L10)
- Validation cases: [`fdfd/pml-reflection`](validation.md?plain=1#L134), [`fdfd3d/pml-reflection`](validation.md?plain=1#L162)
- Code: [src/fdfd/mod.rs:29](../src/fdfd/mod.rs#L29), [src/fdfd/three.rs:18](../src/fdfd/three.rs#L18), [src/validation.rs:963](../src/validation.rs#L963), [1159](../src/validation.rs#L1159)

#### Shin and Fan 2013

W. Shin, S. Fan, "Accelerated solution of the frequency-domain Maxwell’s equations by engineering the eigenvalue distribution of the operator", Opt. Express 21, 22578 (2013). [doi:10.1364/OE.21.022578](https://doi.org/10.1364/OE.21.022578)

- Methods pages: [fdfd-3d.md:16](methods/fdfd-3d.md?plain=1#L16), [multigrid-stencil.md:12](methods/multigrid-stencil.md?plain=1#L12)
- Validation cases: [`fdfd3d/qmr-direct`](validation.md?plain=1#L172), [`fdfd3d/qmr-plateau`](validation.md?plain=1#L175), [`fdfd3d/qmr-iterations-curl-curl`](validation.md?plain=1#L176), [`fdfd3d/qmr-iterations-shin-fan`](validation.md?plain=1#L177), [`device/fdfd-3d-iterative`](validation.md?plain=1#L336)
- Code: [src/device/splitter_checks.rs:110](../src/device/splitter_checks.rs#L110), [src/fdfd/mod.rs:21](../src/fdfd/mod.rs#L21), [src/fdfd/three.rs:392](../src/fdfd/three.rs#L392), [src/fdfd/three/iterative.rs:105](../src/fdfd/three/iterative.rs#L105), [src/validation.rs:1229](../src/validation.rs#L1229), [1250](../src/validation.rs#L1250), [1257](../src/validation.rs#L1257), [1264](../src/validation.rs#L1264)
- Other documents: [docs/plans/performance.md:584](plans/performance.md?plain=1#L584)

#### Shoji et al. 1997

I. Shoji, T. Kondo, A. Kitamoto, M. Shirane, R. Ito, "Absolute scale of second-order nonlinear-optical coefficients", J. Opt. Soc. Am. B 14, 2268 (1997). [doi:10.1364/JOSAB.14.002268](https://doi.org/10.1364/JOSAB.14.002268)

- Methods pages: [catalogue.md:60](methods/catalogue.md?plain=1#L60), [nonlinear.md:14](methods/nonlinear.md?plain=1#L14)
- Validation cases: [`material/linbo3-shoji-miller`](validation.md?plain=1#L67), [`nonlinear/miller-linbo3-d33`](validation.md?plain=1#L735), [`nonlinear/miller-linbo3-d31`](validation.md?plain=1#L736), [`nonlinear/miller-linbo3-mgo-d33`](validation.md?plain=1#L737), [`nonlinear/miller-linbo3-mgo-d31`](validation.md?plain=1#L738)
- Code: [src/material/catalogue/references.rs:207](../src/material/catalogue/references.rs#L207), [src/nonlinear/data_checks.rs:333](../src/nonlinear/data_checks.rs#L333), [340](../src/nonlinear/data_checks.rs#L340), [347](../src/nonlinear/data_checks.rs#L347), [354](../src/nonlinear/data_checks.rs#L354), [src/validation.rs:494](../src/validation.rs#L494)
- Other documents: [ROADMAP.md:846](../ROADMAP.md?plain=1#L846)

#### Shoji et al. 2002

I. Shoji, T. Kondo, R. Ito, "Second-order nonlinear susceptibilities of various dielectric and semiconductor materials", Optical and Quantum Electronics 34, 797 (2002). [doi:10.1023/A:1016545417478](https://doi.org/10.1023/A:1016545417478)

- Code: [src/material/catalogue/coming.rs:49](../src/material/catalogue/coming.rs#L49), [112](../src/material/catalogue/coming.rs#L112), [src/material/catalogue/tests.rs:580](../src/material/catalogue/tests.rs#L580)

#### Shortley and Weller 1938

G. H. Shortley, R. Weller, "The Numerical Solution of Laplace's Equation", Journal of Applied Physics 9, 334 (1938). [doi:10.1063/1.1710426](https://doi.org/10.1063/1.1710426)

- Methods pages: [static.md:10](methods/static.md?plain=1#L10)
- Validation cases: [`static/coaxial`](validation.md?plain=1#L518)
- Code: [src/cases/statics/checks.rs:878](../src/cases/statics/checks.rs#L878), [src/statics.rs:34](../src/statics.rs#L34)

#### Siew et al. 2021

S. Y. Siew, B. Li, F. Gao, H. Y. Zheng, W. Zhang, P. Guo, S. W. Xie, A. Song, B. Dong, L. W. Luo, et al., "Review of Silicon Photonics Technology and Platform Development", J. Lightwave Technol. 39, 4374 (2021). [doi:10.1109/JLT.2021.3066203](https://doi.org/10.1109/JLT.2021.3066203), open access.

- Academy lessons: [strip-waveguide.md:142](../academy/strip-waveguide.md?plain=1#L142), [696](../academy/strip-waveguide.md?plain=1#L696)
- Other documents: [docs/plans/fabrication.md:640](plans/fabrication.md?plain=1#L640)

#### Sigmund 2009

O. Sigmund, "Manufacturing tolerant topology optimization", Acta Mech Sin 25, 227 (2009). [doi:10.1007/s10409-009-0240-z](https://doi.org/10.1007/s10409-009-0240-z)

- Other documents: [docs/plans/optimization.md:590](plans/optimization.md?plain=1#L590)

#### Sigmund 2011

O. Sigmund, "On the usefulness of non-gradient approaches in topology optimization", Struct Multidisc Optim 43, 589 (2011). [doi:10.1007/s00158-011-0638-7](https://doi.org/10.1007/s00158-011-0638-7)

- Other documents: [docs/plans/optimization.md:592](plans/optimization.md?plain=1#L592)

#### Silverstone et al. 2013

J. W. Silverstone, D. Bonneau, K. Ohira, N. Suzuki, H. Yoshida, N. Iizuka, M. Ezaki, C. M. Natarajan, M. G. Tanner, R. H. Hadfield, et al., "On-chip quantum interference between silicon photon-pair sources", Nature Photon 8, 104 (2013). [doi:10.1038/nphoton.2013.339](https://doi.org/10.1038/nphoton.2013.339)

- Other documents: [docs/plans/quantum.md:528](plans/quantum.md?plain=1#L528)

#### Sinatkas et al. 2021

G. Sinatkas, T. Christopoulos, O. Tsilipakos, E. E. Kriezis, "Electro-optic modulation in integrated photonics", Journal of Applied Physics 130, 010901 (2021). [doi:10.1063/5.0048712](https://doi.org/10.1063/5.0048712)

- Methods pages: [catalogue.md:124](methods/catalogue.md?plain=1#L124)
- Code: [src/material/catalogue/tags.rs:44](../src/material/catalogue/tags.rs#L44)

#### Skauli et al. 2002

T. Skauli, K. L. Vodopyanov, T. J. Pinguet, A. Schober, O. Levi, L. A. Eyres, M. M. Fejer, J. S. Harris, B. Gerard, L. Becouarn, et al., "Measurement of the nonlinear coefficient of orientation-patterned GaAs and demonstration of highly efficient second-harmonic generation", Opt. Lett. 27, 628 (2002). [doi:10.1364/OL.27.000628](https://doi.org/10.1364/OL.27.000628)

- Methods pages: [catalogue.md:72](methods/catalogue.md?plain=1#L72)
- Code: [src/material/catalogue/references.rs:270](../src/material/catalogue/references.rs#L270)

#### Skauli et al. 2003

T. Skauli, P. S. Kuo, K. L. Vodopyanov, T. J. Pinguet, O. Levi, L. A. Eyres, J. S. Harris, M. M. Fejer, B. Gerard, L. Becouarn, et al., "Improved dispersion relations for GaAs and applications to nonlinear optics", Journal of Applied Physics 94, 6447 (2003). [doi:10.1063/1.1621740](https://doi.org/10.1063/1.1621740)

- Methods pages: [catalogue.md:70](methods/catalogue.md?plain=1#L70)
- Validation cases: [`material/gaas-skauli-shg`](validation.md?plain=1#L83), [`material/gaas-skauli-dndt`](validation.md?plain=1#L84)
- Code: [src/material/catalogue/models.rs:781](../src/material/catalogue/models.rs#L781), [src/material/catalogue/references.rs:263](../src/material/catalogue/references.rs#L263), [src/material/mod.rs:127](../src/material/mod.rs#L127), [src/validation.rs:606](../src/validation.rs#L606), [613](../src/validation.rs#L613)
- Other documents: [ROADMAP.md:851](../ROADMAP.md?plain=1#L851)

#### Snitzer 1961

E. Snitzer, "Cylindrical Dielectric Waveguide Modes\*", J. Opt. Soc. Am. 51, 491 (1961). [doi:10.1364/JOSA.51.000491](https://doi.org/10.1364/JOSA.51.000491)

- Methods pages: [stacks.md:14](methods/stacks.md?plain=1#L14)
- Validation cases: [`stack/oblique-rod`](validation.md?plain=1#L488)
- Code: [src/cases/stack/checks.rs:289](../src/cases/stack/checks.rs#L289), [479](../src/cases/stack/checks.rs#L479)

#### Soldano and Pennings 1995

L. Soldano, E. Pennings, "Optical multi-mode interference devices based on self-imaging: principles and applications", J. Lightwave Technol. 13, 615 (1995). [doi:10.1109/50.372474](https://doi.org/10.1109/50.372474)

- Methods pages: [components.md:10](methods/components.md?plain=1#L10), [devices.md:30](methods/devices.md?plain=1#L30), [generators.md:12](methods/generators.md?plain=1#L12)
- Validation cases: [`components/mmi-beat-length-soldano`](validation.md?plain=1#L301), [`components/mmi-fdfd`](validation.md?plain=1#L302), [`device/mmi-self-imaging`](validation.md?plain=1#L329), [`device/mmi-guided-2d`](validation.md?plain=1#L338), [`device/mmi-guided-3d`](validation.md?plain=1#L339), [`device/mmi-best-power-3d`](validation.md?plain=1#L342)
- Examples: [README.md:77](../examples/README.md?plain=1#L77), [mmi_soldano.rs:5](../examples/mmi_soldano.rs#L5)
- Code: [src/circuit/components/mmi.rs:5](../src/circuit/components/mmi.rs#L5), [188](../src/circuit/components/mmi.rs#L188), [351](../src/circuit/components/mmi.rs#L351), [src/device/mmi.rs:309](../src/device/mmi.rs#L309), [src/device/splitter_checks.rs:61](../src/device/splitter_checks.rs#L61), [124](../src/device/splitter_checks.rs#L124), [131](../src/device/splitter_checks.rs#L131), [152](../src/device/splitter_checks.rs#L152), [src/validation.rs:2132](../src/validation.rs#L2132), [2139](../src/validation.rs#L2139)
- Jobs and circuits: [jobs/mmi-1x2-self-imaging.yaml:2](../jobs/mmi-1x2-self-imaging.yaml#L2), [jobs/mmi-2x2-self-imaging.yaml:2](../jobs/mmi-2x2-self-imaging.yaml#L2)
- Other documents: [ROADMAP.md:751](../ROADMAP.md?plain=1#L751)

#### Somekh et al. 1973

S. Somekh, E. Garmire, A. Yariv, H. Garvin, R. Hunsperger, "Channel optical waveguide directional couplers", Applied Physics Letters 22, 46 (1973). [doi:10.1063/1.1654468](https://doi.org/10.1063/1.1654468)

- Academy lessons: [directional-coupler.md:34](../academy/directional-coupler.md?plain=1#L34), [487](../academy/directional-coupler.md?plain=1#L487)

#### Sonneveld and van Gijzen 2008

P. Sonneveld, M. B. van Gijzen, "IDR( s ): A Family of Simple and Fast Algorithms for Solving Large Nonsymmetric Systems of Linear Equations", SIAM J. Sci. Comput. 31, 1035 (2008). [doi:10.1137/070685804](https://doi.org/10.1137/070685804)

- Other documents: [docs/plans/performance.md:641](plans/performance.md?plain=1#L641)

#### Soref 2006

R. Soref, "The Past, Present, and Future of Silicon Photonics", IEEE J. Select. Topics Quantum Electron. 12, 1678 (2006). [doi:10.1109/JSTQE.2006.883151](https://doi.org/10.1109/JSTQE.2006.883151)

- Academy lessons: [slab-waveguide.md:72](../academy/slab-waveguide.md?plain=1#L72), [582](../academy/slab-waveguide.md?plain=1#L582)

#### Soref and Bennett 1987

R. Soref, B. Bennett, "Electrooptical effects in silicon", IEEE J. Quantum Electron. 23, 123 (1987). [doi:10.1109/JQE.1987.1073206](https://doi.org/10.1109/JQE.1987.1073206)

- Academy lessons: [mach-zehnder.md:34](../academy/mach-zehnder.md?plain=1#L34), [297](../academy/mach-zehnder.md?plain=1#L297)
- Other documents: [ROADMAP.md:747](../ROADMAP.md?plain=1#L747)

#### Soref et al. 1991

R. Soref, J. Schmidtchen, K. Petermann, "Large single-mode rib waveguides in GeSi-Si and Si-on-SiO/sub 2/", IEEE J. Quantum Electron. 27, 1971 (1991). [doi:10.1109/3.83406](https://doi.org/10.1109/3.83406)

- Academy lessons: [strip-waveguide.md:64](../academy/strip-waveguide.md?plain=1#L64), [599](../academy/strip-waveguide.md?plain=1#L599), [681](../academy/strip-waveguide.md?plain=1#L681)

#### Spillane et al. 2013

N. Spillane, V. Dolean, P. Hauret, F. Nataf, C. Pechstein, R. Scheichl, "Abstract robust coarse spaces for systems of PDEs via generalized eigenproblems in the overlaps", Numer. Math. 126, 741 (2013). [doi:10.1007/s00211-013-0576-y](https://doi.org/10.1007/s00211-013-0576-y)

- Other documents: [docs/plans/performance.md:642](plans/performance.md?plain=1#L642)

#### St-Cyr et al. 2007

A. St-Cyr, M. J. Gander, S. J. Thomas, "Optimized Multiplicative, Additive, and Restricted Additive Schwarz Preconditioning", SIAM J. Sci. Comput. 29, 2402 (2007). [doi:10.1137/060652610](https://doi.org/10.1137/060652610)

- Other documents: [docs/plans/performance.md:643](plans/performance.md?plain=1#L643)

#### Steele et al. 2014

G. L. Steele, D. Lea, C. H. Flood, "Fast splittable pseudorandom number generators", Proceedings of the 2014 ACM International Conference on Object Oriented Programming Systems Languages & Applications (2014). [doi:10.1145/2660193.2660195](https://doi.org/10.1145/2660193.2660195)

- Code: [src/device/compact.rs:367](../src/device/compact.rs#L367)

#### Stewart 2002

G. W. Stewart, "A Krylov--Schur Algorithm for Large Eigenproblems", SIAM J. Matrix Anal. Appl. 23, 601 (2002). [doi:10.1137/S0895479800371529](https://doi.org/10.1137/S0895479800371529)

- Methods pages: [eigen.md:10](methods/eigen.md?plain=1#L10)
- Validation cases: [`mode/krylov-schur-box`](validation.md?plain=1#L470), [`mode/every-guided-mode`](validation.md?plain=1#L476)
- Code: [src/cases/eigen/checks.rs:325](../src/cases/eigen/checks.rs#L325), [367](../src/cases/eigen/checks.rs#L367), [src/eigen.rs:11](../src/eigen.rs#L11)
- Other documents: [docs/plans/performance.md:644](plans/performance.md?plain=1#L644)

#### Stolk 2013

C. C. Stolk, "A rapidly converging domain decomposition method for the Helmholtz equation", Journal of Computational Physics 241, 240 (2013). [doi:10.1016/j.jcp.2013.01.039](https://doi.org/10.1016/j.jcp.2013.01.039)

- Other documents: [docs/plans/performance.md:645](plans/performance.md?plain=1#L645)

#### Storn and Price 1997

R. Storn, K. Price, "Differential Evolution – A Simple and Efficient Heuristic for global Optimization over Continuous Spaces", Journal of Global Optimization 11, 341 (1997). [doi:10.1023/A:1008202821328](https://doi.org/10.1023/A:1008202821328)

- Methods pages: [optimization.md:137](methods/optimization.md?plain=1#L137)
- Code: [src/job/optimize.rs:226](../src/job/optimize.rs#L226)
- Other documents: [docs/plans/optimization.md:578](plans/optimization.md?plain=1#L578)

#### Su et al. 2018

L. Su, R. Trivedi, N. V. Sapra, A. Y. Piggott, D. Vercruysse, J. Vučković, "Fully-automated optimization of grating couplers", Opt. Express 26, 4023 (2018). [doi:10.1364/OE.26.004023](https://doi.org/10.1364/OE.26.004023)

- Other documents: [ROADMAP.md:766](../ROADMAP.md?plain=1#L766)

#### Su et al. 2020

L. Su, D. Vercruysse, J. Skarda, N. V. Sapra, J. A. Petykiewicz, J. Vučković, "Nanophotonic inverse design with SPINS: Software architecture and practical considerations", Applied Physics Reviews 7, 011407 (2020). [doi:10.1063/1.5131263](https://doi.org/10.1063/1.5131263)

- Other documents: [ROADMAP.md:771](../ROADMAP.md?plain=1#L771)

#### Suceava et al. 2025

A. Suceava, S. Hazra, A. Ross, I. R. Philippi, D. Sotir, B. Brower, L. Ding, Y. Zhu, Z. Zhang, H. Sarkar, et al., "Colossal Cryogenic Electro‐Optic Response Through Metastability in Strained BaTiO 3 Thin Films", Advanced Materials 38, e07564 (2025). [doi:10.1002/adma.202507564](https://doi.org/10.1002/adma.202507564), open access.

- Methods pages: [catalogue.md:56](methods/catalogue.md?plain=1#L56)
- Validation cases: [`material/batio3-suceava-reflectance`](validation.md?plain=1#L77)
- Code: [src/material/catalogue/batio3_film.rs:2](../src/material/catalogue/batio3_film.rs#L2), [125](../src/material/catalogue/batio3_film.rs#L125), [src/material/catalogue/references.rs:473](../src/material/catalogue/references.rs#L473), [src/validation.rs:564](../src/validation.rs#L564)

#### Sugie and Tada 1976

M. Sugie, K. Tada, "Measurements of the Linear Electrooptic Coefficients and Analysis of the Nonlinear Susceptibilities in Cubic GaAs and Hexagonal CdS", Jpn. J. Appl. Phys. 15, 421 (1976). [doi:10.1143/JJAP.15.421](https://doi.org/10.1143/JJAP.15.421)

- Methods pages: [catalogue.md:76](methods/catalogue.md?plain=1#L76)
- Code: [src/material/catalogue/references.rs:284](../src/material/catalogue/references.rs#L284)

#### Sutherland and Hodgman 1974

I. E. Sutherland, G. W. Hodgman, "Reentrant polygon clipping", Commun. ACM 17, 32 (1974). [doi:10.1145/360767.360802](https://doi.org/10.1145/360767.360802)

- Methods pages: [bodies.md:10](methods/bodies.md?plain=1#L10), [geometry.md:8](methods/geometry.md?plain=1#L8), [stacks.md:16](methods/stacks.md?plain=1#L16)
- Validation cases: [`geometry/fill`](validation.md?plain=1#L395), [`geometry/paths/fill`](validation.md?plain=1#L403), [`geometry/curves/fill`](validation.md?plain=1#L416)
- Code: [src/geometry/checks.rs:250](../src/geometry/checks.rs#L250), [src/geometry/curve_checks.rs:714](../src/geometry/curve_checks.rs#L714), [src/geometry/path_checks.rs:411](../src/geometry/path_checks.rs#L411)

#### Suzuki and Tada 1984

N. Suzuki, K. Tada, "Electrooptic Properties and Raman Scattering in InP", Jpn. J. Appl. Phys. 23, 291 (1984). [doi:10.1143/JJAP.23.291](https://doi.org/10.1143/JJAP.23.291)

- Methods pages: [catalogue.md:86](methods/catalogue.md?plain=1#L86)
- Validation cases: [`material/inp-pettit-turner-suzuki`](validation.md?plain=1#L86), [`material/inp-suzuki-tada-voltages`](validation.md?plain=1#L87), [`material/inp-suzuki-tada-faust-henry`](validation.md?plain=1#L88)
- Code: [src/material/catalogue/references.rs:354](../src/material/catalogue/references.rs#L354), [src/validation.rs:627](../src/validation.rs#L627), [634](../src/validation.rs#L634), [641](../src/validation.rs#L641)

#### Svanberg 1987

K. Svanberg, "The method of moving asymptotes—a new method for structural optimization", Numerical Meth Engineering 24, 359 (1987). [doi:10.1002/nme.1620240207](https://doi.org/10.1002/nme.1620240207)

- Methods pages: [optimization.md:133](methods/optimization.md?plain=1#L133)
- Code: [src/job/optimize.rs:176](../src/job/optimize.rs#L176)
- Other documents: [ROADMAP.md:773](../ROADMAP.md?plain=1#L773), [docs/plans/optimization.md:575](plans/optimization.md?plain=1#L575)

#### Svanberg 2002

K. Svanberg, "A Class of Globally Convergent Optimization Methods Based on Conservative Convex Separable Approximations", SIAM J. Optim. 12, 555 (2002). [doi:10.1137/S1052623499362822](https://doi.org/10.1137/S1052623499362822)

- Methods pages: [optimization.md:134](methods/optimization.md?plain=1#L134)
- Code: [src/job/optimize.rs:189](../src/job/optimize.rs#L189)
- Other documents: [ROADMAP.md:774](../ROADMAP.md?plain=1#L774), [docs/plans/optimization.md:576](plans/optimization.md?plain=1#L576)

### T

#### Taflove and Brodwin 1975

A. Taflove, M. Brodwin, "Numerical Solution of Steady-State Electromagnetic Scattering Problems Using the Time-Dependent Maxwell's Equations", IEEE Trans. Microwave Theory Techn. 23, 623 (1975). [doi:10.1109/TMTT.1975.1128640](https://doi.org/10.1109/TMTT.1975.1128640)

- Methods pages: [fdtd.md:14](methods/fdtd.md?plain=1#L14)
- Validation cases: [`fdtd/dispersion`](validation.md?plain=1#L203)
- Code: [src/fdtd.rs:15](../src/fdtd.rs#L15), [src/validation.rs:1446](../src/validation.rs#L1446)

#### Takagi et al. 1992

A. Takagi, K. Jinguji, M. Kawachi, "Wavelength characteristics of (2\*2) optical channel-type directional couplers with symmetric or nonsymmetric coupling structures", J. Lightwave Technol. 10, 735 (1992). [doi:10.1109/50.143072](https://doi.org/10.1109/50.143072)

- Academy lessons: [directional-coupler.md:58](../academy/directional-coupler.md?plain=1#L58), [332](../academy/directional-coupler.md?plain=1#L332)

#### Takato et al. 1990

N. Takato, T. Kominato, A. Sugita, K. Jinguji, H. Toba, M. Kawachi, "Silica-based integrated optic Mach-Zehnder multi/demultiplexer family with channel spacing of 0.01-250 nm", IEEE J. Select. Areas Commun. 8, 1120 (1990). [doi:10.1109/49.57816](https://doi.org/10.1109/49.57816)

- Academy lessons: [mach-zehnder.md:40](../academy/mach-zehnder.md?plain=1#L40), [362](../academy/mach-zehnder.md?plain=1#L362)

#### Tamir and Oliner 1963a

T. Tamir, A. Oliner, "Guided complex waves. Part 1: Fields at an interface", Proc. Inst. Electr. Eng. UK 110, 310 (1963). [doi:10.1049/piee.1963.0044](https://doi.org/10.1049/piee.1963.0044)

- Academy lessons: [leaky-and-bent-guides.md:35](../academy/leaky-and-bent-guides.md?plain=1#L35), [190](../academy/leaky-and-bent-guides.md?plain=1#L190), [548](../academy/leaky-and-bent-guides.md?plain=1#L548)

#### Tamir and Oliner 1963b

T. Tamir, A. Oliner, "Guided complex waves. Part 2: Relation to radiation patterns", Proc. Inst. Electr. Eng. UK 110, 325 (1963). [doi:10.1049/piee.1963.0045](https://doi.org/10.1049/piee.1963.0045)

- Academy lessons: [leaky-and-bent-guides.md:41](../academy/leaky-and-bent-guides.md?plain=1#L41), [205](../academy/leaky-and-bent-guides.md?plain=1#L205), [551](../academy/leaky-and-bent-guides.md?plain=1#L551)

#### Tan 1998

C. Tan, "Determination of refractive index of silica glass for infrared wavelengths by IR spectroscopy", Journal of Non-Crystalline Solids 223, 158 (1998). [doi:10.1016/S0022-3093(97)00438-9](https://doi.org/10.1016/S0022-3093(97)00438-9)

- Code: [src/material/refractiveindex.rs:335](../src/material/refractiveindex.rs#L335), [602](../src/material/refractiveindex.rs#L602), [603](../src/material/refractiveindex.rs#L603)

#### Tanaka et al. 1986

H. Tanaka, Y. Kawamura, H. Asahi, "Refractive indices of In0.49Ga0.51− x Al x P lattice matched to GaAs", Journal of Applied Physics 59, 985 (1986). [doi:10.1063/1.336581](https://doi.org/10.1063/1.336581)

- Methods pages: [catalogue.md:78](methods/catalogue.md?plain=1#L78)
- Code: [src/material/catalogue/models.rs:891](../src/material/catalogue/models.rs#L891), [src/material/catalogue/references.rs:319](../src/material/catalogue/references.rs#L319)

#### Tatian 1984

B. Tatian, "Fitting refractive-index data with the Sellmeier dispersion formula", Appl. Opt. 23, 4477 (1984). [doi:10.1364/AO.23.004477](https://doi.org/10.1364/AO.23.004477)

- Academy lessons: [material-dispersion.md:44](../academy/material-dispersion.md?plain=1#L44), [212](../academy/material-dispersion.md?plain=1#L212), [441](../academy/material-dispersion.md?plain=1#L441), [461](../academy/material-dispersion.md?plain=1#L461)

#### Taus et al. 2020

M. Taus, L. Zepeda-Núñez, R. J. Hewett, L. Demanet, "L-Sweeps: A scalable, parallel preconditioner for the high-frequency Helmholtz equation", Journal of Computational Physics 420, 109706 (2020). [doi:10.1016/j.jcp.2020.109706](https://doi.org/10.1016/j.jcp.2020.109706)

- Other documents: [docs/plans/performance.md:646](plans/performance.md?plain=1#L646)

#### Tian et al. 2026

X. Tian, Y. Lin, H. Gao, H. Yu, Y. Ma, R. Liang, C. Chen, W. Li, C. Deng, Q. Zheng, et al., "Wafer-scale hybrid molecular beam epitaxy of BaTiO3 and SrTiO3 on silicon", arXiv (2026). [doi:10.48550/arXiv.2604.23259](https://doi.org/10.48550/arXiv.2604.23259)

- Methods pages: [catalogue.md:58](methods/catalogue.md?plain=1#L58)
- Code: [src/material/catalogue/references.rs:480](../src/material/catalogue/references.rs#L480)

#### Tien 1971

P. K. Tien, "Light Waves in Thin Films and Integrated Optics", Appl. Opt. 10, 2395 (1971). [doi:10.1364/AO.10.002395](https://doi.org/10.1364/AO.10.002395)

- Academy lessons: [slab-waveguide.md:42](../academy/slab-waveguide.md?plain=1#L42), [567](../academy/slab-waveguide.md?plain=1#L567)

#### Tien and Ulrich 1970

P. K. Tien, R. Ulrich, "Theory of Prism–Film Coupler and Thin-Film Light Guides", J. Opt. Soc. Am. 60, 1325 (1970). [doi:10.1364/JOSA.60.001325](https://doi.org/10.1364/JOSA.60.001325)

- Academy lessons: [slab-waveguide.md:36](../academy/slab-waveguide.md?plain=1#L36), [176](../academy/slab-waveguide.md?plain=1#L176), [565](../academy/slab-waveguide.md?plain=1#L565)

#### Timurdogan et al. 2017

E. Timurdogan, C. V. Poulton, M. J. Byrd, M. R. Watts, "Electric field-induced second-order nonlinear optical effects in silicon waveguides", Nature Photon 11, 200 (2017). [doi:10.1038/nphoton.2017.14](https://doi.org/10.1038/nphoton.2017.14)

- Methods pages: [catalogue.md:126](methods/catalogue.md?plain=1#L126)
- Code: [src/material/catalogue/tags.rs:45](../src/material/catalogue/tags.rs#L45)

#### Tomm et al. 2021

N. Tomm, A. Javadi, N. O. Antoniadis, D. Najer, M. C. Löbl, A. R. Korsch, R. Schott, S. R. Valentin, A. D. Wieck, A. Ludwig, et al., "A bright and fast source of coherent single photons", Nat. Nanotechnol. 16, 399 (2021). [doi:10.1038/s41565-020-00831-x](https://doi.org/10.1038/s41565-020-00831-x)

- Other documents: [docs/plans/quantum.md:534](plans/quantum.md?plain=1#L534)

#### Triverio et al. 2009

P. Triverio, S. Grivet-Talocia, M. Nakhla, "A Parameterized Macromodeling Strategy With Uniform Stability Test", IEEE Trans. Adv. Packag. 32, 205 (2009). [doi:10.1109/TADVP.2008.2007913](https://doi.org/10.1109/TADVP.2008.2007913)

- Methods pages: [compact.md:12](methods/compact.md?plain=1#L12)
- Validation cases: [`device/compact-ring-closed-form`](validation.md?plain=1#L351), [`compact/param-ring-coupling`](validation.md?plain=1#L385), [`compact/param-ring-piecewise`](validation.md?plain=1#L390)
- Code: [src/cases/compact/checks.rs:768](../src/cases/compact/checks.rs#L768), [803](../src/cases/compact/checks.rs#L803), [src/compact/param.rs:5](../src/compact/param.rs#L5), [1364](../src/compact/param.rs#L1364), [src/device/compact.rs:706](../src/device/compact.rs#L706), [src/job/fit_checks.rs:429](../src/job/fit_checks.rs#L429)

#### Tsai et al. 1993

W. Tsai, M. Delfino, J. A. Fair, D. Hodul, "Temperature dependence of the electrical resistivity of reactively sputtered TiN films", Journal of Applied Physics 73, 4462 (1993). [doi:10.1063/1.352785](https://doi.org/10.1063/1.352785)

- Methods pages: [catalogue.md:100](methods/catalogue.md?plain=1#L100), [devices.md:881](methods/devices.md?plain=1#L881)
- Validation cases: [`material/tin-tsai-eq-3`](validation.md?plain=1#L40), [`device/thermo-tin-resistance`](validation.md?plain=1#L659)
- Code: [src/cases/device/thermo_checks.rs:531](../src/cases/device/thermo_checks.rs#L531), [src/material/catalogue/references.rs:424](../src/material/catalogue/references.rs#L424), [src/material/catalogue/resistivity.rs:7](../src/material/catalogue/resistivity.rs#L7), [135](../src/material/catalogue/resistivity.rs#L135), [src/validation.rs:305](../src/validation.rs#L305)

#### Tsuji et al. 2012

P. Tsuji, B. Engquist, L. Ying, "A sweeping preconditioner for time-harmonic Maxwell’s equations with finite elements", Journal of Computational Physics 231, 3770 (2012). [doi:10.1016/j.jcp.2012.01.025](https://doi.org/10.1016/j.jcp.2012.01.025)

- Other documents: [docs/plans/performance.md:647](plans/performance.md?plain=1#L647)

#### Turner et al. 2006

A. C. Turner, C. Manolatou, B. S. Schmidt, M. Lipson, M. A. Foster, J. E. Sharping, A. L. Gaeta, "Tailored anomalous group-velocity dispersion in silicon channel waveguides", Opt. Express 14, 4357 (2006). [doi:10.1364/OE.14.004357](https://doi.org/10.1364/OE.14.004357)

- Academy lessons: [strip-waveguide.md:106](../academy/strip-waveguide.md?plain=1#L106), [524](../academy/strip-waveguide.md?plain=1#L524), [689](../academy/strip-waveguide.md?plain=1#L689)

### U

#### Ueno et al. 1997

Y. Ueno, V. Ricci, G. I. Stegeman, "Second-order susceptibility of Ga\_05In\_05P crystals at 15 μm and their feasibility for waveguide quasi-phase matching", J. Opt. Soc. Am. B 14, 1428 (1997). [doi:10.1364/JOSAB.14.001428](https://doi.org/10.1364/JOSAB.14.001428)

- Methods pages: [catalogue.md:80](methods/catalogue.md?plain=1#L80)
- Validation cases: [`material/ingap-tanaka-ueno`](validation.md?plain=1#L85)
- Code: [src/material/catalogue/references.rs:333](../src/material/catalogue/references.rs#L333), [src/validation.rs:620](../src/validation.rs#L620)

#### Ulliac et al. 2011

G. Ulliac, B. Guichardaz, J.-Y. Rauch, S. Queste, S. Benchabane, N. Courjal, "Ultra-smooth LiNbO3 micro and nano structures for photonic applications", Microelectronic Engineering 88, 2417 (2011). [doi:10.1016/j.mee.2011.02.024](https://doi.org/10.1016/j.mee.2011.02.024)

- Code: [src/fabrication/references.rs:120](../src/fabrication/references.rs#L120)
- Other documents: [docs/plans/fabrication.md:646](plans/fabrication.md?plain=1#L646)

#### Ulsig et al. 2024a

E. Z. Ulsig, M. L. Madsen, E. J. Stanton, D. V. Reddy, A. Z. Leger, S. R. Sørensen, P. H. Godoy, I. Degli-Eredi, M. J. Stevens, D. R. Hamel, et al., "Efficient and widely tunable mid-infrared sources using GaAs and AlGaAs integrated platforms for second-order frequency conversion", Opt. Express 32, 36986 (2024). [doi:10.1364/OE.523615](https://doi.org/10.1364/OE.523615)

- Methods pages: [catalogue.md:120](methods/catalogue.md?plain=1#L120)
- Code: [src/material/catalogue/references.rs:312](../src/material/catalogue/references.rs#L312)

#### Ulsig et al. 2024b

E. Ulsig, M. Madsen, E. Stanton, D. Reddy, A. Leger, S. Sørensen, I. Degli-Eredi, M. Stevens, D. Hamel, L. Shalm, et al., "Widely tunable integrated GaAs and AlGaAs platforms for efficient second-order frequency conversion", Optica Publishing Group (2024). [doi:10.1364/opticaopen.25540030.v1](https://doi.org/10.1364/opticaopen.25540030.v1), open access.

- Code: [src/material/catalogue/references.rs:313](../src/material/catalogue/references.rs#L313)

#### Umashankar and Taflove 1982

K. Umashankar, A. Taflove, "A Novel Method to Analyze Electromagnetic Scattering of Complex Objects", IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982). [doi:10.1109/TEMC.1982.304054](https://doi.org/10.1109/TEMC.1982.304054)

- Methods pages: [fdtd.md:16](methods/fdtd.md?plain=1#L16)
- Validation cases: [`fdtd/tfsf-leakage`](validation.md?plain=1#L210), [`fdtd/mode-source-fdfd`](validation.md?plain=1#L212)
- Examples: [README.md:64](../examples/README.md?plain=1#L64), [tfsf_square_cylinder.rs:6](../examples/tfsf_square_cylinder.rs#L6)
- Code: [src/fdtd.rs:45](../src/fdtd.rs#L45), [src/fdtd/sources.rs:11](../src/fdtd/sources.rs#L11), [src/validation.rs:1495](../src/validation.rs#L1495), [1509](../src/validation.rs#L1509)

### V

#### Vahala 2003

K. J. Vahala, "Optical microcavities", Nature 424, 839 (2003). [doi:10.1038/nature01939](https://doi.org/10.1038/nature01939)

- Academy lessons: [ring-resonator.md:61](../academy/ring-resonator.md?plain=1#L61), [401](../academy/ring-resonator.md?plain=1#L401)

#### Vaidya et al. 2020

V. D. Vaidya, B. Morrison, L. G. Helt, R. Shahrokshahi, D. H. Mahler, M. J. Collins, K. Tan, J. Lavoie, A. Repingon, M. Menotti, et al., "Broadband quadrature-squeezed vacuum and nonclassical photon number correlations from a nanophotonic device", Sci. Adv. 6, eaba9186 (2020). [doi:10.1126/sciadv.aba9186](https://doi.org/10.1126/sciadv.aba9186), open access.

- Other documents: [docs/plans/quantum.md:532](plans/quantum.md?plain=1#L532)

#### Valdez et al. 2023

F. Valdez, V. Mere, S. Mookherjea, "100 GHz bandwidth, 1 volt integrated electro-optic Mach–Zehnder modulator at near-IR wavelengths", Optica 10, 578 (2023). [doi:10.1364/OPTICA.484549](https://doi.org/10.1364/OPTICA.484549)

- Validation cases: [`pockels/valdez-2023`](validation.md?plain=1#L554)
- Code: [src/cases/mode/pockels/checks.rs:1040](../src/cases/mode/pockels/checks.rs#L1040)
- Other documents: [ROADMAP.md:785](../ROADMAP.md?plain=1#L785)

#### van der Vorst and Melissen 1990

H. van der Vorst, J. Melissen, "A Petrov-Galerkin type method for solving Axk=b, where A is symmetric complex", IEEE Trans. Magn. 26, 706 (1990). [doi:10.1109/20.106415](https://doi.org/10.1109/20.106415)

- Methods pages: [fdfd-3d.md:22](methods/fdfd-3d.md?plain=1#L22), [445](methods/fdfd-3d.md?plain=1#L445)
- Code: [src/cases/fdfd/three/tests.rs:1404](../src/cases/fdfd/three/tests.rs#L1404)
- Other documents: [docs/plans/performance.md:648](plans/performance.md?plain=1#L648)

#### Van Gijzen and Sonneveld 2011

M. B. Van Gijzen, P. Sonneveld, "Algorithm 913: An elegant IDR( s ) variant that efficiently exploits biorthogonality properties", ACM Trans. Math. Softw. 38, 1 (2011). [doi:10.1145/2049662.2049667](https://doi.org/10.1145/2049662.2049667)

- Other documents: [docs/plans/performance.md:649](plans/performance.md?plain=1#L649)

#### Varnava et al. 2008

M. Varnava, D. E. Browne, T. Rudolph, "How Good Must Single Photon Sources and Detectors Be for Efficient Linear Optical Quantum Computation?", Phys. Rev. Lett. 100, 060502 (2008). [doi:10.1103/PhysRevLett.100.060502](https://doi.org/10.1103/PhysRevLett.100.060502)

- Other documents: [docs/plans/quantum.md:489](plans/quantum.md?plain=1#L489)

#### Vassilevski 2018

Tzanio V. Kolev  & Panayot S. Vassilevski, "Parallel Auxiliary Space AMG for \$\\boldsymbol{H}\$(Curl) Problems", JCM 27, 604 (2018). [doi:10.4208/jcm.2009.27.5.013](https://doi.org/10.4208/jcm.2009.27.5.013)

- Other documents: [docs/plans/performance.md:629](plans/performance.md?plain=1#L629)

#### Vatti 1992

B. R. Vatti, "A generic solution to polygon clipping", Commun. ACM 35, 56 (1992). [doi:10.1145/129902.129906](https://doi.org/10.1145/129902.129906)

- Methods pages: [booleans.md:200](methods/booleans.md?plain=1#L200)

#### Vercruysse et al. 2019

D. Vercruysse, N. V. Sapra, L. Su, R. Trivedi, J. Vučković, "Analytical level set fabrication constraints for inverse design", Sci Rep 9, 8999 (2019). [doi:10.1038/s41598-019-45026-0](https://doi.org/10.1038/s41598-019-45026-0), open access.

- Other documents: [ROADMAP.md:770](../ROADMAP.md?plain=1#L770)

#### Vermeulen et al. 2010

D. Vermeulen, S. Selvaraja, P. Verheyen, G. Lepage, W. Bogaerts, P. Absil, D. Van Thourhout, G. Roelkens, "High-efficiency fiber-to-chip grating couplers realized using an advanced CMOS-compatible Silicon-On-Insulator platform", Opt. Express 18, 18278 (2010). [doi:10.1364/OE.18.018278](https://doi.org/10.1364/OE.18.018278)

- Other documents: [ROADMAP.md:753](../ROADMAP.md?plain=1#L753)

#### Vernon et al. 2017

Z. Vernon, M. Menotti, C. C. Tison, J. A. Steidle, M. L. Fanto, P. M. Thomas, S. F. Preble, A. M. Smith, P. M. Alsing, M. Liscidini, et al., "Truly unentangled photon pairs without spectral filtering", Opt. Lett. 42, 3638 (2017). [doi:10.1364/OL.42.003638](https://doi.org/10.1364/OL.42.003638)

- Other documents: [docs/plans/quantum.md:531](plans/quantum.md?plain=1#L531)

#### Veronis et al. 2004

G. Veronis, R. W. Dutton, S. Fan, "Method for sensitivity analysis of photonic crystal devices", Opt. Lett. 29, 2288 (2004). [doi:10.1364/OL.29.002288](https://doi.org/10.1364/OL.29.002288)

- Methods pages: [averaging.md:14](methods/averaging.md?plain=1#L14), [circuit-adjoint.md:8](methods/circuit-adjoint.md?plain=1#L8), [fdfd-adjoint.md:8](methods/fdfd-adjoint.md?plain=1#L8), [fdtd-adjoint.md:8](methods/fdtd-adjoint.md?plain=1#L8)
- Validation cases: [`fdfd/adjoint-gradient-ez`](validation.md?plain=1#L147), [`fdfd3d/adjoint-gradient`](validation.md?plain=1#L168), [`fdfd3d/recycle-adjoint`](validation.md?plain=1#L191), [`fdtd/adjoint-modes`](validation.md?plain=1#L268), [`circuit/adjoint-mzi`](validation.md?plain=1#L283), [`chip/adjoint`](validation.md?plain=1#L373), [`fdfd/fill-derivative-ez`](validation.md?plain=1#L503)
- Code: [src/cases/fdfd/fill_checks.rs:670](../src/cases/fdfd/fill_checks.rs#L670), [src/circuit/adjoint.rs:17](../src/circuit/adjoint.rs#L17), [src/fdfd/adjoint.rs:5](../src/fdfd/adjoint.rs#L5), [src/fdfd/three/adjoint.rs:5](../src/fdfd/three/adjoint.rs#L5), [src/fdtd/adjoint.rs:26](../src/fdtd/adjoint.rs#L26), [src/job/chip_checks.rs:37](../src/job/chip_checks.rs#L37), [src/validation.rs:1054](../src/validation.rs#L1054), [1201](../src/validation.rs#L1201), [1362](../src/validation.rs#L1362), [1901](../src/validation.rs#L1901), [2006](../src/validation.rs#L2006)
- Other documents: [docs/plans/optimization.md:588](plans/optimization.md?plain=1#L588)

#### Vlasov and McNab 2004

Y. A. Vlasov, S. J. McNab, "Losses in single-mode silicon-on-insulator strip waveguides and bends", Opt. Express 12, 1622 (2004). [doi:10.1364/OPEX.12.001622](https://doi.org/10.1364/OPEX.12.001622)

- Academy lessons: [leaky-and-bent-guides.md:95](../academy/leaky-and-bent-guides.md?plain=1#L95), [536](../academy/leaky-and-bent-guides.md?plain=1#L536), [574](../academy/leaky-and-bent-guides.md?plain=1#L574), [strip-waveguide.md:88](../academy/strip-waveguide.md?plain=1#L88), [619](../academy/strip-waveguide.md?plain=1#L619), [684](../academy/strip-waveguide.md?plain=1#L684)

#### Vogelbacher et al. 2019

F. Vogelbacher, S. Nevlacsil, M. Sagmeister, J. Kraft, K. Unterrainer, R. Hainberger, "Analysis of silicon nitride partial Euler waveguide bends", Opt. Express 27, 31394 (2019). [doi:10.1364/OE.27.031394](https://doi.org/10.1364/OE.27.031394)

- Methods pages: [generators.md:8](methods/generators.md?plain=1#L8), [geometry.md:20](methods/geometry.md?plain=1#L20)
- Validation cases: [`geometry/paths/euler-ends`](validation.md?plain=1#L398), [`geometry/paths/vogelbacher-singular`](validation.md?plain=1#L407), [`generator/lengths-and-areas`](validation.md?plain=1#L453)
- Code: [src/generator/checks.rs:960](../src/generator/checks.rs#L960), [src/geometry/path_checks.rs:376](../src/geometry/path_checks.rs#L376), [449](../src/geometry/path_checks.rs#L449)

#### Vurgaftman et al. 2001

I. Vurgaftman, J. R. Meyer, L. R. Ram-Mohan, "Band parameters for III–V compound semiconductors and their alloys", Journal of Applied Physics 89, 5815 (2001). [doi:10.1063/1.1368156](https://doi.org/10.1063/1.1368156)

- Methods pages: [catalogue.md:38](methods/catalogue.md?plain=1#L38)
- Validation cases: [`material/transparency-gaas-gap`](validation.md?plain=1#L739)
- Code: [src/material/catalogue/references.rs:613](../src/material/catalogue/references.rs#L613), [src/material/catalogue/transparency.rs:6](../src/material/catalogue/transparency.rs#L6), [src/nonlinear/data_checks.rs:361](../src/nonlinear/data_checks.rs#L361)
- Other documents: [ROADMAP.md:850](../ROADMAP.md?plain=1#L850)

### W

#### Wang et al. 2010

F. Wang, B. S. Lazarov, O. Sigmund, "On projection methods, convergence and robust formulations in topology optimization", Struct Multidisc Optim 43, 767 (2010). [doi:10.1007/s00158-010-0602-y](https://doi.org/10.1007/s00158-010-0602-y)

- Methods pages: [optimization.md:165](methods/optimization.md?plain=1#L165)
- Other documents: [ROADMAP.md:761](../ROADMAP.md?plain=1#L761), [docs/plans/optimization.md:591](plans/optimization.md?plain=1#L591)

#### Wang et al. 2011

X. Wang, W. Shi, R. Vafaei, N. A. F. Jaeger, L. Chrostowski, "Uniform and Sampled Bragg Gratings in SOI Strip Waveguides with Sidewall Corrugations", IEEE Photon. Technol. Lett. (2011). [doi:10.1109/LPT.2010.2103305](https://doi.org/10.1109/LPT.2010.2103305)

- Methods pages: [generators.md:14](methods/generators.md?plain=1#L14), [125](methods/generators.md?plain=1#L125)
- Academy lessons: [bragg-gratings.md:91](../academy/bragg-gratings.md?plain=1#L91), [302](../academy/bragg-gratings.md?plain=1#L302)
- Validation cases: [`generator/wang-bragg`](validation.md?plain=1#L458)
- Code: [src/generator/checks.rs:1003](../src/generator/checks.rs#L1003)

#### Wang et al. 2012

X. Wang, W. Shi, H. Yun, S. Grist, N. A. F. Jaeger, L. Chrostowski, "Narrow-band waveguide Bragg gratings on SOI wafers with CMOS-compatible fabrication process", Opt. Express 20, 15547 (2012). [doi:10.1364/OE.20.015547](https://doi.org/10.1364/OE.20.015547)

- Academy lessons: [bragg-gratings.md:97](../academy/bragg-gratings.md?plain=1#L97), [306](../academy/bragg-gratings.md?plain=1#L306)

#### Wang et al. 2018a

C. Wang, M. Zhang, X. Chen, M. Bertrand, A. Shams-Ansari, S. Chandrasekhar, P. Winzer, M. Lončar, "Integrated lithium niobate electro-optic modulators operating at CMOS-compatible voltages", Nature 562, 101 (2018). [doi:10.1038/s41586-018-0551-y](https://doi.org/10.1038/s41586-018-0551-y)

- Methods pages: [electro-optic.md:180](methods/electro-optic.md?plain=1#L180)
- Academy lessons: [mach-zehnder.md:112](../academy/mach-zehnder.md?plain=1#L112), [293](../academy/mach-zehnder.md?plain=1#L293), [375](../academy/mach-zehnder.md?plain=1#L375), [459](../academy/mach-zehnder.md?plain=1#L459)
- Other documents: [docs/plans/fabrication.md:649](plans/fabrication.md?plain=1#L649)

#### Wang et al. 2018b

C. Wang, M. Zhang, B. Stern, M. Lipson, M. Lončar, "Nanophotonic lithium niobate electro-optic modulators", Opt. Express 26, 1547 (2018). [doi:10.1364/OE.26.001547](https://doi.org/10.1364/OE.26.001547)

- Methods pages: [devices.md:52](methods/devices.md?plain=1#L52), [electro-optic.md:12](methods/electro-optic.md?plain=1#L12)
- Validation cases: [`pockels/overlap-gamma`](validation.md?plain=1#L551), [`pockels/wang-2018`](validation.md?plain=1#L552), [`device/tfln-wang-vpil`](validation.md?plain=1#L689), [`device/tfln-measured`](validation.md?plain=1#L690)
- Code: [src/cases/device/tfln_checks.rs:333](../src/cases/device/tfln_checks.rs#L333), [340](../src/cases/device/tfln_checks.rs#L340), [src/cases/mode/pockels/checks.rs:1019](../src/cases/mode/pockels/checks.rs#L1019), [1026](../src/cases/mode/pockels/checks.rs#L1026), [src/device/measured/wang-2018.toml:5](../src/device/measured/wang-2018.toml#L5), [src/device/tfln.rs:1253](../src/device/tfln.rs#L1253), [src/mode/pockels.rs:289](../src/mode/pockels.rs#L289)
- Other documents: [ROADMAP.md:781](../ROADMAP.md?plain=1#L781)

#### Wang et al. 2018c

C. Wang, C. Langrock, A. Marandi, M. Jankowski, M. Zhang, B. Desiatov, M. M. Fejer, M. Lončar, "Ultrahigh-efficiency wavelength conversion in nanophotonic periodically poled lithium niobate waveguides", Optica 5, 1438 (2018). [doi:10.1364/OPTICA.5.001438](https://doi.org/10.1364/OPTICA.5.001438)

- Methods pages: [nonlinear.md:28](methods/nonlinear.md?plain=1#L28)
- Validation cases: [`nonlinear/wang-2018-duty-cycle`](validation.md?plain=1#L719)
- Examples: [README.md:85](../examples/README.md?plain=1#L85), [ppln_wang.rs:6](../examples/ppln_wang.rs#L6)
- Code: [src/nonlinear/checks.rs:725](../src/nonlinear/checks.rs#L725)
- Other documents: [ROADMAP.md:859](../ROADMAP.md?plain=1#L859)

#### Wang et al. 2019

J. Wang, F. Sciarrino, A. Laing, M. G. Thompson, "Integrated photonic quantum technologies", Nat. Photonics 14, 273 (2019). [doi:10.1038/s41566-019-0532-1](https://doi.org/10.1038/s41566-019-0532-1)

- Other documents: [docs/plans/quantum.md:523](plans/quantum.md?plain=1#L523)

#### Wang et al. 2022

M. Wang, X. Chen, U. Khan, W. Bogaerts, "Programmable wavelength filter with double ring loaded MZI", Sci Rep 12, 1482 (2022). [doi:10.1038/s41598-021-04598-6](https://doi.org/10.1038/s41598-021-04598-6), open access.

- Examples: [README.md:53](../examples/README.md?plain=1#L53), [optimize_ring_mzi.rs:7](../examples/optimize_ring_mzi.rs#L7)
- Other documents: [docs/plans/optimization.md:623](plans/optimization.md?plain=1#L623)

#### Wang et al. 2024

C. Wang, Z. Li, J. Riemensberger, G. Lihachev, M. Churaev, W. Kao, X. Ji, J. Zhang, T. Blesin, A. Davydova, et al., "Lithium tantalate photonic integrated circuits for volume manufacturing", Nature 629, 784 (2024). [doi:10.1038/s41586-024-07369-1](https://doi.org/10.1038/s41586-024-07369-1), open access.

- Methods pages: [electro-optic.md:385](methods/electro-optic.md?plain=1#L385)
- Other documents: [ROADMAP.md:791](../ROADMAP.md?plain=1#L791)

#### Waqas et al. 2018

A. Waqas, D. Melati, P. Manfredi, A. Melloni, "Stochastic process design kits for photonic circuits based on polynomial chaos augmented macro-modelling", Opt. Express 26, 5894 (2018). [doi:10.1364/OE.26.005894](https://doi.org/10.1364/OE.26.005894)

- Other documents: [ROADMAP.md:827](../ROADMAP.md?plain=1#L827)

#### Weedbrook et al. 2012

C. Weedbrook, S. Pirandola, R. García-Patrón, N. J. Cerf, T. C. Ralph, J. H. Shapiro, S. Lloyd, "Gaussian quantum information", Rev. Mod. Phys. 84, 621 (2012). [doi:10.1103/RevModPhys.84.621](https://doi.org/10.1103/RevModPhys.84.621)

- Other documents: [docs/plans/quantum.md:514](plans/quantum.md?plain=1#L514)

#### Wen 1969

C. Wen, "Coplanar Waveguide: A Surface Strip Transmission Line Suitable for Nonreciprocal Gyromagnetic Device Applications", IEEE Trans. Microwave Theory Techn. 17, 1087 (1969). [doi:10.1109/TMTT.1969.1127105](https://doi.org/10.1109/TMTT.1969.1127105)

- Methods pages: [static.md:16](methods/static.md?plain=1#L16)
- Validation cases: [`static/coplanar-wen`](validation.md?plain=1#L522)
- Code: [src/cases/statics/checks.rs:938](../src/cases/statics/checks.rs#L938)

#### Weng et al. 2015

T.-W. Weng, Z. Zhang, Z. Su, Y. Marzouk, A. Melloni, L. Daniel, "Uncertainty quantification of silicon photonic devices with correlated and non-Gaussian random parameters", Opt. Express 23, 4242 (2015). [doi:10.1364/OE.23.004242](https://doi.org/10.1364/OE.23.004242)

- Other documents: [ROADMAP.md:826](../ROADMAP.md?plain=1#L826)

#### Wengert 1964

R. E. Wengert, "A simple automatic derivative evaluation program", Commun. ACM 7, 463 (1964). [doi:10.1145/355586.364791](https://doi.org/10.1145/355586.364791)

- Methods pages: [devices.md:28](methods/devices.md?plain=1#L28), [expressions.md:8](methods/expressions.md?plain=1#L8)
- Validation cases: [`device/derivatives`](validation.md?plain=1#L312)
- Code: [src/device/checks.rs:82](../src/device/checks.rs#L82), [src/expr.rs:43](../src/expr.rs#L43)

#### Werner and Cary 2007

G. R. Werner, J. R. Cary, "A stable FDTD algorithm for non-diagonal, anisotropic dielectrics", Journal of Computational Physics 226, 1085 (2007). [doi:10.1016/j.jcp.2007.05.008](https://doi.org/10.1016/j.jcp.2007.05.008)

- Methods pages: [fdtd.md:32](methods/fdtd.md?plain=1#L32)
- Validation cases: [`fdtd/smoothing-oblique-nodes`](validation.md?plain=1#L230)
- Code: [src/fdtd/smoothing.rs:20](../src/fdtd/smoothing.rs#L20), [src/validation.rs:1635](../src/validation.rs#L1635)

#### Werner et al. 2013

G. R. Werner, C. A. Bauer, J. R. Cary, "A more accurate, stable, FDTD algorithm for electromagnetics in anisotropic dielectrics", Journal of Computational Physics 255, 436 (2013). [doi:10.1016/j.jcp.2013.08.009](https://doi.org/10.1016/j.jcp.2013.08.009)

- Methods pages: [averaging.md:12](methods/averaging.md?plain=1#L12), [fdtd.md:46](methods/fdtd.md?plain=1#L46)
- Validation cases: [`fdtd/smoothing-triplets-oblique`](validation.md?plain=1#L234), [`fdtd/smoothing-wc07-growth`](validation.md?plain=1#L241), [`fdtd/fdfd-smoothed`](validation.md?plain=1#L267)
- Code: [src/fdfd/fill.rs:27](../src/fdfd/fill.rs#L27), [src/fdtd/smoothing.rs:26](../src/fdtd/smoothing.rs#L26), [258](../src/fdtd/smoothing.rs#L258), [src/validation.rs:1663](../src/validation.rs#L1663), [1712](../src/validation.rs#L1712), [1894](../src/validation.rs#L1894)

#### Wheeler 1942

H. Wheeler, "Formulas for the Skin Effect", Proc. IRE 30, 412 (1942). [doi:10.1109/JRPROC.1942.232015](https://doi.org/10.1109/JRPROC.1942.232015)

- Methods pages: [travelling-wave.md:10](methods/travelling-wave.md?plain=1#L10)
- Validation cases: [`line/wheeler-plates`](validation.md?plain=1#L566)
- Code: [src/cases/electro/line_checks.rs:958](../src/cases/electro/line_checks.rs#L958), [src/electro/line.rs:29](../src/electro/line.rs#L29), [631](../src/electro/line.rs#L631)
- Other documents: [ROADMAP.md:796](../ROADMAP.md?plain=1#L796)

#### Wiecha et al. 2021

P. R. Wiecha, A. Arbouet, C. Girard, O. L. Muskens, "Deep learning in nano-photonics: inverse design and beyond", Photon. Res. 9, B182 (2021). [doi:10.1364/PRJ.415960](https://doi.org/10.1364/PRJ.415960)

- Other documents: [docs/plans/optimization.md:636](plans/optimization.md?plain=1#L636)

#### Williams et al. 2009

S. Williams, A. Waterman, D. Patterson, "Roofline: an insightful visual performance model for multicore architectures", Commun. ACM 52, 65 (2009). [doi:10.1145/1498765.1498785](https://doi.org/10.1145/1498765.1498785)

- Code: [src/bench.rs:10](../src/bench.rs#L10), [src/traffic.rs:3](../src/traffic.rs#L3)
- Other documents: [docs/plans/performance.md:650](plans/performance.md?plain=1#L650)

#### Wooten et al. 2000

E. Wooten, K. Kissa, A. Yi-Yan, E. Murphy, D. Lafaw, P. Hallemeier, D. Maack, D. Attanasio, D. Fritz, G. McBrien, et al., "A review of lithium niobate modulators for fiber-optic communications systems", IEEE J. Select. Topics Quantum Electron. 6, 69 (2000). [doi:10.1109/2944.826874](https://doi.org/10.1109/2944.826874)

- Academy lessons: [mach-zehnder.md:70](../academy/mach-zehnder.md?plain=1#L70), [360](../academy/mach-zehnder.md?plain=1#L360), [457](../academy/mach-zehnder.md?plain=1#L457)

### X

#### Xiong et al. 2012

C. Xiong, W. H. P. Pernice, H. X. Tang, "Low-Loss, Silicon Integrated, Aluminum Nitride Photonic Circuits and Their Use for Electro-Optic Signal Processing", Nano Lett. 12, 3562 (2012). [doi:10.1021/nl3011885](https://doi.org/10.1021/nl3011885)

- Methods pages: [fabrication.md:48](methods/fabrication.md?plain=1#L48)
- Validation cases: [`fabrication/aln-reported-flows`](validation.md?plain=1#L762)
- Code: [src/fabrication/platform_checks.rs:763](../src/fabrication/platform_checks.rs#L763), [src/fabrication/references.rs:200](../src/fabrication/references.rs#L200)
- Other documents: [docs/plans/fabrication.md:661](plans/fabrication.md?plain=1#L661)

#### Xu et al. 2005

Q. Xu, B. Schmidt, S. Pradhan, M. Lipson, "Micrometre-scale silicon electro-optic modulator", Nature 435, 325 (2005). [doi:10.1038/nature03569](https://doi.org/10.1038/nature03569)

- Academy lessons: [ring-resonator.md:73](../academy/ring-resonator.md?plain=1#L73)

### Y

#### Yablonovitch 1987

E. Yablonovitch, "Inhibited Spontaneous Emission in Solid-State Physics and Electronics", Phys. Rev. Lett. 58, 2059 (1987). [doi:10.1103/PhysRevLett.58.2059](https://doi.org/10.1103/PhysRevLett.58.2059)

- Academy lessons: [bragg-gratings.md:67](../academy/bragg-gratings.md?plain=1#L67), [409](../academy/bragg-gratings.md?plain=1#L409)

#### Yariv 1973

A. Yariv, "Coupled-mode theory for guided-wave optics", IEEE J. Quantum Electron. 9, 919 (1973). [doi:10.1109/JQE.1973.1077767](https://doi.org/10.1109/JQE.1973.1077767)

- Academy lessons: [bragg-gratings.md:43](../academy/bragg-gratings.md?plain=1#L43), [239](../academy/bragg-gratings.md?plain=1#L239), [directional-coupler.md:40](../academy/directional-coupler.md?plain=1#L40), [155](../academy/directional-coupler.md?plain=1#L155), [514](../academy/directional-coupler.md?plain=1#L514)

#### Yariv 2000

A. Yariv, "Universal relations for coupling of optical powerbetweenmicroresonators and dielectric waveguides", Electron. Lett. 36, 321 (2000). [doi:10.1049/el:20000340](https://doi.org/10.1049/el:20000340)

- Academy lessons: [ring-resonator.md:49](../academy/ring-resonator.md?plain=1#L49), [237](../academy/ring-resonator.md?plain=1#L237), [281](../academy/ring-resonator.md?plain=1#L281)

#### Yariv and Yeh 2007

A. Yariv, P. Yeh, "Photonics: Optical Electronics in Modern Communications, 6th ed.", Oxford University Press (2007)., open access.

- Methods pages: [slab.md:9](methods/slab.md?plain=1#L9)

#### Ye et al. 2018

Y. Ye, D. Spina, Y. Xing, W. Bogaerts, T. Dhaene, "Numerical modeling of a linear photonic system for accurate and efficient time-domain simulations", Photon. Res. 6, 560 (2018). [doi:10.1364/PRJ.6.000560](https://doi.org/10.1364/PRJ.6.000560)

- Methods pages: [compact.md:157](methods/compact.md?plain=1#L157)
- Examples: [README.md:79](../examples/README.md?plain=1#L79), [compact_mzi_ye.rs:7](../examples/compact_mzi_ye.rs#L7)
- Other documents: [ROADMAP.md:816](../ROADMAP.md?plain=1#L816)

#### Ye et al. 2022

Y. Ye, T. Ullrick, W. Bogaerts, T. Dhaene, D. Spina, "SPICE-Compatible Equivalent Circuit Models for Accurate Time-Domain Simulations of Passive Photonic Integrated Circuits", J. Lightwave Technol. 40, 7856 (2022). [doi:10.1109/JLT.2022.3206818](https://doi.org/10.1109/JLT.2022.3206818)

- Other documents: [ROADMAP.md:817](../ROADMAP.md?plain=1#L817)

#### Yeh et al. 1977

P. Yeh, A. Yariv, C.-S. Hong, "Electromagnetic propagation in periodic stratified media I General theory\*", J. Opt. Soc. Am. 67, 423 (1977). [doi:10.1364/JOSA.67.000423](https://doi.org/10.1364/JOSA.67.000423)

- Methods pages: [travelling-wave.md:32](methods/travelling-wave.md?plain=1#L32)
- Academy lessons: [bragg-gratings.md:49](../academy/bragg-gratings.md?plain=1#L49), [212](../academy/bragg-gratings.md?plain=1#L212), [403](../academy/bragg-gratings.md?plain=1#L403)
- Validation cases: [`line/periodic-uniform`](validation.md?plain=1#L580)
- Code: [src/cases/electro/line_checks.rs:1056](../src/cases/electro/line_checks.rs#L1056), [src/electro/line.rs:43](../src/electro/line.rs#L43), [755](../src/electro/line.rs#L755)

#### Yonekura et al. 2007

K. Yonekura, L. Jin, K. Takizawa, "Measurement of Wavelength Dependence of Electro-Optic Coefficients r 22 of Non-doped and 5% MgO-doped Congruent LiNbO3 Crystals and 1.8% MgO-doped Quasi-stoichiometric LiNbO3 Crystal by Multiple Reflection Interference Method", OPT REV 14, 194 (2007). [doi:10.1007/s10043-007-0194-y](https://doi.org/10.1007/s10043-007-0194-y)

- Code: [src/material/catalogue/references.rs:172](../src/material/catalogue/references.rs#L172)

#### Yu et al. 2012

H. Yu, M. Pantouvaki, J. Van Campenhout, D. Korn, K. Komorowska, P. Dumon, Y. Li, P. Verheyen, P. Absil, L. Alloatti, et al., "Performance tradeoff between lateral and interdigitated doping patterns for high speed carrier-depletion based silicon modulators", Opt. Express 20, 12926 (2012). [doi:10.1364/OE.20.012926](https://doi.org/10.1364/OE.20.012926)

- Other documents: [ROADMAP.md:812](../ROADMAP.md?plain=1#L812)

### Z

#### Zelmon et al. 1997

D. E. Zelmon, D. L. Small, D. Jundt, "Infrared corrected Sellmeier coefficients for congruently grown lithium niobate and 5 mol% magnesium oxide –doped lithium niobate", J. Opt. Soc. Am. B 14, 3319 (1997). [doi:10.1364/JOSAB.14.003319](https://doi.org/10.1364/JOSAB.14.003319)

- Methods pages: [catalogue.md:44](methods/catalogue.md?plain=1#L44)
- Validation cases: [`material/linbo3-zelmon-opo`](validation.md?plain=1#L65)
- Code: [src/material/catalogue/models.rs:345](../src/material/catalogue/models.rs#L345), [src/material/catalogue/references.rs:144](../src/material/catalogue/references.rs#L144), [src/validation.rs:480](../src/validation.rs#L480)
- Other documents: [ROADMAP.md:787](../ROADMAP.md?plain=1#L787)

#### Zhang et al. 2013

Y. Zhang, S. Yang, A. E.-J. Lim, G.-Q. Lo, C. Galland, T. Baehr-Jones, M. Hochberg, "A compact and low loss Y-junction for submicron silicon waveguide", Opt. Express 21, 1310 (2013). [doi:10.1364/OE.21.001310](https://doi.org/10.1364/OE.21.001310)

- Methods pages: [booleans.md:14](methods/booleans.md?plain=1#L14), [devices.md:32](methods/devices.md?plain=1#L32), [geometry.md:27](methods/geometry.md?plain=1#L27)
- Academy lessons: [directional-coupler.md:76](../academy/directional-coupler.md?plain=1#L76), [454](../academy/directional-coupler.md?plain=1#L454)
- Validation cases: [`device/y-branch-ideal-2d`](validation.md?plain=1#L345), [`geometry/curves/zhang-y-junction`](validation.md?plain=1#L420), [`geometry/booleans/zhang-y-junction`](validation.md?plain=1#L428)
- Examples: [README.md:58](../examples/README.md?plain=1#L58), [78](../examples/README.md?plain=1#L78), [optimize_ybranch.rs:6](../examples/optimize_ybranch.rs#L6), [y_junction_zhang.rs:6](../examples/y_junction_zhang.rs#L6)
- Code: [src/device/splitter_checks.rs:173](../src/device/splitter_checks.rs#L173), [src/device/ybranch.rs:20](../src/device/ybranch.rs#L20), [src/geometry/boolean_checks.rs:819](../src/geometry/boolean_checks.rs#L819), [src/geometry/curve_checks.rs:749](../src/geometry/curve_checks.rs#L749)
- Other documents: [ROADMAP.md:752](../ROADMAP.md?plain=1#L752), [docs/plans/optimization.md:607](plans/optimization.md?plain=1#L607)

#### Zhang et al. 2017

M. Zhang, C. Wang, R. Cheng, A. Shams-Ansari, M. Lončar, "Monolithic ultra-high-Q lithium niobate microring resonator", Optica 4, 1536 (2017). [doi:10.1364/OPTICA.4.001536](https://doi.org/10.1364/OPTICA.4.001536)

- Methods pages: [fabrication.md:24](methods/fabrication.md?plain=1#L24)
- Validation cases: [`fabrication/reported-flows`](validation.md?plain=1#L748)
- Code: [src/fabrication/checks.rs:461](../src/fabrication/checks.rs#L461), [src/fabrication/references.rs:128](../src/fabrication/references.rs#L128), [130](../src/fabrication/references.rs#L130)
- Other documents: [docs/plans/fabrication.md:647](plans/fabrication.md?plain=1#L647)

#### Zhang et al. 2022

Y. Zhang, L. Shao, J. Yang, Z. Chen, K. Zhang, K.-M. Shum, D. Zhu, C. H. Chan, M. Lončar, C. Wang, "Systematic investigation of millimeter-wave optic modulation performance in thin-film lithium niobate", Photon. Res. 10, 2380 (2022). [doi:10.1364/PRJ.468518](https://doi.org/10.1364/PRJ.468518)

- Methods pages: [devices.md:50](methods/devices.md?plain=1#L50), [electro-optic.md:14](methods/electro-optic.md?plain=1#L14), [fabrication.md:22](methods/fabrication.md?plain=1#L22), [travelling-wave.md:16](methods/travelling-wave.md?plain=1#L16)
- Validation cases: [`pockels/zhang-2022`](validation.md?plain=1#L553), [`line/response-loss-3db`](validation.md?plain=1#L570), [`line/zhang-2022-index`](validation.md?plain=1#L575), [`line/zhang-2022-full-wave-index`](validation.md?plain=1#L600), [`device/tfln-zhang-vpil`](validation.md?plain=1#L688), [`device/tfln-measured`](validation.md?plain=1#L690), [`device/tfln-zhang-response`](validation.md?plain=1#L691), [`fabrication/reported-flows`](validation.md?plain=1#L748)
- Examples: [README.md:83](../examples/README.md?plain=1#L83), [tfln_zhang.rs:7](../examples/tfln_zhang.rs#L7)
- Code: [crosscode/src/tfln.rs:2](../crosscode/src/tfln.rs#L2), [crosscode/src/zhang_line.rs:2](../crosscode/src/zhang_line.rs#L2), [src/cases/device/tfln_checks.rs:326](../src/cases/device/tfln_checks.rs#L326), [340](../src/cases/device/tfln_checks.rs#L340), [347](../src/cases/device/tfln_checks.rs#L347), [src/cases/electro/line_checks.rs:986](../src/cases/electro/line_checks.rs#L986), [1021](../src/cases/electro/line_checks.rs#L1021), [src/cases/mode/pockels/checks.rs:1033](../src/cases/mode/pockels/checks.rs#L1033), [src/device/measured/zhang-2022-10.8mm.toml:5](../src/device/measured/zhang-2022-10.8mm.toml#L5), [src/device/measured/zhang-2022-5.8mm.toml:5](../src/device/measured/zhang-2022-5.8mm.toml#L5), [src/device/tfln.rs:1208](../src/device/tfln.rs#L1208), [src/electro/full_wave_checks.rs:309](../src/electro/full_wave_checks.rs#L309), [src/electro/line.rs:54](../src/electro/line.rs#L54), [src/fabrication/checks.rs:461](../src/fabrication/checks.rs#L461), [src/fabrication/references.rs:152](../src/fabrication/references.rs#L152), [154](../src/fabrication/references.rs#L154)
- Jobs and circuits: [jobs/tfln-line.yaml:2](../jobs/tfln-line.yaml#L2)
- Other documents: [ROADMAP.md:784](../ROADMAP.md?plain=1#L784)

#### Zhao et al. 2020

J. Zhao, C. Ma, M. Rüsing, S. Mookherjea, "High Quality Entangled Photon Pair Generation in Periodically Poled Thin-Film Lithium Niobate Waveguides", Phys. Rev. Lett. 124, 163603 (2020). [doi:10.1103/PhysRevLett.124.163603](https://doi.org/10.1103/PhysRevLett.124.163603), open access.

- Methods pages: [nonlinear.md:30](methods/nonlinear.md?plain=1#L30)
- Validation cases: [`nonlinear/guide-zhao-period`](validation.md?plain=1#L769), [`job/nonlinear-ridge-zhao-period`](validation.md?plain=1#L778)
- Examples: [README.md:86](../examples/README.md?plain=1#L86), [pairs_zhao.rs:6](../examples/pairs_zhao.rs#L6)
- Code: [src/job/nonlinear_checks.rs:149](../src/job/nonlinear_checks.rs#L149), [src/nonlinear/guide_checks.rs:639](../src/nonlinear/guide_checks.rs#L639)
- Other documents: [ROADMAP.md:861](../ROADMAP.md?plain=1#L861), [docs/plans/quantum.md:481](plans/quantum.md?plain=1#L481)

#### Zhao et al. 2022

J. Zhao, J. Hui, Z. Ye, T. Lai, M. Y. Efremov, H. Wang, L. H. Allen, "Exploring “No Man's Land”—Arrhenius Crystallization of Thin‐Film Phase Change Material at 1 000 000 K s−1 via Nanocalorimetry", Adv Materials Inter 9, 2200429 (2022). [doi:10.1002/admi.202200429](https://doi.org/10.1002/admi.202200429), open access.

- Methods pages: [catalogue.md:417](methods/catalogue.md?plain=1#L417)
- Validation cases: [`material/pcm-gst-zhao-melting`](validation.md?plain=1#L55)
- Code: [src/material/catalogue/phase_change_thermal.rs:3](../src/material/catalogue/phase_change_thermal.rs#L3), [src/material/catalogue/references.rs:529](../src/material/catalogue/references.rs#L529), [530](../src/material/catalogue/references.rs#L530), [src/validation.rs:410](../src/validation.rs#L410)

#### Zhong et al. 2020

H.-S. Zhong, H. Wang, Y.-H. Deng, M.-C. Chen, L.-C. Peng, Y.-H. Luo, J. Qin, D. Wu, X. Ding, Y. Hu, et al., "Quantum computational advantage using photons", Science 370, 1460 (2020). [doi:10.1126/science.abe8770](https://doi.org/10.1126/science.abe8770)

- Other documents: [docs/plans/quantum.md:505](plans/quantum.md?plain=1#L505)

#### Zhou et al. 2015

M. Zhou, B. S. Lazarov, F. Wang, O. Sigmund, "Minimum length scale in topology optimization by geometric constraints", Computer Methods in Applied Mechanics and Engineering 293, 266 (2015). [doi:10.1016/j.cma.2015.05.003](https://doi.org/10.1016/j.cma.2015.05.003)

- Other documents: [ROADMAP.md:762](../ROADMAP.md?plain=1#L762)

#### Zhu et al. 2021

D. Zhu, L. Shao, M. Yu, R. Cheng, B. Desiatov, C. J. Xin, Y. Hu, J. Holzgrafe, S. Ghosh, A. Shams-Ansari, et al., "Integrated photonics on thin-film lithium niobate", Adv. Opt. Photon. 13, 242 (2021). [doi:10.1364/AOP.411024](https://doi.org/10.1364/AOP.411024)

- Methods pages: [electro-optic.md:18](methods/electro-optic.md?plain=1#L18)
- Academy lessons: [slab-waveguide.md:96](../academy/slab-waveguide.md?plain=1#L96), [590](../academy/slab-waveguide.md?plain=1#L590)
- Validation cases: [`pockels/overlap-gamma`](validation.md?plain=1#L551)
- Code: [src/cases/mode/pockels/checks.rs:1019](../src/cases/mode/pockels/checks.rs#L1019), [src/mode/pockels.rs:290](../src/mode/pockels.rs#L290)
- Other documents: [docs/plans/fabrication.md:650](plans/fabrication.md?plain=1#L650)

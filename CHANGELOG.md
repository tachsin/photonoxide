# Changelog

All notable changes to photonoxide are documented in this file, generated from the pull request titles by [release-plz](https://release-plz.dev/).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and photonoxide adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.2](https://github.com/tachsin/photonoxide/compare/v0.5.1...v0.5.2) - 2026-10-10

### <!-- 0 -->Added

- *(facade)* a small, stable façade of plain types for the bindings, with conformance cases ([#288](https://github.com/tachsin/photonoxide/pull/288))
- *(python)* the photonoxide package for Python over the façade, MATLAB wrappers, wheels and PyPI publishing ([#290](https://github.com/tachsin/photonoxide/pull/290))
- *(mode)* Krylov–Schur restarts for shift-and-invert, and every guided mode above an n_eff threshold ([#298](https://github.com/tachsin/photonoxide/pull/298))
- *(fdfd)* recycling across 3D solves: earlier solutions and GCRO-DR, a sweep's, ports', adjoints' ([#300](https://github.com/tachsin/photonoxide/pull/300))

### <!-- 1 -->Fixed

- *(fdfd)* block QMR's checks hold on every system: portable inputs, and no count compared where it runs on rounding ([#286](https://github.com/tachsin/photonoxide/pull/286))
- *(studio)* Nightly installs on Windows: the installer after the old app exits, its failure reported, an earlier watchdog superseded ([#293](https://github.com/tachsin/photonoxide/pull/293))

### <!-- 4 -->Documentation

- Python bindings in this repository, in python/ only: the rule, the roadmap and the plan's decisions ([#287](https://github.com/tachsin/photonoxide/pull/287))
- a table of what works in Rust, Python and MATLAB, checked against the façade ([#292](https://github.com/tachsin/photonoxide/pull/292))
- Stewart's Krylov–Schur paper cited by its online year, December 2001 ([#299](https://github.com/tachsin/photonoxide/pull/299))

## [0.5.1](https://github.com/tachsin/photonoxide/compare/v0.5.0...v0.5.1) - 2026-10-10

### <!-- 0 -->Added

- *(native)* the install guides checked on clean machines, the page written from them, and two discovery fixes they found ([#255](https://github.com/tachsin/photonoxide/pull/255))
- *(backend)* auto chooses the direct solver from this machine's benchmark records ([#254](https://github.com/tachsin/photonoxide/pull/254))
- *(studio)* the Academy opens each lesson on a drawing of its device, in 2D and 3D ([#274](https://github.com/tachsin/photonoxide/pull/274))
- *(native)* MUMPS as a direct solver, its sequential build, releases 5.4 to 5.8 ([#260](https://github.com/tachsin/photonoxide/pull/260))
- *(native)* SuperLU as a direct solver, releases 5 to 7 ([#261](https://github.com/tachsin/photonoxide/pull/261))
- *(mode)* every mode in a region of n_eff by contour integrals (Sakurai–Sugiura, FEAST) ([#276](https://github.com/tachsin/photonoxide/pull/276))
- *(studio)* release channels, Stable and Nightly, Nightly built here from main ([#277](https://github.com/tachsin/photonoxide/pull/277))
- *(native)* Apple Accelerate's sparse solvers as a direct solver on macOS ([#263](https://github.com/tachsin/photonoxide/pull/263))
- *(native)* MUMPS's block low-rank factorization as the backend mumps-blr, refined ([#278](https://github.com/tachsin/photonoxide/pull/278))
- *(backend)* the multifrontal fronts' dense kernels as a trait, with OpenBLAS, oneMKL and Accelerate behind it ([#269](https://github.com/tachsin/photonoxide/pull/269))
- *(fdfd)* block QMR for all of a 3D S-matrix's ports at once, opt-in ([#284](https://github.com/tachsin/photonoxide/pull/284))

### <!-- 1 -->Fixed

- the same example outputs on Linux, Windows and macOS, exp, sin and cos in pure Rust where a result hangs on the last bit ([#282](https://github.com/tachsin/photonoxide/pull/282))

### <!-- 2 -->Performance

- *(fdtd)* smoothing passes over the shapes out of a cell's reach by their boxes, and CI runs the 3D examples on runners of their own ([#265](https://github.com/tachsin/photonoxide/pull/265))
- *(fdfd)* QMR on the curl-curl operator without its matrix, a product 1.6 to 2.6 times faster at a sixth of the memory ([#259](https://github.com/tachsin/photonoxide/pull/259))

### <!-- 4 -->Documentation

- *(plans)* Wang et al.'s review of integrated photonic quantum technologies is from October 2019 ([#262](https://github.com/tachsin/photonoxide/pull/262))
- the README cut to what photonoxide does, without the padding ([#268](https://github.com/tachsin/photonoxide/pull/268))
- *(plans)* Python and MATLAB bindings, a plan and the decisions it needs ([#272](https://github.com/tachsin/photonoxide/pull/272))
- the Academy in the README's studio GIFs, in place of the themes ([#270](https://github.com/tachsin/photonoxide/pull/270))
- papers cited by the year they were published online ([#266](https://github.com/tachsin/photonoxide/pull/266))
- plans, roadmap and notes written in my own voice, not about "the owner" ([#273](https://github.com/tachsin/photonoxide/pull/273))
- *(plans)* Ryser's permanent read in Lundow and Markström 2022, his 1963 book being out of print ([#279](https://github.com/tachsin/photonoxide/pull/279))
- *(roadmap)* 0.5.1 releases the channels, backends and mode search now; many solves at once moves to 0.5.2 ([#283](https://github.com/tachsin/photonoxide/pull/283))
- 0.5.1 released, its GPU report measured on an idle machine, 0.5.2 next ([#285](https://github.com/tachsin/photonoxide/pull/285))

## [0.5.0](https://github.com/tachsin/photonoxide/compare/v0.4.3...v0.5.0) - 2026-10-09

### <!-- 0 -->Added

- *(backend)* solver backends: DirectSolver, Analysis and Factorization traits, a registry, and a choice in the solvers and in job files ([#193](https://github.com/tachsin/photonoxide/pull/193))
- *(bench)* a catalogue of problem families at many sizes, each with its task, its memory and its accuracy check ([#195](https://github.com/tachsin/photonoxide/pull/195))
- *(fdtd)* the Yee scheme in 2D and 3D with the convolutional PML ([#160](https://github.com/tachsin/photonoxide/pull/160)) ([#201](https://github.com/tachsin/photonoxide/pull/201))
- *(native)* photonoxide-native, finding and loading external libraries at run time, with smoke tests ([#202](https://github.com/tachsin/photonoxide/pull/202))
- *(fdtd)* sources: dipoles, total-field/scattered-field, one-way mode sources and Gaussian beams ([#162](https://github.com/tachsin/photonoxide/pull/162)) ([#203](https://github.com/tachsin/photonoxide/pull/203))
- *(fdtd)* Bloch-periodic boundaries and dispersive media by auxiliary differential equations ([#164](https://github.com/tachsin/photonoxide/pull/164)) ([#204](https://github.com/tachsin/photonoxide/pull/204))
- *(bench)* the benchmark runner over problems, backends and threads, with a results database ([#207](https://github.com/tachsin/photonoxide/pull/207))
- *(fdtd)* subpixel smoothing, isotropic and anisotropic ([#161](https://github.com/tachsin/photonoxide/pull/161)) ([#206](https://github.com/tachsin/photonoxide/pull/206))
- *(native)* oneMKL's PARDISO as a direct solver ([#211](https://github.com/tachsin/photonoxide/pull/211))
- *(native)* photonoxide's QMR on the GPU with cuSPARSE, an iterative backend ([#214](https://github.com/tachsin/photonoxide/pull/214))
- *(native)* QMR with ILU(0) on the GPU, photonoxide's factors and cuSPARSE's triangular solves ([#216](https://github.com/tachsin/photonoxide/pull/216))
- *(fdtd)* refuse a smoothed ε⁻¹ at the nodes that isn't positive definite ([#209](https://github.com/tachsin/photonoxide/pull/209)) ([#213](https://github.com/tachsin/photonoxide/pull/213))
- *(fdtd)* monitors: DFT fields, flux, mode overlaps and resonances by harmonic inversion ([#215](https://github.com/tachsin/photonoxide/pull/215))
- *(bench)* factor entries in the measurements and the runner's records ([#225](https://github.com/tachsin/photonoxide/pull/225))
- *(native)* GMRES with photonoxide's multigrid on the GPU ([#229](https://github.com/tachsin/photonoxide/pull/229))
- *(studio)* the Academy: lessons with live charts from the library, the ring resonator and Bragg gratings ([#233](https://github.com/tachsin/photonoxide/pull/233))
- *(fdtd)* Coupling::Triplets, Werner, Bauer & Cary's smoothing stable at any contrast ([#234](https://github.com/tachsin/photonoxide/pull/234))
- *(geometry)* the kernel: regions, primitives, transforms, polygons within a tolerance, fills and a spatial index ([#235](https://github.com/tachsin/photonoxide/pull/235))
- *(expr)* an expression language with units and parameters, in job files and sweeps ([#237](https://github.com/tachsin/photonoxide/pull/237))
- *(studio)* FDTD jobs, live field propagation and monitors ([#239](https://github.com/tachsin/photonoxide/pull/239))
- *(fdtd)* [**breaking**] Werner, Bauer and Cary's triplets are the default smoothing ([#240](https://github.com/tachsin/photonoxide/pull/240))
- *(fdtd)* adjoint gradients in 3D from FDTD runs, the mode's imaginary part included ([#246](https://github.com/tachsin/photonoxide/pull/246))
- *(bench)* the catalogue's remaining problems: stretched PMLs on the device boxes, Diel and the strip with ports at other sizes, Hadley's corners ([#242](https://github.com/tachsin/photonoxide/pull/242))
- *(fdtd)* FDTD on the GPU through wgpu compute, deterministic, judged against the blocked CPU kernel ([#252](https://github.com/tachsin/photonoxide/pull/252))
- *(fdtd)* Liu and Poon's six PDK devices in 3D FDTD against their published Lumerical FDTD and Tidy3D results; Coupling::Diagonal, a faster smoothing that keeps the blocked kernel; PortMode3d::moved_to public ([#253](https://github.com/tachsin/photonoxide/pull/253))
- *(native)* NVIDIA cuDSS, a sparse direct solver on the GPU ([#205](https://github.com/tachsin/photonoxide/pull/205))
- *(native)* cuDSS's complex symmetric L D Lᵀ ([#222](https://github.com/tachsin/photonoxide/pull/222))
- *(studio)* a Libraries page: what is found, licences, and guided installs the user confirms ([#221](https://github.com/tachsin/photonoxide/pull/221))
- *(studio)* a Benchmarks page: runs on the user's machine, charts, and which library wins where ([#226](https://github.com/tachsin/photonoxide/pull/226))

### <!-- 1 -->Fixed

- 3D direct solves were wrong on AMD Zen 3 under Windows, faer's threaded product kernel now the fixed 0.1.22 ([#192](https://github.com/tachsin/photonoxide/pull/192))
- [**breaking**] the ring's closed forms, Marcuse's loss and Marcatili's normalized constant return errors, not NaN, infinities or panics ([#191](https://github.com/tachsin/photonoxide/pull/191))

### <!-- 2 -->Performance

- *(fdtd)* the CPU kernel in f32 and f64, row by row, the same bits as the plain loops ([#218](https://github.com/tachsin/photonoxide/pull/218))
- *(fdtd)* spatial and wavefront diamond blocking, about 4 times the memory's roof beyond the caches ([#248](https://github.com/tachsin/photonoxide/pull/248))

### <!-- 3 -->Changed

- *(bench)* the catalogue's Family, Task and Entry non-exhaustive and Family::ALL a slice, so new families aren't breaking changes ([#197](https://github.com/tachsin/photonoxide/pull/197))

### <!-- 4 -->Documentation

- the plan for optional external libraries loaded at run time, and AGENTS.md's rule for them ([#196](https://github.com/tachsin/photonoxide/pull/196))
- other tools described by what they do, without comparisons that run them down; no oxiphoton ([#198](https://github.com/tachsin/photonoxide/pull/198))
- no more alpha: released milestone by milestone, and the site follows the latest release by itself ([#199](https://github.com/tachsin/photonoxide/pull/199))
- the README says what 0.4.2 and 0.4.3 released, and that 0.5 is next ([#230](https://github.com/tachsin/photonoxide/pull/230))
- no promises to fabricate: foundry and process names out of the plan, the site and the banner ([#231](https://github.com/tachsin/photonoxide/pull/231))
- *(roadmap)* 0.6.1, nonlinear integrated optics, and the materials' transparency windows ([#232](https://github.com/tachsin/photonoxide/pull/232))
- *(roadmap)* Herr et al.'s solitons paper is from December 2013 ([#236](https://github.com/tachsin/photonoxide/pull/236))
- *(plans)* the owner's decisions on the GPU: determinism, precision and where its tests run ([#241](https://github.com/tachsin/photonoxide/pull/241))
- *(plans)* photonic quantum computing: a survey and a plan for photonoxide ([#243](https://github.com/tachsin/photonoxide/pull/243))
- *(plans)* optimization in photonic design: a survey and a plan with genoxide ([#245](https://github.com/tachsin/photonoxide/pull/245))
- *(roadmap)* 0.6.2, quantum light and linear-optical statistics, and the owner's decisions on both plans ([#249](https://github.com/tachsin/photonoxide/pull/249))
- 0.5 released, FDTD and the backends in the README, 0.5.1 next ([#257](https://github.com/tachsin/photonoxide/pull/257))
- *(academy)* the Bragg lesson cites Macleod's fifth edition ([#238](https://github.com/tachsin/photonoxide/pull/238))

## [0.4.3](https://github.com/tachsin/photonoxide/compare/v0.4.2...v0.4.3) - 2026-10-05

### <!-- 0 -->Added

- the direct solves by a multifrontal LU and L D Lᵀ with static pivoting, at PARDISO's fill, 2 to 9 times faster than faer's LU ([#157](https://github.com/tachsin/photonoxide/pull/157))

## [0.4.2](https://github.com/tachsin/photonoxide/compare/v0.4.1...v0.4.2) - 2026-10-05

### <!-- 0 -->Added

- *(bench)* a benchmark harness: fixed problems timed at a stated accuracy, and `photonoxide bench` ([#135](https://github.com/tachsin/photonoxide/pull/135))
- *(fdfd)* GMRES preconditioned by a multigrid cycle for high-contrast 3D problems ([#144](https://github.com/tachsin/photonoxide/pull/144))
- *(job)* job files in JSON and YAML as well as TOML, the same job in each ([#145](https://github.com/tachsin/photonoxide/pull/145))
- *(fdfd)* QMR for complex symmetric matrices on the curl-curl operator's symmetric similarity, twice as fast ([#149](https://github.com/tachsin/photonoxide/pull/149))
- *(bench)* the bandwidth each iterative solve reaches, its bytes counted by the kernels, against the triad ([#152](https://github.com/tachsin/photonoxide/pull/152))
- *(bench)* the direct solvers' systems exported as Matrix Market files, and photonoxide's factorization timed on them ([#153](https://github.com/tachsin/photonoxide/pull/153))

### <!-- 1 -->Fixed

- *(fdfd)* a 3D port's guided mode whose plane crosses PMLs goes forward, not backward ([#136](https://github.com/tachsin/photonoxide/pull/136))

### <!-- 2 -->Performance

- *(job)* a sweep's points solved side by side on rayon's threads, recorded in order ([#146](https://github.com/tachsin/photonoxide/pull/146))
- *(fdfd)* the 3D direct solver orders its LU by nested dissection, twice as fast as COLAMD at 40³ cells ([#147](https://github.com/tachsin/photonoxide/pull/147))
- *(fdfd)* QMR's vector work fused into two passes on rayon's threads, deterministic sums, nothing allocated an iteration ([#148](https://github.com/tachsin/photonoxide/pull/148))
- the examples that sweep by hand solve their points side by side, through parallel::map_in_order ([#151](https://github.com/tachsin/photonoxide/pull/151))

### <!-- 4 -->Documentation

- *(fdfd)* parallel ILU(0) measured on our matrices, both halves, and not used ([#150](https://github.com/tachsin/photonoxide/pull/150))
- photonoxide's direct solvers against PARDISO and MUMPS, where the difference comes from, and what would close it in pure Rust ([#155](https://github.com/tachsin/photonoxide/pull/155))

## [0.4.1](https://github.com/tachsin/photonoxide/compare/v0.4.0...v0.4.1) - 2026-10-04

### <!-- 0 -->Added

- light travels along x in every kind of job ([#102](https://github.com/tachsin/photonoxide/pull/102))
- *(studio)* the settings preview the theme in use and keep the other themes behind "More themes" ([#103](https://github.com/tachsin/photonoxide/pull/103))
- *(studio)* the structure's outline on 2D fields, how a run was solved, and the field from the guide's start ([#104](https://github.com/tachsin/photonoxide/pull/104))
- *(studio)* every length in µm or nm, app-wide, switched by clicking its unit ([#107](https://github.com/tachsin/photonoxide/pull/107))
- *(studio)* a menu that folds without anything in it moving, hints beside folded items, and bars that keep still ([#108](https://github.com/tachsin/photonoxide/pull/108))
- *(material)* crystal tags from the point group, what is coming and from which paper, and the Materials page's stale error ([#123](https://github.com/tachsin/photonoxide/pull/123))
- *(material)* five gaps filled from their papers: MgO:LiNbO3's r13, r33 and r22, LiNbO3's d22, AlGaAs's r41 and AlGaN's d31 and d33 ([#124](https://github.com/tachsin/photonoxide/pull/124))

### <!-- 1 -->Fixed

- *(site)* the overview's version pill shows the release number only, and the title fits a 320 px phone ([#96](https://github.com/tachsin/photonoxide/pull/96))
- *(site)* the hero's name on one line on an iPhone, sized for its monospace width ([#97](https://github.com/tachsin/photonoxide/pull/97))
- *(site)* the hero's animation pauses while the page scrolls, so the sticky bar doesn't shiver on an iPhone ([#98](https://github.com/tachsin/photonoxide/pull/98))
- *(studio)* the viewer follows a running sweep, and an FDFD sweep's points each have their field ([#100](https://github.com/tachsin/photonoxide/pull/100))
- *(job)* the check refuses a wavelength a material has no data at, as the run does ([#101](https://github.com/tachsin/photonoxide/pull/101))
- *(job)* the check refuses badly placed fdfd ports, PMLs that leave no room and a grid too fine, as the run does ([#105](https://github.com/tachsin/photonoxide/pull/105))
- *(job)* an fdfd run's pictures leave out the PMLs, so the light starts and ends where the device is drawn ([#106](https://github.com/tachsin/photonoxide/pull/106))
- *(studio)* the builder's four-column rows drop to two on a narrow form, so a 5-digit nm value isn't clipped ([#109](https://github.com/tachsin/photonoxide/pull/109))
- clear errors for degenerate inputs that hung, filled the memory or gave NaNs ([#110](https://github.com/tachsin/photonoxide/pull/110))
- *(studio)* a 960-wide window fits every page, and the console and the build are quiet ([#111](https://github.com/tachsin/photonoxide/pull/111))
- the check refuses jobs that ran on something else, vector_fit refuses repeated samples, and the viewer fits 960 px ([#121](https://github.com/tachsin/photonoxide/pull/121))
- *(mode)* a bent slab is solved in milliseconds whatever its radius ([#126](https://github.com/tachsin/photonoxide/pull/126))
- a long mode solve heeds the stop and the time limit, and a modes job asks for at most 50 modes ([#125](https://github.com/tachsin/photonoxide/pull/125))
- *(eigen)* six or more modes of a cross-section converge, each restart taking a larger Krylov space ([#129](https://github.com/tachsin/photonoxide/pull/129))

### <!-- 4 -->Documentation

- 0.4.0 is released, and 0.4.1's preconditioner is next ([#93](https://github.com/tachsin/photonoxide/pull/93))
- a performance plan (GPU, distributed, kernels) and the roadmap it changes ([#127](https://github.com/tachsin/photonoxide/pull/127))
- 0.4.1 ships as polish, and the preconditioner moves to 0.4.2 ([#130](https://github.com/tachsin/photonoxide/pull/130))

## [0.4.0](https://github.com/tachsin/photonoxide/compare/v0.3.3...v0.4.0) - 2026-10-03

### Breaking

- `ParametricModel::fit` takes an `Interpolation` in place of the polynomial degree: `Interpolation::Polynomial { degree }` for the previous fit, `Interpolation::PiecewiseLinear` for Triverio's ([#86](https://github.com/tachsin/photonoxide/pull/86))
- 2D FDFD ports: with H along z, S's reflections are the tangential E's, as the 3D ports' and mode expansions', and so minus those before 0.4.0; a mode's backward amplitude follows ([#90](https://github.com/tachsin/photonoxide/pull/90))
- `Boundaries3d` has a new public field, `real_stretch`: struct literals need it, or `..Boundaries3d::pml(cells)` ([#84](https://github.com/tachsin/photonoxide/pull/84))

### <!-- 0 -->Added

- components and netlists, the foundation of circuits ([#71](https://github.com/tachsin/photonoxide/pull/71))
- the circuit solve: one sparse system per netlist, checked against Filipsson's sub-network growth ([#74](https://github.com/tachsin/photonoxide/pull/74))
- read and write Touchstone files, Version 1 and 2.0 ([#72](https://github.com/tachsin/photonoxide/pull/72))
- the circuit adjoint: every parameter's gradient from one transposed solve ([#75](https://github.com/tachsin/photonoxide/pull/75))
- optimization at circuit level through genoxide: a splitter, a ring at critical coupling and a fit ([#76](https://github.com/tachsin/photonoxide/pull/76))
- compact models by vector fitting, over parameters, and the measured fidelity ([#78](https://github.com/tachsin/photonoxide/pull/78))
- 3D FDFD ports: the grid's own full-vector port modes, one-way mode sources and a reciprocal S-matrix ([#77](https://github.com/tachsin/photonoxide/pull/77))
- the first components: waveguide, bend, couplers, MMI, Y-branch, rings and MZI ([#79](https://github.com/tachsin/photonoxide/pull/79))
- *(studio)* a component library and a chip view ([#80](https://github.com/tachsin/photonoxide/pull/80))
- validate against measured Mach-Zehnder interferometers (Dwivedi 2015) ([#85](https://github.com/tachsin/photonoxide/pull/85))
- *(material)* AlN's index (Rigler 2015), AlGaN films (Rigler 2013), and InGaP beyond Tanaka's range (Ferrini 2002) ([#87](https://github.com/tachsin/photonoxide/pull/87))
- [**breaking**] compact models checked against their papers: Triverio's piecewise-linear model with an exact uniform stability test ([#86](https://github.com/tachsin/photonoxide/pull/86))
- [**breaking**] QMR preconditioned by ILU(0) on Shin and Fan's operator, with PMLs stretched as much as they absorb ([#84](https://github.com/tachsin/photonoxide/pull/84))

### <!-- 1 -->Fixed

- a 3D direct solve reaches round-off on any machine, finishing by QMR on an inaccurate factorization ([#82](https://github.com/tachsin/photonoxide/pull/82))
- 0.4 follow-ups: MMI port polarization, provenance -0.000, parallel spectra, roadmap ([#83](https://github.com/tachsin/photonoxide/pull/83))
- [**breaking**] 2D reflections with H along z by the tangential E's convention, as the 3D ports' ([#90](https://github.com/tachsin/photonoxide/pull/90))

### <!-- 4 -->Documentation

- bring the README up to date with 0.3.3 and the 0.4 work on main ([#88](https://github.com/tachsin/photonoxide/pull/88))
- bring the site, getting started, the studio's README and AGENTS.md up to date ([#89](https://github.com/tachsin/photonoxide/pull/89))
- animate the studio in the README, recorded by a script ([#91](https://github.com/tachsin/photonoxide/pull/91))
- describe the crate as 0.4.0 has it, with FDTD, inverse design, layout and PDKs as planned ([#92](https://github.com/tachsin/photonoxide/pull/92))

## [0.3.3](https://github.com/tachsin/photonoxide/compare/v0.3.2...v0.3.3) - 2026-10-03

### <!-- 0 -->Added

- the selected mode travels along its guide in the 3D viewer, and job previews show a modes job whole ([#61](https://github.com/tachsin/photonoxide/pull/61))
- *(validation)* the cases' math as LaTeX, rendered by KaTeX in the studio and on the site ([#62](https://github.com/tachsin/photonoxide/pull/62))
- flip through a sweep's points in the viewer, each with its structure and its modes ([#67](https://github.com/tachsin/photonoxide/pull/67))
- a materials catalogue with provenance, and a Materials page in the studio ([#70](https://github.com/tachsin/photonoxide/pull/70))

### <!-- 1 -->Fixed

- *(studio)* a release in the making is announced as on its way, not as an error ([#58](https://github.com/tachsin/photonoxide/pull/58))
- the check refuses windows that run backwards, a step that isn't positive and sweeps a run can't take; a width sweep's point shows its own cross-section ([#68](https://github.com/tachsin/photonoxide/pull/68))
- new run events go after the old ones, so their discriminants keep their values ([#69](https://github.com/tachsin/photonoxide/pull/69))

### <!-- 4 -->Documentation

- method write-ups' math as GitHub renders it ([#63](https://github.com/tachsin/photonoxide/pull/63))

## [0.3.2](https://github.com/tachsin/photonoxide/compare/v0.3.1...v0.3.2) - 2026-10-02

### <!-- 0 -->Added

- *(studio)* rings, 3D previews of every job, and a steady 3D view ([#54](https://github.com/tachsin/photonoxide/pull/54))
- *(studio)* look for a new release every hour while the window is open ([#56](https://github.com/tachsin/photonoxide/pull/56))

## [0.3.1](https://github.com/tachsin/photonoxide/compare/v0.3.0...v0.3.1) - 2026-10-02

### <!-- 0 -->Added

- *(studio)* the studio as a workspace, with examples inside and updates by one click ([#52](https://github.com/tachsin/photonoxide/pull/52))

## [0.3.0](https://github.com/tachsin/photonoxide/compare/v0.2.0...v0.3.0) - 2026-10-02

### <!-- 0 -->Added

- [**breaking**] make the studio a Tauri app with a 3D view, and the photonoxide program ([#36](https://github.com/tachsin/photonoxide/pull/36))
- open the studio's start page with a bare photonoxide, and release binaries ([#38](https://github.com/tachsin/photonoxide/pull/38))
- add a multilayer stack's reflection and transmission of a plane wave ([#40](https://github.com/tachsin/photonoxide/pull/40))
- add 2D FDFD with stretched-coordinate PMLs and its exact power flux ([#41](https://github.com/tachsin/photonoxide/pull/41))
- add 2D FDFD ports: the grid's own modes, one-way sources and a reciprocal S-matrix ([#42](https://github.com/tachsin/photonoxide/pull/42))
- reuse an FDFD matrix's symbolic analysis across a sweep ([#43](https://github.com/tachsin/photonoxide/pull/43))
- add an fdfd job: a device on one layer by 2D FDFD with ports, live in the studio ([#44](https://github.com/tachsin/photonoxide/pull/44))
- add adjoint gradients for 2D FDFD, checked against finite differences ([#45](https://github.com/tachsin/photonoxide/pull/45))
- add Hadley's high-accuracy interface and corner equations as a full-vector mode solver ([#47](https://github.com/tachsin/photonoxide/pull/47))
- add 3D FDFD on the Yee grid with stretched-coordinate PMLs ([#46](https://github.com/tachsin/photonoxide/pull/46))
- add a QMR iterative solver for 3D FDFD, on the curl-curl operator or Shin and Fan's ([#48](https://github.com/tachsin/photonoxide/pull/48))

### <!-- 1 -->Fixed

- *(fdfd)* put Shin and Fan's ε⁻¹ at the nodes, inside the gradient ([#50](https://github.com/tachsin/photonoxide/pull/50))

### <!-- 4 -->Documentation

- reshape the roadmap around components, circuits and active photonics ([#49](https://github.com/tachsin/photonoxide/pull/49))
- mark 0.2 and 0.3 done in the roadmap ([#51](https://github.com/tachsin/photonoxide/pull/51))

## [0.2.0](https://github.com/tachsin/photonoxide/compare/v0.1.1...v0.2.0) - 2026-10-02

### <!-- 0 -->Added

- add the exact TE and TM modes of three-layer slabs ([#14](https://github.com/tachsin/photonoxide/pull/14))
- add the full-vector finite-difference mode solver, with shift-and-invert Arnoldi ([#16](https://github.com/tachsin/photonoxide/pull/16))
- add mirror walls to the vector mode solver, and validate its corners against Hadley ([#20](https://github.com/tachsin/photonoxide/pull/20))
- add group index, dispersion, loss and mode tracking ([#21](https://github.com/tachsin/photonoxide/pull/21))
- add exact multilayer slab modes and leaky waves by transfer matrices ([#22](https://github.com/tachsin/photonoxide/pull/22))
- add a PML to the vector mode solver for leaky modes ([#23](https://github.com/tachsin/photonoxide/pull/23))
- add the effective index method, with its error against the vector solver ([#24](https://github.com/tachsin/photonoxide/pull/24))
- add bends: an exact bent slab, and bent cross-sections in the vector solver ([#26](https://github.com/tachsin/photonoxide/pull/26))
- add a vector mode's full fields, power and coupling into another mode ([#28](https://github.com/tachsin/photonoxide/pull/28))
- add Marcatili's approximation, validated against the vector solver in its regime ([#29](https://github.com/tachsin/photonoxide/pull/29))
- add planar profiles by 1D finite differences, with a PML ([#30](https://github.com/tachsin/photonoxide/pull/30))
- add the studio's mode viewer, with sweeps over wavelength and width ([#31](https://github.com/tachsin/photonoxide/pull/31))

### <!-- 4 -->Documentation

- add examples, each reproducing a published result ([#18](https://github.com/tachsin/photonoxide/pull/18))
- add the banner, logo and README badges ([#19](https://github.com/tachsin/photonoxide/pull/19))
- add the project site, method write-ups and example outputs ([#25](https://github.com/tachsin/photonoxide/pull/25))
- describe 0.2 as released ([#35](https://github.com/tachsin/photonoxide/pull/35))

## [0.1.1](https://github.com/tachsin/photonoxide/compare/v0.1.0...v0.1.1) - 2026-10-01

### <!-- 0 -->Added

- check the material data against Li, Malitson and Luke, and validate silica against Malitson's Table I ([#11](https://github.com/tachsin/photonoxide/pull/11))

### <!-- 4 -->Documentation

- cite the book's page and words for the 220 nm on 2 um SOI stack ([#13](https://github.com/tachsin/photonoxide/pull/13))

## [0.1.0](https://github.com/tachsin/photonoxide/releases/tag/v0.1.0) - 2026-10-01

0.1 Foundations (ROADMAP.md).

### <!-- 0 -->Added

- add the error type, and CI on Linux, macOS and Windows ([#1](https://github.com/tachsin/photonoxide/pull/1))
- add typed lengths, wavelengths and frequencies, and the e^(-iwt) convention ([#2](https://github.com/tachsin/photonoxide/pull/2))
- add materials with their provenance: Si (Li 1980), SiO2 (Malitson 1965), Si3N4 (Luke 2015) ([#3](https://github.com/tachsin/photonoxide/pull/3))
- add planar shapes, layer stacks with SOI and nitride presets, and structures ([#4](https://github.com/tachsin/photonoxide/pull/4))
- add job files, run directories, event records with exact replay, and stops ([#5](https://github.com/tachsin/photonoxide/pull/5))
- add the validation harness, its report and the photonoxide validate command ([#6](https://github.com/tachsin/photonoxide/pull/6))
- add the studio window, structure jobs and permittivity rasters ([#7](https://github.com/tachsin/photonoxide/pull/7))
- read refractiveindex.info files: its nine formulas, tabulated n, k and nk, with their provenance ([#8](https://github.com/tachsin/photonoxide/pull/8))

### <!-- 3 -->Changed

- raise the minimum Rust to 1.95 for eframe and egui 0.36 ([#9](https://github.com/tachsin/photonoxide/pull/9))

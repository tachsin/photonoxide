import site from "@/photonoxide-site.json";

/**
 * photonoxide's static facts: names, links and the landing page's copy.
 * Everything that changes with the repository (the methods, examples, the
 * validation report, the roadmap, and the highlights' status) is read from it
 * at the latest release instead: see github.js and the loaders beside it.
 */

export const PHOTONOXIDE_REPO = "tachsin/photonoxide";
export const PHOTONOXIDE_BRANCH = "main";

/**
 * The commit these pages were synced from (the app's photonoxide-site.json).
 * The repository's files are read at it too, so the pages and the files they
 * expect always come from the same commit, and a new pin is new URLs.
 */
export const PHOTONOXIDE_COMMIT = site.commit;

export const PHOTONOXIDE_PATH = "/projects/photonoxide";
export const PHOTONOXIDE_OG_IMAGE = "/projects/photonoxide/opengraph-image";

export const PHOTONOXIDE_TAGLINE = "Photonics for Rust: validated, visible while it runs";

/** The project's pages, under PHOTONOXIDE_PATH. */
export const METHODS_PATH = "/projects/photonoxide/methods";
export const EXAMPLES_PATH = "/projects/photonoxide/examples";
export const VALIDATION_PATH = "/projects/photonoxide/validation";
export const ROADMAP_PATH = "/projects/photonoxide/roadmap";
export const DOCS_PATH = "/projects/photonoxide/docs";

export const PHOTONOXIDE_DESCRIPTION =
  "photonoxide is a photonics library for Rust: materials with provenance, mode solvers, 2D and 3D FDFD and FDTD on the CPU and GPU, components and circuits with their adjoint, and compact models, with a studio to build jobs and chips and watch every run live. Every method is validated against analytic solutions and published results. Active devices, inverse design and layout export are planned.";

export const PHOTONOXIDE_LICENSE = "MIT OR Apache-2.0";

const BLOB = `https://github.com/${PHOTONOXIDE_REPO}/blob/${PHOTONOXIDE_BRANCH}/`;

export const PHOTONOXIDE_LINKS = {
  github: `https://github.com/${PHOTONOXIDE_REPO}`,
  roadmap: `${BLOB}ROADMAP.md`,
  readme: `${BLOB}README.md`,
  issues: `https://github.com/${PHOTONOXIDE_REPO}/issues`,
  pitfalls: `${BLOB}ROADMAP.md#pitfalls-ruled-out-by-design`,
  validation: `${BLOB}ROADMAP.md#validation`,
  references: `${BLOB}ROADMAP.md#references`,
  genoxide: "/projects/genoxide",
  crates: "https://crates.io/crates/photonoxide",
  docsRs: "https://docs.rs/photonoxide",
  examplesDir: `https://github.com/${PHOTONOXIDE_REPO}/tree/${PHOTONOXIDE_BRANCH}/examples`,
  methodsDir: `https://github.com/${PHOTONOXIDE_REPO}/tree/${PHOTONOXIDE_BRANCH}/docs/methods`,
  report: `${BLOB}docs/validation.md`,
};

export const PHOTONOXIDE_KEYWORDS = [
  "photonoxide",
  "photonics",
  "silicon photonics",
  "FDTD",
  "FDFD",
  "mode solver",
  "inverse design",
  "topology optimization",
  "adjoint method",
  "GDSII",
  "PDK",
  "Rust",
  "GPU",
  "computational electromagnetics",
  "integrated photonics",
];

/**
 * Landing page highlights: what the library does, then what the roadmap plans. `icon` is a key
 * of HIGHLIGHT_ICONS in the page. `milestone` is the ROADMAP.md milestone that brings it: the
 * page reads its status from the roadmap at the latest release, so a highlight shows as done,
 * next or planned without an edit here. One without a milestone is always true. `hero` is its
 * name in the hero's line, which lists what is done and what is to come.
 */
export const PHOTONOXIDE_HIGHLIGHTS = [
  {
    icon: "modes",
    title: "Mode solvers",
    milestone: "0.2",
    hero: "mode solvers",
    body: "Exact slabs and multilayers, full-vector cross-sections with PMLs and Hadley's corner equations, bends, the effective index method, dispersion and overlaps, and every mode in a region of n_eff by contour integrals.",
  },
  {
    icon: "fields",
    title: "FDFD",
    milestone: "0.3",
    hero: "FDFD",
    body: "Frequency-domain finite differences in 2D and 3D on Yee's grid: stretched-coordinate PMLs, mode ports and reciprocal S-matrices, adjoint gradients, and direct and preconditioned QMR solves, all of a 3D S-matrix's ports as one block.",
  },
  {
    icon: "materials",
    title: "Materials catalogue",
    milestone: "0.3.x",
    body: "Silica, silicon, nitride, lithium niobate, GaAs, AlGaAs, InGaP, InP and AlN: index models, crystals, χ⁽²⁾ and Pockels tensors, every number from its paper.",
  },
  {
    icon: "studio",
    title: "The studio",
    milestone: "0.3.1",
    body: "A desktop app on a workspace: the examples built in, a job builder with a 3D preview, runs live in 3D and 2D, run comparison, the materials, the validation report, the Academy's lessons, and Stable and Nightly channels.",
  },
  {
    icon: "circuits",
    title: "Components and circuits",
    milestone: "0.4",
    hero: "circuits",
    body: "Waveguides, couplers, MMIs, rings and MZIs with ports and fidelities; netlists solved as one sparse system; the circuit adjoint and genoxide's optimizers; the studio's component library and chip view.",
  },
  {
    icon: "compact",
    title: "Compact models",
    milestone: "0.4",
    body: "Vector fitting with its error, stability and passivity, models over parameters, and Touchstone files read and written as the measured fidelity.",
  },
  {
    icon: "time",
    title: "FDTD and GPU",
    milestone: "0.5",
    hero: "FDTD on the CPU and GPU",
    body: "Time-domain finite differences in 2D and 3D: CPML, subpixel smoothing, mode sources and Gaussian beams, DFT monitors and harmonic inversion, Drude and Lorentz media, 3D adjoint gradients, and a GPU backend on wgpu.",
  },
  {
    icon: "solves",
    title: "Many solves at once",
    milestone: "0.5.2",
    body: "Direct solvers chosen from this machine's benchmarks, MUMPS, SuperLU and Accelerate among them; Krylov subspaces recycled across sweeps and adjoints; sweeps and whole jobs farmed to worker processes, here or over TCP.",
  },
  {
    icon: "rust",
    title: "Rust first",
    body: "Pure Rust from the linear algebra up, spectra solved in parallel on the CPU. External solvers are optional, loaded at run time; Python and MATLAB packages wrap a small, stable façade.",
  },
  {
    icon: "active",
    title: "Active devices",
    milestone: "0.6",
    hero: "active devices",
    body: "Thermo-optic phase shifters, Pockels modulators in thin-film lithium niobate, travelling-wave electrodes, and carrier modulators.",
  },
  {
    icon: "inverse",
    title: "Inverse design",
    milestone: "0.7",
    hero: "inverse design",
    body: "Adjoint topology and shape optimization with robust and fabrication-rule constraints, on genoxide's optimizers.",
  },
  {
    icon: "layout",
    title: "Layout and PDKs",
    milestone: "0.9",
    hero: "layout export",
    body: "GDSII and OASIS, parametric cells with ports, routing, and design-rule checks on the rules the user loads.",
  },
  {
    icon: "export",
    title: "Fabrication data",
    milestone: "0.10",
    body: "Layout packages on the user's layer map, with test structures, design and connectivity checks, and a design record.",
  },
];

/** ROADMAP.md's success criteria for 1.0. */
export const PHOTONOXIDE_PROPERTIES = [
  {
    title: "Validated",
    body: "Every solver passes analytic tests, reproduces published results and agrees with an established code, in a public report.",
  },
  {
    title: "Converged",
    body: "Every number comes with its grid, boundaries and run time. Unconverged results are flagged, never quietly reported.",
  },
  {
    title: "Fabrication-aware",
    body: "Designs are checked against the design rules the user supplies and report their performance across process variation, not just at nominal.",
  },
  {
    title: "Reproducible",
    body: "The same input gives bit-for-bit the same result on any number of threads.",
  },
];

/** From a simulation to a checked layout: the path the library covers. */
export const PHOTONOXIDE_PIPELINE = [
  { title: "Simulate", body: "Modes, fields and spectra, each with its convergence." },
  { title: "Optimize", body: "Adjoint inverse design under fabrication rules, robust to process variation." },
  { title: "Lay out", body: "Polygons on the process's layers, with ports, routing and test structures." },
  { title: "Check", body: "Design rules and connectivity, on the rules the user loads." },
  { title: "Export", body: "GDSII or OASIS, with the design record next to it." },
  { title: "Compare", body: "Measured spectra the user has, imported and compared with the prediction." },
];

/**
 * What the validation report has, by tier, and the cross-code comparisons ROADMAP.md plans.
 * `planned` marks a tier the report doesn't have yet.
 */
export const PHOTONOXIDE_VALIDATION = [
  {
    title: "Analytic",
    body: "Closed forms and exact properties: the exact slab and bent slab, Fresnel and transfer-matrix reflection, PML and CPML reflection, the Yee scheme's numerical dispersion, reciprocity, energy conservation and unitarity, ring responses and free spectral ranges, and every adjoint against finite differences.",
  },
  {
    title: "Published",
    body: "Papers' tables and figures reproduced: material data (Li, Malitson, Zelmon and more), Marcatili, Hadley's corner problems, the leaky photonic-wire benchmark, Bogaerts's rings, Gustavsen and Semlyen's vector fitting, Mie's scattering table, Meep's PML convergence rates, and Liu and Poon's open-PDK devices beside two commercial codes' results.",
  },
  {
    title: "Measured",
    body: "Dwivedi et al.'s published Mach-Zehnder interferometers: three wires' effective and group indices, predicted from their measured cross-sections, within the paper's fabrication estimate.",
  },
  {
    title: "Cross-code",
    planned: true,
    body: "The same structures in Meep, MPB, S4 and Ceviche, run as external programs, with the later milestones.",
  },
];

/**
 * The milestones when ROADMAP.md can't be fetched: titles only, no
 * progress. Its headings, in order: the studio's tests check it.
 */
export const FALLBACK_MILESTONES = [
  "0.0: Project setup ✅",
  "0.1: Foundations ✅",
  "0.2: Mode solvers ✅",
  "0.3: Frequency-domain finite differences (FDFD) ✅",
  "0.3.1: The studio as a workspace ✅",
  "0.3.2: The studio, polished ✅",
  "0.3.x: Materials catalogue ✅",
  "0.4: Components and circuits ✅",
  "0.4.1: Polish ✅",
  "0.4.2: A preconditioner for high-contrast 3D FDFD, and the whole machine (CPU) ✅",
  "0.4.3: Direct solves at PARDISO's fill ✅",
  "0.5: Finite-difference time-domain (FDTD) ✅",
  "0.5.1: Channels, more backends, and every mode in a region ✅",
  "0.5.2: Many solves at once ✅",
  "0.6: Thermal and electro-optic devices",
  "0.6.1: Nonlinear integrated optics",
  "0.6.2: Quantum light and linear-optical statistics",
  "0.7: Inverse design",
  "0.7.1: Distributed memory",
  "0.8: Carrier modulators, signals and programmable circuits",
  "0.9: Layout and PDK",
  "0.10: Fabrication data",
  "0.11: Fabrication realism",
  "0.12: Semi-analytic methods",
  "0.13: Device library",
  "0.14: Periodic structures and nanophotonics",
  "0.15: Nonlinear FDTD and fiber optics",
  "0.16: Beyond",
  "1.0: Stable",
];

import site from "@/photonoxide-site.json";

/**
 * photonoxide's static facts: names, links and the landing page's copy.
 * Everything that changes with the repository (the methods, examples, the
 * validation report, the roadmap) is read from it at the pinned commit
 * instead: see github.js and the loaders beside it.
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

export const PHOTONOXIDE_TAGLINE = "Photonics for Rust: validated, fabrication-ready, visible while it runs";

/** The project's pages, under PHOTONOXIDE_PATH. */
export const METHODS_PATH = "/projects/photonoxide/methods";
export const EXAMPLES_PATH = "/projects/photonoxide/examples";
export const VALIDATION_PATH = "/projects/photonoxide/validation";
export const ROADMAP_PATH = "/projects/photonoxide/roadmap";
export const DOCS_PATH = "/projects/photonoxide/docs";

export const PHOTONOXIDE_DESCRIPTION =
  "photonoxide is a photonics library for Rust: materials with provenance, mode solvers, 2D and 3D FDFD, components and circuits with their adjoint, and compact models, with a studio to build jobs and chips and watch every run live. Every method is validated against analytic solutions and published results. FDTD, active devices, inverse design, layout and tape-out are planned.";

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
  "tape-out",
  "Rust",
];

/**
 * Landing page highlights: what the library does now, then what the roadmap plans. `icon` is a
 * key of HIGHLIGHT_ICONS in the page; `status` is "released" (on crates.io), "main" (on the
 * main branch, for the next release) or "planned" (ROADMAP.md's later milestones).
 */
export const PHOTONOXIDE_HIGHLIGHTS = [
  {
    icon: "modes",
    title: "Mode solvers",
    status: "released",
    body: "Exact slabs and multilayers, full-vector cross-sections with PMLs and Hadley's corner equations, bends, the effective index method, dispersion and overlaps.",
  },
  {
    icon: "fields",
    title: "FDFD",
    status: "released",
    body: "Frequency-domain finite differences in 2D and 3D on Yee's grid: stretched-coordinate PMLs, mode ports and reciprocal S-matrices in 2D and 3D, adjoint gradients in 2D, and direct and preconditioned QMR solves.",
  },
  {
    icon: "materials",
    title: "Materials catalogue",
    status: "released",
    body: "Silica, silicon, nitride, lithium niobate, GaAs, AlGaAs, InGaP, InP and AlN: index models, crystals, χ⁽²⁾ and Pockels tensors, every number from its paper.",
  },
  {
    icon: "studio",
    title: "The studio",
    status: "released",
    body: "A desktop app on a workspace: the examples built in, a job builder with a 3D preview, runs live in 3D and 2D, run comparison, the materials, the validation report, and updates by one click.",
  },
  {
    icon: "circuits",
    title: "Components and circuits",
    status: "released",
    body: "Waveguides, couplers, MMIs, rings and MZIs with ports and fidelities; netlists solved as one sparse system; the circuit adjoint and genoxide's optimizers; the studio's component library and chip view.",
  },
  {
    icon: "compact",
    title: "Compact models",
    status: "released",
    body: "Vector fitting with its error, stability and passivity, models over parameters, and Touchstone files read and written as the measured fidelity.",
  },
  {
    icon: "rust",
    title: "Pure Rust",
    status: "released",
    body: "No C, Fortran or Python, from the linear algebra up, and no Python bindings. Spectra are solved in parallel on the CPU.",
  },
  {
    icon: "time",
    title: "FDTD and GPU",
    status: "planned",
    body: "Time-domain finite differences in 2D and 3D with CPML, subpixel smoothing and dispersive media, and a GPU backend.",
  },
  {
    icon: "active",
    title: "Active devices",
    status: "planned",
    body: "Thermo-optic phase shifters, Pockels modulators in thin-film lithium niobate, travelling-wave electrodes, and carrier modulators.",
  },
  {
    icon: "inverse",
    title: "Inverse design",
    status: "planned",
    body: "Adjoint topology and shape optimization with robust and foundry-rule constraints, on genoxide's optimizers.",
  },
  {
    icon: "layout",
    title: "Layout and PDKs",
    status: "planned",
    body: "GDSII and OASIS, parametric cells with ports, routing, design-rule checks, and the open SiEPIC EBeam and Cornerstone PDKs.",
  },
  {
    icon: "tapeout",
    title: "Tape-out",
    status: "planned",
    body: "Submission packages for multi-project wafer runs: the foundry's layers, black-box cells, test structures, sign-off and connectivity checks.",
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
    title: "Fabricable",
    body: "Designs pass an open PDK's design rules and report their performance across process variation, not just at nominal.",
  },
  {
    title: "Reproducible",
    body: "The same input gives bit-for-bit the same result on any number of threads.",
  },
];

/** From a simulation to a measured chip: the path the library covers. */
export const PHOTONOXIDE_PIPELINE = [
  { title: "Simulate", body: "Modes, fields and spectra, each with its convergence." },
  { title: "Optimize", body: "Adjoint inverse design under the foundry's rules, robust to process variation." },
  { title: "Lay out", body: "Polygons on the PDK's layers, with ports, routing and test structures." },
  { title: "Check", body: "Design rules, connectivity, and sign-off with the foundry's own deck." },
  { title: "Tape out", body: "GDSII or OASIS for a multi-project wafer run. SiEPIC openEBL is the first target." },
  { title: "Measure", body: "Measured spectra back into the validation report, next to the prediction." },
];

/**
 * What the validation report has, by tier, and the cross-code comparisons ROADMAP.md plans.
 * `planned` marks a tier the report doesn't have yet.
 */
export const PHOTONOXIDE_VALIDATION = [
  {
    title: "Analytic",
    body: "Closed forms and exact properties: the exact slab and bent slab, Fresnel and transfer-matrix reflection, PML reflection, reciprocity, energy conservation and unitarity, ring responses and free spectral ranges, and every adjoint against finite differences.",
  },
  {
    title: "Published",
    body: "Papers' tables and figures reproduced: material data (Li, Malitson, Zelmon and more), Marcatili, Hadley's corner problems, the leaky photonic-wire benchmark, Bogaerts's rings, Gustavsen and Semlyen's vector fitting.",
  },
  {
    title: "Measured",
    body: "Dwivedi et al.'s Mach-Zehnder interferometers on imec's line: three wires' effective and group indices, predicted from their measured cross-sections, within the paper's fabrication estimate.",
  },
  {
    title: "Cross-code",
    planned: true,
    body: "The same structures in Meep, MPB, S4 and Ceviche, run as external programs, with the later milestones.",
  },
];

/**
 * The milestones when ROADMAP.md can't be fetched: titles only, no
 * progress. Keep in step with the repository's roadmap.
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
  "0.5: Finite-difference time-domain (FDTD)",
  "0.5.1: Many solves at once",
  "0.6: Thermal and electro-optic devices",
  "0.7: Inverse design",
  "0.7.1: Distributed memory",
  "0.8: Carrier modulators, signals and programmable circuits",
  "0.9: Layout and PDK",
  "0.10: Tape-out",
  "0.11: Fabrication realism",
  "0.12: Semi-analytic methods",
  "0.13: Device library",
  "0.14: Periodic structures and nanophotonics",
  "0.15: Nonlinear and fiber optics",
  "0.16: Beyond",
  "1.0: Stable",
];

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
  "photonoxide is a photonics library for Rust: mode solvers, FDFD, FDTD, semi-analytic methods, inverse design, layout, PDKs and tape-out in one library, with a studio to watch every simulation and optimization live. Every solver is validated against analytic solutions and published results.";

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

/** Landing page highlights, from ROADMAP.md's architecture. `icon` is a key of HIGHLIGHT_ICONS in the page. */
export const PHOTONOXIDE_HIGHLIGHTS = [
  {
    icon: "modes",
    title: "Mode solvers",
    body: "Slab and full-vector cross-section modes, the effective index method, bends, dispersion and overlaps.",
  },
  {
    icon: "fields",
    title: "FDFD and FDTD",
    body: "Frequency- and time-domain finite differences in 2D and 3D, with CPML, mode ports, subpixel smoothing, dispersive media and a GPU backend.",
  },
  {
    icon: "layers",
    title: "Semi-analytic methods",
    body: "Transfer matrices for thin films, RCWA for gratings, eigenmode expansion and the beam propagation method.",
  },
  {
    icon: "inverse",
    title: "Inverse design",
    body: "Adjoint gradients for every solver, density and level-set topology optimization, robust and foundry-rule constraints, with genoxide's optimizers.",
  },
  {
    icon: "layout",
    title: "Layout and PDKs",
    body: "Our own geometry engine, GDSII and OASIS, parametric cells with ports, routing, design-rule checks, and the open SiEPIC EBeam and Cornerstone PDKs.",
  },
  {
    icon: "tapeout",
    title: "Tape-out",
    body: "Submission packages for multi-project wafer runs: the foundry's layers, black-box cells, test structures, sign-off and connectivity checks.",
  },
  {
    icon: "studio",
    title: "The studio",
    body: "A native window that shows fields propagating, modes, layouts and optimizations as they run. Every run replays from its record.",
  },
  {
    icon: "rust",
    title: "Pure Rust",
    body: "No C, Fortran or Python, from the linear algebra to the GDS writer. Parallel on the CPU, with the GPU through wgpu.",
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

/** ROADMAP.md's three tiers of validation. */
export const PHOTONOXIDE_VALIDATION = [
  {
    title: "Analytic",
    body: "Fresnel coefficients, slab modes, Bragg stacks, Mie scattering, PML reflection, the Yee scheme's numerical dispersion, MMI self-imaging, ring free spectral ranges.",
  },
  {
    title: "Cross-code",
    body: "The same structures at the same resolution in Meep, MPB, S4, Ceviche and oxiphoton, run as external programs.",
  },
  {
    title: "Published devices",
    body: "Inverse-designed demultiplexers, beamsplitters, mode multiplexers and grating couplers reproduced in 3D, and our own chips measured.",
  },
];

/**
 * The milestones when ROADMAP.md can't be fetched: titles only, no
 * progress. Keep in step with the repository's roadmap.
 */
export const FALLBACK_MILESTONES = [
  "0.0: Project setup ✅",
  "0.1: Foundations",
  "0.2: Mode solvers",
  "0.3: Frequency-domain finite differences (FDFD)",
  "0.4: Finite-difference time-domain (FDTD)",
  "0.5: Semi-analytic methods",
  "0.6: Inverse design",
  "0.7: Layout and PDK",
  "0.8: Tape-out",
  "0.9: Fabrication realism",
  "0.10: Circuits and devices",
  "0.11: Periodic structures and nanophotonics",
  "0.12: Nonlinear and fiber optics",
  "0.13: Beyond",
  "1.0: Stable",
];

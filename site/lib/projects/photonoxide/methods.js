import { cache } from "react";
import { blobUrl, getRepoFile, getRepoFiles, parseFrontMatter } from "./github";

/**
 * The methods photonoxide implements, one page each: docs/methods/<slug>.md
 * in the repository, read at the pinned commit (see github.js).
 *
 * Front matter: title, module (the Rust module, e.g. "mode::vector"),
 * summary, order, papers (a list of { cite, doi }), validation (the
 * validation report's case ids) and examples (example names). The body is
 * Markdown with TeX math (see markdown.js). Which category a write-up is in,
 * and its place there, is CATEGORIES below; `order` only places the ones it
 * doesn't name yet.
 */

export const METHODS_DIR = "docs/methods";

/**
 * The methods by what they compute: each category's groups, and each group's write-ups (their
 * slugs) in reading order. Foundations, last, are what the methods stand on rather than methods.
 * A write-up these don't name (one a newer release added) is shown under "More" at the end;
 * studio/src-tauri/tests/site.rs fails when one in the repository isn't named, or one named
 * isn't there.
 */
export const CATEGORIES = [
  {
    id: "modes",
    title: "Waveguide modes",
    blurb: "A guide's modes and what follows from them, from characteristic equations exact to the last bit to finite differences on any cross-section.",
    groups: [
      { title: "Exact and approximate", slugs: ["slab", "multilayer", "marcatili", "eim"] },
      { title: "On a grid", slugs: ["vector", "hadley", "slab-fd", "bends"] },
      { title: "The window's edges", slugs: ["pml", "walls"] },
      { title: "What a mode gives", slugs: ["fields", "dispersion"] },
    ],
  },
  {
    id: "full-wave",
    title: "Full-wave simulation",
    blurb: "Maxwell's equations on Yee's grid for a whole device: at one frequency as one sparse system, or stepped in time over a band.",
    groups: [
      { title: "Frequency domain (FDFD)", slugs: ["fdfd", "fdfd-ports", "fdfd-3d", "averaging"] },
      { title: "Time domain (FDTD)", slugs: ["fdtd"] },
      { title: "Eigenmode expansion", slugs: ["eme"] },
    ],
  },
  {
    id: "linear-algebra",
    title: "Numerical linear algebra",
    blurb: "The algorithms under the solvers: eigenvalues near a shift or inside a contour, Krylov solves for many sources and long sequences, and sparse direct solves, photonoxide's own or another library's.",
    groups: [
      { title: "Eigenvalues", slugs: ["eigen", "contour"] },
      { title: "Linear systems", slugs: ["block-qmr", "recycling", "multigrid-stencil", "backends"] },
    ],
  },
  {
    id: "gradients",
    title: "Gradients",
    blurb: "A response's derivative with respect to every parameter at the cost of one more solve, by adjoints, each checked against finite differences; and a device's objectives and constraints, optimized on them by genoxide's methods.",
    groups: [{ title: null, slugs: ["fdfd-adjoint", "fdtd-adjoint", "circuit-adjoint", "optimization"] }],
  },
  {
    id: "static",
    title: "Heat and static fields",
    blurb: "Temperature, potential and current on a waveguide's cross-section: one static solver for heaters, electrodes and their Joule heat, exact for layered media and second order elsewhere; the thermo-optic phase shifter on it, its Pπ, time constant and crosstalk, and its compact models in a circuit; the Pockels effect it drives in a mode, to Vπ·L; and the travelling-wave electrode that carries it, its RF index, impedance, losses and the modulator's response.",
    groups: [{ title: null, slugs: ["static", "phase-shifter", "thermal-compact", "electro-optic", "travelling-wave"] }],
  },
  {
    id: "nonlinear",
    title: "Nonlinear optics",
    blurb: "Second- and third-order processes in crystals and waveguides: the wave-vector mismatch of any three- or four-wave process, quasi-phase matching and its duty cycle, birefringent phase matching, and the acceptance bandwidths in wavelength, temperature and angle.",
    groups: [{ title: null, slugs: ["nonlinear"] }],
  },
  {
    id: "circuits",
    title: "Circuits and compact models",
    blurb: "A chip as a netlist of S-matrices, its components' models, the devices that build them from their geometry, spectra fitted as rational functions and written as Touchstone files, and adaptive sweeps that sample a response where it changes.",
    groups: [{ title: null, slugs: ["circuits", "components", "devices", "compact", "sweep"] }],
  },
  {
    id: "fabrication",
    title: "Fabrication",
    blurb: "How a design would be made: the steps in order from published processes, why each one and what else could do it, what each process can achieve with its source, and the flow executed on the library's own geometry to give the design back.",
    groups: [{ title: null, slugs: ["fabrication"] }],
  },
  {
    id: "foundations",
    title: "Foundations",
    blurb: "Not solvers: the units and signs, shapes, stacks and their etches, expressions and material data every method above is built on.",
    groups: [{ title: null, slugs: ["conventions", "geometry", "booleans", "arrays", "bodies", "generators", "stacks", "expressions", "materials", "catalogue", "export", "convergence"] }],
  },
];

/** The category of write-ups no category names yet. */
const MORE = { id: "more", title: "More", blurb: "Write-ups added since this page's categories were.", groups: [] };

/**
 * The named algorithms and where each is written up (slugs), for the index on the methods page:
 * many live inside a larger write-up (QMR and multigrid in FDFD in 3D, subpixel smoothing in
 * FDTD). The site test checks every slug.
 */
export const ALGORITHMS = [
  { name: "AAA rational approximation", does: "a response from few samples as a barycentric rational function, with its error estimated between them", methods: ["sweep"] },
  { name: "Adjoint variable method", does: "every parameter's gradient from one more solve", methods: ["fdfd-adjoint", "fdtd-adjoint", "circuit-adjoint", "averaging"] },
  { name: "Anisotropic averaging from fill fractions", does: "a cell's permittivity from its materials' exact shares and the interface's normal", methods: ["averaging"] },
  { name: "Arnoldi, shift and invert", does: "the eigenvalues nearest a shift", methods: ["eigen"] },
  { name: "Auxiliary differential equations", does: "Drude and Lorentz media stepped in time", methods: ["fdtd"] },
  { name: "Best-first nearest search", does: "the nearest of 10⁴ elements through a hierarchy of their boxes", methods: ["arrays"] },
  { name: "Block QMR with deflation", does: "all of an S-matrix's ports in one Krylov space", methods: ["block-qmr"] },
  { name: "Conformal map", does: "a bend as a straight guide of graded index", methods: ["bends"] },
  { name: "Contour integrals (FEAST)", does: "every eigenvalue inside a disc or an ellipse", methods: ["contour"] },
  { name: "Convolutional PML", does: "absorbing boundaries in the time domain", methods: ["fdtd"] },
  { name: "Diamond blocking", does: "several time steps per pass through memory", methods: ["fdtd"] },
  { name: "Effective index method", does: "a ridge's mode from two slabs", methods: ["eim"] },
  { name: "Finite volumes, conductances in series", does: "heat, potential and current on a graded grid, exact for layered media", methods: ["static"] },
  { name: "Eigenmode expansion", does: "a structure cut into uniform sections, joined by their modes' S-matrices", methods: ["eme"] },
  { name: "Elliptical slices of quadrics", does: "a sphere's, an ellipsoid's or a cone's exact share of a cell", methods: ["bodies"] },
  { name: "GCRO-DR", does: "Krylov recycling across a sequence of solves", methods: ["recycling"] },
  { name: "Hadley's interface and corner equations", does: "high-order differences across interfaces and corners", methods: ["hadley"] },
  { name: "Harmonic inversion", does: "resonances and their Q from a short signal, by filter diagonalization", methods: ["fdtd"] },
  { name: "Krylov–Schur restarts", does: "a fixed-size Krylov space restarted without losing what converged", methods: ["eigen"] },
  { name: "Marcatili's approximation", does: "a rectangular guide from two slab-like equations", methods: ["marcatili"] },
  { name: "Mirror walls", does: "a symmetric guide on half or a quarter of its cross-section", methods: ["walls"] },
  { name: "Morphological opening and closing", does: "corners rounded to a least radius, a first lithography model", methods: ["booleans"] },
  { name: "Multifrontal LU, nested dissection", does: "photonoxide's own sparse direct solve", methods: ["fdfd-3d", "fdfd"] },
  { name: "Multigrid, shifted Laplacian", does: "the 3D iterative solves' preconditioner", methods: ["fdfd-3d", "multigrid-stencil"] },
  { name: "Multimode self-imaging", does: "an MMI's image lengths from its beat length, as geometry inputs", methods: ["generators"] },
  { name: "Nonlinear inverse iteration", does: "an eigenproblem whose matrix depends on its eigenvalue", methods: ["hadley"] },
  { name: "Perturbation theory, first order", does: "a mode's index change from a change of permittivity, a temperature's or an electrode field's", methods: ["fields", "electro-optic"] },
  { name: "Polygon overlay on an integer grid", does: "union, intersection, difference and xor of polygons, exact on shared edges", methods: ["booleans"] },
  { name: "Quasi-phase matching", does: "a process kept in step by a periodically reversed nonlinear coefficient, its period, order, duty cycle and acceptance bandwidths", methods: ["nonlinear"] },
  { name: "QMR, complex symmetric", does: "3D FDFD by iteration, with or without the matrix", methods: ["fdfd-3d"] },
  { name: "Richardson extrapolation", does: "the grid's error removed from three grids", methods: ["vector"] },
  { name: "Shortley–Weller boundary points", does: "curved conductors met between the grid's points, at second order", methods: ["static"] },
  { name: "Stretched-coordinate PML", does: "absorbing layers in the frequency domain", methods: ["pml", "fdfd"] },
  { name: "Sub-network growth", does: "Filipsson's circuit solve, the reference", methods: ["circuits"] },
  { name: "Subpixel smoothing", does: "interfaces between grid points, at second order", methods: ["fdtd", "averaging"] },
  { name: "Total-field/scattered-field", does: "one-way sources of a mode or a plane wave", methods: ["fdfd-ports", "fdtd"] },
  { name: "Transfer matrices", does: "planar stacks' modes, reflection and transmission", methods: ["multilayer"] },
  { name: "Vector fitting", does: "a spectrum as a rational function of frequency", methods: ["compact"] },
  { name: "Yee's grid", does: "Maxwell's curls staggered in space (and leapfrogged in time)", methods: ["fdfd", "fdfd-3d", "fdtd"] },
];

/**
 * @typedef {object} Category
 * @property {string} id
 * @property {string} title
 * @property {string} blurb
 * @property {{ title: string | null, methods: Method[] }[]} groups
 */

/**
 * The methods in their categories and groups, in reading order; empty groups and categories
 * dropped, and the write-ups no category names under "More".
 * @param {Method[]} methods @returns {Category[]}
 */
export function categorize(methods) {
  const bySlug = new Map(methods.map((m) => [m.slug, m]));
  const named = new Set(CATEGORIES.flatMap((c) => c.groups.flatMap((g) => g.slugs)));
  const more = methods.filter((m) => !named.has(m.slug));
  return [...CATEGORIES, { ...MORE, groups: [{ title: null, slugs: more.map((m) => m.slug) }] }]
    .map((c) => ({
      id: c.id,
      title: c.title,
      blurb: c.blurb,
      groups: c.groups
        .map((g) => ({ title: g.title, methods: g.slugs.map((s) => bySlug.get(s)).filter(Boolean) }))
        .filter((g) => g.methods.length),
    }))
    .filter((c) => c.groups.length);
}

/**
 * @typedef {object} Paper
 * @property {string} cite
 * @property {string | null} doi
 */

/**
 * @typedef {object} Method
 * @property {string} slug
 * @property {string} path      the Markdown file in the repository
 * @property {string} title
 * @property {string} summary
 * @property {string | null} module
 * @property {string | null} source   the module's file in the repository, when it exists
 * @property {number} order
 * @property {Paper[]} papers
 * @property {string[]} validation
 * @property {string[]} examples
 * @property {string} body
 * @property {string} category  its category's id
 * @property {string | null} group  its group's title in the category
 */

const list = (value) => (Array.isArray(value) ? value.map(String) : []);

/** The module's source file: src/<path>.rs or src/<path>/mod.rs. */
function sourceOf(module, files) {
  if (!module) return null;
  const base = `src/${module.replace(/::/g, "/")}`;
  return [`${base}.rs`, `${base}/mod.rs`].find((p) => files.has(p)) ?? null;
}

/**
 * The methods in reading order, and their categories.
 * @returns {Promise<{ ok: boolean, methods: Method[], categories: Category[] }>} ok is false when GitHub can't be reached
 */
export const getMethods = cache(async () => {
  const files = await getRepoFiles();
  if (!files) return { ok: false, methods: [], categories: [] };
  const paths = [...files].filter((p) => p.startsWith(`${METHODS_DIR}/`) && p.endsWith(".md")).sort();
  const sources = await Promise.all(paths.map((p) => getRepoFile(p)));
  const methods = paths
    .map((path, i) => {
      if (sources[i] == null) return null;
      const { data, body: text } = parseFrontMatter(sources[i]);
      const slug = path.slice(METHODS_DIR.length + 1, -3);
      const module = data.module ? String(data.module) : null;
      // without a title in the front matter, the write-up's own heading is its title
      const heading = data.title ? null : /^\s*# (.+)\r?\n/.exec(text);
      const body = heading ? text.slice(heading[0].length) : text;
      return {
        slug,
        path,
        title: data.title ? String(data.title) : heading ? heading[1].trim() : slug,
        summary: data.summary ? String(data.summary) : "",
        module,
        source: sourceOf(module, files),
        order: Number.isFinite(data.order) ? data.order : 1000,
        papers: (Array.isArray(data.papers) ? data.papers : []).map((p) => ({
          cite: String(p?.cite ?? ""),
          doi: p?.doi ? String(p.doi) : null,
        })),
        validation: list(data.validation),
        examples: list(data.examples),
        body,
      };
    })
    .filter(Boolean);
  methods.sort((a, b) => a.order - b.order || a.title.localeCompare(b.title));
  // reading order: by category and group, each method knowing its own
  const categories = categorize(methods);
  const ordered = categories.flatMap((c) =>
    c.groups.flatMap((g) => g.methods.map((m) => Object.assign(m, { category: c.id, group: g.title }))),
  );
  return { ok: ordered.length > 0, methods: ordered, categories };
});

/** A category's title, by its id. */
export function categoryTitle(id) {
  return (CATEGORIES.find((c) => c.id === id) ?? MORE).title;
}

/** @param {string} slug @returns {Promise<Method | null>} */
export async function getMethod(slug) {
  const { methods } = await getMethods();
  return methods.find((m) => m.slug === slug) ?? null;
}

/** A DOI's resolver URL. */
export function doiUrl(doi) {
  return `https://doi.org/${encodeURI(doi)}`;
}

export { blobUrl };

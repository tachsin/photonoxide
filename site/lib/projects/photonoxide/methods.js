import { cache } from "react";
import { blobUrl, getRepoFile, getRepoFiles, parseFrontMatter } from "./github";

/**
 * The methods photonoxide implements, one page each: docs/methods/<slug>.md
 * in the repository, read at the pinned commit (see github.js).
 *
 * Front matter: title, module (the Rust module, e.g. "mode::vector"),
 * summary, order, papers (a list of { cite, doi }), validation (the
 * validation report's case ids) and examples (example names). The body is
 * Markdown with TeX math (see markdown.js).
 */

export const METHODS_DIR = "docs/methods";

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
 */

const list = (value) => (Array.isArray(value) ? value.map(String) : []);

/** The module's source file: src/<path>.rs or src/<path>/mod.rs. */
function sourceOf(module, files) {
  if (!module) return null;
  const base = `src/${module.replace(/::/g, "/")}`;
  return [`${base}.rs`, `${base}/mod.rs`].find((p) => files.has(p)) ?? null;
}

/** @returns {Promise<{ ok: boolean, methods: Method[] }>} ok is false when GitHub can't be reached */
export const getMethods = cache(async () => {
  const files = await getRepoFiles();
  if (!files) return { ok: false, methods: [] };
  const paths = [...files].filter((p) => p.startsWith(`${METHODS_DIR}/`) && p.endsWith(".md")).sort();
  const sources = await Promise.all(paths.map((p) => getRepoFile(p)));
  const methods = paths
    .map((path, i) => {
      if (sources[i] == null) return null;
      const { data, body } = parseFrontMatter(sources[i]);
      const slug = path.slice(METHODS_DIR.length + 1, -3);
      const module = data.module ? String(data.module) : null;
      return {
        slug,
        path,
        title: data.title ? String(data.title) : slug,
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
  return { ok: methods.length > 0, methods };
});

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

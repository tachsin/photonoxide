import { readFile } from "node:fs/promises";
import path from "node:path";

/**
 * The Python API reference's pages: where the site reads them, in that order.
 *
 * 1. The app's own copy: `photonoxide-python-reference/` at the app's root, beside
 *    `photonoxide-site.json`. The app's `pnpm sync:photonoxide --python-reference` fills it from
 *    the `python-reference` artifact of the python-wheels.yml run that published the latest
 *    release, and the copy is committed there: pdoc's pages at their own paths (`photonoxide.html`,
 *    `photonoxide/errors.html`, `index.html`) and a `VERSION` file with the release's tag, written
 *    last. A copy without `VERSION` counts as none. The site sync never touches this folder.
 * 2. Else the latest GitHub release's assets `python-reference-<path with "/" as ".">`, which the
 *    releases up to 0.6.2 have.
 *
 * Plain functions with no React or app imports, so the studio's tests run them
 * (`studio/tests/site-python-reference.test.ts`).
 */

/** The folder of the app's copy, relative to the app's root. */
export const PYTHON_REFERENCE_DIR = "photonoxide-python-reference";

/**
 * A page's path as its segments, or null when it isn't a file name pdoc makes (no "..", no odd
 * characters).
 * @param {string} pagePath  e.g. "photonoxide/errors.html"
 * @returns {string[] | null}
 */
export function referenceSegments(pagePath) {
  const segments = String(pagePath).split("/");
  return segments.every((s) => /^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)*$/.test(s)) ? segments : null;
}

/**
 * The release asset a page was attached as: `photonoxide/errors.html` is
 * `python-reference-photonoxide.errors.html`.
 * @param {string[]} segments
 */
export function releaseAssetName(segments) {
  return `python-reference-${segments.join(".")}`;
}

/**
 * The release the app's copy is of (its `VERSION` file, e.g. "v0.6.3"), or null when there is no
 * copy.
 * @param {string} [root]  the app's root
 * @returns {Promise<string | null>}
 */
export async function localReferenceVersion(root = process.cwd()) {
  try {
    const version = (await readFile(path.join(root, PYTHON_REFERENCE_DIR, "VERSION"), "utf8")).trim();
    return version || null;
  } catch {
    return null;
  }
}

/**
 * A page of the app's copy, or null when there is no copy or it has no such page.
 * @param {string[]} segments  from referenceSegments
 * @param {string} [root]  the app's root
 * @returns {Promise<string | null>}
 */
export async function readLocalReferencePage(segments, root = process.cwd()) {
  if (!(await localReferenceVersion(root))) return null;
  try {
    return await readFile(path.join(root, PYTHON_REFERENCE_DIR, ...segments), "utf8");
  } catch {
    return null;
  }
}

/**
 * A page of the reference: the app's copy first, else the release's asset.
 * @param {string} pagePath  e.g. "photonoxide/errors.html"
 * @param {{
 *   readLocal: (segments: string[]) => Promise<string | null>,
 *   fetchRelease: (segments: string[]) => Promise<string | null>,
 * }} sources
 * @returns {Promise<string | null>}
 */
export async function readPythonReferencePage(pagePath, { readLocal, fetchRelease }) {
  const segments = referenceSegments(pagePath);
  if (!segments) return null;
  return (await readLocal(segments)) ?? (await fetchRelease(segments));
}

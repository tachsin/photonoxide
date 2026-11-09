import { cache } from "react";
import { parse as parseYaml } from "yaml";
import { fetchCached } from "@/lib/projects/fetch-cached";
import { latestTagLookup } from "./latest";
import { PHOTONOXIDE_BRANCH, PHOTONOXIDE_COMMIT, PHOTONOXIDE_REPO } from "./meta";
import { readLocalReferencePage, readPythonReferencePage, releaseAssetName } from "./python-reference";

/**
 * Reads the public photonoxide repository on GitHub, server-side, at the
 * latest release: the tag of GitHub's latest release (`v0.4.3`), looked up
 * at most every ten minutes and kept in the server's memory (latest.js), so a
 * release reaches the pages within ten minutes without pinning a new commit,
 * and a deploy (a restart) asks again. When GitHub can't say, the pages read the
 * commit they were synced from (PHOTONOXIDE_COMMIT), as genoxide's pages do.
 *
 * One GitHub API call lists every file (the git tree of the tag); the files
 * themselves come from raw.githubusercontent.com, which has no API rate limit.
 * Both go through Next's data cache (daily). Their URLs name the tag or the
 * commit, neither of which moves, so their cache can't go stale.
 *
 * Links for people ("view on GitHub") go to the branch instead.
 */

const LATEST_URL = `https://api.github.com/repos/${PHOTONOXIDE_REPO}/releases/latest`;
export const BLOB_BASE = `https://github.com/${PHOTONOXIDE_REPO}/blob/${PHOTONOXIDE_BRANCH}/`;

/** One server's lookup, shared by every request (latest.js). */
const latestTag = latestTagLookup({
  ask: async (etag) => {
    try {
      // not through Next's data cache: it outlives deploys and serves stale (latest.js)
      const res = await fetch(LATEST_URL, {
        cache: "no-store",
        headers: {
          Accept: "application/vnd.github+json",
          "User-Agent": "tachsin.gr/projects (+https://tachsin.gr/projects)",
          ...(etag ? { "If-None-Match": etag } : {}),
        },
        signal: AbortSignal.timeout(8000),
      });
      if (res.status === 304) return { status: 304 };
      if (!res.ok) return null;
      const release = await res.json();
      return { status: 200, tag: release?.tag_name ?? null, etag: res.headers.get("etag") };
    } catch {
      return null;
    }
  },
});

/**
 * The latest release's tag, e.g. "v0.4.3", or null when GitHub can't be
 * reached or there is no release.
 * @returns {Promise<string | null>}
 */
export const getLatestTag = cache(() => latestTag());

/** What the pages read: the latest release's tag, or the synced commit. */
const getRef = cache(async () => (await getLatestTag()) ?? PHOTONOXIDE_COMMIT);

/**
 * Every file path at that ref, or null when GitHub can't be reached.
 * @returns {Promise<Set<string> | null>}
 */
export const getRepoFiles = cache(async () => {
  const ref = await getRef();
  const tree = await fetchCached(
    `https://api.github.com/repos/${PHOTONOXIDE_REPO}/git/trees/${ref}?recursive=1`,
    {
      as: "json",
      headers: { Accept: "application/vnd.github+json" },
      tags: ["photonoxide"],
    },
  );
  if (!tree || !Array.isArray(tree.tree)) return null;
  return new Set(tree.tree.filter((e) => e?.type === "blob").map((e) => e.path));
});

/**
 * A file at that ref as text, or null.
 * @param {string} path  repository-relative, e.g. "docs/methods/pml.md"
 */
export const getRepoFile = cache(async (path) => {
  const ref = await getRef();
  return fetchCached(
    `https://raw.githubusercontent.com/${PHOTONOXIDE_REPO}/${ref}/${path
      .split("/")
      .map(encodeURIComponent)
      .join("/")}`,
    { tags: ["photonoxide"] },
  );
});

/**
 * One page of the Python API reference, as HTML text, or null when there is none.
 *
 * pdoc makes the reference in every run of python-wheels.yml (its `python-reference` artifact).
 * The page comes from the app's own copy, which its sync takes from the run that published the
 * latest release and commits; else from the latest release's assets
 * `python-reference-<path with "/" as ".">`, which the releases up to 0.6.2 have
 * (python-reference.js).
 * @param {string} path  the page's path inside the reference, e.g. "photonoxide/errors.html"
 * @returns {Promise<string | null>}
 */
export const getPythonReferencePage = cache((path) =>
  readPythonReferencePage(path, {
    readLocal: (segments) => readLocalReferencePage(segments),
    fetchRelease: async (segments) => {
      const tag = await getLatestTag();
      if (!tag) return null;
      return fetchCached(
        `https://github.com/${PHOTONOXIDE_REPO}/releases/download/${tag}/${encodeURIComponent(releaseAssetName(segments))}`,
        { tags: ["photonoxide"] },
      );
    },
  }),
);

/** A file's page on GitHub, on the branch. */
export function blobUrl(path) {
  return `${BLOB_BASE}${path}`;
}

/** A file itself, on the branch: what an image's `src` takes. */
export function rawUrl(path) {
  return `https://raw.githubusercontent.com/${PHOTONOXIDE_REPO}/${PHOTONOXIDE_BRANCH}/${path}`;
}

const FRONT_MATTER = /^---[ \t]*\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/;

/**
 * Split YAML front matter from a Markdown file.
 * @param {string} source
 * @returns {{ data: Record<string, any>, body: string }}
 */
export function parseFrontMatter(source) {
  const text = String(source ?? "").replace(/^﻿/, "");
  const match = FRONT_MATTER.exec(text);
  if (!match) return { data: {}, body: text };
  let data = {};
  try {
    const parsed = parseYaml(match[1]);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) data = parsed;
  } catch {
    // Malformed YAML: keep the body, drop the fields.
  }
  return { data, body: text.slice(match[0].length) };
}

import { cache } from "react";
import { parse as parseYaml } from "yaml";
import { fetchCached } from "@/lib/projects/fetch-cached";
import { PHOTONOXIDE_BRANCH, PHOTONOXIDE_COMMIT, PHOTONOXIDE_REPO } from "./meta";

/**
 * Reads the public photonoxide repository on GitHub, server-side, at the
 * latest release: the tag of GitHub's latest release (`v0.4.3`), looked up
 * through Next's data cache (daily), so a release reaches the pages within a
 * day without pinning a new commit. When GitHub can't say, the pages read the
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

/**
 * The latest release's tag, e.g. "v0.4.3", or null when GitHub can't be
 * reached or there is no release.
 * @returns {Promise<string | null>}
 */
export const getLatestTag = cache(async () => {
  const release = await fetchCached(LATEST_URL, {
    as: "json",
    headers: { Accept: "application/vnd.github+json" },
    tags: ["photonoxide"],
  });
  const tag = release?.tag_name;
  return typeof tag === "string" && /^v\d+\.\d+\.\d+$/.test(tag) ? tag : null;
});

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

/** A file's page on GitHub, on the branch. */
export function blobUrl(path) {
  return `${BLOB_BASE}${path}`;
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

import { cache } from "react";
import { parse as parseYaml } from "yaml";
import { fetchCached } from "@/lib/projects/fetch-cached";
import { PHOTONOXIDE_BRANCH, PHOTONOXIDE_COMMIT, PHOTONOXIDE_REPO } from "./meta";

/**
 * Reads the public photonoxide repository on GitHub, server-side, at the
 * commit the pages were synced from (PHOTONOXIDE_COMMIT), as genoxide's
 * pages read theirs.
 *
 * One GitHub API call lists every file (the git tree of the commit); the
 * files themselves come from raw.githubusercontent.com, which has no API
 * rate limit. Both go through Next's data cache (daily). The URLs name the
 * commit, so the cache can't go stale: pinning a new commit fetches new URLs.
 *
 * Links for people ("view on GitHub") go to the branch instead.
 */

const TREE_URL = `https://api.github.com/repos/${PHOTONOXIDE_REPO}/git/trees/${PHOTONOXIDE_COMMIT}?recursive=1`;
export const RAW_BASE = `https://raw.githubusercontent.com/${PHOTONOXIDE_REPO}/${PHOTONOXIDE_COMMIT}/`;
export const BLOB_BASE = `https://github.com/${PHOTONOXIDE_REPO}/blob/${PHOTONOXIDE_BRANCH}/`;

/**
 * Every file path of the commit, or null when GitHub can't be reached.
 * @returns {Promise<Set<string> | null>}
 */
export const getRepoFiles = cache(async () => {
  const tree = await fetchCached(TREE_URL, {
    as: "json",
    headers: { Accept: "application/vnd.github+json" },
    tags: ["photonoxide"],
  });
  if (!tree || !Array.isArray(tree.tree)) return null;
  return new Set(tree.tree.filter((e) => e?.type === "blob").map((e) => e.path));
});

/**
 * A file of the commit as text, or null.
 * @param {string} path  repository-relative, e.g. "docs/methods/pml.md"
 */
export const getRepoFile = cache(async (path) =>
  fetchCached(`${RAW_BASE}${path.split("/").map(encodeURIComponent).join("/")}`, {
    tags: ["photonoxide"],
  }),
);

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

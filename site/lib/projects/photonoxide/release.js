import { cache } from "react";
import { getRepoFile } from "./github";

/**
 * The latest release, read from CHANGELOG.md at the pinned commit (see
 * github.js): its first `## [x.y.z]` heading, which release-plz writes when
 * it releases. `## [Unreleased]` is skipped.
 */

const RELEASE = /^##\s+\[(\d+\.\d+\.\d+)\]/;

/** @param {string} markdown @returns {string | null} e.g. "0.3.3" */
export function parseLatestRelease(markdown) {
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
    const match = RELEASE.exec(line);
    if (match) return match[1];
  }
  return null;
}

/** @returns {Promise<string | null>} null when GitHub can't be reached */
export const getLatestRelease = cache(async () => parseLatestRelease(await getRepoFile("CHANGELOG.md")));

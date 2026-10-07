import { cache } from "react";
import { getLatestTag, getRepoFile } from "./github";

/**
 * The latest release: GitHub's latest release (its tag without the "v"), so
 * a new release shows within a day (see github.js); failing that, the first
 * `## [x.y.z]` heading of CHANGELOG.md, which release-plz writes when it
 * releases. `## [Unreleased]` is skipped.
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
export const getLatestRelease = cache(async () => {
  const tag = await getLatestTag();
  if (tag) return tag.slice(1);
  return parseLatestRelease(await getRepoFile("CHANGELOG.md"));
});

import { cache } from "react";
import { getRepoFile } from "./github";
import { FALLBACK_MILESTONES } from "./meta";
import { milestonesFromHeadings, parseMilestones, splitRoadmap } from "./milestones";
import { getLatestRelease } from "./release";

/**
 * photonoxide's milestones, read from ROADMAP.md at the latest release's tag (see github.js),
 * with the titles of FALLBACK_MILESTONES when GitHub can't be reached. The latest release's
 * milestone and every older one are done (milestones.js).
 */

/** @typedef {import("./milestones").Milestone} Milestone */

/**
 * @returns {Promise<{ ok: boolean, milestones: Milestone[] }>}
 *   ok is false when the fallback was used
 */
export const getMilestones = cache(async () => {
  const [markdown, release] = await Promise.all([getRepoFile("ROADMAP.md"), getLatestRelease()]);
  const parsed = markdown ? parseMilestones(markdown, release) : [];
  return parsed.length
    ? { ok: true, milestones: parsed }
    : { ok: false, milestones: milestonesFromHeadings(FALLBACK_MILESTONES, release) };
});

/**
 * ROADMAP.md in its sections and blocks (milestones.js, `splitRoadmap`), its milestones' status
 * as getMilestones has it, or null when it can't be read.
 */
export const getRoadmap = cache(async () => {
  const [markdown, release] = await Promise.all([getRepoFile("ROADMAP.md"), getLatestRelease()]);
  return markdown ? splitRoadmap(markdown, release) : null;
});

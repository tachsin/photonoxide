import { cache } from "react";
import { getRepoFile } from "./github";
import { FALLBACK_MILESTONES } from "./meta";
import { milestonesFromHeadings, parseMilestones } from "./milestones";
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

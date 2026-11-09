import { cache } from "react";
import { getRepoFile } from "./github";
import { FALLBACK_MILESTONES } from "./meta";
import { parseMilestones, splitRoadmap } from "./milestones";

/**
 * photonoxide's milestones, read from ROADMAP.md at the latest release's tag
 * (see github.js), with the titles of FALLBACK_MILESTONES when GitHub can't
 * be reached. The parsing is milestones.js's, which the studio's tests run.
 */

export { parseMilestones };

/** @typedef {import("./milestones").Milestone} Milestone */

function fallback() {
  const milestones = FALLBACK_MILESTONES.map((heading) => {
    const [version, ...rest] = heading.split(": ");
    const title = rest.join(": ");
    const done = title.endsWith("✅");
    return {
      version,
      title: title.replace(/\s*✅$/, ""),
      status: done ? "done" : "planned",
      items: [],
      done: 0,
      total: 0,
    };
  });
  const next = milestones.find((m) => m.status !== "done");
  if (next) next.status = "next";
  return milestones;
}

/**
 * @returns {Promise<{ ok: boolean, milestones: Milestone[] }>}
 *   ok is false when the fallback was used
 */
export const getMilestones = cache(async () => {
  const markdown = await getRepoFile("ROADMAP.md");
  const parsed = markdown ? parseMilestones(markdown) : [];
  return parsed.length ? { ok: true, milestones: parsed } : { ok: false, milestones: fallback() };
});

/**
 * ROADMAP.md in its sections and blocks (milestones.js, `splitRoadmap`), or
 * null when it can't be read.
 */
export const getRoadmap = cache(async () => {
  const markdown = await getRepoFile("ROADMAP.md");
  return markdown ? splitRoadmap(markdown) : null;
});

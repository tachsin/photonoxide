import { cache } from "react";
import { getRepoFile } from "./github";
import { FALLBACK_MILESTONES } from "./meta";

/**
 * photonoxide's milestones, read from ROADMAP.md at the pinned commit
 * (see github.js), with the titles of FALLBACK_MILESTONES when GitHub
 * can't be reached.
 *
 * A milestone is a `### <version>: <title>` heading. Its items are the
 * `- [ ]` / `- [x]` lines under it (top level only), their bold lead-in
 * being the item's name. A heading ending in ✅ is done whatever its boxes
 * say; the first milestone not done is "next".
 */

/**
 * @typedef {object} Milestone
 * @property {string} version   e.g. "0.4"
 * @property {string} title     e.g. "Finite-difference time-domain (FDTD)"
 * @property {"done" | "next" | "planned"} status
 * @property {string[]} items   the bold lead-ins of its checklist, in order
 * @property {number} done      checked items
 * @property {number} total     all items
 */

const HEADING = /^###\s+(\d+(?:\.\d+)*):\s+(.+?)\s*$/;
const ITEM = /^- \[( |x|X)\]\s+(.*)$/;
const BOLD = /\*\*(.+?)\*\*/;

/** @param {string} markdown @returns {Milestone[]} */
export function parseMilestones(markdown) {
  /** @type {Milestone[]} */
  const milestones = [];
  let current = null;
  for (const line of markdown.split(/\r?\n/)) {
    if (line.startsWith("## ")) {
      current = null;
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      const done = heading[2].endsWith("✅");
      current = {
        version: heading[1],
        title: heading[2].replace(/\s*✅$/, ""),
        status: done ? "done" : "planned",
        items: [],
        done: 0,
        total: 0,
      };
      milestones.push(current);
      continue;
    }
    if (line.startsWith("### ")) {
      current = null; // "### Throughout" and the like
      continue;
    }
    const item = current ? ITEM.exec(line) : null;
    if (item) {
      current.total += 1;
      if (item[1] !== " ") current.done += 1;
      const bold = BOLD.exec(item[2]);
      const name = (bold ? bold[1] : item[2]).replace(/[:,.]$/, "").trim();
      if (name) current.items.push(name);
    }
  }
  const next = milestones.find((m) => m.status !== "done");
  if (next) next.status = "next";
  return milestones;
}

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
